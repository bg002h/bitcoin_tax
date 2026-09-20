# Retirement income — Form 1040 lines 4a–6b — SPEC

**Status: DRAFT r1** (written 2026-09-04, against the archived TY2025 finals). Covers **IRA
distributions (4a/4b)**, **pensions and annuities (5a/5b)** and **social security benefits (6a/6b)** —
the three income lines btctax has no field for at all.

Passes an independent review loop to 0 Critical / 0 Important before an implementation plan.
`design/ty2025/SPEC.md`'s parent decisions (D-1 … D-11) bind here unless restated.

---

## 1. Sourcing of record — READ THIS FIRST

**★★★ CITATION CONVENTION, corrected 2026-09-15. Code is cited by FILE and SYMBOL, never by line
number.** This spec originally carried `file.rs:NNN` citations, and by the time the owner's §11 rulings
were taken **15 of 19** of them had drifted — measured, not estimated, by checking whether the symbol named
beside each citation still appeared within ±8 lines of it. Three verified by hand: `IraDeductionClaimed`
248 → 1433, `label_precedes` 270 → 345, `agi_before_student_loan` 1709 → 1826.

★★ A line number in a prose artifact is a **typed list beside a set that grows** — `CLAUDE.md`'s
highest-yield rule, in the one place the rule is easiest to forget. It was correct on 2026-09-04 and the
code moved underneath it; nothing red, and the spec went on citing confidently. Two of those stale
citations misdirected real work this week (§2's prompt-widening site, and S-5's `adjustments` definition).

★ A symbol citation cannot drift: the reader greps it. Where a line number genuinely carries information a
symbol cannot — a specific expression rather than a definition — quote the expression itself.

★★ **What was actually done — and my first attempt at this note was itself wrong (review r1, I-7).** I
converted the 15 drifted citations and then wrote that *"24 line-number citations remain and were VERIFIED
accurate at HEAD"*. **They were not.** My check only covered citations sitting beside a named symbol (19
pairs), and I stated the result as though it covered all 43. The reviewer re-measured at `main` and found
**13 of 26 wrong** — *including three inside the edits I made the same day*.

★★★ **And two of those three drifted WITHIN HOURS, by my own hand.** `return_1040.rs:2425-2428` and
`:2442`, which I wrote into S-5 that morning after measuring them, had moved to 2467 and 2481 by evening —
because I added two tests to that file later in the same session. A line number is unstable at the
timescale of a single afternoon.

★ So **every** `.rs` line citation in this spec is now gone: 39 converted to file+symbol in total. Where a
specific expression matters, the expression is quoted rather than located. There is nothing left here for
a future edit to invalidate.

★ **The standing instruction, therefore: never add a `.rs` line number to this spec.** Cite the file and
the symbol, or quote the expression. Renumbering a stale citation buys one correct reading and restores
the drift class; converting ends it.

★★ **And a caution about the check itself, since mine misled me.** The script that finds drift — does the
symbol named beside a citation still appear within ±8 lines of it — **can only see citations that have a
symbol beside them.** Reporting its result as coverage of all citations is how I produced a false
"verified" claim. If it is re-run, report what it covered, not what it checked.


There is no separate instruction document for these lines: **`i1040gi` carries them**, in the same file
that carries the Schedule 1-A instructions. Everything below is quoted from the extracted text layer,
never from a rendered page (`CLAUDE.md`, *Transcribe IRS forms*).

| authority | hash | what it holds |
|---|---|---|
| `f1040--2025.pdf` | archived, `design/forms/2025/` | the printed lines 4a–6d and line 9 |
| `i1040gi--2025.pdf` | `482e9c48` (`design/ty2025/SPEC.md:70`) | lines 4a/4b, 5a/5b, 6a–6d, the **Social Security Benefits Worksheet** and the **Simplified Method Worksheet** |
| `f1099r--2024.pdf` | `93900e2b` | **Form 1099-R, all 21 printed box labels** (TY2024 edition) |
| `f1099r--2025.pdf` | `d6b7be48` | Form 1099-R, TY2025 edition |
| `i1099r--2024.pdf` | `ca90e8f0` | the 1099-R box instructions (with Form 5498) |
| `i1099r--2025.pdf` | `a2ebc2c5` | same, TY2025 edition |
| `Pub915…pdf` | `44c4053d` | the **SSA-1099, SSA-1042S and RRB-1099 facsimiles**, every box, plus the negative-box-5 and §1341 repayment rules |
| `Pub575…pdf` | `30bd37cc` | the **Simplified Method** in its own publication |
| `i1040sca--2024.pdf` | archived, `design/forms/2024/` | **Schedule A line 5a** — which documents' state-withholding boxes it takes |

★★★ **I-10 FOLDED 2026-09-20 — the seven rows above the extract line are new, and the first
derivation off them found a money defect.** Review r1's I-10: *"The whole input surface has NO archived
primary source … so the box numbers in §7 are asserted from memory and sit outside cite-check."* That
was exactly right. Not one of Form 1099-R, SSA-1099 or RRB-1099 was in the tree; §7's box list was a
hand-written list of five, graded against nothing.

★★ I-10 also predicted what archiving would surface — *"re-derive §7's box set from the extract and see
what else is missing"* — and the first pass found:

> **Form 1099-R box 14, *State tax withheld*, feeds Schedule A line 5a, and §7 omits it.** The
> authority: *"Forms W-2G, 1099-G, **1099-R**, 1099-MISC, and 1099-NEC may also show state and local
> income taxes withheld"* (`design/forms/extract/i1040sca--2024.txt:324`). An itemizing retiree
> understates line 5a by the whole of their state pension withholding. ★ This is the SAME defect
> interview T11 found and fixed for **W-2 boxes 17 and 19** ([[FR-91]]); nobody carried the reasoning
> across to the 1099-R. And the authority was never missing — `i1040sca--2024.txt` has been committed
> the whole time. It was a source nobody read for this form.

★ §7 must therefore be rewritten against the extract, not patched. The form prints **21** box labels
(1, 2a, 2b, 3, 4, 5, 6, 7, 8, 9a, 9b, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19) and §7 names five. At
least four of the omissions carry decisions: box 3 (capital gain in box 2a), box 5 and box 9b (the
Simplified Method's cost in the plan — ⚠️ **not** "the worksheet this spec builds"; that phrase was wrong here too and is corrected per r2 I-3, since §5 REFUSES the Simplified Method Worksheet. The boxes still matter: they are what a future widening needs), and box 14. Per **blank is
the normal case**, the rewrite's gate is not *"does every box carry a value"* but *"does every box have
a determinate provenance"*.

★★ **Box 6 means three different things across this family, so no rule may say "box 6" without naming
its form.** SSA-1099 box 6 is *Voluntary Federal Income Tax Withheld*; SSA-1042S box 6 is *Rate of Tax*
and its withholding is **box 7**; RRB-1099 box 6 is *Workers' Compensation Offset* and its withholding
is **box 10**. C-2's "SSA-1099 box 6 and RRB-1099 box 10" is correct — and was correct for a reason the
spec could not previously check.

★ **The 1099-R is archived but NOT censused, deliberately.** `xtask box-census` covers documents the
product *models* (`DocumentKind::ALL`); the 1099-R has no `DocumentKind` because this feature is not
built. Adding one is now a **build error** in `box_census::modelled_stems`, which is what will demand
the box population at the moment the build starts. Separately, the census enumerator cannot yet read
this form — its contiguity guard reports *"[11] missing from 1..=19 … this is the READER dropping
boxes"*, because the 1099-R packs boxes 10–16 onto one printed row. That reader fix belongs to the
build.

Extracts of record: `design/forms/extract/f1040--2025.txt` and
`design/forms/extract/i1040gi--2025.txt`. Every quote below carries its extract line number, and every
quote is machine-checkable by `cargo run -p xtask -- line-coverage-check`
(`crates/xtask/src/line_coverage_check.rs`).

**Nothing here needs an oracle to establish.** Both engines compute taxable social security, and both
should be asked (`CLAUDE.md`, *Two oracles*) — but §5's worksheet is the authority and an oracle is a
witness.

---

## 2. The gap, measured

**There is no field.** `Form1040Lines` (`crates/btctax-core/src/tax/printed.rs`) declares
`line1z, line1a, line2a, line2b, line3a, line3b, line7, line8, line9, …` and stops. So does
`Form1040Income` (`printed.rs`). Total income is composed at
`crates/btctax-core/src/tax/return_1040.rs`:

```rust
let total_income =
    wages + taxable_interest + ordinary_dividends + capital_gain + schedule_1_income; // L9
```

**And the census already says so, in the form's own words.** The line-9 coverage row
(`crates/btctax-core/src/tax/line_coverage.rs`) carries the instruction
*"Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7, and 8. This is your total income"* under
`Production::Combine`, whose contract is *"Blank iff every operand is blank"*
(`line_coverage.rs`). Three of the eight named operands have no field. The transcription is
verbatim and correct; the struct behind it is missing three summands.

**★★ The catch-all does not catch a retiree, and the registry says why.** The only thing between a
pensioner and a filed return that omits their pension is `other_out_of_scope_income`
(`crates/btctax-core/src/tax/return_refuse.rs`). Its prompt
(`crates/btctax-core/src/tax/questions.rs`) enumerates *"rent or royalties, a farm, a
partnership, S corporation, estate or trust (any Schedule K-1), unreported tips, gambling winnings,
alimony, a business this tool did not capture, or anything else it never asked about."* It never says
**pension**, **IRA** or **Social Security** — and the same registry entry states the rule that makes
this fatal, twelve lines below:

> *"a filer cannot answer `no` to a category they were never shown."* — `questions.rs`

A retiree reading that list truthfully answers **No** (they have no rent, no farm, no K-1) and files a
return omitting §61 and §86 income under §6065. That is
`widening-an-exemption-is-never-the-safe-edit` in its purest form: a residual clause carrying weight
that only an enumerated YES-condition can carry.

**★ The cheapest half of the fix does not need this spec.** Adding *"a pension or annuity, an IRA or
retirement-plan distribution, or social security or railroad retirement benefits"* to that prompt's
limb (a) is a one-line, whole-surface improvement in the SAFE direction (widening a mandatory
question's YES-conditions — `questions.rs`) and should land whether or not this spec is built.
It converts a silent omission into a refusal. **See §11, OQ-1.**

---

## 3. Binding decisions

**S-1. REFUSE where an unanswered branch could UNDERSTATE; ADVISE where it can only OVERSTATE.**
This is the classifier for every branch in §4, so the refusal list is *derived* rather than chosen. It
is the repo's existing rule stated as a decision procedure: a conservative omission is permitted only
if the filer is told (`return_refuse.rs`, the `SingleEmployerExcessSs` retraction), and an
unasked question that can understate refuses (`HsaActivityUnanswered`, `OtherIncomeUnanswered`,
`SstbUnanswered`).

**S-2. LINES 4a AND 5a ARE `Option<Usd>`, BECAUSE THE FORM MANDATES A BLANK THERE.** Not a style
choice — an instruction:

> *"If the distribution from your IRA is fully taxable, enter the total distribution on line 4b;
> **don't make an entry on line 4a**."* — `i1040gi--2025.txt:2664-2667`

> *"If your pension or annuity is fully taxable, enter the total pension or annuity payments (from
> Form(s) 1099-R, box 1) on line 5b; **don't make an entry on line 5a**."* — `i1040gi--2025.txt:2876-2880`

**⚠️ I-2 FOLDED 2026-09-20 — "STRUCTURALLY NEVER POPULATED" WAS WRONG, AND IT IS ADJUDICATED AGAINST
THE FORM HERE. The retracted sentence is kept visible because an M-3 mutation was about to pin it with
a KAT.**

The retracted claim: *"In v1 the fully-taxable branch is the **only** branch that computes (§4), so
**line 4a is structurally never populated**, and line 5a is populated only when the 1099-R shows a
smaller box 2a."*

The quoted instruction above is real but **conditional on a single distribution**, and it is not the
paragraph that governs the plural case. Four passages in the same instructions contradict the
conclusion — the first three are I-2's, the fourth is stronger and I-2 did not reach it:

1. **The IRA plural rule, unscoped** (`i1040gi--2025.txt:2789-2795`; TY2024 `:2726-2732`, verified
   identical):
   > *"**More than one distribution.** If you (or your spouse if filing jointly) received more than one
   > distribution, figure the taxable amount of each distribution and enter the total of the taxable
   > amounts on line 4b. **Enter the total amount of those distributions on line 4a.**"*

   §4.1 defines 4b as *"Σ Form 1099-R box 1 over IRA-flagged documents"* — a Σ, so two IRA documents are
   squarely in v1 scope, and this is the paragraph that governs them. ★ The spec's own cited IRA range
   was `2664-2790`; the paragraph begins at 2789 and was read past by one line.

2. **The asymmetry is the tell, and it is the IRS's own.** The pension analogue is *scoped* —
   *"More than one **partially taxable** pension or annuity"* (`:2978-2980`) — while the IRA version
   carries no such limb. Two adjacent rules, one qualified and one not, is a deliberate difference, not
   an oversight to be read away.

3. **Exception 2's LEAD sentence** instructs *"enter the total distribution on line 4a"* (`:2698`) and
   sub-branch **(b)** then substitutes `-0-` on 4b for a distribution-code-Q Roth IRA (`:2714`). ★ r2's
   M-2: the first version of this item attributed the 4a phrase to the sub-branch, which does not carry
   it — the lead does, and that distinction is exactly what C-1 turned on — a populated 4a on a branch the instructions close in one
   sentence. See the I-3 fold.

4. **⚠️ REPLACED 2026-09-20 by fold-review r2 (I-3) — MY ARGUMENT HERE WAS BUILT ON A WORKSHEET THIS
   SPEC REFUSES.** The retracted text: *"★★★ Line 5a is populated unconditionally whenever the
   Simplified Method runs, by that worksheet's own line 1 … **That is the worksheet this spec
   builds**."* The quote is verbatim (`:2981-2982` ✓) but the clause after it is **false**, and §5 says
   so in its own words: *"The Simplified Method Worksheet is the opposite call and is REFUSED"*
   (R-3/R-5). The worksheet this spec builds is the **Social Security Benefits Worksheet** (S-4). ★ I
   made the identical false claim in the I-10 fold at §1, and both are corrected.

   **The paragraph that actually governs line 5a sits 90 lines earlier and I had not cited it:**

   > *"**Partially Taxable Pensions and Annuities.** Enter the total pension or annuity payments (from
   > Form 1099-R, box 1) on line 5a. If your Form 1099-R doesn't show the taxable amount, you must use
   > the General Rule explained in Pub. 939 to figure the taxable part to enter on line 5b."*
   > — `i1040gi--2025.txt:2888-2891`

   ★★ This is **better authority than the one I used**, and it settles v1 cleanly: for a partially
   taxable pension 5a = Σ box 1 **unconditionally**, and the worksheet/General Rule is reached only when
   box 2a is BLANK. In v1 §4.1 makes 5b = Σ box 2a — box 2a is shown — so the in-scope partially-taxable
   case needs **no worksheet at all**: 5a = Σ box 1, 5b = Σ box 2a.

   ★ So the ORIGINAL S-2 sentence — *"5a is populated only when the 1099-R shows a smaller box 2a"* —
   was substantially **right**, and my retraction of that half overreached. What was wrong was calling
   it a consequence of a comparison rather than the partially-taxable branch's own instruction. The 4a
   half of the retraction stands unaffected.

**THE ADJUDICATION.** Both cells are conditionally populated, and the conditions come from the form:

| line | populated when | authority |
|---|---|---|
| **4a** | **more than one** IRA distribution ⇒ Σ box 1 over IRA documents; **or** a code-Q / code-T Roth sub-branch | `:2789-2795`; `:2707-2715` |
| **4a** | left blank on a **single** fully-taxable IRA distribution | `:2664-2667` |
| **5a** | **any partially taxable pension** — i.e. box 2a is shown and is less than box 1, which is the v1 case | *"**Partially Taxable Pensions and Annuities.** Enter the total pension or annuity payments (from Form 1099-R, box 1) on line 5a"* — `:2888-2891` |
| **5a** | more than one *partially taxable* pension ⇒ Σ box 1 over them | `:2978-2980` |
| **5a** | left blank on a **single** fully-taxable pension | `:2876-2880` |

★★ **`Option<Usd>` is therefore still the right type — for the opposite reason.** S-2's original
argument was *"always `None`, so the type records an instructed blank"*. The real reason is that the
cell is **genuinely two-valued**: blank on one branch and a figure on another, which is precisely what
`Option` exists to carry and what a bare `Usd` would flatten to a fabricated `0`. The type survives the
retraction; the justification for it does not, and [[an-entry-is-testimony]] is why that matters — a
`0` on line 4a would be sworn testimony that the filer received no IRA distributions.

★★★ **M-3 must be rewritten, and this is why I-2 is Important rather than Minor.** As written it
mutates `line4a: Option<Usd>` to `Usd` and requires the read-back to show the 4a cell **present** —
which pins the wrong reading with a passing test. It becomes: *a single-distribution return leaves 4a
blank; a two-document return populates it with Σ box 1.* Two KATs, and the second one reds today.

★ And the understatement direction is the dangerous one: a blank 4a beside a populated 4b leaves the
filed return's operand column empty **on the line the Service document-matches against the payer's
1099-R box 1 totals**. That is the identical shape as the line-1a finding already recorded in
`printed.rs` — *"Its absence left the filed 1z sitting above an EMPTY operand column"*.

Those are not forgotten lines — they are lines whose provenance is *"the form instructs a blank here,
on this branch"*, which is exactly the distinction `CLAUDE.md`'s provenance table draws and which
`FOLLOWUPS.md` §G-11 says must be carried in the types. Precedent for a conditional money cell:
`ScheduleALines.line2: Option<Usd>` (`printed.rs`). Precedent for the emitter declining to
write: lines 34/35a/37 in `crates/btctax-forms/src/form1040_full.rs`, recorded at
`FOLLOWUPS.md:1747`.

**S-3. ONE class-(A) declaration per document, enumerating the YES-conditions FROM THE FORM'S OWN
EXCEPTION LIST — not one declaration per exception.** The instructions already enumerate them
(IRA: four numbered *Exceptions*, `i1040gi--2025.txt:2682/2698/2727/2758`; pension: fully-taxable vs
partially-taxable vs PSO vs line-1h vs rollover, `2846-2906`). The question is *"does any of these
apply?"*, `None` refuses unanswered, `Some(true)` refuses unsupported, `Some(false)` computes. Model:
`ReturnInputs::hsa_activity` (`return_inputs.rs`) and
`ReturnInputs::has_income_exclusion` (`return_inputs.rs`).

**S-4. THE SOCIAL SECURITY BENEFITS WORKSHEET IS IN SCOPE AND TRANSCRIBED IN FULL — 18 lines.**
Justified in §5. It is the whole of 6b, it is self-contained, it needs no other form, and refusing it
would refuse the single most common retirement return in the United States.

**S-5. WORKSHEET LINE 6 IS NOT `AbsoluteReturn::adjustments`, AND IT IS A BLOCK RATHER THAN A LIST.**
The worksheet says *"Schedule 1, lines 11 through 20, and 23 and 25"* (`i1040gi--2025.txt:3403`) — which
**excludes Schedule 1 line 21**, the student-loan interest deduction (`f1040s1--2024.txt:72`), and line 22
(*"Reserved for future use"*, `:73`). Taking `adjustments` whole understates provisional income, which
understates taxable benefits, which understates tax.

**★★★ CORRECTED 2026-09-15 (FR-183). This section previously prescribed *"printed Sch 1 L15 + printed
L18"*, and that omits line 13.** The block *"11 through 20"* **contains line 13**, the §223 health savings
account deduction, which T16 added after this section was written. Measured at HEAD rather than cited from
memory:

| | |
|---|---|
| `Schedule1Lines` adjustment fields | **L13** (HSA), **L15** (half SE), **L18** (early withdrawal), **L21** (student loan) — `printed.rs` |
| `Schedule1Lines.line26` | documented as *"`13 + 15 + 18 + 21` here"* — the spec's old citation of *"`15 + 18 + 21`"* is stale |
| `AbsoluteReturn::adjustments` | `early_wd + half_se + student_loan + hsa_deduction_13` (`return_1040.rs`) — the spec's old citation omitted the HSA term too |

**So worksheet line 6 is `printed Sch 1 L13 + L15 + L18`** — the block minus line 21 —

**⚠️ INCOMPLETE, corrected by review r1 (I-9).** The block is *"lines 11 through 20, **and 23 and
25**"* and I stopped at 20. Lines **23** (Archer MSA deduction) and **25** (other adjustments) are in
it. They are latent today — `Schedule1Lines` carries no field for either — but **the rule is the
deliverable, not the current operand list**, which is the whole point of stating it as a block. T14.3
must derive line 6 from the block's full membership, and a field added later for line 23 or 25 belongs
there the day it is added. ★ OTS corroborates independently: its own comment describes the worksheet's
input as *"L6a and Sched1[11-25]"*.

Reading on with the operands that exist today — transcribed as
operands, never as `line26 − line21`.

**★★ AND THE RULE, because a list will go stale again the next time Schedule 1 grows.** The operand set is
*whatever btctax populates inside the block the worksheet names*, not a list of three. The identical rule
is already written in the code for the §221 MAGI, one function away, with its own direction-of-error
argument (`return_1040.rs`):

> *"…adding it to `adjustments` and not here inflated the MAGI and OVERSTATED the tax. Held by
> `form8889::tests::the_hsa_deduction_is_inside_the_section_221_magi`. ★ It is a BLOCK, not a list: a
> future Schedule 1 lines 11–20 adjustment belongs here the day it is added, and the worksheet's own
> sentence is the rule that says so."*

Nobody carried that sentence to this spec, which is `design/HARNESS.md`'s **B3** shape exactly — the fix
existing in the tree, reasoned, with no reviewer holding both sites at once. **T14.3 must derive line 6
from the block and carry a test that reds when a block member is dropped**, or this recurs on the next
adjustment. ★ Direction, stated so the test can assert it: omitting a block member *shrinks* line 6,
*inflates* provisional income, and **overstates** the tax — the filer overpays.

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
`RefuseReason::IraDeductionClaimed` (`return_refuse.rs`, fired at `return_refuse.rs`), so
the circular branch cannot arise. **That is a guarantee held in another module**, and a future
relaxation of that refusal would silently make this worksheet the wrong instrument — so it lands with
a KAT that reds when the coupling is broken (§10, M-8).

**S-9. ★★ THE COVERAGE CHECKER STILL CANNOT TELL 4b FROM 5b FROM 6b — FR-184 BOUGHT THE CARDINALITY
GUARANTEE, NOT THE ATTRIBUTION ONE. The binding exists in the GEOMETRY; see the I-4 fold below.** All three lines print the identical two words — *"Taxable amount"*
(`f1040--2025.txt:74,76,78`). `label_precedes` (`line_coverage_check.rs`) accepts the bare
sub-letter form `b` and then only requires the stem digit to appear within the preceding ~700
characters; lines 4a–6b sit inside one 700-character window, so **a row labelled `4b` quoting
*"Taxable amount"* matches at line 5b's position and passes.** That is the exact class r7 measured at
71 accepted misattributions (`line_coverage_check.rs`), and it is Form 6251 line 33 again.

**⚠️ I-4 FOLDED 2026-09-20 — THE PARAGRAPH BELOW IS RETRACTED ON TWO COUNTS. Kept visible because it
prescribed a fix that had already been measured and refuted, and because M-1 as written could not red.**

The retracted text: *"The fix that fits the existing design: anchor the match to the physical extract
row whose first token is the stem label — the 4a/4b row is `f1040--2025.txt:74`, 5a/5b is `:76`, 6a/6b
is `:78`, one row each. … **This is a prerequisite of the feature, not a follow-up**."*

**(1) What actually landed is weaker than S-9 asked for, and FR-184 says so in its own words.** FR-184
closed at `main` (`dd8f2e984`) with a **cardinality pigeonhole**: a shared quote must be backed by at
least as many printed occurrences as there are distinct lines claiming it. `line_coverage_check.rs`
states the residue at the rule itself:

> *"A table row saying `f1040:4b` passes, and would pass IDENTICALLY if it said `6b`. … That does not by
> itself pin which row owns which occurrence."*

So **M-1's mutation cannot red.** After swapping the 4b and 6b labels the distinct-line set is still
`{4b, 5b, 6b}` = 3 and *"Taxable amount"* is still printed three times, so the pigeonhole is satisfied
and rule (2b) admits each row individually. A mutation that passes is not a kill, and
[[a-kill-can-certify-the-blindness]] is what that costs: the spec would have shipped the claim *"the
census can tell 4b from 5b from 6b"* with a green test standing under it.

**(2) S-9's prescribed remedy was one of the three FR-184 measured and REFUTED** — window tightening
(it left gaps of 89 and 79 characters), nearest-label (it broke `f8889:17b`, whose run-up ends *"…
included on line 16 … b"*), and trailing amount-box (it broke `f8995` lines 2–8). Prescribing it again
would have spent a build round rediscovering that.

**★★★ AND THE BINDING DOES EXIST — it is just not in the text layer, which is why three text-layer
candidates all failed. `label_reader::label_join` already answers it from the GEOMETRY.** It returns
*AcroForm field → the printed line label that governs it*, x-aware within the field's own row. Run
against `f1040--2024`, it resolves exactly the three lines this section is about, and the three beside
them:

| field | label `label_join` found |
|---|---|
| `…f1_46[0]` | **4a** |
| `…f1_47[0]` | **4b** |
| `…f1_48[0]` | **5a** |
| `…f1_49[0]` | **5b** |
| `…f1_50[0]` | **6a** |
| `…f1_51[0]` | **6b** |

**⚠️ CORRECTED 2026-09-20 by fold-review r2 (I-4) — the two are NOT in the same key space, so the
claim below was wrong and M-1b as first written was circular.** The retracted text: *"Coverage rows are
per field … so the row already carries the key `label_join` is indexed by. Binding `row.line` to
`label_join[row.field]` is therefore a lookup, not a heuristic."*

The structs say otherwise, and r2 quoted both:

| side | its key | evidence |
|---|---|---|
| `LineCoverage.field` | the **Rust** field name, `"line16"` | `line_coverage.rs`: *"The Rust field, e.g. `"line16"`."* |
| `label_join` | the **AcroForm** field name, `topmostSubform[0]…f1_47[0]` | `label_reader.rs`: *"for every AcroForm box, the printed line label that governs it"* |

**The bridge between them is the `f1040.map.toml` row — which this fold's own I-8 proves is absent for
all six lines** (`grep -cE '^line(4a|4b|5a|5b|6a|6b) *='` returns 0 on both years' maps). So M-1b
depended on precisely the thing I had just measured as missing: circular.

★★ **It is still buildable, and the ordering is now explicit.** The binding is a **two-hop join gated on
the map rows the build must add anyway** (I-8's 6 + 6 cells):

> `LineCoverage.field` → its `f1040.map.toml` row → the AcroForm name → `label_join` → the printed
> label, compared against `LineCoverage.line`.

★ Each hop is already a committed artefact and none is a heuristic. But the join **cannot be written
before the map cells exist**, so M-1b is scheduled after them rather than beside them — and if the map
work slips, S-9 must say in the spec that the three rows' mutual attribution is not machine-held and
name the KAT standing in.

★ Two independent derivations agree that these six fields are 4a–6b: this geometry label join, and the
AcroForm numbering plus column coordinates measured under I-8 (`f1_45` = 3b, `f1_52` = 7, subline column
x≈252, amount column x≈504). Neither used the other's evidence.

**THE REVISED POSITION.** The pigeonhole is a real guarantee and it is the one that landed: 4b, 5b and
6b may coexist **because the form really does print those words three times**, which is what stops a
row inventing a line the form never prints. Row-to-line attribution is a *separate* guarantee, it is
**not** held today, and it is available from the geometry join rather than blocked. So this is **no
longer framed as a prerequisite that gates the rows** — it is a build task with a known implementation
and a kill that can actually fail (M-1 and M-1b in §10).

**⚠️⚠️⚠️ S-7/S-10 RETRACTION — review r1, C-1 (CRITICAL). THE WORKSHEET IS *NOT* BYTE-IDENTICAL ACROSS
THE TWO YEARS, AND TY2024 FORM 1040 HAS NO LINE 6d AT ALL.** Controller-verified against the archived
extracts, not accepted on report:

| check | result |
|---|---|
| `f1040--2024.txt` contains `6d` | **0 hits** |
| `f1040--2024.txt` contains *"lived apart"* | **0 hits** |
| `f1040--2025.txt` contains `6d` | 1 hit |
| section heading, `i1040gi--2024.txt:3117` | *"Lines 6a, 6b, and 6c"* |
| section heading, `i1040gi--2025.txt:3233` | *"Lines 6a, 6b, 6c, and 6d"* |
| worksheet line 3, `i1040gi--2024.txt:3370` | *"…lines 1z, 2b, 3b, 4b, 5b, **7**, and 8"* |
| worksheet line 3, `i1040gi--2025.txt:3400` | *"…lines 1z, 2b, 3b, 4b, 5b, **7a**, and 8"* |
| the MFS disclosure, `i1040gi--2024.txt:3416` | *"be sure you entered **“D” to the right of the word** benefits on line 6a"* |

★★ **What S-7 actually machine-checked was the four §86 thresholds, and only that part survives** —
$32,000 / $25,000 / $12,000 / $9,000 do print identically in both years' worksheet blocks. Generalising a
four-constant check into *"the worksheet is byte-identical"* is the whole defect, and it is the
`CLAUDE.md` failure of describing code from a claim about it rather than from the thing. ★ It then
propagated: the review brief repeated it as **settled** and told the reviewer not to spend budget there.
The reviewer found the Critical anyway, by diffing the blocks.

★★★ **So a worksheet transcription CANNOT be year-agnostic.** Two of its own printed sentences differ (ws3
and ws9), and per `CLAUDE.md` a transcription struct is **per line-set revision** — this worksheet has two.
The consequences for the build, all of which T14 must carry:

1. **The TY2024 MFS-lived-apart disclosure is a written character, not a checkbox.** `line6d_checked: bool`
   has no TY2024 AcroForm cell to write to, so the TY2024 path cannot be built as this spec specifies.
   TY2024 needs an annotation leaf for the **`D`** beside the word *benefits* on line 6a.
2. **The consequence of omitting it is named by the IRS itself** and belongs in R-8's wording: *"If you
   don't, you may get a math error notice from the IRS."* The filer gets the right figure on the page and
   an adjustment in the post.
3. **§5.1 must transcribe ws3 and ws9 PER YEAR**, because a doc comment that quotes one year's sentence is
   wrong for the other.
4. **§10's conformance KAT must derive its line set PER YEAR from the extract.** *"Lines 4a–6d must each be
   accounted for"* is a hand-typed range that over-enumerates TY2024 by one line — the exact shape
   *Blank is the normal case* forbids (a `1..=38` range against a 48-label set).
5. **Confirm, never assume, that the TY2024 AcroForm can carry a character next to line 6a**, recording the
   field name from `xtask dump-fields`.

**S-10. THE MODEL IS YEAR-AGNOSTIC; THE COVERAGE ROWS AND THE ACROFORM MAP ARE YEAR-KEYED.** Lines
4a–6b exist in every year and the worksheet is unchanged (S-7). Only two things are year-shaped: the
line-9 operand list, which reads *"…6b, **7**, and 8"* in `f1040--2024.txt:75` and *"…6b, **7a**, and
8"* in `f1040--2025.txt:84`; and the field map. `Coverage` already carries a per-row `year`
(`line_coverage.rs`) with Form 6251 line 1 as precedent (`line_coverage.rs`).

---

## 4. Scope and non-scope

### 4.1 In scope

| line | v1 behaviour |
|---|---|
| **4a** | `Option<Usd>` — ⚠️ **CORRECTED 2026-09-20 (r2 I-2): NOT "always `None`".** `Some(Σ box 1 over IRA documents)` when more than one IRA document exists or any is on the Roth `-0-` sub-branch; `None` on a single fully-taxable document. See S-2's adjudication and the C-1 rule table (§8) |
| **4b** | Σ Form 1099-R box 1 over IRA-flagged documents, when the filer declares no Exception applies |
| **5a** | `Option<Usd>` — `Some(Σ box 1)` when box 2a < box 1 (the partially-taxable branch); `None` when the pension is fully taxable. ★ Authority is `i1040gi--2025.txt:2888-2891`, *not* the Simplified Method Worksheet, which this spec REFUSES (r2 I-3) |
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
   ahead of `agi_before_student_loan` at `return_1040.rs` without moving anything.

**The Simplified Method Worksheet is the opposite call and is REFUSED** (R-3/R-5). It is fully
transcribed in the same document (`i1040gi--2025.txt:2973-3036`, with Table 1 at `3037` and Table 2 at
`3061`) and is the natural next increment — but its line 6, *"the amount recovered tax free in
years after 1986 … enter the amount from line 10 of last year's worksheet"*, is a multi-year
carryforward, which is a feature with a persistence surface and a provenance flag (cf.
`QbiInputs::qbi_carryforward_in_provenance`, `return_inputs.rs`), not a worksheet.

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
because no clamp-free production in the grammar (`line_coverage.rs`) expresses an instructed zero
that is also legitimately blank. That costs one unit of the exception ratchet, knowingly.

---

## 6. The 1040 lines, transcribed

Quotes are the form's own printed text (`f1040--2025.txt`), as the census requires.

| line | printed text | extract | field | production |
|---|---|---|---|---|
| 4a | "IRA distributions" | `:74` | `line4a: Option<Usd>` | `Collected` — ⚠️ **CORRECTED 2026-09-20 (r2 I-2)**: conditionally populated, per S-2's adjudication table and §8's C-1 rule. The previous text said *"always `None` in v1"* and cited *(S-2)* — the section that had retracted it |
| 4b | "Taxable amount" | `:74` | `line4b: Usd` | `Collected` |
| 4c | "Check if (see instructions) 1 Rollover 2 QCD 3" | `:75` | *(no field)* | never checked — every branch that would check it refuses (§4.2) |
| 5a | "Pensions and annuities" | `:76` | `line5a: Option<Usd>` | `Collected` |
| 5b | "Taxable amount" | `:76` | `line5b: Usd` | `Collected` |
| 5c | "Check if (see instructions) 1 Rollover 2 PSO 3" | `:77` | *(no field)* | never checked — rollover refuses (R-3), PSO is advisory (A-1) |
| 6a | "Social security benefits" | `:78` | `line6a: Option<Usd>` | `Carry` — worksheet line 1 |
| 6b | "Taxable amount" | `:78` | `line6b: Option<Usd>` | `Exception`, reason per T-2 |
| 6c | "If you elect to use the lump-sum election method, check here (see instructions)" | `:79` | *(no field)* | never checked — A-2 |
| 6d **(TY2025 ONLY — see the S-7/S-10 retraction; TY2024 has no line 6d, and its disclosure is a written `D` on line 6a)** | "If you are married filing separately and lived apart from your spouse the entire year (see inst.), check here" | `:80` | `line6d_checked: bool` | `Collected` — from the R-8 declaration |
| 9 | "Add lines 1z, 2b, 3b, 4b, 5b, 6b, 7a, and 8. This is your total income" | `:84` | `line9` | `Combine` — **gains three operands** |

★ ⚠️ **CORRECTED 2026-09-20 (r2 M-5).** This note used to say the quotes *"must land **after** S-9's
checker fix, or the three `"Taxable amount"` rows are unverified by construction"* — the gating framing
the I-4 fold retracted in S-9 and M-1 but did not carry here, and §6's closing note is r1 I-4's own
third named section. What holds today: FR-184's pigeonhole admits the three rows **because the form
prints those words three times**, and row-to-line attribution is a separate guarantee scheduled after
I-8's map cells (§10 M-1b). The rows are not blocked; their mutual attribution is simply not
machine-held yet, and M-1b is the kill that will hold it.

---

## 7. The input surface — what must be COLLECTED

New leaves on `ReturnInputs`, in the house shape (`return_inputs.rs` is the model for a typed
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
    /// ★★ Box 7 — the distribution code(s), captured verbatim. **INTERPRETED, for exactly two tokens:**
    /// `Q` and `T` route to the Roth qualified sub-branch the instructions close themselves (4a = Σ box
    /// 1, 4b = `-0-`, no Form 8606) — see the I-3 fold at R-2. Everything else falls through to R-2.
    /// The previous doc comment said *"screened, never interpreted"*, which was wrong: the
    /// instructions' own decision procedure is a box-7 test, so the field that decides was held and
    /// not read. ★★ The test is **EQUALITY, not membership**: the sub-branch is taken only when box 7
    /// is exactly `"Q"` or exactly `"T"`. Table 1's *used with* column for `Q` reads `None`
    /// (`i1099r--2025.txt:2416`), and the Note inside both entries — *"If any other code, such as 8 or
    /// P, applies, use Code J"* — instructs the payer to print `J` INSTEAD, so a composed code carrying
    /// `Q` is a document this spec does not understand and R-2 refuses it. `contains("Q")` would admit
    /// it, which understates tax (r2 I-5). ★ Every Table 1 code is one character, so there is no
    /// separator to parse.
    pub box7_distribution_codes: String,
    /// ★★★ **Box 14 — "State tax withheld". → SCHEDULE A LINE 5a.** Added 2026-09-20 folding r2's I-8:
    /// the I-10 fold DISCOVERED this money defect and then filed it nowhere — no field, no refusal, no
    /// mutation, no follow-up — while the two non-money findings from the same fold both got FR numbers.
    ///
    /// The authority, already committed in this tree the whole time:
    /// *"Forms W-2G, 1099-G, **1099-R**, 1099-MISC, and 1099-NEC may also show state and local income
    /// taxes withheld"* — `design/forms/extract/i1040sca--2024.txt:324` (identical at `--2025:218`).
    ///
    /// ★★ An itemizing retiree otherwise understates Schedule A line 5a by the whole of their state
    /// pension withholding. This is the SAME defect interview T11 found and fixed for **W-2 boxes 17
    /// and 19** (FR-91); nobody carried the reasoning across to this document. `Option<Usd>` because
    /// most 1099-Rs carry no state withholding and a hardcoded `0` would be sworn testimony that none
    /// was withheld.
    ///
    /// ★ **Boxes 15-19 are NOT collected, and that is a stated boundary rather than an omission.**
    /// Box 15 (state/payer's state no.) and 18 (name of locality) are identifiers no federal line
    /// reads; boxes 16 and 19 are state and local DISTRIBUTION amounts, which no federal line reads
    /// either. Box 17 (**local** tax withheld) also feeds Schedule A line 5a by the same instruction
    /// and is deliberately deferred with the QCD widening — recorded in FOLLOWUPS as part of the same
    /// entry so it cannot be lost, because deferring it silently is what produced this finding.
    pub box14_state_tax_withheld: Option<Usd>,
    /// ★★ The SEVENTH question (r2 I-1). Asked **only** when `box7_distribution_codes == "T"` on an IRA
    /// document. `None` REFUSES (`RothLookbackUnanswered`) — unlike A-1/A-2, silence here would admit
    /// the filer to the `-0-` branch and understate tax, so it is a class-(A) declaration.
    /// The `<YEAR>` in the prompt is read from that revision's own instructions and NEVER computed:
    /// `i1040gi--2025.txt:2713` says 2020 and `i1040gi--2024.txt:2647` says 2018 — a two-year jump
    /// across a one-year revision, so no offset formula is correct.
    pub roth_contribution_before_lookback: Option<bool>,
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
    /// Box 4 — "the amount of any benefits you repaid in 2025" (:3237-3239). ★★ R-6 fires on
    /// **Σ box 4 > Σ box 3 across ALL Forms SSA-1099/RRB-1099 of BOTH spouses**, never per form (r2 M-3).
    /// Pub 915: *"If the total amount shown in box 5 of all of your Forms SSA-1099 and RRB-1099 is a
    /// negative figure…"*, and its Ryan/Jordan example nets $3,000 against ($500) to $2,500. A per-form
    /// test would refuse a return the instructions finish in one sentence.
    pub box4_benefits_repaid: Usd,
    /// ★★★ Box 5 is **NOT a field.** It is DERIVED — see the I-11 fold directly below.
}

/// Box 5 — NET benefits, the figure worksheet line 1 reads (:3396). Never collected: the form's own
/// caption defines it, *"Box 5. Net Benefits for 2025 (Box 3 minus Box 4)"*, and it is NEGATIVE when
/// box 4 exceeds box 3 (printed in parentheses). `Usd` is `Decimal` and holds that; no new type.
#[must_use]
pub fn box5_net_benefits(f: &FormSsa1099) -> Usd {
    f.box3_benefits_paid - f.box4_benefits_repaid
}
```

★★★ **I-11 FOLDED 2026-09-20 — box 5 is DERIVED, not collected. But I-11's stated premise is FALSE,
and the true mechanism is worse.**

Review r1's I-11, verbatim: *"`Usd` cannot express a negative SSA-1099 box 5, so the R-6 population
cannot enter what their form says."* **`Usd` can.** It is `pub type Usd = Decimal`
(`btctax-core/src/conventions.rs`), and `rust_decimal::Decimal` is signed — verified empirically, not
inferred from the alias: a probe computing `dec!(3000) - dec!(3500)` yields `-500` with
`is_sign_negative()` true. The finding cited `FOLLOWUPS.md` §G-11, which records that `Usd` cannot
express **blank** — true, and a different property from sign.

★★ **The defect is real, and it is that the two input paths disagree about the exact notation SSA
prints.**

| path | negative money | the SSA notation `(500.00)` |
|---|---|---|
| `btctax-adapters/src/parse.rs` (CSV/exports) | accepted | **parsed** — *"parenthesized accounting negative `(1.23)`"*, with a test at `:210` |
| `btctax-input-form/src/parse.rs` (the interview, which a retiree uses) | **rejected at the door** (`:42`, `negative → Negative`) | rejected |

So the filer transcribing an SSA-1099 into the interview cannot enter what their document prints —
while the adapter path has parsed that exact notation all along. ★ And
`btctax-input-form/src/attribute.rs:333` carries a defensive comment reading *"a negative amount is
unreachable from the form: tier-1 parse rejects it"* — true today, and it becomes false the moment any
SSA-1099 figure arrives through the adapter.

★★★ **I-11's second half is the one that decides the design, and it is entirely right.** Collecting
boxes 3, 4 and 5 as three independent leaves permits a **triple that contradicts itself**, because on
the form box 5 is not independent — the facsimile prints its definition in its own caption: *"Box 5.
Net Benefits for 2025 (Box 3 minus Box 4)"* (the `--2025` edition; the `--2024` edition prints "2024"). So the fix is not a signed field beside two unsigned
ones; it is to stop collecting a box the form computes. Deriving also moots the input-path split above
for this spec, though the split itself remains and is filed as [[FR-256]].

Pub 915 states the negative case in its own words, on **both** forms — quoted here in full, because the
first version of this fold spliced it and left it starting mid-sentence (r2 M-1):

> *"If parentheses are around the figure in box 5, it means that the figure in box 4 is larger than the
> figure in box 3. This is a negative figure and means you repaid more money than you received."*
> — `Pub915_Social_Security_and_RRB_Benefits--2025.txt`, the SSA-1099 discussion and again for RRB-1099

★ **The edition is now named in the citation, and r2's I-7 is why.** This fold originally quoted the box-5
caption as *"Net Benefits for **2024**"* three times while citing an archive that holds the **2025**
edition and prints "2025" — a string that appeared **zero** times in the file cited. Both editions are
archived now (`--2024` and `--2025`) and every quote names one.


★★ **This is transcription, not derivation-in-the-forbidden-sense.** The standing rule bans a closed
form that replaces the document's own steps. Here the document's own step *is* the subtraction, printed
in the box's caption, so applying it is following instructions. The filer is never asked to type a
parenthesised figure, and no typed triple can disagree with itself.

★ **The boundary, stated rather than hidden.** If a payer's form ever printed a box 5 that is not box 3
minus box 4, deriving would silently overwrite it. That is accepted: the alternative — collecting it to
cross-check — cannot be built, because the very case where a discrepancy would matter is the negative
one the filer cannot enter. The exposure is a payer arithmetic error on a form whose caption states the
arithmetic.

★★ **Two consequences for R-6, both confirmed by the authority rather than assumed:**

1. **R-6's Σ is right, and must stay a Σ.** *"If you receive more than one form, a negative figure in
   box 5 of one form is used to offset a positive figure in box 5 of another form for that same year."*
2. **On MFJ it must sum across BOTH spouses, not per filer.** Pub 915's own example: Ryan's box 5 is
   $3,000, Jordan's is ($500), and *"Ryan and Jordan will use $2,500"*. A per-form or per-spouse R-6
   would refuse a return the instructions finish in one sentence — [[widening-an-exemption-is-never-
   the-safe-edit]] inverted: this is a refusal that is too WIDE, and too-wide refusals cost the filer
   the return.

★ **And R-6's remedy text is now sourced.** The >$3,000 repayment is §1341, and Pub 915 gives both
branches: the itemized deduction on **Schedule A line 16**, or a credit on **Schedule 3 line 13z with
"I.R.C. 1341" written on the entry line**, whichever produces the smaller tax. btctax computes neither,
so R-6 correctly refuses — but the refusal should name both routes, because a filer told only about the
deduction may take the worse of the two.

New class-(A) declarations on `ReturnInputs`, each `Option<bool>` — the type the classifier forbids `_`
on (`return_inputs.rs`), registered in `classifier.rs` beside
`c.declaration(other_out_of_scope_income, …)` at `classifier.rs`, and given a `FormQuestion` in
`questions.rs` with `durability: Durability::PerYear` and `neutral: false`:

| field | question, in the form's own YES-conditions | live when | `None` ⇒ |
|---|---|---|---|
| `Form1099R::exception_applies` | IRA: *"did you roll any of it over; do you have basis, a Roth, a conversion, a returned or recharacterized contribution; was any of it a qualified charitable distribution; was any of it an HSA funding distribution?"* | any IRA 1099-R | **R-1** |
| `Form1099R::exception_applies` | pension: *"was any of it rolled over; are you a retired public safety officer excluding premiums; is it a disability pension before your employer's minimum retirement age; is it a corrective distribution; is it a lump-sum distribution you are using Form 4972 for?"* | any pension 1099-R | **R-3** |
| `mfs_lived_apart_all_year` | *"If you are married filing separately and you lived apart from your spouse for all of 2025, check the box on line 6d"* (`3330-3332`) | MFS **and** Σ box 5 > 0 | **R-8** |
| `form_8815_or_adoption_exclusion` | *"You file Form 2555, 4563, or 8815, or you exclude employer-provided adoption benefits or income from sources within Puerto Rico"* (`3261-3264`), minus the part `has_income_exclusion` already asks | Σ box 5 > 0 | **R-7** |

★ `form_8815_or_adoption_exclusion` exists because `has_income_exclusion`
(`return_inputs.rs`) already covers §911 / §931 / §933 — Forms 2555 and 4563 and Puerto Rico —
but **not** Form 8815 (excluded savings-bond interest) or employer-provided adoption benefits. Asking
only the residue keeps the questionnaire honest and reuses the answer btctax already has. It is scoped
`live` to filers with benefits, so nobody else ever sees it. This is `CLAUDE.md`'s corollary applied
literally: *"If the form asks something our input surface cannot answer, collect it."*

**⚠️ I-5 FOLDED 2026-09-20 — THE SENTENCE BELOW WAS TRUE WHEN WRITTEN AND THE §9 REWRITE OF 2026-09-15
FALSIFIED IT WITHOUT COMING BACK HERE. Kept visible: this is the *"edited §X, forgot §Y"* class this
project's own doctrine flags, committed by me two days after writing the doctrine down.**

The stale sentence: *"**No question is added for the PSO exclusion, the lump-sum election, or the §11
SSDI carve-out.** Under S-1 each can only lower the figure, so each gets an advisory instead (§9)."*

§9 now rules **A-1 = ASK** (*"are you a retired public safety officer?"*) and **A-2 = ASK** (*"did the
benefits include a payment for an earlier year?"*). Only A-3, the §11 SSDI carve-out, remains prose. So
two questions WERE added, and they had no field, no live condition, no `None` behaviour and no mutation
row anywhere in the document.

**★★★ AND THE SEMANTICS ARE THE PART THAT MATTERS, because the two obvious homes are both wrong.** Every
`Option<bool>` on `ReturnInputs` is a class-(A) declaration, and the classifier forbids `_` on those —
so making A-1/A-2 class-(A) means an **unanswered one blocks the return**. That directly contradicts S-1:
each can only ever *lower* the figure, so silence is already the conservative outcome and refusing on it
would turn away a filer to protect them from an overpayment they did not have to avoid. Yet leaving them
out of the type system is how a question gets asked with no defined `None` at all.

**THE RULING, folded: A-1 and A-2 are ADVISORY-TRIGGER leaves, not class-(A) declarations. `None` is
SILENT and is never a refusal.** They get their own table because the difference is the whole point:

| field | question | live when | `None` ⇒ | `Some(true)` ⇒ |
|---|---|---|---|---|
| `retired_public_safety_officer` | *"Are you a retired public safety officer?"* (A-1) | Σ pension box 2a > 0 | **silent.** Never a refusal — S-1: the PSO exclusion can only lower line 5b, so silence overstates tax in the safe direction and the filer keeps their return | advisory only (§9); no figure changes |
| `benefits_included_an_earlier_year` | *"Did the benefits include a payment for an earlier year?"* (A-2) | Σ SSA-1099 box 5 > 0 | **silent**, same reason — the lump-sum election can only lower line 6b | advisory only (§9); no figure changes |

★★★ **The home exists, it is exact, and it is already carrying a question of this identical shape — so
"class-(A) or nothing" was a false dilemma.** The premise that *every* `Option<bool>` on `ReturnInputs`
is a refusing class-(A) declaration is **false**: `classifier.rs` also has `c.exempt(…)` and the
`SkippableQuestion` path, and `SKIPPABLE_QUESTIONS` is documented as *"the set of questions whose
silence is LAWFUL, not only the set that costs money"* (`questions.rs`). Measured: **19 of its 20
entries carry `unanswered: None`.**

The precedent is the blind checkbox, and every field A-1/A-2 need is already in it:

```rust
SkippableQuestion {
    id: SkippableId::BlindTaxpayer,
    unanswered: None,            // ← silence is LAWFUL; nothing refuses
    unanswered_detail: "",
    durability: Durability::PerYear,
    prompt: "Are YOU legally blind? (§63(f) additional deduction)",
    help: "…Skipping leaves it unclaimed — lawful, since the burden to claim is yours — and \
           the forgone-benefit advisory fires.",
    kind: SkippableKind::YesNo,
    live: |_ri| true,            // ← the `live` condition I-5 asked for
    get_bool: |ri| ri.header.taxpayer.blind,   // ← an Option<bool> ON ReturnInputs
    set_bool: |ri, v| ri.header.taxpayer.blind = Some(v),
    …
}
```

★★ §63(f) blindness and the PSO exclusion are **the same question shape**: answering yes lowers tax,
the burden to claim is the filer's, silence is lawful and forgoes the benefit, and an advisory fires on
the silence. So A-1 and A-2 are `Option<bool>` leaves on `ReturnInputs` registered in
`SKIPPABLE_QUESTIONS` with `unanswered: None`, their `live` closures as tabled above, and the §9
advisory on `Some(true)`.

★★ **The classifier bucket is named too, because the compiler will demand it (r2 M-4).** Adding two
`Option<bool>` leaves to `ReturnInputs` forces an arm in `classifier.rs`'s exhaustive destructure, and the
same precedent answers it — `blind` is registered as:

```rust
c.exempt(
    blind,
    Class::BenefitClaim,
    "§63(f) blindness — New Colonial Ice: the burden to CLAIM is the filer's, so `false`/absent is \
     lawful; the forgone benefit fires `BlindBoxForfeitedNotDeclared` (§2.2)",
);
```

So A-1 and A-2 are `Class::BenefitClaim` exemptions with the same ground: *New Colonial Ice* puts the
burden to claim on the filer, which is precisely why silence is lawful and why an advisory rather than a
refusal is the right response to it.

★ **PROSE is therefore not the fallback, and the build has no decision left to make here.** The earlier
draft of this fold hedged — *"if the only available `Option<bool>` slot is class-(A), A-1 and A-2 must
go back to PROSE"* — which was written before the mechanism was measured. It is retracted: the slot
exists, `durability: Durability::PerYear` is the right setting (both facts can change between years),
and forcing these into a refusing type was never the only alternative.

★ **⚠️ THE COUNT IS SEVEN, not six — corrected again 2026-09-20 by fold-review r2 (I-1).** The I-3
fold, in the SAME commit that announced "six", added a seventh question in a table cell — the code-T
Roth-contribution-year question — and gave it no field, no `live`, no `None` rule, no refusal id and no
mutation row. So this very paragraph, whose subject is a count that went stale, shipped stale. It is
recorded rather than quietly renumbered, because twice in one document is a pattern and not a slip.

**The seventh question, defined here so it exists somewhere a builder looks:**

| field | question | live when | `None` ⇒ | authority |
|---|---|---|---|---|
| `Form1099R::roth_contribution_before_lookback` | *"Did you contribute or convert to a Roth IRA for &lt;YEAR&gt; or an earlier year?"* — `&lt;YEAR&gt;` read from that revision's own instructions, **never computed** | this document's box 7 contains `T` **and** it is an IRA document | **REFUSES** (`RothLookbackUnanswered`) — the one new question that must: silence here would admit the filer to the `-0-` branch, which understates tax. Silence is not testimony | `:2707-2715`; the year pair `i1040gi--2025.txt:2713` = 2020, `i1040gi--2024.txt:2647` = 2018 |

★★ It is a **class-(A) refusing declaration**, unlike A-1 and A-2 — and the direction of error is why.
A-1 and A-2 can only ever *lower* the figure, so silence is safe. This one *raises* 4b when answered
`no`, so silence must not be read as either answer. **Four refusing + one refusing + two silent = seven
questions, five of which refuse.**

★ The count, corrected everywhere it appears (§7 here, §10's journey walk, §12 R-E). "Four" was right
for the original refusing set and was being read as the whole surface.

★ Two mutation rows are added in §10 (M-12, M-13): answer yes and the advisory must appear; answer no
and it must not; leave it `None` and **nothing must refuse**. That last clause is the one that reds if
someone later files these as class-(A).

TUI surface: one `decl_tristate!` entry per declaration in
`crates/btctax-input-form/src/spec/registries.rs` (model at `:210-211`), plus the `QuestionId` mapping
at `:388`.

---

## 7a. ⚠️ LINE 25b — THE RETIREE'S WITHHOLDING, ADDED BY REVIEW r1 (C-2, CRITICAL)

**The original spec dropped it.** `FormSsa1099` declared `box3_benefits_paid`, `box4_benefits_repaid`,
`box5_net_benefits` and **nothing for box 6**, and no section put line 25b in scope. Voluntary federal
withholding on benefits is a routine election — for many retirees it is the only way tax is withheld at all.

**The instruction names the boxes**, controller-verified at `i1040gi--2025.txt:4239`:

> *"…include the amount withheld in the total on line 25b. This should be shown in box 4 of Form 1099,
> **box 6, of Form SSA-1099, or box 10 of Form RRB-1099**."*

**★★★ And the receiving quantity is a TYPED LIST BESIDE A SET THAT GROWS.** At HEAD
`AbsoluteReturn::withholding_25b` documents itself as *"1040 L25b — federal income tax withheld from Form(s)
1099 (Σ box 4, **across INT/DIV/G**)"* — three document families, hand-listed. This feature adds **two**
(1099-R, and SSA-1099/RRB-1099) and the original spec touched neither the list nor any test of it. By the
spec's own rule — *a guarantee without a test that reds when it is removed does not exist* — there was **no
guarantee that any retirement withholding reached the return**.

**Direction of error, which is why it is Critical:** payments understated ⇒ lines 25d, 33 and 34/37 wrong ⇒
**balance due overstated, refund understated**. A wrong bottom line on a signed return, in the direction the
filer pays for.

**What T14 must carry.**

1. `box6_fed_withheld: Usd` on `FormSsa1099`. ★ One struct serving two forms must name **both** box numbers
   in its doc comment (SSA box 6, RRB box 10) or be split — otherwise an RRB filer's transcription has no
   home and the field name lies about half its sources.
2. Line **25b** in §4.1's in-scope table and §6's transcription table, with its new operands.
3. **`withholding_25b`'s doc comment restated as a DERIVED sum over the document families**, not a
   four-letter list, so the next information return cannot be added silently. This is the fix that outlasts
   the feature.
4. Mutation rows in §10, one per new family: *drop the 1099-R box-4 term from 25b — a household with pension
   withholding must stop reconciling*, and the same for SSA-1099 box 6.

---

## 8. Refusals — exact wording and firing condition

All are `RefuseReason` variants (`return_refuse.rs`) raised through `refuse(reason, detail)`.
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

**R-2 `IraDistributionExceptionUnsupported`** — `exception_applies == Some(true)` on an IRA document
**AND the document is not on the Roth qualified sub-branch below.**

> "you declared that one of the line 4a/4b exceptions applies to an IRA distribution. Each of them
> routes somewhere btctax cannot follow — a rollover to the 60-day rule and a filed statement, basis or
> a Roth to Form 8606, a qualified charitable distribution to the $108,000 limit and its attachment, an
> HSA funding distribution to Form 8889 Part III. btctax models only the fully-taxable case, so it
> refuses rather than file a line 4b it cannot stand behind. File that distribution yourself"

**★★★ I-3 FOLDED 2026-09-20 — R-2 WAS REFUSING THE ORDINARY RETIREE'S ROTH DISTRIBUTION, WHICH THE
INSTRUCTIONS FINISH IN ONE SENTENCE. The clause above is new; option (a) is taken.**

The instructions, verbatim (`i1040gi--2025.txt:2698-2715`, and the same sentence at TY2024 `:2638-2639`, its sub-branches at `:2640-2649` (r2 M-2: the first range was off by two and held only the sub-branches)):

> *"Exception 2. If any of the following apply, enter the total distribution on line 4a and see Form
> 8606 and its instructions to figure the amount to enter on line 4b. … 2. You received a distribution
> from a Roth IRA. **But if either (a) or (b) below applies, enter -0- on line 4b; you don't have to see
> Form 8606 or its instructions.** a. Distribution code T is shown in box 7 of Form 1099-R and you made
> a contribution (including a conversion) to a Roth IRA for 2020 or an earlier year. b. Distribution
> code Q is shown in box 7 of Form 1099-R."*

Under the unamended spec that filer answers `exception_applies = Some(true)` — a Roth distribution *is*
Exception 2 — and hits R-2. But the return is complete in two cells: **4a = total distribution, 4b =
`-0-`**, no Form 8606, no attachment, no election, no carryforward. This is
[[the-answer-is-in-the-manual]] exactly: the spec turned the filer away and the manual answered them in
two lines.

★★ **And the two codes are NOT symmetric — the newly archived payer instructions say why, and this is
what the collapse destroyed.** From `design/forms/extract/i1099r--2025.txt:2407` and `:2440`:

| code | the payer's own meaning | what the FILER must supply |
|---|---|---|
| **Q** | *"Qualified distribution from a Roth IRA"* — used *"if you know that the participant meets the 5-year holding period and"* age 59½ / died / disabled | **nothing.** The payer has already certified the qualification. |
| **T** | *"Roth IRA distribution, exception applies"* — used *"if you do **not** know if the 5-year holding period has been met"* but age 59½ / died / disabled | **exactly one fact**: the contribution/conversion year, which is the 5-year fact the payer lacked. |

So the instructions' asymmetry is not stylistic: code T carries a filer condition **because the payer
could not determine it**. A single `exception_applies` boolean cannot express "no question" and "one
question" as different branches, which is why S-3's collapse was safe for the other three Exceptions
and unsafe for this one.

**⚠️ THE RULE — REWRITTEN 2026-09-20 by fold-review r2 (C-1, CRITICAL). The first version of this table
composed with the I-2 fold in the same commit to UNDERSTATE line 4b, and nothing refused.**

The table as first written keyed its **rows** on a per-DOCUMENT condition (*"box 7 contains `Q`"*) and
labelled its **columns** with per-RETURN 1040 lines (*"line 4a", "line 4b"*). On the two-document return
the I-2 fold brought explicitly into scope, applying it literally understates total income:

| document | box 7 | box 1 | `exception_applies` | what fired |
|---|---|---|---|---|
| Roth IRA, qualified | `Q` | 10,000 | `Some(true)` | exempted from R-2 by the new clause |
| traditional IRA, fully taxable | `7` | 20,000 | `Some(false)` | computes; nothing refuses |

The old table said *box 7 contains `Q` ⇒ line 4b = `-0-`*, so **4b = `-0-`** on a return whose correct
4b is **20,000** — the whole traditional distribution missing from total income, with no refusal behind
it. ★★ The instructions answer the composition themselves and the fold had not transcribed it:
*"enter the part that is not a QCD on line 4b **unless Exception 2 applies to that part**"*
(`i1040gi--2025.txt:2731`) — **per part**, not per return.

**So the sub-branch is PER DOCUMENT and the aggregation is stated once, in the instructions' own
order.** Step 1 is per document; steps 2 and 3 are the only places a 1040 line is written:

| step | rule | authority |
|---|---|---|
| **1. each IRA document's own taxable amount** | `Q` ⇒ **`-0-`**; `T` with the contribution-year answer **yes** ⇒ **`-0-`**; fully taxable (no Exception) ⇒ **its box 1**; anything else ⇒ **R-2 refuses the return** | `:2707-2715`; `:2664-2667` |
| **2. line 4b** | **Σ of the per-document taxable amounts from step 1** | *"figure the taxable amount of each distribution and enter the total of the taxable amounts on line 4b"* (`:2789-2795`) |
| **3. line 4a** | **Σ box 1 over ALL IRA documents** whenever more than one exists, or any one is on the step-1 `-0-` sub-branch; otherwise blank on the single fully-taxable document | `:2789-2795`; `:2698-2700`; `:2664-2667` |

★★★ **The worked case above, under the rewritten rule: 4a = 30,000, 4b = 20,000.** That is the KAT
(§10, M-14) and it reds against the first version of this table.

★ **Why the first version was wrong in a way review was the right instrument for.** Each fold was
correct in isolation — I-3's sub-branch reading is sound (r2 confirmed it on three grounds) and I-2's
plural rule is quoted correctly. They were written in the same sitting, in the same commit, and the
defect existed only in their *composition*. That is the seam class I-5 was itself an instance of, and it
is why `STANDARD_WORKFLOW.md` treats a fold as authorship that re-earns the gate.

★ **A table whose rows and columns have different scopes is the mechanism, not the typo.** The columns
now say *this document's taxable amount*, and no per-document row names a 1040 line.

★★★ **THE YEAR IN THE T QUESTION IS YEAR-SHAPED AND IS NOT A CONSTANT OFFSET. Measured, both
revisions:**

| revision | the sentence says | offset from the tax year |
|---|---|---|
| `i1040gi--2025.txt:2713` | *"a Roth IRA for **2020** or an earlier year"* | −5 |
| `i1040gi--2024.txt:2647` | *"a Roth IRA for **2018** or an earlier year"* | **−6** |

A two-year jump across a one-year revision. So **`tax_year - 5` is wrong for TY2024 by one year and a
hardcoded `2020` is wrong for TY2024 by two** — and either error admits a filer to the `-0-` branch who
belongs on Form 8606, which understates tax. The year MUST be read from that revision's own
instructions, per the repo's per-revision doctrine, and the pair above is the KAT. ★ This is
[[derive-the-list-or-make-the-compiler-hold-it]] on a single integer: the plausible formula is the
defect.

★★ **`box7_distribution_codes` can no longer say "screened, never interpreted"** (§7, corrected in
place). The instructions' own decision procedure *is* a box-7 test, so the field that decides was being
held and not read. Interpretation is now confined to exactly two tokens, `Q` and `T`, with everything
else falling through to R-2 — a closed, enumerable reading rather than an open one.

**⚠️ CORRECTED 2026-09-20 by fold-review r2 (I-5) — MY JUSTIFICATION CITED A SENTENCE THAT SAYS THE
OPPOSITE, AND THE TEST I PRESCRIBED IS LOOSER THAN THE AUTHORITY.** The retracted text: *"Box 7 can carry
more than one code, and the payer instructions' own note — 'If any other code, such as 8 or P, applies,
use Code J' — shows codes are composed."*

That Note is printed **inside the `Q` and `T` entries of Table 1** (`i1099r--2025.txt:2414` under
*"Q—Qualified distribution from a Roth IRA"*, `:2447` under *"T—Roth IRA distribution, exception
applies"*). It says the opposite of what I used it for: for these two codes, if another code applies,
**use Code J instead** — so it is the authority for `Q` and `T` appearing **ALONE**. Table 1's *used
with* column for `Q` reads **`None`** (`:2416`). ★ `T`'s cell is dropped by the layout at `:2448` and
must not be asserted from this extract.

**So the test is EQUALITY, not membership.** The `-0-` sub-branch is taken only when box 7 is **exactly**
`Q`, or **exactly** `T`:

| box 7 | branch | why |
|---|---|---|
| `Q` | the `-0-` sub-branch | Table 1: used with `None` |
| `T` | the `-0-` sub-branch, after the seventh question | same entry's Note |
| `QJ`, `Q8`, anything else containing `Q` | **R-2 refuses** | the payer was instructed to print `J` instead, so a composed `Q` is a document the spec does not understand |

★★ `contains("Q")` is **wrong in the understating direction**, which is why this is Important and not a
style note: it would admit a composed code to the `-0-` branch on a distribution the payer has signalled
is *not* a plain qualified Roth. Equality fails closed.

★ **And the separator question is settled by measurement, not left unstated.** Every box 7 code in Table
1 is a **single character** — `1 2 3 4 5 6 7 8 9 A B C D E F G H J K L M N P Q R S T U W Y`, enumerated
from `i1099r--2025.txt` — so a two-code box 7 is two characters with no separator, and there is nothing
to split on. That is *why* equality is expressible at all, and a KAT must pin a composed box 7 (`"QJ"`)
taking R-2 rather than the sub-branch.

★ **OQ-3's QCD refusal is untouched.** This fold splits out only the sub-branch the instructions close
themselves; the qualified charitable distribution still refuses in v1 per the owner's ruling.

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

**★★★ OWNER RULING 2026-09-15, AND IT OVERRIDES THIS SECTION'S RECOMMENDATION. Verbatim: *"Zero is the
goal when nothing is wrong."*** The spec (OQ-4) proposed keeping all four and measuring the noise in the
journey walk. That is rejected: a plain retiree with one pension and social security, whose return is
**correct**, must see **no advisory at all**.

★★ **The mechanism, because the ruling is not a volume dial.** Every one of A-1…A-4 below fires on a
*condition of the return* — `line 5b > 0`, `line 6a > 0` — rather than on a condition that makes the
advice **apply to this filer**. Most pension recipients are not retired public safety officers; most
Social Security recipients had no lump sum for an earlier year; almost none were injured in a §11 attack.
So each one fires overwhelmingly on returns where nothing is wrong. **An advisory must be keyed on
whether it applies to THIS filer, and when that cannot be known, the choice is to ASK or to say nothing —
never to advise everyone.**

★ **Per-advisory verdicts under the ruling.** A-3's own text concedes the point (*"asking every
beneficiary about a terrorist attack is questionnaire bloat that buys nothing"*), and the ruling answers
it the other way: if the question is not worth asking, the advisory is not worth printing.

| | under the ruling |
|---|---|
| **A-1** PSO premium exclusion | **ASK.** One cheap yes/no ("are you a retired public safety officer?"), asked once, and advise only on `Some(true)`. A PSO who says yes is genuinely overpaying; everyone else sees nothing. |
| **A-2** lump-sum election | **ASK.** The SSA-1099 itself shows a lump-sum breakdown, so the filer can answer it from the paper in their hand. Advise only when they say the benefits included a payment for an earlier year. |
| **A-3** §11-attack SSDI carve-out | **PROSE.** To `btctax limitations`, per the ruling and its own concession. It reaches almost nobody and the failure direction is already conservative. |
| **A-4** Simplified Method not used | **⚠️ I MISREAD THIS — review r1 (I-6).** I called it the one advisory keyed on the filer rather than the return. It is not: §4.1 makes 5b *always* Σ box 2a, so A-4's condition **is** A-1's (`5b > 0`) and it fires on **100%** of in-scope pension returns — failing §9's own new test. And the source says only *"you **may** be able to report a lower taxable amount"*, not that box 2a is the higher figure; when box 2a equals box 1 there is usually nothing to recover. **Verdict: ASK, keyed on `box 2a == box 1`, or drop.** ORIGINAL (wrong) NOTE FOLLOWS: **ARGUABLE, and flagged rather than decided here.** Unlike A-1…A-3 this one fires on a condition that *is* about the filer: box 2a was taken, and box 2a is the **higher** of two lawful figures, so their tax really is above the minimum. But whether the alternative is lower cannot be known without the annuity facts. Either ASK for those facts (large) or keep A-4 as the single advisory a box-2a pension earns. **Owner decision at build time.** |

★★ **AND THE RULING REACHES SHIPPED CODE, which is the part that makes it more than a spec edit.**
Controller-measured 2026-09-15: `Advisory` has **27** variants, of which **8** are of the
"something-conservative-was-omitted" shape (`CtcOdcOmitted`, `EicOmitted`, `AgedBoxForfeitedNoDob`,
`AgedBoxForfeitedDeathUnanswered`, `OtherCreditsOmitted`, `UnmodeledDeductionsOmitted`,
`UnmodeledReturnOptionsOmitted`, `BlindBoxForfeitedNotDeclared`). ★★★ The clearest live violation is
`CtcOdcOmitted { provably_zero: true }`: it already **proves** the credit is worth nothing to this filer
and then prints anyway — *"NOT AVAILABLE TO YOU … here that costs you NOTHING … 1040 line 19 is $0 and
that is the correct figure"*. Under this ruling that is exactly the advisory that must be **silent**: the
return is right, the figure is right, and nothing is wrong. Tracked as FR-253.

★ So the existing `provably_zero` flag is the right *shape* and the wrong *action* — it branches the
message where it should suppress the advisory. Retirement's A-1…A-4 must not copy that pattern.


Added to `Advisory` (`advisories.rs`), each carrying its figure, in the shape of
`CtcOdcOmitted { dependents, provably_zero }`.

**A-1 `PsoPremiumExclusionNotTaken { pensions: usize }`** — ⚠️ **CORRECTED 2026-09-20 (r2 I-6): fires
when `retired_public_safety_officer == Some(true)`, NOT when line 5b > 0.** The old condition is the
very defect r1's I-6 red-flagged for A-4 — it fires on 100% of in-scope pension returns, so it advises
everyone and informs no one, and it violates the owner's ruling that **zero is the goal when nothing is
wrong**. It also contradicted §7's I-5 ruling in the same document and would have red M-12's own plant
(b). `live` for the QUESTION stays `Σ pension box 2a > 0`; the ADVISORY keys on the answer. Quotes the ceiling
verbatim: *"You can exclude from income the smaller of the amount of the premiums paid or $3,000"*
(`i1040gi--2025.txt:2926-2928`). Overstates tax for an eligible retired public safety officer.

**A-2 `SocialSecurityLumpSumElectionNotTaken`** — ⚠️ **CORRECTED 2026-09-20 (r2 I-6): fires when
`benefits_included_an_earlier_year == Some(true)`, NOT when line 6b > 0.** Beyond the A-1 reasoning, the
old condition was wrong in a second way: a filer whose benefits are NOT taxable has `Σ box 5 > 0` and
`6b = 0`, so §9 suppressed the advisory on exactly the return where the lump-sum election could still
matter — and M-13's plant (a) would have red against a spec-conformant build. Quotes the worksheet's own
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

## 9a. ⚠️⚠️⚠️ THE MFS-LIVED-WITH BRANCH HAS **ZERO** ORACLE WITNESSES — review r1 (C-3, CRITICAL)

**§10 promised a two-witness sweep across all five filing statuses *"including the MFS lived-apart /
lived-with pair"*. That is false, and believing it is dangerous.** Controller-verified in both engines'
source, not taken on report:

**Tax-Calculator 6.8.2**, `taxcalc.calcfunctions.SSBenefits` docstring, verbatim:

> *"`SS_thd1[2]`/`SS_thd2[2]` ($25,000/$34,000) encode only the "MFS lived apart all year" case.
> Tax-Calculator records do not carry a lived-apart-all-year flag …, so the code **unconditionally treats
> every MARS=3 filer as lived-apart and under-taxes MFS-lived-with-spouse filers. Not fixable code-side.**"*

**OpenTaxSolver 2024**, `taxsolve_US_1040_2024.c`, `SocSec_Worksheet()`: `ws[8]` is `32000.0` for MFJ and
**`25000.0` for everyone else, unconditionally**; `ws[10]` likewise 12000/9000. Measured across the whole
function: **zero** occurrences of *lived*, *apart* or *skip*. Neither engine implements the instructions'
*"skip lines 8 through 15"* jump for a lived-with MFS filer at all.

**★★★ So the two engines produce the SAME WRONG ANSWER on the branch with the largest money swing in this
feature** — the one T-1 calls a trap, M-5 exists to pin, and R-8 exists to refuse rather than guess. This is
`CLAUDE.md`'s *"Two disqualified oracles can align"*, landing on a new form.

**★★ And it is actively dangerous, not merely uninformative.** A builder who runs the sweep, sees it red on
an MFS-lived-with household, and reconciles to it — the normal and correct instinct with a red sweep — will
adopt the **lived-apart** thresholds for a **lived-with** filer. That understates tax by up to **85% of
benefits from the first dollar**. The instrument would actively drive the build to the wrong answer.

**What T14 must do instead.**

1. **Delete the claim from §10.** The sweep covers four filing statuses plus MFS-lived-**apart**. The
   lived-with branch is **witnessed by nobody**, and the spec must say so where the test plan is written.
2. **Add MFS-lived-with to the witness census as a known-zero-witness cell**, so it is counted rather than
   assumed — the census exists precisely to stop a filing status losing its last witness silently.
3. **Never reconcile the lived-with branch to the oracles.** Its authority is §86(c)(1)(C) and the
   worksheet's own *"skip lines 8 through 15"* sentence, transcribed. A KAT must come from the
   instructions' worked example or be hand-computed from the statute — never from a sweep.
4. ★ R-8's refusal is therefore **load-bearing rather than conservative padding**: with no witness, refusing
   a lived-with MFS retiree is the only defensible v1 behaviour, and that argument should be recorded in R-8
   rather than left implicit.

---

## 10a. ACCEPTANCE VECTORS FROM PUBLIC DATA — the third witness §10 lacked (added 2026-09-16)

**Why this section exists.** §9a (C-3) established that on the MFS-lived-with branch the two oracles
**agree while both wrong**, and [[FR-250]] established that the 107-household golden corpus contains no
retirement income at all. So before this section the feature had *no* independent witness: the oracles
cannot witness the worksheet's hardest branch and the corpus cannot witness the feature at all.

**TaxCalcBench** (`github.com/column-tax/tax-calc-bench`, **MIT**, 51 complete TY2024 returns with
expected MeF output) supplies one. Of the 51, **8** touch retirement and **4** are genuine 1099-R /
SSA-1099 cases rather than excess-Social-Security-tax withholding. Fetched and read 2026-09-16; nothing
committed to this repo yet (see [[FR-255]]).

| case | expected, verbatim from `output.xml` |
|---|---|
| `single-retirement-1099r-alaska-dividend` | 4a `IRADistributionsAmt` **10,000**; 5a `PensionsAnnuitiesAmt` **20,000**; 5b `TotalTaxablePensionsAmt` **20,000**; line 9 **31,000**; AGI **31,000**; deduction **16,550**; taxable **14,450**; tax **1,505**; 25b `WithholdingTaxAmt` **3,000**; refund **1,495** |
| `hoh-schedule-b-ssa1099-unemployment` | 6a `SocSecBnftAmt` **8,742**; **no taxable-benefit element at all**; line 9 **27,038**; AGI **26,447**; deduction **23,850**; taxable **2,597**; tax **259** |
| `mfj-both-blind-nontaxable-social-security` | 6a `SocSecBnftAmt` **7,333**; **no taxable-benefit element at all**; line 9 **5,000**; AGI **5,000**; deduction **35,400**; refund **1,000** |
| `single-w2-retirement-sick-pay-social-security-tip` | line 9 **100**; AGI **100**; deduction **14,600** |

**★★★ TWO INDEPENDENT CORROBORATIONS OF FINDINGS THIS SPEC ALREADY CARRIES, which is what makes this a
witness rather than a fixture.**

1. **C-2 is confirmed by a third party.** `single-retirement-1099r-alaska-dividend` asserts
   `WithholdingTaxAmt` **3,000**, which is exactly the two 1099-R box-4 amounts (1,000 + 2,000). A retiree's
   withholding belongs on line 25b — and this public case would have caught C-2's omission on its own.
2. **"Blank is the normal case" is confirmed from outside this repo.** *Both* non-taxable Social Security
   cases emit **no taxable-benefit element whatsoever** — not a zero. An independent MeF corpus represents
   a non-taxable benefit exactly as `CLAUDE.md` insists btctax must: the absence of testimony, not a
   printed `0`. R-6 and §5.1's ws9 STOP should be asserted against that shape.

**★★ WHAT THESE VECTORS DO NOT ASSERT, stated because assuming otherwise would be a fabricated KAT.**

- **Line 4b is not in the XML.** The 1099-R case's 1040 block carries 4a, 5a, 5b, line 9 and AGI and *no
  taxable-IRA element*. 4b = **10,000** is **DERIVED** (31,000 − 20,000 − 1,000), not read. A KAT may use
  it only with that derivation written beside it.
- **No MFS case exists in the corpus at all**, so C-3's zero-witness branch stays at zero witnesses. This
  section does not close §9a.
- **No charitable and no HSA case**, so [[FR-250]]'s gap is untouched.
- **TY2024 only.** The owner ruled both years; TY2025 vectors must come from elsewhere.
- These are synthetic-but-verified returns, not filed ones. They do **not** substitute for [[FR-64]], the
  owner's real documents driven end to end — which is the acceptance test §10 should name.

**★ And the standing rule when one of these disagrees with btctax: adjudicate against the FORM.** The
precedent is tenforty #278/#279 — OTS was never wrong, the wrapper was. A published corpus is a witness,
never an authority.

---

## 10. How it is tested

Every guarantee below names the mutation that must make it RED (**B1**). A guarantee without one does
not exist.

| # | guarantee | the mutation that must red it |
|---|---|---|
| **M-1** | ★ the pigeonhole holds: three lines may not claim fewer printed occurrences (S-9, FR-184) | give the 4b/5b/6b rows a quote the form prints only **twice**; the pigeonhole must red. ⚠️ **REWRITTEN 2026-09-20 (I-4).** It used to say *"swap the `line` labels on the 4b and 6b rows … it must red before the rows land"* — that mutation **passes and always will**, because the swap leaves the distinct-line set at 3 against 3 occurrences. A mutation that cannot fail is not a kill. |
| **M-1b** | ★★ row-to-line attribution: a 4b row is bound to the 4b FIELD, not merely to a line that prints the same words | swap the 4b and 6b rows' `line` labels; the **two-hop** join must red — `LineCoverage.field` → its `f1040.map.toml` row → the AcroForm name → `label_join` → the printed label. ⚠️ **GATED, and r2 (I-4) is why:** `LineCoverage.field` is the RUST name while `label_join` is keyed by the ACROFORM name, so the map row is the bridge and it does not exist yet for any of the six lines. **Scheduled after I-8's map cells**, not beside them. Measured available once they land: `label_join("f1040--2024")` returns `f1_47 → 4b`, `f1_49 → 5b`, `f1_51 → 6b`. |
| **M-14** | ★★★ **C-1's mixed return.** A Roth `Q` document (box 1 = 10,000) beside a fully-taxable traditional IRA document (box 1 = 20,000) files **4a = 30,000, 4b = 20,000** | implement §8's rule table per RETURN instead of per DOCUMENT — the version this spec carried before 2026-09-20 — and 4b becomes `-0-`, understating total income by 20,000 with no refusal behind it. The KAT must red. |
| **M-2** | 1040 line 9 sums the three new operands | delete `+ line6b` from the line-9 sum; a household with only benefits must stop reconciling |
| **M-3** | line 4a is blank, not `0`, on a fully-taxable IRA | change `line4a: Option<Usd>` to `Usd` / emit `Usd::ZERO`; the emitted-PDF read-back (`extract_lines`, `crates/btctax-forms/tests/extract_lines.rs`) must show the 4a cell present |
| **M-4** | line 5a is blank when the pension is fully taxable and present when box 2a < box 1 | force `line5a = Some(box1)` unconditionally |
| **M-5** | worksheet line 8's MFS-lived-with branch is a JUMP (T-1) | replace the jump with `ws8 = 0` and let 9–15 run. ★ The v1 arithmetic is unchanged, so the KAT must assert the **branch taken**, not only the dollar — a value-only test here is vacuous by construction |
| **M-6** | worksheet line 6 excludes Schedule 1 line 21 (S-5) | change it to `ar.adjustments`; a household with student-loan interest and benefits must move |
| **M-6b** | worksheet line 6 INCLUDES Schedule 1 line 13 (S-5, FR-183) | drop the `line13` operand; a household with an HSA deduction and benefits must move. ★ M-6 alone does NOT cover this — it mutates toward over-INCLUDING line 21, and a mutation that OMITS line 13 passes it. The two directions are separate kills, and the omission is the one that overstates the filer's tax |
| **M-7** | worksheet line 1 reads box 5, not box 3 | swap to `box3_benefits_paid`; a filer with a repayment in box 4 must move |
| **M-8** | the S-8 coupling: the worksheet is only the right instrument while IRA deductions refuse | delete the `IraDeductionClaimed` guard at `return_refuse.rs`; a KAT asserting *"a claimed IRA deduction never reaches the SS worksheet"* must red |
| **M-9** | R-8 refuses rather than defaulting | set `mfs_lived_apart_all_year` to `Some(true)` when unanswered; the refusal test must red |
| **M-10** | the four §86 thresholds are not indexed (S-7) | bump any of them by $1; the test that reads both `i1040gi--2024.txt` and `i1040gi--2025.txt` must red |
| **M-11** | 6b prints `-0-` on a STOP branch and is BLANK when no benefits exist (T-2) | make the no-benefits case emit `Some(Usd::ZERO)`; the read-back must show an empty 6b cell |
| **M-12** | A-1 (PSO) is ASKED, is ADVISORY-ONLY, and its silence NEVER refuses (§9, I-5) | three plants, and the third is the one that matters: (a) `Some(true)` ⇒ the §9 advisory must appear; (b) `Some(false)` ⇒ it must not; (c) ★★ **`None` ⇒ NOTHING may refuse.** Plant (c) by giving the field `unanswered: Some(RefuseReason::…)` in `SKIPPABLE_QUESTIONS` — the test must red, because that is exactly how a silent question becomes a blocking one by accident. |
| **M-13** | A-2 (lump-sum) likewise | the same three plants against `Σ SSA-1099 box 5 > 0` as the `live` condition. ★ Plant (c) is not a duplicate of M-12's: the two fields are registered separately, so one can acquire a refusing `unanswered` without the other. |

**Conformance KAT.** The expected line set is enumerated **from the extract**, never from a range or a
hand-list (`CLAUDE.md`, *Blank is the normal case*) — `crates/xtask/src/label_reader.rs` already derives
the label column from the form itself and classifies each row `Amount` / `Heading` / `NonMoney`
(`label_reader.rs`). Lines 4a–6d must each be **accounted for**: mapped to a field, or recorded
as carrying none **with a reason** (4c, 5c, 6c — §4.2).

**Both oracles, per `CLAUDE.md`.** OpenTaxSolver and Tax-Calculator both compute taxable social
security, so 6b gets a two-witness sweep across all five filing statuses, including the MFS
lived-apart / lived-with pair. ★ Disqualifications are **computed from the mechanism, never listed by
vector name** — the standing lesson from `verify_f6251.py`. 4b and 5b are *collected* figures on both
sides, so oracle agreement there proves nothing (`two-oracle-model` §G-9's limit: *a value the oracle
takes as INPUT is never validated by their agreement*) — those are held by KATs against the
instructions' own branches.

**Journey walk.** Before the plan is frozen, walk one retiree end to end with the owner: an SSA-1099, a
1099-R from a 401(k) with box 2a filled, no IRA. Where does each of the four REFUSING declarations
appear — and where do the two silent questions (A-1, A-2) appear, in
what order, and what does the packet look like when one is skipped?

---

## 11. Open questions for the owner

**OQ-1. Should the `OtherOutOfScopeIncome` prompt be widened NOW, ahead of this feature?** (§2.)
Recommendation: **yes, separately and immediately.** It is a one-line edit in the safe direction, it
turns a silent §61/§86 omission into a refusal today, and it is not coupled to anything in this spec.
Filing it as part of this feature delays the only protection retirees currently lack.

**★★★ RULED 2026-09-15: YES, separately and now — AND IT WAS ALREADY DONE, on 2026-09-04.**
Commit `1548462af`, *"fix(questions): name retirement income in the scope attestation — an
understatement path"*, is an ancestor of `main`. The `OtherOutOfScopeIncome` prompt's limb (a) now
opens with *"a PENSION, ANNUITY or IRA DISTRIBUTION (Form 1099-R), SOCIAL SECURITY or railroad
retirement benefits (Form SSA-1099 or RRB-1099)"*. So the ruling was satisfied before it was given,
and this section had been advertising open work for eleven days.

★★ **Two stale things this uncovered, both worth more than the question.** This section cited
`questions.rs` as the site to widen; that range is now `carryforward_in_present`, an
unrelated function — a drifted citation of exactly the kind that has misdirected three fixes this
week. And the fix commit itself **corrected its own overstatement**, which is the standard this spec
should be held to: *"Stated precisely, because the first write-up overstated it: the prompt DOES end
with 'or anything else it never asked about', so this is a PRIMING gap, not a blind spot. But a filer
holding a 1099-R and an SSA-1099, reading a list that contains nothing resembling their situation,
can give a truthful-feeling `No` on the strength of the enumeration."*

★ So the residual risk is **priming**, not silence — and that is the honest framing for §2's
*"converts a silent omission into a refusal"*, which claims more than the change delivers.

**OQ-2. Does 4b/5b/6b ship for TY2024 as well as TY2025?** The model is year-agnostic and the worksheet
is byte-identical across the two years (S-7, machine-checked) — **⚠️ FALSE, retracted by review r1 (C-1);
see the retraction under S-7.** Only the line-9 quote and the AcroForm
map are year-shaped (S-10), and the TY2025 `f1040.map.toml` is currently a stub carrying **only line
7a** (`crates/btctax-forms/forms/2025/f1040.map.toml:1-10`), so the TY2025 cells must be pinned with
`xtask dump-fields` either way. Recommendation: **both years**, since TY2024 is the year btctax can
actually file.

**★★★ RULED BY THE OWNER 2026-09-15: BOTH YEARS.** Verbatim: *"4-6 ship for both 2024 and 2025 because
I will have at least one of those returns for evaluation."*

★★ **The reason is the load-bearing part, and it changes how this feature must be tested.** "At least
one of those returns for evaluation" means a **real filed return** to validate against — not a
synthetic household. §10's test plan should name it as such.

**⚠️ CORRECTED 2026-09-15 by review r1 (I-1) — MY ORIGINAL JUSTIFICATION WAS FALSE, and it is kept
visible because I also put it in the review brief as the sharpest issue and thereby aimed a review
round with it.** I wrote that *"`FOLLOWUPS.md` §G-9 applies with full force here: both engines take Form
1040 line 6b as an INPUT, so a green two-oracle sweep proves nothing."* **Both engines COMPUTE line 6b.**
Verified in the primary sources:

- taxcalc — `calcfunctions.py`, `SSBenefits`: *"Calculates the taxable portion of OASDI benefits,
  c02500"*.
- OTS — `taxsolve_US_1040_2024.c`: *"SocSec_Worksheet(); /\* This calc. depends on line L6a and
  Sched1[11-25]. **Calculates L6b**, which is L[6]. \*/"*

So the sweep **is** a real three-way check on 6b, and §G-9 does not apply to it. ★ The real-return
requirement still stands on its own merits — [[FR-250]]'s corpus has no retirement household at all, so
day-one oracle coverage is zero regardless. What does not stand is calling it the *only* oracle.

★★★ **And the OTS comment corroborates S-5 independently:** the worksheet's input is *"L6a and
Sched1[11-25]"* — the BLOCK, not a list of two operands. That is the same finding S-5 records, arrived at
from the other direction.

★ **And "both years" is NOT symmetric work.**

**⚠️ I-8 FOLDED 2026-09-20 — THE 2026-09-15 TABLE HERE WAS WRONG AND IT UNDERSTATED THE WORK. It said
14 and 4 rows, a shortfall of "ten". Neither number reproduces under any definition.** It is kept
visible because this is the figure the owner's both-years ruling was sized against, and the error ran
in the direction that made the work look smaller.

Measured at `main`, 2026-09-20, with the commands beside the numbers:

| | money-line cells | command |
|---|---|---|
| `forms/2024/f1040.map.toml` | **35** | `grep -cE '^line[0-9]' <file>` |
| `forms/2025/f1040.map.toml` | **1** (`line7a` only) | same |
| the gap | **34 absent cells** | — |

`line_set` is metadata and does **not** match `^line[0-9]`, so no adjustment applies. Under the other
definition tried — all top-level `key =` — the figures are 134 and 35; the TY2025 residue is
`da_yes`/`da_no` plus a generated four-column dependents grid, not money lines. ★ I-8's own arithmetic
said "34 and 1, so 33 absent"; it subtracted `line_set` from a count that never included it. The
measured gap is **34**.

**★★★ And the deeper finding I-8 did not reach: the TY2024 map has NO 4a–6b cells either.**
`grep -E '^line(4|5|6)'` on the TY2024 file returns **nothing**. Its 1a–11 region maps 1a, 1z, 2a, 2b,
3a, 3b, 7a, 8, 9, 10, 11 — every retirement and Social Security cell is absent from the year this spec
calls *"the year btctax can actually file"*. So "both years" is not *"TY2024 already has them, TY2025
needs 34"*; it is **34 absent cells on TY2025 plus the six retirement cells on BOTH years**.

★★ **The six field names are already determined, and measured rather than guessed** — the AcroForm
numbering counts the gap. `f1_45` is line 3b and `f1_52` is line 7, so `f1_46`…`f1_51` are exactly the
six, and `dump-fields` geometry confirms the column pairing (x≈252 is the subline column, x≈504 the
amount column):

| line | field (TY2024) | x | y |
|---|---|---|---|
| 4a | `…Line4a-11_ReadOrder[0].f1_46[0]` | 252 | 210 |
| 4b | `…f1_47[0]` | 504 | 210 |
| 5a | `…f1_48[0]` | 252 | 198 |
| 5b | `…f1_49[0]` | 504 | 198 |
| 6a | `…f1_50[0]` | 252 | 186 |
| 6b | `…f1_51[0]` | 504 | 186 |

Every cell still gets pinned by `xtask dump-fields` and held by the read-back verifier at build time —
the table above is the starting point, not a substitute. ★ TY2025's numbering differs entirely
(`line7a` is `f1_70[0]` there, against `f1_52[0]` for 2024), so none of it transfers: the maps are
per-revision and the six TY2025 fields must be dumped separately.

★ **This is an owner-level number, so it is stated plainly: shipping both years means 34 + 6 + 6 = 46
map cells, not "six cells and ten rows".** The ruling stands as given — *"4-6 ship for both 2024 and
2025"* — and nothing here reverses it; it is recorded so the sizing is honest and so the owner can
re-confirm against the real figure rather than the one that was wrong.

**OQ-3. Is refusing every QCD acceptable?** A qualified charitable distribution is a common, deliberate
act for someone over 70½ and refusing it turns away a filer who did the ordinary thing. The
counterweight is real (§4.2: three caps, an age test, an attachment, an ordering rule, and a Schedule A
double-benefit path) and `no-users-yet` says the cost of refusing today is low. Recommendation:
**refuse in v1**, and file the QCD split as the first widening once the base lands.

**★★★ RULED 2026-09-15: REFUSE in v1, and the QCD split is the first widening once the base lands.** A QCD filer is turned away loudly rather than handed a wrong figure, and `no-users-yet` makes today's refusal cheap.

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
(`form1040_full.rs`) so this is buildable, but it must be built as a *distinction*, not as two
zeros.

**R-C. S-6's asymmetry invites exactly one compression.** "Taxable amount = box 2a" is the obvious
helper and it is wrong on the IRA side, where the instructions never mention box 2a. Two comments in
this codebase have carried confident equivalence claims that were false (`return_inputs.rs`,
`SPEC_schedule_1a.md` §2).

**R-D. S-5's operand set.** `adjustments` is right there and is wrong by one term. The error is
invisible in every household with no student-loan interest, which is most of them.

**R-E. Six new questions — four mandatory (they refuse when unanswered), two silent** (A-1 PSO and A-2
lump-sum: `None` is never a refusal, per the I-5 fold in §7). ⚠️ This row said *"Four new mandatory
questions"* until 2026-09-20; the §9 rewrite had added two and this section was not revisited. Each is
justified by a direction-of-error argument (§7), but the
questionnaire is a real product surface and this is the largest single addition to it. The journey walk
(§10) is where that gets measured, not a review round.

---

## 13. Cross-references

- `design/ty2025/SPEC.md` — parent; D-1 … D-11.
- `design/ty2025/SPEC_schedule_1a.md` — house style; S-1 (per-part parameters), F-5 (skip-vs-zero), §5 (test/green shape).
- `FOLLOWUPS.md` §G-11 — the emitter cannot express "no testimony"; this spec must not make it worse and must carry the distinction in its types.
- `FOLLOWUPS.md` §G-22 / B11 — the scope attestation this gap escapes through (§2).
- `crates/xtask/src/line_coverage_check.rs`, `crates/xtask/src/label_reader.rs` — the conformance instruments, one of which needs the S-9 fix first.
