//! `tax-profile` command helpers — set/show the per-year `TaxProfile` side-table entry.
//! `report_tax_year` (Task 9) will be added here once the report wiring is ready.
use crate::{tax_profile, CliError, Session};
use btctax_core::TaxProfile;
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
