# Whole-Branch Review — `btctax-store` (05b7ad8..4bdc16b, 17 commits) — Round 1

- **Reviewer:** independent final whole-branch reviewer (most-capable model), full crate + cross-module wiring vs spec §8 / NFR2/NFR3/NFR7/NFR8/R1/R3 + plan. Tests controller-verified green; not re-run.
- **Date:** 2026-06-28
- **Verdict:** **Not ready to merge — 0 Critical, 1 Important (I-1).** Persisted per STANDARD_WORKFLOW §2.

## Headline
Crate is well-built — FFI memory-safe, POSIX crash-safe write sound, wrong-passphrase oracle correct. Whole-branch lens surfaces ONE Important the per-task reviews missed: owner-only hardening was applied to `backup_key`/`export_snapshot` but NOT to the primary key write, so the always-present `vault.key` (passphrase-protected private key) is written world-readable.

## Critical — None.
- No path persists plaintext DB or decrypted private key; `save()` always encrypts; `vault.key` is S2K-encrypted TSK (`Iterated{SHA256,0x3E00000}`); only plaintext sink is `export_snapshot` (NFR2 exception).
- `sqlite_io` FFI sound (no UB/double-free/leak). Wrong-pass vs crypto-failure correct, no false success. No committed vault destroyable/unopenable on POSIX.

## Important
### I-1 — Primary `vault.key`/`vault.pgp` written world-readable; owner-only hardening applied only to backup/export copies (MERGE-BLOCKING)
`atomic::atomic_write` opens files `OpenOptions::new().create(true).write(true).truncate(true)` with **no `.mode()`** → ~0o644. Affects `vault.key` (private key, every `create()`, vault.rs:97), `vault.pgp` (every `save()`, vault.rs:148), their `.tmp` (default) and `.bak` (`fs::copy` propagates 0o644), and the parent dir (`std::fs::create_dir_all`, vault.rs:72 → 0o755). The team rated the IDENTICAL exposure on opt-in `backup_key` as HIGH/must-fix; the primary `vault.key` is strictly more exposed (written for every vault, no test on its mode → per-task review missed it). Both world-readable `vault.key`+`vault.pgp` collapse at-rest confidentiality to an offline ~354ms/guess S2K attack by any local user/process/backup agent. Rated Important (not Critical) only because the key stays S2K-encrypted.
**Fix:** route `vault.key` (mandatory) + `vault.pgp` through owner-only writes; create vault parent dir owner-only. Give `atomic_write` owner-only mode on Unix (target + `.tmp`; set `.bak` perms after `fs::copy`). Add a perms test for `vault.key`/`vault.pgp` mirroring the snapshot/backup tests.

## Minor
- **M-1** `open()`/`recover_target` recovers from `.bak` only when target MISSING, never present-but-corrupt; stated NFR3 guarantee broader than impl (POSIX torn-target effectively impossible). Consider retry-from-`.bak` on decrypt/decode failure (overlaps FOLLOWUPS kill-harness).
- **M-2** Save-path plaintext not zeroized (asymmetric vs open path): `db_to_bytes`/`encode_blob` `Vec`s drop unzeroized. Within accepted R1 bound (SQLite heap already unprotected). Cheap: wrap in `SecretBuf`/zeroize after encrypt.
- **M-3** Windows owner-only = best-effort ACL inheritance; under I-1 fix the primary artifacts also need the Windows-ACL FOLLOWUPS note (currently only backups).
- **M-4** `db_to_bytes` can mislabel a true OOM during the empty-DB `page_count` probe (recorded Task-7 M-1; cosmetic; deferrable).

## Nits
- **N-1** `blob` round-trip doesn't byte-pin BE wire format; add `assert_eq!(&b[0..4],&[0,0,0,1])` (recorded Task-1).
- **N-2** 3 integration tests redundant `.save()` after `create()` (recorded Task-7 M-2).
- **N-3** `empty_db_roundtrip` could assert `sqlite_master` count==0 pre-write (recorded Task-7 N-2).
- **N-4** `.gitignore` `!/crates/btctax-store/src/vault.rs` scoped OK; future `vault*.rs` source would be silently ignored by broad `vault*`. Fine for now.

## Cross-cutting verification (load-bearing claims) — all ✔
1. Wrong-pass oracle sound (Arc<AtomicBool> set only after `decrypt_secret` succeeds; corrupt-ct+correct-pass→Crypto covered; not a network oracle).
2. FFI memory safety sound (empty short-circuited before `sqlite3_malloc64`; non-null checked; `copy_nonoverlapping`; `OwnedData` consumed by value→no double-free; SQLite owns/frees).
3. Crash windows sound on POSIX (tmp-fsync→copy-bak-write-handle-fsync→atomic rename→dir fsync; reap guarded on target-present; recover-before-reap correct for both sidecars; only the documented first-create residual).
4. Cross-platform composition coherent (fs2 WouldBlock mapping MSRV-ok; `fs2::FileExt::unlock` FQ avoids std 1.89 clash; VirtualLock cast sound; mlock/munlock len-consistent, zeroize-before-unlock). Windows ACL gap documented.
5. Spec/plan faithful (§8 layout, S2K assertion, two-file deviation documented, open/save protocols, migrate stub, full public interface; re-lock-on-timeout deferred to CLI).

## Triage of recorded findings
All recorded Critical/Important/HIGH/MEDIUM resolved in code (Task-3 corrupt-ct branch; Task-4 three Importants; Task-8 HIGH/MEDIUM/I-1 backup). Remaining recorded items Minor/Nit, deferrable. **The one merge-blocker (I-1) is NEW — not in the ledger — the canonical whole-branch finding.**

## Verdict (Round 1)
**Not ready to merge. 0 Critical / 1 Important (I-1).** Fix: owner-only `vault.key` (mandatory) + `vault.pgp` + parent dir, with a perms test. Fold M-1/M-2 into the change or FOLLOWUPS; update Windows-ACL note (M-3). Re-review the fold per §2.

## Round 2 — I-1 fold re-review (commit 55980c4)
Independent re-review of the fold (diff 4bdc16b..55980c4). **I-1 FULLY CLOSED; fold sound. 0 Critical / 0 Important / 0 Minor / 1 Nit.**
- New `crate::fsperms` module is the single authoritative source (`open_owner_only` 0o600, `write_owner_only`, `restrict_file_to_owner`, `mkdir_owner_only` 0o700; Unix via OpenOptionsExt/DirBuilderExt/PermissionsExt; non-Unix plain). Old duplicate helpers removed from vault.rs (no divergent copies).
- All reachable secret-artifact write paths covered: `.tmp` opened 0o600 → mode carries through `rename` to `vault.key`/`vault.pgp`; `.bak` explicitly `restrict_file_to_owner` after `fs::copy`; parent dir `mkdir_owner_only` 0o700.
- No crash-safety regression: durability ordering (tmp-fsync → copy-bak → restrict → bak-fsync → atomic rename → dir-fsync) structurally unchanged; `set_permissions` is metadata-only.
- New `#[cfg(unix)] vault_artifacts_are_owner_only` test asserts exact `& 0o777 == 0o600` (key/pgp/bak) and `0o700` (dir), exercising `.bak` via a 2nd save. 29 tests green, clippy/fmt clean.
- **Nit:** `recover_target` could add an explicit `restrict_file_to_owner` on the restored target (belt-and-suspenders; already passively 0o600 since the bak source is 0o600). Folded under FOLLOWUPS M-1 (recover_target hardening).

**Net branch status: 0 Critical / 0 Important — GREEN. Ready to finish.**
