# FR-1 — 1040 line 19: the child tax credit btctax does not figure

**Built 2026-09-04.** Branch `fix/fr1-ctc-line19`, from `feat/schedule-1a-ty2025` (32d593b3).
Not merged, not pushed. `make check` green: **2767 tests run, 2767 passed, 12 skipped**;
`cargo fmt --all --check` clean.

---

## 1. What the form actually requires, before any code

Read from the text layer, not the rendered page.

**Form 1040 line 19** (`design/forms/extract/f1040--2025.txt:112`):

```
19   Child tax credit or credit for other dependents from Schedule 8812
20   Amount from Schedule 3, line 8
21   Add lines 19 and 20
22   Subtract line 21 from line 18. If zero or less, enter -0-
```

Note what lines 19 and 20 do **not** carry: any "-0-" clause. Lines 15 and 22 have one; 19 does not.
It is a **carry** — the census already classifies it `Production::Carry`, *"blank when the source line
is blank"* — and its source is Schedule 8812 line 14.

**Schedule 8812 Part I** (`design/forms/extract/f1040s8--2024.txt:36-44`):

```
12   Is the amount on line 8 more than the amount on line 11?
       No.  STOP. You cannot take the child tax credit, credit for other dependents, or additional
            child tax credit. Skip Parts II-A and II-B. Enter -0- on lines 14 and 27.
       Yes. Subtract line 11 from line 8. Enter the result.
14   Enter the smaller of line 12 or line 13. This is your child tax credit and credit for other
     dependents … Enter this amount on Form 1040, 1040-SR, or 1040-NR, line 19.
```

So the three cases the brief asked me to state:

| case | what the form requires on 1040 L19 | why |
|---|---|---|
| **credit owed** | **BLANK** | L19 carries Sch 8812 L14. btctax emits no 8812 and figures no §24 credit, so there is no source figure. Nothing on the line instructs a zero. |
| **provably zero** | **`0`** | Sch 8812 L12 answers *No* ⇒ L14 is `-0-` by its own instruction ⇒ L14 says *"Enter this amount on Form 1040 … line 19"*. The form asks for the figure; blanking it would be the mirror defect. |
| **not determinable** | **BLANK** | Same as case 1 — neither branch of L12 is established. |

FR-1's written diagnosis was **correct on all three points** (`line19: Option<Usd>`;
`ctc_provably_zero` private and needing `ReturnInputs` + dependents + AGI; 21/22 must treat blank as
the form does). Both of its ★ warnings were also correct and are honoured rather than worked around.

### One thing the diagnosis did not name: `dependents == 0` is not a proof

`HouseholdHeader::dependents` is a `#[serde(default)] Vec<Dependent>`
(`crates/btctax-core/src/tax/return_inputs.rs:252`). **Empty and never-asked are the same value.**

That matters because `ctc_provably_zero` computes the line-8 CEILING as `dependents × $2,000` — and on
an unasked list that is not a ceiling at all. Concretely: a childless-looking Single filer at
$300,000 AGI gets L11 = $5,000 and L8-ceiling = $0, so the predicate returns `true` — but if that
filer actually has three children they never entered, the true ceiling is $6,000 and the credit is
**not** gone. The existing advisory is safe only because it gates `if dependents > 0` before calling.

So the printed lane applies the **same** gate. `dependents == 0` ⇒ blank, never a sworn `-0-`. This is
fail-closed in both directions and is what makes the all-zero return's line 19 blank.

---

## 2. The change

**One predicate, in `btctax-core`, read by both lanes.**

`crates/btctax-core/src/tax/advisories.rs` — new `pub fn ctc_odc_line19(ri, agi) -> Option<Usd>`,
wrapping the existing (still private) `ctc_provably_zero` with the `dependents > 0` gate. The
`CtcOdcOmitted` advisory now sets `provably_zero: ctc_odc_line19(ri, agi).is_some()` instead of
calling `ctc_provably_zero` beside it — equivalent by construction (that arm already gates
`dependents > 0`), and it means the cell and the advice cannot contradict each other on one packet.

`crates/btctax-core/src/tax/return_1040.rs` — `PrintedInputs` gains `ctc_odc_line19: Option<Usd>`,
populated in `assemble_absolute_return` where `ri` and `agi` meet, with the *same* `agi`
(`ar.agi`) that `advisories_for` passes. This is the `form_8960_line9b` pattern, verbatim:
*"Carried rather than re-read so the printed chain and the absolute … cannot end up on different
values; `None` reaches `push_money_opt` and the cell prints BLANK."*

`crates/btctax-core/src/tax/printed.rs` — `Form1040Lines::line19` is `Option<Usd>`, carried from
`ar.printed_inputs`. `line21 = line19.unwrap_or(ZERO) + line20`; line 22 untouched.

`crates/btctax-forms/src/form1040_full.rs` — the page-2 amount array becomes
`[(&MoneyCell, Option<Usd>); 13]` and the loop calls `push_money_opt`. **`btctax-forms` contains no
§24(b) reasoning of any kind.** Line 19 stays *in* the array rather than moving to a conditional push,
so its descent ordinal is stable — `verify_flat` checks per-group ordinal-y descent, and hoisting an
optional line out of the sequence would renumber every line below it on the blank return and not on
the phased-out one.

**No tax figure moves.** Line 21 and line 22 are byte-identical for every household; total tax is
unchanged. The fix removes an assertion, it does not change an amount. (The return still overstates
tax for a credit-eligible family — btctax cannot figure §24 — but it now says so instead of swearing
to it. `CtcOdcOmitted` carries the news, and its text was corrected: it used to end
*"(1040 line 19 is $0)"*, which was describing the defect.)

### Blast radius the compiler found (free, and exactly the point)

`Form1040Lines` literals: `line_coverage.rs`, `printed.rs`, `return_1040.rs`,
`btctax-forms/tests/{extract_lines,full_return_forms}.rs`. `PrintedInputs` literal: `printed.rs`.
`Coverage::line`'s `_value: Usd` widened to `_value: impl Into<Option<Usd>>` — accepts both (std's
`impl<T> From<T> for Option<T>`), so the census keeps its `deny(unused_variables)` consumption
guarantee without an `unwrap_or` that would read like a claim about the figure.

---

## 3. Test-first, and mutation-verified

**The failing test was written first** and was red on exactly the FR-1 defect before any fix:

```
thread 'form_1040_line_19_is_blank_unless_schedule_8812_provably_says_minus_zero' panicked at
crates/btctax-forms/tests/attestation.rs:650:5:
1040 line 19 must be BLANK for a family whose child tax credit btctax never figured. … Paper: Some("0")
```

New KAT (`crates/btctax-forms/tests/attestation.rs`), **two households, read off the emitted PDF** —
the two sides of Schedule 8812 line 12:

* Single, $60,000 wages, **2 dependents** ⇒ line 19 **absent** from the paper.
* MFJ, $2,085,000 wages, **9 dependents** ⇒ L9 $400,000, L10 $1,685,000, L11 $84,250, L8-ceiling
  $18,000 ⇒ L12 *No* ⇒ line 19 prints **`0`**.

Two existing tests also became FR-1 assertions rather than being weakened:

* `the_all_zero_return_files_one_form_whose_every_money_line_is_zero_or_blank` — `("line19", "0")` is
  removed from `ALL_ZERO_1040_PAPER` and replaced by an explicit `!contains_key("line19")` with the
  reason. That table is an **equality**, so a row in it is what makes a printed zero mandatory; the
  row was the bug, not the omission. The doc comment now says so, next to the existing 34/35a note.
* `form_1040_full_fills_every_line_and_reads_back` — asserts line 19's cell is empty for the `None`
  fixture and line 21 still prints, then fills a second PDF with `line19 = Some(0)` and asserts it
  prints `0`. Without the second leg, an unconditional blank would pass the first.

### Mutation verification (cp backups; `git checkout --` never used)

| mutation | tests red |
|---|---|
| `ctc_odc_line19` ⇒ `Some(Usd::ZERO)` (the shipped hardcode) | **5** — the new KAT, the all-zero KAT, `advisories::tests::omissions_fire_together_in_order`, `return_1040::tests::the_lift_moves_no_printed_line`, `examples_golden_matches_committed` |
| `ctc_odc_line19` ⇒ `None` (the unconditional blank FR-1 rejected) | **2** — the new KAT's second household, `nine_dependents_scenario::the_ctc_advisory_tells_this_filer_the_credit_is_gone_not_that_it_is_owed` |
| emitter: `push_money_opt` ⇒ `push_money(value.unwrap_or(ZERO))` | **3** — the new KAT, the all-zero KAT, `form_1040_full_fills_every_line_and_reads_back` |

Both files were restored from `cp` backups; `grep -rn "MUTATION [123]" crates/` returns nothing.

### Documentation corrected in the same pass (each was a live falsehood after the fix)

* `crates/btctax-cli/LIMITATIONS.md` CTC row — said *"Either way 1040 line 19 is **$0**."*
* `crates/btctax-core/src/tax/advisories.rs` — the `CtcOdcOmitted` general-case text.
* `crates/btctax-forms/forms/2024/f1040.map.toml:69` — comment said *"always 0 (conservative omission)"*.
* `docs/examples/examples.md` — regenerated. **Diffed before installing** (the
  "a golden cannot validate its own regeneration" trap): the regeneration changed exactly the three
  lines of that one advisory paragraph and nothing else. `no_worked_example_shows_a_command_that_errored`
  stays green.

`FOLLOWUPS.md` FR-1 is marked ✅ CLOSED with the reasoning above.

---

## 4. Adjacent defects found — NOT fixed, per the brief

* **FR-12 — Form 8960 line 9d still prints `0` when 9b is blank.** Confirmed same class, still open.
  Note the shape is *easier* than FR-1: `PrintedInputs::form_8960_line9b` is already `Option<Usd>`, so
  there is no predicate to expose — only a derived total that must propagate the blank.
* **1040 line 20 has the same latent question.** `line20 = sch_3.map_or(Usd::ZERO, |s| s.line8)`
  (`printed.rs`): a return with no Schedule 3 prints a sworn `0` on *"Amount from Schedule 3, line 8"*,
  a carry from a schedule that was never filed. Identical shape to FR-1, not in FR-1's scope, and not
  touched. Worth filing.
* **Sub-dollar input skew, judged not worth a change.** Schedule 8812 line 1 is *"the amount from line
  11 of your Form 1040"* — the **printed** line 11 — while this predicate is fed `ar.agi` (exact
  cents), because that is what `advisories_for` passes and agreement between the cell and the advisory
  matters more. The two can differ by under $1, which moves L11 by at most $50; it could flip the
  answer only if the L8 ceiling landed inside that $50 window. Recorded rather than fixed: changing the
  input for one lane only would reintroduce exactly the divergence this design removes.

## 5. What a reviewer should look at first

1. **Is `dependents == 0 ⇒ None` right?** It is the one judgment call that is not transcription, it
   changes the all-zero KAT's paper, and the argument rests on `dependents` being a `#[serde(default)]
   `Vec` with no answered-ness marker. If that reasoning is wrong, the all-zero return should print `0`.
2. **The advisory rewiring** (`provably_zero: ctc_odc_line19(...).is_some()`) — claimed equivalent
   because the arm already gates `dependents > 0`. One `if` to check.
3. **The descent-ordinal argument** in `form1040_full.rs` — that keeping line 19 in the array with a
   `None` value is what preserves `verify_flat`'s per-group ordinal check across both branches.
