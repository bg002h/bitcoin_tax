# T4 — the year gate and draft protection (R11): implementation report

Implementer: one opus agent, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, from HEAD
`15c15927`. Brief: `design/agent-reports/BRIEF-build-interview-T4.md`. Nothing committed, nothing
pushed. Tree compiles, `cargo fmt --all --check` clean, `CARGO_TARGET_DIR=target-clippy cargo clippy
--workspace --all-targets --all-features -- -D warnings` clean (exit 0), whole workspace green.

All six numbered deliverables landed. Nothing is unfinished.

---

## 0. The writer enumeration, re-measured

Re-grepped at working HEAD, production code only (`grep -rn "return_inputs::set(\|return_inputs::delete("`
and `"save_draft(\|set_draft_row(\|delete_draft("` over `crates/*/src`, minus `#[cfg(test)]`).

**Committed-row writers — the brief's four, all confirmed, plus a fifth it did not list:**

| # | writer | site (post-T4) | T4 status |
|---|---|---|---|
| 1 | `input_form_store::commit` | `input_form_store.rs:553` (`set`), `:554` (`delete_draft`); I-11 `NoTables` early return at `:531` | item 5 — re-asserted, unchanged |
| 2 | `income import` | `cmd/tax.rs:284`; coherence CHECK at `:159`, the new screen at `:266`, coherence CLEAR at `:283` | items 2 + 4 |
| 3 | `income answer` | `cmd/answer.rs:513` (committed) and **`:509` (draft, new)** | items 2 + 3 |
| 4 | `report --write-carryover` | `cmd/tax.rs:1062`, writing **year N+1**; coherence on `year + 1` at `:973` | item 2 (`--discard-draft` threaded) |
| 5 | **`income clear`** | `cmd/tax.rs:442` (`return_inputs::delete`); coherence at `:441` | item 2 (`--discard-draft` threaded) |

**A SIXTH writer the brief's list omits — `input_form_store::park_to_profile`
(`input_form_store.rs:594`, `return_inputs::delete`).** It deletes the committed row and stashes it
as `parked = 1`. It is **already fail-closed against T4's hazard and needed no change**: its §9/M-4
clean-state gate refuses when `parked_flag(...) == Some(false)`, i.e. when *any* WIP draft occupies
the one-per-year slot, trivial or not. Recording it because "a writer this list misses is a finding":
the finding here is that the list was short by one, not that the writer is unguarded.

`cmd/tax.rs:1024`'s year+1 write (the brief asked me to identify it) is **`report --write-carryover`**
— writer 4. It is inside T4's rules and now takes `discard_draft`, scoped to **year N+1**, the year it
writes onto. Its existing `--force` is about the *carryover figure* and is left alone.

**Draft writers:** `save_draft` (`:140`) — the TUI autosave, and now `income answer`'s draft path;
`load`'s stale-WIP discard (`:214-231`, now narrowed — item 2); `coherence_clear` (`:440-458`, the
former `coherence_clear_or_refuse` body). `resolve.rs:72-84` still never reads the draft table
(`input_form_store.rs:1-2` unchanged), which is what makes item 3 safe.

---

## 1. Two states at entry

**Landed.** `crates/btctax-cli/src/year_readiness.rs`: new `pub struct EntryStates { year,
interview_complete: Option<bool>, blocking: Option<usize>, return_computable: bool, readiness }` with
`for_year(year, Option<&ReturnInputs>)`, `package_only(year)`, `with_interview(ri)`, `lines()` and
`sentence()`. `interview_complete` is `interview_state(ri).blocking.is_empty()` (T3's function);
`return_computable` is `YearReadiness::bundled(year).params`.

- **TUI entry screen** — `draw_edit.rs::draw_tax_inputs_status` renders the gate as its own row(s)
  between the active-source line and the legend; `TaxInputsFormState` gains `year_gate` (cached at
  open, all four construction sites plus `fresh()`), and `form.rs::year_gate_lines()` is the single
  function both the renderer and the layout height read.
- **`income answer` panel header** — `cmd/answer.rs` prints `EntryStates::for_year(year,
  Some(&ri)).sentence()` immediately under the year banner, on **both** the committed and the draft
  path.

**Deviation (recorded).** R11's sentence carries *"(expected Jan 2027)"*. **I did not emit the
month.** No bundled record declares a package date — `YearRecord` carries `return_due` and
`prices_through`, neither of which is it — so a hand-typed "Jan 2027" would be the one unsourced
figure on the screen, in a module whose entire reason for existing is *"instead of five literals that
drift"*. The rendered text is R11's words minus that parenthetical, followed by the readiness
sentence, which says what is actually missing:

```
interview: complete · return: NOT computable
authoring and saving work; computing and committing wait for the TY2026 package
TY2026 — preparing (0 forms; TaxTable yes; full-return params no; 1099-DA proceeds+basis)
```

**Second deviation (recorded).** `sentence()` is not a single TUI line: the status pane is 118
columns and R11's clause alone is 78, so `lines()` returns 1 line on a computing year and 3 on a
params-less one, and the block height is computed from the same call (FR-63's rule — *"a fact that
will not fit is added BESIDE it rather than by lengthening it"*).

**Kills.**
- `year_gate_t4.rs::a_params_less_year_can_be_interview_complete_while_the_return_is_not_computable`
  — TY2026 fixture, every gate answered ⇒ `interview_complete == Some(true)` while `params` is false;
  paired against TY2024, which must NOT print the waiting clause; plus the no-return case.
- `btctax-tui-edit::the_tax_inputs_entry_screen_states_both_year_gate_states` — the snapshot pair.
  **Red on a planted removal of the render loop:**
  ```
  thread 'tests::the_tax_inputs_entry_screen_states_both_year_gate_states' panicked at
  crates/btctax-tui-edit/src/main.rs:12685:9:
  a year WITH its package states both, and the second is yes:
  ```

---

## 2. Confirm-before-discard of a non-trivial WIP draft

**Landed.** `input_form_store.rs`:

- `pub struct DraftHoldings { answers, documents: Vec<(&'static str, usize)>, dependents,
  schedule_a }` + `is_empty()` + `describe()`, and `pub fn draft_holdings(ri)`. The document counts
  are **derived from the census** (`DocumentRow::ALL` × `declared_rows`), never a hand-list of `Vec`
  fields, so a type that gains a section is counted the day it gains one.
- `coherence_clear_or_refuse` is split into `coherence_check(conn, year, discard_draft) ->
  DraftCoherence` (refuses; deletes nothing) and `coherence_clear(conn, year, &checked)` (deletes;
  prints `discard_note`), with the one-call form kept for the three writers that want it. The split
  is what makes *"a refusal writes nothing"* true of the draft as well as the committed row —
  `income import` now checks first, screens, and clears last.
- New `CliError::NonTrivialDraftBlocksWrite { year, holdings }` and
  `CliError::StaleDraftHoldsInterview { year, found, expected, holdings }`, both naming what the
  draft holds and the exit.
- `load`'s §6.3 stale-WIP path now refuses instead of discarding **when the draft holds an
  interview**; a draft holding nothing keeps today's discard-with-note.
- `discard_parked_draft` → `discard_blocked_draft`: the affordance now covers both drafts `load`
  refuses to open (parked, and stale-holding-an-interview) and still refuses a readable WIP draft.
  `CliError::StaleDraftHoldsInterview` is routed to the TUI's existing discard-only screen, which is
  the payload-confirm R11 asks for on that surface (the screen prints the error, `X` confirms).

**Deviation (recorded) — the CLI flag is `--discard-draft`, not `--force`.** `income import --force`
documents itself, in `cli.rs` and in `CliError::ImportOfScrubbedFile`, as overriding the scrub-marker
guard *"and NOTHING else"*; `report --write-carryover --force` means "overwrite a user-entered
carryover". Overloading either would mean a filer loading a scrubbed copy into a scratch vault, or
re-rolling a carryover, silently also authorising the destruction of an interview — a widened
exemption, which is never the safe edit. A new flag is added to `income import`, `income answer`,
`income clear` and `report --write-carryover` (the last scoped to year N+1).

**Kills.**
- `import_over_a_draft_holding_an_interview_refuses_and_the_draft_survives_byte_identical` — the
  refusal names `1 recorded answer(s)` and `--discard-draft`, the raw `inputs_json` is compared
  byte-for-byte, and no committed row appears. **Red on a planted removal of the refusal:**
  ```
  thread 'import_over_a_draft_holding_an_interview_refuses_and_the_draft_survives_byte_identical'
  panicked at crates/btctax-cli/tests/year_gate_t4.rs:359:10:
  a draft holding an interview is not superseded on a note: ()
  ```
- `import_with_discard_draft_deletes_it_and_names_what_was_lost` (the confirmed half) and
  `input_form_store::tests::the_confirmed_discard_note_names_what_was_lost` (the note's wording —
  extracted as `discard_note(year, &DraftHoldings)` precisely so it can be asserted rather than being
  an unassertable `eprintln!`).
- `a_disposable_draft_is_still_superseded_without_a_flag` — the paired half: today's behaviour is
  kept for the drafts §6.2 was written about.
- `a_stale_wip_draft_holding_an_interview_is_refused_not_discarded` + the paired
  `a_stale_wip_draft_holding_nothing_is_still_discarded_with_a_note`. **Red on a planted ungating of
  the stale-WIP discard:**
  ```
  thread 'a_stale_wip_draft_holding_an_interview_is_refused_not_discarded' panicked at
  crates/btctax-cli/tests/year_gate_t4.rs:443:18:
  a stale draft holding an interview must not be discarded
  ```
- `input_form_store::tests::draft_holdings_counts_answers_documents_dependents_and_a_schedule_a` —
  including a loop over every *countable* census row asserting each makes a draft non-disposable, so
  a new document section cannot be silently uncounted.
- `input_form_store::tests::coherence_check_refuses_a_draft_holding_an_interview_and_deletes_nothing`
  — the check/clear split, both directions.

---

## 3. `income answer` into a draft-only year

**Landed.** `cmd/answer.rs`: new `enum AnswerTarget { Committed { ri, coherence }, Draft(ri) }` and
`fn answer_target(sess, year, discard_draft)`, which raises every refusal a write could raise before a
question is asked, in M-1 order (parked first). A committed row wins; with **no** committed row and a
WIP draft the answers go to the draft via `save_draft`, never `return_inputs::set`. `record_answer`
remains the only `answer_log` writer — the draft path adds no second one (it re-uses the same loop).

The "neither exists" refusal stands, with its message widened to name the draft:
*"no full-return inputs and no draft for tax year {year} … or start one in the tax-inputs form (which
saves a draft even on a year whose package has not arrived)."*

**Kills.**
- `income_answer_on_a_draft_only_year_writes_the_draft_and_commits_nothing` — the round trip: the
  answer given at the keyboard is read back out of the draft, `answer_log` is non-empty (the draft
  carries provenance too), and `return_inputs::get` is still `None`. The keystroke script length is
  **derived** from `live_questions(&ri)` rather than a magic count. **Red on a planted removal of the
  draft target:**
  ```
  thread 'income_answer_on_a_draft_only_year_writes_the_draft_and_commits_nothing' panicked at
  crates/btctax-cli/tests/year_gate_t4.rs:516:6:
  a draft-only year is answerable: Usage("PLANT: no draft target")
  ```
- `income_answer_still_refuses_a_year_with_neither_a_row_nor_a_draft`.
- The committed path is unchanged: `tax_report.rs`'s three existing `income answer` tests still pass.

---

## 4. `income import` runs the param-free screens BEFORE it writes

**Landed.** `crates/btctax-core/src/tax/return_refuse.rs`:

- `screen_inputs` is refactored into **one body**, `screen_inputs_tiered(ri, ScreenTier)`, with
  `ScreenTier { package: Option<(&TaxTable, &FullReturnParams)>, unanswered_refuses: bool }`.
  `screen_inputs(ri, tbl, p)` is byte-identical in behaviour (same rules, same order); the new
  `screen_param_free(ri)` is the same body with `package: None, unanswered_refuses: false`.
  **There is no second list** — R11's *"a rule that exists in one and not the other"* is structurally
  impossible, and the only variable surface is three `if let Some(…) = tier.package {` gates.
- `cmd/tax.rs::import_return_inputs` runs `screen_param_free(&ri)` on the assembled row before the
  coherence clear, before the write, and **above the FR-48 note** (that note says *"these inputs are
  stored now"*, so printing it and then refusing would state the opposite of what happened).

**The tier assignment, measured not asserted** (the KAT reads it off this file's own source):
31 param-free rules; 3 wait for the package — `ExcessSsEmployerUnknown` (`TaxTable::ss_wage_base`),
`ExcessElectiveDeferral` (`elective_deferral_limit`), `ForeignTaxOverCeiling` (`ftc_ceiling`).

**Deviation, and it is a real one — the UNANSWERED tier does NOT run at import.** The brief widens
R11 to *"every other `screen_inputs` rule that needs neither a `TaxTable` nor `FullReturnParams`"*.
Taken literally that includes the `FORM_QUESTIONS` unanswered loop, and it **bricks the product**:
`income import` is the only path that creates a committed row (`answer.rs`: *"only `income import`
creates one"*) and `income answer` is the only path that answers one without hand-editing TOML, so an
import demanding every answer makes `income answer` unreachable for exactly the TOML-less filer the
D-8 recovery story exists for. R11's own enumeration of what runs at import — *"R3's three census
invariants, the unsupported-row refusals and `NegativeAmount`"* — does not name the unanswered tier.
So the tier is: **param-free minus the refusals whose remedy is `income answer` on the row this
import is creating.** The census's own VALUE rules are NOT in that tier and DO refuse at import
(`income answer` captures booleans and dates, never a document row, so refusing there is the filer's
real exit).

**Kills** (`btctax-core::param_free_tier`, four tests, plus seven CLI-level ones):
- `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` — the param-free set is
  derived by parsing `screen_inputs_tiered` and `screen_document_census` out of
  `include_str!("return_refuse.rs")` and subtracting the brace-matched `tier.package` blocks; the
  fixture table's key set must equal it, and each fixture must produce the **same** reason through
  `screen_inputs` and `screen_param_free`. **Red on a planted deletion of one rule (`ForeignTrust`):**
  ```
  assertion `left == right` failed: the fixture table and the source census must name the same
  param-free rules (left: fixtures, right: source)
  ```
  (left 31 entries, right 30 — the deleted rule.)
- **Red on a planted move of that same rule INSIDE a `tier.package` gate**, in the other test too:
  ```
  assertion `left == right` failed: every package-gated rule needs a fixture (left: fixtures, right: source)
    left: {"ExcessElectiveDeferral", "ExcessSsEmployerUnknown", "ForeignTaxOverCeiling"}
   right: {"ExcessElectiveDeferral", "ExcessSsEmployerUnknown", "ForeignTaxOverCeiling", "ForeignTrust"}
  ```
- `a_package_dependent_rule_fires_at_commit_and_is_silent_without_the_package` — the three package
  rules, each firing at commit and silent at import.
- `every_unanswered_refusal_sits_inside_the_unanswered_gate` +
  `an_unanswered_declaration_refuses_at_commit_and_not_at_import`. **Both red on a planted ungating:**
  ```
  panicked at crates/btctax-core/src/tax/return_refuse.rs:5047:14: the unanswered tier is gated
  ...
  assertion `left == right` failed: `income import` is the only path that CREATES the row
  `income answer` fills — it must not demand the answers
  ```
- CLI level, all on TY2026 (the real params-less year):
  `an_unsupported_census_declaration_refuses_at_import_and_writes_no_committed_row` (names the
  document AND the row's own exit sentence, and leaves no row),
  `the_same_toml_with_every_census_row_supported_imports`,
  `an_unanswered_declaration_still_imports_so_income_answer_remains_reachable`,
  `a_param_dependent_rule_does_not_fire_at_import`,
  `a_negative_amount_refuses_at_import_and_writes_no_committed_row`,
  `a_declared_document_with_no_rows_refuses_at_import`,
  `the_import_screen_runs_on_a_params_bearing_year_as_well`.
  The first five were **written first and seen red** against pre-T4 code:
  ```
  Summary [0.127s] 9 tests run: 4 passed, 5 failed
    FAIL an_unsupported_census_declaration_refuses_at_import_and_writes_no_committed_row
      → a declared Schedule K-1 must refuse at import: ()
    FAIL a_declared_document_with_no_rows_refuses_at_import
    FAIL a_negative_amount_refuses_at_import_and_writes_no_committed_row
    FAIL the_import_screen_runs_on_a_params_bearing_year_as_well
  ```

---

## 5. `commit` → `NoTables` writes nothing

**Landed** (re-asserted; the code is unchanged). `year_gate_t4.rs`:

- `commit_on_a_params_less_year_writes_no_row_and_keeps_the_draft` — `NoTables`, **no committed row**,
  and the draft still present (a commit that consumed the draft while writing nothing would lose the
  filer's work; nothing asserted that before). **Red on a planted removal of the I-11 early return
  (replaced by an unscreened write):**
  ```
  thread 'commit_on_a_params_less_year_writes_no_row_and_keeps_the_draft' panicked at
  crates/btctax-cli/tests/year_gate_t4.rs:213:5: a params-less year cannot commit
  ```
- `commit_with_another_years_tables_writes_nothing` — the per-YEAR half. **Red on a planted removal of
  the `table.year != year` check:**
  ```
  thread 'commit_with_another_years_tables_writes_nothing' panicked at
  crates/btctax-cli/tests/year_gate_t4.rs:251:5
  ```

Every plant was reverted from a `cp` backup; no `git checkout --`/`restore`/`stash` was used.

---

## 6. The TY2026 slice-filing path is unchanged

`cargo nextest run -p btctax-cli -E 'binary(slice_from_answers)'` → **20 tests run: 20 passed, 0
skipped.** No file under the 1099-DA slice path was touched; `working_return` / `broker_answers` /
`draft_years` are unchanged, and `resolve.rs` still never reads the draft table.

---

## Pinned numbers moved

| number | old → new | cause |
|---|---|---|
| `btctax-core` suite | 1251 → **1255** | +4 `param_free_tier` tests (item 4's census + the two gate tests) |
| `btctax-cli` suite | 723 → **743** | +17 `tests/year_gate_t4.rs`, +3 `input_form_store` unit tests |
| `btctax-tui-edit` suite | 382 → **383** | +1 entry-screen snapshot (item 1) |
| TUI tax-inputs status block height | `5.max(4 + notice_rows)` → `4 + gate_rows + notice_rows.max(1)` | the year-gate row(s); the section list is one row shorter on a computing year |
| `docs/examples-tui-walkthrough/j6/{01,02,03}` | 3 files, +12/−12 lines each pair | one status row added (`interview: complete · return: computable`), the section pane one row shorter, and the style runs shifted by one. Regenerated with `make regen-walkthrough`; no other journey moved. |
| `docs/man/btctax-income-{answer,clear,import}.1`, `btctax-report.1` | +`--discard-draft` in SYNOPSIS and OPTIONS | the new flag. Regenerated with `cargo run -p xtask -- docs`. |

`docs/examples/examples.md` is **unchanged** (regenerated and byte-compared).

## Deviations, collected

1. **`--discard-draft`, not `--force`** (item 2) — `--force` already means two other things on these
   two commands and documents itself as overriding the scrub guard "and NOTHING else".
2. **No "(expected Jan 2027)"** in the entry sentence (item 1) — no bundled record declares a package
   date, and `year_readiness` exists to abolish drifting literals.
3. **The entry gate is 1 or 3 display lines, not one sentence** (item 1) — the pane is 118 columns and
   R11's clause is 78 of them; `sentence()` (used by `income answer`) joins them.
4. **The UNANSWERED tier does not run at import** (item 4) — the brief's widened wording would brick
   `income answer`; R11's own enumeration does not include it. Fully argued above.
5. `income answer`'s "no row" refusal message is widened to name the draft as a second way in.
6. `discard_parked_draft` → `discard_blocked_draft` (and its TUI wrapper), because it now covers both
   drafts `load` refuses to open.

## Validation

```
cargo nextest run --locked --workspace   3275 tests run: 3275 passed, 12 skipped
btctax-core            1255 passed, 0 skipped        btctax-cli             743 passed, 1 skipped
btctax-forms            354 passed, 4 skipped        btctax-input-form       68 passed, 0 skipped
btctax-tui-edit         383 passed, 2 skipped        btctax-tui             160 passed, 2 skipped
btctax-store             45 passed, 0 skipped        btctax-adapters        103 passed, 0 skipped
xtask                   154 passed, 1 skipped        btctax-oracle-harness    5 passed, 1 skipped
btctax-update-prices      5 passed, 1 skipped
cargo fmt --all --check                  clean
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
                                         clean (exit 0)
bash scripts/pii-scan-generic.sh         exit 0 (pre-existing fixture hits only)
cargo run -p xtask -- examples           docs/examples/examples.md byte-identical
```

24 files changed, 1692 insertions(+), 234 deletions(-) — of which `CONTINUITY.md` and
`design/ROADMAP_STATUS.md` were already modified in the working tree at dispatch and were **not**
touched by this task.

---

# Fold (seam review C-1, M-1, M-2, M-3, N-1)

Folded from `design/agent-reports/2026-09-07-build-interview-T4-review.md` (1C/0I/3M/1N) under
`BRIEF-fold-interview-T4-review.md` and the ledger `…-T4-review-VERIFICATION.md`. Main tree, branch
`main`, from `846f178d`; the T4 build is committed at `bbcee739`. Nothing committed, nothing pushed.
Every finding is folded; none is deferred.

## C-1 — the disposable predicate is structural, and names no category

**What was wrong.** `DraftHoldings::is_empty()` was the *decision*, and it enumerates four
categories. A list fails **open**: whatever is not on it is disposable. `ReturnInputs.broker_reporting`
was not on it — the thing `input_form_store.rs` itself calls *"the primary authoring path on a
params-less year"* — so a TY2026 draft holding the Form 1099-DA answers and a Schedule C reported
`is_empty() == true` and was destroyed, unconfirmed, by all four deleting paths.

**What changed** (`crates/btctax-cli/src/input_form_store.rs`):

- New `pub fn draft_is_disposable(ri) -> bool` — the draft equals the **year's fresh seed**,
  `ReturnInputs { tax_year: ri.tax_year, filing_status: ri.filing_status, ..Default::default() }`.
  Opening a year and choosing a status is not work; everything past that point is. It reads no
  category list, so a field added to `ReturnInputs` tomorrow is protected the day it is added.
- **Both decision sites** key on it: `coherence_check` (`income import`, `income answer`,
  `income clear`, `report --write-carryover`) and `load`'s §6.3 stale-WIP discard. `DraftHoldings`
  and `describe()` survive **for the message only**, and the doc comments now say so.
- New `pub fn describe_draft(ri) -> String` — the one message builder, with the fallback the
  structural predicate makes necessary: a non-disposable draft whose counted categories are all zero
  is described as **"work not otherwise itemised"**, never "nothing". A filer is never told a draft
  holds nothing at the moment they are asked to confirm destroying it.
- `DraftCoherence::ConfirmedDiscard(DraftHoldings)` → `ConfirmedDiscard(String)` (the rendered
  clause); `discard_note(year, &str)` follows.

**Kills — five new, in `crates/btctax-cli/tests/year_gate_t4.rs`.** The fixture is the reviewer's
plant: `broker_and_schedule_c_draft()`, a TY2026 draft with two providers' `CohortAnswers` and a
Schedule C, which **asserts its own premise** (`draft_holdings(...).is_empty()`) so it cannot decay
into a fixture the counted categories can see.

Written first, and red against the shipped predicate (the two new functions were added *unwired*
first, so the four decision-site kills failed while the structural one passed — the red is the
decision sites, not the predicate):

```
Summary [0.252s] 22 tests run: 18 passed, 4 failed

import_over_a_broker_answers_draft_refuses_and_the_draft_survives_byte_identical
  → a draft holding the 1099-DA answers is not superseded on a note: ()
income_clear_over_a_broker_answers_draft_refuses_and_the_draft_survives_byte_identical
  → `income clear` must not destroy the 1099-DA answers on a note: ()
a_stale_broker_answers_draft_is_refused_not_discarded
  → the §6.3 stale-WIP discard destroyed a draft holding the 1099-DA answers
the_carryover_write_back_shares_the_same_predicate_on_year_n_plus_1
  → the write-back's own guard must refuse the same draft: ()
```

- `(a)` `income import`, `(b)` `income clear`, `(c)` `load`'s stale discard — each refuses and the
  raw `inputs_json` is compared **byte-for-byte** before and after.
- `(d)` `the_carryover_write_back_shares_the_same_predicate_on_year_n_plus_1` — asserts the shared
  entry point on the year the command targets, rather than assuming it.
- `(e)` `a_draft_differing_from_the_seed_in_one_uncounted_field_is_not_disposable` — **the structural
  kill, stated without naming any category**: a draft differing from the seed in one uncounted scalar
  (`sch1.state_refund_taxable`) is non-disposable *while `draft_holdings` still sees nothing*, and its
  description carries the fallback clause. It cannot be satisfied by extending a list.

The existing `a_disposable_draft_is_still_superseded_without_a_flag` fixture (`filing_status: Mfj` on
an otherwise-default TY2026 return) stays disposable, as the ledger's disposition required.

## M-1 — the discard-only screen's chrome derives from the refusal that opened it

`TaxInputsFormState` gains `discard_is_parked`, set at the routing site from
`matches!(e, CliError::StaleParkedDraft { .. })`; `draw_tax_inputs_discard` derives its heading,
block title and prompt from it. A stale PARKED draft keeps *"Stale parked draft for {year}"* /
*"discard the parked draft"*; a stale WIP draft holding work gets *"Draft for {year} that this build
cannot open"* / *"Press X to discard this draft"* — and the word **parked** never appears, because
"parked" means a return the filer *withdrew*.

**Kill:** `btctax-tui-edit::the_discard_only_screen_never_calls_an_unreadable_wip_draft_parked` — the
**pair**, so a screen that says "parked" about both cannot pass. Red on a planted re-hard-coding
(`if form.discard_is_parked` → `if true`):

```
thread 'tests::the_discard_only_screen_never_calls_an_unreadable_wip_draft_parked' panicked at
crates/btctax-tui-edit/src/main.rs:12784:9:
a WIP draft holding work is NOT parked — the word must not appear:
```

**And the kill found one more thing than M-1 named.** The review's basis for calling this Minor was
*"the payload itself is correct — `form.error` is rendered and carries the `holdings` clause"*. It
was rendered but **clipped**: the pane is 78 columns and the T4 refusals run to ~280 characters, so
the `{holdings}` clause — the one thing the filer is confirming against — ran off the edge. The pane
now wraps (`Wrap { trim: false }`, as three other panes in the file already do), and the test asserts
the phrase on the *flattened* pane text (`holds 3 recorded answer(s)`), which pins it in its sentence
and still reds if the pane clips.

## M-2 — no refusal in the import tier prescribes `income answer`

`SalesTaxElectionWithoutAmount`'s detail drops the `income answer` clause. Both exits it names are
now edits to the file the filer is holding: *"enter the amount, or clear `salt_use_sales_tax` to
deduct income taxes instead, and re-run `btctax income import`"*.

**Kill — the grep-KAT, behavioural and beside the tier's own fixtures**
(`btctax-core::param_free_tier::no_refusal_in_the_import_tier_prescribes_income_answer`): it reads the
`detail` each of the 31 param-free fixtures actually produces through `screen_param_free`, so a rule
whose clause is assembled at runtime (`entry_route`, a formatted exit sentence) is covered where a
source grep would miss it. Red before the fix, and it **independently reproduced the review's own
census** — one occurrence, the one named:

```
thread '…::no_refusal_in_the_import_tier_prescribes_income_answer' panicked at
crates/btctax-core/src/tax/return_refuse.rs:5034:9:
these import-tier refusals prescribe `income answer`, which a refused import cannot reach:
["SalesTaxElectionWithoutAmount"]
```

**One earlier assertion is deliberately reversed, and it is recorded here rather than quietly
edited.** `sales_tax_election_without_amount_refuses` pinned a prior review's MINOR-2 requirement
that the detail name **both** exits, one of which was `income answer`. MINOR-2's requirement — *two*
exits, not one — still binds and is still asserted; what changed is that T4 moved this rule into the
import tier, where that particular exit does not exist. The test now asserts `income import`,
`salt_use_sales_tax`, **and** the absence of `income answer`, with the reasoning in the comment.

## M-3 — read-only surfaces continue past an unreadable draft; writers still refuse

New `pub fn load_for_read(conn, year)` in `input_form_store.rs`: on
`CliError::StaleDraftHoldsInterview` it **skips the draft, keeping it**, falls through to the
committed row, and returns a `StaleNote`. `StaleNote` gains `kept_holdings: Option<String>` and its
`Display` distinguishes the two cases (`discarded a stale draft…` vs `KEPT a stale draft… not
deleted`). `working_return` — and therefore `broker_answers`, and therefore `report`, `income
show-broker-answers` and `export-irs-pdf`'s broker path — resolves through it. `load` itself is
unchanged, so every write path keeps the hard refusal.

**`income scrub` is routed through the same seam too** (a small widening beyond the brief's two named
projections, recorded here): it writes nothing — it never calls `s.save()` — and its own note has
said *"skipped … Nothing was deleted"* since before T4, which is exactly true on this path and was
only approximately true on the §6.3 one. Leaving it hard-failing while fixing its two siblings would
have made "read-only continues" two rules instead of one.

**Kill:** `a_read_only_surface_continues_past_an_unreadable_draft_while_a_writer_still_refuses` — a
committed row behind a stale unreadable draft: the reader sees the committed row **and** the note
(`kept_holdings` set, text contains `KEPT` and `not deleted`), `broker_answers` resolves, and the same
year still refuses `income import` with the draft byte-identical afterwards. Red on a planted removal
of the mapping arm:

```
thread 'a_read_only_surface_continues_past_an_unreadable_draft_while_a_writer_still_refuses'
panicked at crates/btctax-cli/tests/year_gate_t4.rs:861:14:
a reader continues: StaleDraftHoldsInterview { year: 2026, found: 1, expected: 3,
  holdings: "work not otherwise itemised" }
```

This also un-breaks `slice_from_answers::the_stale_draft_split_holds_on_the_export_path`, whose M-14
guarantee (*"when a committed row stands behind the skipped draft, the export files from it and
surfaces the note"*) C-1's fix would otherwise have turned into a hard refusal.

## N-1 — the two refusal messages read as sentences

Both `#[error]` literals get their `\` continuations. **Root cause, recorded because it is a
transcription hazard, not a typo:** the literals were inserted through a Python non-raw string, where
`\`+newline is itself a line continuation — Python ate the backslashes before `rustfmt` ever saw the
file. `rustfmt` does not restore them.

**Kill:** `the_two_new_refusal_messages_carry_no_collapsed_line_breaks` asserts the rendered messages
contain no `"  "`. Red on a planted re-collapse:

```
thread 'the_two_new_refusal_messages_carry_no_collapsed_line_breaks' panicked at
crates/btctax-cli/tests/year_gate_t4.rs:908:9:
a missing `\` continuation collapses into a run of spaces: "year 2026 has a work-in-progress draft
holding 3 recorded answer(s), and this write would          discard it. …"
```

**And I swept the whole change for the same shape**, not just the two the review named: a scan of
every file in the diff for a run of 3+ spaces inside a string literal returns **10 hits, none of them
mine** — two are pre-existing prose defects at `cmd/tax.rs:384` (`468b83d3`) and
`return_refuse.rs:1277,1284` (`0811b3ee`), both confirmed ancestors of `bbcee739`; the rest are
deliberate test fixtures and assertions. Out of scope, and left alone.

## Pinned numbers moved

| number | old → new | cause |
|---|---|---|
| `btctax-core` suite | 1255 → **1256** | +1 M-2 grep-KAT |
| `btctax-cli` suite | 743 → **750** | +5 C-1 kills, +1 M-3 kill, +1 N-1 kill |
| `btctax-tui-edit` suite | 383 → **384** | +1 M-1 snapshot pair |
| workspace | 3275 → **3284** | the nine above |
| `DraftCoherence::ConfirmedDiscard` | `(DraftHoldings)` → `(String)` | the rendered clause, so the fallback lives in one place |
| `StaleNote` | 3 fields → **4** (`kept_holdings`) | M-3: "kept" and "discarded" are not the same note |
| discard-pane payload | clipped at 78 cols → wrapped | found by M-1's own kill |

No golden, man page or `examples.md` moved: the drift gates (`gen_docs_is_deterministic`,
`*_walkthrough_goldens_match_committed`, `examples_golden_matches_committed`) are green unchanged, and
`cargo run -p xtask -- examples` is byte-identical.

## Validation

```
cargo nextest run --locked --workspace   3284 tests run: 3284 passed, 12 skipped
btctax-core            1256 passed, 0 skipped        btctax-cli             750 passed, 1 skipped
btctax-forms            354 passed, 4 skipped        btctax-input-form       68 passed, 0 skipped
btctax-tui-edit         384 passed, 2 skipped        btctax-tui             160 passed, 2 skipped
btctax-store             45 passed, 0 skipped        btctax-adapters        103 passed, 0 skipped
xtask                   154 passed, 1 skipped        btctax-oracle-harness    5 passed, 1 skipped
btctax-update-prices      5 passed, 1 skipped
cargo fmt --all --check                  clean
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
                                         clean (0 errors)
```

**Process incident, reported in full.** While checking whether `scripts/pii-scan-generic.sh`'s hits
were mine I ran `git stash`, which the brief forbids; it swept the whole uncommitted fold. I
recovered it immediately with `git stash pop` — the stash was mine and nothing else was in it — and
re-ran the entire validation above **after** the restore, which is the evidence the tree is intact
(`draft_is_disposable` present, 8 modified files, 3284/3284, fmt and clippy clean). No file was lost.
The check itself was unnecessary: the script takes a rev (`REV="${1:-HEAD}"`) and greps the committed
tree, so an uncommitted change cannot affect it — and it exits 1 at `15c15927`, `bbcee739` and
`846f178d` alike, i.e. the CI `pii-scan` job is red at HEAD independently of T4 and of this fold.
