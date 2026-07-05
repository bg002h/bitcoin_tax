# SPEC — btctax-store security/durability hardening

**Source baseline:** `main` @ `9763331` (branch `feat/store-hardening`). **Review status: R0-GREEN (2 rounds; 0C/0I).
Cleared to implement.** Reviews: `reviews/R0-spec-store-hardening-round-{1,2}.md`. Round 1 2C/3I (both C's were
durability regressions), round 2 0C/0I/1M/2N. **Carry into implementation:** [R0-M1r] the `.bak`-preserving
restore ALSO fsyncs the parent dir after the rename (mirror atomic.rs:27-29); [R0-N1r] a corrupt vault.key
(`Cert::from_bytes` fail, vault.rs:132) propagates with NO `.bak` retry (the KEY is not `.bak`-recoverable);
[R0-N2r] `db_to_bytes`'s internal `data.to_vec()` (sqlite_io.rs:11) is a transient plaintext copy — note under
the honesty bound. Review:
`reviews/R0-spec-store-hardening-round-1.md`. Resolves 3 deferred `btctax-store` FOLLOWUPS. User-approved
(2026-07-05). Storage-layer only; no tax-logic change. **Both round-1 Criticals were durability REGRESSIONS —
the folds below are load-bearing for the "strictly safer" claim.**

## Goal
1. **Zeroize the save-path plaintext buffers** (M-2).
2. **Recover from `.bak` when the present vault is GENUINELY CORRUPT** — never on `WrongPassphrase` NOR
   `UnsupportedSchema`, and without clobbering the good `.bak` (M-1).
3. **A kill-mid-save state-enumeration harness** proving `open` is safe from every intermediate on-disk state.

## Current state (verified)
- `save()` (vault.rs:147-149): `image = db_to_bytes(&conn)` (plaintext) → `encode_blob(SCHEMA_VERSION,&image)`
  (plaintext) → `encrypt_to(..)` — neither zeroized. READ path zeroizes via `SecretBuf` (vault.rs:133-135).
- `open()` (vault.rs:116-136): `recover_target(f)?` restores from `.bak` only when the target is ABSENT
  (atomic.rs:35). Corruption of a PRESENT vault surfaces at decrypt/decode (:133 `decrypt_with`→`Crypto`; :134
  `decode_blob`→`Corrupt`; :135 `migrate`→`UnsupportedSchema`; :136 `db_from_bytes`→`Sqlite`) — `WrongPassphrase`
  is DISTINCT (crypto.rs:137-142, test :173-187). No existing test asserts "corrupt present vault → error" — T2
  flips nothing.
- `atomic_write` (atomic.rs:16-23) copies the CURRENT target over `.bak` BEFORE the rename — safe for a normal
  save, DANGEROUS for a restore (would copy corrupt-over-good, C2).

## T1 — zeroize the save-path plaintext (M-2) [R0-I1 buffer list corrected]
`SecretBuf`-wrap the plaintext intermediates so they scrub on drop, at the ACTUAL sites:
- `save()` — the `db_to_bytes` image + the `encode_blob` output (vault.rs:148-149).
- `export_snapshot` — the `db_to_bytes` image (vault.rs:171) [R0-I1: this is the real site, mislabeled
  "backup path" in round 1].
- `backup_key` — the `armored` secret-key bytes (vault.rs:184) [R0-I1: was unaddressed].
- **NOT `snapshot()`** — its only `Vec` is the caller-owned FR10 return (vault.rs:156-158); there is no
  wrappable intermediate (wrapping the return would break FR10 / be a no-op) [R0-I1].
- **Bound (honest):** the live SQLite connection holds plaintext in heap all session (accepted R1 bound) —
  this is defense-in-depth (shrinks copy count/lifetime), NOT full at-rest secrecy. State it; don't overclaim.
  [R0-N1: the `.bak`/`.tmp` on disk are CIPHERTEXT, not plaintext — not a zeroize concern.]
- KAT: the save path binds `SecretBuf` (type-level); code-review-enforced no-stray-plaintext-`Vec`.

## T2 — recover from `.bak` on a GENUINELY-CORRUPT present vault (M-1)
In `open()`, when decrypt/decode of the PRESENT `vault.pgp` fails, branch on the exact `StoreError`:
- **`WrongPassphrase`** → propagate immediately (never touch `.bak` — caller error; `.bak` would also fail).
- **[R0-C1] `UnsupportedSchema`** → propagate immediately (the vault was written by a NEWER app — decode
  SUCCEEDED; recovering the older `.bak` would silently DOWNGRADE + lose newer tax data). NOT corruption.
- **`Io` (other than the already-handled missing-target)** / **`Locked`** → propagate (not corruption).
- **GENUINE corruption** = `Crypto` (bad ciphertext, correct pass) OR `Corrupt` (bad blob decode) OR a
  deserialize-`Sqlite` (bad image; NOT OOM — `db_from_bytes` remaps OOM→`Io`, sqlite_io.rs:40-43 [R0-M2, pin
  with an assertion]) AND a `.bak` exists → attempt to open the `.bak` (same decrypt+decode helper):
  - `.bak` opens cleanly → **[R0-C1] additionally require the `.bak` schema ≥ target's decoded schema** is
    N/A here (target didn't decode); **[R0-I2] WARN** (`eprintln!` — precedent memlock.rs:16 — "vault.pgp was
    corrupt; recovered from vault.pgp.bak (prior save generation)") + return the recovered Vault, after a
    **[R0-C2] `.bak`-PRESERVING crash-safe restore**: read `.bak` bytes → write `<vault>.tmp` (owner-only) →
    fsync → rename `.tmp`→`vault.pgp`. **NEVER touch `.bak`** (it stays the safety net; a crash mid-restore
    leaves `.bak` intact). Do NOT reuse `atomic_write` (it clobbers `.bak`, C2).
  - `.bak` also fails → propagate the ORIGINAL target corruption error (both unusable; a clear error, never a
    panic/loop — a bounded single `.bak` attempt).
- no `.bak` → propagate the corruption error.
- **[R0-M1]** factor `decrypt+decode+db_from_bytes → Vault` into a helper reused for target + `.bak`; attach
  the single held lock once (don't re-acquire).
- KATs: `open_recovers_from_bak_when_target_genuinely_corrupt` (garbage target bytes, good `.bak` → opens
  `.bak` state, `vault.pgp` restored, `.bak` STILL present, warning emitted); `open_wrong_passphrase_never_touches_bak`;
  **`open_unsupported_schema_never_recovers_from_bak`** [R0-C1] (newer target + older `.bak` → `UnsupportedSchema`,
  `.bak` untouched, NO downgrade); `open_both_corrupt_propagates_and_bak_intact`; `open_missing_target_still_recovers`
  (unchanged); `restore_preserves_bak_and_is_crash_safe` [R0-C2] (after restore, `.bak` bytes unchanged).

## T3 — kill-mid-save state-enumeration harness [R0-I3 extended]
Deterministic (NFR4) enumeration of the on-disk states a kill during `save`/`create`/restore can leave,
asserting `open` is always SAFE (valid Vault OR a specific typed error — never a panic or silent-wrong):
`vault.pgp` ∈ {absent, good, corrupt} × `.bak` ∈ {absent, good, **corrupt** [R0-I3]} × `.tmp` ∈ {absent,
**present-ciphertext-of-a-good-save**, present-garbage} [R0-I3 specify content], `vault.key` present. For each:
assert `Ok(valid)` or the specific `StoreError` (`HalfCreatedVault` for the half-created signature; a
corruption error ONLY when target+bak both unusable); a good `.bak` recovers; the C2 crash-window state (mid
`.tmp` restore) never loses the good `.bak`. Exercises T2 + the half-created path (vault.rs:38). True `kill -9`
OS harness stays a FOLLOWUP.

## Scope / SemVer / lockstep
`btctax-store` only (vault.rs save/open + a recovery helper + a `.bak`-preserving restore primitive; tests). No
public-API break. PATCH-class **(now strictly safer with C1/C2 folded)** — a genuinely-corrupt vault recovers
from `.bak`; a NEWER vault + wrong pass are unchanged (propagate). No btctax-core/cli/tui change; no CLI/doc
surface. Update FOLLOWUPS M-1/M-2/kill-mid-save → resolved.

## Plan (TDD)
- **T1** — `SecretBuf`-wrap the save/`export_snapshot`/`backup_key` plaintext (NOT snapshot); the R1-bound note.
- **T2** — the `decrypt+decode→Vault` helper + `.bak`-preserving crash-safe restore primitive + `open` wiring
  (WrongPassphrase & UnsupportedSchema NEVER recover; only genuine corruption); the 6 recovery KATs + the
  Sqlite-not-OOM assertion.
- **T3** — the extended state-enumeration harness (incl. corrupt `.bak`, `.tmp` content, the C2 window);
  FOLLOWUPS update; whole-diff + full suite.

## Gotchas
- **[C1] `UnsupportedSchema` ≠ corruption** — it's a NEWER vault (decode succeeded); recovering `.bak` would
  downgrade + lose data. Propagate. Same for `WrongPassphrase`.
- **[C2] restore must PRESERVE `.bak`** — write via `.tmp`→rename to `vault.pgp` ONLY; never copy target→`.bak`
  (that clobbers the sole good copy). Do NOT reuse `atomic_write`.
- **[I2] WARN on recovery** — a silent revert to the prior save generation is a data-integrity surprise.
- **Both-corrupt = a clear error, bounded single `.bak` attempt** — never a panic/loop.
- **Don't overclaim zeroize** (SQLite heap holds plaintext all session); on-disk `.bak`/`.tmp` are ciphertext.
- **Determinism** — the harness enumerates states; no RNG/`Date::now`.
