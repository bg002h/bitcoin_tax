# R0 Architect Review (mandatory gate) — `vault-half-created-autorepair` (Slug 4), Round 1

- **Artifact reviewed:** `design/SPEC_vault_half_created_repair.md`
- **Source verified against:** working tree of `crates/btctax-store/src/{vault,atomic,paths,lib}.rs`, `crates/btctax-cli/src/{session.rs,lib.rs,main.rs,cmd/init.rs}`, plus `crates/btctax-store/tests/integration.rs` and all `crates/btctax-cli/tests/*.rs`.
- **Reviewer role:** independent architect (author ≠ reviewer).
- **Date:** 2026-06-29.

## Verdict

**NOT GREEN.** 0 Critical, **1 Important**, 2 Minor, 3 Nit.

The **safety core is correct** — the design provably never deletes `vault.key` when `vault.pgp` or `vault.pgp.bak` exists (verified by trace below). The blocking issue is a **scope/completeness defect**: the `cmd::init::run` signature change breaks ~26 existing call sites, of which the plan enumerates exactly one. The validation gate (`cargo test -p btctax-cli`) will not compile until all are fixed. Fix I1, then this is shippable.

---

## 1. Safety correctness (highest priority) — PASS

The load-bearing invariant ("never delete `vault.key` when `vault.pgp` OR `vault.pgp.bak` exists; the only repair-eligible state is `kp.exists() && !vault.exists() && !bak_of(vault).exists()`") is **correctly enforced**. Traced against the proposed `create_inner`:

```
let lock = VaultLock::acquire(vault)?;          // lock FIRST (preserved)
let kp = paths::suffixed_key(vault);
if vault.exists() || paths::bak_of(vault).exists() {   // GUARD A
    return Err(StoreError::AlreadyExists);              // <-- only exit when pgp/bak present
}
if kp.exists() {
    if repair { remove_file(&kp); remove_file(tmp_of(&kp)); remove_file(tmp_of(vault)); }
    else { return Err(StoreError::HalfCreatedVault(kp)); }
}
```

**(a) AlreadyExists fires before any `remove_file`, even under `repair=true`.** The single deletion site (the `repair` branch) is reachable ONLY after GUARD A did not fire, i.e. `!vault.exists() && !bak_of(vault).exists()` both hold. At deletion time there is provably no ciphertext (live or `.bak`-recoverable) that the key could decrypt. Verified traces:
  - Healthy vault (`vault.pgp` present): GUARD A first disjunct true → `AlreadyExists`, key untouched. ✔
  - `.bak`-recoverable (`vault.pgp` renamed to `vault.pgp.bak`; pgp absent, bak present, key present): GUARD A second disjunct true → `AlreadyExists`, key untouched. ✔
  - True half-created (key present, pgp absent, bak absent): GUARD A false → repair deletes ONLY `vault.key` (+ stray `.tmp` sidecars) → rebuilds. ✔

**(b) `open` half-created guard is placed AFTER `recover_target`/`reap_tmp`.** Confirmed against current `open` (`vault.rs:84-88` loop, then cert read at `:89`). The proposed guard sits between `:88` and `:89`:
```
if !vault.exists() && kp.exists() { return Err(StoreError::HalfCreatedVault(kp)); }
```
For the `.bak`-recoverable case, the `for f in [vault, kp]` loop runs `recover_target(vault)` first: `atomic.rs:35-43` restores `vault.pgp` from `vault.pgp.bak`, so `vault.exists()` is **true** by the time the new guard runs → guard does NOT fire → open proceeds and round-trips. Mis-flagging a recoverable vault is therefore impossible. ✔

**(c) Interleaving / TOCTOU / path edges — no realized hazard.**
  - **Lock ordering:** `VaultLock::acquire(vault)` is taken first (preserved); all check+delete steps run inside the lock. A second `btctax` process is blocked (`lock.rs` flock, `Err → Locked`). The lock file is `vault.pgp.lock` — distinct from every path the guards test, so acquiring it does not perturb any existence check.
  - **In-process check→delete window:** the existence checks (GUARD A) and `remove_file(kp)` are separate syscalls, but no btctax code runs between them and other instances are locked out. A *non*-btctax actor dropping a `vault.pgp` into that microsecond window is outside the threat model and is **not a regression** — the pre-existing `create` already does check-then-write under the same lock. (See Nit-3.)
  - **`.tmp` present:** a stray `vault.pgp.tmp`/`vault.key.tmp` does not block create — `fsperms::open_owner_only` uses `.create(true).truncate(true)` (NOT `O_EXCL`/`create_new`; verified `fsperms.rs:21-39`), so `atomic_write` overwrites any stray `.tmp`. The repair's explicit `remove_file(tmp_of(..))` is therefore belt-and-suspenders, not load-bearing. In `open`, a stray `vault.pgp.tmp` with `vault.pgp` absent is left alone by `reap_tmp` (`atomic.rs:49` requires `target.exists()`), and the new guard returns `HalfCreatedVault` rather than mis-using it. ✔
  - **`.key` path edge:** the `.key`-extension guard (`vault.rs:26-28`) is retained at the top of `create_inner` *before* `suffixed_key(vault)`, so `suffixed_key`'s `debug_assert_ne` (`paths.rs:26`) cannot fire and a `*.key` vault never collides with its own key file. ✔

**(d) `recover_target` does restore `vault.pgp` from `.bak` before the new `open` guard** — confirmed (see (b)). The guard triggers only on the truly-unrecoverable case (pgp absent AND bak absent AND key present). ✔

**Conclusion:** No path deletes a key that a present-or-recoverable vault needs. The safety condition is sound. (No Critical.)

---

## 2. Correctness vs real source — PASS (with one note rolled into §3)

- **`create_inner` refactor preserves invariants.** The `.key`-extension guard, `mkdir_owner_only(parent)`, lock-first ordering, the `cleanup` closure (`vault.rs:40-49`), the `built` closure, `atomic_write(&kp)` then `save()`, and `AlreadyExists` semantics for a healthy vault are all retained. The only semantic change for plain `create` (`repair=false`) is: the exact half-created signature now yields `HalfCreatedVault` instead of `AlreadyExists`. This does **not** break `create_refuses_existing` (`integration.rs:53-65`), which creates **and saves** a real vault (`vault.pgp` present) → GUARD A → `AlreadyExists`. ✔
- **`open` reads `kp` (cert) before `vault` — plan's claim is accurate.** Current `open` reads the cert at `vault.rs:89` then `vault` at `:90`. On a half-created vault the orphan key is a valid serialized TSK (`atomic_write(&kp, cert.as_tsk()…)`), so `Cert::from_bytes` succeeds and `std::fs::read(vault)?` at `:90` fails with `Io(NotFound)` — exactly the confusing dead-end the spec describes (`SPEC` line 13). The new guard pre-empts at `:88½`, returning `HalfCreatedVault` before the cert is even read. Bonus: the A1 test `open_on_half_created_returns_half_created_error` writes a *garbage* key (`b"x"`); because the guard fires before `Cert::from_bytes`, the test passes regardless of key validity — guard placement is correct and necessary for that test. ✔
- **`bak_of(vault)` is the right path and is truly absent on a first-create crash.** `bak_of` appends `.bak` → `vault.pgp.bak` (`paths.rs:13-15`). `atomic_write` writes a `.bak` ONLY `if target.exists()` (`atomic.rs:16-23`); on the first `save()` `vault.pgp` never pre-existed, so no `.bak` is produced. Confirmed: bak is absent in the genuine half-created state. ✔

---

## 3. Test completeness — adequate on safety; two gaps

**The safety refusals are proven.** `repair_refuses_to_clobber_healthy_vault` and `repair_refuses_when_bak_present` both assert `AlreadyExists` + key still present + vault still openable (the latter via `.bak` recovery). The `.bak` construction (`Vault::create` then `fs::rename(vault.pgp → bak_of(vault))`) is valid precisely because store-level `Vault::create` calls `save()` exactly once and thus leaves NO `.bak` (so the test's manual rename is the only `.bak`). ✔

Gaps (Minor):
- **M1 — no test that `repair` on a totally-clean path behaves as plain create.** `Vault::repair` on an empty dir (no key/pgp/bak) should `Ok` (kp absent → skip removal → build fresh). The implementation handles it, but a user defensively passing `--repair` to a first-time init exercises an untested branch. The review brief explicitly calls this out ("repair on a totally-clean path = plain create").
- **M2 — no test for the orphan-`.tmp` case.** The brief lists "orphan `.tmp` present". Add a case: orphan `vault.key` + stray `vault.pgp.tmp` present → `repair` succeeds and the stray `.tmp` is gone (or at minimum does not wedge create); and `open` on the same state returns `HalfCreatedVault` (not a mis-use of the stray `.tmp`).

Both are Minor: behavior is correct by construction; only coverage is missing.

---

## 4. API / SemVer / scope — one Important defect

- **No exhaustive match on `StoreError` anywhere.** Verified: every consumer is either `#[from]`-wrapped (`CliError::Store`, `lib.rs:17`) or a `matches!(…)` test assertion (`lock.rs:44`, `crypto.rs:184`, `blob.rs`, `integration.rs`, `cmd/init.rs:40`). There is **no** `match err { … }` requiring exhaustiveness. Adding `HalfCreatedVault` therefore compiles cleanly and its `Display` propagates via `#[error(transparent)]` → `eprintln!("error: {e}")` in `main.rs`. ✔
- **`Session::create` signature is unchanged**, so its three call sites (`session.rs:101,114,128`) are unaffected; the refactor to `from_fresh_vault` is behavior-preserving. ✔

- **[IMPORTANT] I1 — the `cmd::init::run` signature change breaks ~26 call sites; the plan names one.** The spec changes `run(vault_path, pp, key_backup_path)` → `run(vault_path, pp, key_backup_path, repair)` (a required 4th positional). Every existing 3-arg caller fails to compile. Actual blast radius (verified):
  - In-module unit tests (`cmd/init.rs`): lines **25, 36, 37** (3 calls) — the plan mentions ONLY `init_refuses_to_clobber_an_existing_vault` (lines 36/37); it omits `init_creates_vault_key_and_forced_backup` (line 25).
  - Integration tests (23 calls): `end_to_end.rs:18`; `verify_report.rs:14,36,59,91,194,280`; `reconcile.rs:29,42,199,315,346,384,414,450,484,533,590`; `export.rs:15,125`; `init_import.rs:13`; `fr9_exit_code.rs:77,96`.

  The validation gate `cargo test -p btctax-cli` compiles all `tests/*.rs`, so the suite will **not compile / not be green** until all ~26 sites are updated. The plan's "one reviewable change" + "Update `init_refuses…`" understates the work.

  **Fix (pick one):**
  - **(a) Recommended — avoid the churn.** Keep `pub fn run(vault_path, pp, key_backup_path)` as a thin wrapper delegating `run_with_repair(.., false)`, and add `pub fn run_with_repair(.., repair: bool)` that `main.rs` calls. Net: zero edits to the 23 integration sites + 1 in-module site; only the two new tests are added. This aligns with the workflow's preference for a minimal, well-scoped diff.
  - **(b) If keeping the 4-arg signature**, enumerate ALL ~26 call sites in Task B (append `, false`) — including `cmd/init.rs:25` and every integration file above — so the diff size and gate cost are stated accurately.

- **SemVer label.** "additive variant ⇒ PATCH (pre-1.0)" is acceptable here because the crate is workspace-internal with no external/exhaustive consumers. Strictly, adding a variant to a non-`#[non_exhaustive]` enum is a breaking change per Cargo SemVer; immaterial in this workspace. See Nit-1.

---

## 5. Gaps / over-engineering

- No over-engineering. The `remove_file(tmp_of(..))` pair in `repair` is harmless defensive cleanup (not load-bearing per §1(c)); keep it.
- No interactive prompt is correct per scope: `--repair` IS the explicit consent, and GUARD A makes a destructive misfire impossible.
- The out-of-scope decision (totally-absent vault `open` keeps returning `Io(NotFound)`) is consistent with the guard (`!vault.exists() && kp.exists()` is false when the key is also absent). ✔

---

## Findings summary

| # | Sev | Finding | Fix |
|---|-----|---------|-----|
| I1 | **Important** | `cmd::init::run` 4th-arg change breaks ~26 call sites; plan names 1 (and omits `cmd/init.rs:25`). Gate won't compile. | Wrapper `run` + `run_with_repair` (recommended), OR enumerate all ~26 sites in Task B. |
| M1 | Minor | No test: `repair` on a totally-clean path = plain create. | Add `repair_on_clean_path_is_plain_create`. |
| M2 | Minor | No test: orphan-`.tmp` present (create-refuses/`open`-guard/`repair`-cleans). | Add a `.tmp`-present case to A1. |
| N1 | Nit | Variant added to non-`#[non_exhaustive]` `StoreError` is strictly breaking per Cargo SemVer. | Optionally add `#[non_exhaustive]` to `StoreError` to make future additions truly additive; or note the exception. |
| N2 | Nit | `remove_file(tmp_of(..))` in `repair` is defensive-only (truncating open overwrites strays). | None — keep; maybe a one-line comment. |
| N3 | Nit | In-process check→delete TOCTOU window exists (mitigated by lock; not a regression). | None — document the lock as the protection in the safety-invariants section. |

## Re-review requirement
Author must persist this review verbatim (done), fold I1 (and ideally M1/M2), then **re-run R0** — including after the final fold — until 0 Critical / 0 Important. Do not proceed to implementation while I1 is open.

---

# Round 2 — fold re-review

- **Artifact re-reviewed:** `design/SPEC_vault_half_created_repair.md` (revised fold).
- **Source re-verified against:** `crates/btctax-cli/src/cmd/init.rs`, `crates/btctax-cli/src/main.rs`, `crates/btctax-cli/src/session.rs`, and a full grep of `cmd::init::run` call sites across `crates/btctax-cli/{src,tests}`.
- **Reviewer role:** independent architect (author ≠ reviewer).
- **Date:** 2026-06-29.

## Verdict (Round 2)

**GREEN.** 0 Critical, **0 Important**, 0 new Minor/Nit. The one Important (I1) is closed; both Minors (M1, M2) are folded; the safety core (confirmed correct in Round 1) is untouched; Nits N1/N2/N3 are reasonably handled. **R0 is cleared to implement.**

## 1. I1 — CLOSED (verified)

The plan adopts the recommended fix (a): it **keeps** the 3-arg `pub fn run(vault_path, pp, key_backup_path)` as a thin wrapper delegating to a new `pub fn run_with_repair(.., repair: bool)`, with `main.rs` calling the 4-arg form (SPEC lines 84-95, 106-107; Plan B2 line 132).

Blast radius re-counted against current source — every existing caller still calls the **3-arg** `run`, so all compile unchanged:
- In-module unit tests (`cmd/init.rs`): lines **25, 36, 37** — all 3-arg. The fold explicitly preserves `init_creates_vault_key_and_forced_backup` (the line-25 site the original plan omitted) and `init_refuses_to_clobber_an_existing_vault` (SPEC line 131). ✔
- Integration tests: **23** call sites, all `cmd::init::run(&vault, &pp(), &…)` (3-arg) — verified by grep across `end_to_end.rs:18`, `init_import.rs:13`, `fr9_exit_code.rs:77,96`, `verify_report.rs:14,36,59,91,194,280`, `export.rs:15,125`, `reconcile.rs:29,42,199,315,346,384,414,450,484,533,590`. None touched. ✔
- The **only** caller migrated to `run_with_repair` is `main.rs:184` (the dispatch) — exactly the intended single edit. ✔

Therefore the validation gate `cargo test -p btctax-cli` compiles (all `tests/*.rs` plus in-module tests), and **no existing test was forced to change signature.** I1 is fully resolved.

## 2. M1 + M2 — FOLDED (verified)

Task A test list now includes both brief-mandated cases:
- **M1 — `repair_on_clean_path_behaves_as_create`** (SPEC line 123): empty dir → `Vault::repair` → `Ok`; `open` round-trips. Exercises the `kp` absent → skip-removal → build-fresh branch. ✔
- **M2 — `repair_clears_orphan_tmp_sidecars`** (SPEC line 124): orphan key + orphan `tmp_of(vault)`/`tmp_of(kp)` → `repair` → `Ok`; asserts strays gone + `open` round-trips. ✔

(Name nit: M1's test is named `repair_on_clean_path_behaves_as_create` here vs the Round-1 table's suggested `repair_on_clean_path_is_plain_create`. Immaterial — same semantics; the brief requests this exact name.)

## 3. No new defect / safety intact (verified)

- **Store-side safety guard unchanged.** GUARD A (`vault.exists() || bak_of(vault).exists() → AlreadyExists`, before any `remove_file`, even under `repair=true`) is byte-for-byte the design Round 1 proved sound (SPEC lines 46-48). The `open` half-created guard remains placed **after** the `recover_target`/`reap_tmp` loop and before the cert read (SPEC lines 62-66), so a `.bak`-recoverable vault is restored first and never mis-flagged. Round-1 §1 trace still holds. ✔
- **`Session::repair`/`from_fresh_vault` factoring is behavior-preserving.** Confirmed against current `session.rs:26-32`: today's `create` is `Vault::create → init_schema → init_config_table → save → Ok`. The fold splits the post-`Vault` tail into `from_fresh_vault(vault)` (init_schema + init_config_table + re-save, verbatim) and has both `create` and `repair` construct the `Vault` first (SPEC lines 71-81). `create`'s observable behavior is identical; `repair` differs only in the constructor it calls. ✔
- **The wrapper change is CLI-layer only** — it does not reach into the store guards. `run` (3-arg) → `run_with_repair(.., false)` → `Session::create`; `main.rs` passes the parsed `repair` flag. The CLI test `init_without_repair_on_half_created_errors` (3-arg `run`, repair=false) correctly expects `Err(Store(HalfCreatedVault(_)))`, consistent with the delegation. No path was added that can delete a key a present-or-recoverable vault needs. ✔
- **Nits.** N1 (variant on non-`#[non_exhaustive]` enum) acknowledged with a documented exception, scope-appropriately deferred (SPEC line 138). N2/N3 (`.tmp` removal is defensive-only; `VaultLock` is the TOCTOU protection) folded as a code-comment note (SPEC line 139). All reasonable. ✔

## Findings (Round 2)

None. No Critical, no Important, no new Minor/Nit introduced by the fold.

## Re-review outcome

I1 closed; M1/M2 folded; safety core intact; nits handled. **0 Critical / 0 Important → R0 GREEN.** Cleared to proceed to implementation per the Task A → Task B TDD plan.
