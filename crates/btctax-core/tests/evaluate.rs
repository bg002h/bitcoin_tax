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
use btctax_core::BlockerKind;
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

/// M2: for an EXISTING disposal, an injected selection is validated against the event's RESOLVED
/// principal sat — never the caller-supplied `candidate.sat`. A wrong `candidate.sat` (60k, while
/// the real disposal is 100k) that happens to match the selection's Σ must NOT silently
/// under-consume; it must raise a `LotSelectionInvalid` blocker.
///
/// Before the fix the guard compared Σpicks == `candidate.sat` (60k == 60k → no blocker), and the
/// fold then consumed the real 100k principal with a 60k selection — a silent wrong number
/// (`consume_picks` returns shortfall=0 unconditionally). After the fix Σpicks is compared to the
/// resolved 100k principal → mismatch → blocker.
#[test]
fn existing_disposal_selection_validated_against_resolved_principal_not_candidate_sat() {
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
        sat: 60_000, // WRONG: the real disposal principal is 100_000.
        kind: DisposeKind::Sell,
        proceeds: None,
    };
    // Selection Σ = 60_000 — matches the wrong candidate.sat, but NOT the real 100k principal.
    let picks = vec![LotPick {
        lot: LotId {
            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("B")),
            split_sequence: 0,
        },
        sat: 60_000,
    }];
    let out = evaluate_disposal(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
        &cand,
        Some(&picks),
    )
    .unwrap();
    assert!(
        out.blockers
            .iter()
            .any(|b| matches!(b.kind, BlockerKind::LotSelectionInvalid)),
        "a selection whose Σ (60000) != the resolved principal (100000) must raise \
         LotSelectionInvalid — not silently mis-consume; got {:?}",
        out.blockers
    );
}

/// Pinning KAT (A-Task-9b): `evaluate_disposal(existing, no selection)` is the **no-op identity**
/// — it must produce the SAME legs/gains as a plain `project()` call for that disposal.
///
/// This proves that injecting no candidate selection leaves the projection completely unchanged,
/// and that the evaluate path does not alter any fold output relative to the real projection.
#[test]
fn evaluate_disposal_existing_no_selection_is_no_op_identity() {
    // Simple deterministic ledger: one buy (FIFO lot A) + one sell (consumes A entirely).
    let evs = vec![
        buy(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
    ];

    // Reference: what the real projection computes for disposal "D".
    let state = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(state.disposals.len(), 1, "setup: exactly one disposal");
    let ref_legs = state.disposals[0].legs.clone();

    // evaluate_disposal with the same existing event and NO candidate selection.
    let cand = CandidateDisposal {
        existing_event: Some(EventId::import(Source::Coinbase, SourceRef::new("D"))),
        wallet: w(),
        date: date!(2025 - 07 - 01),
        sat: 100_000,
        kind: DisposeKind::Sell,
        proceeds: None,
    };
    let out = evaluate_disposal(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
        &cand,
        None, // no injected selection — must equal project() exactly
    )
    .unwrap();

    // Legs must be byte-identical to the real projection.
    assert_eq!(
        out.legs, ref_legs,
        "evaluate_disposal with no selection must match project() legs (no-op identity)"
    );
    // No blockers expected for a clean single-lot disposal.
    assert!(
        out.blockers.is_empty(),
        "no blockers expected for a clean disposal with no injected selection; got {:?}",
        out.blockers
    );
    // Gains must match (both are derived from the same legs, but assert independently for clarity).
    let ref_st: rust_decimal::Decimal = ref_legs
        .iter()
        .filter(|l| l.term == btctax_core::Term::ShortTerm)
        .map(|l| l.gain)
        .sum();
    let ref_lt: rust_decimal::Decimal = ref_legs
        .iter()
        .filter(|l| l.term == btctax_core::Term::LongTerm)
        .map(|l| l.gain)
        .sum();
    assert_eq!(
        out.st_gain, ref_st,
        "st_gain must match project() (no-op identity)"
    );
    assert_eq!(
        out.lt_gain, ref_lt,
        "lt_gain must match project() (no-op identity)"
    );
}
