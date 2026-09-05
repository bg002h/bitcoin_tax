//! ★★★ Schedule 1-A (TY2025) input surface — the fail-closed guarantee, held.
//!
//! An earlier review round found this part's danger twice, and **both Criticals were missing
//! ELIGIBILITY, not wrong arithmetic**: a filer who merely typed a car-loan interest figure was
//! handed up to $10,000 of deduction — on a lease, a used car, a non-US-assembled car, a pre-2025
//! loan, or negative equity. That UNDERSTATES tax, which is the direction this project treats as
//! worst. The structural answer is `widening-an-exemption-is-never-the-safe-edit`: enumerate the
//! YES-conditions and default to NO, so every omission fails closed.
//!
//! These tests are that guarantee. Without them the defaults are a convention.

use btctax_core::tax::return_inputs::{Schedule1aOvertime, Schedule1aTips, Schedule1aVehicle};
use rust_decimal_macros::dec;

/// ★★★ The headline: a vehicle nobody answered for does NOT qualify.
#[test]
fn a_defaulted_vehicle_claims_nothing() {
    assert!(
        !Schedule1aVehicle::default().qualifies(),
        "an unanswered vehicle must NOT qualify — a default that grants the deduction is exactly \
         the shipped defect this surface exists to prevent"
    );
}

fn fully_qualified() -> Schedule1aVehicle {
    Schedule1aVehicle {
        description: "truck".into(),
        interest_paid: dec!(1200),
        loan_originated_after_2024: true,
        loan_originated_by_you: true,
        proceeds_used_to_purchase: true,
        personal_use: true,
        secured_by_first_lien: true,
        original_use_starts_with_you: true,
        is_applicable_vehicle_class_under_14000_lbs: true,
        final_assembly_in_us: true,
        excludes_negative_equity: true,
    }
}

#[test]
fn a_vehicle_meeting_every_stated_condition_qualifies() {
    assert!(
        fully_qualified().qualifies(),
        "all nine §163(h)(4) conditions answered YES must qualify, or the surface is unusable"
    );
}

/// ★★★ **Each of the nine conditions is load-bearing on its own.** This is the test that would have
/// caught the original defect: it flips exactly one answer to `false` at a time and requires the
/// deduction to be denied every time. A condition that is collected but not consulted passes every
/// other test in this file.
#[test]
fn flipping_any_single_condition_to_no_disqualifies_the_vehicle() {
    type Flip = (&'static str, fn(&mut Schedule1aVehicle));
    let flips: [Flip; 9] = [
        ("loan originated after 2024-12-31", |v| {
            v.loan_originated_after_2024 = false
        }),
        ("loan originated by the filer", |v| {
            v.loan_originated_by_you = false
        }),
        ("purchase, not a lease", |v| {
            v.proceeds_used_to_purchase = false
        }),
        ("personal use", |v| v.personal_use = false),
        ("first lien", |v| v.secured_by_first_lien = false),
        ("original use starts with the filer", |v| {
            v.original_use_starts_with_you = false
        }),
        ("class and GVWR under 14,000 lb", |v| {
            v.is_applicable_vehicle_class_under_14000_lbs = false
        }),
        ("final assembly in the US", |v| {
            v.final_assembly_in_us = false
        }),
        ("excludes negative equity", |v| {
            v.excludes_negative_equity = false
        }),
    ];
    // The count is asserted so a TENTH condition added to the struct cannot be silently untested:
    // `qualifies()` destructures exhaustively, so the struct change compiles only after someone
    // edits it — and this catches the case where they edit it and forget the flip here.
    assert_eq!(
        flips.len(),
        9,
        "nine stated conditions; update this table with the struct"
    );

    for (why, flip) in flips {
        let mut v = fully_qualified();
        flip(&mut v);
        assert!(
            !v.qualifies(),
            "a vehicle failing \"{why}\" must NOT qualify — every condition the instructions state \
             is a bar on its own, and collecting one without consulting it is the same defect as \
             never collecting it"
        );
    }
}

/// The tips and overtime declarations default to NO for the same reason.
#[test]
fn tips_and_overtime_declarations_default_to_no() {
    let t = Schedule1aTips::default();
    assert!(
        !t.occupation_on_treasury_list,
        "the Treasury-list occupation gate is THE gating condition printed on the form and must \
         default to NO"
    );
    assert!(!t.excludes_unlisted_occupation_tips);
    assert!(!t.meets_qualified_tip_criteria);
    assert_eq!(
        t.qualified_tips_reported,
        btctax_core::conventions::Usd::ZERO
    );

    let o = Schedule1aOvertime::default();
    assert!(
        !o.is_flsa_premium_half_only,
        "the FLSA premium-half trap must default to NO"
    );
    assert!(!o.entitlement_arises_under_flsa);
    assert!(
        !o.excludes_amounts_counted_as_tips,
        "the no-double-dip-with-Part-II confirmation must default to NO"
    );
}

/// ★★★ **The Rust `Default` and the SERDE default are two different defaults, and only one of them
/// is what an imported TOML actually gets.**
///
/// Found by mutation: changing a field to `#[serde(default = "btrue")]` left
/// `a_defaulted_vehicle_claims_nothing` GREEN, because that test builds the value with
/// `Default::default()` — which serde attributes do not touch. A filer importing a TOML that simply
/// omits the key would have been handed the condition as YES, while every test kept passing.
///
/// That is this repo's standing shape: two blanks that look identical and are not the same thing.
/// So the fail-closed guarantee is asserted on the path a real import takes — deserialization of a
/// document that answers nothing.
#[test]
fn an_imported_document_that_answers_nothing_claims_nothing() {
    let v: Schedule1aVehicle =
        serde_json::from_str("{}").expect("every Sch 1-A vehicle field must be serde-defaulted");
    assert!(
        !v.qualifies(),
        "a TOML/JSON that omits every eligibility key must NOT qualify — serde defaults are the \
         ones an import actually gets, and a `true` among them grants a deduction nobody claimed"
    );
    // Named individually so the failure message says WHICH condition was granted, not merely that
    // the vehicle qualified.
    for (name, got) in [
        ("loan_originated_after_2024", v.loan_originated_after_2024),
        ("loan_originated_by_you", v.loan_originated_by_you),
        ("proceeds_used_to_purchase", v.proceeds_used_to_purchase),
        ("personal_use", v.personal_use),
        ("secured_by_first_lien", v.secured_by_first_lien),
        (
            "original_use_starts_with_you",
            v.original_use_starts_with_you,
        ),
        (
            "is_applicable_vehicle_class_under_14000_lbs",
            v.is_applicable_vehicle_class_under_14000_lbs,
        ),
        ("final_assembly_in_us", v.final_assembly_in_us),
        ("excludes_negative_equity", v.excludes_negative_equity),
    ] {
        assert!(
            !got,
            "serde default for `{name}` is TRUE — an unanswered import claims it"
        );
    }

    let t: Schedule1aTips = serde_json::from_str("{}").unwrap();
    assert!(
        !t.occupation_on_treasury_list,
        "serde default granted the tips occupation gate"
    );
    assert!(!t.excludes_unlisted_occupation_tips);
    assert!(!t.meets_qualified_tip_criteria);

    let o: Schedule1aOvertime = serde_json::from_str("{}").unwrap();
    assert!(
        !o.is_flsa_premium_half_only,
        "serde default granted the FLSA premium-half trap"
    );
    assert!(!o.entitlement_arises_under_flsa);
    assert!(!o.excludes_amounts_counted_as_tips);
}
