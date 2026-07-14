# Rounding: why SPEC §3.1's cross-footing election is RIGHT

*This justification has been WRONG THREE TIMES — v1 cited the instructions backwards (too kind to
btctax); v2 over-corrected into "we knowingly depart" (too harsh); v3 claimed the IRM inverted the
exposure in our favour (too kind again, and simply false — the IRM treats line 24 as a **cents** line).
Each time, an independent reviewer caught it. That track record is why this file exists, why every claim
below is quoted rather than paraphrased, and why the conclusion is stated as **SUFFICIENT, not
DECISIVE**.*

**The conclusion has survived all three rounds: btctax's cross-footing is right, and no SPEC amendment
is needed.** What kept changing was the argument for it. The strongest evidence is the MeF wire format,
not the instructions, not the regulation, and not the IRM.

---

## The question

Form 1040 line 24 = line 22 + line 23.

| | line 22 | line 23 | line 24 |
|---|---:|---:|---:|
| exact | 8,354.59 | 8,477.73 | 16,832.32 |
| **Reading A** — round each reported line, sum the rounded lines | **8,355** | **8,478** | **16,833** ✅ the form cross-foots |
| **Reading B** — carry cents, round only the total | 8,355 | 8,478 | **16,832** ❌ printed lines do not add up |

btctax (SPEC §3.1) does **Reading A**. OpenTaxSolver does **Reading B**. The difference is $1 per summed
line and can fall either way.

The Form 1040 instructions ("Rounding Off to Whole Dollars", 2024, p. 23) appear to endorse Reading B:

> "If you have to add two or more amounts to figure the amount to enter on a line, include cents when
> adding the amounts and round off only the total."

**That sentence is real, and it is narrower than it looks.**

---

## 0. ★ WHAT IS LAW, AND WHAT IS NOT — read this before anything below it

**The single biggest error in the first three versions of this file was arguing from sources that are
not law, and then letting one of them (the IRM) talk me *out* of a defensible position.**

| source | what it is | force |
|---|---|---|
| **26 USC § 6102** | the STATUTE | **LAW** |
| **26 CFR 301.6102-1** | Treasury regulation under it | **LAW** |
| Form 1040 instructions | IRS publication | **not law** |
| **Internal Revenue Manual (IRM)** | internal procedures for IRS employees | **NOT LAW** — confers no rights on taxpayers, binds no one |
| MeF XML schema | a transmission format | **not law** |
| IRS Direct File | the IRS's own software | **not law** |
| Drake / TurboTax / OTS | industry practice | **not law** |

Everything below the second row is **evidence of what the IRS DOES** — genuinely useful for predicting
how a filed return will be treated — and **worthless as authority for what the law REQUIRES**. Courts
have repeatedly held the IRM does not have the force of law and creates no taxpayer rights; the same
goes for form instructions and publications.

**This matters concretely.** Version 3 of this file weakened btctax's position because IRM 3.11.3.14.2.28
says IRS clerks transcribe line 24 "as dollars and cents". That is a *keypunch procedure*. It says
nothing about what a taxpayer is required to ENTER on the form, and it cannot make a lawful entry
unlawful. An agency's internal practice is not a source of obligation — and an agency doing something
does not make it right.

## 1. The statute — and it does NOT cleanly resolve the question

**26 U.S.C. § 6102** ([Cornell LII](https://www.law.cornell.edu/uscode/text/26/6102)), verified directly:

> **(a) Elective use of whole-dollar amounts.** … any amount required to be shown on a form … shall be
> entered at the nearest whole-dollar amount…
>
> **(b) Election not to use whole-dollar amounts.** [the filer may report full cents instead]
>
> **(c) Inapplicability to computation of amount.** "The provisions of subsections (a) and (b) shall not
> be applicable to **items which must be taken into account in making the computations necessary to
> determine the amount required to be shown on a form**, but shall be applicable **only to such final
> amount**."

★ **Be honest about what this does and does not settle.** Line 22 is **simultaneously**:
- an *"amount required to be shown on a form"* — §6102(a) says enter it at the nearest whole dollar; and
- an *"item taken into account in making the computations necessary to determine"* line 24 — §6102(c)
  says the rounding rule does not apply to such items.

**Both readings survive the statutory text.** The earlier claim in this file that "the ambiguity
dissolves" was wrong. **Neither Reading A nor Reading B is unlawful** — and note §6102 is *elective* in
the first place (a filer may decline to round at all under (b)).

What tips it toward Reading A is the form's **own instruction for that line**: line 24 reads *"Add lines
22 and 23"* — **the LINES**, which §6102(a) requires to be entered at whole dollars. It does not say "add
the exact amounts underlying lines 22 and 23." Under the statute, the *items* the computation takes into
account are the entered lines. The regulation's own example points the same way (each "item of receipt"
— a receipt is not a line).

That is a **good argument, not a compulsion.** State it that way.

## 2. The regulation

**26 CFR 301.6102-1** — [Cornell LII](https://www.law.cornell.edu/cfr/text/26/301.6102-1), verified
directly:

> **(a) Amounts shown on forms.** … **any amount required to be reported on such form shall be entered
> at the nearest whole dollar amount.**
>
> **(c) Inapplicability to computation of amount.** The provisions of paragraph (a) … **apply only to
> amounts required to be reported** on a return … **They do not apply to items which must be taken into
> account in making the computations necessary to determine such amounts.** For example, each item of
> receipt must be taken into account at its exact amount, including cents, in computing the amount of
> total receipts required to be reported…

This is **supportive, not dispositive** — and the earlier drafts of this file overstated it. The "include
cents" rule governs **items that are not themselves reported on a line** (the individual receipts behind
a total), and lines 22 and 23 *are* reported amounts, which favours Reading A. But the honest counter
survives: line 22's exact value ($8,354.59) is, quite literally, "an item taken into account in making
the computation" of line 24, and the reg's *example* (source receipts) does not necessarily limit the
rule it illustrates. **The ambiguity does not "dissolve"** — the structure and the paragraph-(b) binary
election lean our way, but the text does not compel the result. The decisive evidence is §2.

**The rule, stated properly:**

> **Round at the point of reporting.** An amount PRINTED on a line is rounded to whole dollars, and a
> line that sums other printed lines sums the **rounded, printed** values. Amounts that appear **nowhere**
> on the return (several W-2 box-2 figures behind line 25a) are carried at exact cents and rounded once,
> at the line where they first surface.

btctax already does exactly this.

## 3. Every e-filed 1040 is integer-only — cents cannot even be transmitted (EVIDENCE, not law)

The MeF schema types every 1040 money element as `USAmountType` / `USAmountNNType`, defined in
`efileTypes.xsd` as:

```xml
<xsd:simpleType name="USAmountType">
  <xsd:documentation>Type for a U.S. integer amount field</xsd:documentation>
  <xsd:restriction base="xsd:integer"><xsd:totalDigits value="15"/></xsd:restriction>
</xsd:simpleType>
```

A `USDecimalAmountType` ("dollars and cents") exists in the type library and is used **zero times** on
the 1040. Lines 22, 23 and 24 are **all** `USAmountType` — integers. So an e-filed return transmitting
8,355 and 8,478 and then reporting **16,832** would be **inconsistent with its own transmitted
components**: Reading B is not merely discouraged, it is **unrepresentable on the IRS's wire format**.

★ **This is the decisive argument. Lead with it, not with the regulation.**

⚠️ **Sourcing, stated honestly.** The IRS **gates** MeF schema distribution (Pub 4164 §2.9 — Registered
User Portal / SOR), so there is no canonical public IRS URL for this XSD. The text above is from public
GitHub mirrors, corroborated **byte-identical on these types across three independent mirrors spanning
2011→2025**. That is strong, but it is a mirror, and this file will not pretend otherwise.

⚠️ **A trap for whoever checks this next:** `schemas.liquid-technologies.com/eFile/3.1/…` is the
**2004-era eFile 3.1** library. It has `AmountType` (integer, 11 digits) and **no `USAmountType` at
all**. Verify against a **tax-year-correct** mirror, or you will conclude the type was invented.

## 4. The IRS's OWN software implements Reading A — with a comment saying so (EVIDENCE, not law)

IRS Direct File is open source ([IRS-Public/direct-file](https://github.com/IRS-Public/direct-file)).
Its money type is `BigDecimal` at scale 2 with `HALF_UP` rounding, and **every reported line is wrapped
in `Round(...)` and derives from other already-rounded line facts** — `/agi = Round(Subtract(/totalIncome,
/adjustmentsToIncome))`, where both operands are themselves `Round(...)`. The printed return cross-foots
by construction.

In `tax/interest.xml` they state the rule outright:

```xml
<!-- We're intentionally summing rounded numbers because that is what Schedule B requires -->
<Dependency path="/interestReports/*/roundedTaxableInterest" />
```

And the switch around it *is* the whole answer:
- Schedule B **required** (each payer is printed on a line) ⇒ sum the **rounded** per-payer amounts — **Reading A**.
- Schedule B **not required** (payers appear nowhere) ⇒ sum the **exact** amounts, round once — **Reading B**.

Their own test fixture (`schedule-b-multiple-interest-rounding`, 14 × 1099-INT) diverges by **$7**:
exact sum 9,808.14 → Reading B would print 9,808. The IRS's expected-output PDF prints **9,815** — the
sum of the rounded rows. **Reading A, on the very fact pattern the "include cents" sentence supposedly
governs.**

## 5. The IRM — NOT LAW, and it should never have been allowed to move this argument

★ **Two errors here, and the second is the worse one.**
**(i)** v3 claimed the IRM "inverts the exposure" in btctax's favour — that a Reading-A return reproduces
the IRS's recomputation of line 24 exactly and "cannot draw a math-error notice". **False**, and false in
the direction that flattered us.
**(ii)** v4 then *weakened btctax's position* because IRM 3.11.3.14.2.28 is adverse on line 24. **That was
a category error.** The IRM is **not law** (§0). It describes how IRS *employees key a form into their
system*. It has no bearing on what a taxpayer is required to enter, and an internal procedure cannot make
a lawful entry unlawful.

The IRM is recorded here as **evidence of IRS behaviour** — useful for predicting how a return will be
handled — and for **nothing else**. What it says:

**It supports cross-footing for lines 1–23** — [IRM 3.11.3](https://www.irs.gov/irm/part3/irm_03-011-003r):

> **IRM 3.11.3.14.2:** "All lines on Form 1040 are edited in **dollars only** except lines 24 through 38."
>
> **EC 369:** "Verify that dollars and cents were **not** entered into dollar-only fields."

So when the IRS recomputes the **subtotals** — lines 9, 11, 15 — it is necessarily summing whole-dollar
values. That is genuine (if inferential) support for §3.1 across most of the return.

**It does NOT support us on line 24:**

> **IRM 3.11.3.14.2.28:** line 24 is edited **"as dollars and cents."**

The IRS transcribes line 24 with cents. **This neither helps nor harms btctax** — it is a keypunch
convention, not a rule of entry, and §6102 is what governs what the filer writes.

**And it does not matter, because the exposure is not live in either direction:**

> **IRM 3.12.3.31.15.5:** "If the taxpayer has computed total tax or total payments in 'dollars and
> cents,' but has rounded or truncated the balance due or refund, **follow the taxpayer's intent** and
> adjust total tax or total payments accordingly."

The IRS's stated posture on rounding-sized deltas is to **follow the filer's intent**, not to assess.
(The math-error tolerance itself — **Error Code 334**, whose *Taxpayer Notice Code* is 282; the earlier
draft conflated the two — is **redacted** in the public IRM text: `≡ ≡ ≡`. So it cannot be shown either
way.) The $1 anxiety this whole enquiry started from is simply **not a live exposure**, for us or for
OTS. That is the honest finding, and it is less exciting than the one I wanted.

## 5. Industry

- **Drake** ([KB 14118](https://kb.drakesoftware.com/kb/Drake-Tax/14118.htm)): *"All amounts will be
  rounded to the whole dollar to conform with IRS e-file guidelines… The IRS accepts e-filed returns
  rounded to the whole dollar only."* — Reading A.
- **IRS Free File Fillable Forms**: total lines are calculated fields computing from the entered
  (rounded) lines — cross-foots by construction. Reading A.
- **TurboTax** is inconsistent (rounds each 8949 row and sums the rounded rows — Reading A — but printed
  TY2024 Schedule B with cents).
- **OpenTaxSolver** is Reading B. **It is the outlier, not the norm.**

### A note on OTS's printed form — and why we are NOT reporting it

Verified on OTS's own output for the `single_miner_qbi_limited_by_net_capital_gain` household. OTS's
solver is correct internally (it carries cents); the defect is in the PRINT layer.
`universal_pdf_file_modifier.c` is a **generic text rounder** — it applies `(int)(x + 0.5)` to every
number it encounters and has **no knowledge of line relationships**, so it cannot know that line 24 is a
total of two lines it just rounded. With the shipped default `Round_PDF_to_Whole_Dollars Y`, the printed
1040 reads:

```
  line 22  tax          8,355     ← round(8,354.59)
  line 23  other taxes  8,478     ← round(8,477.73)
  line 24  TOTAL TAX   16,832     ← round(16,832.32), NOT 8,355 + 8,478
```

The line's own printed text says "Add lines 22 and 23". **The form does not add up**, and it is $1 away
from the whole-dollar figure the IRS will recompute (IRM 3.11.3.4.3). For a paper-filing tool, the
printed form *is* the product.

**We are deliberately NOT reporting this** (user decision, 2026-07-14). OTS is on SourceForge, not
GitHub; a project that has stayed there is making a choice about how it wants to be engaged with, and an
unsolicited AI-authored report about a $1 print-layer artifact is a cost to that maintainer, not a gift.
The contrast with `tenforty` is the point: there, the repo was modern and active, and the defects
overcharged real filers by *thousands* (see [[tenforty-upstream-report]]). Magnitude and welcome both
matter. **Do not re-open this without asking.**

---

## Conclusion — SPEC §3.1 stands, unchanged

btctax's cross-footing election is **right**, and no SPEC amendment is needed. The follow-up
`spec-3.1-crossfoot-vs-round-the-total` is **closed**.

But the case is **SUFFICIENT, not DECISIVE**, and it rests where the evidence actually is — in this
order:

**THE LAW (26 USC §6102 + 26 CFR 301.6102-1) is genuinely AMBIGUOUS, and BOTH readings are LAWFUL.**
Rounding is *elective* to begin with. Nothing below the statute can change that — not the instructions,
not the IRM, not the MeF schema, not the IRS's own software. Anyone who tells you this question is
*settled* by an IRS publication has made the mistake this file has now made four times.

Given a lawful choice, btctax elects Reading A because:

1. **§6102(a) + the form's own text.** Line 24 says "Add lines 22 and 23" — the LINES, which the statute
   requires to be entered at whole dollars. The best reading of the statute, though not the only one.
2. **The IRS cannot receive Reading B electronically.** MeF types lines 22, 23 and 24 all as
   `xsd:integer`, so an e-filed return doing round-the-total would contradict its own transmitted
   components. (Evidence, not law — but it means Reading A can never surprise the IRS.)
3. **The IRS's own engine does Reading A**, deliberately, and says so in a code comment.
4. **The filed form visibly adds up.** A human checking it with a calculator finds that it does. For a
   document someone signs under penalty of perjury and mails, that is worth something.

And the $1 is **not an exposure in either direction** (IRM 3.12.3.31.15.5: follow the taxpayer's intent;
the math-error tolerance is redacted). We are right on the merits, not rescued by a penalty — and we were
never at risk from being wrong.

★ **This is a well-founded ELECTION under an ambiguous statute. It is not a compulsion, and it is not
"what the law requires."** Four drafts of this file each over-claimed in one direction or another. Say
what is true and stop there.

### The strongest argument against this conclusion, kept honest

The instruction's literal text is on Reading B's side: line 24 *is* "add[ing] two or more amounts to
figure the amount to enter on a line," with no carve-out on its face. And §301.6102-1(c) can be read
Reading B's way too — line 22's true value ($8,354.59) is, literally, "an item taken into account in
making the computation" of line 24, and the reg's *example* (source receipts) does not necessarily
limit the rule it illustrates. Under that reading, Reading B is the letter and Reading A is a
convention. What defeats it is not the text but the practice: the IRS **cannot** receive cents
electronically, **recomputes** paper returns in whole dollars, and **ships software** that sums rounded
lines and says so in a comment. If the IRS believed Reading B, its own return would not cross-foot.

*(§301.6102-1(b) also permits electing not to round at all — printing every amount with cents. Legal on
paper, exact, and it sidesteps the question entirely; but it forecloses e-filing and is unusual enough
to invite manual review. Not recommended, recorded for completeness.)*
