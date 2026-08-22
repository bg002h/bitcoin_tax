# Lens: SALT + mortgage + student loan + the itemized surface

Vector: MFJ, $1,000,000 wages + $1,000,000 LTCG, $1,000,000 cash gift to a church, 5 dependents,
homeowner with a mortgage, SALT paid, student loan interest paid. AGI = $2,000,000.

## VERDICT — can this slice FILE today?

**Files — but line 8a is WRONG for the mortgage this household almost certainly has, and the error
direction is UNDERSTATEMENT.**

Two refusals in my slice are LIVE and must be answered before anything files
(`MixedUseMortgageUnanswered`, `AmtQualifiedDwellingUnanswered` — both keyed to
`mortgage_interest_1098 > 0`, `crates/btctax-core/src/tax/questions.rs:244-248`). Answered
neutrally (all proceeds used to buy/build/improve; the dwelling is a principal residence), the
household clears every gate and Schedule A emits: line 4 = 0, line 7 = **$10,000** (§164(b)(6) flat
cap, correct), line 8a = the full Form 1098 interest, line 14 = $1,000,000, line 17 ≈ $1,010,000 +
mortgage interest. SALT is transcribed correctly and is year-aware. The §63(e) line-18 box correctly
stays unchecked. **The defect is line 8a**: btctax asks only the §163(h)(3)(F) *use* question and
never asks the §163(h)(3)(B) *amount* question, so a $2,000,000-income filer with a jumbo mortgage
deducts 100% of interest on debt the statute caps at $750,000. Nothing refuses, nothing advises, and
no oracle catches it — both oracles take the deductible interest as an INPUT (the §G-9 limit
exactly). Separately, the whole path is **TY2024-only**: `BundledFullReturnTables::load` inserts
only 2024 (`crates/btctax-adapters/src/tax_tables.rs:101`) and `schedule_a_pdf` maps only 2024
(`crates/btctax-forms/src/pdf.rs:183-188`), so the TY2025 OBBBA SALT worksheet — which is written
and unit-tested — is unreachable.

## WHAT IS MISSING

### 1. §163(h)(3)(B) acquisition-debt CEILING — the $750k / $1M limit is never asked (CRITICAL, understates)

**The form/statute requires.** i1040sca-2024 ("Limits on home mortgage interest",
`design/forms/extract/i1040sca--2024.txt:875-925`) enumerates **four** independent limits, not one:

- proceeds not used to buy/build/**substantially** improve — btctax asks this;
- *"For qualifying debt taken out on or before December 15, 2017, you can only deduct home mortgage
  interest on up to **$1,000,000 ($500,000 if you are married filing separately)** of that debt"* —
  **never asked**;
- *"For qualifying debt taken out after December 15, 2017, you can only deduct home mortgage
  interest on up to **$750,000 ($375,000 if you are married filing separately)** of that debt"* —
  **never asked**;
- *"Limit when loans exceed the fair market value of the home"* — **never asked**.

And line 8a's own instruction (same file, §"Line 8a"): *"Enter on line 8a mortgage interest and
points reported to you on Form 1098 **unless one or more of the limits on home mortgage interest
apply to you**."*

**What btctax does instead.** `crates/btctax-core/src/tax/return_1040.rs:426-433` — line 8a is the
raw `mortgage_interest_1098` unless the mixed-use box is set, in which case it is `Usd::ZERO`.
There is no other branch. The whole input surface for Schedule A is **7 money leaves + 2 tri-states**
(`crates/btctax-input-form/src/spec/sections.rs:788-845`); there is **no** field for loan balance,
origination date, or the home's FMV, and `grep -rn "750000\|acquisition_debt"` over
`crates/btctax-core/src` returns nothing.

**Consequence.** A filer whose mortgage is $2,000,000 of pure post-2017 acquisition debt answers
"yes, all proceeds went to buy the home" — truthfully — and btctax deducts **100%** of the interest
where §163(h)(3)(B)(ii) allows 750/2000 = **37.5%**. At ~6.5% that is ~$130,000 claimed against
~$48,750 allowed: an **$81,000 overstated deduction, ~$30,000 of understated federal tax**, on a
return that files clean with zero advisories. This is the highest-severity item in my slice and it
lands squarely on this household — a $2M-income homeowner is the modal jumbo-mortgage filer.

**Why nothing catches it.** The mixed-use question is about *use*; this is about *amount*. They are
disjoint conditions in the same instruction block, and only one was transcribed. Neither oracle
derives it (both consume Schedule A line 8a), so the two-oracle sweep is blind here by construction.

### 2. §221 student loan interest — collected, computed to zero, and correctly ABSENT here (no consequence for this vector)

**Collected**: `ScheduleAInputs`' sibling `Schedule1Inputs::student_loan_interest_paid`
(`crates/btctax-core/src/tax/return_inputs.rs:559`). **Computed**: `student_loan_deduction`
(`crates/btctax-core/src/tax/return_1040.rs:938-959`) — `min(paid, $2,500)` phased over the MFJ band
$165,000–$195,000 (`crates/btctax-adapters/src/tax_tables.rs:144-145`), $0 for MFS per §221(e)(2).
At MAGI $2,000,000 the result is **$0**, which is right.

**Does it print a zero?** For *this* household, **no** — `schedule_1_lines`
(`crates/btctax-core/src/tax/printed.rs:464`) returns `None` when both Part I and Part II total zero,
and this household has no Schedule 1 income and no other adjustment, so Schedule 1 is not filed at
all and line 21 never exists. That is the correct paper.

**But it is one dollar away from firing.** Add any crypto ordinary income (line 8v) or a state
refund (line 1) and Schedule 1 files; `printed.rs:461` is `let line21 = round_dollar(p.student_loan_21)`
and Schedule 1's filler uses the always-writing `push_money`, so line 21 prints **"0"**.

**Is a computed zero on a phased-out §221 deduction lawful testimony?** Here it is *not what the
instructions produce*. i1040gi-2024 "Line 21" (`design/forms/extract/i1040gi--2024.txt:42211-42221`)
reads *"You can take this deduction **only if all of the following apply** … Your modified adjusted
gross income (AGI) is less than … $195,000 if married filing jointly."* At $2M MAGI the filer is
outside the deduction entirely and never reaches the worksheet, so the intended entry is a **blank**.
(Contrast the case where the filer *is* eligible but phased to zero: the worksheet's line 7 says *"If
the result is 1.000 or more, enter 1.000"* and line 9 says *"Enter the result here and on Schedule 1,
line 21"* — there a computed 0 IS the worksheet's own output and printing it is lawful.) So the rule
is: **eligible-and-phased-to-zero ⇒ 0 is lawful; ineligible ⇒ blank**, and btctax cannot currently
express the second. Consequence: on a Schedule-1-filing variant of this household the return swears
a §221 figure the filer was never entitled to compute. Tax effect: none. Testimony defect: yes.

### 3. Schedule A prints "0" on every mapped line, including four cells of a medical computation nobody asked for (§G-11, visible and large here)

`crates/btctax-forms/src/schedule_a.rs:44-73` builds an 18-entry plan and drives every one through
`push_money`, which "takes a bare `Usd` and always writes"
(`crates/btctax-forms/src/cells.rs:46-47`). `push_money_opt` exists and is used by
`schedule_d_full.rs` and `form8995a.rs` — **Schedule A has zero call sites** (`grep -c push_money_opt
crates/btctax-forms/src/schedule_a.rs` → 0).

For this household with no medical expenses, Schedule A prints: **line 1 = "0"**, **line 2 =
"2000000"**, **line 3 = "150000"**, line 4 = "0" — an entire §213(a) floor computation, with a
$150,000 figure in the box, on a return where the filer never entered a medical expense. Same for
line 5c ("0") and lines 12/13 ("0").

`ScheduleAInputs::medical` is `Usd` with `#[serde(default)]`
(`crates/btctax-core/src/tax/return_inputs.rs:508-509`), so it cannot distinguish "entered 0" from
"never asked". The input-form path *does* pose it (`FieldId::SaMedical`,
`crates/btctax-input-form/src/spec/coverage.rs:540`), but a TOML-import path never does.

**Consequence.** Files an incomplete-provenance return: a sworn zero whose answered-ness depends on
which entry path the filer used. No tax effect. Documented as class-(B) forgone-benefit at
`return_inputs.rs:783` ("omitting a deduction can only OVERSTATE tax"), which is true of the *value*
and silent about the *testimony*.

### 4. Schedule A line 9 / Form 4952 — the gap is CONSISTENT, and it matters only as a forgone benefit

The census records it explicitly: `crates/btctax-forms/forms/2024/f1040sa.map.toml:100` —
`f1_23` = line 9, `rule = "unmodeled"`, *"btctax models no Form 4952, so line 10 equals line 8e."*
Line 10 is `line8e` at `crates/btctax-core/src/tax/printed.rs:1370`.

**Does it matter here?** Three answers, and they line up:

- **No inconsistency.** Form 6251's module doc (`crates/btctax-core/src/tax/form6251.rs:6-13`) lists
  §4952 investment interest among lines 2c–2t as having no field. Since the regular-tax deduction is
  structurally zero, the AMT *difference* is genuinely zero too. Two zeros that agree for the same
  reason — this is the correct kind of gap, not a laundered one.
- **It is a real forgone amount if this filer margin-borrowed against bitcoin.** §163(d) would allow
  the interest against net investment income, and Form 4952 line 4g carries the
  §163(d)(4)(B)(iii) election to treat net long-term capital gain as investment income — a lever
  aimed directly at this household's $1,000,000 LTCG. btctax offers no way to enter it and **fires no
  advisory** (`Advisory` has 18 variants; none names investment interest —
  `crates/btctax-core/src/tax/advisories.rs:53-182`).
- **It costs NIIT too.** `crates/btctax-forms/forms/2024/f8960.map.toml:78-79` records line 9a
  ("Investment interest expenses") and **9b ("State/local/foreign income tax allocable to NII")** as
  unmodeled. 9b is in my slice: the capped $10,000 SALT has an NII-allocable share (roughly the
  NII/AGI ratio, ~50% here), so ~$5,000 of deduction is dropped ⇒ **~$190 of overstated NIIT**.
  Small, safe-direction, but silent.

**Consequence:** overstates tax; never blocks filing. Direction is safe, so it is not a gate — but a
filer paying six figures of margin interest gets **no signal whatsoever**, which is the part worth
fixing.

### 5. The five named refusals — when each fires, and whether this household hits it

| refusal | fires when | this household |
|---|---|---|
| `MixedUseMortgageUnanswered` | `schedule_a.is_some() ∧ mortgage_interest_1098 > 0 ∧ mortgage_all_used_to_buy_build_improve == None` (`questions.rs:244-248`, `:374-400`) | **LIVE — must answer.** "Yes" ⇒ full 8a. "No" ⇒ 8a = $0 + box checked + `MixedUseMortgageNotAllocated` advisory (`return_1040.rs:428-433`, `advisories.rs:836-843`) — forfeits the ENTIRE mortgage deduction, an overstatement that can run tens of thousands on this mortgage. Recovery is FOLLOWUPS.md P9(a) (`mortgage_interest_deductible`, owned by **P8**, still OPEN — `FOLLOWUPS.md:2832-2836`). |
| `AmtQualifiedDwellingUnanswered` | identical liveness (`mortgage_question_live`), `mortgage_dwelling_is_amt_qualified == None` (`questions.rs:401-427`) | **LIVE — must answer.** |
| `AmtNonQualifiedDwelling` | that question answered `Some(false)` (`return_refuse.rs:730-745`) | **Not hit** for a principal residence. A houseboat/RV second home would HARD-REFUSE the whole return (v1 models no §56(b)(1)(C) line-3 add-back). |
| `SaltSalesTaxWithoutElection` | `salt_sales_tax_amount > 0 ∧ salt_use_sales_tax != Some(true)` (`return_refuse.rs:808-815`) | **Not hit** on the natural path (income-tax state, no sales-tax amount entered). |
| `SalesTaxElectionWithoutAmount` | `salt_use_sales_tax == Some(true) ∧ amount == 0 ∧ income_tax_salt(ri,a) > 0` (`return_refuse.rs:821-832`) | **Not hit.** |

Note `salt_use_sales_tax == None` does **not** refuse — it silently takes the income-tax path and
fires the `SalesTaxElectionNotAsked` advisory (`advisories.rs:846-856`). For this household that is
correct and economically inert: W-2 box 17 on $1,000,000 of wages exceeds the $10,000 cap by itself
in any income-tax state, so the election cannot change line 5e. In a no-income-tax state (TX/FL/WA/NV)
it would matter — and jumbo real-estate tax on line 5b would usually still exhaust the cap alone.

### 6. SALT itself is correct — and the correct answer is $10,000 by the TY2024 instrument, not the TY2025 one

`SaltLimitation` (`crates/btctax-core/src/tax/tables.rs:296-345`) is an enum with `FlatCap` (TY2024)
and `Worksheet2025` (the OBBBA 10-line worksheet with the 30% phase-down, the $10,000 floor, and the
MFS halving at worksheet line 10 only), and `line_5e` (`tables.rs:348-397`) transcribes both. The
`Worksheet2025` arm at MFJ / MAGI $2,000,000 gives line 6 = $1,500,000, line 7 = $450,000,
line 8 = −$410,000, line 9 = max(−410000, 10000) = **$10,000** — same answer as `FlatCap`.

**But `Worksheet2025` is never constructed by the shipped tables.** `grep -n "Worksheet2025"
crates/btctax-adapters/src/tax_tables.rs` → no matches; the only `salt:` field in the shipped bundle
is `FlatCap { 10000, 5000 }` at `tax_tables.rs:130-133`, and `BundledFullReturnTables::load` inserts
**2024 only** (`tax_tables.rs:101`), guarded by two deliberate fail-closed tests
(`ty2025_full_return_must_stay_fail_closed_until_complete` at `tax_tables.rs:814`,
`ty2026_full_return_must_stay_fail_closed` at `tax_tables.rs:906`). Reinforced on the emitter side:
`crates/btctax-forms/forms/2025/` contains only f1040, f8283, f8949, schedule_d, schedule_se — **no
2025 Schedule A**, and `schedule_a_pdf` errors `UnsupportedYear` for anything but 2024
(`crates/btctax-forms/src/pdf.rs:183-188`).

**Consequence for the plan (not a defect — a scope fact the plan must state):** this vector is a
**TY2024** vector or it does not file. Corollary: `IncomeExclusionUnanswered` cannot fire (the
`HasIncomeExclusion` question is `live: |ri| ri.tax_year >= 2025`, `questions.rs:504`), and TY2026's
OBBBA itemized rules (the new §68 2/37 haircut, the §170(p) 0.5%-of-AGI charitable floor) are
unmodeled anywhere in `btctax-core` — which is safe only because 2026 fails closed.

### 7. Structurally unreachable Schedule A lines — all accounted for, none silent

Every unmapped field carries a written census decision
(`crates/btctax-forms/forms/2024/f1040sa.map.toml:83-105`), and `f1040sa` is in `CENSUSED` with
`GAPS = 0` (`crates/btctax-forms/tests/field_census.rs:29-51`, `:187`):

- **6** (other taxes, write-in) — `unmodeled`; ⇒ line 7 = line 5e.
- **8b / 8c** (interest not on a 1098; points) — `unmodeled`; only `mortgage_interest_1098` is
  collected. *Relevant here:* a seller-financed or co-borrower mortgage is undeductible in btctax.
- **8d** — `artifact`: the IRS ships a fillable widget for a line whose own text is "Reserved for
  future use". Never written.
- **9** (investment interest) — `unmodeled`; see item 4.
- **15** (casualty/theft, Form 4684) — `unmodeled`.
- **16** (other itemized, write-in) — `unmodeled`.

Populated lines: 1, 2, 3, 4, 5a, 5b, 5c, 5d, 5e, 7, 8a, 8e, 10, 11, 12, 13, 14, 17 + the three
checkboxes (5a sales-tax election, 8 mixed-use, 18 §63(e)). That is the complete answer to "which
lines can btctax populate".

One uncollected input with no consequence here: `income_tax_salt`
(`crates/btctax-core/src/tax/return_1040.rs:188-195`) sums **W-2 box 17/19 only** plus estimates
plus prior-year balance — 1099 state withholding has no field
(`return_inputs.rs:72` is the only `state_tax_withheld`). The $10,000 cap binds on box 17 alone here,
so it cannot move line 5e for this household.

## THE SMALLEST THING THAT CLOSES IT

Sequenced. Items 1 and 2 are the only ones that change a number.

**S1 — Gate the §163(h)(3)(B) debt limit. One declaration, mirroring the two that already exist.**
This is a transcription of an instruction block that is already in the repo, not new tax logic.

1. `crates/btctax-core/src/tax/return_inputs.rs` — add
   `ScheduleAInputs::mortgage_within_debt_limit: Option<bool>` immediately after
   `mortgage_all_used_to_buy_build_improve` (line 532). `#[serde(default)]`, doc comment carrying
   i1040sca "Limits on home mortgage interest" verbatim (the $1,000,000, $750,000/$375,000 and FMV
   limits).
2. `crates/btctax-core/src/tax/return_refuse.rs` — add
   `RefuseReason::MortgageDebtLimitUnanswered` (registry-loop `None` case) and
   `RefuseReason::MortgageOverDebtLimit` (adverse-value case, written exactly like
   `AmtNonQualifiedDwelling` at `:730-745`: *computing without the Pub. 936 Table 1 allocation would
   UNDERSTATE your tax*).
3. `crates/btctax-core/src/tax/questions.rs` — add `QuestionId::MortgageWithinDebtLimit` to
   `QuestionId::ALL` and a `FormQuestion` in `FORM_QUESTIONS`, `live: mortgage_question_live`
   (the **existing** predicate at `:244-248`, unchanged — the liveness is identical to the other two
   mortgage questions), `durability: PerYear`, `neutral: true`.
   Prompt phrased so the *deductible* answer is the affirmative one and every omission fails closed
   (per `widening-an-exemption-is-never-the-safe-edit`): *"Counting all home mortgages together, was
   the balance $750,000 or less all year ($375,000 if MFS) — or, for debt taken out on or before
   December 15, 2017, $1,000,000 or less ($500,000 MFS) — AND not more than the home's fair market
   value?"*
4. `crates/btctax-input-form/src/spec/sections.rs` — one `decl_tristate!` in `SCHEDULE_A_FIELDS`
   (next to `SaMortgageAllUsed` at `:840`), plus its `FieldId` and `coverage.rs` row.
5. **B1 kill-test**: a KAT with `mortgage_within_debt_limit: Some(false)` that asserts the refusal,
   and one with `None` that asserts the unanswered refusal — both must red when the registry entry is
   deleted. The compiler does the rest of the sweep: the new `RefuseReason` variant reds every
   exhaustive cross-crate match, and the new `ScheduleAInputs` field reds the classifier's exhaustive
   destructure (`crates/btctax-core/src/tax/classifier.rs:572`).

**S2 — Recover the money, once, for both mortgage refusals.** FOLLOWUPS.md P9(a)
(`mortgage_interest_deductible`, owned by P8, `FOLLOWUPS.md:2832-2836`) is already filed for the
mixed-use case. **The same input closes S1's adverse branch**: a filer who has run the Pub. 936
"Deductible Home Mortgage Interest Worksheet" enters its result and line 8a takes it, whether the
limit that bit was *use* or *amount*. Fold S1's adverse branch into that follow-up rather than
opening a second one — and note the ownership: P8 now owns two refusals, not one.

**S3 — Make Schedule 1 line 21 expressible as blank.** In
`crates/btctax-core/src/tax/printed.rs`, change `Schedule1Lines::line21` to `Option<Usd>`; set it
`None` when the §221 *eligibility* test fails (MFS, or MAGI ≥ the status ceiling, or `paid == 0`) and
`Some(v)` when the filer is eligible and the worksheet produced `v` — including `Some(Usd::ZERO)`,
which is the worksheet's own line 9. Then use `push_money_opt` at the Schedule 1 call site. Note
`student_loan_deduction` (`return_1040.rs:938-959`) already computes both facts; it currently
collapses them into one `Usd::ZERO` return at `:948` and `:952`.

**S4 — Advise on the deductions this household silently forgoes.** Add one `Advisory` variant
(same shape as `MixedUseMortgageNotAllocated`, `advisories.rs:165`) covering Schedule A line 9 /
Form 4952 and Form 8960 lines 9a–9b, fired when the return itemizes AND has investment income,
naming the §163(d)(4)(B)(iii) Form 4952 line 4g election explicitly because this household's
$1,000,000 LTCG is exactly what it targets. **No Form 4952 implementation** — the advisory is the
whole fix, because the direction is already safe and the missing thing is the *signal*.

**S5 — Schedule A blank-vs-zero (§G-11), scoped.** Convert `schedule_a.rs:44-63`'s plan from
`(Usd, usize)` to `(Option<Usd>, usize)` + `push_money_opt`, `None` for the **Collected** leaves
(1, 5a, 5b, 5c, 8a, 11, 12, 13) when the underlying input was never supplied. This requires the
inputs to become `Option<Usd>` — that is the real work and it is a **collect** change, not a
formatting one. Leave the Combine/Bounded lines (5d, 5e, 7, 8e, 10, 14, 17), line 2 (Carry) and
line 4 (its own "-0-" clause) writing unconditionally. Lowest priority in this slice: no tax effect.

## WHAT I AM NOT SURE OF

- **Whether S1's adverse branch should REFUSE or zero line 8a.** I proposed refuse, mirroring
  `AmtNonQualifiedDwelling`, because the alternative (zero 8a, as mixed-use does) forfeits the entire
  deduction for a filer who is only marginally over $750,000 — a very large overstatement. But
  zeroing keeps the return *fileable*, and the house precedent for an unmodelled allocation is to
  zero and advise, not to refuse. This is a genuine design call for the synthesis pass. Note the
  asymmetry that decided me: mixed-use has a **line-8 checkbox** on the form to affirm the $0; the
  debt limit has **no box**, so a $0 line 8a would be an unexplained blank.
- **Whether a blank line 1 should also blank lines 2 and 3.** Line 2's own text ("Enter amount from
  Form 1040 or 1040-SR, line 11") and line 3's ("Multiply line 2 by 7.5%") are unconditional, so
  strict transcription says print them; ordinary practice leaves the whole Medical block empty when
  there are no medical expenses. I did not find an instruction sentence that settles it and I did not
  read i1040sca's line 1–4 block closely enough to rule one out.
- **The exact §1.1411-4(f)(3)(iv) allocation for Form 8960 line 9b.** I used the NII/AGI ratio to
  size the ~$190 NIIT overstatement; the reg permits any reasonable method, so that figure is
  order-of-magnitude, not exact.
- **The §221 "Exception" (Form 2555 / 4563 / Puerto Rico ⇒ use Pub. 970 instead of the worksheet).**
  `student_loan_deduction` runs the worksheet unconditionally, and the `HasIncomeExclusion` gate is
  deliberately not live before TY2025. Omitting the exclusion add-back would *raise* the deduction
  and understate tax — but at $2,000,000 MAGI the deduction is $0 under any method, so it cannot
  matter for this vector. I did not check whether it matters for a lower-income TY2024 filer.
- **Whether any charitable interaction changes line 17.** I assumed the §170(b)(1)(G) 60%-of-AGI cash
  ceiling ($1,200,000) does not bind on the $1,000,000 gift, so lines 11/14 carry the full amount.
  That is the charitable lens's call, not mine — but if it binds, my line 17 figure moves.
- **Whether the mixed-use prompt's wording is tight enough.** The prompt says "buy, build, or
  improve"; the form's line-8 text says the same, but i1040sca's *Limits* section says "buy, build,
  or **substantially** improve". A filer who spent proceeds on a non-substantial improvement answers
  "yes" truthfully against the prompt and wrongly against the instruction. I did not check whether
  the IRS treats the two phrasings as synonymous.
