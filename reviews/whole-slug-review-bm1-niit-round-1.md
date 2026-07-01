# Whole-slug review — P2 B-M1 §1411 NIIT net-capital-loss fix (round 1)

**Reviewer role:** comprehensive (task-review + whole-diff gate).
**Commit under review:** `d2efadd` (`bb8ac14..d2efadd`), single commit.
**Class:** tax-figure change in engine B — wrong rule/direction, gain-year regression, or a
non-reproducing golden = Critical.
**Method:** static analysis + independent hand re-derivation of every asserted golden against current
source (`compute.rs`, `tables.rs`, `render.rs`, the KAT tables). The full validation gate (491 tests,
clippy `-D warnings`, fmt, release, PII) was reported GREEN by the report; per instruction I did **not**
re-run it — I verified the math, direction, and no-regression.

---

## Verdict: ✅ READY TO MERGE — 0 Critical / 0 Important

The fix implements §1.1411-4(d) / Form 8960 line 5a correctly and in the verified direction; all three new
goldens re-derive by hand to the asserted cents; no existing golden's expected value is touched by the
diff; scope is contained to NII + the NIIT floor + disclosure.

Findings: **2 Minor, 1 Nit** (all non-blocking).

---

## 1. The fix — re-derivation (highest priority)

Code confirmed at `crates/btctax-core/src/tax/compute.rs`:
- L344–346: `nii_with = qd + with.ordinary_gain + with.preferential_gain − with.loss_deduction`;
  `nii_without` symmetric. **Matches D1.**
- L353–364: `niit` closure — `over = max(0, magi−thr)`, `capped = min(nii, over)`,
  `base = max(0, capped)`, `round_cents(base * NIIT_RATE)`. **Matches D2.** `NIIT_RATE = dec!(0.038)`
  (`tables.rs:113`), `round_cents` = ties-to-even (`conventions.rs:22`).

### 1a. Headline — `niit_loss_year_reduces_nii_by_1211_allowed_loss` → asserts `r.niit == -684.00`

Inputs: Single (thr $200,000, §1211 cap $3,000). OTI 270,000; QD 5,000; `other_net_capital_gain` +15,000
(LT); `magi_excluding_crypto` 290,000; crypto net ST −80,000; crypto_lt 0; crypto_ord 0; zero carryforward.

**§1222 WITH** `net_1222(−80000, 0, +15000, 0, 0, 3000)`:
st_net −80,000; lt_net +15,000 → (ST-loss, LT-gain) cross-net, `−st(80k) ≤ lt(15k)?` no →
`(st+lt, 0) = (−65,000, 0)` → ordinary_gain 0, preferential_gain 0, net_loss 65,000,
`loss_deduction = min(65k, 3k) = 3,000`. (Test also asserts `st_net −80000`, `loss_deduction 3000` ✓.)

**§1222 WITHOUT** `net_1222(0,0,+15000,0,0,3000)`: both gains → preferential_gain 15,000, loss_deduction 0.

**NII:** nii_with = 5,000 + 0 + 0 − 3,000 = **2,000**; nii_without = 5,000 + 0 + 15,000 − 0 = **20,000**.

**MAGI:** crypto_agi = (0−3,000) − (15,000) + 0 = −18,000; magi_with = 290,000 − 18,000 = **272,000**.

**NIIT:**
- niit_with = 3.8% × max(0, min(2,000, 272,000−200,000=72,000)) = 3.8% × 2,000 = **76.00**
- niit_without = 3.8% × max(0, min(20,000, 290,000−200,000=90,000)) = 3.8% × 20,000 = **760.00**
- **DELTA niit = 76.00 − 760.00 = −684.00** ✓ (asserts `dec!(-684.00)`)

**Pre-fix check:** without the loss subtraction nii_with = 5,000 → niit_with = 3.8%×5,000 = 190.00 →
delta = 190 − 760 = **−570.00** — matches the reported pre-fix value. Fix moves it to −684.00 ✓.

**total:** ordinary_delta = tax(267,000) 54,440 − tax(270,000) 55,400 = −960.00 (= 3,000×32%);
ltcg_tax = pref(267k;5k) 750.00 − pref(270k;20k) 3,000.00 = −2,250.00; niit −684.00 →
total = **−3,894.00** ✓ (asserts `dec!(-3894.00)`; identity `total = ord_delta + ltcg + niit` holds).

### 1b. NII-negative floor — `niit_base_floored_at_zero_when_nii_negative` → `r.niit == 0.00`

Single; QD 0; ncg 0; `magi_excluding_crypto` 200,000 (exactly at threshold → over_without 0); crypto ST
−80,000 → loss_deduction 3,000. nii_with = 0+0+0−3,000 = **−3,000**; crypto_agi −3,000 →
magi_with 197,000 (< 200,000 → over_with 0).
- niit_with = 3.8% × max(0, min(−3,000, 0)) = 3.8% × **0** = 0.00
- niit_without = 3.8% × max(0, min(0,0)) = 0.00 → DELTA **0.00** ✓

**Floor genuinely exercised:** without D2, `min(−3,000, 0) = −3,000` → niit_with = round(−3,000×0.038) =
**−114.00** → delta −114.00. The 0.00 assertion pins `max(0,…)`. ✓

### 1c. MFS $1,500 — `niit_loss_year_mfs_1500_limit` → `r.niit == -57.00`

MFS (thr $125,000, cap $1,500). OTI 50,000; QD 5,000; ncg 0; magi_excl 300,000; crypto ST −80,000 →
`loss_deduction = min(80k, 1,500) = 1,500` (asserts `dec!(1500)` ✓ — `loss_limit(Mfs)=1500`, `tables.rs:146`).
nii_with = 5,000 − 1,500 = 3,500; nii_without = 5,000. magi_with = 298,500.
- niit_with = 3.8% × min(3,500, 173,500) = **133.00**; niit_without = 3.8% × min(5,000, 175,000) = **190.00**
- DELTA = **−57.00** ✓ (= 3.8% × −1,500). A wrong $3,000 cap → loss_deduction 3,000 → nii_with 2,000 →
  niit_with 76.00 → delta −114.00; the −57.00 assertion pins the MFS half-cap. ✓

## 2. Direction — CORRECT

Post-fix nii_with is lower by the §1211-allowed loss (5,000→2,000), so the WITH-scenario NIIT drops
190→76. The shipped "can only ever understate" claim was directionally **wrong** (pre-fix niit_with 190 >
correct 76 → it OVERSTATED in the loss year). The fix reproduces §1.1411-4(d)(2)/(3)(ii) **Example 1**
exactly: surviving net gain floored at $0 (ordinary_gain/preferential_gain clamped ≥0 in `net_1222`) **and**
NII additionally reduced by the $3,000 §1211(b) loss → capital contribution 0 − 3,000 = −3,000; +QD 5,000 =
NII 2,000. This is Form 8960 line 5a (= 1040 line 7a Schedule-D §1211-limited net) flowing in as a
negative. Rule and sign both correct.

## 3. No gain-year regression — CONFIRMED

- In gain years `loss_deduction == 0` in both scenarios → D1 subtracts 0 (no-op); nii > 0 → D2 floor no-op.
- The diff makes **only additions** to the two test files (every changed test line is a `+`; no existing
  expected value is deleted or edited). Verified line-by-line in the diff.
- Gain-year goldens re-checked as no-ops and unchanged: `niit_threshold_crossing` **760.00** and
  `full_worked_example_…` **2280.00** (`tax_compute.rs`, both crypto-gain, loss_deduction 0);
  `kat_rate_engine.rs` **3800.00, 2280.00, 3800.00** and the three-way-nonzero **570.00** (a Single LT-gain
  +mining case, loss_deduction 0 — coincidental magnitude). All D1/D2 no-ops.
- Extra evidence the combined fix is internally consistent: the **existing** loss-year test
  `kat_rate_engine::mfs_1500_loss_cap_and_carryforward` (asserts total **−330.00**) has `with.loss_deduction
  = 1,500` and magi_with 58,500 < $125k. D1 alone would push nii_with to −1,500 → niit_with −57.00 → total
  −387.00 (would BREAK the golden); D2 floors niit_with back to 0 → total stays −330.00. Its continued green
  is direct proof D2 is applied and D1+D2 preserve pre-existing loss-year goldens.

## 4. Scope — CONTAINED

`crypto_ord` still EXCLUDED from NII (nii = qd + gains − loss_deduction; crypto_ord appears only in
`bottom_with` L327 and `crypto_agi` L350). `crypto_agi`/MAGI formula unchanged (diff context lines,
no `+/−`). `net_1222`, §1211 loss-limit, §1212 carryforward, ordinary stack, §1(h) preferential tax all
untouched. Only nii, the NIIT base floor, and the disclosure changed. 4 files / 226 insertions,
12 deletions — matches stat.

## 5. Disclosure — CORRECTED

`render.rs` L820–828 footer (Computed branch only): removed "MAY UNDERSTATE" and "does not reduce NII by
the allowed §1211 loss"; now states NII is reduced by the §1211(b)-allowed net capital loss (≤$3,000/$1,500
MFS, Form 8960 line 5a / §1.1411-4(d)), floored at $0, crypto ordinary income correctly excluded, residual
caveat = crypto-lending interest §1411(c)(1)(A)(i). `compute.rs` doc (L214–220) + inline comment (L336–343)
reworded identically; "can only ever understate" removed. KAT
`report_tax_year_footer_discloses_1211_loss_and_lending_interest_caveat` pins the new wording
(negative + positive assertions). ✓

## 6. NFR4 / NFR5

Determinism preserved — scalar Decimal ops only, no collection/ordering change. No float: floor is a
Decimal comparison (`capped > Usd::ZERO`), `NIIT_RATE = dec!(0.038)`, `round_cents` = Decimal ties-to-even;
all four goldens' products (76, 760, 133, 190, 0) are exact to the cent so rounding mode is not load-bearing.

---

## Findings

**Minor 1 (coverage).** The now-reachable delta path where the *no-crypto* scenario itself carries a §1211
loss (`without.loss_deduction > 0`) **and** `magi_without > threshold` is not pinned by any asserting
golden. `kat_rate_engine::single_3k_loss_limit…` r_y2 does exercise `without.loss_deduction = 3,000`, but
its MAGI (60,000) is sub-threshold so niit floors to 0 and niit isn't asserted; no fixture uses a negative
`other_net_capital_gain`. The fix is symmetric and correct there (nii_without now accurately drops by the
allowed loss), so this is a coverage gap, not a defect. Non-blocking; candidate for FOLLOWUPS.

**Minor 2 (observation, positive — no action).** `mfs_1500_loss_cap_and_carryforward` (−330.00) already
acts as a regression guard for the D1+D2 interaction (see §3). Worth keeping; consider a comment noting it
now also guards the NIIT floor.

**Nit 1.** The footer KAT's `assert!(!rendered.contains("can only ever understate"))` is vacuous for the
rendered output — that phrase only ever lived in a `compute.rs` code comment, never in the footer (the
footer said "MAY UNDERSTATE"). Harmless/defensive; the `!"MAY UNDERSTATE"` and `!"does not reduce NII"`
assertions are the meaningful ones.

## Unverifiable
None load-bearing. Per instruction the GREEN gate (491 tests / clippy / fmt / release / PII) was not
re-run; all math, direction, and no-regression claims were verified independently above.
