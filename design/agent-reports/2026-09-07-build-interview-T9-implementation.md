# T9 (real estate — R8) — build report

Branch `main`, dispatched at `e396ae0e`. **Nothing committed, nothing pushed.** 49 files changed,
3701 insertions / 269 deletions.

---

## Commands, with their real output

```
$ cargo run -q -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
f1040s3:5 f1040sa:21 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12 f8959:17
f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)

$ cargo run -q -p xtask -- census-join
census join: 283 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -q -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 91 registry prompts scanned;
no forbidden shape

$ cargo run -q -p xtask -- prompt-check
xtask prompt-check: OK — 88 assertions, all verbatim

$ cargo run -q -p xtask -- box-census
  f1098--2022 (i1098 instructions): 11 boxes — 8 collected, 1 refuse-if-nonzero, 2 not read
  f1098--2025 (i1098 instructions): 11 boxes — 8 collected, 1 refuse-if-nonzero, 2 not read
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)

$ bash scripts/pii-scan-generic.sh
pii-scan: clean (HEAD).

$ cargo fmt --all                                        # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
                                                          # clean, exit 0
$ make check
     Summary [  19.640s] 3469 tests run: 3469 passed, 12 skipped     # (3470 after the last kill)
$ echo $?
0
```

**Final suite: `3470 passed / 0 failed / 12 skipped`** (HEAD was 3455/12 — **+15 tests**).

---

## What landed, per numbered item of the brief

### 1. `Form1098` as a top-level repeating section; `mortgage_interest_1098` removed

`return_inputs.rs` — `Form1098 { lender, lender_tin, transcribed_on, box1_interest,
box2_outstanding_principal, box3_origination_date: Option<Date>, box4_refund_overpaid_interest,
box5_mortgage_insurance, box6_points, box7_property_address_same_as_payer: bool,
box8_property_address, box10_other, other_borrower_paid_interest: Option<bool> }`, on
`ReturnInputs.form_1098` (top-level: the document arrives whether or not the filer itemizes).
`ScheduleAInputs::mortgage_interest_1098` is **gone**.

Three derivations, each with exactly one home, on `impl ReturnInputs`:

- `form_1098_interest_and_points()` = **Σ(box 1 + box 6)** → Schedule A line 8a. The line's own
  caption is *"Home mortgage interest **and points** reported to you on Form 1098"* and its
  instruction says the same (`i1040sca--2025.txt:1060-1061`); **box 6** is *"Points paid on purchase
  of principal residence"*, read off the archived face (`f1098--2022`/`--2025`, both editions).
- `form_1098_outstanding_principal()` = Σ box 2 → the ceiling check.
- `mortgage_interest_not_on_1098_total()` = Σ the 8b rows.

**Liveness (R8/I8).** `questions::mortgage_question_live` is now
`schedule_a.is_some() && !form_1098.is_empty()`; `document_census::row_is_live(Form1098)` is
`schedule_a.is_some()`. `declared_rows` counts the `Vec`, `requires_transcription` flipped to `true`
(the 1099-G's exact reason: the excuse was a scalar, and the scalar is gone), `row_is_pre_named` and
`drop_pre_named_rows` gained real arms.

`SectionId::Form1098s` with 13 `Field`s, every box's `help` carrying the caption **verbatim from
`xtask box-census`'s reading of the extract**, plus what the box reaches.

### 2. The ceiling warning, the box-4 refusal, the shared-interest refusal, box 5 → 8d

**`tables.rs` — `AcquisitionDebtCeiling`**, a new `FullReturnParams` field. All **four** printed
figures transcribed rather than two plus a halving rule:

| field | value | source |
|---|---|---|
| `after_dec_15_2017` | $750,000 | *"Limit on loans taken out after December 15, 2017 … up to $750,000"* — `i1040sca--2025.txt:1040-1042` |
| `after_dec_15_2017_mfs` | $375,000 | *"($375,000 if you are married filing separately)"* — `:1041-1042` |
| `on_or_before_dec_15_2017` | $1,000,000 | *"Limit on loans taken out on or before December 15, 2017 … up to $1,000,000"* — `i1040sca--2025.txt:1027-1030` |
| `on_or_before_dec_15_2017_mfs` | $500,000 | *"($500,000 if you are married filing separately)"* — `:1029-1030` |

Not indexed and not expiring: Pub. L. 119-21 (OBBBA) **§70108(a)** struck *", and before January 1,
2026"* from §163(h)(3)(F)(i) and re-headed it *"BEGINNING AFTER 2017"*
(`legal/text/statute-irc/PLAW-119publ21_OBBBA.txt:5211-5229`). Present on **both** bundled packages
(TY2024 and the TY2026 constants function), and on all three test-side `FullReturnParams` literals —
including `testonly::ty2024_params()`, which is the independently-transcribed *validated* side of
`shipped_tables_are_the_validated_tables`, so the two sides are compared.

`transcription_warnings::acquisition_debt_ceiling_warning(ri, Option<AcquisitionDebtCeiling>)` is the
ONE derivation: **aggregate** Σ box 2, measured against the **smallest** ceiling any row earns, halved
for MFS, `None` box 3 taking the stricter post-2017 limit (fail-closed — an unknown date can make the
warning fire sooner, never later). It **writes nothing**; `MortgageWithinDebtLimit` stays the filer's
testimony and still refuses on `None` and on `Some(false)`.

Displayed on **three** surfaces: `report` (`transcription_warnings`, now a 3-arg call taking the
ceiling from the year's package), the tax-inputs editor (both `SectionId::Form1098s` **and**
`SectionId::ScheduleA`, which is where the declaration is answered), and `income answer` — printed as
a line **before** the `MortgageWithinDebtLimit` prompt, never folded into it (T7's C-1: the prompt is
what `record_answer` hashes, so a computed figure inside it would re-ask a question whose answer had
not gone stale). The hash is asserted to be the registry's words.

**Box 4 → `MortgageInterestRefundNotComputed`**, param-free, naming Schedule 1 line 8z. The
instruction is explicit that the refund is not netted against the deduction
(`i1040sca--2025.txt:1069-1072`). **Shared interest** → `SharedMortgageInterest` on `Some(true)` and
`SharedMortgageInterestUnanswered` on `None` (deviation D-2 below). **Box 5** collected against the
reserved 8d, with the OBBBA §70108 restoration and the draft TY2026 Schedule A's own *"8d Mortgage
insurance premiums"* / *"8e Add lines 8a through 8d"* (`f1040sa--2026-DRAFT.txt:100-102`) recorded on
the field, the census entry and `printed.rs`.

### 3. Schedule A 8b rows, 8c, 8e = 8a + 8b + 8c, the map cells, the line-coverage productions

`NonForm1098Interest { recipient_name, recipient_tin, recipient_address, amount }` as
`ScheduleAInputs.mortgage_interest_not_on_1098` (its own repeating section
`SectionId::NonForm1098Interest`), and `points_not_on_1098: Usd` for 8c (the Schedule A section's
`SaPointsNotOn1098`, which replaced `SaMortgage1098`). An empty `recipient_tin` refuses
`NonForm1098InterestRecipientUnidentified`, quoting the $50-penalty Caution
(`i1040sca--2025.txt:1109-1119`).

`ScheduleAParts` gained `mortgage_8b`, `mortgage_8c`, `mortgage_8b_payee: Vec<String>`;
`ScheduleALines` gained `line8b`, `line8c`, `line8b_payee`; `line8e = line8a + line8b + line8c` and
line 10 / line 17 follow. **Both structs lost `Copy`** (they now carry text the form prints).

Map cells, **measured with `xtask dump-fields` on the bundled blanks, not read off the render**:

| year | 8b amount | 8b dotted lines | 8c |
|---|---|---|---|
| 2024 | `f1_19` (y=360) | `f1_17` (y=348), `f1_18` (y=336), x=[115.2, 396.0] | `f1_20` (y=312) |
| 2025 | `f1_17` (y=324) | `Line8b_ReadOrder[0].f1_16` (y=[300,324], x=[122.4, 381.6]) | `f1_18` (y=276) |

The emitter gained a **fourth x-cluster** for the dotted lines — `COL_PAYEE`, band `[245, 266]` on the
x-**centre** (`verify_flat` bands the centre: TY2024 255.6, TY2025 252.0) — and its own descent group,
because the payee rows sit between two money cells in y. More recipients than the form has dotted
lines prints the instruction's own *"See attached"* (`i1040sca--2025.txt:1126-1131`).

`line-coverage` productions: **8a** `doc_box_slots("f1098", "1", &["6"])` (it reads TWO boxes and the
caption says so), **8b** and **8c** `FilerRecords` quoting the instruction's own sentences
(`:1102-1104`, `:1136-1140`), **8e** `Combine` on *"Add lines 8a through 8c"*.

### 4. The Form 8396 gate

`ReturnInputs.claiming_mortgage_interest_credit`, `QuestionId::ClaimingMortgageInterestCredit`
(`FORM_QUESTIONS` index 61, `FieldId::DeclClaimingMortgageInterestCredit`), live iff any 1098 row or
any 8b row, neutral at `false`. `Some(true)` refuses `MortgageInterestCreditUnsupported`, naming Form
8396 line 3 (`i1040sca--2025.txt:1091-1096`).

### 5. `home_sale: HomeSale`

`HomeSale { sold_main_home, test1_owned_2_years_and_lived_2_years_of_last_5,
test2_no_exclusion_on_another_home_in_2_years, can_exclude_all_gain }`, its own singleton
`SectionId::HomeSale`, four registry questions (indices 62–65). `sold_main_home` is **always live and
NOT neutral**; the three tests are live iff it is `Yes`. No amount is asked.

**The branch table as implemented and tested** (`s_1099` is the census row):

| test 1 | test 2 | can exclude all | `s_1099` | outcome |
|---|---|---|---|---|
| Y | Y | Y | `Some(false)` | **blank by decision**, the four answers on record |
| Y | Y | N | `Some(false)` | `HomeSaleNotComputed("you said you cannot exclude all of your gain")` |
| Y | N | Y/N | `Some(false)` | `HomeSaleNotComputed("… did not meet Test 2 …")` |
| N | * | Y/N | `Some(false)` | `HomeSaleNotComputed("… did not meet Test 1 …")` (or the exclusion arm, which is checked first) |
| Y | Y | Y | `Some(true)` | `DocumentTypeUnsupported { S1099 }` — the census's own §2.2 sentence, which already names **Form 8949 code H and the Pub. 523 worksheet** |
| Y | Y | Y | `None` | `DocumentCensusUnanswered { S1099 }` at commit; `HomeSaleNotComputed("this return does not say whether a Form 1099-S arrived")` at import |

Seven of the eight test-triples refuse with `HomeSaleNotComputed`, every message naming **PUB. 523**
and **code H**; exactly one is blank. Both 1099-S dimensions are asserted, so a reordering that lost
one is visible.

### 6. Fixtures

- `coverage.rs::maximal_fixture` gained one `form_1098` row (also the liveness primer for the three
  declarations and the 8396 gate) and one 8b row; `addr_for` and `fixture_for` gained the new
  sections; **`documents.form_1098` left `EXEMPT_PREFIXES`** — the last uncovered leaf in the whole
  spec, closed by T9 (248 of 249 covered → **271 of 271**).
- `scrub_axis.rs`: `f1098(tag, tin)` with four identity leaves, two 8b rows with a third party's SSN,
  the home sale on its blank branch, the 8396 gate answered.
- `testonly.rs`: `form_1098_with_interest(interest)` (the fixture form of the removed scalar — box 4
  zero, shared-interest gate answered), the kitchen sink's $22,000 moved to a 1098 row, the oracle
  corpus's `e19200`/`A8a` figure moved to a 1098 row (the sweep reconciles unchanged).
- The two committed example TOMLs migrated; `fullreturn_inputs.toml` regenerated from its emitter.
- 50 test-side `ScheduleAInputs { mortgage_interest_1098: … }` sites migrated (script-driven, then
  five hand-fixed where the closure shape needed it).

---

## The Form 1098 box census, per edition

Both archived editions — **Rev. January 2022** (in force TY2022–TY2024) and **Rev. April 2025**
(TY2025 on) — print the identical eleven captions, so one entry per box lists both editions and one
struct serves both. Per edition: **8 collected, 1 refuse-if-nonzero, 2 not read.**

| box | caption (verbatim, from the extract) | decision |
|---|---|---|
| 1 | `1 Mortgage interest received from payer(s)/borrower(s)` | `Collected(Form1098Box1Interest)` → Schedule A 8a with box 6 |
| 2 | `2 Outstanding mortgage` | `Collected(Form1098Box2Principal)` → the AGGREGATE §163(h)(3)(B) warning; no printed line |
| 3 | `3 Mortgage origination date` | `Collected(Form1098Box3OriginationDate)` → which ceiling |
| 4 | `4 Refund of overpaid` | `RefuseIfNonzero(MortgageInterestRefundNotComputed)` — Schedule 1 line 8z |
| 5 | `5 Mortgage insurance` | `Collected(Form1098Box5MortgageInsurance)` → 8d, reserved for TY2024/25, restored TY2026 |
| 6 | `6 Points paid on purchase of principal residence` | `Collected(Form1098Box6Points)` → 8a with box 1 |
| 7 | `7 If address of property securing mortgage is the same` | `Collected(Form1098Box7AddressSame)` — a checkbox; reaches no line |
| 8 | `8 Address or description of property securing mortgage (see` | `Collected(Form1098Box8PropertyAddress)` — scrubbed as identity |
| 9 | `9 Number of properties securing the` | `NotRead` — no Schedule A line reads it; the ceiling reads box 2 |
| 10 | `10 Other` | `Collected(Form1098Box10Other)` — free text; the help points a filer whose lender reported property tax at Schedule A 5b |
| 11 | `11 Mortgage` | `NotRead` — the ACQUISITION date; the §163(h)(3)(B) test reads box 3 |

`section_of_stem("f1098")` is now `Some(SectionId::Form1098s)`, so **every archived document has a
form section** — the 1098 was the last one out, and its excuse expired with the scalar.

---

## Every kill, with its planted defect and the observed red

Fourteen plants, each run against the real test and restored from a `cp` backup.

**1 — 8e is 8a alone.** `printed.rs`: `let line8e = line8a + line8b + line8c;` → `= line8a;`
```
panicked at printed.rs:2863: assertion `left == right` failed: ★ THE KILL: "Add lines 8a through 8c" — 12,700 + 940 + 300
  left: 12700
 right: 13940
```

**2 — the box-4 rule cannot fire.** `if false && m.box4_refund_overpaid_interest > Usd::ZERO`
```
panicked at return_refuse.rs:3771: a box-4 refund refuses on BOTH tiers
```

**3 — the shared-interest BLANK is silently a "no."** the `None` arm made unreachable
```
panicked at return_refuse.rs:3811: assertion `left == right` failed: a BLANK is not a "no": line 8a
sums box 1 in full, so silence would claim it all
  left: None
 right: Some(SharedMortgageInterestUnanswered)
```

**4 — the declarations lose the itemize-election conjunct.** `mortgage_question_live` →
`!ri.form_1098.is_empty()`
```
panicked at return_refuse.rs:3852: MortgageAllUsedToBuyBuildImprove must NOT be live on a
standard-deduction return
```

**4b — the census row loses it.** `row_is_live(Form1098)` → `true`
```
panicked at return_refuse.rs:3857: the `form_1098` census row is live iff the filer itemizes
```

**5 — the 8b identity rule cannot fire.** `if false && r.recipient_tin.trim().is_empty()`
```
panicked at return_refuse.rs:3946: an unidentified 8b recipient refuses on BOTH tiers
```

**6 — the 8396 rule cannot fire.**
```
panicked at return_refuse.rs:3988: the mortgage interest credit refuses on BOTH tiers
```

**7 — the home-sale "cannot exclude all the gain" branch is dropped.**
```
panicked at return_refuse.rs:4050: (true,true,false) must not be the blank branch
```

**8 — the ceiling becomes a strict `<`.** `if total <= limit` → `if total < limit`
```
panicked at transcription_warnings.rs:436: assertion `left == right` failed: AT the ceiling is not
over it — the instruction says "up to $750,000"
  left: Some("the outstanding mortgage principal in box 2 of the 1 Form 1098 on this return adds up
        to $750000, which is more than the $750000 the §163(h)(3)(B) limit allows …")
 right: None
```

**9 — the ceiling becomes PER-ROW instead of aggregate.** `Σ box 2` → `max box 2`
```
panicked at transcription_warnings.rs:460: TWO of them are $1,000,000 of acquisition debt and must
warn on the SUM
```

**10 — an untranscribed box 3 defaults to the PRE-2018 (larger) ceiling.** `is_some_and` →
`is_none_or`
```
panicked at transcription_warnings.rs:498: an UNTRANSCRIBED box 3 takes the stricter post-2017
ceiling — fail-closed
```

**11 — MFS is not halved.** `(false, Mfs) => after_dec_15_2017_mfs` → `after_dec_15_2017`
```
panicked at transcription_warnings.rs:484: $400,000 is over the $375,000 MFS ceiling
```

**12 — the box-4 rule cannot fire, watched from the IMPORT side.**
```
panicked at year_gate_t4.rs:105: a box-4 refund must refuse at import: ()
```

**13 — the ceiling display beside the declaration is removed.**
`if false && q.id == QuestionId::MortgageWithinDebtLimit`
```
panicked at tax_report.rs:3466: no ceiling warning on the screen:
```

**14 — the opener's Form 1098 arms, observed red BEFORE they existed.** The seed test was written
first and failed on the real gap twice, which is why both halves are here:
```
panicked at open_next_year_t4b.rs:1596: the prompt names the lender and its TIN:
    Last year  issued you a Form 1098. Did  issue one for 2025?     ← `payer_of` had no arm
panicked at open_next_year_t4b.rs:1604: assertion `left == right` failed: the lender identity is
    carried;  left: 0  right: 1                                     ← `seed` did not carry it
```

**The box census's own three plants** run inside
`the_1098_entries_red_on_a_deleted_box_a_drifted_caption_and_a_narrowed_edition`, driving the REAL
eleven entries against BOTH real editions: a deleted box-4 entry (*"we forgot this box"*), a
one-character caption drift `Outstanding` → `Outstandng` (*"caption does not match the extract"*), and
an entry narrowed to the other edition (*"we forgot this box"* naming *Mortgage origination date*).
All three are `expect_err`, so the test passing IS the red having been observed on each.

**New tests (15).** `a_1098_box_4_refund_refuses_naming_schedule_1_line_8z_and_a_zero_does_not`,
`the_shared_interest_gate_refuses_on_yes_and_on_a_blank_but_not_on_no`,
`a_standard_deduction_filer_with_a_900k_1098_is_asked_nothing_and_refuses_nothing`,
`a_line_8b_row_with_no_identifying_number_refuses_and_one_with_a_number_files`,
`the_form_8396_gate_refuses_on_yes_and_is_not_even_asked_without_mortgage_interest`,
`the_home_sale_table_is_one_blank_and_seven_refusals_naming_pub_523`,
`the_acquisition_debt_warning_fires_over_the_ceiling_and_is_silent_under_it`,
`the_acquisition_debt_warning_sums_every_1098_row`,
`the_acquisition_debt_warning_halves_for_mfs_and_reads_the_origination_date`,
`the_ceiling_warning_answers_nothing_and_waits_for_the_years_package`,
`schedule_a_line_8e_adds_8a_8b_and_8c_from_the_return`,
`the_1098_entries_red_on_a_deleted_box_a_drifted_caption_and_a_narrowed_edition`,
`a_form_1098_lender_is_seeded_as_an_identity_with_every_box_blank`,
`a_1098_box_4_refund_refuses_at_import_naming_line_8z_and_writes_no_row`,
`the_acquisition_debt_ceiling_is_shown_beside_the_debt_limit_question`.

---

## Every pinned number moved, old → new, with cause

| pin | old | new | cause |
|---|---|---|---|
| `line-coverage` money lines (f1040sa) | 373 (19) | **375 (21)** | 8b and 8c are `Collected` lines now |
| `line-coverage` exceptions / unverifiable / not-line-bound | 31 / 0 / 17 | **unchanged** | — |
| `census-join` unmodeled entries | 290 | **283** | 4 TY2024 + 3 TY2025 Schedule A cells left the census for the map |
| `stop-list` registry prompts | 86 | **91** | five new `FORM_QUESTIONS` |
| `prompt-check` assertions | 88 | **unchanged** | — |
| `box-census` totals | 268 boxes / 19 editions / 9 returns | **unchanged** | the boxes were already enumerated; their DECISIONS changed |
| `f1098` per edition | 1 collected / 0 refuse / 10 not read | **8 / 1 / 2** | ten `NotRead("T9 …")` placeholders became real decisions |
| `QuestionId::ALL` / `FORM_QUESTIONS` | 61 | **66** | the 8396 gate + four home-sale answers |
| `Declarations` `Decl*` fields | 39 | **40** | the 8396 gate (the four home-sale answers dedup to `HomeSale`) |
| `DECLARATIONS.fields.len()` | 40 | **41** | ditto + `foreign_country_names` |
| deduped declarations | 2 | **6** | the two Schedule-A mortgage boxes + T9's four home-sale answers |
| form-spec `Field` count | 249 | **271** | 13 (1098) + 4 (8b) + 4 (home sale) + 1 (8396); 8c swapped for `SaMortgage1098` |
| distinctly-covered leaves | 248 | **271** | `documents.form_1098` left `EXEMPT_PREFIXES` — the last gap |
| TUI last live section index | 20 | **23** | three new sections |
| answer-log records in the re-import kill | 37 | **38** | `SoldMainHome` is always live |
| `ScheduleAMap::lines()` / the emitter plan | 19 | **21** | 8b and 8c |
| the Schedule A forms fixture | 8e 12000 / 10 12000 / 17 28000 | **13200 / 16200 / 32200** | 8b = 900, 8c = 300 now print |
| `the_rows_invariant` countable / demanding rows | 8 / 6 | **9 / 7** | the 1098 gained a `Vec` and a transcription demand |
| suite | 3455 / 12 | **3470 / 12** | +15 tests |

---

## Deviations from the brief and the spec, with reasoning

**D-1 — the TY2024 map cells are not the pair R8 names.** R8 says *"the four TY2024 map cells
`f1_17`/`f1_19`"* (8b) and *"`f1_18`/`f1_20`"* (8c). **Measured** with `xtask dump-fields` on the
bundled blank: `f1_19` (y=360) is the 8b AMOUNT; `f1_17` (y=348) and `f1_18` (y=336) are **both** 8b's
wide free-text rows (x=[115.2, 396.0]); `f1_20` (y=312) is line 8c, which has **no description cell on
this form at all**. Both map headers already said so; the `[census]` reason strings did not, and the
one calling `f1_18` *"line 8c description"* was simply wrong. Mapped from the measurement, with the
measurement recorded in the map.

**D-2 — the shared-interest gate refuses on `None` as well as on `Some(true)`.** R8 states only the
`Some(true)` refusal. A blank read as *"no co-borrower"* is btctax claiming the whole box 1 on line 8a
on the filer's behalf — testimony nobody gave, in the understatement direction. The precedent is
exact: the Form 1099-B line-1a gate refuses on `None` for the same reason. Classified as
`Class::BenefitClaim` with that reason stated, and the `None` half is kill-tested (plant 3). Its exit
is real rather than a wall — the gate is a field of the document row, set exactly where the row is.

**D-3 — boxes 7 and 8 are two fields.** The brief's summary says `box7_8_property_address`; §5.2 gives
`box7_property_address_same_as_payer: bool` and `box8_property_address: String`. §5.2 is followed: they
are two printed boxes, and box 7 is a checkbox the LENDER ticked.

**D-4 — the `s_1099 = Some(true)` branch refuses through the census, not `HomeSaleNotComputed`.**
`screen_document_census` runs before every value-dependent rule by design. Its §2.2 sentence for the
1099-S already names Form 8949 code H and the Pub. 523 worksheet — the same exit, one rule earlier and
more specific. The `s_1099` conjunct is still in the home-sale rule so it is complete on its own terms
at import, and both paths are asserted.

**D-5 — `ScheduleAParts` and `ScheduleALines` are no longer `Copy`.** Line 8b's dotted lines are text
the form prints, so the printed-lines struct carries it (the precedent is `ScheduleBLines::part1_rows`).
Two call sites needed `as_ref()`/reordering; no behaviour change.

**D-6 — more 8b recipients than dotted lines prints *"See attached"*.** The instruction's own escape,
printed on this very line (`i1040sca--2025.txt:1126-1131`). The emitter chooses between two strings the
form supplies and composes neither. **No packet-manifest item was added for the attachment** — a
follow-up candidate for the controller (the `hand_marks` block takes only `&PrintedReturn`, and wiring
it needs a reader for `ScheduleALines::line8b_payee` there).

**D-7 — line 8a's `line-coverage` row names TWO boxes.** `doc_box_slots("f1098", "1", &["6"])`. 8a is
Σ(box 1 + box 6), so naming box 1 alone would state a narrower provenance than the line has — the same
*"the label is narrower than what is read"* defect the `also_labels` slot was added for. Its doc
comment was widened from *"a repeating box"* to cover both shapes.

**D-8 — three instrument/helper repairs this build had to make, each a latent defect it surfaced.**

1. **`scrub.rs` treated `schedule_a` as *"money only"*.** Schedule A line 8b names a THIRD PARTY by
   name, **SSN** and address — on a seller-financed mortgage the identifying number is an
   individual's SSN. The destructure arm is now `schedule_a` and the rows are scrubbed through the
   same `EinMap` every payer TIN uses; three matrix rows were added to `scrub_axis`, whose
   completeness check is derived from `replaced_paths` and would otherwise have reported drift.
   (Form 1098 box 8, the property address, and box 10's free text are scrubbed for the same reason.)
2. **`xtask::label_reader::line_bindings` silently dropped ARRAY-valued line bindings.** `line8b_payee
   = [ … ]` produced zero bindings while `numbered_line_keys` counted the key — the exact *"partial
   silent drop"* shape `map_reach_problem` exists to catch, and the reason it caught this one. The
   extractor now reads single-line and multi-line arrays.
3. **`open_next_year`'s `payer_of` and `seed` had no Form 1098 arm.** `payer_of`'s `match` is
   deliberately exhaustive-with-no-`_` for exactly this, but `Form1098` sat in the "no section"
   bucket; the seed test caught both halves before either existed (plant 14).

**D-9 — `testonly::reconcile_document_census` gained a second one-direction rule.** A row that is
LIVE, unanswered and has zero rows is answered `Some(false)`. `form_1098` is the first census row
whose LIVENESS a fixture's shape decides, so a builder that answers every live declaration BEFORE its
shape adds a Schedule A cannot reach it. Only rows with a `Vec` to count are touched, so the §2.2
families keep whatever the fixture stated.

**D-10 — TY2026's ceiling is present, not absent.** The brief said *"TY2026 absent"*. §163(h)(3)(B)'s
figures are **statute** (and OBBBA §70108 made the $750,000 limit permanent), not a form question, so
by the repo's own rule (Fable plan review I4: constants that are *"statute and Rev. Proc., not form"*
settle now) they are on the TY2026 constants function too. `full_return_for(2026)` stays `None`, so no
TY2026 return sees them yet — the check simply does not run on a year with no package.

---

## Regenerated goldens (each diff read before accepting)

- `docs/examples/examples.md` — `income show` now prints `form_1098` rows, the census row `true`, the
  8b/8c leaves and the `home_sale` block; `mortgage_interest_1098` is gone.
- `docs/examples-tui-walkthrough/j6/*.txt` — three new sections in the left pane, in §4.1's order:
  *Forms 1098 (home mortgage interest)*, *Schedule A*, *Schedule A line 8b*, *Sale of your main home*.
- `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` — regenerated from its emitter.

## Oracles

The two-oracle sweep (`check_mode_reconciles_every_line_of_the_anchors_and_pinned_cells`) passes
unchanged: the corpus's `itemized_deductions + mortgage_interest` figure, which OTS reads as `A8a` and
Tax-Calculator as `e19200`, now travels through a `form_1098` row's box 1 to the same line 8a. The
corpus carries no 8b/8c, so the sweep exercises 8a only — per R13 those absorb into `e19200` when a
corpus grows them.

## Not done / open

- The packet manifest gains no *"attach a statement for line 8b"* item (D-6).
- `income project --year N` (R13) and the R12 panel are not T9's.
