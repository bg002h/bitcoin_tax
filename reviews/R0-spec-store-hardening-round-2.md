# R0 — SPEC review, round 2 — `design/SPEC_store_hardening.md`

- **Artifact:** `design/SPEC_store_hardening.md` (round-1 folded; 2C/3I/2M/2N merged in-place)
- **Baseline:** branch `feat/store-hardening` @ `735d25a` (main `9763331`) — verified `git rev-parse HEAD == 735d25a…`, `main == 9763331…`
- **Reviewer role:** independent architect (author ≠ reviewer); read-only verification pass
- **Scope re-verified against current source:** `crates/btctax-store/src/{vault,crypto,atomic,blob,sqlite_io,memlock,paths,lib}.rs`, `FOLLOWUPS.md`
- **Prior round:** `reviews/R0-spec-store-hardening-round-1.md` (2 Critical / 3 Important / 2 Minor / 2 Nit — NOT GREEN)
- **Bar:** 0 Critical / 0 Important

## Verdict: 0 Critical / 0 Important / 1 Minor / 2 Nit — **R0-GREEN**

Both round-1 Criticals (durability regressions) and all three Importants are resolved and correct against source. The remaining items are non-blocking clarity/durability-completeness notes for the plan/implementation phase. The spec may proceed to Plan.

---

## Fold-by-fold confirmation (each verified against current source)

### [C1] RESOLVED & CORRECT — `UnsupportedSchema` excluded from the recovery trigger

- **The propagate-set is now correct.** Spec §T2 line 42–43 lists `UnsupportedSchema` under **propagate immediately** ("the vault was written by a NEWER app — decode SUCCEEDED; recovering the older `.bak` would silently DOWNGRADE + lose newer tax data. NOT corruption"). The genuine-corruption trigger is now exactly `Crypto` OR `Corrupt` OR deserialize-`Sqlite` (spec line 46–47).
- **`migrate` really returns `UnsupportedSchema` only AFTER a successful decode.** Trace in `vault.rs`: `:133` `decrypt_with` → `SecretBuf` (decrypt succeeded), `:134` `decode_blob` → `(ver, image)` (decode succeeded), `:135` `blob::migrate(ver, …)`. `blob.rs:20–26`: `migrate` returns `Ok(image)` when `version == SCHEMA_VERSION` else `Err(UnsupportedSchema(version))`. So reaching `UnsupportedSchema` **proves** decrypt+decode both succeeded → it is genuinely a newer/foreign vault, not corruption. Excluding it is correct.
- **The corruption variants map as the spec claims.** `Crypto` from `decrypt_with` (crypto.rs:137–141: `Crypto` only when the key *did* unlock but decrypt failed; `WrongPassphrase` is the disjoint never-unlocked case — pinned by `corrupted_ciphertext_with_correct_pass_is_crypto_err_not_wrongpass`, crypto.rs:173–187); `Corrupt` from `decode_blob` (blob.rs:11–12, `< 4-byte header`); `Sqlite` from `db_from_bytes` `deserialize` (sqlite_io.rs:48).
- **KAT pins it.** `open_unsupported_schema_never_recovers_from_bak` (spec line 61–62): newer target + older `.bak` → `UnsupportedSchema`, `.bak` untouched, NO downgrade. This is the exact data-loss window from round-1 C1.
- **FOLLOWUPS grounding reinforces the fold.** `FOLLOWUPS.md:1484` scopes M-1 to *"retrying from `.bak` on decrypt/decode failure … must NOT retry on WrongPassphrase."* A schema mismatch is **not** a decrypt/decode failure (both succeeded), so excluding it is faithful to the followup, not a narrowing.

### [C2] RESOLVED & CORRECT — `.bak`-PRESERVING restore, `atomic_write` not reused

- **`atomic_write` really would be the C2 bug.** `atomic.rs:16–23`: when `target.exists()`, it does `fs::copy(target, &bak)` (`:19`) **before** the `fs::rename(&tmp, target)` (`:24`). In a restore, `target` (`vault.pgp`) is the corrupt file and `.bak` is the sole good copy — so reuse would copy corrupt-over-good. Confirmed dangerous.
- **The new primitive avoids it.** Spec §T2 line 52–53: read `.bak` bytes → write `<vault>.tmp` (owner-only) → fsync → rename `.tmp`→`vault.pgp`; **"NEVER touch `.bak`"**; "Do NOT reuse `atomic_write` (it clobbers `.bak`, C2)." Gotchas line 92–93 repeat the constraint. This omits the `fs::copy(target, &bak)` step entirely.
- **The primitive is itself crash-safe** (no new window). Because `.bak` is never mutated, every crash point during restore is recoverable on the next `open()`:
  - crash before/during `.tmp` write → `{pgp:corrupt, bak:good, tmp:absent|garbage}`; `open` runs `reap_tmp(vault)` (atomic.rs:47–51, removes `.tmp` since target present) then re-recovers from the intact `.bak`.
  - crash after fsync, before rename → `{pgp:corrupt, bak:good, tmp:good}`; same path — `reap_tmp` drops the `.tmp`, `.bak` re-recovers.
  - crash after rename → `{pgp:good, bak:good, tmp:gone}`; `open` succeeds directly.
  The invariant "`.bak` is the untouched safety net" makes the restore effectively idempotent/repeatable.
- **No stale-`.tmp` collision with a normal save.** The restore runs inside `open` under the held `VaultLock` (no concurrent `save`), and `open` already ran `reap_tmp(vault)` at `vault.rs:125` before the decrypt/recovery region (`:133`), plus `open_owner_only` truncates — so any pre-existing `.tmp` is gone/overwritten.
- **KAT pins it.** `restore_preserves_bak_and_is_crash_safe` (spec line 63): after restore, `.bak` bytes unchanged. Combined with the T3 crash-window state (below), C2 is covered.

### [I1] RESOLVED & CORRECT — plaintext-buffer list re-enumerated precisely

Verified each site in `vault.rs`:
- **`save()` (`:148–149`)** — `:148 let image = sqlite_io::db_to_bytes(&self.conn)?` (plaintext DB image) and the `:149 blob::encode_blob(SCHEMA_VERSION, &image)` output (a fresh `Vec` that copies the plaintext image with a 4-byte header, blob.rs:3–8, passed to `encrypt_to` then dropped). Both real, wrappable. ✓ (spec line 28)
- **`export_snapshot()` image (`:171`)** — `let image = sqlite_io::db_to_bytes(&self.conn)?` written to disk then dropped; real plaintext intermediate, correctly re-labelled from round-1's mislabel "backup path." ✓ (spec line 30)
- **`backup_key()` armored (`:184`)** — `self.cert.as_tsk().armored().to_vec()`; the S2K-encrypted private key. Lower sensitivity than tax plaintext, but the spec now **names the decision to include it** (round-1 I1 asked only that the decision be named). ✓ (spec line 31)
- **NOT `snapshot()` (`:156–158`)** — body is `sqlite_io::db_to_bytes(&self.conn)` **returned directly**; the sole `Vec` is the caller-owned FR10 return, no wrappable intermediate. Spec correctly excludes it and states why (wrapping the return would break FR10 / be a no-op). ✓ (spec line 32–33)

These are the real plaintext intermediates in the save/export/backup paths; `snapshot()` has none. Confirmed.

### [I2] RESOLVED & CORRECT — recovery warns

Spec §T2 line 51–52 emits an `eprintln!` on `.bak` recovery ("vault.pgp was corrupt; recovered from vault.pgp.bak (prior save generation)"). Precedent verified: `memlock.rs:16` uses exactly this `eprintln!("warning: …")` pattern on mlock failure. The KAT `open_recovers_from_bak_when_target_genuinely_corrupt` asserts the warning is emitted (spec line 60–61). No longer a silent revert. ✓

### [I3] RESOLVED & CORRECT — harness cross-product completed

Spec §T3 line 68–72 now enumerates `vault.pgp ∈ {absent, good, corrupt} × .bak ∈ {absent, good, corrupt} × .tmp ∈ {absent, present-ciphertext-of-a-good-save, present-garbage}`. This adds the round-1 gaps: **corrupt `.bak`** (so the both-corrupt propagation branch is reachable), specified **`.tmp` content** (so the good-`.tmp` state is constructible), and an explicit assertion that **the C2 crash-window state (mid `.tmp` restore) never loses the good `.bak`.** The enumeration can now reach every branch it claims to prove. ✓

### [M1] RESOLVED — recovery helper + single lock

Spec line 57: factor `decrypt+decode+db_from_bytes → Vault` into a helper reused for target + `.bak`; "attach the single held lock once (don't re-acquire)." The helper boundary (`decrypt+decode+db_from_bytes`, i.e. `vault.rs:133–136`) correctly excludes the `Cert::from_bytes` cert-load at `:132` (which *also* maps to `StoreError::Crypto` — atomic.rs mapping via `.map_err(StoreError::Crypto)`), so a corrupt **key** is not inside the recovery trigger. Single-lock attachment is specified. (One residual clarity nit below — the explicit "corrupt-key propagates without `.bak` retry" sentence round-1 requested is *implied* by the boundary but not spelled out.)

### [M2] RESOLVED & CORRECT — `Sqlite`-not-OOM pinned

Spec line 46–47 restricts the recoverable `Sqlite` to deserialize corruption and notes "NOT OOM — `db_from_bytes` remaps OOM→`Io`, sqlite_io.rs:40–43 [pin with an assertion]." Verified: `sqlite_io.rs:39–43` returns `StoreError::Io(OutOfMemory)` on `sqlite3_malloc64` failure (excluded from the trigger, which routes to `Io`→propagate, spec line 44); the only `Sqlite` escaping `db_from_bytes` is the `deserialize` error at `:48`. The plan carries the assertion (spec line 85). So a `Sqlite` reaching the recover-and-restore path is deterministic image corruption, never a transient OOM. ✓

---

## Self-consistency & new-gap re-scan (answering the round-2 charter)

1. **Residual contradiction after the rewrite?** None found. The propagate-set (line 44), corruption-set (line 46–47), Gotchas (line 90–95), the SemVer "strictly safer" claim (line 76–78), and the plan (line 81–87) are mutually consistent. The "both round-1 Criticals were durability REGRESSIONS" framing (line 6–7) is accurate for C1 (silent downgrade) and C2 (`.bak` clobber + crash window).

2. **Other `StoreError` classes where `.bak` recovery is still dangerous?** Re-scanned every variant (`lib.rs:17–40`): `Io`, `Crypto`, `Locked`, `WrongPassphrase`, `Corrupt`, `Sqlite`, `UnsupportedSchema`, `AlreadyExists`, `HalfCreatedVault`, `InvalidVaultPath`.
   - `WrongPassphrase`, `UnsupportedSchema`, `Io`, `Locked` → propagate (spec correctly excludes). `AlreadyExists`/`HalfCreatedVault`/`InvalidVaultPath` fire in `create`/guards, before the decrypt region.
   - A truncated ciphertext body can surface as `Io` from `std::io::copy` during `decrypt_with` (crypto.rs:145) rather than `Crypto` — this **propagates** instead of recovering. That is *fail-safe* (conservative miss, `.bak` untouched), not dangerous. No data loss. Not a finding.
   - A `Connection::open_in_memory()` failure *inside* `db_from_bytes` (sqlite_io.rs:34) would also be `Sqlite` and (harmlessly) trigger a `.bak` retry, but that path only *restores* if `.bak` opens cleanly — and an environmental in-memory-open failure would fail identically for `.bak` → propagate. Not dangerous. (Round-1 M2 already blessed the `Sqlite` class as deterministic-corruption for the dominant path.)
   No dangerous class remains.

3. **`UnsupportedSchema`-propagate vs. the missing-target `recover_target` path — is the "newer `.bak` than a missing target" case N/A? Confirmed N/A.** `recover_target` (atomic.rs:35–42) copies `.bak`→target **only when the target is ABSENT** — i.e. there is no newer data to lose (the `.bak` is the sole copy). After it copies, `open` decrypts/decodes/`migrate`s the restored bytes: if the `.bak` was a newer schema, `migrate` returns `UnsupportedSchema` and propagates (upgrade prompt) — no downgrade, no loss; if same schema, clean open. So the C1 downgrade concern (present newer vault clobbered by older `.bak`) cannot arise on the missing-target path. Separately, in the **new** T2 present-corrupt path, a `.bak` that is a newer schema fails the shared helper (returns `UnsupportedSchema`, i.e. "not opened cleanly") → the spec propagates the ORIGINAL target error and **does not restore** (line 54–55), leaving `.bak` intact. Spec line 50–51 explicitly acknowledges the schema-≥ comparison is "N/A here (target didn't decode)." Consistent and safe.

4. **Is the crash-safe restore primitive itself free of a new crash window?** Yes — see C2 above; every crash point is recoverable because `.bak` is never mutated and `open`'s `reap_tmp`/re-recovery loop reconverges on the intact `.bak`.

---

## Residual findings (all non-blocking; for the plan/implementation phase)

### [M1r] MINOR — restore primitive omits an explicit directory-fsync step

Round-1's C2 fix directive included "Also add the fsync-dir step for durability, mirroring atomic.rs:27–29." The spec's restore description (line 52) lists `write .tmp → fsync → rename` but not the parent-dir fsync that `atomic_write` performs (`atomic.rs:27–29`, best-effort). **Not a durability regression** — because `.bak` is preserved, a rename lost to a crash simply re-recovers on the next `open`. But for consistency with `atomic_write` and to make the rename durable on the happy path, the implementation should mirror the dir-fsync. Add one clause to the restore-primitive step, or note it as an accepted best-effort omission.

### [N1r] NIT — M1's "corrupt-key propagates without `.bak` retry" is implied but not stated

The helper boundary (`decrypt+decode+db_from_bytes`, excluding the `:132` cert load) correctly keeps the corrupt-**key** `Crypto` out of the recovery trigger, but round-1 M1 explicitly asked the spec to *state* that a corrupt key (vault.rs:132) propagates `Crypto` without a pointless `.bak` retry. A naive implementer wrapping the whole `:132–136` region in the trigger would do a harmless-but-misleading `.bak` retry (the `.bak` is encrypted to the same broken cert → also fails → propagate original). One sentence pinning the boundary would close the ambiguity. Cosmetic only (no data-loss/behavioral risk).

### [N2r] NIT — round-1 N2 residue (`db_to_bytes` transient `data.to_vec()`) not explicitly folded

`sqlite_io.rs:11` does `data.to_vec()`; the `OwnedData data` holds the serialized plaintext DB and is dropped un-zeroized below the `vault.rs` layer T1 wraps. The spec's honesty bound (line 34–36) names the live-SQLite-heap plaintext but not this specific transient copy. Round-1 recorded it as an accepted Nit; folding one clause ("plus the transient copy inside `db_to_bytes`, sqlite_io.rs:11") under the bound makes the bound exhaustive. Optional.

---

## Verified-correct carry-forward (no finding; recorded so a later round needn't re-derive)

- **`WrongPassphrase`/`Crypto` discrimination is real** (crypto.rs:137–141; tests `wrong_pass` :163–170, `corrupted_ciphertext…` :173–187) — the propagate-vs-recover branch is discriminable on the exact variant.
- **Lock / `recover_target` / `reap_tmp` / half-created ordering (`vault.rs:120–131`) runs before the decrypt region** and is unchanged by T2; the C2 restore reuses the same `.tmp` path safely under the held lock after `reap_tmp` already ran.
- **`N1` (round-1) folded** — spec line 36 states the on-disk `.bak`/`.tmp` are CIPHERTEXT, out of T1 scope.
- **Scope / SemVer** — `btctax-store`-only, no public-API break; `open`/`save`/`snapshot` signatures unchanged; PATCH "strictly safer" now holds (genuine-corruption auto-recovers with `.bak` preserved + warned; newer/wrong-pass unchanged). FOLLOWUPS M-1 (:1484), M-2 (:1485), kill-mid-save (:1490) grounding intact.

## Bottom line
All 2 Critical + 3 Important round-1 findings are folded correctly and verified against `735d25a` source. Remaining items are 1 Minor + 2 Nit, none blocking. **R0-GREEN** — the spec is cleared to proceed to Plan; carry M1r/N1r/N2r into the implementation phase.
