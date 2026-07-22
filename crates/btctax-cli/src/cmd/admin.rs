//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the pre-2025 lot method; export/backup arrive in Task 15.
use crate::cli::FormArg;
use crate::config::{set_fee_treatment, set_pre2025_method as config_set_pre2025_method};
use crate::render::write_csv_exports;
use crate::{require_attestation, CliConfig, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    compute_se_tax, se_net_income, FeeTreatment, LedgerEvent, LotMethod, ScheduleDPart, Severity,
    TaxTables, Usd, DIGITAL_ASSET_8949_FIRST_YEAR,
};
use btctax_forms::Form1040Inputs;
use btctax_store::{fsperms, Passphrase};
use std::path::{Path, PathBuf};

/// Outcome of the CLI `export_snapshot` wrapper: the written snapshot path plus the count of
/// UNRESOLVED Hard blockers (`severity() == Hard`) in the projection. Any Hard blocker gates EVERY
/// tax year (`compute_tax_year` short-circuits on the projection-wide first Hard blocker), so
/// `unresolved_hard > 0` means every exported Form 8949 / Schedule D / projection CSV is
/// INFORMATIONAL, not final — the `ExportSnapshot` main.rs arm warns on stderr accordingly. A
/// fully-resolved ledger yields `0` and no warning. Advisory blockers (incl. `PseudoReconcileActive`,
/// `SelfTransferInboundZeroBasis`) never count.
#[derive(Debug, Clone)]
pub struct ExportReport {
    pub path: PathBuf,
    pub unresolved_hard: usize,
}

pub fn show_config(vault_path: &Path, pp: &Passphrase) -> Result<CliConfig, CliError> {
    Session::open(vault_path, pp)?.config()
}

/// Persist a new TP8 fee treatment (None = leave unchanged), then return the resulting config.
pub fn set_config(
    vault_path: &Path,
    pp: &Passphrase,
    fee_treatment: Option<FeeTreatment>,
) -> Result<CliConfig, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    if let Some(t) = fee_treatment {
        set_fee_treatment(session.conn(), t)?;
        session.save()?;
    }
    session.config()
}

/// Persist the pre-2025 lot identification method and attestation flag, then return the resulting config.
pub fn set_pre2025_method(
    vault_path: &Path,
    pp: &Passphrase,
    m: LotMethod,
    attested: bool,
) -> Result<CliConfig, CliError> {
    let mut session = Session::open(vault_path, pp)?;
    config_set_pre2025_method(session.conn(), m, attested)?;
    session.save()?;
    session.config()
}

/// **BG-D8 (Task 14) — the export COMPLETENESS gate.** REFUSES the export (writing ZERO bytes: called
/// FIRST in each export fn, before any `mkdir_out`/file write) when a promoted-basis DISPOSAL leg is filed
/// in the exported range but its Form 8275 disclosure is absent or INCOMPLETE (an empty/scaffold-only Part
/// II). Reg §1.6662-4(f) makes a disclosure adequate only on a COMPLETED Form 8275; a promoted leg filed
/// without one is inadequate disclosure — a HARD refusal, never a warning.
///
/// Mirrors the pseudo-active attestation slot (`if state.pseudo_active() { require_attestation(...)? }`):
/// a real refuse-before-bytes gate. It is deliberately NOT the always-written `basis_methodology.txt`
/// pattern (which unconditionally writes and can never refuse) — a refused export leaves `out_dir`
/// untouched.
///
/// `year: Some(y)` scopes the check to `y` (the per-year PDF packets). `year: None` — the non-year-scoped
/// CSV/snapshot export — means "ANY year with a promoted filed disposal leg in the exported range" (N-3),
/// so an all-years dump can never smuggle out an inadequately-disclosed promoted position either.
///
/// The refusing state is only reachable via a hand-crafted raw-vault write (an empty `part_ii_narrative`):
/// the T10 `promote-tranche` verb refuses an empty narrative at record time (BG-D7), so a CLI-recorded
/// promote is complete by construction — this gate is the type-level backstop for the corner it cannot.
pub fn promote_export_gate(
    state: &btctax_core::state::LedgerState,
    events: &[LedgerEvent],
    year: Option<i32>,
) -> Result<(), CliError> {
    // The year(s) to check: the requested one, or — for the whole-range CSV/snapshot dump — every year in
    // which a promoted disposal leg files.
    let years: Vec<i32> = match year {
        Some(y) => vec![y],
        None => {
            let mut ys = std::collections::BTreeSet::new();
            for d in &state.disposals {
                if d.legs
                    .iter()
                    .any(|l| state.promoted_origins.contains(&l.lot_id.origin_event_id))
                {
                    ys.insert(d.disposed_at.year());
                }
            }
            ys.into_iter().collect()
        }
    };
    for y in years {
        // `disclosure_8275` is `Some` iff a promoted DISPOSAL leg files in `y`; refuse when its Part II is
        // empty/incomplete (the `incomplete` flag T13 exposes for exactly this gate).
        if let Some(disc) = btctax_core::tax::form8275::disclosure_8275(events, state, y) {
            if disc.incomplete {
                return Err(CliError::Usage(format!(
                    "refusing to export a packet with a promoted-basis leg but no complete Form 8275 \
                     disclosure for {y}: Reg \u{00a7}1.6662-4(f) makes disclosure adequate only on a \
                     COMPLETED Form 8275, and this promoted disposal has an empty Part II narrative. \
                     Record the Part II explanation (re-run `btctax reconcile promote-tranche … \
                     --part-ii-file <path>`) before exporting."
                )));
            }
        }
    }
    Ok(())
}

/// FR10 / NFR2 exception: decrypted SQLite image (via the store) + the projected ledger as CSV.
/// When `tax_year` is `Some(y)`, the per-tax-year Form 8949 + Schedule D CSVs are also written,
/// year-scoped to `y` (P2-B); when `None`, only the all-years CSVs are written.
///
/// Sub-project 3 attestation gate: when the projection is pseudo-active (a synthetic default
/// contributes), producing any form/data file requires the exact `ATTEST_PHRASE` in `attest`
/// (trimmed, case-sensitive). Checked FIRST — before any bytes are written — so a refused export
/// leaves `out_dir` untouched. A fully-real (not-pseudo-active) ledger ignores `attest` entirely.
pub fn export_snapshot(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: Option<i32>,
    attest: Option<&str>,
) -> Result<ExportReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    // Two refuse-before-bytes gates, checked FIRST (before the vault snapshot / CSV writes) so a refused
    // export leaves out_dir untouched:
    //  1. BG-D8 completeness gate — a promoted-basis leg filed without its complete Form 8275 is a HARD
    //     refusal (Reg §1.6662-4(f)). `year: None` (a whole-range dump) means "ANY year with a promoted
    //     filed leg" (N-3), so the all-years CSV cannot smuggle an inadequately-disclosed position either.
    //  2. Attestation gate — no fictional snapshot/8949/Schedule D leaves the machine unguarded when a
    //     synthetic default contributes and the attestation is missing/wrong.
    let (events, state, _cfg) = session.load_events_and_project()?;
    promote_export_gate(&state, &events, tax_year)?;
    if state.pseudo_active() {
        require_attestation(attest)?;
    }
    // UX-P4-8: name the --out path (and hint) when the export directory cannot be created (a
    // colliding file / missing parent / permission problem), instead of a bare `io: File exists`.
    let sqlite = session
        .vault()
        .export_snapshot(out_dir)
        .map_err(|e| crate::store_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))?; // writes out_dir/snapshot.sqlite
                                                                                      // P2-D: standalone Schedule SE §1401 figure for the year-scoped export. Needs the year's filing
                                                                                      // status (profile) + the year's ss_wage_base (bundled table); `None` when either is absent or
                                                                                      // there is no business SE income. The "present but no table" note is a text-report concern
                                                                                      // (render_schedule_se) — the CSV carries the computed figure only.
    let se_result = match tax_year {
        Some(y) => {
            let tables = BundledTaxTables::load();
            // Resolve (ReturnInputs-derived → stored → …); an uncomputable/refused profile just omits the
            // SE figure — the export (a data snapshot) still proceeds, never emitting a wrong number.
            let profile = match session.resolve_screened(&state, y, &tables)? {
                crate::resolve::ProfileOutcome::Ready { profile, .. } => profile,
                crate::resolve::ProfileOutcome::Uncomputable { .. } => None,
            };
            profile.and_then(|p| {
                tables.table_for(y).and_then(|t| {
                    compute_se_tax(
                        &state,
                        y,
                        p.filing_status,
                        t,
                        p.w2_ss_wages,
                        p.w2_medicare_wages,
                        p.schedule_c_expenses,
                    )
                })
            })
        }
        None => None,
    };
    let donation_details = session.donation_details()?;
    // UX-P4-8: name the --out path for any I/O failure writing under out_dir — a `mkdir_owner_only`/
    // `open_owner_only` failure (a SUBPATH collision such as `out_dir/lots.csv` being a directory)
    // arrives as `CliError::Store(StoreError::Io)`, a mid-write `flush`/`writeln` as `CliError::Io`;
    // `cli_io_with_path` enriches BOTH. A `csv::Error` (serialization, not a path problem) passes
    // through.
    write_csv_exports(
        out_dir,
        &state,
        tax_year,
        se_result.as_ref(),
        &donation_details,
    )
    .map_err(|e| crate::cli_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))?;
    // BG-D8: emit the Form 8275 disclosure by its OWN name alongside the year-scoped packet (mirrors the
    // basis_methodology.txt emit inside write_csv_exports). The gate above already guaranteed every
    // promoted leg in the exported range carries a complete Part II.
    match tax_year {
        Some(y) => {
            crate::render::write_form_8275_txt(out_dir, &state, &events, y)
                .map_err(|e| crate::cli_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))?;
        }
        None => {
            // Task 16 / M2: the all-years dump emits promoted rows (lots/disposals.csv) for EVERY
            // promoted year in range, so it must co-emit the 8275 for every one of them too — not just
            // whichever year a `Some(y)` caller happened to name. Year-suffixed filenames (never the
            // bare `form_8275.txt`): a real vault can have promoted disposal legs in more than one tax
            // year, and the bare name would let a second year silently overwrite the first's disclosure.
            let mut promoted_years: std::collections::BTreeSet<i32> =
                std::collections::BTreeSet::new();
            for d in &state.disposals {
                if d.legs
                    .iter()
                    .any(|l| state.promoted_origins.contains(&l.lot_id.origin_event_id))
                {
                    promoted_years.insert(d.disposed_at.year());
                }
            }
            for y in promoted_years {
                crate::render::write_form_8275_txt_named(
                    out_dir,
                    &state,
                    &events,
                    y,
                    &format!("form_8275_{y}.txt"),
                )
                .map_err(|e| crate::cli_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))?;
            }
        }
    }
    // [R0-I1] Count UNRESOLVED Hard blockers only. Any Hard blocker gates every year, so the count
    // alone (no per-year `compute_tax_year` call, no profile/tables dependency) drives the main.rs
    // stderr "INFORMATIONAL, not final" disclosure. Advisory blockers never count.
    let unresolved_hard = state
        .blockers
        .iter()
        .filter(|b| b.kind.severity() == Severity::Hard)
        .count();
    Ok(ExportReport {
        path: sqlite,
        unresolved_hard,
    })
}

/// Probe: would an export be gated? `true` when the projection is pseudo-active (a synthetic default
/// contributes). Used by the `export-snapshot` CLI arm to decide whether to PROMPT for the attestation
/// phrase; the authoritative gate lives inside `export_snapshot` itself. Kept in the library so main.rs
/// stays a thin dispatch (no session-open / projection business logic in the binary).
pub fn export_pseudo_active(vault_path: &Path, pp: &Passphrase) -> Result<bool, CliError> {
    let (state, _cfg) = Session::open(vault_path, pp)?.project()?;
    Ok(state.pseudo_active())
}

/// Outcome of `export_irs_pdf`: the written PDF paths, the unresolved-Hard-blocker count (same
/// INFORMATIONAL disclosure as `export-snapshot`), whether the fill was watermarked (pseudo-active),
/// the count of rows that MIGHT belong on a separate broker-reported 8949 (the [I5] advisory — the
/// separate boxes are year-aware: A/B/D/E from a 1099-B pre-TY2025, G/H/J/K from a 1099-DA from
/// TY2025), and the SP2 packet (Schedule SE + Form 8283 + Form 1040 cap-gains) with the advisories
/// each one drives.
#[derive(Debug, Clone)]
pub struct IrsPdfReport {
    pub f8949_path: Option<PathBuf>,
    pub schedule_d_path: Option<PathBuf>,
    pub tax_year: i32,
    pub unresolved_hard: usize,
    pub broker_reported_rows: usize,
    pub watermarked: bool,
    /// Schedule SE — written only when SE income ≥ the $400 floor (and selected).
    pub schedule_se_path: Option<PathBuf>,
    /// SE tax was computed but net earnings were below the $400 floor → SE not owed, form skipped.
    pub se_below_floor: bool,
    /// `Some(addl)` when the §1401(b)(2) Additional Medicare Tax is nonzero — a Form 8959 item, NOT on
    /// Schedule SE (a loud advisory).
    pub se_addl_medicare: Option<Usd>,
    /// SE-eligible business income exists but no profile/table was available (`compute_se_tax` → None
    /// for a reason other than "no SE income") → a NOTE, not a silent skip (the `se_net_income`
    /// discriminator).
    pub se_income_without_profile: bool,
    /// Form 8283 — written only when there are donations (and selected).
    pub form_8283_path: Option<PathBuf>,
    /// Any Form 8283 row needs manual review (incomplete appraiser/donee declaration) → escalate.
    pub form_8283_needs_review: bool,
    /// The Form 8283 section actually written (`Some(true)` = Section B, `Some(false)` = Section A).
    pub form_8283_section_b: Option<bool>,
    /// Form 1040 cap-gains — written only when there is reportable digital-asset activity (and selected).
    pub form_1040_path: Option<PathBuf>,
    /// Line 7a received a value on the written 1040.
    pub form_1040_filled_7a: bool,
    /// The 1040 was skipped for a NET LOSS on line 7a (the §1211 line-21 cap is the filer's).
    pub form_1040_loss: bool,
    /// Form 8275 (Disclosure Statement) — Task 16: written only on the crypto-slice path, only when a
    /// promoted-basis disposal leg files in `tax_year` (and selected). Always `None` on the full-return
    /// path — its 8275 is inside `full_return_paths` instead (sequence-prefixed, e.g. `92_f8275.pdf`).
    pub form_8275_path: Option<PathBuf>,
    /// ★ The FULL-RETURN packet's files, in Attachment Sequence order (empty on the crypto-slice path).
    /// The two paths write NON-OVERLAPPING names, so no two runs can be collated into a chimera return.
    pub full_return_paths: Vec<PathBuf>,
    /// The full-return packet's manifest (the filer's stapling order).
    pub full_return_manifest: Option<PathBuf>,
    /// UX-P4-5: `true` when a `--forms` SLICE was passed on a full-return year and therefore IGNORED
    /// (the whole jointly-computed packet writes; honoring a slice of it is tax-unsound). The caller
    /// warns on stderr. Always `false` on the crypto-slice path (there `--forms` is honored).
    pub forms_ignored_full_return: bool,
}

/// The **[I5]** broker-reporting advisory line, year-aware — or `None` when no disposition may have
/// been broker-reported (`broker_reported_rows == 0`).
///
/// The "separate 8949 / not-reported box" pairing depends on the form revision, exactly as the box
/// assignment does (mirrors [`btctax_core::DIGITAL_ASSET_8949_FIRST_YEAR`]): pre-TY2025 an exchange
/// disposal may have been reported on a **1099-B**, belongs on a separate 8949 under **Box A/B (ST) /
/// D/E (LT)**, and this export files every row under **Box C/F**; from TY2025 it is the **1099-DA**,
/// **Box G/H/J/K**, and every row files under **Box I/L**. Emitting the 2025 pairing on a pre-2025
/// export would steer the filer to boxes that do not exist on that revision — hence the year gate.
pub fn broker_reporting_advisory(tax_year: i32, broker_reported_rows: usize) -> Option<String> {
    if broker_reported_rows == 0 {
        return None;
    }
    let (broker_form, separate_boxes, filed_boxes) = if tax_year >= DIGITAL_ASSET_8949_FIRST_YEAR {
        ("1099-DA", "Box G/H/J/K", "Box I/L")
    } else {
        ("1099-B", "Box A/B (ST) / D/E (LT)", "Box C/F")
    };
    Some(format!(
        "⚠ [I5] {broker_reported_rows} disposition(s) occurred on an exchange that MAY have issued \
         {broker_form} broker basis reporting — those would belong on a SEPARATE Form 8949 under \
         {separate_boxes}. This export files EVERY Bitcoin row under {filed_boxes} (not-reported \
         default) and says so; reclassify by hand if you received a {broker_form}."
    ))
}

/// Whether a form is included: the packet is every applicable form unless `--forms` opts in to a subset.
fn wants(selected: &[FormArg], f: FormArg) -> bool {
    selected.is_empty() || selected.contains(&f)
}

/// A Schedule D part is "active" (worth reporting) iff it has any proceeds/cost/gain.
fn sd_part_active(p: &ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

/// `export-irs-pdf`: fill the OFFICIAL IRS PDFs for `tax_year` and write them (owner-only) to
/// `out_dir`. The packet is Form 8949 + Schedule D (always applicable) plus — when applicable and
/// selected — Schedule SE (SE income ≥ $400), Form 8283 (donations), and Form 1040 cap-gains
/// (reportable digital-asset activity). The form data is REUSED from the projection
/// (`form_8949`/`schedule_d`/`form_8283`/`compute_se_tax`) — nothing capital-gains is recomputed; the
/// SE §1401 figure is computed here from the year's stored `TaxProfile`. Same pseudo-active
/// attestation gate as `export-snapshot`: checked FIRST, so a refused export leaves `out_dir`
/// untouched; a pseudo fill is additionally DRAFT-watermarked.
pub fn export_irs_pdf(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: i32,
    forms: &[FormArg],
    attest: Option<&str>,
) -> Result<IrsPdfReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;

    // ★ THE DISPATCH (P6.5). Exactly one function decides which pipeline runs, and the two write
    // NON-OVERLAPPING filenames, so artifacts from two runs can never be collated into a chimera
    // return: the full packet writes `f1040.pdf`, `f1040s1.pdf`, … + a manifest; the crypto slice
    // writes `form_1040_capgains.pdf`, `schedule_d.pdf`, `f8949.pdf`, ….
    //
    // This REPLACES the P5-C1 refusal (`CryptoSliceExportForFullReturnYear`). That guard existed only
    // because the slice's Schedule D carries the crypto totals alone — no line 13 for 1099-DIV box-2a
    // capital-gain distributions, no lines 6/14 for capital-loss carryovers — so on a full-return year
    // it was a complete-LOOKING form with income missing. The full pipeline fills all of them, plus
    // every attachment the forms cite, so the reason for the refusal is gone. Deleting it downgrades a
    // type-level impossibility to a branch, which is why the branch is HERE, alone, and pinned by KATs
    // in BOTH directions.
    if crate::return_inputs::exists(session.conn(), tax_year)? {
        // The full-return pipeline runs the BG-D8 gate itself (checked first there too).
        let mut report = export_full_return(&session, &state, &events, out_dir, tax_year, attest)?;
        // UX-P4-5: a --forms slice cannot be honored on a full-return year (the 14-form packet is
        // jointly computed; a slice of it is tax-unsound). The packet still writes in full; flag the
        // ignored slice so the caller warns.
        report.forms_ignored_full_return = !forms.is_empty();
        return Ok(report);
    }

    // BG-D8 completeness gate (crypto-slice path) — a promoted-basis leg without its complete Form 8275
    // is a HARD refusal, checked FIRST (before the pseudo watermark check and any byte written).
    promote_export_gate(&state, &events, Some(tax_year))?;

    // Attestation gate — no fictional tax form leaves the machine unguarded, and a refusal
    // writes no bytes. (A fully-real ledger ignores `attest`.)
    let watermarked = state.pseudo_active();
    if watermarked {
        require_attestation(attest)?;
    }

    // Reuse the projection's capital-gains data verbatim (no recompute).
    let rows = btctax_core::form_8949(&state, tax_year);
    let totals = btctax_core::schedule_d(&state, tax_year);

    // Form 8275 (Disclosure Statement) — Task 16: `Some` iff a promoted-basis disposal leg files in
    // `tax_year` (the same `disclosure_8275` scoping `promote_export_gate` above already used to confirm
    // completeness).
    let printed_8275 = btctax_core::tax::form8275::disclosure_8275(&events, &state, tax_year)
        .map(|d| btctax_core::tax::printed::printed_8275(&d));
    // Task 16 / ADD-2 (mirrors `export_full_return`'s pre-check below): v1 does not paginate Form 8275 —
    // refuse HERE, before `mkdir_out`, so an overflowing year (> 6 promoted disposal legs) names the year
    // + a concrete remedy and writes ZERO bytes, instead of a bare `FormsError::Overflow` display after
    // other files (`basis_methodology.txt`, `form_8275.txt`) already exist on disk.
    if let Some(p) = &printed_8275 {
        let cap = btctax_forms::Form8275Map::for_year(tax_year)?.rows.len();
        if p.part_i.len() > cap {
            return Err(CliError::Usage(format!(
                "cannot export {tax_year}: {n} promoted disposal leg(s) each need a Form 8275 Part I \
                 row, but this revision holds only {cap} — Form 8275 cannot yet paginate beyond {cap} \
                 rows. File the 8275 manually for {tax_year}, or reduce the number of promoted disposal \
                 legs filed in {tax_year} (e.g. void one of the promotes) and re-export.",
                n = p.part_i.len(),
            )));
        }
    }

    // A pseudo-active fill DRAFT-watermarks every page before it hits disk.
    let stamp = |bytes: Vec<u8>| -> Result<Vec<u8>, CliError> {
        Ok(if watermarked {
            btctax_forms::stamp_draft_watermark(&bytes)?
        } else {
            bytes
        })
    };

    mkdir_out(out_dir)?;

    // I-3 (D-4): the MANDATORY conservative-filing methodology disclosure rides the PDF packet too, not
    // just the CSV paths — a filer mailing the flagship filing-ready artifact must get the i8949-required
    // basis explanation whenever a $0-basis tranche row is present. Writes nothing for a no-tranche year.
    crate::render::write_basis_methodology_txt(out_dir, &state, tax_year)?;
    // BG-D8: the Form 8275 disclosure rides the packet by its OWN name. The gate above guaranteed a
    // promoted leg reaching here has a complete Part II. Writes nothing for a no-promoted-leg year.
    crate::render::write_form_8275_txt(out_dir, &state, &events, tax_year)?;

    // ── Form 8949 + Schedule D (always applicable). ──
    let f8949_path = if wants(forms, FormArg::F8949) {
        let bytes = stamp(btctax_forms::fill_form_8949(&rows, tax_year)?)?;
        let path = out_dir.join("f8949.pdf");
        write_bytes_owner_only(&path, &bytes)?;
        Some(path)
    } else {
        None
    };
    let schedule_d_path = if wants(forms, FormArg::ScheduleD) {
        let bytes = stamp(btctax_forms::fill_schedule_d(&totals, tax_year)?)?;
        let path = out_dir.join("schedule_d.pdf");
        write_bytes_owner_only(&path, &bytes)?;
        Some(path)
    } else {
        None
    };

    // ── Schedule SE (self-employment tax). Compute the §1401 figure from the year's TaxProfile. ──
    let se_computed = {
        let tables = BundledTaxTables::load();
        let profile = match session.resolve_screened(&state, tax_year, &tables)? {
            crate::resolve::ProfileOutcome::Ready { profile, .. } => profile,
            crate::resolve::ProfileOutcome::Uncomputable { .. } => None, // export proceeds; SE omitted
        };
        profile.and_then(|p| {
            tables.table_for(tax_year).and_then(|t| {
                compute_se_tax(
                    &state,
                    tax_year,
                    p.filing_status,
                    t,
                    p.w2_ss_wages,
                    p.w2_medicare_wages,
                    p.schedule_c_expenses,
                )
                .map(|se| (se, p.w2_ss_wages, t.ss_wage_base))
            })
        })
    };
    // Discriminator: SE income present but `compute_se_tax` returned None (no profile / no table) → a
    // NOTE, not a silent skip (mirrors the render layer; never a fabricated form).
    let se_income_without_profile =
        se_computed.is_none() && !se_net_income(&state, tax_year).is_zero();
    let mut schedule_se_path = None;
    let mut se_below_floor = false;
    let mut se_addl_medicare = None;
    if wants(forms, FormArg::ScheduleSe) {
        if let Some((se, w2_ss_wages, ss_wage_base)) = se_computed {
            if !se.addl.is_zero() {
                se_addl_medicare = Some(se.addl);
            }
            match btctax_forms::fill_schedule_se(&se, w2_ss_wages, ss_wage_base, tax_year)? {
                Some(bytes) => {
                    let bytes = stamp(bytes)?;
                    let path = out_dir.join("schedule_se.pdf");
                    write_bytes_owner_only(&path, &bytes)?;
                    schedule_se_path = Some(path);
                }
                None => se_below_floor = true, // below the $400 floor — SE not owed.
            }
        }
    }

    // ── Form 8283 (noncash charitable contributions). ──
    let mut form_8283_path = None;
    let mut form_8283_needs_review = false;
    let mut form_8283_section_b = None;
    if wants(forms, FormArg::Form8283) {
        let details = session.donation_details()?;
        let rows_8283 = btctax_core::form_8283(&state, tax_year, &details);
        if let Some(bytes) = btctax_forms::fill_form_8283(&rows_8283, tax_year)? {
            form_8283_needs_review = rows_8283.iter().any(|r| r.needs_review);
            form_8283_section_b = rows_8283
                .iter()
                .find_map(|r| r.section)
                .map(|s| s == btctax_core::Form8283Section::B);
            let bytes = stamp(bytes)?;
            let path = out_dir.join("form_8283.pdf");
            write_bytes_owner_only(&path, &bytes)?;
            form_8283_path = Some(path);
        }
    }

    // ── Form 1040 cap-gains cells + the digital-asset question. ──
    let mut form_1040_path = None;
    let mut form_1040_filled_7a = false;
    let mut form_1040_loss = false;
    if wants(forms, FormArg::Form1040) {
        let da_yes = !rows.is_empty()
            || state
                .income_recognized
                .iter()
                .any(|i| i.recognized_at.year() == tax_year)
            || state
                .removals
                .iter()
                .any(|r| r.removed_at.year() == tax_year);
        let inputs = Form1040Inputs {
            da_yes,
            schedule_d_active: sd_part_active(&totals.st) || sd_part_active(&totals.lt),
            schedule_d_line16: totals.st.gain + totals.lt.gain,
        };
        if let Some(fill) = btctax_forms::fill_form_1040_capgains(&inputs, tax_year)? {
            form_1040_filled_7a = fill.filled_7a;
            form_1040_loss = fill.loss;
            let bytes = stamp(fill.pdf)?;
            let path = out_dir.join("form_1040_capgains.pdf");
            write_bytes_owner_only(&path, &bytes)?;
            form_1040_path = Some(path);
        }
    }

    // ── Form 8275 (Disclosure Statement) — the OFFICIAL PDF, crypto-slice fill (Task 16). Rides beside
    // the `write_form_8275_txt` content emitted above; no filer identity (mirrors the Form 8283
    // crypto-slice fill). `promote_export_gate` already guaranteed a complete Part II, and the overflow
    // pre-check above already guaranteed the Part I rows fit this revision's capacity. ──
    // BG-D8 (whole-branch tax M-1): the Form 8275 is the MANDATORY disclosure that must travel WITH the
    // promoted 8949 position — so it rides UNCONDITIONALLY whenever a promoted disposal leg is filed,
    // NOT behind `wants(forms, Form8275)`. Otherwise `--forms f8949` would export the estimate position
    // without its official disclosure PDF (Reg §1.6662-4(f) makes disclosure adequate only on a COMPLETED
    // Form 8275). This mirrors the always-emitted `form_8275.txt` and the unconditional overflow refusal:
    // the disclosure is never a `--forms`-narrowable slice. (`printed_8275` is `Some` iff a promoted
    // disposal leg files this year; a non-promoted export writes no 8275.)
    let mut form_8275_path = None;
    if let Some(p) = &printed_8275 {
        if let Some(bytes) = btctax_forms::fill_form_8275_slice(p, tax_year)? {
            let bytes = stamp(bytes)?;
            let path = out_dir.join("form_8275.pdf");
            write_bytes_owner_only(&path, &bytes)?;
            form_8275_path = Some(path);
        }
    }

    let unresolved_hard = state
        .blockers
        .iter()
        .filter(|b| b.kind.severity() == Severity::Hard)
        .count();
    Ok(IrsPdfReport {
        full_return_paths: Vec::new(),
        full_return_manifest: None,
        forms_ignored_full_return: false, // crypto-slice path honors --forms
        f8949_path,
        schedule_d_path,
        tax_year,
        unresolved_hard,
        broker_reported_rows: btctax_forms::rows_possibly_broker_reported(&rows),
        watermarked,
        schedule_se_path,
        se_below_floor,
        se_addl_medicare,
        se_income_without_profile,
        form_8283_path,
        form_8283_needs_review,
        form_8283_section_b,
        form_1040_path,
        form_1040_filled_7a,
        form_1040_loss,
        form_8275_path,
    })
}

/// Write `bytes` to `path` with owner-only (0o600) permissions, matching the CSV export path.
fn write_bytes_owner_only(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    use std::io::Write;
    let mut f = fsperms::open_owner_only(path)?;
    f.write_all(bytes)?;
    f.flush()?;
    Ok(())
}

/// UX-P4-8: create the export `--out` directory, NAMING the path + a one-clause hint when it cannot be
/// created (a colliding file, a missing parent, a permission problem) — instead of the bare
/// `io: File exists (os error 17)` the pathless `StoreError::Io` produces. The single choke point for
/// every directory-producing export (`export-snapshot` via `write_csv_exports`, `export-irs-pdf`,
/// `export-full-return`).
fn mkdir_out(out_dir: &Path) -> Result<(), CliError> {
    fsperms::mkdir_owner_only(out_dir)
        .map_err(|e| crate::store_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))
}

/// §8: export the passphrase-protected key (escape hatch; HIGH-security write).
pub fn backup_key(vault_path: &Path, pp: &Passphrase, out_path: &Path) -> Result<(), CliError> {
    Session::open(vault_path, pp)?
        .vault()
        .backup_key(out_path)
        // UX-P4-8: name the --out path (and hint) when the key file cannot be written (a colliding
        // directory, a missing parent, a permission problem), not a bare `io: …`.
        .map_err(|e| crate::store_io_with_path(e, out_path, crate::EXPORT_OUT_HINT))?;
    Ok(())
}

/// ★ **The full-return export** (P6.3b / P6.5) — the whole filable packet, not the crypto slice.
///
/// Runs the same fail-closed screens the report runs (a return the report will not compute is a return
/// the exporter must not print), assembles the printed packet in CORE, and fills it ALL-OR-NOTHING: if
/// any member form refuses, zero bytes reach the disk. A 1040 whose line 2b cites a Schedule B that is
/// not attached is a wrong return, so partial emission would be a fail-open.
///
/// **The packet exports CLEAN** (no DRAFT watermark, no attestation) — the user's §9 decision, folded
/// into the SPEC. The one exception is PSEUDO-reconciled figures, which are FICTIONAL and can never be
/// filed: those are watermarked regardless, and that gate composes with (and dominates) everything else.
fn export_full_return(
    session: &Session,
    state: &btctax_core::state::LedgerState,
    events: &[LedgerEvent],
    out_dir: &Path,
    tax_year: i32,
    attest: Option<&str>,
) -> Result<IrsPdfReport, CliError> {
    use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
    use btctax_core::tax::tables::{FullReturnTables, TaxTables};
    use std::fmt::Write as _;

    // BG-D8 completeness gate — checked FIRST (before the tables lookup, the fail-closed screens, and any
    // byte written): a promoted-basis leg without its complete Form 8275 is a HARD refusal.
    promote_export_gate(state, events, Some(tax_year))?;

    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let (Some(params), Some(table)) = (
        fr_tables.full_return_for(tax_year),
        tables.table_for(tax_year),
    ) else {
        return Err(CliError::Usage(format!(
            "no full-return tables for {tax_year} — the full-return packet needs a supported tax year \
             (TY2024)"
        )));
    };

    let ri = crate::return_inputs::get(session.conn(), tax_year)?
        .ok_or_else(|| CliError::Usage(format!("no return_inputs stored for {tax_year}")))?;

    // Fail-closed screens, in the same order the report runs them. A refusal writes NO bytes.
    let refuse = |r: btctax_core::tax::return_refuse::Refusal| {
        CliError::Usage(format!(
            "the {tax_year} return is not computable [{:?}]: {} — no forms were written",
            r.reason, r.detail
        ))
    };
    if let Some(r) = btctax_core::tax::return_refuse::screen_inputs(&ri, table, params) {
        return Err(refuse(r));
    }
    if let Some(r) =
        btctax_core::tax::return_1040::screen_compute_dependent(&ri, state, tax_year, params)
    {
        return Err(refuse(r));
    }
    let ar = btctax_core::tax::return_1040::assemble_absolute(&ri, state, params, table, tax_year);
    if let Some(r) = btctax_core::tax::return_1040::screen_absolute(&ri, &ar, params) {
        return Err(refuse(r));
    }

    // Pseudo figures are FICTIONAL: they are watermarked no matter what, and the attestation gate for
    // them is unchanged. A clean (real-ledger) packet needs no attestation — SPEC §9 as amended.
    let watermarked = state.pseudo_active();
    if watermarked {
        require_attestation(attest)?;
    }

    let details = session.donation_details()?;
    let printed = btctax_core::tax::packet::assemble_printed_return(
        &ri, state, &details, &ar, table, tax_year, events,
    )
    .map_err(|e| {
        // `HeaderError`'s Display carries the right remedy per variant (a malformed SSN, an unanswered
        // declaration, or an MFJ return with no spouse) — no longer always "fix the identity" (P9 §3.2).
        CliError::Usage(format!("the {tax_year} return cannot be printed: {e}"))
    })?;

    // Task 16 / ADD-2: Form 8275 v1 does not paginate (unlike Form 8283's `overflow::merge_copies`) — a
    // promoted year with more than the revision's Part I row capacity (6 rows) cannot be filled at all.
    // Refuse HERE, before the whole-packet fill, so the failure names the year + a concrete remedy
    // instead of surfacing as a bare `FormsError::Overflow` display deep inside an all-or-nothing fill.
    if let Some(f8275) = &printed.forms.f8275 {
        let cap = btctax_forms::Form8275Map::for_year(tax_year)?.rows.len();
        if f8275.part_i.len() > cap {
            return Err(CliError::Usage(format!(
                "cannot export {tax_year}: {n} promoted disposal leg(s) each need a Form 8275 Part I \
                 row, but this revision holds only {cap} — Form 8275 cannot yet paginate beyond {cap} \
                 rows. File the 8275 manually for {tax_year}, or reduce the number of promoted disposal \
                 legs filed in {tax_year} (e.g. void one of the promotes) and re-export.",
                n = f8275.part_i.len(),
            )));
        }
    }

    // ★ ALL-OR-NOTHING: every form fills BEFORE anything is written.
    let packet = btctax_forms::fill_full_return(&printed, tax_year)?;

    mkdir_out(out_dir)?;
    // I-3 (D-4): the MANDATORY methodology disclosure rides the full-return packet too (see export_irs_pdf).
    crate::render::write_basis_methodology_txt(out_dir, state, tax_year)?;
    // BG-D8: the Form 8275 disclosure rides the full-return packet by its OWN name (gate above guaranteed
    // a complete Part II). Writes nothing for a no-promoted-leg year.
    crate::render::write_form_8275_txt(out_dir, state, events, tax_year)?;
    let mut manifest = String::from("# btctax full-return packet — staple in this order\n");
    let mut paths: Vec<PathBuf> = Vec::new();
    for form in &packet {
        let bytes = if watermarked {
            btctax_forms::stamp_draft_watermark(&form.bytes)?
        } else {
            form.bytes.clone()
        };
        // ★ The packet's filenames are SEQUENCE-PREFIXED (`00_f1040.pdf`, `12A_f8949.pdf`, …). Two
        // reasons, and the first is a correctness guarantee: the crypto slice writes bare stems
        // (`f8949.pdf`, `schedule_d.pdf`, `schedule_se.pdf`) and THREE of them collided with the
        // packet's — so a full-return run and a slice run into one directory could silently interleave
        // a cents Schedule SE with a whole-dollar 1040: the chimera return the dispatch mitigation
        // exists to prevent (Fable P6 r1 I7). Second, the prefix IS the stapling order.
        let path = out_dir.join(format!(
            "{}_{}.pdf",
            form.attachment_sequence.unwrap_or("00"),
            form.name
        ));
        write_bytes_owner_only(&path, &bytes)?;
        let seq = form.attachment_sequence.unwrap_or("—");
        let _ = writeln!(
            manifest,
            "{seq:>4}  {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        paths.push(path);
    }
    let manifest_path = out_dir.join("manifest.txt");
    write_bytes_owner_only(&manifest_path, manifest.as_bytes())?;

    let unresolved_hard = state
        .blockers
        .iter()
        .filter(|b| b.kind.severity() == Severity::Hard)
        .count();
    Ok(IrsPdfReport {
        watermarked,
        tax_year,
        unresolved_hard,
        broker_reported_rows: 0,
        full_return_paths: paths,
        full_return_manifest: Some(manifest_path),
        forms_ignored_full_return: false, // set by the dispatch (which has `forms`), not here
        // The crypto-slice PATHS are absent (the two pipelines are disjoint), but the 8283's loud
        // escalations are NOT slice-specific: the packet can contain a Section-B 8283 whose appraiser
        // declaration is unsigned, and silencing that guard on the one path that announces a "clean,
        // filable" packet would be a fail-open (Fable P6 r1 I8b).
        f8949_path: None,
        schedule_d_path: None,
        schedule_se_path: None,
        se_below_floor: false,
        se_addl_medicare: None,
        se_income_without_profile: false,
        form_8283_path: None,
        form_8283_needs_review: printed
            .forms
            .f8283
            .as_ref()
            .is_some_and(|r| r.rows().iter().any(|row| row.needs_review)),
        form_8283_section_b: printed.forms.f8283.as_ref().map(|r| {
            r.rows()
                .iter()
                .any(|row| row.section == Some(btctax_core::Form8283Section::B))
        }),
        form_1040_path: None,
        form_1040_filled_7a: false,
        form_1040_loss: false,
        // The full-return path's 8275 (when present) is inside `full_return_paths` — sequence-prefixed
        // (`92_f8275.pdf`), not this crypto-slice-only bare-named field.
        form_8275_path: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [I5] r2/NEW-IMPORTANT-1: the broker-reporting advisory is YEAR-AWARE. On the TY2025+
    /// digital-asset revision it must cite the 1099-DA and the digital-asset boxes (G/H/J/K separate,
    /// I/L filed) — never the securities boxes.
    #[test]
    fn broker_advisory_ty2025_cites_1099da_and_digital_asset_boxes() {
        let msg = broker_reporting_advisory(2025, 3).expect("3 broker rows → an advisory");
        assert!(msg.contains("1099-DA"), "TY2025 cites the 1099-DA: {msg}");
        assert!(msg.contains("Box G/H/J/K"), "separate 8949 boxes: {msg}");
        assert!(msg.contains("Box I/L"), "filed-under boxes: {msg}");
        assert!(msg.contains('3'), "the row count: {msg}");
        // The securities-era pairing must NOT leak onto a 2025 export.
        assert!(
            !msg.contains("1099-B"),
            "no pre-2025 1099-B on TY2025: {msg}"
        );
        assert!(
            !msg.contains("Box C/F"),
            "no securities C/F on TY2025: {msg}"
        );
    }

    /// [I5] r2/NEW-IMPORTANT-1: pre-TY2025 (the securities-box revisions — TY2024/TY2017 are shipped
    /// export years) the advisory must cite the 1099-B and the securities boxes (A/B, D/E separate,
    /// C/F filed) — boxes G–L do not exist on those form revisions.
    #[test]
    fn broker_advisory_pre_2025_cites_1099b_and_securities_boxes() {
        let msg = broker_reporting_advisory(2024, 1).expect("1 broker row → an advisory");
        assert!(msg.contains("1099-B"), "pre-2025 cites the 1099-B: {msg}");
        assert!(msg.contains("Box A/B"), "separate ST securities box: {msg}");
        assert!(msg.contains("D/E"), "separate LT securities box: {msg}");
        assert!(
            msg.contains("Box C/F"),
            "filed-under securities boxes: {msg}"
        );
        // The digital-asset-era pairing must NOT leak onto a pre-2025 export.
        assert!(!msg.contains("1099-DA"), "no 1099-DA pre-2025: {msg}");
        assert!(!msg.contains("G/H/J/K"), "no digital boxes pre-2025: {msg}");
        assert!(
            !msg.contains("Box I/L"),
            "no digital filed-boxes pre-2025: {msg}"
        );
    }

    /// [I5]: no exchange disposition → no advisory, in either era.
    #[test]
    fn broker_advisory_is_none_without_broker_rows() {
        assert!(broker_reporting_advisory(2025, 0).is_none());
        assert!(broker_reporting_advisory(2024, 0).is_none());
    }

    /// UX-P4-8 (fold I2): `mkdir_out` — the shared export-`--out` directory creator — names the
    /// offending PATH and the remedy HINT when the directory cannot be created (here: an `--out`
    /// that collides with a plain file), instead of a bare `io: File exists`.
    #[test]
    fn mkdir_out_collision_names_path_and_hint() {
        let dir = tempfile::tempdir().unwrap();
        let collide = dir.path().join("collide");
        std::fs::write(&collide, b"i am a file, not a directory").unwrap();
        let err = mkdir_out(&collide).expect_err("a file collision must error");
        match &err {
            CliError::PathIo { path, hint, .. } => {
                assert!(path.contains("collide"), "names the path: {path}");
                assert_eq!(hint, crate::EXPORT_OUT_HINT, "carries the export-out hint");
            }
            other => panic!("expected PathIo, got {other:?}"),
        }
        let msg = err.to_string();
        assert!(msg.contains("collide"), "Display names the path: {msg}");
        // Literal (not self-referential to the const, which `contains("")` would trivially satisfy if
        // the const were emptied) — pins the hint's CONTENT (fold r2-N2).
        assert!(
            msg.contains("does not already exist as a file"),
            "Display carries the hint content: {msg}"
        );
    }
}
