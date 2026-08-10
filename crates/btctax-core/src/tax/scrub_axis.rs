//! **THE DERIVED REPLACED-FIELD AXIS** — SPEC_income_scrub.md §3.3, and §8 step 4's first act.
//!
//! ## Why this module exists at all
//!
//! §3.2 states scrub's rule over a **mechanism**: *every field `scrub_pii` replaces must preserve
//! every property any screen, `ReturnHeader::build`, or form filler reads from it.* Five review
//! passes then found, at five successive depths, that every *instrument* written to enforce that
//! rule had been a **hand-list one level down** — four field names, two divergence entries, and
//! finally a "derivation" that named no operation a test could compute.
//!
//! So the set of fields `scrub_pii` replaces is **derived**, never listed:
//!
//! > the set of paths at which `serde_json::to_value(ri)` and `to_value(scrub_pii(&ri))` **differ**,
//! > over a MAXIMAL sentinel fixture — and a path present on one side and absent on the other is a
//! > difference.
//!
//! ## ★★★ The blind spot is STRUCTURAL ABSENCE, and naming the wrong one is how this stayed broken
//!
//! An earlier draft guarded *value collision* — "a field whose fixture value happens to equal its
//! stand-in shows no diff". That is real, and rare, and **not** the hazard. The hazard is that a
//! replaced field produces no differing path when the fixture **never instantiates it**: `None` on
//! both sides, `""` on both sides, or inside an empty `Vec`. Measured against the repo's natural
//! sentinel base (`kitchen_sink_household`), the derived axis would have silently omitted
//! `w2s[].ein`, `header.ip_pin`, `foreign_country_names` and `b_1099[].payer` — **all four of the
//! fields the review passes were about**, including the one carrying §3.1's Critical. Zero fixtures
//! in `testonly.rs` set `ip_pin`, `foreign_country_names`, or any `ein:` at all.
//!
//! Hence [`maximal_sentinel`], and hence the fact that its maximality is about **value as well as
//! presence** — §3.2's trim-emptiness rule makes every replaced string *conditionally* replaced, so a
//! plain `String` left `""` collapses exactly as an absent `Option` does.
//!
//! ★ Both collapse modes are different mechanisms, so each gets its own kill-test (B1). A checker
//! that has never been observed discriminating does not exist.

use crate::tax::return_inputs::ReturnInputs;
use std::collections::BTreeSet;

/// Walk two JSON values and record every path at which they differ.
///
/// ★ Array elements collapse to a single `[]` segment, so the axis is **field-shaped, not
/// index-shaped**: `w2s[0].ein` and `w2s[1].ein` are one member, `w2s[].ein`. That is the vocabulary
/// §5.1's divergence table is keyed in, so the two are checkable against each other.
///
/// ★★ A key present on one side and absent on the other **is a difference**. `header.ip_pin` is
/// replaced with `None`, so under `toml::Value` — which omits a `None` key entirely — it would differ
/// only by absence and a differ walking one side's keys would miss it. §5.1 argues at length that a
/// cell going *missing* is as much a divergence as one whose contents change; the differ has to
/// agree. `serde_json` renders `None` as `Value::Null` at a present key, which is the other half of
/// why it is the right serializer here.
fn diff_paths(
    a: &serde_json::Value,
    b: &serde_json::Value,
    path: &str,
    out: &mut BTreeSet<String>,
) {
    use serde_json::Value;
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for k in keys {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match (x.get(k), y.get(k)) {
                    (Some(av), Some(bv)) => diff_paths(av, bv, &child, out),
                    // Present on one side only — a PRESENCE difference, which counts.
                    _ => {
                        out.insert(child);
                    }
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            let child = format!("{path}[]");
            if x.len() != y.len() {
                out.insert(child.clone());
            }
            for (av, bv) in x.iter().zip(y.iter()) {
                diff_paths(av, bv, &child, out);
            }
        }
        _ => {
            out.insert(path.to_string());
        }
    }
}

/// The **derived** set of fields `scrub_pii` replaces, for `ri`.
///
/// This is the authority for §3.3's matrix field axis. It is not a `const` list and must never
/// become one: Rust has no reflection over "which fields did this function change", which is exactly
/// why a hand-list looks like a mechanism and is not one.
#[must_use]
pub fn replaced_paths(ri: &ReturnInputs) -> BTreeSet<String> {
    let a = serde_json::to_value(ri).expect("ReturnInputs serializes");
    let b = serde_json::to_value(super::scrub::scrub_pii(ri)).expect("ReturnInputs serializes");
    let mut out = BTreeSet::new();
    diff_paths(&a, &b, "", &mut out);
    out
}

/// A **MAXIMAL** sentinel `ReturnInputs`: every `Option` is `Some`, every `Vec` holds **two**
/// elements, every nested struct is present, and every string is **non-empty and distinct**.
///
/// ★ Written as an exhaustive struct literal — **no `..`, no `..Default::default()`** — for every
/// struct it reaches. A field added anywhere is then a compile error *here*, not a quietly smaller
/// axis. This is the same construction the scrubber's own `..`-free destructure uses, applied to the
/// instrument that polices it.
///
/// ★★ **Two `Vec` elements, not one**, so an index-varying stand-in (`Payer{i+1}`) and `EinMap`'s
/// distinctness both show up as real differences.
///
/// ★ Every string is a distinct `SENTINEL_*` token so that no fixture value can silently equal its
/// own stand-in — the *secondary* precondition (§3.4 models it for one field with
/// `assert_ne!(original.address_city, SCRUB_CITY)`; here it is general by construction).
#[must_use]
pub fn maximal_sentinel() -> ReturnInputs {
    use crate::tax::return_inputs::*;
    use crate::tax::types::{Carryforward, FilingStatus};
    use rust_decimal_macros::dec;
    use time::macros::date;

    let person = |tag: &str| Person {
        first_name: format!("SENTINEL_first_{tag}"),
        last_name: format!("SENTINEL_last_{tag}"),
        // ★ VALID by shape, and distinct per person. See `maximal_sentinel`'s note.
        ssn: match tag {
            "taxpayer" => "000-11-1111",
            _ => "000-44-4444",
        }
        .to_string(),
        date_of_birth: Some(date!(1970 - 01 - 01)),
        date_of_death: Some(date!(2024 - 06 - 01)),
        blind: Some(true),
        occupation: format!("SENTINEL_occupation_{tag}"),
    };
    let dependent = |tag: &str| Dependent {
        name: format!("SENTINEL_dependent_name_{tag}"),
        ssn: match tag {
            "one" => "000-22-2222",
            _ => "000-33-3333",
        }
        .to_string(),
        relationship: format!("SENTINEL_relationship_{tag}"),
        date_of_birth: Some(date!(2015 - 03 - 03)),
    };
    let w2 = |tag: &str, ein: &str| W2 {
        owner: Owner::Taxpayer,
        employer: format!("SENTINEL_employer_{tag}"),
        ein: Some(ein.to_string()),
        box1_wages: dec!(1000),
        box2_fed_withheld: dec!(10),
        box3_ss_wages: dec!(1000),
        box4_ss_withheld: dec!(62),
        box5_medicare_wages: dec!(1000),
        box6_medicare_withheld: dec!(14),
        box7_ss_tips: dec!(1),
        box17_state_tax_withheld: dec!(5),
        box19_local_tax: dec!(2),
        box12: vec![
            Box12Entry {
                code: format!("SENTINEL_box12_code_{tag}_a"),
                amount: dec!(3),
            },
            Box12Entry {
                code: format!("SENTINEL_box12_code_{tag}_b"),
                amount: dec!(4),
            },
        ],
        box8_allocated_tips: dec!(0),
        box10_dependent_care: dec!(0),
    };
    let int_1099 = |tag: &str| Form1099Int {
        payer: format!("SENTINEL_int_payer_{tag}"),
        box1_interest: dec!(11),
        box2_early_withdrawal_penalty: dec!(1),
        box3_treasury_interest: dec!(2),
        box4_fed_withheld: dec!(3),
        box6_foreign_tax: dec!(4),
        box8_tax_exempt_interest: dec!(5),
        box9_private_activity_bond_amt: dec!(0),
    };
    let div_1099 = |tag: &str| Form1099Div {
        payer: format!("SENTINEL_div_payer_{tag}"),
        box1a_ordinary: dec!(21),
        box1b_qualified: dec!(11),
        box2a_capgain_distr: dec!(1),
        box2b_unrecap_1250: dec!(0),
        box2c_section_1202: dec!(0),
        box2d_collectibles_28: dec!(0),
        box4_fed_withheld: dec!(2),
        box5_section_199a: dec!(3),
        box7_foreign_tax: dec!(4),
        box12_exempt_interest_dividends: dec!(5),
        box13_private_activity_amt: dec!(0),
    };
    let g_1099 = |tag: &str| Form1099G {
        payer: format!("SENTINEL_g_payer_{tag}"),
        box1_unemployment: dec!(31),
        box4_fed_withheld: dec!(3),
    };
    let b_1099 = |tag: &str| Form1099B {
        payer: format!("SENTINEL_b_payer_{tag}"),
        short_term_proceeds: dec!(41),
        short_term_basis: dec!(40),
        long_term_proceeds: dec!(51),
        long_term_basis: dec!(50),
        basis_reported_and_no_adjustments: Some(true),
    };

    ReturnInputs {
        tax_year: 2024,
        filing_status: FilingStatus::Mfj,
        header: HouseholdHeader {
            taxpayer: person("taxpayer"),
            spouse: Some(person("spouse")),
            address_street: "SENTINEL_street".into(),
            address_city: "SENTINEL_city".into(),
            address_state: "SENTINEL_state".into(),
            address_zip: "SENTINEL_zip".into(),
            dependents: vec![dependent("one"), dependent("two")],
            can_be_claimed_as_dependent_taxpayer: Some(false),
            can_be_claimed_as_dependent_spouse: Some(false),
            spouse_had_no_income: Some(false),
            spouse_not_filing_a_return: Some(false),
            presidential_fund_taxpayer: true,
            presidential_fund_spouse: true,
            taxpayer_died_during_year: Some(false),
            spouse_died_during_year: Some(false),
            ip_pin: Some("654321".into()), // ★ VALID (six digits) — the baseline must be clean
        },
        w2s: vec![w2("one", "11-1111111"), w2("two", "22-2222222")],
        int_1099: vec![int_1099("one"), int_1099("two")],
        div_1099: vec![div_1099("one"), div_1099("two")],
        g_1099: vec![g_1099("one"), g_1099("two")],
        b_1099: vec![b_1099("one"), b_1099("two")],
        schedule_c: Some(ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "SENTINEL_business_description".into(),
            naics_code: "SENTINEL_naics".into(),
            accounting_method: AccountingMethod::Cash,
            expenses: dec!(100),
            other_gross_receipts: dec!(200),
            payments_requiring_1099: Some(false),
            will_file_required_1099: Some(false),
            qbi_w2_wages: Some(dec!(0)),
            qbi_ubia: Some(dec!(0)),
            is_sstb: Some(false),
            is_cooperative_patron: Some(false),
        }),
        schedule_a: Some(ScheduleAInputs {
            medical: dec!(1),
            salt_use_sales_tax: Some(false),
            salt_sales_tax_amount: dec!(0),
            salt_state_estimated_payments: dec!(2),
            salt_prior_year_balance_paid: dec!(3),
            salt_real_estate: dec!(4),
            salt_personal_property: dec!(5),
            mortgage_interest_1098: dec!(6),
            mortgage_all_used_to_buy_build_improve: Some(true),
            mortgage_dwelling_is_amt_qualified: Some(true),
            charitable: vec![
                CharitableGift {
                    class: CharitableClass::Cash60,
                    amount: dec!(7),
                },
                CharitableGift {
                    class: CharitableClass::Cash30,
                    amount: dec!(8),
                },
            ],
        }),
        itemize_election: ItemizeElection::Auto,
        mfs_spouse_itemizes: Some(false),
        sch1: Schedule1Inputs {
            state_refund_taxable: dec!(1),
            student_loan_interest_paid: dec!(2),
            ira_deduction_claimed: dec!(3),
            hsa_activity: Some(false),
        },
        payments: Payments {
            estimated_tax_payments: dec!(1),
            extension_payment: dec!(2),
            other_withholding: dec!(3),
        },
        capital_loss_carryforward_in: Carryforward {
            short: dec!(1),
            long: dec!(2),
        },
        capital_loss_carryforward_in_provenance: CarryProvenance::User,
        amt_carryover_same_as_regular: Some(true),
        amt_depreciation_same_as_regular: Some(true),
        charitable_carryover_in: vec![
            CharitableCarryItem {
                class: CharitableClass::Cash60,
                amount: dec!(1),
                origin_year: 2022,
                provenance: CarryProvenance::User,
            },
            CharitableCarryItem {
                class: CharitableClass::Cash30,
                amount: dec!(2),
                origin_year: 2023,
                provenance: CarryProvenance::User,
            },
        ],
        charitable_carryover_in_provenance: CarryProvenance::User,
        qbi: QbiInputs {
            reit_ptp_carryforward_in: dec!(1),
            reit_ptp_carryforward_in_provenance: CarryProvenance::User,
            qbi_carryforward_in: dec!(2),
            qbi_carryforward_in_provenance: CarryProvenance::User,
        },
        foreign_accounts: Some(true),
        foreign_trust: Some(true),
        foreign_country_names: "SENTINEL_countryA, SENTINEL_countryB".into(),
        fbar_filing_required: Some(true),
        donations_had_restrictions: Some(false),
        dual_status_alien: Some(false),
        has_income_exclusion: Some(false),
        other_out_of_scope_income: Some(false),
        excluded_puerto_rico_income: dec!(0),
        form_2555_line45: dec!(0),
        form_2555_line50: dec!(0),
        form_4563_line15: dec!(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **THE ONE-COMMAND ANSWER TO FIVE REVIEW PASSES.**
    ///
    /// Every pass was about the same handful of fields, and each pass's instrument failed to reach
    /// them for a *different* reason. This asserts the derivation reaches them. The five names are a
    /// **BACKSTOP against a non-maximal fixture — they do NOT define the axis**, which is whatever
    /// the diff yields; if the scrubber gains a field tomorrow it appears here with no edit.
    #[test]
    fn the_derived_axis_reaches_every_field_the_review_passes_were_about() {
        let paths = replaced_paths(&maximal_sentinel());
        for want in [
            "w2s[].ein",                       // §3.1's CRITICAL — the manufactured §6413(c) credit
            "header.ip_pin",                   // replaced with None; PRINTED on the 1040
            "foreign_country_names",           // printed verbatim on Schedule B line 7b
            "b_1099[].payer",                  // the "(unnamed broker)" refusal-message flip
            "schedule_c.business_description", // trim-empty ⇒ a refusal the copy would not raise
        ] {
            assert!(
                paths.contains(want),
                "the derived axis is missing `{want}` — the sentinel fixture is not maximal and the \
                 derivation is BLIND on it.\nderived: {paths:#?}"
            );
        }
    }

    /// ★ B1 kill-test, collapse mode 1 of 2: **structural absence**.
    ///
    /// With `ip_pin: None` the field is `None` on both sides, so no path differs and it drops out of
    /// the axis silently — nothing reds, and §3.3's matrix would never demand a cell for it. This is
    /// the mode that hid four fields on the repo's natural fixture, and asserting it here is what
    /// makes `maximal_sentinel`'s every-`Option`-is-`Some` clause load-bearing rather than a
    /// stylistic preference.
    #[test]
    fn a_none_option_drops_out_of_the_axis_which_is_why_the_fixture_must_be_maximal() {
        let mut ri = maximal_sentinel();
        assert!(replaced_paths(&ri).contains("header.ip_pin"));

        ri.header.ip_pin = None;
        assert!(
            !replaced_paths(&ri).contains("header.ip_pin"),
            "a None Option must vanish from the derived axis — if this ever fails the blind spot has \
             moved and the maximality clause needs rewriting, not deleting"
        );
    }

    /// ★ B1 kill-test, collapse mode 2 of 2: **emptiness**. A different mechanism from absence, so
    /// one test cannot witness both.
    ///
    /// §3.2 makes every replaced string *conditionally* replaced — trim-empty in ⇒ `""` out — so a
    /// plain `String` left `""` yields `""` on both sides and collapses exactly as an absent `Option`
    /// does. Presence-only maximality would have re-opened the whole hole one field to the left of
    /// where it was closed.
    #[test]
    fn an_empty_string_drops_out_of_the_axis_too() {
        let mut ri = maximal_sentinel();
        assert!(replaced_paths(&ri).contains("schedule_c.business_description"));

        ri.schedule_c.as_mut().unwrap().business_description = String::new();
        assert!(
            !replaced_paths(&ri).contains("schedule_c.business_description"),
            "an empty String must vanish from the derived axis — this is why maximality is about \
             VALUE as well as presence"
        );
    }

    /// ★★ **THE SENTINELS THAT SURVIVE ARE EXACTLY THE FIELDS SCRUB KEEPS ON PURPOSE** — no more, no
    /// fewer. Both directions matter, and neither is the same test:
    ///
    /// - a sentinel that survives on a field NOT in this list is an identity value riding into a file
    ///   the command calls shareable;
    /// - a name leaving this list means a KEPT decision was silently reversed, which moves a figure
    ///   (`relationship` decides child-vs-other-dependent) or destroys a reproducer's fidelity.
    ///
    /// The three are §7's recorded decisions: `relationship` is computational, a **box-12 code is a
    /// taxonomy, not a person**, and `naics_code` is a six-digit federal industry code shared by
    /// thousands of businesses and printed on Schedule C line B.
    #[test]
    fn the_surviving_sentinels_are_exactly_the_deliberately_kept_fields() {
        let ri = maximal_sentinel();
        let scrubbed = super::super::scrub::scrub_pii(&ri);
        let json = serde_json::to_string(&scrubbed).unwrap();

        // Each sentinel is `SENTINEL_<tag>_<suffix>`; group by the field it names.
        let mut survived: BTreeSet<&str> = BTreeSet::new();
        for kept in [
            "SENTINEL_relationship",
            "SENTINEL_box12_code",
            "SENTINEL_naics",
            "SENTINEL_first",
            "SENTINEL_last",
            "SENTINEL_ssn",
            "SENTINEL_occupation",
            "SENTINEL_dependent_name",
            "SENTINEL_dependent_ssn",
            "SENTINEL_street",
            "SENTINEL_city",
            "SENTINEL_state",
            "SENTINEL_zip",
            "SENTINEL_ippin",
            "SENTINEL_employer",
            "SENTINEL_int_payer",
            "SENTINEL_div_payer",
            "SENTINEL_g_payer",
            "SENTINEL_b_payer",
            "SENTINEL_business_description",
            "SENTINEL_countryA",
            "SENTINEL_countryB",
        ] {
            if json.contains(kept) {
                survived.insert(kept);
            }
        }
        let expected: BTreeSet<&str> = [
            "SENTINEL_relationship",
            "SENTINEL_box12_code",
            "SENTINEL_naics",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            survived, expected,
            "the set of sentinels surviving a scrub must be exactly the fields §7 records as KEPT \
             (relationship, box-12 code, NAICS). Anything extra is identity riding into a shareable \
             file; anything missing is a KEPT decision silently reversed."
        );
    }

    /// The *secondary* precondition (§3.4 models it for one field with
    /// `assert_ne!(original.address_city, SCRUB_CITY)`): a fixture value that happens to equal its own
    /// stand-in shows no diff and hides its field from the axis. Here it is general — no stand-in
    /// string scrub can emit may appear anywhere in the ORIGINAL fixture.
    #[test]
    fn no_fixture_value_collides_with_a_stand_in() {
        let json = serde_json::to_string(&maximal_sentinel()).unwrap();
        for stand_in in [
            "Taxpayer",
            "Spouse",
            "Example",
            "Occupation",
            "1 Example St",
            "Springfield",
            "62704",
            "Dependent1",
            "Example business",
            "Country1",
            "Employer1",
            "Payer1",
            "Broker1",
            "Agency1",
        ] {
            assert!(
                !json.contains(stand_in),
                "the sentinel fixture contains `{stand_in}`, which is a value scrub itself emits — \
                 that field would show NO diff and drop out of the derived axis silently"
            );
        }
    }
}

/// **§3.3's FIXTURE MATRIX** — `{derived field set} × {absent, empty, malformed, valid}`, where every
/// cell carries one of three verdicts and the loop **fails on a derived path with no row**.
///
/// ★★★ Why the third verdict is not optional. Without a `NoSuchState` arm the loop is unsatisfiable —
/// there is no fixture with a malformed `occupation` — and the only escape is an exception hand-list,
/// reintroducing exactly what the derivation removes. This project's own conformance rule: a checker
/// that cannot tell *"this cell encodes no decision"* from *"we forgot this cell"* is not a
/// conformance check.
///
/// ★★★ AND THE REASON IS NEVER "THE TYPE". The two axes have **different discriminators**, and
/// conflating them licenses exempting a cell on which the scrubber is wrong:
///
/// - `absent` — **decided by the type**: representable iff the field sits in an `Option` or a `Vec`.
///   For a plain `String` in a present struct, `absent ≡ empty` (both are `""`), so it defers to the
///   `empty` cell rather than claiming a state it cannot hold.
/// - `malformed` — **decided by whether any predicate reads a VALIDITY CLASS off the field**, which is
///   §3.2's ★ mechanism. `header.taxpayer.ssn` is a plain `String` *with* a malformed state, so the
///   type plainly is not what decides. There are exactly three such readers, and they enumerate the
///   malformed-bearing cells: `Ssn::canonical` (`packet.rs:208` taxpayer/spouse, `:425` dependents),
///   `IpPin::canonical` (`packet.rs:169`), `canonical_ein` (`return_1040.rs:681`).
///
///   A type-authorised rule would mark `header.spouse.ssn` and `header.dependents[].ssn` as having no
///   malformed state — *correctly by that wording* — and the loop would stay green with no fixture.
///   But an eight-digit dependent SSN **refuses on the original and EXPORTS on the scrubbed copy**.
#[cfg(test)]
mod matrix {
    use super::*;
    use crate::tax::packet::{HeaderError, ReturnHeader, SsnError};
    use crate::tax::return_inputs::ReturnInputs;
    use crate::tax::return_refuse::screen_inputs;
    use crate::tax::testonly::{ty2024_params, ty2024_table};

    /// A cell is either exercised by a fixture mutation, or explicitly recorded as unrepresentable
    /// **with the reason** — never silently absent.
    enum Cell {
        Fixture(fn(&mut ReturnInputs)),
        NoSuchState(&'static str),
    }
    use Cell::{Fixture, NoSuchState};

    const NO_READER: &str =
        "no predicate reads a validity class off this field — not the type, the absence of a reader";
    const PLAIN_STRING: &str =
        "a plain String in a present struct: absent is the same value as empty";

    /// `[path, absent, empty, malformed]`. `valid` is the maximal sentinel itself and needs no row.
    #[allow(clippy::type_complexity)]
    fn matrix() -> Vec<(&'static str, Cell, Cell, Cell)> {
        vec![
            (
                "header.taxpayer.ssn",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.taxpayer.ssn = String::new()),
                Fixture(|r| r.header.taxpayer.ssn = "123-45-678".into()),
            ),
            (
                "header.taxpayer.first_name",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.taxpayer.first_name = "   ".into()),
                NoSuchState(NO_READER),
            ),
            (
                "header.taxpayer.last_name",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.taxpayer.last_name = String::new()),
                NoSuchState(NO_READER),
            ),
            (
                "header.taxpayer.occupation",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.taxpayer.occupation = String::new()),
                NoSuchState(NO_READER),
            ),
            // ★ The spouse is an Option<Person>, so its fields DO have an absent state.
            (
                "header.spouse.ssn",
                Fixture(|r| r.header.spouse = None),
                Fixture(|r| {
                    if let Some(s) = r.header.spouse.as_mut() {
                        s.ssn = String::new();
                    }
                }),
                Fixture(|r| {
                    if let Some(s) = r.header.spouse.as_mut() {
                        s.ssn = "12345678X".into();
                    }
                }),
            ),
            (
                "header.spouse.first_name",
                Fixture(|r| r.header.spouse = None),
                Fixture(|r| {
                    if let Some(s) = r.header.spouse.as_mut() {
                        s.first_name = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "header.spouse.last_name",
                Fixture(|r| r.header.spouse = None),
                Fixture(|r| {
                    if let Some(s) = r.header.spouse.as_mut() {
                        s.last_name = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "header.spouse.occupation",
                Fixture(|r| r.header.spouse = None),
                Fixture(|r| {
                    if let Some(s) = r.header.spouse.as_mut() {
                        s.occupation = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            // ★★ THE LIVE VECTOR. Ssn::canonical reads every dependent (packet.rs:425) and
            //    scrub_dependent wrote a well-formed stand-in unconditionally, so an ordinary typo
            //    refused on the original and EXPORTED on the copy.
            (
                "header.dependents[].ssn",
                Fixture(|r| r.header.dependents.clear()),
                Fixture(|r| r.header.dependents[0].ssn = String::new()),
                Fixture(|r| r.header.dependents[0].ssn = "123-45-678".into()),
            ),
            (
                "header.dependents[].name",
                Fixture(|r| r.header.dependents.clear()),
                Fixture(|r| r.header.dependents[0].name = String::new()),
                NoSuchState(NO_READER),
            ),
            (
                "header.address_street",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.address_street = String::new()),
                NoSuchState(NO_READER),
            ),
            (
                "header.address_city",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.address_city = String::new()),
                NoSuchState(NO_READER),
            ),
            (
                "header.address_state",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.address_state = String::new()),
                NoSuchState(NO_READER),
            ),
            (
                "header.address_zip",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.header.address_zip = String::new()),
                NoSuchState(NO_READER),
            ),
            (
                "header.ip_pin",
                Fixture(|r| r.header.ip_pin = None),
                Fixture(|r| r.header.ip_pin = Some(String::new())),
                Fixture(|r| r.header.ip_pin = Some("12345".into())),
            ),
            (
                "w2s[].ein",
                Fixture(|r| {
                    for w in &mut r.w2s {
                        w.ein = None;
                    }
                }),
                Fixture(|r| {
                    for w in &mut r.w2s {
                        w.ein = Some(String::new());
                    }
                }),
                Fixture(|r| {
                    for w in &mut r.w2s {
                        w.ein = Some("XX-1111111".into());
                    }
                }),
            ),
            (
                "w2s[].employer",
                Fixture(|r| r.w2s.clear()),
                Fixture(|r| {
                    for w in &mut r.w2s {
                        w.employer = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "int_1099[].payer",
                Fixture(|r| r.int_1099.clear()),
                Fixture(|r| {
                    for f in &mut r.int_1099 {
                        f.payer = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "div_1099[].payer",
                Fixture(|r| r.div_1099.clear()),
                Fixture(|r| {
                    for f in &mut r.div_1099 {
                        f.payer = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "g_1099[].payer",
                Fixture(|r| r.g_1099.clear()),
                Fixture(|r| {
                    for f in &mut r.g_1099 {
                        f.payer = String::new();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            // ★ `screen_inputs` reads `b.payer.trim().is_empty()` to choose the refusal WORDING.
            (
                "b_1099[].payer",
                Fixture(|r| r.b_1099.clear()),
                Fixture(|r| {
                    for f in &mut r.b_1099 {
                        f.payer = "   ".into();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "schedule_c.business_description",
                Fixture(|r| r.schedule_c = None),
                Fixture(|r| {
                    if let Some(c) = r.schedule_c.as_mut() {
                        c.business_description = "   ".into();
                    }
                }),
                NoSuchState(NO_READER),
            ),
            (
                "foreign_country_names",
                NoSuchState(PLAIN_STRING),
                Fixture(|r| r.foreign_country_names = String::new()),
                NoSuchState(NO_READER),
            ),
        ]
    }

    /// §3.3 test 1 — the same `RefuseReason` VARIANT on both sides.
    fn assert_same_refusal(orig: &ReturnInputs, scrubbed: &ReturnInputs, label: &str) {
        let tbl = ty2024_table();
        let p = ty2024_params();
        let a = screen_inputs(orig, &tbl, &p).map(|r| r.reason);
        let b = screen_inputs(scrubbed, &tbl, &p).map(|r| r.reason);
        assert_eq!(
            a, b,
            "{label}: scrubbing changed WHETHER (or why) the return refuses. The filer is sending \
             this file to reproduce a refusal; a copy that refuses differently sends the recipient \
             after a bug that is not there — or, worse, FILES where the original could not."
        );
    }

    /// ★★★ **TEST 1 IS STRUCTURALLY BLIND TO ONE THING, SO THIS IS THE THIRD ASSERTION.**
    ///
    /// `screen_inputs` reads `b.payer.trim().is_empty()` only to choose the refusal *wording* —
    /// `"(unnamed broker)"` versus the payer's name (`return_refuse.rs:672-676`). The
    /// `RefuseReason` VARIANT is `Form1099BNeedsForm8949` either way, so test 1 cannot see the flip:
    /// mutation-verified, and it survived. Comparing the refusal MESSAGE verbatim is not the fix
    /// either — the message legitimately contains the scrubbed payer name, so it would red on a
    /// CORRECT scrub, the same shape as the figure invariant needing normalization.
    ///
    /// So assert the thing §3.2 actually states, directly and over the **whole tree**: for every
    /// string leaf, **trim-emptiness is preserved**. That is derived rather than hand-listed — it
    /// covers every replaced string and every future one — and it is what makes the `""` cells of the
    /// matrix load-bearing instead of decorative.
    fn assert_emptiness_class_preserved(orig: &ReturnInputs, scrubbed: &ReturnInputs, label: &str) {
        fn walk(a: &serde_json::Value, b: &serde_json::Value, path: &str, label: &str) {
            use serde_json::Value;
            match (a, b) {
                (Value::String(x), Value::String(y)) => assert_eq!(
                    x.trim().is_empty(),
                    y.trim().is_empty(),
                    "{label}: `{path}` changed TRIM-EMPTINESS class ({x:?} -> {y:?}). Every screen \
                     keys on `.trim().is_empty()`, so this flips what the recipient's copy does."
                ),
                (Value::Object(x), Value::Object(y)) => {
                    for (k, av) in x {
                        if let Some(bv) = y.get(k) {
                            let child = if path.is_empty() {
                                k.clone()
                            } else {
                                format!("{path}.{k}")
                            };
                            walk(av, bv, &child, label);
                        }
                    }
                }
                (Value::Array(x), Value::Array(y)) => {
                    for (av, bv) in x.iter().zip(y.iter()) {
                        walk(av, bv, &format!("{path}[]"), label);
                    }
                }
                _ => {}
            }
        }
        let a = serde_json::to_value(orig).unwrap();
        let b = serde_json::to_value(scrubbed).unwrap();
        walk(&a, &b, "", label);
    }

    /// §3.3 test 2 — the same `HeaderError` **by value**, with `NotDigits` by discriminant only.
    ///
    /// ★★★ The asymmetry is the fix, and it is deliberate. Value equality is this repo's own idiom on
    /// this call (`return_refuse.rs:1527` asserts `SsnError::NotDigits('X')` as a *value*) and is the
    /// only comparison that holds §3.2's explicitly-payloaded `WrongLength(n) → WrongLength(n)`. But
    /// applied to `NotDigits` it would force the scrubbed SSN to carry the filer's REAL offending
    /// character into the shareable file — and the test would bless it, green. Discriminant equality
    /// everywhere instead drops the `WrongLength` fidelity r1 filed the section to add.
    fn assert_same_header_error(orig: &ReturnInputs, scrubbed: &ReturnInputs, label: &str) {
        let norm = |e: Option<HeaderError>| match e {
            // Coarsen ONLY NotDigits' payload — §3.2 mandates it change, so comparing it by value
            // would demand the leak.
            Some(HeaderError::Ssn(SsnError::NotDigits(_))) => {
                Some(HeaderError::Ssn(SsnError::NotDigits('?')))
            }
            other => other,
        };
        let a = norm(ReturnHeader::build(orig, 2024).err());
        let b = norm(ReturnHeader::build(scrubbed, 2024).err());
        assert_eq!(
            a, b,
            "{label}: scrubbing changed the packet-boundary error. Same variant AND same payload — \
             `is_err()` equality would admit error substitution, and a WrongLength(4) reported as \
             WrongLength(2) sends the recipient after the wrong defect."
        );
    }

    /// ★★★ THE MATRIX. Driven by the DERIVED axis, so a field added to the scrubber cannot reach a
    /// release without someone deciding its class behaviour — the loop fails on a path with no row.
    #[test]
    fn every_replaced_field_preserves_its_class_in_every_representable_state() {
        let base = maximal_sentinel();
        let derived = replaced_paths(&base);
        let table = matrix();

        // (a) Every DERIVED path has a row. This is the anti-hand-list check: the table is a set of
        //     decisions, and its COMPLETENESS is policed by the derivation, not by the author.
        let rows: BTreeSet<&str> = table.iter().map(|(p, ..)| *p).collect();
        for path in &derived {
            assert!(
                rows.contains(path.as_str()),
                "the derived axis contains `{path}` but §3.3's matrix has no row for it. A field was \
                 added to scrub_pii without anyone deciding its class behaviour."
            );
        }
        // (b) …and no row names a path the derivation does not produce, or the table has drifted into
        //     asserting things about fields scrub no longer touches.
        for (path, ..) in &table {
            assert!(
                derived.contains(*path),
                "§3.3's matrix has a row for `{path}`, which is NOT in the derived axis — either the \
                 fixture stopped being maximal or scrub stopped replacing this field"
            );
        }

        // (c) Every cell: exercised, or recorded as unrepresentable with a reason.
        let mut exercised = 0usize;
        let mut excused = 0usize;
        for (path, absent, empty, malformed) in &table {
            for (state, cell) in [
                ("absent", absent),
                ("empty", empty),
                ("malformed", malformed),
            ] {
                match cell {
                    NoSuchState(reason) => {
                        assert!(!reason.is_empty(), "{path}/{state}: a reason is mandatory");
                        excused += 1;
                    }
                    Fixture(mutate) => {
                        let mut ri = maximal_sentinel();
                        mutate(&mut ri);
                        let scrubbed = super::super::scrub::scrub_pii(&ri);
                        let label = format!("{path} @ {state}");
                        assert_same_refusal(&ri, &scrubbed, &label);
                        assert_same_header_error(&ri, &scrubbed, &label);
                        assert_emptiness_class_preserved(&ri, &scrubbed, &label);
                        exercised += 1;
                    }
                }
            }
            // `valid` — the unmutated maximal fixture.
            let scrubbed = super::super::scrub::scrub_pii(&base);
            assert_same_refusal(&base, &scrubbed, &format!("{path} @ valid"));
            assert_same_header_error(&base, &scrubbed, &format!("{path} @ valid"));
            assert_emptiness_class_preserved(&base, &scrubbed, &format!("{path} @ valid"));
        }
        assert!(
            exercised > 0 && excused > 0,
            "a matrix with no exercised cells, or no excused ones, is not exercising the mechanism"
        );
    }
}
