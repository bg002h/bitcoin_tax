# BRIEF — FR-180: eleven refusals that nothing tests, seven of them answered-ness guards

FR-180's question, verbatim from the ledger: *"the census verifies a refusal is RECORDED, never that it
FIRES … can a filer actually reach each refusal, and does each one fire on the state it names?"* Work in
your own worktree. Do **not** spawn subagents. Do **not** commit.

## Facts already MEASURED by the controller — do not re-derive

`RefuseReason` declares **139** variants. Scanning every `.rs` under `crates/` and separating test
context from production (the `#[cfg(test)]` region logic that `xtask`'s
`r15_stop_list::production_mask` implements, plus whole files under a `tests/` directory):

| | count |
|---|---|
| variants named in a TEST context | **128** |
| variants named in **no** test context at all | **11** |

★★ **And here is why a grep census cannot answer FR-180, which you should not spend time rediscovering.**
The same question measured at three scopes gives three answers: over *all* `.rs` → 139 of 139 named
(useless, because `return_refuse.rs` names them itself); over `tests/` directories only → **23**; over
test context properly separated → **128**. A number that swings 23 → 128 → 139 with the scope is not a
measurement. Reachability is not grep-able, and that is the finding FR-180 rests on.

## The eleven

    CooperativePatronUnanswered            HsaExcessEmployerContributions
    FamilyLeaveBenefits                    InterestOrDividendsWithout1099Unanswered
    FilerTinUnanswered                     QbiCarryforwardNeedsSchedule8995AC
    FilingStatusUnconfirmed                SstbInPhaseInRange
    HomeSaleGateUnanswered                 StateRefundWithout1099gUnanswered
                                           WagesWithoutW2Unanswered

★★★ **Seven are `*Unanswered` / `*Unconfirmed` — answered-ness guards, and that is the sharpest part of
this task.** `CLAUDE.md` holds that answered-ness must be **structural**, and the repo's own recorded
failure is *"shipping a CORRECT fix with no test holding it"*. A guard with no test is a guard nobody has
watched refuse. Per `CLAUDE.md`: *"A guarantee without a test that reds when it is removed does not
exist."*

## What to do

**For each of the eleven, write a test that drives a return to the state the variant names and asserts
that this refusal fires.** Then answer, per variant, one of three verdicts — and the second and third are
findings, not failures:

1. **FIRES** — the test exists now and reds if the guard is removed. The ordinary outcome.
2. **UNREACHABLE** — no input can reach it. That is a real defect of its own shape (the ledger cites
   FR-87 as exactly this: an unreachable refusal). Say what blocks it and do **not** delete the variant
   to make the census green.
3. **FIRES ON THE WRONG STATE** — it refuses, but not on the state its name and message claim. That is
   the worst of the three, because the message misleads a filer about what to fix. Two such were found by
   hand recently (a refusal naming two cures that both dead-ended; an export refusal printing
   `btctax set-pii`, a verb that has never existed).

★ Do **not** weaken a refusal to make it testable. *"Widening an exemption is never the safe edit."* If a
guard is hard to reach because the input surface cannot express the state, that is verdict 2 with a
reason.

## The instrument, and it is the durable half

Land a census that asserts **every declared `RefuseReason` variant is named in a test context**, with the
residue pinned and a reason per row — the shape `xtask ledger_check::EXPECTED_CITED_AND_OPEN` uses. It
must red when the residue **grows** (a new refusal arrives untested) and when it **shrinks** without the
pin being updated.

★★ **Derive the variant list from the enum, never type it.** `CLAUDE.md`'s highest-yield rule; and
`blockers::refusal_census` already enumerates every variant and raise site — read it rather than
reimplementing, and say in your report which you reused. ★ State the census's blind spot in its output:
*named in a test* is not *observed firing*, and a test that merely mentions the variant in a comment
satisfies a grep. If you can make the check stronger than naming — e.g. requiring the variant inside an
assertion — do, and say what it now proves.

## Required — B1, seen-red-once
The census lands paired with a planted defect: add a new `RefuseReason` variant with no test and watch the
census red; remove a pinned row while its variant is still untested and watch it red the other way.
★ Per FR-235 do not plant in the checker's own vocabulary — if the census greps for a variant name, do not
plant by adding a name to a comment.

Answer in your report, one sentence each: **"which test reds when this is reverted?"**, with pasted red
output, and the eleven verdicts in a table.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
★ **Never background a gate command and end your turn** (FR-175).
★★ If you add a `RefuseReason` variant, `xtask blockers`'s pinned census count and `xtask ledger-check`
will red — those are the controller's to move. **Report them; do not edit `crates/xtask/**` except to add
your own census module**, and say plainly which xtask files you touched.

## Persisting your report
`Write` is REFUSED for report files. Use a Bash heredoc:

    cat > design/agent-reports/REPORT-fr180-refusal-reachability.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and give the byte count in your summary. If refused, return it inline and say at the top
it is the only copy — never silently trim.

## Your tree
Your worktree branches from an OLDER base than main (~40 commits behind), so recent work may be absent
(`tax/form8283_section_a.rs`, `tax/pub936_table1.rs`, `xtask/src/ledger_check.rs`,
`xtask/src/r15_stop_list.rs`'s `production_mask`). Do not rebase. If a file named here is missing, read
it from `/scratch/code/bitcoin_tax/<path>`.

Do not touch: `crates/btctax-oracle-harness/**`, `design/forms/extract/**` and `legal/**` (archived
authority — read, never edit), `crates/btctax-core/src/tax/packet.rs` or the golden packets (another
agent is working there NOW), any existing `design/agent-reports/REPORT-*`, or `FOLLOWUPS.md`.
