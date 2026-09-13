# REPORT wave1-D: four instruments that did not check what they claimed

Owned files only, all four touched, nothing else: scripts/oracle/gen_goldens.py,
scripts/oracle/taxcalc_exact.py, crates/btctax-core/src/tax/document_census.rs,
crates/btctax-core/src/tax/packet.rs. No commits, no pushes, no subagents.

Status at time of writing (four modified, nothing else):

    M crates/btctax-core/src/tax/document_census.rs
    M crates/btctax-core/src/tax/packet.rs
    M scripts/oracle/gen_goldens.py
    M scripts/oracle/taxcalc_exact.py

Diffstat: 89 insertions(+), 37 deletions(-) across exactly those four files.

## Validation suite (run in the foreground, output captured once and grepped)

- make check (CARGO_TARGET_DIR pointed at this worktree's own target-d): the whole-workspace
  cargo nextest run --workspace --no-fail-fast plus cargo clippy --workspace --all-targets
  --all-features -- -D warnings, run in parallel by the Makefile, both gated by a combined exit
  code. This first attempt exceeded the tool's 120s foreground window (the box had two other
  agents' cargo/rustc/nextest processes running concurrently in sibling worktrees, confirmed via
  ps aux), so the harness moved it to background; its captured output was read directly (cat on
  the task's own output file, not re-run) rather than re-executed, since re-running would have
  duplicated about 4 minutes of CPU under the same contention for no new information:

      EXIT: 0
      ...
           Summary [ 235.163s] 3674 tests run: 3674 passed (6 slow), 12 skipped

  Exit 0 covers BOTH halves of make check's script (st=0; wait $t || st=1; wait $c || st=1; ...
  exit $st), so clippy is clean at -D warnings too, not only nextest.
- cargo fmt --all (no changes), then cargo fmt --all --check: exit 0, empty output.
- Tree state after all kill/revert cycles below is byte-identical to the diffstat above (verified
  by re-diffing before and after) -- the suite result stands for the current tree.

## FR-169 -- assert_baked_provenance_is_current() had no caller

Fix. Wired into gen_goldens.selftest() as claim (5): calls the function, treats AssertionError
as a counted failure with a printed message, otherwise prints OK.

Green (current tree):

      assert_baked_provenance_is_current(): OK (baked _provenance.corpus matches CORPUS_DESCRIPTION)
    gen_goldens: FR-164 year plumbing OK

(exit 0)

Red -- the function's own documented plant (delete the NON_INTERACTION clause from
CORPUS_DESCRIPTION, nothing else):

      FAIL: assert_baked_provenance_is_current() raised: the committed golden's _provenance.corpus is STALE -- it no longer matches what gen_goldens.py would write.
      baked:     ...+ the hand-written NON_INTERACTION cells (households that make a claimed non-interaction observable...
      generator: ...+ the hand-written LOW_END cells (shapes the axis tables cannot reach) + a generated array...
    This is N-4's exact failure: a corpus list was added and the description was not.
    gen_goldens: FR-164 year plumbing FAILED

(exit 1)

Green again after revert: identical to the first block above (exit 0); diffstat on
gen_goldens.py returned to 12 insertions, 1 deletion with no plant residue.

## FR-168 -- build_calculator never compared a row's FLPDYR to year

Fix. build_calculator now collects every row whose FLPDYR disagrees with the year argument and
refuses, naming both years, before constructing tc.Records. Kill case (4b) added to
taxcalc_exact.selftest().

Green (current tree, --selftest):

    taxcalc_exact: `exact` sticks in both directions, an input column is refused by name, and 44 fractional-step vector(s) move off the smooth fallback (taxcalc 6.8.2) -- B1 kills OK

(exit 0)

Direct demonstration (standalone script calling build_calculator directly, not through
selftest) -- the disagreeing case refused, the agreeing case accepted:

    --- mismatched: FLPDYR=2024, year=2025 ---
    REFUSED: row(s) [0] carry FLPDYR [2024] but build_calculator(rows, year=2025) was asked to build 2025. Every row's FLPDYR must equal the year argument -- this function's start_year and advance_to_year follow year unconditionally, so a disagreeing row would be scored against the WRONG year's policy while its own label claims otherwise (FR-168).

    --- matching: FLPDYR=2025, year=2025 ---
    ACCEPTED, calc built: Calculator

Red -- the check itself disabled (mismatched = [] in place of the real comprehension),
--selftest:

    AssertionError: a row claiming FLPDYR 2024 was accepted by build_calculator(rows, year=2025) -- FR-168 is re-armed: the row would be scored against the wrong year's policy

(exit 1) -- and re-running the direct demonstration script against the disabled check shows the
actual pre-fix defect reproduced live: mismatched FLPDYR=2024, year=2025 -> ACCEPTED (BUG).

Green again after revert: identical to the first --selftest block (exit 0); diffstat on
taxcalc_exact.py returned to 31 insertions, 0 deletions with no plant residue.

## FR-179 -- DocumentRow::ALL's completeness pinned by a hand-typed 20

Premise check (asked for explicitly): nothing else in the repo catches a DocumentRow variant
missing from ALL. Traced every consumer outside document_census.rs (interview_state.rs,
form8889.rs, provenance.rs, testonly.rs, return_refuse.rs, return_inputs.rs, scrub_axis.rs,
questions.rs): all of them either iterate DocumentRow::ALL (silently missing whatever ALL omits)
or reference specific variants -- none does an exhaustive match DocumentRow { ... } of its own.
The one place that could have caught it, question_id()'s exhaustive match, only proves ARMS
exist for every variant; it does not prove ALL contains it (same limit questions.rs:3598's own
doc comment already admits for QuestionId::ALL, which is why that codebase already carries an
honest "does not" note for the identical shape). Premise confirmed -- no existing structural
check would have caught this.

Fix (stronger than "add a check that reds"): eliminated the two-list shape entirely.
DocumentRow and ALL are both generated by one new macro, document_rows!, from a single
invocation listing each variant once (doc comments carried through unchanged -- copied, not
retyped, to avoid a transcription risk on twenty verbatim IRS-form doc comments). A variant
added to the invocation appears in the enum AND in ALL in the same edit; there is no longer a
second list to forget. Doc comments on the enum, on ALL, and on
every_row_is_listed_and_round_trips rewritten to state what is actually guaranteed instead of
the false claim.

Baseline green (current tree): cargo build -p btctax-core --lib -- clean; cargo test -p
btctax-core --lib tax::document_census -- 10 passed; 0 failed.

Red -- a 21st variant added to the macro invocation, nothing else changed (this is exactly the
scenario the follow-up describes: "add a 21st row... forget ALL" -- except here there is no ALL
left to forget, so the proof is that the build stops you before that step is even reachable):

    error[E0004]: non-exhaustive patterns: DocumentRow::Fr179PlantedRowFresh not covered

10 such errors, cargo build -p btctax-core --lib exit 101, every one of them inside
document_census.rs at its own exhaustive matches (designation, prompt, question_id,
exit_sentence, and others) -- confirmed by grepping the error locations, all naming this file
and no other. (An earlier same-session run of this identical plant produced 11 errors rather
than 10 -- the count depends on incremental-build/test-vs-lib target scope, not on the
mechanism; the mechanism of red -- E0004, confined to this file -- was identical both times.)

Green again after revert: cargo test -p btctax-core --lib tax::document_census -- 10 passed;
0 failed, exit 0; diffstat on document_census.rs returned to 39 insertions, 34 deletions with
no plant residue.

Related, out of scope: crates/btctax-core/src/tax/provenance.rs:1243-1244 carries the identical
false claim for DocumentKind::ALL (a 9-element hand-typed array), citing "the DocumentRow::ALL
pattern" as its justification -- a citation now stale, since that pattern is fixed here and
DocumentKind::ALL is not. Not touched (outside the owned four files); worth a follow-up entry.

## FR-208 -- stale quotation in packet.rs

Change. The doc comment at packet.rs illustrating the direct-deposit refusal's two remedies
quoted a paraphrase of the message that predates the IRS's October-2025 paper-check retraction
("...the refund then arrives as a paper check"). Replaced with the CURRENT verbatim wording from
return_refuse::screen_direct_deposit's actual refusal message ("Correct it, or delete the
direct-deposit block: a return with no deposit instruction is still complete and filable. Do not
count on a cheque in the post instead") -- confirmed byte-identical against
return_refuse.rs:2625-2635, matching through the shared prefix and the retraction clause.

Was the quote arguing against relying on a cheque? Yes -- its role is to show the refusal names
both remedies (correct the number, or delete the block) and the consequence of the second. Per
the task's framing, the retraction makes that argument stronger, not weaker: a filer who deletes
the block today risks getting no cheque at all, not merely a slower one. The argument was
preserved; only the quotation changed, plus a note citing FR-208 and the retraction
(i1040gi--2025.txt:23824-23827, line numbers verified against the current extract).

No kill needed (doc-comment-only, no behavior). Build check: cargo build -p btctax-core --lib
clean.

## Files changed (absolute paths)

- /scratch/code/bitcoin_tax/.claude/worktrees/agent-aa5b4f9d1a602b5d8/scripts/oracle/gen_goldens.py
- /scratch/code/bitcoin_tax/.claude/worktrees/agent-aa5b4f9d1a602b5d8/scripts/oracle/taxcalc_exact.py
- /scratch/code/bitcoin_tax/.claude/worktrees/agent-aa5b4f9d1a602b5d8/crates/btctax-core/src/tax/document_census.rs
- /scratch/code/bitcoin_tax/.claude/worktrees/agent-aa5b4f9d1a602b5d8/crates/btctax-core/src/tax/packet.rs

No commits, no pushes, no subagents used at any point.
