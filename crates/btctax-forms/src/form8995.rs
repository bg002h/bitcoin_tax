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

use crate::cells::{push_identity, push_literal, push_money};
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
/// Part I row 1i's three cells. They share a y, so they cannot join the ordinal-y descent group — the
/// column band is their check, and it is deliberately TIGHT: `row1_qbi`'s x-center is **529.2**, which
/// sits INSIDE the ordinary AMOUNT cluster [504, 576] (center 540). A loose band would happily accept a
/// cell mis-mapped between the two. `row1_qbi` is additionally pinned as descent ordinal 0, which
/// asserts the row really does sit ABOVE line 2.
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

/// Fill Form 8995 from the core-derived line chain. The serialized bytes are read back through the
/// geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_form_8995_with_map(
    lines: &Form8995Lines,
    header: &ReturnHeader,
    map: &Form8995Map,
) -> Result<Vec<u8>, FormsError> {
    assert_paren_magnitudes(lines)?;

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── Part I row 1i — the trade or business, when there is one. ───────────────────────────────────
    // Written BEFORE the lines so the descent group reads top-down: row 1i(c) is ordinal 0 and line 2
    // must sit strictly below it.
    if !lines.business_name.is_empty() {
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_business,
            &lines.business_name,
            F8995_COL_ROW1_BUSINESS,
        );
        // (b) the TIN. A sole proprietor's is their own SSN; btctax has no EIN input. The cell's
        // /MaxLen is 11, i.e. the HYPHENATED form — `ssn_for_cell` reads the blank PDF's own /MaxLen
        // rather than guessing, exactly as the identity header does.
        push_literal(
            &mut writes,
            &mut placements,
            &map.row1_tin,
            &header.taxpayer.ssn.hyphenated(),
            F8995_COL_ROW1_TIN,
        );
        // (c) this business's QBI. With one business the column total IS the row, so line 2 is written
        // from the same figure — they cannot disagree.
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

    let mut doc = pdf::load(pdf::f8995_pdf(map.year)?)?;
    // Identity header (P6.2): `push_identity` reads the SSN cell's own /MaxLen to decide
    // hyphenated-vs-digits, so it needs the blank PDF's fields.
    let blank_fields = pdf::collect_fields(&doc)?;
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
