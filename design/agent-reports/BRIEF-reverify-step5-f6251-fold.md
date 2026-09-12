# BRIEF — re-verifying the step-5 fold (`6356043e`)

**Artifact:** commit `6356043e`'s own diff — 11 tracked files, **+817/−79**, plus its report. Not the
build it repairs (`d8d023af`), not the review it folds, not the rest of the branch. A fold is authorship
and is the text nobody has independently read.

You are ONE sonnet verifier in an isolated worktree. Read, in order:

1. `design/agent-reports/FOLD-step5-f6251-review.md` — what the fold claims.
2. `design/agent-reports/2026-09-11-review-step5-f6251-obbba.md` — the review it answers (0C/2I/3M/1N).
3. `design/agent-reports/2026-09-11-review-step5-f6251-obbba-VERIFICATION.md` — the controller's ledger
   on that review; do not re-derive what it settles.
4. `git show 6356043e --stat`, then the diff.

Also `CLAUDE.md` — "Derive the list, or make the compiler hold it", "Blank is the normal case", and
harness rule B1 (no checker exists until seen red on a planted defect).

## The one question

> **Does the fold actually close the collision — and is every new check capable of failing?**

## Already settled — do not re-spend budget

| settled | by |
|---|---|
| `make gate` **3594 passed / 12 skipped, exit 0**, `cargo fmt --all --check` clean | controller, main tree at this commit |
| `xtask line-coverage` **OK, 377 money lines, f6251:43** (was 375/41), all three ratchets unmoved | controller, ran the CLI |
| **the central kill, all three layers** — L1 the prescribed edit gives `E0559 … no field named schedule_1a_l37 … available fields are: senior_deduction`; L2 the post-rename arm **compiles** and core's only red is the year list (so the emitter join is load-bearing); L3 neutralising the comparison reds `a_revision_citing_line_43_refuses_a_figure_read_off_line_37` | **controller re-planted all three** |
| `SeniorDeductionSubtotal`'s fields are private with accessors only (`schedule_1a.rs:1419-1422`) | controller |
| FR-120/121/122 filed with owning phases | controller |
| **no expected value moved**: `dec!(34000)` and `dec!(6000)` survive, the assertion only re-worded; the TY2024 line-1 coverage row **moved inside** the new `_`-free match rather than being deleted (`line_coverage.rs:702`), net +2 rows | controller, read all 79 deletions |
| `xtask line-coverage` is run by no test and no CI job (FR-122) | controller, no hits in `.github/workflows/` or the Makefile |

★ **Do not run the suite to reproduce those.** A worktree suite result is not comparable to the main
tree's: on 2026-09-09 a verifier reported 7 failures in a worktree and all seven were environmental
(gitignored `design/forms/2026/` PDF fixtures are not shared across worktrees; the mandated
`CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` trips a hook-lookup test). Never report a suite
red without showing the mechanism is in the code.

## What to hunt — the questions reading the report cannot settle

1. ★★ **Is `SeniorDeductionSubtotal` genuinely unforgeable?** Private fields are only half an argument.
   Derive every construction path: does it derive or implement `Deserialize`, `Default`, `From`, `Clone`
   from something forgeable? Is there a constructor exposed through `testonly`, a `pub(crate)` helper, or
   a builder? **Serde is the specific hole to check** — this crate is serde-heavy and `ReturnInputs`
   round-trips through TOML, so a `Deserialize` impl would let a file supply a subtotal tagged with any
   line number it likes. If the type is forgeable by any route, the join it feeds is decoration.
2. **Does EVERY path that fills an OBBBA Form 6251 go through the join?** The join protects only what
   calls it. Derive the callers — do not accept "the emitter calls it". Look for a second fill path (a
   slice export, a snapshot, a golden generator, a test helper that bypasses the public entry) that could
   print line 1a without the comparison.
3. **Are the two corrected comments now TRUE?** The root cause of this whole round was two doc comments
   that were confidently wrong. Read both replacements against the extracts. A comment wrong in a new way
   is the same defect, and this repo has carried false comments through folds twice.
4. **Do the two new coverage rows quote the real extract, and does the kill discriminate per row?** A
   kill that reds only on "0 rows vs 2" would pass if one row were right and the other rotted. Check what
   the M-1 test actually asserts, and whether it would catch a single wrong sentence.
5. **I-2's ported sweep** — does the read-back **compare values**, or only check presence? Is the
   `checked == 42` total derived from the map, or typed? A presence-only read-back on 29 newly covered
   cells would be a false PASS on exactly the cells the fold exists to cover.
6. **N-1's fix** — does it fail closed on *multiple* occurrences as well as zero, and does the new
   ambiguity test actually distinguish the two?
7. **Anything the fold changed that the review did not ask for** — scope creep in either direction: an
   item silently dropped, or an edit beyond the six.

## Severity

**Critical** — wrong result, data loss, an unmet guarantee, or a defect in what a tool *claims* to have
done (a gate that cannot fail, a refusal that does not refuse, a false PASS). **Important** — a real
defect, missing case, unsound assumption. **Minor/Nit** — recorded. ★ Secret-handling defects never gate.
A **blank** is the normal case; assert provenance, never non-blankness.

**Stop and report rather than build on any premise here you can disprove.** Seven briefs in this arc have
been refuted by measurement; three were the coordinator's. If a "settled" row above is wrong, that is your
most valuable finding.

## Mechanics and output

Worktree only; `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` if you build. Do not commit, do
not push, edit no file but your report. **Do not spawn subagents.**

Write `design/agent-reports/REVERIFY-step5-f6251-fold.md` as your **final action**, then return only: the
counts (`N Critical / N Important / N Minor / N Nit`), the **absolute path** of the report and of your
worktree root, and at most five lines of summary. Do not return the report inline.

Structure: **Verdict** · **Findings** (severity, `file:line`, mechanism, concrete failure scenario) ·
**Refuted premises** · **Checked clean** — one line per numbered hunt, saying what you actually looked at;
this is how the round is judged · **Out of scope**.
