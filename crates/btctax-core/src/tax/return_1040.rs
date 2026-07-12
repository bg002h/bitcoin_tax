//! Full-return v1 **absolute 1040 assembly** (Phase 2+). This module builds the *absolute* filed return
//! from `ReturnInputs` + the projected ledger, and — the load-bearing Phase-2 piece — derives the FROZEN
//! [`TaxProfile`] the crypto-delta engine consumes.
//!
//! **The frozen seam (SPEC §5 / deep/02).** Two AGI notions coexist and must never be conflated:
//! - [`derive_tax_profile`] populates the frozen `TaxProfile` scalars from **NON-crypto line items only**.
//!   `ReturnInputs` holds no crypto (crypto lives in the ledger `state`), so the exclusion is *structural*:
//!   this function cannot see, and therefore cannot double-count, any crypto figure. The frozen engine
//!   (`compute.rs`) adds the crypto AGI delta itself (`compute.rs:339-342` `bottom_with`), so the profile
//!   must exclude it (`types.rs:34-36`).
//! - The *absolute* WITH-crypto 1040 (the filed return, added in a later P2 increment) re-combines the
//!   non-crypto lines with the ledger's crypto figures **itself**, via the shared primitives (`net_1222`,
//!   `ordinary_tax_on`, `preferential_tax`) — never by un-delta-ing `compute_tax_year`.
//!
//! Additive per SPEC §2: `compute.rs` / `types.rs` / `se.rs` stay byte-frozen; this file only reads them.
use crate::conventions::{round_dollar, Usd};
use crate::tax::return_inputs::{Owner, ReturnInputs};
use crate::tax::tables::FullReturnParams;
use crate::tax::types::{FilingStatus, TaxProfile};
use rust_decimal_macros::dec;

/// §221 student-loan-interest deduction (Sch 1 L21): `min(paid, $2,500)` phased out linearly over the
/// filing status's MAGI range (**MFS ⇒ $0**, §221(e)(2)). `magi` is the AGI **before** this deduction.
///
/// In [`derive_tax_profile`] the `magi` passed is the **non-crypto** AGI-before-L21 (the delta baseline);
/// the absolute return uses the with-crypto AGI — a deliberate, documented delta-vs-absolute divergence
/// (SPEC §6), since the frozen engine fixes the deduction at derivation time.
///
/// The IRS worksheet says "round [the ratio] to at least three places"; using the exact ratio satisfies
/// that (∞ places) and we `round_dollar` the final amount per the global half-up policy (SPEC §3.1).
pub fn student_loan_deduction(
    paid: Usd,
    magi: Usd,
    status: FilingStatus,
    params: &FullReturnParams,
) -> Usd {
    let cap = paid.min(dec!(2500));
    if cap <= Usd::ZERO {
        return Usd::ZERO;
    }
    match params.student_loan_phaseout(status) {
        None => Usd::ZERO, // MFS — no deduction
        Some((lo, hi)) => {
            if magi <= lo {
                cap
            } else if magi >= hi {
                Usd::ZERO
            } else {
                let ratio = (magi - lo) / (hi - lo);
                round_dollar(cap * (Usd::ONE - ratio))
            }
        }
    }
}

/// Derive the FROZEN [`TaxProfile`] (crypto-delta-engine input) from the **non-crypto** `ReturnInputs`
/// line items for `year`'s `params` (SPEC §5 stages 1–2, deep/02 §1 Worked Example 1).
///
/// Crypto is **excluded structurally** — `ReturnInputs` carries none; the engine adds the crypto delta on
/// top. P2 uses the **basic** standard deduction only; §63(f) aged/blind, the dependent floor, Schedule A,
/// and QBI land in P3. `magi_excluding_crypto = AGI` exactly (no §911/CFC/PFIC in the model — deep/02 C1).
pub fn derive_tax_profile(ri: &ReturnInputs, params: &FullReturnParams) -> TaxProfile {
    let status = ri.filing_status;

    // ── Income (non-crypto) ──────────────────────────────────────────────────────────────────────
    let wages: Usd = ri.w2s.iter().map(|w| w.box1_wages).sum();
    // 1040 2b taxable interest = box 1 + box 3 (Treasury); box 3 is NOT a subset of box 1.
    let taxable_int: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box1_interest + i.box3_treasury_interest)
        .sum();
    // 1040 3b ordinary dividends = Σ box 1a (which ALREADY includes box 1b qualified — "strip once").
    let ord_div: Usd = ri.div_1099.iter().map(|d| d.box1a_ordinary).sum();
    // 1040 3a qualified dividends = Σ box 1b (the preferential split ONLY — never added to income again).
    let qual_div: Usd = ri.div_1099.iter().map(|d| d.box1b_qualified).sum();
    // Sch D L13 → 1040 L7 (LT character): capital-gain distributions, box 2a. Enters AGI once, via L7.
    let cap_gain_distr: Usd = ri.div_1099.iter().map(|d| d.box2a_capgain_distr).sum();

    // Sch 1 Part I additional income (non-crypto): L1 taxable state refund + L7 Σ unemployment.
    // (L3 Schedule C and L8v digital-asset income are CRYPTO → excluded from the frozen profile.)
    let unemployment: Usd = ri.g_1099.iter().map(|g| g.box1_unemployment).sum();
    let sch1_income = ri.sch1.state_refund_taxable + unemployment;

    // Sch 1 Part II adjustments (non-crypto): L18 early-withdrawal penalty + L21 student-loan.
    // (L15 ½-SE is crypto-Schedule-C-driven → excluded here.)
    let early_wd: Usd = ri
        .int_1099
        .iter()
        .map(|i| i.box2_early_withdrawal_penalty)
        .sum();
    let income_total = wages + taxable_int + ord_div + cap_gain_distr + sch1_income;
    let agi_before_student_loan = income_total - early_wd;
    let student_loan = student_loan_deduction(
        ri.sch1.student_loan_interest_paid,
        agi_before_student_loan,
        status,
        params,
    );
    let adjustments = early_wd + student_loan;

    // ── AGI, deduction, taxable income (basic standard only in P2) ────────────────────────────────
    let agi = income_total - adjustments; // 1040 L11 (non-crypto)
    let deduction = params.std_deduction_for(status); // basic std; P3 adds aged/blind/Sch A
    let taxable_income = (agi - deduction).max(Usd::ZERO); // 1040 L15 (non-crypto)
    // Strip the preferential slice (qualified div + LT cap-gain distr) EXACTLY ONCE — the engine re-adds
    // it on top of the ordinary bottom via `other_net_capital_gain` + the QD channel (deep/02 §1.4).
    let ordinary_taxable_income = (taxable_income - qual_div - cap_gain_distr).max(Usd::ZERO);

    // ── W-2 SE/Medicare channels (two DIFFERENT aggregations — deep/02 §3.4 / C4) ─────────────────
    // §1402(b)(1) SS cap is PER-INDIVIDUAL: `w2_ss_wages` = the SE-earner's OWN box 3 + box 7 tips, NOT
    // the household sum. The SE earner is the single Schedule C owner (Taxpayer when there is no Sch C).
    let se_owner = ri
        .schedule_c
        .as_ref()
        .map(|c| c.owner)
        .unwrap_or(Owner::Taxpayer);
    let w2_ss_wages: Usd = ri
        .w2s
        .iter()
        .filter(|w| w.owner == se_owner)
        .map(|w| w.box3_ss_wages + w.box7_ss_tips)
        .sum();
    // Form 8959 Part I/II uses HOUSEHOLD-total Medicare wages (both spouses' box 5).
    let w2_medicare_wages: Usd = ri.w2s.iter().map(|w| w.box5_medicare_wages).sum();
    let schedule_c_expenses = ri
        .schedule_c
        .as_ref()
        .map(|c| c.expenses)
        .unwrap_or(Usd::ZERO);

    TaxProfile {
        filing_status: status,
        ordinary_taxable_income,
        magi_excluding_crypto: agi,
        qualified_dividends_and_other_pref_income: qual_div,
        other_net_capital_gain: cap_gain_distr,
        capital_loss_carryforward_in: ri.capital_loss_carryforward_in,
        w2_ss_wages,
        w2_medicare_wages,
        schedule_c_expenses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::return_inputs::{Form1099Div, Form1099Int, Form1099G, ScheduleCInputs, W2};
    use crate::tax::types::Carryforward;
    use std::collections::BTreeMap;

    fn ty2024_params() -> FullReturnParams {
        let mut std_deduction = BTreeMap::new();
        std_deduction.insert(FilingStatus::Single, dec!(14600));
        std_deduction.insert(FilingStatus::Mfj, dec!(29200));
        std_deduction.insert(FilingStatus::Mfs, dec!(14600));
        std_deduction.insert(FilingStatus::HoH, dec!(21900));
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
            student_loan_phaseout_unmarried: (dec!(80000), dec!(95000)),
            student_loan_phaseout_married: (dec!(165000), dec!(195000)),
        }
    }

    fn w2(owner: Owner, box1: Usd, box3: Usd, box5: Usd) -> W2 {
        W2 {
            owner,
            box1_wages: box1,
            box3_ss_wages: box3,
            box5_medicare_wages: box5,
            ..Default::default()
        }
    }

    /// deep/02 Worked Example 1 (MFJ, no crypto) — the derived `TaxProfile` cent-exact, every field.
    #[test]
    fn derive_matches_deep02_example1_to_the_cent() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2(Owner::Taxpayer, dec!(180000), dec!(168600), dec!(180000)),
                w2(Owner::Spouse, dec!(90000), dec!(90000), dec!(90000)),
            ],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(4000),
                ..Default::default()
            }],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(10000),
                box1b_qualified: dec!(8000),
                box2a_capgain_distr: dec!(3000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let p = derive_tax_profile(&ri, &ty2024_params());
        assert_eq!(p.filing_status, FilingStatus::Mfj);
        assert_eq!(p.ordinary_taxable_income, dec!(246800)); // 257,800 − 8,000 − 3,000
        assert_eq!(p.magi_excluding_crypto, dec!(287000)); // AGI
        assert_eq!(p.qualified_dividends_and_other_pref_income, dec!(8000));
        assert_eq!(p.other_net_capital_gain, dec!(3000));
        assert_eq!(p.w2_ss_wages, dec!(168600)); // SE-earner (Taxpayer) OWN box 3, NOT the 258,600 sum
        assert_eq!(p.w2_medicare_wages, dec!(270000)); // household Σ box 5
        assert_eq!(p.schedule_c_expenses, dec!(0));
        assert_eq!(p.capital_loss_carryforward_in, Carryforward::default());
        // Round-trip identity (deep/02 §1.4): taxable_income == ord_ti + qd + cap_gain_distr.
        assert_eq!(
            p.ordinary_taxable_income
                + p.qualified_dividends_and_other_pref_income
                + p.other_net_capital_gain,
            dec!(257800)
        );
    }

    /// "Strip once" — box 1a is used for the ordinary total, box 1b ONLY for the preferential split; a
    /// higher box 1b must NOT lower AGI/ordinary income (the income-side double-count bug, deep/02 §1.4).
    #[test]
    fn box1b_does_not_reduce_agi_or_double_count() {
        // Enough wage income that taxable income clears the standard deduction (so the strip is exercised,
        // not floored to zero).
        let base = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000))],
            div_1099: vec![Form1099Div {
                box1a_ordinary: dec!(10000),
                box1b_qualified: dec!(2000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut more_qual = base.clone();
        more_qual.div_1099[0].box1b_qualified = dec!(9000); // more of the SAME $10k is qualified
        let a = derive_tax_profile(&base, &ty2024_params());
        let b = derive_tax_profile(&more_qual, &ty2024_params());
        // AGI unchanged (box 1a is the income; box 1b is only a split) = 100,000 + 10,000.
        assert_eq!(a.magi_excluding_crypto, b.magi_excluding_crypto);
        assert_eq!(a.magi_excluding_crypto, dec!(110000));
        // The larger qualified slice moves MORE out of the ordinary bottom into the preferential channel.
        assert_eq!(b.qualified_dividends_and_other_pref_income, dec!(9000));
        assert!(b.ordinary_taxable_income < a.ordinary_taxable_income);
        // But the difference is exactly the moved slice ($7,000), not a double-count of AGI.
        assert_eq!(a.ordinary_taxable_income - b.ordinary_taxable_income, dec!(7000));
    }

    /// box 2a capital-gain distributions are IN AGI (via L7) AND stripped once — never double-removed.
    #[test]
    fn box2a_is_in_agi_and_stripped_once() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            div_1099: vec![Form1099Div {
                box2a_capgain_distr: dec!(3000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let p = derive_tax_profile(&ri, &ty2024_params());
        assert_eq!(p.magi_excluding_crypto, dec!(3000)); // in AGI
        assert_eq!(p.other_net_capital_gain, dec!(3000)); // re-enters via preferential channel
        assert_eq!(p.ordinary_taxable_income, Usd::ZERO); // 3,000 − std 14,600 floored, then strip
    }

    /// L1 refund + L7 unemployment raise AGI; L18 early-withdrawal lowers it (Sch 1 non-crypto lines).
    #[test]
    fn schedule_1_noncrypto_income_and_adjustments() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000))],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(5000),
                box2_early_withdrawal_penalty: dec!(1000),
                box3_treasury_interest: dec!(2000),
                ..Default::default()
            }],
            g_1099: vec![Form1099G {
                box1_unemployment: dec!(4000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut sch1 = ri.clone();
        sch1.sch1.state_refund_taxable = dec!(600);
        let p = derive_tax_profile(&sch1, &ty2024_params());
        // AGI = 100,000 + (5,000+2,000) int + 4,000 unemp + 600 refund − 1,000 early-wd = 110,600.
        assert_eq!(p.magi_excluding_crypto, dec!(110600));
    }

    /// §221 student-loan deduction: full below the range, phased in-range, zero above; MFS ⇒ $0.
    #[test]
    fn student_loan_phaseout_and_mfs_zero() {
        let params = ty2024_params();
        // Single, MAGI below $80k → full $2,500 cap.
        assert_eq!(
            student_loan_deduction(dec!(3000), dec!(60000), FilingStatus::Single, &params),
            dec!(2500)
        );
        // Single, MAGI at the $87,500 midpoint of 80k–95k → half of the capped $2,500 = $1,250.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(87500), FilingStatus::Single, &params),
            dec!(1250)
        );
        // Single, MAGI ≥ $95k → fully phased out.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(95000), FilingStatus::Single, &params),
            Usd::ZERO
        );
        // MFS → always $0 (§221(e)(2)), even below the range.
        assert_eq!(
            student_loan_deduction(dec!(2500), dec!(40000), FilingStatus::Mfs, &params),
            Usd::ZERO
        );
        // MFJ uses the higher $165k–$195k range: $170k is in-range.
        let d = student_loan_deduction(dec!(2500), dec!(170000), FilingStatus::Mfj, &params);
        assert!(d > Usd::ZERO && d < dec!(2500));
    }

    /// The derivation flows the student-loan deduction into AGI (Single with $1,000 paid, below range).
    #[test]
    fn derive_applies_student_loan_adjustment() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            w2s: vec![w2(Owner::Taxpayer, dec!(50000), dec!(50000), dec!(50000))],
            int_1099: vec![Form1099Int {
                box1_interest: dec!(1000),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut with_loan = ri.clone();
        with_loan.sch1.student_loan_interest_paid = dec!(1000);
        let p = derive_tax_profile(&with_loan, &ty2024_params());
        // AGI = 50,000 + 1,000 − 1,000 student-loan = 50,000.
        assert_eq!(p.magi_excluding_crypto, dec!(50000));
    }

    /// The SE-earner channel: with a spouse-owned Schedule C, `w2_ss_wages` tracks the SPOUSE's box 3,
    /// not the taxpayer's, while Medicare wages stay household-summed.
    #[test]
    fn se_owner_selects_ss_wages_channel() {
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![
                w2(Owner::Taxpayer, dec!(100000), dec!(100000), dec!(100000)),
                w2(Owner::Spouse, dec!(40000), dec!(40000), dec!(40000)),
            ],
            schedule_c: Some(ScheduleCInputs {
                owner: Owner::Spouse,
                expenses: dec!(2500),
                ..Default::default()
            }),
            ..Default::default()
        };
        let p = derive_tax_profile(&ri, &ty2024_params());
        assert_eq!(p.w2_ss_wages, dec!(40000)); // spouse's own box 3
        assert_eq!(p.w2_medicare_wages, dec!(140000)); // household Σ box 5
        assert_eq!(p.schedule_c_expenses, dec!(2500));
    }
}
