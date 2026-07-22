//! **btctax-forms** — fill the OFFICIAL IRS fillable PDFs (Form 8949 + Schedule D) from btctax's
//! already-computed tax data. Offline, deterministic, and **geometry-verified**: every fill is read
//! back from its own serialized bytes and each value's widget `/Rect` is checked against a
//! map-independent column/row band re-derived from the PDF itself. A mis-mapped cell fails closed.
//!
//! ## Sub-project 1 (TY2025)
//! - **Form 8949** — Bitcoin under **Box I** (short-term) / **Box L** (long-term), the 1099-DA
//!   revision (NOT Box C/F, which read "other than digital asset transactions"). 11 rows per part
//!   per page.
//! - **Schedule D** — lines 3 & 7 (ST), 10 & 15 (LT), 16 (total), QOF = No. Lines 17-22 are scoped
//!   out (the caller prints a notice).
//! - These are **static XFA-hybrid** PDFs: the fill removes the `/AcroForm` `/XFA` layer (else
//!   Acrobat shows blank), sets `/NeedAppearances`, and pins determinism (drops `/Info` dates + the
//!   trailer `/ID`).
//!
//! The tax data is REUSED verbatim from the projection (`btctax_core::form_8949` /
//! `btctax_core::schedule_d`) — this crate never recomputes gains.

mod cells;
mod error;
mod fill8949;
mod fill8949_full;
mod form1040;
mod form1040_full;
mod form8275;
mod form8283;
mod form8959;
mod form8960;
mod form8995;
mod map;
mod overflow;
mod packet;
mod pdf;
mod schedule23;
mod schedule_a;
mod schedule_b;
mod schedule_c;
mod schedule_d;
mod schedule_d_full;
mod schedule_se;
mod schedule_se_full;
mod transcribe;
mod verify;
mod watermark;

pub use error::FormsError;
pub use form1040::{Form1040Fill, Form1040Inputs};
pub use map::{
    Form1040Map, Form8275Map, Form8283Map, Form8949Map, Form8959Map, Form8960Map, Form8995Map,
    Schedule1Map, Schedule2Map, Schedule3Map, ScheduleAMap, ScheduleBMap, ScheduleCMap,
    ScheduleDMap, ScheduleSeMap,
};
pub use schedule_se::SE_FLOOR;

use btctax_core::conventions::{TaxDate, Usd};
use btctax_core::{Form8949Row, ScheduleDTotals};
use time::macros::format_description;

/// The tax years this build bundles forms + maps for. Each fill dispatches to the year's committed
/// map + bundled PDF via the `Map::for_year` constructors; an unlisted year fails closed with
/// [`FormsError::UnsupportedYear`].
pub const SUPPORTED_YEARS: &[i32] = &[2017, 2024, 2025];

/// Format a date as **MM/DD/YYYY** — Form 8949's native date format for columns (b)/(c).
pub(crate) fn fmt_date(d: TaxDate) -> Result<String, FormsError> {
    let fmt = format_description!("[month]/[day]/[year]");
    d.format(&fmt)
        .map_err(|e| FormsError::Structure(format!("date format: {e}")))
}

/// Format money exactly as the `form8949.csv` / `schedule_d.csv` do — the raw `Decimal` Display
/// (native scale, no `$`, no thousands separators). Keeping this identical to the CSV is what lets
/// `schedule_d_totals_match_form8949_and_csv` cross-check the three artifacts.
pub(crate) fn fmt_money(d: Usd) -> String {
    d.to_string()
}

/// Fill **Form 8949** (Part I + Part II) for `year` from the projection rows and return the PDF
/// bytes. The box is year-aware: Bitcoin is filed under the digital-asset Box I/L from TY2025, and
/// the securities Box C/F on the pre-2025 revisions. Parts that overflow the revision's grid (11 rows
/// per part on the 2025 form, 14 on 2024/2017) paginate: ⌈rows/grid⌉ page copies per part, each with
/// its own totals; the copies are merged with per-copy field renaming so no two share a value. Every
/// copy is geometry-verified before merge.
pub fn fill_form_8949(rows: &[Form8949Row], year: i32) -> Result<Vec<u8>, FormsError> {
    let map = Form8949Map::for_year(year)?;
    let cap = map.rows_per_page;
    let (st, lt) = fill8949::split_parts(rows);
    let n_pages = div_ceil(st.len(), cap).max(div_ceil(lt.len(), cap)).max(1);

    if n_pages == 1 {
        let short = fill8949::part_data(&st)?;
        let long = fill8949::part_data(&lt)?;
        return fill8949::fill_8949_parts(&short, &long, &map);
    }

    let mut copies = Vec::with_capacity(n_pages);
    for k in 0..n_pages {
        let st_chunk: Vec<&Form8949Row> = st.iter().skip(k * cap).take(cap).copied().collect();
        let lt_chunk: Vec<&Form8949Row> = lt.iter().skip(k * cap).take(cap).copied().collect();
        let short = fill8949::part_data(&st_chunk)?;
        let long = fill8949::part_data(&lt_chunk)?;
        // Each copy is filled on ORIGINAL names and geometry-verified here (fails closed).
        copies.push(fill8949::fill_8949_parts(&short, &long, &map)?);
    }
    overflow::merge_copies(&copies)
}

fn div_ceil(n: usize, d: usize) -> usize {
    n.div_ceil(d)
}

/// Stamp a diagonal `DRAFT — ESTIMATE, NOT FOR FILING` watermark on every page of a filled form.
/// Applied by the CLI when the ledger is pseudo-reconciled (an estimate). The overlay carries its own
/// embedded standard font resource, orthogonal to `/NeedAppearances`.
pub fn stamp_draft_watermark(pdf_bytes: &[u8]) -> Result<Vec<u8>, FormsError> {
    watermark::stamp_draft(pdf_bytes)
}

/// Fill **Schedule D** for `year` from the part totals and return the PDF bytes.
pub fn fill_schedule_d(totals: &ScheduleDTotals, year: i32) -> Result<Vec<u8>, FormsError> {
    let map = ScheduleDMap::for_year(year)?;
    schedule_d::fill_schedule_d_totals(totals, &map)
}

/// Fill **Schedule SE** (Form 1040) for `year` from the computed §1401 `SeTaxResult`, the filer's
/// Form W-2 Social Security wages (line 8a), and the year's Social Security wage base (line 7).
/// Returns `Ok(None)` when net SE earnings are **below the $400 floor** (no SE tax owed — the form is
/// not written). Line 12 = SS + regular Medicare only (the 0.9% Additional Medicare Tax is a Form 8959
/// item, not on Schedule SE); when `se.addl > 0` the caller prints a Form 8959 advisory.
pub fn fill_schedule_se(
    se: &btctax_core::SeTaxResult,
    w2_ss_wages: Usd,
    ss_wage_base: Usd,
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    let map = ScheduleSeMap::for_year(year)?;
    schedule_se::fill_schedule_se_with_map(se, w2_ss_wages, ss_wage_base, &map)
}

/// Fill **Form 8959** (Additional Medicare Tax) for `year` from the core-derived line chain
/// (`btctax_core::tax::other_taxes::form_8959_lines`). Returns `Ok(None)` when the form is not
/// required — line 18 (the tax) AND line 24 (the withholding reconciliation) are both zero.
///
/// Line 24 is not redundant with line 18: an employer withholds the 0.9% on ITS OWN wages over
/// $200,000 with no knowledge of a spouse or a second job, so a taxpayer who owes NO Additional
/// Medicare Tax can still have had some withheld — and that excess is a credit on 1040 line 25c.
/// Skipping the form on line 18 alone would silently forfeit it.
pub fn fill_form_8959(
    lines: &btctax_core::tax::other_taxes::Form8959Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    let map = Form8959Map::for_year(year)?;
    form8959::fill_form_8959_with_map(lines, header, &map)
}

/// Fill **Form 8960** (Net Investment Income Tax, §1411) for `year` from the core-derived line chain
/// (`btctax_core::tax::other_taxes::form_8960_lines`, which returns `None` when no NIIT is owed —
/// there is then no form to file).
pub fn fill_form_8960(
    lines: &btctax_core::tax::other_taxes::Form8960Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Form8960Map::for_year(year)?;
    form8960::fill_form_8960_with_map(lines, header, &map)
}

/// Fill **Form 8995** (QBI deduction, simplified) for `year` from the core-derived line chain
/// (`btctax_core::tax::qbi::form_8995_lines`, which returns `None` when there is no QBI).
///
/// FAILS CLOSED if a parenthesized cell (line 7/16/17) carries a negative value: the form pre-prints
/// the parentheses, so a negative would render as a POSITIVE number on the filed return.
pub fn fill_form_8995(
    lines: &btctax_core::tax::qbi::Form8995Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Form8995Map::for_year(year)?;
    form8995::fill_form_8995_with_map(lines, header, &map)
}

/// Fill **Schedule 1** (Additional Income and Adjustments to Income) for `year` from the core-derived
/// printed chain (`btctax_core::tax::printed::schedule_1_lines`, which returns `None` when there is
/// neither additional income nor an adjustment — the schedule is then not filed).
pub fn fill_schedule_1(
    lines: &btctax_core::tax::printed::Schedule1Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Schedule1Map::for_year(year)?;
    schedule23::fill_schedule_1_with_map(lines, header, &map)
}

/// Fill **Schedule 2** (Additional Taxes) for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::schedule_2_lines`, which returns `None` when there are no other
/// taxes to report — the schedule is then not filed).
pub fn fill_schedule_2(
    lines: &btctax_core::tax::printed::Schedule2Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Schedule2Map::for_year(year)?;
    schedule23::fill_schedule_2_with_map(lines, header, &map)
}

/// Fill **Schedule 3** (Additional Credits and Payments) for `year` from the core-derived printed
/// chain (`btctax_core::tax::printed::schedule_3_lines`, which returns `None` when there is neither a
/// foreign tax credit nor an excess-Social-Security credit).
pub fn fill_schedule_3(
    lines: &btctax_core::tax::printed::Schedule3Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Schedule3Map::for_year(year)?;
    schedule23::fill_schedule_3_with_map(lines, header, &map)
}

/// Fill **Schedule A** (Itemized Deductions) for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::schedule_a_lines`, which returns `None` unless the return actually
/// itemizes — Schedule A is computed even when the standard deduction wins, but only FILED when it is
/// the deduction claimed).
pub fn fill_schedule_a(
    lines: &btctax_core::tax::printed::ScheduleALines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = ScheduleAMap::for_year(year)?;
    schedule_a::fill_schedule_a_with_map(lines, header, &map)
}

/// Fill the **FULL-RETURN Form 1040** for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::form_1040_lines`) — every line, not just the capital-gain cluster.
///
/// This is NOT [`fill_form_1040_capgains`], which writes only line 7 + the Digital-Asset question for
/// the crypto-slice export. That one stays: for a year with no `ReturnInputs` it is what the filer
/// wants.
pub fn fill_form_1040_full(
    lines: &btctax_core::tax::printed::Form1040Lines,
    header: &btctax_core::tax::packet::ReturnHeader,
    status: btctax_core::tax::types::FilingStatus,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Form1040Map::for_year(year)?;
    form1040_full::fill_form_1040_full_with_map(lines, header, status, &map)
}

/// Fill the **FULL-RETURN Schedule D** for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::schedule_d_lines`), including Part III's SPEC §7.2 routing.
///
/// This is NOT [`fill_schedule_d`], which is the **crypto-slice** fill: that one writes only lines
/// 3/7/10/15/16 from the ledger totals, with no line 13 (1099-DIV box-2a capital-gain distributions)
/// and no lines 6/14 (capital-loss carryovers). For a crypto-only year that is complete and correct.
/// For a full return it would be a complete-LOOKING form with income missing.
///
/// FAILS CLOSED if a parenthesized cell (line 6/14/21) carries a negative: the form pre-prints the
/// parentheses, so a negative renders as a POSITIVE number — turning a capital loss into a gain.
pub fn fill_schedule_d_full(
    lines: &btctax_core::tax::printed::ScheduleDLines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = ScheduleDMap::for_year(year)?;
    schedule_d_full::fill_schedule_d_full_with_map(lines, header, &map)
}

/// Fill **Form 8275** (Disclosure Statement) for `year` from the T13 printed disclosure
/// (`btctax_core::tax::printed::printed_8275`) + the FILER's identity. `Ok(None)` when there is no
/// Part I content to disclose.
///
/// Year coverage is MANDATORY, not conditional: Form 8275 is REVISION-versioned, not
/// tax-year-versioned, so the single bundled Rev. 10-2024 asset is aliased to EVERY `SUPPORTED_YEAR`
/// (2017/2024/2025) — a promoted disposal filed in any supported year gets a real fillable
/// disclosure, never a permanent refusal for want of a "2025 revision" that does not exist.
pub fn fill_form_8275(
    printed: &btctax_core::tax::printed::Printed8275,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    form8275::fill_form_8275(printed, header, year)
}

/// Fill the **full-return Form 8283** for `year` — whole-dollar rows plus the FILER's identity block
/// (which the crypto slice never writes). `Ok(None)` when there are no donation rows.
pub fn fill_form_8283_full(
    printed: &btctax_core::tax::printed::Printed8283Rows,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    let map = Form8283Map::for_year(year)?;
    form8283::fill_form_8283_full(printed, header, &map)
}

/// Fill the **full-return Form 8949** for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::form_8949_printed`). Whole dollars, and column (h) is DERIVED from the
/// printed (d) − (e). Schedule D lines 3/10 ARE this form's printed column totals.
pub fn fill_8949_full(
    printed: &btctax_core::tax::printed::Printed8949,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Form8949Map::for_year(year)?;
    fill8949_full::fill_8949_full_with_map(printed, header, &map)
}

/// Fill the **full-return Schedule SE** for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::schedule_se_lines`). Whole dollars — the crypto slice's
/// `fill_schedule_se` keeps its exact-cents rendering and is untouched.
pub fn fill_schedule_se_full(
    lines: &btctax_core::tax::printed::ScheduleSeLines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = ScheduleSeMap::for_year(year)?;
    schedule_se_full::fill_schedule_se_full_with_map(lines, header, &map)
}

/// Fill **Schedule B** (Interest and Ordinary Dividends) for `year` from the core-derived printed
/// chain (`btctax_core::tax::printed::schedule_b_lines`, which returns `None` when Schedule B is not
/// required — interest and dividends both at or under $1,500 and no declared foreign account).
///
/// REFUSES when there are more payers than the form has rows (14 interest / 15 dividend). Truncating
/// the list would leave a form whose printed rows do not add up to its own total.
pub fn fill_schedule_b(
    lines: &btctax_core::tax::printed::ScheduleBLines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = ScheduleBMap::for_year(year)?;
    schedule_b::fill_schedule_b_with_map(lines, header, &map)
}

/// Fill **Schedule C** (Profit or Loss From Business) for `year` from the core-derived printed chain
/// (`btctax_core::tax::printed::schedule_c_lines`, which returns `None` when there is no crypto trade
/// or business).
pub fn fill_schedule_c(
    lines: &btctax_core::tax::printed::ScheduleCLines,
    header: &btctax_core::tax::packet::ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = ScheduleCMap::for_year(year)?;
    schedule_c::fill_schedule_c_with_map(lines, header, &map)
}

/// Fill **Form 8283** (Noncash Charitable Contributions, Rev. 12-2025) for `year` from the projected
/// donation rows + `DonationDetails`. Returns `Ok(None)` when there are no donations in the year.
/// Fills the donee/appraiser IDENTITY + per-row property data (and, for Section B, checks the "k
/// Digital assets" property-type box); leaves BLANK every OTHER party's declaration/signature (a
/// LOUD partial-scope notice is the caller's). More rows than a section holds (4 in Section A / 3 in
/// Section B) overflow onto additional form copies via [`overflow::merge_copies`].
pub fn fill_form_8283(
    rows: &[btctax_core::Form8283Row],
    year: i32,
) -> Result<Option<Vec<u8>>, FormsError> {
    let map = Form8283Map::for_year(year)?;
    form8283::fill_form_8283(rows, &map)
}

/// Fill the capital-gains cells of **Form 1040** for `year`: line 7a (only when Schedule D is ACTIVE
/// and line 16 ≥ 0; active-and-zero → "-0-") and the Digital-Asset Yes/No question (YES iff there is
/// btctax-evidenced qualifying activity). Returns `Ok(None)` — **skip the whole 1040** — when there is
/// no reportable activity (the DA answer would be blank and there is no 7a value). 7b checkboxes are
/// left untouched; a NET LOSS leaves 7a blank (the §1211 line-21 cap is the filer's). A partial-scope
/// notice enumerating exactly what was filled is the caller's.
pub fn fill_form_1040_capgains(
    inputs: &Form1040Inputs,
    year: i32,
) -> Result<Option<Form1040Fill>, FormsError> {
    let map = Form1040Map::for_year(year)?;
    form1040::fill_form_1040_capgains(inputs, &map)
}

/// **[I5]** How many rows might belong on a SEPARATE broker-reported Form 8949 — i.e. disposals on an
/// exchange that may have issued broker basis reporting. The separate boxes and this export's filed
/// boxes are year-aware (see `btctax-cli`'s `broker_reporting_advisory`): a 1099-B / Box A/B/D/E, filed
/// under C/F pre-TY2025; a 1099-DA / Box G/H/J/K, filed under I/L from TY2025. A non-zero count is a
/// loud advisory, not a refusal.
pub fn rows_possibly_broker_reported(rows: &[Form8949Row]) -> usize {
    rows.iter().filter(|r| r.box_needs_review).count()
}

// ── Internals exposed for the KATs (fault injection needs a corruptible map + the verifier). ──────
#[doc(hidden)]
pub use packet::{fill_full_return, NamedForm};

pub mod testonly {
    pub use crate::cells::fmt_money_pair;
    pub use crate::fill8949::{fill_8949_parts, part_data, pdf_has_xfa, split_parts, PartData};
    pub use crate::fill8949_full::fill_8949_full_with_map;
    pub use crate::form1040::{fill_form_1040_capgains as fill_1040_with_map, Form1040Fill};
    pub use crate::form1040_full::fill_form_1040_full_with_map;
    pub use crate::form8275::fill_form_8275_with_map as fill_8275_with_map;
    pub use crate::form8283::fill_form_8283 as fill_8283_with_map;
    pub use crate::form8959::fill_form_8959_with_map;
    pub use crate::form8960::fill_form_8960_with_map;
    pub use crate::form8995::fill_form_8995_with_map;
    // The committed map TOML, for the line-keyed inverse transcriber (`extract_lines`). Downstream
    // read-back tests need the map itself, not just its parsed struct.
    pub use crate::map::{
        AmountCols, Form1040Map, Form8275Map, Form8275Row, Form8283Map, Form8949Map, Form8959Map,
        Form8960Map, Form8995Map, MoneyCell, MoneyPair, PartMap, Schedule1Map, Schedule2Map,
        Schedule3Map, ScheduleAMap, ScheduleBMap, ScheduleCMap, ScheduleDMap, ScheduleSeMap,
    };
    pub use crate::map::{
        F1040_MAP_2024, F8275_MAP_2024, F8283_MAP_2024, F8949_MAP_2024, F8959_MAP_2024,
        F8960_MAP_2024, F8995_MAP_2024, SCHEDULE_1_MAP_2024, SCHEDULE_2_MAP_2024,
        SCHEDULE_3_MAP_2024, SCHEDULE_A_MAP_2024, SCHEDULE_B_MAP_2024, SCHEDULE_C_MAP_2024,
        SCHEDULE_D_MAP_2024, SCHEDULE_SE_MAP_2024,
    };
    pub use crate::pdf::{
        button_on_states, checkbox_on, collect_fields, index, load, text_value, Field,
        F1040_PDF_2017, F1040_PDF_2024, F1040_PDF_2025, F8275_PDF_2024, F8283_PDF_2017,
        F8283_PDF_2024, F8283_PDF_2025, F8949_PDF_2017, F8949_PDF_2024, F8949_PDF_2025,
        F8959_PDF_2024, SCHEDULE_D_PDF_2017, SCHEDULE_D_PDF_2024, SCHEDULE_D_PDF_2025,
        SCHEDULE_SE_PDF_2017, SCHEDULE_SE_PDF_2024, SCHEDULE_SE_PDF_2025,
    };
    pub use crate::schedule23::{
        fill_schedule_1_with_map, fill_schedule_2_with_map, fill_schedule_3_with_map,
    };
    pub use crate::schedule_a::fill_schedule_a_with_map;
    pub use crate::schedule_b::fill_schedule_b_with_map;
    pub use crate::schedule_c::fill_schedule_c_with_map;
    pub use crate::schedule_d::fill_schedule_d_totals;
    pub use crate::schedule_d_full::fill_schedule_d_full_with_map;
    pub use crate::schedule_se::fill_schedule_se_with_map;
    pub use crate::schedule_se_full::fill_schedule_se_full_with_map;
    pub use crate::transcribe::extract_lines;
    pub use crate::verify::{
        no_unmapped_filled, topmost_yes_no_pair, verify_8949, verify_flat, FlatPlacement, Geo,
        Placement,
    };
}
