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
/// **PAGINATES, exactly as the crypto slice does** ([`crate::fill_form_8949`]): more rows than the
/// revision's grid holds (`map.rows_per_page` — 14 on 2024/2017, 11 on the 2025 digital-asset
/// revision) are chunked into ⌈rows/grid⌉ page copies per part, each filled and geometry-verified on
/// ORIGINAL field names, then merged with per-copy field renaming ([`crate::overflow::merge_copies`])
/// so no two copies share a `/V`. Each copy carries the FILER's identity on **both** of its pages —
/// every 8949 page is a filed page and the header is per-page (P6 r1 I3).
///
/// ★ **Per-copy totals; the grand total is Schedule D's, not this function's.** The form's line 2
/// says "Enter each total here", so each copy totals only its own rows, and Σ per-copy totals ≡
/// core's `st_totals`/`lt_totals` by associativity (the cells are already whole dollars, so no
/// rounding survives to re-diverge). Schedule D lines 3 and 10 keep reading core's totals over ALL
/// rows — the schedule's own text is "Totals for all transactions reported on **Form(s) 8949**",
/// plural — so they must never be re-derived per page.
///
/// ★★ This used to REFUSE ([`FormsError::Overflow`]) with a comment claiming it behaved "exactly as
/// the slice does" — which the slice had not done since T2. The consequence was total: the packet is
/// all-or-nothing, so a filer with 15 disposal legs got ZERO bytes, every form lost. The exposure is
/// LOT-COUNT-driven, not dollar-driven (P2b).
pub fn fill_8949_full_with_map(
    printed: &Printed8949,
    header: &ReturnHeader,
    map: &Form8949Map,
) -> Result<Vec<u8>, FormsError> {
    let cap = map.rows_per_page;
    let st = &printed.short_term;
    let lt = &printed.long_term;
    let n_copies = st.len().div_ceil(cap).max(lt.len().div_ceil(cap)).max(1);

    if n_copies == 1 {
        return fill_8949_parts_with_identity(
            &printed_part_data(st),
            &printed_part_data(lt),
            map,
            header,
        );
    }

    let chunk = |rows: &[Printed8949Row], k: usize| -> PartData {
        let lo = (k * cap).min(rows.len());
        let hi = (lo + cap).min(rows.len());
        printed_part_data(&rows[lo..hi])
    };
    let mut copies = Vec::with_capacity(n_copies);
    for k in 0..n_copies {
        // Each copy is filled on ORIGINAL names and geometry-verified there (fails closed).
        copies.push(fill_8949_parts_with_identity(
            &chunk(st, k),
            &chunk(lt, k),
            map,
            header,
        )?);
    }
    crate::overflow::merge_copies(&copies)
}
