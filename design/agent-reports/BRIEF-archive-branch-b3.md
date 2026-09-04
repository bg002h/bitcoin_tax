# BRIEF — the B3 pass on `chore/archive-reconciliation`, scoped `main..HEAD`

One agent, opus. **No subagents. Do not commit, push, or modify any tracked
file** — the only file you write is your report.

This is **the last review before an irreversible action** (merge to `main` of a
public repo). Per `CLAUDE.md` B3: *a per-range review is not a branch review.*
Your scope is the **whole branch**, and your target is **INTERACTION between
commits**, not correctness inside any one of them.

## WHAT THIS BRANCH DOES

Seven commits. It retired two duplicate archive groups, deleted five committed
IRS form PDFs (905,833 bytes) and eleven other tracked files, and **deleted a
dated gate** — `ARCHIVE_RECONCILIATION_REVIEW_BY`, its test, and its `run()`
branch — on the argument that the gate's subject no longer exists and the
replacement guard (`DUPLICATE_SOURCE_GROUPS` pinned at 0) is strictly stronger.

    git log --oneline main..HEAD
    git diff main..HEAD          # 34 files, +1014 / −3544

## ★ THE SEAM — measured, and this is where your budget goes

Only **one** commit has ever been reviewed: `c6c4a7dc`, by a sonnet agent, at
0C/0I/1M/3Nit. Everything else on this branch is **unreviewed**, and five files
were **edited on both sides of that review**, so the reviewer saw a version that
no longer exists:

| file | seen at `c6c4a7dc` | changed again after |
|---|---|---|
| `CONTINUITY.md` | ✅ | ✅ `36c6d12b`, `0a6532d8` |
| `crates/xtask/src/authority_manifest.rs` | ✅ | ✅ `36c6d12b` |
| `crates/xtask/src/authority_conflicts.rs` | ✅ | ✅ `36c6d12b` |
| `design/forms/README.md` | ✅ | ✅ `36c6d12b` |
| `FOLLOWUPS.md` | ✅ | ✅ `36c6d12b` |

And one file was edited **before** the reviewed commit and by it, so the
pre-review half was never read by anyone but its author:

| `crates/xtask/src/archive_check.rs` | edited by `dca6ef25` **and** `c6c4a7dc` |

**The B3 precedent this repo learned from:** three reviews ran on one branch,
all 0C/0I, and the pre-publish pass found an Important in the *earliest* commit
— outside every earlier window. And the fix already existed nine commits later
in the same branch; nobody carried it back, because no reviewer held both
commits at once. **The failure mode is a field of view, not ignorance.**

## WHAT EARLIER ROUNDS ALREADY COVERED — do not redo these

- **The DECISION** (reset vs. resolve vs. retire the gate) was settled by a
  Fable verdict, `design/agent-reports/2026-09-04-archive-tickle-fable.md`, and
  approved by the owner. **Not open. Do not re-litigate it.**
- **`c6c4a7dc` in isolation** — reviewed clean, report at
  `design/agent-reports/2026-09-04-archive-resolution-review.md`. Its four
  findings were folded in `36c6d12b`.
- **Machine-verified at HEAD, green, do not re-derive:** `make check` 2765
  passed / 12 skipped / 0 failed; `cargo fmt --all --check` clean;
  `xtask archive-check` green; `xtask authority-manifest` → 102 entries, **0
  duplicates, pinned 0**, OK; `sha256sum -c legal/SHA256SUMS` → 42 OK;
  `--regen` produces a byte-identical `MANIFEST.json` (empty `git diff`).
- **B1 kill observed** on the current assertion, both directions (rise: a
  planted duplicate; fall: pin raised with none present), then reverted.

## THE QUESTIONS THAT ARE ACTUALLY OPEN

1. **Does the branch contradict itself across commits?** `dca6ef25` wrote a
   RESET LOG row promising precisely what `c6c4a7dc` then did. Is that row now
   an accurate historical record, or does it describe a future that did not
   happen the way it says? It survives in the source **forever** as the only
   at-a-glance account of why the gate was extended twice.
2. **Did the fold `36c6d12b` correct its claims EVERYWHERE they appear?** It
   corrected an overstatement of the author's about a "blind" assertion. The
   original wording also lives in `c6c4a7dc`'s commit message and in a persisted
   report. Persisted reports are verbatim records and must NOT be edited — but
   does anything *downstream* still rely on the wrong claim as if true?
3. **Is anything now orphaned by the deletion of the test?** Grep the whole tree
   for prose describing a mechanism that no longer exists — `HARNESS.md`,
   `CONTINUITY.md`, `STANDARD_WORKFLOW.md`, doc comments, `design/`.
4. **Counts.** `0a6532d8` fixed two stale ones and found one had been wrong for
   some time. **Assume there are more.** Any number in prose describing the
   archives is suspect; measure, then compare.
5. **Is the replacement guard genuinely sufficient?** The tickle covered
   "duplication exists". The pin covers "a duplicate exists". Name anything the
   old gate would have caught that nothing now catches — and say whether it
   matters. FR-23/24/25 already file three known gaps; do not re-file those,
   but do say if they are mis-scoped or if a fourth is missing.

## OUT OF SCOPE

- Do not re-litigate the decision to retire the gate (owner-approved).
- Do not re-audit anything at or before `945d1ac2`.
- Do not propose new checkers or harness features beyond naming a gap.
- Do not hand-count what a command can count.
- Do not edit the two persisted agent reports; they are verbatim records, and
  a claim in one being wrong is expected and already recorded in `42973570`.

## SEVERITY

Critical = wrong result / data loss / unmet guarantee. Important = real defect,
missing case, unsound assumption. **A gate that cannot fail, a refusal that does
not refuse, or a test reporting a false PASS is blocking.** Minor/Nit recorded,
not blocking. Secret-handling defects never gate (owner ruling 2026-08-27).

★ The single most likely defect shape on a branch that deletes a test is **a
guarantee that quietly stopped being enforced.** Hunt that first.

## OUTPUT

Write your report **as your final action** to exactly:

    design/agent-reports/2026-09-04-archive-branch-b3.md

Structure: **VERDICT** (nC/nI/nM/nNit, and a plain merge / do-not-merge) ·
**FINDINGS** (severity, `file:line`, why it is wrong, smallest fix) · **THE
SEAM** (what you found that a single-commit review could not have) · **WHAT I
VERIFIED AND HOW** (command + output) · **WHAT I COULD NOT CHECK**.

Return to the controller **only** a ≤ 8-line summary plus that path.
