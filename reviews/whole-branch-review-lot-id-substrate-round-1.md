# Whole-branch review — Sub-project A (lot-identification substrate), round 1

**Reviewer:** final whole-diff reviewer (cross-cutting net over Tasks 1–9).
**Scope:** `review-ad79667..0abd9e7.diff` (10 commits) against `SPEC_lot_optimization_program.md`
(Sub-project A + Legal grounding + Cross-cutting) and `IMPLEMENTATION_PLAN_lot_id_substrate.md`.
**Workspace gate:** GREEN (257 tests, clippy `-D warnings` clean, fmt clean, release builds) — accepted, not re-run.
**Date:** 2026-06-29.

---

## Verdict

**NOT YET READY TO MERGE — 1 Important open.** 0 Critical, **1 Important**, 3 Minor, 3 Nit.

The substrate is, on the whole, strong: conservation holds across every method/selection/path combination I
traced; acquisition-date FIFO is applied uniformly at all six consume sites + the method-aware
`universal_snapshot` + the CLI pre-2025 projection; the serde-additive event changes are backward-compatible
with `fingerprint = None`; determinism/exact-arithmetic invariants are clean (no float, no `HashMap`
iteration, no `now()`/RNG in core). The A.7 safe-harbor binding is correct and well-tested.

The one blocker is a **compliance-model integrity hole** in the interaction between an applied post-hoc
`LotSelection` and an in-force `MethodElection` — exactly the kind of cross-task seam the per-task gates
cannot see, and which the Task-7 review recorded only as an *untested* Minor (M1) rather than as the defect it
is.

---

## Cross-cutting findings

### IMPORTANT — A post-hoc `LotSelection` that drives the reported result is mis-labeled `StandingOrder`, presenting a forbidden post-hoc identification as compliant

**Where:** `crates/btctax-core/src/project/compliance.rs:133-154` (the `classify` closure), in combination with
the fold's unconditional application of any valid selection (`fold.rs:51-67` `consume_principal`; resolve
retains selections with **no timeliness gate**, `resolve.rs:565-612`).

**The seam.** The fold applies *any* valid (non-voided, non-dup, principal-conserving, feasible) `LotSelection`
to its disposal **regardless of when the selection was recorded** — there is no contemporaneity check on the
computation path. The reported basis/gain in `report`/CSV therefore come from the selected lots. Separately,
`disposal_compliance` classifies in the order: 2027+broker-envelope → `Contemporaneous` (selection
made-date ≤ sale date) → `StandingOrder` (in-force election) → `NonCompliant`. A selection whose made-date is
**after** the sale (post-hoc) fails the `Contemporaneous` test and, per the code's own comment, *"falls
through to StandingOrder check."*

**Consequence (reachable with A's own CLI):**
1. `config --set-forward-method hifo` → in-force `MethodElection`.
2. a post-2025 disposal executes.
3. later, `reconcile select-lots <disposal> --from <lot>:<sat> …` picking lots **other** than the HIFO order →
   a valid post-hoc `LotSelection`, applied by the fold (numbers = the cherry-pick).
4. `verify` reports that disposal as `StandingOrder { … }` — a *compliant* status — while the reported lots are
   a post-hoc selection the standing order would never produce.

An auditor matching the reported lots against the standing-order method finds a mismatch → the position is
undocumented → it collapses to FIFO. The tool has given false comfort. This directly violates the
**load-bearing cross-cutting rule** (SPEC §Cross-cutting): *"no artifact, command, or doc may describe
post-hoc selection as compliant."* The implementation matches A.5's *literal* priority list, but the
cross-cutting principle overrides A.5 where they conflict.

**Note this is a defect, not merely a test gap.** Progress `progress.md` Task 7 records exactly this path as
"MINOR (M1): untested post-hoc-selection+valid-election→StandingOrder path." It is the path itself that is
wrong, and there is no test pinning a (correct) result, so nothing flagged it.

**Minimal fix (one branch).** When a selection is *applied* to a disposal, judge compliance by **that
selection's** timeliness; the standing order must govern only when **no** selection overrode the disposal. In
the `classify` closure, the post-hoc branch should `return ComplianceStatus::NonCompliant` instead of falling
through to the `StandingOrder` check:

```rust
if let Some(made) = sel_made.get(disposal) {
    if *made <= date { return ComplianceStatus::Contemporaneous; }
    return ComplianceStatus::NonCompliant; // post-hoc selection drove the result — not rescued by a standing order
}
```

This is behavior-preserving for every existing compliance test (`post_hoc_selection_is_noncompliant`,
`self_custody_2027_with_election_is_standing_order`, etc.); it changes only the un-pinned masking case. Add the
Task-7-M1 test asserting `NonCompliant` for *post-hoc selection + in-force election*. (If the spec owner instead
elects to treat this as spec-sanctioned and defer the fix to C, that is a material re-entry of the gate, and at
minimum A must (a) document the limitation and (b) add a test pinning the *intended* status — but I recommend
fixing it in A, since A's `select-lots` + `disposal_compliance` produce the mislabel with no C involvement.)

---

### Conservation end-to-end — PASS

Traced Σsat/Σbasis across: FIFO/LIFO/HIFO standing orders; per-disposal selections (feasible + every fallback);
pre-2025 Universal vs per-wallet pools; the method-aware `universal_snapshot`; Path-A reconstruction and Path-B
seeding (incl. the `init_split_counter` I-2 guard, `transition.rs:98-100`). Findings:

- `take_from` (`pools.rs:208-233`) is the verbatim prior pro-rata arithmetic — only the *order* changed —
  so Σsat/Σbasis are conserved on every method (KAT `consumption_conserves_sat_and_basis_under_every_method`).
- Selection fallback (`consume` → `consume_ordered` on infeasibility, `pools.rs:79-101`) consumes `need` by
  method order and raises `LotSelectionInvalid` (Hard) — conserved + gated.
- The real selection path is conservation-safe because `resolve` enforces `Σpicks == principal`
  (`resolve.rs:599-611`) *before* `consume_picks` (which returns `shortfall = 0` by design). The fee leg consumes
  FIFO from the post-selection remainder (`fold.rs` Dispose/SelfTransfer/GiftOut/Donate arms) — A.4(a) honored.
- `conservation_report` is sat-only (`conservation.rs`); sats are method-invariant; Path-B seed `held_sat`
  equals the discarded residue's `held_sat`. `balanced` holds across all combinations.
- A method/recorded-method drift under Path B is caught by the **Hard** `Pre2025MethodConflictsAllocation`
  (not `SafeHarborUnconservable`), gating the result before any basis discontinuity can be relied upon.

### Acquisition-date FIFO consistency — PASS

All six consume sites route through `consume`→`method_order` (acquisition-date asc, tie `lot_id`):
Dispose/GiftOut/Donate/SelfTransfer via `consume_principal`; PendingOut + `consume_fee` via the
`consume_fifo` wrapper (`pools.rs:62-65`, now acquisition-date, not insertion order). `universal_snapshot`
reuses `fold_event` (`transition.rs:60-62`) so the residue cannot diverge; the CLI `safe_harbor_allocate`
re-projects the pre-2025 subset through the same `project()` path (`reconcile.rs:219-242`). No code path walks raw
push order; no leftover insertion-order assumption remains. The relocated-lot and Path-B-seed divergence KATs
exist (pools.rs, method_election.rs, safe_harbor_method.rs). FOLLOWUPS records the deliberate C1 change.

### Compliance model integrity (§1.1012-1(j)) — PASS except the Important above

`MethodElectionBackdated` (effective_from < made-date OR < TRANSITION_DATE), `LotSelectionInvalid`, and
`Pre2025MethodConflictsAllocation` are all `Severity::Hard` (`state.rs:47-63`) and partition to
`VerifyReport.hard` → non-zero FR9 exit. Custody mapping (Exchange=broker / SelfCustody=own-books) and the
2027+ broker envelope are correct and tested. Aside from the masking defect, no path records a post-hoc
identification as compliant by default.

### §7.4 safe-harbor interaction (A.7) — PASS

`SafeHarborAllocation.pre2025_method` is `#[serde(default)]` (→ Fifo) immutable; preserved across re-attest via
`..prior` (`reconcile.rs:488-491`); captured from the same `cfg` the residue was projected under
(`reconcile.rs:253-257`). `universal_snapshot` is method-aware (`transition.rs:32-72`). The conflict fires only
in the single-effective arm, only when live ≠ recorded (`resolve.rs:723-743`), never rewrites/voids the
allocation, clears by reverting config — no deadlock, no spurious fire. Well covered by
`safe_harbor_method.rs` (composition, contrast, conflict, clears, serde-default, acq-date residue KATs).

### Event-sourcing / backward-compat — PASS

`MethodElection`/`LotSelection`/`LotPick` are additive enum variants; `fingerprint()` returns `None` for them
(`persistence.rs:96`); the only new field on an existing payload (`SafeHarborAllocation.pre2025_method`) is
`#[serde(default)]`. No imported-payload fingerprint changed → no event-id/dedup change; old vaults load and an
old effective Path-B allocation reproduces identically (recorded method defaults to Fifo == old config default).

### Determinism / exact-arithmetic / privacy — PASS

`grep` over `crates/btctax-core/src`: no `f32`/`f64`/`as f64`, no `HashMap`/`HashSet`, no `now()`/RNG (only a
doc comment). HIFO is cross-multiplied `Decimal` (`hifo_cmp`, `pools.rs:273-285`), total order with explicit
tiebreaks. Selections grouped/iterated via `BTreeMap` (resolve, compliance, import-selections). Tests use
synthetic fixtures + temp vaults. `determinism_with_elections_and_selections_is_load_order_independent` pins NFR4
over the new surface.

---

## Minor

- **M1 — `disposal_compliance` omits method-honoring SelfTransfers.** SelfTransfers produce no
  Disposal/Removal record, so they never get a compliance row (`compliance.rs:162-189` iterates only
  `state.disposals`/`state.removals`). A.3 lists SelfTransfer as method-honoring (a §1.1012-1(j) "transfer" that
  pre-positions lots for future HIFO/gains) and A.5 says "each method-honoring disposal." A post-hoc
  `select-lots` on a self-transfer is therefore never compliance-flagged. Decide explicitly whether transfers
  belong in the projection; if intentionally excluded, document it.

- **M2 — `evaluate_disposal` validates the existing-event selection against `candidate.sat`, not the resolved
  principal.** `evaluate.rs:143-156` checks `Σpicks == candidate.sat`; the fold then consumes the event's *real*
  principal with that selection. Because `consume_picks` returns `shortfall = 0` unconditionally
  (`pools.rs:175`), a caller passing a `candidate.sat` ≠ the event's real principal would silently under/over-
  consume **without** an `UncoveredDisposal`. Latent in A (`evaluate_disposal` is exported but wired to no A CLI
  surface; C wires it). Tighten before C: validate against the event's resolved principal, or route the
  existing-event selection through resolve's conservation guard.

- **M3 — `config --set-forward-method` silently drops a co-passed `--set-fee-treatment` / `--set-pre2025-method`.**
  `main.rs:264-275` records the `MethodElection` and `return`s early, ignoring the other flags — the same silent-
  drop anti-pattern Task 1 fixed for the `set_pre2025_method`/`set_fee_treatment` pair (`main.rs:285-297`). The
  Task-5 review already flagged that the `Command::Config` dispatch arm has no binary-level test, which is why
  this slipped through. Low harm (unusual flag combo), but inconsistent; add the binary dispatch test.

## Nit

- **N1 — `ComplianceStatus` rendered with `{:?}`** in `render_verify` (`render.rs:645-653`); this is compliance-
  facing output — add a stable `compliance_status_display` (already recorded as a Task-8 nit).
- **N2 — `evaluate` `lots_after` is "as-if-inserted," not "appended."** A past-dated candidate folds into
  canonical order and perturbs later real disposals in the throwaway fold; harmless for the candidate's own
  legs/gains (blockers are filtered to `target_id`), but document the semantics for C.
- **N3 — forward-looking (B, not A):** `MethodElectionBackdated` / `Pre2025MethodConflictsAllocation` are
  attributed to decision events with no tax year. FR9/`verify` gates globally on any Hard blocker (so A is fine),
  but B.4's *per-year* gate must not key solely on disposal-year attribution, or it could emit a `TaxResult` for a
  year whose election was silently rejected. Carry into B's plan.

---

## Triage of recorded per-task Minors / Nits (`progress.md` Tasks 1–9 A)

| Item | Disposition | Reason |
|---|---|---|
| **Task 1 — multi-flag silent-drop + attest guard** | **DEFER (closed)** | Fixed in Task 5 for the two config flags + attest guard (`main.rs:279-297`). The remaining `--set-forward-method` variant is new finding **M3**. |
| **Task 2 — `consume_picks` shortfall=0 by design** | **DEFER** | Real path is guarded by resolve's `Σpicks==principal` (`resolve.rs:599-611`); only the evaluate entrypoint is loose → new finding **M2**. |
| **Task 4 — plan KAT text `dec!(90.00)` vs implemented `90.25`** | **DEFER** | Plan-doc text only; implementation correct (TP8(c) fee re-home). Reconcile plan text as a doc followup. |
| **Task 5 — apply-all/attest-guard not tested at the clap dispatch arm** | **DEFER** | Logic verified by inspection; add a binary-level dispatch test (also covers **M3**). |
| **Task 7 — M1: untested post-hoc-selection + valid-election → StandingOrder** | **BLOCK** | This is the **Important** above — a defect, not a test gap. Fix the classifier + add the test asserting `NonCompliant`. |
| **Task 7 — M2: `collect_elections` duplicates resolve's guard** | **DEFER** | DRY only; the two collectors are spec-kept-in-sync. Extract a shared collector as a followup (would also reduce drift risk on the M1 fix). |
| **Task 8 — Nit: `ComplianceStatus` `{:?}`; `selection_count` missing Decision guard** | **DEFER** | Cosmetic (**N1**); a `LotSelection` payload only ever rides a `Decision` event, so the count guard is moot in practice. |
| **Task 9 — Nit: `u64::MAX` synthetic sentinel; add pinning KAT existing==project()** | **DEFER** | Sentinel documented and unreachable by real seqs; add the `existing-disposal-no-selection == project()` KAT as a followup. |
| **Task 3 (2 nits) / Task 6 (1 nit)** — undetailed in progress | **DEFER** | Reviewed the underlying areas (MethodElection in-force ordering; A.7 method-aware snapshot) and found them sound. |

---

## Bottom line

Resolve the one Important (compliance masking) — a one-branch fix plus the Task-7-M1 test — and re-review. With
that closed, Sub-project A is conservation-sound, deterministic, backward-compatible, and a solid substrate for
B/C. The Minor/Nit items are safe to carry as FOLLOWUPS, except that **M2** should be tightened before C wires
`evaluate_disposal`.

---

# Round 2 — fold re-review (2026-06-29)

**Reviewer:** independent fold re-reviewer (round 2).
**Scope:** the fold `review-0abd9e7..3f4ddbc.diff` (single commit `3f4ddbc`) against the round-1 findings.
Re-reviews only the Important + the two in-area Minors folded (M2, M3) and checks the fold introduced no new
defect. The rest of A is accepted from round 1 (not re-litigated).
**Workspace gate:** accepted as authoritative — 260 pass, clippy/fmt/release clean. Not re-run.

## Verdict

**READY TO MERGE.** The round-1 Important is genuinely closed; M2 and M3 are closed; the fold introduces
**0 new Critical / 0 new Important** (0 Critical, 0 Important, 2 Nit residuals, all non-blocking).

## 1. IMPORTANT — CLOSED (load-bearing) ✓

`compliance.rs::classify` (`compliance.rs:130-169`) now resolves in the exact priority the finding required:

1. **2027+ broker envelope still fires first** — `compliance.rs:134-137`, unchanged, before any selection check.
2. **An applied `LotSelection` is judged by ITS OWN timeliness, with early return** — `compliance.rs:144-149`:
   `made <= date → Contemporaneous`, **`else → return NonCompliant`**. The post-hoc case no longer falls through.
3. **A `MethodElection` standing order is reachable ONLY when no selection was recorded** — the standing-order
   block (`compliance.rs:154-165`) sits *after* the `if let Some(made) = sel_made.get(disposal)` block, which
   returns in **both** arms. There is no longer any control-flow path from a post-hoc selection to
   `StandingOrder`. A standing order can never rescue a post-hoc pick.
4. Fall-through `NonCompliant` (`compliance.rs:168`) for the no-envelope/no-selection/no-election case.

No remaining path labels a post-hoc *applied* selection compliant. The new KAT
`post_hoc_selection_with_in_force_election_is_noncompliant` (`tests/compliance.rs:413-448`) genuinely exercises
the fixed branch: self-custody 2025 (envelope silent), election effective 2025-06-01 ≤ sale 2025-07-01,
selection made 2025-09-01 (post-hoc) and principal-conserving (picks A 100k == disposal 100k → applied by the
fold). Old code: post-hoc → fall-through → in-force election → `StandingOrder` (wrong). New code:
`NonCompliant`. The test would have failed pre-fix.

**Behavior-preserving for the 8 prior compliance tests** (verified by tracing each through the new branch
structure): `post_hoc_selection_is_noncompliant` (no election — now `NonCompliant` by the new direct return at
`:148` rather than fall-through; same result, confirmed `tests/compliance.rs:202-230` has no `MethodElection`);
`contemporaneous_status_when_selection_made_before_sale` (early return at `:146`, unchanged);
`standing_order_status_self_custody`, `broker_2026_own_books_election_is_standing_order`,
`self_custody_2027_with_election_is_standing_order` (no selection → reach the standing-order block, unchanged);
`noncompliant_when_no_basis` (fall-through, unchanged); `broker_2027_plus_is_noncompliant_even_with_election`
(envelope fires first, unchanged).

*Observation (not a finding):* `sel_made` keys on selections *recorded* (raw events), not *applied* (fold
output). The only divergence is a recorded-but-invalid selection (resolve drops it from `res.selections` AND
raises a Hard `LotSelectionInvalid`); such a disposal is on a fully gated report, and the new label moves it in
the conservative (`NonCompliant`) direction. No false-compliant path, and this nuance is unchanged by the fold.

## 2. M2 — CLOSED ✓

`evaluate_disposal` existing-event branch (`evaluate.rs:111-124`) now captures the event's **resolved**
principal via `honoring_sat(&e.op)` from `res.timeline`, and the conservation guard (`evaluate.rs:162-175`)
compares `Σpicks != principal` (the resolved sat), not `candidate.sat`. The new `honoring_sat`
(`evaluate.rs:76-84`) is **byte-for-byte identical in logic** to resolve's authoritative `honoring_principal`
(`resolve.rs:794-802`), and the fold consumes exactly that `sat` for every honoring arm
(`fold.rs:435/595/815/882` all pass `*sat` to `consume_principal`). So evaluate now validates against the same
basis resolve guards and fold consumes — a wrong `candidate.sat` can no longer silently under/over-consume. KAT
`existing_disposal_selection_validated_against_resolved_principal_not_candidate_sat` (`tests/evaluate.rs:188-260`)
pins it: `candidate.sat = 60k` (real principal 100k) with Σpicks = 60k now raises `LotSelectionInvalid` instead
of silently mis-consuming. Synthetic path correctly keeps `principal = candidate.sat` (`evaluate.rs:154`).

## 3. M3 — CLOSED ✓

`Command::Config` (`main.rs:252-305`) applies ALL co-passed flags with no early return: the
`--attest requires --set-pre2025-method` guard is preserved and now checked **first** (`main.rs:264-268`, returns
a clear `CliError::Usage` before any event/mutation); `--set-forward-method` appends the `MethodElection`
(`main.rs:275-285`) with the early `return Ok(...)` removed; `--set-pre2025-method` and `--set-fee-treatment`
then apply independently (`main.rs:290-299`). No silent drop remains. Test
`config_set_forward_method_and_fee_treatment_both_take_effect` asserts both the persisted HIFO `MethodElection`
and the `TreatmentB` fee mutation take effect.

## 4. No new defect ✓

- **Early-return restructure dropped nothing.** Envelope, contemporaneous, standing-order (`max_by` on
  `effective_from` then `decision_seq`), and fall-through logic are all intact; the only delta is the post-hoc
  early return — exactly the intended fix.
- **`honoring_sat` is correct** for all four honoring op types (matches resolve's `honoring_principal` and the
  `*sat` the fold consumes); `None` for non-honoring → `UnknownExistingDisposal`, unchanged.
- **Conservation / determinism / no-float intact.** Compliance is a read-only labeler (no conservation impact);
  M2 only adds a blocker on mismatch (conservative); no `f64`/`HashMap`/`now()` introduced; `sel_made` still
  ascending-seq via `BTreeMap`, output sorted by `EventId`.
- **Deferrals recorded in FOLLOWUPS**, not silently dropped: M1 (SelfTransfer compliance), Task-4 plan-text
  doc, Task-7-M2 (shared collector DRY), Task-8 nits (incl. N1), Task-9 nits.

## Residuals (Nit — non-blocking, carry as followups)

- **Nit (deferral-tracking gap):** round-1 Nits **N2** (`evaluate.lots_after` "as-if-inserted" — document for C)
  and **N3** (B-only per-year gate must not key on disposal-year attribution) are not captured in the new
  FOLLOWUPS section (N1 was, as Task-8(a)). Add them so they survive into C/B planning. Forward-looking and
  Nit-severity; does not block A.
- **Nit (test fidelity):** `config_set_forward_method_and_fee_treatment_both_take_effect` simulates the dispatch
  at library level (calls `set_forward_method` + `set_config` directly) rather than driving the actual clap
  `Command::Config` arm — consistent with the existing `attest_pre2025_method_requires_set_pre2025_method`
  pattern, and the M3 apply-all logic is directly verifiable in `main.rs`. A true binary-level dispatch test
  would fully retire the round-1 Task-5 note.

## Bottom line

Important closed, M2/M3 closed, no new Critical/Important. **Sub-project A is ready to merge.** Two Nit
residuals (two un-tracked round-1 Nits; one test-fidelity caveat) are safe to carry as FOLLOWUPS.
