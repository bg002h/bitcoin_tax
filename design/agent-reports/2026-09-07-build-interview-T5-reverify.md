# Re-verification — interview T5 build + fold (commits `34472416` + `9a845dc5` = `2fe4ba5f`)

Independent re-verifier, own worktree at `2fe4ba5f` (unchanged throughout — every plant made via a
`cp` backup and restored; `git status --short` and `git diff --stat` are empty at the end). No
commits, no subagents. `export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before
every cargo command; every test run scoped per the brief (`cargo nextest run --locked -p <crate>
-E '<filter>'`, never `cargo test`, never the whole workspace). Brief:
`design/agent-reports/BRIEF-reverify-interview-T5.md`.

**One question:** does every kill the T5 build report (and its fold section) claims actually RED
when its defect is planted today, and does every T5 seam-review finding now have a test that holds
it? All twenty build kills (M1–M20), the fold's nine items (I-1…I-4, M-1…M-4, N-1) with every named
plant, the four pointed questions, and the negative claims were re-driven. **Every kill reds.** Two
Minor documentation-only discrepancies were found in the fold report's own quoted test-summary
counts (the numbers still show RED, just not the exact pass/fail split printed). No Critical, no
Important.

---

## Part 1 — the twenty build kills (M1–M20)

| # | plant | command | RED text (verbatim from this run) | verdict |
|---|---|---|---|---|
| M1 | `"f1099int" => Some(SectionId::W2s)` (`section_of_stem`, `box_census.rs:547`) | `-p xtask -E 'test(box_census)'` | `f1099int/1: Int1099Box1Interest is in [Int1099s], not in this document's own section (W2s) — a box collected somewhere else is CollectedElsewhere...` (both `the_box_to_field_join_is_clean` and `the_join_reds_on_a_field_from_the_wrong_section_and_on_a_reworded_caption` fail — see note below) | RED — matches |
| M2 | `Int1099Box1Interest` label/help reworded "Interest earned" (`sections.rs:2052`) | `-p xtask -E 'test(box_census)'` | `f1099int/1: the box prints "interest income", and no field that collects it ([Int1099Box1Interest]) says so — a box whose caption moved between revisions...` | RED — exact match |
| M3 | `if false && w2.box13_statutory_employee {` (`return_refuse.rs:2129`) | `-p btctax-core -E 'test(a_checked_statutory_employee_box_refuses_naming_schedule_c_line_1)'` | `a checked box 13 refuses` | RED — exact match |
| M4 | bond-premium disjunction narrowed to box 11 only (`return_refuse.rs:2195-2198`) | `-p btctax-core -E 'test(any_bond_premium_box_refuses_naming_pub_550)'` | `box12_bond_premium_treasury > 0 must refuse` | RED — exact match |
| M5 | box 10 dropped from `sum_taxable_interest` (`return_1040.rs:601`) | `-p btctax-core -E 'test(box_10_market_discount_raises_form_1040_line_2b_by_its_own_amount)'` | `★ THE KILL: box 10 is a separate addition to line 2b, not a subset of box 1 — left: 2500, right: 2600` | RED — exact match |
| M6 | `sum_student_loan_interest` returns `Usd::ZERO` (`return_1040.rs:636`) | `-p btctax-core -E 'test(derive_applies_student_loan_adjustment_from_the_1098e_rows)'` | `left: 0, right: 1000` | RED — exact match |
| M7 | `Form1099BNeedsForm8949 => vec![Anchor::NotInForm {...}]` (`attribute.rs:337`) | `-p btctax-input-form -E 'test(the_five_reattributed_anchors_point_at_real_form_fields_and_the_count_fell_by_five)'` | `the source now has 14 NotInForm anchors, not 13` (count is 13→14 post-fold, since I-4 added `QualifiedTipsCautionNotMet`; pre-fold this was 12→13 — expected evolution, not a discrepancy) | RED — matches (numbers updated for the fold baseline, as expected) |
| M8 | `"int_1099"` re-added to `EXEMPT_PREFIXES` (`coverage.rs:462`) | `-p btctax-input-form -E 'test(every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt)'` | `EXEMPT_PREFIXES is a RATCHET and may only shrink: 6 entries, ceiling 5. ...adding one back hides a leaf nobody collects.` | RED — exact match |
| M9 | `WagesWithoutW2Question.live = |_| true` (`questions.rs:1615`) | `-p btctax-core -E 'test(each_paired_question_is_live_exactly_on_its_rows_no_and_blocks_there)'` | `W2 = Yes: the filer HAS the document, so WagesWithoutW2Question must not be asked` | RED — exact match |
| M10 | `ItemizedPriorYear.live` narrowed to the 1099-G limb only (`questions.rs:1697`) | same test class, `-p btctax-core -E 'test(the_prior_year_itemize_gate_is_live_with_no_1099g_row_at_all)'` | `★ THE KILL: the §111(a) gate must reach a filer whose refund arrived with no document` | RED — exact match |
| M11 | `requires_transcription(G1099) => false` (`document_census.rs:581`) | `-p btctax-core -E 'test(a_truthful_1099b_on_form_8949_and_a_box_2_only_1099g_both_file)'` | `left: None, right: Some(DocumentDeclaredNotTranscribed { kind: G1099 })` | RED — exact match |
| M12 | `transcription_warnings` returns `Vec::new()` unconditionally | `-p btctax-core -E 'test(a_box_1_in_box_3_slip_warns_and_writes_nothing) or test(an_all_zero_row_warns_on_the_information_returns_and_never_on_a_w2)'` | `the slip breaks the box-4 rate AND the wage base: [] — left: 0, right: 2` **and** `an all-zero W-2 is ordinary...must NOT warn; each information return must: [] left: [] right: [Form1099Int, Form1099Div, Form1099G, Form1098E]` | RED (both) — exact match |
| M13 | `render_transcription_warnings` call deleted from `cmd/tax.rs:711` | `-p btctax-cli -E 'test(a_box_1_in_box_3_slip_prints_a_transcription_warning_on_the_report_and_changes_no_figure)'` | `the slip must be surfaced on the report: ` (whole report, no warnings block) | RED — matches ("the whole report, with no block") |
| M14 | warning loop deleted from `draw_tax_inputs_status` (`draw_edit.rs`) | `-p btctax-tui-edit -E 'test(a_transcription_warning_is_rendered_in_the_editor_and_changes_no_stored_value)'` | `the slip must be surfaced where the row is edited:` (whole screen render, no warning line) | RED — exact match |
| M15 | `undated_rows_block(...)` call deleted from `admin.rs:1850` | `-p btctax-cli -E 'test(the_manifest_names_a_document_row_transcribed_without_a_date_and_stays_silent_when_dated)'` | `the manifest must carry the block:` (manifest shows only the hand-marks section) | RED — matches |
| M16 | empty-rows condition replaced with `&& false` in `FilerRecordsDeclaredNotTranscribed`'s guard (`return_refuse.rs`) | `-p btctax-core -E 'test(each_paired_question_is_live_exactly_on_its_rows_no_and_blocks_there)'` | `a declared record with none refuses` (the `.expect()` panics — rule never fires) | RED — exact match |
| M17 | `TipsDeductionForgoneWithTtoc` push guarded by `if false` (`advisories.rs:1189`) | `-p btctax-core -E 'test(a_treasury_tipped_occupation_code_beside_an_unclaimed_part_ii_advises_and_never_refuses)'` | `left: 0, right: 1` (0 TTOC advisories present, 1 expected) | RED — matches |
| M18 | `G1099Box10FamilyLeave` Field entry deleted from `sections.rs` | `-p btctax-input-form -E 'test(every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt)'` | `these IN-SCOPE leaves are covered by NO Field and are NOT in EXEMPT...: ["g_1099[0].box10_family_leave_benefits"]` | RED — exact match |
| M19 | the 1099-INT box-10 `BoxEntry` deleted from `box_census.rs` | `-p xtask -E 'test(every_printed_box_carries_exactly_one_entry)'` | `box 10 ("10 Market discount") is printed on the form and NOTHING decides it — we forgot this box...` | RED — exact match |
| M20 | family-leave screen made unreachable, `> Usd::ZERO` → `> dec!(999999999)` (`return_refuse.rs:2319`) | `-p btctax-core -E 'test(every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths)'` | `the commit gate must reach FamilyLeaveBenefits on its own fixture — left: None, right: Some("FamilyLeaveBenefits")` | RED — exact match |

**Note on M1/M2's Summary counts (Minor, N-class documentation mismatch).** The fold's I-1 section
quotes `Summary: 12 tests run: 11 passed, 1 failed` for the builder's M1 plant. Because
`section_of_stem` is a pure function keyed by stem string, the plant also breaks the *new* join
test's own positive control (`the_join_reds_on_a_field_from_the_wrong_section_and_on_a_reworded_caption`,
assertion 0), which independently calls `join_failures_for` on a synthetic `f1099int` entry. Measured
here: **both** `the_box_to_field_join_is_clean` and `the_join_reds_on...` fail — `12 tests run: 10
passed, 2 failed`, not 11/1. Same shape for my M2 plant (reworded caption): `10 passed, 2 failed`. The
kill still fires exactly as claimed (`the_box_to_field_join_is_clean` reds with the quoted message);
only the printed pass/fail split in the fold report is off by one test. Not a functional defect — filed
as Minor.

---

## Part 2 — the fold section (I-1 … I-4, M-1 … M-4, N-1)

### I-1 — the join gets a real kill

| plant | command | RED text | verdict |
|---|---|---|---|
| **THE CONTROLLER'S PLANT** — `if true { return Vec::new(); }` at the top of `join_failures_for` | `-p xtask -E 'test(box_census)'` | `★ THE KILL (a Collected naming a field of another section): the join must red on this planted table, and it returned NOTHING — the checker is not checking` | **RED — CONFIRMED.** This is the pointed check: the controller's plant now reds, where before the fold it left the whole suite green (per the ledger's row I-1, reproduced pre-fold: `12 tests run: 12 passed`). |
| the builder's M1 plant, `section_of_stem("f1099int") => Some(W2s)` | `-p xtask -E 'test(box_census)'` | `f1099int/1: Int1099Box1Interest is in [Int1099s], not in this document's own section (W2s)...` — see Part 1 note on the count | RED — matches (message exact; count off by one, see note above) |

### I-2 — the mirror refusal, and the exit that makes it one

Rule as implemented (`return_refuse.rs:1708-1723`): `!ri.schedule_b_filer_records.is_empty() &&
ri.interest_or_dividends_without_1099 != Some(true)` ⇒ `RefuseReason::FilerRecordsContradicted` —
keyed on the **answer**, not `question_is_live`, exactly as the fold's deviation states.

| plant | command | RED text | verdict |
|---|---|---|---|
| PLANT 1 — rule removed (`if false && ...`) | `-p btctax-core -E 'test(filer_records_with_no_answer_authorising_them_refuse_and_stay_removable)'` | `★ THE KILL (a): a "No" beside a transcribed row is a contradiction between sworn testimony and a printed figure — it must refuse, not file` | RED — exact match |
| PLANT 2 — narrowed to `question_is_live(...) && != Some(true)` | same test | `★ THE KILL (b): a closed door with orphan rows must refuse — the rows keep printing on Schedule B and in the 1040 line 2b/3b sums` | RED — exact match |
| PLANT 3 — the review's literal `!(live && Some(true))` form | same test + `-p btctax-core -E 'test(every_replaced_field_preserves_its_class_in_every_representable_state)'` | `★ THE KILL: a standing YES authorises the row even after the census closed the door...` (`left: Some(FilerRecordsContradicted) right: None`) **and**, independently, the `scrub_axis` maximal-sentinel assertion (`the maximal sentinel must be a FILEABLE return...`) | RED (both, `2 tests run: 0 passed, 2 failed`) — exact match |
| exit test — liveness reverted to the door alone (`schedule_b_records_live` = `schedule_b_door_open`) | `-p btctax-input-form -E 'test(orphan_filer_records_stay_visible_while_add_stays_on_the_door)'` | `★ THE KILL: with the door closed and a row on the return the section must STAY VISIBLE, or the filer cannot remove the figure FilerRecordsContradicted is about` | RED — exact match |
| exit test — `add` moved onto liveness instead of the door | same test | `★ THE KILL: add stays on the DOOR — a visible section is not authorisation to create testimony the filer never gave` | RED — exact match |

**★ Pointed check (the brief's specific ask).** Read the current (unplanted) test body
(`return_refuse.rs:2945-3032`) and re-ran it at baseline:
- **State (a)** — door live, answer `Some(false)`, one row → `screen_inputs` returns
  `Some(FilerRecordsContradicted)`. **Still refuses.**
- **State (b)** — both census rows flipped to `Some(true)` (door closed), the fixture's answer
  stays `Some(false)` (stale), two rows present → `screen_inputs` returns
  `Some(FilerRecordsContradicted)`, and the orphan rows are confirmed still printing on Schedule B
  (`schedule_b_lines(&closed)` shows the $1,200 combined row). **Still refuses.**
- **The (Y, closed door, one row) case** — same as (b) except `interest_or_dividends_without_1099`
  is flipped to `Some(true)` (the standing-YES case) → `raw(&still_yes) == None`. **Files.**
  `orphan_filer_records_stay_visible_while_add_stays_on_the_door` (baseline, unplanted) PASSES,
  confirming the section stays visible in this state via `schedule_b_records_live` (door open OR
  rows present).

Both `filer_records_with_no_answer_authorising_them_refuse_and_stay_removable` and
`orphan_filer_records_stay_visible_while_add_stays_on_the_door` **PASS at baseline** (confirmed by a
direct unplanted run after every I-2 plant was reverted). All three probed states verified.

### I-3 — D-7's second limb held

| plant | command | RED text | verdict |
|---|---|---|---|
| delete the 1099-DIV limb | `-p btctax-core -E 'test(each_paired_question_is_live_exactly_on_its_rows_no_and_blocks_there)'` | `★ THE KILL: EITHER row's No opens the door — a filer with 1099-INTs and no 1099-DIV can still hold a nominee distribution` | RED — exact match |
| delete the 1099-INT limb (the fold's added mirror) | same test | `★ THE KILL: ...and symmetrically for the INT limb` | RED — exact match |

### I-4 — fail closed on the tips Caution

Cite verified against the archive: `f1040s1a--2025.txt` exists (`i1040s1a--2025.txt` does not);
lines 24-25 read *"Caution: Fill out Part II only if you received qualified tips. These tips must
have been received in an occupation listed at IRS.gov/TippedOccupations..."* — matches the refusal's
quoted text exactly. `classifier.rs:698-701` was read and confirms the exemption reason now states
"a claimed Part II ... REFUSES ... Unclaimed, `false` still forgoes and cannot overstate" — correcting
the false claim the review found.

| plant | command | RED text | verdict |
|---|---|---|---|
| rule removed (`if false && ...`) | `-p btctax-core -E 'test(a_claimed_part_ii_with_any_caution_condition_false_refuses_and_quotes_the_caution)'` | `★ THE KILL: 3,000 of tips claimed with every gating condition at its serde default false must REFUSE — taking the deduction there is the understatement direction` | RED — exact match |
| third condition dropped from predicate | same test | `★ THE KILL: a false meets_qualified_tip_criteria alone must refuse a claimed Part II` (`left: None, right: Some(QualifiedTipsCautionNotMet)`) | RED — exact match |

`Schedule1A::compute` (`schedule_1a.rs`) confirmed untouched by `git diff` at every checkpoint —
FR-72's compute gating is correctly left alone.

### M-1 — one rule for income boxes with no reader

| plant | command | RED text | verdict |
|---|---|---|---|
| drop the 1099-G box 6 limb | `-p btctax-core -E 'test(every_income_box_with_no_reader_refuses_on_its_own_amount)'` | `★ THE KILL: 1099-G box 6 (taxable grants) carries income and no line reads it — it must REFUSE, not vanish` | RED — exact match |
| `> Usd::ZERO` → `>= Usd::ZERO` on the Schedule F pair | same test | Test reds, but the FIRST panic encountered is `1099-G box 9 (market gain) refused with the wrong reason` (an earlier assertion in test order, triggered because a `>=` on box 7 fires against a zero-valued fixture built for a different box before the loop reaches the "zero must FILE" assertion the fold quotes). The "a zero in {what} must FILE" assertion (`return_refuse.rs:5877`) exists later in the same test and is reachable — confirmed by reading the source. | RED (confirmed) — **Minor**: the exact quoted panic text in the fold report is not the first one this plant actually produces; the underlying kill is real. |

### M-2 — box 4/6 warnings suppressed on box-12 code A/B

| plant | command | RED text | verdict |
|---|---|---|---|
| remove the box-4 suppression | `-p btctax-core -E 'test(uncollected_tax_on_tips_silences_its_own_box_and_nothing_else)'` | `★ THE KILL: box 12 code A says the employer could not collect the tax on tips and must NOT include it in box 4 — warning there trains the filer to ignore the class` | RED — exact match |
| blanket `has_code("A") \|\| has_code("B")` on box-6 | same test | `★ THE KILL: code A is about box 4 alone — a blanket "has A or B" suppression would lose the box-6 check on this W-2` (`left: 0, right: 1`) | RED — exact match |

### M-3 — sweep grouping and mid-round liveness

| plant | command | RED text | verdict |
|---|---|---|---|
| revert the partition to two groups | `-p btctax-cli -E 'test(the_document_less_income_door_is_asked_with_the_census_not_after_the_skippables)'` | `★ THE KILL: the gate declarations stay behind the census and its follow-ups` | RED — exact match |
| drop the mid-round liveness re-check | `-p btctax-cli -E 'test(a_question_that_dies_earlier_in_the_same_round_is_never_put_to_the_filer)'` | `★ THE KILL: the §111(a) gate died the moment the refund question was answered "n" earlier in this very round — putting it to the filer anyway records an answer to a question nothing on the return is asking.` | RED — exact match |

### M-4 — `current_prompt` is the only resolver

| plant | command | RED text | verdict |
|---|---|---|---|
| resolve through the static prompt (`Cow::Borrowed(q.prompt)`) inside `current_prompt` | `-p btctax-core -E 'test(a_rendered_prompt_is_given_not_wording_changed_after_it_is_answered)'` | `★ THE KILL: FilingStatusConfirmed was just answered under the words it is asked in. Reading the STATIC prompt here makes it WordingChanged, which is a refusal firing on a correct answer — D-1 exactly, one surface over.` (`left: WordingChanged, right: Given`) | RED — exact match |
| resolver hands the comparison an empty prompt | `answer_status_against(ri, key, "")` in place of `&p` | same assertion reds, same message | RED — exact match |

### N-1 — dead loop removed

Confirmed by source inspection (`provenance.rs:150-156`): no `for (i, w) in ri.w2s...` loop exists;
the explanatory comment is kept as a standalone `★` note beside the five family loops, exactly as
claimed. No dedicated kill test is claimed for this Nit in the fold report, and none was expected.

---

## Part 3 — negative claims

| claim | check | result |
|---|---|---|
| `box-census` OK with the field join | `cargo run -q -p xtask -- box-census` | `box-census OK: 246 printed boxes across 15 archived editions of 7 information returns, every one decided (246 entries)` — matches exactly |
| `line-coverage` 341 / 24 / 0 / 12 | `cargo run -q -p xtask -- line-coverage` | `line-coverage OK: 341 money lines across 17 form(s) [...], 24 exception(s) (ratchet 24), 0 unverifiable (ratchet 0), 12 not line-bound (ratchet 12)` — exact match |
| `census-join` 298 / 13 | `cargo run -q -p xtask -- census-join` | `census join: 298 unmodeled entries across 13 maps [...]` — exact match |
| TY2024 golden corpus still files | `-p btctax-cli -E 'binary(fullreturn_oracle) or binary(nine_dependents_scenario)'` and `-E 'binary(open_next_year_t4b)'` | `fullreturn_fixture_matches_its_emitter`, `fullreturn_fixture_is_the_kitchen_sink_oracle`, `the_nine_dependent_amt_return_files_a_complete_packet`, `the_ctc_advisory_tells_this_filer_the_credit_is_gone_not_that_it_is_owed` all PASS (4/4); `open_next_year_t4b` 28/28 PASS including `every_leaf_the_seed_carries_is_named_in_the_report` and the 1098-E synthetic-row test |
| no `Usd` leaf outside `EXEMPT_PREFIXES`/`EXEMPT_LEAVES` is uncovered (coverage KAT) | `-p btctax-input-form -E 'test(every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt)'` | PASS at baseline (also independently exercised as the RED target for M8 and M18 above) |
| every new shaped identifier is on `pii-scan-generic.sh`'s allowed list | `bash scripts/pii-scan-generic.sh` and `bash scripts/pii-scan-generic.sh HEAD` (HEAD = `2fe4ba5f`, the fold commit) | `pii-scan: clean (HEAD).` — exit 0, both invocations |

Additionally re-ran the brief's full scoped command set at baseline (unplanted) as a final sanity
close: `-p btctax-input-form` (70/70 passed), `-p btctax-core -E 'test(document_census) \| test(return_refuse) \| test(printed) \| test(provenance)'` (166/166), `-p btctax-cli -E 'test(answer) \| test(tax_report) \| test(fullreturn)'` (46/46), `-p xtask -E 'test(box_census)'` (12/12) — all green, matching the settled fact that the only failures in this worktree are the environment-caused six `form_delta` tests and one `harness_check` test, neither of which was touched by any command run here.

**docs/examples/examples.md check (controller's pointed question (d)):** `grep -c "TRANSCRIPTION WARNINGS" docs/examples/examples.md` → `0`, confirming no committed golden artifact records warning noise from the six now-warning fixtures.

---

## Verdict on the four pointed questions

- **(a) The rendered-prompt hash key.** Confirmed structurally sound: `answer_status` resolves
  through `current_prompt` only, the resolution function is private, and both plants that
  reintroduce D-1's class (static prompt, or an empty comparand) red the same mutation-covering
  test. See M-4 above.
- **(b) The sweep's bound and ordering.** Confirmed: the partition-grouping kill and the mid-round
  liveness kill both red exactly as claimed. See M-3 above.
- **(c) `occupation_on_treasury_list`.** Confirmed resolved by I-4: the refusal now reads all three
  conditions and reds on any of them being `false` while tips are claimed; the classifier's
  exemption text was corrected to match. See I-4 above.
- **(d) The six warning fixtures.** Confirmed as the check working, not a false positive — no
  golden artifact shows the noise (`examples.md` grep = 0), and this reverifier's read of the fixed
  code confirms the box-4/box-6 checks are legitimately anomalous on those W-2s (SS wages present,
  withholding zero, no box-12 code).

---

## Findings

**M-A (Minor).** The fold report's I-1 section quotes `Summary: 12 tests run: 11 passed, 1 failed`
for the builder's M1 plant applied post-fold; the actual result is `10 passed, 2 failed`, because
the plant (a global `section_of_stem` mutation) also breaks the rewritten join test's own positive
control. Same shape independently observed for the M2 plant in Part 1 (both produce 2 failures, not
1). The kill itself is real and the quoted message is exact; only the printed pass/fail split is
off by one test in both places it is quoted.

**M-B (Minor).** M-1's second kill plant (`> Usd::ZERO` → `>= Usd::ZERO` on the Schedule F pair)
reds `every_income_box_with_no_reader_refuses_on_its_own_amount` as claimed, but the FIRST panic
this plant actually triggers is a different assertion in the same test (a "wrong reason" check on
box 9, hit earlier in iteration order because box 7's zero-valued fixture now also refuses) rather
than the "a zero in {what} must FILE" assertion the fold report quotes. That assertion exists later
in the same test body and is reachable; the discrepancy is only in which panic message a reader
would see first.

No Critical, no Important findings. Every kill named in the T5 build report and its fold section
reds under its planted defect; the review's own evidence — including the controller's specific
plant on `join_failures_for` — reds correctly post-fold; all four pointed questions are resolved;
all negative claims hold exactly as stated; the I-2 rule's specific behavior (keying on the answer,
not door liveness) was independently walked through both of the review's probed states plus the
lawful standing-YES exit, and all three behave as the fold's residue item 1 describes.

Counts: C=0 I=0 M=2 N=0
