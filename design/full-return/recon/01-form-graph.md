# Recon 01 — Form Graph & Line-Flow DAG (Full US Individual Return)

**Agent:** Recon 1 of 5 (first pass). **Date:** 2026-07-11.
**Scope:** "Common W-2 household" federal return wrapping the existing crypto engine —
W-2 wages, 1099-INT / 1099-DIV, standard-vs-itemized (Schedule A), Schedule B,
Schedule 1 basics, Schedules 2/3 when triggered, crypto 8949 / Schedule D, and the
absolute-value NIIT (8960) / Additional Medicare (8959) the engine already computes as
deltas. **PDF output only, manual entry. Targets: TY2024 (final) + TY2025 (draft).**

## Source verification

Every line number below was read **directly from the official IRS PDFs**, not memory:

| Form | Source URL | Status read |
|---|---|---|
| Form 1040 (2024) | `irs.gov/pub/irs-prior/f1040--2024.pdf` | FINAL, both pages |
| Schedule 1 (2024) | `.../f1040s1--2024.pdf` | FINAL, both pages |
| Schedule 2 (2024) | `.../f1040s2--2024.pdf` | FINAL, both pages |
| Schedule 3 (2024) | `.../f1040s3--2024.pdf` | FINAL |
| Schedule A (2024) | `.../f1040sa--2024.pdf` | FINAL |
| Schedule B (2024) | `.../f1040sb--2024.pdf` | FINAL |
| Schedule D (2024) | `.../f1040sd--2024.pdf` | FINAL, both pages |
| Form 8959 (2024) | `.../f8959--2024.pdf` | FINAL |
| Form 8960 (2024) | `.../f8960--2024.pdf` | FINAL |
| Form 8995 (2024) | `.../f8995--2024.pdf` | FINAL |
| Form 1040 (2025) | `irs.gov/pub/irs-dft/f1040--dft.pdf` | **DRAFT** created 9/5/25, both pages |
| Schedule 1-A (2025) | `.../f1040s1a--dft.pdf` | **DRAFT** created 11/4/25, both pages |
| Schedule A | `.../f1040sa--dft.pdf` | **DRAFT is already the 2026 rev** (created 5/12/26) — see §7 RISK |

> **Caution:** TY2025 numbers below come from IRS *draft* forms marked "DRAFT — DO NOT
> FILE." Line numbers are stable in practice across late drafts → final, but the Fable
> second pass must re-verify against the released final 2025 forms.

---

## 1. Form inventory & trigger conditions

| # | Form / Schedule | In-scope? | Trigger condition (when it must be filed / computed) |
|---|---|---|---|
| A | **Form 1040** | ALWAYS | Always — the return itself. |
| B | **Schedule B** (Interest & Ordinary Dividends) | YES | Required if taxable interest (1040 2b) **OR** ordinary dividends (1040 3b) **> $1,500**; also if seller-financed mortgage interest, foreign accounts/trusts (Part III), etc. Below $1,500 with no other trigger → Schedule B optional; amounts still go straight to 1040 2b/3b. |
| C | **Schedule D** (Capital Gains & Losses) | YES (crypto) | Required whenever there are capital gains/losses — i.e., any crypto disposition (existing engine) or capital-gain distributions. Can be skipped only if the *only* capital item is cap-gain distributions **and** no other reason (then 1040 line 7 "Sch D not required" box). |
| D | **Form 8949** (Sales & Dispositions) | YES (crypto) | Required to itemize each capital transaction feeding Schedule D lines 1b/2/3 (ST) and 8b/9/10 (LT). Already produced by the crypto engine. |
| E | **Schedule A** (Itemized Deductions) | YES | Only if the taxpayer **itemizes** (itemized total > standard deduction, or MFS-forced, or elects). Otherwise the standard deduction is used and Schedule A is not filed. |
| F | **Schedule 1** (Additional Income & Adjustments) | PARTIAL | File if there is **any** Part I additional income (line 10 ≠ 0) **or** any Part II adjustment (line 26 ≠ 0). For a W-2 household the common live lines are: Part I L1 state-refund, L7 unemployment; Part II L15 ½-SE-tax (from crypto SE), L20 IRA, L21 student-loan interest, L13 HSA. |
| G | **Schedule 2** (Additional Taxes) | YES (conditional) | File if Part I (AMT L2 / excess APTC repayment L1) **or** Part II other taxes are nonzero. In our scope Part II is the live one: **L4 SE tax, L11 Additional Medicare (8959), L12 NIIT (8960)**. |
| H | **Schedule 3** (Additional Credits & Payments) | YES (conditional) | File if any nonrefundable credit (Part I, e.g. L2 dependent-care 2441, L3 education 8863, L1 foreign tax) **or** other payment/refundable credit (Part II, e.g. L9 net PTC, L11 excess SS withheld) exists. |
| I | **Form 8960** (NIIT) | YES | File if MAGI > threshold ($200k S/HOH, $250k MFJ, $125k MFS) **and** net investment income > 0. Engine already computes §1411. |
| J | **Form 8959** (Additional Medicare Tax) | YES | File if Medicare wages (W-2 box 5) + SE income + RRTA > threshold ($200k S/HOH, $250k MFJ, $125k MFS), **or** if W-2 box 6 shows employer-withheld Add'l Medicare (withholding reconciliation, Part V). |
| K | **Schedule SE** (Self-Employment Tax) | YES (crypto/1099) | File if net SE earnings ≥ $400. Engine already computes §1401. |
| L | **Form 8995 / 8995-A** (QBI Deduction) | MARGINAL | File if there is qualified business income / REIT dividends / PTP income. **Usually zero for a pure W-2 household**; line exists but is typically 0. 8995 (simplified) if taxable income before QBI ≤ $191,950 ($383,900 MFJ, 2024). |
| M | **Form 8283** (Noncash Charitable) | YES (crypto gift) | Required when noncash gifts > $500 (Schedule A line 12). Already produced by the crypto engine for donated BTC. |
| N | **Schedule 8812** (Child Tax Credit) | OUT (flag) | CTC / ACTC. Feeds 1040 L19 & L28. Likely out of first scope but header dependents collection is needed. |
| O | **Schedule 1-A** (Additional Deductions) | **NEW TY2025** | OBBBA below-the-line deductions (tips / overtime / car-loan interest / senior). File if claiming any. See §5. Did not exist in 2024. |

**Not in scope** (note for completeness, do NOT build): Schedule C/E/F, Form 6251 (AMT),
Form 8962 (PTC), Form 2441/8863/8880 mechanics beyond passing a number, EIC worksheet.

---

## 2. TY2024 source → destination line-flow table (the DAG edges)

Read as directed edges "money leaves LEFT, lands on RIGHT." All line numbers verified
against the 2024 PDFs.

### Income assembly → Form 1040 page 1

| Source | → Destination | Notes |
|---|---|---|
| W-2 box 1 (all W-2s) | **1040 L1a** | "Total amount from Form(s) W-2, box 1." Sub-lines 1b–1h are niche; 1z = sum of 1a–1h. |
| 1099-INT / 1099-OID interest | Sch B Part I L1 → **L2 → L4** → **1040 L2b** | If Sch B required (>$1,500). Tax-exempt interest → 1040 **L2a** (informational). |
| 1099-DIV box 1a (ordinary div) | Sch B Part II L5 → **L6** → **1040 L3b** | If Sch B required. |
| 1099-DIV box 1b (qualified div) | **1040 L3a** | Direct; not on Sch B. Feeds the §1(h) rate worksheet (see L16). |
| 1099-DIV box 2a (cap-gain distrib) | **Sch D L13** → Sch D L16 → **1040 L7** | Flows through Schedule D. If Sch D not otherwise required, may go direct to L7 with box checked. |
| 1099-R taxable pension/IRA | 1040 L4b / L5b | Out of core scope but lines exist. |
| SSA-1099 | 1040 L6b (taxable amount) | Out of core scope. |
| **Crypto 8949/Sch D net** | **Sch D L16 → 1040 L7** | See §4. |
| Schedule 1 Part I total | **Sch 1 L10 → 1040 L8** | Additional income (state refund L1, unemployment L7, etc.). |
| — | **1040 L9 = 1z+2b+3b+4b+5b+6b+7+8** | **Total income.** |
| Schedule 1 Part II total | **Sch 1 L26 → 1040 L10** | Adjustments to income (incl. ½ SE tax L15). |
| — | **1040 L11 = L9 − L10** | **Adjusted gross income (AGI).** |

### Deduction → taxable income → tax (Form 1040 page 2)

| Source | → Destination | Notes |
|---|---|---|
| Std deduction (filing-status table) **OR** Schedule A L17 | **1040 L12** | Take the larger (branch). 2024 std: $14,600 S/MFS · $29,200 MFJ/QSS · $21,900 HOH. |
| Form 8995 L15 (QBI) | **1040 L13** | Usually 0 in scope. |
| — | **1040 L14 = L12 + L13**; **L15 = L11 − L14** | **L15 = taxable income.** |
| Tax (§1(h) QDCGT worksheet / bracket) | **1040 L16** | Uses L15 + qualified div (3a) + net LTCG (Sch D). Engine already does §1(h) stacking. |
| **Schedule 2 L3** (Part I: AMT + APTC) | **1040 L17** | Usually 0 in scope. |
| — | **1040 L18 = L16 + L17** | |
| Schedule 8812 (CTC/ODC) | **1040 L19** | Out of core scope. |
| **Schedule 3 L8** (Part I nonrefundable credits) | **1040 L20** | Dependent-care/education/foreign-tax etc. |
| — | **L21 = L19+L20**; **L22 = L18 − L21** (≥0) | |
| **Schedule 2 L21** (Part II other taxes) | **1040 L23** | **SE tax + Add'l Medicare + NIIT land here.** See §4. |
| — | **1040 L24 = L22 + L23** | **TOTAL TAX.** |

### Payments → refund/owed

| Source | → Destination | Notes |
|---|---|---|
| W-2 box 2 (fed withholding) | **1040 L25a** | |
| 1099 withholding (box 4 etc.) | **1040 L25b** | |
| **Form 8959 L24** (Add'l Medicare *withholding*) | **1040 L25c** | ⚠️ withholding side of 8959, distinct from the tax on Sch 2 L11. |
| — | **L25d = 25a+25b+25c** | |
| Estimated payments + prior-year applied | **1040 L26** | |
| EIC | 1040 L27 | Out of scope. |
| Additional CTC (8812) | 1040 L28 | Out of scope. |
| **Schedule 3 L15** (Part II payments/refundable) | **1040 L31** | Net PTC L9, excess SS withheld L11, etc. |
| — | **L32 = 27+28+29+31**; **L33 = 25d+26+32** | **Total payments.** |
| — | **L34 = L33 − L24** if >0 (overpaid) → **L35a** refund / **L36** applied | |
| — | **L37 = L24 − L33** if >0 (amount owed); **L38** est-tax penalty | |

---

## 3. Form 1040 header inputs (non-line data the engine must collect)

- **Filing status** (checkbox, exactly one): Single / MFJ / MFS / HOH / QSS. Drives std
  deduction, bracket tables, NIIT & Add'l-Medicare thresholds, SALT-cap MFS halving.
- **Names, SSNs** (taxpayer + spouse), address, foreign-address fields.
- **Digital-asset question** — "At any time during 2024 did you (a) receive… or (b)
  sell/exchange/dispose of a digital asset?" **Already handled by btctax.** (2025 wording
  identical for 2025.)
- **Dependents** — name / SSN / relationship / CTC vs ODC checkbox per dependent (needed
  for L19/L28 even if CTC is out of first scope).
- **Standard-Deduction checkboxes** — "Someone can claim you/spouse as a dependent";
  "Spouse itemizes on a separate return / dual-status alien"; **Age/Blindness** ("born
  before Jan 2, 1960" ×2, "blind" ×2). These modify the standard-deduction amount.
- **Presidential Election Campaign** — $3 checkbox You/Spouse (no tax effect; pass-through).
- Third-Party Designee, signature/occupation, Paid Preparer — pass-through metadata.

---

## 4. Where the EXISTING crypto engine outputs land (and the delta→absolute problem)

The engine today emits **deltas** (tax-with-crypto − tax-without). The full return needs
**absolute** values on real lines. Mapping:

| Engine output | Real form path (2024) |
|---|---|
| Per-lot dispositions | **Form 8949** Part I (ST, box A/B/C) / Part II (LT, box D/E/F) |
| 8949 ST subtotals | **Sch D L1b/2/3** → **L7** (net ST) |
| 8949 LT subtotals | **Sch D L8b/9/10** → **L15** (net LT) |
| §1211/§1212 loss limit (−$3,000 / −$1,500 MFS cap, carryover) | **Sch D L16 / L21** |
| Net capital gain/(loss) | **Sch D L16 → 1040 L7** |
| §1(h) 0/15/20 stacking | consumed in **1040 L16** (QDCGT worksheet) |
| Donated-BTC noncash gift | **Form 8283** → **Sch A L12** |
| **§1411 NIIT** | **Form 8960 L17 → Sch 2 L12 → Sch 2 L21 → 1040 L23** |
| **§1401 SE tax** | Sch SE → **Sch 2 L4** (tax) **and** ½ → **Sch 1 L15 → L26 → 1040 L10** (adjustment) |
| **§0.9% Additional Medicare** | **Form 8959 L18 → Sch 2 L11 → Sch 2 L21 → 1040 L23**; *plus* 8959 L24 withholding → **1040 L25c** |

**The critical shift:** the engine's NIIT/Add'l-Medicare deltas must become **full Form
8960 / 8959 computations** on absolute inputs:
- **Form 8960 (NIIT):** L1 taxable interest, L2 ordinary dividends, L5a net gain from
  disposition (the crypto Sch D result), L8 total investment income, L12 net investment
  income, **L13 MAGI (≈ AGI + certain foreign exclusions)**, L14 threshold, L16 = min(NII,
  MAGI−threshold), **L17 = 3.8% × L16**. → so NIIT now depends on **absolute AGI + absolute
  interest/dividends**, not just crypto.
- **Form 8959 (Add'l Medicare):** Part I on **W-2 box 5 Medicare wages** (new input), Part
  II on SE income (Sch SE L6), threshold by filing status, 0.9%. Part V reconciles
  employer withholding (W-2 box 6) → 1040 L25c.

This is the single biggest engine change: **NIIT and Add'l-Medicare stop being crypto-only
deltas and become whole-household absolute computations** that need W-2 box 5/6, interest,
dividends, and AGI as first-class inputs.

---

## 5. TY2025 deltas (⚠️ decision-relevant — page 2 is renumbered)

The 2025 draft Form 1040 **renumbers page 2** and adds an OBBBA below-the-line deduction
schedule. This is not cosmetic — the engine's PDF-fill line map must be **year-parameterized**.

### 5a. Form 1040 (2025 draft) line renumbering

| Concept | 2024 line | **2025 line** |
|---|---|---|
| Capital gain/(loss) → from Sch D | **L7** | **L7a** (+ L7b checkboxes "Sch D not required" / "includes child's gain") |
| Adjusted gross income | **L11** | **L11a** (page 1) **and L11b** (repeated top of page 2) |
| "Someone can claim" / dual-status / **age & blindness** checkboxes | header block | moved to **page-2 L12a–L12d** |
| Standard-or-itemized deduction | **L12** | **L12e** |
| QBI deduction | **L13** | **L13a** |
| **Additional deductions from Schedule 1-A** | — (n/a) | **L13b** ← NEW |
| Add deductions | L14 = 12+13 | **L14 = 12e + 13a + 13b** |
| Taxable income | L15 = L11−L14 | **L15 = L11b − L14** |
| Refundable adoption credit (Form 8839) | L30 "Reserved" | **L30** now used |
| EIC | L27 | **L27a** (+27b clergy, 27c opt-out) |
| Total other payments | L32 = 27+28+29+31 | **L32 = 27a+28+29+30+31** |
| Age/blindness cutoff birth-year | "before Jan 2, 1960" | **"before Jan 2, 1961"** |

**Unchanged 2024→2025** (verified on the 2025 draft): the Schedule cross-refs L17 (Sch 2
L3), L20 (Sch 3 L8), L23 (Sch 2 L21), L31 (Sch 3 L15), L8 (Sch 1 L10), L10 (Sch 1 L26),
and the whole payments spine L24/L25/L33/L34/L37. **So Schedules 1/2/3 flow to the same
1040 line numbers in both years**; only page-2 deduction/income lines moved.

### 5b. 2025 standard deduction amounts (OBBBA, verified on draft 1040 margin)

$15,750 Single/MFS · $31,500 MFJ/QSS · $23,625 HOH (vs 2024 $14,600 / $29,200 / $21,900).

### 5c. NEW **Schedule 1-A** — "Additional Deductions" (below-the-line, OBBBA, 2025–2028)

Verified on the 11/4/25 draft. **These reduce taxable income but NOT AGI** — they sit
alongside std/itemized + QBI on 1040 L13b:

- **Part I** L1–L3: **MAGI** = 1040 L11b + foreign exclusions (Puerto Rico, Form 2555
  L45/L50, Form 4563 L15). Every deduction below phases out against this MAGI.
- **Part II — No Tax on Tips** (L4–L13): qualified tips, cap **$25,000**, phase-out $100/
  $1,000 over MAGI $150k ($300k MFJ). → **L13**.
- **Part III — No Tax on Overtime** (L14–L21): OT premium, cap **$12,500 ($25,000 MFJ)**,
  same phase-out. → **L21**.
- **Part IV — No Tax on Car Loan Interest** (L22–L30): QPVLI, cap **$10,000**, phase-out
  $200/$1,000 over MAGI $100k ($200k MFJ); US-assembled vehicle, loan after 12/31/2024. → **L30**.
- **Part V — Enhanced Senior Deduction** (L31–L37): **$6,000 per qualifying spouse 65+**
  (born before Jan 2, 1961), phase-out 6% over MAGI $75k ($150k MFJ). → **L37**.
- **Part VI** L38 = L13+L21+L30+L37 → **1040 L13b**.

**Scope call for the team:** Schedule 1-A is *not* in the original scope bullet list, but
the **senior deduction** and **tips/overtime** are extremely common in a "W-2 household,"
and TY2025 is a stated target. Recommend at minimum wiring L13b as an input and Parts II/V
as the likely-needed parts; Part IV (car loan) is nichier. Flag for Spec.

### 5d. SALT cap change

- **TY2025:** SALT cap raised **$10,000 → $40,000** ($20,000 MFS), phase-out 30¢/$ over
  MAGI **$500,000** ($250,000 MFS), floor $10,000. (Schedule A line 5e.)
- The live draft Schedule A URL now serves the **2026** rev (cap $40,400 / $20,200,
  threshold $505,000 — the 1%/yr bump). **I could not pull an official 2025 Schedule A
  draft** — see RISK in §7.

---

## 6. TOPOLOGICALLY ORDERED COMPUTATION SEQUENCE (the engine front-end spec)

This ordering **is** the spec: each step may only read outputs of earlier steps. Bracketed
tags mark [2025-only] and [crypto-engine-exists].

```
STAGE 0 — HEADER / STATIC
  0.1  Filing status, dependents, age/blindness flags, digital-asset answer[exists]
  0.2  Standard-deduction table lookup for status (+ age/blindness bumps)

STAGE 1 — INCOME AGGREGATION  (→ 1040 L1..L8, total income L9)
  1.1  W-2 box 1 sum                          → L1a/1z
  1.2  Interest: Sch B Part I → L2b
  1.3  Dividends: ordinary → Sch B Part II → L3b;  qualified → L3a
  1.4  CRYPTO[exists]: 8949 → Sch D → cap-gain dist L13 → Sch D L16 → L7 (2024) / L7a (2025)
  1.5  Schedule 1 Part I additional income → Sch1 L10 → L8
  1.6  L9 = 1z+2b+3b+4b+5b+6b+7+8            (TOTAL INCOME)

STAGE 2 — ADJUSTMENTS → AGI  (→ 1040 L10, L11)
  2.1  Schedule SE[exists] ½-SE-tax → Sch1 L15
  2.2  other Part-II adjustments (IRA L20, student-loan L21, HSA L13) → Sch1 L26 → L10
  2.3  L11 = L9 − L10                          (AGI)   ★ pivot: many downstream deps read AGI

STAGE 3 — DEDUCTIONS  (needs AGI)
  3.1  Schedule A: medical floor = 7.5%×AGI (L2 reads L11); SALT L5e (cap by year/status);
       mortgage int L8; charity L11-14 (incl 8283[exists]); → Sch A L17 total
  3.2  Choose deduction = max(standard, Sch A L17)   [branch; MFS-force / elect overrides] → L12 (2024) / L12e (2025)
  3.3  QBI: Form 8995 needs taxable-income-before-QBI (= AGI − L12) AND netLTCG+qualDiv → L13 (2024) / L13a (2025)
  3.4  [2025-only] Schedule 1-A: MAGI = L11b + foreign excl → Parts II/III/IV/V → L38 → L13b
  3.5  L14 = deduction + QBI (+ L13b in 2025);  L15 = AGI − L14   (TAXABLE INCOME)

STAGE 4 — REGULAR TAX  (needs taxable income + qual-div + net LTCG)
  4.1  §1(h) QDCGT worksheet / ordinary bracket[exists] → L16
  4.2  Schedule 2 Part I (AMT L2 / APTC L1) → Sch2 L3 → L17;  L18 = L16+L17

STAGE 5 — NONREFUNDABLE CREDITS
  5.1  CTC/ODC (8812) → L19  [out of core scope]
  5.2  Schedule 3 Part I (dependent-care, education, foreign tax) → Sch3 L8 → L20
  5.3  L21 = L19+L20;  L22 = max(0, L18 − L21)

STAGE 6 — OTHER TAXES  (Schedule 2 Part II)  ★ absolute NIIT / Add'l-Medicare live here
  6.1  Schedule SE[exists] → Sch2 L4
  6.2  Form 8959 (needs W-2 box5 + SE income + status threshold) → L18 → Sch2 L11
  6.3  Form 8960 (needs AGI/MAGI + investment income incl crypto Sch D) → L17 → Sch2 L12
  6.4  Sch2 L21 = L4 + L7..L16 + ... → 1040 L23
  6.5  L24 = L22 + L23                          (TOTAL TAX)

STAGE 7 — PAYMENTS  (needs Form 8959 Part V from Stage 6)
  7.1  W-2 box2 → L25a;  1099 wh → L25b;  8959 L24 wh → L25c;  L25d = sum
  7.2  estimated → L26;  Schedule 3 Part II → Sch3 L15 → L31;  L32; L33 = total payments

STAGE 8 — SETTLE
  8.1  if L33>L24: L34 overpaid → L35a refund / L36 applied
  8.2  else: L37 amount owed;  L38 penalty
```

**Key ordering facts / non-obvious dependencies:**
- **AGI (Stage 2) is the pivot.** Schedule A medical (7.5% floor), the SALT phase-out,
  Form 8960 MAGI, Schedule 1-A MAGI, and the itemized-limitation all read AGI. Nothing
  before Stage 3 may read a deduction.
- **QBI (3.3) reads BOTH the deduction (AGI−L12) AND net capital gain + qualified
  dividends.** Even though usually 0 in scope, the dependency is real → compute after Sch D.
- **No true cycle:** Sch A medical needs AGI, not taxable income; QBI needs
  taxable-income-*before*-QBI. The graph is a DAG.
- **8959 spans two stages:** the *tax* (L18→Sch 2 L11, Stage 6) and the *withholding*
  (L24→1040 L25c, Stage 7). Don't collapse them.
- **Deduction branch** (3.2) is the only real either/or; everything else is additive.

---

## 7. Risks, surprises, open questions (for Spec + Fable pass)

1. **RISK — 2025 Schedule A not retrievable as its own draft.** `f1040sa--dft.pdf` already
   serves the **2026** revision (created 5/12/26). TY2025 Schedule A structure is therefore
   *inferred*: SALT cap $40,000/$20,000 MFS, phase-out over MAGI $500,000/$250,000. The
   2026 draft additionally shows major restructuring — 8d PMI restored, charity line 13 →
   "Charitable Contribution Limitation Worksheet" (0.5%-AGI floor), gambling-loss line 17a
   (90% limit), and a **line-18 itemized-deduction limitation at $384,350** (OBBBA §68-style
   benefit cap). **These are TY2026, effective for tax years beginning after 12/31/2025 —
   almost certainly NOT on the 2025 Schedule A.** Fable pass must pull the *final* 2025
   Schedule A to confirm 2025 keeps the 2024 charity/other-itemized structure.
2. **SURPRISE — page-2 renumbering, not just amounts.** 2025 moves std/itemized to **L12e**,
   QBI to **L13a**, cap-gain to **L7a**, AGI to **L11a/L11b**, and adds **L13b (Schedule
   1-A)**. Any hard-coded 2024 line map will silently mis-fill 2025 PDFs. Line map MUST be
   year-keyed.
3. **DECISION — Schedule 1-A (2025) scope.** Senior ($6k) + tips/overtime deductions are
   common in a W-2 household and TY2025 is a target, yet Schedule 1-A isn't in the original
   scope list. Recommend wiring 1040 **L13b** + at least Parts II & V. Escalate to Spec.
4. **ENGINE — NIIT & Add'l-Medicare go absolute.** The biggest lift: Forms 8960/8959 need
   whole-household inputs (W-2 box 5/6, interest, dividends, AGI) instead of crypto-only
   deltas. New required inputs: **W-2 box 5 (Medicare wages), box 6 (Medicare tax
   withheld)**, currently not modeled.
5. **INPUT MODEL — the two opaque scalars must be decomposed.** Today's
   `ordinary_taxable_income` / `magi_excluding_crypto` cannot survive: the full return needs
   AGI, standard-vs-itemized, and each income class as first-class values. Recon 2/3 (input
   model) should treat these scalars as *replaced*, not extended.
6. **QBI marginal.** 8995 line exists but is ~always 0 for a pure W-2 household; keep the
   line/PDF cell but the compute can be a stub returning 0 unless business income is added.
7. **Digital-asset question** already handled — no new work, just confirm it still maps on
   the 2025 header (wording verified identical).
8. **$1,500 Schedule B threshold** is a *filing* trigger, not a flow trigger — interest/
   dividends land on 1040 2b/3b regardless; Schedule B is just the itemized backup + Part
   III foreign-account questions.
