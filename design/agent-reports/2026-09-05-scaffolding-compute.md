# The compute layer — which compute is year-SHAPED, and how a year-shaped struct should be expressed

**Lens:** compute. **Date:** 2026-09-05. **Scope rule followed:** no `git`, `cargo`, `make`, `nextest`.
Tools used: the prebuilt `./target/debug/xtask`, `.venv/bin/python`, and the committed text layers.

> ★ Side effect to know about: `xtask extract-geometry` regenerated
> `design/forms/geometry/f6251--2026-DRAFT.json` and `f1040--2026-DRAFT.json`. Deterministic
> re-extraction of unchanged PDFs; no other file was written by this agent except this report.

---

## Headline

**The repo already contains the right pattern — `SaltLimitation` on `FullReturnParams` — and the three
places that get it wrong get it wrong in three different ways; but the bigger finding is that the
evidence the port plan rests on is partly wrong: `design/forms/2026/f1040--2026-DRAFT.pdf` is the
TY2025 Form 1040, and `form-delta`'s label axis is off by one page on every TY2026 draft, so the
work list's entire "lines that moved" column — including "f6251 moves 28" and the four
"unchanged/mechanical" rows — is an artifact. Measured against the TEXT LAYER instead: Form 6251
does not renumber at all for TY2026, Schedule 1-A Parts IV/V/VI are a pure +6 renumber of an
identical instrument, and the forms that genuinely change shape are Form 8995, Schedule A, and
Schedule 1-A Parts II/III — none of which the work list flags.**

---

## Part A — findings

### C-1 (CRITICAL) · `design/forms/2026/f1040--2026-DRAFT.pdf` is a TY2025 Form 1040

The archived "TY2026 draft" Form 1040 prints 2025 on its face:

```
$ grep -n "Form 1040 (202[0-9])\|Individual Income Tax Return" design/forms/2026/f1040--2026-DRAFT.pdf.txt
33:  1040 U.S. Individual Income Tax Return 2025
120: ... Cat. No. 11320B   Form 1040 (2025) Created 9/5/25
122: Form 1040 (2025)                                                    Page 2
206: Go to www.irs.gov/Form1040 for instructions ...                     Form 1040 (2025)

$ grep -n "2026" design/forms/2026/f1040--2026-DRAFT.pdf.txt
181:  36  Amount of line 34 you want applied to your 2026 estimated tax .  .  .  36
```

The **only** occurrence of "2026" in the document is line 36 — the *next* year's estimated-tax
line, which every TY2025 1040 carries. `scripts/archive_drafts.py:85` reads

```python
years = [int(y) for y in re.findall(r"\b20(?:1[5-9]|2[0-9])\b", out)]
return max(years) if years else None
```

`max()` over pages 1–3, so line 36 wins. Run against the archived file today:

```
$ .venv/bin/python -c "... a.printed_year(Path('design/forms/2026/f1040--2026-DRAFT.pdf'))"
f1040 2026        # <- the check says 2026; the form says 2025
f6251 2026
f1040s1a 2026
```

The module docstring (`scripts/archive_drafts.py:6-11`) says *"every headline form had a TY2026 draft
**except Form 1040**, whose draft URL still served a 2025 document … each fetch is checked against the
year printed ON the document, and a mismatch is REFUSED. That refusal is the feature."* The refusal
did not happen: the file is on disk under a 2026 stem. This is the B1 shape exactly — an instrument
credited with a discrimination it never made. `f8275`/`f8283` were refused (they lack the
next-year-estimated-tax line); the 1040 is the one form on which `max()` cannot work.

**Consequences for the compute lens:** the work-list row `| f1040 | 199 | 0 | 0 | 31 | port |` is a
TY2025-vs-TY2025 comparison. "Form 1040 keeps EVERY field name" is trivially true. **The TY2026 Form
1040's line numbering is unknown**, and it is the arbiter of C-4 below.

Checked all sixteen archived drafts — only this one is wrong-year:

```
f1040--2026-DRAFT            title-years: 2025 2025 2025
f1040s1--2026-DRAFT          title-years: 2026 2026
f1040s1a--2026-DRAFT         title-years: 2026
... (13 more, all 2026)
```

### C-2 (CRITICAL) · `form-delta`'s label axis is off by one page on every TY2026 draft

Two witnesses disagree about what "page" means:

* `crates/xtask/src/form_geometry.rs:228-230` — **box** page comes from the FQN:
  *"The page is not carried on `Field`; the IRS templates always nest widgets under a `PageN[0]`
  subform, so the FQN is the page."*
* `crates/xtask/src/form_geometry.rs:112-117` — **word** page comes from the PDF page ordinal
  (`page_n += 1` per `<page …>`).

A TY2026 draft carries the IRS DRAFT cover sheet as PDF page 1, so form page *N* is PDF page *N+1*:

```
$ ./target/debug/xtask extract-geometry f6251--2025      →  2553 words, 62 boxes, 2 pages
$ ./target/debug/xtask extract-geometry f6251--2026-DRAFT →  2930 words, 62 boxes, 3 pages
$ ./target/debug/xtask extract-geometry f1040--2026-DRAFT →  2605 words, 199 boxes, 3 pages
```

Every form-page-1 box therefore joins against the cover sheet (no labels ⇒ `?`) and every
form-page-2 box joins against form page 1 (wrong labels). Measured:

| stem | boxes with label `?` (2026 draft) | same form, 2025 final |
|---|---|---|
| `f8959` | **26 of 26** | 2 of 26 |
| `f8960` | **38 of 38** | 5 of 38 |
| `f1040` | 168 of 199 (31 labelled) | 87 of 199 |
| `f1040s1a` | 109 of 185 | 2 of 54 |
| `f1040sd` | 47 of 55 | 4 of 55 |
| `f6251` | 34 of 62 (28 labelled) | 2 of 62 |

The work list reports **31** moved for `f1040` and **28** for `f6251` — i.e. **100 % of the labelled
boxes "moved"**, which is the signature of a systematic offset rather than a renumber. The individual
rows are nonsense on their face:

```
$ ./target/debug/xtask form-delta f1040--2025 f1040--2026-DRAFT
  topmostSubform[0].Page2[0].AccountNo[0].f2_33[0]: line 35d -> 1f
  topmostSubform[0].Page2[0].RoutingNo[0].f2_32[0]: line 35b -> 1e
```

Form 1040 page 2 has no line 1e/1f; those are page-1 lines.

**And the "unchanged" rows are a false green, not a weak green.** `f8959` and `f8960` report *0 lines
moved* because **zero** of their boxes carried a comparable label. `form_delta.rs:74-77` skips any
pair where either side is `"?"`, so "no movement detected" and "nothing was compared" are the same
output. The work list's *"Four forms are byte-identical in both axes — those ports are mechanical"*
is therefore unearned for `f8959` and `f8960` (the field axis alone does support it — see C-6).

### C-3 (IMPORTANT, corrective) · Form 6251 does **not** renumber for TY2026

Read from the text layer, which is the authority:

```
2025 (design/forms/extract/f6251--2025.txt):
  1a 2a 3 4 5 6 7 8 9 10 11 12 13 14 ... 40      Part I ends 4 · Part II 5–11 · Part III 12–40
2026 (design/forms/2026/f6251--2026-DRAFT.pdf.txt):
  1a 1b 2a…2t 3 4 5 6 7 8 9 10 11 12 13 … 40     Part I ends 4 · Part II 5–11 · Part III 12–40
```

Line-for-line identical numbering. The only Part I/II differences are **citations and constants**:

| | TY2025 | TY2026 draft |
|---|---|---|
| line 1a | "Subtract Schedule 1-A (Form 1040), line **37**, from … line 14" | "… line **43**, from … line 14" |
| line 4 MFS kicker | "$900,350" | "$640,200" |
| line 5 exemption | 88,100 / 137,000 / 68,500 @ 626,350 / 1,252,700 | **90,100 / 140,200 / 70,100 @ 500,000 / 1,000,000** |
| line 7 cite | "1040 … line 7" | "1040 … line **7a**" |

So `Form6251Line1::Y2025 { line1a, line1b }` (`form6251.rs:60`) is the **correct output shape for
TY2026 as well** — a `Y2026` variant would duplicate an identical shape and fork
`amount_entering_line4` (`:73`), `printed()` (`:265`), `cover_form6251line1`
(`line_coverage.rs:500`) and every exhaustive match for nothing. The only thing blocking reuse is a
field NAME: `Form6251Line1Rule::Y2025 { schedule_1a_l37 }` (`form6251.rs:396`) encodes *another
form's* line number in a Rust identifier.

The TY2026 exemption table also settles R9's magnitude: 70,100 / (640,200 − 500,000) = **0.50**, so
the `dec!(0.25)` literal at `amt.rs:129` is half the 2026 rate.

### C-4 (IMPORTANT) · The two genuine TY2026 drafts disagree about where Schedule 1-A lands, and the arbiter is the missing 1040

```
2026 Schedule 1-A, line 44:  "Add lines 15, 27, 36, and 43. Enter here and on
                              Form 1040, 1040-SR, or 1040-NR, line 13a."
2026 Schedule A,   line 18:  "Is the amount on Form 1040 or 1040-SR, line 11b, minus the
                              amounts on lines 13a and 13b of that form, more than $384,350?"
2025 Schedule 1-A, line 38:  "... Enter here and on Form 1040 or 1040-SR, line 13b,
                              or on Form 1040-NR, line 13c."
```

In TY2025, 13a = QBI and 13b = Schedule 1-A. The 2026 Schedule 1-A puts itself on **13a**; the 2026
Schedule A still subtracts **13a and 13b**. One of the two drafts is stale. `AbsoluteReturn`'s field
is named semantically (`schedule_1a_additional`, `return_1040.rs:1596`) so the *struct* survives
either answer — but `Schedule1A::line_13b` (`schedule_1a.rs:1363`) is named for the 2025 answer, and
`printed.rs::Form1040Lines` has a literal `line13`. **This cannot be resolved without a real TY2026
Form 1040** (C-1).

### C-5 (IMPORTANT) · Schedule 1-A TY2026 is *not* uniformly "rebuilt" — it is two rebuilt parts and four renumbered ones

The work list reads the 219→10 field-name churn as "REBUILT". The field-name axis is an AcroForm
naming fact; the arithmetic tells a different story. Measured part by part:

| part | TY2025 | TY2026 | verdict |
|---|---|---|---|
| I MAGI | 1, 2a–2e, 3 | 1, 2a–…, 3 | same |
| II Tips | 4a–4c, 5–13 | **4a–4e × 5 cols, 6a–6e × 13 cols**, 5,7,8… 15 | **REBUILT — new collection tables** |
| III Overtime | 14a–14c, 15–21 | **16a–16e, 18a–18e**, 17,19,20, 21–27 | **REBUILT collection, renumbered arithmetic** |
| IV Car loan | 22–30 | 28–36 | **+6 renumber**, plus 2 new Yes/No questions per VIN |
| V Seniors | 31–37 | 37–43 | **+6 renumber**, one substantive change |
| VI Total | 38 | 44 | **+6 renumber**, 1040 destination 13b → 13a |

Part V, verbatim, both years:

```
2025 31  Enter the amount from line 3                       2026 37  Enter the amount from line 3
2025 32  Enter $75,000 ($150,000 if MFJ)                    2026 38  Enter $75,000 ($150,000 if MFJ)
2025 33  Subtract line 32 from line 31 … enter $6,000 …35   2026 39  Subtract line 38 from line 37 … $6,000 …41
2025 34  Multiply line 33 by 6% (0.06)                      2026 40  Multiply line 39 by 6% (0.06)
2025 35  Subtract line 34 from $6,000 …                     2026 41  Subtract line 40 from $6,000 …
2025 36a … born before January 2, 1961 …                    2026 42a … born before January 2, 1962 …
2025 36b … born before January 2, 1961 …                    2026 42b … born before January 2, 1962 …
2025 37  Enhanced deduction … Add lines 36a and 36b         2026 43  Enhanced deduction … Add 42a and 42b
```

Every constant is unchanged. The **only** substantive movement is the birth-date cutoff, and that is
*already* year-general in this repo: `return_1040.rs:90`

```rust
pub(crate) fn born_early_enough(dob: Date, year: i32) -> bool {
    Date::from_calendar_date(year - 64, Month::January, 1).is_ok_and(|cutoff| dob <= cutoff)
}
```

2025 − 64 = 1961 ✓, 2026 − 64 = 1962 ✓. So Part V needs **no per-year constant at all** — it needs
the two hardcoded `false`s at `return_1040.rs:2073-2074` replaced with this function. That closes
R13 and ports the cutoff in the same edit.

Counter-evidence for the "rebuilt" reading being *right* where it is right — TY2026 Part II line 4 is
a five-row, five-column employer table (name / EIN / W-2 box 12 code "TP" / Form 4137 line 1 col (c) /
larger of (iii)|(iv)) and line 6 a five-row **thirteen**-column 1099 table. TY2025 had three scalar
lines 4a/4b/4c. That is a different instrument, not a renumbering.

### C-6 (IMPORTANT) · Form 8995 gains a computed line btctax does not model, and moves the deduction

Not flagged anywhere; the work list has `f8995 | 22 | 14 | 11 | 0 | port` with **0** lines moved
(and see C-2 for why that 0 means nothing).

```
2025:  15 Qualified business income deduction. Enter the smaller of line 10 or line 14.
          Also enter this amount on [1040 line 13a]
       16 Total qualified business (loss) carryforward. Combine lines 2 and 3 …
       17 Total qualified REIT dividends and PTP (loss) carryforward …

2026:  15 Qualified business income deduction BEFORE the minimum deduction for active
          qualified business [income]
       16 Minimum deduction for active qualified business income (see instructions)   ← NEW
       17 Qualified business income deduction. Enter the GREATER of line 15 or line 16.
          Also enter this amount on …
       18 Total qualified business (loss) carryforward. Combine lines 2 and 3 …
       19 Total qualified REIT dividends and PTP (loss) carryforward …
       20 Check the box for this line 20 only if … Electing Small [Business Trust]   ← NEW
```

Three consequences in `crates/btctax-core/src/tax/qbi.rs` and its readers:

1. `Form8995Lines::line15` is the deduction today; in TY2026 the deduction is **line 17** and line 15
   is an intermediate. `printed.rs:710`'s `form_1040_lines` reads `q.line15` for 1040 line 13 — that
   would file the *pre-minimum* figure, **understating the §199A deduction and overstating tax**.
   Conservative in direction, wrong in fact, and it silently denies the filer the new minimum.
2. `AbsoluteReturn.qbi_carryforward_out` is documented *"Form 8995 line 16"* (`return_1040.rs:1611`)
   and `qbi_reit_ptp_carryforward_out` *"line 17"* (`:1608`). Both become 18/19. These are
   **written back to next year's return**, so a wrong label here propagates.
3. Line 16 has no production in this repo — it is a new §199A(*) minimum deduction whose eligibility
   ("active qualified business income") is not on the input surface. Under the standing rule, *if the
   form asks something our input surface cannot answer, collect it.*

`f8995a` has a TY2026 draft archived and **no TY2025 extract** (`design/forms/extract/` holds
`f8995a--2024.txt` only), so the 43 line-numbered fields across `qbi_a.rs`'s four Part structs are
**unmeasured for both 2025 and 2026**.

### C-7 (IMPORTANT) · Schedule A TY2026 is a new instrument, not a renumber

```
2025 17  Add the amounts in the far right column for lines 4 through 16.
         Also, enter this amount on Form 1040 or 1040-SR, line 12e
2025 18  If you elect to itemize … check this box

2026 13  Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet  ← NEW ws
2026 14  Carryover from prior year
2026 15  Add lines 13 and 14
2026 16  Casualty and theft loss(es) … (was 15)
2026 17a–17k, 17z  Other itemized deductions, ENUMERATED (was one free-text line 16)
2026 18  Is [1040 line 11b − 13a − 13b] more than $384,350?
           No.  … Add lines 4 through 17z. Also enter … on 1040 line 12e.
           Yes. Your deductions MAY BE LIMITED. See the Itemized Deductions Worksheet …  ← NEW ws
2026 19  If you elect to itemize … check this box
```

The itemized total moves **17 → 18**, and line 18 introduces the OBBBA §68-style limitation that
btctax does not model at all. `AbsoluteReturn.itemized_deduction` is documented *"Schedule A **line
17** itemized total"* (`return_1040.rs:1562`) and `printed.rs::form_1040_lines` takes
`sch_a.line17` for 1040 line 12. `charitable.rs::apply_170b` now feeds a *worksheet* rather than
Schedule A lines 11/12 directly.

### C-8 (IMPORTANT) · `Schedule1A::compute` takes `year` and discards it into the params lookup

`crates/btctax-core/src/tax/schedule_1a.rs:1311-1320`:

```rust
pub fn compute(
    year: i32, agi_line11b: Usd, status: FilingStatus,
    inputs: &Schedule1aInputs, taxpayer_qualifies_as_senior: bool, spouse_qualifies_as_senior: bool,
) -> Option<Self> {
    let p = crate::tax::tables::schedule_1a_params(year)?;
```

`year` is consumed **only** by the params lookup, which returns the same values for 2025..=2028
(`tables.rs:1088-1089`). Every part below is built from the TY2025-numbered structs, so
`Schedule1A::compute(2026, …)` today yields a schedule with lines `4a…38` — the wrong form — and
`Schedule1A::line_13b` (`:1363`) returns `part6.line38`, which for TY2026 must be line 44. This is
R2 stated from the compute side; the call site is correct about *taking* the year
(`return_1040.rs:2064-2069`, with a good comment about not reading `ri.tax_year`), and the callee is
where the year stops mattering.

### C-9 (IMPORTANT) · The Schedule 1-A 2025–2028 guard *enforces* identity rather than checking it

`tables.rs:1147-1159`:

```rust
for year in 2025..=2028 {
    let got = schedule_1a_params(year)...;
    assert_eq!(got.year, year);
    // Nothing here is indexed, so every in-range year is the SAME instrument.
    assert_eq!(Schedule1aParams { year: 2025, ..got.clone() }, p(),
               "TY{year} drifted from TY2025 — none of these amounts is indexed");
}
```

The struct carries no birth-cutoff field, so the one datum that *does* move year to year (C-5) is
invisible to it. If a future change adds a per-year field this test reds **on the correction**. The
right resolution is not to relax it but to keep the cutoff out of the struct entirely and derive it
with `born_early_enough` — a year-general function is stronger than a per-year constant, because
there is nothing to forget to add.

### C-10 (confirming R1, with the mechanism) · the Form 6251 line-1 hardcode is a *selection-site* defect, not a modelling defect

`return_1040.rs:2491-2493`:

```rust
    // TY2024's Part I. TY2025 passes `Y2025 { .. }` here; the year lives at THIS call site,
    // which is the one place that knows it (D-4).
    line1_rule: crate::tax::form6251::Form6251Line1Rule::Y2024,
```

`form6251_inputs_from_parts` (`:2476`) takes 11 parameters and none is `year` or `params`. Its **only**
caller is `assemble_absolute` (`:2277`), which has `params: &FullReturnParams` and `year: i32` in
scope and already passes `&params.amt` to `compute_6251` two lines later. All three Y2025 operands
are also in scope there (`agi`, `total_deductions`, and the local `schedule_1a`). So the fix is a
parameter, and the compiler reds the call site.

Contrast the same repo doing it right, `tables.rs:313-323` and `:465`:

```rust
/// §164(b) SALT limitation, transcribed per year because the two years are different instruments.
/// ★ Why an enum and not four numbers. … the enum makes it unrepresentable.
pub enum SaltLimitation { FlatCap { … }, Worksheet2025 { … } }
```

`SaltLimitation` is a **field of `FullReturnParams`**, so the year picks the instrument *once*, at
table construction in `btctax-adapters/src/tax_tables.rs:130`, and every compute site just calls
`.line_5e(...)`. `Form6251Line1Rule` is the identical idea with the selection left at the call site.
That single difference is the whole defect.

---

## Part B — the answer to the question

### B1 · What is the right pattern for a form whose LINE NUMBERING changes between years?

**All three, chosen per axis — and the axes are separable, which is the point.** A form struct fuses
three independent things today, and each fails in a different year:

| axis | what it is | right expression | repo exemplar (good) | repo instance (bad) |
|---|---|---|---|---|
| **quantity** | *which number this is* | a **semantic field name**, line number in the doc comment | `AbsoluteReturn` — `wages`, `agi`, `deduction`, `qbi_deduction`, `schedule_1a_additional` (`return_1040.rs:1530-1660`); zero `lineNN` fields | `printed.rs` (107 `lineNN` fields over 7 structs), `schedule_1a.rs` (56), `qbi.rs::Form8995Lines` (16) |
| **instrument** | *what arithmetic this line does* | a **variant enum carried on the year's params bundle**, selected once at table construction | `SaltLimitation` on `FullReturnParams` (`tables.rs:323`, `:465`) | `Form6251Line1Rule` (`form6251.rs:389`) — right type, selected at `return_1040.rs:2493` |
| **label** | *what number it prints as* | a **per-year `field → label` table**, read by the emitter and the census, never a literal beside the field | `LineCoverage { form, year, line, field, instruction }` (`line_coverage.rs:105-125`) — the table already exists | `Schedule1A::leaves()` (`schedule_1a.rs:416-551`) — 52 literal labels `("37", …)` welded to one year |

Concretely, the decision procedure for a form in a new year:

1. **Did the arithmetic of a line change?** → new **variant** on the instrument enum, on
   `FullReturnParams` / `Schedule1aParams`. (2026: Form 8995 line 17 `min` → `max`; Schedule A line
   18's §68 gate; Schedule A line 13's charitable worksheet.)
2. **Did only the printed number change?** → new **label row**, nothing else. (2026: Schedule 1-A
   Parts IV/V/VI, +6.)
3. **Did the *collection surface* change — new rows, new columns, new questions?** → **per-year
   struct** for that part only, because the leaf set itself differs. (2026: Schedule 1-A Parts II and
   III; Schedule A's 17a–17z; Part IV's two Yes/No per VIN.)

A **per-year struct for the whole form is the wrong default**, and Schedule 1-A is the proof: a
`Schedule1A2026` would duplicate Parts I, IV, V and VI verbatim under new names, forking `leaves()`,
`is_completed`, `NON_MONEY_LEAVES`, `cover_schedule1a` and every conformance KAT, to express a `+6`.
It would also make the *same* deduction two unrelated Rust types, so nothing downstream could be
written once.

A **variant enum for the whole form is equally wrong** where only labels move — `Form6251Line1::Y2026
{ line1a, line1b }` would be byte-identical to `Y2025` (C-3), and every match on it would grow an arm
that does nothing.

**The discriminator between (2) and (3) is measurable and is not measured today.** `form-delta`
reports two axes — field-name churn and label drift — and both are silent on *"is this the same
sentence?"*. A third axis, **instruction-text equality on corresponding lines**, is what separates a
renumber from a rebuild, and I had to compute it by hand for every finding above.

### B2 · Every year-shaped compute site, with the pattern each needs

357 line-numbered fields across 35 structs in `crates/btctax-core/src/tax/` (counted, not estimated).

| # | site | year-shaped how | pattern needed | TY2026 status |
|---|---|---|---|---|
| 1 | `tables.rs::TaxTable`, `FullReturnParams`, `AmtParams` | constants | per-year DATA — **already right** | needs a bundle entry; `amt` values measured in C-3 |
| 2 | `tables.rs::SaltLimitation` (`:323`) | instrument | variant enum on params — **already right** | TY2026 SALT worksheet **unmeasured** (no 2026 i1040sa read) |
| 3 | `tables.rs::Schedule1aParams` (`:1017`) | constants | per-year DATA | 2026 amounts unchanged; **no birth-cutoff field** (C-9) |
| 4 | `form6251.rs::Form6251Line1Rule` (`:389`) | instrument, **wrong selection site** | move selector onto `FullReturnParams`; rename `schedule_1a_l37` → semantic | 2026 reuses the Y2025 SHAPE; only the cited label changes 37→43 |
| 5 | `form6251.rs::Form6251Line1` (`:47`) | output shape | keep two variants; do **not** add `Y2026` | 2026 = Y2025 shape (C-3) |
| 6 | `form6251.rs::Form6251` (41 line fields) | labels only | one struct + per-year label table | **no renumber** 2025→2026 (C-3) |
| 7 | `schedule_1a.rs` Parts I, IV, V, VI (26 fields) | labels only | label indirection, `+6` | measured pure renumber (C-5) |
| 8 | `schedule_1a.rs` Parts II, III (22 fields) | collection surface | **per-year struct for these parts** | genuinely rebuilt (C-5) |
| 9 | `schedule_1a.rs` 6 worksheet structs (24 fields) | collection surface | follows Parts II/III | rebuilt with them |
| 10 | `schedule_1a.rs::leaves()` (`:416`) + `line_label_of` | labels | the label literals become a per-year table | 52 labels welded to TY2025 |
| 11 | `schedule_1a.rs::line_13b` (`:1363`) | label in a *method name* | rename semantic (`total_additional_deductions`) | destination 13b→13a, contested (C-4) |
| 12 | `qbi.rs::Form8995Lines` (16 fields) | **instrument + labels** | variant enum: `min(l10,l14)` → `max(l15,l16)`, plus a new collected line | **REBUILT** (C-6) |
| 13 | `qbi_a.rs` Form 8995-A (43 fields, 4 structs) | unknown | unknown | **UNMEASURED** — no `f8995a--2025` extract |
| 14 | `printed.rs::ScheduleALines` (22) | **instrument + labels** | variant enum for the §68 gate; total 17→18 | **REBUILT** (C-7) |
| 15 | `printed.rs::Form1040Lines` (34), `Form1040Income` (11) | labels; possibly instrument | semantic fields + per-year label table | **UNKNOWN** — no TY2026 1040 (C-1) |
| 16 | `printed.rs::Schedule1Lines` (10), `Schedule2Lines` (6), `Schedule3Lines` (5) | labels | label table | top-level numbering stable 1..21 / 1..15 |
| 17 | `printed.rs::ScheduleDLines` (18), `ScheduleSeLines` (12), `ScheduleCLines` (12), `ScheduleBLines` (4) | labels | label table | Sch D `1a…22` identical both years |
| 18 | `other_taxes.rs::Form8959Lines` (17) | labels | label table | measured **1..24 identical** both years |
| 19 | `other_taxes.rs::Form8960Lines` (15) | labels | label table | field axis 38/0/0; label axis blind (C-2) |
| 20 | `capital_loss_carryover.rs` (13) | labels | label table | Sch D worksheet, numbering stable |
| 21 | `amt.rs:129,:141` | **constants inlined as literals** | read `AmtParams::exemption_phaseout_rate` / `rate_26` | 0.25 vs measured **0.50** for 2026 |
| 22 | `charitable.rs::apply_170b` (`:106`) | **instrument** | variant enum on params | 2026 moves §170(b) into a Schedule A worksheet (C-7) |
| 23 | `return_1040.rs::form6251_inputs_from_parts` (`:2476`) | **missing the year** | take `params`/`year` | C-10 |
| 24 | `return_1040.rs:2073-2074` seniors `false, false` | **unasked question** | `born_early_enough(dob, year)` | R13 + ports 1961→1962 free (C-5) |
| 25 | `return_1040.rs::born_early_enough` (`:90`), `standard_deduction` (`:147`) | year-general derived | **already right** — the model to copy | — |
| 26 | `printed.rs` as a whole | **no `year` in scope at all** (zero non-test references) | must gain the label table or the year | 107 line-numbered fields decided by nothing |

Statutory constants deliberately *not* in this list, verified as year-independent: `MEDICAL_FLOOR_RATE`
(`return_1040.rs:213`), `SE_6017_FLOOR` (`:1512`), `SCHEDULE_B_THRESHOLD` (`:3479`), `QBI_RATE`
(`qbi.rs:18`), `MEDICARE_EMPLOYEE_RATE` (`other_taxes.rs:25`), `FORM_8283_THRESHOLD`
(`printed.rs:188`), the §1(h) 15/20 % rates (`compute.rs:101,104`), the §170(b) 60/50/30 % ceilings
(`charitable.rs:145-158`), and the §24 5 % phase-out (`advisories.rs:719`).

### B3 · What the Schedule 1-A "rebuild" implies for the choice

**It implies the opposite of what the field count suggests, and it is the strongest argument for
splitting the axes.** 10-of-219 field survival is an *AcroForm naming* fact. Measured against the
form's own text (C-5): Parts IV, V and VI are the same instrument at `+6`, with exactly one
substantive change — a birth-date cutoff this repo already derives year-generally. Parts II and III
are new collection tables.

Three consequences:

1. **A whole-form per-year struct would be mostly duplication and would still be wrong.** Duplicating
   Parts IV/V/VI to express `+6` puts eight identical Part V lines under two type names, and the
   next year does it again. Under this repo's own transcription rule the *doc comments* would be
   duplicated too — and a duplicated instruction is one someone eventually edits on one side only.
2. **A whole-form label indirection would be wrong too**, because Parts II/III genuinely differ in
   their leaf set — a label table cannot express "line 4 became a five-column table".
3. **So the unit of the choice is the PART, not the form.** Schedule 1-A's existing decomposition into
   `Schedule1aPartI…VI` is already the right seam; what is missing is that the *label* is welded into
   `leaves()` and the *field names* are welded into the parts. Split those and TY2026 is:
   `Part V` unchanged + a label row; `Part II`/`Part III` per-year; `Schedule1aParams` unchanged;
   `Form6251Line1Rule` reused with a semantic field name.

That is the shape in which a new year is a **data** change for four parts and a **code** change for
two — which is the honest floor, because two parts really did change.

**The precondition none of this survives without:** the discriminator that told me which parts moved
and which were rebuilt was *reading the two text layers side by side*. Today the repo's only
automated year-delta is `form-delta`, whose field axis cannot see a renumber and whose label axis is
broken on drafts (C-2). Until an instruction-text axis exists, every one of these classifications is
a hand measurement that will not be repeated when the finals land in Nov 2026 – Jan 2027 — and
`form_delta.rs:3-6` says making that a diff rather than a rebuild is the whole reason it exists.

---

## MECHANICAL — a machine can do it

1. **Reconcile the two page indices.** `form_geometry.rs:230` derives a box's page from its FQN;
   `:112-117` derives a word's page from the PDF ordinal. Make one of them authoritative (simplest:
   detect the DRAFT cover sheet and offset, or take the box page from the widget's `/P`). Land it
   **paired with a B1 kill-test** that prepends a blank page to a known fixture and asserts the label
   join goes red — that test is what this class has been missing.
2. **Make `form-delta` refuse rather than report `0`** when a side's `?` rate exceeds a threshold, and
   print `n labels compared` on every run. `f8959` reporting "0 lines moved" from 0 comparisons is
   the exact "green because it never ran" shape.
3. **Re-archive `f1040--2026`.** Replace `printed_year`'s `max()` (`archive_drafts.py:85`) with the
   form's own title/footer year (`Form 1040 (YYYY)` / the masthead year); the next-year
   estimated-tax line defeats `max()` on every 1040. Add a kill-test that feeds it the current
   `f1040--2026-DRAFT.pdf` and asserts `2025`.
4. **Add the third `form-delta` axis: instruction-text equality per corresponding line.** Two forms,
   a label correspondence, and a whitespace-normalised comparison of each line's sentence — the
   same normalisation `LineCoverage.instruction` already uses. This is what tells "renumbered" from
   "rebuilt" and it is the only reason this report could be written.
5. **Give `form6251_inputs_from_parts` the year.** Take `params: &FullReturnParams` (or `year: i32`)
   at `return_1040.rs:2476` and drop the `Y2024` literal at `:2493`; the caller at `:2277` already
   holds both. Move the rule *selector* onto `FullReturnParams`, mirroring `SaltLimitation`.
6. **Replace the two AMT literals** at `amt.rs:129` (`dec!(0.25)`) and `:141` (`dec!(0.26)`) with
   `amt.exemption_phaseout_rate` and `amt.rate_26`, which are already on `AmtParams` and already read
   correctly throughout `form6251.rs`.
7. **Rename the line numbers out of the identifiers** that cross form boundaries:
   `Form6251Line1Rule::Y2025 { schedule_1a_l37 }` and `Schedule1A::line_13b`. Both name *another
   form's* line, both move in 2026, and both are pure renames.
8. **Wire `born_early_enough(dob, year)`** into the two `false` arguments at
   `return_1040.rs:2073-2074`. Closes R13 and carries the 1961→1962 cutoff with no new constant.
9. **Archive the missing extracts** so every emitted form has both years on disk:
   `f8995a--2025` (only `--2024` exists) and a TY2026 `f8995a` correspondence. 43 line-numbered
   fields currently rest on no comparison at all.
10. **Assert the label set of a form struct against the extracted text for its own year**, the way
    `xtask::schedule_1a_membership` already does for TY2025 — but keyed by the struct's declared
    year, so adding TY2026 forces the second membership test to exist.

## HUMAN — someone must read a form

1. **Adjudicate 13a vs 13b (C-4)** against the TY2026 Form 1040 once a genuine one is archived. The
   2026 Schedule 1-A and the 2026 Schedule A contradict each other, and no oracle can settle it.
2. **Read `i8995` for TY2026 line 16, "Minimum deduction for active qualified business income"** —
   its amount, who qualifies, whether "active" is answerable from btctax's input surface, and whether
   a filer who cannot answer must be refused rather than given the smaller `line15`.
3. **Read `i1040sa` for TY2026** — the new *Itemized Deductions Worksheet* behind Schedule A line 18
   (the $384,350 gate) and the new *Charitable Contribution Limitation Worksheet* behind line 13.
   Both are worksheets, i.e. transcription targets, not derivations.
4. **Decide the Schedule 1-A part split**: Parts II and III as per-year structs, Parts I/IV/V/VI as
   one instrument with a per-year label table. This is a design call with a real cost either way and
   it should be made once, explicitly, before TY2026 params are bundled.
5. **Confirm the TY2026 AMT exemption phase-out rate of 0.50** against the statute or Rev. Proc.
   rather than against the draft's implied arithmetic (I derived it as
   70,100 / (640,200 − 500,000); that is corroboration, not authority).
6. **Rule on the Part IV vehicle questions** — TY2026 adds "did original use start with you?" and
   "did final assembly occur in the United States?" per VIN. Under the standing rule these are
   testimony to collect, not defaults to invent.
7. **Decide whether `full_return_for(2026) → Some` may land before the above are closed.** The
   TY2025 guard's own doc comment (`btctax-adapters/src/tax_tables.rs:795-812`) says bundling params
   is the *only* year gate, so the same trap re-arms for 2026 — and the six form-shape findings above
   are exactly the "plausible wrong numbers" it warns about.
