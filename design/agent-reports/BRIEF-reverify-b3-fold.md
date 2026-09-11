# BRIEF — re-verifying the B3 fold

**Artifact under check:** commit `db056c57`, the fold of the B3 whole-branch review. **Not** the review
itself (`587d7a9c`, 1C/2I/0M/1N), and **not** the rest of the branch. Scope is the fold's own diff — 23
files, +1,296/−51 — because a fold is authorship and is the text nobody has independently read.

You are ONE sonnet verifier in an isolated worktree. Read, in order:

1. `design/agent-reports/FOLD-b3-whole-branch-review.md` — what the fold claims it did.
2. `design/agent-reports/2026-09-11-b3-whole-branch-review.md` — the findings it answers.
3. `design/agent-reports/2026-09-11-b3-whole-branch-review-VERIFICATION.md` — the controller's ledger on
   the review, so you do not re-derive the findings.
4. `git show db056c57 --stat` and then the diff itself.

## The one question

> **Does the fold actually close C-1, I-1, I-2 and N-1 — and is every new test capable of failing?**

## Already settled — do not re-spend budget

| settled | by |
|---|---|
| every citation in the B3 report | controller ledger, `22ed5e2a` |
| `make gate` **3571 passed / 12 skipped**, `cargo fmt --all --check` clean, `make docs` no diff | controller, on the main tree at this commit |
| the five xtask instruments (stop-list, box-census, prompt-check, census-join, cite-check) all OK | same |
| the I-1 pin reds on exactly the five rendered prompts (29 comparisons) when `draw_edit.rs:2955` reverts to `f.label` | **controller re-planted it**, red output in the fold commit message |
| all five C-1 kills red under the pre-fix plant (`apply.rs:196` → `tax_year: 0` **and** the `:168-174` reconcile stamp removed) | **controller re-planted it** |
| no expected tax figure or golden changed — all 51 deleted lines inspected individually | controller |
| `apply` has exactly one production caller (`edit/tax_inputs.rs:561`) | controller |

**Do not run the suite to reproduce those numbers.** ★ And know this before reporting any red: a worktree
suite result is **not** comparable to the main tree's. On 2026-09-09 a verifier reported 7 failures in a
worktree and all seven were environmental — the gitignored `design/forms/2026/` PDF fixtures are not shared
across worktrees, and the mandated `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` trips a
hook-lookup test. Never report a suite red unless you can show the mechanism is in the code.

## What to hunt — the questions the controller could not settle by reading

1. **Is the I-1 fix complete, or only complete where it was measured?** `field_label` is called at
   `draw_edit.rs:2955` in `push_field_lines`. **Derive the set:** does any *other* surface put a `Field`'s
   label in front of a filer — another renderer, a row preview, a modal, a panel, a help line, a CLI
   printer? Every such site that draws the static while `apply` hashes the rendered sentence is I-1
   surviving. Do not take the fold's word that `push_field_lines` is the only one; grep `\.label` across the
   workspace and account for each production use.
2. **Can a kill pass while the defect is present?** For each of the ten new tests, ask what it would take
   for the assertion to hold on broken code. Two of the fold's own first drafts were green-and-blind (its
   report says so); assume there is a third. The consequence-2, consequence-4 and store-boundary kills are
   the ones to press hardest, and `every_declaration_field_draws_the_sentence_its_answer_is_recorded_under`
   asserts its own coverage — check that the coverage assertion itself can fail.
3. **Did the nine restated fixtures start testing something different?** The fold gave a year to nine
   fixtures that previously screened at year 0 (`return_1040.rs` ×5, `return_refuse.rs`, `testonly.rs`,
   `spec/sections.rs`, `cmd/tax.rs`). A fixture that now exercises a different branch is a test that
   quietly stopped covering what it was written for. Read each one's assertions against its new year.
4. **Is the new refusal safe?** `RefuseReason::ReturnInputsYearNotStated` is raised **first** in
   `screen_inputs_tiered`, reversing a decision that was deliberate (`return_refuse.rs:2459`). Can it refuse
   a return a filer legitimately holds — a parked draft, a pre-fold stored blob, an `income import` TOML, a
   carryover seed? `get_draft_row` now **refuses** a blob whose `tax_year` disagrees with its row key
   rather than re-labelling it: is that reachable for a real filer, and does the message name an exit?
5. **Can a wrong year be stamped?** `apply` takes `form.year`. Trace where that comes from and whether any
   production path can hand it a year that is not the one the filer opened. A confidently-stamped wrong
   year is worse than year 0, because year 0 now refuses and a wrong year does not.
6. **The four corrected doc comments** (`return_inputs.rs`, `questions.rs`, `return_refuse.rs`,
   `coverage.rs`): are they now **true**? The original defect was four comments asserting an invariant that
   did not hold. A comment that is still wrong in a new way is the same defect.
7. **FOLLOWUPS.md** — FR-118 and FR-119: each has an owning phase, a reproduction, and no collision with an
   existing FR number.

## Severity rules that bind you

- **Critical** — wrong result, data loss, an unmet guarantee, or a defect in what a tool *claims* to have
  done (a gate that cannot fail, a refusal that does not refuse, a test reporting a false PASS).
  **Important** — a real defect, a missing case, an unsound assumption. **Minor / Nit** — recorded only.
- **Secret-handling defects never gate** (owner ruling): file them, do not block on them.
- A **blank** is the normal, correct case on a tax return; assert provenance, never non-blankness.
- **Stop and report rather than build on any premise in this brief you can disprove.** Four briefs in this
  arc have been refuted by measurement, two of them written by this coordinator. If a "settled" row above
  is wrong, that is your most valuable finding — say so and stop.

## Mechanics and output

- Worktree only. `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` if you build. Do not commit, do
  not push, edit no file but your report. Do not spawn subagents.
- Write `design/agent-reports/REVERIFY-b3-fold.md` as your **final action**, then return only: the counts
  (`N Critical / N Important / N Minor / N Nit`), the **absolute path** of the report and of your worktree
  root, and at most five lines of summary. Do not return the report inline.
- Structure: **Verdict** (counts) · **Findings** (severity, `file:line`, mechanism, concrete failure
  scenario) · **Refuted premises** · **Checked clean** (one line per numbered hunt above, saying what you
  actually looked at — this is how the round is judged) · **Out of scope**.
