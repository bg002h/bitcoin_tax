# Brief — re-verification of the interview T2 fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the fold commit `0c942ac2` (named at
dispatch). Read-only for the record: every plant you make is made in YOUR worktree and reverted
(`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the fold report claims actually RED when its defect is planted
today, and does every finding of the seam review now have a test that holds it? Not a fresh audit —
do not review design, do not re-derive the archive (the controller re-fetched and hashed it), do not
re-litigate the ledger's dispositions.

## Settled facts (machine-verified by the controller; do not re-measure)
- Suite at `0c942ac2`: 3215 passed / 12 skipped (`make check`). `authority-manifest` OK (170);
  `box-census` OK: 246 printed boxes / 15 editions / 7 information returns (123 entries);
  `line-coverage` OK: 341 / 24 / 0 / 12.
- The archived PDFs are gitignored: in a worktree `ls design/forms/*/*.pdf | wc -l` → 0. Six
  `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
  fail in a PDF-less worktree with a redirected target dir — **these are environment, not
  findings**. The notes + extracts + geometry fixtures are committed and are what the instruments read.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only: `cargo nextest run --locked -p xtask -E '<filter>'` and
  `-p btctax-core -E 'test(line_coverage)'`; the instruments are `cargo run -q -p xtask -- box-census`
  / `line-coverage` / `authority-manifest` / `authority-refresh --check` (that last one needs the
  network and takes a minute; run it once).

## What to verify
Read `design/agent-reports/2026-09-07-build-interview-T2-fold.md` §9 (the kills table A–I plus the
in-suite plants it lists) and the seam review `2026-09-07-build-interview-T2-review.md` findings
C1 / I1 / I2 / I3 / M1–M4. For EACH of A–I: re-plant the defect as the row describes (in source, in
the census table, or in a note), run the test or instrument the row names, and record RED with the
message, then revert. Then, for each review finding, name the test that now holds it, and plant the
review's OWN evidence (the Schedule A 5b ← Form 6251 AMTFTC sentence; the `f1098e` authority
deleted; a `DocBox` pinned to the newest edition; a mutated note sha256) — RED or not. Finally confirm
the fold's negative claims: nothing pre-existing in `design/forms/` changed (`git diff 1c8a7301..0c942ac2
--stat -- design/forms | grep -v '^ design/forms/20\(22\|24\|26\)\|extract\|geometry'`), the
ratchet constants are untouched, and `revision_in_force` returns the table the KAT pins for the 21
(stem, year) pairs.

## Severity
A kill that does not red, a review finding with no holding test, or a claim in the report that the
tree contradicts is **Important** (a wrong result in a printed figure would be Critical — none is
expected here). Doc-only mismatches are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T2-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per A–I and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
