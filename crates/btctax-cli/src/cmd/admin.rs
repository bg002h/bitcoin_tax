//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the pre-2025 lot method; export/backup arrive in Task 15.
use crate::config::{set_fee_treatment, set_pre2025_method as config_set_pre2025_method};
use crate::render::write_csv_exports;
use crate::{require_attestation, CliConfig, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{compute_se_tax, FeeTreatment, LotMethod, Severity, TaxTables};
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
    // Attestation gate: REFUSE before ANY bytes are written when a synthetic default contributes and the
    // attestation is missing/wrong — no fictional snapshot/8949/Schedule D leaves the machine unguarded.
    // Checked FIRST (before the vault snapshot / CSV writes), so a refused export leaves out_dir untouched.
    let (state, _cfg) = session.project()?;
    if state.pseudo_active() {
        require_attestation(attest)?;
    }
    let sqlite = session.vault().export_snapshot(out_dir)?; // writes out_dir/snapshot.sqlite
                                                            // P2-D: standalone Schedule SE §1401 figure for the year-scoped export. Needs the year's filing
                                                            // status (profile) + the year's ss_wage_base (bundled table); `None` when either is absent or
                                                            // there is no business SE income. The "present but no table" note is a text-report concern
                                                            // (render_schedule_se) — the CSV carries the computed figure only.
    let se_result = match tax_year {
        Some(y) => {
            let tables = BundledTaxTables::load();
            session.tax_profile(y)?.and_then(|p| {
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
    write_csv_exports(
        out_dir,
        &state,
        tax_year,
        se_result.as_ref(),
        &donation_details,
    )?;
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

/// Outcome of `export_irs_pdf`: the two written PDF paths, the unresolved-Hard-blocker count (same
/// INFORMATIONAL disclosure as `export-snapshot`), whether the fill was watermarked (pseudo-active),
/// and the count of rows that MIGHT belong on a separate 1099-DA-reported 8949 (Box G/H/J/K — the
/// [I5] advisory). SP1 files EVERY Bitcoin row under Box I/L and says so.
#[derive(Debug, Clone)]
pub struct IrsPdfReport {
    pub f8949_path: PathBuf,
    pub schedule_d_path: PathBuf,
    pub tax_year: i32,
    pub unresolved_hard: usize,
    pub broker_reported_rows: usize,
    pub watermarked: bool,
}

/// `export-irs-pdf`: fill the OFFICIAL IRS PDFs (Form 8949 + Schedule D) for `tax_year` and write them
/// (owner-only) to `out_dir`. The form data is REUSED from the projection (`form_8949`/`schedule_d`) —
/// nothing is recomputed. Same pseudo-active attestation gate as `export-snapshot`: checked FIRST, so
/// a refused export leaves `out_dir` untouched; a pseudo fill is additionally DRAFT-watermarked.
pub fn export_irs_pdf(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: i32,
    attest: Option<&str>,
) -> Result<IrsPdfReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;
    // Attestation gate FIRST — no fictional tax form leaves the machine unguarded, and a refusal
    // writes no bytes. (A fully-real ledger ignores `attest`.)
    let watermarked = state.pseudo_active();
    if watermarked {
        require_attestation(attest)?;
    }

    // Reuse the projection's form data verbatim (no recompute).
    let rows = btctax_core::form_8949(&state, tax_year);
    let totals = btctax_core::schedule_d(&state, tax_year);

    // Fill the official PDFs. The engine reads each output back geometrically and FAILS CLOSED on a
    // mis-mapped cell (→ CliError::FormFill) — a wrong tax form is never written.
    let mut f8949 = btctax_forms::fill_form_8949(&rows, tax_year)?;
    let mut schedule_d = btctax_forms::fill_schedule_d(&totals, tax_year)?;
    if watermarked {
        f8949 = btctax_forms::stamp_draft_watermark(&f8949)?;
        schedule_d = btctax_forms::stamp_draft_watermark(&schedule_d)?;
    }

    fsperms::mkdir_owner_only(out_dir)?;
    let f8949_path = out_dir.join("f8949.pdf");
    let schedule_d_path = out_dir.join("schedule_d.pdf");
    write_bytes_owner_only(&f8949_path, &f8949)?;
    write_bytes_owner_only(&schedule_d_path, &schedule_d)?;

    let unresolved_hard = state
        .blockers
        .iter()
        .filter(|b| b.kind.severity() == Severity::Hard)
        .count();
    Ok(IrsPdfReport {
        f8949_path,
        schedule_d_path,
        tax_year,
        unresolved_hard,
        broker_reported_rows: btctax_forms::rows_possibly_broker_reported(&rows),
        watermarked,
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

/// §8: export the passphrase-protected key (escape hatch; HIGH-security write).
pub fn backup_key(vault_path: &Path, pp: &Passphrase, out_path: &Path) -> Result<(), CliError> {
    Session::open(vault_path, pp)?
        .vault()
        .backup_key(out_path)?;
    Ok(())
}
