# Re-verification — interview T6 build + fold, every kill re-planted

Independent re-verifier, sonnet, own worktree at `8767b937` (fold `71b91a26`, build `a79492ee`
byte-identical to `crates/` at HEAD). No commits, no subagents. Every plant below was made with the
`Edit` tool directly on the tracked file, run with a scoped `cargo nextest run --locked -p <crate>
-E 'test(<name>)'`, and reverted with `git checkout -- <file>` before the next plant; `git status
--short` was empty before the first plant and after the last. `CARGO_TARGET_DIR=/scratch/code/
bitcoin_tax/target-review` for every cargo command.

Brief: `design/agent-reports/BRIEF-reverify-interview-T6.md`. Checklist source:
`design/agent-reports/2026-09-07-build-interview-T6-implementation.md` (13 build kills P1–P13, 10
fold kills F1–F10). Review: `2026-09-07-build-interview-T6-review.md` (1C/1I/4M/4N). Ledger:
`…-review-VERIFICATION.md`. Fold brief: `BRIEF-fold-interview-T6-review.md`.

## Build kills (P1–P13)

| kill | plant | command | RED text | verdict |
|---|---|---|---|---|
| P1 — delete DA cross-check from `screen_compute_dependent` | `return_1040.rs`: replaced `if let Some(r) = screen_digital_asset_answer(..) { return Some(r); }` with `if false { .. }` | `-p btctax-core -E 'test(the_digital_asset_answer_table_holds_in_all_five_cells)'` | `a \`No\` the ledger contradicts must refuse` (kat_digital_asset_question.rs:270) | RED, matches report verbatim |
| P2 — mirror refuses (symmetric refusal R9 forbids) | `screen_digital_asset_answer`: added an unconditional refuse on `Some(true)` + no event | same test | `★★★ an unwitnessed \`Yes\` may NEVER refuse: …` `left: Some((DigitalAssetAnswerContradictsLedger {…}))  right: None` | RED, matches report verbatim |
| P3 — printed box back to the ledger predicate | `return_1040.rs` (`assemble_absolute`): `digital_asset_answer: ri.digital_asset_activity` → `if digital_asset_activity(state, year) { Some(true) } else { None }` | same test | `a filer with no receipts and no disposals must be able to PRINT "No"`  `left: None  right: Some(false)` | RED, matches report verbatim |
| P4 — delete the off-ledger warning | `advisories.rs`: `if crate::tax::return_1040::digital_asset_yes_is_off_ledger(..)` → `if false && ..` | same test | `an unwitnessed \`Yes\` is accepted WITH the off-ledger warning` | RED, matches report verbatim |
| P5 — emitter back to "Yes or nothing" | `form1040_full.rs`: `if let Some(answer) = lines.digital_asset_answer` → `if lines.digital_asset_answer == Some(true) { let answer = true; ..}` | `-p btctax-forms -E 'test(the_1040_digital_asset_box_prints_the_filers_answer_including_no)'` | `★★★ a \`No\` answer checks the NO box. …`  `left: None  right: Some("2")` | RED, matches report verbatim |
| P6 — standing-order row never fires | `step0.rs`: `if btctax_core::standing_order_in_force(..).is_none()` → `if false && ..` | `-p btctax-cli -E 'test(the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_on_or_before_the_sale)'` | `one custodial venue with no standing order: []  left: 0  right: 1` | RED, matches report verbatim |
| P7 — standing-order row always fires | same call site → `if true` unconditionally | same test | `a standing order recorded the day BEFORE the sale IS the §4.02(2) identification — warning here would contradict the engine's own \`StandingOrder\` verdict: […]` | RED, matches report verbatim |
| P8 — a venue with rows and no answer silently dropped | `step0.rs`: gutted the `for (provider, cohorts) in &unanswered { panel.venues.push(..) }` loop body to a no-op | `-p btctax-cli -E 'test(a_venue_with_rows_and_no_broker_answer_is_named_and_an_answered_one_is_not)'` | `assertion \`left == right\` failed: []  left: 0  right: 2` | RED, matches report verbatim |
| P9 — RENDERED prompts stop being scanned | `r15_stop_list.rs`: `registry_prompts()` — dropped the `.extend(RENDERED_PROMPTS…)` block | `-p xtask -E 'test(stop_list)'` | `0 of 4 RENDERED prompts were scanned — the words a filer is SHOWN are the ones this check exists to read, …` | RED, matches report verbatim |
| P10 — banned word planted inside a RENDERED prompt | `questions.rs::digital_asset_prompt` — inserted the word "lot" into the rendered sentence | same command | `R15/R9: a return-registry prompt asks a LEDGER question …:  RENDERED_PROMPTS DigitalAssetActivity: says "lot"` | RED, matches report verbatim |
| P11 — commit modal sizes itself from unwrapped lines again | `draw_edit.rs`: replaced the `wrap_to` loop with raw `m.summary.lines()` pushed unwrapped | `-p btctax-tui-edit -E 'test(the_commit_modal_shows_the_whole_venue_listing_including_its_last_line)'` | `★★★ the LAST line of the listing must be drawn — a box sized from unwrapped lines clips exactly this: …` (SENTINEL_TAIL absent) | RED, matches report verbatim |
| P12 — Step 0 lines stop being wrapped for the pane | `step0.rs::tui_lines`: `out.extend(wrap(&format!("• {}", row.what), "      "))` → `out.push(format!("• {}", row.what))` | `-p btctax-tui-edit -E 'test(step0_is_rendered_on_the_entry_screen_and_no_line_is_clipped)'` | `a Step 0 line was CLIPPED by the pane — the panel said it and the filer cannot read it. …` | RED, matches report verbatim |
| P13 — `income answer` stops printing Step 0 | `cmd/answer.rs`: dropped the `crate::step0::write_step0(out, &panel)?;` call | `-p btctax-cli -E 'test(income_answer_prints_step_0_before_the_first_census_question)'` | `\`income answer\` prints the Step 0 ledger panel` | RED, matches report verbatim |

## Fold kills (F1–F10)

| kill | plant | command | RED text | verdict |
|---|---|---|---|---|
| F1 — emitter's box back to "always Yes" | `form1040.rs`: `match inputs.digital_asset_answer` → `match Some(true)` | `-p btctax-forms -E 'test(form_1040_digital_asset_box_is_the_answer_yes_no_or_neither)'` | `answer Some(false): the Yes box  left: Some("1")  right: None` | RED, matches report verbatim |
| F2 — the SLICE's box back to the ledger predicate | `admin.rs`: `digital_asset_answer: da_answer` → `digital_asset_answer: Some(reportable_activity)` | `-p btctax-cli -E 'test(the_slice_prints_the_digital_asset_answer_and_never_the_ledger)'` | `answer None: the Yes box on the emitted PDF  left: Some("1")  right: None` | RED, matches report (line number shifted 1490→1496, message identical) |
| F3 — delete the slice's DA cross-check | `admin.rs`: wrapped the arm-(2)/(3) refusal block in `if false { .. }` | `-p btctax-cli -E 'test(a_contradicted_no_refuses_the_slice_and_writes_nothing)'` | `a \`No\` the ledger contradicts must refuse: IrsPdfReport { … advisories: [], … hand_marks: [] }` | RED, matches report verbatim |
| F4 — slice's `advisories` back to `Vec::new()` | `admin.rs`: replaced the `match working.as_ref() { .. }` advisory block with `advisories: Vec::new()` | `-p btctax-cli -E 'test(an_off_ledger_yes_prints_the_slice_with_the_advisory)'` | `…and it is WARNED: []` | RED, matches report verbatim |
| F5 — drop the Step 0 regime gate | `step0.rs`: `let live = regime.is_some_and(..)` → `let live = true` | `-p btctax-cli -E 'test(the_venue_row_fires_only_on_a_year_whose_form_1099da_question_is_live)'` | `TY2024 asks no Form 1099-DA question, so no venue can be 'not accounted for' …: [Step0Row {…}]` | RED, matches report verbatim |
| F6 — relief period always `Within` | `step0.rs::relief_period` → constant `ReliefPeriod::Within` | `-p btctax-cli -E 'test(the_standing_order_row_cites_the_notice_only_inside_its_relief_period)'` | `TY2024: §4.02(2) is asserted as the authority only inside its relief period: … left: true  right: false` | RED, matches report (shape identical) |
| F7 — Step 0 contradiction row never emitted | `step0.rs`: wrapped the `panel.contradictions.push(..)` block in `if false { .. }` | `-p btctax-cli -E 'test(the_step0_panel_names_a_digital_asset_answer_the_ledger_contradicts)'` | `assertion \`left == right\` failed: []  left: 0  right: 1` | RED, matches report verbatim |
| F8 — banned phrase planted inside a RENDERED prompt (granularity) | `questions.rs::digital_asset_prompt` — prefixed the rendered text with `"which account? "` | `-p btctax-cli -E 'test(no_registry_question_asks_about_venue_or_account_granularity)'` | `R9: account granularity is documented, not asked — this prompt asks it: RENDERED_PROMPTS DigitalAssetActivity: which account? at any time during 2026, did you: …` | RED, matches report verbatim |
| F9 — drop the `RENDERED_PROMPTS` chain (anti-vacuity half) | `step0_panel.rs` (test file): removed the `.chain(RENDERED_PROMPTS…)` from the test's own prompt-collection | same command | `every RENDERED prompt must be IN the scanned set — …  left: 0  right: 4` | RED, matches report verbatim |
| F10 — DA event finder loses its year filter | `return_1040.rs::first_digital_asset_event`: all three `.year() == year` filters → `.year() >= 1900` | `-p btctax-core -E 'test(the_refusal_names_the_earliest_qualifying_event_and_fires_exactly_when_one_exists)'` | `a 2023-12-31 disposal and a 2025-01-01 receipt are not 2024 events — the cross-check reads the event's own date, …` | RED, matches report verbatim |

## Review's own evidence, re-verified

| finding | holding test(s) | how verified | verdict |
|---|---|---|---|
| Review seam #3 — the cross-check moved into the param-free tier | `return_refuse.rs::param_free_tier::every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` | Planted a dead `DigitalAssetAnswerContradictsLedger` refuse arm at the top of `screen_inputs_tiered` (the param-free body). `-p btctax-core -E 'test(every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths)'` → **RED**: `the fixture table and the source census must name the same param-free rules … right: {…, "DigitalAssetAnswerContradictsLedger", …}` (the source census; not in the fixture table). Reverted. | Holds |
| Review seam #4 — the panel writes nothing | (no runtime plant possible) | `step0_panel(state: &LedgerState, events: &[LedgerEvent], ri: Option<&ReturnInputs>, year: i32, regime: Option<InformationReturnRegime>) -> Step0Panel` — every parameter is a shared reference or `Copy`; there is no `&mut`, no `Session`/connection handle through which a write could occur. A write is a compile error, exactly as the review measured. | Holds (structural, as the review itself found) |
| C-1 — the R6 arm-(2) slice prints the answer, never the ledger, read back through `Form1040Map::ty2025()` off the emitted `form_1040_capgains.pdf` | `slice_from_answers.rs::the_slice_prints_the_digital_asset_answer_and_never_the_ledger`, `::a_contradicted_no_refuses_the_slice_and_writes_nothing`, `::an_off_ledger_yes_prints_the_slice_with_the_advisory`; `sp2.rs::form_1040_digital_asset_box_is_the_answer_yes_no_or_neither` | Baseline (unplanted) run of all four is green — `da_boxes()` in `slice_from_answers.rs:1427-1443` reads the box via `Form1040Map::ty2025()`, the review's own method. Measured table on this tree: `Some(true)`→`da_yes=Some("1")`/no mark; `None`→neither box/`hand_marks.len()==1`; `Some(false)` against a ledger with a 2025 disposal→**REFUSE**, `wrote_nothing` (F3's baseline). Killed by F2 (box), F3 (cross-check), F4 (advisory). | Holds |
| I-1 — the venue list obeys the year's regime | `step0_panel.rs::the_venue_row_fires_only_on_a_year_whose_form_1099da_question_is_live`, `::a_venue_with_rows_and_no_broker_answer_is_named_and_an_answered_one_is_not` | Baseline green; killed by F5 (regime gate) and P8 (venue-drop). | Holds |
| M-1 — standing-order row obeys the relief period | `step0_panel.rs::the_standing_order_row_cites_the_notice_only_inside_its_relief_period` | Baseline green; killed by F6. | Holds |
| M-2 — a same-day (on-or-before) election silences the row | `step0_panel.rs::the_standing_order_row_fires_with_no_election_and_is_silent_with_one_effective_on_or_before_the_sale`, part (d) (lines ~250-273: election made+effective the day OF the sale, 6h after) | Read the committed fixture — part (d) is present, asserts SILENT, and cites the `resolve_election`/`<=` root plus FR-77 by name. Ran the whole test at baseline (green) three separate times (initial baseline, post-P6-revert, post-P7-revert). | Holds |
| M-3 — granularity test scans the RENDERED prompts too | `step0_panel.rs::no_registry_question_asks_about_venue_or_account_granularity` | Baseline green; killed by F8 (banned phrase) and F9 (anti-vacuity chain). | Holds |
| M-4 — Step 0 surfaces a contradicted `No` | `step0_panel.rs::the_step0_panel_names_a_digital_asset_answer_the_ledger_contradicts` | Baseline green; killed by F7. | Holds |
| N-1 — `disposal_compliance`'s doc comment restored | `crates/btctax-core/src/project/compliance.rs:224-249` | Read the file: the ~25-line doc block (scope boundary, NFR4, "Read-only") sits directly above `pub fn disposal_compliance`; `voided_set` (:183) carries its own short comment. Matches the fold's claim. | Holds (doc-only, no finding) |
| N-2 — report-text inconsistencies (2024 vs 2026 date; 3,358 vs 3,363) | n/a (prose, not code) | Not independently re-checked (doc-only, Minor by rule); the persisted implementation report already reads "2024-06-15" (row 1) and states the sub-total distinction explicitly. | Doc-only, no finding |
| N-3 — off-by-a-few instruction citations | `questions.rs:141,314`, `return_refuse.rs:617`, `kat_digital_asset_question.rs:5,29,342` | `grep -n "i1040gi--2025.txt:13"` across all three files — every site now reads `:1399-1401` (mandatory-answer sentence) and `:1382-1391` + `:1395-1396` (carve-outs), consistently. | Holds (doc-only, no finding) |
| N-4 — KAT comment matches its own assertion | `kat_digital_asset_question.rs::the_refusal_names_the_earliest_qualifying_event_and_fires_exactly_when_one_exists` | Read the test (lines 356-407): moves the year off BOTH events (disposal→2023-12-31, income→2025-01-01), asserts a premise that the events are still present, then asserts no refusal. Killed by F10. | Holds |

## Negative claims

| claim | check | result |
|---|---|---|
| `xtask stop-list` clean | `cargo run -q -p xtask -- stop-list` | `R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 64 registry prompts scanned; no forbidden shape` — matches the fold report's "8 / 4 / 64" exactly |
| census-join / line-coverage unmoved | `-p xtask -E 'test(census_join) \| test(line_coverage) \| test(coverage)'` | 16/16 pass, including `the_join_reds_on_every_planted_defect`, `the_committed_maps_are_covered_and_placed`, `the_committed_coverage_table_is_consistent_with_the_form_text` |
| TY2024 golden corpus still files | `-p btctax-cli -E 'binary(export_irs_pdf) or binary(fullreturn_oracle)'` | 45/45 pass, incl. `ty2024_real_ledger_fills_box_c_f_and_line7_and_da`, `real_ledger_fills_clean_official_pdfs`, `fullreturn_fixture_matches_its_emitter`, `fullreturn_fixture_is_the_kitchen_sink_oracle` |
| R6 slice-path tests green | `-p btctax-cli -E 'binary(slice_from_answers)'` | 23/23 pass |
| `digital_asset_activity = None` blocks commit, listed by `interview_state` | `-p btctax-cli -E 'test(an_unanswered_digital_asset_question_blocks_and_is_listed_by_interview_state)'` | PASS |
| T4b's opener seeds it `None` | `-p btctax-cli -E 'test(the_year_opener_seeds_the_digital_asset_answer_blank)'` | PASS |
| Every new shaped identifier in fixtures is allowed | `bash scripts/pii-scan-generic.sh HEAD` | `pii-scan: clean (HEAD).` exit 0 |

## Final sanity

`git status --short` empty and `git diff --stat` empty at the end — every plant was reverted. Re-ran
the brief's exact four scoped commands one final time, all green:
`-p btctax-core -E 'test(return_1040) \| test(return_refuse) \| test(digital_asset)'` → 210/210;
`-p btctax-cli -E 'test(panel) \| test(answer) \| test(export_irs_pdf) \| test(tax_report)'` → 58/58;
`-p btctax-tui-edit -E 'test(panel) \| test(step0)'` → 1/1;
`-p xtask -E 'test(stop_list)'` → 2/2.

## Verdict

Every one of the 13 build kills, the 10 fold kills, the review's own C-1 PDF read-back, I-1, M-1,
M-2, M-3, M-4, N-1, N-3, N-4, and the two structural claims (census-tier placement, the panel writes
nothing) reproduce exactly as claimed. Every negative claim holds. No box printed from anything but
the answer was found on this tree. Nothing in this pass contradicts the persisted implementation
report, the review, or the fold ledger.

Counts: C=0 I=0 M=0 N=0
