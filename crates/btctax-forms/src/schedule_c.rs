//! Schedule C (Profit or Loss From Business) fill — the crypto trade or business.
//!
//! **This module does no tax arithmetic.** Every printed cell is transcribed from [`ScheduleCLines`].
//!
//! **★ Schedule C's money column is x ≈ [475, 576].** Not the [504, 576] of Schedules 1/2/3/A,
//! Schedule SE and Forms 8959/8960/8995; not Schedule B's [489.6, 576] either. Every form in this
//! crate places its amount column slightly differently, so no cluster constant is shared between
//! them — each filler pins its own, measured from its own blank PDF.
//!
//! **★ Line 31 is not where its label is.** "Net profit or (loss)" spans three printed rows (the
//! label, then two bullet rows telling the filer where the figure goes). Its gutter label sits at
//! y ≈ 144.5 but its amount box is at y ≈ 120.5, two rows lower — and line 30 has the same shape. The
//! map was built by correlating the RIGHT-EDGE line markers (the number printed immediately left of
//! each amount box), not the gutter labels. Line 31 is the figure that feeds both Schedule 1 line 3
//! and Schedule SE line 2, so a mis-map there is wrong income AND wrong self-employment tax.

use crate::cells::push_money;
use crate::error::FormsError;
use crate::map::ScheduleCMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::printed::ScheduleCLines;
use btctax_core::Usd;

/// Schedule C has ONE money column, and it is its own: x ≈ [475, 576] (center 525.6).
const SCHEDULE_C_CLUSTERS: &[(f32, f32)] = &[(475.0, 576.0)];
const COL_AMOUNT: usize = 0;

/// Fill Schedule C from the core-derived printed chain. The serialized bytes are read back through
/// the geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_schedule_c_with_map(
    lines: &ScheduleCLines,
    map: &ScheduleCMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    let plan: [Usd; 7] = [
        lines.line1,  // 1  gross receipts
        lines.line3,  // 3  − returns (blank)
        lines.line5,  // 5  gross profit (− COGS, blank)
        lines.line7,  // 7  gross income (+ other income, blank)
        lines.line28, // 28 total expenses (the flat total)
        lines.line29, // 29 tentative profit
        lines.line31, // 31 net profit -> Schedule 1 L3 AND Schedule SE L2
    ];
    for (ord, (cell, value)) in map.lines().iter().zip(plan).enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            COL_AMOUNT,
            Some((0, ord as u32)),
        );
    }

    let mut doc = pdf::load(pdf::schedule_c_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCHEDULE_C_CLUSTERS)?;
    Ok(bytes)
}
