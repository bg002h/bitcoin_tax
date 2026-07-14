//! Schedule B (Interest and Ordinary Dividends) fill — a *listing* schedule.
//!
//! **This module does no tax arithmetic.** The rows and totals come from [`ScheduleBLines`], and the
//! totals sum the PRINTED row amounts, so the form cross-foots against its own list.
//!
//! **★ Its amount column is x ≈ [489.6, 576]** — not the [504, 576] of Schedules 1/2/3/A, Schedule SE
//! and Forms 8959/8960/8995, and not Schedule C's [475, 576]. A shared amount-column constant would
//! reject every Schedule B cell.
//!
//! **★ Two descent groups, not one.** A row's payer cell and its amount cell sit at the SAME y, so a
//! single descent sequence covering both columns would compare them and fail the strict-decrease
//! check on every row. Amounts descend in group 0; payer names descend in group 1.
//!
//! **★ Overflow FAILS CLOSED.** Part I holds 14 payers and Part II holds 15 (the asymmetry is real).
//! More payers than rows is refused, never truncated: a truncated list would still be totalled from
//! the full sum, so the printed form would not add up — and if it were totalled from the visible rows
//! instead, the return would understate income.
//!
//! **★ Part III is transcribed, never decided.** Lines 7a and 8 carry the filer's own declared
//! answers (`screen_inputs` refuses the return if they were left unanswered). The unnumbered FBAR
//! sub-question under 7a and line 7b's country list are left BLANK — v1 has no input for them, and
//! the `FbarFinCen` advisory tells the filer in terms that they must decide it themselves. An
//! incomplete Part III is the honest output here; a guessed one would not be.

use crate::cells::{push_identity, push_money};
use crate::error::FormsError;
use crate::map::ScheduleBMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::ScheduleBLines;

/// Logical Schedule B columns.
const COL_PAYER: usize = 0;
const COL_AMOUNT: usize = 1;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF. Code-side oracle. The PAYER
/// band spans Part II's row-1 box too — it is indented to x0 = 201.6 rather than 129.6, but its
/// center still falls inside, so it needs no separate column.
const SCHEDULE_B_CLUSTERS: &[(f32, f32)] = &[(129.0, 461.0), (489.0, 576.0)];

/// Descent groups: a row's payer and amount share a y, so they cannot descend together.
const GRP_AMOUNTS: u32 = 0;
const GRP_PAYERS: u32 = 1;

/// Fill Schedule B from the core-derived printed chain. The serialized bytes are read back through
/// the geometric verifier (a mis-mapped cell FAILS CLOSED).
///
/// Refuses when there are more payers than the form has rows — see the module note on why truncating
/// is not an option.
pub fn fill_schedule_b_with_map(
    lines: &ScheduleBLines,
    header: &ReturnHeader,
    map: &ScheduleBMap,
) -> Result<Vec<u8>, FormsError> {
    // A CAPACITY refusal, not a placement failure (`p6-schedule-b-capacity-error-variant`): every cell is
    // mapped correctly — there are simply more payers than the form has rows. Typing it as `Overflow`
    // lets the CLI render "file Schedule B by hand" actionably, and lets the all-or-nothing packet say
    // WHICH form refused. (Truncating is not an option: the printed rows would not add up to the form's
    // own line 2, or the total would be taken from the visible rows and UNDERSTATE interest income.)
    if lines.part1_rows.len() > map.part1_rows.len() {
        return Err(FormsError::Overflow {
            part: "Schedule B Part I",
            rows: lines.part1_rows.len(),
            capacity: map.part1_rows.len(),
        });
    }
    if lines.part2_rows.len() > map.part2_rows.len() {
        return Err(FormsError::Overflow {
            part: "Schedule B Part II",
            rows: lines.part2_rows.len(),
            capacity: map.part2_rows.len(),
        });
    }

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();
    let (mut amt_ord, mut payer_ord) = (0u32, 0u32);

    // ── Part I — the interest payers, then lines 2 and 4. ───────────────────────────────────────
    for (row, cell) in lines.part1_rows.iter().zip(map.part1_rows.iter()) {
        writes.push((cell.payer.clone(), pdf::FieldValue::Text(row.payer.clone())));
        placements.push(FlatPlacement::cell(
            cell.payer.clone(),
            0,
            COL_PAYER,
            GRP_PAYERS,
            payer_ord,
        ));
        payer_ord += 1;

        push_money(
            &mut writes,
            &mut placements,
            &cell.amount,
            row.amount,
            COL_AMOUNT,
            Some((GRP_AMOUNTS, amt_ord)),
        );
        amt_ord += 1;
    }
    for (cell, value) in [(&map.line2, lines.line2), (&map.line4, lines.line4)] {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            COL_AMOUNT,
            Some((GRP_AMOUNTS, amt_ord)),
        );
        amt_ord += 1;
    }

    // ── Part II — the dividend payers, then line 6. ─────────────────────────────────────────────
    for (row, cell) in lines.part2_rows.iter().zip(map.part2_rows.iter()) {
        writes.push((cell.payer.clone(), pdf::FieldValue::Text(row.payer.clone())));
        placements.push(FlatPlacement::cell(
            cell.payer.clone(),
            0,
            COL_PAYER,
            GRP_PAYERS,
            payer_ord,
        ));
        payer_ord += 1;

        push_money(
            &mut writes,
            &mut placements,
            &cell.amount,
            row.amount,
            COL_AMOUNT,
            Some((GRP_AMOUNTS, amt_ord)),
        );
        amt_ord += 1;
    }
    push_money(
        &mut writes,
        &mut placements,
        &map.line6,
        lines.line6,
        COL_AMOUNT,
        Some((GRP_AMOUNTS, amt_ord)),
    );

    // ── Part III — the filer's OWN answers, transcribed. ────────────────────────────────────────
    for (pair, answer) in [
        (&map.line7a, lines.foreign_accounts_7a),
        (&map.line8, lines.foreign_trust_8),
    ] {
        let choice = if answer { &pair.yes } else { &pair.no };
        writes.push((
            choice.field.clone(),
            pdf::FieldValue::Check {
                on: choice.on.clone(),
            },
        ));
        placements.push(FlatPlacement::check(choice.field.clone(), 0));
    }

    let mut doc = pdf::load(pdf::schedule_b_pdf(map.year)?)?;
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

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCHEDULE_B_CLUSTERS)?;
    Ok(bytes)
}
