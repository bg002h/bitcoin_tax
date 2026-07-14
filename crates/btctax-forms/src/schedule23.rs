//! Schedule 2 (Additional Taxes) and Schedule 3 (Additional Credits and Payments) fills.
//!
//! **These modules do no tax arithmetic.** Every printed cell is transcribed from
//! [`Schedule2Lines`] / [`Schedule3Lines`], which core derives — including the composition rule that
//! makes the return tie out: Schedule 2 line 11 is Form 8959's **printed** line 18, not a re-rounding
//! of the exact-cents figure.
//!
//! **Schedule 2's descent groups are PER-PAGE.** Line 21 lives on page 2, and a page-2 y-coordinate
//! is not comparable with a page-1 one — a single descent group spanning both pages would compare
//! them and either reject a correct map or, worse, accept a wrong one. `push_money`'s descent key is
//! `(group, ordinal)`, so the page index is the group.

use crate::cells::{page_of, push_identity, push_money};
use crate::error::FormsError;
use crate::map::{Schedule1Map, Schedule2Map, Schedule3Map};
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::{Schedule1Lines, Schedule2Lines, Schedule3Lines};
use btctax_core::Usd;

/// Both schedules share the 1040-family column geometry: MID ≈ [410,482], AMOUNT ≈ [504,576].
/// Code-side oracle — never taken from the (distrusted) map.
const SCH_CLUSTERS: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];
/// The MID column (Schedule 1 line 8v is the only cell v1 writes there).
const COL_MID: usize = 0;
/// The AMOUNT column — every other cell v1 writes on Schedules 1, 2 and 3.
const COL_AMOUNT: usize = 1;

/// Fill Schedule 2 from the core-derived printed chain. The serialized bytes are read back through
/// the geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_schedule_2_with_map(
    lines: &Schedule2Lines,
    header: &ReturnHeader,
    map: &Schedule2Map,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    let plan: [Usd; 4] = [lines.line4, lines.line11, lines.line12, lines.line21];

    // Descent is grouped BY PAGE (line 21 is on page 2). Ordinals restart per page.
    let mut ord_on_page = [0u32; 2];
    for (cell, value) in map.lines().iter().zip(plan) {
        let page = page_of(cell.fields()[0]) as u32;
        let ord = ord_on_page[page as usize];
        ord_on_page[page as usize] += 1;
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            COL_AMOUNT,
            Some((page, ord)),
        );
    }

    let mut doc = pdf::load(pdf::schedule_2_pdf(map.year)?)?;
    // Identity header (P6.2) — the SSN rendering follows the cell's own /MaxLen.
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

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCH_CLUSTERS)?;
    Ok(bytes)
}

/// Fill Schedule 3 from the core-derived printed chain. Single page, so one descent group.
pub fn fill_schedule_3_with_map(
    lines: &Schedule3Lines,
    header: &ReturnHeader,
    map: &Schedule3Map,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    let plan: [Usd; 4] = [lines.line1, lines.line8, lines.line11, lines.line15];
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

    let mut doc = pdf::load(pdf::schedule_3_pdf(map.year)?)?;
    // Identity header (P6.2) — the SSN rendering follows the cell's own /MaxLen.
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

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCH_CLUSTERS)?;
    Ok(bytes)
}

/// Fill Schedule 1 from the core-derived printed chain.
///
/// **Two pages** — Part II (the adjustments) is entirely on page 2, so descent is grouped BY PAGE,
/// exactly as on Schedule 2. Line 8v is the only cell in the MID column; the rest are AMOUNT.
pub fn fill_schedule_1_with_map(
    lines: &Schedule1Lines,
    header: &ReturnHeader,
    map: &Schedule1Map,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    let plan: [(Usd, usize); 10] = [
        (lines.line1, COL_AMOUNT),  // 1  taxable state/local refund
        (lines.line3, COL_AMOUNT),  // 3  business income (Schedule C net)
        (lines.line7, COL_AMOUNT),  // 7  unemployment
        (lines.line8v, COL_MID),    // 8v digital assets as ordinary income  ← the only MID cell
        (lines.line9, COL_AMOUNT),  // 9  total other income
        (lines.line10, COL_AMOUNT), // 10 -> 1040 L8
        (lines.line15, COL_AMOUNT), // 15 deductible part of SE tax      (page 2)
        (lines.line18, COL_AMOUNT), // 18 early-withdrawal penalty       (page 2)
        (lines.line21, COL_AMOUNT), // 21 student-loan interest          (page 2)
        (lines.line26, COL_AMOUNT), // 26 -> 1040 L10                    (page 2)
    ];

    let mut ord_on_page = [0u32; 2];
    for (cell, (value, col)) in map.lines().iter().zip(plan) {
        let page = page_of(cell.fields()[0]) as u32;
        let ord = ord_on_page[page as usize];
        ord_on_page[page as usize] += 1;
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            col,
            Some((page, ord)),
        );
    }

    let mut doc = pdf::load(pdf::schedule_1_pdf(map.year)?)?;
    // Identity header (P6.2) — the SSN rendering follows the cell's own /MaxLen.
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

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCH_CLUSTERS)?;
    Ok(bytes)
}
