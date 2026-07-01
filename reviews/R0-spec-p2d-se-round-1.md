# R0 architect review — SPEC P2-D (self-employment tax) — Round 1

- **Artifact:** `design/SPEC_p2d_se_tax.md`
- **Baseline verified:** `origin/main` HEAD = `52cdd53` (matches spec baseline). All recon citations
  checked against current source at write time.
- **Reviewer role:** independent architect gate (R0). Numbers independently web-verified — NOT taken
  from the spec.
- **Verdict:** **NOT GREEN.** 1 Critical, 1 Important, 3 Minor, 3 Nit. Blocking findings below must
  reach 0C/0I before implementation.

---

## Web verification (independent — did not trust the spec)

| Claim | Verified value | Source |
|---|---|---|
| §1401(a) Social Security rate | **12.4%** | IRS "Self-employment tax" |
| §1401(b)(1) Medicare rate | **2.9%** (uncapped) | IRS "Self-employment tax" |
| §1401(b)(2) Additional Medicare | **0.9%** | IRS Topic 560 / Form 8959 |
| §1402(a) net-SE-earnings factor | **92.35%** (= 1 − 7.65%) | IRS / Schedule SE line 4a |
| Combined base SE rate | 15.3% | IRS |
| Addl-Medicare thresholds | **$200k** Single/HoH, **$250k** MFJ, **$125k** MFS | IRS Q&A / Form 8959 |
| SS wage base **TY2025** | **$176,100** (was $168,600 in 2024 → year-indexed) | SSA 2024-10-10 announcement |
| Notice 2014-21 **A-9** | business (non-employee) mining = SE income; **hobby mining is NOT SE** | IRS N-2014-21 |

All spec rates/factor/wage-base/threshold values are **correct**. The SS wage base is confirmed
year-indexed (moved $168,600 → $176,100) → correctly belongs in the per-year `TaxTable`, not a
`tables.rs` const. Good.

### Goldens re-derived by hand (all reproduce from the verified rules)

- **Golden 1 — Single, business mining $100,000, no W-2:**
  base = 100,000 × 0.9235 = **92,350.00**; ss = 12.4% × min(92,350, 176,100) = **11,451.40**;
  medicare = 2.9% × 92,350 = **2,678.15**; addl = 0.9% × max(0, 92,350 − 200,000) = **0.00**;
  total = **14,129.55** ✓; deductible_half = round_half_even(14,129.55 / 2 = 7,064.775) = **7,064.78** ✓
  (correct here **only because addl = 0** — see C1).
- **Wage-base cap — mining $250,000:** base = 230,875.00 > 176,100 → ss = 12.4% × 176,100 =
  **21,836.40** ✓.
- **Additional-Medicare — mining $300,000 Single:** base = 277,050.00; addl = 0.9% × (277,050 −
  200,000) = **693.45** ✓. (Also: medicare = 2.9% × 277,050 = 8,034.45; total = 21,836.40 + 8,034.45
  + 693.45 = 30,564.30. **Correct** deductible_half = (21,836.40 + 8,034.45)/2 = **14,935.42** — the
  spec formula `total/2` gives 15,282.15, **wrong by $346.73**; see C1.)

Addl formula matches Form 8959 Part II with w2=0 (line 11 = threshold; line 12 = base − threshold;
line 13 = 0.9% × line 12). Medicare/Addl split correctly yields 2.9% below threshold, 3.8% above.

---

## Recon-citation verification (drift flagged)

| Spec citation | Reality @ 52cdd53 | Status |
|---|---|---|
| `IncomeRecord` `state.rs:178-219` | struct at **state.rs:177–185**; `.income_recognized` field 219 | OK (range overshoots) |
| `IncomeKind{Mining,Staking,Interest,Airdrop,Reward}` "in state.rs" | actually **`event.rs:29-35`**; 5 variants exact | Nit N1 |
| `compute.rs:297-302` crypto_ord | **exact match** (Σ usd_fmv, year-filtered, undifferentiated by business) | OK |
| total identity `compute.rs:368-372` | `total = (ord_with+pref_with+niit_with) − (…without)` at **369**; field set 395 | OK |
| pinned identity `total == ord_delta + ltcg_tax + niit` | **real** — `kat_rate_engine.rs:551 three_way_nonzero_identity` + `tax_compute.rs:305` | OK |
| `tables.rs` NIIT_RATE / niit_threshold / gift_annual_exclusion precedent | **exact** (const 119; fn 136; field 67) | OK |
| `adapters/tax_tables.rs` TY2025 + gift_annual_exclusion pattern | **exact** (field 185; year-indexed pattern to mirror) | OK |
| `TaxProfile` `types.rs:31-50` has no W-2 field | **confirmed** (no w2 field) | OK |
| River `business:false` immutable `river.rs:148-179` | actual path **`crates/btctax-adapters/src/sources/river.rs:149-180`**; `income`→`Reward` business:false (159-160) **and** `interest`→`Interest` business:false (177-178), both documented immutable | Nit N1 (path) + see M2 |
| `render.rs` write_csv_exports / 0o600 / `if let Some(year)` | **exact** (open_owner_only=0o600 at 556; block at 708) | OK |
| round_cents rounding mode | **`MidpointNearestEven`** (conventions.rs:13) → half-even confirmed | OK |

---

## Findings

### C1 (Critical) — `deductible_half` overstates the §164(f) deduction (includes the Additional Medicare Tax)

**Where:** D2 / Plan Task 1 — `total = ss + medicare + addl`; `deductible_half = round_cents(total / 2)`.

**Problem:** §164(f)(1) allows a deduction equal to one-half of the taxes imposed by §1401
**"(other than the taxes imposed by section 1401(b)(2))"** — i.e., the **§1401(b)(2) Additional
Medicare Tax (the 0.9%) is expressly EXCLUDED** from the ½-SE deduction. Schedule SE line 13 (= line
12 × 50%) is computed on line 12 = SS + regular Medicare only; the 0.9% lives on Form 8959 and is
**never** deductible. Because the spec's `deductible_half` divides a `total` that *includes* `addl`,
it overstates the above-the-line deduction by `0.5 × addl` for **every** taxpayer whose base exceeds
the addl-Medicare threshold. This is a wrong-dollar §1401 figure (per the Critical rubric).

- Silent in all three current goldens only because Golden 1 has addl = 0; the $250k/$300k goldens
  don't assert a `deductible_half`, so the bug is untested. For $300k Single it overstates the
  deduction by **$346.73** (15,282.15 vs correct 14,935.42) → understates the taxpayer's income tax.

**Web-confirmed** against §164(f)(1) statutory text (Cornell LII / Bradford / Tax Notes): the
"other than section 1401(b)(2)" carve-out is explicit.

**Fix (required):**
- `deductible_half = round_cents((ss + medicare) / 2)` — exclude `addl`.
- Add an **addl > 0 golden** that pins `deductible_half` (e.g. $300k Single → medicare = 8,034.45,
  addl = 693.45, total = 30,564.30, **deductible_half = 14,935.42**) so this can never regress.
- Keeping `total = ss + medicare + addl` as the displayed "total §1401 liability" is defensible
  (§1401(b)(2) *is* in §1401), but see M4/labeling: the render/CSV should make clear the 0.9% is a
  Form 8959 item and is the non-deductible part.

---

### I1 (Important) — D4 W-2 disclosure covers the SS cap but NOT the Additional-Medicare threshold reduction

**Where:** D4 / D3 render disclosure.

**Problem:** The `w2_ss_wages = 0` default drives **two** W-2 interactions, but the disclosure text
addresses only one. §1401(b)(2)(B) / Form 8959 Part II reduces the addl-Medicare **threshold** by the
taxpayer's Medicare wages (not below zero). The tool assumes $0, so for an **employed** miner it
**understates** the 0.9% component (a lower threshold → more SE income taxed). The D4 text only warns
about the 12.4% SS cap being reduced by W-2 SS wages; it is silent on the threshold shift. Since the
entire justification for the $0 default is "compute standalone but disclose the caveat," a disclosure
that covers half the W-2 interaction leaves the employed-miner path silently wrong.

**Fix:** Extend the `render_schedule_se` disclosure to also state: "assumes $0 W-2 Medicare wages; if
you had a wage job, your $200k/$250k/$125k Additional-Medicare threshold is **reduced** (not below
$0) by your W-2 Medicare wages (Form 8959, Part II) — the 0.9% shown here is understated
accordingly." Deferring the `TaxProfile.w2_ss_wages` (really w2 SS *and* Medicare wages) field to a
FOLLOWUP is fine, but the disclosure must name both interactions. (Borderline I/Minor: text-only fix,
default solo-miner figure is correct — but prompt-flagged and it's a real employed-miner mislead.)

---

### M1 (Minor) — additive `ss_wage_base` field under-scopes its blast radius

Task 1 says "add the field + update `synthetic_table` + `BundledTaxTables::ty2025()`" (2 sites).
`TaxTable` is built by **struct literal** in ~12 more places; adding a non-`Default` field breaks
**every** one → the workspace won't compile and "green" is unreachable until all are touched:
`crates/btctax-cli/src/render.rs:1433` (test helper) plus tests `optimize_score.rs:67`,
`optimize_wash_sale.rs:68`, `method_election.rs:465`, `optimize_mode1.rs:63`, `optimize_mode2.rs:76`,
`optimize_compliance.rs:70`, `tax_compute.rs:62 & 121`, `kat_tax.rs:1919`, `optimize_accept.rs:119`.
Mechanical + compiler-caught, but enumerate them so Task 1 is sized correctly.

### M2 (Minor) — kind-agnostic `business == true` filter can sweep in Interest, which §1402(a)(2) excludes from SE earnings

The SE base = Σ over `business == true` regardless of `IncomeKind`. Mining is clearly SE (A-9);
business staking/reward is "less settled" as the spec says. **But `IncomeKind::Interest` is
different in kind:** §1402(a)(2) generally **excludes** interest from net earnings from
self-employment (it's investment income / potential §1411 NII), so computing 15.3% SE tax on
business-classified interest would be affirmatively wrong, not merely unsettled. Only triggers if a
user classifies interest `--business` (River hard-codes interest → business:false), so it's an
edge — but recommend **excluding `Interest`** from the SE base (or restricting the filter to
Mining/Staking/Reward), or at minimum disclosing the §1402(a)(2) interest carve-out explicitly.

### M3 (Minor) — goldens don't exercise `round_cents` on `base`/components

All three goldens use round-number inputs that yield exact cents; only Golden 1's `deductible_half`
(7,064.775 → 7,064.78) exercises half-even. Add one KAT with a fractional base (e.g. net_se =
$10,000.10 → base = round_cents(9,235.09235) = **9,235.09**; ss/medicare then off the rounded base)
to make the rounding pipeline a genuine, load-bearing assertion (prompt item 9).

---

## Nits

- **N1** — citation drift: `IncomeKind` is in `event.rs:29`, not state.rs; River path is
  `crates/btctax-adapters/src/sources/river.rs`; `IncomeRecord` is `state.rs:177-185`. Update cites.
- **N2** — document that rounding the intermediate `base` is *intentional* (it mirrors Schedule SE
  line 4a being a reported line item, and matches compute.rs's per-component rounding house style) —
  so a later reviewer doesn't read it as a "round_cents end-only" violation. It is not one.
- **N3 (affirm)** — creating a **separate** `se_addl_medicare_threshold(status)` fn rather than
  reusing `niit_threshold` is correct: §1401(b)(2) and §1411(b) are legally distinct thresholds that
  merely coincide numerically ($200k/$250k/$125k) today; keep them decoupled.

---

## Evaluation of the questions posed

**D5 — standalone (SE tax NOT folded into `total_federal_tax_attributable`): CORRECT, keep it.**
All three reasons hold:
1. The pinned identity `total == ordinary_delta + ltcg_tax + niit` is **real and tested**
   (`kat_rate_engine.rs:551`, `tax_compute.rs:305`); `total` is an income-tax + NIIT **incremental
   delta**. §1401 SE tax is a categorically different liability (Chapter 2, not §1/§1411); folding it
   in would break the identity and the delta semantics.
2. §164(f) coordination is genuinely intractable here: the ½-SE deduction reduces AGI/taxable
   income, but `ordinary_taxable_income` is **user-supplied post-deduction** — the engine cannot know
   whether the user already applied it, so folding SE tax in without (double-)applying the deduction
   would misstate the income-tax portion. ("Circular" is loose wording for "under-determined," but
   the conclusion is sound and conservative.)
3. Precedent: every Phase-2 figure (§170 deduction, 8283, 709) is standalone.
   → **Standalone-but-clearly-reported is the right call.** It should NOT feed the total.

**D4 — w2_ss_wages = $0 default + disclose:** acceptable for the canonical solo miner; the
`TaxProfile.w2_ss_wages` deferral is fine — **but** the disclosure must cover the addl-Medicare
threshold reduction too (I1).

**business filter / River immutability / no Schedule C expenses:** filter approach OK for Mining;
tighten for Interest (M2). River `business:false` immutability correctly deferred (no flip path
exists; confirmed at `sources/river.rs:149-180`). Net SE = gross mining income (no Schedule C
expenses) correctly deferred + disclosed.

**Homes / determinism:** rates as `tables.rs` consts (mirrors `NIIT_RATE`) ✓; `ss_wage_base`
year-indexed in `TaxTable` (mirrors `gift_annual_exclusion`) ✓; Σ over income is order-independent
(deterministic) ✓; exact Decimal ✓. Only rounding correction needed is C1 (and clarify N2).

**Scope / TDD:** right-sized (3 tasks); table-missing-with-business-income "no silent drop" mirrors
P2-C m6 ✓; goldens genuine except need C1's addl>0 case + M3's fractional-base case.

---

## Gate result

**0C/0I: NO.** Must fix before implementation:
- **C1** — `deductible_half = (ss + medicare)/2` (exclude §1401(b)(2)); add an addl>0 golden.
- **I1** — extend the W-2 disclosure to the Additional-Medicare threshold reduction.

Recommend also addressing M1–M3 in the same fold (cheap, and M1/M3 affect whether Task 1 reaches
green). Re-review required after the fold (including the last), per §2.

---

# Round 2 — re-review (post-fold)

- **Artifact:** `design/SPEC_p2d_se_tax.md` (revised).
- **Reviewer role:** independent architect gate (R0), round 2. Numbers re-derived independently
  (Decimal, half-even), not taken from the spec. `TaxTable {` literal inventory re-grepped against
  current source.
- **Verdict:** **NOT GREEN.** C1 closed; M1/M2/M3 closed; D5 unchanged/correct. But the **I1 fold
  introduced a new directional error (I2, Important):** the D4/D3 disclosure says the
  Additional-Medicare component "may be **overstated**" for an employed miner — it is **understated**.
  Not 0-new-C/I → re-fold + re-review required.

## Fold-by-fold

**C1 — CLOSED (Critical, tax correctness).** D2 line 61-64 now pins
`deductible_half = round_cents((ss + medicare) / 2)` and states the §164(f)(1) carve-out
("EXPRESSLY EXCLUDES the §1401(b)(2) Additional Medicare Tax … 0.9% is a Form 8959 item, Schedule SE
line 13 = SS + regular Medicare only"). Task 1 (line 122-124) pins the addl>0 golden:
$300k Single → base 277,050.00; ss 21,836.40; medicare 8,034.45; addl 693.45; total 30,564.30;
**deductible_half = (21,836.40 + 8,034.45)/2 = 14,935.42** (independently re-derived, half-even), and
explicitly labels the wrong `total/2` = 15,282.15 as the anti-regression lock. Golden 1 deductible
unchanged at **7,064.78** (re-derived: (11,451.40 + 2,678.15)/2 = 7,064.775 → half-even → .78). ✓
Formula and pinned figures aligned across D2 + Task 1 + CSV/render. No residual on C1.

**I1 — NOT cleanly closed → new Important (I2).** The omission is addressed (D4 now names **both**
W-2 interactions), **but the fold states the wrong direction for the Additional-Medicare component.**
D4 line 84-88: "the 0.9% Additional-Medicare threshold … is REDUCED by your W-2 Medicare wages — so
**both the SS component and the Additional-Medicare component may be overstated** here; adjust
accordingly." The mechanism clause ("threshold is reduced") is right; the conclusion is wrong for the
addl component. A reduced threshold means **more** SE income is taxed at 0.9%, so the actual liability
is **higher** than the tool's $0-W-2 figure → **understated**, never overstated
(addl_actual = 0.9%·max(0, base − max(0, thr − w2_medicare)) ≥ addl_tool = 0.9%·max(0, base − thr)).
This directly contradicts the round-1 I1 fix text, which said "the 0.9% shown here is **understated**
accordingly." The two W-2 effects move in **opposite** directions: SS **overstated** (cap lowered),
addl **understated** (threshold lowered) — the spec lumps both as "overstated."

Worked example (independently computed): employed Single miner, W-2 Medicare wages $150,000,
SE base $100,000 → tool addl (assumes $0 W-2) = 0.9%·max(0, 100,000 − 200,000) = **$0.00**; actual
(Form 8959 Pt II: line 11 = 200,000 − 150,000 = 50,000; line 12 = 100,000 − 50,000 = 50,000) =
0.9%·50,000 = **$450.00**. The tool is $450 low, yet the disclosure tells the employed miner it "may
be overstated … adjust accordingly" — i.e., steers them to reduce a figure they should **increase**.

- **Severity: Important.** Same class as round-1 I1 (default solo-miner computed figure is correct;
  the defect is a user-facing caveat that misleads the employed-miner path). A wrong-direction caveat
  is at least as harmful as the omitted one it replaced — it affirmatively states a false direction of
  tax liability. Text-only fix.
- **Fix:** split the direction — "the SS component may be **overstated** (cap reduced by W-2 SS
  wages); the Additional-Medicare component may be **understated** (threshold reduced by W-2 Medicare
  wages → more SE income taxed at 0.9%)." Propagate to the D3 `render_schedule_se` disclosure (D3 line
  73 defers to the D4 text, so it inherits the same error).

**M2 — CLOSED (tax correctness).** D2 line 51-55: `net_se` filter now
`business == true && kind != IncomeKind::Interest && recognized_at.year() == year`, with the
§1402(a)(2) rationale and B-M1 consistency. Task 1 (line 125-126) pins the KAT: business `Interest` →
NOT in net_se; business `Mining` IS. Mining/Staking/Airdrop/Reward with `business==true` included. ✓

**M1 — CLOSED (adequately).** Task 1 line 114-116 now directs "grep `TaxTable {` across the workspace
(~12 sites …) and update ALL, not just the 2 named here." Re-grep confirms **13** literal construction
sites (excluding the `struct`/`impl`/`fn -> TaxTable` lines): `adapters/tax_tables.rs:178`,
`cli/render.rs:1433`, `core/tables.rs:193` (synthetic_table), `cli/tests/optimize_accept.rs:119`,
`core/tests/kat_tax.rs:1919`, `method_election.rs:465`, `optimize_compliance.rs:70`,
`optimize_mode1.rs:63`, `optimize_mode2.rs:76`, `optimize_score.rs:67`, `optimize_wash_sale.rs:68`,
`tax_compute.rs:62`, `tax_compute.rs:121`. That's the 2 production literals + 11 others (10 test
literals across 9 files + render.rs). The "~12 / ~11 test files" wording is slightly loose (11 other
literals, and tax_compute holds 2), but the operative instruction — grep-and-update-ALL, don't
undercount to 2 — is present and correct. **Nit N4:** tighten "~12 sites: ~11 test files + render.rs"
to "13 literal sites total (2 production + 11 others: 10 test literals across 9 files + render.rs)."
Non-blocking.

**M3 — CLOSED.** Task 1 line 126: fractional-base KAT, mining $12,345.67 → base =
round_cents(12,345.67 × 0.9235). Re-derived: 11,401.226245 → half-even → **11,401.23** (genuinely
exercises `round_cents` on `base`). ✓

**D5 — unchanged, still correct.** Standalone treatment (not folded into
`total_federal_tax_attributable`) and the three reasons are intact (D5 line 90-97); round-1 analysis
stands. No new issue.

## New / residual findings

- **I2 (Important, NEW — tax-correctness of disclosure):** D4/D3 Additional-Medicare W-2 disclosure
  states the wrong direction ("overstated" → must be "understated"). Blocking. Fix above.
- **N4 (Nit):** M1 site count wording ("~12 / ~11 test files") vs actual 13 literal sites
  (11 beyond the 2 named). Non-blocking.

No other new Critical/Important. C1 exclusion, the net_se filter, the CSV/render component set, and the
standalone note are internally consistent; the deductible formula is aligned everywhere.

## Gate result — Round 2

**0C / 0I: NO.** 0 Critical (C1 closed), **1 Important (I2, new)**. The spec is **not R0 GREEN**.
Single remaining blocker: correct the Additional-Medicare W-2 disclosure direction to **understated**
(and split it from the SS "overstated" direction) in D4 + the D3 render text. Cheap, text-only. Fold
N4 opportunistically. Re-review required after the fold (including the last), per §2.

---

# Round 3 — re-review (post-I2-fold, focused)

- **Artifact:** `design/SPEC_p2d_se_tax.md` (revised).
- **Reviewer role:** independent architect gate (R0), round 3. Scope: confirm the single round-2
  residual (I2, the W-2 disclosure direction) is now correct; verify the one-paragraph edit introduced
  no new issue. Directions and the worked example re-derived independently, not taken from the spec.
- **Verdict:** **GREEN.** I2 CLOSED with correct directions; 0 Critical / 0 Important.

## I2 — CLOSED (disclosure directions correct)

D4 (lines 84-89) now states the two W-2 effects with **opposite** directions, each correct:

1. **Social Security component — OVERSTATED.** Text: "the 12.4% Social Security component here may be
   **OVERSTATED** — its cap is the wage base LESS your W-2 Social-Security wages (a lower cap → less
   SS)." Independently: `ss = 12.4% × min(base, cap)`; real `cap = ss_wage_base − w2_ss_wages ≤`
   tool's full `ss_wage_base` → `ss_tool ≥ ss_real` → tool **overstates**. ✓ Matches.
2. **Additional-Medicare component — UNDERSTATED.** Text: "the 0.9% Additional-Medicare component here
   may be **UNDERSTATED** — the §1401(b)(2)(B)/Form 8959 threshold is REDUCED by your W-2 Medicare
   wages (a lower threshold → MORE income taxed at 0.9%)." Independently:
   `addl = 0.9% × max(0, base − thr)`; real `thr = max(0, threshold − w2_medicare) ≤` tool's full
   `threshold` → `addl_tool ≤ addl_real` → tool **understates**. ✓ Matches.

**Worked example (re-derived, half-even):** employed Single miner, W-2 Medicare $150,000, base
$100,000. Real addl = 0.9% × (100,000 − (200,000 − 150,000)) = 0.9% × 50,000 = **$450.00**; tool addl
(full $200k threshold) = 0.9% × max(0, 100,000 − 200,000) = **$0.00** → tool is $450 low →
**understated**. The spec's stated direction agrees. ✓

The round-2 defect (both components lumped as "overstated," steering the employed miner to reduce a
figure they should increase) is gone. `grep -niE "overstat|understat"` over the spec returns exactly
two hits — line 85 SS = OVERSTATED, line 87 addl = UNDERSTATED — with **no residual "both overstated"
wording anywhere**. Mechanism clauses are also precise: SS cap reduced by W-2 *Social-Security* wages,
addl threshold reduced by W-2 *Medicare* wages (no conflation of the two wage bases).

## D3 inheritance — correct

D3 line 73 renders "+ the D4 W-2 disclosure" (defers to D4), so it inherits the corrected directions;
D3 carries no independent directional wording of its own → no divergent copy to drift. ✓

## No new issue from the edit

The change is confined to the one D4 paragraph. Spot-checked the pieces the fold could have disturbed:
- **Deductible formula (C1) — untouched.** D2 line 61: `deductible_half = round_cents((ss + medicare) / 2)`
  still excludes `addl`; §164(f)(1) carve-out rationale intact; Task 1 addl>0 golden ($300k Single →
  $14,935.42) unchanged.
- **net_se filter (M2) — untouched.** D2 line 51-55 still
  `business == true && kind != IncomeKind::Interest && recognized_at.year() == year`.
- **Goldens (Task 1) — untouched.** Golden 1 ($14,129.55 / deductible $7,064.78), $250k cap, $300k
  addl, fractional-base ($12,345.67 → base $11,401.23) all intact.
- **D5 standalone — unchanged, still correct.**

N4 (M1 site-count wording, ~12 vs 13 literal sites) remains a non-blocking nit; optional to fold, does
not affect the gate.

## Gate result — Round 3

**0 Critical / 0 Important — spec is R0 GREEN, ready to implement.** No further review loop required.
