# BRIEF — round 4, on the guard and its test (`101434dc..HEAD`)

One agent, opus. **No subagents. Do not commit, push, or modify any tracked
file** — the only file you write is your report.

## WHY THIS ROUND EXISTS, AND WHAT WOULD MAKE IT THE LAST

Three rounds have run on this branch. **Every one found its Importants in the
previous round's FOLD** — not in the original work. So this round is scoped at
the newest fold and the fix that followed it.

**The honest question is not "is anything wrong" — it is "has this converged."**
Say so plainly in your verdict. A clean result closes the loop; do not
manufacture findings to justify the round, and do not withhold one to end it.

## SCOPE

    git show 101434dc     # fold of round 3 — rewrote the guard and its test
    git show HEAD         # the corrupt-vs-absent fix that followed
    git diff 101434dc~1..HEAD -- crates/xtask/src/authority_manifest.rs

The whole surface at issue is one function and one test:
`regen_would_drop`, `notes_without_binaries`, `regen`'s guard, and
`regen_refuses_to_delete_a_document_whose_binary_is_missing`.

You may raise a finding anywhere in `main..HEAD` if this work made it wrong.

## THE THREE QUESTIONS

1. **Is the invariant now the right one, and completely enforced?** The guard
   is *"no path currently in MANIFEST.json may disappear from a regen"*, with
   three load outcomes: parsed → compare; unreadable-but-present → refuse;
   absent → proceed. **Find a fourth path to the same data loss.** Consider at
   least: a manifest that parses but is empty or truncated to a few entries; a
   path present in both sets but whose entry is silently rewritten (hash, URL,
   storage) rather than dropped; symlinks; a tree in `KNOWN_ARCHIVES` that does
   not exist; `collect_sources` and `regen_would_drop` disagreeing about path
   spelling on Windows (CI runs there).
2. **Is the test finally load-bearing?** It is mutation-verified red against
   four breakages, listed below — **do not repeat those**. Invent a *fifth*
   plausible mutation and report whether the test survives it. In particular:
   does it still hold if someone reorders the arms, or changes `regen` to write
   to a temp file and rename?
3. **Does anything now claim more than the code delivers?** Round 3 found
   exactly that shape twice — a comment and a follow-up entry both broader than
   what was implemented. Check `regen`'s comments, `regen_would_drop`'s doc,
   `FOLLOWUPS.md` FR-25/FR-26, and `CONTINUITY.md`'s new `--regen` paragraph
   against what the code actually does.

## ALREADY VERIFIED — do not re-derive

At HEAD: `make check` **2766 passed / 12 skipped / 0 failed**; `cargo fmt --all
--check` clean; `archive-check` green; `authority-manifest` 102 entries, 0
duplicates, pinned 0, OK; `sha256sum -c legal/SHA256SUMS` 42 OK.

Mutations already planted and observed **red**, then reverted:

| # | mutation | result |
|---|---|---|
| M1 | refusal block moved below `fs::write` | FAIL — "must be BYTE-IDENTICAL after a refusal" |
| M2 | refusal removed entirely | FAIL |
| M3 | guard re-keyed on notes only (the round-3 blind spot) | FAIL |
| M4 | `Err` from `load` treated as empty again (corrupt ≡ absent) | FAIL |

Real-repo probes, both arms, do not repeat: intact manifest + 60 PDFs absent →
refuses, 102 entries preserved. Corrupt manifest + 60 absent → refuses, file
left byte-identical. Before the HEAD fix the second case regenerated to 42.

Also settled: `regen` has one production call site (`main.rs:88`) and
`authority-manifest` appears nowhere in `.github/`, `Makefile` or `scripts/`.

## OUT OF SCOPE

- Do not re-litigate retiring the archive tickle (owner-approved).
- Do not re-audit at or before `945d1ac2`.
- Do not edit the four persisted agent reports; they are verbatim records.
- Do not propose building FR-26's fetcher — it is filed, with a reason.

## SEVERITY

Critical = wrong result / data loss / unmet guarantee. Important = real defect,
missing case, unsound assumption. **A gate that cannot fail, a refusal that does
not refuse, or a test reporting a false PASS is blocking.** Minor/Nit recorded.

## OUTPUT

Write your report **as your final action** to exactly:

    design/agent-reports/2026-09-04-round4.md

Structure: **VERDICT** (nC/nI/nM/nNit, a plain merge / do-not-merge, and an
explicit **converged / not converged**) · **FINDINGS** · **THE FIFTH MUTATION**
(what you invented, and what happened) · **WHAT I VERIFIED AND HOW** · **WHAT I
COULD NOT CHECK**.

Return to the controller **only** a ≤ 8-line summary plus that path.
