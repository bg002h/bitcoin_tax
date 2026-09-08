# Brief — re-verification of the interview T9 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T9
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T9 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T9 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T9-review.md`, the ledger
  `…T9-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T9-review.md` (if one exists),
  and the report `2026-09-07-build-interview-T9-implementation.md` whose kills (8e = 8a + 8b + 8c;
  the ceiling set incl. the aggregate and MFS cases; `other_borrower_paid_interest`; box 4 = $1 vs
  0; the standard-deduction fixture asking and refusing nothing vs the Schedule A twin; the 8396
  gate; the home-sale table; the empty `recipient_tin`; the per-edition census; the opener's
  lender; the import-time box-4 refusal) and fold section are your checklist — all named at
  dispatch.
- Six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_
  new_directory` fail in a PDF-less worktree with a redirected target dir — **environment, not
  findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(form1098) |
  test(home_sale) | test(schedule_a) | test(return_refuse) | test(printed)'`, `-p btctax-forms -E
  'test(f1040sa) | test(field_census)'`, `-p btctax-input-form`, `-p btctax-cli -E 'test(fullreturn)
  | test(tax_report) | test(open_next_year)'`, `-p xtask -E 'test(box_census) |
  test(line_coverage)'`); the instruments `cargo run -q -p xtask -- box-census` / `line-coverage` /
  `census-join`; the oracle harness per `scripts/oracle/README*` (`.venv/bin/python`; if OTS is
  unavailable say so and mark the sweep claim unverified, never passed).

## What to verify
1. **Every kill in the T9 report and its fold section**, one by one: re-plant the defect as the row
   describes, run the named test or instrument, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** `box-census` OK with both 1098 editions joined; `line-coverage` holds
   8a/8b/8c/8e; `census-join` as stated; the two-oracle sweep reconciles (run once); the TY2024
   golden corpus files with the scalar replaced by the row (each moved golden line explained in the
   report — check one); a standard-deduction fixture's printed Schedule A is unchanged; every new
   shaped identifier in fixtures is allowed (`bash scripts/pii-scan-generic.sh <commit>` exits 0).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a standard-deduction filer refused over a 1098, or a figure reaching a line the tool
does not compute, would be Critical). Doc-only mismatches are Minor. Report every verdict,
including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T9-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
