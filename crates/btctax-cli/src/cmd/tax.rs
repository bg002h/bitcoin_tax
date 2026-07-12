//! `tax-profile` command helpers — set/show the per-year `TaxProfile` side-table entry.
//! `report_tax_year` (Task 9) provides the standalone "tax owed / what-if" calculator.
//! `report_tax_year` also runs the M4 carryforward-consistency advisory (Task 10).
use crate::{return_inputs, tax_profile, CliError, Session};
use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::tables::FullReturnTables;
use btctax_core::{
    carryforward_consistency, compute_se_tax, compute_tax_year, schedule_d, se_net_income,
    ScheduleDTotals, TaxOutcome, TaxProfile, TaxTables, Usd,
};
use btctax_store::Passphrase;
use std::path::Path;

/// Persist `p` as the tax profile for `year` in the vault at `vault`, then save.
///
/// **D-4 guard (SPEC §4.12):** when full-return `ReturnInputs` already exist for the year, a raw
/// `tax-profile` would be IGNORED (`resolve_profile` gives `ReturnInputs` precedence). Refuse rather than
/// silently store an unused figure — the two-sources-of-truth cardinal sin — unless `force` is set.
pub fn set_profile(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    p: TaxProfile,
    force: bool,
) -> Result<(), CliError> {
    let mut s = Session::open(vault, pp)?;
    if !force && return_inputs::exists(s.conn(), year)? {
        return Err(CliError::Usage(format!(
            "tax year {year} already has full-return inputs (`income import`); a raw tax-profile would be \
             ignored (full-return inputs take precedence). Re-run with --force to store it anyway."
        )));
    }
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

/// `income import` — parse a full-return [`ReturnInputs`] from a TOML file (offline; key order in the file
/// is irrelevant to deserialization) and persist it in the `return_inputs` side-table for `year`.
pub fn import_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    file: &Path,
) -> Result<(), CliError> {
    let text = std::fs::read_to_string(file)?;
    let ri = parse_return_inputs_toml(&text)?;
    let mut s = Session::open(vault, pp)?;
    return_inputs::set(s.conn(), year, &ri)?;
    s.save()
}

/// Parse a `ReturnInputs` from TOML text (split out for testing).
fn parse_return_inputs_toml(text: &str) -> Result<ReturnInputs, CliError> {
    toml::from_str(text).map_err(|e| CliError::Usage(format!("invalid ReturnInputs TOML: {e}")))
}

/// Redact an SSN/ITIN to `***-**-NNNN` (last 4 digits), or empty/`***-**-****` when too short (review I5).
fn mask_ssn(ssn: &str) -> String {
    if ssn.is_empty() {
        return String::new();
    }
    let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 {
        format!("***-**-{}", &digits[digits.len() - 4..])
    } else {
        "***-**-****".to_string()
    }
}

/// A DISPLAY copy of `ReturnInputs` with all SSNs and the IP-PIN redacted (the stored value is never
/// mutated). Used by `income show` so cleartext PII never reaches stdout/scrollback/pipes (SPEC §4.2).
fn mask_pii(ri: &ReturnInputs) -> ReturnInputs {
    let mut m = ri.clone();
    m.header.taxpayer.ssn = mask_ssn(&m.header.taxpayer.ssn);
    if let Some(sp) = m.header.spouse.as_mut() {
        sp.ssn = mask_ssn(&sp.ssn);
    }
    for d in &mut m.header.dependents {
        d.ssn = mask_ssn(&d.ssn);
    }
    if m.header.ip_pin.is_some() {
        m.header.ip_pin = Some("***".to_string());
    }
    m
}

/// `income clear` — remove the stored full-return inputs for `year` (recovery path so a year with
/// `ReturnInputs` isn't a dead end while derivation is pending — review I3). Returns whether a row existed.
pub fn clear_return_inputs(vault: &Path, pp: &Passphrase, year: i32) -> Result<bool, CliError> {
    let mut s = Session::open(vault, pp)?;
    let removed = return_inputs::delete(s.conn(), year)?;
    s.save()?;
    Ok(removed)
}

/// `income show` — the stored [`ReturnInputs`] for `year` as pretty JSON with PII redacted, or `None`.
/// (JSON, not TOML: serde-toml requires scalar keys before nested tables, which the nested model violates;
/// a TOML round-trip-out is a follow-on. Import accepts TOML.)
pub fn show_return_inputs(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
) -> Result<Option<String>, CliError> {
    let ri = return_inputs::get(Session::open(vault, pp)?.conn(), year)?;
    ri.map(|ri| {
        serde_json::to_string_pretty(&mask_pii(&ri)).map_err(|e| CliError::BadConfigValue {
            key: format!("return_inputs[{year}]"),
            value: e.to_string(),
        })
    })
    .transpose()
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

/// Task 9 (B.5) + Task 10 (M4) + P2-D Task 2 + Chunk-1 D2 + Chunk-3a: load events + project once,
/// read the year's `TaxProfile` + `BundledTaxTables`, call `compute_tax_year`, and assemble the
/// standalone Schedule D / Form 709 / Schedule SE artifacts + the M4 carryforward-consistency
/// advisory + the §170(f)(11)(F) year-aggregate donation appraisal advisory. See [`TaxYearReport`]
/// for the returned bundle. The advisory is `Some(msg)` iff BOTH the current-year and the prior-year
/// profiles exist AND the prior-year computes successfully AND the declared `carryforward_in` does
/// not match the prior year's `carryforward_out`. The advisory and the Schedule SE figure are
/// **never** hard blockers and do **not** change the exit code (non-gating).
///
/// `prior_taxable_gifts`: cumulative prior-year TAXABLE gifts (post-annual-exclusion Form 709
/// amounts), not gross gifts. Default $0 (caller passes $0 when the flag is not provided).
pub fn report_tax_year(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    prior_taxable_gifts: Usd,
) -> Result<TaxYearReport, CliError> {
    let s = Session::open(vault, pp)?;
    let (events, state, cfg) = s.load_events_and_project()?;
    // Pseudo-reconcile (sub-project 2, [R0-M6]): when the mode is ON and the year has NO stored profile,
    // inject a CLI-layer PLACEHOLDER profile (single filer, $0 income/MAGI/qual-div) so the estimate can
    // proceed with zero setup. This clears `TaxProfileMissing` ONLY — it is injected AFTER the projection,
    // so it never touches `state.blockers` and thus can NEVER clear the Hard `TaxYearNotComputable` gate
    // (compute.rs checks Hard blockers BEFORE the profile branch). A real stored profile always wins.
    // Single profile-source resolver (SPEC §4.12 / G4): ReturnInputs (derived) → stored TaxProfile → pseudo
    // → missing. Full-return derivation is gated fail-closed by the refuse-guard inside `resolve_profile`.
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let resolved = crate::resolve::resolve_profile(
        s.conn(),
        year,
        cfg.pseudo_reconcile,
        fr_tables.full_return_for(year),
        tables.table_for(year),
    )?;
    if resolved.is_return_inputs_uncomputable() {
        // Full-return `ReturnInputs` exist but could not be derived — surface WHY; never silently treat the
        // year as profile-less (a wrong number). A refusal carries its reason; otherwise the year is one v1
        // has no full-return tables for (TY2024 only).
        return Err(CliError::Usage(match resolved.refusal {
            Some(r) => format!(
                "tax year {year} cannot be computed from its full-return inputs: {}; run \
                 `income clear --year {year}` to remove them and use a raw `tax-profile`",
                r.detail
            ),
            None => format!(
                "tax year {year} has full-return inputs, but full-return computation is not supported for \
                 {year} in this version (v1 supports TY2024); run `income clear --year {year}` to remove \
                 them and use a raw `tax-profile`"
            ),
        }));
    }
    let profile = resolved.profile;
    let outcome = compute_tax_year(&events, &state, year, profile.as_ref(), &tables);
    // P2-B: the RAW pre-netting Schedule D part totals for the same year, from the same projection.
    let sched_d = schedule_d(&state, year);
    // P2-C Task 3 + Chunk-3a: standalone Form 709 gift advisory + §2505 lifetime-exclusion
    // consumption (does NOT feed engine B). prior_taxable_gifts comes from the CLI flag.
    let gift_advisory =
        crate::render::render_gift_advisory(&state, year, prior_taxable_gifts, &tables);
    // P2-D Task 2: standalone Schedule SE §1401 SE-tax figure (STANDALONE — does NOT feed engine B;
    // `total_federal_tax_attributable` is UNCHANGED by SE tax, D5). Requires the year's filing status
    // (from the profile). Business SE income present but no bundled table → the render emits a
    // "wage base unavailable" note (no silent drop); no business SE income → no Schedule SE section.
    let schedule_se = match profile.as_ref() {
        Some(p) => {
            let gross_se = se_net_income(&state, year);
            let table_opt = tables.table_for(year);
            let table_present = table_opt.is_some();
            let se_result = table_opt.and_then(|t| {
                compute_se_tax(
                    &state,
                    year,
                    p.filing_status,
                    t,
                    p.w2_ss_wages,
                    p.w2_medicare_wages,
                    p.schedule_c_expenses,
                )
            });
            crate::render::render_schedule_se(
                year,
                se_result.as_ref(),
                gross_se,
                table_present,
                p.schedule_c_expenses,
                p.w2_ss_wages,
                p.w2_medicare_wages,
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::return_inputs::CharitableClass;
    use btctax_core::FilingStatus;
    use rust_decimal_macros::dec;

    /// A representative `income import` TOML deserializes into `ReturnInputs` — exercises money-as-string
    /// (serde-str), the FilingStatus/Owner/CharitableClass enum reprs, and nested `[[w2s]]` / charitable
    /// arrays. This is the risky part of the import path (field-order in the file is irrelevant).
    #[test]
    fn return_inputs_toml_parses() {
        let text = r#"
            filing_status = "Mfj"

            [[w2s]]
            owner = "taxpayer"
            employer = "ACME"
            box1_wages = "82000"
            box2_fed_withheld = "9100"
            box5_medicare_wages = "82000"

            [[div_1099]]
            payer = "Vanguard"
            box1a_ordinary = "3400"
            box1b_qualified = "3100"

            [schedule_a]
            mortgage_interest_1098 = "11200"
            salt_real_estate = "6800"

            [[schedule_a.charitable]]
            class = "cash60"
            amount = "2500"

            [payments]
            estimated_tax_payments = "6000"
        "#;
        let ri = parse_return_inputs_toml(text).unwrap();
        assert_eq!(ri.filing_status, FilingStatus::Mfj);
        assert_eq!(ri.w2s.len(), 1);
        assert_eq!(ri.w2s[0].box1_wages, dec!(82000));
        assert_eq!(ri.w2s[0].box5_medicare_wages, dec!(82000));
        assert_eq!(ri.div_1099[0].box1b_qualified, dec!(3100));
        let a = ri.schedule_a.as_ref().unwrap();
        assert_eq!(a.mortgage_interest_1098, dec!(11200));
        assert_eq!(a.charitable[0].class, CharitableClass::Cash60);
        assert_eq!(a.charitable[0].amount, dec!(2500));
        assert_eq!(ri.payments.estimated_tax_payments, dec!(6000));
    }

    /// `income show` redacts SSNs and the IP-PIN in a DISPLAY copy; the stored value is untouched (I5).
    #[test]
    fn mask_ssn_and_pii_redacts() {
        assert_eq!(mask_ssn("123-45-6789"), "***-**-6789");
        assert_eq!(mask_ssn("123456789"), "***-**-6789");
        assert_eq!(mask_ssn(""), "");
        assert_eq!(mask_ssn("12"), "***-**-****");
        let mut ri = ReturnInputs::default();
        ri.header.taxpayer.ssn = "123-45-6789".into();
        ri.header.ip_pin = Some("999999".into());
        ri.header.spouse = Some(btctax_core::tax::return_inputs::Person {
            ssn: "987-65-4321".into(),
            ..Default::default()
        });
        ri.header.dependents = vec![btctax_core::tax::return_inputs::Dependent {
            ssn: "111-22-3333".into(),
            ..Default::default()
        }];
        let masked = mask_pii(&ri);
        assert_eq!(masked.header.taxpayer.ssn, "***-**-6789");
        assert_eq!(masked.header.spouse.as_ref().unwrap().ssn, "***-**-4321");
        assert_eq!(masked.header.dependents[0].ssn, "***-**-3333");
        assert_eq!(masked.header.ip_pin.as_deref(), Some("***"));
        assert_eq!(ri.header.taxpayer.ssn, "123-45-6789"); // original untouched
        assert_eq!(ri.header.spouse.as_ref().unwrap().ssn, "987-65-4321"); // original untouched
    }

    /// Malformed TOML is a typed `Usage` error, never a panic.
    #[test]
    fn bad_toml_is_typed_error() {
        assert!(matches!(
            parse_return_inputs_toml("not = = toml").unwrap_err(),
            CliError::Usage(_)
        ));
    }
}
