//! `tax-profile` command helpers — set/show the per-year `TaxProfile` side-table entry.
//! `report_tax_year` (Task 9) provides the standalone "tax owed / what-if" calculator.
//! `report_tax_year` also runs the M4 carryforward-consistency advisory (Task 10).
use crate::{tax_profile, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    carryforward_consistency, compute_se_tax, compute_tax_year, schedule_d, se_net_income,
    ScheduleDTotals, TaxOutcome, TaxProfile, TaxTables,
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

/// The full `report --tax-year` bundle, in print order:
/// `(income-tax outcome, M4 carryforward advisory, raw Schedule D part totals, Form 709 gift
/// advisory, Schedule SE §1401 section, §170(f)(11)(F) donation appraisal advisory)`. Named to
/// satisfy `clippy::type_complexity`; callers still destructure it as a tuple.
pub type TaxYearReport = (
    TaxOutcome,
    Option<String>,
    ScheduleDTotals,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Task 9 (B.5) + Task 10 (M4) + P2-D Task 2 + Chunk-1 D2: load events + project once, read the
/// year's `TaxProfile` + `BundledTaxTables`, call `compute_tax_year`, and assemble the standalone
/// Schedule D / Form 709 / Schedule SE artifacts + the M4 carryforward-consistency advisory + the
/// §170(f)(11)(F) year-aggregate donation appraisal advisory. See [`TaxYearReport`] for the returned
/// bundle. The advisory is `Some(msg)` iff BOTH the current-year and the prior-year profiles exist
/// AND the prior-year computes successfully AND the declared `carryforward_in` does not match the
/// prior year's `carryforward_out`. The advisory and the Schedule SE figure are **never** hard
/// blockers and do **not** change the exit code (non-gating).
pub fn report_tax_year(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<TaxYearReport, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, state, _cfg) = s.load_events_and_project()?;
    let profile = s.tax_profile(year)?;
    let tables = BundledTaxTables::load();
    let outcome = compute_tax_year(&events, &state, year, profile.as_ref(), &tables);
    // P2-B: the RAW pre-netting Schedule D part totals for the same year, from the same projection.
    let sched_d = schedule_d(&state, year);
    // P2-C Task 3: standalone Form 709 gift over-annual-exclusion advisory (does NOT feed engine B).
    let gift_advisory = crate::render::render_gift_advisory(&state, year, &tables);
    // P2-D Task 2: standalone Schedule SE §1401 SE-tax figure (STANDALONE — does NOT feed engine B;
    // `total_federal_tax_attributable` is UNCHANGED by SE tax, D5). Requires the year's filing status
    // (from the profile). Business SE income present but no bundled table → the render emits a
    // "wage base unavailable" note (no silent drop); no business SE income → no Schedule SE section.
    let schedule_se = match profile.as_ref() {
        Some(p) => {
            let business_income_present = !se_net_income(&state, year).is_zero();
            let se_result = tables
                .table_for(year)
                .and_then(|t| compute_se_tax(&state, year, p.filing_status, t));
            crate::render::render_schedule_se(year, se_result.as_ref(), business_income_present)
        }
        None => None,
    };
    // Chunk-1 D2: §170(f)(11)(F) year-aggregate donation appraisal advisory (STANDALONE — does NOT
    // enter state.advisory / the blocker set; render-time only, consistent with the standalone-forms
    // pattern). Non-gating; does not affect the exit code.
    let donation_appraisal_advisory =
        crate::render::render_donation_appraisal_advisory(&state, year);

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

    Ok((
        outcome,
        advisory,
        sched_d,
        gift_advisory,
        schedule_se,
        donation_appraisal_advisory,
    ))
}
