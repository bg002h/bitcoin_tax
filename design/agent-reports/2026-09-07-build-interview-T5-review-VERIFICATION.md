# Verification ledger — interview T5 seam review (`2026-09-07-build-interview-T5-review.md`, 0C/4I/4M/1N)

Controller machine-checks from the main tree at `adeb91bf`, before any fold.

| # | claim | check run | measured | decision |
|---|---|---|---|---|
| I-1 | gutting `field_join_failures()` leaves every `box_census` test green | planted `if true { return Vec::new(); }` at the fn's first line; `cargo nextest run -p xtask -E 'test(box_census)'`; reverted | `12 tests run: 12 passed` | true — the B1 kill is a shadow; **FOLD** (`join_failures_for(entries)` and a kill that calls it) |
| I-2 | the filer's-records rule covers only the EMPTY direction; the rows sum unconditionally; no `Contradicted` variant exists | grep | `FilerRecordsDeclaredNotTranscribed` at `return_refuse.rs:511,1539`; `FilerRecordsContradicted` 0 hits; `printed.rs:1331,1367` and `return_1040.rs:608` sum `schedule_b_filer_records` unconditionally | true; **FOLD** (the mirror refusal for `Some(false)`-with-rows and for a closed door with orphan rows; the section stays visible while it has rows so they can be removed) |
| I-3 | the 1099-DIV limb of `InterestOrDividendsWithout1099.live` has no kill | the review's plant (whole-suite green on the deleted limb) and the (f1) loop's shape | accepted on the review's quoted evidence; the mechanism (only `Int1099` probed; the `Some(true)` probe closes both rows) is a reading of the test | true; **FOLD** (the asymmetric probe) |
| I-4 | `occupation_on_treasury_list` / `excludes_unlisted_occupation_tips` / `meets_qualified_tip_criteria` have no reader; the classifier's exemption reason claims a behaviour that does not exist | `grep -rn` over core (non-test) | readers only in `classifier.rs:687-703` (the exemptions) and `return_inputs.rs:959,969` (the struct); `classifier.rs:695` states the Caution | true and pre-existing; **FOLD the fail-closed half now** (refuse when `qualified_tips_reported > 0` and any condition is `false`, Caution quoted; the classifier text corrected) — the compute gating stays FR-72 (Schedule 1-A's track) |
| M-1 | income boxes on the same 1099-G (5, 6, 7, 9) and 1099-B box 13 are `NotRead` while box 10 refuses | read of `box_census.rs` | true | fold: an income box with no reader fails closed — `RefuseIfNonzero` naming its line, consistently |
| M-2 | the box 4/6 warnings fire on a lawful W-2 with box 12 codes A/B | read | true | fold inline (suppress when box 12 carries A or B) |
| M-3 | door questions land after the skippables; a question dead mid-round is still asked | read | true | fold: group the door questions with the census in `live_questions`' partition; re-check liveness before asking |
| M-4 | D-1's fix spelled out per site rather than through `current_prompt` | read | `return_refuse.rs:1495`, `interview_state.rs:200` | fold inline |
| N-1 | dead loop in `undated_document_rows` | read | `provenance.rs:150-155` | fold inline |
| (a)–(d) | the controller's four questions | the review's answers | D-1 confirmed on all counts, mutation-covered; the sweep cannot double-ask/skip/loop; I-4 answers (c); the six warning fixtures are the check working | recorded |

## Disposition
- **I-1 — FOLD.** `pub fn join_failures_for(entries: &[BoxEntry]) -> Vec<String>` holds the body;
  `field_join_failures()` = `join_failures_for(BOXES)`; the negative test builds planted tables and
  asserts `!join_failures_for(&planted).is_empty()` with the message checked — a kill that cannot be
  written without the checker being real. Then the controller's plant (gut the body) must red.
- **I-2 — FOLD.** `RefuseReason::FilerRecordsContradicted` in `screen_inputs_tiered` beside the
  empty-rows rule: rows present and NOT (door live ∧ `Some(true)`) ⇒ refuse with the both-ways
  message `DocumentCensusContradicted` uses; the section's liveness = door open OR rows present, so
  orphan rows stay visible and removable in the TUI; kills for both states.
- **I-3 — FOLD.** The asymmetric probe in (f1) (`Int1099 = Some(true)`, `Div1099 = Some(false)` ⇒
  live); deleting the DIV limb reds.
- **I-4 — FOLD the refusal + the classifier text; FR-72 keeps the compute.**
- **M-1 → M-4, N-1 — fold inline.**

Nothing is folded at the time of this ledger. Fold brief: `BRIEF-fold-interview-T5-review.md`.
