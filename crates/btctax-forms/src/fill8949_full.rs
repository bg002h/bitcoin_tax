//! **Full-return** Form 8949 fill — the WHOLE-DOLLAR path (P6.3a / ARCH-P6.3a D2/D6).
//!
//! Form 8949 is not optional on a full return: **Schedule D lines 3 and 10 are literally "Totals for
//! all transactions reported on Form(s) 8949 with Box C / Box F checked"** (pre-2025; the 2025
//! digital-asset revision reads "with Box C or Box I checked" / "Box F or Box L checked"). A Schedule
//! D with those lines filled and no 8949 behind it is an incomplete return.
//!
//! **This module does no tax arithmetic.** The rows come from core's [`Printed8949`] chain, where
//! columns (d) and (e) are rounded at the cell and column **(h) is DERIVED, `h = d − e`** — never
//! rounded independently from the exact gain. That derivation is what makes each row satisfy the form's
//! own column-(h) instruction ("Subtract column (e) from column (d)…") and what makes Σh ≡ Σd − Σe an
//! integer identity, so Schedule D's Part I cross-foots against these very totals.
//!
//! What it DOES do — and the one thing sanctioned despite the "zero arithmetic in forms" rule — is
//! **partition** the rows into pages and sum each page's already-whole-dollar cells for that page's
//! line-2 totals. No rounding remains at that point, so partitioned integer sums cannot re-diverge
//! (Σ page-totals ≡ core's grand total, by associativity), and page capacity is legitimately the map's
//! datum, not core's. The KATs pin it anyway.
//!
//! The crypto-slice filler ([`crate::fill8949`]) keeps its exact-CENTS rendering, untouched: it is
//! CSV-identical shipped behavior, and a crypto-only filer may legitimately file in cents.

use crate::error::FormsError;
use crate::fill8949::{fill_8949_parts_with_identity, PartData};
use crate::map::Form8949Map;
use btctax_core::conventions::TaxDate;
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::{Printed8949, Printed8949Row};
use btctax_core::Usd;

/// Format a date the way the form does (MM/DD/YYYY).
fn fmt_date(d: TaxDate) -> String {
    format!("{:02}/{:02}/{}", d.month() as u8, d.day(), d.year())
}

/// Build one part's `PartData` from the PRINTED rows.
///
/// `PartData` is pre-formatted STRINGS — rounding-agnostic — which is exactly why the geometry half of
/// the slice's filler can be reused without importing its cents arithmetic. (The slice's `part_data`,
/// which sums exact `Decimal`s inside the forms crate, is NOT reused: it is slice-only.)
fn printed_part_data(rows: &[Printed8949Row]) -> PartData {
    let (mut sp, mut sc, mut sh) = (Usd::ZERO, Usd::ZERO, Usd::ZERO);
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        sp += r.proceeds_d;
        sc += r.cost_e;
        sh += r.gain_h;
        out.push([
            r.description.clone(),
            fmt_date(r.date_acquired),
            fmt_date(r.date_sold),
            r.proceeds_d.to_string(),
            r.cost_e.to_string(),
            String::new(), // (f) adjustment code — none in v1
            String::new(), // (g) adjustment amount — none in v1
            r.gain_h.to_string(),
        ]);
    }
    PartData {
        rows: out,
        totals: [
            sp.to_string(),
            sc.to_string(),
            String::new(), // (g) total — blank, no adjustments
            sh.to_string(),
        ],
        adj_nonzero: false,
    }
}

/// Fill the full-return Form 8949 (whole dollars) from the core-derived printed chain.
///
/// Both parts are emitted on one form; more rows than a page holds REFUSES
/// ([`FormsError::Overflow`]) exactly as the slice does — the continuation-page pattern is a
/// post-v1 item.
pub fn fill_8949_full_with_map(
    printed: &Printed8949,
    header: &ReturnHeader,
    map: &Form8949Map,
) -> Result<Vec<u8>, FormsError> {
    let short = printed_part_data(&printed.short_term);
    let long = printed_part_data(&printed.long_term);
    fill_8949_parts_with_identity(&short, &long, map, header)
}
