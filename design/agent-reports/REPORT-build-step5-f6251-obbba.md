# REPORT — step 5 build: `f6251/2025` wired, the OBBBA-era Form 6251 transcription

**Builder:** one opus agent, shared main tree, started at `1bafc125`. **Nothing committed.**
**Brief:** `design/agent-reports/BRIEF-build-step5-f6251-obbba-transcription.md`.
**Gate:** `make gate` **3589 passed / 12 skipped, exit 0** (baseline 3571 / 12); `cargo fmt --all --check` clean.

---

## 0. REFUTED PREMISE — report first, per the brief's §0

### R1 (Important). The brief's §2 says *"the 2025 → 2026 TEXT delta is exactly two cells."* It is **eight numbered lines**, and the brief missed a **second cross-reference**.

Measured by normalising both text layers (collapse whitespace, drop `pdftotext`'s dot-leader runs) and
diffing `design/forms/extract/f6251--2025.txt` against `f6251--2026-DRAFT.txt`:

| line | TY2025 final | TY2026 draft | kind | in the brief? |
|---|---|---|---|---|
| **1a** | Schedule 1-A line **37** | line **43** | **cross-reference** | yes |
| **4** | MFS threshold **$900,350** | **$640,200** | printed constant | yes |
| **5** | 626,350/88,100 · 1,252,700/137,000 · 626,350/68,500 | 500,000/90,100 · 1,000,000/140,200 · 500,000/70,100 | printed constants | **no** |
| **7** | "…capital gain distributions directly on Form 1040 or 1040-SR, line **7**;" · $239,100/$119,550/$4,782/$2,391 | "… line **7a**;" · $244,500/$122,250/$4,890/$2,445 | **cross-reference** + constants | **no** |
| **18** | $239,100/$119,550/$4,782/$2,391 | $244,500/$122,250/$4,890/$2,445 | constants | **no** |
| **19** | $96,700/$48,350/$64,750 | $98,900/$49,450/$66,200 | constants | **no** |
| **25** | $533,400/$300,000/$600,050/$566,700 | $545,500/$306,850/$613,700/$579,600 | constants | **no** |
| **39** | $239,100/$119,550/$4,782/$2,391 | $244,500/$122,250/$4,890/$2,445 | constants | **no** |

The brief's §2 is right that **1b, 2a–2t and 3 are identical word for word** and that 2a cites Form 1040
line 12e in both — its "exactly two cells" claim is true of **Part I** and false of the form. Everything
else in §2 re-measured as stated (see §1 below).

**I did not stop on this**, because the refutation does not invalidate the task — it makes the §3 design
requirement *bigger*, and stopping would have cost the round. What changed as a result:

* the per-revision cell table carries **three** cells, not two: line 1a, line 4, and **line 7's Part III
  routing clause** — the cross-reference the brief did not know about;
* the six constant-only lines are deliberately **not** transcribed per revision (§4 below states why, and
  what that leaves uncovered).

### R2 (finding, not a refutation). The TY2025 Form 6251 cites a Form 1040 line that the TY2025 Form 1040 does not have.

`f6251--2025.txt` row 65 prints *"capital gain distributions directly on Form 1040 or 1040-SR, line 7"*,
while `f1040--2025.txt` row 81 prints that line as **7a** — and `map.rs:764`'s own doc comment already
records "line 7a in 2025 / line 7 in 2024". The TY2026 draft corrects the 6251 to "line 7a". So the
2025→2026 change at line 7 is the **IRS fixing its own stale cross-reference**, not a renumber.

Transcribed **verbatim as printed** (`f6251_revision.rs:157`), with the discrepancy recorded in the
field's doc comment. Writing "7a" into the TY2025 transcription would be adjudicating against the form,
which is backwards: the PDF is the authority and an inconsistency in it is recorded, not repaired.

---

## 1. §2's facts, re-measured (all confirmed)

| fact | verdict |
|---|---|
| `LineSet::F6251_2025 => Schema::Unwired` | confirmed at `line_set.rs:323` (pre-change) |
| the Unwired pin asserted exactly two entries | confirmed, `line_set.rs:357-363` (pre-change) |
| `Form6251Map` carries the 2024 single `line1` | confirmed, `map.rs:302` |
| every map struct carries `#[serde(deny_unknown_fields)]` | confirmed |
| the 2025 map is 24,491 bytes, `line_set = "f6251/2025"` | confirmed |
| 2025→2026 FIELD delta nil — 62 fields, 0 renamed, 0 moved | confirmed, `design/TY2026_WORK_LIST.md:41` |
| `AmtParams.mfs_kicker_start` exists; `ty2026_full_return()` sets `dec!(640200)` | confirmed, `tables.rs:312`, `tax_tables.rs:315` |
| core's `Schedule1A` ends at `line38` (the TY2025 revision) | confirmed |
| 1a subtracts Schedule 1-A line **37** in 2025, **43** in the 2026 draft | confirmed from both text layers |
| line 4 prints **$900,350** in 2025, **$640,200** in the 2026 draft | confirmed |
| 2a cites Form 1040 line **12e** in both years | confirmed — the coordinator's second error stands corrected |

---

## 2. What was built, and where

### New files

| file | lines | what |
|---|---|---|
| `crates/btctax-forms/src/f6251_revision.rs` | 270 | ★★★ the per-revision **year-varying cells** and the `_`-free match that holds them |
| `crates/btctax-forms/tests/f6251_obbba.rs` | 947 | 13 tests: parse, partition, geometry, the extract-derived line set, the §3 trap, fill + read-back, three planted-defect kills |

### Edited

| location | change |
|---|---|
| `line_set.rs:267` | `Schema::Form6251ObbbaMap`, new variant |
| `line_set.rs:348` | `LineSet::F6251_2025 => Schema::Form6251ObbbaMap` (was `Unwired`) |
| `line_set.rs:287-299` | `Schema::Unwired`'s doc comment rewritten: it is now **one** revision, `f1040s1a/2025`, with the owner's 2026-09-11 ruling and the TY2026-rebuild reason written in — a settled destination, not a queue position |
| `line_set.rs:391` | the shrink-only pin: `["f1040s1a/2025"]` (kill 7) |
| `line_set.rs:23-31` | module doc: a new ★★★ block naming many-to-one as the hazard, with the measured delta |
| `map.rs:566` | `pub struct Form6251ObbbaMap` — 42 money cells (1a, 1b, 2a, 2b, 3, 4–40) + identity + the §4 row + census/direction/subtracts, `deny_unknown_fields` |
| `map.rs:740` | `impl`: `ty2025()`, `for_year()`, `parse()`, `revision()`, `money_cells()` (exhaustive destructure, no `..`) |
| `map.rs:800` | `Form6251ObbbaMap::revision()` — resolves this map's own year-varying cells |
| `map.rs:4515` | `pub fn parses_into_its_schema(LineSet)` — exhaustive over `Schema`, replaces a hand-written per-stem dispatch (see §6) |
| `map.rs` (`Form6251Map::for_year` doc) | the stale "TY2025 is deliberately NOT wired" note replaced |
| `form6251.rs:239` | `F6251_OBBBA_CLUSTERS` = `&[(504.0, 576.0), (410.4, 481.6)]` — **two** columns |
| `form6251.rs:269` | `pub fn fill_form_6251_obbba_with_map` |
| `lib.rs` | `pub mod f6251_revision;` + `testonly` exports |
| `tests/line_set_wiring.rs` | rewritten — derived over `LineSet::ALL`, dispatching on each revision's own schema (see §6) |
| `tests/supported_years_cross_product.rs` | `map_resolves`'s `f6251` arm now tries both structs; `(2025,"f6251",&["dispatch"])` deleted from `KNOWN_GAPS`; new derived cross-check `the_per_stem_dispatch_agrees_with_the_derived_dispatch` (see §6) |
| `tests/map_pdf_conformance.rs` | the "TY2025 has no bundled 6251 map" message corrected — it has one, of a different revision |
| `design/FORM_AUTHORITY_TABLE_DESIGN.md` §10 step 5 | marked nine-of-ten wired, with the measured delta and R1 recorded |
| `design/ROADMAP_STATUS.md:288` | "exactly the two" → the one |

`Form6251Map` was **not** edited except for one stale doc sentence. `packet.rs` was **not** wired.
No `FullReturnParams`, no fail-closed gate, no Schedule 1-A emitter touched.

---

## 3. The §3 design requirement — shape chosen, and why

**Neither (a) nor (b) alone. A hybrid, because (a) as stated does not hold the trap.**

* **(a) alone fails.** One struct per revision does force an arm in `line_set::schema` — but the arm can
  say `Schema::Form6251ObbbaMap` for `f6251/2026` and compile. A separate struct per revision only
  separates **field sets**, and the field sets are byte-identical (62 fields, 0 renamed, 0 moved). It
  would red on nothing.
* **What was built:** a separate struct for the **field set** (because the field set genuinely differs
  from TY2024 — one line-1 box versus two), plus **(b)'s per-revision table for the year-varying printed
  cells**, held by an `_`-free match over `LineSet`.

Three legs, because one is not enough:

1. **Forgetting is a compile error.** `f6251_revision::revision(ls)` is `_`-free over `LineSet`
   (`f6251_revision.rs:170`). Pointing `f6251/2026` at this schema without an arm is `E0004` — observed,
   pasted in §5.
2. **An empty arm is a test red.** `=> None` compiles, so
   `every_revision_on_this_schema_has_its_year_varying_cells` derives the revision list from
   `LineSet::ALL` + `schema()` and reds — observed, §5.
3. **A copied arm is a test red.** `the_year_varying_cells_are_verbatim_in_that_revisions_own_extract`
   asserts every stated cell verbatim in **that revision's own** archived extract, and re-derives the
   Schedule 1-A line number from the extract independently. A cell copied from the sibling revision reds
   — observed, §5.

**No dollar literal.** Line 4's figure is never a constant in this crate. The struct's `line4` is a
`MoneyCell` (a PDF field name); the revision's `line4` is the form's **sentence**, and
`ObbbaRevision::mfs_threshold_printed()` **parses the figure out of that sentence** rather than carrying
it as a second key that could disagree with it — the `SubtractSentence` precedent already in `map.rs`.
Same for `schedule_1a_line()` and `form_1040_capital_gain_line()`. All three fail closed (`None`) on a
sentence without the form's shape, and the emitter refuses on `schedule_1a_line() == None`
(`form6251.rs`, the revision gate) rather than defaulting to line 0.

---

## 4. Transcription provenance, and the boundary I am stating

**Source: `design/forms/extract/f6251--2025.txt` only** — the text layer of the FINAL 2025 document
(sha256 `6995bfd2…`, "Form 6251 (2025) Created 9/17/25"). The 2026 draft was read for **evidence only**
and no figure from it is encoded anywhere; `ObbbaRevision::extract` is asserted not to contain `DRAFT`.

| cell | extract rows | verified how |
|---|---|---|
| line 1a sentence | 18–19 | `contains` after the shared normalisation; the Schedule 1-A line number **re-derived from the extract** and compared to the parse |
| line 4 sentence | 44–45 | same; threshold parsed out of the sentence |
| line 7 routing clause | 65 | same; Form 1040 line parsed out of the clause |
| all 42 mapped lines' quotes | whole file | `every_quoted_instruction_is_verbatim_on_its_own_revisions_form` — **42 of 42**, and the checked set is compared to the map's own mapped-line set rather than to a typed count |
| lines 33 / 36 | 136 / 140 | pinned explicitly: 33 subtracts from **22**, 36 from **12** |

### Line 7's clause is a CLAUSE, and that is stated in the source

The text layer interrupts line 7's second bullet with the line-7 box marker (`…complete Part III on the
. . 7 back and enter…`, rows 65–68), so **no contiguous quote of the whole bullet is verbatim in it**.
The cell therefore carries the clause that ends at the semicolon and contains the year-varying
cross-reference, and the field's doc comment says exactly that and why. This is the one place I chose a
sub-sentence, and it is stated rather than trimmed until a checker was satisfied.

### What is deliberately NOT transcribed per revision — and the join nobody owns

The six constant-only lines (5, 7's third bullet, 18, 19, 25, 39) and line 4's dollar **figure** are not
carried here. Those numbers already have exactly one authority per year — `AmtParams` and the year's
`TaxTable` — and re-typing one would create a second, which is how two lists come to disagree.

★ **The honest gap:** `btctax-forms` does **not** depend on `btctax-adapters`, so nothing in this crate
can compare `mfs_threshold_printed()` against `AmtParams::mfs_kicker_start`. The accessor exists so the
comparison is *possible* where both are in scope; today nobody makes it. Stated in
`f6251_revision.rs`'s module doc under "the honest boundary of this module" (`CLAUDE.md` rule 3), and
filed below as a follow-up candidate.

### §4's corroboration evidence — recorded, encoding nothing, no gate touched

* The **2026 draft prints $640,200** on line 4 (`f6251--2026-DRAFT.txt` row 83).
* `ty2026_full_return()` already carries `mfs_kicker_start: dec!(640200)`, derived **statutorily**
  (`tax_tables.rs:239-243`: *"start = the MFS complete-phaseout AMTI ($640,200, §2.10 …) … ★ D8: the
  2026 Form 6251 instructions will print these two; until they do, this is the statutory reading,
  KAT-pinned, to be confirmed AFTER FINALS."*).
* ★ The draft corroborates **more than the kicker**: every figure in `ty2026_full_return()`'s
  `AmtParams` block matches the draft's printed face — exemptions 90,100 / 140,200 / 70,100 (line 5),
  phase-out starts 500,000 / 1,000,000 (line 5's "not over" column), breakpoints 244,500 / 122,250 and
  subtrahends 4,890 / 2,445 (lines 7, 18, 39).
* **A draft is not a final, so this closes nothing.** No gate, KAT or params value was changed.

★★ One adjacent fact the coordinator should have: `ty2026_full_return_must_stay_fail_closed`
(`tax_tables.rs:1231`) names two reasons — *"the 2026 Form 6251 restructured Part I around Schedule 1-A
and needs a re-transcription, and no OTS 2026 exists."* **The first half now partially exists**: the
re-transcription of the restructured Part I is `Form6251ObbbaMap`, and what TY2026 still needs is its
own two cross-reference cells from a FINAL document. The gate stays closed and untouched; I am only
recording that one of its named blockers moved.

---

## 5. Kills — red then green

### 7.1 The 2025 map parses; the TY2024 struct refuses it (both directions, permanent)

RED before the change — `Form6251Map::parse(2025 map)`:

```
TOML parse error at line 80, column 1
   |
80 | line1a = "topmostSubform[0].Page1[0].f1_3[0]"
   | ^^^^^^
unknown field `line1a`, expected one of `form`, `year`, … `line1`, `line2a`, … `identity`
```

GREEN after: `Form6251ObbbaMap::parse` succeeds. The refusal is now a **permanent** assertion in
`the_obbba_map_parses_into_its_own_struct_and_the_ty2024_struct_refuses_it`, which also requires the
message to NAME `line1a`. The reverse direction is pinned too —
`neither_form_6251_revision_accepts_the_others_map`:

```
TOML parse error at line 45, column 1
   |
45 | line1 = "topmostSubform[0].Page1[0].f1_3[0]"
   | ^^^^^
unknown field `line1`, expected one of `form`, … `line1a`, `line1b`, … `identity`
```

### 7.2 A missing required header field → parse refusal

`a_map_missing_a_required_row_field_is_refused` drops each of `irs_stem`, `versioning`,
`template_sha256`, `instructions`, `line_set` (in memory, never on disk) and requires the refusal to
name the dropped key; then drops `line1a` and requires a refusal. Green.

### 7.3 `template_sha256` ≠ the file → refusal, **and the existing gate does traverse this map**

Planted a one-nibble hash mutation in the real `forms/2025/f6251.map.toml` (cp backup, restored):

```
HashMismatch { year: 2025, stem: "f6251" },
NotInManifest { year: 2025, stem: "f6251" },
```

`map_rows.rs::every_committed_map_has_a_row_that_parses_and_all_four_kills_are_green` FAILED. Not
duplicated in the new file — the existing glob-derived instrument covers it, and this is the measurement
that it is not green-without-traversing.

### 7.4 Fill + read-back at the geometry the blank declares; a wrong field name reds

* `the_obbba_fill_writes_both_line_1_boxes_and_reads_back` — 1a = `15000`, 1b = `300000`, 4 = `309500`,
  11 = `2564`; 2b prints `500` (magnitude, not the stored `-500`); Part III lines 12/19/33/40 ABSENT on
  the un-routed path.
* `line_1a_sits_in_its_own_column_and_the_inset_trio_lands_on_the_parenthesised_lines` — from the PDF's
  own `/Rect`s: 1a at x=[410.4, 481.6] (x2 < 500), 1b at x0 ≥ 500, 1a's centre-y above 1b's, and exactly
  three widgets narrower than 70pt of which the mapped one is 2b and the other two are censused.
* `a_map_naming_a_field_the_pdf_does_not_have_refuses_the_fill` — plants `f1_999` in the map text; the
  fill must error naming `f1_999`. (`lopdf` silently writes nothing for a missing field, so without this
  a whole line drops off a filed return with no error.)
* `a_map_with_two_lines_transposed_is_caught_by_the_read_back` — swaps lines 9 and 10's widgets, which
  keeps every FQN existent, the partition intact and both values in the same column; caught only by the
  y-descent leg, and the assertion requires the message to say `descent`.

★ Two ★ extra hazards the geometry found, which the brief did not name: **line 1a is in a different
column** (so a single-cluster read-back would reject a correct map, and a cluster widened to admit both
would stop distinguishing them — hence two clusters), and the inset trio **moved** from TY2024's
`f1_5`/`f1_9`/`f1_22` to `f1_6`/`f1_10`/`f1_23`, so it is identified by measured WIDTH rather than by a
carried-over name list.

### 7.5 The line set is derived from the extract, and a dropped line reds

`printed_labels` reads the form's own left-hand label column out of the extract: a token of one-to-two
digits plus at most one lowercase letter, or a bare sub-line letter attached to the last numbered
parent. **The label column is measured, not typed** — `lo` is the leftmost such indent in the file and
the window is `lo..=lo+4`, because the archived revisions use {1,3,5} (2024), {1,3,4,5} (2025) and
{23,25} (the 2026 draft, whose whole page is shifted 22 columns right). Measured results:

| revision | printed labels | mapped | censused | partition |
|---|---|---|---|---|
| `f6251--2024` | **59** | 41 | 18 | exact, disjoint |
| `f6251--2025` | **60** | 42 | 18 | exact, disjoint |

★ The **60 independently reproduces `xtask label-census f6251--2025`** — run, and it prints
`# f6251--2025 — 60 labels, 62 boxes`. Two completely different methods (text column vs widget
geometry), one answer.

RED on a planted defect — one census label changed `2t` → `2u`:

```
f6251/2025: the form PRINTS lines ["2t"] and the map neither fills them nor records a census decision
for them. A line nothing populated and a line the inputs left blank are identical on the printed page;
only the first is a defect, and this is it.
```

★ **What this check's window does NOT cover, stated in the source:** a wrapped continuation row whose
first token is label-shaped is rejected by the column window and nothing else — there is exactly one per
revision (row 67 of `f6251--2025`, *"16 of Schedule D (Form 1040)…"*, at column 10). If a future revision
printed a real label outside the window it would be dropped from the printed set — which **fails
closed**, because the map then accounts for a label the extract does not print and the other direction
of the same comparison reds. ★ Also stated: **removing a *mapped* line from the map is a parse refusal**
(the field is required), which is a stricter red than this accounting check; this check's unique reach is
the **census** side and the printed-vs-accounted comparison in both directions.

### 7.6 ★★★ THE KILL THIS ROUND EXISTS FOR — three legs, all observed

**Leg 1 — forgetting the cells is a build error.** Planted a stub `LineSet::F6251_2026` (variant, `parse`,
`as_str`, `ALL`, and a `schema` arm pointing at `Schema::Form6251ObbbaMap`), stated no cells:

```
error[E0004]: non-exhaustive patterns: `LineSet::F6251_2026` not covered
   --> crates/btctax-forms/src/f6251_revision.rs:170:11
    |
170 |     match ls {
    |           ^^ pattern `LineSet::F6251_2026` not covered
...
101 |     F6251_2026,
    |     ---------- not covered
error: could not compile `btctax-forms` (lib) due to 1 previous error
```

**Leg 2 — an arm that compiles and states nothing is a test red.** Added
`LineSet::F6251_2026 => None`:

```
FAIL f6251_revision::tests::every_obbba_revision_has_cells
  f6251/2026 is wired to the OBBBA field map but states none of its own year-varying cells — it would
  inherit another revision's Schedule 1-A line number
```

(the same property is restated as an integration test,
`every_revision_on_this_schema_has_its_year_varying_cells`, so the unit-test binary alone cannot be green
on it).

**Leg 3 — an arm copied from the sibling revision is a test red.** Changed TY2025's `line1a` to say
"line 43" — exactly the copy-from-the-other-revision mistake:

```
FAIL f6251_obbba::the_year_varying_cells_are_verbatim_in_that_revisions_own_extract
  f6251/2025: line 1a's stated text is NOT verbatim in f6251--2025.txt — this is what a cell copied
  from a SIBLING REVISION looks like, and it is the only place that difference is visible
```

All three plants reverted by `cp` backup; `git status` confirms no residue.

### 7.7 The Unwired pin shrinks, and `line_set_wiring.rs`

`line_set.rs:391` now asserts `["f1040s1a/2025"]`. `line_set_wiring.rs` was **rewritten**, because its
helper `parses_into_2024_struct` was exactly the defect class the brief warned about — see §6.

---

## 6. ★★ An instrument that would have gone green without traversing the new revision (FR-114's shape)

Two hand-written lists asked "which struct parses this stem's map?" and both hardcoded *the 2024
struct*. Both were correct on the day they were written and wrong the moment a revision got a struct of
its own.

1. **`tests/line_set_wiring.rs::parses_into_2024_struct`** — a `match` on nine `Stem`s. Its asserted
   equivalence was "wired ⇔ parses into the 2024 struct", so wiring `f6251/2025` would have demanded
   that the 1a/1b map parse into the single-line-1 TY2024 struct — i.e. it would have asserted the exact
   defect `deny_unknown_fields` exists to prevent. Replaced by
   `map.rs:4515::parses_into_its_schema`, `_`-free over `Schema` (a new struct is a build error), with
   the iteration derived from `LineSet::ALL` rather than a hand-list of ten. 37 of 38 revisions parse;
   the one that does not name a struct is `f1040s1a/2025`, recorded with its reason.

2. **`tests/supported_years_cross_product.rs::map_resolves`** — ★ this one **stayed green through the
   whole wiring change**, still recording `(2025, "f6251", &["dispatch"])`: a gap for a map that
   dispatches perfectly well. A recorded gap that is not a gap is the quietest wrong instrument. Fixed
   two ways: the arm tries both structs, and a new test
   `the_per_stem_dispatch_agrees_with_the_derived_dispatch` cross-checks every revision against
   `parses_into_its_schema`, so a third revision-struct added without extending the arm reds. **Verified
   red** by restoring the old single-struct arm:

```
FAIL the_per_stem_dispatch_agrees_with_the_derived_dispatch
  f6251/2025: its own row names a struct that ACCEPTS its bundled map, but `map_resolves` says
  Some(false). The stem's arm is asking a struct that is not this revision's — which records a
  `dispatch` gap for a map that dispatches.
FAIL the_cross_product_matrix_matches_the_recorded_gaps
  (2025, f6251) is missing {"dispatch"} and NOTHING records it.
```

---

## 7. §5's boundary, as stated in the source

Written into `Form6251ObbbaMap`'s doc comment (`map.rs:566`), the filler's
(`form6251.rs:269`), and `tests/f6251_obbba.rs`'s module doc:

* **Nothing exercises this struct through a packet, and for TY2025 nothing ever will** — owner ruling
  2026-09-11, quoted; `full_return_for(2025)` is a tested `None`. **The tests in `f6251_obbba.rs` are
  the whole exercise of this code path; there is no packet-level backstop behind them.** Said in the
  test file too, so a reader cannot mistake a green suite for end-to-end coverage.
* **What it is for:** it is **TY2026's** field map and emitter, built against a FINAL document a season
  early, because the field delta is nil.
* **`packet.rs` is deliberately not wired.** It still names `Form6251Map::for_year(year)`, so a TY2026
  packet REFUSES (`Structure`, naming this schema) until someone teaches it to dispatch on
  `line_set::schema`. That refusal is the honest state; the alternative it replaces is filling a 1a/1b
  PDF through the single-line-1 map, which lands **line 11, the AMT itself,** in line 10's box with
  nothing red.
* No fixture pretends a year computes; no `FullReturnParams` bundled; no fail-closed gate touched.
* `Schema::Unwired` now documents `f1040s1a/2025` as a **settled destination**, per brief §6: the owner's
  ruling plus TY2026's Schedule 1-A being a rebuild (10 of 219 fields survive). No Schedule 1-A emitter
  built.

---

## 8. Validation — literal numbers

| instrument | result |
|---|---|
| `make gate` (forced rebuild; nextest + clippy `-D warnings`) | **3589 passed / 12 skipped, exit 0** (baseline 3571 / 12 → **+18**) |
| `cargo fmt --all --check` | clean |
| `xtask line-coverage` | OK — 375 money lines across 18 forms, 31 exceptions (ratchet 31), 0 unverifiable, 17 not line-bound |
| `xtask census-join` | OK — 274 unmodeled entries across 13 maps |
| `xtask box-census` | OK — 268 printed boxes across 19 archived editions of 9 information returns |
| `xtask cite-check` | OK — 51 quotations, all verbatim; 7/38 pairs authority-archived, 31 excused, 0 unaccounted |
| `xtask stop-list` | OK — no forbidden shape |
| `xtask harness-check` | OK — 2 hooks wired |
| `xtask archive-check` | OK — 3 accounted-for archives |
| `xtask authority-conflicts` | OK — 0 undecided, 0 overdue |
| `xtask prompt-check` | OK — 90 assertions, all verbatim |
| `xtask label-census f6251--2025` | `# f6251--2025 — 60 labels, 62 boxes` — corroborates the derived 60 |
| `btctax-forms` alone | 390 run, 390 passed, 4 skipped |

### ★ Instruments that are green **without** traversing the new revision — reported, not fixed

These are boundary statements, not claims of coverage:

* **`xtask line-coverage`** prints `f6251:41` — core's `line_coverage` table for Form 6251 is the
  **TY2024** revision (41 money lines, checked against `f6251--2024.txt`). The 1a/1b revision has **no
  coverage rows at all**. That is core's computation-side surface, not the forms crate's, and out of
  this brief's scope.
* **`xtask cite-check`** lists `f6251--2025` among its 31 excused pairs, so it reads none of the 2025
  extract. `tests/f6251_obbba.rs` is what reads it now.
* **`field_census.rs`** already covered `forms/2025/f6251.map.toml` before this change (it text-scans
  every map); nothing about the wiring changed what it sees.

---

## 9. Findings for the coordinator (none blocking this build)

| # | severity | finding |
|---|---|---|
| F1 | **Important** | §0 R1 — the brief's "exactly two cells" was wrong; **eight** lines differ and the **line 7 → 7a cross-reference was missed**. Folded into the design record. |
| F2 | Minor | §0 R2 — the TY2025 Form 6251 cites Form 1040 line **7** where the TY2025 Form 1040 prints **7a**; the 2026 draft corrects it. Transcribed as printed, discrepancy recorded in the source. |
| F3 | Minor (out of scope) | **`btctax-core`'s `Form6251Line1::Y2025` doc comment hardcodes "line 37"** (`form6251.rs:53-60`). Its *shape* (1a/1b) serves TY2026 too, so a TY2026 computation reusing `Y2025` would subtract Schedule 1-A line 37 instead of 43 — the **computation-side twin** of the defect this build closed on the forms side, and the forms-crate table cannot reach it. Recommend a follow-up: core's line-1 variant should read its Schedule 1-A line from a per-year source, or gain a `Y2026`. |
| F4 | Minor (out of scope) | Nothing joins `ObbbaRevision::mfs_threshold_printed()` to `AmtParams::mfs_kicker_start` — `btctax-forms` does not depend on `btctax-adapters`. The accessor exists; the comparison has no owner. A natural home is `xtask`, which already reads both the extracts and the crates. This is the §55(d)(3) collision class the repo has been bitten by. |
| F5 | Nit (pre-existing) | `Form6251Map`'s doc comment and both f6251 maps' prose say lines 2c–2t are censused as **`gap` (not `unmodeled`)**, while all 36 committed entries carry `rule = "unmodeled"` — and `form6251.rs`'s module doc argues at length for `unmodeled` being correct. The prose in the struct doc and the map headers contradicts the data and the reasoned module doc. Untouched (pre-existing, both revisions); the new struct's doc does not repeat the wrong claim. |
| F6 | informational | `ty2026_full_return_must_stay_fail_closed`'s first named blocker — *"the 2026 Form 6251 restructured Part I around Schedule 1-A and needs a re-transcription"* — is now partially retired (§4). The gate is untouched and stays closed. |

## 10. Mechanics

No commit, push, `git stash`, `git checkout` or revert. Three plants, each reverted by `cp` backup from
the session scratchpad; a throwaway capture test (`tests/zz_capture_tmp.rs`) was created, run, and
deleted. No subagents. No `--no-verify`. Working tree at hand-off: 9 modified files, 2 new files,
nothing else.
