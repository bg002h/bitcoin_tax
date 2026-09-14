# BRIEF — wave 5: drive the TY2026 Schedule A line set, and separate WRONG FIGURE from WRONG LABEL

**Owner-approved 2026-09-13**, after asking a sharp question: *"I thought we intentionally fabricated 2026
unknowns for the purposes of discovering problems though?"* — and they were right. Tier B fabricated two of
its four substitutions on purpose, and those two produced its two best findings.

★ **This parcel needs no fabrication at all.** TY2026 Schedule A's six collisions are already **verified
against a real archived draft** (`design/forms/extract/f1040sa--2026-DRAFT.txt`). So the line set is
**transcribed, not invented** — which is why this is worth doing and why its findings will be trustworthy.

## The question, and it is decision-relevant

Tier A established that **the printed layer cannot see the year**: `schedule_a_lines(ar, line11)` takes
neither a year nor the params, `assemble_printed_forms` is not passed params at all, so the charity block
still sums TY2025's `11 + 12 + 13` and the total is still called 17.

★★★ **What Tier A did NOT establish is whether that produces a WRONG FIGURE or only a WRONG LABEL**, and the
answer decides how urgent FR-162 is:

| | consequence |
|---|---|
| **wrong label** | a right number printed on a wrong line — a defective return, arithmetic intact |
| ★ **wrong figure** | money is wrong, and then **which direction** is the finding |

**For each of Schedule A's six collisions, answer which it is, with the figure.** That is the deliverable.
The six, verified by the controller against both extracts:

| line | TY2025 | TY2026 |
|---|---|---|
| 13 | *"Carryover from prior year"* | *"Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet"* |
| 14 | *"Add lines 11 through 13"* | *"Carryover from prior year"* |
| 15 | Casualty and theft loss(es) | *"Add lines 13 and 14"* |
| 16 | *"Other—from list in instructions"* | Casualty and theft loss(es) |
| 17 | **the TOTAL** → 1040 line 12e | *"Other itemized deductions"*, enumerated 17a–17h+ |
| 18 | ★ **a CHECKBOX** | ★ **the TOTAL** |

★ Line 5e also moves its **figures** while keeping its number — SALT cap $40,000 → **$40,400**, phase-out
$500,000 → **$505,000** (FR-219) — and those ARE already in `ty2026_full_return()` as
`SaltLimitation::Worksheet2025 { line1_cap: dec!(40400), line5_threshold: dec!(505000) }`. So 5e is the
control: a change the chain *does* see. **Use it as such** — a harness that cannot distinguish 5e from 13/14
is not measuring anything.

★★ **The adverse one to price first:** `charitable.rs:32`/`:258` and `return_1040.rs:3918` all document the
prior-year carryover as **"Schedule A line 13"**. In TY2026 line 13 is the worksheet-limited **current-year
gift total**, systematically larger, and the carryover moved to 14. If the chain reads 13 as the carryover it
takes a **bigger** number as the carryover — a larger deduction, **less tax, understatement.** Measure it.

## Shape: a harness, NOT a refactor

**OWNS:** a new test file only — `crates/btctax-cli/tests/ty2026_schedule_a.rs` (Tier A's harness lives in
`btctax-cli/tests/` because a core dev-dep on adapters trips `repo_hygiene`'s pin rule; follow that
precedent and change no manifest). ★ **Change NO shipped source.** Read `printed.rs`, `charitable.rs`,
`return_1040.rs`; edit none of them. Other agents hold `return_refuse.rs`, `tables.rs`,
`interview_state.rs`, `year_readiness.rs` and `cmd/answer.rs` right now.

Build the TY2026 line set from the extract (derive the ids; do not type them), drive the printed chain with
the owner's profile — W-2 + Bitcoin + **itemized** Schedule A + Schedule B — and for each collision report:
what the chain produced, what the TY2026 form requires, and whether the delta is a **label** or a **figure**.

**This validates nothing.** No TY2026 oracle exists. Say so.

## Rules

Own worktree, `CARGO_TARGET_DIR=<worktree>/target-w5`, never `/tmp` (32 GB tmpfs). **FOREGROUND every
command** — FR-175: two agents have stalled by backgrounding a gate, the second *with the prohibition in its
brief*. `make check` excludes `cargo fmt --all --check`; run both. Capture once, grep. **Do not commit, do
not push.** No subagents.
★ Two distinct concurrency failures: `ld.lld: undefined hidden symbol` from a stale `.rlib` →
`cargo clean -p <crate>`; `signal: 9, SIGKILL` → memory, run nextest and clippy serially.
★★ **Thirteen briefs in this arc were refuted by their implementer, nine of them mine. If you can disprove a
premise — including that any of the six is really a collision — STOP and report it.**

Deliverable: Bash heredoc (**not** `Write`) → `design/agent-reports/REPORT-wave5-schedule-a.md`.
