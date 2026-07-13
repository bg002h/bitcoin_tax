//! Full-return v1 **AMT screen** (Phase 4 / SPEC §4.11): the 2024 "Worksheet To See if You Should Fill
//! in Form 6251 — Schedule 2, Line 2", implemented as a **refuse-trigger**. v1 does not compute Form 6251;
//! when the worksheet concludes the taxpayer must fill it in, the return is REFUSED (fail-closed). When the
//! worksheet clears the taxpayer, Schedule 2 line 2 (AMT) is $0 — a sound conclusion, because the worksheet
//! deliberately OVER-estimates AMTI (it adds back every itemized deduction), so clearing it means no AMT.
//!
//! **The worksheet reduces to a closed form.** Line 3 = AGI − QBI in BOTH the itemize and the standard
//! branch: itemizing, line 3 = taxable income (L15) + Schedule A line 7 (= L12) = (AGI − L12 − L13) + L12 =
//! AGI − L13; not itemizing, line 3 = AGI − L13 directly. So no std-vs-itemized branch is needed here.
//!
//! **The worksheet's "Exception"** (preference items that force Form 6251 directly — §4952 investment
//! interest, accelerated depreciation, PAB tax-exempt interest, ISO stock, §1202 exclusion, NOL, …) is
//! covered: PAB interest is already refused (INT box 9 / DIV box 13, `screen_inputs`), and every other
//! exception item is an out-of-scope input v1 never captures.
use crate::conventions::Usd;
use crate::tax::tables::AmtParams;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// Run the 2024 AMT worksheet (SPEC §4.11). Returns `true` when the worksheet says **fill in Form 6251**
/// (→ refuse in v1), `false` when it clears the taxpayer (Schedule 2 line 2 = 0). All comparisons are the
/// worksheet's strict "more than" (`>`).
///
/// - `agi` = 1040 L11; `qbi_deduction` = 1040 L13 (worksheet line 3 = AGI − QBI, both branches).
/// - `state_refund_and_8z` = Schedule 1 lines 1 + 8z (worksheet line 4, subtracted — a state refund is not
///   AMT income). v1 has no Sch 1 L8z input, so this is just the taxable state refund.
/// - `regular_tax_l16` = 1040 L16; `sch2_l1z` = Schedule 2 line 1z (worksheet line 13 = L16 + L1z; L1z is
///   the excess-APTC total, 0 in v1 — no input).
pub fn amt_should_file_6251(
    status: FilingStatus,
    agi: Usd,
    qbi_deduction: Usd,
    state_refund_and_8z: Usd,
    regular_tax_l16: Usd,
    sch2_l1z: Usd,
    amt: &AmtParams,
) -> bool {
    // Lines 3 & 5 — AGI − QBI, then less any state refund (worksheet line 4).
    let line5 = agi - qbi_deduction - state_refund_and_8z;

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
        }
    }

    /// Below the exemption (worksheet line 7 "No") ⇒ no AMT, don't fill Form 6251.
    #[test]
    fn below_exemption_clears() {
        // line5 = 50,000 < 85,700 exemption → STOP.
        assert!(!amt_should_file_6251(
            FilingStatus::Single, dec!(50000), Usd::ZERO, Usd::ZERO, dec!(5000), Usd::ZERO, &amt()
        ));
    }

    /// Line 12 STOP — line 11 over the 26%/28% breakpoint ($232,600) forces Form 6251 (very high income).
    #[test]
    fn over_breakpoint_forces_6251() {
        // agi 900,000: line5 900,000; line7 814,300; phaseout → line9 290,650, line10 min(72,662.50, 85,700)
        // = 72,662.50; line11 886,962.50 > 232,600 → fill 6251.
        assert!(amt_should_file_6251(
            FilingStatus::Single, dec!(900000), Usd::ZERO, Usd::ZERO, dec!(300000), Usd::ZERO, &amt()
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
            FilingStatus::Single, dec!(300000), Usd::ZERO, Usd::ZERO, dec!(45000), Usd::ZERO, &amt()
        ));
        // Same AMTI but L16 = 60,000 → 55,718 ≤ 60,000 → cleared (no AMT).
        assert!(!amt_should_file_6251(
            FilingStatus::Single, dec!(300000), Usd::ZERO, Usd::ZERO, dec!(60000), Usd::ZERO, &amt()
        ));
    }

    /// A state tax refund (worksheet line 4) is SUBTRACTED — it is not AMT income, so it lowers line 5 and
    /// can clear a filer who would otherwise be over the exemption.
    #[test]
    fn state_refund_lowers_line5() {
        // agi 90,000; without the refund line5 = 90,000 > 85,700 (would continue); a $6,000 refund →
        // line5 = 84,000 ≤ 85,700 → STOP (cleared).
        assert!(!amt_should_file_6251(
            FilingStatus::Single, dec!(90000), Usd::ZERO, dec!(6000), dec!(9000), Usd::ZERO, &amt()
        ));
        // Without the refund, the same return continues past the exemption (and here clears on the Next
        // test only because L16 is high) — so the refund is genuinely load-bearing at the line-7 gate.
        assert!(!amt_should_file_6251(
            FilingStatus::Single, dec!(90000), Usd::ZERO, Usd::ZERO, dec!(9000), Usd::ZERO, &amt()
        ));
    }

    /// MFS uses the lower exemption ($66,650) and the halved breakpoint ($116,300): a mid-income MFS filer
    /// with low regular tax is caught where the same numbers under Single would clear.
    #[test]
    fn mfs_lower_thresholds() {
        // MFS, agi 200,000: line5 200,000; exemption 66,650 → line7 133,350 > 116,300 breakpoint → fill 6251.
        assert!(amt_should_file_6251(
            FilingStatus::Mfs, dec!(200000), Usd::ZERO, Usd::ZERO, dec!(40000), Usd::ZERO, &amt()
        ));
        // The identical dollars as Single: line7 = 200,000 − 85,700 = 114,300 ≤ 232,600 (no line-12 STOP);
        // line12 = 26% × 114,300 = 29,718 ≤ 40,000 L16 → cleared. (MFS is stricter.)
        assert!(!amt_should_file_6251(
            FilingStatus::Single, dec!(200000), Usd::ZERO, Usd::ZERO, dec!(40000), Usd::ZERO, &amt()
        ));
    }

    /// The line-12 breakpoint STOP is load-bearing: line 11 just over $232,600 refuses even when the
    /// regular tax is high enough that the "Next" 26% test alone would clear.
    #[test]
    fn breakpoint_stop_is_load_bearing() {
        // agi 325,700 → line5 325,700 (< phase-out); line7 = line11 = 240,000 > 232,600 → STOP (fill 6251),
        // even though 26% × 240,000 = 62,400 < L16 70,000 (which alone would clear on the Next test).
        assert!(amt_should_file_6251(
            FilingStatus::Single, dec!(325700), Usd::ZERO, Usd::ZERO, dec!(70000), Usd::ZERO, &amt()
        ));
    }

    /// QBI is subtracted at line 3 (AGI − QBI): a large QBI deduction lowers line 5.
    #[test]
    fn qbi_reduces_line3() {
        // agi 90,000, QBI 10,000 → line5 = 80,000 ≤ 85,700 → cleared (would continue without the QBI).
        assert!(!amt_should_file_6251(
            FilingStatus::Single, dec!(90000), dec!(10000), Usd::ZERO, dec!(9000), Usd::ZERO, &amt()
        ));
    }
}
