//! Schedule D (Capital Gains and Losses) — the **FULL-RETURN** fill.
//!
//! This is not `schedule_d::fill_schedule_d_totals`, and the two must not be merged. That one is the
//! **crypto-slice** fill: it writes lines 3/7/10/15/16 from the ledger's disposal totals and nothing
//! else. For a crypto-only year that is complete and correct, and `export-irs-pdf` still uses it.
//!
//! For a FULL return it is not enough. The crypto slice has **no line 13** (1099-DIV box-2a
//! capital-gain distributions) and **no lines 6/14** (capital-loss carryovers) — yet all three ARE in
//! the computed return's 1040 line 7. A filer handed that form would mail a complete-looking Schedule
//! D with income missing. That is precisely the defect the P5-C1 refusal exists to cover, and this
//! module is what retires it.
//!
//! **★ Three parenthesized boxes: lines 6, 14 and 21.** The form pre-prints the parentheses, so they
//! ARE the minus sign, and the value written must be a POSITIVE MAGNITUDE — a negative renders as a
//! positive number on a filed return. [`assert_paren_magnitudes`] fails closed on one.
//!
//! **★ The amount column differs between the two pages of the same form.** Page 1's column (h) and its
//! standalone cells sit at x = [504, 576] (center 540); page 2's sit at x = [489.6, 576] (center
//! 532.8). One band covering both holds each center — but a band pinned to page 1 alone would reject
//! every page-2 cell.
//!
//! **★ Part III is a decision tree.** Which of lines 17–22 are answered depends on the sign of line 16
//! and the character of line 15 (SPEC §7.2). The routing is decided in core
//! ([`ScheduleDRouting`]) — this module only transcribes the branch it is given, so an impossible
//! combination cannot be filled.

use crate::cells::push_money;
use crate::error::FormsError;
use crate::map::{MoneyCell, ScheduleDMap};
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::printed::{ScheduleDLines, ScheduleDRouting};
use btctax_core::Usd;

/// Logical Schedule D columns for the full fill.
const COL_PROCEEDS_D: usize = 0;
const COL_COST_E: usize = 1;
/// Column (h) AND every standalone amount cell, on BOTH pages — the band spans page 1's [504,576]
/// (center 540) and page 2's [489.6,576] (center 532.8), each of which falls inside it.
const COL_AMOUNT_H: usize = 2;

const SCHEDULE_D_CLUSTERS: &[(f32, f32)] = &[(288.0, 360.0), (360.0, 432.0), (489.0, 576.0)];

/// Descent groups. A row's (d)/(e)/(h) cells share a y, so each column descends in its own group; and
/// a page-2 y is not comparable with a page-1 y, so page 2 gets its own.
const GRP_P1_AMOUNT: u32 = 0;
const GRP_P1_D: u32 = 1;
const GRP_P1_E: u32 = 2;
const GRP_P2_AMOUNT: u32 = 3;

/// Fail closed if a parenthesized cell carries a negative. On the printed form the parentheses supply
/// the minus sign, so a negative would RENDER AS POSITIVE — turning a capital LOSS carryover into a
/// gain. Invisible to any geometric check; hence an explicit guard.
fn assert_paren_magnitudes(lines: &ScheduleDLines) -> Result<(), FormsError> {
    let mut cells = vec![("6", lines.line6), ("14", lines.line14)];
    if let ScheduleDRouting::NetLoss { line21, .. } = lines.routing {
        cells.push(("21", line21));
    }
    for (line, v) in cells {
        if v < Usd::ZERO {
            return Err(FormsError::Geometry(format!(
                "Schedule D line {line} is a PARENTHESIZED box (the form prints the minus sign), so it \
                 must carry a positive magnitude — got {v}. Writing this would render as a POSITIVE \
                 number on the filed return, turning a capital loss into a gain."
            )));
        }
    }
    Ok(())
}

/// A map cell the full fill requires but the crypto-slice maps (2017/2025) do not carry.
fn need<'a, T>(cell: &'a Option<T>, what: &str, year: i32) -> Result<&'a T, FormsError> {
    cell.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            "the TY{year} Schedule D map has no `{what}` — the full-return fill needs it. Full-return \
             v1 is TY2024-only."
        ))
    })
}

/// Fill the FULL-RETURN Schedule D from the core-derived printed chain, including Part III's routing.
/// The serialized bytes are read back through the geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_schedule_d_full_with_map(
    lines: &ScheduleDLines,
    map: &ScheduleDMap,
) -> Result<Vec<u8>, FormsError> {
    assert_paren_magnitudes(lines)?;
    let y = map.year;

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── Page 1, column (h) + the standalone amount cells, top to bottom. ────────────────────────
    // Column (g) (adjustments from Form 8949) is never written — v1 models no basis adjustment.
    let p1_amounts: [(&MoneyCell, Usd); 7] = [
        (&MoneyCell::Single(map.line3.gain_h.clone()), lines.line3_h),
        (need(&map.line6, "line6", y)?, lines.line6), // ★ PAREN
        (&MoneyCell::Single(map.line7_h.clone()), lines.line7),
        (
            &MoneyCell::Single(map.line10.gain_h.clone()),
            lines.line10_h,
        ),
        (need(&map.line13, "line13", y)?, lines.line13),
        (need(&map.line14, "line14", y)?, lines.line14), // ★ PAREN
        (&MoneyCell::Single(map.line15_h.clone()), lines.line15),
    ];
    for (ord, (cell, value)) in p1_amounts.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_AMOUNT_H,
            Some((GRP_P1_AMOUNT, ord as u32)),
        );
    }

    // ── Page 1, columns (d) and (e) — the two 8949 totals rows. ─────────────────────────────────
    for (ord, (fqn, value)) in [
        (&map.line3.proceeds_d, lines.line3_d),
        (&map.line10.proceeds_d, lines.line10_d),
    ]
    .iter()
    .enumerate()
    {
        push_money(
            &mut writes,
            &mut placements,
            &MoneyCell::Single((*fqn).clone()),
            *value,
            COL_PROCEEDS_D,
            Some((GRP_P1_D, ord as u32)),
        );
    }
    for (ord, (fqn, value)) in [
        (&map.line3.cost_e, lines.line3_e),
        (&map.line10.cost_e, lines.line10_e),
    ]
    .iter()
    .enumerate()
    {
        push_money(
            &mut writes,
            &mut placements,
            &MoneyCell::Single((*fqn).clone()),
            *value,
            COL_COST_E,
            Some((GRP_P1_E, ord as u32)),
        );
    }

    // ── Page 2 — line 16, then Part III's routed branch (SPEC §7.2). ────────────────────────────
    let mut p2_ord = 0u32;
    let mut push_p2 = |cell: &MoneyCell,
                       value: Usd,
                       writes: &mut Vec<(String, pdf::FieldValue)>,
                       placements: &mut Vec<FlatPlacement>| {
        push_money(
            writes,
            placements,
            cell,
            value,
            COL_AMOUNT_H,
            Some((GRP_P2_AMOUNT, p2_ord)),
        );
        p2_ord += 1;
    };
    push_p2(
        &MoneyCell::Single(map.line16_h.clone()),
        lines.line16,
        &mut writes,
        &mut placements,
    );

    let check = |pair: &crate::map::YesNoPair,
                 yes: bool,
                 writes: &mut Vec<(String, pdf::FieldValue)>,
                 placements: &mut Vec<FlatPlacement>| {
        let choice = if yes { &pair.yes } else { &pair.no };
        writes.push((
            choice.field.clone(),
            pdf::FieldValue::Check {
                on: choice.on.clone(),
            },
        ));
        placements.push(FlatPlacement::check(choice.field.clone(), 1));
    };

    match lines.routing {
        // L16 > 0 and L15 > 0 — both gains. 17 = Yes; 18 = 19 = 0; 20 = Yes → QDCGT.
        // Lines 21 and 22 are NOT completed: the form says so in terms.
        ScheduleDRouting::BothGains => {
            check(
                need(&map.line17, "line17", y)?,
                true,
                &mut writes,
                &mut placements,
            );
            push_p2(
                need(&map.line18, "line18", y)?,
                Usd::ZERO,
                &mut writes,
                &mut placements,
            );
            push_p2(
                need(&map.line19, "line19", y)?,
                Usd::ZERO,
                &mut writes,
                &mut placements,
            );
            check(
                need(&map.line20, "line20", y)?,
                true,
                &mut writes,
                &mut placements,
            );
        }
        // L16 > 0, L15 ≤ 0 — 17 = No ⇒ skip 18 through 21 ⇒ 22.
        ScheduleDRouting::ShortGainLongLoss { line22_yes } => {
            check(
                need(&map.line17, "line17", y)?,
                false,
                &mut writes,
                &mut placements,
            );
            check(
                need(&map.line22, "line22", y)?,
                line22_yes,
                &mut writes,
                &mut placements,
            );
        }
        // L16 < 0 — skip 17 through 20; line 21 carries the §1211(b) offset (a MAGNITUDE); 22 answered.
        ScheduleDRouting::NetLoss { line21, line22_yes } => {
            push_p2(
                need(&map.line21, "line21", y)?,
                line21,
                &mut writes,
                &mut placements,
            );
            check(
                need(&map.line22, "line22", y)?,
                line22_yes,
                &mut writes,
                &mut placements,
            );
        }
        // L16 = 0 — skip 17 through 21; 22 answered.
        ScheduleDRouting::Zero { line22_yes } => {
            check(
                need(&map.line22, "line22", y)?,
                line22_yes,
                &mut writes,
                &mut placements,
            );
        }
    }

    let mut doc = pdf::load(pdf::schedule_d_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    let doc2 = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&doc2)?;
    verify_flat(&doc2, &fields, &placements, SCHEDULE_D_CLUSTERS)?;
    Ok(bytes)
}
