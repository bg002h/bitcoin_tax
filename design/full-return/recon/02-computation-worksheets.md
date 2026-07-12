# Recon 02 — Computation Worksheets & Math the Full-Return Engine Must Add

**Agent:** Recon 2 of 5 (first-pass; Fable sweeps later). **Scope:** the computation
math to grow `btctax` from a crypto-delta engine into a **complete common W-2 household
1040** (absolute tax, PDF fill). **Years:** TY2024 + TY2025.

**Bottom line up front.** The existing core fns (`ordinary_tax_on`, `preferential_tax`/
`PrefSplit`, `net_1222`, the NIIT closure, `se.rs`) already implement *every rate-application
primitive* the return needs — they are the exact building blocks of the IRS Qualified
Dividends & Capital Gain Tax Worksheet. The work is **(a)** stop subtracting the "without
crypto" scenario and emit an **absolute** number, **(b)** bundle **standard deduction** +
**Schedule A** limits (net-new; no home in the engine today), and **(c)** two TY2025 OBBBA
overrides (standard deduction *and* SALT cap) that make 2025 ≠ Rev. Proc. 2024-40.

Two **decision-forcing correctness traps** are flagged up top because they change bundled
data and matching behavior:

1. **TY2025 is not Rev. Proc. 2024-40 for deductions.** OBBBA (Pub. L. 119-21, 7/4/2025)
   *retroactively* raised the 2025 **standard deduction** to **$15,750 / $31,500 / $23,625**
   (single/MFJ/HoH — NOT the Rev. Proc.'s $15,000/$30,000/$22,500) **and** the **SALT cap** to
   **$40,000** ($20,000 MFS, phased down above $500k MAGI — NOT $10,000). The engine's own
   `tax_tables.rs` header already knows OBBBA "raised the 2025 standard deduction"; that dormant
   note now becomes load-bearing.
2. **Tax Table vs. exact formula.** The IRS **requires** the binned **Tax Table** for taxable
   income **< $100,000**; the engine's `ordinary_tax_on` is the exact marginal formula (= the
   **Tax Computation Worksheet**, used only at ≥ $100k). They differ by a few dollars. A return
   that must match the IRS-official Form 1040 line 16 has to decide whether to replicate the
   $50-bin midpoint (see §6).

---

## 1. Standard Deduction (net-new — nothing bundled today)

`ordinary_taxable_income` arrives **post-deduction** today, so no standard-deduction amount
exists anywhere in the workspace. A full return computes taxable income itself, so these must
be bundled per year. **Recommendation: add to the per-year `TaxTable`** in
`btctax-adapters/src/tax_tables.rs` (§63(c)(4)/§1(f) **inflation-indexed** → same home as
`gift_annual_exclusion`/`ss_wage_base`; the aged/blind and dependent-floor amounts are indexed
under §63(f)/§63(c)(5) too). The `+$450` dependent add-on is the one piece that has been
constant — it can be a statutory const in `core/tax/tables.rs`, but keeping it in the table is
simpler.

### Basic standard deduction (§63(c)(2))

| Filing status | **TY2024** (Rev. Proc. 2023-34 §.15) | **TY2025** (OBBBA §70102) | *(TY2025 Rev. Proc. 2024-40 — SUPERSEDED)* |
|---|---|---|---|
| MFJ / Surviving spouse | $29,200 | **$31,500** | *$30,000* |
| Head of household | $21,900 | **$23,625** | *$22,500* |
| Single | $14,600 | **$15,750** | *$15,000* |
| MFS | $14,600 | **$15,750** | *$15,000* |

> **TY2025 must use the OBBBA column.** Rev. Proc. 2024-40 §.15 (the number the engine would
> "naturally" transcribe, matching the bracket source) is **wrong for the filed return**. Cite
> OBBBA Pub. L. 119-21 §70102 + IRS 2025 Form 1040 instructions; corroborated by Tax Foundation
> / H&R Block. (First-pass caveat: confirm the exact statutory dollar amounts against OBBBA
> engrossed text / final 2025 Pub. 501 before bundling.)

### Additional standard deduction — aged (65+) and/or blind (§63(f))

Added **per condition** (65+ is one, blind is one → up to 2 per taxpayer, up to 4 on an MFJ
return with two elderly-and-blind spouses).

| | TY2024 | TY2025 |
|---|---|---|
| Married (MFJ/MFS/QSS), per box checked | **$1,550** | **$1,600** |
| Unmarried & not a surviving spouse (Single/HoH), per box | **$1,950** | **$2,000** |

Source: Rev. Proc. 2023-34 §.15(3) / Rev. Proc. 2024-40 §.15(3) (both read verbatim). **Total
std deduction = basic + Σ additional.** OBBBA did **not** disturb the §63(f) amounts.

### Dependent's standard deduction (§63(c)(5))

`min( regular_std_for_status , max( floor , earned_income + $450 ) )` where the **floor** =
**$1,300 (TY2024)** / **$1,350 (TY2025)** (Rev. Proc. §.15(2), verbatim). A dependent who is
also aged/blind **still adds** the §63(f) amount on top.

### Adjacency (out of scope — note only)

OBBBA **§70103** adds a **$6,000 "senior deduction"** per individual 65+ (2025–2028), *on top*
of §63(f), phasing out at 6% of MAGI over **$75,000 single / $150,000 joint**. Available whether
itemizing or not. Flag for a later cycle; do **not** build now.

---

## 2. Schedule A Itemized Deductions (net-new aggregation; charitable partly exists)

All confirmed from the **2024 Instructions for Schedule A** (irs.gov/pub/irs-prior/
i1040sca--2024.pdf, read directly).

### 2a. Medical & dental (line 1–4)
Deduct only the excess over **7.5% of AGI** (§213(a), permanent). `deduction = max(0,
medical − 0.075 × AGI)`. **Depends on AGI**, so Schedule A cannot be finalized until AGI is
known → ordering constraint (§3 below and recon-agent-1's AGI assembly).

### 2b. SALT (line 5a–5e) — **the big TY2024≠TY2025 fork**
Sum of state/local income-or-sales tax (5a) + real-estate tax (5b) + personal-property tax
(5c), **capped**:

| | TY2024 (§164(b)(6), TCJA) | TY2025 (OBBBA §70120) |
|---|---|---|
| Cap (most) | **$10,000** | **$40,000** |
| Cap (MFS) | **$5,000** | **$20,000** |
| Phase-down | none | reduce by **30% of MAGI over $500,000** ($250k MFS), **floored at $10,000** ($5,000 MFS) |

TY2025 formula: `cap = max(10_000, 40_000 − 0.30 × max(0, MAGI − 500_000))` (halve both
constants for MFS). Indexed +1%/yr 2026–2029, reverts to $10k in 2030. Cite OBBBA §70120 +
2025 Schedule A instructions (first-pass; verify the MAGI definition & rounding against final
IRS text).

### 2c. Home-mortgage interest (line 8a–8e)
Interest on **acquisition debt** (used to buy/build/substantially improve the home securing
the loan). Deductible interest is limited by the debt principal, tiered by origination date:

| Loan origination | Acquisition-debt limit | MFS |
|---|---|---|
| After **12/15/2017** | **$750,000** | $375,000 |
| **10/14/1987 – 12/15/2017** (grandfathered) | **$1,000,000** | $500,000 |
| On/before **10/13/1987** | fully deductible (no cap) | — |

Home-equity interest **not** used to buy/build/improve is **not** deductible (TCJA, through
2025; OBBBA made this permanent). The precise limit needs a Pub. 936 "average balance"
worksheet; a common-case engine can take deductible mortgage interest as a **user input**
(reported on Form 1098) and skip the balance proration. Cite Sch. A instr. "Limits on home
mortgage interest" + §163(h).

### 2d. Charitable (lines 11–14) + AGI ceilings + 5-year carryover
Schedule A lines: **11** cash/check, **12** other-than-cash (property), **13** carryover from
prior year, **14** total. **AGI-percentage ceilings** (the Schedule A instructions only print
the *trigger* percentages; the ceilings themselves live in **§170(b)** / **Pub. 526**):

| Gift type → donee | AGI ceiling | Cite |
|---|---|---|
| **Cash** → public charity (50%-org) | **60%** | §170(b)(1)(G) |
| **Capital-gain property** (FMV) → 50%-org | **30%** | §170(b)(1)(C)(i) |
| Ordinary-income property / cash → 30%-org (e.g., some private fdns) | **30%** | §170(b)(1)(B) |
| **Capital-gain property** → non-50%-org | **20%** | §170(b)(1)(D)(i) |

Excess over the ceiling **carries forward 5 years** (§170(d)(1); "you have 5 years to use
contributions limited in the earlier year," Sch. A instr. line 13). Ceilings apply in a
statutory ordering (60% cash first, then 50%-org non-cash, etc.) — a small net-new "charitable
limitation engine."

### 2e. How the existing crypto Form 8283 deduction plugs in
`btctax` already computes the crypto **noncash charitable deduction** with **§170(e)** basis
reduction and fills **Form 8283** (SPEC_p2a_170e_deduction.md, SPEC_p2c_form8283_709.md;
`QUALIFIED_APPRAISAL_THRESHOLD` = $5,000 already in `core/tax/tables.rs`). That dollar amount
is exactly a **Schedule A line 12** entry:
- **BTC held > 1 yr** = long-term **capital-gain property** → FMV deduction, **30%-of-AGI**
  ceiling (row 2 above).
- **BTC held ≤ 1 yr** = ordinary-income property → §170(e)(1)(A) caps the deduction at **basis**,
  **60%/50%** ceiling.
- Noncash > $500 → **Form 8283** required (already produced); > $5,000 → qualified appraisal
  (already gated). So the plug-in is: **feed the engine's computed 8283 deduction into the
  line-12 charitable total, then run it through the §170(b) ceiling + 5-yr carryover.**

---

## 3. Standard-vs-Itemized decision (net-new; trivial)
Take the **larger** of (total standard deduction from §1) and (Schedule A line 17 total).
`deduction = max(std_total, itemized_total)`; taxable income = `max(0, AGI − deduction − QBI)`.
**MFS coupling rule (§63(c)(6)(A)):** if one spouse itemizes, the other's standard deduction is
**$0** (both must itemize or both take standard). Model as a per-return flag; the engine can't
silently pick the larger for an MFS filer whose spouse itemized. Also honor the taxpayer's
right to **elect** itemized even when smaller (§63(e); Sch. A line 18 checkbox) — e.g., to match
a state return.

---

## 4. Qualified Dividends & Capital Gain Tax Worksheet — the core mapping

**This is where the existing engine already does the hard part.** Full 25-line 2024 worksheet
(Form 1040 instructions p. 36, transcribed verbatim) with the exact engine mapping. Notation:
`bp = LtcgBreakpoints{max_zero, max_fifteen}`, `sched = OrdinarySchedule`.

| WS line | Operation | Engine equivalent |
|---|---|---|
| **1** | Taxable income (TI) | `taxable_income` (net-new AGI−deduction assembly) |
| **2** | Qualified dividends (1040 3a) | **1099-DIV box 1b** (net-new income sourcing) |
| **3** | net capital gain = `min(Sch D 15, 16)`, ≥0 | `net_1222(...).preferential_gain` |
| **4** | L2 + L3 = total preferential `pref` | `qd + preferential_gain` |
| **5** | L1 − L4 = ordinary bottom `B`, ≥0 | `bottom_with` (= TI − pref) |
| **6** | 0% breakpoint | `bp.max_zero` |
| **7** | min(L1, L6) | — |
| **8** | min(L5, L7) | — |
| **9** | L7 − L8 = **taxed at 0%** | **`PrefSplit.at_0`** |
| **10** | min(L1, L4) | `pref` |
| **11** | = L9 | — |
| **12** | L10 − L11 | — |
| **13** | 15% breakpoint | `bp.max_fifteen` |
| **14** | min(L1, L13) | — |
| **15** | L5 + L9 | — |
| **16** | max(0, L14 − L15) | — |
| **17** | min(L12, L16) = **taxed at 15%** | **`PrefSplit.at_15`** |
| **18** | 0.15 × L17 | part of `PrefSplit.tax` |
| **19** | L9 + L17 | — |
| **20** | L10 − L19 = **taxed at 20%** | **`PrefSplit.at_20`** |
| **21** | 0.20 × L20 | part of `PrefSplit.tax` |
| **22** | tax on **L5** (Tax Table/**TCW**) | **`ordinary_tax_on(sched, B)`** |
| **23** | L18 + L21 + L22 | `ordinary_tax_on(sched,B) + PrefSplit.tax` |
| **24** | tax on **L1** (Tax Table/**TCW**) | `ordinary_tax_on(sched, TI)` |
| **25** | **min(L23, L24)** → 1040 line 16 | *(see divergence b)* |

**Exact identity:** `preferential_tax(bp, bottom=B, pref=qd+preferential_gain)` returns
`at_0=L9, at_15=L17, at_20=L20, tax = 0.15·L17 + 0.20·L20 = L18+L21`. So the whole preferential
block (lines 6–21) **is** one existing call, and line 16 = `min( ordinary_tax_on(B) +
PrefSplit.tax , ordinary_tax_on(TI) )`. The engine's `compute.rs` already builds precisely
`ord_with + pref_with` = worksheet L23.

**Where they diverge (the net-new deltas):**

- **(a) Absolute, not delta.** Today `compute.rs` runs L23 for a *with-crypto* and a
  *without-crypto* scenario and **subtracts** them (`total_federal_tax_attributable`). The
  return needs **L25 itself**. Net-new: a thin "absolute assembly" reusing the same two fns
  without the subtraction. All the arithmetic already exists and is cent-exact.
- **(b) The min(L23, L24) / line-24 comparison is absent.** The delta engine never computes L24
  because it cancels. For an ordinary return **L23 ≤ L24 always** (each preferential rate ≤ the
  ordinary rate on the same slice, since the 20% breakpoint sits below the top ordinary bracket),
  so the min is **non-binding in the common case** — but must be added for exact form fidelity
  (it *can* bind with a §1(h) "capital gain excess"/Form 2555 interaction, out of scope).
- **(c) QD sourcing.** `qualified_dividends_and_other_pref_income` is an opaque profile scalar
  today; the return must source **QD from 1099-DIV box 1b** and **net LT gain from Schedule D**
  (computed via `net_1222`), not user-entered. Ordinary dividends (box 1a) and taxable interest
  (1099-INT / Schedule B) land in the **ordinary** bottom `B`.
- **(d) Tax Table binning on L22 & L24** — see §6. Note the `< $100k` test is applied to **L5 and
  L1 *separately***, so one line can be table-based and the other TCW-based inside one worksheet.

Alignment already correct: `preferential_tax` clamps `bottom<0→0` and `pref≤0→0` (matches the
worksheet "if zero or less, enter -0-"); a net **capital-loss** year yields `preferential_gain=0`
but QD still flows preferential (matches L3=0, L2>0); the §1211 **$3,000** loss reduces `B`
(matches the negative on 1040 line 7). `net_1222` cross-netting reproduces the `min(Sch D 15,16)`
subtlety exactly.

---

## 5. Schedule D Tax Worksheet — OUT OF SCOPE (confirmed), with the trigger

The **Schedule D Tax Worksheet** replaces the QDCGT worksheet **only** when **Schedule D line 18
> 0 (28%-rate gain)** *or* **line 19 > 0 (unrecaptured §1250 gain)** — i.e., you must use it if
you have **collectibles gain / §1202 QSBS exclusion** (28% rate, §1(h)(4)–(5)) or **depreciated
real-property recapture** (25% rate, §1(h)(1)(E)). Confirmed: *"If lines 18 and 19 of Schedule D
are both zero or blank and you are not filing Form 4952, complete the Qualified Dividends and
Capital Gain Tax Worksheet; if not, complete the Schedule D Tax Worksheet"* (2025 Sch. D instr.).

**For the common W-2 + Bitcoin household: both are always $0.** Bitcoin is not a collectible for
§1(h) (it is property/§1221, not §408(m)) and generates no §1250 gain. → **QDCGT worksheet
always applies; Schedule D Tax Worksheet is out of scope.** Trigger that would pull it in: user
enters a K-1/1099-DIV with 28%-rate or unrecaptured-1250 amounts, or sells collectibles — none
of which the Bitcoin engine produces. Recommend a **guard**: if any 28%/§1250 input is ever
present, **refuse** (`NotComputable`) rather than silently run the wrong worksheet.

---

## 6. Tax Tables vs. Tax Computation Worksheet — the $100,000 matching decision

Confirmed verbatim (2024 Form 1040 instr. p. 33 & the QDCGT worksheet L22/L24):
- **Taxable income < $100,000 → MUST use the Tax Table.**
- **≥ $100,000 → use the Tax Computation Worksheet (TCW).**

**The TCW is the exact marginal formula.** Read from p. 76, each TCW row is `taxable × rate −
subtraction` (e.g., single $100,000–$100,525: `×22% − $4,947.00`). That is **algebraically
identical** to `ordinary_tax_on`. → **At ≥ $100k the engine already matches the IRS to the cent.**

**The Tax Table bins and can differ by a few dollars.** Read from p. 74, rows are **$50-wide**
("84,000 / 84,050"), and the tax is computed on the **bin midpoint**, rounded to whole dollars.
Worked check: single at midpoint $84,025 → exact formula = $13,538.50 → table prints **$13,539**
(matches). A taxpayer at $84,010 owes an *exact* $13,535.20 but the **Tax Table says $13,539** —
a **~$4 difference** the IRS treats as the official number.

> **DECISION (flag for spec):** For taxable income < $100k, `ordinary_tax_on` does **not**
> reproduce the official Form 1040 line 16 exactly (off by up to ≈ $25 × top-rate ≈ **$8–9**,
> usually a few dollars). Options: **(A)** replicate the $50-bin midpoint + whole-dollar rounding
> when TI < $100k (recommended for a *filed-return* product — mismatches can trigger IRS notices),
> applied independently to QDCGT L22 and L24; or **(B)** keep the exact formula and disclose the
> "may differ by a few dollars from the IRS Tax Table" caveat. **Bin edges:** $50 for income ≥
> $3,000; $25 below $3,000; special small bins under $25 (verify the low-income bins if the
> product supports very low incomes — irrelevant for a W-2 household).

---

## 7. Additional Medicare Tax (8959) & NIIT (8960) — absolute vs. today's delta

### Form 8960 — NIIT (§1411)
The engine's `niit` closure already **is** Form 8960 Part III math:
`NIIT = 3.8% × max(0, min( NII , MAGI − threshold ))` = form lines 13–17
(L13 MAGI, L14 threshold, L15 = L13−L14, L16 = min(L12 NII, L15), **L17 = 0.038 × L16**).
`NIIT_RATE` (0.038) and `niit_threshold()` are already correct & statutory:
**MFJ/QSS $250,000 · Single/HoH $200,000 · MFS $125,000** (not indexed). **Net-new = surface the
absolute** (drop the with/without subtraction) and build **Part I NII** from the income side
(taxable interest + dividends + net capital gain + crypto-lending interest) instead of the
`magi_excluding_crypto` scalar.

### Form 8959 — Additional Medicare Tax (§1401(b)(2)/§3101(b)(2))
0.9% over the **same** thresholds ($200k/$250k/$125k), already in `se.rs`
(`se_addl_medicare_threshold`, `SE_RATE_ADDL_MEDICARE = 0.009`). Form 8959 has 4 parts; for a
W-2 household the missing piece is **Part I (wages)**: `0.9% × max(0, Medicare wages(W-2 box 5) −
threshold)`. **Threshold coordination (already half-built):** wages consume the threshold first
(Part I uses the full threshold); SE income uses **threshold − Medicare wages** (Part II) — which
is exactly what `w2_medicare_wages` already does in `se.rs`. **Net-new = add Part I (wages) +
absolute total (Part IV line 18 → Schedule 2).** RRTA (Part III) out of scope.

Both taxes flow to **Schedule 2** (8960→line 12, 8959→line 11) → 1040 line 23 → total tax.

---

## 8. Child Tax Credit — adjacency only (do NOT spec)

Near-universal for households but **out of the chosen scope**. **Schedule 8812.** Briefly:
**$2,000/qualifying child < 17** (TY2024; OBBBA raised to **$2,200** for 2025+, indexed),
refundable **ACTC up to $1,700** (= 15% × earned income over $2,500). **$500** nonrefundable
"credit for other dependents." Phase-out: **−$50 per $1,000 of MAGI over $400,000 MFJ /
$200,000 others.** It is a **credit** (reduces tax after line 16 / line 22), so it slots cleanly
into a later 1040-line-22/Schedule-3/Schedule-8812 assembly without touching the taxable-income
math above. Flag for a follow-on cycle.

---

## Reuse-existing-fn vs. net-new — summary table

| Computation | Existing fn / data | Reuse or net-new |
|---|---|---|
| Ordinary marginal tax (= TCW, TI ≥ $100k) | `ordinary_tax_on()` | **Reuse as-is** |
| §1(h) 0/15/20 stacking (QDCGT L6–L21) | `preferential_tax()` → `PrefSplit` | **Reuse as-is** |
| §1222 netting + §1211 $3k limit + §1212 carryforward | `net_1222()` → `CapNet` | **Reuse as-is** |
| NIIT / Form 8960 (3.8%×min(NII, MAGI−thr)) | `niit` closure + `niit_threshold()` + `NIIT_RATE` | **Reuse math**; net-new absolute + Part-I NII sourcing |
| Add'l Medicare / 8959 **SE** side + threshold coord. | `se.rs` + `se_addl_medicare_threshold` + `SE_RATE_ADDL_MEDICARE` | **Reuse**; net-new **Part I (wages)** + absolute total |
| SE tax (§1401) | `se.rs` | **Reuse as-is** |
| Crypto noncash charitable amount + §170(e) + Form 8283 | SPEC_p2a/p2c; `QUALIFIED_APPRAISAL_THRESHOLD` | **Reuse**; feed into Sch. A line 12 |
| Per-year indexed table home | `TaxTable` in `tax_tables.rs` | **Reuse struct**; add std-deduction fields |
| **QDCGT L25 absolute assembly** (ord(B)+PrefSplit.tax, min vs ord(TI)) | — (engine only emits a delta) | **Net-new** thin wrapper |
| **Standard deduction** (basic + §63(f) aged/blind + dependent) | — (not bundled) | **Net-new** data (per-year) + selection |
| **Schedule A** (medical 7.5%, SALT cap+phasedown, mortgage, charitable §170(b) ceilings + 5-yr carryover) | partial (charitable) | **Net-new** aggregation + AGI-limit engine |
| **Std-vs-itemized** decision (+ MFS coupling) | — | **Net-new** (trivial) |
| **Tax Table binning** (TI < $100k, $50 midpoint) | — (engine = exact formula) | **Net-new IF fidelity chosen (§6)** |
| **AGI / taxable-income / total-tax** 1040-line assembly | — (profile hands post-deduction scalars) | **Net-new** (recon-1 territory) |
| QD/interest/dividend income sourcing (1099-INT/DIV, Sch. B) | — (opaque profile scalars) | **Net-new** income side |
| Schedule D Tax Worksheet (28% / §1250) | — | **Out of scope** (guard/refuse) |
| Child Tax Credit / Schedule 8812 | — | **Out of scope** (adjacency) |

---

## Primary sources (all read directly unless noted)

- **Rev. Proc. 2023-34** (TY2024) §.15 — std deduction basic/aged-blind/dependent
  (irs.gov/pub/irs-drop/rp-23-34.pdf, read verbatim).
- **Rev. Proc. 2024-40** (TY2025) §.15 — same (irs.gov/pub/irs-drop/rp-24-40.pdf, read verbatim;
  **superseded for basic amounts by OBBBA**).
- **OBBBA / Pub. L. 119-21** — §70102 std deduction ($15,750/$31,500/$23,625), §70103 senior
  deduction ($6,000), §70120 SALT cap ($40k, 30% phasedown >$500k). *Secondary corroboration
  (Tax Foundation, H&R Block, IRS newsroom); verify exact §-numbers + dollars against engrossed
  text / final 2025 IRS instructions.*
- **2024 Form 1040 Instructions** — QDCGT Worksheet—Line 16 (p. 36, verbatim 25 lines); Tax
  Table vs. TCW $100k rule (p. 33); Tax Table $50 bins (p. 74); TCW `×rate − subtraction` (p. 76).
- **2024 Instructions for Schedule A** — medical 7.5% (line 1), SALT $10k/$5k (line 5),
  mortgage $750k/$1M tiers (line 8), charitable + 5-yr carryover (lines 11–14) — read verbatim.
- **2025 Instructions for Schedule D** — QDCGT-vs-Sch-D-Tax-Worksheet trigger (lines 18/19).
- **Form 8959 / 8960 instructions** — thresholds & line structure.
- **IRC** §63(c)/(f), §164(b)(6), §163(h), §170(b)/(d)/(e), §1(h), §1211/§1212/§1222, §1401(b),
  §1411 — as cited inline; existing engine cites already validated in `core/tax/`.
