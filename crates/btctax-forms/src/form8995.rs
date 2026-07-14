//! Form 8995 (Qualified Business Income Deduction, Simplified) fill — read back through the
//! flat-form geometric oracle (column-x cluster + ordinal-y descent + no-unmapped).
//!
//! **This module does no tax arithmetic.** Every printed cell is transcribed from [`Form8995Lines`].
//!
//! **★ The parenthesized boxes (lines 7, 16, 17).** The form pre-prints literal `(   )` around these
//! cells: the parentheses ARE the minus sign. A value written there must be a POSITIVE MAGNITUDE, or
//! the filed form reads `(-1,234)` — a positive number, and a wrong return. `Form8995Lines` stores
//! them as magnitudes ≥ 0 for exactly this reason, and [`assert_paren_magnitudes`] fails closed if
//! that invariant is ever broken upstream. The invariant is invisible in the PDF's field data — it is
//! only visible on the rendered page — so it gets an explicit guard rather than a comment.
//!
//! **★ Part I row 1i carries the trade or business** (Fable P7 r1 I1). Line 2's own text is "Total
//! qualified business income or (loss). **Combine lines 1i through 1v, column (c)**" — so a non-zero
//! line 2 over an empty column is a filed form that totals nothing and names no business for the
//! deduction it claims. Before P7 the table was legitimately blank (line 2 was always ZERO, a total of
//! nothing over nothing); P7 gave the crypto Schedule C a §199A deduction and left the column empty.
//! The row is written exactly when there IS a business — `Form8995Lines::business_name` is empty
//! otherwise, and then line 2 is zero and the row stays blank.

use crate::cells::{push_identity, push_literal, push_money, render_ssn};
use crate::error::FormsError;
use crate::map::Form8995Map;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::qbi::Form8995Lines;
use btctax_core::Usd;

/// Logical Form 8995 columns: col 0 = MID, col 1 = AMOUNT.
///
/// The four parenthesized cells are inset ~4pt inside their column (MID-paren [414.4,478.4] within
/// MID [410.4,481.6]; AMOUNT-paren [508,572] within AMOUNT [504,576]) — but the oracle bands the
/// widget's x-CENTER, and the inset boxes share their column's center (446.4 ≈ 446.0; 540.0 = 540.0).
/// So two clusters suffice, and no cell gets a weaker check than its neighbours.
const F8995_COL_MID: usize = 0;
const F8995_COL_AMOUNT: usize = 1;
/// Part I row 1i's three cells.
///
/// The column bands are deliberately TIGHT: `row1_qbi`'s x-center is **529.2**, which sits INSIDE the
/// ordinary AMOUNT cluster [504, 576] (center 540), so a loose band would happily accept a cell
/// mis-mapped between the two.
///
/// Only ONE of the three can carry an ordinal-y descent ordinal — they share a y, and descent demands
/// a STRICT decrease — so `row1_qbi` takes ordinal 0, which asserts the row really does sit above line
/// 2. The other two are y-banded instead (`Y_ROW1`): the (a) and (b) columns are shared by all FIVE
/// table rows (y = 564/540/516/492/468), so without a y check a map typo pointing `row1_business` at
/// row 1ii would print the business name on a different row than its income, and pass geometry
/// silently (Fable P7 r2, Minor).
const F8995_COL_ROW1_BUSINESS: usize = 2;
const F8995_COL_ROW1_TIN: usize = 3;
const F8995_COL_ROW1_QBI: usize = 4;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF. Code-side oracle; never the map.
const F8995_CLUSTERS: &[(f32, f32)] = &[
    (410.0, 482.0), // MID
    (504.0, 576.0), // AMOUNT
    (226.0, 235.0), // row 1i (a) — center 230.5
    (435.0, 443.0), // row 1i (b) — center 439.2
    (525.0, 533.0), // row 1i (c) — center 529.2  (NOT 540: tight, or AMOUNT would swallow it)
];

/// Fail closed if any parenthesized cell carries a negative value. On the printed form the
/// parentheses supply the minus sign, so a negative here would RENDER AS POSITIVE — silently
/// converting a loss carryforward into income. Cheap, and it guards a hazard no geometric check can
/// see.
fn assert_paren_magnitudes(lines: &Form8995Lines) -> Result<(), FormsError> {
    for (line, v) in [
        ("7", lines.line7),
        ("16", lines.line16),
        ("17", lines.line17),
    ] {
        if v < Usd::ZERO {
            return Err(FormsError::Geometry(format!(
                "Form 8995 line {line} is a PARENTHESIZED box (the form prints the minus sign), so it \
                 must carry a positive magnitude — got {v}. Writing this would render as a POSITIVE \
                 number on the filed return."
            )));
        }
    }
    Ok(())
}

/// ★ **A row is a row.** The three Part I row-1i cells must occupy the same ROW of the table.
///
/// Only `row1_qbi` can carry a descent ordinal (the three share a row, and descent demands a STRICT
/// y-decrease), and the (a) and (b) columns are shared by ALL FIVE table rows — the row bands are
/// y = [552,576], [528,552], [504,528], [480,504], [456,480]. So without this, a map typo pointing
/// `row1_business` at row **1ii**'s name cell would pass every geometric check and print the business's
/// name on a different row from its income (Fable P7 r2, Minor).
///
/// Their y-CENTERS are not equal and must not be required to be. Measured from the blank PDF: (a) spans
/// y [551.97, 575.97] — the full 24pt row, center 564 — while (b) and (c) are 12pt cells at
/// y [551.97, 563.97], center 558, sitting in its LOWER half. The invariant is CONTAINMENT: (b) and (c)
/// must fall inside (a)'s y-extent, which *is* the row. Mis-map any ONE of the three and containment
/// breaks — including (a) itself, since the band then moves to another row and leaves (b)/(c) outside.
///
/// It does NOT reject all three moved CONSISTENTLY to row 1ii — and it should not: line 2 reads "Combine
/// lines 1i through **1v**", so a business listed on row 1ii is a validly filled form. This catches
/// exactly the harmful case, which is a row whose name and income are on different lines.
fn assert_row1_is_one_row(blank: &[pdf::Field], map: &Form8995Map) -> Result<(), FormsError> {
    let field = |cell: &crate::map::MoneyCell| -> Result<&pdf::Field, FormsError> {
        let fqn = match cell {
            crate::map::MoneyCell::Single(f) => f,
            crate::map::MoneyCell::Pair(mp) => &mp.dollars_field,
        };
        blank
            .iter()
            .find(|f| &f.fqn == fqn)
            .ok_or_else(|| FormsError::MapFieldMissing(fqn.clone()))
    };
    // (a) spans the whole row: its rect IS the row band.
    let a = field(&map.row1_business)?;
    let [_, y0, _, y1] = a.rect.ok_or_else(|| {
        FormsError::Geometry("Form 8995 row 1i(a) has no /Rect to band the row with".into())
    })?;
    let (lo, hi) = (y0.min(y1), y0.max(y1));

    for (what, cell) in [("(b) TIN", &map.row1_tin), ("(c) QBI", &map.row1_qbi)] {
        let f = field(cell)?;
        let cy = f
            .cy()
            .ok_or_else(|| FormsError::MapFieldMissing(f.fqn.clone()))?;
        if cy < lo || cy > hi {
            return Err(FormsError::Geometry(format!(
                "Form 8995 Part I row 1i is not one ROW: cell {what} sits at y {cy:.1}, outside row 1i's \
                 band [{lo:.1}, {hi:.1}] (taken from the full-height (a) cell). A row whose name and \
                 income are on different lines names the WRONG business."
            )));
        }
    }
    Ok(())
}

/// Fill Form 8995 from the core-derived line chain. The serialized bytes are read back through the
/// geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_form_8995_with_map(
    lines: &Form8995Lines,
    header: &ReturnHeader,
    map: &Form8995Map,
) -> Result<Vec<u8>, FormsError> {
    assert_paren_magnitudes(lines)?;

    // The blank PDF is loaded FIRST: row 1i's TIN rendering reads that cell's own /MaxLen, and the
    // row-integrity check bands the row off the (a) cell's own /Rect. Both ask the PDF, not the map.
    let mut doc = pdf::load(pdf::f8995_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;
    assert_row1_is_one_row(&blank_fields, map)?;

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── Part I row 1i — the trade or business. ─────────────────────────────────────────────────────
    //
    // ★ Keyed off the QBI (line 2), NOT off the business NAME (Fable P7 r2 I2). Gating on the name made
    // the whole defect conditional on an unvalidated free-text field: `business_description` is
    // `#[serde(default)]`, so an import that omitted it produced a blank name — and therefore a blank
    // row — under a NON-ZERO line 2. That is precisely the defect r1 I1 raised, re-created. Core now
    // REFUSES an unnamed Schedule C (`ScheduleCNoBusinessDescription`), and this fails closed if one
    // ever reaches here anyway: a form claiming a deduction for a business it cannot name must not be
    // produced at all.
    if lines.line2 > Usd::ZERO {
        if lines.business_name.trim().is_empty() {
            return Err(FormsError::Geometry(
                "Form 8995 line 2 is non-zero but the trade or business has no name: line 2 is \"Total \
                 qualified business income or (loss). Combine lines 1i through 1v, column (c)\", so this \
                 would file a total over an EMPTY column, claiming a §199A deduction for a business the \
                 return never names."
                    .into(),
            ));
        }
        // (b) the business's TIN. ★ The PROPRIETOR's, not the return's primary taxpayer's (Fable P7 r2
        // I1). A spouse-owned business files under the SPOUSE's name and SSN even on a joint return —
        // Schedule C and Schedule SE already do this via `header.proprietor`, and an 8995 reporting a
        // business TIN that matches no Schedule C in the same packet is exactly the mismatch IRS
        // matching flags. A sole proprietor's TIN is their own SSN; btctax has no EIN input.
        let proprietor = header.proprietor.as_ref().ok_or_else(|| {
            FormsError::Geometry(
                "Form 8995 has trade-or-business QBI but the return names no proprietor to file the \
                 business under"
                    .into(),
            )
        })?;
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_business,
            &lines.business_name,
            F8995_COL_ROW1_BUSINESS,
        );
        // ★ Rendered through `render_ssn` against the cell's OWN /MaxLen, not hardcoded (Fable P7 r3,
        // Minor). Every other SSN cell in this crate goes through that guard, and it fails closed on a
        // cell too narrow to hold an SSN — "the map points at the wrong widget". TY2024's row 1i(b) has
        // /MaxLen 11 (hyphenated), but hardcoding that would silently write 11 characters into a 9-char
        // comb if a later revision narrows the cell, while every other SSN in the packet failed closed.
        let tin_fqn = map.row1_tin.fields()[0];
        let tin_max_len = blank_fields
            .iter()
            .find(|f| f.fqn == tin_fqn)
            .ok_or_else(|| FormsError::MapFieldMissing(tin_fqn.to_string()))?
            .max_len;
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_tin,
            &render_ssn(&proprietor.ssn, tin_max_len)?,
            F8995_COL_ROW1_TIN,
        );
        // (c) this business's QBI. With one business the column total IS the row, so it is written from
        // the same value as line 2 — they cannot disagree.
        push_money(
            &mut writes,
            &mut placements,
            &map.row1_qbi,
            lines.line2,
            F8995_COL_ROW1_QBI,
            Some((0, 0)),
        );
    }

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 15] = [
        (lines.line2, F8995_COL_MID),     // 2  total QBI (table blank ⇒ 0)
        (lines.line4, F8995_COL_MID),     // 4  combine 2 and 3
        (lines.line5, F8995_COL_AMOUNT),  // 5  QBI component
        (lines.line6, F8995_COL_MID),     // 6  qualified REIT dividends + PTP income
        (lines.line7, F8995_COL_MID),     // 7  ★ paren — prior-year loss carryforward (magnitude)
        (lines.line8, F8995_COL_MID),     // 8  combine 6 and 7
        (lines.line9, F8995_COL_AMOUNT),  // 9  REIT/PTP component
        (lines.line10, F8995_COL_AMOUNT), // 10 add 5 and 9
        (lines.line11, F8995_COL_MID),    // 11 taxable income before QBI
        (lines.line12, F8995_COL_MID),    // 12 net capital gain + qualified dividends
        (lines.line13, F8995_COL_MID),    // 13 11 - 12, floored
        (lines.line14, F8995_COL_AMOUNT), // 14 income limitation
        (lines.line15, F8995_COL_AMOUNT), // 15 the deduction -> 1040 L13
        (lines.line16, F8995_COL_AMOUNT), // 16 ★ paren — QB loss carryforward (magnitude)
        (lines.line17, F8995_COL_AMOUNT), // 17 ★ paren — REIT/PTP loss carryforward (magnitude)
    ];
    for (ord, (cell, (value, col))) in map.lines().iter().zip(plan).enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            col,
            // +1: row 1i(c) took ordinal 0, and it sits above line 2 on the page.
            Some((0, ord as u32 + 1)),
        );
    }

    // Identity header (P6.2): `push_identity` reads the SSN cell's own /MaxLen to decide
    // hyphenated-vs-digits, so it needs the blank PDF's fields.
    push_identity(
        &mut writes,
        &mut placements,
        &map.identity,
        &header.name_line,
        &header.taxpayer.ssn,
        &blank_fields,
    )?;
    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F8995_CLUSTERS)?;
    Ok(bytes)
}
