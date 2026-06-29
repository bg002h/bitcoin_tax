# Review — IMPLEMENTATION_PLAN_foundation_01_store.md (Round 1)

- **Artifact:** `design/IMPLEMENTATION_PLAN_foundation_01_store.md` (Plan 1 of 4, btctax-store)
- **Reviewer:** independent senior Rust reviewer, fresh context; verified Sequoia 1.21 + rusqlite 0.31 APIs against upstream docs/source.
- **Date:** 2026-06-28
- **Verdict:** NOT sound to implement as written — **2 Critical + 5 Important** + 8 Minor / 4 Nit. Architecture/decomposition/TDD cadence good; crypto+persistence API and atomic-write crash-safety need folding before implementation.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

---

## Critical

### C1 — rusqlite persistence (Task 2) does not match the pinned API; won't build; unspike'd
Plan's load path `conn.deserialize(DatabaseName::Main, image: &[u8], false)`. In **rusqlite 0.31.0** the real signatures are:
```
pub fn serialize(&self, schema: DatabaseName) -> Result<Data>     // Data = enum { Shared(SharedData<'conn>), Owned(OwnedData) }
pub fn deserialize(&mut self, schema: DatabaseName, data: OwnedData, read_only: bool) -> Result<()>
```
`deserialize` takes **`OwnedData`, not `&[u8]`**; `OwnedData`'s only constructor is `unsafe fn from_raw_nonnull(ptr, sz)` requiring a buffer allocated by **`sqlite3_malloc64`**. No safe `from_vec`. So `db_from_bytes` needs an undisclosed unsafe FFI dance (sqlite3_malloc64 → copy_nonoverlapping → from_raw_nonnull), pulling in `libsqlite3-sys`/`rusqlite::ffi`. Also: `serialize` returns `Data` (not Vec/slice) so `.to_vec()` is not guaranteed; `deserialize` needs `&mut self` while code binds `let conn`. **Task 0 spike exercises only Sequoia, not rusqlite serialize/deserialize** — the harder API. NFR2 blocks the easy fallback (write temp .sqlite + open → plaintext on disk). **Fix:** add a rusqlite serialize→deserialize round-trip to the Task 0 spike and implement/prove the OwnedData-from-decrypted-bytes path before Task 7 depends on it.

### C2 — Atomic write has an absent-target crash window; `reap_tmp` then destroys the newest copy (NFR3 unmet)
`atomic_write`: write+fsync tmp → remove bak → **rename(target→bak)** → rename(tmp→target). Between the two renames **`vault.pgp` does not exist**. Crash there → target missing, bak = previous good, tmp = new fsync'd. Next startup: `open()` does `read(vault_path)?` → fails (no `.bak` fallback anywhere); `reap_tmp()` unconditionally deletes `vault.pgp.tmp` — discarding the newest copy. A crash during save can render a committed vault unopenable and then delete the recovery copy. Violates §8/NFR3. Integration test only simulates a stray `.tmp` with target intact (the safe window). **`.bak` should be a copy, not a rename-away of the only good copy.** **Fix:** copy target→bak (+fsync) *before* a single `rename(tmp→target)` (atomic replace; target never absent); teach `open()` to recover from `.bak`; never reap a tmp while target is absent; add an OS-level kill-mid-save test.

---

## Important

### I1 — `Encryptor` deprecated in Sequoia 1.21 → fails `clippy -- -D warnings`
docs.rs 1.21.0 marks `serialize::stream::Encryptor` deprecated ("Use Encryptor2 instead"). The §0 gate runs `cargo clippy -- -D warnings` → deprecation becomes an error. Tasks 0 and 3 use `Encryptor`; the spike runs only `cargo test` so it passes and the gate fails later. **Fix:** use `Encryptor2` (drop-in `for_recipients`) everywhere.

### I2 — Wrong-passphrase via `with_policy(&mut helper)` then reading `helper.unlocked` won't compile; unspike'd
`DecryptorBuilder::with_policy<T,H>(self, policy, time, helper: H) -> Result<Decryptor<H>>` takes the helper **by value**. Passing `&mut helper` requires `&mut H: DecryptionHelper` (not provided); idiomatic recovery is `Decryptor::into_helper(self)` (unusable on the `Err` path). The wrong-passphrase mechanism (dedicated §8 requirement + test) is broken as written; the Task 0 smoke passes the helper by value and never reads post-state. **Fix:** keep helper by value, put `unlocked` behind a shared handle (`Rc<Cell<bool>>`/`Arc<AtomicBool>`) cloned into it, observable on Ok or Err; spike that exact shape.

### I3 — S2K neither pinned nor strengthened (contradicts R3/§8); un-revisitable create-time decision
R3: pin S2K (Argon2 if available) before first build; §8: "strongest available S2K." Plan uses bare `set_password` (library default) and defers S2K to FOLLOWUPS. S2K is baked into the secret key **at generation** — cannot be upgraded later without regenerating. "Raise it later" leaves every created vault on the default permanently. **Fix:** set the strongest S2K explicitly in Task 3 (Argon2id if 1.21 supports it for CertBuilder/encrypt_secret), or at minimum verify+assert the 1.21 default is Argon2id in the spike and record per R3.

### I4 — `Vault::create` silently clobbers an existing vault
`create()` unconditionally writes `vault.key` and `save()`s (rotating an existing live vault into .bak then overwriting). No "refuse if exists" guard, no test. Re-running CLI `init` on an existing path destroys the ledger. Guard belongs in the store. **Fix:** `create()` errors if `vault_path` or key file exists; add a test.

### I5 — `tmp_of`/`bak_of` use `with_extension`, so `vault.key` collides with the vault's temp/bak namespace
`with_extension` *replaces*: `vault.key`.with_extension("pgp.tmp") == `vault.pgp.tmp`; bak == `vault.pgp.bak` — same paths the vault uses. Latent today (key written once), but a future `vault.key` rewrite while `vault.pgp.bak` exists would delete the vault's real backup and replace it with the key file. `backup_key` reuses the scheme. **Fix:** derive temp/bak by *appending* to the full filename (`vault.pgp`→`vault.pgp.tmp`; `vault.key`→`vault.key.tmp`), not `with_extension`.

---

## Minor
- **M1** Task 7 "Interfaces" describes a dead `vault.pub`+TSK-in-DB scheme contradicting the actual sibling-`vault.key` code; delete the dead text.
- **M2** §8 says "one `vault.pgp`"; design adds `vault.key` (S2K-encrypted, so NFR2 holds) — deviation from single-artifact wording; co-located, not an off-host backup. Ack explicitly.
- **M3** `export_snapshot` return type: top interface says `Result<()>`, Task 8 implements `Result<PathBuf>`. Pick one.
- **M4** mlock uses `b.len()`, munlock uses `capacity()` — page-granular so harmless; use one length.
- **M5** Vault has no explicit `Drop`; the "Drop → zeroize/munlock" claim is overstated (SecretBufs are locals dropped in `open()`; live data is in SQLite's heap, never zeroized). Consistent with R1 but reword.
- **M6** `cert.as_tsk().serialize()` writes binary, not ASCII armor despite the `armored` name; use the armor writer if armor intended.
- **M7** Task 0 Cargo.toml omits `anyhow` (used by StoreError); enables `zeroize` feature `zeroize_derive` though `#[derive(Zeroize)]` is unused.
- **M8** Vault is encrypted but not signed; anyone with the public cert can craft a vault.pgp that decrypts (no authenticity). Low risk single-user; document the decision or sign-on-save.

## Nits
- **N1** lock file / orphan partial-create artifacts never cleaned (harmless).
- **N2** `migrate`'s `v < SCHEMA_VERSION` arm collapses to `UnsupportedSchema` (fine at v1).
- **N3** §8 "re-lock on timeout" not implemented (only on-exit Drop); session/CLI concern; note it.
- **N4** `export_snapshot` uses plain `fs::write` with no parent-dir check.

## Spec-faithfulness (§8/§16 step 1)
atomic write+bak — present but **unsafe crash window (C2)**; flock — ✓; encrypt/decrypt+wrong-pass — encrypt OK (**Encryptor I1**), wrong-pass **won't compile (I2)**; mlock+warn — ✓ (M4/M5); schema_version+migrate — ✓; in-memory SQLite — **API mismatch (C1)**; key backup/export-snapshot — present (M3/M6); orphan .tmp reap — **reaps recovery copy in C2 window**; no-plaintext-except-export — ✓ but new artifact (M2); strongest-S2K pinned up front — **deferred, violates R3 (I3)**; init forces key-backup — not in store (CLI/Plan 4). Plan-quality: no placeholders, TDD concrete; but the self-review's signature-consistency claim is overstated (M3 + C1 signatures don't hold).

## Highest-leverage fixes
1. **Re-aim Task 0 spike + fix rusqlite reality (C1):** round-trip serialize/deserialize in the spike; implement OwnedData-from-decrypted-bytes (sqlite3_malloc64) before Task 7.
2. **Crash-safe + recoverable atomic write (C2):** copy target→bak (fsync) before a single atomic rename(tmp→target); open() recovers from .bak; don't reap tmp when target absent; kill-mid-save test.
3. **Spike the two crypto unknowns + decide S2K up front (I1/I2/I3):** Encryptor2; wrong-pass via shared flag; pin strongest S2K (Argon2id if available) at keygen.

### Sources
rusqlite 0.31.0 serialize.rs (`serialize -> Data`, `deserialize(&mut, .., OwnedData, bool)`, `OwnedData::from_raw_nonnull` sqlite3_malloc64); Sequoia 1.21.0 `Encryptor` deprecated→`Encryptor2`; `DecryptorBuilder::with_policy(.., helper: H)` by value + `Decryptor::into_helper`; flock(2) cross-OFD semantics.
