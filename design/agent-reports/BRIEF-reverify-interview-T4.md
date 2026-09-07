# Brief — re-verification of the interview T4 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T4
fold commit). Every plant is made in YOUR worktree and reverted (`git checkout -- <file>` is fine
there); no commits, no subagents.

**The one question:** does every kill the T4 build report and its fold section claim actually RED
when its defect is planted today, and does every finding of the T4 seam review now have a test that
holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build: `bbcee739`; the review `2026-09-07-build-interview-T4-review.md` (1C/0I/3M/1N); the
  ledger `…T4-review-VERIFICATION.md`; the fold brief `BRIEF-fold-interview-T4-review.md`; the report
  `2026-09-07-build-interview-T4-implementation.md` whose kills (§1–§5 and the appended fold section)
  are your checklist.
- The archived PDFs are gitignored: six `form_delta` tests and
  `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
  PDF-less worktree with a redirected target dir — **environment, not findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-cli -E 'binary(year_gate_t4) |
  test(input_form_store) | test(answer)'`, `-p btctax-core -E 'test(param_free) | test(return_refuse)'`,
  `-p btctax-tui-edit -E 'test(discard) | test(gate)'`).

## What to verify
1. **Every kill in the T4 report and its fold section**, one by one: re-plant the defect as the row
   describes (a rule deleted from `screen_inputs_tiered`; a rule moved inside a `tier.package` gate;
   the unanswered tier ungated; `income answer` writing the committed row on the draft path; the
   disposable predicate re-narrowed to the four categories; the `\` continuations removed; the
   parked wording restored; the `income answer` exit restored in the sales-tax detail), run the named
   test, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold:** a TY2026 draft with two providers'
   `CohortAnswers` and a Schedule C, then `income import` without `--discard-draft`, `income clear`
   without it, and `load` on a stale schema — each refuses and the draft survives byte-identical;
   `report --write-carryover` on year N behaves the same on N+1; a draft differing from the seed in
   one uncounted field is non-disposable; the discard screen never says "parked" for an interview
   draft; `income show-broker-answers` and `report` on a stale-interview-draft year render with a
   note while `income import` on it refuses.
3. **Negative claims:** `return_inputs::get(2026)` is `None` after `income answer` on a draft-only
   year; the TY2024 golden corpus still files (`-p btctax-cli -E 'test(fullreturn)'`); `census-join`
   298 / 13 and `line-coverage` 341 / 24 / 0 / 12 unmoved.

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a draft destroyed unconfirmed would be Critical). Doc-only mismatches are Minor.
Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T4-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
