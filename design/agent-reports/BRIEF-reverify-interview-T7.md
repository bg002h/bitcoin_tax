# Brief — re-verification of the interview T7 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T7
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T7 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T7 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T7-review.md`, the ledger
  `…T7-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T7-review.md` (if one exists),
  and the report `2026-09-07-build-interview-T7-implementation.md` whose kills (the truth-table KAT
  per flowchart edge; every gate `None` while live; the required DOB; the declined taxpayer-DOB
  case; the born-in-year CTC edge; the waiting §152(d) gate; the classifier; the no-dependents
  `Single` return; the identity-keyed records; FR-70's seeded dependent) and fold section are your
  checklist — all named at dispatch.
- The archived PDFs are gitignored: six `form_delta` tests and
  `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
  PDF-less worktree with a redirected target dir — **environment, not findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(dependent) |
  test(return_refuse) | test(interview_state) | test(provenance) | test(classifier)'`, `-p
  btctax-input-form`, `-p btctax-cli -E 'test(answer) | test(open_next_year) | test(tax_report)'`);
  the instruments `cargo run -q -p xtask -- line-coverage` / `census-join` / `stop-list`.

## What to verify
1. **Every kill in the T7 report and its fold section**, one by one: re-plant the defect as the row
   describes (an edge of the truth table given the wrong outcome; a live gate's `None` accepted; a
   row printed with `date_of_birth = None`; Step 1 computed from the taxpayer's declinable DOB; the
   Exception dropped from row (5)(a) so the born-in-year child lands on ODC; `gross_income_under_
   limit` blocking on a params-less year; a gate keyed by row index; a seeded dependent with a
   carried record), run the named test, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** `line-coverage` holds row (5)(a)'s `help` verbatim against
   `i1040gi--2025.txt:1905-1913` (plant a one-character drift → red); `census-join` and `stop-list`
   unmoved; the TY2024 golden corpus and TY2024 emitter output unchanged (`-p btctax-cli -E
   'test(fullreturn)'`; the identity-grid golden); the §152(d) figure and its cite for TY2024/25;
   every new shaped identifier in fixtures is allowed (`bash scripts/pii-scan-generic.sh <commit>`
   exits 0).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a dependent printable on an incomplete chain would be Critical). Doc-only mismatches
are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T7-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
