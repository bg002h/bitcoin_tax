# BRIEF — re-review the B3 fold (`f6d91e59`), the last gate before merge

One agent, opus. **No subagents. Do not commit, push, or modify any tracked
file** — the only file you write is your report.

## WHY THIS ROUND EXISTS

The workflow says *re-review after every fold, including the last.* On this
branch that is not ceremony: the previous round found **3 Important**, and
**two of the three lived in a fold** that the round before it had never seen.
A fold is authorship and re-earns the gate.

This fold is the riskiest one yet, because unlike its predecessors it adds
**new production code** — a refusal that gates a destructive command — plus a
new test that is now the sole holder of a guarantee.

## SCOPE

    git show f6d91e59            # the fold under review
    git log --oneline main..HEAD # 10 commits, for context

Review `f6d91e59` **in the context of the whole branch**. You may raise a
finding anywhere in `main..HEAD` if the fold made it wrong or left it wrong.

## THE THREE QUESTIONS

1. **Is the new refusal CORRECT — and is it correctly SCOPED?** `regen` now
   returns `Err` when any provenance note's binary is absent
   (`authority_manifest.rs`, `notes_without_binaries` + the guard at the top of
   `regen`). Ask both directions:
   - **Too narrow?** Is there a way to still destroy manifest entries that this
     does not catch — a document with no note at all, a note in a tree the walk
     misses, a path shape `collect_notes` classifies differently from
     `collect_sources`?
   - **Too broad?** Does it block a legitimate workflow? What is a maintainer
     supposed to *do* when it fires, and does the message actually tell them?
2. **Is `regen_refuses_to_delete_a_document_whose_binary_is_missing` a REAL
   kill, or does it merely pass?** It was observed red on a planted defect
   (refusal neutered) and green when restored. Is the assertion strong enough
   that a *different* plausible breakage still reds it? Is its positive control
   real, or does it pass for an unrelated reason?
3. **Are the corrected claims now TRUE?** This is the third attempt at the same
   sentence. `c6c4a7dc` said the removed `assert!` had "stopped
   discriminating"; `36c6d12b` corrected that to "never weakened at any pin",
   which was **also wrong**; `f6d91e59` now says the fall arm is dormant at 0
   and that the tripwire is replaced deliberately. **Read the current wording
   and decide whether it is finally accurate** — in
   `authority_manifest.rs` (the pin doc + the assertion body comment) and in
   `archive_check.rs` (the RETIRED block and the shape doc).

## ALREADY MACHINE-VERIFIED — do not re-derive

At `f6d91e59`, all green: `make check` → **2766 passed / 12 skipped / 0
failed**; `cargo fmt --all --check` clean; `xtask archive-check` green; `xtask
authority-manifest` → 102 entries, 0 duplicates, pinned 0, OK; `sha256sum -c
legal/SHA256SUMS` → 42 OK.

**The refusal cannot break CI, settled before dispatch:** `regen()` has exactly
three call sites — `main.rs:88` (the CLI) and the fold's own two test calls,
both on `tempfile` roots. No test invokes it on the real repo root, and
`authority-manifest` appears nowhere in `.github/`, `Makefile` or `scripts/`.
Do not spend budget re-checking that; **do** tell me if you find a fourth call
site or a way CI reaches it.

**The I-1 regression is real and was reproduced twice** (by the reviewer and
independently by me): with the 60 gitignored PDFs absent, the pre-fold
`--regen` rewrote the manifest 102 → 42 entries with every instrument green,
and the same manifest reds at `main`'s pin of 7. Not in question.

## OUT OF SCOPE

- Do not re-litigate retiring the archive tickle (owner-approved).
- Do not re-audit at or before `945d1ac2`.
- Do not edit the three persisted agent reports; they are verbatim records, and
  a claim in one being wrong is expected and already recorded.
- Do not re-file FR-23/24/25; FR-25 was re-scoped in this fold — say only
  whether that re-scoping is now correct.

## SEVERITY

Critical = wrong result / data loss / unmet guarantee. Important = real defect,
missing case, unsound assumption. **A gate that cannot fail, a refusal that
does not refuse, or a test reporting a false PASS is blocking.** Minor/Nit
recorded, not blocking. Secret-handling never gates (owner ruling 2026-08-27).

★ The likeliest defect shape here is **a refusal that does not refuse**, or a
kill test that would pass against the very regression it was written for. Hunt
those first. Second likeliest: the corrected wording is wrong a third time.

## OUTPUT

Write your report **as your final action** to exactly:

    design/agent-reports/2026-09-04-b3-fold-rereview.md

Structure: **VERDICT** (nC/nI/nM/nNit, and a plain merge / do-not-merge) ·
**FINDINGS** (severity, `file:line`, why, smallest fix) · **THE THREE
QUESTIONS, ANSWERED** · **WHAT I VERIFIED AND HOW** (command + output) ·
**WHAT I COULD NOT CHECK**.

Return to the controller **only** a ≤ 8-line summary plus that path.
