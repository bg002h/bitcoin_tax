use crate::{
    atomic, blob,
    crypto::{self, Passphrase},
    fsperms::{mkdir_owner_only, write_owner_only},
    lock::VaultLock,
    memlock::SecretBuf,
    paths, sqlite_io, StoreError, SCHEMA_VERSION,
};
use openpgp::parse::Parse;
use openpgp::serialize::{Serialize, SerializeInto};
use rusqlite::Connection;
use sequoia_openpgp as openpgp;
use std::path::{Path, PathBuf};

// ── Vault ─────────────────────────────────────────────────────────────────────

pub struct Vault {
    path: PathBuf,
    cert: openpgp::Cert,
    conn: Connection,
    _lock: VaultLock,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl Vault {
    pub fn create(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        Self::create_inner(vault, pp, false)
    }

    /// Like `create`, but first clears a known **orphan key** from an interrupted init
    /// (the half-created signature: `vault.key` present, `vault.pgp` and `.bak` absent).
    /// NEVER clears a key when a real or recoverable vault is present — `AlreadyExists` fires first.
    pub fn repair(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        Self::create_inner(vault, pp, true)
    }

    fn create_inner(vault: &Path, pp: &Passphrase, repair: bool) -> Result<Vault, StoreError> {
        if vault.extension().and_then(|e| e.to_str()) == Some("key") {
            return Err(StoreError::InvalidVaultPath);
        } // M1
        if let Some(parent) = vault.parent() {
            if !parent.as_os_str().is_empty() {
                mkdir_owner_only(parent)?; // M3 — 0o700 on Unix, ACL-inherited on non-Unix
            }
        }
        // VaultLock acquired FIRST — this is the TOCTOU protection for the checks below.
        let lock = VaultLock::acquire(vault)?;
        let kp = paths::suffixed_key(vault);
        // Refuse to clobber a real OR recover_target-recoverable vault — even under --repair.
        // This guard MUST come before the orphan-key check so `repair` never deletes the key
        // when a healthy or bak-recoverable vault is present.
        if vault.exists() || paths::bak_of(vault).exists() {
            return Err(StoreError::AlreadyExists);
        }
        if kp.exists() {
            if repair {
                // Half-created: only the orphan key (and any stray .tmp sidecars) remain.
                // SAFETY: the `vault.exists() || bak.exists()` guard above did NOT fire, so
                // there is no real or recoverable vault — removing the key is safe.
                // VaultLock (acquired first) is the TOCTOU protection; .tmp removal is defensive-only
                // (open_owner_only truncates, not O_EXCL, so a stray .tmp is never load-bearing).
                let _ = std::fs::remove_file(&kp);
                let _ = std::fs::remove_file(paths::tmp_of(&kp));
                let _ = std::fs::remove_file(paths::tmp_of(vault));
            } else {
                return Err(StoreError::HalfCreatedVault(kp));
            }
        }
        // on ANY failure, remove partial artifacts so a retry isn't wedged (Minor-1)
        let cleanup = || {
            for f in [
                &kp,
                &paths::tmp_of(&kp),
                &vault.to_path_buf(),
                &paths::tmp_of(vault),
            ] {
                let _ = std::fs::remove_file(f);
            }
        };
        let built = (|| -> Result<(openpgp::Cert, Connection), StoreError> {
            let cert = crypto::generate_cert(pp)?;
            let mut tsk = Vec::new();
            cert.as_tsk()
                .serialize(&mut tsk)
                .map_err(StoreError::Crypto)?;
            atomic::atomic_write(&kp, &tsk)?;
            Ok((cert, sqlite_io::open_in_memory()?))
        })();
        let (cert, conn) = match built {
            Ok(x) => x,
            Err(e) => {
                cleanup();
                return Err(e);
            }
        };
        let mut v = Vault {
            path: vault.to_path_buf(),
            cert,
            conn,
            _lock: lock,
        };
        if let Err(e) = v.save() {
            cleanup();
            return Err(e);
        }
        Ok(v)
    }

    pub fn open(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        if vault.extension().and_then(|e| e.to_str()) == Some("key") {
            return Err(StoreError::InvalidVaultPath);
        } // M1
        let lock = VaultLock::acquire(vault)?;
        let kp = paths::suffixed_key(vault);
        for f in [vault, kp.as_path()] {
            // crash-safety for BOTH sidecars (Minor-2)
            atomic::recover_target(f)?;
            atomic::reap_tmp(f)?;
        }
        // After recover_target/reap_tmp: if vault.pgp is still absent but vault.key exists,
        // this is the half-created signature — give a clear error instead of Io(NotFound).
        if !vault.exists() && kp.exists() {
            return Err(StoreError::HalfCreatedVault(kp));
        }
        let cert = openpgp::Cert::from_bytes(&std::fs::read(&kp)?).map_err(StoreError::Crypto)?;
        let plaintext = SecretBuf::new(crypto::decrypt_with(&cert, pp, &std::fs::read(vault)?)?);
        let (ver, image) = blob::decode_blob(plaintext.as_slice())?;
        let image = SecretBuf::new(blob::migrate(ver, image.to_vec())?);
        let conn = sqlite_io::db_from_bytes(image.as_slice())?;
        Ok(Vault {
            path: vault.to_path_buf(),
            cert,
            conn,
            _lock: lock,
        })
    }
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
    pub fn save(&mut self) -> Result<(), StoreError> {
        let image = sqlite_io::db_to_bytes(&self.conn)?;
        let ct = crypto::encrypt_to(&self.cert, &blob::encode_blob(SCHEMA_VERSION, &image))?;
        atomic::atomic_write(&self.path, &ct)
    }
    pub fn export_snapshot(&self, out_dir: &Path) -> Result<PathBuf, StoreError> {
        // [MEDIUM security] restricted directory + owner-only file (plaintext tax data)
        mkdir_owner_only(out_dir)?;
        let image = sqlite_io::db_to_bytes(&self.conn)?;
        let out = out_dir.join("snapshot.sqlite");
        write_owner_only(&out, &image)?;
        Ok(out)
    }
    pub fn backup_key(&self, out_path: &Path) -> Result<(), StoreError> {
        // N-1: ensure parent directory exists before writing (restricted on Unix)
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                mkdir_owner_only(parent)?;
            }
        }
        // [HIGH security] write S2K-encrypted private key owner-only (mode 0o600 on Unix)
        let armored = self
            .cert
            .as_tsk()
            .armored()
            .to_vec()
            .map_err(StoreError::Crypto)?;
        write_owner_only(out_path, &armored)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn pp() -> Passphrase {
        Passphrase::new("test-pass".into())
    }

    /// `create` on a half-created state (vault.key present, vault.pgp absent) must return
    /// `HalfCreatedVault`, not `AlreadyExists`.
    #[test]
    fn create_on_half_created_returns_half_created_error() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let kp = paths::suffixed_key(&vault);
        std::fs::write(&kp, b"orphan").unwrap();
        let err = Vault::create(&vault, &pp()).unwrap_err();
        assert!(
            matches!(err, StoreError::HalfCreatedVault(_)),
            "expected HalfCreatedVault, got: {err:?}"
        );
    }

    /// `open` on a half-created state must return `HalfCreatedVault`, not `Io(NotFound)`.
    #[test]
    fn open_on_half_created_returns_half_created_error() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let kp = paths::suffixed_key(&vault);
        std::fs::write(&kp, b"orphan").unwrap();
        let err = Vault::open(&vault, &pp()).unwrap_err();
        assert!(
            matches!(err, StoreError::HalfCreatedVault(_)),
            "expected HalfCreatedVault, got: {err:?}"
        );
    }

    /// `repair` clears the orphan key and builds a fresh vault; `open` then round-trips.
    #[test]
    fn repair_clears_orphan_key_and_creates_fresh() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let kp = paths::suffixed_key(&vault);
        std::fs::write(&kp, b"orphan").unwrap();
        Vault::repair(&vault, &pp()).unwrap();
        // open must succeed (write + read a row)
        let mut v = Vault::open(&vault, &pp()).unwrap();
        v.conn()
            .execute("CREATE TABLE IF NOT EXISTS t (x TEXT)", [])
            .unwrap();
        v.save().unwrap();
    }

    /// `repair` on a HEALTHY vault (pgp present) must refuse with `AlreadyExists`;
    /// the key must still exist and `open` must still work afterwards.
    #[test]
    fn repair_refuses_to_clobber_healthy_vault() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let kp = paths::suffixed_key(&vault);
        Vault::create(&vault, &pp()).unwrap();
        assert!(kp.exists(), "key must exist after create");
        let err = Vault::repair(&vault, &pp()).unwrap_err();
        assert!(
            matches!(err, StoreError::AlreadyExists),
            "expected AlreadyExists, got: {err:?}"
        );
        assert!(kp.exists(), "key must still exist after refused repair");
        Vault::open(&vault, &pp()).unwrap(); // must still work
    }

    /// `repair` when only the `.bak` is present (pgp absent, bak present, key present) must
    /// refuse with `AlreadyExists`; key is untouched; `open` recovers from `.bak`.
    #[test]
    fn repair_refuses_when_bak_present() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let bak = paths::bak_of(&vault);
        let kp = paths::suffixed_key(&vault);
        Vault::create(&vault, &pp()).unwrap();
        // simulate: vault.pgp → vault.pgp.bak (pgp absent, bak present, key present)
        std::fs::rename(&vault, &bak).unwrap();
        assert!(!vault.exists());
        assert!(bak.exists());
        assert!(kp.exists(), "key must exist");
        let err = Vault::repair(&vault, &pp()).unwrap_err();
        assert!(
            matches!(err, StoreError::AlreadyExists),
            "expected AlreadyExists, got: {err:?}"
        );
        assert!(kp.exists(), "key must still exist after refused repair");
        // `open` must recover the vault from `.bak`
        Vault::open(&vault, &pp()).unwrap();
    }

    /// [R0-M1] `repair` on a completely clean path (no key/pgp/bak) behaves exactly like `create`.
    #[test]
    fn repair_on_clean_path_behaves_as_create() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Vault::repair(&vault, &pp()).unwrap();
        let mut v = Vault::open(&vault, &pp()).unwrap();
        v.conn()
            .execute("CREATE TABLE IF NOT EXISTS t (x TEXT)", [])
            .unwrap();
        v.save().unwrap();
    }

    /// [R0-M2] `repair` removes orphan `.tmp` sidecars alongside the orphan key.
    #[test]
    fn repair_clears_orphan_tmp_sidecars() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let kp = paths::suffixed_key(&vault);
        let tmp_vault = paths::tmp_of(&vault);
        let tmp_kp = paths::tmp_of(&kp);
        std::fs::write(&kp, b"orphan").unwrap();
        std::fs::write(&tmp_vault, b"orphan_tmp_vault").unwrap();
        std::fs::write(&tmp_kp, b"orphan_tmp_kp").unwrap();
        Vault::repair(&vault, &pp()).unwrap();
        assert!(!tmp_vault.exists(), "orphan vault.pgp.tmp must be gone");
        assert!(!tmp_kp.exists(), "orphan vault.key.tmp must be gone");
        Vault::open(&vault, &pp()).unwrap();
    }
}
