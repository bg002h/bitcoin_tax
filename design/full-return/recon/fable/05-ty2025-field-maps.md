# Fable Recon 05 — TY2025 Field-Map Skeletons + TY2024 Spot-Verify

**Agent:** Fable recon F5, round 2. **Date:** 2026-07-11.
**Depends on:** `../deep/03-ty2024-field-maps.md` (TY2024 skeletons), `../03-irs-fillable-pdfs.md`
(root-flip prediction), `../01-form-graph.md` §5 (2025 renumbering).

## Method — real extraction, no invented names

All six TY2025 PDFs pulled from `https://www.irs.gov/pub/irs-pdf/` (`f1040.pdf`, `f1040s1.pdf`,
`f1040s2.pdf`, `f1040s3.pdf`, `f1040sa.pdf`, `f1040sb.pdf`; all HTTP 200). `pdfinfo` titles
confirm every one is the **final TY2025 revision** ("2025 Form 1040" created 1/2/26, "2025
Schedule 1/2/3" created 1/2/26, "2025 Schedule A" created 12/18/25, "2025 Schedule B" created
11/3/25). **`pdftotext` finds zero "DRAFT" hits on any of the six — no draft caveat applies;
these are the filing-season finals.** (The §5 renumbering seen on the 9/5/25 draft survived to
final unchanged: L7a, L11a/L11b, L12e, L13a, L13b.)

Extraction method identical to deep/03: **`qpdf --json --json-key=acroform`** (fullname /
fieldtype / pageposfrom1) joined to **`qpdf --json --json-key=qpdf`** objects (widget `/Rect`,
`/AP /N` on-states), line numbers assigned **geometrically** against `pdftotext -bbox` +
`-layout` captions (top-origin, page height 792pt). No `/TU` tooltips exist on the 2025 forms
either — mapping remains geometry-derived. Every field name below is copied verbatim from the
actual AcroForm.

**Field-name convention** (same as deep/03): tables give the FQN **suffix after the root**;
full map key = `<ROOT> + "." + <suffix>`. Money leaves `f<page>_NN[0]`; checkboxes `c<page>_NN[0]`.

---

## Part (a) — TY2024 spot-verify: **13/13 OK, zero mismatches**

Re-pulled the six `irs-prior/*--2024.pdf` files and resolved a 13-name sample from deep/03
against the live AcroForms:

| deep/03 name (full FQN) | Check | Result |
|---|---|---|
| `topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].f1_52[0]` | 1040 L7, /Tx, p1 | **OK** |
| `topmostSubform[0].Page1[0].f1_57[0]` | 1040 L12, bare Page1 | **OK** |
| `topmostSubform[0].Page2[0].f2_10[0]` | 1040 L24, p2 | **OK** |
| `topmostSubform[0].Page1[0].FilingStatus_ReadOrder[0].c1_3[1]` | MFJ, on-state `/3` | **OK** (`/3` confirmed) |
| `topmostSubform[0].Page1[0].c1_5[1]` | digital-asset No, `/2` | **OK** (`/2` confirmed) |
| `form1[0].Page1[0].f1_38[0]` | Sch 1 L10 | **OK** |
| `form1[0].Page2[0].f2_31[0]` | Sch 1 L26 | **OK** |
| `form1[0].Page2[0].f2_25[0]` | Sch 2 L21 page-2 split | **OK** |
| `topmostSubform[0].Page1[0].f1_26[0]` | Sch 3 L8 | **OK** |
| `topmostSubform[0].Page1[0].f1_34[0]` | Sch A L17 | **OK** |
| `topmostSubform[0].Page1[0].f1_3[0]` | Sch A L1, no-leading-zero leaf | **OK** |
| `topmostSubform[0].Page1[0].ReadOrderControl[0].f1_34[0]` | Sch B div row-1 name | **OK** |
| `topmostSubform[0].Page1[0].f1_64[0]` | Sch B L6 | **OK** |

Field counts also re-verified exactly (1040=141, S1=69, S2=60, S3=39, SA=37, SB=72), and the
Sch 3 quirk leaf resolves at `topmostSubform[0].Page1[0].Line6z_ReadOrder[0].f2_23[0]`,
matching deep/03's row verbatim. **No defect found anywhere in the deep/03 skeletons.**

---

## Part (b) — The six TY2025 root container FQNs (quote these into each map)

| Form | Root FQN (TY2024) | **Root FQN (TY2025)** | Flip? | Pages | Fields 24→25 |
|---|---|---|---|---|---|
| Form 1040 | `topmostSubform[0]` | `topmostSubform[0]` | — | 2 | 141 → **199** (+58) |
| Schedule 1 | `form1[0]` | **`topmostSubform[0]`** | **FLIPPED** | 2 | 69 → 73 |
| Schedule 2 | `form1[0]` | `form1[0]` | — | 2 | 60 → 63 |
| Schedule 3 | `topmostSubform[0]` | `topmostSubform[0]` | — | 1 | 39 → 37 |
| Schedule A | `topmostSubform[0]` | **`form1[0]`** | **FLIPPED** | 1 | 37 → 33 |
| Schedule B | `topmostSubform[0]` | `topmostSubform[0]` | — | 1 | 72 → 72 |

Exactly as `../03-irs-fillable-pdfs.md` R1 predicted: **Sch 1 and Sch A flip roots in 2025.**
Never hardcode the root; capture per (form, year).

---

## Form 1040 (TY2025) — root `topmostSubform[0]` — the +58-field rebuild

Page 1 now ends at **L11a (AGI)**; page 2 opens with **L11b** (AGI repeated) and carries the
deduction block. The +58 fields decompose into: new special-situation header (≈17), split
first/last name fields (+2/, unchanged net), restructured+transposed dependents grid (≈+18),
new sub-line checkboxes 1040 3c/4c/5c/6d/7b (≈+11), 12a–12d moved to page 2 (same count), new
EIC sub-items 27b/27c + former-spouse SSN (+3), L30 un-reserved (adoption credit), L38 penalty
box, direct-deposit block unchanged.

### Identity / header (page 1)

| Line/Concept | Field suffix | Page | Notes |
|---|---|---|---|
| Fiscal-year begin / end / yr | `Page1[0].f1_01[0]` / `f1_02[0]` / `f1_03[0]` | 1 | blank for calendar-year |
| **NEW** "Filed pursuant to section 301.9100-2" box | `Page1[0].c1_1[0]` (on `/1`) | 1 | special-situation header row, y≈62 |
| **NEW** "Combat zone" box + write-in | `Page1[0].c1_2[0]` (on `/1`) + `f1_04[0]` | 1 | write-in x≈210–371 |
| **NEW** "Deceased" box + MM/DD/YYYY | `Page1[0].c1_3[0]` (on `/1`) + `f1_05..f1_07` | 1 | taxpayer date-of-death |
| **NEW** spouse deceased MM/DD/YYYY | `Page1[0].f1_08..f1_10` | 1 | |
| **NEW** "Other" box + 3 write-ins | `Page1[0].c1_4[0]` (on `/1`) + `f1_11..f1_13` | 1 | |
| Your first name & MI / last name / SSN | `Page1[0].f1_14[0]` / `f1_15[0]` / `f1_16[0]` | 1 | |
| Spouse first & MI / last / SSN | `Page1[0].f1_17[0]` / `f1_18[0]` / `f1_19[0]` | 1 | |
| Street / Apt | `Page1[0].Address_ReadOrder[0].f1_20[0]` / `f1_21[0]` | 1 | |
| City / State / ZIP | `Address_ReadOrder[0].f1_22[0]` / `f1_23[0]` / `f1_24[0]` | 1 | |
| Foreign country / province / postal | `Address_ReadOrder[0].f1_25[0]` / `f1_26[0]` / `f1_27[0]` | 1 | |
| **NEW** "main home in U.S. > half of 2025" box | `Page1[0].c1_5[0]` (on `/1`) | 1 | right margin, y≈147 |
| Pres. Election Campaign You / Spouse | `Page1[0].c1_6[0]` / `c1_7[0]` (on `/1`) | 1 | y≈194.5 |

### Filing status — still FIVE independent `/Btn` boxes, **renamed AND re-valued**

Leaf renamed `c1_3` → **`c1_8`**, container renamed `FilingStatus_ReadOrder` →
**`Checkbox_ReadOrder`**, and the **on-state assignment changed for MFJ/MFS/HOH**:

| Status | TY2025 field suffix | **On-state 2025** | (2024 was) |
|---|---|---|---|
| Single | `Page1[0].Checkbox_ReadOrder[0].c1_8[0]` | `/1` | `/1` |
| MFJ | `Page1[0].Checkbox_ReadOrder[0].c1_8[1]` | **`/2`** | `/3` |
| MFS | `Page1[0].Checkbox_ReadOrder[0].c1_8[2]` | **`/3`** | `/4` |
| HOH | `Page1[0].c1_8[0]` | **`/4`** | `/2` |
| QSS | `Page1[0].c1_8[1]` | `/5` | `/5` |

MFS spouse-name write-in `Page1[0].Checkbox_ReadOrder[0].f1_28[0]`; HOH/QSS child-name
write-in `Page1[0].f1_29[0]` (one shared field in 2025 — 2024 had separate HOH/QSS fields).
NRA/dual-status-alien-spouse box `Page1[0].c1_9[0]` (on `/1`) + name `Page1[0].f1_30[0]`.
**NEW** standalone box y≈424 (below dependents): "filing status is MFS or HOH and you lived
apart from your spouse for the last 6 months of 2025 / legally separated" =
`Page1[0].c1_32[0]` (on `/1`).

### Digital-asset question (leaf renamed `c1_5` → `c1_10`)

| Answer | Field suffix | On-state |
|---|---|---|
| Yes | `Page1[0].c1_10[0]` | `/1` |
| No | `Page1[0].c1_10[1]` | `/2` |

### Dependents grid — **TRANSPOSED in 2025** (attributes = rows, dependents = columns)

Grid capacity still 4 dependents, but the container rows are now the *attributes*; dependent
N occupies x-column N (x≈145–251 / 253–359 / 361–467 / 469–576). New attributes (5) and (6)
were added:

| Attribute row | Dep 1 | Dep 2 | Dep 3 | Dep 4 |
|---|---|---|---|---|
| (1) First name — `Table_Dependents[0].Row1[0].` | `f1_31[0]` | `f1_32[0]` | `f1_33[0]` | `f1_34[0]` |
| (2) Last name — `Row2[0].` | `f1_35[0]` | `f1_36[0]` | `f1_37[0]` | `f1_38[0]` |
| (3) SSN — `Row3[0].` | `f1_39[0]` | `f1_40[0]` | `f1_41[0]` | `f1_42[0]` |
| (4) Relationship — `Row4[0].` | `f1_43[0]` | `f1_44[0]` | `f1_45[0]` | `f1_46[0]` |
| **NEW** (5)(a) lived w/ you > half 2025 — `Row5[0].DependentN[0].` | `c1_12[0]` | `c1_14[0]` | `c1_16[0]` | `c1_18[0]` |
| **NEW** (5)(b) and in the U.S. — `Row5[0].DependentN[0].` | `c1_13[0]` | `c1_15[0]` | `c1_17[0]` | `c1_19[0]` |
| **NEW** (6) full-time student / perm. disabled — `Row6[0].DependentN[0].` | `c1_20[0]`/`c1_21[0]` | `c1_22[0]`/`c1_23[0]` | `c1_24[0]`/`c1_25[0]` | `c1_26[0]`/`c1_27[0]` |
| (7) Credits CTC (`/1`) vs ODC (`/2`) — `Row7[0].DependentN[0].` | `c1_28[0]`/`[1]` | `c1_29[0]`/`[1]` | `c1_30[0]`/`[1]` | `c1_31[0]`/`[1]` |

All (5)/(6) boxes on `/1`. Attribute (7) is now a **two-widget same-leaf pair per dependent**
(`c1_28[0]` on `/1` = child tax credit, `c1_28[1]` on `/2` = credit for other dependents) —
different fill shape than 2024's two independent leaves per row. ">4 dependents" box:
`Page1[0].Dependents_ReadOrder[0].c1_11[0]` (on `/1`).

### Income lines (page 1) — **all leaf numbers shifted by the header additions**

| 2025 Line | Field suffix | Page | Notes |
|---|---|---|---|
| 1a Total W-2 box 1 | `Page1[0].f1_47[0]` | 1 | right col x≈504–576 (2024: f1_32) |
| 1b–1h | `Page1[0].f1_48..f1_53` + `f1_54[0]`(1h type)+`f1_55[0]`(1h amt) | 1 | 1b=f1_48 … 1g=f1_53 |
| 1i nontaxable combat pay | `Page1[0].f1_56[0]` | 1 | mid col x≈410–482 |
| 1z Sum | `Page1[0].f1_57[0]` | 1 | ⚠ `f1_57` was **L12** in 2024 — cross-year key collision |
| 2a Tax-exempt interest | `Page1[0].f1_58[0]` | 1 | mid col x≈252–323 |
| 2b Taxable interest | `Page1[0].f1_59[0]` | 1 | right col |
| 3a Qualified dividends | `Page1[0].f1_60[0]` | 1 | mid col |
| 3b Ordinary dividends | `Page1[0].f1_61[0]` | 1 | right col |
| **NEW 3c** child's-dividends-included boxes | `Page1[0].c1_33[0]` (in 3a, `/1`) / `c1_34[0]` (in 3b, `/1`) | 1 | |
| 4a / 4b IRA | `Page1[0].f1_62[0]` / `f1_63[0]` | 1 | |
| **NEW 4c** Rollover / QCD / other boxes + write-in | `c1_35[0]` / `c1_36[0]` / `c1_37[0]` (all `/1`) + `f1_64[0]` | 1 | |
| 5a / 5b Pensions | `Page1[0].f1_65[0]` / `f1_66[0]` | 1 | |
| **NEW 5c** Rollover / PSO / other + write-in | `c1_38[0]` / `c1_39[0]` / `c1_40[0]` (all `/1`) + `f1_67[0]` | 1 | |
| 6a / 6b Social security | `Page1[0].f1_68[0]` / `f1_69[0]` | 1 | |
| 6c lump-sum election box | `Page1[0].c1_41[0]` (on `/1`) | 1 | |
| **NEW 6d** MFS-lived-apart-entire-year box | `Page1[0].c1_42[0]` (on `/1`) | 1 | |
| **7a Capital gain/(loss)** | `Page1[0].f1_70[0]` | 1 | **the btctax money line** (2024: `Line4a-11_ReadOrder[0].f1_52[0]`; the `Line4a-11_ReadOrder` container is GONE in 2025 — all income lines are bare `Page1[0]`) |
| 7b "Sch D not required" box | `Page1[0].c1_43[0]` (on `/1`) | 1 | |
| **NEW 7b** "Includes child's capital gain or (loss)" box | `Page1[0].c1_44[0]` (on `/1`) | 1 | + write-in `f1_71[0]` (x≈403–475; TODO: per-instructions semantics — child's-gain amount) |
| 8 Additional income (Sch 1 L10) | `Page1[0].f1_72[0]` | 1 | |
| 9 Total income | `Page1[0].f1_73[0]` | 1 | |
| 10 Adjustments (Sch 1 L26) | `Page1[0].f1_74[0]` | 1 | |
| **11a AGI** | `Page1[0].f1_75[0]` | 1 | page 1 ends here |

### Deduction / tax / payments (page 2) — renumbered block

| 2025 Line | Field suffix | Page | Notes |
|---|---|---|---|
| **11b** AGI repeated ("Amount from line 11a") | `Page2[0].f2_01[0]` | 2 | NEW line |
| 12a Someone-can-claim You / Spouse | `Page2[0].c2_1[0]` / `c2_2[0]` (on `/1`) | 2 | std-ded boxes moved p1→p2 |
| 12b Spouse itemizes | `Page2[0].c2_3[0]` (on `/1`) | 2 | |
| 12c You were dual-status alien | `Page2[0].c2_4[0]` (on `/1`) | 2 | split out of 2024's combined box |
| 12d You born before 1/2/1961 / blind | `Page2[0].c2_5[0]` / `c2_6[0]` (on `/1`) | 2 | |
| 12d Spouse born before 1/2/1961 / blind | `Page2[0].c2_7[0]` / `c2_8[0]` (on `/1`) | 2 | |
| **12e Std/itemized deduction** | `Page2[0].f2_02[0]` | 2 | was L12/f1_57 p1 in 2024 |
| **13a QBI deduction** | `Page2[0].f2_03[0]` | 2 | was L13 |
| **13b Additional deductions from Schedule 1-A, line 38** | `Page2[0].f2_04[0]` | 2 | **NEW (OBBBA)** — the Schedule 1-A hook |
| 14 Add 12e+13a+13b | `Page2[0].f2_05[0]` | 2 | |
| 15 Taxable income (11b − 14) | `Page2[0].f2_06[0]` | 2 | |
| 16 Tax | `Page2[0].f2_08[0]` | 2 | boxes 8814/4972/other = `c2_9`/`c2_10`/`c2_11` (on `/1`); "3 =" write-in `f2_07[0]` |
| 17 Sch 2 L3 | `Page2[0].f2_09[0]` | 2 | |
| 18 Add 16+17 | `Page2[0].f2_10[0]` | 2 | |
| 19 CTC/ODC (Sch 8812) | `Page2[0].f2_11[0]` | 2 | |
| 20 Sch 3 L8 | `Page2[0].f2_12[0]` | 2 | |
| 21 Add 19+20 | `Page2[0].f2_13[0]` | 2 | |
| 22 Subtract 21 from 18 | `Page2[0].f2_14[0]` | 2 | |
| 23 Other taxes (Sch 2 L21) | `Page2[0].f2_15[0]` | 2 | |
| 24 Total tax | `Page2[0].f2_16[0]` | 2 | |
| 25a W-2 withholding | `Page2[0].f2_17[0]` | 2 | mid col x≈410–482 |
| 25b 1099 withholding | `Page2[0].f2_18[0]` | 2 | mid col |
| 25c Other withholding | `Page2[0].f2_19[0]` | 2 | mid col |
| 25d Sum 25a–25c | `Page2[0].f2_20[0]` | 2 | right col |
| 26 Estimated payments | `Page2[0].f2_21[0]` | 2 | |
| **NEW** former-spouse SSN (joint est. payments) | `Page2[0].SSN_ReadOrder[0].f2_22[0]` | 2 | text, not money |
| 27a EIC | `Page2[0].f2_23[0]` | 2 | mid col |
| **NEW 27b** clergy-filing-Sch-SE box | `Page2[0].c2_12[0]` (on `/1`) | 2 | |
| **NEW 27c** decline-EIC box | `Page2[0].c2_13[0]` (on `/1`) | 2 | |
| 28 ACTC | `Page2[0].f2_24[0]` | 2 | + **NEW** decline-ACTC box `Line28_ReadOrder[0].c2_14[0]` (on `/1`) |
| 29 American opportunity credit | `Page2[0].f2_25[0]` | 2 | |
| **30 Refundable adoption credit (8839)** | `Page2[0].f2_26[0]` | 2 | was "Reserved" in 2024 |
| 31 Sch 3 L15 | `Page2[0].f2_27[0]` | 2 | |
| 32 Total other payments/refundable credits | `Page2[0].f2_28[0]` | 2 | |
| 33 Total payments | `Page2[0].f2_29[0]` | 2 | |
| 34 Overpaid | `Page2[0].f2_30[0]` | 2 | |
| 35a Refund | `Page2[0].f2_31[0]` | 2 | 8888 box `c2_15[0]` (on `/1`) |
| 35b/c/d routing / type / account | `RoutingNo[0].f2_32[0]` / `c2_16[0]`(`/1`=Checking)/`c2_16[1]`(`/2`=Savings) / `AccountNo[0].f2_33[0]` | 2 | |
| 36 Applied to 2026 est. tax | `Page2[0].f2_34[0]` | 2 | mid col |
| 37 Amount you owe | `Page2[0].f2_35[0]` | 2 | |
| 38 Est.-tax penalty | `Page2[0].f2_36[0]` | 2 | mid col |

(Third-party designee `c2_17[0]`/`[1]` = Yes `/1`/No `/2`, `f2_37..f2_39`; sign/preparer block
`f2_40..f2_51`, `c2_18[0]` self-employed — out of fill scope, listed for completeness.)

---

## Schedule 1 (TY2025) — root **`topmostSubform[0]`** (FLIPPED from `form1[0]`) — 2 pages

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN.

**NEW top-of-form line (above Part I):** "For 2025, enter the amount reported to you on
Form(s) 1099-K that was included in error or for personal items sold at a loss" =
`Page1[0].f1_03[0]` (right col). This shifts every Part I leaf by +1 vs 2024 through 8a.

### Part I — Additional Income (page 1, `f1_`)

| Line | Field suffix | Notes |
|---|---|---|
| 1 State refund | `Page1[0].f1_04[0]` | right col x≈504–576 (2024: same suffix by coincidence) |
| 2a Alimony received | `Page1[0].f1_05[0]` | |
| 2b Date (text) | `Page1[0].f1_06[0]` | |
| 3 Business income | `Page1[0].f1_07[0]` | |
| 4 Other gains | `Page1[0].f1_08[0]` | **NEW** source boxes: 4797 = `c1_1[0]`, 4684 = `c1_2[0]` (on `/1`) |
| 5 Rental/Sch E | `Page1[0].f1_09[0]` | |
| 6 Farm | `Page1[0].f1_10[0]` | |
| 7 Unemployment | `Page1[0].f1_12[0]` | **NEW** repaid-overpayment box `Line7_ReadOrder[0].c1_3[0]` (on `/1`) + repaid-amount write-in `Line7_ReadOrder[0].f1_11[0]` |
| 8a NOL | `Page1[0].Line8a_ReadOrder[0].f1_13[0]` | negative `( )`, mid col x≈414–478 (2024: f1_12) |
| 8b–8v | `Page1[0].f1_14[0]` … `f1_34[0]` | 8b=f1_14, 8c=f1_15 … 8u=f1_33, 8v=f1_34 (all +1 vs 2024) |
| 8z desc (text) | `Page1[0].Line8z_ReadOrder[0].f1_35[0]` | **single** desc field in 2025 (2024 had two: f1_34+f1_35) |
| 8z amount | `Page1[0].f1_36[0]` | mid col |
| 9 Total other income | `Page1[0].f1_37[0]` | |
| 10 → 1040 L8 | `Page1[0].f1_38[0]` | right col (same suffix as 2024 by coincidence — root differs!) |

### Part II — Adjustments (page 2, `f2_`)

| Line | Field suffix | Notes |
|---|---|---|
| 11 Educator | `Page2[0].f2_01[0]` | |
| 12 Form 2106 | `Page2[0].f2_02[0]` | |
| 13 HSA (8889) | `Page2[0].f2_03[0]` | |
| 14 Moving (3903) | `Page2[0].f2_04[0]` | **NEW** "claiming only storage fees" box `c2_1[0]` (on `/1`) |
| 15 ½ SE tax | `Page2[0].f2_05[0]` | |
| 16 SEP/SIMPLE | `Page2[0].f2_06[0]` | |
| 17 SE health ins | `Page2[0].f2_07[0]` | |
| 18 Early-withdrawal penalty | `Page2[0].f2_08[0]` | |
| 19a / 19b SSN / 19c date | `f2_09[0]` / `Line19b_CombField[0].f2_10[0]` / `f2_11[0]` | |
| 20 IRA deduction | `Page2[0].f2_12[0]` | **NEW** MFS-lived-apart box `c2_2[0]` (on `/1`) |
| 21 Student-loan interest | `Page2[0].f2_13[0]` | |
| 22 Reserved | `Page2[0].f2_14[0]` | |
| 23 Archer MSA | `Page2[0].f2_15[0]` | |
| 24a Jury duty | `Page2[0].Line24a_ReadOrder[0].f2_16[0]` | mid col x≈410–482 |
| 24b–24k | `Page2[0].f2_17[0]` … `f2_26[0]` | 24b=f2_17 … 24k=f2_26 (same as 2024) |
| 24z desc (text) | `Page2[0].Line24z_ReadOrder[0].f2_27[0]` | single desc field (2024 had two) |
| 24z amount | `Page2[0].f2_28[0]` | (2024: f2_29) |
| 25 Total other adjustments | `Page2[0].f2_29[0]` | (2024: f2_30) |
| 26 → 1040 L10 | `Page2[0].f2_30[0]` | (2024: f2_31) |

---

## Schedule 2 (TY2025) — root `form1[0]` — 2 pages · **page split unchanged (L16 | L17)**

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN. Part I was rebuilt for 2025
(1b/1c = clean-vehicle credit repayments, 1d/1e/1f = Form 4255 EP/EPE lines with 4-state
checkbox groups `c1_1`/`c1_2`, on-states `/1`–`/4`) — context only, not in btctax scope.

### In-scope lines (+ context spine)

| Line | Field suffix | Page | Notes |
|---|---|---|---|
| 1z Sum 1a–1y | `Page1[0].f1_11[0]` | 1 | context (2024: same) |
| 2 AMT (6251) | `Page1[0].f1_12[0]` | 1 | context |
| 3 → 1040 L17 | `Page1[0].f1_13[0]` | 1 | context |
| **4 SE tax (Sch SE)** | `Page1[0].f1_15[0]` | 1 | right col. **shifted from f1_14** — 2025 adds exemption boxes: 4361 = `Line4_ReadOrder[0].c1_3[0]`, 4029 = `c1_4[0]`, other = `c1_5[0]` (all `/1`) + form write-in `Line4_ReadOrder[0].f1_14[0]` |
| 5 Unreported-tip SS/Medicare (4137) | `Page1[0].f1_16[0]` | 1 | mid col |
| 6 Uncollected SS/Medicare wages (8919) | `Page1[0].f1_17[0]` | 1 | mid col |
| 7 Add 5+6 | `Page1[0].f1_18[0]` | 1 | |
| 8 Additional tax on IRAs etc. (5329) | `Page1[0].f1_19[0]` | 1 | not-required box `Line8_ReadOrder[0].c1_6[0]` (on `/1`) |
| 9 Household employment (Sch H) | `Page1[0].f1_20[0]` | 1 | |
| 10 Reserved | `Page1[0].f1_21[0]` | 1 | |
| **11 Additional Medicare (8959)** | `Page1[0].f1_22[0]` | 1 | right col (2024: f1_21) |
| **12 NIIT (8960)** | `Page1[0].f1_23[0]` | 1 | right col (2024: f1_22) |
| 13 Uncollected SS/Medicare (W-2 box 12) | `Page1[0].f1_24[0]` | 1 | (2024: f1_23) |
| 17a–17q other-additional-taxes | `Page2[0].f2_02..f2_18` | 2 | 17a amt = `Line17a_ReadOrder[0].f2_02[0]`, desc = `Line17a_ReadOrder[0].Line17_ReadOrder[0].f2_01[0]`; 17b=f2_03 … 17q=f2_18 |
| 17z desc / amount | `Line17z_ReadOrder[0].f2_19[0]` / `f2_20[0]` | 2 | single desc field |
| 18 Total additional taxes | `Page2[0].f2_21[0]` | 2 | (2024: f2_22) |
| 19 Recapture of net EPE (4255) | `Page2[0].f2_22[0]` | 2 | 2025 line content differs from 2024 L19 |
| 20 Section 965 installment | `Page2[0].f2_23[0]` | 2 | mid col (2024: Line20_ReadOrder f2_24) |
| **21 Total other taxes → 1040 L23** | `Page2[0].f2_24[0]` | **2** | **shifted from f2_25** — page-2 leaf, `f2_` prefix |

---

## Schedule 3 (TY2025) — root `topmostSubform[0]` — 1 page

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN. Two fields net **removed**
(37 vs 39): the 6z and 13z write-ins each collapsed from two text leaves to one.

### Part I — Nonrefundable credits

| Line | Field suffix | Notes |
|---|---|---|
| 1 Foreign tax credit | `Page1[0].f1_03[0]` | right col x≈504–576 |
| 2 Dependent care (2441) | `Page1[0].f1_04[0]` | |
| 3 Education (8863) | `Page1[0].f1_05[0]` | |
| 4 Retirement savings (8880) | `Page1[0].f1_06[0]` | |
| 5a Residential clean energy | `Page1[0].f1_07[0]` | |
| 5b Energy-efficient home | `Page1[0].f1_08[0]` | |
| 6a General business (3800) | `Page1[0].Line6a_ReadOrder[0].f1_09[0]` | mid col x≈410–482 |
| 6b–6m | `Page1[0].f1_10[0]` … `f1_21[0]` | 6b=f1_10 … 6m=f1_21 (same as 2024) |
| 6z desc (text) | `Page1[0].Line6z_ReadOrder[0].f2_22[0]` | **⚠ the `f2_N`-on-a-1-page-form quirk persists but RENAMED** (2024: `f2_23`) — single desc field now |
| 6z amount | `Page1[0].f1_23[0]` | (2024: f1_24) |
| 7 Total other nonref credits | `Page1[0].f1_24[0]` | (2024: f1_25) |
| 8 → 1040 L20 | `Page1[0].f1_25[0]` | right col (2024: f1_26) |

### Part II — Refundable credits / payments

| Line | Field suffix | Notes |
|---|---|---|
| 9 Net PTC (8962) | `Page1[0].f1_26[0]` | (2024: f1_27) |
| 10 Extension payment | `Page1[0].f1_27[0]` | |
| 11 Excess SS / RRTA | `Page1[0].f1_28[0]` | |
| 12 Fuel credit (4136) | `Page1[0].f1_29[0]` | |
| 13a Form 2439 | `Page1[0].Line13_ReadOrder[0].f1_30[0]` | mid col |
| 13b Repayment-years credit | `Page1[0].f1_31[0]` | |
| 13c Net elective payment (3800) | `Page1[0].f1_32[0]` | |
| 13d Deferred 965 | `Page1[0].f1_33[0]` | |
| 13z desc / amount | `Line13z_ReadOrder[0].f1_34[0]` / `Page1[0].f1_35[0]` | single desc field |
| 14 Total other payments | `Page1[0].f1_36[0]` | (2024: f1_38) |
| 15 → 1040 L31 | `Page1[0].f1_37[0]` | right col (2024: f1_39) |

---

## Schedule A (TY2025) — root **`form1[0]`** (FLIPPED from `topmostSubform[0]`) — 1 page

Leaf names remain **`f1_N` with no leading zero**. Identity: `Page1[0].f1_1[0]` name,
`Page1[0].f1_2[0]` SSN. **x-columns moved:** item col now **x≈410–482** (2024: 417–489),
result col x≈504–576 (unchanged), L2 inline now **x≈317–388** (2024: 331–402). Four fields
net removed (33 vs 37): lines 6 / 8b / 16 write-in text collapsed to one leaf each.

| Line | Field suffix | x-col | Notes |
|---|---|---|---|
| 1 Medical expenses | `Page1[0].f1_3[0]` | item | |
| 2 AGI — now "from 1040 **line 11b**" | `Page1[0].Line2_ReadOrder[0].f1_4[0]` | inline x≈317–388 | **NEW container `Line2_ReadOrder[0]`**; reads 1040 L11b |
| 3 Line 2 × 7.5% | `Page1[0].f1_5[0]` | item | |
| 4 Subtract 3 from 1 | `Page1[0].f1_6[0]` | result | |
| 5a Income or sales tax | `Page1[0].f1_7[0]` | item | sales-tax election box `Page1[0].c1_1[0]` (on `/1`) |
| 5b Real-estate taxes | `Page1[0].f1_8[0]` | item | |
| 5c Personal-property taxes | `Page1[0].f1_9[0]` | item | |
| 5d Add 5a–5c | `Page1[0].f1_10[0]` | item | |
| 5e SALT cap | `Page1[0].f1_11[0]` | item | **2025 wording: "smaller of line 5d or $40,000 ($20,000 MFS). If 1040 line 11b > $500,000 ($250,000 MFS) … see instructions"** — OBBBA cap + phase-out confirmed on the FINAL form |
| 6 Other taxes — type (text) | `Page1[0].f1_12[0]` | left desc | **single** write-in (2024 had two) |
| 6 amount | `Page1[0].f1_13[0]` | item | |
| 7 Add 5e + 6 | `Page1[0].f1_14[0]` | result | |
| 8 "not used to buy/build/improve" box | `Page1[0].Line8_ReadOrder[0].c1_2[0]` | — | on `/1` |
| 8a Home-mtg int + points on 1098 | `Page1[0].f1_15[0]` | item | |
| 8b amount (not on 1098) | `Page1[0].f1_17[0]` | item | ⚠ ordering anomaly **reversed vs 2024**: desc leaf `f1_16` now numbered BEFORE amount `f1_17` |
| 8b payer name/ID/addr (text) | `Page1[0].Line8b_ReadOrder[0].f1_16[0]` | left | **single** write-in (2024 had two) |
| 8c Points not on 1098 | `Page1[0].f1_18[0]` | item | |
| 8d Reserved | `Page1[0].f1_19[0]` | item | |
| 8e Add 8a–8c | `Page1[0].f1_20[0]` | item | |
| 9 Investment interest (4952) | `Page1[0].f1_21[0]` | item | |
| 10 Add 8e + 9 | `Page1[0].f1_22[0]` | result | |
| 11 Gifts by cash/check | `Page1[0].f1_23[0]` | item | 2025 wording: "If you made any gift of $250 or more, see instructions" |
| 12 Other than cash/check (8283 if > $500) | `Page1[0].f1_24[0]` | item | 2024 charity structure retained — no TY2026 0.5%-floor lines |
| 13 Carryover | `Page1[0].f1_25[0]` | item | |
| 14 Add 11–13 | `Page1[0].f1_26[0]` | result | |
| 15 Casualty/theft | `Page1[0].f1_27[0]` | result | |
| 16 Other — type (text) | `Page1[0].f1_28[0]` | left | **single** write-in (2024 had three) |
| 16 amount | `Page1[0].f1_29[0]` | result | |
| **17 Total itemized → 1040 L12e** | `Page1[0].f1_30[0]` | result | form text cites "line 12e" ✓ (2024: f1_34) |
| 18 elect-to-itemize box | `Page1[0].Line18_ReadOrder[0].c1_3[0]` | — | on `/1` |

---

## Schedule B (TY2025) — root `topmostSubform[0]` — 1 page · structure ≡ 2024 (72 = 72)

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN.
Grid columns unchanged: payer-name x≈130–461, amount x≈490–576.

### Part I — Interest (line 1): 14 rows

Row 1: name `Page1[0].Line1_ReadOrder[0].f1_03[0]`, amount `Page1[0].f1_04[0]`.
Rows 2–14: name = odd `f1_05..f1_29`, amount = even `f1_06..f1_30` (identical to 2024).

| Line | Field suffix |
|---|---|
| 2 Add line-1 amounts | `Page1[0].f1_31[0]` |
| 3 Excludable savings-bond interest (8815) | `Page1[0].f1_32[0]` |
| 4 → 1040 L2b | `Page1[0].f1_33[0]` |

### Part II — Ordinary dividends (line 5): 15 rows

Row 1: name `Page1[0].ReadOrderControl[0].f1_34[0]`, amount `Page1[0].f1_35[0]`.
Rows 2–15: name = even `f1_36..f1_62`, amount = odd `f1_37..f1_63` (identical to 2024).

| Line | Field suffix |
|---|---|
| 6 → 1040 L3b | `Page1[0].f1_64[0]` |

### Part III — Foreign accounts & trusts (`/1`=Yes, `/2`=No)

| Line | Yes / No fields | Notes |
|---|---|---|
| 7a financial interest / signature authority | `Page1[0].TagcorrectingSubform[0].c1_1[0]` / `c1_1[1]` | **⚠ NEW container `TagcorrectingSubform[0]`** (2024: bare `Page1[0].c1_1`) |
| 7a-ii required to file FinCEN 114 | `Page1[0].c1_2[0]` / `c1_2[1]` | bare Page1, unchanged |
| 7b foreign country names (text) | `Page1[0].f1_65[0]` (+ `f1_66[0]` cont.) | |
| 8 foreign trust | `Page1[0].c1_3[0]` / `c1_3[1]` | |

---

## Cross-year hazards surfaced by this extraction

1. **Same suffix, different meaning.** 1040 `Page1[0].f1_57[0]` = **L12 deduction in 2024**
   but **L1z wages-sum in 2025**. Sch 2 `f1_14` = L4 amount (2024) vs L4 form-number write-in
   (2025). A 2024 map applied to a 2025 PDF fills silently and wrongly — the per-(form,year)
   map + geometric read-back is the only defense, exactly as scoped.
2. **Filing-status on-states are re-assigned, not just renamed** (MFJ `/3`→`/2`, MFS `/4`→`/3`,
   HOH `/2`→`/4`). The map must carry (FQN, on-state) pairs per status per year; never reuse
   2024 on-state constants.
3. **1040 income lines lost their `Line4a-11_ReadOrder[0]` container** — every 2025 income
   leaf is bare `Page1[0]`. Container prefixes remain per-line, per-year facts.
4. **Sch 3 quirk migrates:** the 1-page `f2_N` leaf is now `Line6z_ReadOrder[0].f2_22[0]`
   (was `f2_23`). Copy verbatim; never normalize.
5. **Sch B is the only byte-stable form** (identical leaf layout both years, only the
   `TagcorrectingSubform[0]` container on 7a changed). Grid capacities still 14 interest /
   15 dividend rows — overflow plan unchanged.
6. **Geometric read-back deltas:** Sch A item column moved ≈7pt left (410–482) and L2 inline
   moved to 317–388; 1040 page 2 now opens at y≈36 with L11b; the dependents area on page 1
   is a 4-column × 7-row checkbox/text lattice (same-y multi-column, needs the x-band
   disambiguation already specced for Sch A).
7. **Write-in desc leaves collapsed** (Sch 1 8z/24z, Sch 3 6z/13z, Sch A 6/8b/16: two-or-three
   text leaves → one). btctax fill policy for these stays "leave blank" per deep/03 TODO.

## TODO / unresolved

- **TODO:** 1040 `Page1[0].f1_71[0]` (x≈403–475, on the 7b checkbox row) — geometry says it
  belongs to line 7b; presumed child's-capital-gain amount write-in. Confirm exact semantics
  against the final i1040 instructions before wiring (btctax never fills it either way).
- **TODO:** Schedule 1-A (new TY2025 form, 1040 L13b source) was **not extracted here** — it
  is outside the six-form set assigned to F5. If Spec pulls L13b into scope, a seventh
  per-year map (f1040s1a) needs its own extraction pass.
- **Not re-verified:** checkbox export values on out-of-scope blocks (direct-deposit,
  third-party designee) beyond what the acroform dump shows; harmless — btctax never sets them.
