# Re-verification — interview T8 build + fold, at `a17b9d6a`

Independent verifier, own worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a62cad5883bd5a424`, commit `a17b9d6a` (the T8 fold).
`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` for every cargo command; scoped runs only,
per the brief. Every plant made from a `sed`/`python3` in-place edit or a `cp` backup, reverted before
the next plant; no commits, no `git checkout --` used destructively, no subagents. `git status
--porcelain` is empty at the time of writing.

**Environment.** `crates/btctax-forms/forms/2025/f1040.pdf` is present (220,237 bytes — the same size
the review recorded), so every emitter read-back kill below ran against the real TY2025 template, not
a stub. No claim is marked unverified for a missing template.

## Kill table — build report + fold report

| # | Kill | Plant | Command | Observed RED | Verdict |
|---|---|---|---|---|---|
| 1 | `the_credit_column_reaches_the_row_the_emitter_prints` (core) — CTC edge | `credit: CreditColumn::default()` in `ReturnHeader::build` | `nextest -p btctax-core -E test(the_credit_column_reaches_the_row_the_emitter_prints)` | `assertion left == right failed: the CTC edge itself: the row the emitter prints / left: Neither / right: ChildTaxCredit` | **PASS** — matches report verbatim |
| 1b | same test, I-1 half (stale (5)(b)) | dropped `walk.demands(gate) &&` for `lived_with_you_in_us` | same test | `packet.rs:...:1736: row (5)(b) is blank under an unchecked (5)(a): the walk never demanded it, so the stale Some(true) leaf must not reach the page` | **PASS** — I-1's core-side fix holds |
| 1c | `a_stale_row_5b_answer_is_not_printed_under_an_unchecked_row_5a` (forms, I-1's page-level kill) | same plant, via `push_dependents_grid` into the real TY2025 template | `nextest -p btctax-forms -E test(a_stale_row_5b_answer_is_not_printed_under_an_unchecked_row_5a)` | `full_return_forms.rs:772: assertion left == right failed: dependent 1, lived_with_you_in_us: (5)(b) may not print under an unchecked (5)(a) / left: Some("1") / right: None` | **PASS** — I-1's fix holds on the printed page |
| 2 | `the_committed_ty2025_grid_map_is_the_one_measured_off_the_form` (xtask) | swapped dependent 1's row-(6) FQNs (`c1_20[0]`↔`c1_21[0]`) in `forms/2025/f1040.map.toml` | `nextest -p xtask -E test(the_committed_ty2025_grid_map_is_the_one_measured_off_the_form)` | `dependent 1, FullTimeStudent / left: "…c1_21[0]" / right: "…c1_20[0]"` | **PASS** — matches report verbatim |
| 3 | `a_transposed_pair_reds`, `a_missing_cell_reds` | self-contained (in-test geometry mutation, no external plant needed) | `nextest -p xtask -E test(dependents_grid)` | both PASS at HEAD (standing negative tests, verified to run and discriminate) | **PASS** |
| 4 | the register (`census_accounts_for_every_field`) | left `field_census.rs`'s `(2025, "f1040", n)` at `196` with 25 cells mapped | `nextest -p btctax-forms -E test(census_accounts_for_every_field)` | `2025/f1040: recorded 196 unaccounted field(s), measured 171. The register is SHRINK-ONLY…` | **PASS** — matches report verbatim |
| 5 | `the_ty2025_dependents_grid_prints_the_answers` (forms) | `credit_for_other_dependents` written on `g.credit != ChildTaxCredit` | `nextest -p btctax-forms -E test(the_ty2025_dependents_grid_prints_the_answers)` | `full_return_forms.rs:682: assertion left == right failed: dependent 3, credit_for_other_dependents / left: Some("2") / right: None` | **PASS** — matches report verbatim; confirms the 3-dependent fixture (not 2) is what discriminates |
| 6 | `the_ty2024_1040_ignores_the_computed_grid_and_fills_deterministically` (renamed, M-1) | added `check(w, p, &row.ctc, d.grid.credit == CreditColumn::ChildTaxCredit)` in the TY2024 row loop | `nextest -p btctax-forms -E test(the_ty2024_1040_ignores_the_computed_grid_and_fills_deterministically)` | `full_return_forms.rs:880: assertion left == right failed: row 0 ctc / left: Some("1") / right: None` | **PASS** — matches report/fold verbatim |
| 7 | `hoh_refuses_until_the_instructions_own_tests_are_answered` (core) | class-(A) skippable loop stops refusing (`for sk in SKIPPABLE_QUESTIONS { continue; …}`) | `nextest -p btctax-core -E test(hoh_refuses_until_the_instructions_own_tests_are_answered)` | `return_refuse.rs:4728: assertion left == right failed / left: None / right: Some(HohMaritalBasisUnanswered)` | **PASS** — line drifted 4725→4728 (fold-report-documented `cargo fmt` drift); mechanism and assertion match |
| 8 | `the_untranscribed_marital_bases_refuse_naming_their_rule` (core) | `MarriedLivedApart` arm made unreachable (`if false` guard) | `nextest -p btctax-core -E test(the_untranscribed_marital_bases_refuse_naming_their_rule)` | `return_refuse.rs:4797:45: this basis refuses on BOTH tiers` | **PASS** — matches report's cited line 4797 exactly |
| 9 | `qss_asks_all_five_of_the_instructions_conditions` (core) | QSS `No` stops refusing (`if false && get == Some(false)`) | `nextest -p btctax-core -E test(qss_asks_all_five_of_the_instructions_conditions)` | `return_refuse.rs:4896:45: a NO refuses on BOTH tiers` | **PASS** — matches fold report's cited line 4896 exactly |
| 10 | `the_line_19_forgo_is_sized_from_the_package_and_blank_without_one` (core) | `each: Some(dec!(2000))` hardcoded, ignoring `params` | `nextest -p btctax-core -E test(the_line_19_forgo_is_sized_from_the_package_and_blank_without_one)` | `interview_state.rs:681: assertion left == right failed: no package ⇒ no invented figure / left: Some(2000) / right: None` | **PASS** — matches report verbatim |
| 11 | `income_answer_asks_every_live_skippable_including_the_class_a_one` (cli) | class-(A) blocking push in `interview_state_with` disabled (`if false && (...)`) | `nextest -p btctax-cli -E test(income_answer_asks_every_live_skippable_including_the_class_a_one)` | `answer.rs:2172: HohMaritalBasis declares a refusal, so the panel must list it as BLOCKING` | **PASS** — line drifted 2120→2172 (fold-noted drift); message matches |
| 12 | `income_answer_prints_the_line_19_forgo_with_its_size` (cli) | renderer's `NOT COMPUTED` heading write disabled (`if false && !st.not_computed.is_empty()`) | `nextest -p btctax-cli -E test(income_answer_prints_the_line_19_forgo_with_its_size)` | `answer.rs:2112: panicked` (assertion on missing "NOT COMPUTED" string) | **PASS** — matches report's cited line 2112 exactly |
| 13 | `xtask prompt-check` (M-2 primary) | `HohPaidOverHalfCostOfKeepingUpHome`'s quoted clause: "over half" → "most of" | `cargo run -p xtask -- prompt-check` | `question clause 5 (HohPaidOverHalfCostOfKeepingUpHome) is NOT in the string the filer reads: "You paid over half the cost of keeping up a home"` | **PASS** — the fold moved this clause INTO the checked span (M-2's fix); the same edit that was green at HEAD before the fold is now caught |
| 13b | `xtask prompt-check` (M-2 sweep finding) | `HohQualifyingPerson` Test 1's operative clause: "You paid over" → "You paid most of" | `cargo run -p xtask -- prompt-check` | `question clause 1 (HohQualifyingPerson) is NOT in the string the filer reads: "You paid over half the cost of keeping up a home that was the main home"` | **PASS** — the fold's second M-2 fix (found in the sweep) also holds |
| I-2a | `every_bundled_years_ctc_per_child_is_the_named_ceiling` (adapters, new pin) | bundled TY2026 (`by_year.insert(2026, ty2026_full_return());`) in `BundledFullReturnTables::load` | `nextest -p btctax-adapters -E test(every_bundled_years_ctc_per_child_is_the_named_ceiling)` | `shipped_tables_are_the_validated_tables.rs:1084: ["TY2026: the bundled package says §24(h)(2) is 2200 per child, advisories::CTC_PER_CHILD_SS24H2 says 2000"]` | **PASS** — matches fold verbatim |
| I-2b | same plant, old core positive control | same TY2026 bundle plant | `nextest -p btctax-core -E test(the_named_ceiling_is_the_figure_the_proof_multiplies_by)` | still **PASS** (green) with the plant in place | **PASS** — confirms the fold's own claim that the OLD test cannot see a bundled package (still blind by design, now honestly named) |
| I-2c | same new pin, plant B | shipped TY2024 figure moved `2000`→`2200` in `tax_tables.rs:151` | `nextest -p btctax-adapters -E test(every_bundled_years_ctc_per_child_is_the_named_ceiling)` | `["TY2024: the bundled package says §24(h)(2) is 2200 per child, advisories::CTC_PER_CHILD_SS24H2 says 2000"]` | **PASS** — matches fold verbatim |
| I-3a | `the_shared_entry_space_prints_the_qualifying_childs_name_on_hoh_and_qss` + `the_shared_entry_space_has_exactly_one_claimant_per_status` | dropped `Qss` arm from `FilingStatus::wants_qualifying_child_name` | `nextest -p btctax-forms -E test(the_shared_entry_space_prints_the_qualifying_childs_name_on_hoh_and_qss)`; `nextest -p btctax-core -E test(the_shared_entry_space_has_exactly_one_claimant_per_status)` | forms: `left: None / right: Some("Robin Roe")` for Qss; core: `left: [Mfs, HoH] / right: [Mfs, HoH, Qss]` | **PASS** — both reproduce exactly |
| I-3b | same forms kill, defect-as-shipped | deleted the emitter's HoH/QSS write (`if status.wants_qualifying_child_name() … { text(…) }` block) entirely | `nextest -p btctax-forms -E test(the_shared_entry_space_prints_the_qualifying_childs_name_on_hoh_and_qss)` | `left: None / right: Some("Robin Roe")` for **HoH** | **PASS** — matches fold verbatim |
| Dev-5 | `every_filing_status_is_in_all` + claimant test (totality anchor) | dropped `Qss` from `FilingStatus::ALL` (`[FilingStatus; 4]`) | `nextest -p btctax-core -E test(every_filing_status_is_in_all) or test(the_shared_entry_space_has_exactly_one_claimant_per_status)` | `left: 4 / right: 5`; claimant test also reds `[Mfs, HoH] / [Mfs, HoH, Qss]` | **PASS** — matches Deviation 5's claim exactly |
| FR-67 | `the_nra_spouse_election_gate_refuses_unanswered_and_on_yes` (core) | `Yes` branch of the §6013(g)/(h) gate made unreachable (`if false && …`) | `nextest -p btctax-core -E test(the_nra_spouse_election_gate_refuses_unanswered_and_on_yes)` | `return_refuse.rs:4840:41: a YES refuses on BOTH tiers` | **PASS** — matches both report's and fold's cited line 4840 exactly |

**"Other standing kills" not individually replanted** (`only_the_two_credit_arms_check_a_box`,
`single_and_mfj_ask_no_hoh_or_qss_question`, `a_hoh_test_answered_no_refuses_with_the_exit`,
`the_qss_window_is_derived_from_the_tax_year`, `the_ty2025_grid_checks_the_more_than_four_box_with_its_statement`,
`every_slot_caption_is_the_forms_own_words`): all run and PASS at HEAD in the scoped suite below;
no per-kill plant description was given by the brief or the report for these, so they are recorded as
"ran, currently green" rather than independently mutation-verified. This is a gap in this
re-verification's coverage, not a finding against the fold — flagged rather than silently skipped.

## Negative claims (brief item 3)

| claim | check | result |
|---|---|---|
| TY2024 golden/emitter snapshot unchanged | full scoped suite green pre- and post- every plant, `git status --porcelain` empty throughout | **confirmed** |
| `UNCENSUSED` register's `f1040` count = 171 | `census_accounts_for_every_field` PASS at HEAD; reds to `196` when reverted (kill 4) | **confirmed** |
| every mapped cell has a `[census]` entry | **does not apply as literally stated** — TY2025's `f1040.map.toml` has NO `[census]` section at all (it is on the `UNCENSUSED` register instead, confirmed by kill 4); "mapped" and "censused" are mutually exclusive per the review's own Seam 2 finding (`verdict()` refuses a field that is both). Recorded as **N/A**, not skipped. |
| `line-coverage` / `census-join` / `stop-list` unmoved or as stated | ran fresh: `373 money lines / 18 forms / 31 exceptions (ratchet 31) / 0 unverifiable`; `290 unmodeled entries / 13 maps`; `8+4 sources / 86 registry prompts` | **confirmed, byte-identical to the fold report** |
| `prompt-check` = 88 assertions | ran fresh: `xtask prompt-check: OK — 88 assertions, all verbatim` | **confirmed** |
| no-brick test covers every new question | `income_answer_asks_every_live_declaration` is registry-derived (loops `FORM_QUESTIONS` directly, no hand-list); `scenario_for` has explicit arms for `HohQualifyingPerson`/`HohPaidOverHalfCostOfKeepingUpHome`/`NraSpouseResidentElection`/all five QSS ids; test PASSES | **confirmed** |
| PII scan clean | `bash scripts/pii-scan-generic.sh HEAD` | `pii-scan: clean (HEAD).` exit 0 — **confirmed** |

## Environment note

Six `form_delta` tests were not run individually (out of the brief's scoped filters) but the TY2025
`f1040.pdf` template's presence — the actual gating fact — was confirmed directly (220,237 bytes).
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` was not invoked;
it is named by the brief as an expected environment failure and out of this checklist's scope.

## Scoped baseline (before and after all plants)

```
btctax-core   -E 'test(dependent)|test(hoh)|test(qss)|test(return_refuse)|test(interview_state)'  → 129/129
btctax-forms  -E 'test(f1040)|test(field_census)|test(dependents)'                                 → 4/4
btctax-input-form (whole crate)                                                                    → 72/72
btctax-cli    -E 'test(answer)|test(fullreturn)|test(export_irs_pdf)'                               → 65/65
xtask         -E 'test(dependents_grid)'                                                            → 5/5
btctax-adapters -E 'test(ctc)'                                                                      → 1/1
```
All green, both before the first plant and after the last revert. `git status --porcelain` empty
throughout; no plant was left in place.

## Disposition

Every kill named by the T8 build report, the T8 fold report, and the T8 seam review's six findings
(I-1, I-2, I-3, M-1, M-2, N-1) reproduces its claimed RED when planted today, with messages and (where
cited) line numbers matching the persisted reports — small line-number drift on two kills (7, 11) is
exactly the `cargo fmt` drift the fold report itself flags in its preamble, and the assertion text at
the new location matches. No kill failed to red, no review finding lacks a holding test, and no
inspected claim was contradicted by the tree. The one coverage gap is that six "other standing kills"
named in the build report without a specific plant description were confirmed to PASS but not
independently mutation-verified in this pass (see the table note above).

Counts: C=0 I=0 M=0 N=0
