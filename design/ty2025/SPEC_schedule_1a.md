# Schedule 1-A (TY2025) — SPEC

**Status: DRAFT r3** (r1 folded 2026-07-29 — 1 Critical, 2 Important, 2 Minor; 6 findings refuted. r2→r3: §7's two remaining open questions CLOSED against the archived instructions, and §7a folds in five facts the extraction surfaced — see F-1 … F-5). Spun out of `design/ty2025/SPEC.md` §8a **B3**, which sized it
as "its own spec-sized feature, not a section of one": 38 lines, six parts, four phase-outs, a filed
VIN, and ~25 collected inputs across five crates.

Passes an independent review loop to 0 Critical / 0 Important before an implementation plan.
Parent decisions (D-1 … D-11) in `design/ty2025/SPEC.md` bind here unless restated.

---

## 1. Sourcing of record — READ THIS FIRST

★ **`design/full-return/recon/fable/` already transcribes Schedule 1-A in full**, written 2026-07-11
against the enacted Pub. L. 119-21 and the TY2025 finals. Use it; do not re-derive. The parent spec
learned that lesson expensively (§2a): the SALT rule was re-derived from scratch and got the MFS shape
wrong **twice** when `01`'s line 13 already named that exact error.

| file | what it holds for Schedule 1-A |
|---|---|
| `03-followon-math-sch1a-qbi-ctc.md` §1.0–1.7 | per-part line formulas, statutory cites, input-side definitions, and **four worked examples** |
| `01-ty2025-finals-obbba.md` §190–195 | the six-part table with caps, thresholds and rounding directions |

**Authority** (archived, hashed, in `design/amt-form6251/`): `f1040s1a--2025.pdf` (`64f97b38`) and
`i1040gi--2025.pdf` (`482e9c48`), which carries the Schedule 1-A instructions. The recon is our own
notes — **re-verify each figure against those finals at write time**.

---

## 2. Binding decisions

**S-1. THE PHASE-OUT ROUNDING DIRECTION IS A PARAMETER, NEVER A SHARED CONSTANT.** The single most
dangerous fact in this form, and it is statutory rather than an IRS quirk:

| part | rule | direction | statute |
|---|---|---|---|
| II — tips | $100 per $1,000 over MAGI | **floor** (line 11: "decrease the result to the next **lower** whole number") | §224(b)(2) — "for each $1,000" |
| III — overtime | $100 per $1,000 | **floor** (line 19, same wording) | §225(b)(2) — "for each $1,000" |
| IV — car loan | $200 per $1,000 | **CEIL** (line 28: "**increase** the result to the next **higher** whole number") | §163(h)(4)(B)(iii) — "for each $1,000 **or portion thereof**" |
| V — seniors | 6% of the excess | **smooth**, no stair-step at all | §151(d)(5)(C) |

★ Schedule 8812 line 10 also **ceils**, for the same "or fraction thereof" reason (§24). So a
`phase_out(excess, per_step, step)` helper with a baked-in direction is **silently wrong on one side**.
The direction is an explicit argument, and **each part carries its own test at its own knee**. The
recon's worked examples (b) and (c) are exactly the $100 and $200 errors a shared helper produces.

**S-2. ONE MAGI, SHARED.** Part I line 3 is `1040 L11b + §933 PR + Form 2555 L45 + L50 + Form 4563 L15`
— the statutory definition is *identical* in §224(d), §225(d), §151(d)(5)(C) and §163(h)(4)(C), and the
**same** quantity drives §164(b)'s SALT phase-down (already built) and **Schedule 8812**. It is
computed once. `ReturnInputs::modified_agi` and the four `has_income_exclusion`-gated amounts already
exist (parent D-9); this spec consumes them and adds no new MAGI surface.

**S-3. THE TIPS AND OVERTIME CAPS ARE PER-RETURN, NOT PER-SPOUSE.** $25,000 tips "regardless of your
filing status" (§224(b)(1), and the form's line 7 prints no MFJ figure); $12,500 overtime, $25,000 MFJ
(§225(b)(1)). A combined per-return cap, so an MFJ couple with two tipped earners shares one $25,000 —
the natural per-person reading is wrong and would overstate the deduction.

**S-4. PART V's REDUCTION IS COMPUTED ONCE, THEN MULTIPLIED.** `L35 = max(0, 6,000 − 6% × L33)` is a
**per-person** amount reduced once; `L37 = L35 × (qualifying individuals)`. So an MFJ couple with two
seniors loses **12¢ per $1** of MAGI in the band. Everyone reaches $0 at `threshold + 100,000`.
★ §151(d)(5) stacks **on top of** the §63(f) aged-65 addition and, unlike §63(f), **survives itemizing**
— it is codified in §151, not §63(f).

**S-5. FILING-STATUS AND ELIGIBILITY BARS ARE PART OF THE TRANSCRIPTION** (parent D-7). Parts II, III
and V each print "If married, you must file jointly to claim this deduction" ⇒ **zero for MFS**. Part
IV carries no such caution ⇒ **allowed for MFS**, adjudicated against the form against OTS, which bars
all four. Parts II/III/V additionally require a **valid SSN** per person.

**S-6. EVERY NUMBERED LINE IS COLLECTED** (parent D-9), and each "see instructions" branch is
transcribed or refuses (parent §2's r1-8 note).

★ **The branch list below is illustrative, NOT exhaustive** — the r1 draft called it "all", which r1
falsified: the instructions carry four "Keep for Your Records" worksheets and several branches the
list omitted. **The enumeration of record is the instructions themselves; the build walks them.** Known
so far: line 4's more-than-one-occupation case; line 4a's W-2-box-5-over-$176,100 case; the *Qualified
Tips From More Than One Employer* worksheet; the *Multiple Trades or Businesses* worksheet (line 5);
line 14a's amounts not in W-2 box 1; line 22's ">two VINs ⇒ attach a statement"; 36a's valid-SSN
condition; and S-8's death carve-out.

**S-7. THE PER-YEAR TABLE COVERS 2025–2028 AND NOTHING IS INDEXED.** All four provisions expire after
TY2028. Caps and thresholds are fixed dollar amounts in the statute, so a "next year's Rev. Proc."
lookup is not merely unnecessary — it is wrong. **TY2029+ must fail closed**, like TY2026 does today.

**S-8. THE DEATH CARVE-OUT IS A RULE, NOT AN OPEN QUESTION — AND IT IS ALREADY A LIVE DEFECT.**
r1's Critical. The r1 draft filed this as OQ-1 ("does btctax collect a date of death?") when the
archived instructions state it flatly, with the IRS's own boundary pair:

> **Death of a taxpayer in 2025.** If a taxpayer was born before January 2, 1961, but died in 2025
> before reaching age 65, then the taxpayer **doesn't qualify** for the enhanced deduction for seniors.
> A person is considered to reach age 65 on the day before the person's 65th birthday.

and, in the Part V narrative: born **1960-02-14**, died **2025-02-13** ⇒ qualifies; died **2025-02-12**
⇒ **does not**. Under parent D-10 tier 1 a worked example in the instructions is the *strongest*
evidence class in this project — so deferring it as a question inverted D-10's own ranking, and §1's
"grep the archive before deferring" applied exactly.

★★ **Checking it exposed a LIVE DEFECT in shipped code**, filed as `FOLLOWUPS.md` §G-9: the identical
rule governs the §63(f) **spouse aged box** on 1040 line 12a, which btctax files today from
`is_aged(dob, year)` — date of birth alone, no death branch, no date of death collected. A spouse who
died in-year before reaching 65 gets +$1,550 of standard deduction (TY2024), **understating tax**, and
neither oracle can catch it (OTS takes a filer-answered Y/N; taxcalc has only `age_spouse`), so every
gate stays green. **G-9 was a prerequisite for Part V, not a consequence of it** — and it is now **CLOSED** (fixed 2026-07-29; `is_aged` takes the death gate and date, KATs mutation-verified). Part V inherits the machinery: `HouseholdHeader::{taxpayer,spouse}_died_during_year` plus `Person::date_of_death`, with the day-before-the-65th-birthday convention in `reaches_65_on` — Part V multiplies the
same predicate's stake to $6,000 per person.

**Therefore:** a per-person date of death is collected (D-5 semantics — the tri-state gate plus a date,
exactly like `has_income_exclusion`: "did they die during the tax year?" → yes requires the date,
unanswered refuses), `is_aged` grows the death branch, and **both sides of the Feb-13/Feb-12 pair are
KATs**, mutation-verified.

---

## 3. Non-goals

- **Not** an optimizer change. The phase-out bands add hidden marginal-rate adders (each $1,000 of MAGI
  claws back $100 + $100 + $200 staired and 6%/12% smooth), so per-$1 what-ifs will show $0 then a
  cliff. ★ **Document that; do not "fix" it** — the step function is the law.
- **Not** Schedule 8812. It shares S-2's MAGI and S-1's ceil, and is its own work.
- **Not** determining whether an occupation is on the Treasury tipped-occupation list. The filer
  answers; we record the code.

---

## 4. Scope

### 4.1 Inputs (~25 leaves)
Part II: `L4a` W-2 box 7, `L4b` Form 4137, `L4c` multi-employer resolution, `L5` per-business
trade-or-business tips **plus** each business's net-profit ceiling. Part III: `L14a` W-2-side FLSA
**premium** portion, `L14b` 1099-NEC/MISC side. Part IV: per-vehicle **VIN**, total QPVLI, and
column (ii)'s portion already deducted on Schedule C/E/F. Part V: derived from `date_of_birth`
(already collected for §63(f)) **plus** the valid-SSN predicate per person.

★ **Definitional traps in the input surface — FOUR, not two.** The r1 draft summarised the recon and
dropped half of §225(d)'s exclusions; all of these are prompt wording rather than arithmetic, and
**none is visible on any W-2**:

1. *Qualified overtime* is only the **premium ("half") portion** of FLSA §7 time-and-a-half — not
   double-time's second half, not holiday or weekend premiums absent >40 hours.
2. ★ **It excludes any amount received as a qualified tip** — no double-dip between Parts II and III.
   The green suite offers the same dollars to both parts and the surface must refuse to count them
   twice.
3. ★ **State-law-only overtime for FLSA-ineligible employees does not qualify** — the entitlement must
   arise under FLSA §7.
4. Tips from an **SSTB** employer, or SSTB self-employment tips, are **not qualified** (§224(d)(3)).

### 4.2 Computation
Six parts, transcribed line by line in the form's own numbering with the printed text as doc comments.
Order: `AGI → Part I MAGI → Parts II–V → L38 → then Form 8995 line 11` (which subtracts L13b) →
taxable income. Schedule 1-A never reads a deduction, so the DAG stays acyclic.

### 4.3 Emission
`L38 → 1040 L13b`; `L37 → Form 6251 line 1a` (parent D-3 — the *senior* subtotal, not the total).
File Schedule 1-A only when `L38 > 0`. Needs a new PDF and AcroForm map (parent §5.4), including the
VIN's per-character comb boxes — `recon/fable/05-ty2025-field-maps.md` has the extracted field names.

---

## 5. Test / green definition

1. **The five gates**, as the parent spec defines them.
2. **Each part's phase-out tested AT ITS OWN KNEE, in its own direction** (S-1). At minimum the recon's
   worked examples: (b) single, MAGI $157,350, tips $3,000 ⇒ **$2,300** (a ceil gives $2,200); (c)
   single, MAGI $104,050, QPVLI $6,000 ⇒ **$5,000** (a floor gives $5,200). ★ A test that passes under
   both rounding modes has not tested the rounding.
3. **Both oracles per part**, with disqualifications **computed and sized** — and OTS 2025's Part IV is
   already known defective three ways, while taxcalc has the wrong QSS threshold (parent D-8). So
   **QSS Part IV in the phase-out band ships zero-oracle** and the census must say so.
4. **All five filing statuses**, because S-3's per-return caps and S-5's MFS bar are status-dependent.
5. **A two-senior MFJ case** proving the 12¢-per-$1 aggregate slope (recon example (d): MAGI $200,000
   ⇒ L37 = $6,000, $3,000 each).
6. **`L38 > 0` gates filing**, and `L37` (not `L38`) reaches Form 6251 line 1a.
6b. ★ **A Schedule 1-A deduction moves 1040 L15 and Form 8995 line 11 — and NOTHING ELSE.** These
   deductions sit **below** the AGI line, so every AGI-keyed quantity must be byte-identical with and
   without them: Form 8960's NIIT MAGI, Schedule A's 7.5% medical floor, the §164(b) SALT phase-down
   MAGI, and the IRA/student-loan phase-outs. Assert that directly — it is the cheapest possible guard
   against wiring a below-the-line deduction into an above-the-line consumer. (And all four parts are
   available whether the filer itemizes or takes the standard deduction.)
7. **TY2029 fails closed**, mutation-verified, like `ty2026_full_return_must_stay_fail_closed`.
8. **Mutation-verified guards**, and — the parent's hardest-won lesson — **a test whose mutation
   survives is not a test**. Two of this session's guards were vacuous until a mutation said so.

---

## 6. Risks

**R-1. The rounding asymmetry** (S-1). A shared helper is the natural implementation and it is wrong.

**R-2. Input definitions with no source document.** Qualified overtime's premium-half rule and the
SSTB exclusion are invisible on a W-2; the filer must be asked precisely. Wrong prompt wording is a
wrong return that every test passes.

**R-3. Part IV has the weakest oracle coverage of anything in TY2025** — three OTS defects plus
taxcalc's QSS threshold. Expect to lean on the form's own arithmetic and say so.

**R-4. The VIN is a new class of filed data** — a per-character comb-box string. Parent §8's OQ-3
recorded that the generic PII scanner does not cover VINs and why; the emitter's tests assert no
VIN-shaped literal in committed fixtures.

**R-5. Expiry.** Four provisions die after TY2028 (S-7). A table that quietly extends them files a
deduction that does not exist.

---

## 7. Open questions

1. ~~Does Part V's death rule reach us?~~ — **CLOSED, and it was never a question.** The archived
   instructions state it flatly with a worked boundary pair; see **S-8**. Checking it found a live
   defect in shipped code (`FOLLOWUPS.md` §G-9). btctax collects no date of death — that is the fix,
   not the question.
2. ~~Per-business or aggregate for line 5's net-income limitation?~~ — **CLOSED against
   `i1040gi--2025.pdf` p.101-110** (*Instructions for Schedule 1-A*, **Net income limitation**). Per
   business, stated flatly: *"The net income limitation applies to each separate trade or business in
   which you [received qualified tips]."* The recon's reading was right — **and the instructions are
   stricter than it recorded.** The limitation is NOT simply net profit:

   > Qualified tips from a trade or business can't be more than the gross income from the trade or
   > business in which the qualified tips were received minus the total of all deductions allocable to
   > that trade or business, **including the deductible part of self-employment tax; the deduction for
   > contributions to self-employed SEP, SIMPLE, and qualified plans; and the self-employed health
   > insurance deduction, but not including the deduction for qualified tips.** … reduce the net profit
   > (Schedule C, line 31; the total of Schedule E, line 28(g) through 28(k); or Schedule F, line 34) by
   > the amount of these deductions. **Do not reduce it below zero.**

   So the ceiling is a **derived** per-business quantity, not a leaf: three named deductions come off
   the net profit, floored at zero, and the tips deduction itself is expressly excluded from the
   subtraction (no circularity). ★ Under the transcription rule this ceiling is its own transcribed
   worksheet — the *Multiple Trades or Businesses Worksheet* — not a `min()` in the emitter.
3. ~~Notice 2025-69 transition relief~~ — **CLOSED: it governs the SSTB determination, not what we
   accept as `L4a`.** Verbatim: *"Until the issuance of final regulations determining whether a trade or
   business is an SSTB for purposes of this deduction … the IRS will treat employees and self-employed
   individuals as having received tips in the course of a trade or business that is **not** an SSTB if
   the employee is in an occupation that customarily and regularly received tips on or before December
   31, 2024."* It **relaxes** §4.1 trap 4, and changes no line's arithmetic. The prompt for the SSTB
   question must carry the relief, or it will refuse filers the statute allows.

### 7a. Facts the extraction added, folded here rather than left implicit

**F-1. ★ W-2 box 7 is NOT the qualified-tips figure — it is only the starting point.** Verbatim: *"For
tax year 2025, Form W-2, Form 1099-NEC, Form 1099-MISC, and Form 1099-K **were not updated to
separately identify tips that may qualify for this deduction**."* §4.1 describes `L4a` as "W-2 box 7",
which is how the form's own line reads, but the amount is a filer-determined SUBSET of it and the line
carries two branches off it (box 5 over $176,100; tips not subject to social security and Medicare
tax). So `L4a` is a **collected declaration with its own worksheet**, never a projection of a W-2 leaf
— exactly the answered-ness class. This is the single largest input-surface consequence in the form.

**F-2. Four "Keep for Your Records" worksheets, located.** *Qualified Tips From More Than One Employer*
(instr. p.417), *Multiple Trades or Businesses* (546), *Qualified Overtime Compensation From More Than
One **Employer***  (1039) and *… From More Than One **Payor*** (1056) — note the last two are DISTINCT
worksheets, W-2 side and 1099 side, which the r1 branch list collapsed into one.

**F-3. "Valid SSN" has a definition, and it is a date test.** *"A valid SSN … is one that is valid for
employment and that is issued by the Social Security Administration (SSA) before the due date of [the
return]."* So Parts II/III/V's SSN bar is not "an SSN is present" — btctax cannot see validity-for-
employment or issue date, so it is a **declaration**, per person.

**F-4. Part IV's threshold differs from every other part.** Line 26 is **$100,000 ($200,000 MFJ)**,
against $150,000/$300,000 for Parts II–III (lines 9, 17) and $75,000/$150,000 for Part V (line 32).
Three distinct threshold pairs on one form — a shared `threshold_for(status)` is wrong the way S-1's
shared rounding helper is wrong.

**F-5. Two lines SKIP rather than zero.** Lines 10, 18 and 27 read *"If zero or less, enter the amount
from line 7 [15, 24] on line 13 [21, 30]"* — a jump PAST the phase-out, not a zero. Line 33 reads *"If
zero or less, **enter $6,000 on line 35**"* — a jump that writes a NONZERO constant into a later line.
Transcribing 33 as `max(0, …)` and letting 34–35 run gives the same answer only because 6% × 0 = 0;
transcribing it as `-0-` gives **$0 instead of $6,000**. Pin the branch.

---

## 8. Cross-references

- `design/ty2025/SPEC.md` — parent; D-1 … D-11, and §8a's branch plan.
- `design/full-return/recon/fable/03-…md` §1.0–1.7 — **the transcription of record**.
- `design/ty2025/reviews/` — the parent's r1/r2/r3.
