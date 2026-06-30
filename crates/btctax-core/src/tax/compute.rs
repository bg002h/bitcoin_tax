//! Rate-application primitives (Sub-project B, Task 3): the exact-`Decimal` arithmetic core.
//!
//! Two pure functions:
//! - [`ordinary_tax_on`] — progressive marginal-bracket tax on ordinary taxable income.
//! - [`preferential_tax`] — §1(h) 0/15/20% preferential stacking (long-term gain + qualified
//!   dividends sit ON TOP of ordinary income for breakpoint placement), returning a [`PrefSplit`].
//!
//! **Exactness/determinism (NFR4/NFR5):** all math is `Decimal`; there is **no float** anywhere
//! (every rate is a `Decimal` literal). This is the **exact marginal-bracket formula method at cent
//! precision** — NOT the IRS binned Tax Tables and NOT whole-dollar rounding — with `ROUND_HALF_EVEN`
//! to cents applied at the END only (the project's canonical `round_cents`).
use crate::conventions::{round_cents, Usd};
use crate::tax::tables::{LtcgBreakpoints, OrdinarySchedule};

/// Exact marginal-bracket tax on `taxable` (≥ 0). Sums (min(taxable, next_lower) − lower) × rate over each
/// bracket the income reaches; the open-ended top bracket has no upper bound. ROUND_HALF_EVEN to cents at
/// the END only (NFR5). NOT the IRS binned Tax Tables and NOT whole-dollar rounding — the exact formula
/// method at cent precision (deliberate determinism/exactness choice).
pub fn ordinary_tax_on(schedule: &OrdinarySchedule, taxable: Usd) -> Usd {
    if taxable <= Usd::ZERO {
        return Usd::ZERO;
    }
    let b = &schedule.brackets;
    let mut tax = Usd::ZERO;
    for (i, br) in b.iter().enumerate() {
        if taxable <= br.lower {
            break;
        }
        let upper = b.get(i + 1).map(|n| n.lower).unwrap_or(taxable); // open-ended top
        let span_top = if taxable < upper { taxable } else { upper };
        tax += (span_top - br.lower) * br.rate;
    }
    round_cents(tax)
}

/// The §1(h) preferential-rate split: how many preferential dollars land in each rate zone, plus the tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefSplit {
    pub at_0: Usd,
    pub at_15: Usd,
    pub at_20: Usd,
    pub tax: Usd,
}

/// §1(h) stacking: preferential income `pref` (= QD + net LT gain) sits ON TOP of `bottom` (ordinary
/// taxable income incl. net ST gain). Breakpoints are compared against TOTAL taxable income (bottom+pref);
/// ordinary income fills the bottom of the stack first. Exact Decimal; ROUND_HALF_EVEN at the end.
pub fn preferential_tax(bp: &LtcgBreakpoints, bottom: Usd, pref: Usd) -> PrefSplit {
    let z = Usd::ZERO;
    if pref <= z {
        return PrefSplit {
            at_0: z,
            at_15: z,
            at_20: z,
            tax: z,
        };
    }
    let bottom = if bottom < z { z } else { bottom };
    let top = bottom + pref;
    let clamp = |v: Usd| if v < z { z } else { v };
    // 0% zone: pref dollars below max_zero
    let at_0 = {
        let room = clamp(bp.max_zero - bottom);
        if room < pref {
            room
        } else {
            pref
        }
    };
    // 15% zone: (max_zero, max_fifteen]
    let lower15 = if bottom > bp.max_zero {
        bottom
    } else {
        bp.max_zero
    };
    let upper15 = if top < bp.max_fifteen {
        top
    } else {
        bp.max_fifteen
    };
    let at_15 = clamp(upper15 - lower15);
    let at_20 = pref - at_0 - at_15; // remainder above max_fifteen
    let tax = round_cents(at_15 * dec_15() + at_20 * dec_20());
    PrefSplit {
        at_0,
        at_15,
        at_20,
        tax,
    }
}
fn dec_15() -> Usd {
    rust_decimal_macros::dec!(0.15)
}
fn dec_20() -> Usd {
    rust_decimal_macros::dec!(0.20)
}

/// The result of §1222 ST/LT netting + the §1211/§1212(b) loss limit, by character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapNet {
    /// §1222(5)/(6): within-character net short-term (signed; after `cf_short`).
    pub st_net: Usd,
    /// §1222(7)/(8): within-character net long-term (signed; after `other_lt` & `cf_long`).
    pub lt_net: Usd,
    /// Net short-term gain surviving cross-net (≥0) → ordinary rates.
    pub ordinary_gain: Usd,
    /// §1222(11) net capital gain surviving cross-net (≥0) → §1(h) preferential rates.
    pub preferential_gain: Usd,
    /// §1211(b) ordinary offset used this year (≥0).
    pub loss_deduction: Usd,
    /// §1212(b) short-term carryforward out (≥0).
    pub st_carry: Usd,
    /// §1212(b) long-term carryforward out (≥0).
    pub lt_carry: Usd,
}

/// §1222 ST/LT netting + the §1211/§1212(b) capital-loss limit.
///
/// Inputs are signed: gains positive, losses negative for `crypto_st`/`crypto_lt`/`other_lt`.
/// `cf_short`/`cf_long` are prior-year carryforward LOSS magnitudes (≥0) — they REDUCE the matching
/// character. `loss_limit` is the statutory §1211(b) cap ($3,000 / $1,500 MFS).
///
/// Steps: (1) §1222(5)–(8) within-character netting, treating each prior-year carryforward as a loss of
/// its own character; (2) cross-net a gain in one character against a loss in the other, the residual
/// loss retaining the character it survived in; (3) §1211(b) deduct up to `loss_limit` against ordinary
/// income in a net-loss year; (4) §1212(b) carry the remainder forward by character, the deduction
/// absorbed **short-term-first** (the §1212(b)(2) deemed-short-term-gain ordering).
pub fn net_1222(
    crypto_st: Usd,
    crypto_lt: Usd,
    other_lt: Usd,
    cf_short: Usd,
    cf_long: Usd,
    loss_limit: Usd,
) -> CapNet {
    let z = Usd::ZERO;
    // §1222(5)/(6): within-character net short-term (carryforward-in is a short-term loss → subtract).
    let st_net = crypto_st - cf_short;
    // §1222(7)/(8): within-character net long-term (other_net_capital_gain is LT-character; cf_long subtracts).
    let lt_net = crypto_lt + other_lt - cf_long;

    // Cross-net a gain in one character against a loss in the other (§1222 / Schedule D line 16).
    let (st2, lt2) = match (st_net >= z, lt_net >= z) {
        (true, true) | (false, false) => (st_net, lt_net), // both gains, or both losses: no cross-net
        (true, false) => {
            // ST gain, LT loss
            if -lt_net <= st_net {
                (st_net + lt_net, z)
            } else {
                (z, st_net + lt_net)
            }
        }
        (false, true) => {
            // ST loss, LT gain
            if -st_net <= lt_net {
                (z, lt_net + st_net)
            } else {
                (st_net + lt_net, z)
            }
        }
    };
    let ordinary_gain = if st2 > z { st2 } else { z };
    let preferential_gain = if lt2 > z { lt2 } else { z };
    let net_st_loss = if st2 < z { -st2 } else { z };
    let net_lt_loss = if lt2 < z { -lt2 } else { z };
    let net_loss = net_st_loss + net_lt_loss;

    // §1211(b) limit + §1212(b) ST-first absorption, character-preserving carryforward (M3).
    let loss_deduction = if net_loss < loss_limit {
        net_loss
    } else {
        loss_limit
    };
    let absorbed_st = if net_st_loss < loss_deduction {
        net_st_loss
    } else {
        loss_deduction
    };
    let absorbed_lt = loss_deduction - absorbed_st;
    CapNet {
        st_net,
        lt_net,
        ordinary_gain,
        preferential_gain,
        loss_deduction,
        st_carry: net_st_loss - absorbed_st,
        lt_carry: net_lt_loss - absorbed_lt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::tables::{LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule};
    use rust_decimal_macros::dec;

    fn sched() -> OrdinarySchedule {
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.10),
                },
                OrdinaryBracket {
                    lower: dec!(10000),
                    rate: dec!(0.20),
                },
                OrdinaryBracket {
                    lower: dec!(40000),
                    rate: dec!(0.30),
                },
            ],
        }
    }

    #[test]
    fn ordinary_tax_sums_marginal_brackets_exactly() {
        // 0 → 0
        assert_eq!(ordinary_tax_on(&sched(), dec!(0)), dec!(0.00));
        // exactly at a boundary: $10,000 all at 10% = $1,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(10000)), dec!(1000.00));
        // $25,000 = 10%·10,000 + 20%·15,000 = 1,000 + 3,000 = 4,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(25000)), dec!(4000.00));
        // into the open-ended top: $50,000 = 1,000 + 20%·30,000(=6,000) + 30%·10,000(=3,000) = 10,000.00
        assert_eq!(ordinary_tax_on(&sched(), dec!(50000)), dec!(10000.00));
    }

    fn bp() -> LtcgBreakpoints {
        LtcgBreakpoints {
            max_zero: dec!(48350),
            max_fifteen: dec!(533400),
        }
    }

    #[test]
    fn preferential_zero_then_fifteen() {
        // bottom 40,000 ordinary, pref 20,000 LT → 8,350 @ 0%, 11,650 @ 15% = 1,747.50
        let s = preferential_tax(&bp(), dec!(40000), dec!(20000));
        assert_eq!(s.at_0, dec!(8350));
        assert_eq!(s.at_15, dec!(11650));
        assert_eq!(s.at_20, dec!(0));
        assert_eq!(s.tax, dec!(1747.50));
    }

    #[test]
    fn preferential_fifteen_then_twenty() {
        // bottom 500,000 ordinary, pref 100,000 → 33,400 @ 15% + 66,600 @ 20% = 5,010 + 13,320 = 18,330.00
        let s = preferential_tax(&bp(), dec!(500000), dec!(100000));
        assert_eq!(s.at_0, dec!(0));
        assert_eq!(s.at_15, dec!(33400));
        assert_eq!(s.at_20, dec!(66600));
        assert_eq!(s.tax, dec!(18330.00));
    }

    #[test]
    fn preferential_all_zero_when_under_max_zero() {
        // bottom 10,000, pref 20,000, top 30,000 < 48,350 → all 0%
        let s = preferential_tax(&bp(), dec!(10000), dec!(20000));
        assert_eq!(s.at_0, dec!(20000));
        assert_eq!(s.tax, dec!(0.00));
    }

    #[test]
    fn preferential_zero_pref_is_zero_tax() {
        assert_eq!(
            preferential_tax(&bp(), dec!(100000), dec!(0)).tax,
            dec!(0.00)
        );
    }
}

#[cfg(test)]
mod net_tests {
    use super::*;
    use rust_decimal_macros::dec;
    fn lim() -> Usd {
        dec!(3000)
    }

    #[test]
    fn both_gains_no_crossnet() {
        let n = net_1222(dec!(5000), dec!(8000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.ordinary_gain, dec!(5000));
        assert_eq!(n.preferential_gain, dec!(8000));
        assert_eq!(n.loss_deduction, dec!(0));
    }

    #[test]
    fn within_character_then_crossnet_order() {
        // ST gain 10,000; LT loss 4,000 → LT loss offsets ST gain → net ST gain 6,000, no preferential.
        let n = net_1222(dec!(10000), dec!(-4000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.st_net, dec!(10000));
        assert_eq!(n.lt_net, dec!(-4000));
        assert_eq!(n.ordinary_gain, dec!(6000));
        assert_eq!(n.preferential_gain, dec!(0));
        assert_eq!(n.loss_deduction, dec!(0));
    }

    #[test]
    fn st_loss_offsets_lt_gain_to_preferential() {
        // ST loss 3,000; LT gain 9,000 → net capital gain 6,000 (preferential), no ordinary.
        let n = net_1222(dec!(-3000), dec!(9000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.ordinary_gain, dec!(0));
        assert_eq!(n.preferential_gain, dec!(6000));
    }

    #[test]
    fn loss_year_3k_limit_st_first_carryforward() {
        // ST loss 5,000; LT loss 2,000 → total loss 7,000; deduct 3,000 (ST-first); carry 2,000 ST + 2,000 LT.
        let n = net_1222(dec!(-5000), dec!(-2000), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.loss_deduction, dec!(3000));
        assert_eq!(n.st_carry, dec!(2000)); // §1212(b): the $3k came out of ST loss first
        assert_eq!(n.lt_carry, dec!(2000));
    }

    #[test]
    fn loss_limit_is_mfs_1500() {
        let n = net_1222(dec!(-5000), dec!(0), dec!(0), dec!(0), dec!(0), dec!(1500));
        assert_eq!(n.loss_deduction, dec!(1500));
        assert_eq!(n.st_carry, dec!(3500));
        assert_eq!(n.lt_carry, dec!(0));
    }

    #[test]
    fn multi_year_carryforward_preserves_character() {
        // Year 1: ST loss 5,000 + LT loss 2,000 → carry {short:2000, long:2000} (from prior test).
        let y1 = net_1222(dec!(-5000), dec!(-2000), dec!(0), dec!(0), dec!(0), lim());
        // Year 2: LT gain 10,000, no crypto ST; carry-in {short:2000, long:2000}.
        // st_net = 0 - 2000 = -2000; lt_net = 10000 - 2000 = 8000; cross-net: ST loss offsets LT gain →
        // preferential 6,000, no loss.
        let y2 = net_1222(
            dec!(0),
            dec!(10000),
            dec!(0),
            y1.st_carry,
            y1.lt_carry,
            lim(),
        );
        assert_eq!(y2.preferential_gain, dec!(6000));
        assert_eq!(y2.ordinary_gain, dec!(0));
        assert_eq!(y2.loss_deduction, dec!(0));
    }

    #[test]
    fn st_loss_only_3k_all_st_character() {
        let n = net_1222(dec!(-10000), dec!(0), dec!(0), dec!(0), dec!(0), lim());
        assert_eq!(n.loss_deduction, dec!(3000));
        assert_eq!(n.st_carry, dec!(7000));
        assert_eq!(n.lt_carry, dec!(0));
    }
}
