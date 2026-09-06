# Build report — Form 4868 / Form 1040-V, spec tasks T1 + T5

Implementer: single agent, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, from HEAD
`a0e90f1b`. Nothing is committed or pushed. Brief: `design/agent-reports/BRIEF-build-4868-T1-T5.md`;
spec: `design/SPEC_form_4868_1040v.md` (GREEN r6), R1 and plan items T1 / T5.

**Result: both tasks landed. Every suite for every crate I touched is green, and so is every other
workspace crate.** 3105 → **3109** tests (the four new kills). `cargo fmt --all --check` clean;
`cargo clippy --workspace --all-targets --locked` emits nothing.

---

## 1. What changed, file by file

### New files (8)

| file | what |
|---|---|
| `crates/btctax-forms/forms/2024/f4868.pdf` | byte-for-byte `cp` of `design/forms/2024/f4868--2024.pdf` (sha256 `8d1a031a…`, 531664 B) |
| `crates/btctax-forms/forms/2025/f4868.pdf` | byte-for-byte `cp` of `design/forms/2025/f4868--2025.pdf` (sha256 `36c7607d…`, 529415 B) |
| `crates/btctax-forms/forms/2024/f1040v.pdf` | byte-for-byte `cp` of `design/forms/2024/f1040v--2024.pdf` (sha256 `84fcdaf1…`, 97292 B) |
| `crates/btctax-forms/forms/2025/f1040v.pdf` | byte-for-byte `cp` of `design/forms/2025/f1040v--2025.pdf` (sha256 `2426bb33…`, 97663 B) |
| `crates/btctax-forms/forms/2024/f4868.map.toml` | the TY2024 row + 12 bindings + a 5-entry `[census]` (17 fields) |
| `crates/btctax-forms/forms/2025/f4868.map.toml` | the TY2025 row + 12 bindings + a 5-entry `[census]` (17 fields) |
| `crates/btctax-forms/forms/2024/f1040v.map.toml` | the TY2024 row + 12 bindings + a 3-entry `[census]` (15 fields) |
| `crates/btctax-forms/forms/2025/f1040v.map.toml` | the TY2025 row + 12 bindings + a 3-entry `[census]` (15 fields) |

Every FQN in all four maps was taken from `cargo run -p xtask -- dump-fields` on **that year's
archived PDF**, never from the brief. The two 2024↔2025 differences the brief warned about are both
real and are why the 4868 maps are not copies of one another: the Part I subform is `Part1_ReadOrder`
in 2024 and `PartI_ReadOrder` in 2025, and the page-3 confirmation cell is `Page3[0].Col3[0].f3_1[0]`
in 2024 and `Page3[0].Col4[0].f3_1[0]` in 2025. The 1040-V's 15 FQNs are identical across the years
(diffed with the same command, names **and** rects).

### Edited files (13)

| file | what |
|---|---|
| `crates/btctax-forms/src/bundled.rs` | `Stem::{F1040v, F4868}` — variant, `ALL`, `file_stem`. `from_file_stem` and `Display` derive from those, so no further arm. **No other exhaustive `match` on `Stem` existed**: the build was clean on the first compile after this edit, which is the compiler's own answer to "list every site". |
| `crates/btctax-forms/src/line_set.rs` | `LineSet::{F1040v_2024, F4868_2024, F1040v_2025, F4868_2025}` (+ `parse`, `as_str`, `ALL`); `Schema::{Form1040VMap, Form4868Map}`; four `schema()` arms. |
| `crates/btctax-forms/src/map.rs` | `Form4868Map` and `Form1040VMap` — row keys, top-level `String` bindings, `census`, plus `parse` / `ty2024` / `ty2025` / `bundled_pdf` / `for_year` / `field_names`, in `Form8959Map`'s and `Form8275Map`'s shape. Line 8 is a `CheckChoice { field, on }` — the simplest existing checkbox convention in the corpus. |
| `crates/btctax-forms/src/lib.rs` | `testonly` re-exports both new types. |
| `crates/btctax-forms/forms/{2024,2025}/YEAR.toml` | `forms_expected` gains `"f1040v"` and `"f4868"`. |
| `crates/btctax-forms/forms/{2017,2026}/YEAR.toml` | `[forms_absent]` gains both, with a reason each. |
| `crates/btctax-forms/tests/map_rows.rs` | the two row-gate edits (below), the row floor, and one new kill. |
| `crates/btctax-forms/tests/map_pdf_conformance.rs` | the typed fieldset guard for the four new rows + its B1 kill. |
| `crates/btctax-forms/tests/supported_years_cross_product.rs` | `map_resolves` arms, `BUNDLED_FORMS_PER_YEAR`, the `BUNDLED.len()` liveness floor. |
| `crates/btctax-forms/tests/year_record.rs` | the anti-shrink expected-count pin. |
| `crates/xtask/src/cite_check.rs` | the row-count equality, two doc-comment counts, and `AUTHORITY_NOT_YET_ARCHIVED`. |
| `crates/xtask/src/label_reader.rs` | **all of T5** (below). |
| `design/TY2026_WORK_LIST.md`, `docs/examples/examples.md` | consequences; see §6. |

---

## 2. The three row-gate edits (R1 / spec I-3)

**(a) `map_rows.rs` — `attachment_sequence`.** `assert_eq!(r.attachment_sequence.is_none(), r.form ==
"f1040")` became `matches!(r.form.as_str(), "f1040" | "f4868" | "f1040v")`, with a comment saying
plainly that the widened shape check is a hand-list of three and that what actually holds the value is
kill 4 in the same file — the comparison against `printed_sequence(<that year's archived extract>)`.
Confirmed by measurement: `grep -c "Sequence No"` on `f4868--{2024,2025}.txt` and
`f1040v--{2024,2025}.txt` is **0** on all four.

**(b) `map_rows.rs` — `instr_pages` is now a SET**, not one pinned row:

```
(2024, "f1040v", [1, 2]), (2024, "f4868", [1, 4]),
(2025, "f1040s1a", [101, 110]), (2025, "f1040v", [1, 2]), (2025, "f4868", [1, 4]),
```

**(c) `cite_check.rs:1378`** — `every_archived_rows_instructions_stem_is_a_manifest_entry`. Verified
to pass **as it stands**: `instructions = "f4868"` resolves to `design/forms/<year>/f4868--<year>.pdf`
and both years are `MANIFEST.json` entries (lines 93–108 and 399–414). No edit was needed there. The
cite-check test that *did* need an edit was a different one — see §6.

---

## 3. `instr_pages` — what I measured, and a DEVIATION from the brief

The brief predicted `4868: pages 2–4`. **I measured [1, 4] and recorded that instead**, and the
1040-V as [1, 2]. Measurement, from the committed extracts' form-feed page breaks:

| extract | form feeds | pages | pages carrying instruction text |
|---|---|---|---|
| `f4868--2024.txt` | 4 | 4 | 1 (above the `DETACH HERE` rule: *"There are three ways to request an automatic extension…"*, *"Qualifying for the Extension"*), 2, 3, 4 |
| `f4868--2025.txt` | 4 | 4 | same |
| `f1040v--2024.txt` | 2 | 2 | 1 (upper half: *"What Is Form 1040-V?"*, *"How To Fill in Form 1040-V"*, *"How To Prepare Your Payment"*), 2 |
| `f1040v--2025.txt` | 2 | 2 | same |

**Why I did not use [2, 4].** Both forms put the *form* on the bottom of page 1 and *instructions* on
the top of page 1, so "exclude page 1" is the only rule that gets [2,4] — and applied consistently it
would give the 1040-V [2, 2], which drops *"How To Fill in Form 1040-V — Line 1. Enter your social
security number…"*, the single block a citation fixture most needs. The rule I applied instead is
"the pages that carry instruction text", which is honest about both forms and consistent between them.
`instr_pages` is consumed by `cite_check::extract` as `pdftotext -f <first> -l <last>`, so [1,4] /
[1,2] slices exactly the instruction text and nothing is lost. Flagging it here because the spec does
not pin the numbers and a reviewer with the brief in hand would otherwise see a discrepancy.

---

## 4. T5 — the reader walk

`crates/xtask/src/label_reader.rs`, all inside `#[cfg(test)] mod map_label_join_tests`. **The reader
proper is untouched**; `label_join` still refuses the 1040-V, as the spec requires.

- **`GRID_MAPS`** gains `("2024","f1040v", …)` and `("2025","f1040v", …)` with the measured reason.
- **New pure `grid_reason(year, form, keys) -> Option<&'static str>`** — `Some` iff `keys == 0` **and**
  `GRID_MAPS` names the pair.
- **New pure `disposition(year, form, stem, keys) -> Disposition`** — `Unwitnessed` / `Grid` / `Join`,
  deciding the **`m.stem` geometry join FIRST and the grid declaration SECOND** (r4 R4-I1). That order
  is the whole guarantee and is why this is a function rather than three inline `if`s: a declared grid
  whose fixture is gone is still `Unwitnessed`, so deleting a declared grid's fixture still reds its
  year.
- **`YearReach` gains `grid: Vec<String>`**, printed on every run as `DECLARED GRID <form> — grid: …`,
  and counted in **neither** bucket. **Neither `max_unwitnessed` was raised** (2024 stays 1, 2025 stays
  0).
- The walk consults `disposition` after computing `keys`, before `label_join`.

### The measured caption offsets (the reason the 1040-V is a grid)

`pdftotext -bbox` against the dumped widget rects, `f1040v--2025.pdf` page 1: the box-1 caption `1`
occupies PDF y **156.5–164.8** while its widget `f1_1` is y **132.0–144.0** — 20.8pt top-to-top,
12.5pt of clear space *above* the field. The box-3 caption `3` sits at x **327.3–331.2** while its
widget `f1_3` starts at x **460.8** — **129.6pt to its right**. Captions, not line labels. Recorded in
both 1040-V maps and in both `GRID_MAPS` reasons.

### Label-walk numbers, before → after

| year | maps | joins | unwitnessed | declared grids |
|---|---|---|---|---|
| 2017 | 5 → 5 | 0 → 0 | 5 → 5 | 0 → 0 |
| 2024 | 17 → **19** | 277 → **282** | 1 (`f8283`) → 1 | 2 (`f8275`, `f8949`) → **3** (+`f1040v`) |
| 2025 | 15 → **17** | 231 → **236** | 0 → 0 | 2 (`f8283`, `f8949`) → **3** (+`f1040v`) |

Per-map, the 4868 contributes exactly **5 numbered line keys, 5 bindings extracted, 5 joined, 0 not a
geometry box** in each year — `line4`…`line8`, matching `xtask label-boxes` (`f1_11`→"4", `f1_12`→"5",
`f1_13`→"6", `f1_14`→"7", `c1_1`→"8"). The 1040-V contributes 0 by design.

**The spurious `9a` heading the spec anticipated did not surface.** `xtask label-boxes f4868--2024`
and `--2025` print no `9a` at all; the only labels the reader emits for these forms are `4`–`9` and
`?`. Nothing to record a reason for.

---

## 5. Every pinned number I moved — old → new, with its cause

| where | old | new | cause |
|---|---|---|---|
| `line_set.rs` `LineSet::ALL.len()` | 37 | **41** | four new revisions |
| `cite_check.rs` `from_rows.len()` (equality) | 37 | **41** | four new map rows |
| `map_rows.rs` `rows.len() >=` (floor) | 37 | **41** | four new map rows (raised, not left stale) |
| `supported_years_cross_product.rs` `BUNDLED.len() >=` (liveness floor) | 37 | **41** | four new (stem, year) bindings |
| `supported_years_cross_product.rs` `BUNDLED_FORMS_PER_YEAR` | (2024, 17), (2025, 15) | **(2024, 19), (2025, 17)** | two new forms per year |
| `year_record.rs` expected-count pin | 2024 ⇒ 17, 2025 ⇒ 15 | **19 / 17** | same |
| `label_reader.rs` `YEAR_FLOORS[2024].min_joins` | 261 | **282** | **+16 pre-existing drift, +5 the 4868** — see below |
| `label_reader.rs` `YEAR_FLOORS[2025].min_joins` | 215 | **236** | same split: +16 drift, +5 the 4868 |
| `docs/examples/examples.md` (golden, 4 lines) | "15 forms" ×2, "17 forms" ×2 | **"17 forms" ×2, "19 forms" ×2** | the readiness sentence counts bundled forms |
| `field_census.rs` `UNCENSUSED_ENTRIES` / `UNCENSUSED_FIELDS` | 10 / 713 | **unchanged** | the new maps carry a full `[census]`, so no register entry — exactly as the brief required |

### ★ The +16 per year is a PRE-EXISTING stale floor, not my change

Commit `3070a797`'s own message records its measurement: *"2024 261 joins / 1 unreachable, 2025 215 /
0"*. At HEAD `a0e90f1b`, before any edit of mine, the measurement was **277 / 231**. The +16 in each
year comes from `c25f7489` (spec 1099-DA T4), which **mapped sixteen Schedule D cells that had been
censused** — lines `1b`/`2`/`8b`/`9` × columns `d`/`e`/`g`/`h`, in `forms/2024/schedule_d.map.toml`
and `forms/2025/schedule_d.map.toml`, four lines × four cells in each year — without raising this
floor. `min_joins` is a floor and only reds when coverage **falls**, so the stale value passed
silently. Both halves are written into the ratchet's comment so the next reader can tell the drift
from the addition. My own contribution is +5 per year and nothing else.

(How I know HEAD was 277/231 without stashing in a shared tree: the run prints per-map join counts,
`2024/f4868` and `2025/f4868` contribute exactly 5 each, the 1040-V contributes 0 in both the old
shape and the new, and my edits touch no other map. 282 − 5 = 277, 236 − 5 = 231.)

---

## 6. Things I had to touch that the brief did not name

Three, all of them consequences of adding a stem rather than choices:

1. **`crates/xtask/src/cite_check.rs` — `AUTHORITY_NOT_YET_ARCHIVED`.**
   `authority_coverage_may_only_improve` went red: *"btctax can print [f1040v--2024, f1040v--2025,
   f4868--2024, f4868--2025] with no archived primary source for THAT YEAR and no explicit excuse."*
   That ratchet counts a pair as archived only when `FORMS` names it **and** the two
   `crates/btctax-core/src/tax/fixtures/<stem>_{form,instructions}.txt` cite-check extracts exist —
   which today is `f1040s1a/2025` alone; all 36 other emitted pairs are on the excuse list. I added
   `("f1040v", &[2024, 2025])` and `("f4868", &[2024, 2025])` with a comment recording that the PDFs,
   MANIFEST entries, `-layout` extracts and geometry fixtures *are* on disk and that what is missing
   is only the pair of cite-check fixtures — so closing these is a `FORMS` row plus an extract, not an
   archive hunt. `cite-check` now reports `1/41 emitted pairs archived, 40 excused, 0 unaccounted`.
   **This is the one place I widened an excuse list, and it is the same treatment every other bundled
   form-year gets.** Flagging it explicitly for the reviewer.

2. **`design/TY2026_WORK_LIST.md`.** `form_delta::the_committed_work_list_matches_form_delta_at_head`
   went red: *"stems with a map in a bundled year but NO row in either work-list table:
   [\"f1040v\", \"f4868\"]"*. I ran the document's own generator —
   `cargo run -p xtask -- port-status 2025 2026-DRAFT` — and added the two rows it prints, in the
   "Not listed" table, with the claims the printer makes (prior side `f4868--2025` / `f1040v--2025`,
   TY2026 side **NO DRAFT**). Both `the_committed_work_list_matches_form_delta_at_head` and
   `port_status_prints_the_committed_work_list` are green.

3. **`docs/examples/examples.md`.** The `report` readiness sentence prints the bundled-form count.
   Regenerated with `cargo run -p xtask -- examples`; I diffed before overwriting and the diff is
   **exactly four lines**, all form counts (2 × TY2025 15→17, 2 × TY2024 17→19). Nothing else moved.

**One thing I found and did NOT change**, per the brief's "say so rather than doing it":
`btctax_forms::attachment_sequence` (`crates/btctax-forms/src/packet.rs:74`) returns `None` for the
two new stems through its `_ => None` catch-all rather than through an explicit arm. The row gate
`packet_sequences_agree_with_every_map_row` is green because `None == None`, so it is *correct* — but
it is correct by default rather than by decision, which is the shape this repo distrusts. It is
arguably T2/T4's business (the fillers). Worth a follow-up or an explicit arm in the next task.

---

## 7. Kills — every one seen RED on a planted defect (B1), with the red text

Four are permanent tests I added; all eight were **observed** red here. Plants were applied with a
`cp` backup and restored with `cp` (never `git checkout --`); every file's sha256 was re-checked after
restore and the suites re-run green.

**1. The census accounts for 17 + 15 boxes per year — a dropped `[census]` entry reds.**
Deleted the `c1_2[0]` line from `forms/2025/f4868.map.toml`:
```
2025/f4868: 1 field(s) are in NEITHER the map nor the [census] — this is the "we forgot this line"
defect, invisible on the printed page and to both oracles: ["topmostSubform[0].Page1[0].c1_2[0]"]
```
(`census_accounts_for_every_field`; the gate also names its own denominator — *"failed for 1 of 41
committed maps"*, up from 37. The pure-function planted-defect test for the same gate is the
pre-existing `the_gate_reds_on_every_planted_defect`.)

**2. A row with an `attachment_sequence` whose extract prints none reds.**
New permanent test `a_planted_sequence_number_on_a_form_that_prints_none_is_reported` (`map_rows.rs`),
modelled on `a_planted_wrong_sequence_number_is_reported`: it plants `attachment_sequence = "99"` on a
tempdir copy of `2025/f4868` and asserts `SequenceMismatch { row: Some("99"), printed: None }`, then
asserts the *unplanted* copy is clean. Observed red by making the plant a no-op (`|t| t`):
```
a_planted_sequence_number_on_a_form_that_prints_none_is_reported panicked at map_rows.rs:581: []
```
(the empty problem list — the checker reports nothing when there is nothing to report, so the green
result is not vacuous.)

**3. `instr_pages` holds every row that declares pages.**
Changed `forms/2024/f1040v.map.toml` to `instr_pages = [2, 2]`:
```
assertion `left == right` failed
  left: [(2024, "f1040v", [2, 2]), (2024, "f4868", [1, 4]), (2025, "f1040s1a", [101, 110]), …]
 right: [(2024, "f1040v", [1, 2]), (2024, "f4868", [1, 4]), (2025, "f1040s1a", [101, 110]), …]
```

**4. The typed fieldset guard — `Form4868Map`/`Form1040VMap` `field_names()` are AcroForm names.**
New permanent test `the_4868_and_1040v_maps_name_only_fields_their_own_bundled_pdf_carries`
(`map_pdf_conformance.rs`), per year and against **that year's own** `bundled_pdf`. Observed red by
pointing `line7` at `f1_99[0]`:
```
TY2025 f4868: topmostSubform[0].Page1[0].f1_99[0] is not a field of its own PDF
```

**4b. …and its own B1 kill, which turned out to be REAL.**
`the_4868_1040v_fieldset_guard_reds_when_the_pdf_is_the_other_form` points each map at the *other*
form's PDF. The first cut asserted all 12 bindings would be absent; the run said **8**:
```
assertion `left == right` failed: every one of the 4868's 12 bindings must be absent from the
1040-V's AcroForm; found [… PartI_ReadOrder f1_4 … f1_10, c1_1] missing
  left: 8   right: 12
```
**`f1_11`…`f1_14` — Form 4868's lines 4, 5, 6 and 7, the money — are spelled
`topmostSubform[0].Page1[0].f1_NN[0]` on BOTH forms**, so those four names exist in the 1040-V's
AcroForm too (and `f1_11`/`f1_12`/`f1_13` the other way round). A 4868 map applied to the voucher
would write the balance due into the voucher's name and address boxes with **every field name
resolving**. So an existence check is necessary and *not sufficient* for this pair, which is exactly
why the line→label join matters here. Both directions are now measured and pinned (8 and 9) with that
finding written into the test's doc comment.

**5. T5 — a declared grid with a lost fixture must still red (the branch ORDER).**
Reversed `disposition` to consult `grid_reason` before the `m.stem` join:
```
a DECLARED grid whose fixture is gone must be UNWITNESSED, not laundered as grid:
Grid("f1040v — grid: the voucher's box numbers are cell CAPTIONS, not line labels: …")
```

**6. T5 — a keyless map NOT in `GRID_MAPS` must still red.**
Replaced `grid_reason`'s `GRID_MAPS` lookup with `Some("any keyless map is a grid")`:
```
assertion `left == right` failed
  left: Some("any keyless map is a grid")   right: None
```
(the assertion is `grid_reason("2025", "f1040", 0) == None` — an undeclared keyless map must fall
through to the label join, where `map_reach_problem`'s `(false, 0, …)` arm reds it.)

Both live in the new permanent test
`the_grid_branch_reds_on_an_undeclared_blank_and_never_launders_a_lost_fixture`, which also pins that
a grid-bucket entry spends **none** of `max_unwitnessed` while an unwitnessed one spends it. I also
extended `map_reach_problem_reds_on_every_planted_shape` with three rows: `f1040v` keyless is the
recorded blank, `f1040v` with a numbered key reds, and **`f4868` with no numbered key reds** (the 4868
is *not* a grid — if it ever stopped binding `line4`…`line8`, that must be loud).

**7. End-to-end: undeclaring the grid must not go quiet.**
Removed `("2025","f1040v", …)` from `GRID_MAPS` and ran the real walk:
```
per-year coverage of the line->label join is not what was recorded:
  2025: 1 map(s) this check cannot reach, allowance is 0:
      f1040v — f1040v--2025: no numbered label column found — refusing to report zero labels
```
This is the exact failure T5 exists to prevent — the committed voucher map spending the year's
ratchet — reproduced on demand.

**8. End-to-end: the 4868's five numbered lines are really joined to its own printed labels.**
Swapped the `line4`/`line5` FQNs in `forms/2025/f4868.map.toml`:
```
2 mapped line(s) land on a box the form labels differently — a filled value would print on the
WRONG LINE of a signed return:
  2025/f4868: map says line 4 -> …f1_12[0], but the form prints "5" beside that box
  2025/f4868: map says line 5 -> …f1_11[0], but the form prints "4" beside that box
```

---

## 8. Commands run, with their summary lines

```
cargo nextest run --locked -p btctax-forms            → 344 tests run: 344 passed, 4 skipped
cargo nextest run --locked -p xtask                   → 129 tests run: 129 passed, 1 skipped
cargo nextest run --locked -p btctax-core             → 1211 tests run: 1211 passed, 0 skipped
cargo nextest run --locked -p btctax-cli              → 662 tests run: 662 passed, 1 skipped
cargo nextest run --locked -p btctax-store            → 45 tests run: 45 passed, 0 skipped
cargo nextest run --locked -p btctax-adapters         → 103 tests run: 103 passed, 0 skipped
cargo nextest run --locked -p btctax-input-form       → 65 tests run: 65 passed, 0 skipped
cargo nextest run --locked -p btctax-oracle-harness   → 5 tests run: 5 passed, 1 skipped
cargo nextest run --locked -p btctax-tui              → 159 tests run: 159 passed, 2 skipped
cargo nextest run --locked -p btctax-tui-edit         → 381 tests run: 381 passed, 2 skipped
cargo nextest run --locked -p btctax-update-prices    → 5 tests run: 5 passed, 1 skipped
cargo nextest run --locked -p btctax                  → 0 tests run
                                                        TOTAL 3109 (HEAD was 3105; +4 new kills)

cargo clippy --workspace --all-targets --locked       → no diagnostics
cargo fmt --all && cargo fmt --all --check            → clean
cargo run -q -p xtask -- authority-manifest           → OK — every entry resolves and every source is listed
cargo run -q -p xtask -- cite-check                   → OK — 51 quotations, all verbatim;
                                                        authority archived + extracted for 1/41 emitted
                                                        (form, year) pairs; 40 excused, 0 unaccounted
```

I did **not** run the whole workspace in one invocation, per the brief; `make check` is the
controller's gate. Builds went to the default `target/` (never `target-review/`). No commit, no push,
no subagents.

---

## 9. Decisions I made, and what I could not do

1. **`instr_pages` = [1, 4] / [1, 2], not [2, 4].** Measured, argued in §3. The one place I depart
   from the brief's text; the spec pins no numbers.
2. **Part I bound by name, `line4`…`line8` by number** — spec R1's recorded deviation, implemented as
   written, with the reasoning restated in `Form4868Map`'s doc comment and in both 4868 maps' headers
   so a future reader meets it where the decision lives.
3. **No `[identity]` block on either map.** The 4868's name/SSN cells are `name_line` /
   `taxpayer_ssn` / `spouse_ssn`, and the voucher's are `box1_ssn` / `box4_first_name` / …; adding an
   `IdentityCells` on top would point two keys at one field and red
   `no_committed_map_points_two_lines_at_the_same_pdf_field`.
4. **Line 8 as `CheckChoice { field, on = "1" }`** — the on-state is what `dump-fields` reports
   (`on=["1"]`), not an assumption.
5. **`census` rules are all `unmodeled`, never `gap`.** `GAPS` stays pinned at 0. Each reason names
   what using btctax forgoes: calendar-year-only filing (the three `VoucherHeader` cells), no Form
   1040-NR (`c1_2`), no electronic payment (the page-3 confirmation cell), and a domestic address
   (the voucher's three foreign cells, worded as `forms/2024/f1040.map.toml:202-204` words its own).
6. **Nothing in T2/T4 territory was touched** — no filler, no command, no packet hook. `Stem::F4868`
   and `Stem::F1040v` are reachable, dispatchable and censused, and nothing fills them yet.
