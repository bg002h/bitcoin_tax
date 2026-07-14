# btctax — LIMITATIONS & Supported Forms (full-return v1)

**Version:** full-return v1 · **Tax year supported: TY2024 only** · Federal only · Print-and-mail (no e-file)

**This is not tax advice.** `btctax` is a mechanical calculator: it computes from the figures you give it and
the ledger you reconciled. It does not interpret your facts, and it is not a substitute for a tax
professional. No Paid Preparer / PTIN is filled — this is a self-prepared return.

**On DRAFT watermarking and attestation, precisely.** The watermark + attestation gate applies when the ledger
is **pseudo-reconciled** (a synthetic, non-persisted default is contributing to the projection): such an export
is stamped `ESTIMATE, NOT FOR FILING` and refuses without the exact attestation phrase. Pseudo figures are
FICTIONAL and can never be filed, so that gate dominates everything else. An export from a fully-real ledger —
including the **full-return packet** — is **not** watermarked and needs no attestation: it is a clean, filable
document, and it is yours to review and sign.

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
home-mortgage interest reported on Form 1098, Schedule A line 8a · charitable with the §170(b) class ceilings and 5-year
class/vintage carryover, including crypto donations at their §170(e) value).

**Self-employment:** Schedule SE + §1401 SE tax on business crypto income · the §164(f) one-half-SE deduction ·
the 0.9% Additional Medicare on SE income via **Form 8959 Part II** (the §6017 $400 floor is honored).

**Adjustments to income (Schedule 1 Part II):** the §164(f) one-half-SE-tax deduction (line 15) · the
**student-loan interest deduction** (line 21), including its §221(b)(2) MAGI phase-out · the **early-withdrawal
penalty** from 1099-INT box 2 (line 18).

**Other income (Schedule 1 Part I):** a **taxable state or local income-tax refund** (line 1, §111 tax-benefit
rule) · crypto ordinary income with no other home (line 8v).

**QBI:** the simplified **Form 8995** path for §199A REIT dividends (1099-DIV box 5), with the REIT/PTP loss
carryforward.

**Credits:** §904(j) **foreign tax credit** (≤ $300 / $600 MFJ, passive, 1099-reported — no Form 1116) ·
**excess Social Security** credit (multi-employer, per person).

**Other taxes:** **Form 8960** Net Investment Income Tax (rebuilt from your line items) · **Form 8959**
Additional Medicare Tax, Parts I (wages), II (SE) and V (withholding).

**Forms — computed vs. filled.** Two different things, and the difference matters:

- **Filled as an official IRS PDF** (`export-irs-pdf`), for a year with full-return inputs: **the whole
  packet** — Form 1040 and Schedules 1, 2, 3, A, B, C, D, SE, plus Forms 8949, 8959, 8960, 8995 and (when
  required) 8283. They come out in IRS **Attachment Sequence No.** order with a `manifest.txt`, which is your
  stapling order.
- **The packet is ALL-OR-NOTHING.** If any form cannot be filled correctly, **nothing is written** — you never
  find a half-packet on disk. A 1040 whose line 2b cites a Schedule B that is not attached is a wrong return.
- **A year with no full-return inputs** still gets the **crypto slice** (Form 8949 + Schedule D from the
  ledger's disposals, Schedule SE, Form 8283, and the 1040's capital-gain cluster). The packet's files are
  **sequence-prefixed** (`00_f1040.pdf`, `01_f1040s1.pdf`, … `12A_f8949.pdf`) and the slice's are not, so the
  two name-spaces are disjoint: running both into one directory cannot shuffle a cents form from one into the
  whole-dollar packet of the other. A test runs both pipelines into one directory and asserts every packet
  file is still **byte-for-byte** what the packet wrote — and it fails if the guarantee is removed.

**Whole dollars, and why the report agrees with the PDF.** The filed packet takes the IRS's round-all-amounts
election (2024 i1040 p. 23): every printed line is rounded at the line, and every printed total sums the
already-rounded lines, so each form cross-foots and each form agrees with the ones it cites. The **report now
prints those same whole-dollar figures**, so the "amount you owe" on your screen is the "amount you owe" on the
form. Internally the engine still computes in exact cents; the difference between the two is at most a few
dollars, and the **filed figure is the one the forms are built from**.

One consequence worth knowing: because Form 8949's rows are each rounded to whole dollars, the total gain on a
many-row year can differ by a few dollars from rounding the exact total once. That is what electing whole
dollars *means* — it is exactly what a human rounding by hand produces — and it is the figure that makes
Schedule D add up against the 8949 behind it.

**The §199A deduction on your mining business IS computed.** If your crypto activity is a trade or business
(Schedule C), you are entitled to a **qualified business income deduction of up to 20%** of that profit
(Form 8995), and btctax computes it — the QBI base is your Schedule C net profit less the deductible half of
your self-employment tax. Above the §199A(e)(2) income threshold ($191,950 single / $383,900 joint for
TY2024) the simplified Form 8995 no longer applies and btctax **refuses** rather than guess at the
wage-and-property limits that take over (that is Form 8995-A, which it does not fill).

**What the packet still will not do for you:**
- **Non-crypto NONCASH gifts over $500 REFUSE.** Form 8283 must list the property (description, how acquired,
  date, appraiser), and btctax holds none of those details for property that did not come from your ledger. It
  will not attach an 8283 that under-reports its own property list and put your deduction at risk — complete
  that form by hand, or remove the gift.
- **More than 14 interest payers / 15 dividend payers on Schedule B, or more than 4 dependents on the 1040,
  REFUSE.** Those grids are full, and the IRS's remedy is a continuation statement this version cannot
  generate. It will not silently print a subset of your payers or your children.
- **The spouse's Identity Protection PIN is not captured** (yours is, and it prints). If your spouse has one,
  write it on the form.
- **A HoH/QSS qualifying person who is not one of your listed dependents** is not captured; that cell is left
  blank for you to complete.

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
- **"Someone can claim you as a dependent" unanswered** — required on **every** return. It selects the §63(c)(5) dependent standard-deduction floor over the basic standard deduction, gates the §1(g)/Form-8615 kiddie-tax refusal, and is a **checkbox printed on your 1040**. There is no safe default: guessing "no" *understates* your tax and files an unchecked box you never affirmed; guessing "yes" overstates it. Only you know. (Likewise for your spouse, on a return that has one.) **Answer it with `btctax income answer --year N`** — no TOML file needed.

  ⚠️ **A return stored by v0.2.0 will newly refuse on this question.** That is the bug surfacing, not a
  regression: v0.2.0 stored the flag as a plain true/false defaulting to *false*, so a return nobody was
  ever asked about was recorded as though the answer were "no". Those cannot be told apart from a real
  "no", and "no" is the reading that understates tax — so a v0.2.0 answer is treated as **unasked**, and
  the year refuses until it is answered. A stored "yes" is preserved (nothing ever defaulted to "yes").
- **A Schedule A sales-tax amount with the §164(b)(5) sales-tax election OFF** (a silent drop would hide an input error).
- **MFS without stating whether your spouse itemizes** (§63(c)(6) couples the choice).
- **A charitable contribution to a non-50% organization** (private foundation etc. — the Pub. 526 "special 30% limit" ordering is unmodeled).
- **A claimable-as-dependent spouse** (it limits the joint standard deduction).
- **A W-2 box-12 code outside the inert allowlist** `{D,E,F,G,H,S,AA,BB,EE,DD}` — e.g. **W** (HSA), **K**, **R**, **T**, **Z**.
- **Elective deferrals (box 12 D/E/F/G/S) over the §402(g) limit** ($23,000 for TY2024) for one person.
- **W-2 box 8** (allocated tips → Form 4137) or **box 10** (dependent-care → Form 2441).
- **1099-INT box 9 / 1099-DIV box 13** (private-activity-bond interest — an AMT preference).
- **1099-DIV box 2b / 2c / 2d** (unrecaptured §1250, §1202, 28% collectibles → the Schedule D Tax Worksheet).
- **Foreign tax above the §904(j) ceiling** ($300 / $600 MFJ) → Form 1116.
- **A single employer over-withholding Social Security** (not creditable — recover it from the employer).
- **Schedule 1 line 13** (HSA → Form 8889) or **line 20 with an IRA deduction claimed** (the active-participant phase-out is unmodeled).

**From the computation / your ledger:**

- **Business-flagged crypto `Interest`** — §1402(a)(2) excludes it from SE tax yet it is not sheltered from NIIT; it has no clean home in v1.
- **SE-eligible business crypto income with no Schedule C** (owner and description are unknowable).
- **A Schedule C loss** (net < 0) — §465 at-risk substantiation is out of scope.
- **Form 8615 "kiddie tax"** — a claimable-as-dependent filer with unearned income over the §1(g) threshold ($2,600) must be taxed at the parent's rate.
- **Taxable income (before the QBI deduction) above the §199A(e)(2) threshold** ($191,950 / $383,900 MFJ) — the Form 8995-A phase-in is unmodeled. (It is the taxable-income figure that is tested, not the QBI itself.)
- **The AMT screen trips.** v1 does **not compute the Alternative Minimum Tax**. It runs the official 2024 *"Worksheet To See if You Should Fill in Form 6251"*; if that worksheet says you may owe AMT, the return is **refused**. If the worksheet clears you, Schedule 2 line 2 is $0 — which is a sound conclusion, because the worksheet deliberately over-estimates.
- **Taxable income ≤ $0 with a capital-loss carryforward** — the §1211/§1212 Capital Loss Carryover Worksheet edge is unmodeled. (A refund-only filer with *no* carryforward is fine: tax = $0, withholding refunded.)

## (iii) UNREPRESENTABLE — no input exists (would refuse if it did)

There is nowhere to enter these, and a return that needs them is out of scope:

- **Retirement / pension / IRA / annuity / Social Security income** (1040 lines 4a–6b; 1099-R, SSA-1099).
- **Marketplace health coverage — Form 1095-A / excess advance premium tax credit** (Schedule 2 line 1a). There is no input for it; if there were, it would refuse (repaying excess APTC *increases* tax, so omitting it would understate).
- **Schedule E** (rental, royalty, partnership/S-corp K-1) and **Schedule F** (farm).
- **A non-crypto Schedule C** (any self-employment other than the crypto trade/business).
- **A second self-employed earner.** v1 models exactly one Schedule C; there is no way to represent a second SE earner's business.
- **Non-passive foreign tax** (a Form 1116 category other than passive). The only foreign-tax inputs are
  1099-INT box 6 and 1099-DIV box 7, which are passive by construction — so there is no way to *enter* a
  non-passive foreign tax. If there were, it would refuse.
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

### No authorisation to file

`btctax` is a **mechanical calculator**. It computes figures from the numbers you give it and the ledger you
reconciled, and it can fill official IRS forms with those figures.

**No right is granted, and no authorisation is given, to use this software — or anything it produces — to
prepare or file a tax return.** Nothing in the MIT or Unlicense grant is, or may be read as, an
authorisation, an endorsement, a certification, or a representation that this software or its output is fit,
complete, or correct for filing with any tax authority. The permissive licence grants you broad rights over
the *software*; it says nothing whatever about whether the *output* is fit to file. Those are different
questions, and only the second one matters to the IRS.

### No warranty of fitness for filing

The authors and contributors make **no representation and give no warranty** — express, implied, statutory or
otherwise — that any figure, form, schedule or PDF produced by this software is accurate, complete, current,
compliant, or suitable for submission to the Internal Revenue Service or any other tax authority. This is in
addition to, and does not limit, the general warranty disclaimer in the licence.

The software may be wrong. It may be silently wrong. It refuses in many cases where it cannot be sure (every
one of them is listed above), but **a refusal is a best effort, not a guarantee, and the absence of a refusal
is not a certification.**

### You are the preparer

If you file a return, in whole or in part, on the basis of anything this software produced: you do so
**entirely on your own responsibility**; **you** are the preparer of that return and are solely responsible
for its accuracy, completeness and timeliness; you are responsible for reviewing every figure and every form
against the Internal Revenue Code, the IRS forms and instructions, and your own facts, before you sign
anything; and the authors and contributors accept **no liability** of any kind for any tax, interest,
penalty, addition to tax, professional fee, loss or other consequence arising from that filing.

No Paid Preparer is identified and no PTIN is filled, because there is none: this is a self-prepared return.
The signature on it is yours alone.

### Not tax advice

Nothing produced by this software, and nothing in its documentation, is tax, legal or accounting advice, and
none of it is a substitute for a qualified professional. This software does not interpret your facts. It does
not know your circumstances. It does arithmetic on what you tell it. If your situation is not simple, or if
you are unsure, consult a professional.

### Licence

Licensed permissively (**MIT OR Unlicense**) — unchanged and unrestricted. The clauses above are a NOTICE
(see the `NOTICE` file); they disclaim authorisation, warranty and liability. They do **not** restrict the
licence grant and do not purport to forbid anything. Clean-room implementation from primary sources (the
Internal Revenue Code, IRS forms and instructions, and the applicable Revenue Procedures) — no GPL-derived
tax logic.
