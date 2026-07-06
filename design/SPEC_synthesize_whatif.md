# SPEC — what-if / synthesize-transaction tax-planning tool (task #43)

**Source baseline:** `main` @ `283238f`. **Review status: R0-GREEN (2 rounds; 0C/0I).
Cleared to implement (P0 first).** Reviews: `reviews/R0-spec-whatif-round-{1,2}.md`. r1 0C/2I (Opus — the
algorithm rendered faithfully, no Fable re-consult; 2 report-signal baseline-subtraction Importants + minors);
r2 0C/0I/2M/2N (Opus — folds verified vs source; SemVer precision synced: this is a 0.4.0 breaking cycle).** Brainstorm: `design/BRAINSTORM_synthesize_whatif.md`. Optimizer algorithm (authoritative):
`design/agent-reports/fable-harvest-optimizer-advice.md`. Memory: [[future-synthesize-transaction-tax-planning]].
Reuses the proven non-persisted synthetic-disposal seam (`synthetic_state` optimize.rs:1230 / `score_synthetic`
:1274). Scope: **sell what-if + harvest optimizer**, **CLI + TUI overlay**, + the `optimize consult` marginal fix.

## Goal
Posit a HYPOTHETICAL, NON-persisted transaction and see its MARGINAL federal tax effect on the current-year
position — never writing the vault. Everything routes through `compute_tax_year` (the single audited tax
engine); the tool invents no tax authority. Refuses exactly where the engine does (Hard blocker, missing
table/profile, pre-2025, future date without price).

## [★ engine delta — additive, required] Expose the with-scenario LTCG split on `TaxResult`
`compute_tax_year` computes the §1(h) `PrefSplit{ at_0, at_15, at_20 }` + the stack bottom internally
(compute.rs:53-95, 342) but KEEPS ONLY `.tax`. The harvest predicates need `at_15`/`at_20` == 0 exactly.
**Add `pref_split: PrefSplit` (and `bottom_with: Usd`) to `TaxResult`** (types.rs:90). `MarginalRates.ltcg` is
NOT a substitute — it reports a rate from `top` even with ZERO preferential dollars (compute.rs:383), disagreeing
with the vacuous `zero-ltcg` case. Pure surfacing of already-computed values; every existing tax number is
unchanged (regression KAT: all current tax KATs byte-identical).

## Core module `btctax-core::whatif`
Two entry fns, both returning BOTH the baseline and with-scenario `TaxResult` so callers show every marginal
field. Both build on a shared injector `synthetic_year(events, prices, config, year, profile, tables, dispose,
picks: Option<…>) -> Result<(TaxResult /*baseline*/, TaxResult /*withhyp*/), WhatIfError>` = the `synthetic_state`
push + two `compute_tax_year` calls (baseline on the unmodified timeline). **[R0-M2] `whatif` lives IN
btctax-core**, giving it crate-internal access to the today-private `synthetic_state`/`fold_as_of`/`pool_key`/
`method_order`/`hifo_cmp` (call directly, or lift to `pub(crate)` — no public API widening). **[R0-M1] proceeds
scale with the candidate N** (unlike `ConsultRequest`'s fixed total): `--price` is per-BTC → each candidate's
`proceeds = round_cents(price × N / 1e8)`; `price: None` (dataset date) → `fmv_of` prices each candidate N. **Marginal = `withhyp.total_federal_tax_
attributable − baseline.total_…`** (exact Decimal; the shared no-crypto term cancels — the identity the
`consult` bug violates).

### `whatif::sell(req: SellRequest) -> Result<SellReport, WhatIfError>`
`SellRequest { sell_sat, wallet, at, price: Option<Usd/*per BTC*/>, method: Option<LotMethod|explicit lots> }`.
Injects the synthetic `Op::Dispose` (HIFO default via the standing method, or explicit `res.selections`).
`SellReport`:
- lots consumed: per-leg `(lot_id, sat, basis, acquired→sold, term ST/LT, gain)`;
- `st_gain`, `lt_gain`; the with-scenario `pref_split` → **which §1(h) bracket (0/15/20)** + room remaining;
- `marginal_tax` (exact), `effective_rate` (= marginal ÷ gain, guarded for gain≤0);
- **[★ §1212, R0-I1] `carryforward_delta` = withhyp.carryforward_out − baseline.carryforward_out** (ST + LT).
  The disclosure is DELTA-BASED, never a hard-coded "$3,000": the **this-year ordinary offset** =
  `withhyp.loss_deduction − baseline.loss_deduction` (reuse `TaxResult.loss_deduction`, types.rs:104) — which
  is **$0 when the baseline already consumes the §1211(b) cap** (pre-existing real losses or a carryforward-in).
  Report "$<offset_delta> offsets ordinary income this year, **$<carryforward_delta> carried to next year**";
  the current-year marginal alone does NOT represent a loss sale's value.
- **[★ R0-I2] `niit_incremental`** = `withhyp.niit − baseline.niit` (reuse `TaxResult.niit`, types.rs:102) —
  the DELTA, NOT the raw `MarginalRates.niit_applies` (which is crypto-vs-no-crypto and misreports "NIIT
  applies" for a NIIT-REDUCING loss harvest on a year with real disposals). `niit_applies` in the report ≙
  `niit_incremental != 0`; show the sign. `status`.

### `whatif::harvest(req: HarvestRequest) -> Result<HarvestReport, WhatIfError>`
`HarvestRequest { wallet, at, price: Option<Usd/*per BTC*/>, target: HarvestTarget }`,
`HarvestTarget = ZeroLtcg | FifteenLtcg | Gain(Usd) | Tax(Usd)`. **Algorithm = the architect's segment walk
(follow `fable-harvest-optimizer-advice.md` §2 exactly):**
- **P0** as-of pool (`fold_as_of`/`pool_key`); `N_avail` = Σ remaining_sat in consumption order truncated at
  the first basis-pending lot; baseline `total(0)`; if predicate fails at N=0 ⇒ `AlreadyBreached`.
- **P1** ONE synthetic fold of `N_avail` with NO injected selection (fold consumes by the STANDING method —
  reuse `method_order`/`hifo_cmp`, never re-implement) → read legs as the per-lot schedule.
- **P2** `compute_tax_year` at every lot edge (≤ pool+1 folds); first edge with predicate true→false.
  (Theorem **T1** — tax(N) monotone within one lot segment — makes the edge checks bound the interior.)
- **P3** boundary inside that segment: analytic linear solve as SEED, sat-bisection (sound per T1) as DECIDER,
  **mandatory final engine-verify** — never return an unverified N. Tolerance τ (documented, e.g. 1,024 sats
  < $0.05 tax).
- **Answer semantics (ALL targets): max N such that the predicate holds on the ENTIRE PREFIX [0,N]** (safe
  under partial fills + non-contiguous feasible sets under a FIFO/LIFO election). Predicates:
  `ZeroLtcg`⇒`pref_split.at_15+at_20==0`; `FifteenLtcg`⇒`at_20==0`; `Gain(X)`⇒sale-local `st_gain+lt_gain≤X`
  (require X≥0); `Tax(X)`⇒`marginal(N)≤X` (require X≥0).
- **ST/LT: full feedback by construction** (predicate reads the engine's whole stacked position). NOT LT-only.
- `HarvestReport { n_sat, n_btc, with_result: TaxResult, binding_constraint, marginal_tax, carryforward_delta,
  niit_incremental (= withhyp.niit − baseline.niit, per R0-I2 — NOT the raw flag), plateau_note, status }`.
- **Status enum:** `Found | NotBinding | AlreadyBreached | NoLots | ProceedsRequired | PreTransitionYear |
  YearNotComputable(Blocker)`.
- **Mandatory disclosures** in the report: carryforward-burn (`carryforward_out(N*)−(0)`); NIIT kink (a
  bracket-target answer can still cost +3.8% — surface `niit_incremental` [R0-I2 delta] + the approx crossing);
  the plateau note.

## [★ consult fix — user-approved] Correct `optimize consult`'s marginal reporting
`ConsultReport.total_federal_tax_attributable` (optimize.rs:1203) is the WHOLE-YEAR figure (real + hyp vs no
crypto), NOT marginal-vs-baseline → over-reports the hypothetical's own effect on a year with existing
disposals. **Fix:** compute the baseline `compute_tax_year` and add a `marginal_tax` field (= the same exact
subtraction); render it as the headline, keeping the whole-year figure clearly relabeled. KAT: on a year WITH
real disposals, consult's marginal ≠ its whole-year figure.

## CLI (`btctax-cli`)
New read-only `Command::WhatIf(WhatIf)` with `Sell { sell, wallet, at, price, method }` and `Harvest { wallet,
at, price, target }` (mirrors the `Optimize::Consult` clap shape, cli.rs:294). **Ad-hoc profile:** flags
`--filing-status`, `--income` (ordinary taxable), `--magi` (+ optional `--carryforward-in`) build a NON-persisted
`TaxProfile` (mirrors `placeholder_tax_profile`, cmd/tax.rs:16) so a user can plan without `tax-profile set`;
fall back to the stored `Session::tax_profile(year)` when present + no flags. **[R0-M4] `--magi` defaults to
`--income`** (a floor — NEVER $0, which would silently suppress every NIIT disclosure) with a printed caveat
"MAGI assumed = ordinary income; NIIT may be understated if you have other MAGI". `--price` required for a future
`at`. Renders via new `render::render_whatif_sell` / `render_whatif_harvest` (the `render_consult` template,
render.rs:1740). Reuses `Session` load verbatim (read-only).

## TUI overlay (`btctax-tui`, phase P3)
An interactive what-if panel in the viewer: an amount input → live `SellReport` (marginal + carryforward +
bracket), and a harvest-target selector → `HarvestReport`. Reuses the core module; no new tax logic. Read-only;
never mutates the vault. (Detailed TUI state/keybindings in the P3 spec-slice at implementation.)

## KATs (the non-monotone traps FIRST — from the architect report §6)
- **★ non-persistence:** `whatif_never_persists` (vault byte-identical after any sell/harvest) — non-negotiable.
- **★ marginal identity:** `whatif_marginal_equals_withhyp_minus_baseline` (exact) + `…_cancels_no_crypto_term`.
- **★ consult fix:** `consult_marginal_subtracts_baseline` (year with real disposals).
- **★ §1212:** `whatif_sell_loss_reports_carryforward_delta` (offset-delta used, excess carried, ST/LT
  preserved); **[R0-I1] `whatif_sell_offset_delta_is_zero_when_baseline_caps`** (baseline already consumes the
  §1211(b) cap → the sell's this-year ordinary offset = `loss_deduction` delta = $0, ALL carried — NOT "$3,000");
  `carryforward_in_consumed_first`; `harvest_carryforward_burn_disclosed` (cf_long=$50k, all-gain pool →
  marginal $0 across absorption, report shows the burn).
- **[R0-I2] `whatif_niit_incremental_not_raw_flag`** — a NIIT-REDUCING loss harvest on a year with real
  disposals: `niit_incremental < 0`, and the raw `MarginalRates.niit_applies` (which would say "applies") is
  NOT used.
- **★ optimizer traps:** `harvest_dip` (loss-lot-first → marginal<0 then rising; naive bisection lands wrong);
  `harvest_fifo_non_contiguous` (true→false→true; prefix semantics returns the FIRST boundary);
  `harvest_3k_pin_flat`; `harvest_st_feedback_shrinks_zero_room`; `harvest_cross_net_expands_room`;
  `harvest_qd_stacking`; `harvest_dual_basis_gift_zones` (gain/loss/NGNL); `harvest_two_edge_0_15_20`;
  `harvest_niit_kink`; `harvest_per_segment_monotone` (T1 within the cent band); `harvest_boundary_exactness`
  (predicate true at N*, false at N*+τ').
- **status/refusal:** `harvest_all_loss_notbinding` + $3k disclosure; `harvest_no_lots`; `harvest_already_breached`;
  `harvest_pending_basis_caps_n`; missing profile/table ⇒ refusal; MFS $1,500 variant; FIFO/LIFO election;
  **[R0-M5] Qss→Mfj status mapping** inherited (the breakpoint/threshold lookup).
- **sell:** `sell_reports_correct_ltcg_bracket` (0/15/20 by stacking); `sell_niit_crossing`; `sell_effective_rate`.
- **engine delta:** `taxresult_pref_split_matches_internal`; **all existing tax KATs byte-identical** (regression).

## Scope / SemVer / lockstep
btctax-core (+`whatif` module, +`pref_split`/`bottom_with` on `TaxResult`) + btctax-cli (`what-if` command +
ad-hoc profile + render + the consult fix) + btctax-tui (P3 overlay). No persistence surface. **[R0-M3+r2-m1/m2 SemVer]
This is a 0.3.0→0.4.0 (breaking) cycle REGARDLESS** — adding pub fields to the re-exported, non-`#[non_exhaustive]`
`TaxResult` is breaking, and adding `#[non_exhaustive]` to an already-exposed struct is *itself* breaking. So:
bump to **0.4.0**, AND add `#[non_exhaustive]` to `TaxResult` + `PrefSplit` in THIS cycle to future-proof all
*future* field additions (it does not make this release non-breaking). `PrefSplit` is ALREADY public at
`btctax_core::tax::PrefSplit` (tax/mod.rs:11) — the action is a crate-ROOT re-export for parity with `TaxResult`. Man page + README (`what-if`, the ad-hoc profile, the disclosures).
Network isolation unchanged (all pure compute). No new tax authority — cites the same tables.

## Plan (TDD, phased; each slice: R0-cleared → tests-first → whole-diff)
- **P0 (engine delta)** — `PrefSplit`/`bottom_with` on `TaxResult`; the surfacing + the regression KAT (all
  existing tax numbers unchanged).
- **P1 (core + CLI sell)** — `whatif::sell` + `synthetic_year` + the ad-hoc profile + the marginal/carryforward/
  bracket/NIIT report; `what-if sell` CLI + render; the consult fix; the sell + non-persistence + §1212 KATs.
- **P2 (core + CLI harvest)** — the segment-walk optimizer (P0-P3, all 4 targets, prefix semantics, status enum,
  disclosures) + `what-if harvest` CLI + render; the full trap-KAT battery.
- **P3 (TUI overlay)** — the interactive panel (spec-slice at impl); reuse the core.

## Gotchas
- **[★ NOT bisection]** the optimizer is a segment walk (analytic-seed + per-segment sat-bisection + engine-verify);
  global bisection is UNSOUND (non-monotone: loss-first, $3k pin, carryforward burn). Prefix semantics.
- **[★ engine-verify]** never return an N the engine didn't fold + verify true; the analytic solve is only a seed.
- **[★ §1212 carryforward]** surface the carryforward delta (a loss sale's value ≠ this-year tax); disclose the burn.
- **[★ marginal subtraction]** always subtract the baseline (the consult bug); the no-crypto term cancels exactly.
- **[reuse standing method]** the schedule follows `applicable_method`/`method_order` — never re-implement HIFO.
- **[NIIT kink]** disclose on bracket targets (a 0%/15% answer can cost +3.8%).
- **[non-persistence]** clone-fold-discard; sentinel `EventId::Decision{u64::MAX}`; vault byte-identical.
- **[refusals]** inherit the engine/consult taxonomy verbatim; ad-hoc profile clears only `TaxProfileMissing`.
