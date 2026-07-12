//! Bundled per-year tax tables — TY2024, TY2025, and TY2026 indexed numbers from Rev. Proc.
//! 2023-34, Rev. Proc. 2024-40, and Rev. Proc. 2025-32 respectively.
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
//! TY2024 values are encoded verbatim from:
//! - **Rev. Proc. 2023-34 §3.01** — rate tables under §1(j)(2) (ordinary brackets)
//! - **Rev. Proc. 2023-34 §3.03** — Maximum Capital Gains Rate under §1(h) (LTCG breakpoints)
//! - **Rev. Proc. 2023-34 §3.43** — §2503(b) gift-tax annual exclusion per donee ($18,000)
//! - **Rev. Proc. 2023-34 §3.41** — §2010(c)(3) basic exclusion amount / lifetime gift+estate
//!   exclusion ($13,610,000)
//! - **SSA announcement 2023-10-12** — §230 Social Security Act (42 U.S.C. §430) wage base
//!   ($168,600)
//!
//! TY2025 values are encoded verbatim from:
//! - **Rev. Proc. 2024-40 §2.01** — rate tables under §1(j)(2) (ordinary brackets)
//! - **Rev. Proc. 2024-40 §2.03** — Maximum Capital Gains Rate under §1(h) (LTCG breakpoints)
//! - **Rev. Proc. 2024-40 §2.43** — §2503(b) gift-tax annual exclusion per donee ($19,000)
//! - **Rev. Proc. 2024-40 §2.41** — §2010(c)(3) basic exclusion amount / lifetime gift+estate
//!   exclusion ($13,990,000)
//!
//! The **One Big Beautiful Bill Act** (Pub. L. 119-21, 2025) made the TCJA rate structure
//! permanent and raised the 2025 standard deduction, but did **not** change the **TY2025** bracket
//! thresholds or the §1(h) breakpoints (the extra inflation bump to the 10%/12% brackets begins
//! 2026).  This crate receives `ordinary_taxable_income` (already post-deduction) and does not
//! use the standard deduction, so the TY2025 indexed values are exactly Rev. Proc. 2024-40.
//! (OBBBA is a 2025 enactment and does not affect TY2024 values.)
//!
//! TY2026 values are encoded verbatim from:
//! - **Rev. Proc. 2025-32 §4.01** — rate tables under §1(j)(2) (ordinary brackets)
//! - **Rev. Proc. 2025-32 §4.03** — Maximum Capital Gains Rate under §1(h) (LTCG breakpoints)
//! - **Rev. Proc. 2025-32 §4.42(1)** — §2503(b) gift-tax annual exclusion per donee ($19,000)
//! - **OBBBA Pub. L. 119-21 §70106** — §2010(c)(3) basic exclusion (flat statutory $15,000,000
//!   for CY2026; Rev. Proc. 2025-32 §2.14 confirms; first inflation-indexed 2027)
//! - **SSA determination (Fed. Reg. 2025-11-03)** — §230 Social Security Act (42 U.S.C. §430)
//!   wage base ($184,500)
//!
//! # Later years
//! TY2027+ are omitted: the IRS/SSA publish those figures in fall 2026, after our data horizon.
//! Callers requesting a year with no bundled table receive `None` from [`TaxTables::table_for`],
//! which the compute layer converts to `TaxOutcome::NotComputable(TaxTableMissing)` (B.4/I6).
use btctax_core::tax::tables::{
    FullReturnParams, FullReturnTables, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable,
    TaxTables,
};
use btctax_core::{FilingStatus, Usd};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

/// Compiled-in tax tables for the years whose Rev. Procs. have been independently verified.
///
/// Currently contains **TY2024** (Rev. Proc. 2023-34), **TY2025** (Rev. Proc. 2024-40), and
/// **TY2026** (Rev. Proc. 2025-32 + OBBBA Pub. L. 119-21). TY2027+ are added once the IRS/SSA
/// publish those figures (fall 2026).
///
/// Mirrors the `BundledPrices` load-invariant: pure, deterministic, no I/O.
#[derive(Debug, Clone)]
pub struct BundledTaxTables {
    by_year: BTreeMap<i32, TaxTable>,
}

impl BundledTaxTables {
    /// Build the compiled-in tables (TY2024, TY2025, and TY2026 bundled; later years added as
    /// their Rev. Procs. are verified).
    pub fn load() -> Self {
        let mut by_year = BTreeMap::new();
        by_year.insert(2017, ty2017());
        by_year.insert(2024, ty2024());
        by_year.insert(2025, ty2025());
        by_year.insert(2026, ty2026());
        Self { by_year }
    }
}

impl TaxTables for BundledTaxTables {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        self.by_year.get(&year)
    }
}

/// Compiled-in **full-return** per-year parameters (standard deduction + the year-varying limits the
/// absolute 1040 needs). NEW for the full-return build; separate from [`BundledTaxTables`] by design —
/// published-crate-API stability + v1-only fail-closed gating (see `btctax_core::tax::tables::FullReturnParams`).
/// **v1 bundles TY2024 only**; a year without params returns `None` → the caller fails closed.
#[derive(Debug, Clone)]
pub struct BundledFullReturnTables {
    by_year: BTreeMap<i32, FullReturnParams>,
}

impl BundledFullReturnTables {
    pub fn load() -> Self {
        let mut by_year = BTreeMap::new();
        by_year.insert(2024, ty2024_full_return());
        Self { by_year }
    }
}

impl FullReturnTables for BundledFullReturnTables {
    fn full_return_for(&self, year: i32) -> Option<&FullReturnParams> {
        self.by_year.get(&year)
    }
}

/// TY2024 full-return parameters. Standard deduction + §63(f)/§63(c)(5) amounts from **Rev. Proc.
/// 2023-34 §3.15**; the SALT cap (§164(b)(6) TCJA), §1(g)(4) kiddie threshold, §402(g)(1) deferral
/// limit (Notice 2023-75), and §904(j) FTC ceiling are the TY2024 figures.
fn ty2024_full_return() -> FullReturnParams {
    let mut std_deduction = BTreeMap::new();
    std_deduction.insert(FilingStatus::Single, dec!(14600));
    std_deduction.insert(FilingStatus::Mfj, dec!(29200));
    std_deduction.insert(FilingStatus::Mfs, dec!(14600));
    std_deduction.insert(FilingStatus::HoH, dec!(21900));
    FullReturnParams {
        year: 2024,
        std_deduction,
        std_aged_blind_married: dec!(1550),   // §63(f), Rev. Proc. 2023-34 §3.15(3)
        std_aged_blind_unmarried: dec!(1950),
        dependent_std_floor: dec!(1300),      // §63(c)(5), Rev. Proc. 2023-34 §3.15(2)
        dependent_std_earned_addon: dec!(450),
        salt_cap: dec!(10000),                // §164(b)(6) (MFS = $5,000 at the use site)
        kiddie_unearned_threshold: dec!(2600),// §1(g)(4)
        elective_deferral_limit: dec!(23000), // §402(g)(1), Notice 2023-75
        ftc_ceiling: dec!(300),               // §904(j) (MFJ = $600 at the use site)
    }
}

/// Construct an `OrdinaryBracket` from a (lower, rate) pair.
fn br(lower: Usd, rate: Usd) -> OrdinaryBracket {
    OrdinaryBracket { lower, rate }
}

/// TY2017 — **pre-TCJA** — Rev. Proc. 2016-55 §2.01 (rate tables) + §2.03 (Maximum Capital Gains
/// Rate), with the SSA §230 Social Security wage base ($127,200, SSA 2016-10-18).
///
/// 2017 is the last full pre-TCJA year: **seven** ordinary brackets at the historic
/// **10 / 15 / 25 / 28 / 33 / 35 / 39.6%** rates (NOT the TCJA 10/12/22/24/32/35/37 of 2018+), and the
/// §1(h) preferential rates keyed to the ordinary brackets (0% through the top of the 15% bracket,
/// 20% starting at the 39.6% threshold). Encoded VERBATIM from Rev. Proc. 2016-55 (I.R.B. 2016-49);
/// transcribed, never re-derived; pinned by `ty2017_table_matches_rev_proc_2016_55`.
///
/// This table exists so `export-irs-pdf --tax-year 2017` can compute the Schedule SE §1401 figure
/// (which needs the year's `ss_wage_base`); the 8949 / Schedule D / 8283 / 1040 packets are
/// table-free. QSS is not inserted explicitly; `TaxTable::key` maps `Qss → Mfj` at lookup time.
fn ty2017() -> TaxTable {
    let mut ordinary = BTreeMap::new();

    // §2.01 Table 3 — Single (§1(c) rate schedule).
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(9325), dec!(0.15)),
                br(dec!(37950), dec!(0.25)),
                br(dec!(91900), dec!(0.28)),
                br(dec!(191650), dec!(0.33)),
                br(dec!(416700), dec!(0.35)),
                br(dec!(418400), dec!(0.396)),
            ],
        },
    );

    // §2.01 Table 1 — Married Filing Jointly / Qualifying Surviving Spouse (§1(a) rate schedule).
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(18650), dec!(0.15)),
                br(dec!(75900), dec!(0.25)),
                br(dec!(153100), dec!(0.28)),
                br(dec!(233350), dec!(0.33)),
                br(dec!(416700), dec!(0.35)),
                br(dec!(470700), dec!(0.396)),
            ],
        },
    );

    // §2.01 Table 2 — Head of Household (§1(b) rate schedule).
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(13350), dec!(0.15)),
                br(dec!(50800), dec!(0.25)),
                br(dec!(131200), dec!(0.28)),
                br(dec!(212500), dec!(0.33)),
                br(dec!(416700), dec!(0.35)),
                br(dec!(444550), dec!(0.396)),
            ],
        },
    );

    // §2.01 Table 4 — Married Filing Separately (§1(d) rate schedule).
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(9325), dec!(0.15)),
                br(dec!(37950), dec!(0.25)),
                br(dec!(76550), dec!(0.28)),
                br(dec!(116675), dec!(0.33)),
                br(dec!(208350), dec!(0.35)),
                br(dec!(235350), dec!(0.396)),
            ],
        },
    );

    // §2.03 — §1(h) LTCG breakpoints (0% through top of the 15% ordinary bracket; 20% from the 39.6%
    // ordinary threshold). max_zero = top of the 0% band; max_fifteen = top of the 15% band.
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(37950),
            max_fifteen: dec!(418400),
        },
    );
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(75900),
            max_fifteen: dec!(470700),
        },
    );
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(50800),
            max_fifteen: dec!(444550),
        },
    );
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(37950),
            max_fifteen: dec!(235350),
        },
    );

    TaxTable {
        year: 2017,
        source: "Rev. Proc. 2016-55 §2.01/§2.03 (TY2017, pre-TCJA 10/15/25/28/33/35/39.6%); \
                 SSA 2016-10-18 (ss_wage_base $127,200)",
        ordinary,
        ltcg,
        // §2503(b) gift annual exclusion per donee — Rev. Proc. 2016-55 §2.35(1) (TY2017 = $14,000).
        gift_annual_exclusion: dec!(14000),
        // §230 SSA (42 U.S.C. §430) Social Security wage base — SSA announced 2016-10-18
        // (TY2017 = $127,200, up from TY2016 $118,500).
        ss_wage_base: dec!(127200),
        // §2010(c)(3) basic exclusion amount (unified credit / lifetime gift+estate exclusion) —
        // Rev. Proc. 2016-55 §2.41 (TY2017 = $5,490,000).
        gift_lifetime_exclusion: dec!(5_490_000),
    }
}

/// TY2024 — Rev. Proc. 2023-34 §3.01 (rate tables) + §3.03 (Maximum Capital Gains Rate).
///
/// Values verified against Rev. Proc. 2023-34 (irs.gov/pub/irs-drop/rp-23-34.pdf, §3.01 Tables
/// 1–4; §3.03; §3.43; §3.41) and SSA announcement 2023-10-12.
///
/// Note: the official Rev. Proc. 2023-34 PDF prints "$191,150" in the 32%-row description of
/// Tables 2–4, but the bound column in those rows reads "$191,950" — consistent with IRB 2023-48
/// HTML and the base-tax arithmetic.  The correct 32% lower bound is **$191,950** throughout.
///
/// Note: MFS `max_fifteen` = $291,850 (Rev. Proc. 2023-34 §3.03 verbatim; NOT exactly half of
/// MFJ $583,750 — independent rounding by the Rev. Proc.).
///
/// QSS is not inserted explicitly; `TaxTable::key` maps `Qss → Mfj` at lookup time.
fn ty2024() -> TaxTable {
    let mut ordinary = BTreeMap::new();

    // §3.01 — Single (§1(j)(2)(C): Unmarried Individuals rate table)
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(11600), dec!(0.12)),
                br(dec!(47150), dec!(0.22)),
                br(dec!(100525), dec!(0.24)),
                br(dec!(191950), dec!(0.32)),
                br(dec!(243725), dec!(0.35)),
                br(dec!(609350), dec!(0.37)),
            ],
        },
    );

    // §3.01 — Married Filing Jointly / Qualifying Surviving Spouse (§1(j)(2)(A) rate table)
    // QSS aliases MFJ via TaxTable::key; no separate QSS entry needed.
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(23200), dec!(0.12)),
                br(dec!(94300), dec!(0.22)),
                br(dec!(201050), dec!(0.24)),
                br(dec!(383900), dec!(0.32)),
                br(dec!(487450), dec!(0.35)),
                br(dec!(731200), dec!(0.37)),
            ],
        },
    );

    // §3.01 — Head of Household (§1(j)(2)(B) rate table)
    // Note: 35% starts at $243,700 (vs Single/MFS $243,725 — distinct value per Rev. Proc.).
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(16550), dec!(0.12)),
                br(dec!(63100), dec!(0.22)),
                br(dec!(100500), dec!(0.24)),
                br(dec!(191950), dec!(0.32)),
                br(dec!(243700), dec!(0.35)),
                br(dec!(609350), dec!(0.37)),
            ],
        },
    );

    // §3.01 — Married Filing Separately (§1(j)(2)(D) rate table)
    // Note: lower bands 10%–35% mirror Single; 37% starts at $365,600 (half of MFJ $731,200,
    // stated explicitly in Rev. Proc. 2023-34 §3.01 Table 4).
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(11600), dec!(0.12)),
                br(dec!(47150), dec!(0.22)),
                br(dec!(100525), dec!(0.24)),
                br(dec!(191950), dec!(0.32)),
                br(dec!(243725), dec!(0.35)),
                br(dec!(365600), dec!(0.37)),
            ],
        },
    );

    // §3.03 — §1(h) LTCG breakpoints (max_zero = top of 0% band; max_fifteen = top of 15% band)
    // QSS aliases MFJ via TaxTable::key; no separate QSS entry needed.
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(47025),
            max_fifteen: dec!(518900),
        },
    );
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(94050),
            max_fifteen: dec!(583750),
        },
    );
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(63000),
            max_fifteen: dec!(551350),
        },
    );
    // Note: MFS max_fifteen = $291,850 (NOT $291,875 = $583,750/2; independent rounding in Rev. Proc.).
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(47025),
            max_fifteen: dec!(291850),
        },
    );

    TaxTable {
        year: 2024,
        source: "Rev. Proc. 2023-34 §3.01/§3.03 + §3.43 + §3.41 (TY2024); \
                 SSA 2023-10-12 (ss_wage_base $168,600)",
        ordinary,
        ltcg,
        // §2503(b) gift annual exclusion per donee — Rev. Proc. 2023-34 §3.43 (TY2024 = $18,000).
        gift_annual_exclusion: dec!(18000),
        // §230 SSA (42 U.S.C. §430) Social Security wage base — SSA announced 2023-10-12
        // (TY2024 = $168,600).
        ss_wage_base: dec!(168600),
        // §2010(c)(3) basic exclusion amount (unified credit / lifetime gift+estate exclusion) —
        // Rev. Proc. 2023-34 §3.41 (TY2024 = $13,610,000).
        gift_lifetime_exclusion: dec!(13_610_000),
    }
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
        source: "Rev. Proc. 2024-40 §2.01/§2.03 + §2.43 + §2.41 (TY2025); OBBBA Pub. L. 119-21 \
                 left 2025 brackets/breakpoints unchanged",
        ordinary,
        ltcg,
        // §2503(b) gift annual exclusion per donee — Rev. Proc. 2024-40 §2.43 (TY2025 = $19,000).
        gift_annual_exclusion: dec!(19000),
        // §230 SSA (42 U.S.C. §430) Social Security wage base — SSA announced 2024-10-10
        // (TY2025 = $176,100).
        ss_wage_base: dec!(176100),
        // §2010(c)(3) basic exclusion amount (unified credit / lifetime gift+estate exclusion) —
        // Rev. Proc. 2024-40 §2.41 (TY2025 = $13,990,000).
        gift_lifetime_exclusion: dec!(13_990_000),
    }
}

/// TY2026 — Rev. Proc. 2025-32 §4.01 (rate tables) + §4.03 (Maximum Capital Gains Rate).
///
/// Values verified against the PRIMARY sources (Rev. Proc. 2025-32, I.R.B. 2025-45; SSA
/// determination Fed. Reg. 2025-11-03; OBBBA Pub. L. 119-21 §70106):
/// - §4.01 Tables 1–4 — §1(j)(2) ordinary rate tables.
/// - §4.03 — §1(h) Maximum Capital Gains Rate (LTCG breakpoints).
/// - §4.42(1) — §2503(b) gift-tax annual exclusion per donee ($19,000, unchanged from TY2025).
/// - §2010(c)(3) basic exclusion — a flat statutory $15,000,000 set by OBBBA Pub. L. 119-21
///   §70106 (Rev. Proc. 2025-32 §2.14 confirms); first inflation-indexed in 2027, so NOT a
///   §1(f) item this year.
/// - §230 SSA (42 U.S.C. §430) SS wage base $184,500 — SSA determination (Fed. Reg. 2025-11-03),
///   up from TY2025 $176,100.
///
/// Traps (transcribed, never re-derived): HoH 32%/35% start at $201,750/$256,200 — NOT Single's
/// $201,775/$256,225. MFS lower bands 10%–35% mirror Single, but 37% starts at $384,350
/// (= ½ of MFJ $768,700).
///
/// QSS is not inserted explicitly; `TaxTable::key` maps `Qss → Mfj` at lookup time, avoiding
/// drift between the two identical schedules.
fn ty2026() -> TaxTable {
    let mut ordinary = BTreeMap::new();

    // §4.01 Table 3 — Single (§1(j)(2)(C): Unmarried Individuals rate table)
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(12400), dec!(0.12)),
                br(dec!(50400), dec!(0.22)),
                br(dec!(105700), dec!(0.24)),
                br(dec!(201775), dec!(0.32)),
                br(dec!(256225), dec!(0.35)),
                br(dec!(640600), dec!(0.37)),
            ],
        },
    );

    // §4.01 Table 1 — Married Filing Jointly / Qualifying Surviving Spouse (§1(j)(2)(A) rate table)
    // QSS aliases MFJ via TaxTable::key; no separate QSS entry needed.
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(24800), dec!(0.12)),
                br(dec!(100800), dec!(0.22)),
                br(dec!(211400), dec!(0.24)),
                br(dec!(403550), dec!(0.32)),
                br(dec!(512450), dec!(0.35)),
                br(dec!(768700), dec!(0.37)),
            ],
        },
    );

    // §4.01 Table 2 — Head of Household (§1(j)(2)(B) rate table)
    // TRAP: 32%/35% start at $201,750/$256,200 — distinct from Single's $201,775/$256,225.
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(17700), dec!(0.12)),
                br(dec!(67450), dec!(0.22)),
                br(dec!(105700), dec!(0.24)),
                br(dec!(201750), dec!(0.32)),
                br(dec!(256200), dec!(0.35)),
                br(dec!(640600), dec!(0.37)),
            ],
        },
    );

    // §4.01 Table 4 — Married Filing Separately (§1(j)(2)(D) rate table)
    // Note: lower bands 10%–35% mirror Single; 37% starts at $384,350 (half of MFJ $768,700).
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                br(dec!(0), dec!(0.10)),
                br(dec!(12400), dec!(0.12)),
                br(dec!(50400), dec!(0.22)),
                br(dec!(105700), dec!(0.24)),
                br(dec!(201775), dec!(0.32)),
                br(dec!(256225), dec!(0.35)),
                br(dec!(384350), dec!(0.37)),
            ],
        },
    );

    // §4.03 — §1(h) LTCG breakpoints (max_zero = top of 0% band; max_fifteen = top of 15% band)
    // QSS aliases MFJ via TaxTable::key; no separate QSS entry needed.
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(49450),
            max_fifteen: dec!(545500),
        },
    );
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(98900),
            max_fifteen: dec!(613700),
        },
    );
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(66200),
            max_fifteen: dec!(579600),
        },
    );
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(49450),
            max_fifteen: dec!(306850),
        },
    );

    TaxTable {
        year: 2026,
        source: "Rev. Proc. 2025-32 §4.01/§4.03 + §4.42 (TY2026); §2010(c)(3) basic exclusion \
                 $15,000,000 per OBBBA Pub. L. 119-21 §70106; SS wage base $184,500 per SSA \
                 (Fed. Reg. 2025-11-03)",
        ordinary,
        ltcg,
        // §2503(b) gift annual exclusion per donee — Rev. Proc. 2025-32 §4.42(1)
        // (TY2026 = $19,000; unchanged from TY2025).
        gift_annual_exclusion: dec!(19000),
        // §230 SSA (42 U.S.C. §430) Social Security wage base — SSA determination
        // (Fed. Reg. 2025-11-03; TY2026 = $184,500, up from $176,100).
        ss_wage_base: dec!(184500),
        // §2010(c)(3) basic exclusion amount (unified credit / lifetime gift+estate exclusion) —
        // flat statutory $15,000,000 set by OBBBA Pub. L. 119-21 §70106 (Rev. Proc. 2025-32 §2.14
        // confirms; first inflation-indexed 2027, NOT a §1(f) item for TY2026).
        gift_lifetime_exclusion: dec!(15_000_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::tables::{loss_limit, niit_threshold, NIIT_RATE};
    use btctax_core::{
        compute_tax_year, BasisSource, Disposal, DisposalLeg, DisposeKind, EventId, LedgerState,
        LotId, Source, SourceRef, TaxOutcome, TaxProfile, Term, WalletId,
    };
    use rust_decimal_macros::dec;
    use time::macros::date;

    /// Full-return Phase 0 / plan-review C1 / KAT-3: every bundled year's ordinary schedule is
    /// Tax-Table-binnable — every sub-$100k bracket edge is a multiple of $25 (a $50-bin boundary OR
    /// its exact midpoint, which still reproduces the printed cell since the IRS taxes at the midpoint).
    /// deep/01's stricter "no interior edge" was TY2024-only; TY2017 (9,325) and TY2025 (11,925 / 48,475)
    /// have midpoint edges (≡ 25 mod 50) that this correctly permits.
    #[test]
    fn all_bundled_years_are_tax_table_binnable() {
        use btctax_core::tax::method::first_unbinnable_edge;
        let t = BundledTaxTables::load();
        // Derive the covered years from what is actually bundled (don't hardcode the list).
        let mut checked = 0;
        for year in 2000..=2100 {
            let Some(tbl) = t.table_for(year) else { continue };
            for status in [
                FilingStatus::Single,
                FilingStatus::Mfj,
                FilingStatus::Mfs,
                FilingStatus::HoH,
            ] {
                assert_eq!(
                    first_unbinnable_edge(tbl.ordinary_for(status)),
                    None,
                    "year {year} {status:?}: a sub-$100k bracket edge is not a $25 multiple"
                );
            }
            checked += 1;
        }
        assert!(checked >= 4, "expected every bundled year swept; checked {checked}");
    }

    /// TY2024 full-return params are bundled with the correct Rev. Proc. 2023-34 / statutory figures;
    /// v1 bundles only TY2024 (other years → None → the caller fails closed).
    #[test]
    fn ty2024_full_return_params_bundled() {
        let t = BundledFullReturnTables::load();
        let p = t.full_return_for(2024).unwrap();
        assert_eq!(p.std_deduction_for(FilingStatus::Single), dec!(14600));
        assert_eq!(p.std_deduction_for(FilingStatus::Mfj), dec!(29200));
        assert_eq!(p.std_deduction_for(FilingStatus::HoH), dec!(21900));
        assert_eq!(p.std_deduction_for(FilingStatus::Qss), dec!(29200)); // Qss→Mfj
        assert_eq!(p.std_aged_blind_married, dec!(1550));
        assert_eq!(p.std_aged_blind_unmarried, dec!(1950));
        assert_eq!(p.dependent_std_floor, dec!(1300));
        assert_eq!(p.dependent_std_earned_addon, dec!(450));
        assert_eq!(p.salt_cap, dec!(10000));
        assert_eq!(p.kiddie_unearned_threshold, dec!(2600));
        assert_eq!(p.elective_deferral_limit, dec!(23000));
        assert_eq!(p.ftc_ceiling, dec!(300));
        assert!(t.full_return_for(2025).is_none()); // v1 = TY2024 only → fail closed elsewhere
        assert!(t.full_return_for(2017).is_none());
    }

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

    /// P2-C Task 3: TY2025 §2503(b) gift annual exclusion is $19,000 (Rev. Proc. 2024-40 §2.43).
    #[test]
    fn ty2025_gift_annual_exclusion_is_19000() {
        let t = BundledTaxTables::load();
        assert_eq!(
            t.table_for(2025).unwrap().gift_annual_exclusion,
            dec!(19000)
        );
    }

    #[test]
    fn statutory_values_are_not_in_the_table_and_constant_across_years() {
        // STATUTORY (I4): no TaxTable field carries NIIT/loss-limit; the cited fns are
        // year-independent and must never appear in a TaxTable.
        assert_eq!(niit_threshold(FilingStatus::Mfj), dec!(250000));
        assert_eq!(loss_limit(FilingStatus::Mfs), dec!(1500));
        assert_eq!(NIIT_RATE, dec!(0.038));

        // Indexed-moves / statutory-fixed contrast: TY2026 indexed breakpoints DIFFER from TY2025
        // (inflation-adjusted under §1(f)) while the statutory values above are unchanged for 2026.
        let t = BundledTaxTables::load();
        let s25 = t
            .table_for(2025)
            .unwrap()
            .ordinary_for(FilingStatus::Single);
        let s26 = t
            .table_for(2026)
            .unwrap()
            .ordinary_for(FilingStatus::Single);
        assert_ne!(s25.brackets[1].lower, s26.brackets[1].lower); // 12% start moved
        assert_ne!(
            t.table_for(2025)
                .unwrap()
                .ltcg_for(FilingStatus::Single)
                .max_zero,
            t.table_for(2026)
                .unwrap()
                .ltcg_for(FilingStatus::Single)
                .max_zero
        );
        // Statutory functions are the same regardless of year (no per-year table field for them).
        assert_eq!(niit_threshold(FilingStatus::Mfj), dec!(250000));
        assert_eq!(NIIT_RATE, dec!(0.038));
    }

    // ── TY2026 KATs (Rev. Proc. 2025-32 §4.01/§4.03 + §4.42; OBBBA §70106; SSA Fed. Reg. 2025-11-03)

    #[test]
    fn ty2026_single_ordinary_brackets_match_rev_proc_2025_32() {
        let t = BundledTaxTables::load();
        let s = t
            .table_for(2026)
            .unwrap()
            .ordinary_for(FilingStatus::Single);
        assert_eq!(s.brackets[1].lower, dec!(12400)); // 12% start
        assert_eq!(s.brackets[2].lower, dec!(50400)); // 22% start
        assert_eq!(s.brackets[3].lower, dec!(105700)); // 24% start
        assert_eq!(s.brackets[4].lower, dec!(201775)); // 32% start
        assert_eq!(s.brackets[5].lower, dec!(256225)); // 35% start
        assert_eq!(s.brackets[6].lower, dec!(640600)); // 37% start
        assert_eq!(s.brackets[6].rate, dec!(0.37));
    }

    #[test]
    fn ty2026_mfj_ordinary_brackets_match_rev_proc_2025_32() {
        let t = BundledTaxTables::load();
        let s = t.table_for(2026).unwrap().ordinary_for(FilingStatus::Mfj);
        assert_eq!(s.brackets[1].lower, dec!(24800)); // 12% start
        assert_eq!(s.brackets[2].lower, dec!(100800)); // 22% start
        assert_eq!(s.brackets[3].lower, dec!(211400)); // 24% start
        assert_eq!(s.brackets[4].lower, dec!(403550)); // 32% start
        assert_eq!(s.brackets[5].lower, dec!(512450)); // 35% start
        assert_eq!(s.brackets[6].lower, dec!(768700)); // 37% start
        assert_eq!(s.brackets[6].rate, dec!(0.37));
    }

    /// ★ Fault-inject target: HoH 32%/35% start at $201,750/$256,200 — NOT Single's
    /// $201,775/$256,225. Swapping either to the Single value must turn this RED.
    #[test]
    fn ty2026_hoh_ordinary_brackets_match_rev_proc_2025_32() {
        let t = BundledTaxTables::load();
        let s = t.table_for(2026).unwrap().ordinary_for(FilingStatus::HoH);
        assert_eq!(s.brackets[1].lower, dec!(17700)); // 12% start
        assert_eq!(s.brackets[2].lower, dec!(67450)); // 22% start
        assert_eq!(s.brackets[3].lower, dec!(105700)); // 24% start
        assert_eq!(s.brackets[4].lower, dec!(201750)); // 32% start — TRAP: NOT Single's $201,775
        assert_eq!(s.brackets[5].lower, dec!(256200)); // 35% start — TRAP: NOT Single's $256,225
        assert_eq!(s.brackets[6].lower, dec!(640600)); // 37% start
        assert_eq!(s.brackets[6].rate, dec!(0.37));
    }

    #[test]
    fn ty2026_mfs_37_pct_starts_at_384350() {
        let t = BundledTaxTables::load();
        let s = t.table_for(2026).unwrap().ordinary_for(FilingStatus::Mfs);
        // Lower bands 10%–35% mirror Single.
        assert_eq!(s.brackets[1].lower, dec!(12400)); // 12% start
        assert_eq!(s.brackets[4].lower, dec!(201775)); // 32% start
        assert_eq!(s.brackets[5].lower, dec!(256225)); // 35% start
        assert_eq!(s.brackets.last().unwrap().lower, dec!(384350)); // 37% start = ½ MFJ $768,700
        assert_eq!(s.brackets.last().unwrap().rate, dec!(0.37));
    }

    #[test]
    fn ty2026_ltcg_breakpoints_all_statuses() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2026).unwrap();
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Single),
            LtcgBreakpoints {
                max_zero: dec!(49450),
                max_fifteen: dec!(545500)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Mfj),
            LtcgBreakpoints {
                max_zero: dec!(98900),
                max_fifteen: dec!(613700)
            }
        );
        // QSS ≡ MFJ (TaxTable::key maps Qss → Mfj)
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Qss),
            LtcgBreakpoints {
                max_zero: dec!(98900),
                max_fifteen: dec!(613700)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::HoH),
            LtcgBreakpoints {
                max_zero: dec!(66200),
                max_fifteen: dec!(579600)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Mfs),
            LtcgBreakpoints {
                max_zero: dec!(49450),
                max_fifteen: dec!(306850)
            }
        );
    }

    #[test]
    fn ty2026_gift_annual_exclusion_is_19000() {
        let t = BundledTaxTables::load();
        assert_eq!(
            t.table_for(2026).unwrap().gift_annual_exclusion,
            dec!(19000)
        );
    }

    #[test]
    fn ty2026_ss_wage_base_is_184500() {
        let t = BundledTaxTables::load();
        assert_eq!(t.table_for(2026).unwrap().ss_wage_base, dec!(184500));
    }

    #[test]
    fn ty2026_lifetime_exclusion_is_15_million() {
        let t = BundledTaxTables::load();
        assert_eq!(
            t.table_for(2026).unwrap().gift_lifetime_exclusion,
            dec!(15_000_000)
        );
    }

    #[test]
    fn ty2026_table_is_available() {
        assert!(BundledTaxTables::load().table_for(2026).is_some());
    }

    /// [R0-N1] Bundling TY2026 must not perturb the older tables — spot-check their headline values.
    #[test]
    fn ty2024_and_2025_tables_unchanged() {
        let t = BundledTaxTables::load();

        let t24 = t.table_for(2024).unwrap();
        assert_eq!(
            t24.ordinary_for(FilingStatus::Single).brackets[6].lower,
            dec!(609350)
        );
        assert_eq!(
            *t24.ltcg_for(FilingStatus::Single),
            LtcgBreakpoints {
                max_zero: dec!(47025),
                max_fifteen: dec!(518900)
            }
        );
        assert_eq!(t24.gift_annual_exclusion, dec!(18000));
        assert_eq!(t24.ss_wage_base, dec!(168600));
        assert_eq!(t24.gift_lifetime_exclusion, dec!(13_610_000));

        let t25 = t.table_for(2025).unwrap();
        assert_eq!(
            t25.ordinary_for(FilingStatus::Single).brackets[6].lower,
            dec!(626350)
        );
        assert_eq!(
            *t25.ltcg_for(FilingStatus::Single),
            LtcgBreakpoints {
                max_zero: dec!(48350),
                max_fifteen: dec!(533400)
            }
        );
        assert_eq!(t25.gift_annual_exclusion, dec!(19000));
        assert_eq!(t25.ss_wage_base, dec!(176100));
        assert_eq!(t25.gift_lifetime_exclusion, dec!(13_990_000));
    }

    // ── TY2017 KATs (pre-TCJA; Rev. Proc. 2016-55 §2.01/§2.03; SSA 2016-10-18) ─────────────────────

    /// ★ [tax-critical, R0-r2-M1] TY2017 FULL-SCHEDULE equality lock: every one of the 28 ordinary
    /// bracket edges (4 statuses × 7) AND all 28 rates AND the four §1(h) LTCG pairs AND the
    /// $127,200 SS wage base, asserted by DIRECT equality against arrays transcribed VERBATIM from
    /// Rev. Proc. 2016-55 §2.01 Tables 1–4 / §2.03. A wrong 2017 rate or edge = a wrong 2017 return,
    /// so this is the primary-source gate (a few spot-pins would leave the delta-cancellation hole).
    ///
    /// [R0-r3-Mb] the NIIT threshold is NOT asserted here — it is the year-independent statutory
    /// `niit_threshold()` fn, never a `TaxTable` field.
    #[test]
    fn ty2017_table_matches_rev_proc_2016_55() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2017).unwrap();

        // §2.01 Tables 1–4 — (lower, rate) pairs, verbatim. Historic pre-TCJA rates.
        // Table 3 — Single.
        let single: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(9325), dec!(0.15)),
            (dec!(37950), dec!(0.25)),
            (dec!(91900), dec!(0.28)),
            (dec!(191650), dec!(0.33)),
            (dec!(416700), dec!(0.35)),
            (dec!(418400), dec!(0.396)),
        ];
        // Table 1 — Married Filing Jointly / Qualifying Surviving Spouse.
        let mfj: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(18650), dec!(0.15)),
            (dec!(75900), dec!(0.25)),
            (dec!(153100), dec!(0.28)),
            (dec!(233350), dec!(0.33)),
            (dec!(416700), dec!(0.35)),
            (dec!(470700), dec!(0.396)),
        ];
        // Table 2 — Head of Household.
        let hoh: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(13350), dec!(0.15)),
            (dec!(50800), dec!(0.25)),
            (dec!(131200), dec!(0.28)),
            (dec!(212500), dec!(0.33)),
            (dec!(416700), dec!(0.35)),
            (dec!(444550), dec!(0.396)),
        ];
        // Table 4 — Married Filing Separately.
        let mfs: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(9325), dec!(0.15)),
            (dec!(37950), dec!(0.25)),
            (dec!(76550), dec!(0.28)),
            (dec!(116675), dec!(0.33)),
            (dec!(208350), dec!(0.35)),
            (dec!(235350), dec!(0.396)),
        ];

        for (status, expected) in [
            (FilingStatus::Single, &single),
            (FilingStatus::Mfj, &mfj),
            (FilingStatus::HoH, &hoh),
            (FilingStatus::Mfs, &mfs),
        ] {
            let sched = tt.ordinary_for(status);
            assert_eq!(
                sched.brackets.len(),
                7,
                "{status:?}: TY2017 must have exactly 7 ordinary brackets"
            );
            for (i, (lower, rate)) in expected.iter().enumerate() {
                assert_eq!(
                    sched.brackets[i].lower, *lower,
                    "{status:?} bracket[{i}] lower must match Rev. Proc. 2016-55 §2.01 verbatim"
                );
                assert_eq!(
                    sched.brackets[i].rate, *rate,
                    "{status:?} bracket[{i}] rate must match Rev. Proc. 2016-55 §2.01 verbatim"
                );
            }
        }

        // §2.03 §1(h) LTCG pairs.
        for (status, max_zero, max_fifteen) in [
            (FilingStatus::Single, dec!(37950), dec!(418400)),
            (FilingStatus::Mfj, dec!(75900), dec!(470700)),
            (FilingStatus::HoH, dec!(50800), dec!(444550)),
            (FilingStatus::Mfs, dec!(37950), dec!(235350)),
        ] {
            assert_eq!(
                *tt.ltcg_for(status),
                LtcgBreakpoints {
                    max_zero,
                    max_fifteen
                },
                "{status:?} LTCG pair must match Rev. Proc. 2016-55 §2.03 verbatim"
            );
        }

        // ★ the $127,200 SS wage base (the SE leg's dependency — the reason this table exists).
        assert_eq!(
            tt.ss_wage_base,
            dec!(127200),
            "TY2017 SS wage base must be $127,200 (SSA 2016-10-18)"
        );

        // Qss aliases MFJ (not a stored key).
        assert!(!tt.ordinary.contains_key(&FilingStatus::Qss));
        assert_eq!(
            tt.ordinary_for(FilingStatus::Qss),
            tt.ordinary_for(FilingStatus::Mfj)
        );
    }

    /// TY2017 ancillary indexed fields (Rev. Proc. 2016-55 §2.35 / §2.41).
    #[test]
    fn ty2017_ancillary_fields() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2017).unwrap();
        assert_eq!(tt.gift_annual_exclusion, dec!(14000));
        assert_eq!(tt.gift_lifetime_exclusion, dec!(5_490_000));
        assert_eq!(tt.ss_wage_base, dec!(127200));
    }

    #[test]
    fn ty2017_table_is_available() {
        assert!(BundledTaxTables::load().table_for(2017).is_some());
    }

    // ── TY2024 KATs ──────────────────────────────────────────────────────────────────────────────────

    // ── TY2024 helpers ───────────────────────────────────────────────────────────────────────────────

    fn kat24_eid(n: u64) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(format!("kat24-{n}")))
    }

    fn kat24_lot(n: u64) -> LotId {
        LotId {
            origin_event_id: kat24_eid(n),
            split_sequence: 0,
        }
    }

    /// Minimal 2024 `DisposalLeg` with the given signed gain and term.
    fn leg24(gain: Usd, term: Term) -> DisposalLeg {
        let proceeds = if gain >= dec!(0) { gain } else { dec!(0) };
        let basis = proceeds - gain;
        DisposalLeg {
            lot_id: kat24_lot(1),
            sat: 1,
            proceeds,
            basis,
            gain,
            term,
            basis_source: BasisSource::ExchangeProvided,
            gift_zone: None,
            acquired_at: date!(2024 - 01 - 01),
            wallet: WalletId::Exchange {
                provider: "cb".into(),
                account: "m".into(),
            },
            pseudo: false,
        }
    }

    /// LedgerState with one disposal on 2024-06-15 (so compute_tax_year year=2024 picks it up).
    fn state24_with_legs(legs: Vec<DisposalLeg>) -> LedgerState {
        LedgerState {
            disposals: vec![Disposal {
                event: kat24_eid(0),
                kind: DisposeKind::Sell,
                disposed_at: date!(2024 - 06 - 15),
                legs,
                fee_mini_disposition: false,
            }],
            ..LedgerState::default()
        }
    }

    fn state24_st(gain: Usd) -> LedgerState {
        state24_with_legs(vec![leg24(gain, Term::ShortTerm)])
    }

    fn state24_lt(gain: Usd) -> LedgerState {
        state24_with_legs(vec![leg24(gain, Term::LongTerm)])
    }

    /// Profile factories for TY2024 KATs.  `ord` = ordinary taxable income; `magi` = MAGI excl.
    /// crypto.  Convention: `magi_excluding_crypto = ord` (no non-crypto pref income in these KATs).
    fn p24_single(ord: Usd, magi: Usd) -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: ord,
            magi_excluding_crypto: magi,
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Default::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: Usd::ZERO,
        }
    }

    fn p24_mfj(ord: Usd, magi: Usd) -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: ord,
            magi_excluding_crypto: magi,
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Default::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: Usd::ZERO,
        }
    }

    fn p24_hoh(ord: Usd, magi: Usd) -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::HoH,
            ordinary_taxable_income: ord,
            magi_excluding_crypto: magi,
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Default::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: Usd::ZERO,
        }
    }

    fn p24_mfs(ord: Usd, magi: Usd) -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::Mfs,
            ordinary_taxable_income: ord,
            magi_excluding_crypto: magi,
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Default::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: Usd::ZERO,
        }
    }

    fn computed24(state: LedgerState, profile: TaxProfile) -> btctax_core::TaxResult {
        match compute_tax_year(&[], &state, 2024, Some(&profile), &BundledTaxTables::load()) {
            TaxOutcome::Computed(r) => r,
            TaxOutcome::NotComputable(b) => panic!("unexpected not-computable: {:?}", b),
        }
    }

    // ── KAT-A1 — Single bracket table matches Rev. Proc. 2023-34 §3.01 Table 3 ────────────────────

    #[test]
    fn ty2024_single_ordinary_brackets_match_rev_proc_2023_34() {
        let t = BundledTaxTables::load();
        let s = t
            .table_for(2024)
            .unwrap()
            .ordinary_for(FilingStatus::Single);
        assert_eq!(s.brackets[1].lower, dec!(11600)); // 12% start
        assert_eq!(s.brackets[2].lower, dec!(47150)); // 22% start
        assert_eq!(s.brackets[6].lower, dec!(609350)); // 37% start
        assert_eq!(s.brackets[6].rate, dec!(0.37));
    }

    // ── KAT-A2 — MFS 37% starts at $365,600 (Table 4); MFJ at $731,200 (Table 1) ─────────────────

    #[test]
    fn ty2024_mfs_37_pct_starts_at_365600_and_mfj_at_731200() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2024).unwrap();
        assert_eq!(
            tt.ordinary_for(FilingStatus::Mfs)
                .brackets
                .last()
                .unwrap()
                .lower,
            dec!(365600)
        );
        assert_eq!(
            tt.ordinary_for(FilingStatus::Mfj)
                .brackets
                .last()
                .unwrap()
                .lower,
            dec!(731200)
        );
    }

    // ── KAT-A3 — LTCG breakpoints all statuses — §3.03 ──────────────────────────────────────────────

    #[test]
    fn ty2024_ltcg_breakpoints_all_statuses() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2024).unwrap();
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Single),
            LtcgBreakpoints {
                max_zero: dec!(47025),
                max_fifteen: dec!(518900)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Mfj),
            LtcgBreakpoints {
                max_zero: dec!(94050),
                max_fifteen: dec!(583750)
            }
        );
        // QSS ≡ MFJ (TaxTable::key maps Qss → Mfj)
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Qss),
            LtcgBreakpoints {
                max_zero: dec!(94050),
                max_fifteen: dec!(583750)
            }
        );
        assert_eq!(
            *tt.ltcg_for(FilingStatus::HoH),
            LtcgBreakpoints {
                max_zero: dec!(63000),
                max_fifteen: dec!(551350)
            }
        );
        // MFS max_fifteen = $291,850 (NOT $291,875 = $583,750/2; independent rounding).
        assert_eq!(
            *tt.ltcg_for(FilingStatus::Mfs),
            LtcgBreakpoints {
                max_zero: dec!(47025),
                max_fifteen: dec!(291850)
            }
        );
    }

    // ── KAT-A4 — Ancillary fields — §3.43 / §3.41 / SSA ─────────────────────────────────────────────

    #[test]
    fn ty2024_ancillary_fields() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2024).unwrap();
        assert_eq!(tt.gift_annual_exclusion, dec!(18000));
        assert_eq!(tt.ss_wage_base, dec!(168600));
        assert_eq!(tt.gift_lifetime_exclusion, dec!(13_610_000));
    }

    // ── KAT-A5 — TY2024 now available: `table_for(2024)` returns `Some` ─────────────────────────────

    #[test]
    fn ty2024_table_is_available() {
        assert!(BundledTaxTables::load().table_for(2024).is_some());
    }

    // ── KAT-A6a — Single, 22% bracket entry (§3.01 Table 3; 22% starts at $47,150) ────────────────
    //
    // OTI = $47,150; Crypto ST gain = $1,000; magi_excl = $47,150 (= OTI).
    // WITH: $1,000 falls entirely in 22% band ($47,150 → $48,150 < $100,525).
    //   ord_delta = 22% × $1,000 = $220.00.
    // NIIT: nii_with = 1,000 (ST gain IS NII); MAGI_with = 48,150 < $200,000 → niit = $0.
    // total = $220.00.

    #[test]
    fn ty2024_a6a_single_22pct_bracket_entry() {
        let r = computed24(state24_st(dec!(1000)), p24_single(dec!(47150), dec!(47150)));
        assert_eq!(r.total_federal_tax_attributable, dec!(220.00));
        assert_eq!(r.niit, dec!(0));
    }

    // ── KAT-A6b — MFJ, 22%/24% boundary (§3.01 Table 1; 24% starts at $201,050) ────────────────────
    //
    // OTI = $200,000; Crypto ST gain = $2,000; magi_excl = $200,000.
    // WITH: $1,050 at 22% ($200,000→$201,050), $950 at 24% ($201,050→$202,000).
    //   ord_delta = 22% × $1,050 + 24% × $950 = $231.00 + $228.00 = $459.00.
    // NIIT: MAGI_with = 202,000 < $250,000 MFJ threshold → niit = $0.
    // total = $459.00.

    #[test]
    fn ty2024_a6b_mfj_22_24_boundary() {
        let r = computed24(state24_st(dec!(2000)), p24_mfj(dec!(200000), dec!(200000)));
        assert_eq!(r.total_federal_tax_attributable, dec!(459.00));
        assert_eq!(r.niit, dec!(0));
    }

    // ── KAT-A6c — HoH, 12%/22% boundary (§3.01 Table 2; 22% starts at $63,100) ──────────────────
    //
    // OTI = $63,000; Crypto ST gain = $500; magi_excl = $63,000.
    // WITH: $100 at 12% ($63,000→$63,100), $400 at 22% ($63,100→$63,500).
    //   ord_delta = 12% × $100 + 22% × $400 = $12.00 + $88.00 = $100.00.
    // NIIT: MAGI_with = 63,500 < $200,000 HoH threshold → niit = $0.
    // total = $100.00.

    #[test]
    fn ty2024_a6c_hoh_12_22_boundary() {
        let r = computed24(state24_st(dec!(500)), p24_hoh(dec!(63000), dec!(63000)));
        assert_eq!(r.total_federal_tax_attributable, dec!(100.00));
        assert_eq!(r.niit, dec!(0));
    }

    // ── KAT-A6d — MFS, 35%/37% boundary with NIIT (§3.01 Table 4; 37% starts at $365,600) ────────
    //
    // OTI = $365,000; Crypto ST gain = $1,000; magi_excl = $365,000.
    // WITH: $600 at 35% ($365,000→$365,600), $400 at 37% ($365,600→$366,000).
    //   ord_delta = 35% × $600 + 37% × $400 = $210.00 + $148.00 = $358.00.
    // NIIT [R0-C1]: ST gain IS NII (§1411(c)(1)(A)(iii)); MFS threshold $125,000:
    //   nii_with = 1,000; MAGI_with = 366,000 > 125,000; base = min(1,000, 241,000) = 1,000.
    //   niit = 3.8% × 1,000 = $38.00.
    // total = $358.00 + $0 (ltcg) + $38.00 = $396.00.

    #[test]
    fn ty2024_a6d_mfs_35_37_boundary_with_niit() {
        let r = computed24(state24_st(dec!(1000)), p24_mfs(dec!(365000), dec!(365000)));
        assert_eq!(r.total_federal_tax_attributable, dec!(396.00));
        assert_eq!(r.niit, dec!(38.00));
    }

    // ── KAT-A7 — TY2024 LTCG threshold KAT, Single crossing 0%→15% ─────────────────────────────────
    //
    // OTI = $40,000; Crypto LT gain = $10,000; magi_excl = $40,000.
    // TY2024 Single: max_zero = $47,025; max_fifteen = $518,900.
    //   at_0  = 47,025 − 40,000 = $7,025 → 0%
    //   at_15 = 10,000 − 7,025  = $2,975 → 15%
    //   ltcg_tax = 2,975 × 0.15 = $446.25.
    // NIIT: MAGI_with = 50,000 < $200,000 → $0.
    // total = $446.25.

    #[test]
    fn ty2024_a7_single_ltcg_0_to_15_threshold() {
        let r = computed24(
            state24_lt(dec!(10000)),
            p24_single(dec!(40000), dec!(40000)),
        );
        assert_eq!(r.ltcg_tax, dec!(446.25));
        assert_eq!(r.total_federal_tax_attributable, dec!(446.25));
        assert_eq!(r.niit, dec!(0));
    }

    // ── KAT-A8 — TY2024 FULL-SCHEDULE equality lock (burndown-3 D5, closing TY2024 M1) ─────────────
    //
    // The exhaustive lock: every one of the 28 ordinary bracket edges (4 statuses × 7) and all 28
    // rates asserted by DIRECT equality against arrays transcribed VERBATIM from
    // design/SPEC_ty2024_tables.md §3.01 Tables 1–4 (triple-verified against Rev. Proc. 2023-34 /
    // IRB 2023-48 — transcribed, never re-derived), plus the four §3.03 LTCG pairs (re-asserting
    // KAT-A3 so this single test is the complete TY2024 schedule lock).
    //
    // Why it exists: A1/A2 pin only a handful of edges, and the A6a–A6d compute KATs assert
    // marginal DELTAS, which a lower-edge transposition can cancel (the M1 delta-cancellation
    // hole). Direct full-schedule equality closes it. A1–A7 remain as the readable spot-checks.

    #[test]
    fn ty2024_full_schedule_equality_all_28_edges_and_ltcg() {
        let t = BundledTaxTables::load();
        let tt = t.table_for(2024).unwrap();

        // §3.01 Tables 1–4 — (lower, rate) pairs, verbatim from SPEC_ty2024_tables.md.
        // Table 3 — §1(j)(2)(C): Unmarried Individuals (Single)
        let single: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(11600), dec!(0.12)),
            (dec!(47150), dec!(0.22)),
            (dec!(100525), dec!(0.24)),
            (dec!(191950), dec!(0.32)),
            (dec!(243725), dec!(0.35)),
            (dec!(609350), dec!(0.37)),
        ];
        // Table 1 — §1(j)(2)(A): Married Filing Jointly / Qualifying Surviving Spouse
        let mfj: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(23200), dec!(0.12)),
            (dec!(94300), dec!(0.22)),
            (dec!(201050), dec!(0.24)),
            (dec!(383900), dec!(0.32)),
            (dec!(487450), dec!(0.35)),
            (dec!(731200), dec!(0.37)),
        ];
        // Table 2 — §1(j)(2)(B): Head of Household (35% starts $243,700 — NOT Single/MFS $243,725)
        let hoh: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(16550), dec!(0.12)),
            (dec!(63100), dec!(0.22)),
            (dec!(100500), dec!(0.24)),
            (dec!(191950), dec!(0.32)),
            (dec!(243700), dec!(0.35)),
            (dec!(609350), dec!(0.37)),
        ];
        // Table 4 — §1(j)(2)(D): Married Filing Separately (37% starts $365,600)
        let mfs: [(Usd, Usd); 7] = [
            (dec!(0), dec!(0.10)),
            (dec!(11600), dec!(0.12)),
            (dec!(47150), dec!(0.22)),
            (dec!(100525), dec!(0.24)),
            (dec!(191950), dec!(0.32)),
            (dec!(243725), dec!(0.35)),
            (dec!(365600), dec!(0.37)),
        ];

        for (status, expected) in [
            (FilingStatus::Single, &single),
            (FilingStatus::Mfj, &mfj),
            (FilingStatus::HoH, &hoh),
            (FilingStatus::Mfs, &mfs),
        ] {
            let sched = tt.ordinary_for(status);
            assert_eq!(
                sched.brackets.len(),
                7,
                "{status:?}: TY2024 must have exactly 7 ordinary brackets"
            );
            for (i, (lower, rate)) in expected.iter().enumerate() {
                assert_eq!(
                    sched.brackets[i].lower, *lower,
                    "{status:?} bracket[{i}] lower must match Rev. Proc. 2023-34 §3.01 verbatim"
                );
                assert_eq!(
                    sched.brackets[i].rate, *rate,
                    "{status:?} bracket[{i}] rate must match Rev. Proc. 2023-34 §3.01 verbatim"
                );
            }
        }

        // §3.03 LTCG pairs (re-asserting KAT-A3 for a single self-contained full-schedule lock).
        for (status, max_zero, max_fifteen) in [
            (FilingStatus::Single, dec!(47025), dec!(518900)),
            (FilingStatus::Mfj, dec!(94050), dec!(583750)),
            (FilingStatus::HoH, dec!(63000), dec!(551350)),
            // MFS max_fifteen = $291,850 (NOT $291,875 = $583,750/2; independent rounding).
            (FilingStatus::Mfs, dec!(47025), dec!(291850)),
        ] {
            assert_eq!(
                *tt.ltcg_for(status),
                LtcgBreakpoints {
                    max_zero,
                    max_fifteen
                },
                "{status:?} LTCG pair must match Rev. Proc. 2023-34 §3.03 verbatim"
            );
        }

        // [R0-N3] Qss is NOT a stored key in either map (it aliases MFJ via TaxTable::key) —
        // direct assertions on the pub fields, plus the alias check.
        assert!(
            !tt.ordinary.contains_key(&FilingStatus::Qss),
            "Qss must not be a stored ordinary key (aliases Mfj at lookup)"
        );
        assert!(
            !tt.ltcg.contains_key(&FilingStatus::Qss),
            "Qss must not be a stored ltcg key (aliases Mfj at lookup)"
        );
        assert_eq!(
            tt.ordinary_for(FilingStatus::Qss),
            tt.ordinary_for(FilingStatus::Mfj),
            "Qss ordinary lookup must alias the MFJ schedule"
        );
    }
}
