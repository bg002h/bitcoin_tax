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

## 1. The operative authority is a REGULATION, not the instruction

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

## 2. Every e-filed 1040 is integer-only — cents cannot even be transmitted

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

## 3. The IRS's OWN software implements Reading A — with a comment saying so

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

## 4. The IRM: confirms the whole-dollar regime for lines 1–23 — and is ADVERSE on line 24

★ **An earlier version of this file claimed the IRM "inverts the exposure" in btctax's favour — that a
Reading-A return reproduces the IRS's recomputation of line 24 exactly and "cannot draw a math-error
notice", while OTS is off by $1 "by construction". THAT WAS FALSE**, and it was false in the direction
that flattered us. The reviewer caught it. What the IRM actually says:

**It supports cross-footing for lines 1–23** — [IRM 3.11.3](https://www.irs.gov/irm/part3/irm_03-011-003r):

> **IRM 3.11.3.14.2:** "All lines on Form 1040 are edited in **dollars only** except lines 24 through 38."
>
> **EC 369:** "Verify that dollars and cents were **not** entered into dollar-only fields."

So when the IRS recomputes the **subtotals** — lines 9, 11, 15 — it is necessarily summing whole-dollar
values. That is genuine (if inferential) support for §3.1 across most of the return.

**But it does NOT support us on line 24, which is the line we kept building on:**

> **IRM 3.11.3.14.2.28:** line 24 is edited **"as dollars and cents."**

So the IRS transcribes line 24 *with cents*, and no inversion argument is available there.

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

1. **MeF (airtight).** Lines 22, 23 and 24 are all `xsd:integer`. Reading B is unrepresentable on the
   IRS's own wire format; an e-filed return doing it would contradict its own transmitted components.
2. **IRS Direct File (strong).** The IRS's own engine deliberately sums rounded values, says so in a
   code comment, and ships a fixture whose expected output is **$7** off a true sum in order to keep the
   printed form adding up.
3. **26 CFR 301.6102-1 (supportive, not dispositive).** Structure and the paragraph-(b) election lean our
   way; the text does not compel the result.
4. **The IRM (partial).** Confirms dollar-only transcription for lines **1–23**. Is **adverse** on line
   24, which it treats as a cents line — and forecloses the $1 exposure in **both** directions anyway
   (follow the taxpayer's intent).

The difference between "sufficient" and "decisive" is exactly what made this citation wrong three times.
It is stated as sufficient.

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
