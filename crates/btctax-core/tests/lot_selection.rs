//! §A.4 KATs — per-disposal specific-identification (`LotSelection`/`LotPick`) wired through the fold.
//! A valid selection re-orders WHICH lots are consumed (basis/term flip) while Σsat/Σbasis stay invariant;
//! an invalid selection raises the hard `LotSelectionInvalid` blocker and falls back to method order
//! (still conserved). Duplicate selections for one disposal → `DecisionConflict`; voided are excluded.
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::LotMethod;
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
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
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
fn election(seq: u64, made: time::OffsetDateTime, eff: time::Date, m: LotMethod) -> LedgerEvent {
    dec_ev(
        seq,
        made,
        EventPayload::MethodElection(MethodElection {
            effective_from: eff,
            method: m,
            wallet: None,
        }),
    )
}
fn has(st: &LedgerState, k: BlockerKind) -> bool {
    st.blockers.iter().any(|b| b.kind == k)
}
fn pid(rf: &str) -> LotId {
    LotId {
        origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        split_sequence: 0,
    }
}
fn lot_selection(
    seq: u64,
    ts: time::OffsetDateTime,
    disposal_ref: &str,
    picks: Vec<LotPick>,
) -> LedgerEvent {
    dec_ev(
        seq,
        ts,
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new(disposal_ref)),
            lots: picks,
        }),
    )
}

// Post-2025 pool with 3 lots whose method orders are distinct (FIFO->A, LIFO->C, HIFO->B).
fn three_post2025() -> Vec<LedgerEvent> {
    vec![
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
        buy(
            "C",
            datetime!(2025-04-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ),
    ]
}

#[test]
fn selection_overrides_in_force_method() {
    let mut evs = three_post2025();
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
    )); // HIFO would pick B
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    // explicit selection of the FIFO lot A overrides HIFO for this disposal.
    evs.push(lot_selection(
        2,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("A"),
            sat: 100_000,
        }],
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::LotSelectionInvalid));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // picked A, not HIFO's B
}

#[test]
fn selection_principal_conservation_violation_blocks() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(lot_selection(
        1,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("A"),
            sat: 50_000,
        }],
    )); // 50k != 100k
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn selection_unknown_lot_blocks() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(lot_selection(
        1,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("NOPE"),
            sat: 100_000,
        }],
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn selection_cross_wallet_blocks_post_2025() {
    // Two wallets; disposal in wallet cb; pick a lot held in self-custody -> §1.1012-1(j) cross-account ID forbidden.
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let evs = vec![
        buy(
            "CB",
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        LedgerEvent {
            id: EventId::import(Source::Swan, SourceRef::new("COLD")),
            utc_timestamp: datetime!(2025-02-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(cold.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(40.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        },
        sell(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ), // in cb()
        lot_selection(
            1,
            datetime!(2025-07-01 00:00:00 UTC),
            "D",
            vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::Swan, SourceRef::new("COLD")),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn duplicate_selection_for_one_disposal_is_decision_conflict() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(lot_selection(
        1,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("A"),
            sat: 100_000,
        }],
    ));
    evs.push(lot_selection(
        2,
        datetime!(2025-07-02 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("C"),
            sat: 100_000,
        }],
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::DecisionConflict));
}

#[test]
fn voided_selection_is_excluded() {
    let mut evs = three_post2025();
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
    ));
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(lot_selection(
        2,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("A"),
            sat: 100_000,
        }],
    ));
    evs.push(dec_ev(
        3,
        datetime!(2025-07-05 00:00:00 UTC),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: EventId::decision(2),
        }),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.disposals[0].legs[0].basis, dec!(90.00)); // voided selection -> HIFO -> B
}

#[test]
fn selection_targeting_pending_out_is_invalid() {
    // An unmatched TransferOut folds to PendingOut (non-honoring); a selection on it is rejected.
    let evs = vec![
        buy(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            utc_timestamp: datetime!(2025-06-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(w()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 50_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        },
        lot_selection(
            1,
            datetime!(2025-06-01 00:00:00 UTC),
            "OUT",
            vec![LotPick {
                lot: pid("A"),
                sat: 50_000,
            }],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::LotSelectionInvalid));
}

#[test]
fn fee_bearing_reclassified_disposal_under_selection_consumes_fee_fifo_from_remainder() {
    // Reclassified TransferOut->Dispose with fee_sat; selection picks the principal lot; the on-chain fee
    // consumes FIFO from the post-selection remainder (A.4(a)). Conservation must balance.
    let evs = vec![
        buy(
            "OLD",
            datetime!(2025-02-01 00:00:00 UTC),
            60_000,
            dec!(30.00),
        ), // FIFO remainder for the fee
        buy(
            "NEW",
            datetime!(2025-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            utc_timestamp: datetime!(2025-07-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(w()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 100_000,
                fee_sat: Some(500),
                dest_addr: None,
                txid: None,
            }),
        },
        dec_ev(
            1,
            datetime!(2025-08-01 00:00:00 UTC),
            EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
                as_: OutflowClass::Dispose {
                    kind: DisposeKind::Sell,
                },
                principal_proceeds_or_fmv: dec!(120.00),
                fee_usd: None,
                donee: None,
            }),
        ),
        // selection picks NEW for the 100k principal; the 500-sat fee then FIFO-consumes OLD.
        lot_selection(
            2,
            datetime!(2025-07-01 00:00:00 UTC),
            "OUT",
            vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("NEW")),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::LotSelectionInvalid));
    let report = btctax_core::conservation_report(&st);
    assert!(report.balanced, "{report:?}");
    let leg = &st.disposals[0].legs[0];
    // Principal picked NEW ($90.00 basis); under TP8(c) DEFAULT the 500-sat on-chain fee is consumed
    // FIFO from the post-selection remainder (OLD: 30.00/60_000 per sat → 500 sat = $0.25) and its basis
    // re-homes onto this single disposal leg (non-taxable; full basis carries, fold.rs rehome_onto_disposal_leg).
    // So the reported basis is NEW's $90.00 + the $0.25 fee-sat carry = $90.25 (NOT OLD's $30 — selection honored).
    assert_eq!(leg.basis, dec!(90.25));
    assert_eq!(st.stats.fee_sats_consumed, 500); // fee taken from remainder (OLD), FIFO
}

#[test]
fn pre2025_selection_in_universal_pool() {
    let evs = vec![
        buy(
            "A",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy(
            "B",
            datetime!(2024-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        sell(
            "D",
            datetime!(2024-09-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
        lot_selection(
            1,
            datetime!(2024-09-01 00:00:00 UTC),
            "D",
            vec![LotPick {
                lot: pid("B"),
                sat: 100_000,
            }],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::LotSelectionInvalid));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(90.00)); // picked B from the Universal pool
}

#[test]
fn determinism_with_elections_and_selections_is_load_order_independent() {
    let mut a = three_post2025();
    a.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
    ));
    a.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    a.push(lot_selection(
        2,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("C"),
            sat: 100_000,
        }],
    ));
    let mut b = a.clone();
    b.reverse();
    let s1 = project(&a, &StaticPrices::default(), &ProjectionConfig::default());
    let s2 = project(&b, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(s1.disposals, s2.disposals);
    assert_eq!(s1.lots, s2.lots);
}
