//! TASK 6 — A.7: `pre2025_method` ↔ effective `SafeHarborAllocation`.
//!
//! The pre-2025 residue is computed under the allocation's *recorded* `pre2025_method` (not the live
//! config), so a non-FIFO filer's Path B conserves; a post-attestation live-config change is caught by
//! the dedicated hard `Pre2025MethodConflictsAllocation` (never the generic `SafeHarborUnconservable`),
//! and the irrevocable allocation (§7.4) is never rewritten.

use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, LotMethod, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

fn cb() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
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
fn has(st: &LedgerState, k: BlockerKind) -> bool {
    st.blockers.iter().any(|b| b.kind == k)
}

// Composition KAT: a non-FIFO (LIFO) pre-2025 residue -> Path B conserves under the recorded method.
// Pre-2025 buys A($30/50k, older) + B($50/50k, newer); a pre-2025 sell of 50k.
//   LIFO consumes B -> residue = A ($30/50k).  Allocation records pre2025_method=Lifo + that residue.
#[test]
fn lifo_residue_path_b_conserves_method_aware() {
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Lifo,
        ..ProjectionConfig::default()
    };
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 50_000, dec!(50.00)),
        sell("S", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(45.00)), // LIFO consumes B
        alloc(
            1,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true, // attested
            LotMethod::Lifo,
            vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 02 - 01))], // residue A under LIFO
        ),
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)), // a 2025 seed trigger
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(!has(&st, BlockerKind::Pre2025MethodConflictsAllocation));
    assert!(st
        .lots
        .iter()
        .any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
}

// Contrast: the SAME allocation residue ($30, recorded under LIFO) does NOT conserve under a FIFO
// recorded method (the FIFO residue is B = $50), proving the snapshot is genuinely method-aware.
#[test]
fn fifo_residue_differs_from_lifo_recorded_residue() {
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        ..ProjectionConfig::default()
    };
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 50_000, dec!(50.00)),
        sell("S", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(45.00)), // FIFO consumes A -> residue B ($50)
        // Allocation records FIFO but lists the LIFO residue ($30) -> does NOT conserve (FIFO residue is $50).
        alloc(
            1,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 02 - 01))],
        ),
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)),
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(
        has(&st, BlockerKind::SafeHarborUnconservable),
        "FIFO residue basis ($50) differs from the listed $30 -> unconservable"
    );
    assert!(st
        .lots
        .iter()
        .all(|l| l.basis_source != BasisSource::SafeHarborAllocated)); // Path A (inert)
}

// Conflict KAT: live config (Fifo) != the effective allocation's recorded method (Lifo) -> dedicated hard
// blocker, NOT SafeHarborUnconservable; Path B still governs (the irrevocable allocation pins the method).
#[test]
fn live_config_differs_from_recorded_method_is_pre2025_conflict() {
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        ..ProjectionConfig::default()
    }; // != recorded Lifo
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 50_000, dec!(50.00)),
        sell("S", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(45.00)),
        alloc(
            1,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true,
            LotMethod::Lifo,
            vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 02 - 01))], // residue A under LIFO
        ),
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)),
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(has(&st, BlockerKind::Pre2025MethodConflictsAllocation));
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable)); // method change is NOT misread as bad data
    assert!(st
        .lots
        .iter()
        .any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B still governs
}

// Conflict-clears KAT: reverting the live config to the recorded method clears the conflict; the
// irrevocable allocation is never rewritten (Path B governs in BOTH states) -> no §7.4 deadlock.
#[test]
fn reverting_live_config_to_recorded_method_clears_conflict() {
    let evs = vec![
        buy("A", datetime!(2024-02-01 00:00:00 UTC), 50_000, dec!(30.00)),
        buy("B", datetime!(2024-03-01 00:00:00 UTC), 50_000, dec!(50.00)),
        sell("S", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(45.00)),
        alloc(
            1,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true,
            LotMethod::Lifo,
            vec![alloc_lot(cb(), 50_000, dec!(30.00), date!(2024 - 02 - 01))],
        ),
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)),
    ];
    // Conflicting live config (Fifo): blocker present, allocation still effective.
    let conflicting = project(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig {
            pre2025_method: LotMethod::Fifo,
            ..ProjectionConfig::default()
        },
    );
    assert!(has(
        &conflicting,
        BlockerKind::Pre2025MethodConflictsAllocation
    ));
    let path_b_under_conflict = conflicting
        .lots
        .iter()
        .filter(|l| l.basis_source == BasisSource::SafeHarborAllocated)
        .count();
    assert!(
        path_b_under_conflict > 0,
        "Path B governs even under conflict"
    );

    // Revert live config to the recorded method (Lifo): conflict clears; allocation unchanged.
    let reverted = project(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig {
            pre2025_method: LotMethod::Lifo,
            ..ProjectionConfig::default()
        },
    );
    assert!(!has(
        &reverted,
        BlockerKind::Pre2025MethodConflictsAllocation
    ));
    assert!(!has(&reverted, BlockerKind::SafeHarborUnconservable));
    assert!(!has(&reverted, BlockerKind::DecisionConflict)); // §7.4 not disturbed; no deadlock
    let path_b_after_revert = reverted
        .lots
        .iter()
        .filter(|l| l.basis_source == BasisSource::SafeHarborAllocated)
        .count();
    // The irrevocable allocation is never rewritten: the same Path-B seed governs in both states.
    assert_eq!(path_b_under_conflict, path_b_after_revert);
}

// Backward-compat: a SafeHarborAllocation JSON without pre2025_method deserializes to Fifo.
#[test]
fn safe_harbor_allocation_pre2025_method_serde_default_fifo() {
    let a = SafeHarborAllocation {
        lots: vec![],
        as_of_date: date!(2025 - 01 - 01),
        method: AllocMethod::ActualPosition,
        timely_allocation_attested: false,
        pre2025_method: LotMethod::Hifo,
    };
    let mut v: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
    v.as_object_mut().unwrap().remove("pre2025_method");
    let old: SafeHarborAllocation = serde_json::from_value(v).unwrap();
    assert_eq!(old.pre2025_method, LotMethod::Fifo);
}

// ── C1 divergence KAT (b) — Path-B seeding in NON-acquired_at order; post-seed FIFO consumes oldest-first ──
// The allocation lists two SAME-WALLET seed lots newer-first; seed is pushed in alloc-index order, so the
// wallet pool's insertion order is [newer (split 0), older (split 1)]. A post-2025 partial FIFO Dispose MUST
// consume the OLDER lot first (acquisition-date FIFO), NOT seed-index 0 (which the legacy front-walk took).
#[test]
fn path_b_seed_in_non_acq_order_consumes_oldest_first_under_fifo() {
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        ..ProjectionConfig::default()
    };
    let evs = vec![
        // Pre-2025 Universal residue = 200k sat / $100 basis (FIFO, no pre-2025 disposal).
        buy(
            "U1",
            datetime!(2024-01-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ),
        buy(
            "U2",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ),
        // Allocation lists NEWER first (idx 0) and OLDER second (idx 1) — non-acquired_at order. Totals conserve.
        alloc(
            1,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true,
            LotMethod::Fifo,
            vec![
                alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01)), // seed split_sequence 0 (NEWER)
                alloc_lot(cb(), 100_000, dec!(40.00), date!(2024 - 01 - 01)), // seed split_sequence 1 (OLDER)
            ],
        ),
        sell(
            "D",
            datetime!(2025-09-01 00:00:00 UTC),
            100_000,
            dec!(120.00),
        ), // post-2025 partial FIFO Dispose in cb()
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(!has(&st, BlockerKind::SafeHarborUnconservable));
    assert!(st
        .lots
        .iter()
        .any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
    let leg = &st
        .disposals
        .iter()
        .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D")))
        .unwrap()
        .legs[0];
    assert_eq!(
        leg.basis,
        dec!(40.00),
        "acq-date FIFO consumes the OLDER seed lot; insertion-order would pick the newer index-0 lot"
    );
    assert_eq!(leg.lot_id.split_sequence, 1); // the OLDER lot was the one listed SECOND
}

// ── C1 divergence KAT (c) — pre-2025 SelfTransfer reorders the Universal pool; snapshot residue under acq-date ──
// A pre-2025 SelfTransfer relocates the OLDER lot to the BACK of the single Universal pool (insertion !=
// acquisition order). A pre-2025 partial disposal then consumes a DIFFERENT lot under acquisition-date FIFO
// (the older B1') than the legacy front-walk would (B2), so the conservation residue snap.basis differs ($60
// vs the legacy $40). An allocation built against the ACQUISITION-DATE-order residue ($60) must conserve.
#[test]
fn pre2025_self_transfer_reorders_universal_snapshot_residue_under_acq_date_fifo() {
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        ..ProjectionConfig::default()
    };
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let evs = vec![
        buy(
            "B1",
            datetime!(2024-01-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ), // OLDER, $40 (cb())
        buy(
            "B2",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(60.00),
        ), // NEWER, $60 (cb())
        // pre-2025 SelfTransfer: consume B1 (oldest) from Universal, re-push B1' to the BACK (still Universal, pre-2025).
        LedgerEvent {
            id: EventId::import(Source::Swan, SourceRef::new("OUT")),
            utc_timestamp: datetime!(2024-09-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(cb()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 100_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        },
        dec_ev(
            1,
            datetime!(2024-09-02 00:00:00 UTC),
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Swan, SourceRef::new("OUT")),
                in_event_or_wallet: TransferTarget::Wallet(cold.clone()),
            }),
        ), // pre-2025 dest -> still Universal pool
        // pre-2025 partial disposal: acq-date FIFO consumes the OLDER B1' (basis $40) -> residue = B2 ($60).
        sell(
            "D",
            datetime!(2024-10-01 00:00:00 UTC),
            100_000,
            dec!(70.00),
        ),
        // Allocation built against the acquisition-date residue ($60); recorded method matches live Fifo.
        alloc(
            2,
            datetime!(2024-12-01 00:00:00 UTC),
            AllocMethod::ActualPosition,
            true,
            LotMethod::Fifo,
            vec![alloc_lot(cb(), 100_000, dec!(60.00), date!(2024 - 06 - 01))],
        ),
        sell("S2", datetime!(2025-09-01 00:00:00 UTC), 1, dec!(0.01)), // post-2025 seed trigger
    ];
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert!(
        !has(&st, BlockerKind::SafeHarborUnconservable),
        "snapshot residue computed under acquisition-date FIFO is $60; the allocation conserves"
    );
    assert!(st
        .lots
        .iter()
        .any(|l| l.basis_source == BasisSource::SafeHarborAllocated)); // Path B effective
    let d = st
        .disposals
        .iter()
        .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D")))
        .unwrap();
    assert_eq!(d.legs[0].basis, dec!(40.00)); // the pre-2025 disposal consumed the OLDER relocated lot, not the front-of-Vec B2
}
