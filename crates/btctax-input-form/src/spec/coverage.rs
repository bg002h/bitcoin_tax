//! ★ Task 6 — the coverage KAT (spec §5.6). A drift-proofing test: every IN-SCOPE `ReturnInputs`
//! leaf-path must be covered by exactly one `Field`'s `set` (observed, not declared), or be listed in the
//! explicit `EXEMPT` surface (the §5.8 deferred structs/leaves). A newly-added struct field on an in-scope
//! struct — not covered, not exempt — makes the two sets unequal, so the build goes red until someone gives
//! it a form `Field` (or exempts it deliberately). That standing bite is the whole point of the task.
//!
//! ★ **Mechanism — mutate-and-diff (drift-proof).** The covered-path set is DERIVED BY OBSERVATION, never
//! hand-declared: for each `Field` in `form_spec()`, clone a MAXIMALLY-POPULATED fixture, apply the Field's
//! `set` with a per-kind sentinel, and record which serde_json leaf path(s) actually changed. The union is
//! the covered set. A hand-written `FieldId → path` table could silently drift from what the accessors
//! really touch AND would not perturb when a new struct field appears; observation cannot. It also re-catches
//! a wrong-field mapping for free (a Field that writes the wrong leaf shows the wrong path here).
//!
//! ★ **Maximal fixture, not `default()`.** `ReturnInputs::default()` leaves `spouse`/`schedule_a` `None`,
//! `w2s`/`dependents`/`charitable`/`box12` empty, `ip_pin` `None` — so those leaf paths NEVER appear in the
//! serialized `Value`, and a KAT built on `default()` would give FALSE drift-protection for every W-2 /
//! spouse / Schedule-A / dependent field. This fixture forces every optional present and ≥1 element in every
//! in-scope Vec, so all 66 in-scope leaves are realized.
//!
//! ★ `serde_json::Value` walking is permitted HERE ONLY — the §4 veto is on get/set/production paths, not a
//! test. No accessor in this crate walks `Value`.

use super::{field_to_question, form_spec};
use crate::seam::{
    Field, FieldId, FieldKind, FieldValue, RowAddr, SecretView, SectionId, SetError,
};
use btctax_core::tax::return_inputs::{
    Box12Entry, CharitableClass, CharitableGift, Dependent, Person, ReturnInputs, ScheduleAInputs,
    W2,
};
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::date;

/// Flatten a JSON value into `leaf-path → leaf-value`. A leaf is any node that represents ONE logical struct
/// field: a scalar (string / number / bool / `null` — including an `Option::None`), OR an array whose every
/// element is itself scalar. That second case matters because `time::Date` serializes as `[year, ordinal]`
/// — a two-int array that is ONE struct leaf, not two. A real `Vec<Struct>` (its elements are objects)
/// instead recurses per element (`parent[i].field`), which is exactly the per-row granularity we want.
fn walk(v: &Value, prefix: &str, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                let p = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                walk(child, &p, out);
            }
        }
        // A real Vec<Struct> — at least one element is compound — recurses per element.
        Value::Array(arr) if arr.iter().any(|e| e.is_object() || e.is_array()) => {
            for (i, child) in arr.iter().enumerate() {
                walk(child, &format!("{prefix}[{i}]"), out);
            }
        }
        // A scalar, or an all-scalar array (notably a serialized `time::Date`) — one logical leaf.
        leaf => {
            out.insert(prefix.to_string(), leaf.clone());
        }
    }
}

/// The full leaf-path map of a `ReturnInputs`.
fn leaf_map(ri: &ReturnInputs) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    walk(
        &serde_json::to_value(ri).expect("ReturnInputs serializes"),
        "",
        &mut out,
    );
    out
}

/// A MAXIMALLY-POPULATED fixture. Every optional is `Some`, every IN-SCOPE Vec has ≥1 element, so every
/// in-scope leaf path is realized. Leaf VALUES are the empty/zero/`None` defaults (bar the two liveness
/// primers below), so every sentinel below is guaranteed to differ from what's here — the diff can never be
/// a false "not covered". Exempt Vecs (`int_1099`/…) stay empty and `schedule_c` stays `None`: they are
/// exempt by struct-prefix regardless.
///
/// ★ Two liveness primers (review I-4 makes the delegating `set` gate on `live`, so the KAT's `set` of every
/// field must land on a return where that field is live): `filing_status = Mfs` makes `DeclMfsSpouseItemizes`
/// live, and `mortgage_interest_1098 = 1` makes `SaMortgageAllUsed` live. Both primer values are non-default
/// but still differ from their own field's sentinel, so their diff stays exact.
fn maximal_fixture() -> ReturnInputs {
    let mut ri = ReturnInputs {
        // ★★ §G-15 — a MAXIMALLY-POPULATED fixture must state its year, or every year-scoped
        // question is not live and its leaves silently drop out of the coverage census. 2025 is the
        // year in which every registry question is live.
        tax_year: 2025,
        filing_status: FilingStatus::Mfs,
        ..Default::default()
    };
    // ★★ R10.4 / T4b — liveness primer for `DeclFilingStatusConfirmed`: the confirmation is live
    //    only on a year the OPENER made, and this fixture must reach every field's setter.
    ri.opened_from = Some(2024);
    ri.header.spouse = Some(Person::default());
    ri.header.ip_pin = Some("000000".to_string());
    ri.header.dependents = vec![Dependent::default()];
    ri.w2s = vec![W2 {
        // `Box12Entry` has no `Default`; a blank code + zero dollars is the empty new row (per sections.rs).
        box12: vec![Box12Entry {
            code: String::new(),
            amount: dec!(0),
        }],
        ..Default::default()
    }];
    // ★★★ R4 / §5.7 — ONE ROW OF EVERY DOCUMENT `Vec`. They were EXEMPT by struct-prefix until T5
    //     (`int_1099`, `div_1099`, `g_1099`, `b_1099`), which is why the fixture left them empty;
    //     the sections landed, the exemptions went, and an empty `Vec` would now realize no leaf at
    //     all and give FALSE drift-protection for every 1099 box.
    //
    // ★ Leaf VALUES stay at the zero/empty default so every sentinel differs — except the two
    //   LIVENESS PRIMERS noted below, which must be non-default and still differ from their own
    //   field's sentinel.
    ri.int_1099 = vec![btctax_core::tax::return_inputs::Form1099Int::default()];
    ri.div_1099 = vec![btctax_core::tax::return_inputs::Form1099Div::default()];
    ri.b_1099 = vec![btctax_core::tax::return_inputs::Form1099B::default()];
    ri.g_1099 = vec![btctax_core::tax::return_inputs::Form1099G {
        // ★★ LIVENESS PRIMER for `DeclItemizedPriorYear`: the §111(a) gate is live iff a 1099-G
        //    box 2 is > 0 (or the document-less refund question is Yes). `1` is non-default and
        //    differs from the Money sentinel `4242`, so the box-2 field's own diff stays exact.
        box2_state_refund: dec!(1),
        ..Default::default()
    }];
    ri.form_1098e = vec![btctax_core::tax::return_inputs::Form1098E::default()];
    // ★★★ R4 / T16 — one row of each HSA information return, same reason as the four 1099 `Vec`s.
    ri.sa_1099 = vec![btctax_core::tax::return_inputs::Form1099Sa::default()];
    ri.sa_5498 = vec![btctax_core::tax::return_inputs::Form5498Sa::default()];
    // ★★★ T16 — the LIVENESS PRIMER for Form 8889: its seven declarations and its seven money
    //     leaves are live iff the §223 trigger is affirmed, so an unprimed fixture would cover none
    //     of the fourteen and give false drift-protection for the whole form.
    ri.sch1.hsa_activity = Some(true);
    // ★★★ R5 — one filer's-records row. Its five leaves are covered only while the R3 door is open,
    //     which `fixture_for` primes per field (the door's own declaration cannot be covered on a
    //     fixture that already holds its answer).
    ri.schedule_b_filer_records = vec![btctax_core::tax::return_inputs::ScheduleBRecord::default()];
    ri.schedule_a = Some(ScheduleAInputs {
        // `CharitableGift` has no `Default`; Cash60/zero is the sections.rs `add` starting point.
        charitable: vec![CharitableGift {
            class: CharitableClass::Cash60,
            amount: dec!(0),
        }],
        ..Default::default()
    });
    // ★★★ R8 / T9 — one Form 1098 row, same reason as the four 1099 `Vec`s: an empty one would
    //     realize no leaf at all and give FALSE drift-protection for every 1098 box. It doubles as
    //     the LIVENESS PRIMER for `SaMortgageAllUsed`, `SaMortgageWithinDebtLimit`,
    //     `DeclAmtQualifiedDwelling` and `DeclClaimingMortgageInterestCredit`, whose liveness is now
    //     `schedule_a.is_some() && !form_1098.is_empty()`. Its three non-default leaves each differ
    //     from their own field's sentinel, so every diff stays exact.
    ri.form_1098 = vec![btctax_core::tax::testonly::form_1098_with_interest(dec!(1))];
    // ★★★ R8 / T9 — one Schedule A line 8b row, for the same reason.
    ri.schedule_a
        .as_mut()
        .expect("the fixture just built one")
        .mortgage_interest_not_on_1098 =
        vec![btctax_core::tax::return_inputs::NonForm1098Interest::default()];
    // Liveness primer for `DeclAmtCarryoverSame` (Form 6251 line 2k). The VALUE leaves stay exempt via
    // EXEMPT_PREFIXES; this only makes the declaration that guards them live.
    ri.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
        short: dec!(1),
        long: dec!(0),
    };
    // Liveness primer for `DeclAmtDepreciationSame` (Form 6251 line 2l) — a Schedule C with a nonzero
    // FLAT expense total, which is all btctax ever sees of Part II.
    ri.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
        expenses: dec!(1),
        ..Default::default()
    });
    // ★ spec 1099-DA T6 — one provider with BOTH slots answered, so the map has leaves to police
    //   (`skip_serializing_if` drops an unanswered slot, and an empty map has no leaf at all).
    ri.broker_reporting.0.insert(
        "coinbase".to_string(),
        btctax_core::forms::CohortAnswers {
            covered: Some(btctax_core::forms::BrokerReported::NotReported),
            noncovered: Some(btctax_core::forms::BrokerReported::NotReported),
        },
    );
    // ★★ R10.3 — one record in each of the two provenance stores, so their EXEMPT entries below are
    //    LIVE. An empty `BTreeMap`/`Vec` yields no leaf at all, and the stale-exemption assert (l)
    //    would then fire on a perfectly correct exemption — the same trap the census exists to spring
    //    on a real one.
    //
    //    ★ Deliberately `Question`/`Skippable` keys and NO `DependentGate`: seeding one would make
    //      the `DepSsn` setter's identity-supersede fire during the mutate-and-diff, moving a SECOND
    //      leaf and breaking the "exactly one leaf" derivation for reasons that have nothing to do
    //      with coverage.
    //
    //    ★★ **FR-97 — `apply` DOES write a dependent-gate record now**, so the old reason given here
    //       (*"nothing in the form spec writes a dependent-gate record"*) is retired. This census is
    //       unaffected because it drives each `Field`'s own `set`, never `apply`: the answer log is
    //       written one layer up, which is exactly why `answer_log` is EXEMPT by prefix below.
    // ★★★ **T10 / §5.4 — the TRAILER's two liveness primers.**
    //
    // ★ `foreign_country` non-empty is the §5.4 rule for the province and the postal code
    //   (`HouseholdHeader::foreign_address_is_live`), so an unprimed fixture would cover neither
    //   leaf and give false drift-protection for the whole foreign block. The value is non-default
    //   and differs from the Text sentinel (`SENTINEL`), so the country's own diff stays exact.
    ri.header.foreign_country = "Elbonia".to_string();
    // ★ The direct-deposit block is an OPTIONAL SINGLETON: an absent one realizes no leaf at all
    //   (spec §5.7 requires it PRESENT on the coverage fixture for exactly this reason). Both
    //   strings differ from their own sentinels, and `Savings` differs from the Enum sentinel
    //   `Checking` below.
    ri.header.direct_deposit = Some(btctax_core::tax::return_inputs::DirectDeposit {
        routing: "123456780".to_string(),
        kind: Some(btctax_core::tax::return_inputs::DepositAccountKind::Savings),
        account: "0000000000".to_string(),
    });
    // ★ A spouse PIN, so the Secret leaf is realized (the spouse itself is created above).
    ri.header.spouse_ip_pin = Some("000000".to_string());
    ri.answer_log.insert(
        btctax_core::tax::provenance::AnswerKey::Question(
            btctax_core::tax::questions::QuestionId::ForeignTrust,
        ),
        btctax_core::tax::provenance::AnswerRecord {
            answered_on: date!(2025 - 06 - 01),
            // ★ The REGISTRY's words, not an invented string: a class-(A) record hashing anything
            //   else is refused as unanswered by `screen_inputs` (R10.3), and a fixture that refuses
            //   for a reason unrelated to coverage is a trap for the next reader.
            prompt_hash: btctax_core::tax::provenance::prompt_hash(
                // ★ R10.4 — `current_prompt` renders a question's words FROM the return, so the
                //   fixture hands it the return it is building the record for.
                &btctax_core::tax::provenance::current_prompt(
                    &btctax_core::tax::provenance::AnswerKey::Question(
                        btctax_core::tax::questions::QuestionId::ForeignTrust,
                    ),
                    &ri,
                )
                .expect("a registry question has a prompt"),
            ),
            state: btctax_core::tax::provenance::AnswerState::Given,
        },
    );
    ri.answer_log_history.push((
        btctax_core::tax::provenance::AnswerKey::Skippable(
            btctax_core::tax::questions::SkippableId::BlindTaxpayer,
        ),
        btctax_core::tax::provenance::AnswerRecord {
            answered_on: date!(2025 - 05 - 01),
            prompt_hash: btctax_core::tax::provenance::prompt_hash("older words"),
            state: btctax_core::tax::provenance::AnswerState::Declined,
        },
    ));
    ri
}

/// A per-`FieldKind` sentinel guaranteed to differ from the fixture's default leaf value, so a `set` that
/// truly writes ALWAYS produces a diff. Enum sentinels are chosen per `FieldId` to be a REAL variant that
/// differs from the fixture's default variant; a new Enum `Field` panics here until a sentinel is added.
fn sentinel(f: &Field) -> FieldValue {
    match f.kind {
        FieldKind::Money => FieldValue::Money(dec!(4242)),
        FieldKind::Text => FieldValue::Text("SENTINEL".to_string()),
        FieldKind::Bool => FieldValue::Bool(true),
        FieldKind::TriState => FieldValue::TriState(Some(true)),
        FieldKind::Date => FieldValue::Date(Some(date!(1990 - 01 - 02))),
        FieldKind::Secret => FieldValue::SecretEntry("123456789".to_string()),
        FieldKind::Enum(_) => {
            let choice = match f.id {
                FieldId::FilingStatus => "Single",          // fixture is Mfs
                FieldId::ItemizeElection => "ForceItemize", // fixture is Auto
                FieldId::W2Owner => "Spouse",               // fixture default is Taxpayer
                FieldId::CharClass => "OrdinaryProp50",     // fixture is Cash60
                // ★ FR-29 — Form 8615 condition 4. The fixture leaves it UNANSWERED (`None`), which
                //   is not a variant at all, so every token differs; `CannotKnow` is chosen because it
                //   is the one whose meaning is *"answered, and not yes or no"* — the distinction the
                //   whole three-valued type exists for.
                FieldId::Form8615Condition4ParentAlive => "CannotKnow",
                // spec 1099-DA T6 — the fixture's slots are NotReported
                FieldId::BrokerCovered | FieldId::BrokerNoncovered => "BasisMatches",
                // ★ R5 — the fixture row's kind is the `Default`, `Interest`.
                FieldId::SbRecordKind => "Dividend",
                // ★ T16 — both HSA account-type checkboxes are `None` in the fixture (the
                //   `Default`, and the state that REFUSES), so every real choice differs. `MaMsa`
                //   is chosen over `Hsa` for the same reason `CannotKnow` is chosen above: it is
                //   the answer no default could ever be.
                FieldId::Sa1099Box5AccountType | FieldId::Sa5498Box6AccountType => "MaMsa",
                // ★ R7 / T8 — the HoH marital basis. The fixture leaves it UNANSWERED (`None`),
                //   which is not a variant at all, so every token differs; `NotMarried` is the one
                //   the instruction states first and the only one that does not itself refuse.
                FieldId::HohMaritalBasis => "NotMarried",
                // ★ T10 — the fixture's block is `Savings`, so `Checking` is the choice that
                //   differs. Both are real variants; there is no third.
                FieldId::DdKind => "Checking",
                other => panic!("no Enum sentinel for {other:?} — add a distinct real choice"),
            };
            FieldValue::Choice(choice.to_string())
        }
    }
}

/// The [`DependentGate`] a `DepGate*` field carries — the inverse of `attribute::dependent_gate_field`,
/// and `None` for every other field.
fn gate_of_field(id: FieldId) -> Option<btctax_core::tax::provenance::DependentGate> {
    use btctax_core::tax::provenance::DependentGate as G;
    Some(match id {
        FieldId::DepGateQcRelationship => G::QcRelationship,
        FieldId::DepGateYoungerThanYouOrSpouse => G::YoungerThanYouOrSpouse,
        FieldId::DepGateFullTimeStudent => G::FullTimeStudent,
        FieldId::DepGatePermanentlyAndTotallyDisabled => G::PermanentlyAndTotallyDisabled,
        FieldId::DepGateProvidedOverHalfOwnSupport => G::ProvidedOverHalfOwnSupport,
        FieldId::DepGateFilingJointReturn => G::FilingJointReturn,
        FieldId::DepGateJointReturnOnlyToClaimRefund => G::JointReturnOnlyToClaimRefund,
        FieldId::DepGateLivedWithYouOverHalfYear => G::LivedWithYouOverHalfYear,
        FieldId::DepGateLivedWithYouInUs => G::LivedWithYouInUs,
        FieldId::DepGateQualifyingChildOfAnotherPerson => G::QualifyingChildOfAnotherPerson,
        FieldId::DepGateCitizenNationalResidentOrCanadaMexico => {
            G::CitizenNationalResidentOrCanadaMexico
        }
        FieldId::DepGateMarried => G::Married,
        FieldId::DepGateTinIssuedByDueDate => G::TinIssuedByDueDate,
        FieldId::DepGateCitizenNationalOrResidentAlien => G::CitizenNationalOrResidentAlien,
        FieldId::DepGateSsnsValidForEmploymentIssuedByDueDate => {
            G::SsnsValidForEmploymentIssuedByDueDate
        }
        FieldId::DepGateQrRelationshipOrMemberOfHousehold => G::QrRelationshipOrMemberOfHousehold,
        FieldId::DepGateQualifyingChildOfAnyTaxpayer => G::QualifyingChildOfAnyTaxpayer,
        FieldId::DepGateGrossIncomeUnderLimit => G::GrossIncomeUnderLimit,
        FieldId::DepGateYouProvidedOverHalfSupport => G::YouProvidedOverHalfSupport,
        FieldId::DepGateDivorcedSeparatedMultipleSupportOrKidnappedRuleApplies => {
            G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies
        }
        _ => return None,
    })
}

/// ★★★ **Make `gate` LIVE on row 0, and leave it UNANSWERED so its sentinel differs.**
///
/// Three steps, and the middle one is the point: clear the row, set the ONE precondition the
/// flowchart puts in front of this gate, then let the fixture helper walk the rest — so the path is
/// derived from `walk_dependent`, not typed here. Finally the gate itself is un-answered through the
/// registry's own `clear`.
///
/// ★★ Scaffolding, exactly like `fixture_for`'s other arms: a wrong precondition leaves the gate
///      not live, `set` returns `NoSuchRow`, and the KAT fails loudly. It cannot yield a false PASS.
fn prime_dependent_gate(ri: &mut ReturnInputs, gate: btctax_core::tax::provenance::DependentGate) {
    use btctax_core::tax::dependent_gates::{entry, DEPENDENT_GATES};
    use btctax_core::tax::provenance::DependentGate as G;
    // ★★ The two RETURN-LEVEL answers the flowchart consults on its way through: Step 2 question 4
    //    / Step 4 question 5 ("could you be claimed?") and Step 5 question 1 (the filer's own TIN).
    //    Without them the walk stops at `WaitingOnQuestion` and never reaches Step 3 or Step 5, so
    //    no gate past Step 2 would be live — which is a scenario bug, not a gate bug.
    ri.header.can_be_claimed_as_dependent_taxpayer = Some(false);
    ri.header.filer_tin_issued_by_due_date = Some(true);
    let d = &mut ri.header.dependents[0];
    for q in DEPENDENT_GATES {
        (q.clear)(d);
    }
    match gate {
        // Step 4's own gates are reached only when Step 1 says "not a qualifying child".
        G::QrRelationshipOrMemberOfHousehold
        | G::QualifyingChildOfAnyTaxpayer
        | G::GrossIncomeUnderLimit
        | G::YouProvidedOverHalfSupport
        | G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies => {
            d.qc_relationship = Some(false);
        }
        // The instruction opens the refund-only limb only on a joint return.
        G::JointReturnOnlyToClaimRefund => d.filing_joint_return = Some(true),
        // Row (5)(b) "And in the U.S." prints under a checked row (5)(a).
        G::LivedWithYouInUs => d.lived_with_you_over_half_year = Some(true),
        _ => {}
    }
    btctax_core::tax::testonly::answer_all_dependent_gates(ri);
    (entry(gate).clear)(&mut ri.header.dependents[0]);
}

/// A per-`FieldId` fixture VARIANT for a field whose liveness gate the maximal fixture cannot satisfy at
/// the same time as covering the gate itself.
///
/// ★ The §G-9 dates of death are the only case, and it is structural, not incidental: `DodTaxpayer` is live
/// only while `died_during_year == Some(true)`, but `DeclTaxpayerDiedDuringYear`'s own coverage needs the
/// fixture to hold something OTHER than the `TriState(Some(true))` sentinel — so one fixture cannot serve
/// both. Priming here (and using the primed return as this field's own diff BASELINE) keeps each field's
/// diff exact. Scaffolding, exactly like [`addr_for`]: a wrong tweak makes `set` return `Err` or leaves the
/// leaf uncovered, so it can never yield a false PASS.
fn fixture_for(field: &Field, base: &ReturnInputs) -> ReturnInputs {
    let mut ri = base.clone();
    // ★★★ **T7 / R6 — THE DEPENDENT GATES, primed by the WALK rather than one by one.**
    //
    //     A gate is live for a row iff the flowchart REACHES it, so no single fixture can cover all
    //     twenty: Step 3's gates need Step 1 to have said *"qualifying child"* and Step 4's need it
    //     to have said *"no"*. Structural, exactly like the §G-9 dates of death — and handled the
    //     same way, per field.
    if let Some(gate) = gate_of_field(field.id) {
        prime_dependent_gate(&mut ri, gate);
        return ri;
    }
    match field.id {
        FieldId::DodTaxpayer => ri.header.taxpayer_died_during_year = Some(true),
        // ★ MFJ too: `DodSpouse` is MFJ-gated (the only status whose spouse §63(f) box is counted),
        //   so a spouse `Person` plus the death gate is no longer enough to make it live.
        FieldId::DodSpouse => {
            ri.filing_status = btctax_core::tax::types::FilingStatus::Mfj;
            ri.header.spouse_died_during_year = Some(true);
        }
        // Likewise the spouse death GATE itself.
        FieldId::SpouseDiedDuringYear => {
            ri.filing_status = btctax_core::tax::types::FilingStatus::Mfj;
        }
        // ★ Schedule C lines I/J need a `schedule_c` to write onto; line J additionally needs line I
        //   answered YES (the form's own "If 'Yes,'"), or it is not live and `set` is `NoSuchRow`.
        FieldId::ScheduleC1099Required => {
            ri.schedule_c = Some(Default::default());
        }
        FieldId::ScheduleC1099Filed => {
            ri.schedule_c = Some(btctax_core::tax::return_inputs::ScheduleCInputs {
                payments_requiring_1099: Some(true),
                ..Default::default()
            });
        }
        // ★ The FBAR sub-question is live only under a Schedule B 7a "Yes"; a set on a non-live
        //   question correctly refuses with `NoSuchRow`, so the fixture must make it live.
        FieldId::FbarFilingRequired => ri.foreign_accounts = Some(true),
        // ★★★ T16 — the §223 trigger declaration is the GATE for Form 8889's fourteen leaves, so the
        //     maximal fixture primes it `Some(true)`. That makes covering the gate ITSELF
        //     impossible on that fixture: the `TriState(Some(true))` sentinel equals what is
        //     already there, and the mutate-and-diff observes nothing. Structural, exactly like the
        //     §G-9 dates of death and the document-less income door above — one fixture cannot both
        //     satisfy a gate and cover it.
        FieldId::DeclHsaActivity => ri.sch1.hsa_activity = None,
        // ★★★ R8 / T9 — the three sale-of-a-main-home TESTS are live only while `sold_main_home` is
        //     YES, and `sold_main_home` itself must be covered on a fixture holding something OTHER
        //     than the `TriState(Some(true))` sentinel. Same structural shape as `DeclHsaActivity`
        //     above: one fixture cannot both satisfy the gate and cover it, so the gate is primed
        //     here for the three it opens and left blank for its own row.
        FieldId::HomeSaleTest1 | FieldId::HomeSaleTest2 | FieldId::HomeSaleCanExcludeAllGain => {
            ri.home_sale.sold_main_home = Some(true)
        }
        FieldId::HomeSaleSoldMainHome => ri.home_sale.sold_main_home = None,
        // ★★★ FR-29 — the SPEC §6.3 dead-end fact is live ONLY once condition 4 is answered CANNOT
        //     KNOW. That is the owner ruling's first constraint discharged in the liveness predicate,
        //     so it is structural exactly like the §G-9 dates of death above: one fixture cannot both
        //     cover condition 4 and satisfy this gate, because covering condition 4 requires the
        //     fixture to hold something OTHER than this field's primer.
        FieldId::Form8615ParentIdentityUnobtainable => {
            ri.header.form8615_condition4_parent_alive =
                Some(btctax_core::tax::return_inputs::ParentAliveAnswer::CannotKnow);
        }
        // ★★★ R3 / T5 — THE DOCUMENT-LESS INCOME DOOR. Each of the three paired questions is live
        //     EXACTLY when its census row says `No`, and the maximal fixture leaves every census
        //     row `None` (unanswered — the honest starting point, and what makes the census rows'
        //     own sentinels differ). So each door question's coverage needs its own row answered
        //     `No` first: structural, exactly like the §G-9 dates of death above, because one
        //     fixture cannot both cover a census row and satisfy the question that row opens.
        FieldId::DeclWagesWithoutW2 => {
            ri.documents.set(
                btctax_core::tax::document_census::DocumentRow::W2,
                Some(false),
            );
        }
        FieldId::DeclStateRefundWithout1099g => {
            ri.documents.set(
                btctax_core::tax::document_census::DocumentRow::G1099,
                Some(false),
            );
        }
        FieldId::DeclInterestOrDividendsWithout1099 => {
            ri.documents.set(
                btctax_core::tax::document_census::DocumentRow::Int1099,
                Some(false),
            );
        }
        // ★★★ Seam review M-1 — the SAME structural reason one form over: the door is live exactly
        //     when the Form 1099-SA census row says `No`, and the maximal fixture transcribes a
        //     1099-SA row (so `reconcile` answers that row `Yes`). One fixture cannot both hold the
        //     document and open the door that exists for its absence.
        FieldId::DeclHsaDistributionWithout1099sa => {
            ri.documents.set(
                btctax_core::tax::document_census::DocumentRow::Sa1099,
                Some(false),
            );
        }
        // ★★★ R7 / T8 — HEAD OF HOUSEHOLD and QUALIFYING SURVIVING SPOUSE. Each test is live iff the
        //     FILING STATUS asserts it, and the maximal fixture files MFS — so covering these needs
        //     the status set, exactly as `DodSpouse` needs MFJ above. A status can be only one thing
        //     at a time, which is why HoH's and QSS's fixtures are separate arms rather than one.
        FieldId::DeclHohQualifyingPerson
        | FieldId::DeclHohPaidOverHalfCostOfKeepingUpHome
        | FieldId::HohMaritalBasis
        | FieldId::QualifyingChildName => {
            ri.filing_status = btctax_core::tax::types::FilingStatus::HoH;
        }
        FieldId::DeclQssSpouseDiedInWindow
        | FieldId::DeclQssChildYouCanClaim
        | FieldId::DeclQssChildLivedAllYear
        | FieldId::DeclQssPaidOverHalfCost
        | FieldId::DeclQssCouldHaveFiledJointly => {
            ri.filing_status = btctax_core::tax::types::FilingStatus::Qss;
        }
        // ★ R5 — the filer's-records ROWS need the door both live AND answered YES: the section is
        //   invisible otherwise, and a row nobody could see would be testimony never given.
        FieldId::SbRecordPayerName
        | FieldId::SbRecordPayerSsn
        | FieldId::SbRecordPayerAddress
        | FieldId::SbRecordAmount
        | FieldId::SbRecordKind => {
            ri.documents.set(
                btctax_core::tax::document_census::DocumentRow::Int1099,
                Some(false),
            );
            ri.interest_or_dividends_without_1099 = Some(true);
        }
        _ => {}
    }
    ri
}

/// The `RowAddr` at which a section's `set` addresses row 0 (nested sections need a deeper path). A wrong
/// addr makes `set` return `Err`, or panics on an out-of-bounds index — it can NEVER yield a false PASS, so
/// this scaffolding map is not part of the coverage source of truth (the fixture already has row 0 present).
fn addr_for(id: SectionId) -> RowAddr {
    match id {
        SectionId::W2Box12 => RowAddr(vec![0, 0]),
        SectionId::Dependents
        | SectionId::W2s
        | SectionId::ScheduleACharitable
        | SectionId::BrokerReporting
        // ★ R4 / R5 / T5 — the six document sections are depth-1 repeating groups.
        | SectionId::Int1099s
        | SectionId::Div1099s
        | SectionId::B1099s
        | SectionId::G1099s
        | SectionId::Form1098Es
        // ★ R4 / R8 / T9 — the Form 1098 rows and Schedule A line 8b's recipient rows.
        | SectionId::Form1098s
        | SectionId::NonForm1098Interest
        // ★ R4 / T16 — the two HSA information returns.
        | SectionId::Sa1099s
        | SectionId::Sa5498s
        | SectionId::ScheduleBFilerRecords => RowAddr(vec![0]),
        _ => RowAddr::default(),
    }
}

/// ★ THE COVERAGE KAT. Every in-scope leaf of the maximal fixture is covered by exactly one `Field`, or is
/// listed EXEMPT; and nothing is both. A new in-scope struct field bites here.
#[test]
fn every_in_scope_leaf_is_covered_by_exactly_one_field_or_exempt() {
    let fixture = maximal_fixture();
    let before = leaf_map(&fixture);

    // ── 1. The COVERED set — derived by OBSERVATION (mutate-and-diff), never hand-declared. ──
    let mut covered: BTreeMap<String, FieldId> = BTreeMap::new();
    let mut seen_kinds: BTreeSet<&'static str> = BTreeSet::new();
    for section in form_spec() {
        let addr = addr_for(section.id);
        for field in section.fields {
            seen_kinds.insert(match field.kind {
                FieldKind::Money => "Money",
                FieldKind::Text => "Text",
                FieldKind::Bool => "Bool",
                FieldKind::TriState => "TriState",
                FieldKind::Date => "Date",
                FieldKind::Enum(_) => "Enum",
                FieldKind::Secret => "Secret",
            });

            let s = sentinel(field);
            let base = fixture_for(field, &fixture);
            // ★★ R3 — a Field that CANNOT be live on any return is skipped, and the skip is DERIVED
            //    from the census's own liveness predicate rather than from a name list. Two census
            //    rows are shadowed by a scalar today (`form_1098` → T9, `form_1098e` → T5), so
            //    their `set` correctly answers `NoSuchRow` and the mutate-and-diff has nothing to
            //    observe; their leaves are EXEMPT below, with the task that removes the exemption
            //    named. The moment T9/T5 flips `row_is_live`, the Field re-enters this loop and the
            //    exemption goes stale — which assertion (l) then reds on.
            let census_row = field_to_question(field.id)
                .and_then(btctax_core::tax::document_census::row_of_question);
            if census_row.is_some_and(|r| !btctax_core::tax::document_census::row_is_live(&base, r))
            {
                continue;
            }
            let before = leaf_map(&base); // shadows the outer baseline — see `fixture_for`
            let mut ri = base.clone();
            (field.set)(&mut ri, &addr, s.clone()).unwrap_or_else(|e| {
                panic!("set failed for {:?} in {:?}: {e:?}", field.id, section.id)
            });

            // ── ★ I-6 (spec §10): the get→set round-trip. A non-Secret field must read back EXACTLY what
            // was written — catching a `get` that reads a DIFFERENT leaf than `set` writes (untested for ~40
            // of 62 fields before this) and Enum token drift. A Secret keeps the §10 I-2 carve-out: `get`
            // returns PRESENCE (a masked `SecretView`), never the entry — so assert the asymmetry, not a
            // symmetric round-trip.
            let read_back = (field.get)(&ri, &addr);
            if let FieldKind::Secret = field.kind {
                assert!(
                    matches!(read_back, Some(FieldValue::Secret(SecretView::Set { .. }))),
                    "{:?} ({:?}): Secret get must return a Set presence view after a non-empty entry",
                    field.id,
                    section.id
                );
                assert_ne!(
                    read_back.as_ref(),
                    Some(&s),
                    "{:?}: a Secret get must NOT echo the SecretEntry back (§4/§5.5)",
                    field.id
                );
            } else {
                assert_eq!(
                    read_back,
                    Some(s.clone()),
                    "{:?} ({:?}): get after set must read back the written value (get↔set pairing, §10)",
                    field.id,
                    section.id
                );
            }

            let after = leaf_map(&ri);
            let all_keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
            let changed: Vec<String> = all_keys
                .into_iter()
                .filter(|k| before.get(*k) != after.get(*k))
                .cloned()
                .collect();
            assert_eq!(
                changed.len(),
                1,
                "{:?} ({:?}) must touch EXACTLY ONE leaf; it touched {:?}",
                field.id,
                section.id,
                changed
            );
            let path = changed.into_iter().next().unwrap();
            if let Some(prev) = covered.insert(path.clone(), field.id) {
                panic!(
                    "leaf `{path}` is double-covered — by {prev:?} and {:?}",
                    field.id
                );
            }
        }
    }

    // ── 2. The EXEMPT surface (spec §5.8) — a LITERAL in the test, so a new in-scope field still bites. ──
    // Struct-prefix for wholly out-of-scope top-level structs; explicit leaf paths for the deferred leaves
    // of the IN-SCOPE `sch1` (its `hsa_activity` IS in scope, so `sch1` cannot be exempted wholesale). The
    // "provenance leaves" (`qbi.*_provenance`, `charitable_carryover_in[].provenance`) fall under their
    // struct-prefix exemptions.
    // ★★★ **THE RATCHET (R4's kill): `EXEMPT_PREFIXES` MAY ONLY SHRINK.**
    //
    // T5 removed FOUR of the nine — `int_1099`, `div_1099`, `g_1099`, `b_1099` — because the four
    // 1099 sections landed and every box is now a `Field`. What is left is either provenance (ours,
    // never the filer's) or a deferred section with its owning task named. The count is asserted
    // `<=` below, so a later task may drop an entry but never add one back without moving the pin
    // deliberately.
    const EXEMPT_PREFIXES: &[&str] = &[
        "capital_loss_carryforward_in",
        "charitable_carryover_in",
        // ★★★ **Schedule 1-A (TY2025) — EXEMPTED DELIBERATELY, AND THIS IS A KNOWN GAP, NOT A
        //     JUDGEMENT THAT IT DOES NOT BELONG HERE.**
        //
        //     Unlike every other prefix above, this is NOT an information-return import surface. It
        //     is ~19 filer DECLARATIONS plus three money leaves, and declarations are exactly what
        //     an interactive form is for. The core landed first (struct, answered-ness classifier,
        //     negative-money screen, PII scrub) because those are compiler-enforced and cannot be
        //     half-done; the form section is ~22 Fields plus per-vehicle repeating-row addressing
        //     across four files.
        //
        //     ★ It is held back rather than rushed because the implementation plan is explicit that
        //     **prompt wording is the deliverable here, not plumbing** — "a wrong prompt is a wrong
        //     return that every test passes". Every prompt has to state the condition that permits a
        //     YES and default to the answer that cannot overstate the deduction. Writing 22 of those
        //     quickly is how the two Criticals in an earlier round happened.
        //
        //     ★ Nothing is reachable through the gap today: TY2025 full returns are fail-closed
        //     (`ty2025_full_return_must_stay_fail_closed_until_complete`), so Schedule 1-A cannot be
        //     emitted by any path, and the TOML import surface still carries these fields.
        //     REMOVE THIS PREFIX when the section lands — the coverage KAT will then police it.
        "schedule_1a",
        // ★★★ **R10.3 — THE ANSWER LOG IS PROVENANCE, NOT TESTIMONY, and that is the reason it is
        //     exempt rather than an oversight.** Every other exemption above is a deferred INPUT
        //     surface; these two are not inputs at all. The filer never types a record — btctax writes
        //     one because it observed an act — so there is no `Field` that could cover it and no
        //     prompt that could ask for it. Same class as the `*_provenance` leaves below: *ours, not
        //     the filer's*.
        //
        //     ★ It is a PREFIX (not a leaf) because the map's keys are data: `answer_log.<key>.state`
        //       is one leaf per recorded answer, and there is no fixed set of them.
        "answer_log",
        // ★ A SEPARATE entry, and not a redundant one: the prefix matcher requires `.` or `[` after
        //   the prefix, so `answer_log` does NOT cover `answer_log_history`. Spelling it out is what
        //   keeps the exemption honest rather than accidental.
        "answer_log_history",
    ];
    const EXEMPT_LEAVES: &[&str] = &[
        // ★★★ **R10.4 / T4b — WHICH YEAR THIS RETURN WAS OPENED FROM.** Provenance, in the same class
        //     as the answer log above and the `*_provenance` leaves below: *ours, not the filer's*.
        //     `open_next_year` stamps it; no form field could ask for it, because the filer does not
        //     know or choose it — and the one thing it decides (whether the carried filing status is
        //     confirmed) is asked as its own declaration, which DOES have a field.
        "opened_from",
        // ★★ §G-15 — `tax_year` is the SCOPE the form is filled in, not a value the filer types into
        // it. It is set by the command (`--year`) and stamped from the storage row key, so an input
        // field for it would invite the filer to contradict the year their return is filed under.
        // Exempt DELIBERATELY, which is what this census exists to force someone to decide.
        "tax_year",
        // ★ `schedule_c` is no longer a WHOLESALE exemption: lines I and J are in scope (class-(B)
        //   skippables), so the struct is exempted LEAF BY LEAF, exactly as `sch1` is for the same
        //   reason. A blanket prefix here would have silently re-exempted the two the moment they
        //   became covered — which is precisely the contradiction this census caught.
        // ★ §G-22 — `qbi` is no longer a WHOLESALE exemption: both loss carryforwards are now in
        //   scope (they UNDERSTATE tax when omitted, unlike every other carryforward family). Only
        //   the two provenance leaves stay exempt — they are ours, not the filer's.
        // ★ §G-20a — the two BENEFIT-carryover provenance leaves. Exempt for the same reason as the
        //   QBI ones: provenance is OURS, not the filer's — they never type it, btctax stamps it.
        //   ★ The census caught these the moment they were added, because the wholesale prefixes
        //   `capital_loss_carryforward_in` / `charitable_carryover_in` do NOT match the `_provenance`
        //   siblings. That is the census working, not a nuisance.
        // ★★ §G-20 — i1040gi's two MFS conditions. Exempt DELIBERATELY for now: they are class-(B)
        //   benefit claims whose silence forgoes, and the form surface for them is the next increment.
        //   `Advisory::Mfs63fSpouseBoxesForgone` already names the cost, so a filer is not left
        //   guessing — but this exemption is the reason it still fires for anyone who wants the boxes.
        "header.spouse_had_no_income",
        "header.spouse_not_filing_a_return",
        // ★★★ **R3 — THE SCALAR-SHADOWED CENSUS ROWS ARE GONE FROM THIS LIST, both of them, and
        //     that is the mechanism working rather than an edit anyone had to remember.**
        //     `documents.form_1098e` left at **T5** when `Form1098E` replaced the
        //     `sch1.student_loan_interest_paid` scalar; `documents.form_1098` left at **T9** when
        //     `Form1098` replaced `schedule_a.mortgage_interest_1098`. In each case `row_is_live`
        //     opened the row, the mutate-and-diff skip (derived from that same predicate) stopped
        //     skipping it, and assertion (3) below then demanded a Field for the leaf — while
        //     assertion (l) reds on the stale exemption if it is left behind, which is how this one
        //     was found.
        "capital_loss_carryforward_in_provenance",
        "charitable_carryover_in_provenance",
        "qbi.reit_ptp_carryforward_in_provenance",
        "qbi.qbi_carryforward_in_provenance",
        "schedule_c.owner",
        "schedule_c.business_description",
        "schedule_c.naics_code",
        "schedule_c.accounting_method",
        "schedule_c.expenses",
        // ★★ §G-28/B3 — Schedule C line 1's non-ledger half, exempt for the SAME reason as
        //    `expenses` directly above it: Schedule C is not a v1 input-form section, so its owner,
        //    description, NAICS code, method and money lines all arrive through `income import`. A
        //    lone gross-receipts field in the editor would be incoherent — there would be no business
        //    to attach it to. ★ If Schedule C ever becomes a form section, these two arrive together.
        //    (`schedule_c.qbi_w2_wages`/`qbi_ubia` are NOT exempt: their refusal points at them, so
        //    the filer must be able to reach them from the editor.)
        "schedule_c.other_gross_receipts",
        "sch1.state_refund_taxable",
        "sch1.ira_deduction_claimed",
    ];
    let is_exempt = |path: &str| {
        EXEMPT_LEAVES.contains(&path)
            || EXEMPT_PREFIXES.iter().any(|p| {
                path == *p
                    || path.starts_with(&format!("{p}."))
                    || path.starts_with(&format!("{p}["))
            })
    };

    // ── (l): keep the EXEMPT lists LIVE. A stale entry (a renamed/removed leaf or struct) silently
    // over-exempts — it could mask a genuine coverage gap the moment some in-scope leaf starts matching a
    // dead prefix. Assert every EXEMPT_LEAF is a real fixture leaf and every EXEMPT_PREFIX matches ≥1 leaf.
    for leaf in EXEMPT_LEAVES {
        assert!(
            before.contains_key(*leaf),
            "EXEMPT_LEAVES entry {leaf:?} matches no fixture leaf — stale exemption (would mask a gap)"
        );
    }
    for prefix in EXEMPT_PREFIXES {
        let matches = before.keys().any(|p| {
            p == prefix
                || p.starts_with(&format!("{prefix}."))
                || p.starts_with(&format!("{prefix}["))
        });
        assert!(
            matches,
            "EXEMPT_PREFIXES entry {prefix:?} matches no fixture leaf — stale exemption (would mask a gap)"
        );
    }

    // ── ★★★ (l2) THE RATCHET — `EXEMPT_PREFIXES` may only SHRINK (R4's kill). ──
    //
    // A count, not a list, and asserted `<=` rather than `==`: a later task that drops an entry
    // passes, and one that ADDS an exemption back — the only direction that can hide a leaf nobody
    // collects — reds. Moving the pin up must be a deliberate edit with a reason beside it.
    const EXEMPT_PREFIX_CEILING: usize = 5;
    assert!(
        EXEMPT_PREFIXES.len() <= EXEMPT_PREFIX_CEILING,
        "EXEMPT_PREFIXES is a RATCHET and may only shrink: {} entries, ceiling {EXEMPT_PREFIX_CEILING}. \
         T5 took it from 9 to 5 by landing the four 1099 sections; adding one back hides a leaf \
         nobody collects. {EXEMPT_PREFIXES:?}",
        EXEMPT_PREFIXES.len()
    );

    // ── 3. THE ASSERTION: {all in-scope leaves} == {covered} ∪ {exempt}, and nothing is both. ──
    let uncovered: Vec<&String> = before
        .keys()
        .filter(|p| !covered.contains_key(*p) && !is_exempt(p))
        .collect();
    assert!(
        uncovered.is_empty(),
        "these IN-SCOPE leaves are covered by NO Field and are NOT in EXEMPT — add a Field (or exempt it \
         deliberately in §5.8): {uncovered:#?}"
    );
    let covered_and_exempt: Vec<&String> = before
        .keys()
        .filter(|p| covered.contains_key(*p) && is_exempt(p))
        .collect();
    assert!(
        covered_and_exempt.is_empty(),
        "these leaves are BOTH covered by a Field AND listed EXEMPT — resolve the contradiction: \
         {covered_and_exempt:#?}"
    );

    // A covered path that is NOT a fixture leaf means a Field wrote somewhere the maximal fixture never
    // realized — a fixture/accessor mismatch. (Belt-and-suspenders; must be empty.)
    let phantom: Vec<&String> = covered
        .keys()
        .filter(|p| !before.contains_key(*p))
        .collect();
    assert!(
        phantom.is_empty(),
        "Fields touched non-fixture (phantom) leaves: {phantom:#?}"
    );

    // Every FieldKind must have been exercised (requirement 4) — including Bool, Date, and Secret.
    for k in [
        "Money", "Text", "Bool", "TriState", "Date", "Enum", "Secret",
    ] {
        assert!(
            seen_kinds.contains(k),
            "FieldKind {k} was never exercised by the KAT"
        );
    }

    // Count tripwires — pin the 65-leaf / 65-Field identity so a silent drop is loud even if some other
    // change happened to keep the sets balanced.
    let field_count: usize = form_spec().iter().map(|s| s.fields.len()).sum();
    assert_eq!(
        field_count, 279,
        "expected 216 Fields — 117 before T5, plus its FIFTY-EIGHT: the four document-less income \
         declarations (R3), W-2 boxes 13 and 14b, and the six document sections (1099-INT 14, \
         1099-DIV 14, 1099-B 8, 1099-G 7, 1098-E 4, and R5's five filer's-records leaves) — plus \
         the SEVEN the seam review's M-1 added, one per income box that had no reader and now \
         refuses (1099-G 5/6/7/9, 1099-B 13, 1099-DIV 9/10). A refuse-guard needs a Field on the \
         document's own row or the guard is a brick the filer cannot reach, which is the FR-65 \
         defect T5 itself was fixing. ★ R9 / T6 added the 183rd, Form 1040 page 1's DIGITAL ASSETS \
         question. ★★★ T16 / FR-76 added THIRTY-THREE for Form 8889: two census rows, the Form \
         1099-SA section's 8 and the Form 5498-SA section's 9, Form 8889's own 7 money leaves, and \
         its 7 declarations. \u{2605} The T16 SEAM REVIEW added TWO more: I-3's spouse-plan \
         declaration (Form 8889 line 1 / line 3 rule 1) and M-1's document-less distribution door \
         (line 14a). ★★★ T7 / R6 added TWENTY-ONE: the twenty per-row §152 gates of Who \
         Qualifies as Your Dependent, plus Step 5 question 1 — the one gate that is about the \
         FILER rather than about a row. \u{2605}\u{2605}\u{2605} R7 / T8 added TEN: Head of household's two tests, its MARITAL BASIS (a `Choice`, and the registry's first class-(A) skippable), the entry space for a non-dependent qualifying child, FR-67's \u{a7}6013(g)/(h) nonresident-alien-spouse election gate, and Qualifying surviving spouse's five conditions. \u{2605}\u{2605}\u{2605} R8 / T9 added TWENTY-TWO: the Form 1098 section's THIRTEEN (lender, TIN, transcription date, boxes 1, 2, 3, 4, 5, 6, 7, 8 and 10, and the per-row shared-interest gate), Schedule A line 8b's FOUR (the recipient's name, identifying number and address, and the amount), the sale-of-a-main-home section's FOUR, and Schedule A's Line 8a Caution (the Form 8396 mortgage interest credit). Line 8c replaced `SaMortgage1098` on the Schedule A section, so that one is a swap and not a twenty-third. \u{2605}\u{2605}\u{2605} T10 / \u{a7}5.4 added EIGHT \u{2014} the TRAILER: the spouse's Identity Protection PIN (this census's own motivating gap), the header's three foreign-address cells, the signature block's phone number, and the direct-deposit block's three (routing, account type, account number). Every one of the eight was a TY2024 cell censused `unmodeled` behind `Advisory::UnmodeledReturnOptionsOmitted`; collecting them is what let nine census entries retire."
    );
    assert_eq!(
        covered.len(),
        279,
        "★★★ EVERY Field is now distinctly covered — 279 of 279, and the last gap closed at T9. \
         It was 115 of 117 before T5, then 174 of 175, then 182 of 183, and the one always missing \
         was `DocForm1098`, whose census row was shadowed by the \
         `schedule_a.mortgage_interest_1098` scalar and so was never live. T9 replaced the scalar \
         with the `form_1098` document rows, `row_is_live` opened the row on the itemize election, \
         and the leaf became coverable. T16's thirty-three, the seam review's two, R7 / T8's ten \
         and R8 / T9's twenty-two are all covered, and so are T10's eight."
    );

    // ── 5. ★ I-6: PIN the observed FieldId → leaf-path map against a literal (kills TRANSPOSITION). ──
    // The cardinality asserts above cannot see a coherent Field↔leaf SWAP between two same-typed leaves
    // (e.g. `Box3SsWages` ↔ `Box5MedWages`): both accessors move together, so the bijection stays perfect
    // and the get↔set round-trip also passes (both read/write the same wrong leaf). This literal is the
    // ground truth — a transposition or a re-pointed accessor names itself in the assert diff.
    // Compared in the `leaf-path → FieldId` direction (`String` is `Ord`, so no seam `Ord` on `FieldId`);
    // `covered` already IS the observed map. The pinned literal is inverted into the same shape.
    let expected: BTreeMap<String, FieldId> = EXPECTED_LEAF_PATHS
        .iter()
        .map(|(id, p)| ((*p).to_string(), *id))
        .collect();
    assert_eq!(
        covered, expected,
        "the observed leaf-path → FieldId map drifted from the pinned expectation — a Field writes a \
         DIFFERENT leaf than declared (a transposition, or an accessor re-pointed at the wrong leaf)"
    );

    // ── 6. ★ I-6 (spec §10): a wrong-`FieldValue`-kind `set` on a representative field per kind → WrongKind. ──
    let wrong_kind: &[(FieldId, FieldValue)] = &[
        (FieldId::Box1Wages, FieldValue::Text("x".to_string())), // Money
        (FieldId::TpFirstName, FieldValue::Money(dec!(1))),      // Text
        (
            FieldId::TpPresidentialFund,
            FieldValue::Text("x".to_string()),
        ), // Bool
        (FieldId::DeclForeignAccounts, FieldValue::Money(dec!(1))), // TriState (delegating, always live)
        (FieldId::DepDob, FieldValue::Money(dec!(1))),              // Date
        (FieldId::FilingStatus, FieldValue::Money(dec!(1))),        // Enum
        (FieldId::TpSsn, FieldValue::Text("x".to_string())),        // Secret
    ];
    for (id, bad) in wrong_kind {
        let (field, addr) = locate(*id);
        let mut ri = fixture.clone();
        assert_eq!(
            (field.set)(&mut ri, &addr, bad.clone()),
            Err(SetError::WrongKind),
            "a wrong-kind set on {id:?} must be WrongKind (§10)"
        );
    }
}

/// Find a `Field` and the `RowAddr` its section addresses row 0 at (test helper for the §10 kind-mismatch).
fn locate(id: FieldId) -> (&'static Field, RowAddr) {
    for s in form_spec() {
        if let Some(f) = s.fields.iter().find(|f| f.id == id) {
            return (f, addr_for(s.id));
        }
    }
    panic!("field {id:?} not found in form_spec()");
}

/// ★ THE PINNED FieldId → serde-leaf-path GROUND TRUTH (review I-6). Every one of the 62 in-scope `Field`s
/// maps to the exact `ReturnInputs` leaf its `set` must write, at the maximal fixture's row 0. The KAT
/// asserts the OBSERVED (mutate-and-diff) map equals this literal, so a Field wired to the wrong same-typed
/// leaf — invisible to a pure cardinality check — fails loudly and names itself.
const EXPECTED_LEAF_PATHS: &[(FieldId, &str)] = &[
    (FieldId::FilingStatus, "filing_status"),
    (FieldId::ItemizeElection, "itemize_election"),
    (FieldId::TpFirstName, "header.taxpayer.first_name"),
    (FieldId::TpLastName, "header.taxpayer.last_name"),
    (FieldId::TpSsn, "header.taxpayer.ssn"),
    (FieldId::TpOccupation, "header.taxpayer.occupation"),
    (
        FieldId::TpPresidentialFund,
        "header.presidential_fund_taxpayer",
    ),
    (FieldId::IpPin, "header.ip_pin"),
    (FieldId::SpFirstName, "header.spouse.first_name"),
    (FieldId::SpLastName, "header.spouse.last_name"),
    (FieldId::SpSsn, "header.spouse.ssn"),
    (FieldId::SpOccupation, "header.spouse.occupation"),
    (
        FieldId::SpPresidentialFund,
        "header.presidential_fund_spouse",
    ),
    // ★★★ T10 / §5.4 — the trailer.
    (FieldId::SpIpPin, "header.spouse_ip_pin"),
    (FieldId::AddrStreet, "header.address_street"),
    (FieldId::AddrCity, "header.address_city"),
    (FieldId::AddrState, "header.address_state"),
    (FieldId::AddrZip, "header.address_zip"),
    (FieldId::AddrForeignCountry, "header.foreign_country"),
    (FieldId::AddrForeignProvince, "header.foreign_province"),
    (FieldId::AddrForeignPostalCode, "header.foreign_postal_code"),
    (FieldId::AddrPhone, "header.phone"),
    (FieldId::DdRouting, "header.direct_deposit.routing"),
    (FieldId::DdKind, "header.direct_deposit.kind"),
    (FieldId::DdAccount, "header.direct_deposit.account"),
    (FieldId::DepName, "header.dependents[0].name"),
    (FieldId::DepSsn, "header.dependents[0].ssn"),
    (
        FieldId::DepRelationship,
        "header.dependents[0].relationship",
    ),
    (FieldId::DepDob, "header.dependents[0].date_of_birth"),
    // ── ★★★ T7 / R6 — the twenty per-row §152 gates. ──
    (
        FieldId::DepGateQcRelationship,
        "header.dependents[0].qc_relationship",
    ),
    (
        FieldId::DepGateYoungerThanYouOrSpouse,
        "header.dependents[0].younger_than_you_or_spouse",
    ),
    (
        FieldId::DepGateFullTimeStudent,
        "header.dependents[0].full_time_student",
    ),
    (
        FieldId::DepGatePermanentlyAndTotallyDisabled,
        "header.dependents[0].permanently_and_totally_disabled",
    ),
    (
        FieldId::DepGateProvidedOverHalfOwnSupport,
        "header.dependents[0].provided_over_half_own_support",
    ),
    (
        FieldId::DepGateFilingJointReturn,
        "header.dependents[0].filing_joint_return",
    ),
    (
        FieldId::DepGateJointReturnOnlyToClaimRefund,
        "header.dependents[0].joint_return_only_to_claim_refund",
    ),
    (
        FieldId::DepGateLivedWithYouOverHalfYear,
        "header.dependents[0].lived_with_you_over_half_year",
    ),
    (
        FieldId::DepGateLivedWithYouInUs,
        "header.dependents[0].lived_with_you_in_us",
    ),
    (
        FieldId::DepGateQualifyingChildOfAnotherPerson,
        "header.dependents[0].qualifying_child_of_another_person",
    ),
    (
        FieldId::DepGateCitizenNationalResidentOrCanadaMexico,
        "header.dependents[0].citizen_national_resident_or_canada_mexico",
    ),
    (FieldId::DepGateMarried, "header.dependents[0].married"),
    (
        FieldId::DepGateTinIssuedByDueDate,
        "header.dependents[0].tin_issued_by_due_date",
    ),
    (
        FieldId::DepGateCitizenNationalOrResidentAlien,
        "header.dependents[0].citizen_national_or_resident_alien",
    ),
    (
        FieldId::DepGateSsnsValidForEmploymentIssuedByDueDate,
        "header.dependents[0].ssns_valid_for_employment_issued_by_due_date",
    ),
    (
        FieldId::DepGateQrRelationshipOrMemberOfHousehold,
        "header.dependents[0].qr_relationship_or_member_of_household",
    ),
    (
        FieldId::DepGateQualifyingChildOfAnyTaxpayer,
        "header.dependents[0].qualifying_child_of_any_taxpayer",
    ),
    (
        FieldId::DepGateGrossIncomeUnderLimit,
        "header.dependents[0].gross_income_under_limit",
    ),
    (
        FieldId::DepGateYouProvidedOverHalfSupport,
        "header.dependents[0].you_provided_over_half_support",
    ),
    (
        FieldId::DepGateDivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
        "header.dependents[0].divorced_separated_multiple_support_or_kidnapped_rule_applies",
    ),
    (FieldId::W2Owner, "w2s[0].owner"),
    (FieldId::W2Employer, "w2s[0].employer"),
    (FieldId::W2Ein, "w2s[0].ein"),
    (FieldId::Box1Wages, "w2s[0].box1_wages"),
    (FieldId::Box2FedWh, "w2s[0].box2_fed_withheld"),
    (FieldId::Box3SsWages, "w2s[0].box3_ss_wages"),
    (FieldId::Box4SsWh, "w2s[0].box4_ss_withheld"),
    (FieldId::Box5MedWages, "w2s[0].box5_medicare_wages"),
    (FieldId::Box6MedWh, "w2s[0].box6_medicare_withheld"),
    (FieldId::Box7SsTips, "w2s[0].box7_ss_tips"),
    (FieldId::Box17StateWh, "w2s[0].box17_state_tax_withheld"),
    (FieldId::Box19LocalTax, "w2s[0].box19_local_tax"),
    (FieldId::Box8AllocTips, "w2s[0].box8_allocated_tips"),
    (FieldId::Box10DepCare, "w2s[0].box10_dependent_care"),
    (FieldId::Box12Code, "w2s[0].box12[0].code"),
    (FieldId::Box12Amount, "w2s[0].box12[0].amount"),
    (FieldId::SaMedical, "schedule_a.medical"),
    (FieldId::SaSaltRealEstate, "schedule_a.salt_real_estate"),
    (
        FieldId::SaSaltPersonalProp,
        "schedule_a.salt_personal_property",
    ),
    (
        FieldId::SaSaltStateEst,
        "schedule_a.salt_state_estimated_payments",
    ),
    (
        FieldId::SaSaltPriorYear,
        "schedule_a.salt_prior_year_balance_paid",
    ),
    (
        FieldId::SaSaltSalesTaxAmt,
        "schedule_a.salt_sales_tax_amount",
    ),
    (FieldId::SaPointsNotOn1098, "schedule_a.points_not_on_1098"),
    // ★★★ R8 / T9 — Schedule A line 8b's rows, and the Form 1098 document rows.
    (
        FieldId::Sa8bRecipientName,
        "schedule_a.mortgage_interest_not_on_1098[0].recipient_name",
    ),
    (
        FieldId::Sa8bRecipientTin,
        "schedule_a.mortgage_interest_not_on_1098[0].recipient_tin",
    ),
    (
        FieldId::Sa8bRecipientAddress,
        "schedule_a.mortgage_interest_not_on_1098[0].recipient_address",
    ),
    (
        FieldId::Sa8bAmount,
        "schedule_a.mortgage_interest_not_on_1098[0].amount",
    ),
    (FieldId::Form1098Lender, "form_1098[0].lender"),
    (FieldId::Form1098LenderTin, "form_1098[0].lender_tin"),
    (
        FieldId::Form1098TranscribedOn,
        "form_1098[0].transcribed_on",
    ),
    (FieldId::Form1098Box1Interest, "form_1098[0].box1_interest"),
    (
        FieldId::Form1098Box2Principal,
        "form_1098[0].box2_outstanding_principal",
    ),
    (
        FieldId::Form1098Box3OriginationDate,
        "form_1098[0].box3_origination_date",
    ),
    (
        FieldId::Form1098Box4Refund,
        "form_1098[0].box4_refund_overpaid_interest",
    ),
    (
        FieldId::Form1098Box5MortgageInsurance,
        "form_1098[0].box5_mortgage_insurance",
    ),
    (FieldId::Form1098Box6Points, "form_1098[0].box6_points"),
    (
        FieldId::Form1098Box7AddressSame,
        "form_1098[0].box7_property_address_same_as_payer",
    ),
    (
        FieldId::Form1098Box8PropertyAddress,
        "form_1098[0].box8_property_address",
    ),
    (FieldId::Form1098Box10Other, "form_1098[0].box10_other"),
    (
        FieldId::Form1098OtherBorrowerPaid,
        "form_1098[0].other_borrower_paid_interest",
    ),
    // ★★★ R8 / T9 — the sale of a main home.
    (FieldId::HomeSaleSoldMainHome, "home_sale.sold_main_home"),
    (
        FieldId::HomeSaleTest1,
        "home_sale.test1_owned_2_years_and_lived_2_years_of_last_5",
    ),
    (
        FieldId::HomeSaleTest2,
        "home_sale.test2_no_exclusion_on_another_home_in_2_years",
    ),
    (
        FieldId::HomeSaleCanExcludeAllGain,
        "home_sale.can_exclude_all_gain",
    ),
    (
        FieldId::SaInvestmentInterest,
        "schedule_a.investment_interest",
    ),
    (FieldId::Nii8960Line9b, "form_8960_line9b"),
    (FieldId::SaSaltUseSalesTax, "schedule_a.salt_use_sales_tax"),
    (
        FieldId::SaMortgageAllUsed,
        "schedule_a.mortgage_all_used_to_buy_build_improve",
    ),
    (
        FieldId::SaMortgageWithinDebtLimit,
        "schedule_a.mortgage_within_debt_limit",
    ),
    (FieldId::CharClass, "schedule_a.charitable[0].class"),
    (FieldId::CharAmount, "schedule_a.charitable[0].amount"),
    (FieldId::PayEstimated, "payments.estimated_tax_payments"),
    (FieldId::PayExtension, "payments.extension_payment"),
    (FieldId::PayOtherWh, "payments.other_withholding"),
    (
        FieldId::DeclDependentTaxpayer,
        "header.can_be_claimed_as_dependent_taxpayer",
    ),
    (
        FieldId::DeclDependentSpouse,
        "header.can_be_claimed_as_dependent_spouse",
    ),
    (FieldId::DeclMfsSpouseItemizes, "mfs_spouse_itemizes"),
    (FieldId::DeclForeignAccounts, "foreign_accounts"),
    (FieldId::DeclForeignTrust, "foreign_trust"),
    (FieldId::DeclHsaActivity, "sch1.hsa_activity"),
    (FieldId::DeclDualStatusAlien, "dual_status_alien"),
    // §911/931/933 exclusion gate + the four MAGI add-backs it gates (Schedule 1-A Part I lines
    // 2a-2d / the SALT worksheet's lines 3a-3d — one quantity, five phase-outs).
    (FieldId::DeclHasIncomeExclusion, "has_income_exclusion"),
    (
        FieldId::DeclOtherOutOfScopeIncome,
        "other_out_of_scope_income",
    ),
    (FieldId::DeclFilingForm4952, "filing_form_4952"),
    // The Capital Loss Carryover Worksheet's two unnumbered header conditions.
    (
        FieldId::DeclCarryoverIncludesSpousesJointLoss,
        "carryover_includes_spouses_joint_loss",
    ),
    (FieldId::DeclExcludedCanceledDebt, "excluded_canceled_debt"),
    (FieldId::ScheduleCIsSstb, "schedule_c.is_sstb"),
    (
        FieldId::ScheduleCIsCooperativePatron,
        "schedule_c.is_cooperative_patron",
    ),
    (FieldId::QbiW2Wages, "schedule_c.qbi_w2_wages"),
    (FieldId::QbiUbia, "schedule_c.qbi_ubia"),
    (FieldId::FbarFilingRequired, "fbar_filing_required"),
    (
        FieldId::TaxpayerDiedDuringYear,
        "header.taxpayer_died_during_year",
    ),
    (
        FieldId::SpouseDiedDuringYear,
        "header.spouse_died_during_year",
    ),
    (
        FieldId::ScheduleC1099Required,
        "schedule_c.payments_requiring_1099",
    ),
    (
        FieldId::ScheduleC1099Filed,
        "schedule_c.will_file_required_1099",
    ),
    (
        FieldId::QbiReitPtpCarryforwardIn,
        "qbi.reit_ptp_carryforward_in",
    ),
    (FieldId::QbiCarryforwardIn, "qbi.qbi_carryforward_in"),
    (
        FieldId::DonationsHadRestrictions,
        "donations_had_restrictions",
    ),
    (FieldId::CharitableCwaObtained, "charitable_cwa_obtained"),
    // ★★★ FR-29 — Form 8615's trio.
    (
        FieldId::Form8615Condition3AgeSupport,
        "header.form8615_condition3_age_support",
    ),
    (
        FieldId::Form8615Condition4ParentAlive,
        "header.form8615_condition4_parent_alive",
    ),
    (
        FieldId::Form8615ParentIdentityUnobtainable,
        "header.form8615_parent_identity_unobtainable",
    ),
    (FieldId::ExclPuertoRico, "excluded_puerto_rico_income"),
    (FieldId::Excl2555L45, "form_2555_line45"),
    (FieldId::Excl2555L50, "form_2555_line50"),
    (FieldId::Excl4563L15, "form_4563_line15"),
    (
        FieldId::DeclAmtQualifiedDwelling,
        "schedule_a.mortgage_dwelling_is_amt_qualified",
    ),
    (
        FieldId::DeclAmtCarryoverSame,
        "amt_carryover_same_as_regular",
    ),
    (
        FieldId::DeclAmtDepreciationSame,
        "amt_depreciation_same_as_regular",
    ),
    // spec 1099-DA T6 — the two slots of the fixture's one provider row
    (FieldId::BrokerCovered, "broker_reporting.coinbase.covered"),
    (
        FieldId::BrokerNoncovered,
        "broker_reporting.coinbase.noncovered",
    ),
    (FieldId::ForeignCountryNames, "foreign_country_names"),
    // ★ R3 / §5.1 — the sixteen LIVE document-census rows (the two scalar-shadowed ones are
    //   EXEMPT above, so the mutate-and-diff never observes them).
    (FieldId::DocW2, "documents.w2"),
    (FieldId::DocInt1099, "documents.int_1099"),
    (FieldId::DocDiv1099, "documents.div_1099"),
    (FieldId::DocB1099, "documents.b_1099"),
    (FieldId::DocG1099, "documents.g_1099"),
    (FieldId::DocR1099, "documents.r_1099"),
    (FieldId::DocSsa1099, "documents.ssa_1099"),
    (FieldId::DocNecMiscK1099, "documents.nec_misc_k_1099"),
    (FieldId::DocK1, "documents.k1"),
    (FieldId::DocScheduleERental, "documents.schedule_e_rental"),
    (FieldId::DocS1099, "documents.s_1099"),
    (FieldId::DocOid1099, "documents.oid_1099"),
    (FieldId::DocW2g, "documents.w2g"),
    (FieldId::DocC1099, "documents.c_1099"),
    (FieldId::DocA1095, "documents.a_1095"),
    (FieldId::DocT1098, "documents.t_1098"),
    (
        FieldId::DeclFilingStatusConfirmed,
        "filing_status_confirmed",
    ),
    (FieldId::BlindTaxpayer, "header.taxpayer.blind"),
    (FieldId::BlindSpouse, "header.spouse.blind"),
    (FieldId::DobTaxpayer, "header.taxpayer.date_of_birth"),
    (FieldId::DobSpouse, "header.spouse.date_of_birth"),
    (FieldId::DodTaxpayer, "header.taxpayer.date_of_death"),
    (FieldId::DodSpouse, "header.spouse.date_of_death"),
    // ── ★★★ R3 / T5 — the DOCUMENT-LESS INCOME DOOR's four declarations. ──
    (FieldId::DeclWagesWithoutW2, "w2_wages_without_w2"),
    (
        FieldId::DeclInterestOrDividendsWithout1099,
        "interest_or_dividends_without_1099",
    ),
    (
        FieldId::DeclStateRefundWithout1099g,
        "state_refund_without_1099g",
    ),
    (FieldId::DeclItemizedPriorYear, "itemized_prior_year"),
    // ── ★★★ R9 / T6 — Form 1040 page 1's DIGITAL ASSETS question. ──
    (FieldId::DeclDigitalAssetActivity, "digital_asset_activity"),
    // ── ★★★ R4 / T5 — W-2 boxes 13 and 14b, which the struct had no field for at all. ──
    (
        FieldId::W2Box13StatutoryEmployee,
        "w2s[0].box13_statutory_employee",
    ),
    (
        FieldId::W2Box14bTtoc,
        "w2s[0].box14b_treasury_tipped_occupation_codes",
    ),
    // ── ★★★ R4 / T5 — Form 1099-INT. ──
    (FieldId::Int1099Payer, "int_1099[0].payer"),
    (FieldId::Int1099PayerTin, "int_1099[0].payer_tin"),
    (FieldId::Int1099TranscribedOn, "int_1099[0].transcribed_on"),
    (FieldId::Int1099Box1Interest, "int_1099[0].box1_interest"),
    (
        FieldId::Int1099Box2EarlyWithdrawal,
        "int_1099[0].box2_early_withdrawal_penalty",
    ),
    (
        FieldId::Int1099Box3Treasury,
        "int_1099[0].box3_treasury_interest",
    ),
    (
        FieldId::Int1099Box4FedWithheld,
        "int_1099[0].box4_fed_withheld",
    ),
    (
        FieldId::Int1099Box6ForeignTax,
        "int_1099[0].box6_foreign_tax",
    ),
    (
        FieldId::Int1099Box8TaxExempt,
        "int_1099[0].box8_tax_exempt_interest",
    ),
    (
        FieldId::Int1099Box9PrivateActivity,
        "int_1099[0].box9_private_activity_bond_amt",
    ),
    (
        FieldId::Int1099Box10MarketDiscount,
        "int_1099[0].box10_market_discount",
    ),
    (
        FieldId::Int1099Box11BondPremium,
        "int_1099[0].box11_bond_premium",
    ),
    (
        FieldId::Int1099Box12BondPremiumTreasury,
        "int_1099[0].box12_bond_premium_treasury",
    ),
    (
        FieldId::Int1099Box13BondPremiumTaxExempt,
        "int_1099[0].box13_bond_premium_tax_exempt",
    ),
    // ── ★★★ R4 / T5 — Form 1099-DIV. ──
    (FieldId::Div1099Payer, "div_1099[0].payer"),
    (FieldId::Div1099PayerTin, "div_1099[0].payer_tin"),
    (FieldId::Div1099TranscribedOn, "div_1099[0].transcribed_on"),
    (FieldId::Div1099Box1aOrdinary, "div_1099[0].box1a_ordinary"),
    (
        FieldId::Div1099Box1bQualified,
        "div_1099[0].box1b_qualified",
    ),
    (
        FieldId::Div1099Box2aCapGain,
        "div_1099[0].box2a_capgain_distr",
    ),
    (
        FieldId::Div1099Box2bUnrecap1250,
        "div_1099[0].box2b_unrecap_1250",
    ),
    (
        FieldId::Div1099Box2cSection1202,
        "div_1099[0].box2c_section_1202",
    ),
    (
        FieldId::Div1099Box2dCollectibles,
        "div_1099[0].box2d_collectibles_28",
    ),
    (
        FieldId::Div1099Box4FedWithheld,
        "div_1099[0].box4_fed_withheld",
    ),
    (
        FieldId::Div1099Box5Section199a,
        "div_1099[0].box5_section_199a",
    ),
    (
        FieldId::Div1099Box7ForeignTax,
        "div_1099[0].box7_foreign_tax",
    ),
    (
        FieldId::Div1099Box9CashLiquidation,
        "div_1099[0].box9_cash_liquidation",
    ),
    (
        FieldId::Div1099Box10NoncashLiquidation,
        "div_1099[0].box10_noncash_liquidation",
    ),
    (
        FieldId::Div1099Box12ExemptInterest,
        "div_1099[0].box12_exempt_interest_dividends",
    ),
    (
        FieldId::Div1099Box13PrivateActivity,
        "div_1099[0].box13_private_activity_amt",
    ),
    // ── ★★★ R4 / T5 — Form 1099-B. ──
    (FieldId::B1099Payer, "b_1099[0].payer"),
    (FieldId::B1099PayerTin, "b_1099[0].payer_tin"),
    (FieldId::B1099TranscribedOn, "b_1099[0].transcribed_on"),
    (
        FieldId::B1099ShortTermProceeds,
        "b_1099[0].short_term_proceeds",
    ),
    (FieldId::B1099ShortTermBasis, "b_1099[0].short_term_basis"),
    (
        FieldId::B1099LongTermProceeds,
        "b_1099[0].long_term_proceeds",
    ),
    (FieldId::B1099LongTermBasis, "b_1099[0].long_term_basis"),
    (FieldId::B1099Box13Bartering, "b_1099[0].box13_bartering"),
    (
        FieldId::B1099BasisReportedNoAdjustments,
        "b_1099[0].basis_reported_and_no_adjustments",
    ),
    // ── ★★★ R4 / T5 — Form 1099-G. ──
    (FieldId::G1099Payer, "g_1099[0].payer"),
    (FieldId::G1099PayerTin, "g_1099[0].payer_tin"),
    (FieldId::G1099TranscribedOn, "g_1099[0].transcribed_on"),
    (
        FieldId::G1099Box1Unemployment,
        "g_1099[0].box1_unemployment",
    ),
    (FieldId::G1099Box2StateRefund, "g_1099[0].box2_state_refund"),
    (FieldId::G1099Box4FedWithheld, "g_1099[0].box4_fed_withheld"),
    (FieldId::G1099Box5Rtaa, "g_1099[0].box5_rtaa_payments"),
    (
        FieldId::G1099Box6TaxableGrants,
        "g_1099[0].box6_taxable_grants",
    ),
    (
        FieldId::G1099Box7Agriculture,
        "g_1099[0].box7_agriculture_payments",
    ),
    (FieldId::G1099Box9MarketGain, "g_1099[0].box9_market_gain"),
    (
        FieldId::G1099Box10FamilyLeave,
        "g_1099[0].box10_family_leave_benefits",
    ),
    // ── ★★★ R4 / T5 — Form 1098-E, and the census row its section opened. ──
    (FieldId::Form1098eLender, "form_1098e[0].lender"),
    (FieldId::Form1098eLenderTin, "form_1098e[0].lender_tin"),
    (
        FieldId::Form1098eTranscribedOn,
        "form_1098e[0].transcribed_on",
    ),
    (
        FieldId::Form1098eBox1Interest,
        "form_1098e[0].box1_interest",
    ),
    // ★ T9 — the row opened when `Form1098` replaced the Schedule A scalar.
    (FieldId::DocForm1098, "documents.form_1098"),
    (
        FieldId::DeclClaimingMortgageInterestCredit,
        "claiming_mortgage_interest_credit",
    ),
    (FieldId::DocForm1098e, "documents.form_1098e"),
    // ── ★★★ R4 / T16 — the two HSA information returns, their census rows, and Form 8889's own
    //    money leaves and declarations. ──
    (FieldId::Sa1099Payer, "sa_1099[0].payer"),
    (FieldId::Sa1099PayerTin, "sa_1099[0].payer_tin"),
    (FieldId::Sa1099TranscribedOn, "sa_1099[0].transcribed_on"),
    (
        FieldId::Sa1099Box1GrossDistribution,
        "sa_1099[0].box1_gross_distribution",
    ),
    (
        FieldId::Sa1099Box2EarningsOnExcess,
        "sa_1099[0].box2_earnings_on_excess",
    ),
    (
        FieldId::Sa1099Box3DistributionCode,
        "sa_1099[0].box3_distribution_code",
    ),
    (
        FieldId::Sa1099Box4Fmv,
        "sa_1099[0].box4_fmv_on_date_of_death",
    ),
    (
        FieldId::Sa1099Box5AccountType,
        "sa_1099[0].box5_account_type",
    ),
    (FieldId::DocSa1099, "documents.sa_1099"),
    (FieldId::Sa5498Trustee, "sa_5498[0].trustee"),
    (FieldId::Sa5498TrusteeTin, "sa_5498[0].trustee_tin"),
    (FieldId::Sa5498TranscribedOn, "sa_5498[0].transcribed_on"),
    (
        FieldId::Sa5498Box1ArcherContributions,
        "sa_5498[0].box1_archer_msa_contributions",
    ),
    (
        FieldId::Sa5498Box2TotalContributions,
        "sa_5498[0].box2_total_contributions",
    ),
    (
        FieldId::Sa5498Box3NextYearForThisYear,
        "sa_5498[0].box3_contributions_next_year_for_this_year",
    ),
    (
        FieldId::Sa5498Box4Rollover,
        "sa_5498[0].box4_rollover_contributions",
    ),
    (FieldId::Sa5498Box5Fmv, "sa_5498[0].box5_fair_market_value"),
    (
        FieldId::Sa5498Box6AccountType,
        "sa_5498[0].box6_account_type",
    ),
    (FieldId::DocSa5498, "documents.sa_5498"),
    (
        FieldId::HsaLine2Contributions,
        "hsa.line2_contributions_you_made",
    ),
    (
        FieldId::HsaEmployerPriorYear,
        "hsa.employer_contributions_prior_year",
    ),
    (
        FieldId::HsaEmployerNextYear,
        "hsa.employer_contributions_next_year",
    ),
    (
        FieldId::HsaLine10FundingDistribution,
        "hsa.line10_qualified_funding_distribution",
    ),
    (
        FieldId::HsaLine14bRollovers,
        "hsa.line14b_rollovers_and_withdrawn_excess",
    ),
    (
        FieldId::HsaLine15MedicalExpenses,
        "hsa.line15_qualified_medical_expenses",
    ),
    (
        FieldId::HsaLine16Excepted,
        "hsa.line16_amount_meeting_an_exception",
    ),
    (FieldId::DeclHsaFamilyCoverage, "hsa.family_coverage"),
    (
        FieldId::DeclHsaEligibleEveryMonth,
        "hsa.eligible_every_month_same_coverage",
    ),
    (
        FieldId::DeclHsaAge55OrOlder,
        "hsa.age_55_or_older_at_year_end",
    ),
    (
        FieldId::DeclHsaMedicareEnrollment,
        "hsa.enrolled_in_medicare_any_month",
    ),
    (
        FieldId::DeclHsaBothSpousesHaveHsas,
        "hsa.both_spouses_have_hsas",
    ),
    (FieldId::DeclHsaArcherMsaActivity, "hsa.archer_msa_activity"),
    (
        FieldId::DeclHsaTestingPeriodFailure,
        "hsa.testing_period_failure",
    ),
    // ── ★★★ The T16 SEAM REVIEW's two declarations. ──
    (
        FieldId::DeclHsaSpouseFamilyCoverage,
        "hsa.spouse_family_coverage",
    ),
    (
        FieldId::DeclHsaDistributionWithout1099sa,
        "hsa_distribution_without_1099sa",
    ),
    // ── ★★★ T7 / R6 — Step 5 question 1, the one dependent gate about the FILER. ──
    (
        FieldId::DeclFilerTinIssuedByDueDate,
        "header.filer_tin_issued_by_due_date",
    ),
    // ── ★★★ R7 / T8 — HEAD OF HOUSEHOLD, QUALIFYING SURVIVING SPOUSE, and FR-67's election gate. ──
    (
        FieldId::DeclHohQualifyingPerson,
        "header.hoh_qualifying_person",
    ),
    (
        FieldId::DeclHohPaidOverHalfCostOfKeepingUpHome,
        "header.hoh_paid_over_half_cost_of_keeping_up_home",
    ),
    (FieldId::HohMaritalBasis, "header.hoh_marital_basis"),
    (FieldId::QualifyingChildName, "header.qualifying_child_name"),
    (
        FieldId::DeclNraSpouseResidentElection,
        "header.nra_spouse_resident_election",
    ),
    (
        FieldId::DeclQssSpouseDiedInWindow,
        "header.qss_spouse_died_in_window_and_not_remarried",
    ),
    (
        FieldId::DeclQssChildYouCanClaim,
        "header.qss_child_you_can_claim",
    ),
    (
        FieldId::DeclQssChildLivedAllYear,
        "header.qss_child_lived_in_your_home_all_year",
    ),
    (
        FieldId::DeclQssPaidOverHalfCost,
        "header.qss_paid_over_half_cost_of_keeping_up_home",
    ),
    (
        FieldId::DeclQssCouldHaveFiledJointly,
        "header.qss_could_have_filed_jointly_in_year_of_death",
    ),
    // ── ★★★ R5 / T5 — the filer's-records rows for Schedule B lines 1 and 5. ──
    (
        FieldId::SbRecordPayerName,
        "schedule_b_filer_records[0].payer_name",
    ),
    (
        FieldId::SbRecordPayerSsn,
        "schedule_b_filer_records[0].payer_ssn",
    ),
    (
        FieldId::SbRecordPayerAddress,
        "schedule_b_filer_records[0].payer_address",
    ),
    (
        FieldId::SbRecordAmount,
        "schedule_b_filer_records[0].amount",
    ),
    (FieldId::SbRecordKind, "schedule_b_filer_records[0].kind"),
];
