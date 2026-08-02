# Schedule 1-A (TY2025) — IMPLEMENTATION PLAN

**Status: r3** (r2→r3: the 13-agent provenance census folded — see `reviews/PROVENANCE_CENSUS_schedule_1a.md`. It found a MISSING INPUT SURFACE and a hole in T2's own conformance approach; C-1 grew. Earlier: (r1 folded 2026-07-29 — **2 Critical, 4 Important, 2 Minor, 1 Nit**, all confirmed against the text layer before folding; reviewer output persisted verbatim at `design/ty2025/reviews/PLAN_schedule_1a-opus-r1.md`, with my re-verification notes appended there rather than mixed into it). The two Criticals were both **missing eligibility**, not wrong arithmetic — the defect class this project keeps rediscovering. Implements `design/ty2025/SPEC_schedule_1a.md` **r3** (0 Critical / 0 Important),
which is branch **B3** of `design/ty2025/SPEC.md` §8a. Parent decisions D-1 … D-11 bind.

**★★★ r4 FOLD (2026-08-01) — 0 Critical / 9 Important, two independent lenses.** This plan read
**r3** while `reviews/` held exactly ONE independent review of it; the r2→r3 fold was a 13-agent
provenance CENSUS, which is not a review, and it *grew* a Critical. Two folds went unreviewed against a
rule that says re-review after every fold. Reviews persisted verbatim at
`reviews/PLAN_schedule_1a-conformance-r4.md` and `reviews/PLAN_schedule_1a-buildability-r4.md`.

| # | finding | resolution |
|---|---|---|
| C-I1 | T2's KAT cannot produce 48 from the extract — the text layer yields **50** labels; lines **4 and 22** are headings with instruction text and no box | drive the expected set from `xtask/src/label_reader.rs` (landed 2026-07-30, one day AFTER this plan's last commit), which already adjudicates 50 / 48 / 2 with a required `note` per non-Amount row |
| C-I2 | **no COMPLETION conditions** — the only gate is `L38 > 0`, so a car-loan-only filer computes 15 phase-out lines the form says to skip and **line 35 prints $6,000 for a non-senior**; a filer with no §911 exclusion prints `$0` on 2a–2e | transcribe each part's Caution as a completion predicate; make the affected leaves able to express *not completed* **at T2**, because B4 cannot fix it if T2 makes every line non-optional |
| C-I3 | ★ the **refinance balance cap** was LOST in the r1 fold, and the prose states the rule BACKWARDS | see T3 below — added as a per-vehicle condition; **understates tax** without it |
| C-I4 | Part II's *"not qualified tips"* list has **three** bullets; the table carries one | add the illegal-service exclusion, worded on the IRS's own matched examples |
| B-I1 | the KAT has CLAUDE.md's half (a) and **not half (b)** — no per-line provenance, so "present but never populated" passes | every line carries a `Production` (or an `Exception` with a reason); build the actual set as `(label, got.lineN)` pairs so the compiler ties names to the struct |
| B-I2 | `line_coverage` was structurally blind to a new `schedule_1a.rs` | ✅ **FIXED IN CODE** — scope is now derived from the emitter, `Option<Usd>` counts as money, and the year is per-row |
| B-I3 | T2's doc-comment gate is `cite-check` semantics: it proves a quotation is *the form's* words, not *that line's* | use `tables.rs::printed_line(label)` — which **T1, in this same plan, already built for exactly this**, whose own doc says *"The fix is not more citation checking"* |
| B-I4 | T3a refuses on `schedule_c.is_some()` | ✅ corrected in T3a below — gate on the CLAIM |
| B-I5 | line **4b** (Form 4137) is a third line of the T3a shape with no recorded disposition | ✅ added to the T3a table |

★★ **AND THE CENTRAL SEQUENCING CLAIM WAS CORRECTED.** The author had concluded §G-11's `line_coverage`
was the prerequisite for T3's ~25 `Option<Usd>` inputs. It is not — `line_coverage` is a **printed-line**
instrument and does not answer the input-side question. The input-side answer **already exists**:
`return_inputs.rs:626-652`, where answered-ness rides an `Option<bool>` **class-(A) gate** (which the
classifier *forbids* `_` on) with the amounts hanging off it as plain `Usd`. Its own doc says an
`Option<Usd>` is a scalar the `_` rule permits — which would make this convention again.
**⇒ T3 uses ONE CLAIM GATE PER PART, not ~25 loose `Option<Usd>`s.** Part I needs no new input at all —
its four add-backs are already collected.

★ **Sizing correction (r4 M-1):** "~25 leaves plus six declarations" is stale by ~3×. Recount: Part II 5,
Part III 3, **Part IV 9 PER VEHICLE**, plus the SSN bar per person for II/III/V ⇒ **≈19 declarations**,
against a line-22 structure (2 rows × 3 columns) `ReturnInputs` has no shape for. T3's touchpoint list
also omits `btctax-input-form/src/attribute.rs` and the pinned counters (`questions.rs:985-986`,
`coverage.rs`'s 80-row `EXPECTED_LEAF_PATHS`). All compiler- or test-forced, so nothing escapes — but
"landed whole in one pass" is a materially larger pass than stated.

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

★★ **THE EXPECTED SET COMES FROM BOTH EXTRACTS, NOT THE FORM ALONE** (census **F-4** — a hole in this
task's own approach, found by measurement). `grep -c "Keep for Your Records"` on the **form** extract is
**0**: the four worksheets exist only in the *instructions* extract. So a label census driven off the form
fixture — which is what r2 specified — **could never red on a worksheet omission.** It would have passed by
finding nothing, which is the exact false-completeness this plan warns about everywhere else.

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

★★ **r4 CORRECTIONS TO THIS KAT — three, and the first two are what make it a conformance check.**

1. **Half (b) is missing.** The bullets above are (a), (a) and quotation. CLAUDE.md is explicit that a checker
   which cannot distinguish *this line encodes no decision* from *we forgot this line* is not a
   conformance check. As written, a field declared, doc-commented and **never assigned by T4** passes. ⇒ every
   line additionally carries a `Production` from `tax/line_coverage.rs` — `Collected`, `Carry`,
   `Combine`, `Clamped(Polarity)`, `Scaled`, `Bounded`, `Constant`, or an `Exception` **with a written
   reason**, ratcheted. Schedule 1-A maps onto it almost exactly; lines 10/18/27/33 are `Exception`s
   (conditional *jumps*, not clamps — the class the ratchet already records for `f1040:34`).
2. **The doc-comment check must be PER LINE, not per document.** *"Checked against a committed
   extract"* is `cite-check` semantics, which proves a quotation is **the form's** words, not **that
   line's**. Line 28's rounding sentence sitting on line 11 passes — and that swap *inverts the
   rounding for Parts II/III*. ★ `tables.rs:1295-1313` already has `printed_line(label)`, built by **T1
   in this same plan**, whose own doc says in terms that the fix is **not** more citation checking Use it:
   `printed_line(<the field's label>)` ∋ the quoted instruction, whitespace-normalized.
3. **The expected set comes from `label_reader`, not a fresh parse.** The text layer yields **50**
   labels; lines 4 and 22 are headings that carry instruction text and no amount box, and the layer
   *cannot say which* — `label_reader.rs`'s own doc says distinguishing a heading from a label
   means knowing whether the line has an amount box — **which the text layer does not directly say**. It
   resolves this with two witnesses and asserts 50 / 48 / `["4","22"]`. Assert against that. ★ The
   plan's gloss elsewhere names line **14** as the second box-less heading — that is wrong; 14a is a
   real entry line, and the two headings are **4 and 22**. The per-part counts (7+12+10+10+8+1 = 48 entry lines) are right.
4. **Completion conditions are part of conformance** (r4 C-I2). Each part's Caution — *"Fill out Part
   II only if you received qualified tips"* — is transcribed as a completion predicate, and the KAT
   asserts an uncompleted part's lines are **not entered**. This is a T2 decision because B4 cannot
   fix it if T2 makes every line non-optional.

### T3 — the input surface, landed whole

~25 leaves plus **≈19 declarations** (r4 M-1 recount: Part II 5, Part III 3, **Part IV 9 per
vehicle**, plus the SSN bar per person for II/III/V — the header said six), through the whole stack in
one pass (the G-9 walk, since the user
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
non-US-assembled car, ~~a refinance~~, a pre-2025 loan, or negative equity. **Understates tax.** The
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

★★★ **r4 CONFORMANCE — TWO ELIGIBILITY CONDITIONS ARE MISSING, BOTH IN THE UNDERSTATEMENT DIRECTION.**
This is the third and fourth instance of the class r1 found twice; this plan already records that r1 found
the same table incomplete in the dangerous direction, **twice**.

**(1) Part IV — the refinance balance cap (r4 C-I3).** r1's C-2 named a refinance **beyond the prior
balance**; the fold reduced that to a bare refinance and kept no condition. Worse, the prose listed a
refinance among the things that DISQUALIFY. The instructions say the opposite, in one paragraph
(`i1040gi--2025.txt:44486`):

> **Refinanced loan.** If your prior loan that had QPVLI is later refinanced, interest paid on the
> refinanced amount is **generally eligible** for the deduction, so long as the new loan is secured by
> a first lien on the APV with respect to which the refinanced loan was incurred. **The loan amount is
> limited to the outstanding balance of the refinanced loan as of the date of the refinancing.**

Without the cap, a cash-out refi deducts interest on the **entire** new balance — every YES-condition
the plan collects answers *yes* — up to $10,000 of interest on non-qualifying principal. ⇒ add a
per-vehicle row asking whether this is a refinancing and, if so, the outstanding balance of the
refinanced loan on the refinancing date — limiting the interest to that fraction; and correct the prose.

**(2) Part II — the illegal-service exclusion (r4 C-I4).** The *"Amounts received that are not
qualified tips"* list has **three** bullets and the table carries one (SSTB). Missing: tips for a
service that is a felony or misdemeanour, and amounts for prostitution or pornographic activity.
★ The occupation gate **cannot** answer this, and the IRS proves it with a matched pair — a bartender
who served alcohol unlawfully has **non**-qualified tips, while a server working legally has qualified
ones. ⇒ one more YES-condition defaulting to NO, worded on that distinction (the *service* must be
legal; the employer's unrelated violations do not disqualify), citing both examples in the prompt.

★ Also carried (r4 N-1): loan requirement 2's *"change in obligor by reason of previous obligor's
death"* exception (an heir assuming a qualifying loan **does** qualify — omitting it fails closed, so
it is a Minor), and the *personal use* operative test (*"more than 50% of the time"*), which the plan
collects as a bare declaration and the instructions define.

### T3a — ★★ THREE LINES HAVE NO INPUT PATH AT ALL, AND MUST NOT PRINT ZERO

Census **F-1**, confirmed against source. `ReturnInputs` carries `w2s`, `int_1099`, `div_1099`, `g_1099`
and **nothing else** (`return_inputs.rs:417-423`). But the form reads:

- **line 5** — *"Qualified tip amount included in Form 1099-NEC, box 1; Form 1099-MISC, box 3; or Form
  1099-K, box 1a."*
- **line 14b** — *"Qualified overtime compensation included in Form 1099-NEC, box 1, or Form 1099-MISC,
  box 3."*

**There is no 1099-NEC, 1099-MISC or 1099-K struct anywhere in the input model.** So both lines would be
blank *because nothing can populate them* — permanently, and indistinguishably on the page from a filer who
truly had no such income. Under `FOLLOWUPS.md` **§G-11** that is not a gap but **fabricated testimony**: a
printed `0` is an affirmative sworn statement that the amount IS zero.

★ **btctax has exactly three lawful moves here — collect, refuse, or genuinely blank — and "silently zero"
is none of them.** For B3, choose per line and record the reason:

| line | move | why |
|---|---|---|
| 4b | **form-directed `-0-`**, with the reason recorded | *"If Form 4137 is not filed, enter -0-."* btctax emits no Form 4137, so that condition is **true of every return it produces** — this is the form's own conditional constant, not a guess. ★ But the existing guard is PARTIAL: `return_refuse.rs:769` refuses on `w2.box8_allocated_tips > 0` (*allocated* tips), while Form 4137 is also required for tips the employee did not report to the employer, which `W2` cannot see. T2 must state whether that refusal needs a companion declaration. (r4 buildability I-5.) |
| 5 | **REFUSE** when the Part II **claim gate** is `Some(true)` | ★★ **NOT `schedule_c.is_some()`** (r4 buildability I-4). A Schedule C **is the mining household** — btctax's core case — and Part II's own Caution makes the deduction opt-in: *"Fill out Part II only if you received qualified tips."* Refusing on the presence of a Schedule C would refuse **every** TY2025 mining return once the fail-closed gate comes out, on a part the filer was told not to complete. The tree already learned this: `questions.rs:546-552` records a class-(A) declaration that was `live: |_| true` and blocked **every** return btctax could compute, buying nothing. T3 already collects the gate (occupation on the Treasury list, defaulting to NO), so condition the refusal on the CLAIM. |
| 14b | **REFUSE** when the Part III **claim gate** is `Some(true)`, else genuinely blank | Same correction. ★ And the old rationale — that with no Schedule C there is no payor relationship to report — is wrong on its own terms for the **1099-MISC box 3** half — that is Other Income on Schedule 1 line 8z, not Schedule C. |

**Collecting the 1099 surface is the right long-term answer** (CLAUDE.md: *if the form asks something our
input surface cannot answer, collect it* — that is following instructions, not scope creep). It is out of
scope for B3 only because it is a new multi-form input surface with its own spec; file it, do not fake it.

★ **F-2 makes the line-5 ceiling un-implementable as specified, so it refuses rather than computing.**
Plan r2 folded "net profit − Schedule 1 line 15". The instructions require more: *"including the
deductible part of self-employment tax; the deduction for contributions to self-employed SEP, SIMPLE, and
qualified plans; and the self-employed health insurance deduction, but not including the deduction for
qualified tips."* Printed Schedule 1 Part II carries lines **15/18/21 only** (`printed.rs:384-387`) — no
SEP/SIMPLE field, no self-employed-health-insurance field, and (**F-3**) no Schedule E or Schedule F input
at all, which worksheet column (b) also reads. A ceiling built from what we have is structurally **too
high** ⇒ line 5 too large ⇒ **understates tax**. Computing it anyway would be the compression this
project's standing rule exists to forbid.

### T4 — compute, transcribed line by line

**The skip branches are the risk, not the arithmetic** (spec F-5):

- Lines **10, 18, 27** are the same routing three times, and each is quoted here as the form prints it
  rather than as one bracketed composite (a synthesized quotation is not a citation — `xtask cite-check`
  rejects it, which is how this one was caught): line 10 *"If zero or less, enter the amount from line 7
  on line 13"*, line 18 *"If zero or less, enter the amount from line 15 on line 21"*, line 27 *"If zero
  or less, enter the amount from line 24 on line 30"*. A jump **past** the phase-out, not a zero.
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

★ **First, delete a comment that expires** (census **F-6**). `return_1040.rs:1269` is
`let schedule_1a_additional = Usd::ZERO;` under a comment reading *"the 2024 form has no such line, so zero is the RIGHT
value there, not a stub."* That is true for TY2024 and **false the moment TY2025 lands** — and a comment
cannot red. It is §G-11's shape in miniature: a correct blank and a laundered one sharing one code path.
T5 replaces it with the real line 38 and a per-year assertion that TY2024 still yields zero *because the
form has no such line*, not because a literal says so.

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
