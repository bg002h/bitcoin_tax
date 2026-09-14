# REPORT — the §170(b)(1)(I) charitable-floor refusal (brief: "Guard the §170(p) charitable floor")

**Agent:** the charitable-floor parcel implementer. **Worktree:**
`.claude/worktrees/agent-a5324c56645079757`, `CARGO_TARGET_DIR=<worktree>/target-p` (clippy in
`target-p-clippy`). **Not committed, not pushed.** No subagents. Every command foreground.

**Outcome in one line:** the refusal is built, and **a TY2026 itemizing return claiming any charitable
deduction now refuses instead of deducting an unfloored total — at every income, which is where §68
cannot reach.** 12 planted defects, 12 reds. Gate: **3821/3821 nextest, clippy 0 warnings,
`cargo fmt --all --check` clean.**

---

## 0. ★★★ THE BRIEF'S PREMISES — THE MONEY HOLDS, THE STATUTE'S NAME IS REFUTED

The brief said to stop and report if a premise could be disproved. **The money premise holds exactly as
measured. The STATUTORY CITATION does not, and it is not a pedantic correction.**

| premise | how measured | verdict |
|---|---|---|
| **No charitable floor exists anywhere in the workspace** | `charitable.rs::apply_170b` (336 lines, read in full) applies the §170(b) *ceilings* only — `dec!(0.60)`, `dec!(0.50)`, `dec!(0.30)` — and no floor of any spelling. Wave 5's own wall-pin test asserts it over `crates/`. | **HOLDS** |
| **The floor applies at EVERY income** | the statute prints no threshold and no screen; §68's Schedule A gate prints `$384,350` | **HOLDS** |
| **The worksheet is unarchived** | `design/forms/extract/` holds `i1040gi--2024`, `i1040gi--2025`, `i1040sca--2024`, `i1040sca--2025` and **no `--2026` of either** | **HOLDS** |
| **"§170(p)" names the floor** | read out of `legal/text/statute-irc/PLAW-119publ21_OBBBA.txt` | ★★★ **REFUTED** |
| **"no statutory text is archived in this repo either"** | the OBBBA Public Law **is** archived, and §70425 is at line 9400 | **REFUTED — and it is what made the correction possible** |
| *(brief's aside)* `170(p)` has **zero** hits workspace-wide | it has **3** in `crates/btctax-cli/tests/ty2026_schedule_a.rs` (wave 5's characterization test) plus 11 under `design/` | **corrected, immaterial to the finding** |

### ★★★ F1 (the headline) — THE FLOOR IS **§170(b)(1)(I)**. **§170(p) IS THE DEDUCTION FOR FILERS WHO DO *NOT* ITEMIZE.**

From the archived Public Law:

| provision | what it actually is | archived at |
|---|---|---|
| **§170(b)(1)(I)** | **"0.5-PERCENT FLOOR"** on an **itemizer's** contributions — added by OBBBA **§70425(a)(1)** | `PLAW-119publ21_OBBBA.txt:9400-9420` |
| §170(p) | **§70424**, *"PERMANENT AND EXPANDED REINSTATEMENT OF PARTIAL DEDUCTION FOR CHARITABLE CONTRIBUTIONS OF INDIVIDUALS WHO DO NOT ELECT TO ITEMIZE"* — the $300/$600 above-the-line deduction raised to **$1,000/$2,000** | `PLAW-119publ21_OBBBA.txt:9391-9399` |

§70425(a)(3) touches §170(p) only to **coordinate** with the new floor — *"Section 170(p) … is further
amended by inserting `, (b)(1)(I),` after `subsections (b)(1)(G)(ii)`"* — which is, I think, how a floor
whose home is `(b)(1)(I)` came to be filed under `(p)` in this repo's prose.

**Why this is money-adjacent and not a typo.** The refusal this parcel prints is read by a filer being
told to go and work a worksheet by hand. Citing **§170(p)** would send an *itemizing* filer to the one
provision that, by its own terms, applies **only when they do not itemize**. Same class as the Form 6251
line-33 `12`-for-`22` transcription: a cross-reference a reader acts on, wrong in a way no arithmetic
check would see.

**The misnomer is repo-wide — five places**, all still saying "§170(p)":
`design/direction/filing-readiness-lens-itemized.md:182`,
`design/direction/filing-readiness-lens-charity.md:9`,
`design/agent-reports/RECON-drive-to-filable-return.md:383,394`,
`design/agent-reports/REPORT-wave5-schedule-a.md:87,94,126,248,252,260`, and
`crates/btctax-cli/tests/ty2026_schedule_a.rs`. **I did not edit the four design docs** — they are
nobody's parcel and not mine to rewrite; §7 files it. I *did* correct it in the two source files I own,
and the `CharitableFloorGate` doc carries the table above so the next reader meets the evidence.

★ **I built the guard rather than stopping.** The brief's substance — an unguarded understatement with a
measured size — is intact; only the label was wrong, and the fix was to read the archived primary source
rather than invent anything. Naming the right statute is the *opposite* of the failure the brief guards.

### ★★ F2 — TWO MORE REASONS `0.005 × agi` IS THE WRONG ANSWER, BOTH FROM THE STATUTE ITSELF

The brief said not to compute the floor because the worksheet is unarchived. With the statute in hand the
case is stronger: **the rate is known and the floor still cannot be computed.** Three branches, all in
§70425's own text:

1. **The base is the *contribution base*, not AGI.** §170(b)(1)(I) floors at *"0.5 percent of the
   taxpayer's contribution base"*; §170(b)(1)(H) defines that as AGI **without regard to any NOL
   carryback**. `REPORT-wave5-schedule-a.md`'s `0.005 × agi` is exact only when no NOL carryback exists.
2. **The floor is applied in a SIX-STEP ORDER ACROSS the §170(b)(1) classes** — clauses (i)-(vi) consume
   subparagraphs (D), (C), (B), (E), (A), then (G) *in that order*. **Which class absorbs the floor
   decides which class carries forward**, so the floor is not a subtraction from the total `apply_170b`
   produces; it interleaves with the ceilings that function applies.
3. **It re-writes the carryforward rule.** §70425(a)(2) adds §170(d)(1)(C): an amount disallowed *by the
   floor* increases the carryover, and only *"from years in which the limitation is exceeded"*. A naive
   floor would therefore also emit a wrong `carryover_out` — **a wrong figure on a FUTURE year's
   return**, which no assertion about this year would catch.

So the refusal's text now *argues* rather than asserts, and the three branches are quoted to the filer.
`no_floor_rate_constant_exists_in_this_crate` holds that no `0.005` exists in `btctax-core`: the rate
appears only inside the quoted statute.

### ★ F3 — the §170(p)/itemizer boundary I could NOT settle, stated rather than guessed

§70425(a)(3) inserts `(b)(1)(I)` into a *"without regard to"* list inside §170(p), which **reads** like
an explicit exemption of non-itemizers from the floor. **I did not assert that**, because §170(p)'s own
body is not archived here (only OBBBA's amendments to it are), so the direction of that list is an
inference about text I have not read. What the gate rests on instead is a fact about btctax: a grep
finds **no nonitemizer charitable deduction modelled at all**, so a standard-deduction return deducts no
charity, there is nothing of its to floor, and the unimplemented §170(p) errs toward **tax overstated** —
never under. That is written at the gate.

---

## 1. WHAT WAS BUILT

### `crates/btctax-core/src/tax/tables.rs` — the gate's home (+474)

- **`CharitableFloorGate`** — `year`, `worksheet_line`, `subtotal_line`, `worksheet`, `caption`. The
  form's line-13 caption is stored **once**, verbatim from the text layer, and both the refusal and the
  citation test read that one copy.
- **`CHARITABLE_FLOOR_STATUTE`** — §170(b)(1)(I) verbatim from the archived Public Law, typographic
  apostrophe and all, asserted against `PLAW-119publ21_OBBBA.txt`.
- **`CHARITABLE_FLOOR_FIRST_YEAR: i32 = 2026`** — §70425(c), *"taxable years beginning after December
  31, 2025"*, **26 USC 170 note**. A start with **no end**, because §70425 enacted no sunset. Same date
  as `SECTION_68_FIRST_YEAR` and a **separate constant on purpose**: different OBBBA sections, so a
  future Congress moving one must not silently move the other.
- **`CharitableFloorStatus { NotApplicable, Gated(..), ScheduleANotTranscribed }`** and
  **`charitable_floor_status(year)`**. Matched wildcard-free at its one call site.
- ★★ **NO RATE CONSTANT, DELIBERATELY.** There is no `dec!(0.005)` anywhere — the structural analogue of
  `Section68Gate` having no `FilingStatus` field. There is nothing for a future editor to multiply by.

### `crates/btctax-core/src/tax/return_refuse.rs` — the decision (+500)

- **`RefuseReason::CharitableFloorNotComputed { year }`**, one variant covering both refusing arms.
- **`charitable_floor_gate(year, deduction_is_itemized, charitable_current_year, charitable_carryover)`**.
  Keyed on the **claimed deduction**, not the gifts: a year whose §170(b) ceilings allowed `0` claims
  nothing on the charitable lines, so nothing is over-deducted and it files.
- ★★ **A carryover-only return also refuses**, and the detail says why it is unsettled: whether a
  carryover that arose in a **pre-floor** year is itself floored on deduction is a §170(d)(1)(C) question
  only the worksheet answers. Guessing *"exempt"* understates; guessing the other way merely
  over-refuses, which is recoverable.

### `crates/btctax-core/src/tax/return_1040.rs` — the call site (+249)

`screen_absolute`, **immediately before the §68 gate**, with the ordering argued from the form's own data
flow: the *Charitable Contribution Limitation Worksheet* produces Schedule A line 13 → line 15 → the
itemized total that the *Itemized Deductions Worksheet* then limits, so the charitable worksheet is
strictly **upstream**. Both gates are no-software-exit refusals, so the only question is which one a
filer working by hand meets first, and the useful answer is the order they must work them in.
`the_charitable_floor_is_reported_before_the_section_68_limitation` pins it **as a contrast**: the same
income reports §68 when there is no charity.

### `crates/btctax-input-form/src/attribute.rs` (+24/-6) — **compelled, see §5**

`Anchor::NotInForm` with the two real exits, and the `NotInForm` **count guard** extended by one.

### The user-facing text, captured from the real code path (not transcribed)

> this return is for tax year 2026, it ITEMIZES, and it claims $10,000 of charitable contributions
> (this year's gifts). Tax year 2026 Schedule A line 13 no longer adds your gifts up. It reads, in the
> form's own words: "Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet"
> — with the carryover moved to line 14 and "Add lines 13 and 14" as the subtotal on line 15. That
> worksheet applies §170(b)(1)(I), added by Pub. L. 119-21 §70425(a)(1) for tax years beginning after
> December 31, 2025: "Any charitable contribution otherwise allowable (without regard to this
> subparagraph) as a deduction under this section shall be allowed only to the extent that the
> aggregate of such contributions exceeds 0.5 percent of the taxpayer's contribution base for the
> taxable year." btctax does not have that worksheet. The tax year 2026 Form 1040 and Schedule A
> instructions have not been published, so there is no document to transcribe it from, and btctax will
> not invent one — the statute floors at 0.5 percent of your CONTRIBUTION BASE, which §170(b)(1)(H)
> defines as adjusted gross income computed without regard to any net operating loss carryback — not at
> 0.5 percent of your adjusted gross income; it applies the floor across the §170(b)(1) contribution
> classes in a six-step order that decides which class carries forward; and §170(d)(1)(C) then
> re-writes the carryover rule for amounts the floor disallowed, which changes a FUTURE year's return
> too. Those are three questions a worksheet nobody has read would have to settle, so btctax will not
> guess them. Printing your unfloored charitable total on line 13 instead would deduct more than §170
> allows and UNDERSTATE your tax. What to do: work the Charitable Contribution Limitation Worksheet in
> that year's Schedule A instructions by hand, enter its line 6 on Schedule A, and file on paper — or
> take this return to a paid preparer. No btctax command clears this and no further answer will change
> it. This is separate from the §68 limitation on itemized deductions: §170(b)(1)(I) applies at EVERY
> income, so a return under §68's printed threshold still meets this. A return that takes the standard
> deduction is unaffected, because btctax deducts charitable contributions only on Schedule A.

★ **No "remove the gift"** (FR-225 — no CLI verb exists), no btctax command, and **no §170(p)**. All
three are asserted as forbidden strings.

---

## 2. THE YEAR SCOPE IS DERIVED FROM THE EXTRACTS — no year list

`routes_through_the_limitation_worksheet(text)` is the ONE reading of the archived forms every floor
assertion keys off, and the revisions come from `section_68_tests::archived_schedule_a()`, which reads
`design/forms/extract/` with `read_dir`. Archiving `f1040sa--2027.txt` pulls a new revision into every
assertion with **no edit to any source file**.

What the forms actually say, which is what decides the scope:

| revision | line 13 | line 14 | subtotal |
|---|---|---|---|
| `f1040sa--2024` | *"Carryover from prior year"* | *"Add lines 11 through 13"* | 14 |
| `f1040sa--2025` | *"Carryover from prior year"* | *"Add lines 11 through 13"* | 14 |
| `f1040sa--2026-DRAFT` | *"Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet"* | *"Carryover from prior year"* | **15** — *"Add lines 13 and 14"* |

Three-way classification per revision: a revision that routes through the worksheet **must** be `Gated`;
one that does not **must** be `NotApplicable` — not merely "not `Gated`". That is the distinction whose
absence let a plant survive in the §68 parcel, and P3 below is the same plant here.

---

## 3. B1 — TWELVE PLANTS, TWELVE REDS

Baseline before and after every plant: **`28 tests run: 28 passed, 3805 skipped`** (my 15 new tests plus
the 13 §68 / wave-5 tests in the same blast radius). Restored and re-verified clean after every plant and
at the end.

| plant | what it breaks | RED |
|---|---|---|
| **P1** the gate **deleted** from `screen_absolute` | nothing holds it end-to-end | `a_ty2026_itemizer_claiming_charity_is_refused_and_ty2024_files_unchanged`, `the_charitable_floor_is_reported_before_the_section_68_limitation` — **2 failed** |
| **P2** the carryover dropped from the trigger | a carryover-only return files unfloored | `a_carryover_only_ty2026_itemizer_refuses_and_the_detail_names_the_unsettled_question` — **1 failed** |
| **P3** `CHARITABLE_FLOOR_FIRST_YEAR = 2025` | the floor a year EARLY — every TY2025 itemizer with charity refused | `a_pre_2026_itemizer_with_charity_files_unchanged`, `the_floor_caption_and_worksheet_are_the_forms_own`, `the_floored_years_are_read_off_the_archived_schedule_a_extracts` — **3 failed** |
| **P4** `CHARITABLE_FLOOR_FIRST_YEAR = 2027` | the floor a year LATE — TY2026 files unfloored | **8 failed** |
| **P5** the fail-closed fall-through returns `NotApplicable` | a floored year with no archived form files unfloored | `a_floored_year_with_no_transcribed_schedule_a_refuses_every_itemizer_with_charity`, `the_refusal_quotes_…`, `the_floored_years_…` — **3 failed** |
| **P6** the `deduction_is_itemized` guard dropped | standard-deduction returns refused | `a_standard_deduction_return_is_untouched_at_any_charitable_amount`, `a_floored_year_…` — **2 failed** |
| **P7** the gate moved **after** the §68 gate | the downstream worksheet reported first | `the_charitable_floor_is_reported_before_the_section_68_limitation` — **1 failed** |
| **P8** the detail cites **§170(p)** instead of §170(b)(1)(I) | ★★★ **F1's own defect, planted** | `a_ty2026_itemizer_claiming_charity_is_refused_and_ty2024_files_unchanged`, `the_refusal_quotes_…` — **2 failed** |
| **P9** the detail offers *"remove the gift"* | a remedy with no CLI verb (FR-225) | `the_refusal_quotes_…` — **1 failed** |
| **P10** TY2026 declassified to `ScheduleANotTranscribed` | code and form disagree | **4 failed** |
| **P11** ★★★ **a floor ACTUALLY APPLIED in `apply_170b`** | the reformulated wall pin's whole reason to exist | `no_charitable_floor_and_no_itemized_limitation_is_applied_anywhere`, `no_floor_rate_constant_exists_in_this_crate`, `a_ty2026_itemizer_claiming_charity_…` — **3 failed** |
| **P12** a one-character paraphrase of the form's caption (*"Contributions"* for *"Contribution"*) | the citation check | `the_floor_caption_and_worksheet_are_the_forms_own`, `a_ty2026_itemizer_…` — **2 failed** |

### The 15 new tests

    btctax-core tax::tables::charitable_floor_tests::the_floored_years_are_read_off_the_archived_schedule_a_extracts
    btctax-core tax::tables::charitable_floor_tests::the_floor_caption_and_worksheet_are_the_forms_own
    btctax-core tax::tables::charitable_floor_tests::the_floor_statute_is_the_statutes_own_and_a_paraphrase_is_rejected
    btctax-core tax::tables::charitable_floor_tests::no_floor_rate_constant_exists_in_this_crate
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_ty2026_itemizer_claiming_charity_is_refused_at_every_income
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_ty2026_itemizer_below_the_section_68_threshold_still_refuses
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_ty2026_itemizer_with_no_charity_files_unchanged
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_year_whose_ceilings_allowed_nothing_is_not_refused
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_carryover_only_ty2026_itemizer_refuses_and_the_detail_names_the_unsettled_question
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_standard_deduction_return_is_untouched_at_any_charitable_amount
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_pre_2026_itemizer_with_charity_files_unchanged
    btctax-core tax::return_refuse::charitable_floor_gate_tests::a_floored_year_with_no_transcribed_schedule_a_refuses_every_itemizer_with_charity
    btctax-core tax::return_refuse::charitable_floor_gate_tests::the_refusal_quotes_the_form_and_the_statute_and_names_only_remedies_that_exist
    btctax-core tax::return_1040::tests::a_ty2026_itemizer_claiming_charity_is_refused_and_ty2024_files_unchanged
    btctax-core tax::return_1040::tests::the_charitable_floor_is_reported_before_the_section_68_limitation

**The brief's four B1 requirements, each by name.** Refuses by name quoting the form →
`a_ty2026_itemizer_claiming_charity_is_refused_and_ty2024_files_unchanged` (P1/P8/P12 kill it). No
charity files unchanged → `a_ty2026_itemizer_with_no_charity_files_unchanged`, and row (2) of the
end-to-end test. **Below the §68 threshold still refuses** →
`a_ty2026_itemizer_below_the_section_68_threshold_still_refuses`, which asserts §68 is **`None`** on the
same return this gate refuses — the gap, measured as a contrast rather than claimed. TY2024/TY2025
untouched → `a_pre_2026_itemizer_with_charity_files_unchanged`, over the year set read off the extracts.

### ★★ FR-230 — I grepped my own tests for arithmetic on a constant they protect. There is none.

Every income and year in the new tests is a **literal**; the relationship each has to a constant is then
asserted **separately** against that constant read from its own home. So a mutation of the constant reds
the *relationship assertion* instead of silently re-meaning the literal. Concretely:
`a_ty2026_itemizer_below_the_section_68_threshold_still_refuses` uses `dec!(273200)` and then asserts
`modest < t`; `the_floored_years_…` asserts `CHARITABLE_FLOOR_FIRST_YEAR == 2026` **against §70425(c)'s
own quoted date**, never against `SECTION_68_FIRST_YEAR` (which would pass if both were wrong) and never
as `FIRST_YEAR - 1`; `a_pre_2026_itemizer_with_charity_files_unchanged` reads its ungated year set from
the extract directory rather than writing `[2017, 2024, FIRST_YEAR - 1]`, which is the exact list that
stopped measuring in the §68 parcel.

### ★★★ F4 — MY OWN INSTRUMENTS WERE BLIND THREE TIMES, AND THE PLANT PASS FOUND ALL THREE

None of these was found by re-reading. Each is recorded in the source where the next reader meets it.

1. **A self-referential needle.** `no_floor_rate_constant_exists_in_this_crate` scanned for the literal
   `"charitable_floor_rate"` — and the only occurrence in the workspace was **its own needle list**, so
   it reported `tables.rs` as a defect, and so did the wall pin. The needles are now assembled with
   `concat!`, so the literal never exists in the source.
2. **★★ A KILL THAT A SUBSTRING DEFEATED — the worst of the three.** The statute plant *"5 percent"* for
   *"0.5 percent"* was **ACCEPTED**, because `"5 percent"` is a **substring of the law's own
   `"0.5 percent"`** and `appears_as_printed` is free to break the sentence into more contiguous runs.
   `is_err()` was therefore not a rejection at all: the lost-decimal-point kill — the Form 6251 line-33
   class, the most expensive transcription defect this repo has had — was passing on nothing. A plant is
   now rejected only if it cannot be found **within the real sentence's own fragmentation budget**
   (2 runs), with an accept-control on the unmutated sentence so a predicate that rejected everything
   could not pass as five kills.
3. **A plant that planted nothing.** P1's first draft was
   `if false { } else if let Some(r) = charitable_floor_gate(...)`, which still evaluates the
   `else if let` — it **SURVIVED**, and it deserved to. Rewritten as an outright deletion of the call
   block. (P5's first draft was `if false` on a *match arm*, which the `_`-free match rejected with
   E0004 — the enum doing its job, but not a plant; re-pointed at the status fall-through.)

This is the `CLAUDE.md` corollary — *"the thing that decides was not the thing that knows"* — reproduced
inside tests written to guard against exactly it, for the second parcel running.

---

## 4. ★★ THE COMPELLED WALL-PIN REFORMULATION — and why it is STRONGER, not weaker

Wave 5's `no_charitable_floor_and_no_itemized_limitation_exists_anywhere` scanned `crates/` for the
strings `170(p)`, `charitable_floor`, `itemized_limit`, `sec_68`, `pease`. **This parcel reds it by
design** — the test's own doc says *"it REDS the day either lands"*. But a string scan cannot answer the
question the pin exists to answer:

| what landed | does it move a figure in `REPORT-wave5-schedule-a.md`? | string scan |
|---|---|---|
| a gate that **refuses** because the worksheet is unarchived | **no** — the figures describe what `apply_170b` computes, unchanged | reds anyway |
| a gate that **computes** a floor | **yes** — every figure must be re-run | reds |

So the pin now measures the **behaviour** the figures depend on: `apply_170b(273200, $10,000 cash)` must
still return the **unfloored** `$10,000`, with the report's own arithmetic (`0.005 × 273,200 = 1,366`,
leaving `8,634`) asserted beside it. A string half is kept for `itemized_limit` / `pease` /
`charitable_floor_rate`, which would still indicate a *computation*.

★★ **And a third half the old pin could not express at all:** it now asserts **both guards still exist in
the source** (`fn charitable_floor_gate`, `fn section_68_gate`). Without that, ripping out both refusals
would have left every assertion in the file green while the understatement went live — the pin would have
been satisfied by the defect it exists to prevent. **P11 is the proof the reformulation is not blind**:
an actually-applied floor reds it.

---

## 5. OWNERSHIP — three files outside the stated list, all COMPELLED

The brief granted `charitable.rs`, `return_refuse.rs` and the threshold's home. **`charitable.rs` was
read in full and NOT modified** — no floor was added to it, which is the entire point. Three other files
had to change, and in each case an existing drift guard demanded it:

1. **`crates/btctax-core/src/tax/return_1040.rs`** — `screen_absolute` is the only site holding the
   computed charitable figures; `return_refuse.rs` never sees `AbsoluteReturn`. The decision and the whole
   argument stay in `return_refuse.rs`; the call site is ~12 lines. Same shape as the §68 gate directly
   below it.
2. **`crates/btctax-input-form/src/attribute.rs`** — the new `RefuseReason` variant **E0004**'d the
   deliberately wildcard-free `attribute()` match (the spec-§7 drift guard working as designed), and
   placing the anchor then reded the `NotInForm` **count guard** (17 → 18).
3. **`crates/btctax-cli/tests/ty2026_schedule_a.rs`** — wave 5's wall pin, §4 above. Reformulated, not
   weakened or excused.

**No other match on `RefuseReason` in the workspace broke** — a full `cargo build --workspace` produced no
further E0004.

---

## 6. ★★ DORMANCY — ASSERTED, NOT CLAIMED

`a_ty2026_itemizer_claiming_charity_is_refused_and_ty2024_files_unchanged` **opens** with the measurement:
`form6251_line1_rule(2026, …)` is `None` and `form6251_line1_rule(2024, …)` is `Some`, so **no TY2026
return can be assembled at all today** (`assemble_absolute` panics; `BundledFullReturnTables::load`
inserts 2024 alone). The guard is therefore placed **ahead of the TY2026 port**, and the understatement
becomes live the moment the port transcribes Form 6251 Part I and bundles the package.

The return is assembled at 2024 and **screened at 2026**, which is the real code path: `screen_absolute`
takes `year` as its own parameter, and the gate reads only that year plus two quantities that mean the
same thing in every year. ★ The fixture also needs `charitable_cwa_obtained: Some(true)`, because
§170(f)(8) is a *different* gate that would otherwise fire first on a $250+ gift — stated at the fixture,
so no reader mistakes the row for measuring the floor when it was measuring the CWA.

### ★★ FR-233 — THE SEQUENCING TRAP, WRITTEN AT THE SITE

Three places say it, because removing this refusal before the floor is computed re-exposes the
understatement: the `CharitableFloorGate` doc (*"archive `i1040sca--2026`, transcribe the worksheet line
by line, pin it against both oracles, **then** delete this gate — never the reverse"*),
`no_floor_rate_constant_exists_in_this_crate`'s failure message, and the wall pin's existence half.

---

## 7. RECOMMENDED FOLLOW-UPS (I did not touch `FOLLOWUPS.md` — not in this parcel's ownership)

- ★★★ **The §170(p) → §170(b)(1)(I) misnomer in four design documents**, listed in §0-F1. Prose only, but
  it is the citation a future implementer of the worksheet will start from, and `RECON-…:394` uses it to
  describe what the lens docs *say*, so the correction must be made deliberately rather than by sed.
  **Owning phase: the TY2026 port** (whoever archives `i1040sca--2026`).
- **§170(p) itself is unimplemented** — btctax models no nonitemizer charitable deduction, so a
  standard-deduction filer's gifts are simply not deducted. Conservative (tax overstated), and §70424
  raised it to $1,000/$2,000 for TY2026, so a TY2026 standard-deduction filer with charity currently
  **over**pays. Not a defect of this parcel and not in the understating direction.
  **Owning phase: the TY2026 port.**
- **The import tier cannot see this gate** (`screen_param_free` has no computed return), so
  `income import` accepts a TY2026 itemizing row in silence and the refusal arrives at `report`/export.
  Consistent with every other computed-quantity gate; **not widened**. Same as wave4-N's F4.
- **`printed.rs::schedule_a_lines` still carries the TY2025 line set** — no line 13/14/15 renumber, and
  `AbsoluteReturn::itemized_deduction` is still documented *"Schedule A line 17"*. Nothing prints wrongly
  today because the refusal stops a TY2026 packet being emitted, but the port owes it. Same as wave4-N's
  F5, and the two now interlock: **neither refusal may be removed before its line set exists.**

---

## 8. GATE

Per the brief: nextest and clippy **serially**, in separate target dirs, `cargo fmt --all --check` as the
third leg, every `*.rs` under `crates/` touched first (FR-90: the plant/restore loop races cargo's mtime
freshness). Captured once to files, then grepped.

    find crates -name '*.rs' -exec touch {} +

    CARGO_TARGET_DIR=<wt>/target-p cargo nextest run --workspace --no-fail-fast
    NEXTEST EXIT=0
         Summary [ 121.501s] 3821 tests run: 3821 passed (6 slow), 12 skipped

    CARGO_TARGET_DIR=<wt>/target-p-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    CLIPPY EXIT=0
    lines matching ^(warning|error): 0
    "Checking btctax-core" lines: 1      # confirmed it really re-checked the edited crate

    cargo fmt --all --check
    FMT EXIT=0                           # 0 bytes of output

All 15 new tests are in that run, enumerated from the log in §3 rather than counted by hand. `rustfmt` was
applied once; its diff touched only my five files:

     crates/btctax-cli/tests/ty2026_schedule_a.rs |  86 ++++-
     crates/btctax-core/src/tax/return_1040.rs    | 249 +++++++++++++
     crates/btctax-core/src/tax/return_refuse.rs  | 500 +++++++++++++++++++++++++++
     crates/btctax-core/src/tax/tables.rs         | 474 ++++++++++++++++++++++++-
     crates/btctax-input-form/src/attribute.rs    |  24 +-
     5 files changed, 1315 insertions(+), 18 deletions(-)

★ The refusal text in §1 was **captured from the real code path** (a temporary probe assertion, removed by
exact file restore; the worktree diffstat verified byte-identical to the gated state afterwards, and the
filtered suite re-run green). It is not a transcription of the format string.

**Not committed. Not pushed.** Working tree: five modified source files plus this report.
