//! Full-return v1 **Schedule 2 other taxes** (Phase 4 task 3/5): the absolute Form 8959 (Additional
//! Medicare Tax) and Form 8960 (Net Investment Income Tax), plus the Schedule SE → Schedule 2 line 4
//! unbundle. Federal only, exact Decimal, cents carried (printed lines round at fill time, P6).
//!
//! **Two hard subtleties (deep/02 C4/C5):**
//! - The §1401(b)(2) **0.9% Additional Medicare Tax is UNBUNDLED** from Schedule 2 line 4 (which carries
//!   §1401(a) SS + §1401(b)(1) regular Medicare ONLY) and routed to Form 8959 Part II — bundling it would
//!   double-count against the 8959 (deep/02 C5).
//! - The absolute Form 8960 **NII is rebuilt from the return's own line items** (full 1040 3b dividends,
//!   2b interest, §1211-limited L7, crypto lending interest), NOT read from the frozen delta engine's
//!   `nii_with` — which is a delta-oriented approximation using QUALIFIED dividends only (deep/02 C2). This
//!   is the §6 divergence: the absolute Form 8960 is the correct filed figure; the delta is the crypto
//!   attribution.
use crate::conventions::{round_cents, Usd};
use crate::tax::se::SeTaxResult;
use crate::tax::tables::{
    niit_threshold, se_addl_medicare_threshold, NIIT_RATE, SE_RATE_ADDL_MEDICARE,
};
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// §3101(b)(1) employee Medicare (HI) tax rate (1.45%) — Form 8959 Part V line 20 (the regular Medicare
/// that should have been withheld on Medicare wages). Statutory; distinct from the 2.9% combined SE rate.
const MEDICARE_EMPLOYEE_RATE: Usd = dec!(0.0145);

/// Schedule 2 **line 4** self-employment tax = §1401(a) Social Security + §1401(b)(1) regular Medicare
/// **ONLY**. The §1401(b)(2) 0.9% Additional Medicare Tax is EXCLUDED here — it routes to Form 8959 Part
/// II, and bundling it into line 4 would double-count (deep/02 C5). Zero when there is no SE tax (below the
/// §6017 floor / no business income).
pub fn sch2_line4_se(se: Option<&SeTaxResult>) -> Usd {
    se.map_or(Usd::ZERO, |s| s.ss + s.medicare)
}

/// Form 8959 — Additional Medicare Tax (§1401(b)(2) / §3101(b)(2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form8959 {
    /// Part I — on wages: `0.9% × max(0, Σ box5 Medicare wages − threshold)`.
    pub part1_wages: Usd,
    /// Part II — on self-employment income: the §1401(b)(2) `se.addl` (0 when there is no SE tax). The
    /// Form 8959 Part II inner clamp (the threshold reduced by Σ box5, floored at 0) is ALREADY applied by
    /// `se.rs` (`compute_se_tax`'s `addl`), so this is a direct read — the same value the standalone SE
    /// report uses (the reduce-to-delta anchor: with non-crypto inputs 0, this is the whole 8959).
    pub part2_se: Usd,
    /// Form 8959 **line 18** = Part I + Part II → Schedule 2 line 11.
    pub additional_medicare_tax: Usd,
    /// Part V — Additional Medicare Tax **withholding** = `max(0, Σ box6 − 1.45% × Σ box5)` → 1040 line 25c.
    pub part5_withholding: Usd,
}

/// Compute Form 8959. `medicare_wages` = Σ W-2 box 5 (household total); `medicare_withheld` = Σ box 6;
/// `se` = the §6017-floored Schedule SE result (its `addl` is Part II).
pub fn form_8959(
    status: FilingStatus,
    medicare_wages: Usd,
    medicare_withheld: Usd,
    se: Option<&SeTaxResult>,
) -> Form8959 {
    let thr = se_addl_medicare_threshold(status);
    let part1 = round_cents(SE_RATE_ADDL_MEDICARE * (medicare_wages - thr).max(Usd::ZERO));
    let part2 = se.map_or(Usd::ZERO, |s| s.addl);
    let regular_medicare = round_cents(MEDICARE_EMPLOYEE_RATE * medicare_wages);
    let part5 = (medicare_withheld - regular_medicare).max(Usd::ZERO);
    Form8959 {
        part1_wages: part1,
        part2_se: part2,
        additional_medicare_tax: part1 + part2,
        part5_withholding: part5,
    }
}

/// Form 8960 — Net Investment Income Tax (§1411), the ABSOLUTE figure (rebuilt from line items).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form8960 {
    /// Line 8 — net investment income = 2b interest + 3b dividends + §1211-limited L7 + crypto lending
    /// interest. May be reduced by a net capital LOSS (the §1211-limited amount, §1.1411-4(d)).
    pub nii: Usd,
    /// Line 13 — MAGI = AGI (fail-closed: no §911/CFC/PFIC add-backs are modeled).
    pub magi: Usd,
    /// Line 17 = `3.8% × max(0, min(max(0, NII), max(0, MAGI − threshold)))` → Schedule 2 line 12.
    pub tax: Usd,
}

/// Compute the absolute Form 8960. `net_capital_gain` = the 1040 L7 amount (§1211-limited; may be negative
/// in a loss year — it REDUCES NII by the limited loss, §1.1411-4(d) Example 1); `crypto_lending_interest`
/// = Σ non-business crypto Interest (the L7/L8v NII modification, R3-M5, since it is not on 1040 2b).
/// Schedule C business income is EXCLUDED (§1411(c)(6) active business), as are hobby mining/staking/reward
/// (non-investment "other income"). MAGI = AGI (fail-closed).
pub fn form_8960(
    status: FilingStatus,
    taxable_interest: Usd,
    ordinary_dividends: Usd,
    net_capital_gain: Usd,
    crypto_lending_interest: Usd,
    agi: Usd,
) -> Form8960 {
    let nii = taxable_interest + ordinary_dividends + net_capital_gain + crypto_lending_interest;
    let magi = agi;
    let thr = niit_threshold(status);
    let over = (magi - thr).max(Usd::ZERO);
    // 3.8% × max(0, min(max(0, NII), max(0, MAGI − thr))) — mirrors compute.rs's frozen `niit` closure.
    let base = nii.max(Usd::ZERO).min(over);
    Form8960 {
        nii,
        magi,
        tax: round_cents(NIIT_RATE * base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-built SE result (the se.rs `c1_lock` $300k Single golden) so the unbundle is discriminating
    /// (addl > 0). Fields are exactly `compute_se_tax`'s output for that fixture.
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

    /// KAT-6 — the §1401(b)(2) 0.9% is UNBUNDLED: Schedule 2 line 4 = SS + regular Medicare only (NOT the
    /// total), and the 0.9% shows up as Form 8959 Part II (`se.addl`) instead. Bundling would double-count.
    #[test]
    fn kat6_sch2_l4_unbundles_the_addl_medicare() {
        let se = se_300k_single();
        assert_eq!(sch2_line4_se(Some(&se)), dec!(29870.85)); // 21,836.40 + 8,034.45 (NOT 30,564.30)
        assert_ne!(sch2_line4_se(Some(&se)), se.total);
        let f = form_8959(FilingStatus::Single, Usd::ZERO, Usd::ZERO, Some(&se));
        assert_eq!(f.part2_se, dec!(693.45)); // the unbundled 0.9% lands here
        assert_eq!(f.additional_medicare_tax, dec!(693.45)); // Part I 0 (no wages) + Part II
                                                             // No SE at all → line 4 is 0.
        assert_eq!(sch2_line4_se(None), Usd::ZERO);
    }

    /// C1 (Fable r1): a **Qualifying Surviving Spouse** uses the $200,000 Additional-Medicare threshold —
    /// NOT the $250,000 joint amount. A QSS is not a joint return (§1401(b)(2)(C) "any other case"; the
    /// 2024 Form 8959 chart lists "single, head of household, or qualifying surviving spouse — $200,000").
    #[test]
    fn form_8959_qss_uses_200k_threshold_not_250k() {
        // Σ box5 $240,000 → 0.9% × (240,000 − 200,000) = $360 (would be $0 under the wrong $250k threshold).
        let f = form_8959(FilingStatus::Qss, dec!(240000), Usd::ZERO, None);
        assert_eq!(f.part1_wages, dec!(360.00));
    }

    /// Form 8959 Part I — 0.9% on Medicare wages over the filing-status threshold; zero at/below it.
    #[test]
    fn form_8959_part1_wages_over_threshold() {
        // Single threshold $200,000; Σ box5 $250,000 → 0.9% × 50,000 = $450.
        let over = form_8959(FilingStatus::Single, dec!(250000), Usd::ZERO, None);
        assert_eq!(over.part1_wages, dec!(450.00));
        assert_eq!(over.additional_medicare_tax, dec!(450.00)); // no SE
                                                                // At/below the threshold → Part I is 0.
        let under = form_8959(FilingStatus::Single, dec!(200000), Usd::ZERO, None);
        assert_eq!(under.part1_wages, Usd::ZERO);
        // MFJ uses $250,000; $260,000 → 0.9% × 10,000 = $90.
        let mfj = form_8959(FilingStatus::Mfj, dec!(260000), Usd::ZERO, None);
        assert_eq!(mfj.part1_wages, dec!(90.00));
    }

    /// Form 8959 Part I + Part II compose into line 18 (Sch 2 L11): wages-over threshold AND SE addl.
    #[test]
    fn form_8959_part1_and_part2_compose() {
        let se = se_300k_single();
        // Single, Σ box5 $210,000 → Part I 0.9% × 10,000 = $90; Part II = se.addl $693.45.
        let f = form_8959(FilingStatus::Single, dec!(210000), Usd::ZERO, Some(&se));
        assert_eq!(f.part1_wages, dec!(90.00));
        assert_eq!(f.part2_se, dec!(693.45));
        assert_eq!(f.additional_medicare_tax, dec!(783.45)); // 90 + 693.45
    }

    /// Form 8959 Part V — Additional-Medicare withholding = max(0, Σ box6 − 1.45% × Σ box5) → 1040 25c.
    #[test]
    fn form_8959_part5_withholding_reconciliation() {
        // Σ box5 $250,000 → regular 1.45% × 250,000 = $3,625; box6 $4,000 → excess $375 withheld.
        let f = form_8959(FilingStatus::Single, dec!(250000), dec!(4000), None);
        assert_eq!(f.part5_withholding, dec!(375.00));
        // box6 below the 1.45% regular amount → no excess withholding (floored at 0, never negative).
        let none = form_8959(FilingStatus::Single, dec!(250000), dec!(3000), None);
        assert_eq!(none.part5_withholding, Usd::ZERO);
    }

    /// Form 8960 — NII arm binds (NII < MAGI − threshold): tax = 3.8% × NII.
    #[test]
    fn form_8960_nii_binding() {
        // interest 5,000 + dividends 10,000 + L7 20,000 + crypto lending 2,000 = NII 37,000.
        // MAGI 300,000 > 200,000 → over 100,000; base = min(37,000, 100,000) = 37,000.
        let f = form_8960(
            FilingStatus::Single,
            dec!(5000),
            dec!(10000),
            dec!(20000),
            dec!(2000),
            dec!(300000),
        );
        assert_eq!(f.nii, dec!(37000));
        assert_eq!(f.tax, dec!(1406.00)); // 3.8% × 37,000
    }

    /// Form 8960 — MAGI arm binds (MAGI − threshold < NII): tax = 3.8% × (MAGI − threshold).
    #[test]
    fn form_8960_magi_binding() {
        // NII 37,000 but MAGI 210,000 → over 10,000; base = min(37,000, 10,000) = 10,000.
        let f = form_8960(
            FilingStatus::Single,
            dec!(5000),
            dec!(10000),
            dec!(20000),
            dec!(2000),
            dec!(210000),
        );
        assert_eq!(f.tax, dec!(380.00)); // 3.8% × 10,000
    }

    /// Form 8960 — below the §1411(b) MAGI threshold ⇒ no NIIT even with large investment income.
    #[test]
    fn form_8960_below_threshold_is_zero() {
        let f = form_8960(
            FilingStatus::Single,
            dec!(50000),
            dec!(50000),
            dec!(50000),
            Usd::ZERO,
            dec!(150000),
        );
        assert_eq!(f.tax, Usd::ZERO);
    }

    /// Form 8960 — a §1211-limited net capital LOSS reduces NII (§1.1411-4(d)); a NII that goes negative is
    /// floored to a $0 base (D2 — never a refundable NIIT).
    #[test]
    fn form_8960_capital_loss_reduces_nii_and_floors_at_zero() {
        // interest 5,000, L7 −3,000 → NII 2,000; over 100,000; base 2,000 → tax 3.8% × 2,000 = $76.
        let f = form_8960(
            FilingStatus::Single,
            dec!(5000),
            Usd::ZERO,
            dec!(-3000),
            Usd::ZERO,
            dec!(300000),
        );
        assert_eq!(f.nii, dec!(2000));
        assert_eq!(f.tax, dec!(76.00));
        // interest 5,000, L7 −10,000 → NII −5,000 → base max(0, −5,000) = 0 → no NIIT.
        let neg = form_8960(
            FilingStatus::Single,
            dec!(5000),
            Usd::ZERO,
            dec!(-10000),
            Usd::ZERO,
            dec!(300000),
        );
        assert_eq!(neg.nii, dec!(-5000));
        assert_eq!(neg.tax, Usd::ZERO);
    }
}
