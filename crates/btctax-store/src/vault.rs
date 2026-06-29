use crate::{
    atomic, blob,
    crypto::{self, Passphrase},
    lock::VaultLock,
    memlock::SecretBuf,
    paths, sqlite_io, StoreError, SCHEMA_VERSION,
};
use openpgp::parse::Parse;
use openpgp::serialize::{Serialize, SerializeInto};
use rusqlite::Connection;
use sequoia_openpgp as openpgp;
use std::path::{Path, PathBuf};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Write `data` to `path` with owner-only permissions (mode 0o600 on Unix).
/// On non-Unix platforms the file inherits the user-profile directory ACL.
#[cfg(unix)]
fn write_owner_only(path: &Path, data: &[u8]) -> Result<(), StoreError> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(data))?;
    Ok(())
}

#[cfg(not(unix))]
fn write_owner_only(path: &Path, data: &[u8]) -> Result<(), StoreError> {
    std::fs::write(path, data)?;
    Ok(())
}

/// Create `path` (and all parents) with owner-only permissions (mode 0o700 on Unix).
/// On non-Unix platforms uses `create_dir_all`.
#[cfg(unix)]
fn mkdir_owner_only(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    Ok(())
}

#[cfg(not(unix))]
fn mkdir_owner_only(path: &Path) -> Result<(), StoreError> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

// ── Vault ─────────────────────────────────────────────────────────────────────

pub struct Vault {
    path: PathBuf,
    cert: openpgp::Cert,
    conn: Connection,
    _lock: VaultLock,
}

impl Vault {
    pub fn create(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        if vault.extension().and_then(|e| e.to_str()) == Some("key") {
            return Err(StoreError::InvalidVaultPath);
        } // M1
        if let Some(parent) = vault.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        } // M3
        let lock = VaultLock::acquire(vault)?; // lock FIRST — no TOCTOU (Nit-1)
        let kp = paths::suffixed_key(vault);
        if vault.exists() || kp.exists() {
            return Err(StoreError::AlreadyExists);
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
