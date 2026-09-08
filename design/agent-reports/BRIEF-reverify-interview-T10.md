# Brief — re-verification of the interview T10 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T10
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents.

**The one question:** does every kill the T10 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T10 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T10-review.md`, the ledger
  `…T10-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T10-review.md` (if one
  exists), and the report `2026-09-07-build-interview-T10-implementation.md` whose kills (the
  routing validator's prefixes and checksum; the advisory silent/present; the spouse-PIN asymmetry
  and marker guard; the foreign block present/absent; the mapped cells read back; the TY2024
  golden; the opener not carrying the deposit or PINs) and fold section are your checklist — all
  named at dispatch.
- Six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_
  new_directory` fail in a PDF-less worktree with a redirected target dir — **environment, not
  findings**; if a bundled `f1040` template is absent, mark the emitter claims unverified, never
  passed.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(direct_deposit) |
  test(routing) | test(advisories) | test(return_refuse)'`, `-p btctax-forms -E 'test(f1040) |
  test(field_census)'`, `-p btctax-input-form`, `-p btctax-cli -E 'test(fullreturn) |
  test(export_irs_pdf) | test(open_next_year) | test(tax_report)'`); the instruments `cargo run -q
  -p xtask -- line-coverage` / `census-join` / `stop-list`.

## What to verify
1. **Every kill in the T10 report and its fold section**, one by one: re-plant the defect as the
   row describes, run the named test or instrument, record RED with its message, revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** no emitted artifact (snapshot, golden, `report`, the filled PDF's text
   layer outside the spouse cell) contains a fixture's spouse PIN digits; the `UNCENSUSED`
   register's `f1040` count matches the report; the TY2024 golden unchanged except the explained
   cells; `stop-list`, `census-join`, `line-coverage` as stated; every new shaped identifier in
   fixtures is allowed (`bash scripts/pii-scan-generic.sh <commit>` exits 0 — a routing number is
   not a PII shape, but check the fixture uses a documented test value).

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a refund routable from an invalid number would be Critical). Secret-handling
defects are logged, never Critical/Important. Doc-only mismatches are Minor. Report every verdict,
including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T10-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per
review finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return
ONLY a 3-line summary plus the path.
