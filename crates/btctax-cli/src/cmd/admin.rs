//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the pre-2025 lot method; export/backup arrive in Task 15.
use crate::config::{set_fee_treatment, set_pre2025_method as config_set_pre2025_method};
use crate::render::write_csv_exports;
use crate::{CliConfig, CliError, Session};
use btctax_core::{FeeTreatment, LotMethod};
use btctax_store::Passphrase;
use std::path::{Path, PathBuf};

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
pub fn export_snapshot(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
    tax_year: Option<i32>,
) -> Result<PathBuf, CliError> {
    let session = Session::open(vault_path, pp)?;
    let sqlite = session.vault().export_snapshot(out_dir)?; // writes out_dir/snapshot.sqlite
    let (state, _cfg) = session.project()?;
    write_csv_exports(out_dir, &state, tax_year)?;
    Ok(sqlite)
}

/// §8: export the passphrase-protected key (escape hatch; HIGH-security write).
pub fn backup_key(vault_path: &Path, pp: &Passphrase, out_path: &Path) -> Result<(), CliError> {
    Session::open(vault_path, pp)?
        .vault()
        .backup_key(out_path)?;
    Ok(())
}
