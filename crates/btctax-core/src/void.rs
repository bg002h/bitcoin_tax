//! The voidable-candidate predicate — the SINGLE shared source of truth for "which reconcile
//! decisions may be swept-voided" (SPEC_bulk_void §Candidate set). Pure: it reads only the events +
//! the projected blockers, so it is reachable by the CLI (`Session::bulk_void_plan` via `project()`)
//! AND the TUI (`open_void_flow` / `open_bulk_void_flow` via the snapshot). There is NO second copy —
//! any drift between the single-void filter and the bulk sweep is a tax-safety bug (the #7 exclusion).

use crate::event::{EventPayload, LedgerEvent};
use crate::identity::EventId;
use crate::state::{Blocker, BlockerKind};
use std::collections::{BTreeMap, BTreeSet};

/// Return `true` when `payload` is a revocable decision type.
///
/// Revocable: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw,
/// MethodElection, LotSelection, ReclassifyIncome, SelfTransferPassthrough, SafeHarborAllocation,
/// DeclareTranche, PromoteTranche.
/// Non-revocable (excluded from void list): SupersedeImport, RejectImport, VoidDecisionEvent,
/// and imported event payloads (Acquire, Income, Dispose, TransferOut, TransferIn, Unclassified,
/// ImportConflict — these carry Import EventIds, not Decision EventIds, so they cannot appear in
/// the void list; the check on Decision-id'd events guards the decision payload variants only).
pub fn is_revocable_payload(payload: &EventPayload) -> bool {
    matches!(
        payload,
        EventPayload::TransferLink(_)
            | EventPayload::ReclassifyOutflow(_)
            | EventPayload::ClassifyInbound(_)
            | EventPayload::ManualFmv(_)
            | EventPayload::ClassifyRaw(_)
            | EventPayload::MethodElection(_)
            | EventPayload::LotSelection(_)
            | EventPayload::ReclassifyIncome(_)
            | EventPayload::SelfTransferPassthrough(_)
            | EventPayload::SafeHarborAllocation(_)
            | EventPayload::DeclareTranche(_)
            // BG-D9 (Task 7): a promote is a layered decision — revoke it to revert the tranche to $0.
            | EventPayload::PromoteTranche(_)
    )
}

/// Enumerate the reconcile **Decision** events that may be voided, applying the exact single-void
/// filter chain (SPEC_bulk_void §Candidate set) over `events` + the projected `blockers`:
///
/// 1. `EventId::Decision { .. }` — only decision events (imported/conflict ids can never be here).
/// 2. NOT already-voided — `e.id` is not the `target_event_id` of any `VoidDecisionEvent`.
/// 3. `is_revocable_payload(&e.payload)` — excludes `SupersedeImport` / `RejectImport` /
///    `VoidDecisionEvent` (a void is never itself voidable; resolve decisions are not swept here).
/// 4. `!effective_alloc` — **the #7 exclusion.** `effective_alloc` = the payload is a
///    `SafeHarborAllocation` AND NEITHER `SafeHarborTimebar` NOR `SafeHarborUnconservable` blocker
///    fired on `e.id`. Engine evidence: unconservable ⟹ blocker (`resolve.rs:989-994`), timebarred ⟹
///    blocker (`resolve.rs:997-1002`), and voiding an EFFECTIVE allocation → Hard `DecisionConflict`
///    (`resolve.rs:1030-1039`) — a permanent, damaging no-op that gates the whole tax year. INERT
///    allocations (timebarred OR unconservable) STAY voidable — the void applies cleanly (source
///    invariant `resolve.rs:1030-1031`; behavior pinned by `btctax-core/tests/transition.rs:403`).
/// 5. `!promoted_target` — **BG-D9 (Task 7), the tranche analogue of #7.** A `DeclareTranche` named by
///    EXACTLY ONE non-voided `PromoteTranche` (i.e. held in force by a LIVE promote) is excluded: voiding
///    it is inert + `DecisionConflict` (`resolve.rs` deferred tranche-void adjudication), the same damaging
///    no-op #7 guards against. ≥2 promotes is itself a conflict with NONE in force, so that tranche's void
///    APPLIES and it STAYS voidable; the `PromoteTranche` decision itself is ALWAYS voidable (revoke → $0).
///
/// Returned in `events` iteration order (the pre-sort filter order of the shipped single-void flow);
/// callers sort by `seq` for display.
pub fn voidable_decisions<'a>(
    events: &'a [LedgerEvent],
    blockers: &[Blocker],
) -> Vec<&'a LedgerEvent> {
    // Build the voided set (ids targeted by any VoidDecisionEvent).
    let voided: BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| {
            if let EventPayload::VoidDecisionEvent(v) = &e.payload {
                Some(v.target_event_id.clone())
            } else {
                None
            }
        })
        .collect();

    // #7: an EFFECTIVE SafeHarborAllocation (no timebar / no unconservable blocker on its id) is
    // irrevocable — voiding it fires a Hard DecisionConflict. Inert allocations stay voidable.
    let effective_alloc = |e: &LedgerEvent| {
        matches!(e.payload, EventPayload::SafeHarborAllocation(_)) && {
            let has = |k| {
                blockers
                    .iter()
                    .any(|b| b.kind == k && b.event.as_ref() == Some(&e.id))
            };
            !has(BlockerKind::SafeHarborTimebar) && !has(BlockerKind::SafeHarborUnconservable)
        }
    };

    // BG-D9 (Task 7): a `DeclareTranche` held in force by a LIVE promote is likewise excluded — voiding it
    // is inert + `DecisionConflict` (`resolve.rs` tranche-void adjudication), a damaging no-op that mirrors
    // the #7 effective-allocation exclusion. "Live" = EXACTLY one non-voided `PromoteTranche` names it —
    // matching `live_promotes`' membership: ≥2 is itself a conflict with NONE in force, so that tranche's
    // void APPLIES and it STAYS voidable. The `PromoteTranche` decision ITSELF stays voidable (revoke → $0).
    let promote_count: BTreeMap<EventId, usize> = {
        let mut m: BTreeMap<EventId, usize> = BTreeMap::new();
        for e in events {
            if let EventPayload::PromoteTranche(p) = &e.payload {
                if !voided.contains(&e.id) {
                    *m.entry(p.target.clone()).or_insert(0) += 1;
                }
            }
        }
        m
    };
    let promoted_target = |e: &LedgerEvent| {
        matches!(e.payload, EventPayload::DeclareTranche(_))
            && promote_count.get(&e.id).copied() == Some(1)
    };

    events
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .filter(|e| !voided.contains(&e.id))
        .filter(|e| is_revocable_payload(&e.payload))
        .filter(|e| !effective_alloc(e))
        .filter(|e| !promoted_target(e))
        .collect()
}
