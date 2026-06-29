//! reconcile decision emitters (FR6/FR7/FR8, §7.2). Each fn builds exactly ONE `EventPayload` decision
//! variant and appends it via `append_decision` (monotonic `decision_seq`), then saves. Decisions are
//! append-only and re-projectable; the engine resolves precedence (latest-`decision_seq`, Void-first).
//! `now` is the injected decision creation-time / safe-harbor made-date (§6.2) — deterministic in tests.
use crate::{CliError, Session};
use btctax_adapters::BundledPrices;
use btctax_core::conventions::{tax_date, TRANSITION_DATE};
use btctax_core::persistence::{append_decision, load_all};
use btctax_core::{
    project, AllocLot, AllocMethod, BlockerKind, ClassifyInbound, ClassifyRaw, EventId,
    EventPayload, InboundClass, LedgerEvent, ManualFmv, OutflowClass, ReclassifyOutflow,
    RejectImport, SafeHarborAllocation, SupersedeImport, TransferLink, TransferTarget, Usd,
    VoidDecisionEvent,
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
pub fn reclassify_outflow(
    vault_path: &Path,
    pp: &Passphrase,
    out_ref: &str,
    class: OutflowClass,
    principal: Usd,
    fee_usd: Option<Usd>,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let transfer_out_event = parse_event_id(out_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    let payload = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
        transfer_out_event,
        as_: class,
        principal_proceeds_or_fmv: principal,
        fee_usd,
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
pub fn void(
    vault_path: &Path,
    pp: &Passphrase,
    target_ref: &str,
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let target_event_id = parse_event_id(target_ref)?;
    let mut session = Session::open(vault_path, pp)?;
    append_and_save(
        &mut session,
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id }),
        now,
    )
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

    // Pre-2025-only event subset (see the I-1 note above).
    let pre2025: Vec<LedgerEvent> = load_all(session.conn())?
        .into_iter()
        .filter(|e| match &e.id {
            EventId::Import { .. } => tax_date(e.utc_timestamp, e.original_tz) < TRANSITION_DATE,
            _ => !matches!(e.payload, EventPayload::SafeHarborAllocation(_)),
        })
        .collect();
    let cfg = session.config()?.to_projection();
    let prices = BundledPrices::load()?;
    let residue = project(&pre2025, &prices, &cfg); // == the 2025-01-01 Universal residue

    let lots: Vec<AllocLot> = residue
        .lots
        .iter()
        .filter(|l| l.remaining_sat > 0)
        .map(|l| AllocLot {
            wallet: l.wallet.clone(),
            sat: l.remaining_sat,
            usd_basis: l.usd_basis,
            acquired_at: l.acquired_at,
        })
        .collect();
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
    });
    append_and_save(&mut session, payload, now)
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
