# Fable Recon 03 — Deferred Follow-On Math, Spec-Grade (Schedule 1-A · QBI/§199A · CTC/Schedule 8812)

**Agent:** Fable second pass (round 2), F3. **Date:** 2026-07-11.
**Scope:** spec-grade formulas for the items the first (opus) pass deliberately deferred:
the full **TY2025 Schedule 1-A** phase-out math (OBBBA), the **Form 8995** simplified QBI
path, and **CTC/ODC/ACTC (Schedule 8812)**. These target the follow-on cycles (TY2025 +
CTC), but the formulas are locked now to de-risk v1 (TY2024) data capture.
**Convention:** `floor`/`ceil` are on the quotient of a division; all money is USD;
`excess ≥ 0` means `max(0, …)` is applied first.

## Source verification (all read directly, this session)

| Source | Provenance | Status |
|---|---|---|
| **Schedule 1-A (Form 1040), 2025** | `irs.gov/pub/irs-pdf/f1040s1a.pdf` | **FINAL** (PDF created 1/2/2026), both pages read line-by-line. Supersedes the 11/4/25 draft the first pass used — line structure identical. |
| **Instructions for Schedule 1-A** | inside **2025 Instructions for Form 1040**, pp. 101–110 (`irs.gov/pub/irs-pdf/i1040gi.pdf`, rev. created 2/25/2026) | FINAL. There is **no standalone `i1040s1a.pdf`** — the Sch 1-A instructions live in the 1040 general instructions. My copy already includes the two 2/27/2026 errata (tips net-income-limitation expansion; "Death of a taxpayer in 2025" senior rule) per `irs.gov/forms-pubs/changes-to-the-2025-instructions-for-form-1040`. |
| **Form 8995 (2024)** + 2024 instructions | `irs-prior/f8995--2024.pdf`, `irs-prior/i8995--2024.pdf` | FINAL, read verbatim. |
| **Form 8995 (2025)** + 2025 instructions | `irs-pdf/f8995.pdf`, `irs-pdf/i8995.pdf` (created 1/26/2026) | FINAL, read verbatim. |
| **Schedule 8812 (2024 & 2025)** + both years' instructions | `irs-prior/f1040s8--2024.pdf`, `i1040s8--2024.pdf`; `irs-pdf/f1040s8.pdf` (12/19/2025), `i1040s8.pdf` (1/23/2026) | FINAL, read verbatim incl. Credit Limit Worksheet A + Earned Income Worksheet. |
| **IRC §224, §225, §151(d)(5), §163(h)(4), §24** (as amended by OBBBA) | `uscode.house.gov` prelim edition | Statutory text quoted below; every OBBBA §-number cross-checked (Pub. L. 119-21, 7/4/2025: §70201 tips, §70202 overtime, §70203 car loan, §70103 senior, §70104 CTC, §70105 §199A). |

---

## 1. Schedule 1-A "Additional Deductions" (TY2025–2028 only)

### 1.0 Placement — confirmed

- All four deductions are **below-the-line but non-itemized**: they reduce **taxable
  income, never AGI**. Confirmed structurally on the final 2025 Form 1040: Sch 1-A
  **line 38 → 1040 line 13b**; `L14 = L12e + L13a + L13b`; `L15 = L11b − L14`
  (1040-NR: line 13c). Instructions: "Use Schedule 1-A to report additional deductions
  that can't be entered directly on Form 1040… You can claim these deductions whether you
  claim the standard deduction or itemize" (i1040gi 2025 p. 101).
- **Consequence for the engine:** AGI-keyed quantities are untouched — Form 8960 NIIT
  MAGI, Sch A 7.5% medical floor, SALT phase-down MAGI, IRA/student-loan phase-outs all
  read AGI/MAGI *above* this line. Only taxable income (and therefore the §1(h) stack
  input and, via 8995 L11, the QBI limit — see §2.3) moves.
- **Termination:** every part expires for taxable years beginning after 12/31/2028
  (§224(g), §225(g), §163(h)(4)(E), §151(d)(5)(A)). None of the four caps/thresholds is
  inflation-indexed.
- **Filing-status gate (all four parts):** if married, **must file jointly** (§224(e)(1),
  §225(e)(1), §151(d)(5)(B)(iv), §163(h)(4)(D); form Cautions). MFS gets $0 from every
  part. Recipient (tips/OT) or qualifying individual (senior) must have a **valid SSN**
  (employment-valid, issued before the return due date).

### 1.1 Part I — MAGI (lines 1–3)

```
MAGI = 1040 line 11b (AGI)
     + excluded Puerto Rico income          (§933)   [L2a]
     + Form 2555 line 45 + line 50          (§911)   [L2b, L2c]
     + Form 4563 line 15                    (§931)   [L2d]
```
Statutory MAGI definition is identical in all four provisions: "adjusted gross income
increased by any amount excluded from gross income under section 911, 931, or 933"
(§224(d), §225(d), §151(d)(5)(C), §163(h)(4)(C)). This **one** Part-I MAGI (line 3)
feeds every part (lines 8, 16, 25, 31). For the common W-2 household (no foreign/PR
exclusions): **MAGI = AGI**, and the same add-back set is used by Schedule 8812 (§3.1)
— one shared `magi_with_foreign_addbacks` value serves both schedules.

### 1.2 Part II — No Tax on Tips (lines 4–13)

**Statute:** new IRC **§224** (OBBBA §70201). Cap **$25,000 regardless of filing status**
(§224(b)(1)) — a **per-return, combined** cap for MFJ, not per-spouse (instructions TIP,
p. 101). Phase-out (§224(b)(2)): "reduced (but not below zero) by **$100 for each $1,000**
by which … MAGI exceeds **$150,000 ($300,000** in the case of a joint return)."

**Form mechanics (final form, verbatim):**
```
qualified_tips = L4c (employee tips) + L5 (trade/business tips)          [L6]
L7  = min(L6, 25_000)
L10 = MAGI − (150_000 | 300_000 MFJ)          ; if ≤ 0 → deduction = L7, done
L11 = floor(L10 / 1_000)      ← form: "decrease to the next LOWER whole number"
L12 = L11 × 100
L13 = max(0, L7 − L12)        → tips deduction
```
Fully phased out at MAGI ≥ threshold + $251,000 (if at the $25,000 cap); in general at
`threshold + 1000·ceil(cap/100)`.

**Input-side definitions (needed for data capture):**
- *Qualified tips* = voluntary cash/charge tips (incl. tip-sharing/pools), paid without
  negotiation, amount determined by the payer, received in an occupation on the Treasury
  tipped-occupation list (TTOC codes, `IRS.gov/TippedOccupations`) that "customarily and
  regularly received tips on or before December 31, 2024" (§224(d); instr. pp. 101–103).
  Mandatory service charges/auto-gratuities are NOT qualified (unless customer could
  freely modify). Tips of an employee of an **SSTB** employer — or self-employment tips
  in an SSTB — are NOT qualified (§224(d)(3)).
- **L4a** = W-2 box 7. ⚠ Box 7 under-reports when box 5 > **$176,100** (2025 SS wage
  base) — tips above the base don't appear; alternates: Form 4070 totals, employer box-14
  statement, or Form 4137 amounts (instr. "Determining the amount…", methods 1–4; W-2 was
  **not** updated for 2025 — separate tip accounting starts on the 2026 W-2).
- **L4c multi-employer rule:** per employer take **max**(W-2/4070-reported tips, Form 4137
  line 1 col (c) total) then sum ("Qualified Tips From More Than One Employer Worksheet")
  — max, because the 4137 per-employer total already includes reported tips.
- **L5 (self-employment tips), net-income limitation** (§224(a); instr. incl. the
  2/27/2026 expansion): per trade or business,
  `L5_contribution = min(qualified_tips_of_business, max(0, net_profit − allocable_deductions))`
  where allocable deductions include the ½-SE-tax deduction (Sch 1 L15), self-employed
  SEP/SIMPLE and health-insurance deductions attributable to that business — but not the
  tips deduction itself. A net-loss business contributes $0.
- Transition relief for 2025 reporting: **Notice 2025-69**.

### 1.3 Part III — No Tax on Overtime (lines 14–21)

**Statute:** new IRC **§225** (OBBBA §70202). Cap **$12,500 ($25,000 MFJ)** — again a
combined, per-return cap. Phase-out identical in form to tips: **$100 per $1,000** over
MAGI **$150,000/$300,000 MFJ** (§225(b)(2)).

```
qual_ot = L14a (W-2-side FLSA OT premium) + L14b (1099-NEC box 1 / 1099-MISC box 3 side)
L15 = min(qual_ot, 12_500 | 25_000 MFJ)
L18 = MAGI − (150_000 | 300_000 MFJ)          ; if ≤ 0 → deduction = L15, done
L19 = floor(L18 / 1_000)                       ← same LOWER-whole-number rule as tips
L20 = L19 × 100
L21 = max(0, L15 − L20)       → overtime deduction
```

**Input-side definition:** *qualified overtime compensation* = only the **premium ("half")
portion of FLSA §7-required time-and-a-half** (§225(d)). Excludes: pay above the required
premium (e.g. the second "half" of double-time), holiday/weekend premiums without >40 hrs,
state-law-only overtime for FLSA-ineligible employees, and **qualified tips** (no
double-dip). Not separately reported on the 2025 W-2 (some employers use box 14);
instructions bless estimation methods 1–5 (e.g. total pay for OT hours ÷ 3 under
time-and-a-half; special §7 work-period rules for fire/law-enforcement). For the engine
this is a **user-entered scalar** with a "premium portion only" prompt.

### 1.4 Part IV — No Tax on Car Loan Interest (lines 22–30)

**Statute:** §163(h)(2)(F) + new **§163(h)(4)** (OBBBA §70203): "qualified passenger
vehicle loan interest" (QPVLI) is excluded from nondeductible "personal interest" for
TY2025–2028. Cap **$10,000** (§163(h)(4)(B)(ii)). Phase-out (§163(h)(4)(B)(iii)):
"reduced (but not below zero) by **$200 for each $1,000 (or portion thereof)** by which
MAGI exceeds **$100,000 ($200,000** joint)."

```
L23 = Σ per-VIN column (iii)   [total QPVLI paid − portion deducted on Sch C/E/F, col (ii)]
L24 = min(L23, 10_000)
L27 = MAGI − (100_000 | 200_000 MFJ)          ; if ≤ 0 → deduction = L24, done
L28 = ceil(L27 / 1_000)        ← form: "INCREASE to the next HIGHER whole number"
L29 = L28 × 200
L30 = max(0, L24 − L29)        → QPVLI deduction
```
Fully phased out at MAGI > threshold + $49,000 (at the $10,000 cap) — a fast 20%-of-MAGI
slope in $200 stair-steps.

> **⚠ SPEC-CRITICAL ASYMMETRY (the surprise of this recon):** Parts II/III round the
> phase-out quotient **DOWN** (floor — taxpayer-favorable: "decrease 1.5 to 1, and 0.05
> to 0"), Part IV rounds **UP** (ceil: "increase 1.5 to 2, and 0.05 to 1"), and Schedule
> 8812 line 10 also rounds **UP** (§3.1). This is statutory, not an IRS whim: §224/§225
> say "$100 for each $1,000"; §163(h)(4) and §24 say "for each $1,000 **(or fraction/
> portion thereof)**". A single shared `phase_out(excess, per, step)` helper with a
> hard-coded direction would be silently wrong for one side or the other — the rounding
> mode must be a parameter, with a per-part test each.

**Eligibility (data capture, verified against §163(h)(4) + instr. pp. 109–110):** loan
**originated after 12/31/2024** by the taxpayer; proceeds purchase (not lease) an
*applicable passenger vehicle*; **secured by a first lien** on it; vehicle: **original
use commences with the taxpayer (new, not used)**, car/minivan/van/SUV/pickup/motorcycle,
≥2 wheels, GVWR < 14,000 lb, street use, and **final assembly in the United States**
(dealer info label or NHTSA VIN decoder). Personal use = expected >50% non-business use.
Refinance keeps eligibility (up to refinanced balance, first lien). Loan amount may
include customarily-financed items (sales tax, extended warranty) but not negative
equity or insurance. **The VIN must be reported on the return** (§163(h)(4)(B)(iv), form
line 22 col (i)) — a new PDF-fill field (per-character comb boxes on the official form).
Interest deducted on Sch C/E/F cannot be double-counted (col (ii)).

### 1.5 Part V — Enhanced Deduction for Seniors (lines 31–37)

**Statute:** new IRC **§151(d)(5)** (OBBBA §70103) — codified in §151 (personal
exemptions), *not* §63(f); it stacks **on top of** the §63(f) aged-65 additional standard
deduction, and (unlike §63(f)) survives itemizing. "$6,000 for each qualified individual"
(taxpayer/spouse who "**attained age 65** before the close of the taxable year"), "reduced
by **6 percent** of so much of … MAGI as exceeds **$75,000 ($150,000** joint)."

**Form mechanics — the reduction is computed ONCE on the per-person $6,000, then each
qualifying person claims that same reduced amount:**
```
L33 = MAGI − (75_000 | 150_000 MFJ)           ; if ≤ 0 → L35 = 6_000
L34 = 0.06 × L33                               (no rounding step — smooth, not staired)
L35 = max(0, 6_000 − L34)
L37 = L35 × (# of qualifying individuals: 36a you, 36b spouse)   → senior deduction
```
- Qualifying: **born before January 2, 1961** (attains 65 by 1/1/2026; a person reaches
  65 the day *before* the 65th birthday) + valid SSN + MFJ if married. **Death rule**
  (2/27/2026 errata): died in 2025 *before* reaching 65 → not qualified, even if born
  before 1/2/1961.
- **Phase-out geometry (flag for optimizer):** because L35 is per-person, an MFJ couple
  with two seniors loses **12¢ per $1 of MAGI** in the band (each person's $6,000 falls
  6%); everyone (1 or 2 seniors, any status) hits $0 at `threshold + 100,000` — MAGI
  $175,000 single / $250,000 MFJ. Unlike Parts II–IV this phase-out is **smooth** (no
  $1,000 stair-steps).

### 1.6 Part VI + engine notes

`L38 = L13 + L21 + L30 + L37 → 1040 L13b`. File Schedule 1-A only if L38 > 0.

- **Computation order (extends recon-01 §6 Stage 3):** AGI → Sch 1-A MAGI → L38 →
  **then** 8995 L11 (which subtracts L13b, see §2.3) → taxable income. Schedule 1-A never
  reads a deduction, so the DAG stays acyclic.
- **Marginal-effect coupling (what-if/optimize):** crypto gains raise MAGI 1:1, so in the
  phase-out bands each extra $1,000 of MAGI claws back $100 (tips) + $100 (OT) staired,
  $200 (car, staired), and 6%/12% (senior, smooth) of deduction — i.e. **hidden marginal
  rate adders** on 2025 returns; Parts II–IV make marginal tax a step function. The
  existing delta engine will see this automatically once Sch 1-A is inside both scenarios,
  but per-$1 what-ifs will show $0 then a cliff — document it, don't "fix" it.
- **Per-year table (btctax-adapters, 2025–2028):** tips_cap 25 000; ot_cap 12 500/25 000;
  qpvli_cap 10 000; senior_amt 6 000; thresholds 150k/300k (tips+OT), 100k/200k (car),
  75k/150k (senior); steps $100-floor, $100-floor, $200-ceil, 6%-smooth. **None indexed;
  all die after TY2028.**

### 1.7 Worked examples (each part)

**(a) MFJ, no phase-out + senior in-band.** MAGI $163,500; spouse A (bartender, on the
TTOC list) W-2 box 7 = $8,000; spouse A FLSA OT premium (box 14) = $6,200; car loan
(new US-assembled SUV, loan 3/2025, first lien) interest = $4,800; spouse B born 1958.
Tips: L7 = 8,000; MAGI ≤ 300k → **13 = 8,000**. OT: L15 = 6,200; ≤ 300k → **21 = 6,200**.
Car: L24 = 4,800; ≤ 200k → **30 = 4,800**. Senior: L33 = 13,500 → L34 = 810 →
L35 = 5,190; one qualifier (36b) → **37 = 5,190**. **L38 = 24,190 → 1040 L13b.**

**(b) Tips/OT floor rounding.** Single, MAGI $157,350, qualified tips $3,000. Excess
7,350 → `floor(7.35) = 7` → reduction $700 → **deduction $2,300**. (Ceiling would have
given $2,200 — an $100 error a naive shared helper would make.)

**(c) Car-loan ceiling rounding.** Single, MAGI $104,050, QPVLI $6,000. Excess 4,050 →
`ceil(4.05) = 5` → reduction $1,000 → **deduction $5,000**. (Floor would give $5,200.)

**(d) Two-senior MFJ.** MAGI $200,000, both born before 1/2/1961, both SSNs. L33 =
50,000 → L34 = 3,000 → L35 = 3,000 → **L37 = 6,000** (3,000 each; note aggregate slope:
$50,000 excess cost the couple $6,000 of $12,000 = 12%).

---

## 2. QBI / §199A — Form 8995 simplified path

### 2.1 Which form — the taxable-income cutover

Form-face rule (read from both years' Form 8995): use **8995** iff taxable income
*before* the QBI deduction ≤ **$191,950 / $383,900 MFJ (TY2024)**; **$197,300 / $394,600
MFJ (TY2025)** — and not an agricultural/horticultural co-op patron. Above → **Form
8995-A**. (Indexed annually: §199A(e)(2), Rev. Proc. 2023-34 / 2024-40.)

**Common-case simplification (box-5-only household):** the W-2/UBIA wage limit applies
only to the *QBI component* per trade or business (§199A(b)(2)); the **REIT/PTP component
(§199A(b)(1)(B)) is never wage-limited and never SSTB-limited**. So for a household whose
only §199A item is 1099-DIV box 5, the *math* below is identical on 8995-A (Part IV lines
28–31 mirror 8995 lines 6–9) — only the **PDF changes**. Engine recommendation: implement
the math once; above the threshold either fill 8995-A Part IV or `NotComputable`-refuse if
actual QBI (not just REIT dividends) exists.

### 2.2 The computation (Form 8995, 2024/2025 lines identical)

```
L2  = Σ qualified business income (QBI)              — 0 for pure W-2 household
L3  = prior-year QBI loss carryforward (negative)
L4  = max(0, L2 + L3)
L5  = 0.20 × L4                                      — QBI component
L6  = qualified REIT dividends (Σ 1099-DIV box 5) + qualified PTP income
L7  = prior-year REIT/PTP loss carryforward (negative)
L8  = max(0, L6 + L7)
L9  = 0.20 × L8                                      — REIT/PTP component
L10 = L5 + L9
L11 = taxable income BEFORE the QBI deduction        (see §2.3 — year-dependent!)
L12 = net capital gain = qualified dividends (1040 3a)
      + (if Sch D filed: max(0, min(Sch D L15, Sch D L16)); else 1040 L7 amount)
L13 = max(0, L11 − L12)
L14 = 0.20 × L13                                     — income limitation
L15 = min(L10, L14)  → 1040 L13 (2024) / L13a (2025) — the QBI deduction
L16 = min(0, L2 + L3)   carryforward out (QBI)
L17 = min(0, L6 + L7)   carryforward out (REIT/PTP)
```
L12's definition is verbatim from i8995 (both years): "line 3a, plus your net capital
gain … the smaller of Schedule D line 15 or 16, unless line 15 or 16 is zero or less, in
which case nothing is added to the qualified dividends" — i.e. **exactly the engine's
`preferential_gain` + QD**, the same quantity as QDCGT-worksheet L4. Statutory: §199A(a),
(b)(1), (e)(3) ("net capital gain" = §1(h) net capital gain + qualified dividend income).

### 2.3 Line 11 — the year-dependent definition (new finding)

- **2024** (i8995-2024): `L11 = 1040 line 11 − 1040 line 12` (AGI − std/itemized).
- **2025** (i8995-2025, verbatim): `L11 = 1040 line 11a − lines 12e **and 13b**`.

**⇒ Schedule 1-A REDUCES the QBI income-limitation base.** Ordering is therefore
AGI → Sch 1-A → 8995 → taxable income (no cycle: Sch 1-A reads only MAGI). A 2024-shaped
`AGI − deduction` formula reused for 2025 silently over-allows QBI for anyone with a
Sch 1-A deduction. Both years: L11 is *before* QBI but *after* every other deduction.

### 2.4 Carryforwards + REIT-dividend hygiene

Negative combined QBI (or REIT/PTP) totals produce **no current deduction from that
component** and carry forward indefinitely against the same component (L16/L17;
§199A(c)(2)). New engine state: two per-year carryforward scalars (mirror the existing
capital-loss `Carryforward` discipline). Box-5 hygiene: §199A dividends are a *subset* of
1099-DIV box 1a ordinary (non-qualified) dividends — they stay in the **ordinary** income
stack for tax purposes; box 5 only feeds this deduction (no income-side double-count).
Box 5 must have been held ≥ 46 days to qualify (payer responsibility; capture as given).

**TY2026+ heads-up (OBBBA §70105, NOT applicable to 2024/2025):** §199A permanent;
SSTB/wage-limit phase-in ranges widen $50k→$75k ($100k→$150k MFJ); new **$400 minimum
deduction** when active-business QBI ≥ $1,000 (both indexed after 2026) — effective
taxable years beginning after 12/31/2025. A TY2026 adapter table must not clone 2025.

### 2.5 Worked example

MFJ 2024. Taxable income before QBI (L11) = $148,000 (< $383,900 → Form 8995). 1099-DIV
box 5 = $2,400; no QBI/PTP, no carryforwards. QD (3a) = $3,100; Sch D L15 = $9,500,
L16 = $11,200 → net capital gain = 3,100 + min(9,500, 11,200) = $12,600.
L8 = 2,400 → **L9/L10 = 480**. L13 = 148,000 − 12,600 = 135,400 → L14 = 27,080.
**L15 = min(480, 27,080) = $480 → 1040 L13.**
*Limit-binding variant:* retiree, L11 = $30,000 of which QD+LTCG = $28,000 →
L14 = 0.20 × 2,000 = 400 < L10; a $2,400 box 5 yields only **$400**.

---

## 3. CTC / ODC / ACTC — Schedule 8812

**Statute:** IRC §24 as amended by **OBBBA §70104** (effective TY2025; also makes the
$400k/$200k thresholds and the $500 ODC permanent and adds the taxpayer-SSN rule).
Landing lines (both years): **Part I L14 → 1040 L19** (nonrefundable CTC+ODC);
**Part II L27 → 1040 L28** (refundable ACTC). Both unchanged 2024→2025 on the 1040.

### 3.1 Part I — the credit and phase-out

```
L1  = 1040 line 11 (AGI)
L3  = MAGI = L1 + excluded PR income + Form 2555 L45+L50 + Form 4563 L15   [same add-backs as Sch 1-A Part I]
L4  = # qualifying children under 17 at year-end with valid SSN
L5  = L4 × CTC_PER_CHILD          (2024: $2,000 · 2025: $2,200)
L6  = # other dependents (§152 dependents that aren't L4 children: 17+, ITIN kids, parents…)
L7  = L6 × 500                     (ODC, not indexed)
L8  = L5 + L7
L9  = 400_000 (MFJ) | 200_000 (all others)         — not indexed (permanent, §24(h)/(i))
L10 = MAGI − L9, rounded UP to the next multiple of $1,000   ← "(or fraction thereof)", §24(b)
L11 = 0.05 × L10
L12 = L8 − L11                     ; ≤ 0 → NO CTC/ODC/ACTC at all (form STOP box)
L13 = Credit Limit Worksheet A     (tax-liability cap, below)
L14 = min(L12, L13)  → 1040 L19
```
- **Phase-out slope:** −$50 per $1,000-or-fraction of MAGI over the threshold, applied to
  the **combined** CTC+ODC pool (ceiling rounding — same direction as Sch 1-A Part IV,
  opposite to Parts II/III).
- **Credit Limit Worksheet A** (2024 & 2025 identical, read verbatim):
  `CLW-A = 1040 L18 − (Sch 3 L1 + L2 + L3 + L4 + L5b + L6d + L6f + L6l + L6m)`
  minus a Credit Limit Worksheet B amount only when claiming Form 8396/8839/5695-Part-I/
  8859 credits (rare; out of scope — treat as 0, guard-refuse if those forms appear).
  For the common household: **CLW-A = L16 tax + Sch 2 L3 − foreign-tax/dependent-care/
  education/saver's credits** — i.e. CTC comes *after* the other Sch 3 Part I credits.
- **Qualifying child** (§24(c) → §152(c)): under 17 at year-end, SSN valid for employment
  issued before return due date; child ITIN → falls to the $500 ODC pool. **Dependent
  model needs a birth date, not a static `ctc_eligible` flag** (a 16-year-old flips to
  ODC next year), plus an `ssn_valid_for_employment` bool.

### 3.2 Part II — refundable ACTC

```
L16a = L12 − L14                    ; 0 → no ACTC
L16b = L4 × ACTC_CAP                (2024: $1,700 · 2025: $1,700)   [$1,400 indexed, §24(h)(5)/(d)]
L17  = min(L16a, L16b)
L18a = earned income (worksheet below);  L18b = nontaxable combat pay
L19  = max(0, L18a − 2_500)         ($2,500 floor, §24(d)(1)(B)(i), not indexed)
L20  = 0.15 × L19
if L16b < 3 × ACTC_CAP ($5,100):    ACTC = min(L17, L20)            [≤ 2 children]
else (3+ children):                                                  [Part II-B]
    L21 = SS + Medicare + Add'l Medicare withheld (W-2 boxes 4+6; 8959-adjusted)
    L22 = Sch 1 L15 (½ SE tax) + Sch 2 L5 + L6 + L13
    L25 = max(0, L21 + L22 − (EIC (1040 L27) + Sch 3 L11 excess-SS))
    ACTC = min(L17, max(L20, L25))
L27 = ACTC → 1040 L28
```
- **Earned income (L18a)**, from the Earned Income Worksheet (read verbatim): 1040 L1z
  wages + nontaxable combat pay + statutory-employee Sch C income + Sch C/F net profit
  (SE) − excluded Medicaid-waiver pay − **Sch 1 L15 (½ SE tax)**. Pure W-2 household:
  `L18a = L1z (+ combat pay)`. With crypto-SE income the engine already holds both the
  net profit and the ½-SE-tax adjustment — reuse those.
- **Form 2555 filers cannot claim ACTC** (form caution). CTC (nonrefundable) still OK.
- 2025 form delta: old L15 "opt out of ACTC" checkbox is **"reserved for future use"**;
  Part II-B line refs (Sch 1 L15; Sch 2 L5/L6/L13) unchanged.

### 3.3 2024 vs 2025 parameter table (per-year adapter entries)

| Parameter | TY2024 | TY2025 | Indexing |
|---|---|---|---|
| CTC per qualifying child | $2,000 | **$2,200** | indexed after 2025 (§24(i)(2), 2024 base, round DOWN to $100) |
| ODC per other dependent | $500 | $500 | never |
| Phase-out threshold | $400k MFJ / $200k | same | never (permanent) |
| Phase-out slope | $50 per $1,000-or-fraction (ceil) | same | — |
| ACTC refundable cap /child | $1,700 | $1,700 | $1,400 indexed (Rev. Proc. 2023-34 / 2024-40) |
| Earned-income floor / rate | $2,500 / 15% | same | never |
| SSN rules | each QC needs SSN | **+ taxpayer (≥1 spouse if MFJ) needs valid SSN** (§24(h)(7), OBBBA) | — |

### 3.4 Worked example (2024)

MFJ, MAGI $412,300; two QCs (7 and 12, SSNs) + one 19-year-old dependent (ODC);
1040 L18 = $3,200; only Sch 3 credit: $250 foreign tax (L1); wages L1z = $420,000.
L5 = 4,000; L7 = 500; L8 = 4,500. L10 = 12,300 → **13,000** (ceil to $1,000 multiple);
L11 = 650. L12 = 3,850. CLW-A = 3,200 − 250 = 2,950 → L14 = **2,950 → 1040 L19**.
ACTC: L16a = 900; L16b = 2 × 1,700 = 3,400 (< 5,100 → skip II-B); L17 = 900;
L19 = 417,500 → L20 = 62,625; ACTC = min(900, 62,625) = **900 → 1040 L28**.
Total CTC delivered $3,850 of the $4,500 pre-phase-out pool.
*(2025 same facts: L5 = 4,400, L8 = 4,900, L12 = 4,250; and the return must carry a valid
taxpayer SSN.)*

---

## 4. Cross-cutting engine consequences (v1 data-capture de-risking)

1. **Birth dates, not booleans.** Senior deduction (born < 1/2/1961), age-65 std-deduction
   bump, and CTC "under 17 at year-end" all want `Person.date_of_birth` /
   `Dependent.date_of_birth` captured in v1 — deriving flags per-year beats storing them.
2. **SSN-validity flags.** Four Sch 1-A parts + CTC/ACTC hinge on "valid for employment"
   SSNs (taxpayer, spouse, each QC). Capture `ssn_valid_for_employment: bool` now.
3. **One shared MAGI-with-foreign-add-backs.** Sch 1-A Part I ≡ Sch 8812 L1–L3 (§911/931/
   933). Common case = AGI; keep the add-back hook in one place.
4. **One parameterized phase-out helper** —
   `phase_out(excess, per=1000, step, rounding: Floor|Ceil|Smooth)` — with the direction
   per §1.4's warning: Floor (tips, OT), Ceil (car loan, CTC), Smooth 6% (senior).
5. **New W-2-adjacent scalars with no box of their own (2025):** qualified tips (box 7 ±
   corrections), qualified OT premium (box 14/statement), QPVLI + VIN. All must be
   user-entered `Usd` fields; the 2026 W-2/1099 revisions will add real boxes.
6. **QBI needs two new carryforward scalars** (L16/L17) and the year-keyed L11 definition
   (2025 subtracts 1040 L13b).
7. **Ordering (final):** AGI → [Sch A | std] → **Sch 1-A (MAGI)** → **8995 (L11 uses
   both)** → taxable income → L16 tax → Sch 3 Part I credits → **CTC (CLW-A)** → L22 →
   Sch 2 Part II other taxes → **ACTC needs Sch 2/Sch 3/EIC complete** (Part II-B L22/L24)
   → payments. ACTC is the last credit computed — it reads Schedule 3 L11 and Sch 2
   L5/6/13, so it belongs in Stage 7, not Stage 5, of recon-01 §6.

## 5. Uncertainty / open flags

- **Tips occupation list**: qualification is per-TTOC occupation (plus SSTB and
  legality carve-outs) — unenforceable offline; treat "qualified tips" as user-attested
  input with a caution note, like the existing attest-gate pattern.
- **Overtime premium estimation** (instr. methods 1–5) is inherently approximate for
  2025; keep the raw user entry + provenance note in the vault.
- **Form 8995-A PDF fill** (box-5-only, income above threshold) — math identical
  (§2.1) but the form/PDF map is unbuilt; decide fill-vs-refuse at spec time.
- **TY2026**: §199A phase-in/minimum-deduction changes (§70105), CTC indexing ($2,200
  base), 8995 thresholds, and the 2026 W-2 tip/OT boxes all change — the per-year table
  discipline (never clone a prior year) is the defense.
- Rev. Proc. 2025-32 (TY2026 amounts) exists but was not pulled; out of scope here.
