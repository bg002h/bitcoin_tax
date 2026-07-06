//! Form 8283 (Rev. 12-2025) fill: donee/appraiser IDENTITY + per-row property data, read back through
//! the SP2 flat oracle (per-column x-cluster + PER-COLUMN ordinal-y descent [R0-M1] + no-unmapped).
//!
//! **Scope (a fill/blank table; [R0-I4]):** we FILL from `form_8283()`/`DonationDetails` — the donee
//! name/EIN/address (Part V identity), the donee/date/description/FMV/cost per row, the appraiser
//! identity name/address/TIN (Part IV identity), and (Section B) the "**k Digital assets**" property-
//! type box. We leave BLANK every OTHER party's declaration/signature: the Part II restriction
//! questions, the Part III taxpayer signature, the Part IV appraiser SIGNATURE/date, and the Part V
//! donee ACKNOWLEDGMENT (receipt date, "unrelated use?", authorized signature/title/date). A Section-B
//! 8283 without a signed Part IV/V is NOT filing-ready — the CLI says so and escalates when any row
//! `needs_review`.
//!
//! **Conditional + overflow:** written only when donations exist; one row per `RemovalLeg`, so a
//! multi-lot donation overflows the 4 Section-A / 3 Section-B rows onto additional form copies via
//! [`crate::overflow::merge_copies`] ("Attach one or more Forms 8283" sanctions it).

use crate::error::FormsError;
use crate::map::Form8283Map;
use crate::verify::{verify_flat, FlatPlacement};
use crate::{fmt_date, fmt_money, overflow, pdf};
use btctax_core::{Form8283HowAcquired, Form8283Row, Form8283Section};
use time::macros::format_description;

/// Section A column x-clusters (hand-pinned): donee(a), desc(c), date_contrib(d), date_acq(e), how(f),
/// cost(g), fmv(h), method(i).
const SEC_A_CLUSTERS: &[(f32, f32)] = &[
    (58.0, 230.0),
    (404.0, 576.0),
    (58.0, 122.0),
    (123.0, 186.0),
    (188.0, 280.0),
    (281.0, 352.0),
    (353.0, 424.0),
    (426.0, 576.0),
];
/// Section B column x-clusters (hand-pinned): desc(a), fmv(c), date_acq(d), how(e), cost(f),
/// deduction(i).
const SEC_B_CLUSTERS: &[(f32, f32)] = &[
    (59.0, 258.0),
    (504.0, 576.0),
    (58.0, 130.0),
    (131.0, 287.0),
    (288.0, 359.0),
    (504.0, 576.0),
];

/// Rows one physical Section A / Section B form copy holds before overflow.
const SEC_A_CAP: usize = 4;
const SEC_B_CAP: usize = 3;

/// Render Form 8283 "how acquired by donor" as the form word. `Review` (acquisition origin lost) is an
/// honest blank — the row is separately flagged `needs_review`.
fn how_str(h: Form8283HowAcquired) -> &'static str {
    match h {
        Form8283HowAcquired::Purchased => "Purchased",
        Form8283HowAcquired::Gift => "Gift",
        Form8283HowAcquired::Other => "Other",
        Form8283HowAcquired::Review => "",
    }
}

/// Format a date as **MM/YYYY** — Form 8283's "(mo., yr.)" date-acquired format (NOT SP1's MM/DD/YYYY).
fn fmt_mo_yr(d: btctax_core::TaxDate) -> Result<String, FormsError> {
    let fmt = format_description!("[month]/[year]");
    d.format(&fmt)
        .map_err(|e| FormsError::Structure(format!("mo/yr date format: {e}")))
}

/// Fill Form 8283 from the projected donation rows. `Ok(None)` when there are no donation rows.
pub fn fill_form_8283(
    rows: &[Form8283Row],
    map: &Form8283Map,
) -> Result<Option<Vec<u8>>, FormsError> {
    if rows.is_empty() {
        return Ok(None);
    }
    // The section is UNIFORM across the year (all BTC is one "similar property" class); read it off the
    // first carrier row (falls back to A only for a degenerate all-non-carrier input).
    let section = rows
        .iter()
        .find_map(|r| r.section)
        .unwrap_or(Form8283Section::A);
    let cap = match section {
        Form8283Section::A => SEC_A_CAP,
        Form8283Section::B => SEC_B_CAP,
    };
    let n_copies = rows.len().div_ceil(cap).max(1);

    if n_copies == 1 {
        let chunk: Vec<&Form8283Row> = rows.iter().collect();
        return Ok(Some(fill_one(&chunk, section, map)?));
    }
    let mut copies = Vec::with_capacity(n_copies);
    for k in 0..n_copies {
        let chunk: Vec<&Form8283Row> = rows.iter().skip(k * cap).take(cap).collect();
        // Each copy is filled on ORIGINAL names + geometry-verified here (fails closed), then merged.
        copies.push(fill_one(&chunk, section, map)?);
    }
    Ok(Some(overflow::merge_copies(&copies)?))
}

/// A page-1 row cell: written + authorized only when non-empty. `col` is both the x-cluster index and
/// the per-column ordinal-y descent group; `ord` is the row index (rows A→D / A→C descend in y).
fn push_cell(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    fqn: &str,
    value: String,
    col: usize,
    ord: u32,
) {
    if value.is_empty() {
        return;
    }
    w.push((fqn.to_string(), pdf::FieldValue::Text(value)));
    p.push(FlatPlacement::cell(
        fqn.to_string(),
        0,
        col,
        col as u32,
        ord,
    ));
}

/// A page-2 free-text identity cell (geometry-exempt): written + authorized only when non-empty.
fn push_free(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    fqn: &str,
    value: &str,
) {
    if value.is_empty() {
        return;
    }
    w.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
    p.push(FlatPlacement::free(fqn.to_string(), 1));
}

/// Fill one physical Form 8283 copy (a chunk of ≤ cap rows) and read it back geometrically.
fn fill_one(
    rows: &[&Form8283Row],
    section: Form8283Section,
    map: &Form8283Map,
) -> Result<Vec<u8>, FormsError> {
    let mut w: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut p: Vec<FlatPlacement> = Vec::new();

    let clusters: &[(f32, f32)] = match section {
        Form8283Section::A => {
            for (i, row) in rows.iter().enumerate() {
                let m = &map.section_a.rows[i];
                let ord = i as u32;
                push_cell(&mut w, &mut p, &m.donee, row.donee.clone(), 0, ord);
                push_cell(&mut w, &mut p, &m.desc, row.description.clone(), 1, ord);
                push_cell(
                    &mut w,
                    &mut p,
                    &m.date_contrib,
                    fmt_date(row.date_contributed)?,
                    2,
                    ord,
                );
                push_cell(
                    &mut w,
                    &mut p,
                    &m.date_acq,
                    fmt_mo_yr(row.date_acquired)?,
                    3,
                    ord,
                );
                push_cell(
                    &mut w,
                    &mut p,
                    &m.how,
                    how_str(row.how_acquired).to_string(),
                    4,
                    ord,
                );
                push_cell(&mut w, &mut p, &m.cost, fmt_money(row.cost_basis), 5, ord);
                push_cell(&mut w, &mut p, &m.fmv, fmt_money(row.fmv), 6, ord);
                push_cell(&mut w, &mut p, &m.method, row.fmv_method.clone(), 7, ord);
            }
            SEC_A_CLUSTERS
        }
        Form8283Section::B => {
            let b = &map.section_b;
            // [★] "k Digital assets" property-type box (MUST be checked for BTC).
            w.push((
                b.k_digital_assets.field.clone(),
                pdf::FieldValue::Check {
                    on: b.k_digital_assets.on.clone(),
                },
            ));
            p.push(FlatPlacement::check(b.k_digital_assets.field.clone(), 0));
            for (i, row) in rows.iter().enumerate() {
                let m = &b.rows[i];
                let ord = i as u32;
                push_cell(&mut w, &mut p, &m.desc, row.description.clone(), 0, ord);
                push_cell(&mut w, &mut p, &m.fmv, fmt_money(row.fmv), 1, ord);
                push_cell(
                    &mut w,
                    &mut p,
                    &m.date_acq,
                    fmt_mo_yr(row.date_acquired)?,
                    2,
                    ord,
                );
                push_cell(
                    &mut w,
                    &mut p,
                    &m.how,
                    how_str(row.how_acquired).to_string(),
                    3,
                    ord,
                );
                push_cell(&mut w, &mut p, &m.cost, fmt_money(row.cost_basis), 4, ord);
                if let Some(ded) = row.claimed_deduction {
                    push_cell(&mut w, &mut p, &m.deduction, fmt_money(ded), 5, ord);
                }
            }
            // Part IV (appraiser) + Part V (donee) IDENTITY — from the first carrier row's details.
            if let Some(details) = rows.iter().find_map(|r| r.details.as_ref()) {
                push_free(&mut w, &mut p, &b.appraiser_name, &details.appraiser_name);
                if let Some(a) = &details.appraiser_address {
                    push_free(&mut w, &mut p, &b.appraiser_address, a);
                }
                // §6695A appraiser identifier: TIN, else PTIN.
                if let Some(tin) = details
                    .appraiser_tin
                    .as_ref()
                    .or(details.appraiser_ptin.as_ref())
                {
                    push_free(&mut w, &mut p, &b.appraiser_tin, tin);
                }
                push_free(&mut w, &mut p, &b.donee_name, &details.donee_name);
                if let Some(ein) = &details.donee_ein {
                    push_free(&mut w, &mut p, &b.donee_ein, ein);
                }
                if let Some(addr) = &details.donee_address {
                    push_free(&mut w, &mut p, &b.donee_address, addr);
                }
            }
            SEC_B_CLUSTERS
        }
    };
    let writes = w;
    let placements = p;

    let mut doc = pdf::load(pdf::F8283_PDF_2025)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the SERIALIZED output.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, clusters)?;
    Ok(bytes)
}
