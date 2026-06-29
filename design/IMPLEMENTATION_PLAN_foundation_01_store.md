# btctax-store (Encrypted Vault) Implementation Plan — Foundation Plan 1 of 4

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `btctax-store`, the PGP-encrypted local vault that persists an opaque `[schema_version][SQLite image]` blob and exposes a live in-memory SQLite handle — durable (atomic write), single-instance-safe (flock), and key/passphrase-protected — with no dependency on the domain model.

**Architecture:** The only on-disk artifact is `vault.pgp` (Sequoia-OpenPGP, encrypted to an app-managed keypair whose secret key is passphrase-protected). At runtime the blob is decrypted into RAM, loaded into an in-memory SQLite database (`:memory:`), operated on, then serialized + re-encrypted + atomically rewritten. No plaintext DB is ever written except the explicit `export-snapshot`.

**Tech Stack:** Rust (edition 2021), `sequoia-openpgp` 1.x (OpenPGP), `rusqlite` (bundled SQLite, `serialize` feature), `rustix` (flock + mlock), `zeroize`, `thiserror`, `tempfile`-free atomic write via `rustix`/`std::fs`.

## Global Constraints
(Spec `design/SPEC_foundation.md`; every task implicitly includes these.)
- **Money/exactness:** N/A in this crate (opaque blob), but never introduce float-based persistence.
- **NFR2 Encryption at rest:** the only artifact written automatically is `vault.pgp`; **no plaintext DB ever written** except the explicit `export-snapshot`.
- **NFR3 Durability:** no save may corrupt/lose the vault — write-temp → `fsync` → atomic `rename` → rotate `.bak`.
- **NFR7 Single-user safety:** concurrent instances must not silently clobber — `flock(LOCK_EX|LOCK_NB)`, fail fast.
- **R1 (best-effort, honest):** `mlock`/`zeroize` are defense-in-depth; warn (don't fail) if `mlock` is unavailable; do not claim full swap protection.
- **R3:** pin an exact `sequoia-openpgp` version and crypto backend before writing crypto code (Task 0).
- **§8 blob layout:** `[schema_version: u32 big-endian][SQLite serialized image bytes]`.
- **Licensing:** workspace `license = "MIT OR Unlicense"`.
- **Validation:** `cargo test` + `cargo clippy -- -D warnings` + `cargo fmt --check` all green; "green" = suite passes AND 0 Critical/0 Important on review.

## File Structure
```
Cargo.toml                      # [workspace] root
crates/btctax-store/
  Cargo.toml                    # crate manifest, pinned deps + crypto backend
  src/lib.rs                    # pub API + StoreError + SCHEMA_VERSION; wires modules
  src/blob.rs                   # encode/decode [version][image]; migrate(version,bytes)
  src/sqlite_io.rs              # in-memory SQLite <-> bytes (serialize/deserialize)
  src/crypto.rs                 # Sequoia: gen_key, encrypt, decrypt (passphrase)
  src/atomic.rs                 # atomic_write (tmp->fsync->rename->.bak), reap_tmp
  src/lock.rs                   # VaultLock (flock LOCK_EX|LOCK_NB)
  src/memlock.rs               # best-effort mlock + zeroize-on-drop buffer
  src/vault.rs                  # Vault: create/open/save/conn/export_snapshot/backup_key
  tests/integration.rs          # end-to-end: create->save->reopen, crash, concurrency
```
**Public interface this plan PRODUCES (consumed by Plan 2 `btctax-core` etc.):**
- `pub const SCHEMA_VERSION: u32`
- `pub enum StoreError` (`thiserror`): `Io`, `Crypto`, `Locked`, `WrongPassphrase`, `Corrupt`, `Sqlite`, `UnsupportedSchema(u32)`.
- `pub struct Vault` with:
  - `fn create(vault_path: &Path, passphrase: &Passphrase) -> Result<Vault, StoreError>` — generate key, write empty encrypted DB.
  - `fn open(vault_path: &Path, passphrase: &Passphrase) -> Result<Vault, StoreError>` — flock, decrypt, mlock, deserialize.
  - `fn conn(&self) -> &rusqlite::Connection` — the live in-memory DB (core runs DDL/queries here).
  - `fn save(&mut self) -> Result<(), StoreError>` — serialize → encrypt → atomic write.
  - `fn export_snapshot(&self, out_dir: &Path) -> Result<(), StoreError>` — write decrypted `snapshot.sqlite` (NFR2 exception).
  - `fn backup_key(&self, out_path: &Path) -> Result<(), StoreError>` — export the passphrase-protected secret key (TSK).
  - `Drop` → zeroize buffers, munlock, release flock.
- `pub struct Passphrase(/* zeroizing String */)` with `fn new(s: String) -> Self`.

---

### Task 0: Workspace scaffold + pin crypto stack (the de-risking spike)

**Files:**
- Create: `Cargo.toml` (workspace), `crates/btctax-store/Cargo.toml`, `crates/btctax-store/src/lib.rs`
- Test: `crates/btctax-store/tests/smoke.rs`

**Interfaces:**
- Produces: a compiling workspace + the chosen `sequoia-openpgp` version & crypto backend (recorded in the crate Cargo.toml and in `FOLLOWUPS.md`).

- [ ] **Step 1: Create the workspace root `Cargo.toml`**
```toml
[workspace]
resolver = "2"
members = ["crates/btctax-store"]

[workspace.package]
edition = "2021"
license = "MIT OR Unlicense"
rust-version = "1.74"
```

- [ ] **Step 2: Create `crates/btctax-store/Cargo.toml` with pinned deps + a chosen crypto backend**

Backend decision (record the choice + rationale in `FOLLOWUPS.md`): `crypto-nettle` is Sequoia's mature default but needs the system `nettle`/`gmp` libs (not a pure-static binary); `crypto-openssl` uses system OpenSSL; `crypto-rust` is pure-Rust (closest to the "self-contained binary" goal) but Sequoia documents it as not recommended for general use. **Default this plan to `crypto-nettle`** (security first); revisit `crypto-rust` only if a fully static binary becomes a hard requirement.
```toml
[package]
name = "btctax-store"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
sequoia-openpgp = { version = "1.21", default-features = false, features = ["crypto-nettle"] }
rusqlite = { version = "0.31", features = ["bundled", "serialize"] }
rustix = { version = "0.38", features = ["fs", "mm"] }
zeroize = { version = "1", features = ["zeroize_derive"] }
thiserror = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Stub `src/lib.rs` so the crate compiles**
```rust
//! btctax-store: PGP-encrypted local vault for the bitcoin_tax ledger.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("openpgp: {0}")] Crypto(#[from] anyhow::Error),
    #[error("another instance holds the vault lock")] Locked,
    #[error("wrong passphrase or corrupt key")] WrongPassphrase,
    #[error("vault blob is corrupt: {0}")] Corrupt(String),
    #[error("sqlite: {0}")] Sqlite(#[from] rusqlite::Error),
    #[error("unsupported schema version {0}")] UnsupportedSchema(u32),
}
```
(Add `anyhow = "1"` to `[dependencies]` — Sequoia returns `anyhow::Result`.)

- [ ] **Step 4: Write a smoke test that exercises the Sequoia API you will rely on (pins the exact symbols)**
```rust
// crates/btctax-store/tests/smoke.rs
use sequoia_openpgp as openpgp;
use openpgp::cert::CertBuilder;
use openpgp::serialize::stream::{Encryptor, LiteralWriter, Message};
use openpgp::parse::{Parse, stream::{DecryptorBuilder, DecryptionHelper, VerificationHelper}};
use openpgp::policy::StandardPolicy;
use std::io::Write;

#[test]
fn sequoia_encrypt_decrypt_roundtrip_with_passphrase() {
    let p = StandardPolicy::new();
    // generate a passphrase-protected cert
    let (cert, _rev) = CertBuilder::new()
        .add_userid("vault@btctax.local")
        .add_storage_encryption_subkey()
        .set_password(Some("hunter2".into()))
        .generate().unwrap();

    // encrypt
    let recipients = cert.keys().with_policy(&p, None)
        .supported().for_storage_encryption()
        .map(|ka| ka.key()).collect::<Vec<_>>();
    let mut ct = Vec::new();
    {
        let msg = Message::new(&mut ct);
        let msg = Encryptor::for_recipients(msg, recipients).build().unwrap();
        let mut w = LiteralWriter::new(msg).build().unwrap();
        w.write_all(b"hello-vault").unwrap();
        w.finalize().unwrap();
    }
    assert_ne!(ct, b"hello-vault");

    // decrypt (helper unlocks the secret key with the passphrase)
    struct H { cert: openpgp::Cert }
    impl VerificationHelper for H {
        fn get_certs(&mut self, _: &[openpgp::KeyHandle]) -> openpgp::Result<Vec<openpgp::Cert>> { Ok(vec![]) }
        fn check(&mut self, _: openpgp::parse::stream::MessageStructure) -> openpgp::Result<()> { Ok(()) }
    }
    impl DecryptionHelper for H {
        fn decrypt<D>(&mut self, pkesks: &[openpgp::packet::PKESK], _: &[openpgp::packet::SKESK],
            sym: Option<openpgp::types::SymmetricAlgorithm>, mut decrypt: D) -> openpgp::Result<Option<openpgp::Fingerprint>>
        where D: FnMut(openpgp::types::SymmetricAlgorithm, &openpgp::crypto::SessionKey) -> bool {
            let p = StandardPolicy::new();
            for ka in self.cert.keys().with_policy(&p, None).secret().for_storage_encryption() {
                let mut pair = ka.key().clone().decrypt_secret(&"hunter2".into()).unwrap().into_keypair().unwrap();
                for pkesk in pkesks {
                    if pkesk.decrypt(&mut pair, sym).map(|(a, sk)| decrypt(a, &sk)).unwrap_or(false) {
                        return Ok(Some(ka.key().fingerprint()));
                    }
                }
            }
            Ok(None)
        }
    }
    let mut pt = Vec::new();
    let mut d = DecryptorBuilder::from_bytes(&ct).unwrap()
        .with_policy(&p, None, H { cert }).unwrap();
    std::io::copy(&mut d, &mut pt).unwrap();
    assert_eq!(pt, b"hello-vault");
}
```

- [ ] **Step 5: Run the smoke test; if Sequoia symbol names differ for the pinned version, adjust to the compiler's guidance and re-pin**

Run: `cargo test -p btctax-store --test smoke -- --nocgapture`
Expected: PASS. (If `Encryptor` is named `Encryptor2` or the `DecryptionHelper::decrypt` signature differs in the resolved 1.x, fix the calls — this step exists to lock the exact API before later tasks depend on it.)

- [ ] **Step 6: Record the pinned version + backend in FOLLOWUPS.md, then commit**
```bash
git add Cargo.toml crates/btctax-store/Cargo.toml crates/btctax-store/src/lib.rs crates/btctax-store/tests/smoke.rs FOLLOWUPS.md
git commit -m "feat(store): scaffold btctax-store workspace + pin sequoia crypto backend"
```

---

### Task 1: Blob codec + migration (`[version][image]`)

**Files:**
- Create: `crates/btctax-store/src/blob.rs`
- Modify: `crates/btctax-store/src/lib.rs` (add `mod blob;`)
- Test: in `src/blob.rs` `#[cfg(test)]`

**Interfaces:**
- Produces: `fn encode_blob(version: u32, image: &[u8]) -> Vec<u8>`; `fn decode_blob(blob: &[u8]) -> Result<(u32, &[u8]), StoreError>`; `fn migrate(version: u32, image: Vec<u8>) -> Result<Vec<u8>, StoreError>`.

- [ ] **Step 1: Write the failing tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_blob() {
        let blob = encode_blob(1, b"IMG");
        let (v, img) = decode_blob(&blob).unwrap();
        assert_eq!(v, 1);
        assert_eq!(img, b"IMG");
    }
    #[test]
    fn rejects_short_blob() {
        assert!(matches!(decode_blob(b"\x00\x00"), Err(StoreError::Corrupt(_))));
    }
    #[test]
    fn migrate_identity_for_current_version() {
        let out = migrate(SCHEMA_VERSION, b"IMG".to_vec()).unwrap();
        assert_eq!(out, b"IMG");
    }
    #[test]
    fn migrate_rejects_future_version() {
        assert!(matches!(migrate(SCHEMA_VERSION + 1, vec![]), Err(StoreError::UnsupportedSchema(_))));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store blob`
Expected: FAIL (functions not defined).

- [ ] **Step 3: Implement `blob.rs`**
```rust
use crate::{StoreError, SCHEMA_VERSION};

pub fn encode_blob(version: u32, image: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + image.len());
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(image);
    out
}

pub fn decode_blob(blob: &[u8]) -> Result<(u32, &[u8]), StoreError> {
    if blob.len() < 4 {
        return Err(StoreError::Corrupt("blob shorter than 4-byte version header".into()));
    }
    let version = u32::from_be_bytes(blob[0..4].try_into().unwrap());
    Ok((version, &blob[4..]))
}

/// Migrate a decoded SQLite image from `version` to `SCHEMA_VERSION`.
/// v1 is current → identity. Future versions: add match arms that deserialize,
/// transform DDL/rows, and re-serialize (spec §8: migration spans layout+DDL+payload).
pub fn migrate(version: u32, image: Vec<u8>) -> Result<Vec<u8>, StoreError> {
    match version {
        v if v == SCHEMA_VERSION => Ok(image),
        v if v > SCHEMA_VERSION => Err(StoreError::UnsupportedSchema(v)),
        v => Err(StoreError::UnsupportedSchema(v)), // no <v1 history yet; becomes real arms later
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store blob`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/btctax-store/src/blob.rs crates/btctax-store/src/lib.rs
git commit -m "feat(store): blob codec + migration framework"
```

---

### Task 2: In-memory SQLite ⇄ bytes

**Files:**
- Create: `crates/btctax-store/src/sqlite_io.rs`
- Modify: `src/lib.rs` (`mod sqlite_io;`)
- Test: `#[cfg(test)]` in the module

**Interfaces:**
- Produces: `fn open_in_memory() -> Result<rusqlite::Connection, StoreError>`; `fn db_to_bytes(conn: &rusqlite::Connection) -> Result<Vec<u8>, StoreError>`; `fn db_from_bytes(image: &[u8]) -> Result<rusqlite::Connection, StoreError>`.

- [ ] **Step 1: Write the failing test**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn db_roundtrip_preserves_rows() {
        let c = open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (42);").unwrap();
        let bytes = db_to_bytes(&c).unwrap();
        let c2 = db_from_bytes(&bytes).unwrap();
        let x: i64 = c2.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(x, 42);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store sqlite_io`
Expected: FAIL.

- [ ] **Step 3: Implement using rusqlite's `serialize`/`deserialize`**
```rust
use rusqlite::{Connection, DatabaseName};
use crate::StoreError;

pub fn open_in_memory() -> Result<Connection, StoreError> {
    Ok(Connection::open_in_memory()?)
}

pub fn db_to_bytes(conn: &Connection) -> Result<Vec<u8>, StoreError> {
    // rusqlite "serialize" feature: returns the main DB as a contiguous image.
    let data = conn.serialize(DatabaseName::Main)?;
    Ok(data.to_vec())
}

pub fn db_from_bytes(image: &[u8]) -> Result<Connection, StoreError> {
    let conn = Connection::open_in_memory()?;
    // deserialize copies the image into a fresh in-memory db (writable).
    conn.deserialize(DatabaseName::Main, image, false)?;
    Ok(conn)
}
```
(If the resolved rusqlite exposes `serialize` as returning `Option<OwnedData>` or the `deserialize` arity differs, adjust to the compiler — the `serialize` feature is enabled in Task 0.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store sqlite_io`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/btctax-store/src/sqlite_io.rs crates/btctax-store/src/lib.rs
git commit -m "feat(store): in-memory sqlite <-> bytes"
```

---

### Task 3: Crypto — passphrase-protected keygen + encrypt/decrypt

**Files:**
- Create: `crates/btctax-store/src/crypto.rs`
- Modify: `src/lib.rs` (`mod crypto;`, add `Passphrase`)
- Test: `#[cfg(test)]` in the module

**Interfaces:**
- Consumes: the Sequoia API pinned in Task 0.
- Produces: `pub struct Passphrase`; `fn generate_cert(pp: &Passphrase) -> Result<openpgp::Cert, StoreError>`; `fn encrypt_to(cert: &openpgp::Cert, plaintext: &[u8]) -> Result<Vec<u8>, StoreError>`; `fn decrypt_with(cert: &openpgp::Cert, pp: &Passphrase, ciphertext: &[u8]) -> Result<Vec<u8>, StoreError>`.

- [ ] **Step 1: Write the failing tests** (round-trip + wrong-passphrase)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let pp = Passphrase::new("correct horse".into());
        let cert = generate_cert(&pp).unwrap();
        let ct = encrypt_to(&cert, b"secret-image").unwrap();
        assert_ne!(ct, b"secret-image");
        let pt = decrypt_with(&cert, &pp, &ct).unwrap();
        assert_eq!(pt, b"secret-image");
    }
    #[test]
    fn wrong_passphrase_fails() {
        let cert = generate_cert(&Passphrase::new("right".into())).unwrap();
        let ct = encrypt_to(&cert, b"x").unwrap();
        let err = decrypt_with(&cert, &Passphrase::new("wrong".into()), &ct).unwrap_err();
        assert!(matches!(err, StoreError::WrongPassphrase));
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store crypto`
Expected: FAIL.

- [ ] **Step 3: Implement `crypto.rs`** (lift the confirmed pattern from Task 0's smoke test; map a decrypt-secret failure to `StoreError::WrongPassphrase`)
```rust
use sequoia_openpgp as openpgp;
use openpgp::cert::CertBuilder;
use openpgp::serialize::stream::{Encryptor, LiteralWriter, Message};
use openpgp::parse::{Parse, stream::{DecryptorBuilder, DecryptionHelper, VerificationHelper, MessageStructure}};
use openpgp::policy::StandardPolicy;
use std::io::Write;
use zeroize::Zeroize;
use crate::StoreError;

pub struct Passphrase(String);
impl Passphrase { pub fn new(s: String) -> Self { Self(s) } fn as_pw(&self) -> openpgp::crypto::Password { self.0.as_str().into() } }
impl Drop for Passphrase { fn drop(&mut self) { self.0.zeroize(); } }

pub fn generate_cert(pp: &Passphrase) -> Result<openpgp::Cert, StoreError> {
    let (cert, _rev) = CertBuilder::new()
        .add_userid("vault@btctax.local")
        .add_storage_encryption_subkey()
        .set_password(Some(pp.as_pw()))
        .generate().map_err(StoreError::Crypto)?;
    Ok(cert)
}

pub fn encrypt_to(cert: &openpgp::Cert, plaintext: &[u8]) -> Result<Vec<u8>, StoreError> {
    let p = StandardPolicy::new();
    let recips = cert.keys().with_policy(&p, None).supported()
        .for_storage_encryption().map(|ka| ka.key()).collect::<Vec<_>>();
    if recips.is_empty() { return Err(StoreError::Corrupt("no encryption subkey".into())); }
    let mut ct = Vec::new();
    let msg = Message::new(&mut ct);
    let msg = Encryptor::for_recipients(msg, recips).build().map_err(StoreError::Crypto)?;
    let mut w = LiteralWriter::new(msg).build().map_err(StoreError::Crypto)?;
    w.write_all(plaintext)?;
    w.finalize().map_err(StoreError::Crypto)?;
    Ok(ct)
}

pub fn decrypt_with(cert: &openpgp::Cert, pp: &Passphrase, ciphertext: &[u8]) -> Result<Vec<u8>, StoreError> {
    struct H<'a> { cert: &'a openpgp::Cert, pw: openpgp::crypto::Password, unlocked: bool }
    impl VerificationHelper for H<'_> {
        fn get_certs(&mut self, _: &[openpgp::KeyHandle]) -> openpgp::Result<Vec<openpgp::Cert>> { Ok(vec![]) }
        fn check(&mut self, _: MessageStructure) -> openpgp::Result<()> { Ok(()) }
    }
    impl DecryptionHelper for H<'_> {
        fn decrypt<D>(&mut self, pkesks: &[openpgp::packet::PKESK], _: &[openpgp::packet::SKESK],
            sym: Option<openpgp::types::SymmetricAlgorithm>, mut decrypt: D) -> openpgp::Result<Option<openpgp::Fingerprint>>
        where D: FnMut(openpgp::types::SymmetricAlgorithm, &openpgp::crypto::SessionKey) -> bool {
            let p = StandardPolicy::new();
            for ka in self.cert.keys().with_policy(&p, None).secret().for_storage_encryption() {
                let Ok(key) = ka.key().clone().decrypt_secret(&self.pw) else { continue };
                self.unlocked = true;
                let mut pair = key.into_keypair()?;
                for pkesk in pkesks {
                    if pkesk.decrypt(&mut pair, sym).map(|(a, sk)| decrypt(a, &sk)).unwrap_or(false) {
                        return Ok(Some(ka.key().fingerprint()));
                    }
                }
            }
            Ok(None)
        }
    }
    let p = StandardPolicy::new();
    let mut helper = H { cert, pw: pp.as_pw(), unlocked: false };
    let res = DecryptorBuilder::from_bytes(ciphertext).map_err(StoreError::Crypto)?
        .with_policy(&p, None, &mut helper);
    let mut dec = match res {
        Ok(d) => d,
        Err(_) if !helper.unlocked => return Err(StoreError::WrongPassphrase),
        Err(e) => return Err(StoreError::Crypto(e)),
    };
    let mut pt = Vec::new();
    std::io::copy(&mut dec, &mut pt)?;
    Ok(pt)
}
```
Note: the `H` helper is passed by `&mut` so `unlocked` is observable after the call — that is how a wrong passphrase (no subkey unlocked) is distinguished from other crypto errors.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store crypto`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**
```bash
git add crates/btctax-store/src/crypto.rs crates/btctax-store/src/lib.rs
git commit -m "feat(store): passphrase-protected keygen + encrypt/decrypt"
```

---

### Task 4: Atomic write + `.bak` rotation + orphan `.tmp` reap

**Files:**
- Create: `crates/btctax-store/src/atomic.rs`
- Modify: `src/lib.rs` (`mod atomic;`)
- Test: `#[cfg(test)]` in the module (uses `tempfile`)

**Interfaces:**
- Produces: `fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), StoreError>`; `fn reap_tmp(target: &Path) -> Result<(), StoreError>`. Convention: temp = `target.with_extension("pgp.tmp")`, backup = `target.with_extension("pgp.bak")`.

- [ ] **Step 1: Write the failing tests** (write creates target; second write rotates `.bak`; reap deletes orphan tmp; interrupted write leaves target intact)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn write_then_overwrite_rotates_bak() {
        let dir = tempfile::tempdir().unwrap();
        let t = dir.path().join("vault.pgp");
        atomic_write(&t, b"v1").unwrap();
        assert_eq!(fs::read(&t).unwrap(), b"v1");
        atomic_write(&t, b"v2").unwrap();
        assert_eq!(fs::read(&t).unwrap(), b"v2");
        assert_eq!(fs::read(t.with_extension("pgp.bak")).unwrap(), b"v1");
    }
    #[test]
    fn interrupted_write_leaves_target_intact() {
        let dir = tempfile::tempdir().unwrap();
        let t = dir.path().join("vault.pgp");
        atomic_write(&t, b"good").unwrap();
        // simulate a crash mid-write: a stray .tmp exists, target untouched
        fs::write(t.with_extension("pgp.tmp"), b"partial").unwrap();
        assert_eq!(fs::read(&t).unwrap(), b"good");
        reap_tmp(&t).unwrap();
        assert!(!t.with_extension("pgp.tmp").exists());
        assert_eq!(fs::read(&t).unwrap(), b"good");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store atomic`
Expected: FAIL.

- [ ] **Step 3: Implement `atomic.rs`**
```rust
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use crate::StoreError;

fn tmp_of(t: &Path) -> std::path::PathBuf { t.with_extension("pgp.tmp") }
fn bak_of(t: &Path) -> std::path::PathBuf { t.with_extension("pgp.bak") }

pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = tmp_of(target);
    {
        let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;            // fsync the temp file's data
    }
    if target.exists() {
        let bak = bak_of(target);
        let _ = fs::remove_file(&bak);
        fs::rename(target, &bak)?; // keep previous good copy
    }
    fs::rename(&tmp, target)?;      // POSIX-atomic publish
    if let Some(dir) = target.parent() {
        let _ = File::open(dir).and_then(|d| d.sync_all()); // best-effort dir fsync
    }
    Ok(())
}

pub fn reap_tmp(target: &Path) -> Result<(), StoreError> {
    let tmp = tmp_of(target);
    if tmp.exists() { fs::remove_file(&tmp)?; }
    Ok(())
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store atomic`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/btctax-store/src/atomic.rs crates/btctax-store/src/lib.rs
git commit -m "feat(store): atomic write + .bak rotation + tmp reap"
```

---

### Task 5: Single-instance lock (`flock`)

**Files:**
- Create: `crates/btctax-store/src/lock.rs`
- Modify: `src/lib.rs` (`mod lock;`)
- Test: `#[cfg(test)]` in the module

**Interfaces:**
- Produces: `pub struct VaultLock(File)`; `fn acquire(vault_path: &Path) -> Result<VaultLock, StoreError>` (locks `<vault>.lock`); released on `Drop`.

- [ ] **Step 1: Write the failing test** (second acquire on the same path fails fast with `Locked`)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn second_acquire_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let v = dir.path().join("vault.pgp");
        let _first = VaultLock::acquire(&v).unwrap();
        let second = VaultLock::acquire(&v);
        assert!(matches!(second, Err(StoreError::Locked)));
        drop(_first);
        assert!(VaultLock::acquire(&v).is_ok()); // released after drop
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store lock`
Expected: FAIL.

- [ ] **Step 3: Implement `lock.rs`** (non-blocking exclusive flock via rustix)
```rust
use std::fs::{File, OpenOptions};
use std::path::Path;
use rustix::fs::{flock, FlockOperation};
use crate::StoreError;

pub struct VaultLock(File);

impl VaultLock {
    pub fn acquire(vault_path: &Path) -> Result<VaultLock, StoreError> {
        let lock_path = vault_path.with_extension("pgp.lock");
        let f = OpenOptions::new().create(true).write(true).open(&lock_path)?;
        match flock(&f, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(VaultLock(f)),
            Err(rustix::io::Errno::WOULDBLOCK) => Err(StoreError::Locked),
            Err(e) => Err(StoreError::Io(std::io::Error::from(e))),
        }
    }
}
impl Drop for VaultLock {
    fn drop(&mut self) { let _ = flock(&self.0, FlockOperation::Unlock); }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store lock`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/btctax-store/src/lock.rs crates/btctax-store/src/lib.rs
git commit -m "feat(store): single-instance flock guard"
```

---

### Task 6: Best-effort `mlock` + zeroizing buffer (R1)

**Files:**
- Create: `crates/btctax-store/src/memlock.rs`
- Modify: `src/lib.rs` (`mod memlock;`)
- Test: `#[cfg(test)]` in the module

**Interfaces:**
- Produces: `pub struct SecretBuf { bytes: Vec<u8>, locked: bool }` with `fn new(bytes: Vec<u8>) -> SecretBuf` (attempts `mlock`, sets `locked`, **warns to stderr on failure, never errors**), `fn as_slice(&self) -> &[u8]`; `Drop` zeroizes then munlocks.

- [ ] **Step 1: Write the failing test**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secretbuf_exposes_bytes_and_never_errors() {
        let s = SecretBuf::new(b"abc".to_vec());
        assert_eq!(s.as_slice(), b"abc");
        // locked is best-effort; just assert it constructed without panicking/erroring.
        let _ = s.is_locked();
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store memlock`
Expected: FAIL.

- [ ] **Step 3: Implement `memlock.rs`**
```rust
use zeroize::Zeroize;

pub struct SecretBuf { bytes: Vec<u8>, locked: bool }

impl SecretBuf {
    pub fn new(bytes: Vec<u8>) -> SecretBuf {
        let locked = Self::try_mlock(&bytes);
        if !locked {
            eprintln!("warning: mlock failed (RLIMIT_MEMLOCK?); decrypted vault may be swappable. \
                       Recommend encrypted or disabled swap.");
        }
        SecretBuf { bytes, locked }
    }
    pub fn as_slice(&self) -> &[u8] { &self.bytes }
    pub fn is_locked(&self) -> bool { self.locked }

    #[cfg(unix)]
    fn try_mlock(b: &[u8]) -> bool {
        if b.is_empty() { return true; }
        unsafe { rustix::mm::mlock(b.as_ptr() as *mut _, b.len()).is_ok() }
    }
    #[cfg(not(unix))]
    fn try_mlock(_b: &[u8]) -> bool { false }
}

impl Drop for SecretBuf {
    fn drop(&mut self) {
        self.bytes.zeroize();
        #[cfg(unix)]
        if self.locked && !self.bytes.is_empty() {
            unsafe { let _ = rustix::mm::munlock(self.bytes.as_ptr() as *mut _, self.bytes.capacity()); }
        }
    }
}
```
(Document R1 honestly in the module doc-comment: protects this buffer only — not SQLite's internal heap.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store memlock`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/btctax-store/src/memlock.rs crates/btctax-store/src/lib.rs
git commit -m "feat(store): best-effort mlock + zeroizing secret buffer"
```

---

### Task 7: `Vault` session — create / open / save / conn (integration)

**Files:**
- Create: `crates/btctax-store/src/vault.rs`
- Modify: `src/lib.rs` (`mod vault; pub use vault::Vault;`)
- Test: `crates/btctax-store/tests/integration.rs`

**Interfaces:**
- Consumes: `blob`, `sqlite_io`, `crypto`, `atomic`, `lock`, `memlock` from Tasks 1–6.
- Produces: `Vault` per the public interface at the top of this plan. The keypair (TSK) is stored **inside** the encrypted DB on `create` (table `_vault_key(armored TEXT)`), so the single `vault.pgp` is self-contained; `open` first decrypts with the passphrase using the cert it then finds — therefore the cert is also kept in a small **separate** unencrypted `vault.pub` (public cert only) so decryption can locate the secret-key packets. (Secret key material remains encrypted: the secret packets live in `vault.pgp`'s DB; `vault.pub` holds only the public cert needed to drive decryption.)

  *Implementation note:* simplest correct design — store the **full TSK armored** in a sibling `vault.key` file (the secret key is itself passphrase-encrypted by Sequoia's S2K, so it is safe at rest), and encrypt the DB to its public cert. `open` reads `vault.key`, parses the `Cert`, and calls `crypto::decrypt_with`. This keeps Task 3's API unchanged.

- [ ] **Step 1: Write the failing integration tests**
```rust
use btctax_store::{Vault, Passphrase};
#[test]
fn create_save_reopen_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let vp = dir.path().join("vault.pgp");
    {
        let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
        v.conn().execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (7);").unwrap();
        v.save().unwrap();
    }
    let v2 = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap();
    let x: i64 = v2.conn().query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
    assert_eq!(x, 7);
}
#[test]
fn open_with_wrong_passphrase_fails() {
    let dir = tempfile::tempdir().unwrap();
    let vp = dir.path().join("vault.pgp");
    Vault::create(&vp, &Passphrase::new("right".into())).unwrap().save().unwrap();
    assert!(matches!(Vault::open(&vp, &Passphrase::new("wrong".into())),
        Err(btctax_store::StoreError::WrongPassphrase)));
}
#[test]
fn second_open_is_locked() {
    let dir = tempfile::tempdir().unwrap();
    let vp = dir.path().join("vault.pgp");
    Vault::create(&vp, &Passphrase::new("pw".into())).unwrap().save().unwrap();
    let _a = Vault::open(&vp, &Passphrase::new("pw".into())).unwrap();
    assert!(matches!(Vault::open(&vp, &Passphrase::new("pw".into())),
        Err(btctax_store::StoreError::Locked)));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store --test integration`
Expected: FAIL.

- [ ] **Step 3: Implement `vault.rs`** (ties the modules together)
```rust
use std::path::{Path, PathBuf};
use rusqlite::Connection;
use sequoia_openpgp as openpgp;
use openpgp::parse::Parse;
use openpgp::serialize::Serialize;
use crate::{blob, sqlite_io, crypto::{self, Passphrase}, atomic, lock::VaultLock, memlock::SecretBuf, StoreError, SCHEMA_VERSION};

pub struct Vault {
    path: PathBuf,
    cert: openpgp::Cert,
    conn: Connection,
    _lock: VaultLock,
}

fn key_path(vault: &Path) -> PathBuf { vault.with_extension("key") }

impl Vault {
    pub fn create(vault_path: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        let lock = VaultLock::acquire(vault_path)?;
        atomic::reap_tmp(vault_path)?;
        let cert = crypto::generate_cert(pp)?;
        // persist the (passphrase-encrypted) TSK alongside the vault
        let mut armored = Vec::new();
        cert.as_tsk().serialize(&mut armored).map_err(StoreError::Crypto)?;
        atomic::atomic_write(&key_path(vault_path), &armored)?;
        let conn = sqlite_io::open_in_memory()?;
        let mut v = Vault { path: vault_path.to_path_buf(), cert, conn, _lock: lock };
        v.save()?;
        Ok(v)
    }

    pub fn open(vault_path: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        let lock = VaultLock::acquire(vault_path)?;
        atomic::reap_tmp(vault_path)?;
        let armored = std::fs::read(key_path(vault_path))?;
        let cert = openpgp::Cert::from_bytes(&armored).map_err(StoreError::Crypto)?;
        let ct = std::fs::read(vault_path)?;
        let plaintext = SecretBuf::new(crypto::decrypt_with(&cert, pp, &ct)?);
        let (version, image) = blob::decode_blob(plaintext.as_slice())?;
        let image = blob::migrate(version, image.to_vec())?;
        let image = SecretBuf::new(image);
        let conn = sqlite_io::db_from_bytes(image.as_slice())?;
        Ok(Vault { path: vault_path.to_path_buf(), cert, conn, _lock: lock })
    }

    pub fn conn(&self) -> &Connection { &self.conn }

    pub fn save(&mut self) -> Result<(), StoreError> {
        let image = sqlite_io::db_to_bytes(&self.conn)?;
        let blob = blob::encode_blob(SCHEMA_VERSION, &image);
        let ct = crypto::encrypt_to(&self.cert, &blob)?;
        atomic::atomic_write(&self.path, &ct)?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store --test integration`
Expected: PASS (all three).

- [ ] **Step 5: Run the full crate suite + lint + fmt**

Run: `cargo test -p btctax-store && cargo clippy -p btctax-store -- -D warnings && cargo fmt --check`
Expected: all green.

- [ ] **Step 6: Commit**
```bash
git add crates/btctax-store/src/vault.rs crates/btctax-store/src/lib.rs crates/btctax-store/tests/integration.rs
git commit -m "feat(store): Vault session (create/open/save) wiring all primitives"
```

---

### Task 8: `export_snapshot` + `backup_key` (recovery escape hatches, NFR2 exception)

**Files:**
- Modify: `crates/btctax-store/src/vault.rs` (add two methods)
- Test: `crates/btctax-store/tests/integration.rs` (extend)

**Interfaces:**
- Produces: `fn export_snapshot(&self, out_dir: &Path) -> Result<PathBuf, StoreError>` (writes `out_dir/snapshot.sqlite`, the decrypted DB image); `fn backup_key(&self, out_path: &Path) -> Result<(), StoreError>` (writes the armored TSK).

- [ ] **Step 1: Write the failing tests**
```rust
#[test]
fn export_snapshot_is_a_readable_sqlite_db() {
    let dir = tempfile::tempdir().unwrap();
    let vp = dir.path().join("vault.pgp");
    let mut v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
    v.conn().execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (9);").unwrap();
    v.save().unwrap();
    let snap = v.export_snapshot(dir.path()).unwrap();
    let c = rusqlite::Connection::open(&snap).unwrap();
    let x: i64 = c.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
    assert_eq!(x, 9);
}
#[test]
fn backup_key_writes_a_parseable_cert() {
    let dir = tempfile::tempdir().unwrap();
    let vp = dir.path().join("vault.pgp");
    let v = Vault::create(&vp, &Passphrase::new("pw".into())).unwrap();
    let kp = dir.path().join("backup.key");
    v.backup_key(&kp).unwrap();
    assert!(sequoia_openpgp::Cert::from_bytes(&std::fs::read(&kp).unwrap()).is_ok());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p btctax-store --test integration export_snapshot backup_key`
Expected: FAIL.

- [ ] **Step 3: Implement the two methods on `Vault`**
```rust
impl Vault {
    pub fn export_snapshot(&self, out_dir: &std::path::Path) -> Result<std::path::PathBuf, StoreError> {
        let image = sqlite_io::db_to_bytes(&self.conn)?;
        let out = out_dir.join("snapshot.sqlite");
        // write the raw SQLite image (a valid standalone db file) directly
        std::fs::write(&out, &image)?;
        Ok(out)
    }
    pub fn backup_key(&self, out_path: &std::path::Path) -> Result<(), StoreError> {
        let mut armored = Vec::new();
        openpgp::serialize::Serialize::serialize(&self.cert.as_tsk(), &mut armored).map_err(StoreError::Crypto)?;
        atomic::atomic_write(out_path, &armored)?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p btctax-store --test integration`
Expected: PASS.

- [ ] **Step 5: Full gate**

Run: `cargo test -p btctax-store && cargo clippy -p btctax-store -- -D warnings && cargo fmt --check`
Expected: all green.

- [ ] **Step 6: Commit**
```bash
git add crates/btctax-store/src/vault.rs crates/btctax-store/tests/integration.rs
git commit -m "feat(store): export-snapshot + backup-key recovery hatches"
```

---

## Self-Review (run against the spec §8 + §16 step 1)

**Spec coverage (§8 / §16 step 1):** atomic write + `.bak` (Task 4) ✓; `flock` (Task 5) ✓; encrypt/decrypt round-trip + wrong-passphrase (Tasks 3, 7) ✓; `mlock`+warn (Task 6) ✓; `schema_version` + `migrate` (Task 1) ✓; in-memory SQLite (Task 2, 7) ✓; key lifecycle/backup + `export-snapshot` (Tasks 7, 8) ✓; orphan `.tmp` reaped on open (Task 4 `reap_tmp`, called in Task 7 `open`/`create`) ✓; no-plaintext-except-export (Task 8 only) ✓. **Deferred to a later store iteration (logged in FOLLOWUPS):** strong-S2K selection on the TSK (Task 0 uses Sequoia's default `set_password`; confirm/raise the S2K to Argon2 if the pinned version supports it — spec §8/R3); crash-injection test that kills mid-`save` at the OS level (Task 4 simulates via a stray `.tmp`; a process-kill harness is a hardening follow-up).

**Placeholder scan:** none — every code step is concrete.

**Type consistency:** `Passphrase` (Task 3) used unchanged in Tasks 7–8; `StoreError` variants (Task 0) referenced consistently; `atomic_write`/`reap_tmp`/`VaultLock::acquire`/`SecretBuf::new`/`encode_blob`/`decode_blob`/`migrate`/`db_to_bytes`/`db_from_bytes`/`generate_cert`/`encrypt_to`/`decrypt_with` signatures match their call sites in `vault.rs`.

## Notes for Plans 2–4 (foundation remainder)
- **Plan 2 — `btctax-core`:** domain types + `source_ref`/`EventId`/`decision_seq`/`LotId` + canonical order + two-pass projection (spec §6–§7), consuming `Vault::conn()` for persistence. Pure logic; heaviest test surface (property + determinism + KATs).
- **Plan 3 — `btctax-adapters`:** Coinbase/Gemini/River/Swan parsers + `PriceProvider` dataset (spec §9), each with real-fixture tests.
- **Plan 4 — reconciliation + `btctax-cli`** (spec §10–§12): `import`/`reconcile`/`verify`/`reconstruct-2025`/`allocate-2025`/`export-snapshot`/`backup-key`, golden end-to-end.
Each follows the same spec→plan→implement→review-to-green cycle.
