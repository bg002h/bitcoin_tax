# Brief — re-verification of the interview T5 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T5
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T5 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T5 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build: `34472416` + `9a845dc5`; the review `2026-09-07-build-interview-T5-review.md`; the
  ledger `…T5-review-VERIFICATION.md`; the fold brief `BRIEF-fold-interview-T5-review.md` (if one
  exists); the report `2026-09-07-build-interview-T5-implementation.md` whose twenty kills and fold
  section are your checklist.
- The archived PDFs are gitignored: six `form_delta` tests and
  `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
  PDF-less worktree with a redirected target dir — **environment, not findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-input-form`, `-p btctax-core -E
  'test(document_census) | test(return_refuse) | test(printed) | test(provenance)'`, `-p btctax-cli
  -E 'test(answer) | test(tax_report) | test(fullreturn)'`, `-p xtask -E 'test(box_census)'`); the
  instruments `cargo run -q -p xtask -- box-census` / `line-coverage` / `census-join`.

## What to verify
1. **Every kill in the T5 report and its fold section**, one by one: re-plant the defect as the row
   describes (a deleted `[boxes]` entry; an entry for a caption the extract does not carry; a
   `Collected` naming a field outside its section; a one-character caption drift; W-2 box 13
   checked; 1099-INT box 11 > 0; box 10 not reaching the 2b sum; a paired question live on the
   wrong census state; `itemized_prior_year` not live on a refund-question `Yes`; a warning that
   writes; `EXEMPT_PREFIXES` regrown; a `NotInForm` anchor restored; the 1098-E sum not reaching
   Schedule 1 line 21; the manifest not printing *transcribed without a date*), run the named test
   or instrument, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — and the controller's
   four pointed questions as resolved (the rendered-prompt hash key; the sweep's bound and ordering;
   `occupation_on_treasury_list`; the six warning fixtures): name the test that holds each and plant
   its removal.
3. **Negative claims:** `box-census` OK with the field join; `line-coverage` 341 / 24 / 0 / 12;
   `census-join` 298 / 13; the TY2024 golden corpus still files; no `Usd` leaf outside
   `EXEMPT_PREFIXES`/`EXEMPT_LEAVES` is uncovered (the coverage KAT); every new shaped identifier
   in fixtures is on `scripts/pii-scan-generic.sh`'s allowed list (`bash scripts/pii-scan-generic.sh
   <commit>` exits 0 for the fold commit).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a box reaching the wrong printed line would be Critical). Doc-only mismatches are
Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T5-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
