# T5 — the 1099-INT / DIV / B / G and 1098-E sections (R4, R5, R3's paired questions)

Implementer report. Branch `main`, shared tree, dispatched at `85962806`. **Nothing committed or
pushed**; the tree is fmt-clean, clippy-clean and green on every crate.

**Gates, run at the end and quoted verbatim:**

```
cargo fmt --all --check                                       → exit 0, no output
CARGO_TARGET_DIR=target-clippy cargo clippy --workspace \
    --all-targets --all-features -- -D warnings               → Finished (no diagnostics)
cargo run -p xtask -- box-census      → box-census OK: 246 printed boxes across 15 archived
                                        editions of 7 information returns, every one decided
                                        (246 entries)
cargo run -p xtask -- line-coverage   → line-coverage OK: 341 money lines across 17 form(s) […],
                                        24 exception(s) (ratchet 24), 0 unverifiable (ratchet 0),
                                        12 not line-bound (ratchet 12)      ← UNCHANGED from T4b
cargo run -p xtask -- census-join     → census join: 298 unmodeled entries across 13 maps […]
                                                                            ← scope UNCHANGED (13)
cargo run -p xtask -- cite-check      → cite-check: OK — 51 quotations, all verbatim.
bash scripts/pii-scan-generic.sh      → pii-scan: clean (HEAD).
```

## Suite lines per crate

| crate | result |
|---|---|
| `btctax-core` | 1274 tests run: **1274 passed**, 0 skipped |
| `btctax-input-form` | 69 tests run: **69 passed**, 0 skipped |
| `btctax-cli` | 781 tests run: **781 passed**, 1 skipped |
| `btctax-tui-edit` | 389 tests run: **389 passed**, 2 skipped |
| `btctax-tui` | 160 tests run: **160 passed**, 2 skipped |
| `xtask` | 157 tests run: **157 passed**, 1 skipped |
| `btctax-forms` | 354 tests run: **354 passed**, 4 skipped |
| `btctax-adapters` | 103 tests run: **103 passed**, 0 skipped |
| `btctax-store` | 45 tests run: **45 passed**, 0 skipped |
| `btctax-oracle-harness` | 5 tests run: **5 passed**, 1 skipped |
| `btctax-update-prices` | 5 tests run: **5 passed**, 1 skipped |

**3342 tests, all passing** (3321 at dispatch). `xtask` is 158 → 157 because the two join tests I
first wrote were merged into one that shares its checker with `run()` — see D-3.

---

## 1. The four 1099 sections, the 1098-E section, and the `[boxes]` census

### The sections (`crates/btctax-input-form/src/spec/sections.rs`, `+SectionId`/`+FieldId` in `seam.rs`)

Six new repeating sections, modelled on `W2s`, in `form_spec()` between `W2Box12` and `ScheduleA`:

| `SectionId` | `Vec` | Fields |
|---|---|---|
| `Int1099s` | `int_1099` | 14 (payer, TIN, transcribed_on + boxes 1,2,3,4,6,8,9,10,11,12,13) |
| `Div1099s` | `div_1099` | 14 (payer, TIN, transcribed_on + boxes 1a,1b,2a,2b,2c,2d,4,5,7,12,13) |
| `ScheduleBFilerRecords` | `schedule_b_filer_records` | 5 (payer_name, payer_ssn `Secret`, payer_address, amount, kind) |
| `B1099s` | `b_1099` | 8 (payer, TIN, transcribed_on, ST/LT proceeds+basis, the gate) |
| `G1099s` | `g_1099` | 7 (payer, TIN, transcribed_on + boxes 1, 2, 4, 10) |
| `Form1098Es` | `form_1098e` | 4 (lender, lender TIN, transcribed_on, box 1) |

Plus two on the W-2 row: `W2Box13StatutoryEmployee` (`Bool`) and `W2Box14bTtoc` (`Text`).
**58 new `Field`s**; `form_spec()` goes 117 → 175.

Struct changes (`return_inputs.rs`): `Form1099Int` +box10/11/12/13; `Form1099G` +`box2_state_refund`
+`box10_family_leave_benefits`; `W2` +`box13_statutory_employee` +`box14b_treasury_tipped_occupation_codes`;
new `Form1098E` and `ScheduleBRecord`/`ScheduleBRecordKind`; `ReturnInputs` +`form_1098e`,
+`schedule_b_filer_records`, +the four R3 tri-states. `Schedule1Inputs::student_loan_interest_paid`
**removed**; Schedule 1 line 21 now reads `sum_student_loan_interest(ri)` = Σ `form_1098e[].box1_interest`.

### The `[boxes]` census, now JOINED to the registry (`crates/xtask/src/box_census.rs`)

T2 landed the instrument with decisions as prose and said in terms that a join before the variants
existed *"would be a hand-list pretending to be a check."* The variants exist now, so:

```rust
pub enum BoxDecision {
    Collected           { fields: &'static [FieldId], note },       // this document's OWN section
    CollectedElsewhere  { fields: &'static [FieldId], note },       // the header, box 12, Schedule A
    RefuseIfNonzero     { field: FieldId, reason: fn() -> RefuseReason, note },
    NotRead(&'static str),
}
pub fn section_of_stem(stem: &str) -> Option<SectionId>   // derived, exhaustive over the archive
pub fn field_join_failures() -> Vec<String>               // run by BOTH `run()` and the suite
```

The join asserts, over all 246 boxes: a `FieldId` that does not exist **does not compile**; one that
is not a `Field` of the document's own section **reds**; a `CollectedElsewhere` field that IS on the
row **reds**; a `RefuseReason` variant that is deleted **does not compile** (and the closure is
*called*, so it must be constructible); the box's own printed **caption** must appear in the label or
help of a field that collects it; and a `NotRead` with an empty reason **reds**.

`RefuseReason` is stored as `fn() -> RefuseReason` because a `&'static [BoxEntry]` cannot hold a type
with drop glue. `BoxDecision` drops `PartialEq` — Rust's own
`unpredictable_function_pointer_comparisons` says why comparing one is meaningless, and nothing needs
decision equality (the census compares labels and captions).

**Every `NotRead("T5 …")` is gone** (`grep -c 'NotRead("T5' → 0`).

### Census decision table (collected + refusing; the `NotRead` rows are unchanged from T2 except where noted)

**Form W-2** — 32 entries / 21 collected / 2 refuse / 9 not read

| box | caption | decision | field(s) |
|---|---|---|---|
| a | `a Employee's social security number` | CollectedElsewhere | `TpSsn`, `SpSsn` |
| b | `b Employer identification number (EIN)` | Collected | `W2Ein` |
| c | `c Employer's name, address, and ZIP code` | Collected | `W2Employer` |
| e | `e Employee's first name and initial` | CollectedElsewhere | `TpFirstName`, `TpLastName`, `SpFirstName`, `SpLastName` |
| f | `f Employee's address and ZIP code` | CollectedElsewhere | `AddrStreet`, `AddrCity`, `AddrState`, `AddrZip` |
| 1–7, 17, 19 | as printed | Collected | `Box1Wages` … `Box19LocalTax` |
| 8 | `8 Allocated tips` | RefuseIfNonzero | `Box8AllocTips` → `AllocatedTips` |
| 10 | `10 Dependent care benefits` | RefuseIfNonzero | `Box10DepCare` → `DependentCareBenefit` |
| 12a–12d | box-12 slots | CollectedElsewhere | `Box12Code`, `Box12Amount` (the nested section) |
| **13** | `13 Statutory` (2024/25) / `13 employee` (2026) | **Collected** | `W2Box13StatutoryEmployee` |
| **14b** | `14b Treasury Tipped Occupation Code(s)` | **Collected** | `W2Box14bTtoc` |

**Form 1099-INT** — 17 / 7 / 4 / 6

| box | decision | field → reason |
|---|---|---|
| 1, 2, 3, 4, 6, 8 | Collected | as R4's table |
| **10 Market discount** | **Collected** | `Int1099Box10MarketDiscount` → Schedule B line 1 + 1040 2b |
| 9 | RefuseIfNonzero | `Int1099Box9PrivateActivity` → `PrivateActivityBondAmt` |
| **11, 12, 13 Bond premium** | **RefuseIfNonzero** | `Int1099Box11/12/13…` → `AmortizableBondPremiumNotComputed` |

**Form 1099-DIV** — 22 / 7 / 4 / 11. 1a, 1b, 2a, 4, 5, 7, 12 collected; 2b/2c/2d →
`UnrecapturedOrSpecialRateGain`; 13 → `PrivateActivityBondAmt`. **Box 3 stays `NotRead`** with R4's
own reason (*"reduces basis, does not reach a line this year; Pub. 550"*).

**Form 1099-G** — 16 / 3 / 1 / 12. Box 1, **box 2 (new)** and box 4 collected;
**box 10 → `FamilyLeaveBenefits`**. Box 8's reason was rewritten (it no longer says "which is T5's").

**Form 1099-B** — 23 / 4 / 0 / 19. Boxes 1d, 1e, 2 and 12 collected onto the four totals and the gate.

**Form 1098** — 11 / 1 / 0 / 10. Box 1 is `CollectedElsewhere` (`SaMortgage1098`) — **T9's**, untouched.

**Form 1098-E** — 2 / 1 / 0 / 1. Box 1 → `Form1098eBox1Interest` → Schedule 1 line 21.

### FR-65 — the two 2026-only boxes, decided, with the cites

**(a) 1099-G Rev. Dec 2026 box 10 *Family leave benefits* → `RefuseIfNonzero(FamilyLeaveBenefits)`
naming Schedule 1 line 8z.**
Read: `i1099g--2026.txt:17-25` (*"Rev. Rul. 2025-4 requires states to … furnish to an employee a Form
1099 to report family leave benefits"*, and the `10a/10b/11 → 11a/11b/12` renumber) and
`:292-300` (Box 10's own instruction). `i1040gi--2025.txt:280-296` / `:42031-42037` name only the
CONTRIBUTIONS side; no line is named for benefits RECEIVED, and `f1040s1--2026-DRAFT.txt:94` shows
Schedule 1's only home for them is **line 8z, "Other income. List type and amount"** — for which
btctax models no inflow (the same reason 1099-G boxes 5 and 6 are `NotRead`). An income box with no
reader understates, so it fails closed. **It was already `RefuseIfNonzero` at T2 with NO FIELD — a
decision that could not fire.** T5 gave it `Form1099G::box10_family_leave_benefits` and a real screen.

**(b) W-2 2026 box 14b *Treasury Tipped Occupation Code(s)* → `Collected(W2Box14bTtoc)`, joined to
Schedule 1-A line 4a's gate through an advisory.**
`i1040s1a--2025.txt` **does not exist** in the archive (only the form, `f1040s1a--2025.txt`), so the
rule was read off the form's own Caution, `f1040s1a--2025.txt:24-25`: *"Fill out Part II only if you
received qualified tips. These tips must have been received in an occupation listed at
IRS.gov/TippedOccupations."* `i1040gi--2025.txt:43514-43526` names the TTOC as the code identifying
that occupation, and `iw2w3--2026.txt:2931` is the employer's instruction for the box.
Decision: **collect it on the row**, and its reader is the new
`Advisory::TipsDeductionForgoneWithTtoc` — a code on the paper beside an unclaimed Part II is a
**forgone** §224 deduction, i.e. the OVERSTATEMENT direction, which §3.4 makes an advisory and never
a refusal. btctax cannot know from a code how much of box 7 is a *qualified* tip (service charges,
automatic gratuities and unlisted-occupation tips are excluded), so there is no figure a refusal
could demand. FR-65 is marked **CLOSED** in `FOLLOWUPS.md` with both decisions written out.

FR-66 (the join's scope) was **not widened**: `census-join` still reports 13 maps. FR-68 untouched.

---

## 2. The paired document-less income questions (R3)

Four new `FormQuestion`s, `QuestionId::ALL` indices 36–39 (appended, for the `decl_tristate!`
array-index reason), four new `FieldId::Decl*` in the `Declarations` section, all classified class (A)
in `classifier.rs` with no `..` and no `_`:

| question | live iff | `Yes` |
|---|---|---|
| `WagesWithoutW2Question` | `documents.w2 == Some(false)` | refuses `WagesWithoutW2`, naming **Form 1040 LINE 1a** and quoting *"Even if you don't get a Form W-2, you must still report your earnings"* (`i1040gi--2025.txt:2442-2444`) |
| `InterestOrDividendsWithout1099` | `int_1099` **or** `div_1099` is `Some(false)` | opens `schedule_b_filer_records`; empty ⇒ `FilerRecordsDeclaredNotTranscribed` |
| `StateRefundWithout1099g` | `documents.g_1099 == Some(false)` | makes the §111(a) gate live |
| `ItemizedPriorYear` | `StateRefundWithout1099g` is **live and** `Some(true)`, **or** any `g_1099[].box2_state_refund > 0` | refuses `StateAndLocalRefundWorksheetNotComputed`, naming the **State and Local Income Tax Refund Worksheet** |

`ItemizedPriorYear` is **new** — the grep the brief asked for (`itemized`) found no prior-year flag
anywhere in `crates/`, so nothing was moved; it is created return-level from the start.

`StateRefundWithout1099g` and `ItemizedPriorYear` join `RENDERED_PROMPTS`, because the instruction's
own sentence names a YEAR (*"a refund … in 2025"*, and the tax-benefit rule looks at *"the year you
paid the tax"*). That exposed a **latent defect** — see D-1.

`ScheduleBRecord { payer_name, payer_ssn, payer_address, amount, kind }` is `Source::FilerRecords` in
`LEAF_SOURCE`; the rows reach Schedule B lines 1 and 5 and the 1040 2b/3b sums, and
**the seller-financed row is listed FIRST with the buyer's SSN and address in the name column**
(i1040sb: *"list first any interest the buyer paid you … and show that buyer's social security number
(SSN) and address"*).

`sum_taxable_interest` / `sum_ordinary_dividends` now carry box 10 and the filer's records; the
§163(d) Form-4952 ceiling was re-pointed at those shared helpers instead of its own local copy (it had
silently diverged the moment box 10 landed).

---

## 3. Warnings — displayed, never written

New module `crates/btctax-core/src/tax/transcription_warnings.rs`, params-free except the wage base
(passed as `Option`, so it runs while the year's package has not arrived — R11):

- W-2 box 4 ≈ 6.2% × box 3;
- W-2 box 6 **bracketed** to `[1.45% × box 5, 2.35% × box 5]` — the 0.9% Additional Medicare Tax is
  withheld into box 6 too, so a single-rate check would fire on every filer over $200,000 and be
  trained away. The bracket needs no $200,000 threshold of its own;
- W-2 box 3 ≤ `TaxTable::ss_wage_base` (skipped when the table is absent);
- the **all-zero row** on 1099-INT / DIV / G and 1098-E, each message naming that issuer's own
  threshold ($10 / $10 / $10-or-any-refund / $600). **Not on the W-2** — an employer must issue one at
  any wage, so warning there trains the filer to ignore the class.

Tolerance is `max($1, 0.1% of expected)`: these catch a figure in the WRONG BOX, never a cent of
rounding.

Rendered in **`report`** (`render::render_transcription_warnings`, beside the advisories, at
`cmd::tax`'s dual-report site) and **where the row is edited** — `TaxInputsFormState::
transcription_warning_lines()` shows the whole message when the cursor is on the row it is about and a
count naming the rows otherwise; `draw_edit.rs` prints it in the status block.

---

## 4. `EXEMPT_PREFIXES`, the counts, and the five anchors

- `EXEMPT_PREFIXES` **9 → 5**: `int_1099`, `div_1099`, `g_1099`, `b_1099` removed. A new
  `EXEMPT_PREFIX_CEILING = 5` asserts `len() <= ceiling` — **the ratchet**, so a later task may drop
  an entry and never add one back.
- `EXEMPT_LEAVES`: `sch1.student_loan_interest_paid` (the leaf no longer exists) and
  `documents.form_1098e` removed. `documents.form_1098` stays, with T9 named.
- Field count **117 → 175**; distinctly-covered leaves **115 → 174** (every Field but `DocForm1098`,
  whose row is still scalar-shadowed). `EXPECTED_LEAF_PATHS` gained 59 entries.
- `NotInForm` anchors **17 → 12**, a fall of **exactly five**, counted from
  `include_str!("attribute.rs")` **inside `attribute`'s own body** so the test's prose cannot inflate
  it. The five — `PrivateActivityBondAmt`, `UnrecapturedOrSpecialRateGain`,
  `InconsistentDividendSubset`, `ForeignTaxOverCeiling`, `Form1099BNeedsForm8949` — now anchor on the
  boxes that raise them, each asserted to be a real `Field` of a real section.
  **`IraDeductionClaimed` stays `NotInForm`**, asserted positively.
- `document_census.rs`: `declared_rows(Form1098e) = Some(n)`; `requires_transcription(G1099)` and
  `(Form1098e)` **flipped to `true`**; `row_is_live` loses `Form1098e`.

---

## 5. `LEAF_SOURCE` and the manifest

- `DocumentKind::Form1098E` added; `("form_1098e", Document(Form1098E))` and
  `("schedule_b_filer_records", FilerRecords)` added to `LEAF_SOURCE`. Both directions of its KAT pass.
- `provenance::undated_document_rows(ri)` lists every transcribed row with `transcribed_on == None`;
  the packet manifest gains a **`── TRANSCRIBED WITHOUT A DATE ──`** block naming each (document, row
  number, issuer), and prints nothing when every row is dated.

---

## 6. Fixtures

- `coverage.rs::maximal_fixture` carries one row of each new `Vec` (1099-INT/DIV/B/G, 1098-E and one
  `ScheduleBRecord`), with the 1099-G row's `box2_state_refund = 1` as the liveness primer for the
  §111(a) gate. `fixture_for` primes R3's door per field (a census row answered `No`, and `Yes` on the
  interest question for the five filer's-records leaves) — structural, exactly like the §G-9 dates of
  death: one fixture cannot both cover a census row and satisfy the question that row opens.
- `scrub.rs` scrubs the 1098-E lender + TIN through the same `EinMap`, and the **filer's-records
  rows** — including a **third party's SSN**, through `synthetic_ssn_like` so the validity class
  survives. `scrub_axis.rs` gained five matrix rows and the fixture instantiates every one.
- TY2024 example fixtures: the removed scalar held **`"0"`** in all three
  (`testonly.rs::J10_FULLRETURN_TOML`, `nine_dependents_amt_inputs.toml`, `fullreturn_inputs.toml`),
  so **no 1098-E row was needed** — the key was deleted and `documents.form_1098e = false` added, plus
  the now-live door answers. The one place it held a figure was
  `open_next_year_t4b.rs` (`dec!(600)`), which became a real `[[form_1098e]]` row with a documented
  synthetic TIN (`10-1010101`).
- Goldens regenerated: `fullreturn_inputs.toml` (emitter), `docs/examples/examples.md`
  (`xtask examples`), the three `docs/examples-tui-walkthrough/j6/*.txt` (the six new sections in the
  left pane, and the W-2 labels now carrying their captions).

New synthetic identifiers: SSNs `000-00-0001`, `000-44-4444`, `000-55-5555` (area 000 — never issued,
admitted structurally); EINs `11-1111111`, `99-9999999`, `10-1010101` (all already in `ALLOWED_EIN`).
`pii-scan-generic.sh` is clean.

---

## Kills — each seen RED once, with the red quoted

Every plant was applied to a `cp` backup, run, and restored; the tree was re-verified green after all
twenty.

| # | plant | test | the red |
|---|---|---|---|
| M1 | `section_of_stem("f1099int") => Some(W2s)` | `xtask box_census::the_box_to_field_join_is_clean` | `f1099int/1: Int1099Box1Interest is in [Int1099s], not in this document's own section (W2s) — a box collected somewhere else is CollectedElsewhere` |
| M2 | 1099-INT box 1's label/help re-worded to "Interest earned" | same | `f1099int/1: the box prints "interest income", and no field that collects it ([Int1099Box1Interest]) says so` |
| M3 | box-13 screen disabled | `a_checked_statutory_employee_box_refuses_naming_schedule_c_line_1` | `a checked box 13 refuses` (the `expect` on `screen_inputs`) |
| M4 | bond-premium disjunction narrowed to box 11 | `any_bond_premium_box_refuses_naming_pub_550` | `box12_bond_premium_treasury > 0 must refuse` |
| M5 | box 10 dropped from `sum_taxable_interest` | `box_10_market_discount_raises_form_1040_line_2b_by_its_own_amount` | `★ THE KILL: box 10 is a separate addition to line 2b, not a subset of box 1 — left: 2500, right: 2600` |
| M6 | `sum_student_loan_interest` returns ZERO | `derive_applies_student_loan_adjustment_from_the_1098e_rows` | `left: 0, right: 1000` |
| M7 | `Form1099BNeedsForm8949` restored to `NotInForm` | `the_five_reattributed_anchors_…_count_fell_by_five` | `the source now has 13 NotInForm anchors, not 12 — left: 13, right: 12` |
| M8 | `"int_1099"` added back to `EXEMPT_PREFIXES` | `every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt` | `EXEMPT_PREFIXES is a RATCHET and may only shrink: 6 entries, ceiling 5` |
| M9 | `WagesWithoutW2Question.live = \|_\| true` | `each_paired_question_is_live_exactly_on_its_rows_no_and_blocks_there` | `W2 = Yes: the filer HAS the document, so WagesWithoutW2Question must not be asked` |
| M10 | `ItemizedPriorYear.live` reads only the 1099-G rows | `the_prior_year_itemize_gate_is_live_with_no_1099g_row_at_all` | `★ THE KILL: the §111(a) gate must reach a filer whose refund arrived with no document` |
| M11 | `requires_transcription(G1099) => false` | `a_truthful_1099b_on_form_8949_and_a_box_2_only_1099g_both_file` | `left: None, right: Some(DocumentDeclaredNotTranscribed { kind: G1099 })` |
| M12 | `transcription_warnings` returns empty | its two core tests | `the slip breaks the box-4 rate AND the wage base: [] — left: 0, right: 2`; `an all-zero W-2 is ordinary … each information return must: []` |
| M13 | the `render_transcription_warnings` call deleted from `cmd::tax` | `a_box_1_in_box_3_slip_prints_a_transcription_warning_on_the_report_…` | `the slip must be surfaced on the report:` (the whole report, with no block) |
| M14 | the warning loop deleted from `draw_tax_inputs_status` | `a_transcription_warning_is_rendered_in_the_editor_…` | `the slip must be surfaced where the row is edited:` (the whole screen, with no line) |
| M15 | the `undated_rows_block` call deleted | `the_manifest_names_a_document_row_transcribed_without_a_date_…` | `the manifest must carry the block:` (manifest with only the hand-marks section) |
| M16 | the empty-rows condition dropped from the filer's-records rule | `each_paired_question_…` | `a declared record with none refuses` |
| M17 | the TTOC advisory disabled | `a_treasury_tipped_occupation_code_beside_an_unclaimed_part_ii_…` | `assertion left == right failed: [EicOmitted, OtherCreditsOmitted, …]` (0 TTOC advisories, expected 1) |
| M18 | `G1099Box10FamilyLeave` Field deleted | `every_in_scope_leaf_…` | `these IN-SCOPE leaves are covered by NO Field and are NOT in EXEMPT: ["g_1099[0].box10_family_leave_benefits"]` |
| M19 | the 1099-INT box-10 census entry deleted | `every_printed_box_carries_exactly_one_entry` | `box 10 ("10 Market discount") is printed on the form and NOTHING decides it — we forgot this box` |
| M20 | the family-leave screen disabled | `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` | `the commit gate must reach FamilyLeaveBenefits on its own fixture — left: None, right: Some("FamilyLeaveBenefits")` |

The **all-zero census entry / wrong-caption** kills M19 and M2 are the two T2 named as its own; M1
and M2 together are T5's new join, and both were watched red before green.

---

## Every pinned number moved

| pin | old → new | cause |
|---|---|---|
| `coverage.rs` `field_count` | **117 → 175** | 58 new `Field`s (4 door declarations, W-2 boxes 13 and 14b, six document sections) |
| `coverage.rs` `covered.len()` | **115 → 174** | the 58, plus `documents.form_1098e` which had a Field but was not live |
| `coverage.rs` `EXEMPT_PREFIXES` | **9 → 5** entries, new ceiling **5** | the four 1099 sections landed |
| `attribute.rs` `NotInForm` | **17 → 12** | the five re-attributed anchors |
| `questions.rs` `QuestionId::ALL.len()` / `FORM_QUESTIONS.len()` | **36 → 40** | R3's four door questions |
| `spec/mod.rs` `decl_count` / `decls.fields.len()` | **16 → 20** / **17 → 21** | the same four, in `Declarations` |
| `main.rs` section-cursor clamp | **11 → 17** | six new sections |
| `tax_report.rs` `answer_log` record count | **30 → 34** | the 1098-E census row + the three door questions |
| `answer.rs` Single-filer census list | 16 → **17** rows | `DocForm1098e` opened |
| `return_refuse.rs` `countable` | 5 → **6** kinds | `Form1098e` gained a `Vec` |
| `return_refuse.rs` `demanding` | `{W2, INT, DIV}` → **`{W2, INT, DIV, G, 1098-E}`** | box 2 and the 1098-E rows gave both a field to transcribe into |
| `line-coverage` | **341 / 24 / 0 / 12 — UNCHANGED** | no printed line's PRODUCTION changed: Schedule 1 line 21 and 1040 2b/3b changed their INPUT, not their map entry |
| `census-join` | **13 maps — UNCHANGED** | FR-66 deliberately not widened |
| `box-census` | 246 boxes / 15 editions / **123 → 123** entries | no entry added or removed; 49 decisions rewritten |

---

## Deviations and judgment calls

**D-1 — a latent defect fixed as collateral: `screen_inputs` compared a RENDERED prompt against the
STATIC one.** `record_answer` hashes the words actually shown (`prompt_text`), and
`provenance::current_prompt` documents in terms that reading the static `prompt` *"would hand every
surface a hash that never changes when the value does"* — but `screen_inputs_tiered` passed
`q.prompt`. Any question with a rendered prompt was therefore reported **wording-changed the instant
it was answered**. It was latent because `FilingStatusConfirmed` was the only such question and no
test screened a return after answering it; T5's two year-quoting prompts made it fire immediately
(`a_pre_d8_vault_refuses_until_answered…` red with *"the wording of this question changed"*). Fixed to
`&q.prompt_text(ri)` with the reasoning recorded at the site. **This is a refusal that fired on a
correct answer** — worth flagging to the reviewer as the one behaviour change outside T5's brief.

**D-2 — `income answer` now SWEEPS instead of asking from a snapshot.** R3's door makes a question
live *because another was answered*, so a single pass over `live_questions` ends the run with the new
question unanswered and the filer running the command again to reach a question their own answer
created. `answer_return_inputs` now re-derives the live set until it stops growing (each item asked at
most once, keyed by `AnswerKey`; bounded at 8 sweeps, which refuses loudly on a liveness cycle). Three
test scripts that hand-counted keystrokes were replaced by helpers that simulate the same sweep —
a count from one snapshot cannot be right even in principle now.

**D-3 — the census join runs inside `xtask box-census`, not only in the suite.** Written first as two
tests; moved into `field_join_failures()`, shared by `run()` and one test. `xtask box-census` is what a
human types when a form changes, and a check only the suite performs arrives a round late. (This is
why `xtask` is 158 → 157 tests.)

**D-4 — the W-2 field labels are now the box captions verbatim.** The caption check has to be
universal or it is a hand-list of exemptions; `Box1Wages`'s label was *"Box 1 — wages"* and its help
said *"other comp."* where the form prints *"other compensation"*. Eleven W-2 labels and two help
strings were rewritten to carry the caption, with the reach moved into the help. This moved the TUI
walkthrough goldens.

**D-5 — box 14b is `Collected` with an ADVISORY as its reader, not a refusal and not
`CollectedElsewhere`.** The brief allowed *"join it to Schedule 1-A line 4a's tips gate … or refuse
when non-empty"*. Refusing would brick every tipped filer whose Schedule 1-A Part II arrives by TOML
today. Pointing it at `schedule_1a.tips.treasury_occupation_code` would have been laundering:
**nothing reads that leaf** (`Schedule1aPartII::compute` takes only `qualified_tips_reported`), so
calling the box "collected" there would be the *figure-with-no-reader* shape. The advisory is a real
reader in the only direction §3.4 permits silently.

**D-6 — the §111(a) gate decides the refund refusal, not the question's `Yes` alone.** R3 says *"`Yes`
refuses naming the State and Local Income Tax Refund Worksheet, **the same exit 1099-G box 2 takes**"*,
and R4 spells that exit out: `itemized_prior_year = No ⇒ line 1 blank by decision; Yes ⇒ refuse`.
Refusing on the question's `Yes` alone would refuse a standard-deduction filer whose refund is not
income at all — the over-refusing shape the T3 seam review's I2/I3 was about. Both exits are still
BLOCKING (`ItemizedPriorYearUnanswered`), and the worksheet is named in the refusal.

**D-7 — the interest/dividend question is live on EITHER row's `No`.** One question is paired with two
census rows (§5.1 lists three questions for four rows). A filer with 1099-INTs and no 1099-DIV can
still hold a nominee distribution, so `int == Some(false) || div == Some(false)`.

**D-8 — a 1098-E row is NOT pre-namable.** `open_next_year` seeds W-2 employers, 1099 payers and
venues, never loan servicers, so `row_is_pre_named(Form1098e, …)` is `false` and a `No` beside a
transcribed row correctly REFUSES rather than deleting testimony. Asserted positively in the two
`drop_pre_named_rows` tests so the arm cannot become a silent skip.

**D-9 — `scrub_axis`'s `payer_tin @ malformed` cell stayed `NoSuchState`.** Its comment predicted
*"when T5 gives it a reader, this cell becomes a `Fixture`"*. **T5 did not**: the screens it built
refuse on box VALUES, never on the shape of a payer TIN. The comment is rewritten to record the
prediction as spent rather than leave it looking like an open promise.

**D-10 — the all-zero-row warning does not cover the W-2**, deliberately: an employer must issue one
at any wage, so the ISSUER-THRESHOLD reasoning R4 gives (*"a payer issues a 1099-INT at $10 or
more"*) does not apply, and warning there would train the filer to ignore the class. Stated in the
module header and asserted in the test.

## Residue for the reviewer

1. **The prompt-hash fix (D-1) is a behaviour change outside T5's stated scope.** It is a refusal that
   fired on a correct answer, so leaving it would have shipped a brick — but it deserves its own look.
2. **`Schedule1aTips::occupation_on_treasury_list` has no reader.** `Schedule1aPartII::compute` takes
   `qualified_tips_reported` alone, so the Caution's gate is collected and never enforced. That is
   Schedule 1-A's own task, not T5's, and D-5 is why box 14b did not point at it — but it is a live
   `figure-with-no-reader` and probably wants a follow-up.
3. **The sweep's ordering (D-2)** puts R3's door questions after the skippables, because a question
   that exists only because of an answer must come after that answer. A T12 pass over `income answer`'s
   presentation may want to group them with the census instead.
4. Six existing test fixtures now carry a transcription warning in `report` output (W-2s with
   `box4 = box6 = 0` beside non-zero wages). That is the check working on anomalous fixtures, not a
   false positive — a $0 box 4 beside $40,000 of Social Security wages forgoes a credit — but a future
   fixture pass may want to give them real withholding.
