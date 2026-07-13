//! Full-return v1 **Schedule 2 other taxes** (Phase 4 task 3/5): the absolute Form 8959 (Additional
//! Medicare Tax) and Form 8960 (Net Investment Income Tax), plus the Schedule SE → Schedule 2 line 4
//! unbundle. Federal only, exact Decimal, cents carried. The PRINTED chain (`form_8959_lines`, P6)
//! rounds each line half-up and cross-foots — see [`Form8959Lines`]; it is not the cents figure.
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
use crate::conventions::{round_cents, round_dollar, Usd};
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

/// The printable **Form 8959 line chain** — whole dollars, cross-footing (SPEC §3.1).
///
/// `btctax-forms` must never do tax arithmetic: a divergence between what we COMPUTE and what we
/// PRINT is a silently wrong return, and no core KAT would catch it. So the chain is derived HERE
/// and the filler transcribes it cell-for-cell.
///
/// **This is NOT a copy of the computed [`Form8959`].** Under the SPEC §3.1 *round-all-amounts*
/// election, every printed line is `round_dollar`ed **at the line**, and a printed total **sums the
/// already-rounded lines above it** so that the filed form cross-foots. That deliberately differs
/// from rounding the exact-cents total: with Part I = $274.50 and Part II = $499.50, the printed
/// line 18 is `275 + 500 = 775`, not `round_dollar(774.00) = 774`. Each printed line is also
/// computed from the *printed* (already-rounded) line it references — line 13 is 0.9% × **printed**
/// line 12 — because that is what a human filling the paper form does. So `line13` may differ from
/// `round_dollar(se.addl)` by a dollar, and that is correct, not a bug.
///
/// Consequence, and it is intended: the printed PDF can differ from the exact-cents computed return
/// by a few dollars. The PDF is the filed document and it ties out to itself; the report is the
/// precise computation. See `report_and_pdf_may_differ_by_rounding` in FOLLOWUPS.
///
/// **Unmodeled lines are absent, not zero** — they are left BLANK on the paper form: line 2 (Form
/// 4137 unreported tips), line 3 (Form 8919 wages), and all of Part III (lines 14–17, RRTA
/// compensation) plus line 23 (RRTA withholding). That is exactly why `line4 == line1`,
/// `line18 == line7 + line13` with no line-17 term, and `line24 == line22`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form8959Lines {
    /// L1 — Σ W-2 box 5 Medicare wages and tips.
    pub line1: Usd,
    /// L4 — add lines 1–3. Lines 2/3 are unmodeled (blank) ⇒ `= line1`.
    pub line4: Usd,
    /// L5 — the filing-status threshold ($250k MFJ / $125k MFS / $200k Single, HoH, **QSS**).
    pub line5: Usd,
    /// L6 — subtract 5 from 4, floored at 0.
    pub line6: Usd,
    /// L7 — 0.9% × line 6 (of the PRINTED line 6). May differ from `round_dollar(part1_wages)`.
    pub line7: Usd,
    /// L8 — SE income from Schedule SE Part I line 6 (`se.base`); 0 when there is no SE tax.
    pub line8: Usd,
    /// L9 — the same filing-status threshold as line 5.
    pub line9: Usd,
    /// L10 — enter the amount from line 4.
    pub line10: Usd,
    /// L11 — subtract 10 from 9, floored at 0. (This is `se.rs`'s `addl_threshold`.)
    pub line11: Usd,
    /// L12 — subtract 11 from 8, floored at 0. (This is `se.rs`'s `over`.)
    pub line12: Usd,
    /// L13 — 0.9% × the PRINTED line 12. The frozen `se.rs` chain is Form 8959 Part II term for
    /// term, so this tracks `se.addl` — but rounded at the line, so it may differ by a dollar.
    pub line13: Usd,
    /// L18 — add PRINTED 7 + 13 (17 unmodeled) → Schedule 2 line 11. ★ The form cross-foots, so
    /// this is deliberately NOT `round_dollar(additional_medicare_tax)` (SPEC §3.1 / KAT-9).
    pub line18: Usd,
    /// L19 — Σ W-2 box 6 Medicare tax withheld.
    pub line19: Usd,
    /// L20 — enter the amount from line 1.
    pub line20: Usd,
    /// L21 — 1.45% × line 20 (the regular Medicare that *should* have been withheld).
    pub line21: Usd,
    /// L22 — subtract PRINTED 21 from PRINTED 19, floored at 0.
    pub line22: Usd,
    /// L24 — add 22 and 23 (23 unmodeled) → 1040 line 25c ⇒ `== line22`.
    pub line24: Usd,
}

/// Derive the printed Form 8959 line chain (SPEC §3.1: `round_dollar` at each line; totals sum the
/// already-rounded lines).
///
/// `se` is the §6017-floored Schedule SE result — an *unprinted worksheet*, so it carries cents and
/// is rounded once here, where it lands on printed line 8. `se` must have been computed with this
/// same `medicare_wages` as its `w2_medicare_wages`, since Part II's line-11 clamp is the same
/// §1401(b)(2) threshold reduction; [`crate::tax::return_1040`] threads both from one place.
pub fn form_8959_lines(
    status: FilingStatus,
    medicare_wages: Usd,
    medicare_withheld: Usd,
    se: Option<&SeTaxResult>,
) -> Form8959Lines {
    // Thresholds are statutory whole dollars; wages/withholding are inputs, rounded at first use.
    let thr = se_addl_medicare_threshold(status);
    let line1 = round_dollar(medicare_wages);
    let line4 = line1; // = 1 + 2 + 3; lines 2 and 3 are unmodeled ⇒ blank
    let line6 = (line4 - thr).max(Usd::ZERO);
    let line7 = round_dollar(SE_RATE_ADDL_MEDICARE * line6);

    let line8 = round_dollar(se.map_or(Usd::ZERO, |s| s.base)); // the SE worksheet lands here
    let line10 = line4;
    let line11 = (thr - line10).max(Usd::ZERO);
    let line12 = (line8 - line11).max(Usd::ZERO);
    let line13 = round_dollar(SE_RATE_ADDL_MEDICARE * line12);

    // ★ SPEC §3.1 / KAT-9: the printed total sums the PRINTED lines, so the form cross-foots. This
    // is deliberately NOT round_dollar(part1_wages + part2_se) — with two .50 components the two
    // differ by a dollar, and the cross-footing one is what gets filed.
    let line18 = line7 + line13;

    let line19 = round_dollar(medicare_withheld);
    let line20 = line1;
    let line21 = round_dollar(MEDICARE_EMPLOYEE_RATE * line20);
    let line22 = (line19 - line21).max(Usd::ZERO);
    let line24 = line22; // + line 23 (RRTA), unmodeled ⇒ blank

    Form8959Lines {
        line1,
        line4,
        line5: thr,
        line6,
        line7,
        line8,
        line9: thr,
        line10,
        line11,
        line12,
        line13,
        line18,
        line19,
        line20,
        line21,
        line22,
        line24,
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

    /// The deep/02 **example 2** household, as a coherent SE result: MFJ, Σ box5 $280,000 of W-2
    /// Medicare wages and $60,000 of mining. Because the wages exceed the $168,600 SS wage base the
    /// §1401(a) SS portion is fully displaced (`ss == 0`); the §1401(b)(2) `addl` is clamped with
    /// **this same** $280,000 (its threshold reduces to 0), which is the precondition
    /// `form_8959_lines` asserts.
    fn se_mining_60k_mfj_with_280k_wages() -> SeTaxResult {
        SeTaxResult {
            net_se: dec!(60000),
            base: dec!(55410.00),          // 60,000 × 92.35%
            ss: dec!(0.00),                // wages 280k ≥ the 168,600 wage base ⇒ cap fully used
            medicare: dec!(1606.89),       // 2.9% × 55,410
            addl: dec!(498.69),            // 0.9% × (55,410 − max(0, 250,000 − 280,000)=0)
            total: dec!(2105.58),          // 0 + 1,606.89 + 498.69
            deductible_half: dec!(803.44), // (0 + 1,606.89)/2, half-even
        }
    }

    /// The printed Form 8959 chain for the deep/02 example-2 household — every one of the 17 filled
    /// cells, in whole dollars. This is the cell-for-cell contract the P6 PDF filler transcribes:
    /// if any line here is wrong, the FILED paper form is wrong even when the computed tax is right.
    #[test]
    fn form_8959_lines_deep02_example2_printed_chain() {
        let se = se_mining_60k_mfj_with_280k_wages();
        let l = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));

        // Part I — wages.
        assert_eq!(l.line1, dec!(280000));
        assert_eq!(l.line4, dec!(280000)); // lines 2/3 unmodeled ⇒ line 4 == line 1
        assert_eq!(l.line5, dec!(250000)); // MFJ threshold
        assert_eq!(l.line6, dec!(30000));
        assert_eq!(l.line7, dec!(270)); // 0.9% × 30,000 = 270.00
                                        // Part II — self-employment.
        assert_eq!(l.line8, dec!(55410)); // Schedule SE line 6 (55,410.00), rounded as it lands
        assert_eq!(l.line9, dec!(250000));
        assert_eq!(l.line10, dec!(280000)); // = line 4
        assert_eq!(l.line11, Usd::ZERO); // 250,000 − 280,000, floored: the threshold is used up
        assert_eq!(l.line12, dec!(55410)); // the WHOLE SE base is over the threshold
        assert_eq!(l.line13, dec!(499)); // 0.9% × 55,410 = 498.69 → half-up 499
                                         // Part IV — the total that lands on Schedule 2 line 11.
        assert_eq!(l.line18, dec!(769)); // 270 + 499, summing the PRINTED lines
                                         // Part V — withholding reconciliation → 1040 line 25c.
        assert_eq!(l.line19, dec!(4240));
        assert_eq!(l.line20, dec!(280000)); // = line 1
        assert_eq!(l.line21, dec!(4060)); // 1.45% × 280,000 = 4,060.00
        assert_eq!(l.line22, dec!(180)); // 4,240 − 4,060
        assert_eq!(l.line24, dec!(180)); // line 23 (RRTA) unmodeled ⇒ line 24 == line 22
    }

    /// ★ **SPEC §10 KAT-9 — printed-line rounding + cross-foot.** The discriminating fixture: two
    /// `.50` components. Part I is $274.50 → printed **275**; Part II is $499.50 → printed **500**;
    /// the printed line 18 is `275 + 500 = 775`. Rounding the exact-cents TOTAL instead would give
    /// `round_dollar(774.00) = 774` — a dollar less, and a form that does not cross-foot.
    ///
    /// (SPEC §10 illustrates KAT-9 with 271.50/499.50. A Part I of exactly `x.50` is unreachable on
    /// the real form: line 6 is itself a printed whole-dollar line, so `0.9% × line6` ends in `.50`
    /// only when `line6 ≡ 500 (mod 1000)`. This fixture uses line 6 = 30,500 → 274.50, which
    /// preserves the property SPEC is pinning — two `.50` components that each round UP — exactly.)
    #[test]
    fn kat9_printed_lines_round_then_cross_foot() {
        // MFJ, Σ box5 = $280,500 ⇒ line 6 = 30,500 ⇒ Part I = 0.9% × 30,500 = $274.50.
        // SE base = $55,500 ⇒ line 12 = 55,500 ⇒ Part II = 0.9% × 55,500 = $499.50.
        let se = SeTaxResult {
            net_se: dec!(60097.46),
            base: dec!(55500.00),
            ss: dec!(0.00),
            medicare: dec!(1609.50),
            addl: dec!(499.50), // 0.9% × 55,500 — the threshold is fully used up by the wages
            total: dec!(2109.00),
            deductible_half: dec!(804.75),
        };
        let (status, wages) = (FilingStatus::Mfj, dec!(280500));
        let f = form_8959(status, wages, Usd::ZERO, Some(&se));
        let l = form_8959_lines(status, wages, Usd::ZERO, Some(&se));

        // The exact-cents computation carries the two .50 components.
        assert_eq!(f.part1_wages, dec!(274.50));
        assert_eq!(f.part2_se, dec!(499.50));
        assert_eq!(f.additional_medicare_tax, dec!(774.00));

        // Each printed line rounds half-up AT THE LINE…
        assert_eq!(l.line7, dec!(275));
        assert_eq!(l.line13, dec!(500));
        // …and the printed total sums the PRINTED lines, so the filed form cross-foots.
        assert_eq!(l.line18, dec!(775));
        assert_eq!(l.line7 + l.line13, l.line18);

        // ★ The whole point: this is NOT round_dollar of the exact total.
        assert_eq!(round_dollar(f.additional_medicare_tax), dec!(774));
        assert_ne!(l.line18, round_dollar(f.additional_medicare_tax));
    }

    /// The printed form must **cross-foot**: a human re-adding the printed column gets the printed
    /// answer, for every derived line. Non-tautological — each total is re-derived from the OTHER
    /// printed cells, never from the value under test.
    #[test]
    fn form_8959_printed_lines_cross_foot() {
        for (status, wages, withheld, se) in [
            (
                FilingStatus::Mfj,
                dec!(280000),
                dec!(4240),
                Some(se_mining_60k_mfj_with_280k_wages()),
            ),
            (FilingStatus::Single, dec!(250000.49), dec!(4000), None), // cents in, dollars out
            (FilingStatus::Qss, dec!(240000), dec!(3000), None),       // Part V floors at 0
            (FilingStatus::Mfs, dec!(50000), dec!(725), None),         // under threshold ⇒ L7 = 0
        ] {
            let l = form_8959_lines(status, wages, withheld, se.as_ref());

            assert_eq!(l.line4, l.line1, "L4 = 1 + 2 + 3, with 2/3 blank");
            assert_eq!(
                l.line6,
                (l.line4 - l.line5).max(Usd::ZERO),
                "L6 = 4 − 5, floored"
            );
            assert_eq!(
                l.line7,
                round_dollar(SE_RATE_ADDL_MEDICARE * l.line6),
                "L7 = 0.9% × 6"
            );
            assert_eq!(l.line9, l.line5, "the two threshold cells agree");
            assert_eq!(l.line10, l.line4, "L10 = line 4");
            assert_eq!(
                l.line11,
                (l.line9 - l.line10).max(Usd::ZERO),
                "L11 = 9 − 10, floored"
            );
            assert_eq!(
                l.line12,
                (l.line8 - l.line11).max(Usd::ZERO),
                "L12 = 8 − 11, floored"
            );
            assert_eq!(
                l.line13,
                round_dollar(SE_RATE_ADDL_MEDICARE * l.line12),
                "L13 = 0.9% × 12"
            );
            assert_eq!(l.line18, l.line7 + l.line13, "L18 = 7 + 13 (+17, blank)");
            assert_eq!(l.line20, l.line1, "L20 = line 1");
            assert_eq!(
                l.line21,
                round_dollar(MEDICARE_EMPLOYEE_RATE * l.line20),
                "L21 = 1.45% × 20"
            );
            assert_eq!(
                l.line22,
                (l.line19 - l.line21).max(Usd::ZERO),
                "L22 = 19 − 21, floored"
            );
            assert_eq!(l.line24, l.line22, "L24 = 22 + 23 (23 blank)");

            // Every printed cell is a whole dollar (scale may be 0; the VALUE must be integral).
            for cell in [
                l.line1, l.line4, l.line5, l.line6, l.line7, l.line8, l.line9, l.line10, l.line11,
                l.line12, l.line13, l.line18, l.line19, l.line20, l.line21, l.line22, l.line24,
            ] {
                assert_eq!(
                    cell.fract(),
                    Usd::ZERO,
                    "printed cells are whole dollars: {cell}"
                );
            }
        }
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
