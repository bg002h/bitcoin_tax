# Whole-branch review — Sub-project C (rate-aware optimizer) — round 1

**Scope.** Final whole-diff review, the cross-cutting net over all 11 C tasks (the last sub-project of
A→B→C). Diff `review-01e7f72..50b0737.diff` (18 commits). Reviewed against
`SPEC_lot_optimization_program.md` (Sub-project C + Legal grounding + Cross-cutting),
`IMPLEMENTATION_PLAN_optimizer.md` (R0 folds + C1/C2/C3 + R2-C1/R2-I1 invariants),
`progress.md` (C section), and `FOLLOWUPS.md`. Workspace gate reported GREEN (423 tests, clippy -D,
fmt, release, PII clean) — not re-run; code/diff reviewed.

**Reviewer.** Independent (author ≠ reviewer). Persisted before any fold.

---

## VERDICT: READY TO MERGE — 0 Critical / 0 Important.

The headline ("pay the least tax the law permits and regulation does not forbid") holds end-to-end in
every production-reachable path. No path renders a non-global as the optimum without the banner; no path
labels or persists a post-hoc/divergent pick as compliant; `run`/`consult` mutate nothing; `delta ≤ 0`
always; the optimizer matches the independent oracle on every optimality KAT. New findings are 3 Minor +
triage below, all DEFER (one with a future-cycle gating caveat).

---

## Headline dimensions — findings

### 1. Optimality + honesty — SOUND
- `approximate == false` is **strictly** "fully enumerated + exhaustively scored = proven global" across
  all three paths. `optimize_year` (optimize.rs:692-921) sets `approximate=true` for **any** of:
  product > `MAX_COMBOS` (coordinate descent), a contended group beyond `GROUP_COMBO_BOUND`
  (`contended_unenum>0`), or any pool > `LOT_ENUM_BOUND` (`pool_heuristic_lots.is_some()`,
  optimize.rs:777-779). Precedence ComboCapExceeded > ContentionUnenumerated > PoolHeuristic
  (optimize.rs:821-837). `approximate ⇔ approx_reason.is_some()` holds (each true-setter has a matching
  reason arm), so the renderer's `None` arm is dead/defensive.
- **`delta ≤ 0` always.** The search is baseline-seeded (`exhaustive_min`/`coordinate_descent` start the
  incumbent at `base.total_federal_tax_attributable`, optimize.rs:962-963, 1019-1020) and only evicts on a
  strict improvement (or a lex-smaller exact tie that keeps `best_total`). Crucially `optimized_tax` uses
  the tracked `best_total`, **not** a re-fold of `best` (optimize.rs:908-915) — this is the right call: a
  re-fold re-injects picks in lot-id order vs the original FIFO order and can shift a pro-rata remainder
  cent between an ST and an LT leg, which would push `opt.total` a cent above `base.total` and break the
  `≤ 0` invariant on a multi-leg `best==baseline` row. (`marginal_rates` still comes from the re-fold; it
  is descriptive and a sub-cent shift never changes the bucket — acceptable.)
- **Oracle agreement.** Every Mode-1 optimality KAT asserts `optimized_tax == oracle_min_total` against a
  genuinely **independent** exhaustive oracle (optimize_mode1.rs:183-238 — enumerates whole-lot subsets,
  cartesian product, scores via `score_assignment`; no optimizer generation code) AND beats the named
  naive baseline: HIFO-beats-FIFO, rate-aware ST/LT (HIFO loses to a 15% LT pick), loss-harvest, per-wallet
  exclusion, contention joint optimum. Both PoolHeuristic directions, ComboCapExceeded, and
  ContentionUnenumerated are pinned, including the mirror `small_pool_is_not_approximate` so
  `approximate==false ⇔ fully-enumerated-global` is nailed both ways.
- **Renderer honesty** (render.rs:676-781): "NOTHING is filed or bound by running this"; when approximate,
  a `⚠ APPROXIMATE — NOT a guaranteed global minimum` banner + the specific reason + "NOT 'the least
  tax.'"; when false, no banner ("optimized federal tax" is then the proven optimum). Footer reinforces
  "adequate ID must exist by the time of sale — §1.1012-1(j)". No false-global claim anywhere.

### 2. §1.1012-1(j) compliance integrity — SOUND
- `proposed_compliance_status` (optimize.rs:467-486) judges a proposed pick by **its own** made-date; a
  standing order can never rescue a divergent post-hoc pick (`proposed==current` is the only path that may
  report `StandingOrder`). 2027+ broker → `NonCompliant` first.
- `compliance_overlay` (optimize.rs:494-512) upgrades `NonCompliant → AttestedRecording` only when
  `attested ∧ unchanged ∧ ¬(broker∧≥2027)` — R2-I1: an attestation binds **only** the exact persisted+
  attested selection (`unchanged` = `proposed==current`), never a divergent re-run pick. Verified by
  `accept_then_divergent_baseline_stays_noncompliant` and the side-table integration test.
- **Atomic accept co-persist** (cmd/optimize.rs:246-267): the `LotSelection` decision and the attestation
  row land in the same in-memory conn, flushed by a single `session.save()`; any early `?` discards both
  (no partial write). So baseline==attested==persisted, and R2-I1 holds on the next run.
- **Void closes the post-void mislabel hole** (cmd/reconcile.rs:109-138): voiding a `LotSelection` also
  clears its `optimize_attestation` row atomically. This is load-bearing — without it, a stale attestation
  row + a re-run where the FIFO default equals the proposal (`proposed==current`, `unchanged`) would launder
  a plain FIFO disposal to `AttestedRecording`. KAT `void_clears_attestation_row_prevents_mislabel...`.
- **2027+ broker categorically refused** in `accept` (cmd/optimize.rs:230-239) even with `--attest`
  (`accept_refuses_2027_broker_held_even_with_attestation`). Blanket-attest rejected above the loop
  (cmd/optimize.rs:188-193) before any append.
- A's `disposal_compliance` (compliance.rs) is used only for the BASELINE status of a `proposed==current`
  row; its classifier independently fires 2027+→NonCompliant first and never lets a standing order rescue an
  applied post-hoc selection. Consistent with C's overlay.

### 3. Read-only invariants — SOUND
`run` and `consult` (cmd/optimize.rs:35-67, 99-122) use `Session::open` and never call `save()`/`append`.
`consult_sale` and its helpers (`fold_as_of`/`state_as_of`/`synthetic_state`) are clone-fold-discard over
borrowed events. KATs assert event-count/event-log byte-identical before/after consult and run. Only
`accept` writes, and only the atomic co-persist.

### 4. Cross-task seams — SOUND
- `fold::pools_before` and `fold::state_as_of` (fold.rs:388-486, +116/-0, no change to `fold`) reuse the
  real `seed_transition` and mirror `fold`'s exact ordering (`sort_canonical` + stable tax-date partition).
  The §7.4 seed fires at the correct boundary under **both** Path A (residue relocated) and Path B (residue
  discarded, allocation lots installed) **including when the target is the first ≥2025 timeline event** —
  KAT'd `available_lots_before_path_b_first_2025_disposal_returns_seeded_lots` (+ Path-A counterpart). The
  available-lots-match-real-fold property holds.
- `state_as_of` uses `continue` (not `break`) so mixed-per-event-tz timelines (utc-ascending ≠
  date-ascending) fold every `date()≤at` event; the boundary seed is forced before `finalize` when `at` is
  post-transition but no real ≥2025 event preceded it. Correct.
- `LotPick: Ord` is purely additive (event.rs:190, +1/-1 derive only; serde unchanged).
- attest side-table ↔ overlay binding is correct (attested_set → overlay `attested`; `unchanged` →
  gate; void → clear).

### 5. Determinism (NFR4) + exact arithmetic (NFR5) — SOUND
No `f32`/`f64`, no `HashMap`/`HashSet` anywhere in the optimize path (grep clean). All money `Decimal`,
sats `i64`; HIFO heuristic uses `usd_basis * Usd::from(sat)` (Decimal). All containers BTreeMap/BTreeSet/
sorted Vec; tie-break is the §0 total order (total, then lex assignment). Core is clock-free (made-date
threaded from the CLI seam). Federal-only. Synthetic fixtures + temp vaults only. §1091 module doc is
legally correct (crypto exempt; Notice 2014-21 property; Rev. Rul. 2023-14 reframed as a general
reaffirmation, NOT §1091 authority; no enacted statute; monitor) with the freely-selectable-loss KAT.

### 6. Backward-compat — SOUND
C is additive over shipped A+B: `evaluate.rs`/`resolve.rs`/`state.rs`/`project/mod.rs` have **zero**
changes; `fold.rs` +116/-0 (two new pub fns, `fold` untouched); `event.rs` +1/-1 (Ord derive);
core `lib.rs` +5 (re-exports). CLI changes are new modules + dispatch + an additive void behavior. A's
substrate and B's engine are intact.

---

## New findings the per-task reviews missed (all Minor, all DEFER)

**M-1 (Minor) — exact-tie tie-break can emit (and, in non-prod-reachable cases, auto-persist) a
`delta == 0` divergent selection.** In `exhaustive_min` (optimize.rs:980) a candidate that *ties* the
baseline total but is lexicographically smaller than `baseline_assignment` evicts the baseline incumbent
(`best_total` stays `== base.total`). The result is `best != baseline_assignment` with `delta == 0`, so a
disposal with two equal-basis/equal-term lots can yield `proposed != current` at zero tax benefit. Effect:
`run` shows a "change … needs `--attest`" line for no benefit; and a future-dated (`made≤sale`) disposal
would let a bare `accept` auto-persist a no-benefit divergent `LotSelection` as `Contemporaneous`. **No
invariant is broken** — `delta=0` is shown ("always ≤ 0"), the pick is gated/legally valid, the reported
optimum is still a true minimum. It is needless churn / a pointless attestation prompt. The lex-smallest
tie-break is the spec'd §0 total order, so this is a quality choice, not a correctness bug. *Recommend*
preferring the baseline on an exact tie (only evict on `total < best_total`). **DEFER** (non-blocking).

**M-2 (Minor) — Mode-2 `consult_sale` silently discards the `candidate_selections` heuristic flag.**
optimize.rs:1135 binds `let (cands, _heuristic) = candidate_selections(&lots, req.sell_sat)`. For a wallet
pool > `LOT_ENUM_BOUND` (12) — common for weekly-DCA / active-trading wallets — the candidate set is a
deterministic *incomplete* subset, so the "proposed selection" may not be the true tax-minimum, with **no
disclosure**, unlike Mode-1's `PoolHeuristic` banner. Mitigation: `ConsultReport` has no `approximate`
field and the renderer hedges ("read-only what-if", "proposed selection", "federal tax attributable
(estimated)") rather than claiming "the optimum" — so it is *not* a false-global claim, which is why this
is Minor, not Critical. The plan explicitly scoped R2-C1's disclosure to Mode-1. *Recommend* a parallel
"heuristic — searched a subset of a large pool" note in `render_consult` for symmetry. **DEFER**.

**M-3 (Minor) — the optimizer's search space excludes self-transfer lot-selection; the
`approximate==false` "global" claim is global over taxable-disposal selections only, undocumented.**
`optimize_year` targets only `baseline_state.disposals` (optimize.rs:713-725); SelfTransfers produce no
Disposal/Removal record, so a same-year self-transfer's lot routing is held at its baseline. Spec §A.3
lists SelfTransfer as method-honoring and says it "lets the optimizer pre-position lots," so a user could
read "proven global minimum" as including self-transfer re-routing. In practice the available-lots pools
are still *correct* (the real fold, incl. self-transfers at baseline, is replayed), and self-transfers are
non-taxable so they affect the single-year objective only indirectly via an uncommon intra-year
move-then-sell pattern — hence Minor, not Important, and arguably out of single-year specific-ID scope. (A's
own whole-branch review already left the sibling `disposal_compliance`-omits-SelfTransfers item OPEN.)
*Recommend* documenting the scope boundary in the proposal footer (mirroring the R0-M2 vertex-granularity
caveat). **DEFER**.

*(Observation, not a finding.)* `ConsultReport.total_federal_tax_attributable` is the **whole year's**
crypto-attributable tax with the synthetic sale added (existing in-year disposals kept at current ID), not
the marginal tax of the contemplated sale alone. This matches B's definition and the holistic objective
(and the `saving_if_waited` timing delta is a correct marginal), and the renderer labels it "(estimated)".
Defensible/intended; noted only for clarity.

---

## Triage of recorded C-section Minors / Nits

- **Task-4 Nit — `persistability`/`proposed_compliance_status` asymmetry for a divergent *contemporaneous*
  (`made≤sale`) 2027+ broker pick** (FOLLOWUPS.md:364-380). For that input `persistability` returns
  `ContemporaneousNow` (so `accept` would persist it as "Contemporaneous") while `proposed_compliance_status`
  returns `NonCompliant` — the two core functions disagree, and an own-books "Contemporaneous" record is
  insufficient for a 2027+ broker. **DEFER for this branch — UNREACHABLE in production:** `BundledTaxTables`
  is TY2025 only, so any 2027 year is `TaxYearNotComputable` and `optimize_year` returns before the
  per-disposal gate; the only computable year (2025) can never hit the `≥2027` branch. **MUST-FIX before any
  TY2027+ table is bundled** — at that point the path goes live and would persist a legally-insufficient
  own-books record as compliant. The one-line alignment (give `Persistability` a `ForbiddenBroker2027` arm
  reached on `made≤sale` too, OR widen `proposed_compliance_status`) is cheap; recommend doing it now to
  retire the latent risk rather than carrying a release-gating caveat.
- **C.5 wash-sale monitor** (FOLLOWUPS.md:7-24): **DEFER** — correctly documented (exempt), KAT present,
  lockstep guard noted; no code action until §1091 is enacted.
- **Task-3 IMPORTANT — `available_lots_before` Path-B first-2025-disposal**: **RESOLVED** (verified in code
  + two KATs); not open.
- **Task-9 nits — dead `None` arm in `render_optimize_proposal`; `ContentionUnenumerated` human string omits
  combos/cap**: **DEFER** — the `None` arm is unreachable (`approximate ⇔ approx_reason.is_some()`); the
  `run` eprintln logs the full `{:?}` reason (combos/cap included).
- **Task-5 nit — no broker-2027+ E2E through `optimize_year`**: **DEFER** — covered by direct-call KATs +
  the CLI `accept` 2027 E2E.
- **Task-11 nit — dead test-assertion OR branch**: **DEFER** (test-only).
- **A whole-branch M1 — `disposal_compliance` omits method-honoring SelfTransfers** (related to M-3 above):
  **DEFER** — consistent with C scoping to taxable disposals; document.

---

## Bottom line
Sub-project C is **ready to merge: 0 Critical / 0 Important.** The optimality, honesty, compliance,
read-only, determinism/exactness, cross-task-seam, and backward-compat invariants all hold in
production-reachable paths, backed by independent-oracle optimality KATs and the full compliance-lifecycle
(accept→run→void) test suite. The three new Minors and the triaged Nits are non-blocking; the **only item
warranting a follow-up before the next cycle ships a TY2027+ tax table** is the Task-4
persistability/status asymmetry (latent today, live the moment a 2027 table is bundled).
