# Phase 2 review — the refusal surface + the Phase-1 folds, scoped `8f0f982a..HEAD`

_Fable, read-only, dispatched per `reviews/BRIEF-filing-readiness-phase2.md`. The range starts at the
commit that persisted the Phase-1 review, so it covers everything nobody had reviewed: the folds that
RESPONDED to that review, all of Phase 2, and the controller's hand-resolved merge. **Persisted
VERBATIM before any fold.**_

★ The agent could not write this file itself — the harness blocks subagents from writing report files.
The controller is therefore the scribe; the text below is copied verbatim from the agent's returned
message, in its own commit, before any fold, so `git diff <this>..<fold>` still means what it should.

---

VERDICT: needs-changes

## REFUSAL-SURFACE FINDINGS

**R1 — `MortgageDebtLimitUnanswered`**
- WHEN IT FIRES: `schedule_a.mortgage_interest_1098 > 0` and the declaration is `None` (registry loop, `mortgage_question_live`).
- WHEN IT SHOULD: exactly then, as an *ask* — this matches the two pre-existing mortgage siblings (`MixedUseMortgageUnanswered`, `AmtQualifiedDwellingUnanswered`) and is consistent.
- WRONGLY STOPS: nobody, as the ask. But it is one half of the R2 trap: for the filer R2 describes, *no* answer computes — `None` refuses here, `false` refuses there, `true` is false testimony. Severity rolls into R2.
- SEVERITY: — (sound in isolation).

**R2 — `MortgageOverDebtLimit`** and **R5a — `Form4952Required` (the line-9 bound), one root cause**
- WHEN THEY FIRE: in `screen_inputs` (`return_refuse.rs:~805` and `~840`), on inputs alone — R2 on `Some(false)` with 1098 interest > 0; R5a on `Some(false)` with `investment_interest` over the i4952 ceiling.
- WHEN THEY SHOULD: only on a return that *deducts* the thing being limited. Both refusals condition a Schedule A deduction, and both fire on returns whose §63(e) election takes the **standard deduction** — where line 8a / line 9 never prints, nothing is sworn, and btctax can compute the return exactly. `screen_inputs` cannot see the election; the election is computed in `assemble_absolute`. The branch's *own* P4 establishes the correct shape — its gate sits in `screen_absolute` behind `ar.deduction_is_itemized`, and its mutation (A) was specifically rebuilt to prove the schedule_a-data-but-standard-election filer computes (`the_cwa_question_is_never_posed_...`, fixture 1). P1 and P7 refuse that same filer.
- THE FILER WRONGLY STOPPED — both populations are real, not contrived:
  - **P1**: the December-closing jumbo homebuyer. $1M post-2017 loan, one month's 1098 interest ≈ $5–6k; itemized total under the $29,200 MFJ standard. Truthful answer to the debt-limit question is No → refused. First-year-standard-deduction is the *common* outcome for late-year buyers.
  - **P7**: the crypto-margin renter — btctax's core audience. Bitcoin yields no interest or dividends, so the ceiling is ~$0 and *any* line-9 entry with a truthful "not filing 4952" refuses — including when SALT + margin interest lose to the standard deduction and the correct return claims nothing.
  - Neither has an honest in-product cure: `Some(true)` is a lie; there is no `ForceStandard` election; deleting the input is safe only if the filer first does the standard-vs-itemized arithmetic by hand — the computation the software exists to do. And the refusal text actively misdirects this filer: "file this year's Schedule A by hand" (they have no Schedule A to file) / "correct the line-9 amount" (the amount is correct).
- Determinacy note: the fix is not simply "gate on `deduction_is_itemized`" — when itemized-with-the-full-amount *beats* standard, btctax cannot know the true capped figure or therefore the true election, and refusing is right. The precise silent region is *itemized-with-full-amount < standard*: there the election is determinate under both hypotheses. `deduction_is_itemized` (computed against the full amounts) is exactly that predicate, and `screen_absolute` has it.
- On D3: this is not a re-litigation. D3's REFUSE ruling rests on "the form's 8a output is a determinate nonzero worksheet result btctax does not model" — a rationale that presupposes the deduction is claimed. Its accepted adverse branch describes over-limit filers by debt level because it assumed debt ⇒ affected return. Scoping the refusal to the population the *reasoning* covers is the same words-vs-reasoning move the implementer correctly made on P4's macro.
- SEVERITY: **C** — the brief's named Critical class: a refusal that stops a filer entitled to file, with a message that falsely tells them btctax cannot produce their return.

**R3 — `CharitableCwaUnresolved`**
- WHEN IT FIRES: `screen_absolute`, on `deduction_is_itemized` && (lines 11+12 post-§170(b)) > 0 && largest single contribution (FMV, `claimed_deduction > 0` per item) ≥ $250, answer `None` or `Some(false)`.
- WHEN IT SHOULD: also in the contribution year when the claim is **wholly deferred to carryover**. The `cwa_claimed > 0` conjunct treats a §170(b)-ceiling-zeroed year as "nothing claimed, nothing to disallow" (mutation (C)'s own message) — but a ceiling-zero is a *deferral*, not a denial: §170(d) carries the claim forward, btctax's own P6 machinery computes the carryover-out and tells the filer to roll it, and next year's line-13 claim is (correctly) outside this gate. Meanwhile §170(f)(8)(C) fixes the CWA deadline at the **contribution-year** filing. So the exact household in the gate's own fixture 3 (`AGI $0, itemizes on mortgage, $50k crypto gift`) files with the question never asked, the cure extinguishes at that filing (*Durden*), and the $50k is then deducted across later years unsubstantiated or lost. This is D4's prong (3) — the silently lost right — on the gate built to close it. The §170(e)-zero case (per-item `claimed_deduction = 0`, the misfire D4 names) is genuinely different: that claim is extinguished forever, and the per-item filter in `max_single_donation_contribution` handles it correctly. The two zeros differ in kind; the aggregate conjunct conflates them.
- THE FILER IT WRONGLY LETS THROUGH: the low-AGI itemizer whose whole charitable deduction defers — narrow but constructible, and the product's carryover advisory actively sets up the future claim.
- Otherwise this is the sharpest-scoped refusal in the set: the itemize conjunct, the per-contribution (never aggregate) measure, the FMV measure, the line-13 exclusion, and the deadline-quoting messages are all right, and the mutation record is exemplary (the rebuilt fixture after the surviving mutation is B1 working as designed).
- SEVERITY: **I**.

**R4 — `Form4952DeclarationUnanswered` (always live)**
- The always-live choice is **right**, and no liveness predicate was missed. Any `&ReturnInputs` predicate would have to guess at the ledger-driven both-gains routing — under-asking exactly the $0-income household the defect was measured on. The available alternative (P4's skippable-plus-`screen_absolute` shape, scoped to the routing) was considered and is genuinely inferior here: the answer is material beyond line 20 — the QDCGT worksheet's own lines 2–3 net out Form 4952 line 4g amounts, so the declaration conditions essentially every preferential-rate return, not just `BothGains`. The spared population would be filers with no capital events, no preferential income and no line 9, for whom the question costs one trivially-answerable yes/no. `OtherOutOfScopeIncome` is the correct precedent and the fail-closed prompt (i4952's exception negated, default Yes) is the right shape.
- One accepted edge, for the record: a filer who files Form 4952 solely to track a disallowed-interest carryover (no 4g election, no both-gains, standard deduction) answers Yes truthfully and is refused though their btctax return is unaffected. Vanishingly rare, fail-closed, fine. SEVERITY: — (sound); edge is N.

**R5 — `Form4952Required`**
- The `Some(true)` leg is right: btctax fills neither Form 4952, the SDTW, nor a 4g-adjusted QDCGT, and each of those is an understatement path.
- **R5a** (bound fires on standard-deduction returns): **C**, merged with R2 above.
- **R5b** — the ceiling under-counts the instruction it transcribes: it sums `int_1099` **box 1 only**, omitting box 3 Treasury interest — while the tree's own `sum_taxable_interest` says in terms "2b = box 1 + box 3; box 3 is NOT a subset of box 1". Treasury interest is unambiguously "investment income from interest" under i4952's exception. A filer with a T-bill ladder ($20k box-3) and $5k margin interest satisfies the exception, truthfully answers No, and is refused — over-refusal with no honest cure except forfeiting a lawful deduction or filing by hand. (Adjacent, defensible: crypto *lending* interest is also outside the ceiling although btctax treats it as interest-like for §1411 — its "interest" character under §163(d) is genuinely arguable, so excluding it is acceptable conservatism; note it in the doc.) SEVERITY: **I**.
- **R5c** — the bound's refusal detail is malformed: one string literal with embedded runs of ~20 spaces mid-sentence (`return_refuse.rs:827` — missing `\` continuations), printed to the filer as-is. It is also the only new refusal whose message no test pins (P1's and P4's both have message tests). SEVERITY: **M**.

## MERGE-RESOLUTION FINDINGS

1. **admin.rs manifest order — sound.** ATT line among the staple-order items, hand-marks block at the foot; the appraisal correctly kept out of the marks list (not a mark, not the filer's to write). The manifest reads correctly to a paper-assembler. One Minor: `claims_property_deduction` is `ar.schedule_a` parts `charitable_noncash_12 > 0` **without** `deduction_is_itemized` — `ScheduleAParts` is built whenever inputs exist regardless of election, so a standard-electing packet (low-AGI donor whose allowed-line-12 slice plus everything else loses to standard) gets the ATT line on a return claiming nothing, the exact instruction the resolution's own comment says it avoids. SEVERITY: **M**.
2. **`BothGains { line20_yes: true }` in the P2b cross-foot test — correct.** The fixture's subject is the 8949→line-10 cross-foot; lines 18/19 zero, no 4952; matches every other site; the merge comment says exactly why.
3. **The P6 fixtures answering `charitable_cwa_obtained: Some(true)` — correct, and P4 was not weakened.** Machine-checked: those tests prove carryover round-trip/persistence properties untouched by the answer; the alternative (weakening P4) would have been the real damage; and a $40k-gift fixture with no CWA does model a statutorily denied deduction. Dodge-sweep of the whole range: **no** fixture removed a gift, reduced an amount below a threshold, dropped a `schedule_a`, or removed an existing answer (`git diff` grep for removed gifts/amounts/`Some(..)` answers is empty apart from the `BothGains` variant change). Every fixture change in the range answers a question; none dodges one.
4. **export_irs_pdf.rs / full_return_forms.rs — nothing lost, nothing duplicated.** Machine-checked: the union of `fn` names in both parents (bca2c92e, d93e0131) minus HEAD is empty for both files (and for tax_report.rs); no duplicate `fn` names in HEAD; no test asserts what a neighbour contradicts (the moved `f1_23` blank-list assertion is now a positive read-back, and the blank list no longer contains it).

**Fold findings (Phase-1 folds, in scope):** F2–F5 are faithful to the review and correct as built. One Minor on the F1/M4 fold: the authority chooser falls back only when the prior year fails `screen_absolute` — it never runs `screen_inputs`, so a prior year btctax would refuse at input screening (e.g. an over-limit mortgage, whose overstated 8a understates TI and *overstates* the worksheet carryforward) still supplies its `capital_loss_carryforward_out` as "the authority", violating the fold's own stated rule ("rather than quoting a number off a refused return"). M4 is non-gating, hence **M**.

## ADJUDICATION-DEVIATION VERDICTS

(The enumerated list lives in the phase-2 worktree report outside this range; I judged the four the commits record.)

1. **P4 macro (skippable, not `decl_tristate!`): sound — and the adjudication's text is what is wrong.** `donations_had_restrictions` *is* a skippable; D4's parenthetical mislabels its own referent. Building `decl_tristate!` would have refused every standard-deduction filer at the registry loop, which D4's operative text ("a standard-deduction filer … is never asked", honored "structurally, by liveness") forbids. Following the reasoning over the label was the only reading that satisfies the ruling; the ADJUDICATION file should get a one-line erratum.
2. **P4 measure (contribution FMV, not the §170(e) claim; per-item `claimed_deduction > 0`): sound.** §170(f)(8)(A) conditions on "any *contribution* of $250 or more"; the min(FMV, basis) measure would drop the CWA exactly where ST-appreciated crypto needs it (understatement direction); the per-item filter implements D4's named misfire correctly.
3. **P4 added aggregate conjunct (`cwa_claimed > 0`, post-ceiling lines 11+12): wrong in one branch.** Right for §170(e)-zero (claim extinguished); wrong for §170(b)-ceiling-zero (claim *deferred* under §170(d), CWA deadline still dies at this filing) — this is finding R3.
4. **P5 two-instrument scoping (advisory pre-election, manifest on the claiming packet; the $500k test pre-ceiling everywhere): sound in design** — the operand discipline is exactly D7, both adjudication vectors are pinned pulling opposite directions — with the Minor that the manifest predicate as coded doesn't fully implement its own scoping rule (merge finding 1).

## WHAT WOULD MAKE THIS REVIEW WRONG

If Schedule-A inputs coexisting with a standard-deduction election were not a reachable filer state at the product surface, the Critical would collapse — but the engine's own P4 fixture 1 models precisely that state as valid and computing, and no `ForceStandard` election exists for the filer to escape through, so I found no such barrier.

---

Note on scope discipline: I did not re-run the ~17 mutations (declared machine-checked), did not touch Phase 3, and did not re-litigate D3/D4/D5/D7 — the Critical asks the refusals to honor the rulings' own rationales, not to reverse them. The two things attacked hardest were the merge's fixture answers (question 3, which came back clean and machine-checked) and the four refusals' liveness against the filer populations they can reach — which is where everything above came from.
