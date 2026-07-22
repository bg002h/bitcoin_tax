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
    // ★ §6.2 (M-1): reconcile the crash-recovery draft BEFORE any committed-row read/write — clear a WIP
    // draft (regenerable) so it can't shadow this write, or refuse a parked one (its sole copy).
    crate::input_form_store::coherence_clear_or_refuse(s.conn(), year)?;
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
///
/// ★ P9 §2.3 — REJECTS unknown keys, via `serde_ignored` rather than a hand-written key list (which would
/// be the exact drift-prone hand-wiring P9 abolishes). `serde_ignored` reports every ignored path DURING
/// the same deserialization, so the key set is DERIVED from the type: no list to forget, and `[[w2s]]`
/// arrays, nested tables and comments all work for free. This binds ONLY the CLI's TOML import — the
/// stored-JSON path (`return_inputs::get`) keeps its documented forward-compat and is untouched. Without
/// this, a faithfully-transcribed `box13_retirement_plan` (a deleted field) or a `hsa_present` (the §2.4
/// rename) would import CLEAN and silently vanish — no error, no trace even in `income show`.
fn parse_return_inputs_toml(text: &str) -> Result<ReturnInputs, CliError> {
    // Parse to the TOML tree FIRST (toml's streaming deserializer + serde_ignored mishandles arrays of
    // tables), then run `serde_ignored` over the in-memory `Value` to collect every unknown path.
    let value: toml::Value = toml::from_str(text)
        .map_err(|e| CliError::Usage(format!("invalid ReturnInputs TOML: {e}")))?;
    let mut ignored: Vec<String> = Vec::new();
    let ri: ReturnInputs = serde_ignored::deserialize(value, |path| ignored.push(path.to_string()))
        .map_err(|e| CliError::Usage(format!("invalid ReturnInputs TOML: {e}")))?;
    if !ignored.is_empty() {
        return Err(CliError::Usage(format!(
            "unknown key(s) in the ReturnInputs TOML: {}. btctax does not honor these — likely a typo or a \
             field removed in this version (e.g. `hsa_present` was RENAMED to `sch1.hsa_activity`; \
             `box13_retirement_plan` and `ssn_valid_for_employment` were REMOVED). Fix or delete them, then \
             re-run `btctax income import` — a silently-ignored key would drop data you meant to enter.",
            ignored.join(", ")
        )));
    }
    Ok(ri)
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
    // ★ §6.2 (M-1): a parked draft is the sole copy of a screened return — refuse rather than let this
    // clear leave it silently orphaned; a WIP draft is cleared alongside the committed-row delete.
    crate::input_form_store::coherence_clear_or_refuse(s.conn(), year)?;
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
        let mkerr = |e: serde_json::Error| CliError::BadConfigValue {
            key: format!("return_inputs[{year}]"),
            value: e.to_string(),
        };
        // M-1 (DONE, post-v0.7.0): `serde_json` `preserve_order` is enabled workspace-wide, so routing
        // through `to_value` to host the DOB transform now preserves the ReturnInputs struct's declared
        // field order (curated) instead of sorting keys alphabetically. `income show` is display-only and
        // never parsed (M8); typed serde (the STORED serialization) is field-ordered regardless, so the
        // persisted bytes + fingerprints are unaffected by the flip.
        let mut val = serde_json::to_value(mask_pii(&ri)).map_err(mkerr)?;
        format_dobs_readable(&mut val); // UX-P1-5: render date_of_birth as MM/DD/YYYY, not raw [year, ordinal]
        serde_json::to_string_pretty(&val).map_err(mkerr)
    })
    .transpose()
}

/// UX-P1-5: `income show`'s JSON serializes each `time::Date` as a raw `[year, ordinal-day]` array (e.g.
/// `[2012, 106]`), which no filer reads as a calendar date. Rewrite every `date_of_birth` value in the
/// DISPLAY tree to a human `MM/DD/YYYY` string. Display-only — the STORED serialization is untouched
/// (`income show` is for viewing, never parsed back — M8).
fn format_dobs_readable(v: &mut serde_json::Value) {
    use time::macros::format_description;
    match v {
        serde_json::Value::Object(map) => {
            for (k, val) in map.iter_mut() {
                if k == "date_of_birth" {
                    // Extract MM/DD/YYYY (the closure's immutable borrow of `val` ends before the write).
                    let readable = val.as_array().filter(|a| a.len() == 2).and_then(|a| {
                        let y = a[0].as_i64()? as i32;
                        let o = a[1].as_u64()? as u16;
                        let d = time::Date::from_ordinal_date(y, o).ok()?;
                        d.format(&format_description!("[month]/[day]/[year]")).ok()
                    });
                    if let Some(s) = readable {
                        *val = serde_json::Value::String(s);
                        continue;
                    }
                }
                format_dobs_readable(val);
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(format_dobs_readable),
        _ => {}
    }
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
    /// Conservative-filing (D-9) advisory: per-disposal tranche dip lines + per-wallet method-inversion
    /// warnings. Provenance-neutral; non-gating (never affects the outcome or exit code).
    pub tranche_advisory: Option<String>,
    /// The §6 dual-report block (absolute filed return + crypto delta + the P5 advisories). `Some` only
    /// for a `ReturnInputs`-provenance year; `None` on the delta-only path.
    pub dual_report: Option<String>,
    /// UX-P4-1: the pseudo-disclosure channel for this year's figures — the full §3.1 predicate
    /// (`pseudo_active() OR PseudoPlaceholder`, Synthetic-wins). Drives the banner + `[PSEUDO]` suffix on
    /// every number-bearing surface (delta report, dual-report absolute totals, TUI Tax tab) and the
    /// fail-closed `--write-carryover` gate; `None` when the figures are not pseudo-contributed.
    pub pseudo_contributed: crate::render::PseudoDisclosure,
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

    // UX-P4-1: the pseudo-disclosure channel for the figures below. `Synthetic` (a pseudo synthetic
    // lot/FMV feeds the number) wins over `Placeholder` (computed on the all-$0 placeholder profile) — the
    // two are mutually exclusive by precedence though the states can co-occur (SPEC §3.1). Read from the
    // LIVE pseudo-ON projected state + provenance (NOT a pseudo-OFF view — that would zero the count and
    // silence the banner, reinstating the answered-ness false-negative).
    let pseudo_contributed = if state.pseudo_active() {
        crate::render::PseudoDisclosure::Synthetic
    } else if provenance == crate::resolve::Provenance::PseudoPlaceholder {
        crate::render::PseudoDisclosure::Placeholder
    } else {
        crate::render::PseudoDisclosure::None
    };

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
                        //
                        // ★ P6.3b: the block renders the PRINTED figures — exactly what the filed PDF
                        // carries. `assemble_printed_forms` is infallible and PII-free, so a household
                        // that has entered no identity yet still sees the real numbers (only the filable
                        // ARTIFACT needs a name and an SSN).
                        let details = s.donation_details()?;
                        let printed = btctax_core::tax::packet::assemble_printed_forms(
                            &ri, &state, &details, &ar, table, year, &events,
                        );
                        let mut block = crate::render::render_dual_report(
                            year,
                            &ar,
                            &printed,
                            &outcome,
                            provenance,
                            pseudo_contributed,
                        );
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

    // Conservative-filing (P3 / D-9): tranche dip + method-inversion advisory. Non-gating; render-time
    // only, like the standalone-forms advisories above. The shared core assembler keeps the CLI + TUI
    // surfaces identical.
    let tranche_advisory = btctax_core::conservative::tranche_report_advisory(
        &state,
        &events,
        s.prices(),
        &cfg,
        year,
        profile.as_ref(),
        &tables,
    );

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
        tranche_advisory,
        dual_report,
        pseudo_contributed,
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
    // ★ §6.2 (M-1): write-back reads AND writes the year+1 committed row, so it reconciles the year+1
    // draft here — before the year+1 read below, which early-returns on an absent row (a parked year has
    // none) and would otherwise shadow the parked-refuse remedy.
    crate::input_form_store::coherence_clear_or_refuse(s.conn(), year + 1)?;
    let (events, state, cfg) = s.load_events_and_project()?;
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (Some(params), Some(table)) = (fr_tables.full_return_for(year), tables.table_for(year))
    else {
        return Err(CliError::Usage(format!(
            "no full-return tables for {year} — carryover write-back needs a supported tax year (TY2024)"
        )));
    };
    // Must be a ReturnInputs-provenance year with both refuse screens passed (fail-closed).
    let (profile, provenance) = match crate::resolve::resolve_and_screen(
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
        crate::resolve::ProfileOutcome::Ready {
            profile,
            provenance,
        } => (profile, provenance),
    };
    if provenance != crate::resolve::Provenance::ReturnInputs {
        return Err(CliError::Usage(format!(
            "carryover write-back needs full-return inputs for {year} (`income import`); the resolved \
             profile source is {provenance:?}"
        )));
    }
    // UX-P4-1 surface 4 (SPEC §3.1 clause 4) [T-C1 + G2-NEW-4]: NEVER persist a carryover derived from a
    // pseudo-tainted OR hard-blocked ledger into year+1's stored inputs. Next year `pseudo_active()` is
    // false and the UX-P4-1 banner correctly does not fire — so an unflagged, deliberately-fictional (or
    // unanswerable) figure would ride into a real input. Fail-closed, consistent with the export gate.
    // (4a) At this gate the `PseudoPlaceholder` disjunct is structurally inert (provenance is ReturnInputs,
    // just checked), so `pseudo_active()` is the operative half of the §3.1 predicate.
    if state.pseudo_active() {
        return Err(CliError::Usage(format!(
            "carryover write-back REFUSED for {year}: pseudo-reconcile mode is contributing synthetic \
             default(s), so the derived carryover is an ESTIMATE — persisting it as {next}'s real input \
             would launder a deliberately-synthetic figure. Resolve the pseudo entries (or turn the mode \
             off) first.",
            next = year + 1
        )));
    }
    // (4b) A `NotComputable` crypto-delta means the ledger carries Hard blockers the engine refuses to
    // answer for; a carryover assembled over that state must not be persisted (the same laundering class
    // minus the pseudo mechanism).
    if let btctax_core::TaxOutcome::NotComputable(b) =
        compute_tax_year(&events, &state, year, profile.as_ref(), &tables)
    {
        return Err(CliError::Usage(format!(
            "carryover write-back REFUSED for {year}: the crypto-delta ledger is NOT COMPUTABLE [{:?}]: {} \
             — a carryover from an unanswerable ledger must not be written into {next}'s inputs.",
            b.kind,
            b.detail,
            next = year + 1
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

    /// Shared temp-vault fixture (mirrors `input_form_store.rs`'s helper, M-3): `create` + drop releases
    /// the store single-instance lock so a later `Session::open` (here, the one inside the command under
    /// test) can re-acquire it. The `TempDir` guard MUST be kept alive by the caller.
    fn tmp_vault() -> (tempfile::TempDir, std::path::PathBuf, Passphrase) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.pgp");
        {
            let _ = Session::create(&path, &Passphrase::new("test-pass".into())).unwrap();
        }
        (dir, path, Passphrase::new("test-pass".into()))
    }

    /// ★ §6.2 wiring — `income clear` REFUSES a year that holds a PARKED draft (the draft is the sole copy
    /// of a screened return, C-1), and never destroys it. `clear_return_inputs` needs no pre-existing
    /// committed row, so the coherence call is the cheapest reachable parked-refuse: this test pins the
    /// wiring into a real writer. Remove the `coherence_clear_or_refuse` call from `clear_return_inputs`
    /// and this goes red (mutation-check b).
    #[test]
    fn income_clear_refuses_a_parked_draft_and_preserves_it() {
        let (_dir, path, pp) = tmp_vault();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        {
            let mut s = Session::open(&path, &pp).unwrap();
            crate::input_form_store::set_draft_row(s.conn(), 2024, &ri, true).unwrap(); // parked
            s.save().unwrap();
        }
        let err = clear_return_inputs(&path, &pp, 2024).unwrap_err();
        assert!(
            matches!(err, CliError::ParkedDraftBlocksWrite { year: 2024 }),
            "income clear must refuse a parked-draft year, got {err:?}"
        );
        // the parked draft is STILL present — a committed-row write never silently destroys it.
        let s = Session::open(&path, &pp).unwrap();
        assert!(
            crate::input_form_store::draft_exists(s.conn(), 2024).unwrap(),
            "a refused clear must leave the parked draft intact"
        );
    }

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

    /// ★ P9 §2.3 / §3.5 (r7 I-2) — `income import` REJECTS unknown TOML keys via `serde_ignored`, not a
    /// hand-written key list. A TOML carrying `hsa_present` (the §2.4 rename) AND `box13_retirement_plan`
    /// (a deleted dead field — a real W-2 box 13 faithfully transcribed) must REFUSE naming BOTH, rather
    /// than import clean and silently vanish (the exact hole §2.3 exists to close). Mutation: revert to a
    /// bare `toml::from_str` ⇒ this fails.
    #[test]
    fn income_import_rejects_unknown_toml_keys_naming_each() {
        let text = r#"
            filing_status = "Single"
            hsa_present = false

            [[w2s]]
            owner = "taxpayer"
            employer = "ACME"
            box1_wages = "50000"
            box2_fed_withheld = "8000"
            box13_retirement_plan = true
        "#;
        let err = parse_return_inputs_toml(text).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("hsa_present"),
            "must name the renamed key: {msg}"
        );
        assert!(
            msg.contains("box13_retirement_plan"),
            "must name the deleted dead field so a transcribed W-2 box 13 can't silently vanish: {msg}"
        );
    }
}
