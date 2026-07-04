//! KATs for the shared voidable-candidate predicate (`btctax_core::voidable_decisions`) — the SINGLE
//! source of truth the single-void flow, the CLI `bulk_void_plan`, and the TUI `V` sweep all share
//! (SPEC_bulk_void §Candidate set / §KATs). These pin the exact filter chain (Decision-id ∧ not-voided
//! ∧ is_revocable_payload ∧ #7 !effective_alloc) at the crate where the predicate lives.

use btctax_core::event::{
    AllocMethod, ClassifyInbound, EventPayload, LedgerEvent, MethodElection, RejectImport,
    SafeHarborAllocation, SupersedeImport, VoidDecisionEvent,
};
use btctax_core::identity::{EventId, Source, SourceRef};
use btctax_core::state::{Blocker, BlockerKind};
use btctax_core::{
    is_revocable_payload, voidable_decisions, InboundClass, IncomeKind, LotMethod, WalletId,
};
use time::macros::{date, datetime};
use time::UtcOffset;

fn decision(seq: u64, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::Decision { seq },
        utc_timestamp: datetime!(2026-02-01 12:00:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: None,
        payload,
    }
}

fn import_ev(name: &str, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::River, SourceRef::new(name)),
        utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        }),
        payload,
    }
}

fn method_election(seq: u64) -> LedgerEvent {
    decision(
        seq,
        EventPayload::MethodElection(MethodElection {
            effective_from: date!(2024 - 01 - 01),
            method: LotMethod::Fifo,
        }),
    )
}

fn classify_inbound(seq: u64, target: EventId) -> LedgerEvent {
    decision(
        seq,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: target,
            as_: InboundClass::Income {
                kind: IncomeKind::Staking,
                fmv: None,
                business: false,
            },
        }),
    )
}

fn allocation(seq: u64) -> LedgerEvent {
    decision(
        seq,
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            method: AllocMethod::ActualPosition,
            as_of_date: date!(2025 - 01 - 01),
            lots: vec![],
            timely_allocation_attested: false,
            pre2025_method: LotMethod::Fifo,
        }),
    )
}

fn blocker_on(kind: BlockerKind, id: &EventId) -> Blocker {
    Blocker {
        kind,
        event: Some(id.clone()),
        detail: String::new(),
    }
}

/// `voidable_decisions` returns EXACTLY the same set the shipped single-void inline filter listed:
/// only Decision-id'd revocable, non-voided, non-effective-allocation events. Imported events,
/// non-revocable resolve decisions (SupersedeImport/RejectImport), already-voided decisions, and the
/// VoidDecisionEvent itself are all excluded (the Task-1 zero-behavior re-point proof).
#[test]
fn voidable_decisions_matches_single_void_flow() {
    let ti = import_ev(
        "vd-ti",
        EventPayload::TransferIn(btctax_core::event::TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    );

    // seq 1: ClassifyInbound (revocable, live)     → INCLUDED
    // seq 2: MethodElection  (revocable, live)     → INCLUDED
    // seq 3: SupersedeImport (non-revocable)       → excluded
    // seq 4: RejectImport    (non-revocable)       → excluded
    // seq 5: ClassifyInbound (revocable) voided-by → excluded (already voided)
    // seq 6: VoidDecisionEvent (targets seq 5)     → excluded (non-revocable + is the void)
    let ci_live = classify_inbound(1, ti.id.clone());
    let me = method_election(2);
    let supersede = decision(
        3,
        EventPayload::SupersedeImport(SupersedeImport {
            conflict_event: ti.id.clone(),
        }),
    );
    let reject = decision(
        4,
        EventPayload::RejectImport(RejectImport {
            conflict_event: ti.id.clone(),
        }),
    );
    let ci_voided = classify_inbound(5, ti.id.clone());
    let void_of_5 = decision(
        6,
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: ci_voided.id.clone(),
        }),
    );

    let events = vec![
        ti,
        ci_live.clone(),
        me.clone(),
        supersede,
        reject,
        ci_voided,
        void_of_5,
    ];
    let blockers: Vec<Blocker> = vec![];

    let got: Vec<EventId> = voidable_decisions(&events, &blockers)
        .into_iter()
        .map(|e| e.id.clone())
        .collect();

    assert_eq!(
        got,
        vec![ci_live.id.clone(), me.id.clone()],
        "voidable set = the live revocable decisions only (order = events order)"
    );

    // Cross-check the payload predicate directly.
    assert!(is_revocable_payload(&ci_live.payload));
    assert!(is_revocable_payload(&me.payload));
    assert!(!is_revocable_payload(&EventPayload::SupersedeImport(
        SupersedeImport {
            conflict_event: EventId::decision(99)
        }
    )));
    assert!(!is_revocable_payload(&EventPayload::VoidDecisionEvent(
        VoidDecisionEvent {
            target_event_id: EventId::decision(99)
        }
    )));
}

/// #7 [tax-safety]: an EFFECTIVE `SafeHarborAllocation` (NO timebar / NO unconservable blocker on its
/// id) is NEVER a bulk-void candidate — voiding it would fire a Hard `DecisionConflict`.
#[test]
fn bulk_void_excludes_effective_allocation() {
    let alloc = allocation(1);
    let events = vec![alloc.clone()];
    // No blocker on the allocation id → effective.
    let blockers: Vec<Blocker> = vec![];
    let got = voidable_decisions(&events, &blockers);
    assert!(
        got.is_empty(),
        "an effective allocation must NOT be a voidable candidate (#7)"
    );
}

/// An INERT allocation (timebarred OR unconservable) STAYS voidable — the void applies cleanly.
#[test]
fn bulk_void_includes_inert_allocation() {
    let alloc = allocation(1);
    let events = vec![alloc.clone()];

    // Timebarred → inert → listed.
    let timebar = vec![blocker_on(BlockerKind::SafeHarborTimebar, &alloc.id)];
    assert_eq!(
        voidable_decisions(&events, &timebar)
            .into_iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>(),
        vec![alloc.id.clone()],
        "a timebarred (inert) allocation stays voidable"
    );

    // Unconservable → inert → listed.
    let unconservable = vec![blocker_on(BlockerKind::SafeHarborUnconservable, &alloc.id)];
    assert_eq!(
        voidable_decisions(&events, &unconservable)
            .into_iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>(),
        vec![alloc.id.clone()],
        "an unconservable (inert) allocation stays voidable"
    );
}
