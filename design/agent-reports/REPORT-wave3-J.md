# REPORT — wave 3, parcel J. The state-refund path, end to end: FR-221, FR-196's refusal removal, FR-222, FR-196a.

**Status: BUILT AND GREEN. The wall is down.** An itemizer in an income-tax state who supplies the
prior-year figures now FILES, with the worksheet's own figure on Schedule 1 line 1; a filer who
elected the §164(b)(5) general sales tax in the prior year files with that line **blank**, and needs
none of the worksheet's inputs to get there.

**Gate, measured. Three legs, all FOREGROUND, nextest and clippy run SERIALLY in separate target dirs
(`target-j`, `target-j-clippy`) per FR-223 — no SIGKILL, no stale-rlib link error:**

| leg | command | result |
|---|---|---|
| suite | `cargo nextest run --workspace --no-fail-fast` | **3738 passed, 0 failed, 12 skipped** |
| lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **exit 0**, zero error/warning lines |
| fmt | `cargo fmt --all --check` | **exit 0**, zero bytes of output |

**Diffstat: 24 files, 1,564 insertions / 167 deletions.** Not committed, not pushed. No subagents.

---

## 1. ★★★ ONE PREMISE REFUTED — E's §7(5) recipe was UNREACHABLE as written

E's §7 obligation (5) says: *"A non-zero `refund_not_on_a_1099g` beside `state_refund_without_1099g
== Some(false)` is the same contradiction one question over, and should refuse too."* The rule is
right; **the place the recipe puts it cannot fire.**

`state_refund_without_1099g` is live **only** when the Form 1099-G census row says *"I received
none"* — and that forbids a transcribed box 2 (a row beside a `No` is `DocumentCensusContradicted`).
So on a return where that question is live and answered `No`, the §111(a) gate
(`QuestionId::ItemizedPriorYear`) can be live **only through that same question being answered
`Yes`**. Inside the `gate_live` + `itemized_prior_year == Some(true)` branch the two conditions are
mutually exclusive: the rule would have shipped as a refusal that cannot refuse — *"a gate that
cannot fail"*, the shape this repo names as its own dominant defect.

**Resolved by relocating it, not by dropping it.** The rule now sits **ahead of** the gate, requiring
only (a) the question live, (b) answered `Some(false)`, (c) a non-zero `refund_not_on_a_1099g`. That
state is reachable (census row `No`, door answered `No`, a facts block carrying an off-form amount)
and the census fixture fires it on both screen paths. The source states the reasoning at the rule.

## 2. FR-221 — limb (b) is asked, and it is asked at RETURN level

`design/forms/extract/i1040gi--2025.txt:41882-41887` gives two exits; the refusal offered only (a).

**The field E added inside `StateLocalRefundFacts` has been MOVED OUT to
`ReturnInputs::prior_year_elected_sales_tax`, and that move is the substance of FR-221 rather than a
tidy-up.** Inside the block every field is serde-REQUIRED, so the sales-tax filer could only reach
*"none of your refund is taxable"* by first transcribing twenty-one prior-year figures that decide
nothing for them — the exit *required the worksheet's inputs*, which is exactly what the brief
forbids. `figure()` now takes limb (b) as its own argument and checks it **before**
`FactsNotCollected`, so the exit is taken with `facts: None`.

- `QuestionId::PriorYearElectedSalesTax` (index 66), a class-(A) declaration, **live iff limb (a) is
  live AND answered `Yes`** — the TIP is a disjunction, so a filer who did not itemize is asked
  nothing. Neutral `false`: forgoing the exit routes into the worksheet, which can only make the
  refund MORE taxable (the overstating, recoverable direction).
- A `RENDERED_PROMPTS` entry quoting **year N−1**, the same year limb (a)'s prompt quotes, with the
  TIP verbatim after it. The static fallback names no year.
- **Named so the two cannot be confused, in both directions**: `ScheduleAInputs::salt_use_sales_tax`
  (this year's election, this year's Schedule A line 5a) now carries a doc note pointing at the new
  field and vice versa, and `question_to_field` deliberately does **not** dedup the new declaration
  to `SaSaltUseSalesTax` — same words, a different year, an independent answer.
- **Exactly one place on the return answers limb (b)**, so the two-testimonies shape cannot arise
  inside the block.

## 3. The refusal removal, per E's §7 — and its five obligations

`RefuseReason::StateAndLocalRefundWorksheetNotComputed` **survives, narrowed to three states**, each
of which the worksheet itself reports: `FactsNotCollected`, `NoArchivedRevision`,
`NoLine5AmountForStatus`. The *condition* is gone; a call to `state_local_refund::decide(ri)` stands
in its place, and `Ok(_)` refuses **nothing** — that is the wall coming down. Each of the three now
carries its own detail naming its own remedy (supply the `[state_local_refund]` keys / the revision
is not archived / line 5 names no amount for that status).

1. **A NEW reason, `Pub525ItemizedDeductionRecovery(Pub525Exception)`** — ranked first by E, built
   first. It quotes **that revision's own sentence** for the condition the filer affirmed, through
   `Revision::exception_text`, plus the Exception header and *"Itemized Deduction Recoveries in
   Pub. 525"*. Not a reuse: *"the form forbids this worksheet for you"* cannot be cured by supplying
   any figure. It has a `param_free_fixtures` entry (the reachability census) and an `attribute`
   mapping.
2. **The reachability fixture is UNCHANGED, and CHECKED.** It still refuses
   `StateAndLocalRefundWorksheetNotComputed`, now via `FactsNotCollected`: `answer_remaining` answers
   the new limb-(b) question at its neutral, so no edit was needed. Its comment now says so.
3. **Two testimonies about one line REFUSE** — `TwoTestimoniesAboutStateRefund`, fired when
   `sch1.state_refund_taxable` is non-zero beside **either** a present `state_local_refund` block
   **or** `prior_year_elected_sales_tax == Some(true)`. Neither trigger is reachable on a return that
   files today (the block arrived with FR-196, the question with FR-221), so the refusal walls
   nothing that was fileable before it existed.
4. **The `Option` is preserved all the way onto the page.** `Schedule1Parts::state_refund_1` and
   `printed::Schedule1Lines::line1` are `Option<Usd>`, and `btctax-forms::schedule23` writes line 1
   with `push_money_opt` — the emitter DECLINES TO WRITE, the same *"fixed at the writer"* repair
   Form 1040 lines 34/35a/37 already carry, because `Usd` still cannot express blank (§G-11 P0b).
   `Some(0)` still prints: a zero the worksheet computed is different testimony from an empty line.
5. See §1.

**One obligation E could not have known about, because the wiring creates it.** While the worksheet's
output was unread, a stray facts block changed nothing; once Schedule 1 line 1 reads it, a block on a
return that reports **no refund** would print income with the TIP's limb (a) never asked — the §111(a)
gate is not live to ask it. `NotUsable::NoRefundReported` (raised by `decide`, not by `figure`, because
it is a fact about the return) and `RefuseReason::StateLocalRefundFactsWithoutARefund` close it, in
the same commit as the wiring.

## 4. FR-222 — box 2 is the worksheet's input, and the kill MOVES it

`refund_on_forms_1099g(ri)` sums every `g_1099[].box2_state_refund` (*"Form(s)"*, plural) and feeds
`figure`'s line 1, where the block's own *"(or similar statement)"* half is **added** to it before the
line-1 cap. The source states that taxcalc's `e00700` makes the output oracle-checkable once
`GoldenInputs` gains the axis (FR-196d) and that until then it is self-consistent only — the axis was
**not** built here.

★ The test is written so *"900 in, 900 out"* could not pass by accident: it **halves the box and
demands the taxable part halve**, adds a second 1099-G and demands the total grow, then adds the
off-form amount and demands it add rather than replace. `refund_on_forms_1099g` returning `ZERO` (the
defect restored) reds it.

★ A stale claim my own change created, found and fixed: `testonly.rs`'s `ORACLE_INVISIBLE` note for
`g_1099[].box2_state_refund` said *"the taxable state refund is taken from
`sch1.state_refund_taxable`"* — true until the worksheet was wired. It now names the real reason the
box is oracle-invisible (no `GoldenInputs` axis) and points at FR-196d.

## 5. FR-196a — the predicate is shared, and a derived test joins the code to the document

- `btctax_core::tax::return_inputs::import_forces_provenance_to_user(path)` is now the **one**
  definition of *"does import force this key's provenance to `user`?"*. `xtask::toml_schema::note_for`
  calls it instead of re-spelling the `ends_with`.
- The five-site forcing is extracted as `cmd::tax::force_carry_provenance_to_user`, and
  `cmd::tax::tests::every_provenance_key_the_schema_says_is_forced_actually_is` joins the two:
  it **derives** the key set from `scrub_axis::maximal_sentinel` through that predicate, plants
  `computed` at every one of them through the real TOML surface, **asserts the plant took** (or the
  test would prove nothing), runs the real normalisation, and demands `user` back. It also refuses to
  run on fewer than the six keys that existed when it was written — a silently empty expectation is
  how this class of check goes green while blind.
- ★ Honest boundary, stated in the source: the predicate keys on the PATH, so a `CarryProvenance`
  leaf named something other than `provenance` / `*_provenance` is outside both the forcing and the
  published annotation. That is *consistent* (the document and the code are wrong together) but not
  total; the repair is a name, not a second list.

## 6. B1 — every new instrument was observed RED on a planted defect, and the greens are pasted

**(a) The sales-tax filer files with a BLANK line 1.** Plant: collapse the blank at the writer
(`let line1 = Some(round_dollar(p.state_refund_1.unwrap_or(Usd::ZERO)))`) — the §G-11 defect exactly.

```
thread '…::b1_the_sales_tax_election_filer_files_with_a_blank_schedule_1_line_1' panicked at
crates/btctax-core/src/tax/state_local_refund.rs:2411:9:
assertion `left == right` failed: Schedule 1 FILES (line 7 carries the unemployment) and its line 1 is BLANK
  left: Some(Some(0))
 right: Some(None)
```

**(b) The emitter itself declines to write.** Plant: `push_money_opt` back to `push_money` with an
`unwrap_or(ZERO)`.

```
thread 'schedule_1_line_1_is_left_empty_when_section_111a_says_blank' panicked at
crates/btctax-forms/tests/full_return_forms.rs:1392:5:
assertion `left == right` failed: L1 must be EMPTY, not "0" — a zero here is testimony the filer never gave
  left: Some("0")
 right: None
```

**(c) The Pub. 525 exception refuses under its OWN reason.** Plant: reuse
`StateAndLocalRefundWorksheetNotComputed` for the exception arm — the collapse E warned against.

```
thread '…::b1_a_pub525_exception_refuses_under_the_new_reason_quoting_the_form' panicked at
crates/btctax-core/src/tax/state_local_refund.rs:2477:9:
assertion `left == right` failed: NOT `StateAndLocalRefundWorksheetNotComputed` — supplying more figures cannot cure it
  left: StateAndLocalRefundWorksheetNotComputed
 right: Pub525ItemizedDeductionRecovery(E6OwedAmtInPriorYear)
```

That plant ALSO reds `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths`,
because the reason leaves the source census while its fixture still expects it — two independent
instruments catching one defect.

**(d) Two testimonies refuse.** Plant: an `if false` guard on the rule.

```
thread '…::b1_two_testimonies_about_schedule_1_line_1_refuse' panicked at
crates/btctax-core/src/tax/state_local_refund.rs:2524:9:
assertion `left == right` failed
  left: None
 right: Some(TwoTestimoniesAboutStateRefund)
```

**(e) FR-222's box has a reader.** Plant: `refund_on_forms_1099g` returns `Usd::ZERO`.

```
thread '…::fr222_form_1099g_box2_is_what_the_worksheet_reads' panicked at …:2599:9:
  left: 0   right: 900
thread '…::b1_the_income_tax_filer_with_the_facts_files_via_the_worksheet' panicked at …:2440:9:
  left: Amount(0)   right: Amount(900)
```

**(f) A planted sixth un-forced field reds the FR-196a test.** Plant: delete the
`state_local_refund` forcing — literally FR-196's own defect restored.

```
thread 'cmd::tax::tests::every_provenance_key_the_schema_says_is_forced_actually_is' panicked at
crates/btctax-cli/src/cmd/tax.rs:1600:9:
`docs/income-import-schema.md` publishes these keys as "forced to `user`" and `import_return_inputs`
does not force them, so the document asserts a guarantee the code does not keep (FR-196a):
["state_local_refund.provenance"]
```

**All six restored, and green:**

```
Summary [   0.017s] 38 tests run: 38 passed, 1395 skipped     # btctax-core state_local_refund (was 32)
        PASS  btctax-cli cmd::tax::tests::every_provenance_key_the_schema_says_is_forced_actually_is
        PASS  btctax-forms::full_return_forms schedule_1_line_1_is_left_empty_when_section_111a_says_blank
Summary [  19.537s] 3738 tests run: 3738 passed, 12 skipped   # whole workspace
```

The income-tax vector is hand-checked line by line against the **TY2024** revision and asserted at
three depths — the worksheet (`line5 == Some(13_850)`, `part == Amount(900)`), the printed line
(`Schedule1Lines::line1 == Some(900)`), **and AGI** (`ar.agi == 2_100`, i.e. $1,200 unemployment plus
$900 refund), because a printed-line assertion alone would miss that AGI is the argument to the §221
phase-out, the §1411 MAGI, the AMT, the §170(b) base and the itemize election.

## 7. Two decisions worth reviewing

**(i) FR-196e is DECIDED, not silenced — and decided AGAINST a refusal.** A non-zero
`sch1.state_refund_taxable` beside `itemized_prior_year == Some(false)` (the TIP's limb (a) alone,
with no facts block and no limb-(b) `Yes`) does **not** refuse, and the attested figure prints exactly
as it does today. Three reasons, in order: the two are not necessarily a contradiction (Pub. 525's
*Itemized Deduction Recoveries* can make a recovery of an item from **another** year income on this
same line, and limb (a) speaks only about the year *this* refund's tax was paid in); printing the
filer's own figure can only OVERSTATE against the TIP, never understate; and refusing it would be a
brand-new wall on a state that files today, which is the opposite of this parcel's purpose. The
reasoning sits in `schedule_1_line1`'s doc comment, where the branch is.

**(ii) `schedule_1_line1` prefers a NON-ZERO attestation over a decision, and the refusal is what
makes that safe.** The branch is `Ok(d) if attested == ZERO => d.schedule_1_line1()`. Without the
preference, a decision could silently replace a figure the filer typed and the return would go out
understated by the whole of it; without the two-testimonies refusal, the preference would silently
leave every answer the filer gave the worksheet unread. The pair IS the comparison, which is the
instrument `a-figure-with-no-reader` asks for. A zero attestation is not testimony here — nothing on
the input surface asks for it, and it is `Usd::ZERO` on a defaulted return (§G-23's *"stated zero"*).

## 8. Files edited OUTSIDE parcel J's stated ownership — reported, per the brief

Every one is a compiler- or KAT-forced consequence, and **none touches parcel K's files**
(`fill8949*.rs`, `overflow.rs`, the 8949 tests).

| file | why | forced by |
|---|---|---|
| `btctax-core/tax/classifier.rs` | the `_`-free destructures of `ReturnInputs` and of the facts block; the new declaration classified beside limb (a) | E0027 / E0026 |
| `btctax-core/tax/scrub.rs` | PII partition destructure | E0027 |
| `btctax-core/tax/scrub_axis.rs` | `maximal_sentinel` realizes the new leaf, and loses the removed one | E0063 / E0560 |
| `btctax-core/tax/return_1040.rs` | `Schedule1Parts::state_refund_1: Option<Usd>` plus its five readers (the AGI sum, Form 8615 unearned income, the crypto-attributable baseline, Form 6251 line 2b, and the printed parts) | E §7(4) |
| `btctax-core/tax/printed.rs` | `Schedule1Lines::line1: Option<Usd>`, rounding INSIDE the `Option`, line 10 adding what the paper adds | E §7(4) |
| `btctax-core/tax/line_coverage.rs` | one fixture literal | E0308 |
| `btctax-core/tax/testonly.rs` | the stale `ORACLE_INVISIBLE` note (see §4) | correctness |
| `btctax-forms/src/schedule23.rs` | `push_money_opt` for line 1 | E §7(4) |
| `btctax-forms/tests/full_return_forms.rs` | one fixture literal, plus the new emitter B1 test | E0308 / B1 |
| `btctax-input-form/src/{seam,attribute}.rs`, `src/spec/{registries,coverage,mod}.rs` | a `FieldId`, both directions of `question_to_field`, the exhaustive `attribute` match, the coverage pair and its liveness primer, three count pins | E0004 / red KATs |
| `btctax-tui-edit/src/draw_edit.rs` | one new probe, so the drawn-sentence walk is not reporting success about words it never saw | red KAT |
| `btctax-cli/src/cmd/answer.rs` | the per-question scenario for the new declaration | red KAT |
| `docs/income-import-schema.md`, `docs/examples/examples.md` | regenerated by their own generators, never hand-edited | red KATs |

★ `spec/coverage.rs` also carries a **corrected exemption justification**. The `state_local_refund`
`EXEMPT_LEAVES` entry claimed *"nothing is reachable through the gap today — `screen_inputs` still
refuses, so no return carrying this block can be committed at all."* That was true when written and is
**false now**. The entry stands (the block is still TOML-only) but its comment now says the gap is a
usability one rather than an unreachable one, and that FR-196b is on the critical path.

## 9. Follow-ups

- **FR-221 — CLOSED.** Limb (b) is asked at return level, and the exit needs none of the worksheet's
  inputs.
- **FR-222 — CLOSED.** Box 2 is worksheet line 1's input, with a kill that MOVES the figure.
- **FR-196a — CLOSED** by a shared predicate plus a derived joining test, with its blind spot stated
  in the source.
- **FR-196e — CLOSED by decision** (§7(i)), not by a refusal.
- **FR-196b (the input-form section) is now on the critical path, not a tidy-up.** A filer can only
  reach the worksheet's twenty-one figures through an `income import` TOML; the editor cannot ask
  them. Its exemption's *"nothing is reachable"* justification is gone.
- **FR-196d (the `GoldenInputs` taxable-state-refund axis) is the cheapest remaining witness.** The
  worksheet's output is self-consistent only; taxcalc's `e00700` and OTS's `S1_1` both exist.
- **NEW, Minor — `Schedule1Lines::line1` is the ONLY blankable line on Schedule 1.** Lines 3, 5, 6, 7
  and the rest still print `0` where the form would leave them empty (line 5 is Schedule E and line 6
  Schedule F — both asserted blank today only because they are unrepresentable). The `Option` plus
  `push_money_opt` pattern now has a worked precedent on this very form; extending it is §G-11 P0b's
  own work and was deliberately NOT done here.
