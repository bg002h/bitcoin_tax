# Lens: Five children at $1M — what actually changes

## VERDICT — can this slice FILE today?

**Files clean.** The dependents slice is one of the most finished areas of the repo, and the trap in
the brief is real: at this income the children change **no federal line on this return**. Schedule
8812's absence is a **correct non-gap**, not a gap. AGI is ~$2,000,000 ($1M wages + $1M LTCG; the
$1M charitable contribution is below-the-line and the student-loan-interest deduction is $0, its
TY2024 MFJ phase-out ending at $195,000 MAGI). Schedule 8812's own arithmetic, transcribed from
`design/forms/extract/f1040s8--2024.txt:11-41`, runs: L1 = 2,000,000 → L3 = 2,000,000; L9 = 400,000
(MFJ, the form prints the constant); L10 = 1,600,000; L11 = 5% × 1,600,000 = **$80,000**; L8 ≤ 5 ×
$2,000 = **$10,000**. L12 asks *"Is the amount on line 8 more than the amount on line 11?"* — No —
and the form answers itself: *"**No. STOP.** You cannot take the child tax credit, credit for other
dependents, **or additional child tax credit**. Skip Parts II-A and II-B. **Enter -0- on lines 14 and
27.**"* So 1040 L19 = $0 and L28 = $0 are the figures the form itself dictates, no Schedule 8812 is
required to be attached, and btctax prints exactly those. `ctc_provably_zero`
(`crates/btctax-core/src/tax/advisories.rs:471-515`) proves this from a MAGI **lower bound** and
flips the advisory text to *"NOT AVAILABLE TO YOU … costs you NOTHING … that is the correct figure —
there is no Schedule 8812 for you to file"* (`advisories.rs:217-227`).

Five dependents print correctly and are fully tested: four rows on the page-1 grid, the *"If more
than four dependents"* box checked, and a `dependents_statement.txt` continuation page emitted into
the packet with an `attach` manifest entry. This is not theoretical — `crates/btctax-cli/tests/
nine_dependents_scenario.rs` drives a **nine**-dependent return with AMT and Form 8995-A through the
real assembly path end-to-end and asserts it files. Five is strictly easier.

Two caveats keep this from being a clean "nothing to do": **§21 survives** (below), and a
dependent-care FSA — very common in this household — hard-**refuses** the whole return.

## WHAT IS MISSING

**1. §21 Child and Dependent Care Credit — the one credit that survives $2M AGI. Overstates tax by
up to $1,200.**
The brief's suspicion is correct. §21(a)(2) reduces the 35% applicable percentage by 1 percentage
point per $2,000 (or fraction) of AGI over $15,000 **but not below 20%** — it floors, it does not
phase out. At $2,000,000 AGI the percentage is 20%. §21(c) caps employment-related expenses at
$3,000 (one qualifying individual) / $6,000 (two or more) — not indexed, and ARPA's $8,000/$16,000
expired after 2021. Max credit = 20% × $6,000 = **$1,200**. It is nonrefundable, but §26(a) (as made
permanent by ATRA 2012 §104) allows nonrefundable personal credits against regular tax **plus** the
§55(a) AMT, so the AMT in this vector does not extinguish it, and the regular tax here dwarfs any
Credit Limit Worksheet cap.
Destination: **Schedule 3 line 2** — *"Credit for child and dependent care expenses from Form 2441,
line 11. Attach Form 2441"* (`design/forms/extract/f1040s3--2024.txt:17`) → 1040 L20.
btctax: `Schedule3Lines` has **no `line2` field** (`crates/btctax-core/src/tax/printed.rs:1421-1436`,
with `let line8 = line1; // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)` at
`printed.rs:1442`) and `Schedule3Map` has **no `line2` cell** (`crates/btctax-forms/src/map.rs:
1550-1569`). That is the *correct* posture — the cell is structurally unwritable, so this is a
genuine blank and one of the few places §G-11 does not bite. `Advisory::OtherCreditsOmitted`
(`advisories.rs:369-374`, pushed unconditionally at `advisories.rs:752`) names Form 2441 explicitly.
**Consequence: overstates tax by up to $1,200** — but only if the household has (a) at least one
dependent under 13 (§21(b)(1)(A)), (b) actual employment-related expenses, and (c) **both** spouses
with earned income, since §21(d)(1) reduces the dollar limit to the lesser of the two spouses'
earned income on a joint return. A single-earner $1M MFJ household gets $0 regardless, absent the
§21(d)(2) student/incapacitated-spouse rule. So this is real but **conditional on facts the vector
does not state**.

**2. W-2 box 10 dependent-care benefits REFUSE the entire return.**
`crates/btctax-core/src/tax/return_refuse.rs:968-972` refuses unconditionally on `box10_dependent_care
> Usd::ZERO` with `RefuseReason::DependentCareBenefit` ("W-2 box 10 dependent-care benefits require
Form 2441"). The §129 dependent-care FSA exclusion is **$5,000 and income-independent** — it does not
phase out — so a $1M MFJ household with five children is a *likely* holder, and the employer has
already excluded it from box 1. Form 2441 Part III exists to substantiate that exclusion.
**Consequence: REFUSES.** This is the honest gate (filing with an unsubstantiated §129 exclusion
would understate), but it means a plausible instance of this exact vector cannot file at all, and the
cause is dependent-driven. Worth naming in the plan because it is invisible until a W-2 carries the
box.

**3. `LIMITATIONS.md:171` still makes the false claim the advisory was fixed to stop making.**
The row reads: *"1040 line 19 is pinned to **$0**. | File Schedule 8812 yourself. Your tax is
overstated by up to that amount."* Unconditional. This is precisely the defect removed from
`Advisory::CtcOdcOmitted` by the `provably_zero` branch (`advisories.rs:46-51` records the filing
trial that caught it: a filer at AGI $2,085,000 with nine children told to claim $18,000 that §24(b)
had already erased). The advisory was fixed; the document was not, and `grep -n "8812\|overstated"
crates/btctax-cli/tests/limitations.rs` returns **nothing** — no test pins this row.
**Consequence: a filer matching this vector who reads LIMITATIONS.md prepares and files a Schedule
8812 that pays $0**, contradicting the advisory their own report printed. Minor severity (no filed
figure is wrong), but it is a doc that actively contradicts the code for exactly this household.

**4. Stale doc comment at `crates/btctax-forms/src/map.rs:467-470`.**
`more_than_four_dependents` is documented as *"v1 REFUSES instead (the continuation statement is a
synthetic page generator we do not have …). Mapped so the refusal can name the cell it will not
fill."* Contradicted by `crates/btctax-forms/src/form1040_full.rs:519`, which now *checks* the box,
and `crates/btctax-forms/src/packet.rs:228-230`, which emits the statement.
**Consequence: no filing defect** — but this is the comment that would talk a future reviewer into
re-adding a refusal that §G-28/B2 deliberately removed. Nit/Minor.

**Explicitly NOT missing** (checked, all correctly nil at this income, none of them worth a plan
item): CTC/ODC and ACTC (above); EIC (`EicOmitted` is AGI-gated, `advisories.rs:745`); §25A AOTC and
Lifetime Learning (MFJ MAGI phase-outs end at $180,000); §23 adoption (TY2024 MAGI $252,150–
$292,150); §36B PTC. Dependents do **not** touch the AMT — §55(d) has no dependent term, personal
exemptions are suspended by TCJA and disallowed for AMT by §56(b)(1)(E) regardless. They do not touch
NIIT (§1411(b)'s $250,000 MFJ threshold is statutory, unindexed, no dependent term) or the Additional
Medicare Tax ($250,000 MFJ, same). They do not touch the standard deduction — §63(c)(5) turns on the
*filer* being claimable, not on having dependents — and this household itemizes anyway.
`RefuseReason::KiddieTax` (`return_refuse.rs:202-204`) is likewise a non-issue: it is predicated on
the *filer* being claimable as a dependent, not on the children, whose own unearned income belongs on
their own returns.

**Printing five dependents — verified, no gap.** `DEPENDENTS_GRID_ROWS = 4`
(`crates/btctax-core/src/tax/dependents_statement.rs:28`), from the form's four
`Table_Dependents[0].RowN[0]` groups. `dependents_split()` (`:40-43`) partitions via `split_at`, so
grid ++ overflow reproduces the input in capture order by construction. `more_than_four_dependents()`
(`:50-52`) is **one predicate with two consumers** — the page-1 checkbox and the statement — so "box
checked, no statement" and "statement, no box" are both inexpressible. `form1040_full.rs:507-516`
refuses at fill time if the map's `dependent_rows.len()` disagrees with core's constant. Column (4)
prints its two check positions **empty** and asserts nothing (`dependents_statement.rs:100-133`;
`the_rendered_statement_transcribes_all_four_columns_and_claims_nothing` at `:198` forbids the
strings "NOT CLAIMED"/"no credit"). Tests: `the_split_is_an_exact_ordered_partition` (n = 0..=12),
`more_than_four_dependents_checks_the_box_and_prints_the_first_four`
(`crates/btctax-forms/tests/full_return_forms.rs:2188`), and the end-to-end nine-dependent scenario.
`wrap.rs`/`overflow.rs` are **not** in this path — `overflow.rs:1-5` is Form 8949 11-rows-per-part
pagination, and the dependent name is a single cell spanning the printed first/last columns
(`map.rs:476-478`), so nothing wraps.

I ran `cargo nextest run -p btctax-core ctc_phaseout`: **8/8 pass**, including
`nine_children_at_two_million_agi_is_provably_zero`, `the_gate_being_unasked_does_not_defeat_the_
proof` (the production state — the §911/931/933 gate is `live: |ri| ri.tax_year >= 2025` and so is
never asked on TY2024), and `the_phase_out_boundary_is_the_forms_own_rounding` (MFJ/2 deps: $479,000
survives, $479,001 does not).

## THE SMALLEST THING THAT CLOSES IT

Sequenced, smallest first. **Items 1–2 are the whole plan for this slice; items 3–4 are real work but
should not be scoped to this vector.**

1. **Fix `crates/btctax-cli/LIMITATIONS.md:171` and pin it with a test.** Split the "What to do"
   cell so it matches the two advisory branches already in the code: when §24(b) provably removes the
   credit, *"line 19 is $0 and that is the correct figure — there is no Schedule 8812 for you to
   file"*; otherwise the existing "overstated" text. Then add the missing kill-test to
   `crates/btctax-cli/tests/limitations.rs` asserting the doc row and
   `Advisory::CtcOdcOmitted::message()` do not contradict each other — the checker that would have
   caught this never existed (B1: no test currently reds when this row is wrong). One paragraph plus
   one test; no engine change.

2. **Delete the stale refusal claim at `crates/btctax-forms/src/map.rs:467-470`** and point the doc
   at `ReturnHeader::more_than_four_dependents` / `dependents_statement()`. One comment.

3. **§21 / Form 2441 — file as a follow-up with an owning phase, do NOT build it for this vector.**
   The credit is at most $1,200, is zero on a single-earner household, and needs input btctax does
   not collect. If it is ever built, the sequence is forced by the form and should be stated that way:
   (a) extract `f2441--2024.txt` and `i2441--2024.txt` into `design/forms/extract/` — the repo has
   neither today, which is why nothing here can be transcribed yet; (b) **collect** three new input
   groups, which is following instructions, not scope creep: Part I care-provider name / address /
   TIN (one row per provider), per-dependent qualifying-individual status and employment-related
   expenses paid, and each spouse's earned income (or §21(d)(2) student/disabled status) — the last
   is the one that decides whether the credit is $0; (c) transcribe Form 2441 lines 1–11 (Part II) and
   lines 12–26 (Part III) one field per numbered line onto a new `Form2441Lines` struct next to
   `Form8995Lines` in `printed.rs`; (d) add `line2: Usd` to `Schedule3Lines` (`printed.rs:1421`) and
   `line2: MoneyCell` to `Schedule3Map` (`map.rs:1550`), and extend `line8`'s sum — note that adding
   the field `E0063`s every `Schedule3Lines` literal, which is the free blast radius; (e) narrow
   `RefuseReason::DependentCareBenefit` from "box 10 > 0" to "box 10 > 0 **and** Part III unanswered",
   so a filer who answers Form 2441 Part III stops being refused. Owning phase: whichever phase takes
   the credits surface. **Not this one.**

4. **Do not build Schedule 8812.** For this vector it would be a form the IRS instructions say not to
   attach. If it is built later, the trigger is item 5 below, and the existing `ctc_provably_zero`
   already encodes L1–L12 correctly against the extracted text — the work is Parts II-A/II-B and a map.

5. **The income level that makes Schedule 8812 a real gap** (the brief asked; stating it so the
   synthesis agent can scope a future vector). Solve `deps × $2,000 > 50 × ceil((MAGI − 400,000) /
   1000)` for MFJ. With **five CTC-qualifying children**: the credit survives while
   `ceil(over/1000) ≤ 199`, i.e. **MAGI ≤ $599,000** — at exactly $599,000 the credit is $10,000 −
   $9,950 = **$50**; at $599,001 it is $0. Below $400,000 it is the full **$10,000**. If some
   dependents are ODC-only at $500, the window shrinks — all five ODC-only gives L8 = $2,500 and a
   zero point of **MAGI $449,000**. **This vector sits at ~$2,000,000, i.e. 3.3× above the top of the
   window.** A "children matter" vector needs MAGI at or under roughly $600,000 MFJ, which is a
   materially different household from this one.

## WHAT I AM NOT SURE OF

- **The §21 figures are from statute as I hold it, not from a primary source in this repo.** Form
  2441 and its instructions are **not** in `design/forms/extract/` (I listed the directory — 60-odd
  files, no `f2441`/`i2441`), so the 20% floor, the $3,000/$6,000 caps, and the §21(d)(1) earned-
  income limit could not be checked against an extracted text layer the way I checked Schedule 8812
  line-by-line. House doctrine says transcribe from the text layer; I could not, and flag it. If the
  synthesis agent wants the $1,200 figure load-bearing, extract Form 2441 first. My confidence in the
  *direction* (the credit survives $2M AGI, unlike every other dependent credit) is high; my
  confidence in the exact dollar cap is good but unverified here.
- **§26(a)'s AMT interaction.** I am confident nonrefundable personal credits are allowed against
  the §55(a) AMT (ATRA 2012 made the §26(a)(2) provision permanent), but I did not verify it against
  a source in-repo, and this vector has live AMT. It only matters if item 3 is ever built.
- **Which tax year the plan targets.** All my arithmetic is TY2024 ($2,000 CTC, $1,700 ACTC —
  the latter confirmed from `f1040s8--2024.txt:52`, $400,000 MFJ threshold). `f1040s1a--2025.txt` and
  a `2025` extract set exist, and the CTC per-child amount and the §21 caps changed for 2025+ under
  the 2025 reconciliation act. The **conclusion is unaffected** — the phase-out still erases the
  credit many times over at $2M — but the exact $599,000 boundary in item 5 is a TY2024 number and
  would need re-deriving for TY2025.
- **Whether the vector's household is single- or dual-earner.** The brief says "$1,000,000 ordinary
  income (wages)" without attributing it. This single fact decides whether item 3 is worth $1,200 or
  $0, and I could not settle it. If the plan wants a dependents-driven finding with teeth, it should
  pin this down first.
- **Whether the MFJ × 5-dependent combination is pinned end-to-end.** It is not, as far as I can
  tell: the executable `nine_dependents_scenario.rs` fixture is **`filing_status = "Single"`**
  (`crates/btctax-cli/tests/fixtures/examples/nine_dependents_amt_inputs.toml:51`), and the MFJ
  threshold branch is covered only by unit tests on `ctc_provably_zero`. The split logic is
  filing-status-independent by construction so I do not believe this is a defect, but I did not
  execute an MFJ overflow return, and I did not run the full suite — only the eight `ctc_phaseout`
  tests.
