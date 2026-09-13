# REPORT — wave 2, parcel E. FR-196: the §111(a) State and Local Income Tax Refund Worksheet.

**Status: BUILT AND GREEN.** New module `crates/btctax-core/src/tax/state_local_refund.rs` (2,173 lines,
32 tests), the collection surface on `ReturnInputs`, and the compiler-forced consequences.
**The refusal is NOT removed** — `return_refuse.rs` is parcel F's, so §7 below is the exact removal.

**Gate, measured. All three legs run in the FOREGROUND, after `find crates -name '*.rs' -exec touch {} +`:**

| leg | command | result |
|---|---|---|
| suite | `cargo nextest run --workspace --no-fail-fast` | **3722 passed, 0 failed, 12 skipped** (baseline at start of parcel: 3690/3690) |
| lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **exit 0**, zero error/warning lines |
| fmt | `cargo fmt --all --check` | **exit 0**, zero bytes of output |

`make check` was NOT used for the reported run, and the reason is worth passing to the other parcels: it
runs nextest and clippy **concurrently**, and on this box (24 GB of 62 GB already in use by sibling
agents) that OOM-killed rustc compiling `btctax-forms` test `broker_boxes` with `signal: 9, SIGKILL`.
The two halves were run serially in separate target dirs (`target-e`, `target-e-clippy`). Same coverage,
no memory race. **A SIGKILL out of `make check` here is memory pressure, not a code defect** — and it is
not FR-176's stale-rlib link error either.

Not committed, not pushed. No subagents.

---

## 1. Premise check — three confirmations, one correction, one material scoping fact

**CONFIRMED — the worksheet is in `i1040gi`.** `design/forms/extract/i1040gi--2025.txt:42061` and
`i1040gi--2024.txt:41290`, both titled `State and Local Income Tax Refund Worksheet—Schedule 1, Line 1`.
Nine numbered lines in both, **derived from the extract**, never hand-counted.

**CONFIRMED — the four inputs the refusal names are real.** Last year's Schedule A (lines 5d, 5e, 17); its
SALT cap (line 5e *is* the §164(b)(6) limitation as that year's Schedule A applied it); the standard
deduction that could have been taken (line 5, plus line 6's §63(f) boxes).

**CONFIRMED — a prior-year return in the vault cannot be the route.** See §3.

**★ CORRECTION — the refusal's requirement list is INCOMPLETE.** The worksheet needs **five** further
things, and two of them are exits that make the refund **non-taxable**:

1. **The prior-year FILING STATUS.** Lines 5, 6 and the MFS Note all read *"the filing status claimed on
   your Y−1 Form 1040 or 1040-SR"* — not this year's. Nothing on `ReturnInputs` held it.
2. **The TIP's limb (b) — the §164(b)(5) SALES-TAX ELECTION in the year the tax was paid.** *"None of your
   refund is taxable if, in the year you paid the tax, you either (a) didn't itemize deductions, **or (b)
   elected to deduct state and local general sales taxes instead of state and local income taxes**"*
   (`i1040gi--2025.txt:41882-41887`). btctax asks limb (a) — `itemized_prior_year` — and has **never asked
   limb (b)**. That filer deducted no income tax at all, owes nothing under §111(a), and is refused today
   with nothing to compute. One bool closes it.
3. **The NINE Pub. 525 Exception conditions.** The worksheet's own first instruction is *"Before you begin:
   Be sure you have read the Exception in the instructions for this line to see if you can use this
   worksheet instead of Pub. 525"*, and under any of nine conditions it may **not** be used.
4. **Line 6's four §63(f) checkboxes for the PRIOR year**, plus the footnote's three MFS conditions. Not
   derivable from this year's `AgedBlindBoxes`: the aged label is a birth-date cutoff that moves every
   revision (`January 2, 1959` vs `1960`), and blindness is a point-in-time status *"at the end of"* a year
   btctax holds no record of.
5. **The refund amount itself, for a filer with no Form 1099-G.** `state_refund_without_1099g` is a BOOLEAN
   with no amount beside it, while line 1 reads *"from Form(s) 1099-G **(or similar statement)**"* and the
   line-1 instructions say *"Report any taxable refund you received even if you didn't receive Form
   1099-G"* (`i1040gi--2025.txt:41897-41898`). That filer declared a refund whose size was **unstatable**.
   The refusal was the only thing hiding it.

**★★ MATERIAL SCOPING FACT, not a refutation.** The worksheet is **per revision**, and each revision's
lines 5 and 6 print the **prior year's** standard-deduction constants. Only `i1040gi--2024.txt` and
`i1040gi--2025.txt` are archived, so this lands **TY2024 and TY2025**. The owner's first *filed* year is
**TY2026**, whose `i1040gi` does not exist until the January-2027 finals — so TY2026 refuses on
`NotUsable::NoArchivedRevision` until that revision is fetched and transcribed, **by design rather than by
omission** (it does not borrow a neighbouring year's figures). The structure lands now; the constants
arrive with the finals, and a test reds the day the archive appears.

## 2. What was transcribed, from the TEXT LAYER, per revision

Two `Revision` constants (`REVISION_2024`, `REVISION_2025`), each carrying, verbatim: the nine numbered
line instructions; `before_you_begin`; line 2's two branches; line 3's Yes branch; the STOP text; the MFS
Note; line 5's three bullets with their printed amounts; line 6's four checkbox labels, its `No boxes
checked. Enter -0-.`, its multiply sentence and its MFS footnote; line 8's Yes branch; the TIP; the
Exception header; and all **nine** Exception conditions.

**★ Two cross-reference facts, both exactly the Form 6251 line-33 class, read off the text layer and
recorded rather than adjudicated:**

- **Line 1's cap is `line 5d`, verbatim** — *"the amount of your state and local income taxes shown on your
  Y−1 Schedule A, **line 5d**"* — and 5d is the 5a+5b+5c TOTAL, not the income-tax component alone. The
  text says 5d; the module says 5d. It is also the conservative direction: a larger cap can only make line
  1, and hence the taxable part, **larger**.
- **Exception 7 cites 1040 `line 18` in the 2025 revision and `line 16` in the 2024 one** — the same
  sentence, a different line, because the 1040 renumbered underneath it. A single shared quote would have
  been wrong for one of the two, which is the concrete reason the table is per revision. The revisions also
  differ in apostrophe glyph (`couldn't` is ASCII in 2024, U+2019 in 2025), and both are byte-checked
  against their own extract.

Ten numeric constants transcribed and cross-checked against the sentence that prints them:
`$13,850 / $27,700 / $20,800` + `$1,500 / $1,850` (2024) and `$14,600 / $29,200 / $21,900` +
`$1,550 / $1,950` (2025). The 2025 per-box pair matches `btctax-adapters::tax_tables::ty2024_full_return`'s
`std_aged_blind_married` / `_unmarried` — an independent witness, pinned by a test.

**The nine Exceptions are modelled, not excused.** `Pub525Exception` with `ALL`, an `_`-free match in
`number()`, and an `_`-free match over `ALL` in `exception_that_applies()`. Any affirmed condition is a
REFUSAL (`NotUsable::Pub525Exception`), because btctax models no part of Pub. 525's *Itemized Deduction
Recoveries*. A tenth condition on a future revision does not compile until both matches name it, **and the
set itself is read off the extract**.

**Skips are skips, never `-0-`.** `Option<Usd>` for line 2 on its own *No* branch, lines 5–7 on the MFS
Note, and everything after either STOP. `Decision::schedule_1_line1()` returns `Option<Usd>`: `None` is a
line the form says not to fill in, `Some(0)` a computed zero, and the type refuses to collapse them.

## 3. Route chosen: COLLECT, not carry — and why

**Collect.** Three reasons, ascending:

1. `open_next_year`'s own stated rule is *"**Never a carried amount**"* — only the §1212(b) carryforwards
   cross a year boundary, read from year N's frozen RETURN — and `open_next_year_t4b.rs:1787` pins
   `grep -c schedule_a crates/btctax-cli/src/open_next_year.rs` at **0**.
2. Decisively: a refund received in year *Y* is of tax paid in *Y−1*, and **the first year btctax files is
   by construction a year whose predecessor it did not file.** A carry-only design would leave exactly the
   filer FR-196 names — the owner, in TY2026, whose TY2025 return was prepared elsewhere — still walled.
3. The form itself instructs a transcription: *"the amount **reported on** your Y−1 Schedule A, line 5d"* is
   a figure the filer reads off a piece of paper they are holding. `Source::FilerRecords`, the provenance
   Schedule A's own lines already carry. (No new `DocumentKind`: nobody issues a prior-year 1040 to anybody,
   and inventing one would add a census row and a `transcribed_on` column to a document with no issuer.)

**The carry is not foreclosed.** `StateLocalRefundFacts::provenance: CarryProvenance` is present from the
first commit, so when a prior-year btctax return does exist the same fields can be seeded and stamped
`ComputedFromPriorReturn { year }` — a carried figure is then never the same bytes as a typed one (§G-23's
"stated zero"). Two of the nine exceptions — 3 (*zero rate on preferential income*) and 6 (*AMT owed*) —
are computable from a prior-year `AbsoluteReturn`, and their field docs name that mechanism rather than
pretending it is unknowable.

## 4. The input surface added

`ReturnInputs::state_local_refund: Option<StateLocalRefundFacts>`, `#[serde(default)]`. **`None` = never
collected**, which is where every existing vault is, and which the worksheet refuses on
(`NotUsable::FactsNotCollected`).

**★★ Answered-ness is structural at the BLOCK, and that is the design choice most worth reviewing.** Inside
a present block **no field carries `#[serde(default)]`** — a TOML omitting one refuses to parse, and the
regenerated `docs/income-import-schema.md` now lists all 22 as `**required**`. So there is no `false` for an
unanswered exception declaration to hide in, and no thirteen new `FORM_QUESTIONS` rows for a registry to
remember. The classifier records them as `Class::SerdeRequired`, which is §2.8's own name for exactly this.
A test plants the omission and watches serde name the missing field.

**Fail-closed on the MFS spouse boxes.** Line 6's footnote permits a prior-year MFS filer to check the
SPOUSE boxes only if three conditions hold. Line 6 raises line 7, line 7 is subtracted from line 4 — so an
over-claimed box makes the taxable part **smaller**, i.e. understates. The three conditions are one
conjoined affirmation (`mfs_spouse_boxes_permitted`), and unless it is affirmed the spouse boxes do not
count. Same posture and same cited reasoning as `packet::AgedBlindBoxes` for this year's boxes.

## 5. The instruments, and what each was observed RED on (B1)

32 tests, all in-module — so `crates/xtask/` (parcel G's) is untouched. The `#[cfg(test)]` block reads
`design/forms/extract/` through `CARGO_MANIFEST_DIR`; the precedent is `return_refuse.rs::tests::repo_root`,
and keeping it in `cfg(test)` avoids the `include_str!`-out-of-crate-root publishing trap.

| instrument | derived from | the kill that reds it |
|---|---|---|
| verbatim **and whole**, every quote, per revision | the extract, whitespace-normalised only | `a_paraphrase_is_rejected` ("itemised"); `a_truncation_is_rejected` (line 1 cut at its first sentence); `a_wrong_line_cross_reference_is_rejected` (line 17 → line 12); `a_quote_from_the_wrong_revision_is_rejected` (TY2024's line 4 against the TY2025 extract) |
| the LINE set | standalone `N.` physical lines in the worksheet block | `dropping_a_line_reds_the_completeness_check` (`should_panic`, calls the real assertion) |
| the EXCEPTION set | `^N. [A-Z]` physical lines in the Exception block | `dropping_an_exception_reds_the_completeness_check` (`should_panic`) |
| the REVISION set | the extract **directory** | `an_untranscribed_archived_revision_reds` (`should_panic`) |
| line 6's two per-box amounts, in printed order | the multiply sentence | `swapping_line6s_per_box_amounts_reds` (`should_panic`) |
| every `FilingStatus` mapped by exactly one line-5 bullet | the type | the count assertion itself |
| every one of the nine exceptions actually refuses | `Pub525Exception::ALL` | the walk demands a refusal per variant, so no variant can be inert |

Each `should_panic` kill calls the **same** assertion helper the real test calls (`assert_line_numbers`,
`assert_exception_numbers`, `assert_revisions_transcribed`) with a gutted set, so what is observed is the
real check firing — not two sets merely differing.

**★★ The two set-derivations were observed RED on real defects mid-build, not only on synthetic plants.**
The first version of both read tokens out of the whitespace-normalised blob, which made line 4's *"line
17."* and TY2024 exception 7's *"line 16."* look like worksheet lines 17 and 16 — the checks red with
`right: {1,…,9,17}` and `{1,…,9,16}`. The two sets have **different shapes** (a worksheet label is a
standalone physical line; an exception number *starts* a line while a cross-reference *ends* one), so they
cannot share a predicate. That is recorded in the source, because pretending they could is how it broke.

**★ The weak terminator's blind spot is measured, not asserted.**
`no_sentence_end_quote_hides_an_internal_full_stop` enumerates every `SentenceEnd`-terminated quote carrying
an internal full stop — the ones a truncation could land on — and reds if any quote outside the named set
does.

**The three B1 vectors the brief asked for, plus six more.** Every figure below hand-checked line by line
against the transcribed 2025 revision:

- **fully taxable** — Single, $900 refund, TY2024 5d = 5e = $8,000, line 17 $20,000 ⇒ L2 blank, L3 900,
  L5 14,600, L6 -0-, L7 14,600, L8 5,400, **L9 = $900**.
- **partial benefit** — MFJ, $3,000 refund, line 17 $30,000 against a $29,200 standard deduction ⇒ L8 =
  **$800**; L9 = min(3,000, 800) = **$800**. $2,200 of the refund is not income. *This is the vector a
  closed form gets wrong in both directions: `taxable = refund` overstates by $2,200, `taxable = 0`
  understates by $800.*
- **standard deduction last year ⇒ NONE taxable, and the line comes out BLANK, not zero** —
  `schedule_1_line1() == None`, asserted explicitly against `Some(0)`. Also asserted with the facts block
  PRESENT: the **answer** decides, not the block's existence.
- plus: the sales-tax-election exit (also blank); the line-3 STOP (the SALT cap had already disallowed more
  than the whole refund); the line-8 STOP (itemizing bought nothing); line 1's cap binding; line 6's boxes
  erasing the benefit entirely (2 boxes × $1,550 pushes L7 to $32,300 > $30,000 ⇒ STOP); the MFS Note
  skipping lines 5–7 — **and the same filer without the Note's condition reaching the line-8 STOP instead**,
  so the Note is shown to be worth $500 of income rather than cosmetic; and the no-1099-G refund reaching
  line 1 and then being capped by line 1's own limit (adding before capping, not after).

## 6. Compiler-forced blast radius — and the two findings inside it

One new field on `ReturnInputs` produced **four compile errors and four red tests**, every one a totality
check doing its job.

| file | why | forced by |
|---|---|---|
| `tax/mod.rs`, `tax/state_local_refund.rs` | the module | — |
| `tax/return_inputs.rs` | the field + the `Default` literal | E0063 |
| `tax/classifier.rs` | `_`-free destructure; 18 leaves classified `SerdeRequired` / `DataDerived` | E0027 |
| `tax/scrub.rs` | PII partition — no identity in the block, and no jurisdiction either (the worksheet asks for amounts, never for which state) | E0027 |
| `tax/scrub_axis.rs` | `maximal_sentinel` realizes the block (an absent `Option` contributes no money leaf, so a prefix would match nothing) | E0063 |
| `tax/return_refuse.rs` | `first_negative_amount` — **four money leaves negative-screened**; a negative line 5e would make L2 larger than 5d and understate | E0027 |
| `tax/provenance.rs` | one `LEAF_SOURCE` prefix, `FilerRecords` | red KAT |
| `tax/testonly.rs` | one `ORACLE_INVISIBLE` entry, `PriorYearCarryIn` | red KAT |
| `btctax-input-form/src/spec/coverage.rs` | one `EXEMPT_LEAVES` entry | red KAT |
| `btctax-cli/src/cmd/tax.rs` | the import provenance normalisation — **finding (a)** | NOT forced; found by reading |
| `docs/income-import-schema.md`, `docs/examples/examples.md` | generated; regenerated with their own generators | red KATs |

★ Several of these sit outside the parcel's stated ownership. **None belongs to another wave-2 parcel**, and
every one but `cmd/tax.rs` is a mechanical consequence the compiler or a KAT demanded. The single edit
inside parcel F's `return_refuse.rs` is a **destructure plus a negative screen** — no refusal added, none
removed, none re-worded.

### (a) FINDING — a generated doc asserted a guarantee the code did not keep. Important.

`cmd::tax::import_return_inputs` forces every `CarryProvenance` to `User` on import, because *"`Computed` IS
BTCTAX'S SIGNATURE, AND THE IMPORT SURFACE MUST NOT BE ABLE TO SIGN IT"* — three surfaces act on that stamp
(`m4_authority` goes silent, `BenefitCarryoversNotStated` stops asking, the write-back's `--force` guard
stops protecting). It does so with a **hand-written list of five sites**, whose own comment reads *"THE WHOLE
CLASS, not the one field the review named."* My field is the **sixth**, and nothing red — while
`xtask::toml_schema::note_for` **derives** its *"forced to `user`"* annotation from the path suffix, so the
regenerated schema doc asserted `state_local_refund.provenance` was normalised at the same moment the code
did not touch it. A hand-written TOML could have stamped `computed_from_prior_return` on figures nobody
derived.

**Closed inline** (one `if let` in `cmd/tax.rs`, with the class named in the comment). **The class defect is
open**: the normalisation is a LIST where the schema generator is a DERIVATION, and no test joins them.
*Recommended fix:* walk every `CarryProvenance` leaf (the `leaf_walk` machinery exists) instead of listing
five — or at minimum, a test that every path `note_for` annotates as forced actually is. This is the
highest-value residue of the parcel and it is not FR-196's to fix.

### (b) FINDING — the input-form coverage prefix ratchet is AT its ceiling. Minor, and by design.

`EXEMPT_PREFIXES.len() <= EXEMPT_PREFIX_CEILING = 5`, and it holds exactly 5. So FR-196's block could not be
exempted by prefix without moving a pin whose own comment says the only direction that hides a leaf is
**up**. Resolved with an `EXEMPT_LEAVES` entry instead — **narrower**, not a workaround: it matches only
while the census fixture leaves the block `None`, and the day the fixture populates it, 22 leaf paths appear
and the census demands a `Field` for each. The exemption cites the `schedule_1a` precedent verbatim
(*"prompt wording is the deliverable here, not plumbing"*) and names its own removal condition.

### (c) Observation, not a finding — `g_1099[].box2_state_refund` has no money reader today

Grepped: box 2 appears only in `transcription_warnings` (a display sum) and the census. The figure that
actually reaches Schedule 1 line 1 is the hand-attested `sch1.state_refund_taxable`
(`return_1040.rs:1369, 1710, 2315, 2369, 3072`), whose doc says *"user attests; §111 worksheet not
modeled"*. So a transcribed box 2 is a typed number with no reader, masked entirely by the refusal.
**This matters for the wiring: the reader already exists.** And Tax-Calculator's `e00700` is *"Taxable
refunds of state and local income taxes"* (verified in the installed `taxcalc/records_variables.json`), so
the worksheet's OUTPUT is oracle-checkable even though `GoldenInputs` has no field for it yet.

---

## 7. THE EXACT REFUSAL REMOVAL — the final recommendation, for parcel F or the next round

**Do not delete `RefuseReason::StateAndLocalRefundWorksheetNotComputed`.** It must survive, NARROWED, for
three states the worksheet genuinely cannot answer. Replace the *condition*, not the variant.

**Site:** `crates/btctax-core/src/tax/return_refuse.rs:3634-3661` at this worktree's HEAD — the block opening
`if crate::tax::questions::question_is_live(QuestionId::ItemizedPriorYear, ri) && ri.itemized_prior_year == Some(true)`.
(It was at 3592 before this parcel; my `first_negative_amount` addition shifted it by 49 lines. The variant
declaration is at 1549; the two in-file tests are at 5777 and 6139; the reachability entry at 10459.)

Replace the body with a call to the worksheet, refusing only on its own refusals:

```text
if question_is_live(QuestionId::ItemizedPriorYear, ri) && ri.itemized_prior_year == Some(true) {
    let refund_1099g = ri.g_1099.iter().map(|g| g.box2_state_refund).sum();
    match state_local_refund::figure(
        ri.tax_year, ri.itemized_prior_year, refund_1099g, ri.state_local_refund.as_ref(),
    ) {
        Ok(_) => {}                                     // the worksheet decided; NO refusal
        Err(NotUsable::FactsNotCollected)         => StateAndLocalRefundWorksheetNotComputed
        Err(NotUsable::NoArchivedRevision { .. }) => StateAndLocalRefundWorksheetNotComputed
        Err(NotUsable::NoLine5AmountForStatus(_)) => StateAndLocalRefundWorksheetNotComputed
        Err(NotUsable::Pub525Exception(e))        => a NEW reason, see (1)
    }
}
```

**Five things the removal must carry with it, in order of importance:**

1. **A NEW `RefuseReason` for the Pub. 525 exceptions** — `Pub525ItemizedDeductionRecovery(Pub525Exception)`
   — not a reuse of the worksheet reason. They are different facts with different remedies: *"btctax does
   not compute this worksheet"* versus *"the form FORBIDS this worksheet for you; Pub. 525's Itemized
   Deduction Recoveries governs."* The detail should quote that revision's own sentence via
   `Revision::exception_text`, so the filer reads back the condition they affirmed. Needs a `verdict_reach`
   entry and a `btctax-input-form::attribute` mapping.
2. **Keep the existing reachability fixture unchanged.** `return_refuse.rs:10459` sets
   `state_refund_without_1099g = Some(true)` and `itemized_prior_year = Some(true)`, which under the new
   rule still refuses — via `FactsNotCollected`. No edit needed; worth checking rather than assuming.
3. **Two testimonies about one line must REFUSE.** `sch1.state_refund_taxable` is the hand-attested Schedule
   1 line 1, and the worksheet now computes the same line. A present `state_local_refund` block **and** a
   non-zero `sch1.state_refund_taxable` is a contradiction btctax cannot adjudicate — refuse it, the way
   `FilerRecordsContradicted` refuses its analogue. Do **not** silently prefer one: that is
   `a-figure-with-no-reader` in its two-chains form, where the correct instrument is the comparison.
4. **Wire `Decision::schedule_1_line1()` in, preserving the `Option`.** `None` must print a BLANK, not `0`.
   `return_1040.rs` currently sums a bare `Usd` into Schedule 1 line 1, so that is the one place the
   blank/zero distinction can be lost on the way to paper (§G-11).
5. **A non-zero `refund_not_on_a_1099g` beside `state_refund_without_1099g == Some(false)`** is the same
   contradiction one question over, and should refuse too.

**What the removal buys, concretely.** An itemizer in an income-tax state files at all. And **two
populations refused today turn out to owe nothing**, so they file with a correctly **blank** Schedule 1
line 1 instead of going to a preparer: the sales-tax-election filer (TIP limb (b), never asked before), and
anyone whose prior-year itemized total did not clear the standard deduction they could have taken instead
(the line-8 STOP).

## 8. Follow-ups to file

- **FR-196a (Important). Owning phase: NOW.** The `CarryProvenance` import normalisation is a five-item list
  standing against a derived schema annotation, with no test joining them; the sixth site was silently
  unnormalised and the generated doc claimed otherwise. Fix by walking the leaves, or pin the agreement.
  §6(a) has the reproduction.
- **FR-196b. Owning phase: the input-form section.** `state_local_refund` is exempt from the input-form
  coverage census. ~14 declarations and four figures, and the prompt wording is the deliverable — each of
  the nine exceptions must state the condition that permits a YES (*"You owed alternative minimum tax in
  2024"* is answerable only off last year's return). Removing the exemption is the census's own instruction.
- **FR-196c. Owning phase: the TY2026 port / January-2027 finals.** `i1040gi--2026.txt` must be fetched and
  transcribed before TY2026 can work this worksheet. `every_archived_revision_is_transcribed` reds the day
  the archive lands, so it cannot be forgotten — but it is on the critical path for the owner's first filed
  year and the port runbook should say so explicitly.
- **FR-196d. Owning phase: with the wiring.** `GoldenInputs` has no taxable-state-refund field, so no corpus
  household can exercise Schedule 1 line 1 against either oracle. taxcalc's `e00700` exists (verified);
  OTS's `S1_1` is already named in `ORACLE_INVISIBLE`. Adding the axis is what makes the worksheet's output
  double-oracle-checked rather than only self-consistent.
- **FR-196e (Minor, observation). Owning phase: with §7(3).** A non-zero `sch1.state_refund_taxable` beside
  `itemized_prior_year == Some(false)` is unscreened today. Arguably lawful (a Pub. 525 recovery from
  another year), but it is the same two-testimonies shape and deserves a decision rather than a silence.

## 9. Files changed

New: `crates/btctax-core/src/tax/state_local_refund.rs` (2,173 lines, 32 tests).
Modified (`git diff --stat`, 364 insertions / 2 deletions across 12 files):
`btctax-cli/src/cmd/tax.rs`, `btctax-core/src/tax/{classifier,mod,provenance,return_inputs,return_refuse,scrub,scrub_axis,testonly}.rs`,
`btctax-input-form/src/spec/coverage.rs`, `docs/income-import-schema.md`, `docs/examples/examples.md`
(the last two regenerated by their own generators, not hand-edited).
