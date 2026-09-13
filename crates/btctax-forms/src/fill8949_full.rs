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
        box_letter: rows.first().map(|r| format!("{:?}", r.box_)),
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
/// ORIGINAL field names, then merged with per-copy field renaming ([`crate::overflow::merge_pages`])
/// so no two copies share a `/V`. Each copy carries the FILER's identity on every page it FILES —
/// every filed 8949 page needs its own header (P6 r1 I3). ★ FR-113 — and the merged
/// page order GROUPS the parts (every Part I page, then every Part II page), a permutation of the
/// page tree that leaves every cell and value where it was.
///
/// ★★★ **FR-218 — the two parts paginate INDEPENDENTLY.** The emitted form carries `|ST pages|` Part I
/// pages and `|LT pages|` Part II pages, and nothing else. The COPY count is still `max(|ST|, |LT|)`
/// (a copy is one physical template), but a copy whose part has run out of rows files only its other
/// page — `i8949` says *"complete and file **either Part I or II**"*
/// (`design/forms/extract/i8949--2024.txt:422-424`). Before the fix the copy count was the max and
/// each copy filed BOTH pages, so a long-term-only filer needing two Part II pages received two blank
/// Part I pages carrying their name and SSN. The owner found it by printing the packet, with the
/// golden, the read-back verifier, both oracles and the Schedule D roll-up all green.
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
    // ★ spec 1099-DA R3/T3 — the same rule as `fill_form_8949`: per part, rows grouped by BOX in
    //   letter order, each group paginated on its own, the groups' pages concatenated; copies =
    //   max(|ST pages|, |LT pages|); a page never mixes boxes.
    //   ★★ FR-218 — an exhausted side is NOT "left blank": it contributes no page to that copy.
    fn pages(part: &[Printed8949Row], cap: usize) -> Vec<Vec<Printed8949Row>> {
        let mut by_box: std::collections::BTreeMap<String, Vec<Printed8949Row>> =
            std::collections::BTreeMap::new();
        for r in part {
            by_box
                .entry(format!("{:?}", r.box_))
                .or_default()
                .push(r.clone());
        }
        let mut out = Vec::new();
        for (_, group) in by_box {
            for chunk in group.chunks(cap) {
                out.push(chunk.to_vec());
            }
        }
        out
    }
    let st_pages = pages(&printed.short_term, cap);
    let lt_pages = pages(&printed.long_term, cap);
    let n_copies = st_pages.len().max(lt_pages.len()).max(1);
    let empty: Vec<Printed8949Row> = Vec::new();
    let mut copies = Vec::with_capacity(n_copies);
    for k in 0..n_copies {
        copies.push(fill_8949_parts_with_identity(
            &printed_part_data(st_pages.get(k).unwrap_or(&empty)),
            &printed_part_data(lt_pages.get(k).unwrap_or(&empty)),
            map,
            header,
        )?);
    }
    if n_copies == 1 {
        // One copy is already the whole document, in the right order, with FR-218's reduction applied
        // by the filler — so the common case is byte-identical to a direct fill.
        return Ok(copies.remove(0));
    }
    // ★ FR-113 — grouped by part, exactly as the slice path does: every Part I page, then every Part
    //   II page. A permutation of the page tree; the rows, their cells and their totals are untouched.
    // ★★ FR-218 — and the plan emits exactly |ST pages| Part I pages and |LT pages| Part II pages.
    crate::overflow::merge_pages(
        &copies,
        &part_page_plan(map, st_pages.len(), lt_pages.len())?,
    )
}

/// The [`crate::overflow::PagePick`] plan for these two parts' page counts, with each part's page index
/// read off the MAP rather than assumed ("Part I is page 0").
pub(crate) fn part_page_plan(
    map: &Form8949Map,
    st_pages: usize,
    lt_pages: usize,
) -> Result<Vec<crate::overflow::PagePick>, FormsError> {
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (term, n) in [("short", st_pages), ("long", lt_pages)] {
        if n == 0 {
            continue; // a part with no rows contributes no page, so its page index is not consulted
        }
        let page = map.part(term).map(|p| p.page).ok_or_else(|| {
            FormsError::Structure(format!(
                "the {} Form 8949 map declares no {term}-term part, so the {n} page(s) of rows \
                 routed to it have no page to print on",
                map.year
            ))
        })?;
        groups.push((page, n));
    }
    Ok(crate::overflow::independent_part_plan(&groups))
}
