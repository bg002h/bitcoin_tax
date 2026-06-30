# R0 architect review — IMPLEMENTATION_PLAN_optimizer.md (Sub-project C), round 1

**Artifact:** `design/IMPLEMENTATION_PLAN_optimizer.md`
**Contract:** `design/SPEC_lot_optimization_program.md` (Sub-project C + Cross-cutting + Legal grounding)
**Grounded against shipped A+B source** (re-read 2026-06-30):
`crates/btctax-core/src/project/{evaluate.rs,compliance.rs,fold.rs,resolve.rs,pools.rs}`,
`event.rs`, `state.rs`, `identity.rs`, `conventions.rs`, `tax/{compute.rs,types.rs}`;
`crates/btctax-adapters/src/tax_tables.rs`;
`crates/btctax-cli/src/{session.rs,tax_profile.rs,eventref.rs,render.rs,main.rs,lib.rs}`,
`crates/btctax-core/src/persistence.rs`.

**Verdict: NOT GREEN. 3 Critical, 4 Important, 5 Minor, 2 Nit.** Must reach 0C/0I before code.

The algorithmic skeleton is largely sound — the vertex argument is correct, determinism intent is
right, the feasibility-by-scoring self-elimination is genuinely airtight against shipped source, and
the Mode-2 read-only contract holds. The blocking problems are all in the **honesty surface**: the
tool can present a non-global result *and* a post-hoc/divergent pick as "the least tax" / "compliant"
without disclosure. Those are exactly the failure modes the headline forbids.

---

## What I verified as CORRECT (so it isn't re-litigated)

- **Vertex sufficiency (1a) — sound.** `make_disposal_legs` allocates proceeds **pro-rata by sat**
  with remainder-takes-the-rest (`fold.rs:106-182`), so proceeds-per-sat (`total_net_proceeds / need`)
  is a constant **independent of lot choice**; each lot's (and each dual-basis zone's) per-sat gain is
  therefore a fixed constant, and each disposal's `(ST,LT)` contribution is **linear** in the per-lot
  sats over the box-capped simplex `{Σx=N, 0≤x≤r}`. `compute_tax_year` nets all year disposals into a
  single `(crypto_st, crypto_lt)` (`compute.rs:276-287`), so the year tax depends only on the **summed**
  `(ST,LT)`. The achievable region is the Minkowski sum of per-disposal convex polygons; its vertices
  are sums of per-disposal vertices, and per-disposal vertices are exactly "subset whole + ≤1 partial."
  `candidate_selections` enumerates that set **completely** for n≤12. The tax function is **piecewise
  linear**, so the only optima not at a polytope vertex sit on a tax-kink hyperplane (limitation (i)) —
  the plan's characterization is accurate. **Caveat: completeness holds only for non-contended
  disposals — see C3.**
- **Feasibility-by-scoring (5) — sound.** An infeasible injected selection (cross-wallet / unknown /
  over-draw) trips `selection_feasible` (`pools.rs:107-153`) → `consume` falls back to method order and
  sets `selection_error` (`pools.rs:79-94`) → `consume_principal` raises hard `LotSelectionInvalid`
  (`fold.rs:62-66`) → `first_hard_blocker` returns it (`compute.rs:415-420`, gates on **any**
  `severity()==Hard` anywhere) → `compute_tax_year` returns `NotComputable` (`compute.rs:235-249`) →
  `score_assignment` returns `NotComputable` → `exhaustive_min` **skips it**. `LotSelectionInvalid` is
  Hard (`state.rs:39,68`). Cross-disposal contention manifests as the later disposal over-drawing the
  depleted pool, which self-eliminates the same way. **An infeasible assignment can never be selected.**
- **Mode-2 side-effect-free (3) — confirmed.** `evaluate_disposal` borrows read-only and
  clone-fold-discards (`evaluate.rs:98-213`); `existing_event=None` appends a synthetic `Op::Dispose`
  (`evaluate.rs:139-153`). `consult_sale` only ever clone-folds; `cmd::optimize::consult` never calls
  `save()` and writes no side-table. `compute_tax_year` ignores `events` (`compute.rs:228`) and reads
  `state.disposals` (which includes the synthetic), so the synthetic-not-in-`events` is fine.
- **I6 refusal — confirmed.** `compute.rs:235-249` gates every year on any Hard blocker; `optimize_year`
  refuses up front when the baseline is `NotComputable` (plan line 489-492).
- **Source signatures** — `resolve`/`Resolution.{timeline,selections}` pub (`resolve.rs:128-136,269`),
  `fold` pub (`fold.rs:330`), `compute_tax_year`/`TaxOutcome`/`TaxResult`/`MarginalRates`
  (`compute.rs:221`, `tax/types.rs:54-96`), `disposal_compliance`/`ComplianceStatus`/`DisposalCompliance`
  (`compliance.rs:18-33,76`), `evaluate_disposal`/`CandidateDisposal`/`EvaluateError`
  (`evaluate.rs:31-104`), `append_decision` (`persistence.rs:238-262`), `BundledTaxTables::load()`
  returns `Self` (`tax_tables.rs:48`), side-table pattern (`tax_profile.rs:16-81`), Session accessors
  (`session.rs:41-112`), `now` seam (`main.rs:294,497`), `wallet_label` (`render.rs:107`), CLI parsers
  (`eventref.rs`), `CliError` (`lib.rs:16-42`) — **all accurate.** §1091 note is accurate.

---

## CRITICAL

### C1 — Oversized-input coordinate-descent fallback returns a **silent local optimum** presented as "the least tax" (no disclosure, no cap log, may worsen baseline).
Plan §4 (line 123) and Task 4 (lines 524-528, 588-593): beyond `MAX_COMBOS = 50_000` the search
falls back to `coordinate_descent` ("start all-HIFO; … iterate to a fixed point"). Coordinate descent
on this non-separable objective (the §1(h) breakpoints, §1211 cap, §1222 cross-netting couple all
disposals) converges to a **local** optimum, not the global minimum. Yet:
- `OptimizeProposal` (lines 188-196) has **no "approximate/non-guaranteed" field**;
- `render_optimize_proposal` (Task 9, lines 916-935) prints `optimized_tax` / `delta` **unconditionally**
  as the optimum;
- nothing **logs that the cap was hit**.

The review brief is explicit: a silent local optimum = Critical; the plan MUST flag the result as
approximate AND log the cap. It does neither. Worse, the sketch seeds the **exhaustive** search with
the baseline score (line 583) but the **coordinate-descent** sketch does not — if all-HIFO and its
local basin are worse than the baseline identification, `optimized_tax > baseline_tax` and `delta > 0`,
violating the documented invariant `delta ≤ 0` (line 193) and presenting a result **worse than doing
nothing** as "optimized."

**Fix (all required):** (a) add an `optimality: Exact | Approximate { reason }` (or equivalent) field
to `OptimizeProposal`, set `Approximate` whenever the coordinate-descent path runs; (b) render a loud
"APPROXIMATE — not guaranteed to be the global minimum (input exceeded the exhaustive bound)" banner
and remove the bare "optimum" framing on that path; (c) log the cap hit (combos vs `MAX_COMBOS`);
(d) seed coordinate descent's incumbent with the **baseline assignment** (and each disposal's
`baseline_selection`) so `optimized_tax ≤ baseline_tax` always; (e) a KAT that drives the fallback and
asserts the Approximate flag + `delta ≤ 0`.

### C2 — Mode-1 `run` proposal **misrepresents the compliance and persistability of the proposed selection** (post-hoc / standing-order-divergent picks shown as compliant/persistable).
Two independent defects, both surfaced in the primary user-facing `run` output:

**(a) `status` is computed from `events`, not from the proposed pick.** Task 4 (lines 538-548) builds
each row's `status` from `disposal_compliance(events, &opt_state)`. But the proposed selection is
injected into the **fold** (`res.selections`, `fold_with`, lines 297-301) — it is **not** a persisted
`LotSelection` in `events`. `disposal_compliance` reads `events` for `sel_made` (`compliance.rs:98-119`);
in a pure what-if that map is **empty**, so the classifier **skips the selection branch (step 2)** and
falls through to **`StandingOrder`** when any `MethodElection` is in force (`compliance.rs:144-165`).
Result: when the optimizer proposes a pick that **diverges** from the in-force standing order (entirely
expected — the spec notes the HIFO standing order is zone-blind and C's scored optimum may legitimately
differ, A.3/M1), the proposal labels that divergent post-hoc cherry-pick **`StandingOrder` = compliant**,
even though the standing order would consume *different* lots. That is precisely the forbidden
"post-hoc selection described as compliant" (Cross-cutting; §0 line 20). The brief's requirement
"confirm the overlay can't mark a post-hoc pick `Contemporaneous`/`StandingOrder`" is **not met**: the
overlay itself is fine (NonCompliant→AttestedRecording only), but the **pipeline feeding it** returns
`StandingOrder` for a divergent pick. (The Task-4 comment "status reflects the proposed selections" is
false — see N1.)

**(b) `persistable` is `ContemporaneousNow` for *every* disposal.** Task 4 line 549 / Task 5 line 668
set `persistable = persistability(wallet, date, date)`. With `selection_made == sale_date`,
`persistability` (lines 635-643) always returns `ContemporaneousNow`. So `render_optimize_proposal`
prints "persistable now (made ≤ sale → Contemporaneous)" for **every** disposal — including the common
Mode-1 case of an already-executed 2025 self-custody disposal that is actually post-hoc
(`NeedsAttestation`). The plan calls `date` a "conservative stand-in," but `ContemporaneousNow` is the
**least** conservative verdict (see N2). The user reads the `run` output to decide whether to accept;
it tells them post-hoc picks are contemporaneously persistable. (`accept` recomputes correctly with the
real `now`, but the proposal the user acts on is wrong.)

**Fix:** compute each row's `status`/`persistable` against the **actual proposed selection** and a
**truthful** posture: a proposed pick that is not already a persisted contemporaneous selection and not
identical to the in-force standing order's output must render as a *would-be* selection classified by
`persistability` against a real/unknown made-date — never `StandingOrder`/`ContemporaneousNow` by
fall-through. Acceptable shapes: (i) thread the real `now`/`made` into core so `persistable` is honest;
(ii) gate `StandingOrder` on `proposed_selection == standing-order-dictated selection`; (iii) otherwise
mark "pending — not bound by running this; see `accept`." Add KATs: divergent-from-standing-order pick
is **not** `StandingOrder`; already-executed disposal is **not** `ContemporaneousNow` in the proposal.

### C3 — **Contended same-wallet disposals** yield an under-generated candidate space → a **non-optimal result presented as the optimum**, with no output disclosure.
`available_lots_before(D)` clone-folds the timeline truncated before `D` **using the baseline
(method/persisted) selections of earlier disposals** (Task 3, lines 359-379) — it never considers the
injected candidate for an earlier same-pool disposal. So for two disposals `D1`,`D2` drawing from
overlapping lots in one wallet, `D2`'s candidate set is generated against the **post-`D1`-baseline**
pool. The cartesian product therefore explores `{D1 vertices} × {D2 vertices feasible after D1-baseline}`
and **misses** optima where `D1` deviates from baseline in a way that frees a lot `D2` should take.
This is realistic and material: e.g. two sells from one wallet in the same year with a lot that is ST at
`D1`'s date but LT at `D2`'s date — assigning it to `D2` (LT, taxed at 15%) vs `D1` (ST, ordinary) can
be the optimum, and that reassignment is exactly what the per-disposal-independent generation drops.
Plan limitation (ii) (line 127) frames this as "at worst a feasible candidate is missed" and files it
to `FOLLOWUPS.md` — but `optimized_tax` is still rendered as **the optimum** with no caveat. A
non-optimal result presented as optimal is the Critical the headline forbids.

**Fix (one of):** (a) joint enumeration over contended same-wallet/same-pool disposals (generate `D2`'s
candidates against each `D1` candidate's resulting pool); or, at minimum, (b) **detect contention**
(≥2 method-honoring disposals in the same post-2025 pool whose available-lot sets overlap) and mark the
proposal `Approximate { reason: contended-disposals }` with a rendered disclosure — same honesty
mechanism as C1. Silent non-optimality is not acceptable; (b) is the floor, (a) is the real fix. Add a
KAT with two contended intra-year sells across an ST/LT crossover asserting either the true optimum is
found (a) or the Approximate flag is set (b).

---

## IMPORTANT

### I1 — `available_lots_before` truncates the timeline in **unsorted build order**, so it can compute the wrong "lots available before the disposal."
`resolve` builds `Resolution.timeline` in **`events`/DB order** (`resolve.rs:493-518`); it does **not**
sort. `fold` sorts canonically and stable-partitions by transition side **inside** `fold`
(`fold.rs:335-341`). But Task 3's `available_lots_before` does
`res.timeline.iter().position(|e| &e.id == disposal)` then `res.timeline.truncate(idx)` **before** any
sort (lines 367-371). `position`/`truncate` therefore operate on **load order**, not time order: the
truncation keeps an arbitrary subset (events appearing before `D` in the DB), which fold then re-sorts.
A lot acquired before `D` in time but loaded after `D` is **dropped** from the available set
(→ missed candidate → non-optimal); a not-yet-acquired lot loaded earlier is **included**
(→ extra candidate that later self-eliminates — wasteful but safe). The shipped A.6 path
(`evaluate.rs`) doesn't hit this because it appends and folds the **whole** timeline; this is new code.

**Fix:** `sort_canonical(&mut res.timeline)` **and** apply fold's stable pre-2025 partition
(`fold.rs:341`) before locating/truncating the disposal — or compute available lots by folding the full
timeline and reading the pre-disposal pool a different way. **The Task-3/Task-4 KATs must use fixtures
where load order ≠ canonical order**, else this bug ships green.

### I2 — `LotPick` does not derive `Ord`/`PartialOrd`; the determinism/dedup machinery requires it.
`event.rs:190` derives only `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. The plan relies on
`Vec<LotPick>: Ord` in multiple load-bearing spots: `candidate_selections` keys a
`BTreeSet<Vec<LotPick>>` (Task 3 line 385); `consult_sale` compares `(total, &Vec<LotPick>)`
tuples (Task 6 line 740); `exhaustive_min`'s tie-break is "lexicographically smallest assignment"
over `BTreeMap<EventId, Vec<LotPick>>` (§0 line 17, Task 4 line 583). None compiles without
`LotPick: Ord`. Yet the plan asserts it "reuses `LotPick` … verbatim" with "no A-side visibility
change required" (lines 66, 320, 1133).

**Fix:** add `PartialOrd, Ord` to `LotPick`'s derive (cheap — `LotId: Ord` `identity.rs:116`,
`Sat=i64`), as an explicit task, and confirm it doesn't perturb A's serde or any existing `LotPick`
ordering assumption; **or** restructure every key to a `Vec<(LotId, Sat)>` (already `Ord`). Either way,
drop the "no A-side change / reuse verbatim" claim.

### I3 — The loss-harvest-within-$3k optimality KAT is **unsound under the stated single-year objective**.
Task 4 (line 607) asserts the optimum "harvests only enough loss to use the $3,000 §1211 offset" and
that `carryforward_out`/`loss_deduction` match a specific $3k-capped figure. But the objective is
`total_federal_tax_attributable` for **one year** only, and `net_1222` caps the current deduction at
`loss_limit` (`compute.rs:174-178`): once a selection offsets all gains and takes the $3k, **any**
additional realized loss only grows `carryforward_out` — `total_federal_tax_attributable` is
**identical**. So "harvest exactly enough" and "over-harvest" are **ties** on the objective, broken
**lexicographically** (§0 line 17), *not* by minimal harvest. The optimizer may legitimately return an
over-harvesting selection (different, larger `carryforward_out`), and the KAT's hand-derived figure
will not match. The spec's "over-harvesting wastes high-basis lots" intuition is a **multi-year** effect
the single-year objective does not price.

**Fix:** either (a) make the KAT assert only what the objective guarantees (`optimized_tax ==
oracle_min_total` and full offset of gains + $3k), and add an explicit note that the objective is
indifferent to carryforward/basis-preservation; or (b) if "harvest only enough" is a real requirement,
add a documented **secondary** objective (minimize unnecessary realization / maximize retained basis)
into the tie-break — but that is a scope change and must be specced. Also: because the tool can
over-realize losses with no current benefit and present it as "optimal," the carryforward-blindness of
the objective should be **disclosed** in output or docs.

### I4 — Mode-2 ST→LT timing insight re-scores in the **crossover year**, which lacks a bundled table and a matching profile → consult **errors** instead of degrading, and the comparison is cross-year/cross-profile.
`timing_insight` (Task 6 lines 747, 772) re-scores "the same selection with `candidate.date =
latest_crossover`" to get `tax_if_sold_long_term`. `score_synthetic` runs `compute_tax_year(year =
latest_crossover.year())`. Only **TY2025** is bundled (`tax_tables.rs:48-54`); a crossover into 2026+
→ `TaxTableMissing` → `NotComputable` → `timing_insight` returns `Err` → the **whole `consult_sale`
fails** rather than omitting the timing line. Separately, the profile passed is for `at.year()`
(plan line 696, 770), so even when a table exists the LT re-score mixes `at`-year profile with a
different year's brackets — `saving_if_waited` is not a clean apples-to-apples delta, and moving
`candidate.date` also re-terms *other* lots in the selection.

**Fix:** compute `tax_if_sold_long_term` by holding the **same tax year and profile** fixed and forcing
only the relevant lots' term to LT (term flip), not by relocating the disposal date into a future year;
and if any required table/profile is genuinely unavailable, **degrade** (omit `timing`) rather than
error the consult. KAT: a crossover that would land in an unbundled year still yields a valid
`ConsultReport` (timing omitted or computed term-flipped within the year).

---

## MINOR

- **M1 — `score_assignment` trusts principal conservation that `consume_picks` does not enforce.**
  `consume_picks` hardcodes `shortfall = 0` (`pools.rs:175`) and `selection_feasible` checks only
  per-lot availability, **not** `Σpicks == need` (`pools.rs:107-153`). `fold_with` injects directly
  into `res.selections`, bypassing resolve's `Σ==principal` guard (`resolve.rs:599-611`). A
  non-conserving injected assignment therefore under-consumes **silently with no blocker** → a falsely
  low score. `optimize_year` never reaches this (candidates always conserve), but `score_assignment` is
  `pub` for KATs — add a `debug_assert`/guard that each injected pick-set conserves, or document the
  precondition loudly.
- **M2 — multi-partial kink-aligned optimum (limitation i).** Correctly characterized and documented,
  and exotic, but the output still labels the result "the optimum / least tax." Surface the limitation
  (same Approximate mechanism, or a one-line caveat) rather than only filing it to `FOLLOWUPS.md`.
- **M3 — consult available-lots = end-of-timeline pool, valid only for `at` ≥ last ledger event.**
  `consult_sale` reads the post-full-fold pool filtered by `acquired_at <= at` (Task 6 lines 708-713).
  For a past/interleaved `at` this is the wrong pool (lots later disposed are missing; the "truncate
  == end of timeline" justification only holds when the synthetic disposal is genuinely last). Guard
  (`at` must be ≥ the latest ledger event date) or compute the as-of-`at` pool properly.
- **M4 — `one_year_after(start).next_day()` returns `Option<Date>`.** `time::Date::next_day` is
  `Option` (Dec-31 / max-date edge). Handle it (Task 6 line 772) rather than assuming infallibility.
- **M5 — `accept` appends before the blanket-attest guard.** In the `only=None` loop a
  `ContemporaneousNow` disposal is appended to `conn` before a later `NeedsAttestation` disposal trips
  the "no blanket attestation" `Err` (Task 10 lines 998-1011). No disk corruption (no `save()` on the
  error path), but validate the `--attest`-without-`--disposal` precondition **before** the loop.

## NIT

- **N1 — false comment.** Task 4 line 538: "status reflects the proposed selections" — it does not
  (see C2a); fix the comment when fixing C2.
- **N2 — backwards wording.** Task 4 line 595 / Task 5 line 668 call `persistability(wallet, date,
  date)` a "conservative stand-in"; `ContemporaneousNow` is the *least* conservative verdict.

---

## Required to reach GREEN (0C/0I)

1. **C1** — Approximate flag + render banner + cap log + baseline-seeded coordinate descent + KAT.
2. **C2** — proposal `status`/`persistable` computed against the actual proposed pick and a truthful
   made-date; no `StandingOrder`/`ContemporaneousNow` by fall-through; KATs (divergent pick ≠
   StandingOrder; already-executed ≠ ContemporaneousNow).
3. **C3** — joint enumeration for contended pools **or** contention-detection + Approximate disclosure;
   KAT with two contended intra-year sells across an ST/LT crossover.
4. **I1** — sort (+ partition) before truncating in `available_lots_before`; KAT fixtures with
   load-order ≠ canonical-order.
5. **I2** — derive `Ord` on `LotPick` (as a task) or re-key dedup/tie-break to an `Ord` tuple; drop the
   "no A-side change" claim.
6. **I3** — fix the loss-harvest KAT to what the single-year objective guarantees (and disclose
   carryforward-blindness) or spec a secondary objective.
7. **I4** — timing insight: term-flip within the same year/profile; degrade (omit) instead of erroring
   on an unbundled crossover year; KAT.

Re-review after every fold, including the last (STANDARD_WORKFLOW §2).

---

# Round 2 — re-review (independent, post-fold)

**Artifact (revised):** `design/IMPLEMENTATION_PLAN_optimizer.md` (folds R0 round 1).
**Re-grounded against CURRENT shipped A+B source (re-read 2026-06-30):**
`crates/btctax-core/src/project/{compliance.rs,resolve.rs,fold.rs,pools.rs,evaluate.rs}`,
`event.rs`, `state.rs`, `identity.rs`, `conventions.rs`, `tax/compute.rs`;
`crates/btctax-adapters/src/tax_tables.rs`.

**Scope (per the round-2 brief):** confirm each round-1 finding's fold actually closed it, and that the
fold introduced no new defect. The algorithm SPINE (vertex sufficiency, feasibility-by-scoring
self-elimination, Mode-2 read-only, I6 refusal) was certified sound in round 1 and is **not**
re-litigated here.

**Verdict: NOT GREEN. 1 Critical, 1 Important, 2 Minor.** Five of the seven round-1 blockers are fully
closed (C3, I1, I2, I3, I4) and all five Minors + both Nits are correctly folded. But **C1 and C2 are each
closed only on the path round 1 named** — each has a *sibling* path the fold left uncovered, and both
siblings reproduce the exact honesty failure the headline forbids (a non-global result shown as "the
optimum"; an un-attested post-hoc pick shown as compliant). One more small fold is required.

## Source citations re-verified at round-2 fold time (all accurate)
- `compliance.rs:144-149` — applied-selection branch judges by the selection's own made-date; standing
  order (step 3) is "only reachable when NO selection was applied." So in a **what-if** (proposed pick
  lives in the fold's `selections`, not in `events`) `sel_made` is empty and a divergent pick *does*
  fall through to `StandingOrder` — C2's root cause is real and current.
- `pools.rs:107-153` `selection_feasible` checks per-lot availability / cross-wallet / existence only —
  **not** `Σpicks == need`; `pools.rs:156-176` `consume_picks` returns `(out, 0)` (shortfall hardcoded).
  M1's precondition and `debug_assert` are correctly targeted.
- `compute.rs:174-177` `loss_deduction = if net_loss < loss_limit { net_loss } else { loss_limit }` —
  confirms I3 (over-harvest beyond the §1211 cap is an objective tie).
- `tax_tables.rs` bundles TY2025 only — confirms I4's degrade-not-error rationale.
- `event.rs:190` `LotPick` still derives only `Debug, Clone, PartialEq, Eq, Serialize, Deserialize` —
  I2's additive `Ord` is still required and accurate.
- `fold.rs:335,341` does exactly `sort_canonical(&mut res.timeline)` then
  `sort_by_key(|e| e.date() >= TRANSITION_DATE)`; `resolve` (resolve.rs:269) does not sort; fields
  `timeline`/`selections` and `sort_canonical` (resolve.rs:805) / `fold` (fold.rs:330) are `pub` — I1's
  fix mirrors the fold ordering precisely.

## Round-1 findings — closure status

- **C1 (silent local optimum shown as "the least tax") — CLOSED on the coordinate-descent path; NOT
  closed on the large-pool path. See new finding R2-C1.** `approximate`/`approx_reason` added;
  baseline-seeded incumbent in both `exhaustive_min` and `coordinate_descent`; CLI logs + render banner.
  `delta ≤ 0` is now structurally guaranteed (incumbent seeded at `base.total_federal_tax_attributable`;
  `fold_with(baseline_assignment)` reproduces the baseline `(ST,LT)` → identical tax, so the worst case is
  `delta == 0`). The coordinate-descent sub-case is genuinely fixed. **But** `approximate` is wired to
  exactly two triggers (`product > MAX_COMBOS`; a contended group returning `None`) — it is NOT wired to
  the per-disposal `candidate_selections` **`> LOT_ENUM_BOUND` heuristic branch** (plan lines 485-516),
  which returns a strict *subset* of vertices with no signal to the caller. → R2-C1.
- **C2 (post-hoc / standing-order-divergent picks shown compliant/persistable) — CLOSED for the
  `proposed_compliance_status` + `persistability(.., proposal_made)` paths; NOT closed for the
  `compliance_overlay` path. See new finding R2-I1.** Status now judged by the proposed pick's own
  made-date; `StandingOrder` only when `proposed == current`; `persistable` uses the real `proposal_made`
  (already-executed ⇒ never `ContemporaneousNow`); the old `disposal_compliance(events,&opt_state)` and
  `persistability(date,date)` calls are removed (§5 confirms). N1/N2 fixed. **But** the overlay upgrade
  `NonCompliant → AttestedRecording` is gated only on `attested.contains(disposal)` + envelope — **not**
  on `proposed == current`. → R2-I1.
- **C3 (contended same-wallet cross-period optimum) — CLOSED.** `contention_groups` +
  `group_candidate_assignments` (nested generation, prior candidate's resulting pool) jointly enumerate
  within `GROUP_COMBO_BOUND` (recovering the ST-at-D1/LT-at-D2 reassignment); beyond the bound → independent
  fallback + `approximate=true, ContentionUnenumerated`. Cartesian product across disjoint-pool groups;
  cross-group infeasibility self-eliminates via `score_assignment → NotComputable` (verified airtight in
  round 1). KAT asserts the joint optimum within bound and the flag beyond it. The reason-precedence
  (`ComboCapExceeded` dominates `ContentionUnenumerated`) is acceptable — both still raise the banner.
- **I1 (truncate in load order) — CLOSED.** `sort_canonical` + transition partition before
  `position`/`truncate`, exactly mirroring `fold.rs:335,341`; the truncated set is a canonical prefix so
  fold's internal re-sort is idempotent. Load-order≠canonical KATs specified.
- **I2 (`LotPick: Ord`) — CLOSED.** Additive `PartialOrd, Ord` derive (both fields `Ord`); serde
  unchanged; the "reuse verbatim / no A-side change" claims corrected to acknowledge the one exception;
  `BTreeSet<Vec<LotPick>>` KAT.
- **I3 (loss-harvest KAT) — CLOSED.** KAT now asserts only `optimized_tax == oracle_min_total`,
  `optimized_tax < baseline_tax`, and in-year `loss_deduction == $3,000`; no carryforward-split assertion;
  carryforward-blindness disclosed (§4 + FOLLOWUPS).
- **I4 (Mode-2 timing) — CLOSED.** `timing_insight -> Option`; term-flip within `at`'s own
  year/profile/table (synthetic disposal dated `latest_crossover` carrying the actual proceeds), only when
  `latest_crossover.year() == at.year()` and table+profile present; otherwise omits; `next_day() == None`
  (M4) omits. Degrade-not-error KAT specified.
- **Minors/Nits — FOLDED.** M1 (`debug_assert` Σ==principal + should_panic KAT), M2 (render caveat),
  M3 (`fold_as_of`), M4 (`next_day` Option), M5 (blanket-attest guard hoisted above the loop and above the
  recompute), N1/N2 (comments) — all reflected in the revised plan.

---

## CRITICAL

### R2-C1 — Large-pool (`> LOT_ENUM_BOUND`) heuristic candidate sets are scored and returned with `approximate = false`, i.e. presented as "the PROVEN global minimum," with no banner and no cap log. (Residual of C1; the fold's new `approximate` contract does not cover this path.)
`candidate_selections` is complete only for `lots.len() <= LOT_ENUM_BOUND` (= 12); above that it returns
a **deterministic heuristic subset** (FIFO/LIFO/HIFO/LT-first greedy fills + per-lot lead, plan lines
485-516) and returns `Vec<Vec<LotPick>>` with **no indication** that the heuristic branch was taken.
`optimize_year` sets `approximate` only when `product > MAX_COMBOS` or a contended group returns `None`
(lines 668-683). For a **single** disposal drawing from, say, a 20-lot pool, the heuristic yields ~`4 + n`
candidates → `product ≈ 24 ≪ MAX_COMBOS` → `exhaustive_min` runs over the **incomplete** list →
`approximate = false`, `approx_reason = None`.

Consequences:
- The Task-1 doc contract is then false: `approximate == false` is documented as "the result is the
  PROVEN global minimum over the vertex space" (optimize.rs doc, plan lines 214-216) — but the vertex
  space was *not* completely enumerated for that pool.
- `render_optimize_proposal` prints `optimized_tax`/`delta` with **no** "⚠ APPROXIMATE" banner (Task 9
  shows the banner only `if p.approximate`), and `run` logs nothing.
- This is precisely the headline-forbidden case ("no non-fully-enumerated result rendered as 'the
  optimum' without the banner") and the same failure-class round 1 graded Critical (C1/C3). `> 12`-lot
  single-wallet pools are common (weekly DCA, any active trader), so it is readily reachable in
  production — not a corner case.

Mitigation that bounds (but does not remove) the harm: the search is baseline-seeded, so `delta ≤ 0`
still holds — the tool never recommends *worse* than the current filing position. The defect is the
**false "this is the global optimum" claim**, not an unsafe selection. That is still a Critical on the
honesty surface the headline governs.

**Why this is in scope (not spine re-litigation):** round 1 certified vertex completeness only for
`n ≤ 12` and treated the `> 12` heuristic as an explicit approximation; it predates the `approximate`
flag. The *fold* introduced the `approximate == false ⇒ "proven global minimum"` contract without
extending it to this pre-existing approximate path — so the fold left C1's own acceptance criterion
("No path renders a non-fully-enumerated result as 'the optimum' without the banner") unmet.

**Fix (mechanical, same disclosure mechanism as C1/C3):** have `candidate_selections` (or a thin wrapper)
report whether it used the `> LOT_ENUM_BOUND` heuristic branch; in `optimize_year`, if **any** target
disposal's candidate set was heuristic, set `approximate = true` with a new
`ApproxReason::PoolHeuristic { lots, bound }` (and let `ComboCapExceeded` keep precedence). The renderer
already shows the banner for any `approximate`; extend the `match` arm. Add a KAT: a single disposal over
a `> LOT_ENUM_BOUND` pool ⇒ `approximate == true` + `PoolHeuristic`, and `delta ≤ 0`; a `≤ 12`-lot pool
⇒ `approximate == false`.

## IMPORTANT

### R2-I1 — `compliance_overlay` can upgrade a **divergent, un-attested** proposed pick to `AttestedRecording`, presenting an un-attested post-hoc cherry-pick as compliant. (Residual of C2; the overlay is disposal-scoped, not gated on `proposed == current`.)
The attestation side-table is keyed by `disposal_event` only (Task 8: `optimize_attestation(disposal_event
PRIMARY KEY, attestation, attested_at)`) — it attests a *disposal*, not a *specific selection*.
`compliance_overlay` (Task 5, plan lines 880-896) upgrades `NonCompliant → AttestedRecording` whenever
`attested.contains(&c.disposal)` and the envelope allows — with **no** check that the row's *proposed*
pick equals the one that was actually attested. `optimize_year` applies the overlay to **all** proposal
rows (line 716).

Failure sequence: user runs `optimize accept --disposal X --attest "…"` → persists selection `P1` for X
and writes an attestation row for X. Later (a new lot acquired, or a new same-pool disposal introduces
contention) a re-`run` finds a strictly-better **divergent** pick `P2 ≠ P1` for X. Now in
`optimize_year`: `proposed = P2 ≠ current = P1` → `proposed_compliance_status` correctly returns
`NonCompliant` (divergent post-hoc) → **but** `compliance_overlay` upgrades it to `AttestedRecording`
because X ∈ `attested`. The `run` output therefore labels a **new, never-attested** post-hoc cherry-pick
as `AttestedRecording` = compliant. This is the forbidden "post-hoc selection described as compliant,"
reintroduced through the overlay — and it is *asymmetric* with the fold's own C2 fix, which correctly
refuses to let a **standing order** rescue a divergent pick (`StandingOrder` only when
`proposed == current`) but lets an **attestation** do exactly that.

Reachability is moderate-to-low (requires a prior attestation followed by a divergent better pick), and
`accept` still gates the *write* correctly (`persistability` ⇒ `NeedsAttestation`, requiring a fresh
`--attest`), so no wrong persistence occurs — the violation is confined to the read-only proposal the
user reads to decide. Hence Important, not Critical.

**Fix:** gate the `AttestedRecording` upgrade on `proposed == current` (the only row whose persisted,
attested selection is what is being shown) — mirror the `proposed_compliance_status` rule. Concretely,
either have `optimize_year` pass overlay-eligibility per row (upgrade only `proposed == current` rows) or
extend `compliance_overlay`'s signature with a per-disposal "diverged" predicate. Add a KAT: attest X for
`P1`; force a re-`run` whose proposed pick for X is `P2 ≠ P1`; assert the row is `NonCompliant`
(**not** `AttestedRecording`).

## MINOR

- **R2-M1 — `persistable` line on a no-change row is misleading.** When `proposed == current` for an
  already-executed disposal, `persistability(wallet, date, proposal_made)` returns `NeedsAttestation`
  (made > sale), so `render_optimize_proposal` prints "already executed — needs `optimize accept
  --disposal <ref> --attest …`" for a disposal the optimizer is **not** asking to change (and which
  `accept` correctly *skips* as "already optimal," Task 10 line 1278). No wrong write occurs, but the
  line invites a pointless/contradictory attestation. Suppress or relabel the persistability line when
  `proposed_selection == current_selection`.
- **R2-M2 — NFR4's determinism input-tuple is now incomplete.** §0 line 17 states identical
  `(events, prices, config, year, profile, tables)` ⇒ "byte-identical `OptimizeProposal`." After the
  fold the proposal's `status`/`persistable` also depend on `proposal_made` and `attested` (the
  optimization core — picks/`optimized_tax`/`delta` — does not, which is correct). Add `proposal_made`
  and `attested` to the stated tuple so the NFR4 claim is true as written (behavior is deterministic
  given all inputs; only the wording lags).

## What remains correct (not re-litigated)
Vertex sufficiency for `n ≤ 12`, feasibility-by-scoring self-elimination, Mode-2 read-only,
I6 refusal, the baseline-seed `delta ≤ 0` guarantee, determinism (BTreeMap/BTreeSet/sorted-Vec,
`Decimal`/`i64` only, no `HashMap` iteration, no clock in core — `proposal_made`/`at` threaded from the
CLI seam), and the closed findings above.

## Required to reach GREEN (round 3)
1. **R2-C1** — flag `approximate = true` (+ `ApproxReason::PoolHeuristic`) whenever any disposal's
   candidate set used the `> LOT_ENUM_BOUND` heuristic branch; render banner + log; KATs both ways.
2. **R2-I1** — gate the `AttestedRecording` overlay upgrade on `proposed == current`; KAT (divergent
   pick on an attested disposal stays `NonCompliant`).
3. **R2-M1 / R2-M2** — fold the no-change persistability line and the NFR4 input-tuple wording.

Re-review after this fold, including the last (STANDARD_WORKFLOW §2). The two residuals are narrow and
mechanical, but both reproduce headline-forbidden honesty failures, so they are blocking until folded
and re-reviewed to 0C/0I.

---

# Round 3 — re-review (independent, post-fold)

**Artifact (revised):** `design/IMPLEMENTATION_PLAN_optimizer.md` (folds R0 round 2: 1 Critical + 1
Important + 2 Minor residuals).
**Re-grounded against CURRENT shipped A+B source (re-read 2026-06-30):**
`crates/btctax-core/src/project/{compliance.rs,fold.rs,pools.rs}`, `event.rs`.

**Scope (per the round-3 brief):** confirm the two round-2 residuals (R2-C1, R2-I1) and the two round-2
Minors (R2-M1, R2-M2) are closed by the fold, and that the fold introduced no new Critical/Important.
The algorithm spine and the round-1 findings (C1/C2/C3/I1/I2/I3/I4 + round-1 minors), all confirmed
closed in rounds 1–2, are **not** re-litigated — I checked only whether the round-2 fold disturbed any
of them (it did not).

**Verdict: GREEN. 0 Critical, 0 Important, 0 new Minor.** R2-C1 and R2-I1 are both genuinely closed, the
two Minors are folded, and the two signature changes (`candidate_selections -> (.., bool)`; the
`unchanged` threading + `compliance_overlay`'s third arg) are wired through every caller without
breaking determinism, exactness, or any previously-closed finding. **C's plan is R0 GREEN — ready to
implement.**

## Source citations re-verified at round-3 fold time (all accurate)
- `compliance.rs:144-149` — an applied `LotSelection` is judged by its OWN made-date; `StandingOrder`
  (step 3, lines 151-165) is reachable ONLY when no selection was applied. Confirms (a) C2's root cause
  is real (a what-if's divergent pick lives in the fold's `selections`, not `events`, so it falls through
  to `StandingOrder`), and (b) the plan correctly does NOT feed `disposal_compliance(events,&opt_state)`
  the proposed pick — it uses `proposed_compliance_status` instead.
- `event.rs:190` — `LotPick` still derives only `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`
  (the additive `Ord` of I2/Task-1 is still required and accurate).
- `event.rs:214-218` — `LotSelection { disposal_event, lots }`, keyed by `disposal_event`. Underwrites
  R2-I1's "persisted+attested P1 drives the baseline ⇒ `current == P1`" reasoning.
- `pools.rs:156-176` `consume_picks` — for each `LotPick` it consumes exactly one matching lot
  (`take_from` once per pick) ⇒ **one pick → one `Consumed` → one `DisposalLeg`** (`fold.rs:116-180`
  pushes one leg per consumed fragment, including the dual-basis four-zone path). So `baseline_selection`
  reconstructs P1's picks one-for-one (sorted canonically) and `proposed == current` is an exact, sound
  equality — the `unchanged` gate cannot be defeated by a leg/pick representation mismatch. (Validates the
  load-bearing premise the brief asked me to confirm.)

## Round-2 findings — closure status

### R2-C1 — large-pool heuristic set shown as the PROVEN global minimum. **CLOSED.**
The heuristic-branch-taken signal genuinely propagates end to end:
- `candidate_selections(lots, need) -> (Vec<Vec<LotPick>>, bool)` (plan line 476): `heuristic =
  lots.len() > LOT_ENUM_BOUND` (line 477), returned at line 537. Honest signal, not a guess.
- `group_candidate_assignments -> Option<(maps, Option<usize>)>` (lines 567-570): the inner
  `Option<usize>` carries the largest nested-heuristic lot count; the outer `None` is reserved for
  beyond-`GROUP_COMBO_BOUND`. Both signals are distinct and both reach the caller.
- `optimize_year` accumulates `pool_heuristic_lots` for the singleton path (lines 680-682, before the
  empty-cands fallback at 683, so the flag is never lost) and the jointly-enumerated group path (lines
  688-690), then `if pool_heuristic_lots.is_some() { approximate = true; }` (line 703).
- `approx_reason` precedence (lines 721-729) is `ComboCapExceeded > ContentionUnenumerated >
  PoolHeuristic`, with `PoolHeuristic { lots, bound: LOT_ENUM_BOUND }` reporting the largest such pool.

I verified the contract holds *strictly*: `approximate == false` requires simultaneously `product ≤
MAX_COMBOS` (else line 714), `contended_unenum == 0` (else line 694), AND `pool_heuristic_lots.is_none()`
(else line 703) — i.e. every pool fully enumerated (≤ bound) **and** every contended group jointly
enumerated **and** exhaustive (not coordinate-descent). That is exactly the corrected Task-1 contract
(plan lines 223-230): `approximate == false ⇔ fully enumerated + exhaustively scored ⇒ proven global
minimum over the vertex space`. I also checked the inverse: every branch that sets `approximate = true`
also yields a `Some(reason)` (no banner-without-reason), and every reason implies `approximate == true`
(no reason-without-banner). The KATs are genuine both-ways — Task 3 unit KAT on the flag (line 589),
Task 4 `PoolHeuristic { lots: 20, bound: 12 }` vs `≤12 ⇒ approximate==false`/`reason==None` (line 863),
Task 9 render-banner KAT (line 1311) — and all assert `delta ≤ 0`. The baseline-seed (lines 705-717) is
untouched, so `delta ≤ 0` still holds on the heuristic path. **No incomplete-enumeration path can render
as "the optimum" without the banner.**

### R2-I1 — attestation overlay launders a divergent pick. **CLOSED.**
`compliance_overlay`'s upgrade predicate (plan lines 956-959) now requires
`NonCompliant ∧ attested.contains(disposal) ∧ unchanged.contains(disposal) ∧ ¬(broker ∧ year ≥ 2027)`.
`optimize_year` builds `unchanged` from `row_meta` as exactly the disposals with `proposed == current`
(lines 768-772) and passes it (line 773). Because a persisted+attested `LotSelection` P1 drives the
baseline fold (keyed by `disposal_event`, `event.rs:214-218`; one-pick-one-leg per `consume_picks`),
`current == P1` on re-run; a strictly-better divergent re-run pick `P2 ≠ P1` has `proposed != current` ⇒
∉ `unchanged` ⇒ stays `NonCompliant`. The overlay therefore confers `AttestedRecording` ONLY on the
exact attested selection — **symmetric** with the C2 standing-order fix (`proposed_compliance_status`
returns `StandingOrder`/baseline only on the `proposed == current` path, line 920). The KATs cover both
the pure overlay (line 984: attested+unchanged ⇒ upgrade; attested-but-not-unchanged ⇒ stays
`NonCompliant`; 2027+ broker ⇒ stays `NonCompliant`) and end-to-end through `optimize_year` (line 987:
no-change re-run ⇒ `AttestedRecording`; divergent `P2` ⇒ `NonCompliant`). I confirmed the `accept` write
path is consistent: a divergent `NeedsAttestation` pick requires a **fresh** `--attest` (Task 10 line
1380-1392), after which `current == P2` and the overlay legitimately reports `AttestedRecording` — no
laundering at the write boundary either.

### R2-M1 — no-change row shows a misleading "needs --attest" line. **CLOSED.**
`render_optimize_proposal` short-circuits the persistability line when `proposed == current`, printing a
"no change — already optimal under current identification" note and `continue` (plan lines 1290-1294),
consistent with `accept`'s skip (Task 10 lines 1367-1369). The status line is still printed (correct —
it truthfully shows e.g. `AttestedRecording`/`NonCompliant`); only the actionable-but-pointless
persistability line is suppressed. Render KAT at line 1311.

### R2-M2 — NFR4 determinism tuple incomplete. **CLOSED.**
§0 line 17 now reads `(events, prices, config, year, profile, tables, proposal_made, attested)`, with the
correct caveat that `proposal_made`/`attested` affect only `status`/`persistable`, not the optimization
core (picks/`optimized_tax`/`delta`). Doc-lag only; behavior was already deterministic.

## New-defect sweep (did the fold disturb anything?)

- **Determinism / exactness preserved.** The new `bool` is `Copy`; `pool_heuristic_lots: Option<usize>`
  is folded by `max` (deterministic, order-independent); `unchanged: BTreeSet<EventId>` is built from the
  EventId-ordered `row_meta` Vec. No `HashMap`, no float, no clock introduced. NFR4/NFR5 intact.
- **Precedence has no inversion.** `ComboCapExceeded > ContentionUnenumerated > PoolHeuristic`; every
  case still sets `approximate = true` and raises the banner, so precedence only selects which (most
  severe) reason is *named* — it can never demote a non-global to `approximate == false`.
- **Closed findings undisturbed.** C3 (contention grouping), I1 (`available_lots_before` ordering), and
  I4 (Mode-2 timing) are orthogonal to the two signature changes; `unchanged` is computed *after* the
  search from `row_meta`, and the overlay touches only `NonCompliant` rows.
- **Mode-2 threading is a correct no-op.** `consult_sale` destructures `(cands, _heuristic)` (line 1053);
  `ConsultReport` carries no `approximate` field and `render_consult` makes no "optimum" claim, so R2-C1
  (scoped to the Mode-1 `OptimizeProposal` "proven optimum" claim) does not extend here. This matches the
  round-2 scoping and is not a regression.

## Informational (non-blocking — NOT findings)

- When a contended group exceeds `GROUP_COMBO_BOUND`, `optimize_year` falls back to
  `independent_group_maps` (line 696) whose signature does not surface a per-pool heuristic flag. This is
  harmless: that branch unconditionally sets `approximate = true` and `contended_unenum += members.len()`
  (lines 694-695), and `ContentionUnenumerated` already dominates `PoolHeuristic` in the precedence — so
  the banner is raised and the reported reason is the more-severe one by design. No false-global can
  escape. Recorded only so a future maintainer doesn't "fix" `independent_group_maps` to thread a flag
  that the precedence makes moot.
- Mode-2 `consult` over a `> LOT_ENUM_BOUND` pool searches a heuristic subset (its `proposed_selection`
  is a tax-min over that subset, not a proven global min). This is a pre-existing property of Mode-2, not
  introduced by this fold, and `ConsultReport` makes no global-minimum claim, so it is correctly out of
  R2-C1's (Mode-1) scope. If a future cycle wants Mode-2 to disclose pool-heuristic incompleteness too,
  that is a new (Minor) scope item, not a regression here.

## Required to reach GREEN
Nothing. 0 Critical / 0 Important / 0 new Minor. Per the round-3 brief and STANDARD_WORKFLOW §2, this was
the re-review after the (last) fold; with a clean result the plan exits the review loop. **C's
IMPLEMENTATION_PLAN_optimizer.md is R0 GREEN and ready to implement** (subagent-driven, one implementer
carrying the whole plan; Task 12 remains the mandatory whole-diff Phase-E gate).
