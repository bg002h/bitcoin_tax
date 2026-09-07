# Brief — re-verification of the interview T6 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T6
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T6 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T6 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T6-review.md`, the ledger
  `…T6-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T6-review.md` (if one exists),
  and the report `2026-09-07-build-interview-T6-implementation.md` whose kills and fold section are
  your checklist — all named at dispatch.
- The archived PDFs are gitignored: six `form_delta` tests and
  `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
  PDF-less worktree with a redirected target dir — **environment, not findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(return_1040) |
  test(return_refuse) | test(digital_asset)'`, `-p btctax-cli -E 'test(panel) | test(answer) |
  test(export_irs_pdf) | test(tax_report)'`, `-p btctax-tui-edit -E 'test(panel) | test(step0)'`,
  `-p xtask -E 'test(stop_list)'`).

## What to verify
1. **Every kill in the T6 report and its fold section**, one by one: re-plant the defect as the row
   describes (the five-row DA table's cells — a contradicted `No` accepted, an unwitnessed `Yes`
   refused, a purchase counted as activity, the wrong event named; the cross-check moved into the
   param-free tier; the panel given a write; the standing-order warning silenced or fired on the
   wrong day; the venue-vs-answer row dropped; a `reconcile` word in a registry prompt; the printed
   box read from the ledger instead of the answer), run the named test or instrument, record RED
   with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** `stop-list` clean; `census-join` and `line-coverage` unmoved; the TY2024
   golden corpus still files (`-p btctax-cli -E 'test(fullreturn)'`); the R6 slice-path tests green;
   `digital_asset_activity = None` blocks commit; T4b's opener seeds it `None`; every new shaped
   identifier in fixtures is allowed (`bash scripts/pii-scan-generic.sh <commit>` exits 0).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a box printed from anything but the answer would be Critical). Doc-only mismatches
are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T6-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
