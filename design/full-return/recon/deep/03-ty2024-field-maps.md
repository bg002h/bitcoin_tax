# Deep Recon 03 — TY2024 Field-Map Skeletons (extracted from the official PDFs)

**Agent:** Opus deep-dive recon, round 2. **Date:** 2026-07-11.
**Depends on:** `../03-irs-fillable-pdfs.md` (feasibility GREEN), `../01-form-graph.md` §2
(TY2024 line-flow). This artifact turns that feasibility into **concrete, PDF-verified
field-name skeletons** for the six TY2024 forms.

## Method — real extraction, no invented names

All six PDFs pulled from `https://www.irs.gov/pub/irs-prior/` (`f1040--2024.pdf`,
`f1040s1--2024.pdf`, `f1040s2--2024.pdf`, `f1040s3--2024.pdf`, `f1040sa--2024.pdf`,
`f1040sb--2024.pdf`; all HTTP 200, `application/pdf`). Field names, types, page, checkbox
on-states and the parent/subform chain came from **`qpdf --json --json-key=acroform`**
(v2, for `fullname`/`pageposfrom1`/type) joined to **`qpdf --json=1 --json-key=objects`**
(v1, for widget `/Rect` and `/AP /N` on-states). Line numbers were assigned **geometrically**:
each field's widget `/Rect` (converted to top-origin, page height 792pt) was matched to the
printed line label/caption on the same visual row via **`pdftotext -bbox`**. Extracted field
counts match the first pass exactly (1040=141, S1=69, S2=60, S3=39, SA=37, SB=72).

> **No `/TU` tooltips exist on these forms** (the descriptive tooltips were dropped when XFA
> was flattened — `grep /TU` = 0 hits on every file). Line→field mapping is therefore
> **geometry-derived**, exactly as the geometric read-back oracle will re-derive it. Every
> field name below is copied from the actual AcroForm; nothing is fabricated.

**Field-name convention.** Every table gives the FQN **suffix after the root**. The full
map key = `<ROOT> + "." + <suffix>` (e.g. root `topmostSubform[0]` + suffix
`Page1[0].Line4a-11_ReadOrder[0].f1_52[0]` = the exact existing-map key for 1040 line 7).
Money leaves are `f<page>_NN[0]`; checkboxes `c<page>_NN[0]`. **Page-2 fields use the `f2_`/
`c2_` prefix** (this is the page indicator, independent of any subform container).

---

## The six TY2024 root container FQNs (quote these into each map)

| Form | **Root FQN (TY2024)** | Pages | In-scope leaf prefix |
|---|---|---|---|
| Form 1040 | `topmostSubform[0]` | 2 | `Page1[0].` / `Page2[0].` |
| Schedule 1 | `form1[0]` | 2 | `Page1[0].` / `Page2[0].` |
| Schedule 2 | `form1[0]` | 2 | `Page1[0].` / `Page2[0].` |
| Schedule 3 | `topmostSubform[0]` | 1 | `Page1[0].` |
| Schedule A | `topmostSubform[0]` | 1 | `Page1[0].` |
| Schedule B | `topmostSubform[0]` | 1 | `Page1[0].` |

These are the **exact strings** observed in the 2024 dumps (Schedule 1 & 2 are `form1[0]`,
the other four are `topmostSubform[0]`), confirming R1 of the first pass. **Never hardcode
`topmostSubform[0]`** — S1/S2 would silently mis-key.

---

## Form 1040 — root `topmostSubform[0]`

### Identity / header (page 1)

| Field | Suffix | Notes |
|---|---|---|
| Tax-year begin/mid/end (fiscal) | `Page1[0].f1_01[0]` / `f1_02[0]` / `f1_03[0]` | usually blank for calendar-year |
| Your first name & MI | `Page1[0].f1_04[0]` | |
| Your last name | `Page1[0].f1_05[0]` | |
| Your SSN | `Page1[0].f1_06[0]` | |
| Spouse first name & MI | `Page1[0].f1_07[0]` | |
| Spouse last name | `Page1[0].f1_08[0]` | |
| Spouse SSN | `Page1[0].f1_09[0]` | |
| Home address (no. & street) | `Page1[0].Address_ReadOrder[0].f1_10[0]` | |
| Apt no. | `Page1[0].Address_ReadOrder[0].f1_11[0]` | |
| City/town | `Page1[0].Address_ReadOrder[0].f1_12[0]` | |
| State | `Page1[0].Address_ReadOrder[0].f1_13[0]` | |
| ZIP | `Page1[0].Address_ReadOrder[0].f1_14[0]` | |
| Foreign country / province / postal | `Address_ReadOrder[0].f1_15[0]` / `f1_16[0]` / `f1_17[0]` | |
| Pres. Election Campaign You / Spouse | `Page1[0].c1_1[0]` / `c1_2[0]` (on `/1`) | $3 boxes, no tax effect |

### Filing-status checkboxes — **FIVE INDEPENDENT `/Btn` boxes (NOT a radio group)**

Each is a standalone checkbox with a single on-state; the leaf name `c1_3` is **reused across
two containers**, so the FQN is what disambiguates. App must enforce mutual exclusivity.

| Status | Field suffix | On-state | x |
|---|---|---|---|
| Single | `Page1[0].FilingStatus_ReadOrder[0].c1_3[0]` | `/1` | 103 |
| MFJ | `Page1[0].FilingStatus_ReadOrder[0].c1_3[1]` | `/3` | 103 |
| MFS | `Page1[0].FilingStatus_ReadOrder[0].c1_3[2]` | `/4` | 103 |
| HOH | `Page1[0].c1_3[0]` | `/2` | 369 |
| QSS | `Page1[0].c1_3[1]` | `/5` | 369 |

HOH/QSS write-in name field (child-not-dependent / QSS): `Page1[0].f1_18[0]` (HOH) and
`Page1[0].f1_19[0]` (QSS); nonresident-alien-spouse box `Page1[0].c1_4[0]` (on `/1`).

### Digital-asset question (unchanged from existing map)

| Answer | Field suffix | On-state |
|---|---|---|
| Yes | `Page1[0].c1_5[0]` | `/1` |
| No | `Page1[0].c1_5[1]` | `/2` |

### Standard-deduction / age-blind checkboxes (all `/Btn`, on `/1`)

| Concept | Field suffix |
|---|---|
| Someone can claim **You** as dependent | `Page1[0].c1_6[0]` |
| Someone can claim **Your spouse** | `Page1[0].c1_7[0]` |
| Spouse itemizes / dual-status alien | `Page1[0].c1_8[0]` |
| You born before Jan 2, 1960 | `Page1[0].c1_9[0]` |
| You blind | `Page1[0].c1_10[0]` |
| Spouse born before Jan 2, 1960 | `Page1[0].c1_11[0]` |
| Spouse blind | `Page1[0].c1_12[0]` |

### Dependents grid — **Row 1** (grid capacity = 4 rows `Row1`..`Row4`)

| Column | Field suffix (Row 1) |
|---|---|
| (1) Name | `Page1[0].Table_Dependents[0].Row1[0].f1_20[0]` |
| (2) SSN | `Page1[0].Table_Dependents[0].Row1[0].f1_21[0]` |
| (3) Relationship | `Page1[0].Table_Dependents[0].Row1[0].f1_22[0]` |
| (4) Child Tax Credit box | `Page1[0].Table_Dependents[0].Row1[0].c1_14[0]` (on `/1`) |
| (4) Credit-Other-Dep box | `Page1[0].Table_Dependents[0].Row1[0].c1_15[0]` (on `/1`) |

Rows 2–4: text `f1_23..f1_31`, boxes `c1_16..c1_21` (Row2 = c1_16/c1_17, Row3 = c1_18/c1_19,
Row4 = c1_20/c1_21). ">4 dependents" box: `Page1[0].Dependents_ReadOrder[0].c1_13[0]` (on `/1`).

### Income / deduction / tax lines (in-scope money leaves)

| Line | Field suffix | Page | Notes |
|---|---|---|---|
| 1a Total W-2 box 1 | `Page1[0].f1_32[0]` | 1 | amount col x≈504–576 |
| 1z Sum 1a–1h | `Page1[0].f1_41[0]` | 1 | (1b–1i = f1_33..f1_40) |
| 2a Tax-exempt interest | `Page1[0].f1_42[0]` | 1 | **mid col** x≈252–323 |
| 2b Taxable interest | `Page1[0].f1_43[0]` | 1 | right col x≈504–576 |
| 3a Qualified dividends | `Page1[0].f1_44[0]` | 1 | mid col |
| 3b Ordinary dividends | `Page1[0].f1_45[0]` | 1 | right col |
| 7 Capital gain/(loss) | `Page1[0].Line4a-11_ReadOrder[0].f1_52[0]` | 1 | **matches existing f1040 map** |
| 7 "Sch D not required" box | `Page1[0].Line4a-11_ReadOrder[0].c1_23[0]` | 1 | on `/1` |
| 8 Additional income (Sch 1 L10) | `Page1[0].Line4a-11_ReadOrder[0].f1_53[0]` | 1 | |
| 9 Total income | `Page1[0].Line4a-11_ReadOrder[0].f1_54[0]` | 1 | |
| 10 Adjustments (Sch 1 L26) | `Page1[0].Line4a-11_ReadOrder[0].f1_55[0]` | 1 | |
| 11 AGI | `Page1[0].Line4a-11_ReadOrder[0].f1_56[0]` | 1 | |
| 12 Std/itemized deduction | `Page1[0].f1_57[0]` | 1 | **bare Page1** (not in the ReadOrder subform) |
| 13 QBI deduction | `Page1[0].f1_58[0]` | 1 | |
| 14 Add 12+13 | `Page1[0].f1_59[0]` | 1 | |
| 15 Taxable income | `Page1[0].f1_60[0]` | 1 | |
| 16 Tax | `Page2[0].f2_02[0]` | 2 | (`f2_01[0]` = the "3=" other-form write-in box; boxes 8814/4972/other = `c2_1/c2_2/c2_3`) |
| 17 Sch 2 L3 | `Page2[0].f2_03[0]` | 2 | |
| 18 Add 16+17 | `Page2[0].f2_04[0]` | 2 | |
| 22 Subtract 21 from 18 | `Page2[0].f2_08[0]` | 2 | |
| 23 Other taxes (Sch 2 L21) | `Page2[0].f2_09[0]` | 2 | |
| 24 Total tax | `Page2[0].f2_10[0]` | 2 | |
| 25a W-2 withholding | `Page2[0].f2_11[0]` | 2 | **mid col** x≈410–482 |
| 25b 1099 withholding | `Page2[0].f2_12[0]` | 2 | mid col |
| 25c Other withholding (8959 L24) | `Page2[0].f2_13[0]` | 2 | mid col |
| 25d Sum 25a–25c | `Page2[0].f2_14[0]` | 2 | right col x≈504–576 |
| 26 Estimated payments | `Page2[0].f2_15[0]` | 2 | |
| 33 Total payments | `Page2[0].f2_22[0]` | 2 | |
| 34 Overpaid | `Page2[0].f2_23[0]` | 2 | |
| 35a Refund | `Page2[0].f2_24[0]` | 2 | Form 8888 box = `Page2[0].c2_4[0]` (on `/1`) |
| 37 Amount you owe | `Page2[0].f2_28[0]` | 2 | |

---

## Schedule 1 — root `form1[0]` — 2 pages

Identity: `Page1[0].f1_01[0]` = name(s) shown on 1040, `Page1[0].f1_02[0]` = your SSN.

### Part I — Additional Income (page 1, `f1_`)

| Line | Field suffix | Notes |
|---|---|---|
| 1 State refund | `Page1[0].f1_04[0]` | amount col x≈504–576 |
| 2a Alimony received | `Page1[0].f1_05[0]` | |
| 2b Date (text) | `Page1[0].f1_06[0]` | date write-in |
| 3 Business income | `Page1[0].f1_07[0]` | |
| 4 Other gains | `Page1[0].f1_08[0]` | |
| 5 Rental/Sch E | `Page1[0].f1_09[0]` | |
| 6 Farm | `Page1[0].f1_10[0]` | |
| 7 Unemployment | `Page1[0].f1_11[0]` | |
| 8a NOL | `Page1[0].Line8a_ReadOrder[0].f1_12[0]` | negative `( )` field, mid col x≈414–478 |
| 8b–8v | `Page1[0].f1_13[0]` … `f1_33[0]` | one field/letter (8b=f1_13, 8c=f1_14, … 8v=f1_33), mid col x≈410–482 |
| 8z type (write-in text) | `Page1[0].Line8z_ReadOrder[0].f1_34[0]` | "List type" (x≈215–382) |
| 8z desc (write-in text) | `Page1[0].Line8z_ReadOrder[0].f1_35[0]` | 2nd desc line (x≈65–382) |
| 8z amount | `Page1[0].f1_36[0]` | mid col |
| 9 Total other income | `Page1[0].f1_37[0]` | |
| 10 → 1040 L8 | `Page1[0].f1_38[0]` | right col |

### Part II — Adjustments (page 2, `f2_`)

| Line | Field suffix | Notes |
|---|---|---|
| 11 Educator | `Page2[0].f2_01[0]` | |
| 12 Form 2106 | `Page2[0].f2_02[0]` | |
| 13 HSA (8889) | `Page2[0].f2_03[0]` | |
| 14 Moving (3903) | `Page2[0].f2_04[0]` | |
| 15 ½ SE tax | `Page2[0].f2_05[0]` | |
| 16 SEP/SIMPLE | `Page2[0].f2_06[0]` | |
| 17 SE health ins | `Page2[0].f2_07[0]` | |
| 18 Early-withdrawal penalty | `Page2[0].f2_08[0]` | |
| 19a Alimony paid | `Page2[0].f2_09[0]` | |
| 19b Recipient SSN (text) | `Page2[0].Line19b_CombField[0].f2_10[0]` | |
| 19c Date (text) | `Page2[0].f2_11[0]` | |
| 20 IRA | `Page2[0].f2_12[0]` | |
| 21 Student-loan int | `Page2[0].f2_13[0]` | |
| 22 Reserved | `Page2[0].f2_14[0]` | |
| 23 Archer MSA | `Page2[0].f2_15[0]` | |
| 24a Jury duty | `Page2[0].Line24a_ReadOrder[0].f2_16[0]` | mid col x≈410–482 |
| 24b–24k | `Page2[0].f2_17[0]` … `f2_26[0]` | 24b=f2_17 … 24k=f2_26 |
| 24z type (text) | `Page2[0].Line24z_ReadOrder[0].f2_27[0]` | |
| 24z desc (text) | `Page2[0].Line24z_ReadOrder[0].f2_28[0]` | |
| 24z amount | `Page2[0].f2_29[0]` | |
| 25 Total other adjustments | `Page2[0].f2_30[0]` | |
| 26 → 1040 L10 | `Page2[0].f2_31[0]` | |

---

## Schedule 2 — root `form1[0]` — 2 pages · **page split is between L16 (p1) and L17 (p2)**

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN.

| Line | Field suffix | Page | Notes |
|---|---|---|---|
| 4 SE tax (Sch SE) | `Page1[0].f1_14[0]` | 1 | right col x≈504–576 |
| 11 Additional Medicare (8959) | `Page1[0].f1_21[0]` | 1 | right col |
| 12 NIIT (8960) | `Page1[0].f1_22[0]` | 1 | right col |
| 13 Uncollected SS/Medicare (W-2 box 12) | `Page1[0].f1_23[0]` | 1 | right col |
| 21 Total other taxes → 1040 L23 | `Page2[0].f2_25[0]` | **2** | **`f2_` — this is the whole reason to note the split** |

Context spine (not required but adjacent): L1z=`f1_11`, L2 AMT=`f1_12`, L3→1040 L17=`f1_13`
(all p1); L18 total-additional-taxes=`Page2[0].f2_22[0]`, L19=`f2_23`, L20=`Page2[0].Line20_ReadOrder[0].f2_24[0]`
(all p2). Lines 5–16 (p1) and 17a–20 (p2) carry lettered sub-lines with the same fixed-field
pattern as Schedule 1.

---

## Schedule 3 — root `topmostSubform[0]` — 1 page (all `/Tx`, cleanest form)

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN.

### Part I — Nonrefundable credits

| Line | Field suffix | Notes |
|---|---|---|
| 1 Foreign tax credit | `Page1[0].f1_03[0]` | right col x≈504–576 |
| 2 Dependent care (2441) | `Page1[0].f1_04[0]` | |
| 3 Education (8863) | `Page1[0].f1_05[0]` | |
| 4 Retirement savings (8880) | `Page1[0].f1_06[0]` | |
| 5a Residential clean energy | `Page1[0].f1_07[0]` | |
| 5b Energy-efficient home | `Page1[0].f1_08[0]` | |
| 6a | `Page1[0].Line6a_ReadOrder[0].f1_09[0]` | mid col x≈410–482 |
| 6b–6m | `Page1[0].f1_10[0]` … `f1_21[0]` | 6b=f1_10 … 6m=f1_21 |
| 6z type (text) | `Page1[0].Line6z_ReadOrder[0].f1_22[0]` | |
| 6z desc (text) | `Page1[0].Line6z_ReadOrder[0].f2_23[0]` | **⚠ leaf is `f2_23` on a 1-page form** (IRS numbering quirk) |
| 6z amount | `Page1[0].f1_24[0]` | |
| 7 Total other nonref credits | `Page1[0].f1_25[0]` | |
| 8 → 1040 L20 | `Page1[0].f1_26[0]` | right col |

### Part II — Refundable credits / payments

| Line | Field suffix | Notes |
|---|---|---|
| 9 Net PTC (8962) | `Page1[0].f1_27[0]` | |
| 10 Extension payment | `Page1[0].f1_28[0]` | |
| 11 Excess SS / RRTA | `Page1[0].f1_29[0]` | |
| 12 Fuel credit (4136) | `Page1[0].f1_30[0]` | |
| 13a | `Page1[0].Line13_ReadOrder[0].f1_31[0]` | mid col |
| 13b–13d | `Page1[0].f1_32[0]` / `f1_33[0]` / `f1_34[0]` | |
| 13z type/desc/amount | `Line13z_ReadOrder[0].f1_35[0]` / `f1_36[0]` / `Page1[0].f1_37[0]` | |
| 14 Total other payments | `Page1[0].f1_38[0]` | |
| 15 → 1040 L31 | `Page1[0].f1_39[0]` | right col |

---

## Schedule A — root `topmostSubform[0]` — 1 page · **leaf names are `f1_N` (NO leading zero)**

Identity: `Page1[0].f1_1[0]` name, `Page1[0].f1_2[0]` SSN. **Two amount x-columns:**
item/left col **x≈417–489**, result/right col **x≈504–576**, plus line-2 inline at **x≈331–402**.

| Line | Field suffix | x-col | Notes |
|---|---|---|---|
| 1 Medical expenses | `Page1[0].f1_3[0]` | item | |
| 2 AGI from 1040 L11 | `Page1[0].f1_4[0]` | **inline x≈331–402** | reads 1040 L11 |
| 3 Line 2 × 7.5% | `Page1[0].f1_5[0]` | item | |
| 4 Subtract 3 from 1 | `Page1[0].f1_6[0]` | result | |
| 5a State/local income **or** sales tax | `Page1[0].f1_7[0]` | item | **sales-vs-income checkbox** = `Page1[0].c1_1[0]` (on `/1`, x≈388) |
| 5b Real-estate taxes | `Page1[0].f1_8[0]` | item | |
| 5c Personal-property taxes | `Page1[0].f1_9[0]` | item | |
| 5d Add 5a–5c | `Page1[0].f1_10[0]` | item | |
| 5e Smaller of 5d / cap | `Page1[0].f1_11[0]` | item | SALT cap (10k/5k MFS) |
| 6 Other taxes — type text | `Page1[0].f1_12[0]` + `f1_13[0]` | left desc | 2 write-in text fields |
| 6 amount | `Page1[0].f1_14[0]` | item | |
| 7 Add 5e + 6 | `Page1[0].f1_15[0]` | result | |
| 8a Home-mtg int on 1098 | `Page1[0].f1_16[0]` | item | line-8 **"not on 1098" checkbox** = `Page1[0].Line8_ReadOrder[0].c1_2[0]` (on `/1`) |
| 8b Home-mtg int NOT on 1098 | `Page1[0].f1_19[0]` | item | **⚠ amount leaf `f1_19` numbered BEFORE its name/addr text** |
| 8b payer name/addr (text) | `Page1[0].f1_17[0]` + `f1_18[0]` | left | 2 text fields |
| 8c Points not on 1098 | `Page1[0].f1_20[0]` | item | |
| 8d Reserved | `Page1[0].f1_21[0]` | item | (blank/"Reserved" on 2024) |
| 8e Add 8a–8c | `Page1[0].f1_22[0]` | item | |
| 9 Investment interest (4952) | `Page1[0].f1_23[0]` | item | |
| 10 Add 8e + 9 | `Page1[0].f1_24[0]` | result | |
| 11 Gifts by cash/check | `Page1[0].f1_25[0]` | item | |
| 12 Gifts other than cash (8283) | `Page1[0].f1_26[0]` | item | |
| 13 Carryover | `Page1[0].f1_27[0]` | item | |
| 14 Add 11–13 | `Page1[0].f1_28[0]` | result | |
| 15 Casualty/theft | `Page1[0].f1_29[0]` | result | |
| 16 Other — type text | `Page1[0].f1_30[0]` + `f1_31[0]` + `f1_32[0]` | left | 3 write-in text fields |
| 16 amount | `Page1[0].f1_33[0]` | result | |
| 17 Total itemized → 1040 L12 | `Page1[0].f1_34[0]` | result | |
| (line 18 "elect to itemize" box) | `Page1[0].Line18_ReadOrder[0].c1_3[0]` | — | on `/1` |

---

## Schedule B — root `topmostSubform[0]` — 1 page · **payer grids**

Identity: `Page1[0].f1_01[0]` name, `Page1[0].f1_02[0]` SSN.
**Grid columns:** payer-name **x≈130–461**, amount **x≈490–576**.

### Part I — Interest (line 1): **14 rows** (capacity = 14)

Row 1: name `Page1[0].Line1_ReadOrder[0].f1_03[0]`, amount `Page1[0].f1_04[0]`.
Rows 2–14: name = odd `f1_05,f1_07,…,f1_29`; amount = even `f1_06,f1_08,…,f1_30`
(all bare `Page1[0]`; only row 1's **name** carries the `Line1_ReadOrder[0]` subform).

| Line | Field suffix |
|---|---|
| 2 Add amounts on line 1 | `Page1[0].f1_31[0]` |
| 3 Excludable savings-bond int (8815) | `Page1[0].f1_32[0]` |
| 4 Subtract 3 from 2 → 1040 L2b | `Page1[0].f1_33[0]` |

### Part II — Ordinary dividends (line 5): **15 rows** (capacity = 15)

Row 1: name `Page1[0].ReadOrderControl[0].f1_34[0]`, amount `Page1[0].f1_35[0]`.
Rows 2–15: name = even `f1_36,f1_38,…,f1_62`; amount = odd `f1_37,f1_39,…,f1_63`.

| Line | Field suffix |
|---|---|
| 6 Add line 5 → 1040 L3b | `Page1[0].f1_64[0]` |

### Part III — Foreign accounts & trusts (Yes/No `/Btn` pairs, on `/1`=Yes, `/2`=No)

| Line | Yes / No fields |
|---|---|
| 7a Financial interest/signature authority | `Page1[0].c1_1[0]` / `c1_1[1]` |
| 7a-ii Required to file FinCEN 114 | `Page1[0].c1_2[0]` / `c1_2[1]` |
| 7b Name of foreign country (text) | `Page1[0].f1_65[0]` (+ `f1_66[0]` cont.) |
| 8 Foreign trust distribution/grantor | `Page1[0].c1_3[0]` / `c1_3[1]` |

---

## Geometry watch-items for the map-independent read-back

1. **Schedule A — two amount columns + inline L2.** Item col x≈417–489, result col x≈504–576,
   and **L2 alone** sits inline at x≈331–402. The x-cluster read-back must resolve **three**
   x-bands, not two, and must not confuse the same-y **left** description text on lines 6 / 8b / 16
   (x≈65–482) with an amount. Also flag the **8b ordering anomaly** (amount leaf `f1_19` is
   numbered before its name/address text `f1_17`/`f1_18`).
2. **Schedule 2 — 2-page split.** Lines 1–16 are page 1 (`f1_`); lines 17a–21 are page 2 (`f2_`).
   The map's L21 (`Page2[0].f2_25[0]`) and L18/19/20 are all page-2 leaves — the read-back must
   band them on page index 1 (0-based), not page 0.
3. **Schedule B — grid capacity.** Interest line 1 = **14** rows; dividend line 5 = **15** rows
   (asymmetric — not 14/14). Clean 12pt y-descent, two x-columns. Overflow (>14 payers / >15
   payers) → reuse the 8949 continuation-statement path; totals-only on the face form.
4. **Form 1040 line 2a/3a "mid column."** 2a/3a (tax-exempt int / qualified div) live in a
   **mid x-band (≈252–323)** distinct from the 504–576 result column; and 25a–25c sit in a
   **mid band (≈410–482)** vs 25d at 504–576. Two-column read-back per page required.
5. **Filing status = 5 independent checkboxes across 2 containers** (see 1040 §). Not a radio
   group; the map must carry all five (field, on-state) tuples and the app enforces exclusivity.
   The existing same-y `/Btn`-adjacency Yes/No oracle does **not** model a 5-way select.

---

## Extraction surprises / TODO

- **SURPRISE (fill-logic):** Form 1040 **filing status is five separate `/Btn` checkboxes**,
  each with a distinct on-state (`/1`..`/5`), and leaf `c1_3` is reused across the
  `FilingStatus_ReadOrder[0]` container (Single/MFJ/MFS) and bare `Page1[0]` (HOH/QSS). The
  digital-asset Yes/No is the tidy pair the existing oracle already handles; filing status is not.
- **SURPRISE (naming):** Schedule A leaf fields are `f1_N` with **no leading zero** (`f1_3`), unlike
  every other form's `f1_03`. Schedule 3's line-6z **description leaf is `f2_23` on a 1-page form**.
  Copy names verbatim; don't normalize.
- **SURPRISE (subform boundary on 1040):** income lines **4a–11 are inside**
  `Line4a-11_ReadOrder[0]` (so L7 = `…Line4a-11_ReadOrder[0].f1_52[0]`, matching the shipped map),
  but **lines 12–15 are bare `Page1[0]`** and 1a–3b are bare `Page1[0]`. The container prefix is
  per-line, not per-region — must be captured field-by-field.
- **No `/TU` tooltips** on any of the six forms → line labels are geometry-derived only. This is
  fine for the fill (geometry read-back is the safety net) but means **there is no in-PDF text key**
  to auto-generate maps from; the tables above are the hand-verified source of truth.
- **TODO (write-in "type" leaves):** the lettered-total write-in lines (Sch 1 8z/24z, Sch 2 1y/17z,
  Sch 3 6z/13z) each have a *type-description text* leaf + an *amount* leaf. Only the amount is a
  money cell; the Spec must decide whether btctax ever populates the type text (likely blank in the
  W-2-household scope — the crypto engine writes none of these). Marked as structure-captured,
  fill-policy-open.
- **TODO (2025 re-extract):** this artifact is TY2024 only. R1/R3 of the first pass warn that S1 &
  SA **flip their root FQN** in 2025 and that 1040 page-2 renumbers (L7a/L11b/L12e/L13a/L13b). The
  2025 skeleton is a **separate extraction** against the released final 2025 PDFs (not done here).
- **Not re-verified:** exact overflow-cap byte behavior of the merge/rename step on the Sch B grids
  (functional, not structural — belongs to implementation, not recon).
