# REVERIFY — the B3 fold (`db056c57`), independent sonnet pass

**Scope:** the fold's own diff only (23 files, +1,296/−51, per `BRIEF-reverify-b3-fold.md`). Worktree at
`95027636` (one commit past the fold). Did not re-derive anything the brief marked settled; pressed the
seven named hunts plus a fresh green-and-blind sweep.

---

## Verdict

**0 Critical / 0 Important / 0 Minor / 0 Nit.**

The fold closes C-1, I-1, I-2 and N-1 as claimed. Every one of the ten new tests is non-vacuous and tied to
a real code path (traced by hand, then run directly — see below); none of the four corrected doc comments
overstates anything; the two new FOLLOWUPS entries (FR-118/FR-119) don't collide with existing numbers and
carry real reproductions. `cargo check --workspace --tests` and `cargo clippy --workspace --tests
--all-targets -- -D warnings` are both clean in this worktree (I ran them myself, not quoted from the
fold). I ran the ten new tests directly (not the whole suite) and all ten pass:

```
b3_c1_the_year_reaches_every_reader::{the_acquisition_debt_ceiling_warning_renders_on_a_year_whose_package_exists,
  a_thirty_year_old_dependent_is_walked_down_the_qualifying_relative_branch,
  the_entry_screens_completeness_line_counts_a_year_scoped_question,
  a_sixty_year_old_is_not_asked_the_form_8615_questions}                                    PASS (4)
draw_edit::b3_i1_the_drawn_sentence_is_the_recorded_one::every_declaration_field_draws_...   PASS (1)
apply::tests::{a_yearless_surface_cannot_materialize_or_edit_anything,
  a_surface_editing_a_different_year_than_the_return_is_refused_and_nothing_changes}         PASS (2)
input_form_store::tests::{every_working_return_the_store_hands_out_states_the_year_it_was_loaded_for,
  the_draft_write_states_the_year_and_refuses_another_years_return}                          PASS (2)
main.rs tests::a_return_authored_and_committed_in_the_form_screens_clean_on_the_committed_row PASS (1)
open_next_year_t4b::nothing_about_year_n_changes                                             PASS (1)
return_refuse::param_free_tier::every_param_free_rule_is_censused_from_the_source_...         PASS (1)
```
(12 lines above cover the ten distinct tests; some ran in one nextest invocation.)

---

## Findings

None.

---

## Refuted premises

None. Every "already settled" row in the brief held up under spot-check (I did not fully re-derive the
citation sweep or the two controller-replanted kills, per the brief's instruction not to repeat them, but
nothing I touched while pursuing the seven hunts contradicted them).

---

## Checked clean — one line per hunt

**1. Is the I-1 fix complete, or only complete where measured?** Derived the full production set of
`.label` occurrences across the workspace (`grep -rn '\.label\b'` over every crate, then triaged each
non-test hit by hand): `draw_edit.rs:2955` (`push_field_lines`, via `field_label` — the fix) is the only
production site that draws a `Field`'s registry label. Two other `.label` sites looked plausible and were
run down and cleared: `draw_edit.rs:2063` draws `filing_status_field().label` directly (NI-2, before a
return exists), but `field_to_question(FieldId::FilingStatus)` returns `None`, so `field_label` would
resolve to the identical static string anyway — no divergence, now or if this were routed through the
resolver. `draw_edit.rs:2392` / `main.rs:12212` draw `PendingRemove.label` and `FileReport.label` — unrelated
structs, not `Field`. Also checked `cmd/answer.rs`'s CLI prompt loop (`q.prompt_text(&ri)` at line 901): it
was never on `Field.label` in the first place — it already goes straight from `FORM_QUESTIONS` into the
rendered text, so it was never a second instance of I-1. No other production caller of `field_label` or
`push_field_lines` exists (`grep -rn field_label` across `crates/` returns exactly one call site).

**2. Can a kill pass while the defect is present?** Read all ten new tests' bodies and traced what each
would do under a narrower-than-full plant. Consequence-2 and consequence-4 (the ones the fold's own report
flags as previously green-and-blind) now leave exactly one question unanswered / read the whole rendered
message and assert both figures — non-vacuous, and I confirmed by hand that a materialization-only plant
(no per-edit reconcile) would still red exactly these two, because neither issues a second `apply()` call
before reading `ri.tax_year`-derived output, while the other three C-1 kills (the end-to-end test,
consequence-3, consequence-5) each issue a *second* `apply()` call after materializing, so the per-edit
reconcile (`apply.rs:163-174`) would silently repair a materialization-only regression for those three.
This matches the fold's own disclosure ("partial plant reds 2 of 5 — the reconcile stamp is load-bearing")
exactly, and is why the *full* plant (both halves reverted) is the one that reproduces the true pre-fix
state — which the controller already confirmed reds all five. Re-ran all ten tests directly myself
(command output above) rather than trusting the fold's transcript. The I-1 pin's own coverage assertion
(`never_drawn.is_empty()`) fails safe: `current_prompt` returning `None` for a probe/question pair skips
that probe without marking it `seen`, so a genuinely-unreachable `QuestionId` is caught, not hidden; the
anti-vacuity floor (`comparisons > QuestionId::ALL.len()`) is also present.

**3. Did the nine restated fixtures start testing something different?** Read all nine diffs
(`return_1040.rs` ×5, `return_refuse.rs`, `testonly.rs::amt_owing_household`, `spec/sections.rs`,
`cmd/tax.rs`). Each adds exactly one `tax_year: 2024` (or, for `cmd/tax.rs`, a `stamp_year` call on an
existing TOML-parse fixture) with no other line touched. Confirmed the money paths these fixtures exercise
take the year as a function argument, never `ri.tax_year` — `return_1040.rs:2436-2439`'s own comment
records a first draft used `ri.tax_year` and a mutation caught it, and that comment predates this fold
(pre-existing, `03527f73`-era code, not part of this diff). No fixture moved to a different branch of any
rule as a result of stating its year.

**4. Is the new refusal safe?** Enumerated every non-test production caller of `screen_inputs` /
`screen_param_free` (grep across `crates/`): all resolve through `screen_inputs_tiered`, where the new
year check is unconditional and first, before `first_negative_amount` — so nothing else can pre-empt or be
pre-empted by it. Traced all nine producers named in `return_inputs.rs`'s corrected doc table and confirmed
each stamps before any screen can see the return: committed row read/write (§G-15, pre-existing), draft
row read/write (this fold, code read directly), `apply` (this fold), the TUI commit gate (this fold, stamp
now precedes `screen_inputs` at `input_form_store.rs:686-695`), `income import` (FR-103, pre-existing,
unchanged except a test fixture), `open_next_year::seed` (pre-existing, unchanged, not in this diff). The
one residual case the fold names — `get_draft_row` refusing a pre-fold year-0 blob — is real but
unreachable today: confirmed via `MEMORY.md`'s own "No users yet" note (btctax has never shipped a user)
that there is no such blob in the wild. A TOML with an explicit conflicting `tax_year` is caught by
`stamp_year`'s own disagreement error before `screen_param_free` runs (pre-existing FR-103 behavior,
unaffected by this fold).

**5. Can a wrong year be stamped?** Traced `form.year`'s only origin: `TaxInputsFormState.year` is set once,
at construction (`main.rs:815`, `let year = app.selected_year;`), and is never mutated afterward (grepped
for reassignment; found none in production code). The three `Loaded` variants (`Fresh`/`Committed`/`Draft`)
all construct `TaxInputsFormState` with that same `year`, and `Committed`/`Draft` carry a `ri` already
stamped to that same `year` by `load()`'s own boundary — so `form.year == form.working.tax_year` by
construction whenever both exist. `apply`'s one production caller (`edit/tax_inputs.rs:561`, confirmed at
that exact line) always passes `form.year`. No path hands `apply` a year different from the screen's own.

**6. The four corrected doc comments.** Read all four in full against the current code:
`return_inputs.rs:2104+`'s nine-producer table — every row checked against source and correct;
`questions.rs:920+`'s claim that the predicate is "true by construction" — true, given hunt 5's finding;
`return_refuse.rs:2459+`'s "now BELT, not the rule's premise" — matches the code (the new check runs first,
the old conjunct stays as a second layer); `coverage.rs:696+`'s corrected exemption reason — now names
`apply` and the editing surface rather than only the CLI. None overstates.

**7. FOLLOWUPS.md.** FR-118 and FR-119 are new, sequential after FR-117 (the prior highest), no duplicate
numbers anywhere in the file. FR-118's reproduction (`$1200000.00`/`$750000.00`) is the literal string the
consequence-4 kill asserts — verified by reading that test. FR-119's reproduction (`cargo doc` warnings)
was not independently re-run by me (N-1's own fix was, see below), but the named files/lines are real
doc-comment `[TAG]` spans, spot-checked one (`forms.rs`).

**N-1 spot-check (not a numbered hunt, but load-bearing for the Nit count):** `grep -rn
"ExcessSsSingleEmployerNotCreditable" crates/` returns zero hits — the dead link name is fully gone.

---

## Out of scope

Per-task correctness, the rest of the branch, the citation sweep already machine-checked by the controller
(`22ed5e2a`), the two kills the controller already re-planted (I-1's full revert and C-1's full plant) — not
repeated, per the brief. `open_next_year.rs` (unchanged by this fold, confirmed by its absence from the
diff's file list) and the pre-existing FR-103 `income import` stamping path were read only far enough to
confirm this fold's new code composes safely with them, not re-audited for their own correctness.
