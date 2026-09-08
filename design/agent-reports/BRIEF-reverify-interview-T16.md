# Brief — re-verification of the interview T16 build + fold (sonnet, worktree, plants every kill)

You are an independent VERIFIER in your own git worktree at the commit named at dispatch (the T16
fold commit, or the build commit if the review folded nothing). Every plant is made in YOUR
worktree and reverted (`git checkout -- <file>` is fine there); no commits, no subagents. The
archived PDFs are gitignored — the notes, extracts and geometry fixtures are what the instruments
read; re-fetch from a note's URL only if a check needs bytes.

**The one question:** does every kill the T16 build report (and its fold section, if any) claims
actually RED when its defect is planted today, and does every finding of the T16 seam review now
have a test that holds it? Not a fresh audit; do not re-derive the ledger's dispositions; do not
review design.

## Settled facts (machine-verified by the controller; do not re-measure)
- The build commit, the review `2026-09-07-build-interview-T16-review.md`, the ledger
  `…T16-review-VERIFICATION.md`, the fold brief `BRIEF-fold-interview-T16-review.md` (if one exists),
  and the report `2026-09-07-build-interview-T16-implementation.md` whose kills (`line-coverage` on
  every Form 8889 line; the 1099-SA / 5498-SA censuses; the contribution limit from params; the
  `hsa_activity` rows rules; the 8f + 17c reach; the golden HSA household on both oracles; the map
  cells read back; `census-join`'s counts; the opener's trustee identity) and fold section are your
  checklist — all named at dispatch.
- Six `form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_
  new_directory` fail in a PDF-less worktree with a redirected target dir — **environment, not
  findings**.
- Build setup: `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo
  command; scoped runs only (`cargo nextest run --locked -p btctax-core -E 'test(form8889) |
  test(hsa) | test(return_refuse) | test(printed)'`, `-p btctax-forms -E 'test(f8889) |
  test(field_census)'`, `-p btctax-input-form`, `-p btctax-cli -E 'test(fullreturn) |
  test(tax_report)'`, `-p xtask -E 'test(box_census) | test(line_coverage) | test(authority)'`);
  the instruments `cargo run -q -p xtask -- line-coverage` / `box-census` / `census-join` /
  `authority-manifest`; the oracle harness per `scripts/oracle/README*` (`.venv/bin/python`).

## What to verify
1. **Every kill in the T16 report and its fold section**, one by one: re-plant the defect as the row
   describes (a Form 8889 line's production deleted; a one-character quote drift in a line's doc
   comment; a computed line's line reference changed; a 1099-SA census entry deleted; a caption the
   extract does not carry; a contribution over the year's limit silently clamped; `hsa_activity =
   Some(true)` with no row accepted; a distribution not reaching 8f/17c; a map cell unmapped; the
   form emitted on a no-HSA return), run the named test or instrument, record RED with its message,
   revert.
2. **The review's own evidence, re-planted after the fold**, per finding — name the test that holds
   each and plant its removal.
3. **Negative claims:** `authority-manifest` OK with the new documents; `line-coverage` holds every
   8889 line; `census-join`'s counts fell by exactly the four reach lines per year the report
   states; `box-census` OK with the field join; the TY2024 emitter output for a no-HSA fixture
   byte-identical (the golden corpus); the golden HSA household reconciles on BOTH oracles (run the
   harness once; if OTS is unavailable say so and mark the claim unverified, never passed); every
   new shaped identifier in fixtures is allowed (`bash scripts/pii-scan-generic.sh <commit>` exits
   0); no `NotRead("T16 …")` remains.

## Severity
A kill that does not red, a review finding with no holding test, or a claim the tree contradicts is
**Important** (a wrong printed figure or a mis-referenced computed line would be Critical). Doc-only
mismatches are Minor. Report every verdict, including the ones that pass.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T16-reverify.md` in your worktree: a table
(kill / plant made / command / RED text or "did not red" / verdict), one row per kill and per review
finding, then the negative-claim checks, then `Counts: C=<n> I=<n> M=<n> N=<n>`. Return ONLY a
3-line summary plus the path.
