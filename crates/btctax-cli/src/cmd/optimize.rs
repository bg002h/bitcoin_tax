//! `optimize run` (Task 9) — §C.2 Mode-1 what-if proposal. READ-ONLY: opens the vault, projects,
//! optimizes, and returns the proposal. Appends / persists NOTHING.
//!
//! `optimize accept` (Task 10) — §C.2 gated persistence. The ONLY Mode-1 path that writes. It
//! RECOMPUTES the same deterministic optimum (never trusts a stale proposal — NFR4), then for each
//! disposal applies the §1.1012-1(j) gate: persist the proposed `LotSelection` ONLY when it is
//! genuinely contemporaneous (made ≤ sale) OR — for an already-executed disposal within the own-books
//! envelope — behind a NARROW per-disposal `--attest`; a 2027+ broker-held pick is CATEGORICALLY
//! refused (own-books is insufficient; no attestation can cure it). When attested, the proposed
//! `LotSelection` decision AND the attestation side-table row are co-persisted ATOMICALLY (both land
//! in the same in-memory DB and are flushed by a SINGLE `session.save()`), so the persisted selection
//! == the attested selection == the new baseline (closes the Task-8 operational note; R2-I1 holds on a
//! later re-run). Revocation reuses the existing `reconcile void` on the returned decision id.
//!
//! `optimize consult` (Task 11) — §C.3 Mode-2 read-only pre-trade what-if. Opens the vault,
//! projects, calls `consult_sale`, returns a `ConsultReport`. READ-ONLY: appends NOTHING, no
//! decision, no side-table write (Mode-2 produces nothing). Tax decision-support (consequences),
//! NOT buy/sell advice.
use crate::{CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::conventions::tax_date;
use btctax_core::persistence::append_decision;
use btctax_core::{
    consult_sale, optimize_year, ConsultReport, ConsultRequest, DisposeKind, EvaluateError,
    EventId, EventPayload, LotPick, LotSelection, OptimizeError, OptimizeProposal, Persistability,
    TaxDate, TaxTables, Usd, WalletId,
};
use btctax_store::Passphrase;
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};

/// `optimize run` — Mode 1 what-if. READ-ONLY: opens the vault, projects, optimizes, returns the
/// proposal. Appends/persists NOTHING. `now` is the CLI clock seam → the proposed picks' made-date
/// (R0-C2: core stays clock-free; the proposal the user reads is judged against the REAL made-date).
pub fn run(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    now: OffsetDateTime,
) -> Result<OptimizeProposal, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, _state, cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(year)?;
    let prices = s.prices();
    let tables = BundledTaxTables::load();
    let attested = s.optimize_attested_set()?;
    let proposal_made = tax_date(now, UtcOffset::UTC); // R0-C2: real made-date threaded into core
    let p = optimize_year(
        &events,
        prices,
        &cfg,
        year,
        profile.as_ref(),
        &tables,
        &attested,
        proposal_made,
    )
    .map_err(map_opt_err)?;
    // R0-C1: core has no logger — log the cap/why HERE (CLI seam) when the result is approximate.
    if p.approximate {
        eprintln!(
            "warning: optimize result is APPROXIMATE (not a guaranteed global minimum): {:?}",
            p.approx_reason
        );
    }
    Ok(p)
}

pub(crate) fn map_opt_err(e: OptimizeError) -> CliError {
    match e {
        OptimizeError::YearNotComputable(b) => CliError::Usage(format!(
            "year not computable — resolve the blocker first: [{:?}] {}",
            b.kind, b.detail
        )),
        OptimizeError::PreTransitionYear(y) => CliError::Usage(format!(
            "{y} is pre-2025: a pre-2025 selection restates a closed year — not an optimization (M7)"
        )),
        OptimizeError::NoDisposals => {
            CliError::Usage("no method-honoring disposals in that year".into())
        }
        OptimizeError::NoLots => CliError::Usage("no lots available to sell".into()),
        OptimizeError::Evaluate(EvaluateError::ProceedsRequired) => CliError::Usage(
            "--proceeds <usd> is required for a date with no bundled dataset price \
             (--fmv alone cannot resolve proceeds for a future or off-dataset date)"
                .into(),
        ),
        OptimizeError::Evaluate(ev) => CliError::Usage(format!("evaluate error: {ev:?}")),
    }
}

/// `optimize consult` — §C.3 Mode-2 READ-ONLY pre-trade what-if.
///
/// Opens the vault, runs the pure deterministic projection, and calls `consult_sale` with the
/// synthetic `ConsultRequest`. Returns a `ConsultReport` with the tax-minimizing lot selection,
/// the resulting ST/LT split, the federal tax attributable to the hypothetical sale, and — when
/// present — the ST→LT timing insight (crossover + saving). **Appends NOTHING, writes NOTHING,
/// calls no `session.save()`.** The result is tax decision-support (consequences of a contemplated
/// sale), NOT buy/sell/hold advice (§C.2 scope invariant).
pub fn consult(
    vault: &Path,
    pp: &Passphrase,
    sell_sat: i64,
    wallet: WalletId,
    at: TaxDate,
    proceeds: Option<Usd>,
    kind: DisposeKind,
) -> Result<ConsultReport, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, _state, cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(at.year())?;
    let prices = s.prices();
    let tables = BundledTaxTables::load();
    let req = ConsultRequest {
        sell_sat,
        wallet,
        at,
        proceeds,
        kind,
    };
    // consult_sale is READ-ONLY (clone-fold-discard on every call); no save() is ever called.
    consult_sale(&events, prices, &cfg, profile.as_ref(), &tables, &req).map_err(map_opt_err)
}

/// The result of `optimize accept` — what was persisted vs skipped (for rendering). `persisted` carries
/// `(disposal, decision, basis)`: the disposal whose pick was adopted, the appended `LotSelection`
/// decision id (pass it to `reconcile void` to revoke), and the §A.5 basis label
/// (`"Contemporaneous"` / `"AttestedRecording"`). `skipped` carries `(disposal, reason)`.
#[derive(Debug)]
pub struct AcceptOutcome {
    pub persisted: Vec<(EventId, EventId, &'static str)>,
    pub skipped: Vec<(EventId, String)>,
}

/// `optimize accept` — apply the recomputed optimum, gated per disposal (§C.2 / §1.1012-1(j)).
///
/// `only`: if `Some(disposal)`, restrict to that one disposal (the form that carries `--attest`).
/// `attestation`: the user's narrow contemporaneous-ID statement, REQUIRED to persist an
/// already-executed disposal; the app NEVER fabricates it and refuses to persist a post-hoc selection
/// without it. A bare `accept` (no `--attest`) persists only genuinely-contemporaneous picks; it
/// remains what-if for already-executed ones. `now` is the CLI clock seam → the proposed picks'
/// made-date (core stays clock-free; the persisted decision is judged against the REAL made-date).
pub fn accept(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    only: Option<&str>,
    attestation: Option<&str>,
    now: OffsetDateTime,
) -> Result<AcceptOutcome, CliError> {
    accept_with_tables(
        vault,
        pp,
        year,
        only,
        attestation,
        now,
        &BundledTaxTables::load(),
    )
}

/// Implementation seam for [`accept`] with injectable tax tables. The public `accept` uses the bundled
/// tables (TY2024 and TY2025); tests inject a table for a later year to exercise the 2027+ broker
/// refusal end-to-end (a 2027 disposal is otherwise `YearNotComputable` under the bundled-tables-only
/// path). Not part of the stable surface.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn accept_with_tables(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    only: Option<&str>,
    attestation: Option<&str>,
    now: OffsetDateTime,
    tables: &dyn TaxTables,
) -> Result<AcceptOutcome, CliError> {
    let mut session = Session::open(vault, pp)?;
    let (events, _state, cfg) = session.load_events_and_project()?;
    let profile = session.tax_profile(year)?;
    let prices = session.prices();
    let attested = session.optimize_attested_set()?;
    let made = tax_date(now, UtcOffset::UTC); // the LotSelection's made-date (decisions are UTC)
    let only_id = only.map(crate::eventref::parse_event_id).transpose()?;

    // R2-M5/R0-M5: validate the --attest/--disposal precondition BEFORE recomputing or appending
    // ANYTHING — `--attest` requires a single `--disposal` scope (the app never invites a blanket false
    // attestation across all disposals). Hoisting this guard ABOVE the loop guarantees no disposal is
    // appended before it fires (no partial/abandoned writes on the rejected path).
    if attestation.is_some() && only_id.is_none() {
        return Err(CliError::Usage(
            "--attest must be scoped to ONE disposal via --disposal (no blanket attestation)"
                .into(),
        ));
    }

    // RECOMPUTE the same deterministic optimum (NFR4) — never trust a stale proposal. R0-C2: judge the
    // proposal against the REAL made-date (`made`) so `run` and `accept` agree on persistability.
    let proposal = optimize_year(
        &events,
        prices,
        &cfg,
        year,
        profile.as_ref(),
        tables,
        &attested,
        made,
    )
    .map_err(map_opt_err)?;

    let mut out = AcceptOutcome {
        persisted: vec![],
        skipped: vec![],
    };
    for d in &proposal.per_disposal {
        if let Some(target) = &only_id {
            if &d.disposal != target {
                continue;
            }
        }
        // Nothing to persist if the proposed selection equals the current one (no-change row).
        if d.proposed_selection == d.current_selection {
            out.skipped.push((
                d.disposal.clone(),
                "already optimal under current identification".into(),
            ));
            continue;
        }
        // The §C.2 gate. `d.persistable` was computed by `optimize_year` against the SAME `made`
        // (== `persistability(&d.wallet, d.date, made)`), so it is the per-disposal verdict here.
        match d.persistable {
            Persistability::ForbiddenBroker2027 => {
                // NEVER persist — own-books is insufficient for 2027+ broker-held units, and no
                // attestation can cure it (categorical refusal; FIFO is the defensible position).
                out.skipped.push((
                    d.disposal.clone(),
                    "2027+ broker-held: own-books is insufficient; cannot persist \
                     (FIFO is the defensible position)"
                        .into(),
                ));
            }
            Persistability::ContemporaneousNow => {
                // Made ≤ sale → genuinely contemporaneous; persist freely (no attestation needed).
                let id = persist_selection(&mut session, &d.disposal, &d.proposed_selection, now)?;
                out.persisted
                    .push((d.disposal.clone(), id, "Contemporaneous"));
            }
            Persistability::NeedsAttestation => {
                // Already executed within the own-books envelope: refuse WITHOUT a narrow per-disposal
                // attestation (the app NEVER auto-attests a post-hoc selection).
                let Some(att) = attestation else {
                    out.skipped.push((
                        d.disposal.clone(),
                        "already executed — re-run \
                         `optimize accept --disposal <ref> --attest \"<genuine contemporaneous ID>\"`"
                            .into(),
                    ));
                    continue;
                };
                // Blanket-attest is already rejected up-front (above the loop), so here
                // `only_id == Some(d.disposal)` — no append-before-guard can occur on any error path.
                // Co-persist the LotSelection decision AND the attestation row ATOMICALLY: both land in
                // the same in-memory DB and are flushed together by the single `session.save()` below,
                // so the persisted selection == the attested selection == the new baseline.
                let id = persist_selection(&mut session, &d.disposal, &d.proposed_selection, now)?;
                crate::optimize_attest::set(session.conn(), &d.disposal, att, &made.to_string())?;
                out.persisted
                    .push((d.disposal.clone(), id, "AttestedRecording"));
            }
        }
    }
    session.save()?;
    Ok(out)
}

/// Append the `LotSelection` decision for one disposal (no save; the caller batches the single save so
/// the decision + any attestation row are flushed atomically). `fingerprint = None`, consistent with
/// all decisions (`append_decision` passes `None`).
fn persist_selection(
    session: &mut Session,
    disposal: &EventId,
    picks: &[LotPick],
    now: OffsetDateTime,
) -> Result<EventId, CliError> {
    let payload = EventPayload::LotSelection(LotSelection {
        disposal_event: disposal.clone(),
        lots: picks.to_vec(),
    });
    Ok(append_decision(
        session.conn(),
        payload,
        now,
        UtcOffset::UTC,
        None,
    )?)
}
