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
use rust_decimal::Decimal;
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
    /// §2503(b) gift-tax **annual exclusion per donee**. INDEXED — inflation-adjusted under
    /// §2503(b)(2). TY2025 = $19,000 (**Rev. Proc. 2024-40 §2.43** — NOT §2.01/§2.03). Feeds the
    /// standalone Form 709 over-annual-exclusion advisory only; does NOT feed engine B /
    /// `compute_tax_year`. Belongs in the per-year table (not a `tables.rs` statutory constant)
    /// precisely because it moves year-over-year.
    pub gift_annual_exclusion: Usd,
    /// Social Security **contribution and benefit base** (the SE-tax OASDI wage base) for the year.
    /// INDEXED — wage-indexed under **§230 of the Social Security Act** (42 U.S.C. §430); announced
    /// annually by SSA. TY2025 = $176,100 (SSA 2024-10-10). Caps the 12.4% Social-Security portion of
    /// the §1401(a) SE tax (`ss = 12.4% × min(net SE earnings, ss_wage_base − W-2 SS wages)`). Feeds the
    /// standalone Schedule SE §1401 figure only; does NOT feed engine B / `compute_tax_year`. Belongs in
    /// the per-year table (not a `tables.rs` statutory constant) precisely because it moves year-over-year.
    pub ss_wage_base: Usd,
    /// §2010(c)(3) **basic exclusion amount** (the unified-credit equivalent for gift/estate tax).
    /// INDEXED — inflation-adjusted under §2010(c)(3) and announced annually via Rev. Proc.
    /// TY2025 = $13,990,000 (**Rev. Proc. 2024-40 §2.41**). Feeds the standalone §2505 lifetime-
    /// exclusion consumption advisory only; does NOT feed engine B / `compute_tax_year`. Belongs in
    /// the per-year table (not a `tables.rs` statutory constant) precisely because it moves
    /// year-over-year.
    pub gift_lifetime_exclusion: Usd,
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

/// §3101(a): the employee-share Social Security (OASDI) tax rate.
/// **STATUTORY** — 26 U.S.C. §3101(a).  Fixed in the Code; NOT inflation-indexed.
/// Value: 6.2% = 0.062 (exact Decimal; never a float, NFR5).  The §6413(c) excess-SS credit maximum
/// per person is `EMPLOYEE_OASDI_RATE × ss_wage_base` (the year-indexed base lives in `TaxTable`).
pub const EMPLOYEE_OASDI_RATE: Usd = dec!(0.062);

/// §1401(a): the Social Security (OASDI) portion of the self-employment tax rate.
/// **STATUTORY** — 26 U.S.C. §1401(a).  Fixed in the Code; NOT inflation-indexed.
/// Value: 12.4% = 0.124 (exact Decimal; never a float, NFR5).  Applies to net SE earnings up to the
/// year-indexed SS wage base (`TaxTable::ss_wage_base`, less any W-2 SS wages).
pub const SE_RATE_SS: Usd = dec!(0.124);

/// §1401(b): the Medicare (HI) portion of the self-employment tax rate.
/// **STATUTORY** — 26 U.S.C. §1401(b)(1).  Fixed in the Code; NOT inflation-indexed.
/// Value: 2.9% = 0.029 (exact Decimal; never a float, NFR5).  Uncapped (no wage-base ceiling).
pub const SE_RATE_MEDICARE: Usd = dec!(0.029);

/// §1401(b)(2): the Additional Medicare Tax rate on high self-employment income.
/// **STATUTORY** — 26 U.S.C. §1401(b)(2)(A).  Fixed in the Code; NOT inflation-indexed.
/// Value: 0.9% = 0.009 (exact Decimal; never a float, NFR5).  Applies to net SE earnings above the
/// `se_addl_medicare_threshold(status)`.  Per §164(f)(1) it is EXCLUDED from the one-half-SE-tax
/// above-the-line deduction (a Form 8959 item — Schedule SE line 13 counts SS + regular Medicare only).
pub const SE_RATE_ADDL_MEDICARE: Usd = dec!(0.009);

/// §1402(a): net-earnings-from-self-employment factor (1 − 7.65%).
/// **STATUTORY** — 26 U.S.C. §1402(a)(12).  Fixed in the Code; NOT inflation-indexed.
/// Value: 92.35% = 0.9235 (exact Decimal; never a float, NFR5).  Net SE earnings = Schedule C net
/// income × this factor; the SE-tax rates above are applied to that product.
pub const SE_NET_EARNINGS_FACTOR: Usd = dec!(0.9235);

/// §1401(b)(2): the net-SE-earnings threshold above which the 0.9% Additional Medicare Tax applies (also
/// Form 8959 Part I/II). **STATUTORY** — 26 U.S.C. §1401(b)(2)(A)/§3101(b)(2).  The dollar amounts are
/// fixed in the Code and do NOT move year-over-year.  Must never be placed in a `TaxTable`.
///
/// Thresholds per filing status:
/// - MFJ: $250,000  (§1401(b)(2)(A)(i) — "in the case of a joint return")
/// - MFS: $125,000  (§1401(b)(2)(A)(ii))
/// - Single / HoH / **QSS**: $200,000  (§1401(b)(2)(A)(iii) — "in any other case"). A **qualifying
///   surviving spouse is NOT a joint return**, so it takes the $200,000 amount, NOT MFJ's $250,000 — the
///   2024 Form 8959 chart / Schedule 2 L11 instructions confirm "single, head of household, or QSS —
///   $200,000" (Fable IMPL-P4 r1 C1). This DIFFERS from [`niit_threshold`], where §1411(b)(1) expressly
///   *includes* "a surviving spouse" at $250,000 — the two statutes disagree on QSS, deliberately.
pub fn se_addl_medicare_threshold(status: FilingStatus) -> Usd {
    match status {
        FilingStatus::Mfj => dec!(250000),
        FilingStatus::Mfs => dec!(125000),
        FilingStatus::Single | FilingStatus::HoH | FilingStatus::Qss => dec!(200000),
    }
}

/// §170(f)(11)(C): qualified-appraisal threshold for charitable contributions of property.
/// **STATUTORY** — 26 U.S.C. §170(f)(11)(C).  Fixed in the Code; NOT inflation-indexed.
/// Value: $5,000 (exact Decimal; never a float, NFR5).
/// Must never be placed in a `TaxTable`.
pub const QUALIFIED_APPRAISAL_THRESHOLD: Usd = dec!(5000);

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

// ── Full-return per-year parameters (INDEXED; NEW) ──────────────────────────────────────────────

/// Full-return v1 per-year parameters: the standard deduction and the year-varying limits the absolute
/// 1040 needs. **NEW for the full-return build.** These values are INDEXED — they move year-over-year
/// (OBBBA moved the SALT cap; §1(g)/§402(g)/§63 amounts are inflation-adjusted) — so they belong in a
/// per-year table, not as year-independent statutory constants. Bundled in `btctax-adapters` (TY2024 for v1).
///
/// **Kept OUT of [`TaxTable`] as a deliberate design choice** (a documented deviation from SPEC §8, which
/// suggested `TaxTable` — see `design/full-return/FOLLOWUPS.md`): (1) `TaxTable` is a **published-crate API**
/// (btctax-core on crates.io) read by the crypto-**delta** path, which never needs these fields; a separate
/// table keeps that surface stable and the full-return data isolated. (2) v1 bundles these for **TY2024
/// only**, so a separate table with **fail-closed per-year gating** (`None` ⇒ `NotComputable`) has the
/// smallest blast radius. This does NOT rely on any frozen-file constraint (`se.rs` only *calls* the
/// unfrozen `synthetic_table`, so `TaxTable` could technically gain a field).
/// §55(d)/§55(b)(1) AMT amounts for the 2024 "Worksheet To See if You Should Fill in Form 6251"
/// (SPEC §4.11). All INDEXED (§55(d)(4) inflation adjustment). Grouped by the worksheet's
/// (differing) filing-status bucketings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmtParams {
    /// §55(d)(1) exemption — Single / HoH (worksheet line 6).
    pub exemption_single_hoh: Usd,
    /// §55(d)(1) exemption — MFJ / QSS.
    pub exemption_mfj_qss: Usd,
    /// §55(d)(1) exemption — MFS.
    pub exemption_mfs: Usd,
    /// §55(d)(3) exemption phase-out start — Single / HoH / **MFS** (worksheet line 8 groups MFS here).
    pub phaseout_start_single_hoh_mfs: Usd,
    /// §55(d)(3) phase-out start — MFJ / QSS.
    pub phaseout_start_mfj_qss: Usd,
    /// §55(b)(1) 26%/28% breakpoint — general (worksheet line 12).
    pub breakpoint_28pct: Usd,
    /// §55(b)(1) 26%/28% breakpoint — MFS.
    pub breakpoint_28pct_mfs: Usd,
    /// **Form 6251 line 4 / i6251 p.9** — the MFS AMTI add-back threshold: *"If your filing status is
    /// married filing separately and line 4 is more than $875,950, you must include an additional
    /// amount on line 4."* §55(d)(3).
    pub mfs_kicker_start: Usd,
    /// **i6251 p.9** — the MFS add-back cap: *"If line 4 is $1,142,550 or more, include an additional
    /// $66,650. Otherwise, include 25% of the excess of the amount on line 4 over $875,950."*
    pub mfs_kicker_max: Usd,
    /// §55(d)(3) **exemption phase-out** rate (Exemption Worksheet line 5: *"Multiply line 4 by 25%
    /// (0.25)"*).
    ///
    /// ★ SPLIT FROM `mfs_kicker_rate`, which was the same field until 2026-07-29. These are two
    /// different rules that happen to share a rate: the Exemption Worksheet reduces the exemption,
    /// and Form 6251 line 4 *increases AMTI*. Both are 25% in TY2024 and TY2025, so one field was
    /// invisible and green — and the TY2026 draft form's own arithmetic implies this one moves to
    /// **50%** (500,000 + 2 × 70,100 = 640,200, the published MFS kicker start) while nothing says
    /// the kicker's rate follows. One field standing for two form rules is the compression
    /// `CLAUDE.md` forbids; here it would have printed a wrong number on a **signed** form with
    /// nothing reding.
    pub exemption_phaseout_rate: Usd,
    /// **i6251 p.9** — the rate in the MFS AMTI add-back: *"include **25% of the excess** of the
    /// amount on line 4 over $875,950."* §55(d)(3)'s flush sentence. Distinct from
    /// [`Self::exemption_phaseout_rate`]; see the note there.
    pub mfs_kicker_rate: Usd,
    /// §55(b)(1)(A) lower AMT rate (Form 6251 lines 7/18/39: *"multiply … by 26% (0.26)"*).
    pub rate_26: Usd,
    /// §55(b)(1)(B) upper AMT rate (Form 6251 lines 7/18/39: *"multiply … by 28% (0.28)"*).
    pub rate_28: Usd,
    /// The §55(b)(1) 28%-bracket subtrahend — general (Form 6251 lines 7/18/39: *"subtract $4,652"*).
    pub rate_28_subtrahend: Usd,
    /// The 28%-bracket subtrahend — MFS (*"$2,326 if married filing separately"*).
    pub rate_28_subtrahend_mfs: Usd,
}

/// §164(b) SALT limitation, transcribed per year because the two years are different instruments.
///
/// ★ **Why an enum and not four numbers.** TY2024's Schedule A line 5e is a bare cap. OBBBA turned
/// TY2025's into a **10-line worksheet** with a phase-down, a floor, and — the part a parameterisation
/// gets wrong — an MFS halving that happens at *line 10 only*, after the floor rather than on the
/// constants. Encoding the 2025 rule as "halve the cap and halve the floor" gives a different answer:
/// at MFS / MAGI $300,000 the worksheet pays **$12,500** and the halved-constants shape pays $5,000.
/// This project has already made that exact mistake twice in the spec; the enum makes it unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SaltLimitation {
    /// **TY2024 Schedule A line 5e, verbatim:** "Enter the smaller of line 5d or $10,000 ($5,000 if
    /// married filing separately)." No worksheet exists for this year — the form asks one question.
    FlatCap {
        /// $10,000 — the general cap.
        cap: Usd,
        /// $5,000 — MFS. A separate field rather than `cap / 2`, because the form prints both numbers
        /// and a future year could break the relationship.
        cap_mfs: Usd,
    },
    /// **TY2025: the "State and Local Tax Deduction Worksheet"** in the Schedule A instructions,
    /// line by line. Field names are the worksheet's own line numbers.
    Worksheet2025 {
        /// Worksheet line 1 — "…**Yes.** Enter $40,000". ★ NOT halved for MFS; see `line_10_mfs_halves`.
        line1_cap: Usd,
        /// Worksheet line 5 — "Enter $500,000 ($250,000 if married filing separately)". The one
        /// genuinely per-status constant in the worksheet.
        line5_threshold: Usd,
        /// Worksheet line 5, MFS — $250,000.
        line5_threshold_mfs: Usd,
        /// Worksheet line 7 — "Multiply line 6 by 30% (0.30)".
        line7_rate: Usd,
        /// Worksheet line 9 — "Enter the **larger** of the amount on line 8 or $10,000". ★ NOT halved.
        line9_floor: Usd,
        /// Worksheet line 1's short-circuit — "Is the amount on Schedule A, line 5d more than $10,000
        /// ($5,000 if married filing separately)? **No.** Your deduction isn't limited."
        line1_trigger: Usd,
        /// Worksheet line 1's short-circuit, MFS — $5,000.
        line1_trigger_mfs: Usd,
        /// Worksheet line 10 — "…the smaller of the amount on line 9 (**half** the amount on line 9 if
        /// married filing separately) or the amount from Schedule A, line 5d". `true` records that the
        /// halving lives HERE and nowhere else.
        line_10_mfs_halves: bool,
    },
}

impl SaltLimitation {
    /// Schedule A **line 5e** — the deductible SALT.
    ///
    /// `line_5d` is the SALT actually paid; `magi` is the §164(b)(7)(B)(iv) modified AGI (AGI plus any
    /// §911/931/933 exclusion), the *same* quantity Schedule 1-A Part I line 3 computes.
    ///
    /// ★ **`magi: None` means "the filer was never asked", and it FAILS CLOSED.** `assemble_absolute`
    /// is infallible by design — btctax computes the whole return even for one it will not file — so
    /// this cannot refuse here. `screen_inputs` refuses first (`QuestionId::HasIncomeExclusion` is live
    /// from TY2025 — §G-15 year-scoped it; TY2024's `FlatCap` ignores `magi` entirely, so the
    /// narrower gate cannot move a TY2024 figure), and this arm is the belt to that brace. Its DIRECTION is what matters: an absent
    /// MAGI is treated as *fully phased down*, giving the floor — the SMALLEST deduction, which can
    /// only overstate tax. The §63(f) rule, applied: an unknown must fail in the direction that
    /// cannot understate. Note `FlatCap` ignores `magi` entirely, so TY2024 is untouched either way.
    pub fn line_5e(&self, line_5d: Usd, magi: Option<Usd>, status: FilingStatus) -> Usd {
        let mfs = status == FilingStatus::Mfs;
        match self {
            Self::FlatCap { cap, cap_mfs } => line_5d.min(if mfs { *cap_mfs } else { *cap }),
            Self::Worksheet2025 {
                line1_cap,
                line5_threshold,
                line5_threshold_mfs,
                line7_rate,
                line9_floor,
                line1_trigger,
                line1_trigger_mfs,
                line_10_mfs_halves,
            } => {
                // Line 1 — "Is 5d more than $10,000 ($5,000 MFS)? No ⇒ STOP, not limited."
                let trigger = if mfs {
                    *line1_trigger_mfs
                } else {
                    *line1_trigger
                };
                if line_5d <= trigger {
                    return line_5d;
                }
                let line1 = *line1_cap;
                // Lines 4/5/6 — MAGI against the threshold. An UNASKED MAGI collapses to the
                // floor: the smallest deduction, which cannot understate tax (see the doc comment).
                let line9 = match magi {
                    None => *line9_floor,
                    Some(magi) => {
                        let line5 = if mfs {
                            *line5_threshold_mfs
                        } else {
                            *line5_threshold
                        };
                        let line6 = (magi - line5).max(Usd::ZERO);
                        // Line 7, then line 8 = line 1 − line 7.
                        let line7 = *line7_rate * line6;
                        let line8 = line1 - line7;
                        // Line 9 — "the LARGER of line 8 or $10,000" (unhalved).
                        line8.max(*line9_floor)
                    }
                };
                // Line 10 — half of line 9 for MFS, then the smaller of that and 5d.
                let line9_for_status = if mfs && *line_10_mfs_halves {
                    line9 / Decimal::from(2)
                } else {
                    line9
                };
                line9_for_status.min(line_5d)
            }
        }
    }
}

impl AmtParams {
    /// §55(d)(1) AMT exemption for `status` (worksheet line 6).
    pub fn exemption(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj | FilingStatus::Qss => self.exemption_mfj_qss,
            FilingStatus::Mfs => self.exemption_mfs,
            FilingStatus::Single | FilingStatus::HoH => self.exemption_single_hoh,
        }
    }
    /// §55(d)(3) exemption phase-out start for `status` (worksheet line 8; MFS groups with unmarried).
    pub fn phaseout_start(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj | FilingStatus::Qss => self.phaseout_start_mfj_qss,
            _ => self.phaseout_start_single_hoh_mfs,
        }
    }
    /// §55(b)(1) 26%/28% breakpoint for `status` (worksheet line 12; MFS is halved).
    pub fn breakpoint_28pct(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfs => self.breakpoint_28pct_mfs,
            _ => self.breakpoint_28pct,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullReturnParams {
    pub year: i32,
    /// §63(c)(2) basic standard deduction, keyed by filing status (Qss→Mfj via [`std_deduction_for`]).
    pub std_deduction: BTreeMap<FilingStatus, Usd>,
    /// §63(f) additional standard deduction per aged (65+) / blind box — married (MFJ/MFS/QSS).
    pub std_aged_blind_married: Usd,
    /// §63(f) additional standard deduction per aged/blind box — unmarried (Single/HoH).
    pub std_aged_blind_unmarried: Usd,
    /// §63(c)(5) dependent standard-deduction floor.
    pub dependent_std_floor: Usd,
    /// §63(c)(5) dependent earned-income add-on ($450).
    pub dependent_std_earned_addon: Usd,
    /// §164(b) state-and-local-tax limitation. **Per-year INSTRUMENT, not a per-year constant** — the
    /// 2024 and 2025 Schedule A ask different questions, so [`SaltLimitation`] is an enum and the
    /// compiler forces every consumer to handle both.
    pub salt: SaltLimitation,
    /// §1(g)(4) kiddie-tax unearned-income threshold (Form 8615 refuse trigger, spec C1).
    pub kiddie_unearned_threshold: Usd,
    /// §402(g)(1) elective-deferral limit (excess-deferral refuse trigger, spec F3).
    pub elective_deferral_limit: Usd,
    /// §904(j) no-Form-1116 foreign-tax-credit ceiling (general; MFJ = double at the use site).
    pub ftc_ceiling: Usd,
    /// §199A(e)(2) taxable-income-before-QBI threshold — **unmarried base** (Single/HoH/MFS/QSS). At or
    /// below this the simplified Form 8995 path applies; above it the 8995-A phase-in (unmodeled in v1)
    /// is required, so QBI **refuses** (SPEC §4.5). TY2024 = $191,950.
    pub qbi_ti_threshold_unmarried: Usd,
    /// §199A(e)(2) threshold — **MFJ** (200% of the base, §199A(e)(2)(B)). TY2024 = $383,900. A QSS is
    /// NOT a joint return, so it uses the unmarried base (the lower threshold refuses sooner — the
    /// fail-closed direction; mirrors the §904(j) FTC ceiling / §221 student-loan QSS treatment).
    pub qbi_ti_threshold_married: Usd,
    /// **Form 8995-A line 23 — the phase-in range WIDTH**, unmarried base. TY2024 = **$50,000**.
    ///
    /// > *"Phase-in range. Enter $50,000 ($100,000 if married filing jointly)"*
    ///
    /// ★★★ **STATUTORY, AND NOT INFLATION-INDEXED** (§199A(b)(3)(B)(ii)(II) / §199A(d)(3)(A)) — unlike
    /// the threshold immediately above it, which IS indexed (§199A(e)(2)(B), republished every year in
    /// the Rev. Proc.). A table update that "indexes" this alongside its neighbour is wrong, and the
    /// invariant below is what catches it.
    ///
    /// ★★ **DO NOT COPY THE REV. PROC.'s "PHASE-IN RANGE AMOUNT" INTO THIS FIELD.** Rev. Proc. 2023-34
    /// §.27 publishes a two-column table whose second column is headed *"Phase-in range amount"* and
    /// reads **$483,900** (MFJ) / **$241,950** (all others). That is the **TOP OF THE RANGE**
    /// (threshold + width), not the width the form asks for. Substituting it puts $483,900 where
    /// $100,000 belongs — off by roughly 5× — and it **cross-foots perfectly**, because line 24 merely
    /// divides by it: the phase-in percentage collapses toward zero, the deduction is barely reduced,
    /// and the tax is UNDERSTATED. Nothing downstream would look wrong.
    pub qbi_phase_in_range_unmarried: Usd,
    /// Form 8995-A line 23 — the phase-in range width, **MFJ**. TY2024 = **$100,000** (200% of the
    /// base). Same statutory/not-indexed caveat as [`Self::qbi_phase_in_range_unmarried`].
    pub qbi_phase_in_range_married: Usd,
    /// §221(b)(2) student-loan-interest deduction MAGI phase-out `(start, end)` — unmarried (Single/HoH).
    pub student_loan_phaseout_unmarried: (Usd, Usd),
    /// §221(b)(2) phase-out `(start, end)` — MFJ/QSS. MFS gets **no** deduction (§221(e)(2)), so no range.
    pub student_loan_phaseout_married: (Usd, Usd),
    /// §55(d)/§55(b)(1) AMT amounts for Form 6251 and its screening worksheet (SPEC §4.11).
    pub amt: AmtParams,
}

impl FullReturnParams {
    /// §63(c)(2) basic standard deduction for `status` (maps `Qss → Mfj`).
    pub fn std_deduction_for(&self, status: FilingStatus) -> Usd {
        self.std_deduction[&TaxTable::key(status)]
    }

    /// §199A(e)(2) taxable-income-before-QBI threshold for `status` (MFJ doubles; QSS uses the
    /// unmarried base — QSS is not a joint return, and the lower threshold is the fail-closed direction
    /// for the QBI refuse, matching this crate's §904(j)/§221 QSS-≠-joint convention).
    pub fn qbi_ti_threshold(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj => self.qbi_ti_threshold_married,
            _ => self.qbi_ti_threshold_unmarried,
        }
    }

    /// Form 8995-A line 23 — the phase-in range WIDTH for `status`. Follows the threshold's own
    /// MFJ-doubles / QSS-uses-the-unmarried-base convention, so the two always pair.
    pub fn qbi_phase_in_range(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj => self.qbi_phase_in_range_married,
            _ => self.qbi_phase_in_range_unmarried,
        }
    }

    /// The **top** of the §199A phase-in range — the figure Rev. Proc. 2023-34 §.27 publishes under
    /// *"Phase-in range amount"*.
    ///
    /// ★★★ This exists so the invariant is CHECKABLE against the primary source: `threshold + width`
    /// must equal the published amount. It ties a STATUTORY width to an INDEXED threshold through the
    /// Rev. Proc.'s own number, so "indexing" the width — or pasting the published amount into the
    /// width field — breaks an equality rather than sailing through as a plausible figure.
    pub fn qbi_phase_in_top(&self, status: FilingStatus) -> Usd {
        self.qbi_ti_threshold(status) + self.qbi_phase_in_range(status)
    }

    /// §221 student-loan-interest MAGI phase-out `(start, end)` for `status`; `None` for **MFS**
    /// (§221(e)(2): a separate filer gets no deduction). §221(b)(2)(B) doubles the floor **only "in the
    /// case of a joint return"** — MFJ only. A **QSS is NOT a joint return** (Pub 970 ch. 4 / the Sch 1
    /// worksheet group "single, HoH, or qualifying surviving spouse" at $80k–$95k), so it takes the
    /// UNMARRIED range — same QSS-≠-joint distinction this crate makes for the §904(j) FTC ceiling. (This
    /// differs from §63(c)(2) std deduction, where "surviving spouse" IS in the joint bucket — `Qss → Mfj`.)
    pub fn student_loan_phaseout(&self, status: FilingStatus) -> Option<(Usd, Usd)> {
        match status {
            FilingStatus::Mfs => None,
            FilingStatus::Mfj => Some(self.student_loan_phaseout_married),
            FilingStatus::Single | FilingStatus::HoH | FilingStatus::Qss => {
                Some(self.student_loan_phaseout_unmarried)
            }
        }
    }
}

/// Lookup for the per-year [`FullReturnParams`]. Bundled impl in `btctax-adapters` (TY2024 for v1);
/// a year without full-return params returns `None` → the caller fails closed (`NotComputable`).
pub trait FullReturnTables {
    fn full_return_for(&self, year: i32) -> Option<&FullReturnParams>;
}

impl FullReturnTables for BTreeMap<i32, FullReturnParams> {
    fn full_return_for(&self, year: i32) -> Option<&FullReturnParams> {
        self.get(&year)
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
        // Hand-chosen synthetic value (NOT a real IRS figure — real numbers come from
        // BundledTaxTables); happens to equal the TY2025 §2503(b) exclusion for convenience.
        gift_annual_exclusion: dec!(19000),
        // Hand-chosen synthetic SS wage base (happens to equal the real TY2025 §230 figure).
        ss_wage_base: dec!(176100),
        // Hand-chosen synthetic lifetime exclusion (happens to equal the real TY2025 §2010(c)(3)
        // figure per Rev. Proc. 2024-40 §2.41).
        gift_lifetime_exclusion: dec!(13_990_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// The TY2025 §164(b) worksheet, as its own instrument — cap $40,000, 30% phase-down over
    /// $500,000 ($250,000 MFS), $10,000 floor, MFS halving at **line 10 only**.
    fn salt_2025() -> SaltLimitation {
        SaltLimitation::Worksheet2025 {
            line1_cap: dec!(40000),
            line5_threshold: dec!(500000),
            line5_threshold_mfs: dec!(250000),
            line7_rate: dec!(0.30),
            line9_floor: dec!(10000),
            line1_trigger: dec!(10000),
            line1_trigger_mfs: dec!(5000),
            line_10_mfs_halves: true,
        }
    }

    /// ★ THE MFS CASE IS THE WHOLE POINT, and it is why this is a worksheet and not four constants.
    ///
    /// The TY2025 worksheet halves at **line 10** — after the floor — not on the cap and floor
    /// themselves. Parameterising it as "halve every constant" (cap $20,000, floor $5,000) is what
    /// Tax-Calculator does (`ID_AllTaxes_c[mseparate] = 20000`), and it gives a DIFFERENT answer:
    ///
    /// | MFS, 5d $30,000 | worksheet | halved-constants |
    /// |---|---|---|
    /// | MAGI $300,000 | **$12,500** | $5,000 |
    ///
    /// $300,000 is the point of maximum divergence between the two shapes, and the halved-constants
    /// version also puts the floor at MAGI $300,000 when the worksheet reaches it at **$350,000**.
    /// This project encoded the wrong shape twice in the TY2025 spec before a review caught it; the
    /// numbers below come from the archived `i1040sca--2025.pdf` worksheet, computed line by line.
    #[test]
    fn ty2025_salt_worksheet_halves_at_line_10_not_on_the_constants() {
        let w = salt_2025();
        let mfs = FilingStatus::Mfs;

        // L6=50,000 → L7=15,000 → L8=25,000 → L9=25,000 → halved 12,500 → min(12,500, 30,000)
        assert_eq!(
            w.line_5e(dec!(30000), Some(dec!(300000)), mfs),
            dec!(12500),
            "the worksheet halves line 9, so MFS at MAGI 300,000 deducts 12,500 — not the 5,000 a \
             halved-cap/halved-floor parameterisation gives"
        );
        // The MFS floor is reached where L8 falls to the UNHALVED 10,000: MAGI = 250,000 + 30,000/0.30
        assert_eq!(
            w.line_5e(dec!(30000), Some(dec!(350000)), mfs),
            dec!(5000),
            "MFS floor: L8 = 40,000 − 0.30×100,000 = 10,000, L9 = 10,000, halved = 5,000"
        );
        assert!(
            w.line_5e(dec!(30000), Some(dec!(349999)), mfs) > dec!(5000),
            "just below MAGI 350,000 the MFS floor must NOT yet bind"
        );
    }

    /// Every region of the TY2025 worksheet, single filer, from the archived instructions.
    #[test]
    fn ty2025_salt_worksheet_covers_every_region() {
        let w = salt_2025();
        let s = FilingStatus::Single;
        // Line 1's own short-circuit: "Is 5d more than $10,000? No ⇒ your deduction isn't limited."
        assert_eq!(w.line_5e(dec!(7000), Some(dec!(450000)), s), dec!(7000));
        // Under the cap, under the threshold ⇒ all of it.
        assert_eq!(w.line_5e(dec!(30000), Some(dec!(450000)), s), dec!(30000));
        // Over the cap, under the threshold ⇒ the $40,000 cap.
        assert_eq!(w.line_5e(dec!(60000), Some(dec!(450000)), s), dec!(40000));
        // Phasing: L6=100,000 → L7=30,000 → L8=10,000.
        assert_eq!(w.line_5e(dec!(60000), Some(dec!(600000)), s), dec!(10000));
        // Past the floor: L8 goes NEGATIVE (−20,000) and line 9's `larger of` holds it at 10,000.
        assert_eq!(w.line_5e(dec!(60000), Some(dec!(700000)), s), dec!(10000));
    }

    /// ★ Line 1's short-circuit is a **proved no-op** at TY2025's constants — and this is what makes
    /// that a checked fact rather than an assumption.
    ///
    /// Removing the branch survived mutation testing, which is the correct outcome: line 9 floors at
    /// $10,000, identically line 1's trigger, and line 10 takes `min(line 9, 5d)`. So for any
    /// `5d ≤ trigger`, `line9 ≥ floor = trigger ≥ 5d` and the min already returns 5d. MFS halves both
    /// sides consistently ($10,000/2 = $5,000 = the MFS trigger).
    ///
    /// **It stays in the code because it is on the form**, and this sweep guards the proof: if a
    /// future year's floor drops below its trigger — or the MFS halving stops applying to both — the
    /// branch becomes load-bearing and this reds instead of silently mattering.
    #[test]
    fn ty2025_salt_line_1_short_circuit_is_a_proved_no_op() {
        let w = salt_2025();
        let SaltLimitation::Worksheet2025 {
            line9_floor,
            line1_trigger,
            line1_trigger_mfs,
            ..
        } = &w
        else {
            panic!("salt_2025() must be the worksheet variant")
        };
        assert!(
            line9_floor >= line1_trigger && *line9_floor / Decimal::from(2) >= *line1_trigger_mfs,
            "the no-op proof needs floor ≥ trigger on both sides: floor {line9_floor}, trigger \
             {line1_trigger}, MFS trigger {line1_trigger_mfs}"
        );
        let mut checked = 0u32;
        for status in [
            FilingStatus::Single,
            FilingStatus::Mfj,
            FilingStatus::Mfs,
            FilingStatus::HoH,
        ] {
            let trigger = if status == FilingStatus::Mfs {
                *line1_trigger_mfs
            } else {
                *line1_trigger
            };
            let mut d = 0i64;
            while Decimal::from(d) <= trigger {
                let l5d = Decimal::from(d);
                let mut m = 0i64;
                while m < 1_400_000 {
                    // Below the trigger the answer must be 5d itself, at every MAGI.
                    assert_eq!(
                        w.line_5e(l5d, Some(Decimal::from(m)), status),
                        l5d,
                        "{status:?}: 5d {l5d} at MAGI {m} must be unlimited"
                    );
                    checked += 1;
                    m += 6_997;
                }
                d += 331;
            }
        }
        assert!(checked > 5_000, "the sweep must cover ground: {checked}");
    }

    /// ★ An UNASKED modified AGI fails closed — to the floor, never to the cap.
    ///
    /// `assemble_absolute` is infallible by design, so the worksheet cannot refuse mid-computation;
    /// `screen_inputs` refuses first because `QuestionId::HasIncomeExclusion` is live from TY2025
    /// (§G-15 year-scoped it; TY2024's `FlatCap` never reads `magi`). This is the belt to that brace, and its DIRECTION is the whole content: an absent MAGI is treated as
    /// fully phased down, so the filer gets the SMALLEST deduction the worksheet can produce. That
    /// can only overstate tax — the §63(f) rule that an unknown must fail in the direction which
    /// cannot understate. The opposite default (assume MAGI is under the threshold, keep the $40,000
    /// cap) would hand an unasked filer the LARGEST deduction, understating tax on a signed return.
    #[test]
    fn an_unasked_modified_agi_fails_closed_to_the_floor() {
        let w = salt_2025();
        for status in [FilingStatus::Single, FilingStatus::Mfj, FilingStatus::HoH] {
            // 5d well past the cap, so line 1's short-circuit cannot mask the choice.
            let unknown = w.line_5e(dec!(60000), None, status);
            assert_eq!(
                unknown,
                dec!(10000),
                "{status:?}: absent MAGI must give line 9's floor"
            );
            // Bracketed by the two extremes: never more than the best case, never less than the worst.
            let best = w.line_5e(dec!(60000), Some(dec!(0)), status);
            let worst = w.line_5e(dec!(60000), Some(dec!(9000000)), status);
            assert_eq!(
                best,
                dec!(40000),
                "{status:?}: MAGI under the threshold gives the cap"
            );
            assert_eq!(
                unknown, worst,
                "{status:?}: absent must equal the WORST case, not the best"
            );
            assert!(unknown < best, "{status:?}: absent must not be generous");
        }
        // MFS halves the floor at line 10, so its fail-closed value is 5,000.
        assert_eq!(
            w.line_5e(dec!(60000), None, FilingStatus::Mfs),
            dec!(5000),
            "MFS: the floor is halved at line 10 like every other line-9 value"
        );
        // TY2024 ignores MAGI entirely, so `None` changes nothing there.
        let flat = SaltLimitation::FlatCap {
            cap: dec!(10000),
            cap_mfs: dec!(5000),
        };
        assert_eq!(
            flat.line_5e(dec!(30000), None, FilingStatus::Single),
            flat.line_5e(dec!(30000), Some(dec!(600000)), FilingStatus::Single),
            "FlatCap never reads MAGI, so TY2024 is unaffected by an unasked gate"
        );
    }

    /// TY2024 is a different instrument: one question, no worksheet, and the halving is on the cap.
    #[test]
    fn ty2024_salt_is_a_flat_cap_and_ignores_magi() {
        let flat = SaltLimitation::FlatCap {
            cap: dec!(10000),
            cap_mfs: dec!(5000),
        };
        for magi in [dec!(50000), dec!(600000), dec!(5000000)] {
            assert_eq!(
                flat.line_5e(dec!(30000), Some(magi), FilingStatus::Single),
                dec!(10000)
            );
            assert_eq!(
                flat.line_5e(dec!(30000), Some(magi), FilingStatus::Mfs),
                dec!(5000)
            );
            assert_eq!(
                flat.line_5e(dec!(3000), Some(magi), FilingStatus::Single),
                dec!(3000)
            );
        }
    }

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
        // §1401(b)(2) Additional-Medicare threshold — QSS is $200,000 (NOT a joint return), the deliberate
        // asymmetry with §1411's $250,000 QSS above (Fable IMPL-P4 r1 C1).
        assert_eq!(se_addl_medicare_threshold(FilingStatus::Mfj), dec!(250000));
        assert_eq!(se_addl_medicare_threshold(FilingStatus::Qss), dec!(200000)); // ≠ niit_threshold(Qss)
        assert_eq!(
            se_addl_medicare_threshold(FilingStatus::Single),
            dec!(200000)
        );
        assert_eq!(se_addl_medicare_threshold(FilingStatus::HoH), dec!(200000));
        assert_eq!(se_addl_medicare_threshold(FilingStatus::Mfs), dec!(125000));
        // §170(f)(11)(C) statutory threshold — Task 1 KAT.
        assert_eq!(QUALIFIED_APPRAISAL_THRESHOLD, dec!(5000));
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

// ── Schedule 1-A (TY2025-2028) — the four OBBBA additional deductions ─────────────────────────

/// Which way a Schedule 1-A phase-out rounds its **step count** — an **explicit parameter, never
/// baked into a shared helper**, because this one form rounds *both* ways and the two are one word
/// apart on the page.
///
/// | part | line | printed instruction | direction |
/// |---|---|---|---|
/// | II tips | 11 | "decrease the result to the next **lower** whole number" | [`Self::Floor`] |
/// | III overtime | 19 | "decrease the result to the next **lower** whole number" | [`Self::Floor`] |
/// | IV car loan | 28 | "increase the result to the next **higher** whole number" | [`Self::Ceil`] |
///
/// ★ Part IV ceils because §163(h)(4)(B)(iii) says "for each $1,000 **or portion thereof**", where
/// §224(b)(2) and §225(b)(2) say only "for each $1,000". So this is **statutory**, not an IRS
/// formatting quirk, and a `phase_out()` helper with one baked-in direction is silently wrong on one
/// side — by exactly $100 for Parts II/III and $200 for Part IV. (Schedule 8812 line 10 ceils for the
/// same "or fraction thereof" reason.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepRounding {
    /// "decrease the result to the next lower whole number. (For example, decrease 1.5 to 1, and
    /// decrease 0.05 to 0.)"
    Floor,
    /// "increase the result to the next higher whole number. (For example, increase 1.5 to 2, and
    /// increase 0.05 to 1.)"
    Ceil,
}

/// One Schedule 1-A stair-step phase-out: the three lines that read *divide, round, multiply*
/// (10-12, 18-20, 27-29). The **smooth** Part V reduction is deliberately NOT modelled here — it has
/// no step at all (§151(d)(5)(C) is a flat 6%), and giving it a fake step is how a smooth phase-out
/// acquires a stair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StairStepPhaseOut {
    /// Lines 9 / 17 / 26 — the threshold for every status **except** MFJ.
    pub threshold: Usd,
    /// Lines 9 / 17 / 26 — the MFJ figure the line prints in parentheses. A separate field rather
    /// than `threshold * 2` because the form prints both and a future year could break the relation.
    pub threshold_mfj: Usd,
    /// Lines 11 / 19 / 28 — the divisor. $1,000 on every part.
    pub step: Usd,
    /// Lines 12 / 20 / 29 — the multiplier. $100 (Parts II/III) or $200 (Part IV).
    pub per_step: Usd,
    /// Lines 11 / 19 / 28 — see [`StepRounding`]. The field that must never be shared.
    pub rounding: StepRounding,
}

impl StairStepPhaseOut {
    /// Lines 9 / 17 / 26. MFJ takes the parenthesised figure; **every other status takes the base**,
    /// which the Part IV instructions state in exactly those words: "Married filing jointly—$200,000.
    /// All other filing statuses—$100,000."
    pub fn threshold_for(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj => self.threshold_mfj,
            FilingStatus::Single | FilingStatus::HoH | FilingStatus::Mfs | FilingStatus::Qss => {
                self.threshold
            }
        }
    }

    /// Lines 11-12 / 19-20 / 28-29 as one quantity: the amount subtracted from the capped deduction.
    /// `excess` is line 10 / 18 / 27 (already known positive — a zero-or-less line 10 **skips** the
    /// phase-out entirely rather than passing zero here, which is the form's own routing).
    pub fn reduction(&self, excess: Usd) -> Usd {
        let steps = excess / self.step;
        let steps = match self.rounding {
            StepRounding::Floor => steps.floor(),
            StepRounding::Ceil => steps.ceil(),
        };
        steps * self.per_step
    }

    /// The excess at which this phase-out first reaches **zero deduction**, given `cap`.
    ///
    /// ★★ **This is per-direction, and that is the whole point of the function.** A flooring part
    /// exhausts exactly at `(cap / per_step) × step`; a **ceiling** part exhausts one dollar PAST the
    /// last full step, because any portion of a step counts as a whole one. For Part IV with the full
    /// $10,000 cap that is **$49,001**, not $50,000: at an excess of $49,000 line 28 is 49, line 29 is
    /// $9,800 and line 30 still stands at **$200**.
    ///
    /// Used only by the tests, as the closed form that ties threshold, rate and cap together instead
    /// of pinning three independent literals.
    pub fn exhaustion_excess(&self, cap: Usd) -> Usd {
        let full_steps = (cap / self.per_step).floor();
        match self.rounding {
            StepRounding::Floor => full_steps * self.step,
            StepRounding::Ceil => (full_steps - Decimal::ONE) * self.step + Decimal::ONE,
        }
    }
}

/// **Schedule 1-A (Form 1040) — the four OBBBA "additional deductions", TY2025-2028.**
///
/// ★ **Nothing here is indexed and everything here EXPIRES.** All four provisions (§§224, 225,
/// 163(h)(4), 151(d)(5)) sunset after TY2028 and the statute fixes the caps and thresholds as plain
/// dollar amounts — so a "next year's Rev. Proc." lookup is not merely unnecessary, it is **wrong**,
/// and **TY2029+ must fail closed** exactly as TY2026 does today. That is why this is a
/// `statutory`-section type keyed by year rather than a `TaxTable` field.
///
/// ★ Three distinct threshold pairs and three distinct caps live on one form; none is shared. See
/// [`StairStepPhaseOut::rounding`] for the fourth thing that must not be shared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule1aParams {
    pub year: i32,
    /// Line 7 — "Enter the smaller of the amount on line 6 or $25,000." §224(b)(1).
    ///
    /// ★ **Per RETURN, not per spouse, and it does not vary by status**: the line prints no MFJ
    /// figure, and the instructions say so twice — "You can't deduct more than $25,000 of qualified
    /// tips, **regardless of your filing status**" and "the $25,000 maximum amount of deduction limit
    /// applies to your **combined** qualified tip income. **It is not a per spouse limit.**"
    pub tips_cap: Usd,
    /// Lines 8-12 — the §224(b)(2) phase-out. **Floors.**
    pub tips_phase_out: StairStepPhaseOut,
    /// Line 15 — "Enter the smaller of the amount on line 14c or $12,500 ($25,000 if married filing
    /// jointly)." §225(b)(1). Unlike the tips cap, this one DOES double for MFJ.
    pub overtime_cap: Usd,
    /// Line 15's parenthesised MFJ figure.
    pub overtime_cap_mfj: Usd,
    /// Lines 16-20 — the §225(b)(2) phase-out. **Floors.**
    pub overtime_phase_out: StairStepPhaseOut,
    /// Line 24 — "Enter the smaller of the amount on line 23 or $10,000." §163(h)(4)(B)(ii).
    pub qpvli_cap: Usd,
    /// Lines 25-29 — the §163(h)(4)(B)(iii) phase-out. **CEILS** — the one ceiling on the form.
    pub qpvli_phase_out: StairStepPhaseOut,
    /// Line 33's constant and line 35's minuend — "$6,000". §151(d)(5)(A). **Per qualifying
    /// individual**: line 35 is computed once and lines 36a/36b each enter it, so an MFJ couple with
    /// two seniors loses **12¢ per $1** of MAGI in the band while each of them loses 6¢.
    pub senior_per_person: Usd,
    /// Line 32 — "Enter $75,000 ($150,000 if married filing jointly)."
    pub senior_threshold: Usd,
    /// Line 32's parenthesised MFJ figure.
    pub senior_threshold_mfj: Usd,
    /// Line 34 — "Multiply line 33 by 6% (0.06)." §151(d)(5)(C). **Smooth, no stair-step at all.**
    ///
    /// ★ Line 34 is its own printed dollar line with no rounding instruction of its own, so the
    /// general IRS whole-dollar convention governs it and **line 35 subtracts the PRINTED line 34**.
    /// `6,000 − round(0.06 × L33)` and `round(6,000 − 0.06 × L33)` differ by $1 whenever `0.06 × L33`
    /// lands on a half-dollar (excess ≡ 25 mod 50), doubled on a two-senior return — and the
    /// round-the-difference form is the one that understates tax.
    pub senior_rate: Decimal,
}

impl Schedule1aParams {
    /// Line 32 / line 9 / line 17 style: MFJ takes the parenthesised figure, every other status the base.
    pub fn senior_threshold_for(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj => self.senior_threshold_mfj,
            FilingStatus::Single | FilingStatus::HoH | FilingStatus::Mfs | FilingStatus::Qss => {
                self.senior_threshold
            }
        }
    }

    /// Line 15: "$12,500 ($25,000 if married filing jointly)".
    pub fn overtime_cap_for(&self, status: FilingStatus) -> Usd {
        match status {
            FilingStatus::Mfj => self.overtime_cap_mfj,
            FilingStatus::Single | FilingStatus::HoH | FilingStatus::Mfs | FilingStatus::Qss => {
                self.overtime_cap
            }
        }
    }
}

/// Schedule 1-A parameters for `year`, or `None` where the form does not exist.
///
/// ★★ **`None` for TY2029+ is the load-bearing behaviour, not an omission.** The four provisions
/// expire after TY2028 (§224(f), §225(f), §163(h)(4)(F), §151(d)(5)(D)); a table that quietly extends
/// them files a deduction that does not exist. `None` for 2024-and-earlier is likewise correct — the
/// form was created by Pub. L. 119-21 and there is no 2024 Schedule 1-A.
///
/// Values are identical across 2025-2028 because **nothing here is indexed**; they are written once
/// and returned for each year in range rather than duplicated per year, so no year can drift.
pub fn schedule_1a_params(year: i32) -> Option<Schedule1aParams> {
    if !(2025..=2028).contains(&year) {
        return None;
    }
    Some(Schedule1aParams {
        year,
        tips_cap: dec!(25000),
        tips_phase_out: StairStepPhaseOut {
            threshold: dec!(150000),
            threshold_mfj: dec!(300000),
            step: dec!(1000),
            per_step: dec!(100),
            rounding: StepRounding::Floor,
        },
        overtime_cap: dec!(12500),
        overtime_cap_mfj: dec!(25000),
        overtime_phase_out: StairStepPhaseOut {
            threshold: dec!(150000),
            threshold_mfj: dec!(300000),
            step: dec!(1000),
            per_step: dec!(100),
            rounding: StepRounding::Floor,
        },
        qpvli_cap: dec!(10000),
        qpvli_phase_out: StairStepPhaseOut {
            threshold: dec!(100000),
            threshold_mfj: dec!(200000),
            step: dec!(1000),
            per_step: dec!(200),
            rounding: StepRounding::Ceil,
        },
        senior_per_person: dec!(6000),
        senior_threshold: dec!(75000),
        senior_threshold_mfj: dec!(150000),
        senior_rate: dec!(0.06),
    })
}

#[cfg(test)]
mod schedule_1a_tests {
    use super::*;
    use crate::conventions::round_dollar;

    fn p() -> Schedule1aParams {
        schedule_1a_params(2025).expect("TY2025 has a Schedule 1-A")
    }

    /// ★★ **TY2029+ MUST FAIL CLOSED, and TY2024 has no Schedule 1-A at all.** The four provisions
    /// expire after TY2028, so a table that quietly extends them files a deduction that does not
    /// exist — the same class of defect as `ty2026_full_return_must_stay_fail_closed`.
    #[test]
    fn schedule_1a_exists_only_for_2025_through_2028() {
        for year in [2017, 2023, 2024, 2029, 2030, 2035] {
            assert!(
                schedule_1a_params(year).is_none(),
                "TY{year} must have NO Schedule 1-A — the form was created by Pub. L. 119-21 and \
                 §§224(f)/225(f)/163(h)(4)(F)/151(d)(5)(D) expire it after 2028"
            );
        }
        for year in 2025..=2028 {
            let got = schedule_1a_params(year).unwrap_or_else(|| panic!("TY{year} needs params"));
            assert_eq!(got.year, year);
            // Nothing here is indexed, so every in-range year is the SAME instrument.
            assert_eq!(
                Schedule1aParams {
                    year: 2025,
                    ..got.clone()
                },
                p(),
                "TY{year} drifted from TY2025 — none of these amounts is indexed"
            );
        }
    }

    /// Every constant against the printed line it comes from.
    #[test]
    fn every_constant_matches_its_printed_line() {
        let p = p();
        // L7 "Enter the smaller of the amount on line 6 or $25,000" — no MFJ figure printed.
        assert_eq!(p.tips_cap, dec!(25000));
        // L9 / L17 "Enter $150,000 ($300,000 if married filing jointly)".
        for po in [&p.tips_phase_out, &p.overtime_phase_out] {
            assert_eq!(po.threshold, dec!(150000));
            assert_eq!(po.threshold_mfj, dec!(300000));
            assert_eq!(po.per_step, dec!(100)); // L12 / L20 "Multiply line 11 by $100"
            assert_eq!(po.step, dec!(1000));
        }
        // L15 "$12,500 ($25,000 if married filing jointly)" — this cap DOES double.
        assert_eq!(p.overtime_cap, dec!(12500));
        assert_eq!(p.overtime_cap_mfj, dec!(25000));
        // L24 "$10,000"; L26 "$100,000 ($200,000 if married filing jointly)"; L29 "by $200".
        assert_eq!(p.qpvli_cap, dec!(10000));
        assert_eq!(p.qpvli_phase_out.threshold, dec!(100000));
        assert_eq!(p.qpvli_phase_out.threshold_mfj, dec!(200000));
        assert_eq!(p.qpvli_phase_out.per_step, dec!(200));
        // L32 "$75,000 ($150,000 if married filing jointly)"; L34 "6% (0.06)"; L35 "from $6,000".
        assert_eq!(p.senior_threshold, dec!(75000));
        assert_eq!(p.senior_threshold_mfj, dec!(150000));
        assert_eq!(p.senior_rate, dec!(0.06));
        assert_eq!(p.senior_per_person, dec!(6000));
    }

    /// ★★ **THE ROUNDING DIRECTION IS PER PART.** Parts II/III floor (lines 11, 19); Part IV ceils
    /// (line 28). This asserts the direction where they DIFFER — at a fractional step — because that
    /// is the only place the two are distinguishable. The form's own examples are the fixtures:
    /// "decrease 1.5 to 1, and decrease 0.05 to 0" vs "increase 1.5 to 2, and increase 0.05 to 1".
    #[test]
    fn parts_two_and_three_floor_the_step_count_while_part_four_ceils() {
        let p = p();
        // 1.5 steps.
        assert_eq!(p.tips_phase_out.reduction(dec!(1500)), dec!(100)); // floor(1.5)=1 × $100
        assert_eq!(p.overtime_phase_out.reduction(dec!(1500)), dec!(100));
        assert_eq!(p.qpvli_phase_out.reduction(dec!(1500)), dec!(400)); // ceil(1.5)=2 × $200
                                                                        // 0.05 steps — the form's own second example, and the case a truncating `as i64` gets right
                                                                        // by accident on the floor side and wrong on the ceil side.
        assert_eq!(p.tips_phase_out.reduction(dec!(50)), Usd::ZERO); // floor(0.05)=0
        assert_eq!(p.qpvli_phase_out.reduction(dec!(50)), dec!(200)); // ceil(0.05)=1 × $200
                                                                      // A WHOLE number of steps is where the two agree — so a test using only $1,000 multiples
                                                                      // would pass under either direction and prove nothing. Recorded, not relied on.
        assert_eq!(p.tips_phase_out.reduction(dec!(3000)), dec!(300));
        assert_eq!(p.qpvli_phase_out.reduction(dec!(3000)), dec!(600));
    }

    /// ★★ **THE PAIRED KNEE ASSERTION** — "at the knee, $0" AND "one step below, still > $0". A
    /// knee-only test passes under both rounding directions, which is exactly the S-1 failure mode.
    ///
    /// ★ And the knee is **per direction**: a flooring part exhausts at `(cap/per_step) × step`, but a
    /// **ceiling** part exhausts one dollar PAST the last full step. Part IV with the full $10,000 cap
    /// exhausts at an excess of **$49,001**, not $50,000 — at $49,000 line 28 is 49, line 29 is $9,800
    /// and line 30 still stands at $200. Asserting `+$50,000` there would force Part IV to floor.
    #[test]
    fn each_phase_out_exhausts_at_its_own_knee_and_not_one_step_earlier() {
        let p = p();
        let cases: [(&str, &StairStepPhaseOut, Usd, Usd); 3] = [
            ("II tips", &p.tips_phase_out, p.tips_cap, dec!(250000)),
            (
                "III overtime",
                &p.overtime_phase_out,
                p.overtime_cap,
                dec!(125000),
            ),
            ("IV car loan", &p.qpvli_phase_out, p.qpvli_cap, dec!(49001)),
        ];
        for (name, po, cap, expected_knee) in cases {
            let knee = po.exhaustion_excess(cap);
            assert_eq!(knee, expected_knee, "{name}: wrong exhaustion excess");
            // AT the knee: the reduction has consumed the whole cap.
            assert!(
                po.reduction(knee) >= cap,
                "{name}: at the knee ({knee}) the reduction must reach the cap {cap}"
            );
            // ONE STEP BELOW the knee: a deduction must still stand. This is the half that dies if
            // the direction is flipped.
            let below = knee - po.step;
            assert!(
                po.reduction(below) < cap,
                "{name}: one step below the knee ({below}) a deduction must still stand"
            );
        }
        // ★ The $200 that proves Part IV ceils rather than floors, spelled out.
        assert_eq!(p.qpvli_phase_out.reduction(dec!(49000)), dec!(9800));
        assert_eq!(
            p.qpvli_cap - p.qpvli_phase_out.reduction(dec!(49000)),
            dec!(200)
        );
        // A floor would give ceil→floor(49.5)=49 here and leave $200 standing where the form gives $0.
        assert!(p.qpvli_phase_out.reduction(dec!(49500)) >= p.qpvli_cap);
    }

    /// Part V is **smooth** — 6% of the excess, no step at all — and everyone reaches $0 at
    /// `threshold + $100,000`. This is the closed form tying threshold, rate and cap together
    /// (`per_person / rate = 6,000 / 0.06 = 100,000`) rather than pinning a fourth literal.
    #[test]
    fn part_five_is_smooth_and_exhausts_one_hundred_thousand_above_its_threshold() {
        let p = p();
        assert_eq!(p.senior_per_person / p.senior_rate, dec!(100000));
        // 6¢ per $1 per person; an MFJ couple with two seniors loses 12¢ per $1 (S-4).
        let l34 = |excess: Usd| round_dollar(excess * p.senior_rate);
        assert_eq!(l34(dec!(1000)), dec!(60));
        assert_eq!(p.senior_per_person - l34(dec!(100000)), Usd::ZERO);
        assert!(p.senior_per_person - l34(dec!(99000)) > Usd::ZERO);
    }

    /// ★ **Line 34 is rounded as its own PRINTED line, then line 35 subtracts it** — the project's
    /// standing rule that every line takes that schedule's printed figure. The two orders differ by
    /// $1 exactly when `0.06 × L33` lands on a half-dollar (excess ≡ 25 mod 50), and the
    /// round-the-difference order gives the LARGER deduction, i.e. understates tax.
    #[test]
    fn line_34_rounds_before_line_35_subtracts() {
        let p = p();
        let excess = dec!(50025); // 0.06 × 50,025 = 3,001.50 — exactly on the half-dollar
        let raw = excess * p.senior_rate;
        assert_eq!(
            raw,
            dec!(3001.50),
            "the fixture must actually straddle a half-dollar"
        );
        let round_the_line = p.senior_per_person - round_dollar(raw); // 6,000 − 3,002 = 2,998
        let round_the_difference = round_dollar(p.senior_per_person - raw); // round(2,998.50) = 2,999
        assert_eq!(round_the_line, dec!(2998));
        assert_eq!(round_the_difference, dec!(2999));
        assert!(
            round_the_difference > round_the_line,
            "the wrong order is the one that inflates the deduction — so it must not be chosen \
             by accident"
        );
    }

    /// Thresholds: MFJ takes the parenthesised figure, **every other status the base** — which the
    /// Part IV instructions state in those words: "Married filing jointly—$200,000. All other filing
    /// statuses—$100,000." ★ Notably MFS takes the BASE, not half of it: unlike §164(b)'s SALT
    /// worksheet, no Schedule 1-A line halves anything for MFS.
    #[test]
    fn only_mfj_takes_the_parenthesised_threshold_and_mfs_takes_the_base() {
        let p = p();
        for status in [
            FilingStatus::Single,
            FilingStatus::HoH,
            FilingStatus::Mfs,
            FilingStatus::Qss,
        ] {
            assert_eq!(p.qpvli_phase_out.threshold_for(status), dec!(100000));
            assert_eq!(p.tips_phase_out.threshold_for(status), dec!(150000));
            assert_eq!(p.senior_threshold_for(status), dec!(75000));
            assert_eq!(p.overtime_cap_for(status), dec!(12500));
        }
        assert_eq!(
            p.qpvli_phase_out.threshold_for(FilingStatus::Mfj),
            dec!(200000)
        );
        assert_eq!(
            p.tips_phase_out.threshold_for(FilingStatus::Mfj),
            dec!(300000)
        );
        assert_eq!(p.senior_threshold_for(FilingStatus::Mfj), dec!(150000));
        assert_eq!(p.overtime_cap_for(FilingStatus::Mfj), dec!(25000));
        // ★ The tips cap is the ONE that does not vary at all — "regardless of your filing status",
        // "It is not a per spouse limit". There is deliberately no `tips_cap_for(status)`.
        assert_eq!(p.tips_cap, dec!(25000));
    }

    /// ★ The recon's worked examples (b) and (c) — the two figures that DIFFER under the wrong
    /// rounding, which is why the spec names them as the minimum bar (§5.2).
    #[test]
    fn the_two_worked_examples_that_distinguish_floor_from_ceil() {
        let p = p();
        // (b) Single, MAGI $157,350, tips $3,000 ⇒ $2,300. A ceil would give $2,200.
        let excess_b = dec!(157350) - p.tips_phase_out.threshold_for(FilingStatus::Single);
        assert_eq!(excess_b, dec!(7350));
        let cap_b = dec!(3000).min(p.tips_cap);
        assert_eq!(cap_b - p.tips_phase_out.reduction(excess_b), dec!(2300));
        // (c) Single, MAGI $104,050, QPVLI $6,000 ⇒ $5,000. A floor would give $5,200.
        let excess_c = dec!(104050) - p.qpvli_phase_out.threshold_for(FilingStatus::Single);
        assert_eq!(excess_c, dec!(4050));
        let cap_c = dec!(6000).min(p.qpvli_cap);
        assert_eq!(cap_c - p.qpvli_phase_out.reduction(excess_c), dec!(5000));
    }
}

/// The Schedule 1-A form, as `pdftotext -layout` extracts it. **IN-CRATE deliberately** — an
/// `include_str!` reaching outside the crate ships a broken tarball with exit 0 (see the crate-publishing
/// note on `GOLDEN_RETURNS_JSON`). Regenerate with `cargo run -p xtask -- extract-schedule-1a`.
#[cfg(test)]
const SCHEDULE_1A_FORM_TEXT: &str = include_str!("fixtures/schedule_1a_2025_form.txt");

#[cfg(test)]
mod schedule_1a_conformance {
    use super::*;

    /// The printed text of numbered line `label`: the line that begins with it, plus every continuation
    /// line up to the next numbered label. Returns `None` if the label is absent — which is itself a
    /// finding, so callers `expect` it rather than defaulting.
    fn printed_line(label: &str) -> Option<String> {
        let mut lines = SCHEDULE_1A_FORM_TEXT.lines();
        let first_word = |l: &str| l.split_whitespace().next().map(str::to_string);
        let starts_a_label = |l: &str| {
            first_word(l).is_some_and(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()))
        };
        let first = lines
            .by_ref()
            .find(|l| first_word(l).is_some_and(|w| w == label))?;
        let mut out = first.trim().to_string();
        for l in lines {
            if starts_a_label(l) || l.trim().is_empty() {
                break;
            }
            out.push(' ');
            out.push_str(l.trim());
        }
        Some(out)
    }

    /// ★★ **THE ROUNDING DIRECTION IS READ OFF THE FORM, NOT HAND-ASSIGNED.**
    ///
    /// This closes the gap `FOLLOWUPS.md` §G-10 records against `xtask cite-check`: that tool proves a
    /// quotation is the *form's* words but not that they are *that line's* words, so moving line 28's
    /// "increase … to the next higher" onto line 11 survives it — and that swap inverts the rounding for
    /// Parts II/III, the most dangerous single fact in this form.
    ///
    /// The fix is not more citation checking. Floor-vs-ceiling is a **decision the code makes**, so it
    /// can be *derived* from the printed instruction and compared to what `schedule_1a_params` assigned.
    /// Neither side can then drift alone: editing the params reds this, and so does editing the extract.
    #[test]
    fn each_phase_out_rounds_the_way_its_own_printed_line_says_to() {
        /// The direction the printed text states, refusing to guess.
        fn direction_from(text: &str) -> StepRounding {
            let lower = text.to_lowercase();
            let says_down = lower.contains("decrease the result to the next")
                && lower.contains("lower whole number");
            let says_up = lower.contains("increase the result to the next")
                && lower.contains("higher whole number");
            match (says_down, says_up) {
                (true, false) => StepRounding::Floor,
                (false, true) => StepRounding::Ceil,
                // Both or neither: the extract changed shape. Fail loudly — a default here would make
                // this whole test pass by assuming the answer it exists to read.
                _ => panic!(
                    "cannot read a rounding direction from the printed line: {text:?}\n\
                     (down={says_down}, up={says_up} — regenerate the extract with \
                     `cargo run -p xtask -- extract-schedule-1a`)"
                ),
            }
        }
        let p = schedule_1a_params(2025).expect("TY2025 has a Schedule 1-A");
        // (printed line, the excess line it divides, the params field it governs)
        let cases: [(&str, &str, &StairStepPhaseOut); 3] = [
            ("11", "10", &p.tips_phase_out),
            ("19", "18", &p.overtime_phase_out),
            ("28", "27", &p.qpvli_phase_out),
        ];
        for (label, divides, po) in cases {
            let text = printed_line(label)
                .unwrap_or_else(|| panic!("line {label} is not in the form extract"));
            assert_eq!(
                direction_from(&text),
                po.rounding,
                "line {label} of the form says one thing and `schedule_1a_params` says another"
            );
            // ★ And the CROSS-REFERENCE, read off the same line. This is the Form 6251 line-33 defect
            // class — "Subtract line 32 from line 12" where the form said line 22 — caught mechanically
            // instead of by a reviewer noticing two adjacent digits.
            assert!(
                text.contains(&format!("Divide line {divides} by $1,000")),
                "line {label} must divide line {divides}; printed text is {text:?}"
            );
            // The divisor and multiplier are on the same page as the direction, so check them here too.
            assert_eq!(po.step, dec!(1000));
        }
    }

    /// The same treatment for the two lines that state a **dollar amount** the code carries: line 7's
    /// $25,000 tips cap (which prints NO filing-status variant — the S-3 per-return reading) and line 24's
    /// $10,000 QPVLI cap.
    #[test]
    fn the_caps_that_do_not_vary_by_status_print_no_variant() {
        let p = schedule_1a_params(2025).expect("TY2025 has a Schedule 1-A");
        let l7 = printed_line("7").expect("line 7");
        assert!(
            l7.contains("Enter the smaller of the amount on line 6 or $25,000"),
            "line 7 text drifted: {l7:?}"
        );
        assert!(
            !l7.contains("married filing jointly"),
            "★ line 7 prints NO MFJ figure — that absence IS the evidence for S-3's per-return cap, so \
             if the form ever adds one this reading must be revisited: {l7:?}"
        );
        assert_eq!(p.tips_cap, dec!(25000));

        // By contrast line 15 DOES print a variant, which is why `overtime_cap_for` exists and
        // `tips_cap_for` deliberately does not.
        let l15 = printed_line("15").expect("line 15");
        assert!(
            l15.contains("$12,500 ($25,000 if married filing jointly)"),
            "line 15 text drifted: {l15:?}"
        );
        let l24 = printed_line("24").expect("line 24");
        assert!(
            l24.contains("Enter the smaller of the amount on line 23 or $10,000"),
            "line 24 text drifted: {l24:?}"
        );
        assert_eq!(p.qpvli_cap, dec!(10000));
    }

    /// ★ Guard the reader itself. `printed_line` returning an empty or truncated string would make every
    /// assertion above pass vacuously, so pin its behaviour on a line whose shape is known.
    #[test]
    fn the_printed_line_reader_captures_continuations_and_stops_at_the_next_label() {
        let l28 = printed_line("28").expect("line 28");
        // Continuation captured: the direction word is on the FIRST line, the example on the SECOND.
        assert!(l28.contains("increase the result to the next"));
        assert!(l28.contains("increase 1.5 to 2, and increase 0.05 to 1"));
        // Stopped at the next label: line 29's text must NOT have been swallowed.
        assert!(
            !l28.contains("Multiply line 28 by $200"),
            "the reader ran past line 28 into line 29: {l28:?}"
        );
        assert!(
            printed_line("999").is_none(),
            "a missing label must be None, not a default"
        );
    }
}
