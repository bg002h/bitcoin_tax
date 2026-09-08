# Brief — re-verification of the interview T12 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T12
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T12 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T12 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T12-review.md`, the ledger
  `…T12-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T12-review.md` (if one
  exists), and the report `2026-09-07-build-interview-T12-implementation.md` whose kills (the pane
  and modal snapshots with N blocking rows; `(declined)` in forgoing never blocking; the forgo
  size present/absent; the refusing list's exits; `make docs` clean and byte-stable; the manifest
  block on a fixture packet; `report`'s census and masked-TIN provenance; the walkthrough goldens;
  the R15 greps) and fold section are your checklist — all named at dispatch.
- Six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_
  new_directory` fail in a PDF-less worktree with a redirected target dir — **environment, not
  findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-tui-edit`, `-p btctax-cli -E
  'test(report) | test(manifest) | test(answer) | test(export_irs_pdf)'`, `-p btctax-core -E
  'test(interview_state)'`, `-p xtask -E 'test(stop_list) | test(examples)'`); `make docs` twice
  (byte-identical the second time).

## What to verify
1. **Every kill in the T12 report and its fold section**, one by one: re-plant the defect as the
   row describes (a state dropped from the pane; `Declined` rendered in blocking; the forgo size
   shown without params; the refusing list hidden from the modal; an unmasked TIN in `report`; the
   word "progress" in a pane; a render path that writes), run the named test or instrument, record
   RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** `make docs` byte-stable; `stop-list` clean; the walkthrough goldens moved
   only where a screen changed (check three moved lines against the report's explanations); no
   render code calls `save_draft` / `return_inputs::set` / `record_answer` (grep); `LIMITATIONS.md`
   carries the interview's stop list incl. the venue granularity note (FR-73).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a refusing row hidden before commit or a render that writes would be Critical).
Secret-handling defects are logged, never Critical/Important. Doc-only mismatches are Minor.
Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T12-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per
review finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return
ONLY a 3-line summary plus the path.
