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
