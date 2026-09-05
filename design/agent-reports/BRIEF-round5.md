# BRIEF — round 5. Short by design. The question is CONVERGENCE.

One agent, opus. **No subagents. Do not commit, push, or modify any tracked
file** — the only file you write is your report.

## THE SITUATION, HONESTLY

Four rounds have run. **Every one found its Importants in the previous round's
fold** — four for four. The severity trend is 3I → 3I → 1I, and round 4
explicitly certified the *guard itself* correct: it found no way to make the
code lose a manifest entry silently. Round 4's single Important was in the test
fixture, not in behaviour, and round 4's own report called the remedy "a short
round-5, not a reopening."

**Your job is to end this or to justify continuing it.** State
**CONVERGED / NOT CONVERGED** explicitly. A clean verdict is the expected and
useful outcome. Do not manufacture a finding to justify the round; do not
suppress one to end it. If the only things left are Minor/Nit, say CONVERGED
and list them as non-blocking — that is a merge.

## SCOPE — three commits, nothing else

    git show 544218a6   # round-4 fold: hermetic-ish fixture, check-ignore hardening, env scrubbing
    git show 5f974c6a   # FR-26 now names the tempting shortcut and says it reds
    git show HEAD       # fixture made hermetic against the developer's global git config

    git diff 101434dc..HEAD -- crates/xtask/src/authority_manifest.rs

The surface is one function and one test. You may raise a finding anywhere in
`main..HEAD` if these three commits made it wrong.

## WHAT TO ATTACK

1. **The fixture now shells out to `git` — three times — inside a unit test that
   runs on Linux, macOS and Windows CI.** That is new, and it is the riskiest
   thing here. Can it fail, hang, or lie on a machine unlike this one? Consider:
   `git` absent from PATH; a `git` too old for the flags used; Windows path
   handling through `to_str()`; `init.defaultBranch` warnings; a repo created
   inside an existing repo; `core.excludesFile` set at SYSTEM level rather than
   global; `GIT_*` env vars the scrub list misses; tempdir on a filesystem where
   `git init` behaves oddly.
2. **Is the storage-axis assertion right, and is it in the right place?** It
   asserts exactly `1 note / 1 committed`. Is exact-count the correct assertion,
   or does it make the test brittle against a future third fixture document?
3. **Does anything still claim more than it delivers?** That shape has appeared
   in three of four rounds. Check the fixture's doc comment, `regen`'s comments,
   FR-25, FR-26, and the round-4 fold's commit message against the code.

## ALREADY VERIFIED — do not re-derive, do not repeat

At HEAD: `make check` **2766 passed / 12 skipped / 0 failed**; `cargo fmt --all
--check` clean; the test run 5× consecutively, PASS each time.

Mutation ledger, all re-run against current code, each observed **red**, then
reverted:

| # | mutation | result |
|---|---|---|
| M1 | refusal block moved below `fs::write` | FAIL |
| M2 | refusal removed entirely | FAIL |
| M3 | guard re-keyed on notes only | FAIL |
| M4 | `Err` from `load` treated as absent (corrupt ≡ absent) | FAIL |
| M5 | guard restricted to `Storage::Committed` | FAIL |
| M7 | the fixture's repo-local `core.excludesFile` line removed, under a hostile `HOME` | FAIL |
| — | pristine, normal env **and** hostile `HOME` | PASS |

Environment probe already done: with a global `core.excludesFile` ignoring
`*.html`, the fixture reported "2 note / 0 committed" and reded **before** HEAD;
it passes after. Do not repeat that probe — extend it if you can think of a
sharper one.

## OUT OF SCOPE

- Do not re-litigate retiring the archive tickle (owner-approved).
- Do not re-audit at or before `945d1ac2`.
- Do not edit the five persisted agent reports — verbatim records.
- Do not propose building FR-26's fetcher; it is filed with a reason.

## SEVERITY

Critical = wrong result / data loss / unmet guarantee. Important = real defect,
missing case, unsound assumption. **A gate that cannot fail, a refusal that does
not refuse, or a test reporting a false PASS is blocking.** Minor/Nit recorded,
**not** blocking — and with four rounds behind this, Minor/Nit findings should be
reported as such and NOT treated as reasons to withhold a merge.

## OUTPUT

Write your report **as your final action** to exactly:

    design/agent-reports/2026-09-04-round5.md

Structure: **VERDICT** (nC/nI/nM/nNit, plain merge / do-not-merge, and explicit
**CONVERGED / NOT CONVERGED**) · **FINDINGS** · **WHAT I ATTACKED AND HOW**
(command + output; include anything you tried that did NOT break it) · **WHAT I
COULD NOT CHECK**.

Return to the controller **only** a ≤ 8-line summary plus that path.
