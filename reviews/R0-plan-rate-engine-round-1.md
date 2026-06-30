# R0 review — IMPLEMENTATION_PLAN_rate_engine.md (Sub-project B), round 1

**Artifact:** `design/IMPLEMENTATION_PLAN_rate_engine.md` (DRAFT)
**Contract:** `design/SPEC_lot_optimization_program.md` — Sub-project B (§B.1–B.5) + "Rate authorities (B)" + Cross-cutting
**Reviewer role:** independent R0 architect + US-tax review (mandatory gate; review-to-green, `STANDARD_WORKFLOW.md §2`)
**Date:** 2026-06-29
**Gate question:** 0 Critical / 0 Important before implementation? **NO — 1 Important (B-I1).** 0 Critical.

This engine computes federal tax; per the task framing a wrong bracket/threshold, a wrong netting/stacking/NIIT/loss formula, or a double-count is Critical. I verified **every** TY2025 figure against primary/authoritative secondary sources, re-derived the §1222/§1(h)/§1411/§1211/§1212 mechanics from statute, hand-checked the golden KATs, and grounded every cited `file:line` against current source.

---

## 1. Source grounding (every cited signature/type verified against CURRENT source)

All §1 citations in the plan check out against current source:

- `Usd = Decimal` / `Sat = i64` / `TaxDate = Date` — `conventions.rs:6,8,10` ✓; `round_cents` ROUND_HALF_EVEN — `conventions.rs:13,22-24` ✓; `tax_date` — `conventions.rs:52-54` ✓.
- `BlockerKind` enum + `severity()` Hard set — `state.rs:22-46,47-64` ✓ (Hard set today = FmvMissing, UncoveredDisposal, ImportConflict, DecisionConflict, UnknownBasisInbound, Unclassified, SafeHarborUnconservable, MethodElectionBackdated, LotSelectionInvalid, Pre2025MethodConflictsAllocation). `new_blockers_are_hard` test present — `state.rs:206-217` ✓.
- `LedgerState{lots,holdings_by_wallet,disposals,removals,income_recognized,pending_reconciliation,blockers,stats}` — `state.rs:176-186` ✓. `Disposal{event,kind,disposed_at,legs,fee_mini_disposition}` — `state.rs:107-116` ✓. `DisposalLeg{...gain,term...}` — `state.rs:96-106` ✓. `IncomeRecord{event,recognized_at,sat,usd_fmv,kind,business}` — `state.rs:140-148` ✓.
- `IncomeKind{Mining,Staking,Interest,Airdrop,Reward}` — `event.rs:28-35` ✓ (all five are ordinary at FMV — the plan sums all of them, correct). `LedgerEvent{id,utc_timestamp,original_tz,wallet,payload}` — `event.rs:263-271` ✓. `EventId::canonical()` — `identity.rs:86` ✓; `EventId` derives `Ord` (usable as the `BTreeMap` key in `hard_blocker_for_year`) — `identity.rs:54-56` ✓.
- `project(events,prices,config)->LedgerState`, `LotMethod`, `ProjectionConfig` — `project/mod.rs:24-56` ✓.
- Bundled-dataset pattern (`BundledPrices`, `BTreeMap`-backed, `load()`, `impl PriceProvider`) — `adapters/price.rs:10-50` ✓. Side-table pattern (`CliConfig`/`init_config_table`/`read_config`/`set_*`, `CliError::BadConfigValue`) — `cli/config.rs:42-149,86-90` ✓.

The `TaxProfile`/`TaxResult`/`Carryforward` named-struct adaptation and the `tax_profile` side-table / `BundledTaxTables` bundled-data placements faithfully mirror the existing `cli_config` and `BundledPrices` conventions. Crate layering (compute+types+trait+statutory consts in core; bundled numbers in adapters; profile side-table+commands in CLI; real-number goldens in adapters tests) is clean with no cross-crate number duplication. No new ledger events. ✅

---

## 2. PRIMARY-SOURCE TAX VERIFICATION (the Critical surface)

The IRS Rev. Proc. 2024-40 PDF does not text-extract (confirmed — same failure the plan author hit). I cross-checked **every** figure against ≥2 independent authoritative sources that publish the exact Rev. Proc. values.

### 2.1 TY2025 ordinary brackets (Rev. Proc. 2024-40 §2.01) — ALL FIVE statuses

Cross-checked against **Tax Foundation (2025-tax-brackets)** and **Bradford Tax Institute (2025-Federal-Tax-Brackets)**. Plan values (bracket-`lower`, $):

| Status | 12% | 22% | 24% | 32% | 35% | 37% | Verdict |
|---|---|---|---|---|---|---|---|
| Single | 11,925 | 48,475 | 103,350 | 197,300 | 250,525 | **626,350** | ✅ both sources |
| MFJ/QSS | 23,850 | 96,950 | 206,700 | 394,600 | 501,050 | **751,600** | ✅ both sources |
| MFS | 11,925 | 48,475 | 103,350 | 197,300 | 250,525 | **375,800** | ✅ Bradford |
| HoH | 17,000 | 64,850 | 103,350 | 197,300 | **250,500** | 626,350 | ✅ both sources |

All match exactly. **The classic gotcha is correct:** HoH 35% starts at **250,500** while Single/MFS 35% start at **250,525** — the plan distinguishes the $25, and so does the `ty2025()` code (`crates/btctax-adapters/src/tax_tables.rs` Single/MFS use `250525`, HoH uses `250500`). MFS 37% = 375,800 (≠ Single's 626,350), correctly encoded.

### 2.2 TY2025 §1(h) LT 0/15/20% breakpoints (Rev. Proc. 2024-40 §2.03) — ALL FIVE statuses

Cross-checked against **Tax Foundation** and **TheFinanceBuff** (both publish the exact Rev.-Proc. maximum-zero / maximum-15 amounts; Tax Foundation prints them in the "top-of-bracket minus $1" convention — 48,349 / 533,399 — which confirms the same underlying breakpoints 48,350 / 533,400):

| Status | max_zero | max_fifteen | Verdict |
|---|---|---|---|
| Single | 48,350 | 533,400 | ✅ TaxFoundation + FinanceBuff |
| MFJ/QSS | 96,700 | 600,050 | ✅ TaxFoundation + FinanceBuff |
| HoH | 64,750 | 566,700 | ✅ TaxFoundation + FinanceBuff |
| **MFS** | **48,350** | **300,000** | ✅ FinanceBuff (MFS 15% = $300,000 exactly; MFS 0% = ½·MFJ = 48,350) |

**The one figure that needed care:** the MFS 15% breakpoint. A naive "½ of MFJ" heuristic gives 300,025 (and one search hallucinated "583,400" for Single), so I did not assume. TheFinanceBuff (which lists MFS explicitly) confirms the printed Rev. Proc. value is **$300,000** — matching the plan. (Note the 2024 MFS value was $291,850, which is NOT exactly ½ of MFJ, proving the heuristic unreliable and the explicit-source check necessary.) The Single 20% threshold is **533,400**, not 583,400 (that search result was wrong). ✅

### 2.3 Statutory, NON-indexed values

Confirmed against 26 U.S.C. §1411(b) (LII) and IRS NIIT Q&A; §1211(b); and the IRS NIIT guidance ("not indexed for inflation"):

- **NIIT rate 3.8%** — §1411(a). `NIIT_RATE = dec!(0.038)` ✅.
- **NIIT thresholds** — §1411(b): MFJ/surviving-spouse **$250,000**; MFS **$125,000** (½ of MFJ); "any other case" **$200,000** — which is **both Single AND HoH**. The plan's `niit_threshold()` returns $200k for Single|HoH (✅ — HoH is correctly $200k, *not* its own indexed amount, a real gotcha), $250k for Mfj|Qss, $125k for Mfs. **Confirmed not inflation-indexed** (§1411 has no COLA). ✅
- **§1211(b) capital-loss ordinary-offset limit** — **$3,000 / $1,500 MFS**, statutory, not indexed. `loss_limit()` returns 1500 for Mfs else 3000. ✅

The statutory-vs-indexed discipline (I4) is correctly implemented: NIIT rate/threshold + loss limit are year-independent `fn`/`const` with statute cites in `tax::tables`, **never** in `TaxTable`; the KAT (Tasks 2/6) asserts they are constant across years while indexed values move, and that no `TaxTable` field carries them. ✅

**Conclusion: every hard-coded tax figure in the plan is correct. 0 Critical on the numbers.**

---

## 3. MECHANICS VERIFICATION (re-derived from statute)

### 3.1 §1222 ST/LT netting order — CORRECT
`net_1222` does within-character netting first (`st_net = crypto_st − cf_short`; `lt_net = crypto_lt + other_lt − cf_long`), then cross-nets only when one character is a gain and the other a loss, producing `ordinary_gain` (surviving net ST gain → ordinary) and `preferential_gain` (§1222(11) net capital gain → §1(h)). I traced all four sign quadrants plus the four KAT cases (both-gains, ST-gain/LT-loss, ST-loss/LT-gain, both-loss): each matches §1222(5)-(8),(11) and the Schedule D line-16 procedure. Carryforward-in correctly subtracts within matching character. ✅

### 3.2 §1(h) 0/15/20 stacking — CORRECT
`preferential_tax` places `pref` (= QD + net LT gain) ON TOP of `bottom` (ordinary taxable income incl. net ST gain, post loss-deduction), compares against TOTAL taxable income for the breakpoints, fills 0% then 15% then 20%. This is exactly the Qualified Dividends & Capital Gain Tax Worksheet. Ordinary tax is computed on `bottom` only (no double-tax of preferential dollars at ordinary rates). QD correctly shares the preferential bracket space with crypto LT (I9). ✅

### 3.3 §1411 NIIT — CORRECT
`niit = 3.8% × min(NII, MAGI − threshold)`. NII = QD + surviving net ST gain + surviving net LT gain (all are NII). MAGI adds only the incremental crypto AGI (Δnet-capital-amount + crypto ordinary income) onto `magi_excluding_crypto`. **Ambiguity #4 (crypto ordinary in MAGI, not in NII): correctly implemented** — `crypto_ord` enters `magi_with` via `crypto_agi` but is excluded from `nii_with`. The pleasant consequence is correct: crypto ordinary income raising MAGI over the threshold properly increases NIIT on pre-existing NII, and the delta attributes that increase to crypto. ✅

### 3.4 §1211/§1212(b) $3k limit + ST-first carryforward — CORRECT
`loss_deduction = min(net_loss, loss_limit)`; `absorbed_st = min(net_st_loss, loss_deduction)`; remainder absorbs LT; carryforwards preserve character. I verified ST-first against the IRS Capital Loss Carryover Worksheet for two cases (ST 5,000 + LT 2,000 → carry {2,000 ST, 2,000 LT}; ST 1,000 + LT 5,000 → carry {0 ST, 3,000 LT}). The §1212(b)(2) "$3,000 deemed a short-term gain" mechanism does produce ST-first absorption — the plan and KATs match the worksheet exactly. ✅

### 3.5 Incremental delta + no double-count (I5) — CORRECT
Two-scenario (`with`/`without` crypto) on a fixed baseline (`other_net_capital_gain` + `carryforward_in` present in both), so the delta isolates this year's crypto. Crypto ordinary income is excluded from `ordinary_taxable_income` (B.1) and added back exactly once on `bottom_with` (B.3); the double-count guard KAT pins total = 2,200 (not 4,400). **Ambiguity #5 (MAGI adds only the delta): correct** — `magi_with = magi_excluding_crypto + crypto_agi`, no re-adding of the non-crypto baseline; the without-scenario uses `magi_excluding_crypto` as-is. I found no double-count hazard in the computation. ✅

### 3.6 Deltas-vs-levels split (ambiguity #1) — SOUND
`ltcg_tax`/`niit`/`total` are crypto-attributable deltas; `st_net`/`lt_net`/`ordinary_from_crypto`/`loss_deduction`/`carryforward_out`/`marginal_rates` are WITH-crypto levels. The rationale is correct: a *level* `ltcg_tax` is ill-defined once QD shares the §1(h) stack, whereas `pref(with) − pref(without)` is exact; `carryforward_out` MUST be a level (feeds next year). The identity `total == (ord_with − ord_without) + ltcg_tax + niit` is algebraically exact and KAT-pinned. No inconsistency or double-count between the level and delta fields. (One presentation nit — see B-M2.) ✅

### 3.7 Golden KATs are hand-reproducible from the bundled tables — CONFIRMED
I re-derived the headline goldens from the *bundled* TY2025 numbers:
- `single_lt_crosses_0_to_15`: 8,350@0% + 11,650@15% = **1,747.50** ✓
- `single_lt_crosses_15_to_20`: 33,400@15% + 66,600@20% = **18,330.00**; NIIT 3.8%×min(100k,400k) = **3,800.00** ✓
- `single_qd_pushes_crypto_lt_from_15_to_20`: with 15,830 − without 12,000 = **3,830.00** (= 20,000×15% + 16,600×5%) ✓ (the I9 demonstration)
- `mfj_st_gain_stacks_on_ordinary`: tax(110,000) 14,028 − tax(90,000) 10,323 = **3,705.00**; marginal 0.22 ✓
- `single_3k_loss…`: loss_deduction 3,000; carry {2,000 ST, 2,000 LT} ✓

Coverage hits every stated case: bracket crossings (0→15→20), NIIT threshold, ST stacking, $3k + multi-year §1212(b) ST-first, §1222 netting, QD-in-stack, and the incremental-delta double-count guard, plus the `total` identity. Task 6 pins the bundled numbers; Task 7 derives goldens from them — the "literally reproducible from the bundled tables" claim holds. ✅

### 3.8 Determinism / no-float / refusal precedence
All rate math is `Decimal` (`dec!`), rounded with `round_cents` at the end of each reported sub-component; no `f32/f64`; `BTreeMap`/Vec iteration only. Refusal precedence is deterministic (Hard-blocker → table-missing → profile-missing). Missing profile/table are their own hard outcomes. ✅ (See B-I1 for the one gap in the blocker scope.)

---

## 4. FINDINGS

### Important

**B-I1 — Refusal gate under-gates on cross-year basis contamination; a wrong number can be presented.**
`hard_blocker_for_year` (Task 5) treats a Hard blocker as in-scope for `year` only when (a) its `event` is `None` (global), (b) the event's tax-year `== year`, or (c) its kind ∈ {`SafeHarborUnconservable`, `Pre2025MethodConflictsAllocation`, `UnknownBasisInbound`}. The author correctly anticipated cross-year basis contamination (that is why (c) exists) but the enumeration is **incomplete**:

- An **unresolved `ImportConflict`** leaves the original (disputed-basis) import standing in the pool (`resolve.rs:362-377`: the `None`/unresolved branch pushes the blocker but does **not** reject the original import). The blocker's `event` is the conflict event, dated to the disputed transaction (e.g., a 2024 acquisition). A **2025** disposal that consumes that lot computes a gain from the **disputed** basis, raises **no** in-year blocker (the lot is not `basis_pending`, so it does not re-trigger `FmvMissing`), and `ImportConflict` is not in set (c) — so 2025 is **not gated** and B emits a number built on a disputed basis.
- The same hole exists for a basis-affecting **`DecisionConflict`** (e.g., conflicting `ManualFmv`/`ClassifyRaw` on a target, `resolve.rs:388-395`) whose decision made-date can postdate the disposal it affects — so even a "tax-year ≤ year" rule would not fully close it.

Contrast the case the gate *does* handle correctly: a `basis_pending` lot consumed by a disposal re-raises `FmvMissing` **on the in-year disposal event** (`fold.rs:124-131`), so that path is gated. The gap is specifically the non-`basis_pending`, disputed/contaminated lots.

This violates the load-bearing principle ("a wrong number must never be presented as authoritative", B.4 / Cross-cutting). It is not a wrong *formula* (so not Critical per the rubric), but it can surface a wrong *number*, and Important must reach 0 before implementation.

**Fix (pick one, state it in Task 5):**
1. **Simplest, provably safe (recommended):** refuse the year if **any** unresolved `severity()==Hard` blocker exists anywhere in `state.blockers` (drop the per-event/enumerated scoping). The spec's "a year whose disposals are touched" is satisfied conservatively — any open Hard blocker means the projection's basis foundation is unsound. This trades away per-year granularity for a one-line, auditable guarantee. Add a KAT: an out-of-year unresolved `ImportConflict` on a lot consumed in `year` ⇒ `TaxYearNotComputable`.
2. **Preserve per-year granularity** only by implementing lot-lineage gating: a disposal in `year` gates the year if any lot it consumes traces (origin event / basis-determining decision) to an event carrying an unresolved Hard blocker. This is materially more code and must itself be KAT'd for the `ImportConflict`/`DecisionConflict`/`ManualFmv` cases.

At minimum, `ImportConflict` and basis-affecting `DecisionConflict` must gate; do not ship the three-kind enumeration as-is.

### Minor

**B-M1 — NII model simplifications: state the direction.** (a) Crypto ordinary income is excluded from NII (already a filed follow-up) — for investor-level (non-trade/business, non-passive) staking/rewards/airdrops this can be NII, so the model can **understate** NIIT; (b) NII is not reduced by the allowed capital loss in a net-loss year. Both are inside the documented minimal-model envelope and the delta is unaffected in the common loss-year case (both scenarios = QD), so non-blocking — but the labeled-limitation note (render line + `FOLLOWUPS.md`) should say the NIIT simplification can understate, not just "excluded."

**B-M2 — Surfaced numbers don't reconcile to `total`.** `render_tax_outcome` prints `ordinary_from_crypto` (an income *level*) and `total_federal_tax_attributable` (a tax *delta*), but the addend that actually makes `total` reconcile — the ordinary-rate tax delta `(ord_with − ord_without)` — is unnamed and unprinted. A reader cannot reconcile the displayed components to `total`. Add a rendered "ordinary-rate tax (attributable): {ord_with − ord_without}" line (and consider naming it on `TaxResult`), so the printed pieces visibly sum to `total` (the identity KAT already proves they do internally).

**B-M3 — `magi_excluding_crypto` contract must be surfaced to the user.** The engine never adds `qualified_dividends_and_other_pref_income` or `other_net_capital_gain` to MAGI; per ambiguity #5 the user must already include them in `magi_excluding_crypto`, else NIIT is understated. The Task-7 QD KAT fixture even sets `magi_excluding_crypto = 450,000` with `qd = 80,000` (internally inconsistent; harmless only because that KAT doesn't assert NIIT). Make the `tax-profile` CLI help text and a doc comment state this explicitly, and fix the fixture to a self-consistent MAGI so it isn't copied as a pattern.

**B-M4 — `marginal_rates` boundary/empty conventions (cosmetic).** `marginal_ordinary_rate` uses `taxable > br.lower` (reports the lower bracket exactly at a boundary) and `marginal_rates.ltcg` is computed from `bottom_with + qd + preferential_gain` even when there is no crypto preferential income. Display-only; no effect on tax. Fine to leave, but note it.

### Nit

**B-N1 — `TaxOutcome::NotComputable` drops the structured offending event.** It sets `event: None` and folds the real event into the `detail` string. Consider carrying the actual `EventId` for machine/programmatic consumption (C will read this).

**B-N2 — Re-verify line citations at task write time** (the plan's own §0 rule): `new_blockers_are_hard` (state.rs:206-217), `severity()` Hard arm (state.rs:50-60), `Disposal` (state.rs:107-116) are accurate today but decay each merge.

---

## 5. Scope / over-engineering — clean

No drift toward a full 1040: SS/IRMAA/AMT/QBI/AGI-phaseouts are explicitly out and labeled (I5). No new ledger events; side-tables and bundled data only. `marginal_rates` and `carryforward_consistency` (M4) are spec-mandated, not creep. TY2026 deliberately omitted (`TaxTableMissing` as the safety) — correct and conservative. OBBBA note is accurate: Pub. L. 119-21 left the 2025 brackets/breakpoints unchanged and B consumes post-deduction taxable income, so the OBBBA standard-deduction bump does not touch B's TY2025 numbers. No federal-tax scope deviation from the spec.

---

## 6. VERDICT

**NOT 0C/0I. 0 Critical, 1 Important (B-I1), 4 Minor, 2 Nit.**

- **All TY2025 figures verified correct** against ≥2 independent sources each (ordinary brackets all 5 statuses; §1(h) breakpoints all 5 statuses incl. the MFS=$300,000 special case; HoH-35%=$250,500 vs Single/MFS=$250,525). **No bracket/threshold is wrong.**
- **All statutory values verified** ($250k/$200k/$125k NIIT, 3.8%, $3,000/$1,500) and correctly classified as non-indexed; HoH NIIT correctly $200k.
- **All mechanics verified** against statute: §1222 netting, §1(h) stacking (incl. QD-share), §1411 NIIT (incl. crypto-ordinary-in-MAGI-not-NII), §1211/§1212(b) ST-first carryforward, incremental delta with no double-count. Goldens are hand-reproducible from the bundled tables.

**Blocking fix required before implementation:** resolve **B-I1** — widen the `TaxYearNotComputable` gate so an unresolved Hard blocker that contaminates the basis/term of a lot consumed in the computed year (notably out-of-year `ImportConflict`, and basis-affecting `DecisionConflict`) refuses the year. Recommended: gate on "any unresolved `severity()==Hard` blocker in the projection," with a KAT proving an out-of-year `ImportConflict` ⇒ `TaxYearNotComputable`. Address B-M1..M4 / B-N1..N2 in the same fold (none blocking individually). Re-review after the fold (including the last), per §2.

---

# Round 2 — re-review

**Artifact:** `design/IMPLEMENTATION_PLAN_rate_engine.md` (revised; "Fold record (R0 round 1)")
**Reviewer role:** independent R0 architect + US-tax (re-review after the fold, incl. the last — `STANDARD_WORKFLOW.md §2`)
**Date:** 2026-06-29
**Scope of round 2:** confirm B-I1 + the 4 Minors + 2 Nits are closed and the fold introduced no new defect. Per the task framing the TY2025 figures + statutory values + mechanics were independently verified CORRECT in round 1 and were NOT re-derived here except where a fold touched a value (only the Task-7 fixture's MAGI input — checked below).
**Gate question:** 0 Critical / 0 Important? **YES — 0 Critical, 0 Important, 0 Minor, 2 Nit.**

## 1. B-I1 — CLOSED ✅

- **Projection-wide gate present.** `compute_tax_year` step (1) refuses via `first_hard_blocker(state)` (plan lines 683-691, 806-808), which is `state.blockers.iter().find(|b| b.kind.severity() == Severity::Hard)` — keys on the general `severity()` classifier (`state.rs:47-64`), not an enumerated kind/year subset. Any unresolved Hard blocker anywhere ⇒ `TaxYearNotComputable`. Future Hard kinds auto-gate.
- **Genuinely refuses the round-1 leak.** An unresolved `ImportConflict` is `Severity::Hard` (`state.rs:53`) and lands in `state.blockers` (resolve pushes it at `resolve.rs:370-374`; `fold.rs:342-345` moves `res.blockers` onto the state), so `first_hard_blocker` catches it regardless of the conflict event's year. The exact round-1 hole (out-of-year `ImportConflict` / basis-affecting `DecisionConflict` on a non-`basis_pending` lot a later disposal consumes) is now gated.
- **Per-event/per-year enumeration fully deleted.** `hard_blocker_for_year` appears **only** in (a) this round-1 review and (b) the plan's fold record line 1514 (documenting the deletion) — **no residual in the plan body** (grep-confirmed). The per-event `tax_date`/`EventId`/`BTreeMap` scoping imports are dropped (Task-5 interface comment lines 660-661; Task-5 step-3 note line 936). `compute_tax_year` reads dates via `disposed_at.year()` / `recognized_at.year()` on `TaxDate` (= `time::Date`, `conventions.rs:10`) — no `tax_date()` call.
- **New KAT exercises the closed hole.** `refuses_year_with_out_of_year_import_conflict_on_consumed_lot` (lines 902-914): Acquire(2024) → unresolved ImportConflict(2024) targeting it → Dispose(2025) consuming it ⇒ asserts `TaxYearNotComputable`. The fixture is realizable (unresolved conflict leaves the original Acquire lot in the pool, `resolve.rs:362-377`; Acquire lots are `basis_pending: false`, `fold.rs:409`, so no incidental in-year `FmvMissing`) — the ImportConflict is the sole gating blocker, which is precisely the scenario that leaked before.
- **Conservatism documented.** Global Constraints line 19; §4.5 #6 (lines 1491); fold record line 1514; Task-5 doc comment lines 797-805. The per-year-granularity trade-off and the Option-2 lot-lineage recovery path are stated. This is exactly round-1 fix option #1 (the one I recommended). ✅
- **Determinism preserved.** `state.blockers` is sorted in `finalize` by `(kind, event, detail)` (a total order, `fold.rs:959-964`), so `.find()` returns a deterministic first blocker; the `TaxYearNotComputable` detail/event are deterministic (NFR4). ✅

## 2. Minors — all CLOSED ✅

- **B-M1 ✅** Render note (line 1330) reads "MAY UNDERSTATE NIIT; see §5 Phase-2 refinement"; Task-5 NIIT comment (lines 749-750) and §4.5 #4 (line 1489) state the direction (excluding crypto ordinary income from NII + not reducing NII by the §1211 loss can only understate). §5 (line 1500) + Task 11 step 4 file the Phase-2 follow-up.
- **B-M2 ✅** `render_tax_outcome` derives and prints `ordinary_rate_attributable = total − ltcg_tax − niit` (lines 1318-1319), so the three attributable components visibly sum to TOTAL. Reconciliation KAT `report_tax_year_components_reconcile_to_total` (lines 1346-1351). Left unnamed on `TaxResult` (spec-faithful). The identity is exact (see §4 below).
- **B-M3 ✅** `--magi-excluding-crypto` clap help + `TaxProfile::magi_excluding_crypto` doc state the §1411 contract (line 1240; Task-1 type comment line 142). Task-7 QD fixture now internally consistent: `magi_excluding_crypto = 530,000 = OTI 450,000 + QD 80,000` (lines 1124-1127). Asserted `ltcg_tax = 3,830.00` is MAGI-independent and re-derives correctly (without: 80,000@15% = 12,000; with: 83,400@15% + 16,600@20% = 12,510 + 3,320 = 15,830; delta = 3,830.00) — unchanged. ✅
- **B-M4 ✅** `marginal_rates` conventions pinned as display-only (`taxable > br.lower` reports the lower bracket at a boundary; `ltcg` reflects WITH-crypto top-of-stack even with no crypto preferential income) — level-vs-delta note line 811, §4.5.
- **B-N1 ✅** `TaxYearNotComputable` carries the offending `EventId` (`event: b.event.clone()`, line 687).
- **B-N2 ✅** Cites re-verified at fold time and confirmed by me against current source: `severity()` Hard set `state.rs:47-64`; unresolved-ImportConflict branch `resolve.rs:362-377`; ClassifyRaw DecisionConflict `resolve.rs:386-396`; `basis_pending`→`FmvMissing` `fold.rs:124-131`.

## 3. No tax figure changed ✅

Spot-checked Task 6 against the round-1-verified set: Single 37%@626,350 / MFJ@751,600 / MFS@375,800; HoH-35%@250,500 vs Single&MFS@250,525; §1(h) breakpoints Single(48,350;533,400) MFJ(96,700;600,050) HoH(64,750;566,700) MFS(48,350;300,000); statutory `niit_threshold` 250k/200k/125k, `NIIT_RATE` 0.038, `loss_limit` 3,000/1,500 — all identical to round 1. The only numeric edit is the Task-7 QD fixture MAGI input (450,000 → 530,000), which feeds NIIT only; that KAT asserts `ltcg_tax` (MAGI-independent), so its asserted output is unchanged. ✅

## 4. No new defect ✅

- **No over-blocking of computable years.** The gate filters `severity()==Hard`; Advisory kinds (`SafeHarborTimebar`, `UnmatchedOutflows`, `Pre2025MethodNote`, `state.rs:61`) do not trip it — a projection with only advisory blockers still computes. The all-years conservatism is intended/documented (matches round-1 option #1), not a defect.
- **Delta/level identity holds exactly.** `total = (ord_with+pref_with+niit_with) − (ord_without+pref_without+niit_without)` (line 767); `ltcg_tax = pref_with−pref_without`; `niit = niit_with−niit_without`. Therefore `total − ltcg_tax − niit ≡ ord_with − ord_without` — the pref and niit terms cancel by exact Decimal subtraction, and `ord_with`/`ord_without` are each `round_cents`-ed, so the printed `ordinary_rate_attributable` is exact with no extra rounding. ✅
- **Determinism / no-float intact.** `first_hard_blocker` over the sorted `state.blockers`; all rate math `Decimal`/`dec!`; no `HashMap` iteration, no `f32`/`f64` introduced.

## 5. Residual (non-blocking)

- **B-R2-N1 (Nit) — stale citation in §4.3.** Line 1469 still lists `tax_date(utc,tz)` (`conventions.rs:52`) among the symbols `compute_tax_year` consumes. After the B-I1 fold the function no longer calls `tax_date()` (it filters on `disposed_at.year()` / `recognized_at.year()`), so this entry is stale and contradicts the Task-5 "imports removed" note (lines 660-661, 936) and the fold's "all aligned" self-consistency claim (line 1520). Doc-only; no effect on the implementable code or any figure. Drop `tax_date(utc,tz)` from the §4.3 `compute_tax_year` symbol list (it remains correctly cited in the §1 grounding table at line 32, which is a general inventory).
- **B-R2-N2 (Nit, optional test-strengthening).** `refuses_year_with_out_of_year_import_conflict_on_consumed_lot` asserts only `kind == TaxYearNotComputable`. Since by fixture construction the unresolved `ImportConflict` is the sole Hard blocker, the test is correct as-is; adding an assertion that the carried blocker is that conflict (e.g. `b.detail.contains("ImportConflict")` or matching `b.event`) would prove the *out-of-year* conflict — not an incidental in-year blocker — did the gating. Strengthening, not a correctness gap.

## 6. VERDICT — Round 2

**0 Critical / 0 Important / 0 Minor / 2 Nit. B's plan is READY TO IMPLEMENT.**

B-I1 is closed by the projection-wide `first_hard_blocker` gate (round-1 recommended option #1), with the deleted enumeration confirmed gone from the plan body and a new KAT that genuinely exercises the formerly-leaking cross-year contamination path. All four Minors and both Nits are folded; no tax figure changed; the delta/level reconciliation identity is exact; determinism and no-float hold. The two residual Nits (a stale `tax_date` citation in §4.3; an optional KAT-strengthening assertion) are non-blocking and may be swept during implementation. No re-review gate remains open for the plan — proceed to phased TDD implementation, with Task 11's whole-diff review as the closing Phase-E gate.
