//! Schedule A (Itemized Deductions) fill — read back through the flat-form geometric oracle.
//!
//! **This module does no tax arithmetic.** Every printed cell is transcribed from [`ScheduleALines`],
//! which core derives — including the medical floor taken on the *printed* AGI and the SALT cap
//! applied to the *printed* line 5d, so the filed form cross-foots against itself.
//!
//! **★ Three x-clusters.** Schedule A is the only form in this crate that needs a third. Line 2 — the
//! AGI the 7.5% §213(a) medical floor is taken on — is NOT in the MID column: its widget sits inline
//! with the printed sentence at x ≈ [331, 403], some 86pt left of MID, and it is the *same width* as a
//! MID box. So neither a column check keyed on MID nor a width heuristic would catch a cell
//! mis-mapped into it; only its own cluster does. Getting this wrong would print the AGI into the
//! medical-expenses box, and the floor would be computed against the wrong number.

use crate::cells::push_money;
use crate::error::FormsError;
use crate::map::ScheduleAMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::printed::ScheduleALines;
use btctax_core::Usd;

/// Logical Schedule A columns.
const COL_AGI_INLINE: usize = 0;
const COL_MID: usize = 1;
const COL_AMOUNT: usize = 2;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF. Code-side oracle — never taken
/// from the (distrusted) map. The AGI-inline band is deliberately tight: it is the whole reason a
/// mis-mapped line 2 fails closed instead of silently printing the AGI in the wrong box.
const SCHEDULE_A_CLUSTERS: &[(f32, f32)] = &[(331.0, 403.0), (417.0, 489.0), (504.0, 576.0)];

/// Fill Schedule A from the core-derived printed chain. The serialized bytes are read back through
/// the geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_schedule_a_with_map(
    lines: &ScheduleALines,
    map: &ScheduleAMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 18] = [
        (lines.line1, COL_MID),        // 1  medical expenses
        (lines.line2, COL_AGI_INLINE), // 2  ★ AGI — its own column, not MID
        (lines.line3, COL_MID),        // 3  the 7.5% floor
        (lines.line4, COL_AMOUNT),     // 4  medical allowed
        (lines.line5a, COL_MID),       // 5a income OR sales tax
        (lines.line5b, COL_MID),       // 5b real estate
        (lines.line5c, COL_MID),       // 5c personal property
        (lines.line5d, COL_MID),       // 5d add 5a-5c
        (lines.line5e, COL_MID),       // 5e the §164(b) cap
        (lines.line7, COL_AMOUNT),     // 7  add 5e and 6
        (lines.line8a, COL_MID),       // 8a mortgage interest (Form 1098)
        (lines.line8e, COL_MID),       // 8e add 8a-8c
        (lines.line10, COL_AMOUNT),    // 10 add 8e and 9
        (lines.line11, COL_MID),       // 11 gifts by cash or check
        (lines.line12, COL_MID),       // 12 gifts other than cash (incl. crypto)
        (lines.line13, COL_MID),       // 13 prior-year carryover
        (lines.line14, COL_AMOUNT),    // 14 add 11-13
        (lines.line17, COL_AMOUNT),    // 17 total itemized -> 1040 L12
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

    let mut doc = pdf::load(pdf::schedule_a_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCHEDULE_A_CLUSTERS)?;
    Ok(bytes)
}
