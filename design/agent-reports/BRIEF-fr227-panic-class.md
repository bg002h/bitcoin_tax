# BRIEF — FR-227: does the 2026 panic exist on a USER path, and can `blockers` see the class at all?

Two questions, in this order. The second is the deliverable; the first decides what the second must
cover. Work in your own worktree. Do **not** spawn subagents of your own. Do **not** commit.

## Q1 — REACHABILITY. Reproduce the user-facing panic, or correct the entry.

`FOLLOWUPS.md` FR-227's headline is *"`report --tax-year 2026` PANICS rather than refusing."*
**The controller could not reproduce that**, and you must settle it before anything is "fixed":

    cargo run -q -p btctax-cli --bin btctax -- report --tax-year 2026
    → error: io: No such device or address (os error 6)        # a clean error, NOT a panic

So either the panic needs a specific input shape the bare command does not supply (a ledger, a stored
return, a TTY), or the headline overstates what is reachable. **Establish which, with a pasted
reproduction or a pasted failure to reproduce.** If a user *can* reach it, that is a blocking
user-facing defect and it should become a refusal carrying a `RefuseReason` — the product already has
`ReturnVerdict`/`RefuseReason` machinery for exactly this, so do not invent a new mechanism. If a user
*cannot* reach it, say so plainly and **correct the FR-227 entry's headline** rather than leaving a
false claim on the ledger.

★ One measurement you should not repeat: the controller already checked the confirmed live site
`crates/btctax-core/src/tax/form6251.rs:845`. It is a **KAT-fixture comparison helper** (*"fixture has
no `year`, so it is TY2024 — but compute produced the TY2025 Part I shape"*), i.e. vector-verification
support that happens to live in `src/`, not the `report` path. Do not present it as the user panic
without showing a call chain from the CLI.

## Q2 — THE INSTRUMENT. `xtask blockers` must cover the abort class, or say that it does not.

FR-227's own words: *"have `blockers` derive the panicking/unhandled-year class too, or state in its
own output that it does not cover it. An honest boundary is reviewable; a silent one is the defect."*
`blockers` derives year walls from `RefuseReason` sites and year-keyed gates; a `panic!` is neither, so
the class is currently **invisible** to it.

**Prefer deriving it.** Follow the existing shape in `crates/xtask/src/blockers.rs`: `refusal_census`
enumerates every `RefuseReason::…` construction and then classifies each, and `gates()` carries
`keyed_by` attribution. An abort census should be the same shape — enumerate, classify, and state its
own blind spots in the OUTPUT the way the refusal census already does (see the *"WHAT IT DOES NOT
CLAIM"* line it prints).

### Two counts that disagree, and why you must not pin either
- Tier B measured **9** instances of *"a typed per-year list inside a shipped crate with no TY2026
  arm."*
- The controller measured **4** year-ish abort macros outside `#[cfg(test)]` modules, with a
  brace-tracking script: `form6251.rs:845`, `form6251.rs:1114`, `year_record.rs:121`,
  `btctax-tui-edit/src/main.rs:24399`.

**These are different classes, and neither number is the answer.** A per-year list with no 2026 arm may
fall through via `_ => None`, `expect()`, or a default — no `panic!` needed. So do **not** hardcode 4
or 9. Derive the class from a stated definition, print the definition alongside the count, and let the
number be whatever it is. ★ Per `CLAUDE.md` *"Derive the list, or make the compiler hold it"* — a
pinned count here is the exact disease this instrument exists to detect.

★ Note `btctax-tui-edit/src/main.rs:24399` (`"EXCL: LotsForm must open…"`) looks like test-support code
that is not marked `#[cfg(test)]`. Decide whether your definition should include it and say why — that
judgment is part of the deliverable, not a detail.

## Required — B1, seen-red-once
Whatever you build lands **paired with a test that plants the exact defect it exists to catch and is
observed RED**, then green. For the abort census that means planting a year-keyed abort in a shipped
crate and asserting the census reports it. ★ Per FR-235, do not write the plant in the checker's own
vocabulary — if your census greps for `panic!`, a plant that adds a `panic!` measures only the grep.
Plant a *different* member of the class you defined.

Answer in your report, in one sentence each: **"which test reds when this is reverted?"** with the
pasted red output.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
    cargo run -q -p xtask -- blockers 2026     # the command must still work; show the new section
★ **Never background a gate command and end your turn** (FR-175: two agents stalled exactly this way).

## Do not touch
- `crates/btctax-oracle-harness/**` and `.github/workflows/**` — another agent is working there NOW
- `design/agent-reports/TY2026-BLOCKERS-PREDICTION-2026-09-13.txt` (frozen stage-2 baseline)
- anything in `design/forms/extract/` (archived IRS text), or any existing `design/agent-reports/REPORT-*`

## Final action — persist your report
Write `design/agent-reports/REPORT-fr227-panic-class.md` as your LAST action: the Q1 verdict with its
pasted reproduction (or failure to reproduce), your stated class definition and the count it yields,
the B1 answers with pasted red output, the gate output, and anything this brief got wrong. Return only
a short summary plus that path.
