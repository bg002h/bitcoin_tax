# BRAINSTORM — what-if / synthesize-transaction tax-planning tool (task #43)

**Status: DESIGN — awaiting user approval before SPEC.** Recon map: (this session). Reuses the proven
non-persisted synthetic-disposal seam from `optimize consult` (optimize.rs `synthetic_state` :1230 /
`score_synthetic` :1274). Scope (user-chosen 2026-07-06): **sell what-if + harvest optimizer**, delivered as
**CLI + TUI overlay**.

## What it does
Posit a HYPOTHETICAL, NON-persisted transaction and see its MARGINAL federal tax effect on the current-year
position — without ever writing to the vault. Two verbs:

### `what-if sell` — the marginal effect of selling N BTC
`btctax what-if sell <N-btc> --at <DATE> --price <P> [--wallet W] [--method hifo|fifo|lifo|lots=…]`
Injects a synthetic `Op::Dispose` (HIFO-default lot pick, or explicit lots), folds, and reports the **marginal**:
- lots consumed (id, basis, acquired→sold, **term ST/LT**), ST gain / LT gain,
- **which §1(h) LTCG bracket** the LT gain lands in (0 / 15 / 20 %) and how much room remains in the current one,
- **NIIT** impact (does it cross the threshold / add 3.8%),
- **total marginal tax** = `withhyp.total_federal_tax_attributable − baseline.total_federal_tax_attributable`
  (exact Decimal; the no-crypto term cancels), and the **effective marginal rate** (marginal tax ÷ gain).
- **[★ §1212 LOSS CARRYOVER] the carryforward delta** = `withhyp.carryforward_out − baseline.carryforward_out`
  (ST + LT). A loss-harvesting sale's current-year tax UNDERSTATES its value: only $3,000 of net loss offsets
  ordinary income this year; the EXCESS carries FORWARD (ST/LT character preserved) to offset future gains/
  income. The report MUST surface this ("realizes $X loss → $3,000 used this year, **$Y carried to next year**")
  so a loss sale isn't judged solely on this year's tiny tax delta. `TaxResult.carryforward_out` +
  `TaxProfile.capital_loss_carryforward_in` (types.rs) already model it end-to-end; the what-if shows the delta.

### `what-if harvest` — the optimizer (the hard, correctness-critical core)
`btctax what-if harvest --target <zero-ltcg | fifteen-ltcg | gain=$X | tax=$X> --at <DATE> --price <P> …`
Finds the **maximum BTC to sell** that still satisfies the target — e.g. "the most I can sell while ALL the LT
gain stays in the 0 % LTCG bracket", or "…to realize exactly $X of gain", or "…to add no more than $X of tax".
Reports the amount + the same marginal breakdown at that amount, and the **binding constraint** (which bracket
edge / dollar figure stopped it).

## [★] Optimizer approach — Fable-architect DECIDED: segment-walk, NOT bisection
Full analysis: `design/agent-reports/fable-harvest-optimizer-advice.md`. My initial "just bisect" instinct was
**UNSOUND** and the architect proved it: under HIFO (highest per-sat basis first = most-underwater lots first),
realized-gain(N) is a convex VALLEY (losses realized first) and marginal-tax(N) is non-monotone with genuinely
FLAT plateaus — the $3k-loss pin (extra loss → $0 current tax) and the carryforward-absorption plateau (gains
silently burn carryforward at $0 tax). A standing FIFO/LIFO election even makes the feasible set NON-contiguous.
Global engine-bisection can converge to an arbitrary crossing or none. **Rejected.**

**The algorithm (all 4 targets share it; only the predicate differs) — analytic-locate / engine-verify segment walk:**
- **P0 feasibility+baseline:** as-of pool (`fold_as_of`/`pool_key`), `N_avail` capped at the first basis-pending
  lot, baseline `total(0)`, `AlreadyBreached` check. Refusals mirror `consult_sale`.
- **P1 one fold builds the schedule:** append a synthetic `Op::Dispose` of `N_avail` with NO injected selection
  (fold consumes by the STANDING method — reuse `method_order`/`hifo_cmp`, never re-implement), read the legs as
  the exact per-lot (slope, term, gift-zone) schedule.
- **P2 lot-edge walk:** `compute_tax_year` at every cumulative lot edge (≤ pool+1 folds); find the first edge
  where the predicate goes true→false. (Per **theorem T1** — tax(N) is monotone *within* one lot segment —
  checking edges bounds the interior, so the prefix is verified by the walk.)
- **P3 boundary inside that segment:** analytic linear solve as the SEED, sat-bisection (sound per T1) as the
  DECIDER, then a **mandatory engine verify** (never return an unverified N). ~35–100 folds total.
- **Answer semantics (all targets): max N such that the predicate holds on the ENTIRE PREFIX [0,N]** — the only
  safe definition under partial fills + non-contiguous feasible sets (coincides with "largest anywhere" under
  HIFO). Predicates: `zero-ltcg`⇒`at_15+at_20==0`; `fifteen-ltcg`⇒`at_20==0`; `gain=$X`⇒sale-local
  `st_gain+lt_gain≤X` (X≥0); `tax=$X`⇒`total(N)−total(0)≤X`.
- **ST/LT = full feedback by construction** (the predicate reads the engine's whole stacked position — ST gain
  shrinks the 0% room, ST loss expands it, QD fills it first). NOT LT-only (a v2 `--lt-only` via injected
  `res.selections`, deferred — it drags in the 2027+ broker-ID compliance machinery).

**[★ engine spec-delta] Expose the with-scenario `PrefSplit` (`at_0/at_15/at_20`) / `bottom_with` on
`TaxResult`** — `compute_tax_year` today keeps only `.tax` (compute.rs:342); `MarginalRates.ltcg` is NOT a
substitute (it reports a rate even with zero preferential dollars — disagrees in the vacuous case). Additive;
engine stays the single source of truth.

**[★ mandatory disclosures]** carryforward-burn (`carryforward_out(N*) − carryforward_out(0)`); NIIT kink (a
"0%/15% bracket" answer can still cost +3.8% — `MarginalRates.niit_applies`); plateau note. Status enum:
`Found | NotBinding | AlreadyBreached | NoLots | ProceedsRequired | PreTransitionYear | YearNotComputable(Blocker)`.

## The seam (non-persisted, proven)
`let mut res = resolve(events, prices, config); res.timeline.push(Eff{ op: Op::Dispose{…}, id:
EventId::Decision{seq: u64::MAX}, src_ref:"__synthetic__", … }); let state = fold(res, …);
compute_tax_year(&events, &state, year, profile, tables)`. This is exactly `synthetic_state` (optimize.rs:1230).
A new core fn returns **both** `TaxResult`s (baseline + withhyp) so the CLI/TUI show every marginal field
(`MarginalRates{ ordinary, ltcg 0/.15/.20, niit_applies }`, types.rs:73). New core module `btctax-core::whatif`
(sell + harvest) on top of the existing injector; `btctax-cli` `what-if` command + `btctax-tui` overlay.

## [★] Obstacles the design must handle (recon-flagged)
1. **Baseline subtraction** — DO the `compute_tax_year(baseline)` subtraction consult omits (else over-reports).
   FOLLOWUP question: also fix `optimize consult`'s reporting, or leave it? (proposal: fix its label/output too.)
2. **Table horizon** — `compute_tax_year` refuses `TaxTableMissing` if the target year's table isn't bundled.
   **RESOLVED for planning: bundled = TY2017, 2024, 2025, 2026** (`ty2026()` tax_tables.rs:500, Rev. Proc.
   2025-32). "Sell today" (2026) works out of the box — NO table work in v1. (Gap years 2018–2023 are
   unbundled → a what-if there refuses cleanly; historical, out of planning scope.)
3. **Profile required** — brackets are income-relative, so a `TaxProfile` (filing status + ordinary income +
   MAGI) is mandatory (`TaxProfileMissing`). Add inline flags `--filing-status` + `--income` (+ `--magi`) to
   build an AD-HOC non-persisted profile (mirrors `placeholder_tax_profile`, cmd/tax.rs:16) so a user can plan
   without `tax-profile set` first; fall back to the stored profile when present.
4. **Future-date price** — no dataset price for a future `--at` ⇒ require `--price P` (already `ConsultRequest.proceeds`).
5. **Hard-blocker gate** — a what-if on a vault with open Hard blockers refuses (`TaxYearNotComputable`) — correct, unchanged.

## Plan (phased; SPEC will detail)
- **P1 (core + CLI sell)** — `btctax-core::whatif::sell` (baseline + withhyp `TaxResult`s, marginal) + the
  ad-hoc profile + target-year table threading; `what-if sell` CLI + render; the TY2026 table if needed.
- **P2 (core + CLI harvest)** — the engine-bisection optimizer (all target types) + `what-if harvest` CLI + render.
- **P3 (TUI overlay)** — an interactive what-if panel in the viewer (amount input → live marginal; harvest target).
- Each phase: spec-slice → **Fable R0** (the marginal + optimizer correctness) → TDD → fault-injected whole-diff.

## KAT sketch (SPEC expands)
- `whatif_sell_marginal_subtracts_baseline` (a year WITH real disposals — the marginal ≠ the whole-year figure).
- `whatif_marginal_cancels_no_crypto_term` (marginal == tax(real+hyp) − tax(real), exact).
- `whatif_sell_reports_correct_ltcg_bracket` (0/15/20 by taxable-income stacking) + `…_niit_crossing`.
- **[★ §1212] `whatif_sell_loss_reports_carryforward_delta`** — a sale realizing a net loss > $3,000 shows
  $3,000 used against ordinary income this year and the EXCESS as `carryforward_out` delta (ST/LT preserved);
  the marginal current-year tax alone does NOT represent the sale's value. `…_carryforward_in_consumed_first`
  (a profile with an incoming loss carryforward offsets the hypothetical gain before brackets apply).
- `whatif_harvest_zero_ltcg_max_amount` (the largest sell whose LAST gain-dollar is still 0 %); `…_gain_target`;
  `…_tax_target`; monotonicity (harvest amount ≤ any amount that breaches the target).
- `whatif_never_persists` (vault byte-identical after any what-if) — the non-negotiable invariant.
- Non-persistence + blocker-refusal + ad-hoc-profile KATs.

## Non-goals (v1)
Buy/transfer/income hypotheticals; multi-year planning; state-tax; the §170 donation what-if (the "sell +
gift" option not chosen). Deferred to later.
