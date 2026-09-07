# Seam review — interview build T5 (`85962806..9a845dc5`)

Independent adversarial build review, own worktree at `9a845dc5`, read-only for the record: every
plant below was applied in this worktree and reverted; the tree is clean at `9a845dc5` and nothing
was committed. Brief: `design/agent-reports/BRIEF-review-interview-T5.md`. Builder's account:
`design/agent-reports/2026-09-07-build-interview-T5-implementation.md`.

## Commands

```
export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

cargo run -q -p xtask -- box-census
  → box-census OK: 246 printed boxes across 15 archived editions of 7 information returns,
    every one decided (246 entries)                                            [reproduced]
cargo run -q -p xtask -- line-coverage
  → line-coverage OK: 341 money lines across 17 form(s) …, 24 exception(s) (ratchet 24),
    0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)                  [reproduced]
cargo run -q -p xtask -- census-join
  → census join: 298 unmodeled entries across 13 maps …                        [reproduced]

cargo nextest run --locked -p xtask -E 'test(box_census)'      → 12 passed
cargo nextest run --locked -p btctax-core                      → 1274 passed
cargo nextest run --locked -p btctax-core -p btctax-input-form -p btctax-cli
                                                               → 2124 passed, 1 skipped
cargo nextest run --locked -p btctax-tui-edit -p btctax-tui -p btctax-forms -p xtask
                                                               → 1006 passed, 6 failed
    (the six are `form_delta::*` — the settled PDF-less-worktree environment failures,
     plus `harness_check::the_write_hook_denies_new_archives…` when the whole xtask
     binary is run. Not findings.)
```

Plants run (each reverted immediately): the join's `section_of_stem` mis-mapping; the whole
`field_join_failures()` gutted; the D-1 prompt-hash fix reverted; `InterestOrDividendsWithout1099`'s
`||` → `&&`; its 1099-DIV limb deleted; `ItemizedPriorYear`'s 1099-G limb deleted; the family-leave
screen made unreachable (`> Usd::ZERO` → `> dec!(999999999)`). Two probes were compiled into the
tree and removed: one on `Schedule1A::compute`, one on `screen_inputs` + `schedule_b_lines`.

## Summary

The five document sections, the 246-box census, the reach of the new boxes, the refusals, the
warnings and the replacement chain are in good shape — the reach chains are correct and
mutation-covered, the param-free refusal census is a genuinely strong instrument (a screen made
*unreachable* while its source text stayed put still reds), and the `NotInForm` count test is
source-derived and cannot be satisfied by prose.

Four Importants. The headline is **I-1**: T5's own new instrument — the box → `FieldId` join — has
no test that reds when it is removed. Gutting `field_join_failures()` leaves all twelve
`box_census` tests green, *and* the exact defect the join exists to catch (a `Collected` naming a
field of another section) then passes too. The committed "kill" test re-implements the join's
predicates instead of calling it, which is B1 satisfied performatively — the one thing B1 says
cannot happen. **I-3** is the same shape one layer down: D-7, the deviation the builder called out
by name, has no kill either. **I-2** is a missing case on T5's own new surface, with its exact
analogue sitting thirty lines away in the same file. **I-4** answers the controller's question (c)
and is pre-existing, not T5's.

---

## I-1 (Important) — the box → `FieldId` join has no test that reds when it is removed; its B1 kill test is a shadow

**Where.** `crates/xtask/src/box_census.rs:1276` (`field_join_failures`),
`:1824` (`the_box_to_field_join_is_clean`), `:1856`
(`the_join_reds_on_a_field_from_the_wrong_section_and_on_a_reworded_caption`).

**What is wrong.** The join is the whole of T5's contribution to the census — T2 landed the
instrument with the decisions as prose and said in terms that the join was T5's. The clean test
asserts `field_join_failures().is_empty()`, which a function returning `Vec::new()` satisfies
vacuously. The B1 negative test does **not** call `field_join_failures` at all: it re-derives
`form_fields()` / `field_words()` / `normalize()` / `caption_words()` itself and asserts the
*predicates* behave (`!in_section(Box1Wages, Int1099s)`, `!normalize(words).contains(planted)`,
`in_section(Box1Wages, W2s)`). Every one of those assertions stays green with the checker deleted.
So the reviewable sentence B1 demands — *"which test reds when this checker is removed?"* — has the
answer **"none"**.

This is not theoretical: it also disarms the builder's own M1. With the checker gutted, the M1 plant
(`section_of_stem("f1099int") => Some(SectionId::W2s)`, i.e. 14 `Collected` entries naming fields of
another document's section) passes the whole `box_census` suite.

**Evidence.**

```
# (1) the M1 plant alone, checker intact — the clean test reds, as the builder reported:
   "f1099int" => Some(SectionId::Int1099s),   →   "f1099int" => Some(SectionId::W2s),
   cargo nextest run --locked -p xtask -E 'test(box_census)'
   → FAIL box_census::tests::the_box_to_field_join_is_clean
     Summary: 12 tests run: 11 passed, 1 failed

# (2) THE KILL: gut the checker only —
   pub fn field_join_failures() -> Vec<String> {
  +    if true { return Vec::new(); }
       let fields = form_fields();
   cargo nextest run --locked -p xtask -E 'test(box_census)'
   → Summary [0.018s] 12 tests run: 12 passed, 146 skipped          ← ALL GREEN
   cargo nextest run --locked -p xtask            (whole binary)
   → only the six known form_delta + harness_check environment failures; no box_census test red.

# (3) BOTH TOGETHER — the real defect, with the blind checker:
   gutted field_join_failures  +  "f1099int" => Some(SectionId::W2s)
   cargo nextest run --locked -p xtask -E 'test(box_census)'
   → Summary [0.018s] 12 tests run: 12 passed, 146 skipped          ← ALL GREEN
```

Note that `run()` also calls `field_join_failures()`, so `xtask box-census` is blind in exactly the
same way — D-3's stated benefit ("the command a human types") is real but inherits the same hole.

**Minimal change.** Split the per-entry check off the constant so the negative test can drive it:

```rust
pub fn join_failures_for(entries: &[BoxEntry]) -> Vec<String> { …the current body… }
pub fn field_join_failures() -> Vec<String> { join_failures_for(BOXES) }
```

then have `the_join_reds_on_…` build the three planted `BoxEntry` tables it already describes and
assert `!join_failures_for(&planted).is_empty()` for each, with the message text checked. That makes
the kill exercise the instrument rather than a copy of its reasoning, and it cannot be written
without the checker being real.

---

## I-2 (Important) — a filer's "no such income" answer beside filer's-records rows does not refuse, and a closed door leaves rows printing that no surface can show or remove

**Where.** `crates/btctax-core/src/tax/return_refuse.rs:1531` (the
`FilerRecordsDeclaredNotTranscribed` rule — it covers only the *empty* direction);
`crates/btctax-input-form/src/spec/sections.rs:2196` (`schedule_b_records_live`) and `:2355`
(the section's `add` guard); `crates/btctax-core/src/tax/printed.rs:1305`,
`crates/btctax-core/src/tax/return_1040.rs:597` (both sum the rows unconditionally).

**What is wrong.** R5's rows are gated by an answer, but only in one direction. The exact analogue
for documents — `RefuseReason::DocumentCensusContradicted`, *"you answered that you received NO
{doc}, and this return carries {n} transcribed row(s) of exactly that document … btctax cannot know
which of the two is wrong, so it refuses rather than choose"* — sits thirty lines above, in
`screen_document_census` (`return_refuse.rs:1288-1301`). It was not carried across to the door.

Two reachable states:

* **(a)** `interest_or_dividends_without_1099 == Some(false)` with rows present. The filer's own
  class-(A) answer says the income does not exist; Schedule B line 1 names a payer and an amount
  anyway, on a return signed under §6065. Nothing refuses.
* **(b)** The door **closes** (both census rows flip to `Some(true)`) with rows still on the return.
  `schedule_b_records_live` is then `false`, so all five `Field`s are dead and `add` refuses — but
  the orphan rows keep printing on Schedule B and in the 1040 line 2b/3b sums. The filer cannot see,
  edit or withdraw a figure that is on their return.

Direction (b) is at least conservative for tax; (a) is a straight contradiction between sworn
testimony and a printed figure, which is the class this codebase refuses everywhere else.

**Evidence** (probe compiled into `return_refuse.rs`'s test module on top of the module's own
`ri()` fixture, which already sets `interest_or_dividends_without_1099 = Some(false)`; reverted):

```
PROBE3a: screen = None; printed schedule B =
    Some((4400, [ScheduleBRow { payer: "A neighbour", amount: 4400 }]))

PROBE3b: door live = false; screen = None; schedule B =
    Some((6400, [ScheduleBRow { payer: "First Bank", amount: 2000 },
                 ScheduleBRow { payer: "A neighbour", amount: 4400 }]))
```

(3a: one `ScheduleBRecord { amount: 4400, kind: Interest }` on the standard fixture — the return
files clean and prints the row. 3b: same, plus `documents.int_1099 = Some(true)` and
`div_1099 = Some(true)` and one real 1099-INT/1099-DIV each — `question_is_live` is `false`, the
section is invisible, the row still prints.)

**Minimal change.** In `screen_inputs_tiered`, beside the existing empty-rows rule, add the mirror:

```rust
if !ri.schedule_b_filer_records.is_empty()
    && !(question_is_live(QuestionId::InterestOrDividendsWithout1099, ri)
         && ri.interest_or_dividends_without_1099 == Some(true))
{
    return refuse(RefuseReason::FilerRecordsContradicted, "…remove the rows, or answer yes…");
}
```

with the same both-ways message `DocumentCensusContradicted` uses, and a kill for each of the two
states above. (A new `RefuseReason` variant reds the exhaustive cross-crate `attribute` match, so
the anchor is forced.)

---

## I-3 (Important) — D-7's disjunction has no kill: dropping the 1099-DIV limb leaves the whole suite green

**Where.** `crates/btctax-core/src/tax/questions.rs:1642-1647`
(`InterestOrDividendsWithout1099.live`); the test that should hold it is
`return_refuse.rs:2509` `each_paired_question_is_live_exactly_on_its_rows_no_and_blocks_there`,
block (f1).

**What is wrong.** D-7 is a deliberate deviation the builder wrote up by name: *"One question is
paired with TWO census rows … A filer with 1099-INTs and no 1099-DIV can still hold a nominee
distribution, so `int == Some(false) || div == Some(false)`."* The (f1) loop probes only
`DocumentRow::Int1099`, and its `Some(true)` probe closes *both* rows — so the second limb is never
exercised in either direction. Deleting it changes nothing any test can see, which by this repo's
own rule means the guarantee does not exist: a filer with `int_1099 = Yes, div_1099 = No` silently
loses the door for undocumented dividends, an understatement path R3 exists to keep open.

**Evidence.**

```
# plant: delete the 1099-DIV limb
   ri.documents.get(DocumentRow::Int1099) == Some(false)
-      || ri.documents.get(DocumentRow::Div1099) == Some(false)

cargo nextest run --locked -p btctax-core -p btctax-input-form -p btctax-cli
   → Summary [7.005s] 2124 tests run: 2124 passed, 1 skipped        ← ALL GREEN
cargo nextest run --locked -p btctax-tui-edit -p btctax-tui -p btctax-forms -p xtask
   → 1006 passed, 6 failed (the known form_delta/harness environment set only)
```

For contrast, the sibling limb IS covered — deleting
`|| ri.g_1099.iter().any(|g| g.box2_state_refund > Usd::ZERO)` from `ItemizedPriorYear.live` reds
`btctax-cli cmd::answer::tests::income_answer_asks_every_live_declaration` — and turning D-7's `||`
into `&&` reds `btctax-input-form spec::coverage::every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt`,
which is coverage bookkeeping rather than the semantic claim.

**Minimal change.** In (f1), add the asymmetric probe:

```rust
let mut div_only = censused();
div_only.documents.set(DocumentRow::Int1099, Some(true));   // Div stays Some(false)
assert!(question_is_live(QuestionId::InterestOrDividendsWithout1099, &div_only),
        "★ THE KILL: EITHER row's No opens the door — a filer with 1099-INTs and no 1099-DIV \
         can still hold a nominee distribution");
```

---

## I-4 (Important) — `Schedule1aTips`'s three YES-conditions have no reader: the §224 deduction is granted in full with all of them `false`, and the classifier records a reason that is not true

*(This is the controller's question (c). It is **pre-existing** — the classifier arm and
`Schedule1A::compute` are untouched by `85962806..9a845dc5` — but T5's box-14b census note leans on
it, so it is reported here rather than left unsaid.)*

**Where.** `crates/btctax-core/src/tax/schedule_1a.rs:1329-1331` (line 4c is
`qualified_tips_reported` alone); `crates/btctax-core/src/tax/classifier.rs:692-695`;
`crates/btctax-core/src/tax/return_inputs.rs:959, 969, 973`.

**What is wrong.** `occupation_on_treasury_list`, `excludes_unlisted_occupation_tips` and
`meets_qualified_tip_criteria` are collected and read by nothing. A full-file grep finds them only
in the struct definition, the classifier's `exempt` calls, one `scrub` comment and test fixtures —
no refusal, no gate, no compute path. The classifier nevertheless records the exemption reason

> `"§224 / Sch 1-A Part II Caution — tips must be received in an occupation listed at
> IRS.gov/TippedOccupations; \`false\` forgoes the deduction and cannot overstate it"`

which asserts a behaviour that does not exist. Because every one of the three carries
`#[serde(default)]` and defaults to `false`, a TOML author who writes only
`[schedule_1a.tips] qualified_tips_reported = "3000"` takes the full deduction with every gating
condition silently `false` — the understatement direction, laundered as a lawful default.

**Evidence** (probe compiled into `schedule_1a.rs`, reverted):

```
inputs: tips = Some(Schedule1aTips {
    qualified_tips_reported: 3000,
    occupation_on_treasury_list: false,
    treasury_occupation_code: None,
    excludes_unlisted_occupation_tips: false,
    meets_qualified_tip_criteria: false })

PROBE: part2 = Schedule1aPartII { line4a: None, line4b: None, line4c: Some(3000), line5: None,
    line6: Some(3000), line7: Some(3000), line8: Some(60000), line9: Some(150000),
    line10: None, line11_steps: None, line12: None, line13: Some(3000) }
```

**Is box 14b's advisory cover honest without it?** Yes for what it claims, no for what the census
note implies. `Advisory::TipsDeductionForgoneWithTtoc` fires on *code present + no tips claimed*,
which is the overstatement-of-tax direction §3.4 permits silently, and it is a real reader — D-5's
reasoning holds and pointing box 14b at `treasury_occupation_code` would indeed have been
laundering. But the box-census note (`box_census.rs:625`) says box 14b *"is exactly what Schedule
1-A Part II's own Caution turns on"*, and the Caution currently turns on nothing. The converse case
— tips claimed with no TTOC on any W-2, or a code for an unlisted occupation — is the understatement
direction and raises nothing at all.

**Minimal change.** Either gate line 4c on the three conditions in `Schedule1A::compute`, or refuse
in `screen_inputs` when `qualified_tips_reported > 0` and any of the three is `false` (with the
Caution quoted), plus a kill on each; and correct the classifier's exemption reason meanwhile. If
the controller prefers to keep T5's range clean, this wants a **follow-up with an owning phase**
(Schedule 1-A's), not a silent carry — the current state is a green census over a false claim.

---

## M-1 (Minor) — FR-65's own rule is now applied unevenly inside the same document

`box_census.rs`'s new 1099-G box 10 entry refuses because *"an income box with no reader
understates, so it fails closed"*. Three boxes on the same form still say `NotRead`: box 5 (RTAA
payments), box 6 (*Taxable grants*), box 7 (*Agriculture payments*), and box 9 (*Market gain*) — all
income, all with no field, none refusing. Form 1099-B box 13 (*Bartering*) carries the reason
*"reaches Schedule 1 line 8z or Schedule C; btctax models no line 8z inflow"*, which is verbatim the
argument FR-65 used to make box 10 **refuse**. 1099-DIV boxes 9/10 (liquidation distributions) are
the same shape.

This is spec-scoped — `SPEC_interview.md:352` puts only 1099-G payer/TIN/1/2/4 in T5 — so it is
Minor, not a gate. It is recorded because the asymmetry now sits inside one census with two
different answers to one question, and the all-zero-row warning is the only net: a 1099-G with $5,000
in box 6 and zeros elsewhere warns ("check the paper, or remove the row") rather than saying btctax
cannot carry that box. 1099-B box 4's reason claims the forgone credit is *"announced rather than
silent"*; the only announcement is the unconditional `OtherCreditsOmitted` advisory, which names
nothing specific.

## M-2 (Minor) — the W-2 box 4 / box 6 warnings fire on a lawful W-2 with uncollected tax on tips

`transcription_warnings.rs:121-160`. An employee whose wages were insufficient to withhold the
Social Security / Medicare tax due on reported tips gets a W-2 with box 4 < 6.2% × box 3 and/or
box 6 < 1.45% × box 5, with box 12 codes **A** and **B** recording exactly that. Both checks warn.
It is a warning and never blocks, and R4 says every one of these has an edge case — but this one is
named on the form itself and is cheap to suppress (`box12` carrying code A or B). Answers to the
controller's question **(d)** below.

## M-3 (Minor) — the sweep's ordering, and a question that goes dead mid-round is still asked

`cmd/answer.rs:369-395`. `live_questions` partitions census declarations → other declarations →
skippables, so R3's door questions (live only *because* a census row was answered) always land in
sweep 2 — after all nineteen skippables. A filer answers the DOBs, the blindness pair and the
sales-tax election and is then asked "did you receive wages from an employer who issued no Form
W-2?". Separately, `round` is snapshotted at the top of each sweep, so a question that is live at
that moment but dies from an answer given later in the same round is still asked and recorded
(reachable only for `ItemizedPriorYear` after a `state_refund_without_1099g` flip to `n`; the stale
answer is harmless because the refusal is liveness-gated). Both are the builder's residue item 3;
recorded so a T12 presentation pass has them written down.

## M-4 (Minor) — D-1's fix is correct but is spelled out at each site rather than routed through `current_prompt`

`return_refuse.rs:1495` and `interview_state.rs:200` each call
`answer_status(ri, &key, &q.prompt_text(ri))`; `provenance::current_prompt(&key, ri)` already exists
and is what `apply.rs:88` uses. A third surface added later can diverge in exactly the way D-1 did.
Making `answer_status` take the key and resolve the prompt itself (or having both sites call
`current_prompt`) removes the class rather than the instance.

## N-1 (Nit) — dead loop in `undated_document_rows`

`provenance.rs:150-155` iterates `ri.w2s` and does `let _ = (i, w);`. The comment explaining why the
W-2 has no `transcribed_on` is worth keeping; the loop is not.

---

## The controller's four questions

**(a) D-1 — the prompt hash.** *Confirmed, on all three counts.* The defect was real at
`85962806`: `RENDERED_PROMPTS` already contained `FilingStatusConfirmed`
(`git show 85962806:crates/btctax-core/src/tax/questions.rs:102-103`), `income answer` already
recorded `q.prompt_text(&ri)`, and `screen_inputs_tiered` compared against the static `q.prompt` —
so answering that question at the keyboard produced `AnswerStatus::WordingChanged` and refused a
correct answer. `prompt_text` is the right key: it is the sentence `record_answer` hashes, and it is
what `provenance::current_prompt` (`provenance.rs:693-701`) already documents as the only honest
comparand. **No surface still hashes the static prompt.** Every production `record_answer` call goes
through `apply.rs`'s `current_prompt` or `answer.rs`'s `prompt_text`; the three `q.prompt` calls in
`input_form_store.rs` are `#[cfg(test)]` and use `ForeignTrust`, which has no rendered prompt. The
skippable sites correctly use `s.prompt`, matching `current_prompt`'s own skippable arm. The fix is
mutation-covered: reverting it reds `btctax-cli::tax_report
a_pre_d8_vault_refuses_until_answered_and_income_answer_is_the_way_out` (1 failed / 781). See M-4
for the one structural nit.

**(b) D-2 — the sweep.** *No double-ask, no skip, no loop.* Each item is inserted into `asked`
(keyed by `AnswerKey`) before it is asked, so a question cannot be put twice; the loop exits only
when a whole pass finds nothing unasked-and-live, so nothing live can be skipped; `MAX_SWEEPS = 8`
bounds a liveness cycle and today's deepest chain is 3 (census row → door question → §111(a) gate),
so the bound cannot fire on a real filer. Two caveats, both Minor and both in M-3: a question that
dies mid-round is still asked, and the ordering puts the door questions after the skippables. One
observation worth recording — the test helper `answer_script`
(`tests/tax_report.rs:9-58`) is a *re-implementation* of the production sweep, loop bound included,
so a defect shared by both is invisible to it. The builder acknowledged this; it is the right
trade against hand-counted keystrokes, but it is not independent verification of the sweep.

**(c) `Schedule1aTips::occupation_on_treasury_list`.** *Confirmed — see I-4 for the probe output and
the honest/dishonest split on box 14b's advisory cover.* Pre-existing, not T5's.

**(d) The six fixtures that now warn.** *The check working, not a false positive.* Those fixtures
carry W-2s with `box3_ss_wages` / `box5_medicare_wages` non-zero and `box4_ss_withheld` /
`box6_medicare_withheld` left at the `Default` zero — e.g. `return_1040.rs`'s
`w2(Owner::Taxpayer, 50000, 50000, 50000)`. No lawful W-2 reports covered Social Security wages with
no tax withheld on them: an employer whose employee is exempt (§3121 exemptions, MQGE, RRTA, foreign
government) reports **box 3 = 0 too**, and the check is silent there (`tolerance(0) = $1`). Nothing
in the golden output moved (`grep -c "TRANSCRIPTION WARNINGS" docs/examples/examples.md` → 0), so no
committed artifact records the noise. The one genuine false positive is the uncollected-tax-on-tips
case in M-2, which those fixtures are not.

---

## Seams checked clean

1. **Census ↔ registry join (data).** Every `Collected`/`RefuseIfNonzero`/`CollectedElsewhere` entry
   over all 246 boxes passes the join as written: fields exist, sit in the right section, captions
   appear in the collecting field's words, no empty `NotRead` reason, every `RefuseReason` closure
   constructible. `revision_in_force` is pinned by `revision_in_force_pins_the_whole_table`; the two
   2026-only boxes are edition-scoped correctly (`f1099g--2026` alone shows the extra box and the
   one refuse-guard; `fw2--2026` alone shows 30 boxes). `every_stem_maps_to_a_section_or_says_why`
   asserts `f1098` is the only sectionless stem. What is *not* clean is the instrument's own kill —
   I-1.
2. **Reach.** 1099-INT box 10 → `sum_taxable_interest` → Schedule B line 1 row and line 4 → 1040 2b,
   as a *difference* ($2,500 → $2,600), with the printed row carrying it; 1098-E box 1 →
   `sum_student_loan_interest` → the §221 adjustment in **both** `derive_tax_profile` and
   `assemble_absolute`, with the no-row case asserted $1,000 higher; 1099-DIV box 2a →
   `return_1040.rs:645` → Schedule D line 13 (unchanged by T5); 1099-G box 2 → the return-level
   §111(a) gate, blank-by-decision on `No` and `StateAndLocalRefundWorksheetNotComputed` on `Yes`
   naming the worksheet; 1099-B basis/adjustments gate refuses on both `None` and `Some(false)`.
   `screen_absolute`'s §163(d) ceiling was correctly re-pointed at the shared helpers rather than
   keeping a third copy. Refusals: W-2 box 13 → `StatutoryEmployeeW2` naming SCHEDULE C LINE 1;
   1099-INT boxes 11/12/13 → `AmortizableBondPremiumNotComputed` citing §171 and Pub. 550 in the
   over-refusing direction; 1099-G box 10 → `FamilyLeaveBenefits` naming Schedule 1 line 8z.
3. **The param-free refusal census is strong.** Making the family-leave screen *unreachable* while
   leaving its source text in place (`> Usd::ZERO` → `> dec!(999999999)`) still reds
   `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` **and**
   `no_refusal_in_the_import_tier_prescribes_income_answer`. A `RefuseIfNonzero` census entry whose
   screen does not fire cannot survive — which is precisely the FR-65 defect T5 was fixing.
4. **Warnings never write.** `transcription_warnings` takes `&ReturnInputs`, so it is structurally
   incapable of writing; both unit tests and the `report` integration test assert byte-identity
   after (`assert_eq!(stored, slipped)` across a real vault round-trip). Rendered in `report`
   (`cmd/tax.rs:708`) and in the editor status block (`draw_edit.rs:2356`), each with its own
   call-site kill. The all-zero rule correctly excludes the W-2 and names each issuer's own
   threshold; the box-6 bracket [1.45%, 2.35%] correctly admits the Additional Medicare Tax.
5. **The replacement chain.** `sch1.student_loan_interest_paid` is gone from all code (only
   explanatory comments remain); `EXEMPT_PREFIXES` 9 → 5 with a `<=` ratchet; `EXEMPT_LEAVES` lost
   both stale entries and `documents.form_1098` correctly stayed with T9 named; field count
   117 → 175 and covered leaves 115 → 174 with the one uncovered leaf (`DocForm1098`) named and
   justified; the `NotInForm` count is read out of `attribute.rs`'s own body between `pub fn
   attribute(` and the first `#[cfg(test)]`, so neither the test's prose nor its own match arm can
   inflate it — and because it asserts `17 - 5`, any *new* refusal anchored `NotInForm` would red
   too. `IraDeductionClaimed` is asserted positively. All ten new `RefuseReason`s anchor on real
   `Field`s/`Section`s. TY2024 fixtures moved only where the scalar (which held `"0"`) became a row;
   no figure changed in any golden.
6. **TOML and provenance.** `income import` round-trips every new table including
   `transcribed_on = [2026, 34]`, and `serde_ignored` names `int_1099.0.box11_bond_premum` — the
   half that matters, since a dropped bond premium would file a smaller tax than the paper supports.
   The import tier is not package-gated for any of the new box screens, so they run before the
   write. `LEAF_SOURCE` gains `form_1098e` → `Document(Form1098E)` and `schedule_b_filer_records` →
   `FilerRecords`; `undated_document_rows` covers all five dated families (see N-1);
   `record_answer` remains the only production writer of `answer_log` entries.
7. **Classifier.** All four door leaves plus the two new W-2 leaves are destructured by name with no
   `..` and no `_` where an answer lives, and `every_registry_question_is_declared_exactly_once`
   holds the registry ⇔ classifier correspondence in both directions.

Counts: C=0 I=4 M=4 N=1
