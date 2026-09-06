# Fold review — the x-aware label join (c8a35f03)

Reviewer: Claude Opus 5 (1M), independent, read-only. Date: 2026-09-06. HEAD: `2aa4ea98` (the fold
under review is `c8a35f03`; the four commits since it — `22a4c0c4`, `370b8e80`, `2f5e1315`,
`2aa4ea98` — touch no map, no geometry fixture and not `label_reader.rs`, verified with
`git diff --stat c8a35f03..HEAD`).

Read: `git show c8a35f03` (whole diff, all nine files); `crates/xtask/src/label_reader.rs` in full
(1687 lines); `crates/xtask/src/form_geometry.rs` (`Word`/`Box_`/`box_top_down_y`/`load`);
`crates/xtask/src/form_delta.rs` §`label_moved`/`Unwitnessed`; `crates/btctax-forms/tests/field_census.rs`
`every_emittable_form_is_reached_by_the_gate_or_named_absent`; `tests/year_record.rs`;
`crates/btctax-forms/src/packet.rs`; `crates/btctax-cli/src/year_readiness.rs`;
`crates/btctax-adapters/src/tax_tables.rs`; `forms/2017/YEAR.toml`, `forms/2024/YEAR.toml`,
`forms/2025/YEAR.toml`; all 37 `forms/*/*.map.toml`; all 49 `design/forms/geometry/*.json`;
`design/TY2026_WORK_LIST.md`; `design/ROADMAP_STATUS.md` §2; the review report and its
`…VERIFICATION.md` ledger.

Ran:
- `cargo nextest run -p xtask -E 'test(label)' --no-capture` (13 pass; per-map counts captured once
  and grepped, per the never-run-a-suite-twice rule).
- `cargo run -p xtask -- label-census f1040s1--2026-DRAFT` / `… f1040v--2025`.
- `cargo run -p xtask -- form-delta` on four `2025-final → 2026-DRAFT` pairs.
- A Python re-implementation of `candidate_columns` / `column_tokens` / `resolve` / `witness_text` /
  `label_join` over **all 49** committed geometry fixtures and **all 37** maps, run under
  `.venv/bin/python`. **Calibration:** it reproduces the old rule's floors exactly (2024 = **99**,
  2025 = **82**, and the two wrong bindings the fold describes: `2024/f1040 line2b→2a`,
  `line3b→3a`) and the new rule's exactly (**235 / 193 / 0 wrong**, matching the test's own
  `eprintln!`). Every number below is from that harness or from the Rust tools, none hand-counted.

Not re-derived (taken as settled per the brief): that R2 was real; the two-witness design; the owner
rulings; the eight wired maps' `[census]` and map ⊆ PDF facts.

## Verdict: 0 Critical / 4 Important / 4 Minor

## Rule audit: every box where the new rule differs from the old

Across 3,473 boxes in 49 fixtures the in-row rule **fires on 1,425** and **changes the answer on 102**.
Only **2 of the 102 are bound by any committed map** (the `2b`/`3b` pair the fold explains) — so 100
of them are the joins the maps do not exercise, which is what this audit is about. Judged against
each form's own extract text:

| # | form / boxes | new pick | judged correct pick | verdict |
|---|---|---|---|---|
| 5 | `f1040--2024` `f1_43/f1_45/f1_47/f1_49/f1_51` | `2b 3b 4b 5b 6b` | same | **correct** — the fold's stated case; `2b` prints at x 488.5–497.9, box at 504 |
| 5 | `f1040--2025` `f1_59/f1_61/f1_63/f1_66/f1_69` | `2b 3b 4b 5b 6b` | same | **correct** |
| 15 | `f1040sc--2024` `Lines18-27[0].f1_28…f1_40`, `c1_7[0]/[1]` | `18 19 20a 20b 21 22 23 24a 24b 25 26 27a 27b 32a 32b` | same | **correct, and a repair** — Sch C Part II is two money columns; the old rule read the LEFT margin and returned `8 9 10 11 12 13 13 14 14 15 16 16a 17`. 13 boxes were silently mislabelled before this fold |
| 15 | `f1040sc--2025` (same set) | ditto | ditto | **correct, and a repair** |
| 16 | `f1040sc--2026-DRAFT` (same set + `16c`) | ditto | ditto | **correct** |
| 12 | `f1040s2--2026-DRAFT` `f1_04/05/07/08`, `f1_28/31/32/33`, `f2_03/12/13`, `f2_16` | `1b 1c 1e 1f 13d 13g 13h 13i 13o 17b 17c 19a` | same | **correct** — sub-lines the column rule flattened to `1`/`13`/`17`/`19` |
| 8 | `f1040sa--2026-DRAFT` `f1_07/11/15/16/18`, `f2_13/14/17` | `5a 5e 8a 8b 8c 17g 17h 17k` | same | **correct** |
| 2 | `f1040s3--2026-DRAFT` `f1_31/f1_34` | `13b 13e` | same | **correct** |
| 1 | `f6251--2026-DRAFT` `f1_5` | `2a` | same | **correct** |
| **3** | `f1040--2024` `Page2.c2_1/2/3` | **`1` `2` `3`** | **`16`** | **WRONG** — line 16 reads *"Check if any from Form(s): 1 ☐8814 2 ☐4972 3 ☐___"*. Those are OPTION numerals |
| **3** | `f1040--2025` `Page2.c2_9/10/11` | **`1` `2` `3`** | **`16`** | **WRONG** — same row |
| **8** | `f1040--2025` `Page1.c1_33…c1_40` | **`1` `2` `3`** | **`3c` `4c` `5c`** | **WRONG** — *"c Check if … 1 ☐Rollover 2 ☐QCD 3 ☐___"* on lines 3c/4c/5c |
| **3** | `f1040s2--2025` `Line4_ReadOrder.c1_3/4/5` | **`1` `2` `3`** | **`4`** | **WRONG** — line 4 SE tax, *"Check if any exemption from …: 1 ☐4361 2 ☐4029 3 ☐___"* |
| **3** | `f1040s2--2026-DRAFT` `c1_3/4/5` | **`1` `2` `3`** | **`4`** | **WRONG** — same row |
| **2** | `f1040--2024` / `f1040--2025` `Page1.f1_03` | **`20`** | **`?`** (a header field, no line) | **WRONG** — the `20` is the printed literal in *"ending ____, 20___"*; gap 1.2pt. An honest `?` became a confident label |
| **1** | `f4868--2025` `VoucherHeader.f1_3` | **`20`** | **`?`** | **WRONG** — same *"and ending ____, 20___"* literal; gap 1.7pt |

**79 correct (13 of them repairs the old rule got wrong), 23 wrong.** None of the 23 is bound today,
so the suite is green — but see L1 for four constructed bindings on which the check now returns PASS
where the old rule returned FAIL.

Two supporting measurements, both over all 49 fixtures:

- **No box has two distinct in-row candidates** (0 of 3,473), so the `max_by(x2)` tie-break is never
  actually deciding between two labels. The `-2.0 / +4.0` band is therefore not *currently* creating
  row ambiguity; it is creating the wrong-row admissions in the table above.
- **Every one of the 23 wrong picks except the three `20` header cases is a checkbox** — box height
  exactly 8.0pt against a label word 9.5–10.7pt tall. See L1.

## Findings

### L1 — IMPORTANT — the in-row band admits a label that does not fit in the box, and that produces a demonstrable false PASS

**Where:** `crates/xtask/src/label_reader.rs:1067` — `*ly + 4.0 <= *bottom`.

**What is wrong:** the condition is a *proxy* for "the label's vertical extent lies inside the box's
span" (the doc comment's own words, `:1060`), but the fixture records the word's real bottom edge as
`Word::y2` and the rule does not read it. `+4.0` is less than half a label's height. Measured over
4,216 label words in the 49 fixtures: height min 5.83, **p50 10.49**, p95 10.74, max 11.93 — so the
comment's stated basis, *"labels are ~9.6pt tall"*, is itself below the median, and `+4.0` admits a
word that overhangs the box by up to ~6.5pt.

That is exactly what happens on a checkbox. A checkbox box is **8.0pt** tall and the option numeral
beside it is **9.5–10.7pt**, so the numeral never fits — and the `+4.0` slack lets it in anyway,
displacing the row's real line number. All 20 checkbox mislabels in the audit table are this one
mechanism.

It is not only a wrong report. The check's contract is *"a filled value would print on the WRONG LINE
of a signed return"*, and `label_matches` accepts a key whose trailing letter is stripped, so a wrong
binding the old rule caught is now accepted. Constructed and measured (old → new verdict on the same
committed geometry):

| planted binding | old rule | new rule |
|---|---|---|
| `line1a = "…Page2[0].c2_1[0]"` on `2024/f1040` (page-1 wages → page-2 line-16 checkbox) | printed `16` → **FAIL** | printed `1` → **PASS** |
| `line1a = "…Page1[0].c1_33[0]"` on `2025/f1040` | printed `3c` → **FAIL** | printed `1` → **PASS** |
| `line1z = "…Line4_ReadOrder[0].c1_3[0]"` on `2025/f1040s2` | printed `4` → **FAIL** | printed `1` → **PASS** |
| `line20 = "…Page1[0].f1_03[0]"` on `2024/f1040` (the fiscal-year header field) | printed `?` → skipped | printed `20` → **PASS** |

The forward-facing half matters as much: the mislabelled rows are ones a map will plausibly bind.
Schedule 2 line 4's `4361`/`4029` boxes are the §1402(e)/(g) SE exemptions, and btctax fills
Schedule SE. The first real symptom will be a **false FAIL** — `label_matches("4_exempt_4361", "1")`
compares `4` against `1` — arriving at a moment when the tempting repair is to loosen
`label_matches`, i.e. the "widening an exemption is never the safe edit" trap.

**Minimal change:** use the observation instead of the proxy — `w.y2 <= *bottom` in place of
`*ly + 4.0 <= *bottom` (keep `*ly >= top - 2.0`). Measured over all 37 maps and 49 fixtures, this is
a drop-in:

- joins and wrongs **unchanged**: 2024 = 235 / 0, 2025 = 193 / 0 (and the old rule's 2 wrong stay
  wrong, so the fold's own repair is preserved);
- **20 of the 23** false labels disappear — every checkbox one;
- the 6 Schedule C `c1_7` picks fall back from `32a`/`32b` to `32`, which `label_matches` still
  accepts (trailing-letter trim), so nothing is lost;
- every one of the 79 correct picks — including all 13 Schedule C Part II repairs and all 23 draft
  sub-line repairs — survives.

The three `20` header cases survive that fix and are residue, not blockers: they are header text
above the first label row, and no binding names them. Worth a follow-up, not a change here.

### L2 — IMPORTANT — `per_map_zero` would not have fired on R2, the defect it was added for, and no test has ever seen it red

**Where:** `crates/xtask/src/label_reader.rs:1477` — `if !bindings.is_empty() && map_joined == 0`.

**What is wrong:** `bindings` is the output of the *fixed* parser. Under the parser R2 found, a map
whose every binding carried a trailing `# comment` produced an **empty** map, so `bindings.is_empty()`
was true and the new guard is silent by construction. Measured across all 37 maps, old parser vs new:

| | maps whose numbered bindings the OLD parser extracted as **zero** | numbered bindings those maps actually hold |
|---|---|---|
| TY2024 | `f1040s1`, `f1040s3`, `f1040sa`, `f1040sc`, `f8959`, `f8960`, `f8995` (7) | 89 |
| TY2025 | `f1040s1a`, `f1040s3`, `f1040sa`, `f1040sc`, `f8960`, `f8995` (6) | 108 |

**13 maps, 197 bindings — and `per_map_zero` sees none of them**, including `2025/f1040sa` (0 of 19),
the map the review named, and `2025/f1040s1a` (0 of 46). The check the fold added "because the
aggregate floor could not express it" is blind to the exact shape it was written for; only the
aggregate floor (99→235, 82→193) would have caught the regression, which is the thing the fold says
it improved on.

It also has no negative test. B1 asks one sentence with a factual answer — *which test reds when this
checker is removed?* — and for `per_map_zero` the answer is **none**. Contrast `audit_year_reach`,
which the same file deliberately factored into a pure function so its five plants could run without
mutating the tree; `per_map_zero` is inline in the `#[test]` body and cannot be planted the same way.
Per the harness's own note, *an honest kill-test for a blind checker cannot be written without
discovering the blindness* — that is precisely what did not happen here.

**Minimal change:** count the map's line-shaped KEYS separately from the bindings the parser
extracted, and red on `keys > 0 && extracted == 0` as well as on `extracted > 0 && joined == 0`;
lift the predicate into a pure `fn map_reach_problem(keys, extracted, joined) -> Option<String>` and
give it a plant asserting red on `(19, 0, 0)`.

### L3 — IMPORTANT — the fold changed the reader that generated `design/TY2026_WORK_LIST.md` and did not regenerate it; four rows are now wrong

**Where:** `design/TY2026_WORK_LIST.md` "lines that moved" column; the generator is
`xtask form-delta`, which calls `label_join_public` → the rule this fold changed.

**What is wrong:** verified by running the real tool at HEAD, not by inference:

| form | doc says | `form-delta` at HEAD |
|---|---|---|
| `f1040s2` | 27 | **23** |
| `f1040s3` | 3 | **2** |
| `f1040sa` | 5 | **2** |
| `f6251` | **1** | **0** — *"60 of 62 common field(s) COMPARED; none of them changed the printed line it sits beside"* |

The other ten rows are unchanged. The `f6251` cell is the load-bearing one: the document's only claim
that Form 6251 moves a line for TY2026 was an artifact of the old purely-vertical rule reading `2a`
on the 2025 fixture and `2` on the draft. That is the *same class* as the cover-sheet error the
document already carries a boxed correction for — a plausible wrong number from a reader defect — and
it is now sitting under that correction uncorrected. The document's own instruction, *"Regenerate when
a final lands"*, does not name the trigger that actually invalidated it: a change to the reader.

**Minimal change:** regenerate the table at HEAD and add "or when the label reader changes" to the
regeneration trigger. Better, and cheap: a test that reds when the committed table disagrees with
`form_delta` output, so the artifact cannot silently decay again.

### L4 — IMPORTANT — a second silent binding drop survives, on Schedule D, and the fold's new per-map line under-reports it

**Where:** `label_reader.rs:1120` — `rhs.strip_prefix('"')`; and the per-map `eprintln!` at `:1467`.

**What is wrong:** R2 was "a numbered binding the parser drops without saying so." A numbered binding
written as an **inline table** is dropped the same way, with no comment, no counter and no reason:

| map | numbered lines bound as inline tables | FQNs |
|---|---|---|
| `2024/schedule_d` | `line1a`, `line3`, `line8a`, `line10` | 14 |
| `2025/schedule_d` | `line3`, `line10` | 8 |
| `2017/f1040`, `2017/schedule_se`, `2017/schedule_d` | 13 lines | (year unreachable) |

`line_bindings`'s doc comment excuses only *"repeating grids [that] address rows positionally and
carry no single printed label"* — but `line1a`, `line3`, `line8a`, `line10` are single named rows with
several columns, each with a printed margin label. I resolved all 22 live FQNs against the geometry:
**every one MATCHES** (`1a`, `3`, `8a`, `10` as printed). So this is 22 free joins, on the form the
file itself calls *"where btctax's capital gain lands"*, that nothing checks.

The fold's own new accounting line makes the gap invisible rather than visible: it prints
`2024/schedule_d: 9 numbered bindings, 9 joined` for a map that binds **13** numbered lines. An
instrument that mis-states its own reach is the shape this whole review round exists to close.

**Minimal change:** in `line_bindings`, also accept an inline-table RHS by extracting every
`"…[…]…"` value in it and emitting one entry per (line, FQN); or, if that is out of scope, count
such lines and print them as `N line(s) bound by inline table — not joined` so the drop is stated.

### L5 — MINOR — "every TY2025 map contributes" is not true, and a map with zero numbered bindings carries no recorded reason

**Where:** `label_reader.rs:1339` — the TY2025 `YearFloor` comment.

**What is wrong:** two of TY2025's 15 maps — `f8949` and `f8283` — contribute **0** joins (verified in
the test's own output). They hold no `lineN` key at all, being grids. The comment's second clause,
*"the per-map zero-join check below is what would say otherwise"*, is false for exactly those two: the
`!bindings.is_empty()` guard makes them silent. The *load-bearing* claim is fine — the eight maps
wired at step 5 are `f1040s2, f1040s3, f1040sa, f1040sb, f1040sc, f8959, f8960, f8995`, contributing
6/5/19/4/7/17/15/16 joins respectively, all non-zero — so step 5's evidence stands; the comment
overclaims around it.

Answering the brief's question directly: **yes, the two cases must be distinguished.** `YEAR_FLOORS`
already enforces that a permissive floor records WHY (`audit_year_reach`, the `must record WHY`
branch). A map contributing zero joins has no such requirement, so "this map encodes no numbered
line" and "this map's bindings were all dropped" are the same blank — the *blank-is-the-normal-case*
invariant, one level down.

**Minimal change:** correct the comment to "thirteen of fifteen TY2025 maps contribute; `f8949` and
`f8283` are positional grids with no `lineN` key", and make that a recorded reason the check reads
(an allow-list of grid maps that must have zero, red if one of them grows a numbered binding or if any
other map has none) rather than a comment.

### L6 — MINOR — R7's "structural rule" is a three-stem hand-list that never consults the year

**Where:** `crates/btctax-forms/tests/field_census.rs:620` —
`let allowed = |stem: &str| stem == "f1040s1a" || stem == "f8275" || stem == "f8283";`

**What is wrong:** the comment above it states the rule as *"only a schedule that did not exist for
the year (Schedule 1-A, TY2025+) or a periodic form served from another year's revision may be
absent"* — both of which are facts the crate can answer (`bundled::periodic_template(stem, year)`
returns `Some` for the second; Schedule 1-A's first year is TY2025). The code instead names three
stems and ignores `year`, so `f1040s1a` is excusable on **any** filable year, including TY2025+ where
Schedule 1-A does exist. This is the hand-list shape `CLAUDE.md` names as having shipped here before.

Exposure today is small and I want to be accurate about that: only TY2024 is `filable`, its sole
measured absence is `f1040s1a`, and the `measured != recorded` assertion 20 lines above already reds
on any change to an absence set. The `f8275`/`f8283` entries are dead — neither is absent on any
filable year. So this is a latent widening, not a live hole.

**Minimal change:** `let allowed = |stem: &str| periodic_template(stem_of(stem), *year).is_some() ||
first_year_of(stem) > *year;`

### L7 — MINOR — R8's `Default` escape is still open while the new comment reads as if it were closed

**Where:** `crates/btctax-forms/src/packet.rs:52-57`.

**What is wrong:** the review flagged *"`FiledPacket` literal/`Default` constructible from any crate"*.
`#[non_exhaustive]` closes the literal half only: the struct still derives `Default` and both fields
are `pub`, so an external crate can write `let mut p = FiledPacket::default(); p.forms.push(…);` and
obtain an unsorted packet. The new comment — *"a literal `FiledPacket { .. }` is not constructible
outside this crate — go through `stapled`"* — is literally true and reads as a guarantee that it is
the only path. No in-repo caller does this, and btctax has no external users, so this is a wording and
completeness issue rather than a live defect.

**Minimal change:** drop `Default` from the derive (nothing in the workspace calls
`FiledPacket::default()`), or say in the comment that `Default` + field mutation remains reachable and
is held only by convention.

### L8 — MINOR — the 12pt gap is calibrated to this corpus, not derived, and its margin is 1.0pt

**Where:** `label_reader.rs:1070` — `left - *lx2 <= 12.0`.

**What is wrong:** measured over all 1,425 in-row firings: accepted gaps run 0.00 → **11.30**
(p50 6.13, p95 9.30); the nearest **rejected** candidate anywhere in the 49 fixtures is **12.30**
(`f8995--2024` `f1_19`/`f1_23`). The threshold sits inside a 1.0pt window with 0.70pt of headroom
above the largest value it must accept. Nothing in the code derives it; it is the smallest round
number that separates the observed sets. The neighbouring constants are better grounded — row pitch
p50 is exactly **12.00**, which the `-2.0` slack respects — but the `~9.6pt` glyph height the comment
cites is not the measured one (p50 10.49, max 11.93), so the band's stated derivation does not hold
either (see L1).

On the TY2026 drafts the constant does hold with room: draft in-row gaps run 4.0–8.1, including
`f1040sa--2026-DRAFT` and the rebuilt `f1040s1a--2026-DRAFT` (185 boxes, 69 firings, zero differences
from the column rule). So this is not a live failure — it is an undeclared cliff that the next IRS
layout may cross silently, because falling off it degrades to the column rule rather than erroring.

**Minimal change:** state the measured separation (11.30 accepted / 12.30 rejected) in the comment so
a future reader knows the margin, and replace the point threshold with the scale-free property it
approximates — *the label is the last word ending left of the box on the box's row, with no other word
between it and the box*. I implemented that variant and measured it: 235/193 joins, 0 wrong, identical
to the current rule on every one of the 3,473 boxes.

**Related, and worth a follow-up rather than a change:** two committed fixtures make `label_join`
return `Err` outright — `f1040s1--2026-DRAFT` (*"bare sub-letter `a` at page 1 y=120.859 has no
numeric parent"*) and `f1040v--2025` (*"no numbered label column found"*). Both are inert now (neither
has a map), but the first is a TY2026 draft of a form the packet emits, so when TY2026 maps land it
will arrive as an unwitnessed map needing a `why`.

## R1–R12 closure check

| finding | site | faithful? |
|---|---|---|
| R1 | `forms/2025/YEAR.toml` `[oracles] ots` | **yes** — `~/OpenTaxSolver2025_23.06_linux64/bin/` exists (checked); `ots_direct.py:79` reads `OTS_YEAR` and `:73` `OTS_DIR`; the record now says external and un-archived |
| R2 | `label_reader.rs` parser + join + floors | **partly** — the parser fix is right and measured (99→235, 82→193, reproduced independently); the x-aware rule repairs 13 previously-mislabelled Schedule C boxes as well as the 2b/3b pair. But see **L1** (23 new wrong labels, 4 constructed false passes), **L2** (`per_map_zero` blind to R2's own shape), **L3** (derived doc not regenerated), **L4** (a second silent drop) |
| R3/R4 | closed earlier by `c76adf6b` | out of scope, per the ledger |
| R5 | `forms/2025/YEAR.toml` `[forms_absent] f8275` | **yes** — `bundled::periodic_template` exists at `bundled.rs:151` and the reason now names it |
| R6 | `tests/year_record.rs:40` | **yes** — pins 5/17/15; the maps' actual `forms_expected` lengths are 5, 17, 15 (measured); the `other => panic!` arm makes a new year loud |
| R7 | `tests/field_census.rs:620` | **weakly** — see **L6**: the comment states a structural rule, the code is a three-stem hand-list |
| R8 | `src/packet.rs` | **partly** — see **L7**: literal closed, `Default` + `pub` fields open |
| R9 | `forms/2017/YEAR.toml` `taxcalc` | **yes** — "none wired … no slice harness path" |
| R10 | `forms/2025/YEAR.toml` `tables` | **yes** — matches `tax_tables.rs:548` (*"OBBBA … confirmed to leave 2025 brackets/breakpoints"*) |
| R11 | `year_readiness.rs:88` + test | **yes** — the guard reds; the test sets `r.table = false` and asserts the message, a real kill |
| R12 | `tax_tables.rs:60` | **yes** — `by_year.insert(2017, ty2017())` at `:76`, so the doc now matches the map |

**On the floors (asked directly):** 235 and 193 are **measured, not estimated** — I reproduced both
independently and the test prints them. The floors are the right instrument and the ratchet comment is
honest. "Every TY2025 map contributes" is the one overclaim (L5).

**On the kills (asked directly):**
- `the_join_check_reds_on_a_wrong_year_map_that_an_existence_check_would_pass` **still discriminates,
  and for the same reason as before**: `f6251--2025` is one of the fixtures where the in-row rule and
  the column rule agree on every box (60 firings, 0 differences), so `count_wrong(m2024)` is **12
  under both rules** and `count_wrong(m2025)` is **0 under both**. It has not gone blind — but it does
  **not exercise the new rule at all**.
- Removing the in-row rule *does* red `every_mapped_line_lands_on_its_own_printed_label` (2024 goes to
  2 wrong: `line2b→2a`, `line3b→3a`), so the rule is not unheld. What is missing is the plant the
  commit message and `design/ROADMAP_STATUS.md:179` both assert as its kill — *"A planted 2a/2b swap
  reds both boxes"* — which is a manual one-off and is **not a committed test**, in a roadmap section
  titled "the year-package table's instruments and their kills" where every neighbouring entry names
  one. B1 asks for the plant to land paired with the checker.
- `per_map_zero` has **no kill anywhere** and is structurally blind to R2 (L2).

## The instrument may be trusted for step 5's wiring? YES

The eight wired TY2025 maps rest on 89 joins across eight maps, every one correct under the current
rule, under the old rule, and under both alternative rules I measured — none of the 23 defective picks
is bound, and none is on a wired map. The wiring decision does not depend on any of the findings above.
But L1 must be fixed before the check is trusted to *gate* a new map: on a checkbox row it now returns
PASS where it used to return FAIL, and the next Schedule 2 / Form 1040 checkbox binding is the one that
will meet it.
