# Brief — fold the interview T5 seam review (I-1 … I-4, M-1 … M-4, N-1)

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once; quote the red. Synthetic
identifiers: SSNs from the never-issued space (area 000/666, group 00, serial 0000); EINs only from
`scripts/pii-scan-generic.sh`'s `ALLOWED_EIN`.

## The contract
The review `design/agent-reports/2026-09-07-build-interview-T5-review.md` (read whole) and the
ledger `2026-09-07-build-interview-T5-review-VERIFICATION.md` — every claim there is
machine-verified; its Disposition is the fold list. The build you are folding:
`2026-09-07-build-interview-T5-implementation.md` (commits `34472416` + `9a845dc5`). B1 (`CLAUDE.md`,
"seen-red-once"): a kill is a test that RUNS THE INSTRUMENT and reds when the instrument is
removed — never a re-implementation of its predicates.

## The fold
1. **I-1 — the join gets a real kill.** In `crates/xtask/src/box_census.rs`: `pub fn
   join_failures_for(entries: &[BoxEntry]) -> Vec<String>` holds the current body of
   `field_join_failures`; `field_join_failures()` becomes `join_failures_for(BOXES)`. Rewrite
   `the_join_reds_on_a_field_from_the_wrong_section_and_on_a_reworded_caption` to build the three
   planted `BoxEntry` tables it describes and assert `!join_failures_for(&planted).is_empty()` with
   the message text checked for each. Kill (the controller's plant): `if true { return Vec::new(); }`
   at the top of `join_failures_for` → the test reds; the builder's M1 plant (`section_of_stem
   ("f1099int") => Some(SectionId::W2s)`) → `the_box_to_field_join_is_clean` reds. Quote both reds.
2. **I-2 — the mirror refusal.** `RefuseReason::FilerRecordsContradicted` (the exhaustive
   cross-crate `attribute` match forces its anchor) in `screen_inputs_tiered` beside
   `FilerRecordsDeclaredNotTranscribed`: `!ri.schedule_b_filer_records.is_empty() &&
   !(question_is_live(InterestOrDividendsWithout1099, ri) && … == Some(true))` ⇒ refuse with the
   both-ways message `DocumentCensusContradicted` uses (*"remove the rows, or answer yes"*). The
   section's liveness becomes *door open OR rows present* (`schedule_b_records_live`), so a closed
   door with orphan rows keeps them visible and removable in the TUI (the `add` guard stays on the
   door). Kills: state (a) `Some(false)` with one row refuses; state (b) both census rows
   `Some(true)` with one record refuses and the section is still visible; the (Y, one row) case
   passes and prints.
3. **I-3 — the second limb's kill.** In `each_paired_question_is_live_exactly_on_its_rows_no_and_
   blocks_there` (f1): the asymmetric probe (`Int1099 = Some(true)`, `Div1099` stays `Some(false)` ⇒
   `InterestOrDividendsWithout1099` live). Kill: delete the DIV limb → red (quote it).
4. **I-4 — fail closed on the tips Caution, and tell the truth in the classifier.** In
   `screen_inputs_tiered` (param-free tier): `schedule_1a.tips.qualified_tips_reported > 0` and any
   of `occupation_on_treasury_list` / `excludes_unlisted_occupation_tips` /
   `meets_qualified_tip_criteria` is `false` ⇒ refuse `QualifiedTipsCautionNotMet` quoting the
   Caution (`i1040s1a--2025.txt` — cite the line) and naming the three conditions; correct
   `classifier.rs:692-695`'s exemption reason to state what now holds (a `false` refuses when tips are
   claimed). The compute gating (`Schedule1A::compute` reading the three) stays **FR-72** — do not
   touch `schedule_1a.rs`'s arithmetic. Kill: the review's probe fixture (3000 of tips, all three
   `false`) now refuses; with all three `true` it computes as before; the tier census KAT (T4) must
   list the new rule and its fixture.
5. **M-1 — one rule for income boxes with no reader.** In the census, every INCOME box on the
   information returns that has no field and no line becomes `RefuseIfNonzero(<reason naming its
   line>)` — 1099-G boxes 5, 6, 7, 9; 1099-B box 13; 1099-DIV boxes 9/10; and any other you find by
   reading each `NotRead` reason for an amount box the 1040 text calls income — with a comment
   stating the rule once (*an income box with no reader understates, so it fails closed*) at the
   `BoxDecision` doc. Non-income informational boxes stay `NotRead`. List every flipped box.
6. **M-2** — the W-2 box 4/6 arithmetic warnings are suppressed when box 12 carries code A or B
   (uncollected Social Security / Medicare tax on tips — cite `iw2w3` for the codes); kill: a W-2
   with code A and box 4 = 0 does not warn, the same W-2 without the code does.
7. **M-3** — `live_questions` groups R3's door questions with the census (they are the census's own
   follow-ups), and the sweep re-checks liveness immediately before asking each item (a question
   that died from an earlier answer in the round is not asked). Kills: the order snapshot; a
   `state_refund_without_1099g` flip to `n` in the same round leaves `ItemizedPriorYear` unasked.
8. **M-4** — `answer_status` takes the key and resolves the prompt through
   `provenance::current_prompt` (or both sites call it); the per-site `prompt_text` calls go.
   **N-1** — the dead loop in `undated_document_rows` removed, the comment kept.

## Constraints
- `record_answer` stays the only writer; no prompt changes beyond the Caution refusal's text;
  `line-coverage` and `census-join` counts unmoved; `box-census` OK with the field join.
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
APPEND a section `## Fold (seam review I-1 … I-4, M-1 … M-4, N-1)` to
`design/agent-reports/2026-09-07-build-interview-T5-implementation.md`: per item what changed, every
flipped census box, every kill with its red text, every pinned number moved, suite lines per crate.
Return only a 4-line summary.
