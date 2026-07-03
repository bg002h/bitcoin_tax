//! The ONLY module in btctax-tui-edit that touches the mutation surface:
//! conn() / save() / tax_profile::set / append_decision live here and nowhere else.
//!
//! Guarantee: "writes ONLY append-only events + typed side-table upserts via
//! edit/persist.rs, each behind an explicit payload-showing confirmation; the vault
//! file only via Vault::save's atomic path."
//!
//! # VaultLock note
//! The editor holds the vault's exclusive lock for its entire lifetime
//! (session.rs:53–58, vault.rs:137–142). The CLI or viewer cannot run concurrently
//! against the same vault. There is no concurrent-writer case.
//!
//! # Vault-creating constructors
//! `Session::create` / `Session::repair` / `Vault::create` / `Vault::repair`
//! are FORBIDDEN in non-test code of this crate (R0-I1). They create/overwrite a
//! vault file outside `Vault::save`'s atomic path, which violates the guarantee.
//! The mechanized gate (KAT-G1) enforces this crate-wide.
//!
//! # Failed-save rollback (save-rollback cycle)
//! Every persist fn below (except `persist_safe_harbor_attest`) snapshots the in-memory DB before
//! mutating and, if `session.save()` fails, reverts it via `save_or_rollback` — so a failed save
//! leaves NO residue and a retry is clean (same `decision_seq`). `persist_safe_harbor_attest` keeps
//! its own `attest_save_failed` latch (its double-batch is unrecoverable; see that fn + editor.rs).

/// Outcome of an editor persist fn's save-with-rollback. `NoChange` and `RolledBack` are both
/// "nothing persisted, safe to retry"; `ResidueLive` is the (astronomically rare) unrecoverable case.
///
/// # No `Display` [R0-I1]
/// This enum deliberately does NOT implement `Display`, so a lazy `format!("{e}")` is a compile
/// error — every consumer routes through `EditorApp::on_persist_error`, the single site that arms
/// the `rollback_failed` latch on `ResidueLive`.
#[derive(Debug)]
pub enum PersistError {
    /// Failed at `snapshot()` or the append/upsert — NOTHING was written and there is no residue
    /// (snapshot failure = fail-closed refusal; append/upsert failure = no row added).
    NoChange(btctax_cli::CliError),
    /// `save()` failed; the in-memory DB was cleanly reverted to its pre-mutation image. No residue;
    /// a retry re-appends with the SAME `decision_seq`.
    RolledBack(btctax_cli::CliError),
    /// `save()` failed AND the revert ALSO failed — the unsaved mutation is LIVE in the in-memory DB.
    /// The caller MUST latch every mutating opener and prompt an immediate quit (the on-disk vault is
    /// pre-action; quitting discards the residue).
    ResidueLive(btctax_cli::CliError),
}

// Both `From` impls are required: `session.snapshot()?` yields `CliError`, but `append_decision(...)?`
// / `load_all(...)?` yield `btctax_core::CoreError`, and Rust does not chain
// `CoreError → CliError → PersistError`. Neither impl targets `ResidueLive`.
impl From<btctax_cli::CliError> for PersistError {
    fn from(e: btctax_cli::CliError) -> Self {
        PersistError::NoChange(e)
    }
}
impl From<btctax_core::CoreError> for PersistError {
    fn from(e: btctax_core::CoreError) -> Self {
        PersistError::NoChange(e.into()) // CoreError → CliError
    }
}

/// Revert the in-memory DB to `pre` after a mutation-committing step failed, mapping the error:
/// `RolledBack` if the revert succeeds, `ResidueLive` if the revert ALSO fails (residue is live).
fn rollback(
    session: &mut btctax_cli::Session,
    pre: &[u8],
    err: btctax_cli::CliError,
) -> PersistError {
    match session.restore(pre) {
        Ok(()) => PersistError::RolledBack(err),
        Err(_revert_err) => PersistError::ResidueLive(err),
    }
}

/// `session.save()`; on failure revert the in-memory DB to `pre`. `Ok` on success; `RolledBack` on a
/// save failure cleanly reverted; `ResidueLive` if the revert itself failed (residue live).
fn save_or_rollback(session: &mut btctax_cli::Session, pre: Vec<u8>) -> Result<(), PersistError> {
    match session.save() {
        Ok(()) => Ok(()),
        Err(save_err) => Err(rollback(session, &pre, save_err)),
    }
}

/// Upsert the tax profile for `year` and atomically save the vault.
///
/// Mirrors `cmd::tax::set_profile` (cmd/tax.rs:14–23) minus the open/drop —
/// the editor operates on its HELD session (the VaultLock must stay acquired).
///
/// # Called only from the mutation-confirmation modal
/// This is a `pub fn` freely callable; "the confirmation modal gates the ONLY
/// call site" is a procedural guarantee (enforced by KAT-G1's confinement of
/// the surface, the KATs, and whole-diff review), not a type-level proof.
/// A sealed confirmation-token type is a FOLLOWUP if the editor grows more flows.
///
/// # Failed-save rollback [R0-I2]
/// Snapshots before the upsert; if `save` fails, `save_or_rollback` reverts the in-memory side-table
/// to its pre-upsert image so memory == disk and a retry re-runs cleanly. Included in the rollback
/// set for a uniform invariant (reverting an idempotent upsert is unnecessary but harmless).
pub fn persist_tax_profile(
    session: &mut btctax_cli::Session,
    year: i32,
    p: &btctax_core::TaxProfile,
) -> Result<(), PersistError> {
    let pre = session.snapshot()?;
    btctax_cli::tax_profile::set(session.conn(), year, p)?; // typed side-table upsert
    save_or_rollback(session, pre)?; // encrypt + atomic_write; revert on failure
    Ok(())
}

/// Append a `ClassifyInbound` decision event and atomically save the vault.
///
/// `payload` is the **fully-validated** `EventPayload::ClassifyInbound(…)` built by
/// the classify-inbound form.  `now` is the caller-supplied `OffsetDateTime`
/// (injected at Enter-press for test determinism; never derived inside this fn).
///
/// # Strict-append semantics
/// Calls `append_decision(conn, payload, now, UTC, None)` → the event is assigned
/// `decision_seq = MAX(existing) + 1`.  After `session.save()` the vault image on
/// disk contains the new event at the tail.  The KAT-P2a strict-prefix test
/// verifies this invariant.
///
/// # Called only from the classify-inbound confirmation modal
/// Same procedural guarantee as `persist_tax_profile` (see doc there).
pub fn persist_classify_inbound(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `ReclassifyOutflow` decision event and atomically save the vault.
///
/// `payload` is the **fully-validated** `EventPayload::ReclassifyOutflow(…)` built by
/// the reclassify-outflow form.  `now` is the caller-supplied `OffsetDateTime`
/// (injected at Enter-press for test determinism; never derived inside this fn).
///
/// # Strict-append semantics
/// Calls `append_decision(conn, payload, now, UTC, None)` → the event is assigned
/// `decision_seq = MAX(existing) + 1`.  After `session.save()` the vault image on
/// disk contains the new event at the tail.  The KAT-P2b strict-prefix test
/// verifies this invariant.
///
/// # Called only from the reclassify-outflow confirmation modal
/// Same procedural guarantee as `persist_tax_profile` (see doc there).
///
/// # Failed-save rollback [R0-I1]
/// Snapshots before the append; if `save` fails, `save_or_rollback` reverts the in-memory DB so no
/// residue remains. A retry after a failed save is CLEAN — it re-appends with the SAME `decision_seq`
/// (recomputed `MAX+1` over the reverted table), producing no duplicate and no `DecisionConflict`.
pub fn persist_reclassify_outflow(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `ReclassifyIncome` decision event and atomically save the vault.
///
/// `payload` is the **fully-validated** `EventPayload::ReclassifyIncome(…)` built by
/// the reclassify-income form.  `now` is the caller-supplied `OffsetDateTime`
/// (injected at Enter-press for test determinism; never derived inside this fn).
///
/// # Strict-append semantics
/// Calls `append_decision(conn, payload, now, UTC, None)` → the event is assigned
/// `decision_seq = MAX(existing) + 1`.  After `session.save()` the vault image on
/// disk contains the new event at the tail.  The KAT-P2c strict-prefix test
/// verifies this invariant.
pub fn persist_reclassify_income(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `ManualFmv` decision event and atomically save the vault.
///
/// `payload` is the **fully-validated** `EventPayload::ManualFmv(…)` built by
/// the set-fmv form.  `now` is the caller-supplied `OffsetDateTime`
/// (injected at Enter-press for test determinism; never derived inside this fn).
///
/// # Strict-append semantics
/// Calls `append_decision(conn, payload, now, UTC, None)` → the event is assigned
/// `decision_seq = MAX(existing) + 1`.  After `session.save()` the vault image on
/// disk contains the new event at the tail.  The KAT-P2d strict-prefix test
/// verifies this invariant.
pub fn persist_set_fmv(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `VoidDecisionEvent` decision and atomically save the vault.
///
/// `target_event_id` is the EventId of the revocable decision to void.
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # LotSelection side-effect (reconcile.rs:117–147)
/// If the target decision is a `LotSelection`, also calls
/// `btctax_cli::optimize_attest::clear(session.conn(), &ls.disposal_event)` BEFORE save —
/// same atomic batch as the CLI void command. Non-LotSelection targets are unaffected.
///
/// # Failed-save rollback [M1]
/// Snapshots before the append + the `optimize_attest::clear` side-effect; if `save` fails,
/// `save_or_rollback` reverts BOTH the void row AND the side-table clear (the whole-DB restore
/// covers the side-table for free — the load-bearing reason A′ uses whole-DB restore, not a row
/// DELETE). A retry after a failed save is clean — same `decision_seq`, no residue. A retry after a
/// SUCCESSFUL void appends a separate second void row for the same target (idempotent — the
/// resolve.rs BTreeSet insert is inert, no conflict; pinned by KAT-VOID-RETRY).
pub fn persist_void(
    session: &mut btctax_cli::Session,
    target_event_id: btctax_core::EventId,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    use btctax_core::{
        event::VoidDecisionEvent,
        persistence::{append_decision, load_all},
        EventPayload,
    };

    let pre = session.snapshot()?;

    // Detect LotSelection target for the optimize_attest side-effect.
    let events = load_all(session.conn())?;
    let disposal_to_clear: Option<btctax_core::EventId> = events
        .iter()
        .find(|e| e.id == target_event_id)
        .and_then(|e| match &e.payload {
            EventPayload::LotSelection(ls) => Some(ls.disposal_event.clone()),
            _ => None,
        });

    let id = append_decision(
        session.conn(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id }),
        now,
        time::UtcOffset::UTC,
        None,
    )?;

    // A failure AFTER the committed append must roll back — else the void append becomes residue
    // that could piggy-back a later save [WB-M1]. (`clear` is a pure in-memory DELETE; its failure
    // is the same OOM/corruption class as a restore failure — nil reachability, but the invariant is
    // now airtight and symmetric with `save_or_rollback`.)
    if let Some(disposal) = disposal_to_clear {
        if let Err(e) = btctax_cli::optimize_attest::clear(session.conn(), &disposal) {
            return Err(rollback(session, &pre, e));
        }
    }

    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `LotSelection` decision and atomically save the vault.
///
/// `payload` is the VALIDATED `EventPayload::LotSelection(…)`.
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # Failed-save rollback [R0-I2]
/// Snapshots before the append; if `save` fails, `save_or_rollback` reverts the in-memory DB. A retry
/// after a failed save is CLEAN — same `decision_seq`, no duplicate, no `DecisionConflict`. (A genuine
/// DUPLICATE — two SUCCESSFUL selections for one disposal — still conflicts per resolve.rs:787-800 and
/// NEITHER applies; the failed-save path simply no longer creates one.)
///
/// Does NOT write to `optimize_attestation` (only `optimize accept --attest` does that).
/// Clearing `optimize_attestation` on void is handled by `persist_void` (chunk 2b, D4).
pub fn persist_select_lots(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload, // must be EventPayload::LotSelection
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `TransferLink` decision event and atomically save the vault (chunk 4a, D1).
///
/// `payload` is the VALIDATED `EventPayload::TransferLink(…)` built by the link-transfer flow.
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # Single-append shape (NO bespoke latch)
/// Identical `snapshot → append_decision → save_or_rollback` shape as `persist_select_lots`:
/// exactly ONE fallible mutation after the snapshot, so a failed save reverts cleanly and a retry
/// re-appends with the SAME `decision_seq` (no residue, no duplicate). The linked pair projects the
/// TransferOut to `Op::SelfTransfer` (a non-taxable relocation); a genuine DUPLICATE link — two
/// SUCCESSFUL links for one out_event — still fires `DecisionConflict` in resolve.rs and NEITHER
/// applies, but the failed-save path no longer creates one.
pub fn persist_link_transfer(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload, // must be EventPayload::TransferLink
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `ClassifyRaw` decision event and atomically save the vault (chunk 4a, D2).
///
/// `payload` is the VALIDATED `EventPayload::ClassifyRaw(…)` built by the classify-raw flow (its
/// `as_` is a directly-built imported `EventPayload::Acquire`/`Income`). `now` is INJECTED at
/// Enter-press for test determinism.
///
/// # Single-append shape (NO bespoke latch)
/// Identical `snapshot → append_decision → save_or_rollback` shape as `persist_select_lots`: exactly
/// ONE fallible mutation after the snapshot, so a failed save reverts cleanly and a retry re-appends
/// with the SAME `decision_seq`. A genuine DUPLICATE classification of one target still fires
/// `DecisionConflict` in resolve.rs; the failed-save path no longer creates one.
pub fn persist_classify_raw(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload, // must be EventPayload::ClassifyRaw
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append a `SupersedeImport` (accept) or `RejectImport` (reject) decision resolving an
/// `ImportConflict`, and atomically save the vault (chunk 4b, D3).
///
/// `conflict_event` is the `ImportConflict` event id (the blocker's `.event`); `kind` selects the
/// appended variant. `now` is INJECTED at Enter-press for test determinism. Mirrors the two CLI
/// verbs `reconcile accept-conflict` / `reject-conflict` (reconcile.rs:178/194), which differ only in
/// the appended payload — hence one persist fn with a `kind` param.
///
/// # Single-append shape (NO bespoke latch)
/// Identical `snapshot → append_decision → save_or_rollback` shape as `persist_select_lots`: exactly
/// ONE fallible mutation after the snapshot, so a failed save reverts cleanly and a retry re-appends
/// with the SAME `decision_seq` (no residue). On success the target's `ImportConflict` blocker clears
/// (resolve.rs:386-401).
///
/// # Non-revocable [D3]
/// `SupersedeImport`/`RejectImport` are EXCLUDED from `is_revocable_payload`, so the decision cannot
/// be voided in-editor (a later void would fire `DecisionConflict`, resolve.rs:312-313). The modal
/// carries the prominent NON-REVOCABLE warning; this is the correct ceremony (NOT a typed-word gate,
/// which is reserved for the §7.4 unrecoverable-batch attest).
pub fn persist_resolve_conflict(
    session: &mut btctax_cli::Session,
    conflict_event: btctax_core::EventId,
    kind: crate::edit::form::ResolveKind,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    use btctax_core::event::{RejectImport, SupersedeImport};
    use btctax_core::EventPayload;
    let payload = match kind {
        crate::edit::form::ResolveKind::Accept => {
            EventPayload::SupersedeImport(SupersedeImport { conflict_event })
        }
        crate::edit::form::ResolveKind::Reject => {
            EventPayload::RejectImport(RejectImport { conflict_event })
        }
    };
    let pre = session.snapshot()?;
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    save_or_rollback(session, pre)?;
    Ok(id)
}

/// Append the optimizer's proposed `LotSelection` and — for an already-executed disposal — ALSO
/// upsert the `optimize_attestation` side-table row, then atomically save the vault (chunk 4b, D4).
///
/// `disposal` is the disposal EventId; `picks` is the proposed `LotSelection.lots`; `attestation`
/// is `Some(text)` for the `NeedsAttestation`/`AttestedRecording` path (co-persisted) and `None` for
/// the genuinely-contemporaneous path; `made` is the selection's made-date (attested_at string);
/// `now` is INJECTED at Enter-press for the append timestamp. Mirrors `cmd::optimize::accept`'s
/// per-disposal co-persist (cmd/optimize.rs:263-264) but on the HELD session.
///
/// # Dual-write shape — the INVERSE of `persist_void`
/// Snapshots, appends the `LotSelection`, then (if attested) `optimize_attest::set`. A failure AFTER
/// the committed append is routed through `rollback(session, &pre, e)` — symmetric with `persist_void`'s
/// clear-then-rollback — so no residue can piggy-back a later save. The final `save_or_rollback` does a
/// whole-DB restore on a failed save, reverting BOTH the append AND the side-table set. A retry after
/// a failed save re-appends with the SAME `decision_seq` (no residue). Voiding the resulting
/// `LotSelection` via `v` clears the attestation row (the shipped `persist_void` `optimize_attest::clear`
/// — closes the loop with zero `persist_void` changes).
///
/// # Duplicate guard is upstream
/// The MANDATORY duplicate-LotSelection guard (a disposal with a live `LotSelection` is never offered)
/// lives in the opener's `filter_optimize_candidates` pre-filter; a genuine duplicate that slips
/// through (a failed-save race) still fires `DecisionConflict` and NEITHER selection applies.
pub fn persist_optimize_accept(
    session: &mut btctax_cli::Session,
    disposal: btctax_core::EventId,
    picks: Vec<btctax_core::LotPick>,
    attestation: Option<String>,
    made: btctax_core::TaxDate,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    use btctax_core::event::LotSelection;
    use btctax_core::EventPayload;

    let pre = session.snapshot()?;
    let payload = EventPayload::LotSelection(LotSelection {
        disposal_event: disposal.clone(),
        lots: picks,
    });
    let id = btctax_core::persistence::append_decision(
        session.conn(),
        payload,
        now,
        time::UtcOffset::UTC,
        None,
    )?;
    if let Some(att) = attestation {
        // A failure AFTER the committed append must roll back — symmetric with persist_void's
        // clear-then-rollback (the INVERSE side-table op: set here, clear there).
        if let Err(e) =
            btctax_cli::optimize_attest::set(session.conn(), &disposal, &att, &made.to_string())
        {
            return Err(rollback(session, &pre, e));
        }
    }
    save_or_rollback(session, pre)?; // whole-DB restore reverts BOTH the append AND the side-table set
    Ok(id)
}

/// Store `DonationDetails` for `event_id` in the `donation_details` side-table
/// and atomically save the vault (last-write-wins upsert; NOT a decision event).
///
/// Mirrors `tax_profile::set` discipline (chunk 1 D3). No `append_decision` call.
/// `is_review_complete` is NOT checked here — it is checked post-save for the status string.
/// Reverted on a failed save via `save_or_rollback` (retry is clean).
pub fn persist_donation_details(
    session: &mut btctax_cli::Session,
    event_id: &btctax_core::EventId,
    details: &btctax_core::DonationDetails,
) -> Result<(), PersistError> {
    let pre = session.snapshot()?;
    btctax_cli::donation_details::set(session.conn(), event_id, details)?;
    save_or_rollback(session, pre)?;
    Ok(())
}

/// Void the existing live SafeHarborAllocation and re-append it as attested.
///
/// `prior_id` is the EventId of the live (non-voided, timebarred) allocation.
/// `prior_alloc` is the allocation payload (cloned from the pre-flight load).
/// `now` is INJECTED at Enter-press for test determinism.
///
/// # Two-decision atomic batch (reconcile.rs:541-563)
/// 1. Appends `VoidDecisionEvent{target_event_id: prior_id}` (inlines the void).
/// 2. Appends `SafeHarborAllocation{..prior_alloc, timely_allocation_attested: true}`.
///
/// Both land in the same in-memory Connection; the single `session.save()` flushes both.
///
/// # Failed-save + retry (NO retry path — Hard Constraints [R0-C1])
/// On `Err(save)`: the vault is pre-action on-disk, but BOTH appends remain in the
/// in-memory Connection — any later `session.save()` would flush them as a side effect
/// (the piggy-back hazard). The caller MUST set `app.attest_save_failed = true` (the
/// residue latch: all mutating openers refuse until the editor quits). A retry would
/// duplicate the batch → two effective allocations → Hard DecisionConflict + Path A
/// (resolve.rs:958-967), both copies §7.4-unvoidable — unrecoverable [R0-M2]. The flow
/// closes on Err; the safe remediation is QUIT (discards the residue), then the CLI.
pub fn persist_safe_harbor_attest(
    session: &mut btctax_cli::Session,
    prior_id: btctax_core::EventId,
    prior_alloc: btctax_core::event::SafeHarborAllocation,
    now: time::OffsetDateTime,
) -> Result<(btctax_core::EventId, btctax_core::EventId), btctax_cli::CliError> {
    use btctax_core::{
        event::{SafeHarborAllocation, VoidDecisionEvent},
        persistence::append_decision,
        EventPayload,
    };
    use time::UtcOffset;

    let void_id = append_decision(
        session.conn(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: prior_id,
        }),
        now,
        UtcOffset::UTC,
        None,
    )?;
    let attested = SafeHarborAllocation {
        timely_allocation_attested: true,
        ..prior_alloc
    };
    let attest_id = append_decision(
        session.conn(),
        EventPayload::SafeHarborAllocation(attested),
        now,
        UtcOffset::UTC,
        None,
    )?;
    session.save()?;
    Ok((void_id, attest_id))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── KAT-P1 — append-only prefix test (side-table form) ───────────────────
    //
    // For chunk 1: a tax-profile set is a SIDE-TABLE upsert, NOT an event append.
    // The degenerate strong form: event log UNCHANGED by a profile set.
    // In-memory AND after drop+reopen, plus:
    //   - mutation-actually-happened guard (profile round-trips)
    //   - second differing upsert still leaves log == pre

    fn fixture_profile() -> btctax_core::TaxProfile {
        use btctax_core::{Carryforward, FilingStatus, TaxProfile};
        use rust_decimal_macros::dec;
        TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: dec!(120000),
            magi_excluding_crypto: dec!(130000),
            qualified_dividends_and_other_pref_income: dec!(5000),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(0),
                long: dec!(0),
            },
            w2_ss_wages: dec!(80000),
            w2_medicare_wages: dec!(85000),
            schedule_c_expenses: dec!(3000),
        }
    }

    #[test]
    fn kat_p1_append_only_prefix_side_table_form() {
        use btctax_core::event::{EventPayload, MethodElection};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{LotMethod, TaxProfile};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p1-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed ≥ 2 decision events via append_decision (fixture setup — test-region exception)
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let now = OffsetDateTime::now_utc();
            let tz = UtcOffset::UTC;
            let p1 = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: LotMethod::Fifo,
            });
            let p2 = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2025 - 01 - 01),
                method: LotMethod::Hifo,
            });
            append_decision(session.conn(), p1, now, tz, None).unwrap();
            append_decision(session.conn(), p2, now, tz, None).unwrap();
            session.save().unwrap();
        }

        // Open the editor's session and capture the pre-state
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 2, "should have seeded exactly 2 events");

        let p = fixture_profile();
        persist_tax_profile(&mut session, 2025, &p).unwrap();

        // In-memory: log unchanged
        let post_inmem = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post_inmem, pre,
            "event log must be UNCHANGED in-memory after profile set (side-table upsert)"
        );

        // Drop + reopen: persisted image also unchanged
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, pre,
            "event log must be UNCHANGED on disk after profile set"
        );

        // Mutation-actually-happened guard (test cannot vacuously pass on a no-op)
        let stored = session2.tax_profile(2025).unwrap().unwrap();
        assert_eq!(
            stored, p,
            "profile must be readable after persist_tax_profile"
        );

        // Second differing upsert: log still == pre
        let p2 = TaxProfile {
            ordinary_taxable_income: dec!(200000),
            ..p.clone()
        };
        drop(session2);
        let mut session3 =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        persist_tax_profile(&mut session3, 2025, &p2).unwrap();
        let post3 = load_all_ordered(session3.conn()).unwrap();
        assert_eq!(
            post3, pre,
            "log still unchanged after second (differing) upsert"
        );
        let stored2 = session3.tax_profile(2025).unwrap().unwrap();
        assert_eq!(stored2, p2, "second upsert value is readable");
    }

    // ── KAT-P2a — append-only strict prefix test (classify-inbound append form) ──
    //
    // Invariant: persist_classify_inbound appends EXACTLY one decision event
    // to the tail of the event log.
    //
    // Strict-prefix formula (spec §D5):
    //   post == pre ++ [new_event]
    //   post[pre.len()].decision_seq == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
    //
    // Also asserts: payload round-trips, returned EventId matches appended row.

    #[test]
    fn kat_p2a_append_only_strict_prefix_classify_inbound() {
        use btctax_core::event::{ClassifyInbound, EventPayload, InboundClass, IncomeKind};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2a-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed 1 import TransferIn event + 1 decision event to create a non-trivial pre-state.
        // The import event is used as the ClassifyInbound target.
        let import_event_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2a"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![btctax_core::event::LedgerEvent {
                id: import_event_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: None,
                payload: btctax_core::event::EventPayload::TransferIn(
                    btctax_core::event::TransferIn {
                        sat: 100_000,
                        src_addr: None,
                        txid: None,
                    },
                ),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            // Seed one decision so MAX(decision_seq) == 1 in pre.
            let now = OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        };

        // Open editor session, capture pre-state.
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            pre.len(),
            2,
            "pre must have exactly 2 events (1 import + 1 decision)"
        );
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Build ClassifyInbound payload.
        let payload = EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: import_event_id.clone(),
            as_: InboundClass::Income {
                kind: IncomeKind::Mining,
                fmv: Some(dec!(30000.00)),
                business: false,
            },
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let returned_id = persist_classify_inbound(&mut session, payload.clone(), now).unwrap();

        // ── Strict-prefix assertion ───────────────────────────────────────────
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 1,
            "post must be exactly pre.len()+1"
        );

        // Pre-prefix is byte-for-byte identical.
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows must be unchanged (strict prefix)"
        );

        // New tail row: decision_seq == pre_max + 1.
        let tail = &post[pre.len()];
        let tail_seq = tail
            .decision_seq
            .expect("new tail row must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "tail decision_seq must be pre_max+1 (spec KAT-P2a formula)"
        );

        // Returned EventId matches the tail row's event_id.
        let tail_event_id = EventId::Decision {
            seq: tail_seq as u64,
        };
        assert_eq!(
            returned_id, tail_event_id,
            "returned EventId must equal Decision {{ seq: tail_seq }}"
        );

        // Payload round-trips: deserialise tail row and compare.
        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail payload_json must deserialise");
        assert_eq!(
            stored_payload, payload,
            "stored payload must round-trip equal to the one we appended"
        );

        // Drop + reopen: same strict-prefix holds on disk.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, post,
            "on-disk image must equal in-memory post after save"
        );
    }

    // ── KAT-P2b — append-only strict prefix test (reclassify-outflow append form) ──
    //
    // Invariant: persist_reclassify_outflow appends EXACTLY one decision event
    // to the tail of the event log.
    //
    // Strict-prefix formula (spec §D5):
    //   post == pre ++ [new_event]
    //   post[pre.len()].decision_seq == Some(pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0) + 1)
    //
    // Also asserts: payload round-trips, returned EventId matches appended row.

    #[test]
    fn kat_p2b_append_only_strict_prefix_reclassify_outflow() {
        use btctax_core::event::{EventPayload, OutflowClass, ReclassifyOutflow};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{DisposeKind, EventId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2b-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed 1 import TransferOut event + 1 decision event to create a non-trivial pre-state.
        let import_event_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2b"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![btctax_core::event::LedgerEvent {
                id: import_event_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: None,
                payload: btctax_core::event::EventPayload::TransferOut(
                    btctax_core::event::TransferOut {
                        sat: 100_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    },
                ),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            // Seed one decision so MAX(decision_seq) == 1 in pre.
            let now = OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        };

        // Open editor session, capture pre-state.
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            pre.len(),
            2,
            "pre must have exactly 2 events (1 import + 1 decision)"
        );
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Build ReclassifyOutflow payload.
        let payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: import_event_id.clone(),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(640.00),
            fee_usd: Some(dec!(2.50)),
            donee: None,
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let returned_id = persist_reclassify_outflow(&mut session, payload.clone(), now).unwrap();

        // ── Strict-prefix assertion ───────────────────────────────────────────
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 1,
            "post must be exactly pre.len()+1"
        );

        // Pre-prefix is byte-for-byte identical.
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows must be unchanged (strict prefix)"
        );

        // New tail row: decision_seq == pre_max + 1.
        let tail = &post[pre.len()];
        let tail_seq = tail
            .decision_seq
            .expect("new tail row must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "tail decision_seq must be pre_max+1 (spec KAT-P2b formula)"
        );

        // Returned EventId matches the tail row's event_id.
        let tail_event_id = EventId::Decision {
            seq: tail_seq as u64,
        };
        assert_eq!(
            returned_id, tail_event_id,
            "returned EventId must equal Decision {{ seq: tail_seq }}"
        );

        // Payload round-trips: deserialise tail row and compare.
        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail payload_json must deserialise");
        assert_eq!(
            stored_payload, payload,
            "stored payload must round-trip equal to the one we appended"
        );

        // Drop + reopen: same strict-prefix holds on disk.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, post,
            "on-disk image must equal in-memory post after save"
        );
    }

    // ── KAT-P2-LT — append-only strict prefix test (link-transfer append form) ──
    //
    // Invariant: persist_link_transfer appends EXACTLY one decision event to the tail.
    // Strict-prefix formula: post == pre ++ [new_event]; tail.decision_seq == pre_max+1.

    #[test]
    fn kat_p2_lt_append_only_strict_prefix_link_transfer() {
        use btctax_core::event::{EventPayload, TransferLink};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{EventId, TransferTarget, WalletId};
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2lt-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed 1 import TransferOut event + 1 decision event → non-trivial pre-state.
        let import_event_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2lt"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![btctax_core::event::LedgerEvent {
                id: import_event_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(WalletId::Exchange {
                    provider: "River".into(),
                    account: "main".into(),
                }),
                payload: EventPayload::TransferOut(btctax_core::event::TransferOut {
                    sat: 100_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        };

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 2, "pre must have exactly 2 events");
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        let payload = EventPayload::TransferLink(TransferLink {
            out_event: import_event_id.clone(),
            in_event_or_wallet: TransferTarget::Wallet(WalletId::Exchange {
                provider: "Kraken".into(),
                account: "cold".into(),
            }),
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let returned_id = persist_link_transfer(&mut session, payload.clone(), now).unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "post must be pre.len()+1");
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows must be unchanged (strict prefix)"
        );
        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail row must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "tail seq must be pre_max+1"
        );
        assert_eq!(
            returned_id,
            EventId::Decision {
                seq: tail_seq as u64
            },
            "returned EventId must equal Decision {{ seq }}"
        );
        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail payload_json must deserialise");
        assert_eq!(stored_payload, payload, "stored payload must round-trip");

        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(post_disk, post, "on-disk image must equal in-memory post");
    }

    // ── KAT-P2-CR — append-only strict prefix test (classify-raw append form) ──
    //
    // Invariant: persist_classify_raw appends EXACTLY one decision event to the tail.

    #[test]
    fn kat_p2_cr_append_only_strict_prefix_classify_raw() {
        use btctax_core::event::{ClassifyRaw, EventPayload, Income};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{EventId, FmvStatus, IncomeKind};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2cr-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed 1 import Unclassified event + 1 decision event → non-trivial pre-state.
        let import_event_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2cr"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![btctax_core::event::LedgerEvent {
                id: import_event_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: None,
                payload: EventPayload::Unclassified(btctax_core::event::Unclassified {
                    raw: "weird row".into(),
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        };

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 2, "pre must have exactly 2 events");
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Build ClassifyRaw{target, as_: Income} DIRECTLY (not via InboundClass).
        let payload = EventPayload::ClassifyRaw(ClassifyRaw {
            target: import_event_id.clone(),
            as_: Box::new(EventPayload::Income(Income {
                sat: 100_000,
                usd_fmv: Some(dec!(65.00)),
                fmv_status: FmvStatus::ManualEntry,
                kind: IncomeKind::Mining,
                business: true,
            })),
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let returned_id = persist_classify_raw(&mut session, payload.clone(), now).unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "post must be pre.len()+1");
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows must be unchanged (strict prefix)"
        );
        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail row must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "tail seq must be pre_max+1"
        );
        assert_eq!(
            returned_id,
            EventId::Decision {
                seq: tail_seq as u64
            },
            "returned EventId must equal Decision {{ seq }}"
        );
        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail payload_json must deserialise");
        assert_eq!(stored_payload, payload, "stored payload must round-trip");

        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(post_disk, post, "on-disk image must equal in-memory post");
    }

    // ── KAT-G1 — the editor's mechanized source gate ─────────────────────────
    //
    // Clones the E10 scanner structure (export.rs:690–919 in btctax-tui):
    //   - src-walk via CARGO_MANIFEST_DIR
    //   - non-test/test region split at first #[cfg(test)]
    //   - // comment stripping before matching
    //   - file:line failure output
    //   - plant-a-token self-check with runtime-constructed strings
    //
    // Allowlist: edit/persist.rs is the ONLY file permitted to use the write-mutation
    // tokens (conn( / save( / tax_profile::set / append_) in non-test code.
    //
    // R0-I1: Session::create / Session::repair / Vault::create / Vault::repair are
    // FORBIDDEN everywhere in non-test code (they create/overwrite a vault file
    // outside Vault::save's atomic path). One of these (Session::create) is planted
    // in the self-check so the gate cannot silently drop R0-I1 enforcement.

    #[test]
    fn kat_g1_mechanized_source_gate() {
        use std::io::{BufRead, BufReader};

        // ── Locate this crate's src/ directory ────────────────────────────────
        let src_dir = {
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR must be set in tests");
            std::path::PathBuf::from(manifest).join("src")
        };
        assert!(
            src_dir.exists(),
            "btctax-tui-edit/src must exist at {:?}",
            src_dir
        );

        // ── Token lists ───────────────────────────────────────────────────────

        // Non-test forbidden everywhere (no allowlist):
        //   cmd:: — cmd fns open/drop their own sessions (wrong lifecycle, deadlocks held lock)
        //   Session::create / Session::repair / Vault::create / Vault::repair — R0-I1:
        //     these constructors create/overwrite a vault file outside Vault::save's atomic path
        //   export_snapshot / write_csv_exports / write_form_csvs — viewer-only export surface
        let everywhere_tokens: &[&str] = &[
            "cmd::",
            "Session::create",
            "Session::repair",
            "Vault::create",
            "Vault::repair",
            "export_snapshot",
            "write_csv_exports",
            "write_form_csvs",
        ];

        // Non-test FS-write tokens forbidden everywhere (editor performs NO direct fs writes;
        // vault writes go only via Vault::save's atomic path inside btctax-store):
        let fs_write_tokens: &[&str] = &[
            "fsperms",
            "open_owner_only",
            "mkdir_owner_only",
            "File::create",
            "File::options",
            "OpenOptions",
            "fs::write",
            "write_owner_only",
            "create_dir",
            "DirBuilder",
            "set_permissions",
            "fs::copy",
            "fs::rename",
            "fs::remove_",
        ];

        // Non-test write-mutation tokens — FORBIDDEN outside edit/persist.rs:
        // [R0-I4] "donation_details::set" added for the D2 side-table writer parity.
        // [R0-M2] "restore(" added: Session::restore reverts the in-memory DB, so it is a
        // mutation-surface token confinable to edit/persist.rs. Note: this is NOT a false positive
        // for ratatui teardown (that is `restore_terminal` = `restore_`, not `restore(`), and
        // `snapshot(` is deliberately NOT gated (a pure read, and `build_snapshot(` contains it).
        let persist_only_tokens: &[&str] = &[
            "conn(",
            "save(",
            "tax_profile::set",
            "append_",
            "donation_details::set",
            "optimize_attest::set",
            "restore(",
        ];

        // Test-region forbidden everywhere (no viewer export surface in the editor):
        let test_region_tokens: &[&str] =
            &["export_snapshot", "write_csv_exports", "write_form_csvs"];

        // ── Comment stripping [M-R2-1] ────────────────────────────────────────
        /// Strip // comment suffix (covers // and /// doc-comments).
        fn strip_comment(line: &str) -> &str {
            if let Some(idx) = line.find("//") {
                &line[..idx]
            } else {
                line
            }
        }

        // ── Scan helper: non-test region ──────────────────────────────────────
        fn scan_non_test(path: &std::path::Path, tokens: &[&str]) -> Vec<(String, usize)> {
            let file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => return vec![],
            };
            let reader = BufReader::new(file);
            let mut hits = Vec::new();
            let mut in_test = false;
            for (idx, line) in reader.lines().enumerate() {
                let line = line.unwrap_or_default();
                if line.trim_start().starts_with("#[cfg(test)]") {
                    in_test = true;
                }
                if !in_test {
                    let code = strip_comment(&line);
                    for &tok in tokens {
                        if code.contains(tok) {
                            hits.push((tok.to_string(), idx + 1));
                        }
                    }
                }
            }
            hits
        }

        // ── Scan helper: test region ──────────────────────────────────────────
        fn scan_test_region(path: &std::path::Path, tokens: &[&str]) -> Vec<(String, usize)> {
            let content = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let test_start = match content.find("#[cfg(test)]") {
                Some(pos) => pos,
                None => return vec![],
            };
            let test_region = &content[test_start..];
            let prefix_line = content[..test_start].lines().count();
            let mut hits = Vec::new();
            for (idx, line) in test_region.lines().enumerate() {
                let code = strip_comment(line);
                for &tok in tokens {
                    if code.contains(tok) {
                        hits.push((tok.to_string(), prefix_line + idx + 1));
                    }
                }
            }
            hits
        }

        // ── Collect all .rs files under src/ ─────────────────────────────────
        let mut rs_files: Vec<std::path::PathBuf> = Vec::new();
        fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        collect_rs(&p, out);
                    } else if p.extension().is_some_and(|e| e == "rs") {
                        out.push(p);
                    }
                }
            }
        }
        collect_rs(&src_dir, &mut rs_files);
        assert!(
            !rs_files.is_empty(),
            "must find at least one .rs file in src/"
        );

        // ── Classify each file ────────────────────────────────────────────────
        // edit/persist.rs is the allowlisted file for write-mutation tokens and is
        // excluded from the test-region scan (mirrors viewer's exclusion of export.rs).
        fn is_persist_rs(path: &std::path::Path) -> bool {
            let fname = path.file_name().map(|n| n == "persist.rs").unwrap_or(false);
            let in_edit = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|d| d == "edit")
                .unwrap_or(false);
            fname && in_edit
        }

        // ── Scan each file ────────────────────────────────────────────────────
        let mut violations: Vec<String> = Vec::new();

        for path in &rs_files {
            let is_persist = is_persist_rs(path);

            // (1) everywhere_tokens in non-test region of ALL files.
            {
                let hits = scan_non_test(path, everywhere_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden token {:?} (everywhere rule, non-test region)",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // (2) fs_write_tokens in non-test region of ALL files.
            {
                let hits = scan_non_test(path, fs_write_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden FS-write token {:?} in non-test region",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // (3) write-mutation tokens in non-test of ALL files EXCEPT edit/persist.rs.
            if !is_persist {
                let hits = scan_non_test(path, persist_only_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden write-mutation token {:?} outside edit/persist.rs",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }

            // (4) test-region forbidden tokens in ALL files EXCEPT edit/persist.rs.
            // (edit/persist.rs excluded: its test region contains this gate + self-check.)
            if !is_persist {
                let hits = scan_test_region(path, test_region_tokens);
                for (tok, lineno) in hits {
                    violations.push(format!(
                        "{}:{} — forbidden viewer-export token {:?} in test region",
                        path.display(),
                        lineno,
                        tok
                    ));
                }
            }
        }

        // ── Self-check: verify the scanner catches planted tokens ─────────────
        //
        // All tokens are runtime-constructed so NO literal forbidden token appears
        // in this source file (avoids false positives when edit/persist.rs is scanned).
        {
            let tmpdir = tempfile::tempdir().unwrap();
            let planted_path = tmpdir.path().join("planted_test.rs");

            // Construct forbidden tokens at runtime — never appear literally in source.
            let tok_save = format!("{}(", "save"); // "save("
            let tok_conn = format!("{}(", "conn"); // "conn("
            let tok_restore = format!("{}(", "restore"); // "restore(" [R0-M2]
            let tok_tax_set = format!("{}::{}", "tax_profile", "set"); // "tax_profile::set"
            let tok_session_create = format!("{}::{}", "Session", "create"); // "Session::create" [R0-I1]
                                                                             // [R0-I4]: donation_details::set added to persist_only_tokens.
            let tok_dd_set = format!("{}::{}", "donation_details", "set"); // "donation_details::set"
                                                                           // chunk4b: optimize_attest::set added to persist_only_tokens.
            let tok_oa_set = format!("{}::{}", "optimize_attest", "set"); // "optimize_attest::set"

            let content = format!(
                "// planted self-check file\n\
                 pub fn bad() {{\n\
                 \tlet _ = {tok_save});\n\
                 \tlet _ = {tok_conn});\n\
                 \tlet _ = {tok_restore});\n\
                 \tlet _ = {tok_tax_set}(conn, 2025, &p);\n\
                 \tlet _ = {tok_session_create}(&path, &pp);\n\
                 \tlet _ = {tok_dd_set}(conn, &id, &d);\n\
                 \tlet _ = {tok_oa_set}(conn, &id, &a, &at);\n\
                 }}\n"
            );
            std::fs::write(&planted_path, &content).unwrap();

            // Verify scanner catches persist-only tokens.
            let hits_persist = scan_non_test(&planted_path, persist_only_tokens);
            assert!(
                hits_persist.iter().any(|(t, _)| t == "save("),
                "self-check FAILED: scanner did not detect planted write-mutation token — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "conn("),
                "self-check FAILED: scanner did not detect planted write-mutation token — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "restore("),
                "self-check FAILED: scanner did not detect planted restore( token [R0-M2] — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "tax_profile::set"),
                "self-check FAILED: scanner did not detect planted write-mutation token — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "donation_details::set"),
                "self-check FAILED: scanner did not detect planted donation_details::set token [R0-I4] — gate is broken"
            );
            assert!(
                hits_persist.iter().any(|(t, _)| t == "optimize_attest::set"),
                "self-check FAILED: scanner did not detect planted optimize_attest::set token [chunk4b] — gate is broken"
            );

            // Verify scanner catches the R0-I1 vault-creating constructor.
            let hits_everywhere = scan_non_test(&planted_path, everywhere_tokens);
            assert!(
                hits_everywhere.iter().any(|(t, _)| t == "Session::create"),
                "self-check FAILED: scanner did not detect planted Session::create [R0-I1] — gate is broken"
            );
        }

        // ── Assert clean ──────────────────────────────────────────────────────
        assert!(
            violations.is_empty(),
            "KAT-G1 source gate violations found:\n{}",
            violations.join("\n")
        );
    }

    // ── KAT-P2c — append-only strict prefix test (reclassify-income append form) ──
    //
    // Invariant: persist_reclassify_income appends EXACTLY one decision event
    // to the tail of the event log.

    #[test]
    fn kat_p2c_append_only_strict_prefix_reclassify_income() {
        use btctax_core::event::{
            EventPayload, FmvStatus, Income, IncomeKind, LedgerEvent, MethodElection,
            ReclassifyIncome,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2c-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let income_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2c"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: income_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: Some(rust_decimal_macros::dec!(30000)),
                    fmv_status: FmvStatus::PriceDataset,
                    kind: IncomeKind::Staking,
                    business: false,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();
            let p = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            pre.len(),
            2,
            "pre must have 2 events (1 import + 1 decision)"
        );
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1);

        let payload = EventPayload::ReclassifyIncome(ReclassifyIncome {
            income_event: income_id.clone(),
            business: true,
            kind: None,
        });
        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();

        let returned_id = persist_reclassify_income(&mut session, payload.clone(), now).unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1);
        assert_eq!(&post[..pre.len()], pre.as_slice(), "strict prefix");

        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(tail_seq, (pre_max_seq + 1) as i64);

        let tail_event_id = EventId::Decision {
            seq: tail_seq as u64,
        };
        assert_eq!(returned_id, tail_event_id);

        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("must deserialise");
        assert_eq!(stored_payload, payload);

        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(post_disk, post, "on-disk must equal in-memory post");
    }

    // ── KAT-P2d — append-only strict prefix test (set-fmv / ManualFmv append form) ──
    //
    // Invariant: persist_set_fmv appends EXACTLY one decision event
    // to the tail of the event log.

    #[test]
    fn kat_p2d_append_only_strict_prefix_set_fmv() {
        use btctax_core::event::{
            EventPayload, FmvStatus, Income, IncomeKind, LedgerEvent, ManualFmv, MethodElection,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2d-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let income_id: EventId = EventId::import(Source::River, SourceRef::new("ref-p2d"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: income_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: None,
                    fmv_status: FmvStatus::Missing,
                    kind: IncomeKind::Staking,
                    business: false,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();
            let p = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 2);
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1);

        let payload = EventPayload::ManualFmv(ManualFmv {
            event: income_id.clone(),
            usd_fmv: dec!(45.00),
        });
        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();

        let returned_id = persist_set_fmv(&mut session, payload.clone(), now).unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1);
        assert_eq!(&post[..pre.len()], pre.as_slice(), "strict prefix");

        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(tail_seq, (pre_max_seq + 1) as i64);

        let tail_event_id = EventId::Decision {
            seq: tail_seq as u64,
        };
        assert_eq!(returned_id, tail_event_id);

        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("must deserialise");
        assert_eq!(stored_payload, payload);

        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(post_disk, post, "on-disk must equal in-memory post");
    }

    // ── KAT-P2e — append-only strict prefix test (void / VoidDecisionEvent append) ─
    //
    // Invariant: persist_void appends EXACTLY one VoidDecisionEvent to the tail;
    // the existing prefix is unchanged; the tail round-trips as VoidDecisionEvent
    // targeting the seeded MethodElection decision.

    #[test]
    fn kat_p2e_append_only_strict_prefix_void() {
        use btctax_core::event::{EventPayload, MethodElection};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2e-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed a MethodElection decision (the target for the void).
        let me_id: EventId;
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            let p = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            let id = append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            me_id = id;
            session.save().unwrap();
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 1, "pre must have 1 event (the MethodElection)");
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1);

        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let returned_id = persist_void(&mut session, me_id.clone(), now).unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "post must have pre + 1 rows");
        assert_eq!(&post[..pre.len()], pre.as_slice(), "strict prefix");

        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "tail seq must be pre_max+1"
        );

        let tail_event_id = EventId::Decision {
            seq: tail_seq as u64,
        };
        assert_eq!(returned_id, tail_event_id, "returned id must match tail");

        let stored_payload: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail must deserialise");
        match &stored_payload {
            EventPayload::VoidDecisionEvent(v) => {
                assert_eq!(
                    v.target_event_id, me_id,
                    "void must target the seeded MethodElection"
                );
            }
            other => panic!("expected VoidDecisionEvent, got {other:?}"),
        }

        // Drop + reopen: same strict-prefix holds on disk.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(post_disk, post, "on-disk must equal in-memory post");
    }

    // ── KAT-P2f — persist_void clears optimize_attest on LotSelection; MethodElection untouched ──
    //
    // Pins the LotSelection arm of persist_void (reconcile.rs:117–147 / persist.rs:197–217):
    //   1. LotSelection arm: void appends VoidDecisionEvent AND clears the optimize_attest
    //      side-table row for the disposal atomically in the same session.save().
    //   2. MethodElection arm (branch-not-taken): void appends VoidDecisionEvent but does NOT
    //      touch the optimize_attest table — a row planted for an unrelated disposal survives
    //      intact.
    //
    // Mirror of the CLI twin at btctax-cli/tests/optimize_accept.rs:649
    // (`void_clears_attestation_row_prevents_mislabel_as_attested_recording`).

    #[test]
    fn kat_p2f_void_lot_selection_clears_optimize_attest_method_election_does_not() {
        use btctax_core::event::{EventPayload, LotPick, LotSelection, MethodElection};
        use btctax_core::identity::{LotId, Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2f-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Synthetic disposal EventId. persist_void reads the LotSelection payload for its
        // disposal_event field; the disposal need not exist as an imported ledger event.
        let disposal_id = EventId::import(Source::River, SourceRef::new("p2f-disposal"));
        let lot_origin = EventId::import(Source::River, SourceRef::new("p2f-lot"));

        // ── Seed: LotSelection decision + MethodElection decision + optimize_attest row ──
        let ls_id: EventId;
        let me_id: EventId;
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            let t1 = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();

            let ls_payload = EventPayload::LotSelection(LotSelection {
                disposal_event: disposal_id.clone(),
                lots: vec![LotPick {
                    lot: LotId {
                        origin_event_id: lot_origin,
                        split_sequence: 0,
                    },
                    sat: 100_000,
                }],
            });
            ls_id = append_decision(session.conn(), ls_payload, t0, UtcOffset::UTC, None).unwrap();

            let me_payload = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            me_id = append_decision(session.conn(), me_payload, t1, UtcOffset::UTC, None).unwrap();

            // Seed the optimize_attest side-table entry for the disposal.
            btctax_cli::optimize_attest::set(
                session.conn(),
                &disposal_id,
                "p2f-attestation-text",
                "2025-06-01",
            )
            .unwrap();

            session.save().unwrap();
        }

        // ── LotSelection arm: void appends event AND clears optimize_attest ────
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let pre = load_all_ordered(session.conn()).unwrap();
            assert_eq!(pre.len(), 2, "pre must have 2 decision events");
            let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);

            // Confirm optimize_attest row is present before void.
            let pre_attest =
                btctax_cli::optimize_attest::get(session.conn(), &disposal_id).unwrap();
            assert_eq!(
                pre_attest.as_deref(),
                Some("p2f-attestation-text"),
                "pre-void: optimize_attest row must be present"
            );

            let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
            let returned_id = persist_void(&mut session, ls_id.clone(), now).unwrap();

            // Strict-prefix: void is the only new event.
            let post = load_all_ordered(session.conn()).unwrap();
            assert_eq!(
                post.len(),
                pre.len() + 1,
                "void must append exactly one event"
            );
            assert_eq!(
                &post[..pre.len()],
                pre.as_slice(),
                "strict prefix preserved"
            );

            // Tail is a VoidDecisionEvent targeting the LotSelection.
            let tail = &post[pre.len()];
            let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
            assert_eq!(
                tail_seq,
                (pre_max_seq + 1) as i64,
                "tail seq must be pre_max+1"
            );
            assert_eq!(
                returned_id,
                EventId::Decision {
                    seq: tail_seq as u64
                },
                "returned id must match tail"
            );
            let stored: EventPayload =
                serde_json::from_str(&tail.payload_json).expect("tail must deserialise");
            match &stored {
                EventPayload::VoidDecisionEvent(v) => {
                    assert_eq!(
                        v.target_event_id, ls_id,
                        "void must target the seeded LotSelection"
                    );
                }
                other => panic!("expected VoidDecisionEvent, got {other:?}"),
            }

            // PRIMARY assertion: optimize_attest row must be cleared atomically.
            let post_attest =
                btctax_cli::optimize_attest::get(session.conn(), &disposal_id).unwrap();
            assert_eq!(
                post_attest, None,
                "void of a LotSelection must clear the optimize_attest row atomically"
            );
        }

        // ── MethodElection arm (branch-not-taken): optimize_attest untouched ──
        //
        // Re-open, plant a fresh attest row for a different disposal, then void the
        // MethodElection. The row must survive — MethodElection void does NOT call
        // optimize_attest::clear.
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();

            let other_disposal = EventId::import(Source::River, SourceRef::new("p2f-other"));
            btctax_cli::optimize_attest::set(
                session.conn(),
                &other_disposal,
                "p2f-other-attestation",
                "2025-07-01",
            )
            .unwrap();

            let now = OffsetDateTime::from_unix_timestamp(1_748_003_000).unwrap();
            persist_void(&mut session, me_id.clone(), now).unwrap();

            // Attest row for other_disposal must be UNTOUCHED.
            let attest_after =
                btctax_cli::optimize_attest::get(session.conn(), &other_disposal).unwrap();
            assert_eq!(
                attest_after.as_deref(),
                Some("p2f-other-attestation"),
                "MethodElection void must NOT touch the optimize_attest side-table"
            );
        }
    }

    // ── save-rollback: persist_void rolls back the side-table clear on a failed save ──
    //
    // persist_void clears optimize_attest BEFORE save; if the save fails, the whole-DB rollback
    // must revert BOTH the void append AND the side-table clear. (A per-row DELETE would miss the
    // side-table — the load-bearing reason A′ uses whole-DB restore.)
    #[cfg(unix)]
    #[test]
    fn kat_persist_void_rollback_preserves_optimize_attest_on_failed_save() {
        use btctax_core::event::{EventPayload, LotPick, LotSelection};
        use btctax_core::identity::{LotId, Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use std::os::unix::fs::PermissionsExt;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-rollback-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let disposal_id = EventId::import(Source::River, SourceRef::new("vr-disposal"));
        let lot_origin = EventId::import(Source::River, SourceRef::new("vr-lot"));

        let ls_id: EventId;
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            let ls_payload = EventPayload::LotSelection(LotSelection {
                disposal_event: disposal_id.clone(),
                lots: vec![LotPick {
                    lot: LotId {
                        origin_event_id: lot_origin,
                        split_sequence: 0,
                    },
                    sat: 100_000,
                }],
            });
            ls_id = append_decision(session.conn(), ls_payload, t0, UtcOffset::UTC, None).unwrap();
            btctax_cli::optimize_attest::set(
                session.conn(),
                &disposal_id,
                "attestation",
                "2025-06-01",
            )
            .unwrap();
            session.save().unwrap();
        }

        // Root-skip guard (chmod is a no-op as root).
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("void-rollback KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();

        // Make the vault's parent read-only → save() fails inside persist_void.
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = persist_void(&mut session, ls_id.clone(), now);
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must return RolledBack; got: {result:?}"
        );

        // The in-memory log is reverted: NO void row appended.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len(),
            "rollback must revert the void append (no residue)"
        );

        // The side-table clear was reverted too: the optimize_attest row SURVIVES.
        let attest = btctax_cli::optimize_attest::get(session.conn(), &disposal_id).unwrap();
        assert_eq!(
            attest.as_deref(),
            Some("attestation"),
            "rollback must restore the optimize_attest row cleared before the failed save"
        );
    }

    // ── save-rollback: persist_donation_details reverts the side-table upsert on a failed save ──
    #[cfg(unix)]
    #[test]
    fn kat_persist_donation_details_rollback_on_failed_save() {
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::{DonationDetails, EventId};
        use btctax_store::Passphrase;
        use std::os::unix::fs::PermissionsExt;

        fn details(donee: &str, appraiser: &str) -> DonationDetails {
            DonationDetails {
                donee_name: donee.to_owned(),
                donee_address: None,
                donee_ein: None,
                appraiser_name: appraiser.to_owned(),
                appraiser_address: None,
                appraiser_tin: None,
                appraiser_ptin: None,
                appraiser_qualifications: None,
                appraisal_date: None,
                fmv_method_override: None,
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-dd-rollback-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let event_id = EventId::import(Source::River, SourceRef::new("dd-donation"));
        let original = details("Original Donee", "Original Appraiser");
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            persist_donation_details(&mut session, &event_id, &original).unwrap();
        }

        // Root-skip guard.
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("dd-rollback KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let updated = details("UPDATED Donee", "UPDATED Appraiser");
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = persist_donation_details(&mut session, &event_id, &updated);
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must return RolledBack; got: {result:?}"
        );

        // The in-memory side-table reverts to the ORIGINAL value (the failed upsert did not land).
        let got = btctax_cli::donation_details::get(session.conn(), &event_id).unwrap();
        assert_eq!(
            got.as_ref(),
            Some(&original),
            "rollback must revert the donation_details upsert to its prior value"
        );
    }

    // ── KAT-P2g — append-only strict prefix test (select-lots / LotSelection append) ──
    //
    // Seed: Acquire (wallet W) + TransferOut + ReclassifyOutflow(Donate) → a Donation
    // removal in projected state. The `LotSelection` payload references the TransferOut
    // EventId as `disposal_event` and the Acquire's lot as `LotPick`.
    //
    // Strict-prefix formula:
    //   post.len() == pre.len() + 1
    //   post[..pre.len()] == pre[..]
    //   post[pre.len()].kind == "decision"
    //   post[pre.len()].decision_seq == Some(pre_max + 1)
    //   payload round-trips as LotSelection targeting the expected disposal

    #[test]
    fn kat_p2g_append_only_strict_prefix_select_lots() {
        use btctax_core::event::{
            Acquire, EventPayload, LedgerEvent, LotPick, LotSelection, MethodElection,
            OutflowClass, ReclassifyOutflow, TransferOut,
        };
        use btctax_core::identity::{LotId, Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch, load_all_ordered};
        use btctax_core::{EventId, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2g-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let wallet = WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        };

        let acquire_id = EventId::import(Source::River, SourceRef::new("p2g-acquire"));
        let out_id = EventId::import(Source::River, SourceRef::new("p2g-out"));
        let lot_id = LotId {
            origin_event_id: acquire_id.clone(),
            split_sequence: 0,
        };

        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();

            // Seed: Acquire + TransferOut.
            let t0 = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
            let t1 = OffsetDateTime::from_unix_timestamp(1_700_086_400).unwrap();
            let batch = vec![
                LedgerEvent {
                    id: acquire_id.clone(),
                    utc_timestamp: t0,
                    original_tz: UtcOffset::UTC,
                    wallet: Some(wallet.clone()),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 500_000,
                        usd_cost: dec!(15000),
                        fee_usd: dec!(0),
                        basis_source: btctax_core::event::BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: out_id.clone(),
                    utc_timestamp: t1,
                    original_tz: UtcOffset::UTC,
                    wallet: Some(wallet.clone()),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: 500_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            append_import_batch(session.conn(), &batch).unwrap();

            // Seed: ReclassifyOutflow(Donate) decision + MethodElection.
            let t2 = OffsetDateTime::from_unix_timestamp(1_700_100_000).unwrap();
            let ro_payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: out_id.clone(),
                as_: OutflowClass::Donate {
                    appraisal_required: false,
                },
                principal_proceeds_or_fmv: dec!(20000),
                fee_usd: None,
                donee: None,
            });
            append_decision(session.conn(), ro_payload, t2, UtcOffset::UTC, None).unwrap();
            let me_payload = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(session.conn(), me_payload, t2, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        }

        // Open editor session, capture pre-state.
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);

        // Build LotSelection payload.
        let payload = EventPayload::LotSelection(LotSelection {
            disposal_event: out_id.clone(),
            lots: vec![LotPick {
                lot: lot_id.clone(),
                sat: 500_000,
            }],
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_200_000).unwrap();

        let returned_id = persist_select_lots(&mut session, payload.clone(), now).unwrap();

        // ── Strict-prefix assertion ───────────────────────────────────────────
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 1,
            "KAT-P2g: post must be pre.len()+1"
        );
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "KAT-P2g: first pre.len() rows must be unchanged (strict prefix)"
        );

        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(
            tail_seq,
            (pre_max_seq + 1) as i64,
            "KAT-P2g: tail seq must be pre_max+1"
        );
        assert_eq!(
            returned_id,
            EventId::Decision {
                seq: tail_seq as u64
            },
            "KAT-P2g: returned id must match tail"
        );

        // Payload round-trips.
        let stored: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail must deserialise");
        match &stored {
            EventPayload::LotSelection(ls) => {
                assert_eq!(
                    ls.disposal_event, out_id,
                    "KAT-P2g: LotSelection must target out_id"
                );
                assert_eq!(ls.lots.len(), 1, "KAT-P2g: one lot pick");
                assert_eq!(ls.lots[0].lot, lot_id, "KAT-P2g: lot id matches");
                assert_eq!(ls.lots[0].sat, 500_000, "KAT-P2g: sat matches");
            }
            other => panic!("KAT-P2g: expected LotSelection, got {other:?}"),
        }

        // Drop + reopen.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, post,
            "KAT-P2g: on-disk must equal in-memory post"
        );
    }

    // ── KAT-DD-PERSIST — side-table write test for persist_donation_details ───
    //
    // Not a strict-prefix test (no decision event). Pattern mirrors `persist_tax_profile`'s KAT:
    // - Create temp vault; seed a Donation removal event.
    // - Call persist_donation_details → assert side-table has the stored value.
    // - Drop + reopen: same value on disk.
    // - Call again with full_details (upsert): assert the value is replaced.
    // - Assert event log has NO new decision rows (strict: post.len() == pre.len()).

    #[test]
    fn kat_dd_persist_side_table_write() {
        use btctax_core::event::{
            Acquire, EventPayload, LedgerEvent, OutflowClass, ReclassifyOutflow, TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch, load_all_ordered};
        use btctax_core::{DonationDetails, EventId, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-dd-persist-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let wallet = WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        };
        let acquire_id = EventId::import(Source::River, SourceRef::new("dd-acquire"));
        let out_id = EventId::import(Source::River, SourceRef::new("dd-out"));

        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
            let t1 = OffsetDateTime::from_unix_timestamp(1_700_086_400).unwrap();
            let batch = vec![
                LedgerEvent {
                    id: acquire_id.clone(),
                    utc_timestamp: t0,
                    original_tz: UtcOffset::UTC,
                    wallet: Some(wallet.clone()),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 500_000,
                        usd_cost: dec!(15000),
                        fee_usd: dec!(0),
                        basis_source: btctax_core::event::BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: out_id.clone(),
                    utc_timestamp: t1,
                    original_tz: UtcOffset::UTC,
                    wallet: Some(wallet.clone()),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: 500_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            append_import_batch(session.conn(), &batch).unwrap();
            let t2 = OffsetDateTime::from_unix_timestamp(1_700_100_000).unwrap();
            let ro_payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: out_id.clone(),
                as_: OutflowClass::Donate {
                    appraisal_required: false,
                },
                principal_proceeds_or_fmv: dec!(20000),
                fee_usd: None,
                donee: None,
            });
            append_decision(session.conn(), ro_payload, t2, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        }

        let minimal_details = DonationDetails {
            donee_name: "Test Charity".to_owned(),
            donee_address: None,
            donee_ein: None,
            appraiser_name: "Test Appraiser".to_owned(),
            appraiser_address: None,
            appraiser_tin: None,
            appraiser_ptin: None,
            appraiser_qualifications: None,
            appraisal_date: None,
            fmv_method_override: None,
        };
        let full_details = DonationDetails {
            donee_name: "Test Charity Full".to_owned(),
            donee_address: Some("123 Main St".to_owned()),
            donee_ein: Some("12-3456789".to_owned()),
            appraiser_name: "Jane Appraiser".to_owned(),
            appraiser_address: Some("456 Appraise Ave".to_owned()),
            appraiser_tin: Some("987654321".to_owned()),
            appraiser_ptin: None,
            appraiser_qualifications: Some("Certified BTC Appraiser".to_owned()),
            appraisal_date: Some(time::macros::date!(2025 - 05 - 20)),
            fmv_method_override: None,
        };

        // Open editor session, capture pre-state.
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();

        // First persist (minimal_details).
        persist_donation_details(&mut session, &out_id, &minimal_details).unwrap();

        // In-memory: side-table has the stored value.
        let stored = btctax_cli::donation_details::get(session.conn(), &out_id).unwrap();
        assert_eq!(
            stored,
            Some(minimal_details.clone()),
            "KAT-DD-PERSIST: minimal_details must be stored in-memory"
        );

        // Event log unchanged.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post, pre,
            "KAT-DD-PERSIST: event log must be UNCHANGED after donation_details set"
        );

        // Drop + reopen: same value on disk.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let stored_disk = btctax_cli::donation_details::get(session2.conn(), &out_id).unwrap();
        assert_eq!(
            stored_disk,
            Some(minimal_details.clone()),
            "KAT-DD-PERSIST: minimal_details must be on disk after drop+reopen"
        );
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, pre,
            "KAT-DD-PERSIST: event log unchanged on disk"
        );
        drop(session2);

        // Upsert: second persist replaces prior value.
        let mut session3 =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        persist_donation_details(&mut session3, &out_id, &full_details).unwrap();
        let stored2 = btctax_cli::donation_details::get(session3.conn(), &out_id).unwrap();
        assert_eq!(
            stored2,
            Some(full_details.clone()),
            "KAT-DD-PERSIST: upsert must replace minimal_details with full_details"
        );
        let post2 = load_all_ordered(session3.conn()).unwrap();
        assert_eq!(
            post2, pre,
            "KAT-DD-PERSIST: event log unchanged after upsert"
        );
    }

    // ── KAT-P2h — two-decision strict-prefix test for persist_safe_harbor_attest ──
    //
    // Invariant: persist_safe_harbor_attest appends EXACTLY TWO decision events:
    //   post[pre.len()]   = VoidDecisionEvent targeting prior_id
    //   post[pre.len()+1] = SafeHarborAllocation with timely_allocation_attested=true
    //
    // Strict-prefix formula (spec D5 KAT-P2h):
    //   post.len() == pre.len() + 2
    //   post[..pre.len()] == pre[..]
    //   post[pre.len()].kind == "decision"
    //   post[pre.len()].decision_seq == pre_max_seq + 1
    //   post[pre.len()+1].kind == "decision"
    //   post[pre.len()+1].decision_seq == pre_max_seq + 2
    //   payload round-trips: VoidDecisionEvent{ target_event_id: prior_id }
    //   payload round-trips: SafeHarborAllocation { ..prior_alloc, timely_allocation_attested: true }

    #[test]
    fn kat_p2h_persist_safe_harbor_attest_two_decision_strict_prefix() {
        use btctax_core::event::{
            AllocMethod, EventPayload, SafeHarborAllocation, VoidDecisionEvent,
        };
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{EventId, LotMethod};
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2h-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed: a SafeHarborAllocation with timely_allocation_attested: false + one extra decision.
        let prior_alloc = SafeHarborAllocation {
            lots: vec![],
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ActualPosition,
            timely_allocation_attested: false,
            pre2025_method: LotMethod::Fifo,
        };
        let prior_id: EventId;
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            let t1 = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();
            // Seed an extra decision first so pre_max_seq > 1.
            append_decision(
                session.conn(),
                EventPayload::MethodElection(btctax_core::event::MethodElection {
                    effective_from: date!(2024 - 01 - 01),
                    method: LotMethod::Fifo,
                }),
                t0,
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            prior_id = append_decision(
                session.conn(),
                EventPayload::SafeHarborAllocation(prior_alloc.clone()),
                t1,
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        // Open editor session, capture pre-state.
        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 2, "pre must have 2 events");
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 2, "pre max decision_seq must be 2");

        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let (void_id, attest_id) =
            persist_safe_harbor_attest(&mut session, prior_id.clone(), prior_alloc.clone(), now)
                .unwrap();

        // ── Strict-prefix assertion ───────────────────────────────────────────
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 2,
            "post must be pre.len()+2 (two decisions appended)"
        );

        // Pre-prefix is byte-for-byte identical.
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows must be unchanged (strict prefix)"
        );

        // First new row: VoidDecisionEvent.
        let void_row = &post[pre.len()];
        let void_seq = void_row
            .decision_seq
            .expect("void row must have decision_seq");
        assert_eq!(
            void_seq,
            (pre_max_seq + 1) as i64,
            "void seq must be pre_max+1"
        );
        let expected_void_id = EventId::Decision {
            seq: void_seq as u64,
        };
        assert_eq!(void_id, expected_void_id, "returned void_id must match row");
        let void_payload: EventPayload =
            serde_json::from_str(&void_row.payload_json).expect("void row must deserialise");
        match &void_payload {
            EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id }) => {
                assert_eq!(
                    *target_event_id, prior_id,
                    "VoidDecisionEvent must target prior_id"
                );
            }
            other => panic!("expected VoidDecisionEvent, got {other:?}"),
        }

        // Second new row: SafeHarborAllocation with timely_allocation_attested=true.
        let attest_row = &post[pre.len() + 1];
        let attest_seq = attest_row
            .decision_seq
            .expect("attest row must have decision_seq");
        assert_eq!(
            attest_seq,
            void_seq + 1,
            "attest seq must follow void seq immediately"
        );
        let expected_attest_id = EventId::Decision {
            seq: attest_seq as u64,
        };
        assert_eq!(
            attest_id, expected_attest_id,
            "returned attest_id must match row"
        );
        let attest_payload: EventPayload =
            serde_json::from_str(&attest_row.payload_json).expect("attest row must deserialise");
        match &attest_payload {
            EventPayload::SafeHarborAllocation(a) => {
                assert!(
                    a.timely_allocation_attested,
                    "re-appended allocation must have timely_allocation_attested=true"
                );
                assert_eq!(
                    a.as_of_date, prior_alloc.as_of_date,
                    "all other fields must match prior_alloc"
                );
                assert_eq!(a.method, prior_alloc.method);
                assert_eq!(a.pre2025_method, prior_alloc.pre2025_method);
                assert_eq!(a.lots, prior_alloc.lots);
            }
            other => panic!("expected SafeHarborAllocation, got {other:?}"),
        }

        // Drop + reopen: same strict-prefix holds on disk.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post_disk = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post_disk, post,
            "on-disk image must equal in-memory post after save"
        );
    }

    // ── Resolve-conflict helper: seed an ImportConflict on an Acquire (chunk 4b, D3) ──
    //
    // Import Acquire{usd_cost:30000} at target X, then re-import Acquire{usd_cost:50000} at the SAME
    // (source, source_ref) → append_import_batch emits ONE ImportConflict whose new_payload carries
    // usd_cost:50000. Returns (target, conflict_event). The unresolved conflict fires an
    // ImportConflict blocker; the baseline lot keeps usd_basis 30000 until the conflict is resolved.
    #[cfg(test)]
    fn seed_acquire_conflict(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> (btctax_core::EventId, btctax_core::EventId) {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_import_batch, load_all};
        use btctax_core::{EventId, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let target = EventId::import(Source::River, SourceRef::new("rc-acq"));
        let wallet = Some(WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        });
        let ts = OffsetDateTime::from_unix_timestamp(1_740_000_000).unwrap();

        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let v1 = vec![LedgerEvent {
                id: target.clone(),
                utc_timestamp: ts,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 100_000,
                    usd_cost: dec!(30000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            }];
            append_import_batch(session.conn(), &v1).unwrap();
            session.save().unwrap();

            // Re-import the SAME (source, source_ref) with different content → ImportConflict.
            let v2 = vec![LedgerEvent {
                id: target.clone(),
                utc_timestamp: ts,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 100_000,
                    usd_cost: dec!(50000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            }];
            append_import_batch(session.conn(), &v2).unwrap();
            session.save().unwrap();

            let events = load_all(session.conn()).unwrap();
            let conflict_event = events
                .iter()
                .find(|e| matches!(e.payload, EventPayload::ImportConflict(_)))
                .expect("ImportConflict must exist after re-import with changed content")
                .id
                .clone();
            (target, conflict_event)
        }
    }

    // ── KAT-P2-RC-A — resolve-conflict ACCEPT: strict prefix + target adopts new_payload ──
    //
    // persist_resolve_conflict(Accept) appends EXACTLY one SupersedeImport{conflict_event}; the
    // ImportConflict blocker clears AND the target lot's basis becomes the NEW payload's 50000.
    #[test]
    fn kat_p2_rc_accept_supersede_adopts_new_payload() {
        use crate::edit::form::ResolveKind;
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::{BlockerKind, EventId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-rc-a-pass";
        let (target, conflict_event) = seed_acquire_conflict(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();

        // Baseline: unresolved conflict → blocker present; lot basis is the ORIGINAL 30000.
        let (_e0, s0, _c0) = session.load_events_and_project().unwrap();
        assert!(
            s0.blockers
                .iter()
                .any(|b| b.kind == BlockerKind::ImportConflict),
            "baseline must carry an ImportConflict blocker"
        );
        assert_eq!(
            s0.lots
                .iter()
                .find(|l| l.original_sat == 100_000)
                .unwrap()
                .usd_basis,
            dec!(30000),
            "baseline lot basis must be the ORIGINAL 30000"
        );

        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);

        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let id = persist_resolve_conflict(
            &mut session,
            conflict_event.clone(),
            ResolveKind::Accept,
            now,
        )
        .unwrap();

        // Strict prefix: exactly one decision appended at the tail.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "post must be pre.len()+1");
        assert_eq!(&post[..pre.len()], pre.as_slice(), "strict prefix");
        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(tail_seq, (pre_max_seq + 1) as i64, "tail seq = pre_max+1");
        assert_eq!(
            id,
            EventId::Decision {
                seq: tail_seq as u64
            }
        );
        let stored: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail must deserialise");
        match &stored {
            EventPayload::SupersedeImport(s) => {
                assert_eq!(s.conflict_event, conflict_event, "targets the conflict")
            }
            other => panic!("expected SupersedeImport, got {other:?}"),
        }

        // Re-project: blocker cleared AND the lot adopts the NEW payload's 50000 basis.
        let (_e1, s1, _c1) = session.load_events_and_project().unwrap();
        assert!(
            s1.blockers
                .iter()
                .all(|b| b.kind != BlockerKind::ImportConflict),
            "ImportConflict blocker must clear after accept"
        );
        assert_eq!(
            s1.lots
                .iter()
                .find(|l| l.original_sat == 100_000)
                .unwrap()
                .usd_basis,
            dec!(50000),
            "accept must adopt the NEW payload (basis 50000)"
        );
        let _ = target;
    }

    // ── KAT-P2-RC-R — resolve-conflict REJECT: original stands, blocker clears ──
    #[test]
    fn kat_p2_rc_reject_keeps_original() {
        use crate::edit::form::ResolveKind;
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::BlockerKind;
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-rc-r-pass";
        let (_target, conflict_event) = seed_acquire_conflict(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();

        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let id = persist_resolve_conflict(
            &mut session,
            conflict_event.clone(),
            ResolveKind::Reject,
            now,
        )
        .unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1);
        assert_eq!(&post[..pre.len()], pre.as_slice(), "strict prefix");
        let tail = &post[pre.len()];
        let stored: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail must deserialise");
        match &stored {
            EventPayload::RejectImport(r) => assert_eq!(r.conflict_event, conflict_event),
            other => panic!("expected RejectImport, got {other:?}"),
        }
        let _ = id;

        // Re-project: blocker cleared AND the lot keeps the ORIGINAL 30000 basis.
        let (_e1, s1, _c1) = session.load_events_and_project().unwrap();
        assert!(
            s1.blockers
                .iter()
                .all(|b| b.kind != BlockerKind::ImportConflict),
            "ImportConflict blocker must clear after reject"
        );
        assert_eq!(
            s1.lots
                .iter()
                .find(|l| l.original_sat == 100_000)
                .unwrap()
                .usd_basis,
            dec!(30000),
            "reject must keep the ORIGINAL payload (basis 30000)"
        );
    }

    // ── KAT-RC-NONREVOCABLE — SupersedeImport/RejectImport are NOT revocable ──
    //
    // Pins that a resolve-conflict decision never appears in the void ('v') list: the void-list
    // pre-filter offers only `is_revocable_payload` decisions, and these two are excluded (a later
    // void fires DecisionConflict, resolve.rs:312-313). This is the persist-level anchor for the
    // spec's non-revocability requirement.
    #[test]
    fn kat_rc_supersede_reject_are_non_revocable() {
        use btctax_core::event::{RejectImport, SupersedeImport};
        use btctax_core::{EventId, EventPayload};
        let ce = EventId::decision(1);
        assert!(
            !crate::edit::form::is_revocable_payload(&EventPayload::SupersedeImport(
                SupersedeImport {
                    conflict_event: ce.clone()
                }
            )),
            "SupersedeImport must NOT be revocable"
        );
        assert!(
            !crate::edit::form::is_revocable_payload(&EventPayload::RejectImport(RejectImport {
                conflict_event: ce
            })),
            "RejectImport must NOT be revocable"
        );
    }

    // ── save-rollback: persist_resolve_conflict reverts on a failed save ──
    #[cfg(unix)]
    #[test]
    fn kat_persist_resolve_conflict_rollback_on_failed_save() {
        use crate::edit::form::ResolveKind;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::BlockerKind;
        use btctax_store::Passphrase;
        use std::os::unix::fs::PermissionsExt;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-rc-rollback-pass";
        let (_target, conflict_event) = seed_acquire_conflict(&vault, &key, pp_str);

        // Root-skip guard (chmod is a no-op as root).
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("rc-rollback KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();

        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result =
            persist_resolve_conflict(&mut session, conflict_event, ResolveKind::Accept, now);
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must return RolledBack; got: {result:?}"
        );

        // Rollback: NO decision appended, and the conflict stays unresolved.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len(), "rollback must revert the append");
        let (_e, s, _c) = session.load_events_and_project().unwrap();
        assert!(
            s.blockers
                .iter()
                .any(|b| b.kind == BlockerKind::ImportConflict),
            "conflict must remain unresolved after a rolled-back save"
        );
    }

    // ── Optimize-accept persist helpers ──────────────────────────────────────

    /// Seed a vault with ONE MethodElection decision (so pre_max_seq == 1). Returns the disposal +
    /// lot ids to synthesize a proposed LotSelection (the persist fn does not require the disposal to
    /// exist as an imported ledger event — mirrors kat_p2f).
    #[cfg(test)]
    fn seed_optimize_accept_base(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> (btctax_core::EventId, btctax_core::LotPick) {
        use btctax_core::event::{EventPayload, MethodElection};
        use btctax_core::identity::{LotId, Source, SourceRef};
        use btctax_core::persistence::append_decision;
        use btctax_core::{EventId, LotPick};
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
        append_decision(
            session.conn(),
            EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            }),
            t0,
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        session.save().unwrap();

        let disposal = EventId::import(Source::River, SourceRef::new("oa-disposal"));
        let lot_origin = EventId::import(Source::River, SourceRef::new("oa-lot"));
        let pick = LotPick {
            lot: LotId {
                origin_event_id: lot_origin,
                split_sequence: 0,
            },
            sat: 100_000,
        };
        (disposal, pick)
    }

    // ── KAT-P2-OA-ATTEST — attested optimize-accept: LotSelection + attest row; void clears it ──
    //
    // E2E attested: post.len()+1 (LotSelection) AND optimize_attest::get == Some(text). Then the
    // shipped persist_void round-trip → optimize_attest::get == None (INVERSE of the set here).
    #[test]
    fn kat_p2_oa_attested_appends_lotselection_and_attest_row_then_void_clears() {
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-oa-attest-pass";
        let (disposal, pick) = seed_optimize_accept_base(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);

        let made = date!(2026 - 07 - 03);
        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();
        let id = persist_optimize_accept(
            &mut session,
            disposal.clone(),
            vec![pick.clone()],
            Some("contemporaneous-id-statement".to_string()),
            made,
            now,
        )
        .unwrap();

        // Strict prefix: exactly one LotSelection appended at the tail.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "post = pre.len()+1");
        assert_eq!(&post[..pre.len()], pre.as_slice(), "strict prefix");
        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(tail_seq, (pre_max_seq + 1) as i64);
        assert_eq!(
            id,
            EventId::Decision {
                seq: tail_seq as u64
            }
        );
        let stored: EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail must deserialise");
        match &stored {
            EventPayload::LotSelection(ls) => {
                assert_eq!(ls.disposal_event, disposal, "targets the disposal");
                assert_eq!(ls.lots, vec![pick.clone()], "picks round-trip");
            }
            other => panic!("expected LotSelection, got {other:?}"),
        }

        // Attest row is present and equals the attested text.
        let att = btctax_cli::optimize_attest::get(session.conn(), &disposal).unwrap();
        assert_eq!(
            att.as_deref(),
            Some("contemporaneous-id-statement"),
            "attest row must be co-persisted"
        );

        // Void round-trip: voiding the LotSelection clears the attest row (shipped persist_void).
        let vnow = OffsetDateTime::from_unix_timestamp(1_752_001_000).unwrap();
        persist_void(&mut session, id, vnow).unwrap();
        let att_after = btctax_cli::optimize_attest::get(session.conn(), &disposal).unwrap();
        assert_eq!(att_after, None, "void must clear the attest row");
    }

    // ── KAT-P2-OA-CONTEMP — contemporaneous optimize-accept: LotSelection, NO attest row ──
    #[test]
    fn kat_p2_oa_contemporaneous_appends_lotselection_no_attest_row() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-oa-contemp-pass";
        let (disposal, pick) = seed_optimize_accept_base(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();

        let made = date!(2026 - 07 - 03);
        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();
        persist_optimize_accept(&mut session, disposal.clone(), vec![pick], None, made, now)
            .unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "one LotSelection appended");
        // No attestation row for the contemporaneous path.
        let att = btctax_cli::optimize_attest::get(session.conn(), &disposal).unwrap();
        assert_eq!(att, None, "contemporaneous path writes NO attest row");
    }

    // ── save-rollback: persist_optimize_accept reverts BOTH the append AND the attest set ──
    #[cfg(unix)]
    #[test]
    fn kat_persist_optimize_accept_rollback_on_failed_save() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use std::os::unix::fs::PermissionsExt;
        use time::{macros::date, OffsetDateTime};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-oa-rollback-pass";
        let (disposal, pick) = seed_optimize_accept_base(&vault, &key, pp_str);

        // Root-skip guard.
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("oa-rollback KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let made = date!(2026 - 07 - 03);
        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();

        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = persist_optimize_accept(
            &mut session,
            disposal.clone(),
            vec![pick],
            Some("attest".to_string()),
            made,
            now,
        );
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must return RolledBack; got: {result:?}"
        );
        // Rollback reverts BOTH the append AND the attest set (whole-DB restore).
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len(), "append reverted");
        let att = btctax_cli::optimize_attest::get(session.conn(), &disposal).unwrap();
        assert_eq!(att, None, "attest set reverted by the whole-DB restore");
    }
}
