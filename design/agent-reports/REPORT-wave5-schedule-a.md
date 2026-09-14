# REPORT — wave 5. TY2026 Schedule A: WRONG FIGURE, or WRONG LABEL?

**Brief:** `design/agent-reports/BRIEF-wave5-schedule-a.md` (commit `4a9120217`), read in full.
**Owns:** one new test file, `crates/btctax-cli/tests/ty2026_schedule_a.rs` (18 tests).
**No shipped source modified, no manifest touched, nothing committed, nothing pushed.** The working
tree carries exactly one untracked path: `crates/btctax-cli/tests/ty2026_schedule_a.rs`.

---

## 0. THIS VALIDATES NOTHING

OpenTaxSolver 2026 does not exist until ~2027-01; taxcalc has no validated TY2026 parameters; no
`f1040sa--2026` **final** is archived. Every TY2026 fact below comes from
`design/forms/extract/f1040sa--2026-DRAFT.txt`, marked *"DRAFT — evidence only, never transcribed as
authority"*. The figures prove that **a mechanism reads a field**. They do not prove any number is
right. Two documents the TY2026 Schedule A depends on — the **Charitable Contribution Limitation
Worksheet** (its line 6 IS line 13) and the **Itemized Deductions Worksheet** (behind line 18's
$384,350 screen) — live in `i1040sca--2026` / `i1040gi--2026` and **neither is on disk**. Where a
figure would need one, this run names the gap and asserts nothing.

---

## 1. THE ANSWER, PER COLLISION

One fixture: the owner's shape — MFJ, W-2 wages $220,000, two Bitcoin disposals (one LT, one ST),
Schedule B on both halves ($4,200 interest + $9,000 ordinary dividends), **itemized** Schedule A
(medical $3,000, real-estate tax $9,500, state withholding $36,000, $6,000 cash gift), no Schedule C,
no retirement. Engine AGI **$273,200**. A second variant adds a **$4,000** prior-year Cash60 carryover.

| line | what the chain produced | what TY2026 requires there | verdict |
|---|---|---|---|
| **13** | the **carryover allowed** — `$0` / **`$4,000`** | *"Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet"* — a **current-year** quantity | ★ **WRONG FIGURE** |
| **14** | `11 + 12 + 13`, the **whole charity block** — `$6,000` / **`$10,000`** | *"Carryover from prior year"* — `$0` / `$4,000` | ★★ **WRONG FIGURE, overstated by exactly the current-year gift total ($6,000)**, printed under the heading *Carryover from prior year* |
| **15** | **nothing — `ScheduleALines` has no field** | *"Add lines 13 and 14"* = `$10,000` | ★★ **WRONG FIGURE BY OMISSION.** Not "blank because the inputs say so" — blank because nothing ever populated it, and it is a subtotal the 2026 **total** is figured from |
| **16** | **nothing — no field** | casualty/theft, and the clause **widened** to *"federally **or state-declared**"* | **WRONG LABEL on this household** (correctly blank: no casualty loss) + FR-187's eligibility widening |
| **17** | the **whole itemized total**, **`$65,400`** | *"Other itemized deductions"* parent, enumerated 17a–17k + 17z — **all correctly blank** for this filer | ★★ **WRONG FIGURE.** A $65,400 total in a box whose correct value is empty |
| **18** | a **BOOLEAN** (`line18_elects_smaller: bool`, `false` here) | **the total**, behind the $384,350 screen, feeding 1040 line 12e | ★★★ **WRONG FIGURE / TYPE INVERSION.** A checkmark where a six-figure amount belongs — and the chain has **no 19** at all for the §63(e) checkbox it does compute |

**Score: four of the six are WRONG FIGURES, one is a wrong figure by omission, one is a wrong label.**

★★ **And 13 + 14 as the chain fills them is `$14,000`** — so TY2026's own line 15 (*"Add lines 13 and
14"*) would **double-count the carryover**: `$4,000 + $10,000` where the schedule's 15 should read
`$10,000`. The chain has no 15 to print, so the arithmetic error surfaces the moment a TY2026 field map
exists.

Tests: `collisions_13_and_14_are_a_swap_of_quantities_not_of_labels`,
`collision_15_has_no_field_and_the_total_depends_on_it`,
`collision_16_is_a_label_move_on_this_household_but_the_rule_widened`,
`collision_17_puts_the_whole_total_where_other_itemized_deductions_belong`,
`collision_18_is_a_checkbox_where_ty2026_prints_the_total`.

---

## 2. THE CONTROL DISCRIMINATES — measured, not asserted

`the_control_moves_and_the_charity_block_does_not`. **One fixture, one AGI, three parameter sets**,
differing in nothing but `params.salt`:

| params | 5d | **5e** | 11 / 12 / 13 / 14 |
|---|---|---|---|
| TY2024 (bundled, `FlatCap`) | $45,500 | **$10,000** | 6,000 / 0 / 0 / 6,000 |
| TY2025 (`Worksheet2025`, figures **parsed from `f1040sa--2025.txt`**) | $45,500 | **$40,000** | 6,000 / 0 / 0 / 6,000 |
| TY2026 (`ty2026_full_return()`) | $45,500 | **$40,400** | 6,000 / 0 / 0 / 6,000 |

Line 5e — whose **number did not move** — produces three different cells. The charity block — whose
**numbers did move** — is identical to the cent in all three. And `line17` moves by **exactly** the 5e
delta (`l26.line17 − l24.line17 == l26.line5e − l24.line5e`), so the total tracks the parameter and is
blind to the renumber. **5e is seen; 13–18 are not.**

★ Further: `line_5e_keeps_its_number_and_every_figure_moves` parses all four dollar figures out of each
extract (`40,000 / 20,000 / 500,000 / 250,000` → `40,400 / 20,200 / 505,000 / 252,500`) and asserts the
**bundled** `ty2026_full_return().salt` carries the ones the 2026 sheet prints. So the params were
transcribed from this document, which is what licenses 5e as the control.

---

## 3. ★★★ THE BOTTOM LINE: the renumber alone costs no money — a NEW FLOOR does, and it UNDERSTATES

`the_renumber_leaves_the_total_alone_but_the_new_charitable_floor_does_not`.

The chain's total is `4 + 7 + 10 + (11 + 12 + 13)`. TY2026's is `4 + 7 + 10 + 15 + 16 + 17z` with
`15 = 13 + 14`. Under the 2026 meanings those are the **same quantity if and only if** the *Charitable
Contribution Limitation Worksheet*'s line 6 equals the §170(b)-limited current-year total
`charitable.rs` computes. That identity is asserted in the test.

★★ **It does not hold, and the reason is statute rather than layout.** Pub. L. 119-21 **§70425** adds
**§170(p)**, a **0.5%-of-AGI floor** on an itemizer's charitable contributions for tax years beginning
after 2025 — already recorded in this repo at `design/OWNER_DECISIONS_2026-09-04.md:212`,
`design/direction/filing-readiness-lens-itemized.md:182` and
`design/agent-reports/RECON-schedule-a-ty2026.md:385`. A *limitation worksheet* appearing in the same
revision that stops summing gifts straight into the subtotal is exactly where such a floor lands.

**Measured on the owner's profile: `0.005 × $273,200 = $1,366`** — smaller than the `$10,000`
charitable total it would reduce, so the whole $1,366 is deduction btctax claims and §170(p) does not
allow. **Deduction overstated, tax UNDERSTATED.** The size cannot be made exact without the worksheet
(does the floor hit the current-year total, the carryover, or the sum?) — that is the S-4 wall, and this
run asserts nothing about it.

`no_charitable_floor_and_no_itemized_limitation_exists_anywhere` pins the absence over the whole
workspace source (`170(p)`, `charitable_floor`, `itemized_limit`, `sec_68`, `pease` → **0 hits**), and
`the_source_scanner_actually_reads_files` is its positive control — the scanner is first asked for
`§170(b)`, must find it in `charitable.rs` among others, and is then asked for a string no file
contains. **Without that control the wall pin would look identical to a scanner walking the wrong
directory.**

---

## 4. ★★ REFUTED / REFINED PREMISES

**R-1 (the brief's adverse mechanism — REFUTED as an arithmetic path).** The brief's *"price this one
first"* says reading 13 as the carryover *"takes a bigger number as the carryover — a larger deduction,
less tax, understatement."* **No code in btctax reads a Schedule A line number for the carryover.**
Next year's `charitable_carryover_in` is seeded from `AbsoluteReturn::charitable_carryover_out` — a
value the §170(b) engine **computed** (`return_1040.rs:2789`; write-back at `return_1040.rs:3834`;
preservation at `cmd/tax.rs:182-198`; restamp at `open_next_year.rs:801-814`). The three *"Schedule A
line 13"* sites are **prose**:

| site | kind |
|---|---|
| `charitable.rs:32` | doc comment on `CharitableResult::allowed_carryover` |
| `charitable.rs:258` | doc comment on a unit test |
| `return_1040.rs:3945` | ★ a **filer-facing refusal message**: *"{next}'s Schedule A line 13 is deliberately outside the acknowledgment gate"* |

Only the third can mislead a human, and for `next = 2026` it names the wrong line. Its consequence is a
filer directed to the wrong box, **not a figure btctax computes**. So the adverse direction is real and
is **larger** than the brief supposed — but it arrives by **§170(p)** (§3 above), not by line 13.

**R-2 (citation drift).** The brief and `FOLLOWUPS.md:8309` (FR-185) both cite `return_1040.rs:3918`.
Measured: the string *"Schedule A line 13"* is at **`return_1040.rs:3945`**; 3918 is a comment header
(*"FINAL-REVIEW FINDING 1"*). Nit.

**R-3 (FR-185 undercounts line 17's children).** FR-185 and the brief say *"enumerated 17a–17h+"*.
Derived from the extract: **17a, 17b, 17c, 17d, 17e, 17f, 17g, 17h, 17i, 17j, 17k and 17z** — twelve
sub-ids, eleven data lines plus the 17z subtotal. Nit.

**R-4 (every premise that mattered, CONFIRMED).** All six collisions are real, derived rather than typed
(`the_schedule_a_renumber_cascade_is_derived_from_the_two_extracts`): **two captions move VERBATIM to a
new line number** — 2026's 14 is 2025's 13 (*"Carryover from prior year"*) and 2026's 19 is 2025's 18
(*"If you elect to itemize deductions…"*) — which is the mechanical signature of a shift-by-one cascade
and needs no hand-written move table. The derived changed-caption set over shared ids is exactly
**{5e, 8d, 8e, 13, 14, 15, 16, 17, 18}**: the six from 13 up are role moves; 5e / 8d / 8e are
**in-place** changes on numbers that did not move. 19 is new; 17 gained sub-lines and 2025's 16 had none.

**R-5 (Tier A's A-7 trap reproduced and neutralised).** A raw id-set difference over-reports because
`pdftotext -layout` puts the 2025 sheet's sub-line letters in their own column. The derivation here
tracks the integer parent and accepts a bare letter only when it is the expected next one, which
recovers `5a`–`5e` and `8a`–`8e` on the 2025 sheet — so **8d and 8e appear as in-place changes rather
than as new lines**, which is the class FR-185 needs them filed under.

---

## 5. ★★ WHAT ALREADY PROTECTS THE PORT, AND WHAT IT CANNOT DO — the FR-162 answer

`the_schedule_a_label_census_is_quoted_at_a_single_year`: every one of `cover_schedulealines`'s rows is
stamped **`"2024"`** (the collector never calls `Coverage::quoting`, so it takes `DEFAULT_ROW_YEAR`).
It pins *"Carryover from prior year"* at **13**, *"Add lines 11 through 13"* at **14**, *"Add the
amounts in the far right column for lines 4 through 16."* at **17**, and covers **no 15 and no 16 at
all**. So the label census, on its own, **cannot notice a 2026 renumber**.

★ It is not unprotected, and this refines rather than adds a finding. `xtask line-coverage`'s
`MAX_UNQUOTED_BUNDLED_YEARS` ratchet already contains `f1040sa--2025`. Live-measured this run:

```
line-coverage OK: 377 money lines across 18 form(s) [... f1040sa:21 ...] ... ;
12 bundled year(s) quoted from another (ratchet 12)
```

so bundling `forms/2026/f1040sa.map.toml` makes it **13 > 12** and **REDS**, naming the pair.

★★★ **But the ratchet reds on a COUNT and names a `(form, year)`; it never says which lines moved — and
for one struct serving two revisions the only way to satisfy it is FR-162's revision axis.** That is the
urgency answer:

* **Nothing ships a wrong TY2026 Schedule A today.** `full_return_for(2026)` is `None`, `forms/2026/`
  holds only `YEAR.toml` (no map, no template), and Tier A's A-1 `panic!` aborts before Schedule A is
  even reached.
* **The moment TY2026 params + a map land, four of the six become wrong figures on a printed page** —
  one of them the itemized total that feeds 1040 line 12e, one a boolean in a money cell, and one a
  required subtotal that simply is not there.
* So **FR-162 is gating on the TY2026 port and must land BEFORE `forms/2026/f1040sa.map.toml`**, in the
  same slot as Tier A's A-8. Landing it after means the first TY2026 packet is emitted through a TY2025
  line set with only a count-ratchet between it and the filer.

★ Independent cross-check of this file's cell classification: `xtask line-coverage` reports
`f1040sa:21` money lines; `cells_the_struct_expresses`' `Kind::Money` set has **21** entries. Two
instruments built from different sources agree on the count.

---

## 6. THE SUBSTITUTION LIST — two items, both stated in the file's own header

| # | substituted | from | why it is legitimate |
|---|---|---|---|
| **W5-1** | the **year** the chain is assembled at: **TY2024** | the only year whose `FullReturnParams` are bundled (`BundledFullReturnTables::load()` inserts 2024 alone) | `assemble_absolute` at 2026 **aborts** (A-1) and a test cannot bypass it without editing shipped source. TY2026's own `ScheduleAParts` is then substituted into `AbsoluteReturn::schedule_a` — **the one field `schedule_a_lines` reads for every Schedule A cell** — at the engine's own AGI from the same assembled return, so two runs differ in *nothing but the parameters*. |
| **W5-2** | a **TY2025** parameter set | `ty2026_full_return()` with line 5e's figures replaced by the ones the **2025 extract prints**, parsed from the text layer | it exists only to be the control. Nothing else about it is used or claimed. |

**Nothing else was substituted and no figure was invented.** TY2024 and TY2025 share the 11–18 line set
(verified against `f1040sa--2024.txt`), so W5-1 does not smuggle in a third line-set revision.

---

## 7. B1 — EVERY PROBE SEEN DISCRIMINATING, WITH NO SHIPPED SOURCE EDITED

| probe | its kill |
|---|---|
| the caption / move derivation | `a_reverted_caption_is_no_longer_detected_as_a_move` — plants 2025's line-14 caption back at 14 in the text the checker reads; the verbatim-move detector must drop `14 ← 13` and report only `19 ← 18` |
| the 5e figure parser | `the_salt_figure_parser_follows_the_text_it_is_given` — plants `$41,900` for `$40,400`; only that figure may move, and it must |
| the unexpressible-cell difference | `dropping_a_cell_from_the_classification_makes_it_unexpressible` — removes `13` from the classification and demands it surface |
| the source scanner (the floor wall pin) | `the_source_scanner_actually_reads_files` — positive control on `§170(b)` (at least 3 hits, `charitable.rs` among them) and negative control on a string no file contains |
| **the discriminator itself** | self-killing: `(l24.line5e, l25.line5e, l26.line5e)` must be **three distinct values**. If the chain stopped reading `params.salt` they collapse to one and the test reds. |
| the year-blindness claim | **compiler-held**: `let _: fn(&AbsoluteReturn, Usd) -> Option<ScheduleALines> = schedule_a_lines;`. Give the function a year, a `&FullReturnParams`, or a revision selector and this file stops compiling. |
| the cell set | **compiler-held**: `ScheduleALines` destructured with **no `..`**, every binding consumed. A new `line15` does not compile until it is classified. |

★★ **B1's "cannot be satisfied performatively" property fired for real.** Writing the caption kill
exposed a genuine blindness in this file's own `norm()`: the first draft popped trailing line ids in a
**loop**, which ate the caption's own operands — *"Add lines 8a through 8c  8e"* became *"Add lines 8a
through"*, and so did 2026's *"Add lines 8a through 8d  8e"*, so the two revisions compared **equal**
and **line 8e silently dropped out of the changed set**. A checker that erases the very numbers it is
comparing is precisely the class `CLAUDE.md` guards against. One pop is the answer-box; the reasoning is
now in the function's doc comment.

---

## 8. GATE — foreground, serial, in the worktree's own target dirs

Run after `find crates -name '*.rs' -exec touch {} +` (the `make gate` discipline), each command in the
**foreground**, nextest and clippy **serially**, `CARGO_TARGET_DIR` under the worktree (never `/tmp`):

```
cargo nextest run --workspace --no-fail-fast   -> 3788 tests run: 3788 passed, 12 skipped   exit 0
cargo clippy --workspace --all-targets --all-features -- -D warnings
                                               -> 0 warnings, 0 errors                     exit 0
cargo fmt --all -- --check                     -> clean                                    exit 0
cargo run -p xtask -- line-coverage            -> line-coverage OK (quoted in section 5)    exit 0
```

`ty2026_schedule_a` contributes **18** of the 3788. `make check` excludes `cargo fmt --all --check`, so
it was run separately, as the brief requires. The working tree carries one untracked path and no
modifications. **Nothing committed. Nothing pushed. No subagents dispatched.**

---

## 9. FOLLOW-UP CANDIDATES (for the controller to file; I filed none)

| id | item | owning phase |
|---|---|---|
| **W5-1** | ★★★ **FR-162 is gating on the TY2026 port and must land BEFORE `forms/2026/f1040sa.map.toml`.** Four of Schedule A's six collisions are wrong FIGURES, not labels — including the itemized total (17), a boolean in the total's cell (18), and a missing required subtotal (15). The only gate in front of them is a **count** ratchet that names a `(form, year)` pair. | the TY2026 port, **before** any `forms/2026/` Schedule A map — the same slot as A-8 |
| **W5-2** | ★★★ **§170(p)'s 0.5%-of-AGI charitable floor is an UNDERSTATEMENT with a measured size** — `$1,366` on the owner's own profile — and **0** modelling of it exists anywhere in `crates/`. It is what makes line 13 a wrong *figure* rather than a wrong *label*. Blocked on `i1040sca--2026` for the worksheet's line structure, **not** for the direction. | the TY2026 port; archiving `i1040sca--2026` is its prerequisite |
| **W5-3** | `return_1040.rs:3945`'s **filer-facing refusal** tells the filer *"{next}'s Schedule A line 13"*. For `next = 2026` that is the worksheet line, not the carryover. Year-free prose in a refusal — the same class as A-4's `UnmodeledDeductionsOmitted`. Find them together or they arrive a round apart. | the TY2026 port |
| **W5-4** | `cover_schedulealines` covers **no line 15 and no line 16** on any revision, and is quoted at **2024** only. When 15 becomes a required subtotal and 16 becomes casualty, the census has nothing to re-point. | with FR-162 |
| **W5-5** | Citation drift, both Nits: `return_1040.rs:3918` → **3945** (brief and `FOLLOWUPS.md:8309`); FR-185's *"17a–17h+"* → **17a–17k plus 17z**, twelve sub-ids. | opportunistic |
| **W5-6** | The brief's adverse mechanism (reading line 13 as the carryover) is **not** an arithmetic path — the carryover is computed, never read off a line. FR-185's *"★ The adverse one, in the UNDERSTATING direction"* paragraph should be re-pointed at §170(p), or it will send a future reader hunting a bug that is not there while the real one sits beside it. | with FR-185 |

---

## 10. AND ONCE MORE

Every figure above was produced by an engine reading **unbundled** parameters against a **draft** form,
with no oracle in existence and two required worksheets not archived. They are evidence about
**mechanisms**, and about the **direction** of two gaps (§170(p) unapplied means tax understated; the
line-set renumber means four wrong figures the moment a map exists). They are not evidence that any
number on any TY2026 return is correct.
