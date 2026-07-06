# Harvest-optimizer algorithm — architect's advice (read-only recon)

Author: tax-engine architect agent (Fable), 2026-07-06. Advisory only — nothing here is implemented.
All citations verified against the working tree at commit `b0fffcc` (branch `feat/tui-edit-chunk3`).

Scope: the `--target zero-ltcg | fifteen-ltcg | gain=$X | tax=$X` harvest tool — "find the MAX
BTC to sell (hypothetically, non-persisted) such that the target constraint holds," computed
through the proven synthetic-`Op::Dispose` seam
(`synthetic_state`, `crates/btctax-core/src/optimize.rs:1230`; `compute_tax_year`,
`crates/btctax-core/src/tax/compute.rs:228`).

---

## 0. Executive recommendation (one paragraph)

**Do not bisect the engine over N. Use a lot-edge segment walk with the engine as the only
oracle.** Build the exact consumption schedule for the wallet's as-of pool under the *standing
lot method* (HIFO default — `hifo_cmp`, `crates/btctax-core/src/project/pools.rs:274`; order via
`method_order`, `pools.rs:249` — reuse it, never re-implement it), evaluate `compute_tax_year`
at every lot-edge N (≤ pool-size + 1 calls), find the first segment where the target predicate
transitions true→false, then resolve the boundary *inside that one segment* by sat-level
bisection — sound there because tax(N) and gain(N) are **monotone within a single lot segment**
(proved in §1) — with an analytic linear solve as the bisection seed and a mandatory final
engine verification of the returned N. Answer semantics for every target: **the largest N such
that the predicate holds on the entire prefix [0, N]** (engine-evaluated at all lot edges ≤ N
plus the boundary). Global bisection is unsound because marginal-tax(N) and realized-gain(N)
are **not monotone** (§1), and under a FIFO/LIFO standing election they are not even unimodal,
so the feasible set can be non-contiguous.

---

## 1. Q1 — Monotonicity verdict: NOT monotone. Bisection over [0, N_avail] is UNSOUND.

### 1.1 Realized-gain(N) is not monotone

Fix the hypothetical per-sat price `p` (pro-rata proceeds allocation makes every leg's per-sat
proceeds equal — `make_disposal_legs`, `crates/btctax-core/src/project/fold.rs:122`,
`split_pro_rata`, `conventions.rs:34`). Lot *i* in consumption order contributes a constant
per-sat gain slope `s_i = p − b_i` (per-sat gain basis `b_i`), so cumulative realized gain
G(N) is piecewise-linear with slope `s_i` on lot *i*'s segment.

HIFO consumes **highest per-sat gain basis first** (`hifo_cmp`, pools.rs:274 — per-sat basis
DESC, cross-multiplied, basis-pending last). Highest basis at a fixed price = most underwater.
So the first lots consumed are exactly the loss lots: `b_1 ≥ b_2 ≥ … ⇒ s_1 ≤ s_2 ≤ …` —
**slopes ascending** ⇒ G(N) is **convex, valley-shaped**: it *decreases* while `b_i > p`
(loss lots), bottoms out where per-sat basis crosses the price, then increases. Any pool
holding at least one underwater lot makes G(N) non-monotone. Confirmed exactly as the prompt
suspected.

### 1.2 Marginal-tax(N) is not monotone — and has exactly-flat plateaus

`marginal(N) = total(N) − total(0)` is exact (the shared no-crypto scenario inside each
`compute_tax_year` cancels; compute.rs:315-330, 378). Along the schedule it is piecewise-linear
in N with kinks at: lot edges; §1222 netting sign changes and the cross-net regime flips
(`net_1222`, compute.rs:148-166); the §1211(b) $3,000/$1,500 cap (compute.rs:174-178,
`loss_limit`, tables.rs:204); ordinary bracket edges (`ordinary_tax_on`, compute.rs:24); the
§1(h) `max_zero`/`max_fifteen` breakpoints measured on TOTAL taxable income
(`preferential_tax`, compute.rs:53; `LtcgBreakpoints`, tables.rs:41); and the §1411 MAGI
threshold + NII cap (the `niit` closure, compute.rs:362-373; `niit_threshold`, tables.rs:190).

Three structurally non-monotone behaviors, all traceable to `net_1222` (compute.rs:133):

- **The dip.** HIFO realizes losses first ⇒ marginal(N) goes *negative* (offsets baseline
  gains dollar-for-dollar via cross-netting, compute.rs:148-166; then up to $3k against
  ordinary income, compute.rs:174) before later gain lots pull it back up.
- **The $3k pin (flat-at-a-loss).** Once net loss exceeds `loss_limit`, `loss_deduction` pins
  (compute.rs:174-178): additional loss changes *current-year* tax by exactly $0 — only
  `st_carry`/`lt_carry` grow (compute.rs:191-192). marginal(N) is exactly flat there.
- **The absorption plateau (flat-at-a-gain).** After the sale's own losses (or a profile
  carryforward, compute.rs:143-145) put the year in a net-loss position deeper than $3k, the
  next gain dollars merely shrink the carryforward: current-year tax unchanged ⇒ marginal(N)
  flat while **economic value (carryforward) is silently burned**. This is the single most
  dangerous trap for a "max N such that tax ≤ $X" tool — see §5 (disclosures) and the KATs.

**Verdict: marginal-tax(N) is non-monotone and piecewise, with genuinely flat plateaus.
Bisection over the full range with `compute_tax_year` as a black-box oracle is UNSOUND — it
can converge to an arbitrary crossing, or to no crossing at all on a plateau. Say so in the
spec in bold.**

### 1.3 Two structure theorems the spec should state (and test)

**(T1) Per-segment monotonicity.** Within a single lot segment the sign of d(tax)/dN equals
the sign of that lot's per-sat gain slope, in every regime:
- A gain dollar either (a) shrinks a >$3k net loss — tax flat (slope 0); (b) shrinks
  `loss_deduction` under the cap — raises `bottom` ⇒ ordinary tax up (compute.rs:336) and NII
  up (compute.rs:352-353); or (c) survives netting — taxed ≥ 0 at ordinary or §1(h) rates,
  NIIT ≥ 0. Never negative.
- A loss dollar symmetrically never increases tax (offset → deduction-to-cap → flat).
So tax(N) is monotone (non-strict) *inside* each segment, up to the cent-rounding band of
§4.3. **This is what licenses sat-level bisection inside the boundary segment — and only
there.**

**(T2) HIFO unimodality (do not rely on it).** Under pure HIFO with the fixed per-sat price,
segment slope signs follow the pattern `(zero|negative)* (zero|positive)*` — including the
§1015 dual-basis gift zones (gain zone / loss zone / no-gain-no-loss, fold.rs:149-202): a
positive-slope lot can never precede a negative-slope lot in `hifo_cmp` order (a gain-zone lot
first ⇒ every later, lower-basis lot is also a gain; an NGNL lot cannot follow a gain lot).
Hence tax(N) and G(N) are valley-shaped (quasi-convex) under HIFO and the feasible set of
`tax ≤ X` (X ≥ 0) is contiguous from 0. **But**: the taxpayer may have a standing FIFO/LIFO
election (`fold::applicable_method`, fold.rs:30; `method_order`, pools.rs:249-268), under
which slope order is arbitrary and the feasible set can be **non-contiguous** ("tax ≤ X" true,
then false, then true again). The algorithm must not assume T2; the segment walk (§2) is
method-agnostic and costs almost nothing extra.

---

## 2. Q2 — The algorithm: analytic-locate / engine-verify segment walk (refined option B)

### 2.1 Why not the alternatives

- **(A) Global engine bisection:** unsound per §1. Even under HIFO's unimodality it silently
  breaks the moment the user has a FIFO/LIFO election, a dual-basis gift lot mid-order, or a
  target with a non-contiguous feasible set. Reject.
- **Pure analytic (no engine at the boundary):** requires re-implementing `net_1222`,
  `preferential_tax`, the NIIT closure, the four-zone gift logic, pro-rata cent rounding, QD
  stacking, carryforwards… Guaranteed drift; violates the project ethos that a number is
  presented only if the engine computed it. Reject as the *decider* — keep it as the *seed*.
- **Per-N re-optimized lot selection (running `consult_sale`'s min-tax pick inside the
  search):** destroys the schedule structure entirely (no fixed consumption order ⇒ no
  segments, min-of-PL functions ⇒ worse non-monotonicity) and would propose a filing position
  the standing order doesn't produce. Reject for v1. (Post-hoc: after finding N*, optionally
  run `consult_sale(N*)` to *suggest* a better explicit identification — outside the loop.)

### 2.2 The recommended algorithm (all targets share it; only the predicate differs)

**Phase 0 — feasibility + baseline.**
Reuse `fold_as_of` + `pool_key` filtering (optimize.rs:1214, 1124-1137) for the wallet's as-of
pool. `N_avail` = Σ `remaining_sat` in consumption order **truncated at the first
basis-pending lot** (consuming one fires the `FmvMissing` Hard blocker, fold.rs:141-148 ⇒
`compute_tax_year` refuses, compute.rs:242). Compute the baseline `total(0)` on the unmodified
timeline. Refusals mirror `consult_sale` (optimize.rs:1111): pre-transition year, empty pool
(`NoLots`), future date with no dataset price and no `--price` (`ProceedsRequired`), any Hard
blocker anywhere (`YearNotComputable`). If the predicate already fails at N=0 → return
`AlreadyBreached` (see §5).

**Phase 1 — build the schedule with ONE engine fold.**
Append a synthetic `Op::Dispose` of `N_avail` (fee $0, `DisposeKind::Sale`, sentinel
`EventId::Decision{seq: u64::MAX}` — optimize.rs:1244) **without injecting a selection**, so the
fold consumes by the standing method (pools.rs:67, fold.rs:30) — the schedule *is* the filing
default, tie-breaks and dual-zone classification included. Read the disposal's legs in order:
per-leg `(lot_id, sat, per-sat slope, term, gift_zone)`. Zones and terms are stable across N
because per-sat proceeds are constant (pro-rata) and the date is fixed. Pass
`proceeds: None` to let `fmv_of` price each candidate N consistently, or
`proceeds = round_cents(price_per_btc × N/1e8)` when `--price` is given.

**Phase 2 — lot-edge walk.**
Evaluate `compute_tax_year` (via `synthetic_state` + fold, exactly the `score_synthetic` path,
optimize.rs:1274) at every cumulative lot edge `E_0=0 < E_1 < … < E_K = N_avail`. Scan from 0:
find the first edge pair `(E_j, E_{j+1})` with predicate true at `E_j`, false at `E_{j+1}`.
If the predicate holds at every edge including `N_avail` → answer `N_avail`, status
`NotBinding` ("target does not bind — the full position fits"). Per T1, checking the edges
bounds the interior (up to the §4.3 cent band), so the prefix condition is verified by the
walk itself — no interior probes needed.

**Phase 3 — boundary resolution inside the segment.**
Within `(E_j, E_{j+1}]` the predicate is monotone (T1). Compute the **analytic candidate**
(linear solve on the segment's exact-Decimal slope against the current regime's threshold —
e.g. for `gain=$X`: `N* ≈ E_j + (X − G(E_j))/s_{j+1}`, floored to a sat) as the seed, then
**sat-level bisection** with the engine as oracle, then the mandatory verify step: the
returned N* MUST satisfy `predicate(engine(N*)) == true` (hard invariant — never return an
unverified N), and the spec documents a tolerance τ (see §4.3) within which N* is the maximum.
Cost: (K+1) + ~⌈log₂(segment sats)⌉ ≤ ~35 extra folds — trivially cheap next to
`consult_sale`'s up-to-4096-candidate enumeration (optimize.rs:327-433).

**Phase 4 — report** (§5): N* in sats/BTC, the with-N* `TaxResult`, the binding constraint,
and the mandatory disclosures (carryforward burn, NIIT crossing, plateau note).

### 2.3 Per-target predicate, well-posedness, and precise answer semantics

Shared semantics: **`answer = max { N ∈ [0, N_avail] : predicate(engine(n)) holds ∀ n ∈ [0, N] }`**
(prefix semantics), with `n` probed at lot edges + the boundary (sufficient per T1).
Rationale: (a) it is the only semantics that stays safe under a partial fill — if the user's
real order executes for less than N*, every smaller n still satisfies the target; (b) it is
well-posed for every target even when the feasible set is non-contiguous (FIFO/LIFO, §1.3);
(c) under HIFO+T2 it coincides with "largest N anywhere" for X ≥ 0, so nothing is given up in
the common case. "Largest N anywhere" is REJECTED: under a FIFO election it can return an N
whose smaller executions violate the target — indefensible for a pre-trade tool.

| Target | Predicate (with-N engine scenario) | Well-posed? | Notes |
|---|---|---|---|
| `zero-ltcg` | `PrefSplit.at_15 + at_20 == 0` — zero preferential dollars above `max_zero` | Yes, under prefix semantics | Vacuously true when `qd + preferential_gain == 0`. Do **not** use `MarginalRates.ltcg` (compute.rs:383-389): it reports a rate from `top` even when there are zero preferential dollars, so it disagrees with the vacuous case. Requires exposing the with-scenario `PrefSplit` (or `bottom_with`) on `TaxResult` — today `compute_tax_year` keeps only `.tax` (compute.rs:342). Additive engine change; keep the engine the single source of truth. |
| `fifteen-ltcg` | `PrefSplit.at_20 == 0` | Yes | NIIT is **not** part of the predicate (a "15%-bracket" dollar can cost 18.8%) — disclose separately, §5. |
| `gain=$X` | sale-local realized net gain `st_gain + lt_gain ≤ X` (the synthetic disposal's own legs, signed — the `evaluate_disposal` split, optimize.rs:1284) | Yes for X ≥ 0 | v1: require X ≥ 0 (G(0)=0; X<0 makes the prefix set empty — reject with a hint that loss-harvesting is a different target). Deliberately sale-local, not year-net: "realize at most $X of gain *with this sale*". A future `year-gain=$X` flag can target `st_net+lt_net` instead — name them distinctly. |
| `tax=$X` | `marginal(N) = total(N) − total(0) ≤ X` | Yes for X ≥ 0 under prefix semantics; this is *the* target the non-contiguity warning exists for | `tax=$0` is the flagship harvest primitive ("sell as much as possible adding zero federal tax") — it rides the dip + plateaus, so the carryforward-burn disclosure (§5) is load-bearing. |

---

## 3. Q3 — ST vs LT interaction: answer (b), full feedback, by construction

"Max sellable in the 0% bracket" must be defined on the **whole resulting filing position**,
not on LT lots in isolation: the correct, statutorily defensible definition is —

> the maximum N such that, after the hypothetical sale is folded into the year, the §1(h)
> stack — ordinary taxable income + crypto ordinary income + surviving net ST gain − the
> §1211(b) deduction as the bottom (compute.rs:336), with QD + surviving net LT gain stacked
> on top (compute.rs:342) — leaves **zero preferential dollars above `max_zero`**.

Because the predicate is evaluated on the engine's with-scenario `PrefSplit`, every feedback
channel is automatically priced in: ST gain raises `bottom_with` dollar-for-dollar and
shrinks the 0% room (compute.rs:66-74); ST **loss** cross-nets against LT gain and *expands*
the room (compute.rs:158-165); the §1211 deduction lowers the bottom; QD occupies room first;
`other_net_capital_gain` and both carryforward characters flow through `net_1222`
(compute.rs:143-145).

Option (a) — "only consider LT lots" — is wrong twice: HIFO/the standing method will not
consume only LT lots (simulating a consumption the fold won't produce misstates the filing
position), and it ignores the ST-feedback shrinkage. Option (c) worth naming in the spec as a
**v2**: `--lt-only`, which *injects an explicit LT-only selection* through the existing
`res.selections` seam (optimize.rs:1263) — the engine supports it today — but it inherits the
specific-ID compliance machinery (2027+ broker prohibition, contemporaneous lever) already
modeled in optimize.rs:444-459, so it must ship with those disclosures, not before.

---

## 4. Q4 — Exactness and the boundary

### 4.1 Mid-lot boundaries are the generic case, and they are well-defined

Bracket edges have no reason to align with lot edges, so the boundary generically falls
**mid-lot**. That is fully well-defined: the sat is the atomic unit (`Sat = i64`); a partial
consume takes pro-rata basis with remainder-conservation (pools consume; `split_pro_rata`,
conventions.rs:34-47 — rounded part + exact remainder, Σ conserved), and the leg math is exact
Decimal (`round_cents` HALF_EVEN at defined points only). Any N in [0, N_avail] folds to an
exact, deterministic `TaxResult`.

### 4.2 Which quantities are exact vs cent-quantized

- Bracket-dollar quantities (`at_0/at_15/at_20`) are **unrounded Decimals**
  (compute.rs:67-87 — only `.tax` is rounded at :88): the `zero-ltcg`/`fifteen-ltcg`
  predicates are exact, no rounding pitfall.
- Leg gains are cent-rounded per leg (fold.rs:166,176,197), and proceeds allocation uses
  remainder-takes-the-rest (fold.rs:133-140): `crypto_st/lt` — hence `gain=$X` and `tax=$X` —
  are cent-quantized functions of N.

### 4.3 The rounding band — the one real pitfall

Because each leg rounds independently and the pro-rata remainder migrates as N changes, the
sat-granular tax/gain sequence can wiggle **non-monotonically by up to ~⌈legs/2⌉ cents**
inside a segment. Consequences the spec must encode:

- One sat ≈ $0.0005–0.001 of proceeds at current prices ⇒ cent quantization creates
  **sat-plateaus**; near-zero-slope lots (basis ≈ price) can make the plateau span most of a
  lot. Plateaus are harmless for bisection (non-strict monotonicity suffices); the *wiggle*
  is what can mislocate the boundary by a handful of sats.
- Therefore: "the last gain-dollar still at 0%" (and every boundary) is defined **with
  respect to the engine's arithmetic, not an idealized real-valued model**. Hard invariant:
  the returned N* is engine-verified true. Documented tolerance: N* is the maximum within τ
  sats (τ small, e.g. 1,024 — worth < $0.05 of tax, sub-materiality), found by bisection +
  a bounded final verify. Never burn thousands of folds chasing a cent.
- The analytic seed can be off by a few sats vs the engine (leg rounding + remainder
  placement + `round_cents(price × N)` proceeds quantization) — which is exactly why it is a
  seed, never the decider.

---

## 5. Q5 — Degenerate/edge cases the spec must handle

- **All-loss position.** Every target with X ≥ 0 holds ∀N ⇒ answer `N_avail`, status
  `NotBinding`. Mandatory disclosure: only `loss_limit` ($3,000/$1,500 MFS, tables.rs:204) is
  deductible this year; the rest is `carryforward_out` (report short/long split,
  compute.rs:400-403).
- **Sale crossing TWO bracket edges (0→15→20) inside one lot.** Handled natively: multiple
  kinks inside one segment don't break T1 (slope magnitude changes, sign doesn't), so the
  segment bisection stays sound. For `fifteen-ltcg` the predicate is `at_20 == 0` regardless
  of how many edges the segment spans. Report the full split at N*.
- **NIIT threshold crossing independent of the LTCG edge.** A kink at MAGI = threshold
  (compute.rs:362-374) not aligned with any §1(h) edge. For `tax=$X` it is just another kink
  the engine prices. For the bracket targets it does **not** move the predicate — but the
  report MUST disclose "NIIT applies at N* / begins at N ≈ …" or the "0%/15% bracket" answer
  misleads by up to 3.8 points. (`MarginalRates.niit_applies`, types.rs:83, is the
  incremental flag to surface.)
- **Zero available BTC / insufficient pool.** `N_avail == 0` ⇒ `NoLots` status (mirror
  `OptimizeError::NoLots`, optimize.rs:1137-1139).
- **Target already breached at N=0.** E.g. QD alone or `other_net_capital_gain` or baseline
  in-year crypto already puts `at_15 > 0` for `zero-ltcg`; or baseline marginal 0 > X. Return
  N=0, status `AlreadyBreached`, with the baseline split and the overage — do not report a
  bare "0" without the why.
- **Loss carryforward in the profile.** `cf_short/cf_long` subtract in `net_1222`
  (compute.rs:143-145): carryforward **expands** the harvestable-gain room (gains are absorbed
  before touching the pref stack) — and creates the absorption plateau of §1.2: `tax=$X` will
  gleefully consume the entire carryforward for $0 current tax. Mandatory disclosure:
  `carryforward_out(N*) vs carryforward_out(0)` ("this harvest burns $Y of carryforward").
  Also note the subtle positive slope where gains shrink a <$3k `loss_deduction` (ordinary
  tax rises at the ordinary rate before any gain is 'taxed').
- **Basis-pending lots.** Cap `N_avail` at the first pending lot in consumption order (HIFO
  sorts them last, pools.rs:271-277; FIFO/LIFO may not) with a disclosure ("N capped: further
  lots have pending basis").
- **Dual-basis gift lots (§1015, fold.rs:149-202).** Three per-lot regimes at fixed price:
  gain zone (slope vs `gain_basis`, tacked HP), loss zone (slope vs `loss_basis`, HP from the
  gift date — the same lot can flip ST/LT character between zones), NGNL (slope exactly 0 —
  a flat segment consuming sats for zero gain). All three must appear in KATs; they are the
  reason T2 is stated as HIFO-shape only.
- **Standing FIFO/LIFO election.** The schedule must follow `applicable_method`, not assume
  HIFO; non-unimodal shapes are the norm here (KAT below).
- **Refusal propagation.** Missing table/profile, any Hard blocker, pre-2025 `at`, future
  date without price ⇒ the engine's/consult's refusal taxonomy verbatim; the harvest tool
  invents no new authority.
- **Statuses enum** (spec): `Found | NotBinding | AlreadyBreached | NoLots |
  ProceedsRequired | PreTransitionYear | YearNotComputable(Blocker)`.

---

## 6. Invariants + KATs the implementation must satisfy

Hard invariants:

1. **Engine-verified answer:** the returned N* always satisfies `predicate(engine(N*))`;
   the tool never emits an N it did not fold and verify. (Never trust the analytic seed.)
2. **Prefix safety:** predicate holds at every lot edge ≤ N* (checked by the walk itself).
3. **Determinism (NFR4):** identical inputs ⇒ identical N* — no float, no RNG, no clock;
   every tie-break inherited from `method_order`/`hifo_cmp`.
4. **Marginal identity:** reported marginal tax == `total(N*) − total(0)` recomputed from two
   independent `compute_tax_year` calls (both exact Decimals).
5. **Non-persistence:** clone-fold-discard only; the sentinel `EventId::Decision{u64::MAX}`
   never persists (same guarantee as consult, optimize.rs:1224-1264).
6. **Selection = standing method** in v1: the answer describes the position the user would
   actually file under their standing order; any optimized identification is a separate,
   clearly-labeled post-hoc suggestion.

KATs (the non-monotone traps first):

- **Dip KAT:** pool = [high-basis loss lot, low-basis gain lot], price between: assert
  marginal(N) < 0 on the first segment, rising after; assert a naive global bisection over
  `tax ≤ X` would land wrong (pin the walk's answer against a hand-computed truth).
- **FIFO non-contiguous KAT:** FIFO election, gains-then-losses-then-gains acquisition order:
  `tax ≤ X` true → false → true across segments; assert the tool returns the FIRST boundary
  (prefix semantics), not the later feasible island.
- **$3k-pin flat KAT:** deep net loss: marginal(N) exactly flat across a loss segment;
  carryforward grows; `tax=$0` answer extends across the plateau with the burn disclosure.
- **Absorption-plateau / carryforward-burn KAT:** profile `cf_long = $50k`, all-gain pool:
  marginal(N) = $0 across absorption; assert the report shows
  `carryforward_out(0) − carryforward_out(N*) == gains absorbed`.
- **ST-feedback KAT (zero-ltcg):** mixed ST/LT pool: each ST-gain dollar shrinks the 0% room
  dollar-for-dollar; pin N* strictly below the LT-only naive answer.
- **QD-stacking KAT:** QD partially fills the 0% zone at baseline; N* shrinks accordingly;
  QD alone over `max_zero` ⇒ `AlreadyBreached`.
- **Cross-net expansion KAT:** ST-loss lots consumed first expand LT 0% room
  (compute.rs:158-165 path) — N* strictly above the no-feedback answer.
- **Dual-basis KATs:** NGNL zero-slope segment traversed correctly; loss-zone leg uses
  `loss_basis` + gift-date HP (character flip changes which stack moves).
- **Two-edge KAT:** one huge lot crossing `max_zero` and `max_fifteen`; `fifteen-ltcg`
  boundary mid-lot; per-segment bisection exact.
- **NIIT kink KAT:** `tax=$X` boundary determined by the MAGI-threshold crossing mid-segment;
  bracket-target reports disclose `niit_applies`.
- **Per-segment monotonicity property KAT:** seeded lot sets; assert tax(N) monotone within
  each segment to within the documented ⌈legs/2⌉-cent band (T1 regression).
- **Boundary exactness KAT:** `predicate(N*)` true and `predicate(N*+τ')` false for the
  documented tolerance; plateau case (near-zero-slope lot) returns the plateau's far edge.
- **Edge-status KATs:** all-loss ⇒ `NotBinding` + $3k disclosure; empty pool ⇒ `NoLots`;
  baseline breach ⇒ `AlreadyBreached`; pending-basis cap; missing profile/table ⇒ refusal;
  MFS $1,500 variant; Qss→Mfj mapping inherited.

Spec-surface deltas required (additive, engine stays the authority):

- Expose the with-scenario `PrefSplit` (`at_0/at_15/at_20`) and/or `bottom_with` on
  `TaxResult` (today only `.tax` survives, compute.rs:342; `MarginalRates.ltcg` is not a
  substitute — vacuous-pref disagreement, compute.rs:380-391).
- A `HarvestRequest { wallet, at, price: Option<Usd /*per BTC*/>, target }` sibling of
  `ConsultRequest` (optimize.rs:107) — proceeds must scale with candidate N
  (`proceeds: None` ⇒ `fmv_of` per candidate; explicit price ⇒ `round_cents(price·N)`),
  unlike consult's fixed total.
- `method_order`/pool plumbing made reachable for the schedule build (or the Phase-1 trick:
  one full-`N_avail` fold and read the legs — zero new ordering code).

— end —
