//! Pure event-scan predicates for the tranche⇄allocation mutual-exclusion guard (D-8).
//!
//! Defensive Filing Wizard Task 5 (C-2): MOVED here from `btctax-cli::cmd::tranche` so btctax-core
//! callers (Task 6's `journey_view` safe-harbor read, Task 8's declare flow) can consult them without a
//! cli→core dependency inversion. All three predicates are events-only scans (never touch persistence);
//! `TRANSITION_DATE` is the same conservative-filing/safe-harbor boundary used everywhere else in core.
//!
//! The GUARD functions themselves (`guard_allocation_vs_tranche` / `guard_tranche_vs_allocation`) STAY in
//! `btctax-cli::cmd::tranche` — they map a refusal to a `CliError`, a cli-only concern. Only these three
//! pure, `CliError`-free scans moved; this module is the single source for both crates.

use crate::conventions::TRANSITION_DATE;
use crate::event::EventPayload;
use crate::identity::EventId;
use crate::LedgerEvent;
use std::collections::BTreeSet;

/// The set of event ids targeted by any `VoidDecisionEvent` in the log — the record-time "voided" view.
///
/// Mirrors `resolve.rs` pass-1 step 1a and the attest site's own `voided` set: a decision is not-in-force
/// once a `VoidDecisionEvent` names it. (A void of a `SafeHarborAllocation` is resolver-deferred to Task 12
/// for its EFFECTIVE-vs-inert semantics, but for THIS friendly record-time layer the presence of the void
/// is enough — the engine backstop is the guarantee behind it.)
pub(crate) fn void_targets(events: &[LedgerEvent]) -> BTreeSet<EventId> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect()
}

/// True iff an IN-FORCE (non-voided) `SafeHarborAllocation` exists — **effective OR inert** (arch r2
/// New-3: an inert allocation can be flipped effective, so it too collides with a new pre-2025 tranche).
/// Deliberately NOT scoped to effective allocations (that would let a pre-2025 tranche slip in beside an
/// inert one and silently discard it once the allocation later goes effective).
pub fn in_force_allocation_exists(events: &[LedgerEvent]) -> bool {
    // In force = a NON-voided `SafeHarborAllocation` (present, effective OR inert — an inert one can be
    // flipped effective, arch r2 New-3). A voided allocation is NOT in force here: the ENGINE resolves the
    // void (T16 review r2 / I-1) — a void of an inert allocation retires it (§7.4 retirement pass), and a
    // void of an effective allocation raises a Hard `DecisionConflict` there; either way the record-time
    // predicate correctly ADMITS the tranche and the engine backstop is the guarantee. (This replaces the
    // r1 blocker-absence "effective" mirror, which coupled badly with the backstop's blocker retraction.)
    let voided = void_targets(events);
    events.iter().any(|e| {
        matches!(e.payload, EventPayload::SafeHarborAllocation(_)) && !voided.contains(&e.id)
    })
}

/// True iff a non-voided PRE-2025 (`window_end < TRANSITION_DATE`) `DeclareTranche` exists — the only
/// tranche that collides with the pre-2025 Universal residue a `SafeHarborAllocation` reconstructs
/// (tax r1 I-2). A `window_end ≥ 2025` tranche folds into a post-transition per-wallet pool and never
/// touches Rev-Proc-2024-28, so it does NOT block an allocation.
pub fn pre2025_tranche_exists(events: &[LedgerEvent]) -> bool {
    let voided = void_targets(events);
    events.iter().any(|e| {
        matches!(&e.payload, EventPayload::DeclareTranche(t) if t.window_end < TRANSITION_DATE)
            && !voided.contains(&e.id)
    })
}
