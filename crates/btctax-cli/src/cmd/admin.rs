//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the pre-2025 lot method; export/backup arrive in Task 15.
use crate::config::{set_fee_treatment, set_pre2025_method as config_set_pre2025_method};
use crate::render::write_csv_exports;
use crate::{require_attestation, CliConfig, CliError, Session};
use btctax_adapters::BundledTaxTables;
use btctax_core::{compute_se_tax, FeeTreatment, LotMethod, Severity, TaxTables};
use btctax_store::Passphrase;
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

/// §8: export the passphrase-protected key (escape hatch; HIGH-security write).
pub fn backup_key(vault_path: &Path, pp: &Passphrase, out_path: &Path) -> Result<(), CliError> {
    Session::open(vault_path, pp)?
        .vault()
        .backup_key(out_path)?;
    Ok(())
}
