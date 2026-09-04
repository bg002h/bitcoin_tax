# BRIEF — review the archive-reconciliation resolution (`c6c4a7dc`)

One agent, sonnet tier. **No subagents. Do not commit, push, or modify any
tracked file** — the only file you write is your report.

## THE ONE QUESTION

`c6c4a7dc` retired two duplicate archive groups and **deleted a dated gate**
(`ARCHIVE_RECONCILIATION_REVIEW_BY`, its test, and its `run()` branch).

**Did anything break, and did the deletion take anything with it that is still
needed?** The questions are mechanical, and there are exactly four:

1. **Every consumer redirected?** Five (B) form PDFs, their five text extracts,
   and three `periodic/` notes were deleted. Does anything still point at a path
   that no longer exists, other than a deliberate historical record?
2. **Did the guard that replaces the tickle actually survive intact?** After the
   rewrite, does `duplicate_source_groups_may_only_shrink` still red in BOTH
   directions, and does its message still name the offending documents?
3. **Is any authority now unreachable?** `legal/SOURCES.md` is the legal-defense
   index. For each of the five retired documents, can a reader still get from a
   citation to the bytes — a URL and a sha256 that agree?
4. **Does any doc now describe a mechanism that is gone**, or claim a count that
   the tree does not have?

## ALREADY MACHINE-VERIFIED — do NOT re-derive, spend your budget elsewhere

Run at `c6c4a7dc`, all green:

- `make check` → 2765 passed, 12 skipped, 0 failed (2766 before; the deleted
  tickle test is the difference). `cargo fmt --all --check` clean.
- `xtask archive-check` → no primary source outside the 5 accounted-for trees.
- `xtask authority-manifest` → 102 entries, **0 duplicates, pinned 0**, "OK —
  every entry resolves and every source is listed".
- `sha256sum -c legal/SHA256SUMS` → **42 OK** (47 before), and `legal/SOURCES.md`
  now states 42 in both places.
- Round-trip before deletion: `irs-prior/{f8275--2024,i8275--2024,f8283--2025}`
  all HTTP 200 and hash-exact.
- B1 kill, on the rewritten assertion, both directions observed red then
  reverted (rise: a planted `2026/f8949--2026.pdf`; fall: pin set to 1 at zero
  duplicates). Manifest byte-identical to pre-plant afterwards.
- Tracked files still naming a deleted path: `CONTINUITY.md`,
  `archive_check.rs`, `authority_manifest.rs`,
  `design/no-testimony/CONSULT-architect-fable.md`,
  `legal/_provenance/fetch_log.tsv`. All believed deliberate records — **verify
  that judgement**, do not re-enumerate the list.

## CONTEXT YOU NEED

The governing decision and its full reasoning is
`design/agent-reports/2026-09-04-archive-tickle-fable.md` (a Fable verdict, and
this commit executes its §3). Its dispatch brief is
`design/agent-reports/BRIEF-archive-tickle.md`. Two claims in that report were
found WRONG by the controller and corrected before execution — see `31cc7d36`'s
message. Read both; you are reviewing the execution, not re-litigating the
decision.

## OUT OF SCOPE

- Do **not** re-argue whether retiring the tickle was right. That was decided by
  the owner. Review the execution.
- Do **not** propose new checkers or harness features. FR-25 already files the
  known gap (no manifest-freshness test) with its reproduction.
- Do **not** re-audit anything before `945d1ac2`.
- Do **not** hand-count what the commands above already counted.

## SEVERITY

Per `/scratch/code/CLAUDE.md`: Critical = wrong result / data loss / unmet
guarantee. Important = real defect, missing case, unsound assumption. A gate
that cannot fail, a refusal that does not refuse, or a test reporting a false
PASS is **blocking**. Minor/Nit are recorded, not blocking.

★ Pay particular attention to **a guarantee that quietly stopped being enforced**
— that is the single most likely defect shape in a commit that deletes a test.

## OUTPUT

Write your report **as your final action** to exactly:

    design/agent-reports/2026-09-04-archive-resolution-review.md

Structure: **VERDICT** (counts: nC/nI/nM/nNit) · **FINDINGS** (each with
severity, `file:line`, why it is wrong, and the smallest fix) · **WHAT I
VERIFIED AND HOW** (the command, and its output) · **WHAT I COULD NOT CHECK**.

Return to the controller **only** a ≤ 8-line summary plus that path.
