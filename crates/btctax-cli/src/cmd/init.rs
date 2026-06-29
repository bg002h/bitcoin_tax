//! `init` — create the encrypted vault (`Vault::create`), initialize the core event schema + CLI
//! config table (via `Session::create`), and FORCE the §8 key-backup step. The key-backup path is a
//! required argument: a vault with no backed-up key is unrecoverable, so `init` never skips it.
use crate::{CliError, Session};
use btctax_store::Passphrase;
use std::path::Path;

/// Create the vault and force a key backup. Delegates to `run_with_repair(.., false)`.
/// The ~24 existing callers (tests + main.rs) use this 3-arg form and are UNCHANGED (R0-I1).
pub fn run(vault_path: &Path, pp: &Passphrase, key_backup_path: &Path) -> Result<(), CliError> {
    run_with_repair(vault_path, pp, key_backup_path, false)
}

/// Like `run`, but when `repair` is true delegates to `Session::repair` to clear a half-created
/// vault (orphan key + no pgp/bak) before initializing fresh. Called by `main.rs` `init --repair`.
pub fn run_with_repair(
    vault_path: &Path,
    pp: &Passphrase,
    key_backup_path: &Path,
    repair: bool,
) -> Result<(), CliError> {
    let session = if repair {
        Session::repair(vault_path, pp)?
    } else {
        Session::create(vault_path, pp)?
    };
    // §8 key lifecycle: a forced backup of the passphrase-protected key (HIGH-security write, owner-only).
    session.vault().backup_key(key_backup_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_store::Passphrase;

    #[test]
    fn init_creates_vault_key_and_forced_backup() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let backup = dir.path().join("backup/key.asc");
        run(&vault, &Passphrase::new("pw".into()), &backup).unwrap();
        assert!(vault.exists(), "vault.pgp written");
        assert!(dir.path().join("vault.key").exists(), "sidecar key written");
        assert!(backup.exists(), "forced key backup written");
    }

    #[test]
    fn init_refuses_to_clobber_an_existing_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let backup = dir.path().join("k.asc");
        run(&vault, &Passphrase::new("pw".into()), &backup).unwrap();
        let err = run(&vault, &Passphrase::new("pw".into()), &backup).unwrap_err();
        assert!(matches!(
            err,
            CliError::Store(btctax_store::StoreError::AlreadyExists)
        ));
    }

    /// `init --repair` on a half-created vault (key present, pgp absent) succeeds;
    /// `Session::open` round-trips afterwards.
    #[test]
    fn init_repair_recovers_a_half_created_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let backup = dir.path().join("k.asc");
        let pp = Passphrase::new("pw".into());
        // Create a proper vault, then remove vault.pgp (and .bak if any) to leave half-created state.
        run(&vault, &pp, &backup).unwrap();
        std::fs::remove_file(&vault).unwrap();
        let bak = btctax_store::paths::bak_of(&vault);
        if bak.exists() {
            std::fs::remove_file(&bak).unwrap();
        }
        // vault.key still present; vault.pgp absent → half-created
        let backup2 = dir.path().join("k2.asc");
        run_with_repair(&vault, &pp, &backup2, true).unwrap();
        // Must be able to open after repair
        crate::Session::open(&vault, &pp).unwrap();
    }

    /// The 3-arg `run` (repair=false) on a half-created vault returns `HalfCreatedVault`, not
    /// `AlreadyExists`. This verifies the 3-arg wrapper is untouched (R0-I1).
    #[test]
    fn init_without_repair_on_half_created_errors() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let backup = dir.path().join("k.asc");
        let pp = Passphrase::new("pw".into());
        run(&vault, &pp, &backup).unwrap();
        std::fs::remove_file(&vault).unwrap();
        let bak = btctax_store::paths::bak_of(&vault);
        if bak.exists() {
            std::fs::remove_file(&bak).unwrap();
        }
        // 3-arg run → repair=false → HalfCreatedVault
        let err = run(&vault, &pp, &backup).unwrap_err();
        assert!(
            matches!(
                err,
                CliError::Store(btctax_store::StoreError::HalfCreatedVault(_))
            ),
            "expected HalfCreatedVault, got: {err:?}"
        );
    }
}
