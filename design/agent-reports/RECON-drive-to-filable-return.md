# RECON — driving a real filer profile to a printed packet (TY2024), and every wall on the way

**Agent:** DISCOVERY (opus, own worktree). **Changed no source file.** Nothing committed, nothing pushed.
**Brief:** `design/agent-reports/BRIEF-drive-to-filable-return.md` @ `45e4c5b16`.
**Binary under test:** `target-drive/debug/btctax` @ `45e4c5b16` (v0.18.0), built in this worktree.
**Scratch:** `/tmp/.../scratchpad/filer` and `/tmp/.../scratchpad/v_*` (outside the repo). Every
journey log is on disk there; this report cites what was observed, not what was expected.

## 0. The answer to the one question

**YES — the owner's shape reaches a printed, complete packet on TY2024.** A W-2 + Bitcoin-dispositions
+ itemized-Schedule-A filer went from an empty directory to four filled official PDFs and a manifest,
and the arithmetic on those pages is right (hand-checked against the 2024 rate schedule below).

**But the corridor is narrow, and I walked out of it five times with facts an ordinary filer has.**
Five *plausible* variations on that same profile produce **no forms at all** — not a partial packet, not
the Bitcoin pages, nothing. Three of them refuse before anything is even stored. Ranked by how likely a
real filer is to hit them, the two worst are:

1. **an itemizer who got a state income-tax refund** — the textbook second year of itemizing; and
2. **a W-2 with box 12 code C or code V** — employer life insurance over $50k, or an NSO exercise; both
   amounts are *already inside box 1* and change no figure on the return.

The packet also prints `0` on six money lines nobody was ever asked about, and on two lines the
product's own advisory says it "leaves blank".

### The profile driven

Dana & Alex Reyes, MFJ, Boulder CO. One W-2 ($145,000 wages, $21,000 federal withheld, $8,000 state).
Coinbase: a 2021 lot of 0.5 BTC, a 2024 lot of 0.1 BTC, two 2024 sells (0.2 + 0.2 BTC) — HIFO gives
three 8949 rows, short-term **and** long-term. Schedule A: Form 1098 interest $19,500, real-estate tax
$6,500, cash gifts $4,000. No Schedule C, no retirement, no dependents. Itemized $33,500 beats the
$29,200 MFJ standard, so Schedule A, the §164(b) cap and the charitable ceiling are all live.

---

## 1. The exact commands, in order

Passphrase supplied through `BTCTAX_PASSPHRASE`. `$B` = the built binary.

```
 1  $B --vault v.pgp init --key-backup key-backup.asc                  -> 0
 2  $B --vault v.pgp import coinbase.csv                               -> 0   4 rows, 4 events
 3  $B --vault v.pgp verify                                            -> 0   BALANCED, 0 hard blockers
 4  $B --vault v.pgp report --tax-year 2024                            -> 1   NOT COMPUTABLE [TaxProfileMissing]
 5  $B --vault v.pgp income import --year 2024 --file inputs.toml      -> 2   [SharedMortgageInterestUnanswered]
 6  (add other_borrower_paid_interest = false) ; repeat 5              -> 0   "Imported full-return inputs"
 7  $B --vault v.pgp report --tax-year 2024                            -> 2   [DependentStatusUnanswered]
 8  $B --vault v.pgp export-irs-pdf --out irs --tax-year 2024          -> 2   same - "no forms were written"
 9  $B --vault v.pgp income answer --year 2024 < /dev/null             -> 2   "input ended before every
                                                                              question was answered -
                                                                              nothing was stored"
10  $B --vault v.pgp income answer --year 2024  (46 prompts, answered) -> 0
11  $B --vault v.pgp report --tax-year 2024                            -> 0   full return computes
12  $B --vault v.pgp export-irs-pdf --out irs --tax-year 2024          -> 0   4 forms + manifest.txt
13  $B --vault v.pgp income project --year 2024                        -> 0
14  $B --vault v.pgp export-snapshot --out snap --tax-year 2024         -> 0
15  $B tui-edit --help                                                 ->     "unrecognized subcommand"
16  $B --vault v.pgp export-irs-pdf --out irs2025 --tax-year 2025      -> 0   crypto slice only (2 forms)
17  $B --vault v.pgp export-irs-pdf --out irs2026 --tax-year 2026      -> 2   price dataset ends 2026-06-03
18  $B --vault v.pgp extension --tax-year 2026 ...                     -> 2   flag is --year, not --tax-year
```

Step 12's packet: `00_f1040.pdf`, `07_f1040sa.pdf`, `12_schedule_d.pdf`, `12A_f8949.pdf`, `manifest.txt`.

Then the same journey re-run end to end on **fourteen variants** (`v_v2` .. `v_v15`), each in its own
vault, each driven through the real 46-question interview. Field values were read back out of the filled
PDFs with `qpdf --json=latest --json-key=acroform` joined against
`crates/btctax-forms/forms/2024/*.map.toml`, and the pages were read as printed with `pdftotext -layout`.

---

## 2. THE WALL LIST — ranked by whether a real filer hits it

### W1 — *** An itemizer who received a state income-tax refund cannot print one page

*Variant `v_v14b_state_refund`: a $1,400 Colorado refund on a 1099-G, `itemized_prior_year = true`.*

**What the filer saw** — at `income import`, before anything was stored:

> `error: usage: the 2024 inputs were NOT stored — [StateAndLocalRefundWorksheetNotComputed] you
> received a refund, credit or offset of state or local income taxes and you ITEMIZED on your
> prior-year return, so §111(a)'s tax-benefit rule makes some or all of it income on Schedule 1 line 1.
> How much is decided by the STATE AND LOCAL INCOME TAX REFUND WORKSHEET in the Form 1040
> instructions, which btctax does not compute — it needs last year's Schedule A, its SALT cap, the
> standard deduction you could have taken and the §164(b)(6) limitation. btctax refuses rather than
> guess a figure in the understatement direction. Work the worksheet by hand and file with a preparer,
> or answer "no" if you did not itemize in the year you paid the tax`

**Why it ranks first.** This is not an edge case, it is the *definition* of itemizing in a state with an
income tax: you deduct state income tax, you over-withheld, the state refunds you, and next year part
of it is income. Every itemizing W-2 filer in CO / CA / NY / IL meets this in their **second** year of
itemizing — which is the year the owner will be in. The refusal is correct law and honestly reasoned;
the problem is its **blast radius**: the whole packet, including the Bitcoin pages that have nothing to
do with it.

**What they would reasonably do next.** Two temptations, one dangerous. (a) Answer "no" to the refund
question — the refusal text itself offers that as an out, and a filer under time pressure reads
"or answer no" as permission rather than as a statement of a different fact. That is sworn testimony
under §6065 on Schedule 1 line 1. (b) Delete the 1099-G row. Same thing.
**Classification: refusal — correct, but the escape hatch it prints is an invitation to misstate.**

### W2 — *** A W-2 box 12 code the allowlist does not carry refuses the whole return

*Variants `v_v13_box12_code_C` and `v_v13b_box12_code_V`.*

> `error: usage: the 2024 inputs were NOT stored — [UnsupportedBox12Code("C")] W-2 box 12 code C is not
> supported in v1`

The allowlist is `INERT_BOX12_CODES = ["D","E","F","G","H","S","AA","BB","EE","DD","W"]`
(`crates/btctax-core/src/tax/return_refuse.rs:39`) — 11 of the ~26 codes the 2024 W-2 can print.
Two of the missing ones are ordinary:

| code | what it is | effect on the return |
|---|---|---|
| **C** | taxable cost of employer group-term life insurance over $50,000 | **none** — already inside box 1 |
| **V** | income from the exercise of a nonstatutory stock option | **none** — already inside box 1 |

Both are informational boxes already reflected in box 1 wages. Code C reaches anyone whose employer buys
more than $50,000 of life cover; code V reaches essentially every software engineer who has exercised an
NSO. **This is FR-102's shape exactly** — a tax-neutral box on a universal document that makes the
packet unprintable — and it is arguably worse than the HSA case, because there the box at least changed
a figure.

**What they would reasonably do next.** Delete the box-12 line from the TOML. It changes no number, so
the return is then *correct* — but the filer has learned that deleting inconvenient transcription makes
btctax cooperate, which is the habit this product exists to prevent.
**Classification: refusal — wrong instrument. An inert code belongs on the allowlist or in an advisory,
not in a refusal.**

### W3 — ** A mortgage over the §163(h)(3) limit: no pages, and the fix it names cannot be followed

*Variant `v_v2_mortgage_over_limit`: $900,000 principal, $41,000 interest, an honest "no" to the
debt-limit question.*

> `[MortgageOverDebtLimit] ... NEITHER number it could print is your return ... The cure is the one the
> instructions prescribe: work Pub. 936's Deductible Home Mortgage Interest Worksheet ... btctax does
> not yet have a place to enter that result — the mortgage_interest_deductible input is filed as
> FOLLOWUPS P9(a)/S2 — so until it lands, file this year's Schedule A by hand. btctax report still
> runs. ... — no forms were written`

The reasoning is first-rate and `btctax limitations` discloses it in full (line 341). Two things still
bite:

- **`--forms` cannot rescue the Bitcoin pages.** `export-irs-pdf --forms f8949,schedule-d` produces the
  identical refusal — the flag is documented "Ignored on a full-return year". So the refusal tells the
  filer to do Schedule A by hand while withholding the four pages that would let them.
- **The only workaround destroys the interview.** `income clear --year 2024` plus a hand-computed
  `tax-profile` does yield `f8949 / schedule_d / form_1040_capgains` — but it discards 46 recorded
  answers and every transcribed document, and the resulting 1040 slice then prints
  *"the Digital Asset question ... neither Yes nor No is marked, because this return does not record an
  answer to it"* — the answer the filer had already given, deleted by the workaround.

$750,000 is exceeded by ordinary houses in ordinary metros. **Classification: refusal — correct, but
whole-packet where it should be per-form.**

### W4 — ** `income clear` deletes a committed interview with no guard, while a *draft* is protected

Discovered inside W3. `income import` and `income clear` both take `--discard-draft`, whose own help says
a draft's *"recorded answers cannot be re-created by re-typing (btctax records when and in what words it
asked)"* — and refuses without the flag. A **committed** return holds strictly more of that (46 answers,
every document, the Schedule A) and `income clear --year 2024` removes it on one line with no flag, no
confirmation and no summary of what was lost:

```
$ btctax --vault v.pgp income clear --year 2024
Cleared full-return inputs for tax year 2024.
```

The weaker artifact is guarded and the stronger one is not.
**Classification: data loss — asymmetric protection.**

### W5 — ** A $600 bag of clothes to a thrift store: no pages

*Variant `v_v5_noncash_gift` (`class = "ordinary_prop50", amount = "600"`).*

> `[NonCryptoNoncashGift] a non-crypto NONCASH charitable gift pushes total noncash gifts over $500,
> which requires a Form 8283 listing ALL of the contributed property — and btctax holds no details for
> property that did not come from your ledger (description, acquisition date, appraiser). Complete Form
> 8283 by hand, or remove the gift. — no forms were written`

Disclosed in `limitations` line 102. Still: donating used goods is *what itemizing filers do*, the
threshold is $500, and "remove the gift" is a ~$130 tax cost offered as a remedy.
**Classification: refusal — correct law, disclosed; the scope (whole packet) is the defect.**

### W6 — * The interview offers the sales-tax election and cannot service a "yes"

*Variant `v_v4_sales_tax_election`.* `income answer` asks *"Deduct general SALES taxes instead of
state/local income taxes? (§164(b)(5))"*; answering **y** never prompts for the amount. The interview
then prints, in its own after-panel, the refusal it just created:

> `REFUSING (1) — an answer already given that stops the return:`
> `[SalesTaxElectionWithoutAmount] ... Schedule A line 5a would be $0 and your state/local income taxes
> (W-2 box 17/19 withholding, estimates, prior-year balance) drop out — enter the amount, or clear
> salt_use_sales_tax to deduct income taxes instead, and re-run btctax income import`

Well flagged — the filer is told immediately and in the right place. The gap is that the cure lives in a
**different surface**: the interview cannot supply the amount it needs, and it points back at
`income import`, i.e. back to the TOML (see W7).
**Classification: warning — adequate; the prompt should either collect the amount or not be offered here.**

### W7 — ** The only file-based way in has no published schema, and the pointer to the other way in is a command that does not exist

`income import --file <FILE>` is "Path to the TOML file describing the full-return inputs" and **no
shipped document states the format.** `docs/examples/examples.md` J6 runs
`income import --file fullreturn.toml` and never prints that file; the only place those key names appear
in `docs/` is the **JSON** echo of `income show`. The real TOML lives in
`crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml`, which a filer does not have.

The other authoring surface is the TUI editor — and the one advisory that names it, printed on every
refund-due packet, names it wrong:

> "add your routing and account numbers in the tax-inputs editor (`btctax tui-edit`, then T on the year)"
> — `crates/btctax-core/src/tax/advisories.rs:623`

```
$ btctax tui-edit --help
error: unrecognized subcommand 'tui-edit'
```

The binary is `btctax-tui-edit` (a separate executable). So the documented CLI path needs an unpublished
file format, and the pointer to the GUI path is a typo. `income scrub` *does* emit TOML — but only from a
year you already have, so it cannot bootstrap a first year.
**Classification: documentation — but it is the front door.** I only got in because I had the test
fixture open; my TOML then took **two** attempts, not the dozen a filer would need.

### W8 — * A 1099-DIV with box 2b refuses the return — i.e. an index-fund investor

*Variant `v_v15_reit_1250`: $120 of unrecaptured §1250 gain on a consolidated 1099.*

> `[UnrecapturedOrSpecialRateGain] 1099-DIV box 2b/2c/2d requires the Schedule D Tax Worksheet — out of
> scope`

REIT and balanced funds put box 2b on ordinary consolidated 1099s every year. The refusal direction is
right (the Schedule D Tax Worksheet would change the rate) and the magnitude here is cents — but the
outcome is again zero pages. **Classification: refusal — correct, whole-packet.**

### W9 — * A 1099-G transcribed as the paper reads will not parse

Before W1 could even be reached, a 1099-G carrying only a state refund failed:

> `error: usage: invalid ReturnInputs TOML: missing field 'box1_unemployment'`

Measured across the document structs in `return_inputs.rs`, the money fields with **no**
`#[serde(default)]` are exactly: `W2.box1_wages`, `W2.box2_fed_withheld`, `Form1099Int.box1_interest`,
`Form1099Div.box1a_ordinary`, `Form1099G.box1_unemployment`. Every other box on every document row is
defaulted. The 1099-G is the only one whose mandatory box is routinely **blank on the paper** — a
state-refund 1099-G has box 2 and nothing in box 1 — so the filer must type
`box1_unemployment = "0"`, asserting no unemployment compensation, to make the file load. The message
names a Rust field, not a form line, and does not say which row.
**Classification: refusal — a mandatory-field list that does not match the forms.**

### W10 — * The mixed-use declaration silently takes Schedule A off the packet

*Variant `v_v10_mixed_use`: an honest "no" to "did you use ALL of your home-mortgage loan(s) to buy,
build, or improve that home?" — a HELOC spent on anything else.*

Line 8a is treated as $0 (documented, `limitations` 473), the itemized total falls to $14,000, the
standard deduction wins, **the packet shrinks from 4 forms to 3 and Schedule A disappears**, and the tax
rises $946. There is a clear advisory (*"Your return took the standard deduction. Because you declared a
mixed-use mortgage, line 8a was treated as $0 ..."*). Recorded because the owner has a mortgage and one
HELOC answer erases the entire reason they were itemizing.
**Classification: default + warning — adequate.**

### W11 — * TY2026's first wall is the price dataset, not the forms

> `error: usage: cannot export TY2026: the bundled price dataset ends 2026-06-03, before TY2026's
> prices_through 2026-12-31 — every figure on the packet would be computed from a PARTIAL year. No forms
> were written. Update the bundled daily-close dataset (scripts/ — the price updater appends closes) and
> re-export.`

A filer cannot act on that: it names a **source directory**, not the `btctax-update-prices` binary that
is actually shipped for the job. **Classification: documentation.**

### W12 — nits, recorded not argued

- `extension` takes `--year`; `export-irs-pdf` takes `--tax-year`. The error is clear; the inconsistency
  is real.
- `income answer`'s Step 0 prints *"authoring works with an unresolved ledger; committing and exporting
  do not"* — yet `v_v11_self_custody` exported cleanly with two unresolved (advisory) ledger blockers.
  The sentence means *hard* blockers.
- A top-level TOML key written after the first table header produces an excellent diagnostic
  (`unknown key(s) ... schedule_a.?.charitable.0.itemized_prior_year`) that resolves the misplacement but
  never states the actual cure ("top-level keys must precede every table header") — and hand-written TOML
  makes this the single most likely filer mistake.

---

## 3. Blank-provenance verdict, line by line

Read off the filled PDFs (`qpdf` field values plus `pdftotext -layout`), classified against the interview
transcript and the map files. **The structural half is in good shape:** `cargo test -p btctax-forms
--test field_census` passes 5/5 (including `the_gate_reds_on_every_planted_defect`), so every *unmapped*
cell on these forms carries a written `rule` / `reason` / `covered_by` census decision. What follows is
mostly about cells that are mapped and **written as `0`**.

### Form 1040 — 45 fields written, 36 blank

| line | printed | verdict |
|---|---|---|
| 1a, 1z, 9, 11, 12, 14, 15, 16, 18, 22, 24, 25a, 25d, 33, 34, 35a | figures | **correct**, all tie out |
| 7 | `3960` | **correct** — equals Schedule D line 16 |
| 2a, 2b, 3a, 3b | `0` | **correct as testimony** — the filer answered the 1099-INT/DIV census *and* the "interest or dividends with no 1099" follow-up. Two questions, both asked, both "no". |
| 8 | `0` | **correct** — the long Schedule-1 catch-all question was asked and answered |
| 13 | `0` | **correct** — derived (no business) |
| 10, 17, 23, 31, 32 | `0` | **advisory-covered** — `UnmodeledDeductionsOmitted` / `OtherCreditsOmitted` say these are not computed. The page still swears `0`. |
| **26** | **`0`** | **THE DEFECT. "2024 estimated tax payments and amount applied from 2023 return" is asked by no question, covered by no advisory, and mentioned nowhere in any output captured** — a grep of the owing-variant log for "estimated" finds only line 36's *next*-year text. `Payments.estimated_tax_payments` exists in the TOML and defaults to zero; the string does not appear in `questions.rs` or `advisories.rs`. A filer who sold Bitcoin at a gain and paid quarterlies gets `0` printed under penalties of perjury, and an overstated balance due. |
| 19, 20, 21, 27, 28, 29, 30, 36, 38 | blank | **correct** — no dependents, no credits, refund case |
| 1b-1i, 4a-6c | blank | **correct** — census-decided, and each is inside the catch-all question |
| 25b, 25c | `0` | **borderline** — 25b derives from the 1099 census; 25c is `payments.other_withholding`, never asked |
| 35b-35d, phone, IP PIN, signature | blank | **correct and deliberate** — the manifest's "COMPLETE BY HAND" names the signature block; the refund advisory names the deposit cells |

### Schedule A — 23 written, 3 blank (all three are checkboxes)

| line | printed | verdict |
|---|---|---|
| 2, 3, 5a, 5b, 5d, 5e, 7, 8a, 8e, 10, 11, 14, 17 | figures | **correct.** SALT $8,000 + $6,500 = $14,500 capped to $10,000; total $33,500 |
| 1 | `0` | **never asked.** `schedule_a.medical` is TOML-only: no interview question, no advisory. Prints "I had no medical expenses". |
| 4 | `0` | **correct** — the form itself says "enter -0-" |
| 5c | `0` | never asked (personal-property tax); harmless here, the cap already binds |
| **8b / 8c** | **`0`** | **THE SECOND DEFECT, and it contradicts the packet's own advisory.** `Advisory::UnmodeledDeductionsOmitted` — printed on this very packet — says btctax "models no home mortgage interest not reported to you on Form 1098 (line 8b), no points not reported to you on Form 1098 (line 8c) ... **Each one it leaves blank** is money you may be entitled to keep ... claim them yourself". Neither half holds any more: T9 added `schedule_a.mortgage_interest_not_on_1098` and `points_not_on_1098` (recorded in the map at :129-:131), and the emitter writes `0`, not blank. The map's own header still lists both as "UNMAPPED ON PURPOSE — left BLANK, never a misleading 0" (:25-:28). **Three statements about one pair of cells, two of them stale.** The same advisory also claims v1 models no "HSA ... contributions" while `return_1040.rs:2312` computes `hsa_deduction_13` onto Schedule 1 line 13. A filer reading it is sent elsewhere for three deductions the tool supports. |
| 9 | `0` | **never asked.** Answering "I am not filing Form 4952" does **not** assert zero investment interest — Form 4952's own exception lets you deduct it *without* the form. So this `0` can understate a real deduction, and no question or advisory covers it. |
| 12 | `0` | asserts no noncash gifts; internally consistent, since anything over $500 refuses (W5) |
| **13** | **`0`** | **the sharpest contradiction.** The advisory says, verbatim, that btctax "has no ... charitable carryover (§170(d)(1)) on file ... so **it cannot tell 'you have none' from 'nobody asked'**". The form then prints `0`. And `charitable_carryover_in_provenance` defaults to **`User`** — an omitted block is recorded as the *filer's* answer. |
| 6, 15, 16 | blank | **correct** — censused `unmodeled`, advisory-covered. Note the printed convention is inconsistent *within one form*: 6/15/16 blank, 8b/8c/9 `0`. |

### Schedule D — 10 mapped fields written, 5 blank

Part I line 3 = 6175 / 9510 / -3335; Part II line 10 = 24125 / 16830 / 7295; line 7 = -3335;
line 15 = 7295; line 16 = 3960; line 17 "Yes"; line 20 "Yes"; QOF "No" — all **correct**.

| line | printed | verdict |
|---|---|---|
| **6 / 14** | **`( 0 )`** | **same class as Schedule A 13** — capital-loss carryover in, printed as zero, while the advisory says btctax cannot distinguish "none" from "nobody asked"; `capital_loss_carryforward_in_provenance` also defaults to `User`. |
| 13 | `0` | correct — 1099-DIV box 2a, censused and asked |
| 18 / 19 | blank | **correct here, but silently so.** The crypto-slice export prints an explicit note (*"18 and 19 are blank because btctax never asked, not because they are zero"*); the **full-return** path prints no such note. Defensible, because `UnrecapturedOrSpecialRateGain` refuses a 1099-DIV carrying those amounts — but that screen reads only 1099-DIV box 2b/2c/2d, so a **directly sold collectible** (coins, art: no 1099-B, and not named in the catch-all question's list of capital transactions) would leave line 18 blank and let line 20 check "Yes", routing to the QDCGT worksheet instead of the Schedule D Tax Worksheet. Outside the stated profile; recorded because line 20's "Yes" is the box earlier work made answered-ness-safe. |
| 21 / 22 | blank | **correct** — line 16 is a gain, and line 20's "Yes" branch says do not complete 21-22 |

### Form 8949 — Box C on page 1, Box F on page 2, three rows

`0.10000000 BTC` 02/20/2024 -> 07/15/2024, 6175 / 9510 / -3335 . `0.10000000 BTC` 05/10/2021 ->
07/15/2024, 6175 / 5610 / 565 . `0.20000000 BTC` 05/10/2021 -> 11/10/2024, 17950 / 11220 / 6730.
Totals tie to Schedule D exactly. Proceeds are net of sale fees and basis includes the buy fee — correct
under §1001. **No blank-provenance issue on this form.**

### The arithmetic, hand-checked (no oracle run, per the brief)

Taxable income 115,460; net capital gain = min(Sch D 15, 16) = 3,960; ordinary slice 111,500.
MFJ 2024: 10% x 23,200 = 2,320; 12% x (94,300 - 23,200) = 8,532; 22% x (111,500 - 94,300) = 3,784, so
**14,636**. LTCG: taxable income exceeds the $94,050 zero-rate ceiling, so 3,960 x 15% = **594**.
14,636 + 594 = **15,230** = the printed line 16, and that 594 equals the separately reported
crypto-attributable LTCG tax. The packet's figures are internally and legally consistent.

---

## 4. TY2026 — the same profile, from the extracts on paper

TY2026 is the first *filed* year, and the gap is not one thing. In the order a filer meets it:

1. **Prices.** `export-irs-pdf --tax-year 2026` refuses today: the bundled daily-close dataset ends
   **2026-06-03** against `prices_through = 2026-12-31`. Nothing else is reachable until that is extended
   (W11).
2. **Templates: zero.** `crates/btctax-forms/forms/2026/YEAR.toml` has `forms_expected = []` and a
   21-row `[forms_absent]` table — every 1040-family form is *"TY2026 revision not released ... January
   2027 package"*. `report --tax-year 2026` says so out loud: **"TY2026 — preparing (0 forms; TaxTable
   yes; full-return params no; 1099-DA proceeds+basis)"**.
3. **Params: transcribed, deliberately not wired.** `ty2026_full_return()` exists with every cell cited to
   statute and Rev. Proc. 2025-32, but is **not inserted into `by_year`**
   (`crates/btctax-adapters/src/tax_tables.rs:206`), so `full_return_for(2026)` is `None` and **no
   absolute return computes for 2026 at all** — only the crypto delta, and only with a raw `tax-profile`.
   Two gates are named there: the restructured 2026 Form 6251 (line 1 -> 1a/1b) needs re-transcription,
   and **no OTS 2026 exists**, so the two-oracle rule cannot be met.
4. **Schedule A is a different document.** From `design/forms/extract/f1040sa--2026-DRAFT.txt`, every part
   of this profile's Schedule A moves:
   - **line 5e** — the cap is **$40,400 ($20,200 MFS)**, with a phase-down referenced when **1040 line
     11b** exceeds **$505,000** (so the 1040 itself is re-lettered). The params already carry
     $40,400 / 30% over $505,000 / a $10,000 floor (§164(b)(6)-(7), Pub. L. 119-21 §70120).
   - **line 8d** is now **"Mortgage insurance premiums"** — in 2024 it is a ReadOnly "Reserved for future
     use" widget — and **8e = add lines 8a through 8d**.
   - **Gifts to Charity is renumbered and re-plumbed:** 13 = *"Enter the amount from line 6 of the
     **Charitable Contribution Limitation Worksheet**"*, 14 = carryover from prior year, 15 = 13 + 14.
     Lines 11 and 12 no longer feed the total directly — a **new worksheet** stands between them, and it
     is where §170(p)'s **0.5%-of-AGI floor** (Pub. L. 119-21 §70425) lands. On this profile that floor
     is 0.005 x AGI = about **$745** of the $4,000 gift, in the **understatement** direction if
     unmodelled.
   - the write-in line becomes **17a-17k / 17z** (deductible gambling losses get their own line),
     casualty moves to 16, and the total moves to **18 behind a new §68-style gate** — *"Is the amount on
     Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and 13b of that form, more than
     $384,350?"*

   Already tracked: `design/TY2026_PORT_REPORT.md` row 20 and its `**new**` row rate this
   **DORMANT / IMPORTANT** ("the $384,350 gate and the Itemized Deductions / Charitable Contribution
   Limitation worksheets are unmodelled"), and the two readiness lenses
   (`design/direction/filing-readiness-lens-itemized.md:182` and `-charity.md:9`) say the §170(p) floor
   and the §68 2/37 haircut are unmodelled anywhere in `btctax-core`, *"safe only because 2026 fails
   closed"*. **Confirmed by driving it: 2026 does fail closed.**
5. **The Bitcoin side changes shape.** TY2026 is the first **basis**-reporting Form 1099-DA year
   (`[information_returns.f1099da] proceeds = true, basis = true`). The broker-reporting answers become
   mandatory — one table per provider per cohort, `[broker_reporting.coinbase]` with `covered` /
   `noncovered` — a missing one refuses the return and an extra one refuses as unread
   (`BrokerReportingUnanswered` / `BrokerAnswerUnread` / `BrokerBasisDiffers`), and the 8949 boxes move
   from C/F to I/L. On TY2024 the interview says this plainly: *"TY2024's Form 1099-DA regime reports
   nothing ... your disposals on coinbase are boxed by mechanism, not by an answer."* In TY2026 they are
   boxed by **an answer the filer must read off the paper** — a document-transcription step this profile
   has never performed.
6. **Schedule 1-A goes live** (tips / overtime / car-loan interest / senior). This profile answers "no" to
   all four — but they are `tax_year`-scoped questions whose "no" must be *recorded*, not assumed, so the
   interview grows.
7. **AMT gets sharper.** §70107(c) doubles the exemption phase-out rate 0.25 -> **0.50**, and the port
   report flags `amt.rs`'s inlined `dec!(0.25)` / `dec!(0.26)` as **DORMANT / IMPORTANT** — half the
   correct rate — plus R23: the installed taxcalc understates TY2026 AMTI by the whole standard
   deduction. A large-LTCG Bitcoin year is exactly the population that reaches AMT.

**Net for TY2026:** nothing about this profile is *wrong* today, because every path fails closed. The work
is (a) the price dataset, (b) the January-2027 finals for ~21 forms, (c) inserting the params behind the
Form 6251 re-transcription, (d) a new Schedule A line-set **plus two worksheets that do not exist yet**,
(e) the 1099-DA broker-reporting interview, and (f) a second AMT witness that will not exist before about
2027-01-27.

---

## 5. What worked, and worked well — so it is not re-litigated

- **The packet prints.** Values are burned into real appearance streams: `pdftotext -layout` on every page
  shows the figures, so no viewer has to honour `NeedAppearances` for the paper to be right.
- **The interview is genuinely good.** 46 prompts, each in the form's own words with a cite; it announces
  that answering can make the list **grow** and then does (two questions appeared only after the 1099
  census was answered "no"); it refuses to store anything on EOF (*"input ended before every question was
  answered — nothing was stored"*); and the after-panel names any answer that now refuses.
- **`manifest.txt` is the best artifact in the packet** — stapling order, "COMPLETE BY HAND" (the
  signature block, correctly), the payment-versus-no-payment mailing-address distinction, postage,
  couriers, and a records-retention section that correctly says a lot's acquisition record's clock starts
  when the lot is *disposed of*, not when this return is filed.
- **`btctax limitations` (578 lines) discloses W3, W5 and W6 before the fact**, each with its statute.
- **The field-provenance census is real and enforced** — `field_census` 5/5 green, including a
  planted-defect kill.
- **A self-custody withdrawal does not become a phantom sale** (`v_v11_self_custody`): the 0.15 BTC Send is
  an `UnmatchedOutflows` **advisory**, Schedule D totals are untouched, and the packet still prints.
- **`extension --pay` warns about the very gap I went looking for**: *"the $4000 on line 7 is NOT yet on
  your 2024 return. Record it as the extension payment ... Schedule 3 line 10"*. That is precisely the
  disclosure 1040 line 26 lacks — the pattern already exists in the product and is simply not applied to
  estimated payments.
- Bigger and stranger shapes stay inside the corridor: a $138,000 long-term gain grows the packet to 6
  forms (Schedule 2 + Form 8960); a $120,000 gift correctly caps at 60% of AGI and reports a §170(d)
  carryover-out with `--write-carryover`; $23,000 of medical clears the 7.5% floor; box-12 codes D and DD
  are accepted; a $2,100 1099-INT correctly adds Schedule B; and a Single filing status works identically.

---

## 6. One line each: what is missing (no implementation proposals, per the brief)

- **W1 / W2 / W5 / W8:** a per-form escape, so a return-level refusal does not withhold the forms it does
  not affect (`--forms` is currently ignored on a full-return year).
- **W2:** codes `C` and `V` — and the rest of the inert set — belong on `INERT_BOX12_CODES`.
- **W4:** `income clear` needs the guard `income import` already has for a draft.
- **W7:** a published TOML schema, or an `income template`; and `advisories.rs:623` should say
  `btctax-tui-edit`.
- **1040 line 26:** a question, or the advisory that `extension --pay` already knows how to write.
- **Schedule A 8b / 8c / 9 / 13 and Schedule D 6 / 14:** the emitter cannot express a blank (the standing
  §G-11 item), and `Advisory::UnmodeledDeductionsOmitted` is stale on 8b, 8c and HSA.
- **W9:** `Form1099G.box1_unemployment` needs `#[serde(default)]` like every sibling box.
