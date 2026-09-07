# Brief — re-verification of the interview T4b build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T4b
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR worktree
and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T4b build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T4b seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build: `44ca7075`; the review `2026-09-07-build-interview-T4b-review.md`; the ledger
  `…T4b-review-VERIFICATION.md`; the fold brief `BRIEF-fold-interview-T4b-review.md` (if one exists);
  the report `2026-09-07-build-interview-T4b-implementation.md` whose kills section (17 kills) and
  fold section are your checklist.
- The archived PDFs are gitignored: six `form_delta` tests and
  `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
  PDF-less worktree with a redirected target dir — **environment, not findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-cli -E 'binary(open_next_year_t4b)
  | test(open_next_year) | test(input_form_store)'`, `-p btctax-core -E 'test(provenance) |
  test(leaf_walk)'`, `-p btctax-tui-edit -E 'test(open) | test(next_year)'`).

## What to verify
1. **Every kill in the T4b report and its fold section**, one by one: re-plant the defect as the row
   describes (a `PerYear` gate copied from year N; a money box copied; an `AnswerRecord` carried; the
   seed reading year N's `_in` instead of the frozen `_out`; the vacuity guard's 21 non-zero leaves
   reduced to zero; the T4 refusals bypassed; the TUI action shown without a year-N row), run the
   named test, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — and the controller's
   three pointed questions as they were resolved (the carried `filing_status`; the header identities;
   the un-asked seeded dependent): for each, name the test that holds the resolution and plant its
   removal.
3. **Negative claims:** `return_inputs::get(N+1)` is `None` after opening; year N's committed row,
   draft table and answer log are byte-identical after opening; `report --write-carryover`'s tests
   green; `census-join` 298 / 13 and `line-coverage` 341 / 24 / 0 / 12 unmoved.

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a carried answer or amount reaching the seed unnoticed would be Critical). Doc-only
mismatches are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T4b-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
