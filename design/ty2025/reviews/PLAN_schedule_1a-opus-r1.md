# Schedule 1-A IMPLEMENTATION_PLAN — independent review r1 (Opus)

**Persisted verbatim before folding**, per `STANDARD_WORKFLOW.md` §2. Reviewer: independent Opus agent,
2026-07-29, against `design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md` r1 and the archived primary
sources (`f1040s1a--2025.pdf`, `i1040gi--2025.pdf` pp. 101-110).

Brief given: one question ("will following this plan produce a correct Schedule 1-A, or does it permit a
defect that would print a wrong number on a signed return?"), judged against the primary sources rather
than the spec's prose; fresh-audit scope creep forbidden; §G-9, OQ-2, OQ-3, TY2026/TY2029 fail-closed,
B1/B2 state, and the already-captured rounding asymmetry listed as settled.

**VERDICT: 2C / 4I / 2M / 1N**

---

### C-1. The line-5 net-income ceiling is placed only in the multi-business worksheet, leaving the **single**-business path — btctax's only reachable one — with the form's bare "net profit"

**Where:** Plan §T2, the ★ paragraph ("*The **Multiple Trades or Businesses** worksheet is where OQ-2's real content lives*"). Contradicted by `i1040gi--2025.pdf` p.105 (*Line 5* and *Net income limitation*), and by `f1040s1a--2025.pdf` line 5.

**Evidence:** The worksheet is conditional — "*If you and/or your spouse received qualified tips in the course of **more than one** trade or business, complete the Multiple Trades or Businesses Worksheet.*" The ceiling is not: "***Line 5.** Include the qualified tips … but only to the extent the trade or business in which you received the qualified tips has net income. See Net income limitation, later.*" And the instructions give the one-business case explicitly:

> "For example, a sole proprietor who **only has one business** and received qualified tips in the business … The net income limitation will be the net profit shown on the Schedule C for the business, **less the amount from Schedule 1, line 15**. The sole proprietor would include on line 5 of Schedule 1-A the lesser of (i) the qualified tips received in the business, or (ii) the net profit for the business less the amount from Schedule 1, line 15."

The form's own line 5 says only "*Do not enter more than the net profit from the trade or business*" — so a transcription that satisfies T2's mechanical doc-comment gate implements the **un**reduced ceiling.

**Why it matters:** `ReturnInputs::schedule_c` is `Option<ScheduleCInputs>` (singular, `crates/btctax-core/src/tax/return_inputs.rs:425`), so the multi-business branch is unreachable today and the single-business branch is the *only* path. Building it as `min(tips, net_profit)` overstates the ceiling by Schedule 1 line 15 (half the SE tax) — printed Schedule 1 line 15 already exists (`printed.rs:384`, `p.half_se_15`), so no new input is needed. A larger line 5 → larger lines 6/7/13/38 → **understates tax**.

**Fix:** In T2/T4, state the ceiling as a property of **line 5 itself** — `min(tips, max(0, net_profit − Schedule 1 L15 − SEP/SIMPLE − SE health insurance))`, applied per business including when there is exactly one — and make the worksheet the multi-row *aggregation* of that same ceiling, not its only home.

---

### C-2. Part IV has **no eligibility declarations at all**; the plan's six-row table covers only Parts II/III

**Where:** Plan §T3, "The declarations, and why each is a declaration rather than a derived value" (six rows: SSN, SSTB, tips ⊆ box 7, overtime premium half, overtime excludes tips, state-law-only overtime). Contradicted by `i1040gi--2025.pdf` pp.109-110, *Qualified passenger vehicle loan interest* / *Applicable passenger vehicle* / *Personal use*.

**Evidence:**

> "To qualify for the QPVLI deduction, the interest must be paid or accrued on a loan that generally meets **all** the following requirements. 1. Your loan was originated **after December 31, 2024**. 2. The loan was originated **by you**. 3. The proceeds from your loan were used to **purchase** an APV (**lease payments do not qualify**). 4. Your APV is for **personal use** … 5. Your loan is secured by a **first lien** on the purchased APV."

> "In general, an APV is any vehicle that meets the following conditions: • The **original use** of the vehicle starts with you (**a used vehicle does not qualify**) … • The vehicle is a car, minivan, van, SUV, pickup truck, or motorcycle, and has a **gross vehicle weight rating of less than 14,000 pounds**, and • The vehicle has undergone **final assembly in the United States**."

> "amounts representing debt on a vehicle traded in as part of the purchase transaction for the APV (so-called negative equity), **is not eligible** for the deduction."

Not one of these appears anywhere in the plan (the only Part IV mentions are the $10,000 cap, the threshold pair, the ceil, and the VIN deferred to B4). None is derivable by btctax.

**Why it matters:** T3 as written collects a QPVLI dollar amount and column (ii), so **every** filer who types a car-loan interest figure gets the deduction — including a lease, a used car, a non-US-assembled car, a refinance beyond the prior balance, a pre-2025 loan, or negative equity. Up to $10,000 of deduction that does not exist → **understates tax**. This is the same class as the plan's own S-5 principle ("eligibility bars are part of the transcription"), which T4 applies only to the filing-status bars.

**Fix:** Add the Part IV conditions to T3's declaration table as YES-conditions defaulting to NO, per vehicle: loan originated after 2024-12-31 by the filer to purchase (not lease) the vehicle, secured by a first lien; vehicle new (original use), US final assembly, GVWR < 14,000 lb, in the listed body classes; and the loan-amount composition (no negative equity / non-customary items). Same "enumerate the YES-conditions" treatment the plan already gives the three overtime traps.

---

### I-1. "All 38 numbered lines" is a miscount — the label set has **48** members, and the plan's mechanical conformance gate is defined on that count

**Where:** Plan §1 exit criterion 2 and §T2 ("*every one of the 38 numbered lines is a field, with the list closed at both ends via a `BTreeSet` comparison*"). Contradicted by `f1040s1a--2025.pdf` text layer.

**Evidence:** 38 is the highest line *number*, not the number of lines. The lettered lines are: `2a 2b 2c 2d 2e`, `4a 4b 4c` (there is **no** entry box for a bare line 4 — it is the heading for 4a-4c), `14a 14b 14c` (no bare 14), `22a 22b`, `36a 36b`. Counting labels: Part I 7, Part II 12, Part III 10, Part IV 10, Part V 8, Part VI 1 = **48**. And line 22 carries three columns per row — "*(i) Vehicle identification number (VIN)*", "*(ii) Deducted on Schedule C, Schedule E, or Schedule F*", "*(iii) Schedule 1-A*" — so the data leaves are **52**.

**Why it matters:** A `BTreeSet` closed at both ends built from `1..=38` either reds on every lettered field as an "unexpected extra", or the struct is collapsed to match and a sub-line is lost. That is the exact defect class CLAUDE.md records as shipped ("*Later drafts dropped Form 6251 line 2b*"). Dropping 2b or 2c under-adds MAGI → phase-outs too small → **understates tax**; dropping 22 column (ii) permits the same interest to be deducted twice.

**Fix:** Replace "38 numbered lines" with the 48-label set (naming the lettered runs) and state that line 22 is 2 rows × 3 columns, so the KAT's expected set is enumerated from the extract rather than from a range.

---

### I-2. T1's closed-form phase-out identity is false for Part IV, because Part IV ceils

**Where:** Plan §T1, "*The statutory identity worth asserting here: every part reaches $0 exactly at `threshold + cap/per_step × step`*". Contradicted by `f1040s1a--2025.pdf` line 28.

**Evidence:** "*28 Divide line 27 by $1,000. If the resulting number isn't a whole number, **increase** the result to the next **higher** whole number. (For example, increase 1.5 to 2, and increase 0.05 to 1.)*"

With the full $10,000 cap: line 30 = 0 as soon as `200 × ceil(E/1000) ≥ 10,000`, i.e. `ceil(E/1000) ≥ 50`, i.e. **E > $49,000**. At E = $49,000 exactly, line 28 = 49 → line 29 = $9,800 → line 30 = **$200**. At E = $49,001, line 30 = $0. So Part IV exhausts at `threshold + $49,001`, not `threshold + $50,000`. The identity holds exactly only for the two flooring parts (II: +$250,000; III: +$125,000 / +$250,000 MFJ) and for Part V (+$100,000, smooth).

**Why it matters:** T1's stated purpose is to prevent the floor/ceil confusion, and this is a floor-shaped identity asserted over the one ceiling part. If the implementation is bent to satisfy it, Part IV floors — which for any excess that is not a multiple of $1,000 leaves $200 of deduction standing that the form removes (e.g. E = $49,500 → $200 instead of $0) → **understates tax**.

**Fix:** State the identity per direction: floor parts exhaust at `threshold + (cap/per_step) × step`; the ceil part exhausts at `threshold + (cap/per_step − 1) × step + 1` — and keep the paired assertion "one step below the knee is still > $0", which is what actually distinguishes the two directions.

---

### I-3. Part II's *qualified-tip* gates are missing: the listed-occupation eligibility bar, the multi-occupation carve-out, and the cash/voluntary/customer-determined definition

**Where:** Plan §T3's declaration table (Part II gets only "SSTB tips" and "qualified tips ⊆ W-2 box 7") and §T4 (which transcribes only the *filing-status* bars under S-5). Contradicted by `f1040s1a--2025.pdf` Part II caution and `i1040gi--2025.pdf` pp.101-104.

**Evidence:** The form prints the eligibility bar itself:

> "**Caution:** Fill out Part II only if you received qualified tips. **These tips must have been received in an occupation listed at IRS.gov/TippedOccupations.**"

> "In order for a tip to be a qualified tip, it must have been paid to you while you were working in an occupation that customarily and regularly received tips on or before December 31, 2024."

> "If you received tips as an employee in more than one occupation for the same employer, only those tips that were received in an occupation on the list … are considered qualified tips. **Do not include tips received in occupations that are not included on this list in line 4a, 4b, or 4c.**"

And the amount definition: "*Qualified tips do not include service charges, automatic gratuities, or any other mandatory amounts automatically added to a customer's bill …*" (Example 1: the 18% automatic gratuity "*is not a qualified tip and may not be deducted*"), plus "*Cash tips don't include … Event tickets, Meals, Services*".

**Why it matters:** This is the *first* condition on the form, and it is not derivable — the plan's SSTB row is nearly subsumed by it (Notice 2025-69 answers SSTB *from* the occupation list), so the plan carries the derived question and drops the gating one. Without it, any filer with a box-7 figure deducts up to $25,000 → **understates tax**. The "⊆ box 7" prompt as described gives the filer no criteria for computing the subset, which is precisely where the auto-gratuity and non-cash-medium exclusions live.

**Fix:** Add two rows to T3: (a) the occupation gate — record the Treasury Tipped Occupation Code / "is this occupation on the IRS list", defaulting to NO (spec §3 already says "the filer answers; we record the code", but no task collects it); (b) the qualified-tip amount criteria (cash medium, paid voluntarily, not negotiated, customer-determined; service charges and automatic gratuities excluded) as the YES-conditions of the line-4a/line-5 prompts, with the multi-occupation restriction stated.

---

### I-4. T5's below-the-line invariant lists only the must-**not**-move consumers and omits the one that must **move** — the guarantee B2 explicitly parked on B3

**Where:** Plan §T5, spec §5.6b as carried ("*every AGI-keyed quantity must be **byte-identical** with and without a Schedule 1-A deduction: Form 8960's NIIT MAGI, Schedule A's 7.5% medical floor, the §164(b) SALT phase-down MAGI, and the IRA/student-loan phase-outs*"). Contradicted by `crates/btctax-core/src/tax/return_1040.rs:1432` and the test comment at `:2460-2466`.

**Evidence:** B2 left a written obligation on this branch:

> "★ **This test is DELIBERATELY WEAK, and saying so is the point.** `assemble_absolute` hardcodes a zero 13b until Schedule 1-A lands (**B3**), so both assertions below are trivial identities and every mutation of the composition survives them — verified: dropping the 13b term from `total_deductions`, and subtracting 13a from `ti_before_qbi`, both leave this green. **It records the SHAPE so B3 has something to make real.**"

The quantity is `ti_before_qbi: agi - deduction - schedule_1a_additional` (`:1432`) — Form 8995 line 11, which gates both the 20%-of-taxable-income limit and `qbi_over_threshold`'s refusal point.

**Why it matters:** `ti_before_qbi` is *derived from AGI* and therefore looks AGI-keyed; a mechanical reading of "every AGI-keyed quantity must be byte-identical with and without Schedule 1-A" would **require the bug** (dropping the 13b term). B2's own comment names the direction: omitting 13b overstates `ti_before_qbi`, inflating the §199A deduction → **understates tax**. Today the code is right but the guarantee is held by nothing — a vacuous guard the plan's exit criterion 5 does not name.

**Fix:** In T5, split the invariant: `1040 L15` and `Form 8995 line 11` (`ti_before_qbi`) **must move** by exactly L38; the four AGI-keyed consumers must be byte-identical. Name `form_1040_line14_sums_12e_13a_and_13b_while_form_8995_line11_excludes_only_12e_and_13b` as the test B3 makes non-vacuous with a nonzero 13b, and mutation-verify it.

---

### M-1. Line 34's 6% multiply is a **third** rounding site with no direction stated on the form

**Where:** Plan §T1, which parameterizes rounding for the two step functions (lines 11/19 floor, line 28 ceil) and treats Part V as smooth. `f1040s1a--2025.pdf` lines 34-35.

**Evidence:** "*34 Multiply line 33 by 6% (0.06)*" … "*35 Subtract line 34 from $6,000. If zero or less, enter -0-*". Line 34 is its own printed dollar line with no rounding instruction, so the general whole-dollar convention governs (`round_dollar`, `MidpointAwayFromZero`, `crates/btctax-core/src/conventions.rs:28`).

**Why it matters:** `6,000 − round(0.06 × L33)` and `round(6,000 − 0.06 × L33)` differ by $1 whenever `0.06 × L33` lands exactly on a half-dollar (excess ≡ 25 mod 50, e.g. $50,025 → $3,001.50), and the difference is doubled on a two-senior MFJ return. The "round the difference" form gives the larger deduction → **understates tax** by $1-$2 of deduction. The project's own rule ("every line takes that schedule's PRINTED figure") settles it: round line 34, then subtract.

**Fix:** One sentence in T1/T4: line 34 is rounded as its own printed line under the IRS whole-dollar convention, and line 35 subtracts the *printed* line 34.

---

### M-2. Line 22's ">two VINs ⇒ attach a statement" arity is a T2 struct-shape decision, not a B4 emitter concern, and the plan never states it

**Where:** Plan §3 defers "the PDF and AcroForm map, including the VIN's per-character comb boxes" to B4; nothing in T2/T4 addresses line 22's row arity. `f1040s1a--2025.pdf` line 22-23 and `i1040gi--2025.pdf` p.110 *Line 22*.

**Evidence:** "*22 Applicable passenger vehicle (see instructions). **If more than two VINs, see instructions.***" — rows a and b only. "*23 Add lines 22a and 22b, column (iii)*". Instructions: "*If you need to report more than two VINs, **attach a statement to your return** showing the information required on line 22.*" Spec S-6 lists this as a known branch; the plan does not carry it.

**Why it matters:** T2 is a declared chokepoint that fixes the shapes everything downstream depends on, so a two-row-vs-N-row decision cannot be discovered in B4. Capping at two silently drops a third vehicle's interest (overstates tax); summing three into two rows produces a correct line 23 while omitting a VIN the deduction is conditioned on.

**Fix:** State in T2 whether line 22 is a fixed 2-row structure with a refusal above two vehicles, or an N-row structure whose rows 3+ route to a statement — and pin the choice with a KAT.

---

### N-1. The valid-SSN declaration is unscoped; Part IV carries no SSN requirement

**Where:** Plan §T3 declaration row 1, "valid SSN, per person". `f1040s1a--2025.pdf` Part IV caution and `i1040gi--2025.pdf` Part IV.

**Evidence:** Parts II, III and V each have a *Valid SSN* paragraph in the instructions and print the requirement in their caution. Part IV's caution reads only "*Fill out Part IV only if you, or your spouse if married filing jointly, paid or accrued qualified passenger vehicle loan interest (QPVLI)*" and the Part IV instructions contain no SSN paragraph at all.

**Why it matters:** Gating all four parts on the SSN declaration denies a QPVLI deduction §163(h)(4) allows — overstates tax, so not blocking.

**Fix:** Scope the row to Parts II/III/V.

---

**Judgment calls checked and found sound** (no finding): the MFS treatment of Part IV is **correct** — Parts II/III/V each print "*If married, you must file jointly to claim this deduction*" while Part IV prints no such caution and its instructions read "*• Married filing jointly—$200,000. • **All other filing statuses**—$100,000*", which affirmatively contemplates MFS; OTS is a witness, not the authority. The three threshold pairs, the three caps, the floor/ceil assignment to lines 11/19/28, the four skip branches (10→13, 18→21, 27→30, and 33's `$6,000` write into line 35), `L38 → 1040 L13b`, `L37 → Form 6251 line 1a` (the form reads "*Subtract Schedule 1-A (Form 1040), line 37, from Form 1040 … line 14*", already built at `form6251.rs:332`), the fail-closed gate's condition-4 mapping, and worked examples (b) $2,300 / (c) $5,000 / (d) $6,000 all verify against the text layer.

VERDICT: 2C/4I/2M/1N

---

## Author's verification notes (added when folding, NOT part of the reviewer's output)

All four blocking findings independently re-verified against the text layer before folding:

- **C-1 CONFIRMED** verbatim, and with a second sentence the reviewer did not quote: *"If the business
  shows a net loss on Schedule C, then the sole proprietor would **not include any** qualified tips
  received in the business on line 5."* ★ **A tension worth recording:** the two numbered *Examples*
  that follow compute a "net income limitation" of $4,500 (gross $5,000 − expenses $500) and $1,000
  (gross $15,000 − expenses $14,000) **without** subtracting Schedule 1 line 15. The narrative states
  the rule, the examples illustrate an outcome; the narrative governs, and it also happens to be the
  fail-closed direction (a lower ceiling ⇒ smaller deduction ⇒ higher tax). Recorded so a future
  reviewer or oracle pointing at the examples is met with the adjudication rather than a rederivation.
- **C-2 CONFIRMED** — the five loan conditions are at instr. lines 1194-1215, the APV conditions
  (original use / GVWR < 14,000 lb / US final assembly) at 1245-1263, negative equity at 1208.
- **I-1 CONFIRMED** by direct count of the label set: 7 + 12 + 10 + 10 + 8 + 1 = 48, with no bare
  amount box on line 4 or line 14, and 52 data leaves once line 22's three columns are counted.
- **I-2 CONFIRMED** by arithmetic: at excess $49,000 line 28 = 49, line 29 = $9,800, line 30 = **$200**;
  at $49,001 line 28 = 50 and line 30 = $0. My identity would have put the knee at +$50,000.
- **I-4 CONFIRMED as to the fix, PARTLY OVERSTATED as to the premise.** "Today the code is right but
  the guarantee is held by nothing" is not accurate: the same doc comment continues *"The load-bearing
  version is `printed::tests::printed_1040_line14_needs_all_three_terms`, which drives the printed path
  with a nonzero 13b and **does** die to that mutation."* So the composition IS held — by the printed-path
  test, deliberately. The actionable part of the finding stands and is folded in full: the invariant must
  be split into must-move and must-not-move halves, because the plan as written invited a reading that
  would have required the bug.
