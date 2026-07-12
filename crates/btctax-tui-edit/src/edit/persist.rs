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
    // D-4 guard (SPEC §4.12; mirror `cmd/tax.rs` `set_profile`): when full-return `ReturnInputs` exist for
    // the year, a raw tax-profile is IGNORED (`resolve_profile` gives ReturnInputs precedence). Refuse
    // rather than silently store an unused/escape-hatch-clobbering value (review N1). The editor has no
    // `--force`; the user must `income clear --year N` (or the CLI `tax-profile set --force`) first.
    if btctax_cli::return_inputs::exists(session.conn(), year)? {
        return Err(PersistError::NoChange(btctax_cli::CliError::Usage(format!(
            "tax year {year} has full-return inputs (`income import`); a raw tax-profile would be ignored \
             (full-return inputs take precedence). Run `income clear --year {year}`, or the CLI \
             `tax-profile set --force`, first."
        ))));
    }
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

/// Append a per-account `MethodElection` decision (§A.5(a)) and atomically save the vault.
///
/// `payload` is the **fully-built** `EventPayload::MethodElection(…)` — the flow constructs it with a
/// `Some(WalletId::Exchange{..})` scope (in the PAYLOAD, [R0-M1]) and `effective_from` defaulted to the
/// made-date. Setting the method IS the attestation (a forward election the user can update going
/// forward — NOT the irrevocable typed-word safe-harbor flow). `now` is INJECTED at Enter-press for
/// test determinism. Strict-append: `append_decision` assigns `decision_seq = MAX(existing) + 1`; a
/// SINGLE `save_or_rollback` reverts the whole append on a save failure (mirrors `persist_set_fmv`).
pub fn persist_method_election(
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

    // Detect the target's side-effect keys BEFORE appending the void:
    //  - a `LotSelection` target → clear its optimizer attestation (`optimize_attest`);
    //  - [R0-I1] a `ReclassifyOutflow` target → clear its `bulk_estimated` flag (else a stale `[est]`
    //    survives a void + single-`o` re-reclassify that carries a REAL price).
    let events = load_all(session.conn())?;
    let target = events.iter().find(|e| e.id == target_event_id);
    let disposal_to_clear: Option<btctax_core::EventId> = target.and_then(|e| match &e.payload {
        EventPayload::LotSelection(ls) => Some(ls.disposal_event.clone()),
        _ => None,
    });
    let reclass_out_to_clear: Option<btctax_core::EventId> =
        target.and_then(|e| match &e.payload {
            EventPayload::ReclassifyOutflow(ro) => Some(ro.transfer_out_event.clone()),
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
    // [R0-I1] Idempotent — clearing an absent row is Ok, so voiding a single-`o` reclassify that was
    // never flagged is a harmless no-op (mirrors the LotSelection arm exactly).
    if let Some(out_event) = reclass_out_to_clear {
        if let Err(e) = btctax_cli::bulk_estimated::clear(session.conn(), &out_event) {
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

/// Append a `SelfTransferPassthrough` decision (the DROP primitive) and atomically save the vault
/// (self-transfer-passthrough C3). Both legs of the confirmed passthrough project to `Op::Skip`
/// (non-taxable, no lot).
///
/// # Single-append shape (NO bespoke latch)
/// Identical `snapshot → append_decision → save_or_rollback` shape as `persist_link_transfer`: exactly
/// ONE fallible mutation after the snapshot, so a failed save reverts cleanly and a retry re-appends
/// with the SAME `decision_seq` (no residue, no duplicate). A genuine DUPLICATE (two SUCCESSFUL
/// passthroughs sharing a leg) still fires `DecisionConflict` in resolve.rs; the failed-save path no
/// longer creates one. Voidable via `persist_void` (re-exposes both legs).
pub fn persist_self_transfer_passthrough(
    session: &mut btctax_cli::Session,
    payload: btctax_core::event::EventPayload, // must be EventPayload::SelfTransferPassthrough
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

/// The shared bulk-persist loop every batch flow delegates to (bulk-resolve-conflict Task 1): refuse
/// empty, snapshot, append EACH payload, and on ANY append error revert the WHOLE batch, then ONE save.
///
/// # Empty guard [R0-M2]
/// An empty `payloads` (user unchecked everything) is REFUSED BEFORE any snapshot/append — a
/// `NoChange(CliError::Usage(empty_label.into()))`, so the caller never appends zero + saves. The
/// `empty_label` is CALLER-SUPPLIED so each front-end preserves its EXACT "nothing selected" string
/// (the re-point of the two shipped bulk flows is therefore truly byte-for-byte / zero-behavior).
///
/// # Mid-batch append rollback (the load-bearing distinction from the CLI path)
/// `append_decision` commits per-call to the in-memory conn. If the append at row k>1 fails, appends
/// 1..k-1 are ALREADY LIVE residue; a bare `?` would return `NoChange` (contract: "vault unchanged")
/// while phantom decisions sit in the DB. So on ANY append error the WHOLE batch is reverted via
/// `rollback(session, &pre, e.into())` (whole-DB restore to the pre-batch snapshot) — NOT `?`. A
/// `save` failure likewise reverts the whole batch via `save_or_rollback`. Never a partial apply.
pub fn persist_bulk_decisions(
    session: &mut btctax_cli::Session,
    payloads: Vec<btctax_core::EventPayload>,
    now: time::OffsetDateTime,
    empty_label: &str,
) -> Result<usize, PersistError> {
    if payloads.is_empty() {
        return Err(PersistError::NoChange(btctax_cli::CliError::Usage(
            empty_label.into(),
        )));
    }
    let n = payloads.len();
    let pre = session.snapshot()?;
    for payload in payloads {
        // Do NOT use `?` here: a mid-batch failure at row k>1 leaves appends 1..k-1 as live residue
        // AND would leak a bare NoChange over phantom decisions. Revert the WHOLE batch.
        if let Err(e) = btctax_core::persistence::append_decision(
            session.conn(),
            payload,
            now,
            time::UtcOffset::UTC,
            None,
        ) {
            return Err(rollback(session, &pre, e.into()));
        }
    }
    save_or_rollback(session, pre)?; // ONE save; on failure the whole batch reverts
    Ok(n)
}

/// Append ONE `TransferLink { out_event, Wallet(dest) }` per `out_event`, then a SINGLE
/// `save_or_rollback` (bulk-link-transfer D3). All-or-nothing. Builds its `Vec<EventPayload>` and
/// delegates to the shared `persist_bulk_decisions` (Task 1) — the empty guard + mid-batch rollback +
/// single save all live there. Passes its EXACT empty-label string so the re-point is zero-behavior.
pub fn persist_bulk_link_transfer(
    session: &mut btctax_cli::Session,
    out_events: Vec<btctax_core::EventId>,
    dest: btctax_core::WalletId,
    now: time::OffsetDateTime,
) -> Result<usize, PersistError> {
    let payloads = out_events
        .into_iter()
        .map(|out_event| {
            btctax_core::EventPayload::TransferLink(btctax_core::event::TransferLink {
                out_event,
                in_event_or_wallet: btctax_core::TransferTarget::Wallet(dest.clone()),
            })
        })
        .collect();
    persist_bulk_decisions(session, payloads, now, "bulk link: nothing selected")
}

/// Append ONE `ClassifyInbound { transfer_in_event, SelfTransferMine{None,None} }` per `in_event`,
/// then a SINGLE `save_or_rollback` (bulk-classify-inbound-self-transfer D3). All-or-nothing; each
/// inbound is given the $0 conservative basis / receipt-date HP (non-taxable "my own coins"). Builds
/// its `Vec<EventPayload>` and delegates to the shared `persist_bulk_decisions` (Task 1) — the empty
/// guard + mid-batch rollback + single save all live there. Passes its EXACT empty-label string so the
/// re-point is zero-behavior.
pub fn persist_bulk_self_transfer_in(
    session: &mut btctax_cli::Session,
    in_events: Vec<btctax_core::EventId>,
    now: time::OffsetDateTime,
) -> Result<usize, PersistError> {
    let payloads = in_events
        .into_iter()
        .map(|in_event| {
            btctax_core::EventPayload::ClassifyInbound(btctax_core::event::ClassifyInbound {
                transfer_in_event: in_event,
                as_: btctax_core::InboundClass::SelfTransferMine {
                    basis: None,
                    acquired_at: None,
                },
            })
        })
        .collect();
    persist_bulk_decisions(
        session,
        payloads,
        now,
        "bulk classify-inbound-self-transfer: nothing selected",
    )
}

/// Append the pre-built `ClassifyInbound{Income{kind, Some(fmv), business}}` payloads (one per included
/// row; the per-row auto-FMV is resolved by the flow from the plan), then a SINGLE `save_or_rollback`
/// (bulk-classify-inbound-income, Cycle 4). All-or-nothing. UNLIKE the STI/link wrappers, the payloads
/// are built by the CALLER (each row carries a DISTINCT `fmv`, so there is no uniform per-id payload to
/// synthesize here) and passed straight through — this is the thin classify-income wrapper the R0-I1
/// split calls for: the TUI CAN reach `persist_bulk_decisions` (the empty guard + mid-batch rollback +
/// single save all live there); the CLI, which cannot, uses its OWN append-loop
/// (`apply_bulk_classify_inbound_income`). Passes the classify-income empty-label so the empty guard's
/// message is exact. [#a] `Income{fmv:None}` is structurally unrepresentable from the bulk path: the
/// plan excluded the missing-price rows, so every `payload`'s `fmv` is `Some(_)`.
pub fn persist_bulk_classify_income(
    session: &mut btctax_cli::Session,
    payloads: Vec<btctax_core::EventPayload>,
    now: time::OffsetDateTime,
) -> Result<usize, PersistError> {
    persist_bulk_decisions(
        session,
        payloads,
        now,
        "bulk classify-inbound-income: nothing selected",
    )
}

/// Append ONE `SupersedeImport` (accept) or `RejectImport` (reject) per `conflict_event`, then a SINGLE
/// `save_or_rollback` (bulk-resolve-conflict D3). All-or-nothing; a thin wrapper that builds the
/// per-row payload from the batch-wide `kind` and delegates to `persist_bulk_decisions` (Task 1) — the
/// empty guard + mid-batch rollback + single save all live there.
///
/// # Non-revocable [G2]
/// `SupersedeImport`/`RejectImport` are EXCLUDED from `is_revocable_payload`, so a wrong accept/reject
/// CANNOT be voided in-editor (a later void fires `DecisionConflict`). The confirm modal carries the
/// Tier-B non-revocable warning (NOT a typed-word gate, which is reserved for the §7.4 attest).
pub fn persist_bulk_resolve_conflict(
    session: &mut btctax_cli::Session,
    conflict_events: Vec<btctax_core::EventId>,
    kind: crate::edit::form::ResolveKind,
    now: time::OffsetDateTime,
) -> Result<usize, PersistError> {
    use btctax_core::event::{RejectImport, SupersedeImport};
    use btctax_core::EventPayload;
    let payloads = conflict_events
        .into_iter()
        .map(|conflict_event| match kind {
            crate::edit::form::ResolveKind::Accept => {
                EventPayload::SupersedeImport(SupersedeImport { conflict_event })
            }
            crate::edit::form::ResolveKind::Reject => {
                EventPayload::RejectImport(RejectImport { conflict_event })
            }
        })
        .collect();
    persist_bulk_decisions(
        session,
        payloads,
        now,
        "bulk resolve-conflict: nothing selected",
    )
}

/// Sweep-void N revocable decisions in ONE atomic batch (bulk-void D3) — the BESPOKE bulk analog of the
/// single `persist_void` (persist.rs:248-300) across N. Appends one `VoidDecisionEvent` per target AND,
/// for each `LotSelection` target, clears its optimizer attestation (`optimize_attest::clear`), ALL
/// inside ONE atomic envelope; a mid-batch append OR clear failure reverts the WHOLE batch (both void
/// rows AND side-table clears — whole-DB restore covers the side-table for free, per persist_void [M1]).
///
/// # Bespoke — NOT a hook on `persist_bulk_decisions` [R0-adjudicated: blast-radius isolation]
/// LOCKSTEP with `persist_bulk_decisions` (the shared bulk safety skeleton): the empty guard + the
/// mid-batch `rollback(session, &pre, e)`-not-`?` discipline + the single trailing `save_or_rollback`
/// are IDENTICAL — a future edit to that shared invariant MUST be echoed here. This fn stays separate
/// only because it carries the dangerous per-`LotSelection` side-effect (`optimize_attest::clear`),
/// which is safer isolated than threaded as a closure through the 3-flow shared helper.
///
/// `targets` carry `disposal_to_clear` PRECOMPUTED ONCE by the caller from the snapshot (a `LotSelection`
/// target → `Some(ls.disposal_event)`), so this fn never re-loads the log per row. [R0-N1] if two voided
/// `LotSelection`s target the SAME disposal, `clear` runs twice — harmless (a pure idempotent DELETE).
pub fn persist_bulk_void(
    session: &mut btctax_cli::Session,
    targets: Vec<crate::edit::form::VoidTarget>,
    now: time::OffsetDateTime,
    empty_label: &str,
) -> Result<usize, PersistError> {
    use btctax_core::{event::VoidDecisionEvent, persistence::append_decision, EventPayload};

    // Empty guard [mirrors persist_bulk_decisions]: refuse BEFORE any snapshot/append.
    if targets.is_empty() {
        return Err(PersistError::NoChange(btctax_cli::CliError::Usage(
            empty_label.into(),
        )));
    }
    let n = targets.len();
    let pre = session.snapshot()?;
    for t in targets {
        // Do NOT use `?`: a mid-batch append at row k>1 that fails leaves appends/clears 1..k-1 as live
        // residue AND would leak a bare NoChange over phantom voids. Revert the WHOLE batch.
        if let Err(e) = append_decision(
            session.conn(),
            EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                target_event_id: t.target_event_id,
            }),
            now,
            time::UtcOffset::UTC,
            None,
        ) {
            return Err(rollback(session, &pre, e.into()));
        }
        // The per-LotSelection clear is INSIDE the envelope — a failure AFTER the committed append must
        // roll back the WHOLE batch (symmetric with persist_void's clear-then-rollback arm), else the
        // committed void rows become residue that piggy-backs a later save.
        if let Some(disposal) = t.disposal_to_clear {
            if let Err(e) = btctax_cli::optimize_attest::clear(session.conn(), &disposal) {
                return Err(rollback(session, &pre, e));
            }
        }
        // [R0-I1] The per-ReclassifyOutflow `bulk_estimated` clear — same in-envelope, guarded arm as
        // above (mirrors persist_void's ReclassifyOutflow clear). Idempotent (an absent row is Ok).
        if let Some(out_event) = t.reclass_out_to_clear {
            if let Err(e) = btctax_cli::bulk_estimated::clear(session.conn(), &out_event) {
                return Err(rollback(session, &pre, e));
            }
        }
    }
    save_or_rollback(session, pre)?; // ONE save; on failure the whole batch reverts
    Ok(n)
}

/// Reclassify N pending outflows as `Dispose{kind}` with an auto-FMV as ESTIMATED proceeds, AND flag
/// each in the `bulk_estimated` side-table, in ONE atomic batch (bulk-reclassify-outflow D3, Cycle 5 —
/// the LAST) — the BESPOKE bulk analog of the single `persist_reclassify_outflow` across N, with the
/// per-row side-table `mark` co-persisted.
///
/// # Bespoke — NOT a hook on `persist_bulk_decisions` [mirror `persist_bulk_void`]
/// LOCKSTEP with `persist_bulk_decisions` (the shared bulk safety skeleton): the empty guard + the
/// mid-batch `rollback(session, &pre, e)`-not-`?` discipline + the single trailing `save_or_rollback`
/// are IDENTICAL — a future edit to that shared invariant MUST be echoed here. This fn stays separate
/// only because it carries the per-row side-effect (`bulk_estimated::mark`) that must land in the SAME
/// atomic envelope as the `ReclassifyOutflow` append (a mid-batch failure must not leave a decision
/// without its flag, or vice versa). The whole-DB restore covers the side-table for free on any failure
/// (documented invariant, same as `persist_optimize_accept`/`optimize_attestation`).
///
/// `rows` carry `(out_event, resolved fmv)` — the fmv is the plan's per-row auto-FMV (the modal captured
/// the CHECKED rows), so a missing-price row is never here [#a: the plan excluded them; `fmv: Usd`].
/// `kind` is UNIFORM; `fee_usd: None` always (the on-chain `fee_sat` still flows); `donee: None`.
pub fn persist_bulk_reclassify_outflow(
    session: &mut btctax_cli::Session,
    rows: Vec<(btctax_core::EventId, btctax_core::Usd)>,
    kind: btctax_core::DisposeKind,
    now: time::OffsetDateTime,
    empty_label: &str,
) -> Result<usize, PersistError> {
    use btctax_core::event::ReclassifyOutflow;
    use btctax_core::persistence::append_decision;
    use btctax_core::{EventPayload, OutflowClass};

    // Empty guard [mirrors persist_bulk_decisions]: refuse BEFORE any snapshot/append.
    if rows.is_empty() {
        return Err(PersistError::NoChange(btctax_cli::CliError::Usage(
            empty_label.into(),
        )));
    }
    let n = rows.len();
    let pre = session.snapshot()?;
    // Provenance stamp (the batch made-date) recorded on each side-table row.
    let marked_at = btctax_core::conventions::tax_date(now, time::UtcOffset::UTC).to_string();
    for (out_event, fmv) in rows {
        // Do NOT use `?`: a mid-batch append at row k>1 that fails leaves appends/marks 1..k-1 as live
        // residue AND would leak a bare NoChange over phantom decisions. Revert the WHOLE batch.
        if let Err(e) = append_decision(
            session.conn(),
            EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: out_event.clone(),
                as_: OutflowClass::Dispose { kind },
                principal_proceeds_or_fmv: fmv,
                fee_usd: None,
                donee: None,
            }),
            now,
            time::UtcOffset::UTC,
            None,
        ) {
            return Err(rollback(session, &pre, e.into()));
        }
        // The side-table mark is INSIDE the envelope — a failure AFTER the committed append must roll
        // back the WHOLE batch (symmetric with persist_bulk_void's clear-then-rollback arm), else the
        // committed decision rows become residue that piggy-backs a later save.
        if let Err(e) = btctax_cli::bulk_estimated::mark(session.conn(), &out_event, &marked_at) {
            return Err(rollback(session, &pre, e));
        }
    }
    save_or_rollback(session, pre)?; // ONE save; on failure the whole batch reverts
    Ok(n)
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

/// Append a `SafeHarborAllocation` decision event (chunk 5, D5) and atomically save the vault.
///
/// `lots`/`method`/`pre2025_method` come from the allocate flow (`lots` + `pre2025_method` computed
/// ONCE at open via `Session::safe_harbor_residue`; `method` is the user's toggle). `timely_allocation_
/// attested` is hard-coded `false` — creation yields a REVOCABLE allocation (voidable while inert). The
/// `as_of_date` is the fixed `TRANSITION_DATE` (2025-01-01) universal snapshot. `now` is INJECTED at
/// Enter-press for test determinism.
///
/// # Standard single-append template (NOT the attest special-case)
/// A single `append_decision` rolls back CLEANLY on a failed save — no latch, no side-table. On
/// `Err(save)`, `save_or_rollback` reverts the in-memory DB (retry re-appends with the SAME
/// `decision_seq`); errors route through `EditorApp::on_persist_error`. Contrast
/// `persist_safe_harbor_attest`, whose two-decision batch is unrecoverable and needs the
/// `attest_save_failed` latch.
///
/// # Called only from the safe-harbor-allocate confirmation modal
/// Same procedural guarantee as `persist_tax_profile` (see doc there).
pub fn persist_safe_harbor_allocate(
    session: &mut btctax_cli::Session,
    lots: Vec<btctax_core::AllocLot>,
    method: btctax_core::AllocMethod,
    pre2025_method: btctax_core::LotMethod,
    now: time::OffsetDateTime,
) -> Result<btctax_core::EventId, PersistError> {
    use btctax_core::event::SafeHarborAllocation;
    use btctax_core::EventPayload;

    let pre = session.snapshot()?;
    let payload = EventPayload::SafeHarborAllocation(SafeHarborAllocation {
        lots,
        as_of_date: btctax_core::conventions::TRANSITION_DATE,
        method,
        timely_allocation_attested: false,
        pre2025_method,
    });
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
                wallet: None,
            });
            let p2 = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2025 - 01 - 01),
                method: LotMethod::Hifo,
                wallet: None,
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

    /// [P2-N1] D-4 guard in the editor save path: when full-return `ReturnInputs` exist for the year, the
    /// editor REFUSES a raw tax-profile write (would be ignored + clobber the escape hatch) — matching the
    /// CLI `tax-profile set`. Nothing is written; a different year (no ReturnInputs) still persists.
    #[test]
    fn persist_tax_profile_refuses_when_return_inputs_exist_d4() {
        use btctax_store::Passphrase;
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp = "d4-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp.into()), &key).unwrap();

        // Seed full-return inputs for 2025 via the CLI import path.
        let toml = dir.path().join("inputs.toml");
        std::fs::write(&toml, "filing_status = \"Single\"\n").unwrap();
        btctax_cli::cmd::tax::import_return_inputs(
            &vault,
            &Passphrase::new(pp.into()),
            2025,
            &toml,
        )
        .unwrap();

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp.into())).unwrap();
        // A raw profile for the SAME year is refused (D-4) — nothing stored.
        let err = persist_tax_profile(&mut session, 2025, &fixture_profile()).unwrap_err();
        assert!(matches!(
            err,
            PersistError::NoChange(btctax_cli::CliError::Usage(_))
        ));
        assert!(
            session.tax_profile(2025).unwrap().is_none(),
            "no raw profile may be stored for a ReturnInputs year"
        );
        // A DIFFERENT year (no ReturnInputs) persists normally.
        persist_tax_profile(&mut session, 2024, &fixture_profile()).unwrap();
        assert!(session.tax_profile(2024).unwrap().is_some());
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
                wallet: None,
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
                wallet: None,
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
                wallet: None,
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

    // ── KAT-P2-STP — append-only strict prefix test (self-transfer-passthrough append form) ──
    //
    // Invariant: persist_self_transfer_passthrough appends EXACTLY one decision event to the tail.
    // Strict-prefix formula: post == pre ++ [new_event]; tail.decision_seq == pre_max+1.

    #[test]
    fn kat_p2_stp_append_only_strict_prefix_self_transfer_passthrough() {
        use btctax_core::event::{EventPayload, SelfTransferPassthrough};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{EventId, WalletId};
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p2stp-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed 1 import TransferIn + 1 import TransferOut + 1 decision → non-trivial pre-state.
        let in_id: EventId = EventId::import(Source::River, SourceRef::new("in-p2stp"));
        let out_id: EventId = EventId::import(Source::River, SourceRef::new("out-p2stp"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let wallet = Some(WalletId::Exchange {
                provider: "River".into(),
                account: "main".into(),
            });
            let batch = vec![
                btctax_core::event::LedgerEvent {
                    id: in_id.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::TransferIn(btctax_core::event::TransferIn {
                        sat: 100_000,
                        src_addr: None,
                        txid: None,
                    }),
                },
                btctax_core::event::LedgerEvent {
                    id: out_id.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_500).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet,
                    payload: EventPayload::TransferOut(btctax_core::event::TransferOut {
                        sat: 100_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
                wallet: None,
            });
            append_decision(session.conn(), p, now, UtcOffset::UTC, None).unwrap();
            session.save().unwrap();
        };

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        assert_eq!(pre.len(), 3, "pre must have exactly 3 events");
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        let payload = EventPayload::SelfTransferPassthrough(SelfTransferPassthrough {
            in_event: in_id.clone(),
            out_event: out_id.clone(),
        });
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let returned_id =
            persist_self_transfer_passthrough(&mut session, payload.clone(), now).unwrap();

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
                wallet: None,
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
                wallet: None,
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
                wallet: None,
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
                wallet: None,
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
                wallet: None,
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

    // ── persist_bulk_void: empty targets → NoChange, no snapshot/append (bulk-void D3) ──
    #[test]
    fn kat_bulk_void_empty_refuses() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-void-empty-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();

        let result = persist_bulk_void(&mut session, vec![], now, "bulk void: nothing selected");
        match result {
            Err(PersistError::NoChange(btctax_cli::CliError::Usage(label))) => {
                assert_eq!(
                    label, "bulk void: nothing selected",
                    "empty guard must carry the caller-supplied empty label"
                );
            }
            other => panic!("empty targets must be NoChange(Usage(label)); got {other:?}"),
        }

        // No snapshot/append: the log is byte-for-byte unchanged.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post, pre, "empty refuse must append nothing");
    }

    // ── persist_bulk_void: a failing save reverts the WHOLE batch (bulk-void D3, safety) ──
    //
    // Two LotSelection targets + two attestation rows. persist_bulk_void appends BOTH void rows and
    // clears BOTH attestations in-memory, then save() fails (read-only parent) → the whole-DB restore
    // must revert EVERYTHING: no phantom void rows AND both attestations survive (snapshot == post).
    // This is the batch analog of kat_persist_void_rollback (same rollback machinery, N targets).
    #[cfg(unix)]
    #[test]
    fn kat_bulk_void_reverts_mid_batch() {
        use crate::edit::form::VoidTarget;
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
        let pp_str = "kat-bulk-void-revert-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let disposal_a = EventId::import(Source::River, SourceRef::new("bvr-disposal-a"));
        let disposal_b = EventId::import(Source::River, SourceRef::new("bvr-disposal-b"));
        let lot_origin = EventId::import(Source::River, SourceRef::new("bvr-lot"));

        let (ls_a, ls_b): (EventId, EventId);
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            let t1 = OffsetDateTime::from_unix_timestamp(1_748_000_100).unwrap();
            let mk_ls = |disposal: &EventId| {
                EventPayload::LotSelection(LotSelection {
                    disposal_event: disposal.clone(),
                    lots: vec![LotPick {
                        lot: LotId {
                            origin_event_id: lot_origin.clone(),
                            split_sequence: 0,
                        },
                        sat: 100_000,
                    }],
                })
            };
            ls_a = append_decision(session.conn(), mk_ls(&disposal_a), t0, UtcOffset::UTC, None)
                .unwrap();
            ls_b = append_decision(session.conn(), mk_ls(&disposal_b), t1, UtcOffset::UTC, None)
                .unwrap();
            btctax_cli::optimize_attest::set(session.conn(), &disposal_a, "attest-a", "2025-06-01")
                .unwrap();
            btctax_cli::optimize_attest::set(session.conn(), &disposal_b, "attest-b", "2025-06-02")
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
                eprintln!("bulk-void-revert KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let targets = vec![
            VoidTarget {
                target_event_id: ls_a.clone(),
                disposal_to_clear: Some(disposal_a.clone()),
                reclass_out_to_clear: None,
            },
            VoidTarget {
                target_event_id: ls_b.clone(),
                disposal_to_clear: Some(disposal_b.clone()),
                reclass_out_to_clear: None,
            },
        ];

        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = persist_bulk_void(&mut session, targets, now, "bulk void: nothing selected");
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must revert the WHOLE batch as RolledBack; got: {result:?}"
        );

        // No phantom void rows: the log is reverted to the pre-batch snapshot.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len(),
            "rollback must revert BOTH void appends (no residue)"
        );
        assert_eq!(post, pre, "whole-batch restore == pre-batch snapshot");

        // No phantom-cleared attestations: BOTH side-table rows survive.
        assert_eq!(
            btctax_cli::optimize_attest::get(session.conn(), &disposal_a)
                .unwrap()
                .as_deref(),
            Some("attest-a"),
            "rollback must restore attestation A cleared before the failed save"
        );
        assert_eq!(
            btctax_cli::optimize_attest::get(session.conn(), &disposal_b)
                .unwrap()
                .as_deref(),
            Some("attest-b"),
            "rollback must restore attestation B cleared before the failed save"
        );
    }

    // ── Bulk reclassify-outflow (bulk-reclassify-outflow, Cycle 5) ────────────

    /// Seed a vault with an Acquire (backing lot) + two PRICED pending `TransferOut`s. Returns the two
    /// TransferOut ids. Shared by the persist-layer reclassify KATs.
    fn seed_bulk_reclass_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> [btctax_core::EventId; 2] {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::append_import_batch;
        use btctax_core::{EventId, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::macros::datetime;
        use time::UtcOffset;

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let wallet = Some(WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        });
        let acq = EventId::import(Source::River, SourceRef::new("bro-acq"));
        let o1 = EventId::import(Source::River, SourceRef::new("bro-o1"));
        let o2 = EventId::import(Source::River, SourceRef::new("bro-o2"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let out = |id: &EventId, ts, sat| LedgerEvent {
            id: id.clone(),
            utc_timestamp: ts,
            original_tz: UtcOffset::UTC,
            wallet: wallet.clone(),
            payload: EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        };
        append_import_batch(
            session.conn(),
            &[
                LedgerEvent {
                    id: acq.clone(),
                    utc_timestamp: datetime!(2025-01-15 12:00:00 UTC),
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 1_000_000,
                        usd_cost: dec!(100.00),
                        fee_usd: dec!(0),
                        basis_source: BasisSource::ComputedFromCost,
                    }),
                },
                out(&o1, datetime!(2025-03-01 12:00:00 UTC), 60_000),
                out(&o2, datetime!(2025-06-15 12:00:00 UTC), 80_000),
            ],
        )
        .unwrap();
        session.save().unwrap();
        [o1, o2]
    }

    /// The `bulk_estimated` flag is CLEARED when its `ReclassifyOutflow` is voided — via BOTH
    /// `persist_void` (single) and `persist_bulk_void` (bulk) — and a single-`o` reclassify is NEVER
    /// flagged (a control). Closes the R0-I1 stale-`[est]` hole.
    #[test]
    fn kat_bulk_reclassify_void_clears_estimated_flag() {
        use btctax_core::event::{EventPayload, OutflowClass, ReclassifyOutflow};
        use btctax_core::persistence::load_all;
        use btctax_core::{DisposeKind, EventId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bro-voidclear";
        let [o1, o2] = seed_bulk_reclass_vault(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let t = |s: i64| OffsetDateTime::from_unix_timestamp(s).unwrap();

        // Bulk-reclassify BOTH → both flagged.
        persist_bulk_reclassify_outflow(
            &mut session,
            vec![(o1.clone(), dec!(50.40)), (o2.clone(), dec!(54.00))],
            DisposeKind::Sell,
            t(1_748_000_000),
            "bulk reclassify-outflow: nothing selected",
        )
        .unwrap();
        {
            let flagged = session.bulk_estimated().unwrap();
            assert!(
                flagged.contains_key(&o1) && flagged.contains_key(&o2),
                "both outflows flagged after bulk reclassify"
            );
        }

        // Find the ReclassifyOutflow decision ids (transfer_out_event → decision id).
        let ro_id = |session: &btctax_cli::Session, out: &EventId| -> EventId {
            load_all(session.conn())
                .unwrap()
                .into_iter()
                .find_map(|e| match &e.payload {
                    EventPayload::ReclassifyOutflow(ro) if &ro.transfer_out_event == out => {
                        Some(e.id.clone())
                    }
                    _ => None,
                })
                .expect("a live ReclassifyOutflow for the outflow")
        };

        // Arm 1 — persist_void (single) clears o1's flag; o2 survives.
        let ro1 = ro_id(&session, &o1);
        persist_void(&mut session, ro1, t(1_748_000_100)).unwrap();
        {
            let flagged = session.bulk_estimated().unwrap();
            assert!(!flagged.contains_key(&o1), "persist_void cleared o1's flag");
            assert!(
                flagged.contains_key(&o2),
                "o2's flag untouched by the single void"
            );
        }

        // Arm 2 — persist_bulk_void clears o2's flag (VoidTarget carries reclass_out_to_clear).
        let ro2 = ro_id(&session, &o2);
        persist_bulk_void(
            &mut session,
            vec![crate::edit::form::VoidTarget {
                target_event_id: ro2,
                disposal_to_clear: None,
                reclass_out_to_clear: Some(o2.clone()),
            }],
            t(1_748_000_200),
            "bulk void: nothing selected",
        )
        .unwrap();
        assert!(
            session.bulk_estimated().unwrap().is_empty(),
            "persist_bulk_void cleared o2's flag — no orphan rows remain"
        );

        // Control — o1 is pending again; re-reclassify via SINGLE `o` (a real price) → NOT flagged.
        persist_reclassify_outflow(
            &mut session,
            EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: o1.clone(),
                as_: OutflowClass::Dispose {
                    kind: DisposeKind::Sell,
                },
                principal_proceeds_or_fmv: dec!(99.99), // a REAL user-entered price
                fee_usd: None,
                donee: None,
            }),
            t(1_748_000_300),
        )
        .unwrap();
        assert!(
            !session.bulk_estimated().unwrap().contains_key(&o1),
            "single-`o` reclassify is NEVER flagged (no stale [est] survives the void)"
        );
    }

    /// [mirror `kat_bulk_void_reverts_mid_batch`] A mid-batch APPEND failure at row k>1 in
    /// `persist_bulk_reclassify_outflow` reverts the WHOLE batch — event log byte-unchanged AND NO
    /// phantom `bulk_estimated` flag rows survive (the whole-DB restore covers the side-table).
    #[test]
    fn kat_persist_bulk_reclassify_side_table_reverts_on_mid_batch_failure() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::DisposeKind;
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bro-midbatch";
        let [o1, o2] = seed_bulk_reclass_vault(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();

        // Fail the SECOND decision append (decision_seq 2): append #1 + its mark commit in-memory,
        // append #2 aborts → the WHOLE batch (incl. the row-1 mark) reverts.
        session
            .conn()
            .execute_batch(
                "CREATE TRIGGER inject_bro_midbatch BEFORE INSERT ON events \
                 WHEN NEW.decision_seq = 2 \
                 BEGIN SELECT RAISE(ABORT, 'injected mid-batch append failure'); END;",
            )
            .unwrap();

        let now = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let result = persist_bulk_reclassify_outflow(
            &mut session,
            vec![(o1.clone(), dec!(50.40)), (o2.clone(), dec!(54.00))],
            DisposeKind::Sell,
            now,
            "bulk reclassify-outflow: nothing selected",
        );
        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "mid-batch append failure must revert the whole batch (RolledBack); got {result:?}"
        );

        // Drop the trigger, then assert: event log reverted AND no phantom flag rows.
        session
            .conn()
            .execute_batch("DROP TRIGGER inject_bro_midbatch;")
            .unwrap();
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post, pre,
            "whole batch reverted — no phantom decision survives"
        );
        assert!(
            session.bulk_estimated().unwrap().is_empty(),
            "NO phantom bulk_estimated flag rows survive (row-1 mark reverted too)"
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
                wallet: None,
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
                    wallet: None,
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
            !btctax_core::is_revocable_payload(&EventPayload::SupersedeImport(SupersedeImport {
                conflict_event: ce.clone()
            })),
            "SupersedeImport must NOT be revocable"
        );
        assert!(
            !btctax_core::is_revocable_payload(&EventPayload::RejectImport(RejectImport {
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
                wallet: None,
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

    // ── KAT-P-ALLOCATE-STRICT-PREFIX — chunk 5, D5 ───────────────────────────
    //
    // persist_safe_harbor_allocate appends EXACTLY one SafeHarborAllocation to the tail, with
    // timely_allocation_attested == false, as_of_date == TRANSITION_DATE, and the supplied
    // method/pre2025_method. Strict-prefix: post == pre ++ [new]; returned id == tail row; payload
    // round-trips.
    #[test]
    fn kat_persist_allocate_single_append_strict_prefix() {
        use btctax_core::event::{EventPayload, SafeHarborAllocation};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::{AllocLot, AllocMethod, EventId, LotMethod, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p-alloc-sp-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed one decision so pre MAX(decision_seq) == 1 → tail must be seq 2.
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: LotMethod::Fifo,
                wallet: None,
            });
            append_decision(
                session.conn(),
                p,
                OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        let lots = vec![AllocLot {
            wallet: WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            },
            sat: 20_000_000,
            usd_basis: dec!(8550.00),
            acquired_at: date!(2024 - 01 - 15),
            dual_loss_basis: None,
            donor_acquired_at: None,
        }];
        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();
        let returned_id = persist_safe_harbor_allocate(
            &mut session,
            lots.clone(),
            AllocMethod::ProRata,
            LotMethod::Hifo,
            now,
        )
        .unwrap();

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 1, "exactly one append");
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows unchanged (strict prefix)"
        );
        let tail = &post[pre.len()];
        let tail_seq = tail.decision_seq.expect("tail must have decision_seq");
        assert_eq!(tail_seq, (pre_max_seq + 1) as i64, "tail seq == pre_max+1");
        assert_eq!(
            returned_id,
            EventId::Decision {
                seq: tail_seq as u64
            },
            "returned id must equal the tail Decision id"
        );

        // Exactly one SafeHarborAllocation in the whole log; it is the tail; attested == false.
        let allocs: Vec<SafeHarborAllocation> = post
            .iter()
            .filter_map(
                |r| match serde_json::from_str::<EventPayload>(&r.payload_json).unwrap() {
                    EventPayload::SafeHarborAllocation(a) => Some(a),
                    _ => None,
                },
            )
            .collect();
        assert_eq!(allocs.len(), 1, "exactly one SafeHarborAllocation appended");
        let a = &allocs[0];
        assert!(
            !a.timely_allocation_attested,
            "creation must be REVOCABLE (timely_allocation_attested == false)"
        );
        assert_eq!(a.as_of_date, btctax_core::conventions::TRANSITION_DATE);
        assert_eq!(a.method, AllocMethod::ProRata, "method threaded verbatim");
        assert_eq!(
            a.pre2025_method,
            LotMethod::Hifo,
            "pre2025_method threaded verbatim (G5)"
        );
        assert_eq!(a.lots, lots, "lots threaded verbatim");
    }

    // ── KAT-P-ALLOCATE-ROLLBACK — chunk 5, D5 (single append rolls back cleanly) ──
    #[cfg(unix)]
    #[test]
    fn kat_persist_allocate_rolls_back_on_failed_save() {
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::{AllocLot, AllocMethod, LotMethod, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use std::os::unix::fs::PermissionsExt;
        use time::{macros::date, OffsetDateTime};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p-alloc-rb-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Root-skip guard (chmod is a no-op as root).
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("alloc-rollback KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let lots = vec![AllocLot {
            wallet: WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            },
            sat: 20_000_000,
            usd_basis: dec!(8550.00),
            acquired_at: date!(2024 - 01 - 15),
            dual_loss_basis: None,
            donor_acquired_at: None,
        }];

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();

        // Make the vault's parent read-only → save() fails inside the persist fn.
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = persist_safe_harbor_allocate(
            &mut session,
            lots.clone(),
            AllocMethod::ActualPosition,
            LotMethod::Fifo,
            now,
        );
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must return RolledBack (single append rolls back cleanly); got: {result:?}"
        );

        // In-memory log reverted: NO SafeHarborAllocation residue.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len(),
            "rollback reverts the append (no residue)"
        );
        assert!(
            post.iter().all(|r| !matches!(
                serde_json::from_str::<EventPayload>(&r.payload_json).unwrap(),
                EventPayload::SafeHarborAllocation(_)
            )),
            "no SafeHarborAllocation may survive the rollback"
        );

        // Retry after restoring perms is CLEAN (re-appends with the SAME decision_seq).
        let retry = persist_safe_harbor_allocate(
            &mut session,
            lots,
            AllocMethod::ActualPosition,
            LotMethod::Fifo,
            now,
        )
        .unwrap();
        let post2 = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post2.len(), pre.len() + 1, "retry appends exactly one");
        assert_eq!(
            retry,
            btctax_core::EventId::Decision {
                seq: post2.last().unwrap().decision_seq.unwrap() as u64
            },
            "retry id matches the tail row"
        );
    }

    // ── KAT-BULK — persist_bulk_link_transfer (bulk-link-transfer D3) ─────────────

    /// Seed a vault with `pre` = 1 import TransferOut + 1 MethodElection decision (pre max
    /// decision_seq == 1). Returns the opened session (mut) via the passphrase string.
    fn bulk_seed(vault: &std::path::Path, key: &std::path::Path, pp_str: &str) {
        use btctax_core::event::{EventPayload, LedgerEvent, MethodElection, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use btctax_core::{EventId, WalletId};
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let batch = vec![LedgerEvent {
            id: EventId::import(Source::River, SourceRef::new("bulk-out")),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(WalletId::Exchange {
                provider: "River".into(),
                account: "main".into(),
            }),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 100_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        }];
        append_import_batch(session.conn(), &batch).unwrap();
        let p = EventPayload::MethodElection(MethodElection {
            effective_from: date!(2024 - 01 - 01),
            method: btctax_core::LotMethod::Fifo,
            wallet: None,
        });
        append_decision(
            session.conn(),
            p,
            OffsetDateTime::from_unix_timestamp(1_700_001_000).unwrap(),
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        session.save().unwrap();
    }

    fn cold_dest() -> btctax_core::WalletId {
        btctax_core::WalletId::SelfCustody {
            label: "cold".into(),
        }
    }

    fn synth_outs(n: usize) -> Vec<btctax_core::EventId> {
        use btctax_core::identity::{Source, SourceRef};
        (0..n)
            .map(|i| {
                btctax_core::EventId::import(Source::River, SourceRef::new(format!("bulk-o{i}")))
            })
            .collect()
    }

    /// EXACTLY N TransferLinks are tail-appended, all `Wallet(dest)`, over a strict prefix.
    #[test]
    fn kat_persist_bulk_link_strict_prefix() {
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::TransferTarget;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-sp-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        let outs = synth_outs(3);
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let n = persist_bulk_link_transfer(&mut session, outs.clone(), cold_dest(), now).unwrap();
        assert_eq!(n, 3, "three outs linked");

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 3, "exactly three appends");
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows unchanged (strict prefix)"
        );
        // The three tail rows are TransferLinks, in order, all Wallet(cold), seq pre_max+1..+3.
        for (i, out) in outs.iter().enumerate() {
            let row = &post[pre.len() + i];
            let seq = row.decision_seq.expect("tail decision_seq");
            assert_eq!(seq, (pre_max_seq + 1 + i as i64), "monotonic seq");
            let payload: EventPayload = serde_json::from_str(&row.payload_json).unwrap();
            match payload {
                EventPayload::TransferLink(tl) => {
                    assert_eq!(&tl.out_event, out, "out_event threaded in order");
                    assert_eq!(
                        tl.in_event_or_wallet,
                        TransferTarget::Wallet(cold_dest()),
                        "every link targets Wallet(dest)"
                    );
                }
                other => panic!("tail must be TransferLink, got {other:?}"),
            }
        }
        // On-disk == in-memory.
        drop(session);
        let s2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        assert_eq!(load_all_ordered(s2.conn()).unwrap(), post);
    }

    /// Empty `out_events` → `NoChange`, log byte-unchanged (never append zero + save).
    #[test]
    fn kat_persist_bulk_link_refuses_empty() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-empty-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result = persist_bulk_link_transfer(&mut session, vec![], cold_dest(), now);
        assert!(
            matches!(result, Err(PersistError::NoChange(_))),
            "empty selection must refuse with NoChange; got {result:?}"
        );
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post, pre, "refusal writes nothing");
    }

    /// chmod → failed save → whole batch reverts (`RolledBack`); log unchanged; retry clean.
    #[cfg(unix)]
    #[test]
    fn kat_persist_bulk_link_rolls_back_on_failed_save() {
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use std::os::unix::fs::PermissionsExt;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-rb-pass";
        bulk_seed(&vault, &key, pp_str);

        // Root-skip guard (chmod is a no-op as root).
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("bulk-rollback KAT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let outs = synth_outs(3);
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();

        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = persist_bulk_link_transfer(&mut session, outs.clone(), cold_dest(), now);
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "failed save must return RolledBack; got {result:?}"
        );
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len(),
            "rollback reverts ALL appends (no residue)"
        );
        assert!(
            post.iter().all(|r| !matches!(
                serde_json::from_str::<EventPayload>(&r.payload_json).unwrap(),
                EventPayload::TransferLink(_)
            )),
            "no TransferLink may survive the rollback"
        );

        // Retry after restoring perms re-appends all three cleanly.
        let n = persist_bulk_link_transfer(&mut session, outs, cold_dest(), now).unwrap();
        assert_eq!(n, 3);
        let post2 = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post2.len(), pre.len() + 3, "retry appends exactly three");
    }

    /// [R0-I1] A mid-batch APPEND failure at row k>1 reverts the WHOLE batch — event-log
    /// byte-unchanged, NO phantom residue from the appends before k, retry clean. Injection: a
    /// BEFORE-INSERT trigger that RAISE(ABORT)s when the SECOND bulk append's decision_seq lands, so
    /// append #1 has ALREADY committed (per-call) when append #2 fails.
    #[test]
    fn kat_persist_bulk_link_reverts_mid_batch_append_failure() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-midbatch-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Fail the SECOND bulk append: pre_max+1 (seq 2) commits, pre_max+2 (seq 3) aborts.
        let target = pre_max_seq + 2;
        session
            .conn()
            .execute_batch(&format!(
                "CREATE TRIGGER inject_midbatch_fail BEFORE INSERT ON events \
                 WHEN NEW.decision_seq = {target} \
                 BEGIN SELECT RAISE(ABORT, 'injected mid-batch append failure'); END;"
            ))
            .unwrap();

        let outs = synth_outs(3);
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result = persist_bulk_link_transfer(&mut session, outs.clone(), cold_dest(), now);
        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "mid-batch append failure must revert the whole batch (RolledBack); got {result:?}"
        );

        // Event-log byte-unchanged: the seq-2 phantom append is reverted too.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post, pre,
            "whole batch reverted — no phantom decision from append #1 survives"
        );

        // Retry after removing the injection is clean (all three appended).
        session
            .conn()
            .execute_batch("DROP TRIGGER inject_midbatch_fail;")
            .unwrap();
        let n = persist_bulk_link_transfer(&mut session, outs, cold_dest(), now).unwrap();
        assert_eq!(n, 3, "retry links all three cleanly");
        let post2 = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post2.len(), pre.len() + 3);
    }

    // ── KAT-BULK-DECISIONS — persist_bulk_decisions direct unit KATs (Task 1) ─────────────────────

    /// Build N `TransferLink { out_event, Wallet(cold) }` payloads over `synth_outs` — a concrete
    /// payload vec to drive the shared helper directly (any decision payload would do).
    fn synth_link_payloads(n: usize) -> Vec<btctax_core::EventPayload> {
        synth_outs(n)
            .into_iter()
            .map(|out_event| {
                btctax_core::EventPayload::TransferLink(btctax_core::event::TransferLink {
                    out_event,
                    in_event_or_wallet: btctax_core::TransferTarget::Wallet(cold_dest()),
                })
            })
            .collect()
    }

    /// Empty `payloads` → `NoChange` carrying the CALLER-SUPPLIED label; log byte-unchanged.
    #[test]
    fn kat_persist_bulk_decisions_refuses_empty() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bd-empty-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result = persist_bulk_decisions(&mut session, vec![], now, "custom-empty-label");
        match result {
            Err(PersistError::NoChange(btctax_cli::CliError::Usage(msg))) => {
                assert_eq!(
                    msg, "custom-empty-label",
                    "empty label passes through verbatim"
                );
            }
            other => panic!("empty payloads must refuse with NoChange(Usage); got {other:?}"),
        }
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post, pre, "refusal writes nothing");
    }

    /// A mid-batch APPEND failure at row k>1 reverts the WHOLE batch via the shared helper — event-log
    /// byte-unchanged, NO phantom residue, retry clean.
    #[test]
    fn kat_persist_bulk_decisions_reverts_mid_batch() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bd-midbatch-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Fail the SECOND append: pre_max+1 (seq 2) commits, pre_max+2 (seq 3) aborts.
        let target = pre_max_seq + 2;
        session
            .conn()
            .execute_batch(&format!(
                "CREATE TRIGGER inject_bd_midbatch_fail BEFORE INSERT ON events \
                 WHEN NEW.decision_seq = {target} \
                 BEGIN SELECT RAISE(ABORT, 'injected mid-batch append failure'); END;"
            ))
            .unwrap();

        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result =
            persist_bulk_decisions(&mut session, synth_link_payloads(3), now, "bd: nothing");
        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "mid-batch append failure must revert the whole batch (RolledBack); got {result:?}"
        );
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post, pre,
            "whole batch reverted — no phantom decision from append #1 survives"
        );

        // Retry after removing the injection is clean (all three appended).
        session
            .conn()
            .execute_batch("DROP TRIGGER inject_bd_midbatch_fail;")
            .unwrap();
        let n = persist_bulk_decisions(&mut session, synth_link_payloads(3), now, "bd: nothing")
            .unwrap();
        assert_eq!(n, 3, "retry appends all three cleanly");
        let post2 = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post2.len(), pre.len() + 3);
    }

    // ── KAT-BULK-STI — persist_bulk_self_transfer_in (bulk-classify-inbound-self-transfer D3) ─────

    fn synth_ins(n: usize) -> Vec<btctax_core::EventId> {
        use btctax_core::identity::{Source, SourceRef};
        (0..n)
            .map(|i| {
                btctax_core::EventId::import(Source::River, SourceRef::new(format!("bulk-in{i}")))
            })
            .collect()
    }

    /// EXACTLY N `ClassifyInbound{SelfTransferMine{None,None}}` tail-appended over a strict prefix,
    /// each threading its `transfer_in_event` in order; on-disk == in-memory.
    #[test]
    fn kat_persist_bulk_sti_strict_prefix() {
        use btctax_core::event::EventPayload;
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::InboundClass;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulksti-sp-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        let ins = synth_ins(3);
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let n = persist_bulk_self_transfer_in(&mut session, ins.clone(), now).unwrap();
        assert_eq!(n, 3, "three inbounds classified");

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post.len(), pre.len() + 3, "exactly three appends");
        assert_eq!(
            &post[..pre.len()],
            pre.as_slice(),
            "first pre.len() rows unchanged (strict prefix)"
        );
        for (i, in_ev) in ins.iter().enumerate() {
            let row = &post[pre.len() + i];
            let seq = row.decision_seq.expect("tail decision_seq");
            assert_eq!(seq, (pre_max_seq + 1 + i as i64), "monotonic seq");
            let payload: EventPayload = serde_json::from_str(&row.payload_json).unwrap();
            match payload {
                EventPayload::ClassifyInbound(ci) => {
                    assert_eq!(&ci.transfer_in_event, in_ev, "in_event threaded in order");
                    assert!(
                        matches!(
                            ci.as_,
                            InboundClass::SelfTransferMine {
                                basis: None,
                                acquired_at: None
                            }
                        ),
                        "every classification is SelfTransferMine{{None,None}}"
                    );
                }
                other => panic!("tail must be ClassifyInbound, got {other:?}"),
            }
        }
        drop(session);
        let s2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        assert_eq!(load_all_ordered(s2.conn()).unwrap(), post);
    }

    /// [R0-M1] Empty `in_events` → `NoChange`, log byte-unchanged (never append zero + save).
    #[test]
    fn kat_persist_bulk_sti_refuses_empty() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulksti-empty-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result = persist_bulk_self_transfer_in(&mut session, vec![], now);
        assert!(
            matches!(result, Err(PersistError::NoChange(_))),
            "empty selection must refuse with NoChange; got {result:?}"
        );
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post, pre, "refusal writes nothing");
    }

    /// [bulk-I1] A mid-batch APPEND failure at row k>1 reverts the WHOLE batch — event-log
    /// byte-unchanged, NO phantom residue from the appends before k, retry clean. Injection: a
    /// BEFORE-INSERT trigger that RAISE(ABORT)s when the SECOND bulk append's decision_seq lands, so
    /// append #1 has ALREADY committed (per-call) when append #2 fails.
    #[test]
    fn kat_persist_bulk_sti_reverts_mid_batch_append_failure() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulksti-midbatch-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let pre_max_seq = pre.iter().filter_map(|r| r.decision_seq).max().unwrap_or(0);
        assert_eq!(pre_max_seq, 1, "pre max decision_seq must be 1");

        // Fail the SECOND bulk append: pre_max+1 (seq 2) commits, pre_max+2 (seq 3) aborts.
        let target = pre_max_seq + 2;
        session
            .conn()
            .execute_batch(&format!(
                "CREATE TRIGGER inject_sti_midbatch_fail BEFORE INSERT ON events \
                 WHEN NEW.decision_seq = {target} \
                 BEGIN SELECT RAISE(ABORT, 'injected mid-batch append failure'); END;"
            ))
            .unwrap();

        let ins = synth_ins(3);
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result = persist_bulk_self_transfer_in(&mut session, ins.clone(), now);
        assert!(
            matches!(result, Err(PersistError::RolledBack(_))),
            "mid-batch append failure must revert the whole batch (RolledBack); got {result:?}"
        );

        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post, pre,
            "whole batch reverted — no phantom decision from append #1 survives"
        );

        session
            .conn()
            .execute_batch("DROP TRIGGER inject_sti_midbatch_fail;")
            .unwrap();
        let n = persist_bulk_self_transfer_in(&mut session, ins, now).unwrap();
        assert_eq!(n, 3, "retry classifies all three cleanly");
        let post2 = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post2.len(), pre.len() + 3);
    }

    // ── KAT-BULK-INCOME — persist_bulk_classify_income (bulk-classify-inbound-income, Cycle 4) ─────

    /// [empty-guard] Empty `payloads` → `NoChange`, log byte-unchanged (never append zero + save). The
    /// classify-income wrapper delegates to `persist_bulk_decisions`, so the empty guard is inherited.
    #[test]
    fn bulk_income_empty_refuses() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_store::Passphrase;
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulkincome-empty-pass";
        bulk_seed(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();
        let now = OffsetDateTime::from_unix_timestamp(1_700_002_000).unwrap();
        let result = persist_bulk_classify_income(&mut session, vec![], now);
        assert!(
            matches!(result, Err(PersistError::NoChange(_))),
            "empty selection must refuse with NoChange; got {result:?}"
        );
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(post, pre, "refusal writes nothing");
    }
}
