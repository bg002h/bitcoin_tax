# Seam review — interview build T9 (real estate, R8)

Reviewer: independent build reviewer, own worktree at `18935bfd` (`main`).
Worktree: `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a9954f54b59f045e5` — **clean at review end**;
every plant was reverted from a `cp` backup and `git status --short` is empty.
Brief: `design/agent-reports/BRIEF-review-interview-T9.md`. Artifact under review:
`design/agent-reports/2026-09-07-build-interview-T9-implementation.md`.

---

## Commands

All cargo runs used `CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review`, scoped.

```
cargo nextest run --locked -p btctax-core -E 'test(ZZ_probe_standard_deduction_8396)' --no-capture
cargo nextest run --locked -p btctax-core -E 'test(ZZ_probe_std_ded_1098_rules)'      --no-capture
cargo nextest run --locked -p btctax-core -E 'test(ZZ_probe_seeded_row_on_a_standard_deduction_year)' --no-capture
cargo nextest run --locked -p btctax-core                       # with the candidate fix applied
cargo nextest run --locked -p btctax-oracle-harness             # 5 passed, 1 skipped
cargo nextest run --locked -p btctax-forms                      # baseline 364 passed, 4 skipped
cargo nextest run --locked -p btctax-forms                      # with the map plant  -> 2 FAILED
cargo nextest run --locked -p btctax-forms -E 'test(zz_probe_see_attached)' --no-capture
cargo run -q -p xtask -- dump-fields crates/btctax-forms/forms/2024/f1040sa.pdf
cargo run -q -p xtask -- dump-fields crates/btctax-forms/forms/2025/f1040sa.pdf
```

Not re-measured (settled by the controller in the brief): `make check`, `line-coverage`,
`census-join`, `stop-list`, `prompt-check`, `box-census`, and the TY2024 `f1_17`/`f1_18` geometry.
I did re-run `dump-fields` on **both** Schedule A blanks because seam 4 covers the TY2025 twins as
well, and the controller's settled measurement named only TY2024.

---

## Summary

**One Critical.** The itemize-election conjunct is present on the three older mortgage declarations
and on the census row, and **absent from all three rules T9 added**. A standard-deduction filer who
transcribes a Form 1098 — which the always-live `Forms 1098` section invites them to do — is asked
the Form 8396 question and is **refused** by `SharedMortgageInterestUnanswered`, by
`MortgageInterestCreditUnanswered`, and (on a truthful *yes*) by `SharedMortgageInterest` and
`MortgageInterestCreditUnsupported`, over a Schedule A line 8a their return does not have. This is
the exact scenario the brief's one question names and the exact scenario spec journey **J-24** says
must not happen, and `open_next_year` seeds precisely the shape that trips it.

**One Important.** When there are more line-8b recipients than the year's form has dotted lines, the
emitter prints the instruction's *"See attached"* — but nothing anywhere produces or asks for the
statement, and no `hand_marks` entry names it. On **TY2025 two recipients is enough**. The branch has
no test.

Both adjudication questions are answered in the build's favour: **D-1's geometry is right and the
spec is wrong** (independently re-measured, both years), and **D-2's fail-closed `None` refusal is
correct** — subject to C-1's gate.

---

## C-1 (Critical) — a standard-deduction filer is asked and refused over a mortgage they do not deduct

**Where.**
- `crates/btctax-core/src/tax/questions.rs:2512-2518` — `mortgage_interest_credit_question_live`
- `crates/btctax-core/src/tax/return_refuse.rs:3198` and `:3225-3255` — the per-row Form 1098 loop's
  shared-interest arms
- `crates/btctax-tui-edit/src/edit/form.rs:791-808` — `section_is_live`

**What is wrong.** `mortgage_question_live` (the three older declarations) and
`document_census::row_is_live(Form1098)` both carry `schedule_a.is_some()`. The three rules T9 added
do not:

```rust
fn mortgage_interest_credit_question_live(ri: &ReturnInputs) -> bool {
    !ri.form_1098.is_empty()
        || ri.schedule_a.as_ref()
             .is_some_and(|a| !a.mortgage_interest_not_on_1098.is_empty())
}
```

and the shared-interest gate is an unconditional `for (i, m) in ri.form_1098.iter().enumerate()`.

The doc comment above `mortgage_interest_credit_question_live` states the premise that makes the
omission look safe, and **the premise is false**:

> ★ It does NOT read `schedule_a.is_some()` on its own: the 1098 SECTION is already gated on the
> itemize election, so a row can only exist on an itemizing return […]

`SectionId::Form1098s` is not gated anywhere. `section_is_live` handles `W2Box12`,
`ScheduleACharitable`, `Spouse`, `QbiLimitation` and `BrokerReporting`, and everything else — the
1098 section included — falls into `_ => true`. Its 13 `Field`s are all `live: |_| true`. So the
section is offered to every filer, which is what the build's own §1 says it should be ("top-level:
the document arrives whether or not the filer itemizes").

Spec §R8's rationale for the conjunct is exactly this scenario, and journey **J-24**
(`design/SPEC_interview.md:1164`) states the guarantee: *"takes the standard deduction and holds a
$900k 2019 1098 | the census row and the three declarations are not live; nothing is transcribed and
**nothing refuses**"*.

**Evidence.** Probe added to `return_refuse.rs`'s test module (reverted), building the spec's own
J-24 fixture — `ri()` with a $900,000 / 2019 Form 1098 and `schedule_a: None`:

```
PROBE-A live(ClaimingMortgageInterestCredit) = true
PROBE-A 8396 unanswered   -> Some(MortgageInterestCreditUnanswered)
PROBE-A 8396 = Some(true) -> Some(MortgageInterestCreditUnsupported)
PROBE-B shared-interest blank      -> Some(SharedMortgageInterestUnanswered)
PROBE-B shared-interest Some(true) -> Some(SharedMortgageInterest)
PROBE-C ceiling warning on a standard-deduction return -> is_some = true
```

The shipped test `a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing`
does not see any of it, for two reasons that compound: it enumerates only the **three older**
declarations (`return_refuse.rs:3840-3844`), and its fixture pre-answers both new gates —
`ri()` sets `claiming_mortgage_interest_credit = Some(false)` (`:3658`) and the fixture literal sets
`other_borrower_paid_interest: Some(false)` (`:3853`). Its `assert_eq!(reason(&standard), None)` is
therefore true of a return in which both gates were already answered, not of the filer it names.

**This is reachable without the filer doing anything.** `open_next_year` seeds a prior lender as an
identity with every box `Default` (`open_next_year_t4b.rs:1567-1618`), and it carries **no**
`schedule_a` — `grep -n schedule_a crates/btctax-cli/src/open_next_year.rs` returns nothing. A filer
who itemized last year and takes the standard deduction this year opens the year into exactly that
shape. Second probe (reverted), constructing the seeded row verbatim:

```
PROBE seeded row, standard deduction -> Some(SharedMortgageInterestUnanswered)
PROBE detail: the Form 1098 from Home Savings does not say whether someone other than your spouse
also paid interest on that mortgage. btctax adds box 1 to Schedule A line 8a IN FULL, and the
instruction allows that only for interest that was yours ("you can only deduct your share of the
interest"), so a blank here is not a "no" — it is the whole deduction claimed on your behalf. Set
`other_borrower_paid_interest` on the row (in the tax-inputs editor, or in the Form 1098 table of
your import file)
```

The message names a Schedule A line 8a that this return does not have, on a row whose box 1 is $0.

**Minimal change.**

1. `mortgage_interest_credit_question_live` →
   `ri.schedule_a.as_ref().is_some_and(|a| !ri.form_1098.is_empty() || !a.mortgage_interest_not_on_1098.is_empty())`,
   and delete the false premise from its doc comment.
2. In `screen_param_free`'s Form 1098 loop, skip the shared-interest arms when
   `ri.schedule_a.is_none()`. **Leave the box-4 refusal ungated** — that refund is income on
   Schedule 1 line 8z, which is owed whether or not the filer itemizes, so it is correctly
   unconditional.
3. Extend `a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing` so it
   is the kill for what it claims: assert `!question_is_live(ClaimingMortgageInterestCredit)` on the
   standard-deduction half, and assert `reason(&standard) == None` with the shared-interest gate at
   `None` and at `Some(true)` and with `claiming_mortgage_interest_credit` at `None` and at
   `Some(true)`. As shipped, the fixture pre-answers both, so no mutation of either rule can red it.

I applied (1) and (2) in my worktree and ran `cargo nextest run --locked -p btctax-core`: **nothing
in that crate depends on the un-gated behaviour.** The only reds came from a *third*, optional edit I
made at the same time (gating the ceiling warning, M-1 below): `the_acquisition_debt_warning_*`
×3, whose fixtures carry no `schedule_a`.

**Note for the fold — the spec disagrees with itself here and should be settled in the same pass.**
`SPEC_interview.md` R8 says both *"top-level — it arrives whether or not the filer itemizes"* and
*"the `form_1098` census row **and the 1098 section** are live iff `schedule_a.is_some()`"*, and
separately says the 8396 gate is *"live iff any 1098 or 8b row"*. The build followed the first and
third and dropped the second. Gating the two rules (above) satisfies R8's stated **rationale** and
J-24 while keeping the document top-level; if instead the section itself is gated, the section
becomes invisible to a standard-deduction filer and R8's first sentence needs rewording. Pick one and
say so in R8.

---

## I-1 (Important) — line 8b prints *"See attached"* for a statement nothing produces, asks for, or names

**Where.** `crates/btctax-forms/src/schedule_a.rs:102-131` (`SEE_ATTACHED`, the `fits` branch);
`crates/btctax-cli/src/cmd/admin.rs:449-506` (`hand_marks`).

**What is wrong.** When `lines.line8b_payee.len() > map.line8b_payee.len()`, the emitter replaces the
recipients' identities with the single string `"See attached"`. That is the instruction's own escape
(`i1040sca--2025.txt:1126-1131`) — but the instruction's sentence is *"identify the person by
**attaching a statement to your paper return** and printing "See attached""*. btctax prints the
second half and does nothing about the first:

- no statement is generated;
- `hand_marks` has no entry for it (`grep -rn "See attached\|SEE_ATTACHED" crates/` returns
  `schedule_a.rs` only) — so the packet manifest's *"COMPLETE BY HAND"* block, which the manifest
  itself frames as the closed list of marks btctax deliberately did not make, does not mention it;
- no advisory, no transcription warning, nothing on stderr.

So the filer signs, under §6065, a Schedule A that asserts a document is attached which they were
never told to write. Under this repo's own standing rule (*an entry is testimony*), that is
fabricated testimony on a printed line, and the instruction prices the omission it causes at the
same $50 penalty the 8b-identity refusal quotes.

**The trigger is closer than the map header suggests.** TY2025 merged TY2024's two single dotted rows
into ONE 24pt box, and `line8b_payee` for 2025 has one entry — so **two** recipients overflow, on a
cell the IRS sized for two lines.

**And the branch has no test.** `full_return_forms.rs:1124` exercises one recipient only, and
asserts the second dotted line stays blank; nothing exercises `fits == false`. Per B1, the branch has
never been observed doing anything.

**Evidence.** Probe appended to `crates/btctax-forms/tests/full_return_forms.rs` (reverted), driving
the real `fill_schedule_a`:

```
PROBE 2024 two payees,   f1_17 = Some("JANE SELLER, 000-00-0000, 1 MAIN ST")
PROBE 2024 two payees,   f1_18 = Some("JOHN SELLER, 000-00-0001, 2 OAK AVE")
PROBE 2025 two payees,   payee box = Some("See attached")
PROBE 2025 two payees,   8b amount = Some("900")
PROBE 2024 three payees, f1_17 = Some("See attached")
PROBE 2024 three payees, f1_18 = None
```

**Minimal change.** Add a `hand_marks` entry conditioned on the overflow actually having occurred
(the same "conditioned on the mark being blank in THIS packet" rule the function's own doc comment
states), naming line 8b and listing the recipients whose identities did not fit; and add a test that
drives `fill_schedule_a` with `len(payee) > len(map.line8b_payee)` on both years, asserting both the
printed `"See attached"` and the manifest item. The build already filed this as D-6/"Not done"; it is
Important rather than a follow-up because the printed page makes an assertion the filer never made
and is never told about.

---

## M-1 (Minor) — the ceiling warning fires for a standard-deduction filer, and points at a question they are never asked

**Where.** `crates/btctax-core/src/tax/transcription_warnings.rs:341-373` (early return at `:346`).

**What is wrong.** Same root cause as C-1: the guard is `ri.form_1098.is_empty()`, not the itemize
election. A standard-deduction filer holding a $900,000 1098 gets, in `btctax report` and in the
tax-inputs editor, a warning ending *"Check the figure against the limit before you answer 'were you
inside EVERY home-mortgage debt limit this year?'"* — and `MortgageWithinDebtLimit` is **not live**
for them, so that question is never asked. Seam 1 asks for "nothing printed from it".

Minor rather than blocking because the warning changes nothing, writes nothing, and says so.

**Evidence.** `PROBE-C ceiling warning on a standard-deduction return -> is_some = true` (above).

**Minimal change.** `if ri.form_1098.is_empty() || ri.schedule_a.is_none() { return None; }`, and
give the three `the_acquisition_debt_warning_*` fixtures a `schedule_a: Some(ScheduleAInputs::default())`
(they are the only reds; I confirmed this by applying the change and running `-p btctax-core`).

---

## M-2 (Minor) — the home-sale table test is 8 + 2 branches, not the 16 the brief and the report describe

**Where.** `crates/btctax-core/src/tax/return_refuse.rs:4033-4108`.

**What is wrong.** The build report and the brief both describe the census dimension as fully
crossed. The test crosses the three tests (8 rows) at `s_1099 = Some(false)` only, then adds
`Some(true)` and `None` **on the blank branch alone** — 10 of the 24 combinations. The gap is benign
(every refusing test-triple refuses on its own arm before the 1099-S conjunct is reached, and the
census screens first anyway), but the described coverage is not the delivered coverage.

**Evidence.** The loop is `for t1 in [true,false] { for t2 … { for exclude … } } }` over `base(...)`,
which leaves `documents` at `ri()`'s all-`Some(false)`; the two extra assertions after the loop both
start from `base(true, true, true)`.

**Minimal change.** Either lift the 1099-S state into the loop (24 rows, still instant), or correct
the sentence in the report and the test's doc comment to say what is actually crossed.

---

## N-1 (Nit) — the ceiling warning is the only warning in its module that does not use the module's money formatter

`transcription_warnings.rs:362-372` interpolates `${total}` and `${limit}` directly, so the filer
reads *"adds up to $900000, which is more than the $750000"*. Every other warning in the file goes
through `money()` (`:312`). Use `money()` for consistency, or better, add thousands separators —
`$900000` is genuinely hard to read at a glance and this is the one warning whose whole content is a
comparison of two large numbers.

---

## Verdicts on the two adjudication questions

### D-1 — the TY2024 8b/8c cell pairing: **the build is right, the spec is wrong.**

I re-measured both blanks with `xtask dump-fields` (the controller settled TY2024; TY2025 was not in
the settled set, so seam 4's "TY2025 twins" needed its own measurement):

| year | cell | x | y | what it is |
|---|---|---|---|---|
| 2024 | `f1_16` | 417.6–488.9 | 408–420 | line 8a |
| 2024 | `f1_19` | 417.6–488.9 | 360–372 | **line 8b AMOUNT** |
| 2024 | `f1_17` | **115.2–396.0** | 348–360 | **8b dotted line 1** |
| 2024 | `f1_18` | **115.2–396.0** | 336–348 | **8b dotted line 2** |
| 2024 | `f1_20` | 417.6–488.9 | 312–324 | **line 8c** (no description cell exists) |
| 2024 | `f1_21` | 417.6–488.9 | 300–312 | line 8d (reserved) |
| 2024 | `f1_22` | 417.6–488.9 | 288–300 | line 8e |
| 2025 | `f1_15` | 410.4–481.6 | 366–378 | line 8a |
| 2025 | `f1_17` | 410.4–481.6 | 324–336 | **line 8b AMOUNT** |
| 2025 | `Line8b_ReadOrder[0].f1_16` | **122.4–381.6** | 300–324 | **8b's single 24pt dotted box** |
| 2025 | `f1_18` | 410.4–481.6 | 276–288 | **line 8c** |
| 2025 | `f1_20` | 410.4–481.6 | 252–264 | line 8e |

`f1_17` and `f1_18` are the same width and the same x-band, both wide free-text, both **below** the
8b amount; `f1_20` sits in the money column with nothing wide beside it. The form's own text
confirms it: `f1040sa--2024.txt:40-48` prints four caption lines for 8b ending *"and address"*
followed by two blank rows, then `c Points not reported to you on Form 1098. See instructions for
special rules . . . 8c` with no description slot. **The committed map is correct; SPEC R8's
sentence** — *"the four TY2024 map cells `f1_17`/`f1_19` […] **8c** […] (`f1_18`/`f1_20`)"* — **is
wrong and should be replaced** with: *8b = `f1_19` (amount) with `f1_17` and `f1_18` as its two
dotted lines; 8c = `f1_20`, which has no description cell on either form; TY2025 merges the two
dotted rows into one 24pt box, `Line8b_ReadOrder[0].f1_16`.*

I also confirmed the pairing is **held by a kill, not just by a comment.** Plant: point TY2024
`line8b_payee[0]` at `f1_13` (line 6's write-in description, whose x-centre 255.6 is inside the new
`COL_PAYEE` band, so the geometric verifier alone cannot separate them):

```
Summary [1.044s] 161/364 tests run: 159 passed, 2 failed, 4 skipped
  FAIL btctax-forms::full_return_forms schedule_a_fills_the_printed_chain_and_reads_back
  FAIL btctax-forms::field_census census_accounts_for_every_field

panicked at crates/btctax-forms/tests/full_return_forms.rs:1169:5:
assertion `left == right` failed: L8b's dotted line must carry the recipient's name, identifying
number and address
  left: None
 right: Some("JANE SELLER, 000-00-0000, 1 MAIN ST")
```

Reverted. The read-back test pins the fqn, so the weak discrimination of the new `COL_PAYEE` cluster
(which `f1_1` and `f1_13`/`f1_12` also fall inside) is not exploitable — no finding there.

### D-2 — `other_borrower_paid_interest` refusing on `None`: **correct, keep it.**

The instruction is *"If you and at least one other person … were liable for and paid interest on a
mortgage that was your home, you can only deduct your share"* (`i1040sca--2025.txt:1071-1080`), and
line 8a sums box 1 **in full**. Reading a blank as "no co-borrower" is btctax answering an
understatement-direction question on the filer's behalf — the class this repo has ruled on repeatedly
(*blank is the normal case*: a blank is no testimony, and only the filer may forgo). It does **not**
refuse "a filer who simply has no co-borrower": it refuses until they say *no*, and the exit is one
tri-state field on the row itself (`sections.rs:2522-2556`, `FieldKind::TriState`, `clear` writes
`None`), with help that says in terms what a blank means. That is an ask, not a wall — the same shape
as every other class-(A) declaration. The Form 1099-B line-1a precedent the build cites is exact.

**Two conditions on keeping it**, both from C-1: (a) it must carry the itemize-election conjunct, or
it refuses filers who have no line 8a; and (b) its message must not assert *"btctax adds box 1 to
Schedule A line 8a IN FULL"* on a return with no Schedule A — with (a) in place, it never will.

---

## Seams checked clean

- **Seam 2 — the ceiling arithmetic.** The four figures match the instruction's own sentences
  (`i1040sca--2025.txt:1026-1035` for $1,000,000/$500,000, `:1036-1046` for $750,000/$375,000) and are
  identical on the TY2024 bundle and the TY2026 constants function
  (`btctax-adapters/src/tax_tables.rs:145-150`, `:280-285`). `for_loan` takes the grandfathered branch
  only for `origination <= 2017-12-15` and halves for MFS on both branches; `None` takes the stricter
  post-2017 limit. The aggregate-vs-`min` design is **exactly** right, not merely conservative: with
  pre-2018 debt `P` and post-2017 debt `Q`, the instruction's reduction rule makes the true ceiling
  `max(P, 750k)`, and `total > max(P, 750k)` iff `P + Q > 750k` whenever a post-2017 row exists — which
  is the only case where `min` differs from the true limit. The warning writes nothing (asserted in
  `the_ceiling_warning_answers_nothing_and_waits_for_the_years_package`), is printed **before** the
  prompt and the stored `prompt_hash` is the registry's words, driven end-to-end through
  `answer_return_inputs` in `tax_report.rs:3409-3495`.
- **Seam 3 — box 4 and the import tier.** `a_1098_box_4_refund_refuses_at_import_naming_line_8z_and_writes_no_row`
  (`year_gate_t4.rs:75-125`) drives the real `cmd::tax::import_return_inputs`, asserts the message
  names *line 8z* and *REFUND OF OVERPAID INTEREST*, asserts `show_return_inputs` is `None`
  afterwards, and asserts the zero-refund twin **imports**. Box 4's ungated-by-itemizing placement is
  right: the refund is Schedule 1 line 8z income.
- **Seam 4 — the 8e chain and the sweep.** `f1040sa--2024.txt:50` prints *"e Add lines 8a through
  8c"*; `line8e = line8a + line8b + line8c` and the printed test pins 12,700 + 940 + 300. Line 8a =
  Σ(box 1 + box 6) matches both the line caption and `i1040sca--2025.txt:1060-1061`, and box 6 is
  *"Points paid on purchase of principal residence"* on both archived editions. Map cells verified by
  measurement (table above) for **both** years. `cargo nextest run -p btctax-oracle-harness`:
  5 passed, 1 skipped, including `check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells`.
- **Seam 5 — the home sale.** `HomeSale` carries four `Option<bool>` and **no `Usd`**
  (`return_inputs.rs:441-461`). `the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523`
  calls `reason`/`screen_param_free` directly (not a re-implementation), asserts `blanks == 1`, and
  asserts *PUB. 523* and *code H* in every refusing detail. Both extra 1099-S dimensions are asserted
  on the blank branch, so a reordering that lost one is visible. Nothing on the blank branch is
  printed: there is no `HomeSale` cell on any map. (Coverage gap recorded as M-2.)
- **Seam 6 — the census, the editions and the opener.**
  `the_1098_entries_red_on_a_deleted_box_a_drifted_caption_and_a_narrowed_edition` drives the **real**
  eleven `BOXES` entries against **both** real extracts and takes all three reds as `expect_err`, so
  the test passing *is* the red having been observed. The brief's suggested plant ("a caption only the
  other edition prints") is not constructible for this document — both editions print the identical
  eleven captions — and the build's substitute (narrow an entry's edition list, assert it reds on the
  edition it no longer covers) tests the same property; I accept the substitution.
  `a_form_1098_lender_is_seeded_as_an_identity_with_every_box_blank` compares the seeded row against
  the identity-only literal rather than a hand-list of boxes, so a box added later is covered the day
  it is added, and asserts the census row itself stays `None`.
- **Box 5 / line 8d.** Collected with no reader for TY2024/25 is correct, not a held-figure defect:
  §163(h)(3)(E) is terminated for those years, the form prints *"Reserved for future use"*, and the
  field help and census note both say so. Nothing is forgone, so no advisory is owed.
- **Deviations D-3, D-4, D-5, D-7, D-8, D-9, D-10** — all read and accepted as stated. D-4's ordering
  claim is verified in the test (both paths asserted). D-10 is consistent with the repo's own rule
  that statute-and-Rev.-Proc. constants settle now, and `full_return_for(2026)` is still `None`, so
  no TY2026 return can see them.

---

Counts: C=1 I=1 M=2 N=1
