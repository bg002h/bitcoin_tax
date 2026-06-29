//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the (single-variant) lot method; export/backup arrive in Task 15.
use crate::config::set_fee_treatment;
use crate::render::write_csv_exports;
use crate::{CliConfig, CliError, Session};
use btctax_core::FeeTreatment;
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

/// FR10 / NFR2 exception: decrypted SQLite image (via the store) + the projected ledger as CSV.
pub fn export_snapshot(
    vault_path: &Path,
    pp: &Passphrase,
    out_dir: &Path,
) -> Result<PathBuf, CliError> {
    let session = Session::open(vault_path, pp)?;
    let sqlite = session.vault().export_snapshot(out_dir)?; // writes out_dir/snapshot.sqlite
    let (state, _cfg) = session.project()?;
    write_csv_exports(out_dir, &state)?;
    Ok(sqlite)
}

/// §8: export the passphrase-protected key (escape hatch; HIGH-security write).
pub fn backup_key(vault_path: &Path, pp: &Passphrase, out_path: &Path) -> Result<(), CliError> {
    Session::open(vault_path, pp)?
        .vault()
        .backup_key(out_path)?;
    Ok(())
}
