//! `verify` (FR9) + `report`/`show` (FR4) — read-only inspection of the pure projection. `verify`
//! arrives in Task 6; this file starts with `report`.
use crate::{CliError, Session};
use btctax_core::LedgerState;
use btctax_store::Passphrase;
use std::path::Path;

/// FR4: project the ledger for display. `year` filters realized sections in the renderer; holdings are
/// always the current per-lot position.
pub fn report(
    vault_path: &Path,
    pp: &Passphrase,
    _year: Option<i32>,
) -> Result<LedgerState, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;
    Ok(state)
}
