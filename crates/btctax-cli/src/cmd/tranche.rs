//! Conservative-filing `DeclareTranche` record path + the tranche⇄allocation mutual-exclusion guard
//! (D-8 UX layer). A tranche is undocumented BTC declared at $0 basis (the IRS fallback), folded to an
//! `EstimatedConservative` lot homed at `window_end`. See `design/conservative-filing/SPEC.md` D-8.
//!
//! The engine backstop (Task 5 — `SafeHarborUnconservable` denies a `SafeHarborAllocation` effectiveness
//! over a live tranche residue) is the GUARANTEE; these record-time guards are the early, friendly error.
//! They are the single source of the mutual-exclusion predicate for ALL FOUR allocation append sites
//! (CLI `safe_harbor_allocate` + `safe_harbor_attest`; TUI `persist_safe_harbor_allocate` +
//! `persist_safe_harbor_attest`) and the tranche record path here.

use crate::{CliError, Session};
use btctax_core::conventions::TRANSITION_DATE;
use btctax_core::event::DeclareTranche;
use btctax_core::persistence::{append_decision, load_all};
use btctax_core::{
    Blocker, BlockerKind, EventId, EventPayload, LedgerEvent, Sat, TaxDate, WalletId,
};
use btctax_store::Passphrase;
use std::collections::BTreeSet;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

/// Tranche-side hedge (tax r2 N-3): the user is recording a tranche and is blocked by an allocation, so
/// the finality caveat is about that ALLOCATION. A filed safe-harbor allocation cannot be silently unwound.
const ALLOCATION_IS_FINAL_HINT: &str = "revisit the in-app safe-harbor allocation; if your filed \
    allocation is already final, unallocated pre-2025 units are a facts-and-circumstances matter for a \
    professional";

/// Allocation-side hedge (tax review r1 Nit): the user is recording an allocation and is blocked by a
/// tranche, so the finality caveat is about that TRANCHE (a filed $0 basis), not the allocation.
const TRANCHE_IS_FINAL_HINT: &str = "Void the tranche first (`reconcile void <decision-ref>`); if you \
    have already filed the tranche's $0 basis, unallocated pre-2025 units are a facts-and-circumstances \
    matter for a professional";

/// The set of event ids targeted by any `VoidDecisionEvent` in the log — the record-time "voided" view.
///
/// Mirrors `resolve.rs` pass-1 step 1a and the attest site's own `voided` set: a decision is not-in-force
/// once a `VoidDecisionEvent` names it. (A void of a `SafeHarborAllocation` is resolver-deferred to Task 12
/// for its EFFECTIVE-vs-inert semantics, but for THIS friendly record-time layer the presence of the void
/// is enough — the engine backstop is the guarantee behind it.)
fn void_targets(events: &[LedgerEvent]) -> BTreeSet<EventId> {
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
pub fn in_force_allocation_exists(events: &[LedgerEvent], blockers: &[Blocker]) -> bool {
    let voided = void_targets(events);
    events.iter().any(|e| {
        matches!(e.payload, EventPayload::SafeHarborAllocation(_)) && {
            // In force = NON-voided (present, effective OR inert — an inert one can be flipped effective,
            // arch r2 New-3) OR still EFFECTIVE despite a void. Arch r1 Minor (T16): the engine keeps an
            // EFFECTIVE allocation in force after a (conflicting) void — voiding it fires a Hard
            // DecisionConflict rather than removing it (`resolve` effective-vs-inert semantics), so a
            // voided-BUT-effective allocation must still block a pre-2025 tranche. Effectiveness mirrors
            // void.rs `effective_alloc`: no SafeHarborTimebar / SafeHarborUnconservable blocker on its id.
            let non_voided = !voided.contains(&e.id);
            let effective = {
                let has = |k| {
                    blockers
                        .iter()
                        .any(|b| b.kind == k && b.event.as_ref() == Some(&e.id))
                };
                !has(BlockerKind::SafeHarborTimebar) && !has(BlockerKind::SafeHarborUnconservable)
            };
            non_voided || effective
        }
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

/// P8 Nit: is `wallet` referenced by any prior event — an import's `wallet`, or a prior tranche
/// declaration's target wallet? A `false` result means `--wallet` likely has a TYPO. This drives a WARN
/// only, NEVER a refusal: a `$0` tranche lot in any wallet still files at `$0` (tax-neutral), so a typo
/// merely strands the lot in a phantom wallet rather than mis-stating tax. Pure over the event log.
pub fn wallet_is_known(events: &[LedgerEvent], wallet: &WalletId) -> bool {
    events.iter().any(|e| {
        e.wallet.as_ref() == Some(wallet)
            || matches!(&e.payload, EventPayload::DeclareTranche(t) if &t.wallet == wallet)
    })
}

/// ALLOCATION-side guard (the chokepoint for all four allocation append sites): refuse recording a
/// `SafeHarborAllocation` while a pre-2025 tranche is on file. v1 makes them mutually exclusive (D-8).
pub fn guard_allocation_vs_tranche(events: &[LedgerEvent]) -> Result<(), CliError> {
    if pre2025_tranche_exists(events) {
        return Err(CliError::Usage(format!(
            "refusing to record a safe-harbor allocation while a pre-2025 conservative-filing tranche \
             ($0 EstimatedConservative) is on file — v1 makes the two mutually exclusive. \
             {TRANCHE_IS_FINAL_HINT}."
        )));
    }
    Ok(())
}

/// TRANSITION-side guard for the tranche record path: refuse recording a PRE-2025 tranche while an
/// in-force allocation exists. A `window_end ≥ 2025` tranche is NOT blocked (records cleanly beside an
/// effective allocation — else P7's mandatory disclosure is foreclosed for the mixed-records filer).
fn guard_tranche_vs_allocation(
    events: &[LedgerEvent],
    window_end: TaxDate,
    blockers: &[Blocker],
) -> Result<(), CliError> {
    if window_end < TRANSITION_DATE && in_force_allocation_exists(events, blockers) {
        return Err(CliError::Usage(format!(
            "refusing to record a pre-2025 conservative-filing tranche while a safe-harbor allocation \
             is on file — v1 makes the two mutually exclusive; {ALLOCATION_IS_FINAL_HINT}."
        )));
    }
    Ok(())
}

/// Append a `DeclareTranche` decision (a $0-basis `EstimatedConservative` lot) and persist.
///
/// `now` is the injected decision creation-time (deterministic in tests). The tranche folds via the
/// shared `Op::Acquire` path to a lot homed at `window_end`, $0 basis, tagged `EstimatedConservative`.
/// Record-time guard (D-8): a PRE-2025 tranche is refused while an in-force allocation exists.
pub fn declare_tranche(
    vault_path: &Path,
    pp: &Passphrase,
    sat: Sat,
    wallet: WalletId,
    window_start: TaxDate,
    window_end: TaxDate,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    // Input validation (record-time refuse — no vault access needed).
    if sat <= 0 {
        // A `sat <= 0` tranche would bump `stats.sigma_in` by a non-positive amount (fold.rs),
        // corrupting Σ-conservation; there is no such thing as declaring zero/negative undocumented BTC.
        return Err(CliError::Usage(format!(
            "tranche amount must be > 0 sat (got {sat})"
        )));
    }
    if window_start > window_end {
        return Err(CliError::Usage(format!(
            "tranche window_start ({window_start}) must be <= window_end ({window_end})"
        )));
    }

    let mut session = Session::open(vault_path, pp)?;
    let events = load_all(session.conn())?;
    // Guard FIRST (arch N-1): project the EXISTING events (record-time, mirrors `would_conflict`) so the
    // tranche-side guard sees the engine's effective-vs-inert allocation view — an EFFECTIVE-but-voided
    // allocation still blocks a pre-2025 tranche (arch r1 Minor, T16). Running it before the phantom-wallet
    // warning means a REFUSED declaration never emits the misleading "stranded lot" note.
    let cfg = session.config()?;
    let state = btctax_core::project::project(&events, session.prices(), &cfg.to_projection());
    guard_tranche_vs_allocation(&events, window_end, &state.blockers)?;
    // The declaration WILL be recorded now — warn (never refuse) on a `--wallet` that no prior event
    // references (a likely typo that strands the $0 lot in a phantom wallet; it still files at $0).
    if !wallet_is_known(&events, &wallet) {
        eprintln!(
            "warning: --wallet {} has no prior events in this vault; if this is a typo the $0 tranche \
             lot is stranded in a phantom wallet (it still files at $0). Re-run with the intended \
             --wallet if this was unintended.",
            crate::render::wallet_label(&wallet)
        );
    }
    let payload = EventPayload::DeclareTranche(DeclareTranche {
        sat,
        wallet,
        window_start,
        window_end,
    });
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}
