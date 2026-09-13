# The `btctax income import` TOML schema

**GENERATED — do not edit `docs/income-import-schema.md`.** Regenerate it with:

```
cargo run -p xtask -- toml-schema > docs/income-import-schema.md
```

`xtask::toml_schema::tests::the_committed_schema_matches_a_fresh_generation` reds when this file is stale, so a field added to `ReturnInputs` cannot reach a release unpublished.

## What this file is

`btctax income import --year <Y> --file <f.toml>` reads one TOML file describing a whole federal return — the household, every document transcribed off paper, the schedules, and the payments. It is the authoring surface for anything the interview (`btctax income answer`) cannot express, and the surface every *"use `income import` instead"* refusal points at.

Start from [the worked example](#a-complete-worked-example) below and delete what does not apply to you.

**Almost every key is optional to the PARSER** — `ReturnInputs` carries `#[serde(default)]` on nearly every field — so a short file parses. **63 are not**, and they are measured rather than remembered — each one was deleted from a complete file and handed back to the deserializer:

- `b_1099[].payer`
- `capital_loss_carryforward_in.long`
- `capital_loss_carryforward_in.short`
- `charitable_carryover_in[].amount`
- `charitable_carryover_in[].class`
- `charitable_carryover_in[].origin_year`
- `div_1099[].box1a_ordinary`
- `div_1099[].payer`
- `filing_status`
- `form_1098[].box1_interest`
- `form_1098[].lender`
- `form_1098e[].lender`
- `g_1099[].box1_unemployment`
- `g_1099[].payer`
- `header.dependents[].name`
- `header.dependents[].relationship`
- `header.dependents[].ssn`
- `header.direct_deposit.account`
- `header.direct_deposit.routing`
- `header.spouse.first_name`
- `header.spouse.last_name`
- `header.spouse.ssn`
- `header.taxpayer.first_name`
- `header.taxpayer.last_name`
- `header.taxpayer.ssn`
- `int_1099[].box1_interest`
- `int_1099[].payer`
- `sa_1099[].payer`
- `sa_5498[].trustee`
- `schedule_a.charitable[].amount`
- `schedule_a.charitable[].class`
- `schedule_a.mortgage_interest_not_on_1098[].amount`
- `schedule_a.mortgage_interest_not_on_1098[].recipient_name`
- `schedule_b_filer_records[].payer_name`
- `schedule_c.owner`
- `state_local_refund.exception_could_be_claimed_as_dependent`
- `state_local_refund.exception_joint_state_return_not_joint_now`
- `state_local_refund.exception_last_estimated_payment_in_filing_year`
- `state_local_refund.exception_not_an_income_tax_refund`
- `state_local_refund.exception_owed_amt_in_prior_year`
- `state_local_refund.exception_refund_exceeds_incremental_deduction`
- `state_local_refund.exception_refund_for_another_year`
- `state_local_refund.exception_unusable_credits`
- `state_local_refund.exception_zero_rate_on_preferential_income`
- `state_local_refund.mfs_spouse_boxes_permitted`
- `state_local_refund.prior_year_aged_blind`
- `state_local_refund.prior_year_aged_blind.spouse_aged`
- `state_local_refund.prior_year_aged_blind.spouse_blind`
- `state_local_refund.prior_year_aged_blind.taxpayer_aged`
- `state_local_refund.prior_year_aged_blind.taxpayer_blind`
- `state_local_refund.prior_year_filing_status`
- `state_local_refund.prior_year_mfs_spouse_itemized`
- `state_local_refund.prior_year_schedule_a_line17`
- `state_local_refund.prior_year_schedule_a_line5d`
- `state_local_refund.prior_year_schedule_a_line5e`
- `state_local_refund.provenance`
- `state_local_refund.refund_not_on_a_1099g`
- `w2s[].box12[].amount`
- `w2s[].box12[].code`
- `w2s[].box1_wages`
- `w2s[].box2_fed_withheld`
- `w2s[].employer`
- `w2s[].owner`

A key marked **required** in the table below cannot be omitted while its parent table is present; leaving it out is a TOML parse error, not a refusal, so it is reported differently and earlier — and note how many of them are `[[table]]` row fields, which is what makes *"required when its parent is present"* the load-bearing qualifier: a return with no W-2 needs none of the `w2s[]` keys.

What decides whether the return is COMPLETE is a separate gate, which refuses at import time with a named reason and the form's own words — see [Refusals you should expect](#refusals-you-should-expect). Parsing and completeness are two different instruments and this document does not conflate them.

## Three things that will cost you time

**1. A bare key written after a `[[table]]` header belongs to THAT TABLE.** This is TOML's own rule, not a btctax quirk, and it is the easiest way to lose an answer:

```toml
[[form_1098]]
lender = "Example Bank"
box1_interest = "12400"

# WRONG — this is now `form_1098.0.claiming_mortgage_interest_credit`,
# a key that does not exist, and the import is REFUSED by name.
claiming_mortgage_interest_credit = false
```

A return-level or section-level key must appear **before** the first `[table]`/`[[table]]` header that follows it, or under the table it actually belongs to. btctax catches this one rather than silently dropping it (trap 3), but only once you have run the import.

**2. The document census must agree with the rows you supplied.** `[documents]` is one answer per document type, and it is not a derivation: `w2s` being empty means either *no W-2* or *nobody asked*, and those are the same blank on a printed page. So `int_1099 = false` beside a `[[int_1099]]` row is refused as a **contradiction** — btctax cannot know which of the two is wrong, so it refuses rather than choose. Answer every row of `[documents]` explicitly, including the `false`s: a recorded "none" is the honest record.

**3. An unknown key is REFUSED, by name — it is never ignored.** `income import` runs `serde_ignored` over the same serde shape this document is generated from, so a typo, or a field renamed or removed in a later version, stops the import and is printed:

```
unknown key(s) in the ReturnInputs TOML: form_1098.0.claiming_mortgage_interest_credit. btctax does not honor these — likely a typo or a field removed in this version … a silently-ignored key would drop data you meant to enter.
```

That is the behaviour to rely on: if the import does not complain about a key, every key in your file was read.

## Refusals you should expect

An import that stops has written nothing. Each refusal names its reason and quotes the form:

- a **census contradiction** (trap 2 above);
- an **unanswered declaration** — a yes/no box with no safe default. Mortgage interest with `claiming_mortgage_interest_credit` left out is refused with Schedule A's own Line 8a Caution quoted and cited; guessing `false` would print an unsubtracted line 8a the filer never affirmed. (Note the path: it is a **return-level** key, not a `schedule_a` one — the document arrives whether or not you itemize — so writing it after `[[form_1098]]` or `[schedule_a]` hits trap 1.) Answer these in the TOML, or run `btctax income answer --year <Y>`;
- a **year whose return cannot compute yet** is NOT refused — it is stored with a note, so `report --write-carryover` can write onto it before that year's package exists.

Two key groups are read and then **normalised away**, with a note on stderr rather than a refusal, because `income scrub` emits them and btctax must be able to read a file it writes:

- `answer_log` / `answer_log_history` — btctax's record of *when, and in what words, it asked you*. A hand-written record would be provenance for an act this vault never observed, so the import discards what the file carried (announcing how many) and keeps whatever is already on the row. Answer the questions with `income answer` instead.
- every `*_provenance` key — forced to `user`. A carryover the file supplies is the filer's, never btctax's own `computed` authorship.

## Every key `income import` honours

**389 paths, 352 of them leaves that take a value.** Derived from the serialized shape of `ReturnInputs` over `btctax_core::tax::scrub_axis::maximal_sentinel()` — the fixture whose every `Option` is `Some`, every `Vec` non-empty and every nested struct present, written as an exhaustive `..`-free struct literal so a new field is a compile error before it can be an unpublished key.

**Reading the paths.** `a.b` is the key `b` under `[a]`. `a[]` is a repeated table, written `[[a]]` once per row, and `a[].b` is a key inside one of those rows. A `<placeholder>` segment is a key YOU choose, not a literal:

- `broker_reporting.<provider>` — the exchange/provider key, exactly as `report --tax-year` lists it.
- `answer_log.<answer-key>` — btctax's own record of asking — NOT importable, see below.

**Money is a string.** Every dollar figure is a decimal serialized as a quoted string — `"12400"`, `"1234.56"` — never a bare number, because a TOML float cannot carry a cent exactly.

**A DATE takes EITHER spelling, and the 14 `date` leaves below were found by trying one.** Write `date_of_birth = "2012-04-15"` — that is what the deserializer accepts and what you should type. btctax's own `income scrub` writes the same value as `time`'s compact `[2012, 106]` (year, ordinal day), which also reads back, so a scrubbed file round-trips without editing. Each of those leaves was identified by substituting an ISO date string at the path and re-parsing the whole file. A leaf still shown below as `array of integers` was NOT confirmed that way — every one of them is under `answer_log*`, which the import discards anyway, and the generator fails if such a leaf ever turns up anywhere else.

**The last column** is `required` when omitting the key breaks the parse (measured, see above), plus what the import does with it BEYOND storing it (derived from the path). Blank means optional and stored as given.

| key | TOML type | notes |
|---|---|---|
| `amt_carryover_same_as_regular` | boolean |  |
| `amt_depreciation_same_as_regular` | boolean |  |
| `answer_log` | table | **discarded** on import |
| `answer_log.<answer-key>` | table | **discarded** on import |
| `answer_log.<answer-key>.answered_on` | array of integers | **discarded** on import |
| `answer_log.<answer-key>.answered_on[]` | integer | **discarded** on import |
| `answer_log.<answer-key>.prompt_hash` | string | **discarded** on import |
| `answer_log.<answer-key>.state` | string | **discarded** on import |
| `answer_log_history` | array | **discarded** on import |
| `answer_log_history[]` | array | **discarded** on import |
| `answer_log_history[][]` | table | **discarded** on import |
| `answer_log_history[][].answered_on` | array of integers | **discarded** on import |
| `answer_log_history[][].answered_on[]` | integer | **discarded** on import |
| `answer_log_history[][].prompt_hash` | string | **discarded** on import |
| `answer_log_history[][].state` | string | **discarded** on import |
| `b_1099` | array of tables |  |
| `b_1099[]` | table |  |
| `b_1099[].basis_reported_and_no_adjustments` | boolean |  |
| `b_1099[].box13_bartering` | string |  |
| `b_1099[].long_term_basis` | string |  |
| `b_1099[].long_term_proceeds` | string |  |
| `b_1099[].payer` | string | **required** |
| `b_1099[].payer_tin` | string |  |
| `b_1099[].short_term_basis` | string |  |
| `b_1099[].short_term_proceeds` | string |  |
| `b_1099[].transcribed_on` | date |  |
| `broker_reporting` | table |  |
| `broker_reporting.<provider>` | table |  |
| `broker_reporting.<provider>.covered` | string |  |
| `broker_reporting.<provider>.noncovered` | string |  |
| `capital_loss_carryforward_in` | table |  |
| `capital_loss_carryforward_in.long` | string | **required** |
| `capital_loss_carryforward_in.short` | string | **required** |
| `capital_loss_carryforward_in_provenance` | string | forced to `user` |
| `carryover_includes_spouses_joint_loss` | boolean |  |
| `charitable_carryover_in` | array of tables |  |
| `charitable_carryover_in[]` | table |  |
| `charitable_carryover_in[].amount` | string | **required** |
| `charitable_carryover_in[].class` | string | **required** |
| `charitable_carryover_in[].origin_year` | integer | **required** |
| `charitable_carryover_in[].provenance` | string | forced to `user` |
| `charitable_carryover_in_provenance` | string | forced to `user` |
| `charitable_cwa_obtained` | boolean |  |
| `claiming_mortgage_interest_credit` | boolean |  |
| `digital_asset_activity` | boolean |  |
| `div_1099` | array of tables |  |
| `div_1099[]` | table |  |
| `div_1099[].box10_noncash_liquidation` | string |  |
| `div_1099[].box12_exempt_interest_dividends` | string |  |
| `div_1099[].box13_private_activity_amt` | string |  |
| `div_1099[].box1a_ordinary` | string | **required** |
| `div_1099[].box1b_qualified` | string |  |
| `div_1099[].box2a_capgain_distr` | string |  |
| `div_1099[].box2b_unrecap_1250` | string |  |
| `div_1099[].box2c_section_1202` | string |  |
| `div_1099[].box2d_collectibles_28` | string |  |
| `div_1099[].box4_fed_withheld` | string |  |
| `div_1099[].box5_section_199a` | string |  |
| `div_1099[].box7_foreign_tax` | string |  |
| `div_1099[].box9_cash_liquidation` | string |  |
| `div_1099[].payer` | string | **required** |
| `div_1099[].payer_tin` | string |  |
| `div_1099[].transcribed_on` | date |  |
| `documents` | table |  |
| `documents.a_1095` | boolean |  |
| `documents.b_1099` | boolean |  |
| `documents.c_1099` | boolean |  |
| `documents.div_1099` | boolean |  |
| `documents.form_1098` | boolean |  |
| `documents.form_1098e` | boolean |  |
| `documents.g_1099` | boolean |  |
| `documents.int_1099` | boolean |  |
| `documents.k1` | boolean |  |
| `documents.nec_misc_k_1099` | boolean |  |
| `documents.oid_1099` | boolean |  |
| `documents.r_1099` | boolean |  |
| `documents.s_1099` | boolean |  |
| `documents.sa_1099` | boolean |  |
| `documents.sa_5498` | boolean |  |
| `documents.schedule_e_rental` | boolean |  |
| `documents.ssa_1099` | boolean |  |
| `documents.t_1098` | boolean |  |
| `documents.w2` | boolean |  |
| `documents.w2g` | boolean |  |
| `donations_had_restrictions` | boolean |  |
| `dual_status_alien` | boolean |  |
| `excluded_canceled_debt` | boolean |  |
| `excluded_puerto_rico_income` | string |  |
| `fbar_filing_required` | boolean |  |
| `filing_form_4952` | boolean |  |
| `filing_status` | string | **required** |
| `filing_status_confirmed` | boolean |  |
| `foreign_accounts` | boolean |  |
| `foreign_country_names` | string |  |
| `foreign_trust` | boolean |  |
| `form_1098` | array of tables |  |
| `form_1098[]` | table |  |
| `form_1098[].box10_other` | string |  |
| `form_1098[].box1_interest` | string | **required** |
| `form_1098[].box2_outstanding_principal` | string |  |
| `form_1098[].box3_origination_date` | date |  |
| `form_1098[].box4_refund_overpaid_interest` | string |  |
| `form_1098[].box5_mortgage_insurance` | string |  |
| `form_1098[].box6_points` | string |  |
| `form_1098[].box7_property_address_same_as_payer` | boolean |  |
| `form_1098[].box8_property_address` | string |  |
| `form_1098[].lender` | string | **required** |
| `form_1098[].lender_tin` | string |  |
| `form_1098[].other_borrower_paid_interest` | boolean |  |
| `form_1098[].transcribed_on` | date |  |
| `form_1098e` | array of tables |  |
| `form_1098e[]` | table |  |
| `form_1098e[].box1_interest` | string |  |
| `form_1098e[].lender` | string | **required** |
| `form_1098e[].lender_tin` | string |  |
| `form_1098e[].transcribed_on` | date |  |
| `form_2555_line45` | string |  |
| `form_2555_line50` | string |  |
| `form_4563_line15` | string |  |
| `form_8960_line9b` | string |  |
| `g_1099` | array of tables |  |
| `g_1099[]` | table |  |
| `g_1099[].box10_family_leave_benefits` | string |  |
| `g_1099[].box1_unemployment` | string | **required** |
| `g_1099[].box2_state_refund` | string |  |
| `g_1099[].box4_fed_withheld` | string |  |
| `g_1099[].box5_rtaa_payments` | string |  |
| `g_1099[].box6_taxable_grants` | string |  |
| `g_1099[].box7_agriculture_payments` | string |  |
| `g_1099[].box9_market_gain` | string |  |
| `g_1099[].payer` | string | **required** |
| `g_1099[].payer_tin` | string |  |
| `g_1099[].transcribed_on` | date |  |
| `has_income_exclusion` | boolean |  |
| `header` | table |  |
| `header.address_city` | string |  |
| `header.address_state` | string |  |
| `header.address_street` | string |  |
| `header.address_zip` | string |  |
| `header.can_be_claimed_as_dependent_spouse` | boolean |  |
| `header.can_be_claimed_as_dependent_taxpayer` | boolean |  |
| `header.dependents` | array of tables |  |
| `header.dependents[]` | table |  |
| `header.dependents[].citizen_national_or_resident_alien` | boolean |  |
| `header.dependents[].citizen_national_resident_or_canada_mexico` | boolean |  |
| `header.dependents[].date_of_birth` | date |  |
| `header.dependents[].divorced_separated_multiple_support_or_kidnapped_rule_applies` | boolean |  |
| `header.dependents[].filing_joint_return` | boolean |  |
| `header.dependents[].full_time_student` | boolean |  |
| `header.dependents[].gross_income_under_limit` | boolean |  |
| `header.dependents[].joint_return_only_to_claim_refund` | boolean |  |
| `header.dependents[].lived_with_you_in_us` | boolean |  |
| `header.dependents[].lived_with_you_over_half_year` | boolean |  |
| `header.dependents[].married` | boolean |  |
| `header.dependents[].name` | string | **required** |
| `header.dependents[].permanently_and_totally_disabled` | boolean |  |
| `header.dependents[].provided_over_half_own_support` | boolean |  |
| `header.dependents[].qc_relationship` | boolean |  |
| `header.dependents[].qr_relationship_or_member_of_household` | boolean |  |
| `header.dependents[].qualifying_child_of_another_person` | boolean |  |
| `header.dependents[].qualifying_child_of_any_taxpayer` | boolean |  |
| `header.dependents[].relationship` | string | **required** |
| `header.dependents[].ssn` | string | **required** |
| `header.dependents[].ssns_valid_for_employment_issued_by_due_date` | boolean |  |
| `header.dependents[].tin_issued_by_due_date` | boolean |  |
| `header.dependents[].you_provided_over_half_support` | boolean |  |
| `header.dependents[].younger_than_you_or_spouse` | boolean |  |
| `header.direct_deposit` | table |  |
| `header.direct_deposit.account` | string | **required** |
| `header.direct_deposit.kind` | string |  |
| `header.direct_deposit.routing` | string | **required** |
| `header.filer_tin_issued_by_due_date` | boolean |  |
| `header.foreign_country` | string |  |
| `header.foreign_postal_code` | string |  |
| `header.foreign_province` | string |  |
| `header.form8615_condition3_age_support` | boolean |  |
| `header.form8615_condition4_parent_alive` | string |  |
| `header.form8615_parent_identity_unobtainable` | boolean |  |
| `header.hoh_marital_basis` | string |  |
| `header.hoh_paid_over_half_cost_of_keeping_up_home` | boolean |  |
| `header.hoh_qualifying_person` | boolean |  |
| `header.ip_pin` | string |  |
| `header.nra_spouse_resident_election` | boolean |  |
| `header.phone` | string |  |
| `header.presidential_fund_spouse` | boolean |  |
| `header.presidential_fund_taxpayer` | boolean |  |
| `header.qss_child_lived_in_your_home_all_year` | boolean |  |
| `header.qss_child_you_can_claim` | boolean |  |
| `header.qss_could_have_filed_jointly_in_year_of_death` | boolean |  |
| `header.qss_paid_over_half_cost_of_keeping_up_home` | boolean |  |
| `header.qss_spouse_died_in_window_and_not_remarried` | boolean |  |
| `header.qualifying_child_name` | string |  |
| `header.spouse` | table |  |
| `header.spouse.blind` | boolean |  |
| `header.spouse.date_of_birth` | date |  |
| `header.spouse.date_of_death` | date |  |
| `header.spouse.first_name` | string | **required** |
| `header.spouse.last_name` | string | **required** |
| `header.spouse.occupation` | string |  |
| `header.spouse.ssn` | string | **required** |
| `header.spouse_died_during_year` | boolean |  |
| `header.spouse_had_no_income` | boolean |  |
| `header.spouse_ip_pin` | string |  |
| `header.spouse_not_filing_a_return` | boolean |  |
| `header.taxpayer` | table |  |
| `header.taxpayer.blind` | boolean |  |
| `header.taxpayer.date_of_birth` | date |  |
| `header.taxpayer.date_of_death` | date |  |
| `header.taxpayer.first_name` | string | **required** |
| `header.taxpayer.last_name` | string | **required** |
| `header.taxpayer.occupation` | string |  |
| `header.taxpayer.ssn` | string | **required** |
| `header.taxpayer_died_during_year` | boolean |  |
| `home_sale` | table |  |
| `home_sale.can_exclude_all_gain` | boolean |  |
| `home_sale.sold_main_home` | boolean |  |
| `home_sale.test1_owned_2_years_and_lived_2_years_of_last_5` | boolean |  |
| `home_sale.test2_no_exclusion_on_another_home_in_2_years` | boolean |  |
| `hsa` | table |  |
| `hsa.employer_contributions_next_year` | string |  |
| `hsa.employer_contributions_prior_year` | string |  |
| `hsa.line10_qualified_funding_distribution` | string |  |
| `hsa.line14b_rollovers_and_withdrawn_excess` | string |  |
| `hsa.line15_qualified_medical_expenses` | string |  |
| `hsa.line16_amount_meeting_an_exception` | string |  |
| `hsa.line2_contributions_you_made` | string |  |
| `hsa_distribution_without_1099sa` | boolean |  |
| `int_1099` | array of tables |  |
| `int_1099[]` | table |  |
| `int_1099[].box10_market_discount` | string |  |
| `int_1099[].box11_bond_premium` | string |  |
| `int_1099[].box12_bond_premium_treasury` | string |  |
| `int_1099[].box13_bond_premium_tax_exempt` | string |  |
| `int_1099[].box1_interest` | string | **required** |
| `int_1099[].box2_early_withdrawal_penalty` | string |  |
| `int_1099[].box3_treasury_interest` | string |  |
| `int_1099[].box4_fed_withheld` | string |  |
| `int_1099[].box6_foreign_tax` | string |  |
| `int_1099[].box8_tax_exempt_interest` | string |  |
| `int_1099[].box9_private_activity_bond_amt` | string |  |
| `int_1099[].payer` | string | **required** |
| `int_1099[].payer_tin` | string |  |
| `int_1099[].transcribed_on` | date |  |
| `interest_or_dividends_without_1099` | boolean |  |
| `itemize_election` | string |  |
| `itemized_prior_year` | boolean |  |
| `mfs_spouse_itemizes` | boolean |  |
| `opened_from` | integer |  |
| `other_out_of_scope_income` | boolean |  |
| `payments` | table |  |
| `payments.estimated_tax_payments` | string |  |
| `payments.extension_payment` | string |  |
| `payments.other_withholding` | string |  |
| `prior_year_elected_sales_tax` | boolean |  |
| `qbi` | table |  |
| `qbi.qbi_carryforward_in` | string |  |
| `qbi.qbi_carryforward_in_provenance` | string | forced to `user` |
| `qbi.reit_ptp_carryforward_in` | string |  |
| `qbi.reit_ptp_carryforward_in_provenance` | string | forced to `user` |
| `sa_1099` | array of tables |  |
| `sa_1099[]` | table |  |
| `sa_1099[].box1_gross_distribution` | string |  |
| `sa_1099[].box2_earnings_on_excess` | string |  |
| `sa_1099[].box3_distribution_code` | string |  |
| `sa_1099[].box4_fmv_on_date_of_death` | string |  |
| `sa_1099[].box5_account_type` | string |  |
| `sa_1099[].payer` | string | **required** |
| `sa_1099[].payer_tin` | string |  |
| `sa_1099[].transcribed_on` | date |  |
| `sa_5498` | array of tables |  |
| `sa_5498[]` | table |  |
| `sa_5498[].box1_archer_msa_contributions` | string |  |
| `sa_5498[].box2_total_contributions` | string |  |
| `sa_5498[].box3_contributions_next_year_for_this_year` | string |  |
| `sa_5498[].box4_rollover_contributions` | string |  |
| `sa_5498[].box5_fair_market_value` | string |  |
| `sa_5498[].box6_account_type` | string |  |
| `sa_5498[].transcribed_on` | date |  |
| `sa_5498[].trustee` | string | **required** |
| `sa_5498[].trustee_tin` | string |  |
| `sch1` | table |  |
| `sch1.hsa_activity` | boolean |  |
| `sch1.ira_deduction_claimed` | string |  |
| `sch1.state_refund_taxable` | string |  |
| `schedule_1a` | table |  |
| `schedule_1a.vehicles` | array of tables |  |
| `schedule_1a.vehicles[]` | table |  |
| `schedule_1a.vehicles[].description` | string |  |
| `schedule_1a.vehicles[].excludes_negative_equity` | boolean |  |
| `schedule_1a.vehicles[].final_assembly_in_us` | boolean |  |
| `schedule_1a.vehicles[].interest_paid` | string |  |
| `schedule_1a.vehicles[].is_applicable_vehicle_class_under_14000_lbs` | boolean |  |
| `schedule_1a.vehicles[].loan_originated_after_2024` | boolean |  |
| `schedule_1a.vehicles[].loan_originated_by_you` | boolean |  |
| `schedule_1a.vehicles[].original_use_starts_with_you` | boolean |  |
| `schedule_1a.vehicles[].personal_use` | boolean |  |
| `schedule_1a.vehicles[].proceeds_used_to_purchase` | boolean |  |
| `schedule_1a.vehicles[].secured_by_first_lien` | boolean |  |
| `schedule_a` | table |  |
| `schedule_a.charitable` | array of tables |  |
| `schedule_a.charitable[]` | table |  |
| `schedule_a.charitable[].amount` | string | **required** |
| `schedule_a.charitable[].class` | string | **required** |
| `schedule_a.investment_interest` | string |  |
| `schedule_a.medical` | string |  |
| `schedule_a.mortgage_all_used_to_buy_build_improve` | boolean |  |
| `schedule_a.mortgage_dwelling_is_amt_qualified` | boolean |  |
| `schedule_a.mortgage_interest_not_on_1098` | array of tables |  |
| `schedule_a.mortgage_interest_not_on_1098[]` | table |  |
| `schedule_a.mortgage_interest_not_on_1098[].amount` | string | **required** |
| `schedule_a.mortgage_interest_not_on_1098[].recipient_address` | string |  |
| `schedule_a.mortgage_interest_not_on_1098[].recipient_name` | string | **required** |
| `schedule_a.mortgage_interest_not_on_1098[].recipient_tin` | string |  |
| `schedule_a.mortgage_within_debt_limit` | boolean |  |
| `schedule_a.points_not_on_1098` | string |  |
| `schedule_a.salt_personal_property` | string |  |
| `schedule_a.salt_prior_year_balance_paid` | string |  |
| `schedule_a.salt_real_estate` | string |  |
| `schedule_a.salt_sales_tax_amount` | string |  |
| `schedule_a.salt_state_estimated_payments` | string |  |
| `schedule_a.salt_use_sales_tax` | boolean |  |
| `schedule_b_filer_records` | array of tables |  |
| `schedule_b_filer_records[]` | table |  |
| `schedule_b_filer_records[].amount` | string |  |
| `schedule_b_filer_records[].kind` | string |  |
| `schedule_b_filer_records[].payer_address` | string |  |
| `schedule_b_filer_records[].payer_name` | string | **required** |
| `schedule_b_filer_records[].payer_ssn` | string |  |
| `schedule_c` | table |  |
| `schedule_c.accounting_method` | string |  |
| `schedule_c.business_description` | string |  |
| `schedule_c.expenses` | string |  |
| `schedule_c.is_cooperative_patron` | boolean |  |
| `schedule_c.is_sstb` | boolean |  |
| `schedule_c.naics_code` | string |  |
| `schedule_c.other_gross_receipts` | string |  |
| `schedule_c.owner` | string | **required** |
| `schedule_c.payments_requiring_1099` | boolean |  |
| `schedule_c.qbi_ubia` | string |  |
| `schedule_c.qbi_w2_wages` | string |  |
| `schedule_c.will_file_required_1099` | boolean |  |
| `state_local_refund` | table |  |
| `state_local_refund.exception_could_be_claimed_as_dependent` | boolean | **required** |
| `state_local_refund.exception_joint_state_return_not_joint_now` | boolean | **required** |
| `state_local_refund.exception_last_estimated_payment_in_filing_year` | boolean | **required** |
| `state_local_refund.exception_not_an_income_tax_refund` | boolean | **required** |
| `state_local_refund.exception_owed_amt_in_prior_year` | boolean | **required** |
| `state_local_refund.exception_refund_exceeds_incremental_deduction` | boolean | **required** |
| `state_local_refund.exception_refund_for_another_year` | boolean | **required** |
| `state_local_refund.exception_unusable_credits` | boolean | **required** |
| `state_local_refund.exception_zero_rate_on_preferential_income` | boolean | **required** |
| `state_local_refund.mfs_spouse_boxes_permitted` | boolean | **required** |
| `state_local_refund.prior_year_aged_blind` | table | **required** |
| `state_local_refund.prior_year_aged_blind.spouse_aged` | boolean | **required** |
| `state_local_refund.prior_year_aged_blind.spouse_blind` | boolean | **required** |
| `state_local_refund.prior_year_aged_blind.taxpayer_aged` | boolean | **required** |
| `state_local_refund.prior_year_aged_blind.taxpayer_blind` | boolean | **required** |
| `state_local_refund.prior_year_filing_status` | string | **required** |
| `state_local_refund.prior_year_mfs_spouse_itemized` | boolean | **required** |
| `state_local_refund.prior_year_schedule_a_line17` | string | **required** |
| `state_local_refund.prior_year_schedule_a_line5d` | string | **required** |
| `state_local_refund.prior_year_schedule_a_line5e` | string | **required** |
| `state_local_refund.provenance` | string | **required**; forced to `user` |
| `state_local_refund.refund_not_on_a_1099g` | string | **required** |
| `state_refund_without_1099g` | boolean |  |
| `tax_year` | integer |  |
| `w2_wages_without_w2` | boolean |  |
| `w2s` | array of tables |  |
| `w2s[]` | table |  |
| `w2s[].box10_dependent_care` | string |  |
| `w2s[].box12` | array of tables |  |
| `w2s[].box12[]` | table |  |
| `w2s[].box12[].amount` | string | **required** |
| `w2s[].box12[].code` | string | **required** |
| `w2s[].box13_statutory_employee` | boolean |  |
| `w2s[].box14b_treasury_tipped_occupation_codes` | string |  |
| `w2s[].box17_state_tax_withheld` | string |  |
| `w2s[].box19_local_tax` | string |  |
| `w2s[].box1_wages` | string | **required** |
| `w2s[].box2_fed_withheld` | string | **required** |
| `w2s[].box3_ss_wages` | string |  |
| `w2s[].box4_ss_withheld` | string |  |
| `w2s[].box5_medicare_wages` | string |  |
| `w2s[].box6_medicare_withheld` | string |  |
| `w2s[].box7_ss_tips` | string |  |
| `w2s[].box8_allocated_tips` | string |  |
| `w2s[].ein` | string |  |
| `w2s[].employer` | string | **required** |
| `w2s[].owner` | string | **required** |

## A complete worked example

A married-filing-jointly household with wages, interest, dividends, an itemized Schedule A and Bitcoin dispositions. This is journey J10's fixture (`btctax_cli::testonly::J10_FULLRETURN_TOML`), reproduced VERBATIM — the same bytes `docs/examples/examples.md` drives through a real `income import`, so it is a file known to work rather than one that reads as if it should.

★ Note that it answers **every** row of `[documents]`, including the `false`s.

```toml
filing_status = "Mfj"
foreign_accounts = false
foreign_trust = false
foreign_country_names = ""
dual_status_alien = false
# ★ §G-21 — Form 8283 5a/5b/5c, asked once for the whole return. Harmless here (no donations); it is
# MANDATORY only on a year that files a Section B, i.e. donations over $5,000.
donations_had_restrictions = false
# Schedule D line 20 / Schedule A line 9 — not filing Form 4952.
filing_form_4952 = false
has_income_exclusion = false
other_out_of_scope_income = false
itemize_election = "auto"
charitable_carryover_in = []
# ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. The census rows for 1099-INT / 1099-DIV / 1099-G are
#     "none" below, which makes these two questions LIVE: the instructions name interest and
#     dividends nobody issues a 1099 for (a bank paying under $10, a nominee distribution) and a
#     state refund reportable "even if you didn't receive Form 1099-G". This household has neither.
#     (The wage question is not live: this household HOLDS two Forms W-2.)
interest_or_dividends_without_1099 = false
state_refund_without_1099g = false
# ★★★ R9 / T6 — Form 1040 page 1's DIGITAL ASSETS question. Mandatory on every return ("You must
#     answer the digital asset question on Form 1040 whether or not you received a Form 1099-DA"),
#     and this household SOLD 2 BTC in 2024, so the answer is Yes — which is also what its own
#     ledger witnesses, so the T6 cross-check agrees.
digital_asset_activity = true

[capital_loss_carryforward_in]
long = "0"
short = "0"

# ★★★ R8 / T9 — the sale of a main home. ALWAYS asked, and a blank is not a "no": the Schedule D
#     instructions' own answer is "You may not need to report the sale or exchange of your main
#     home", and which branch a filer is on decides whether a Form 8949 belongs on the return.
[home_sale]
sold_main_home = false

# ★★★ R3 / §5.1 — THE DOCUMENT CENSUS. One tri-state per document TYPE; an unanswered row refuses,
# because "none" and "nobody asked" are the same blank on the printed page and are not the same
# testimony. This household holds two W-2s and nothing else.
[documents]
w2 = true
int_1099 = false
div_1099 = false
b_1099 = false
g_1099 = false
r_1099 = false
ssa_1099 = false
nec_misc_k_1099 = false
k1 = false
schedule_e_rental = false
s_1099 = false
oid_1099 = false
w2g = false
c_1099 = false
a_1095 = false
t_1098 = false
# ★ T5 — the 1098-E row opened when `Form1098E` replaced the `sch1.student_loan_interest_paid`
#   scalar. This household holds no student loan, so the answer is a truthful "none".
# ★ R8 / T9 — no Form 1098 (this household takes the standard deduction), so the row is not
# even live; answered anyway, because a stated "none" is the honest record.
form_1098 = false
form_1098e = false
# ★ T16 — the two HSA information returns opened with Form 8889. This household has no health
#   savings account, so both answers are a truthful "none" — and `sch1.hsa_activity = false` below
#   says the same thing about the §223 triggers, which is why no Form 8889 files.
sa_1099 = false
sa_5498 = false

[header]
address_street = "88 Larkspur Way"
address_city = "Boulder"
address_state = "CO"
address_zip = "80301"
can_be_claimed_as_dependent_taxpayer = false
can_be_claimed_as_dependent_spouse = false
presidential_fund_taxpayer = false
presidential_fund_spouse = false
taxpayer_died_during_year = false
spouse_died_during_year = false
# ★★★ FR-67 / R7 — the §6013(g)/(h) NONRESIDENT-ALIEN-SPOUSE election gate, live on any return that
#   carries a spouse. Neither of the Okafors is an alien, so the answer is a truthful "no"; a "yes"
#   would refuse, because the election puts the alien spouse's WORLDWIDE income on the return and
#   btctax collects none of it.
nra_spouse_resident_election = false

[header.taxpayer]
first_name = "Nina"
last_name = "Okafor"
occupation = "Anesthesiologist"
ssn = "123-45-6789"

[header.spouse]
first_name = "Tomas"
last_name = "Okafor"
occupation = "Structural Engineer"
ssn = "987-65-4321"

[payments]
estimated_tax_payments = "0"
extension_payment = "0"
other_withholding = "0"

[qbi]
reit_ptp_carryforward_in = "0"
reit_ptp_carryforward_in_provenance = "user"
qbi_carryforward_in = "0"
qbi_carryforward_in_provenance = "user"

[sch1]
hsa_activity = false
ira_deduction_claimed = "0"
state_refund_taxable = "0"

[[w2s]]
owner = "taxpayer"
employer = "Front Range Health"
box1_wages = "400000"
box2_fed_withheld = "96000"
box3_ss_wages = "168600"
box4_ss_withheld = "10453.20"
box5_medicare_wages = "400000"
box6_medicare_withheld = "7150"
box7_ss_tips = "0"
box8_allocated_tips = "0"
box10_dependent_care = "0"
box12 = []
box17_state_tax_withheld = "0"
box19_local_tax = "0"
```
