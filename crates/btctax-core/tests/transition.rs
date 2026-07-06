use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{
    conservation_report, project, FeeTreatment, LotMethod, ProjectionConfig,
};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use std::collections::BTreeSet;
use time::macros::{date, datetime, offset};

fn cb() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn imp(
    src: Source,
    src_ref: &str,
    ts: time::OffsetDateTime,
    w: WalletId,
    p: EventPayload,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(src, SourceRef::new(src_ref)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w),
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
fn buy(
    src_ref: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    cost: rust_decimal::Decimal,
) -> LedgerEvent {
    imp(
        Source::Coinbase,
        src_ref,
        ts,
        cb(),
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}
fn sell(
    src_ref: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    proceeds: rust_decimal::Decimal,
) -> LedgerEvent {
    imp(
        Source::Coinbase,
        src_ref,
        ts,
        cb(),
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
/// [reconcile-defaults] A global FIFO standing order effective at TRANSITION_DATE — pins the post-2025
/// disposal method to FIFO so these tests keep their pre-change (FIFO) lot ordering after the default
/// flipped to HIFO. Made-date == effective_from (2025-01-01) → not backdated.
fn elect_fifo(seq: u64) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2025-01-01 00:00:00 UTC),
        EventPayload::MethodElection(MethodElection {
            effective_from: date!(2025 - 01 - 01),
            method: LotMethod::Fifo,
            wallet: None,
        }),
    )
}
#[allow(clippy::too_many_arguments)]
fn alloc(
    seq: u64,
    made: time::OffsetDateTime,
    method: AllocMethod,
    attested: bool,
    pre2025_method: LotMethod,
    lots: Vec<AllocLot>,
) -> LedgerEvent {
    dec_ev(
        seq,
        made,
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots,
            as_of_date: date!(2025 - 01 - 01),
            method,
            timely_allocation_attested: attested,
            pre2025_method,
        }),
    )
}
fn alloc_lot(w: WalletId, sat: i64, basis: rust_decimal::Decimal, acq: time::Date) -> AllocLot {
    AllocLot {
        wallet: w,
        sat,
        usd_basis: basis,
        acquired_at: acq,
        dual_loss_basis: None,
        donor_acquired_at: None,
    }
}
fn alloc_lot_dual(
    w: WalletId,
    sat: i64,
    basis: rust_decimal::Decimal,
    acq: time::Date,
    dual_loss_basis: Option<rust_decimal::Decimal>,
    donor_acquired_at: Option<time::Date>,
) -> AllocLot {
    AllocLot {
        wallet: w,
        sat,
        usd_basis: basis,
        acquired_at: acq,
        dual_loss_basis,
        donor_acquired_at,
    }
}
fn gift_recv(src_ref: &str, ts: time::OffsetDateTime, sat: i64) -> LedgerEvent {
    imp(
        Source::Coinbase,
        src_ref,
        ts,
        cb(),
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        }),
    )
}
fn classify_gift(
    seq: u64,
    ts: time::OffsetDateTime,
    src_ref: &str,
    donor_basis: Option<rust_decimal::Decimal>,
    fmv_at_gift: rust_decimal::Decimal,
    donor_acq: Option<time::Date>,
) -> LedgerEvent {
    dec_ev(
        seq,
        ts,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
            as_: InboundClass::GiftReceived {
                donor_basis,
                donor_acquired_at: donor_acq,
                fmv_at_gift,
            },
        }),
    )
}
fn has(st: &LedgerState, k: BlockerKind) -> bool {
    st.blockers.iter().any(|b| b.kind == k)
}

// (i) ActualPosition: a first-2025 disposition BEFORE the made-date bars at the earlier-of -> inert + timebar + Path A.
#[test]
fn actual_position_barred_by_earlier_first_disposition_is_inert_path_a() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        sell("S", datetime!(2025-02-01 00:00:00 UTC), 40_000, dec!(30.00)), // first 2025 disposition
        alloc(
            1,
            datetime!(2025-03-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false, // made AFTER the sell
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::SafeHarborTimebar)); // advisory
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable)); // conservation passed; only the bar tripped
    assert_eq!(st.holdings_by_wallet[&cb()], 60_000);
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A governs
}

// (ii) ActualPosition: no 2025 disposition, made-date AFTER 2026-04-15 (return-due prong) -> inert + timebar.
#[test]
fn actual_position_barred_by_return_due_date_is_inert() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        buy(
            "B25",
            datetime!(2025-02-01 00:00:00 UTC),
            50_000,
            dec!(40.00),
        ), // a 2025 ACQUIRE triggers the seed but is NOT a disposition
        alloc(
            1,
            datetime!(2026-05-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false, // made after 2026-04-15 (no 2025 disposition -> return-due prong governs)
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::SafeHarborTimebar)); // barred at 2026-04-15 (return-due prong)
                                                       // Path A governs: the pre-2025 lot is reconstructed; NO lot is safe-harbor-seeded (the 2025 buy is a normal lot).
    assert!(st
        .lots
        .iter()
        .any(|l| l.basis_source == BasisSource::ReconstructedPerWallet));
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source != BasisSource::SafeHarborAllocated));
}

// (iii) Attestation bypasses BOTH prongs -> Path B governs (no timebar fires).
#[test]
fn attestation_bypasses_the_bar_path_b_governs() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        sell("S", datetime!(2025-02-01 00:00:00 UTC), 40_000, dec!(30.00)),
        alloc(
            1,
            datetime!(2025-03-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true, // attested
            LotMethod::Fifo,
            vec![
                alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 06 - 01)),
                alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 07 - 01)),
            ],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::SafeHarborTimebar)); // attestation suppresses it
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B
    assert_eq!(st.holdings_by_wallet[&cb()], 60_000);
}

// (iv) A confirmed self-transfer (default (c)) dated before the made-date does NOT trip prong (a).
#[test]
fn confirmed_self_transfer_does_not_trip_the_bar() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        imp(
            Source::Coinbase,
            "OUT",
            datetime!(2025-01-15 00:00:00 UTC),
            cb(),
            EventPayload::TransferOut(TransferOut {
                sat: 100_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            Source::Swan,
            "IN",
            datetime!(2025-01-15 01:00:00 UTC),
            cold(),
            EventPayload::TransferIn(TransferIn {
                sat: 100_000,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            1,
            datetime!(2026-01-01 00:00:00 UTC),
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Swan,
                    SourceRef::new("IN"),
                )),
            }),
        ),
        // made-date after the self-transfer but BEFORE 2026-04-15; (c) self-transfer is no disposition -> bar = return-due -> effective.
        // The coins were on `cb` at 2025-01-01 (the 2025 self-transfer moves them later), so the allocation assigns cb.
        alloc(
            2,
            datetime!(2025-06-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::SafeHarborTimebar)); // effective: the (c) self-transfer did NOT trip prong (a)
    assert!(!has(&st, BlockerKind::UncoveredDisposal));
    assert!(st.disposals.is_empty()); // the self-transfer is non-taxable (TP7)
    assert_eq!(st.holdings_by_wallet[&cold()], 100_000); // coins relocated cb -> cold post-seed
}

// (v) Conservation mismatch -> HARD safe_harbor_unconservable, NOT timebar; falls back to Path A.
#[test]
fn conservation_mismatch_is_hard_unconservable_not_timebar() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        sell("S", datetime!(2025-06-01 00:00:00 UTC), 30_000, dec!(24.00)), // made-date is BEFORE this -> bar not tripped
        alloc(
            1,
            datetime!(2025-02-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 90_000, dec!(54.00), date!(2024 - 06 - 01))], // Σsat 90k != 100k
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::SafeHarborUnconservable)); // hard
    assert!(!has(&st, BlockerKind::SafeHarborTimebar)); // the bar was not the failure
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A fallback
}

// (vi) Void of an EFFECTIVE allocation -> decision_conflicts; the allocation STAYS in force (irrevocable, §7.4(2)).
#[test]
fn void_of_effective_allocation_is_a_decision_conflict() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        sell("S", datetime!(2025-06-01 00:00:00 UTC), 30_000, dec!(24.00)),
        alloc(
            1,
            datetime!(2025-02-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true, // effective
            LotMethod::Fifo,
            vec![
                alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 06 - 01)),
                alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 07 - 01)),
            ],
        ),
        dec_ev(
            2,
            datetime!(2026-01-01 00:00:00 UTC),
            EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                target_event_id: EventId::decision(1),
            }),
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::DecisionConflict));
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B still governs
}

// (vii) Void of an INERT allocation -> the void APPLIES (no decision_conflicts); stays Path A.
#[test]
fn void_of_inert_allocation_applies_no_conflict() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        sell("S", datetime!(2025-02-01 00:00:00 UTC), 40_000, dec!(30.00)), // bars the ActualPosition alloc made later
        alloc(
            1,
            datetime!(2025-03-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false, // inert (timebar)
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
        dec_ev(
            2,
            datetime!(2026-01-01 00:00:00 UTC),
            EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                target_event_id: EventId::decision(1),
            }),
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::DecisionConflict)); // void of an inert/revocable allocation is valid
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A
}

// (viii) Re-evaluation is a pure function of the SET: a ReclassifyOutflow that creates a first-2025 disposition
//        before the made-date flips the SAME allocation effective -> inert deterministically.
#[test]
fn reclassify_creating_a_disposition_flips_effective_to_inert() {
    let base = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        imp(
            Source::Coinbase,
            "OUT",
            datetime!(2025-02-01 00:00:00 UTC),
            cb(),
            EventPayload::TransferOut(TransferOut {
                sat: 40_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        sell(
            "TRIG",
            datetime!(2025-09-01 00:00:00 UTC),
            1_000,
            dec!(1.00),
        ), // a post-made-date seed trigger
        alloc(
            1,
            datetime!(2025-03-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
    ];
    // Variant 1: OUT left unclassified -> provisional (pending), no first-2025 disposition before made-date -> EFFECTIVE.
    let st1 = project(
        &base,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st1
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::SafeHarborAllocated));
    assert!(!has(&st1, BlockerKind::SafeHarborTimebar));
    // Variant 2: reclassify OUT -> Dispose at 2025-02-01 (before made-date 2025-03-01) -> earlier-of bar trips -> INERT.
    let mut v2 = base.clone();
    v2.push(dec_ev(
        2,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(35.00),
            fee_usd: None,
            donee: None,
        }),
    ));
    let st2 = project(&v2, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st2, BlockerKind::SafeHarborTimebar));
    assert!(st2
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A now
}

// (ix) Path A across mixed vintages: a pre-2025 lot reconstructed at the boundary keeps its acquired_at, so a
//      2025 disposition of it is LONG-TERM; conservation balances.
#[test]
fn path_a_mixed_vintages_post_2025_term_and_conservation() {
    let evs = vec![
        buy(
            "OLD",
            datetime!(2024-06-01 00:00:00 UTC),
            50_000,
            dec!(30.00),
        ),
        buy(
            "NEW",
            datetime!(2025-03-01 00:00:00 UTC),
            50_000,
            dec!(40.00),
        ),
        sell("S", datetime!(2025-08-01 00:00:00 UTC), 50_000, dec!(60.00)), // FIFO consumes the 2024 lot
        elect_fifo(1), // pin FIFO (post-2025 default is now HIFO) so the older 2024 lot is consumed
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.term, Term::LongTerm); // tacks from 2024-06-01 across the boundary
    assert_eq!(leg.basis, dec!(30.00));
    // conservation (computed inline; `conservation_report` itself lands in Task 13): in == disposed + held.
    let disposed: i64 = st
        .disposals
        .iter()
        .flat_map(|d| &d.legs)
        .map(|l| l.sat)
        .sum();
    let held: i64 = st.lots.iter().map(|l| l.remaining_sat).sum();
    assert_eq!(disposed + held, 100_000);
    assert_eq!(st.holdings_by_wallet[&cb()], 50_000);
}

// (x) §6.1 calendar-date boundary: a UTC-2025 disposition whose original_tz date is 2024 is PRE-2025 (counts
//     toward neither the first-2025-disposition trigger nor the seed), so a 2025 allocation stays effective.
#[test]
fn calendar_date_boundary_keeps_a_2024_local_disposition_pre_2025() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        // 2025-01-01 02:00 UTC is 2024-12-31 in UTC-05:00 -> a PRE-2025 disposal (Universal pool; pre2025_method_note).
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("PRE")),
            utc_timestamp: datetime!(2025-01-01 02:00:00 UTC),
            original_tz: offset!(-05:00),
            wallet: Some(cb()),
            payload: EventPayload::Dispose(Dispose {
                sat: 20_000,
                usd_proceeds: dec!(15.00),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        },
        sell("S", datetime!(2025-09-01 00:00:00 UTC), 10_000, dec!(9.00)), // a real 2025 seed trigger, AFTER made-date
        alloc(
            1,
            datetime!(2025-03-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 80_000, dec!(48.00), date!(2024 - 06 - 01))], // conserves to Universal-after-pre2025 sale
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::Pre2025MethodNote)); // the 2024-local sale folded pre-2025
                                                       // Verify the Pre2025MethodNote detail is actionable: contains FIFO assumption and verification guidance.
    let note = st
        .blockers
        .iter()
        .find(|b| b.kind == BlockerKind::Pre2025MethodNote)
        .expect("Pre2025MethodNote blocker should exist");
    assert!(
        note.detail.contains("FIFO"),
        "Pre2025MethodNote detail must mention FIFO assumption, got: {}",
        note.detail
    );
    // D2 (Task 2): unattested advisory now names the actionable declaration command rather than
    // generic "verify" guidance — check for the updated actionable text.
    // M1 (review Minor): fixture uses ProjectionConfig::default() → attested=false, so ONLY the
    // unattested branch fires; the OR was over-permissive — tighten to the exact expected branch.
    assert!(
        note.detail.contains("have NOT declared"),
        "Pre2025MethodNote detail must contain 'have NOT declared' (unattested branch), got: {}",
        note.detail
    );
    assert!(!has(&st, BlockerKind::SafeHarborTimebar)); // it is NOT a first-2025 disposition -> bar not tripped
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
}

// (xi) Path-A default fallback when NO allocation exists at all.
#[test]
fn path_a_is_the_default_with_no_allocation() {
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        sell("S", datetime!(2025-06-01 00:00:00 UTC), 40_000, dec!(30.00)),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(
        !has(&st, BlockerKind::SafeHarborTimebar)
            && !has(&st, BlockerKind::SafeHarborUnconservable)
    );
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source == BasisSource::ReconstructedPerWallet)); // Path A
    assert_eq!(st.holdings_by_wallet[&cb()], 60_000);
}

// (xii) Eng-review Minor: the boundary seed partitions by TAX-DATE, not raw UTC order. A pre-2025 lot whose
//       UTC instant lands AFTER an early-UTC post-2025 event must still fold into the Universal residue and be
//       seeded — a reversed-offset straddle of 2025-01-01. Proves `universal_snapshot` matches the pre-seed residue.
#[test]
fn reversed_offset_straddle_seeds_on_tax_date_not_utc_order() {
    let evs = vec![
        // post-2025 TAX-DATE but EARLIER utc (the seed trigger): 2025-01-01 00:00 UTC, +00:00 -> 2025-01-01.
        buy(
            "NEW",
            datetime!(2025-01-01 00:00:00 UTC),
            50_000,
            dec!(40.00),
        ),
        // pre-2025 TAX-DATE but LATER utc: 2025-01-01 03:00 UTC, -05:00 -> 2024-12-31.
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("OLD")),
            utc_timestamp: datetime!(2025-01-01 03:00:00 UTC),
            original_tz: offset!(-05:00),
            wallet: Some(cb()),
            payload: EventPayload::Acquire(Acquire {
                sat: 50_000,
                usd_cost: dec!(30.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        },
        // a 2025 sale that must FIFO-consume the pre-2025 (OLD) lot FIRST (acq 2024-12-31 < 2025-01-01).
        sell("S", datetime!(2026-06-01 00:00:00 UTC), 50_000, dec!(80.00)),
        elect_fifo(1), // pin FIFO (post-2025 default is now HIFO) so the older OLD lot is consumed first
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::UncoveredDisposal)); // OLD seeded into cb, not stranded in Universal
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.basis, dec!(30.00)); // FIFO consumed the pre-2025 OLD lot, not NEW
    assert_eq!(leg.term, Term::LongTerm); // tacks from 2024-12-31 across the boundary
    assert_eq!(leg.basis_source, BasisSource::ReconstructedPerWallet); // OLD was drained at the seed (Path A)
    assert_eq!(st.holdings_by_wallet[&cb()], 50_000); // only NEW remains (ExchangeProvided)
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source != BasisSource::SafeHarborAllocated)); // Path A, no safe-harbor lots
    let disposed: i64 = st
        .disposals
        .iter()
        .flat_map(|d| &d.legs)
        .map(|l| l.sat)
        .sum();
    let held: i64 = st.lots.iter().map(|l| l.remaining_sat).sum();
    assert_eq!(disposed + held, 100_000);
}

// (xiii) TP8(b) self-transfer fee mini-disposition DOES trip prong (a): an early-2025 fee mini-disposition
//        before the made-date bars an ActualPosition allocation -> inert + timebar + Path A.
#[test]
fn tp8b_self_transfer_fee_mini_disposition_trips_the_bar() {
    let cfg = ProjectionConfig {
        self_transfer_fee: FeeTreatment::TreatmentB,
        ..ProjectionConfig::default()
    };
    let evs = vec![
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        imp(
            Source::Coinbase,
            "OUT",
            datetime!(2025-01-15 00:00:00 UTC),
            cb(),
            EventPayload::TransferOut(TransferOut {
                sat: 90_000,
                fee_sat: Some(500), // (b): the fee-sats are a mini-disposition at 2025-01-15
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            Source::Swan,
            "IN",
            datetime!(2025-01-15 01:00:00 UTC),
            cold(),
            EventPayload::TransferIn(TransferIn {
                sat: 90_000,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            1,
            datetime!(2026-01-01 00:00:00 UTC),
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Swan,
                    SourceRef::new("IN"),
                )),
            }),
        ),
        // made AFTER the 2025-01-15 fee mini-disposition; under (b) that mini-disposition IS a first-2025
        // disposition -> ActualPosition bar = 2025-01-15 -> made 2025-06-01 is past it -> inert + timebar.
        alloc(
            2,
            datetime!(2025-06-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(has(&st, BlockerKind::SafeHarborTimebar)); // (b) fee mini-disposition tripped prong (a)
                                                       // Path A fallback: no lot is safe-harbor-seeded (relocated lots are CarriedFromTransfer).
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source != BasisSource::SafeHarborAllocated));
}

// (xiv) I-2 fix: Path-B seeded lots + post-2025 SelfTransfer relocation — no LotId collision.
// Seed lots occupy split_sequence 0..seed_len-1. Without the fix, bump_split(allocation_id)
// returns 0 (colliding with seed lot 0). With the fix, it returns seed_len (fresh unique index).
#[test]
fn path_b_seeded_lot_relocation_no_lotid_collision() {
    let evs = vec![
        // Pre-2025 buy: feeds the Universal snapshot for conservation (100k sats / $60).
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        // Effective Path-B allocation (attested): two lots totalling 100k sats / $60 (conserves).
        alloc(
            1,
            datetime!(2025-02-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true, // attested → effective regardless of first-disposition timing
            LotMethod::Fifo,
            vec![
                alloc_lot(cb(), 60_000, dec!(36.00), date!(2024 - 01 - 01)),
                alloc_lot(cb(), 40_000, dec!(24.00), date!(2024 - 06 - 01)),
            ],
        ),
        // Post-2025 SelfTransfer: partially relocate 30k sats from cb() to cold().
        // FIFO consumes from seed lot 0 (seq=0, 60k sats); the relocated fragment
        // must get split_sequence >= seed.len() = 2 (I-2 fix).
        imp(
            Source::Coinbase,
            "OUT",
            datetime!(2025-06-01 00:00:00 UTC),
            cb(),
            EventPayload::TransferOut(TransferOut {
                sat: 30_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            Source::Swan,
            "IN",
            datetime!(2025-06-01 01:00:00 UTC),
            cold(),
            EventPayload::TransferIn(TransferIn {
                sat: 30_000,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            2,
            datetime!(2026-01-01 00:00:00 UTC),
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Swan,
                    SourceRef::new("IN"),
                )),
            }),
        ),
    ];

    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());

    // Path-B effective; no blocking errors.
    assert!(!has(&st, BlockerKind::SafeHarborTimebar));
    assert!(!has(&st, BlockerKind::UncoveredDisposal));
    assert!(!has(&st, BlockerKind::DecisionConflict));
    assert!(st.disposals.is_empty() && st.removals.is_empty()); // non-taxable self-transfer

    // I-2: All LotIds in final state must be UNIQUE (no collision between seeded and relocated).
    // Expected: {alloc_id, seq=0} (30k rem in cb), {alloc_id, seq=1} (40k in cb),
    //           {alloc_id, seq=2} (30k in cold). seq=2 = seed.len(), NOT 0.
    let lot_ids: Vec<_> = st.lots.iter().map(|l| l.lot_id.clone()).collect();
    let unique: BTreeSet<_> = lot_ids.iter().collect();
    assert_eq!(
        unique.len(),
        lot_ids.len(),
        "I-2: all LotIds must be unique after Path-B seed + SelfTransfer relocation (no collision)"
    );
    assert_eq!(
        st.lots.len(),
        3,
        "seed lot 0 (partial), seed lot 1, relocated fragment"
    );

    // Conservation holds: all 100k sats still tracked across both wallets.
    let report = conservation_report(&st);
    assert!(
        report.balanced,
        "conservation balanced after Path-B seed + SelfTransfer: {report:?}"
    );
    assert_eq!(report.sigma_held, 100_000);
    assert_eq!(st.holdings_by_wallet[&cb()], 70_000); // 30k (lot 0 remainder) + 40k (lot 1)
    assert_eq!(st.holdings_by_wallet[&cold()], 30_000); // relocated fragment
}

// ── Slug 1: AllocLot dual-basis + tacking preservation ─────────────────────────────────────────

/// Path-B seed must preserve the §1015(a) dual basis (GAIN=donor carryover, LOSS=FMV-at-gift) so
/// a post-2025 loss-zone disposal uses the FMV-at-gift loss basis, not the donor/gain basis.
/// Scenario: gift lot (100k sat, donor basis $100, FMV-at-gift $40). Dispose at $30 (loss zone:
/// $30 < $40). Expected: loss = $10 (basis=$40), NOT $70 (which would result from single basis=$100).
/// Under the OLD code `dual_loss_basis: None` ⇒ single-basis ⇒ loss $70 — this test proves the fix.
#[test]
fn path_b_preserves_gift_dual_loss_basis() {
    let evs = vec![
        // Pre-2025 gift received (TransferIn + ClassifyInbound → GiftReceived dual lot):
        // gain basis = $100 (donor carryover), loss basis = $40 (FMV-at-gift), gift date = 2024-06-01.
        gift_recv("gift1", datetime!(2024-06-01 00:00:00 UTC), 100_000),
        classify_gift(
            1,
            datetime!(2024-06-15 00:00:00 UTC),
            "gift1",
            Some(dec!(100.00)),          // donor (gain) basis
            dec!(40.00),                 // FMV-at-gift (loss basis, since < donor basis)
            Some(date!(2021 - 01 - 01)), // donor_acquired_at
        ),
        // Path-B allocation made timely (2024-12-01 is before any 2025 dispose).
        // The AllocLot carries the dual basis fields.
        alloc(
            2,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot_dual(
                cb(),
                100_000,
                dec!(100.00),                // usd_basis = GAIN basis = donor carryover
                date!(2024 - 06 - 01),       // acquired_at = gift date (loss-zone HP start)
                Some(dec!(40.00)),           // dual_loss_basis = LOSS basis = FMV-at-gift
                Some(date!(2021 - 01 - 01)), // donor_acquired_at for gain-zone tacking
            )],
        ),
        // Post-2025 disposal in the LOSS zone: proceeds $30 < FMV-at-gift $40.
        sell(
            "D1",
            datetime!(2025-09-01 00:00:00 UTC),
            100_000,
            dec!(30.00),
        ),
    ];
    // [reconcile-defaults] The allocation records pre2025_method Fifo; keep the live config in sync so it
    // does not fire Pre2025MethodConflictsAllocation against the now-HIFO default.
    let st = project(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig {
            pre2025_method: LotMethod::Fifo,
            ..ProjectionConfig::default()
        },
    );

    // No blockers: allocation is effective (timely, conserved); no uncovered disposals.
    assert!(
        st.blockers.is_empty(),
        "unexpected blockers: {:?}",
        st.blockers
    );

    // The Path-B seed must have preserved dual_loss_basis and donor_acquired_at.
    // (In Path B, original lots are discarded and the seeded lots govern; no residual lots.)
    assert_eq!(st.disposals.len(), 1, "one disposal");
    let leg = &st.disposals[0].legs[0];

    // Loss zone: basis must be the FMV-at-gift LOSS basis ($40), not the donor/gain basis ($100).
    assert_eq!(
        leg.basis,
        dec!(40.00),
        "loss-zone basis must be FMV-at-gift $40, not donor basis $100; got {}",
        leg.basis
    );
    // Loss = proceeds $30 - loss basis $40 = -$10.
    assert_eq!(
        leg.gain,
        dec!(-10.00),
        "loss must be $10 (proceeds $30 - FMV-at-gift basis $40); got {}",
        leg.gain
    );
    // Zone is confirmed Loss.
    assert_eq!(leg.gift_zone, Some(GiftZone::Loss));
}

/// Path-B seed must preserve §1223(2) donor tacking (`donor_acquired_at`) so that a gain-zone
/// disposal uses the donor's HP start date, not the gift date.
/// Scenario: same dual gift lot (donor_acquired_at=2021-01-01, gift_date=2024-06-01).
/// Dispose on 2025-03-01 in the GAIN zone (proceeds $150 > gain basis $100 → gain $50).
/// 2025-03-01 is >1yr after donor_acquired_at (2021-01-01) → LONG-TERM via tacking.
/// Without tacking (old code, donor_acquired_at=None): 2025-03-01 < 2025-06-01 → SHORT-TERM.
#[test]
fn path_b_preserves_gift_tacking() {
    let evs = vec![
        gift_recv("gift2", datetime!(2024-06-01 00:00:00 UTC), 100_000),
        classify_gift(
            1,
            datetime!(2024-06-15 00:00:00 UTC),
            "gift2",
            Some(dec!(100.00)),
            dec!(40.00),
            Some(date!(2021 - 01 - 01)),
        ),
        // Alloc made at 2024-12-01, before the 2025-03-01 disposal → timely (ActualPosition bar =
        // earlier of first-2025-dispose 2025-03-01 and return-due 2026-04-15 = 2025-03-01).
        alloc(
            2,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            false,
            LotMethod::Fifo,
            vec![alloc_lot_dual(
                cb(),
                100_000,
                dec!(100.00),
                date!(2024 - 06 - 01),
                Some(dec!(40.00)),
                Some(date!(2021 - 01 - 01)),
            )],
        ),
        // Post-2025 disposal in the GAIN zone on 2025-03-01:
        //   proceeds $150 > gain basis $100 → gain $50.
        //   HP from donor_acquired_at 2021-01-01 to 2025-03-01: >1yr → LONG-TERM.
        //   HP from gift date 2024-06-01 to 2025-03-01: <1yr → SHORT-TERM (old, wrong behavior).
        sell(
            "D2",
            datetime!(2025-03-01 00:00:00 UTC),
            100_000,
            dec!(150.00),
        ),
    ];
    // [reconcile-defaults] Allocation records pre2025_method Fifo → keep the live config in sync (else the
    // now-HIFO default fires Pre2025MethodConflictsAllocation).
    let st = project(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig {
            pre2025_method: LotMethod::Fifo,
            ..ProjectionConfig::default()
        },
    );

    assert!(
        st.blockers.is_empty(),
        "unexpected blockers: {:?}",
        st.blockers
    );
    assert_eq!(st.disposals.len(), 1);
    let leg = &st.disposals[0].legs[0];

    // Gain zone: basis = gain basis $100, gain = $50.
    assert_eq!(
        leg.basis,
        dec!(100.00),
        "gain-zone basis must be $100; got {}",
        leg.basis
    );
    assert_eq!(leg.gain, dec!(50.00), "gain must be $50; got {}", leg.gain);
    assert_eq!(leg.gift_zone, Some(GiftZone::Gain));

    // LONG-TERM via tacking: donor_acquired_at 2021-01-01 → >1yr before 2025-03-01.
    // Under old code (donor_acquired_at: None) this would be SHORT-TERM (gift_date 2024-06-01 → <1yr).
    assert_eq!(
        leg.term,
        Term::LongTerm,
        "must be LONG-TERM via §1223(2) tacking from donor_acquired_at 2021-01-01; \
         without tacking (gift date 2024-06-01) this would be SHORT-TERM"
    );
}

/// Round-trip an `AllocLot` with `Some(..)` dual fields; and verify that JSON omitting the new
/// fields deserializes to `None` (backward compat for pre-existing persisted events, [R0-N1]).
#[test]
fn alloc_lot_serde_backward_compat() {
    let lot = alloc_lot_dual(
        cb(),
        100_000,
        dec!(100.00),
        date!(2024 - 06 - 01),
        Some(dec!(40.00)),
        Some(date!(2021 - 01 - 01)),
    );
    // Round-trip with Some values.
    let json = serde_json::to_string(&lot).unwrap();
    let round_tripped: AllocLot = serde_json::from_str(&json).unwrap();
    assert_eq!(lot, round_tripped);

    // Old persisted format (fields absent) → both None; `#[serde(default)]` makes this work.
    let mut val: serde_json::Value = serde_json::from_str(&json).unwrap();
    val.as_object_mut().unwrap().remove("dual_loss_basis");
    val.as_object_mut().unwrap().remove("donor_acquired_at");
    let old: AllocLot = serde_json::from_value(val).unwrap();
    assert_eq!(old.dual_loss_basis, None);
    assert_eq!(old.donor_acquired_at, None);
}
