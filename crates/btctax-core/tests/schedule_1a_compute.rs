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

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// T6 — the worked examples, all five filing statuses, and the aggregate slope
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **The recon's two-senior worked example, and the 12¢-per-$1 aggregate slope.**
///
/// MFJ at MAGI $200,000: the senior threshold is $150,000, so line 33 is $50,000, line 34 is 6% of
/// that = $3,000, and line 35 is $6,000 − $3,000 = **$3,000**. Line 35 is computed ONCE and lines
/// 36a and 36b EACH enter it, so a two-senior couple's line 37 is **$6,000** — $3,000 each.
///
/// ★ That is why the couple loses **12¢ per $1** of MAGI in the band while each of them loses 6¢:
/// the phase-out is per-person on a per-return excess. A design that computed line 35 twice, or that
/// halved it, would be wrong in opposite directions and both would look plausible.
#[test]
fn two_seniors_mfj_at_200k_each_enter_3000_for_a_line37_of_6000() {
    let p = p();
    let f = Schedule1aFacts {
        magi: dec!(200000),
        status: FilingStatus::Mfj,
        taxpayer_qualifies_as_senior: true,
        spouse_qualifies_as_senior: true,
    };
    let v = Schedule1aPartV::compute(&p, &f);

    assert_eq!(v.line33, Some(dec!(50000)), "L33 = 200,000 − 150,000");
    assert_eq!(v.line34, Some(dec!(3000)), "L34 = 6% × 50,000");
    assert_eq!(
        v.line35,
        Some(dec!(3000)),
        "L35 = 6,000 − 3,000, computed ONCE"
    );
    assert_eq!(v.line36a, Some(dec!(3000)), "the taxpayer enters line 35");
    assert_eq!(
        v.line36b,
        Some(dec!(3000)),
        "and the spouse enters the SAME line 35"
    );
    assert_eq!(v.line37, Some(dec!(6000)), "L37 = 36a + 36b");

    // The slope, stated as a property rather than a second literal: one more dollar of MAGI costs
    // the couple 12¢ and each of them 6¢.
    let f2 = Schedule1aFacts {
        magi: dec!(201000),
        ..f
    };
    let v2 = Schedule1aPartV::compute(&p, &f2);
    assert_eq!(
        v.line37.unwrap() - v2.line37.unwrap(),
        dec!(120),
        "$1,000 more MAGI costs a two-senior couple $120 — 6% of 1,000 = $60 PER PERSON, twice. \
         That is the 12¢-per-$1 aggregate slope, and it is why line 35 is computed once and \
         entered twice rather than being halved."
    );
}

/// ★★ **All five filing statuses.** S-3's caps and S-5's MFS bar are status-dependent, and three of
/// the five statuses are neither MFJ nor MFS — a test that only checks the MFJ/MFS poles would miss
/// that Single, HoH and QSS all take the BASE threshold, which is what the Part IV instructions say
/// in exactly those words: *"Married filing jointly—$200,000. All other filing statuses—$100,000."*
#[test]
fn every_filing_status_takes_the_right_threshold_and_cap() {
    let p = p();
    for status in [
        FilingStatus::Single,
        FilingStatus::HoH,
        FilingStatus::Qss,
        FilingStatus::Mfj,
        FilingStatus::Mfs,
    ] {
        let want_qpvli = if matches!(status, FilingStatus::Mfj) {
            dec!(200000)
        } else {
            dec!(100000)
        };
        assert_eq!(
            p.qpvli_phase_out.threshold_for(status),
            want_qpvli,
            "Part IV threshold for {status:?}"
        );

        // The tips cap does NOT vary by status — the instructions say so twice, including
        // "regardless of your filing status" — while the overtime cap DOES double for MFJ.
        let f = facts(dec!(50000), status);
        let ii = Schedule1aPartII::compute(&p, &f, Some(dec!(99999)), None);
        let iii = Schedule1aPartIII::compute(&p, &f, Some(dec!(99999)));
        if matches!(status, FilingStatus::Mfs) {
            assert_eq!(ii.line7, None, "MFS is barred from Part II entirely");
            assert_eq!(iii.line15, None, "…and Part III");
        } else {
            assert_eq!(
                ii.line7,
                Some(p.tips_cap),
                "the tips cap is per RETURN for {status:?}"
            );
            let want_ot = if matches!(status, FilingStatus::Mfj) {
                p.overtime_cap_mfj
            } else {
                p.overtime_cap
            };
            assert_eq!(iii.line15, Some(want_ot), "overtime cap for {status:?}");
        }
    }
}

/// ★ **Filing is gated on `L38 > 0`.** A schedule computed but empty must not be attached: an
/// attached page of blanks is a claim that the filer worked it.
#[test]
fn the_schedule_is_filed_only_when_line_38_is_positive() {
    let none_claimed = Schedule1A::compute(
        2025,
        dec!(80000),
        FilingStatus::Single,
        &Schedule1aInputs::default(),
        false,
        false,
    )
    .unwrap();
    assert_eq!(
        none_claimed.part6.line38, None,
        "nothing claimed ⇒ nothing to file"
    );

    let claimed = Schedule1A::compute(
        2025,
        dec!(80000),
        FilingStatus::Single,
        &one_qualifying_vehicle(dec!(1)),
        false,
        false,
    )
    .unwrap();
    assert_eq!(
        claimed.part6.line38,
        Some(dec!(1)),
        "even a $1 qualifying deduction is a filed schedule — the gate is `> 0`, not a threshold"
    );
}

/// ★★ **Line 37 — the SENIOR SUBTOTAL — is what reaches Form 6251 line 1a, NOT line 38.**
///
/// The AMT add-back is the §151(d)(5) senior deduction alone; the other three parts are not added
/// back. Wiring line 38 there would add tips, overtime and car-loan interest to alternative minimum
/// taxable income, overstating AMT for every filer who claims any of them.
#[test]
fn form_6251_takes_the_senior_subtotal_line_37_not_the_total_line_38() {
    let p = p();
    let f = Schedule1aFacts {
        magi: dec!(80000),
        status: FilingStatus::Single,
        taxpayer_qualifies_as_senior: true,
        spouse_qualifies_as_senior: false,
    };
    let v = Schedule1aPartV::compute(&p, &f);
    let s = Schedule1A::compute(
        2025,
        dec!(80000),
        FilingStatus::Single,
        &one_qualifying_vehicle(dec!(4000)),
        true,
        false,
    )
    .unwrap();

    assert_eq!(s.part5.line37, v.line37, "Part V is unaffected by Part IV");
    assert_ne!(
        s.part6.line38, s.part5.line37,
        "the fixture must make the two DIFFER, or this test cannot discriminate"
    );
    assert_eq!(
        s.part6.line38,
        Some(v.line37.unwrap() + dec!(4000)),
        "line 38 adds the car-loan deduction; line 37 does not — so a Form 6251 add-back reading \
         line 38 would inflate AMTI by the other three parts"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// THE STEP-ARITHMETIC KAT — the same boundary vectors `scripts/oracle/verify_schedule_1a.py` drives
// ══════════════════════════════════════════════════════════════════════════════════════════════════
//
// ★★★ WHY A SECOND FILE-LEVEL SECTION, WHEN THE CONSTANTS ARE ALREADY PINNED. Every constant on this
// form agrees with Tax-Calculator's, and has always agreed. The arithmetic that APPLIES them does not
// follow from them, because the rounding direction is not a constant: lines 11 and 19 print *"decrease
// the result to the next lower whole number"* and line 28 prints *"increase the result to the next
// higher whole number"*. Flip one and the deduction moves by a FULL STEP — $100 on Parts II/III, $200
// on Part IV — for every filer in the phase-out band, with every constant still matching and the
// constants census still printing 0 divergences. `StepRounding`'s own doc comment calls it "the field
// that must never be shared"; this is the test that makes that true rather than hoped.
//
// ★★ THE NUMBERS BELOW ARE INDEPENDENT OF BTCTAX. They are the form's own arithmetic, computed in
// Python from the sentences quoted above (`verify_schedule_1a.py::_form_expected`) and confirmed
// against BOTH reference engines at the same MAGIs — so this is a KAT, not a snapshot of our own
// output. Where an engine cannot witness a vector the census says so per vector and per filing
// status; the two standing gaps are Part IV for QSS (taxcalc hands a qualifying surviving spouse the
// MFJ threshold, and OpenTaxSolver 2025 both floors line 28 and multiplies it by $300) and Part V's
// half-dollar cases (neither engine rounds the printed line 34). Those rest on the FORM.
//
// ★ The OFFSETS are derived from the parameters, never typed: two of them are the form's own worked
// examples (0.05 of a step and 1.5 steps), which are the two that tell the directions apart at all.

use btctax_core::tax::tables::{StairStepPhaseOut, StepRounding};

/// The other direction. `_`-free on purpose: a third rounding mode would be a compile error here
/// rather than a silently unflipped mutant, which would make the discriminating-power assertion below
/// pass while testing nothing.
fn opposite(r: StepRounding) -> StepRounding {
    match r {
        StepRounding::Floor => StepRounding::Ceil,
        StepRounding::Ceil => StepRounding::Floor,
    }
}

/// The six excesses that straddle one stepped phase-out — DERIVED from the phase-out itself.
///
/// `at the threshold` exercises the form's JUMP; `+$1` and `+0.05 step` are where floor and ceil first
/// disagree (the form's own example: *"decrease 0.05 to 0"* against *"increase 0.05 to 1"*); `+1.5
/// steps` is its other example; and the last two bracket exhaustion, which is itself per-direction —
/// a ceiling part exhausts one dollar past the last full step, so Part IV's last live vector is an
/// excess of $49,000 carrying $200, not $49,999.
fn stepped_offsets(po: &StairStepPhaseOut, cap: Usd) -> [Usd; 6] {
    let exhaust = po.exhaustion_excess(cap);
    [
        Usd::ZERO,
        dec!(1),
        po.step / dec!(20),
        po.step + po.step / dec!(2),
        exhaust - dec!(1),
        exhaust,
    ]
}

/// Walk one part's six boundary vectors for one filing status.
///
/// `want` is `None` for a part this status is barred from, and the assertion is then that the line is
/// **blank** — not zero. A blank and a `0` are identical on the printed page and are not the same
/// thing: `0` on a line the filer never claimed is sworn testimony that the amount IS zero.
fn assert_boundaries(
    label: &str,
    po: &StairStepPhaseOut,
    cap: Usd,
    status: FilingStatus,
    want: Option<[Usd; 6]>,
    deduction: impl Fn(Usd) -> Option<Usd>,
) {
    let offsets = stepped_offsets(po, cap);
    // ★★ THE KAT'S OWN DISCRIMINATING POWER, asserted rather than assumed. A vector set that happened
    //    to sit on whole steps would agree under either direction, and this whole section would be an
    //    instrument that cannot fail — the exact shape (B1) this repo keeps catching. So: at least one
    //    offset must give a different reduction under the opposite rounding.
    let flipped = StairStepPhaseOut {
        rounding: opposite(po.rounding),
        ..po.clone()
    };
    assert!(
        offsets
            .iter()
            .any(|&e| e > Usd::ZERO && flipped.reduction(e) != po.reduction(e)),
        "{label}: not one of these offsets distinguishes {:?} from {:?}, so nothing here holds the \
         rounding direction",
        po.rounding,
        flipped.rounding,
    );
    let threshold = po.threshold_for(status);
    for (i, off) in offsets.iter().enumerate() {
        let magi = threshold + off;
        let got = deduction(magi);
        match want {
            None => assert!(
                got.is_none(),
                "{label}/{status:?} @ MAGI {magi} — a barred part stays BLANK, not zero; got {got:?}"
            ),
            Some(w) => assert_eq!(
                got,
                Some(w[i]),
                "{label}/{status:?} @ MAGI {magi} (excess {off}, threshold {threshold})"
            ),
        }
    }
}

/// ★★★ **Parts II, III and IV: every printed boundary, every filing status, against the census.**
#[test]
fn the_step_arithmetic_matches_the_oracle_census() {
    let p = p();
    let (tips_claimed, overtime_claimed, qpvli_claimed) = (dec!(40000), dec!(30000), dec!(12000));
    for status in FilingStatus::ALL {
        // ── Part II, tips (§224). Cap $25,000 for EVERY status — line 7 prints no MFJ figure — so
        //    the six values are the same for every status that may claim it, and the MAGIs differ.
        let want = match status {
            // "If married, you must file jointly to claim this deduction." (Part II Caution)
            FilingStatus::Mfs => None,
            FilingStatus::Single | FilingStatus::Mfj | FilingStatus::HoH | FilingStatus::Qss => {
                Some([
                    dec!(25000),
                    dec!(25000), // floor(0.001) = 0 steps — "decrease 0.05 to 0"
                    dec!(25000), // floor(0.05)  = 0 steps
                    dec!(24900), // floor(1.5)   = 1 step  × $100
                    dec!(100),   // 249 steps of the 250 the cap allows
                    dec!(0),     // "If zero or less, enter -0-"
                ])
            }
        };
        assert_boundaries(
            "Part II line 13",
            &p.tips_phase_out,
            p.tips_cap,
            status,
            want,
            |magi| {
                Schedule1aPartII::compute(&p, &facts(magi, status), Some(tips_claimed), None).line13
            },
        );

        // ── Part III, overtime (§225). Line 15's cap DOES double for MFJ, so MFJ's six values are
        //    the tips shape and every other status's are half of it.
        let want = match status {
            FilingStatus::Mfs => None,
            FilingStatus::Mfj => Some([
                dec!(25000),
                dec!(25000),
                dec!(25000),
                dec!(24900),
                dec!(100),
                dec!(0),
            ]),
            FilingStatus::Single | FilingStatus::HoH | FilingStatus::Qss => Some([
                dec!(12500),
                dec!(12500),
                dec!(12500),
                dec!(12400),
                dec!(100), // 124 of the 125 steps the $12,500 cap allows
                dec!(0),
            ]),
        };
        assert_boundaries(
            "Part III line 21",
            &p.overtime_phase_out,
            p.overtime_cap_for(status),
            status,
            want,
            |magi| {
                Schedule1aPartIII::compute(&p, &facts(magi, status), Some(overtime_claimed)).line21
            },
        );

        // ── Part IV, car loan interest (§163(h)(4)). **The one CEILING on the form**, and the one
        //    part with no "must file jointly" caution — so MFS claims it, and every status shares the
        //    same six values because the $10,000 cap prints no status variant.
        let want = match status {
            FilingStatus::Single
            | FilingStatus::Mfj
            | FilingStatus::Mfs
            | FilingStatus::HoH
            | FilingStatus::Qss => Some([
                dec!(10000),
                dec!(9800), // ceil(0.001) = 1 step — "increase 0.05 to 1"
                dec!(9800), // ceil(0.05)  = 1 step  × $200
                dec!(9600), // ceil(1.5)   = 2 steps
                dec!(200),  // excess $49,000 — 49 whole steps, NOT exhausted
                dec!(0),    // excess $49,001 — the 50th part-step finishes it
            ]),
        };
        assert_boundaries(
            "Part IV line 30",
            &p.qpvli_phase_out,
            p.qpvli_cap,
            status,
            want,
            |magi| Schedule1aPartIV::compute(&p, &facts(magi, status), Some(qpvli_claimed)).line30,
        );
    }
}

/// ★★★ **Part V is SMOOTH, and these vectors red if it ever acquires a stair.**
///
/// §151(d)(5)(C) / line 34 is a flat 6% with no rounding instruction of its own, so there is no step
/// anywhere in this part — and `StairStepPhaseOut`'s own comment names the trap: *"giving it a fake
/// step is how a smooth phase-out acquires a stair."* A planted stair with the same average slope
/// (one $1,000 step worth $60) agrees with the true line at every multiple of $1,000, so the vectors
/// that matter are the ones INSIDE a step — which is why `+$500` and `+$1,500` are here, derived from
/// the $1,000 the stepped parts use, and why `+$25` is (0.06 × 25 = $1.50, the only place line 34's
/// whole-dollar rounding is visible at all).
#[test]
fn part_v_is_smooth_and_a_planted_stair_would_show_here() {
    let p = p();
    let step = p.tips_phase_out.step; // the $1,000 a fake stair would plausibly use
    let exhaust = p.senior_per_person / p.senior_rate; // $6,000 / 0.06 = $100,000 of excess
    let offsets = [
        Usd::ZERO,
        dec!(1),
        dec!(25),
        dec!(50),
        step / dec!(2),
        step + step / dec!(2),
        exhaust - dec!(1),
        exhaust,
    ];
    // Line 35, per qualifying individual. Independently computed from "Multiply line 33 by 6% (0.06)"
    // and "Subtract line 34 from $6,000", with line 34 rounded to whole dollars as a printed dollar
    // line — and confirmed against both engines except at the half-dollar, where NEITHER rounds it.
    let want_line35 = [
        dec!(6000), // L33 "If zero or less, enter $6,000 on line 35" — a nonzero constant
        dec!(6000), // 0.06 × 1 = $0.06 ⇒ line 34 prints $0
        dec!(5998), // 0.06 × 25 = $1.50 ⇒ line 34 prints $2 (IRS half-up), not $1
        dec!(5997), // 0.06 × 50 = $3 exactly
        dec!(5970), // 0.06 × 500 = $30 — a $60-per-$1,000 stair would read $0 here
        dec!(5910), // 0.06 × 1,500 = $90 — that stair would read $60
        dec!(0),    // 0.06 × 99,999 = $5,999.94 ⇒ line 34 prints $6,000
        dec!(0),
    ];
    for status in FilingStatus::ALL {
        // L36b — "If you are married filing jointly, your spouse … enter the amount from line 35",
        // so the second senior is reachable for MFJ only, and the loss is 12¢ per $1 of MAGI for the
        // couple while it is 6¢ for each of them.
        let counts: &[u32] = match status {
            FilingStatus::Mfj => &[1, 2],
            FilingStatus::Single | FilingStatus::Mfs | FilingStatus::HoH | FilingStatus::Qss => {
                &[1]
            }
        };
        for &seniors in counts {
            for (i, off) in offsets.iter().enumerate() {
                let magi = p.senior_threshold_for(status) + off;
                let f = Schedule1aFacts {
                    magi,
                    status,
                    taxpayer_qualifies_as_senior: true,
                    spouse_qualifies_as_senior: seniors == 2,
                };
                let got = Schedule1aPartV::compute(&p, &f);
                // "If married, you must file jointly to claim this deduction." (Part V Caution)
                if matches!(status, FilingStatus::Mfs) {
                    assert!(
                        got.line35.is_none() && got.line37.is_none(),
                        "Part V/{status:?} @ MAGI {magi} — barred, so BLANK rather than zero"
                    );
                    continue;
                }
                assert_eq!(
                    got.line35,
                    Some(want_line35[i]),
                    "Part V line 35/{status:?} @ MAGI {magi} (excess {off})"
                );
                assert_eq!(
                    got.line37,
                    Some(want_line35[i] * Usd::from(seniors)),
                    "Part V line 37/{status:?} @ MAGI {magi} with {seniors} senior(s)"
                );
            }
        }
    }
}
