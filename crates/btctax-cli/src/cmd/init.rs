//! `init` — create the encrypted vault (`Vault::create`), initialize the core event schema + CLI
//! config table (via `Session::create`), and FORCE the §8 key-backup step. The key-backup path is a
//! required argument: a vault with no backed-up key is unrecoverable, so `init` never skips it.
use crate::{CliError, Session};
use btctax_store::Passphrase;
use std::path::Path;

pub fn run(vault_path: &Path, pp: &Passphrase, key_backup_path: &Path) -> Result<(), CliError> {
    let session = Session::create(vault_path, pp)?;
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
}
