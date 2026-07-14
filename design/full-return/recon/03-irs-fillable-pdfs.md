# Recon 03 — IRS Fillable PDFs for a Complete W-2 Household Return

**Agent:** recon 3 of 5 · **Scope:** feasibility of extending the `btctax-forms`
fill recipe to FULL Form 1040 + Schedules 1, 2, 3, A, B for **TY2024** and
**TY2025**. First-pass (Fable re-sweeps later). Date: 2026-07-11.

**Method.** Downloaded all 12 official PDFs (6 forms × 2 years) from irs.gov and
inspected each AcroForm directly with `qpdf --json` + `pdfinfo` (field names,
types, `/XFA` presence, widget `/Rect` geometry). Field counts below are **exact
AcroForm widget counts** from qpdf (not inferred). Row counts for grids are read
off widget y-clusters. HTTP status/size confirmed via `curl`.

---

## Bottom line

**All six forms are GREEN.** Every one is an XFA-hybrid AcroForm authored by the
same tool (Adobe LiveCycle "Designer 6.5") with the identical leaf-field naming
(`f1_N[0]` text, `c1_N[0]` checkbox) the existing recipe already targets. The
drop-`/XFA` + `NeedAppearances=true` + geometric read-back approach extends
uniformly. There are **four non-blocking risks** (below), the sharpest being that
the top-level container name is **not stable** across years/forms.

---

## Per-form table

| Form | Current URL (TY2025) · Prior URL (TY2024) | Pages | Fields (2024→2025) | Pattern match | Overflow | Verdict |
|---|---|---|---|---|---|---|
| **Form 1040** | `/pub/irs-pdf/f1040.pdf` · `/pub/irs-prior/f1040--2024.pdf` | 2 | 141 → **199** | Y (`topmostSubform`) | none (fixed-line) | **GREEN** |
| **Schedule 1** | `/pub/irs-pdf/f1040s1.pdf` · `/pub/irs-prior/f1040s1--2024.pdf` | 2 | 69 → 73 | Y — **root flips** `form1`(24)→`topmostSubform`(25) | none (fixed-line)¹ | **GREEN** |
| **Schedule 2** | `/pub/irs-pdf/f1040s2.pdf` · `/pub/irs-prior/f1040s2--2024.pdf` | 2 | 60 → 63 | Y (`form1` both yrs) | none (fixed-line)¹ | **GREEN** |
| **Schedule 3** | `/pub/irs-pdf/f1040s3.pdf` · `/pub/irs-prior/f1040s3--2024.pdf` | 1 | 39 → 37 | Y (`topmostSubform` both) | none (fixed-line) | **GREEN** (cleanest) |
| **Schedule A** | `/pub/irs-pdf/f1040sa.pdf` · `/pub/irs-prior/f1040sa--2024.pdf` | 1 | 37 → 33 | Y — **root flips** `topmostSubform`(24)→`form1`(25) | none (fixed-line) | **GREEN** |
| **Schedule B** | `/pub/irs-pdf/f1040sb.pdf` · `/pub/irs-prior/f1040sb--2024.pdf` | 1 | 72 → 72 | Y (`topmostSubform` both) | **YES** — payer grids | **GREEN** (needs overflow) |

All 12 URLs returned `200 application/pdf`. All 12 are AcroForm + `/XFA`-hybrid
(verified via the AcroForm dict, which carries `/DA /DR /Fields /SigFlags /XFA`
on every file). Leaf-field pattern is `f1_N[0]`/`c1_N[0]` on **all** forms.

¹ *Sch 1/2 have lettered sub-lines (8a–8z income "type", 24a–24z adjustments;
Sch 2 line 8/17 "other taxes" with code boxes). These are **fixed, individually
named fields**, not open-ended grids — no continuation statement, just a
write-in text field + code checkboxes to map.*

---

## Confirmed facts (established, not re-litigated)

- **`irs-pdf/` current == TY2025.** `pdfinfo` Title on `f1040.pdf` = "2025 Form
  1040"; prior year lives at `irs-prior/fNNNN--2024.pdf` (Title "2024 …"). The
  `--YYYY` prior-year filename convention holds for every schedule.
- **XFA-hybrid, all 12.** Same as the forms already shipped → the existing
  drop-`/XFA` + `AcroForm << /NeedAppearances true >>` step applies unchanged.
  (Shipped files show `NeedAppearances=false`; the fill step sets it true.)
- **Money is single-cell (no dollars/cents split).** Zero narrow (<30pt) cents
  boxes on any 2024/2025 form; every amount is one `/Tx` field ~71–128pt wide.
  So `MoneyCell::Single`, matching the existing 2024/2025 f1040 map — the
  2017-era `MoneyCell::Pair` (dollars+cents) is **not** needed for these.
- **Field names shift 2024→2025** (justifies per-year maps): counts change on
  every form (1040 141→199, Sch1 69→73, Sch2 60→63, Sch3 39→37, SchA 37→33), and
  even the root container name flips (see risks).
- **Public domain.** IRS forms are works of the U.S. federal government, not
  copyrightable (17 U.S.C. § 105). Freely bundlable; no runtime fetch — continue
  the existing "commit the PDF under `forms/<year>/`" approach.

---

## Risks / gotchas (all non-blocking)

**R1 — Root container name is NOT stable (highest-value gotcha).**
The fully-qualified field prefix is *not* always `topmostSubform[0]`:

| | 2024 root | 2025 root |
|---|---|---|
| 1040 | topmostSubform | topmostSubform |
| Sch 1 | **form1** | **topmostSubform** |
| Sch 2 | form1 | form1 |
| Sch 3 | topmostSubform | topmostSubform |
| Sch A | **topmostSubform** | **form1** |
| Sch B | topmostSubform | topmostSubform |

The prefix flips **between years for the same form** (Sch 1 and Sch A flip in
opposite directions). **Map authors must capture the exact FQN per (form, year)
from the actual PDF — never hardcode `topmostSubform[0]`.** This is a map-time
correctness trap. The geometric read-back is *map-independent* (works off widget
`/Rect` clusters, not names), so it is unaffected and remains the safety net that
fails-closed on a wrong prefix.

**R2 — Schedule B is the only form with overflow grids.** Highly regular
two-column layout: payer name at x≈130 (w≈331) + amount at x≈490 (w≈86),
descending in clean 12pt y-steps. Capacity: **~14 interest rows (line 1)** and
**~14–15 dividend rows (line 5)**. IRS overflow rule (i1040sb, 2025): you may
list several payers in one entry space (show each amount by name); if still short
on space, *"attach separate statements using the same format as lines 1 and 5,"*
put name + SSN on them, show only the **totals** on Schedule B, attach at end of
return. → Reuse the existing `overflow.rs` continuation-statement pattern (as
8949 does). For the target "common W-2 household," >14 payers is rare, so this is
a correctness-completeness item, not a common path. (Schedule B filing itself is
triggered by >$1,500 interest **or** >$1,500 ordinary dividends.)

**R3 — 2025 Form 1040 is materially bigger (+58 fields, 141→199).** The 2025 form
adds lines vs 2024 (new deduction lines in the TY2025 statute). Feasibility is
unchanged — mechanically identical fill — but the "fill ALL income/deduction/tax/
payment lines" scope is a larger mapping job for 2025, and the *set of lines*
differs from 2024. Enumerating exactly which new lines exist is a **tax-logic
recon** job (another agent), not a PDF-structure blocker.

**R4 — Mild geometric irregularity on Schedule A & Schedule 2** (within the
existing oracle's capability, but warrants targeted read-back tests):
- **Schedule A** has **two amount x-columns** — line-item amounts at x≈410 and
  subtotals/right-hand results at x≈500 — plus a same-y *description+amount* pair
  on line 6 (name at x≈122, amount at x≈410) and 3 election checkboxes (5a
  sales-vs-income tax, line 8 "not on 1098"). The column-x cluster read-back
  handles two columns natively; add a test that the two clusters resolve
  independently and the line-6 description field isn't confused with an amount.
- **Schedule 2** is two-page (`f1_*` page 1 / `f2_*` page 2), with multi-checkbox
  rows (e.g., line 1e box pair at x≈86 & x≈202) and write-in code boxes on lines
  8/17. Checkbox rows resolve via the existing same-y `/Btn`-adjacency oracle;
  confirm adjacency spacing distinguishes the intended pair.

Schedules 1 and 3 are geometrically the simplest (Sch 3 is *all* `/Tx`, no
checkboxes); 1040 checkboxes (filing status, digital-asset) are already handled.

---

## Read-back / geometry verdict

The existing map-independent read-back (column-x clusters + per-column ordinal-y
descent + same-y `/Btn` adjacency for checkboxes) **extends cleanly** to all six
forms. Schedule B is textbook-regular. Schedule A (2 columns) and Schedule 2
(multi-check + 2-page) need a couple of extra targeted read-back cases but fall
inside the existing oracle's model. No form exhibits rotated pages, cents boxes,
or comb/segmented fields that would break the geometric assumptions.

---

## Recommended per-form field-map sketch (starting points)

- **Form 1040** (2 pp): income lines 1a–9, adjustments 10, AGI 11, deductions
  12–15, tax 16–24, payments 25–33, refund/owe 34–37 — all single `/Tx`; filing
  status + digital-asset + dependents checkboxes already patterned. Map the ~40
  money lines; the rest are identity fields.
- **Schedule 1** (2 pp): Part I additional income (1–9, incl. 8a–8z write-ins),
  Part II adjustments (11–25, incl. 24a–24z). Root = `form1`(24)/`topmostSubform`(25).
- **Schedule 2** (2 pp): Part I tax (1–3, AMT/APTC), Part II other taxes (4–21,
  SE tax, Additional Medicare, NIIT, code boxes). Root = `form1` both years.
- **Schedule 3** (1 p): Part I nonrefundable credits (1–8), Part II refundable
  credits/payments (9–13). All `/Tx`. Root = `topmostSubform` both years.
- **Schedule A** (1 p): medical (1–4), taxes (5a–7), interest (8a–10), gifts
  (11–14), casualty (15), other (16), total (17). Two amount columns; 3 checkboxes.
- **Schedule B** (1 p): Part I interest grid (line 1, ~14 rows) + 2–4; Part II
  dividend grid (line 5, ~14–15 rows) + 6; Part III foreign-accounts/trusts
  checkboxes (7a/7b/8, same-y `/Btn` pairs). Wire the overflow path.

---

## Maintenance caveat

`irs-pdf/fNNNN.pdf` always points at the *latest* revision. Today it is TY2025;
when TY2026 forms drop (~Dec 2026) that URL becomes 2026 and 2025 migrates to
`irs-prior/fNNNN--2025.pdf`. Because the recipe **bundles** the PDF (no runtime
fetch), this is only a "when we add a new tax year, pull the then-current file and
pin it" note — existing years are frozen by the committed bytes.

## Sources
- IRS fillable PDFs (current): https://www.irs.gov/pub/irs-pdf/f1040.pdf ,
  f1040s1.pdf, f1040s2.pdf, f1040s3.pdf, f1040sa.pdf, f1040sb.pdf
- IRS prior-year PDFs: https://www.irs.gov/pub/irs-prior/f1040--2024.pdf (and
  `f1040s1--2024.pdf` … `f1040sb--2024.pdf`)
- Instructions for Schedule B (Form 1040) (2025): https://www.irs.gov/instructions/i1040sb
  (overflow / attach-statement rule; >$1,500 filing threshold)
- 17 U.S.C. § 105 (U.S. Government works, no copyright)
