# Brief — re-verification of the interview T8 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T8
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T8 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T8 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T8-review.md`, the ledger
  `…T8-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T8-review.md` (if one exists),
  and the report `2026-09-07-build-interview-T8-implementation.md` whose kills (the flowchart truth
  table with the credit column incl. the born-in-year CTC row; TY2024 byte-identical; the TY2025
  fixture rows read back; the `UNCENSUSED` register falling by exactly the mapped cells; the HoH
  kills incl. `MarriedLivedApart` and `NraSpouseNoElection`; the FR-67 gate; the QSS `None`/`No`
  kills and the joint-rate computation; the derived window; the forgo size present/absent; the
  no-brick extension) and fold section are your checklist — all named at dispatch.
- Six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_
  new_directory` fail in a PDF-less worktree with a redirected target dir — **environment, not
  findings**. The TY2025 `f1040` template must be present for the emitter kills — if it is absent
  in the worktree, say so and mark those claims unverified, never passed.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(dependent) |
  test(hoh) | test(qss) | test(return_refuse) | test(interview_state)'`, `-p btctax-forms -E
  'test(f1040) | test(field_census) | test(dependents)'`, `-p btctax-input-form`, `-p btctax-cli -E
  'test(answer) | test(fullreturn) | test(export_irs_pdf)'`); the instruments `cargo run -q -p
  xtask -- line-coverage` / `census-join` / `stop-list`.

## What to verify
1. **Every kill in the T8 report and its fold section**, one by one: re-plant the defect as the row
   describes, run the named test or instrument, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** the TY2024 golden and emitter snapshot unchanged; the `UNCENSUSED` register's
   `f1040` count equals the report's new value and every mapped cell has a `[census]` entry
   (plant one without → red); `line-coverage`, `census-join`, `stop-list` unmoved or as stated; the
   no-brick test covers every new question; every new shaped identifier in fixtures is allowed
   (`bash scripts/pii-scan-generic.sh <commit>` exits 0).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a credit box or filing status printed without the answers establishing it would be
Critical). Doc-only mismatches are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T8-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
