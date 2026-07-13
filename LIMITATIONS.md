# btctax — LIMITATIONS & Supported Forms (full-return v1)

**Version:** full-return v1 · **Tax year supported: TY2024 only** · Federal only · Print-and-mail (no e-file)

**This is not tax advice.** `btctax` is a mechanical calculator: it computes from the figures you give it and
the ledger you reconciled. It does not interpret your facts, and it is not a substitute for a tax
professional. Every full return it produces is a **DRAFT** (watermarked) and requires your attestation
before export. No Paid Preparer / PTIN is filled — this is a self-prepared return.

## The one rule that governs everything: fail closed

A wrong return is worse than a refusal. **Any in-scope line that cannot be computed, and any input you
captured that is not modeled but could change a reported figure, makes the return `NotComputable` — never a
silent $0 or a plausible-but-wrong number.** The single carve-out is a *purely taxpayer-favorable* benefit
this version deliberately omits: those are **omitted conservatively** (they can only OVERSTATE your tax,
never understate it), with a loud advisory. The three lists below are exactly that split.

---

## Supported: what a v1 full return covers

**Household shape:** a "common W-2 household" — wages, bank/brokerage interest and dividends, unemployment,
and the crypto activity `btctax` already tracks (including a crypto miner/staker operating as a business).

**Income:** W-2 wages (multi-employer, both spouses) · 1099-INT · 1099-DIV (ordinary, qualified,
capital-gain distributions, §199A REIT dividends) · 1099-G unemployment · crypto capital gains (the existing
8949/Schedule D pipeline) · crypto **ordinary** income — hobby/other → Schedule 1 line 8v; business/
self-employment → **Schedule C** → Schedule 1 line 3.

**Deductions:** standard (basic + §63(f) aged/blind + the §63(c)(5) dependent floor) vs **Schedule A**
(medical over the 7.5% floor · SALT with the §164(b)(5) income-or-sales election, capped $10,000/$5,000 MFS ·
home-mortgage interest, Form 1098 line 8a · charitable with the §170(b) class ceilings and 5-year
class/vintage carryover, including crypto donations at their §170(e) value).

**Self-employment:** Schedule SE + §1401 SE tax on business crypto income · the §164(f) one-half-SE deduction ·
the 0.9% Additional Medicare on SE income via **Form 8959 Part II** (the §6017 $400 floor is honored).

**QBI:** the simplified **Form 8995** path for §199A REIT dividends (1099-DIV box 5), with the REIT/PTP loss
carryforward.

**Credits:** §904(j) **foreign tax credit** (≤ $300 / $600 MFJ, passive, 1099-reported — no Form 1116) ·
**excess Social Security** credit (multi-employer, per person).

**Other taxes:** **Form 8960** Net Investment Income Tax (rebuilt from your line items) · **Form 8959**
Additional Medicare Tax, Parts I (wages), II (SE) and V (withholding).

**Forms — computed vs. filled.** Two different things, and the difference matters:

- **Computed** (every line, to the cent): 1040 · Schedules 1, 2, 3, A, B, C, D, SE · Forms 8949, 8283, 8959,
  8960, 8995. These figures appear on the report.
- **Filled as an official IRS PDF** (`export-irs-pdf`): **1040 · Schedule D · Schedule SE · Form 8949 · Form
  8283**. The remaining schedules and forms (1, 2, 3, A, B, C, and Forms 8959, 8960, 8995) are **computed but
  not yet PDF-filled** — their figures are on the report, and until the fillers ship you must transcribe them
  onto the official forms yourself.

**Carryovers:** charitable (per class + vintage) and the QBI REIT/PTP loss carryforward are computed and can
be written forward to next year with `btctax report --tax-year Y --write-carryover`. A carryover you typed in
yourself is never silently overwritten (pass `--force` if you mean to).

---

## (i) OMISSIONS — favorable-only, omitted conservatively (your tax is OVERSTATED at worst)

These are benefits you may be entitled to that v1 does **not** compute. Leaving them out can only make your
tax **higher** than it should be — never lower — so the return is still safe to rely on as an upper bound.
**Each fires a loud advisory on the report.** If any apply to you, claim them yourself (or see a
professional) before filing.

| Omitted | What it would do | What to do |
|---|---|---|
| **Child Tax Credit / Credit for Other Dependents** (Schedule 8812) | Up to $2,000 per qualifying child; $500 per other dependent. 1040 line 19 is pinned to **$0**. | File Schedule 8812 yourself. Your tax is overstated by up to that amount. |
| **Earned Income Credit** | A refundable credit for lower-income working households. | Check EIC eligibility (Pub. 596) yourself. |
| **Education credits** (AOTC / Lifetime Learning), **dependent-care** (Form 2441), **saver's**, **energy**, **adoption** credits | Various nonrefundable/refundable credits. | Claim separately if eligible. |
| **Direct deposit** of a refund (1040 lines 35b–35d) | Faster refund. | Left blank — you will receive a **paper check**. |

## (ii) REFUSALS — v1 stops rather than guess (`NotComputable`)

If any of these appear in your inputs or ledger, `btctax` **refuses to produce a return**. This is
deliberate: each is something that could make the return *wrong* (usually by understating tax), and v1
cannot model it correctly.

**From your inputs:**

- **Negative money anywhere.** Every captured amount is a form-box magnitude (≥ 0); a negative is a corrupt import.
- **A 1099-DIV whose box 1b (qualified) or box 5 (§199A) exceeds its box 1a** (ordinary) — box 1b/box 5 are subsets of box 1a.
- **A spouse-owned W-2 or Schedule C on a non-joint return.**
- **Foreign trust** (`foreign_trust = true`) → Form 3520.
- **Schedule B filed but its Part III (foreign accounts / foreign trust) unanswered** — v1 will not guess a disclosure answer.
- **A Schedule A sales-tax amount with the §164(b)(5) sales-tax election OFF** (a silent drop would hide an input error).
- **MFS without stating whether your spouse itemizes** (§63(c)(6) couples the choice).
- **A charitable contribution to a non-50% organization** (private foundation etc. — the Pub. 526 "special 30% limit" ordering is unmodeled).
- **A claimable-as-dependent spouse** (it limits the joint standard deduction).
- **A W-2 box-12 code outside the inert allowlist** `{D,E,F,G,H,S,AA,BB,EE,DD}` — e.g. **W** (HSA), **K**, **R**, **T**, **Z**.
- **Elective deferrals (box 12 D/E/F/G/S) over the §402(g) limit** ($23,000 for TY2024) for one person.
- **W-2 box 8** (allocated tips → Form 4137) or **box 10** (dependent-care → Form 2441).
- **1099-INT box 9 / 1099-DIV box 13** (private-activity-bond interest — an AMT preference).
- **1099-DIV box 2b / 2c / 2d** (unrecaptured §1250, §1202, 28% collectibles → the Schedule D Tax Worksheet).
- **Foreign tax above the §904(j) ceiling** ($300 / $600 MFJ) or non-passive → Form 1116.
- **A single employer over-withholding Social Security** (not creditable — recover it from the employer).
- **Schedule 1 line 13** (HSA → Form 8889) or **line 20 with an IRA deduction claimed** (the active-participant phase-out is unmodeled).

**From the computation / your ledger:**

- **Business-flagged crypto `Interest`** — §1402(a)(2) excludes it from SE tax yet it is not sheltered from NIIT; it has no clean home in v1.
- **SE-eligible business crypto income with no Schedule C** (owner and description are unknowable).
- **A Schedule C loss** (net < 0) — §465 at-risk substantiation is out of scope.
- **Form 8615 "kiddie tax"** — a claimable-as-dependent filer with unearned income over the §1(g) threshold ($2,600) must be taxed at the parent's rate.
- **QBI above the §199A(e)(2) threshold** ($191,950 / $383,900 MFJ) — the Form 8995-A phase-in is unmodeled.
- **The AMT screen trips.** v1 does **not compute the Alternative Minimum Tax**. It runs the official 2024 *"Worksheet To See if You Should Fill in Form 6251"*; if that worksheet says you may owe AMT, the return is **refused**. If the worksheet clears you, Schedule 2 line 2 is $0 — which is a sound conclusion, because the worksheet deliberately over-estimates.
- **Taxable income ≤ $0 with a capital-loss carryforward** — the §1211/§1212 Capital Loss Carryover Worksheet edge is unmodeled. (A refund-only filer with *no* carryforward is fine: tax = $0, withholding refunded.)

## (iii) UNREPRESENTABLE — no input exists (would refuse if it did)

There is nowhere to enter these, and a return that needs them is out of scope:

- **Retirement / pension / IRA / annuity / Social Security income** (1040 lines 4a–6b; 1099-R, SSA-1099).
- **Marketplace health coverage — Form 1095-A / excess advance premium tax credit** (Schedule 2 line 1a). There is no input for it; if there were, it would refuse (repaying excess APTC *increases* tax, so omitting it would understate).
- **Schedule E** (rental, royalty, partnership/S-corp K-1) and **Schedule F** (farm).
- **A non-crypto Schedule C** (any self-employment other than the crypto trade/business).
- **A second self-employed earner.** v1 models exactly one Schedule C; there is no way to represent a second SE earner's business.
- **State and local returns** — federal only.
- **E-filing** — print and mail.
- **Any tax year other than TY2024**, and the TY2025 Schedule 1-A.
- **Any line requiring a worksheet v1 does not model.**

---

## Conservative simplifications (they overstate, never understate)

- **Form 8960 (NIIT), Part II — the state/local tax allocation is omitted.** Properly allocated state income tax attributable to net investment income would *reduce* NII. Omitting it can only make your NIIT **higher**.
- **A `None` date of birth is treated as "not 65."** The §63(f) additional standard deduction ($1,550 / $1,950 per box) is forfeited rather than granted on an unsubstantiated birthdate. If you are 65+, enter your DOB.
- **The crypto-delta figure's deduction is fixed at derivation time.** The "tax attributable to crypto" number and the absolute filed return answer **different questions** and are never reconciled to the dollar — see the §6 note the report prints.

## Advisories the report will show you

- **FBAR / FinCEN.** Under FinCEN Notice 2020-2, accounts holding *only* virtual currency are (for now) outside the FBAR requirement — but this is under active reconsideration, and an account holding crypto **plus** fiat or securities may well be reportable. `btctax` **never auto-answers Schedule B Part III** for you. Decide, and answer, yourself.
- **Charitable donee class.** The ledger classifies a crypto donation assuming a **public charity (50% organization)** donee — long-term gifts at FMV under the 30% ceiling. If your donee is a **private foundation**, the correct treatment is the 20% ceiling at *basis*, which v1 refuses. Verify who you gave to.
- **Qualified appraisal.** A year's BTC donations totaling **more than $5,000** need a qualified appraisal and Form 8283 Section B (CCA 202302012: crypto does *not* get the readily-valued exception).

---

## Legal

Mechanical calculation only (no unauthorized practice of law/accountancy). Every full return is a **DRAFT**
until you attest. Not tax advice. Licensed permissively (MIT OR Unlicense); clean-room implementation from
primary sources (the Internal Revenue Code, IRS forms and instructions, and the applicable Revenue
Procedures) — no GPL-derived tax logic.
