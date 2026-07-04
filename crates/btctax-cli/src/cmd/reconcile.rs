//! reconcile decision emitters (FR6/FR7/FR8, §7.2). Each fn builds exactly ONE `EventPayload` decision
//! variant and appends it via `append_decision` (monotonic `decision_seq`), then saves. Decisions are
//! append-only and re-projectable; the engine resolves precedence (latest-`decision_seq`, Void-first).
//! `now` is the injected decision creation-time / safe-harbor made-date (§6.2) — deterministic in tests.
//!
//! Also contains the `set-donation-details` / `show-donation-details` side-table commands (no decision
//! append — these write to the `donation_details` side-table directly, like `tax-profile set`).
use crate::{
    BulkFilter, BulkLinkPlan, BulkResolvePlan, BulkStiFilter, BulkStiPlan, BulkVoidPlan, CliError,
    MatchProposal, Session,
};
use btctax_core::conventions::TRANSITION_DATE;
use btctax_core::persistence::{append_decision, load_all};
use btctax_core::{
    AllocMethod, BlockerKind, ClassifyInbound, ClassifyRaw, DonationDetails, EventId, EventPayload,
    InboundClass, IncomeKind, LotId, LotMethod, LotPick, LotSelection, ManualFmv, MethodElection,
    OutflowClass, ReclassifyIncome, ReclassifyOutflow, RejectImport, RemovalKind,
    SafeHarborAllocation, SelfTransferPassthrough, SupersedeImport, TaxDate, TransferLink,
    TransferTarget, Usd, VoidDecisionEvent, WalletId,
};
use btctax_store::Passphrase;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

use crate::eventref::parse_event_id;

/// Append one decision (creation tz = UTC; decisions are not wallet-scoped) and persist.
fn append_and_save(
    session: &mut Session,
    payload: EventPayload,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let id = append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    session.save()?;
    Ok(id)
}

/// FR6: classify an externally-sourced inbound `TransferIn` as Income or a received Gift. For Income
/// this supplies the FMV basis; for Gift it supplies donor basis/date + fmv_at_gift (TP11 dual-basis).
/// This is the re-supply path for the §9.1 Swan `deposit` basis GAP.
pub fn classify_inbound(
    vault_path: &Path,
    pp: &Passphrase,
    in_ref: &str,
    class: InboundClass,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_in_event = parse_event_id(in_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ClassifyInbound(ClassifyInbound {
        transfer_in_event,
        as_: class,
    });
    append_and_save(&mut session, payload, now)
}

/// FR6: reclassify a pending `TransferOut` as a Sell/Spend disposition, a Gift out, or a Donation.
/// `principal` is the gross proceeds (Dispose) or FMV-at-transfer (Gift/Donate); `fee_usd` is the
/// optional disposition fee (TP8 / TP2). The engine applies the configured TP8 (c)/(b) fee treatment.
/// `donee` is the optional free-form donee identifier (Chunk 2); `None` for disposals and legacy records.
#[allow(clippy::too_many_arguments)]
pub fn reclassify_outflow(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    class: OutflowClass,
    principal: Usd,
    fee_usd: Option<Usd>,
    donee: Option<String>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
        transfer_out_event,
        as_: class,
        principal_proceeds_or_fmv: principal,
        fee_usd,
        donee,
    });
    append_and_save(&mut session, payload, now)
}

/// FR3: set a manual FMV on an event (`ManualEntry`), clearing its `fmv_missing` blocker.
pub fn set_fmv(
    vault_path: &Path,
    pp: &Passphrase,
    event_ref: &str,
    usd_fmv: Usd,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let event = parse_event_id(event_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::ManualFmv(ManualFmv { event, usd_fmv }),
        now,
    )
}

/// FR8: void a revocable decision. Voiding a non-revocable / effective-allocation target raises
/// `decision_conflicts` in the projection (no effect) — the CLI only appends; the engine adjudicates.
///
/// When the voided decision is a `LotSelection`, also clears that disposal's `optimize_attestation`
/// row ATOMICALLY (same in-memory DB, one `session.save()`). This closes the revocation-completeness
/// edge: without the clear, a post-void re-run where the FIFO default equals the optimum
/// (`proposed==current`, D ∈ unchanged) could mislabel D as `AttestedRecording` from a stale row
/// the user never attested in this context. Non-LotSelection decisions are unaffected (the
/// `optimize_attestation` table has no row to clear — the delete is a no-op).
pub fn void(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let target_event_id = parse_event_id(target_ref)?;
    let mut session = Session::open(vault_path, pp)?;

    // Determine if the target decision is a LotSelection so we can clear its attestation row.
    // Load events first; the find is O(n) but n is small and void is infrequent.
    let events = load_all(session.conn())?;
    let disposal_to_clear: Option<EventId> = events
        .iter()
        .find(|e| e.id == target_event_id)
        .and_then(|e| match &e.payload {
            EventPayload::LotSelection(ls) => Some(ls.disposal_event.clone()),
            _ => None,
        });

    // Append the VoidDecisionEvent (no save yet — we batch with the attestation clear below).
    let id = append_decision(
        session.conn(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id }),
        now,
        UtcOffset::UTC,
        None,
    )?;

    // If the voided decision was a LotSelection, clear the attestation row for its disposal
    // ATOMICALLY — both the void event and the row delete land in the same in-memory Connection
    // and are flushed together by the single `session.save()` below (mirrors accept's atomic
    // co-persist). Idempotent: clearing an absent row is Ok (no error).
    if let Some(disposal) = disposal_to_clear {
        crate::optimize_attest::clear(session.conn(), &disposal)?;
    }

    session.save()?;
    Ok(id)
}

/// FR2/§7.3: resolve an `Unclassified` row to a real imported payload (preserving the target EventId).
/// The payload is supplied as JSON (`EventPayload` is `Deserialize`) — e.g. `{"Acquire":{…}}`.
pub fn classify_raw(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    payload_json: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let target = parse_event_id(target_ref)?;
    let as_: EventPayload = serde_json::from_str(payload_json)
        .map_err(|e| CliError::Usage(format!("bad --payload-json: {e}")))?;
    if !as_.is_imported() {
        return Err(CliError::Usage(
            "classify-raw payload must be an imported variant (Acquire/Income/Dispose/TransferOut/TransferIn/Unclassified)".into(),
        ));
    }
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::ClassifyRaw(ClassifyRaw {
            target,
            as_: Box::new(as_),
        }),
        now,
    )
}

/// FR1/FR8: accept an `ImportConflict` (apply the new payload to the target, keeping its EventId).
pub fn accept_conflict(
    vault_path: &Path,
    pp: &Passphrase,
    conflict_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let conflict_event = parse_event_id(conflict_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::SupersedeImport(SupersedeImport { conflict_event }),
        now,
    )
}

/// FR1/FR8: reject an `ImportConflict` (keep the original; clear the blocker).
pub fn reject_conflict(
    vault_path: &Path,
    pp: &Passphrase,
    conflict_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let conflict_event = parse_event_id(conflict_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::RejectImport(RejectImport { conflict_event }),
        now,
    )
}

/// bulk-link-transfer D2 — Phase 1 (read): open the session and compute the bulk link-transfer plan.
///
/// Two-phase by design [R0-M2]: this read phase opens/renders NOTHING to the vault, so the
/// interactive `y/N` confirmation stays a thin, untested shell in the `main.rs` dispatch. The plan is
/// the shared read helper `Session::bulk_link_transfer_plan` (D1). The session (and its VaultLock) is
/// dropped on return, before the confirmation prompt runs.
pub fn bulk_link_plan(
    vault_path: &Path,
    pp: &Passphrase,
    filter: BulkFilter,
    dest: WalletId,
) -> Result<BulkLinkPlan, CliError> {
    let session = Session::open(vault_path, pp)?;
    session.bulk_link_transfer_plan(filter, dest)
}

/// bulk-link-transfer D2 — Phase 2 (write): atomically link every `out_event` to `dest` as a
/// self-transfer. Appends one `TransferLink { out_event, Wallet(dest) }` per row, then a SINGLE
/// `save`. All-or-nothing: a mid-batch `append_decision` failure returns `Err` BEFORE the save, and
/// the local `Session` is dropped with nothing written — the exact one-session / N-append / one-save
/// atomicity of `import_selections`. Returns the number of outflows linked.
pub fn apply_bulk_link_transfer(
    vault_path: &Path,
    pp: &Passphrase,
    out_events: Vec<EventId>,
    dest: WalletId,
    now: OffsetDateTime,
) -> Result<usize, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    for out_event in &out_events {
        let payload = EventPayload::TransferLink(TransferLink {
            out_event: out_event.clone(),
            in_event_or_wallet: TransferTarget::Wallet(dest.clone()),
        });
        // `?` on a mid-batch failure returns before `save` — the in-memory session is discarded, so
        // nothing lands on disk (CLI atomicity; the TUI path must instead ROLL BACK, see D3 [R0-I1]).
        append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    }
    session.save()?;
    Ok(out_events.len())
}

/// bulk-classify-inbound-self-transfer D2 — Phase 1 (read): open the session and compute the bulk STI
/// plan. Two-phase by design (mirrors `bulk_link_plan`): this read phase renders NOTHING to the vault,
/// so the interactive `y/N` confirmation stays a thin, untested shell in the `main.rs` dispatch. The
/// plan is the shared read helper `Session::bulk_self_transfer_in_plan` (D1). The session (and its
/// VaultLock) is dropped on return, before the confirmation prompt runs.
pub fn bulk_self_transfer_in_plan(
    vault_path: &Path,
    pp: &Passphrase,
    filter: BulkStiFilter,
) -> Result<BulkStiPlan, CliError> {
    let session = Session::open(vault_path, pp)?;
    session.bulk_self_transfer_in_plan(filter)
}

/// bulk-classify-inbound-self-transfer D2 — Phase 2 (write): atomically classify every `in_event` as
/// a `SelfTransferMine { basis: None, acquired_at: None }` ($0 conservative basis, non-taxable).
/// Appends one `ClassifyInbound { transfer_in_event, SelfTransferMine{None, None} }` per row, then a
/// SINGLE `save`. All-or-nothing: a mid-batch `append_decision` failure returns `Err` BEFORE the save,
/// and the local `Session` is dropped with nothing written — the exact one-session / N-append /
/// one-save atomicity of `apply_bulk_link_transfer`. Returns the number of inbounds classified.
pub fn apply_bulk_self_transfer_in(
    vault_path: &Path,
    pp: &Passphrase,
    in_events: Vec<EventId>,
    now: OffsetDateTime,
) -> Result<usize, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    for in_event in &in_events {
        let payload = EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: in_event.clone(),
            as_: InboundClass::SelfTransferMine {
                basis: None,
                acquired_at: None,
            },
        });
        // `?` on a mid-batch failure returns before `save` — the in-memory session is discarded, so
        // nothing lands on disk (CLI atomicity; the TUI path must instead ROLL BACK, see D3).
        append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    }
    session.save()?;
    Ok(in_events.len())
}

/// bulk-classify-inbound-income (Cycle 4) — Phase 1 (read): open the session and compute the bulk
/// classify-income plan. Two-phase by design (mirrors `bulk_self_transfer_in_plan`): this read phase
/// renders NOTHING to the vault. The plan is the shared read helper `Session::bulk_classify_income_plan`
/// — it excludes missing-price rows [#a tax-safety] and reports them as `excluded_missing_price`.
pub fn bulk_classify_income_plan(
    vault_path: &Path,
    pp: &Passphrase,
    filter: crate::BulkIncomeFilter,
) -> Result<crate::BulkIncomePlan, CliError> {
    let session = Session::open(vault_path, pp)?;
    session.bulk_classify_income_plan(filter)
}

/// bulk-classify-inbound-income (Cycle 4) — Phase 2 (write): atomically classify every `in_event` as
/// `Income { kind, fmv, business }` with a PER-ROW auto-FMV. Its OWN append-loop (mirrors
/// `apply_bulk_self_transfer_in`; NOT the tui-edit `persist_bulk_decisions`, which btctax-cli cannot
/// reach — dependency cycle, R0-I1). One `ClassifyInbound { transfer_in_event, Income{..} }` per row,
/// bare `?`-before-`save` (a mid-batch failure returns before `save` → the in-memory session is
/// discarded, nothing lands on disk = CLI atomicity), then a SINGLE `session.save()`.
///
/// **[#a] Auto-FMV is resolved per-row via `fmv_of(date, sat)`.** The dispatch derives `in_events`
/// from `plan.included` (the fmv-`Some` rows only), so every id here resolves to a real price and the
/// `fmv: Option<Usd>` field is `Some` — a persisted `Income{fmv:None}` (→ Hard `FmvMissing` year-gate)
/// is never emitted. `kind` + `business` are UNIFORM across the batch; `fmv` is per-row. Returns the
/// number classified.
pub fn apply_bulk_classify_inbound_income(
    vault_path: &Path,
    pp: &Passphrase,
    in_events: Vec<EventId>,
    kind: IncomeKind,
    business: bool,
    now: OffsetDateTime,
) -> Result<usize, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    let prices = btctax_adapters::BundledPrices::load()?;
    let events = load_all(session.conn())?;
    let index: std::collections::HashMap<&EventId, &btctax_core::LedgerEvent> =
        events.iter().map(|e| (&e.id, e)).collect();
    // Resolve (sat, date) per in_event from the event log (the FMV is a per-row auto-value; R0-M3:
    // `fmv` is already Option<Usd> so it takes `fmv_of(..)` DIRECTLY — never `Some(fmv_of(..))`).
    let mut n = 0usize;
    for in_event in &in_events {
        let Some(ev) = index.get(in_event) else {
            continue;
        };
        let EventPayload::TransferIn(ti) = &ev.payload else {
            continue;
        };
        let date = btctax_core::conventions::tax_date(ev.utc_timestamp, ev.original_tz);
        let fmv = btctax_core::price::fmv_of(&prices, date, ti.sat);
        let payload = EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: in_event.clone(),
            as_: InboundClass::Income {
                kind,
                fmv,
                business,
            },
        });
        // `?` on a mid-batch failure returns before `save` — the in-memory session is discarded, so
        // nothing lands on disk (CLI atomicity; the TUI path instead ROLLS BACK).
        append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
        n += 1;
    }
    session.save()?;
    Ok(n)
}

/// bulk-resolve-conflict D2 — Phase 1 (read): open the session and compute the bulk resolve-conflict
/// plan. Two-phase by design (mirrors `bulk_link_plan`): this read phase renders NOTHING to the vault,
/// so the interactive `y/N` confirmation stays a thin, untested shell in the `main.rs` dispatch. The
/// plan is the shared read helper `Session::bulk_resolve_conflict_plan` (D1). The session (and its
/// VaultLock) is dropped on return, before the confirmation prompt runs.
pub fn bulk_resolve_conflict_plan(
    vault_path: &Path,
    pp: &Passphrase,
) -> Result<BulkResolvePlan, CliError> {
    let session = Session::open(vault_path, pp)?;
    session.bulk_resolve_conflict_plan()
}

/// bulk-resolve-conflict D2 — Phase 2 (write), ACCEPT: atomically append one `SupersedeImport` per
/// conflict (adopt each `new_payload` onto its target id), then a SINGLE `save`. All-or-nothing: a
/// mid-batch `append_decision` failure returns `Err` BEFORE the save, and the local `Session` is
/// dropped with nothing written — the exact one-session / N-append / one-save atomicity of
/// `apply_bulk_link_transfer`. Mirrors the shipped single-item split `accept_conflict`/`reject_conflict`
/// [R0-I1 — NO `ResolveKind` in the CLI; it lives only in btctax-tui-edit]. Returns the number accepted.
pub fn apply_bulk_accept_conflicts(
    vault_path: &Path,
    pp: &Passphrase,
    conflict_events: Vec<EventId>,
    now: OffsetDateTime,
) -> Result<usize, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    for conflict_event in &conflict_events {
        let payload = EventPayload::SupersedeImport(SupersedeImport {
            conflict_event: conflict_event.clone(),
        });
        // `?` on a mid-batch failure returns before `save` — the in-memory session is discarded, so
        // nothing lands on disk (CLI atomicity; the TUI path instead ROLLS BACK via persist_bulk_decisions).
        append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    }
    session.save()?;
    Ok(conflict_events.len())
}

/// bulk-resolve-conflict D2 — Phase 2 (write), REJECT: atomically append one `RejectImport` per
/// conflict (keep each target's current payload; clear the blocker), then a SINGLE `save`. Same
/// all-or-nothing CLI atomicity as `apply_bulk_accept_conflicts`. Returns the number rejected.
pub fn apply_bulk_reject_conflicts(
    vault_path: &Path,
    pp: &Passphrase,
    conflict_events: Vec<EventId>,
    now: OffsetDateTime,
) -> Result<usize, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    for conflict_event in &conflict_events {
        let payload = EventPayload::RejectImport(RejectImport {
            conflict_event: conflict_event.clone(),
        });
        append_decision(session.conn(), payload, now, UtcOffset::UTC, None)?;
    }
    session.save()?;
    Ok(conflict_events.len())
}

/// bulk-void D2 — Phase 1 (read): open the session and compute the bulk-void plan. Two-phase by design
/// (mirrors `bulk_resolve_conflict_plan`): this read phase renders NOTHING to the vault, so the
/// interactive `y/N` confirmation stays a thin, untested shell in the `main.rs` dispatch. The plan is
/// the shared read helper `Session::bulk_void_plan` (D1) — the SINGLE `voidable_decisions` predicate,
/// which OMITS effective allocations (#7). The session (and its VaultLock) is dropped on return.
pub fn bulk_void_plan(vault_path: &Path, pp: &Passphrase) -> Result<BulkVoidPlan, CliError> {
    let session = Session::open(vault_path, pp)?;
    session.bulk_void_plan()
}

/// bulk-void D2 — Phase 2 (write): atomically append one `VoidDecisionEvent` per `target` AND, for each
/// `LotSelection` target, clear its optimizer attestation (`optimize_attest::clear`), then a SINGLE
/// `save`. All-or-nothing: a mid-batch `append_decision` / `clear` failure returns `Err` BEFORE the
/// save, and the local `Session` is dropped with nothing written — the exact one-session / N-append /
/// one-save atomicity of `apply_bulk_accept_conflicts` (the TUI path instead ROLLS BACK explicitly via
/// `persist_bulk_void`).
///
/// # [R0-M3] the ONLY CLI-layer #7 defense
/// `targets` MUST be exactly the `bulk_void_plan` rows (predicate-filtered), re-derived from the vault
/// inside the dispatch — NEVER raw `--ref` ids. The single CLI `void` does NO `effective_alloc` check,
/// so a raw-id bulk path would let a caller void an effective allocation → Hard `DecisionConflict`.
/// Each target is `(target_event_id, disposal_to_clear)` carried straight from the plan row. Returns
/// the number voided.
pub fn apply_bulk_void(
    vault_path: &Path,
    pp: &Passphrase,
    targets: Vec<(EventId, Option<EventId>)>,
    now: OffsetDateTime,
) -> Result<usize, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    for (target_event_id, disposal_to_clear) in &targets {
        // `?` on a mid-batch failure returns before `save` — the in-memory session is discarded, so
        // nothing lands on disk (CLI atomicity; the TUI path instead ROLLS BACK via persist_bulk_void).
        append_decision(
            session.conn(),
            EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                target_event_id: target_event_id.clone(),
            }),
            now,
            UtcOffset::UTC,
            None,
        )?;
        // Per-LotSelection side-effect: clear its disposal's optimizer attestation ATOMICALLY (same
        // in-memory conn, flushed by the single save below). Idempotent — clearing an absent row is Ok.
        if let Some(disposal) = disposal_to_clear {
            crate::optimize_attest::clear(session.conn(), disposal)?;
        }
    }
    session.save()?;
    Ok(targets.len())
}

/// FR6/TP7: confirm a self-transfer. `target` is a destination `TransferIn` event (`--to-event`) or a
/// known wallet (`--to-wallet`); the engine relocates the lots carrying basis + acquired_at.
pub fn link_transfer(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    target: TransferTarget,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::TransferLink(TransferLink {
        out_event,
        in_event_or_wallet: target,
    });
    append_and_save(&mut session, payload, now)
}

/// self-transfer-passthrough C3 — Phase 1 (read): compute the self-transfer match proposals on the HELD
/// session (READ-ONLY; appends/persists NOTHING). Mirrors `bulk_link_plan`: the shared read helper is
/// `Session::self_transfer_match_plan`; the session (and its VaultLock) is dropped on return.
pub fn self_transfer_match_plan(
    vault_path: &Path,
    pp: &Passphrase,
) -> Result<Vec<MatchProposal>, CliError> {
    let session = Session::open(vault_path, pp)?;
    session.self_transfer_match_plan()
}

/// self-transfer-passthrough C3 — Phase 2 (write), DROP: append one `SelfTransferPassthrough` decision
/// mapping BOTH legs to `Op::Skip` (non-taxable passthrough). Mirrors `link_transfer` (one append + save).
/// The RELOCATE case is NOT here — it routes to the EXISTING `link_transfer(out, InEvent(in))` (G-RELOCATE-REUSE).
pub fn apply_self_transfer_passthrough(
    vault_path: &Path,
    pp: &Passphrase,
    in_ref: &str,
    out_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let in_event = parse_event_id(in_ref)?;
    let out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::SelfTransferPassthrough(SelfTransferPassthrough {
        in_event,
        out_event,
    });
    append_and_save(&mut session, payload, now)
}

/// FR7/§7.4: build a Path-B safe-harbor allocation that seeds from the **pre-2025 residue** (the
/// 2025-01-01 Universal-pool position), so it conserves against the engine's allocation-independent
/// conservation guard.
///
/// I-1: the engine's guard compares `Σ alloc.lots.sat`/`usd_basis` to `transition::universal_snapshot`
/// — a pre-2025-ONLY fold of the Universal pool (resolve.rs §7.4, step 3). The FULL projection's
/// `state.lots` reflects POST-2025-disposal residuals (a 2025 Sell consumes pre-2025 lots in FIFO), so
/// seeding from them would yield `alloc_sat < snap.held_sat` → hard `SafeHarborUnconservable` → Path A,
/// breaking the normal workflow. Instead we re-project a pre-2025-only event subset and read ITS lots:
///   - keep ONLY import events whose tax-date `< 2025-01-01` (drop every 2025+ acquire/dispose/transfer);
///   - keep ALL reconciliation decisions/conflicts — they SHAPE the residue (a 2026 `ClassifyInbound`
///     supplies a pre-2025 `TransferIn`'s basis; `ReclassifyOutflow`/`TransferLink` consume/relocate a
///     pre-2025 lot) — and carry a 2026 made-date, so they must NOT be tax-date-filtered;
///   - DROP any prior `SafeHarborAllocation` so the residue stays allocation-INDEPENDENT (matches
///     `universal_snapshot`, which never applies a seed) → re-allocation is idempotent.
///
/// This subset re-runs the IDENTICAL `fold_event` arms the engine's snapshot uses, so the totals match
/// exactly (the only difference, Path A's per-wallet relocation, preserves sat/basis 1:1; `finalize`
/// attributes Universal-pool lots by `lot.wallet`). For `ActualPosition` the per-wallet assignment falls
/// out of those residue lots' `wallet` (= the wallet holding each lot at 2025-01-01). `ProRata` still
/// seeds from these actuals; a true cross-wallet pro-rata redistribution is a manual-input refinement
/// (Open question O4). The engine's `SafeHarborUnconservable` guard remains the backstop for any residual
/// drift (e.g. a rare self-transfer straddling the 2025 boundary) — fails closed, never silent wrong tax.
pub fn safe_harbor_allocate(
    vault_path: &Path,
    pp: &Passphrase,
    method: AllocMethod,
    attested: bool,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let mut session = Session::open(vault_path, pp)?;

    // R-M7b / D3: attestation gate — EARLY, before empty-lots / conservation work.
    // A SafeHarborAllocation permanently records pre2025_method and is irrevocable; require an
    // explicit declared+attested pre-2025 method so the commitment is a deliberate, attested choice
    // rather than a silent FIFO default. Note: the `attested` parameter to this function is
    // `timely_allocation_attested` (§5.02(4)) — a separate attestation; do not conflate the two.
    {
        let cli_cfg = session.config()?;
        if !cli_cfg.pre2025_method_attested {
            let m = match cli_cfg.pre2025_method {
                LotMethod::Fifo => "fifo",
                LotMethod::Lifo => "lifo",
                LotMethod::Hifo => "hifo",
            };
            return Err(CliError::Usage(format!(
                "refusing to record a safe-harbor allocation under an UNDECLARED pre-2025 method \
                 ({m}); a safe-harbor allocation permanently records the method used to reconstruct \
                 your pre-2025 basis. Declare your filed method first: \
                 config --set-pre2025-method {m} --attest-pre2025-method"
            )));
        }
    }

    // Pre-2025 residue + the recorded `pre2025_method` come from the single shared read helper
    // (`Session::safe_harbor_residue`, D3) — the same source the TUI allocate opener uses. It reads
    // config ONCE, so the returned method is STRUCTURALLY the one the residue was projected under.
    let (lots, pre2025_method) = session.safe_harbor_residue()?;
    if lots.is_empty() {
        return Err(CliError::Usage(
            "no pre-2025 lots to allocate (Path A applies; safe harbor unnecessary)".into(),
        ));
    }
    let payload = EventPayload::SafeHarborAllocation(SafeHarborAllocation {
        lots,
        as_of_date: TRANSITION_DATE,
        method,
        timely_allocation_attested: attested,
        // §A.7: the recorded pre-2025 method is the SAME config method the residue above was projected
        // under (both from the one `safe_harbor_residue` config read), so the listed lots conserve
        // against the engine's method-aware snapshot. Immutable thereafter; a later live-config change
        // fires the hard `Pre2025MethodConflictsAllocation` rather than silently breaking conservation.
        pre2025_method,
    });
    append_and_save(&mut session, payload, now)
}

/// §A.4 / SPEC Task 5: emit a `LotSelection` decision for a specific disposal. `disposal_ref` is the
/// canonical `EventId` string of the disposal event; `picks` is at least one `LotPick`. The engine
/// validates completeness (Σsat == disposal principal) and lot existence in the fold; this function
/// only appends the decision — it does NOT attempt to validate up-front (that would require a full
/// projection, which the engine always does). Identification must exist by the time of sale (§1.1012-1(j)).
pub fn select_lots(
    vault_path: &Path,
    pp: &Passphrase,
    disposal_ref: &str,
    picks: Vec<LotPick>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let disposal_event = parse_event_id(disposal_ref)?;
    if picks.is_empty() {
        return Err(CliError::Usage(
            "select-lots needs at least one --from <lot-id>:<sat>".into(),
        ));
    }
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::LotSelection(LotSelection {
            disposal_event,
            lots: picks,
        }),
        now,
    )
}

/// §A.4 / SPEC Task 5: batch-import `LotSelection` decisions from a CSV file.
///
/// CSV format: `disposal_ref,origin_event_id,split_sequence,sat` (header required, validated loudly).
/// Rows sharing a `disposal_ref` are grouped into a single `LotSelection` (one decision per disposal);
/// grouping is by BTreeMap on the canonical disposal_ref string → deterministic order (NFR4).
/// All decisions are written in one session → one `save()`.
pub fn import_selections(
    vault_path: &Path,
    pp: &Passphrase,
    csv_path: &Path,
    now: OffsetDateTime,
) -> Result<Vec<EventId>, CliError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_path)?;
    {
        let hdr = rdr.headers()?;
        let cols: Vec<&str> = hdr.iter().collect();
        if cols != ["disposal_ref", "origin_event_id", "split_sequence", "sat"] {
            return Err(CliError::Usage(format!(
                "import-selections CSV header must be \
                 disposal_ref,origin_event_id,split_sequence,sat; got {cols:?}"
            )));
        }
    }
    // Group picks by disposal_ref; BTreeMap gives deterministic iteration order (NFR4).
    let mut by_disposal: std::collections::BTreeMap<String, Vec<LotPick>> =
        std::collections::BTreeMap::new();
    for rec in rdr.records() {
        let rec = rec?;
        let disposal_ref = rec
            .get(0)
            .ok_or_else(|| CliError::Usage("missing disposal_ref".into()))?
            .to_string();
        let origin_str = rec
            .get(1)
            .ok_or_else(|| CliError::Usage("missing origin_event_id".into()))?;
        let split_str = rec
            .get(2)
            .ok_or_else(|| CliError::Usage("missing split_sequence".into()))?;
        let sat_str = rec
            .get(3)
            .ok_or_else(|| CliError::Usage("missing sat".into()))?;
        let origin_event_id = parse_event_id(origin_str)?;
        let split_sequence = split_str
            .trim()
            .parse::<u32>()
            .map_err(|e| CliError::Usage(format!("bad split_sequence {split_str:?}: {e}")))?;
        let sat = sat_str
            .trim()
            .parse::<i64>()
            .map_err(|e| CliError::Usage(format!("bad sat {sat_str:?}: {e}")))?;
        by_disposal.entry(disposal_ref).or_default().push(LotPick {
            lot: LotId {
                origin_event_id,
                split_sequence,
            },
            sat,
        });
    }
    let mut session = Session::open(vault_path, pp)?;
    let mut ids = Vec::new();
    for (disposal_ref, lots) in by_disposal {
        let disposal_event = parse_event_id(&disposal_ref)?;
        let id = append_decision(
            session.conn(),
            EventPayload::LotSelection(LotSelection {
                disposal_event,
                lots,
            }),
            now,
            UtcOffset::UTC,
            None,
        )?;
        ids.push(id);
    }
    session.save()?;
    Ok(ids)
}

/// M3 / SPEC A.1: append a `MethodElection` decision — the forward standing order.
///
/// This is an EVENT, not a config flag mutation. The standing order is irrevocable (unless voided)
/// and governs all method-honoring disposals on/after `effective_from`. Back-dating is blocked by
/// the engine (`MethodElectionBackdated` hard blocker when `effective_from < made-date`).
///
/// When `effective_from` is `None`, defaults to the decision's made-date (`now` in UTC), which
/// satisfies the `effective_from >= made-date` invariant by construction.
pub fn set_forward_method(
    vault_path: &Path,
    pp: &Passphrase,
    m: LotMethod,
    effective_from: Option<TaxDate>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let effective_from = effective_from.unwrap_or_else(|| now.to_offset(UtcOffset::UTC).date());
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::MethodElection(MethodElection {
            effective_from,
            method: m,
        }),
        now,
    )
}

/// FR7: attest an existing allocation. Events are immutable, so attestation = void the single live prior
/// allocation and re-append it with `timely_allocation_attested = true`. Attestation only cures a
/// §5.02(4) TIME-BAR; it is NOT valid on an already-effective allocation (which needs nothing) nor on one
/// that fails CONSERVATION (which needs a corrected allocation, not an attestation).
///
/// Uses `Session::load_events_and_project` to load the event log exactly once — the old pattern
/// (separate `load_all(session.conn())` + `session.project()`) loaded the same DB rows twice.
pub fn safe_harbor_attest(
    vault_path: &Path,
    pp: &Passphrase,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;

    // Eng-I1 / I-2(a): EXCLUDE voided allocations from the single-allocation guard, so the legitimate
    // allocate→inert→void→re-allocate→attest workflow (which leaves an OLD, voided allocation in the log)
    // is not blocked by "multiple allocations present." Build the voided-target set from `VoidDecisionEvent`s
    // (mirrors resolve.rs pass-1 step 1a) and keep only LIVE (non-voided) allocations.
    let voided: std::collections::BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect();
    let allocs: Vec<(&EventId, &SafeHarborAllocation)> = events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => Some((&e.id, a)),
            _ => None,
        })
        .collect();
    let (prior_id, prior) = match allocs.as_slice() {
        [one] => (one.0.clone(), one.1.clone()),
        [] => {
            return Err(CliError::Usage(
                "no allocation to attest; run `safe-harbor allocate` first".into(),
            ))
        }
        _ => {
            return Err(CliError::Usage(
                "multiple live allocations present; void the stale one before attesting".into(),
            ))
        }
    };
    if prior.timely_allocation_attested {
        return Err(CliError::Usage("allocation is already attested".into()));
    }

    // I-2(b) / N-2: classify the prior allocation's CURRENT status via the projection loaded above
    // (`load_events_and_project`), reading the engine's own effectiveness verdict (the blockers it
    // stamps onto `prior_id`):
    //   * `SafeHarborUnconservable` (hard) → attestation CANNOT cure it (only a corrected allocation can).
    //   * `SafeHarborTimebar` (advisory)   → inert PURELY because of the §5.02(4) bar → attestation cures it.
    //   * neither                          → ALREADY EFFECTIVE → attesting would Void an effective allocation
    //     (→ irrevocable `decision_conflicts`, §7.4) AND append a second effective allocation (→ two effective
    //     → Path A, irrecoverable). Refuse and advise `verify` (NOT "void the effective one").
    let blocked_with = |k: BlockerKind| {
        state
            .blockers
            .iter()
            .any(|b| b.event.as_ref() == Some(&prior_id) && b.kind == k)
    };
    let unconservable = blocked_with(BlockerKind::SafeHarborUnconservable);
    let timebarred = blocked_with(BlockerKind::SafeHarborTimebar);
    // (closure's borrow of `prior_id` ends here, so the move into the Void below is sound.)
    if unconservable {
        return Err(CliError::Usage(
            "allocation fails conservation (not a time-bar); re-run `safe-harbor allocate` to rebuild it — attestation cannot cure conservation".into(),
        ));
    }
    if !timebarred {
        return Err(CliError::Usage(
            "allocation already effective; no attestation needed — run `verify`".into(),
        ));
    }

    // Inert PURELY due to a time-bar → attestation cures it. Append Void(prior) + a re-attested copy.
    // (N2: same `now` for both; `decision_seq` orders/distinguishes them — Void first, then re-attest.)
    append_decision(
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
        ..prior
    };
    let id = append_decision(
        session.conn(),
        EventPayload::SafeHarborAllocation(attested),
        now,
        UtcOffset::UTC,
        None,
    )?;
    session.save()?;
    Ok(id)
}

/// SE-completion Chunk C (D3): flip `business` (and optionally `kind`) on an already-imported
/// `Income` event. Enables SE-tax treatment for professional miners / stakers whose River (and
/// other adapter) income arrives with `business: false` hard-coded at ingest time.
///
/// The engine validates the target at collection time: if the referenced event does not exist OR its
/// effective payload is not `Income`, a Hard `DecisionConflict` blocker fires and the decision is
/// excluded (not silently inert, not a panic). To correct a `TransferIn` row use `classify-inbound-income`
/// instead. **DecisionConflict is Hard — to re-decide, `void` the prior decision then re-issue.**
pub fn reclassify_income(
    vault_path: &Path,
    pp: &Passphrase,
    income_ref: &str,
    business: bool,
    kind: Option<IncomeKind>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let income_event = parse_event_id(income_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ReclassifyIncome(ReclassifyIncome {
        income_event,
        business,
        kind,
    });
    append_and_save(&mut session, payload, now)
}

/// Chunk 3b D2: store Form 8283 Section-B donation + appraiser details in the
/// `donation_details` side-table for the donation identified by `event_ref`.
///
/// **[R0-M] Projected-removals validation:** the `event_ref` must resolve to a
/// `Removal { kind == Donation }` in the PROJECTED `state.removals` — NOT by scanning the raw
/// event log. A ref to a non-donation removal (Gift) or to an event that produces no removal
/// (e.g. an Acquire) → `CliError::Usage` with a clear message. No decision is appended; this
/// is a side-table write (last-write-wins upsert, like `tax_profile::set`).
pub fn set_donation_details(
    vault_path: &Path,
    pp: &Passphrase,
    event_ref: &str,
    details: DonationDetails,
) -> Result<(), CliError> {
    let event_id = parse_event_id(event_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;

    // [R0-M] Validate against the PROJECTED state.removals.
    let matched_removal = state.removals.iter().find(|r| r.event == event_id);
    match matched_removal {
        None => {
            return Err(CliError::Usage(format!(
                "not a donation / not found: {event_ref:?} does not match any removal in the \
                 projected ledger (check removals.csv 'event' column for the correct ref)"
            )));
        }
        Some(r) if r.kind != RemovalKind::Donation => {
            return Err(CliError::Usage(format!(
                "not a donation: {event_ref:?} is a {:?} removal, not a Donation",
                r.kind
            )));
        }
        Some(_) => {} // confirmed Donation — proceed
    }

    crate::donation_details::set(session.conn(), &event_id, &details)?;
    session.save()?;
    Ok(())
}

/// Chunk 3b D2: read back stored `DonationDetails` for the donation identified by `event_ref`.
/// Returns `None` when no details have been stored yet. Read-only — no projection needed.
pub fn show_donation_details(
    vault_path: &Path,
    pp: &Passphrase,
    event_ref: &str,
) -> Result<Option<DonationDetails>, CliError> {
    let event_id = parse_event_id(event_ref)?;
    let session = Session::open(vault_path, pp)?;
    crate::donation_details::get(session.conn(), &event_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::event::{Acquire, BasisSource, TransferOut};
    use btctax_core::persistence::append_import_batch;
    use btctax_core::{
        EventId, LedgerEvent, OutflowClass, ReclassifyOutflow, Source, SourceRef, WalletId,
    };
    use btctax_store::Passphrase;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn pp() -> Passphrase {
        Passphrase::new("test-pass".into())
    }

    /// Build a minimal `DonationDetails` for tests (synthetic — no real PII).
    fn test_details() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: None,
            donee_ein: Some("12-3456789".into()),
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: Some("987654321".into()),
            appraiser_ptin: None,
            appraiser_qualifications: Some("Certified bitcoin appraiser".into()),
            appraisal_date: Some(date!(2025 - 06 - 01)),
            fmv_method_override: None,
        }
    }

    /// Create a vault + Acquire + TransferOut + ReclassifyOutflow(Donate). Returns
    /// (vault_path, donation_event_id, acquire_event_id) where donation_event_id is the
    /// TransferOut EventId (which becomes the Removal.event in the projected ledger).
    /// PRIVACY: synthetic values only.
    fn setup_donation_vault(dir: &tempfile::TempDir) -> (std::path::PathBuf, EventId, EventId) {
        use crate::Session;

        let vault_path = dir.path().join("vault.pgp");
        let mut session = Session::create(&vault_path, &pp()).unwrap();

        // Fixed timestamps (deterministic, reproducible). Both pre-2025 (< 2025-01-01 =
        // Unix 1_735_689_600) to stay in the Universal-pool path (no per-wallet allocation needed).
        // ts_acq ≈ 2023-11-14, ts_out ≈ 2024-07-03.
        let ts_acq = time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let ts_out = time::OffsetDateTime::from_unix_timestamp(1_720_000_000).unwrap();

        // Both events use the same exchange wallet so lots can be consumed from the same pool.
        let wallet = WalletId::Exchange {
            provider: "coinbase".into(),
            account: "default".into(),
        };

        // Acquire 2_000_000 sats at $60,000 cost basis.
        let acq_id = EventId::import(Source::Coinbase, SourceRef::new("in|test-acq-001"));
        let acq_ev = LedgerEvent {
            id: acq_id.clone(),
            utc_timestamp: ts_acq,
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 2_000_000,
                usd_cost: dec!(60000),
                fee_usd: dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        };
        append_import_batch(session.conn(), &[acq_ev]).unwrap();

        // TransferOut 500_000 sats from the same wallet.
        let out_id = EventId::import(Source::Coinbase, SourceRef::new("out|test-donation-001"));
        let out_ev = LedgerEvent {
            id: out_id.clone(),
            utc_timestamp: ts_out,
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet.clone()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 500_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        };
        append_import_batch(session.conn(), &[out_ev]).unwrap();

        // ReclassifyOutflow as Donation with explicit FMV (no price lookup needed).
        let classify_payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: out_id.clone(),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(15000),
            fee_usd: None,
            donee: Some("Test Charity".into()),
        });
        append_decision(
            session.conn(),
            classify_payload,
            ts_out,
            UtcOffset::UTC,
            None,
        )
        .unwrap();

        session.save().unwrap();
        (vault_path, out_id, acq_id)
    }

    /// `set_donation_details` on a real Donation event stores it;
    /// `show_donation_details` reads it back correctly.
    #[test]
    fn set_then_show_round_trips_on_real_donation() {
        let dir = tempfile::tempdir().unwrap();
        let (vault_path, out_id, _acq_id) = setup_donation_vault(&dir);

        // Store details.
        set_donation_details(&vault_path, &pp(), &out_id.canonical(), test_details()).unwrap();

        // Read back.
        let stored = show_donation_details(&vault_path, &pp(), &out_id.canonical())
            .unwrap()
            .expect("details must be present");
        assert_eq!(stored, test_details());
    }

    /// Targeting a missing ref → a clear `CliError::Usage` (not a panic).
    #[test]
    fn set_donation_details_missing_ref_is_usage_error() {
        let dir = tempfile::tempdir().unwrap();
        let (vault_path, _out_id, _acq_id) = setup_donation_vault(&dir);

        let bogus = EventId::import(Source::Coinbase, SourceRef::new("out|no-such-event"));
        let err = set_donation_details(&vault_path, &pp(), &bogus.canonical(), test_details())
            .unwrap_err();
        assert!(
            matches!(err, CliError::Usage(_)),
            "expected Usage error, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("not a donation") || msg.contains("not found"),
            "error must mention 'not a donation' or 'not found': {msg}"
        );
    }

    /// Targeting the Acquire event (not a Donation removal) → a clear `CliError::Usage`.
    #[test]
    fn set_donation_details_non_donation_event_is_usage_error() {
        let dir = tempfile::tempdir().unwrap();
        let (vault_path, _out_id, acq_id) = setup_donation_vault(&dir);

        // The Acquire event is not a Donation removal in the projected ledger.
        let err = set_donation_details(&vault_path, &pp(), &acq_id.canonical(), test_details())
            .unwrap_err();
        assert!(
            matches!(err, CliError::Usage(_)),
            "expected Usage error, got: {err}"
        );
    }

    /// `show_donation_details` returns `None` before any details are stored.
    #[test]
    fn show_donation_details_returns_none_before_set() {
        let dir = tempfile::tempdir().unwrap();
        let (vault_path, out_id, _acq_id) = setup_donation_vault(&dir);

        let stored = show_donation_details(&vault_path, &pp(), &out_id.canonical()).unwrap();
        assert_eq!(stored, None);
    }

    /// Build a vault with a Gift removal (not a Donation). Returns (vault_path, gift_event_id).
    fn setup_gift_vault(dir: &tempfile::TempDir) -> (std::path::PathBuf, EventId) {
        use crate::Session;
        let vault_path = dir.path().join("gift-vault.pgp");
        let mut session = Session::create(&vault_path, &pp()).unwrap();

        let ts_acq = time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let ts_out = time::OffsetDateTime::from_unix_timestamp(1_720_000_000).unwrap();
        let wallet = WalletId::Exchange {
            provider: "coinbase".into(),
            account: "default".into(),
        };

        let acq_id = EventId::import(Source::Coinbase, SourceRef::new("in|gift-acq-001"));
        let acq_ev = LedgerEvent {
            id: acq_id.clone(),
            utc_timestamp: ts_acq,
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 2_000_000,
                usd_cost: dec!(60000),
                fee_usd: dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        };
        append_import_batch(session.conn(), &[acq_ev]).unwrap();

        let out_id = EventId::import(Source::Coinbase, SourceRef::new("out|gift-001"));
        let out_ev = LedgerEvent {
            id: out_id.clone(),
            utc_timestamp: ts_out,
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet.clone()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 500_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        };
        append_import_batch(session.conn(), &[out_ev]).unwrap();

        // Reclassify as GiftOut (NOT Donate)
        let classify_payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: out_id.clone(),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(15000),
            fee_usd: None,
            donee: None,
        });
        append_decision(
            session.conn(),
            classify_payload,
            ts_out,
            UtcOffset::UTC,
            None,
        )
        .unwrap();

        session.save().unwrap();
        (vault_path, out_id)
    }

    /// `set_donation_details` targeting a Gift removal → "is a Gift removal, not a Donation" error.
    /// Exercises the `Some(r) if r.kind != RemovalKind::Donation` arm (previously untested).
    #[test]
    fn set_donation_details_gift_removal_is_usage_error() {
        let dir = tempfile::tempdir().unwrap();
        let (vault_path, gift_out_id) = setup_gift_vault(&dir);

        let err =
            set_donation_details(&vault_path, &pp(), &gift_out_id.canonical(), test_details())
                .unwrap_err();
        assert!(
            matches!(err, CliError::Usage(_)),
            "expected Usage error, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("not a donation") || msg.contains("Gift"),
            "error must mention 'not a donation' or 'Gift': {msg}"
        );
    }
}
