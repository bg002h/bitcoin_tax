# Schedule 1-A IMPLEMENTATION_PLAN — review brief (r4)

**Artifact:** `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md`, **current text (Status: r3)**.
**Implements:** `design/ty2025/SPEC_schedule_1a.md` (r3, 0C/0I), branch **B3** of `design/ty2025/SPEC.md` §8a.

## Why this review exists

The plan says **Status: r3**, but `design/ty2025/reviews/` holds exactly one independent review of the
PLAN — `PLAN_schedule_1a-opus-r1.md`. The r1→r2 and r2→r3 folds were never re-reviewed; r2→r3 folded a
**13-agent provenance census**, which is a census, not a review, and it *grew* a Critical.

This repo's own rule is **re-review after every fold, including the last**, and no work proceeds past a
gate while a blocking finding is open. **This is that missing review.** T1 is built; T2–T7 are not.

★ And the stakes are named by the code itself. `tax_tables.rs:789-812` warns that a partial TY2025
landing *"is not a smaller version of TY2025 support. It is a silently wrong return."* The owner's plan
is to **diff btctax's TY2025 output against their own real, prepared TY2025 return** — so a plausible-
wrong number here is worse than a refusal: it either falsely validates or sends us chasing phantoms.

## THE ONE QUESTION

**If a competent implementer executed this plan exactly as written, would the result be a correct
Schedule 1-A — every line present, every figure right, and every unanswerable input refusing rather
than defaulting?**

Not "is the plan well written". Not "what could be clearer". **Would executing it produce a correct
return.**

## Where the risk is concentrated

1. **The eligibility class.** r1's two Criticals were both **missing eligibility**, not wrong
   arithmetic — *"the defect class this project keeps rediscovering"* (the plan's own words). Look for
   it again: a line computed for a filer who does not qualify, a phase-out not applied, a status
   carve-out missed.
2. **★★ The `_`-on-money hole, and whether the plan closes it.** `design/ty2025/SPEC.md` §8a records
   that `classifier.rs` **permits** `_` on money leaves by its own stated rule, so *"`None` refuses"*
   for **~25 `Option<Usd>` fields would be held by CONVENTION** — the exact class D-5 exists to
   prevent. **T3a** claims to address two lines with no input path. Does the plan actually make the
   other ~23 structural, or does it rely on the convention?
   ★ Relevant precedent landed since: `crates/btctax-core/src/tax/line_coverage.rs` forbids `_` on
   money for **printed** structs and is enforced by `xtask line-coverage`. The same mechanism on
   `ReturnInputs` money leaves is available. Say whether the plan should adopt it, and whether T3/T3a
   are sufficient without it.
3. **T2's conformance KAT.** The plan delegates the "is every line present" gate to a test rather than
   restating 48 lines. The provenance census found *"a hole in T2's own conformance approach"*. Is the
   KAT as specified actually capable of failing on a missing line — and on a **wrong doc comment**?
   Enumerate the line set FROM the extracted text, never a range or hand-list.
4. **The rounding asymmetry** (S-1 in the child spec; `StepRounding`, lines 11/19/28 — *"the field that
   must never be shared"*). T1 built it. Does the plan's use of it stay correct per line?
5. **Sequencing.** T1 is built. Are T2–T7 genuinely independently landable, and does each leave the
   tree green? ★ The plan explicitly does NOT delete the fail-closed gate (that is B4) — verify nothing
   in T2–T7 opens TY2025 early.

## Settled — do NOT re-derive

The parent SPEC and the child SPEC are both green (0C/0I) and are **not** under review. D-1…D-11 bind.
The four-branch cut (B1–B4) is decided; B3 satisfies gate **condition 4 only**. `f1040s1a--2025.pdf`
and `i1040gi--2025.pdf` pp. 101-110 are the archived primary sources. r1's findings are folded — read
`reviews/PLAN_schedule_1a-opus-r1.md` so you do not re-file them.

## Output

`VERDICT: green` or `VERDICT: <n> Critical / <n> Important`

```
SEVERITY: Critical | Important | Minor | Nit
WHERE: PLAN §N (or a file:line for a code claim)
CLAIM: one sentence.
CONSEQUENCE: what the executed plan produces that is wrong, or fails to catch.
EVIDENCE: quote the plan AND the form text / code that contradicts it.
```

**Critical** = executing this yields a wrong figure or a missing line on a filed return, or an
unanswerable input that defaults instead of refusing. **Important** = a real gap that surfaces during
the build. Green is a fine outcome — the artifact has had a spec review, a plan review and a 13-agent
census already.

End with `ALSO CHECKED, SOUND:` and `WHAT WOULD MAKE THIS REVIEW WRONG:`.

**Constraints:** READ-ONLY. No edits, no commits, no subagents. Quote the extracted form text — it is
in `design/amt-form6251/` — rather than reasoning from the plan's description of it.
