//! Full-return v1 **Form 8995** — the simplified §199A qualified-business-income deduction, REIT
//! (and PTP) dividend path only (Phase 4 task 1 / SPEC §4.5).
//!
//! v1's ONLY QBI source is §199A REIT dividends (1099-DIV box 5). Crypto Schedule C business income is
//! **not** §199A QBI in v1 (a follow-on adds the QBI-on-Schedule-C path), so Form 8995 lines 1–5 (the
//! trade/business QBI component) are always zero here and the deduction is the REIT/PTP component only.
//!
//! **Above the §199A(e)(2) threshold the simplified 8995 is unavailable** (the 8995-A phase-in is
//! unmodeled) — the caller REFUSES (`qbi_over_threshold`) rather than under-deduct. The REIT/PTP loss
//! carryforward (line 17) persists to next year (§4.5 write-back).
use crate::conventions::{round_dollar, Usd};
use crate::tax::tables::FullReturnParams;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// §199A REIT/PTP-component rate (Form 8995 lines 9 & 14).
const QBI_RATE: Usd = dec!(0.20);

/// The simplified Form 8995 result: the QBI deduction (→ 1040 L13) + the REIT/PTP loss carryforward-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Qbi8995 {
    /// Form 8995 **line 15** → 1040 **L13** — the §199A qualified-business-income deduction (whole dollars).
    pub deduction: Usd,
    /// Form 8995 **line 17** — the qualified REIT-dividend / PTP-income LOSS carryforward to next year
    /// (a magnitude ≥ 0: the unused prior-year loss carryforward net of this year's REIT income).
    pub reit_ptp_carryforward_out: Usd,
}

/// Whether the return has ANY qualified business income (v1: only §199A REIT dividends or a prior-year
/// REIT/PTP loss carryforward). When false there is no Form 8995 — no deduction, and no above-threshold
/// refuse (crypto Schedule C is not §199A QBI in v1, so it never triggers the 8995).
pub fn has_qbi(reit_dividends: Usd, reit_ptp_carryforward_in: Usd) -> bool {
    reit_dividends > Usd::ZERO || reit_ptp_carryforward_in > Usd::ZERO
}

/// Compute the simplified **Form 8995** QBI deduction (→ 1040 L13) — REIT/PTP path.
///
/// - `reit_dividends` = Σ 1099-DIV box 5 (§199A dividends), a magnitude ≥ 0.
/// - `reit_ptp_carryforward_in` = the prior-year REIT/PTP LOSS carryforward (magnitude ≥ 0; Form 8995
///   line 7 enters it as a loss, so it REDUCES line 8).
/// - `ti_before_qbi` = 1040 AGI − L12 (Form 8995 line 11).
/// - `net_capital_gain` = qualified dividends + net LTCG taxed at preferential rates (Form 8995 line 12).
///
/// The caller MUST have already refused when `ti_before_qbi` is above the §199A(e)(2) threshold with QBI
/// present ([`qbi_over_threshold`]) — the 8995-A phase-in is unmodeled. Lines 1–5 (trade/business QBI)
/// are 0 in v1, so the deduction is the REIT/PTP component (line 9) capped by the income limit (line 14).
pub fn compute_8995(
    reit_dividends: Usd,
    reit_ptp_carryforward_in: Usd,
    ti_before_qbi: Usd,
    net_capital_gain: Usd,
) -> Qbi8995 {
    // Line 8 — total qualified REIT dividends + PTP income (line 6 + line-7 loss, not below zero).
    let line8 = (reit_dividends - reit_ptp_carryforward_in).max(Usd::ZERO);
    // Line 9 = line 10 (line 5 QBI component is 0 in v1) — REIT/PTP component = 20% of line 8.
    let component = round_dollar(QBI_RATE * line8);
    // Line 13 — taxable income before QBI, less net capital gain (≥ 0).
    let line13 = (ti_before_qbi - net_capital_gain).max(Usd::ZERO);
    // Line 14 — income limit = 20% of line 13.
    let income_limit = round_dollar(QBI_RATE * line13);
    // Line 15 → 1040 L13 — the lesser of the component and the income limit.
    let deduction = component.min(income_limit);
    // Line 17 — the prior-year loss carryforward unused against this year's REIT income (magnitude).
    let reit_ptp_carryforward_out = (reit_ptp_carryforward_in - reit_dividends).max(Usd::ZERO);
    Qbi8995 {
        deduction,
        reit_ptp_carryforward_out,
    }
}

/// Whether QBI must be **refused**: there IS QBI (REIT dividends or a carryforward) AND the taxable
/// income before the QBI deduction exceeds the §199A(e)(2) threshold (at/below the threshold the
/// simplified Form 8995 applies; above it the 8995-A phase-in is required — unmodeled in v1, SPEC §4.5).
pub fn qbi_over_threshold(
    reit_dividends: Usd,
    reit_ptp_carryforward_in: Usd,
    ti_before_qbi: Usd,
    status: FilingStatus,
    params: &FullReturnParams,
) -> bool {
    has_qbi(reit_dividends, reit_ptp_carryforward_in)
        && ti_before_qbi > params.qbi_ti_threshold(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn params() -> FullReturnParams {
        let mut std_deduction = BTreeMap::new();
        for s in [
            FilingStatus::Single,
            FilingStatus::Mfj,
            FilingStatus::Mfs,
            FilingStatus::HoH,
        ] {
            std_deduction.insert(s, dec!(14600));
        }
        FullReturnParams {
            year: 2024,
            std_deduction,
            std_aged_blind_married: dec!(1550),
            std_aged_blind_unmarried: dec!(1950),
            dependent_std_floor: dec!(1300),
            dependent_std_earned_addon: dec!(450),
            salt_cap: dec!(10000),
            kiddie_unearned_threshold: dec!(2600),
            elective_deferral_limit: dec!(23000),
            ftc_ceiling: dec!(300),
            qbi_ti_threshold_unmarried: dec!(191950),
            qbi_ti_threshold_married: dec!(383900),
            student_loan_phaseout_unmarried: (dec!(80000), dec!(95000)),
            student_loan_phaseout_married: (dec!(165000), dec!(195000)),
            amt: crate::tax::tables::AmtParams {
                exemption_single_hoh: dec!(85700),
                exemption_mfj_qss: dec!(133300),
                exemption_mfs: dec!(66650),
                phaseout_start_single_hoh_mfs: dec!(609350),
                phaseout_start_mfj_qss: dec!(1218700),
                breakpoint_28pct: dec!(232600),
                breakpoint_28pct_mfs: dec!(116300),
            },
        }
    }

    /// KAT-16 (QBI): REIT box5 × 20% when the income limit does NOT bind. $10,000 REIT dividends →
    /// line 9 = $2,000; income limit 20% × ($100,000 − $5,000 net cap gain) = $19,000 → L13 = $2,000.
    #[test]
    fn reit_component_when_income_limit_slack() {
        let r = compute_8995(dec!(10000), Usd::ZERO, dec!(100000), dec!(5000));
        assert_eq!(r.deduction, dec!(2000));
        assert_eq!(r.reit_ptp_carryforward_out, Usd::ZERO);
    }

    /// The TI-before-QBI income limit (line 14) binds when it is smaller than the REIT component.
    /// $10,000 REIT → component $2,000; TI-before-QBI $6,000, no net cap gain → limit 20%×6,000 = $1,200.
    #[test]
    fn income_limit_binds_below_component() {
        let r = compute_8995(dec!(10000), Usd::ZERO, dec!(6000), Usd::ZERO);
        assert_eq!(r.deduction, dec!(1200));
    }

    /// Net capital gain (qualified dividends + preferential LTCG) is SUBTRACTED from the income-limit
    /// base (line 13): a return that is all preferential income gets a $0 QBI income limit.
    #[test]
    fn net_capital_gain_reduces_the_income_limit() {
        // TI-before-QBI $50,000 all of which is net capital gain → line 13 = 0 → limit 0 → L13 = 0.
        let r = compute_8995(dec!(10000), Usd::ZERO, dec!(50000), dec!(50000));
        assert_eq!(r.deduction, Usd::ZERO);
    }

    /// A prior-year REIT/PTP loss carryforward reduces this year's line 8 and, if it exceeds this year's
    /// REIT income, produces a fresh loss carryforward-out (Form 8995 line 17, a magnitude).
    #[test]
    fn loss_carryforward_in_reduces_income_and_carries_out() {
        // $4,000 REIT − $10,000 prior loss → line 8 = 0 → deduction 0; $6,000 loss carries out.
        let r = compute_8995(dec!(4000), dec!(10000), dec!(100000), Usd::ZERO);
        assert_eq!(r.deduction, Usd::ZERO);
        assert_eq!(r.reit_ptp_carryforward_out, dec!(6000));
    }

    /// No REIT dividends and no carryforward ⇒ no QBI at all: no deduction, no carryforward, not "over".
    #[test]
    fn no_qbi_when_no_reit() {
        assert!(!has_qbi(Usd::ZERO, Usd::ZERO));
        let r = compute_8995(Usd::ZERO, Usd::ZERO, dec!(500000), Usd::ZERO);
        assert_eq!(r.deduction, Usd::ZERO);
        assert_eq!(r.reit_ptp_carryforward_out, Usd::ZERO);
        // Even far above the threshold, no QBI ⇒ no refuse.
        assert!(!qbi_over_threshold(
            Usd::ZERO,
            Usd::ZERO,
            dec!(500000),
            FilingStatus::Single,
            &params()
        ));
    }

    /// §199A(e)(2) refuse: with QBI present, TI-before-QBI ABOVE the threshold refuses (8995-A unmodeled);
    /// AT the threshold is fine (simplified 8995 applies). MFJ uses the doubled threshold; QSS the base.
    #[test]
    fn over_threshold_refuse_boundary() {
        let p = params();
        // Single: $191,951 > $191,950 → refuse; exactly $191,950 → OK.
        assert!(qbi_over_threshold(
            dec!(1000),
            Usd::ZERO,
            dec!(191951),
            FilingStatus::Single,
            &p
        ));
        assert!(!qbi_over_threshold(
            dec!(1000),
            Usd::ZERO,
            dec!(191950),
            FilingStatus::Single,
            &p
        ));
        // MFJ threshold is $383,900: $300,000 OK, $400,000 refuses.
        assert!(!qbi_over_threshold(
            dec!(1000),
            Usd::ZERO,
            dec!(300000),
            FilingStatus::Mfj,
            &p
        ));
        assert!(qbi_over_threshold(
            dec!(1000),
            Usd::ZERO,
            dec!(400000),
            FilingStatus::Mfj,
            &p
        ));
        // QSS is NOT a joint return → uses the $191,950 base (refuses at $300,000, unlike MFJ).
        assert!(qbi_over_threshold(
            dec!(1000),
            Usd::ZERO,
            dec!(300000),
            FilingStatus::Qss,
            &p
        ));
        // A carryforward alone (no current REIT) is still QBI for the refuse trigger.
        assert!(qbi_over_threshold(
            Usd::ZERO,
            dec!(5000),
            dec!(200000),
            FilingStatus::Single,
            &p
        ));
    }

    /// Printed Form 8995 lines are `round_dollar` (half-up): a $2,502.50 REIT component rounds the 20%
    /// line to $501 (not the round-half-even $500) — the printed-line rounding policy (SPEC §3.1).
    #[test]
    fn component_line_rounds_half_up() {
        let r = compute_8995(dec!(2502.50), Usd::ZERO, dec!(100000), Usd::ZERO);
        // 20% × 2,502.50 = 500.50 → half-up 501 (income limit 20% × 100,000 = 20,000 does not bind).
        assert_eq!(r.deduction, dec!(501));
    }
}
