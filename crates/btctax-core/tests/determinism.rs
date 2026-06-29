use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn acq(src_ref: &str, h: u8, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
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
fn identical_set_any_order_same_state() {
    let prices = StaticPrices::default();
    let cfg = ProjectionConfig::default();
    let a = acq("A", 1, 100_000, dec!(60.00));
    let b = acq("B", 2, 50_000, dec!(31.00));
    let s1 = project(&[a.clone(), b.clone()], &prices, &cfg);
    let s2 = project(&[b, a], &prices, &cfg); // reversed load order
    assert_eq!(s1, s2);
    assert_eq!(s1.holdings_by_wallet.values().sum::<i64>(), 150_000);
}
