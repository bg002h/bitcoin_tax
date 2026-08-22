# BRIEF — Fable review of Phase 2 (the refusal surface) + the Phase-1 folds

## SCOPE

`git diff 8f0f982a..HEAD` on `feat/filing-readiness`. That range is deliberately chosen: `8f0f982a`
is the commit that persisted the Phase-1 review, so this range is **everything nobody has reviewed
yet** — the folds that responded to that review, plus all of Phase 2, plus my merge resolutions.

Read `git log 8f0f982a..HEAD` first; the commit messages carry each item's mutations with verbatim
RED output, so you need not re-derive them.

## THE TWO QUESTIONS

**1. Is the refusal surface RIGHT?** Four new refusals now stand between a filer and a return:
`MortgageDebtLimitUnanswered`, `MortgageOverDebtLimit`, `CharitableCwaUnresolved`,
`Form4952DeclarationUnanswered`/`Form4952Required`. Each one can stop a lawful filer from filing.

- Does each fire **exactly** when it should — and stay silent when it should not?
- Is any of them reachable by a filer who owes nothing and should simply file?
- **P7's declaration is ALWAYS LIVE** — every filer now answers one more yes/no before any return
  computes. The implementer judged it forced (line 20 prints on every both-gains Schedule D, and that
  routing is a ledger fact liveness cannot see) and cited `OtherOutOfScopeIncome` as precedent. Is
  that right, or is there a liveness predicate it missed?

**2. What did the MERGE break?** Phase 2 branched from `3fc88497`, **not** from Phase 1 — it never
saw Phase 1's code. I resolved four conflicts and two hard breaks by hand, and **nobody has reviewed
those resolutions**. That is the highest-risk surface in this range.

## ★★ THE MERGE RESOLUTIONS — review these hardest

1. **`admin.rs` manifest order.** P5 appends an ATTACHMENT line (the qualified appraisal); N4 appends
   the hand-marks block at the foot. I kept both, appraisal first, and deliberately did NOT fold the
   appraisal into the hand-marks list (it is not a mark, and it is not the filer's to write). Is that
   the right split, and does the resulting manifest read correctly to someone assembling paper?
2. **`ScheduleDRouting::BothGains { line20_yes: true }`** in lane A's P2b cross-foot test. P7 made it
   a struct variant; that test did not exist in P7's branch. I chose `true` — lines 18/19 zero, no
   Form 4952. Correct for that fixture?
3. **★ Three of lane B's P6 carryover fixtures now answer `charitable_cwa_obtained: Some(true)`.**
   P4's gate refused them. I updated the FIXTURES rather than weakening P4, reasoning that a $40,000
   gift with no acknowledgment models a deduction §170(f)(8)(A) denies. **Check I did not thereby
   weaken what those tests prove** — and check whether any OTHER fixture in the tree is now passing
   only because it dodges a question rather than answering it.
4. Two large test-block conflicts in `export_irs_pdf.rs` and `full_return_forms.rs`, resolved by
   keeping both sides and re-closing a dangling `assert!`/`match`. Verify nothing was lost or
   duplicated, and that no test now asserts something its neighbour contradicts.

## ★ ALREADY MACHINE-CHECKED — do not re-run

- **2717 tests pass**, `clippy -D warnings` clean, `cargo fmt --all --check` clean.
- ~17 mutations across Phase 2, each observed RED, pasted into the commit messages.
- The Phase-1 folds were machine-checked before folding (F1's contradiction was reproduced from the
  tree's own tests).

## THE ADJUDICATION IS BINDING — but read the deviations

`design/direction/ADJUDICATION-2026-08-21.md` settled D3/D4/D5/D7 against primary sources. Do not
re-litigate the rulings. **Do** check the implementer's four recorded deviations, especially:

★ **P4's macro.** The adjudication says "mirroring `donations_had_restrictions` (`decl_tristate!`)" —
which is internally inconsistent, because `donations_had_restrictions` is a SKIPPABLE using
`skippable_tristate!`. The implementer built the skippable, on the grounds that `decl_tristate!` is
always-live and would refuse the standard-deduction filer D4 explicitly says must never be asked. It
followed the ruling's REASONING over its WORDS. **Is that the right call?** If it is, the adjudication
text is what is wrong, and say so.

Also check: P4 measures "any single contribution ≥ $250" on the **contribution amount (FMV)**, not the
§170(e)-reduced claim, narrowed by `claimed_deduction > 0`.

## FORBIDDEN

- Re-reviewing Phase 1's items for individual correctness — done, with mutations, and reviewed once.
- Re-reporting anything in `reviews/filing-readiness-phase1-review.md`. F1–F5 are folded; F6 was
  deliberately filed rather than half-built (extending the census program to 2025 is not a nit).
- Re-litigating the settled adjudications.
- Style, prose, naming.
- **Phase 3 (P8 / Form 8960 line 9b) is being built RIGHT NOW in parallel. It is not in this range.
  Do not review it or report its absence.**

## KNOWN-OPEN, do not re-list

N3 (needs a core lane); TY2017 8283 column (i); TUI carryover reader; L0 filing-threshold note;
`LIMITATIONS.md:231`; the IN-side carryforward refusal; P5's carryover-year recurrence (assigned to
P6, mitigated in message text only); F6's 2025 census. You MAY argue a deferral was the wrong call.

## OUTPUT FORMAT

```
VERDICT: <sound | needs-changes | wrong-shape>

REFUSAL-SURFACE FINDINGS:
  For each: THE REFUSAL / WHEN IT FIRES / WHEN IT SHOULD / THE FILER IT WRONGLY STOPS (or wrongly lets through) / SEVERITY <C|I|M|N>

MERGE-RESOLUTION FINDINGS: <the four above, plus anything else the merge broke>

ADJUDICATION-DEVIATION VERDICTS: <for each of the implementer's four: sound / wrong, one line each>

IF SOUND: say so plainly, and name the two things you attacked hardest and how.

WHAT WOULD MAKE THIS REVIEW WRONG: <one sentence>
```

Critical only for a wrong figure on a filed return, data loss, a false safety claim, or a refusal
that stops a filer who is entitled to file.

**Return findings as TEXT** — the harness blocks subagents from writing report files. I will persist
your output verbatim in its own commit before folding any of it.
