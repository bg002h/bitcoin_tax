# Review — IMPLEMENTATION_PLAN_foundation_01_store.md (v2), Round 2

- **Reviewer:** independent senior Rust reviewer, round 2; API claims verified against rusqlite 0.31.0 source + sequoia-openpgp 1.21.0 docs.
- **Date:** 2026-06-28
- **Verdict:** 0 Critical, **2 Important** (+5 Minor / 3 Nit). Both round-1 Criticals (C1 rusqlite FFI, C2 atomic write) correctly resolved & API-verified. Not yet 0/0.
- Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

## Round-1 fix verification
C1 RESOLVED & sound (OwnedData `from_raw_nonnull` unsafe + Drop→sqlite3_free; `deserialize` does `into_raw()`+`mem::forget`+`FREEONCLOSE` → no double-free/leak; spike round-trips rusqlite). C2 RESOLVED & sound (copy target→bak+fsync BEFORE single atomic `rename(tmp→target)`; target never absent for a committed vault; `recover_target` only acts when target absent; `reap_tmp` only when target present). I1 Encryptor2 (confirmed not deprecated in 1.21). I2 shared `Arc<AtomicBool>` — correct, no false pos/neg (decrypt_secret is cryptographically verified). I4 create→AlreadyExists (minor TOCTOU). I5 append paths RESOLVED. M1/M2/M3/M4/M6/M7/M8 RESOLVED. M5 partial (still inaccurate).

## CRITICAL — None.

## IMPORTANT
### Important-1 — I3 (strong S2K) NOT resolved; remediation targets a non-existent API
Verified: sequoia 1.21 `S2K::default()` = `Iterated{SHA256}` — **no Argon2 variant exists in 1.21**. `Key::encrypt_secret(self, &Password) -> Result<Self>` takes **no S2K argument**. So the plan's Task-3 note ("re-protect via encrypt_secret with an Argon2id S2K") is not implementable. The Task-0 spike only `eprintln!`s `ka.key().secret()` (Debug of `SecretKeyMaterial` renders `Encrypted(..)` and does not cleanly expose the S2K) — it neither asserts nor pins. Net: every vault would use the default Iterated-SHA256 S2K, contradicting §8 "strongest available" / R3 "pin before crypto code," with no working remediation. **Fix:** the spec's own fallback is "else high-work-factor iterated-salted" — adopt it honestly: in Task 0 extract the real S2K (`match SecretKeyMaterial::Encrypted => .s2k()`), assert it is `S2K::Iterated`, record the variant+count in FOLLOWUPS (R3), and document Argon2 is unavailable in 1.21 (revisit on upgrade). If a public high-work-factor setter exists, use it; otherwise pin the library default as the strongest available via the supported API.

### Important-2 — `testing` module unreachable from `tests/integration.rs`; recovery test won't compile under the gate
`pub mod testing` is behind `#[cfg(any(test, feature="testing"))]`. Integration tests link the lib WITHOUT `--cfg test`, and no `testing` feature is declared (and the gate runs no `--features testing`) → `btctax_store::testing` is absent → `open_recovers_from_bak_if_target_missing` fails `E0433` → `cargo test` gate can't pass. Worse, the undefined `feature="testing"` cfg trips `unexpected_cfgs` → hard error under `cargo clippy --all-targets -- -D warnings`. **Fix:** make `paths` a `pub mod` (the helpers are harmless) and call `btctax_store::paths::bak_of` from the test, or compute the `.bak` path inline. Reflect in the gate command.

## MINOR
- **Minor-1** First-create crash wedges the store: `create()` writes `vault.key` (no prior → no `.bak`) then `save()`s `vault.pgp`; a crash during that first rename leaves target absent with no `.bak` (recover_target no-op) AND `vault.key` present (→ `create` returns `AlreadyExists`). Neither create nor open works until manual cleanup. No committed data lost. The `recover_target` comment describes exactly the case it cannot handle. **Fix:** clean partial artifacts on failed `create` (or detect/repair in open/create).
- **Minor-2** `vault.key` is never recovered/reaped (open only handles the `vault.pgp` target). Orphan `vault.key.tmp` never cleaned. Latent; inconsistent with the crash-safety story.
- **Minor-3** `paths::suffixed_key` appears only as prose under Task 7 but the self-review lists it as consistent; fold the function + unit test into Task 4. Edge: a vault literally named `*.key` → suffixed_key collides; guard it.
- **Minor-4** M5 Drop wording still inaccurate — `Vault` holds no `SecretBuf` fields (they're locals in `open()`); reword to "releases the flock; transient decrypt buffer zeroized in open; SQLite heap not zeroized (R1)."
- **Minor-5** `db_from_bytes` maps any `sqlite3_malloc64` NULL (incl. OOM) to `Corrupt`; OOM isn't corruption. Minor mislabel.

## NIT
- **Nit-1** TOCTOU in `create()`: existence check before `flock`; two concurrent creates → loser gets `Locked` not `AlreadyExists`. No corruption. (Acquire lock first.)
- **Nit-2** `backup_key` via `atomic_write` leaves a stray `<out>.bak` if the chosen path exists; `export_snapshot` no parent-dir check.
- **Nit-3** Spike S2K "inspection" yields nothing useful via Debug; must match `SecretKeyMaterial::Encrypted` and read its `S2K` to record per R3.

## Verified-correct (NEW-problem hunts)
sqlite3_malloc64/OwnedData ownership sound (forget+FREEONCLOSE); copy→rename has no remaining window for a committed vault; `recover_target` never resurrects stale bak over good target; `suffixed_key` collision-free vs append families except `*.key`-named vault; shared flag no false pos/neg; `as_tsk().armored().to_vec()` real in 1.21.

## Verdict
0 Critical, 2 Important (I3 S2K unresolved/non-implementable remediation; `testing` module unreachable → gate won't pass). Both Criticals correctly resolved. Fix the two Importants (and ideally Minor-1/Minor-3), re-review per §2 before implementation.

### Sources
rusqlite 0.31.0 `src/serialize.rs` (serialize→Data; deserialize(&mut,..,OwnedData,bool); OwnedData::from_raw_nonnull unsafe + Drop→sqlite3_free; into_raw/mem::forget/FREEONCLOSE). sequoia 1.21.0: Encryptor2 (not deprecated); S2K::default=Iterated{SHA256}, no Argon2; encrypt_secret(self,&Password) no S2K param; TSK::armored()→impl SerializeInto; PKESK::decrypt; decrypt_secret/into_keypair.
