# BRIEF — the archive-reconciliation tickle is red, and `main`'s CI with it

**Dispatched** 2026-09-04 · one agent, Fable · **no subagents, no parallelism.**

## THE ONE QUESTION

`main`'s CI is red on exactly one test: the dated archive-reconciliation tickle,
now **7 days past** its `review-by`. The gate's own doc forbids the easy fix
(*"Do not simply push the date out"*). **What is the right way to get `main`
green — honouring the gate's stated intent rather than defeating it?**

Answer with a **recommendation**, not a survey. Rank options by what best serves
the project's actual objective: emitting a correct, signable US federal 1040.
Archive hygiene serves that only insofar as it protects the
transcribe-from-primary-source discipline.

Three candidate shapes, and you may propose a fourth:

- **(a)** A second dated reset. If so: what date, and what recorded reason? Or is
  a second reset exactly the decoration the gate warns about?
- **(b)** Actually resolve one or both duplicate groups now. If so: what
  concretely, and what breaks (see consumers below)?
- **(c)** A structural change to the gate itself — e.g. it conflates *"a
  duplicate exists"* with *"a decision is owed"*, or it fails the **whole suite**
  (reddening CI **and** blocking every commit) over a documentation-hygiene
  question with no bearing on filing correctness.

## SETTLED FACTS — machine-verified 2026-09-04. Do NOT re-derive these.

### CI state (`gh run view 33351747634`, commit `945d1ac2`)

| result | jobs |
|---|---|
| green (6) | examples, pii-scan, fmt, msrv, net-isolation, clippy |
| red (3) | test (ubuntu-latest), test (windows-latest), test (macos-latest) |

All three red jobs fail the **same single test**:
`xtask::archive_check::tests::the_archive_reconciliation_is_not_past_its_review_by`.

Cause is purely the date. `ARCHIVE_RECONCILIATION_REVIEW_BY = "2026-08-28"`
(`crates/xtask/src/archive_check.rs:193`); today is `2026-09-04` UTC. **The
previous `main` (`3fc88497`) carried the identical date** — this is not
merge-induced, and `main` would be red today either way.

Live reproduction:

```
$ cargo run -q -p xtask -- archive-check
archive-check: no primary source outside the 5 accounted-for tree(s)
archive-check: 3 archive(s) pending reconciliation (CONTINUITY.md §0 step ③), review-by 2026-08-28  ** OVERDUE **
xtask archive-check: the four-archive reconciliation is PAST its review-by (2026-08-28, today 2026-09-04).
```

### The gate's two halves

1. **The ratchet** (`strays()`) — no *new* archive may appear. **Currently
   CLEAN**, and independently proven by a planted-defect test
   (`a_third_archive_at_a_novel_path_is_caught`). This half is not in question.
2. **The tickle** — the 3 existing archives must be reconciled by `review-by`.
   This is the half that is red.

`KNOWN_ARCHIVES` = 3: `design/forms`, `legal/primary-sources`, `legal/text`.

The test's own doc, verbatim:

> ★ **Do not simply push the date out.** That converts the tickle into the
> decoration it exists to prevent. Re-decide, and record what changed.

The **RESET LOG** (`archive_check.rs:183-185`) has exactly **one** entry:
2026-08-13 → 2026-08-28, decided 2026-08-20 by owner, reason *"model usage is
expected to be more available"*. The log's own framing, verbatim:

> Two entries here is a decision; five is the gate being routed around, and that
> must be visible without reading git history.

A second reset would therefore be entry **#2** — which by that sentence's own
words is still inside "a decision". Weigh that; do not assume it settles the
question.

### The residual duplication — measured by sha256, exactly 7 groups

Matches `DUPLICATE_SOURCE_GROUPS = 7`
(`crates/xtask/src/authority_manifest.rs:139`). Full walk of `design/forms` +
`legal/primary-sources`:

**Group A — a year-named path aliasing `periodic/` (3 groups):**

```
design/forms/2024/f8275--2024.pdf   == design/forms/periodic/f8275.pdf
design/forms/2024/i8275--2024.pdf   == design/forms/periodic/i8275.pdf
design/forms/2025/f8283--2025.pdf   == design/forms/periodic/f8283.pdf
```

**Group B — the genuine (A)/(B) convention overlap (5 groups):**

```
design/forms/2025/f8949--2025.pdf   == legal/primary-sources/irs-forms/Form_8949.pdf
design/forms/2025/i8949--2025.pdf   == legal/primary-sources/irs-forms/Instructions_8949.pdf
design/forms/2025/f1040sd--2025.pdf == legal/primary-sources/irs-forms/Schedule_D_1040.pdf
design/forms/2025/i1040sd--2025.pdf == legal/primary-sources/irs-forms/Instructions_Schedule_D.pdf
design/forms/2025/f8283--2025.pdf   == legal/primary-sources/irs-forms/Form_8283_Noncash_Charitable.pdf
```

`f8283` is a **3-way** duplicate and is counted in both groups: 3 + 5 − 1 = 7.

### Consumers of the (B)-side and `periodic/` files (grep, `.claude/worktrees` excluded)

- `legal/SOURCES.md` §3 indexes **all five** (B)-side files **with their sha256
  prefixes** — deleting the binaries strands that table.
- `legal/_scripts/fetch_sources.sh:48-54` fetches each by IRS URL.
- `legal/research/ADDENDUM_open_questions_verified.md` cites
  `Instructions_Schedule_D.pdf` (line 40) and `Form_8283_Noncash_Charitable.pdf`
  (line 90) **by path**.
- `design/forms/MANIFEST.json` carries entries for
  `design/forms/periodic/{f8275,f8283,i8275}.pdf`.
- `crates/btctax-forms/forms/2024/f8283.map.toml:83` cites
  `design/forms/periodic/f8283.pdf`, noting the **bundled runtime asset is Rev.
  December 2023 while the periodic copy is Rev. December 2025**.

### Why `legal/primary-sources/` cannot simply be deleted

It holds the rungs `design/forms/` lacks: **16 × 26 USC** (rung 4 — the statute,
the only actual law) and **6 × 26 CFR** (rung 3). Only its `irs-forms/` overlap
is at issue.

### Blast radius while red

`git config core.hooksPath` = `scripts`; `scripts/pre-commit` runs `make check`
(nextest + clippy) and blocks the commit on red. `--no-verify` is separately
denied at the tool layer by `scripts/hooks/deny-bypass.sh`. **So while this test
is red, every commit in the repo is blocked** — including a pending
`CONTINUITY.md` update, currently uncommitted.

### Other standing context

- Repo is **public**; MIT OR Unlicense; **no users yet** (back-compat is not
  sacred).
- Secret-handling defects never gate (owner ruling 2026-08-27) — not relevant
  here, stated so you don't reach for it.
- Neither tax oracle witnesses archive layout. Do not propose an oracle check.

## OUT OF SCOPE — do not do these

- Do **not** re-audit the `feat/filing-readiness` branch that just merged. It
  passed a whole-branch Fable seam review at 0C/0I and is not the question.
- Do **not** hand-count anything above; every count is measured — use it.
- Do **not** spawn subagents.
- You **may** propose disabling or restructuring the test — but if you do, say
  plainly that that is what you are proposing, and argue why it is not the
  muting the gate's own doc warns against. An unacknowledged mute is the one
  answer that is worse than no answer.

## OUTPUT

Write your report **as your final action** to:

    design/agent-reports/2026-09-04-archive-tickle-fable.md

Structure:

1. **VERDICT** — one paragraph: the recommendation.
2. **WHY** — ≤ 6 bullets.
3. **EXACT ACTIONS** — numbered, concrete: `file:line`, what changes to what.
   If you recommend a reset, give the **precise reset-log row text** to paste.
4. **WHAT THIS COSTS** — what is given up; what would go undetected.
5. **REJECTED ALTERNATIVES** — ≤ 4 lines each.

Return to the controller **only** a ≤ 10-line summary plus that path.
