# Whole-branch review — NII interest slice — round 1

**Artifact:** diff `43c653e..6c70aa0` (2 commits: `f3c22bf` spec/R0, `6c70aa0` Task 1)
**Reviewer role:** independent FINAL whole-diff reviewer (author ≠ reviewer)
**Gate class:** ENGINE-B §1411 NIIT behavior change. Critical bar = wrong scenario attribution,
MAGI double-count, a moved/edited regression golden, or a non-reproducing golden.
**Verdict:** **0 Critical / 0 Important → READY TO MERGE.** 0 Minor + 2 Nit (non-blocking).

I re-derived every number by hand from the current source (`compute.rs`, `tables.rs`, the `synth`
table in `tax_compute.rs`) — I did **not** trust the report. Everything reconciles.

---

## 1. The insertion — exactly one behavioral line in the NII math (verified from source)

`crates/btctax-core/src/tax/compute.rs`:
- **Import (13):** `use crate::event::{IncomeKind, LedgerEvent};` — `IncomeKind` added; scoped correctly
  (mirrors `se.rs`). No wildcard.
- **`interest_nii` sum (302–311):** `Σ i.usd_fmv` over `income_recognized` filtered by
  `recognized_at.year() == year && i.kind == IncomeKind::Interest`. Kind-only filter, business-agnostic
  (per D-legal). Mirrors `crypto_ord` (296–301) exactly except the `kind` predicate. `usd_fmv: Usd` ⇒
  `interest_nii ≥ 0`.
- **`nii_with` (352–353):** `qd + with.ordinary_gain + with.preferential_gain - with.loss_deduction
  + interest_nii` — the `+ interest_nii` is the **only** behavioral change.
- **UNTOUCHED (byte-verified against the pre-change formula):** `nii_without` (354–355),
  `crypto_agi` (357–359), `magi_without/magi_with` (360–361), the `niit` closure (362–373),
  `niit_with/niit_without` (374–375), the `total` delta (378), `bottom_with/bottom_without` (335–338),
  `crypto_ord` (296–301). The module-doc and inline comments changed (non-behavioral).

**Conclusion:** exactly one behavioral line + the sum binding + the import. No second behavioral change.
✔ Matches point 1's requirement precisely.

## 2. Goldens re-derived by hand (synth table — NOT BundledTaxTables)

Both new goldens run on `synth(2025)` (Single ordinary 0→10% / 50k→22% / 250k→32%; NIIT statutory
`niit_threshold(Single) == dec!(200000)` [tables.rs:285], `NIIT_RATE == dec!(0.038)` [tables.rs:133]).
The `4,400` bracket math is re-derived below **for that synthetic table**, which is what the test uses.

### (a) Headline `interest_nii_headline_interest_plus_min_cap`
Profile `(ord 150,000, magi_excl 195,000, qd 0)`, no disposals, one Interest $20,000.
- `interest_nii` = 20,000; `nii_with` = 0+0+0−0+20,000 = **20,000**; `nii_without` = 0.
- `crypto_agi` = 0 + `crypto_ord` 20,000 = 20,000; `magi_with` = 195,000+20,000 = **215,000**.
- `niit_with`: over = 215,000−200,000 = 15,000; `if nii < over` → 20,000<15,000 false → capped = **over
  15,000** (min-cap binds on `over`); 3.8% × 15,000 = **$570.00**.
- `niit_without`: 195,000 > 200,000 is **false** → over 0 → **$0.00**.
- **`r.niit` = 570.00 − 0 = 570.00.** ✔ (assert `dec!(570.00)`)

**Absolute total $4,970.00 — bracket math on the synthetic Single schedule:**
- `bottom_with` = 150,000 + `crypto_ord` 20,000 = **170,000**; `bottom_without` = **150,000**.
- `ordinary_tax_on(170,000)` = 10%×50,000 + 22%×(170,000−50,000) = 5,000 + 22%×120,000 = 5,000 + 26,400
  = **31,400.00**.
- `ordinary_tax_on(150,000)` = 10%×50,000 + 22%×(150,000−50,000) = 5,000 + 22×1,000×… = 5,000 + 22,000
  = **27,000.00**.
- **ord_delta = 31,400 − 27,000 = $4,400.00** (the incremental $20k sits wholly in the 22% band —
  both 150k and 170k are below the 250k→32% breakpoint). Correct for the synth table.
- `pref_with = pref_without = 0` (qd 0, no pref gain) ⇒ `ltcg_tax` = 0.
- **`total` = (31,400 + 0 + 570) − (27,000 + 0 + 0) = 31,970 − 27,000 = $4,970.00.** ✔
  (assert `total_federal_tax_attributable == dec!(4970.00)`; and the identity assert `== dec!(4400.00)
  + ltcg_tax + niit` jointly pins ord_delta = 4,400 since total=4,970 and niit=570 force ltcg_tax=0.)

### (b) Mixed `interest_nii_mixed_mining_plus_interest_exclusion_boundary`
Profile `(ord 50,000, magi_excl 200,000, qd 0)`, Mining $30,000 + Interest $10,000.
- `crypto_ord` = 40,000; **`interest_nii` = 10,000 (Interest only — Mining excluded)**.
- `nii_with` = 10,000; `crypto_agi` = 40,000; `magi_with` = **240,000**; over = 40,000.
- `niit_with`: `if nii < over` → 10,000<40,000 true → capped = **nii 10,000**; 3.8% × 10,000 = **$380.00**.
- `niit_without`: `magi_without` = 200,000; `200,000 > 200,000` **false** → over 0 → **$0.00**
  (the exactly-at-threshold `>` boundary is load-bearing and correct).
- **`r.niit` = 380.00 − 0 = 380.00.** ✔ (assert `dec!(380.00)`)
- **Boundary lock:** Mining wrongly in NII ⇒ nii_with 40,000 ⇒ min(40,000,40,000) ⇒ 3.8%×40,000 =
  **$1,520.00** ⇒ golden fails. Genuine exclusion lock.

The two goldens exercise **both** branches of `min(nii, over)` (over-bound in (a), nii-bound in (b)).

**TDD red evidence credible:** pre-change `nii_with` has no interest term ⇒ `nii_with` = 0 ⇒ `niit_with`
= 0 ⇒ `r.niit` = 0 for both. The report's red output (`left: 0 right: 570.00` / `... 380.00`) is
exactly what this code produces pre-`+ interest_nii`. Real red→green, not retrofit. ✔

## 3. Regression net — byte-identical, none edited to fit

The `tax_compute.rs` diff is **purely additive** (two hunks: the `income_rec_interest` helper after
`income_rec`, and the two new tests after `determinism_…`). No existing golden hunk touched.
Confirmed present and unchanged in current source:

| Golden | Line | niit | Income kind |
|---|---|---|---|
| `niit_threshold_crossing` | 357 | `dec!(760.00)` | none (`vec![]`) |
| `full_worked_example_…` | 395 | `dec!(2280.00)` | `income_rec` → **Mining** (167) |
| `niit_loss_year_reduces_nii_by_1211_allowed_loss` | 445 | `dec!(-684.00)` | none |
| `niit_loss_year_mfs_1500_limit` | 517 | `dec!(-57.00)` | none |
| `double_count_guard_…` | 212 | 0.00 | Mining (220) |
| `niit_base_floored_at_zero_when_nii_negative` | 463 | 0.00 | none |

`income_rec` hard-codes `IncomeKind::Mining` (167); the only `IncomeKind::Interest` producer is the new
`income_rec_interest` (177), used solely by the two new tests. Every regression fixture has
`interest_nii == 0` ⇒ NII unchanged. **No expected value moved.** ✔ (point 3 satisfied — no Critical.)

## 4. No MAGI double-count

`crypto_agi` (357–359) is byte-unchanged and already adds `crypto_ord` (the kind-agnostic sum that
already contains interest). `interest_nii` enters **only** `nii_with` — no new MAGI term. Interest hits
MAGI once (via crypto_ord→crypto_agi, pre-existing) and NII once (via interest_nii). Interest appearing
in both the ordinary stack (`bottom_with`) and NII is two distinct taxes (Ch.1 income tax + Ch.2A NIIT),
not a double-count. ✔

## 5. Disclosure — all three sites gone, KAT semantic

`grep` over `crates/` finds **no** surviving "cannot yet isolate" / "residual understatement" /
"Phase-2 refinement" in any source or rendered string — the only hits are the KAT lines asserting
their **absence**. The three sites:
- **render.rs footer (1026–1027):** now "…is correctly excluded from NII; crypto-lending interest
  income (§1411(c)(1)(A)(i)) is INCLUDED in NII; mining/staking/airdrops/rewards remain excluded (SE
  income per §1411(c)(6) or non-NII other income)." Legally accurate.
- **compute.rs module-doc:** now "…**plus** crypto-lending **interest** income (§1411(c)(1)(A)(i), NII)."
- **compute.rs inline (349–351):** now "Crypto-lending INTEREST (§1411(c)(1)(A)(i)) is INCLUDED in NII
  (`interest_nii`, WITH-scenario only). Mining/staking/airdrops/rewards remain EXCLUDED…"

**KAT (tax_report.rs:208–258):** renamed `…_lending_interest_caveat` →
`…_interest_nii_included`. Asserts `contains("is INCLUDED in NII")` **and**
`!contains("cannot yet isolate")` (both **fail against the OLD footer** — old text has neither the new
phrase nor lacks the disclaimer ⇒ genuinely distinguishes old→new). Retains the wrong-direction
negatives (`can only ever understate`, `MAY UNDERSTATE`, `does not reduce NII`) plus the §1211 positive
(`reduces NII by the §1211(b)-allowed net capital loss`). The rendered footer contains every asserted
substring (verified against render.rs 1020–1028 after `\`-continuation whitespace collapse). ✔

## 6. Exactness / determinism / scope

- **Exact Decimal, no float:** `interest_nii: Usd`, `NIIT_RATE = dec!(0.038)`, `round_cents` at the end
  of the closure only. No `f64` anywhere on the path. ✔
- **Determinism:** the `sum()` over a deterministically-ordered `Vec` is order-stable; the determinism
  KAT is unchanged and passes. ✔
- **`se.rs` untouched:** not in the changed-files set (render.rs, tax_report.rs, compute.rs,
  tax_compute.rs, spec, R0). Interest stays SE-excluded (`se.rs:58`), consistent with §1411(c)(6). ✔
- **§1411(c)(2) exception:** disclosed out of scope in the spec ("Out of scope" + "disclosed as
  inapplicable"). Kind-only rule is conservative (would only ever *overstate*, never understate, in the
  atypical active-lending-T-or-B edge). ✔

---

## Findings ledger

**Critical (0):** none.
**Important (0):** none.

**Nit:**
- **NIT-1 — footer names the excluded categories twice.** render.rs 1025–1027 says
  "crypto ordinary income (mining/staking/airdrops/rewards) is correctly excluded from NII" **and**
  "mining/staking/airdrops/rewards remain excluded (SE income per §1411(c)(6)…)". Slight redundancy;
  cosmetic. Optional to collapse.
- **NIT-2 — R0-M2 optional code comment not added.** R0 suggested a one-line comment beside
  `interest_nii` flagging the §1411(c)(2) active-lending-T-or-B overstatement edge. The spec discloses
  it out of scope, so the omission is non-blocking; the inline comment (302–305) already explains the
  inclusion/exclusion rule.

---

## Gate decision

**0 Critical / 0 Important → GREEN. Ready to merge.** The insertion is a single behavioral line into
`nii_with` only; `nii_without`/`crypto_agi`/MAGI/the `niit` closure/the delta/the bottom stacks are
byte-identical. Both new goldens re-derive **exactly** (570.00 / 4,970.00 with ord_delta 4,400.00 on
the synth Single schedule; 380.00) and exercise both `min(nii, over)` branches; the TDD red is genuine.
The five prior NIIT goldens + the double-count guard + the floor golden are untouched (additive diff,
no Interest in any fixture). No MAGI double-count. All three disclosure sites corrected and the KAT is
semantic (fails on old text). Exact Decimal, deterministic, `se.rs` untouched, §1411(c)(2) disclosed
out of scope. The 2 Nits are optional polish.
