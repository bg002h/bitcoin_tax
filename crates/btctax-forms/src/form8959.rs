//! Form 8959 (Additional Medicare Tax) fill — the §1401(b)(2) / §3101(b)(2) line chain, read back
//! through the flat-form geometric oracle (column-x cluster + ordinal-y descent + no-unmapped).
//!
//! **This module does no tax arithmetic.** Every printed cell is transcribed from
//! [`Form8959Lines`], which core derives from the same computed `Form8959` the rest of the return
//! uses. A second, independent derivation here is exactly how a filed PDF comes to disagree with
//! the tax it reports — so there isn't one.
//!
//! **Skip rule:** when line 18 (the Additional Medicare Tax) and line 24 (the withholding
//! reconciliation) are BOTH zero, the form is not required and is not produced (`Ok(None)`). Line
//! 24 matters independently of line 18: a taxpayer can owe no Additional Medicare Tax yet still
//! have had some over-withheld by an employer (each employer withholds on ITS OWN wages over
//! $200,000, with no knowledge of a spouse or a second job), and that excess is a *credit* on 1040
//! line 25c. Skipping on line 18 alone would silently forfeit it.

use crate::cells::push_money;
use crate::error::FormsError;
use crate::map::Form8959Map;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::other_taxes::Form8959Lines;
use btctax_core::Usd;

/// Logical Form 8959 columns: col 0 = MID, col 1 = AMOUNT.
const F8959_COL_MID: usize = 0;
const F8959_COL_AMOUNT: usize = 1;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF. This is the geometry ORACLE —
/// deliberately code-side, NEVER read from the (distrusted) map, so a map that points a line at the
/// wrong column fails closed instead of quietly printing a number in the wrong place.
const F8959_CLUSTERS: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];

/// Fill Form 8959 for `year` from the core-derived line chain. Returns `Ok(None)` when the form is
/// not required (line 18 and line 24 both zero — see the module note on why line 24 is not
/// redundant). Otherwise returns the serialized PDF bytes, read back through the geometric verifier
/// (a mis-mapped cell FAILS CLOSED).
pub fn fill_form_8959_with_map(
    lines: &Form8959Lines,
    map: &Form8959Map,
) -> Result<Option<Vec<u8>>, FormsError> {
    // The filing decision is a CORE fact (`p6-form8959-must-file-belongs-in-core`), so the packet's KATs
    // can see it — the filler only obeys it.
    if !lines.must_file() {
        return Ok(None);
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 17] = [
        (lines.line1, F8959_COL_MID),     // 1
        (lines.line4, F8959_COL_MID),     // 4
        (lines.line5, F8959_COL_MID),     // 5
        (lines.line6, F8959_COL_AMOUNT),  // 6
        (lines.line7, F8959_COL_AMOUNT),  // 7
        (lines.line8, F8959_COL_MID),     // 8
        (lines.line9, F8959_COL_MID),     // 9
        (lines.line10, F8959_COL_MID),    // 10
        (lines.line11, F8959_COL_MID),    // 11
        (lines.line12, F8959_COL_AMOUNT), // 12
        (lines.line13, F8959_COL_AMOUNT), // 13
        (lines.line18, F8959_COL_AMOUNT), // 18
        (lines.line19, F8959_COL_MID),    // 19
        (lines.line20, F8959_COL_MID),    // 20
        (lines.line21, F8959_COL_MID),    // 21
        (lines.line22, F8959_COL_AMOUNT), // 22
        (lines.line24, F8959_COL_AMOUNT), // 24
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

    let mut doc = pdf::load(pdf::f8959_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F8959_CLUSTERS)?;
    Ok(Some(bytes))
}
