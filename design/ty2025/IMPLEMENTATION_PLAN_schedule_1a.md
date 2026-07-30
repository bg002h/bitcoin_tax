# Schedule 1-A (TY2025) — IMPLEMENTATION PLAN

**Status: r2** (r1 folded 2026-07-29 — **2 Critical, 4 Important, 2 Minor, 1 Nit**, all confirmed against the text layer before folding; reviewer output persisted verbatim at `design/ty2025/reviews/PLAN_schedule_1a-opus-r1.md`, with my re-verification notes appended there rather than mixed into it). The two Criticals were both **missing eligibility**, not wrong arithmetic — the defect class this project keeps rediscovering. Implements `design/ty2025/SPEC_schedule_1a.md` **r3** (0 Critical / 0 Important),
which is branch **B3** of `design/ty2025/SPEC.md` §8a. Parent decisions D-1 … D-11 bind.

**Branch:** `feat/amt-e2-vector-population` (current; B1 + B2 already on it).

---

## 0. What this plan is, and what it deliberately is not

**It is a sequencing document.** The transcription itself lives in the code, per this project's standing
rule: *one field per numbered line, in the form's own numbering, carrying the official instruction text
verbatim as its doc comment.* This plan does **not** restate the 48 lines — restating them here would
create a second, unexecutable copy that can drift from both the form and the struct. The extracted text
layer is the source: `f1040s1a--2025.pdf` (`64f97b38`) and `i1040gi--2025.pdf` (`482e9c48`) pp. 101-110,
both archived in `design/amt-form6251/`.

**Consequently the review gate on this work is mechanical** (CLAUDE.md): *is every line present, and
does each doc comment match the instruction text?* That is a test, and T2 writes it.

**★ This plan does NOT delete `ty2025_full_return_must_stay_fail_closed_until_complete`.** B3 satisfies
the gate's **condition 4** only. Conditions 2 and 3 landed in B2; condition 1 (the `FullReturnParams`
themselves) is the LAST thing to land, after B4. Until then Schedule 1-A is fully built and fully tested
against synthetic params and reaches no filed return.

---

## 1. Exit criteria (the definition of green for B3)

1. The **five gates**: `make check` · `cargo fmt --all --check` · `cargo +1.88 check --workspace
   --locked` · `cargo run -p xtask -- check-isolation` · `bash scripts/pii-scan-generic.sh`.
2. **All 48 line LABELS present** — not 38, which is merely the highest line *number* (r1 I-1).
   Asserted by a closed-at-both-ends KAT (T2), plus all four worksheets.
3. **Every part's phase-out tested at its own knee in its own direction** (S-1), including the recon's
   worked examples (b) $2,300 and (c) $5,000 — figures that differ under the wrong rounding.
4. **TY2024 provably unmoved**: golden matrix md5 `c4e1853ed82d113ca5cd97ffd8abbf47`, both oracles
   exit 0.
5. **Mutation-verified**, per guard. A guard whose mutation survives is not a guard.
6. **TY2029+ fails closed**, mutation-verified.

---

## 2. Task sequence

Each task is test-first and lands green. Tasks T1-T2 are the chokepoints — T3 onward all depend on the
shapes they fix, so they are done first and reviewed before the surface work fans out.

### T1 — the per-year table, with rounding as a parameter

`crates/btctax-core/src/tax/tables.rs`.

A `Schedule1aParams` carried per year for **2025-2028 only** (S-7: nothing is indexed, all four
provisions expire after TY2028, so a Rev. Proc. lookup is not merely unnecessary but wrong).

**The three things that must not be shared:**

- **Rounding direction is an explicit argument, never baked in** (S-1). Parts II/III **floor** (lines 11,
  19: *"decrease the result to the next lower whole number"*); Part IV **ceils** (line 28: *"increase the
  result to the next higher whole number"*). A `phase_out(excess, per_step, step)` helper with one
  direction is silently wrong on one side, by exactly $100 and $200 — which is precisely what worked
  examples (b) and (c) measure.
- **Three distinct threshold pairs** (spec F-4): $150,000/$300,000 (lines 9, 17), $100,000/$200,000
  (line 26), $75,000/$150,000 (line 32). No `threshold_for(status)`.
- **Three distinct caps**: $25,000 tips (line 7, per-return regardless of status — S-3), $12,500/$25,000
  MFJ overtime (line 15), $10,000 QPVLI (line 24).

★ **A THIRD rounding site, easily missed because the form states no direction for it** (r1 M-1). Line 34
(*"Multiply line 33 by 6% (0.06)"*) is its own printed dollar line, so the general IRS whole-dollar
convention governs it (`round_dollar`, `MidpointAwayFromZero`, `conventions.rs:28`) and line 35 subtracts
the **printed** line 34. `6,000 − round(0.06 × L33)` and `round(6,000 − 0.06 × L33)` differ by $1 whenever
`0.06 × L33` lands on a half-dollar (excess ≡ 25 mod 50, e.g. $50,025 → $3,001.50) — **doubled** on a
two-senior MFJ return, and the "round the difference" form is the one that understates tax. Round the
line, then subtract; that is the project's standing rule that every line takes that schedule's *printed*
figure.

**Tests.** Each constant against the extracted line text. `TY2029+` returns `None` and a mutation that
extends the table reds.

★★ **The exhaustion identity is PER DIRECTION — a single closed form is false for Part IV** (r1 I-2, and
the plan's own r1 asserted the false one). Because Part IV **ceils**, it exhausts one dollar past the
last full step, not at the round number:

| part | direction | exhausts at |
|---|---|---|
| II tips | floor | `threshold + (cap/per_step) × step` = `+$250,000` |
| III overtime | floor | `+$125,000` (`+$250,000` MFJ) |
| **IV car loan** | **ceil** | `threshold + (cap/per_step − 1) × step + 1` = **`+$49,001`**, NOT `+$50,000` |
| V seniors | smooth | `+$100,000` (S-4) |

Check the arithmetic once, here, so nobody re-derives it: at excess $49,000 line 28 = 49 → line 29 =
$9,800 → line 30 = **$200**; at $49,001 line 28 = 50 → line 30 = $0. ★ **The assertion that actually
distinguishes the two directions is the paired one** — "at the knee, $0" *and* "one step below the knee,
still > $0". A knee-only test passes under both roundings, which is exactly the failure S-1 warns about.

### T2 — the struct: 48 line labels (52 leaves) + four worksheets, and the conformance KAT

`crates/btctax-core/src/tax/schedule_1a.rs` (new).

One field per line LABEL, named for it (`line4a`, `line36b`, …), instruction text verbatim as the
doc comment. Sub-structs per part keep the names short without renumbering.

**The four worksheets are their own transcribed types**, not `min()` calls in the emitter (spec F-2, and
OQ-2's closure): *Qualified Tips From More Than One Employer*, *Multiple Trades or Businesses*,
*Qualified Overtime Compensation From More Than One **Employer***, and *… From More Than One **Payor***.
The last two are distinct forms of the same idea (W-2 side vs 1099 side) and the r1 branch list
collapsed them.

★★ **THE LINE-5 CEILING IS A PROPERTY OF LINE 5, NOT OF THE WORKSHEET** (r1 **C-1** — the highest-value
finding in the round, and my r1 got it wrong in the direction that understates tax).

The ceiling is *not* net profit. It is net profit (Schedule C line 31 / the total of Schedule E lines
28(g) through 28(k) / Schedule F line 34) **minus** the deductible part of self-employment tax, the
deduction for contributions to self-employed SEP/SIMPLE/qualified plans, and the self-employed health
insurance deduction, **floored at zero**, and expressly **not** reduced by the qualified-tips deduction
itself (which is what keeps it acyclic).

★ **And it applies when there is exactly ONE business, which is btctax's only reachable case.**
`ReturnInputs::schedule_c` is `Option<ScheduleCInputs>` — singular — so the multi-business branch cannot
be reached today and the single-business branch is the whole surface. My r1 put the reduction *only* in
the *Multiple Trades or Businesses* worksheet, which the instructions make conditional (*"If … more than
one trade or business, complete the …Worksheet"*) while the ceiling itself is not. The instructions give
the one-business case outright:

> "For example, a sole proprietor who **only has one business** … The net income limitation will be the
> net profit shown on the Schedule C for the business, **less the amount from Schedule 1, line 15**. The
> sole proprietor would include on line 5 of Schedule 1-A the lesser of (i) the qualified tips received
> in the business, or (ii) the net profit for the business less the amount from Schedule 1, line 15. **If
> the business shows a net loss on Schedule C, then the sole proprietor would not include any qualified
> tips** received in the business on line 5.

`min(tips, net_profit)` would overstate the ceiling by half the SE tax → larger lines 6/7/13/38 →
**understates tax**. No new input is needed: printed Schedule 1 line 15 already exists
(`printed.rs:384`, `p.half_se_15`). So **the ceiling lives on line 5, and the worksheet is the multi-row
aggregation of that same ceiling** — its only home was the defect.

★ **A tension to adjudicate in the code comment, not rediscover.** The two numbered *Examples* that
follow compute a limitation of $4,500 (gross $5,000 − expenses $500) and $1,000 (gross $15,000 −
expenses $14,000) **without** subtracting Schedule 1 line 15. The narrative states the rule; the examples
illustrate an outcome. **The narrative governs** — and it is also the fail-closed direction (lower
ceiling ⇒ smaller deduction ⇒ higher tax). Cite both in the doc comment so a future reviewer or oracle
pointing at the examples meets the adjudication instead of re-deriving it.

★ **LINE 22's ARITY IS A T2 DECISION, NOT A B4 ONE** (r1 M-2). The form has rows **a and b only** —
*"If more than two VINs, see instructions"*, and the instructions say *"attach a statement to your return
showing the information required on line 22."* Line 23 adds *"lines 22a and 22b, column (iii)"*. T2 is a
declared chokepoint, so this cannot be discovered in B4: decide now between a fixed 2-row structure that
**refuses** above two vehicles and an N-row structure whose rows 3+ route to a statement, and pin the
choice with a KAT. Capping silently at two drops a third vehicle's interest (overstates tax); summing
three into two rows yields a correct line 23 while omitting a VIN the deduction is conditioned on.

**The conformance KAT** — the mechanical gate, executed:

- ★ **all 48 line LABELS are fields** (r1 I-1). 38 is the highest line *number*, not the count. The
  lettered runs are `2a-2e`, `4a-4c`, `14a-14c`, `22a-22b`, `36a-36b`, and **neither line 4 nor line 14
  has a bare amount box** — each is a heading for its lettered sub-lines. Per part: I 7, II 12, III 10,
  IV 10, V 8, VI 1 = **48**; and **52 data leaves** once line 22's three columns ((i) VIN, (ii) deducted
  on Schedule C/E/F, (iii) Schedule 1-A) are counted. ★ **The expected set is ENUMERATED FROM THE
  EXTRACT, never from a range** — a `BTreeSet` built from `1..=38` either reds on every lettered field as
  an unexpected extra, or the struct gets collapsed to match and a sub-line is lost. That is the exact
  defect CLAUDE.md records as shipped: *"Later drafts dropped Form 6251 line 2b."* Dropping 2b or 2c
  under-adds MAGI ⇒ phase-outs too small ⇒ understates tax; dropping 22 column (ii) lets the same
  interest be deducted twice.
- the list is **closed at both ends** via a `BTreeSet` comparison so neither a missing line nor an
  unexpected extra passes (the pattern the Form 6251 KAT settled on);
- each field's doc comment contains the line's own instruction text, checked against a committed
  extract of the text layer, so a paraphrase reds.

### T3 — the input surface, landed whole

~25 leaves plus **six declarations**, through the whole stack in one pass (the G-9 walk, since the user
has directed that the input surface not lag the core): `return_inputs.rs` → `classifier.rs` →
`questions.rs` → `return_refuse.rs` → `input-form` `seam/registries/coverage/sections` → CLI `answer.rs`
→ TUI. The exhaustive matches and the coverage KAT force every site; nothing here is found by grep.

**The declarations, and why each is a declaration rather than a derived value.** ★★ r1 found this table
**incomplete in the dangerous direction, twice** (C-2 and I-3): it carried the *derived* questions and
dropped the *gating* ones, so a filer who merely typed a dollar figure got the deduction. Both Criticals
in the round were missing eligibility, not wrong arithmetic — the defect class CLAUDE.md names.

**Part II — tips.** ★ The FIRST condition is printed on the form itself and r1 had no row for it:

> **Caution:** Fill out Part II only if you received qualified tips. **These tips must have been received
> in an occupation listed at IRS.gov/TippedOccupations.**

| declaration | why btctax cannot answer it |
|---|---|
| ★ **occupation is on the Treasury list** (record the code) | The gating condition. *"In order for a tip to be a qualified tip, it must have been paid to you while you were working in an occupation that customarily and regularly received tips on or before December 31, 2024."* Spec §3 already says "the filer answers; we record the code" — r1 collected it nowhere. Defaults to **NO**. |
| ★ **multi-occupation carve-out** | *"If you received tips as an employee in more than one occupation for the same employer, only those tips that were received in an occupation on the list … are considered qualified tips. **Do not include tips received in occupations that are not included on this list in line 4a, 4b, or 4c.**"* |
| ★ **the qualified-tip amount criteria** | Cash medium, paid voluntarily, not negotiated, customer-determined — and *"Qualified tips do not include service charges, automatic gratuities, or any other mandatory amounts automatically added to a customer's bill"* (the instructions' own example: an 18% automatic gratuity *"is not a qualified tip and may not be deducted"*), nor *"Event tickets, Meals, Services"*. These ARE the criteria the "⊆ box 7" prompt needs; without them the prompt asks for a subset and offers no way to compute it. |
| SSTB tips | §224(d)(3), **as relaxed by Notice 2025-69** (OQ-3). ★ Nearly *subsumed* by the occupation gate — the relief answers SSTB **from** the occupation list — which is why carrying this row while dropping the gate was backwards. |
| qualified tips ⊆ W-2 box 7 | Spec **F-1**: the 2025 W-2/1099s *"were not updated to separately identify tips that may qualify"*. Box 7 is a starting point, not the figure. |

**Part III — overtime.** The three §4.1 traps, unchanged from r1: the FLSA **premium half** only (not
double-time's second half, not holiday/weekend premiums absent >40 hours); **excludes any amount received
as a qualified tip** (no double-dip with Part II — the surface must refuse the same dollars twice, not
silently allow them); and **state-law-only overtime for FLSA-ineligible employees does not qualify** (the
entitlement must arise under FLSA §7).

**Part IV — car loan interest.** ★★ r1 had **no eligibility declarations at all** for this part (C-2):
only the cap, the threshold, the ceil, and a VIN deferred to B4. So every filer who typed a car-loan
interest figure got up to **$10,000** of deduction — including on a lease, a used car, a
non-US-assembled car, a refinance, a pre-2025 loan, or negative equity. **Understates tax.** The
instructions state the conditions flatly, and none is derivable:

> "the interest must be paid or accrued on a loan that generally meets **all** the following
> requirements. 1. Your loan was originated **after December 31, 2024**. 2. The loan was originated **by
> you**. 3. The proceeds from your loan were used to **purchase** an APV (**lease payments do not
> qualify**). 4. Your APV is for **personal use** … 5. Your loan is secured by a **first lien** on the
> purchased APV."

> "In general, an APV is any vehicle that meets the following conditions: • The **original use** of the
> vehicle starts with you (**a used vehicle does not qualify**) … • The vehicle is a car, minivan, van,
> SUV, pickup truck, or motorcycle, and has a **gross vehicle weight rating of less than 14,000
> pounds**, and • The vehicle has undergone **final assembly in the United States**."

> "amounts representing debt on a vehicle traded in as part of the purchase transaction for the APV
> (so-called negative equity), **is not eligible** for the deduction."

Collected **per vehicle**, as YES-conditions defaulting to NO — the same treatment the overtime traps
get. This is S-5's own principle ("eligibility bars are part of the transcription") applied past the
filing-status bars, which is as far as r1 took it.

**Parts II/III/V — the valid-SSN bar, per person.** Spec F-3: *"valid for employment and … issued by the
Social Security Administration (SSA) before the due date"* — neither property is visible to btctax.
★ **Scoped to Parts II, III and V only** (r1 N-1): Part IV's caution carries no SSN requirement and its
instructions have no *Valid SSN* paragraph, so gating Part IV on it would deny a deduction §163(h)(4)
allows.

★ **Prompt wording is the deliverable here, not plumbing** (R-2). A wrong prompt is a wrong return that
every test passes. Each prompt states the condition that permits a *yes* and defaults to the answer that
cannot overstate the deduction — the structural lesson from
`widening-an-exemption-is-never-the-safe-edit`: enumerate the YES-conditions so every omission fails
closed.

Death of a taxpayer/spouse needs **no new collection**: §G-9 landed
`HouseholdHeader::{taxpayer,spouse}_died_during_year` and `Person::date_of_death`, with the
day-before-the-65th-birthday convention in `reaches_65_on`. Part V reuses them at $6,000 per person.

### T4 — compute, transcribed line by line

**The skip branches are the risk, not the arithmetic** (spec F-5):

- Lines **10, 18, 27**: *"If zero or less, enter the amount from line 7 [15, 24] on line 13 [21, 30]"* —
  a jump **past** the phase-out, not a zero.
- Line **33**: *"If zero or less, **enter $6,000 on line 35**"* — a jump that writes a **nonzero
  constant** into a later line. Transcribing this as `-0-` yields **$0 instead of $6,000**: the whole
  senior deduction lost for every filer under the threshold, which is most of them. It happens to agree
  with `max(0, …)` only because 6% × 0 = 0, so a `max(0, …)` transcription passes for the wrong reason
  and breaks if the rate ever moves. Pin the branch itself.

Filing-status bars are transcribed, not inferred (S-5): Parts II, III and V print *"If married, you must
file jointly to claim this deduction"* ⇒ **zero for MFS**; Part IV prints no such caution ⇒ **allowed for
MFS**, which is adjudicated against the form over OTS 2025 (which bars all four — a witness, not the
authority).

### T5 — wiring, and the below-the-line invariant

`L38 → 1040 L13b` (the `AbsoluteReturn::schedule_1a_additional` seam B2 already threaded);
`L37 → Form 6251 line 1a` — the **senior subtotal**, not the total (parent D-3). File Schedule 1-A only
when `L38 > 0`.

★★ **Spec §5.6b is the cheapest high-value guard in the whole branch, and it has TWO halves — r1 stated
only one** (r1 I-4). As written it listed the quantities that must **not** move and omitted the ones that
**must**, which invited a reading that would have *required* a bug: `ti_before_qbi` is derived from AGI
and so looks AGI-keyed, and a mechanical "every AGI-keyed quantity is byte-identical" would mandate
dropping the 13b term.

| | quantity | with vs without a Schedule 1-A deduction |
|---|---|---|
| **must MOVE**, by exactly L38 | `1040 L15`; **Form 8995 line 11** (`ti_before_qbi = agi − deduction − schedule_1a_additional`, `return_1040.rs:1432`) | changes |
| **must NOT move** | Form 8960's NIIT MAGI · Schedule A's 7.5% medical floor · the §164(b) SALT phase-down MAGI · the IRA/student-loan phase-outs | byte-identical |

Direction, from B2's own note: omitting 13b **overstates** `ti_before_qbi`, inflating the §199A deduction
→ **understates tax**, and it fires `qbi_over_threshold` too early.

★ **This is a debt B2 explicitly booked to B3.** `form_1040_line14_sums_12e_13a_and_13b_while_form_8995_
line11_excludes_only_12e_and_13b` carries a doc comment saying it is *"DELIBERATELY WEAK, and saying so is
the point … `assemble_absolute` hardcodes a zero 13b until Schedule 1-A lands (B3), so both assertions
below are trivial identities … It records the SHAPE so B3 has something to make real."* T5 makes it real
with a nonzero 13b and mutation-verifies it. (For accuracy: the composition is *not* currently unguarded —
the same comment names `printed::tests::printed_1040_line14_needs_all_three_terms` as the load-bearing
version that does die to the mutation. What T5 discharges is the *semantic* test's vacuity, and the
must-move/must-not-move split.)

### T6 — tests

Beyond each task's own: the recon's four worked examples ((b) $2,300, (c) $5,000, (d) two-senior MFJ at
MAGI $200,000 ⇒ L37 $6,000 / $3,000 each, proving S-4's 12¢-per-$1 aggregate slope); all five filing
statuses (S-3's per-return caps and S-5's MFS bar are status-dependent); `L38 > 0` gates filing; `L37`
(not `L38`) reaches Form 6251 line 1a.

**Mutation-verify every guard.** Two of the previous session's guards were vacuous until a mutation said
so, both because every term in them was zero — so for each guard, ask first *which real input axis it
cannot express* (the parametrization lesson from the AMT sweep, where pinning the base standard deduction
hid the §63(f) add-back direction entirely).

### T7 — the two-oracle census, per part

Disqualifications **computed and sized**, never a name list (the shape `verify_f6251.py` converged on).
Known going in: **OTS 2025's Part IV is defective three ways** and **taxcalc has the wrong QSS
threshold** (parent D-8), so **QSS Part IV inside the phase-out band ships zero-oracle** — and the census
must *say so per vector* rather than printing two independent "OK" lines. That failure mode is on the
record: for MFS AMT, two separately-disqualified oracles agreed and left three vectors with no witness at
all, and only a per-vector witness count found it.

Where the oracles cannot adjudicate, the **form** does, and the citation goes in the code (the standing
policy: we are literally told what to do on the form or in its instructions).

---

## 3. Out of scope for B3

- **The PDF and AcroForm map**, including the VIN's per-character comb boxes — parent **B4** owns form
  assets (`recon/fable/05-ty2025-field-maps.md` has the extracted field names). R-4 stands: the generic
  PII scanner does not cover VINs, so B4's emitter tests assert no VIN-shaped literal in fixtures.
- **`FullReturnParams` for TY2025** — lands after B4, and only then does the fail-closed gate come out.
- **Schedule 8812**, which shares the MAGI and the ceil but is its own work (§3).
- **Optimizer changes.** The phase-out bands add hidden marginal-rate adders, so per-$1 what-ifs show $0
  then a cliff. **Document it; do not "fix" it** — the step function is the law (§3).

---

## 4. Risks carried from the spec

R-1 the rounding asymmetry (T1 answers it); R-2 input definitions with no source document (T3, and the
reason prompt wording is the deliverable); R-3 Part IV's weak oracle coverage (T7 states it rather than
papering over it); R-4 the VIN as a new class of filed data (B4); R-5 expiry after TY2028 (T1).
