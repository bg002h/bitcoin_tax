//! §1401 self-employment tax on **business** crypto income (Sub-project P2-D).
//!
//! Mined crypto in a **trade or business** (not a hobby) is self-employment income (Notice 2014-21
//! A-9) → Schedule C net income → Schedule SE. This module computes the standalone §1401 SE-tax
//! figure. It is **standalone** (D5): the result is NOT folded into
//! `TaxResult::total_federal_tax_attributable`; the §164(f) one-half-SE-tax deduction is not
//! auto-coordinated into the income-tax total.
//!
//! Exactness/determinism (NFR4/NFR5): all math is exact `Decimal`; no float anywhere (every rate is
//! a `Decimal` literal). `round_cents` (ROUND_HALF_EVEN) is applied at the END of each component; the
//! intermediate `base` (× 92.35%) is itself cent-rounded, which is the intentional Schedule SE order.
use crate::conventions::{round_cents, Usd};
use crate::event::IncomeKind;
use crate::state::LedgerState;
use crate::tax::tables::{
    se_addl_medicare_threshold, TaxTable, SE_NET_EARNINGS_FACTOR, SE_RATE_ADDL_MEDICARE,
    SE_RATE_MEDICARE, SE_RATE_SS,
};
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// The computed §1401 self-employment tax for one tax year (standalone; not folded into engine B).
/// All fields are exact `Decimal` (cent-scaled where a rate was applied).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeTaxResult {
    /// §1402(a) net self-employment income BEFORE the 92.35% factor: Σ `usd_fmv` over business
    /// SE-eligible income in the year (Mining/Staking/Airdrop/Reward with `business == true`;
    /// Interest EXCLUDED per §1402(a)(2)). Here == gross (no Schedule C expenses modeled — FOLLOWUP).
    pub net_se: Usd,
    /// §1402(a): net SE earnings = `round_cents(net_se × 92.35%)` — the base the SE-tax rates apply to.
    pub base: Usd,
    /// §1401(a): Social Security portion = `12.4% × min(base, ss_wage_base − W-2 SS wages)`
    /// (W-2 SS wages assumed $0 — D4).
    pub ss: Usd,
    /// §1401(b)(1): Medicare portion = `2.9% × base` (uncapped).
    pub medicare: Usd,
    /// §1401(b)(2): Additional Medicare portion = `0.9% × max(0, base − threshold(status))`.
    pub addl: Usd,
    /// §1401: total SE tax = `ss + medicare + addl`.
    pub total: Usd,
    /// §164(f)(1): the above-the-line one-half-SE-tax deduction = `round_cents((ss + medicare) / 2)`.
    /// EXCLUDES `addl` — §164(f)(1) expressly excludes the §1401(b)(2) Additional Medicare Tax
    /// (a Form 8959 item; Schedule SE line 13 = SS + regular Medicare only). Informational.
    pub deductible_half: Usd,
}

/// §1402(a) net self-employment INCOME for `year` (BEFORE the 92.35% factor): `Σ usd_fmv` over
/// `income_recognized` where `business == true && kind != Interest && recognized_at.year() == year`.
/// **Interest is EXCLUDED** (§1402(a)(2)). Zero when there is no SE-eligible business income.
///
/// The single source of the SE-eligibility predicate: `compute_se_tax` uses it, and callers use
/// `!se_net_income(..).is_zero()` to distinguish "no SE income" from "SE income but no bundled
/// table" (the latter must emit a note, not silently drop — mirrors P2-C's m6).
pub fn se_net_income(state: &LedgerState, year: i32) -> Usd {
    state
        .income_recognized
        .iter()
        .filter(|i| i.business && i.kind != IncomeKind::Interest && i.recognized_at.year() == year)
        .map(|i| i.usd_fmv)
        .sum()
}

/// Compute the §1401 SE tax on the year's **business** crypto income, or `None` when there is no
/// SE-eligible business income (net SE == 0). `table` supplies the year-indexed `ss_wage_base`
/// (§230 SSA). The caller is responsible for the "business income present but no bundled table"
/// case (it cannot construct a `table` there — see `render_schedule_se`, no silent drop).
///
/// - `net_se = Σ usd_fmv` over `income_recognized` where `business == true && kind != Interest &&
///   recognized_at.year() == year`. **Interest is EXCLUDED** (§1402(a)(2) — investment income, not
///   SE; consistent with B-M1 treating crypto-lending interest as NII). `net_se == 0` → `None`.
/// - `base = round_cents(net_se × 92.35%)` (§1402(a)).
/// - `ss = round_cents(12.4% × min(base, ss_wage_base − W-2 SS wages))`, W-2 SS wages = $0 (D4).
/// - `medicare = round_cents(2.9% × base)`.
/// - `addl = round_cents(0.9% × max(0, base − threshold(status)))` (§1401(b)(2)).
/// - `total = ss + medicare + addl`.
/// - `deductible_half = round_cents((ss + medicare) / 2)` — EXCLUDES `addl` (§164(f)(1)).
///
/// Deterministic; exact Decimal; end-only `round_cents` per component (`base` rounding is the
/// intentional Schedule SE order).
pub fn compute_se_tax(
    state: &LedgerState,
    year: i32,
    status: FilingStatus,
    table: &TaxTable,
) -> Option<SeTaxResult> {
    let net_se = se_net_income(state, year);
    if net_se.is_zero() {
        return None;
    }

    // §1402(a): net SE earnings = net SE income × 92.35% (intermediate cent-round is the Schedule SE order).
    let base = round_cents(net_se * SE_NET_EARNINGS_FACTOR);

    // §1401(a): Social Security portion, capped at the wage base LESS W-2 SS wages (D4: W-2 = $0).
    let w2_ss_wages = Usd::ZERO;
    let ss_cap = {
        let c = table.ss_wage_base - w2_ss_wages;
        if c < Usd::ZERO {
            Usd::ZERO
        } else {
            c
        }
    };
    let ss_taxable = if base < ss_cap { base } else { ss_cap };
    let ss = round_cents(SE_RATE_SS * ss_taxable);

    // §1401(b)(1): Medicare portion (uncapped).
    let medicare = round_cents(SE_RATE_MEDICARE * base);

    // §1401(b)(2): Additional Medicare portion on net SE earnings over the status threshold.
    let over = {
        let o = base - se_addl_medicare_threshold(status);
        if o < Usd::ZERO {
            Usd::ZERO
        } else {
            o
        }
    };
    let addl = round_cents(SE_RATE_ADDL_MEDICARE * over);

    let total = ss + medicare + addl;
    // §164(f)(1): one-half of SS + regular Medicare ONLY (excludes the §1401(b)(2) addl medicare).
    let deductible_half = round_cents((ss + medicare) / dec!(2));

    Some(SeTaxResult {
        net_se,
        base,
        ss,
        medicare,
        addl,
        total,
        deductible_half,
    })
}

#[cfg(test)]
mod tests {
    //! Hand-verified golden KATs (assert EXACT). Rates + $176,100 wage base + 92.35% factor +
    //! the §164(f)(1) addl-exclusion independently confirmed against primary source (§1401/§1402/§164).
    //! PRIVACY: synthetic values only.
    use super::*;
    use crate::conventions::TaxDate;
    use crate::identity::EventId;
    use crate::state::IncomeRecord;
    use crate::tax::tables::synthetic_table;
    use time::macros::date;

    fn income(kind: IncomeKind, business: bool, fmv: Usd, d: TaxDate) -> IncomeRecord {
        IncomeRecord {
            event: EventId::decision(1),
            recognized_at: d,
            sat: 100_000_000,
            usd_fmv: fmv,
            kind,
            business,
        }
    }
    fn state_with(income: Vec<IncomeRecord>) -> LedgerState {
        LedgerState {
            income_recognized: income,
            ..Default::default()
        }
    }
    /// Synthetic table carries the real TY2025 $176,100 wage base (see `synthetic_table`).
    fn tbl() -> TaxTable {
        synthetic_table(2025)
    }

    /// Golden 1 — Single, business mining $100,000, no W-2.
    #[test]
    fn golden1_single_100k_business_mining() {
        let st = state_with(vec![income(
            IncomeKind::Mining,
            true,
            dec!(100000),
            date!(2025 - 03 - 01),
        )]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).expect("SE tax expected");
        assert_eq!(r.net_se, dec!(100000));
        assert_eq!(r.base, dec!(92350.00));
        assert_eq!(r.ss, dec!(11451.40));
        assert_eq!(r.medicare, dec!(2678.15));
        assert_eq!(r.addl, dec!(0.00));
        assert_eq!(r.total, dec!(14129.55));
        assert_eq!(r.deductible_half, dec!(7064.78));
    }

    /// [C1 lock] — $300,000 Single: exercises the wage-base cap AND the 0.9% Additional Medicare
    /// Tax, and locks `deductible_half = (ss + medicare)/2` (EXCLUDES the $693.45 addl; the wrong
    /// total/2 would give $15,282.15).
    #[test]
    fn c1_lock_single_300k_addl_medicare_and_deductible_half_excludes_addl() {
        let st = state_with(vec![income(
            IncomeKind::Mining,
            true,
            dec!(300000),
            date!(2025 - 06 - 15),
        )]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).expect("SE tax expected");
        assert_eq!(r.base, dec!(277050.00));
        assert_eq!(r.ss, dec!(21836.40)); // capped at 12.4% × 176,100
        assert_eq!(r.medicare, dec!(8034.45));
        assert_eq!(r.addl, dec!(693.45)); // 0.9% × (277,050 − 200,000)
        assert_eq!(r.total, dec!(30564.30));
        // §164(f)(1): (21,836.40 + 8,034.45)/2 = 14,935.42 — NOT (total)/2 = 15,282.15.
        assert_eq!(r.deductible_half, dec!(14935.42));
        assert_ne!(r.deductible_half, dec!(15282.15));
    }

    /// Wage-base cap — $250,000 Single: base $230,875 > $176,100 → ss pinned at 12.4% × 176,100.
    #[test]
    fn wage_base_cap_250k_single() {
        let st = state_with(vec![income(
            IncomeKind::Mining,
            true,
            dec!(250000),
            date!(2025 - 02 - 01),
        )]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).expect("SE tax expected");
        assert_eq!(r.base, dec!(230875.00));
        assert_eq!(r.ss, dec!(21836.40));
    }

    /// MFS uses the $125,000 Additional-Medicare threshold (lower than Single's $200,000).
    #[test]
    fn mfs_uses_125k_addl_medicare_threshold() {
        assert_eq!(se_addl_medicare_threshold(FilingStatus::Mfs), dec!(125000));
        let st = state_with(vec![income(
            IncomeKind::Mining,
            true,
            dec!(200000),
            date!(2025 - 04 - 01),
        )]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Mfs, &tbl()).expect("SE tax expected");
        assert_eq!(r.base, dec!(184700.00));
        assert_eq!(r.ss, dec!(21836.40)); // capped
        assert_eq!(r.medicare, dec!(5356.30));
        // 0.9% × (184,700 − 125,000) = 0.9% × 59,700 = 537.30 (Single would use 200k → 537.30 vs less).
        assert_eq!(r.addl, dec!(537.30));
        assert_eq!(r.total, dec!(27730.00));
    }

    /// [M2] business Interest is EXCLUDED from net SE (§1402(a)(2)); business Mining IS included.
    #[test]
    fn m2_business_interest_excluded_mining_included() {
        let st = state_with(vec![
            income(
                IncomeKind::Mining,
                true,
                dec!(100000),
                date!(2025 - 03 - 01),
            ),
            income(
                IncomeKind::Interest,
                true,
                dec!(50000),
                date!(2025 - 03 - 02),
            ),
        ]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).expect("SE tax expected");
        // Interest's $50,000 is NOT in net_se → identical to Golden 1.
        assert_eq!(r.net_se, dec!(100000));
        assert_eq!(r.base, dec!(92350.00));
        assert_eq!(r.total, dec!(14129.55));
    }

    /// [M3] fractional-base — mining $12,345.67 genuinely exercises `round_cents`.
    #[test]
    fn m3_fractional_base_rounds_at_cent() {
        let st = state_with(vec![income(
            IncomeKind::Mining,
            true,
            dec!(12345.67),
            date!(2025 - 05 - 05),
        )]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).expect("SE tax expected");
        assert_eq!(r.base, dec!(11401.23)); // round_cents(12,345.67 × 0.9235)
        assert_eq!(r.ss, dec!(1413.75)); // round_cents(0.124 × 11,401.23)
        assert_eq!(r.medicare, dec!(330.64)); // round_cents(0.029 × 11,401.23)
        assert_eq!(r.addl, dec!(0.00));
        assert_eq!(r.total, dec!(1744.39));
        assert_eq!(r.deductible_half, dec!(872.20)); // round_cents(1,744.39 / 2), ties-to-even
    }

    /// No business income at all → None.
    #[test]
    fn no_business_income_is_none() {
        let st = state_with(vec![income(
            IncomeKind::Mining,
            false,
            dec!(100000),
            date!(2025 - 03 - 01),
        )]);
        assert!(compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).is_none());
    }

    /// Hobby (business == false) mining is EXCLUDED (Notice 2014-21 A-9 — hobby ≠ SE).
    #[test]
    fn hobby_mining_excluded_even_with_business_interest() {
        // business=false mining + business Interest (excluded by kind) → net_se == 0 → None.
        let st = state_with(vec![
            income(
                IncomeKind::Mining,
                false,
                dec!(100000),
                date!(2025 - 03 - 01),
            ),
            income(
                IncomeKind::Interest,
                true,
                dec!(50000),
                date!(2025 - 03 - 02),
            ),
        ]);
        assert!(compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).is_none());
    }

    /// Year filter: business mining in a DIFFERENT year is excluded from this year's net SE.
    #[test]
    fn only_this_years_business_income_counts() {
        let st = state_with(vec![
            income(
                IncomeKind::Mining,
                true,
                dec!(100000),
                date!(2025 - 03 - 01),
            ),
            income(
                IncomeKind::Mining,
                true,
                dec!(999999),
                date!(2024 - 03 - 01),
            ),
        ]);
        let r = compute_se_tax(&st, 2025, FilingStatus::Single, &tbl()).expect("SE tax expected");
        assert_eq!(r.net_se, dec!(100000)); // 2024's income not counted
    }
}
