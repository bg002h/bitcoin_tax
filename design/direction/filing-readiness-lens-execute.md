# EXECUTE lens — I built the vector and ran it through the binary

Binary: `/scratch/code/bitcoin_tax/target/debug/btctax` v0.17.0, rebuilt from `f5ba41a1`
(`cargo build --bin btctax` → `Finished dev profile in 24.85s`, no source edits). Everything below
ran in `/tmp/btctax-execute-lens/`. **The repo was not touched** (`git status --short` empty at the
end).

Env for every run: `BTCTAX_PASSPHRASE=pw TZ=UTC LC_ALL=C`.

---

## VERDICT — can this slice FILE today?

**It FILES CLEAN, and both oracles witness every headline figure to the dollar.** The vector as
literally specified — MFJ, $1,000,000 wages, $1,000,000 long-term bitcoin gain, $1,000,000 cash gift
to a church, 5 dependents, a mortgage, SALT, student-loan interest — is *not* a refusal and is not
wrong. `btctax report --tax-year 2024` exits 0 and `btctax export-irs-pdf` writes an 8-form packet
plus a dependents continuation statement and a stapling manifest. OpenTaxSolver 2024 and PSL
Tax-Calculator both reproduce AGI, itemized deduction, taxable income, regular tax, AMTI, AMT, NIIT,
Additional Medicare Tax and total tax **exactly**.

The one thing the vector's premise gets wrong is its own AMT: **there is no AMT on this household,
and that is the correct answer.** A $1,000,000 charitable deduction is fully allowed for the AMT, so
AMTI is only $10,000 above taxable income (the capped SALT add-back at line 2a), TMT $124,045 <
regular $148,705, and Form 6251 is correctly *not* attached. Post-TCJA, with SALT capped at $10,000,
this household shape has essentially no AMT lever left except an ISO exercise — and an ISO exercise
is the one thing that makes btctax **refuse the whole return** (verbatim message below). So the
honest statement of this slice is: *it files when there is no AMT, and it refuses rather than
understates when there is one it cannot see.* The gaps that remain are all in the
**overstates-your-tax / forgone-benefit** direction, plus one silent loss of a $400,000 carryover
figure.

---

## The runs

### Inputs

`/tmp/btctax-execute-lens/coinbase.csv` (Coinbase export shape, copied from
`crates/btctax-cli/src/testonly.rs:60`):

```
mv-buy,2020-01-01 12:00:00 UTC,Buy,BTC,20.00000000,USD,10000.00,200000.00,200000.00,0.00,,,
mv-sell,2024-06-03 12:00:00 UTC,Sell,BTC,20.00000000,USD,60000.00,1200000.00,1200000.00,0.00,,,
```

`/tmp/btctax-execute-lens/megavector.toml` — built from
`crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` + `nine_dependents_amt_inputs.toml`.
Salient entries: `filing_status = "Mfj"`, one W-2 `box1_wages = "1000000"` /
`box2_fed_withheld = "300000"` / `box5_medicare_wages = "1000000"` / `box6_medicare_withheld = "21700"`
/ `box17_state_tax_withheld = "60000"`, 5 `[[header.dependents]]`, `[schedule_a]` with
`mortgage_interest_1098 = "30000"`, `salt_state_estimated_payments = "60000"`,
`salt_real_estate = "20000"`, one `[[schedule_a.charitable]]` `class = "cash60"` /
`amount = "1000000"`, and `[sch1] student_loan_interest_paid = "2500"`.

### Commands, verbatim

```
$ btctax --vault v.pgp init --key-backup key-backup.asc
Initialized vault v.pgp (key backed up to key-backup.asc)

$ btctax --vault v.pgp import coinbase.csv
Import:
  coinbase [coinbase.csv]: parsed 2 rows -> 2 BTC events (0 dropped no-BTC, 0 unclassified)
  appended 2 | duplicates 0 | NEW import-conflicts 0

$ btctax --vault v.pgp verify          # 0 hard blockers, 1 advisory (Pre2025MethodNote)

$ btctax --vault v.pgp income import --year 2024 --file megavector.toml
Imported full-return inputs for tax year 2024.

$ btctax --vault v.pgp report --tax-year 2024        # EXIT 0
```

### The figures (VARIANT A — the vector as specified)

```
═══ Absolute filed return (Form 1040) — tax year 2024 ═══
  Profile source: ReturnInputs (derived from line items)
  Total income (1040 L9):   2000000.00
  Adjustments (L10):        0.00
  AGI (L11):                2000000.00
  Deduction (L12, itemized): 1040000.00
  Taxable income (L15):     960000.00
  Tax (L16):                148705.00
  Additional Medicare (Form 8959 → Sch 2 L11): 6750.00
  Net Investment Income Tax (Form 8960 → Sch 2 L12): 38000.00
  Alternative Minimum Tax (Form 6251 → Sch 2 L2): 0.00
    AMTI (L4) 970000.00 · exemption (L5) 133300.00 · tentative minimum tax (L9) 124045.00 · regular tax (L10) 148705.00
    Line 7 does not exceed line 10 → no Form 6251 attachment is required.
  TOTAL TAX (L24):          193455.00
  Total payments (L33):     307200.00
  → REFUND (L35a):          113745.00
```

Adjustments = $0 because §221 phases the $2,500 of student-loan interest out entirely at this MAGI
(`return_1040.rs:938 student_loan_deduction`) — correct, not a drop.

### The PDF export

```
$ btctax --vault v.pgp export-irs-pdf --out irs --tax-year 2024    # EXIT 0

Full-return packet — 8 form(s), in IRS Attachment Sequence order:
  irs/00_f1040.pdf
  irs/02_f1040s2.pdf
  irs/07_f1040sa.pdf
  irs/12_schedule_d.pdf
  irs/12A_f8949.pdf
  irs/71_f8959.pdf
  irs/72_f8960.pdf
  irs/dependents_statement.txt
  irs/manifest.txt  ← your stapling order
```

No Schedule 1 (no additional income, $0 adjustments), no Schedule 3 (no credits, no extension
payment), no Schedule B (no interest/dividends), no Schedule 8812 — each correct for this vector.
The dependents statement carries child 5 with the page-1 "more than four dependents" box checked
(1040 field `c1_13[0] = /1`).

AcroForm read-back (`qpdf --qdf` + a field dumper; `/tmp/btctax-execute-lens/dumpfields.py`) confirms
the pages actually carry the figures — 1040 L9 `1000000`+`1000000`, L11 `1000000`, L12 `1040000`,
L15 `960000`, L16 `148705`, L23 `44750`, L24 `193455`, L33 `307200`, L35a `113745`; Schedule 2
L11 `6750` / L12 `38000` / L21 `44750`; Schedule A 5a `120000` (W-2 box 17 + estimates), 5e `10000`,
7 `10000`, 8a `30000`, 11 `1000000`, 14 `1000000`, 17 `1040000`; Schedule D 10 `1200000`/`200000`/
`1000000`, 15 `1000000`, 16 `1000000`, 17 "Yes", 20 "Yes"; Form 8949 Box F row
`20.00000000 BTC / 01/01/2020 / 06/03/2024 / 1200000 / 200000 / 1000000`.

### TWO ORACLES — both witness, exactly

```
$ OTS_DIR=~/OpenTaxSolver2024_22.07_linux64 OTS_YEAR=2024 .venv/bin/python ots_run.py 1000000
OTS version: OpenTaxSolver 2024 (OpenTaxSolver2024_22.07_linux64)
adjusted_gross_income        2000000.0
deduction_taken              1040000.0
taxable_income               960000.0
income_tax_before_credits    148705.0
amt                          0.0
form6251  {'line1': 960000.0, 'line2a': 10000.0, 'line4': 970000.0, 'line5': 133300.0,
           'line6': 836700.0, 'line7': 124045.0, 'line9': 124045.0, 'line10': 148705.0, 'line11': 0.0, …}
niit                         38000.0
additional_medicare_tax      6750.0
total_tax                    193455.0

$ .venv/bin/python tc.py 1000000        # PSL Tax-Calculator
c00100 (AGI)      2,000,000.00      c62100 (AMTI)   970,000.00
c04470 (itemized) 1,040,000.00      c09600 (AMT)          0.00
c04800 (TI)         960,000.00      niit             38,000.00
taxbc               148,705.00      ptax_amc          6,750.00
c09200 (total)      193,455.00      c07100/c11070/odc     0.00   ← CTC really is $0
```

Every btctax figure is on both witnesses. The CTC advisory is independently confirmed:
taxcalc's `c07100 = 0`, `c07220 = 0`, `c11070 = 0`, `odc = 0` with `n24 = nu18 = 5`.

★ One §G-9 catch while doing this: my first taxcalc run fed `e03210 = 2500` and taxcalc allowed the
whole $2,500 — because `e03210` is the already-limited *deduction*, an input it takes on faith, not
the amount paid. btctax runs the §221 worksheet. A value the oracle takes as INPUT is never
validated by its agreement.

### VARIANT B — forcing an AMT, to exercise the 6251 emitter

Identical household with the cash gift cut to $200,000 (everything else byte-identical):

```
  Deduction (L12, itemized): 240000.00        Taxable income (L15): 1760000.00
  Tax (L16): 407326.00
  Alternative Minimum Tax (Form 6251 → Sch 2 L2): 3622.00
    AMTI (L4) 1770000.00 · exemption (L5) 0.00 · tentative minimum tax (L9) 410948.00 · regular tax (L10) 407326.00
    Form 6251 line 7 exceeds line 10 → the form MUST be attached (i6251, Who Must File, condition 1).
  TOTAL TAX (L24): 455698.00
```
Packet grows to 9 forms with `irs/32_f6251.pdf`. OTS: `amt 3622.52`, `total_tax 455698.0`.
Tax-Calculator: `c09600 3,622.50`, `c09200 455,698.00`. Three engines agree.

Schedule 2 line 2 carries it (`f1_12[0] = 3622`, line 3 `3622`), which matches the 2024 form text
(`pdftotext -layout crates/btctax-forms/forms/2024/f1040s2.pdf`: "2 Alternative minimum tax. Attach
Form 6251").

**And the f6251 read-back is the good news of this lens:** page-1 fields `f1_6[0]` … `f1_23[0]` —
exactly 18 fields, exactly Form 6251 lines **2c through 2t** — carry **no `/V` at all**. They print
genuinely BLANK, not `0`. §G-11 does not bite here.

### VARIANT F — the church gift given IN BITCOIN (the crypto-native reading)

40 BTC lot, 20 sold, 20 sent to `bc1qchurch` and reclassified:

```
$ btctax --vault v.pgp reconcile reclassify-outflow "import|coinbase|out|mv-donate" \
      --as-kind donate --amount 1000000.00 --donee "First Church of Springfield"
Recorded decision decision|1
$ btctax --vault v.pgp reconcile set-donation-details "import|coinbase|out|mv-donate" \
      --donee-name "First Church of Springfield" --donee-ein 98-7654321 --appraiser-name "Jane Appraiser" …
Donation details saved.

  Deduction (L12, itemized): 640000.00     ← §170(b)(1)(C) 30%-of-AGI ceiling applied: 600,000 of 1,000,000
  Taxable income (L15): 1360000.00   Tax (L16): 261298.00   AMT: 0.00   NIIT: 38000.00
  TOTAL TAX (L24): 306048.00
```
Packet = 9 forms including `irs/155_f8283.pdf` (Section B, "k Digital assets", donee EIN, appraiser
identity, 20.00000000 BTC / FMV `1000000` / basis `200000` / acquired `01/2020`). Schedule A line 12
prints the limited `600000` while the 8283 prints the contributed `1000000` — correct.
Tax-Calculator with `e20100 = 1000000`: `c04470 640,000.00`, `c04800 1,360,000.00`,
`taxbc 261,297.50`, `c09200 306,047.50`. Agrees.

### The two REFUSALS I could reach from this vector (verbatim, exit code 2)

**(D) the same household plus an ISO exercise / any other AMT item** — flip
`other_out_of_scope_income = true`:

```
error: usage: tax year 2024 cannot be computed from its full-return inputs: you answered YES to
something this version cannot model — income it never asked about (rent, royalties, a farm, a K-1,
tips, gambling, alimony, an uncaptured business), an INCENTIVE STOCK OPTION exercise you still held
at year end (Form 6251 line 2i, from your Form 3921), or another alternative-minimum-tax item.
btctax models Form 6251 lines 2, 2a and 2b only, so any other Part I add-back would print as ZERO —
and because `must_attach` tests line 7 against line 10, a missing add-back would also stop the AMT
screen from firing at all. It refuses rather than file a return that understates on a line it cannot
see. Remove that item and file the rest yourself; run `income clear --year 2024` to remove them and
use a raw `tax-profile`
```

**(E) the $1,000,000 church gift entered as NONCASH in the TOML** (`class = "cap_gain_prop30"`):

```
error: usage: tax year 2024 cannot be computed from its full-return inputs: a non-crypto NONCASH
charitable gift pushes total noncash gifts over $500, which requires a Form 8283 listing ALL of the
contributed property — and btctax holds no details for property that did not come from your ledger
(description, acquisition date, appraiser). Complete Form 8283 by hand, or remove the gift.; run
`income clear --year 2024` to remove them and use a raw `tax-profile`
```

---

## WHAT IS MISSING

Ordered by consequence. Every item names a line and a direction.

### 1. Form 8960 Part II lines 9a/9b/9c are not fields at all; 9d is a hardcoded zero — NIIT is overstated

**Form requires:** Form 8960 line 9b, "State, local, and foreign income tax (see instructions)".
i8960 and Reg. §1.1411-4(f)(3)(ii) allow the portion of state/local income tax *properly allocable*
to net investment income (any reasonable method), limited by the §164(b)(6) cap on what was actually
deducted.

**btctax does instead:** `Form8960Lines` (`crates/btctax-core/src/tax/other_taxes.rs:257-288`) has
**no `line9a`, `line9b` or `line9c` field**; only a `line9d`, set at
`other_taxes.rs:312` — `let line9d = Usd::ZERO; // v1 models no investment expenses…`. This is a
closed form with no equivalence proof, exactly the compression the house rule forbids. The emitted
PDF leaves 9a/9b/9c blank (`f1_16`/`f1_17`/`f1_18` carry no `/V`) but prints `9d = 0` and `11 = 0`.

**Consequence — OVERSTATES tax.** This household deducted $10,000 of SALT and has $1,000,000 of NII
against $2,000,000 of income; a 50% reasonable-method allocation is $5,000, so line 12 should be
$995,000 and line 17 $37,810. btctax charges $38,000. **~$190 overstated, silently, with no
advisory attached to the figure.** ★ Both oracles are blind here too (Tax-Calculator has no channel;
`ots_direct.py` never feeds `L9b`), so oracle agreement proves nothing — this is a §G-9 instance and
must be closed against the form, not against a witness. It IS disclosed in prose:
`btctax limitations` line 282, "Form 8960 (NIIT), Part II — the state/local tax allocation is
omitted."

### 2. The §170(d)(1) charitable carryover-OUT is computed and never shown to the filer

**Statute requires:** §170(d)(1) carries the disallowed excess forward five years; the filer must
record the amount and its class/vintage to claim it on Schedule A line 13 in 2025–2029.

**btctax does instead:** it computes it —
`AbsoluteReturn::charitable_carryover_out` (`crates/btctax-core/src/tax/return_1040.rs:1272`), built
by `charitable.rs:176-186` — and then **never prints it**. `grep -rn "charitable_carryover_out"
crates/btctax-cli/src/` returns **nothing**. The only path that surfaces it is
`report --tax-year Y --write-carryover`, which on variant F refuses outright:

```
error: usage: year 2025 has no full-return inputs yet — the carryover is written onto that row, so
import it first (`income import --year 2025 --file <toml>`) and then re-run `--write-carryover`.
```

So the filer who runs the tool for TY2024 today — before any TY2025 row can exist — is never told
the number.

**Consequence — a $400,000 deduction the filer cannot see.** On variant F the excess is
$1,000,000 − $600,000 = $400,000 of `CapGainProp30` vintage 2024. At this household's marginal rates
that is roughly $148,000 of future tax. Nothing on the printed return records it (Schedule A has no
carryover-out line), and the run prints only the *inbound* advisory ("PRIOR-YEAR CARRYOVERS NOT
STATED"), which is about the opposite direction. Direction: OVERSTATES future tax / loses a benefit.

### 3. Schedule A has no line 6, 8b, 8c, 9, 15 or 16 — and no gate says so

**Form requires:** Schedule A line 9, "Investment interest. Attach Form 4952 if required"; line 6,
"Other taxes"; lines 8b/8c (points, interest not on a 1098); line 15 (casualty); line 16 (other).

**btctax does instead:** `ScheduleAInputs`
(`crates/btctax-core/src/tax/return_inputs.rs:507-546`) carries exactly `medical`, the SALT cluster,
`mortgage_interest_1098`, its two declarations, and `charitable`. The other lines print blank —
correctly blank as *testimony*, but nothing ever asked. The scope attestation
(`questions.rs:514`, `QuestionId::OtherOutOfScopeIncome`) is scoped to **income** and **AMT items**;
it does not ask about deductions, so a filer with margin interest answers it truthfully "no" and
forgoes the deduction.

**Consequence — OVERSTATES tax, in the disclosed/forgone-benefit direction.** Not triggered by this
vector as specified (no margin, no points), which is why I rank it third; a $1M-wage / $1M-LTCG
household with a margin loan is not exotic.

### 4. Form 6251 lines 2c–2t have no fields — closed by refusal, at the cost of the whole return

**Form requires:** 18 numbered Part I adjustment lines (2c investment interest, 2h §1202,
**2i ISO exercise**, 2m passive activities, …).

**btctax does instead:** `Form6251` (`crates/btctax-core/src/tax/form6251.rs:6-13`, its own module
doc) has fields for lines 1, 2a, 2b, 3 only; `line4 = line1 + line2a + line2b + line3` treats the
other 18 as $0. The doc says so plainly: *"Tier 2, which files the form rather than only computing
it, must give 2c–2t real fields."* **btctax already files it** (variant B emits
`irs/32_f6251.pdf`), so that boundary has been crossed. The mitigation is real and it works: the
scope attestation's limb (b) enumerates the ISO exercise by name and limb (c) enumerates depletion /
tax-shelter farm / passive / R&E, and a `yes` refuses (variant D, verbatim above).

**Consequence — REFUSES the whole return.** Not an understatement: the gate is correctly closed. But
it means *this vector's own premise ("AMT in play") is unreachable in a filing sense*: the only AMT
btctax can file for a post-TCJA MFJ household of this shape is the one driven by the line-2a
SALT/standard-deduction add-back (variant B, $3,622). The dominant real trigger — an executive with
$1,000,000 of wages exercising ISOs — turns the return away entirely. **The filed form is also
missing an honest signal:** with 18 blanks and no "these were never asked" marker, a printed 6251
looks identical whether the filer has no 2c–2t items or was never asked about them.

### 5. Schedule 8812 is absent — no consequence *here*, and that is provable

`btctax limitations` line 171 pins 1040 line 19 to $0. On this vector the CTC is genuinely $0 under
§24(b), and the advisory says so correctly ("NOT AVAILABLE TO YOU … costs you NOTHING"). **Both
oracles confirm** (`c07100 = c07220 = c11070 = odc = 0` with 5 qualifying children; OTS's total tax
matches btctax's with no credit). So for THIS vector Schedule 8812's absence has **no consequence**
and should not be planned as if it did. It becomes a real overstatement only below the §24(b)
phase-out.

### 6. Minor: printed `0`s where the form would take a blank

Not this vector's risk, but observed in the read-back: 1040 line 17 prints `0` while Schedule 2
line 3 is blank; Schedule 2 line 4 (self-employment tax) prints `0` with no Schedule SE attached;
Form 8960 line 9d prints `0` as a sum of three blank lines. Each is a §G-11/testimony question, none
changes a figure. The emitter clearly *can* leave a line blank (6251 2c–2t, Sch A 6/9/15/16, 8960
9a–9c all came out with no `/V`), so this is a per-line choice, not an architectural wall.

---

## THE SMALLEST THING THAT CLOSES IT

Sequenced. Nothing here needs a new form.

1. **Transcribe Form 8960 lines 9a, 9b, 9c as real fields on `Form8960Lines`**
   (`crates/btctax-core/src/tax/other_taxes.rs:257`), replacing the hardcoded
   `let line9d = Usd::ZERO` at `:312` with `line9d = line9a + line9b + line9c`. Doc-comment each with
   i8960's own sentence.
   **This requires COLLECTING one number from the filer** — a new `Option<Usd>` on
   `ScheduleAInputs` or a sibling on `ReturnInputs`: *"how much of your deducted state/local income
   tax is properly allocable to your investment income?"* — which is following instructions, not
   scope creep. `None` must stay `None` (blank on the page, benefit forgone), never `0`; a
   class-(B) skippable with an advisory, because omission only ever overstates. A default
   apportionment must NOT be computed for the filer: §1.1411-4(f)(3)(ii) says *reasonable method*,
   and picking one is the filer's testimony, not ours. 9a and 9c can ship as collect-or-blank in the
   same edit; both are pure benefit.
   Gate: a KAT with SALT $10,000 / NII $1M / total income $2M asserting line 12 = 995,000 and
   line 17 = 37,810, plus a mutation that reds when 9b is dropped.

2. **Print the charitable carryover-OUT in `report --tax-year`.** One line in the CLI's absolute-return
   render reading `ar.charitable_carryover_out` (`return_1040.rs:1272`), itemised by class and
   origin year, unconditional on `--write-carryover`. No new input, no new form, no computation —
   the value already exists and is already tested (`charitable.rs:221-332`). Add the same for
   `capital_loss_carryforward_out`, which the report *does* already print, so only the charitable
   side is missing. Gate: a test that reds when the line is removed on a vector whose gift exceeds
   its ceiling.

3. **Add Schedule A line 9 (`investment_interest_4952: Option<Usd>`) to `ScheduleAInputs`**
   (`return_inputs.rs:507`). Blank stays blank. If it is `Some`, the return must refuse unless a
   Form 4952 is attachable — Form 4952 has no AcroForm map, so the honest v1 move is: collect it,
   print it on Schedule A line 9, and **refuse** with a new `RefuseReason` when the §163(d) limit
   could bind (i.e. when it exceeds net investment income), because that is the branch btctax cannot
   compute. Lines 6, 8b, 8c, 15, 16 stay uncollected and blank; say so in `limitations`.

4. **Give Form 6251 lines 2c–2t real fields** — `form6251.rs`'s own module doc already schedules
   this ("Tier 2 … must give 2c–2t real fields"). Transcribe all 18 as
   `Option<Usd>` so `None` prints blank and is distinguishable from an answered zero, and change
   `line4` to sum them. This does **not** require answering any of them: keep the
   `OtherOutOfScopeIncome` refusal exactly as it is (it is correct and it works — variant D), and
   let the fields exist so the refusal can later become a per-line collection for the two that
   matter most to this household (2i ISO, 2c §4952). **Order matters: do this AFTER 3**, because 2c
   reads the Schedule A line 9 amount.

5. **Do NOT plan Schedule 8812 off this vector.** Both oracles put the credit at exactly $0 here.
   If it is built, justify it on a below-phase-out household, not this one.

---

## WHAT I AM NOT SURE OF

- **Schedule A line 12 presentation.** btctax prints the *limited* $600,000 on line 12 and the full
  $1,000,000 on Form 8283. I believe that is right (i1040sa applies the §170(b) limits before lines
  11–13, Pub. 526 does the arithmetic) but I did not settle it against i1040sa's own text; both
  oracles reach the same taxable income either way, so neither can adjudicate it. Worth one look at
  the instructions before anyone changes it.
- **The exact reasonable-method allocation for 8960 line 9b.** I used gross-income proportion (50%)
  to size the consequence at ~$190. The regulation permits any reasonable method, so the *number* is
  illustrative; the *direction* (overstatement) and the *existence* of the omitted line are not.
- **Whether `line9d`'s printed `0` is defensible.** The form says "Add lines 9a, 9b, and 9c", so a
  computed sum of blanks arguably is `0`. I did not resolve whether i8960 wants a blank there when
  all three components are blank. Same question for Schedule 2 line 4 and 1040 line 17.
- **Whether any *other* Form 6251 Part I line could be reached by btctax's existing inputs** —
  I checked 2i/2c/2h/2m by reading `amt.rs`'s module doc and `questions.rs`, and PAB interest
  (1099-INT box 9 / 1099-DIV box 13) is separately refused at `return_refuse.rs:1000` and `:1035`.
  I did not audit all 18 against the whole input surface; `amt.rs`'s module doc claims to be that
  audit and I took it as read rather than re-deriving it.
- **TY2025.** Everything above is TY2024. The CLI advertises TY2017/2024/2025 tables, and Form 6251
  line 1 forks per-year (`Form6251Line1::Y2025`), but the fixtures and `modified_agi`'s doc both say
  TY2024 is the only year the full-return path can file. I did not run this vector at 2025.
- **`--write-carryover`'s ordering constraint.** It refuses until next year's row exists. That may
  be deliberate (the message reads deliberate) or may be a usability gap that finding 2 makes moot.
  I did not read the §4 R3-M6 spec.
