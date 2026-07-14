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

/// Whether the return has ANY qualified business income: a crypto **Schedule C trade or business**, §199A
/// REIT dividends, or a prior-year REIT/PTP loss carryforward. When false there is no Form 8995 — no
/// deduction, and no above-threshold refuse.
pub fn has_qbi(business_qbi: Usd, reit_dividends: Usd, reit_ptp_carryforward_in: Usd) -> bool {
    business_qbi > Usd::ZERO || reit_dividends > Usd::ZERO || reit_ptp_carryforward_in > Usd::ZERO
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
    business_qbi: Usd,
    reit_dividends: Usd,
    reit_ptp_carryforward_in: Usd,
    ti_before_qbi: Usd,
    net_capital_gain: Usd,
) -> Qbi8995 {
    // ── Lines 1–5 — the QUALIFIED BUSINESS component (the crypto Schedule C trade or business).
    //
    // ★ `business_qbi` is Schedule C's net profit REDUCED by the §164(f) deductible half of SE tax. The
    // Form 8995 instructions define QBI net of the deductible part of SE tax, self-employed health
    // insurance and self-employed retirement contributions — of which v1 models only the first (the other
    // two have no input). A crypto mining trade or business is a qualified trade or business (it is not an
    // SSTB), so its owner is entitled to this deduction, and omitting it OVERSTATED their tax by ~20% of
    // their business income. Found by the P7 independent-oracle cross-check.
    //
    // A Schedule C LOSS cannot reach here: it refuses upstream (`ScheduleCLoss`), so there is no negative
    // QBI and no QBI loss carryforward in v1 (Form 8995 lines 3 and 16 stay blank).
    let line5 = round_dollar(QBI_RATE * business_qbi.max(Usd::ZERO));

    // Line 8 — total qualified REIT dividends + PTP income (line 6 + line-7 loss, not below zero).
    let line8 = (reit_dividends - reit_ptp_carryforward_in).max(Usd::ZERO);
    // Line 9 — the REIT/PTP component = 20% of line 8.
    let line9 = round_dollar(QBI_RATE * line8);
    // Line 10 — the QBI deduction BEFORE the income limitation = line 5 + line 9. The two components ADD.
    let component = line5 + line9;
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
    business_qbi: Usd,
    reit_dividends: Usd,
    reit_ptp_carryforward_in: Usd,
    ti_before_qbi: Usd,
    status: FilingStatus,
    params: &FullReturnParams,
) -> bool {
    // ★ Now that a Schedule C trade or business EARNS the deduction, it is also subject to the
    // §199A(e)(2) threshold: above it the simplified Form 8995 no longer applies (the W-2-wage / UBIA
    // limitations and the SSTB phase-in take over, which is Form 8995-A), and v1 REFUSES rather than
    // compute a deduction it cannot bound.
    has_qbi(business_qbi, reit_dividends, reit_ptp_carryforward_in)
        && ti_before_qbi > params.qbi_ti_threshold(status)
}

/// The printable **Form 8995 line chain** — whole dollars, cross-footing (SPEC §3.1). See
/// `other_taxes::Form8959Lines` for why the chain is derived in core and only transcribed by
/// `btctax-forms`.
///
/// **The Part I table (rows 1i–1v) is BLANK**: v1's only QBI is §199A REIT dividends, so there is no
/// trade or business to list. Lines 2/4/5 are nevertheless PRINTED as zero — the form's arithmetic
/// adds them (line 10 = line 5 + line 9), and a reader re-adding the column must find them. Line 3
/// (a prior-year trade/business QBI loss carryforward) has no v1 input and stays blank.
///
/// **★ Lines 3, 7, 16 and 17 are PARENTHESIZED boxes on the printed form: the parentheses supply the
/// minus sign, so the value written must be a POSITIVE MAGNITUDE.** Writing `-1234` renders as
/// `(-1,234)` — a positive number. Every one of these fields is a loss/carryforward, and every one is
/// stored here as a magnitude ≥ 0 for exactly that reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form8995Lines {
    /// L2 — total QBI from lines 1i–1v column (c). Always 0 in v1 (no trade/business QBI).
    pub line2: Usd,
    /// L4 — combine 2 and 3; if zero or less, `-0-`. Always 0 in v1.
    pub line4: Usd,
    /// L5 — QBI component = 20% × line 4. Always 0 in v1.
    pub line5: Usd,
    /// L6 — qualified REIT dividends + PTP income (Σ 1099-DIV box 5).
    pub line6: Usd,
    /// L7 — prior-year REIT/PTP **loss** carryforward. **Positive magnitude** (parenthesized box).
    pub line7: Usd,
    /// L8 — combine 6 and 7; if zero or less, `-0-`. (The line-7 loss REDUCES line 6.)
    pub line8: Usd,
    /// L9 — REIT/PTP component = 20% × line 8.
    pub line9: Usd,
    /// L10 — QBI deduction before the income limitation = line 5 + line 9.
    pub line10: Usd,
    /// L11 — taxable income before the QBI deduction (1040 AGI − L12).
    pub line11: Usd,
    /// L12 — net capital gain, increased by qualified dividends.
    pub line12: Usd,
    /// L13 — subtract 12 from 11; if zero or less, `-0-`.
    pub line13: Usd,
    /// L14 — income limitation = 20% × line 13.
    pub line14: Usd,
    /// L15 — the QBI deduction = the smaller of line 10 or line 14 → 1040 **L13**.
    pub line15: Usd,
    /// L16 — total qualified business (loss) carryforward = combine 2 and 3; if > 0, `-0-`. Always 0
    /// in v1. **Positive magnitude** (parenthesized box).
    pub line16: Usd,
    /// L17 — total REIT/PTP (loss) carryforward = combine 6 and 7; if > 0, `-0-`. Carries to next
    /// year. **Positive magnitude** (parenthesized box).
    pub line17: Usd,
}

/// Derive the printed Form 8995 chain from the same inputs as [`compute_8995`].
///
/// Returns `None` when there is no QBI at all ([`has_qbi`] is false) — no REIT dividends and no
/// prior-year carryforward means no Form 8995, no deduction, and nothing to carry forward.
///
/// The caller MUST already have refused when [`qbi_over_threshold`] (the 8995-A phase-in is
/// unmodeled), exactly as for [`compute_8995`].
///
/// Note the printed line 15 is `min(printed line 10, printed line 14)` — each rounded at its own
/// line — so it can differ by a dollar from `compute_8995`'s `deduction`, which rounds only the 20%
/// products. That is the SPEC §3.1 round-all-amounts election, not a defect: the printed form
/// cross-foots against itself, which is what gets filed.
pub fn form_8995_lines(
    business_qbi: Usd,
    reit_dividends: Usd,
    reit_ptp_carryforward_in: Usd,
    ti_before_qbi: Usd,
    net_capital_gain: Usd,
) -> Option<Form8995Lines> {
    if !has_qbi(business_qbi, reit_dividends, reit_ptp_carryforward_in) {
        return None;
    }
    // Part I — the trade-or-business QBI (the crypto Schedule C), net of the §164(f) half-SE deduction.
    // Line 3 (prior-year QBI loss carryforward) stays BLANK: a Schedule C loss refuses upstream, so v1
    // never carries one.
    let line2 = round_dollar(business_qbi.max(Usd::ZERO));
    let line4 = line2;
    let line5 = round_dollar(QBI_RATE * line4);

    // Part I (cont.) — the REIT/PTP component. Line 7 is a positive magnitude that REDUCES line 6.
    let line6 = round_dollar(reit_dividends);
    let line7 = round_dollar(reit_ptp_carryforward_in);
    let line8 = (line6 - line7).max(Usd::ZERO);
    let line9 = round_dollar(QBI_RATE * line8);
    let line10 = line5 + line9;

    // Part II — the taxable-income limitation.
    let line11 = round_dollar(ti_before_qbi);
    let line12 = round_dollar(net_capital_gain);
    let line13 = (line11 - line12).max(Usd::ZERO);
    let line14 = round_dollar(QBI_RATE * line13);
    let line15 = line10.min(line14);

    // Carryforwards out. Both are magnitudes: the form's parentheses supply the sign.
    let line16 = Usd::ZERO; // combine 2 and 3 (= 0); "if greater than zero, enter -0-"
    let line17 = (line7 - line6).max(Usd::ZERO); // the prior-year loss unused against this year's REIT

    Some(Form8995Lines {
        line2,
        line4,
        line5,
        line6,
        line7,
        line8,
        line9,
        line10,
        line11,
        line12,
        line13,
        line14,
        line15,
        line16,
        line17,
    })
}

#[cfg(test)]
mod tests {

    /// ★ **§199A on Schedule C — the deduction the P7 oracle proved we were giving away.**
    ///
    /// A crypto MINING trade or business is a qualified trade or business (not an SSTB), so its owner is
    /// entitled to a §199A deduction of 20% of QBI. btctax v1 originally computed the deduction for REIT
    /// dividends ONLY, silently overstating a miner's tax by ~20% of their business income. The PSL
    /// Tax-Calculator applies it; the golden cross-check exposed the gap; the user's call is to follow the
    /// law ("20% is way too much to give away for free").
    ///
    /// **QBI is the Schedule C net profit REDUCED by the §164(f) deductible half of SE tax** (Form 8995
    /// instructions: QBI is net of the deductible part of SE tax, SE health insurance and SE retirement
    /// contributions — of which v1 models only the first). The oracle independently confirms the rule:
    /// $60,000 profit − $4,239 half-SE = $55,761 of QBI ⇒ a $11,152 deduction, to the dollar.
    #[test]
    fn schedule_c_business_income_earns_the_199a_deduction_net_of_the_half_se_deduction() {
        // The deep/02 Ex.2 shape: $60k of mining, $40k of wages.
        let qbi = dec!(60000) - dec!(4239); // Sch C net − the §164(f) half-SE deduction
        let r = compute_8995(qbi, Usd::ZERO, Usd::ZERO, dec!(95761), Usd::ZERO);

        assert_eq!(
            r.deduction,
            dec!(11152),
            "20% × $55,761 of QBI — the figure the independent oracle computes"
        );
    }

    /// The §199A(b)(3) INCOME LIMITATION still binds: the deduction is the LESSER of 20% of QBI and 20% of
    /// (taxable income − net capital gain). A business owner whose taxable income is mostly preferential
    /// gain cannot deduct against income taxed at capital-gain rates.
    #[test]
    fn the_199a_deduction_is_capped_by_the_income_limitation() {
        // $50,000 of QBI (⇒ a $10,000 component), but only $12,000 of ordinary taxable income.
        let r = compute_8995(dec!(50000), Usd::ZERO, Usd::ZERO, dec!(60000), dec!(48000));

        // Line 13 = 60,000 − 48,000 = 12,000 ⇒ line 14 = 20% × 12,000 = 2,400 < the 10,000 component.
        assert_eq!(
            r.deduction,
            dec!(2400),
            "the income limitation binds — the deduction cannot exceed 20% of NON-preferential income"
        );
    }

    /// Both components ADD: a filer with a business AND REIT dividends gets 20% of each (Form 8995 line
    /// 10 = line 5 + line 9).
    #[test]
    fn the_business_and_reit_components_add() {
        let r = compute_8995(dec!(50000), dec!(10000), Usd::ZERO, dec!(200000), Usd::ZERO);
        assert_eq!(
            r.deduction,
            dec!(12000),
            "20% × 50,000 (business) + 20% × 10,000 (REIT) = 10,000 + 2,000"
        );
    }
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
        let r = compute_8995(
            Usd::ZERO, /* no business QBI */
            dec!(10000),
            Usd::ZERO,
            dec!(100000),
            dec!(5000),
        );
        assert_eq!(r.deduction, dec!(2000));
        assert_eq!(r.reit_ptp_carryforward_out, Usd::ZERO);
    }

    /// The TI-before-QBI income limit (line 14) binds when it is smaller than the REIT component.
    /// $10,000 REIT → component $2,000; TI-before-QBI $6,000, no net cap gain → limit 20%×6,000 = $1,200.
    #[test]
    fn income_limit_binds_below_component() {
        let r = compute_8995(
            Usd::ZERO, /* no business QBI */
            dec!(10000),
            Usd::ZERO,
            dec!(6000),
            Usd::ZERO,
        );
        assert_eq!(r.deduction, dec!(1200));
    }

    /// Net capital gain (qualified dividends + preferential LTCG) is SUBTRACTED from the income-limit
    /// base (line 13): a return that is all preferential income gets a $0 QBI income limit.
    #[test]
    fn net_capital_gain_reduces_the_income_limit() {
        // TI-before-QBI $50,000 all of which is net capital gain → line 13 = 0 → limit 0 → L13 = 0.
        let r = compute_8995(
            Usd::ZERO, /* no business QBI */
            dec!(10000),
            Usd::ZERO,
            dec!(50000),
            dec!(50000),
        );
        assert_eq!(r.deduction, Usd::ZERO);
    }

    /// A prior-year REIT/PTP loss carryforward reduces this year's line 8 and, if it exceeds this year's
    /// REIT income, produces a fresh loss carryforward-out (Form 8995 line 17, a magnitude).
    #[test]
    fn loss_carryforward_in_reduces_income_and_carries_out() {
        // $4,000 REIT − $10,000 prior loss → line 8 = 0 → deduction 0; $6,000 loss carries out.
        let r = compute_8995(
            Usd::ZERO, /* no business QBI */
            dec!(4000),
            dec!(10000),
            dec!(100000),
            Usd::ZERO,
        );
        assert_eq!(r.deduction, Usd::ZERO);
        assert_eq!(r.reit_ptp_carryforward_out, dec!(6000));
    }

    /// No REIT dividends and no carryforward ⇒ no QBI at all: no deduction, no carryforward, not "over".
    #[test]
    fn no_qbi_when_no_reit() {
        assert!(!has_qbi(Usd::ZERO, Usd::ZERO, Usd::ZERO));
        let r = compute_8995(
            Usd::ZERO, /* no business QBI */
            Usd::ZERO,
            Usd::ZERO,
            dec!(500000),
            Usd::ZERO,
        );
        assert_eq!(r.deduction, Usd::ZERO);
        assert_eq!(r.reit_ptp_carryforward_out, Usd::ZERO);
        // Even far above the threshold, no QBI ⇒ no refuse.
        assert!(!qbi_over_threshold(
            Usd::ZERO,
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
            Usd::ZERO,
            dec!(1000),
            Usd::ZERO,
            dec!(191951),
            FilingStatus::Single,
            &p
        ));
        assert!(!qbi_over_threshold(
            Usd::ZERO,
            dec!(1000),
            Usd::ZERO,
            dec!(191950),
            FilingStatus::Single,
            &p
        ));
        // MFJ threshold is $383,900: $300,000 OK, $400,000 refuses.
        assert!(!qbi_over_threshold(
            Usd::ZERO,
            dec!(1000),
            Usd::ZERO,
            dec!(300000),
            FilingStatus::Mfj,
            &p
        ));
        assert!(qbi_over_threshold(
            Usd::ZERO,
            dec!(1000),
            Usd::ZERO,
            dec!(400000),
            FilingStatus::Mfj,
            &p
        ));
        // QSS is NOT a joint return → uses the $191,950 base (refuses at $300,000, unlike MFJ).
        assert!(qbi_over_threshold(
            Usd::ZERO,
            dec!(1000),
            Usd::ZERO,
            dec!(300000),
            FilingStatus::Qss,
            &p
        ));
        // A carryforward alone (no current REIT) is still QBI for the refuse trigger.
        assert!(qbi_over_threshold(
            Usd::ZERO,
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
        let r = compute_8995(
            Usd::ZERO, /* no business QBI */
            dec!(2502.50),
            Usd::ZERO,
            dec!(100000),
            Usd::ZERO,
        );
        // 20% × 2,502.50 = 500.50 → half-up 501 (income limit 20% × 100,000 = 20,000 does not bind).
        assert_eq!(r.deduction, dec!(501));
    }

    /// The printed Form 8995 chain for the ordinary v1 case: REIT dividends, no carryforward, income
    /// limit not binding. Part I's table is blank and lines 2/4/5 print as zero.
    #[test]
    fn form_8995_printed_chain_reit_only() {
        // $10,000 REIT dividends; TI-before-QBI $100,000; net capital gain $20,000.
        // line 9 = 20% × 10,000 = 2,000. line 13 = 80,000 → line 14 = 16,000. line 15 = min = 2,000.
        let l =
            form_8995_lines(Usd::ZERO, dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();
        assert_eq!(l.line2, Usd::ZERO);
        assert_eq!(l.line4, Usd::ZERO);
        assert_eq!(l.line5, Usd::ZERO);
        assert_eq!(l.line6, dec!(10000));
        assert_eq!(l.line7, Usd::ZERO);
        assert_eq!(l.line8, dec!(10000));
        assert_eq!(l.line9, dec!(2000));
        assert_eq!(l.line10, dec!(2000));
        assert_eq!(l.line11, dec!(100000));
        assert_eq!(l.line12, dec!(20000));
        assert_eq!(l.line13, dec!(80000));
        assert_eq!(l.line14, dec!(16000));
        assert_eq!(l.line15, dec!(2000)); // the component binds, not the income limit
        assert_eq!(l.line16, Usd::ZERO);
        assert_eq!(l.line17, Usd::ZERO);
    }

    /// The income limitation binds: line 15 takes line 14, not line 10.
    #[test]
    fn form_8995_printed_chain_income_limit_binds() {
        // TI-before-QBI 12,000 all of which is capital gain → line 13 = 0 → line 14 = 0 → no deduction.
        let l =
            form_8995_lines(Usd::ZERO, dec!(10000), Usd::ZERO, dec!(12000), dec!(12000)).unwrap();
        assert_eq!(l.line10, dec!(2000)); // the component is there…
        assert_eq!(l.line13, Usd::ZERO);
        assert_eq!(l.line14, Usd::ZERO);
        assert_eq!(l.line15, Usd::ZERO); // …but the income limit wipes it out
    }

    /// ★ **The parenthesized-box invariant.** Lines 3/7/16/17 are printed inside literal `(   )` on the
    /// form — the parentheses supply the minus sign — so the value must be a POSITIVE MAGNITUDE. A
    /// negative here renders as `(-1,234)`, i.e. a POSITIVE number on the filed form: a wrong return.
    /// A prior-year loss carryforward larger than this year's REIT income must therefore surface as a
    /// positive line 7 AND a positive line 17, never as a negative anything.
    #[test]
    fn form_8995_loss_carryforward_lines_are_positive_magnitudes() {
        // Prior-year REIT/PTP loss carryforward $15,000 against only $10,000 of REIT dividends.
        let l =
            form_8995_lines(Usd::ZERO, dec!(10000), dec!(15000), dec!(100000), Usd::ZERO).unwrap();
        assert_eq!(l.line6, dec!(10000));
        assert_eq!(l.line7, dec!(15000)); // POSITIVE magnitude, not −15,000
        assert_eq!(l.line8, Usd::ZERO); // 10,000 − 15,000, floored: no REIT income survives
        assert_eq!(l.line9, Usd::ZERO);
        assert_eq!(l.line15, Usd::ZERO); // no deduction this year
        assert_eq!(l.line17, dec!(5000)); // POSITIVE magnitude: 15,000 − 10,000 carries forward

        // Every parenthesized cell is non-negative, always. This is the invariant the filler relies on.
        for cell in [l.line7, l.line16, l.line17] {
            assert!(
                cell >= Usd::ZERO,
                "parenthesized cells are magnitudes: {cell}"
            );
        }
    }

    /// No REIT dividends and no carryforward ⇒ no Form 8995 at all.
    #[test]
    fn form_8995_absent_when_there_is_no_qbi() {
        assert!(
            form_8995_lines(Usd::ZERO, Usd::ZERO, Usd::ZERO, dec!(100000), Usd::ZERO).is_none()
        );
        // …but a bare carryforward, with no REIT income this year, DOES produce the form (it must
        // carry the loss forward on line 17, or the carryforward is silently lost).
        let l = form_8995_lines(Usd::ZERO, Usd::ZERO, dec!(5000), dec!(100000), Usd::ZERO).unwrap();
        assert_eq!(l.line17, dec!(5000));
    }

    /// The printed chain cross-foots: every derived line re-derives from the OTHER printed cells.
    #[test]
    fn form_8995_printed_lines_cross_foot() {
        for (reit, cf_in, ti, ncg) in [
            (dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)),
            (dec!(10000), dec!(15000), dec!(100000), Usd::ZERO),
            (dec!(2502.50), Usd::ZERO, dec!(80000.49), dec!(0.50)), // cents in, dollars out
            (dec!(10000), Usd::ZERO, dec!(12000), dec!(12000)),     // income limit binds
        ] {
            let l = form_8995_lines(Usd::ZERO, reit, cf_in, ti, ncg).unwrap();
            assert_eq!(l.line4, l.line2, "L4 = 2 + 3 (3 blank)");
            assert_eq!(l.line5, round_dollar(QBI_RATE * l.line4), "L5 = 20% × 4");
            assert_eq!(
                l.line8,
                (l.line6 - l.line7).max(Usd::ZERO),
                "L8 = 6 + 7(loss), floored"
            );
            assert_eq!(l.line9, round_dollar(QBI_RATE * l.line8), "L9 = 20% × 8");
            assert_eq!(l.line10, l.line5 + l.line9, "L10 = 5 + 9");
            assert_eq!(
                l.line13,
                (l.line11 - l.line12).max(Usd::ZERO),
                "L13 = 11 − 12, floored"
            );
            assert_eq!(
                l.line14,
                round_dollar(QBI_RATE * l.line13),
                "L14 = 20% × 13"
            );
            assert_eq!(
                l.line15,
                l.line10.min(l.line14),
                "L15 = smaller of 10 or 14"
            );
            assert_eq!(
                l.line17,
                (l.line7 - l.line6).max(Usd::ZERO),
                "L17 = 6 + 7, if > 0 then -0-"
            );
            for cell in [
                l.line2, l.line4, l.line5, l.line6, l.line7, l.line8, l.line9, l.line10, l.line11,
                l.line12, l.line13, l.line14, l.line15, l.line16, l.line17,
            ] {
                assert_eq!(
                    cell.fract(),
                    Usd::ZERO,
                    "printed cells are whole dollars: {cell}"
                );
            }
        }
    }
}
