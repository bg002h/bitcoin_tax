//! `tax-profile` command helpers — set/show the per-year `TaxProfile` side-table entry.
//! `report_tax_year` (Task 9) provides the standalone "tax owed / what-if" calculator.
//! `report_tax_year` also runs the M4 carryforward-consistency advisory (Task 10).
use crate::{tax_profile, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    carryforward_consistency, compute_tax_year, schedule_d, ScheduleDTotals, TaxOutcome, TaxProfile,
};
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

/// Task 9 (B.5) + Task 10 (M4): load events + project once, read the year's `TaxProfile` +
/// `BundledTaxTables`, call `compute_tax_year`, and also run the M4 carryforward-consistency
/// advisory. Returns `(TaxOutcome, Option<advisory_msg>)`. The advisory is `Some(msg)` iff BOTH
/// the current-year and the prior-year profiles exist AND the prior-year computes successfully AND
/// the declared `carryforward_in` does not match the prior year's `carryforward_out`. The advisory
/// is **never** a hard blocker and does **not** change the exit code (non-gating, Task 10).
pub fn report_tax_year(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<(TaxOutcome, Option<String>, ScheduleDTotals, Option<String>), CliError> {
    let s = Session::open(vault, pp)?;
    let (events, state, _cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(year)?;
    let tables = BundledTaxTables::load();
    let outcome = compute_tax_year(&events, &state, year, profile.as_ref(), &tables);
    // P2-B: the RAW pre-netting Schedule D part totals for the same year, from the same projection.
    let sched_d = schedule_d(&state, year);
    // P2-C Task 3: standalone Form 709 gift over-annual-exclusion advisory (does NOT feed engine B).
    let gift_advisory = crate::render::render_gift_advisory(&state, year, &tables);

    // M4 carryforward consistency advisory (Task 10): only when both this year's profile AND
    // the prior year's profile exist AND the prior year is Computed.  Never a hard blocker.
    let advisory: Option<String> = if let Some(p) = &profile {
        let prior_profile = s.tax_profile(year - 1)?;
        if let Some(prev_p) = prior_profile {
            let prior_out = compute_tax_year(&events, &state, year - 1, Some(&prev_p), &tables);
            if let TaxOutcome::Computed(prev) = prior_out {
                carryforward_consistency(
                    Some(&prev.carryforward_out),
                    &p.capital_loss_carryforward_in,
                )
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok((outcome, advisory, sched_d, gift_advisory))
}
