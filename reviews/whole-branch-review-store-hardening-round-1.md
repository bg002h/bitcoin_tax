# Whole-diff review (Phase E) — feat/store-hardening — round 1

**Verdict: 0 Critical / 0 Important / 0 Minor / 0 Nit — SHIP.**

Independent Phase-E review. Diff `main (9763331)..HEAD` — 3 task commits (T1-T3), 7 files, +768/−23. Contract:
`design/SPEC_store_hardening.md` (R0-GREEN, 2 rounds — both round-1 Criticals were durability regressions).
`btctax-store` only; no tax-logic change.

## Fault-injection of the two Critical durability invariants (both restored byte-for-byte)
- **[★ C1 — `UnsupportedSchema` must NEVER recover (downgrade data loss)] CONFIRMED load-bearing.**
  `is_genuine_corruption` (vault.rs:202-207) = `matches!(e, Crypto | Corrupt | Sqlite)` — correctly EXCLUDES
  `UnsupportedSchema` (a NEWER vault whose decode succeeded) + `WrongPassphrase`. **Fault-inject:** adding
  `| UnsupportedSchema(_)` (so a newer vault would recover the older `.bak` → silent downgrade) drove
  `open_unsupported_schema_never_recovers_from_bak` RED. The most dangerous data-loss path is guarded.
- **[★ C2 — restore must PRESERVE the good `.bak`] CONFIRMED load-bearing.** `restore_from_bak` (vault.rs:215)
  writes `.bak` bytes → `.tmp` → fsync file → rename → fsync parent dir; it **never** touches `.bak` (does NOT
  reuse `atomic_write`, which copies target→`.bak` first). **Fault-inject:** injecting a `.bak` clobber after
  the rename drove `restore_preserves_bak_and_is_crash_safe` RED. A crash mid-restore leaves the sole good copy intact.

## The rest (inspection + named KATs)
- **T2 recovery** — fires ONLY on genuine corruption (`Crypto`/`Corrupt`/deserialize-`Sqlite`, NOT OOM — pinned
  vs sqlite_io.rs), a single bounded `.bak` attempt, `eprintln!` warning (memlock.rs:16 precedent), corrupt
  `vault.key` propagates with no retry (N1r). KATs: `open_recovers_from_bak_when_target_genuinely_corrupt`,
  `open_wrong_passphrase_never_touches_bak`, `open_both_corrupt_propagates_and_bak_intact`,
  `open_missing_target_still_recovers` (unchanged). Lock/recover_target/half-created ordering preserved.
- **T1 zeroize** — `SecretBuf`-wrapped at the real sites (save image+blob, export_snapshot image, backup_key
  armored; NOT snapshot's FR10 return); honest defense-in-depth doc-bound (SQLite heap holds plaintext all
  session; on-disk `.tmp`/`.bak` are ciphertext).
- **T3 harness** — deterministic 3×3×3 (`vault.pgp`×`.bak`×`.tmp`) `open`-is-always-safe enumeration incl. the
  C2 crash-window on every good-`.bak` state.
- No public-API change; existing `wrong_passphrase` / `second_open_locked` / half-created tests stay green.

## Full suite
`cargo test --workspace --locked` **1146 passed / 0 failed**; clippy -D + fmt clean. PATCH-class (now strictly
safer — a genuinely-corrupt vault recovers; newer/wrong-pass unchanged).

**SHIP.**
