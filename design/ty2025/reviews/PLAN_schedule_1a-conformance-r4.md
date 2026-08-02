# Schedule 1-A PLAN — review r4, FORM-CONFORMANCE LENS (Opus)

**Date:** 2026-08-01 · **Artifact:** `IMPLEMENTATION_PLAN_schedule_1a.md` (Status: r3) ·
**Brief:** [`../BRIEF-plan-r4.md`](../BRIEF-plan-r4.md)

**Why r4 exists:** the plan reads r3 but this directory held exactly ONE independent review of the plan
(r1). The r2→r3 fold was a 13-agent provenance CENSUS — not a review — and it *grew* a Critical. Two
folds were unreviewed against a rule that says re-review after every fold, including the last.

**Result: 0 Critical / 4 Important / 2 Minor / 1 Nit.** The arithmetic is right — every phase-out
re-derived from the printed lines, both worked examples reproduced, and the ceil/floor knee asymmetry
confirmed in all four directions. Every finding is in the two places the brief pointed at.

★★★ **I-2 SETTLES THE SEQUENCING QUESTION.** The plan's only filing gate is `L38 > 0`, but the form
gates at LINE and PART level — *"Fill out Part II only if you received qualified tips"*. So a filer
claiming only car-loan interest computes fifteen phase-out lines they were told to skip, **line 35
prints $6,000 for someone who is not a senior**, and a filer with no §911/931/933 exclusion prints `$0`
on lines 2a–2e — an affirmative statement about a Form 2555 they never filed. The reviewer's own words:
*"it is a struct-shape decision (`Usd` vs `Option<Usd>`) at the declared T2 chokepoint … and **B4 cannot
fix it if T2 makes every line non-optional**."*
**That is §G-11's defect, in TY2025, before a line of it is written — and it means `Collected<Usd>` is a
PREREQUISITE for T2, not a competing project.**

★★ **I-3 is an UNDERSTATEMENT path, lost in a fold.** r1's C-2 named *"a refinance **beyond the prior
balance**"*; the fold reduced it to *"a refinance"* and kept no balance condition. A cash-out refi then
deducts interest on the **entire** new balance — every YES-condition the plan collects answers *yes*.
The instructions state both halves in one paragraph, and the plan quotes neither. Its prose is
separately wrong: a refinance *does* qualify.

★ **I-1 is staleness, and the instrument already exists.** T2 says the expected set is "enumerated from
the extract", but the text layer yields **50** labels — lines 4 and 22 are headings with instruction
text and no amount box. `xtask/src/label_reader.rs` landed **one day after the plan's last commit** and
already adjudicates exactly this (50 labels / 48 entry / 2 headings, each with a required `note`). Drive
T2 from it rather than a fresh parse.

★ **I-4 is the same defect a third time** — Part II's *"not qualified tips"* list has three bullets and
the table carries one, with matched worked examples proving the occupation gate cannot answer it (a
bartender serving alcohol unlawfully has **non**-qualified tips).

**Verbatim below.**

---

I have everything I need. Writing the review.

---

# Schedule 1-A IMPLEMENTATION_PLAN — independent review r4, **FORM-CONFORMANCE LENS**

Reviewer: independent agent, 2026-08-01, against `IMPLEMENTATION_PLAN_schedule_1a.md` **r3** and the archived primary sources `design/forms/extract/f1040s1a--2025.txt` (`sha256:64f97b38…`) and `design/forms/extract/i1040gi--2025.txt` pp. 101-110. r1's findings read and **not** re-filed. Parent/child specs treated as settled.

**VERDICT: 0 Critical / 4 Important** (+ 2 Minor, 1 Nit)

The arithmetic is right. I re-derived every phase-out from the printed lines and every one matches, including both worked examples and the ceil/floor knee asymmetry. All four findings are in the two places the brief pointed at: **eligibility conditions the plan's own tables dropped**, and **the conformance KAT's ability to fail**.

---

### I-1. T2's conformance KAT, as specified, **cannot** produce the 48-line set from the extract — and the instrument that resolves it landed in-repo one day after the plan and is never referenced

**WHERE:** PLAN §T2, the conformance-KAT bullets; §1 exit criterion 2.

**CLAIM:** The plan requires the expected set to be "ENUMERATED FROM THE EXTRACT, never from a range", but the text layer provably yields **50** labels, not 48, and it does not carry the information needed to drop the two extras.

**CONSEQUENCE:** An implementer executing T2 literally reaches 50 and must either (a) hand-exclude lines 4 and 22 — the hand-written list CLAUDE.md forbids, which also removes both labels' instruction text from the conformance surface entirely, or (b) add two spurious money fields. Under (a) the KAT can never red on the loss of **line 4's multi-employer routing rule**, which is the only place on the form that says lines 4a/4b go to `-0-`; nor on line 22's ">two VINs" rule, whose arity the plan itself elevated to a T2 decision.

**EVIDENCE.** The plan:

> ★ **The expected set is ENUMERATED FROM THE EXTRACT, never from a range** — a `BTreeSet` built from `1..=38` either reds on every lettered field as an unexpected extra…

and exit criterion 2: "**All 48 line LABELS present**".

The form extract prints line 4 and line 22 as labels in the label column, each with instruction text and no amount box:

> `  4       Qualified tips received as an employee. If you received tips as an employee with`
> `          respect to employment with more than one employer, enter -0- on lines 4a and`
> `          4b and see the instructions to determine the amount to enter on line 4c.`

> ` 22   Applicable passenger vehicle (see instructions). If more than two VINs, see instructions.`

`crates/xtask/src/label_reader.rs` (landed `5d4d462`, 2026-07-30 — the plan's last commit is `c92cb9b`, 2026-07-29) states the impossibility in its own module doc:

> *"distinguishing a heading from a label means knowing whether the line has an amount box, **which the text layer does not directly say**."*

and measures the exact gap, against this exact form:

```rust
assert_eq!(labels.len(), 50, "printed label count changed: {labels:?}");
…
assert_eq!(entry.len(), 48, "the hand-established figure is 48 ENTRY lines; …");
assert_eq!(headings, vec!["4", "22"],
    "exactly lines 4 and 22 head their sub-rows and take no entry of their own");
```

It also already implements the "accounted for **with a reason**" half CLAUDE.md demands, which the plan's KAT does not specify at all:

```rust
/// A numbered line that is a HEADING for its lettered sub-rows and carries no box of its own
/// (Schedule 1-A lines 4, 14, 22). It is still a label and must still be accounted for.
Heading,
…
/// Why this row is what it is. Required for anything that is not a plain `Amount`, because a
/// bare classification nobody justified is exactly the "we forgot this line" case in disguise.
pub note: String,
```

**FIX:** Drive T2's expected set from `label_reader`'s adjudicated `Vec<Row>` (text witness + AcroForm box witness), not from a fresh parse of `f1040s1a--2025.txt`. State that the KAT asserts 50 labels / 48 entry lines / 2 headings-with-notes, and that a heading's instruction text is checked too. The census-F-4 point (worksheets come from the *instructions* extract) stands and is orthogonal.

---

### I-2. Nothing in the plan carries the form's **completion conditions** — so a filer prints five Part I lines and up to fifteen phase-out lines the instructions tell them to skip

**WHERE:** PLAN §T2 (struct shape), §T4 (compute), §T5 ("File Schedule 1-A only when `L38 > 0`").

**CLAIM:** The plan's only filing gate is `L38 > 0`. The form and instructions gate at **line level and part level**, and the plan never transcribes those gates.

**CONSEQUENCE:** Two concrete cases, both emitting sworn testimony the form does not ask for — the plan's own §T3a class:

- **Part I.** A filer with no §911/931/933 exclusion — nearly every filer — prints `$0` on lines 2a, 2b, 2c, 2d, 2e. Printing `0` on *"Enter the amount from Form 2555, line 45"* is an affirmative statement about a Form 2555 the filer never filed.
- **Parts II/III/V.** A filer claiming only car-loan interest still gets lines 8-12, 16-20 and 31-35 computed: a MAGI, a threshold, a positive excess, a step count and a reduction in each — **fifteen lines** under three cautions that say not to fill the part out. Line 35 in particular prints **$6,000** for a filer who is not a senior.

No figure is wrong (`L13/L21/L37` still resolve to `$0`), which is why this is Important and not Critical — but it is a struct-shape decision (`Usd` vs `Option<Usd>` on ~30 leaves) at the declared T2 chokepoint, exactly as line 22's arity was, and B4 cannot fix it if T2 makes every line non-optional.

**EVIDENCE.** Instructions, Part I (`i1040gi--2025.txt:43274-43285`):

> "If you don't have income from Puerto Rico that you excluded from your income, or you aren't filing Form 2555 or 4563, then **enter the amount from Form 1040, 1040-SR, or 1040-NR, line 11b, on Schedule 1-A, line 3**. If you do have excluded income from Puerto Rico, or you are filing Form 2555 or 4563, **complete lines 2a through 2e** in Part I of Schedule 1-A to figure your MAGI."

The form's own cautions:

> "**Caution:** **Fill out Part II only if you received qualified tips.**"
> "**Caution:** **Fill out Part III only if you received qualified overtime compensation.**"
> "**Caution:** **Fill out Part IV only if** you, or your spouse if married filing jointly, paid or accrued qualified passenger vehicle loan interest (QPVLI)."

and for Part V, the instructions:

> "**Fill out Schedule 1-A, Part V, only if:** • You (and/or your spouse if filing a joint return) were born before January 2, 1961. • You have a valid social security number (SSN)."

The plan applies precisely this doctrine two sections earlier and then stops:

> ★ **btctax has exactly three lawful moves here — collect, refuse, or genuinely blank — and "silently zero" is none of them.**

and §T4 resolves the MFS bar to a *value* rather than a non-completion: "Parts II, III and V print *'If married, you must file jointly to claim this deduction'* ⇒ **zero for MFS**".

**FIX:** In T2, give each part a completion predicate transcribed from its caution, and Part I lines 2a-2e the `has_income_exclusion == Some(false)` ⇒ *not completed* branch; make the affected leaves `Option<Usd>` so "not completed" is representable before B4 needs it.

---

### I-3. The fold of r1's C-2 **lost** one of its named Part IV conditions: the refinance balance cap has no declaration, and the plan's prose now mis-states refinancing entirely

**WHERE:** PLAN §T3, "Part IV — car loan interest", the declaration paragraph and the three block quotes.

**CLAIM:** r1's C-2 listed "a refinance **beyond the prior balance**" among the cases that must be barred. The plan's fold reduced this to "a refinance" in prose and carries **no** collected condition for the balance limit.

**CONSEQUENCE:** A filer who refinanced a qualifying 2025 auto loan and took cash out deducts the interest on the **entire** new balance. Every YES-condition the plan does collect answers *yes* for such a loan: it was originated after 2024-12-31, by the filer, secured by a first lien on the purchased APV, on a new US-assembled personal-use vehicle, with no negative equity. Up to $10,000 of interest on non-qualifying principal → **understates tax**. The plan's prose ("including on … a refinance …") is separately wrong: a refinance *does* qualify.

**EVIDENCE.** The instructions state both halves in one paragraph (`i1040gi--2025.txt:44486-44494`):

> "**Refinanced loan.** If your prior loan that had QPVLI is later refinanced, interest paid on the refinanced amount is **generally eligible** for the deduction, so long as the new loan is secured by a first lien on the APV with respect to which the refinanced loan was incurred. **The loan amount is limited to the outstanding balance of the refinanced loan as of the date of the refinancing.**"

The plan's quoted authority stops at the five numbered loan requirements, the APV conditions and negative equity — the *Refinanced loan* paragraph appears nowhere. Its only mention is the prose list:

> So every filer who typed a car-loan interest figure got up to **$10,000** of deduction — including on a lease, a used car, a non-US-assembled car, **a refinance**, a pre-2025 loan, or negative equity.

**FIX:** Add a per-vehicle row to T3's declaration table: *"Is this a refinancing? If yes, what was the outstanding balance of the refinanced loan on the refinancing date?"* — with the interest limited to that fraction — and correct the prose so a refinance is not described as disqualifying.

---

### I-4. Part II's *"Amounts received that are not qualified tips"* list has **three** bullets; T3's declaration table carries only the first

**WHERE:** PLAN §T3, the Part II declaration table (five rows: occupation list, multi-occupation carve-out, qualified-tip amount criteria, SSTB tips, ⊆ box 7).

**CLAIM:** The plan folded the SSTB bullet and dropped the two beside it in the same list — the illegal-service exclusion and the prostitution/pornography exclusion — and neither is derivable, nor is either subsumed by the occupation gate.

**CONSEQUENCE:** A filer whose tips came from a listed occupation performed unlawfully deducts them. Understates tax, up to the $25,000 cap. This is the third instance of the same defect the plan itself names — "r1 found this table **incomplete in the dangerous direction, twice**".

**EVIDENCE.** One list, three bullets (`i1040gi--2025.txt:43450-43485`):

> "**Amounts received that are not qualified tips.** The following are examples of amounts that are not qualified tips.
> • If your employer is in a specified service trade or business (SSTB), tips received as an employee of that employer are not qualified tips. …
> • **Tips received while performing a service that is a felony or misdemeanor under applicable law are not qualified tips.** However, tips you received for a service that is legal but were received while working for an establishment that violates applicable law in other respects may be qualified tips.
> • **Amounts received for prostitution and pornographic activity are not qualified tips.**"

The occupation gate does **not** answer it, and the instructions prove that with a matched pair of worked examples — parent D-10's *strongest* evidence class:

> **Example 1.** "…"Bartender" **is on the list** of occupations that customarily and regularly received tips. However, because you served alcohol in violation of applicable state law, the $10,000 in tips that you received in 2025 **are not qualified tips and may not be deducted**."
> **Example 2.** "…because working as a server is legal under state law, the $10,000 in tips you received in 2025 **are qualified tips** and qualify for the deduction."

**FIX:** One more row in T3's Part II table, as a YES-condition defaulting to NO, worded on the Example-1/Example-2 distinction (the *service* must be legal; the employer's unrelated violations do not disqualify). Cite both examples in the prompt's doc comment so the distinction is not re-derived.

---

### M-1. Line 4b's provenance is never stated — Form 4137 is a third form btctax does not model, and the census enumerated exactly two

**WHERE:** PLAN §T3a ("★★ **TWO LINES HAVE NO INPUT PATH AT ALL**"), §T3's leaf list.

Form line 4b: *"Qualified tips included on Form 4137, line 1, row A, column (c). If Form 4137 is not filed, enter -0-."* There is no Form 4137 struct in `ReturnInputs`; the only trace is a refusal on one of the form's two triggers (`return_refuse.rs:772`, *"W-2 box 8 allocated tips require Form 4137"*) — the other trigger, *"you received cash and charge tips of $20 or more in a calendar month and didn't report all of those tips to your employer"*, is neither asked nor visible. The child spec §4.1 lists "`L4b` Form 4137" as a collected leaf, so this is probably a declaration rather than a gap — but T3a's census asserts a count of two and never adjudicates 4b, which leaves a printed `-0-` whose provenance is undecided. The direction is safe (line 4c takes the *larger* of 4a/4b, so a false `-0-` shrinks the deduction), which is why this is Minor. State the move — collect, or the form's own conditional constant with the condition recorded.

### M-2. "48 line LABELS" names the wrong set; the label set is 50

`§1` exit criterion 2 and `§T2` both say "48 line LABELS". Per `label_reader.rs`'s measurement the printed **label** count is 50 and 48 is the count of labels that **take an entry**. The plan's per-part arithmetic (7+12+10+10+8+1) is correct for the entry set. Rename it, or exit criterion 2 will be written as an assertion on the wrong quantity — which is how I-1 becomes a hand-list.

### N-1. Two Part IV conditions in the instructions are not carried

The *"Change in obligor by reason of previous obligor's death"* exception to loan requirement 2 (an heir who assumes a qualifying loan **does** qualify — omitting it denies a deduction, so it fails closed), and the *Personal use* definition's operative test (*"you expect that the APV will be used for personal use for more than 50% of the time"*, with the household-relative list). The plan collects a bare "personal use" declaration; the instructions define it.

---

## ALSO CHECKED, SOUND

- **Every phase-out re-derived from the printed lines.** Caps $25,000 / $12,500-$25,000 MFJ / $10,000 map to lines 7, 15, 24; thresholds $150,000-$300,000 (lines 9, 17), $100,000-$200,000 (line 26), $75,000-$150,000 (line 32). Floor on lines 11 and 19 (*"decrease the result to the next lower whole number"*), **ceil** on line 28 (*"increase the result to the next higher whole number"*), smooth on 34. Per-step $100/$100/$200/6%.
- **The exhaustion table is right in all four directions**, including the one that matters: at excess $49,000 line 28 = 49 → line 29 = $9,800 → line 30 = **$200**; at $49,001 → $0. Part IV exhausts at `+$49,001`, not `+$50,000`. Parts II/III/V at `+$250,000` / `+$125,000` (`+$250,000` MFJ) / `+$100,000`.
- **Both worked examples reproduce exactly.** (b) MAGI $157,350, tips $3,000: L10 = 7,350 → L11 = 7 → L12 = 700 → **L13 = $2,300** (a ceil gives $2,200). (c) MAGI $104,050, QPVLI $6,000: L27 = 4,050 → L28 = **5** → L29 = 1,000 → **L30 = $5,000** (a floor gives $5,200). And (d) MFJ two seniors at MAGI $200,000: L33 = 50,000 → L34 = $3,000 → L35 = $3,000 → **L37 = $6,000**.
- **Line 34's third rounding site.** `0.06 × 50,025 = 3,001.50`; excess ≡ 25 (mod 50) is the exact collision set, and *round-the-difference* is the form that inflates the deduction. The plan's ruling — round line 34, then subtract — is right.
- **Part I is not a gap.** `ReturnInputs::modified_agi` already computes `AGI + §933 + 2555 L45 + 2555 L50 + 4563 L15` and returns `None` when `has_income_exclusion` was never asked, so the MAGI refuses rather than defaulting. 1040 line **11b** is confirmed as the page-2 restatement of AGI (`f1040--2025.txt`: *"11b Amount from line 11a (adjusted gross income)"*).
- **`L37 → Form 6251 line 1a`** verified against the 2025 form: *"1a Subtract Schedule 1-A (Form 1040), line 37, from Form 1040, 1040-SR, or 1040-NR, line 14"* — the **senior subtotal**, as D-3 says.
- **52 data leaves is now mechanically corroborated**: `f1040s1a--2025.json` carries **54** AcroForm boxes = 52 + the name and SSN header fields.
- **MFS is right.** Parts II/III/V print the joint-filing caution; Part IV does not, and its instructions read *"• Married filing jointly—$200,000. • **All other filing statuses**—$100,000."*
- **T7's QSS scoping is right and I tried to break it.** taxcalc's QSS defect is genuinely Part IV-only — `TipIncomeDed_ps`, `OvertimeIncomeDed_ps` and `SeniorDed_ps` all leave QSS at the unmarried figure — so "QSS Part IV inside the band ships zero-oracle" is correctly narrow, not under-scoped.
- **Four worksheets, correctly distinguished**, with their targets confirmed: tips-multi-employer → line **4c**, multiple-trades → line **5**, overtime-multi-**employer** → line **14a**, overtime-multi-**payor** → line **14b**. The last two are distinct and the plan does not collapse them.
- **Line 4c is `max`, not `sum`** (*"enter the larger of line 4a or line 4b"*), and the instructions' bartender example is consistent with it because Form 4137 line 1 column (c) is *total* tips, not unreported ones. A verbatim transcription gets this right; no finding.
- **The line-5 ceiling adjudication** (narrative governs over the two Examples; fail-closed direction) and **T3a's refusals** are correct as reasoned.
- **Nothing in T2-T7 opens TY2025 early**; the fail-closed gate is untouched and B3 is condition 4 only.

## WHAT WOULD MAKE THIS REVIEW WRONG

- **I-1** collapses if T2's KAT was always intended to consume `label_reader`'s output and the plan's "enumerated from the extract" is shorthand for it. The plan (r3, 2026-07-29) predates the label reader (2026-07-30) and names neither it nor `Kind::Heading`, so I read it as a fresh parse — but this is a *staleness* finding, not a design error, and one sentence closes it.
- **I-2** assumes the emitted PDF prints a computed `Usd::ZERO` rather than leaving the box empty. If B4's emitter already suppresses zero-valued boxes for uncompleted parts, the printed-testimony consequence disappears and only the `Option` struct-shape decision remains — still a T2 chokepoint question, but a Minor one. I did not read the emitter; §G-11's recorded position ("the emitter cannot express blank") is why I assumed it prints.
- **I-3 and I-4** assume T3's declaration table is the enumeration of record. The child spec's S-6 says the opposite — *"The branch list below is illustrative, NOT exhaustive … the build walks them"* — under which an omission from the plan is not yet a defect. I resolved against that reading because the plan's own §T3 treats the table as the deliverable and r1's two Criticals were filed on exactly that basis; if the parent prefers S-6's reading, both drop to Minor and the burden moves onto whoever walks the instructions during the build.
- I reviewed **conformance to the form only**. I did not assess the `_`-on-money hole, T3's plumbing, sequencing, or whether T2-T7 each land green — those are the other lens's.