//! Tax-table types, the `TaxTables` lookup trait, and **statutory** (non-indexed) constants.
//!
//! **Statutory-vs-indexed separation (I4 / Global Constraints):**
//! - **Indexed** values (ordinary brackets, §1(h) LTCG breakpoints) belong in a per-year `TaxTable`
//!   keyed by `(year, FilingStatus)` and sourced from the applicable Rev. Proc.
//! - **Statutory** values (`NIIT_RATE`, `niit_threshold`, `loss_limit`) are fixed in the U.S. Code
//!   and do **not** move year-over-year.  They are year-independent constants/functions here, with
//!   their statute cite, and are **never** placed in a `TaxTable`.
//!
//! Federal only (app charter / spec intro).  No float (NFR5).
use crate::conventions::Usd;
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

// ── Indexed table types ────────────────────────────────────────────────────────────────────────

/// One bracket of the ordinary-income rate schedule (§1(c)/§1(a)/§1(d)/§1(b)).
/// `rate` applies to taxable income in the half-open interval `[lower, next.lower)`;
/// the last bracket in the schedule is open-ended (no upper bound).
/// Rate is a `Decimal` fraction, e.g. `dec!(0.22)` for 22%.  Never a float (NFR5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryBracket {
    pub lower: Usd, // bottom of this bracket (inclusive)
    pub rate: Usd,  // marginal rate as a Decimal fraction
}

/// The full ordinary-income marginal-bracket schedule for one filing status in one tax year.
/// Brackets are stored in ascending order of `lower`; the last bracket is open-ended.
/// Sourced from the Rev. Proc. for the applicable year (§1 + Inflation Adjustment Act).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinarySchedule {
    pub brackets: Vec<OrdinaryBracket>, // ascending by `lower`; last is open-ended
}

/// §1(h) preferential-rate breakpoints for one filing status in one tax year.
/// `max_zero` is the top of the 0% LTCG rate (income at/below this pays 0%);
/// `max_fifteen` is the top of the 15% rate (income above `max_fifteen` pays 20%).
/// Sourced from the Rev. Proc. for the applicable year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtcgBreakpoints {
    pub max_zero: Usd,    // §1(h)(1)(B): 0% rate applies while taxable income ≤ this
    pub max_fifteen: Usd, // §1(h)(1)(C): 15% rate applies up to this; above → 20%
}

/// All indexed per-year tax parameters for one tax year.
/// Contains **only** inflation-indexed values (ordinary schedules + §1(h) LTCG breakpoints).
/// **Never** contains the NIIT rate/threshold or the §1211(b) loss limit — those are statutory
/// (year-independent) and live in the free functions below (I4 / Global Constraints).
///
/// `source` is a human-readable cite, e.g. `"Rev. Proc. 2024-40 §2.01/§2.03 (TY2025)"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxTable {
    pub year: i32,
    pub source: &'static str, // e.g. "Rev. Proc. 2024-40 §2.01/§2.03 (TY2025)"
    /// Ordinary-income bracket schedules keyed by filing status.
    /// INDEXED to the year's Rev. Proc. — never NIIT/loss-limit.
    pub ordinary: BTreeMap<FilingStatus, OrdinarySchedule>,
    /// §1(h) LTCG breakpoints keyed by filing status.
    /// INDEXED to the year's Rev. Proc. — never NIIT/loss-limit.
    pub ltcg: BTreeMap<FilingStatus, LtcgBreakpoints>,
}

impl TaxTable {
    /// §1(h) / §1 / §1411: a Qualifying Surviving Spouse (`Qss`) uses the MFJ schedule and
    /// breakpoints for all rate lookups.  Map `Qss → Mfj`; all other statuses are identity.
    fn key(status: FilingStatus) -> FilingStatus {
        match status {
            FilingStatus::Qss => FilingStatus::Mfj,
            s => s,
        }
    }

    /// Return the ordinary-income schedule for `status` (maps `Qss → Mfj`).
    /// Panics if the table was constructed without the required status (programming error;
    /// bundled tables always contain all four canonical statuses).
    pub fn ordinary_for(&self, status: FilingStatus) -> &OrdinarySchedule {
        &self.ordinary[&Self::key(status)]
    }

    /// Return the §1(h) LTCG breakpoints for `status` (maps `Qss → Mfj`).
    /// Panics if the table was constructed without the required status (programming error;
    /// bundled tables always contain all four canonical statuses).
    pub fn ltcg_for(&self, status: FilingStatus) -> &LtcgBreakpoints {
        &self.ltcg[&Self::key(status)]
    }
}

// ── TaxTables trait ────────────────────────────────────────────────────────────────────────────

/// Lookup interface for the per-year indexed tax tables.  The primary implementation is
/// `BundledTaxTables` in `btctax-adapters`; tests use a `BTreeMap`-backed test double.
pub trait TaxTables {
    /// Return the `TaxTable` for `year`, or `None` if no table is available for that year
    /// (callers must return `TaxOutcome::NotComputable(TaxTableMissing)` in that case — B.4/I6).
    fn table_for(&self, year: i32) -> Option<&TaxTable>;
}

/// Convenience `TaxTables` impl over a `BTreeMap<i32, TaxTable>`.  Used by tests in Tasks 2–5
/// and by `BundledTaxTables` (adapter crate, Task 6).
impl TaxTables for BTreeMap<i32, TaxTable> {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        self.get(&year)
    }
}

// ── STATUTORY constants and functions (year-independent, I4) ──────────────────────────────────

/// §1411(a): Net Investment Income Tax rate.
/// **STATUTORY** — 26 U.S.C. §1411(a)(1).  Fixed in the Code; NOT inflation-indexed.
/// Value: 3.8% = 0.038 (exact Decimal; never a float, NFR5).
/// Must never be placed in a `TaxTable`.
pub const NIIT_RATE: Usd = dec!(0.038);

/// §1411(b): MAGI threshold above which the NIIT applies.
/// **STATUTORY** — 26 U.S.C. §1411(b)(1).  The dollar amounts are fixed in the Code and do
/// NOT move year-over-year (unlike bracket thresholds which are adjusted under §1(f)(3)).
/// Must never be placed in a `TaxTable`.
///
/// Thresholds per filing status:
/// - MFJ / QSS: $250,000  (§1411(b)(2)(A))
/// - Single / HoH: $200,000  (§1411(b)(1)(A))
/// - MFS: $125,000  (§1411(b)(3)(A))
pub fn niit_threshold(status: FilingStatus) -> Usd {
    match status {
        FilingStatus::Mfj | FilingStatus::Qss => dec!(250000),
        FilingStatus::Single | FilingStatus::HoH => dec!(200000),
        FilingStatus::Mfs => dec!(125000),
    }
}

/// §1211(b): capital-loss ordinary-offset limit for non-corporate taxpayers.
/// **STATUTORY** — 26 U.S.C. §1211(b).  Fixed in the Code; NOT inflation-indexed.
/// Must never be placed in a `TaxTable`.
///
/// - MFS: $1,500  (§1211(b)(1) — one-half of the general $3,000 for married filing separately)
/// - All other statuses: $3,000  (§1211(b)(1))
pub fn loss_limit(status: FilingStatus) -> Usd {
    match status {
        FilingStatus::Mfs => dec!(1500),
        _ => dec!(3000),
    }
}

// ── Test support ──────────────────────────────────────────────────────────────────────────────

/// A minimal synthetic `TaxTable` for use in Tasks 2–5 tests.  Numbers are hand-chosen to hit
/// bracket boundaries clearly; they are NOT real IRS numbers (those come in Task 6).
/// Exposed as `pub(crate)` under `#[cfg(test)]` so sibling test modules can reuse it without
/// duplication.
#[cfg(test)]
pub(crate) fn synthetic_table(year: i32) -> TaxTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.10),
                },
                OrdinaryBracket {
                    lower: dec!(10000),
                    rate: dec!(0.22),
                },
                OrdinaryBracket {
                    lower: dec!(100000),
                    rate: dec!(0.32),
                },
            ],
        },
    );
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(40000),
            max_fifteen: dec!(400000),
        },
    );
    TaxTable {
        year,
        source: "SYNTHETIC",
        ordinary,
        ltcg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// STATUTORY values are constant across years while indexed values move (I4 KAT).
    /// Asserts: niit_threshold returns the correct statutory amounts for every filing status;
    /// NIIT_RATE is 3.8%; loss_limit is $3,000 (general) / $1,500 (MFS).
    #[test]
    fn statutory_values_are_constant_across_years() {
        for status in [
            FilingStatus::Single,
            FilingStatus::Mfj,
            FilingStatus::Mfs,
            FilingStatus::HoH,
            FilingStatus::Qss,
        ] {
            // year-independent by construction: calling twice returns identical values
            assert_eq!(niit_threshold(status), niit_threshold(status));
        }
        assert_eq!(niit_threshold(FilingStatus::Mfj), dec!(250000));
        assert_eq!(niit_threshold(FilingStatus::Qss), dec!(250000));
        assert_eq!(niit_threshold(FilingStatus::Single), dec!(200000));
        assert_eq!(niit_threshold(FilingStatus::HoH), dec!(200000));
        assert_eq!(niit_threshold(FilingStatus::Mfs), dec!(125000));
        assert_eq!(NIIT_RATE, dec!(0.038));
        assert_eq!(loss_limit(FilingStatus::Mfs), dec!(1500));
        assert_eq!(loss_limit(FilingStatus::Single), dec!(3000));
        assert_eq!(loss_limit(FilingStatus::Mfj), dec!(3000));
        assert_eq!(loss_limit(FilingStatus::HoH), dec!(3000));
        assert_eq!(loss_limit(FilingStatus::Qss), dec!(3000));
    }

    /// QSS aliases MFJ for the indexed lookups (ordinary schedule + LTCG breakpoints).
    #[test]
    fn qss_uses_mfj_schedule() {
        let mut t = synthetic_table(2025);
        // Give MFJ a distinct schedule; QSS must resolve to it.
        t.ordinary.insert(
            FilingStatus::Mfj,
            OrdinarySchedule {
                brackets: vec![
                    OrdinaryBracket {
                        lower: dec!(0),
                        rate: dec!(0.10),
                    },
                    OrdinaryBracket {
                        lower: dec!(50000),
                        rate: dec!(0.22),
                    },
                ],
            },
        );
        t.ltcg.insert(
            FilingStatus::Mfj,
            LtcgBreakpoints {
                max_zero: dec!(80000),
                max_fifteen: dec!(500000),
            },
        );
        assert_eq!(
            t.ordinary_for(FilingStatus::Qss).brackets,
            t.ordinary_for(FilingStatus::Mfj).brackets
        );
        assert_eq!(
            *t.ltcg_for(FilingStatus::Qss),
            *t.ltcg_for(FilingStatus::Mfj)
        );
    }
}
