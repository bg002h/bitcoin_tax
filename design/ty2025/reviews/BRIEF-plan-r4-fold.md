# BRIEF — review the r4 FOLD of the Schedule 1-A plan. One round, then we build.

One agent, opus. **No subagents. Do not modify any tracked file** — write only
your report.

## THE ONE QUESTION

`design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` took an r4 fold on
2026-08-01 answering 0C/9I from two independent lenses. **That fold has never
been reviewed.** On this repo's own rule — re-review after every fold, including
the last — the plan is therefore not green, and the immediately preceding
history is a warning: the r2→r3 "fold" was a census rather than a review, and it
*grew* a Critical.

**Is the r4 fold sound enough to build T2 against?** Answer that, not "is the
whole plan perfect." A verdict of *0C/0I — build it* is the expected outcome if
the fold holds.

## WHY THIS ROUND IS WORTH ONE AGENT AND NOT FIVE

T2 is a declared **chokepoint**. The fold's own words: *"B4 cannot fix it if T2
makes every line non-optional."* The fold changed the plan's central sequencing
claim — from ~25 loose `Option<Usd>` inputs to **one claim gate per part** — and
added completion predicates and a refinance balance cap. Those are structural.
Getting the struct shape wrong at T2 is expensive; getting prose wrong is not.

So: **weight your attention on anything that decides a TYPE or a GATE.** Prose
inconsistencies are Nits here, explicitly.

## ATTACK THESE, IN ORDER

1. **C-I2's completion predicates.** The fold says each part's Caution becomes a
   completion predicate, and affected leaves must be able to express *not
   completed* at T2. Check that against the **form's own text**
   (`design/forms/extract/f1040s1a--2025.txt`, 113 lines — read it). Does the
   resolution actually cover the two cases named (a car-loan-only filer
   computing 15 phase-out lines and printing $6,000 on line 35 for a non-senior;
   a filer with no §911 exclusion printing $0 on 2a–2e)? Is any OTHER part's
   Caution missed?
2. **The one-claim-gate-per-part decision.** The fold cites
   `crates/btctax-core/src/tax/return_inputs.rs:626-652` as the existing
   class-(A) pattern. **Read that code.** Does it actually support the claim
   made about it? Is one gate per part sufficient, or does some part contain two
   independently-claimable things?
3. **C-I3's refinance balance cap** — the fold says the r1 fold LOST it and the
   prose stated the rule BACKWARDS. Verify the restored wording against the
   form/instruction text. This one **understates tax** when wrong, which is the
   direction this project treats as worst.
4. **C-I1's expected-set source.** The fold drives T2's KAT from
   `xtask/src/label_reader.rs`. Machine-checked already, do not redo:
   `cargo run -p xtask -- label-census f1040s1a--2025` prints
   *"48 entry line(s), 2 without a box — each of the latter needs a recorded
   reason."* Confirm the plan uses that adjudication rather than a hand-list,
   and that the KAT's B1 half (a defect that makes it red) is specified.
5. **Anything the fold CLAIMS to have fixed that it did not.** Three of the nine
   are marked ✅ FIXED IN CODE. Verify each against the code, not the claim.

## OUT OF SCOPE

- Do not re-review the SPEC (`SPEC_schedule_1a.md` r3, 0C/0I).
- Do not re-litigate r1's or r4's original findings — only the FOLD's answers.
- Do not propose new scope, new forms, or new checkers.
- Do not hand-count what a command counts.

## CONTEXT

Prior reviews, verbatim, in `design/ty2025/reviews/`:
`PLAN_schedule_1a-opus-r1.md`, `PLAN_schedule_1a-conformance-r4.md`,
`PLAN_schedule_1a-buildability-r4.md`, `PROVENANCE_CENSUS_schedule_1a.md`.

Doctrine: `CLAUDE.md` and `STANDARD_WORKFLOW.md` at the repo root. Note
especially *"Tests for conformance, reviews for judgment"* — if a finding of
yours is really a test that should exist, say so in those terms.

★ Why this matters beyond the branch: TY2025 is the year being filed now, and
Schedule 1-A is its hard prerequisite. Nothing else in the TY2025 form set can
land until this does.

## SEVERITY

Critical = wrong result / data loss / unmet guarantee. Important = real defect,
missing case, unsound assumption — **including a type or gate decision that T2
would lock in wrongly.** Minor/Nit recorded, not blocking.

## OUTPUT

Write your report **as your final action** to exactly:

    design/ty2025/reviews/PLAN_schedule_1a-r4fold-review.md

Structure: **VERDICT** (nC/nI/nM/nNit, and a plain **BUILD T2** /
**DO NOT BUILD YET**) · **FINDINGS** (severity, what the fold says, what is
actually true, smallest fix) · **WHAT I VERIFIED AND HOW** · **WHAT I COULD NOT
CHECK**.

Return to the controller **only** a ≤ 8-line summary plus that path.
