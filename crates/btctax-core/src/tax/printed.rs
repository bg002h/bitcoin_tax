//! Full-return v1 **printed line chains** for the 1040's numbered schedules (P6 / SPEC §3.1).
//!
//! Every form btctax files needs a *printed* chain distinct from the exact-cents computation:
//!
//! - each printed line is `round_dollar`ed **at the line**, and
//! - each printed **total sums the already-rounded lines above it**, so the filed form cross-foots.
//!
//! That is deliberately NOT `round_dollar(exact_total)` — with two `.50` components the two differ by
//! a dollar, and the cross-footing one is what a human re-adding the column gets (SPEC §10 KAT-9).
//!
//! **The chains compose.** A schedule that carries a figure from another form takes that form's
//! **printed** line, never the exact-cents computation behind it — Schedule 2 line 11 is Form 8959's
//! printed line 18, not `round_dollar(additional_medicare.additional_medicare_tax)`. Otherwise the
//! attached form and the schedule that references it would disagree by a dollar, and the return would
//! not tie out. This is why the builders below take the upstream `*Lines` structs as arguments.
//!
//! **`btctax-forms` does no tax arithmetic**: it transcribes these structs cell-for-cell. A second,
//! independent derivation in the filler is exactly how a filed PDF comes to disagree with the tax it
//! reports, and no core KAT would catch it.

use crate::conventions::{round_dollar, Usd};
use crate::tax::other_taxes::{sch2_line4_se, Form8959Lines, Form8960Lines};
use crate::tax::return_1040::AbsoluteReturn;

/// The printable **Schedule 2 (Additional Taxes)** line chain.
///
/// **Part I is entirely BLANK in v1**, and that is a load-bearing fact rather than an omission:
/// line 1a (excess advance premium tax credit) has no input and would REFUSE if it did (repaying it
/// *increases* tax, so omitting it would understate), and line 2 (AMT) is $0 by construction — the
/// return is refused outright if the official "Should You Fill In Form 6251" worksheet trips. So
/// 1040 line 17 is zero, and nothing in Part I is printed.
///
/// Part II carries the three taxes v1 does compute. Note **line 4 excludes the 0.9% Additional
/// Medicare Tax**: that is a Form 8959 item routed to line 11, and bundling it into line 4 would
/// double-count it (deep/02 C5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule2Lines {
    /// L4 — self-employment tax (Schedule SE): §1401(a) Social Security + §1401(b)(1) regular
    /// Medicare ONLY.
    pub line4: Usd,
    /// L11 — Additional Medicare Tax: **Form 8959's printed line 18**.
    pub line11: Usd,
    /// L12 — net investment income tax: **Form 8960's printed line 17**. Zero when no NIIT is owed.
    pub line12: Usd,
    /// L21 — total other taxes = add 4, 7 through 16, 18 and 19 ⇒ `4 + 11 + 12` here → 1040 **L23**.
    pub line21: Usd,
}

/// Derive the printed Schedule 2 chain. Takes the **printed** 8959/8960 chains, not the computed
/// figures, so the schedule and its attachments agree to the dollar.
///
/// Returns `None` when there is nothing to report — no SE tax, no Additional Medicare Tax, no NIIT —
/// in which case Schedule 2 is not filed at all and 1040 line 23 is zero.
pub fn schedule_2_lines(
    ar: &AbsoluteReturn,
    f8959: &Form8959Lines,
    f8960: Option<&Form8960Lines>,
) -> Option<Schedule2Lines> {
    let line4 = round_dollar(sch2_line4_se(ar.se.as_ref()));
    let line11 = f8959.line18; // already a printed whole dollar
    let line12 = f8960.map_or(Usd::ZERO, |f| f.line17); // ditto
    let line21 = line4 + line11 + line12; // ★ sums the PRINTED lines

    if line21 <= Usd::ZERO {
        return None;
    }
    Some(Schedule2Lines {
        line4,
        line11,
        line12,
        line21,
    })
}

/// The printable **Schedule 3 (Additional Credits and Payments)** line chain.
///
/// Part I carries only the **foreign tax credit** (line 1). Every other nonrefundable credit on the
/// schedule — education, dependent-care, saver's, residential-energy, adoption — is a §3.4
/// *conservative omission*: v1 does not compute it, which can only OVERSTATE tax, and the report
/// fires a loud advisory ([`crate::tax::advisories::Advisory::OtherCreditsOmitted`]). They are left
/// BLANK, never a misleading 0.
///
/// Part II carries only the **§6413(c) excess Social Security** credit (line 11), computed per person
/// and never pooled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule3Lines {
    /// L1 — foreign tax credit (the §904(j) de-minimis election; above the ceiling the return refuses).
    pub line1: Usd,
    /// L8 — total nonrefundable credits = add 1 through 4, 5a, 5b and 7 ⇒ `= line1` → 1040 **L20**.
    pub line8: Usd,
    /// L11 — §6413(c) excess Social Security and tier-1 RRTA tax withheld.
    pub line11: Usd,
    /// L15 — total other payments and refundable credits = add 9 through 12 and 14 ⇒ `= line11`
    /// → 1040 **L31**.
    pub line15: Usd,
}

/// Derive the printed Schedule 3 chain. Returns `None` when there is neither a foreign tax credit nor
/// an excess-Social-Security credit — the schedule is then not filed.
pub fn schedule_3_lines(ar: &AbsoluteReturn) -> Option<Schedule3Lines> {
    let line1 = round_dollar(ar.foreign_tax_credit);
    let line8 = line1; // lines 2-4, 5a, 5b, 7 are all conservatively omitted (blank)
    let line11 = round_dollar(ar.excess_social_security);
    let line15 = line11; // lines 9, 10, 12, 14 are blank

    if line8 <= Usd::ZERO && line15 <= Usd::ZERO {
        return None;
    }
    Some(Schedule3Lines {
        line1,
        line8,
        line11,
        line15,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::other_taxes::form_8959_lines;
    use crate::tax::se::SeTaxResult;
    use crate::tax::types::FilingStatus;
    use rust_decimal_macros::dec;

    fn se_300k_single() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(300000),
            base: dec!(277050.00),
            ss: dec!(21836.40),
            medicare: dec!(8034.45),
            addl: dec!(693.45),
            total: dec!(30564.30),
            deductible_half: dec!(14935.42),
        }
    }

    /// An `AbsoluteReturn` carrying only what the Schedule 2/3 chains read; everything else zero.
    ///
    /// Spelled out in full rather than `..Default::default()` on purpose — `AbsoluteReturn`
    /// deliberately has no `Default`, because a silently-zeroed field on a real tax return is
    /// exactly the class of bug this codebase fails closed against.
    fn ar_with(se: Option<SeTaxResult>, ftc: Usd, excess_ss: Usd) -> AbsoluteReturn {
        use crate::tax::other_taxes::{Form8959, Form8960};
        let z = Usd::ZERO;
        AbsoluteReturn {
            wages: z,
            taxable_interest: z,
            ordinary_dividends: z,
            qualified_dividends: z,
            capital_gain: z,
            schedule_1_income: z,
            total_income: z,
            adjustments: z,
            half_se_deduction: z,
            agi: z,
            se,
            standard_deduction: z,
            itemized_deduction: None,
            deduction: z,
            deduction_is_itemized: false,
            qbi_deduction: z,
            total_deductions: z,
            taxable_income: z,
            net_ltcg: z,
            charitable_carryover_out: Vec::new(),
            qbi_reit_ptp_carryforward_out: z,
            regular_tax: z,
            se_tax_sch2_l4: z,
            schedule_2_other_taxes: z,
            additional_medicare: Form8959 {
                part1_wages: z,
                part2_se: z,
                additional_medicare_tax: z,
                part5_withholding: z,
            },
            niit: Form8960 {
                nii: z,
                magi: z,
                tax: z,
            },
            foreign_tax_credit: ftc,
            ctc_odc_credit: z,
            tax_after_credits: z,
            total_tax: z,
            excess_social_security: excess_ss,
            total_withholding: z,
            total_payments: z,
            overpayment_refund: z,
            amount_owed: z,
        }
    }

    /// ★ Schedule 2 line 4 EXCLUDES the 0.9% Additional Medicare Tax — that is a Form 8959 item, and
    /// it lands on line 11 instead. Bundling it into line 4 would double-count it against the 8959.
    #[test]
    fn schedule_2_line4_excludes_the_addl_medicare_which_lands_on_line_11() {
        let se = se_300k_single();
        let ar = ar_with(Some(se), Usd::ZERO, Usd::ZERO);
        let f8959 = form_8959_lines(FilingStatus::Single, Usd::ZERO, Usd::ZERO, Some(&se));
        let s2 = schedule_2_lines(&ar, &f8959, None).unwrap();

        // line 4 = ss + regular Medicare = 21,836.40 + 8,034.45 = 29,870.85 → 29,871.
        assert_eq!(s2.line4, dec!(29871));
        assert_ne!(
            s2.line4,
            round_dollar(se.total),
            "NOT the §1401 total (that folds in the 0.9%)"
        );
        // the 0.9% shows up HERE instead…
        assert_eq!(s2.line11, dec!(693)); // 8959 printed line 18
        assert_eq!(s2.line12, Usd::ZERO); // no NIIT
        assert_eq!(s2.line21, dec!(30564)); // 29,871 + 693
    }

    /// ★ **The chains COMPOSE on the PRINTED lines.** Schedule 2 line 11 must be Form 8959's printed
    /// line 18 — not `round_dollar` of the exact-cents figure. With the KAT-9 fixture (Part I of
    /// $274.50 and Part II of $499.50) those differ by a dollar: the printed 8959 says 775, while the
    /// exact total rounds to 774. If Schedule 2 took the latter, the schedule and its own attachment
    /// would disagree, and the filed return would not tie out.
    #[test]
    fn schedule_2_line11_takes_the_printed_8959_line_18_not_the_rounded_total() {
        let se = SeTaxResult {
            net_se: dec!(60097.46),
            base: dec!(55500.00),
            ss: dec!(0.00),
            medicare: dec!(1609.50),
            addl: dec!(499.50),
            total: dec!(2109.00),
            deductible_half: dec!(804.75),
        };
        let ar = ar_with(Some(se), Usd::ZERO, Usd::ZERO);
        let f8959 = form_8959_lines(FilingStatus::Mfj, dec!(280500), Usd::ZERO, Some(&se));

        assert_eq!(f8959.line18, dec!(775)); // the printed, cross-footing 8959 total
        let exact_total = dec!(274.50) + dec!(499.50); // what the engine carries, in cents
        assert_eq!(round_dollar(exact_total), dec!(774)); // …which rounds to something ELSE

        let s2 = schedule_2_lines(&ar, &f8959, None).unwrap();
        assert_eq!(
            s2.line11,
            dec!(775),
            "Schedule 2 must carry the 8959's PRINTED line 18"
        );
        assert_ne!(s2.line11, round_dollar(exact_total));
    }

    /// Nothing to report ⇒ no Schedule 2 at all (1040 line 23 is zero).
    #[test]
    fn schedule_2_absent_when_no_other_taxes() {
        let ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        let f8959 = form_8959_lines(FilingStatus::Single, dec!(50000), Usd::ZERO, None);
        assert_eq!(f8959.line18, Usd::ZERO);
        assert!(schedule_2_lines(&ar, &f8959, None).is_none());
    }

    /// Schedule 3 carries the FTC and the excess-SS credit, and cross-foots to 1040 L20 / L31.
    #[test]
    fn schedule_3_carries_ftc_and_excess_social_security() {
        let ar = ar_with(None, dec!(287.40), dec!(1234.56));
        let s3 = schedule_3_lines(&ar).unwrap();
        assert_eq!(s3.line1, dec!(287)); // FTC, rounded at the line
        assert_eq!(s3.line8, dec!(287)); // → 1040 L20 (every other credit is blank)
        assert_eq!(s3.line11, dec!(1235)); // excess SS, half-up
        assert_eq!(s3.line15, dec!(1235)); // → 1040 L31

        // Either one alone still files the schedule…
        assert!(schedule_3_lines(&ar_with(None, dec!(100), Usd::ZERO)).is_some());
        assert!(schedule_3_lines(&ar_with(None, Usd::ZERO, dec!(100))).is_some());
        // …but neither means no schedule.
        assert!(schedule_3_lines(&ar_with(None, Usd::ZERO, Usd::ZERO)).is_none());
    }
}
