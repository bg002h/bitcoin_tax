# Re-verification — interview T4 build (`bbcee739`) + fold (`4ad17aaa`)

Independent verifier, own worktree, checked out to `68467dde` (the brief commit) as instructed. No
commits, no subagents. Every plant below was made with `cp`-backup / restore in this worktree and
reverted before the next plant; `git status --short` is clean at both the start and the end of this
pass (confirmed above and below).

Build setup used throughout:
```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review
```
Baseline (unplanted) scoped suites, confirmed green before any plant:
- `-p btctax-cli -E 'binary(year_gate_t4) | test(input_form_store) | test(answer)'` → 74/74
- `-p btctax-core -E 'test(param_free) | test(return_refuse)'` → 63/63 (includes 5/5 `param_free_tier`)
- `-p btctax-tui-edit -E 'test(discard) | test(gate)'` → 6/6

Settled per the brief (not re-measured): the seven `form_delta`/`harness_check` environment failures in
a PDF-less worktree; the build/review/ledger/fold-brief citations.

## Kills — one row per plant, reverted after each

| # | kill (report §) | plant made | command | RED text (or "did not red") | verdict |
|---|---|---|---|---|---|
| 1 | §1 TUI entry-screen states both gates | removed the `for g in &gate_lines { … }` render loop in `draw_edit.rs::draw_tax_inputs_status` | `-p btctax-tui-edit -E 'test(the_tax_inputs_entry_screen_states_both_year_gate_states)'` | `panicked at crates/btctax-tui-edit/src/main.rs:12690:9: a year WITH its package states both, and the second is yes:` | RED, exact match |
| 2 | §2 import refuses a draft holding an interview | removed the `Err(CliError::NonTrivialDraftBlocksWrite{…})` arm in `coherence_check` (superseded on a note unconditionally) | `-p btctax-cli -E 'test(import_over_a_draft_holding_an_interview…) or test(import_over_a_broker_answers_draft…) or test(income_clear_over_a_broker_answers_draft…) or test(the_carryover_write_back_shares…) or test(a_disposable_draft_is_still_superseded…) or test(coherence_check_refuses_a_draft…)'` | 5/6 reds, incl. `a draft holding an interview is not superseded on a note: ()` / `a draft holding the 1099-DA answers is not superseded on a note: ()` / ``income clear` must not destroy the 1099-DA answers on a note: false``; `a_disposable_draft_is_still_superseded_without_a_flag` correctly stayed green (single choke point) | RED, exact match |
| 3 | §2 stale-WIP discard refuses a draft holding an interview | removed the `if !draft_is_disposable(&d.ri) { return Err(StaleDraftHoldsInterview…) }` block in `load` | `-p btctax-cli -E 'test(a_stale_wip_draft_holding_an_interview…) or test(a_stale_broker_answers_draft…) or test(a_stale_wip_draft_holding_nothing…)'` | `a stale draft holding an interview must not be discarded`; `the §6.3 stale-WIP discard destroyed a draft holding the 1099-DA answers`; the paired "holding nothing" test correctly stayed green | RED, exact match |
| 4 | §3 `income answer` writes the draft on a draft-only year | replaced the `Loaded::Draft{ri, parked:false} => Ok(AnswerTarget::Draft(ri))` arm with a `Usage` error | `-p btctax-cli -E 'test(income_answer_on_a_draft_only_year…) or test(income_answer_still_refuses_a_year_with_neither…)'` | `a draft-only year is answerable: Usage("PLANT: no draft target")`; the both-absent test correctly stayed green | RED, exact match |
| 5 | §4 rule deleted from the param-free tier (`ForeignTrust`) | deleted the `if ri.foreign_trust == Some(true) { return refuse(…) }` block entirely | `-p btctax-core -E 'test(param_free_tier)'` | `assertion left == right failed: …fixtures, right: source)` (left 31, right 30) AND `ForeignTrust must refuse on its own fixture` | RED, exact match (2 tests caught it) |
| 6 | §4 rule moved inside a `tier.package` gate | wrapped the same block in `if let Some((_tbl,_p)) = tier.package { … }` | same filter | `assertion left == right failed: … ForeignTrust must fire there too (left: None, right: Some("ForeignTrust"))` AND the fixture-must-refuse panic | RED, exact match (2 tests) |
| 7 | §4 unanswered tier ungated | changed `if tier.unanswered_refuses {` to `let plant_always_run_the_unanswered_loop = true; if plant_always_run_the_unanswered_loop {` (first attempt embedded the literal search string in a comment and produced a false-negative on one of the two tests — see Observation below; the clean plant reds both) | same filter | `the unanswered tier is gated` (source-census test) AND `` `income import` is the only path that CREATES the row `income answer` fills — it must not demand the answers `` (behavioural test) | RED, exact match (both tests, matching the report's two quoted panics verbatim) |
| 8 | §5 I-11 `NoTables` guard removed | removed the `let (Some(table),Some(params)) = … else { return Ok(NoTables) }` guard, proceeding to an unscreened write | `-p btctax-cli -E 'test(commit_on_a_params_less_year…) or test(commit_with_another_years_tables…)'` | `a params-less year cannot commit`; `assertion failed: matches!(outcome, CommitOutcome::NoTables)` | RED, exact match (both) |
| 9 | §5 per-YEAR table check removed (isolated) | removed only `if table.year != year \|\| params.year != year { return Ok(NoTables) }`, guard #8 left intact | same filter | `commit_with_another_years_tables_writes_nothing` reds at line 255; `commit_on_a_params_less_year_writes_no_row_and_keeps_the_draft` correctly stayed green (doesn't exercise this branch) | RED, exact match, correctly isolated |
| 10 | Fold C-1 — predicate re-narrowed to the four categories | `draft_is_disposable` body replaced with `draft_holdings(ri).is_empty()` | `-p btctax-cli -E 'test(import_over_a_broker_answers_draft…) or test(income_clear_over_a_broker_answers_draft…) or test(a_stale_broker_answers_draft…) or test(the_carryover_write_back_shares…) or test(a_draft_differing_from_the_seed…)'` | all 5 red, incl. the structural kill `a_draft_differing_from_the_seed_in_one_uncounted_field_is_not_disposable` (breaking the predicate itself hits every consumer, a stronger result than the report's "4 of 5" since that was against an *unwired* predicate, not a broken one) | RED, exact match (stronger) |
| 11 | Fold M-1 — parked wording restored | `if form.discard_is_parked` → `let plant_always_parked = true; if plant_always_parked` | `-p btctax-tui-edit -E 'test(discard) \| test(gate)'` | `panicked at crates/btctax-tui-edit/src/main.rs:12784:9: a WIP draft holding work is NOT parked — the word must not appear:` | RED, exact match |
| 12 | Fold M-2 — `income answer` exit restored in `SalesTaxElectionWithoutAmount` | restored the two-exit wording (`income import` **or** `income answer`) | `-p btctax-core -E 'test(param_free_tier) or test(sales_tax_election_without_amount_refuses)'` | `these import-tier refusals prescribe `income answer`, which a refused import cannot reach: ["SalesTaxElectionWithoutAmount"]` AND `the second exit must be reachable from the TOML: …` | RED, exact match (both, incl. the reversed assertion) |
| 13 | Fold M-3 — read-only mapping arm removed | `load_for_read` body replaced with a bare `load(conn, year)` (no `StaleDraftHoldsInterview` → note mapping) | `-p btctax-cli -E 'test(a_read_only_surface_continues…) or test(the_stale_draft_split_holds_on_the_export_path)'` | `a reader continues: StaleDraftHoldsInterview { year: 2026, found: 1, expected: 3, holdings: "work not otherwise itemised" }` AND (in `slice_from_answers`) the same error surfaced as an `Err` | RED, exact match; confirms the report's "un-breaks `slice_from_answers`" claim |
| 14 | Fold N-1 — `\` continuations removed | removed the two `\`-newline continuations in the `NonTrivialDraftBlocksWrite`/`StaleDraftHoldsInterview` `#[error(…)]` literals | `-p btctax-cli -E 'test(the_two_new_refusal_messages_carry_no_collapsed_line_breaks)'` | `a missing \`\\\` continuation collapses into a run of spaces: "…and this write would\n         discard it…"` | RED (assertion is on any run of 2+ spaces; my plant produced newline+indent rather than the report's pure-space run since Rust, unlike the original Python-authoring accident, keeps a literal `\n` — same test, same assertion, same verdict) |

## Pointed check — does any OTHER param-free rule's detail still name `income answer`, and does the grep-KAT actually red on it?

Full grep of `income answer` in `return_refuse.rs` (14 hits): the only occurrence inside a rule's own
refusal text besides `SalesTaxElectionWithoutAmount` (already fixed) is `WORDING_CHANGED_DETAIL`
(line 508-512), which fires **only** from inside the `if tier.unanswered_refuses {` loop — exempt by
construction, and correctly so per the KAT's own doc comment (line 5036-5037). The rest are comments
(explaining the fix) or the KATs' own source/assertions.

`no_refusal_in_the_import_tier_prescribes_income_answer` (`return_refuse.rs:5038`) iterates **all 31**
`param_free_fixtures()` through `screen_param_free`, not just the one M-2 named — confirmed by reading
the loop body. Planted an `income answer` mention into an unrelated param-free rule
(`ScheduleCNoBusinessDescription`'s detail, appended `". PLANT: or run \`btctax income answer\` instead"`)
and reran `-p btctax-core -E 'test(param_free_tier)'`:

```
thread '…::no_refusal_in_the_import_tier_prescribes_income_answer' panicked at
crates/btctax-core/src/tax/return_refuse.rs:5049:9:
these import-tier refusals prescribe `income answer`, which a refused import cannot reach:
["ScheduleCNoBusinessDescription"]
```

**Verdict: PASS.** No other param-free rule currently names `income answer` as an exit, and the KAT is
genuinely a sweep over the whole tier (behavioural on `.detail`, not a grep limited to one rule) — it
reds on a plant anywhere in the 31-rule set, not only on the specific row the review flagged.

## Observation (not a finding against the T4 work) — a text-based brace matcher can be fooled by a comment

My first attempt at plant #7 wrote the replacement as `if true { // PLANT: … (was \`if tier.unanswered_refuses {\`)`,
embedding the old literal inside a comment for documentation. `every_unanswered_refusal_sits_inside_the_unanswered_gate`
still passed, because its `body.find("if tier.unanswered_refuses {")` and subsequent brace-counting do
not skip comments or string literals — they found the string inside my comment and brace-matched from
there, coincidentally landing on a valid-looking span. This is a real (if narrow) blind spot in that
one checker's mechanism: a future edit that removes the gate but leaves an explanatory comment quoting
the old code verbatim would not be caught. It is **not** a defect in the T4 fold or in any kill this
brief asked me to verify — the checker reds correctly on every plant that does not happen to re-embed
its own search string — and neither the build report nor the review claims robustness against this
shape. Recorded as a Nit for the record, not gating.

## Review's own evidence, re-planted after the fold (brief item 2)

All of the following are exercised, unplanted, by the baseline scoped runs above (all green), and each
underlying mechanism was separately confirmed to RED under the matching plant in the table:

| claim | test | baseline verdict |
|---|---|---|
| TY2026 broker+ScheduleC draft, `income import` (no `--discard-draft`) refuses, draft byte-identical | `import_over_a_broker_answers_draft_refuses_and_the_draft_survives_byte_identical` | PASS (and RED under plant #10) |
| same draft, `income clear` (no `--discard-draft`) refuses, draft byte-identical | `income_clear_over_a_broker_answers_draft_refuses_and_the_draft_survives_byte_identical` | PASS (and RED under plant #10) |
| same draft, `load` on stale schema refuses (not discarded) | `a_stale_broker_answers_draft_is_refused_not_discarded` | PASS (and RED under plants #3, #10) |
| `report --write-carryover` on year N behaves the same on N+1 | `the_carryover_write_back_shares_the_same_predicate_on_year_n_plus_1` | PASS (and RED under plant #10) |
| a draft differing from the seed in one uncounted field is non-disposable | `a_draft_differing_from_the_seed_in_one_uncounted_field_is_not_disposable` | PASS (and RED under plant #10) |
| the discard screen never says "parked" for an interview draft | `the_discard_only_screen_never_calls_an_unreadable_wip_draft_parked` | PASS (and RED under plant #11) |
| `income show-broker-answers`/`report` (via `working_return`/`broker_answers`) render with a note on a stale-interview-draft year, while `income import` on the same year still refuses with the draft byte-identical | `a_read_only_surface_continues_past_an_unreadable_draft_while_a_writer_still_refuses` — read the test body directly: it asserts `working_return` returns the committed row + a `KEPT`/`not deleted` note, `broker_answers` resolves, then `import_return_inputs` on the same year errors and `raw_draft_json` is unchanged | PASS (and RED under plant #13); confirms the claim at the level of the actual functions `income show-broker-answers`/`report`/`export-irs-pdf` share |

Also baseline-confirmed passing (paired/derivation tests the report names without a specific "red on
planted X" quote): `import_with_discard_draft_deletes_it_and_names_what_was_lost`,
`input_form_store::tests::the_confirmed_discard_note_names_what_was_lost`,
`draft_holdings_counts_answers_documents_dependents_and_a_schedule_a`,
`coherence_check_refuses_a_draft_holding_an_interview_and_deletes_nothing`,
`a_stale_wip_draft_holding_nothing_is_still_discarded_with_a_note`, and the 7 CLI-level `param_free`
tests — all ran green in the baseline scoped suites above.

## Negative claims (brief item 3)

| claim | check | result |
|---|---|---|
| `return_inputs::get(2026)` is `None` after `income answer` on a draft-only year | asserted inside `income_answer_on_a_draft_only_year_writes_the_draft_and_commits_nothing`, baseline PASS | confirmed |
| TY2024 golden corpus still files | `-p btctax-cli -E 'test(fullreturn)'` → `fullreturn_fixture_matches_its_emitter`, `fullreturn_fixture_is_the_kitchen_sink_oracle`, 2/2 passed | confirmed |
| `census-join` 298/13 unmoved | `cargo run -p xtask -- census-join` → `census join: 298 unmodeled entries across 13 maps` | confirmed exact |
| `line-coverage` 341/24/0/12 unmoved | `cargo run -p xtask -- line-coverage` → `341 money lines … 24 exception(s) (ratchet 24), 0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)` | confirmed exact |

## Final state

`git status --short` is clean (all 14 plants reverted via `cp`-backup restore, no `git checkout --`
used mid-sequence). No file left modified.

Every kill named in the T4 implementation report (§1–§5) and its appended fold section (C-1, M-1, M-2,
M-3, N-1) reds on its described defect when re-planted today. The one pointed check (whether the M-2
grep-KAT is a genuine sweep of the whole param-free tier, or blind to any rule but the one named) came
back clean: it is a genuine behavioural sweep over all 31 fixtures and reds on a plant anywhere in the
tier. No claim in the report or review was contradicted by the tree.

Counts: C=0 I=0 M=0 N=1
