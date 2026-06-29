# Whole-branch review — `btctax-cli` (round 1)

> **Round 2 — fold re-review (578e256..10dd6e2) — CLOSED, GREEN.** Independent re-review confirms
> **CLI-I1 CLOSED** (CSVs now opened via `fsperms::open_owner_only` → `csv::Writer::from_writer`,
> guaranteed 0o600 regardless of umask/pre-existing dir; `#[cfg(unix)]` test asserts 0o600 on a
> pre-existing out-dir — fails under the old `from_path`) and **CLI-I2 CLOSED** (`safe_harbor_status`
> checks the `BasisSource::SafeHarborAllocated` effective-Path-B signal before the `SafeHarborTimebar`
> advisory; ordering verified against the engine's continue-on-blocker semantics — unconservable/genuine
> time-bar still surface correctly; happy-path test sets up both the allocated lots AND the stale advisory
> then asserts "effective" / not "time-barred"). One NEW narrow **Minor** (status dark if ALL allocated
> lots consumed before verify) → FOLLOWUPS. No new float/determinism/borrow/error-model defect.
> **Net: 0 Critical / 0 Important — btctax-cli GREEN, ready to merge.**



**Scope:** final independent whole-branch review of `btctax-cli`, the CLI + reconciliation
crate that completes Phase-1. Diff reviewed: `.superpowers/sdd/review-a657453..578e256.diff`
(20 commits, 23 files). Contract: `design/SPEC_foundation.md` (FR1–FR10, NFR2/4/5/6/8,
TP1–TP11, §7.2/§7.4). Engine APIs cross-checked against `crates/btctax-core/src/`
(event.rs, identity.rs, persistence.rs, state.rs, project/{resolve,conservation}.rs) and
`crates/btctax-store/src/{vault,fsperms}.rs`. Full workspace gate is GREEN (per the
ledger); I did not re-run it — this is a code/diff review.

**Verdict: NOT ready to merge — 0 Critical / 2 Important.**
The command→event correctness, TP8-(c) protection, FR9 exit-code wiring, FR10
engine-only export values, NFR4/NFR5 (no float, no CLI tax math), and the
lib/bin/seam split are all sound. Two cross-cutting defects block:

- **CLI-I1 (privacy / NFR2):** the FR10 CSV export writes the decrypted tax ledger with
  default (world-readable) file permissions — the store crate's deliberately-hardened
  `snapshot.sqlite` (0o600) and the CLI's CSV halves of the *same* export diverge. NEW —
  not recorded anywhere.
- **CLI-I2 (FR9 output correctness):** `verify`'s 2025-transition status line affirmatively
  mislabels an **effective Path B** as "time-barred → using Path A" on the documented
  attest happy path (and on plain `allocate; allocate --attest`). Recorded in FOLLOWUPS as
  "display polish"; I am escalating it.

Both fixes are small. Neither produces a wrong *dollar* amount or a wrong exit code.

---

## What was verified clean (the cross-cutting checks)

1. **Command→event correctness, end-to-end.** Every reconcile emitter builds exactly one
   correct `EventPayload` with correct fields and appends via `append_decision` (signature
   matches `persistence.rs`):
   - `link_transfer` → `TransferLink{out_event, in_event_or_wallet}`; `--to-event`/`--to-wallet`
     mutually exclusive (clap `conflicts_with` + dispatch guard).
   - `classify_inbound` → `ClassifyInbound` with `InboundClass::{Income,GiftReceived}` passed
     through untouched — this is the §9.1 Swan-`deposit` transfer-basis-gap re-supply path.
   - `reclassify_outflow` → `ReclassifyOutflow{principal, fee_usd}`; the on-chain `fee_sat`
     is **not** touched by the CLI — it rides from the original `TransferOut.fee_sat` and is
     routed through TP8 in `resolve::build_op` (the core whole-branch I-1 fix). TP2 fee-reduces-
     proceeds for Dispose; TP8/§7.3 fee for Gift/Donate. Correct.
   - `classify_raw` → `ClassifyRaw` with an `is_imported()` guard (rejects decision payloads
     with a distinct message, proven by a serialized-real-decision test; malformed JSON → distinct
     parse error).
   - `accept_conflict`/`reject_conflict` → `SupersedeImport`/`RejectImport`; conflict `EventId`
     round-trips through `eventref::parse_event_id` and matches `by_id` in resolve.
   - `set_fmv`/`void` → `ManualFmv`/`VoidDecisionEvent`. Correct.
   - safe-harbor allocate/attest: see CLI-I2 below for the one defect; the *mechanics* (pre-2025
     residue re-projection = `universal_snapshot`, voided-prior exclusion, already-effective
     rejection, conservation backstop) are correct.

2. **TP8 default (c) is never implicitly flippable to (b).** Protected at three layers:
   `ProjectionConfig::default` (core) = TreatmentC; `CliConfig::default` (config.rs) hard-codes
   TreatmentC with a "DO NOT change" guard (no delegation); `read_config` returns (c) for any
   unset key and **errors** (`BadConfigValue`) on an unrecognized stored value (no silent (b)).
   `set_fee_treatment`/`config --set-fee-treatment b` is the only path to (b) and is explicit
   opt-in. Every projection routes through `Session::config()?.to_projection()`. Solid.

3. **Security/privacy boundary.** No plaintext DB write except `export-snapshot` (the NFR2
   exception) — except for the **file-permission** gap in CLI-I1. Passphrase is never hardcoded
   or logged (`BTCTAX_PASSPHRASE` env seam else `rpassword` prompt; `init` uses confirm=true;
   error strings never include the passphrase). Tests use only `tempfile::tempdir()` + synthetic
   §9.1-header fixtures — no real-file reads, no PII (`fixtures.rs` documents this). The
   `&Passphrase` and `now` test seams are preserved; production resolution lives only in `main.rs`
   (`OffsetDateTime::now_utc()` / passphrase()).

4. **FR9 verify + exit code.** `build_verify` partitions blockers by `BlockerKind::severity()`
   (engine, not hardcoded); `has_hard_blockers()` = non-empty hard list; `main.rs` maps it to
   `ExitCode::from(1)`, any `CliError` → 2, clean → 0. Binary-level regression tests
   (`fr9_exit_code.rs`) assert the real process codes 0 and 1 and fail-if-the-mapping-is-removed.

5. **FR10 export values are engine-computed.** `write_csv_exports` and `render_*` only
   stringify engine fields (`Decimal::to_string`, `i64::to_string`); `conservation_report` is
   the engine's. No CLI tax arithmetic anywhere; no float money (`parse_usd_arg` =
   `Decimal::from_str`). NFR5 holds.

6. **Determinism (NFR4).** `project()` is fed `load_all` straight (no CLI sort/dedup/cache).
   Rendered/CSV output iterates `holdings_by_wallet` (BTreeMap) and the engine's canonically-
   ordered Vecs; output is deterministic. Decision clock is the injected `now`.

7. **Lib/bin split & borrow discipline.** `main.rs` is a thin clap dispatch with no business
   logic; all commands are library fns over `(vault_path, &Passphrase, …, now)` returning
   structured outcomes. `conn()` (`&`) / `save()` (`&mut`) borrows never overlap (incl. the two
   sequential `append_decision` calls + `save` in `safe_harbor_attest`). `CliError` is coherent
   (incl. the `Csv(#[from] csv::Error)` variant that `Io(#[from] io::Error)` cannot cover).

8. **Plan-vs-spec command surface.** Spec §11's `wallets`/`holdings`/`lots`/`events`/`fmv`/
   `reconstruct-2025` are absent as top-level commands, but this matches the **green** Plan-4
   command surface (read commands consolidated into `report`/`show`; `fmv` → `reconcile set-fmv`;
   `allocate-2025` → `reconcile safe-harbor allocate`; Path A is the no-event default). This is
   deliberate, already-reviewed re-scoping — NOT drift. (One residual: FR5 "label self-custody
   wallets" has no dedicated command — a self-custody wallet only comes into existence by being
   referenced in a `TransferLink --to-wallet self:LABEL`. Functional for Phase-1; noted as an
   observation, not a finding.)

---

## Important findings (block merge)

### CLI-I1 — FR10 CSV export writes decrypted tax PII without owner-only file permissions (NFR2)

**Files:** `crates/btctax-cli/src/render.rs` (`write_csv_exports`),
`crates/btctax-cli/src/cmd/admin.rs` (`export_snapshot`).

`write_csv_exports` creates the output directory with `std::fs::create_dir_all(out_dir)`
(default perms) and writes `lots.csv`, `disposals.csv`, `removals.csv`, `income.csv` via
`csv::Writer::from_path(...)` — i.e. default file permissions (0o644 minus umask). Those CSVs
contain the **fully decrypted ledger**: per-lot `usd_basis`, realized `gain`, wallet labels,
acquisition dates — the same class of PII the store crate's security review (Task 8 SECURITY,
the "MEDIUM: export_snapshot writes PLAINTEXT tax PII with default perms" item) deliberately
hardened: `Vault::export_snapshot` now writes `snapshot.sqlite` **0o600** inside a **0o700**
directory via `fsperms::{mkdir_owner_only, write_owner_only}`.

The CLI's CSV half of the very same export does not honor that posture. Concretely, in
`admin::export_snapshot` the store call runs first (so a *freshly created* `out_dir` is 0o700,
and the 0o644 CSVs inside it are protected by directory-traversal perms) — **but**
`mkdir_owner_only` is a no-op on an **already-existing** directory (it uses
`DirBuilder::recursive(true).mode(0o700).create()`, which only applies the mode to dirs it
creates). So the realistic flow `mkdir -p ~/exports && btctax export-snapshot --out ~/exports`
(or any re-export into a user-made directory) lands the four CSVs world-readable (0o644 in a
0o755 dir), while the sibling `snapshot.sqlite` stays 0o600. This is precisely the asymmetry
the store's file-level hardening exists to prevent, re-opened by the second plaintext export
path the CLI added.

This is the exact class of cross-task gap a per-task gate cannot see (it's "store hardened X;
CLI later added a parallel X′ that bypasses the hardening"), and it mirrors the store
whole-branch I-1 (primary-vault world-readable) that the prior review caught.

**Fix.** Route the CSV writers through `btctax_store::fsperms` (both `pub`):
create the dir with `mkdir_owner_only(out_dir)` and open each CSV via
`csv::Writer::from_writer(btctax_store::fsperms::open_owner_only(&path)?)` (0o600 on Unix,
ACL-inherited on non-Unix — keeps NFR8 portability). That makes the CSV files self-protecting
regardless of pre-existing directory perms, matching `snapshot.sqlite`. Add a Unix perms
assertion (mirroring the store's export tests).

### CLI-I2 — `verify` mislabels an effective Path B safe-harbor as "time-barred → Path A"

**File:** `crates/btctax-cli/src/render.rs` (`safe_harbor_status`).

`safe_harbor_status` decides the 2025-transition line purely from raw blocker *presence*:
`unconservable` → fails-conservation; else `timebar` (any `SafeHarborTimebar` blocker) → "Path B
time-barred → using Path A (advisory); `reconcile safe-harbor attest` if timely"; else
`has_alloc` → "Path B … effective"; else Path A.

But `resolve.rs` routes a `VoidDecisionEvent` targeting a `SafeHarborAllocation` into
`allocation_voids` (NOT the `voided` set), so a **voided/inert** prior allocation is **still
evaluated** in pass-1 step 3 and **still emits its advisory `SafeHarborTimebar`** (the
recorded core Task-12 Minor — but its "Path A either way" rationale is false here). The CLI's
own `safe_harbor_attest` deliberately produces exactly this state: it appends `Void(prior) +
re-attested copy`, so after a successful attest there is an **effective Path B** allocation
*coexisting* with one or more stale `SafeHarborTimebar` blockers from the voided prior(s).
Because the `timebar` branch precedes the effective-Path-B branch, `verify` then prints
"Path B time-barred → using Path A … attest if timely" — the opposite of reality (the engine
is on Path B; `state.lots` carry `BasisSource::SafeHarborAllocated`), and it tells the user to
attest *immediately after they just successfully attested*.

Reachability is broader than the attest cycle: `safe_harbor_allocate` has **no guard** against
appending a second live allocation, so the plain flow `allocate-2025` (unattested → timebarred,
inert) then `allocate-2025 --attest` (effective) leaves the first allocation's stale timebar +
an effective second → same mislabel, with no void/attest involved.

This is FR9's *integrity* command emitting an affirmatively-wrong statement about which
basis-transition governs, on the feature's documented happy path. No dollar amount or exit
code is wrong (the timebar is advisory; lots/disposals are correct), so it is display-only — and
it *is* already recorded in `FOLLOWUPS.md` ("attest leaves a stale `safe_harbor_timebar`
advisory … Display-only … OPEN (display polish)"). I am escalating it from "polish" to Important:
for a tax tool, `verify` is the trust surface, the message is actively misleading on the happy
path, and the fix is ~3 lines. (A reasonable reviewer could keep this as Minor; CLI-I1 blocks
the merge regardless, so the verdict is unchanged either way.)

**Fix (CLI-side, cheapest).** In `safe_harbor_status`, derive Path-B-effective from the
projection itself and let it take precedence over the advisory timebar, e.g. check
`state.lots.iter().any(|l| l.basis_source == BasisSource::SafeHarborAllocated)` and report
"Path B effective" before the `timebar` branch. (Engine-side alternative noted in FOLLOWUPS:
suppress re-evaluation of an allocation an attestation supersedes; or have `resolve` drop the
stale advisory once another allocation is effective.) Add a test that asserts the rendered
status after the attest happy path is the effective-Path-B string, not the timebar string.

---

## Recorded CLI Minor triage (from `progress.md`, Tasks 1–16)

All recorded CLI Minors are **DEFER** — none individually blocks merge:

| Task | Minor | Disposition |
|---|---|---|
| 1 | M1 silent config fallback; M2 no `CREATE TABLE IF NOT EXISTS` | **DEFER** — already FIXED in Task 2 (`BadConfigValue` + ensure-table). Verified in `config.rs`. |
| 3 | no rollback if `backup_key` fails after vault created → stranded vault | **DEFER** — recoverability papercut on the user's own path; non-security, non-correctness. FOLLOWUP (`--recover`). |
| 4 | `render_file_reports` no unit test | **DEFER** — exercised via integration (`init_import`). |
| 5 | disposals/removals/income rely on engine fold order vs explicit renderer sort | **DEFER** — engine fold order is canonical/deterministic → output deterministic (NFR4 holds). |
| 6 | tighten KAT to assert `hard[].kind==UnknownBasisInbound` | **DEFER** — `verify_report.rs` now asserts `unknown_basis_inbounds > 0`; full kind-pin is test polish. |
| 8 | KAT doesn't re-assert non-taxable/basis-carry; `--to-event` path untested | **DEFER** — core KATs cover the tax math; CLI coverage gap only. |
| 9 | `GiftReceived` path no CLI integration test | **DEFER** — TP11 fully core-covered; the CLI passes `InboundClass` through untouched (verified). |
| 10 | Sell gain unasserted; Spend untested; GiftOut zero-gain comment aspirational | **DEFER** — core KATs cover values; CLI proves the disposal/removal *shape*. |
| 11 | set-fmv KAT targets `Acquire`, doesn't prove a blocker clears | **DEFER** — FOLLOWUP N-1. See Nit N3 (related: ManualFmv silently no-ops on non-Income targets). |
| 13 | "already attested" shortcut before re-projection | **DEFER** — safe; an attested-but-unconservable case still can't be cured by attest. |
| 15 | Debug-format enums in CSV (schema-fragile) | **DEFER** — FOLLOWUP eng-M2; acceptable for Phase-1 human-readable export; harden before any downstream CSV consumer. |

---

## Nits (non-blocking)

- **N1.** `verify`'s non-zero exit keys only on hard blockers, not on `!conservation.balanced`.
  The two are coupled by the engine (drift ⇒ `UncoveredDisposal`, which is hard), so this is
  safe today; a defensive `|| !report.conservation.balanced` would make `verify` fail closed if
  that invariant ever regressed.
- **N2.** `safe_harbor_allocate` has no guard against appending a second live allocation
  (only `attest` checks). It also seeds `--method pro-rata` from the actual per-wallet residue
  (a true cross-wallet pro-rata redistribution is the documented O4 refinement) — and unattested
  ProRata is always time-barred by the engine, so `allocate-2025 --method pro-rata` alone is
  always inert. Both are documented; mainly contributes to CLI-I2's reachability.
- **N3.** `set-fmv` (`ManualFmv`) silently no-ops on a non-`Income` target (resolve only applies
  `ManualFmv` in the `Income` arm); the CLI does not validate the target type, so a user can
  "set" an FMV on an `Acquire`/`TransferOut` with no effect and no diagnostic. Consider a
  target-type check (or document it). (Overlaps FOLLOWUP N-1.)
- **N4 (observation, already tracked).** `AllocLot` carries no `dual_loss_basis`/`donor_acquired_at`,
  so a pre-2025 received-gift lot loses its TP11 dual basis when re-seeded via Path B. Spec-faithful
  (the `AllocLot` type has no such field) and Path A (default) preserves it; FOLLOWUP M-2 (Phase-2,
  spec change). Not introduced by this branch.

---

## Bottom line

`btctax-cli` is well-built and faithful to the contract on the dimensions that matter most
(command→event correctness, TP8-(c) protection, FR9/FR10, NFR4/5, seams). It is **not yet
mergeable at 0 Critical / 0 Important**:

- **Must fix:** **CLI-I1** (CSV export owner-only perms — privacy/NFR2; route through
  `btctax_store::fsperms`).
- **Should fix (escalated from the recorded "display polish"):** **CLI-I2** (`verify`
  mislabels effective Path B as time-barred Path A on the attest/allocate-twice happy path).

Re-review after the fold (including a re-run of the workspace gate) per the standard workflow.
