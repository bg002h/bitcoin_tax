//! Schedule D fill: lines 3 & 7 (short-term), 10 & 15 (long-term), 16 (total), and the QOF Yes/No
//! question (SP1 answers **No**). Lines 7/15 are pure arithmetic (only line 3 / line 10 feed them in
//! SP1), so filling 16 while leaving 7/15 blank would be self-inconsistent — we fill all of them.
//!
//! **Scope-out:** lines 17-22 (the 28%-rate / unrecaptured-§1250 / QDI worksheet path) are NOT filled
//! — the CLI prints a notice. Line 21 (the §1211 loss limit) lives inside that scoped-out block.
//!
//! Schedule D line 3 reads "Box C **or Box I**" and line 10 "Box F **or Box L**", so the Box I/L
//! digital-asset totals from Form 8949 flow straight onto these lines.

use crate::error::FormsError;
use crate::map::{AmountCols, ScheduleDMap};
use crate::verify::{column_x_bands, in_band, no_unmapped_filled, Geo, Placement};
use crate::{fmt_money, pdf};
use btctax_core::ScheduleDTotals;

/// Schedule D's Part I amount-column subform token (columns d,e,g,h).
const SCHED_D_TABLE_TOKEN: &str = "Table_PartI";
/// Amount-column indices as ordered left→right in the Part I grid: d=0, e=1, g=2, h=3.
const SD_COL_D: usize = 0;
const SD_COL_E: usize = 1;
const SD_COL_H: usize = 3;

/// A part is "active" (worth a Schedule D line) iff it has any proceeds/cost/gain.
fn active(p: &btctax_core::ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

fn push_amount_line(
    line: &AmountCols,
    proceeds: &str,
    cost: &str,
    gain: &str,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    // (d) proceeds, (e) cost, (h) gain — (g) adjustment stays blank (crypto models no adjustment).
    for (fqn, value, col) in [
        (&line.proceeds_d, proceeds, SD_COL_D),
        (&line.cost_e, cost, SD_COL_E),
        (&line.gain_h, gain, SD_COL_H),
    ] {
        writes.push((fqn.clone(), pdf::FieldValue::Text(value.to_string())));
        placements.push(Placement {
            fqn: fqn.clone(),
            geo: Geo::Data { row: 0, col },
        });
    }
}

/// Fill Schedule D from the year's part totals and return the serialized bytes, read back through a
/// geometric + no-unmapped verifier (a mis-mapped amount column fails closed).
pub fn fill_schedule_d_totals(
    totals: &ScheduleDTotals,
    map: &ScheduleDMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    // These are just amount-column bands checks, so reuse Geo::Data{row:0} (row band is ignored for
    // Schedule D — only the column band matters; see verify_schedule_d below).

    let st_active = active(&totals.st);
    let lt_active = active(&totals.lt);

    if st_active {
        // Line 3 (Part I total = Box I).
        push_amount_line(
            &map.line3,
            &fmt_money(totals.st.proceeds),
            &fmt_money(totals.st.cost_basis),
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
        // Line 7 (net short-term = line 3 h in SP1).
        push_h_line(
            &map.line7_h,
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
    }
    if lt_active {
        // Line 10 (Part II total = Box L).
        push_amount_line(
            &map.line10,
            &fmt_money(totals.lt.proceeds),
            &fmt_money(totals.lt.cost_basis),
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
        // Line 15 (net long-term = line 10 h in SP1).
        push_h_line(
            &map.line15_h,
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
    }
    if st_active || lt_active {
        // Line 16 = line 7 + line 15.
        let total = totals.st.gain + totals.lt.gain;
        push_h_line(
            &map.line16_h,
            &fmt_money(total),
            &mut writes,
            &mut placements,
        );
    }
    // Always answer the QOF question — No for SP1.
    writes.push((
        map.qof_no.field.clone(),
        pdf::FieldValue::Check {
            on: map.qof_no.on.clone(),
        },
    ));
    placements.push(Placement {
        fqn: map.qof_no.field.clone(),
        geo: Geo::Check,
    });

    let mut doc = pdf::load(pdf::SCHEDULE_D_PDF_2025)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the serialized output.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_schedule_d(&check, &fields, &placements)?;
    Ok(bytes)
}

fn push_h_line(
    fqn: &str,
    value: &str,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    writes.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
    placements.push(Placement {
        fqn: fqn.to_string(),
        geo: Geo::Total { col: SD_COL_H }, // single h-column amount (Total variant = "amount column")
    });
}

/// Geometric + no-unmapped read-back for Schedule D. The amount-column x-bands (d,e,g,h) are
/// re-derived from the Part I grid; each written value must land in the column its logical line
/// demands. Row/line y is not a uniform grid on Schedule D, so only the column band is geometric
/// here — enough to catch a swapped-amount-column map — plus the no-unmapped guard.
pub fn verify_schedule_d(
    doc: &lopdf::Document,
    fields: &[pdf::Field],
    placements: &[Placement],
) -> Result<(), FormsError> {
    let bands = column_x_bands(fields, 0, SCHED_D_TABLE_TOKEN)?;
    let index: std::collections::HashMap<&str, &pdf::Field> =
        fields.iter().map(|f| (f.fqn.as_str(), f)).collect();
    for p in placements {
        let field = index
            .get(p.fqn.as_str())
            .ok_or_else(|| FormsError::MapFieldMissing(p.fqn.clone()))?;
        let col = match &p.geo {
            Geo::Data { col, .. } => *col,
            Geo::Total { col } => *col,
            Geo::Check => continue,
        };
        // Line 16 lives on page 2 (its own h column); skip the page-1 band check for it but keep it
        // in the no-unmapped set.
        if p.fqn.contains("Page2") {
            continue;
        }
        let cx = field
            .cx()
            .ok_or_else(|| FormsError::Geometry(format!("{}: no /Rect", p.fqn)))?;
        let band = *bands
            .get(col)
            .ok_or_else(|| FormsError::Geometry(format!("column {col} out of range")))?;
        if !in_band(cx, band) {
            return Err(FormsError::Geometry(format!(
                "{}: x-center {cx:.1} not in amount column {col} band {band:?} (mis-mapped column)",
                p.fqn
            )));
        }
    }
    no_unmapped_filled(doc, fields, placements)
}
