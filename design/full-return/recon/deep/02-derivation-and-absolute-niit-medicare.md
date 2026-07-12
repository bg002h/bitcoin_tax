# Deep Recon 02 — `derive_tax_profile` to the cent + Absolute Form 8960 (NIIT) / 8959 (Add'l Medicare)

**Agent:** Opus deep-dive recon, round 2 (spec-grade lock). **Date:** 2026-07-11.
**Builds on:** first-pass `01-form-graph.md` §4, `02-computation-worksheets.md` §7,
`04-input-data-model.md` §5, and the Fable F4 brief in `FABLE_RECON.md`. This report **verifies,
corrects, and deepens** those to spec grade with worked MFJ examples proven to the cent against the
**actual bundled TY2024 table** (`crates/btctax-adapters/src/tax_tables.rs:234` `ty2024()`) and the
**frozen engine** (`crates/btctax-core/src/tax/compute.rs`, `se.rs`, `tables.rs`).

**Primary IRS sources read directly this pass** (PDFs fetched + `pdftotext`-extracted, not memory):
- **2025 Form 8960** (`irs.gov/pub/irs-pdf/f8960.pdf`) — Part I/II/III line labels transcribed verbatim.
- **2024 Instructions for Form 8960** (`irs.gov/pub/irs-prior/i8960--2024.pdf`) — **Line 13 MAGI**
  section + **Line 13—MAGI Worksheet** + **Line 5a** transcribed verbatim (the load-bearing cites).
- **2025 Form 8959** (`irs.gov/pub/irs-pdf/f8959.pdf`) — all five Parts, lines 1–24, transcribed verbatim.
- **2025 Instructions for Form 8959** (`irs.gov/instructions/i8959`) — Part II threshold-coordination
  worked example.

Scope for this report is **TY2024 MFJ** (v1). Absolute liability. Rounding-method (Layer 0 Tax
Table / whole-dollar) is a **separate** work item — figures below are the engine's exact-Decimal cent
values; where a form line would round to whole dollars I say so and cross-reference Layer 0.

---

## 0. Corrections / confirmations to the first pass (LOUD)

| # | First-pass claim | Verdict | Note |
|---|---|---|---|
| C1 | `magi_excluding_crypto = AGI` because tax-exempt interest is **not** a §1411 add-back (`04` §5 step 12; `00-SYNTHESIS` §7) | **CONFIRMED — to primary source, and STRONGER than stated** | 2024 Form 8960 instr., *Line 13—MAGI*: "**If you didn't exclude any amounts from your gross income under section 911 and you don't own a CFC or PFIC, your MAGI is your AGI as reported on Form 1040 or 1040-SR.**" The **Line 13—MAGI Worksheet** adds to AGI **only** (line 2) the Form 2555 foreign-earned-income exclusion and (line 3) CFC/PFIC adjustments. Muni/tax-exempt interest is **not on the worksheet at all** → not added back. For the common W-2 household MAGI **is** AGI, exactly. |
| C2 | The engine's `niit` closure math **is** Form 8960 Part III and can be reused for the absolute (`02` §7) | **CONFIRMED for the *rate/threshold* math; CORRECTED for *NII sourcing*** | The closure `3.8%×max(0, min(NII, MAGI−thr))` is byte-identical to Form 8960 L15–L17. **But the engine's `nii_with` is NOT the absolute Form 8960 NII** — it deliberately omits whole-household taxable interest and non-qualified dividends (they cancel in the with/without delta). The absolute assembly must **re-build NII from line items**, not read `nii_with`. This is the single most important reconciliation finding — see §4. |
| C3 | `se.rs` "already half-built" the 8959 threshold coordination (`02` §7) | **CONFIRMED for Part II (SE side); Part I (wages) is genuinely net-new** | `compute_se_tax` computes Form 8959 **Part II L13** exactly (`addl` field), including the L11 reduced-threshold coordination. It does **not** compute Part I L7 (wages 0.9%) — no such field exists on `SeTaxResult`. §3 verifies the Part II math to the cent and specs the net-new Part I + Part V. |
| C4 | (implicit in `04` §5) a single `w2_ss_wages` / `w2_medicare_wages` scalar suffices | **CORRECTED — two DIFFERENT aggregation rules on an MFJ return** | Form 8959 Part II uses **household-total** Medicare wages (both spouses' box 5) to reduce the SE threshold, but the §1402(b)(1) SS-cap uses **only the SE-earner's own** box 3. `ReturnInputs` must tag each W-2 with an owner (taxpayer/spouse) and derive the two channels differently. New flag — see §3.4. |
| C5 | (implicit) `SeTaxResult.total` can feed the return's SE-tax line | **CORRECTED — `total` bundles the 0.9% addl; the return must UNBUNDLE it** | `se.rs.total = ss + medicare + addl`. On the forms, Schedule SE line 12 / **Sch 2 L4** = `ss + medicare` **only**; the `addl` is a **Form 8959** item → **Sch 2 L11**. Feeding `total` to Sch 2 L4 while also running 8959 double-counts the 0.9%. Double-count trap — see §4.3. |

---

## 1. `derive_tax_profile` — verified to the cent (Worked Example 1, MFJ)

### 1.1 The household (all synthetic)

TY2024, **MFJ**, standard deduction, **no crypto** (crypto is added in §4 to exercise the delta engine).

| Form | Field | Value |
|---|---|---|
| W-2 #1 (taxpayer) | box 1 wages / box 3 SS wages / box 5 Medicare wages / box 2 fed WH | 180,000 / 168,600¹ / 180,000 / 30,000 |
| W-2 #2 (spouse) | box 1 / box 3 / box 5 / box 2 | 90,000 / 90,000 / 90,000 / 12,000 |
| 1099-INT | box 1 interest | 4,000 |
| 1099-DIV | box 1a ordinary div / **box 1b qualified (⊂ 1a)** / box 2a cap-gain distr | 10,000 / **8,000** / 3,000 |

¹ Box 3 is capped at the TY2024 SS wage base $168,600 (`ty2024().ss_wage_base`). Immaterial to §1 (no SE income here); load-bearing in §3.

### 1.2 Derivation arithmetic (crypto excluded throughout — the engine adds crypto on top)

```
1  wages          = 180,000 + 90,000                       = 270,000   → 1040 L1a
2  taxable_int    = 4,000  (+ box3 treasury 0)             =   4,000   → 1040 L2b
3  ord_div        = Σ box1a                                =  10,000   → 1040 L3b   (1a INCLUDES 1b)
4  qual_div       = Σ box1b                                =   8,000   → 1040 L3a   (preferential split ONLY)
5  cap_gain_distr = Σ box2a                                =   3,000   → Sch D L13 → 1040 L7 (LT character)
6  Sch 1 add'l income / adjustments                        =       0
7  AGI = 270,000 + 4,000 + 10,000 + 3,000 − 0             = 287,000   → 1040 L11
       └ qual_div (8,000) is NOT separately added: it is a subset of L3b already in the sum.
       └ cap_gain_distr (3,000) enters ONCE, via Sch D → L7 → total income.
8  deduction = max(std_MFJ 29,200, Sch A 0)                =  29,200   → 1040 L12   (std)
9  qbi                                                     =       0
10 taxable_income = max(0, 287,000 − 29,200 − 0)          = 257,800   → 1040 L15
11 ordinary_taxable_income = max(0, 257,800 − 8,000 − 3,000) = 246,800  ← STRIP THE PREF SLICE ONCE
```

### 1.3 The resulting `TaxProfile` (every field with its exact current engine meaning)

```
filing_status                              = Mfj
ordinary_taxable_income                    = 246,800    (step 11)
magi_excluding_crypto                      = 287,000    (= AGI, step 7 — see C1; §1411 contract holds)
qualified_dividends_and_other_pref_income  =   8,000    (step 4)
other_net_capital_gain                     =   3,000    (step 5 — box 2a, LT character)
w2_ss_wages                                = 168,600    (SE-earner's OWN box 3 — see §3.4; here N/A, no SE)
w2_medicare_wages                          = 270,000    (Σ box 5, household — 8959 Part I/II)
capital_loss_carryforward_in               = {0, 0}
schedule_c_expenses                        =       0
```

### 1.4 "Strip once" — why double-subtracting is the bug (the invariant)

The engine reconstructs `taxable_income` by adding the preferential slice back **exactly once** on top
of the ordinary bottom (`compute.rs:339` `bottom_with`, then `preferential_tax` sits `qd +
preferential_gain` on top). So the derivation must satisfy the **round-trip identity**:

```
taxable_income  ==  ordinary_taxable_income + qual_div + cap_gain_distr
257,800         ==  246,800 + 8,000 + 3,000                               ✓
```

Two ways to break it (the real bugs to KAT):

- **Double-strip on the deduction side.** Subtract `qual_div + cap_gain_distr` from `taxable_income`
  (correct, once) **and also** exclude box 2a from AGI ("it's preferential, keep it out of the ordinary
  base"). Then the $3,000 is removed twice — once from the base and once via the strip — yet the engine
  still adds it back once through `other_net_capital_gain`. Net effect: total income, taxable income,
  **and** NIIT NII/MAGI are all understated by $3,000. Box 2a must be **IN AGI** (via Sch D → L7) **and**
  stripped **once**; it re-enters the tax only through the `other_net_capital_gain` preferential channel.
- **Double-count on the income side.** Add box 1b (qualified, 8,000) to income *in addition to* box 1a
  (10,000, which already includes 1b). AGI is overstated by 8,000. Use **box 1a for the ordinary total,
  box 1b only for the preferential split.**

### 1.5 Box 2a shares the crypto Schedule-D channel — regression-test the coupling

`cap_gain_distr` (box 2a) lands in `TaxProfile.other_net_capital_gain`, the **same** field the engine
nets against crypto Schedule D in `net_1222(crypto_st, crypto_lt, other_net_capital_gain, …)`
(`compute.rs:319`). With the $3,000 here and, say, a crypto LT loss, the box-2a distribution is netted
against crypto capital results **before** the §1(h) stack and §1211 limit apply. Coupling KATs needed:
(a) box 2a alone (no crypto) → LT preferential; (b) box 2a + crypto LT gain → both preferential, summed;
(c) box 2a + crypto LT loss ≥ box 2a → net loss year, §1211 $3,000/$1,500-MFS limit engages. §4.2 works case (b).

### 1.6 Fail-closed guard the derivation must carry (from C1)

`magi_excluding_crypto = AGI` is exact **only** absent a §911 foreign-earned-income exclusion (Form 2555)
and absent CFC/PFIC ownership. Both are out of scope. The derivation must **refuse** (fail-closed
`NotComputable`, per `05` legal posture) if a Form 2555 exclusion or CFC/PFIC input is ever present,
rather than silently apply `MAGI = AGI` when the MAGI worksheet would add the §911 excess.

---

## 2. Form 8960 (NIIT) — absolute, locked

### 2.1 Form structure (2025 Form 8960, transcribed verbatim; identical line map 2024)

```
Part I  Investment Income
 1  Taxable interest
 2  Ordinary dividends
 3  Annuities
 4a Rental/royalty/partnership/S-corp/trade-or-business … / 4b non-§1411 adj / 4c combine 4a+4b
 5a Net gain or loss from disposition of property
 5b Net gain not subject to NIIT / 5c partnership-interest/S-corp-stock adj / 5d combine 5a–5c
 6  CFC/PFIC adjustments
 7  Other modifications
 8  Total investment income = L1 + L2 + L3 + L4c + L5d + L6 + L7
Part II  9a interest exp / 9b state-local-foreign income tax / 9c misc / 9d sum / 10 add'l mod / 11 total
Part III (Individuals)
 12 Net investment income = L8 − L11 (if ≤0, enter -0-)
 13 Modified adjusted gross income (see instructions)
 14 Threshold based on filing status
 15 L13 − L14 (if ≤0, -0-)
 16 smaller of L12 or L15
 17 NIIT = L16 × 3.8% (0.038)   → include on your tax return (Sch 2 L12 → 1040 L23)
```

### 2.2 In-scope Part I NII assembly (whole household)

For the "common W-2 + Bitcoin" scope only lines 1, 2, 5a feed NII (3, 4a–c, 5b–c, 6, 7 = 0; Part II = 0 —
no investment-interest expense modeled):

- **L1 Taxable interest** = Σ 1099-INT box 1 (+ box 3 treasury) **+ crypto-lending interest**
  (§1411(c)(1)(A)(i); the engine's `interest_nii`, `IncomeKind::Interest`, `compute.rs:310`).
- **L2 Ordinary dividends** = Σ 1099-DIV **box 1a** — the **full** ordinary-dividend total (qualified
  *and* non-qualified). The qualified/ordinary split affects only the §1(h) *rate*, never §1411
  *inclusion*: **all** dividends are NII.
- **L5a Net gain from disposition** = **1040 L7** (2024 Instr. Line 5a, verbatim: "Calculate and enter
  the amount of net gain or loss … by combining … **Form 1040 or 1040-SR, line 7**, and Schedule 1 …").
  Line 7 **is** the Schedule D net **including the crypto 8949/Schedule D result** + box 2a. A net capital
  **loss** enters L5a limited to the §1211 −$3,000/−$1,500-MFS figure that reaches L7 (matches
  §1.1411-4(d) and the engine's `− with.loss_deduction` term, `compute.rs:360`).
- **L8 NII** = L1 + L2 + L5d.

### 2.3 L13 MAGI (locked to primary source)

**L13 = AGI** (2024 Instr., *Line 13—MAGI*, verbatim quote in C1; MAGI Worksheet adds only §911 + CFC/PFIC,
neither in scope). Tax-exempt interest / exempt-interest dividends (INT box 8, DIV box 12) are captured for
the PDF but **excluded** from L13. **L14 threshold** = `niit_threshold(status)` (`tables.rs:190`, statutory:
MFJ/QSS 250,000 · Single/HoH 200,000 · MFS 125,000 — matches Form 8960 exactly).

### 2.4 The engine closure IS Form 8960 L15–L17 (identity)

`compute.rs:369` `niit = |nii, magi| 3.8% × max(0, min(nii, max(0, magi − thr)))` ≡ L15 (`magi−thr`,
floored) → L16 (`min`) → L17 (`×0.038`). Same `NIIT_RATE`, same `niit_threshold`, same clamps. **Reuse the
formula; supply whole-household NII (from §2.2) and MAGI (= AGI).** Do **not** modify `compute.rs`.

### 2.5 Worked absolute NIIT — Example 1 (no crypto)

```
L1 = 4,000 · L2 = 10,000 · L5a = 3,000 → L8 NII = 17,000 · L11 = 0 → L12 = 17,000
L13 MAGI = 287,000 · L14 = 250,000 · L15 = 37,000 · L16 = min(17,000, 37,000) = 17,000
L17 = 0.038 × 17,000 = 646.00   → Sch 2 L12 → 1040 L23
```
Engine crypto-**delta** NIIT for this profile = **0.00** (no crypto → with == without). Both correct: the
$646 absolute is the household NIIT that lands on the return; the $0 delta says crypto added nothing.
**Surfacing recommendation:** compute the absolute in the new assembly layer and put **$646** on Sch 2
L12; the engine's `TaxResult.niit` delta is a *planning* number, not a return line — keep them distinct.

*(Note: Example 1's wages of $270,000 > $250,000 also trigger Form 8959 Part I: L7 = 0.9% × 20,000 =
**$180.00**. §3 works the full both-sides 8959 on Example 2.)*

---

## 3. Form 8959 (Additional Medicare Tax) — absolute, locked

### 3.1 Form structure (2025 Form 8959, transcribed verbatim; identical map 2024)

```
Part I  (wages)
 1 Medicare wages (Σ W-2 box 5)  2 Form 4137 tips  3 Form 8919 wages  4 = L1+L2+L3
 5 threshold (MFJ 250,000 / MFS 125,000 / Single·HoH·QSS 200,000)
 6 L4 − L5 (if ≤0, -0-)          7 = L6 × 0.9%            → go to Part II
Part II (self-employment)
 8 SE income from Schedule SE Part I, line 6 (if loss, -0-)     9 threshold (same amounts)
10 amount from line 4 (= Part I Medicare wages)                11 L9 − L10 (if ≤0, -0-)  ← reduced threshold
12 L8 − L11 (if ≤0, -0-)                                       13 = L12 × 0.9%          → go to Part III
Part III (RRTA) 14–17  — OUT OF SCOPE (= 0)
Part IV  18 = L7 + L13 + L17     → Schedule 2 (Form 1040) line 11
Part V  (withholding reconciliation)
19 Medicare tax withheld (Σ W-2 box 6)     20 amount from line 1 (Σ box 5)
21 L20 × 1.45%  (regular Medicare withholding)
22 L19 − L21 (if ≤0, -0-)  = Additional Medicare Tax withheld on wages
23 RRTA add'l withholding (box 14)  = 0
24 = L22 + L23   → include with fed income-tax withholding on 1040 line 25c
```

Threshold coordination confirmed verbatim (2025 Instr., Part II example): "**The $130,000 of Kathleen's
wages reduces Liam's self-employment income threshold to $120,000 ($250,000 threshold minus the $130,000
of wages).**" → the spouse's Medicare wages **do** reduce the SE-earner's threshold (joint-form
aggregation). Note **L8 uses Schedule SE line 6** — the SE base **after** the ×92.35% factor (= `se.rs`
`base`), *not* gross Schedule C.

### 3.2 Worked both-sides 8959 — Example 2 (MFJ; wage AND SE)

Household: taxpayer W-2 box 5 = 220,000 (box 3 = 168,600 capped, box 6 = 3,370); spouse W-2 box 5 =
60,000 (box 3 = 60,000, box 6 = 870). Crypto **business** mining, Schedule C net = 60,000 → Schedule SE
line 6 base = `round_cents(60,000 × 0.9235)` = **55,410.00**.

```
Part I   L1 = 220,000 + 60,000 = 280,000 · L4 = 280,000 · L5 = 250,000
         L6 = 30,000 · L7 = 0.9% × 30,000 = 270.00                            (NET-NEW: se.rs has no L7)
Part II  L8 = 55,410 · L9 = 250,000 · L10 = 280,000
         L11 = max(0, 250,000 − 280,000) = 0        ← threshold fully consumed by household wages
         L12 = max(0, 55,410 − 0) = 55,410 · L13 = 0.9% × 55,410 = 498.69
Part IV  L18 = 270.00 + 498.69 + 0 = 768.69         → Sch 2 L11 → 1040 L23
Part V   L19 = 3,370 + 870 = 4,240 · L20 = 280,000 · L21 = 1.45% × 280,000 = 4,060.00
         L22 = max(0, 4,240 − 4,060) = 180.00 · L23 = 0 · L24 = 180.00   → 1040 L25c
```
(The $180 L22 is the employer-withheld 0.9% on the taxpayer's wages over $200,000 — employers apply the
flat $200k trigger per employer regardless of filing status; it returns as a **payment** on L25c, distinct
from the $768.69 **tax** on Sch 2 L11. **Do not collapse the two stages** — `01` §6.)

### 3.3 `se.rs` verification — Part II matches to the cent (C3)

`compute_se_tax(status=Mfj, w2_medicare_wages=280,000, w2_ss_wages=168,600, gross_se=60,000, expenses=0)`
(`se.rs:99`):

```
net_se = 60,000 · base = round_cents(60,000 × 0.9235) = 55,410.00
addl_threshold = max(0, se_addl_medicare_threshold(Mfj) − w2_medicare_wages)
               = max(0, 250,000 − 280,000) = 0                    ≡ Form 8959 L11
over = max(0, 55,410 − 0) = 55,410                                ≡ Form 8959 L12
addl = round_cents(0.009 × 55,410) = 498.69                       ≡ Form 8959 L13   ✓ exact
ss   = 0.124 × min(55,410, max(0, 168,600 − 168,600)=0) = 0.00    (SS cap already used by W-2 wages)
medicare = round_cents(0.029 × 55,410) = 1,606.89
total = 0 + 1,606.89 + 498.69 = 2,105.58
```
**`se.rs.addl` (498.69) = Form 8959 Part II L13 exactly, with the reduced-threshold coordination.**
Confirmed: Part II (the SE side) is already built. **Net-new for the absolute 8959:** Part I L1–L7
(wages), Part IV L18 sum, Part V L19–L24 (withholding). None of those exist in `se.rs`.

### 3.4 The two W-2 channels have DIFFERENT aggregation (new flag, C4)

- **8959 Part II threshold reduction** (L10 = L4 = Σ box 5) uses **household-total** Medicare wages →
  `w2_medicare_wages` passed to `se.rs` = **both spouses'** box 5 (280,000 here). Correct as coded.
- **§1402(b)(1) SS cap** (`ss_cap = max(0, ss_wage_base − w2_ss_wages)`) is **per-individual** — each
  spouse files a **separate** Schedule SE. `w2_ss_wages` must be the **SE-earner's OWN** box 3 (168,600),
  **not** the household sum (228,600). This example doesn't expose the divergence (both cap to 0), but if
  the SE-earner had low W-2 SS wages and the spouse high, summing would wrongly zero the SS cap and
  understate SE tax. **`ReturnInputs` must tag each W-2 with an owner (taxpayer/spouse)** and derive
  `w2_ss_wages` from the SE-earner's W-2s only, while `w2_medicare_wages` sums the household.

---

## 4. Reconciliation with the frozen delta engine

### 4.1 How they coexist without touching `compute.rs`

Two computations answer two questions and run side by side:

| | Question | Engine (`compute.rs`, FROZEN) | New absolute assembly |
|---|---|---|---|
| NIIT | how much NIIT does *crypto* cause? | `TaxResult.niit` = `niit_with − niit_without` (delta) | Form 8960 L17 from whole-household line items |
| Add'l Medicare | ditto | (not currently in `TaxResult`) | Form 8959 L18 from Σ box 5 + SE base |

The absolute 8960/8959 are **new code in the assembly layer** that **reuse the statutory primitives**
(`NIIT_RATE`, `niit_threshold`, `se_addl_medicare_threshold`, `SE_RATE_ADDL_MEDICARE`, and the closure
shape) but are fed **whole-household** inputs. `compute.rs`, `TaxProfile`, and the ~80 constructors stay
byte-for-byte frozen (the additive design of `04` §5). The engine keeps emitting its crypto delta for the
what-if / optimizer paths.

### 4.2 The central finding: the engine's NII is NOT the absolute NII (do not reuse `nii_with`)

Add crypto to Example 1: **crypto LT gain $40,000** + **crypto-lending interest $2,000**. Engine
(`compute.rs`) with the §1.3 profile:

```
with  = net_1222(0, 40,000, other=3,000, …) → preferential_gain = 43,000, loss_deduction = 0
crypto_ord = 2,000 (lending interest) · interest_nii = 2,000
nii_with  = qd 8,000 + ord_gain 0 + pref_gain 43,000 − loss 0 + interest 2,000 = 53,000
magi_with = 287,000 + [(43,000 − 3,000) + 2,000] = 329,000
niit_with = 0.038 × min(53,000, 329,000−250,000=79,000) = 0.038 × 53,000 = 2,014.00
niit_without = 0.038 × min(11,000, 37,000)             = 0.038 × 11,000 =   418.00
TaxResult.niit (DELTA) = 2,014.00 − 418.00 = 1,596.00
```

**Absolute Form 8960** (whole household, same crypto):
```
L1 = 4,000 + 2,000 = 6,000 · L2 = 10,000 · L5a = 40,000 + 3,000 = 43,000 → L8 NII = 59,000
L13 MAGI = 329,000 · L15 = 79,000 · L16 = min(59,000, 79,000) = 59,000
L17 = 0.038 × 59,000 = 2,242.00
```

`nii_with` = **53,000**, absolute NII = **59,000**. The **$6,000 gap** = non-crypto **taxable interest
($4,000)** + **non-qualified dividends ($2,000** = box1a 10,000 − box1b 8,000)**. The engine's NII carries
only the §1(h)-preferential `qd` + capital gains + crypto interest and **omits** ordinary interest and
non-qualified dividends because, for a *delta*, they are constant and cancel. For the *absolute* return
they are NII and must be included. **⇒ The assembly must assemble NII from line items (§2.2); it must not
read `TaxResult`'s internal `nii_with`.** (Interestingly, `magi_with = 329,000` **does** equal the
absolute L13 MAGI, because AGI already includes all interest/dividends — so MAGI *is* reusable while NII
is not. State this asymmetry in the spec.)

### 4.3 Invariants to KAT (the coexistence contract)

1. **Absolute reduces to the delta when non-crypto vanishes.** Set non-crypto interest/dividends/cap-gain
   = 0 and `magi_excluding_crypto` = 0, `qd` = 0, `other_net_capital_gain` = 0. Then absolute-8960 NII =
   crypto interest + crypto Sch D = engine `nii_with`; absolute MAGI = engine `magi_with`; `niit_without`
   = 0 ⇒ **absolute NIIT == `TaxResult.niit`**. Same for 8959 (no W-2 wages ⇒ Part I L7 = 0, Part II L13 =
   `se.rs.addl`). A regression KAT must pin this equality.
2. **Absolute ≥ delta, always** (crypto is a subset of household NII/wages). If assembly < engine delta,
   something is mis-summed.
3. **Unbundle the SE 0.9% (C5 double-count trap).** Route `se.rs.addl` → Form 8959 L13 → **Sch 2 L11**.
   Schedule SE line 12 / **Sch 2 L4** must use **`se.rs.ss + se.rs.medicare` only** (NOT `se.rs.total`).
   Feeding `total` to L4 while also filing 8959 taxes the 0.9% twice. A KAT must assert Sch 2 L4 excludes
   `addl` and Sch 2 L11 includes it exactly once.
4. **Two-stage 8959.** Part IV L18 → Sch 2 L11 (tax); Part V L24 → 1040 L25c (withholding payment). Never
   net them into one line; both appear on the 1040 (one raises tax, one raises payments).

### 4.4 Loudness / source-of-truth (ties to `04` §5.3)

When a year has a stored `ReturnInputs`, the absolute 8960/8959 numbers on the PDF come from the assembly,
while the engine's delta feeds only planning/what-if. `report`/TUI must label which is which so a reviewer
never mistakes the crypto delta ($1,596 NIIT) for the return line ($2,242 NIIT). This is the "wrong number
presented as authoritative" cardinal-sin surface (`types.rs:114`) and must be explicit in the spec.

---

## Appendix — figures cross-checked against the actual bundled table

`ty2024()` (`crates/btctax-adapters/src/tax_tables.rs:255-336`): MFJ ordinary breakpoints
23,200 / 94,300 / 201,050 / 383,900 / 487,450 / 731,200; §1(h) `max_zero` 94,050, `max_fifteen` 583,750;
`ss_wage_base` 168,600. Std deduction MFJ 2024 = $29,200 (`02` §1, Rev. Proc. 2023-34; **net-new bundling**
— not yet in the table). Example-1 regular tax context (out of this report's strict scope, shown for the
bottom line): `ordinary_tax_on(246,800)` = 45,317.00; `preferential_tax(bp, 246,800, 11,000)` at_15 =
11,000 → 1,650.00; QDCGT L16 = 46,967.00. All hand-computed values above match the frozen primitives.
```
