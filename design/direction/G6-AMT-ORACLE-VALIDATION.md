# §G-6 — the two-oracle validation of the FILED AMT (2026-08-03)

`CLAUDE.md`: *"Tax figures are validated against **two independent engines**, never one… A single
oracle disagreeing is ambiguous; two splitting is diagnostic."* The Form 6251 emitter moved a figure
onto a filed page (Schedule 2 line 2 → 1040 line 17 → line 24) and changed `l18` in the absolute
chain, so the AMT is now a **filed** number and needs both witnesses.

## The vector

The one the real binary filed (`b4_inputs.toml`, fresh vault, no ledger):

| | |
|---|---|
| filing status | Single, no dependents, standard deduction |
| Schedule C | $85,000 non-ledger receipts, $0 expenses ⇒ line 31 = **$85,000** |
| Form 1099-B | LT proceeds $2,500,000 − basis $500,000 ⇒ **$2,000,000** LT gain |
| everything else | no W-2, no interest, no dividends, no W-2 wages / UBIA in the business |

Chosen because it is the shape that actually owes AMT in btctax's input class: a large **preferential**
slice with a modest ordinary one, at an AMTI where the §55(d)(3) exemption is fully phased out.

## The result

| figure | btctax | Tax-Calculator 6.7.2 | OpenTaxSolver 2024 |
|---|---:|---:|---:|
| AGI | 2,078,995 | 2,078,995 | — |
| standard deduction | 14,600 | 14,600 | — |
| **Form 6251 line 2a** (std-ded add-back) | **14,600** | *omitted* ✗ | 14,600 |
| SE tax | 12,010 | 12,010 | 12,010.12 |
| ½-SE deduction | 6,005 | 6,005 | — |
| NIIT | 71,402 | 71,402 | 71,401.81 |
| §199A deduction | **0** | **0** | 12,878.99 ✗ |
| taxable income | 2,064,395 | 2,064,395 | 2,051,515.95 ✗ |
| regular tax (6251 L10) | 386,491 | 386,494 | 383,019.80 |
| tentative minimum tax (L9) | 397,813 | 394,017 | 393,820.94 |
| **AMT (6251 L11 → Sch 2 L2)** | **11,322** | 7,523 ✗ | 10,801.15 ✗ |
| total tax (1040 L24) | 481,225 | 477,429 | 477,232.87 |

Every line either agrees to the dollar or diverges by an amount each engine's **own** documented defect
predicts exactly. Whole-dollar differences on the agreeing lines are btctax's per-line rounding
(SPEC §3.1) against the oracles' float chaining.

## Divergence 1 — Tax-Calculator, and its size is exact

taxcalc omits Form 6251 **line 2a**, the standard-deduction add-back
(PSLmodels/Tax-Calculator#3108, open; `calcfunctions.py` computes
`c62100 = c00100 - e00700 - qbided - standard` and never adds it back). This vector takes the standard
deduction, so taxcalc's AMTI is short by exactly **$14,600**.

The ordinary AMT slice here is AMTI − preferential = 2,078,995 − 2,000,000 = **$78,995**, well below
the $232,600 breakpoint, so the marginal AMT rate is **26%**:

```
14,600 × 26% = 3,796
```

Observed gap in tentative minimum tax: 397,813 − 394,017 = **3,796**. In total tax:
481,225 − 477,429 = **3,796**. To the dollar, in both places.

★ Corrected for its own defect, taxcalc's AMT is 7,523 + 3,796 = **11,319** against btctax's **11,322** —
a $3 float-chaining residue. **That is a genuine independent witness of the filed figure**, not an
excuse: the harness's rule is that a divergence must be of the predicted *mechanism and size*, and this
one is.

## Divergence 2 — OpenTaxSolver is not disagreeing about AMT; it is computing a different return

OTS gets line 2a **right** ($14,600). Its AMT is low because its **taxable income** is low: it claims a
§199A deduction of **$12,878.99** where btctax and Tax-Calculator both say **$0**.

This is `FOLLOWUPS.md` §G-9 and `design/direction/B3-B4-ORACLE-VALIDATION.md` again, unchanged:
`taxsolve_US_1040_2024.c` reads the QBI deduction as a hand-fed input
(`GetLine( "L13", &L[13] )`), so the figure is the *wrapper's*, and the wrapper computes the simplified
Form 8995 — 20% × (taxable income − net capital gain) = 20% × 64,395 = **12,879**. OTS models no
Form 8995-A at all. This filer's taxable income before the QBI deduction is $2,064,395, far above the
top of the phase-in range, so §199A(b)(2) governs: the cap is the greater of 50% × $0 W-2 wages and
25% × $0 + 2.5% × $0 UBIA = **$0**.

The cascade reconciles exactly:

```
taxable income   2,064,395 − 12,879 = 2,051,516   ✓ OTS
6251 line 6      2,051,516 + 14,600 = 2,066,116   ✓ OTS
AMT gap          (397,813 − 393,821) − (386,491 − 383,020) = 3,992 − 3,471 = 521
                 observed: 11,322 − 10,801 = 521                              ✓
```

**A value an oracle takes as INPUT is never validated by its agreement** (§G-9). OTS's AMT *machinery*
agrees with btctax; its §199A input does not, and that input is the only reason the AMT differs.

## What this does and does not establish

**Does:** the AMT btctax now FILES on Schedule 2 line 2 / 1040 line 17 is independently witnessed.
Tax-Calculator reproduces it to $3 once its own open defect is corrected for, and OpenTaxSolver
reproduces the whole Part I/Part III structure — lines 2a, 6, 12, 13, 15–20, 22, 24–34, 38–40 — on a
return that differs only in a §199A input it does not compute. The `l18 = regular_tax + amt.line11`
change is confirmed by both totals moving together with the AMT.

**Does not:** witness lines 2c–2t (unmodelled, censused, reachable only through the §G-22 refusal — the
ISO gap of §G-6 is unchanged by this). Does not witness an AMT-owing MFS vector, where §55(d)(3) puts
both oracles' blind spots at the identical $875,950 and neither can witness (see the witness census).
Does not witness a vector where line 8 (AMTFTC) is non-zero, so the "form required while AMT is $0"
branch of *Who Must File* condition 1 remains held by KAT only. Stated so the silence is not mistaken
for coverage.

---

## Addendum (2026-08-03) — the NINE-DEPENDENT variant, with interest and a mortgage

The committed fixture `crates/btctax-cli/tests/fixtures/examples/nine_dependents_amt_inputs.toml`
adds to the vector above: **9 dependents**, **$45,000** of 1099-INT bank interest, and **$12,000** of
Form 1098 mortgage interest.

| figure | btctax | Tax-Calculator 6.7.2 | OpenTaxSolver 2024 |
|---|---:|---:|---:|
| AGI | 2,123,995 | 2,123,995 | — |
| standard deduction | 14,600 | 14,600 | — |
| itemized (not taken) | 12,000 | 0 (std wins) | — |
| taxable income | 2,109,395 | 2,109,395 | 2,093,595.94 ✗ |
| SE tax | 12,010 | 12,010 | 12,010.12 |
| NIIT | 73,112 | 73,112 | 73,111.81 |
| **Form 6251 line 2a** | **14,600** | *omitted* ✗ | 14,600 |
| **AMT (6251 L11)** | **12,941** | 9,145 ✗ | 12,490.94 ✗ |
| total tax | 496,885 | 493,089 ✗ | 491,987.67 |

**Tax-Calculator** matches AGI, the standard deduction, taxable income, SE tax and NIIT to the dollar.
Its AMT and its total each differ by **exactly $3,796**, and the ordinary AMT slice here is
2,123,995 − 2,000,000 = $123,995, still below the $232,600 breakpoint, so the marginal AMT rate is
26%: `14,600 × 26% = 3,796` — its own open line-2a defect (PSLmodels#3108), unchanged in size by the
new income because the standard deduction did not change. Corrected for it, taxcalc lands on
**12,941** and **496,885** exactly.

**OpenTaxSolver** gets line 2a right and its NIIT agrees. Its taxable income is low by **$15,799**,
which is precisely the simplified-Form-8995 deduction its wrapper hand-feeds
(`GetLine("L13", &L[13])`) and which §199A(b)(2) denies this filer. That cascade accounts for the whole
$450.49 AMT difference: the add-back raises both the tentative minimum tax and the regular tax, and
the AMT is their difference. §G-9 again — a value an oracle takes as INPUT is never validated by its
agreement.

★ **What the mortgage contributes is a REFUSAL, not a deduction.** $12,000 loses to the $14,600
standard deduction, so no Schedule A files and no figure moves. It is in the fixture because omitting
`mortgage_dwelling_is_amt_qualified` refuses the entire return with `AmtQualifiedDwellingUnanswered`
and writes zero bytes — i6251 line 3 adds back interest on a dwelling that is not AMT-qualified, and
guessing would understate the tax. Deleting that one line is a live kill-test, verified red.
