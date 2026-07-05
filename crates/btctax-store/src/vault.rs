use crate::{
    atomic, blob,
    crypto::{self, Passphrase},
    fsperms::{mkdir_owner_only, open_owner_only, write_owner_only},
    lock::VaultLock,
    memlock::SecretBuf,
    paths, sqlite_io, StoreError, SCHEMA_VERSION,
};
use openpgp::parse::Parse;
use openpgp::serialize::{Serialize, SerializeInto};
use rusqlite::Connection;
use sequoia_openpgp as openpgp;
use std::io::Write as _;
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
        // [R0-N1r] A corrupt `vault.key` surfaces here at `Cert::from_bytes` and PROPAGATES with
        // NO `.bak` retry — the KEY is not `.bak`-recoverable (only the encrypted image is).
        let cert = openpgp::Cert::from_bytes(&std::fs::read(&kp)?).map_err(StoreError::Crypto)?;
        let ct = std::fs::read(vault)?;
        match Self::decode_conn(&cert, pp, &ct) {
            Ok(conn) => Ok(Vault {
                path: vault.to_path_buf(),
                cert,
                conn,
                _lock: lock,
            }),
            // [R0-M1] GENUINE corruption of the PRESENT vault AND a `.bak` exists → ONE bounded
            // recovery attempt from `.bak`. WrongPassphrase / UnsupportedSchema / Io / Locked never
            // satisfy `is_genuine_corruption`, so they propagate unchanged and NEVER touch `.bak`.
            Err(orig) if Self::is_genuine_corruption(&orig) && paths::bak_of(vault).exists() => {
                let bak_ct = std::fs::read(paths::bak_of(vault))?;
                match Self::decode_conn(&cert, pp, &bak_ct) {
                    Ok(conn) => {
                        // [R0-I2] WARN — a silent revert to the prior save generation is a
                        // data-integrity surprise the operator must see (precedent: memlock.rs:16).
                        eprintln!(
                            "warning: vault.pgp was corrupt; recovered from vault.pgp.bak (prior save generation)"
                        );
                        // [R0-C2] `.bak`-preserving crash-safe restore; NEVER touches `.bak`.
                        Self::restore_from_bak(vault, &bak_ct)?;
                        Ok(Vault {
                            path: vault.to_path_buf(),
                            cert,
                            conn,
                            _lock: lock,
                        })
                    }
                    // `.bak` also unusable → propagate the ORIGINAL target error (bounded: one try,
                    // no panic/loop).
                    Err(_) => Err(orig),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// [R0-M1] Decode ONE encrypted vault blob (`vault.pgp` or its `.bak`) into an in-memory SQLite
    /// connection: `decrypt` → decode blob header → `migrate` → `db_from_bytes`. Shared by `open`
    /// for BOTH the primary target and the `.bak` fallback so the two decode paths are identical.
    /// Holds the plaintext only inside [`SecretBuf`]s (scrubbed on drop). Does NOT touch the
    /// `VaultLock`: the single lock is acquired once in `open` and attached to the resulting Vault.
    fn decode_conn(
        cert: &openpgp::Cert,
        pp: &Passphrase,
        ct: &[u8],
    ) -> Result<Connection, StoreError> {
        let plaintext = SecretBuf::new(crypto::decrypt_with(cert, pp, ct)?);
        let (ver, image) = blob::decode_blob(plaintext.as_slice())?;
        let image = SecretBuf::new(blob::migrate(ver, image.to_vec())?);
        sqlite_io::db_from_bytes(image.as_slice())
    }

    /// Genuine on-disk corruption of a PRESENT `vault.pgp` that a `.bak` might recover:
    /// bad ciphertext (`Crypto`), bad blob framing (`Corrupt`), or a SQLite deserialize failure
    /// (`Sqlite`). Deliberately EXCLUDES:
    ///   * `WrongPassphrase` — caller error; the `.bak` shares the key and would also fail.
    ///   * [R0-C1] `UnsupportedSchema` — a NEWER vault (decode SUCCEEDED); recovering the older
    ///     `.bak` would silently DOWNGRADE and lose newer tax data. NOT corruption.
    ///   * `Io` / `Locked` — not corruption.
    ///
    /// [R0-M2] INVARIANT (pinned): `db_from_bytes` remaps a `sqlite3_malloc64` OOM to
    /// `StoreError::Io` (sqlite_io.rs:40-43), so a `Sqlite` reaching this classifier is ALWAYS a
    /// deserialize failure of a corrupt image, NEVER an allocation failure. If that remap is ever
    /// changed, this classifier must be revisited (an OOM must not be treated as corruption).
    fn is_genuine_corruption(e: &StoreError) -> bool {
        matches!(
            e,
            StoreError::Crypto(_) | StoreError::Corrupt(_) | StoreError::Sqlite(_)
        )
    }

    /// [R0-C2] Crash-safe, `.bak`-PRESERVING restore of `vault.pgp` from already-read `.bak` bytes.
    /// Writes `<vault>.tmp` owner-only → fsync file → rename `.tmp`→`vault.pgp` → [R0-M1r] fsync the
    /// parent dir (mirrors atomic.rs:27-29). **NEVER touches `.bak`** — it stays the sole surviving
    /// good copy, so a crash anywhere in this sequence leaves `.bak` intact and a re-`open` recovers
    /// again. Deliberately does NOT reuse `atomic::atomic_write`, which copies the (corrupt) target
    /// over `.bak` BEFORE the rename — that would clobber the good `.bak` (the C2 bug).
    fn restore_from_bak(vault: &Path, bak_bytes: &[u8]) -> Result<(), StoreError> {
        let tmp = paths::tmp_of(vault);
        {
            let mut f = open_owner_only(&tmp)?;
            f.write_all(bak_bytes)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, vault)?;
        if let Some(dir) = vault.parent() {
            let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
        }
        Ok(())
    }
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
    /// Serialize the in-memory DB, wrap it in the versioned blob, encrypt, and atomically persist.
    ///
    /// **Zeroize bound (defense-in-depth — honest, NOT full at-rest secrecy) [M-2]:** the plaintext
    /// SQLite image and the encoded blob are held in [`SecretBuf`]s that mlock + scrub on drop, so
    /// each plaintext copy on the save path has the smallest possible count and lifetime. This does
    /// NOT make the vault secret at rest while open: the live SQLite connection keeps the plaintext
    /// in its own heap for the whole session (accepted bound). The `.tmp`/`.bak` that `atomic_write`
    /// writes to disk are CIPHERTEXT, not plaintext, so they are not a zeroize concern. [N2r] the
    /// `data.to_vec()` inside `db_to_bytes` (sqlite_io.rs) is a transient plaintext copy that is
    /// moved straight into the `SecretBuf` below and scrubbed with it.
    pub fn save(&mut self) -> Result<(), StoreError> {
        let image = SecretBuf::new(sqlite_io::db_to_bytes(&self.conn)?);
        let blob = SecretBuf::new(blob::encode_blob(SCHEMA_VERSION, image.as_slice()));
        let ct = crypto::encrypt_to(&self.cert, blob.as_slice())?;
        atomic::atomic_write(&self.path, &ct)
    }

    /// Serialize the current in-memory DB image (no disk I/O). A prior `snapshot()` result can be
    /// fed to `restore()` to revert an attempted mutation whose `save()` failed. Reuses the exact
    /// serialization the vault round-trips on every `open()`.
    pub fn snapshot(&self) -> Result<Vec<u8>, StoreError> {
        sqlite_io::db_to_bytes(&self.conn)
    }

    /// Replace the in-memory DB with `image` (a prior `snapshot()`). No disk I/O; the vault file,
    /// `VaultLock`, `cert`, and `path` are untouched, so the exclusive lock is held across the swap.
    /// On `Err` (only `db_from_bytes` OOM) the assignment never runs, so `self.conn` is UNCHANGED —
    /// the caller MUST treat `Err` as "unsaved residue may still be live", never silently swallow it.
    pub fn restore(&mut self, image: &[u8]) -> Result<(), StoreError> {
        self.conn = sqlite_io::db_from_bytes(image)?;
        Ok(())
    }
    pub fn export_snapshot(&self, out_dir: &Path) -> Result<PathBuf, StoreError> {
        // [MEDIUM security] restricted directory + owner-only file (plaintext tax data)
        mkdir_owner_only(out_dir)?;
        // [M-2] scrub the transient plaintext image on drop (the on-disk export is
        // deliberately plaintext SQLite; only the in-memory intermediate is wrapped).
        let image = SecretBuf::new(sqlite_io::db_to_bytes(&self.conn)?);
        let out = out_dir.join("snapshot.sqlite");
        write_owner_only(&out, image.as_slice())?;
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
        // [M-2] wrap the armored key bytes so the in-memory copy scrubs on drop.
        let armored = SecretBuf::new(
            self.cert
                .as_tsk()
                .armored()
                .to_vec()
                .map_err(StoreError::Crypto)?,
        );
        write_owner_only(out_path, armored.as_slice())?;
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

    /// `snapshot()` then a further in-memory mutation then `restore()` reverts the in-memory DB
    /// byte-for-byte to the snapshot (the core of the save-rollback mechanism).
    #[test]
    fn snapshot_restore_reverts_in_memory_mutation() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let mut v = Vault::create(&vault, &pp()).unwrap();
        v.conn().execute("CREATE TABLE t (x INTEGER)", []).unwrap();
        v.conn().execute("INSERT INTO t VALUES (1)", []).unwrap();
        v.save().unwrap();

        let snap = v.snapshot().unwrap();
        // Further mutation, not yet saved to disk.
        v.conn().execute("INSERT INTO t VALUES (2)", []).unwrap();
        let n: i64 = v
            .conn()
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2, "pre-restore: snapshot row + the extra row");

        v.restore(&snap).unwrap();
        let n: i64 = v
            .conn()
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "restore must revert to the snapshot's single row");
        let x: i64 = v
            .conn()
            .query_row("SELECT x FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            x, 1,
            "the surviving row must be the snapshot's, not the reverted insert"
        );
    }

    /// `restore()` performs NO disk I/O — the on-disk vault file bytes are unchanged.
    #[test]
    fn restore_does_not_touch_the_vault_file() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let mut v = Vault::create(&vault, &pp()).unwrap();
        v.conn().execute("CREATE TABLE t (x INTEGER)", []).unwrap();
        v.save().unwrap();
        let snap = v.snapshot().unwrap();
        v.conn().execute("INSERT INTO t VALUES (9)", []).unwrap();

        let before = std::fs::read(&vault).unwrap();
        v.restore(&snap).unwrap();
        let after = std::fs::read(&vault).unwrap();
        assert_eq!(before, after, "restore must not write the vault file");
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

    /// [R0-C2] The restore primitive is `.bak`-PRESERVING and crash-safe: it writes via `.tmp`
    /// → rename to `vault.pgp` and NEVER reads or writes `.bak`. Even with a pre-existing garbage
    /// `.tmp` present, the good `.bak` is byte-identical afterwards (so a crash mid-restore — before
    /// the rename — leaves the sole good copy intact and a re-`open` recovers again), and the target
    /// ends up byte-identical to the `.bak` bytes it was restored from.
    #[test]
    fn restore_preserves_bak_and_is_crash_safe() {
        let dir = tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let bak = paths::bak_of(&vault);
        let tmp = paths::tmp_of(&vault);

        // Pre-state: a corrupt target, a GOOD `.bak`, and a stray garbage `.tmp` (the exact
        // artifacts a kill mid-save/restore can leave). `.bak` bytes are distinct + known.
        let good_bak = b"GOOD-BAK-CIPHERTEXT-BYTES".to_vec();
        std::fs::write(&vault, b"corrupt-target").unwrap();
        std::fs::write(&bak, &good_bak).unwrap();
        std::fs::write(&tmp, b"stray-garbage-tmp").unwrap();

        Vault::restore_from_bak(&vault, &good_bak).unwrap();

        // Target restored to the `.bak` bytes.
        assert_eq!(
            std::fs::read(&vault).unwrap(),
            good_bak,
            "target must be restored byte-for-byte from the `.bak` bytes"
        );
        // `.bak` NEVER touched — still the exact good copy (the C2 guarantee).
        assert!(bak.exists(), "`.bak` must still be present after restore");
        assert_eq!(
            std::fs::read(&bak).unwrap(),
            good_bak,
            "restore must NOT modify `.bak` (it is the sole surviving good copy)"
        );
        // `.tmp` consumed by the rename — no stray left behind.
        assert!(
            !tmp.exists(),
            "`.tmp` must be renamed away, not left as a stray"
        );
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
