# FOLD — the B3 whole-branch review

**Report folded:** `design/agent-reports/2026-09-11-b3-whole-branch-review.md` (persisted verbatim at
`587d7a9c`, 1C/2I/0M/1N). **Ledger:** `…-VERIFICATION.md` (`22ed5e2a`). **Brief:**
`BRIEF-fold-b3-whole-branch-review.md` (`900f3a62`). One opus implementer, shared main tree, nothing
committed.

**All four findings are folded.** Nothing in the brief was refuted; the one premise it asked me to test
before building on it was **confirmed empirically**, and the measurement is below.

---

## Gate

| check | result |
|---|---|
| `make gate` (forced recompile, then `nextest` + clippy `-D warnings`) | **3571 tests run: 3571 passed, 12 skipped** |
| baseline at `900f3a62` | 3561 passed / 12 skipped → **+10 tests, 0 regressions** |
| `cargo fmt --all --check` | clean |
| `make docs` | **no diff** (`git status --short docs man` empty) |
| `xtask stop-list` | OK — 8 input-form sources, 4 state-bearing, 6 renderer, **91 registry prompts and 209 AUTHORED field labels across 30 sections** (70 transcribed captions exempt); no forbidden shape |
| `xtask box-census` | OK — 268 printed boxes across 19 editions of 9 information returns, every one decided |
| `xtask prompt-check` | OK — 90 assertions, all verbatim |
| `xtask census-join` | OK — 274 unmodeled entries across 13 maps |
| `xtask cite-check` | OK — 51 quotations, all verbatim |
| `cargo doc --workspace --no-deps` | the two N-1 links now resolve (`grep -i ExcessSs` on the warning stream returns nothing) |

---

## Refuted premises — none. The one measured premise is CONFIRMED

The brief flagged one claim to test before building on it (§0): *"stamping only at the commit gate would
leave consequences 2–5 open **and** leave a year-0 sentence already hashed in the answer log."*

I measured it rather than reasoning about it. With the C-1 kill in place and **nothing else changed**, I
planted a gate-only stamp — `stamp_year` immediately above `screen_inputs` in
`input_form_store::commit`, exactly the shape the brief described — and ran the blocking scenario:

```
thread 'tests::a_return_authored_and_committed_in_the_form_screens_clean_on_the_committed_row' panicked at
crates/btctax-tui-edit/src/main.rs:11324:9:
the commit gate reported success: Some("you answered this question, but the wording of this question
changed since you answered — so the answer on file was given to a different question. It has been kept in
the answer log's history and is no longer treated as your answer. Re-answer it with `btctax income answer`
(or in the tax-inputs editor); nothing else about your return has changed")
```

The test fails at the **earlier** assertion — the commit never happens. A gate-only stamp converts an
unprintable *committed* return into an **uncommittable** one: the year-0 sentence is already hashed by the
time the filer presses `s` (`apply.rs:116-121` records during editing), so screening the stamped row
misses the hash and the gate refuses a correct answer with a false reason. Fail-closed, still wrong, and
consequences 2–5 all run before the gate and are untouched. The brief's direction stands; the plant was
reverted before any fix was written.

---

## C-1 (Critical) — the form-engine surface never stamped `ReturnInputs.tax_year`

### The fix: the year is a session fact the engine REQUIRES, plus a stamp at every storage boundary

**One rule, one implementation.** `ReturnInputs::stamp_year`
(`crates/btctax-core/src/tax/return_inputs.rs:2712`) — `0` takes the year, an equal year is a no-op, any
other value returns `Err(existing)`. It lives in core because two crates that cannot see each other both
need it: `btctax_cli::return_inputs::stamp_year` (`crates/btctax-cli/src/return_inputs.rs:119`) is now only
the CLI's error vocabulary over it. That is the FR-103 comment's own reasoning applied to itself — *"two
copies of 'which year is this row' is precisely how they come to disagree"*, and C-1 **was** the
disagreement.

**The engine takes the year and refuses to run without one** —
`crates/btctax-input-form/src/apply.rs:154`:

```rust
pub fn apply(w: &mut Working, e: Edit, year: i32, now: Date) -> Result<(), ApplyError>
```

- `year == 0` → `ApplyError::TaxYearNotStated` (`seam.rs:831`);
- an already-materialized return is stamped/reconciled first, so a disagreement is
  `ApplyError::WrongTaxYear { on_return, surface }` (`seam.rs:836`) with nothing mutated;
- materialization states it: `apply.rs:196`, `tax_year: year` over `ReturnInputs::default()`.

**Why the parameter rather than a stamp at the call site.** A call site is a typed list of one, and the
next surface is free to omit it — the failure `CLAUDE.md` records eight times. Here the compiler holds it:
`apply` is the **only** production producer or mutator of a `Working` (measured: a `#[cfg(test)]`-aware
scan finds exactly one production caller, `edit/tax_inputs.rs:561`; every other `apply(` in the workspace
is inside a test module), so a renderer that does not state a year **does not build**, and `0` is refused
at run time. A yearless working return is no longer reachable from any surface.

★ `ReturnInputs::tax_year`'s own doc comment argues against `live(year, &ri)` because *"a year passed
alongside invites a caller to pass one that disagrees with the row's own origin"* — that reasoning is about
**readers**. `apply` is a **writer**, and it is the same shape as `return_inputs::set`, which has taken a
`year` alongside the return and refused a disagreement since §G-15. Recorded at `apply.rs:127-152`.

**The stamp runs ahead of the per-edit guards**, and `apply.rs:163-167` says why that does not weaken *"a
refused edit changed nothing"*: stating which year a screen is editing is the **surface** speaking, not
the filer, it writes no `AnswerRecord`, and it is idempotent. Stamping after the guards would let a failed
edit leave a return whose next successful edit hashes a sentence rendered at year 0.

**The three storage boundaries now agree** (the report's table, closed):

| boundary | site | change |
|---|---|---|
| draft row, **read** | `input_form_store.rs:65` | stamps from the row key, as `row_to_inputs` has since §G-15 |
| draft row, **write** | `input_form_store.rs:88` (`set_draft_row`) | stamps, refusing a disagreement, as `set` does |
| TUI **commit gate** | `input_form_store.rs:693` | `stamp_year` **then** screen — FR-103's shape on the other writer |
| fresh working return | `apply.rs:196` | states the surface's year |

The draft read boundary matters most: `input_form_store.rs:348-352` makes the draft the *primary* store for
months on a params-less year, and `draft_is_disposable`'s own seed comment already **assumed** a draft
carries the filer's `tax_year` — the code disagreed with itself.

The production caller — `crates/btctax-tui-edit/src/edit/tax_inputs.rs:561`,
`apply(&mut form.working, edit, form.year, form.now)`. `form.year` is `EditorApp::selected_year`, whose
default is `year_readiness::default_year()` = the max bundled year, so neither new `ApplyError` is
reachable from this flow; `apply_error_msg` maps both to sentences rather than `unreachable!()`ing, because
the engine's refusal is the guard and a future renderer must meet a message, not a panic in an
alternate-screen TUI (`tax_inputs.rs:948-960`).

### Closing the CLASS: `screen_inputs` now refuses a yearless return

`RefuseReason::ReturnInputsYearNotStated` (`return_refuse.rs:98`), raised **first** in
`screen_inputs_tiered` (`return_refuse.rs:2450-2470`), anchored `NotInForm` in
`attribute.rs:313` (the tax year is the one `ReturnInputs` scalar with no `Field` at all), and the
`NotInForm` count pin bumped with its justification (`attribute.rs:876-891`).

**This revisits the decision the brief pointed at**, and the brief was right that C-1 refutes its premise.
`return_refuse.rs:2459` declined to refuse `tax_year == 0` reasoning *"a yearless `ReturnInputs` is a test
convenience, and refusing it would be an assertion about a year nobody named."* The first half was false —
year 0 was the normal state of the editing surface — and the second is backwards: a rule going **silent**
on an unstated year is the assertion, because it asserts that every year-scoped question is inapplicable.
The old `tax_year != 0` conjunct on the Schedule 1-A rule is kept as belt, with its reasoning corrected
(`return_refuse.rs:2489-2496`).

**Cost, measured before deciding:** a probe refusal at the top of the screen reddened **9** of 3561 tests
— nine fixtures that screened at year 0 against a TY2024 package, i.e. fixtures whose screen and package
disagreed about the year. All nine now state one (`return_1040.rs` ×5 sites, `return_refuse.rs`,
`testonly.rs::amt_owing_household`, `spec/sections.rs`, `cmd/tax.rs`). The AMT vector's figures do not
move: every one is derived from the package and the `year` argument, never from `ri.tax_year`
(`return_1040.rs:2436-2439` records that a first draft used `ri.tax_year` and a mutation caught it), and
`golden_packet`/`attestation`/the oracle harness are all green.

**And it is not an unreachable ornament — it is what makes C-1 itself fail closed.** With the *original*
code (no stamp anywhere, the exact pre-fix state), the commit gate now **refuses and names its exit**
instead of reporting success on a return no printing surface will accept. Measured, under a full plant:

```
the commit gate reported success: Some("this return does not say which tax year it is for, so no rule that
depends on the year can be applied to it — the year decides which questions you are asked, which schedules
exist, and the words those questions are put in. The year is not something you type on the return: it comes
from the year you are working in (`--year`, or the year you opened in the tax-inputs editor). Reopen the
year this return belongs to, or re-import it under that year. (the tax year is not a field of this form —
it is the year you opened (`--year`), stated onto the return by the surface you are editing in. Reopen the
year this return belongs to)")
```

### The four asserted premises — all now true, or corrected

| site | what changed |
|---|---|
| `return_inputs.rs:2104+` | the false half (*"the storage boundary stamps this…"* — true of the committed row and nothing else) replaced by a **table of all nine producers** plus the screen backstop |
| `questions.rs:920+` | no longer calls a year-0 return a *fixture*; states that every producer states the year, and that the `>= 2025` direction is unchanged |
| `return_refuse.rs:2459` | the decision itself reversed (above); the surviving conjunct's comment corrected |
| `coverage.rs:696+` | the exemption's **reason** corrected — it reasoned about the CLI while governing the editor's working return, which is the gap that was the Critical |

Also corrected: `edit/form.rs:457-468`, whose comment said the ceiling check *"waits exactly as the box-3
wage-base check does"* on a params-less year. It waited on **every** year; the sentence is true now, and
the note says why reading `ri.tax_year` (rather than `self.year`) is the better read once `apply` refuses a
disagreement.

Removed: the FR-97 fixture's hand-stamp (`apply.rs`, `with_one_dependent`, formerly `w.tax_year = 2024`). The report named it as the
tell, and it was: the fixture compensated in one line for what production never did.

### Kills — C-1

**1. The blocking scenario, end to end through the seam** —
`crates/btctax-tui-edit/src/main.rs:11280`,
`a_return_authored_and_committed_in_the_form_screens_clean_on_the_committed_row`. Materialize by choosing
a filing status, answer `DeclDigitalAssetActivity` **through `apply`** (one of the five `RENDERED_PROMPTS`,
`live` for every filer in every year), commit through the modal, then screen the committed row as `report`
and `export-irs-pdf` do.

RED at the unfixed HEAD (`900f3a62`), before one line of the fix existed:

```
thread 'tests::a_return_authored_and_committed_in_the_form_screens_clean_on_the_committed_row' panicked at
crates/btctax-tui-edit/src/main.rs:11347:9:
the commit gate said this return was clean and the committed row refuses: Some((DigitalAssetActivityUnanswered,
"you answered this question, but the wording of this question changed since you answered — so the answer on
file was given to a different question. …"))
```

GREEN after the fix. Exactly the report's consequence 1, including the refusal's false reason.

**2–5. One kill per consequence** — `crates/btctax-tui-edit/src/edit/form.rs`, module
`b3_c1_the_year_reaches_every_reader`, all four driven through `apply` with the year the screen is open on:

| # | test | line |
|---|---|---|
| 2 | `the_entry_screens_completeness_line_counts_a_year_scoped_question` | `4753` |
| 4 | `the_acquisition_debt_ceiling_warning_renders_on_a_year_whose_package_exists` | `4785` |
| 5 | `a_sixty_year_old_is_not_asked_the_form_8615_questions` | `4833` |
| 3 | `a_thirty_year_old_dependent_is_walked_down_the_qualifying_relative_branch` | `4893` |

All four RED under a plant that removes only `apply`'s stamp (the exact pre-fix state):

```
consequence 2: a TY2025 return with the §911 exclusion question unanswered is NOT complete — at year 0
  that question was not live, was not counted, and this line said `complete`:
  interview: complete · return: NOT computable · authoring and saving work; …

consequence 4: a $1.2M 2021 acquisition-debt balance on a Schedule A return must warn on TY2024, whose
  package declares the $750,000 ceiling: ""

consequence 5: assertion `left == right` failed: neither age-gated Form 8615 skippable may be live for a
  filer who is provably 24 or older
    left: 2 / right: 0

consequence 3: the §24(h) under-17 test must fail too — the CTC column is decided by the same year
```

All four GREEN after. Each is non-vacuous in its own body: consequence 2 leaves **exactly one** question
unanswered and it is the year-scoped one; consequence 4 reads the *whole* message with the cursor on the
Form 1098 row and asserts both figures; consequence 5 asserts a 14-year-old **is** asked both, so a
uniformly-false predicate cannot pass it; consequence 3 asserts a nine-year-old **does** pass §152(c)(3)
and the under-17 test.

★ The first assertion of consequence 2's original draft did **not** discriminate (a fresh TY2025 return has
28 blocking items at either year, so the line said *"28 question(s) still to answer"* both ways). I
rewrote it to reproduce the reported **false `complete`** before accepting it. Recorded because it is the
failure this repo's B1 exists to catch: the test would have shipped green and blind.

**6–7. The structural guard's own planted-defect tests** —
`crates/btctax-input-form/src/apply.rs:606` `a_yearless_surface_cannot_materialize_or_edit_anything` and
`:644` `a_surface_editing_a_different_year_than_the_return_is_refused_and_nothing_changes`.

```
PLANT A (delete the `year == 0` arm):
  assertion `left == right` failed: year 0 is §G-15's "NOT STATED" — it is not a year, and materializing
  at it is C-1
    left: Ok(()) / right: Err(TaxYearNotStated)

PLANT B (delete the `stamp_year` call):
  assertion `left == right` failed
    left: Ok(()) / right: Err(WrongTaxYear { on_return: 2024, surface: 2025 })
```

**8–9. The store boundary's kills** —
`crates/btctax-cli/src/input_form_store.rs:1124`
`every_working_return_the_store_hands_out_states_the_year_it_was_loaded_for` (an `_`-free match over
`Loaded`'s three variants, so a future variant carrying a `ReturnInputs` is a compile error here; the
fixture writes a year-0 blob through **raw SQL**, past the stamping writer, because the thing under test is
the read) and `:1167` `the_draft_write_states_the_year_and_refuses_another_years_return` (reads the **blob**,
not the accessor, so a read-time stamp cannot make an unstamped write look fine).

```
PLANT C (delete the draft-read stamp):
  the row key is authoritative: a working return loaded for 2026 must SAY 2026
    left: 0 / right: 2026

PLANT D (delete the draft-write stamp):
  the bytes on disk state the year, so the two representations cannot differ
    left: 0 / right: 2024
```

**10. The screen backstop's kill** — `return_refuse.rs:8679`, a fixture in `param_free_fixtures()` that
sets `tax_year = 0`. The derived census
(`every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths`) **demanded** it the moment
the rule landed, reading the rule set out of the screen's own body, and asserts it fires on both the
param-free and the packaged tier. That census is also what red-flagged the rule to begin with:

```
the fixture table and the source census must name the same param-free rules (left: fixtures, right: source)
  right: {…, "QualifiedTipsCautionNotMet", "ReturnInputsYearNotStated", "SaAccountTypeNotTranscribed", …}
```

### A latent fixture defect the fix uncovered

`crates/btctax-cli/tests/open_next_year_t4b.rs` — `draft_holding_an_interview()` hardcoded
`tax_year: TO` (2025) and `nothing_about_year_n_changes` stored the result under `FROM` (2024): **a TY2025
return saved as year N's draft.** Nothing reded, because the draft store neither stamped nor checked the
year. The new write-side stamp reddened it immediately, with both years named. The fixture now takes the
year as a parameter. This is the FR-99 shape again (a literal beside a set that grew), and it is the
deriving-finds-more evidence the rule predicts.

---

## I-1 (Important) — the registry drew the static prompt and hashed the rendered one

### The fix: the drawn words resolve through the SAME resolver as the hashed words

`btctax_input_form::field_label(f, ri)` — `crates/btctax-input-form/src/spec/registries.rs:729`, exported
at `spec/mod.rs:11`. For a `FieldId` that `field_to_question` maps, it returns
`provenance::current_prompt(&AnswerKey::Question(q), ri)`; otherwise `Cow::Borrowed(f.label)`. The renderer
calls it — `crates/btctax-tui-edit/src/draw_edit.rs:2955`, in `push_field_lines`.

**Why a resolver and not five edited literals.** `Field.label` is `&'static str` and *cannot* be the
sentence for a question whose subject is a value on the return, so editing the labels is not available;
and editing anything leaves the sixth free to diverge. `current_prompt` is documented as *"the one place
that knows which words a key is asked in"* and is already what `apply` hashes through, so routing the
drawn words through it makes the two **one string by construction**. That is the standard the sibling
registry's pin states in terms (`apply.rs:2076`,
`the_gate_fields_draw_the_words_they_hash_except_the_one_named_date_leaf`) — FR-99's shape with the set
being *"registries whose `Field` label must equal the words it hashes"*.

**Scope stated in source** (FR-99 option 3, `registries.rs:716-726`): the declaration registry only. A
skippable's `Field` label *is* `SKIPPABLE_QUESTIONS[i].prompt` and `current_prompt` returns that same
static, so there is nothing to diverge; the twenty-one dependent gates are the sibling registry, already
held, with its one filed exception (FR-100, `DepDob`). `RENDERED_PROMPTS` is keyed by `QuestionId`, so a
skippable cannot gain a rendered prompt today — if that changes, `field_label` must widen with it, and the
doc comment says so.

### The two interactions — checked, not assumed

**R15's label scan is NOT blinded, and I measured why.** `Field.label` is deliberately **unchanged** (still
a `&'static str`), so `r15_stop_list.rs:457` scans exactly the same text it did before — and separately,
`registry_prompts()` at `r15_stop_list.rs:379-383` *already* renders every `RENDERED_PROMPTS` entry against
a probe `ReturnInputs::default()` and scans the result. So the sentence that is now drawn was already in
R15's input set. Instrument output after the fold is byte-identical on the counts: *"91 registry prompts
and 209 AUTHORED form_spec field labels across 30 sections scanned (70 transcribed captions exempt)"*.

**`box_census`'s caption join is untouched** for the same reason: `box_census.rs:1391` builds
`format!("{} {}", f.label, f.help)` from the unchanged `f.label`. `xtask box-census` reports OK — 268
boxes, every one decided. (None of the five is a `DocumentCaption` field in any case.)

### Kill — I-1

`crates/btctax-tui-edit/src/draw_edit.rs:8965`,
`b3_i1_the_drawn_sentence_is_the_recorded_one::every_declaration_field_draws_the_sentence_its_answer_is_recorded_under`.
Walks `QuestionId::ALL` (no list of five), maps each to its `FieldId` through the total `question_to_field`,
renders **`push_field_lines`' own output** and asserts the pane contains `current_prompt`'s sentence.

Asserting on the drawn line rather than on `field_label` is deliberate: comparing `field_label` to
`current_prompt` would be a tautology, since one is defined in terms of the other. What must hold is that
the field pane puts those words on the screen.

**It asserts its own coverage**, and that is what saved it. The first draft used one probe return and
reddened on only **2** of the five — `StateRefundWithout1099g`, `ItemizedPriorYear` and
`QssSpouseDiedInWindowAndNotRemarried` were not `live` on it, so the walk skipped them silently. The test
now walks **9 probe returns** and fails naming any `QuestionId` it could never draw:

```
these declarations were never DRAWN on any probe, so this pin says nothing about them — extend `probes()`
rather than letting the walk report success about words it never saw:
[MortgageAllUsedToBuyBuildImprove, AmtQualifiedDwelling, AmtCarryoverSameAsRegular,
 AmtDepreciationSameAsRegular, MortgageWithinDebtLimit, CarryoverIncludesSpousesJointLoss,
 ExcludedCanceledDebt, ItemizedPriorYear, HsaSpouseFamilyCoverage, HsaDistributionWithout1099sa,
 FilerTinIssuedByDueDate, NraSpouseResidentElection, ClaimingMortgageInterestCredit]
```

Mutation-verified after the fix — revert `draw_edit.rs:2955` to `f.label` and it reds on **29
comparisons covering all five, and only those five**:

```
a declaration that draws one sentence and records another is B3's I-1: the filer's answer would read as
given under words they never saw. 29 such:
  FilingStatusConfirmed (probe Single)
      drawn:    Is the filing status carried from last year's return still your filing status for this tax year? …
      recorded: Your TY2023 return filed as Single. Is Single your filing status for TY2024? …
  …
      9  DigitalAssetActivity
      9  FilingStatusConfirmed
      1  ItemizedPriorYear
      1  QssSpouseDiedInWindowAndNotRemarried
      9  StateRefundWithout1099g
```

No exception is claimed for this registry, so none is filed. FR-100 remains the gate registry's.

---

## I-2 (Important) — shipped filer-facing text named a command that prints nothing

**Default fix taken: the sentence names the command that actually prints it.**
`crates/btctax-cli/LIMITATIONS.md:426`:

> btctax will tell you when this is live: `export-irs-pdf` — the command that prints the Form 8949 — names,
> as a `⚠ [I5]` advisory, how many of your dispositions occurred on an exchange and so *may* carry broker
> reporting.

I did **not** wire the advisory into `report`. The brief allowed it only on proof of behaviour-safety, and
the case against it is affirmative rather than merely cautious: the advisory's whole content is about
*which Form 8949 box the rows were filed under*, and `report` prints no Form 8949 — so on `report` it would
be a warning about a page the command did not produce. Naming `export-irs-pdf` also tells the filer the
truer thing: the advisory arrives at the moment the boxes are actually chosen. The substance of the
paragraph was already right (`broker_reporting_advisory(2024, Regime::NONE, 1)` returns
`Some(..)`, pinned at `admin.rs:2470`), and the added clause *"the command that prints the Form 8949"*
means a filer who has only ever run `report` knows why they saw nothing.

`LIMITATIONS.md` is `include_str!`'d at `main.rs:582`; `make docs` shows **no diff** — checked, and the
reason is worth recording: `docs/man/btctax-limitations.1` carries only the clap help text, not the
document body (13 lines), so the *body* is single-sourced into the **binary** and nowhere else. The
brief's and the report's "single-sourced into the man page" is true of the command's summary line, which
this change does not touch.

---

## N-1 (Nit) — two broken intra-doc links

Both fixed inline: `crates/btctax-core/src/tax/return_1040.rs:2016` and
`crates/btctax-core/src/tax/return_refuse.rs:308` now link
`Advisory::ExcessSsNotCreditable` (the variant that exists, `advisories.rs:153`). Verified by running
`cargo doc --workspace --no-deps`: nothing matching `ExcessSs` appears in the warning stream. No other doc
link chased, per the brief.

---

## Residue — filed, not fixed

Both in `FOLLOWUPS.md`, under a new heading *"From the B3 whole-branch review fold (2026-09-11 …)"*:

- **FR-118 (Nit, owning phase: ownerless residue — batch when convenient)** — the §163(h)(3)(B) ceiling
  warning prints `$1200000.00` / `$750000.00`, no thousands separators, because
  `transcription_warnings.rs`'s `money()` is `format!("${v:.2}")`. Measured while writing consequence 4's
  kill. The whole content of that warning is a comparison of two seven-figure numbers; the module's own
  N-1 fold already moved these onto `money()` *because* raw interpolation printed `$900000`. The fix moves
  two existing tests that pin the current spelling, and changing a filer-facing number format deserves its
  own decision about which formatter the product uses everywhere.
- **FR-119 (Nit, same owning phase)** — bracketed review tags in doc comments (`[I5]`, `[N6]`, `[N2r]`,
  `[r]`) read as broken intra-doc links, so `cargo doc` emits `unresolved link to …` warnings. Same class
  as N-1 and found by running `cargo doc` to verify N-1's fix; nothing is mis-linked, but the noise is what
  let N-1 survive. Wants one mechanical sweep plus `-D rustdoc::broken_intra_doc_links` in CI. The brief
  scoped N-1 to *"Do not chase other doc links."*

### Stated boundaries rather than follow-ups

- `field_label` covers the declaration registry; the skippable and dependent-gate registries are named in
  its doc comment with the reason each needs nothing (`registries.rs:716-726`). Not filed, because
  `RENDERED_PROMPTS` is `QuestionId`-keyed and the gate registry already carries its own derived pin.
- `get_draft_row` now **refuses** a draft blob whose `tax_year` disagrees with its row key, rather than
  re-labelling it. Reachable only from a blob written before this fold (there are no users) or by a future
  bug, since `set_draft_row` refuses to write one; re-labelling would be exactly the misattribution
  `stamp_year` exists to prevent. Recorded here so the choice is visible.

---

## What was NOT touched

No scope beyond the four findings. Per-task correctness, the two journey walks, the 8949 box truth table,
`verdict_reach`, the oracle routing partition and FR-95/FR-100/FR-101/FR-110/FR-117 are untouched. The
mechanical part of the C-1 fix — adding the `year` argument — moved **116 test call sites** of `apply`
across four files (`btctax-input-form/src/apply.rs` 84, `btctax-tui-edit/src/draw_edit.rs` 19,
`btctax-tui-edit/src/main.rs` 7, `btctax-cli/tests/tax_report.rs` 6) plus the one production caller; each
was rewritten by a script that inserted the argument before the existing `Date` argument, and the suite is
what checked it.

**Nothing committed, pushed, stashed or checked out.** 23 files modified in the working tree.
