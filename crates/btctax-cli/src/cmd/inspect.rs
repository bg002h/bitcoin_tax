//! `verify` (FR9) + `report`/`show` (FR4) — read-only inspection of the pure projection. `verify`
//! arrives in Task 6; this file starts with `report`.
use crate::render::{build_verify, EventRow, VerifyReport};
use crate::{CliError, Session};
use btctax_core::LedgerState;
use btctax_store::Passphrase;
use std::path::Path;

/// UX-P4-11: enumerate every DECIDABLE event (the imported rows a `reconcile` verb can act on) with
/// its ref, kind, date, amount, and decision status, in event-sequence (insertion) order. Read-only.
///
/// The decidable universe = imported `TransferIn` / `TransferOut` / `Unclassified` / `ImportConflict`
/// / `Income` rows (an `Acquire`/`Dispose` is a fully-determined import — no reconcile verb retargets
/// it). Decision status is derived ONLY from PERSISTED decisions (reverse-mapped from the raw log,
/// minus voided ones): a pseudo-defaulted event mints no persisted decision, so it correctly lists as
/// **decidable**. Ordering is the raw insertion order (`load_all_ordered`) — "event sequence" (§3.6).
pub fn events_list(vault_path: &Path, pp: &Passphrase) -> Result<Vec<EventRow>, CliError> {
    use btctax_core::conventions::tax_date;
    use btctax_core::persistence::{load_all, load_all_ordered};
    use btctax_core::{EventId, EventPayload, LedgerEvent, Sat};
    use std::collections::{BTreeSet, HashMap};

    let session = Session::open(vault_path, pp)?;
    let conn = session.conn();
    let prices = session.prices();
    let decoded = load_all(conn)?;
    let ordered = load_all_ordered(conn)?; // insertion order = event sequence

    // Decoded events keyed by canonical id (for O(1) lookup while walking the ordered raw log).
    let by_id: HashMap<String, &LedgerEvent> =
        decoded.iter().map(|e| (e.id.canonical(), e)).collect();

    // A voided decision does not count as "decided" — collect the voided decision ids first.
    let voided: BTreeSet<EventId> = decoded
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::VoidDecisionEvent(v) => Some(v.target_event_id.clone()),
            _ => None,
        })
        .collect();

    // Reverse-map: decidable SOURCE event id -> its live (non-voided) decision id. Each decision
    // payload names its target; `SelfTransferPassthrough` decides BOTH legs. Later live decisions
    // win (a void→re-decide leaves only the survivor here).
    let mut decided: HashMap<EventId, EventId> = HashMap::new();
    for e in &decoded {
        if voided.contains(&e.id) {
            continue;
        }
        match &e.payload {
            EventPayload::ClassifyInbound(d) => {
                decided.insert(d.transfer_in_event.clone(), e.id.clone());
            }
            EventPayload::ReclassifyOutflow(d) => {
                decided.insert(d.transfer_out_event.clone(), e.id.clone());
            }
            EventPayload::ManualFmv(d) => {
                decided.insert(d.event.clone(), e.id.clone());
            }
            EventPayload::ReclassifyIncome(d) => {
                decided.insert(d.income_event.clone(), e.id.clone());
            }
            EventPayload::ClassifyRaw(d) => {
                decided.insert(d.target.clone(), e.id.clone());
            }
            EventPayload::SupersedeImport(d) => {
                decided.insert(d.conflict_event.clone(), e.id.clone());
            }
            EventPayload::RejectImport(d) => {
                decided.insert(d.conflict_event.clone(), e.id.clone());
            }
            EventPayload::TransferLink(d) => {
                decided.insert(d.out_event.clone(), e.id.clone());
            }
            EventPayload::SelfTransferPassthrough(d) => {
                decided.insert(d.in_event.clone(), e.id.clone());
                decided.insert(d.out_event.clone(), e.id.clone());
            }
            _ => {}
        }
    }

    let mut rows = Vec::new();
    for raw in &ordered {
        let Some(ev) = by_id.get(&raw.event_id) else {
            continue;
        };
        let (kind, sat): (&'static str, Option<Sat>) = match &ev.payload {
            EventPayload::TransferIn(ti) => ("transfer-in", Some(ti.sat)),
            EventPayload::TransferOut(to) => ("transfer-out", Some(to.sat)),
            EventPayload::Unclassified(_) => ("unclassified", None),
            EventPayload::ImportConflict(_) => ("import-conflict", None),
            EventPayload::Income(inc) => ("income", Some(inc.sat)),
            // Acquire / Dispose (determined imports), decisions, and other system events are not rows.
            _ => continue,
        };
        let date = tax_date(ev.utc_timestamp, ev.original_tz);
        let usd = match &ev.payload {
            EventPayload::Income(inc) => inc
                .usd_fmv
                .or_else(|| sat.and_then(|s| btctax_core::price::fmv_of(prices, date, s))),
            _ => sat.and_then(|s| btctax_core::price::fmv_of(prices, date, s)),
        };
        rows.push(EventRow {
            reff: ev.id.canonical(),
            kind,
            date,
            sat,
            usd,
            decision_ref: decided.get(&ev.id).map(|d| d.canonical()),
        });
    }
    Ok(rows)
}

/// FR4: project the ledger for display. `year` filters realized sections in the renderer; holdings are
/// always the current per-lot position.
pub fn report(
    vault_path: &Path,
    pp: &Passphrase,
    _year: Option<i32>,
) -> Result<LedgerState, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;
    Ok(state)
}

/// FR9: project the ledger → compute the sat-conservation report, partition blockers by severity, and
/// summarize pending reconciliation + safe-harbor status. The binary maps `has_hard_blockers()` to a
/// non-zero exit (a hard blocker gates downstream tax computation, §7.1).
///
/// Uses `Session::load_events_and_project` to load the event log exactly once (avoiding the
/// double `load_all` that the old `project()` + separate `load_all(conn)` pattern incurred).
/// Task 8: also reads the CLI config (declared `pre2025_method` + attestation flag) and passes
/// it to `build_verify` so that `render_verify` can surface them.
pub fn verify(vault_path: &Path, pp: &Passphrase) -> Result<VerifyReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;
    let cli = session.config()?;
    Ok(build_verify(&state, &events, &cli))
}
