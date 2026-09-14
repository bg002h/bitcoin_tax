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

/// §170(f)(8)(A): the contemporaneous-written-acknowledgment threshold — *"any contribution of $250
/// or more"*. **STATUTORY** — 26 U.S.C. §170(f)(8)(A); fixed in the Code, NOT inflation-indexed.
///
/// ★ Tested with `>=`, not `>`: the statute says *"$250 **or more**"*, unlike §170(f)(11)(C)'s
/// *"more than $5,000"* directly above — the two thresholds have opposite boundary conventions and
/// sit five lines apart. Must never be placed in a `TaxTable`.
pub const CWA_SUBSTANTIATION_THRESHOLD: Usd = dec!(250);

/// §170(f)(11)(D): the threshold above which a **qualified appraisal must be ATTACHED to the return** —
/// *"In the case of contributions of property for which a deduction of more than $500,000 is claimed,
/// the requirements … are met … if [the taxpayer] attaches to the return for the taxable year a
/// qualified appraisal."* **STATUTORY** — 26 U.S.C. §170(f)(11)(D); fixed in the Code, NOT indexed.
///
/// ★★ **The operand is the pre-ceiling amount CLAIMED for the property** — post-§170(e) reduction,
/// aggregated across all similar items given in the year (§170(f)(11)(F); Reg §1.170A-16(f)(5)(ii)),
/// determined WITHOUT regard to the §170(b) AGI ceiling / §170(d) carryover split. That is exactly
/// [`crate::forms::year_donation_deduction`], and it is emphatically NOT Schedule A line 12: keying
/// the gate to the post-ceiling line would make a statutory attachment depend on AGI (the same $700k
/// gift would require an appraisal for a $3M-AGI filer and not for a $1M-AGI one), which
/// Reg §1.170A-16(f)(3) contradicts by extending the duty to *"the return for any carryover year"*.
///
/// ★ Strict `>`: the statute says *"more than $500,000"*, like §170(f)(11)(C) and unlike
/// [`CWA_SUBSTANTIATION_THRESHOLD`]. Must never be placed in a `TaxTable`.
pub const APPRAISAL_ATTACHMENT_THRESHOLD: Usd = dec!(500000);

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
/// ★★★ **§223(b) — the HSA annual contribution limitation** (T16 / Form 8889 line 3).
///
/// **Where these come from, and why not the usual Rev. Proc.** §223(g) requires the Secretary to
/// publish the HSA amounts **by June 1 of the PRECEDING calendar year**, so they do not appear in
/// the autumn inflation revenue procedure that carries the brackets and the standard deduction —
/// each year gets its own spring Revenue Procedure, a year and a half ahead of the return. TY2024 is
/// Rev. Proc. 2023-23 §2.01(1), TY2025 Rev. Proc. 2024-25 §2.01(1), TY2026 Rev. Proc. 2025-19
/// §2.01(1); all three are archived under `legal/primary-sources/irs-guidance/`.
///
/// ★★ The two limitations ARE indexed (§223(g)(1)); the **$1,000 additional contribution amount is
/// not** — §223(b)(3)(B) states it as a flat statutory figure with no indexing clause, which is why
/// it has read $1,000 unchanged since 2009. A table update that "indexes" it alongside its
/// neighbours is wrong, exactly as for [`FullReturnParams::qbi_phase_in_range_unmarried`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HsaParams {
    /// §223(b)(2)(A) — *"the annual limitation on deductions … for an individual with **self-only**
    /// coverage under a high deductible health plan"*. TY2024 = **$4,150**, which is the figure Form
    /// 8889 (2024) line 3 prints (`design/forms/extract/f8889--2024.txt:22`).
    pub self_only_limit: Usd,
    /// §223(b)(2)(B) — the same for **family** coverage. TY2024 = **$8,300**
    /// (`design/forms/extract/f8889--2024.txt:22`).
    pub family_limit: Usd,
    /// §223(b)(3)(B) — the **additional contribution amount** for an individual who has attained age
    /// 55 before the close of the taxable year. **$1,000, statutory and NOT indexed.** It reaches
    /// line 3 for an unmarried filer (or one married with self-only coverage all year) and line 7 for
    /// a filer married with family coverage — the form's own split.
    pub additional_contribution_55: Usd,
}

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

/// ★★★ **§68 — THE OVERALL LIMITATION ON ITEMIZED DEDUCTIONS, AS SCHEDULE A PRINTS ITS GATE.**
///
/// Pub. L. 119-21 (OBBBA) **§70111(a)** rewrote §68 entirely, and **§70111(c)** applies it *"to
/// taxable years beginning after December 31, 2025"* — no sunset
/// (`legal/text/statute-irc/PLAW-119publ21_OBBBA.txt:5303-5332`). The TY2026 Schedule A carries it as
/// a **gate in front of the itemized total**, which that revision also moves from line 17 to **line
/// 18**:
///
/// > *"Is the amount on Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and 13b of
/// > that form, more than $384,350?"*
/// > *"No. Your deductions are not limited. Add the amounts in far-right column for lines 4 through
/// > 17z. Also enter this amount on Form 1040 or 1040-SR, line 12e."*
/// > *"Yes. Your deductions may be limited. See the Itemized Deductions Worksheet in the
/// > instructions to figure the amount to enter"*
/// > (`design/forms/extract/f1040sa--2026-DRAFT.txt:158-163`.)
///
/// ★★★ **THIS TYPE CARRIES THE SCREEN AND NOTHING ELSE. btctax DOES NOT COMPUTE THE LIMITATION,
/// DELIBERATELY.** The *Yes* branch's answer lives in the **Itemized Deductions Worksheet** in the
/// TY2026 Form 1040 / Schedule A instructions, and **neither `i1040gi--2026` nor `i1040sca--2026` is
/// archived** — the documents do not exist yet. Transcribing a worksheet we cannot read would mean
/// inventing its lines; computing §68(a)'s `2/37` arithmetic from the statute instead would be the
/// closed form `CLAUDE.md` forbids, on the one branch where getting it wrong OVERSTATES the
/// deduction and therefore UNDERSTATES the tax. So the *Yes* branch **refuses**
/// ([`crate::tax::return_refuse::section_68_gate`]). Refusing is how a tool avoids making up values.
///
/// ★★★ **WHY THERE IS NO 1040 LINE NUMBER IN THIS TYPE, AND WHY THE CALLER TESTS A *SEMANTIC*
/// QUANTITY.** The gate's own text cites *"Form 1040 or 1040-SR, line 11b"* minus *"lines 13a and
/// 13b"* — and **the TY2026 Form 1040 is NOT ARCHIVED.** `FOLLOWUPS.md` **FR-214** records that the
/// only evidence in this tree that the TY2026 1040 renumbers its lines at all is Form 6251's own
/// text citing *"line 7a"*: a *referencing* form is the single indirect witness to a change in the
/// *referenced* one. Keying anything on `11b`/`13a`/`13b` would therefore be an assertion about a
/// document nobody here has read. What those lines MEAN is not in doubt — AGI, less the §199A QBI
/// deduction, less the Schedule 1-A additional deductions — and btctax already computes all three
/// (`AbsoluteReturn::agi`, `::qbi_deduction`, `::schedule_1a_additional`). The gate is keyed on the
/// meaning; [`Self::gate_sentence`] carries the form's numbering as the quotation it is.
///
/// ★★★ **$384,350 IS TRANSCRIBED FROM THE FORM AND IS *NOT* THE FILER'S OWN 37% BRACKET START.**
/// §68(a)(2) measures against *"the dollar amount at which the 37 percent rate bracket under section
/// 1 begins with respect to the taxpayer"* — a per-status figure, which for TY2026 is Single
/// **$640,600**, MFJ **$768,700**, HoH **$640,600**, MFS **$384,350**. The form's gate prints **one**
/// number, **$384,350**, **with no filing-status parenthetical** — unlike line 5e directly above it,
/// which does carry one (*"$40,400 ($20,200 if married filing separately)"*). That single number is
/// the **minimum** over the four statuses, so the printed screen is deliberately **over-inclusive**:
/// it screens in every filer who *could* be limited and leaves the worksheet to apply the taxpayer's
/// own bracket start. The form says *"may be limited"*, and this type says exactly that much.
///
/// ★★ **`FOLLOWUPS.md` FR-186 names the trap, so it is spelled out here rather than left to care:**
/// `dec!(384350)` already exists in this workspace as the TY2026 **MFS 37% bracket start**
/// (`btctax-adapters::tax_tables::ty2026`, `ty2026_mfs_37_pct_starts_at_384350`). Reading the
/// threshold off the bracket table "for the filer's status" therefore looks like a tidy-up and is a
/// **defect in the understating direction**: a Single filer would be measured against $640,600 and
/// screened **OUT** of a limitation the form screens them **IN** to. There is no `FilingStatus`
/// parameter anywhere in this type or in [`section_68_status`], which is what makes that edit
/// impossible rather than merely discouraged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section68Gate {
    /// The tax year whose Schedule A this gate was transcribed from.
    pub year: i32,
    /// The threshold the gate PRINTS — `$384,350` on TY2026 — status-independent, see the type doc.
    pub threshold: Usd,
    /// The Schedule A line the itemized total sits on behind this gate (`"18"` on TY2026; it was
    /// line 17 through TY2025). Carried so a refusal can name the line without a second copy.
    pub total_line: &'static str,
    /// The worksheet the *Yes* branch sends the filer to, in the form's own words.
    pub worksheet: &'static str,
    /// The gate's question, **verbatim from the text layer**, with the extract's mid-sentence wrap
    /// closed up. One copy: the refusal quotes this, and
    /// `the_gate_sentence_and_threshold_are_the_forms_own` asserts it against the extract, so the
    /// sentence btctax shows a filer cannot drift from the sentence on the paper.
    pub gate_sentence: &'static str,
}

/// ★★★ **THE FIRST TAX YEAR §68 APPLIES TO — the ONE place that boundary is written.**
///
/// Pub. L. 119-21 (OBBBA) §70111(c): *"The amendments made by this section shall apply to taxable
/// years beginning after December 31, 2025."* Statute, with **no** sunset — unlike
/// [`SCHEDULE_1A_YEARS`], this window has a start and no end, and writing it as a closed range would
/// be inventing an expiry Congress did not enact.
///
/// TCJA §11046 had suspended the old §68 for TY2018-TY2025, which is why TY2024 and TY2025 Schedule
/// A print no such gate — asserted from the extracts by
/// `the_gated_years_are_read_off_the_archived_schedule_a_extracts`, not assumed here.
pub const SECTION_68_FIRST_YEAR: i32 = 2026;

/// What §68 does to `year`'s return. Returned by [`section_68_status`].
///
/// ★★★ **Three arms, and the third is the load-bearing one.** Matched with no `_` at the one call
/// site ([`crate::tax::return_refuse::section_68_gate`]), so a fourth arm is a compile error there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section68Status {
    /// `year < SECTION_68_FIRST_YEAR` — §68 was suspended by TCJA §11046 and this year's Schedule A
    /// prints no gate. The itemized total is unlimited, which is the **correct** return.
    NotApplicable,
    /// §68 applies and the year's Schedule A has been transcribed, so the printed threshold is known.
    Gated(Section68Gate),
    /// ★★★ **§68 applies and NO Schedule A revision for this year has been transcribed, so the
    /// threshold is UNKNOWN — and this refuses unconditionally.**
    ///
    /// This arm is why [`section_68_status`] is not "a list of years that print the gate". The
    /// threshold is the 37% bracket start, an **indexed** figure that moves every year, so a single
    /// transcribed constant can never cover a later year — and a year that fell through to
    /// *"unlimited"* would file a deduction larger than allowed, silently, in the understating
    /// direction. A year this code has no document for therefore fails **closed**: adding TY2027
    /// support is archiving `f1040sa--2027` and transcribing its threshold, and forgetting to is a
    /// refusal rather than a wrong figure (`CLAUDE.md`, *"Derive the list, or make the compiler hold
    /// it"* — here the unknown year is held by the `match`'s own fall-through, not by a list).
    ThresholdNotTranscribed,
}

/// What §68 does to `year`'s return. See [`Section68Status`].
///
/// ★ **What this covers, stated plainly** (`CLAUDE.md` rule (3)): exactly the Schedule A revisions
/// whose text layer is committed under `design/forms/extract/` AND prints the gate — today, **TY2026
/// alone**. Every later year is [`Section68Status::ThresholdNotTranscribed`] and refuses; every
/// earlier year is [`Section68Status::NotApplicable`] and files unchanged. The year set is not
/// asserted here: `the_gated_years_are_read_off_the_archived_schedule_a_extracts` derives it from
/// the extract directory and reds if this function and the archived forms disagree in either
/// direction.
#[must_use]
pub fn section_68_status(year: i32) -> Section68Status {
    if year < SECTION_68_FIRST_YEAR {
        return Section68Status::NotApplicable;
    }
    match year {
        // `f1040sa--2026-DRAFT.txt:158-163`. The threshold is read off the FORM, never off the
        // §1(j)(2) bracket table that carries the same numeral for MFS — see `Section68Gate`.
        2026 => Section68Status::Gated(Section68Gate {
            year: 2026,
            threshold: dec!(384350),
            total_line: "18",
            worksheet: "Itemized Deductions Worksheet",
            gate_sentence: "Is the amount on Form 1040 or 1040-SR, line 11b, minus the amounts on \
                            lines 13a and 13b of that form, more than $384,350?",
        }),
        _ => Section68Status::ThresholdNotTranscribed,
    }
}

#[cfg(test)]
pub(crate) mod section_68_tests {
    use super::*;

    /// The workspace root, from this crate's manifest directory.
    pub(crate) fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("crates/btctax-core -> workspace root")
            .to_path_buf()
    }

    /// Collapse runs of whitespace so a sentence the PDF text layer wrapped mid-line — and
    /// interleaved with the stub column's *"Total / Itemized / Deductions"* caption — can be
    /// compared with the sentence as a human reads it off the page.
    pub(crate) fn squash(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Every archived Schedule A revision, as `(year, squashed text)`, oldest first.
    ///
    /// ★ **Read out of the extract DIRECTORY, never from a list of years typed here** (the FR-197
    ///   shape, `return_refuse.rs::archived_w2_instructions`). Archiving `f1040sa--2027.txt` pulls a
    ///   new revision into every assertion below with no edit to this file — which is the entire
    ///   point, because the thing being guarded against is a year list that did not know its set
    ///   had grown.
    pub(crate) fn archived_schedule_a() -> Vec<(i32, String)> {
        let dir = repo_root().join("design/forms/extract");
        let mut out: Vec<(i32, String)> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
            .map(|e| e.expect("a readable directory entry").path())
            .filter_map(|p| {
                let name = p.file_name()?.to_str()?.to_string();
                // `f1040sa--2024.txt` and `f1040sa--2026-DRAFT.txt` are both revisions of the year
                // they name; a draft is the only document that exists for a year before finals.
                let year: i32 = name
                    .strip_prefix("f1040sa--")?
                    .strip_suffix(".txt")?
                    .trim_end_matches("-DRAFT")
                    .parse()
                    .ok()?;
                let text = std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()));
                Some((year, squash(&text)))
            })
            .collect();
        out.sort_by_key(|(y, _)| *y);
        // ★ A BROKEN GLOB MUST BE LOUD. An empty vector makes every loop below pass on nothing, which
        //   is the exact failure `design/HARNESS.md` B1 exists to refuse.
        assert!(
            out.len() >= 3,
            "the archived Schedule A extracts have moved or vanished — {} found under {}, and \
             everything below would then be measuring nothing",
            out.len(),
            dir.display()
        );
        out
    }

    /// Whether a Schedule A revision's text prints the §68 gate.
    ///
    /// ★ Two of the gate's phrases, so a re-extract that re-flows one of them still measures. Shared
    ///   rather than re-written per test: this is the ONE reading of the archived forms that every
    ///   §68 assertion in the crate keys off, including
    ///   `return_refuse::section_68_gate_tests::a_pre_2026_itemizer_over_the_threshold_files_unchanged`.
    pub(crate) fn prints_section_68_gate(text: &str) -> bool {
        text.contains("Your deductions may be limited")
            && text.contains("Itemized Deductions Worksheet")
    }

    /// ★★★ **THE YEAR SET IS THE FORMS' — in both directions.**
    ///
    /// A revision that prints the gate and is NOT [`Section68Status::Gated`] would file an unlimited
    /// itemized total, i.e. understate tax; a revision that does NOT print it and IS gated would
    /// refuse a return the IRS accepts. Both are asserted, per archived revision, so neither
    /// direction can drift silently.
    #[test]
    fn the_gated_years_are_read_off_the_archived_schedule_a_extracts() {
        let mut gated_by_form: Vec<i32> = Vec::new();
        for (year, text) in &archived_schedule_a() {
            let by_form = prints_section_68_gate(text);
            // ★★★ THE FULL THREE-WAY CLASSIFICATION, not `is Gated` vs `is not`. A plant that moved
            //     `SECTION_68_FIRST_YEAR` back to 2025 SURVIVED the two-way version of this check:
            //     TY2025 became `ThresholdNotTranscribed`, which is "not Gated" and passed — while
            //     REFUSING every TY2025 itemizer. A year whose form prints no gate must be
            //     `NotApplicable` and nothing else.
            let expected = if by_form { "Gated" } else { "NotApplicable" };
            let actual = match section_68_status(*year) {
                Section68Status::Gated(_) => "Gated",
                Section68Status::NotApplicable => "NotApplicable",
                Section68Status::ThresholdNotTranscribed => "ThresholdNotTranscribed",
            };
            assert_eq!(
                actual,
                expected,
                "f1040sa--{year}: the form {} print the §68 gate, so `section_68_status` must be \
                 {expected} and it is {actual}. `NotApplicable` on a gated year files an UNLIMITED \
                 itemized total and understates tax; `Gated` or `ThresholdNotTranscribed` on an \
                 ungated year REFUSES a return the IRS accepts",
                if by_form { "DOES" } else { "does NOT" },
            );
            if by_form {
                gated_by_form.push(*year);
            }
        }
        assert_eq!(
            gated_by_form,
            vec![2026],
            "TY2026 is the only archived Schedule A printing the §68 gate; if this moved, the \
             threshold for the new year must be transcribed from ITS form"
        );
        // Every year at or after the statutory start with no transcribed form fails CLOSED.
        assert_eq!(
            section_68_status(2027),
            Section68Status::ThresholdNotTranscribed,
            "a §68 year with no archived Schedule A must refuse, not file unlimited"
        );
        // ★ The statutory boundary, against the STATUTE's own date rather than against itself —
        //   `SECTION_68_FIRST_YEAR - 1` would move with the constant and measure nothing.
        assert_eq!(
            SECTION_68_FIRST_YEAR, 2026,
            "Pub. L. 119-21 §70111(c): \"taxable years beginning after December 31, 2025\""
        );
    }

    /// Where `sentence` stops matching `text`, or `Ok(pieces)` — the number of contiguous runs the
    /// sentence had to be broken into to be found.
    ///
    /// ★★★ **The extract does NOT carry this sentence contiguously, and pretending otherwise is how a
    /// citation check comes to check nothing.** `pdftotext -layout` renders Schedule A's stub column
    /// beside the cell, so the gate's two printed lines arrive with the caption *"Itemized"* wedged
    /// between *"…lines 13a and"* and *"13b of that form…"*. A `contains` over the squashed page
    /// therefore fails on a sentence that IS on the paper — and the tempting repair, dropping to
    /// "check a short fragment", is exactly the blindness `design/HARNESS.md` B1 was written about.
    ///
    /// So the match is COMPUTED: walk the sentence taking the longest run of words that appears
    /// contiguously from the current position, advance, repeat. The caller pins how many runs are
    /// allowed, which is what makes this an assertion about the form's layout rather than an ordered
    /// subsequence over the whole page (a subsequence would match almost anything).
    ///
    /// ★ Every word is load-bearing: the walk stops at the FIRST word that does not continue the run,
    ///   so a changed line number, a changed threshold or a paraphrased verb all report their own
    ///   position. `a_paraphrase_and_a_wrong_threshold_are_both_rejected` is the kill.
    pub(crate) fn appears_as_printed(text: &str, sentence: &str) -> Result<usize, String> {
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let mut cursor = 0usize;
        let mut from = 0usize;
        let mut pieces = 0usize;
        while cursor < words.len() {
            let mut run = 0usize;
            let mut next_from = from;
            for end in (cursor + 1)..=words.len() {
                let candidate = words[cursor..end].join(" ");
                match text[from..].find(&candidate) {
                    Some(at) => {
                        run = end - cursor;
                        next_from = from + at + candidate.len();
                    }
                    None => break,
                }
            }
            if run == 0 {
                return Err(format!(
                    "stops matching at word {cursor} ({:?}); matched {:?} so far",
                    words[cursor],
                    words[..cursor].join(" ")
                ));
            }
            cursor += run;
            from = next_from;
            pieces += 1;
        }
        Ok(pieces)
    }

    /// ★★★ **THE THRESHOLD AND THE SENTENCE ARE THE FORM'S OWN — and the sentence carries NO
    /// filing-status parenthetical, which is the FR-186 kill.**
    ///
    /// This is the assertion that makes reading `$384,350` off the §1(j)(2) MFS bracket table
    /// impossible to pass off as an equivalent: the numeral must appear inside the gate's own sentence
    /// in the extract, and the sentence must not carry the *"if married filing separately"* clause
    /// that line 5e directly above it DOES carry. The 5e half is checked too — a matcher that cannot
    /// see the parenthetical where it exists proves nothing by not seeing it where it does not.
    #[test]
    fn the_gate_sentence_and_threshold_are_the_forms_own() {
        use crate::tax::advisories::fmt_usd;
        let mut checked = 0usize;
        for (year, text) in &archived_schedule_a() {
            let Section68Status::Gated(g) = section_68_status(*year) else {
                continue;
            };
            checked += 1;
            let pieces = appears_as_printed(text, g.gate_sentence).unwrap_or_else(|e| {
                panic!(
                    "f1040sa--{year} must print the gate sentence verbatim, and it {e}. \
                     `Section68Gate::gate_sentence` reads {:?}",
                    g.gate_sentence
                )
            });
            assert_eq!(
                pieces, 2,
                "the gate prints on exactly two lines of f1040sa--{year}, interrupted once by the \
                 stub caption. A different count means the text layer was re-extracted with a \
                 different layout and this citation needs re-reading against the new one"
            );
            // The threshold, formatted the way the form prints it, INSIDE the gate's own sentence —
            // not merely somewhere on the page, where the bracket table's numeral could satisfy it.
            let printed = fmt_usd(g.threshold);
            assert!(
                squash(g.gate_sentence).contains(&printed),
                "the gate sentence must carry the threshold {printed}"
            );
            assert_eq!(
                text.matches(&printed).count(),
                1,
                "f1040sa--{year} prints {printed} exactly once — the gate. A second occurrence means \
                 the numeral now means two things on one form and this test can no longer tell them \
                 apart"
            );
            // ★ THE FR-186 KILL: no per-status split in the gate, and there IS one on 5e.
            assert!(
                !squash(g.gate_sentence).contains("married filing separately"),
                "the §68 gate prints ONE threshold for every filing status; a parenthetical here \
                 would mean the form had become status-aware and this type must grow a \
                 `FilingStatus` parameter"
            );
            assert!(
                text.contains("$40,400 ($20,200 if married filing"),
                "f1040sa--{year} line 5e DOES carry a filing-status parenthetical — asserted so the \
                 check above is known to be able to see one"
            );
        }
        assert_eq!(checked, 1, "exactly one archived Schedule A is gated today");
    }

    /// ★★★ **B1 — THE KILL. The citation checker above is watched distinguishing a true sentence from
    /// four false ones**, so *"which test reds when this checker is removed?"* has an answer.
    ///
    /// Each plant is a defect this project has actually made or nearly made:
    /// - the **threshold swapped for the Single 37% bracket start** ($640,600) — FR-186's trap, the
    ///   one that screens a filer OUT of a limitation the form screens them IN to;
    /// - the **threshold with a digit lost** — the Form 6251 line-33 class (a rendered `12` for `22`);
    /// - a **changed line number** (`11b` → `11`) — asserting a 1040 numbering FR-214 says rests on
    ///   one indirect witness;
    /// - a **paraphrase** presented as the form's words (*"greater than"* for *"more than"*).
    #[test]
    fn a_paraphrase_and_a_wrong_threshold_are_both_rejected() {
        let sa26 = archived_schedule_a()
            .into_iter()
            .find(|(y, _)| *y == 2026)
            .expect("f1040sa--2026-DRAFT must be archived")
            .1;
        let real = match section_68_status(2026) {
            Section68Status::Gated(g) => g.gate_sentence,
            other => panic!("TY2026 must be gated, got {other:?}"),
        };
        assert_eq!(
            appears_as_printed(&sa26, real),
            Ok(2),
            "the real sentence must be accepted, or every rejection below proves nothing"
        );
        for (plant, what) in [
            (
                real.replace("$384,350", "$640,600"),
                "the Single 37% bracket start (FR-186's trap)",
            ),
            (real.replace("$384,350", "$38,435"), "a lost digit"),
            (
                real.replace("line 11b", "line 11"),
                "a changed 1040 line number",
            ),
            (
                real.replace("more than", "greater than"),
                "a paraphrase of the form's own verb",
            ),
        ] {
            assert!(
                appears_as_printed(&sa26, &plant).is_err(),
                "{what} must be REJECTED — the checker accepted {plant:?}"
            );
        }
    }
}
/// ★★★ **THE 0.5% CHARITABLE FLOOR — AND ITS STATUTE IS §170(b)(1)(I), *NOT* §170(p).**
///
/// TY2026 Schedule A stops adding gifts straight into its charitable subtotal. Where TY2024 and
/// TY2025 print *"13 Carryover from prior year"* and *"14 Add lines 11 through 13"*, the TY2026
/// revision prints, on **line 13**:
///
/// > *"Enter the amount from line 6 of the Charitable Contribution Limitation Worksheet"*
/// > (`design/forms/extract/f1040sa--2026-DRAFT.txt:116-117`), with the carryover moved to line 14
/// > and *"15 Add lines 13 and 14"* as the subtotal.
///
/// ★★★ **THE NAMING CORRECTION, BECAUSE FIVE PLACES IN THIS REPO GET IT BACKWARDS.**
/// `design/direction/filing-readiness-lens-itemized.md:182`,
/// `design/direction/filing-readiness-lens-charity.md:9`,
/// `design/agent-reports/RECON-drive-to-filable-return.md:383`,
/// `design/agent-reports/REPORT-wave5-schedule-a.md:87` and
/// `crates/btctax-cli/tests/ty2026_schedule_a.rs` all call this floor **"§170(p)"**. It is not.
/// Reading Pub. L. 119-21 out of `legal/text/statute-irc/PLAW-119publ21_OBBBA.txt`:
///
/// | provision | what it actually is | archived at |
/// |---|---|---|
/// | **§170(b)(1)(I)** | **the 0.5% floor on an ITEMIZER's contributions** — added by OBBBA §70425(a)(1) | `PLAW-119publ21_OBBBA.txt:9400-9420` |
/// | §170(p) | the partial deduction for individuals who **do NOT elect to itemize** — $1,000/$2,000, raised by OBBBA §70424(a) | `PLAW-119publ21_OBBBA.txt:9391-9399` |
///
/// §70425(a)(3) touches §170(p) only to *coordinate* with it, *"by inserting `, (b)(1)(I),` after
/// `subsections (b)(1)(G)(ii)`"* — which is how a floor whose real home is `(b)(1)(I)` came to be
/// filed under `(p)` in this repo's prose. **The distinction is not pedantry: §170(p) is the
/// provision that applies precisely when the filer does NOT itemize, i.e. exactly the returns this
/// gate must leave alone.** A refusal citing it would hand a filer the wrong statute to look up.
///
/// ★★★ **THIS TYPE CARRIES THE FORM'S ROUTING AND NOTHING ELSE. btctax DOES NOT COMPUTE THE FLOOR,
/// DELIBERATELY — AND THERE IS NO RATE CONSTANT IN THIS FILE ON PURPOSE.** The floor's answer lives
/// in the **Charitable Contribution Limitation Worksheet** in the TY2026 Form 1040 / Schedule A
/// instructions, and **neither `i1040gi--2026` nor `i1040sca--2026` is archived** — the documents do
/// not exist yet. The statute *is* archived and the *rate* is therefore known, which makes the
/// temptation concrete and worth naming: `0.5% × AGI` is **not** the floor, for three reasons the
/// statute itself states.
///
/// 1. **The base is the *contribution base*, not AGI.** §170(b)(1)(I) floors at *"0.5 percent of the
///    taxpayer's contribution base"*, and §170(b)(1)(H) defines that as AGI computed **without regard
///    to any net operating loss carryback**. `REPORT-wave5-schedule-a.md`'s `0.005 × agi` is an
///    approximation that is exact only when no NOL carryback exists.
/// 2. **The floor is applied in a SIX-STEP ORDER ACROSS the §170(b)(1) subparagraphs** —
///    §170(b)(1)(I) clauses (i)-(vi) consume, in order, the contributions governed by subparagraphs
///    (D), (C), (B), (E), (A) and then (G). Which class absorbs the floor changes which class carries
///    forward, so the floor is **not** a subtraction from the total that
///    [`crate::tax::charitable::apply_170b`] already computes; it interleaves with the ceilings that
///    function applies.
/// 3. **It re-writes the carryforward rule.** §70425(a)(2) adds §170(d)(1)(C), under which an amount
///    disallowed *by the floor* increases the carryover — and only *"from years in which the
///    limitation is exceeded"*. So a floor applied naively would also emit a wrong
///    `CharitableResult::carryover_out`, i.e. a wrong figure on a **future** year's return.
///
/// Each of the three is a branch `CLAUDE.md` demands a written equivalence proof and a KAT for, and
/// none can be written from a worksheet nobody has read. So the branch **refuses**
/// ([`crate::tax::return_refuse::charitable_floor_gate`]), and this file stores **no** `0.005`: the
/// rate appears only inside [`CHARITABLE_FLOOR_STATUTE`], as the quotation it is. That absence is the
/// same structural move as [`Section68Gate`] having no `FilingStatus` field — there is nothing here
/// for a future editor to multiply by, and `no_floor_rate_constant_exists_in_this_crate` holds it.
///
/// ★★ **FR-233 — THE SEQUENCING TRAP, AT THE SITE.** This refusal must **not** be removed until the
/// floor is actually computed. Removing it re-exposes an understatement whose size on the owner's own
/// profile is already measured (`REPORT-wave5-schedule-a.md` §3: MFJ, AGI $273,200, $10,000 of
/// charity, **$1,366** of deduction §170(b)(1)(I) does not allow). The order is: archive
/// `i1040sca--2026`, transcribe the *Charitable Contribution Limitation Worksheet* line by line, pin
/// it against both oracles, **then** delete this gate — never the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharitableFloorGate {
    /// The tax year whose Schedule A this routing was transcribed from.
    pub year: i32,
    /// The Schedule A line that reads the worksheet's result (`"13"` on TY2026).
    pub worksheet_line: &'static str,
    /// The Schedule A line the charitable subtotal sits on behind this routing (`"15"` on TY2026;
    /// it was line 14 through TY2025, when it summed the gifts directly).
    pub subtotal_line: &'static str,
    /// The worksheet the line sends the filer to, in the form's own words.
    pub worksheet: &'static str,
    /// The line's caption, **verbatim from the text layer**, whitespace-squashed. One copy: the
    /// refusal quotes this and `the_floor_caption_and_worksheet_are_the_forms_own` asserts it against
    /// the extract, so the sentence btctax shows a filer cannot drift from the sentence on the paper.
    pub caption: &'static str,
}

/// ★★★ **§170(b)(1)(I) ITSELF, verbatim from the archived statute** —
/// `legal/text/statute-irc/PLAW-119publ21_OBBBA.txt:9405-9412`, as added by Pub. L. 119-21
/// §70425(a)(1).
///
/// ★ The typographic apostrophe in *taxpayer’s* is the statute's own (U+2019); the archive is a GPO
///   text render, and normalising it would make the citation test compare against something the
///   primary source does not say. `the_floor_statute_is_the_statutes_own_and_a_paraphrase_is_rejected`
///   matches this against the archive with the page-break artifact (*"139 STAT. 236 … Applicability."*)
///   that interrupts it mid-sentence accounted for, rather than by shortening the quotation until it
///   fits.
pub const CHARITABLE_FLOOR_STATUTE: &str =
    "Any charitable contribution otherwise allowable (without regard to this subparagraph) as a \
     deduction under this section shall be allowed only to the extent that the aggregate of such \
     contributions exceeds 0.5 percent of the taxpayer’s contribution base for the taxable year.";

/// ★★★ **THE FIRST TAX YEAR THE CHARITABLE FLOOR APPLIES TO — the ONE place that boundary is
/// written.**
///
/// Pub. L. 119-21 (OBBBA) §70425(c): *"The amendments made by this section shall apply to taxable
/// years beginning after December 31, 2025."* (`PLAW-119publ21_OBBBA.txt:9432-9434`, marginal note
/// *26 USC 170 note*.) Statute, with **no** sunset — writing it as a closed range would invent an
/// expiry Congress did not enact.
///
/// ★★ It is the identical date as [`SECTION_68_FIRST_YEAR`] (§70111(c)) and the two are **independent
/// constants on purpose**: they are different sections of OBBBA, and a future Congress moving one
/// must not silently move the other.
///
/// ★ That TY2024 and TY2025 Schedule A print no worksheet routing is asserted from the extracts by
/// `the_floored_years_are_read_off_the_archived_schedule_a_extracts`, not assumed here.
pub const CHARITABLE_FLOOR_FIRST_YEAR: i32 = 2026;

/// What §170(b)(1)(I) does to `year`'s return. Returned by [`charitable_floor_status`].
///
/// ★★★ **Three arms, and BOTH of the last two refuse.** Matched with no `_` at the one call site
/// ([`crate::tax::return_refuse::charitable_floor_gate`]), so a fourth arm is a compile error there.
///
/// ★★ **Unlike [`Section68Status`], there is no "computable" arm and there will not be one while the
/// worksheet is unarchived.** §68's gate at least prints a threshold that decides whether the
/// limitation binds; the charitable floor prints no screen at all, because it applies to **every**
/// itemizer who claims a contribution, at **every** income. That is the whole reason this gate exists
/// beside the §68 one: §68's screen fires only above $384,350, so a TY2026 itemizer *below* that with
/// charity was unguarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharitableFloorStatus {
    /// `year < CHARITABLE_FLOOR_FIRST_YEAR` — §170(b)(1)(I) did not exist and this year's Schedule A
    /// sums its gifts directly into the subtotal. The unfloored total is the **correct** return.
    NotApplicable,
    /// The floor applies and this year's Schedule A has been transcribed, so the refusal can quote
    /// the form's own line, caption and worksheet name.
    Gated(CharitableFloorGate),
    /// ★★★ **The floor applies and NO Schedule A revision for this year has been transcribed — so
    /// the refusal quotes the statute alone. It still refuses.**
    ///
    /// This arm is why [`charitable_floor_status`] is not "a list of years that print the routing".
    /// §170(b)(1)(I) has no sunset, so every year after 2025 is floored whether or not this tree has
    /// the form; a year that fell through to *"unfloored"* would deduct more than §170 allows,
    /// silently, in the understating direction. Adding TY2027 support is archiving `f1040sa--2027`
    /// **and** the worksheet, and forgetting to is a refusal rather than a wrong figure (`CLAUDE.md`,
    /// *"Derive the list, or make the compiler hold it"* — here the unknown year is held by the
    /// `match`'s own fall-through, not by a list).
    ScheduleANotTranscribed,
}

/// What §170(b)(1)(I) does to `year`'s return. See [`CharitableFloorStatus`].
///
/// ★ **What this covers, stated plainly** (`CLAUDE.md` rule (3)): exactly the Schedule A revisions
/// whose text layer is committed under `design/forms/extract/` AND route their charitable line
/// through the *Charitable Contribution Limitation Worksheet* — today, **TY2026 alone**. Every later
/// year is [`CharitableFloorStatus::ScheduleANotTranscribed`] and refuses; every earlier year is
/// [`CharitableFloorStatus::NotApplicable`] and files unchanged. The year set is not asserted here:
/// `the_floored_years_are_read_off_the_archived_schedule_a_extracts` derives it from the extract
/// directory and reds if this function and the archived forms disagree in either direction.
#[must_use]
pub fn charitable_floor_status(year: i32) -> CharitableFloorStatus {
    if year < CHARITABLE_FLOOR_FIRST_YEAR {
        return CharitableFloorStatus::NotApplicable;
    }
    match year {
        // `f1040sa--2026-DRAFT.txt:116-120`. Nothing numeric is transcribed here — see the
        // `CharitableFloorGate` doc on why this file stores no `0.005`.
        2026 => CharitableFloorStatus::Gated(CharitableFloorGate {
            year: 2026,
            worksheet_line: "13",
            subtotal_line: "15",
            worksheet: "Charitable Contribution Limitation Worksheet",
            caption: "Enter the amount from line 6 of the Charitable Contribution Limitation \
                      Worksheet",
        }),
        _ => CharitableFloorStatus::ScheduleANotTranscribed,
    }
}

#[cfg(test)]
pub(crate) mod charitable_floor_tests {
    use super::section_68_tests::{appears_as_printed, archived_schedule_a, repo_root, squash};
    use super::*;

    /// Whether a Schedule A revision routes its charitable line through the limitation worksheet.
    ///
    /// ★ Two phrases, so a re-extract that re-flows one of them still measures. Shared rather than
    ///   re-written per test: this is the ONE reading of the archived forms that every floor
    ///   assertion in the crate keys off, including
    ///   `return_refuse::charitable_floor_gate_tests::a_pre_2026_itemizer_with_charity_files_unchanged`.
    pub(crate) fn routes_through_the_limitation_worksheet(text: &str) -> bool {
        text.contains("Charitable Contribution Limitation Worksheet")
            && text.contains("Enter the amount from line 6")
    }

    /// ★★★ **THE YEAR SET IS THE FORMS' — in both directions.**
    ///
    /// A revision that routes through the worksheet and is NOT [`CharitableFloorStatus::Gated`] would
    /// file an unfloored charitable total, i.e. understate tax; a revision that does NOT route through
    /// it and IS gated would refuse a return the IRS accepts. Both are asserted, per archived
    /// revision, so neither direction can drift silently.
    ///
    /// ★★ The full THREE-WAY classification, not "is Gated" vs "is not" — the two-way form of this
    ///    check is what let a plant on [`SECTION_68_FIRST_YEAR`] survive in the §68 parcel (a year
    ///    that became `ThresholdNotTranscribed` is "not Gated" and passed, while refusing every
    ///    return of that year).
    #[test]
    fn the_floored_years_are_read_off_the_archived_schedule_a_extracts() {
        let mut floored_by_form: Vec<i32> = Vec::new();
        for (year, text) in &archived_schedule_a() {
            let by_form = routes_through_the_limitation_worksheet(text);
            let expected = if by_form { "Gated" } else { "NotApplicable" };
            let actual = match charitable_floor_status(*year) {
                CharitableFloorStatus::Gated(_) => "Gated",
                CharitableFloorStatus::NotApplicable => "NotApplicable",
                CharitableFloorStatus::ScheduleANotTranscribed => "ScheduleANotTranscribed",
            };
            assert_eq!(
                actual,
                expected,
                "f1040sa--{year}: the form {} route its charitable line through the limitation \
                 worksheet, so `charitable_floor_status` must be {expected} and it is {actual}. \
                 `NotApplicable` on a floored year files an UNFLOORED charitable total and \
                 understates tax; `Gated` or `ScheduleANotTranscribed` on an unfloored year REFUSES \
                 a return the IRS accepts",
                if by_form { "DOES" } else { "does NOT" },
            );
            if by_form {
                floored_by_form.push(*year);
            }
        }
        assert_eq!(
            floored_by_form,
            vec![2026],
            "TY2026 is the only archived Schedule A routing its charitable line through the \
             Charitable Contribution Limitation Worksheet; if this moved, the new year's form must \
             be read too"
        );
        // Every year at or after the statutory start with no transcribed form fails CLOSED.
        assert_eq!(
            charitable_floor_status(2027),
            CharitableFloorStatus::ScheduleANotTranscribed,
            "a floored year with no archived Schedule A must refuse, not file unfloored"
        );
        // ★ FR-230 — the boundary against the STATUTE's own date, never against itself.
        //   `CHARITABLE_FLOOR_FIRST_YEAR - 1` would move with the constant and measure nothing.
        assert_eq!(
            CHARITABLE_FLOOR_FIRST_YEAR, 2026,
            "Pub. L. 119-21 §70425(c): \"taxable years beginning after December 31, 2025\""
        );
        // ★★ The two OBBBA sections are independent constants, each pinned to the same date by its
        //    OWN citation — `CHARITABLE_FLOOR_FIRST_YEAR == SECTION_68_FIRST_YEAR` would pass if
        //    BOTH were wrong, which is the FR-230 shape one assertion up.
        assert_eq!(
            SECTION_68_FIRST_YEAR, 2026,
            "Pub. L. 119-21 §70111(c): \"taxable years beginning after December 31, 2025\""
        );
    }

    /// ★★★ **THE CAPTION IS THE FORM'S OWN, AND THE PRE-2026 FORMS DO NOT CARRY IT.**
    ///
    /// Both halves matter. A matcher that cannot see the caption where it exists proves nothing by
    /// not seeing it where it does not — so TY2024/TY2025 are checked to print *"Carryover from prior
    /// year"* on line 13 and *"Add lines 11 through 13"* as the subtotal, which is the routing the
    /// 2026 revision replaced.
    #[test]
    fn the_floor_caption_and_worksheet_are_the_forms_own() {
        let mut checked = 0usize;
        for (year, text) in &archived_schedule_a() {
            match charitable_floor_status(*year) {
                CharitableFloorStatus::Gated(g) => {
                    checked += 1;
                    let pieces = appears_as_printed(text, g.caption).unwrap_or_else(|e| {
                        panic!(
                            "f1040sa--{year} must print the line-{} caption verbatim, and it {e}. \
                             `CharitableFloorGate::caption` reads {:?}",
                            g.worksheet_line, g.caption,
                        )
                    });
                    assert_eq!(
                        pieces, 1,
                        "the caption is contiguous in the squashed extract; {pieces} runs means the \
                         layout moved and this assertion is no longer reading one sentence"
                    );
                    assert!(
                        text.contains(g.worksheet),
                        "f1040sa--{year} must name {:?}",
                        g.worksheet
                    );
                    // ★★ THE LINE NUMBERS THE REFUSAL PRINTS, each bound to the caption beside it
                    //    in the extract — so a renumber cannot leave the refusal citing a line that
                    //    moved. `worksheet_line` must carry the caption; the carryover must have
                    //    moved DOWN one; the subtotal must add the two.
                    assert!(
                        text.contains(&format!(
                            "{} Enter the amount from line 6",
                            g.worksheet_line
                        )),
                        "f1040sa--{year}: the caption must sit on line {}, which is the number the \
                         refusal tells the filer to enter the worksheet's result on",
                        g.worksheet_line,
                    );
                    assert!(
                        text.contains("14 Carryover from prior year"),
                        "f1040sa--{year}: the carryover must have moved DOWN to line 14 — the \
                         refusal says so, and the pre-2026 arm below asserts it was on 13"
                    );
                    assert!(
                        text.contains(&format!("{} Add lines 13 and 14", g.subtotal_line)),
                        "f1040sa--{year} must print \"{} Add lines 13 and 14\"",
                        g.subtotal_line
                    );
                }
                CharitableFloorStatus::NotApplicable => {
                    // The routing the 2026 revision replaced — so the matcher is watched on a form
                    // that genuinely lacks the caption, not merely on one it fails to parse.
                    assert!(
                        text.contains("13 Carryover from prior year")
                            && text.contains("14 Add lines 11 through 13"),
                        "f1040sa--{year} is not floored, so it must still sum gifts directly: \
                         line 13 the carryover, line 14 \"Add lines 11 through 13\""
                    );
                    assert!(
                        !text.contains("Charitable Contribution Limitation Worksheet"),
                        "f1040sa--{year} must NOT name the limitation worksheet"
                    );
                }
                CharitableFloorStatus::ScheduleANotTranscribed => panic!(
                    "f1040sa--{year} IS archived but `charitable_floor_status` has no arm for it — \
                     transcribe that revision's charitable routing before this can pass"
                ),
            }
        }
        assert_eq!(
            checked, 1,
            "exactly one archived revision is floored today; if that changed, the new one's caption \
             must be transcribed from ITS form"
        );
    }

    /// ★★★ **THE STATUTE QUOTED TO THE FILER IS THE STATUTE'S OWN — matched against the archived
    /// Public Law, and the KILL is in the loop.**
    ///
    /// ★★ The GPO render breaks this sentence across a page boundary: *"…exceeds 0.5 percent of the
    ///    taxpayer's contribution base"* / *"139 STAT. 236 PUBLIC LAW 119-21—JULY 4, 2025"* /
    ///    *"Applicability."* / *"for the taxable year."* A `contains` over the squashed page therefore
    ///    fails on a sentence that IS in the law, and the tempting repair — shortening the quotation
    ///    until it fits — is exactly the blindness `design/HARNESS.md` B1 was written about. So the
    ///    number of contiguous runs is COMPUTED and pinned.
    #[test]
    fn the_floor_statute_is_the_statutes_own_and_a_paraphrase_is_rejected() {
        let path = repo_root().join("legal/text/statute-irc/PLAW-119publ21_OBBBA.txt");
        let law = squash(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display())),
        );
        // ★ A POSITIVE CONTROL FIRST: a test reading the wrong file would reject every plant below
        //   and look exactly like five successful kills. §70425's own heading is unquestionably there.
        assert!(
            law.contains("0.5 PERCENT FLOOR ON DEDUCTION OF CONTRIBUTIONS"),
            "{} is not the OBBBA text this test needs",
            path.display()
        );

        let pieces = appears_as_printed(&law, CHARITABLE_FLOOR_STATUTE).unwrap_or_else(|e| {
            panic!("§170(b)(1)(I) must appear verbatim in the archived Public Law, and it {e}")
        });
        assert_eq!(
            pieces, 2,
            "the statute is interrupted once by the 139 STAT. 236 page break; {pieces} runs means \
             the archive was re-rendered and this quotation must be re-read against it"
        );

        // ★★★ THE KILL — AND THE BUDGET IS PART OF IT, which the first draft of this test got
        //     wrong and the plant pass caught. `is_err()` alone is NOT a rejection here: the walk
        //     in `appears_as_printed` may break a sentence into more runs, and *"5 percent"* is a
        //     SUBSTRING of the law's own *"0.5 percent"*, so the lost-decimal-point plant was
        //     ACCEPTED — in 3 runs instead of 2 — and the kill silently passed nothing. A plant is
        //     rejected only if it cannot be found at the REAL sentence's fragmentation.
        let rejected_within_budget = |plant: &str| match appears_as_printed(&law, plant) {
            Err(_) => true,
            Ok(p) => p > pieces,
        };
        for (plant, what) in [
            (
                CHARITABLE_FLOOR_STATUTE.replace("0.5 percent", "5 percent"),
                "a lost decimal point — the Form 6251 line-33 class",
            ),
            (
                CHARITABLE_FLOOR_STATUTE.replace("contribution base", "adjusted gross income"),
                "AGI substituted for the contribution base (§170(b)(1)(H))",
            ),
            (
                CHARITABLE_FLOOR_STATUTE.replace("exceeds", "does not exceed"),
                "the floor inverted into a ceiling",
            ),
            (
                CHARITABLE_FLOOR_STATUTE.replace("aggregate of such", "aggregate of all"),
                "a paraphrase of the statute's own scope",
            ),
            (
                CHARITABLE_FLOOR_STATUTE.replace("otherwise allowable", "otherwise allowed"),
                "a paraphrase of the statute's own term of art",
            ),
        ] {
            assert!(
                rejected_within_budget(&plant),
                "{what} must be REJECTED — the checker found it within {pieces} runs, the real \
                 sentence's own fragmentation: {plant:?}"
            );
        }
        // ★ AND THE CONTROL, so a predicate that rejected everything could not pass as five kills.
        assert!(
            !rejected_within_budget(CHARITABLE_FLOOR_STATUTE),
            "the unmutated sentence must still be ACCEPTED within its own budget"
        );
    }

    /// ★★ **THE FLOOR IS NOT MODELLED, AND THAT IS ASSERTED RATHER THAN CLAIMED — the FR-233
    /// sequencing guard, as a test.** No 0.5% rate constant lives in this crate's source: the only
    /// *"0.5 percent"* a grep can find is inside [`CHARITABLE_FLOOR_STATUTE`]'s prose, as the
    /// quotation it is. If a future editor adds one, this reds and sends them to the
    /// [`CharitableFloorGate`] doc, which names the three statutory branches a naive `0.005 × agi`
    /// gets wrong.
    #[test]
    fn no_floor_rate_constant_exists_in_this_crate() {
        let src = repo_root().join("crates/btctax-core/src");
        let mut stack = vec![src.clone()];
        let mut hits: Vec<String> = Vec::new();
        let mut files = 0usize;
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).expect("the crate source is readable") {
                let p = e.expect("a readable dir entry").path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    files += 1;
                    let t = std::fs::read_to_string(&p).unwrap_or_default();
                    // ★★★ ASSEMBLED, NEVER SPELLED. A literal needle here makes this file its
                    //     own hit — measured: the first draft of this test reported `tables.rs` as
                    //     a defect, and so did `ty2026_schedule_a.rs`'s wall pin, because the only
                    //     occurrence in the workspace was the needle list itself. `concat!` is
                    //     const-folded, so the literal never exists in the source.
                    for needle in [
                        concat!("dec!(0", ".005)"),
                        concat!("half", "_percent"),
                        concat!("charitable_floor", "_rate"),
                    ] {
                        if t.contains(needle) {
                            hits.push(format!("{}: {needle}", p.display()));
                        }
                    }
                }
            }
        }
        // ★ POSITIVE CONTROL: the walk must actually be reading files, or "no hits" is vacuous —
        //   and it must be reading the file that would carry such a constant.
        assert!(files >= 10, "the walk read only {files} source files");
        assert!(
            std::fs::read_to_string(src.join("tax/charitable.rs"))
                .expect("charitable.rs is readable")
                .contains("dec!(0.50)"),
            "the §170(b) ceilings must still be in charitable.rs, or this walk is reading nothing"
        );
        assert!(
            hits.is_empty(),
            "a 0.5% rate constant now exists — if the Charitable Contribution Limitation Worksheet \
             has been archived and transcribed, delete `charitable_floor_gate` in the SAME change \
             (FR-233); if not, this is the closed form `CLAUDE.md` forbids:\n{}",
            hits.join("\n")
        );
    }
}

/// ★★★ **§163(h)(3)(B) — THE HOME ACQUISITION-DEBT CEILINGS**, transcribed from the four figures
/// the Schedule A instructions print under *Limits on home mortgage interest* (R8 / T9).
///
/// > *"Limit on loans taken out on or before December 15, 2017. For qualifying debt taken out on or
/// > before December 15, 2017, you can only deduct home mortgage interest on up to $1,000,000
/// > ($500,000 if you are married filing separately) of that debt."*
/// > *"Limit on loans taken out after December 15, 2017. For qualifying debt taken out after
/// > December 15, 2017, you can only deduct home mortgage interest on up to $750,000 ($375,000 if
/// > you are married filing separately) of that debt."*
/// > (`design/forms/extract/i1040sca--2025.txt:1027-1046`; Pub. 936, *Part II — Limits on Home
/// > Mortgage Interest Deduction*.)
///
/// ★★ **All four figures are TRANSCRIBED, not two figures and a halving rule.** The instruction
/// prints the MFS amount beside each limit, so carrying it is following the form; deriving it would
/// be the compression this repo's standing rule exists to refuse — and a year in which Congress
/// moved one without the other would be silent.
///
/// ★ **Nothing here writes a line.** The ceilings drive a **WARNING** over Σ box 2; Schedule A line
/// 8a stays the filer's own `MortgageWithinDebtLimit` testimony, because the deductible figure on an
/// over-limit return is a nonzero output of Pub. 936's *Deductible Home Mortgage Interest Worksheet*
/// that btctax does not model.
///
/// ★ Not indexed and not expiring: Pub. L. 119-21 (OBBBA) §70108(a) struck *", and before January 1,
/// 2026"* from §163(h)(3)(F)(i) and re-headed it *"BEGINNING AFTER 2017"*, so the $750,000 limit is
/// permanent (`legal/text/statute-irc/PLAW-119publ21_OBBBA.txt:5211-5229`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcquisitionDebtCeiling {
    /// *"up to $750,000"* — qualifying debt taken out AFTER December 15, 2017.
    pub after_dec_15_2017: Usd,
    /// *"($375,000 if you are married filing separately)"*.
    pub after_dec_15_2017_mfs: Usd,
    /// *"up to $1,000,000"* — qualifying debt taken out ON OR BEFORE December 15, 2017.
    pub on_or_before_dec_15_2017: Usd,
    /// *"($500,000 if you are married filing separately)"*.
    pub on_or_before_dec_15_2017_mfs: Usd,
}

impl AcquisitionDebtCeiling {
    /// The ceiling for `status` and a loan originated on `origination` (Form 1098 box 3).
    ///
    /// ★★ **`None` — an origination date the filer never transcribed — takes the STRICTER
    /// post-2017 ceiling.** That is the fail-closed direction for a WARNING: an unknown date can
    /// only make the warning fire sooner, never later, so a blank box 3 cannot silence it.
    #[must_use]
    pub fn for_loan(&self, status: FilingStatus, origination: Option<time::Date>) -> Usd {
        // *"on or before December 15, 2017"* versus *"after December 15, 2017"* — the instruction's
        // own boundary, so the older, larger limit needs a date at or before 2017-12-15.
        let grandfathered = origination.is_some_and(|d| {
            d <= time::Date::from_calendar_date(2017, time::Month::December, 15)
                .expect("2017-12-15 is a valid date")
        });
        match (grandfathered, status) {
            (true, FilingStatus::Mfs) => self.on_or_before_dec_15_2017_mfs,
            (true, _) => self.on_or_before_dec_15_2017,
            (false, FilingStatus::Mfs) => self.after_dec_15_2017_mfs,
            (false, _) => self.after_dec_15_2017,
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
    /// ★★★ **§163(h)(3)(B) — the home acquisition-debt ceilings** (R8 / T9). See
    /// [`AcquisitionDebtCeiling`]: they drive a WARNING over the SUM of Form 1098 box 2, never a
    /// printed line.
    pub acquisition_debt_ceiling: AcquisitionDebtCeiling,
    /// ★★★ **§152(d)(1)(B) — the qualifying-relative GROSS INCOME limit** (T7 / R6, Step 4).
    ///
    /// The flowchart states the test with the year's figure in it: *"Who had gross income of less
    /// than **$5,200** in 2025"* (`design/forms/extract/i1040gi--2025.txt:1690`). It is the §151(d)
    /// exemption amount republished annually — **TY2024 $5,050** (Rev. Proc. 2023-34 §3.24), **TY2025
    /// $5,200** (Rev. Proc. 2024-40 §2.24), **TY2026 $5,300** (Rev. Proc. 2025-32 §4.23) — so it is a
    /// per-year parameter and not a constant.
    ///
    /// ★★ **The gate's prompt QUOTES it**, which is why the gate is *waiting for the year package*
    /// rather than blocking on a year with no [`FullReturnParams`] (R6/M7, R12's `waiting` row): a
    /// prompt that cannot state the figure asks the filer to derive it, and a derived answer to a
    /// §6065 declaration is exactly what this interview exists to prevent.
    pub qualifying_relative_gross_income_limit: Usd,
    /// ★★★ **§24(h)(2) — the CHILD TAX CREDIT per qualifying child**, and **§24(h)(4)** below for the
    /// CREDIT FOR OTHER DEPENDENTS per other qualifying person.
    ///
    /// btctax computes NEITHER credit: 1040 line 19 is a carry from Schedule 8812, which it does not
    /// file, so the line is the filer's own blank (`advisories::ctc_odc_line19`). These figures exist
    /// so the R12 panel can SIZE that forgo — *"child tax credit not computed — n children with a
    /// credit box; up to $X each"* — from the year's package rather than from a number typed beside
    /// it. On a params-less year the panel prints the count and no size, which is R12's own rule: a
    /// figure invented from no package is worse than a gap the filer can see.
    ///
    /// ★ Year-scoped, not constant: §24(h)(2)'s $2,000 is the TCJA figure through **TY2024**, and
    ///   Pub. L. 119-21 (OBBBA) §70104(a)(2) raises it to $2,200 for *"taxable years beginning after
    ///   December 31, 2024"* (§70104(f)) — so TY2025 is already $2,200 — with §24(i)(2) indexing it
    ///   for years beginning after 2025.
    pub child_tax_credit_per_child: Usd,
    /// §24(h)(4) — *"$500 for each dependent of the taxpayer other than a qualifying child"*.
    pub credit_for_other_dependents_per_person: Usd,
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
    /// §223(b) HSA contribution limitation — Form 8889 line 3 and line 7 (T16). See [`HsaParams`].
    pub hsa: HsaParams,
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

/// ★★★ **THE YEARS A SCHEDULE 1-A EXISTS FOR — the ONE place the window is written.**
///
/// [`schedule_1a_params`] tests membership of it and
/// [`crate::tax::return_refuse::RefuseReason::Schedule1aNotOnThisYearsReturn`] states it to the
/// filer, so the refusal's sentence and the table's `None` can never name different years
/// (`CLAUDE.md`, *"Derive the list, or make the compiler hold it"* — a window retyped inside a
/// message is exactly the list that a later sunset edit walks past).
///
/// The form was created by Pub. L. 119-21 (OBBBA), and the four provisions it carries expire after
/// TY2028 — §224(f) (tips), §225(f) (overtime), §163(h)(4)(F) (car-loan interest) and §151(d)(5)(D)
/// (the senior deduction).
pub const SCHEDULE_1A_YEARS: std::ops::RangeInclusive<i32> = 2025..=2028;

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
    if !SCHEDULE_1A_YEARS.contains(&year) {
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

    /// The Schedule 1-A **instructions** (`i1040gi--2025.pdf` pp. 101-110), as `pdftotext -layout`
    /// extracts them. **IN-CRATE deliberately**, exactly like [`SCHEDULE_1A_FORM_TEXT`]: an
    /// `include_str!` reaching outside the crate ships a broken tarball with exit 0.
    ///
    /// ★★ The four worksheets exist ONLY here. `grep -c "Keep for Your Records"` on the FORM extract is
    /// **0**, which is why a census driven off the form alone could never red on a worksheet omission —
    /// it would have passed by finding nothing.
    const SCHEDULE_1A_INSTRUCTIONS_TEXT: &str =
        include_str!("fixtures/schedule_1a_2025_instructions.txt");

    /// The struct under test, read as SOURCE TEXT because doc comments do not exist at runtime.
    ///
    /// ★ In-crate only, and that is the second independent reason this half of the KAT lives in
    /// `btctax-core` rather than in `xtask`. The one in-tree precedent is `classifier.rs`, which
    /// `include_str!`s `return_inputs.rs` and itself for the same reason.
    const SCHEDULE_1A_SOURCE: &str = include_str!("schedule_1a.rs");

    /// Whitespace-normalized, because `pdftotext -layout` wraps clauses mid-sentence.
    fn norm(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// The rightmost column a MARGIN label may start at.
    ///
    /// ★★★ **This is the one number in the reader, and it is what keeps a body-text digit out.** The
    /// form prints each line number in the margin AND again in the gutter beside its amount box, and it
    /// also prints label-shaped tokens inside prose — line 4's own text wraps onto a line that BEGINS
    /// `"4b and see the instructions…"` at column 10. Indentation is the text layer's proxy for x, and
    /// every genuine margin label on this form starts at column 6 or less while every continuation that
    /// begins label-shaped starts further right. ★ The number is not trusted on its own: the reader's
    /// output is asserted to be exactly the **50** labels that `xtask`'s two geometry witnesses derive
    /// from the AcroForm by a completely independent route, so a mis-parse cannot pass quietly.
    const MARGIN_COLUMN: usize = 6;

    fn is_numeric_label(t: &str) -> bool {
        let d = t.trim_end_matches(|c: char| c.is_ascii_lowercase());
        !d.is_empty() && d.len() <= 2 && d.chars().all(|c| c.is_ascii_digit()) && t.len() <= 3
    }

    fn is_bare_letter(t: &str) -> bool {
        t.len() == 1 && t.chars().all(|c| c.is_ascii_lowercase())
    }

    /// Every label the form prints, mapped to its own printed text.
    ///
    /// A physical line OPENS a label when its first token is label-shaped and starts at or before
    /// [`MARGIN_COLUMN`]; a bare letter continues the current numeric stem (`b` under `2a` is `2b`),
    /// which is how the form abbreviates its sub-rows. A line indented four columns or more that opens
    /// nothing CONTINUES the open span; a blank line or a line at the left edge closes it — the left
    /// edge is where the part headers, the Cautions and the page furniture print.
    fn spans() -> BTreeMap<String, String> {
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        let mut open: Option<String> = None;
        let mut stem = String::new();
        for raw in SCHEDULE_1A_FORM_TEXT.lines() {
            if raw.trim().is_empty() {
                open = None;
                continue;
            }
            let indent = raw.len() - raw.trim_start().len();
            let tok = raw.split_whitespace().next().unwrap_or("");
            if indent <= MARGIN_COLUMN && (is_numeric_label(tok) || is_bare_letter(tok)) {
                let label = if is_numeric_label(tok) {
                    stem = tok.chars().take_while(char::is_ascii_digit).collect();
                    tok.to_string()
                } else {
                    format!("{stem}{tok}")
                };
                let rest = raw.trim_start()[tok.len()..].trim().to_string();
                assert!(
                    out.insert(label.clone(), rest).is_none(),
                    "label {label:?} is opened twice — the reader has mis-parsed the margin, and a \
                     silently overwritten span would make every quotation check read the wrong line"
                );
                open = Some(label);
            } else if indent >= 4 {
                if let Some(l) = &open {
                    let e = out.get_mut(l).expect("an open span is in the map");
                    if !e.is_empty() {
                        e.push(' ');
                    }
                    e.push_str(raw.trim());
                }
            } else {
                open = None;
            }
        }
        out
    }

    /// The printed text of numbered line `label`: the line the form prints it on in the margin, plus
    /// every continuation line. Returns `None` if the label is absent — which is itself a finding, so
    /// callers `expect` it rather than defaulting.
    fn printed_line(label: &str) -> Option<String> {
        spans().remove(label)
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

    // ══════════════════════════════════════════════════════════════════════════════════════════════
    //  THE SCHEDULE 1-A CONFORMANCE KAT — halves 1b, 2, 3 and 4.
    //
    //  ★★★ Half 1a — MEMBERSHIP against the form's 48 printed labels — is NOT here. It is
    //  `xtask::schedule_1a_membership`, because its instrument is `label_reader`'s two geometry
    //  witnesses over `design/forms/geometry/f1040s1a--2025.json`, a repo-root fixture this crate
    //  deliberately cannot reach. Membership is the one half that splits across the crate line, and
    //  only because its two sources differ: the FORM has an AcroForm the box witness can read, and the
    //  INSTRUCTIONS (where the four worksheets live) have none at all.
    //
    //  ★★ Every half below is a PURE FUNCTION over its inputs, called twice: once on the real
    //  artifact, which must be clean, and once on a planted defect, which must be caught. A checker
    //  that can only read the real file cannot be watched going red, and B1's reviewable question —
    //  *"which test reds when this checker is removed?"* — then has the answer "none".
    // ══════════════════════════════════════════════════════════════════════════════════════════════

    use crate::tax::line_coverage::{cover_schedule1a, LineCoverage, Production};
    use crate::tax::schedule_1a::{
        self, Leaf, Schedule1A, Schedule1aCompletion, WorksheetShape, NON_MONEY_LEAVES,
    };
    use std::collections::BTreeSet;

    // ───────────────────────── half 1b — the four worksheets ─────────────────────────

    /// One worksheet as the INSTRUCTIONS print it: title, the Schedule 1-A line its total feeds, its
    /// lettered columns and its lettered rows, all read off the text layer.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct PrintedWorksheet {
        title: String,
        target_line: String,
        columns: Vec<String>,
        rows: Vec<String>,
        /// The whole block, normalized — the haystack a column header must appear in.
        window: String,
    }

    /// The worksheets the instructions actually print, found by their own anchor.
    ///
    /// ★★ **THE ANCHOR COUNT IS PINNED, and that is the difference between this and a checker that
    /// passes by finding nothing.** Every one of the four sheets ends its title with
    /// `— Keep for Your Records`; if a fixture regeneration ever drops the anchor, this returns an
    /// error rather than an empty list that would make the whole half vacuous.
    fn printed_worksheets(fixture: &str) -> Result<Vec<PrintedWorksheet>, String> {
        const ANCHOR: &str = "— Keep for Your Records";
        let lines: Vec<&str> = fixture.lines().collect();
        let at: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains(ANCHOR))
            .map(|(i, _)| i)
            .collect();
        if at.len() != 4 {
            return Err(format!(
                "expected 4 `{ANCHOR}` anchors in the Schedule 1-A instructions, found {} — a reader \
                 that finds nothing must FAIL, never pass by having nothing to check",
                at.len()
            ));
        }
        let mut out = Vec::new();
        for (k, &i) in at.iter().enumerate() {
            let end = at.get(k + 1).copied().unwrap_or(lines.len());
            let title = norm(&lines[i].replace(ANCHOR, ""))
                .trim_matches(|c: char| c.is_control())
                .trim()
                .to_string();
            let block = norm(&lines[i..end].join(" "));
            // The sheet ends where it says where its total goes.
            let needle = "Schedule 1-A, line ";
            let cut = block
                .find(needle)
                .ok_or_else(|| format!("worksheet {title:?} never names its Schedule 1-A line"))?;
            let target: String = block[cut + needle.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
                .collect();
            let window = block[..cut + needle.len() + target.len()].to_string();
            let mut columns = Vec::new();
            let bytes: Vec<char> = window.chars().collect();
            for (n, w) in bytes.windows(3).enumerate() {
                if w[0] == '('
                    && w[1].is_ascii_lowercase()
                    && w[2] == ')'
                    && (n == 0 || !bytes[n - 1].is_alphanumeric())
                {
                    let c = w[1].to_string();
                    if !columns.contains(&c) {
                        columns.push(c);
                    }
                }
            }
            let rows: Vec<String> = ["A", "B", "C", "D", "E"]
                .into_iter()
                .filter(|r| window.split(' ').any(|w| w == *r))
                .map(str::to_string)
                .collect();
            out.push(PrintedWorksheet {
                title,
                target_line: target,
                columns,
                rows,
                window,
            });
        }
        Ok(out)
    }

    /// Does the transcription match what the instructions print? Pure, so it can be watched going red.
    ///
    /// ★ **The two headers the text layer cannot bind are COUNTED, not waved through.** The *Multiple
    /// Trades or Businesses* sheet's (a) and (b) headers are interleaved by the two-column reflow —
    /// the block reads `"(a) Name of (b) Net your business profit of business from Schedule C…"` — so
    /// no contiguous run of either header survives. Those two are checked as an in-order SUBSEQUENCE
    /// of the block instead, which still rejects a paraphrase or a reordering, and the number of them
    /// is pinned so a third cannot appear silently.
    fn worksheet_violations(
        shapes: &[WorksheetShape<'_>],
        printed: &[PrintedWorksheet],
    ) -> Vec<String> {
        let mut errs = Vec::new();
        if shapes.len() != printed.len() {
            errs.push(format!(
                "{} worksheets are transcribed but the instructions print {}",
                shapes.len(),
                printed.len()
            ));
            return errs;
        }
        let mut unbindable = Vec::new();
        for p in printed {
            let Some(s) = shapes.iter().find(|s| s.title == p.title) else {
                errs.push(format!(
                    "the instructions print a worksheet nothing transcribes: {:?}",
                    p.title
                ));
                continue;
            };
            if s.target_line != p.target_line {
                errs.push(format!(
                    "{:?} enters its total on Schedule 1-A line {:?}, transcribed as {:?}",
                    p.title, p.target_line, s.target_line
                ));
            }
            let have: Vec<String> = s.columns.iter().map(|(l, _, _)| l.to_string()).collect();
            if have != p.columns {
                errs.push(format!(
                    "{:?} prints columns {:?}, transcribed as {:?}",
                    p.title, p.columns, have
                ));
            }
            let rows: Vec<String> = s.rows.iter().map(|r| r.to_string()).collect();
            if rows != p.rows {
                errs.push(format!(
                    "{:?} prints rows {:?}, transcribed as {:?}",
                    p.title, p.rows, rows
                ));
            }
            for (letter, header, _) in &s.columns {
                let want = norm(header);
                if p.window.contains(&want) {
                    continue;
                }
                if is_ordered_subsequence(&want, &p.window) {
                    unbindable.push(format!("{}({letter})", p.title));
                } else {
                    errs.push(format!(
                        "{:?} column ({letter}): the transcribed header is neither printed verbatim \
                         nor present in order — {want:?}",
                        p.title
                    ));
                }
            }
        }
        if unbindable.len() != 2 {
            errs.push(format!(
                "{} column header(s) could not be bound verbatim; exactly 2 are expected (the \
                 Multiple Trades or Businesses sheet's (a) and (b), interleaved by the two-column \
                 reflow): {unbindable:?}",
                unbindable.len()
            ));
        }
        errs
    }

    /// Do `needle`'s words appear in `haystack` in order (not necessarily adjacently)?
    fn is_ordered_subsequence(needle: &str, haystack: &str) -> bool {
        let mut hay = haystack.split(' ');
        needle.split(' ').all(|w| hay.any(|h| h == w))
    }

    /// ★★★ **HALF 1b — the four worksheets, driven off the instructions' own anchors.**
    #[test]
    fn the_four_worksheets_are_transcribed_as_the_instructions_print_them() {
        let printed =
            printed_worksheets(SCHEDULE_1A_INSTRUCTIONS_TEXT).expect("four anchored worksheets");
        let ws = schedule_1a::Schedule1aWorksheets::default();
        let errs = worksheet_violations(&ws.shapes(), &printed);
        assert!(errs.is_empty(), "{errs:#?}");
        // The four targets, stated so a silent re-pointing is visible in the diff as well as in the run.
        let targets: Vec<&str> = printed.iter().map(|p| p.target_line.as_str()).collect();
        assert_eq!(targets, ["4c", "5", "14a", "14b"]);
    }

    /// ★★★ **B1 for half 1b — the plant the r6 fold left this half without.**
    #[test]
    fn a_dropped_worksheet_row_or_column_or_sheet_is_caught_and_a_fixture_with_no_anchors_errors() {
        let printed =
            printed_worksheets(SCHEDULE_1A_INSTRUCTIONS_TEXT).expect("four anchored worksheets");
        let ws = schedule_1a::Schedule1aWorksheets::default();
        let good = ws.shapes();
        assert!(
            worksheet_violations(&good, &printed).is_empty(),
            "the control must PASS, or every plant below passes for the wrong reason"
        );

        // (1) A DROPPED COLUMN — the shape a `min()` in the emitter would have hidden entirely.
        let mut one_column_short = good.clone();
        one_column_short[0].columns.pop();
        assert!(
            worksheet_violations(&one_column_short, &printed)
                .iter()
                .any(|e| e.contains("prints columns")),
            "dropping column (d) from the tips worksheet must red"
        );

        // (2) A DROPPED ROW — the sheets print A through E, and four rows is a quietly smaller sheet.
        let mut one_row_short = good.clone();
        one_row_short[1].rows.pop();
        assert!(
            worksheet_violations(&one_row_short, &printed)
                .iter()
                .any(|e| e.contains("prints rows")),
            "dropping row E must red"
        );

        // (3) A WHOLE SHEET — the r1 defect, which collapsed the two overtime worksheets into one.
        let three: Vec<WorksheetShape<'_>> = good[..3].to_vec();
        assert!(
            !worksheet_violations(&three, &printed).is_empty(),
            "transcribing three of four worksheets must red"
        );

        // (4) A PARAPHRASED HEADER.
        let mut paraphrased = good.clone();
        paraphrased[0].columns[0].1 = "The employer's name";
        assert!(
            worksheet_violations(&paraphrased, &printed)
                .iter()
                .any(|e| e.contains("neither printed verbatim")),
            "a paraphrased column header must red"
        );

        // (5) ★★ AND THE READER ITSELF: no anchors must be an ERROR, never an empty list. This is the
        //     census-F-4 failure in miniature — a checker that passes by finding nothing.
        assert!(
            printed_worksheets("no worksheets here at all\n").is_err(),
            "a fixture with no anchors must FAIL, not silently check zero worksheets"
        );
    }

    // ───────────────────────── half 2 — per-line quotation ─────────────────────────

    /// Every `**Line <label>** — "<quote>"` marker in a source file.
    ///
    /// ★ Pure over the source text, which is what makes the plant below possible at all.
    fn doc_quotes(src: &str) -> Vec<(String, String)> {
        let mut doc = String::new();
        for l in src.lines() {
            if let Some(rest) = l.trim_start().strip_prefix("///") {
                doc.push(' ');
                doc.push_str(rest.trim());
            }
        }
        let mut out = Vec::new();
        let mut rest = doc.as_str();
        while let Some(i) = rest.find("**Line ") {
            rest = &rest[i + "**Line ".len()..];
            let Some(j) = rest.find("** — \"") else {
                continue;
            };
            // ★ A LABEL IS SHORT. Without this the scanner runs from a `**Line …**` that opens no
            //   quotation all the way to the next one that does, and reports a sentence as a label —
            //   a legible failure beats a confusing one, and the set comparison reds either way.
            if j > 12 {
                continue;
            }
            let label = rest[..j].trim().to_string();
            let after = &rest[j + "** — \"".len()..];
            let Some(k) = after.find('"') else {
                continue;
            };
            out.push((label, norm(&after[..k])));
            rest = &after[k..];
        }
        out
    }

    /// The doc-comment marker a leaf label is quoted under. Line 22's two rows share one Rust type, so
    /// their three columns are quoted once, against the heading whose span carries the column headers.
    fn marker_of(leaf_label: &str) -> String {
        match leaf_label.find('(') {
            Some(i) => {
                let stem: String = leaf_label[..i]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                format!("{stem} {}", &leaf_label[i..])
            }
            None => leaf_label.to_string(),
        }
    }

    /// Is every quoted instruction printed as **that line's own text**?
    ///
    /// ★★★ **THIS IS THE HALF THAT IS NOT `cite-check`.** A citation checker proves a quotation is the
    /// FORM's words; it does not prove they are THAT LINE's words. Line 28's *"increase the result to
    /// the next higher whole number"* sitting on line 11 survives citation checking — and that swap
    /// inverts the rounding for Parts II and III, which is the most dangerous single fact on this form.
    fn quotation_violations(src: &str, expected: &BTreeSet<String>) -> Vec<String> {
        let mut errs = Vec::new();
        let quotes = doc_quotes(src);
        let found: BTreeSet<String> = quotes.iter().map(|(l, _)| l.clone()).collect();
        for missing in expected.difference(&found) {
            errs.push(format!(
                "no doc comment quotes line {missing:?} — a field with no instruction text is a line \
                 nobody transcribed"
            ));
        }
        for extra in found.difference(expected) {
            errs.push(format!(
                "a doc comment quotes line {extra:?}, which is not a leaf of the struct"
            ));
        }
        for (label, quote) in &quotes {
            if !expected.contains(label) {
                continue;
            }
            let line = label.split(' ').next().unwrap_or(label);
            let Some(span) = printed_line(line) else {
                errs.push(format!("line {line:?} is not printed on the form at all"));
                continue;
            };
            if !norm(&span).contains(quote) {
                errs.push(format!(
                    "line {label}'s doc comment quotes text that is NOT printed as line {line}'s own \
                     text:\n      {quote:?}\n    printed: {:?}",
                    norm(&span)
                ));
            }
        }
        errs
    }

    /// The doc-comment markers the struct's own leaves require, derived from [`Schedule1A::leaves`].
    fn expected_markers() -> BTreeSet<String> {
        Schedule1A::default()
            .leaves()
            .iter()
            .map(|(l, _)| marker_of(l))
            .collect()
    }

    /// ★★★ **HALF 2 — every field's doc comment carries its own line's printed instruction.**
    #[test]
    fn every_field_doc_comment_quotes_its_own_printed_line() {
        let errs = quotation_violations(SCHEDULE_1A_SOURCE, &expected_markers());
        assert!(errs.is_empty(), "{errs:#?}");
        // 52 leaves collapse to 49 markers, because line 22's two rows share one Rust type.
        assert_eq!(Schedule1A::default().leaves().len(), 52);
        assert_eq!(expected_markers().len(), 49);
    }

    /// ★★★ **B1 for half 2 — the swap that inverts the rounding, planted and caught.**
    #[test]
    fn a_quotation_moved_onto_the_wrong_line_is_rejected() {
        let expected: BTreeSet<String> = ["11", "28"].iter().map(|s| s.to_string()).collect();
        let honest = "\
    /// **Line 11** — \"decrease the result to the next lower whole number\"
    pub line11_steps: Option<Usd>,
    /// **Line 28** — \"increase the result to the next higher whole number\"
    pub line28_steps: Option<Usd>,
";
        assert!(
            quotation_violations(honest, &expected).is_empty(),
            "the control must PASS"
        );

        // THE DEFECT: line 28's ceiling sentence moved onto line 11. Verbatim from the form, on the
        // wrong line — which is exactly what `cite-check` cannot see.
        let swapped = honest.replace(
            "**Line 11** — \"decrease the result to the next lower whole number\"",
            "**Line 11** — \"increase the result to the next higher whole number\"",
        );
        let errs = quotation_violations(&swapped, &expected);
        assert!(
            errs.iter().any(|e| e.contains("line 11")),
            "the rounding swap must red: {errs:#?}"
        );

        // …and a field whose quote is simply gone must red too, or the check only catches swaps.
        let dropped = honest.replace("**Line 28** — ", "Line 28: ");
        assert!(
            quotation_violations(&dropped, &expected)
                .iter()
                .any(|e| e.contains("no doc comment quotes line \"28\"")),
            "a field with no quoted instruction must red"
        );
    }

    // ───────────────────────── half 3 — provenance ─────────────────────────

    /// Does every leaf have a determinate PROVENANCE?
    ///
    /// ★★★ **NOT "is every line populated" — most of this schedule is blank on a correct return.** The
    /// invariant is that each leaf is accounted for: a money leaf carries a `Production` (or an
    /// `Exception` with a written reason) in the coverage table, and a non-money leaf is recorded as
    /// carrying none, **with a reason**. Two blanks look identical on the printed page and are not the
    /// same thing; a checker that cannot tell *"this line encodes no decision"* from *"we forgot this
    /// line"* is not a conformance check.
    fn provenance_violations(
        rows: &[LineCoverage],
        leaves: &[(&str, Leaf<'_>)],
        non_money: &[(&str, &str)],
    ) -> Vec<String> {
        let mut errs = Vec::new();
        let covered: BTreeSet<&str> = rows.iter().map(|r| r.line.as_str()).collect();
        let recorded: BTreeSet<&str> = non_money.iter().map(|(l, _)| *l).collect();
        for (label, leaf) in leaves {
            let money = matches!(leaf, Leaf::Money(_) | Leaf::Steps(_));
            match (money, covered.contains(label), recorded.contains(label)) {
                (true, true, false) => {}
                (false, false, true) => {}
                (true, false, _) => errs.push(format!(
                    "leaf {label:?} is money and has NO production — declared, doc-commented and \
                     never assigned is exactly the \"present but never populated\" case"
                )),
                (false, true, _) => errs.push(format!(
                    "leaf {label:?} is not money yet carries a money-census row"
                )),
                (false, false, false) => errs.push(format!(
                    "leaf {label:?} carries no production and no recorded REASON for carrying none"
                )),
                (true, true, true) => errs.push(format!(
                    "leaf {label:?} is recorded as non-money and also carries a money row"
                )),
            }
        }
        let leaf_labels: BTreeSet<&str> = leaves.iter().map(|(l, _)| *l).collect();
        for row in rows {
            if !leaf_labels.contains(row.line.as_str()) {
                errs.push(format!(
                    "the coverage table carries {:?}, which is not a leaf of the struct",
                    row.line
                ));
            }
        }
        for (label, reason) in non_money {
            if reason.trim().is_empty() {
                errs.push(format!("{label:?} is recorded as non-money with no reason"));
            }
            if !leaf_labels.contains(label) {
                errs.push(format!(
                    "{label:?} is recorded as non-money but is not a leaf"
                ));
            }
        }
        errs
    }

    /// ★★★ **HALF 3 — every leaf is accounted for, and every Exception has a written reason.**
    #[test]
    fn every_leaf_has_a_determinate_provenance() {
        let s = Schedule1A::default();
        let cov = cover_schedule1a(&s);
        let errs = provenance_violations(&cov.0, &s.leaves(), &NON_MONEY_LEAVES);
        assert!(errs.is_empty(), "{errs:#?}");

        // The rows are quoted from the 2025 extract, not 2024 — the `quoting_year` call is what makes
        // that true, and a row that lost it would send `cite-check` to a file that does not exist.
        assert!(
            cov.0
                .iter()
                .all(|r| r.year == "2025" && r.form == "f1040s1a"),
            "every Schedule 1-A row is quoted from f1040s1a--2025"
        );
        // Every Exception carries a reason. `xtask line-coverage` enforces this over the whole table;
        // asserted here too, because during B3 this KAT is the only guard that can see this form.
        for r in cov
            .0
            .iter()
            .filter(|r| r.production == Production::Exception)
        {
            assert!(
                r.reason.is_some_and(|x| x.len() > 40),
                "{}:{} is an Exception with no substantive reason",
                r.form,
                r.line
            );
        }
    }

    /// ★★★ **B1 for half 3 — a field declared and never given a production.**
    #[test]
    fn a_leaf_with_no_production_is_rejected() {
        let s = Schedule1A::default();
        let cov = cover_schedule1a(&s);
        let leaves = s.leaves();
        assert!(
            provenance_violations(&cov.0, &leaves, &NON_MONEY_LEAVES).is_empty(),
            "the control must PASS"
        );

        // THE DEFECT: line 13 — the qualified tips deduction itself — loses its row.
        let gutted: Vec<LineCoverage> = cov.0.iter().filter(|r| r.line != "13").cloned().collect();
        assert!(
            provenance_violations(&gutted, &leaves, &NON_MONEY_LEAVES)
                .iter()
                .any(|e| e.contains("\"13\"") && e.contains("NO production")),
            "a money leaf with no production must red"
        );

        // …and the mirror: a VIN quietly recorded as money.
        let vin_as_money: Vec<(&str, Leaf<'_>)> = leaves
            .iter()
            .map(|(l, leaf)| {
                if *l == "22a(i)" {
                    (*l, Leaf::Money(None))
                } else {
                    (*l, *leaf)
                }
            })
            .collect();
        assert!(
            !provenance_violations(&cov.0, &vin_as_money, &NON_MONEY_LEAVES).is_empty(),
            "a non-money leaf reclassified as money, with no row to back it, must red"
        );

        // …and a non-money leaf with no recorded reason — the case that distinguishes a blank that
        // encodes no decision from a blank nobody looked at.
        assert!(
            !provenance_violations(&cov.0, &leaves, &[]).is_empty(),
            "an unrecorded non-money leaf must red"
        );
    }

    // ───────────────────────── half 4 — completion ─────────────────────────

    /// Is each line completed exactly when the form (or, for Parts I and V, the instructions) says?
    fn completion_violations(
        predicate: impl Fn(&str) -> Option<bool>,
        expected: &[(&str, bool)],
    ) -> Vec<String> {
        let mut errs = Vec::new();
        for (label, want) in expected {
            match predicate(label) {
                Some(got) if got == *want => {}
                Some(got) => errs.push(format!("line {label}: completed = {got}, expected {want}")),
                None => errs.push(format!("line {label} has no completion rule at all")),
            }
        }
        errs
    }

    /// ★★★ **HALF 4 — completion is a PER-LINE decision, and an unmet condition leaves a line NOT
    /// COMPLETED rather than zero.**
    #[test]
    fn completion_is_per_line_and_an_unmet_condition_leaves_the_line_blank() {
        let lines: Vec<String> = Schedule1A::default()
            .leaves()
            .iter()
            .map(|(l, _)| schedule_1a::line_label_of(l).to_string())
            .collect();
        let all_labels: BTreeSet<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(all_labels.len(), 48, "48 entry lines: {all_labels:?}");

        // (1) The COMMON FILER — no tips, no overtime, no car loan, not a senior, no excluded income.
        //     Three lines are completed and forty-five are blank, and that is the CORRECT return.
        let plain = Schedule1aCompletion::default();
        let completed: BTreeSet<&str> = all_labels
            .iter()
            .filter(|l| schedule_1a::is_completed(l, &plain) == Some(true))
            .copied()
            .collect();
        assert_eq!(
            completed,
            ["1", "3", "38"].into_iter().collect::<BTreeSet<&str>>(),
            "lines 1 and 3 are ALWAYS entered — a part-scoped Part I predicate would blank line 3, \
             the MAGI that lines 8, 16, 25 and 31 each read — and line 38 is the total"
        );

        // (2) THE C-I2 CASE: a filer with tips who is NOT a senior. Part V must not be completed, so
        //     line 35 cannot print $6,000 for a non-senior.
        let tipped_non_senior = Schedule1aCompletion {
            part2_received_qualified_tips: true,
            ..Schedule1aCompletion::default()
        };
        let errs = completion_violations(
            |l| schedule_1a::is_completed(l, &tipped_non_senior),
            &[
                ("3", true),
                ("2a", false),
                ("4a", true),
                ("13", true),
                ("21", false),
                ("30", false),
                ("35", false),
                ("37", false),
                ("38", true),
            ],
        );
        assert!(errs.is_empty(), "{errs:#?}");

        // (3) A SENIOR completes Part V and nothing else.
        let senior = Schedule1aCompletion {
            part5_born_before_january_2_1961: true,
            ..Schedule1aCompletion::default()
        };
        let errs = completion_violations(
            |l| schedule_1a::is_completed(l, &senior),
            &[("31", true), ("35", true), ("37", true), ("13", false)],
        );
        assert!(errs.is_empty(), "{errs:#?}");

        // (4) A label that is not on the form has no rule — the check is closed at that end too.
        assert!(schedule_1a::is_completed("39", &plain).is_none());
    }

    /// ★★★ **THE SOURCE IS NAMED PER PART, AND PART V's IS NOT ITS CAUTION.** This is the r5 I-1
    /// finding, pinned mechanically: the form's Part V Caution is an ELIGIBILITY bar with no birth date
    /// anywhere in it, and the completion condition is instructions-only. A transcription that read the
    /// Caution alone would let a non-senior complete Part V, and line 35 would print $6,000 — with the
    /// KAT green and blind to it.
    #[test]
    fn part_vs_caution_is_an_eligibility_bar_and_the_birth_date_is_instructions_only() {
        let caution = part_caution("V").expect("Part V prints a Caution");
        assert!(
            caution.contains("valid social security number")
                && caution.contains("you must file jointly"),
            "Part V's Caution drifted: {caution:?}"
        );
        assert!(
            !caution.contains("born before"),
            "★ Part V's Caution must NOT carry the birth date — if the form ever adds it, the \
             instructions-only reading must be revisited rather than silently kept: {caution:?}"
        );
        let ins = norm(SCHEDULE_1A_INSTRUCTIONS_TEXT);
        assert!(ins.contains("Fill out Schedule 1-A, Part V, only if:"));
        assert!(ins.contains("were born before January 2, 1961."));
        // …and Part I prints no Caution at all, which is why its source is the instructions too.
        assert!(
            part_caution("I").is_none(),
            "Part I prints no Caution; inventing a part-level predicate for it blanks line 3"
        );
        // ★ The recorded source for EVERY part is checked against the form, not just Part V's —
        //   a table keyed to the one case that was got wrong reds on nothing when a second appears.
        for (part, source, why) in schedule_1a::COMPLETION_SOURCES {
            assert!(!why.trim().is_empty(), "part {part} records no source text");
            let printed = part_caution(part);
            match source {
                schedule_1a::CompletionSource::FormCaution => assert!(
                    printed.as_deref().is_some_and(|c| c.contains("only if")),
                    "part {part} names the form's Caution as its completion source, but the form \
                     prints no Caution stating the condition: {printed:?}"
                ),
                schedule_1a::CompletionSource::InstructionsOnly => assert!(
                    printed.as_deref().is_none_or(|c| !c.contains("only if")),
                    "part {part} is recorded as instructions-only, yet its printed Caution DOES \
                     state the condition — re-read it before keeping this classification: {printed:?}"
                ),
                schedule_1a::CompletionSource::Unconditional => assert!(
                    printed.is_none(),
                    "part {part} is recorded as unconditional but prints a Caution: {printed:?}"
                ),
            }
        }
    }

    /// ★★★ **B1 for half 4 — the r4 reading, planted: complete Part V on the Caution alone.**
    #[test]
    fn completing_a_part_whose_predicate_is_false_is_rejected() {
        let non_senior = Schedule1aCompletion::default();
        let expect = [("35", false), ("37", false)];
        assert!(
            completion_violations(|l| schedule_1a::is_completed(l, &non_senior), &expect)
                .is_empty(),
            "the control must PASS"
        );

        // THE DEFECT: Part V's predicate reduced to its Caution (a valid SSN, filing jointly), which a
        // non-senior satisfies. Lines 31-35 are computed and line 35 prints $6,000 for a non-senior.
        let caution_only = |l: &str| match l {
            "31" | "32" | "33" | "34" | "35" | "36a" | "36b" | "37" => Some(true),
            other => schedule_1a::is_completed(other, &non_senior),
        };
        let errs = completion_violations(caution_only, &expect);
        assert!(
            errs.iter().any(|e| e.contains("line 35")),
            "a non-senior reaching line 35 must red: {errs:#?}"
        );

        // …and a line with no rule at all must red rather than defaulting either way.
        assert!(!completion_violations(|_| None, &expect).is_empty());
    }

    /// The Caution the form prints under a part heading, if it prints one.
    ///
    /// ★ The part is matched as a WHOLE TOKEN. `starts_with("Part I")` also matches `Part II`,
    /// `Part III` and `Part IV`, which would have handed Part I someone else's Caution — and Part I's
    /// whole point here is that it has none.
    fn part_caution(roman: &str) -> Option<String> {
        let mut lines = SCHEDULE_1A_FORM_TEXT.lines();
        lines.by_ref().find(|l| {
            let mut t = l.split_whitespace();
            t.next() == Some("Part") && t.next() == Some(roman)
        })?;
        let mut out = String::new();
        for l in lines {
            let t = l.trim();
            if t.is_empty() {
                break;
            }
            let indent = l.len() - l.trim_start().len();
            let tok = t.split_whitespace().next().unwrap_or("");
            if indent <= MARGIN_COLUMN && (is_numeric_label(tok) || is_bare_letter(tok)) {
                break;
            }
            if out.is_empty() && !t.starts_with("Caution:") {
                continue;
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(t);
        }
        (!out.is_empty()).then_some(norm(&out))
    }
}
