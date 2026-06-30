//! `tax-profile` command helpers — set/show the per-year `TaxProfile` side-table entry.
//! `report_tax_year` (Task 9) provides the standalone "tax owed / what-if" calculator.
use crate::{tax_profile, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{compute_tax_year, TaxOutcome, TaxProfile};
use btctax_store::Passphrase;
use std::path::Path;

/// Persist `p` as the tax profile for `year` in the vault at `vault`, then save.
pub fn set_profile(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    p: TaxProfile,
) -> Result<(), CliError> {
    let mut s = Session::open(vault, pp)?;
    tax_profile::set(s.conn(), year, &p)?;
    s.save()
}

/// Return the stored `TaxProfile` for `year` from the vault at `vault`, or `None`.
pub fn show_profile(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<Option<TaxProfile>, CliError> {
    tax_profile::get(Session::open(vault, pp)?.conn(), year)
}

/// Task 9 (B.5): load events + project once, read the year's `TaxProfile` + `BundledTaxTables`,
/// call `compute_tax_year`, and return the `TaxOutcome` for rendering. Standalone "tax owed /
/// what-if" calculator; exact Decimal; deterministic (NFR4/NFR5).
pub fn report_tax_year(vault: &Path, pp: &Passphrase, year: i32) -> Result<TaxOutcome, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, state, _cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(year)?;
    let tables = BundledTaxTables::load();
    Ok(compute_tax_year(
        &events,
        &state,
        year,
        profile.as_ref(),
        &tables,
    ))
}
