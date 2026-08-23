# BRIEF — Fable review of Phase 1, scoped to the WHOLE branch and pointed at INTERACTION

## The ONE question

Three implementers worked in **parallel git worktrees off one base** (`3fc88497`) and never saw each
other's code. Each shipped a passing kill-test per item. The suite is green at **2703 tests**.

> **What is wrong in the SEAMS between them?**

Not "is each item correct" — each lane verified that, inline, with mutations observed red. The
question is what is only visible when you hold **all three diffs at once**, which is a view no lane
and no per-item review has ever had.

## ★★★ Why this brief exists: the seam defect already found, as your calibration

`36707a23`. Lane B (btctax-cli) shipped a Form 8949 overflow **preflight that refuses**. Lane A
(btctax-forms) made that same overflow **paginate**. Result: the CLI refused a packet the filler
would have produced — reintroducing the exact total loss (exit 2, zero bytes, every form gone) that
lane A's item existed to end, for the DCA population lane B's own comment named as most exposed.

**Both lanes' kill-tests passed while asserting opposite things about one 16-leg filer.** Green per
lane, broken in the seam. That is the shape you are hunting. **It is already fixed — do not re-report
it.** It is here to tell you what "interaction defect" means in this branch.

## SCOPE

`git diff 3fc88497..HEAD` on branch `feat/filing-readiness`. Read the commit messages too
(`git log 3fc88497..HEAD`) — each carries its mutations and their verbatim RED output, so you need
not re-derive what was already proven.

## ★★ ALREADY MACHINE-CHECKED — do not re-run, do not re-derive

- **2703 tests pass**, `clippy -D warnings` clean, `cargo fmt --all --check` clean. Baseline was 2667.
- **Every item was mutation-verified by its lane**, with the RED output pasted into its commit
  message. Roughly 20 mutations across the three lanes.
- The P2a/P2b seam above is found, fixed, and mutation-verified.

Spend your budget on what tools cannot reach: design, cross-lane interaction, whether a guarantee is
real, and whether any of this is wrong as **tax**.

## WHAT LANDED, by lane

**Lane A — btctax-forms**
- P2b: full-return Form 8949 paginates (⌈rows/grid⌉ copies, per-copy totals, identity on every page)
- P3: Form 8283 Section B column (i) no longer written — it is the pass-through entity's cell;
  `Section8283BRow::deduction` became `Option<MoneyCell>`; 3 sha256 PDF goldens rolled
- P10: `map.rs` >4-dependents comment corrected

**Lane B — btctax-cli**
- `git_pointed_at()` GIT_DIR hermeticity fix in xtask (production half)
- P2a: 8949 overflow preflight — **since deleted**, see above
- P6: §170(d)(1) charitable carryover-out now printed on `report --tax-year` and export stderr
- N4: the packet NAMES the marks btctax deliberately did not make (manifest + stderr pointer)
- P10: Schedule 8812 LIMITATIONS row + kill-test

**Lane C — btctax-core**
- N1: §1212(b)(2)(B) Capital Loss Carryover Worksheet on the carryforward-OUT, all 13 lines
  transcribed, skips modelled as `Option<Usd>`. L4 goes 17,000 → 20,000; L3 stays 17,000.
- P10: `donation.rs` Form 8283 Part III/IV → IV/V labels
- A new xtask conformance checker for the worksheet's line citations

**Controller (me)** — the adjudication doc, the oracle-trap doc, the `scripts/pre-commit` GIT_DIR
containment fix, and the P2a deletion.

## ★ SEAMS I WOULD LOOK AT — offered so you can disagree

These are guesses. If they are wrong, say so in one line and go where the evidence leads.

1. **Form 8283 has TWO lanes in it.** Lane A changed Section B column (i) and made `deduction` an
   `Option`; lane C changed the Part III/IV → IV/V labels in `donation.rs`. Do they agree about what
   the parts ARE, and about which revision is filed? Lane A extracted `f8283--2024.txt` from the
   **Rev. 12-2023** asset. Lane C read the labels from somewhere. Same form, two readings.
2. **Lane A left TY2017 still writing column (i)**, deliberately, because the Rev. 12-2014
   instructions are not in the repo and it refused to invent the rule. Is that the right call, and is
   the resulting inconsistency (2024/2025 blank, 2017 filled) safe on a filed return?
3. **N1 changed a carryforward figure that `report` still prints from the frozen engine.** Lane C
   added a non-gating advisory naming BOTH figures. Is a filer shown two different numbers for one
   thing, and is the advisory's mechanism-based firing condition actually right?
4. **N4's manifest marks vs. the forms the packet now contains.** Lane B enumerated the hand-marks
   from one view of the packet; lanes A and C changed what the packet contains. Does the manifest
   still describe the packet that is actually written?
5. **`Section8283BRow::deduction: Option<MoneyCell>`** — does a `None` here reach any emitter that
   prints a blank where the form wants a figure, or vice versa? Recall this repo's standing rule:
   blank and zero are different testimony, and the emitter historically could not express blank.
6. **The two GIT_DIR fixes** (lane B's `git_pointed_at` in xtask, mine in `scripts/pre-commit`) are
   deliberately independent layers. Is either one wrong, and does either mask the other's failure?

## FORBIDDEN — where a pass like this usually wastes its budget

- **Re-reviewing each item for correctness.** That was done, inline, with mutations. Re-deriving it
  spends exactly the budget that makes this pass worth running.
- **Re-reporting the P2a/P2b seam.** Found and fixed.
- **Re-litigating settled adjudications.** `design/direction/ADJUDICATION-2026-08-21.md` settled
  D3/D4/D5/D7 against primary sources. Those are decisions, not open questions. **Phase 2 has not
  been built yet** — do not review it.
- **Style, prose, naming, doc-comment length.** Several docs are long on purpose.
- **"Add more tests"** without naming the class the test catches and the mutation that kills it.
- **Scope creep into the whole codebase.** This is a branch review, not an audit of btctax.

## KNOWN-OPEN, deliberately — do not report these as findings

- **N3** (1040 line 19 blank-vs-zero) NOT BUILT — needs a btctax-core lane; lane A refused to
  implement a §24(b) predicate inside btctax-forms, which would have been a second divergent
  implementation and a worse answered-ness violation.
- **TY2017 8283 column (i)** still written (see seam 2).
- **TUI has no reader** for the charitable carryover (lane B's P6 covered CLI only).
- **L0 filing-threshold note** not built — needs i1040 Chart A transcribed.
- **`LIMITATIONS.md:231`** still describes only the refusing IN side of the §1211/§1212 edge.
- **The IN-side carryforward refusal** is untouched by design; lane C reports it is now mechanically
  liftable and correctly declined to do it.

Say if you think any of these was the WRONG thing to defer — that is in scope. Re-listing them as
findings is not.

## OUTPUT FORMAT

```
VERDICT: <sound | needs-changes | wrong-shape>

SEAM FINDINGS (the point of this pass):
  For each:
    THE SEAM: <which lanes, which files>
    WHAT EACH SIDE ASSUMED:
    WHAT IS ACTUALLY TRUE:
    THE FAILING CASE: <concrete inputs -> wrong output on a filed return>
    SEVERITY: <C | I | M | N>   — C only for a wrong figure on a filed return, data loss, or a false safety claim

SINGLE-LANE FINDINGS: <only if genuinely missed by that lane's own verification>

IF NOTHING IS WRONG IN THE SEAMS: say so plainly, and name the two seams you attacked hardest and how.

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence naming the assumption it depends on>
```

A verdict of `sound` is a real and useful outcome. Do not manufacture findings to justify the pass;
equally, do not soften a Critical because three lanes and a green suite preceded you — that is
precisely why an unfound one would still be here.

**Return your findings as TEXT.** The harness blocks subagents from writing report files; I will
persist your output verbatim, in its own commit, before folding any of it.
