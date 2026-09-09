# Re-verification — interview T12 build + fold (sonnet, own worktree, every kill replanted)

Worktree `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a6cf0a5caf52613cc` at `cf319fc4` (the
T12 fold, the last commit of the interview arc). `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`
for every command below. Every plant was made in this worktree from a `cp` backup and reverted with
`cp` back (not `git checkout --`, except three files reverted with `git checkout --` per-file where
noted — this worktree only, no shared tree), followed by `find crates -name '*.rs' -exec touch {} +`
after **both** the plant and the restore (FR-90). No commits, no pushes, no subagents.

Baseline confirmed before any plant, and again after the last revert: scoped run across
`btctax-tui-edit`, `btctax-cli`, `btctax-core`, `xtask`, `btctax-forms` reproduces **exactly** the
documented environment-failure set — six `form_delta::*` tests and
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` — and nothing
else. `git status --short` is empty at the end.

---

## Part 1 — the fold's own kills (review findings C-1, I-1, I-2, M-1, M-2, N-3)

| Finding / kill | Plant made | Command | RED text | Verdict |
|---|---|---|---|---|
| **C-1** — `the_filer_can_scroll_the_answer_panel_to_its_last_line_through_the_key_handler` | Restored the handler's logical-line clamp (`main.rs`, `panel_open` block) | `cargo nextest run -p btctax-tui-edit -E 'test(the_filer_can_scroll…)'` | `PgDn must reach the LAST panel line … wanted: (0 answered, 51 not applicable…) screen: … showing 39–75 of 141 lines …` | **RED, exact match.** Kill body read (source): presses `p` then 40×`PageDown` through `handle_key`, reads drawn frames — **no assignment to `panel_scroll` anywhere in the test.** Confirmed. |
| **I-1a** — `the_answer_panel_lists_every_refusal_the_value_tier_of_the_screen_raises` | Replaced `interview_state_with`'s `screen_param_free(ri)` call with `None::<Refusal>` | `cargo nextest run -p btctax-core -E 'test(the_answer_panel_lists_every_refusal…)'` | 54-reason list incl. `"HomeSaleNotComputed"` | **RED, exact match** to review/fold. |
| **I-1b** — `the_panel_does_not_claim_nothing_refuses_when_the_commit_screen_refuses` | (same plant) | `cargo nextest run -p btctax-cli -E 'test(the_panel_does_not_claim_nothing_refuses…)'` | `the commit gate refuses this return (HomeSaleNotComputed…) and the panel told the filer nothing refuses: […, "(39 answered, 47 not applicable to this return)"]` | **RED, byte-identical** to the fold report's quote. |
| **I-2a** — `every_document_family_that_carries_a_transcription_date_is_named` | `document_row_facts`: `Form1098 => TranscribedOn::NoColumn` | `cargo nextest run -p btctax-core -E 'test(every_document_family…)'` | `every family but the W-2 carries a transcription date today: […, missing Form 1098]` | **RED**, matches. |
| **I-2b** — `the_transcription_date_columns_in_the_source_are_all_walked` | (same plant, then plant B below) | `cargo nextest run -p btctax-core -E 'test(the_transcription_date_columns…)'` | Plant A: `left: 7 right: 8`. Plant B (W-2 gains `transcribed_on`, all 4 compiler errors at `classifier.rs`, `return_refuse.rs`, `scrub.rs` ×2 patterns, `scrub_axis.rs` fixed): `left: 8 right: 9`, compiler fully satisfied, `every_document_family…` **passes** (its blind half) while this one still reds | **RED both plants, exact match**, incl. the "compiler satisfied, check still catches it" case the fold specifically claims. Compile errors reproduced **verbatim**: `classifier.rs:732`, `return_refuse.rs:1086`, `scrub.rs:1021` (3×E0027), `scrub_axis.rs:222` (E0063). |
| **M-1** — `the_manifest_lists_the_forgone_benefits_marking_the_declined_ones_and_names_the_undated_rows` (extended) | `forgoing_block`: reverted `not_computed` rendering to `for n in &st.not_computed { push(&mut s, &n.line()); }` (no heading) | `cargo nextest run -p btctax-cli -E 'test(the_manifest_lists_the_forgone_benefits…)'` | `the manifest must carry the NOT COMPUTED block the panel renders — heading and instruction included. wanted: NOT COMPUTED (1) — btctax does not file the schedule these are figured on…` | **RED, exact match.** |
| **M-2** — `the_answer_panel_pane_keeps_its_close_legend_at_every_terminal_width` | Reverted pane footer to one unwrapped `Line`, no `wrap_to` | `cargo nextest run -p btctax-tui-edit -E 'test(the_answer_panel_pane_keeps_its_close_legend…)'` | `at 72×16 the only printed way OUT of a full-screen overlay was cut off: …` | **RED**, same size (72×16) the review measured. |
| **N-3** — `every_file_that_renders_the_panel_is_in_the_progress_widget_checks_field_of_view` | `renderer_sources()` narrowed to `draw_edit.rs` alone | `cargo nextest run -p xtask -E 'test(every_file_that_renders_the_panel…)'` | `these files render the answer panel and the progress-widget check cannot see them, so a hand-rolled bar there is caught by nothing: ["crates/btctax-cli/src/cmd/admin.rs", "crates/btctax-cli/src/cmd/answer.rs", "crates/btctax-cli/src/render.rs", "crates/btctax-tui-edit/src/edit/form.rs", "crates/btctax-tui-edit/src/main.rs"]` | **RED, byte-identical** to the fold report's quote. |
| N-1 (nit, doc-only) | — read only | `grep -c "T12 — A PAYLOAD TALLER" draw_edit.rs` | 1 occurrence (was 2) | **Confirmed fixed**, no test expected. |
| N-2 (nit, doc-only) | — read only | `grep "four checks\|Four checks" r15_stop_list.rs` | "Four checks" language present at 3 sites, incl. the module header | **Confirmed fixed**, no test expected. |

Every plant reverted; each test re-ran green after restore (spot-checked per row above).

---

## Part 2 — the two stated boundaries

**I-1's boundary** ("cannot run the five package-gated rules"): confirmed true and honestly stated,
not narrower than reality.
- All five (`ExcessSsEmployerUnknown` `return_refuse.rs:3141`, `ExcessElectiveDeferral` `:3237`,
  `ForeignTaxOverCeiling` `:3536`, `HsaExcessContributionsNeedForm5329` `:3669`,
  `HsaExcessEmployerContributions` `:3692`) are gated behind `if let Some((tbl,_))` /
  `if let Some((_,p))) = tier.package` — read directly, all five, at their line numbers.
  `screen_param_free` calls `screen_inputs_tiered` with `package: None`, so none of the five can ever
  fire at that tier — mechanically impossible, not merely untested.
- The qualifier sentence rides in the **same code path** as the claim
  (`answer.rs::panel_lines`, `out.push` immediately follows the zero-open sentence, both inside the
  `if st.open_items() == 0` block) — cannot be edited away independently.
- `LIMITATIONS.md`'s REFUSING bullet states the identical mechanism ("a Social Security wage base, an
  elective-deferral cap, the no-Form-1116 foreign-tax ceiling, an HSA contribution limit … need the
  year's published figures, which the panel does not hold").
- **Verdict: honest, matches the tree exactly.**

**I-2's boundary** ("totality at the function, not the packet fixture; the surface test still
populates `int_1099` and only `int_1099`"): confirmed true.
- `grep transcribed_on crates/btctax-cli/tests/export_irs_pdf.rs` shows only `int_1099`'s
  `transcribed_on: dated` (the parameterized fixture) and one `transcribed_on: None` at line 3014 —
  no `form_1098`, `sa_1099`, or `sa_5498` row is ever given a `transcribed_on` in that test file.
- **Verdict: honest, matches the tree exactly.**

---

## Part 3 — the eight deviations

| # | Claim | Check | Verdict |
|---|---|---|---|
| 1 | `draw_tax_inputs_form`/`panel`/`modal` take `&mut TaxInputsFormState`; the handler's leftover clamp was C-1 | Read `main.rs`/`draw_edit.rs` signatures and the C-1 fix | **Confirmed** — see Part 1 |
| 2 | Crate-root re-export (`lib.rs`) required by KAT-G1 | `lib.rs:62` `pub use cmd::answer::{forgoing_lines, not_computed_lines, panel_lines, refusing_lines};` | **Confirmed present** |
| 3 | `home_sale_decision` extracted, one decider two readers | `grep home_sale_decision`: called at `return_refuse.rs:3517` (`screen_inputs_tiered`) and `render.rs:3192` (§4.4 block) | **Confirmed**, exactly two production call sites |
| 4 | Commit modal scrolls, its `Wrap` removed | `main.rs:1261-1265` `m.scroll` arms (move only); `draw_tax_inputs_modal` (`draw_edit.rs:2247-2383`) has no `.wrap(Wrap{...})` on its `Paragraph::new(lines).block(...)` — confirmed by line-range check against the two other `Wrap`-bearing `Paragraph::new` calls, which sit in the two *other* modal functions (`:2384+`, `:2418+`) | **Confirmed** |
| 5 | R15 gained a 4th check (`progress_widgets`) rather than a wider field walk | N-3 replant above; `r15_stop_list.rs` header | **Confirmed** |
| 6 | `leaf_walk`'s "nothing production reads it" sentence amended | `provenance.rs:504-511`, names `collected_figures` as the reader and the 30.5 ms measurement | **Confirmed** |
| 7 | `FOLLOWUPS.md` FR-98 edited to record N-3's widening | `FOLLOWUPS.md:6649` "★ Widened 2026-09-07 (T12 fold, seam review N-3)" | **Confirmed** |
| 8 | Not done: an undated Form 1098/1099-SA/5498-SA row on the packet-manifest fixture | See I-2 boundary above | **Confirmed true, honestly stated** |

---

## Part 4 — the T12 build report's own kills (checklist part 1)

All plants below made in production code, one at a time, reverted with `cp` (three via
`git checkout --` on this worktree's own copy, noted), `touch`ed after both plant and restore.

| # | Test | Plant | RED text (or PASS confirmed by design) | Verdict |
|---|---|---|---|---|
| 1 | `the_answer_panel_pane_draws_every_line_it_renders_and_names_the_blocking_items` | `answer.rs::panel_lines` blocking loop → `.iter().skip(1)` | `a blocking item was not rendered: Can someone claim YOU as a dependent…` | **RED, exact match** |
| 2 | (same test) | `draw_edit.rs` renderer clamp → clamp by `form.panel_lines().len()` (logical) instead of wrapped `total` | `the LAST panel line must be reachable by scrolling. wanted: (0 answered, 51 not applicable…)` | **RED**, same shape as reported |
| 3 | `the_pane_marks_a_declined_benefit_declined_and_never_lists_it_as_blocking` | `forgoing_lines`: `let mark = "";` | `assertion left==right failed: the declined benefit is listed once, marked` | **RED, exact match** |
| 4 | `on_a_params_less_year_the_pane_sizes_nothing_and_lists_the_waiting_gate` | `form.rs::panel_lines` hardcoded year 2024 instead of `self.year` | `a params-quoting gate WAITS rather than blocks: …` | **RED, exact match** |
| 5 | `the_commit_modal_prints_the_forgoing_and_refusing_lists_with_the_exit` | `commit_summary_with_step0`: panel lists dropped | `the refusing heading is DRAWN, not merely built: …` | **RED, exact match** |
| 6 | `the_commit_modal_keeps_its_legend_on_screen_and_reaches_its_last_line` | Re-added `.wrap(Wrap{trim:false})` to the modal's `Paragraph` | `the modal that WRITES THE VAULT must always show how to confirm and how to cancel: …` | **RED, exact match** |
| 7 | `a_return_the_panel_lists_as_refusing_is_refused_by_the_commit_screen_too` | Census §2.2 rule away (`if let Some(exit) = row.exit_sentence()` → `None::<&str>`) | `…and the EXISTING commit gate refuses it — the panel adds no gate, it shows one` | **RED**, but the message differs from the build report's table quote for this row, which instead shows the home-sale-table's message (below). Genuine kill either way — see Minor M-R1 |
| 7′ | `the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523` (FR-87's kill, same plant) | (same plant) | `(true,true,true,s_1099=Some(true)) refused unexpectedly: None` | **RED, exact match** |
| 8 | `the_dependents_pane_and_the_panel_word_the_waiting_gate_identically` | `dependent_gates.rs` fallback prompt reverted to a question | `…and it must read as a wait rather than as something to answer: Dependents #1 …` | **RED, exact match** |
| 22 | `a_params_quoting_gates_fallback_is_a_waiting_label_and_its_rendered_prompt_is_the_question` (same plant) | (same plant) | `the figureless fallback must not read as a question a filer can answer — it is the label of a WAITING item: …` | **RED, exact match** |
| 9 | `p_opens_the_answer_panel_from_the_entry_and_from_a_section_and_esc_closes_the_pane_only` | `main.rs`: `p` wired to `Char('P')` | `` `p` opens the pane at the entry screen too — §4.1 says it is visible from every section`` | **RED, exact match** |
| 10 | (same test) | `if form.panel_open && key.code != KeyCode::Esc` (Esc no longer swallowed) | `Esc inside a read-only overlay must not drop the filer out of the editor` | **RED, exact match** |
| 11 | `the_report_block_prints_the_census_and_masks_every_payer_tin` | `document_identity`: return raw `f.tin` unmasked | Test failed with unmasked `"12-3456789"` printed in the block | **RED, exact match** |
| 21 | `a_documents_payer_tin_is_masked_in_every_collected_figure` (same plant) | (same plant) | 65-entry list, every TIN unmasked | **RED, exact match** |
| 12 | `the_report_block_prints_the_census_and_masks_every_payer_tin` | `render.rs` census loop → `DocumentRow::ALL.iter().skip(1)` | Test failed (missing Form W-2 row) | **RED, exact match** |
| 13 | (same test) | Dropped the undated-rows section from `render_interview_block` | `an undated row is NAMED: …` | **RED, exact match** |
| 14 | (same test) | Dropped the panel loop from `render_interview_block` | `the panel is in the block: …` | **RED, exact match** |
| 15 | `the_report_block_states_the_home_sale_decision_and_the_answers_behind_it` | Hardcoded `HomeSaleDecision::NoSale` | Assertion failure (decision text absent) | **RED, exact match** |
| 16 | `the_report_block_prints_row_7_for_every_dependent_from_the_flowchart` | `col` hardcoded to `CreditColumn::Neither` | `row (7) is printed per dependent: …` | **RED, exact match** |
| 17 | `report_tax_year_prints_the_interview_block_after_the_existing_chains` | `cmd/tax.rs`: block computed, never `push_str`'d | `the §4.4 block reaches the command's output: …` | **RED, exact match** |
| 18 | `the_manifest_lists_the_forgone_benefits_marking_the_declined_ones_and_names_the_undated_rows` | `admin.rs`: `forgoing_block(...)` never appended | `the manifest carries the forgone block: …` | **RED, exact match** |
| 19 | (same test) | `forgoing_lines`: `let mark = "";` | `…and MARKS the benefit the filer was asked about and passed over, ON ITS OWN ROW…` | **RED, exact match** |
| 20 | `every_collected_money_leaf_is_listed_with_its_source` | `collected_figures`: `Source::FilerRecords` dropped from the filter | `assertion left==right failed: the listed set IS the derived set` (left/right sets differ by exactly the FilerRecords paths) | **RED, exact match** |
| 23 | `limitations_names_every_excluded_document_family_the_census_refuses` | *(no plant — build report says it reds on the committed doc without one; not replantable meaningfully)* | ran clean at HEAD | **PASS, as expected; not a red-on-plant kill by design** |
| 24 | `limitations_carries_the_venue_account_granularity_note` | `step0.rs`: `VENUE_GRANULARITY_NOTE` rewritten, dropping both load-bearing tokens | `the shipped constant must still carry "exchange:<venue>:default" — if it does not, this test is checking the wrong thing: …` | **RED, confirms it asserts the SOURCE before the doc, as claimed** |
| 25 | `the_progress_widget_check_reds_on_a_gauge_and_not_on_its_near_misses` | *(self-contained — plants are inline fabricated source strings passed to `progress_widgets`)* | ran green (all 3 plants → finding, all 3 near-misses → no finding) | **Confirmed discriminates, as designed** |
| 26a | `only_the_two_credit_arms_check_a_box` | `credit_column`: `NoCreditBox` → `CreditForOtherDependents` | `assertion left==right failed: NoCreditBox / left: CreditForOtherDependents / right: Neither` | **RED, byte-identical** |
| 26b | `single_and_mfj_ask_no_hoh_or_qss_question` | `HohQualifyingPerson.live` → `\|_ri\| true` | `HohQualifyingPerson must not be live on Single — a filing-status test asked of a filer who did not claim that status is a question with no answer` | **RED, byte-identical** |
| 26c | `a_hoh_test_answered_no_refuses_with_the_exit` | Dropped "CHOOSE ANOTHER FILING STATUS…" from the refusal detail | `a refusal with no exit is a brick: …` | **RED, byte-identical** |
| 26d | `the_qss_window_is_derived_from_the_tax_year` | `qss_window_prompt`: `a=y-2,b=y-1` → literal `2023,2024` | `TY2026: Qualifying surviving spouse, condition 1: "Your spouse died in 2023 or 2024 and you didn't remarry before the end of 2026."` | **RED, byte-identical** |
| 26e | `the_ty2025_grid_checks_the_more_than_four_box_with_its_statement` | `push_dependents_grid`: `check(…, !overflow.is_empty())` → `check(…, false)` | `assertion left==right failed / left: None / right: Some("1")` | **RED, byte-identical** |
| 26f | `every_slot_caption_is_the_forms_own_words` | `SLOT_CAPTIONS`: `"And in the U.S."` → `"And in the United States"` | `LivedWithYouInUs's caption is not in design/forms/extract/f1040--2025.txt: "And in the United States"` | **RED, byte-identical** |

All 26 line items (including both FR-73 kills verified in Part 2/Part 1, all 6 FR-86 kills, and
FR-87's kill) reproduce a red on their stated plant. **Every kill claimed by the T12 build report
and fold report actually reds.**

---

## Part 5 — negative claims

| Claim | Check | Result |
|---|---|---|
| `make check` / `make docs` / the five instruments | Already machine-verified by the controller (brief's settled facts) | **Not re-measured, per brief** |
| Walkthrough goldens moved only where a screen changed | `git diff 5743ecd6..833ce3f1 -- docs/examples-tui-walkthrough/j6/0{1,2,3}-*.txt` | `01` and `02`: identical single change each — row 37 glyph line + one style-run width (`0..92`→`0..111`), matching the report's cause (`[p] answer panel` legend). `03`: 108 insertions/76 deletions, consistent with the reported cause (taller modal, FORGOING list, scroll footer) | **Confirmed** |
| No render code calls `save_draft`/`return_inputs::set`/`record_answer` | `git diff 5743ecd6..cf319fc4` over every changed `.rs` file, `grep -E "^\+.*(save_draft\|return_inputs::set\|record_answer)"` | 3 hits total, in `export_irs_pdf.rs` (test), `tax_report.rs` (test), and `draw_edit.rs` — the last one is `the_pane_marks_a_declined_benefit_declined_and_never_lists_it_as_blocking`, confirmed inside the file's `#[cfg(test)] mod tests` (last `#[cfg(test)]` at line 6469, hit at line 6946) | **Confirmed — every hit is test-only** |
| `LIMITATIONS.md` carries the interview stop list incl. FR-73's venue note | `grep "The interview —" and "exchange:<venue>:default"` | Both present (`:210`, `:284`) | **Confirmed** |

---

## Verdicts summary

- **C-1**: holds. Handler clamp deleted; kill enters through `handle_key`, never assigns
  `panel_scroll`; planting the clamp back reproduces the exact `showing 39–75 of 141 lines` red.
- **I-1**: holds, both kills; the value-tier boundary (5 package-gated rules) is real and honestly
  stated, and the qualifier is structurally bound to the claim.
- **I-2**: holds, both directions (new-variant compile error AND existing-family-gains-a-column, the
  latter surviving a full realistic 4-error fix); the function-vs-fixture boundary is real and
  honestly stated.
- **M-1, M-2, N-3**: all hold.
- **N-1, N-2**: confirmed fixed by inspection (doc-only, no test expected).
- **All 8 deviations**: verified true against the source, none overstated.
- **All 26+ build-report kill rows** (including all 6 FR-86 kills and FR-87's kill): reproduce red on
  their stated plant.

### One Minor finding

**M-R1 (Minor, doc-only mismatch)** — build report row **#7**
(`a_return_the_panel_lists_as_refusing_is_refused_by_the_commit_screen_too`) quotes the observed red
as `(true,true,true,s_1099=Some(true)) refused unexpectedly: None`, which is actually the message
produced by a **different** test (`the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523`,
FR-87's kill) sharing the same plant. Row #7's own test reds with a different message:
`…and the EXISTING commit gate refuses it — the panel adds no gate, it shows one`. Both tests
genuinely red under the stated plant — the kill claim itself is true — but the quoted text in the
report's table is transcribed from the wrong test. No functional consequence; the underlying
guarantee (removing the census's §2.2 rule breaks both the K1-refusal panel/gate agreement and the
home-sale table) is real and independently confirmed for both tests above.

No other discrepancies found. No Critical, no other Important.

---

Counts: C=0 I=0 M=1 N=0
