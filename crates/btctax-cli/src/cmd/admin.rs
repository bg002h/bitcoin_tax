//! `config`, `export-snapshot` (FR10), `backup-key` — administrative commands. Config surfaces the TP8
//! (c)/(b) treatment + the (single-variant) lot method; export/backup arrive in Task 15.
use crate::config::set_fee_treatment;
use crate::{CliConfig, CliError, Session};
use btctax_core::FeeTreatment;
use btctax_store::Passphrase;
use std::path::Path;

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
