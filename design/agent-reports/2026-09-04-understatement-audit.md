# Understatement audit — what inputs make btctax emit a signed 1040 that UNDERSTATES tax, without refusing?

Branch `feat/schedule-1a-ty2025`, HEAD `45f59793`. Read-only audit. Every citation below was
independently re-opened and re-quoted by me from the working tree; where the sweep's citation did not
hold up I say so in place.

---

## 1. VERDICT

**Yes. Multiple input sets currently produce an understated return with no refusal, no advisory, and no
warning.** These are not blanks that are correct because the inputs say so — each is a line whose
*provenance is missing*: nothing collected it, nothing asked about it, and the guard that was supposed
to cover it is keyed on a predicate the form does not contain.

| Severity | Count |
|---|---|
| **Critical** | **2** |
| **Important** | **6** |
| Cleared (checked, fine) | 5 |
| Known-and-documented, restated not re-found | 2 |

The two Criticals share one shape, and it is the shape this repo already has a rule for
(*"widening an exemption is never the safe edit"*): **a guard whose YES-condition is a paraphrase of
the form's, narrower than the form's, so the omitted cases fail OPEN.**

One correction to the sweep that the owner must apply before folding: **the §402(g) Roth finding as it
was handed to me is a wrong fix.** Adding codes AA/BB/EE to the *line-1h amount* would contradict the
instruction's own carve-out. It belongs in the *limit test* and not in the *line-1h amount* — two
different sums. Detail in F7.

---

## 2. FINDINGS, ranked by filing risk

### C1 — CRITICAL. The Form 8615 kiddie-tax refusal is gated on "claimable as a dependent", a condition Form 8615's *Who Must File* does not contain

**The filer.** A 16-year-old with $60,000 of net 2025 crypto capital gains who supports himself out of
those gains, or a 21-year-old full-time student with $100,000 of gains and dividends. Neither is
claimable by anyone — §152(c)(1)(D)'s support test fails (they provided over half their own support)
and §152(d)(1)(B)'s gross-income test fails. They truthfully answer *"Can someone claim YOU as a
dependent?"* → **No**. All five of Form 8615's conditions nonetheless hold.

**The mechanism.** `crates/btctax-core/src/tax/return_1040.rs:989`:

```rust
    if ri.header.can_be_claimed_as_dependent_taxpayer != Some(false) {
        let unearned = sum_taxable_interest(ri)
            + sum_ordinary_dividends(ri)
            + capital_gain_line7(ri, state, year, ri.filing_status)
            ...
        if unearned > params.kiddie_unearned_threshold {
            return refusal(
                RefuseReason::KiddieTax,
                "a claimable-as-dependent filer with unearned income over the §1(g) threshold needs Form 8615 (parent's-rate tax) — out of scope for v1",
            );
```

Past `screen_inputs` the `None` arm is unreachable (the registry refuses it), so the gate is
effectively `== Some(true)`.

**The form** — `design/forms/extract/i1040gi--2025.txt:3927-3944`, verbatim:

> You must file Form 8615 if you meet all of the following conditions.
> 1. You had more than $2,700 of unearned income …
> 2. You are required to file a tax return.
> 3. You were either: a. Under age 18 at the end of 2025, b. Age 18 at the end of 2025 and didn't have
>    earned income that was more than half of your support, or c. A full-time student at least age 19
>    but under age 24 at the end of 2025 and didn't have earned income that was more than half of your
>    support.
> 4. At least one of your parents was alive at the end of 2025.
> 5. You don't file a joint return in 2025.

Identical at `i1040gi--2024.txt:3559-3576` with the $2,600 threshold. **Not one of the five conditions
is "you can be claimed as a dependent."** Condition 3(a) carries no support test at all. Conditions
3(b)/(c) test **earned** income against support — which unearned support satisfies.

**Direction.** §1(g)(1) makes the tax the **greater of** the child's-rate computation and the
allocable-parental-rate computation. Skipping §1(g) can therefore only ever *lower* the figure.
Magnitude is unbounded: the delta between the child's brackets and the parents' marginal rate on the
whole unearned amount.

**No other guard.** `grep -rn "8615" --include=*.rs crates/` returns this one screen plus docs — 10
hits, all in `tables.rs`, `return_refuse.rs` doc comments, `return_inputs.rs` doc comment, and this
block. No advisory. The scope attestation cannot reach it either: the child's gains **are** entered, so
their truthful answer there is also No. And nothing reads the filer's own DOB — `grep -rn
"date_of_birth" crates/btctax-core/src/ | grep -v "test\|assert"` returns only `classifier.rs` (ignored
bindings), `scrub.rs` and `scrub_axis.rs`; `return_1040.rs` never reads it.

**The compression is pinned by a green test measuring the wrong predicate** —
`return_1040.rs:4465-4468`:

```rust
        // NOT claimable as a dependent ⇒ never kiddie, even with high unearned income.
        let mut not_dep = dependent(dec!(9000));
        not_dep.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        assert_eq!(screened(&not_dep, &empty), None);
```

Note the adjacent D-8 test at `:4471-4480` correctly closed the *unanswered* hole and left the
truthful-`Some(false)` hole untouched. `design/SPEC_dependent_flag.md` states the acceptance criterion
as *"A filer who **can** be claimed as a dependent **cannot** silently escape the Form 8615 screen"* —
which is the paraphrase, written down.

**Smallest fix.** Widen the gate to the form's own disjunction, defaulting to refuse. The transcription
already exists in the extract; the facts it needs are age (btctax holds `date_of_birth`), a living
parent, and "earned income was more than half your support". Collect the two it lacks as class-(A)
declarations, live only when unearned > threshold and the filer is under 24, and refuse when unanswered.
The one-line stopgap, if collection is deferred: drop `!= Some(false)` so the screen runs on unearned
income alone and over-refuses a handful of adults — an over-refusal is not a defect here.

---

### C2 — CRITICAL. Schedule 2 **additions to tax** are structurally outside the scope attestation's reach, and its census asserts the opposite

**The filers, three concrete instances of one defect.**

1. **Excess APTC repayment (line 1a).** A family enrolls in a marketplace plan for 2025 estimating
   $50,000 of income and receives ~$18,000 of advance premium tax credit. They then realize $400,000 of
   bitcoin gain. Household income lands far above 400% FPL, so §36B(f)(2)(B)'s repayment cap does not
   apply and the **entire** advance must be repaid on Schedule 2 line 1a. §36B(f)(2)(A) makes it *"an
   increase in the tax imposed"*, **not gross income**.
2. **§4973 excise on an excess Roth/IRA contribution (line 8).** Single filer contributes $7,000 to a
   Roth in January 2025; $300,000 of realized gains puts MAGI past the §408A(c)(3) ceiling, so the whole
   contribution is an excess and 6% ($420) is owed on Form 5329 **every year until corrected**.
3. **Household employment tax (line 9).** MFJ filers pay a nanny $9,000 in 2025, crossing
   §3121(a)(7)(B)'s $2,800 threshold; ~$1,377 of FICA plus FUTA lands on Schedule H.

In all three cases the filer answers the scope attestation **truthfully No** and the packet emits.

**The mechanism.** The attestation is the *only* backstop for anything btctax never collects, and all
three of its limbs are scoped away from a non-income addition to tax.
`crates/btctax-core/src/tax/questions.rs:548-556`, the entire text a filer ever sees:

> In this tax year, did ANY of these happen? (a) **You received income** other than what you have
> entered here — a PENSION, ANNUITY or IRA DISTRIBUTION (Form 1099-R), SOCIAL SECURITY or railroad
> retirement benefits …, rent or royalties, a farm, a partnership, S corporation, estate or trust (any
> Schedule K-1), unreported tips, gambling winnings, alimony, a business this tool did not capture, **or
> anything else it never asked about**. (b) You EXERCISED AN INCENTIVE STOCK OPTION (ISO) … (c) You had
> any other item this tool never asked about that changes your ALTERNATIVE MINIMUM TAX …

The catch-all *"or anything else it never asked about"* sits grammatically **inside** limb (a)'s "You
received income" clause. Limb (b) is an ISO exercise; limb (c) is an AMT item. An APTC repayment, an
excess-contribution excise and a tax for **paying** wages are none of these.

**The form** — `design/forms/extract/f1040s2--2025.txt`:

```
    a   Excess advance premium tax credit repayment. Attach Form 8962 .   .   .   .   1a
  8     Additional tax on IRAs or other tax-favored accounts. Attach Form 5329 if required.
  9     Household employment taxes. Attach Schedule H  .  .  .  .  .  .  .  .  .  .   9
```

**The refusal that is supposed to cover line 1a cannot fire, and the code says so.**
`crates/btctax-core/src/tax/printed.rs:306-308`:

```rust
/// **Part I carries the AMT since §G-6 (2026-08-03), and nothing else.** Line 1a (excess advance
/// premium tax credit) has no input and would REFUSE if it did — repaying it *increases* tax, so
/// omitting it would understate.
```

A refusal conditioned on an input that does not exist refuses nothing. This is a **gate that cannot
fail** — the class this project's severity rule lists as still-blocking.

**The census reasons are non-sequiturs of the exact shape the project already flags.**
`crates/btctax-forms/forms/2024/f1040s2.map.toml`:

- `:79` — `{ line = "1a", rule = "unmodeled", reason = "Excess advance premium tax credit repayment (Form 8962) — btctax collects no Form 1095-A, so no marketplace APTC can exist to repay." }`
- `:110` — `{ line = "8", … reason = "Additional tax on IRAs or other tax-favored accounts (Form 5329) — btctax models no IRA, HSA, MSA, Coverdell or ABLE account." }`
- `:112` — `{ line = "9", … reason = "Household employment taxes (Schedule H) — btctax models no household employees." }`

btctax not collecting a 1095-A does not stop the filer's APTC from existing; btctax not modelling an
IRA does not stop the filer from having one. Each of these is *"btctax does not do X"* offered as
provenance for *"the filer did not do X"*.

**And the census asserts its own completeness against a rule these lines break.**
`f1040s2.map.toml:64-75`:

```
#    ★★ DIRECTION — THIS FORM IS THE DANGEROUS ONE. … every line here INCREASES what is owed, so a
#    blank that should have carried a number is an UNDERSTATEMENT. … Three such lines exist, and all
#    three are fail-CLOSED by an actual refusal, verified in source …
#      line 2  (AMT) … line 5 (Form 4137) … line 13 (W-2 box 12) …
#    Everything else on this form is a tax on income or an account btctax does not model at all, so
#    there is no value it could have carried.
```

Line 9 is a tax on wages the filer **paid** — neither a tax on income nor on an account. Line 1a is the
repayment of a credit. Both falsify the closing sentence. Lines 10 (Form 5405 first-time-homebuyer
repayment) and 17b (federal mortgage subsidy recapture) sit in the same class.

**Direction and emission.** `crates/btctax-core/src/tax/printed.rs:773` —
`let line23 = sch_2.map_or(Usd::ZERO, |s| s.line21);` — and
`crates/btctax-forms/src/form1040_full.rs:175` — `(need(&map.line23, "line23", y)?, Some(lines.line23)),`
— an **unconditional** push, unlike line 19 beside it. Total tax (line 24 = line 22 + line 23) is short
by the whole amount. `return_1040.rs:2011` confirms `Sch 2 Part I: L1z (excess-APTC) = 0 (no input)`.

**No other guard.** Machine-checked:
`grep -rniE "8962|1095-a|advance premium|aptc|household employ|8606|\b5329\b|\broth\b|4973" --include=*.rs crates/ | wc -l` → **10**, and every one is a comment, a test name, or a SHA constant
(`form6251.rs:446`, `amt.rs:103`, `map.rs:1526`, `full_return_forms.rs:516/585`, `sp4.rs:305` (a hash),
`return_1040.rs:1398/2011`, `oracle-harness/main.rs:319`, `golden_packet.rs:432`). No `RefuseReason`
among the **62** in `return_refuse.rs`; no `FORM_QUESTIONS` entry among the **17** `QuestionId`
variants; no `Advisory`. The one IRA guard that exists is not this one —
`return_refuse.rs:1273` fires on `ri.sch1.ira_deduction_claimed > Usd::ZERO`, the *deducted* traditional
amount, which is $0 by definition for a Roth or nondeductible contribution.

**The asymmetry is visible inside the registry itself.** `questions.rs:373-376` asks about an HSA
**contribution** — *"(a) anyone (you, your employer, or anyone else on your behalf) put money into one
for you"* — precisely because a contribution alone can trigger a Form 8889 excise. No equivalent
question exists for an IRA, where §4973 imposes the same class of excise.

**Smallest fix.** One edit, closing all three plus lines 10 and 17b: add a **limb (d)** to the
attestation, phrased about *tax owed* rather than *income received*, and pull the catch-all out of limb
(a) so it governs the whole question. Draft: *"(d) Anything happened that makes you OWE a tax this tool
never asked about — you had marketplace health coverage with advance premium tax credit (Form 1095-A),
you put money into an IRA, Roth IRA or other tax-favored account, you paid a household employee, or you
must repay a credit you claimed in an earlier year."* Then correct the five census reasons to name the
attestation limb as the gate, and correct the closing completeness sentence. Pair it with a wording-pin
test, as `the_out_of_scope_question_names_the_iso_exercise_and_the_amt_items` (`questions.rs:1409`)
already does for limbs (a)/(b).

---

### I3 — IMPORTANT. The SSTB prompt drops **Trading** and **Dealing** from Form 8995-A's own field list — the two bullets that name this tool's population

**The filer.** A sole proprietor running an over-the-counter bitcoin desk — buying and selling to
customers, ordinary dealer income on Schedule C. TY2025 taxable income before QBI is $600,000 (MFJ),
i.e. `Qbi199aRegime::AboveThePhaseInRange`. A proprietary trader with a §475(f) mark-to-market election
hits the identical path.

**The prompt** — `crates/btctax-core/src/tax/questions.rs:1232-1236`:

```rust
        prompt: "Is your business a SPECIFIED SERVICE trade or business? Answer YES if its principal \
                 asset is the reputation or skill of its owners or employees, or if it is in health, \
                 law, accounting, actuarial science, performing arts, consulting, athletics, financial \
                 services, brokerage services, or investing and investment management. (Form 8995-A, \
                 Part I, column (b).)",
```

**The source, two bullets absent from it** — `design/forms/extract/i8995a--2024.txt:256-260`:

> • **Trading**, including persons who trade in securities (as defined in section 475(c)(2)),
>   commodities (as defined in section 475(e)(2)), or partnership interests;
> • **Dealing** securities (as defined in section 475(c)(2)), commodities (as defined in section
>   475(e)(2)), or partnership interests; and

A dealer reads the eleven listed fields and matches none: they advise nobody, manage nobody's money,
and are not a broker *"arrang[ing] transactions between a buyer and a seller … for a commission or
fee"* (`:247-249`). There is no *"if unsure, answer YES"* fallback in this prompt. They answer **No**.

**The mechanism.** `Some(false)` is the one answer that produces no refusal at all —
`return_1040.rs:2709-2737` refuses on `None` (`SstbUnanswered`) and on `Some(true)` **inside** the
phase-in range (`SstbInPhaseInRange`); `Some(_) => {}` falls through. Then
`crates/btctax-core/src/tax/qbi_a.rs:122-128`:

```rust
pub fn qbi_after_sstb_exclusion(business_qbi: Usd, is_sstb: bool, regime: Qbi199aRegime) -> Usd {
    if is_sstb && regime == Qbi199aRegime::AboveThePhaseInRange { Usd::ZERO } else { business_qbi }
}
```

grants the full deduction where §199A(d)(3) allows **$0** — up to 20% of QBI.

**The prompt's own help text names the consequence** (`questions.rs:1238-1241`): *"past the phase-in
range an SSTB's qualified business income is EXCLUDED ENTIRELY (§199A(d)(3)), so an unasked \"no\" would
hand you a deduction the statute denies and understate your tax."*

**No other guard.** `grep -rniE "\btrading\b|\bdealing\b" --include=*.rs crates/` → 3 hits, all
unrelated (`optimize.rs:78` a comment, `event.rs:549` a test fixture `account: "trading"`,
`gemini.rs:29` "trading pair"). Nothing inspects the business description or a NAICS code. **No test
pins this prompt's field list** — contrast `questions.rs:1409`, which does pin the scope
attestation's — so shortening the list reds nothing. The string is single-sourced: the TUI renders
`SKIPPABLE_QUESTIONS[…].prompt` via `skippable_tristate!`, so this is the only wording any filer sees
on any surface.

**Evidence the author worked from the source, so this is a gap not a paraphrase-of-a-paraphrase:** the
prompt correctly omits *"engineering, architecture"*, which §199A(d)(2)(A) expressly excludes from the
§1202(e)(3)(A) list.

**Smallest fix.** Add the two bullets to the prompt string and a wording-pin test naming *trading* and
*dealing*. Two lines.

---

### I4 — IMPORTANT. The §402(g) refusal compares every filer against the **general** limit; the instruction sets a lower one for a SIMPLE-only filer

**The filer.** She changes jobs in 2025; both employers sponsor SIMPLE plans. She defers $16,500 at A
and $6,000 at B — two W-2s, each box 12 **code S**. Her §402(g) limit is **$16,500** because she has
only SIMPLE plans, so $6,000 is an excess deferral belonging on 1040 line 1h.

**The mechanism.** `crates/btctax-core/src/tax/return_refuse.rs:1203`:

```rust
    if deferral_tp > p.elective_deferral_limit || deferral_sp > p.elective_deferral_limit {
        return refuse(
            RefuseReason::ExcessElectiveDeferral,
            "one person's elective deferrals exceed the §402(g) limit — the taxable excess (1040 line 1h) is unmodeled in v1",
```

with `crates/btctax-adapters/src/tax_tables.rs:135` — `elective_deferral_limit: dec!(23000),  //
§402(g)(1), Notice 2023-75`. $22,500 < $23,000 ⇒ **no refusal**. Code S is on `INERT_BOX12_CODES`
(`return_refuse.rs:29`), so nothing else stops it, and line 1h does not exist —
`printed.rs:637`: *"L1a is the Σ of W-2 box 1, and L1z is 'Add lines 1a through 1h' — v1 has no 1b–1h"*.

**The form** — `design/forms/extract/i1040gi--2025.txt:2349-2355`, verbatim:

> If the total amount you (or your spouse if filing jointly) deferred for 2025 under all plans was more
> than $23,500 (excluding catch-up contributions, as explained later), include the excess on line 1h.
> This limit is generally **(a) $16,500 if you have only SIMPLE plans**, and (b) $26,500 for section
> 403(b) plans if you qualify for the 15-year rule in Pub. 571.

TY2024 identical at `i1040gi--2024.txt:2311-2318` with $23,000 / $16,000 / $26,000.

**Direction.** The other two limits the instruction names move the **safe** way — the 403(b) 15-year
limit and the age-50 catch-up make btctax *over*-refuse, which is why they are invisible. The SIMPLE
limit is the only one that moves the other way. Gap: up to **$7,000** of omitted §61 income in both
years.

**No other guard.** `grep -rn "16000\|16500\|SIMPLE\|catch-up" --include=*.rs crates/` returns no guard,
no advisory, no follow-up. `design/SPEC_full_return.md:391` carries the same compression: *"Σ box-12
elective deferrals (D/E/F/G/S) across employers > §402(g) $23,000"*, with no plan-type split.

**Smallest fix.** Detectable from the data btctax already holds: if **every** deferral entry for that
person is code S, the person has only SIMPLE plans. Add a `simple_only_deferral_limit` to
`FullReturnParams` and select it in the comparison. Guard the direction — an ambiguous mix should keep
the general limit only if the mix can be *shown* not to be SIMPLE-only.

---

### I5 — IMPORTANT. §108(b)(2)(G) bites on the year of **discharge**, but the refusal's liveness is the carryover coming **IN** — so the discharge year files clean and stamps an unreduced carryover into the next year as `Computed`

**The filer.** A 2025 filer with **no** capital-loss carryforward in, a $50,000 long-term crypto loss
realized in 2025, and $40,000 of debt cancellation excluded under §108(a) (insolvency), filing Form 982
by hand. §108(b)(2)(G) puts *"any net capital loss for the taxable year of the discharge, and any
capital loss carryover to such taxable year"* on the attribute-reduction list, so the $47,000 carrying
out of 2025 must be reduced. btctax never asks, files 2025 clean, and `report --write-carryover` stamps
the unreduced $47,000 into 2026's inputs. In 2026 the question *is* live, but it asks about 2026 and
the truthful answer is **No** — so the figure prints on 2026 Schedule D lines 6 and 14 under §6065, too
large, and continues to roll.

**The mechanism.** Both the unanswered refusal (the registry loop, `return_refuse.rs:816`) and the
adverse-value refusal share one liveness predicate — `crates/btctax-core/src/tax/questions.rs:200-203`:

```rust
pub fn carryforward_in_present(ri: &ReturnInputs) -> bool {
    let cf = ri.capital_loss_carryforward_in;
    cf.short > Usd::ZERO || cf.long > Usd::ZERO
}
```

used at `return_refuse.rs:847-853`:

```rust
    if crate::tax::questions::question_is_live(
        crate::tax::questions::QuestionId::ExcludedCanceledDebt, ri,
    ) && ri.excluded_canceled_debt == Some(true)
    { return refuse(RefuseReason::ExcludedCanceledDebtAttributeReduction, …
```

The worksheet's header condition is about the year being **filed**, not the year the carryover came
from — `design/forms/extract/i1040sd--2025.txt:1819`: *"If you excluded canceled debt from income in
2025, see Pub. 4681."* — transcribed verbatim at `capital_loss_carryover.rs:165`.

**The write it leaves open.** `crates/btctax-core/src/tax/return_1040.rs:3180-3184`:

```rust
pub fn capital_loss_roll_is_grounded(ar: &AbsoluteReturn, ri: &ReturnInputs) -> bool {
    ri.capital_loss_carryforward_in_provenance == CarryProvenance::Computed
        || ri.capital_loss_carryforward_in != Carryforward::default()
        || rounded_capital_loss_carryforward_out(ar) != Carryforward::default()
}
```

The third disjunct authorises the write and never reads `capital_loss_carryforward_in`.
`apply_carryover_writeback` (`return_1040.rs:2949`) carries a §170(f)(8) / Reg §1.170A-7 gate and two
Form 8283 gates — I read the body — and **nothing for §108(b)**.

**Prior art, partial.** `FOLLOWUPS.md:5528-5539` (FR-20) records the *wording* mismatch and the Form 982
income-scope gap, and is marked **"Owning phase: none — needs a decision, not a phase."** It does not
reach the write-back. Note its proposed stopgap — narrowing the message to *"…while you are carrying a
capital-loss carryforward"* — would **entrench** this path rather than close it. That matters: folding
FR-20 as written makes this worse.

**Smallest fix.** Make `ExcludedCanceledDebt` live whenever the year could *produce* a capital-loss
carryover as well as when it carries one in — i.e. OR in a current-year net capital loss. That is one
disjunct, it fails closed, and it needs no new input.

---

### I6 — IMPORTANT. §221 ignores the collected "claimed as a dependent" answer the instruction makes a condition

**The filer.** A graduate student with a summer W-2 and a crypto ledger, claimed as a dependent on their
parents' return, who paid $2,500 of qualified student-loan interest. They answer
`can_be_claimed_as_dependent_taxpayer = Some(true)` (mandatory — `DependentStatusUnanswered` refuses
otherwise), their unearned income is under the §1(g) threshold so `screen_inputs` passes, and
`standard_deduction` correctly applies the §63(c)(5) dependent floor. Schedule 1 line 21 then prints the
full $2,500 anyway.

**The form** — `design/forms/extract/i1040gi--2024.txt:42212-42227` (grep misses it on the line break;
read with `sed -n '42210,42230p'`):

> **Line 21 / Student Loan Interest Deduction.** You can take this deduction only if all of the
> following apply. • You paid interest in 2024 on a qualified student loan … • Your filing status is any
> status except married filing separately. • Your modified adjusted gross income (AGI) is less than
> $95,000 … • **You, or your spouse if filing jointly, aren't claimed as a dependent on someone else's
> (such as your parent's) 2024 tax return.**

Same condition at `i1040gi--2025.txt:43070`.

**The mechanism.** `crates/btctax-core/src/tax/return_1040.rs:1017-1039` takes no `&ReturnInputs`, so it
structurally cannot consult the flag:

```rust
pub fn student_loan_deduction(
    paid: Usd, magi: Usd, status: FilingStatus, params: &FullReturnParams,
) -> Usd {
    let cap = paid.min(dec!(2500));
    if cap <= Usd::ZERO { return Usd::ZERO; }
    match params.student_loan_phaseout(status) {
        None => Usd::ZERO, // MFS — no deduction
```

Two of the instruction's four conditions are enforced (MFS, MAGI); the dependency condition is not. The
answer **exists and is mandatory** — `return_inputs.rs:262`, consumed at `return_1040.rs:160` for the
§63(c)(5) floor and at `:989` for the kiddie screen — but never here. Call site
`return_1040.rs:1719-1724` passes `ri.sch1.student_loan_interest_paid` and nothing about the filer.

**No other guard.** `grep -rn "student_loan" return_refuse.rs advisories.rs` returns only a
negative-amount screen (`return_refuse.rs:712`) and phase-out constants. The result flows into
`Schedule1Parts::student_loan_21`, is printed unconditionally at `printed.rs:461`, then Schedule 1 line
26 → 1040 line 10. The filer's own truthful YES sits on page 1's checkbox and is contradicted by line 21
on the same signed document.

**Direction.** Understates AGI by up to $2,500. Small in dollars (~$250–925), but it is a *self-
contradicting* return and the fix is trivial.

**Smallest fix.** Take `can_be_claimed_as_dependent_taxpayer` as a parameter and return `Usd::ZERO` on
`Some(true)`. Fail closed on `None` too. One argument, one branch, one test.

---

### I7 — IMPORTANT (and the sweep's version of it is a WRONG FIX). The §402(g) limit test omits designated-Roth codes it admits — but the *line-1h amount* must exclude them

**What the sweep handed me.** *"`ELECTIVE_DEFERRAL_CODES` omits AA/BB/EE — a pre-tax + Roth mix over the
limit files with 1040 line 1h blank"*, with the implied fix of adding those codes to the sum.

**Why that fix is wrong, from the instruction itself** — `i1040gi--2024.txt:2318-2322` / `--2025.txt:2356-2361`, verbatim:

> **Although designated Roth contributions are subject to this limit, don't include the excess
> attributable to such contributions on line 1h. They are already included as income in box 1 of your
> Form W-2.**

So there are **two different sums**: Roth deferrals belong in the **limit test** (they are subject to the
limit) and must be **excluded from the line-1h amount** (already in box 1). A single widened
`ELECTIVE_DEFERRAL_CODES` conflates them.

**What survives, verified.** The two lists at `crates/btctax-core/src/tax/return_refuse.rs:29-32`:

```rust
const INERT_BOX12_CODES: &[&str] = &["D", "E", "F", "G", "H", "S", "AA", "BB", "EE", "DD"];
/// The §402(g) elective-deferral codes whose cross-employer sum is capped (SPEC F3).
const ELECTIVE_DEFERRAL_CODES: &[&str] = &["D", "E", "F", "G", "S"];
```

btctax **admits** AA/BB/EE and does not count them, so the sum the guard tests is not the sum the statute
caps: a filer with code D $15,000 at employer A and code AA $15,000 at employer B is $7,000 over the
limit and is not refused. Whether that becomes an understatement depends on how the corrective
distribution is designated — if against the pre-tax deferrals (the ordinary correction), Reg
§1.402(g)-1(e) makes it includible for the year of deferral on line 1h, which v1 cannot print. So the
**refusal** is unambiguously missing; the **omitted income** is conditional.

**Prior review, never folded.** `design/full-return/reviews/DESIGN-audit-fold-confirm-r2.md:120-124`
raised it as m-r2-5 and said to record it in FOLLOWUPS. `grep -n "Roth\|AA/BB\|m-r2-5" FOLLOWUPS.md`
returns nothing.

**One more thing the folder must not break.** `return_refuse.rs:1203` compares a flat sum against the
limit **without subtracting catch-up contributions**, which `i1040gi--2024.txt:2332-2335` says *"isn't
subject to the overall limit on elective deferrals."* That currently **over-refuses** a 50-plus filer at
$30,500 — not our target, but it means the right fix touches this comparison in two directions at once.

**Smallest fix.** Add AA/BB/EE to a *separate* `ROTH_DEFERRAL_CODES` list, test the limit against the
union, and keep the refusal's detail text accurate about which excess reaches line 1h. Do **not** widen
`ELECTIVE_DEFERRAL_CODES` in place.

---

### I8 — IMPORTANT. Five refusals are named in no test anywhere in the workspace — four of them on the QBI path, which is where finding I3 also lives

`RefuseReason` has **62** variants (counted from the enum body). These are constructed in production and
never named in any test, `tests/` directory included:

| Variant | Construction site | Total refs in `crates/**/*.rs` |
|---|---|---|
| `SstbInPhaseInRange` | `return_1040.rs:2728` | 1 |
| `CooperativePatron` | `return_1040.rs:2693` | 2 (the other is a comment in `line_coverage.rs:598`) |
| `CooperativePatronUnanswered` | `return_1040.rs:2684` | 1 |
| `QbiCarryforwardNeedsSchedule8995AC` | `return_1040.rs:2778` | 1 |
| `IncomeExclusionUnanswered` | `questions.rs:516` | 1 |

Measured with `for v in …; do grep -rn "RefuseReason::$v" --include=*.rs crates/ | wc -l; done`.

This is the repo's own **B1** rule unmet: *"which test reds when this checker is removed?"* — for these
five, none. Note "named in a test" is *weaker* than seen-red-once, so the other 57 are **unproven**, not
proven. Four of the five gate §199A, the same neighbourhood as I3: if `SstbInPhaseInRange` silently
stopped firing, a partially-excluded SSTB would get a full deduction and nothing would red.

**Smallest fix.** Five assertions. They are cheap and they are the B1 minimum.

---

## 3. CLEARED — checked, and fine

| Claim checked | Verdict |
|---|---|
| W-2 box 5 `#[serde(default)]` silently zeroes §3101(b)(2) Additional Medicare Tax (`return_inputs.rs:65`) | **Cleared.** Refuted 3/3 by the sweep's own verification; I did not re-open it. |
| The two-oracle sweep has no witness for 1040 lines 25–33 / Schedule 3 Part II | **Cleared** as a *defect*; it is a real coverage boundary, not an understatement path. Refuted 3/3. |
| §170(e)/(b)/(d) charitable chain is unwitnessable by the corpus admission rule | **Cleared.** Refuted 3/3. |
| The promote-basis drift advisory is reachable only from `btctax verify`, never the filing path | **Cleared.** Refuted 3/3. |
| **The crypto-slice export fail-open** — `admin.rs:712` maps `ProfileOutcome::Uncomputable` to `None`, violating `resolve.rs:163`'s *"The caller MUST surface `detail` and NOT compute (fail-closed)"* | **Cleared, with a caveat.** There **is** a compensating guard the completeness critic missed: `admin.rs:731` computes `se_income_without_profile = se_computed.is_none() && !se_net_income(state, tax_year).is_zero()` — commented *"a NOTE, not a silent skip … never a fabricated form"* — and it is surfaced to the user at `crates/btctax-cli/src/main.rs:938`. SE income with no profile is therefore **not silent**. What remains is a real but lesser defect: the `detail` string is discarded, so the note is generic where a specific reason existed. Not an understatement finding. |

Also checked and **not** reported, because the guard exists:

- **Schedule 2 line 13** (uncollected SS/Medicare on tips / group-term life, box 12 codes A/B/M/N) — closed by the fail-closed **allowlist** at `return_refuse.rs:29`; any code outside it refuses with `UnsupportedBox12Code`.
- **Schedule 2 line 5** (Form 4137) — closed by `RefuseReason::AllocatedTips` on W-2 box 8.
- **Schedule 2 line 6** (Form 8919) — direction is safe and the census says why: such income reaches btctax as SE income and pays **both** halves through Schedule SE, so omitting Form 8919 **over**states.
- **Schedule 2 line 2** (AMT) — closed by `return_1040.rs:1576` on `ar.amt.must_attach()`.

---

## 4. AUDIT GAPS — what the next round must cover

Ranked by exposure. The first three are the ones I would not ship without.

1. **The crypto engine was audited by no lens — and it is the only part of the return derived from the
   filer's own data.** `fold.rs`, `pools.rs`, `resolve.rs`, `conservative.rs`, `conservative_promote.rs`,
   `optimize.rs`, `tax/method.rs` — ~9,000 LOC. Every finding above and below cites the 1040
   *scaffolding*. Nothing asked whether the **gain figures** are right. And this layer is structurally
   invisible to the double-oracle sweep — it is the §G-9 limit exactly: the oracles are *handed*
   proceeds and basis (`scripts/oracle/ots_direct.py:122` writes `f"  {basis + gain:.2f}\t6-01-2024"`),
   they never derive them. An overstated basis floor or a short-term lot printed as long-term
   understates tax on a signed Form 8949 with every 1040-layer refusal satisfied and both oracles green.
   `conservative_promote.rs:87` notes *"the FOLD is UNCHANGED — it always folds the STORED `filed_basis`
   … NOTHING is written"*, which is the seam to attack first.

2. **The year axis was never traversed.** `crates/btctax-forms/src/lib.rs:68` —
   `pub const SUPPORTED_YEARS: &[i32] = &[2017, 2024, 2025];` — while
   `crates/btctax-adapters/src/tax_tables.rs:91` says *"**v1 bundles TY2024 only**; a year without params
   returns `None` → the caller fails closed."* Measured: `ls crates/btctax-forms/forms/{2025,2024,2017}/*.map.toml | wc -l` → **5 / 17 / 5**. Every citation in this report is TY2024 or TY2025-extract. A
   TY2025 filer — the year this branch exists to build — has no `FullReturnParams`, so the full-return
   pipeline is unavailable and they are routed to the crypto slice, where **none of the 62 refusals
   run** (`admin.rs:574` dispatches on `return_inputs::exists`). Nobody asked what that filer receives.
   This is B3's field-of-view failure on the *year* dimension.

3. **Nobody asked whether a correctly computed line lands in the right BOX — and this branch's own spec
   documents the verifier going blind.** `design/ty2025/SPEC_schedule_a_ty2025.md:308-325`, verbatim:
   *"The ordinal-y descent leg catches nothing. … A uniform downward shift preserves monotonicity.
   Measured: 0 violations across all 16 resolvable writes."* — while
   `crates/btctax-forms/src/schedule_a.rs:33` promises *"read back through the geometric verifier (a
   mis-mapped cell FAILS CLOSED)"* and `:31` hardcodes TY2024-measured clusters
   `[(331.0, 403.0), (417.0, 489.0), (504.0, 576.0)]` that the moved TY2025 boxes still happen to fall
   inside. The spec names the concrete outcome: line 9 printing in the line 11 box and line 11 in the
   line 13 box, green on every leg. **This is already written down** — it needs closing, not
   re-discovering.

4. **No lens executed anything.** All six were static reads, and so was mine. The repo's own
   highest-yield instrument — the operator journey walk — was not among them, and the worked example
   this audit was seeded with (the retiree pattern-matching a prompt list) is itself a *wording* defect,
   findable only by reading a prompt as a human in situ. `crates/btctax-tui-edit/src/edit/form.rs` and
   `edit/persist.rs` (9,130 LOC of what a filer actually types into) are cited by zero findings. To keep
   the next round from re-deriving it: the orphan-question worry is **CLOSED** — all **17** `QuestionId`
   variants are named in `btctax-input-form` or `btctax-tui-edit`. The gap is wording and traversal,
   not wiring.

5. **Schedule 2 and Schedule 3 were sampled by hand where a census exists.**
   `cargo run -q -p xtask -- line-coverage` → *"279 money lines across 16 form(s) [f1040:45 f1040s1:10
   **f1040s2:6** f1040s3:5 …]"* — 6 of Schedule 2's 21 lines are modelled. C2 above names five of the
   other fifteen (1a, 8, 9, 10, 17b) and clears four (2, 5, 6, 13). **Six remain untriaged**: lines 11,
   12, 14, 15, 16, and the 17a/17c–17z menu. Subtract the census from the extract's own label set rather
   than hand-picking — the *"enumerate the line set FROM the form"* rule, applied to the audit.

6. **`AbsoluteReturn` → PDF for Schedule 3 Part II and the payments block.** The refuted "no oracle
   witness" claim is a coverage boundary, not a defect — but a coverage boundary with no witness and no
   lens is exactly where the next Critical hides. A refundable credit overstated understates tax just as
   surely as a missing addition.

---

## 5. WHAT I VERIFIED, AND HOW

Every finding was re-opened from the working tree. Commands run and their output:

```
$ git log --oneline -1
45f59793 decisions(D-A..D-H): Fable rules as the owner's delegate — and flags a REAL-MONEY deadline

$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 279 money lines across 16 form(s) [f1040:45 f1040s1:10 f1040s2:6 f1040s3:5
f1040sa:19 f1040sb:5 f1040sc:7 f1040sd:19 f1040sse:22 f6251:41 f8949:12 f8959:17 f8960:15 f8995:16
f8995a:39 i1040gi:1], 15 exception(s) (ratchet 15), 0 unverifiable (ratchet 0), 8 not line-bound (ratchet 8)

$ cargo run -q -p xtask -- --help
usage: cargo run -p xtask -- <docs [--pdf] | examples | subcommand-coverage | check-isolation |
line-coverage | cite-check | authority-conflicts | harness-check | archive-check | authority-manifest
[--regen] | extract-geometry <stem> | label-census <stem> | label-proof <stem> | label-boxes <stem> |
classify-path <path> | extract-schedule-1a | dump-fields <pdf>>

# C1 — the guard, the form, the pinning test
$ sed -n '980,1010p' crates/btctax-core/src/tax/return_1040.rs      # `!= Some(false)` confirmed
$ sed -n '3920,3950p' design/forms/extract/i1040gi--2025.txt        # five conditions, no dependency
$ sed -n '3555,3580p' design/forms/extract/i1040gi--2024.txt        # identical, $2,600
$ sed -n '4455,4480p' crates/btctax-core/src/tax/return_1040.rs     # the Some(false) assertion
$ grep -rn "8615" --include=*.rs crates/ | wc -l                    # 10 — all docs + this one screen
$ grep -rn "date_of_birth" crates/btctax-core/src/ | grep -v "test\|assert"
                                     # classifier/scrub only; return_1040.rs never reads it

# C2 — the prompt, the form, the census, the emission, the guard grep
$ sed -n '540,575p' crates/btctax-core/src/tax/questions.rs         # all three limbs quoted
$ sed -n '10,60p' design/forms/extract/f1040s2--2025.txt            # lines 1a, 8, 9 quoted
$ sed -n '60,130p' crates/btctax-forms/forms/2024/f1040s2.map.toml  # census + completeness claim
$ sed -n '300,315p;768,780p' crates/btctax-core/src/tax/printed.rs  # "would REFUSE if it did"; line23
$ sed -n '170,182p' crates/btctax-forms/src/form1040_full.rs        # unconditional Some(lines.line23)
$ grep -rniE "8962|1095-a|advance premium|aptc|household employ|8606|\b5329\b|\broth\b|4973" \
      --include=*.rs crates/ | wc -l                                # 10, every one a comment/test/hash

# I3 — SSTB
$ sed -n '1225,1250p' crates/btctax-core/src/tax/questions.rs       # eleven fields listed
$ sed -n '215,270p' design/forms/extract/i8995a--2024.txt           # thirteen, incl. Trading, Dealing
$ sed -n '2690,2740p' crates/btctax-core/src/tax/return_1040.rs     # Some(_) => {} falls through
$ sed -n '115,135p' crates/btctax-core/src/tax/qbi_a.rs             # full deduction on is_sstb=false
$ grep -rniE "\btrading\b|\bdealing\b" --include=*.rs crates/       # 3 hits, all unrelated

# I4 / I7 — §402(g)
$ sed -n '25,40p;1188,1215p' crates/btctax-core/src/tax/return_refuse.rs
$ grep -rn "elective_deferral_limit" --include=*.rs crates/         # dec!(23000), one scalar
$ sed -n '2305,2345p' design/forms/extract/i1040gi--2024.txt        # $16,000 SIMPLE + the Roth carve-out
$ sed -n '2347,2361p' design/forms/extract/i1040gi--2025.txt        # $23,500 / $16,500 / $26,500

# I5 — §108(b)(2)(G)
$ sed -n '195,215p' crates/btctax-core/src/tax/questions.rs         # carryforward_in_present
$ sed -n '840,860p' crates/btctax-core/src/tax/return_refuse.rs     # the gated refusal
$ sed -n '3170,3195p' crates/btctax-core/src/tax/return_1040.rs     # third disjunct authorises the write
$ grep -n "pub fn apply_carryover_writeback" -A 45 crates/btctax-core/src/tax/return_1040.rs
                                     # §170(f)(8) + two 8283 gates; nothing for §108(b)
$ grep -n "excluded canceled debt" design/forms/extract/i1040sd--2025.txt   # :1819, "in 2025"
$ grep -n "FR-20" -A 12 FOLLOWUPS.md                                # :5528, open, no code guard

# I6 — §221
$ sed -n '1010,1050p;1712,1730p' crates/btctax-core/src/tax/return_1040.rs # no &ReturnInputs
$ sed -n '42210,42230p' design/forms/extract/i1040gi--2024.txt      # the four conditions
$ grep -n "aren't claimed as a dependent\|aren’t claimed as a dependent" \
      design/forms/extract/i1040gi--2024.txt design/forms/extract/i1040gi--2025.txt   # :42225, :43070
$ grep -rn "student_loan" crates/btctax-core/src/tax/return_refuse.rs \
      crates/btctax-core/src/tax/advisories.rs   # only a negative-amount screen + constants

# I8 — untested refusals
$ awk '/^pub enum RefuseReason/,/^}/' crates/btctax-core/src/tax/return_refuse.rs \
      | grep -cE '^    [A-Z][A-Za-z0-9]*(\(|,)'                     # 62
$ for v in IncomeExclusionUnanswered CooperativePatron SstbInPhaseInRange \
           QbiCarryforwardNeedsSchedule8995AC CooperativePatronUnanswered; do
      grep -rn "RefuseReason::$v" --include=*.rs crates/ | wc -l; done   # 1 2 1 1 1

# Clearing the slice-path fail-open
$ sed -n '155,170p' crates/btctax-cli/src/resolve.rs                # the fail-closed contract
$ sed -n '700,760p' crates/btctax-cli/src/cmd/admin.rs              # :712 the None, :731 the NOTE
$ grep -rn "se_income_without_profile" crates/ --include=*.rs       # surfaced at main.rs:938

# Gaps 2 and 3
$ sed -n '60,75p' crates/btctax-forms/src/lib.rs                    # SUPPORTED_YEARS = 2017,2024,2025
$ sed -n '88,100p' crates/btctax-adapters/src/tax_tables.rs         # "v1 bundles TY2024 only"
$ ls crates/btctax-forms/forms/{2025,2024,2017}/*.map.toml | wc -l  # 5 / 17 / 5
$ sed -n '305,326p' design/ty2025/SPEC_schedule_a_ty2025.md         # "0 violations across all 16"
$ sed -n '25,40p' crates/btctax-forms/src/schedule_a.rs             # TY2024 hardcoded clusters
$ awk '/^pub enum QuestionId/,/^}/' crates/btctax-core/src/tax/questions.rs \
      | grep -cE '^    [A-Z][A-Za-z0-9]*,'                          # 17
```

**Where I departed from the sweep.**

- **Merged** the two independently-surviving Form 8615 findings (refusals lens, branches lens) — they
  are the same defect at the same line and cost one verification round each if kept apart.
- **Merged** the three Schedule 2 findings (APTC, Schedule H, IRA excise) into one structural Critical
  with three verified instances, and added lines 10 and 17b to its scope. They share one cause, one
  gate, and one fix; three tickets would produce three partial fixes.
- **Corrected** the §402(g) Roth finding. The instruction's own carve-out (*"don't include the excess
  attributable to such contributions on line 1h"*) makes the sweep's implied fix wrong. Restated as two
  sums, with the catch-up over-refusal named so the folder does not trade one direction for the other.
- **Cleared** the crypto-slice fail-open, which the completeness critic raised as Critical. The
  `se_income_without_profile` note at `admin.rs:731` → `main.rs:938` is the guard it did not grep for.
- I did **not** re-derive the four items the sweep refuted 3/3; they are listed as cleared on that basis
  and are labelled as such.
