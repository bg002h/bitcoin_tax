//! Form 8949 fill: format the tax data into grid cells, authorize each write against the per-year
//! map, fill the bundled PDF, and READ BACK the serialized bytes through the geometric verifier.
//!
//! **Digital-asset box:** Bitcoin is filed under **Box I** (short-term) / **Box L** (long-term) —
//! the 2025 1099-DA revision — NEVER Box C/F ("other than digital asset transactions"). The box is
//! chosen by the part's on-state in the map, so the core `Form8949Box::{C,F}` taxonomy is not reused.

use crate::error::FormsError;
use crate::map::{Form8949Map, PartMap};
use crate::verify::{verify_8949, Geo, Placement};
use crate::{fmt_date, pdf};
use btctax_core::forms::Form8949Row;
use btctax_core::Form8949Part;
use rust_decimal::Decimal;

/// One part's formatted rows + totals, ready to place on the form.
pub struct PartData {
    /// Each row's 8 column strings (a..h); an empty string means "leave that cell blank".
    pub rows: Vec<[String; 8]>,
    /// Totals row: (d) proceeds, (e) cost, (g) adjustment, (h) gain — as display strings.
    pub totals: [String; 4],
    /// Whether the (g) adjustment total is non-zero (else that cell stays blank).
    pub adj_nonzero: bool,
}

/// Column indices on Form 8949 (a=0 … h=7).
const COL_D: usize = 3;
const COL_E: usize = 4;
const COL_G: usize = 6;
const COL_H: usize = 7;

/// Format one core row into its 8 column strings. Money is the exact `Decimal` Display (identical to
/// the `form8949.csv`); dates render MM/DD/YYYY (the form's native date format); the (f) code and a
/// zero (g) adjustment stay blank per IRS convention.
fn row_cells(r: &Form8949Row) -> Result<[String; 8], FormsError> {
    Ok([
        r.description.clone(),
        fmt_date(r.date_acquired)?,
        fmt_date(r.date_sold)?,
        r.proceeds.to_string(),
        r.cost_basis.to_string(),
        r.adjustment_code.clone(),
        if r.adjustment_amount.is_zero() {
            String::new()
        } else {
            r.adjustment_amount.to_string()
        },
        r.gain.to_string(),
    ])
}

/// Build one part's `PartData` from its rows.
pub fn part_data(rows: &[&Form8949Row]) -> Result<PartData, FormsError> {
    let mut out = Vec::with_capacity(rows.len());
    let (mut sp, mut sc, mut sg, mut sh) =
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    for r in rows {
        out.push(row_cells(r)?);
        sp += r.proceeds;
        sc += r.cost_basis;
        sg += r.adjustment_amount;
        sh += r.gain;
    }
    Ok(PartData {
        rows: out,
        totals: [
            sp.to_string(),
            sc.to_string(),
            sg.to_string(),
            sh.to_string(),
        ],
        adj_nonzero: !sg.is_zero(),
    })
}

/// Accumulate the writes + placements for one part onto its map page.
fn place_part(
    part: &PartMap,
    data: &PartData,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    if data.rows.is_empty() {
        return; // nothing to report on this part — leave the box + totals blank
    }
    // Digital-asset box (Box I / Box L).
    writes.push((
        part.box_field.clone(),
        pdf::FieldValue::Check {
            on: part.box_on.clone(),
        },
    ));
    placements.push(Placement {
        fqn: part.box_field.clone(),
        geo: Geo::Check,
    });
    // Data rows.
    for (ri, cells) in data.rows.iter().enumerate() {
        for (ci, value) in cells.iter().enumerate() {
            if value.is_empty() {
                continue; // blank cell — do not write, do not authorize
            }
            let fqn = part.rows[ri][ci].clone();
            writes.push((fqn.clone(), pdf::FieldValue::Text(value.clone())));
            placements.push(Placement {
                fqn,
                geo: Geo::Data { row: ri, col: ci },
            });
        }
    }
    // Per-part totals (the line-2 text says "Enter each total here").
    let totals = [
        (COL_D, &part.totals.proceeds_d, true),
        (COL_E, &part.totals.cost_e, true),
        (COL_G, &part.totals.adj_g, data.adj_nonzero),
        (COL_H, &part.totals.gain_h, true),
    ];
    let vals = [
        &data.totals[0],
        &data.totals[1],
        &data.totals[2],
        &data.totals[3],
    ];
    for (i, (col, fqn, include)) in totals.into_iter().enumerate() {
        if !include {
            continue;
        }
        writes.push((fqn.clone(), pdf::FieldValue::Text(vals[i].clone())));
        placements.push(Placement {
            fqn: fqn.clone(),
            geo: Geo::Total { col },
        });
    }
}

/// Fill Form 8949 (Part I + Part II, ≤ 11 rows each) into the bundled TY2025 PDF and return the
/// serialized bytes. The output is read back through the geometric verifier — a mis-mapped cell or a
/// stray write FAILS CLOSED (no bytes returned). Pagination for > 11 rows is handled by
/// [`crate::fill_form_8949`], which chunks before calling this.
pub fn fill_8949_parts(
    short: &PartData,
    long: &PartData,
    map: &Form8949Map,
) -> Result<Vec<u8>, FormsError> {
    if short.rows.len() > map.rows_per_page {
        return Err(FormsError::Overflow {
            part: "Part I",
            rows: short.rows.len(),
            capacity: map.rows_per_page,
        });
    }
    if long.rows.len() > map.rows_per_page {
        return Err(FormsError::Overflow {
            part: "Part II",
            rows: long.rows.len(),
            capacity: map.rows_per_page,
        });
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    if let Some(p) = map.part("short") {
        place_part(p, short, &mut writes, &mut placements);
    }
    if let Some(p) = map.part("long") {
        place_part(p, long, &mut writes, &mut placements);
    }

    let mut doc = pdf::load(pdf::f8949_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_8949(&check, &fields, &placements, &map.table_token)?;
    // XFA must be gone on the actual output.
    if pdf_has_xfa(&check)? {
        return Err(FormsError::Structure(
            "output still carries /XFA after fill".into(),
        ));
    }
    Ok(bytes)
}

/// Whether the document's AcroForm still carries an `/XFA` key.
pub fn pdf_has_xfa(doc: &lopdf::Document) -> Result<bool, FormsError> {
    let acro = match doc.catalog()?.get(b"AcroForm") {
        Ok(lopdf::Object::Reference(id)) => doc.get_dictionary(*id)?,
        Ok(lopdf::Object::Dictionary(d)) => d,
        _ => return Ok(false),
    };
    Ok(acro.has(b"XFA"))
}

/// Split the year's rows into Part I (short-term) and Part II (long-term), preserving the core's
/// deterministic order.
pub fn split_parts(rows: &[Form8949Row]) -> (Vec<&Form8949Row>, Vec<&Form8949Row>) {
    let mut st = Vec::new();
    let mut lt = Vec::new();
    for r in rows {
        match r.part {
            Form8949Part::ShortTerm => st.push(r),
            Form8949Part::LongTerm => lt.push(r),
        }
    }
    (st, lt)
}
