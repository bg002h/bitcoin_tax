//! Bundled per-year tax tables — TY2025 indexed numbers from **Rev. Proc. 2024-40**.
//!
//! # What is bundled here
//! **Indexed** values only (ordinary brackets + §1(h) LTCG breakpoints).  These are
//! inflation-adjusted every year by the IRS under §1(f)(3) and sourced from the annual
//! Rev. Proc.
//!
//! **Statutory** constants (`NIIT_RATE`, `niit_threshold`, `loss_limit`) are fixed in the U.S.
//! Code and are **never** placed in a `TaxTable` — see
//! [`btctax_core::tax::tables`](btctax_core::tax::tables) (I4 / Global Constraints).
//!
//! # Source citation
//! TY2025 values are encoded verbatim from:
//! - **Rev. Proc. 2024-40 §2.01** — rate tables under §1(j)(2) (ordinary brackets)
//! - **Rev. Proc. 2024-40 §2.03** — Maximum Capital Gains Rate under §1(h) (LTCG breakpoints)
//!
//! The **One Big Beautiful Bill Act** (Pub. L. 119-21, 2025) made the TCJA rate structure
//! permanent and raised the 2025 standard deduction, but did **not** change the 2025 bracket
//! thresholds or the §1(h) breakpoints (the extra inflation bump to the 10%/12% brackets begins
//! 2026).  This crate receives `ordinary_taxable_income` (already post-deduction) and does not
//! use the standard deduction, so the TY2025 indexed values are exactly Rev. Proc. 2024-40.
//!
//! # TY2026
//! TY2026 is omitted (pending verification against Rev. Proc. 2025-32 + OBBBA structural law).
//! Callers requesting a year with no bundled table receive `None` from [`TaxTables::table_for`],
//! which the compute layer converts to `TaxOutcome::NotComputable(TaxTableMissing)` (B.4/I6).
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::{FilingStatus, Usd};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

/// Compiled-in tax tables for the years whose Rev. Procs. have been independently verified.
///
/// Currently contains **TY2025** only (from Rev. Proc. 2024-40).  TY2026 will be added once
/// verified against Rev. Proc. 2025-32 + OBBBA structural law.
///
/// Mirrors the `BundledPrices` load-invariant: pure, deterministic, no I/O.
#[derive(Debug, Clone)]
pub struct BundledTaxTables {
    by_year: BTreeMap<i32, TaxTable>,
}

impl BundledTaxTables {
    /// Build the compiled-in tables (TY2025 mandatory; later years added as their Rev. Procs. are
    /// verified).
    pub fn load() -> Self {
        let mut by_year = BTreeMap::new();
        by_year.insert(2025, ty2025());
        // by_year.insert(2026, ty2026());
        // ^ add ONLY when verified vs Rev. Proc. 2025-32 + OBBBA structural law
        Self { by_year }
    }
}

impl TaxTables for BundledTaxTables {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        self.by_year.get(&year)
    }
}

/// Construct an `OrdinaryBracket` from a (lower, rate) pair.
fn br(lower: Usd, rate: Usd) -> OrdinaryBracket {
    OrdinaryBracket { lower, rate }
}

/// TY2025 — Rev. Proc. 2024-40 §2.01 (rate tables) + §2.03 (Maximum Capital Gains Rate).
///
/// Values verified 2026-06-30 against Rev. Proc. 2024-40 (cross-checked vs Tax Foundation &
/// IRS IR-2024-273). OBBBA Pub. L. 119-21 confirmed to leave 2025 brackets/breakpoints
/// unchanged.
///
/// QSS is not inserted explicitly; `TaxTable::key` maps `Qss → Mfj` at lookup time, avoiding
/// drift between the two identical schedules.
fn ty2025() -> TaxTable {
    let mut ordinary = BTreeMap::new();

    // §2.01 — Single (§1(c) rate table)
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(11925), dec!(0.12)),
                br(dec!(48475), dec!(0.22)),
                br(dec!(103350), dec!(0.24)),
                br(dec!(197300), dec!(0.32)),
                br(dec!(250525), dec!(0.35)),
                br(dec!(626350), dec!(0.37)),
            ],
        },
    );

    // §2.01 — Married Filing Jointly / Qualifying Surviving Spouse (§1(a) rate table)
    // QSS aliases MFJ via TaxTable::key; no separate QSS entry needed.
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(23850), dec!(0.12)),
                br(dec!(96950), dec!(0.22)),
                br(dec!(206700), dec!(0.24)),
                br(dec!(394600), dec!(0.32)),
                br(dec!(501050), dec!(0.35)),
                br(dec!(751600), dec!(0.37)),
            ],
        },
    );

    // §2.01 — Head of Household (§1(b) rate table)
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(17000), dec!(0.12)),
                br(dec!(64850), dec!(0.22)),
                br(dec!(103350), dec!(0.24)),
                br(dec!(197300), dec!(0.32)),
                br(dec!(250500), dec!(0.35)),
                br(dec!(626350), dec!(0.37)),
            ],
        },
    );

    // §2.01 — Married Filing Separately (§1(d) rate table)
    // Note: lower bands match Single; 37% starts at $375,800 (half of MFJ $751,600).
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(11925), dec!(0.12)),
                br(dec!(48475), dec!(0.22)),
                br(dec!(103350), dec!(0.24)),
                br(dec!(197300), dec!(0.32)),
                br(dec!(250525), dec!(0.35)),
                br(dec!(375800), dec!(0.37)),
            ],
        },
    );

    // §2.03 — §1(h) LTCG breakpoints (max_zero = top of 0% band; max_fifteen = top of 15% band)
    // QSS aliases MFJ via TaxTable::key; no separate QSS entry needed.
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(48350),
            max_fifteen: dec!(533400),
        },
    );
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(96700),
            max_fifteen: dec!(600050),
        },
    );
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(64750),
            max_fifteen: dec!(566700),
        },
    );
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(48350),
            max_fifteen: dec!(300000),
        },
    );

    TaxTable {
        year: 2025,
        source: "Rev. Proc. 2024-40 §2.01/§2.03 (TY2025); OBBBA Pub. L. 119-21 \
                 left 2025 brackets/breakpoints unchanged",
        ordinary,
        ltcg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::tables::{loss_limit, niit_threshold, NIIT_RATE};
    use rust_decimal_macros::dec;

    #[test]
    fn ty2025_single_ordinary_brackets_match_rev_proc_2024_40() {
        let t = BundledTaxTables::load();
        let s = t
            .table_for(2025)
            .unwrap()
            .ordinary_for(FilingStatus::Single);
        assert_eq!(s.brackets[1].lower, dec!(11925)); // 12% start
        assert_eq!(s.brackets[2].lower, dec!(48475)); // 22% start
        assert_eq!(s.brackets[6].lower, dec!(626350)); // 37% start
        assert_eq!(s.brackets[6].rate, dec!(0.37));
    }

    #[test]
    fn ty2025_ltcg_breakpoints_all_statuses() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2025).unwrap();
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Single),
            LtcgBreakpoints {
                max_zero: dec!(48350),
                max_fifteen: dec!(533400)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Mfj),
            LtcgBreakpoints {
                max_zero: dec!(96700),
                max_fifteen: dec!(600050)
            }
        );
        // QSS ≡ MFJ (TaxTable::key maps Qss → Mfj)
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Qss),
            LtcgBreakpoints {
                max_zero: dec!(96700),
                max_fifteen: dec!(600050)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::HoH),
            LtcgBreakpoints {
                max_zero: dec!(64750),
                max_fifteen: dec!(566700)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Mfs),
            LtcgBreakpoints {
                max_zero: dec!(48350),
                max_fifteen: dec!(300000)
            }
        );
    }

    #[test]
    fn mfs_37_pct_starts_at_375800_and_mfj_at_751600() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2025).unwrap();
        assert_eq!(
            tt.ordinary_for(FilingStatus::Mfs)
                .brackets
                .last()
                .unwrap()
                .lower,
            dec!(375800)
        );
        assert_eq!(
            tt.ordinary_for(FilingStatus::Mfj)
                .brackets
                .last()
                .unwrap()
                .lower,
            dec!(751600)
        );
    }

    #[test]
    fn missing_year_returns_none() {
        assert!(BundledTaxTables::load().table_for(2099).is_none());
    }

    #[test]
    fn statutory_values_are_not_in_the_table_and_constant_across_years() {
        // STATUTORY (I4): no TaxTable field carries NIIT/loss-limit; the cited fns are
        // year-independent and must never appear in a TaxTable.
        assert_eq!(niit_threshold(FilingStatus::Mfj), dec!(250000));
        assert_eq!(loss_limit(FilingStatus::Mfs), dec!(1500));
        assert_eq!(NIIT_RATE, dec!(0.038));
        // If TY2026 is bundled, assert its indexed breakpoints DIFFER from TY2025 while the
        // statutory values above remain identical — the indexed-moves / statutory-fixed contrast.
    }
}
