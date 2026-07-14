//! Form 8960 (Net Investment Income Tax, §1411) fill — read back through the flat-form geometric
//! oracle (column-x cluster + ordinal-y descent + no-unmapped).
//!
//! **This module does no tax arithmetic.** Every printed cell is transcribed from [`Form8960Lines`],
//! which core derives from the same figures the rest of the return uses. A second, independent
//! derivation here is exactly how a filed PDF comes to disagree with the tax it reports.
//!
//! The form is produced only when core says NIIT is owed (`form_8960_lines` returns `Some`); §1411
//! imposes no tax below the MAGI threshold or with no net investment income, so there is nothing to
//! file. Part III's estates-and-trusts branch (lines 18a–21) is never touched — on an individual
//! return it must be blank.

use crate::cells::{push_identity, push_money};
use crate::error::FormsError;
use crate::map::Form8960Map;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::other_taxes::Form8960Lines;
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::Usd;

/// Logical Form 8960 columns: col 0 = MID, col 1 = AMOUNT.
const F8960_COL_MID: usize = 0;
const F8960_COL_AMOUNT: usize = 1;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF. The geometry ORACLE — code-side,
/// never read from the (distrusted) map, so a map that points a line at the wrong column fails closed.
const F8960_CLUSTERS: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];

/// Fill Form 8960 from the core-derived line chain. The serialized bytes are read back through the
/// geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_form_8960_with_map(
    lines: &Form8960Lines,
    header: &ReturnHeader,
    map: &Form8960Map,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 14] = [
        (lines.line1, F8960_COL_AMOUNT),  // 1  taxable interest
        (lines.line2, F8960_COL_AMOUNT),  // 2  ordinary dividends
        (lines.line5a, F8960_COL_MID),    // 5a net gain/loss on disposition
        (lines.line5d, F8960_COL_AMOUNT), // 5d combine 5a-5c
        (lines.line7, F8960_COL_AMOUNT),  // 7  other modifications
        (lines.line8, F8960_COL_AMOUNT),  // 8  total investment income
        (lines.line9d, F8960_COL_AMOUNT), // 9d add 9a-9c (zero, but the form adds it)
        (lines.line11, F8960_COL_AMOUNT), // 11 total deductions/modifications
        (lines.line12, F8960_COL_AMOUNT), // 12 net investment income
        (lines.line13, F8960_COL_MID),    // 13 MAGI
        (lines.line14, F8960_COL_MID),    // 14 threshold
        (lines.line15, F8960_COL_MID),    // 15 13 - 14, floored
        (lines.line16, F8960_COL_AMOUNT), // 16 smaller of 12 or 15
        (lines.line17, F8960_COL_AMOUNT), // 17 3.8% x 16 -> Schedule 2 line 12
    ];
    for (ord, (cell, (value, col))) in map.lines().iter().zip(plan).enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            col,
            Some((0, ord as u32)),
        );
    }

    let mut doc = pdf::load(pdf::f8960_pdf(map.year)?)?;
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
    verify_flat(&check, &fields, &placements, F8960_CLUSTERS)?;
    Ok(bytes)
}
