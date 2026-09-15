# BRIEF — FR-244: btctax can omit a Form 8283 the instructions require

A live wrong-output defect on a filed return, **pre-existing** (it governs crypto donations today, not
just FR-200b's Section A). Work in your own worktree. Do **not** spawn subagents. Do **not** commit.

## The defect, and it is a MEASURE not a trigger

`crates/btctax-core/src/tax/packet.rs:1232` decides whether to attach Form 8283 with

    .filter(|a| a.line12 > FORM_8283_THRESHOLD)

`a.line12` is the **post-ceiling** printed Schedule A line 12 — confirmed by btctax's own doc comment
above `Printed8283Rows`: *"the §170(b) ceilings legitimately make L12 smaller than the sum of the 8283's
per-donation amounts (the excess becomes carryover)."*

So when a §170(b) percentage ceiling binds — gross noncash over $500, printed line 12 at or under $500 —
**the instructions require a Form 8283 and btctax attaches none.** An omitted required attachment is a
§170(f)(11) denial risk: exactly the risk `RefuseReason::NonCryptoNoncashGift` was created to avoid,
reappearing one layer down.

## The authority — settled by the controller, do not re-derive

Three passages, all archived, and **all three say the measure is PRE-ceiling**:

| source | verbatim |
|---|---|
| `design/forms/extract/i8283--2024.txt:51-52` | *"For this purpose, “amount of your deduction” means your deduction **before applying any income limits** that could…"* |
| `design/forms/extract/i1040sca--2024.txt:1255-1259` | *"Deduction more than $500. If the amount of your deduction is more than $500, you must complete and attach Form 8283. For this purpose, the “amount of your deduction” means your deduction **before applying any income limits** that could result in a carryover of contributions."* |
| `design/forms/extract/f8283--2024.txt:11` | the form's own header — *"Attach one or more Forms 8283 to your tax return if you claimed a total deduction of over $500 for all contributed property."* |

★★ **The tension, and how I adjudicated it — do not re-open this.** Schedule A **line 12's own printed
text** says *"You must attach Form 8283 if over $500"*, about line 12, which is post-ceiling. Read
literally that points the other way. The explicit definitions win: both instruction booklets say *"For
this purpose … means your deduction before applying any income limits"* — a definition that exists
precisely to override the terse on-form pointer. ★ And the direction confirms it: pre-ceiling ≥
post-ceiling, so the pre-ceiling measure attaches the form **more** often. A form attached when it was
not strictly required is not a denial risk; a missing one is.

★ **The TRIGGER is a total and stays a total.** The form's own header says *"a total deduction of over
$500 for all contributed property"*. i8283 additionally phrases it per contribution and per group of
similar items; the union still fires whenever the total clears $500, so the existing total-based trigger
is right. **Change the measure, not the trigger.**

## What to do

Make the attachment decision read the **pre-ceiling** noncash total. `form8283_section_a` already
distinguishes the two measures — its own doc says *"This is NOT `FORM_8283_THRESHOLD`, which is measured
over the tota…"* — so find the pre-ceiling quantity rather than inventing one, and say in your report
which value you used and where it comes from.

★★ **The reason this was not fixed the day it was found, and the care it needs:** the fix moves a
**presence test that byte-goldens depend on**. Some golden packets will gain a Form 8283. That is the
correct new output, but read `CLAUDE.md`'s *"a golden cannot validate its own regeneration"* before you
touch one: if the remedy for a red golden is "regenerate", then regenerating to match a **breakage** is
always green. So for every golden whose page set changes, state in your report **why** the new page set
is right — name the household, its gross noncash, its printed line 12, and which authority makes the
attachment required.

★ Do not "fix" it in the other direction: making line 12 pre-ceiling would be wrong (the ceiling
genuinely limits the deduction and the excess genuinely carries over). Only the **attachment decision's
input** changes.

## Required — B1, seen-red-once
The kill is a household where the two measures **disagree**: gross noncash over $500, printed line 12 at
or under $500. Build it, assert the packet now contains Form 8283, and show the test RED with the
pre-fix filter. ★ Per FR-235 do not plant in the checker's vocabulary — vary the household's *ceiling
position*, not the predicate. ★★ And assert the near-miss too: a household whose gross noncash is at or
under $500 must still attach nothing, or the fix is "always attach" wearing a measure's clothes.

Answer in your report, one sentence each: **"which test reds when this is reverted?"**, with pasted red
output.

## Gate — SERIALLY, foreground, paste real output
    cargo nextest run --workspace          # FR-223: nextest + clippy CONCURRENTLY get SIGKILLed
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
★ **Never background a gate command and end your turn** (FR-175). ★ A new `RefuseReason` variant reds
`xtask blockers`'s pinned census count and `xtask ledger-check` watches FOLLOWUPS/source agreement —
both are the controller's to move; report them rather than editing `crates/xtask/**`.

## Persisting your report
`Write` is REFUSED for report files. Use a Bash heredoc:

    cat > design/agent-reports/REPORT-fr244-8283-threshold.md <<'RPTEOF'
    ... full report ...
    RPTEOF

then `wc -c` it and give the byte count in your summary. If refused, return it inline and say at the top
it is the only copy — never silently trim.

## Your tree
Your worktree branches from an OLDER base than main — possibly ~30 commits behind — so recent work may
be absent (`tax/form8283_section_a.rs`, `tax/pub936_table1.rs`, `xtask/src/ledger_check.rs`). Do not
rebase. If a file named here is missing, read it from `/scratch/code/bitcoin_tax/<path>`.

Do not touch: `crates/xtask/**`, `crates/btctax-oracle-harness/**`, `design/forms/extract/**` and
`legal/**` (archived authority — read, never edit), any existing `design/agent-reports/REPORT-*`, or
`FOLLOWUPS.md` (put recommendations in your report).
