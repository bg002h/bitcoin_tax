//! `optimize run` (Task 9) — §C.2 Mode-1 what-if proposal. READ-ONLY: opens the vault, projects,
//! optimizes, and returns the proposal. Appends / persists NOTHING.
use crate::{CliError, Session};
use btctax_adapters::{BundledPrices, BundledTaxTables};
use btctax_core::conventions::tax_date;
use btctax_core::{optimize_year, OptimizeError, OptimizeProposal};
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
    let prices = BundledPrices::load()?;
    let tables = BundledTaxTables::load();
    let attested = s.optimize_attested_set()?;
    let proposal_made = tax_date(now, UtcOffset::UTC); // R0-C2: real made-date threaded into core
    let p = optimize_year(
        &events,
        &prices,
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
        OptimizeError::Evaluate(ev) => CliError::Usage(format!("evaluate error: {ev:?}")),
    }
}
