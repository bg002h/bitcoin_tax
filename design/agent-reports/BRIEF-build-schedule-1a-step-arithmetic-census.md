# BRIEF — extend the Schedule 1-A two-oracle census from PARAMETERS to APPLIED RESULTS

**Owner ask 2026-09-11:** *"Extend the corpus over the OBBBA axes."* Recon then narrowed it, and the brief
says why up front so you do not rebuild what exists.

You are ONE opus agent in the **shared main tree**. Nothing is committed while you work.

---

## 0. What already exists — do NOT rebuild it

`scripts/oracle/verify_schedule_1a.py` is already a **per-part two-oracle census for TY2025 Schedule 1-A**,
covering all four provisions, and it passes today with **0 divergences**:

| part | provision | verified |
|---|---|---|
| II | tips §224 | cap, phase-out start (base + MFJ), step size, per-step rate |
| III | overtime §225 | same |
| IV | car loan interest §163(h)(4) | same, **and it already caught a real taxcalc defect** — taxcalc gives QSS the MFJ $200,000 threshold when the form says *"Married filing jointly—200,000. All other filing statuses—100,000"*, so taxcalc is DISQUALIFIED as a QSS witness on that part, computed from the parameter itself |
| V | seniors §151(d)(5) | cap, phase-out start, 6% rate |

So the OBBBA **constants** are covered, per part, with mechanism-derived disqualifications. That is the
instrument the owner's request pointed at, and it is done.

## 1. ★★★ The gap — the census checks the CONSTANTS, never the APPLIED RESULT

The parameters agree. **Nothing compares a computed deduction.** And the arithmetic that turns those
parameters into a deduction carries a field the source itself flags as the dangerous one:

`crates/btctax-core/src/tax/tables.rs:1059-1084` — `StepRounding::{Floor, Ceil}`, on
`StairStepPhaseOut`, with the doc comment: *"Lines 11 / 19 / 28 … **The field that must never be
shared.**"* The two variants quote the form's own opposite instructions:

- `Floor` — *"decrease the result to the next lower whole number. (For example, decrease 1.5 to 1, and
  decrease 0.05 to 0.)"*
- `Ceil` — *"increase the result to the next higher whole number. (For example, increase 1.5 to 2, and
  increase 0.05 to 1.)"*

**A wrong direction is a full step.** At `per_step` $100 (tips, overtime) or $200 (car loan), the deduction
is off by $100–$200 for **every** filer in the phase-out band — and every constant still agrees, so the
existing census stays green. That is the defect this task exists to make impossible.

★ And `StairStepPhaseOut`'s own comment records the adjacent trap: **Part V is deliberately NOT stepped**
(§151(d)(5)(C) is a flat 6%) because *"giving it a fake step is how a smooth phase-out acquires a stair."*

## 2. Why this is TY2026 work, not TY2025 work

The owner ruled 2026-09-11 that TY2025 is never filed and that TY2025 work must **transfer**.
`SCHEDULE_1A_YEARS` is `2025..=2028` (`tables.rs:1215`), so Schedule 1-A computes for **TY2026 today**,
independently of `FullReturnParams` — the step arithmetic you are about to pin is the arithmetic TY2026
will file with. State that in the artefact.

## 3. Deliverables

1. **Step-boundary vectors for the three stepped parts** (tips lines 10-12, overtime 18-20, car-loan
   27-29). Per part, at MAGIs chosen to straddle the arithmetic's edges — **exactly at the threshold, one
   dollar over, 0.05 of a step over (the form's own example), 1.5 steps over (its other example), and at
   the MAGI where the deduction is fully phased out** — compare the computed deduction against taxcalc.
   Derive the vector set from the parameters, do not hand-type a list of MAGIs.
2. **Part V checked as SMOOTH**, with a vector that reds if a fake step is introduced.
3. **A Rust-side KAT** pinning the same boundary vectors, mirroring the existing
   `schedule_1a_table_matches_the_oracle_census`, so a drift on either side reds.
4. **Settle the SALT question, which is the other OBBBA axis the owner named.** `scripts/oracle/corpus.py`
   carries `SALT_CAP_BY_YEAR = {2024: 10_000, 2025: 40_000}` and asserts the TY2025 axis straddles the
   $40,000 cap — but the sweep drives btctax's **full return**, and `full_return_for(2025)` is a deliberate
   `None`. **Measure whether that TY2025 axis can ever execute.** If it cannot, it is an axis that never
   runs and you must say so plainly and state the boundary in source (or wire it to something that does
   run). ★ Do **NOT** bundle `FullReturnParams` to make it run — that gate is the owner's.

## 4. The rules this census already obeys — keep them

- **Compute the expected value INDEPENDENTLY in Python, from the statute's own words.** The existing file
  says why, and it is the whole design: *"literals ON PURPOSE: an oracle check that imported our value
  would be comparing our input to itself — an echo, not a witness."* Never import btctax's number.
- **Disqualifications are COMPUTED from the defect's mechanism, never keyed by vector name.** Both
  name-keyed excuse lists this project ever had went stale on the first new vectors.
- **Count independent witnesses per vector.** Two disqualified oracles can agree — that is on record for
  MFS AMT, where three vectors had no witness at all and the run printed "OK" twice.
- Check whether **OTS** models these provisions for TY2025 at all. If it does not, that is a computed
  disqualification and the census is single-witness there — express it, do not hide it.
- `.venv/bin/python` (taxcalc 6.8.2, version floor enforced). Bare `python3` has neither taxcalc nor pandas.

## 5. ★★ The kill — B1, and it is the point of the task

**Flip one stepped part's `StepRounding` Floor↔Ceil and show your new check reds**, with the output pasted.
Then restore. Do it for each of the three stepped parts, and for Part V plant a fake step.

A check that stays green under a flipped rounding direction is worthless here, because that is the exact
defect the field's own doc comment warns about. Paste red-then-green for every one.

## 6. Severity and scope

Do not widen scope: this census, its KAT, and the SALT question. Anything else goes to `FOLLOWUPS.md` with
an owning phase. Do not bundle params, do not touch the fail-closed gates, do not transcribe from a draft.
A **blank** is the normal case; assert provenance, never non-blankness.

## 7. Mechanics

**Main tree. Do NOT commit, push, `git stash`, `git checkout` or revert anything** — a builder in this arc
ran `git stash` against its brief. Revert a mutation with a **cp backup**. **No subagents.** No
`--no-verify`. Never hand-count what a tool can count, and **never quote a number from a `head`/`tail`
view** — the coordinator made that error four times today; read the tool's complete output.

Baseline: `make gate` **3596 passed / 12 skipped**, fmt clean. Report numbers as numbers.

## 8. Your report — final action

Write `design/agent-reports/REPORT-build-schedule-1a-step-arithmetic-census.md`: what you built and where;
the vector set and how it was DERIVED; per-part witness counts and any computed disqualification; the kills
with pasted red-then-green for every rounding flip; **the SALT answer with its measurement**; refuted
premises; residue with owning phases; the literal gate numbers. Return only a short summary plus the path.
