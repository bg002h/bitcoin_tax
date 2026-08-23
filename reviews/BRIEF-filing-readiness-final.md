# BRIEF — the FINAL review: whole branch, `main..HEAD`, pointed at INTERACTION

## Why this pass exists, in one paragraph

Harness **B3**: *a per-range review is not a branch review.* This branch has already had three
reviews, each thorough inside its window, and **the two most serious defects on it were both found
outside every window** — one by the controller while merging, one by a reviewer given a range that
happened to span two phases. A stack of per-phase reviews does not add up to a branch review.

> **What is wrong that no per-phase review could have seen?**

## SCOPE

`git diff main..HEAD` — **34 commits, 75 files, +8465 / −217**, across `btctax-core`,
`btctax-cli`, `btctax-forms`, `btctax-input-form`, `xtask`, the oracle scripts, the form maps, and
the docs. `git log main..HEAD` first: every commit message carries its mutations with verbatim RED
output, so nothing below needs re-deriving.

**This is the last review before the branch is handed to its owner.** It will NOT be merged, tagged
or published by me. Your verdict is the final quality signal.

## ★★★ ALREADY MACHINE-CHECKED — do not re-run, do not re-derive

- **2740 tests pass** (baseline at `main` was 2667), `clippy -D warnings` clean,
  `cargo fmt --all --check` clean, `pii-scan` clean, `xtask check-isolation` clean.
- **~50 mutations** planted and observed RED across the phases, each pasted into its commit message.
- `verify_f6251.py` over 31 vectors, BOTH engines: 0 unexpected on taxcalc; 759 lines vs OTS, 0
  unexpected; the Tier-2 attach gate clear on all four filing statuses.
- The oracle corpus regenerates reproducibly (104/104 households identical before each change).

Spend your budget on what tools cannot reach.

## ★★ WHAT THE EARLIER ROUNDS COVERED — so you can skip it

Three reviews ran. Two are persisted **in the repo, verbatim**; read them rather than re-deriving:

| what | where | verdict |
|---|---|---|
| Phase 1 (three parallel lanes), seam-scoped | `reviews/filing-readiness-phase1-review.md` | needs-changes → 1 I + 4 M folded, 1 N filed |
| Phase 2 (the refusal surface) + the phase-1 folds | `reviews/filing-readiness-phase2-review.md` | needs-changes → **1 C** + 2 I + 3 M folded |
| Phase 4 (attestation KATs + docs) | running in parallel with you; **not yours to duplicate** | — |

**Phase 3 (Form 8960 line 9b) has had NO independent review.** It is one commit,
`merge(phase 3)`, and it is the largest un-reviewed surface in this range. Weight accordingly.

## ★★★ THE FOUR SEAM DEFECTS ALREADY FOUND — your calibration, and all FIXED

Do **not** re-report these. They are here to show what "interaction defect" means on this branch.

1. **P2a refused what P2b made fileable.** A CLI overflow preflight and a forms-layer paginator,
   built in parallel worktrees. Both kill-tests passed while asserting **opposite** things about the
   same 16-leg filer. Two green suites, one broken product.
2. **N1 and the frozen M4 check gave one filer contradictory sworn figures**, and M4 phrased the
   wrong one as an audit of the right one. Nobody touched M4 — the *base tree* was the second lane.
3. **P7 changed a variant that lane A's test used**, and P4's new gate refused three of lane B's
   fixtures. Both surfaced only at merge, because phase 2 branched from the wrong base.
4. **THE CRITICAL: two refusals sat in `screen_inputs`, which cannot see the §63(e) election**, so
   they refused standard-deduction filers for whom the lines never print. Moved to
   `screen_absolute` behind `ar.deduction_is_itemized`.

## ★ WHERE I WOULD LOOK — offered so you can disagree

1. **The refusal surface as a WHOLE.** Five refusals were added across two phases
   (`MortgageDebtLimitUnanswered`, `MortgageOverDebtLimit`, `CharitableCwaUnresolved`,
   `Form4952DeclarationUnanswered`/`Form4952Required`, `Nii9bExceedsDeductedSalt`). Each was reviewed
   against *its own* population. **Is there a filer who trips two at once, or one whose only escape
   from A is to trigger B?** That is the question no per-item review asked. The Critical was exactly
   this shape in miniature — a filer for whom all three answers refused.
2. **Phase 3 (line 9b) against phases 1–2.** It adds a collected input, a refusal in
   `screen_absolute`, and an advisory — the same surfaces phase 2 rebuilt. It merged with zero
   conflicts, which is reassuring about text and says nothing about semantics.
3. **The §170 cluster now spans three phases**: P5's appraisal advisory + manifest line (phase 2),
   P4's CWA gate (phase 2, later widened to ceiling-zeroed years), P6's carryover display (phase 1),
   the 8283 column-(i) blanking (phase 1), and the Part IV/V hand-marks (phase 1 + the merge). Do
   they agree about *what a charitable deduction is* — contributed vs claimed vs allowed vs carried?
   Four different measures are in play and each was adjudicated separately.
4. **Blank vs zero across the whole branch.** Phase 3 established that line 9d still prints `0` when
   9b is blank. N3 was deliberately not built. Is there any NEW line on this branch that prints a
   zero nobody testified to?
5. **The three docs that make promises**: `LIMITATIONS.md` (rewritten in phase 4),
   `design/direction/ADJUDICATION-2026-08-21.md` (carries an erratum), and the refusal message texts
   themselves. A refusal message is the only remedy a refused filer gets. Do any of them now describe
   behaviour the branch changed?

## FORBIDDEN

- Re-reviewing phases 1, 2 or 4 for individual correctness — done, and two reports are in the repo.
- Re-reporting the four seams above.
- Re-litigating `ADJUDICATION-2026-08-21.md` (D3/D4/D5/D7, settled against primary sources; its one
  known error already carries an erratum).
- Style, prose, naming, doc-comment length.
- **Deliberately open items** — do not re-list: N3 (needs a core lane); TY2017 8283 column (i); the
  TUI carryover reader; the L0 filing-threshold note; the IN-side carryforward refusal;
  `--write-carryover` not rolling the capital-loss sibling; F6's 2025 census; line 9d's zero; Form
  8960 line 9a (newly derivable); EITC/ACTC. You **may** argue a deferral was the wrong call.

## OUTPUT FORMAT

```
VERDICT: <sound | needs-changes | wrong-shape>

CROSS-PHASE FINDINGS:
  For each:
    THE SEAM: <which phases/files, and why no single review could see it>
    THE FAILING CASE: <concrete inputs -> wrong output, wrong refusal, or false statement>
    SEVERITY: <C | I | M | N>

PHASE 3 FINDINGS: <the un-reviewed surface, whatever severity>

IS THE REFUSAL SURFACE COHERENT AS A WHOLE? <yes/no + the filer you tested hardest>

IF SOUND: say so plainly, and name the three things you attacked hardest and how.

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence>
```

Critical only for: a wrong figure on a filed return, data loss, a false safety claim, a false
statement in a promise-surface doc, or **a refusal that stops a filer entitled to file** — that last
one is as harmful here as a wrong number, and this branch has already produced one.

A verdict of `sound` is a real and useful outcome. Do not manufacture findings to justify the pass;
equally, do not soften a Critical because three reviews and 2740 green tests preceded you — that is
precisely why an unfound one would still be here.

**Return findings as TEXT.** The harness blocks subagents from writing report files; I will persist
your output verbatim, in its own commit, before folding any of it.
