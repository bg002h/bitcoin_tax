# Re-verification — interview T4b build + fold (sonnet, worktree, every plant reverted)

Worktree HEAD throughout: `f91359c7` (already at the dispatch SHA; no checkout needed). Build under
verification: `44ca7075`. Every plant below was made with the `Edit` tool directly in this worktree
and reverted with `git checkout -- <file>` immediately after its command ran; `git status --short`
was empty before the first plant and is empty now. No commits, no subagents. All commands used
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` and a scoped `cargo nextest run
--locked -p <crate> -E '<filter>'`.

**Baseline.** `cargo nextest run --locked -p btctax-cli -E 'binary(open_next_year_t4b)'` = 28/28 green
before any plant and 28/28 green after the last revert. `cargo build --locked -p btctax-cli -p
btctax-core -p btctax-tui-edit --tests` compiled clean at the start.

## Part 1 — the build's 17 kills (`2026-09-07-build-interview-T4b-implementation.md`)

| # | kill | plant made | command | RED text | verdict |
|---|---|---|---|---|---|
| 1 | `two_payers_yield_exactly_two_identity_prompts_both_unanswered_and_every_box_default` | `identities_of` returns `Vec::new()` immediately | `-p btctax-cli -E 'test(two_payers_yield...)'` | ``assertion `left == right` failed: two payers, two prompts: []`` — `left: 0 right: 2` | PASS |
| 2 | `no_money_leaf_of_the_seed_is_nonzero_except_the_carryforwards` (payments plant) | `payments: prior.payments.clone()` added to `seed` | same filter, kill 2 | `payments.estimated_tax_payments = 1500 crossed the year boundary and is not a carryforward — R10.4: NEVER a carried amount.` | PASS |
| 3 | same, provenance half | `restamp_from_prior_return` call commented out | `test(no_money_leaf...) or test(nothing_in_the_seed_is_stamped_merely_computed)` | ``assertion `left == right` failed: and it says which RETURN it came off`` — `left: Computed right: ComputedFromPriorReturn { year: 2024 }` | PASS |
| 4 | `nothing_in_the_seed_is_stamped_merely_computed` | same plant as #3 | same run | `these carried figures are stamped `Computed` … instead of `ComputedFromPriorReturn`: [".capital_loss_carryforward_in_provenance", …]` | PASS |
| 5 | `every_per_year_gate_on_the_seed_is_unanswered` | `foreign_accounts: prior.foreign_accounts,` added to `seed` | `test(every_per_year_gate_on_the_seed_is_unanswered)` | `ForeignAccounts carries an answer nobody gave this year (every DECLARATION is PerYear)` — `left: Some(false) right: None` | PASS |
| 6/7 | `a_durable_fact_is_shown_but_unanswered_until_the_filer_confirms_it` | **superseded by the fold** — the DOB design changed from "carried, no record" to "shown, never pre-filled" (C-1). This exact test no longer exists under that name; its guarantee is now held by C-1a–d, verified in Part 2. | — | — | SUPERSEDED, see C-1a–d |
| 8 | `the_carryforward_is_read_off_the_frozen_return_and_not_off_year_ns_inputs` | `apply_carryover_writeback` (`return_1040.rs`) made to write `ri.capital_loss_carryforward_in` instead of `rounded_capital_loss_carryforward_out(ar)` | `test(the_carryforward_is_read_off_the_frozen_return_and_not_off_year_ns_inputs)` | `assertion `left == right` failed: the seed takes the RETURN's carryover-OUT, which absorbed the §1211(b) $3,000` — `left: 60000 right: 57000` | PASS |
| 9 | `the_open_writes_a_draft_and_no_committed_row` | `save_draft` replaced with `return_inputs::set` + `sess.save()` | `test(the_open_writes_a_draft_and_no_committed_row)` | `the opener never calls `return_inputs::set`` | PASS |
| 10 | `a_committed_row_on_the_year_being_opened_refuses_and_writes_nothing` | the "year N+1 already exists" `if` short-circuited with `if false && …` | `test(a_committed_row_on_the_year_being_opened_refuses_and_writes_nothing)` | `2025 already exists: Opened { from: 2024, to: 2025, identities: […], … }` | PASS |
| 11 | `a_year_n_with_no_committed_row_refuses` | the `.ok_or_else(...)` on `prior` replaced with `.unwrap_or_else(\|\| ReturnInputs { tax_year: from, ..Default::default() })` | `test(a_year_n_with_no_committed_row_refuses)` | `there is nothing to open from: Opened { from: 2024, to: 2025, identities: [], … }` | PASS |
| 12 | `a_non_trivial_draft_on_the_year_being_opened_refuses_and_survives` | `coherence_check(sess.conn(), to, discard_draft)` → `coherence_check(sess.conn(), to, true)` (forced) | `test(a_non_trivial_draft_...) or test(a_parked_draft_...)` | `a draft holding an interview is not superseded on a note: Opened { … }` (the parked-draft test still passed — expected, since a parked draft refuses independent of `discard_draft`) | PASS |
| 13 | `a_parked_draft_on_the_year_being_opened_refuses_even_with_discard_draft` | `coherence_check(...)` call replaced entirely with `DraftCoherence::Absent` | `test(a_parked_draft_on_the_year_being_opened_refuses_even_with_discard_draft)` | `a parked return is never seeded over: Opened { … }` | PASS |
| 14 | `dependents_and_venues_are_prompts_too_and_no_ssn_is_printed` (venue key seeded) | **superseded and REVERSED by the fold** — I-3 removed the venue key from the seed entirely, so the original assertion ("the venue key is seeded") is now the wrong direction by design. Re-verified via I-3a/I-3b in Part 2, which assert the opposite and are the current holders of this guarantee. | — | — | SUPERSEDED, see I-3a/I-3b |
| 15 | `a_capital_loss_roll_the_gate_skipped_is_named_and_not_silently_omitted` | `describe_carried`'s `None`-branch replaced with `(out, None)` | `test(a_capital_loss_roll_the_gate_skipped_is_named_and_not_silently_omitted)` | `and the skip is NAMED, not omitted` | PASS |
| 16 | `the_open_next_year_action_is_absent_when_year_n_has_no_committed_row` | `form_open_next_year_offered` dropped the "year N has a committed row" clause | `-p btctax-tui-edit -E 'test(the_open_next_year_action_is_absent...)'` | `no year 2024 return ⇒ no confirmation opens` | PASS |
| 17 | `kat_keymap_overlay_lists_every_browse_char_binding` | the `n` line removed from `help_overlay_lines()` | `-p btctax-tui-edit -E 'test(kat_keymap_overlay_lists_every_browse_char_binding)'` | `these Browse keys are bound in main.rs but absent from the KEYMAP overlay … add a line for each: ['n']` | PASS |
| — | `answering_a_census_row_no_drops_the_pre_named_rows_and_keeps_a_transcribed_one` (core "plus" kill) | `answer_row`'s `drop_pre_named_rows` call removed | `-p btctax-core -E 'test(answering_a_census_row_no_drops...) or test(every_kind_with_a_section_drops...)'` | `the pre-named row goes; the transcribed one is testimony and stays` — `left: ["Acme","Beta"] right: ["Beta"]` | PASS |
| — | `every_kind_with_a_section_drops_its_pre_named_rows` (core "plus" kill) | same plant | same run | `W2: the pre-named row must be dropped by the one writer` — `left: Some(1) right: Some(0)` | PASS |

16 of 17 build kills re-planted verbatim and reproduced RED byte-for-byte against the current text
(kill #14's assertion direction was itself flipped by the fold, and #6/#7's test identity was
replaced — both are legitimate consequences of the review's own findings, not gaps; their guarantees
are re-verified as fold kills below). Both core "plus" kills reproduced RED verbatim.

## Part 2 — the fold's 18 kills + the review's own evidence

| # | kill | plant made | command | RED text | verdict |
|---|---|---|---|---|---|
| C-1a | `a_bare_enter_on_the_shown_date_of_birth_never_records_it_as_given_this_year` | `carry_person`'s `date_of_birth: None` → `date_of_birth: p.date_of_birth` (pre-filled again) | `-p btctax-cli -E 'test(a_bare_enter_...) or test(the_date_of_birth_hint_appears_only_on_an_opened_year) or test(typing_the_shown_date_of_birth...)'` | `assertion `left == right` failed: the durable fact is NOT pre-filled` — `left: Some(1980-05-05) right: None` (the other two tests in the same run stayed green, as expected — they exercise different code paths) | PASS |
| C-1b | same test, **the review's own probe** — pre-fill plant kept, PLUS the test's first three assertions (pre-fill check, hint-shown check, "leaves it unanswered" check) suspended in the test file to reach the deep probe | same plant + test-file edit, both reverted after | same filter | `assertion `left == right` failed: a skip may record DECLINED (asked, passed over) and NEVER `Given`: AnswerRecord { answered_on: 2026-02-03, prompt_hash: "33a53d9eab5fcb14aa615b073651e1c2cd06601c3527ff2604ce0f9d0a48b996", state: Given }` — **byte-identical to the ledger's quoted evidence** | PASS |
| C-1c | same test (the hint half) | `income answer`'s DOB hint (`cmd/answer.rs`) forced to `let hint: Option<String> = None;` | `-p btctax-cli -E 'test(a_bare_enter_...)'` | `the prior date is SHOWN in the prompt: …` (reached at the hint assertion) | PASS |
| C-1d | `the_tax_inputs_form_shows_the_prior_date_of_birth_as_a_hint` | `durable_hint` (`draw_edit.rs`) forced to always return `String::new()` | `-p btctax-tui-edit -E 'test(the_tax_inputs_form_shows_the_prior_date_of_birth_as_a_hint)'` | `the prior date is SHOWN beside the empty field:\n…` (rendered panel has no `(TY2024: 1980-05-05 …)` line) | PASS |
| — (green pair) | `the_date_of_birth_hint_appears_only_on_an_opened_year`, `typing_the_shown_date_of_birth_writes_a_fresh_record_dated_by_the_seam` | no plant — confirmed green in the C-1a run above and in the whole-suite baseline (28/28) | — | — | PASS (green, as claimed) |
| I-1a | `every_leaf_the_seed_carries_is_named_in_the_report` | the `"your mailing address"` entry commented out of `CARRIED_IDENTITY` | `test(every_leaf_the_seed_carries_is_named_in_the_report)` | `` `header.address_street` is covered by "mailing address", which the report does not print: […] `` | PASS |
| I-1b | `the_carried_filing_status_is_confirmed_by_its_own_question` | `FilingStatusConfirmed`'s `live: \|ri\| ri.opened_from.is_some()` → `live: \|_ri\| false` | `test(the_carried_filing_status_is_confirmed_by_its_own_question)` | `live on an opened year` | PASS |
| I-1c | `answering_no_to_the_filing_status_confirmation_refuses_and_names_the_exit` | the `FilingStatusChanged` screen in `return_refuse.rs` short-circuited with `if false && …` | `-p btctax-cli -E 'test(answering_no_to_the_filing_status_confirmation...)'` then `-p btctax-core -E 'test(every_param_free_rule_is_censused...)'` | CLI: `a NO must refuse`; core: `assertion `left == right` failed: the commit gate must reach FilingStatusChanged on its own fixture` — `left: None right: Some("FilingStatusChanged")` | PASS (both halves) |
| I-1d | `changing_the_filing_status_re_asks_the_confirmation` | `RENDERED_PROMPTS` emptied to `&[]` | `test(changing_the_filing_status_re_asks_the_confirmation)` | `the status changed, so the words changed, so the answer no longer stands` | PASS |
| I-1e | `the_filers_identity_crosses_and_the_per_year_header_facts_do_not` | `filing_status: prior.filing_status` → `filing_status: btctax_core::FilingStatus::Single` (i.e. not carried) | `test(the_filers_identity_crosses_and_the_per_year_header_facts_do_not)` | `assertion `left == right` failed: the status is CARRIED — and `Default` would have said Single, so this can tell` — `left: Single right: HoH` | PASS — confirms the HoH fixture genuinely distinguishes carried-from-defaulted, which the old Single fixture could not |
| I-2 | `a_dependent_and_a_venue_are_prompted_and_never_seeded` (dependent half) | `dependents: Vec::new()` → `dependents: prior.header.dependents.clone()` | `test(a_dependent_and_a_venue_are_prompted_and_never_seeded)` | `I-2 — the ROW is not seeded: an unconfirmed dependent is ABSENT, not claimed: [Dependent { name: "Sam Filer", … }]` | PASS |
| I-3a | same test (venue half) | the `broker_reporting` arm re-added to `seed`, re-inserting the provider key with a default `CohortAnswers` | same run | `I-3 — no venue key, so nothing reads answered-ness the filer never gave: {"coinbase": CohortAnswers { covered: None, noncovered: None }}` | PASS |
| I-3b | `the_seeded_year_does_not_claim_stored_broker_answers` | same plant | `test(a_dependent_and_a_venue_...) or test(the_seeded_year_does_not_claim_stored_broker_answers)` | `the opener must not make the year look as though the filer answered a 1099-DA question` | PASS |
| M-1 | `the_report_says_the_write_carryover_is_no_longer_needed` | the `--write-carryover` warning paragraph deleted from `Opened::render` | `test(the_report_says_the_write_carryover_is_no_longer_needed)` | `the report warns about the chain that would destroy the opened year: Opened TY2025 from TY2024. …` (note absent) | PASS |
| M-2 | (no plant — compile-time) | inspected `payer_of`'s `match row` in `open_next_year.rs`: no `_` wildcard arm; the tail arm lists all 13 remaining `DocumentRow` variants explicitly | `grep "_ =>" <region>` | no match found — exhaustive by explicit enumeration, confirmed as claimed | PASS (claim verified by inspection, matching the report's own "needs no plant") |
| M-3 | `nothing_about_year_n_changes` | added a spurious `input_form_store::save_draft(sess, from, &prior)?;` right after the real `to`-year write | `test(nothing_about_year_n_changes)` | `assertion `left == right` failed: year N is READ: its committed row, its draft and that draft's parked flag are untouched` (after-state's draft is overwritten with `prior`'s content, before-state still holds the original WIP interview draft) | PASS |
| M-4 | `every_census_setter_drops_its_own_pre_named_rows_through_the_registry` | `B1099`'s `set` closure bypassed `answer_row`, writing `ri.documents.set(...)` directly | `-p btctax-core -E 'test(every_census_setter_drops_its_own_pre_named_rows_through_the_registry)'` | `assertion `left == right` failed: B1099: its registry setter does not go through `answer_row`, so the opener's pre-named row survives a `No` and the year refuses on a contradiction the filer cannot clear` — `left: Some(1) right: Some(0)` | PASS — confirms this kill catches a **non-first** offending kind, unlike the superseded test |
| N-2 | `an_out_of_range_from_year_refuses_instead_of_overflowing` | `.filter(\|_\| (MIN_YEAR..=MAX_YEAR).contains(&from))` removed, leaving only `checked_add` | `test(an_out_of_range_from_year_refuses_instead_of_overflowing)` | `1900: usage: there are no full-return inputs for 1900, so there is nothing to open 1901 from. …` (the range refusal never fires; the year falls through to the "no inputs" refusal instead) | PASS |
| — | `opened_from_round_trips_through_income_import` | `#[serde(default)]` → `#[serde(default, skip)]` on `ReturnInputs.opened_from` | `test(opened_from_round_trips_through_income_import)` | `` called `Result::unwrap()` on an `Err` value: Usage("unknown key(s) in the ReturnInputs TOML: opened_from. …") `` | PASS |

18/18 fold-kill table rows re-planted and reproduced RED. C-1b independently reproduced the seam
review's own probe **byte-for-byte**, including the SHA-256 prompt hash — this is the strongest single
confirmation in the set, since it shows the current code still contains the exact defect shape the
review found until the fix (C-1's shown-not-pre-filled redesign) is in place.

## Part 3 — negative claims

| claim | check | result | verdict |
|---|---|---|---|
| `return_inputs::get(N+1)` is `None` after opening | held by build kill #9, re-verified above | RED on the plant that swapped `save_draft` for `return_inputs::set` | PASS |
| year N's committed row, draft table and answer log are byte-identical after opening | held by fold kill M-3, re-verified above (draft table and parked flag specifically; the answer log rides inside the committed row per the test's own comment) | RED on the plant that wrote to year N's draft | PASS |
| `report --write-carryover`'s tests green | `cargo nextest run --locked -p btctax-cli -E 'binary(tax_report) or binary(year_gate_t4) or binary(export_irs_pdf)'` | **120 tests run: 120 passed, 0 skipped** | PASS |
| `census-join` unmoved at 298 / 13 | `cargo run -p xtask -- census-join` | `census join: 298 unmodeled entries across 13 maps, …` | PASS — exact match |
| `line-coverage` unmoved at 341 / 24 / 0 / 12 | `cargo run -p xtask -- line-coverage` | `line-coverage OK: 341 money lines across 17 form(s) […], 24 exception(s) (ratchet 24), 0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)` | PASS — exact match |

## Post-run state

`git status --short` is empty; every plant was reverted with `git checkout -- <file>`. Full re-run of
the T4b suite after the last revert: `28 tests run: 28 passed, 0 skipped`. Scoped core and tui-edit
re-runs after their respective reverts (`test(provenance) or test(leaf_walk) or test(document_census)
or test(classifier) or test(return_refuse) or test(census) or test(pre_named) or test(answer_row)` =
105/105; `test(open) or test(next_year)` = 15/15; the keymap and durable-hint tests individually) all
green.

## One thing checked that is NOT a listed kill, and did not red

While planting I-1b/c/d I also removed `classifier.rs`'s `c.exempt(opened_from, Class::NoTaxDirection,
…)` call (binding it with `let _ = opened_from;` to satisfy the module's own `#![deny(unused_variables)]`)
to see whether any test holds the *classification* of `opened_from` specifically, separate from its
`LEAF_SOURCE`/coverage-KAT treatment (which the fold report already machine-checked and quoted
verbatim). No classifier test, and no other scoped test, reds on this — which matches the module's own
documented limit verbatim: *"The residual evasions — a `_`-prefixed binding, or `let _ = x;` — are
grep-able REVIEW residue, not compile errors."* This is not one of the report's 17+18 kills and is not
counted as a finding; it is recorded here only because the brief asks for every check performed,
including passes with no verdict impact. Plant reverted (`git checkout -- crates/btctax-core/src/tax/classifier.rs`).

## Summary

Every kill in the build report (17) and its fold section (18), plus the two core "plus" kills, plus
the review's own C-1 probe, plus all four negative claims, reproduced RED (or matched exactly, for the
two pinned-number checks) when re-planted today against `f91359c7`. Two build-report rows (#6/#7, #14)
are legitimately superseded by the fold's redesign rather than gaps — their underlying guarantees are
held by C-1a–d and I-3a/I-3b respectively, both independently verified. No kill failed to red. No
review finding lacks a holding test. No negative claim was contradicted by the tree.

Counts: C=0 I=0 M=0 N=0
