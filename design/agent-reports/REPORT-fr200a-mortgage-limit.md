# REPORT — FR-200(a): Pub. 936 Table 1 transcribed; the over-$750k mortgage no longer refuses the packet

**Status: BUILT, GREEN, UNCOMMITTED.** The work is in the worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-ae1a991a7c3186921`, branch
`worktree-agent-ae1a991a7c3186921`, base `cc29bb30e`. Nothing was committed.

## Gate (run SERIALLY, in the foreground, after the last edit)

```
$ cargo nextest run --workspace --no-fail-fast
     Summary [  23.045s] 3855 tests run: 3855 passed, 12 skipped

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 3.49s

$ cargo fmt --all --check
fmt-exit=0
```

Archive integrity — `legal/` was read and never edited:

```
$ sha256sum legal/text/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.txt
2506899efa7530e4f22038bee4a20538b2459157fb7c5156762e323343a53e5c
```

★ **One brief correction.** The brief gives the archived TEXT LAYER's sha256 as
`a4ef802b92ddd40ce2d00331b5012834dd274fd6ce74624586a1ea7dc31c32de`. That is the hash of
`legal/primary-sources/irs-publications/Pub936_Home_Mortgage_Interest_Deduction.pdf`; the `.txt`
extract — which is what was transcribed from, per `CLAUDE.md` — hashes to `2506899e…` above. Both
copies (main tree and worktree) are byte-identical, so nothing was mis-read; the brief simply
attributed the PDF's hash to the text path.

## What was built

`crates/btctax-core/src/tax/pub936_table1.rs` — **1,995 lines**, new. Pub. 936 (2025) **Table 1,
*Worksheet To Figure Your Qualified Loan Limit and Deductible Home Mortgage Interest for the Current
Year***, transcribed: one field per numbered line, named for the line, in the form's own numbering,
carrying the official text verbatim as its doc comment. **Sixteen** numbered lines (the brief says
fifteen; the form prints sixteen — line 16 is *"Subtract the amount on line 15 from the amount on line
13 … This isn't home mortgage interest"*).

**The branches are transcribed as branches**, per the brief's star:

| branch | modelled as |
|---|---|
| Part I bullet 1 — *"If you have no home acquisition debt incurred after December 15, 2017, or the amount on line 6 is $750,000 ($375,000 if married filing separately) or more, line 6 is your qualified loan limit. Enter this amount on line 11 and go to Part II, line 12."* | lines 7–10 are `None` — **skipped**, never `-0-` |
| Part I bullet 2 — *"If you have home acquisition debt incurred after December 15, 2017, go to line 7."* | the other limb |
| Part II bullet 1 — *"If line 11 is less than line 12, go on to line 13."* | lines 13–16 worked |
| Part II STOP — *"If line 11 is equal to or more than line 12, stop here. All of your interest on all the mortgages included on line 12 is deductible as home mortgage interest on Schedule A (Form 1040)."* | `Outcome::AllDeductible`; lines 13–16 are `None` |

### Collected, because the form asks for it

`ScheduleAInputs::pub936_table1: Option<Table1Facts>` — four average balances (lines 1, 2, 7, 12) plus
the two conditions the publication and i1040sca make the filer test, plus `provenance`. **`None`
REFUSES.** No leaf carries `#[serde(default)]`: a block present but missing a balance is a mistyped
block, not a lawful state, and a `0` invented for an unasked balance would make *every* dollar of
interest deductible (a zero line 12 forces the STOP). Published in `docs/income-import-schema.md` as
`[schedule_a.pub936_table1]`, regenerated with `cargo run -p xtask -- toml-schema`; the only change is
the new block (63 to 69 serde-required keys, 389 to 397 paths).

### Line 13 is DERIVED, not collected a second time

The line-13 instruction names exactly two things btctax already holds row by row — *"you should get
Form 1098 … This form will show the amount of interest to enter on line 13"* (the sum of **box 1**) and
*"Also, include on this line any other interest payments made on debts secured by a qualified home for
which you didn't receive a Form 1098"* (the line-8b rows). A `line13` field beside them would be a
second testimony about one figure. `pub936_table1::line13_interest_paid` is the single reader.

★ **Points are excluded from line 13** (*"Don't include points or mortgage insurance premiums on this
line"*) and reach Schedule A through the separate path the publication prescribes — see below.

### Where line 15 lands: Schedule A 8a / 8b / 8c

Line 15 says *"Enter this amount on Schedule A (Form 1040)"* with **no line number**, because Table 1
spans three Schedule A lines. `pub936_table1::apportion` multiplies each component by line 14, which is
the publication's own rule rather than an inference — *Claiming your deductible points*, item 2:
*"Multiply the amount in item 1 by the decimal amount on line 14. Enter the result on Schedule A (Form
1040), line 8a or 8c, whichever applies."* Table 2 names the three destinations, and
`tests::table_2_routes_each_component_to_the_schedule_a_line_this_module_uses` reads them off the
extract — both physical lines of each row, since the 8a and 8b rows differ only in their second.

## ★ WHICH OF THE FOUR LIMITS NOW COMPUTE, AND WHICH STILL REFUSE

i1040sca's *Limits on home mortgage interest* block states four limits and says *"see Pub. 936 to
figure your deduction"* four times.

| # | limit | status after FR-200a |
|---|---|---|
| 1 | *"Limit for loan proceeds not used to buy, build, or substantially improve your home"* (the mixed-use limit) | **already modelled** — §163(h)(3)(F), the Schedule A line-8 checkbox, `mortgage_all_used_to_buy_build_improve`. Untouched. |
| 2 | *"Limit on loans taken out on or before December 15, 2017"* — $1,000,000 / $500,000 MFS | **COMPUTES** — Table 1 lines 2, 3, 4, 5, 6 |
| 3 | *"Limit on loans taken out after December 15, 2017"* — $750,000 / $375,000 MFS, *"reduced by the amount of your qualifying debt subject to the $1,000,000 limit"* | **COMPUTES** — Table 1 lines 7, 8, 9, 10, 11 |
| 4 | *"Limit when loans exceed the fair market value of the home"* | **STILL REFUSES** — `RefuseReason::MortgageFairMarketValueLimit` |

Also still refusing, each with its own reason and its own remedy — nothing was widened into silence:

| condition | reason | why it cannot compute |
|---|---|---|
| the FMV limit affirmed | `MortgageFairMarketValueLimit` | **Pub. 936 (2025) prints no worksheet for it and does not mention it.** See F1. |
| the April-2018 written-binding-contract exception affirmed (Figure A, footnote 3) | `MortgageApril2018TransitionRule` | it RE-BUCKETS a balance from line 7 to line 2 — the **filer-favourable** direction — and btctax does not collect which loan or how much. Ignoring it overstates; applying it may understate. btctax picks neither. |
| Table 1 line 12 below lines 1 + 2 + 7 | `Pub936Table1Line12BelowItsComponents` | line 12's own sentence makes it the total of those lines, so a smaller figure contradicts the form. Refused rather than clamped: line 12 is line 14's DENOMINATOR, so too small a figure makes the deduction too **large**. |
| *"yes, I was inside every limit"* **and** a Table 1 that says a limit bites | `MortgageDebtLimitContradicted` | two answers to one line on a return signed under §6065. |
| the answer given and **no** Table 1 figures | `MortgageOverDebtLimit` (**narrowed**) | the state every existing vault is in. Its exit now names `[schedule_a.pub936_table1]` and all four lines instead of telling the filer to abandon the year. |

★ The pre-Oct-1987 grandfathering *exception* the brief names (the balloon-note refinancing rule) is
**not** a separate refusal, and deliberately: it decides whether a balance IS grandfathered, which is
the determination the filer makes when they enter line 1's average balance. Table 1 takes the balance
and never asks how it was classified. The classification rules are quoted in `Table1Facts`'s line-1 doc
comment so the filer reads them where they type the figure.

## FINDINGS

### F1 (the substantive one) — i1040sca sends four limits to Pub. 936; Pub. 936 (2025) figures three

`i1040sca--2025.txt:1054`: *"Limit when loans exceed the fair market value of the home. If the total
amount of all mortgages is more than the fair market value of the home, see Pub. 936 to figure your
deduction."* Pub. 936 (2025) contains **no** fair-market-value worksheet and does not mention such a
limit: the only occurrences of *"fair market value"* in the whole publication are the cost-allocation
rule for a partly-non-qualified home and one figure inside an example. It is a pre-TCJA instruction
whose worksheet went away with home-equity-debt deductibility.

So the fourth limit is a **wall by the publication's own silence**, not by btctax's omission, and
`MortgageFairMarketValueLimit` says exactly that to the filer. It is pinned both ways by
`tests::pub936_2025_prints_no_fair_market_value_worksheet_which_is_why_that_limit_refuses`: i1040sca
must still name the limit (or the refusal has no premise) **and** Table 1 must not mention fair market
value (if a future revision reinstates it, the test reds and the refusal should become a transcription).

### F2 — line 12's two texts disagree, and the disagreement is a money direction

The worksheet line says *"the total of the average balances of all mortgages **from lines 1, 2, and
7**"*; the line-12 instruction says *"the average balance … of each **outstanding home mortgage**"*,
and its Note puts **home equity debt** — which is in none of lines 1, 2 and 7 — in the same average.
The instruction's set is a strict SUPERSET.

A derived `line12 = line1 + line2 + line7` would therefore drop home equity debt out of line 14's
denominator, making the ratio too large and the deduction too big — an **understatement**, invisible to
both oracles. So line 12 is **collected**, and the bound the two texts jointly prove is enforced:
`line12 >= line1 + line2 + line7`, else refuse.

### F3 — the two "you don't need Table 1" situations ARE the line-12 STOP (proved, not assumed)

The Table 1 Instructions open with an exit: *"You can deduct all of the interest … in either of the
following two situations. • All the mortgages are grandfathered debt. • The total of the mortgage
balances for the entire year is within the limits discussed earlier under Home Acquisition Debt. In
either of those cases, you don't need Table 1."*

Both are exactly the state Part II's own STOP produces, so the module needs no separate exit and
running the worksheet unconditionally is never wrong:

- *all grandfathered* ⇒ lines 2 and 7 are 0 and line 12 is line 1 ⇒ line 5 = line 1, line 4 = max(line
  1, $1,000,000) ≥ line 1, line 6 = min = line 1 = line 11 = line 12 ⇒ **line 11 ≥ line 12 ⇒ STOP**;
- *within the limits* ⇒ line 11 is the limit, line 12 the balance ⇒ **STOP**.

Pinned by `the_all_grandfathered_situation_reaches_the_line_12_stop` (at $2.5M, MFS) and
`the_within_the_limits_situation_reaches_the_line_12_stop` (every `FilingStatus`, exactly at the
ceiling).

### F4 — a SECOND filer this unwalls: the apportionment can flip the §63(e) election

`deduction_is_itemized` is computed from `schedule_a_parts(..).total_17`, which now carries the
apportioned 8a/8b/8c. §63(e) compares the standard deduction against the itemized total the filer would
actually claim, so the capped figure is the right side of that comparison.

The filer this unwalls: $16,000 of Form 1098 interest on a $1,000,000 post-2017 loan, single. The full
figure beats the $14,600 standard deduction; 0.750 x $16,000 = $12,000 does not. **Before FR-200a they
were `deduction_is_itemized` on the uncapped figure and `MortgageOverDebtLimit` refused the whole
packet — over a Schedule A they were never going to file.** Now the election is STANDARD, the refusal
block is never entered, and the correct return goes out with no Schedule A at all. Pinned by
`fr200a_apportionment_can_flip_the_63e_election_to_the_standard_deduction`.

★ For the facts-**missing** population the predicate is still the uncapped total, so the existing "why
`deduction_is_itemized` is exactly the right predicate" reasoning — and the phase-2 Critical it fixed —
is untouched.

### F5 — pdftotext transposes one token in Table 1, and the repair is one token wide

The text layer emits Part I's second branch bullet as *"• you have home acquisition debt … go to line
7."* with the leading **`If`** on the **following** physical line. `table1_block` moves a line whose
entire content is `If` to the front of the nearest preceding bullet, and nothing else is reordered.
`the_if_repair_moves_exactly_one_token_and_invents_nothing` pins the scope two ways: the raw block
contains **exactly one** orphaned `If`, and the repaired block has the same multiset of words as the raw
one — so the repair cannot make an arbitrary quote match.

### F6 (found by an existing instrument, fixed) — the seventh `CarryProvenance` leaf

`cmd::tax::tests::every_provenance_key_the_schema_says_is_forced_actually_is` redded the moment the leaf
existed, naming `schedule_a.pub936_table1.provenance` verbatim:

```
`docs/income-import-schema.md` publishes these keys as "forced to `user`" and `import_return_inputs`
does not force them, so the document asserts a guarantee the code does not keep (FR-196a):
["schedule_a.pub936_table1.provenance"]
```

`force_carry_provenance_to_user` now covers it. **This is FR-196a's instrument doing exactly the job it
was built for** — a hand-maintained list of provenance leaves, policed by a derived set.

### F7 (found by an existing instrument, fixed) — the oracle-projection partition was blind on one setting

`every_money_leaf_either_projects_or_is_named_in_oracle_invisible` redded with the four balances
"unaccounted". The cause is the T11 lesson verbatim: `probe_fixtures()` measured only
`mortgage_within_debt_limit == Some(true)`, where the apportionment never runs, so all four balances
looked invisible — while in fact they **do** reach the oracle row, through the FILED line 8a that
`mortgage_interest` projects.

The fix is a **third probe fixture**, not an `ORACLE_INVISIBLE` entry, which would have been a lie. It
sets `Some(false)` plus coherent balances (line 11 = $500,000 against line 12 = $1,000,000, so line 14
= 0.500 and each of the four moves it when perturbed). ★ The balances had to be set coherently rather
than left at the derived `1000 + 137 * i` probe amounts: in field order those give line 12 the largest
value but still **below** lines 1 + 2 + 7, which trips `Line12BelowItsComponents` and makes the
worksheet refuse rather than apportion — a fixture that refuses measures nothing.

## VALIDATION — no oracle was used, and none could be

**Both oracles consume Schedule A line 8a as an INPUT** (`FOLLOWUPS.md` §G-9), so a green two-oracle
sweep proves nothing about a single line of Table 1. No oracle run is cited anywhere in this work, and
`pub936_table1.rs`'s module docs say so at the top so a future reader cannot mistake the omission for an
oversight.

Every asserted figure comes from an IRS document, with its source named in the test:

| KAT | expectation source |
|---|---|
| `pub936s_own_worked_example_reaches_its_printed_line_15_and_line_16` | **Pub. 936 (2025), the *Line 16* instructions' business-allocation Example**, extract lines 1122-1148: *"Mortgage A had an average balance of $90,000, and mortgage B had an average balance of $110,000"* and *"$200,000 (the total average balance of all mortgages)"* ⇒ **line 12 = $200,000**; *"the amount on line 13 (the $30,000 total interest paid)"* ⇒ **line 13 = $30,000**; *"You determine that $15,000 of the interest can be deducted as home mortgage interest"* ⇒ **line 15 = $15,000**; *"The amount on Table 1, line 16, of the worksheet ($15,000)"* ⇒ **line 16 = $15,000** |
| `i1040sca_says_the_750000_limit_is_reduced_by_the_1000000_debt_and_line_11_agrees` | **a second IRS document.** i1040sca (`i1040sca--2025.txt:1039-1046`): *"the $750,000 limit for debt taken out after December 15, 2017, is reduced by the amount of your qualifying debt subject to the $1,000,000 limit."* For $400,000 pre-2017 plus $500,000 post-2017 that is $400,000 + ($750,000 − $400,000) = **$750,000**, and Table 1's lines 9/10/11 must produce it with the subtraction typed nowhere. The test also asserts the sentence is still in the extract. |
| `grandfathered_debt_consumes_the_1000000_bucket_as_the_publication_says` | Pub. 936: *"The limits above are reduced (but not below zero) by the amount of your grandfathered debt"* plus *"Grandfathered debt isn't limited… However, the amount of your grandfathered debt reduces the limit for home acquisition debt."* ⇒ line 11 == line 1. All three sentences asserted present. |
| `the_line_6_branch_at_the_printed_750000_threshold_is_inclusive` | the branch's own *"or more"*, plus a cent-below twin so the word is exercised |
| `the_line_12_stop_makes_all_interest_deductible` | the STOP's own *"All of your interest on all the mortgages included on line 12 is deductible"* |
| `mfs_halves_both_printed_ceilings_all_the_way_to_line_11` | lines 3 and 8's printed *"($500,000 if married filing separately)"* / *"($375,000 …)"* |
| `line_14_rounds_to_three_places_the_way_the_form_says` | *"(rounded to three places)"* — 1/3 prints 0.333, so line 15 is 0.333 x line 13, **not** a third of it |
| `the_apportionment_multiplies_every_component_by_line_14` | *Claiming your deductible points*, item 2 |

★ **The one derived thing, with its equivalence proof.** The first branch bullet's first clause — *"If
you have no home acquisition debt incurred after December 15, 2017"* — is read as `line 7 == 0`. Proof,
written beside the branch: with line 7 = 0, line 10 = line 6 and line 9 = max(line 6, line 8) ≥ line 6,
so line 11 = min(line 9, line 10) = line 6, **identically what the branch prescribes**, unconditionally.
**Where it breaks:** nowhere for line 11; only for *which lines are blank* on the printed sheet, and
Table 1 is a worksheet btctax files no copy of. `both_limbs_of_the_line_6_branch_agree_on_line_11` walks
a 3 x 3 x 5 grid (lines 1, 2, every filing status) and asserts the two agree.

★★ **Honesty note on the publication KAT's inputs.** Pub. 936's example does not state Part I. It states
line 12, line 13, line 15 and line 16, and those four fix the qualified loan limit arithmetically: line
14 = 15,000 / 30,000 = 0.500, so line 11 = 0.500 x 200,000 = $100,000. The Part I facts in the vector
are *a* fact pattern producing that limit; what is ASSERTED is the publication's $15,000 and $15,000.
The test says so in its own comment. There is exactly one example in Pub. 936 (2025) that states Table 1
outputs, and this is it.

## Conformance — the doc comment IS the checked artifact

**32 tests** in `pub936_table1.rs`, **5** of them `#[should_panic]` B1 kills.

`every_numbered_line_doc_comment_is_verbatim_in_the_extract` parses **this module's own source**
(`include_str!("pub936_table1.rs")`), pulls the quoted span out of each `pub lineN:` field's doc block,
and matches it against the extract's Table 1 block — so there is **exactly one copy** of each line's
text and it cannot drift from a second `&'static str` table. Truncation is caught structurally: on the
worksheet every instruction runs into the dot leaders that reach its answer box, so a quote that ends
anywhere else stopped early.

Derived sets, never hand lists:

- the **line numbers** come off the extract (`printed_line_numbers`), not from `1..=16`;
- the **instructed-line set** {1, 2, 7, 12, 13, 16} comes off the `Line N` headings the Instructions
  print, so a quote for a heading the form does not print — or a heading nobody transcribed — reds;
- the **archived revisions** come off `legal/text/irs-publications/`, so a later Pub. 936 that reworded
  one line reds this module until it is re-transcribed. **This is the stated boundary in place of a
  per-year revision table**: every constant Table 1 prints is statutory and unindexed
  (§163(h)(3)(B)(ii), §163(h)(3)(F)(i)(II), the two bucket dates), and P.L. 119-21 made the second pair
  permanent — unlike `state_local_refund`, whose lines 5 and 6 print a *year's* standard deduction;
- the **two-column gutter** is measured per page (it is 61 on one page and 64 on another), never typed;
- the **page footer** is dropped before reflowing — leaving it in spliced a page number into the middle
  of the Line 2 instruction, which crosses a column break. *(That was found while building: the check
  redded reading "…and enter the total on **16** line 2. Include…".)*

Two normalisations, with their blind spots named in the source: `norm` (whitespace only) for the
single-column worksheet block, and `norm_wordwise` (whitespace **and hyphens** removed from both sides)
only for the two-column Instructions prose, because pdftotext cannot distinguish a soft hyphen in
*"after Octo- ber 13"* from a real one in *"mixed-use"*. `the_worksheet_block_has_no_soft_hyphens` is
what licenses using the stronger comparison on the sixteen line texts, and reds if a revision reflows
the block.

## ★ B1 — "which test reds when this is reverted?" with pasted red output

Five mutations were applied to the real source, run in the foreground, and reverted. The files were
restored from byte-checked `cp` backups (never a checkout, per the standing note about uncommitted work
being swept), and `md5sum` confirmed byte-identity afterwards.

**M1 — line 6's `.min` becomes `.max` ("enter the smaller of" becomes "the larger of").** 6 tests red,
including the publication KAT:

```
FAIL btctax-core tax::pub936_table1::tests::pub936s_own_worked_example_reaches_its_printed_line_15_and_line_16
  assertion `left == right` failed: line 13 — $14,000 + $16,000
    left: None
   right: Some(30000)
FAIL ...::i1040sca_says_the_750000_limit_is_reduced_by_the_1000000_debt_and_line_11_agrees
  assertion `left == right` failed: the pre-2017 bucket, inside its $1,000,000 ceiling
    left: 1000000
   right: 400000
FAIL ...::mfs_halves_both_printed_ceilings_all_the_way_to_line_11
  assertion `left == right` failed: ★ MFS is capped at the halved ceiling
    left: 500000
   right: 375000
FAIL ...::both_limbs_of_the_line_6_branch_agree_on_line_11
FAIL ...::line_14_rounds_to_three_places_the_way_the_form_says
FAIL ...::the_apportionment_multiplies_every_component_by_line_14
```

**M2 — the Part II STOP branch dropped (`if line11 >= line12` becomes `if false`).** 5 tests red:

```
Summary  33 tests run: 28 passed, 5 failed
FAIL ...::the_line_12_stop_makes_all_interest_deductible
  assertion `left == right` failed
    left: Limited { ratio: 1, deductible: 24000, not_home_mortgage_interest: 0 }
   right: AllDeductible
FAIL ...::the_all_grandfathered_situation_reaches_the_line_12_stop
  "All of the interest you paid on grandfathered debt is fully deductible" — even at $2.5M, and even
  for MFS, whose ceilings are halved
    left: Limited { ratio: 1, deductible: 95000, not_home_mortgage_interest: 0 }
   right: AllDeductible
FAIL ...::the_within_the_limits_situation_reaches_the_line_12_stop
FAIL ...::mfs_halves_both_printed_ceilings_all_the_way_to_line_11
FAIL ...::a_zero_line_12_stops_before_the_division
```

**M3 — `schedule_a_parts` never apportions.** Three tests red, one of them an *existing* instrument:

```
FAIL btctax-core::kat_attestation fr200a_an_over_limit_mortgage_files_once_pub936_table_1_is_supplied
  assertion `left == right` failed: ★ line 8a carries line 14's fraction — 0.750 x $40,000 — and NOT
  the whole Form 1098 amount, which is what would have understated the tax
    left: 40000
   right: 30000
FAIL btctax-core::oracle_projection every_money_leaf_either_projects_or_is_named_in_oracle_invisible
FAIL btctax-core::kat_attestation fr200a_apportionment_can_flip_the_63e_election_to_the_standard_deduction
  ★ 0.750 x $16,000 = $12,000 LOSES to the $14,600 standard deduction, so §63(e) elects the standard
  deduction — the apportioned total is the right side of that comparison
```

**M4 — the FMV limit widened into silence (its `return refusal(...)` neutered).** Two tests red:

```
FAIL btctax-core::kat_attestation fr200a_an_over_limit_mortgage_files_once_pub936_table_1_is_supplied
  assertion `left == right` failed: ★ "widening an exemption is never the safe edit" — the fourth
  limit is still a wall, and a DIFFERENT one, because no figure the filer supplies can cure it
    left: None
   right: Some("MortgageFairMarketValueLimit")
FAIL btctax-core tax::return_refuse::tests::the_over_limit_refusal_states_both_directions_and_names_the_pub_936_worksheet
  the fair-market-value limit still refuses
```

**M5 — the wall restored (the `Ok(_) => {}` arm made to refuse again).** The fix's own test reds:

```
FAIL btctax-core::kat_attestation fr200a_an_over_limit_mortgage_files_once_pub936_table_1_is_supplied
  assertion `left == right` failed: ★★★ THE FIX: an over-limit itemizer with Pub. 936 Table 1's
  figures FILES. Their Form 8949 and Schedule D — the part this product exists to produce — are no
  longer collateral damage of a worksheet nobody typed in.
    left: Some("MortgageOverDebtLimit")
   right: None
```

Both mutated files were then restored and the whole gate re-run from scratch: the 3855-test / 23.0s run
quoted at the top of this report IS that run.

### The five `#[should_panic]` kills inside the module

Each plants a human mis-transcription rather than the checker's own vocabulary (FR-235):

| test | plant |
|---|---|
| `a_paraphrased_line_is_rejected` | *"Enter the lesser of the amounts on lines 4 and 5"* — what a careful summariser writes for line 6 |
| `a_truncated_line_is_rejected` | line 11 without *"This is your qualified loan limit"* — losing the only place the worksheet names its own output |
| `a_wrong_line_cross_reference_is_rejected` | **the Form 6251 line-33 defect, planted here**: line 16 subtracting from line **12** (a balance) instead of line 13 |
| `dropping_a_line_reds_the_completeness_check` | a transcription missing line 14 (the divide) |
| `a_branch_comment_that_drifts_from_the_checked_text_reds` | `figure`'s inline narration tightened from *"stop here"* to *"stop"* |

★ `every_branch_text_appears_verbatim_in_figures_own_comments` is the anti-drift half: `figure` narrates
each branch inline beside the code that implements it, and that narration must be byte-identical (modulo
whitespace) to the `const` the extract is checked against. **It redded on my own drift while being
written** — the comment markers were still in the normalised text, so the check read *"…or the amount on
`//` line 6 is $750,000…"* and found nothing. It fails loudly rather than passing falsely, which is how
it was found.

## Files changed (all in the worktree, uncommitted)

```
 crates/btctax-core/src/tax/pub936_table1.rs    | 1995 (NEW)
 crates/btctax-cli/src/cmd/tax.rs               |   11 +
 crates/btctax-core/src/tax/classifier.rs       |   54 +
 crates/btctax-core/src/tax/mod.rs              |    5 +
 crates/btctax-core/src/tax/return_1040.rs      |  220 ++-
 crates/btctax-core/src/tax/return_inputs.rs    |   32 +-
 crates/btctax-core/src/tax/return_refuse.rs    |  163 +-
 crates/btctax-core/src/tax/scrub_axis.rs       |   20 +
 crates/btctax-core/tests/kat_attestation.rs    |  313 +++
 crates/btctax-core/tests/oracle_projection.rs  |   34 +-
 crates/btctax-input-form/src/attribute.rs      |   54 +-
 crates/btctax-input-form/src/spec/coverage.rs  |   23 +
 docs/examples/examples.md                      |    1 +
 docs/income-import-schema.md                   |   18 +-
```

What each non-obvious one is for:

- **`classifier.rs`** — the answered-ness census. The block is class `SerdeRequired` (answered-ness
  lives one level up, in whether the `Option` is `Some`), with the two boolean conditions RECORDED
  rather than bound with `_`, exactly as `state_local_refund`'s are.
- **`scrub_axis.rs`** — the maximal PII sentinel realises the block (a maximal fixture leaves no
  `Option` at `None`), with balances chosen to reach the line-12 STOP so it does not contradict the
  sentinel's own *"yes, inside every limit"* answer.
- **`coverage.rs`** — one `EXEMPT_LEAVES` entry (`schedule_a.pub936_table1`), narrow and with FR-200b
  named as the task that removes it. A leaf, not a prefix: the prefix ratchet is at its ceiling, and a
  leaf covers the block only while the fixture leaves it `None`.
- **`attribute.rs`** — refusal-to-form-field anchors for the four new/changed reasons, plus the
  `NotInForm` count test updated with its reason.

Nothing in `crates/btctax-oracle-harness/**`, `crates/xtask/**` (only *run*, to regenerate two generated
docs), `design/forms/extract/**`, `legal/**`, any existing `design/agent-reports/REPORT-*`, or
`FOLLOWUPS.md` was modified. Form 8283 / noncash charitable gifts were not touched. No subagents or forks
were spawned. No gate command was backgrounded.

★ **`docs/examples/examples.md` regenerated, and the diff inspected rather than trusted** — *"a golden
cannot validate its own regeneration"*. The entire change is one line, `"pub936_table1": null,` in a
serialized dump. Nothing else moved.

## Recommendations (for `FOLLOWUPS.md` — not filed, per the brief)

1. **FR-200b — a form section for `[schedule_a.pub936_table1]`.** The block is reachable today only
   through an `income import` TOML; the v1 editor has no section for it. Two new exemption entries (one
   `EXEMPT_LEAVES`, one pair of `Anchor::NotInForm` arms) name FR-200b as the task that removes them.
   Same shape and same priority as FR-196b. **Prompt wording is the deliverable, not plumbing**: the
   fair-market-value and April-2018 conditions each need a prompt that states exactly what permits a
   YES, and a wrong prompt turns a required refusal into a computed deduction.
2. **The average-balance sub-worksheets are not transcribed.** Pub. 936's *Average Mortgage Balance*
   section prints three methods (average of first and last balance; interest paid divided by interest
   rate; the monthly-statement average) plus the mixed-use per-category method. btctax collects the
   *result* and the filer figures it, exactly as Schedule A line 8c already works for points. The
   publication's own worked examples are ready-made KATs the day they are transcribed: $2,500 / 0.09 =
   $27,778 for the interest-rate method, and $150,000 (2025) / $179,750 (2026) for the mixed-use
   per-category method.
3. **Considered and DECLINED: a `Pub936FactsWithoutAMortgage` refusal.** FR-196 added
   `StateLocalRefundFactsWithoutARefund` because that block's figures could reach Schedule 1 line 1 with
   the gate question never asked. The analogue does not arise here: Table 1 cannot reach line 8a unless
   `mortgage_within_debt_limit == Some(false)`, which is itself an answer to a live question, so a stray
   block on a mortgage-less return prints nothing and moves no figure. Recorded so the asymmetry with
   FR-196 is a decision rather than a gap.
4. **`Line14DivisionFailed` is unreachable by construction** — line 12 = 0 forces the STOP, since line
   11 ≥ 0 always — and is carried as a value rather than a panic because this crate's compute core does
   not panic. It shares `Pub936Table1Line12BelowItsComponents`'s `RefuseReason` with a "please report
   this" message. If a future reviewer wants it distinguishable it needs its own variant; today that
   would be a variant with no reachable fixture.
5. **Line 14's tie-break is a stated choice.** The form says *"rounded to three places"* and names no
   tie-break; this uses the IRS's own half-up convention (`DOLLAR_ROUNDING`). A tie at the fourth decimal
   is the only observable case and moves line 15 by at most `0.0005 x line 13` in the filer's favour.
   Stated in the module docs so it is reviewable rather than incidental.
6. **A journey walk is worth scheduling.** This was built as a transcription, and the standing rule says
   the live walk finds what correctness passes cannot. The moment to walk: *"I typed `btctax income
   answer`, said NO to the debt-limit question, and now what?"* — the answer is an `income import` TOML,
   which is exactly the seam recommendation 1 exists to close.
