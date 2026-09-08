# Brief — re-verification of the interview T11 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T11
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T11 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T11 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T11-review.md`, the ledger
  `…T11-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T11-review.md` (if one
  exists), and the report `2026-09-07-build-interview-T11-implementation.md` whose kills (the
  projection inverse over every golden and the two-dependent household; `ORACLE_INVISIBLE`
  complete by the leaf walk; the line-19 excuse exact; the dependents block deleted → red; no
  identity in `income project`; `check_return.py` reconciling on both oracles; the sweep) and fold
  section are your checklist — all named at dispatch.
- Six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_
  new_directory` fail in a PDF-less worktree with a redirected target dir — **environment, not
  findings**. The Python stack is `.venv/bin/python`; OpenTaxSolver per `scripts/oracle/README*` —
  if unavailable, say so and mark every two-oracle claim UNVERIFIED, never passed on one oracle.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(project) |
  test(golden) | test(oracle) | test(provenance)'`, `-p btctax-cli -E 'test(project) |
  test(fullreturn)'`, `-p btctax-oracle-harness`); the harness per `scripts/oracle/sweep.py`.

## What to verify
1. **Every kill in the T11 report and its fold section**, one by one: re-plant the defect as the
   row describes (a leaf projected to the wrong box; a new `Usd` leaf with neither projection nor
   listing; the dependents block removed; an off-by-one excuse; an identity string leaking into the
   projection), run the named test, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** every golden household round-trips; `ORACLE_INVISIBLE`'s doc states the
   §G-9 limit; no excuse is keyed by a vector name (grep `check_return.py`, `sweep.py` and the
   KATs); the existing sweep reconciles (run once, both oracles, or mark unverified); every new
   shaped identifier in fixtures is allowed (`bash scripts/pii-scan-generic.sh <commit>` exits 0).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a projection to the wrong box or identity in the projection would be Critical).
Doc-only mismatches are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T11-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per
review finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return
ONLY a 3-line summary plus the path.
