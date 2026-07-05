# SPEC — btctax-store security/durability hardening

**Source baseline:** `main` @ `9763331` (branch `feat/store-hardening`). **Review status: DRAFT — awaiting R0
(2 rounds to 0C/0I).** Resolves the three deferred `btctax-store` FOLLOWUPS (M-2 zeroize, M-1 .bak-on-corrupt,
kill-mid-save fuzz). User-approved (2026-07-05). No tax-logic change — storage-layer only.

## Goal
Three focused hardening improvements to `btctax-store`, each independently valuable + testable:
1. **Zeroize the save-path plaintext buffers** (M-2).
2. **Recover from `.bak` when the present vault is CORRUPT** (not only when missing) — but NEVER on
   `WrongPassphrase` (M-1).
3. **A kill-mid-save state-enumeration harness** proving `open` is safe from every intermediate on-disk state.

## Current state (verified)
- `save()` (vault.rs:147-149): `image = sqlite_io::db_to_bytes(&conn)` (plaintext SQLite `Vec<u8>`) →
  `blob::encode_blob(SCHEMA_VERSION, &image)` (plaintext `Vec<u8>`) → `crypto::encrypt_to(..)`. Neither the
  `image` nor the encoded blob is zeroized on drop. Same for `snapshot()` (156-157) + the backup path (171).
  The READ path already zeroizes via `SecretBuf` (vault.rs:133-135) — the pattern to reuse.
- `open()` (vault.rs:116-136): `recover_target(f)?` restores from `.bak` only when the target is **absent**
  (atomic.rs:35); a PRESENT-but-corrupt vault is NOT retried. Corruption surfaces at decrypt/decode
  (vault.rs:133 `decrypt_with` → `Crypto`; :134 `decode_blob` → `Corrupt`; :136 `db_from_bytes` → `Sqlite`;
  `UnsupportedSchema`) — all DISTINCT from `WrongPassphrase` (crypto.rs:140/168; the test
  `corrupted_ciphertext_with_correct_pass_is_crypto_err_not_wrongpass` crypto.rs:173 pins the distinction).

## T1 — zeroize the save-path plaintext (M-2)
Wrap every save-path plaintext `Vec<u8>` in the existing zeroizing wrapper (`SecretBuf`, as the read path
does) so it is scrubbed on drop: the `db_to_bytes` image + the `encode_blob` output in `save()`, `snapshot()`,
and the backup path. `encrypt_to`/`encode_blob` take `&[u8]` → `SecretBuf::as_slice()`. `snapshot()` returns
`Vec<u8>` (the FR10 plaintext-export exception) — keep its return type, but zeroize the intermediate.
- **Bound (state honestly):** the live SQLite connection holds plaintext in heap all session (accepted R1
  bound) — zeroizing the save buffers is defense-in-depth (shrinks the plaintext-copy lifetime/count), NOT
  full at-rest secrecy. Document it; do not overclaim.
- KAT: a test that the save path produces a `SecretBuf` (compile-level) + (best-effort) that no extra
  un-zeroized plaintext `Vec` remains in `save()` (code-review-enforced; a unit test asserts the wrapper type).

## T2 — recover from `.bak` on a CORRUPT present vault (M-1)
In `open()`, after the existing `recover_target` (missing-target case), when decrypt/decode of the PRESENT
`vault.pgp` fails:
- if the error is `WrongPassphrase` → **propagate immediately** (never touch `.bak` — it would also
  `WrongPassphrase`; a caller error, not corruption).
- else (a corruption class: `Crypto` / `Corrupt` / `Sqlite` / `UnsupportedSchema`) AND a `.bak` exists → retry
  the open using the `.bak`; if the `.bak` decrypts+decodes cleanly → **atomically restore** it
  (`.bak` → `vault.pgp`, via the existing atomic write) and return the recovered vault; else → propagate the
  ORIGINAL target error (both corrupt).
- no `.bak` → propagate the corruption error.
Factor the "decrypt+decode+db_from_bytes into a Vault" steps into a helper reused for target + `.bak`.
- KATs: `open_recovers_from_bak_when_target_corrupt` (present target = garbage bytes, good `.bak` → opens the
  `.bak` state + `vault.pgp` is restored); `open_wrong_passphrase_never_touches_bak` (wrong pp, valid target +
  `.bak` → `WrongPassphrase`, `.bak` untouched, no recovery); `open_both_corrupt_propagates`
  (target + `.bak` both garbage → the corruption error, not a panic); `open_missing_target_still_recovers`
  (the existing behavior unchanged).

## T3 — kill-mid-save state-enumeration harness
A deterministic test enumerating the on-disk states a kill during `save`/`create` can leave, asserting `open`
is always SAFE (yields a valid Vault OR a clear typed error — never a panic or a silently-wrong state). States
(the cross-product that actually occurs): `vault.pgp` ∈ {absent, good, corrupt} × `.bak` ∈ {absent, good} ×
`.tmp` ∈ {absent, present}, with `vault.key` present. For each: `open` → assert Ok(valid) or a specific
`StoreError` (e.g. `HalfCreatedVault`, `WrongPassphrase` N/A here, a corruption error only when BOTH
target+bak unusable), and that a recoverable state (good `.bak`) recovers. (Deterministic NFR4 — a systematic
enumeration, not RNG; a true `kill -9` OS harness stays a FOLLOWUP.) Reuses T2's recovery.

## Scope / SemVer / lockstep
`btctax-store` only (vault.rs save/open + a recovery helper; tests). No public-API break (save/open/snapshot
signatures unchanged; `SecretBuf` is internal). No btctax-core/cli/tui change. PATCH-class (internal hardening;
no behavior change except a corrupt-vault now recovers from `.bak` instead of erroring — strictly safer).
Lockstep: none (no CLI/doc surface). Update the FOLLOWUPS `btctax-store` M-1/M-2 + kill-mid-save entries → resolved.

## Plan (TDD)
- **T1** — `SecretBuf`-wrap the save-path plaintext; the wrapper KAT; note the R1 bound in the doc-comment.
- **T2** — the `.bak`-on-corrupt recovery helper + `open` wiring (WrongPassphrase-never); the 4 recovery KATs.
- **T3** — the state-enumeration harness; FOLLOWUPS update; whole-diff + full suite.

## Gotchas
- **NEVER retry `.bak` on `WrongPassphrase`** — it would also fail + mislead ("recovered" when the password is
  just wrong). Discriminate on the exact `StoreError` variant.
- **Restore atomically** — recovering `.bak → vault.pgp` must use the existing atomic write (crash-safe), not a
  bare copy.
- **Don't overclaim zeroize** — the live SQLite heap holds plaintext all session; this is defense-in-depth.
- **Both-corrupt = a clear error, never a panic/loop** — bounded single retry of `.bak`.
- **Determinism** — the harness enumerates states; no RNG/`Date::now`.
