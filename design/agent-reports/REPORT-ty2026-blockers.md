# REPORT — stage 1: the TY2026 blocker list, made DERIVABLE

**Agent:** stage-1 implementer. **Brief:** `design/agent-reports/BRIEF-ty2026-blockers.md` (`df1a88b53`), read in full.
**Deliverable:** `xtask blockers [<year>]` — `crates/xtask/src/blockers.rs` (new, ~1,100 lines incl. 12 tests)
plus a dispatch arm and a `SUBCOMMANDS` row in `crates/xtask/src/main.rs`. **Nothing committed, nothing pushed.**
**Gate:** `cargo nextest run --workspace --no-fail-fast` **3754 passed / 0 failed / 12 skipped**;
`cargo clippy --workspace --all-targets --all-features -- -D warnings` **exit 0**; `cargo fmt --all --check` **exit 0**.
Working tree at exit: `M crates/xtask/src/main.rs`, `?? crates/xtask/src/blockers.rs` — every plant restored.

---

## 0. The answer, in one command

```
$ cargo run -p xtask -- blockers          # year defaults to default_year() -> 2026, derived from the glob
# TY2026 blockers — DERIVED at HEAD, 58 rows (15 UNMEASURED)
## clears itself in January — waiting only on an IRS document — 41 row(s)
## we must build — 13 row(s)
## we must decide — 3 row(s)
## not ours — 1 row(s)
## what this measured, and what it did not
```

No year appears in the command name and no blocker is typed anywhere. Asked for a year the work list was
not generated for, it **refuses instead of answering wrongly**:

```
$ cargo run -p xtask -- blockers 2025
xtask blockers 2025: the committed work list is generated for `2026-DRAFT`, not TY2025: the form axis
would compare the wrong revision. Regenerate the work list for TY2025 first.
```

## 1. Three premises tested. TWO SURVIVED; ONE WAS REFUTED BY A PLANT.

The brief asked me to stop and report if I could disprove a premise.

| premise | verdict |
|---|---|
| `YearReadiness` is the right base | **HOLDS.** It is consumed, not re-derived — `declared`/`table`/`params`/`forms_bundled`/`prices_max_date`, and `problems()`'s sentences are pushed **verbatim**. Plant C (below) reds through it. ★ I did **not** add a method to it: the axis lives in xtask and calls the shipped type, so the blast radius is one dev-only crate. That is "build from there" without editing a published crate to serve a dev instrument. |
| the refusal buckets separate cleanly | **HOLDS, and the separation is sharper than the brief expected** — see §3. |
| the 1040 is genuinely unarchived | **HOLDS.** `design/forms/extract/` holds `f1040--{2020,2021,2022,2023,2024,2025}` and no 2026 anything; `i1040gi--{2024,2025}` and no 2026. `f1040`'s only TY2026 artefact was the file archived as `f1040--2026-DRAFT.pdf`, which `forms/2026/YEAR.toml` itself records as *"the TY2025 form"*. |

### ★★ REFUTED: *"the blockers that clear themselves"* — archiving the document does NOT clear them

The brief says the `state_local_refund` shape is *"the blocker that will clear itself."* My first draft
printed exactly that: *"archiving `i1040gi--2026` clears this without a code change."* **Plant B falsified it
in one command.** I copied `i1040gi--2025.txt` to `i1040gi--2026.txt` and re-ran: the archive-gap row
cleared and the §111(a) gate **stayed shut**. `slr::revision_for` reads `REVISIONS`, a *transcription*;
the extract directory only decides what the **suite** demands
(`every_archived_revision_is_transcribed`). So archiving reds the suite and makes a transcription **due**.

That is the same false-completeness this instrument exists against, committed inside the instrument, and
it is the finding I would not have had without the plant. Three things changed as a result:

1. the gate is renamed *"…is **TRANSCRIBED**"* and its refusal says so, with the distinction spelled out;
2. the per-revision row now says archiving *"REDS the suite and makes the transcription due — it does not
   silently clear, and it does not silently pass"*;
3. a **new derived row** appeared: *"`<fam>--<y>` is ARCHIVED but not TRANSCRIBED"*, which is the work item
   January actually creates and which was invisible to both the archive axis (the document is there) and
   the gate axis (which only asks about the target year).

Re-running plant B on the fixed instrument now moves the row **from `clears itself in January` to `we must
build`** — which is what really happens in January.

## 2. What each axis derives, and from what

| axis | derived from | never typed |
|---|---|---|
| year package | `YearReadiness::bundled(year)`, `problems()`, `price_coverage_or_refuse` | the 5 year facts + the refusal sentences |
| form-level state | `form_delta::port_status(prior,new)` — **the work list's own generator** — parsed **by column name** over 12 required columns | all 14 cells per stem |
| fillable-form set | `Stem::ALL` (21), minus `map_text(stem,year)`, minus `periodic_template(stem,year)` | which forms a return needs |
| the reason + the ETA | `forms/<year>/YEAR.toml [forms_absent]`, verbatim | "January 2027 package" |
| archive gaps | `design/forms/extract/` × every family a `.rs` file cites by path (361 files scanned) | which documents exist |
| revision shape | the same citations, split into per-revision **prefixes** vs pinned **literals** | which modules notice a new revision |
| year-keyed gates | 7 live `fn(i32)` probes, evaluated at **every** bundled year | which gate is year-specific |
| refusal attribution | the `RefuseReason` sites in `return_refuse.rs` that **read the year**, each anchored | which refusal a gate raises |
| owner decisions | `design/ROADMAP_STATUS.md` §0a's own table; a row saying `RULED` is not pending | S1/S2/S7, and the appraisal |

★ The form-axis tags come out of the work list's **own regeneration command** (`port-status 2025 2026-DRAFT`),
so document and tool cannot disagree.

## 3. The refusal question — separated, and the answer is much smaller than "~128"

**The brief's framing implies several year-specific refusals. There is essentially one.** The derivation:
a refusal that never reads the tax year cannot fire *because* the year is 2026, so the honest denominator
is not the 133 variants (`census_join::variant_paths` counts **133**, not ~128) but the **sites that read
the year**. `return_refuse.rs`'s non-test source has **11** such sites across **5** variants, all attributed:

```
Pub525ItemizedDeductionRecovery @ 4023/4024;  ReturnInputsYearNotStated @ 3444;
Schedule1aNotOnThisYearsReturn @ 3492/3494/3498/3511;
StateAndLocalRefundWorksheetNotComputed @ 3971/3989/4006;  StateLocalRefundFactsWithoutARefund @ 3962
```

0 UNCLASSIFIED. Bucketed by *probing the gate at every bundled year*:

* **fires because the year is 2026** — `StateAndLocalRefundWorksheetNotComputed`, via
  `slr::revision_for(2026) == None` (open for 2024 and 2025). **This is the only `RefuseReason` variant in
  the list.** The anchor `Err(NotUsable::NoArchivedRevision { tax_year }) =>` welds the gate to the variant,
  and a moved anchor is an error, not a silent un-attribution.
* **fires on any year** — `ReturnInputsYearNotStated` (`tax_year == 0`).
* **does NOT fire for 2026** — `Schedule1aNotOnThisYearsReturn`: 2026 is inside `SCHEDULE_1A_YEARS`
  (`2025..=2028`), so the gate is **open**. Had this been hand-listed it would very likely have been listed.
* **128 variants never read the year** and are reported as a count, never as a clean list.

The year-specific *walls* are therefore mostly **not** `RefuseReason` at all — they are 4 year-keyed gates:
`full_return_for(2026)` (the compute gate; open only for 2024), `slice_can_print(2026)`
(`Form8949Map`/`ScheduleDMap` refuse — `export-irs-pdf` writes no byte), `price_coverage_or_refuse(2026)`
(dataset ends **2026-06-03**, `prices_through` is 2026-12-31), and the §111(a) transcription. 3 gates are open.
Separating the buckets was the right instruction: conflating them would have produced a list of ~130 and
hidden the one.

## 4. Findings the instrument produced that the brief did not ask for

1. **★★ `crates/btctax-core/src/tax/capital_loss_carryover.rs:169` pins `SOURCE_EXTRACT` to
   `i1040sd--2025.txt`** — a **shipped** worksheet transcribed from one revision, with no per-revision
   table. Archiving `i1040sd--2026` reds nothing and changes nothing. This is the T8 shape from
   `CLAUDE.md`, in a compute crate, and it is the sibling `state_local_refund` was built to avoid.
   It is the answer to brief item 4 that item 4 did not anticipate: **`state_local_refund` is the ONLY
   per-revision family in the whole workspace** (1 prefix citation vs **11** literal pins).
2. **The instrument axis is stale too.** `xtask/src/prompt_check.rs` pins `i1040gi--2025`, `i8615--2025`,
   `i1099sa--2025`; `dependents_grid.rs:125-126` pins `f1040--2025` + `i1040gi--2025`;
   `line_coverage_check.rs:2239` pins `f8889--2025`. These checkers will keep checking the TY2025 documents
   after the TY2026 ones land **and stay green** — reported as `we must build` / **UNMEASURED**.
3. **17 archive gaps, not 2.** FR-181 names `f1040` and `i1040gi`. Derived against the families the source
   actually cites, the gap is 17 — including `i8949` (9 citing files), `i1040sca` (6), `i8615` (4),
   `f8889` (5), `i8283`, `i8960`, `i1040sd`, `i8275`, `f8615`, `f1099sa`, `f1098`, `f1099int`, `f1040s8`.
   Ranked by citing-file count, `i1040gi` (17) and `f1040` (9) are correctly the top two.
4. **`f8949` is the UNMEASURED case the brief predicted** — `the FORM = **UNWITNESSED**`, `line numbers
   UNREAD = 2`, `boxes UNREAD = 16`, and the row says *"the TY2026 revision is NOT known to be unchanged."*
   It is also the only stem whose UNWITNESSED axis is reported **twice**: once as a form blocker and once
   on its own, because the axis is a fact about the form whether or not the map is bundled.
5. **Two `Stem::ALL` members are NOT blockers and are suppressed with a reason** — `f8275` (served from
   `forms/2024/`) and `f8283` (from `forms/2025/`) resolve through `periodic_template`. A list would almost
   certainly have carried them.
6. **The `f8283` archive row is annotated PERIODIC from the declaration's own words**, so *"no TY2026
   revision"* is not read as a January wait for a form the IRS revises on its own calendar.

## 5. B1 — observed RED on three planted blockers. Output pasted.

Baseline: `58 rows (15 UNMEASURED)`, `41 / 13 / 3 / 1`. Every plant was reverted from a byte copy, never
by a checkout, and the working tree was confirmed clean after each.

### Plant A — hide an archived extract. The list GREW by exactly the right row.

`mv design/forms/extract/f1098e--2026.txt <scratch>` then re-run; `diff` against the baseline:

```
1c1
< # TY2026 blockers — DERIVED at HEAD, 58 rows (15 UNMEASURED)
---
> # TY2026 blockers — DERIVED at HEAD, 59 rows (15 UNMEASURED)
5c5
< ## clears itself in January — waiting only on an IRS document — 41 row(s)
---
> ## clears itself in January — waiting only on an IRS document — 42 row(s)
26a27
> | BLOCKED | when the IRS posts it | no TY2026 revision of `f1098e` (form) is archived; the newest is `f1098e--2025`, and 1 source file(s) read this family | `design/forms/extract/` holds f1098e--{2024,2025}; cited by crates/xtask/src/box_census.rs |
88c89
< * archive: … 11 already have a TY2026 revision archived [f1040s1a, f1040s2, f1040sa, f1040sc, f1098e, …]
---
> * archive: … 10 already have a TY2026 revision archived [f1040s1a, f1040s2, f1040sa, f1040sc, f6251, …]
```

+1 row, the right family, its newest prior revision named.

### Plant B — archive the January document. The row MOVES BUCKET (and refuted my own wording, §1).

`cp design/forms/extract/i1040gi--2025.txt design/forms/extract/i1040gi--2026.txt`, re-run, `diff`:

```
5c5
< ## clears itself in January — waiting only on an IRS document — 41 row(s)
---
> ## clears itself in January — waiting only on an IRS document — 40 row(s)
32d31
< | BLOCKED | when the IRS posts it | no TY2026 revision of `i1040gi` (instructions) is archived; the newest is `i1040gi--2025`, and 17 source file(s) read this family | … |
51c50
< ## we must build — 13 row(s)
---
> ## we must build — 14 row(s)
55a55
> | BLOCKED | now | `i1040gi--2026` is ARCHIVED but not TRANSCRIBED: the revision table for this family has no TY2026 entry, so every year that reads it refuses | `design/forms/extract/i1040gi--2026.txt` exists; `state_local_refund::revision_for(2026)` is None |
```

### Plant C — flip the year declaration to `filable`. The `YearReadiness` half GREW two rows.

`sed -i` on `crates/btctax-forms/forms/2026/YEAR.toml` (`preparing` -> `filable`), re-run, `diff`:

```
1c1
< # TY2026 blockers — DERIVED at HEAD, 58 rows (15 UNMEASURED)
---
> # TY2026 blockers — DERIVED at HEAD, 59 rows (15 UNMEASURED)
51c51
< ## we must build — 13 row(s)
---
> ## we must build — 14 row(s)
55c55,56
< | BLOCKED | with the params | TY2026 declares `status = "preparing"`, not `filable` … |
---
> | BLOCKED | now | TY2026: declared filable but no FullReturnParams are bundled — the full return cannot compute | `btctax_cli::year_readiness::YearReadiness::problems()` |
> | BLOCKED | now | TY2026: declared filable but the bundled price dataset ends 2026-06-03, before prices_through 2026-12-31 | `btctax_cli::year_readiness::YearReadiness::problems()` |
```

### Plants inside the suite — 12 tests, each carrying its own plant

`a_renamed_generator_column_is_refused_not_guessed` (one column renamed => `Err` naming it),
`an_unwitnessed_axis_is_unmeasured_and_an_unchanged_one_is_not` (both directions),
`hiding_an_archived_extract_grows_an_archive_gap_row` (asserts **exactly +1**),
`a_pinned_revision_is_unmeasured_and_a_prefix_one_clears_itself`,
`the_transcription_probe_is_not_the_archive`,
`every_gate_anchor_still_resolves_and_a_moved_one_is_refused` (plants a comment over each anchor),
`the_year_site_census_attributes_the_real_sites` (plants the same-line shape **and** an unknown year read),
`tags_are_stripped_of_markdown_emphasis`, `the_when_cell_takes_the_clause_that_names_a_date`,
`the_owner_axis_reads_the_pending_decisions_and_skips_the_ruled_ones`, `every_bucket_has_a_heading`,
`the_command_answers_for_ty2026_at_head`.

**Mutation-verified**, not merely asserted: reverting the tag stripper to a bare backtick trim reds
`tags_are_stripped_of_markdown_emphasis` **and** `the_command_answers_for_ty2026_at_head`
(10 passed, 2 failed). Restored, `find crates -name '*.rs' -exec touch {} +`, re-run green.

## 6. Three defects in my OWN instrument, found by running it

Recorded because each is the shape the brief warns about, and none would have been visible in review.

1. **★★ The tag parse — a plausible wrong answer with no error.** `design/TY2026_WORK_LIST.md` writes the
   regeneration command with markdown bold *outside* the code span, so a bare backtick trim yielded a tag
   with a stray backtick and two asterisks still attached. That passes `starts_with("2026")`, pairs against
   **no** archived stem, and turned all fifteen numeric rows into *"NO form axis ran"* with exit 0. Caught
   only by checking the note line `15 stems have a numeric row, 6 are in the excused table` against
   `port-status`'s real output. The integration test now asserts that sentence.
2. **Same-line attribution.** `RefuseReason::Schedule1aNotOnThisYearsReturn { year: ri.tax_year }` puts the
   construction *before* the year read on one line, so a forward search from the character offset skipped
   past it and attributed the year read to the **next** rule's variant. Fixed to search from the line start;
   the planted shape is in the test.
3. **A dropped site.** A year-reading site with no *preceding* construction was silently dropped instead of
   reported — the exact "invisible gap" failure. Now falls back to the forward search.

Plus one self-reference: `scan_citations` walks every `.rs` under `crates/`, **including `blockers.rs`**, so
its own test fixtures made it look like a module reading `i1040sd--2025`. Fixed by assembling the fixture
path at runtime rather than by exempting the file — a self-exemption would have hidden the class.

## 7. Design choices worth a sentence, for the record

* **`xtask blockers [<year>]`, not `ty2026-blockers`.** A year in a subcommand name is the rot the brief
  objects to. The default comes from `default_year()` (the newest bundled record, from the glob).
* **The generator's cells are PARSED, not re-derived** — indexed by header **name** over `REQUIRED_COLUMNS`,
  so a renamed column is an error rather than a plausible wrong cell. The generator gained six columns in
  two days; a positional reader would have drifted through all of them silently.
* **`Who` is `_`-free everywhere** with an `ALL` const a test holds to `heading()`, so a fifth bucket is a
  build error rather than a group that never prints.
* **Stated boundaries, per `CLAUDE.md` rule 3** (printed in the report's own *"what this measured, and what
  it did not"* section): the gate set and the transcription probe are declared per family with `None` for
  "no probe"; the archive axis sees only families a `.rs` file names by path; it reads no network, so
  *published* is outside it and *archived* is what it knows; `when` is the declaration's claim, not a
  prediction.
* **`BLOCKED` vs `UNMEASURED` reuses the work list's `unchanged` / `UNWITNESSED` split** rather than
  inventing a second vocabulary, as the brief required. 15 of 58 rows are UNMEASURED.
* **Nothing was added to a shipped crate.** The year-level axis calls `YearReadiness`'s existing public
  surface; the whole instrument is one dev-only module plus three lines of dispatch, so stage 2 can delete
  or reshape it without touching anything published.

## 8. What this hands stage 2

The command is the prediction; **stage 2's product is the gap between it and what actually breaks.**
The throwaway run should be designed to hit what these rows say it will hit first — and the interesting
outcome is a wall that appears in **none** of the 58 rows.

* The first wall is not a form. It is `price_coverage_or_refuse(2026)`: the bundled dataset ends
  **2026-06-03**, so even a fully-made-up TY2026 return refuses at export before any byte. A made-up run
  must either satisfy `prices_through` or accept that `export-irs-pdf` is unreachable — decide which
  **before** starting, because it changes what stage 2 can observe at all.
* Then `full_return_for(2026) == None`: the full return does not compute. `ty2026_full_return()` exists and
  is deliberately not inserted (`tax_tables.rs` gate reasons 2 and 3, FR-47). A run that wants a computed
  return has to decide whether to insert it **in a throwaway branch**.
* Then `slice_can_print(2026) == false`: no Form 8949 / Schedule D map for the year, so the crypto slice
  cannot print either. Both exits are closed; **neither** artefact is reachable at HEAD.
* `income import` on a TY2026 return **with a state or local income-tax refund and prior-year itemizing** is
  the one refusal that is genuinely year-specific — a cheap, high-value probe, and the only place a
  `RefuseReason` will differ from TY2025.
* The 15 UNMEASURED rows are where unknown unknowns will concentrate: 11 pinned revisions and `f8949`'s
  UNWITNESSED axis are, by construction, the places nothing is watching.

★ Run `cargo run -p xtask -- blockers` at the **start and end** of stage 2 and diff the two outputs: rows
that disappeared are blockers stage 2 cleared, and walls that appear in the transcript but in **neither**
run are the report stage 2 is actually for.

## 9. Files

* `crates/xtask/src/blockers.rs` — new, untracked.
* `crates/xtask/src/main.rs` — modified: `mod blockers;`, one `SUBCOMMANDS` row, one dispatch arm.
* `crates/btctax-core/src/tax/state_local_refund.rs` and the 8949 emitters were **read only** and are
  untouched, per the brief. No TY2026 param was bundled, no IRS figure invented, nothing archived.
* Nothing committed. Nothing pushed.
