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

mod error;
mod fill8949;
mod map;
mod pdf;
mod schedule_d;
mod verify;

pub use error::FormsError;
pub use map::{Form8949Map, ScheduleDMap};

use btctax_core::conventions::{TaxDate, Usd};
use btctax_core::{Form8949Row, ScheduleDTotals};
use time::macros::format_description;

/// The only tax year SP1 bundles forms + maps for.
pub const SUPPORTED_YEAR: i32 = 2025;

fn require_year(year: i32) -> Result<(), FormsError> {
    if year == SUPPORTED_YEAR {
        Ok(())
    } else {
        Err(FormsError::UnsupportedYear(year))
    }
}

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
/// bytes. Bitcoin is filed under Box I/L. **> 11 rows per part are not yet paginated** (SP1-T1):
/// this returns [`FormsError::Overflow`]; pagination lands in T2.
pub fn fill_form_8949(rows: &[Form8949Row], year: i32) -> Result<Vec<u8>, FormsError> {
    require_year(year)?;
    let map = Form8949Map::ty2025();
    let (st, lt) = fill8949::split_parts(rows);
    let short = fill8949::part_data(&st)?;
    let long = fill8949::part_data(&lt)?;
    fill8949::fill_8949_parts(&short, &long, &map)
}

/// Fill **Schedule D** for `year` from the part totals and return the PDF bytes.
pub fn fill_schedule_d(totals: &ScheduleDTotals, year: i32) -> Result<Vec<u8>, FormsError> {
    require_year(year)?;
    let map = ScheduleDMap::ty2025();
    schedule_d::fill_schedule_d_totals(totals, &map)
}

/// **[I5]** How many rows might belong on a SEPARATE 1099-DA-reported Form 8949 (Box G/H/J/K) — i.e.
/// disposals on an exchange that may have issued broker basis reporting. SP1 files EVERY Bitcoin row
/// under Box I/L and says so; a non-zero count is a loud advisory, not a refusal.
pub fn rows_possibly_broker_reported(rows: &[Form8949Row]) -> usize {
    rows.iter().filter(|r| r.box_needs_review).count()
}

// ── Internals exposed for the KATs (fault injection needs a corruptible map + the verifier). ──────
#[doc(hidden)]
pub mod testonly {
    pub use crate::fill8949::{fill_8949_parts, part_data, pdf_has_xfa, split_parts, PartData};
    pub use crate::map::{AmountCols, Form8949Map, PartMap, ScheduleDMap};
    pub use crate::pdf::{
        checkbox_on, collect_fields, index, load, text_value, Field, F8949_PDF_2025,
        SCHEDULE_D_PDF_2025,
    };
    pub use crate::schedule_d::fill_schedule_d_totals;
    pub use crate::verify::{no_unmapped_filled, verify_8949, Geo, Placement};
}
