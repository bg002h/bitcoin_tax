# R0 — SPEC review, round 1 — `design/SPEC_store_hardening.md`

- **Artifact:** `design/SPEC_store_hardening.md` (DRAFT)
- **Baseline:** branch `feat/store-hardening` @ `56e4d42` (main `9763331`)
- **Reviewer role:** independent architect (author ≠ reviewer)
- **Scope verified against current source:** `crates/btctax-store/src/{vault,crypto,atomic,blob,sqlite_io,memlock,paths,lock,lib}.rs`, `crates/btctax-store/tests/{integration,smoke}.rs`, `FOLLOWUPS.md`
- **Bar:** 0 Critical / 0 Important

## Verdict: 2 Critical / 3 Important / 2 Minor / 2 Nit — **NOT GREEN** (blocks; fold before implementation)

The two Criticals are both in T2 and both cause **silent / crash-window data loss** — i.e. the hardening as specified is, in two concrete states, a *durability regression*, which falsifies the spec's own "strictly safer" PATCH justification (line 65). Fix before any code.

---

### [C1] CRITICAL — `UnsupportedSchema` in the recover-and-restore set silently destroys a newer vault on downgrade

**Where:** spec §T2 lines 41–43 list the corruption class as `Crypto / Corrupt / Sqlite / UnsupportedSchema` and instruct: on any of these + a good `.bak`, **atomically restore `.bak → vault.pgp`**. Source: `blob::migrate` (vault.rs:135 → blob.rs:20–26) returns `UnsupportedSchema(version)` **after** a *successful* decrypt (vault.rs:133) and a *successful* decode (vault.rs:134).

**Problem:** `UnsupportedSchema` is not corruption — it is the "*this vault was written by a newer app*" signal, the whole reason `migrate()` exists. Treating it as corruption and restoring an older `.bak` **overwrites the newer vault with stale data and discards the newer data silently**. Concrete data-loss window:

1. New build (SCHEMA_VERSION=2) saves → `atomic_write` copies the old v1 `vault.pgp` to `vault.pgp.bak`, writes v2. State: `vault.pgp`=v2, `.bak`=v1.
2. User runs an **older** build (SCHEMA_VERSION=1). `open` → decrypt OK, decode OK, `migrate(2)` → `UnsupportedSchema(2)`.
3. Per spec: it's in the "corruption" set + `.bak` (v1) decodes cleanly under the v1 build → **restore `.bak`(v1) → `vault.pgp`, clobbering v2.** The user's most recent tax data is gone, no warning.

Even the *in-memory* fallback (open the v1 `.bak` without a disk restore) is wrong: it silently presents a **stale generation** of a financial ledger as if current.

Note the original FOLLOWUP (FOLLOWUPS.md:1484) scoped this to *"retrying from `.bak` on decrypt/decode failure"* — it never included schema-version mismatch. The spec **widened** the trigger set beyond the followup, and that widening is the bug.

**Fix:** Restrict the recovery trigger to *genuine* corruption only — a failed **decrypt** (`Crypto` from vault.rs:133), a failed **decode** (`Corrupt` from vault.rs:134), or a failed **deserialize** (`Sqlite` from vault.rs:136 / sqlite_io.rs:48). **Exclude `UnsupportedSchema`** — it must propagate so the caller tells the user to upgrade the app. Update the "strictly safer" SemVer claim (line 65) to hold only after this exclusion.

---

### [C2] CRITICAL — reusing `atomic_write` for the `.bak → vault.pgp` restore clobbers the good `.bak` and opens a crash-window that can make a *recoverable* vault *unrecoverable*

**Where:** spec §T2 line 42–43 and Gotchas line 76: *"Restore atomically — recovering `.bak → vault.pgp` must use the existing atomic write."* The only existing atomic write is `atomic::atomic_write(target, bytes)` (atomic.rs:6–31), whose signature forces the call `atomic_write(vault.pgp, good_bak_bytes)`.

**Problem:** `atomic_write`, when `target` exists, **copies the current `target` over `.bak` before the rename** (atomic.rs:16–23: `fs::copy(target, &bak)`). In the restore scenario `target` (`vault.pgp`) is the **corrupt** file and `.bak` is the **only good copy** — so the reuse copies the corrupt vault over the good `.bak`, destroying the recovery source, and creates a crash window with **no good copy on disk except a `.tmp` that the next `open()` reaps**:

Precondition (a plausible post-crash state): `vault.pgp`=corrupt, `.bak`=good, key good, `.tmp`=absent.
1. `open` → decrypt corrupt → `.bak` decodes clean → restore via `atomic_write(vault.pgp, good_bytes)`:
   - write `good_bytes` → `vault.pgp.tmp`, fsync → state `{pgp:corrupt, bak:good, tmp:good}`
   - `target.exists()` → `fs::copy(vault.pgp → .bak)` → state `{pgp:corrupt, bak:CORRUPT, tmp:good}` **← crash here** (only good copy is in `.tmp`)
2. Reboot → `open` → `recover_target(vault)` no-op (pgp present) → **`reap_tmp(vault)` removes `vault.pgp.tmp`** (atomic.rs:47–53, tmp present && target present) → state `{pgp:corrupt, bak:corrupt, tmp:gone}` → decrypt corrupt → `.bak` corrupt → both-corrupt → propagate. **The good copy is destroyed; the vault is unrecoverable.**

Before T2, that same precondition (`pgp` corrupt, `.bak` good) simply propagated a corruption error and **left `.bak` intact for manual recovery**. So the "hardening" turns a manually-recoverable state into a potentially-unrecoverable one — a durability regression, and it defeats T2's own purpose (it destroys the recovery source). Even absent a crash, after a successful restore `.bak` is now a copy of the corrupt file until the next `save()`, so a second corruption of `vault.pgp` in that window is unrecoverable.

**Fix:** The restore must **preserve `.bak` as the source of truth** and must never `fs::copy(corrupt-target → .bak)`. Add a dedicated crash-safe primitive (e.g. `restore_from_bak`) that writes `.bak`'s bytes to `vault.pgp.tmp` then `rename(tmp → vault.pgp)` **without touching `.bak`** (leave the good backup in place; a subsequent normal `save()` refreshes it). Do **not** reuse `atomic_write` verbatim. Also add the fsync-dir step for durability, mirroring atomic.rs:27–29.

---

### [I1] IMPORTANT — T1's plaintext-buffer enumeration is inaccurate: `snapshot()` has no wrappable intermediate, and the real secret buffers (`export_snapshot` image, `backup_key` armored key) are mis-named / missed

**Where:** spec §"Current state" line 17 (*"Same for `snapshot()` (156-157) + the backup path (171)"*) and §T1 line 28–29 (*"`snapshot()` returns `Vec<u8>` … keep its return type, but zeroize the intermediate"*).

**Problem:** verified against vault.rs:
- `snapshot()` (vault.rs:156–158) is exactly `sqlite_io::db_to_bytes(&self.conn)` **returned directly**. It never calls `encode_blob` and has **no intermediate** — the sole `Vec` *is* the caller-owned FR10 return. "Zeroize the intermediate in `snapshot()`" is therefore a **no-op as written**, or, if taken literally, forces a return-type change that **breaks FR10**. The spec's "Same for `snapshot()` (image + encode_blob output)" is factually wrong.
- Line 171 (the spec's *"backup path"*) is actually inside **`export_snapshot()`** (vault.rs:168–175): `let image = sqlite_io::db_to_bytes(...)` written to disk then dropped — this **is** a genuine, wrappable plaintext intermediate, but the spec mis-labels it "backup."
- The function literally named `backup_key()` (vault.rs:176–192) serializes the (S2K-encrypted) private key into `armored` (vault.rs:184–189) — a separate un-zeroized key-material buffer T1 does **not** address at all.

**Fix:** Re-enumerate precisely: (a) `save()` (vault.rs:148–149) — wrap the `db_to_bytes` image **and** the `encode_blob` output [real, keep]; (b) `export_snapshot()` image (vault.rs:171) — wrap the intermediate [real]; (c) `snapshot()` — **nothing to wrap** (its only `Vec` is the FR10 return the caller owns); state this explicitly instead of "zeroize the intermediate"; (d) decide `backup_key`'s `armored` (vault.rs:184) in-scope or explicitly out (it's the encrypted TSK, lower-sensitivity, but name the decision).

---

### [I2] IMPORTANT — T2 recovery is silent; a tax ledger reverting to a prior save generation must warn the user

**Where:** spec §T2 says restore and *"return the recovered vault"* with no user signal; §Goal line 2 frames T2 as recovery.

**Problem:** even in the *correct* (genuine-corruption) case, recovering from `.bak` reverts to the **previous save generation** (atomic.rs:16–23 makes `.bak` the N-1 copy) and silently discards the corrupt current generation. For a financial/tax ledger a user could re-file on a **stale** state believing it's current, unaware a recovery+revert happened. The codebase already has the precedent for exactly this kind of signal — `SecretBuf::new` emits an `eprintln!` warning on mlock failure (memlock.rs:16).

**Fix:** `open` must **not** recover silently. Minimum: emit an `eprintln!` warning on `.bak` recovery ("vault.pgp was corrupt; recovered from backup — changes since the last successful save may be lost"), matching the memlock.rs:16 precedent; better: surface a `recovered: bool` (or a distinct `Ok`-with-notice) so the CLI/TUI can tell the user. Add a KAT asserting the warning/flag fires on recovery and does **not** fire on a clean open.

---

### [I3] IMPORTANT — T3's enumerated cross-product cannot reach the both-corrupt branch (nor the state C2's bug lives in); it can't fulfill its "safe from every intermediate state" charter

**Where:** spec §T3 line 55–56 enumerates `vault.pgp ∈ {absent, good, corrupt} × .bak ∈ {absent, good} × .tmp ∈ {absent, present}`, but line 58–59 asserts it tests *"a corruption error only when BOTH target+bak unusable"* and line 53–54 claims it proves `open` *"safe from every intermediate on-disk state."*

**Problem:** internal inconsistency — `.bak` ranges over `{absent, good}` only, so the enumeration **structurally cannot** produce a corrupt `.bak`, hence cannot reach the both-corrupt propagation branch it claims to cover (and cannot exercise T2's `open_both_corrupt_propagates` KAT). It also omits two states that C2 shows are reachable and dangerous: (a) the **normal post-restore** state `vault.pgp`=good + `.bak`=corrupt; (b) the crash-window state `vault.pgp`=corrupt + `.bak`=corrupt + `.tmp`=**good** (where the only good copy sits in a `.tmp` that `reap_tmp` deletes). `.tmp ∈ {absent, present}` also leaves the `.tmp` *content* (good vs garbage) unspecified, so (b) is unreachable even by accident.

**Fix:** extend `.bak` to `{absent, good, corrupt}` and specify `.tmp` content (`absent | garbage | good`). Add explicit assertions for both-corrupt→typed-error, post-restore `{pgp:good, bak:corrupt}`→opens, and the crash-window `{pgp:corrupt, bak:corrupt, tmp:good}` state (this is the one C2 must be shown to survive). The "true `kill -9` OS harness deferred to FOLLOWUP" scope call (line 60, matching FOLLOWUPS.md:1490) is **acceptable** — a deterministic enumeration satisfies NFR4 — *provided* the enumeration is actually complete, which it currently is not.

---

### [M1] MINOR — pin the recovery-helper boundary: start after the cert load (vault.rs:132) and attach the lock exactly once

**Where:** spec §T2 line 46 (*"factor the decrypt+decode+db_from_bytes into a helper reused for target + `.bak`"*).

**Problem:** `Cert::from_bytes` on the **key** file (vault.rs:132) also maps to `StoreError::Crypto`. If the helper is factored to include line 132, a corrupt **key** (not vault) `Crypto` would pointlessly retry `.bak` — the `.bak` is encrypted to the same broken cert, so it's harmless but misleading ("attempted recovery" on an unfixable key). Also `_lock` (vault.rs:120, 141) must be moved into the **final** returned `Vault` exactly once — a per-source helper must not consume it.

**Fix:** spec the helper as `fn open_image(cert: &Cert, pp, ct_bytes: &[u8]) -> Result<Connection, StoreError>` covering only decrypt (133) + decode (134/135) + deserialize (136); load the cert once outside it; attach `_lock` only to the finally-selected source's `Vault`. State that the corrupt-**key** case (line 132) propagates `Crypto` without a `.bak` retry.

### [M2] MINOR — pin the invariant that makes `Sqlite` safe in the recovery set (so a future `db_from_bytes` refactor can't smuggle a transient error into the restore path)

**Where:** spec §T2 includes `Sqlite` in the corruption class.

**Note:** this is currently **correct** — `db_from_bytes` remaps `sqlite3_malloc64` OOM to `StoreError::Io` (sqlite_io.rs:40–43, excluded from the retry set), and `db_to_bytes` remaps the 0-page `NOMEM` to empty bytes (sqlite_io.rs:17–22). So a `Sqlite` reaching vault.rs:136 really is deterministic image corruption from `deserialize` (sqlite_io.rs:48), safe to recover from. **Fix:** state this invariant in the spec so a later refactor doesn't route a transient/OOM `Sqlite` into the recover-**and-restore** (C2) path.

### [N1] NIT — the `.bak` copy and `.tmp` in the save path are ciphertext, not plaintext — say so, so T1 scope is unambiguous

`atomic_write` writes the encrypted `ct` to `.tmp` (atomic.rs:12–14) and `fs::copy`s the encrypted `target` to `.bak` (atomic.rs:18). Neither is a plaintext leak, so both are correctly out of T1 scope. One line in the spec closes the question the review prompt raised about the ".bak copy."

### [N2] NIT — `db_to_bytes` leaves one more transient plaintext copy T1 can't reach from `vault.rs`

`db_to_bytes` does `data.to_vec()` (sqlite_io.rs:11): the `OwnedData` `data` holds the serialized plaintext DB and is dropped un-zeroized after the copy. It lives inside `sqlite_io`, below the `vault.rs` layer T1 wraps. Fold it explicitly under the accepted "SQLite/library internals hold plaintext all session" bound (§T1 line 30–32) so the bound is honest about *where* the residue is.

---

## Verified correct (no finding — recorded so round 2 needn't re-derive)

- **T2(a) — the WrongPassphrase / corruption discrimination is real and implementable.** `decrypt_with` returns `WrongPassphrase` only when the secret key never unlocked and `Crypto` when it did unlock but decryption failed (crypto.rs:137–142); pinned by `corrupted_ciphertext_with_correct_pass_is_crypto_err_not_wrongpass` (crypto.rs:173–187) and `wrong_pass` (crypto.rs:163–170). So "propagate on `WrongPassphrase`, recover on corruption" is discriminable on the exact variant.
- **No existing test asserts the OLD "corrupt present vault → error" behavior — T2 flips nothing.** The only present-vault open tests are `wrong_passphrase` (integration.rs:24–36; `.bak` *is* present after two saves, yet must stay `WrongPassphrase` — this actually reinforces T2's never-touch-`.bak`-on-WrongPassphrase rule) and `second_open_locked` (integration.rs:38–51; `Locked` fires at vault.rs:120 before decrypt). `open_recovers_from_bak_if_target_missing` (integration.rs:95–116) exercises only the missing-target path, unchanged by T2. `smoke.rs` has no present-corrupt-vault open.
- **Lock / recover_target / half-created ordering is preserved.** `VaultLock::acquire` (vault.rs:120), the `recover_target`/`reap_tmp` loop (vault.rs:122–126), and the half-created guard (vault.rs:129–131) all run **before** decrypt; T2 is a fallback around vault.rs:133–136 and does not reorder them (subject to M1's helper-boundary caveat).
- **Scope = `btctax-store` only, no public-API break.** `open`/`save`/`snapshot` signatures unchanged; `SecretBuf` internal (memlock.rs). PATCH is defensible for an internal crate — **but only once C1 is fixed** (recovery on `UnsupportedSchema` is strictly *more* dangerous, so today the "strictly safer" rationale on line 65 is false).
- **FOLLOWUPS grounding checks out:** M-1 (FOLLOWUPS.md:1484 — note it scopes the trigger to *"decrypt/decode failure,"* supporting C1's exclusion of `UnsupportedSchema`), M-2 (FOLLOWUPS.md:1485), kill-mid-save (FOLLOWUPS.md:1490). The half-created-auto-repair half of the kill-mid-save followup is already shipped (`repair`, vault.rs:40–42), leaving the harness as the open item — matching T3.

## Required before R0-round-2 re-review
Fold C1 (exclude `UnsupportedSchema`; genuine-corruption-only trigger), C2 (a `.bak`-preserving crash-safe restore primitive; do **not** reuse `atomic_write` verbatim), I1 (re-enumerate T1 buffers), I2 (recovery warning/flag), I3 (complete the T3 cross-product to cover corrupt-`.bak` + good-`.tmp`). Persist this file verbatim before folding. Re-review after the fold — including whether the C2 fix reopens any lock/`.tmp` ordering question.
