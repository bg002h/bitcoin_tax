# Brief — re-verification of the interview T3 fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T3
fold commit). Every plant is made in YOUR worktree and reverted (`git checkout -- <file>` is fine
there); no commits, no subagents.

**The one question:** does every kill the T3 fold report claims actually RED when its defect is
planted today, and does every finding of the T3 seam review now have a test that holds it? Not a
fresh audit; do not re-derive the ledger's dispositions; do not review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build: `0807335b` + pre-review fold `d44f82e4`; the review `2026-09-07-build-interview-T3-review.md`
  (0C/5I/5M/1N); the ledger `…T3-review-VERIFICATION.md`; the fold brief `BRIEF-fold-interview-T3-review.md`;
  the fold report `2026-09-07-build-interview-T3-fold.md` (its kills section is your checklist).
- The archived PDFs are gitignored: six `form_delta` tests and
  `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory` fail in a
  PDF-less worktree with a redirected target dir — **environment, not findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p xtask -E '<filter>'`, `-p btctax-core -E
  'test(document_census) | test(interview_state) | test(return_refuse)'`, `-p btctax-cli -E
  'test(answer)'`); the instruments: `cargo run -q -p xtask -- census-join` / `stop-list` /
  `line-coverage`.

## What to verify
1. **Every kill in the fold report**, one by one: re-plant the defect as the row describes, run the
   named test or instrument, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold:** (I1) the 2024 f1040 `Income` block given
   a `direction = "NoDollar"` key → the map fails to parse (quote it); a `NoDollar` reading given a
   line range → red; one reading deleted from `DIRECTION_OF_CAPTION` → red naming the orphaned
   caption; a reading added for a caption no map carries → red; line 1b's cover moved to an
   `Advisory` → red under the Understates rule (there is now no key to downgrade the block). (I2/I3)
   the probes P1 (`b_1099 = true`, zero rows, ledger disposals) and P2 (`g_1099 = true`, box 2 only)
   pass `screen_inputs`; `b_1099 = false` with one row and `w2 = true` with zero rows still refuse;
   `requires_transcription(B1099)` flipped to `true` → P1 reds. (I4) the `names = "fuel tax"` keyword
   removed from the attestation → the narrow Schedule C line-6 KAT reds. (I5) the old sentence
   (*"None of these changes your tax"* applied to the §6013 election) restored → the snapshot reds.
   (M5) the census rows are asked first in `live_questions`' output on a Single TY2024 fixture; (N1)
   `clear` on a non-live row is a no-op.
3. **Negative claims:** `census-join` still reports 298 entries / 13 maps; `line-coverage` still
   341 / 24 / 0 / 12; no `direction =` key remains in any map outside comments (`grep -rn '^[^#]*direction *='
   crates/btctax-forms/forms/` → 0; the 11 hits of the unanchored grep are comment lines saying the key
   is a parse error); the T3 build's original 19 kills and the D1/D11 kills are still green.

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important**. Doc-only mismatches are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T3-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per fold kill and per
review finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY
a 3-line summary plus the path.
