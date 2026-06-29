# Cycle-Prep Recon — bitcoin_tax (TaxApp)

**Origin SHA:** `428f457`  
**Branch:** `main`  
**Sync:** up-to-date with `origin/main`  
**Untracked:** (none) — working tree clean  
**Date:** 2026-06-29  
**Scope:** Four FOLLOWUP slugs from `FOLLOWUPS.md`; read-only — no source files modified.

---

## Slug 1 — `reconcile-allocation-dual-loss-basis`

**WHAT (1-2 lines):** `AllocLot` carries only a single `usd_basis` field; when a pre-2025 received-gift lot (which holds a `dual_loss_basis` on the `Lot` in-engine) is re-seeded via Path B, the loss-leg basis collapses to the single `usd_basis`. Path A preserves dual basis correctly.

### Citation verification

| Citation | Asserted | Verdict | Evidence |
|---|---|---|---|
| `AllocLot` fields in `event.rs` | `{ wallet, sat, usd_basis, acquired_at }` — single-basis, no `dual_loss_basis` | **ACCURATE** | `event.rs:145-150`: `pub struct AllocLot { pub wallet: WalletId, pub sat: Sat, pub usd_basis: Usd, pub acquired_at: TaxDate, }` — exactly four fields, no `dual_loss_basis` or `donor_acquired_at` |
| `SafeHarborAllocation.lots` element type | `Vec<AllocLot>` | **ACCURATE** | `event.rs:152-153`: `pub lots: Vec<AllocLot>,` |
| Path-B re-seed in `seed_transition` (`transition.rs`) | Seeds fresh lots from `seed`; discards Universal remainder | **ACCURATE** | `transition.rs:67-81`: `TransitionMode::PathB { seed }` branch — iterates `seed.iter().cloned()`, calls `pools.push_lot`; the `seed` lot objects come from `resolve.rs` (below) |
| Seed lots in `resolve.rs` have no dual basis | `dual_loss_basis: None`, `donor_acquired_at: None` set on all Path-B seed lots | **ACCURATE** | `resolve.rs:570-584`: `Lot { ... usd_basis: l.usd_basis, basis_source: BasisSource::SafeHarborAllocated, dual_loss_basis: None, donor_acquired_at: None, basis_pending: false, }` — confirms collapse |
| `Lot.dual_loss_basis` / `donor_acquired_at` in `state.rs` | Carried as `Option<Usd>` / `Option<TaxDate>` | **ACCURATE** | `state.rs:66-67`: `pub dual_loss_basis: Option<Usd>,` / `pub donor_acquired_at: Option<TaxDate>,` |
| §1015(a) four-zone logic in `make_disposal_legs` (`fold.rs`) | `dual → {gain-zone/loss-zone/NoGainNoLoss/non-dual}` | **ACCURATE** | `fold.rs:80-109`: `if c.dual { let loss_basis = c.loss_basis.expect(...); if proceeds > c.gain_basis { ... } else if proceeds < loss_basis { ... } else { ... } } else { ... }` — four-zone implemented |
| Path A preserves dual basis | Universal lot fields copied unchanged to per-wallet pool | **ACCURATE** | `transition.rs:59-65`: `TransitionMode::PathA` arm iterates Universal lots, sets `basis_source = ReconstructedPerWallet`, calls `pools.push_lot(key, lot)` — existing `lot.dual_loss_basis` / `lot.donor_acquired_at` carried unchanged |
| CLI `safe_harbor_allocate` build site (`reconcile.rs`) | Builds `AllocLot` from residue; copies `usd_basis` but not `dual_loss_basis` | **ACCURATE** | `reconcile.rs:233-239`: `AllocLot { wallet: l.wallet.clone(), sat: l.remaining_sat, usd_basis: l.usd_basis, acquired_at: l.acquired_at, }` — no `dual_loss_basis` mapping; residue `l.dual_loss_basis` is silently dropped at the CLI build site |
| Claim: Path A preserves / Path B collapses dual basis | Stated as the asymmetry motivating the fix | **ACCURATE** | Follows from the two verifications above: Path A carries existing `Lot` fields; Path B seeds fresh lots with `dual_loss_basis: None` |

### Numeric / factual claims

No numeric thresholds or line numbers were specified in the FOLLOWUP; all structural claims verified above.

### Action for brainstorm spec (source SHA `428f457`)

All citations are accurate; the gap is real and unambiguous. The brainstorm/spec must design:
1. Extending `AllocLot` with `dual_loss_basis: Option<Usd>` and `donor_acquired_at: Option<TaxDate>` (spec change — currently `SafeHarborAllocation` in spec §6.4 lists `{wallet, sat, usd_basis, acquired_at}`).
2. Updating `safe_harbor_allocate` in `reconcile.rs` to copy `l.dual_loss_basis` and `l.donor_acquired_at` from the residue lot.
3. Updating `seed_transition`'s Path-B arm (or the `resolve.rs` seed builder) to propagate those fields onto the seeded `Lot`.
4. Noting the taxpayer-impact: this affects only Path-B taxpayers who held a received gift pre-2025 with `FMV-at-gift < donor-basis`. Path A is unaffected; the fix cannot regress Path A.

---

## Slug 2 — `appraisal-trigger-precision`

**WHAT (1-2 lines):** The precise §170(f)(11)(C) qualified-appraisal trigger is *claimed deduction > $5,000* (aggregating similar items per §170(f)(11)(F)), not FMV > $5,000. For §170(e)-reduced property (≤1-yr / ordinary-income asset) the deduction is capped at basis, so high-FMV short-term donations may not trigger an appraisal even when FMV > $5k.

### Citation verification

| Citation | Asserted | Verdict | Evidence |
|---|---|---|---|
| `OutflowClass::Donate { appraisal_required: bool }` in `event.rs` | Field exists as `bool` | **ACCURATE** | `event.rs:107`: `Donate { appraisal_required: bool },` |
| `appraisal_required` on `Removal` in `state.rs` | `pub appraisal_required: bool` | **ACCURATE** | `state.rs:122`: `pub appraisal_required: bool, // donation (>$5k FMV over-flag, FOLLOWUPS)` |
| CLI `reclassify_outflow` Donate path | Builds `OutflowClass::Donate { appraisal_required }` | **ACCURATE** | `reconcile.rs:54-72`: `reclassify_outflow` takes `class: OutflowClass` and passes it through to `ReclassifyOutflow`; the `appraisal_required` value comes from the caller |
| "Phase 1 flags `Donate.appraisal_required` on FMV > $5k (safe over-flag)" — implied auto-computation | An automatic FMV-vs-$5k threshold fires in core or CLI to set `appraisal_required = true` | **STRUCTURALLY WRONG** | No such auto-computation exists anywhere in the codebase. `appraisal_required` is a raw user-supplied CLI boolean: `main.rs:101` declares `appraisal: bool` as a CLI arg; `main.rs:307` sets `appraisal_required: appraisal` from that arg. There is no `if fmv > 5000 { appraisal_required = true }` code in core, CLI, or adapters. The user must set `--appraisal` manually. |
| Spec pointer "§16 + tax-review wording on the appraisal trigger" | §16 discusses appraisal trigger | **STRUCTURALLY WRONG** | Spec §16 ("Suggested implementation order", spec lines 239-244) is purely the phase-ordering list and contains no reference to appraisal trigger. The correct spec citations are: TP10 (`Donate { …, appraisal_required: bool }` for Phase-2); spec fold-record R3 tag "TAX-N2 appraisal trigger precision → FOLLOWUPS" (spec line 251); and the FOLLOWUPS entry itself under "Deferred — precise Phase-2 tax refinements" (`tax-review R3 N2`). |
| Current FMV > $5k threshold location + value | A $5k constant or comparison exists in source | **STRUCTURALLY WRONG** | No `5000`, `5_000`, `5k`, `fmv_threshold`, or any dollar-amount constant tied to appraisal exists in any `.rs` file. The "FMV > $5k" description in the FOLLOWUP characterizes a *design intent* that was never implemented; the field is purely manual. |

### Numeric / factual claims

- §170(f)(11)(C) trigger = "claimed deduction > $5,000" — tax-law claim verifiable against `design/SPEC_foundation.md` TP10 and the archived legal materials; consistent with what the FOLLOWUP cites (no contradiction in-repo).
- §170(f)(11)(F) aggregation — same; not contradicted by in-repo sources.
- §170(e)-reduced property / deduction capped at basis — consistent with spec TP10 and fold-record TAX-N2 note; not contradicted.

### Action for brainstorm spec (source SHA `428f457`)

Two corrections required before this enters a brainstorm:

1. **Fix the "Phase 1 flags on FMV>$5k" assertion.** The current Phase 1 reality is that `appraisal_required` is a *fully manual user-supplied flag* — there is no automatic threshold. The brainstorm should decide whether Phase 1 should gain an auto-flag (FMV>$5k as a safe over-flag) or remain manual; either way the FOLLOWUP as written mis-describes the current state.
2. **Fix the spec pointer.** "§16" should be replaced with "TP10 + spec fold-record R3/TAX-N2 + FOLLOWUPS § deferred-precise-Phase-2-tax-refinements". The appraisal trigger discussion is entirely in the tax-review (R3 N2) and FOLLOWUPS, not in spec §16.

The core Phase-2 refinement claim (deduction > $5k vs FMV > $5k, §170(f)(11)(F) aggregation, §170(e) basis cap) is tax-law accurate per in-repo sources; no tax-law correction needed.

---

## Slug 3 — `pre2025-filed-method-reconciliation`

**WHAT (1-2 lines):** Pre-2025 FIFO is the legal default. If the taxpayer's filed pre-2025 returns used a different method, the mismatch should be surfaced as a verify note/blocker rather than silently assumed. The FOLLOWUP status is "OPEN (runtime reconciliation) — spec §7.4, eng-review I-2."

### Citation verification

| Citation | Asserted | Verdict | Evidence |
|---|---|---|---|
| spec §7.4 exact wording — mandates a verify note/blocker | "deviation from the taxpayer's filed method → `pre2025_method_note`" | **ACCURATE** | Spec lines 137-138: "Pre-2025: **`UniversalPool`** tracking **lots (with dates), un-partitioned by wallet**; pre-2025 disposals consume **FIFO** (legal default; deviation from the taxpayer's filed method → `pre2025_method_note`)." Exact wording confirmed. |
| `BlockerKind::Pre2025MethodNote` does not yet exist (implied "must be surfaced") | Missing from `state.rs` | **DRIFTED / WRONG** | `state.rs:33`: `Pre2025MethodNote,` exists. `state.rs:46-47`: severity `Advisory` — `SafeHarborTimebar \| UnmatchedOutflows \| Pre2025MethodNote => Severity::Advisory`. The variant is present and has the correct severity. |
| No note/blocker emitter exists in the fold / transition code | The emission hasn't been implemented | **DRIFTED / WRONG** | `fold.rs:28-41`: `fn note_pre2025_once(st: &mut LedgerState, date: TaxDate, ev: &EventId)` — emits `BlockerKind::Pre2025MethodNote` with detail `"pre-2025 disposal consumed the Universal FIFO pool (§7.4)"` the first time a pre-2025 Dispose/GiftOut/Donate occurs. Called at `fold.rs:366` (Dispose arm), `fold.rs:744` (GiftOut arm), `fold.rs:810` (Donate arm). |
| FIFO is the only pre-2025 method assumed | No alternate-method path exists | **ACCURATE** | The Universal-pool fold uses `sort_canonical` (canonical FIFO order); no config or variant for LIFO/HIFO/specific-ID pre-2025 exists in `project/` or `resolve.rs`. |
| "OPEN (runtime reconciliation)" — deeper user mechanism | Not implemented | **ACCURATE** | There is no CLI flag, config option, or engine path allowing the user to specify a non-FIFO pre-2025 filing method and have the engine compute an adjusted carryforward. The advisory blocker fires (see above) but there is no remediation path. |

### Drift summary

The FOLLOWUP's framing implies the advisory blocker does not yet exist and "must be surfaced." In the current source @ 428f457 the blocker IS implemented and IS emitted. The FOLLOWUP status "OPEN" refers specifically to the *runtime reconciliation mechanism* — a way for users to declare their actual pre-2025 method and see an adjusted carryforward — which does NOT exist. The statement "surfaced as verify note/blocker" is fully satisfied; the word "reconciled" in "must be reconciled" is the open part.

### Action for brainstorm spec (source SHA `428f457`)

1. **Clarify the scope for brainstorm.** The "surfaced as note/blocker" requirement is ALREADY MET (`Pre2025MethodNote` advisory, `note_pre2025_once`, `fold.rs:28-41`, `state.rs:33`). What remains OPEN is the *reconciliation mechanism*: a means for the user to declare a pre-2025 filing method other than FIFO and have the engine either (a) warn more precisely ("your pre-2025 returns may have used LIFO — the reconstructed carryforward may differ from what you filed") or (b) accept a user-supplied pre-2025 method config and compute accordingly. The brainstorm must decide which of (a) or (b) is in scope.
2. **No code citation is wrong** (spec §7.4 pointer is correct; `BlockerKind::Pre2025MethodNote` pointer is correct). The only inaccuracy is the implication that the blocker is absent.

---

## Slug 4 — `vault-half-created-autorepair`

**WHAT (1-2 lines):** A process kill between the `vault.key` atomic write and the first `vault.pgp` rename (inside `Vault::create`) leaves `vault.key` present but `vault.pgp` and `vault.pgp.bak` absent. Subsequent `create` returns `AlreadyExists`; `open` returns `Io(NotFound)`. No auto-repair exists; manual key deletion is required.

### Citation verification

| Citation | Asserted | Verdict | Evidence |
|---|---|---|---|
| `create` check — `kp.exists()` causes `AlreadyExists` | `vault.rs:36` check fires when key present + pgp absent | **ACCURATE** | `vault.rs:36`: `if vault.exists() \|\| kp.exists() { return Err(StoreError::AlreadyExists); }` — `kp.exists()` is TRUE in the half-created state; returns `AlreadyExists` before any cleanup |
| `StoreError::AlreadyExists` variant exists | Named variant present in error enum | **ACCURATE** | `lib.rs:34`: `#[error("vault already exists at this path")] AlreadyExists,` |
| `recover_target` in `atomic.rs` — only restores when target is MISSING | Does NOT repair when target is corrupt-but-present | **ACCURATE** | `atomic.rs:35-44`: `if !target.exists() { let bak = paths::bak_of(target); if bak.exists() { fs::copy(&bak, target)?; ... } }` — no-op when target exists; no-op when bak absent. For the half-created state: `vault.pgp` is absent AND `vault.pgp.bak` is absent → `recover_target(vault)` is a no-op. |
| `open` → `Io(NotFound)` after recover_target no-op | `std::fs::read(vault)?` fails with Not Found | **ACCURATE** | `vault.rs:83-90`: `recover_target(f)?` for both vault and kp → `vault.pgp` absent, `vault.pgp.bak` absent → no-op; then `std::fs::read(&kp)?` succeeds (key exists); then `std::fs::read(vault)?` → `io::Error(NotFound)` → `StoreError::Io(NotFound)`. |
| In-process failures are cleaned up | `cleanup()` fn called on `built` closure failure or `v.save()` failure | **ACCURATE** | `vault.rs:40-48` defines `cleanup()`; called at `vault.rs:60-64` (closure `Err` branch) and `vault.rs:72-75` (`v.save()` `Err` branch). However `cleanup()` is NOT called on OS kill — it runs only within the process on Rust error propagation. |
| No auto-repair for "key present + pgp+bak absent" | Not implemented | **ACCURATE** | No code path in `vault.rs`, `atomic.rs`, or `lib.rs` detects this specific state and auto-repairs it. The `recover_target` call in `open` only acts when target is missing AND bak exists; it cannot handle "key present, pgp+bak both absent." |
| Manual key deletion needed | Only recovery is `rm vault.key` | **ACCURATE** | With `kp.exists() == true` and `vault.pgp.exists() == false`, no committed data is lost (the kill happened before the first save completed), so `rm vault.key` + `rm vault.key.lock` (if any) + retry `create` is the correct recovery. |

### Numeric / factual claims

No numeric thresholds; all cited file paths and behaviors verified above.

### Action for brainstorm spec (source SHA `428f457`)

All citations accurate; the gap is real and self-contained. The brainstorm/spec should design:
1. A detection path in `Vault::open` (or a new `Vault::repair` helper): if `kp.exists() && !vault.exists() && !bak_of(vault).exists()` → treat as half-created (no committed data); either auto-delete `kp` and return `AlreadyExists` (forcing a clean retry) or emit a new `StoreError::HalfCreated` with a clear message.
2. Alternatively, a detection path in `Vault::create`: before returning `AlreadyExists`, check the half-created state and run `cleanup()` automatically if `!vault.exists() && !bak_of(vault).exists()`.
3. No spec change required (§8 already specifies durability/atomic-write behavior; this is an implementation-level hardening of the existing spec). SemVer: PATCH.
4. Must NOT auto-repair when `vault.pgp` exists (even corrupt) — that case falls under M-1 (bak-on-corrupt) which is a separate FOLLOWUP. The half-created repair is gated specifically on `!vault.exists() && !bak_of(vault).exists()`.

---

## Cross-cutting observations

### Structural citation errors

1. **Slug 2 — wrong spec pointer.** The FOLLOWUP cites "spec §16" for the appraisal-trigger wording. Spec §16 is the implementation-order suggestion list and contains no appraisal-trigger content. Correct pointers: TP10 (spec line 33) for the `appraisal_required` capture requirement; fold-record R3 tag "TAX-N2" (spec line 251) for the precision deferral to FOLLOWUPS.

2. **Slug 2 — wrong behavioral claim.** "Phase 1 flags `Donate.appraisal_required` on FMV>$5k" describes a non-existent auto-computation. Actual current behavior: fully manual `--appraisal` CLI boolean (main.rs:101, 307). Any brainstorm spec that treats FMV>$5k auto-flagging as existing Phase-1 behavior will be building on a false foundation.

### Claim-counting ambiguities

- Slug 3's "OPEN (runtime reconciliation)" vs the implemented advisory blocker creates an apparent inconsistency in FOLLOWUPS.md: the item is listed under "Standing notes / decisions (informational)" but reads as fully unimplemented. The standing-note status is accurate for the reconciliation mechanism; the blocker surfacing is done. The note should distinguish between the two sub-features.

### Incidental version / cross-pin staleness

None found. All cited file paths exist at the expected locations under `crates/btctax-{core,cli,store}/src/`.

### Sync state

Clean at 428f457; `origin/main` up to date; no uncommitted changes. No citation drift due to mid-cycle merges.

### DRIFTED findings

- **Slug 3** has one DRIFTED finding: `Pre2025MethodNote` advisory blocker (`state.rs:33`, `fold.rs:28-41`) is implemented and emitted, contradicting the FOLLOWUP's implied "not yet surfaced." DRIFT magnitude: the blocker emitter was added during the Phase-1 core implementation (committed as part of the core projection tasks). The brainstorm item should be scoped to the RECONCILIATION mechanism, not the blocker.

---

## Recommended brainstorm-session scope ordering

### Grouping

| Slug | Cycle recommendation | Rough LOC | SemVer | Risk |
|---|---|---|---|---|
| **4 — vault-half-created-autorepair** | Cycle 1 (first, alone) | ~30–50 LOC in `vault.rs` + `atomic.rs` + test | PATCH | Lowest — self-contained, no spec change, no cross-crate dependencies, no tax logic |
| **1 — reconcile-allocation-dual-loss-basis** | Cycle 2 | ~80–120 LOC across `event.rs` + `reconcile.rs` + `resolve.rs` + `transition.rs` + tests | MINOR (public struct `AllocLot` gains optional fields; backward-compat if `Option`-al) | Moderate — requires spec change (§6.4 `AllocLot`); touches the safe-harbor allocation path; needs a KAT for pre-2025 gift lot → Path-B seed → loss-zone disposal |
| **3 — pre2025-filed-method-reconciliation** | Cycle 3 (after clarifying scope in brainstorm) | ~40–200 LOC depending on scope (advisory message tweak = PATCH; new CLI config for method = PATCH-MINOR) | PATCH to PATCH-MINOR | Low-medium — scope is unclear until brainstorm decides between message-improvement vs. full alternate-method config |
| **2 — appraisal-trigger-precision** | Phase 2 cycle (not Phase 1) | N/A for Phase 1; Phase 2 requires form-level deduction computation and §170(f)(11)(F) aggregation across all donations in a tax year | MINOR (new computation, new output field) | Medium-high complexity; genuinely Phase-2 work |

### Inter-slug dependencies

- Slug 4 has zero dependencies.
- Slug 1 has zero runtime dependencies but requires a spec PR (§6.4 AllocLot) to be reviewed before implementation.
- Slug 3 depends on a brainstorm decision; the advisory blocker is already in place, so no regression risk from Slug 1 or 4.
- Slug 2 has no Phase-1 implementation dependency; it is deferred.

### Lowest-risk first pick

**Slug 4 (vault-half-created-autorepair)** is the clear first-cycle choice:
- No spec change required.
- Fully self-contained in `btctax-store` (single crate, no core/CLI/adapter interaction).
- The fix is a guard clause + cleanup call in `vault.rs` (< 50 LOC including tests).
- The failure mode is concrete, reproducible, and the fix has a clear acceptance criterion (crash-between-writes → re-try create succeeds without manual intervention).
- No tax-law reasoning involved.

### SemVer notes (pre-1.0)

All four slugs touch a pre-1.0 library. Under the project's semantic intent:
- **Slug 4:** PATCH — no public API change; internal durability fix.
- **Slug 1:** MINOR — `AllocLot` is a public struct in `btctax-core`; adding fields (even optional) is a breaking change under strict SemVer but a MINOR under pre-1.0 conventions. The CLI `safe_harbor_allocate` is the only current build site so the callsite impact is bounded.
- **Slug 3:** PATCH (message improvement) or PATCH-MINOR (new CLI flag).
- **Slug 2:** Phase-2 MINOR (new computation + output).

### GUI / CLI manual locksteps

N/A (no GUI, no manual docs).
