//! The 2024 **"Worksheet To See if You Should Fill in Form 6251 — Schedule 2, Line 2"** (SPEC §4.11).
//!
//! ★ **As of v0.14.0 this worksheet is a CROSS-CHECK, not a gate.** btctax computes the real
//! [`Form6251`] for every return ([`crate::tax::form6251`]) and refuses on the form's own test —
//! i6251, *Who Must File*, condition 1: **line 7 greater than line 10**. The worksheet appears on no
//! production path. It survives here because its clearing branch is a valid *sufficient* condition for
//! "no AMT" — the soundness argument below — and that implication is swept by
//! `return_1040`'s `a_cleared_screen_never_hides_a_must_attach_return`.
//!
//! Before v0.14.0 this was a refuse-trigger and v1 did not compute Form 6251 at all. Do not restore
//! that shape: gating the safety check on a proxy meant that making the proxy *more correct* made it
//! skip the check more often.
//!
//! [`Form6251`]: crate::tax::form6251::Form6251
//!
//! **Worksheet line 2 is the two §56(b)(1) add-backs, and nothing else.** It reads: *"Enter the amount from
//! Schedule A (Form 1040), line 7, or your standard deduction from Form 1040 or 1040-SR, line 12."*
//! Schedule A **line 7** is `5e + 6` — the SALT total after the §164(b)(6) cap, plus other taxes — i.e.
//! **taxes paid**, disallowed for AMT by §56(b)(1)(A)(ii). It is NOT the itemized total (that is Schedule A
//! **line 17**, which is what flows to 1040 L12). The standard-deduction branch adds back the standard
//! deduction because §56(b)(1)(D) disallows it outright. Every other Schedule-A line — mortgage interest,
//! charitable, medical — is AMT-**allowed** and is never added back.
//!
//! **★ 2026-07-27 FIX (`fix/amt-screen-line2`).** This module previously reduced the worksheet to the
//! closed form `line 3 = AGI − QBI` in BOTH branches, on the stated premise that "Schedule A line 7 (= L12)".
//! That premise conflated Schedule A line 7 with line 17. The closed form is correct for a STANDARD-deduction
//! filer (`L15 + std = AGI − QBI`) but wrong for an ITEMIZER, where it silently added back mortgage interest,
//! charitable gifts and medical expenses on top of taxes — inflating worksheet line 3 by the filer's entire
//! non-tax itemized deduction. It never understated tax (the error is fail-closed: too MANY refusals), but it
//! manufactured false refusals for ordinary itemizers. Reachable example, now pinned as a KAT below: MFJ, AGI
//! $300,000, $80,000 itemized of which $10,000 SALT → the true worksheet clears (26% × $96,700 = $25,142 ≤
//! $38,885 regular tax) while the closed form refused (26% × $166,700 = $43,342 > $38,885). The verbatim
//! lines 1-3 are now evaluated instead; `line1` is 1040 L15, so its floor-at-$0 is inherited for free and the
//! old "L15 floored to $0" caveat is gone with the closed form that needed it.
//!
//! **Soundness of a CLEAR (why Schedule 2 line 2 = $0 is safe).** Within v1's accepted inputs, worksheet
//! line 3 is not an over-estimate of AMTI — it is AMTI **exactly**: every Form 6251 adjustment other than
//! the taxes/standard-deduction add-back is either refused upstream (PAB interest; a divergent AMT
//! capital-loss carryover, line 2k; a divergent Schedule C depreciation amount, line 2l) or is an input v1
//! never captures (ISO, §1202, §4952, NOL, Form 8801, K-1 adjustments) — see "the worksheet's
//! Exception" below — and the §199A deduction is allowed for AMT by §199A(f)(2), so subtracting it is right.
//! The line-12 test is then a valid sufficient condition: in that branch `line11 ≤` the 26/28 breakpoint, so
//! `26% × line11` IS the tentative minimum tax before §55(b)(3), and preserving the §1(h) rate on any net
//! capital gain can only LOWER it. So `26% × line11 ≤ regular tax` ⇒ `TMT ≤ regular tax` ⇒ no AMT.
//!
//! **The worksheet's "Exception"** (2024 i1040gi p.97 — items that force Form 6251 directly: §4952
//! investment interest, accelerated depreciation, PAB tax-exempt interest, ISO stock, §1202 exclusion,
//! NOL, the **foreign tax credit**, the Form 8801 prior-year-minimum-tax credit, …). Coverage:
//! - PAB interest is already refused (INT box 9 / DIV box 13, `screen_inputs`); ISO/§1202/§4952/NOL/8801
//!   are all out-of-scope inputs v1 never captures.
//! - **Accelerated depreciation is NOT absent — it is uncapturable in aggregate**, and so is refused by
//!   DECLARATION rather than by scope. `ScheduleCInputs::expenses` is a flat filer-supplied total, and
//!   Schedule C Part II line 13 is "Depreciation and section 179 expense deduction" — so a §56(a)(1)
//!   adjustment rides inside an accepted input, invisible. `QuestionId::AmtDepreciationSameAsRegular`
//!   makes the filer affirm it away, exactly as line 2k does for the carryover twin. (An earlier draft of
//!   this module listed depreciation among the "inputs v1 never captures". That was factually wrong, and
//!   it was the sole cover for the one open understatement channel — whole-branch review C-2.)
//! - The **§904(j) foreign tax credit** (Sch 3 L1) IS a live v1 input, but it does not require a separate
//!   refuse: the Exception exists because in general the AMT foreign tax credit (§59(a)) differs from the
//!   regular FTC, but for the **≤ $300/$600 passive-1099 §904(j) credit** this crate screens for, the AMT
//!   FTC equals the regular FTC, so it **cancels from both sides** of the AMT comparison —
//!   `(TMT − AMTFTC) > (regular_tax − FTC)` ⇔ `TMT > regular_tax` when `AMTFTC = FTC` — which is exactly
//!   the worksheet's line-12-vs-line-13 test (line 13 = L16, no FTC subtraction). The FTC therefore
//!   changes neither line and cannot flip the screen. (§59 caveat: this equivalence holds only within the
//!   ≤ $300/$600 passive scope; anything above refuses upstream as `ForeignTaxOverCeiling`.)
use crate::conventions::Usd;
use crate::tax::tables::AmtParams;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// Worksheet **line 2** — *"Enter the amount from Schedule A (Form 1040), line 7, or your standard deduction
/// from Form 1040 or 1040-SR, line 12."* The two §56(b)(1) add-backs and nothing else.
///
/// - `schedule_a_line7` is Schedule A `5e + 6` — the CAPPED SALT total plus other taxes (v1 does not model
///   line 6, so callers pass `salt_5e`). **Never pass the itemized total** (Schedule A line 17): mortgage
///   interest, charitable gifts and medical expenses are AMT-allowed and must not be added back. Passing
///   line 17 here is the exact defect this helper exists to prevent — see the module header.
/// - When the standard deduction was taken, §56(b)(1)(D) disallows it entirely, so the whole of it is the
///   add-back.
pub fn amt_worksheet_line2(
    deduction_is_itemized: bool,
    standard_deduction: Usd,
    schedule_a_line7: Usd,
) -> Usd {
    if deduction_is_itemized {
        schedule_a_line7
    } else {
        standard_deduction
    }
}

/// Run the 2024 AMT worksheet (SPEC §4.11). Returns `true` when the worksheet says **fill in Form 6251**
/// (→ refuse in v1), `false` when it clears the taxpayer (Schedule 2 line 2 = 0). All comparisons are the
/// worksheet's strict "more than" (`>`).
///
/// - `taxable_income_l15` = worksheet line 1 = 1040 L15 (already floored at $0 by the 1040 itself, which is
///   exactly what the worksheet's "If zero or less, enter -0-" asks for).
/// - `deduction_add_back_l2` = worksheet line 2 — build it with [`amt_worksheet_line2`], NEVER by passing
///   the itemized total. See this module's header: it is Schedule A line **7** (taxes), not line **17**.
/// - `state_refund_and_8z` = Schedule 1 lines 1 + 8z (worksheet line 4, subtracted — a state refund is not
///   AMT income). v1 has no Sch 1 L8z input, so this is just the taxable state refund.
/// - `regular_tax_l16` = 1040 L16; `sch2_l1z` = Schedule 2 line 1z (worksheet line 13 = L16 + L1z; L1z is
///   the excess-APTC total, 0 in v1 — no input).
pub fn amt_should_file_6251(
    status: FilingStatus,
    taxable_income_l15: Usd,
    deduction_add_back_l2: Usd,
    state_refund_and_8z: Usd,
    regular_tax_l16: Usd,
    sch2_l1z: Usd,
    amt: &AmtParams,
) -> bool {
    // Lines 1-3 — taxable income plus the §56(b)(1) add-back; then line 5 = less any state refund (line 4).
    let line3 = taxable_income_l15 + deduction_add_back_l2;
    let line5 = line3 - state_refund_and_8z;

    // Line 6/7 — the §55(d)(1) exemption. Line 5 at/below it ⇒ STOP, no AMT.
    let exemption = amt.exemption(status);
    if line5 <= exemption {
        return false;
    }
    let line7 = line5 - exemption;

    // Lines 8–11 — §55(d)(3) exemption phase-out (25% of the excess over the phase-out start, capped at the
    // exemption) added back to line 7.
    let phaseout_start = amt.phaseout_start(status);
    let line11 = if line5 > phaseout_start {
        let line9 = line5 - phaseout_start;
        let line10 = (dec!(0.25) * line9).min(exemption);
        line7 + line10
    } else {
        line7
    };

    // Line 12 — over the §55(b)(1) 26%/28% breakpoint ⇒ STOP, fill Form 6251.
    if line11 > amt.breakpoint_28pct(status) {
        return true;
    }

    // "Next" — tentative minimum tax (26% × line 11) more than the regular tax (L16 + Sch 2 L1z) ⇒ fill 6251.
    let line12 = dec!(0.26) * line11;
    let line13 = regular_tax_l16 + sch2_l1z;
    line12 > line13
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::AmtParams;

    fn amt() -> AmtParams {
        AmtParams {
            exemption_single_hoh: dec!(85700),
            exemption_mfj_qss: dec!(133300),
            exemption_mfs: dec!(66650),
            phaseout_start_single_hoh_mfs: dec!(609350),
            phaseout_start_mfj_qss: dec!(1218700),
            breakpoint_28pct: dec!(232600),
            breakpoint_28pct_mfs: dec!(116300),
            mfs_kicker_start: dec!(875950),
            mfs_kicker_max: dec!(66650),
            phaseout_rate: dec!(0.25),
            rate_26: dec!(0.26),
            rate_28: dec!(0.28),
            rate_28_subtrahend: dec!(4652),
            rate_28_subtrahend_mfs: dec!(2326),
        }
    }

    // NOTE on the cases below that pass a bare figure as arg 2 with `Usd::ZERO` as arg 3: since the
    // 2026-07-27 fix those arguments are (worksheet line 1 = 1040 L15) and (worksheet line 2 = the
    // §56(b)(1) add-back), not (AGI, QBI). Passing the whole figure as line 1 with a zero add-back yields
    // the SAME worksheet line 3, so each case still exercises exactly the line-5 value its comment names.

    /// ★ THE REGRESSION KAT for the 2026-07-27 line-7-vs-line-17 fix. An ordinary MFJ itemizer whom the
    /// real worksheet CLEARS, and whom the old closed form (`line 3 = AGI − QBI`) falsely refused.
    ///
    /// MFJ, AGI $300,000, $80,000 itemized of which only $10,000 is SALT (the §164(b)(6) cap) — the other
    /// $70,000 is mortgage interest and charitable gifts, both AMT-**allowed**. Taxable income $220,000;
    /// regular tax $38,885.
    ///   CORRECT: line 1 = 220,000 + line 2 = 10,000 → line 3 = 230,000; line 7 = 96,700; below the
    ///   phase-out so line 11 = 96,700 ≤ 232,600; line 12 = 26% × 96,700 = 25,142 ≤ 38,885 → CLEARED.
    ///   OLD BUG: line 5 = AGI 300,000 → line 7 = 166,700 → line 12 = 43,342 > 38,885 → REFUSED.
    ///
    /// MUTATION: pass `dec!(300000)` as line 1 (i.e. restore `AGI − QBI`) and this assertion reds.
    #[test]
    fn itemizer_addback_is_schedule_a_line7_not_the_itemized_total() {
        let line2 = amt_worksheet_line2(true, dec!(29200), dec!(10000));
        assert_eq!(
            line2,
            dec!(10000),
            "an itemizer adds back ONLY Schedule A line 7 (capped SALT)"
        );
        assert!(
            !amt_should_file_6251(
                FilingStatus::Mfj,
                dec!(220000), // 1040 L15
                line2,
                Usd::ZERO,
                dec!(38885), // 1040 L16
                Usd::ZERO,
                &amt()
            ),
            "the real worksheet clears this filer (26% x 96,700 = 25,142 <= 38,885); adding back the \
             non-SALT itemized deductions manufactures a false refusal"
        );
        // The counterfactual the bug computed, pinned so the two are visibly different decisions.
        assert!(
            amt_should_file_6251(
                FilingStatus::Mfj,
                dec!(300000), // the buggy line 3 (= AGI, i.e. every itemized deduction added back)
                Usd::ZERO,
                Usd::ZERO,
                dec!(38885),
                Usd::ZERO,
                &amt()
            ),
            "sanity: the OLD closed form really did refuse this filer — the KAT above is a live fix, \
             not a tautology"
        );
    }

    /// The standard-deduction branch is unchanged by the fix: §56(b)(1)(D) disallows the standard deduction
    /// outright, so the whole of it is the add-back and `L15 + std == AGI - QBI` still holds.
    #[test]
    fn standard_deduction_branch_adds_back_the_whole_standard_deduction() {
        assert_eq!(
            amt_worksheet_line2(false, dec!(29200), dec!(10000)),
            dec!(29200),
            "a standard-deduction filer adds back the standard deduction, never Schedule A line 7"
        );
        // MFJ, AGI 300,000, standard 29,200, no QBI → L15 = 270,800; line 3 = 270,800 + 29,200 = 300,000,
        // which is exactly the old closed form's AGI − QBI. Equivalence pinned.
        let line2 = amt_worksheet_line2(false, dec!(29200), Usd::ZERO);
        assert!(!amt_should_file_6251(
            FilingStatus::Mfj,
            dec!(270800),
            line2,
            Usd::ZERO,
            dec!(60000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// Below the exemption (worksheet line 7 "No") ⇒ no AMT, don't fill Form 6251.
    #[test]
    fn below_exemption_clears() {
        // line5 = 50,000 < 85,700 exemption → STOP.
        assert!(!amt_should_file_6251(
            FilingStatus::Single,
            dec!(50000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(5000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// Line 12 STOP — line 11 over the 26%/28% breakpoint ($232,600) forces Form 6251 (very high income).
    #[test]
    fn over_breakpoint_forces_6251() {
        // agi 900,000: line5 900,000; line7 814,300; phaseout → line9 290,650, line10 min(72,662.50, 85,700)
        // = 72,662.50; line11 886,962.50 > 232,600 → fill 6251.
        assert!(amt_should_file_6251(
            FilingStatus::Single,
            dec!(900000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(300000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// The "Next" test — high AMTI but LOW regular tax (LTCG-heavy filer): tentative 26% exceeds L16 ⇒ fill
    /// 6251 (the worksheet over-triggers because it ignores the LTCG preferential AMT rate; v1 refuses since
    /// it can't compute the real 6251). A large-enough L16 clears it.
    #[test]
    fn next_test_high_amti_low_regular_tax() {
        // agi 300,000 (< phase-out): line5 300,000; line7 214,300 ≤ 232,600 (no STOP); line12 = 26% ×
        // 214,300 = 55,718. L16 = 45,000 → 55,718 > 45,000 → fill 6251.
        assert!(amt_should_file_6251(
            FilingStatus::Single,
            dec!(300000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(45000),
            Usd::ZERO,
            &amt()
        ));
        // Same AMTI but L16 = 60,000 → 55,718 ≤ 60,000 → cleared (no AMT).
        assert!(!amt_should_file_6251(
            FilingStatus::Single,
            dec!(300000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(60000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// A state tax refund (worksheet line 4) is SUBTRACTED — it is not AMT income, so it lowers line 5 and
    /// can clear a filer who would otherwise be over the exemption.
    #[test]
    fn state_refund_lowers_line5() {
        // agi 90,000; without the refund line5 = 90,000 > 85,700 (would continue); a $6,000 refund →
        // line5 = 84,000 ≤ 85,700 → STOP (cleared).
        assert!(!amt_should_file_6251(
            FilingStatus::Single,
            dec!(90000),
            Usd::ZERO,
            dec!(6000),
            dec!(9000),
            Usd::ZERO,
            &amt()
        ));
        // Without the refund, the same return continues past the exemption (and here clears on the Next
        // test only because L16 is high) — so the refund is genuinely load-bearing at the line-7 gate.
        assert!(!amt_should_file_6251(
            FilingStatus::Single,
            dec!(90000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(9000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// MFS uses the lower exemption ($66,650) and the halved breakpoint ($116,300): a mid-income MFS filer
    /// with low regular tax is caught where the same numbers under Single would clear.
    #[test]
    fn mfs_lower_thresholds() {
        // MFS, agi 200,000: line5 200,000; exemption 66,650 → line7 133,350 > 116,300 breakpoint → fill 6251.
        assert!(amt_should_file_6251(
            FilingStatus::Mfs,
            dec!(200000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(40000),
            Usd::ZERO,
            &amt()
        ));
        // The identical dollars as Single: line7 = 200,000 − 85,700 = 114,300 ≤ 232,600 (no line-12 STOP);
        // line12 = 26% × 114,300 = 29,718 ≤ 40,000 L16 → cleared. (MFS is stricter.)
        assert!(!amt_should_file_6251(
            FilingStatus::Single,
            dec!(200000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(40000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// The line-12 breakpoint STOP is load-bearing: line 11 just over $232,600 refuses even when the
    /// regular tax is high enough that the "Next" 26% test alone would clear.
    #[test]
    fn breakpoint_stop_is_load_bearing() {
        // agi 325,700 → line5 325,700 (< phase-out); line7 = line11 = 240,000 > 232,600 → STOP (fill 6251),
        // even though 26% × 240,000 = 62,400 < L16 70,000 (which alone would clear on the Next test).
        assert!(amt_should_file_6251(
            FilingStatus::Single,
            dec!(325700),
            Usd::ZERO,
            Usd::ZERO,
            dec!(70000),
            Usd::ZERO,
            &amt()
        ));
    }

    /// I1 (Fable r1): the worksheet's "fill 6251 if you claimed the Foreign Tax Credit" Exception needs no
    /// separate refuse — for the §904(j) ≤$300/$600 passive FTC, the AMT FTC equals the regular FTC, so it
    /// cancels from both sides of the AMT comparison `(TMT − AMTFTC) vs (L16 − FTC)` → `TMT vs L16`, exactly
    /// the screen's line-12-vs-line-13 test. This pins the Next-test threshold AND that subtracting any
    /// common FTC `f` from both lines leaves the decision unchanged (the cancellation, §59-scoped).
    #[test]
    fn ftc_cancels_from_the_amt_decision() {
        let a = amt();
        // agi 300,000 (< phase-out) → line 11 = 214,300 ≤ 232,600 (no line-12 STOP); line 12 = 26% ×
        // 214,300 = 55,718. The Next test is `55,718 > L16`; subtracting any FTC f from both line 12 and
        // line 13 gives `(55,718 − f) > (L16 − f)` ⇔ `55,718 > L16` — unchanged.
        for l16 in [dec!(50000), dec!(55000), dec!(55718), dec!(56000)] {
            let decision = amt_should_file_6251(
                FilingStatus::Single,
                dec!(300000),
                Usd::ZERO,
                Usd::ZERO,
                l16,
                Usd::ZERO,
                &a,
            );
            assert_eq!(decision, dec!(55718) > l16, "L16 = {l16}");
        }
    }

    /// QBI is subtracted at line 3 (AGI − QBI): a large QBI deduction lowers line 5.
    #[test]
    fn qbi_reduces_line3() {
        // agi 90,000, QBI 10,000 → line5 = 80,000 ≤ 85,700 → cleared (would continue without the QBI).
        assert!(!amt_should_file_6251(
            FilingStatus::Single,
            dec!(90000),
            dec!(10000),
            Usd::ZERO,
            dec!(9000),
            Usd::ZERO,
            &amt()
        ));
    }
}
