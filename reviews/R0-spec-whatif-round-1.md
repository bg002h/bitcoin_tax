# R0 — SPEC review, what-if / synthesize tax-planning tool (task #43), round 1

- **Artifact under review:** `design/SPEC_synthesize_whatif.md` @ `ef08724` (branch `feat/whatif`; main == `283238f`).
- **Authoritative inputs cross-checked:** `design/BRAINSTORM_synthesize_whatif.md`;
  `design/agent-reports/fable-harvest-optimizer-advice.md` (the Fable architect's optimizer algorithm).
- **Reviewer:** independent architect (R0). Read-only; no implementation.
- **Grounding:** every claim below verified against current source at review time (file:line).

## VERDICT: **0 Critical / 2 Important / 5 Minor / 5 Nit** — NOT R0-GREEN (fix the two Important, then re-review).

**No Fable re-consult required.** The architect's non-monotone-safe segment-walk algorithm is rendered
**faithfully and completely** (predicates, P0–P3 phases, T1/T2, prefix semantics, engine-verify, τ, disclosures,
status enum — all present and correct; see the faithfulness matrix §A). Neither Important is a hole in the
architect's algorithm — both are **report-signal derivation bugs in the spec's own reporting layer** (the §1212
this-year framing and the NIIT flag), and both are the *exact same error-class* the spec exists to fix (the
consult "whole-year vs baseline-subtracted marginal" bug) leaking back into two secondary disclosures. Both fixes
reuse fields **already surfaced** on `TaxResult` (`loss_deduction`, `niit`) — no engine change, no re-consult.

---

## A. Faithfulness matrix — spec vs architect `fable-harvest-optimizer-advice.md` §2/§5/§6

| Architect element | Spec rendering | Verdict |
|---|---|---|
| P0 as-of pool (`fold_as_of`/`pool_key`); `N_avail` = Σ remaining_sat in consumption order truncated at first basis-pending lot; baseline `total(0)`; `AlreadyBreached` at N=0 | SPEC:48 | ✅ faithful |
| P1 ONE fold of `N_avail`, NO injected selection (standing method), read legs as schedule | SPEC:49-50 | ✅ faithful (proceeds-scaling note dropped — **M1**) |
| P2 `compute_tax_year` at every lot edge (≤ pool+1), first edge true→false; T1 bounds interior | SPEC:51-52 | ✅ faithful |
| P3 analytic seed → sat-bisection decider → **mandatory engine-verify**; τ (e.g. 1,024 sats) | SPEC:53-55, 125 | ✅ faithful |
| Prefix semantics: max N s.t. predicate holds on entire [0,N] | SPEC:56-57, 124 | ✅ faithful |
| `zero-ltcg` ⇒ `at_15+at_20==0`; `fifteen-ltcg` ⇒ `at_20==0`; `gain=$X` ⇒ sale-local `st+lt≤X` (X≥0); `tax=$X` ⇒ `marginal(N)≤X` (X≥0) | SPEC:58-59 | ✅ all four predicates exact |
| Do **not** use `MarginalRates.ltcg` (vacuous-pref disagreement) — expose with-scenario `PrefSplit` | SPEC:15-21 | ✅ faithful |
| Full ST/LT feedback by construction (predicate reads whole stacked position) | SPEC:60 | ✅ faithful |
| Mandatory disclosures: carryforward-burn, NIIT kink, plateau note | SPEC:66 | ✅ present |
| Status enum (7 variants) | SPEC:63-64 | ✅ verbatim |
| Hard invariants 1–6 (engine-verified answer; prefix safety; determinism; marginal identity; non-persistence; standing-method selection) | SPEC:90-130 gotchas + KATs | ✅ present |
| §6 KAT battery (dip, FIFO-non-contiguous, $3k-pin, absorption-burn, ST-feedback, QD-stacking, cross-net, dual-basis, two-edge, NIIT-kink, per-segment-monotone, boundary-exactness, edge-status) | SPEC:96-103 | ✅ all present except **Qss→Mfj mapping (M5)** |

**Conclusion on faithfulness:** no predicate is mis-stated, no phase is dropped, no mandatory disclosure is lost,
and the "never trust the analytic seed / engine-verify" invariant is stated twice (SPEC:54, 125). The spec is a
faithful, complete rendering of the architect's algorithm. The findings below are in the spec's *own additions*
(the sell/consult reporting layer and packaging), not in the optimizer core.

---

## B. Verification of the four load-bearing engine facts (review Qs 2–4)

**Q2 — the engine delta is real and additive.** Confirmed. `compute_tax_year` computes the §1(h) split for both
scenarios but keeps only `.tax`: `preferential_tax(&bp, bottom_with, qd + with.preferential_gain).tax`
(compute.rs:342) / `…without….tax` (compute.rs:343). `PrefSplit{at_0,at_15,at_20,tax}` already exists as a type
(compute.rs:42-48). `MarginalRates.ltcg` is computed from `top = bottom_with + qd + with.preferential_gain`
(compute.rs:380) via `if top <= max_zero {0} else if top <= max_fifteen {.15} else {.20}` (compute.rs:383-389) —
so with **zero** preferential dollars (`qd + preferential_gain == 0`) and `bottom_with > max_zero` it still reports
15/20%, disagreeing with the vacuous `zero-ltcg` case exactly as the spec claims. `bottom_with` is the local at
compute.rs:335. Adding `pref_split`/`bottom_with` changes no computed value (regression KAT well-motivated). *One
packaging note folded into N2.*

**Q3 — the marginal identity is exact and the no-crypto term cancels.** Confirmed by construction. For a fixed
profile, the WITHOUT scenario (`net_1222(0,0,other,cf.short,cf.long,limit)` + `bottom_without`/`pref_without`/
`niit_without`, compute.rs:323-330,337-338,343,355) depends **only** on the profile, never on the disposals in
`state`. Hence in `total = (ord_with+pref_with+niit_with) − (ord_without+pref_without+niit_without)`
(compute.rs:378) the `_without` bracket is byte-identical between the baseline and with-hyp computations, so
`withhyp.total − baseline.total = tax(real+hyp) − tax(real)` exactly (two cent-rounded Decimals subtracted).
The spec's marginal (SPEC:27-29) is therefore exact. **The consult over-report is real:**
`ConsultReport.total_federal_tax_attributable` = `r.total_federal_tax_attributable` (optimize.rs:1203, via
`score_synthetic` optimize.rs:1291) = `tax(real+hyp) − tax(nocrypto)`, which folds the *real* disposals' own
effect into the reported figure on a year with existing disposals. The fix (add ONE baseline `compute_tax_year`
outside the candidate loop, subtract, report the marginal as headline) is **correct and sufficient**: baseline is
selection-independent, so `argmin(total) == argmin(total − baseline)` — the min-tax selection is unchanged
(optimize.rs:1176), only the reported number changes. ✅

**Q4 — §1212 surfacing is pure surfacing.** Confirmed. `TaxResult.carryforward_out` (types.rs:106) =
`{short: with.st_carry, long: with.lt_carry}` (compute.rs:400-403); `TaxProfile.capital_loss_carryforward_in`
(types.rs:49) → `cf` (compute.rs:313) → `net_1222` `cf_short`/`cf_long` (compute.rs:143-145). So
`carryforward_delta = withhyp.carryforward_out − baseline.carryforward_out` is pure surfacing (no new tax logic).
✅ — **but the "$3k this year / $Y carried" *framing* is not pure surfacing and is wrong in a realistic case: I1.**

**Q1 — faithfulness:** covered in §A. No simplification, contradiction, dropped disclosure, or mis-stated
predicate found in the harvest core.

---

## FINDINGS

### [Important I1] The §1212 sell disclosure hard-codes "$3,000 offsets ordinary income this year" — wrong whenever the baseline already consumes the §1211(b) cap

**Where:** SPEC:38-40 (`SellReport` §1212 bullet) + the KAT SPEC:93.

**Evidence.** The §1211(b) ordinary offset is capped by `loss_limit` **for the whole year**, not per-sale:
`loss_deduction = min(net_loss, loss_limit)` (compute.rs:174-178). `TaxResult.loss_deduction` is the WITH-scenario
*level* (compute.rs:399, types.rs:104). The **marginal** this-year offset attributable to the hypothetical is
therefore `withhyp.loss_deduction − baseline.loss_deduction`, which is **not** always $3,000:

- baseline has no capital loss, sale realizes > $3k loss ⇒ delta = $3,000 (the spec's assumed case). ✅
- **baseline already at the cap** (e.g. the user has real in-year ST losses, or a profile carryforward-in already
  driving `loss_deduction = $3,000`), sale adds more loss ⇒ `withhyp.loss_deduction` still $3,000, **delta = $0**:
  the sale offsets **$0** this year and the *entire* new loss carries forward.
- baseline partially uses the cap ($1,000), sale tops it to $3,000 ⇒ only **$2,000** of the sale offsets this year.

The spec lists no `loss_deduction_delta` field in `SellReport` (SPEC:34-41), so the only figure an implementer can
source the "used this year" number from is the literal cap `$3,000` (= `loss_limit`, tables.rs:204) — which is the
trap. For a user who is *already* loss-harvesting and asks "what if I sell one more losing lot," the report would
state "**$3,000 offsets ordinary income this year**" when the truth is "$0 this year, all $Y carried." In a
tax-critical planning tool whose entire §1212 value-prop is "don't under-value a loss sale," this is a materially
misleading **number**, not just prose. The reported `marginal_tax` and `carryforward_delta` are themselves correct;
it is the derived this-year-offset sentence that is wrong. The KAT `whatif_sell_loss_reports_carryforward_delta`
(SPEC:93) exercises only the simple baseline-no-loss case, so the coverage misses exactly the breaking scenario.

**Fix (cheap — no engine change; `loss_deduction` is already surfaced, types.rs:104).** Add
`loss_deduction_delta = withhyp.loss_deduction − baseline.loss_deduction` to `SellReport` (and `HarvestReport`),
and frame the disclosure from it: "**$X** of this loss offsets ordinary income this year (up to the
$3,000/$1,500 annual §1211(b) cap; **$0** here because your existing losses already reach the cap), **$Y** carries
to next year." Add a KAT `sell_loss_when_baseline_already_pinned` (baseline net ST loss $5k ⇒ `loss_deduction`
pinned; hyp adds −$4k ⇒ this-year offset delta == $0, `carryforward_delta` == $4k), and a carryforward-in variant.

---

### [Important I2] The `niit_applies` report signal must be baseline-subtracted (`withhyp.niit − baseline.niit`) — using the raw `MarginalRates.niit_applies` reintroduces the consult whole-position error on years with real disposals

**Where:** SPEC:41 (`SellReport.niit_applies`), SPEC:61,66 (`HarvestReport.niit_applies` + the NIIT-kink
disclosure). Root: the architect's §5 literally cites `MarginalRates.niit_applies` (types.rs:83) as "the
incremental flag to surface," and the spec inherits that citation without pinning the computation.

**Evidence.** `MarginalRates.niit_applies` = `niit_with > niit_without` **within a single `compute_tax_year`**
(compute.rs:390), i.e. "does *all* crypto (real + hyp) raise NIIT vs *no* crypto" (types.rs:76-83). That is a
whole-position, crypto-vs-no-crypto flag — **not** the hypothetical's incremental effect. It diverges from the
correct signal exactly on a year with real disposals (the case the tool and the consult fix exist for):

- baseline (real disposals) already triggers NIIT ⇒ `baseline.marginal_rates.niit_applies == true`. A hypothetical
  **loss-harvest** sale *reduces* NII ⇒ the sale adds **$0** (or negative) NIIT, yet `withhyp.marginal_rates.
  niit_applies` is still `true`. The report would warn "NIIT applies" for a sale that in fact lowers NIIT.

The statutorily meaningful "does *this sale* add 3.8%" signal is the **baseline-subtracted incremental NIIT**,
which is exact and already available: `TaxResult.niit` is itself the crypto-attributable NIIT *delta*
(compute.rs:398, types.rs:102), and `withhyp.niit − baseline.niit = niit(real+hyp) − niit(real)` (the no-crypto
term cancels, identical algebra to `total`). This is the *same* whole-year-vs-marginal error the spec is fixing in
`optimize consult` (SPEC:68-73) — it must not be reintroduced in the NIIT disclosure.

**Fix (cheap — no engine change; `niit` delta already surfaced, types.rs:102).** Define the report field as
`niit_applies := (withhyp.niit − baseline.niit) > 0` (the sale *adds* NIIT), or `!= 0` if you want "changes"; do
**not** read `withhyp.marginal_rates.niit_applies`. State this explicitly in the spec (override the architect's §5
citation for the with-real-disposals case). Make `sell_niit_crossing` (SPEC:104) / `harvest_niit_kink` (SPEC:100)
use a **baseline that already pays NIIT from real disposals**, and assert the flag is `false` for a NIIT-reducing
loss-harvest and `true` only when the hyp's own gain crosses/adds NIIT.

---

### [Minor M1] Proceeds must scale with candidate N (per-BTC `--price` → total proceeds) — the architect's §6 delta is not restated in the spec's sell/harvest bodies

**Where:** SPEC:32,44 (`price: Option<Usd /*per BTC*/>`), SPEC:49 (harvest P1), SPEC:81 (`--price`). Architect §6:
"proceeds must scale with candidate N (`proceeds: None` ⇒ `fmv_of` per candidate; explicit price ⇒
`round_cents(price·N)`), **unlike consult's fixed total**"; §2.2 Phase-1: `proceeds = round_cents(price_per_btc ×
N/1e8)`.

**Evidence.** The reused seam takes a **total** proceeds: `ConsultRequest.proceeds: Option<Usd>` (optimize.rs:111)
/ `CandidateDisposal.proceeds` → `Op::Dispose{ proceeds, … }` (optimize.rs:1254-1260). The harvest folds at many
different N, and even the single sell needs `proceeds = round_cents(price_per_btc × sell_sat / 1e8)`. The spec
annotates the field `/* per BTC */` and says "follow §2 exactly," but neither the sell body nor harvest P1 states
the conversion, and consult's CLI uses `--proceeds` (total) / `--fmv` (bool), not a per-BTC price (cli.rs:305-317)
— so "mirrors the Consult clap shape" (SPEC:80) is loose here. A naive implementer could pass `price` straight
into `proceeds` (off by ~1e8×). Any KAT catches it, but for a "0 open blocking questions" bar it should be
explicit. **Fix:** state in both bodies (and the CLI section) that per-BTC `--price` is converted to total proceeds
per candidate via `round_cents(price × N / 1e8)`, and `--price` absent ⇒ `fmv_of` per candidate.

---

### [Minor M2] The spec cites private fns (`fold_as_of`, `synthetic_state`, `score_synthetic`, `method_order`, `hifo_cmp`) as "reused" — none is reachable from a new sibling `whatif` module; specify expose-vs-reglue

**Where:** SPEC:6,49-50 ("Reuses … `synthetic_state` :1230 / `score_synthetic` :1274"; "reuse `method_order`/
`hifo_cmp`, never re-implement"), SPEC:48 (`fold_as_of`/`pool_key`).

**Evidence.** `fold_as_of` (optimize.rs:1214), `synthetic_state` (optimize.rs:1230), `score_synthetic`
(optimize.rs:1274), `candidate_selections` (optimize.rs:327) are all **private** to `optimize.rs`; `method_order`
(pools.rs:249), `hifo_cmp` (pools.rs:274), `applicable_method` (fold.rs:33) are private to their modules. A new
`btctax-core::whatif` sibling module (lib.rs:3-14) **cannot call** them. This is *achievable without any visibility
change* — `pool_key` (pools.rs:15) and `state_as_of` (fold.rs:505) are `pub`, `evaluate_disposal` is `pub`
(evaluate.rs:98), and the Phase-1 "one fold, read the legs" trick means the standing method is applied *inside*
the fold (no direct `hifo_cmp`/`method_order` call needed). So the whatif module writes its own injector mirroring
`synthetic_state` (using pub `resolve`/`fold`/`Eff`/`Op`/`EventId`) and its own `fold_as_of` (pub
`resolve`+`state_as_of`+`pool_key`). The architect flagged this (§6: "plumbing made reachable … or the Phase-1
trick"). **Fix:** the spec should say which seams are *re-glued from pub primitives* vs *newly exposed
`pub(crate)`*, so an implementer does not try to call a private fn (and does not read "reuse `synthetic_state`" as
"call it"). Related wording issue → N1.

---

### [Minor M3] Adding public fields to `TaxResult` is a breaking change for the published crate — "MINOR" mislabels the SemVer

**Where:** SPEC:107-111 ("MINOR (new read-only command + additive engine field)").

**Evidence.** `TaxResult` is `#[derive(Debug, Clone, PartialEq, Eq)]` with **all-public fields and no
`#[non_exhaustive]`** (types.rs:90-112), and is re-exported from the crate root (lib.rs:40) — public API. Per Cargo
SemVer, adding a field to such a struct is a **breaking** change (it breaks downstream struct-literal construction
and exhaustive field patterns), which under 0.x semantics is a `0.2 → 0.3` bump, not a minor. The crates are
published at v0.2.0 (per project memory). Practical breakage is low (external consumers *read* engine output rather
than construct `TaxResult`), but the classification is wrong as written. Also: `PrefSplit` is `pub` in compute.rs
but **not** re-exported in lib.rs (absent from the lib.rs:39-40 list) — adding `pref_split: PrefSplit` to the
re-exported `TaxResult` requires re-exporting `PrefSplit` too. **Fix:** either bump to 0.3.0 and say so, or add
`#[non_exhaustive]` to `TaxResult` now (itself breaking → same bump) and document it; and add `PrefSplit` to the
lib.rs re-export list.

---

### [Minor M4] Ad-hoc-profile `--magi` default is unspecified — if it silently defaults to $0, every NIIT disclosure is suppressed

**Where:** SPEC:76-82 ("flags `--filing-status`, `--income` (ordinary taxable), `--magi` (+ optional
`--carryforward-in`) build a NON-persisted `TaxProfile`").

**Evidence.** `placeholder_tax_profile` sets `magi_excluding_crypto: Usd::ZERO` (cmd/tax.rs:20). The NIIT closure
fires only when `magi > threshold` (compute.rs:363). If a user supplies `--income`/`--filing-status` but omits
`--magi` and it defaults to $0, MAGI never exceeds the threshold ⇒ `niit_with == niit_without == 0` ⇒ `niit`
delta is always $0 and `niit_applies` always false. That silently guts the very NIIT-crossing / NIIT-kink
disclosures the spec makes load-bearing (SPEC:66,104), producing an *understated* plan with no warning. The spec's
"(+ optional `--carryforward-in`)" implies filing-status/income/magi are the required trio, but it never says
`--magi` is required or how it defaults. **Fix:** state that `--magi` is required when using the ad-hoc trio (or
defaults to `--income`, never $0), and add a refusal/warning if MAGI is left unset while NIIT could apply.

---

### [Minor M5] Dropped KAT — the architect's Qss→Mfj filing-status mapping edge-status KAT is absent

**Where:** SPEC:102-103 (status/refusal KATs). Architect §6 edge-status list: "MFS $1,500 variant; **Qss→Mfj
mapping inherited**."

**Evidence.** `FilingStatus::Qss` uses the MFJ schedule/thresholds for all §1(h)/§1/§1411 lookups (types.rs:4-15).
The spec lists the MFS $1,500 variant and the FIFO/LIFO election KAT but drops the Qss→Mfj coverage the architect
called out. Low impact (a filing-status routing check), but it is a named §6 trap the spec's list omits. **Fix:**
add `harvest_qss_maps_to_mfj_breakpoints` (a Qss profile yields the same `max_zero`/`max_fifteen`/NIIT-threshold
answer as the MFJ profile).

---

### Nits

- **[N1]** SPEC:50 "reuse `method_order`/`hifo_cmp`, never re-implement" reads as *calling* those private fns; the
  actual mechanism is the Phase-1 fold applying `applicable_method` internally (fold.rs:33) and the schedule being
  *read from the legs*. Reword to "the fold consumes by the standing method — read the resulting legs; do not
  re-derive the ordering."
- **[N2]** The added `pref_split` is the **WITH-scenario** split, whose `.tax` is a *level* (`pref_with`,
  compute.rs:342), while `TaxResult.ltcg_tax` is the *delta* `pref_with − pref_without` (compute.rs:397). The two
  will coexist on `TaxResult`; the spec should note `pref_split` is the with-scenario split (its `.tax` is not the
  attributable delta) so downstream code reads `at_15`/`at_20` for the predicates and `ltcg_tax` for the number.
- **[N3]** The bracket predicates (`at_15`/`at_20`) are computed from **unrounded** Decimals (compute.rs:67-87;
  architect §4.2) — they are *exact* and need no τ; only `gain=$X`/`tax=$X` are cent-quantized and need τ. The
  spec's single τ note (SPEC:54-55) could distinguish "exact bracket targets vs τ-bounded dollar targets."
- **[N4]** The architect's optional post-hoc `consult_sale(N*)` "better identification" suggestion (§2.1, §6
  invariant 6) is not mentioned. Fine to defer, but a one-line "v2/optional, outside the loop" note would close it.
- **[N5]** CLI arg unit is unstated: `optimize consult --sell` is **satoshis** (cli.rs:295-297) while the brainstorm
  wrote `what-if sell <N-btc>` (BRAINSTORM:13) and `SellRequest` uses `sell_sat` (SPEC:32). Pin the CLI arg unit
  (BTC decimal vs sats) and the sat conversion explicitly.

---

## C. Re-consult determination

**Not needed.** The architect's algorithm is sound and the spec renders it faithfully and completely (§A). The two
Important findings are in the spec's own reporting layer (§1212 framing, NIIT flag), are the same
baseline-subtraction discipline the spec already applies to `marginal_tax`, and are fixable with fields already
surfaced on `TaxResult` (`loss_deduction` types.rs:104, `niit` types.rs:102) — zero engine or algorithm change.

## D. Gate

**BLOCKED at R0** on I1 + I2 (2 Important). Fold both (plus the Minors/Nits as the author sees fit), then re-run
R0 round 2. When 0 Critical / 0 Important remain, this artifact is R0-GREEN and cleared to plan/implement.
