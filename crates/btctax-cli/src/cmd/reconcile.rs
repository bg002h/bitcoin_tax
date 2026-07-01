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
    EventPayload, InboundClass, LedgerEvent, LotId, LotMethod, LotPick, LotSelection, ManualFmv,
    MethodElection, OutflowClass, ReclassifyOutflow, RejectImport, SafeHarborAllocation,
    SupersedeImport, TaxDate, TransferLink, TransferTarget, Usd, VoidDecisionEvent,
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
            dual_loss_basis: l.dual_loss_basis,
            donor_acquired_at: l.donor_acquired_at,
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
        // §A.7: capture the live attested pre-2025 method at attestation time. The residue above was
        // projected under this SAME `cfg.pre2025_method`, so the listed lots conserve against the engine's
        // method-aware snapshot. Immutable thereafter; a later live-config change fires the hard
        // `Pre2025MethodConflictsAllocation` rather than silently breaking conservation.
        pre2025_method: cfg.pre2025_method,
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
