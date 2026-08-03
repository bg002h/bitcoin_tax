# §G-28/B3 + B4 — the two-oracle validation (2026-08-03)

`CLAUDE.md`: *"Tax figures are validated against **two independent engines**, never one… A single oracle
disagreeing is ambiguous; two splitting is diagnostic."* B3 (non-ledger Schedule C receipts) and B4
(Form 1099-B totals) both change money that flows into the SE-tax and capital-gain stacks, so unlike
§199A — where OpenTaxSolver is structurally blind — **both oracles genuinely compute these figures** and
the rule can be met.

## The vector

The one the real binary filed (`b4_small.toml`), chosen so **both features are live at once**: B3
changes the SE base and B4 changes the capital-gain stack, and they interact through AGI, which is the
threshold for both §1411 NIIT and §1401(b)(2) Additional Medicare.

| | |
|---|---|
| filing status | Single, no dependents, standard deduction |
| Schedule C | $240,000 mined (ledger) + **$85,000 non-ledger consulting** (B3), $0 expenses ⇒ line 31 = **$325,000** |
| Form 1099-B | long-term proceeds $260,000 − basis $200,000 (B4) ⇒ **$60,000** LT gain on Schedule D line 8a |
| everything else | no W-2, no interest, no dividends |

## The result

| figure | btctax | Tax-Calculator 6.7.2 | OpenTaxSolver 2024 |
|---|---:|---:|---:|
| AGI | 370,195 | 370,195 | 370,194.81 |
| standard deduction | 14,600 | 14,600 | 14,600.00 |
| **Schedule D → 1040 line 7** | **60,000** | 60,000 | **60,000.00** |
| SE tax | 29,610 | 29,610 | 29,610.39 |
| ½-SE deduction | 14,805 | 14,805 | — |
| Schedule SE line 10 (OASDI) | 20,906 | — | 20,906.40 |
| Form 8959 Additional Medicare | 901 | — | 901.24 |
| NIIT | 2,280 | 2,280 | 2,280.00 |
| income tax before credits | 82,833 | 82,833 | — |
| taxable income | 355,595 | 355,595 | 296,475.85 ✗ |
| **§199A deduction** | **0** | **0** | 59,118.96 ✗ |
| total tax | 115,624 | 115,625 | — |

Whole-dollar differences are btctax's per-line rounding (SPEC §3.1) against the oracles' float chaining;
the $1 on the total is that, compounded.

## The one divergence, adjudicated against the FORM

OTS reports a **$59,118.96** §199A deduction where btctax and Tax-Calculator both report **$0**. Two
oracles split, which this repo treats as diagnostic rather than ambiguous — so it was adjudicated
against the form, not averaged.

**btctax is right, and OTS is not disagreeing — it is not answering.** `taxsolve_US_1040_2024.c` reads
the QBI deduction as a hand-fed input:

```c
GetLine( "L13", &L[13] );   /* Qualified business income deduction. */
```

So the 59,118.96 is the *wrapper's* figure, and the wrapper computes the SIMPLIFIED Form 8995 —
20% × (taxable income − net capital gain) = 20% × 295,594.81 — because OTS models no Form 8995-A at
all. This filer's taxable income before the QBI deduction is $355,595, which is **above** the top of
the §199A phase-in range ($241,950 single), so Form 8995-A Part II governs:

* line 3 = 20% of QBI
* line 10 = greater of (50% × $0 W-2 wages) and (25% × $0 + 2.5% × $0 UBIA) = **$0**
* line 11 = smaller of line 3 and line 10 = **$0**

A sole proprietor with no employees and no qualified property is capped at zero. That is the whole
point of B1b, and Tax-Calculator — which does model 8995-A — agrees with btctax exactly.

★ This is `FOLLOWUPS.md` §G-9 confirmed empirically rather than merely asserted: **a value an oracle
takes as INPUT is never validated by its agreement**, and here it is not even a disagreement worth the
name. The §199A row of this table is the only one OTS cannot witness, and it is exactly the row its own
source says it does not compute.

## What this does and does not establish

**Does:** B3's non-ledger receipts reach the Schedule C net, the SE base, the §1401(a) OASDI cap, the
§1401(b)(2) Additional-Medicare threshold and AGI at the same figures two independent engines compute.
B4's 1099-B totals reach 1040 line 7 and the NIIT base identically. The routing of both around the
FROZEN engine files (`compute_se_tax`'s expenses argument for B3; `net_1222`'s arguments for B4)
produces the same numbers as engines that never had that constraint.

**Does not:** exercise a 1099-B **loss**, the §1211(b) limit with securities, short-term 1099-B gain, a
1099-B against a crypto gain of the other character, or the phase-in range. Those are held by KATs
(`a_1099b_loss_nets_within_character_and_hits_the_1211_limit`,
`non_ledger_receipts_reach_schedule_se_exactly`) and are unwitnessed by any oracle. Stated so the
silence is not mistaken for coverage.
