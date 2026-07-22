# Defensive Filing Wizard — IMPLEMENTATION_PLAN tax-correctness review (Opus, r5)

Lens: US-federal-tax-correctness. Independent re-derivation from CURRENT source at plan HEAD `de4c9cd`
(the plan's self-citations / line numbers were NOT trusted — every load-bearing claim below was
re-grepped this round). Sources verified:
`conservative.rs:683-790` (`promote_prior_year_advisory`: fold pair `:701-707`, candidate-year gather
`:720-728`, the `< current` filter `:729`, per-year leg-set Vec-eq diff `:755-761`, doc `:683-687`/`:716-719`);
`conservative.rs:27` (`tranche_dip_advisory`), `:61` (`method_inversion_advisory`);
`cmd/tranche.rs:40` (`void_targets`, private `fn`), `:54` (`in_force_allocation_exists`, pub), `:71`
(`pre2025_tranche_exists`, pub), `:93` (`guard_allocation_vs_tranche`), `:107` (`guard_tranche_vs_allocation`),
predicate call-sites `:61,72,94,111`, declare gate `:154`, phantom-wallet `eprintln!` `:159`;
`session.rs:702-724` (`safe_harbor_residue`; the FIFTH direct `pre2025_tranche_exists` consumer at `:714`);
`cmd/admin.rs:78` (`promote_export_gate`), `:261` (`IrsPdfReport`), `:350`/`:358` (`export_irs_pdf` + its
`Session::open`), `:373` (`return_inputs::exists` dispatch), `:578` (crypto-slice `IrsPdfReport`), `:642`
(`export_full_return(session: &Session, …)`);
`cmd/promote.rs:95` (`resolve_live_tranche`), `:333` (`render_consent`), `:346` (`require_promote_ack`),
`:364-488` (pipeline: advisory print `:443`, `gift_only_flagged_years` `:449`, `wide_window_note` `:454`,
`shown_terms: terms` `:468`, `would_conflict` `:478`);
`project/fold.rs` (the `UncoveredDisposal` emit sites — the SIX sat-carrying at `:388,710,831,876,1196,1274`
among the fifteen total; the others are the without-wallet / degenerate variants);
allocation append sites: `reconcile.rs:1015,1258` + `edit/persist.rs:1032,1105` (all call the STAYING
`guard_allocation_vs_tranche`); `lib.rs:27` (`pub use cmd::tranche::guard_allocation_vs_tranche`).

## Verdict

**GREEN — 0 Critical / 0 Important / 0 Minor / 2 Nit**

The r4→r5 fold (commit `de4c9cd`) discharges all three r4 non-gaters (M-1 KAT reframe, M-2 fifth-consumer
rewire, N-1 `< current` filter) correctly, preserves every binding SPEC decision (DFW-D1..D12), and adds
NO new tax logic: `flagged_years` replicates the SHIPPED `promote_prior_year_advisory` fold-pair machinery
(`conservative.rs:701-761`) into a typed `BTreeSet<i32>` return — same fold, same per-year leg-set Vec-eq
diff, no new tax computation. All four mandated confirmations re-derive clean:

1. **The `< current` filter on `flagged_years` is correct.** `plan_export.years = {current} ∪ { y ∈
   flagged_years(...) : y < current }` (plan:261) drops ONLY `y >= current` from the prior-year
   contribution while `{current}` unconditionally supplies the current-year packet. So (a) a year `>=
   current` still being AUTHORED (or future-dated) never gets a premature Form 1040-X, and (b) NO
   genuinely-changed PRIOR year (`y < current`) is dropped — the filter keeps every `< current` member. It
   mirrors `promote_prior_year_advisory`'s own `years.retain(|y| *y < current)` (`conservative.rs:729`)
   exactly, and is a strict correctness IMPROVEMENT over the r4 (unfiltered) state.
2. **The reframed two-promote KAT's stated mutation is killable by its fixture.** The mutation is now
   "`flagged_years` iterates only ONE promote / last-promote-wins → the other year drops → reds"
   (plan:291-292). With two live promotes flagging DISJOINT prior years Y1≠Y2 and the assertion "BOTH ∈
   `plan_export.years`", a single-promote-only (or last-wins) implementation returns {Y1} or {Y2}, the
   missing year fails the assertion, and the mutant reds. The fixture is realizable (a straight extension of
   the same-step single-promote KAT), Y1,Y2 are prior (`< current`) so they survive the N-1 filter, and the
   plan honestly no longer claims this KAT discriminates union-vs-whole-state (unfalsifiable at leg-set
   altitude — documented, not asserted). This is exactly r4-M-1 fix option (a).
3. **The `session.rs:714` rewire is tax-neutral.** `:714`'s `crate::cmd::tranche::pre2025_tranche_exists(&all)`
   is a read-only refusal to OPEN the safe-harbor allocate flow (returns `CliError::Usage`, appends nothing).
   Rewiring the call target to `btctax_core::tranche_guard::pre2025_tranche_exists(&all)` invokes the SAME
   pure event-scan predicate → identical refusal → no filed number moves.
4. **Nothing in the fold introduced a wrong-filing path.** The KAT reframe is test-only; the `< current`
   filter only removes over-export of a not-yet-filed year (never a real prior-year 1040-X); the
   fifth-consumer rewire is a pure-predicate retarget. Every filing path remains behavior-preserving over
   the characterization-pinned shipped primitives.

Two NEW findings from fresh re-derivation are Nits (below): a plan-precision imprecision in the Task-5-Step-2
call-site labelling, and a display-vs-export asymmetry the N-1 fold left in `journey_view.flagged_years`.
Both are tax-neutral and non-gating.

## r4-resolution audit — M-1, M-2, N-1 all FOLDED CORRECTLY

- **r4 M-1 (two-promote KAT names an unkillable mutation) — FOLDED CORRECTLY (option a).** r4 found the KAT
  claimed a "single whole-state with-all-vs-without-all diff whose two promotes' per-year effects cancel"
  mutation that a DISJOINT-year (Y1≠Y2) construction cannot kill (with disjoint years a whole-state diff
  ALSO returns {Y1,Y2}, so the mutant survives). The fold (plan:290-295) restated the mutation as
  "`flagged_years` iterates only ONE promote / last-promote-wins → the other year drops → reds — this pins
  per-promote ITERATION," and added the honest NB: "disjoint years can't cancel, so this does NOT falsify
  union-vs-whole-state … the union is taken as the DFW-D11-mandated provably-safe superset — a same-year
  cancellation fixture is unrealizable at leg-set-equality altitude, so it is deliberately NOT asserted."
  Re-verified: the last-wins mutation IS realizable as a mutation of the union-over-promotes loop and IS
  killed by asserting BOTH years present (Confirm #2 above). The plan no longer overclaims. **Correct.**

- **r4 M-2 (C-2 move under-enumerates callers; fifth consumer `session.rs:714` unlisted) — FOLDED
  CORRECTLY.** The fold adds `session.rs:714` to the File Structure Map (plan:69-70, 332-334) and to Task 5
  Step 2 (plan:374-378: "★ arch-I-new-2/tax-M-2 the FIFTH, DIRECT consumer `session.rs:714`
  (`crate::cmd::tranche::pre2025_tranche_exists(&all)` → `btctax_core::tranche_guard::pre2025_tranche_exists(&all)`
  — rewire, do NOT leave a duplicate)"), and states "The four allocation APPEND sites … call the STAYING
  `guard_allocation_vs_tranche` and are preserved automatically (no rewire)." I independently re-grepped
  ALL callers of the three moved predicates across the workspace: exactly FIVE production sites —
  `tranche.rs:61,72` (`void_targets` inside the two moved predicates), `tranche.rs:94,111` (predicate calls
  inside the two staying guards), and `session.rs:714` (the direct fifth consumer) — and confirmed there is
  NO sixth. The four `guard_allocation_vs_tranche` append sites (`reconcile.rs:1015,1258`;
  `edit/persist.rs:1032,1105`) and the `lib.rs:27` re-export all reference the STAYING guard, unaffected.
  Enumeration now complete. **Correct.** (One residual labelling imprecision → Nit N-1 below; tax-neutral.)

- **r4 N-1 (`flagged_years` lacks the `< current` filter its mirror has) — FOLDED CORRECTLY.** The fold
  applies the filter in `plan_export`: "`plan_export.years` = `{current} ∪ { y ∈ flagged_years(...) : y <
  current }`" (plan:261) with the rationale "★ tax-N-1: the `< current` filter mirrors
  `promote_prior_year_advisory`'s prior-year filter (`conservative.rs:729`) so a year ≥ current still being
  AUTHORED is never emitted a premature 1040-X packet — `{current}` supplies the current year" (plan:264-266).
  Re-verified at source: `conservative.rs:729` is `years.retain(|y| *y < current);`, and its doc
  (`:716-719`) confirms the split — the advisory filters `< current` while the BG-D6 consent Σ deliberately
  does NOT (the realized-saving total must include the current year). The plan's export split is the exact
  analogue: prior-year 1040-X packets are `< current`; the current-year packet comes from `{current}`.
  **Correct** — and a strict correctness improvement. (A display-vs-export asymmetry the filter placement
  left behind → Nit N-2 below; tax-neutral.)

## Re-derived confirmation of the four mandated audit areas

### 1. The `< current` filter prevents a premature 1040-X while dropping no changed prior year

- **Filter placement is correct and self-consistent.** `flagged_years(events, state, prices, tables, cfg)`
  (plan:241-242) takes NO `current` param, so it CANNOT filter internally — it returns the raw union of
  per-promote fold-diff years. `plan_export(…, current_year: i32, …)` (plan:246-247) is the only surface
  with `current`, and it applies `{ y ∈ flagged_years : y < current }` then unions `{current}` (plan:261).
  The composition is exactly `{current} ∪ ({changed years} ∩ {< current})`.
- **No changed prior year is dropped.** The filter is `< current`; every changed year strictly below the
  current year survives. A `== current` change is absorbed by the unconditional `{current}` union (the
  current-year original return is always exported). The only years excluded are `> current`, which by
  construction have no already-filed return to amend — dropping them is CORRECT (you do not file a 1040-X
  for a year with no prior filing), and for this audience (historical no-records disposals) `> current`
  legs are out-of-scenario anyway. Even were one to occur, the direction is over-export-safe, never a wrong
  number.
- **Faithful to the mirrored source.** `promote_prior_year_advisory` gathers candidate years from BOTH
  folds' disposals∪removals (`conservative.rs:720-728`), retains `< current` (`:729`), then emits a line
  only where the per-year leg SET actually changed (`:755-761`). `flagged_years` uses the same fold-pair +
  leg-set-diff (plan:242, 265, 297-298); `plan_export` applies the same `< current`. **Confirmed.**

### 2. The reframed two-promote KAT's mutation is killable by its fixture

- **Union-over-promotes vs the named mutant.** `flagged_years` = ∪_{p ∈ live promotes} { y : project(ALL)
  vs project(ALL \ {p}) changes y's leg set } (plan:265, 297-298). The KAT fixture: P1 flags prior year Y1,
  P2 flags prior year Y2, Y1≠Y2, both `< current`. Correct code returns {Y1,Y2}. The mutant ("iterates only
  ONE promote / last-promote-wins") returns {Y1} or {Y2}; the missing year fails "BOTH ∈ plan_export.years"
  → reds. **Killable.**
- **Realizable.** Each promote independently flags its year via the per-`promote_id` marginal diff (removing
  P1 keeps P2, isolating P1's effect on Y1). Disjoint pools/vintages feeding disjoint prior-year removals
  make P1↔Y1 and P2↔Y2 independent — a direct extension of the same-step single-promote donation-reorder
  KAT (plan:286-288). Both years are prior, so they clear the N-1 `< current` filter; neither equals
  `current`, so the `{current}` union does not mask a drop.
- **The overclaim is gone.** The plan explicitly documents that this KAT does NOT falsify union-vs-whole-state
  (a whole-state diff also returns {Y1,Y2} for disjoint years) and that the union is the DFW-D11 safe
  superset, asserted only as per-promote ITERATION coverage. Honest and pinned. **Confirmed.**

### 3. The `session.rs:714` rewire is tax-neutral

- Re-verified `session.rs:702-724` (`safe_harbor_residue`): `:714`'s
  `if crate::cmd::tranche::pre2025_tranche_exists(&all)` is a READ-ONLY pre-flight refusal — when a pre-2025
  tranche is on file it returns `Err(CliError::Usage(…))` and opens no flow (the TUI opener surfaces the
  Err as its pre-flight status rather than showing a misleading residue; the CLI allocate path already
  refuses via `guard_allocation_vs_tranche` upstream). It APPENDS nothing and computes no filed number.
- Retargeting the call to `btctax_core::tranche_guard::pre2025_tranche_exists(&all)` invokes the SAME pure
  predicate (a non-voided `DeclareTranche` with `window_end < TRANSITION_DATE`, moved verbatim) over the
  same `&all` events → identical boolean → identical refusal. No behavior, no filed number, moves.
  **Tax-neutral.**

### 4. The r4→r5 fold introduces no wrong-filing path

- **KAT reframe (M-1):** test-only; touches no production code and no filing path.
- **`< current` filter (N-1):** removes only over-export of a `>= current` year; keeps every changed prior
  year and the unconditional current-year packet. Strictly improves correctness; no under-filing.
- **Fifth-consumer rewire (M-2):** pure-predicate retarget; identical refusal; no filed number.
- Every remaining filing path is untouched by this fold and remains behavior-preserving over the
  characterization-pinned shipped primitives (`consent_terms`, `filed_basis_for`,
  `promote_prior_year_advisory`, `append_decision`, `export_irs_pdf`/`export_full_return`). **Confirmed.**

## Findings

### Nit

- **N-1 (Task 5 Step 2, plan:371-376 — the predicate call-site labelling is imprecise; `:61,72` are inside
  the MOVED predicates, not "the two guards"; tax-neutral, arch-flavored).** The step says: "4 sites inside
  the two guards `guard_tranche_vs_allocation`/`guard_allocation_vs_tranche` (`:107,93` … only their
  internal predicate calls at `tranche.rs:61,72,94,111` rewire)." Re-grepped: `tranche.rs:94`
  (`pre2025_tranche_exists`) is inside `guard_allocation_vs_tranche` (`:93`) and `:111`
  (`in_force_allocation_exists`) is inside `guard_tranche_vs_allocation` (`:107`) — those two ARE the
  guards' internal calls. But `:61` and `:72` are `void_targets(events)` calls inside
  `in_force_allocation_exists` (`:54`) and `pre2025_tranche_exists` (`:71`) — the predicates BEING MOVED,
  not the guards. Since all three predicates (incl. `void_targets`) move to core as a unit, `:61,72` travel
  WITH their enclosing bodies and become intra-core sibling calls; they are not "rewired to
  `btctax_core::tranche_guard::*`" the way `:94,111,session:714` are. **Failure scenario:** none that
  reaches a filing — the mislabel is compiler-caught (any wrong reference breaks `make check` at Step 3) and
  the verbatim move naturally carries `:61,72` along. It is a wording/precision gap that could momentarily
  confuse an implementer about which sites "rewire" vs "move." **Tax-neutral / non-gating.** **Fix
  (optional, arch lens):** reword to "the two guards' internal predicate calls (`:94,111`) rewire to core;
  `void_targets`'s internal calls (`:61,72`) travel with the moved predicate bodies; plus the direct
  `session.rs:714` consumer rewires."

- **N-2 (Task 6 plan:416/456-457 vs Task 3 plan:261 — `journey_view.flagged_years` is UNFILTERED while
  `plan_export.years` filters `< current`; tax-neutral display-vs-export asymmetry the N-1 fold left
  behind).** `journey_view(events, state, prices, tables, cfg)` (plan:418) has NO `current_year` param, and
  its `flagged_years: BTreeSet<i32>` field (plan:416) is populated via the raw `flagged_years()` fn
  (plan:456-457) — so it carries EVERY fold-diff year, including any `>= current`. `plan_export` (which does
  have `current_year`) filters those out for the actual export set (plan:261). **Failure scenario:** a filer
  with a current-year no-records disposal whose promoted floor reorders that current year's legs would see
  `current` in `journey_view.flagged_years`; if the dashboard renders that field as "prior years to amend /
  file a 1040-X for," a `>= current` member is mislabeled as a prior-year amendment — the very mislabel
  `conservative.rs:729`'s `< current` filter exists to prevent. **No wrong filing flows:** the ACTUAL export
  uses the filtered `plan_export.years` (Task 10 Step 1 KAT (a), plan:557-558), not the view field; and for
  this audience the affected years are `<= current` with a `== current` member being a legitimate
  original-return change, so it is practically inert. **Tax-neutral / non-gating.** **Fix (optional):**
  either have the dashboard render `journey_view.flagged_years` with year-agnostic "affected filings" copy
  (reserving "prior-year 1040-X" language for `< current` members), or filter the display field to `< current`
  for amend-labelled rows — mirroring `conservative.rs:729`'s rationale; note in Task 6/7 which is intended.

## Confirmed correct (adversarially re-checked, no finding)

- **No new tax logic; every filed number flows through shipped primitives.** `flagged_years` is the only
  candidate for new logic, and it replicates the SHIPPED `promote_prior_year_advisory` fold-pair
  (`with = project(ALL)`, `without = project(ALL \ {promote_id})`, `conservative.rs:701-707`) + per-year
  leg-set Vec-eq diff (`:755-761`) exactly, returning a typed `BTreeSet<i32>` instead of `Vec<String>` — no
  new tax computation, no `Blocker.detail` string-parse re-entry (Task 6 Step 3 explicitly uses the typed
  fn, plan:456-457). **Confirmed.**
- **DFW-D6 pseudo-off remains the ONE intended behavior change, correctly scoped.** `plan_promote` forces
  `cfg.pseudo_reconcile=false` on its own copy before `consent_terms`/`promote_prior_year_advisory`/
  `gift_only_flagged_years` (plan:154-156), mirroring `would_conflict`. Untouched by the r4→r5 fold; the
  fold changes no other behavior. **Confirmed.**
- **Over-coverage stays a derived dashboard ADVISORY (DFW-D5.3).** `OverCovered{by_sat}` is scoped to
  `covered_sat>0 ∧ live_sat>covered_sat` with the fully-undisposed carve, derived read-only in
  `journey_view`; no guard added to the shared promote chokepoint. Untouched by this fold. **Confirmed.**
- **`Acknowledgment.shown_terms` byte-identical CLI vs TUI.** Both surfaces drive the SAME
  `plan_promote`(pseudo-forced-off) → `render_consent` → `apply_promote`; the Task 4 full-driver parity
  harness compares recorded artifacts across both driver paths. Re-verified the shipped print order
  (advisory `:443` → `render_consent(&terms,&gift_only)` `:453` → `wide_window_note` `:454`) and
  `shown_terms: terms` `:468`. Untouched by this fold. **Confirmed.**
- **The six sat-carrying `UncoveredDisposal` emit sites are exactly `:388,710,831,876,1196,1274`.**
  Re-grepped: `UncoveredDisposal` appears at fifteen `fold.rs` lines; the plan's six are the sat-carrying
  subset (the other nine are the without-wallet / degenerate variants routed to DataFix per DFW-D4). No
  drift since r4. **Confirmed.**
- **`admin.rs` export dispatch anchors are stable.** `export_irs_pdf` `:350` / its `Session::open` `:358` /
  `return_inputs::exists` dispatch `:373` / crypto-slice `IrsPdfReport` `:578` / `export_full_return(&Session)`
  `:642` / `IrsPdfReport` struct `:261` — all match the plan's citations. The `&Session` inner extraction
  (arch-C-1) and the once-inside dispatch remain behavior-preserving over both arms. Untouched by this fold.
  **Confirmed.**
- **`method_inversion_advisory`/`tranche_dip_advisory` surfaced verbatim, no filed-number path.**
  `tranche_dip_advisory` (`conservative.rs:27`) and `method_inversion_advisory` (`:61`) are the pure,
  provenance-neutral, never-instruct-a-tax-understating-action builders carried into
  `Advisory::{TrancheDip,MethodInversion}(String)`. **Confirmed.**
