//! ★★★ Schedule 1-A (TY2025) compute — **the skip branches are the risk, not the arithmetic.**
//!
//! Lines 10, 18 and 27 all read *"…If zero or less, enter the amount from line N on line M"* — a
//! JUMP PAST the phase-out, not a clamp to zero. Line 33 is worse: *"If zero or less, enter $6,000
//! on line 35"* — a jump that writes a **nonzero constant** into a later line. Transcribing that one
//! as `-0-` loses the whole senior deduction for every filer under the threshold, and it agrees with
//! `max(0, …)` only because 6% × 0 = 0, so a `max(0, …)` transcription passes for the wrong reason
//! and breaks the moment the rate moves.
//!
//! These tests pin the BRANCHES, not the arithmetic that shadows them.

use btctax_core::conventions::Usd;
use btctax_core::tax::schedule_1a::{
    Schedule1aFacts, Schedule1aPartII, Schedule1aPartIII, Schedule1aPartIV, Schedule1aPartV,
    Schedule1aPartVI,
};
use btctax_core::tax::tables::schedule_1a_params;
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;

fn p() -> btctax_core::tax::tables::Schedule1aParams {
    schedule_1a_params(2025).expect("TY2025 has a Schedule 1-A")
}

fn facts(magi: Usd, status: FilingStatus) -> Schedule1aFacts {
    Schedule1aFacts {
        magi,
        status,
        taxpayer_qualifies_as_senior: false,
        spouse_qualifies_as_senior: false,
    }
}

/// ★★★ **Under the threshold, the capped figure carries STRAIGHT DOWN and the phase-out lines stay
/// BLANK.** A `max(0, …)` transcription would put the deduction at zero here.
#[test]
fn line10_under_the_threshold_jumps_past_the_phase_out_and_keeps_the_whole_deduction() {
    let p = p();
    let f = facts(dec!(50000), FilingStatus::Single); // well under the tips threshold
    let ii = Schedule1aPartII::compute(&p, &f, Some(dec!(4000)), None);

    assert_eq!(ii.line7, Some(dec!(4000)), "L7 = smaller of L6 and the cap");
    assert_eq!(
        ii.line13,
        Some(dec!(4000)),
        "L10 is zero-or-less, so the form says enter the amount from line 7 ON LINE 13 — the whole \
         deduction survives. A clamp-to-zero reading loses it for most filers."
    );
    assert_eq!(
        ii.line10, None,
        "the jump SKIPS line 10 — it is not a printed zero"
    );
    assert_eq!(ii.line11_steps, None, "…and line 11");
    assert_eq!(ii.line12, None, "…and line 12");
}

/// Over the threshold, the phase-out actually runs and the lines are populated.
#[test]
fn line10_over_the_threshold_runs_the_phase_out() {
    let p = p();
    let threshold = p.tips_phase_out.threshold_for(FilingStatus::Single);
    let f = facts(threshold + dec!(3000), FilingStatus::Single);
    let ii = Schedule1aPartII::compute(&p, &f, Some(dec!(25000)), None);

    assert_eq!(ii.line10, Some(dec!(3000)), "L10 = L8 − L9, positive");
    assert!(
        ii.line12.is_some(),
        "the reduction must be printed when the phase-out runs"
    );
    let reduced = ii.line13.expect("L13 present");
    assert!(
        reduced < ii.line7.unwrap(),
        "over the threshold the deduction must be REDUCED, got {reduced} vs cap {:?}",
        ii.line7
    );
}

/// ★★★ **Line 33's jump writes $6,000, not $0.** This is the single highest-value assertion in the
/// file: the wrong transcription silently deletes the entire senior deduction for most filers.
#[test]
fn line33_under_the_threshold_writes_six_thousand_not_zero() {
    let p = p();
    let mut f = facts(dec!(40000), FilingStatus::Single);
    f.taxpayer_qualifies_as_senior = true;
    let v = Schedule1aPartV::compute(&p, &f);

    assert_eq!(
        v.line35,
        Some(p.senior_per_person),
        "\"If zero or less, enter $6,000 on line 35\" — a NONZERO constant. Transcribing this as \
         -0- loses the whole senior deduction for every filer under the threshold."
    );
    assert_eq!(v.line33, None, "the jump skips line 33");
    assert_eq!(v.line34, None, "…and line 34");
    assert_eq!(
        v.line36a,
        Some(p.senior_per_person),
        "the taxpayer enters line 35"
    );
    assert_eq!(v.line36b, None, "no spouse senior on a Single return");
    assert_eq!(v.line37, Some(p.senior_per_person));
}

/// ★★ Line 35 subtracts the **printed** line 34, not the unrounded product. The two differ by $1
/// whenever 0.06 × L33 lands on a half-dollar, and the round-the-difference form UNDERSTATES tax.
#[test]
fn line35_subtracts_the_printed_line34_not_the_unrounded_product() {
    let p = p();
    let threshold = p.senior_threshold_for(FilingStatus::Single);
    // excess ≡ 25 (mod 50) puts 0.06 × excess exactly on a half-dollar.
    let excess = dec!(25025);
    let mut f = facts(threshold + excess, FilingStatus::Single);
    f.taxpayer_qualifies_as_senior = true;
    let v = Schedule1aPartV::compute(&p, &f);

    let l34 = v.line34.expect("the phase-out ran, so line 34 prints");
    let printed_form = (p.senior_per_person - l34).max(Usd::ZERO);
    assert_eq!(
        v.line35,
        Some(printed_form),
        "line 35 must subtract the PRINTED line 34"
    );
    assert_eq!(
        l34,
        btctax_core::conventions::round_dollar(p.senior_rate * excess),
        "line 34 is its own printed dollar line and rounds"
    );
}

/// ★★ Parts II, III and V print *"If married, you must file jointly to claim this deduction."*
/// Part IV prints **no such caution** and is therefore ALLOWED for MFS — adjudicated against the
/// FORM, over an oracle that bars all four. An oracle is a witness; the form is the authority.
#[test]
fn mfs_is_barred_from_parts_ii_iii_and_v_but_allowed_in_part_iv() {
    let p = p();
    let mut f = facts(dec!(50000), FilingStatus::Mfs);
    f.taxpayer_qualifies_as_senior = true;

    assert_eq!(
        Schedule1aPartII::compute(&p, &f, Some(dec!(4000)), None).line13,
        None,
        "Part II prints \"If married, you must file jointly\" — MFS claims nothing"
    );
    assert_eq!(
        Schedule1aPartIII::compute(&p, &f, Some(dec!(4000))).line21,
        None,
        "Part III prints the same caution"
    );
    assert_eq!(
        Schedule1aPartV::compute(&p, &f).line37,
        None,
        "Part V prints the same caution"
    );
    assert_eq!(
        Schedule1aPartIV::compute(&p, &f, Some(dec!(4000))).line30,
        Some(dec!(4000)),
        "Part IV prints NO such caution, so barring MFS here would deny a deduction §163(h)(4) \
         allows. The form is the authority, not an oracle that bars all four."
    );
}

/// Line 38 is a `Combine`: blank when every operand is blank (FR-39), never a fabricated zero.
#[test]
fn line38_is_blank_when_every_part_is_blank_and_sums_when_any_is_live() {
    let p = p();
    let f = facts(dec!(50000), FilingStatus::Single);

    let empty = Schedule1aPartVI::compute(
        &Schedule1aPartII::compute(&p, &f, None, None),
        &Schedule1aPartIII::compute(&p, &f, None),
        &Schedule1aPartIV::compute(&p, &f, None),
        &Schedule1aPartV::compute(&p, &f),
    );
    assert_eq!(
        empty.line38, None,
        "a filer claiming none of the four parts must leave line 38 BLANK — a printed 0 swears they \
         worked the schedule and it came to nothing"
    );

    let live = Schedule1aPartVI::compute(
        &Schedule1aPartII::compute(&p, &f, Some(dec!(1000)), None),
        &Schedule1aPartIII::compute(&p, &f, None),
        &Schedule1aPartIV::compute(&p, &f, Some(dec!(500))),
        &Schedule1aPartV::compute(&p, &f),
    );
    assert_eq!(
        live.line38,
        Some(dec!(1500)),
        "L38 = add lines 13, 21, 30 and 37"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// T5 — the wiring, and the invariant a COMMENT used to carry
// ══════════════════════════════════════════════════════════════════════════════════════════════════

use btctax_core::tax::return_inputs::{Schedule1aInputs, Schedule1aVehicle};
use btctax_core::tax::schedule_1a::Schedule1A;

fn one_qualifying_vehicle(interest: Usd) -> Schedule1aInputs {
    Schedule1aInputs {
        vehicles: vec![Schedule1aVehicle {
            description: "truck".into(),
            interest_paid: interest,
            loan_originated_after_2024: true,
            loan_originated_by_you: true,
            proceeds_used_to_purchase: true,
            personal_use: true,
            secured_by_first_lien: true,
            original_use_starts_with_you: true,
            is_applicable_vehicle_class_under_14000_lbs: true,
            final_assembly_in_us: true,
            excludes_negative_equity: true,
        }],
        ..Default::default()
    }
}

/// ★★★ **Census F-6 — the invariant a COMMENT used to carry, made real.**
///
/// `return_1040.rs` held `let schedule_1a_additional = Usd::ZERO;` under a comment reading *"the 2024
/// form has no such line, so zero is the RIGHT value there, not a stub."* True for TY2024 and FALSE
/// the moment TY2025 landed — and **a comment cannot red**. It is §G-11's shape in miniature: a
/// correct blank and a laundered one sharing one code path, told apart only by prose.
///
/// Now the YEAR decides, and this test is what holds it: TY2024 yields zero because there is no
/// schedule to compute, not because a literal says so.
#[test]
fn a_year_with_no_schedule_1a_yields_a_zero_line_13b_because_no_schedule_exists() {
    let inputs = one_qualifying_vehicle(dec!(3000));

    for year_without in [2023, 2024, 2029, 2030] {
        let s = Schedule1A::compute(
            year_without,
            dec!(80000),
            FilingStatus::Single,
            &inputs,
            false,
            false,
        );
        assert!(
            s.is_none(),
            "TY{year_without} has no Schedule 1-A — the four provisions did not exist or have \
             sunset, so there must be no schedule to compute"
        );
        assert_eq!(
            Schedule1A::line_13b(s.as_ref()),
            Usd::ZERO,
            "…and 1040 line 13b is therefore a REAL zero, not a stub"
        );
    }

    // TY2025 has one, and the same inputs now produce a deduction. Without this half the assertion
    // above would pass for a build in which Schedule 1-A never computes at all.
    let live = Schedule1A::compute(
        2025,
        dec!(80000),
        FilingStatus::Single,
        &inputs,
        false,
        false,
    )
    .expect("TY2025 HAS a Schedule 1-A");
    assert_eq!(
        Schedule1A::line_13b(Some(&live)),
        dec!(3000),
        "TY2025 must actually carry the deduction to line 13b"
    );
}

/// ★★ Part IV sums only vehicles that pass EVERY §163(h)(4) condition. A vehicle failing one bar
/// contributes NOTHING — which is the entire reason the conditions are collected. This is the test
/// that would have caught the original defect at the schedule level rather than the struct level.
#[test]
fn part_iv_sums_only_vehicles_that_pass_every_condition() {
    let mut inputs = one_qualifying_vehicle(dec!(2000));
    // A second vehicle, identical except that it is leased.
    let mut leased = inputs.vehicles[0].clone();
    leased.description = "leased sedan".into();
    leased.interest_paid = dec!(5000);
    leased.proceeds_used_to_purchase = false;
    inputs.vehicles.push(leased);

    let s = Schedule1A::compute(
        2025,
        dec!(80000),
        FilingStatus::Single,
        &inputs,
        false,
        false,
    )
    .unwrap();
    assert_eq!(
        s.part4.line30,
        Some(dec!(2000)),
        "only the qualifying truck's interest may be deducted — \"lease payments do not qualify\", \
         and a household with one of each must be able to say so"
    );
}

/// A filer who claims nothing leaves the whole schedule blank — line 38 included (FR-39).
#[test]
fn a_filer_claiming_nothing_produces_a_blank_line_38() {
    let s = Schedule1A::compute(
        2025,
        dec!(80000),
        FilingStatus::Single,
        &Schedule1aInputs::default(),
        false,
        false,
    )
    .unwrap();
    assert_eq!(
        s.part6.line38, None,
        "a printed 0 would swear the filer worked all four parts and they came to nothing"
    );
    assert_eq!(
        Schedule1A::line_13b(Some(&s)),
        Usd::ZERO,
        "and 1040 13b adds a blank as nothing"
    );
}
