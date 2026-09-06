//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the pre-2025 lot method; export/backup arrive in Task 15.
use crate::cli::FormArg;
use crate::config::{set_fee_treatment, set_pre2025_method as config_set_pre2025_method};
use crate::render::write_csv_exports;
use crate::{require_attestation, CliConfig, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    compute_se_tax, se_net_income, FeeTreatment, LedgerEvent, LotMethod, ScheduleDPart, Severity,
    TaxTables, Usd,
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
    /// Approach-B experimental disclosure (`design/approach-b-experimental-notice`, fix round 1
    /// Important #4): `btctax_core::experimental::uses_approach_b(events)` on the projected events —
    /// `true` iff a live (non-voided) DeclareTranche/PromoteTranche is on file. `export-snapshot` writes
    /// the same `form_8275.txt`/`basis_methodology.txt` disclosure files `export-irs-pdf` does (this is
    /// the CSV/preparer-handoff path), so it gets the SAME stderr notice (main.rs). Interface-only — the
    /// notice is never written to `out_dir`.
    pub experimental_notice_active: bool,
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
    // which a promoted disposal leg files. ★ Task 3 (arch-m-2/DFW-D11): the `None` arm's enumeration is
    // single-sourced from `chokepoint::promoted_filing_years` — the SAME 8275-completeness set, never
    // duplicated here (and NOT the fold-diff export set, which is strictly larger — see `flagged_years`).
    let years: Vec<i32> = match year {
        Some(y) => vec![y],
        None => crate::chokepoint::promoted_filing_years(state)
            .into_iter()
            .collect(),
    };
    for y in years {
        // `disclosure_8275` is `Some` iff a promoted DISPOSAL leg files in `y`; refuse when its Part II is
        // empty/incomplete (the `incomplete` flag T13 exposes for exactly this gate).
        // ★ FR-29: `None`. This gate exists to refuse a PROMOTED leg whose Part II narrative the
        //   filer never recorded. A §1(g) no-path item carries a Part II btctax writes itself
        //   (`section_1g_part_ii`), so it can never be incomplete and has nothing to gate.
        if let Some(disc) = btctax_core::tax::form8275::disclosure_8275(events, state, y, None) {
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
    //  3. ★ FR-48 / port report §2.5: `export-snapshot` was unstamped — TY2026's and TY2099's
    //     `form8949.csv` were byte-identical. The year and its readiness are stamped into the export
    //     directory below (`TAX_YEAR.txt`). NOT gated: this is a data export, valid for any year the
    //     ledger holds (a filed-tranche TY2020 exports with no table bundled) — see `export_stamp`.
    let stamp = match tax_year {
        Some(y) => crate::year_readiness::export_stamp(y),
        None => format!(
            "all promoted filing years; this build bundles {}",
            btctax_forms::bundled::years_sentence()
        ),
    };
    // UX-P4-8: name the --out path (and hint) when the export directory cannot be created (a
    // colliding file / missing parent / permission problem), instead of a bare `io: File exists`.
    let sqlite = session
        .vault()
        .export_snapshot(out_dir)
        .map_err(|e| crate::store_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))?; // writes out_dir/snapshot.sqlite
    write_bytes_owner_only(
        &out_dir.join("TAX_YEAR.txt"),
        format!("{stamp}\n").as_bytes(),
    )?;
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
    // spec 1099-DA — the year's regime and the stored answers (if any) route the 8949 CSV's boxes
    // ★ spec 1099-DA T9 — the answers come from the ONE resolution every surface reads (a draft
    //   shadows the committed row; a parked draft carries none), never `return_inputs::get`: the CSV
    //   a filer hands a preparer must show the same boxes the PDF export files.
    let broker = match tax_year {
        Some(y) => Some((
            crate::year_readiness::regime_or_pre_regime(y)?,
            crate::input_form_store::broker_answers(session.conn(), y)?,
        )),
        None => None,
    };
    write_csv_exports(
        out_dir,
        &state,
        tax_year,
        se_result.as_ref(),
        &donation_details,
        broker.as_ref().map(|(r, a)| (*r, a.as_ref())),
    )
    .map_err(|e| crate::cli_io_with_path(e, out_dir, crate::EXPORT_OUT_HINT))?;
    // BG-D8: emit the Form 8275 disclosure by its OWN name alongside the year-scoped packet (mirrors the
    // basis_methodology.txt emit inside write_csv_exports). The gate above already guaranteed every
    // promoted leg in the exported range carries a complete Part II.
    match tax_year {
        Some(y) => {
            // ★ FR-29: `None` — this is the CSV snapshot, which produces no Form 1040.
            crate::render::write_form_8275_txt(out_dir, &state, &events, y, None)
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
                    None, // ★ FR-29: the CSV snapshot produces no Form 1040 (see above).
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
        experimental_notice_active: btctax_core::experimental::uses_approach_b(&events),
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

/// The **Form 1040-V** decision the filer makes at export time (spec R4).
///
/// It is a CHOICE, never an inference. Most filers who owe pay online — *"Save time by paying online."*
/// says the voucher's own page 2 — and IRS Direct Pay / EFTPS need no voucher at all; printing one
/// unasked would put a page in the envelope that the return does not need and the form says not to
/// staple. So the default is *no voucher, and a note saying it exists*.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VoucherChoice {
    /// `--pay-by-check` — the filer is enclosing a check or money order, so print Form 1040-V.
    pub pay_by_check: bool,
    /// `--pay <whole dollars>` — a PARTIAL payment, overriding Form 1040 line 37 in box 3.
    ///
    /// ★ Its ceiling is line 37, and that asymmetry with `btctax extension --pay` is deliberate (spec,
    /// box 3): the voucher pays a COMPUTED balance, so more than it is a filer error rather than a
    /// choice the form names — while the 4868's line 7 is an estimate the filer may deliberately
    /// overshoot to limit interest.
    pub pay: Option<Usd>,
}

/// Outcome of `export_irs_pdf`: the written PDF paths, the unresolved-Hard-blocker count (same
/// INFORMATIONAL disclosure as `export-snapshot`), whether the fill was watermarked (pseudo-active),
/// the count of rows that MIGHT belong on a separate broker-reported 8949 (the [I5] advisory — the
/// separate boxes are year-aware: A/B/D/E from a 1099-B pre-TY2025, G/H/J/K from a 1099-DA from
/// TY2025), and the SP2 packet (Schedule SE + Form 8283 + Form 1040 cap-gains) with the advisories
/// each one drives.
#[derive(Debug, Clone)]
pub struct IrsPdfReport {
    /// ★ spec 1099-DA R6 (M-6) — the CRYPTO-SLICE-FILED-FROM-ANSWERS note, printed AFTER the file
    /// list. `Some` only on arm (2): the year's answers are stored, its full-return parameters are
    /// not bundled, and what was written is an ATTACHMENT SET rather than a return.
    pub slice_attachment_note: Option<String>,
    /// ★ spec 1099-DA R6 (M-14) — the §6.3 [`crate::input_form_store::StaleNote`], rendered the way
    /// `scrub` renders it, for printing BEFORE the file list: a schema-stale WIP draft was skipped,
    /// so the answers filed came from the committed row (or none). `None` on every other path.
    pub stale_draft_note: Option<String>,
    pub f8949_path: Option<PathBuf>,
    pub schedule_d_path: Option<PathBuf>,
    pub tax_year: i32,
    pub unresolved_hard: usize,
    pub broker_reported_rows: usize,
    /// spec 1099-DA — the year's Form 1099-DA regime, joined from its record; the [I5] advisory's
    /// wording follows it, never the constant.
    pub regime: btctax_core::forms::InformationReturnRegime,
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
    /// ★ **Form 1040-V** (spec R4) — the payment voucher, written BESIDE the packet when the return
    /// owes (`line37 > 0`) and the filer passed `--pay-by-check`. Deliberately NOT in
    /// [`Self::full_return_paths`]: that list is the STAPLING ORDER, and the voucher is enclosed
    /// LOOSE (*"Do not staple or attach this voucher to your payment or return."*).
    pub form_1040v_path: Option<PathBuf>,
    /// The voucher NOTE, when there is one to give: the return owes but no `--pay-by-check` was
    /// passed (Direct Pay / EFTPS need no voucher), or the flag was passed on a return that owes
    /// nothing. `None` when neither applies. Silence about a balance due is the one answer a tax tool
    /// may not give.
    pub form_1040v_note: Option<String>,
    /// ★ The FULL-RETURN packet's files, in Attachment Sequence order (empty on the crypto-slice path).
    /// The two paths write NON-OVERLAPPING names, so no two runs can be collated into a chimera return.
    pub full_return_paths: Vec<PathBuf>,
    /// The full-return packet's manifest (the filer's stapling order).
    pub full_return_manifest: Option<PathBuf>,
    /// UX-P4-5: `true` when a `--forms` SLICE was passed on a full-return year and therefore IGNORED
    /// (the whole jointly-computed packet writes; honoring a slice of it is tax-unsound). The caller
    /// warns on stderr. Always `false` on the crypto-slice path (there `--forms` is honored).
    pub forms_ignored_full_return: bool,
    /// Approach-B experimental disclosure (`design/approach-b-experimental-notice`):
    /// `btctax_core::experimental::uses_approach_b(events)` on the projected events — `true` iff a live
    /// (non-voided) DeclareTranche/PromoteTranche is on file. Drives main.rs's stderr notice ONLY — the
    /// notice is interface-only and is never written to `out_dir`.
    pub experimental_notice_active: bool,
    /// ★ §G-19d — the full return's non-gating ADVISORIES, so the export path shows them too.
    ///
    /// `advisories_for` had exactly ONE production caller (`cmd::tax::report_tax_year`), which meant a
    /// filer who ran `export-irs-pdf` and never ran `report` saw NONE of them: not the forgone §63(f)
    /// boxes, not the FBAR sub-question left blank, not the Schedule C 1099 pair. The export path is
    /// the one that hands them a PDF to sign, so it is the last place the omissions should be silent.
    /// Empty on the crypto-slice path, which computes no full return.
    pub advisories: Vec<btctax_core::tax::advisories::Advisory>,
    /// ★ P6 (FILING-READINESS-PLAN rank 9) — the §170(d)(1) charitable carryover to next year, per
    /// class and vintage, so the EXPORT path can tell the filer it exists.
    ///
    /// `AbsoluteReturn::charitable_carryover_out` is computed on every full-return run and was read by
    /// exactly ONE caller — `apply_carryover_writeback`, reachable only from `report
    /// --write-carryover`, which errors unless a year+1 row already exists. A filer whose gift
    /// exceeded its §170(b) ceiling was therefore never told: a deduction already paid for, silently
    /// forgone. This is not a rich-filer case — the ceiling is a FRACTION of AGI, so at $0 AGI the
    /// ceiling is $0 and 100% of the gift carries.
    ///
    /// Empty on the crypto-slice path, which computes no full return and no Schedule A.
    pub charitable_carryover_out: Vec<btctax_core::tax::return_inputs::CharitableCarryItem>,
    /// ★★★ FINAL-REVIEW FINDING 1 — how much of [`charitable_carryover_out`] btctax cannot vouch for
    /// under §170(f)(8), or `None` when it can.
    ///
    /// See [`btctax_core::tax::return_1040::cwa_unvouched_carryover`]. Carried out rather than
    /// re-derived so this surface and `report --tax-year`'s cannot drift; `None` on the crypto-slice
    /// path, which computes no full return.
    pub charitable_carryover_cwa_unvouched: Option<btctax_core::conventions::Usd>,
    /// ★ N4 (FILING-READINESS-PLAN rank 14) — the marks btctax deliberately did NOT make on this
    /// packet, one string per mark. See [`hand_marks`].
    ///
    /// Carried out so the caller can say how many there are without re-deriving the list; the text
    /// itself lives in the packet's `manifest.txt` (owner decision 13), which is the artifact the
    /// filer is told to follow while assembling paper. Empty on the crypto-slice path, whose 1040 is
    /// watermarked "WORKSHEET — NOT A COMPLETE FORM 1040" and is not signed or filed.
    pub hand_marks: Vec<String>,
}

/// ★ N4 — **the marks btctax deliberately leaves for the filer**, in the order they appear on the
/// form. One string per mark; empty is impossible (the signature is every filer's).
///
/// **The signal, never the answer.** Each of these is blank because it is the filer's to make, not
/// because it is zero — "an entry is testimony," and a `0` or a checked box btctax cannot vouch for
/// is fabricated testimony on a §6065-signed page. What was missing was not the mark; it was any
/// statement anywhere in the product's output that the mark exists. A no-crypto filer signed a return
/// with a mandatory question unanswered, and the packet's `manifest.txt` was one line of stapling
/// order.
///
/// Each entry is CONDITIONED on the mark actually being blank in THIS packet, because a list that
/// always says the same thing signals nothing — and telling a filer to hand-mark a box on a
/// correctly-filed form is worse than silence.
fn hand_marks(printed: &btctax_core::tax::packet::PrintedReturn) -> Vec<String> {
    let mut marks = Vec::new();
    if !printed.forms.f1040.digital_asset_yes {
        marks.push(
            "Form 1040 — the Digital Asset question (above line 1a): neither \"Yes\" nor \"No\" is \
             marked. btctax found no digital-asset activity in this vault, but it will not swear \
             \"No\" for a ledger it was never given — a wrong \"No\" here is sworn testimony under \
             §6065. The question is MANDATORY: answer it yourself before you sign."
                .to_string(),
        );
    }
    if !printed.forms.sch_d.must_file() {
        marks.push(
            "Form 1040 line 7 — \"Attach Schedule D if required. If not required, check here\": the \
             box is blank and no Schedule D is in this packet. btctax cannot establish that Schedule \
             D is NOT required — it has no input for Schedule D lines 4, 5, 11 or 12 (Forms 6252, \
             4684, 6781, 8824, 4797, 2439, or a K-1). Check the box only if you know none of those \
             applies to you."
                .to_string(),
        );
    }
    // ★★ F2 (phase-1 seam review). A Section B Form 8283 is NOT filing-ready without a signed Part IV
    //    (appraiser) and Part V (donee acknowledgement), and those were named only on stderr — the
    //    exact surface N4's own rationale rejects, since the manifest is the artifact the filer
    //    follows while assembling paper and the one that does not scroll away. A filer working from
    //    an 8283-bearing packet's manifest saw an authoritative-sounding closed list of "1 mark(s)"
    //    that omitted the two signatures without which the form cannot be filed at all.
    //
    //    ★ These differ in kind from every other mark here: they are THIRD-PARTY signatures. The
    //    others the filer can make at the kitchen table the moment they read this; these take
    //    calendar time to obtain, which is precisely why burying them in scrolled-away stderr is
    //    worse than burying a mark the filer could make on the spot.
    //
    //    The seam: lane B enumerated the marks from the 1040's view while lanes A and C were
    //    changing what the packet's 8283 contains. Neither could see the other.
    if printed.forms.f8283.as_ref().is_some_and(|r| {
        r.rows()
            .iter()
            .any(|row| row.section == Some(btctax_core::Form8283Section::B))
    }) {
        marks.push(
            "Form 8283 Section B — Part IV (Declaration of Appraiser) and Part V (Donee \
             Acknowledgement): both are blank, and NEITHER is yours to sign. A Section B Form 8283 is \
             not filing-ready without the qualified appraiser's signed declaration and the donee \
             organization's acknowledgement. Both come from other people, so start early — this is \
             the one item on this list you cannot finish at your desk tonight."
                .to_string(),
        );
    }
    marks.push(
        "Form 1040 page 2 — the signature block: your signature, the date, your occupation, and the \
         Identity Protection PIN if the IRS issued you one — and the same again for your spouse if \
         you are filing jointly. A return is not filed until it is signed under penalties of perjury \
         (§6065), and no software may sign it for you."
            .to_string(),
    );
    marks
}

/// Render [`hand_marks`] as the packet manifest's closing section — the manifest is the artifact the
/// filer is told to follow while assembling paper, which is why the marks live there (decision 13)
/// rather than only on a stderr line that scrolls away.
fn hand_marks_block(marks: &[String]) -> String {
    use std::fmt::Write as _;
    let mut s = String::from(
        "\n# ── COMPLETE BY HAND — marks btctax deliberately did NOT make ──\n\
         #\n\
         # These are blank because they are YOURS to make, not because they are zero. btctax does\n\
         # not answer for the filer, and it will not sign.\n#\n",
    );
    for m in marks {
        for line in crate::render::wrap_bulleted(m).lines() {
            let _ = writeln!(s, "#{line}");
        }
    }
    s
}

/// The **[I5]** broker-reporting advisory line, regime-aware — or `None` when no disposition may have
/// been broker-reported (`broker_reported_rows == 0`).
///
/// The wording follows the year's Form 1099-DA **regime** (spec 1099-DA R1/R4, joined from
/// `forms/<year>/YEAR.toml`), never a constant:
///
/// - **no proceeds reporting** (pre-TY2025): an exchange disposal may have been reported on a
///   **1099-B**, belongs on a separate 8949 under **Box A/B (ST) / D/E (LT)**, and this export files
///   every row under **Box C/F**;
/// - **proceeds only** (TY2025): it is the **1099-DA**, **Box G/H/J/K**, and every row files under
///   **Box I/L** — the question is not asked on that year (R1);
/// - **proceeds and basis** (live, TY2026 on): the rows were ROUTED by the filer's answers, and the
///   advisory is R4's. ★ r3 M-1 — the parenthetical names **all three** pairs an answer can choose
///   (I/L not reported, H/K proceeds only, G/J basis reported), because this line reads only the row
///   COUNT and never the answers: a filer who answered `not_reported` for every key has every row on
///   I/L and no Form 1099-DA lists any of them, and naming only G/H/J/K asserted a form they never
///   received. Then: compare column (e) of every G/J row with **box 1g** and column (d) of every
///   listed row with **box 1f**; a difference needs the broker's figure in that column and the
///   correction in (g) — "Note: If you checked Box A or Box G above but the basis reported to the IRS
///   was incorrect, enter in column (e) the basis as reported to the IRS, and enter an adjustment in
///   column (g) to correct the basis." (f8949--2025.txt:54). btctax prints NET proceeds and the
///   broker reduces box 1f by transaction costs too (Instructions_1099-DA.txt:470-472), so the figures
///   normally coincide; the advisory names the limb anyway. It also states once how a custodial venue
///   outside the four adapters earns a 1099-DA key (spec r5 NEW-3).
///
/// Emitting the 2025 pairing on a pre-2025 export would steer the filer to boxes that do not exist
/// on that revision — hence the regime gate.
pub fn broker_reporting_advisory(
    tax_year: i32,
    regime: btctax_core::forms::InformationReturnRegime,
    broker_reported_rows: usize,
) -> Option<String> {
    if broker_reported_rows == 0 {
        return None;
    }
    if regime.basis {
        return Some(format!(
            "⚠ [I5] {broker_reported_rows} disposition(s) occurred on a venue that issues Form 1099-DA for TY{tax_year}; each was filed under the Form 8949 box your answer chose (I/L where you answered that nothing was reported, H/K where only proceeds were, G/J where basis was), with columns (f) and (g) left blank. Compare column (e) of every G/J row with box 1g of the 1099-DA, and column (d) of every listed row with box 1f; if any differs, the return needs the broker's figure in that column and the correction in column (g) — see the Note on Form 8949. A custodial venue outside the built-in adapters must be recorded as `exchange:PROVIDER:ACCOUNT` to get a 1099-DA key."
        ));
    }
    let (broker_form, separate_boxes, filed_boxes) = if regime.proceeds {
        ("1099-DA", "Box G/H/J/K", "Box I/L")
    } else {
        ("1099-B", "Box A/B (ST) / D/E (LT)", "Box C/F")
    };
    Some(format!(
        "⚠ [I5] {broker_reported_rows} disposition(s) occurred on an exchange that MAY have issued {broker_form} broker basis reporting — those would belong on a SEPARATE Form 8949 under {separate_boxes}. This export files EVERY Bitcoin row under {filed_boxes} (not-reported default) and says so; reclassify by hand if you received a {broker_form}."
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

/// A statement's body as it is written to disk — banner-prefixed exactly when the packet is DRAFT.
///
/// ★★★ Extracted so the guarantee is TESTABLE. Every PDF in a pseudo-reconciled packet is stamped
/// `DRAFT — ESTIMATE, NOT FOR FILING`; a `.txt` cannot carry a diagonal watermark, so without this it
/// would leave the machine looking like a clean page — and a continuation statement is the one artifact
/// a filer DETACHES, so it is the most likely of all of them to be separated from the forms carrying
/// the warning.
///
/// ★★ Inline, this could only be exercised through a full pseudo-reconciled export, and the only year
/// with a full-return path (TY2024) is not the year the pseudo fixtures use — so the test would have
/// been a conditional that asserts nothing. A rule with no reachable red is not a rule.
pub(crate) fn statement_body(body: &str, watermarked: bool) -> String {
    if !watermarked {
        return body.to_string();
    }
    format!(
        "*** DRAFT — ESTIMATE, NOT FOR FILING ***\n\
         *** This statement was produced from a PSEUDO-RECONCILED ledger: at least one figure on the \
         return it belongs to is a synthetic default, not your data. ***\n\n{body}"
    )
}

#[cfg(test)]
mod statement_body_tests {
    use super::statement_body;

    /// Both legs. A banner that always fires is as wrong as one that never does — the filer learns to
    /// ignore the words.
    #[test]
    fn a_statement_is_marked_draft_exactly_when_the_packet_is() {
        let page = "Form 1040 (2024) — CONTINUATION STATEMENT: DEPENDENTS\nKid 4 ...";
        let clean = statement_body(page, false);
        assert_eq!(clean, page, "a real ledger's statement is untouched");
        assert!(!clean.contains("NOT FOR FILING"));

        let draft = statement_body(page, true);
        assert!(
            draft.starts_with("*** DRAFT — ESTIMATE, NOT FOR FILING ***"),
            "the banner must be the FIRST thing on a detached page: {draft}"
        );
        assert!(
            draft.contains("PSEUDO-RECONCILED") && draft.contains("synthetic default"),
            "it must say WHY, not just shout: {draft}"
        );
        assert!(draft.contains(page), "the statement itself survives intact");
    }
}

/// `export-irs-pdf`: fill the OFFICIAL IRS PDFs for `tax_year` and write them (owner-only) to
/// `out_dir`. THIN OPENER (★ arch-C-1, Defensive Filing Wizard Task 3): opens its OWN `Session` and
/// projects ONCE, then delegates everything else to [`export_irs_pdf_from_session`] — the `&Session`
/// inner a future TUI (which already holds the vault's `VaultLock`) can call directly, without a SECOND
/// `Session::open` (which would deadlock the editor, `session.rs:662`).
pub fn export_irs_pdf(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: i32,
    forms: &[FormArg],
    attest: Option<&str>,
    voucher: VoucherChoice,
) -> Result<IrsPdfReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;
    export_irs_pdf_from_session(
        &session, &state, &events, out_dir, tax_year, forms, attest, voucher,
    )
}

/// The `&Session` inner of `export_irs_pdf` (★ arch-C-1): fill the OFFICIAL IRS PDFs for `tax_year` over
/// an ALREADY-OPEN `session` + an ALREADY-PROJECTED `state`/`events` — no `Session::open`, no re-project.
/// The packet is Form 8949 + Schedule D (always applicable) plus — when applicable and selected —
/// Schedule SE (SE income ≥ $400), Form 8283 (donations), and Form 1040 cap-gains (reportable
/// digital-asset activity). The form data is REUSED from the projection
/// (`form_8949`/`schedule_d`/`form_8283`/`compute_se_tax`) — nothing capital-gains is recomputed; the
/// SE §1401 figure is computed here from the year's stored `TaxProfile`. Same pseudo-active attestation
/// gate as `export-snapshot`: checked FIRST, so a refused export leaves `out_dir` untouched; a pseudo
/// fill is additionally DRAFT-watermarked.
///
/// ★ arch-m-new-1/n-new-1: the full-vs-slice `return_inputs::exists` dispatch lives ONCE, HERE — both
/// the thin `export_irs_pdf` opener AND the chokepoint's `apply_export` (`chokepoint/mod.rs`) route
/// through this ONE fn, so the dispatch is never duplicated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn export_irs_pdf_from_session(
    session: &Session,
    state: &btctax_core::state::LedgerState,
    events: &[LedgerEvent],
    out_dir: &Path,
    tax_year: i32,
    forms: &[FormArg],
    attest: Option<&str>,
    voucher: VoucherChoice,
) -> Result<IrsPdfReport, CliError> {
    export_irs_pdf_from_session_with_regime(
        session, state, events, out_dir, tax_year, forms, attest, voucher, None,
    )
}

/// [`export_irs_pdf_from_session`] with the year's Form 1099-DA **regime** supplied by the caller
/// instead of joined from its `YEAR.toml` record.
///
/// ★★★ **A TEST SEAM, and the only one this build needs (spec 1099-DA R6, kills).** The R6 arms are
/// observable only where a year has BOTH a live (basis-reporting) regime AND bundled form
/// templates, and no bundled year has both: TY2026's record is live and it bundles zero templates
/// (nothing about a printed slice is observable there, N-5), while TY2025 has fifteen templates and
/// a `proceeds`-only regime. Every other injection point on this path — the answers, the ledger,
/// the vault — is already a parameter; the regime was the one fact the export read from a bundled
/// file, so this hands it in. `None` is production: the record decides, exactly as before.
#[allow(clippy::too_many_arguments)]
pub(crate) fn export_irs_pdf_from_session_with_regime(
    session: &Session,
    state: &btctax_core::state::LedgerState,
    events: &[LedgerEvent],
    out_dir: &Path,
    tax_year: i32,
    forms: &[FormArg],
    attest: Option<&str>,
    voucher: VoucherChoice,
    regime_override: Option<btctax_core::InformationReturnRegime>,
) -> Result<IrsPdfReport, CliError> {
    // ★★★ (seam review I-2) The MIRROR of `extension`'s refusal (4), and the direction that happens
    // FIRST in time: April's `btctax extension` writes `f4868.pdf`, October's `export-irs-pdf` reuses
    // the same `--out`, and the whole packet lands AROUND the extension application — no refusal, no
    // warning, and a manifest that does not mention it, so a filer collating that directory by
    // filename would attach the one page the form forbids attaching ("Don't attach a copy of Form
    // 4868 to your return.", page 2). The guard was one-directional; this is the other direction, in
    // both pipelines, before any byte. It costs the filer a `--out` argument.
    if out_dir.join("f4868.pdf").exists() {
        return Err(CliError::Usage(format!(
            "{} already holds f4868.pdf — that is an EXTENSION application, and Form 4868 is MAILED \
             SEPARATELY, weeks before the return exists. The form's own page 2 says \"Don't attach a \
             copy of Form 4868 to your return.\", so the return packet may not be written on top of \
             it: a filer collating this directory by filename would put it in the envelope. Write \
             the return to its own --out directory.",
            out_dir.display()
        )));
    }

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
    // ★★★ spec 1099-DA R6 — THE DISPATCH IS THREE-WAY, and its predicate is a FUNCTION CALL, never a
    //     declared status: `full_return_for(year).is_none()` is what "the year cannot compute a full
    //     return" means, and `YEAR.toml`'s `status` is a declaration that can disagree with the build.
    //
    //     1. inputs stored AND the parameters bundled → the full-return packet (unchanged).
    //     2. the ANSWERS stored AND the parameters NOT bundled → the CRYPTO SLICE, filed from those
    //        answers. Reached by NOT returning early above.
    //     3. otherwise → the slice as it has always been: a live year with an exchange disposition
    //        refuses before any byte and names the exit. A year whose parameters ARE bundled but has
    //        no committed row is arm (3) too, and its refusal names COMMITTING, not answering.
    let params_bundled = {
        use btctax_core::tax::tables::FullReturnTables;
        btctax_adapters::BundledFullReturnTables::load()
            .full_return_for(tax_year)
            .is_some()
    };
    if params_bundled && crate::return_inputs::exists(session.conn(), tax_year)? {
        // The full-return pipeline runs the BG-D8 gate itself (checked first there too).
        let mut report =
            export_full_return(session, state, events, out_dir, tax_year, attest, voucher)?;
        // UX-P4-5: a --forms slice cannot be honored on a full-return year (the 14-form packet is
        // jointly computed; a slice of it is tax-unsound). The packet still writes in full; flag the
        // ignored slice so the caller warns.
        // ★ `--forms full-return` is the one selection this path can HONOR, so it must not be reported
        //   as ignored — the filer asked for exactly what they got (filing trial, B9).
        report.forms_ignored_full_return =
            !forms.is_empty() && forms.iter().any(|f| *f != FormArg::FullReturn);
        return Ok(report);
    }

    // ★★★ spec 1099-DA T9 — the year's WORKING return, resolved ONCE and read by EVERY arm-(2) gate
    //     below, so no two gates can see a different return than the one the answers came from.
    //     Called only AFTER arm (1) is ruled out (N-7). It creates no row: a draft shadows the
    //     committed row (§6.1), a stale WIP draft is discarded with a note, a stale PARKED draft
    //     REFUSES here, and a current parked draft carries no live answers at all.
    let (working, stale_note) = crate::input_form_store::working_return(session.conn(), tax_year)?;
    // Arm (2)'s predicate: the answers are stored AND the year's parameters are not bundled.
    let answers_stored = working
        .as_ref()
        .is_some_and(|ri| !ri.broker_reporting.0.is_empty());
    let files_from_answers = answers_stored && !params_bundled;

    // ── The gates, IN ORDER, before any byte (spec 1099-DA R6 I-5). ──────────────────────────────
    // BG-D8 completeness gate (crypto-slice path) — a promoted-basis leg without its complete Form 8275
    // is a HARD refusal, checked FIRST (before the pseudo watermark check and any byte written).
    promote_export_gate(state, events, Some(tax_year))?;

    // Form 8275 (Disclosure Statement) — Task 16: `Some` iff a promoted-basis disposal leg files in
    // `tax_year` (the same `disclosure_8275` scoping `promote_export_gate` above already used to confirm
    // completeness).
    // ★ FR-29: `None`. `export-irs-pdf` fills Form 8949 + Schedule D, never a Form 1040 — and the
    //   §1(g) no-path position is a position on 1040 line 16. Disclosing it beside a packet that
    //   carries no 1040 would disclose a position this artifact does not take. The full-return
    //   export (`export_full_return`) is where it belongs, and it passes it.
    let printed_8275 = btctax_core::tax::form8275::disclosure_8275(events, state, tax_year, None)
        .map(|d| btctax_core::tax::printed::printed_8275(&d));
    let details = session.donation_details()?;
    let rows_8283 = btctax_core::form_8283(state, tax_year, &details);

    // ★★★ spec 1099-DA R6 (I-1/I-7) — THE FORM-LEVEL GATE. A partially ported year must write
    //     NOTHING: every map the SELECTED forms can reach has to resolve first, and the refusal
    //     names the FIRST missing stem. (`SUPPORTED_YEARS` is a YEAR-level answer — "does any form
    //     of this year fill" — and is a different question, checked below.)
    // ★ R6 fold (M-2) — it is NOT scoped to arm (2): the gate reads only the year and the selected
    //   forms, never the answers, and "write nothing" is a guarantee of the export, not of one arm.
    //   [`form_level_gate_runs`] holds the one condition it does keep.
    if form_level_gate_runs(files_from_answers, tax_year) {
        slice_map_gate(
            tax_year,
            forms,
            !rows_8283.is_empty(),
            !se_net_income(state, tax_year).is_zero(),
            printed_8275.is_some(),
        )?;
    }

    // Attestation gate — no fictional tax form leaves the machine unguarded, and a refusal
    // writes no bytes. (A fully-real ledger ignores `attest`.)
    let watermarked = state.pseudo_active();
    if watermarked {
        require_attestation(attest)?;
    }

    // Reuse the projection's capital-gains data verbatim (no recompute).
    let mut rows = btctax_core::form_8949(state, tax_year);
    let regime = match regime_override {
        Some(r) => r,
        None => crate::year_readiness::regime_or_refuse(tax_year)?,
    };
    // ★★★ spec 1099-DA R6 fold (C-1 + M-3) — THE FORM 8283 RESTRICTION GATE, ON BOTH ARMS, before
    //     any byte. It reads the year's WORKING return, never the answers, so scoping it to arm (2)
    //     was a fail-OPEN: R6 widened the dispatch to `params_bundled && return_inputs::exists(…)`,
    //     and a COMMITTED row on a params-less year whose `broker_reporting` is empty — the normal
    //     shape of a TY2025 import, since the 1099-DA question is not live there — then fell to arm
    //     (3) with the row PRESENT and UNREAD. `donations_had_restrictions == Some(true)` was never
    //     consulted and `form_8283.pdf`, written from the same `rows_8283` on both arms, printed the
    //     gift at full fair market value: an overstated claimed deduction on a form the filer signs.
    // ★★ The DECISION is `return_refuse::donation_restriction_gate`, shared with the full return, so
    //    the slice and the full return cannot disagree about when a restriction blocks a filing.
    //    Only the premises are the slice's own, because it has no Schedule A to condition on: an
    //    8283 attaches iff the year EMITS one, and the year is Section B iff the printed rows say so
    //    (`forms::form_8283` splits on the §170(f)(11)(C) year aggregate over $5,000 — the same term
    //    the full return reads).
    if let Some(ri) = working.as_ref() {
        use btctax_core::tax::return_refuse::DonationRestrictionGate as Gate;
        let section_b = rows_8283
            .iter()
            .any(|r| r.section == Some(btctax_core::Form8283Section::B));
        match btctax_core::tax::return_refuse::donation_restriction_gate(
            ri.donations_had_restrictions,
            !rows_8283.is_empty(),
            section_b,
        ) {
            Some(Gate::Declared) => {
                return Err(CliError::Usage(format!(
                    "TY{tax_year}: you declared that at least one donated property had a restriction or a \
                     retained right (Form 8283 line 5a, 5b or 5c). Under Reg §1.170A-7 that REDUCES or \
                     DENIES the §170 deduction, and btctax values every donation at full fair market \
                     value — so the Form 8283 it would print for {tax_year} overstates the gift. It \
                     cannot tell which gift is affected: complete Form 8283 for the restricted donation \
                     by hand, with the reduced amount. No forms were written."
                )));
            }
            Some(Gate::UnansweredSectionB) => {
                return Err(CliError::Usage(format!(
                    "TY{tax_year}: this year files a Form 8283 SECTION B (donations over $5,000), whose \
                     lines 5a, 5b and 5c ask whether any donated property carried a restriction or a \
                     retained right. A \"Yes\" to any of them reduces or denies the §170 deduction \
                     (Reg §1.170A-7), and btctax values every donation at full fair market value — so it \
                     cannot print the form without the answer. Answer it (`btctax income answer`, or the \
                     TUI's input form) and re-export. No forms were written."
                )));
            }
            None => {}
        }
    }

    if files_from_answers {
        // ★ ARM (2). The boxes are ROUTED from the stored answers through EXACTLY the screen and the
        //   router the full return uses — one screen, one router, so a filer's slice and their full
        //   return can never carry different boxes.
        let ri = working.as_ref().expect("answers ⇒ a working return");
        if let Some(r) =
            btctax_core::tax::return_refuse::screen_broker_reporting(ri, state, tax_year, regime)
        {
            // ★ The SLICE's own sentence, carrying the SAME reason and detail. It must never say
            //   "the return is not computable" — on this arm that is false: the slice computes, and
            //   what is missing is one Form 1099-DA answer.
            return Err(CliError::Usage(format!(
                "TY{tax_year}: the crypto slice cannot choose a Form 8949 box [{:?}]: {} No forms were written.",
                r.reason, r.detail
            )));
        }
        // ★ The Form 8283 restriction gate ran ABOVE, on both arms (R6 fold C-1/M-3).
        btctax_core::route_8949_boxes(&mut rows, regime, &ri.broker_reporting).map_err(|e| {
            CliError::Usage(format!(
                "TY{tax_year}: the stored Form 1099-DA answers do not settle every Form 8949 row ({e}). No forms were written."
            ))
        })?;
    } else {
        // ★ ARM (3) — spec 1099-DA R1 / R6: no answers are stored, so a LIVE year refuses BEFORE any
        //   byte and names the exit its own state actually has. TY2025 (proceeds only) and a live
        //   year with no exchange disposition fill as before.
        if let Some(e) = slice_broker_refusal(tax_year, regime, &rows) {
            return Err(e);
        }
    }

    // ★ spec 1099-DA R6 (M-5/M-7) — the EXPORT-TIME price-coverage check, on BOTH arms, before any
    //   byte: a packet whose price dataset stops mid-year is a return computed from a partial year.
    crate::year_readiness::price_coverage_or_refuse(tax_year)?;

    // ★★★ (I-7) `--pay-by-check` on a year with no full return must REFUSE, naming the reason. The
    // crypto slice computes no Form 1040 at all, so there is no line 37 for box 3 to carry — and a
    // voucher whose amount btctax invented would tell the Service the filer is paying a figure no
    // return supports. Writing the slice silently and skipping the voucher the filer asked for is the
    // one answer a tax tool may not give.
    // ★ spec 1099-DA R6 (M-10) — RE-WORDED when the year's parameters are not bundled: telling a
    //   filer who has already authored their inputs to "see `income import`" names a step they have
    //   done, and hides the real reason (this build carries no full return for the year).
    if voucher.pay_by_check {
        return Err(if params_bundled {
            CliError::Usage(format!(
                "there is no Form 1040 line 37 for {tax_year} — Form 1040-V accompanies a full return; \
                 see `income import`"
            ))
        } else {
            CliError::Usage(format!(
                "there is no Form 1040 line 37 for {tax_year} — Form 1040-V accompanies a full return, \
                 and the TY{tax_year} full-return parameters are not bundled in this build, so btctax \
                 computes no Form 1040 for the year at all. Authoring inputs will not change that; the \
                 full return follows when the year's package is bundled. No forms were written."
            ))
        });
    }

    // ★★★ `--forms full-return` on a year with no full return must REFUSE, loudly. `wants()` is
    // `selected.is_empty() || selected.contains(f)`, so this selection matches no crypto-slice form and
    // would otherwise write an EMPTY export directory and exit 0 — a filer would reasonably read that
    // as "there was nothing to file". Silence is the one answer a tax tool may not give here.
    if forms.contains(&FormArg::FullReturn) {
        return Err(if params_bundled {
            CliError::Usage(format!(
                "--forms full-return asks for the complete return packet, but tax year {tax_year} has no \
                 full-return inputs. Author them first with `btctax income import --year {tax_year} \
                 --file <inputs.toml>`, or drop --forms to export the crypto slice."
            ))
        } else {
            CliError::Usage(format!(
                "--forms full-return asks for the complete return packet, but the TY{tax_year} \
                 full-return parameters are not bundled in this build — btctax computes no Form 1040 \
                 for {tax_year} at all, whatever inputs are stored. Drop --forms to export the crypto \
                 slice (Form 8949, Schedule D, …), which your own Form 1040 carries; the full return \
                 follows when the year's package is bundled."
            ))
        });
    }

    // ★★ spec 1099-DA R6 fold (N-1) — a `--forms` narrowing that keeps Schedule D but DROPS Form
    //    8949 must REFUSE, and the refusal is the answer rather than probing both maps.
    //    Since T8 the Schedule D this path writes carries PER-BOX lines (1b/2/3, 8b/9/10) whose own
    //    captions read "Totals for all transactions reported on Form(s) 8949 with Box … checked" —
    //    they cite a page-set by name. Written into a directory that holds no `f8949.pdf`, the
    //    schedule states totals for an attachment the packet does not contain, and the filer mails
    //    the directory. Fail closed: the narrowing is refused with the missing form named, so the
    //    filer either adds `f8949` or exports the two separately, deliberately. (The converse is
    //    fine and stays allowed — an 8949 without its Schedule D cites nothing.)
    if !forms.is_empty() && forms.contains(&FormArg::ScheduleD) && !forms.contains(&FormArg::F8949)
    {
        return Err(CliError::Usage(format!(
            "--forms selects `schedule-d` without `f8949`, and TY{tax_year}'s Schedule D cites it by \
             name: its per-box lines are captioned \"Totals for all transactions reported on Form(s) \
             8949 with Box … checked\", so the schedule would state totals for a page-set this export \
             directory does not contain. Add `f8949` to --forms (or drop --forms for the whole crypto \
             slice). No forms were written."
        )));
    }

    let totals = btctax_core::schedule_d(state, tax_year);
    let by_box = btctax_core::schedule_d_by_box(&rows);

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

    // T-f8275-part-ii-overflow round 2 finding 2: refuse an OVERFLOWING Part II narrative HERE too,
    // before `mkdir_out` — same reasoning as the Part I row check just above. Without this, the
    // narrative's overflow was only discovered mid-write, deep inside `fill_form_8275_slice` (called
    // AFTER `basis_methodology.txt`, `form_8275.txt`, and possibly `f8949.pdf`/`schedule_d.pdf` were
    // already on disk) — leaving an estimated-basis 8949 filed with no 8275 PDF behind it, exactly the
    // §6662(d) exposure this whole disclosure feature exists to close.
    if let Some(p) = &printed_8275 {
        if let btctax_forms::PartIiCapacity::Overflow(overflow) =
            btctax_forms::part_ii_capacity_check(&p.part_ii, tax_year)?
        {
            return Err(CliError::Usage(part_ii_overflow_message(
                tax_year, &overflow,
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

    // ★ whole-branch tax M-2: refuse an UNSUPPORTED year HERE, before `mkdir_out` — mirroring the Form
    // 8275 pre-check directly above (and `export_full_return`'s own pre-write table lookup). Without it,
    // a year outside `btctax_forms::SUPPORTED_YEARS` created `out_dir/` and wrote
    // `basis_methodology.txt` + `form_8275.txt` BEFORE `fill_form_8949` raised `UnsupportedYear`,
    // leaving a HALF-POPULATED packet directory beside the reported failure — a filer could mail a
    // directory holding a methodology disclosure with no forms behind it. The error is byte-identical to
    // the one `fill_form_8949` used to raise (`CliError::FormFill(FormsError::UnsupportedYear(year))`),
    // so the single-year CLI `export-irs-pdf` path is unchanged apart from writing ZERO bytes.
    if !btctax_forms::SUPPORTED_YEARS.contains(&tax_year) {
        return Err(CliError::FormFill(
            btctax_forms::FormsError::UnsupportedYear(tax_year),
        ));
    }

    mkdir_out(out_dir)?;

    // I-3 (D-4): the MANDATORY conservative-filing methodology disclosure rides the PDF packet too, not
    // just the CSV paths — a filer mailing the flagship filing-ready artifact must get the i8949-required
    // basis explanation whenever a $0-basis tranche row is present. Writes nothing for a no-tranche year.
    crate::render::write_basis_methodology_txt(out_dir, state, tax_year)?;
    // BG-D8: the Form 8275 disclosure rides the packet by its OWN name. The gate above guaranteed a
    // promoted leg reaching here has a complete Part II. Writes nothing for a no-promoted-leg year.
    // ★ FR-29: `None` — this path fills Form 8949 + Schedule D, not a Form 1040 (see above).
    crate::render::write_form_8275_txt(out_dir, state, events, tax_year, None)?;
    // Approach-B experimental disclosure (`design/approach-b-experimental-notice`): an INTERFACE-only
    // signal for main.rs's stderr notice — deliberately NEVER written to `out_dir` (the export directory
    // is what the filer mails/hands to a preparer; the notice belongs on stderr/TUI/NOTICE only).
    let experimental_notice_active = btctax_core::experimental::uses_approach_b(events);

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
        let bytes = stamp(btctax_forms::fill_schedule_d(&totals, &by_box, tax_year)?)?;
        let path = out_dir.join("schedule_d.pdf");
        write_bytes_owner_only(&path, &bytes)?;
        Some(path)
    } else {
        None
    };

    // ── Schedule SE (self-employment tax). Compute the §1401 figure from the year's TaxProfile. ──
    let se_computed = {
        let tables = BundledTaxTables::load();
        let profile = match session.resolve_screened(state, tax_year, &tables)? {
            crate::resolve::ProfileOutcome::Ready { profile, .. } => profile,
            crate::resolve::ProfileOutcome::Uncomputable { .. } => None, // export proceeds; SE omitted
        };
        profile.and_then(|p| {
            tables.table_for(tax_year).and_then(|t| {
                compute_se_tax(
                    state,
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
        se_computed.is_none() && !se_net_income(state, tax_year).is_zero();
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
        // ★ spec 1099-DA R6 — the SAME `rows_8283` the arm-(2) gates read above (one derivation, so
        //   the restriction refusal and the printed form can never disagree about whether the year
        //   emits an 8283).
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
            // ★ This form is a WORKSHEET, not a return. btctax vouches for exactly two cells on it
            // (the digital-asset question and line 7a) and leaves wages, every deduction and every
            // tax line blank — yet it renders as a Form 1040 with a large line 7a. The stderr note
            // saying so is transient; the PDF is not. Stamp the disclosure onto the page itself, so a
            // filer who finds this file a month later cannot mistake it for their return. The
            // full-return `f1040.pdf` is complete and is NEVER stamped this way.
            let bytes = btctax_forms::stamp_partial_worksheet_watermark(&fill.pdf)?;
            let bytes = stamp(bytes)?;
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
        // ★ spec 1099-DA R6 (M-6) — what this packet IS, said once, on the arm that files from the
        //   stored answers. `form_1040_capgains.pdf` renders as a Form 1040 and is a WORKSHEET; the
        //   set as a whole is an attachment to the filer's OWN 1040, not a return.
        slice_attachment_note: files_from_answers.then(|| {
            format!(
                "TY{tax_year}: full-return parameters are not bundled in this build — this is the \
                 crypto slice, an ATTACHMENT SET (Form 8949, Schedule D, …) with the boxes routed \
                 from your Form 1099-DA answers; your own Form 1040 carries it, and \
                 `form_1040_capgains.pdf` is a worksheet, not a return. The full return follows when \
                 the year's package is bundled."
            )
        }),
        // ★ spec 1099-DA R6 (M-14) — reworded, not `Display`ed verbatim: this path never calls
        //   `session.save()`, so nothing was persistently discarded; the load-bearing half is that
        //   the draft the filer was editing was SKIPPED and is not what filed.
        stale_draft_note: stale_note.map(|n| {
            format!(
                "your {}-schema draft for {} could not be read by this build (expected v{}), so it \
                 was skipped and the last COMMITTED return is what these forms were filed from. \
                 Nothing was deleted.",
                n.found, n.year, n.expected
            )
        }),
        // The crypto slice computes no full return, so there are no full-return advisories — and no
        // Schedule A, hence no §170(d)(1) carryover either.
        advisories: Vec::new(),
        charitable_carryover_out: Vec::new(),
        charitable_carryover_cwa_unvouched: None,
        // N4 does not apply to the slice: its 1040 is a WORKSHEET (watermarked "NOT A COMPLETE FORM
        // 1040"), it is never signed or filed, and the note printed for it already says every other
        // line is the filer's.
        hand_marks: Vec::new(),
        // The crypto slice never reaches a Form 1040 line 37, and `--pay-by-check` already refused
        // above — so there is no voucher to write.
        form_1040v_path: None,
        // ★ (seam review M-1) …but a `--pay` given WITHOUT `--pay-by-check` may not VANISH here
        //   either. The full-return path already says so outright (the "was IGNORED" clause in
        //   `write_payment_voucher`); this arm never reaches that function, so the same filer slip
        //   was silent on a crypto-slice year. Same class, same answer: a note, never a refusal.
        // ★ spec 1099-DA R6 (M-10, the same class as the two refusals above) — the REASON clause
        //   follows `params_bundled` too. On arm (2) the filer HAS authored inputs, so "has no
        //   full-return inputs … see `income import`" names a step they have done and hides the
        //   real one (this build carries no full return for the year).
        form_1040v_note: voucher.pay.map(|p| {
            let why = if params_bundled {
                format!(
                    "{tax_year} has no full-return inputs, so there is no Form 1040 line 37 for box 3 \
                     to carry (see `income import`)"
                )
            } else {
                format!(
                    "the TY{tax_year} full-return parameters are not bundled in this build, so there \
                     is no Form 1040 line 37 for box 3 to carry"
                )
            };
            format!(
                "--pay ${p} was IGNORED: it sets Form 1040-V box 3, and no voucher was written — {why}."
            )
        }),
        full_return_paths: Vec::new(),
        full_return_manifest: None,
        forms_ignored_full_return: false, // crypto-slice path honors --forms
        f8949_path,
        schedule_d_path,
        tax_year,
        unresolved_hard,
        broker_reported_rows: btctax_forms::rows_possibly_broker_reported(&rows),
        regime,
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
        experimental_notice_active,
    })
}

/// The literal sentence the manifest's voucher block leads with. Kept as a constant so the KAT and
/// the file assert the SAME string — the manifest is what tells a filer what to put in the envelope,
/// and this is the line that stops them stapling it.
pub const ENCLOSE_LOOSE_LINE: &str = "ENCLOSE LOOSE — do not staple: f1040v.pdf with the check";

/// Write Form 1040-V beside the packet when the return owes AND the filer asked for it, appending
/// its own block to `manifest`. Returns `(path, note)` for [`IrsPdfReport`].
///
/// The four cases, all of them from spec R4 and none of them silent:
///
/// | line 37 | `--pay-by-check` | outcome |
/// |---|---|---|
/// | `> 0` | yes | the voucher, plus the manifest block |
/// | `> 0` | no  | NO file, and a note naming Direct Pay / EFTPS and the flag |
/// | `0`   | yes | NO file, and a note saying the return owes nothing |
/// | `0`   | no  | nothing at all — there is nothing to say |
///
/// `--pay` (a partial payment) is refused above line 37, below zero, at zero, or with cents — but
/// NOT HERE: all four refusals run PRE-BYTE in [`export_full_return`], immediately after
/// `assemble_printed_return` (seam review I-1). By the time this function runs the packet is already
/// on disk and `manifest.txt` is not, which is a state `btctax extension`'s own guard cannot
/// distinguish from an empty directory. That ceiling is the asymmetry with `btctax extension --pay`,
/// which has none: the voucher pays a COMPUTED balance, so more than it is a filer error; the 4868's
/// line 7 pays against an ESTIMATE, which a filer may deliberately overshoot to limit interest.
fn write_payment_voucher(
    printed: &btctax_core::tax::packet::PrintedReturn,
    out_dir: &Path,
    tax_year: i32,
    voucher: VoucherChoice,
    watermarked: bool,
    manifest: &mut String,
) -> Result<(Option<PathBuf>, Option<String>), CliError> {
    use std::fmt::Write as _;

    let owed = printed.forms.f1040.line37;

    if owed <= Usd::ZERO {
        let note = voucher.pay_by_check.then(|| {
            format!(
                "no Form 1040-V was written — the {tax_year} return owes nothing (Form 1040 line 37 \
                 is ${owed}). A payment voucher accompanies a payment; there is none to make."
            )
        });
        return Ok((None, note));
    }

    if !voucher.pay_by_check {
        // ★ A note, NOT a refusal and NOT a voucher. Most filers who owe pay online — the voucher's
        //   own page 2 opens *"Save time by paying online."* — and Direct Pay / EFTPS need no
        //   voucher at all. But a filer who is about to post a cheque needs to know the page exists,
        //   and silence about a balance due is the one answer a tax tool may not give.
        //
        // ★★ A `--pay` given WITHOUT `--pay-by-check` is discarded here, and the note says so
        //    outright. It is not a refusal — refusing the whole packet over an inapplicable flag
        //    would cost the filer every form to make a point about one — but it may not be silent
        //    either: they typed an amount they meant to pay, and nothing else on the run would tell
        //    them it went nowhere.
        let ignored = match voucher.pay {
            Some(p) => format!(
                " (--pay ${p} was IGNORED: it sets Form 1040-V box 3, and no voucher was written)"
            ),
            None => String::new(),
        };
        return Ok((
            None,
            Some(format!(
                "amount owed ${owed} — Direct Pay / EFTPS need no voucher; pass --pay-by-check for \
                 Form 1040-V{ignored}"
            )),
        ));
    }

    // The amount: line 37 as printed, or the partial payment the filer chose.
    //
    // ★★★ (seam review I-1) EVERY `--pay` REFUSAL WAS HOISTED OUT OF HERE. This function runs with
    // the packet already on disk and `manifest.txt` not yet written, so a refusal at this point left
    // a directory that `btctax extension`'s guard reads as empty ground. All four now run in
    // `export_full_return` right after `assemble_printed_return`, under the SAME
    // `pay_by_check && owed > 0` guard this match sat behind. Nothing may be added back here.
    let amount = voucher.pay.unwrap_or(owed);
    debug_assert!(
        amount > Usd::ZERO && amount <= owed && amount == amount.trunc(),
        "--pay is validated PRE-BYTE upstream; {amount} reached the voucher writer against ${owed}"
    );

    let bytes = btctax_forms::fill_form_1040v(printed, tax_year, amount)?;
    let bytes = if watermarked {
        btctax_forms::stamp_draft_watermark(&bytes)?
    } else {
        bytes
    };
    let path = out_dir.join("f1040v.pdf");
    write_bytes_owner_only(&path, &bytes)?;

    // ★ A block of its OWN, below the stapling list. The blank line and the un-indented heading are
    //   what make it read as a separate instruction rather than one more thing to staple.
    let _ = write!(
        manifest,
        "\n{ENCLOSE_LOOSE_LINE}\n  \
         Form 1040-V is the payment voucher for the ${amount} you are paying by check or money order. \
         Put it in the envelope with your payment, LOOSE — the voucher says \"Do not staple or attach \
         this voucher to your payment or return.\"\n  \
         Make the check or money order payable to \"United States Treasury\", and write your SSN, your \
         daytime phone number and \"{tax_year} Form 1040\" on it.\n"
    );

    let note = (amount < owed).then(|| {
        format!(
            "Form 1040-V carries a PARTIAL payment of ${amount} against the ${owed} owed (Form 1040 \
             line 37) — interest and any late-payment penalty run on the ${rest} left unpaid.",
            rest = owed - amount
        )
    });
    Ok((Some(path), note))
}

/// The Form 8275 Part II narrative overflow refusal (T-f8275-part-ii-overflow round 2 finding 2) —
/// shared by BOTH export paths (`export_irs_pdf_from_session` + `export_full_return`) so the wording
/// never drifts between them. The narrative is FIXED once recorded (the vault is append-only —
/// `plan_promote` refuses to re-promote an already-promoted tranche), so "shorten it and re-run
/// promote-tranche" is not an available remedy at export time; the honest remedy is void-and-redo (a
/// known follow-up: `design/f8275-part-ii-overflow/FOLLOWUPS.md`).
fn part_ii_overflow_message(tax_year: i32, overflow: &btctax_forms::PartIiOverflow) -> String {
    format!(
        "cannot export {tax_year}: the Form 8275 Part II narrative needs about {rows} single-line \
         fields but only {cap} are available (Part II's own line 1 + Part IV's continuation lines) at \
         8pt \u{2014} roughly the first {chars} characters of it would fit. The narrative is fixed once \
         recorded (the vault is append-only), so shortening it now means voiding the promote(s) whose \
         narrative is too long and re-recording with a shorter --part-ii-file. File the 8275 manually \
         for {tax_year} instead, or void and re-record, then re-export.",
        rows = overflow.rows_needed,
        cap = overflow.capacity,
        chars = overflow.chars_fit,
    )
}

// ★ `form_8949_overflow_message` stood here and was DELETED with its preflight when P2b landed in the
// same branch. It was the honest refusal for a full-return 8949 that overflowed one page; that path
// now paginates without a ceiling, so the message described a state the code can no longer reach. A
// refusal message for an unreachable branch is worse than none: it documents a limit that does not
// exist, and the next reader budgets around it. The two Form 8275 messages above are unaffected —
// those paths genuinely do not paginate.

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

/// The screened, computed return that BOTH full-return-shaped paths start from: the packet export
/// (`export_full_return`) and the extension application (`extension`).
///
/// ★ It exists so spec R2's *"`export_full_return`'s three refusals, reused as they stand and in its
/// order"* is STRUCTURAL rather than a promise. The three are `no full-return tables for {y}`, `no
/// return_inputs stored for {y}`, and the `not computable … — no forms were written` screens; a
/// second hand-copy of them in the extension arm is exactly the seam this repo's B3 note is about —
/// each lane green, the product wrong where they meet.
///
/// The tables are loaded by the CALLER and borrowed here, because `params` and `table` are references
/// into them and must outlive this call.
struct ScreenedReturn<'a> {
    params: &'a btctax_core::tax::tables::FullReturnParams,
    table: &'a btctax_core::tax::tables::TaxTable,
    ri: btctax_core::tax::return_inputs::ReturnInputs,
    ar: btctax_core::tax::return_1040::AbsoluteReturn,
    regime: btctax_core::InformationReturnRegime,
}

/// The year's tables, the stored inputs, and the three fail-closed screens the report runs — in the
/// report's own order. A refusal here has written NO bytes, by construction: nothing in it touches
/// the filesystem.
fn screen_full_return<'a>(
    session: &Session,
    state: &btctax_core::state::LedgerState,
    tax_year: i32,
    tables: &'a BundledTaxTables,
    fr_tables: &'a btctax_adapters::BundledFullReturnTables,
) -> Result<ScreenedReturn<'a>, CliError> {
    use btctax_core::tax::tables::{FullReturnTables, TaxTables};

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
    // ★ spec 1099-DA R1 — the year's Form 1099-DA regime, joined from its record, never assumed
    let regime = crate::year_readiness::regime_or_refuse(tax_year)?;
    if let Some(r) =
        btctax_core::tax::return_1040::screen_absolute(&ri, &ar, params, state, tax_year, regime)
    {
        return Err(refuse(r));
    }
    Ok(ScreenedReturn {
        params,
        table,
        ri,
        ar,
        regime,
    })
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
#[allow(clippy::too_many_arguments)]
fn export_full_return(
    session: &Session,
    state: &btctax_core::state::LedgerState,
    events: &[LedgerEvent],
    out_dir: &Path,
    tax_year: i32,
    attest: Option<&str>,
    voucher: VoucherChoice,
) -> Result<IrsPdfReport, CliError> {
    use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
    use std::fmt::Write as _;

    // BG-D8 completeness gate — checked FIRST (before the tables lookup, the fail-closed screens, and any
    // byte written): a promoted-basis leg without its complete Form 8275 is a HARD refusal.
    promote_export_gate(state, events, Some(tax_year))?;

    // ★ spec 1099-DA R6 (M-5/M-7) — the export-time price-coverage check runs on BOTH arms, before
    //   any byte. `YearReadiness::problems` asks the same question but only of a `filable` year and
    //   only as a static report; a packet computed from a price dataset that stops mid-year is a
    //   wrong number on a form the filer signs.
    crate::year_readiness::price_coverage_or_refuse(tax_year)?;

    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let ScreenedReturn {
        params,
        table,
        ri,
        ar,
        regime,
    } = screen_full_return(session, state, tax_year, &tables, &fr_tables)?;
    // ★ §G-19d — the same advisories `report --tax-year` shows, carried out on the report so the
    // EXPORT path surfaces them too. Derived from the identical `advisories_for` call, never a second
    // list: two derivations would drift, and the one the filer saw would depend on which command they
    // happened to run.
    let advisories =
        btctax_core::tax::advisories::advisories_for(&ri, state, &ar, params, tax_year);

    // Pseudo figures are FICTIONAL: they are watermarked no matter what, and the attestation gate for
    // them is unchanged. A clean (real-ledger) packet needs no attestation — SPEC §9 as amended.
    let watermarked = state.pseudo_active();
    if watermarked {
        require_attestation(attest)?;
    }

    let details = session.donation_details()?;
    let printed = btctax_core::tax::packet::assemble_printed_return(
        &ri, state, &details, &ar, table, tax_year, events, regime,
    )
    .map_err(|e| {
        // `HeaderError`'s Display carries the right remedy per variant (a malformed SSN, an unanswered
        // declaration, or an MFJ return with no spouse) — no longer always "fix the identity" (P9 §3.2).
        CliError::Usage(format!("the {tax_year} return cannot be printed: {e}"))
    })?;

    // ★★★ (seam review I-1 + M-2) EVERY `--pay` REFUSAL, hoisted out of `write_payment_voucher` to
    // HERE — the first moment Form 1040 line 37 exists, and still well before `fill_full_return` /
    // `mkdir_out`, so a refusal leaves `--out` exactly as it found it.
    //
    // ★ WHY IT MOVED. Down in the voucher writer these ran AFTER every packet PDF was on disk and
    // BEFORE `manifest.txt` was written. So `--pay 100.25` (a plausible typo) or `--pay <above line
    // 37>` (an ordinary slip) refused with the return's PDFs sitting in `--out` and no manifest
    // beside them — and `btctax extension` decides "is this the return's envelope directory?" by the
    // presence of `manifest.txt` ALONE. It read that directory as empty ground and wrote `f4868.pdf`
    // into it: precisely the outcome that guard exists to prevent. The unsound assumption was
    // `manifest.txt present ⟺ packet directory`, and a refused export falsifies it.
    //
    // ★ The GUARD is the old reachability, exactly: `write_payment_voucher` reached this match only
    // when `--pay-by-check` was given AND the return owed something. Nothing becomes a refusal that
    // was not one — a `--pay` given without `--pay-by-check` is still a NOTE (refusing the whole
    // packet over an inapplicable flag would cost the filer every form to make a point about one),
    // and a return that owes nothing still gets its note. Only the MOMENT changed.
    if voucher.pay_by_check && printed.forms.f1040.line37 > Usd::ZERO {
        let owed = printed.forms.f1040.line37;
        match voucher.pay {
            None => {}
            Some(p) if p < Usd::ZERO => {
                return Err(CliError::Usage(format!(
                    "--pay must be >= 0 (got {p}) — a negative payment is not a payment"
                )))
            }
            Some(p) if p > owed => {
                return Err(CliError::Usage(format!(
                    "--pay ${p} is more than the ${owed} this return owes (Form 1040 line 37). The \
                     voucher pays a COMPUTED balance, so paying above it is a slip rather than a \
                     choice the form names — drop --pay to pay the whole ${owed}, or lower it for a \
                     partial payment. (`btctax extension --pay` has no such ceiling: line 7 there \
                     pays against an ESTIMATE, which you may deliberately overshoot to limit \
                     interest.)"
                )))
            }
            Some(p) if p != p.trunc() => {
                return Err(CliError::Usage(format!(
                    "--pay must be WHOLE DOLLARS (got {p}). The return is printed in whole dollars, \
                     so a voucher carrying cents would disagree with the Form 1040 it accompanies \
                     about what is being paid."
                )))
            }
            // ★ M-2: `--pay 0 --pay-by-check` on a return that OWES used to reach `fill_form_1040v`,
            //   which refused as `FormFill` with "a return that owes nothing needs no Form 1040-V" —
            //   a sentence describing the WRONG state (this return owes ${owed}; the filer typed a
            //   zero) in a class that does not read as a flag error. It also shared the partial-write
            //   shape above.
            Some(p) if p == Usd::ZERO => {
                return Err(CliError::Usage(format!(
                    "--pay $0 writes no voucher — a voucher for $0 is not a payment, and this return \
                     owes ${owed} (Form 1040 line 37). Drop --pay-by-check if you are not paying by \
                     check, or name the amount you are paying."
                )))
            }
            Some(_) => {}
        }
    }

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

    // T-f8275-part-ii-overflow round 2 finding 2: pre-flight for the SAME nicely-worded refusal as the
    // crypto-slice path above ("do it uniformly"). This path was already BYTE-safe without it — the
    // ALL-OR-NOTHING `fill_full_return` below runs before `mkdir_out`, so an overflowing narrative
    // already refused with zero bytes written — but without this check it would surface as this
    // crate's generic `FormsError::Overflow` Display via `CliError::FormFill`, not a message naming the
    // year, the character budget, and a remedy.
    if let Some(f8275) = &printed.forms.f8275 {
        if let btctax_forms::PartIiCapacity::Overflow(overflow) =
            btctax_forms::part_ii_capacity_check(&f8275.part_ii, tax_year)?
        {
            return Err(CliError::Usage(part_ii_overflow_message(
                tax_year, &overflow,
            )));
        }
    }

    // ★★ P2a's Form 8949 overflow preflight USED TO STAND HERE, and was DELETED when P2b landed in the
    // same branch. It refused any export whose Part I or Part II row count exceeded one page's grid,
    // with an honest message naming the year, the capacity and a remedy. That was correct while the
    // full-return path could not paginate. It is a FALSE REFUSAL now that it can: `fill_full_return`
    // reaches `fill_8949_full_with_map` (packet.rs -> lib.rs `fill_8949_full`), which chunks into
    // ⌈rows/grid⌉ page copies with NO ceiling, so there is no row count this path cannot fill.
    //
    // ★★★ WHY THIS IS RECORDED RATHER THAN SILENTLY REMOVED — it is the exact defect shape harness B3
    // exists for, and it was invisible to every test in the branch. P2a (btctax-cli) and P2b
    // (btctax-forms) were built in PARALLEL worktrees against the same base. Each shipped a passing
    // kill-test, and the two tests asserted OPPOSITE things about the same 15-leg filer: the forms test
    // asserted it PAGINATES, the CLI test asserted it REFUSES. Both suites were green at once, because
    // each was scoped to one layer and neither could see the other. The product was broken in the seam:
    // the CLI refused a packet the filler would have produced — reintroducing precisely the total loss
    // (exit 2, zero bytes, every form gone) that P2b existed to end, for the DCA population P2a itself
    // identified as the most exposed. The lesson is B3's: a per-range review is not a branch review, and
    // a green suite per lane does not compose into a correct product.
    //
    // The Form 8275 preflights above are UNAFFECTED and stay: those paths genuinely do not paginate.

    // ★ ALL-OR-NOTHING: every form fills BEFORE anything is written.
    let packet = btctax_forms::fill_full_return(&printed, tax_year)?;

    mkdir_out(out_dir)?;
    // I-3 (D-4): the MANDATORY methodology disclosure rides the full-return packet too (see export_irs_pdf).
    crate::render::write_basis_methodology_txt(out_dir, state, tax_year)?;
    // BG-D8: the Form 8275 disclosure rides the full-return packet by its OWN name (gate above guaranteed
    // a complete Part II). Writes nothing for a no-promoted-leg, non-§1(g) year.
    // ★★★ FR-29 / SPEC §6.3.3 — THIS is the export that files a Form 1040, so this is the one that
    //     carries the §1(g) no-path position. `ar.form8615_certification` was decided once in
    //     `assemble_absolute`; column (f) is 1040 line 16 as filed (`ar.regular_tax`).
    let section_1g =
        ar.form8615_certification
            .map(|c| btctax_core::tax::form8275::Section1gPosition {
                unearned: c.unearned,
                threshold: c.threshold,
                tax_line16: ar.regular_tax,
            });
    crate::render::write_form_8275_txt(out_dir, state, events, tax_year, section_1g.as_ref())?;
    // Approach-B experimental disclosure (`design/approach-b-experimental-notice`): an INTERFACE-only
    // signal for main.rs's stderr notice — deliberately NEVER written to `out_dir` (the export directory
    // is what the filer mails/hands to a preparer; the notice belongs on stderr/TUI/NOTICE only).
    let experimental_notice_active = btctax_core::experimental::uses_approach_b(events);
    let mut manifest = String::from("# btctax full-return packet — staple in this order\n");
    let mut paths: Vec<PathBuf> = Vec::new();
    for form in &packet.forms {
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
    // ★★★ The statements the return OBLIGES. A checked "more than four dependents" box with no
    //     attachment is an incomplete return, so these are written from the same packet that checked
    //     it — and they are listed in the manifest, because the manifest is what tells a filer what to
    //     staple. A page the filer never learns to attach may as well not have been generated.
    for st in &packet.statements {
        let path = out_dir.join(format!("{}.txt", st.name));
        // ★★★ A STATEMENT RIDING WITH DRAFT FORMS MUST SAY SO. Every PDF in a pseudo-reconciled
        //     packet is stamped `DRAFT — ESTIMATE, NOT FOR FILING`; a `.txt` cannot carry a diagonal
        //     watermark, and without this it would leave the machine looking like a clean page. It is
        //     a page the filer DETACHES and attaches to a return — the one artifact most likely to be
        //     separated from the forms that carry the warning.
        write_bytes_owner_only(&path, statement_body(&st.body, watermarked).as_bytes())?;
        let _ = writeln!(manifest, "  ATT  {}.txt  (attach to Form 1040)", st.name);
        paths.push(path);
    }
    // ★★★ §170(f)(11)(D) — THE APPRAISAL IS AN ATTACHMENT, so the stapling order has to name it.
    //
    // The advisory already tells the filer this on stderr; the manifest is what tells them what to
    // PUT IN THE ENVELOPE, and a required attachment that appears in neither is one nobody attaches.
    // btctax cannot generate the page — only a qualified appraiser can — so it is listed as a
    // MISSING item the filer supplies, with the amount that triggered it. Derived from the same
    // advisory list, so the two can never disagree about whether the duty exists.
    //
    // ★★ SCOPED TO THE PACKET THAT ACTUALLY CLAIMS IT. The ADVISORY is about the property and fires
    //    in the year of the gift whatever election the filer makes — correct, because the duty
    //    follows the claim across carryover years and the filer needs to know now. The MANIFEST is
    //    this envelope's stapling order, so it may only say "attach this HERE" on a return that
    //    claims the property deduction (Schedule A line 12 > $0). A standard-deduction year, or one
    //    the §170(b) ceiling zeroes, claims nothing and gets the advisory without the manifest line.
    //    ★ Note what is NOT scoped: the $500,000 TEST itself stays the pre-ceiling claimed amount.
    // ★ `deduction_is_itemized` is REQUIRED here (phase-2 review, merge Minor). `ScheduleAParts` is
    //   built whenever Schedule A inputs exist, regardless of the §63(e) election — so testing
    //   line 12 alone put the ATT line on a standard-deduction packet that claims nothing, which is
    //   the exact instruction this block's own comment says it avoids.
    let claims_property_deduction = ar.deduction_is_itemized
        && ar
            .schedule_a
            .as_ref()
            .is_some_and(|a| a.charitable_noncash_12 > btctax_core::Usd::ZERO);
    if let Some(claimed) = advisories
        .iter()
        .find_map(|a| match a {
            btctax_core::tax::advisories::Advisory::QualifiedAppraisalMustBeAttached {
                claimed,
            } => Some(*claimed),
            _ => None,
        })
        .filter(|_| claims_property_deduction)
    {
        let _ = writeln!(
            manifest,
            "  ATT  qualified appraisal (§170(f)(11)(D) — YOU MUST SUPPLY THIS; btctax cannot \
             generate it). More than $500,000 of charitable deduction is claimed for donated \
             property (${claimed:.2}), so a qualified appraisal must be ATTACHED to this return. Attach a copy \
             again in every §170(d) carryover year (Reg §1.170A-16(f)(3)).",
        );
    }
    // ★ N4 — the marks btctax deliberately did NOT make, enumerated at the FOOT of the manifest: the
    // filer reaches them having just assembled the paper, which is the moment they are actionable.
    // Every one of these blanks is correct; what was missing was any statement that they exist.
    //
    // ★★ MERGE NOTE (phase 1 × phase 2). Both phases append to this manifest and neither knew of the
    // other; the resolution keeps both, in this order, because they are different CATEGORIES and the
    // order encodes that. P5's line above is an ATTACHMENT — a page that goes in the envelope, so it
    // belongs in the stapling list. N4's block below is a set of MARKS ON FORMS the filer must make
    // by hand. The appraisal is deliberately NOT folded into the hand-marks list: it is not a mark,
    // and it is not the filer's to write — only a qualified appraiser can produce it.
    // ★★★ FORM 1040-V (spec R4) — the payment voucher, and its own manifest block.
    //
    // It goes in the envelope but NOT in the stapling list above, and the two facts are the whole
    // point: the voucher says on its own face *"Do not staple or attach this voucher to your payment
    // or return."*, and its instructions again — *"Don't staple or otherwise attach your payment or
    // Form 1040-V"* *"to your return or to each other. Instead, just put them loose in"* *"the
    // envelope."* So it is written BESIDE the packet, listed BELOW the stapling list in a block of its
    // own, and never handed to `FiledPacket::stapled` (whose output IS the stapling order).
    let (form_1040v_path, form_1040v_note) = write_payment_voucher(
        &printed,
        out_dir,
        tax_year,
        voucher,
        watermarked,
        &mut manifest,
    )?;

    let marks = hand_marks(&printed);
    manifest.push_str(&hand_marks_block(&marks));
    let manifest_path = out_dir.join("manifest.txt");
    write_bytes_owner_only(&manifest_path, manifest.as_bytes())?;

    let unresolved_hard = state
        .blockers
        .iter()
        .filter(|b| b.kind.severity() == Severity::Hard)
        .count();
    Ok(IrsPdfReport {
        // The FULL return is a return, not an attachment set, and it files from the COMMITTED row
        // (spec 1099-DA R6's one deliberate exception) — so neither slice note applies here.
        slice_attachment_note: None,
        stale_draft_note: None,
        advisories,
        // ★ P6 — the §170(d)(1) carryover rides out to the caller, which prints it beside the other
        // §170 notes. Taken from the SAME `assemble_absolute` result the packet was printed from, so
        // the figure on the filer's screen is the figure the return produced.
        charitable_carryover_out: ar.charitable_carryover_out.clone(),
        // ★★★ FINAL-REVIEW FINDING 1 — and whether §170(f)(8) stands behind it. The export path is
        // the one that hands the filer a PDF to SIGN, so it is exactly where the acknowledgment
        // deadline has to be named: §170(f)(8)(C) kills the cure at filing.
        charitable_carryover_cwa_unvouched: btctax_core::tax::return_1040::cwa_unvouched_carryover(
            &ri, &ar, state, tax_year,
        ),
        // ★ N4 — the SAME list the manifest rendered, so the stderr count and the paper cannot drift.
        hand_marks: marks,
        watermarked,
        tax_year,
        unresolved_hard,
        // ★ C1 (Fable plan review, 2026-09-05): this was a literal `0`, so the [I5] advisory NEVER
        // fired on the one path that hands the filer a packet to sign — while the slice arm counted
        // the same rows correctly. Read from the SAME `Printed8949` the packet was printed from.
        broker_reported_rows: printed
            .forms
            .f8949
            .as_ref()
            .map_or(0, |f| f.possibly_broker_reported),
        regime,
        form_1040v_path,
        form_1040v_note,
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
        experimental_notice_active,
    })
}

// ── `btctax extension` — Form 4868 (spec SPEC_form_4868_1040v.md R2) ────────────────────────────

/// What `btctax extension` filled and what the filer needs told about it.
///
/// The LINE VALUES ride out here rather than being re-derived for the screen: `btctax_forms`'
/// [`btctax_forms::form_4868_lines`] is asked once, the PDF and this report are both rendered from
/// that one answer, and so the paper and the terminal cannot disagree about what is on line 6.
#[derive(Debug, Clone)]
pub struct ExtensionReport {
    pub tax_year: i32,
    /// The written `f4868.pdf` (owner-only, like every other export).
    pub path: PathBuf,
    /// The ledger was pseudo-reconciled, so every page carries the DRAFT watermark.
    pub watermarked: bool,
    /// Form 4868 Part II exactly as printed — `None` on a line means it is BLANK on the paper.
    pub lines: btctax_forms::Form4868Lines,
    /// The date this application is due: the year record's `return_due`, REPLACED by the
    /// out-of-country June 15 (§7503-shifted) when line 8 is checked.
    pub due: time::Date,
    /// The clock (`BTCTAX_NOW`) is past [`Self::due`]. A WARNING, never a refusal: a filer who is
    /// already late still needs the form, and a useless form is not worse than silence.
    pub past_due: bool,
    /// Line 8 was checked.
    pub out_of_country: bool,
    /// `Some(N)` when the RETURN already prints an extension payment on Schedule 3 line 10 (I-6).
    /// Not a refusal — the field records a payment, not a filing, and recording first and printing
    /// second is the natural order.
    pub recorded_extension_payment: Option<Usd>,
    /// ★ (seam review I-3) A live (non-voided) `DeclareTranche`/`PromoteTranche` is on file, so the
    /// caller prints the Approach-B experimental notice — the SAME field, from the SAME
    /// `uses_approach_b(events)` call, the two export reports carry.
    ///
    /// `btctax_core::experimental`'s own module docs name this class: the notice is correct on
    /// surfaces that merely REFLECT Approach-B, "the export reports (`export-irs-pdf`,
    /// `export-snapshot`, the full-return export)". The dependence here is direct — a live promote
    /// raises basis, lowering the capital gain, lowering Form 1040 line 24, which IS Form 4868
    /// line 4, and therefore line 6 and the default line 7. This is the one form the filer attaches
    /// MONEY to, so it may not be the exception.
    pub experimental_notice_active: bool,
}

/// The date Form 4868 is due, given the year's committed `return_due` and the line-8 choice.
///
/// ★ The out-of-country date is **June 15 of the following year, §7503-shifted** — the date the form
/// itself names (*"If you're out of the country"* *"and file a calendar year income tax return, you
/// can pay the tax and"* *"file your return or this form by June 15, 2026."*) — and NOT
/// `return_due + 2 months`. The two differ whenever the April date was itself shifted: TY2017's
/// committed `return_due` is **2018-04-17** (the Emancipation Day shift), and 04-17 + 2 months is
/// 06-17 against the real **2018-06-15**. Pinned by test in both directions.
///
/// Pure, and deliberately takes the record's date rather than reading it, so a TY2017-shaped record
/// can be exercised on a path TY2017 itself cannot reach (it has no full-return tables, so the
/// command refuses long before the warning).
pub fn extension_due_date(
    tax_year: i32,
    return_due: time::Date,
    out_of_country: bool,
) -> time::Date {
    if !out_of_country {
        // The committed `return_due` has §7503 already applied — shifting it again would move a
        // correct date.
        return return_due;
    }
    let june_15 = time::Date::from_calendar_date(tax_year + 1, time::Month::June, 15)
        .expect("June 15 exists in every year");
    btctax_forms::year_record::section_7503_shift(june_15)
}

/// ★ **The extension application** (spec R2) — Form 4868, written ALONE into `--out`.
///
/// It is not a packet member and never rides with the return: the form's own page 2 says *"Don't
/// attach a copy of Form 4868 to your return."*, and it is mailed weeks earlier in its own envelope.
///
/// **`promote_export_gate` is deliberately NOT applied**, and the spec says why (R2): *"a promoted
/// tranche does lower line 24 and so lines 4/6, but the §1.6662-4(f) disclosure obligation attaches
/// to the RETURN Form 8275 is filed with, not to the extension application; the 1040-V rides the
/// packet path, whose first statement is the gate … so it inherits it (decision, not gap)."*
///
/// Every other gate the packet export runs, this runs — the three fail-closed refusals through the
/// shared [`screen_full_return`], and the pseudo-reconciled attestation + DRAFT watermark, because a
/// form MONEY is attached to may not be the exception (C-2).
#[allow(clippy::too_many_arguments)]
pub fn extension(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: i32,
    pay: Option<Usd>,
    out_of_country: bool,
    attest: Option<&str>,
    now: time::OffsetDateTime,
) -> Result<ExtensionReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (events, state, _cfg) = session.load_events_and_project()?;
    extension_from_session(
        &session,
        &state,
        &events,
        out_dir,
        tax_year,
        pay,
        out_of_country,
        attest,
        now,
    )
}

/// The `&Session` inner of [`extension`] — no `Session::open`, no re-project.
#[allow(clippy::too_many_arguments)]
pub(crate) fn extension_from_session(
    session: &Session,
    state: &btctax_core::state::LedgerState,
    events: &[LedgerEvent],
    out_dir: &Path,
    tax_year: i32,
    pay: Option<Usd>,
    out_of_country: bool,
    attest: Option<&str>,
    now: time::OffsetDateTime,
) -> Result<ExtensionReport, CliError> {
    use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};

    // (1)-(3) The packet export's own three refusals, reused as they stand and in its order — the
    // SAME function it calls, so the wording cannot drift (spec R2). No estimate exists for a year
    // that will not compute, and the instructions demand one *"as accurate as you can"*.
    let tables = BundledTaxTables::load();
    let fr_tables = BundledFullReturnTables::load();
    let screened = screen_full_return(session, state, tax_year, &tables, &fr_tables)?;

    // (4) `--out` is the RETURN's envelope directory. The 4868 is mailed separately and weeks
    // earlier; an unlabelled copy sitting in the return's envelope is the one thing the form
    // forbids, and a filer who collated that directory by filename would attach it.
    if out_dir.join("manifest.txt").exists() {
        return Err(CliError::Usage(format!(
            "{} already holds a manifest.txt — that is a full-return packet directory, and Form 4868 \
             is MAILED SEPARATELY, before the return exists. The form's own page 2 says \"Don't attach \
             a copy of Form 4868 to your return.\", so an unlabelled copy in the return's envelope is \
             exactly what it forbids. Write the extension to its own --out directory.",
            out_dir.display()
        )));
    }

    // (5) `--pay`. Negative or with cents is refused; ABOVE line 6 is allowed — paying ahead of an
    // estimate is the filer's own choice, and the instructions bless paying less as well.
    if let Some(p) = pay {
        if p < Usd::ZERO {
            return Err(CliError::Usage(format!(
                "--pay must be >= 0 (got {p}) — a negative payment is not a payment"
            )));
        }
        if p != p.trunc() {
            return Err(CliError::Usage(format!(
                "--pay must be WHOLE DOLLARS (got {p}). Form 4868's lines 4-6 are printed in whole \
                 dollars, and the form's own rule is all-or-nothing: \"You can round off cents to \
                 whole dollars on Form 4868. If you do round to whole dollars, you must round all \
                 amounts.\" btctax will not round your payment for you — enter the dollars you mean."
            )));
        }
    }

    // (6) The pseudo-reconciled gate (C-2), in the same slot the packet export puts it: pseudo
    // figures are FICTIONAL and can never be filed, so they are attestation-gated and watermarked no
    // matter what. A refusal here has written nothing.
    let watermarked = state.pseudo_active();
    if watermarked {
        require_attestation(attest)?;
    }

    let details = session.donation_details()?;
    let printed = btctax_core::tax::packet::assemble_printed_return(
        &screened.ri,
        state,
        &details,
        &screened.ar,
        screened.table,
        tax_year,
        events,
        screened.regime,
    )
    .map_err(|e| CliError::Usage(format!("the {tax_year} return cannot be printed: {e}")))?;

    let choices = btctax_forms::Form4868Choices {
        pay,
        out_of_country,
    };
    // ★ ONE derivation, two renderings: the report's line values and the PDF's cells come from the
    // same call, so the terminal cannot say one thing and the paper another.
    let lines = btctax_forms::form_4868_lines(&printed, choices)?;
    // ★ The fill runs BEFORE `mkdir_out` — a refusal leaves `--out` untouched, exactly as the packet
    // export's all-or-nothing fill does.
    let bytes = btctax_forms::fill_form_4868(&printed, tax_year, choices)?;
    let bytes = if watermarked {
        btctax_forms::stamp_draft_watermark(&bytes)?
    } else {
        bytes
    };

    mkdir_out(out_dir)?;
    let path = out_dir.join("f4868.pdf");
    write_bytes_owner_only(&path, &bytes)?;

    // The due date, from the YEAR RECORD — never a hardcoded month/day (TY2017's is 2018-04-17).
    let record = btctax_forms::year_record::YearRecord::for_year(tax_year)
        .ok_or_else(|| CliError::FormFill(btctax_forms::FormsError::UnsupportedYear(tax_year)))?;
    let due = extension_due_date(tax_year, record.return_due, out_of_country);

    Ok(ExtensionReport {
        tax_year,
        path,
        watermarked,
        lines,
        due,
        past_due: now.date() > due,
        out_of_country,
        recorded_extension_payment: printed
            .forms
            .sch_3
            .map(|s| s.line10)
            .filter(|v| *v > Usd::ZERO),
        experimental_notice_active: btctax_core::experimental::uses_approach_b(events),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::forms::InformationReturnRegime as Regime;

    /// [I5] r2/NEW-IMPORTANT-1: the broker-reporting advisory is YEAR-AWARE. On the TY2025+
    /// digital-asset revision it must cite the 1099-DA and the digital-asset boxes (G/H/J/K separate,
    /// I/L filed) — never the securities boxes.
    #[test]
    fn broker_advisory_ty2025_cites_1099da_and_digital_asset_boxes() {
        let msg = broker_reporting_advisory(2025, Regime::PROCEEDS_ONLY, 3)
            .expect("3 broker rows → an advisory");
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
        let msg =
            broker_reporting_advisory(2024, Regime::NONE, 1).expect("1 broker row → an advisory");
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
        assert!(broker_reporting_advisory(2026, Regime::PROCEEDS_AND_BASIS, 0).is_none());
        assert!(broker_reporting_advisory(2025, Regime::PROCEEDS_ONLY, 0).is_none());
        assert!(broker_reporting_advisory(2024, Regime::NONE, 0).is_none());
    }

    /// spec 1099-DA T5 (R4): on a live year the advisory names box 1g AND box 1f, the columns to
    /// compare them with, and the correction column — and none of the not-asked wording.
    #[test]
    fn broker_advisory_on_a_live_year_names_box_1g_and_box_1f() {
        let msg = broker_reporting_advisory(2026, Regime::PROCEEDS_AND_BASIS, 2)
            .expect("2 keyed rows → an advisory");
        for needle in [
            "box 1g",
            "box 1f",
            "column (e)",
            "column (d)",
            "column (g)",
            "Note on Form 8949",
            "exchange:PROVIDER:ACCOUNT",
            "1099-DA",
        ] {
            assert!(
                msg.contains(needle),
                "live advisory names {needle:?}:\n{msg}"
            );
        }
        assert!(
            !msg.contains("reclassify by hand"),
            "the live year ROUTED, it does not ask for a hand reclassification:\n{msg}"
        );
        // ★ r3 M-1 — the parenthetical says what ACTUALLY happened, so it names all THREE box
        //   pairs the answers can choose; a filer who answered `not_reported` everywhere has every
        //   row on I/L and no 1099-DA lists any of them. The old needle here was `!contains("Box
        //   I/L")`, which the honest wording would have red for the wrong reason.
        for pair in ["I/L", "H/K", "G/J"] {
            assert!(
                msg.contains(pair),
                "the live advisory must name the box pair {pair:?} your answer can choose:\n{msg}"
            );
        }
        // …and the NOT-LIVE blanket-filing sentence stays absent — the guarantee the old needle held,
        // pinned on the sentence itself rather than on a letter pair the live wording now shares.
        assert!(
            !msg.contains("This export files EVERY"),
            "no blanket single-box filing claim on a live year:\n{msg}"
        );
        // and the not-live wordings never say 1g/1f — they have nothing to compare against
        for (y, r) in [(2025, Regime::PROCEEDS_ONLY), (2024, Regime::NONE)] {
            let m = broker_reporting_advisory(y, r, 1).unwrap();
            assert!(!m.contains("box 1g") && !m.contains("box 1f"), "TY{y}: {m}");
            // ★ the needle the live assertion above is keyed to must actually exist here, or that
            //   assertion is green because the sentence was renamed rather than because it is absent.
            assert!(m.contains("This export files EVERY"), "TY{y}: {m}");
        }
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

/// ★ spec 1099-DA R1 — the crypto-slice arm's Form 1099-DA refusal, as a pure predicate so it can be
/// planted red in every direction (B1): `Some` iff the question is LIVE for these rows (the year's
/// regime reports basis AND ≥ 1 row was disposed on an exchange). The answers are not consulted —
/// they live on `ReturnInputs`, which this arm has by construction not got.
/// ★★★ spec 1099-DA R6 (I-1/I-7) — **THE FORM-LEVEL GATE for the crypto slice's answer-filed arm.**
///
/// A year is ported form by form. `SUPPORTED_YEARS` answers a YEAR-level question — *does any form
/// of this year fill?* — and a year that answers yes can still be missing the map for a form THIS
/// export will write. Without this gate the missing map surfaces mid-write, after `out_dir` and
/// `basis_methodology.txt` are already on disk: a directory a filer could mail with the disclosure
/// in it and the form it discloses absent.
///
/// `--forms` narrows the set through [`wants`], and the three conditional forms are demanded only
/// when this year's DATA will reach them. The refusal names the FIRST missing stem, in the order
/// the packet writes them.
fn slice_map_gate(
    tax_year: i32,
    forms: &[FormArg],
    needs_8283: bool,
    needs_se: bool,
    needs_8275: bool,
) -> Result<(), CliError> {
    let mut probes: Vec<(&'static str, Result<(), btctax_forms::FormsError>)> = Vec::new();
    if wants(forms, FormArg::F8949) {
        probes.push((
            "f8949",
            btctax_forms::Form8949Map::for_year(tax_year).map(|_| ()),
        ));
    }
    if wants(forms, FormArg::ScheduleD) {
        probes.push((
            "schedule_d",
            btctax_forms::ScheduleDMap::for_year(tax_year).map(|_| ()),
        ));
    }
    if wants(forms, FormArg::Form1040) {
        probes.push((
            "f1040",
            btctax_forms::Form1040Map::for_year(tax_year).map(|_| ()),
        ));
    }
    if needs_8283 && wants(forms, FormArg::Form8283) {
        probes.push((
            "f8283",
            btctax_forms::Form8283Map::for_year(tax_year).map(|_| ()),
        ));
    }
    if needs_se && wants(forms, FormArg::ScheduleSe) {
        probes.push((
            "schedule_se",
            btctax_forms::ScheduleSeMap::for_year(tax_year).map(|_| ()),
        ));
    }
    if needs_8275 {
        // NOT behind `wants`: the Form 8275 disclosure rides the packet unconditionally whenever a
        // promoted disposal leg files (BG-D8 / whole-branch tax M-1).
        probes.push((
            "f8275",
            btctax_forms::Form8275Map::for_year(tax_year).map(|_| ()),
        ));
    }
    first_unresolved_map(tax_year, &probes)
}

/// ★ spec 1099-DA R6 fold (M-2) — WHEN the form-level map gate runs on the crypto-slice path.
///
/// R6 built it as `if files_from_answers`, i.e. arm (2) only, so a partially ported year reached on
/// arm (3) got no "write nothing" guarantee at all. The gate reads only the year and the selected
/// forms — nothing about the answers — so the arm is the wrong axis, and it now runs on both.
///
/// The one condition it keeps: on arm (3) a WHOLLY unported year (no bundled template at all) still
/// gets the YEAR-level answer instead — the typed `FormsError::UnsupportedYear` refusal raised
/// further down, which `export_irs_pdf::unsupported_year_is_refused` pins. That is a different
/// question from "which of the forms you selected cannot fill", and only the second one is R6's; on
/// arm (2) R6 mandates the form-level message for such a year (TY2026), so the gate is unconditional
/// there.
fn form_level_gate_runs(files_from_answers: bool, tax_year: i32) -> bool {
    files_from_answers || btctax_forms::SUPPORTED_YEARS.contains(&tax_year)
}

/// The pure half of [`slice_map_gate`]: the FIRST probe whose map did not resolve, as the refusal a
/// filer reads. Split out so a kill can plant a partially ported year — a probe list with one map
/// bound and the next missing — which no BUNDLED year is.
fn first_unresolved_map(
    tax_year: i32,
    probes: &[(&str, Result<(), btctax_forms::FormsError>)],
) -> Result<(), CliError> {
    let Some((stem, err)) = probes
        .iter()
        .find_map(|(stem, r)| r.as_ref().err().map(|e| (*stem, e)))
    else {
        return Ok(());
    };
    Err(CliError::Usage(format!(
        "cannot export TY{tax_year}: this build has no usable `{stem}` map for the year ({err}), and \
         the crypto slice would write that form. A PARTIALLY ported year must write nothing rather \
         than a packet missing one of its forms — a filer collating the directory could not tell \
         which. No forms were written; use `--forms` to select only the forms this build can fill, \
         or wait for the year's package."
    )))
}

pub fn slice_broker_refusal(
    tax_year: i32,
    regime: btctax_core::InformationReturnRegime,
    rows: &[btctax_core::Form8949Row],
) -> Option<CliError> {
    if !btctax_core::broker_question_is_live(rows, regime) {
        return None;
    }
    let n = rows
        .iter()
        .filter(|r| btctax_core::broker_key(r).is_some())
        .count();
    // ★★★ spec 1099-DA R6 (M-13) — THE EXIT SENTENCE NAMES THE EXIT **THIS STATE ACTUALLY HAS**, and
    //     the two states are told apart by one fact: are the year's full-return parameters bundled?
    //
    //     - bundled → a committed return CAN compute, so the exit is COMMITTING one (the fourth
    //       dispatch cell: answers held in the TUI draft, parameters bundled, no committed row —
    //       telling that filer to "answer" names a step they have already done).
    //     - not bundled → no full return exists for the year at any input, and since R6 the SLICE
    //       itself fills from the answers. "Then export the full return" would name an exit the
    //       build does not have.
    let params_bundled = {
        use btctax_core::tax::tables::FullReturnTables;
        btctax_adapters::BundledFullReturnTables::load()
            .full_return_for(tax_year)
            .is_some()
    };
    let exit = if params_bundled {
        "commit the return in the TUI input form (the commit succeeds once the year's parameters \
         are bundled), then export the full return"
    } else {
        "answer them in the TUI input form (the Form 1099-DA block lists your venues) or via \
         `income import` (the `[broker_reporting.<provider>]` table), then export again — the \
         crypto slice fills from the answers; a full return is not required"
    };
    Some(CliError::Usage(format!(
        "TY{tax_year} Form 8949 needs the Form 1099-DA answers for its {n} exchange row(s), and those \
         live on the return inputs — {exit}. No forms were written."
    )))
}

#[cfg(test)]
mod slice_broker_tests {
    use super::*;
    use btctax_core::forms::Cohort;
    use btctax_core::{Form8949Box, Form8949Part, Form8949Row, InformationReturnRegime};
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn row(exchange: bool) -> Form8949Row {
        Form8949Row {
            part: Form8949Part::ShortTerm,
            box_: Form8949Box::I,
            box_needs_review: exchange,
            cohort: Cohort::Covered,
            description: "1.00000000 BTC".into(),
            date_acquired: date!(2026 - 02 - 01),
            date_sold: date!(2026 - 06 - 01),
            proceeds: dec!(1000),
            cost_basis: dec!(400),
            adjustment_code: String::new(),
            adjustment_amount: dec!(0),
            gain: dec!(600),
            wallet: if exchange {
                btctax_core::WalletId::Exchange {
                    provider: "coinbase".into(),
                    account: "default".into(),
                }
            } else {
                btctax_core::WalletId::SelfCustody {
                    label: "cold".into(),
                }
            },
            disposition_kind: btctax_core::event::DisposeKind::Sell,
        }
    }

    /// ★★★ spec 1099-DA R6 (I-1/I-7) KILL — **the partially ported year.** No BUNDLED year is one
    /// (2017/2024/2025 each carry every map their slice can reach), so the fixture is a probe list:
    /// one map bound, the next missing. The gate names the FIRST missing stem in packet order, and
    /// a fully bound list passes — without which a gate that refused everything would look right.
    #[test]
    fn the_form_level_gate_names_the_first_missing_stem() {
        let missing = || Err(btctax_forms::FormsError::UnsupportedYear(2099));
        // Nothing bound at all → the first probe, `f8949`.
        let e = first_unresolved_map(2099, &[("f8949", missing()), ("schedule_d", missing())])
            .expect_err("a year with no f8949 map must refuse")
            .to_string();
        assert!(
            e.contains("no usable `f8949` map") && e.contains("No forms were written"),
            "{e}"
        );

        // The f8949 map ALONE → still refused, and now it names `schedule_d`.
        let e2 = first_unresolved_map(2099, &[("f8949", Ok(())), ("schedule_d", missing())])
            .expect_err("one bound map is not a ported year")
            .to_string();
        assert!(e2.contains("no usable `schedule_d` map"), "{e2}");

        // …and a fully bound list passes.
        first_unresolved_map(2099, &[("f8949", Ok(())), ("schedule_d", Ok(()))])
            .expect("every map resolves ⇒ the gate is silent");
    }

    /// ★ …and the LIVE gate agrees with the build: every bundled year the slice can fill passes it
    /// with the full form set selected, so the probe fixture above is not testing a rule the real
    /// call site never satisfies.
    #[test]
    fn every_bundled_slice_year_passes_the_form_level_gate() {
        for year in btctax_forms::SUPPORTED_YEARS {
            slice_map_gate(*year, &[], true, true, true)
                .unwrap_or_else(|e| panic!("TY{year} must pass its own form-level gate: {e}"));
        }
    }

    /// ★★★ spec 1099-DA R6 fold (M-2) KILL — the form-level map gate is NOT scoped to arm (2).
    ///
    /// R6 built the call site as `if files_from_answers`, so a partially ported year reached on arm
    /// (3) — no stored answers — got none of R6's "write nothing" guarantee. The gate reads the year
    /// and the selected forms only, so the arm was never the right axis. Reverting the condition to
    /// `files_from_answers` reds the two arm-(3) assertions below.
    ///
    /// The one condition kept is the last pair: on arm (3) a WHOLLY unported year keeps its
    /// YEAR-level typed refusal (`FormsError::UnsupportedYear`, pinned by
    /// `export_irs_pdf::unsupported_year_is_refused`) rather than a form-level one, while on arm (2)
    /// R6 mandates the form-level message even there.
    #[test]
    fn the_form_level_gate_runs_on_arm_3_too() {
        for year in btctax_forms::SUPPORTED_YEARS {
            assert!(
                form_level_gate_runs(false, *year),
                "TY{year} is a bundled slice year: arm (3) must get the form-level gate too"
            );
        }
        // arm (2) is unchanged — the gate runs whatever the year.
        assert!(form_level_gate_runs(true, 2026));
        assert!(form_level_gate_runs(true, 2023));
        // …and on arm (3) a wholly unported year is answered at the YEAR level instead.
        assert!(!form_level_gate_runs(false, 2026));
        assert!(!form_level_gate_runs(false, 2023));
    }

    /// ★ the three directions the spec names: a live year refuses before any byte and names the
    /// exit; TY2025 (proceeds only) fills; a live year with only self-custody rows fills (S10).
    #[test]
    fn the_slice_arm_refuses_only_on_a_live_year() {
        let e = slice_broker_refusal(
            2026,
            InformationReturnRegime::PROCEEDS_AND_BASIS,
            &[row(true)],
        )
        .expect("live: refuse");
        let msg = e.to_string();
        assert!(
            msg.contains("income import") && msg.contains("No forms were written"),
            "{msg}"
        );
        // ★★★ spec 1099-DA R6 (M-13) — THE EXIT SENTENCE NAMES THE EXIT THIS STATE HAS, and the two
        //     states differ by ONE fact: are the year's full-return parameters bundled?
        //
        //     TY2026's are not, so the exit is ANSWERING and re-exporting — the slice itself fills
        //     from the answers. (Before R6 this sentence said "then export the FULL return" and
        //     carried `import_note`, which named an exit this build does not have: no input makes
        //     TY2026 compute a full return. The readiness parenthetical went with it.)
        assert!(
            msg.contains("then export again")
                && msg.contains("a full return is not required")
                && !msg.contains("export the full return"),
            "a params-LESS year's exit is the slice itself:\n{msg}"
        );
        assert!(
            !msg.contains("these inputs are stored now"),
            "…and the `import_note` parenthetical, which described the full-return exit, is gone:\n{msg}"
        );
        // TY2024's parameters ARE bundled, so the exit that state has is COMMITTING a return — the
        // fourth dispatch cell (answers in the draft, parameters bundled, no committed row).
        let bundled = slice_broker_refusal(
            2024,
            InformationReturnRegime::PROCEEDS_AND_BASIS,
            &[row(true)],
        )
        .expect("live: refuse")
        .to_string();
        assert!(
            bundled.contains("commit the return in the TUI input form")
                && bundled.contains("then export the full return"),
            "a params-bundled year's exit is committing, not answering:\n{bundled}"
        );
        assert!(
            slice_broker_refusal(2025, InformationReturnRegime::PROCEEDS_ONLY, &[row(true)])
                .is_none(),
            "TY2025 fills"
        );
        assert!(
            slice_broker_refusal(2024, InformationReturnRegime::NONE, &[row(true)]).is_none(),
            "TY2024 fills"
        );
        assert!(
            slice_broker_refusal(
                2026,
                InformationReturnRegime::PROCEEDS_AND_BASIS,
                &[row(false)]
            )
            .is_none(),
            "self-custody only: fills (S10's limb)"
        );
        assert!(
            slice_broker_refusal(2026, InformationReturnRegime::PROCEEDS_AND_BASIS, &[]).is_none(),
            "no rows: fills"
        );
    }
}
