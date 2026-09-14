# BRIEF — FR-199: `income clear` may destroy a committed return's answers with no guard

A **data-loss** report that has never been reproduced. Reproduce it first; fix only what reproduces.
Work in your own worktree. Do **not** spawn subagents. Do **not** commit.

## Q1 — REPRODUCE, before fixing anything

FR-199 says `income clear` destroys a committed return's recorded answers with no guard, in two halves:

- **(a) the asymmetry** — a work-in-progress DRAFT demands an explicit `--discard-draft`, while a
  COMMITTED return demands nothing.
- **(b) the inverted invariant** — after clearing, the packet prints the **Digital Asset question as
  unanswered**, i.e. the very answer `clear` deleted. A destructive command *manufacturing*
  un-answeredness is worse than either state alone.

**What the controller established, so you do not redo it:** `Clear` at `crates/btctax-cli/src/cli.rs:606`
takes exactly two arguments — `year` and `discard_draft` — so there is indeed **no** committed-return
guard in the CLI surface. `--discard-draft` itself is real and well-documented at `:610-618` (without it
such a write is REFUSED). Files mentioning `discard_draft`: `cli.rs`, `main.rs`, `cmd/tax.rs`,
`cmd/answer.rs`, `input_form_store.rs`, `open_next_year.rs`.

**What is NOT established and is yours to settle:** whether a *committed* return's answers are actually
destroyed, and whether (b) happens. Drive it. ★ Use an **isolated vault** — set `HOME` and
`XDG_DATA_HOME` to a scratch directory (the CLI resolves its store via `dirs::data_dir()`), and confirm
your isolation held by checking files actually appear under it. The controller tried this and the
override did **not** redirect the store, so the run reached a real vault instead; work out the correct
isolation and **say what it is** in your report, because every future journey test needs it.

★ If either half does not reproduce, that is a full and valuable answer — say so, and correct the
FR-199 entry's wording. Do not manufacture a defect to have something to fix.

## Q2 — fix what reproduces

Likely shape, but yours to decide and defend:
- **(a)** a committed return is the STRONGER artifact, so if a draft needs an explicit flag a committed
  return needs at least as much. ★ Do not simply copy `--discard-draft`'s spelling; say what the right
  refusal is and what the operator must type to mean it.
- **(b)** is the more serious half if it reproduces, and it is **not** a display bug. Read `CLAUDE.md`'s
  *"an entry is testimony"* and *"blank is the normal case"*: a question that WAS answered and now
  prints unanswered is not a blank the inputs produced, it is testimony deleted and then silently
  re-asked. The invariant is provenance, not value. ★ A `0`/`unanswered` on a line the filer did answer
  **fabricates the absence of testimony** just as surely as a hardcoded `0` fabricates its presence.

★ This is a destructive-command defect, so consider the general case rather than only `clear`: is there
any other verb that can delete recorded answers without an explicit acknowledgement? Say what you
checked and what you found — a derived answer beats a spot fix, per *"derive the list, or make the
compiler hold it."*

## Required — B1, seen-red-once
Every fix lands **paired with a test that plants the exact defect and is observed RED**, then green.
For a data-loss guard the kill is: the destructive path runs **without** the acknowledgement and the
test asserts the answers **survive** (or the command refuses). ★ Per FR-235, do not write the plant in
the checker's own vocabulary. ★ And per `CLAUDE.md`, *"a guarantee without a test that reds when it is
removed does not exist"* — this one guards a filer's recorded testimony, so it must not be the
exception.

Answer in your report, one sentence each: **"which test reds when this is reverted?"**, with pasted red
output.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
★ **Never background a gate command and end your turn** (FR-175: agents have stalled exactly this way).
★ If you change a clap doc comment, `docs/man/**` is generated from it — regenerate, or
`gen_docs_is_deterministic` reds.

## Persisting your report
The `Write` tool **refuses** report files. Use a Bash heredoc:

    cat > design/agent-reports/REPORT-fr199-clear-data-loss.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and give the byte count in your summary. If that is refused too, return the report
inline and say at the top that it is the only copy — never silently trim it.

## Notes on your tree
Your worktree branches from an OLDER base than main, not from HEAD — possibly a dozen commits behind.
Do not try to catch up and do not rebase; just be aware that recent work (an abort census in
`xtask/src/blockers.rs`, an `oracles` map in the oracle harness, a
`btctax-cli/tests/refusal_remedies.rs`) may be absent, and that is expected.

## Do not touch
- `crates/btctax-oracle-harness/**`, `crates/xtask/**`
- `design/forms/extract/` (archived IRS text), any existing `design/agent-reports/REPORT-*`
- `FOLLOWUPS.md` — the controller's ledger; put recommendations in your report
