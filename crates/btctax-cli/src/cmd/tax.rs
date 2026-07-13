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
    let mut ri = parse_return_inputs_toml(&text)?;
    let mut s = Session::open(vault, pp)?;
    // §4 R3-M6 (Fable P4.9 r1 I2): `income import` is a whole-blob upsert, so a re-import would SILENTLY
    // DROP a carryover that `report --write-carryover` computed onto this row. For QBI that is a fail-OPEN
    // (losing the REIT/PTP loss carryforward OVERSTATES the QBI deduction ⇒ understates tax). So a
    // **Computed** carryover-in SURVIVES an import that does not itself supply one; a carryover the TOML
    // *does* supply is the user's and wins (as `User`, which the next write-back then refuses to clobber).
    if let Some(existing) = return_inputs::get(s.conn(), year)? {
        use btctax_core::tax::return_inputs::CarryProvenance;
        let mut preserved: Vec<String> = Vec::new();
        if ri.charitable_carryover_in.is_empty() {
            let computed: Vec<_> = existing
                .charitable_carryover_in
                .iter()
                .filter(|c| c.provenance == CarryProvenance::Computed)
                .cloned()
                .collect();
            if !computed.is_empty() {
                preserved.push(format!("{} charitable carryover item(s)", computed.len()));
                ri.charitable_carryover_in = computed;
            }
        }
        if ri.qbi.reit_ptp_carryforward_in.is_zero()
            && existing.qbi.reit_ptp_carryforward_in > rust_decimal::Decimal::ZERO
            && existing.qbi.reit_ptp_carryforward_in_provenance == CarryProvenance::Computed
        {
            preserved.push(format!(
                "QBI REIT/PTP carryforward ${:.2}",
                existing.qbi.reit_ptp_carryforward_in
            ));
            ri.qbi = existing.qbi.clone();
        }
        if !preserved.is_empty() {
            eprintln!(
                "note: kept the computed carryover already on the {year} row ({}) — your TOML did not \
                 supply one. To replace it, put the carryover in the TOML (it then counts as user-entered), \
                 or re-run `report --tax-year {} --write-carryover`.",
                preserved.join("; "),
                year - 1
            );
        }
    }
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

/// The full `report --tax-year` bundle, in print order. A NAMED STRUCT (was a 7-tuple) so a new field can
/// never silently transpose with an existing one at a call site (Fable IMPL-P4 r1 N1, `p4-r1-n1`).
#[derive(Debug)]
pub struct TaxYearReport {
    /// The frozen crypto-DELTA engine's outcome for the year.
    pub outcome: TaxOutcome,
    /// M4 carryforward-consistency advisory (non-gating).
    pub advisory: Option<String>,
    /// RAW pre-netting Schedule D part totals.
    pub schedule_d: ScheduleDTotals,
    /// Standalone Form 709 gift advisory.
    pub gift_advisory: Option<String>,
    /// Standalone Schedule SE §1401 section.
    pub schedule_se: Option<String>,
    /// §170(f)(11)(F) year-aggregate donation appraisal advisory.
    pub donation_appraisal: Option<String>,
    /// The §6 dual-report block (absolute filed return + crypto delta + the P5 advisories). `Some` only
    /// for a `ReturnInputs`-provenance year; `None` on the delta-only path.
    pub dual_report: Option<String>,
}

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
    // Single resolver + BOTH refuse-guards, fail-closed (SPEC §4.12 / §4.10 / G4): ReturnInputs (derived,
    // input- AND compute-screened) → stored TaxProfile → pseudo → missing. `resolve_and_screen` is the one
    // entry point every computing consumer shares so the app never shows two liabilities for one year.
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (profile, provenance) = match crate::resolve::resolve_and_screen(
        s.conn(),
        &state,
        year,
        cfg.pseudo_reconcile,
        fr_tables.full_return_for(year),
        tables.table_for(year),
    )? {
        crate::resolve::ProfileOutcome::Uncomputable { detail } => {
            return Err(CliError::Usage(detail))
        }
        crate::resolve::ProfileOutcome::Ready {
            profile,
            provenance,
        } => (profile, provenance),
    };
    let outcome = compute_tax_year(&events, &state, year, profile.as_ref(), &tables);

    // §6 DUAL REPORT (SPEC §6 / §5 stages 1–9): the absolute filed return, side-by-side with the crypto
    // delta above. Only meaningful for a `ReturnInputs`-provenance year — the input-screen + compute-
    // dependent screen have already passed inside the resolver (else we returned `Uncomputable`), and
    // TY2024 is the only year with `FullReturnParams` (so both `Option`s are `Some` here). The absolute
    // path adds `screen_absolute` (QBI-over-threshold / AMT / TI≤0-with-carryforward), which — unlike the
    // delta path — can refuse the ABSOLUTE return while the delta still computes; render that as a note.
    let dual_report: Option<String> = if provenance == crate::resolve::Provenance::ReturnInputs {
        match (
            crate::return_inputs::get(s.conn(), year)?,
            fr_tables.full_return_for(year),
            tables.table_for(year),
        ) {
            (Some(ri), Some(params), Some(table)) => {
                let ar = btctax_core::assemble_absolute(&ri, &state, params, table, year);
                match btctax_core::screen_absolute(&ri, &ar, params) {
                    Some(refusal) => Some(format!(
                        "\n═══ Absolute filed return (Form 1040) — tax year {year} ═══\n  \
                         Profile source: {}\n  NOT COMPUTABLE [{:?}]: {}\n",
                        crate::render::provenance_label(provenance),
                        refusal.reason,
                        refusal.detail
                    )),
                    None => {
                        // P5: the full-return block carries the §3.4 conservative-omission advisories
                        // (CTC/ODC, EIC, forfeited §63(f) aged box) + the FBAR / charitable-donee
                        // disclosures. Non-gating: they never change a number or the exit code.
                        let mut block =
                            crate::render::render_dual_report(year, &ar, &outcome, provenance);
                        let advs = btctax_core::tax::advisories::advisories_for(
                            &ri, &state, &ar, params, year,
                        );
                        block.push_str(&crate::render::render_advisories(&advs));
                        Some(block)
                    }
                }
            }
            _ => {
                // ReturnInputs provenance implies the inputs + TY2024 params/table are present (else the
                // resolver returned Uncomputable) — fail loud in debug if that invariant ever breaks.
                debug_assert!(
                    false,
                    "ReturnInputs provenance but missing inputs/params/table for year {year}"
                );
                None
            }
        }
    } else {
        None
    };
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
        // Prior-year profile through the same resolver (ReturnInputs-derived too); the M4 advisory is
        // non-gating, so an uncomputable/refused prior year just skips it rather than failing the report.
        let prior_profile = match s.resolve_screened(&state, year - 1, &tables)? {
            crate::resolve::ProfileOutcome::Ready { profile, .. } => profile,
            crate::resolve::ProfileOutcome::Uncomputable { .. } => None,
        };
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

    Ok(TaxYearReport {
        outcome,
        advisory,
        schedule_d: sched_d,
        gift_advisory,
        schedule_se,
        donation_appraisal: donation_appraisal_advisory,
        dual_report,
    })
}

/// §4 R3-M6 carryover write-back — persist year `year`'s computed charitable + QBI-REIT/PTP carryover-OUT
/// as year (`year+1`)'s carryover-IN in the side-table. Only for a `ReturnInputs`-provenance full-return
/// year (else there is no absolute return). Errors if the absolute return refuses (`screen_absolute`) or if
/// a user-entered next-year carryover would be overwritten without `force`. Returns a human summary.
pub fn write_back_carryover(
    vault: &Path,
    pp: &Passphrase,
    year: i32,
    force: bool,
) -> Result<String, CliError> {
    let mut s = Session::open(vault, pp)?;
    let (_events, state, cfg) = s.load_events_and_project()?;
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (Some(params), Some(table)) = (fr_tables.full_return_for(year), tables.table_for(year))
    else {
        return Err(CliError::Usage(format!(
            "no full-return tables for {year} — carryover write-back needs a supported tax year (TY2024)"
        )));
    };
    // Must be a ReturnInputs-provenance year with both refuse screens passed (fail-closed).
    let provenance = match crate::resolve::resolve_and_screen(
        s.conn(),
        &state,
        year,
        cfg.pseudo_reconcile,
        Some(params),
        Some(table),
    )? {
        crate::resolve::ProfileOutcome::Uncomputable { detail } => {
            return Err(CliError::Usage(detail))
        }
        crate::resolve::ProfileOutcome::Ready { provenance, .. } => provenance,
    };
    if provenance != crate::resolve::Provenance::ReturnInputs {
        return Err(CliError::Usage(format!(
            "carryover write-back needs full-return inputs for {year} (`income import`); the resolved \
             profile source is {provenance:?}"
        )));
    }
    let ri = crate::return_inputs::get(s.conn(), year)?
        .ok_or_else(|| CliError::Usage(format!("no return_inputs stored for {year}")))?;
    let ar = btctax_core::assemble_absolute(&ri, &state, params, table, year);
    if let Some(refusal) = btctax_core::screen_absolute(&ri, &ar, params) {
        return Err(CliError::Usage(format!(
            "the {year} absolute return is not computable [{:?}]: {} — carryover not written",
            refusal.reason, refusal.detail
        )));
    }
    // SPEC §4 R3-M6 writes the carryover "as year (Y+1)'s `*_carryover_in` **on that row**" — the row must
    // ALREADY exist. Fabricating one would put a `ReturnInputs` row at the TOP of the §4.12 precedence
    // ladder for a year v1 has no full-return tables for (Y+1 is always 2025 in v1), which fails closed and
    // would make that year uncomputable — shadowing a stored `TaxProfile` the user was planning with, and
    // blocking `tax-profile --year Y+1` via the D-4 guard (Fable P4.9 r1 I1).
    let next = crate::return_inputs::get(s.conn(), year + 1)?.ok_or_else(|| {
        CliError::Usage(format!(
            "year {next} has no full-return inputs yet — the carryover is written onto that row, so import \
             it first (`income import --year {next} --file <toml>`) and then re-run `--write-carryover`. \
             (Creating the row here would shadow any stored tax-profile for {next} and make it uncomputable \
             in this version, which supports full returns for TY2024 only.)",
            next = year + 1
        ))
    })?;
    let updated =
        btctax_core::apply_carryover_writeback(&ar, next, force).map_err(CliError::Usage)?;
    crate::return_inputs::set(s.conn(), year + 1, &updated)?;
    s.save()?;
    Ok(format!(
        "carryover written back to {}: {} charitable carryover item(s); QBI REIT/PTP carryforward ${:.2}",
        year + 1,
        updated.charitable_carryover_in.len(),
        updated.qbi.reit_ptp_carryforward_in
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
