//! §A.6 KATs — side-effect-free `evaluate_disposal` entrypoint.
//!
//! Covers: synthetic future disposal requires `--proceeds` when no price; synthetic disposal
//! with explicit proceeds returns the correct gain and leaves the ledger unchanged; dataset FMV
//! is used when proceeds is omitted but a price exists; an existing disposal can be re-scored
//! with an injected selection (without mutating the vault).
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{
    evaluate_disposal, project, CandidateDisposal, EvaluateError, ProjectionConfig,
};
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

fn w() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}

fn imp(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w()),
        payload: p,
    }
}

fn buy(rf: &str, ts: time::OffsetDateTime, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}

fn sell(
    rf: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    proceeds: rust_decimal::Decimal,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}

#[test]
fn synthetic_future_disposal_requires_proceeds_when_no_price() {
    let evs = vec![buy(
        "A",
        datetime!(2025-02-01 00:00:00 UTC),
        100_000,
        dec!(50.00),
    )];
    let cand = CandidateDisposal {
        existing_event: None,
        wallet: w(),
        date: date!(2030 - 01 - 01),
        sat: 100_000,
        kind: DisposeKind::Sell,
        proceeds: None,
    };
    let err = evaluate_disposal(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
        &cand,
        None,
    )
    .unwrap_err();
    assert_eq!(err, EvaluateError::ProceedsRequired);
}

#[test]
fn synthetic_disposal_with_proceeds_returns_gain_and_is_side_effect_free() {
    let evs = vec![buy(
        "A",
        datetime!(2025-02-01 00:00:00 UTC),
        100_000,
        dec!(50.00),
    )];
    let cand = CandidateDisposal {
        existing_event: None,
        wallet: w(),
        date: date!(2026 - 06 - 01),
        sat: 100_000,
        kind: DisposeKind::Sell,
        proceeds: Some(dec!(150.00)),
    };
    let out = evaluate_disposal(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
        &cand,
        None,
    )
    .unwrap();
    assert_eq!(out.legs.len(), 1);
    assert_eq!(out.legs[0].gain, dec!(100.00)); // 150 - 50
    assert_eq!(out.lt_gain, dec!(100.00)); // acquired 2025-02, sold 2026-06 -> LT
                                           // side-effect-free: a plain projection of the original events still has no disposals.
    let base = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(base.disposals.is_empty());
}

#[test]
fn synthetic_disposal_uses_dataset_fmv_when_proceeds_omitted_and_price_exists() {
    let mut px = std::collections::BTreeMap::new();
    px.insert(date!(2026 - 06 - 01), dec!(100000.00)); // $/BTC -> FMV(100k sat) = $100
    let prices = StaticPrices(px);
    let evs = vec![buy(
        "A",
        datetime!(2025-02-01 00:00:00 UTC),
        100_000,
        dec!(50.00),
    )];
    let cand = CandidateDisposal {
        existing_event: None,
        wallet: w(),
        date: date!(2026 - 06 - 01),
        sat: 100_000,
        kind: DisposeKind::Sell,
        proceeds: None,
    };
    let out = evaluate_disposal(&evs, &prices, &ProjectionConfig::default(), &cand, None).unwrap();
    assert_eq!(out.legs[0].proceeds, dec!(100.00));
}

#[test]
fn existing_disposal_scored_with_an_injected_selection() {
    // ledger has a post-2025 sell; evaluate a candidate selection over it WITHOUT persisting anything.
    let evs = vec![
        buy(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy(
            "B",
            datetime!(2025-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        sell(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
    ];
    let cand = CandidateDisposal {
        existing_event: Some(EventId::import(Source::Coinbase, SourceRef::new("D"))),
        wallet: w(),
        date: date!(2025 - 07 - 01),
        sat: 100_000,
        kind: DisposeKind::Sell,
        proceeds: None,
    };
    let picks = vec![LotPick {
        lot: LotId {
            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("B")),
            split_sequence: 0,
        },
        sat: 100_000,
    }];
    let out = evaluate_disposal(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
        &cand,
        Some(&picks),
    )
    .unwrap();
    assert_eq!(out.legs[0].basis, dec!(90.00)); // scored against the picked lot B (default FIFO would pick A)
}
