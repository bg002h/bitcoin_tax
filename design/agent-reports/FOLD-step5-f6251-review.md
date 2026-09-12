# FOLD — the step-5 seam review (`f6251/2025`, the OBBBA-era Form 6251)

**Report folded:** `design/agent-reports/2026-09-11-review-step5-f6251-obbba.md` (`fbe9d788`, 0C/2I/3M/1N).
**Ledger:** `…-VERIFICATION.md` (`c91eb5e5`). **Brief:** `BRIEF-fold-step5-f6251-review.md` (`c846ab52`).
**Build under repair:** `d8d023af`. One opus implementer, shared main tree, **nothing committed, no
subagents, no `git stash`/`checkout`/revert** — every mutation reverted from a `cp` backup and the file
touched afterwards.

**Gate, at the folded state:** `make gate` **3594 passed / 12 skipped, exit 0** (baseline 3589/12 — the
delta is the five new tests below), clippy `-D warnings` clean, `cargo fmt --all --check` clean.
`cargo run -p xtask -- line-coverage`: **OK, 377 money lines across 18 forms, f6251:43** (baseline 375,
**f6251:41**), ratchets unmoved — 31 exceptions (31), 0 unverifiable (0), 17 not line-bound (17).
**Diff:** `git diff --stat` → **11 files, +817 / −79**.

All six brief items are folded. Every new or changed check was watched RED on a planted defect; the
outputs are pasted verbatim in §2.

---

## 1. What changed, per finding

### ★★★ (1) The two false source comments — corrected, and one of them was the fix

Both premises confirmed exactly as the brief states, then corrected:

| where | was | now |
|---|---|---|
| `crates/btctax-forms/src/f6251_revision.rs:23-43` | *"line 43 is its TY2026 **renumber**"* | a table read off both extracts' label columns, headed **"37 → 43 IS A COLLISION, NOT A RENUMBER — and this paragraph used to say 'renumber'"**, with the direction and size of the error (`1a` negative ⇒ AMT base overstated by ≈MAGI, taxpayer-adverse, opposite to the TY2025 understatement the core-side kills pin) |
| `crates/btctax-core/src/tax/return_1040.rs:2971-2993` | *"TY2026 is expected to **REUSE the `Y2025` shape** … only the cited Schedule 1-A line moves, 37 → 43 … therefore **a one-line edit**"* | **"TY2026 IS NOT A ONE-LINE EDIT, and this comment used to say it was"**, quoting what it said, citing `f1040s1a--2025.txt:108` / `--2026-DRAFT.txt:213,222`, and naming what makes the arm safe: *transcribe TY2026's schedule, call THAT struct's accessor, and let the refusal be the proof — never re-type a line number here* |

A third, smaller one went with them: `Form6251Line1::Y2025`'s doc comment (core `form6251.rs:53-72`,
the builder's own follow-up F3) no longer asserts "line **37**" as a property of the variant — the
variant now *carries* the line it read.

★ Both corrections are now held by instruments rather than by care: the `f6251_revision` sentence is
asserted verbatim against its own extract (pre-existing), and the core-side claim is asserted by the
new `line-coverage` rows (item 3) plus the emitter join (item 2).

### ★★ (2) I-1 — the two sides joined, structurally

**The design decision, and why.** The brief offered the choice; I took the ★ option, because
`CLAUDE.md`'s transcription **scope note** settles it: `Form6251Line1Rule` is not a transcription of a
printed line-set — it is *"everything Form 6251 reads off the rest of the return"* — so its field is a
**cross-year quantity** and must be named for what it is, with the cross-reference carried as **data**.
`schedule_1a_l37` was the compression the rule forbids, in its worst form: a cell spelled into an
identifier, which is the one shape no test, no `_`-free match and no extract comparison can read.

Four edits, and the mechanism is the third and fourth:

1. **`crates/btctax-core/src/tax/schedule_1a.rs:1367-1394`** — `Schedule1A::SENIOR_DEDUCTION_SUBTOTAL_LINE`
   (`= 37`, *a property of the revision, not of the quantity*) and
   `Schedule1A::senior_deduction_subtotal(Option<&Self>) -> SeniorDeductionSubtotal`, a sibling of the
   existing `Schedule1A::line_13b`.
2. **`SeniorDeductionSubtotal`** — `{ amount: Usd, schedule_1a_line: u32 }` with **private fields and no
   public constructor**, so the only way to obtain one is from the transcription struct that printed the
   figure (`schedule_1a.rs:1396-1437`). **Verified by the compiler, not asserted** — §2.5: `E0451`
   on a forge attempt from `return_1040.rs`.
3. **`crates/btctax-core/src/tax/form6251.rs:408-422, 484-502`** — `Form6251Line1Rule::Y2025` now takes
   `senior_deduction: SeniorDeductionSubtotal`; `compute_6251` subtracts `.amount()` and carries
   `.schedule_1a_line()` forward into `Form6251Line1::Y2025 { line1a, line1b, schedule_1a_line }`
   (documented as **provenance, never a printed box**; `printed()` passes it through unrounded).
4. **`crates/btctax-forms/src/f6251_revision.rs:150-196`** — `ObbbaRevision::schedule_1a_line_agreeing_with(u32)`,
   the join: the line **this revision prints** (parsed out of the form's own sentence) against the line
   **the figure was read off**, returning the agreed number or a refusal that names both and says why
   they are not interchangeable. **`crates/btctax-forms/src/form6251.rs:313-321`** replaces the bare
   `revision.schedule_1a_line().ok_or_else(…)?;` — an existence check whose value was dropped on the
   floor — with `revision.schedule_1a_line_agreeing_with(schedule_1a_line)?;`. Both legs fail closed:
   an unparseable sentence still refuses.

No params were bundled, no `2026` arm added, no fail-closed gate touched.

### ★★ (3) M-1 — `cover_form6251line1` now emits the OBBBA rows, and a `match` holds it

- **`crates/btctax-core/src/tax/line_coverage.rs:680-746`** — the `if let Form6251Line1::Y2024 {…} = p`
  became an **`_`-free `match`** (exhaustiveness applies inside the defining crate despite
  `#[non_exhaustive]`, so a third variant is `E0004` rather than another silent skip), with a
  `quoting_year("2025")` arm carrying rows for **1a** and **1b** quoted verbatim from
  `f6251--2025.txt`.
- ★ **The 1a quote is selected BY the provenance, not typed beside it**: `match schedule_1a_line { 37 =>
  <the TY2025 sentence>, _ => <a sentence verbatim on no form> }`, so a chain whose cited line this table
  has not transcribed produces a row that **cannot resolve** and `line-coverage` reds. A fixed literal
  would have re-asserted TY2025's sentence over another revision's figures — the collision again, one
  layer up.
- **`line_coverage.rs:4073-4088`** — registered in `all()` with its **own instance**, because
  `Form6251::default()` is the TY2024 shape: without this the rows compile and reach no checker. The
  instance's line number is `Schedule1A::SENIOR_DEDUCTION_SUBTOTAL_LINE`, never a literal.
- **`crates/xtask/src/line_coverage_check.rs:1514-1597`** — the B1 kill, in three directions (the variant
  yields rows at all; the committed rows are clean through the real `check()`; the TY2026 cross-reference
  planted into 1a reds naming the row and the extract).

Measured effect: **f6251 goes 41 → 43 rows**, and `Form6251Line1::Y2025`'s transcription is verified by
something for the first time.

### (4) I-2 — the Part III sweep ported, counter intact

**`crates/btctax-forms/tests/f6251_obbba.rs:954-1094`** — `part_iii_routed_2025()` (routed fixture, cents
on Part I's inner column, the parenthesised box and Part III, distinct per-line values) plus
`part_iii_routed_writes_every_money_cell_as_a_whole_dollar_and_the_sweep_sees_all_of_them`: iterates
`map.money_cells()`, asserts whole dollars over the **serialized PDF**, and asserts
**`checked == 42`** — ported from `f6251_fill.rs`'s TY2024 twin (`checked == 41`) rather than reinvented,
per B3. It also pins `map.money_cells().len() == 42` separately, so the count's *meaning* is
unambiguous (42 cells × 1 widget, not 41 cells one of which carries two), and asserts the **line-33/36
pair** on the page — `0` vs `163750` in different boxes — which no test had ever reached, because
reaching them requires the routed path.

Measured: **42**, not assumed. 29 of the 42 cells and `Form6251ObbbaMap::money_cells()` now have a
reader.

### (5) M-2 and M-3

- **M-2** (`f6251_revision.rs:72-80`): the *"those figures are already per-year in `AmtParams`"*
  justification is gone. Replaced with the single-authority reason (which is still the right one) plus
  the measurement that refutes the old sentence: **every `AmtParams::mfs_kicker_start` literal in the
  workspace is `dec!(875950)` (TY2024) or `dec!(640200)` (TY2026) — none is TY2025's printed
  $900,350** (`tax_tables.rs:185,315`; the `:1053` assertion is the derived 640200). The gap it was
  hiding is filed as **FR-120**, not papered over.
- **M-3** (`map.rs:629-647`): `Form6251ObbbaMap::line4`'s doc comment no longer quotes the TY2025
  sentence or `$900,350`. Of the brief's two options I took *"stop quoting a year-specific figure in a
  shared struct"* rather than *"make that copy checked"*, because the checked copy already exists
  (`f6251_revision::line4`, asserted verbatim per revision) and a second checked copy would be the
  two-authorities defect the same module warns about. The comment now points at it and records what was
  wrong. ★ **This fix has no kill and cannot have one** — it deletes an unchecked copy rather than adding
  a checker; the honest statement is that the class is closed by removal, and the surviving copy's
  checker is `the_year_varying_cells_are_verbatim_in_that_revisions_own_extract` (pre-existing, already
  observed red on a planted "line 43").

### (6) N-1 — fixed inline

**`f6251_revision.rs:225-238`** — all three accessors now go through `after_the_only(sentence, anchor)`,
which returns the tail **only if the anchor occurs exactly once**. `split_once` silently took the first
occurrence, so a revision whose clause repeated its anchor had a second reading available and nothing
said which was taken; only the zero-occurrence case failed closed. Verified the real revision's three
anchors each occur once. Fixed inline rather than filed because it is three lines and the ambiguous
case was previously *inexpressible* — which is what made it invisible.

---

## 2. Kills — red, then green

### 2.1 ★★★ The central kill, in three layers

**Layer A — the literally prescribed edit no longer compiles.** Planted the `2026 =>` arm exactly as the
old comment told a porter to write it (`schedule_1a_l37: schedule_1a.and_then(|s| s.part5.line37).unwrap_or(Usd::ZERO)`):

```
error[E0559]: variant `Form6251Line1Rule::Y2025` has no field named `schedule_1a_l37`
    --> crates/btctax-core/src/tax/return_1040.rs:3017:13
     |
3017 |             schedule_1a_l37: schedule_1a
     |             ^^^^^^^^^^^^^^^ `Form6251Line1Rule::Y2025` does not have this field
     |
     = note: available fields are: `senior_deduction`
```

**Layer B — the edit a porter writes NEXT does compile, and I measured what it carries**, because a
compile error the author routes around is not a gate. Re-planted the arm in its post-rename form
(`senior_deduction: Schedule1A::senior_deduction_subtotal(schedule_1a)`) with a probe:

```
FAIL btctax-core tax::return_1040::tests::plant_probe_what_the_2026_arm_carries
  PLANT PROBE: the 2026 arm compiles and carries Schedule 1-A line 37

FAIL btctax-core tax::return_1040::tests::form6251_part_i_refuses_every_year_it_has_not_transcribed
  assertion `left == right` failed: exactly the years whose Form 6251 Part I has been read from the form.
    left: [2024, 2025, 2026]
   right: [2024, 2025]
```

So **core alone does not stop it** — the only core-side red is the year list, which a porter discharges
by editing a vector, and it asserts nothing about *which* Schedule 1-A line the new arm reads. That is
the review's I-1 claim, measured rather than inferred, and it is why the stop has to be the emitter.

**Layer C — the emitter stops it, and the stop can fail.** Two mutations, restored from `cp` backups:

*C1 — the pre-fold emitter line put back* (`revision.schedule_1a_line().ok_or_else(…)?;`, value dropped):

```
FAIL btctax-forms::f6251_obbba the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite
  f6251/2025: this form cites Schedule 1-A line 37 and the figure was read off line 43 — the fill
  SUCCEEDED, so the AMT base is taken from the wrong line of another revision's schedule and nothing on
  the printed page shows it
```

*C2 — the comparison itself neutralised* (`if false && printed != read_by_the_computation`):

```
FAIL btctax-forms f6251_revision::tests::a_revision_citing_line_43_refuses_a_figure_read_off_line_37
  a revision printing "Schedule 1-A line 43" ACCEPTED a figure read off line 37 — that is modified AGI
  in the senior deduction's slot and the AMT base is overstated by roughly the whole AGI: 43
FAIL btctax-forms::f6251_obbba the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite
```

Green after restore. ★ The unit test's plant is a revision whose `line1a` is the **TY2026 draft's own
sentence, verbatim from `design/forms/extract/f6251--2026-DRAFT.txt:51-52`**, so it is the arm a porter
would really write, not a synthetic string. The integration test derives the planted line number from
the revision's *own* printed one (37 ⇄ 43), so it keeps meaning the same thing when a second revision
joins `obbba_revisions()`.

### 2.2 M-1 — the coverage rows

*Mutation 1 — the pre-fold `if let` restored* (the Y2025 arm emits nothing):

```
FAIL xtask line_coverage_check::tests::the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds
  assertion `left == right` failed: the OBBBA line-1 region prints TWO boxes (1a and 1b); an empty
  Coverage here is the `if let` that matched nothing
    left: 0
   right: 2
```

*Mutation 2 — the rows kept, the `all()` registration removed* (the shape that shipped `cover_form8995apartiii` missing):

```
FAIL xtask line_coverage_check::tests::the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds
  assertion `left == right` failed: both OBBBA rows must be registered in `all()` — `Form6251::default()`
  is the TY2024 shape, so they reach the checker only through their own instance
    left: 0
   right: 2
```

*Direction 3 — the rot, in the committed test itself* (1a's quote replaced by the TY2026 draft's
"line 43" sentence) reds through the real `check()` with `f6251:1a … NOT FOUND in f6251--2025.txt`;
this one runs green-side every gate because the plant is inside the test.

### 2.3 I-2 — the Part III sweep

*Mutation — line 36 dropped from the OBBBA `p3` fill list, as a rebase would drop it* (array `29 → 28`):

```
FAIL btctax-forms::f6251_obbba part_iii_routed_writes_every_money_cell_as_a_whole_dollar_and_the_sweep_sees_all_of_them
  assertion `left == right` failed: f6251/2025: all forty-two modelled lines must be read back; a count
  that drifts means the sweep stopped seeing cells and started passing vacuously
    left: 41
   right: 42
 Summary 15 tests run: 14 passed, 1 failed
```

★ **14 of 15 tests in that file passed under the plant** — the partition, the AcroForm existence check,
`verify_flat`, the label census, the verbatim checks, all green with a numbered box blank on a filed
form. That is I-2's "concrete failure scenario" reproduced, and the counter is the only thing that saw
it. `checked` is the expected **42** (measured, and pinned alongside `money_cells().len() == 42`).

### 2.4 N-1 — the ambiguous anchor

*Mutation — `after_the_only` reverted to `split_once`:*

```
PASS btctax-forms f6251_revision::tests::a_sentence_without_the_cross_reference_yields_none_rather_than_a_default
PASS btctax-forms f6251_revision::tests::the_cross_references_and_the_threshold_parse_out_of_the_form_s_own_sentences
FAIL btctax-forms f6251_revision::tests::a_repeated_anchor_is_ambiguous_and_yields_none_rather_than_the_first_reading
  assertion `left == right` failed
    left: Some(43)
   right: None
```

The pre-fold reading silently answered **43** on a sentence that also says 37, and both pre-existing
accessor tests were green while it did.

### 2.5 The structural claim behind item (2), checked by the compiler

Planted a forge in `return_1040.rs` — TY2025's figure with a line number of my own:

```
error[E0451]: fields `amount` and `schedule_1a_line` of struct `SeniorDeductionSubtotal` are private
     |                 ^^^^^^ private field
     |                 ^^^^^^^^^^^^^^^^ private field
```

So the provenance cannot be separated from the figure outside the `schedule_1a` module. **The honest
boundary:** code *inside* that module can construct one — which is exactly where a TY2026 schedule's own
struct and its own `SENIOR_DEDUCTION_SUBTOTAL_LINE = 43` will live. Every reader (`return_1040`,
`form6251`, `line_coverage`) is a sibling module and cannot.

---

## 3. Premises checked, and what I found that the review did not

**No brief premise was refuted.** Both quoted comments read exactly as stated, at the stated lines; the
collision, the discarded `ok_or_else`, the `if let`, the `money_cells()` reader count (0), and the false
`AmtParams` sentence all reproduced. Four things are worth recording anyway:

1. **★★ `xtask line-coverage` is run by no test and by no CI job.** `make gate` is `nextest` + `clippy`;
   `line_coverage_check::run()` — the only caller of `check(&line_coverage::all())` — is reachable only
   from `cargo run -p xtask -- line-coverage`, and CI runs `check-isolation`, `examples` and
   `subcommand-coverage` only. **So fixing M-1 by adding rows would have produced another instrument
   nobody runs** — the review's own defect class, one level up. That is why the M-1 kill lives in
   xtask's `mod tests` (which `make gate` *does* run) and asserts on the real `all()` rows, not only on
   a synthetic table. The residual gap — the whole-table run — is **FR-122**.
2. **Core alone cannot stop the TY2026 arm** (§2.1 Layer B, measured: it carries line 37 and compiles).
   The review inferred this; it is now a pasted number, and it is the argument for putting the join in
   the emitter rather than in core.
3. **`line_coverage_check`'s "is this type printed?" predicate reads string literals.** Naming
   `SeniorDeductionSubtotal` inside a *refusal message* (a `format!`, not a comment) made
   `line-coverage` demand a `cover_seniordeductionsubtotal()` for a type that prints nothing — and the
   only "fix" it would accept is a cover fn that double-covers Schedule 1-A line 37 (rule 5). It is a
   **false RED**, so nothing was ever passed unchecked. I reworded the message; filed as **FR-121**,
   including the cost of the workaround (guidance removed from a refusal a porter will read).
4. **`design/TY2026_PORT_REPORT.md` is NOT the origin of the false framing** — checked, because the old
   comment cited it. Its part table (`:288-295`) states *"V Seniors | 31–37 | **37–43**"* and
   *"IV Car loan | 22–30 | 28–36"*, which already shows 37 on both sides. The compression into
   "a renumber" happened in the two source comments only, so nothing else needed correcting.

★ **One defect the fold introduced and the fold caught.** My first M-2 rewrite quoted a grep *count*
(*"returns exactly one hit"*) — and the sentence itself then contained the pattern, so the claim was
false the moment it was written: the same self-invalidating-comment class I was folding. Re-measured and
replaced it with a claim that does not move (`mfs_kicker_start`'s actual literals). Recorded because it
argues for the standing rule: quote a measurement whose subject is not your own prose.

---

## 4. Residue filed

| item | severity | owning phase |
|---|---|---|
| **FR-120** — `mfs_threshold_printed()` has no reader, and TY2025's printed $900,350 has no first authority in any `AmtParams` | Minor | **the TY2026 port**, before a year on this schema becomes filable |
| **FR-121** — `line_coverage_check`'s printed-type predicate counts string literals (false red) | Nit | ownerless residue — batch |
| **FR-122** — `xtask line-coverage` is in neither `make gate` nor CI; the 377-row table's verbatim run is unheld | Minor | **before the TY2026 port's first form lands** |

Nothing else was widened. Not touched: any `FullReturnParams`, the `2026` arm, the TY2025/TY2026
fail-closed gates, `f1040s1a/2025` staying `Unwired`, the Schedule 1-A emitter, the rest of the branch.

## 5. Files changed (11, +817 / −79)

| file | what |
|---|---|
| `crates/btctax-core/src/tax/schedule_1a.rs` | `SENIOR_DEDUCTION_SUBTOTAL_LINE`, `senior_deduction_subtotal()`, `SeniorDeductionSubtotal` (private fields) |
| `crates/btctax-core/src/tax/form6251.rs` | `Y2025` rule takes the subtotal; `Form6251Line1::Y2025` carries `schedule_1a_line`; `printed()`; the TY2025 Part I test |
| `crates/btctax-core/src/tax/return_1040.rs` | **the false porting instruction corrected**; the rule built through the accessor; two tests |
| `crates/btctax-core/src/tax/line_coverage.rs` | `cover_form6251line1` `_`-free match + OBBBA rows; `all()` registration |
| `crates/btctax-forms/src/f6251_revision.rs` | **the false "renumber" corrected**; **M-2**; the join method; `after_the_only`; two kill tests |
| `crates/btctax-forms/src/form6251.rs` | the emitter calls the join instead of discarding the number |
| `crates/btctax-forms/src/map.rs` | **M-3** — the unchecked `$900,350` copy removed |
| `crates/btctax-forms/tests/f6251_obbba.rs` | routed Part III fixture + `money_cells()` sweep (`checked == 42`); the emitter-level join kill |
| `crates/btctax-forms/tests/f6251_fill.rs` | the new field at the TY2025-shape refusal fixture |
| `crates/xtask/src/line_coverage_check.rs` | the M-1 kill, three directions |
| `FOLLOWUPS.md` | FR-120, FR-121, FR-122 |

**Five tests added:** `a_revision_citing_line_43_refuses_a_figure_read_off_line_37`,
`a_repeated_anchor_is_ambiguous_and_yields_none_rather_than_the_first_reading`,
`the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite`,
`part_iii_routed_writes_every_money_cell_as_a_whole_dollar_and_the_sweep_sees_all_of_them`,
`the_obbba_line_1_rows_are_verbatim_and_a_moved_cross_reference_reds`.
