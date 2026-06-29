# btctax-store (Encrypted Vault) Implementation Plan — Foundation Plan 1 of 4 (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Revision:** v2 folds the round-1 plan review (`reviews/plan-foundation-01-store-round-1.md`): rusqlite `OwnedData`/`sqlite3_malloc64` FFI (C1), crash-safe copy-before-rename atomic write + `.bak` recovery (C2), `Encryptor2` (I1), shared-flag wrong-passphrase detection (I2), strongest-S2K pinned at keygen (I3), `create()` no-clobber guard (I4), append-not-replace temp/bak names (I5), and minors.

**Goal:** Build `btctax-store`, the PGP-encrypted local vault that persists an opaque `[schema_version][SQLite image]` blob and exposes a live in-memory SQLite handle — durable (crash-safe atomic write), single-instance-safe (flock), and key/passphrase-protected — with no dependency on the domain model.

**Architecture:** The on-disk artifacts are `vault.pgp` (Sequoia-OpenPGP, encrypted to an app-managed keypair) and `vault.key` (the same keypair's secret material, itself passphrase-encrypted by a strong OpenPGP S2K). At runtime the blob is decrypted into RAM, loaded into an in-memory SQLite database, operated on, then serialized + re-encrypted + atomically rewritten. No plaintext DB is ever written except the explicit `export-snapshot`.

**Tech Stack:** Rust (edition 2021), `sequoia-openpgp` 1.21 (OpenPGP, `Encryptor2`), `rusqlite` 0.31 (bundled SQLite, `serialize` feature, `rusqlite::ffi` for `sqlite3_malloc64`), `rustix` (flock + mlock), `zeroize`, `anyhow`, `thiserror`.

## Global Constraints
(Spec `design/SPEC_foundation.md`; every task implicitly includes these.)
- **NFR2:** only `vault.pgp`/`vault.key` written automatically; **no plaintext DB ever** except the explicit `export-snapshot`. (`vault.key` holds only S2K-encrypted secret material — see Design Note.)
- **NFR3:** no save may corrupt/lose the vault — **copy `target`→`.bak` (fsync) before a single atomic `rename(tmp→target)`**; `target` is never absent; `open()` recovers from `.bak`.
- **NFR7:** concurrent instances must not silently clobber — `flock(LOCK_EX|LOCK_NB)`, fail fast.
- **R1 (best-effort, honest):** `mlock`/`zeroize` are defense-in-depth (this buffer only; not SQLite's heap); warn, don't fail, if `mlock` is unavailable.
- **R3:** pin exact `sequoia-openpgp` version, crypto backend, **and S2K** in Task 0 (the spike), before crypto code.
- **§8 blob layout:** `[schema_version: u32 big-endian][SQLite serialized image bytes]`.
- **Licensing:** workspace `license = "MIT OR Unlicense"`.
- **Validation gate:** `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` all green; plus 0 Critical/0 Important on review.

## Design Note (folds review M1/M2/M8 — single source of truth, no dead alternatives)
The vault is **two co-located files**: `vault.pgp` (the encrypted DB blob) and `vault.key` (the full keypair/TSK, whose secret packets are passphrase-encrypted with a strong S2K). This is a deliberate, documented deviation from §8's "one `vault.pgp`" wording (logged in FOLLOWUPS): `open` needs the public cert to drive decryption and the (encrypted) secret to unlock it; `vault.key` provides both and stays safe at rest because its secret is S2K-encrypted. *Authenticity:* the blob is encrypted but **not signed** (a holder of the public cert could forge a decryptable blob); accepted for a local single-user app and logged in FOLLOWUPS (sign-on-save is a future option). There is no `vault.pub`/TSK-in-DB scheme.

## File Structure
```
Cargo.toml                      # [workspace] root
crates/btctax-store/
  Cargo.toml                    # pinned deps + crypto backend
  src/lib.rs                    # pub API + StoreError + SCHEMA_VERSION; wires modules
  src/blob.rs                   # encode/decode [version][image]; migrate(version,bytes)
  src/sqlite_io.rs              # in-memory SQLite <-> bytes (serialize/deserialize via OwnedData)
  src/crypto.rs                 # Sequoia: gen_key(strong S2K), encrypt (Encryptor2), decrypt (shared-flag)
  src/paths.rs                  # tmp/bak/lock/key path derivation (APPEND, not with_extension)
  src/atomic.rs                 # crash-safe atomic_write (copy-bak->rename), reap_tmp, recover
  src/lock.rs                   # VaultLock (flock LOCK_EX|LOCK_NB)
  src/memlock.rs                # best-effort mlock + zeroizing buffer
  src/vault.rs                  # Vault: create/open/save/conn/export_snapshot/backup_key
  tests/smoke.rs                # Task 0 spike: pins Sequoia + rusqlite APIs
  tests/integration.rs          # end-to-end: create/open/save, wrong-pass, lock, crash recovery
```
**Public interface this plan PRODUCES (consumed by Plan 2 `btctax-core`):**
- `pub const SCHEMA_VERSION: u32`
- `pub enum StoreError` (`thiserror`): `Io`, `Crypto`, `Locked`, `WrongPassphrase`, `Corrupt`, `Sqlite`, `UnsupportedSchema(u32)`, `AlreadyExists`.
- `pub struct Passphrase` (zeroizing).
- `pub struct Vault`:
  - `fn create(vault_path: &Path, passphrase: &Passphrase) -> Result<Vault, StoreError>` — **errors `AlreadyExists`** if `vault.pgp` or `vault.key` exists.
  - `fn open(vault_path: &Path, passphrase: &Passphrase) -> Result<Vault, StoreError>` — flock, recover-from-bak-if-needed, decrypt, mlock, deserialize.
  - `fn conn(&self) -> &rusqlite::Connection`
  - `fn save(&mut self) -> Result<(), StoreError>`
  - `fn export_snapshot(&self, out_dir: &Path) -> Result<PathBuf, StoreError>` (returns the written path)
  - `fn backup_key(&self, out_path: &Path) -> Result<(), StoreError>` (ASCII-armored TSK)
  - `Drop`: releases the flock and drops the decrypted `SecretBuf`s (zeroized). *(SQLite's internal heap is not zeroized — R1.)*

---

### Task 0: Workspace scaffold + de-risking spike (pins Sequoia + rusqlite + S2K)

**Files:** Create `Cargo.toml` (workspace), `crates/btctax-store/Cargo.toml`, `crates/btctax-store/src/lib.rs`, `crates/btctax-store/tests/smoke.rs`.

**Interfaces:** Produces a compiling workspace and the pinned `sequoia-openpgp` version, crypto backend, and **confirmed S2K** (recorded in FOLLOWUPS per R3).

- [ ] **Step 1: Workspace root `Cargo.toml`**
```toml
[workspace]
resolver = "2"
members = ["crates/btctax-store"]
[workspace.package]
edition = "2021"
license = "MIT OR Unlicense"
rust-version = "1.74"
```

- [ ] **Step 2: `crates/btctax-store/Cargo.toml`** (backend = `crypto-nettle`, the mature default; revisit `crypto-rust` only if a fully-static binary becomes mandatory — log the choice in FOLLOWUPS)
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
zeroize = "1"
anyhow = "1"
thiserror = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Stub `src/lib.rs`** (error enum + version; modules added in later tasks)
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
    #[error("vault already exists at this path")] AlreadyExists,
}
```

- [ ] **Step 4: Spike `tests/smoke.rs` — pins ALL three risky APIs (Sequoia Encryptor2, shared-flag decrypt, rusqlite serialize/deserialize) and inspects the S2K**
```rust
use sequoia_openpgp as openpgp;
use openpgp::cert::CertBuilder;
use openpgp::serialize::stream::{Encryptor2, LiteralWriter, Message};
use openpgp::parse::{Parse, stream::{DecryptorBuilder, DecryptionHelper, VerificationHelper, MessageStructure}};
use openpgp::policy::StandardPolicy;
use std::io::Write;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

#[test]
fn sequoia_roundtrip_with_shared_unlock_flag_and_strong_s2k() {
    let p = StandardPolicy::new();
    let (cert, _rev) = CertBuilder::new()
        .add_userid("vault@btctax.local")
        .add_storage_encryption_subkey()
        .set_password(Some("hunter2".into()))
        .generate().unwrap();

    // R3: inspect the S2K actually applied to the secret key. Record the variant in FOLLOWUPS.
    // If it is not Argon2, Task 3 must re-protect the secret with Argon2id explicitly.
    for ka in cert.keys().secret() {
        eprintln!("secret-key S2K = {:?}", ka.key().secret()); // inspect; assert Argon2 if available
    }

    // encrypt (Encryptor2)
    let recips = cert.keys().with_policy(&p, None).supported()
        .for_storage_encryption().map(|ka| ka.key()).collect::<Vec<_>>();
    let mut ct = Vec::new();
    {
        let m = Message::new(&mut ct);
        let m = Encryptor2::for_recipients(m, recips).build().unwrap();
        let mut w = LiteralWriter::new(m).build().unwrap();
        w.write_all(b"hello").unwrap();
        w.finalize().unwrap();
    }

    // decrypt with a SHARED unlocked-flag (observable on Ok or Err) — the wrong-passphrase mechanism
    struct H { cert: openpgp::Cert, pw: openpgp::crypto::Password, unlocked: Arc<AtomicBool> }
    impl VerificationHelper for H {
        fn get_certs(&mut self, _: &[openpgp::KeyHandle]) -> openpgp::Result<Vec<openpgp::Cert>> { Ok(vec![]) }
        fn check(&mut self, _: MessageStructure) -> openpgp::Result<()> { Ok(()) }
    }
    impl DecryptionHelper for H {
        fn decrypt<D>(&mut self, pkesks: &[openpgp::packet::PKESK], _: &[openpgp::packet::SKESK],
            sym: Option<openpgp::types::SymmetricAlgorithm>, mut decrypt: D) -> openpgp::Result<Option<openpgp::Fingerprint>>
        where D: FnMut(openpgp::types::SymmetricAlgorithm, &openpgp::crypto::SessionKey) -> bool {
            let p = StandardPolicy::new();
            for ka in self.cert.keys().with_policy(&p, None).secret().for_storage_encryption() {
                let Ok(key) = ka.key().clone().decrypt_secret(&self.pw) else { continue };
                self.unlocked.store(true, Ordering::SeqCst);
                let mut pair = key.into_keypair()?;
                for pk in pkesks {
                    if pk.decrypt(&mut pair, sym).map(|(a, sk)| decrypt(a, &sk)).unwrap_or(false) {
                        return Ok(Some(ka.key().fingerprint()));
                    }
                }
            }
            Ok(None)
        }
    }
    let unlocked = Arc::new(AtomicBool::new(false));
    let h = H { cert: cert.clone(), pw: "hunter2".into(), unlocked: unlocked.clone() };
    let mut d = DecryptorBuilder::from_bytes(&ct).unwrap().with_policy(&p, None, h).unwrap();
    let mut pt = Vec::new();
    std::io::copy(&mut d, &mut pt).unwrap();
    assert_eq!(pt, b"hello");
    assert!(unlocked.load(Ordering::SeqCst));
}

#[test]
fn rusqlite_serialize_deserialize_roundtrip_via_owneddata() {
    use rusqlite::{Connection, DatabaseName};
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(42);").unwrap();
    let data = c.serialize(DatabaseName::Main).unwrap();      // rusqlite::serialize::Data (Deref<[u8]>)
    let image: Vec<u8> = data.to_vec();                        // copy out of Shared/Owned
    // rebuild via OwnedData allocated by sqlite3_malloc64
    let owned = unsafe {
        let n = image.len();
        let p = rusqlite::ffi::sqlite3_malloc64(n as u64) as *mut u8;
        assert!(!p.is_null());
        std::ptr::copy_nonoverlapping(image.as_ptr(), p, n);
        rusqlite::serialize::OwnedData::from_raw_nonnull(std::ptr::NonNull::new(p).unwrap(), n)
    };
    let mut c2 = Connection::open_in_memory().unwrap();
    c2.deserialize(DatabaseName::Main, owned, false).unwrap();
    let x: i64 = c2.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
    assert_eq!(x, 42);
}
```

- [ ] **Step 5: Run the spike; pin exact symbols/S2K to the resolved versions**

Run: `cargo test -p btctax-store --test smoke -- --nocapture`
Expected: both PASS. If a symbol/arity differs (e.g., `Data::to_vec`, `OwnedData::from_raw_nonnull` signature, `decrypt_secret` name), fix to the compiler and **record the confirmed S2K variant** from the eprintln in FOLLOWUPS (R3). If the default S2K is not Argon2id, note that Task 3 must set it explicitly.

- [ ] **Step 6: Commit**
```bash
git add Cargo.toml crates/btctax-store/Cargo.toml crates/btctax-store/src/lib.rs crates/btctax-store/tests/smoke.rs FOLLOWUPS.md
git commit -m "feat(store): scaffold + spike pinning sequoia(Encryptor2)+rusqlite(OwnedData)+S2K"
```

---

### Task 1: Blob codec + migration (`[version][image]`)
*(unchanged from review-approved v1)*

**Files:** Create `src/blob.rs`; Modify `src/lib.rs` (`mod blob;`). Test in-module.

**Interfaces:** Produces `encode_blob(u32,&[u8])->Vec<u8>`, `decode_blob(&[u8])->Result<(u32,&[u8]),StoreError>`, `migrate(u32,Vec<u8>)->Result<Vec<u8>,StoreError>`.

- [ ] **Step 1: Failing tests**
```rust
#[cfg(test)] mod tests { use super::*;
  #[test] fn roundtrip(){ let b=encode_blob(1,b"IMG"); let (v,i)=decode_blob(&b).unwrap(); assert_eq!(v,1); assert_eq!(i,b"IMG"); }
  #[test] fn rejects_short(){ assert!(matches!(decode_blob(b"\x00\x00"), Err(StoreError::Corrupt(_)))); }
  #[test] fn migrate_identity(){ assert_eq!(migrate(SCHEMA_VERSION,b"IMG".to_vec()).unwrap(), b"IMG"); }
  #[test] fn migrate_future(){ assert!(matches!(migrate(SCHEMA_VERSION+1,vec![]), Err(StoreError::UnsupportedSchema(_)))); }
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store blob`
- [ ] **Step 3: Implement**
```rust
use crate::{StoreError, SCHEMA_VERSION};
pub fn encode_blob(version: u32, image: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(4 + image.len());
    o.extend_from_slice(&version.to_be_bytes()); o.extend_from_slice(image); o
}
pub fn decode_blob(blob: &[u8]) -> Result<(u32, &[u8]), StoreError> {
    if blob.len() < 4 { return Err(StoreError::Corrupt("blob < 4-byte header".into())); }
    Ok((u32::from_be_bytes(blob[0..4].try_into().unwrap()), &blob[4..]))
}
pub fn migrate(version: u32, image: Vec<u8>) -> Result<Vec<u8>, StoreError> {
    if version == SCHEMA_VERSION { Ok(image) } else { Err(StoreError::UnsupportedSchema(version)) }
}
```
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store blob`
- [ ] **Step 5: Commit.** `git commit -am "feat(store): blob codec + migration"`

---

### Task 2: In-memory SQLite ⇄ bytes (real rusqlite 0.31 API — folds C1)

**Files:** Create `src/sqlite_io.rs`; Modify `src/lib.rs` (`mod sqlite_io;`). Test in-module.

**Interfaces:** Produces `open_in_memory()->Result<Connection,StoreError>`, `db_to_bytes(&Connection)->Result<Vec<u8>,StoreError>`, `db_from_bytes(&[u8])->Result<Connection,StoreError>`.

- [ ] **Step 1: Failing test**
```rust
#[cfg(test)] mod tests { use super::*;
  #[test] fn db_roundtrip(){
    let c=open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(42);").unwrap();
    let b=db_to_bytes(&c).unwrap();
    let c2=db_from_bytes(&b).unwrap();
    let x:i64=c2.query_row("SELECT x FROM t",[],|r|r.get(0)).unwrap();
    assert_eq!(x,42);
  }
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store sqlite_io`
- [ ] **Step 3: Implement (uses the OwnedData/sqlite3_malloc64 path proven in the Task 0 spike)**
```rust
use rusqlite::{Connection, DatabaseName};
use rusqlite::serialize::OwnedData;
use crate::StoreError;

pub fn open_in_memory() -> Result<Connection, StoreError> { Ok(Connection::open_in_memory()?) }

pub fn db_to_bytes(conn: &Connection) -> Result<Vec<u8>, StoreError> {
    let data = conn.serialize(DatabaseName::Main)?; // Data: Deref<Target=[u8]>
    Ok(data.to_vec())
}

pub fn db_from_bytes(image: &[u8]) -> Result<Connection, StoreError> {
    let mut conn = Connection::open_in_memory()?;
    // SQLite owns deserialized memory; it must be allocated by sqlite3_malloc64.
    let owned = unsafe {
        let n = image.len();
        let p = rusqlite::ffi::sqlite3_malloc64(n as u64) as *mut u8;
        if p.is_null() { return Err(StoreError::Corrupt("sqlite3_malloc64 failed".into())); }
        std::ptr::copy_nonoverlapping(image.as_ptr(), p, n);
        OwnedData::from_raw_nonnull(std::ptr::NonNull::new(p).unwrap(), n)
    };
    conn.deserialize(DatabaseName::Main, owned, false)?;
    Ok(conn)
}
```
(If the resolved rusqlite exposes `Data::to_vec` differently or `OwnedData::from_raw_nonnull` has a different arity, adjust per the Task 0 spike — both paths are proven there before this task.)
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store sqlite_io`
- [ ] **Step 5: Commit.** `git commit -am "feat(store): in-memory sqlite <-> bytes via OwnedData"`

---

### Task 3: Crypto — strong-S2K keygen + Encryptor2 + shared-flag decrypt (folds I1/I2/I3)

**Files:** Create `src/crypto.rs`; Modify `src/lib.rs` (`mod crypto; pub use crypto::Passphrase;`). Test in-module.

**Interfaces:** Produces `Passphrase`; `generate_cert(&Passphrase)->Result<Cert,StoreError>`; `encrypt_to(&Cert,&[u8])->Result<Vec<u8>,StoreError>`; `decrypt_with(&Cert,&Passphrase,&[u8])->Result<Vec<u8>,StoreError>`.

- [ ] **Step 1: Failing tests (round-trip + wrong-passphrase → `WrongPassphrase`)**
```rust
#[cfg(test)] mod tests { use super::*;
  #[test] fn roundtrip(){
    let pp=Passphrase::new("correct horse".into());
    let c=generate_cert(&pp).unwrap();
    let ct=encrypt_to(&c,b"img").unwrap(); assert_ne!(ct,b"img");
    assert_eq!(decrypt_with(&c,&pp,&ct).unwrap(), b"img");
  }
  #[test] fn wrong_pass(){
    let c=generate_cert(&Passphrase::new("right".into())).unwrap();
    let ct=encrypt_to(&c,b"x").unwrap();
    assert!(matches!(decrypt_with(&c,&Passphrase::new("wrong".into()),&ct), Err(StoreError::WrongPassphrase)));
  }
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store crypto`
- [ ] **Step 3: Implement** (Encryptor2; shared `Arc<AtomicBool>` unlock flag; if the Task-0 spike showed the default S2K is not Argon2id, set it explicitly here per the recorded R3 finding)
```rust
use sequoia_openpgp as openpgp;
use openpgp::cert::CertBuilder;
use openpgp::serialize::stream::{Encryptor2, LiteralWriter, Message};
use openpgp::parse::{Parse, stream::{DecryptorBuilder, DecryptionHelper, VerificationHelper, MessageStructure}};
use openpgp::policy::StandardPolicy;
use std::io::Write;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use zeroize::Zeroize;
use crate::StoreError;

pub struct Passphrase(String);
impl Passphrase { pub fn new(s: String) -> Self { Self(s) } fn pw(&self) -> openpgp::crypto::Password { self.0.as_str().into() } }
impl Drop for Passphrase { fn drop(&mut self){ self.0.zeroize(); } }

pub fn generate_cert(pp: &Passphrase) -> Result<openpgp::Cert, StoreError> {
    // NOTE: if Task-0 confirmed set_password's default S2K is weaker than Argon2id,
    // re-protect each secret key here via Key::encrypt_secret with an Argon2id S2K.
    let (cert, _rev) = CertBuilder::new()
        .add_userid("vault@btctax.local")
        .add_storage_encryption_subkey()
        .set_password(Some(pp.pw()))
        .generate().map_err(StoreError::Crypto)?;
    Ok(cert)
}

pub fn encrypt_to(cert: &openpgp::Cert, plaintext: &[u8]) -> Result<Vec<u8>, StoreError> {
    let p = StandardPolicy::new();
    let recips = cert.keys().with_policy(&p, None).supported()
        .for_storage_encryption().map(|ka| ka.key()).collect::<Vec<_>>();
    if recips.is_empty() { return Err(StoreError::Corrupt("no encryption subkey".into())); }
    let mut ct = Vec::new();
    let m = Message::new(&mut ct);
    let m = Encryptor2::for_recipients(m, recips).build().map_err(StoreError::Crypto)?;
    let mut w = LiteralWriter::new(m).build().map_err(StoreError::Crypto)?;
    w.write_all(plaintext)?;
    w.finalize().map_err(StoreError::Crypto)?;
    Ok(ct)
}

pub fn decrypt_with(cert: &openpgp::Cert, pp: &Passphrase, ct: &[u8]) -> Result<Vec<u8>, StoreError> {
    struct H { cert: openpgp::Cert, pw: openpgp::crypto::Password, unlocked: Arc<AtomicBool> }
    impl VerificationHelper for H {
        fn get_certs(&mut self, _: &[openpgp::KeyHandle]) -> openpgp::Result<Vec<openpgp::Cert>> { Ok(vec![]) }
        fn check(&mut self, _: MessageStructure) -> openpgp::Result<()> { Ok(()) }
    }
    impl DecryptionHelper for H {
        fn decrypt<D>(&mut self, pkesks: &[openpgp::packet::PKESK], _: &[openpgp::packet::SKESK],
            sym: Option<openpgp::types::SymmetricAlgorithm>, mut decrypt: D) -> openpgp::Result<Option<openpgp::Fingerprint>>
        where D: FnMut(openpgp::types::SymmetricAlgorithm, &openpgp::crypto::SessionKey) -> bool {
            let p = StandardPolicy::new();
            for ka in self.cert.keys().with_policy(&p, None).secret().for_storage_encryption() {
                let Ok(key) = ka.key().clone().decrypt_secret(&self.pw) else { continue };
                self.unlocked.store(true, Ordering::SeqCst);
                let mut pair = key.into_keypair()?;
                for pk in pkesks {
                    if pk.decrypt(&mut pair, sym).map(|(a, sk)| decrypt(a, &sk)).unwrap_or(false) {
                        return Ok(Some(ka.key().fingerprint()));
                    }
                }
            }
            Ok(None)
        }
    }
    let p = StandardPolicy::new();
    let unlocked = Arc::new(AtomicBool::new(false));
    let h = H { cert: cert.clone(), pw: pp.pw(), unlocked: unlocked.clone() };
    let res = DecryptorBuilder::from_bytes(ct).map_err(StoreError::Crypto)?.with_policy(&p, None, h);
    let mut dec = match res {
        Ok(d) => d,
        Err(e) => return Err(if unlocked.load(Ordering::SeqCst) { StoreError::Crypto(e) } else { StoreError::WrongPassphrase }),
    };
    let mut pt = Vec::new();
    std::io::copy(&mut dec, &mut pt)?;
    Ok(pt)
}
```
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store crypto`
- [ ] **Step 5: Commit.** `git commit -am "feat(store): strong-S2K keygen + Encryptor2 + shared-flag decrypt"`

---

### Task 4: Path helpers + crash-safe atomic write (folds C2 + I5)

**Files:** Create `src/paths.rs`, `src/atomic.rs`; Modify `src/lib.rs`. Test in-module (`tempfile`).

**Interfaces:** Produces `paths::{tmp_of, bak_of}` (APPEND `.tmp`/`.bak` to the full filename); `atomic::atomic_write(&Path,&[u8])->Result<(),StoreError>`; `atomic::reap_tmp(&Path)->Result<(),StoreError>`; `atomic::recover_target(&Path)->Result<(),StoreError>` (restores from `.bak` if target missing).

- [ ] **Step 1: Failing tests (rotation; crash windows; name-append; recovery)**
```rust
#[cfg(test)] mod tests { use super::*; use std::fs;
  #[test] fn names_append_not_replace(){
    let p = std::path::Path::new("/x/vault.key");
    assert_eq!(crate::paths::tmp_of(p).file_name().unwrap(), "vault.key.tmp");
    assert_eq!(crate::paths::bak_of(p).file_name().unwrap(), "vault.key.bak");
  }
  #[test] fn write_keeps_prev_in_bak_and_target_never_absent(){
    let d=tempfile::tempdir().unwrap(); let t=d.path().join("vault.pgp");
    atomic_write(&t,b"v1").unwrap(); assert_eq!(fs::read(&t).unwrap(),b"v1");
    atomic_write(&t,b"v2").unwrap(); assert_eq!(fs::read(&t).unwrap(),b"v2");
    assert_eq!(fs::read(crate::paths::bak_of(&t)).unwrap(),b"v1");
  }
  #[test] fn recover_from_bak_when_target_missing(){
    let d=tempfile::tempdir().unwrap(); let t=d.path().join("vault.pgp");
    fs::write(crate::paths::bak_of(&t), b"good").unwrap();   // only the bak survived
    recover_target(&t).unwrap();
    assert_eq!(fs::read(&t).unwrap(), b"good");
  }
  #[test] fn reap_tmp_only_when_target_present(){
    let d=tempfile::tempdir().unwrap(); let t=d.path().join("vault.pgp");
    atomic_write(&t,b"good").unwrap();
    fs::write(crate::paths::tmp_of(&t), b"partial").unwrap();
    reap_tmp(&t).unwrap();
    assert!(!crate::paths::tmp_of(&t).exists());
    assert_eq!(fs::read(&t).unwrap(), b"good");
  }
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store atomic`
- [ ] **Step 3: Implement `paths.rs`** (append to full filename — fixes I5)
```rust
use std::path::{Path, PathBuf};
fn suffixed(p: &Path, suffix: &str) -> PathBuf {
    let mut name = p.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    p.with_file_name(name)
}
pub fn tmp_of(p: &Path) -> PathBuf { suffixed(p, ".tmp") }
pub fn bak_of(p: &Path) -> PathBuf { suffixed(p, ".bak") }
pub fn lock_of(p: &Path) -> PathBuf { suffixed(p, ".lock") }
```
- [ ] **Step 4: Implement `atomic.rs`** (copy-bak BEFORE a single atomic rename — fixes C2; target is never absent)
```rust
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use crate::{paths, StoreError};

pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = paths::tmp_of(target);
    { let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(&tmp)?;
      f.write_all(bytes)?; f.sync_all()?; }
    if target.exists() {
        // keep a fsync'd backup BEFORE we touch the live file
        let bak = paths::bak_of(target);
        fs::copy(target, &bak)?;
        File::open(&bak)?.sync_all()?;
    }
    fs::rename(&tmp, target)?; // atomic replace; target is never absent
    if let Some(dir) = target.parent() { let _ = File::open(dir).and_then(|d| d.sync_all()); }
    Ok(())
}

/// If the target is missing but a backup exists (e.g., a crash on the very first
/// create before any rename completed), restore it.
pub fn recover_target(target: &Path) -> Result<(), StoreError> {
    if !target.exists() {
        let bak = paths::bak_of(target);
        if bak.exists() { fs::copy(&bak, target)?; }
    }
    Ok(())
}

/// Remove a stray temp file — only safe once the target is present.
pub fn reap_tmp(target: &Path) -> Result<(), StoreError> {
    let tmp = paths::tmp_of(target);
    if tmp.exists() && target.exists() { fs::remove_file(&tmp)?; }
    Ok(())
}
```
- [ ] **Step 5: Run → PASS.** `cargo test -p btctax-store atomic paths`
- [ ] **Step 6: Commit.** `git commit -am "feat(store): crash-safe atomic write (copy-bak->rename) + recovery + safe paths"`

---

### Task 5: Single-instance lock (`flock`)
*(unchanged from review-approved v1, using `paths::lock_of`)*

**Files:** Create `src/lock.rs`; Modify `src/lib.rs`. Test in-module.
**Interfaces:** Produces `VaultLock`; `VaultLock::acquire(&Path)->Result<VaultLock,StoreError>` (locks `paths::lock_of(vault)`); released on Drop.

- [ ] **Step 1: Failing test**
```rust
#[cfg(test)] mod tests { use super::*;
  #[test] fn second_acquire_refused(){
    let d=tempfile::tempdir().unwrap(); let v=d.path().join("vault.pgp");
    let a=VaultLock::acquire(&v).unwrap();
    assert!(matches!(VaultLock::acquire(&v), Err(StoreError::Locked)));
    drop(a); assert!(VaultLock::acquire(&v).is_ok());
  }
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store lock`
- [ ] **Step 3: Implement**
```rust
use std::fs::{File, OpenOptions};
use std::path::Path;
use rustix::fs::{flock, FlockOperation};
use crate::{paths, StoreError};
pub struct VaultLock(File);
impl VaultLock {
    pub fn acquire(vault: &Path) -> Result<VaultLock, StoreError> {
        let f = OpenOptions::new().create(true).write(true).open(paths::lock_of(vault))?;
        match flock(&f, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(VaultLock(f)),
            Err(rustix::io::Errno::WOULDBLOCK) => Err(StoreError::Locked),
            Err(e) => Err(StoreError::Io(std::io::Error::from(e))),
        }
    }
}
impl Drop for VaultLock { fn drop(&mut self){ let _=flock(&self.0, FlockOperation::Unlock); } }
```
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store lock`
- [ ] **Step 5: Commit.** `git commit -am "feat(store): single-instance flock guard"`

---

### Task 6: Best-effort `mlock` + zeroizing buffer (R1)
*(unchanged from review-approved v1; consistent length M4)*

**Files:** Create `src/memlock.rs`; Modify `src/lib.rs`. Test in-module.
**Interfaces:** Produces `SecretBuf` with `new(Vec<u8>)`, `as_slice()->&[u8]`, `is_locked()->bool`; Drop zeroizes then munlocks.

- [ ] **Step 1: Failing test**
```rust
#[cfg(test)] mod tests { use super::*;
  #[test] fn exposes_bytes_never_errors(){ let s=SecretBuf::new(b"abc".to_vec()); assert_eq!(s.as_slice(),b"abc"); let _=s.is_locked(); }
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store memlock`
- [ ] **Step 3: Implement** (use `len` consistently for mlock and munlock — M4)
```rust
use zeroize::Zeroize;
pub struct SecretBuf { bytes: Vec<u8>, locked: bool }
impl SecretBuf {
    pub fn new(bytes: Vec<u8>) -> SecretBuf {
        let locked = Self::try_mlock(&bytes);
        if !locked { eprintln!("warning: mlock failed (RLIMIT_MEMLOCK?); decrypted vault may be swappable — use encrypted/disabled swap."); }
        SecretBuf { bytes, locked }
    }
    pub fn as_slice(&self) -> &[u8] { &self.bytes }
    pub fn is_locked(&self) -> bool { self.locked }
    #[cfg(unix)] fn try_mlock(b: &[u8]) -> bool { if b.is_empty() { return true; } unsafe { rustix::mm::mlock(b.as_ptr() as *mut _, b.len()).is_ok() } }
    #[cfg(not(unix))] fn try_mlock(_b: &[u8]) -> bool { false }
}
impl Drop for SecretBuf {
    fn drop(&mut self) {
        let len = self.bytes.len();
        self.bytes.zeroize();
        #[cfg(unix)] if self.locked && len > 0 { unsafe { let _=rustix::mm::munlock(self.bytes.as_ptr() as *mut _, len); } }
    }
}
```
(Doc-comment: protects this buffer only — not SQLite's internal heap; R1.)
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store memlock`
- [ ] **Step 5: Commit.** `git commit -am "feat(store): best-effort mlock + zeroizing buffer"`

---

### Task 7: `Vault` session — create(no-clobber) / open(recover) / save / conn (folds I4 + C2 recovery)

**Files:** Create `src/vault.rs`; Modify `src/lib.rs` (`mod vault; pub use vault::Vault;`). Test `tests/integration.rs`.

**Interfaces:** Consumes Tasks 1–6. Produces `Vault` per the public interface above.

- [ ] **Step 1: Failing integration tests**
```rust
use btctax_store::{Vault, Passphrase, StoreError};
#[test] fn create_save_reopen(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    { let mut v=Vault::create(&vp,&Passphrase::new("pw".into())).unwrap();
      v.conn().execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(7);").unwrap(); v.save().unwrap(); }
    let v2=Vault::open(&vp,&Passphrase::new("pw".into())).unwrap();
    assert_eq!(v2.conn().query_row("SELECT x FROM t",[],|r|r.get::<_,i64>(0)).unwrap(), 7);
}
#[test] fn wrong_passphrase(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    Vault::create(&vp,&Passphrase::new("right".into())).unwrap().save().unwrap();
    assert!(matches!(Vault::open(&vp,&Passphrase::new("wrong".into())), Err(StoreError::WrongPassphrase)));
}
#[test] fn second_open_locked(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    Vault::create(&vp,&Passphrase::new("pw".into())).unwrap().save().unwrap();
    let _a=Vault::open(&vp,&Passphrase::new("pw".into())).unwrap();
    assert!(matches!(Vault::open(&vp,&Passphrase::new("pw".into())), Err(StoreError::Locked)));
}
#[test] fn create_refuses_existing(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    Vault::create(&vp,&Passphrase::new("pw".into())).unwrap().save().unwrap();
    assert!(matches!(Vault::create(&vp,&Passphrase::new("pw".into())), Err(StoreError::AlreadyExists)));
}
#[test] fn open_recovers_from_bak_if_target_missing(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    { let mut v=Vault::create(&vp,&Passphrase::new("pw".into())).unwrap();
      v.conn().execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(5);").unwrap(); v.save().unwrap(); v.save().unwrap(); }
    // simulate a crash that left only the .bak (newest committed copy is in target; older in .bak):
    std::fs::copy(&vp, btctax_store::testing::bak_of(&vp)).unwrap();
    std::fs::remove_file(&vp).unwrap();
    let v=Vault::open(&vp,&Passphrase::new("pw".into())).unwrap();
    assert_eq!(v.conn().query_row("SELECT x FROM t",[],|r|r.get::<_,i64>(0)).unwrap(), 5);
}
```
(Expose `pub mod testing { pub use crate::paths::bak_of; }` behind `#[cfg(any(test, feature="testing"))]` so the integration test can name the bak path.)
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store --test integration`
- [ ] **Step 3: Implement `vault.rs`**
```rust
use std::path::{Path, PathBuf};
use rusqlite::Connection;
use sequoia_openpgp as openpgp;
use openpgp::parse::Parse;
use openpgp::serialize::Serialize;
use crate::{blob, sqlite_io, crypto::{self, Passphrase}, atomic, paths, lock::VaultLock, memlock::SecretBuf, StoreError, SCHEMA_VERSION};

pub struct Vault { path: PathBuf, cert: openpgp::Cert, conn: Connection, _lock: VaultLock }
fn key_path(v: &Path) -> PathBuf { paths::suffixed_key(v) } // = v with ".key" appended-by-stem; see note

impl Vault {
    pub fn create(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        let kp = key_path(vault);
        if vault.exists() || kp.exists() { return Err(StoreError::AlreadyExists); }
        let lock = VaultLock::acquire(vault)?;
        let cert = crypto::generate_cert(pp)?;
        let mut tsk = Vec::new();
        cert.as_tsk().serialize(&mut tsk).map_err(StoreError::Crypto)?;
        atomic::atomic_write(&kp, &tsk)?;
        let conn = sqlite_io::open_in_memory()?;
        let mut v = Vault { path: vault.to_path_buf(), cert, conn, _lock: lock };
        v.save()?;
        Ok(v)
    }
    pub fn open(vault: &Path, pp: &Passphrase) -> Result<Vault, StoreError> {
        let lock = VaultLock::acquire(vault)?;
        atomic::recover_target(vault)?;   // restore from .bak if a crash left target missing
        atomic::reap_tmp(vault)?;
        let cert = openpgp::Cert::from_bytes(&std::fs::read(key_path(vault))?).map_err(StoreError::Crypto)?;
        let plaintext = SecretBuf::new(crypto::decrypt_with(&cert, pp, &std::fs::read(vault)?)?);
        let (ver, image) = blob::decode_blob(plaintext.as_slice())?;
        let image = SecretBuf::new(blob::migrate(ver, image.to_vec())?);
        let conn = sqlite_io::db_from_bytes(image.as_slice())?;
        Ok(Vault { path: vault.to_path_buf(), cert, conn, _lock: lock })
    }
    pub fn conn(&self) -> &Connection { &self.conn }
    pub fn save(&mut self) -> Result<(), StoreError> {
        let image = sqlite_io::db_to_bytes(&self.conn)?;
        let ct = crypto::encrypt_to(&self.cert, &blob::encode_blob(SCHEMA_VERSION, &image))?;
        atomic::atomic_write(&self.path, &ct)
    }
}
```
*Note for `key_path`:* add `paths::suffixed_key(v: &Path) -> PathBuf` that yields `<dir>/<stem-of-vault>.key` — i.e., for `vault.pgp` → `vault.key`. Implement it in `paths.rs` (Task 4) using `with_extension("key")` (safe here because `.key` is a single distinct extension, not the `.pgp.tmp` collision class). Add a unit test `assert_eq!(suffixed_key(Path::new("/x/vault.pgp")).file_name().unwrap(), "vault.key")`.
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store --test integration`
- [ ] **Step 5: Full gate.** `cargo test -p btctax-store && cargo clippy --all-targets -p btctax-store -- -D warnings && cargo fmt --check`
- [ ] **Step 6: Commit.** `git commit -am "feat(store): Vault session (no-clobber create, bak-recovering open, save)"`

---

### Task 8: `export_snapshot` + `backup_key` (NFR2 exception; armored key — folds M3/M6)

**Files:** Modify `src/vault.rs`. Test `tests/integration.rs` (extend).

**Interfaces:** Produces `export_snapshot(&self,&Path)->Result<PathBuf,StoreError>`; `backup_key(&self,&Path)->Result<(),StoreError>` (ASCII-armored TSK).

- [ ] **Step 1: Failing tests**
```rust
#[test] fn export_snapshot_is_readable_sqlite(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    let mut v=Vault::create(&vp,&Passphrase::new("pw".into())).unwrap();
    v.conn().execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(9);").unwrap(); v.save().unwrap();
    let snap=v.export_snapshot(d.path()).unwrap();
    let c=rusqlite::Connection::open(&snap).unwrap();
    assert_eq!(c.query_row("SELECT x FROM t",[],|r|r.get::<_,i64>(0)).unwrap(), 9);
}
#[test] fn backup_key_is_armored_and_parseable(){
    let d=tempfile::tempdir().unwrap(); let vp=d.path().join("vault.pgp");
    let v=Vault::create(&vp,&Passphrase::new("pw".into())).unwrap();
    let kp=d.path().join("backup.asc"); v.backup_key(&kp).unwrap();
    let bytes=std::fs::read(&kp).unwrap();
    assert!(bytes.starts_with(b"-----BEGIN PGP")); // armored
    assert!(sequoia_openpgp::Cert::from_bytes(&bytes).is_ok());
}
```
- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-store --test integration export_snapshot backup_key`
- [ ] **Step 3: Implement** (armored writer for the key — M6)
```rust
use openpgp::serialize::SerializeInto;
impl Vault {
    pub fn export_snapshot(&self, out_dir: &Path) -> Result<PathBuf, StoreError> {
        let image = sqlite_io::db_to_bytes(&self.conn)?;
        let out = out_dir.join("snapshot.sqlite");
        std::fs::write(&out, &image)?; // raw SQLite image = a valid standalone db file
        Ok(out)
    }
    pub fn backup_key(&self, out_path: &Path) -> Result<(), StoreError> {
        let armored = self.cert.as_tsk().armored().to_vec().map_err(StoreError::Crypto)?;
        atomic::atomic_write(out_path, &armored)
    }
}
```
(If `as_tsk().armored().to_vec()` differs in 1.21, use the `armor::Writer`; the spike confirms the available serializer.)
- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-store --test integration`
- [ ] **Step 5: Full gate.** `cargo test -p btctax-store && cargo clippy --all-targets -p btctax-store -- -D warnings && cargo fmt --check`
- [ ] **Step 6: Commit.** `git commit -am "feat(store): export-snapshot + armored backup-key"`

---

## Self-Review (v2, against spec §8/§16 step 1 + round-1 plan review)
**Round-1 findings folded:** C1 (rusqlite OwnedData/sqlite3_malloc64 + spike round-trip) → Task 0 Step 4, Task 2; C2 (copy-bak→rename + recover + safe reap + recovery test) → Task 4, Task 7; I1 (Encryptor2) → Tasks 0/3; I2 (shared-flag decrypt) → Tasks 0/3; I3 (S2K inspected/pinned at keygen) → Task 0/3 + FOLLOWUPS; I4 (no-clobber create) → Task 7; I5 (append-not-replace paths) → Task 4 `paths.rs`; M1 (dead vault.pub text removed) → Design Note; M2/M8 (two-artifact + unsigned ack) → Design Note + FOLLOWUPS; M3 (export_snapshot → PathBuf consistent) → interface + Task 8; M4 (mlock/munlock same len) → Task 6; M5 (Drop wording) → interface; M6 (armored key) → Task 8; M7 (anyhow dep, no unused zeroize feature) → Task 0.
**Placeholder scan:** none. **Type consistency:** `Passphrase`, `StoreError` (now incl. `AlreadyExists`), `paths::{tmp_of,bak_of,lock_of,suffixed_key}`, `atomic::{atomic_write,reap_tmp,recover_target}`, `crypto::{generate_cert,encrypt_to,decrypt_with}`, `sqlite_io::{db_to_bytes,db_from_bytes}`, `SecretBuf` — all match their `vault.rs` call sites.
**Still deferred (FOLLOWUPS, non-blocking):** an OS-level process-kill-mid-save fuzz harness (Task 4 tests the three on-disk crash states deterministically; a kill harness is added hardening); sign-on-save (M8); re-lock-on-timeout (N3, a CLI/session concern).

## Notes for Plans 2–4
- **Plan 2 `btctax-core`:** domain + two-pass projection (spec §6–§7), persisting via `Vault::conn()`.
- **Plan 3 `btctax-adapters`:** the 4 parsers + `PriceProvider` (spec §9).
- **Plan 4 reconciliation + `btctax-cli`** (spec §10–§12) + golden end-to-end.
