//! Task 13: property-based conservation tests + FR9 golden KAT.
//!
//! Proptest generators synthesise fee'd `TransferLink` self-transfers so the C1 / Σbasis
//! path is exercised.  All three property tests have REAL assertion bodies (not `let _ = st;`).
//!
//! If any property test FAILS it reveals a genuine conservation bug — do NOT weaken the assertion.

use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use proptest::prelude::*;
use rust_decimal::Decimal;
use time::macros::{datetime, offset};

// ── wallet helpers ──────────────────────────────────────────────────────────────────────────────

fn wal_a() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn wal_b() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}

// ── proptest generators ──────────────────────────────────────────────────────────────────────────

/// One generated op against a single source wallet (post-2025).
#[derive(Debug, Clone)]
enum Step {
    Acquire {
        sat: i64,
        cents: i64,
    },
    Dispose {
        sat: i64,
        cents: i64,
    },
    /// A fee'd self-transfer A->B (the C1 path): principal `sat`, on-chain `fee`.
    SelfXfer {
        sat: i64,
        fee: i64,
    },
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (1_000i64..5_000_000, 1i64..2_000_000)
            .prop_map(|(sat, cents)| Step::Acquire { sat, cents }),
        (1_000i64..5_000_000, 1i64..2_000_000)
            .prop_map(|(sat, cents)| Step::Dispose { sat, cents }),
        (1_000i64..5_000_000, 0i64..500).prop_map(|(sat, fee)| Step::SelfXfer { sat, fee }),
    ]
}

/// Materialize a step list into a well-formed event vector (unique source_refs / decision_seqs).
fn build(steps: &[Step]) -> Vec<LedgerEvent> {
    let mut evs = Vec::new();
    let (mut seq, ts) = (0u64, datetime!(2025-03-01 00:00:00 UTC));
    for (i, s) in steps.iter().enumerate() {
        match s {
            Step::Acquire { sat, cents } => evs.push(LedgerEvent {
                id: EventId::import(Source::Coinbase, SourceRef::new(format!("A{i}"))),
                utc_timestamp: ts,
                original_tz: offset!(+00:00),
                wallet: Some(wal_a()),
                payload: EventPayload::Acquire(Acquire {
                    sat: *sat,
                    usd_cost: Decimal::new(*cents, 2),
                    fee_usd: Decimal::ZERO,
                    basis_source: BasisSource::ExchangeProvided,
                }),
            }),
            Step::Dispose { sat, cents } => evs.push(LedgerEvent {
                id: EventId::import(Source::Coinbase, SourceRef::new(format!("D{i}"))),
                utc_timestamp: ts,
                original_tz: offset!(+00:00),
                wallet: Some(wal_a()),
                payload: EventPayload::Dispose(Dispose {
                    sat: *sat,
                    usd_proceeds: Decimal::new(*cents, 2),
                    fee_usd: Decimal::ZERO,
                    kind: DisposeKind::Sell,
                }),
            }),
            Step::SelfXfer { sat, fee } => {
                let (out_ref, in_ref) = (format!("O{i}"), format!("I{i}"));
                evs.push(LedgerEvent {
                    id: EventId::import(Source::Coinbase, SourceRef::new(out_ref.clone())),
                    utc_timestamp: ts,
                    original_tz: offset!(+00:00),
                    wallet: Some(wal_a()),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: *sat,
                        fee_sat: Some(*fee),
                        dest_addr: None,
                        txid: None,
                    }),
                });
                evs.push(LedgerEvent {
                    id: EventId::import(Source::Swan, SourceRef::new(in_ref.clone())),
                    utc_timestamp: ts,
                    original_tz: offset!(+00:00),
                    wallet: Some(wal_b()),
                    payload: EventPayload::TransferIn(TransferIn {
                        sat: *sat,
                        src_addr: None,
                        txid: None,
                    }),
                });
                seq += 1;
                evs.push(LedgerEvent {
                    id: EventId::decision(seq),
                    utc_timestamp: datetime!(2026-01-01 00:00:00 UTC),
                    original_tz: offset!(+00:00),
                    wallet: None,
                    payload: EventPayload::TransferLink(TransferLink {
                        out_event: EventId::import(Source::Coinbase, SourceRef::new(out_ref)),
                        in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                            Source::Swan,
                            SourceRef::new(in_ref),
                        )),
                    }),
                });
            }
        }
    }
    evs
}

/// General mix (acquires / covered-or-not disposes / fee'd self-transfers). The `has_uncovered` guard
/// in the test handles the cases where a random Dispose exceeds holdings.
fn arb_events() -> impl Strategy<Value = Vec<LedgerEvent>> {
    prop::collection::vec(arb_step(), 1..8).prop_map(|s| build(&s))
}

/// No basis-pending paths (no income-missing / unknown-basis gifts) AND no disposals/removals — only acquires
/// and fee'd self-transfers — so the residual basis is exactly the acquired basis: the precise C1 check.
fn arb_events_no_pending_basis() -> impl Strategy<Value = Vec<LedgerEvent>> {
    let step = prop_oneof![
        (1_000i64..5_000_000, 1i64..2_000_000)
            .prop_map(|(sat, cents)| Step::Acquire { sat, cents }),
        (1_000i64..5_000_000, 0i64..500).prop_map(|(sat, fee)| Step::SelfXfer { sat, fee }),
    ];
    prop::collection::vec(step, 1..8).prop_map(|s| build(&s))
}

// ── property tests ───────────────────────────────────────────────────────────────────────────────

proptest! {
    /// FR9 sat-conservation identity holds for any generated event set when there are no uncovered disposals.
    #[test]
    fn conservation_holds_when_no_uncovered(evs in arb_events()) {
        let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
        let r = btctax_core::conservation_report(&st);
        if !r.has_uncovered {
            prop_assert_eq!(r.sigma_in, r.sigma_disposed + r.sigma_removed + r.sigma_held + r.sigma_fee_sats + r.sigma_pending);
            prop_assert!(r.balanced);
        }
    }

    /// No lot may have a negative remaining_sat, and no wallet balance may go negative, for any inputs.
    #[test]
    fn no_negative_remainders_ever(evs in arb_events()) {
        let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
        prop_assert!(st.lots.iter().all(|l| l.remaining_sat >= 0));
        prop_assert!(st.holdings_by_wallet.values().all(|&h| h >= 0));
    }

    /// C1: with only acquires + fee'd self-transfers, NO basis may be dropped — Σ remaining lot basis must
    /// equal Σ acquired basis EXACTLY (the pre-fix bug leaked the fee-sats' fragment, e.g. $60.00 -> $59.88).
    #[test]
    fn sigma_lot_basis_conserved_through_feed_self_transfers(evs in arb_events_no_pending_basis()) {
        let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
        let acquired: Decimal = evs.iter().filter_map(|e| match &e.payload {
            EventPayload::Acquire(a) => Some(a.usd_cost + a.fee_usd), _ => None,
        }).sum();
        let remaining: Decimal = st.lots.iter().map(|l| l.usd_basis).sum();
        prop_assert_eq!(acquired, remaining);
    }
}

// ── golden KAT ───────────────────────────────────────────────────────────────────────────────────

/// Deterministic hand-built scenario:
///   Pre-2025  : buy  200_000 sat @ $100.00 in wal_a                  (2024-06-01)
///   Post-2025 : income 10_000 sat @ $5.00 in wal_a                   (2025-02-01)
///               self-transfer wal_a→wal_b 80_000 sat + 200 sat fee   (2025-03-01)  [TP8(c)]
///               gift-out 40_000 sat from wal_b (FMV $20.00)          (2025-04-01)
///               sell 30_000 sat from wal_b @ $15.00                  (2025-05-01)
///
/// Conservation:
///   Σin      = 200_000 + 10_000 = 210_000
///   disposed = 30_000
///   removed  = 40_000
///   fee_sats = 200
///   pending  = 0
///   held     = wal_a (129_800) + wal_b (10_000) = 139_800
///   Check    : 30_000 + 40_000 + 139_800 + 200 + 0 = 210_000  ✓
#[test]
fn golden_kat_cross_boundary_conservation() {
    use rust_decimal_macros::dec;

    let wal_a = WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    };
    let wal_b = WalletId::SelfCustody {
        label: "cold".into(),
    };

    fn imp(
        src_ref: &str,
        ts: time::OffsetDateTime,
        wallet: Option<WalletId>,
        p: EventPayload,
    ) -> LedgerEvent {
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
            utc_timestamp: ts,
            original_tz: offset!(+00:00),
            wallet,
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

    let wa = Some(wal_a.clone());
    let wb = Some(wal_b.clone());

    // 1. Pre-2025 buy
    let buy = imp(
        "BUY",
        datetime!(2024-06-01 00:00:00 UTC),
        wa.clone(),
        EventPayload::Acquire(Acquire {
            sat: 200_000,
            usd_cost: dec!(100.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );

    // 2. Post-2025 income (FMV provided inline)
    let income = imp(
        "INCOME",
        datetime!(2025-02-01 00:00:00 UTC),
        wa.clone(),
        EventPayload::Income(Income {
            sat: 10_000,
            usd_fmv: Some(dec!(5.00)),
            fmv_status: FmvStatus::ExchangeProvided,
            kind: IncomeKind::Interest,
            business: false,
        }),
    );

    // 3. Self-transfer: TransferOut from wal_a + TransferIn at wal_b + TransferLink decision
    let xfer_out = imp(
        "XOUT",
        datetime!(2025-03-01 00:00:00 UTC),
        wa.clone(),
        EventPayload::TransferOut(TransferOut {
            sat: 80_000,
            fee_sat: Some(200),
            dest_addr: None,
            txid: None,
        }),
    );
    let xfer_in = imp(
        "XIN",
        datetime!(2025-03-01 00:00:00 UTC),
        wb.clone(),
        EventPayload::TransferIn(TransferIn {
            sat: 80_000,
            src_addr: None,
            txid: None,
        }),
    );
    let link = dec_ev(
        1,
        datetime!(2025-03-02 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("XOUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Coinbase,
                SourceRef::new("XIN"),
            )),
        }),
    );

    // 4. Gift-out from wal_b via ReclassifyOutflow
    let gift_out_tx = imp(
        "GOUT",
        datetime!(2025-04-01 00:00:00 UTC),
        wb.clone(),
        EventPayload::TransferOut(TransferOut {
            sat: 40_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let gift_dec = dec_ev(
        2,
        datetime!(2025-04-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("GOUT")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(20.00),
            fee_usd: None,
            donee: None,
        }),
    );

    // 5. Sell from wal_b
    let sell = imp(
        "SELL",
        datetime!(2025-05-01 00:00:00 UTC),
        wb.clone(),
        EventPayload::Dispose(Dispose {
            sat: 30_000,
            usd_proceeds: dec!(15.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );

    let events = vec![
        buy,
        income,
        xfer_out,
        xfer_in,
        link,
        gift_out_tx,
        gift_dec,
        sell,
    ];
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // ── pinned structure ──────────────────────────────────────────────────────────────────────────
    // 1 sell disposal (30k sat, !fee_mini_disposition)
    assert_eq!(st.disposals.len(), 1, "should have exactly one disposal");
    assert!(!st.disposals[0].fee_mini_disposition);
    let disp_sat: i64 = st.disposals[0].legs.iter().map(|l| l.sat).sum();
    assert_eq!(disp_sat, 30_000);

    // 1 removal (gift, 40k sat)
    assert_eq!(st.removals.len(), 1, "should have exactly one removal");
    let rem_sat: i64 = st.removals[0].legs.iter().map(|l| l.sat).sum();
    assert_eq!(rem_sat, 40_000);

    // 1 income record
    assert_eq!(
        st.income_recognized.len(),
        1,
        "should have exactly one income record"
    );
    assert_eq!(st.income_recognized[0].sat, 10_000);
    assert_eq!(st.income_recognized[0].usd_fmv, dec!(5.00));

    // holdings: wal_a=129_800, wal_b=10_000
    assert_eq!(st.holdings_by_wallet[&wal_a], 129_800, "wal_a holdings");
    assert_eq!(st.holdings_by_wallet[&wal_b], 10_000, "wal_b holdings");

    // no uncovered-disposal blocker
    assert!(
        !st.blockers
            .iter()
            .any(|b| b.kind == btctax_core::state::BlockerKind::UncoveredDisposal),
        "unexpected UncoveredDisposal blocker"
    );

    // ── FR9 conservation ─────────────────────────────────────────────────────────────────────────
    let r = btctax_core::conservation_report(&st);
    assert_eq!(r.sigma_in, 210_000, "sigma_in");
    assert_eq!(r.sigma_disposed, 30_000, "sigma_disposed");
    assert_eq!(r.sigma_removed, 40_000, "sigma_removed");
    assert_eq!(r.sigma_fee_sats, 200, "sigma_fee_sats");
    assert_eq!(r.sigma_pending, 0, "sigma_pending");
    assert_eq!(r.sigma_held, 139_800, "sigma_held");
    assert!(!r.has_uncovered, "has_uncovered must be false");
    assert!(r.balanced, "conservation_report must be balanced");
    assert_eq!(
        r.sigma_in,
        r.sigma_disposed + r.sigma_removed + r.sigma_held + r.sigma_fee_sats + r.sigma_pending,
        "FR9 identity"
    );
}
