# Review — IMPLEMENTATION_PLAN_foundation_01_store.md (v3), Round 3

- **Reviewer:** independent senior Rust reviewer, round 3. Verified against **extracted sources** of sequoia-openpgp 1.21.0 + rusqlite 0.31.0, plus an **empirical `cargo build` + `cargo clippy --all-targets -- -D warnings`** of the `create()` closure/IIFE pattern.
- **Date:** 2026-06-28
- **Verdict:** **0 Critical, 0 Important** (3 Minor, 3 Nit). v3 is sound to implement — the gate is cleared.
- Persisted per STANDARD_WORKFLOW §2.

## Round-2 findings — resolved?
- **Important-1 (strong S2K) — RESOLVED, honest AND correct.** Confirmed: no Argon2 in 1.21 (`S2K` `#[non_exhaustive]`, only `Iterated` non-deprecated). Spike now extracts (`SecretKeyMaterial::Encrypted(e)` → `Encrypted::s2k()`, both confirmed in `src/packet/key.rs`) and asserts `S2K::Iterated`; the `other => panic!` arm is required (non_exhaustive). The Argon2-via-`encrypt_secret` remediation is gone (`encrypt_secret(self,&Password)` takes no S2K param). Traced the actual value: `set_password`→`encrypt_in_place`→`Unencrypted::encrypt` uses `S2K::default()` = `Iterated{SHA256, hash_bytes: 0x3e00000}` — *the largest count OpenPGP can represent* (~354 ms, AES256). So the recorded S2K is the **maximum-work-factor** iterated-salted S2K — fully satisfies §8 "else high-work-factor iterated-salted" / R3.
- **Important-2 (`paths` public / clippy) — RESOLVED.** `pub mod paths;`; integration test uses `btctax_store::paths::bak_of`; `testing` module + undefined `feature="testing"` cfg gone → `unexpected_cfgs` can't fire; compiles clean.
- **Minors 1–5 / Nits 1–2:** M2 RESOLVED (key recover/reap), M3-fold suffixed_key RESOLVED, M4 Drop wording accurate, M5 OOM→Io RESOLVED, Nit-1 lock-first RESOLVED, Nit-2 backup_key plain write + export mkdir RESOLVED (1.21 docstring literally shows `as_tsk().armored().to_vec()` → `-----BEGIN PGP PRIVATE KEY BLOCK-----`). Minor-1 partially (in-process only — see M2 below).

## New-problem hunt (empirically verified)
The two flagged compile concerns are **unfounded**: the `create()` cleanup-closure + `?`-IIFE + `[&kp, &tmp_of(&kp), &vault.to_path_buf(), &tmp_of(vault)]` array + `suffixed_key` assert **compiles and passes `clippy --all-targets -- -D warnings` (exit 0)**. Both closures take shared borrows (no conflict); array elements uniformly `&PathBuf`; `clippy::redundant_closure_call` does NOT fire on the `?`-returning IIFE; `assert_ne!(PathBuf, &Path)` compiles (std `PartialEq<&Path> for PathBuf`); `open()`'s loop is a clean `[&Path;2]`. Also verified: `DecryptionHelper::decrypt<D>` 1.21 signature matches the plan exactly; `TSK: Serialize`; rusqlite `OwnedData::from_raw_nonnull`/`deserialize(&mut,..,OwnedData,bool)`/`serialize->Data: Deref<[u8]>`/`Drop→sqlite3_free`/`pub use libsqlite3_sys as ffi` all confirmed.

## MINOR (non-blocking)
- **M1** `suffixed_key` uses `assert_ne!` → **panics** on a `*.key` vault path, reachable from both `create()` and `open()` (which otherwise return `Result`). A library shouldn't abort on bad input (§12 typed-errors posture). Low reachability (CLI uses fixed `vault.pgp`). **Fix:** return a typed error (new variant or `Io(InvalidInput)`) and propagate.
- **M2** Minor-1 only closed for the in-process path: a real OS crash between the `vault.key` write and the first `vault.pgp` rename leaves key present + pgp/bak absent → `create`→`AlreadyExists`, `open`→`Io(NotFound)`; manual key deletion needed (no committed data lost). Acknowledge / treat "key present, pgp+bak absent" as a half-created vault to repair; otherwise FOLLOWUPS kill-harness.
- **M3** `create()`/`open()` require the parent dir to pre-exist (lock-first opens `vault.pgp.lock` → `Io(NotFound)` if missing); inconsistent with `export_snapshot`'s mkdir. **Fix:** `create` should `create_dir_all(parent)` up front (or Plan 4 `init` owns dir creation). open() requiring an existing dir is fine.

## NIT
- **N1** A failed/`AlreadyExists` `create` leaves a `.lock` file (lock-first; cleanup not reached on early return). Harmless, conventional; acknowledge.
- **N2** Spike's `if let Encrypted(e)` has no `else`; an `Unencrypted` key would be skipped. With `set_password` all keys are encrypted; optional rigor: assert none is `Unencrypted`.
- **N3** Plan snippets aren't rustfmt-formatted (illustrative; `cargo fmt --check` enforces real code).

## Plan quality
TDD real/consistent across Tasks 1–8; Task 0 correctly a spike. No placeholders. Type/signature consistency holds end-to-end; `.map_err(StoreError::Crypto)` valid (`openpgp::Result` = `anyhow::Result`); three `#[from]` source types distinct (no conflicting `From`).

## Verdict
**v3 is at 0 Critical / 0 Important — sound to implement.** S2K lands on the maximum-work-factor iterated S2K (exceeds "honest minimum"). All load-bearing 1.21/0.31 symbols verified against source; the create() pattern compiles + passes clippy empirically. 3 Minors + 3 Nits non-blocking — fold during implementation or log to FOLLOWUPS (re-review the fold per §2).
