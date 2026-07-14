//! **Full-return** Schedule SE fill — the WHOLE-DOLLAR path (P6.3a / ARCH-P6.3a D5/D6).
//!
//! Parallel to [`crate::schedule_se`], and deliberately NOT a refactor of it. The two are under
//! different rounding regimes and must stay that way:
//!
//! - the **crypto slice** prints exact CENTS (deliberately CSV-identical, and a crypto-only filer may
//!   legitimately file in cents);
//! - the **full return** has elected round-all-amounts (§3.1), so every filed form — including this one
//!   — prints whole dollars and cross-foots.
//!
//! A unified filler, or a "rounding mode" flag, would put both regimes behind one entry point and invite
//! a future harmonization that must never happen. This module therefore does what the other full-return
//! fillers do: it transcribes a core-derived printed chain ([`ScheduleSeLines`]) cell for cell, and does
//! **zero arithmetic**.
//!
//! ★ **The identity is the PROPRIETOR's, not the return's.** Schedule SE's header is "Name of person
//! **with self-employment income**" — on a joint return with a spouse-owned business that is the SPOUSE,
//! with the SPOUSE's SSN. Core decides who that is ([`ReturnHeader::proprietor`]); this module only
//! transcribes. Writing the joint name line here would file the SE tax under the wrong person.

use crate::cells::{push_identity, push_money};
use crate::error::FormsError;
use crate::map::ScheduleSeMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::ScheduleSeLines;
use btctax_core::Usd;

/// Logical Schedule SE columns: 0 = MID, 1 = AMOUNT (the same clusters the slice uses for 2024).
const SE_COL_MID: usize = 0;
const SE_COL_AMOUNT: usize = 1;

/// Hand-pinned column-x clusters for the TY2024 form — the code-side geometric ORACLE, never read from
/// the (distrusted) map.
const SE_CLUSTERS: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];

/// Fill the full-return Schedule SE from the core-derived printed chain.
///
/// Lines 8d/9/10 are SKIPPED when the earner's own W-2 Social Security wages already meet or exceed the
/// wage base — the form's line 9 says "if zero or less, enter -0- here and **go to line 11**", and the
/// §1402(b)(1) cap has then consumed the whole SS band (line 10 is zero by construction).
pub fn fill_schedule_se_full_with_map(
    lines: &ScheduleSeLines,
    header: &ReturnHeader,
    map: &ScheduleSeMap,
) -> Result<Vec<u8>, FormsError> {
    let mut doc = pdf::load(pdf::schedule_se_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ★ The PROPRIETOR — "Name of person with self-employment income" — not the joint name line.
    let identity = map.identity.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            "the {} Schedule SE map has no [identity] block — a full return cannot file an unnamed \
             Schedule SE",
            map.year
        ))
    })?;
    let proprietor = header.proprietor.as_ref().ok_or_else(|| {
        FormsError::Geometry(
            "Schedule SE has no proprietor — the return names no one with self-employment income"
                .into(),
        )
    })?;
    push_identity(
        &mut writes,
        &mut placements,
        identity,
        &proprietor.full_name(),
        &proprietor.ssn,
        &blank_fields,
    )?;

    // The SS band is exhausted when W-2 SS wages already reach the base ⇒ lines 8d/9/10 are not filled.
    let skip_8d_to_10 = lines.line9 == Usd::ZERO;

    // Parallel to `map.lines()` — printed reading order, strictly descending y.
    let plan: [(Usd, usize, bool); 12] = [
        (lines.line2, SE_COL_AMOUNT, true), // 2  ← Schedule C's printed line 31
        (lines.line3, SE_COL_AMOUNT, true), // 3
        (lines.line4a, SE_COL_AMOUNT, true), // 4a
        (lines.line4c, SE_COL_AMOUNT, true), // 4c
        (lines.line6, SE_COL_AMOUNT, true), // 6  ← cited by Form 8959 line 8
        (lines.line8a, SE_COL_MID, true),   // 8a
        (lines.line8d, SE_COL_AMOUNT, !skip_8d_to_10), // 8d
        (lines.line9, SE_COL_AMOUNT, !skip_8d_to_10), // 9
        (lines.line10, SE_COL_AMOUNT, !skip_8d_to_10), // 10
        (lines.line11, SE_COL_AMOUNT, true), // 11
        (lines.line12, SE_COL_AMOUNT, true), // 12 → Schedule 2 line 4
        (lines.line13, SE_COL_MID, true),   // 13 → Schedule 1 line 15
    ];
    for (ord, (cell, (value, col, include))) in map.lines().iter().zip(plan).enumerate() {
        if !include {
            continue;
        }
        push_money(
            &mut writes,
            &mut placements,
            cell,
            value,
            col,
            Some((0, ord as u32)),
        );
    }

    // Factory-prefilled constants on the blank (e.g. the pre-printed wage base) are authorized, never
    // written — so the blank's own `/V` values do not trip the no-unmapped check.
    for fqn in &map.prefilled_exempt {
        placements.push(FlatPlacement::free(fqn.clone(), crate::cells::page_of(fqn)));
    }

    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output against the blank PDF's own widget rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SE_CLUSTERS)?;
    Ok(bytes)
}
