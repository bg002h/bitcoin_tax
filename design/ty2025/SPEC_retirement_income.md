# Retirement income — Form 1040 lines 4a–6b — SPEC

**Status: DRAFT r1** (written 2026-09-04, against the archived TY2025 finals). Covers **IRA
distributions (4a/4b)**, **pensions and annuities (5a/5b)** and **social security benefits (6a/6b)** —
the three income lines btctax has no field for at all.

Passes an independent review loop to 0 Critical / 0 Important before an implementation plan.
`design/ty2025/SPEC.md`'s parent decisions (D-1 … D-11) bind here unless restated.

---

## 1. Sourcing of record — READ THIS FIRST

There is no separate instruction document for these lines: **`i1040gi` carries them**, in the same file
that carries the Schedule 1-A instructions. Everything below is quoted from the extracted text layer,
never from a rendered page (`CLAUDE.md`, *Transcribe IRS forms*).

| authority | hash | what it holds |
|---|---|---|
| `f1040--2025.pdf` | archived, `design/forms/2025/` | the printed lines 4a–6d and line 9 |
| `i1040gi--2025.pdf` | `482e9c48` (`design/ty2025/SPEC.md:70`) | lines 4a/4b, 5a/5b, 6a–6d, the **Social Security Benefits Worksheet** and the **Simplified Method Worksheet** |

Extracts of record: `design/forms/extract/f1040--2025.txt` and
`design/forms/extract/i1040gi--2025.txt`. Every quote below carries its extract line number, and every
quote is machine-checkable by `cargo run -p xtask -- line-coverage-check`
(`crates/xtask/src/line_coverage_check.rs:610-636`).

**Nothing here needs an oracle to establish.** Both engines compute taxable social security, and both
should be asked (`CLAUDE.md`, *Two oracles*) — but §5's worksheet is the authority and an oracle is a
witness.

---

## 2. The gap, measured

**There is no field.** `Form1040Lines` (`crates/btctax-core/src/tax/printed.rs:501-585`) declares
`line1z, line1a, line2a, line2b, line3a, line3b, line7, line8, line9, …` and stops. So does
`Form1040Income` (`printed.rs:598-620`). Total income is composed at
`crates/btctax-core/src/tax/return_1040.rs:1697-1698`:

```rust
let total_income =
    wages + taxable_interest + ordinary_dividends + capital_gain + schedule_1_income; // L9
```

**And the census already says so, in the form's own words.** The line-9 coverage row
(`crates/btctax-core/src/tax/line_coverage.rs:2265-2272`) carries the instruction
*"Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income"* under
`Production::Combine`, whose contract is *"Blank iff every operand is blank"*
(`line_coverage.rs:64-68`). Three of the eight named operands have no field. The transcription is
verbatim and correct; the struct behind it is missing three summands.

**★★ The catch-all does not catch a retiree, and the registry says why.** The only thing between a
pensioner and a filed return that omits their pension is `other_out_of_scope_income`
(`crates/btctax-core/src/tax/return_refuse.rs:988-1001`). Its prompt
(`crates/btctax-core/src/tax/questions.rs:548-556`) enumerates *"rent or royalties, a farm, a
partnership, S corporation, estate or trust (any Schedule K-1), unreported tips, gambling winnings,
alimony, a business this tool did not capture, or anything else it never asked about."* It never says
**pension**, **IRA** or **Social Security** — and the same registry entry states the rule that makes
this fatal, twelve lines below:

> *"a filer cannot answer `no` to a category they were never shown."* — `questions.rs:583-584`

A retiree reading that list truthfully answers **No** (they have no rent, no farm, no K-1) and files a
return omitting §61 and §86 income under §6065. That is
`widening-an-exemption-is-never-the-safe-edit` in its purest form: a residual clause carrying weight
that only an enumerated YES-condition can carry.

**★ The cheapest half of the fix does not need this spec.** Adding *"a pension or annuity, an IRA or
retirement-plan distribution, or social security or railroad retirement benefits"* to that prompt's
limb (a) is a one-line, whole-surface improvement in the SAFE direction (widening a mandatory
question's YES-conditions — `questions.rs:585-587`) and should land whether or not this spec is built.
It converts a silent omission into a refusal. **See §11, OQ-1.**

---

## 3. Binding decisions

**S-1. REFUSE where an unanswered branch could UNDERSTATE; ADVISE where it can only OVERSTATE.**
This is the classifier for every branch in §4, so the refusal list is *derived* rather than chosen. It
is the repo's existing rule stated as a decision procedure: a conservative omission is permitted only
if the filer is told (`return_refuse.rs:203-213`, the `SingleEmployerExcessSs` retraction), and an
unasked question that can understate refuses (`HsaActivityUnanswered`, `OtherIncomeUnanswered`,
`SstbUnanswered`).

**S-2. LINES 4a AND 5a ARE `Option<Usd>`, BECAUSE THE FORM MANDATES A BLANK THERE.** Not a style
choice — an instruction:

> *"If the distribution from your IRA is fully taxable, enter the total distribution on line 4b;
> **don't make an entry on line 4a**."* — `i1040gi--2025.txt:2664-2667`

> *"If your pension or annuity is fully taxable, enter the total pension or annuity payments (from
> Form(s) 1099-R, box 1) on line 5b; **don't make an entry on line 5a**."* — `i1040gi--2025.txt:2876-2880`

★★ In v1 the fully-taxable branch is the **only** branch that computes (§4), so **line 4a is
structurally never populated**, and line 5a is populated only when the 1099-R shows a *smaller*
box 2a. Those are not forgotten lines — they are lines whose provenance is *"the form instructs a
blank here"*, which is exactly the distinction `CLAUDE.md`'s provenance table draws and which
`FOLLOWUPS.md` §G-11 says must be carried in the types. Precedent for a conditional money cell:
`ScheduleALines.line2: Option<Usd>` (`printed.rs:311-340`). Precedent for the emitter declining to
write: lines 34/35a/37 in `crates/btctax-forms/src/form1040_full.rs:263-317`, recorded at
`FOLLOWUPS.md:1747`.

**S-3. ONE class-(A) declaration per document, enumerating the YES-conditions FROM THE FORM'S OWN
EXCEPTION LIST — not one declaration per exception.** The instructions already enumerate them
(IRA: four numbered *Exceptions*, `i1040gi--2025.txt:2682/2698/2727/2758`; pension: fully-taxable vs
partially-taxable vs PSO vs line-1h vs rollover, `2846-2906`). The question is *"does any of these
apply?"*, `None` refuses unanswered, `Some(true)` refuses unsupported, `Some(false)` computes. Model:
`ReturnInputs::hsa_activity` (`return_inputs.rs:606-611`) and
`ReturnInputs::has_income_exclusion` (`return_inputs.rs:960-970`).

**S-4. THE SOCIAL SECURITY BENEFITS WORKSHEET IS IN SCOPE AND TRANSCRIBED IN FULL — 18 lines.**
Justified in §5. It is the whole of 6b, it is self-contained, it needs no other form, and refusing it
would refuse the single most common retirement return in the United States.

**S-5. WORKSHEET LINE 6 IS NOT `AbsoluteReturn::adjustments`.** The worksheet says *"Schedule 1,
lines 11 through 20, and 23 and 25"* (`i1040gi--2025.txt:3403`) — which **excludes Schedule 1 line 21**,
the student-loan interest deduction (`f1040s1--2024.txt:72`), and line 22 (*"Reserved for future
use"*, `:73`). btctax's `adjustments` is `early_wd + half_se + student_loan`
(`return_1040.rs:1715`) and `Schedule1Lines.line26` is documented as *"`15 + 18 + 21` here"*
(`printed.rs:441-442`). So worksheet line 6 in v1 is **printed Sch 1 L15 + printed L18**, transcribed
as two operands, never as `line26 − line21`. Taking `adjustments` understates provisional income,
which understates taxable benefits, which understates tax.

**S-6. THE IRA SIDE AND THE PENSION SIDE TREAT 1099-R BOX 2a DIFFERENTLY, AND THAT ASYMMETRY IS THE
FORM'S.** The pension instructions say outright *"If your Form 1099-R shows a taxable amount, you can
report that amount on line 5b"* (`i1040gi--2025.txt:2902-2904`). The IRA instructions **never mention
box 2a**: they route on fully-taxable-or-an-Exception (`2664-2790`). So 5b may take box 2a; **4b may
not**. A shared "taxable amount = box 2a" helper is wrong on one side, exactly the way Schedule 1-A's
shared rounding helper is wrong on one side (`SPEC_schedule_1a.md` S-1).

**S-7. THE FOUR §86 THRESHOLDS ARE STATUTORY AND UNINDEXED — DO NOT BUILD A PER-YEAR TABLE.**
Machine-checked, not assumed: `i1040gi--2024.txt:3391,3394,3424` prints $32,000 / $25,000 / $12,000 /
$9,000 and `i1040gi--2025.txt:3421,3424,3453` prints the identical four. A year table here would be the
mirror of `SPEC_schedule_1a.md` S-7's error — inventing indexation where the statute fixes the figure.
Pin it with a test that reads both extracts.

**S-8. THE WORKSHEET'S FIRST *Exception* IS ALREADY DISCHARGED BY AN EXISTING REFUSAL, AND THE COUPLING
GETS A TEST.** The bullet — *"You made contributions to a traditional IRA for 2025 and you or your
spouse were covered by a retirement plan at work…"* (`i1040gi--2025.txt:3246-3250`) — exists because the
IRA deduction and taxable benefits are mutually circular (Sch 1 line 20 sits inside worksheet line 6,
and the §219(g) phase-out MAGI includes 6b). btctax already refuses any claimed IRA deduction:
`RefuseReason::IraDeductionClaimed` (`return_refuse.rs:248`, fired at `return_refuse.rs:1273-1278`), so
the circular branch cannot arise. **That is a guarantee held in another module**, and a future
relaxation of that refusal would silently make this worksheet the wrong instrument — so it lands with
a KAT that reds when the coupling is broken (§10, M-8).

**S-9. ★★ THE COVERAGE CHECKER CANNOT TELL 4b FROM 5b FROM 6b TODAY, AND MUST BE STRENGTHENED BEFORE
THESE ROWS LAND (B1).** All three lines print the identical two words — *"Taxable amount"*
(`f1040--2025.txt:74,76,78`). `label_precedes` (`line_coverage_check.rs:270-320`) accepts the bare
sub-letter form `b` and then only requires the stem digit to appear within the preceding ~700
characters; lines 4a–6b sit inside one 700-character window, so **a row labelled `4b` quoting
*"Taxable amount"* matches at line 5b's position and passes.** That is the exact class r7 measured at
71 accepted misattributions (`line_coverage_check.rs:299-306`), and it is Form 6251 line 33 again.

Under **B1** this checker may not be relied on until it has been seen red on the planted defect. The
fix that fits the existing design: anchor the match to the **physical extract row** whose first token
is the stem label — the 4a/4b row is `f1040--2025.txt:74`, 5a/5b is `:76`, 6a/6b is `:78`, one row
each. The planted defect that must red is in §10, M-1. **This is a prerequisite of the feature, not a
follow-up**, because without it the three most important new rows in the census are unverified.

**S-10. THE MODEL IS YEAR-AGNOSTIC; THE COVERAGE ROWS AND THE ACROFORM MAP ARE YEAR-KEYED.** Lines
4a–6b exist in every year and the worksheet is unchanged (S-7). Only two things are year-shaped: the
line-9 operand list, which reads *"…6b, **7**, and 8"* in `f1040--2024.txt:75` and *"…6b, **7a**, and
8"* in `f1040--2025.txt:84`; and the field map. `Coverage` already carries a per-row `year`
(`line_coverage.rs:100-106`) with Form 6251 line 1 as precedent (`line_coverage.rs:462-468`).

---

## 4. Scope and non-scope

### 4.1 In scope

| line | v1 behaviour |
|---|---|
| **4a** | `Option<Usd>`, **always `None` in v1** — the form instructs a blank on the only branch that computes (S-2) |
| **4b** | Σ Form 1099-R box 1 over IRA-flagged documents, when the filer declares no Exception applies |
| **5a** | `Option<Usd>` — `Some(Σ box 1)` when box 2a < box 1; `None` when the pension is fully taxable |
| **5b** | Σ Form 1099-R box 2a (pension-flagged), per `i1040gi--2025.txt:2902-2904` |
| **6a** | Σ Form SSA-1099 / RRB-1099 **box 5** — worksheet line 1 (`i1040gi--2025.txt:3396-3398`) |
| **6b** | the Social Security Benefits Worksheet, transcribed in full (§5) |
| **9** | gains three operands: `+ line4b + line5b + line6b` |

### 4.2 Non-scope, each mapped to a refusal

| not covered | why | refusal (§8) |
|---|---|---|
| IRA rollover (*Exception 1*, `2682-2697`) | 60-day rule, a 2026-rollover statement requirement, and the rolled/not-rolled split are filer determinations | R-1 / R-2 |
| IRA basis, Roth, conversion, returned or recharacterized contributions (*Exception 2*, `2698-2726`) | routes to **Form 8606**, unmodelled | R-1 / R-2 |
| Qualified charitable distribution (*Exception 3*, `2727-2757`) | $108,000 cap, $54,000 one-time SIE sub-cap, age-70½ test, an attachment, and an ordering rule against nondeductible contributions | R-1 / R-2 |
| HSA funding distribution (*Exception 4*, `2758-2788`) | once-per-lifetime election, Form 8889 Part III testing period — and `hsa_activity` already refuses | R-1 / R-2 |
| Line **4c** checkboxes | the boxes exist only to flag Exceptions 1/3/4, all of which refuse ⇒ **structurally never checked, with a reason** | — |
| Simplified Method / General Rule (`2973-3036`, Pub. 939) | needs cost at the annuity starting date, the annuity starting date, age at ASD, months paid, and a **prior-year carryforward** (worksheet line 10) | R-3 / R-4 |
| A 1099-R with box 2a blank or *"Taxable amount not determined"* checked | the instruction is then *"you must use the General Rule"* (`2891-2895`) — there is no figure to transcribe | R-5 |
| PSO insurance-premium exclusion, $3,000 (`2907-2960`) | can only LOWER 5b ⇒ S-1 says advise, not refuse | **A-1** (advisory) |
| Disability pension before minimum retirement age; corrective distributions | *"report them on line 1h"* (`2846-2856`) — btctax has no line 1h | R-3 / R-4 |
| Lump-sum distribution / Form 4972 | ten-year averaging, out of scope | R-3 / R-4 |
| SS worksheet *Exception* 1 — IRA contribution + plan coverage (`3246-3250`) | circular with the IRA deduction; already refused upstream | **S-8**, `IraDeductionClaimed` |
| SS worksheet *Exception* 2 — repayments exceed benefits (`3251-3259`) | *"None of your benefits are taxable"* plus a possible §1341 deduction or credit we cannot compute | R-6 |
| SS worksheet *Exception* 3 — Form 2555 / 4563 / **8815**, adoption benefits, Puerto Rico income (`3261-3264`) | Pub. 915's worksheet adds those exclusions back; ours would give a **lower** 6b ⇒ understatement | R-7 |
| Lump-sum election, line **6c** (`3320-3327`) | can only REDUCE the taxable amount ⇒ S-1 says advise | **A-2** (advisory) |
| §11 terrorist-attack SSDI exclusion (`3273-3304`) | can only reduce 6a and 6b ⇒ S-1 says advise | **A-3** (advisory) |
| Line **6d** unanswered on an MFS return with benefits | assuming *lived apart* understates by up to 85% of benefits from the first dollar | **R-8** |

★ **Line 4c is the form's own list of what we refuse.** Its three checkboxes are `1 Rollover`,
`2 QCD`, `3` (write-in, used for HFD) — `f1040--2025.txt:75` — which is precisely
Exceptions 1, 3 and 4. Exception 2 needs no box. So the refusal set is not an invention: it is 4c read
backwards, and 4c never printing a check is a *consequence with a reason*, not an omission.

---

## 5. Should the Social Security Benefits Worksheet be in scope? — YES, and here is the argument

**Decision: transcribe it, all 18 lines.** Three reasons, in ascending order.

1. **It is the entire content of line 6b.** There is no simpler path: 6b is *"Taxable amount"* and the
   instructions say *"Use the Social Security Benefits Worksheet in these instructions to see if any of
   your benefits are taxable"* (`i1040gi--2025.txt:3242-3244`). Refusing the worksheet is refusing 6b,
   which is refusing every retiree.
2. **It is closed.** Every operand is a 1040 line, a Schedule 1 line, a filing-status constant, or a
   percentage. It reads nothing this spec does not already produce, it needs no other form, and it
   needs no prior-year carryforward — unlike the Simplified Method Worksheet, which needs four
   (§4.2). Its four *Exceptions* are the only escape hatches and all four are handled above.
3. **It is not circular.** Its line 3 combines 1040 lines *"1z, 2b, 3b, 4b, 5b, 7a, and 8"*
   (`3400`) — **6b is absent by construction**. Its line 6 reads Schedule 1 adjustments that in v1 are
   ½-SE and the early-withdrawal penalty, neither of which depends on benefits. So the order is
   `4b, 5b → Sch 1 L15/L18 → worksheet → 6b → line 9 → student-loan MAGI → AGI`, acyclic, and it slots
   ahead of `agi_before_student_loan` at `return_1040.rs:1709` without moving anything.

**The Simplified Method Worksheet is the opposite call and is REFUSED** (R-3/R-5). It is fully
transcribed in the same document (`i1040gi--2025.txt:2973-3036`, with Table 1 at `3037` and Table 2 at
`3061`) and is the natural next increment — but its line 6, *"the amount recovered tax free in
years after 1986 … enter the amount from line 10 of last year's worksheet"*, is a multi-year
carryforward, which is a feature with a persistence surface and a provenance flag (cf.
`QbiInputs::qbi_carryforward_in_provenance`, `return_inputs.rs:648-650`), not a worksheet.

### 5.1 The worksheet, transcribed

One field per numbered line, in the worksheet's own numbering, instruction text verbatim as the doc
comment. `ws` = this worksheet's lines.

| ws | instruction (verbatim) | extract |
|---|---|---|
| 1 | "Enter the total amount from box 5 of all your Forms SSA-1099 and RRB-1099. Also enter this amount on Form 1040 or 1040-SR, line 6a" | `3396-3398` |
| 2 | "Multiply line 1 by 50% (0.50)" | `3399` |
| 3 | "Combine the amounts from Form 1040 or 1040-SR, lines 1z, 2b, 3b, 4b, 5b, 7a, and 8" | `3400` |
| 4 | "Enter the amount, if any, from Form 1040 or 1040-SR, line 2a" | `3401` |
| 5 | "Combine lines 2, 3, and 4" | `3402` |
| 6 | "Enter the total of the amounts from Schedule 1, lines 11 through 20, and 23 and 25" | `3403` |
| 7 | "Is the amount on line 6 less than the amount on line 5? **No. STOP** None of your social security benefits are taxable. Enter -0- on Form 1040 or 1040-SR, line 6b. **Yes.** Subtract line 6 from line 5" | `3404-3408`, `3416` |
| 8 | "If you are: • Married filing jointly, enter $32,000 • Single, head of household, qualifying surviving spouse, or married filing separately and you lived apart from your spouse for all of 2025, enter $25,000 • Married filing separately and you lived with your spouse at any time in 2025, skip lines 8 through 15; multiply line 7 by 85% (0.85) and enter the result on line 16. Then, go to line 17" | `3420-3440` |
| 9 | "Is the amount on line 8 less than the amount on line 7? **No. STOP** None of your social security benefits are taxable. Enter -0- on Form 1040 or 1040-SR, line 6b. If you are married filing separately and you lived apart from your spouse for all of 2025, be sure you checked the box on line 6d. **Yes.** Subtract line 8 from line 7" | `3441-3447` |
| 10 | "Enter $12,000 if married filing jointly; $9,000 if single, head of household, qualifying surviving spouse, or married filing separately and you lived apart from your spouse for all of 2025" | `3453-3454` |
| 11 | "Subtract line 10 from line 9. If zero or less, enter -0-" | `3455` |
| 12 | "Enter the smaller of line 9 or line 10" | `3456` |
| 13 | "Enter one-half of line 12" | `3457` |
| 14 | "Enter the smaller of line 2 or line 13" | `3458` |
| 15 | "Multiply line 11 by 85% (0.85). If line 11 is zero, enter -0-" | `3459` |
| 16 | "Add lines 14 and 15" | `3460` |
| 17 | "Multiply line 1 by 85% (0.85)" | `3461` |
| 18 | "Taxable social security benefits. Enter the smaller of line 16 or line 17. Also enter this amount on Form 1040 or 1040-SR, line 6b" | `3462-3463` |

Plus the three *Before you begin* preconditions (`3389-3394`), of which the second — *"If you are
married filing separately and you lived apart from your spouse for all of 2025, check the box on
line 6d"* — is the input requirement behind R-8.

### 5.2 ★★ Two traps in this worksheet, both of a class this repo has been burned by

**T-1. Line 8's third bullet is a JUMP, and encoding it as "threshold $0" is right by accident.**
It says *skip lines 8 through 15*, then `ws16 = 0.85 × ws7`, then go to 17. Setting `ws8 = 0` and
letting 9–15 run gives: `ws9 = ws7`, `ws10 = 0`, `ws11 = ws7`, `ws12 = 0`, `ws13 = 0`, `ws14 = 0`,
`ws15 = 0.85 × ws7`, `ws16 = 0.85 × ws7` — **the same number**, and only because `ws10` is also zero on
that branch. That is `SPEC_schedule_1a.md` F-5's line-33 trap verbatim (*"gives the same answer only
because 6% × 0 = 0"*). Transcribe the jump; pin the branch with a KAT (M-5).

**T-2. Lines 7 and 9 STOP by writing `-0-` on 6b — an INSTRUCTED zero, not a blank.** This form
carries both halves of the §G-11 distinction on one page: **4a is left blank by instruction** (S-2)
and **6b is written as `-0-` by instruction**. Neither is a default, and a model that renders both as
`Usd::ZERO` has lost the difference the whole doctrine is about. Consequence for the census: 6b's
production is **`Exception` with a written reason** — *"the worksheet's two STOP branches instruct
`-0-`; a blank 6b means the worksheet never ran (no benefits), a `0` means it ran and stopped"* —
because no clamp-free production in the grammar (`line_coverage.rs:58-95`) expresses an instructed zero
that is also legitimately blank. That costs one unit of the exception ratchet, knowingly.

---

## 6. The 1040 lines, transcribed

Quotes are the form's own printed text (`f1040--2025.txt`), as the census requires.

| line | printed text | extract | field | production |
|---|---|---|---|---|
| 4a | "IRA distributions" | `:74` | `line4a: Option<Usd>` | `Collected` — **always `None` in v1** (S-2) |
| 4b | "Taxable amount" | `:74` | `line4b: Usd` | `Collected` |
| 4c | "Check if (see instructions) 1 Rollover 2 QCD 3" | `:75` | *(no field)* | never checked — every branch that would check it refuses (§4.2) |
| 5a | "Pensions and annuities" | `:76` | `line5a: Option<Usd>` | `Collected` |
| 5b | "Taxable amount" | `:76` | `line5b: Usd` | `Collected` |
| 5c | "Check if (see instructions) 1 Rollover 2 PSO 3" | `:77` | *(no field)* | never checked — rollover refuses (R-3), PSO is advisory (A-1) |
| 6a | "Social security benefits" | `:78` | `line6a: Option<Usd>` | `Carry` — worksheet line 1 |
| 6b | "Taxable amount" | `:78` | `line6b: Option<Usd>` | `Exception`, reason per T-2 |
| 6c | "If you elect to use the lump-sum election method, check here (see instructions)" | `:79` | *(no field)* | never checked — A-2 |
| 6d | "If you are married filing separately and lived apart from your spouse the entire year (see inst.), check here" | `:80` | `line6d_checked: bool` | `Collected` — from the R-8 declaration |
| 9 | "Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7a, and 8. This is your total income" | `:84` | `line9` | `Combine` — **gains three operands** |

★ Every one of these quotes must land **after** S-9's checker fix, or the three `"Taxable amount"`
rows are unverified by construction.

---

## 7. The input surface — what must be COLLECTED

New leaves on `ReturnInputs`, in the house shape (`return_inputs.rs:36-131` is the model for a typed
information return; boxes are named for the box, refuse-guards per box).

```
/// One Form 1099-R. Boxes are named for the box, exactly like `W2` and `Form1099Int`.
pub struct Form1099R {
    pub owner: Owner,
    pub payer: String,
    /// Box 1 — "Gross distribution". → 1040 4a/4b or 5a/5b, per `kind`.
    pub box1_gross_distribution: Usd,
    /// Box 2a — "Taxable amount". READ ONLY on the pension side (S-6).
    pub box2a_taxable_amount: Option<Usd>,
    /// Box 2b — the two printed checkboxes. `taxable_amount_not_determined` ⇒ R-5.
    pub box2b_taxable_amount_not_determined: bool,
    pub box2b_total_distribution: bool,
    /// Box 4 — federal income tax withheld. → 1040 25b.
    pub box4_fed_withheld: Usd,
    /// Box 7 — the distribution code(s), captured verbatim; screened, never interpreted.
    pub box7_distribution_codes: String,
    /// Which pair of 1040 lines this document reaches: 4a/4b or 5a/5b.
    pub kind: Form1099RKind,           // Ira | PensionOrAnnuity
    /// ★ class-(A) DECLARATION (S-3). `None` ⇒ R-1/R-3; `Some(true)` ⇒ R-2/R-4.
    pub exception_applies: Option<bool>,
}

/// One Form SSA-1099 or RRB-1099.
pub struct FormSsa1099 {
    pub owner: Owner,
    /// Box 3 — "total social security benefits paid to you" (i1040gi--2025.txt:3236-3237).
    pub box3_benefits_paid: Usd,
    /// Box 4 — "the amount of any benefits you repaid in 2025" (:3237-3239). Box 4 > box 3 ⇒ R-6.
    pub box4_benefits_repaid: Usd,
    /// Box 5 — NET benefits. **Worksheet line 1 reads THIS**, not box 3 (:3396).
    pub box5_net_benefits: Usd,
}
```

New class-(A) declarations on `ReturnInputs`, each `Option<bool>` — the type the classifier forbids `_`
on (`return_inputs.rs:963-966`), registered in `classifier.rs` beside
`c.declaration(other_out_of_scope_income, …)` at `classifier.rs:208`, and given a `FormQuestion` in
`questions.rs` with `durability: Durability::PerYear` and `neutral: false`:

| field | question, in the form's own YES-conditions | live when | `None` ⇒ |
|---|---|---|---|
| `Form1099R::exception_applies` | IRA: *"did you roll any of it over; do you have basis, a Roth, a conversion, a returned or recharacterized contribution; was any of it a qualified charitable distribution; was any of it an HSA funding distribution?"* | any IRA 1099-R | **R-1** |
| `Form1099R::exception_applies` | pension: *"was any of it rolled over; are you a retired public safety officer excluding premiums; is it a disability pension before your employer's minimum retirement age; is it a corrective distribution; is it a lump-sum distribution you are using Form 4972 for?"* | any pension 1099-R | **R-3** |
| `mfs_lived_apart_all_year` | *"If you are married filing separately and you lived apart from your spouse for all of 2025, check the box on line 6d"* (`3330-3332`) | MFS **and** Σ box 5 > 0 | **R-8** |
| `form_8815_or_adoption_exclusion` | *"You file Form 2555, 4563, or 8815, or you exclude employer-provided adoption benefits or income from sources within Puerto Rico"* (`3261-3264`), minus the part `has_income_exclusion` already asks | Σ box 5 > 0 | **R-7** |

★ `form_8815_or_adoption_exclusion` exists because `has_income_exclusion`
(`return_inputs.rs:960-970`) already covers §911 / §931 / §933 — Forms 2555 and 4563 and Puerto Rico —
but **not** Form 8815 (excluded savings-bond interest) or employer-provided adoption benefits. Asking
only the residue keeps the questionnaire honest and reuses the answer btctax already has. It is scoped
`live` to filers with benefits, so nobody else ever sees it. This is `CLAUDE.md`'s corollary applied
literally: *"If the form asks something our input surface cannot answer, collect it."*

★★ **No question is added for the PSO exclusion, the lump-sum election, or the §11 SSDI carve-out.**
Under S-1 each can only lower the figure, so each gets an advisory instead (§9) — the same call
`CtcOdcOmitted` and `AgedBoxForfeitedNoDob` already make (`advisories.rs:44-68`).

TUI surface: one `decl_tristate!` entry per declaration in
`crates/btctax-input-form/src/spec/registries.rs` (model at `:210-211`), plus the `QuestionId` mapping
at `:388`.

---

## 8. Refusals — exact wording and firing condition

All are `RefuseReason` variants (`return_refuse.rs:36-260`) raised through `refuse(reason, detail)`.
Wording follows the house voice: name the mechanism, name the direction of error, name what the filer
can do.

**R-1 `IraDistributionExceptionUnanswered`** — fires when any `Form1099R { kind: Ira }` has
`exception_applies == None`.
> "you entered an IRA distribution but did not say whether any of the four exceptions in the line 4a
> and 4b instructions applies to it — a rollover, a Form 8606 item (basis, a Roth, a conversion, a
> returned or recharacterized contribution), a qualified charitable distribution, or an HSA funding
> distribution. Silence is not testimony that none applies. It matters in BOTH directions: with no
> exception the whole distribution is taxable on line 4b, and a QCD you did not tell us about would
> also let the same gift be deducted again on Schedule A — which the instructions forbid. Answer it —
> run `btctax income answer`"

**R-2 `IraDistributionExceptionUnsupported`** — `exception_applies == Some(true)` on an IRA document.
> "you declared that one of the line 4a/4b exceptions applies to an IRA distribution. Each of them
> routes somewhere btctax cannot follow — a rollover to the 60-day rule and a filed statement, basis or
> a Roth to Form 8606, a qualified charitable distribution to the $108,000 limit and its attachment, an
> HSA funding distribution to Form 8889 Part III. btctax models only the fully-taxable case, so it
> refuses rather than file a line 4b it cannot stand behind. File that distribution yourself"

**R-3 `PensionExceptionUnanswered`** — any `Form1099R { kind: PensionOrAnnuity }` with
`exception_applies == None`. Same shape as R-1, naming the pension YES-conditions from §7.

**R-4 `PensionExceptionUnsupported`** — `Some(true)` on a pension document.
> "you declared that one of the line 5a/5b special cases applies — a rollover, the retired public
> safety officer premium exclusion, a disability pension before your employer's minimum retirement age
> or a corrective distribution (both of which the instructions send to line 1h, which btctax does not
> have), or a lump-sum distribution using Form 4972. btctax models only the case where your Form 1099-R
> states the taxable amount. File that distribution yourself"

**R-5 `PensionTaxableAmountNotDetermined`** — a pension document with `exception_applies == Some(false)`
and either `box2a_taxable_amount == None` or `box2b_taxable_amount_not_determined == true`.
> "your Form 1099-R does not state a taxable amount in box 2a. The instructions are then explicit:
> 'you must use the General Rule explained in Pub. 939 to figure the taxable part to enter on line 5b',
> or the Simplified Method Worksheet if your annuity starting date was after July 1, 1986. Both need
> your cost in the plan at the annuity starting date and how much you have already recovered tax free —
> figures btctax neither holds nor carries between years. It refuses rather than copy box 1 onto line
> 5b, which would OVERSTATE your tax"

**R-6 `SocialSecurityRepaymentsExceedBenefits`** — Σ box 4 > Σ box 3.
> "your Form SSA-1099 shows you repaid more benefits than you received. The instructions say none of
> your benefits are taxable for the year, and that if the excess is more than $3,000 you may be able to
> take an itemized deduction or a credit for part of it. btctax computes neither, and filing without
> them would OVERSTATE your tax. See Pub. 915 and file this return yourself"

**R-7 `SocialSecurityWorksheetBarred`** — Σ box 5 > 0 and
(`has_income_exclusion == Some(true)` or `form_8815_or_adoption_exclusion == Some(true)`); the `None`
case on the latter refuses as `SocialSecurityWorksheetBarUnanswered` with the standard
*"silence is not testimony"* detail.
> "you declared an exclusion the Social Security Benefits Worksheet will not accept — Form 2555, 4563
> or 8815, employer-provided adoption benefits, or income from Puerto Rico. The instructions send you
> to the worksheet in Pub. 915 instead, which adds those excluded amounts back before testing your
> benefits. btctax has only the in-instruction worksheet, so its line 6b would come out too LOW and
> UNDERSTATE your tax. Use the Pub. 915 worksheet and file this return yourself"

**R-8 `MfsLivedApartUnanswered`** — filing status MFS, Σ box 5 > 0, `mfs_lived_apart_all_year == None`.
> "you are married filing separately and received social security benefits, and you have not said
> whether you lived apart from your spouse for ALL of the year. The two answers are not close: living
> apart all year gives you the $25,000 and $9,000 thresholds, while living together for any part of it
> skips straight to 85% of your benefits from the first dollar. btctax will not pick the answer that
> happens to lower your tax. Answer it — run `btctax income answer`"

★ R-8 is the sharpest refusal here and the reason S-1 exists. Defaulting `mfs_lived_apart_all_year` to
`true` is precisely `widening-an-exemption-is-never-the-safe-edit`; defaulting it to `false` overstates
for the larger population. Neither default may be taken.

---

## 9. Advisories — the branches that can only overstate

Added to `Advisory` (`advisories.rs:43`), each carrying its figure, in the shape of
`CtcOdcOmitted { dependents, provably_zero }`.

**A-1 `PsoPremiumExclusionNotTaken { pensions: usize }`** — fires when line 5b > 0. Quotes the ceiling
verbatim: *"You can exclude from income the smaller of the amount of the premiums paid or $3,000"*
(`i1040gi--2025.txt:2926-2928`). Overstates tax for an eligible retired public safety officer.

**A-2 `SocialSecurityLumpSumElectionNotTaken`** — fires when line 6b > 0. Quotes the worksheet's own
TIP: *"If any of your benefits are taxable for 2025 and they include a lump-sum benefit payment that
was for an earlier year, you may be able to reduce the taxable amount"* (`3470-3471`).

**A-3 `SocialSecurityDisabilityExclusionNotTaken`** — fires when line 6a > 0. The §11-attack SSDI
carve-out (`3273-3304`), which the instructions illustrate with a worked example. Overstates tax for
the few it reaches; asking every beneficiary about a terrorist attack is questionnaire bloat that buys
nothing, because the failure direction is already conservative.

**A-4 `SimplifiedMethodNotUsed { pensions: usize }`** — fires when line 5b came from box 2a and box 2a
> 0. Verbatim: *"you may be able to report a lower taxable amount by using the General Rule or the
Simplified Method"* (`2904-2906`). ★ This is the honest cost of S-6: taking box 2a is what the
instructions permit, and it is also the *higher* of the two lawful figures.

---

## 10. How it is tested

Every guarantee below names the mutation that must make it RED (**B1**). A guarantee without one does
not exist.

| # | guarantee | the mutation that must red it |
|---|---|---|
| **M-1** | ★ the census can tell 4b from 5b from 6b (S-9) | swap the `line` labels on the 4b and 6b coverage rows, leaving both quotes as `"Taxable amount"`. **Today this passes** — that is the finding. It must red before the rows land. |
| **M-2** | 1040 line 9 sums the three new operands | delete `+ line6b` from the line-9 sum; a household with only benefits must stop reconciling |
| **M-3** | line 4a is blank, not `0`, on a fully-taxable IRA | change `line4a: Option<Usd>` to `Usd` / emit `Usd::ZERO`; the emitted-PDF read-back (`extract_lines`, `crates/btctax-forms/tests/extract_lines.rs`) must show the 4a cell present |
| **M-4** | line 5a is blank when the pension is fully taxable and present when box 2a < box 1 | force `line5a = Some(box1)` unconditionally |
| **M-5** | worksheet line 8's MFS-lived-with branch is a JUMP (T-1) | replace the jump with `ws8 = 0` and let 9–15 run. ★ The v1 arithmetic is unchanged, so the KAT must assert the **branch taken**, not only the dollar — a value-only test here is vacuous by construction |
| **M-6** | worksheet line 6 excludes Schedule 1 line 21 (S-5) | change it to `ar.adjustments`; a household with student-loan interest and benefits must move |
| **M-7** | worksheet line 1 reads box 5, not box 3 | swap to `box3_benefits_paid`; a filer with a repayment in box 4 must move |
| **M-8** | the S-8 coupling: the worksheet is only the right instrument while IRA deductions refuse | delete the `IraDeductionClaimed` guard at `return_refuse.rs:1273-1278`; a KAT asserting *"a claimed IRA deduction never reaches the SS worksheet"* must red |
| **M-9** | R-8 refuses rather than defaulting | set `mfs_lived_apart_all_year` to `Some(true)` when unanswered; the refusal test must red |
| **M-10** | the four §86 thresholds are not indexed (S-7) | bump any of them by $1; the test that reads both `i1040gi--2024.txt` and `i1040gi--2025.txt` must red |
| **M-11** | 6b prints `-0-` on a STOP branch and is BLANK when no benefits exist (T-2) | make the no-benefits case emit `Some(Usd::ZERO)`; the read-back must show an empty 6b cell |

**Conformance KAT.** The expected line set is enumerated **from the extract**, never from a range or a
hand-list (`CLAUDE.md`, *Blank is the normal case*) — `crates/xtask/src/label_reader.rs` already derives
the label column from the form itself and classifies each row `Amount` / `Heading` / `NonMoney`
(`label_reader.rs:33-42`). Lines 4a–6d must each be **accounted for**: mapped to a field, or recorded
as carrying none **with a reason** (4c, 5c, 6c — §4.2).

**Both oracles, per `CLAUDE.md`.** OpenTaxSolver and Tax-Calculator both compute taxable social
security, so 6b gets a two-witness sweep across all five filing statuses, including the MFS
lived-apart / lived-with pair. ★ Disqualifications are **computed from the mechanism, never listed by
vector name** — the standing lesson from `verify_f6251.py`. 4b and 5b are *collected* figures on both
sides, so oracle agreement there proves nothing (`two-oracle-model` §G-9's limit: *a value the oracle
takes as INPUT is never validated by their agreement*) — those are held by KATs against the
instructions' own branches.

**Journey walk.** Before the plan is frozen, walk one retiree end to end with the owner: an SSA-1099, a
1099-R from a 401(k) with box 2a filled, no IRA. Where does each of the four declarations appear, in
what order, and what does the packet look like when one is skipped?

---

## 11. Open questions for the owner

**OQ-1. Should the `OtherOutOfScopeIncome` prompt be widened NOW, ahead of this feature?** (§2.)
Recommendation: **yes, separately and immediately.** It is a one-line edit in the safe direction, it
turns a silent §61/§86 omission into a refusal today, and it is not coupled to anything in this spec.
Filing it as part of this feature delays the only protection retirees currently lack.

**OQ-2. Does 4b/5b/6b ship for TY2024 as well as TY2025?** The model is year-agnostic and the worksheet
is byte-identical across the two years (S-7, machine-checked). Only the line-9 quote and the AcroForm
map are year-shaped (S-10), and the TY2025 `f1040.map.toml` is currently a stub carrying **only line
7a** (`crates/btctax-forms/forms/2025/f1040.map.toml:1-10`), so the TY2025 cells must be pinned with
`xtask dump-fields` either way. Recommendation: **both years**, since TY2024 is the year btctax can
actually file.

**OQ-3. Is refusing every QCD acceptable?** A qualified charitable distribution is a common, deliberate
act for someone over 70½ and refusing it turns away a filer who did the ordinary thing. The
counterweight is real (§4.2: three caps, an age test, an attachment, an ordering rule, and a Schedule A
double-benefit path) and `no-users-yet` says the cost of refusing today is low. Recommendation:
**refuse in v1**, and file the QCD split as the first widening once the base lands.

**OQ-4. Advisory volume.** A-1 through A-4 mean a plain retiree with one pension and social security
receives up to four advisories on a return where nothing is wrong. Is that the right noise level, or
should A-3 (the §11 SSDI carve-out) be dropped to `btctax limitations` prose? Recommendation: keep all
four for v1 and measure it in the journey walk — advisories are how this project pays for conservative
omissions, and the walk is what tells us when the price is too high.

---

## 12. Risks

**R-A. The blind coverage checker (S-9).** The three most important new rows share one printed
sentence, and today's checker cannot separate them. If the rows land first, the census reports success
over an unverified transcription — F2/F4 exactly.

**R-B. §G-11 is still open and this feature needs both of its halves.** 4a/5a must be blank by
instruction and 6b must print `-0-` by instruction, on the same form. The emitter can decline to write
(`form1040_full.rs:263-317`) so this is buildable, but it must be built as a *distinction*, not as two
zeros.

**R-C. S-6's asymmetry invites exactly one compression.** "Taxable amount = box 2a" is the obvious
helper and it is wrong on the IRA side, where the instructions never mention box 2a. Two comments in
this codebase have carried confident equivalence claims that were false (`return_inputs.rs:48-53`,
`SPEC_schedule_1a.md` §2).

**R-D. S-5's operand set.** `adjustments` is right there and is wrong by one term. The error is
invisible in every household with no student-loan interest, which is most of them.

**R-E. Four new mandatory questions.** Each is justified by a direction-of-error argument (§7), but the
questionnaire is a real product surface and this is the largest single addition to it. The journey walk
(§10) is where that gets measured, not a review round.

---

## 13. Cross-references

- `design/ty2025/SPEC.md` — parent; D-1 … D-11.
- `design/ty2025/SPEC_schedule_1a.md` — house style; S-1 (per-part parameters), F-5 (skip-vs-zero), §5 (test/green shape).
- `FOLLOWUPS.md` §G-11 — the emitter cannot express "no testimony"; this spec must not make it worse and must carry the distinction in its types.
- `FOLLOWUPS.md` §G-22 / B11 — the scope attestation this gap escapes through (§2).
- `crates/xtask/src/line_coverage_check.rs`, `crates/xtask/src/label_reader.rs` — the conformance instruments, one of which needs the S-9 fix first.
