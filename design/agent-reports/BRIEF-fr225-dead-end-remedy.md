# BRIEF — FR-225 (remainder): the product prints a remedy that does nothing, and omits the one that works

`income answer`'s false *"return: computable"* claim is **already fixed** — do not re-fix it. What is
left is a user-facing dead end on a path the repo owner's own return takes. Work in your own worktree.
Do **not** spawn subagents. Do **not** commit.

## Facts already SETTLED by the controller — do not re-derive

1. **Half 1 is CLOSED.** `crates/btctax-cli/src/year_readiness.rs:325-340` now derives the entry line
   from `ReturnVerdict` and states that only the return's own verdict may print the word
   *"computable"*. `package_computes` carries the no-package case separately. Leave this alone.
2. **Both refusal sites really do refuse**, so the condition is not skippable in any sense:
   `crates/btctax-core/src/tax/return_1040.rs:3431` (`None` ⇒ refuse) and `:3462`
   (`Some(false)` ⇒ refuse), both `RefuseReason::CharitableCwaUnresolved`. Only `Some(true)` computes.
3. **The refusal text names two remedies and neither works.** Controller-verified by reading the
   refusal strings at those sites: they name **`btctax income answer`** and **"remove that gift from
   the deduction"**.
   - `income answer` **asks the question zero times** when an answer is already recorded.
   - **No CLI verb removes a gift.** A grep for remove/delete/drop × gift/contribution/charitable
     across `crates/btctax-cli/src` returns nothing.
4. **The one thing that does reach the question is `--re-answer`** (`crates/btctax-cli/src/cli.rs:700`,
   documented at `:681`), and the refusal text **does not mention it**.

So: the filer is told the return is not computable, handed two cures, and both are dead ends.

## What to fix — and the design tension is the real work

The minimum is that a filer following the printed instructions reaches a working outcome. **How** is
yours to decide and defend; these are the candidates, and they are not equivalent:

- **(a) Name `--re-answer` in the refusal.** Smallest edit. ★ But it sends the filer to a flag that
  re-asks **every** question, to change one — and the brief's view is that this is the weakest option
  precisely because it makes the *filer* absorb a product defect.
- **(b) Make the blocking question re-askable on its own** — i.e. if a recorded answer is what causes
  the return to refuse, that question is not settled, and `income answer` should put it again. This is
  the most principled option because it aligns with the repo's answered-ness architecture, and it
  generalises past this one gift. ★ It is also the largest blast radius: work out which other
  `RefuseReason`s are caused by a *recorded* answer rather than a missing one, and say whether your fix
  covers them or names the boundary.
- **(c) Add a verb that removes a gift from the deduction.** ★★ Handle with care, and read
  `CLAUDE.md`'s *"an entry is testimony"* before designing it. Forgoing a deduction and asserting "I
  have no acknowledgment" are **different acts**; a verb that silently converts one into the other
  would be worse than the dead end it replaces. If you build this, it must be clearly a *forgo*, not a
  retraction of testimony.

**You may do more than one.** State your reasoning for what you chose and what you deliberately did
not. ★ What you may NOT do is make the refusal disappear: per `CLAUDE.md`, *"widening an exemption is
never the safe edit"*, and §170(f)(8)(A) genuinely denies the deduction without a contemporaneous
written acknowledgment. The refusal is correct. Only the remedy is broken.

## Required — B1, seen-red-once
Every fix lands **paired with a test that plants the exact defect and is observed RED**, then green.
The one that matters most: **a test that a remedy named in a refusal's text actually exists and
reaches the question.** ★ Per FR-235, do not write the plant in the checker's own vocabulary — if your
test greps the refusal string for `--re-answer`, a plant that edits that string measures only the
grep. Prefer a test that drives the named remedy and asserts the filer's state actually changes.

★ A dead-end remedy is invisible to every existing instrument here — both oracles agree, the golden
packets are byte-stable, and the refusal fires correctly. It took a journey walk to find. So consider
whether the general defect (*"a refusal names a cure that does not work"*) can be made
machine-checkable across all refusals rather than fixed once for this one; if it cannot, say why.

Answer in your report, one sentence each: **"which test reds when this is reverted?"**, with pasted
red output.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
★ **Never background a gate command and end your turn** (FR-175: agents have stalled exactly this way).

## Persisting your report — READ THIS, it bit the last two agents
The `Write` tool **refuses** report files ("Subagents should return findings as text"). Write it with a
Bash heredoc instead:

    cat > design/agent-reports/REPORT-fr225-dead-end-remedy.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and include the byte count in your summary. If the heredoc is refused too, return the
report inline and say at the top that it is the only copy — never silently trim it.

## Do not touch
- `crates/btctax-oracle-harness/**`, `crates/xtask/src/blockers.rs`, `crates/xtask/src/r15_stop_list.rs`
- `design/forms/extract/` (archived IRS text), any existing `design/agent-reports/REPORT-*`
- `design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt` (frozen baseline)
- `FOLLOWUPS.md` — it is the controller's ledger; put your recommendations in your report instead
