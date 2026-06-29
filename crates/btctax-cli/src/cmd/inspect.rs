//! `verify` (FR9) + `report`/`show` (FR4) — read-only inspection of the pure projection. `verify`
//! arrives in Task 6; this file starts with `report`.
use crate::render::{build_verify, VerifyReport};
use crate::{CliError, Session};
use btctax_core::persistence::load_all;
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

/// FR9: project the ledger → compute the sat-conservation report, partition blockers by severity, and
/// summarize pending reconciliation + safe-harbor status. The binary maps `has_hard_blockers()` to a
/// non-zero exit (a hard blocker gates downstream tax computation, §7.1).
pub fn verify(vault_path: &Path, pp: &Passphrase) -> Result<VerifyReport, CliError> {
    let session = Session::open(vault_path, pp)?;
    let (state, _cfg) = session.project()?;
    let events = load_all(session.conn())?;
    Ok(build_verify(&state, &events))
}
