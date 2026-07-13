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
use crate::tax::return_1040::{AbsoluteReturn, MEDICAL_FLOOR_RATE};

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

/// The printable **Schedule 1 (Additional Income and Adjustments to Income)** line chain.
///
/// **Unmodeled lines are BLANK, not zero**: line 2a/2b (alimony), 4 (Form 4797), 5 (Schedule E —
/// unrepresentable in v1), 6 (Schedule F), most of the 8a–8z write-ins, and in Part II lines 11–14,
/// 16, 17, 19, 20, 23 and all of 24a–24z. **Line 22 is the IRS's own "Reserved for future use"** —
/// a live ReadOnly widget that must never be written.
///
/// v1's crypto ordinary income lands on **line 8v** ("Digital assets received as ordinary income not
/// reported elsewhere") when it is NOT a trade or business; business crypto goes to line 3 via
/// Schedule C instead. The two are mutually exclusive by construction, which is why both can be
/// printed without double-counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule1Lines {
    /// L1 — taxable state/local income-tax refund.
    pub line1: Usd,
    /// L3 — business income (the crypto Schedule C net).
    pub line3: Usd,
    /// L7 — unemployment compensation.
    pub line7: Usd,
    /// L8v — digital assets received as ordinary income (non-business).
    pub line8v: Usd,
    /// L9 — total other income = add **printed** 8a through 8z ⇒ `= line8v` here.
    pub line9: Usd,
    /// L10 — combine **printed** 1 through 7 and 9 → 1040 **L8**.
    pub line10: Usd,
    /// L15 — the §164(f) deductible part of self-employment tax.
    pub line15: Usd,
    /// L18 — penalty on early withdrawal of savings.
    pub line18: Usd,
    /// L21 — the §221 student-loan interest deduction (post-phase-out).
    pub line21: Usd,
    /// L26 — add **printed** 11 through 23 and 25 ⇒ `15 + 18 + 21` here → 1040 **L10**.
    pub line26: Usd,
}

/// Derive the printed Schedule 1 chain. Returns `None` when there is neither additional income nor an
/// adjustment — the schedule is then not filed, and 1040 lines 8 and 10 are zero.
pub fn schedule_1_lines(ar: &AbsoluteReturn) -> Option<Schedule1Lines> {
    let p = &ar.schedule_1;

    // Part I — additional income.
    let line1 = round_dollar(p.state_refund_1);
    let line3 = round_dollar(p.schedule_c_net_3);
    let line7 = round_dollar(p.unemployment_7);
    let line8v = round_dollar(p.crypto_ordinary_8v);
    let line9 = line8v; // 8a-8u and 8w-8z are blank
    let line10 = line1 + line3 + line7 + line9; // ★ sums the PRINTED lines

    // Part II — adjustments to income.
    let line15 = round_dollar(p.half_se_15);
    let line18 = round_dollar(p.early_withdrawal_18);
    let line21 = round_dollar(p.student_loan_21);
    let line26 = line15 + line18 + line21; // ★ sums the PRINTED lines

    if line10 <= Usd::ZERO && line26 <= Usd::ZERO {
        return None;
    }
    Some(Schedule1Lines {
        line1,
        line3,
        line7,
        line8v,
        line9,
        line10,
        line15,
        line18,
        line21,
        line26,
    })
}

/// The printable **Schedule C (Profit or Loss From Business)** line chain — the crypto trade or
/// business.
///
/// **Part II is NOT itemized.** The filer supplies a flat expense total, so the individual expense
/// lines (8 through 27a) are BLANK and only the **line 28 total** is printed. Printing a 0 into each
/// of those twenty lines would assert we considered and found no advertising, no insurance, no
/// legal fees — which is a different claim from "the filer gave us one number".
///
/// Also blank, and deliberately: line 2 (returns and allowances), line 4 and Part III (cost of goods
/// sold — mining and staking have no inventory), line 6 (other income), line 30 (the §280A home-office
/// deduction, out of scope), and Part IV (vehicle information).
///
/// **A Schedule C LOSS is refused upstream** (§465 at-risk substantiation is out of scope), so line 31
/// is always ≥ 0 and the at-risk checkboxes on lines 32a/32b are never needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleCLines {
    /// L1 — gross receipts or sales.
    pub line1: Usd,
    /// L3 — subtract line 2 (returns/allowances, blank) from line 1 ⇒ `= line1`.
    pub line3: Usd,
    /// L5 — gross profit = line 3 − line 4 (cost of goods sold, blank) ⇒ `= line3`.
    pub line5: Usd,
    /// L7 — gross income = line 5 + line 6 (other income, blank) ⇒ `= line5`.
    pub line7: Usd,
    /// L28 — total expenses (the flat total; Part II's individual lines are blank).
    pub line28: Usd,
    /// L29 — tentative profit = **printed** line 7 − **printed** line 28.
    pub line29: Usd,
    /// L31 — net profit = line 29 − line 30 (home office, blank) ⇒ `= line29`. Flows to **both**
    /// Schedule 1 line 3 and Schedule SE — one figure, two destinations.
    pub line31: Usd,
}

/// Derive the printed Schedule C chain. Returns `None` when the filer has no crypto trade or business.
pub fn schedule_c_lines(ar: &AbsoluteReturn) -> Option<ScheduleCLines> {
    let p = ar.schedule_c.as_ref()?;

    let line1 = round_dollar(p.gross_receipts_1);
    let line3 = line1; // − line 2 (returns and allowances), blank
    let line5 = line3; // − line 4 (cost of goods sold), blank — no inventory
    let line7 = line5; // + line 6 (other income), blank
    let line28 = round_dollar(p.total_expenses_28);
    let line29 = (line7 - line28).max(Usd::ZERO); // a loss refuses upstream
    let line31 = line29; // − line 30 (home office), blank

    Some(ScheduleCLines {
        line1,
        line3,
        line5,
        line7,
        line28,
        line29,
        line31,
    })
}

/// The printable **Schedule A (Itemized Deductions)** line chain.
///
/// **Every derived line is computed from the PRINTED line above it**, not from the exact-cents
/// components: line 3 is 7.5% of the *printed* line 2, line 4 subtracts the *printed* line 3, line 5e
/// caps the *printed* line 5d, and line 17 sums the *printed* subtotals. That is what a human filling
/// the paper form does, and it is why the form cross-foots.
///
/// **Unmodeled lines are BLANK, not zero** (no field here at all): line 6 (other taxes), line 8b
/// (mortgage interest not on a Form 1098) and 8c (points), line 9 (investment interest), line 15
/// (casualty and theft losses) and line 16 (other itemized deductions). Line 8d is the IRS's own
/// "Reserved for future use" — a ReadOnly widget that must never be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleALines {
    /// L1 — medical and dental expenses.
    pub line1: Usd,
    /// L2 — AGI (the floor's base).
    pub line2: Usd,
    /// L3 — the §213(a) floor: 7.5% × **printed** line 2.
    pub line3: Usd,
    /// L4 — medical allowed = max(0, printed 1 − printed 3).
    pub line4: Usd,
    /// L5a — state/local income taxes, or general sales taxes under the §164(b)(5) election.
    pub line5a: Usd,
    /// L5b — state/local real-estate taxes.
    pub line5b: Usd,
    /// L5c — state/local personal-property taxes.
    pub line5c: Usd,
    /// L5d — add **printed** 5a, 5b and 5c.
    pub line5d: Usd,
    /// L5e — the §164(b) cap: min(printed 5d, $10,000 / $5,000 MFS).
    pub line5e: Usd,
    /// L7 — add 5e and 6 (6 blank) ⇒ `= line5e`.
    pub line7: Usd,
    /// L8a — home-mortgage interest reported on Form 1098.
    pub line8a: Usd,
    /// L8e — add 8a through 8c (8b/8c blank) ⇒ `= line8a`.
    pub line8e: Usd,
    /// L10 — add 8e and 9 (9 blank) ⇒ `= line8e`.
    pub line10: Usd,
    /// L11 — gifts by cash or check.
    pub line11: Usd,
    /// L12 — gifts other than by cash or check (includes crypto donations; Form 8283 over $500).
    pub line12: Usd,
    /// L13 — carryover from a prior year.
    pub line13: Usd,
    /// L14 — add **printed** 11, 12 and 13.
    pub line14: Usd,
    /// L17 — total itemized deductions = printed 4 + 7 + 10 + 14 (15 and 16 blank) → 1040 **L12**.
    pub line17: Usd,
}

/// Derive the printed Schedule A chain.
///
/// Returns `None` unless the return actually **itemizes** — Schedule A is computed even when the
/// standard deduction wins (that is how the `max()` is taken), but it is only *filed* when it is the
/// deduction actually claimed.
///
/// Note the printed line 17 can differ by a dollar from `round_dollar(itemized_deduction)`: it sums
/// the printed subtotals, each already rounded at its own line. That is the SPEC §3.1 election, and
/// the printed figure is the one that appears on 1040 line 12.
pub fn schedule_a_lines(ar: &AbsoluteReturn) -> Option<ScheduleALines> {
    if !ar.deduction_is_itemized {
        return None;
    }
    let p = ar.schedule_a.as_ref()?;

    // Medical — the floor is taken on the PRINTED AGI, and subtracted from the PRINTED expenses.
    let line1 = round_dollar(p.medical_expenses);
    let line2 = round_dollar(p.agi);
    let line3 = round_dollar(MEDICAL_FLOOR_RATE * line2);
    let line4 = (line1 - line3).max(Usd::ZERO);

    // SALT — the cap binds the PRINTED 5d.
    let line5a = round_dollar(p.salt_5a);
    let line5b = round_dollar(p.salt_5b);
    let line5c = round_dollar(p.salt_5c);
    let line5d = line5a + line5b + line5c;
    let line5e = line5d.min(p.salt_cap);
    let line7 = line5e; // + line 6 (other taxes), unmodeled ⇒ blank

    // Interest.
    let line8a = round_dollar(p.mortgage_8a);
    let line8e = line8a; // + 8b/8c, unmodeled ⇒ blank
    let line10 = line8e; // + line 9 (investment interest), unmodeled ⇒ blank

    // Charitable — the §170(b)-limited classes are already Schedule A's own lines 11/12/13.
    let line11 = round_dollar(p.charitable_cash_11);
    let line12 = round_dollar(p.charitable_noncash_12);
    let line13 = round_dollar(p.charitable_carryover_13);
    let line14 = line11 + line12 + line13;

    // ★ The total sums the PRINTED subtotals (15 and 16 are blank).
    let line17 = line4 + line7 + line10 + line14;

    Some(ScheduleALines {
        line1,
        line2,
        line3,
        line4,
        line5a,
        line5b,
        line5c,
        line5d,
        line5e,
        line7,
        line8a,
        line8e,
        line10,
        line11,
        line12,
        line13,
        line14,
        line17,
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
            schedule_1: crate::tax::return_1040::Schedule1Parts {
                state_refund_1: z,
                schedule_c_net_3: z,
                unemployment_7: z,
                crypto_ordinary_8v: z,
                half_se_15: z,
                early_withdrawal_18: z,
                student_loan_21: z,
            },
            schedule_c: None,
            standard_deduction: z,
            itemized_deduction: None,
            schedule_a: None,
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

    /// A `ScheduleAParts` for the printed-chain tests.
    #[allow(clippy::too_many_arguments)]
    fn parts(
        medical: Usd,
        agi: Usd,
        salt_5a: Usd,
        salt_5b: Usd,
        salt_5c: Usd,
        salt_cap: Usd,
        mortgage: Usd,
        cash: Usd,
        noncash: Usd,
        carryover: Usd,
    ) -> crate::tax::return_1040::ScheduleAParts {
        use crate::tax::return_1040::{ScheduleAParts, MEDICAL_FLOOR_RATE};
        let agi = agi.max(Usd::ZERO);
        let floor = MEDICAL_FLOOR_RATE * agi;
        let salt_5d = salt_5a + salt_5b + salt_5c;
        let salt_5e = salt_5d.min(salt_cap);
        let medical_allowed = (medical - floor).max(Usd::ZERO);
        ScheduleAParts {
            medical_expenses: medical,
            agi,
            medical_floor: floor,
            medical_allowed,
            salt_5a,
            salt_5b,
            salt_5c,
            salt_5d,
            salt_5e,
            salt_cap,
            mortgage_8a: mortgage,
            charitable_cash_11: cash,
            charitable_noncash_12: noncash,
            charitable_carryover_13: carryover,
            charitable_14: cash + noncash + carryover,
            total_17: medical_allowed + salt_5e + mortgage + cash + noncash + carryover,
        }
    }

    fn ar_itemizing(p: crate::tax::return_1040::ScheduleAParts) -> AbsoluteReturn {
        let mut ar = ar_with(None, Usd::ZERO, Usd::ZERO);
        ar.schedule_a = Some(p);
        ar.deduction_is_itemized = true;
        ar.itemized_deduction = Some(p.total_17);
        ar
    }

    /// The printed Schedule A chain, end to end: the medical floor binds, the SALT cap binds, and the
    /// total sums the PRINTED subtotals.
    #[test]
    fn schedule_a_printed_chain_medical_floor_and_salt_cap() {
        // AGI 100,000 ⇒ 7.5% floor = 7,500. Medical 10,000 ⇒ 2,500 allowed.
        // SALT 8,000 + 4,000 + 500 = 12,500 ⇒ capped at 10,000.
        // Mortgage 12,000. Charitable: 1,000 cash + 2,000 noncash + 500 carryover = 3,500.
        let ar = ar_itemizing(parts(
            dec!(10000),
            dec!(100000),
            dec!(8000),
            dec!(4000),
            dec!(500),
            dec!(10000),
            dec!(12000),
            dec!(1000),
            dec!(2000),
            dec!(500),
        ));
        let l = schedule_a_lines(&ar).unwrap();

        assert_eq!(l.line1, dec!(10000));
        assert_eq!(l.line2, dec!(100000));
        assert_eq!(l.line3, dec!(7500)); // 7.5% of the PRINTED AGI
        assert_eq!(l.line4, dec!(2500));
        assert_eq!(l.line5d, dec!(12500));
        assert_eq!(l.line5e, dec!(10000)); // ★ the §164(b) cap binds
        assert_eq!(l.line7, dec!(10000));
        assert_eq!(l.line8a, dec!(12000));
        assert_eq!(l.line10, dec!(12000));
        assert_eq!(l.line11, dec!(1000));
        assert_eq!(l.line12, dec!(2000));
        assert_eq!(l.line13, dec!(500));
        assert_eq!(l.line14, dec!(3500));
        assert_eq!(l.line17, dec!(28000)); // 2,500 + 10,000 + 12,000 + 3,500
    }

    /// ★ Schedule A is COMPUTED even when the standard deduction wins (that is how the max() is
    /// taken) — but it is only FILED when it is the deduction actually claimed. Printing a Schedule A
    /// the filer did not use would be a form asserting a deduction they never took.
    #[test]
    fn schedule_a_not_filed_when_the_standard_deduction_wins() {
        let p = parts(
            Usd::ZERO,
            dec!(100000),
            dec!(1000),
            Usd::ZERO,
            Usd::ZERO,
            dec!(10000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        );
        let mut ar = ar_itemizing(p);
        ar.deduction_is_itemized = false; // the standard deduction was larger
        assert!(
            schedule_a_lines(&ar).is_none(),
            "a Schedule A the filer did not use must not be filed"
        );
    }

    /// The printed chain cross-foots, and every cell is a whole dollar — including when the inputs
    /// carry cents and a negative AGI clamps the floor to zero (so the floor can never HELP the filer).
    #[test]
    fn schedule_a_printed_lines_cross_foot() {
        for p in [
            parts(
                dec!(10000.49),
                dec!(100000.51),
                dec!(8000.50),
                dec!(4000),
                dec!(500),
                dec!(10000),
                dec!(12000),
                dec!(1000),
                dec!(2000),
                dec!(500),
            ),
            // Negative AGI: the clamp means floor = 0, so the FULL medical expense is allowed.
            parts(
                dec!(10000),
                dec!(-50000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                dec!(10000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
            ),
            // MFS: the cap is half.
            parts(
                Usd::ZERO,
                dec!(80000),
                dec!(9000),
                dec!(1000),
                Usd::ZERO,
                dec!(5000),
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
                Usd::ZERO,
            ),
        ] {
            let l = schedule_a_lines(&ar_itemizing(p)).unwrap();
            assert_eq!(
                l.line3,
                round_dollar(MEDICAL_FLOOR_RATE * l.line2),
                "L3 = 7.5% × printed L2"
            );
            assert_eq!(
                l.line4,
                (l.line1 - l.line3).max(Usd::ZERO),
                "L4 = 1 − 3, floored"
            );
            assert_eq!(
                l.line5d,
                l.line5a + l.line5b + l.line5c,
                "L5d = 5a + 5b + 5c (printed)"
            );
            assert!(l.line5e <= l.line5d, "L5e never exceeds L5d");
            assert_eq!(l.line7, l.line5e, "L7 = 5e + 6 (6 blank)");
            assert_eq!(l.line10, l.line8a, "L10 = 8e + 9, 8e = 8a (rest blank)");
            assert_eq!(
                l.line14,
                l.line11 + l.line12 + l.line13,
                "L14 = 11 + 12 + 13 (printed)"
            );
            assert_eq!(
                l.line17,
                l.line4 + l.line7 + l.line10 + l.line14,
                "L17 sums the PRINTED subtotals"
            );
            for cell in [
                l.line1, l.line2, l.line3, l.line4, l.line5a, l.line5b, l.line5c, l.line5d,
                l.line5e, l.line7, l.line8a, l.line8e, l.line10, l.line11, l.line12, l.line13,
                l.line14, l.line17,
            ] {
                assert_eq!(
                    cell.fract(),
                    Usd::ZERO,
                    "printed cells are whole dollars: {cell}"
                );
            }
        }

        // The negative-AGI case, specifically: floor = 0 ⇒ the whole $10,000 medical is allowed.
        let neg = schedule_a_lines(&ar_itemizing(parts(
            dec!(10000),
            dec!(-50000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            dec!(10000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
        )))
        .unwrap();
        assert_eq!(
            neg.line2,
            Usd::ZERO,
            "a negative AGI is clamped — the floor must never HELP"
        );
        assert_eq!(neg.line3, Usd::ZERO);
        assert_eq!(neg.line4, dec!(10000));
    }
}
