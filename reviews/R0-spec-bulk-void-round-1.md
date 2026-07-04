# R0 — SPEC_bulk_void.md — round 1 (independent architect)

**Artifact:** `design/SPEC_bulk_void.md` @ `3bee644` (branch `feat/bulk-void`; main == `13cb135`).
**Bar:** 0 Critical / 0 Important. **Scope:** read-only spec review vs CURRENT source. Cycle 3 — the dangerous one.

## Verdict: **0 Critical / 0 Important / 3 Minor / 3 Nit — R0-GREEN on the blocking bar.**

The tax-safety core (#7) is architecturally SOUND: the extracted predicate is byte-equivalent to the shipped
single-void filter (which becomes the single source of truth after Task 1), and the underlying engine
semantics — effective allocation → Hard `DecisionConflict`, inert allocation → void applies cleanly — are
confirmed in `resolve.rs` and pinned by an existing KAT. The persist atomicity is sound (whole-DB
snapshot/restore covers the side-table). Both flagged decisions (bespoke-vs-hook, Tier-A-vs-B) are CORRECT.
The remaining findings are citation hygiene + two explicitness clarifications; none blocks implementation.

---

## Verification of the mandate points

### 1. The #7 tax-safety exclusion — VERIFIED SOUND
- **(a) Signature reachability — CONFIRMED.** The predicate reads `blockers`. TUI reads `snap.state.blockers`
  (single-void: `main.rs:2755-2758`); CLI reaches the identical projected blockers via `Session::project()`
  → `LedgerState.blockers` (`session.rs:299-305`) — the shipped `bulk_resolve_conflict_plan` already uses
  exactly this (`session.rs:595` `load_events_and_project()` → `state.blockers`). `Blocker`/`LedgerEvent`/
  `BlockerKind` are all `btctax-core` types (`state.rs:23,93`, `event.rs:340`), and `btctax-cli` depends on
  core (`Cargo.toml:17`). So `voidable_decisions(events: &[LedgerEvent], blockers: &[Blocker]) -> Vec<&LedgerEvent>`
  is reachable by BOTH front-ends. Sound.
- **(b) Effective void → Hard DecisionConflict — CONFIRMED.** `resolve.rs:1030-1039`: for each allocation
  void, `if effective.iter().any(|(id,_,_)| id == &v.target)` → pushes `BlockerKind::DecisionConflict`
  ("void targets an effective SafeHarborAllocation (irrevocable, §7.4)"). `effective` membership requires
  non-voided ∧ SafeHarborAllocation ∧ ¬unconservable ∧ ¬timebarred (`resolve.rs:952-1027`). The predicate's
  `effective_alloc` = SafeHarborAllocation ∧ ¬`SafeHarborTimebar` ∧ ¬`SafeHarborUnconservable` blocker on
  the id — and unconservable ⟹ that blocker (`resolve.rs:989-994`), timebarred ⟹ that blocker
  (`resolve.rs:997-1002`). Since candidates are pre-filtered `¬voided`, the predicate is EXACTLY equivalent
  to `effective`-membership. Adversarial edge cases all hold: unconservable-first `continue` (995) means only
  `SafeHarborUnconservable` fires, and `!has(Unconservable)` already fails the predicate → stays voidable
  (correct, inert); multiple-effective (`resolve.rs:1064-1073`) → both are in `effective`, voiding either
  conflicts, and both have neither blocker → both excluded (correct). The DecisionConflict-on-void blocker
  itself is keyed to the *void id* (`event: Some(v.void_id)`), never the allocation id, so it can't confuse
  the predicate.
- **(c) Inert allocations stay voidable — CONFIRMED.** `resolve.rs:1030-1031` comment ("a Void of an
  inert/absent allocation simply applies (no conflict)") and the KAT `void_of_inert_allocation_applies_no_conflict`
  (`crates/btctax-core/tests/transition.rs:403`, asserts `!has(DecisionConflict)`). Inert (timebar OR
  unconservable) allocations get their blocker + `continue` (never enter `effective`), so the predicate
  keeps them listed — the spec's "don't over-exclude" is correct.

**Conclusion:** #7 is right. Extracting (not copying) the predicate is the correct architecture — it makes
the filter a single source of truth, eliminating the drift class the spec names as the whole ballgame.

### 2. Task-1 extraction — SOUND
- `is_revocable_payload` (`form.rs:896-911`) is a pure `matches!` over `btctax_core::EventPayload`, no
  tui-edit-only types → freely relocatable to core.
- **All dependents are inside `btctax-tui-edit`** (grep: `form.rs`, `persist.rs`, `editor.rs`, `main.rs`
  — none in `btctax-cli` or elsewhere). So the move needs only a tui-edit re-import (`pub use` or re-point
  the `use` at `main.rs:23`); no cli change. Zero external breakage.
- `voidable_decisions` in core is reachable by cli (cli → core dep confirmed). Re-pointing `open_void_flow`
  (`main.rs:2764-2786`) to it preserves behavior; the `voidable_decisions_matches_single_void_flow` KAT is
  the right pin.

### 3. `persist_bulk_void` side-effect + atomicity — SOUND
- The skeleton faithfully transplants the shipped `persist_void` airtight arm (`persist.rs:248-291`) across
  N, in the exact shape of `persist_bulk_decisions` (`persist.rs:394-422`): empty-guard → `snapshot()` →
  per-item `append_decision` guarded by `return Err(rollback(session,&pre,e.into()))` (NOT `?`) → per-
  `LotSelection` `optimize_attest::clear` guarded by `return Err(rollback(...))` → single `save_or_rollback`.
- **Mid-batch rollback covers BOTH void rows AND side-table clears — CONFIRMED.** `rollback` (`persist.rs:62-71`)
  → `session.restore(pre)` (`session.rs:253-256`) → whole-DB image restore; `snapshot()` (`session.rs:247`)
  captures the entire in-memory SQLite DB including the `optimize_attestation` side-table. This is exactly the
  load-bearing reason `persist_void` uses whole-DB restore, documented at `persist.rs:241-247` [M1]. So a
  failing append OR a failing clear at row k reverts everything.
- **Precomputing `disposal_to_clear` once from the snapshot — CORRECT and preferable.** Single-threaded held
  session; no mutation between the caller building `VoidTarget`s and `persist_bulk_void`'s own `snapshot()`,
  so the precomputed disposals match. Avoids N× `load_all` that the naive port of `persist_void`'s inline
  `load_all` (`persist.rs:262`) would incur. Sound.

### 4. Bespoke-vs-hook — **ADJUDICATED: bespoke is the RIGHT call. AGREE with the spec.**
`persist_bulk_decisions` is a shared safety-critical helper backing THREE shipped flows (bulk-link,
bulk-STI, bulk-resolve-conflict; `persist.rs:428-475,486+`). Threading a per-item side-effect closure through
it to carry the dangerous `optimize_attest::clear` would widen the blast radius of a proven-airtight helper
and complicate its single-responsibility (append-only). A bespoke `persist_bulk_void` isolates the side-effect
path at the cost of ~15 mirrored lines whose invariant is pinned by `bulk_void_reverts_mid_batch`. For a
tax-year-gating op, isolation > DRY. (See Nit N2 for the lockstep note.)

### 5. Tier-A-vs-B — **ADJUDICATED: Tier-B is CORRECT. AGREE with the spec; the roadmap's early Tier-A note is superseded.**
The tier taxonomy is confirmed in source: Tier-A = plain modal for REVOCABLE ops (bulk-link/STI/allocate,
"each voidable", `draw_edit.rs:3520,3719`); **Tier-B = prominent NON-REVOCABLE warning, NOT typed-word**
(single resolve-conflict + bulk-resolve-conflict, `draw_edit.rs:3921-3922`, `persist.rs:485`); Tier-C =
typed-word "ATTEST" gate reserved for the §7.4 irrevocable attestation (`draw_edit.rs:2685-2691`,
`form.rs:1210`). Void events are non-revocable (`is_revocable_payload` EXCLUDES `VoidDecisionEvent`,
`form.rs:892`), so a bulk-void cannot be undone by another void — Tier-B's non-revocable warning is the
right ceremony, matching its sibling bulk-resolve-conflict + adding the blast-radius line. NOT Tier-C,
because void is RECOVERABLE (re-apply the original decision → a new decision_seq that isn't voided), unlike
a truly irrevocable attestation. Correct calibration.

### 6. CLI two-phase + KAT coverage — CONSISTENT (one explicitness gap, see M3)
The shipped bulk dispatch (`main.rs:1235-1278`, bulk-resolve) is: compute plan (predicate) → render → if
`dry_run` return → confirm → **derive apply-targets from `plan.rows`** (`main.rs:1267-1268`) →
`apply_bulk_*` (bare-`?` CLI atomicity, `reconcile.rs:315-332`). `apply_bulk_void` mirroring
`apply_bulk_accept_conflicts` with a per-`LotSelection` `optimize_attest::clear` (as the single CLI `void`
does, `reconcile.rs:143-145`) is consistent. KAT set covers the tax-safety (exclude effective / include
inert), the side-effect (clear on LotSelection), and the mid-batch rollback with BOTH the append-fail AND
the clear-fail arms reverting rows AND clears — the dangerous case IS covered.

### 7. Over-reach / serde / lockstep — CONFIRMED relocation-only
No new `EventPayload` variant: `VoidDecisionEvent` (`persist.rs:254`, `event.rs`) and the `optimize_attest`
side-table (`crates/btctax-cli/src/optimize_attest.rs`) already exist and are reused. Core change is purely
additive/relocation (`is_revocable_payload` move + new `voidable_decisions`). No forward-only serde break;
the projection already folds `VoidDecisionEvent`. Confirmed.

---

## Findings

### [M1] MINOR — `resolve.rs:865-921` mis-cites the effective-allocation logic
**Spec:** §Candidate-set item 4 and §Tax-safety cite "`resolve.rs:865-921`" for `effective_alloc`.
**Source:** `resolve.rs:865-921` is the §A.4 **LotSelection** collection block — unrelated. The
effective/timebar/unconservable computation is `resolve.rs:951-1027` (blocker firing at `991`
`SafeHarborUnconservable`, `999` `SafeHarborTimebar`), and the void-of-effective → `DecisionConflict`
rejection is `resolve.rs:1030-1039`. The mis-citation is inherited verbatim from the shipped single-void
comment (`main.rs:2749`), but the spec re-asserts it and the workflow requires citations verified at write
time. Behavior is CORRECT; only the pointer is wrong.
**Fix:** cite `resolve.rs:988-1004` (blocker firing) + `resolve.rs:1030-1039` (void-of-effective conflict).
Recommend fixing the inherited comment at `main.rs:2749` in the same pass.

### [M2] MINOR — `transition.rs:403` is ambiguous / points to a test, not source
**Spec:** §Candidate-set item 4 and §Tax-safety cite "`transition.rs:403`" for "inert allocations apply
cleanly." **Source:** `crates/btctax-core/src/project/transition.rs` is only 103 lines; line 403 is the KAT
`void_of_inert_allocation_applies_no_conflict` in `crates/btctax-core/tests/transition.rs:403`. The
SOURCE-level statement of the invariant is `resolve.rs:1030-1031`. Same inherited-from-`main.rs:2751` drift.
**Fix:** cite `crates/btctax-core/tests/transition.rs:403` explicitly as the KAT and/or `resolve.rs:1030-1031`
as the source invariant. Disambiguate the two `transition.rs` files.

### [M3] MINOR — make the CLI's #7-enforcement-at-apply explicit
**Spec:** §CLI says Phase 1 `Session::bulk_void_plan` lists candidates via the shared predicate, and Phase 2
`apply_bulk_void(vault, pp, targets, now)` appends the voids. **Source:** the neither the single CLI `void`
(`reconcile.rs:110-149`) nor any shipped bulk `apply_*` re-checks `effective_alloc` — the CLI's #7 defense
holds ONLY because the shipped dispatch derives apply-targets from `plan.rows` (`main.rs:1267-1268`), never
from raw user-supplied ids. The spec should state that `apply_bulk_void`'s `targets` ARE the `bulk_void_plan`
rows (predicate-filtered), matching the bulk-resolve dispatch — otherwise an implementer could give it a
`--ref`-style raw id and reintroduce the trap. (Note: this mirrors the shipped single-`void` exposure and
the resulting `DecisionConflict` is a loud Hard blocker, not silent — hence Minor, not Important.)
**Fix:** add one line to §CLI: "`targets` are the plan's rows (predicate-filtered), never raw `--ref` ids —
no per-id void arg." Consider a `bulk_void_dry_run_excludes_effective` KAT at the plan level.

### [N1] NIT — shared-disposal double-clear is uncovered (harmless)
Two duplicate `LotSelection`s targeting one disposal are BOTH bulk-void candidates (both revocable; neither
is a SafeHarborAllocation so `effective_alloc` doesn't exclude them; the duplicate fires `DecisionConflict`
at `resolve.rs:893-901` but stays voidable). Bulk-voiding both calls `optimize_attest::clear(disposal)`
twice — idempotent per `reconcile.rs:142` ("clearing an absent row is Ok"), so harmless, but no KAT pins it.
**Fix:** optional one-line note in §Persist or a KAT asserting a double-clear is a no-op.

### [N2] NIT — add a lockstep note to the bespoke `persist_bulk_void`
The bespoke fn duplicates `persist_bulk_decisions`'s empty-guard + mid-batch-rollback + single-save
invariant. If that shared contract changes, the copy must follow. Pinned by `bulk_void_reverts_mid_batch`,
but a `// keep in lockstep with persist_bulk_decisions [rollback contract]` comment is cheap insurance.

### [N3] NIT — §Core wording places `optimize_attest` in core; it lives in cli
§Core says core "reuses the existing `VoidDecisionEvent` + `optimize_attest` side-table." `optimize_attest`
is `crates/btctax-cli/src/optimize_attest.rs`, and the `clear` side-effect runs in the tui-edit/cli persist
layer; `voidable_decisions` in core touches neither. Reword to avoid implying the side-table moves to core.
No functional impact.

---

## Adjudication summary
- **Bespoke-vs-hook:** bespoke — **AGREE.** Blast-radius isolation of the dangerous side-effect beats DRY for
  a year-gating op; the ~15 mirrored lines are KAT-pinned.
- **Tier-A-vs-B:** Tier-B — **AGREE.** Void is non-revocable (→ warrants > Tier-A) but recoverable by
  re-applying the decision (→ not Tier-C typed-word); Tier-B matches the sibling bulk-resolve-conflict and
  the confirmed source taxonomy. The roadmap's early Tier-A note is superseded.

**Bottom line: R0-GREEN (0C/0I).** #7 is verified correct end-to-end. Fold the three Minors (citation
hygiene M1/M2 + the CLI-apply explicitness M3) and the Nits before/at implementation; re-review round 2.
