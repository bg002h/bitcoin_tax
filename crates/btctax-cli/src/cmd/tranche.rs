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
use btctax_core::persistence::load_all;
use btctax_core::tranche_guard::{in_force_allocation_exists, pre2025_tranche_exists};
use btctax_core::{EventId, EventPayload, LedgerEvent, Sat, TaxDate, WalletId};
use btctax_store::Passphrase;
use std::path::Path;
use time::OffsetDateTime;

/// Tranche-side hedge (tax r2 N-3): the user is recording a tranche and is blocked by an allocation, so
/// the finality caveat is about that ALLOCATION. A filed safe-harbor allocation cannot be silently unwound.
const ALLOCATION_IS_FINAL_HINT: &str = "revisit the in-app safe-harbor allocation; if your filed \
    allocation is already final, unallocated pre-2025 units are a facts-and-circumstances matter for a \
    professional";

/// Allocation-side hedge (tax review r1 Nit): the user is recording an allocation and is blocked by a
/// tranche, so the finality caveat is about that TRANCHE (a filed basis — `$0` or a promoted floor), not
/// the allocation.
const TRANCHE_IS_FINAL_HINT: &str = "Void the tranche first (`reconcile void <decision-ref>`); if you \
    have already filed the tranche's basis ($0 or a promoted floor), unallocated pre-2025 units are a \
    facts-and-circumstances matter for a professional";

/// P8 Nit: is `wallet` referenced by any prior event — an import's `wallet`, or a prior tranche
/// declaration's target wallet? A `false` result means `--wallet` likely has a TYPO. This drives a WARN
/// only, NEVER a refusal: a conservative-filing tranche lot in any wallet still files at its conservative
/// basis ($0, or a promoted floor) — tax-neutral — so a typo merely strands the lot in a phantom wallet
/// rather than mis-stating tax. Pure over the event log.
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
             ($0 or a promoted floor, EstimatedConservative) is on file — v1 makes the two mutually \
             exclusive. {TRANCHE_IS_FINAL_HINT}."
        )));
    }
    Ok(())
}

/// TRANSITION-side guard for the tranche record path: refuse recording a PRE-2025 tranche while an
/// in-force allocation exists. A `window_end ≥ 2025` tranche is NOT blocked (records cleanly beside an
/// effective allocation — else P7's mandatory disclosure is foreclosed for the mixed-records filer).
///
/// `pub(crate)` (Defensive Filing Wizard Task 2): `crate::chokepoint::plan_declare` also calls this — the
/// single source of the guard stays HERE (this module), not duplicated into the chokepoint.
pub(crate) fn guard_tranche_vs_allocation(
    events: &[LedgerEvent],
    window_end: TaxDate,
) -> Result<(), CliError> {
    if window_end < TRANSITION_DATE && in_force_allocation_exists(events) {
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
///
/// Defensive Filing Wizard Task 2: the actual plan/apply PIPELINE now lives in `crate::chokepoint`
/// (`plan_declare`/`apply_declare`) — a reusable chokepoint a future TUI can drive identically. This
/// function is a THIN DRIVER over it: `Session::open` → `plan_declare` (passing
/// `target_shortfall=None` — the CLI free-form path never runs the DFW-D5.2 clearance shadow, mapping a
/// `Refusal` to a `CliError`) → the phantom-wallet stderr warning (tax-M-3: I/O, not gate logic, so it
/// stays HERE rather than inside the pure `plan_declare`) → `apply_declare`.
pub fn declare_tranche(
    vault_path: &Path,
    pp: &Passphrase,
    sat: Sat,
    wallet: WalletId,
    window_start: TaxDate,
    window_end: TaxDate,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    let events = load_all(session.conn())?;
    let cfg = session.config()?.to_projection();

    let plan = crate::chokepoint::plan_declare(
        &events,
        session.prices(),
        &cfg,
        sat,
        wallet.clone(),
        window_start,
        window_end,
        None,
        now,
    )
    .map_err(CliError::from)?;

    // The declaration WILL be recorded now — warn (never refuse) on a `--wallet` that no prior event
    // references (a likely typo that strands the tranche lot in a phantom wallet; it still files at its
    // conservative basis). tax-M-3: this is I/O, not gate logic, so it stays in the driver, AFTER
    // `plan_declare` succeeds — a REFUSED declaration never emits the misleading "stranded lot" note
    // (arch N-1, preserved: the shipped verb warned only once the guard had already admitted).
    if !wallet_is_known(&events, &wallet) {
        eprintln!(
            "warning: --wallet {} has no prior events in this vault; if this is a typo the conservative \
             tranche lot is stranded in a phantom wallet (it still files at its conservative basis — $0, \
             or a promoted floor). Re-run with the intended --wallet if this was unintended.",
            crate::render::wallet_label(&wallet)
        );
    }

    crate::chokepoint::apply_declare(&mut session, plan, now)
}
