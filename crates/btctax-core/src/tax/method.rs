//! Absolute-return tax method (full-return v1, Phase 0): the IRS **Tax Table** ($50-bin midpoint) vs
//! **Tax Computation Worksheet** selection, and the **Qualified Dividends & Capital Gain Tax Worksheet**
//! (`qdcgt_line16`) — producing 1040 **line 16** at whole dollars.
//!
//! **Distinct from the crypto-DELTA path.** `compute.rs` computes an exact cent-precise marginal *delta*;
//! this module computes the ABSOLUTE filed-return tax with **`round_dollar` (IRS half-up)** and the binned
//! Tax Table. It REUSES the frozen primitives ([`ordinary_tax_on`], [`preferential_tax`]) unchanged — no
//! edit to the delta path (SPEC_full_return §2/§3.1, deep/01 + Fable F2).
//!
//! **No per-year Tax-Table data** (F2 REFINEMENT 2): the bin structure is a *year-independent* rule; the
//! official value is `round_dollar(ordinary_tax_on(schedule, bin_midpoint))` on the existing per-year
//! schedule. The one per-year contingency — that every sub-$100k bracket edge lands on a $25 multiple (a
//! $50-bin boundary or its exact midpoint) — is a checked invariant ([`assert_edges_binnable`], spec §8).
use crate::conventions::{round_dollar, Usd};
use crate::tax::compute::{ordinary_tax_on, preferential_tax};
use crate::tax::tables::{LtcgBreakpoints, OrdinarySchedule};
use rust_decimal_macros::dec;

/// The IRS Tax Table applies below this taxable income; at/above it the Tax Computation Worksheet (the
/// exact marginal formula) applies (2024 i1040 p.33). Inclusive at $100,000 → TCW.
pub const TAX_TABLE_CEILING: Usd = dec!(100_000);

/// The IRS Tax-Table **bin midpoint** for a whole-dollar taxable income `< $100,000` (2024 i1040 pp.64–75,
/// verified Fable F2 §2): special sub-$25 bins `[0,5)→2.5, [5,15)→10, [15,25)→20`; `$25` bins from `$25`
/// to `$3,000` (midpoint = lower + 12.5); `$50` bins from `$3,000` to `$100,000` (midpoint = lower + 25).
/// The IRS computes each printed cell as `round_dollar(tax(midpoint))`.
fn bin_midpoint(ti: Usd) -> Usd {
    debug_assert!(ti >= Usd::ZERO && ti < TAX_TABLE_CEILING);
    if ti < dec!(5) {
        return dec!(2.5);
    }
    if ti < dec!(15) {
        return dec!(10);
    }
    if ti < dec!(25) {
        return dec!(20);
    }
    let width = if ti < dec!(3000) { dec!(25) } else { dec!(50) };
    let lower = (ti / width).floor() * width;
    lower + width / dec!(2)
}

/// Tax on a single amount as the QDCGT worksheet's lines 22/24 take it: **Tax Table** (whole-dollar) when
/// `amt < $100,000`, else the **Tax Computation Worksheet** value kept at **cents** (rounded to whole
/// dollars only once, at line 16 — spec §3.1 "carry cents, round once"). Both amounts in a single worksheet
/// choose independently (Fable F2 §3).
fn worksheet_tax(schedule: &OrdinarySchedule, amt: Usd) -> Usd {
    let amt = amt.max(Usd::ZERO);
    if amt < TAX_TABLE_CEILING {
        // Tax Table: exact tax at the bin midpoint, rounded HALF-UP to whole dollars (the printed cell).
        round_dollar(ordinary_tax_on(schedule, bin_midpoint(amt)))
    } else {
        // Tax Computation Worksheet = the exact marginal formula (`ordinary_tax_on`), cents carried.
        ordinary_tax_on(schedule, amt)
    }
}

/// Regular tax on ordinary-only taxable income (no qualified dividends / net LTCG): 1040 line 16 directly,
/// whole dollars.
pub fn regular_tax(schedule: &OrdinarySchedule, taxable_income: Usd) -> Usd {
    round_dollar(worksheet_tax(schedule, taxable_income))
}

/// **Qualified Dividends & Capital Gain Tax Worksheet → 1040 line 16** (whole dollars).
///
/// `taxable_income` = 1040 line 15 (L1); `qual_div` = 1040 line 3a (L2); `net_ltcg` = the §1(h)
/// preferential net capital gain (= `min(Sch D 15,16)`, ≥0; L3). Reuses [`preferential_tax`] for the
/// 0/15/20% split (worksheet lines 6–21) and [`worksheet_tax`] for the ordinary pieces (L22/L24).
///
/// Two locked subtleties (Fable F2): **(F-A)** the preferential amount is CAPPED at `min(L1, qd+ltcg)`
/// (worksheet L10) — an uncapped pass overstates line 16 when preferential income exceeds taxable income;
/// **(F-B)** `line16 = round_dollar(min(L23, L24))` and the `min` is LOAD-BEARING (same-$50-bin cases make
/// it bind).
pub fn qdcgt_line16(
    schedule: &OrdinarySchedule,
    bp: &LtcgBreakpoints,
    taxable_income: Usd,
    qual_div: Usd,
    net_ltcg: Usd,
) -> Usd {
    let z = Usd::ZERO;
    let ti = taxable_income.max(z);
    let pref_full = (qual_div.max(z)) + (net_ltcg.max(z)); // L4 = L2 + L3
    let bottom = (ti - pref_full).max(z); // L5 = max(0, L1 − L4)
    let pref = pref_full.min(ti); // L10 cap = min(L1, L4)  ★ Fable F-A
    // Worksheet lines 6–21: the 0/15/20% split of `pref` stacked on `bottom` (`bottom + pref = L1`).
    let split = preferential_tax(bp, bottom, pref);
    let l23 = worksheet_tax(schedule, bottom) + split.tax; // L22 + (L18 + L21)
    let l24 = worksheet_tax(schedule, ti); // tax on all taxable income
    round_dollar(l23.min(l24)) // L25 → line 16  ★ Fable F-B (min binds)
}

/// **Checked invariant (spec §8, plan-review C1):** every ordinary bracket edge `< $100,000` must be a
/// multiple of `$25` — i.e. a `$50`-bin boundary OR its exact midpoint (a midpoint edge still reproduces
/// the printed cell because the IRS taxes at the midpoint and the marginal formula is continuous at an
/// edge). deep/01's stricter "no interior edge" form was TY2024-only. Returns the first offending edge, if
/// any; callers (adapter tests) assert `None` for every bundled year.
pub fn first_unbinnable_edge(schedule: &OrdinarySchedule) -> Option<Usd> {
    schedule
        .brackets
        .iter()
        .map(|b| b.lower)
        .filter(|&e| e > Usd::ZERO && e < TAX_TABLE_CEILING)
        .find(|&e| (e % dec!(25)) != Usd::ZERO)
}

/// Panicking convenience wrapper for [`first_unbinnable_edge`] (use in a `TaxTable` construction test).
pub fn assert_edges_binnable(schedule: &OrdinarySchedule, ctx: &str) {
    if let Some(e) = first_unbinnable_edge(schedule) {
        panic!("bracket edge {e} < $100k is not a $25 multiple (Tax-Table unbinnable): {ctx}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::OrdinaryBracket;

    // Real TY2024 schedules (Rev. Proc. 2023-34), constructed inline so Phase-0 tests need no adapter crate.
    fn single_2024() -> OrdinarySchedule {
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket { lower: dec!(0), rate: dec!(0.10) },
                OrdinaryBracket { lower: dec!(11600), rate: dec!(0.12) },
                OrdinaryBracket { lower: dec!(47150), rate: dec!(0.22) },
                OrdinaryBracket { lower: dec!(100525), rate: dec!(0.24) },
                OrdinaryBracket { lower: dec!(191950), rate: dec!(0.32) },
                OrdinaryBracket { lower: dec!(243725), rate: dec!(0.35) },
                OrdinaryBracket { lower: dec!(609350), rate: dec!(0.37) },
            ],
        }
    }
    fn mfj_2024() -> OrdinarySchedule {
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket { lower: dec!(0), rate: dec!(0.10) },
                OrdinaryBracket { lower: dec!(23200), rate: dec!(0.12) },
                OrdinaryBracket { lower: dec!(94300), rate: dec!(0.22) },
                OrdinaryBracket { lower: dec!(201050), rate: dec!(0.24) },
                OrdinaryBracket { lower: dec!(383900), rate: dec!(0.32) },
                OrdinaryBracket { lower: dec!(487450), rate: dec!(0.35) },
                OrdinaryBracket { lower: dec!(731200), rate: dec!(0.37) },
            ],
        }
    }
    fn bp_single_2024() -> LtcgBreakpoints {
        LtcgBreakpoints { max_zero: dec!(47025), max_fifteen: dec!(518900) }
    }
    fn bp_mfj_2024() -> LtcgBreakpoints {
        LtcgBreakpoints { max_zero: dec!(94050), max_fifteen: dec!(583750) }
    }

    #[test]
    fn bin_midpoints_match_irs_structure() {
        assert_eq!(bin_midpoint(dec!(0)), dec!(2.5));
        assert_eq!(bin_midpoint(dec!(4)), dec!(2.5));
        assert_eq!(bin_midpoint(dec!(5)), dec!(10));
        assert_eq!(bin_midpoint(dec!(14)), dec!(10));
        assert_eq!(bin_midpoint(dec!(15)), dec!(20));
        assert_eq!(bin_midpoint(dec!(24)), dec!(20));
        assert_eq!(bin_midpoint(dec!(25)), dec!(37.5)); // first $25 bin [25,50)
        assert_eq!(bin_midpoint(dec!(49)), dec!(37.5));
        assert_eq!(bin_midpoint(dec!(2975)), dec!(2987.5)); // last $25 bin
        assert_eq!(bin_midpoint(dec!(3000)), dec!(3025)); // first $50 bin [3000,3050)
        assert_eq!(bin_midpoint(dec!(58000)), dec!(58025));
        assert_eq!(bin_midpoint(dec!(58049)), dec!(58025));
        assert_eq!(bin_midpoint(dec!(99999)), dec!(99975)); // last bin [99950,100000)
    }

    /// Sampled printed Tax-Table cells reproduce exactly (Fable F2 §4 / deep/01).
    #[test]
    fn tax_table_cells_reproduce() {
        let s = single_2024();
        let m = mfj_2024();
        assert_eq!(worksheet_tax(&s, dec!(58000)), dec!(7819)); // 7,818.50 → half-up
        assert_eq!(worksheet_tax(&s, dec!(60000)), dec!(8259)); // 8,258.50 → half-up
        assert_eq!(worksheet_tax(&m, dec!(79000)), dec!(9019));
        assert_eq!(worksheet_tax(&m, dec!(85000)), dec!(9739));
        // $100k boundary is inclusive → TCW (exact formula, cents kept).
        assert_eq!(worksheet_tax(&s, dec!(100000)), dec!(17053.00));
        assert_eq!(worksheet_tax(&s, dec!(120000)), dec!(21842.50));
    }

    /// deep/01 worked example (a): MFJ TI 85,000, QD 6,000 → Tax-Table path, line 16 = 9,019.
    #[test]
    fn qdcgt_example_a_mfj_table() {
        let l16 = qdcgt_line16(&mfj_2024(), &bp_mfj_2024(), dec!(85000), dec!(6000), dec!(0));
        assert_eq!(l16, dec!(9019));
    }

    /// deep/01 worked example (b): Single TI 120,000, LTCG 20,000 → TCW path, line 16 = 20,053.
    #[test]
    fn qdcgt_example_b_single_tcw() {
        let l16 = qdcgt_line16(&single_2024(), &bp_single_2024(), dec!(120000), dec!(0), dec!(20000));
        assert_eq!(l16, dec!(20053));
    }

    /// deep/01 worked example (c): Single TI 60,000, QD 2,000, net-loss year (LTCG 0) → line 16 = 8,119.
    #[test]
    fn qdcgt_example_c_single_loss_year() {
        let l16 = qdcgt_line16(&single_2024(), &bp_single_2024(), dec!(60000), dec!(2000), dec!(0));
        assert_eq!(l16, dec!(8119));
    }

    /// KAT-1 (Fable F-A pref cap): pref > TI must not overstate. TI 35,400 / QD 50,000 ⇒ line 16 = $0.
    /// Without the `min(L1, qd+ltcg)` cap the worksheet would produce $446.
    #[test]
    fn kat1_pref_cap_pref_exceeds_ti() {
        let l16 = qdcgt_line16(&single_2024(), &bp_single_2024(), dec!(35400), dec!(50000), dec!(0));
        assert_eq!(l16, dec!(0));
    }

    /// KAT-2 (Fable F-B binding min): same-$50-bin case makes `min(L23, L24)` bind. L5=58,000, QD=10,
    /// TI=58,010 (same bin) ⇒ line 16 = 7,819 (via L24), NOT 7,821 (L23 rounded).
    #[test]
    fn kat2_binding_min_same_bin() {
        let l16 = qdcgt_line16(&single_2024(), &bp_single_2024(), dec!(58010), dec!(10), dec!(0));
        assert_eq!(l16, dec!(7819));
    }

    /// KAT-3 (plan-review C1): the real TY2024 schedules are binnable (every sub-$100k edge ≡ 0 mod 25);
    /// and a deliberately-unbinnable edge (mod-50 midpoint that is NOT mod-25) is caught. Also a
    /// midpoint-edge cell still reproduces (an edge AT a $50-bin midpoint).
    #[test]
    fn kat3_edge_binnability_and_midpoint_edge() {
        assert_eq!(first_unbinnable_edge(&single_2024()), None);
        assert_eq!(first_unbinnable_edge(&mfj_2024()), None);
        // An edge at 12,340 (≡ 15 mod 25) is unbinnable — the assertion must catch it.
        let bad = OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket { lower: dec!(0), rate: dec!(0.10) },
                OrdinaryBracket { lower: dec!(12340), rate: dec!(0.12) },
            ],
        };
        assert_eq!(first_unbinnable_edge(&bad), Some(dec!(12340)));
        // Midpoint-edge reproduction: a schedule whose 12% edge is 11,925 (= midpoint of the [11,900,11,950)
        // bin, ≡ 25 mod 50, binnable) still yields a well-defined printed cell.
        let mid = OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket { lower: dec!(0), rate: dec!(0.10) },
                OrdinaryBracket { lower: dec!(11925), rate: dec!(0.12) },
            ],
        };
        assert_eq!(first_unbinnable_edge(&mid), None);
        // tax at the bin midpoint 11,925: 10%×11,925 = 1,192.50 → half-up 1,193 (well-defined).
        assert_eq!(worksheet_tax(&mid, dec!(11925)), dec!(1193));
    }

    /// `regular_tax` (ordinary-only, no preferential income) = 1040 line 16 whole dollars, via the Tax
    /// Table (<$100k) or the TCW (≥$100k, rounded half-up).
    #[test]
    fn regular_tax_table_and_tcw() {
        let s = single_2024();
        assert_eq!(regular_tax(&s, dec!(58000)), dec!(7819)); // Tax Table
        assert_eq!(regular_tax(&s, dec!(120000)), dec!(21843)); // TCW 21,842.50 → half-up 21,843
        assert_eq!(regular_tax(&s, dec!(0)), dec!(0));
    }
}
