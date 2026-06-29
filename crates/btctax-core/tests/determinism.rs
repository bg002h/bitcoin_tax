use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn acq(src_ref: &str, h: u8, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    acq_with_source(Source::Coinbase, src_ref, h, sat, cost)
}

fn acq_with_source(
    source: Source,
    src_ref: &str,
    h: u8,
    sat: i64,
    cost: rust_decimal::Decimal,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(source, SourceRef::new(src_ref)),
        utc_timestamp: datetime!(2025-03-01 00:00:00 UTC).replace_hour(h).unwrap(),
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange {
            provider: "cb".into(),
            account: "m".into(),
        }),
        payload: EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    }
}

#[test]
fn distinct_timestamp_any_order_same_state() {
    let prices = StaticPrices::default();
    let cfg = ProjectionConfig::default();
    let a = acq("A", 1, 100_000, dec!(60.00));
    let b = acq("B", 2, 50_000, dec!(31.00));
    let s1 = project(&[a.clone(), b.clone()], &prices, &cfg);
    let s2 = project(&[b, a], &prices, &cfg); // reversed load order
    assert_eq!(s1, s2);
    assert_eq!(s1.holdings_by_wallet.values().sum::<i64>(), 150_000);
}

#[test]
fn same_timestamp_different_sources_tiebreak_is_deterministic() {
    // Test that source priority (Swan > Coinbase > Gemini > River) correctly orders
    // events with identical timestamps. Events created in different input order
    // should produce identical LedgerState.
    let prices = StaticPrices::default();
    let cfg = ProjectionConfig::default();

    // Create two acquire events at the same timestamp but from different sources.
    // Swan (priority 0) should fold before Coinbase (priority 1).
    let swan = acq_with_source(Source::Swan, "S1", 10, 100_000, dec!(60.00));
    let coinbase = acq_with_source(Source::Coinbase, "C1", 10, 50_000, dec!(31.00));

    // Project with Swan first, then Coinbase
    let s1 = project(&[swan.clone(), coinbase.clone()], &prices, &cfg);

    // Project with Coinbase first, then Swan (reversed)
    let s2 = project(&[coinbase, swan], &prices, &cfg);

    // Regardless of input order, the canonical sort should produce identical state
    assert_eq!(s1, s2);
    assert_eq!(s1.holdings_by_wallet.values().sum::<i64>(), 150_000);
}

#[test]
fn same_timestamp_same_source_different_refs_tiebreak_is_deterministic() {
    // Test that source_ref (lexicographic) correctly orders events with identical
    // timestamps and source. This exercises the 3rd sort key in sort_canonical.
    let prices = StaticPrices::default();
    let cfg = ProjectionConfig::default();

    // Create two acquire events at the same timestamp, same source, but different refs.
    // Refs will sort lexicographically: "REF_A" < "REF_B"
    let ref_a = acq_with_source(Source::River, "REF_A", 10, 100_000, dec!(60.00));
    let ref_b = acq_with_source(Source::River, "REF_B", 10, 50_000, dec!(31.00));

    // Project with REF_A first, then REF_B
    let s1 = project(&[ref_a.clone(), ref_b.clone()], &prices, &cfg);

    // Project with REF_B first, then REF_A (reversed)
    let s2 = project(&[ref_b, ref_a], &prices, &cfg);

    // Regardless of input order, the canonical sort should produce identical state
    assert_eq!(s1, s2);
    assert_eq!(s1.holdings_by_wallet.values().sum::<i64>(), 150_000);
}

#[test]
fn three_events_mixed_tiebreak_deterministic() {
    // Test a shuffle with three events having the same timestamp but different
    // sources and refs, exercising all three sort keys.
    let prices = StaticPrices::default();
    let cfg = ProjectionConfig::default();

    // Three events at the same timestamp, different sources and refs
    // Expected sort order (by priority and ref): Swan-A < Swan-B < Coinbase-C < Gemini-D
    let swan_a = acq_with_source(Source::Swan, "A", 12, 30_000, dec!(18.00));
    let swan_b = acq_with_source(Source::Swan, "B", 12, 20_000, dec!(12.00));
    let coinbase_c = acq_with_source(Source::Coinbase, "C", 12, 25_000, dec!(15.50));

    // Project with original order
    let s1 = project(
        &[swan_a.clone(), swan_b.clone(), coinbase_c.clone()],
        &prices,
        &cfg,
    );

    // Project with permuted order: Coinbase first, then Swan events reversed
    let s2 = project(
        &[coinbase_c.clone(), swan_b.clone(), swan_a.clone()],
        &prices,
        &cfg,
    );

    // Project with another permutation: Swan-B, Coinbase, Swan-A
    let s3 = project(
        &[swan_b.clone(), coinbase_c.clone(), swan_a.clone()],
        &prices,
        &cfg,
    );

    // All three input orders should produce identical state
    assert_eq!(s1, s2);
    assert_eq!(s2, s3);
    assert_eq!(s1.holdings_by_wallet.values().sum::<i64>(), 75_000);
}
