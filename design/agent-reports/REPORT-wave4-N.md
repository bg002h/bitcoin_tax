# REPORT — wave 4, parcel N. The §68 refusal.

**Agent:** parcel N implementer. **Worktree:** `.claude/worktrees/agent-abcc740c2af184917`,
`CARGO_TARGET_DIR=<worktree>/target-n` (clippy in `target-n-clippy`). **Brief:**
`design/agent-reports/BRIEF-wave4.md` @ `789bb786a`. **Not committed, not pushed.** No subagents.
Every command foreground.

**Outcome in one line:** the refusal is built, and **§68 now refuses instead of filing an unlimited
itemized total on a year Schedule A gates.** 10 planted defects, 10 reds. Gate: **3781/3781 nextest,
clippy 0 warnings, `cargo fmt --all --check` clean.**

---

## 0. THE THREE PREMISES — ALL THREE HOLD. Nothing refuted.

| premise | how measured | verdict |
|---|---|---|
| **Zero §68 modelling exists anywhere in the workspace** | a recursive case-insensitive grep over `crates/` for `section 68`, `sec. 68`, `pease`, `overall limitation` restricted to `*.rs` returns **0 hits**; so does one for `itemized[_ a-z]{0,25}limit`. Every hit for `384,350` outside `tax_tables.rs` / `testonly.rs` (the MFS bracket) is prose under `design/` | **HOLDS** |
| **`$384,350` prints once on `f1040sa--2026-DRAFT`, with no filing-status parenthetical** | `grep -c` of the numeral in that extract returns **1**, at line 159. The gate sentence carries no `married filing separately`; line 5e directly above it prints `$40,400 ($20,200 if married filing separately)` | **HOLDS** |
| **A semantic quantity is available** | `AbsoluteReturn::agi`, `::qbi_deduction`, `::schedule_1a_additional` all exist (`return_1040.rs:1877, :1921, :1931`) and `screen_absolute` holds the struct | **HOLDS** |
| *(premise behind the premise)* **the worksheet is unreachable** | the extract directory holds `i1040gi--2024`, `i1040gi--2025`, `i1040sca--2024`, `i1040sca--2025` and **no `--2026` of either** | **HOLDS** |

Two findings **sharpen** the brief rather than contradict it — both recorded in the source, both in §4.

---

## 1. WHAT WAS BUILT

### `crates/btctax-core/src/tax/tables.rs` — the threshold's home (+423)

- **`Section68Gate`** — `year`, `threshold`, `total_line`, `worksheet`, `gate_sentence`. The gate
  sentence is stored **once**, verbatim from the text layer, and both the refusal and the citation
  test read that one copy, so the sentence btctax shows a filer cannot drift from the paper.
- **`SECTION_68_FIRST_YEAR: i32 = 2026`** — Pub. L. 119-21 **§70111(c)**, *"taxable years beginning
  after December 31, 2025"*, read out of `legal/text/statute-irc/PLAW-119publ21_OBBBA.txt:5303-5332`.
  A start with **no end**, because §70111 enacted no sunset — unlike `SCHEDULE_1A_YEARS`, so writing
  it as a closed range would invent an expiry.
- **`Section68Status { NotApplicable, Gated(..), ThresholdNotTranscribed }`** and
  **`section_68_status(year)`**. Matched wildcard-free at its one call site.
- The threshold is `dec!(384350)`, **transcribed from `f1040sa--2026-DRAFT.txt:158-163`**, with the
  FR-186 trap spelled out in the doc comment and **no `FilingStatus` parameter anywhere in the type
  or the function** — which is what makes the bracket-table edit impossible rather than discouraged.

### `crates/btctax-core/src/tax/return_refuse.rs` — the decision (+465)

- **`RefuseReason::ItemizedDeductionLimitationNotComputed { year: i32 }`**, one variant covering both
  refusing arms (the `Section68Status` match holds the arms; the `detail` distinguishes them).
- **`section_68_gate(year, deduction_is_itemized, agi, qbi_deduction, schedule_1a_deduction)`**.
  The tested quantity is AGI minus the QBI deduction minus the Schedule 1-A deduction. **No 1040 line
  number is read anywhere**, with FR-214 named in the source as the reason and a table mapping each of
  the gate's three citations to the quantity btctax already computes.
- Keyed on **`deduction_is_itemized`** (the real §63(e) election, computed on the *unlimited* figure),
  not on "has a Schedule A". §68(a) reduces *"itemized deductions otherwise allowable"*; the source
  argues why the unlimited election is also the fail-closed choice.

### `crates/btctax-core/src/tax/return_1040.rs` — the call site (+136)

`screen_absolute`, **last**, immediately before its final `None`, with the ordering argued: every
refusal above it names something the filer can do *inside* btctax; this one has no software exit, so
meeting it first would hide an answerable question behind an unanswerable one.

### `crates/btctax-input-form/src/attribute.rs` (+26/-4) — **forced, see §5**

`Anchor::NotInForm` with the two real exits, and the `NotInForm` **count guard** extended by one with
its justification. Both edits were compelled by existing drift guards, not chosen.

### The user-facing text, printed from the test (rendered as `NOT COMPUTABLE [reason]: ` + this detail)

Over the threshold:

> this return is for tax year 2026, it ITEMIZES, and §68 — the overall limitation on itemized
> deductions — applies to it. Schedule A line 18 asks, in the form's own words: "Is the amount on
> Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and 13b of that form, more than
> $384,350?" For this return that amount is $384,351 (your adjusted gross income, less your
> qualified-business-income deduction, less your Schedule 1-A additional deductions), which is MORE.
> The form's answer to a Yes is "Your deductions may be limited. See the Itemized Deductions
> Worksheet in the instructions to figure the amount to enter" — and btctax does not have that
> worksheet. The tax year 2026 Form 1040 and Schedule A instructions have not been published, so
> there is no document to transcribe it from, and btctax will not invent one. Printing the unlimited
> total on line 18 instead would deduct more than §68 allows and UNDERSTATE your tax. What to do:
> work the Itemized Deductions Worksheet in the tax year 2026 Schedule A instructions by hand, enter
> its result on Schedule A line 18, and file on paper — or take this return to a paid preparer. No
> btctax command clears this and no further answer will change it. A return that takes the standard
> deduction is unaffected and computes normally, because §68 reduces only itemized deductions.

A §68 year whose Schedule A has never been transcribed (the fail-closed arm):

> this return is for tax year 2027 and it ITEMIZES. §68 — the overall limitation on itemized
> deductions — applies to every tax year beginning after December 31, 2025 (Pub. L. 119-21 §70111),
> and no Schedule A for tax year 2027 has been transcribed into btctax, so it does not know the
> threshold that year's form prints. That threshold is the dollar amount at which the 37% bracket
> begins, which is re-indexed every year, so it cannot be carried forward from an earlier form.
> btctax therefore cannot tell whether your deductions are limited, and it will not file an unlimited
> itemized total on a year §68 limits — that would deduct more than allowed and UNDERSTATE your tax.
> What to do: work Schedule A and its Itemized Deductions Worksheet for tax year 2027 by hand and
> file on paper, or take this return to a paid preparer. A return that takes the standard deduction is
> unaffected and computes normally, because §68 reduces only itemized deductions.

---

## 2. THE YEAR SCOPE IS DERIVED FROM THE EXTRACTS — no year list

`section_68_tests::archived_schedule_a()` reads `design/forms/extract/` with `std::fs::read_dir`,
parsing `f1040sa--YYYY[-DRAFT].txt` — the FR-197 shape already in
`return_refuse.rs::archived_w2_instructions`. Archiving `f1040sa--2027.txt` pulls a new revision into
every §68 assertion with **no edit to any source file**. It asserts at least 3 revisions were found,
so a broken glob is loud rather than vacuously green.

`prints_section_68_gate()` is a **shared** `pub(crate)` reading of the forms, used by the
classification test *and* by `a_pre_2026_itemizer_over_the_threshold_files_unchanged`, so "which
archived Schedule A prints the gate" has exactly one reader.

Three-way classification, per archived revision: a revision that prints the gate **must** be `Gated`;
one that does not **must** be `NotApplicable` — not merely "not `Gated`". §4-F3 explains why that
distinction is the whole test.

---

## 3. B1 — TEN PLANTS, TEN REDS

Baseline before every plant: `11 tests run: 11 passed, 790 skipped`. Restored and re-verified clean
after every plant and at the end (`11 tests run: 11 passed`).

**Correction to my own harness's labelling:** it printed "COMPILE ERROR" whenever stderr began with
`error`, and `error: test run failed` is nextest's normal non-zero exit line. **No plant below
produced a compile error**; every red is a test failure. (One *earlier* draft of the P5 plant was
genuinely invalid Rust and was rewritten — see §4-F3.)

| plant | what it breaks | RED |
|---|---|---|
| **P1** the gate removed from `screen_absolute` | nothing holds it | `return_1040::tests::a_ty2026_itemizer_over_the_section_68_threshold_is_refused_and_one_under_it_files` — 10 passed, **1 failed** |
| **P2** the tested quantity becomes AGI alone (both subtrahends dropped) | the semantic quantity | `the_gate_reads_agi_less_the_qbi_and_schedule_1a_deductions_not_agi_alone` — 10 passed, **1 failed** |
| **P3** `threshold: dec!(640600)` — the Single 37% start | **FR-186's trap** | `the_gate_sentence_and_threshold_are_the_forms_own`, `the_threshold_is_the_lowest_37_percent_start_not_the_filers_own`, `the_refusal_quotes_the_form_and_names_only_remedies_that_exist` — 8 passed, **3 failed** |
| **P4** the *"more than"* boundary flipped to *at or more than* | the at-threshold row | `a_ty2026_itemizer_over_the_threshold_refuses_and_one_a_dollar_under_it_does_not` — 10 passed, **1 failed** |
| **P5** `ThresholdNotTranscribed` returns `None` — an untranscribed §68 year files unlimited | the fail-closed arm | `a_section_68_year_with_no_transcribed_schedule_a_refuses_every_itemizer`, `the_refusal_quotes_the_form_and_names_only_remedies_that_exist` — 9 passed, **2 failed** |
| **P6** the gate extended to TY2025 *and* the boundary moved back so the arm is reachable | gating a year the form does not gate | `a_pre_2026_itemizer_over_the_threshold_files_unchanged`, `the_gate_sentence_and_threshold_are_the_forms_own`, `the_gated_years_are_read_off_the_archived_schedule_a_extracts` — 8 passed, **3 failed** |
| **P7** `SECTION_68_FIRST_YEAR = 2025` | §68 applied a year early, refusing every TY2025 itemizer | `a_pre_2026_itemizer_over_the_threshold_files_unchanged`, `the_gated_years_are_read_off_the_archived_schedule_a_extracts` — 9 passed, **2 failed** |
| **P8** the `deduction_is_itemized` guard dropped | standard-deduction returns refused | `a_standard_deduction_return_is_untouched_at_any_income` — 10 passed, **1 failed** |
| **P9** the detail offers a btctax verb that cannot clear it | a remedy that does not exist | `the_refusal_quotes_the_form_and_names_only_remedies_that_exist` — 10 passed, **1 failed** |
| **P10** `SECTION_68_FIRST_YEAR = 2027` | TY2026 files unlimited | **10 of 11 failed** (every test but `a_section_68_year_with_no_transcribed_schedule_a_refuses_every_itemizer`) |

**Plus a standing in-suite kill, so the citation checker is watched discriminating on every run:**
`tables::section_68_tests::a_paraphrase_and_a_wrong_threshold_are_both_rejected` accepts the real
sentence (2 contiguous runs across the layout's one interruption) and rejects four plants —
`$640,600` (FR-186's trap), `$38,435` (a lost digit, the Form 6251 line-33 class), `line 11b` becoming
`line 11` (an unverifiable 1040 renumbering), and *"more than"* becoming *"greater than"* (a
paraphrase). The answer to *"which test reds when the checker is removed?"* is that one, and it lives
in the suite rather than in this report.

### The 11 new tests

    tax::return_1040::tests::a_ty2026_itemizer_over_the_section_68_threshold_is_refused_and_one_under_it_files
    tax::return_refuse::section_68_gate_tests::a_pre_2026_itemizer_over_the_threshold_files_unchanged
    tax::return_refuse::section_68_gate_tests::a_section_68_year_with_no_transcribed_schedule_a_refuses_every_itemizer
    tax::return_refuse::section_68_gate_tests::a_standard_deduction_return_is_untouched_at_any_income
    tax::return_refuse::section_68_gate_tests::a_ty2026_itemizer_over_the_threshold_refuses_and_one_a_dollar_under_it_does_not
    tax::return_refuse::section_68_gate_tests::the_gate_reads_agi_less_the_qbi_and_schedule_1a_deductions_not_agi_alone
    tax::return_refuse::section_68_gate_tests::the_refusal_quotes_the_form_and_names_only_remedies_that_exist
    tax::return_refuse::section_68_gate_tests::the_threshold_is_the_lowest_37_percent_start_not_the_filers_own
    tax::tables::section_68_tests::a_paraphrase_and_a_wrong_threshold_are_both_rejected
    tax::tables::section_68_tests::the_gate_sentence_and_threshold_are_the_forms_own
    tax::tables::section_68_tests::the_gated_years_are_read_off_the_archived_schedule_a_extracts

`the_gate_reads_agi_less_the_qbi_and_schedule_1a_deductions_not_agi_alone` is the one worth reading:
AGI is $60,000 **over** the threshold and the two deductions remove $60,001, so the form's own
quantity lands a dollar **under** and the return FILES — while **all three** ways of getting the
quantity wrong (AGI alone, dropping 13a, dropping 13b) refuse that same return.

---

## 4. FINDINGS

### F1 — the form's single threshold is the MINIMUM over statuses, and the statute is per-status. The screen is therefore deliberately over-inclusive. (Sharpens the brief; recorded in the source.)

§68(a)(2), as rewritten by §70111(a), measures against *"the dollar amount at which the 37 percent rate
bracket under section 1 begins with respect to the taxpayer"* — a **per-status** figure. The TY2026
starts are Single **$640,600**, MFJ **$768,700**, HoH **$640,600**, MFS **$384,350**
(`tax_tables.rs::ty2026`). The form's gate prints **$384,350** for everyone: the **minimum**, asserted
by `the_threshold_is_the_lowest_37_percent_start_not_the_filers_own`.

So the printed screen **screens in every filer who could be limited** and leaves the worksheet to apply
the taxpayer's own bracket start — which is exactly why the form says *"may be limited"*. The
consequence for this refusal, stated plainly: **a Single filer between $384,350 and $640,600 is refused
although §68 would reduce nothing for them.** That over-refusal declines to file a return, which is
recoverable; FR-186's "tidy-up" would file a deduction that is too large, which is not. **This is the
statutory reason the brief's trap is a trap**, and it is now written where the next editor meets it.

### F2 — a precision on *"today that is entirely silent"*: the gap is DORMANT, not live, and the guard is placed ahead of the port.

Measured, and asserted rather than claimed (`a_ty2026_itemizer_over_the_section_68_threshold...` opens
with it): **no TY2026 return can be assembled at all today.** `return_1040.rs::form6251_line1_rule` has
arms for **2024 and 2025 only** and `form6251_inputs_from_parts`'s `unwrap_or_else` **panics** rather
than file TY2024's Part I under a TY2026 heading; and `BundledFullReturnTables::load` inserts **2024
alone**, so `full_return_for(2026)` is `None`. This matches `design/TY2026_PORT_REPORT.md:454`'s own
classification, **"DORMANT / IMPORTANT"**.

The understatement becomes live the moment the port transcribes Form 6251 Part I and bundles the
package — which is precisely why the guard exists before that happens, and why the end-to-end test
assembles at 2024 and **screens at 2026**: `screen_absolute` takes `year` as its own parameter, so that
is the real code path with the real year, and the gate reads only that year plus three quantities that
mean the same thing in every year.

### F3 — MY OWN INSTRUMENT WAS BLIND, and the plant pass is what found it. Two defects, both the "derive the list" disease inside my tests.

The first plant run had **two survivors**, and one of them was real.

**P7 (`SECTION_68_FIRST_YEAR = 2025`) SURVIVED.** Two independent causes, both mine:

1. `the_gated_years_are_read_off_the_archived_schedule_a_extracts` compared "is it `Gated`" against the
   form — a **two-way** check. With the constant at 2025, TY2025 became `ThresholdNotTranscribed`,
   which is *"not Gated"* and passed — **while refusing every TY2025 itemizer.** Fixed to the full
   three-way classification.
2. `a_pre_2026_itemizer_over_the_threshold_files_unchanged` looped
   `[2017, 2024, SECTION_68_FIRST_YEAR - 1]`. **The third element moved with the mutation** to `2024`,
   so TY2025 was never tested at all. Fixed: the ungated set is now read off the extracts, with an
   assertion that TY2024 is in it and that the set has at least two members, so a shrinking set is loud.

The statutory boundary is now asserted against **the statute's own date** (the constant must equal
2026, with §70111(c) quoted) rather than against itself.

**P6 was a plant that planted nothing** — adding a `2025` arm to the match is unreachable below the
early return for years before the first year. Rewritten to move the boundary too; it now reds three
tests.

This is the corollary in `CLAUDE.md` — *"the same disease infects fixtures ... the thing that decides
was not the thing that knows"* — and I reproduced it inside a test written to guard against it. The
reasoning is recorded in the test's own doc comment, so the next reader meets the evidence rather than
the conclusion.

### F4 — recommended FOLLOWUP: the import tier does not see this gate.

`screen_param_free` (the `income import` tier) cannot reach it, because the gate needs AGI and
`screen_inputs` has no computed return. So `income import` will accept and store a TY2026 itemizing row
in silence, and the refusal arrives at `report` / export. That is **consistent with every other
computed-quantity gate** — the Form 4952, §170(f)(8) and acquisition-debt gates all live in
`screen_absolute` for the identical reason, and `ScreenTier`'s own doc scopes the import tier to what a
params-less importer can honestly run — so I did **not** widen it. Worth a follow-up only if the
controller wants a year-level warning at import time. **Not a defect of this parcel.**

### F5 — recommended FOLLOWUP: the printed chain still has no Schedule A line 18.

`printed.rs::schedule_a_lines` carries the TY2025 line set and `AbsoluteReturn::itemized_deduction` is
still documented *"Schedule A line 17"* (`ty2026_rehearsal.rs` §3 pins both). Nothing prints wrongly
today, because the refusal stops a TY2026 packet being emitted at all — but the port still owes the
line-set change, and **the refusal must not be removed before line 18 exists**. Out of scope here
(`printed.rs` is neither parcel's file).

### F6 — I did not touch `FOLLOWUPS.md`.

F1 / F4 / F5 are the entries I would file. The brief says do not commit, and `FOLLOWUPS.md` is named in
neither parcel's ownership, so I left it to the controller rather than race parcel M for it.

---

## 5. OWNERSHIP — two files outside the stated list, both COMPELLED

The brief granted `return_refuse.rs` and the threshold's home. Two further files had to change, and in
both cases an **existing drift guard** demanded it — neither was a choice:

1. **`crates/btctax-core/src/tax/return_1040.rs`** — `screen_absolute` is the only site holding the
   computed quantities, and `return_refuse.rs` never sees `AbsoluteReturn` (a grep for that type in
   that file returns 0 hits). The decision and the whole argument stay in `return_refuse.rs`; the call
   site is 8 lines. This is the `donation_restriction_gate` / `screen_broker_reporting` shape already in
   the file. The end-to-end B1 row sits next to the call site.
2. **`crates/btctax-input-form/src/attribute.rs`** — adding a `RefuseReason` variant **E0004**'d its
   deliberately wildcard-free `attribute()` match (the spec-§7 drift guard, working exactly as
   designed), and placing the anchor then reded the `NotInForm` **count guard** (16 to 17). Both
   extended with the justification those guards ask for.

**Neither is `interview_state.rs`, `year_readiness.rs` nor `cmd/answer.rs`** — no collision with parcel
M. The short working-tree status lists exactly four modified files and nothing else.

---

## 6. GATE

`make check` excludes `cargo fmt`, and it runs nextest and clippy **concurrently** in the default target
dir. Per the brief I ran `make gate`'s semantics — touching every `*.rs` under `crates/` first (FR-90:
the plant-restore loop races cargo's mtime freshness) — with nextest and clippy **serially**, in my own
target dirs, and `cargo fmt --all --check` as the third leg. Captured once to files, then grepped.

    find crates -name '*.rs' -exec touch {} +

    CARGO_TARGET_DIR=<wt>/target-n cargo nextest run --workspace --no-fail-fast
    NEXTEST EXIT=0
         Summary [  34.899s] 3781 tests run: 3781 passed, 12 skipped

    CARGO_TARGET_DIR=<wt>/target-n-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    CLIPPY EXIT=0
    lines matching ^(warning|error): 0
    "Checking btctax-core" lines: 1      # confirmed it really re-checked the edited crate

    cargo fmt --all --check
    FMT EXIT=0                           # 0 bytes of output

All 11 new tests are in that run; the earlier full run recorded them at indices 1406, 1543-1554 and
1730-1735 of 3781. `rustfmt` was applied once and its diff touched only my four files — 1050
insertions, 4 deletions, and the 4 deletions are the `attribute.rs` count-guard lines:

     crates/btctax-core/src/tax/return_1040.rs   | 136 ++++++++
     crates/btctax-core/src/tax/return_refuse.rs | 465 ++++++++++++++++++++++++++++
     crates/btctax-core/src/tax/tables.rs        | 423 +++++++++++++++++++++++++
     crates/btctax-input-form/src/attribute.rs   |  30 +-
     4 files changed, 1050 insertions(+), 4 deletions(-)

**Not committed. Not pushed.** Working tree: four modified files, nothing else.
