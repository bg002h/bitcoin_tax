# Plan — the B3 whole-branch review of the interview arc

**Status: PLANNED, NOT RUN.** Written 2026-09-09 at the owner's direction, so the round fires from a
considered brief rather than an improvised one. Firing it is the owner's call.

Range: **`121c8805..HEAD`** — 38 commits, 133 files, ~29,000 insertions at time of writing.

---

## 1. Why this is not optional, and why it cannot be cheap

`design/HARNESS.md` **B3** states it: *"The LAST review before an irreversible action is scoped to
`main..HEAD` and pointed at INTERACTION, not correctness-per-commit. A stack of range-scoped reviews
does not add up to a branch review."* The precedent is exact — three range-scoped reviews each returned
0C/0I and the first whole-branch pass found an **Important in the earliest commit**, outside every
earlier window.

This arc has now produced its own, stronger evidence:

- **Twelve tasks each reviewed to 0C/0I**, and a Critical shipped anyway — FR-102, where a filer with an
  HSA could not print a single page. It was found not by a reviewer but by a **filer walking the
  product**.
- **Seven cross-task defects** (`CLAUDE.md`, *"Derive the list…"*): each was correct when written and
  became wrong when a *later* task widened a set beneath it. No per-task review can see that by
  construction — the defect does not exist yet when the task is reviewed.
- **Two controller briefs were refuted by measurement** (FR-105's mechanism; FR-108's mechanism). Both
  times the implementer stopped rather than building on a wrong premise. A review that inherits those
  premises uncritically would launder them.

**So the tier is opus, not sonnet.** `CLAUDE.md`'s tiering puts *"anything needing judgment across a
whole system"* in the opus row; sonnet's row is search, fact-checking and false-PASS hunting. A sonnet
pass over this range was run separately (§4) and is **not** a substitute.

## 2. What the earlier passes already covered — do not re-spend budget here

B3's own rule: *"Tell the final pass what the earlier rounds covered, so it spends its budget on the
seams."*

| already done | evidence |
|---|---|
| Per-task correctness, T1–T12 + T16 | 14 seam reviews + 14 sonnet re-verifications, all closed 0C/0I |
| Every kill replanted after each fold | each re-verification report |
| Two end-to-end filer journeys | `2026-09-07-journey-walk-*`, and walk 2 (single filer, no dependents, 8949-heavy) |
| A cross-task pattern sweep | `2026-09-09-cross-task-pattern-sweep.md` (sonnet) |
| Suite, instruments, docs | `make gate` 3558 green; five instruments stable; `make docs` no diff |

**Do not re-audit any of that.** A finding that a per-task review could have caught is out of scope by
definition — if it were catchable there, it would have been caught there.

## 3. The one question B3 must answer

> **Do the twelve tasks compose into something a person can actually file with — and where do two
> correct pieces meet and produce a wrong whole?**

Four seams to point it at, chosen because each spans tasks and no single review held both sides:

1. **The chain from answer to printed page.** A filer's keystroke → `answer_log` → `ReturnInputs` →
   the screens → `Form1040Lines` → the emitter → the read-back verifier. Every hop was reviewed; the
   *chain* was not. FR-102 lived on the last hop; the T11 root fix found four money defects on an
   earlier one.
2. **Multiple writers of one structure.** `income import`, `income answer`, the TUI seam, and
   `open_next_year::seed` all write `ReturnInputs`. FR-97 was one such disagreement and is fixed;
   FR-109 raised a second (import provenance) and is closed by the ask-once rule. **Are there others?**
   Enumerate every writer and ask what each records, what it skips, and whether the readers agree.
3. **Refusals as a system, not as rules.** ~126 `RefuseReason` variants across twelve tasks. Can a
   filer reach a state where two refusals contradict, or where the exit named by one is blocked by
   another? Is any refusal unreachable (FR-87's shape), and is any reachable state unrefused?
4. **The year boundary.** TY2024 files, TY2025/26 are `preparing`. Every task added year-conditional
   behaviour. The `stamp_year` defect showed a year-scoped screen that was blind on the one command
   that creates a row — **that is one instance of a class**, and the class was never swept.

## 4. Shape, tier and budget

- **One opus reviewer**, `isolation: worktree`, at the range tip. Not a fan-out: `CLAUDE.md` is explicit
  that *"value comes from disjoint lens briefs and an adversarial refute pass, not agent count"*, and
  four seams do not need four agents.
- **A second opus pass only if the first returns Critical findings** — to refute them, not to add
  breadth.
- **Fable is not indicated** unless a first irreversible action (a release) is imminent; that is its
  documented trigger and no release gate is open.
- Report persisted by the agent to `design/agent-reports/2026-09-XX-b3-whole-branch-review.md`, then the
  standard loop: persist verbatim → controller ledger machine-checking every claim → fold brief → fold →
  gate → push → re-verify.

## 5. Preconditions before firing

1. **Walk 2's findings folded**, or explicitly deferred with owning phases — otherwise B3 reviews a tree
   that is about to change.
2. **The pattern sweep's findings likewise.**
3. `make gate` green and everything pushed, so the range tip is stable.
4. The reviewer is told, in the brief, that **two controller briefs in this arc were refuted by
   measurement**, and is instructed to stop and report rather than build on a stated premise it can
   disprove. That instruction has paid twice.

## 6. What would make this round a failure

Per the harness: *"the measure is not that a final pass happened, but that it was scoped to something
the earlier passes could not have seen."* A report full of per-commit correctness findings means the
brief failed, not that the branch is clean. If it returns 0C/0I **and** its "seams checked clean"
section shows it actually traversed the four chains above, that is the result worth having.

---

# ADDENDUM — 2026-09-11, written immediately before firing

The plan above stands unchanged. This section updates it for the eight commits that landed after it was
written, and is the part a dispatching coordinator must fold into the brief.

## A1. The range moved — and the new commits are the LEAST reviewed in it

Range is still `121c8805..HEAD`, but `HEAD` is now `3ae930ab`, eight commits later:

```
9961519f decision(owner): FR-110 ceiling; FR-114 label-only — FR-114's premise refuted
1e6095d7 brief(FR-114)
5a1a94be persist(FR-111 recon)      52b348c2 fold(FR-111)
b9b3810f persist(FR-114 report)     52558813 fold(FR-114)
fd6adda8 persist(re-verification)   3ae930ab ledger(FR-117)
```

★ **Point the reviewer at these first.** Everything before `4fa1e723` has had a seam review AND a sonnet
re-verification per task. These eight have had **one** sonnet re-verification (`fd6adda8`, 0C/0I, scoped
to the two folds) and no independent design review at all. They are the thinnest ice in the range.

Two of them change surfaces that span the whole workspace:

- **`52558813` added a field to a public struct** — `Field.label_source`, no `Default`, no `_` — touched
  by every `Field` literal in the workspace. The re-verification confirmed every site compiles and
  spot-checked 15+ classifications. It did **not** ask the seam question: does anything else read
  `Field` in a way this widens or breaks (the TUI, the renderers, the census join)?
- **`52b348c2` edited `LIMITATIONS.md`, which is `include_str!`'d into the binary** (`main.rs:582`). It is
  **shipped filer-facing text**, not a design doc. Treat it as product surface.

## A2. Added to §2 — also already covered, do not re-spend budget

| already done | evidence |
|---|---|
| FR-110 scope decision (aggregate 1099-B is the permanent ceiling) | owner ruling, `9961519f` — **settled, not a gap** |
| The Form 8949 box truth table, re-derived from source | `RECON-fr111-8949-box-truth.md` + controller verification (`5a1a94be`) |
| Every filer-facing 1099-B / 1099-DA / box claim swept | same recon, Q3 — four findings, all closed or filed |
| The R15 label extension, kill-tested and independently planted | `REPORT-build-fr114-label-scan.md` (`b9b3810f`) |
| Both folds re-verified | `REVERIFY-fr111-fr114.md` (`fd6adda8`) — 0C/0I, 1 Minor (FR-117) |

**FR-110 is a DECISION, not a defect.** A finding that btctax cannot file a 1099-B with adjustments is
out of scope: that is the intended permanent ceiling and the owner ruled on it. Say so in the brief.

## A3. §5's preconditions are now ALL MET

1. ✅ Walk 2's findings folded or filed with owning phases (FR-110/111 closed; FR-112/113 filed).
2. ✅ The pattern sweep's findings likewise (FR-114 closed; FR-115/116/117 filed).
3. ✅ `make gate` **3561 passed / 12 skipped**, `cargo fmt --all --check` clean, everything pushed,
   `HEAD == origin/main`, tree clean, no worktrees.
4. ⚠️ **See A4 — this one got stronger and must be restated in the brief.**

## A4. ★★★ §5.4 was understated: it is now FOUR refuted briefs, not two

The plan says *"two controller briefs in this arc were refuted by measurement."* As of 2026-09-11 it is
**four**, and **two of the four were written by the coordinator itself**:

| premise | author | refuted by |
|---|---|---|
| widening R15 would red on the IRS's `"Covered lots"` vocabulary | FR-114's author | whole-word matching — `"lots" != "lot"` |
| entering a 1099-B reopens the Form 8949 box choice | FR-111's author | `form_8949()` reads only `state.disposals`; a `[[b_1099]]` row never becomes an 8949 row |
| the export's I5 advisory may misdescribe a routed return | **the coordinator** | it is `regime`-gated; the two branches are mutually exclusive |
| *"zero labels red today"* | **the coordinator** | 209 of 279 labels are macro-generated and were invisible to the scan that produced it |

**The generalisation, and the single most load-bearing sentence in this brief:**

> A hand-written scan over one syntactic form is not a measurement of a set produced by another.

Every one of the four was written by someone being careful. What kept all four out of the product was
not care — it was that each implementer **stopped and reported rather than building on a premise it
could disprove**. Instruct the B3 reviewer to do exactly that, about anything in its own brief,
including everything in this addendum. That instruction has now paid four times.

## A5. One seam the original §3 does not name

The four seams stand. Consider a fifth if budget allows, and only as a *seam* question:

5. **Instruments vs. the surfaces they claim to cover.** This arc shipped five instruments plus the R15
   extension. FR-114 proved an instrument can be green because it never ran over the region that
   mattered, and FR-117 shows the exemption's unit (a field) differing from the thing exempted (a span
   of text). **For each instrument, what does it actually traverse, and what does it claim?** That is a
   cross-task question no per-task review could hold, which is precisely B3's remit.
