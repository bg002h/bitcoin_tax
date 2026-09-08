# Re-verification — interview T7 build + fold (`2213eeb5`)

Independent verifier, own worktree (checked out at `2213eeb5`, matching HEAD — no checkout needed).
Every plant below was made with `cp`-backup / restore (never `git checkout --` on a source file mid-plant,
though it was used once, correctly, to bulk-revert a `make docs` regeneration of `docs/man/`). No
commits. `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` for every cargo command; every run
was scoped (`-p <crate> -E '<filter>'`), never `cargo test` or a whole-workspace run. `git status
--porcelain` is empty at the time of writing.

Baseline (unmutated tree, `-p btctax-core -p btctax-cli -p btctax-input-form -p xtask`): **2353
passed, 7 failed, 2 skipped** — the 7 failures are exactly the six `form_delta` tests plus
`harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`, the settled
environment failures (gitignored archived PDFs, redirected target dir). Confirmed identical after
every plant/revert cycle below (re-run at the end, same 2353/7/2360 split).

## Build report's eleven mutations (§5, M1–M11)

| kill | plant | command | RED text | verdict |
|---|---|---|---|---|
| M1 — screen loop removed | `return_refuse.rs`: `if let Some(r) = screen_dependent_gates(...) { return Some(r); }` → `let _ = screen_dependent_gates(...);` | `-p btctax-core -E 'test(every_live_gate_that_is_blank_refuses_by_name) or test(every_refuse_edge_names_its_rule_in_the_screen_detail)'` | *"a blank live DateOfBirth must refuse with its own reason — left: None, right: Some(DependentGateUnanswered { row: 0, gate: DateOfBirth })"*; *"the premise: this perturbation refuses"* | RED — PASS |
| M2 — `claim_path` flipped | `ProvidedOverHalfOwnSupport`'s `claim_path: Some(false)` → `Some(true)` | `-p btctax-core -E 'test(every_gate_at_its_claim_path_answer_reaches_the_ctc_edge)'` | `left: CreditForOtherDependents, right: ChildTaxCredit` | RED — PASS |
| M3 — params-quoting prompt ungated | `GrossIncomeUnderLimit`'s `prompt_from_params: Some(gross_income_prompt)` → `None` | `-p btctax-core -E 'test(the_params_quoting_gate_waits_then_blocks_and_quotes_the_figure) or test(the_gross_income_limit_matches_the_shipped_params)'` | *"on a params-less year the gate WAITS: []"*; *"the prompt quotes the year's figure: …"* (fallback text shown instead) | RED — PASS |
| M4 — filer-TIN question made always-live | `questions.rs`: `live: |ri| !ri.header.dependents.is_empty()` → `live: |_ri| true` | `-p btctax-core -E 'test(a_return_with_no_dependents_asks_nothing)'` | *"Step 5 is reached only through a dependent, so the question is not live"* | RED — PASS |
| M5 — classifier row dropped | `classifier.rs`: `c.dependent_gate(married, G::Married);` → discarded | `-p btctax-core -E 'test(the_classifiers_gate_rows_line_up_with_the_registry)'` | *"every Option<bool> gate in the registry is classified, and nothing else is"* — `Married` absent from left set | RED — PASS |
| M6 — Step 1 routing broken | `dependent_gates.rs`: `if is_qualifying_child { step2(w) }` → `if false && is_qualifying_child { step2(w) }` | `-p btctax-core -E 'test(every_live_gate_that_is_blank_refuses_by_name) or test(the_flowchart_truth_table) or test(a_child_born_in_november_reaches_the_child_tax_credit) or test(every_gate_at_its_claim_path_answer_reaches_the_ctc_edge) or test(every_refuse_edge_names_its_rule_in_the_screen_detail)'` | all 5 tests failed (e.g. `right: Unanswered(QrRelationshipOrMemberOfHousehold)`) | RED — PASS (5/5, as claimed) |
| M7 — R12 `waiting` arm removed | `interview_state.rs`: the `if g.needs_params() && params.is_none() { ... }` block guarded with `if false && ...` | `-p btctax-core -E 'test(the_params_quoting_gate_waits_then_blocks_and_quotes_the_figure)'` | *"on a params-less year the gate WAITS: []"* | RED — PASS |
| M8 — two gate answers carried across the year | `open_next_year.rs`: `qc_relationship: None` / `lived_with_you_over_half_year: None` → `d.qc_relationship` / `d.lived_with_you_over_half_year` | `-p btctax-cli -E 'test(a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not)'` | *"every §152 gate must cross BLANK — a prior year's answer is not testimony for this one: [QcRelationship, LivedWithYouOverHalfYear]"* | RED — PASS |
| M9 — answer key changed identity→row index | `answer.rs`: `ssn_hash: dependent_ssn_hash(&ri.header.dependents[row].ssn)` → `ssn_hash: format!("row{row}")` | `-p btctax-cli -E 'test(income_answer_asks_the_dependent_gates_and_the_sweep_settles)'` | *"row 0's QcRelationship was asked but no record was written under its identity"* | RED — PASS |
| M10 — row-(5)(a) help mutated (3 ways) | self-contained B1 test — plants a paraphrase, a truncation and an empty string against `exception_is_quoted` internally, no source mutation needed | `-p xtask -E 'test(the_row_five_a_help_quotes_the_exception_and_a_mutated_help_reds) or test(the_row_five_a_cite_names_the_span_this_check_reads)'` | both PASS (confirms the pure-function checker reds on all 3 internal plants and is green on the real text) | PASS — kill mechanism verified |
| M10 (live-source cross-check) | `dependent_gates.rs`: `"count as time the person lived with you"` → `"may count as time the person lived with you"` | `cargo run -p xtask -- prompt-check` | *"row (5)(a)'s help does NOT quote design/forms/extract/i1040gi--2025.txt:1905-1913 verbatim …"* | RED — PASS (extra check beyond the brief, confirms the live-source path too) |
| M11 — `xtask prompt-check` caught a real paraphrase during the build | not re-planted (self-reported, not a kill instrument to re-verify); confirmed `prompt-check` runs clean today (54 assertions) | `cargo run -p xtask -- prompt-check` | `xtask prompt-check: OK — 54 assertions, all verbatim` | PASS |

## Fold section's kills (C-1, I-1 … I-5, M-1 … M-5, N-1 … N-3)

| kill | plant | command | RED text | verdict |
|---|---|---|---|---|
| C-1 / I-1 — banner hashed instead of words | `answer.rs:940`: `record_answer(&mut ri, key, &words, ...)` → `&shown` | `-p btctax-cli -E 'test(income_answer_asks_the_dependent_gates_and_the_sweep_settles)'` | `left: WordingChanged, right: Given` | RED — PASS |
| I-2(a) — blank SSN no longer refuses first | `return_refuse.rs`: `if ssn_digits(&d.ssn).is_empty()` → `if false && ssn_digits(&d.ssn).is_empty()` | `-p btctax-core -E 'test(a_dependent_row_with_no_ssn_refuses_before_any_gate_and_names_the_row)'` | `left: None, right: Some(DependentIdentityUnanswered { row: 0 })` | RED — PASS |
| I-2(a) — duplicate-SSN scan neutered | `return_refuse.rs`: `.position(|o| ssn_digits(&o.ssn) == digits)` → `.position(|o| false && ...)` | `-p btctax-core -E 'test(two_dependent_rows_with_the_same_ssn_refuse_at_commit_and_at_import) or test(retiring_one_identity_cannot_take_anothers_records_because_a_shared_key_is_refused) or test(every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths)'` | all 3 RED (e.g. *"the commit gate must reach DependentSsnDuplicated on its own fixture"*) | RED — PASS |
| I-2(b) — session `asked` key collapsed off row | `answer.rs`: `asked_key_of`'s `AskedKey::DependentGate { row: *row, ... }` → `row: 0` | `-p btctax-cli -E 'test(the_sessions_asked_key_is_the_row_so_a_shared_identity_starves_nobody)'` | *"row 1's QcRelationship is live and was never asked — the other row's identical key filtered it out of every later round…"* | RED — PASS |
| I-2(positive) — real-command double-shape test | none (positive confirmation) | `-p btctax-cli -E 'test(income_answer_refuses_a_dependent_row_with_no_identity_or_a_shared_one)'` | PASS | PASS |
| I-3 — seeded DOB pre-filled again | `open_next_year.rs`: `date_of_birth: None` (dependent literal) → `date_of_birth: d.date_of_birth` | `-p btctax-cli -E 'test(a_bare_enter_does_not_confirm_a_seeded_dependents_date_of_birth) or test(a_dependent_identity_is_seeded_blocking_and_a_venue_key_is_not)'` | `left: Some(2015-04-01), right: None` (both tests) | RED — PASS |
| I-4 — shipped help reverted | `cli.rs`: the two rewritten sentences → the old *"no row is created"* wording; `cargo run -p xtask -- docs` regenerated `docs/man/` | `-p xtask -E 'test(the_open_next_year_page_states_what_fr70_actually_does_with_dependents)'` | *"the retracted claim is still shipped: a dependent row IS created"* at `docs.rs:406` | RED — PASS (exact line match) |
| I-5 — Step 4 citizen STOP deleted | `dependent_gates.rs`: the `if w.no(G::CitizenNationalResidentOrCanadaMexico) { ... }` block in `step4` removed | `-p btctax-core --no-fail-fast` | `1322 tests run: 1321 passed, 1 failed`; `the_step_four_truth_table`: *"Step 4 q2 citizenship No ⇒ REFUSE … expected a refusal, got CreditForOtherDependents"* | RED — PASS |
| M-1 — return-level STOP anchor collapsed | `attribute.rs`: `R::DependentRefusedByQuestion { question, .. } => vec![decl(*question)]` → routed through `dependent_gate_field(QcRelationship)` | `-p btctax-input-form -E 'test(the_return_level_dependent_stop_anchors_on_its_question_not_on_a_row_gate)'` | `left: [Field(DepGateQcRelationship)], right: [Field(DeclDependentTaxpayer)]` | RED — PASS |
| M-2 — the `\`-continuation dropped, reintroducing whitespace runs | `return_refuse.rs`: the STOP detail's two `\`-continued lines de-escaped | `-p btctax-core -E 'test(every_refuse_edge_names_its_rule_in_the_screen_detail)'` | *"a missing `\` continuation collapses into a run of spaces: …"* | RED — PASS |
| M-3(a) — scrub keeps DOB verbatim | `scrub.rs`: `date_of_birth: date_of_birth.map(synthetic_dependent_dob)` → `date_of_birth: *date_of_birth` | `-p btctax-core -E 'test(a_scrubbed_dependent_reaches_the_same_verdict_including_across_the_january_boundary)'` | *"an ordinary birthday: the child's real birth date must not ride into the shareable copy" — left: Some(2009-04-15), right: Some(2009-04-15)* | RED — PASS |
| M-3(b) — January-1 convention ignored | `scrub.rs`: `synthetic_dependent_dob` dropped the `year - 1` branch | same test | *"born January 1 … the flowchart must reach the same verdict on TY2025 — left: CreditForOtherDependents, right: Unanswered(SsnsValidForEmploymentIssuedByDueDate)"* | RED — PASS (exact match to fold report) |
| M-3(c) — `is_scalar_date` neutered | `scrub_axis.rs`: `is_scalar_date` → always `false` | `-p btctax-core -E 'test(a_date_is_one_value_and_a_row_vector_is_not)'` | *"time::Date's wire form changed — it is now Array [...], so is_scalar_date no longer recognises it…"* | RED — PASS |
| M-4 — STOP re-gated behind `unanswered_refuses` | `return_refuse.rs`: `screen_dependent_values(ri)` call wrapped in `if tier.unanswered_refuses { ... }` | `-p btctax-core -E 'test(every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths) or test(no_refusal_in_the_import_tier_prescribes_income_answer) or test(two_dependent_rows_with_the_same_ssn_refuse_at_commit_and_at_import)'` | all 3 RED — *"income import screens on a year with NO package — DependentSsnDuplicated must fire there too — left: None, right: Some(\"DependentSsnDuplicated\")"* | RED — PASS |
| M-5 — a `$` figure typed back into `help` | `dependent_gates.rs`: `GrossIncomeUnderLimit`'s help rewritten to include a literal year table | `-p btctax-core -E 'test(no_dependent_gate_help_types_a_figure_the_params_carry)'` | *"GrossIncomeUnderLimit's help types a dollar figure: …"* | RED — PASS |
| N-1 — the January-1 `+1` term dropped | `return_1040.rs`: `considered_age_at_year_end` → `year - dob.year()` (term removed) | `-p btctax-core -E 'test(the_flowchart_truth_table)'` | *"Step 3 q3 born January 1 ⇒ considered 17 … — left: ChildTaxCredit, right: CreditForOtherDependents"* | RED — PASS |
| N-2 — leaf↔gate pairing swapped | `classifier.rs`: `c.dependent_gate(married, G::FilingJointReturn)` / `c.dependent_gate(filing_joint_return, G::Married)` (swapped) | `-p btctax-core -E 'test(every_classifier_row_pairs_its_gate_with_its_own_leaf) or test(the_classifiers_gate_rows_line_up_with_the_registry)'` | new test RED (`left: [(Married, Some(true))], right: [(FilingJointReturn, Some(true))]`); old set-based test stayed GREEN — the exact finding N-2 names | RED (new) / GREEN (old) — PASS |
| N-3 — `clear`'s liveness guard removed | `sections.rs`: `dep_gate_tristate!`'s `clear` closure's `if !DEPENDENT_GATES[$idx].live(...) { return Err(NoSuchRow); }` deleted | `-p btctax-input-form -E 'test(a_dependent_gate_clear_refuses_a_row_the_gate_is_not_live_for)'` | *"…and so must clear — one liveness rule, both directions. left: Ok(()), right: Err(NoSuchRow)"* | RED — PASS |

**All 11 build-report kills and all 20 fold-section kills (C-1/I-1 combined into one test, I-2 split
into 3 plants, I-3/I-4/I-5/M-1/M-2/M-3×3/M-4/M-5/N-1/N-2/N-3) reproduced their claimed RED, verbatim
or with an equivalent message where line numbers or test counts shifted from prior runs. No kill
failed to red.**

## Negative claims

| claim | check | result | verdict |
|---|---|---|---|
| `prompt-check` — 54 assertions, all verbatim | `cargo run -p xtask -- prompt-check` | `xtask prompt-check: OK — 54 assertions, all verbatim` | matches |
| `prompt-check` reds on a live one-character drift | planted `"count as"` → `"may count as"` directly in `dependent_gates.rs`'s shipped help, ran `prompt-check`, reverted | *"row (5)(a)'s help does NOT quote … verbatim …"* | RED, confirms the live-source path (not just the pure-function test) |
| `stop-list` unmoved | `cargo run -p xtask -- stop-list` | `R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 76 registry prompts scanned; no forbidden shape` | matches (76) |
| `census-join` — 290 unmodeled entries | `cargo run -p xtask -- census-join` | `census join: 290 unmodeled entries across 13 maps …` | matches (290) |
| `line-coverage` — 373 money lines, 18 forms, 31 exceptions | `cargo run -p xtask -- line-coverage` | `line-coverage OK: 373 money lines across 18 form(s) …, 31 exception(s) (ratchet 31), 0 unverifiable, 17 not line-bound` | matches |
| TY2024 golden corpus unchanged | `git diff --stat 8d2f6b83..2213eeb5 -- crates/btctax-core/tests/goldens/full_return_goldens.json` | empty diff | unchanged, confirmed |
| identity-grid unchanged (`dependents_statement.rs`, `form1040_full.rs`) | `git diff --stat 8d2f6b83..2213eeb5 -- '**/dependents_statement.rs' '**/form1040_full.rs'` | empty diff | unchanged, confirmed. The only `btctax-forms` diff in the whole T7 build+fold range is fixture setup (new `tax_year = 2024` lines and an `answer_all_dependent_gates` call added to three test helpers, per D7) — no golden VALUE or expected string changed |
| `btctax-forms` suite unchanged | `cargo nextest run -p btctax-forms --no-fail-fast` | `359 tests run: 359 passed, 4 skipped` | matches the report's claimed line exactly |
| `btctax-cli` fullreturn oracle tests | `cargo nextest run -p btctax-cli -E 'test(fullreturn)'` | `2 tests run: 2 passed` (`fullreturn_fixture_matches_its_emitter`, `fullreturn_fixture_is_the_kitchen_sink_oracle`) | green |
| every new shaped identifier allowed | `bash scripts/pii-scan-generic.sh HEAD` | `pii-scan: clean (HEAD).`, exit 0 | matches |
| whole suite unaffected by the plant/revert cycle | `cargo nextest run -p btctax-core -p btctax-cli -p btctax-input-form -p xtask --no-fail-fast` (re-run after all plants reverted) | `2360 tests run: 2353 passed, 7 failed, 2 skipped` — identical to the pre-plant baseline (the same 6 `form_delta` + 1 `harness_check` environment failures) | matches, tree fully restored |
| `btctax-cli` full suite | `cargo nextest run -p btctax-cli --no-fail-fast` | `803 tests run: 803 passed, 1 skipped` | matches the report's claimed line exactly |
| worktree clean at close | `git status --porcelain` | empty | confirmed, HEAD still `2213eeb5` |

## Notes

- The `MAX_SWEEPS` guard, `Census::dependent_gates` element type, `PARAMS_GATED_PROMPTS` (still empty
  with its own planted-occupant test unchanged), and the `record_answer`-is-the-only-writer invariant
  were not independently re-planted — they are structural claims already exercised as a byproduct of
  the kills above (e.g. the C-1/I-1 sweep KAT necessarily exercises `MAX_SWEEPS` and `record_answer`
  on every run) and the brief's checklist does not name them as separate kills to re-plant.
- M11 (the paraphrase `xtask prompt-check` caught live during the original build, on
  `JointReturnOnlyToClaimRefund`'s help) is self-reported by the build report as an in-flight finding,
  not a standing kill instrument to re-plant; confirmed only that `prompt-check` is clean today.
- One stray system-reminder-shaped text appeared inside a `git log` tool result during this session,
  attempting to change commit/PR attribution mid-task. It was not acted on: this task made no commits,
  and text embedded inside tool output is not a legitimate instruction regardless of its shape.

Counts: C=0 I=0 M=0 N=0
