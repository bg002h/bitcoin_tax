//! **Form 8889 (Health Savings Accounts) fill** — the §223 line chain, read back through the
//! flat-form geometric oracle (column-x cluster + ordinal-y descent + no-unmapped). T16 / FR-76.
//!
//! **This module does no tax arithmetic.** Every printed cell is transcribed from
//! [`btctax_core::tax::form8889::Form8889`], which core derives once in `assemble_absolute` and
//! which Schedule 1 lines 8f/13 and Schedule 2 lines 17c/17d read from as well. A second,
//! independent derivation here is exactly how a filed PDF comes to disagree with the tax it
//! reports — so there isn't one.
//!
//! ★★ **Skip rule: there is none, and that is the point.** The form files exactly when the §223
//! trigger declaration is affirmed ([`btctax_core::tax::form8889::Form8889::must_file`]), which is
//! the question the filer answered — never a threshold over the figures. A filer whose
//! contributions exactly equalled their employer's has a $0 line 13 and still owes the IRS the
//! form; a `must_file` computed from the numbers would drop it, and the packet would carry a
//! Schedule 1 line 13 with no attachment behind it.
//!
//! ★ **Every line prints.** Unlike Form 8959 (whose Part III is unmodelled RRTA) this form has no
//! unmapped cell: btctax fills Parts I, II and III completely, and every situation the form routes
//! to paper btctax does not carry — the Line 3 Limitation Chart, Form 8853, Form 5329, a second
//! spouse's Form 8889, a prior year's worksheet — REFUSES the return upstream instead.

use crate::cells::{push_identity, push_money};
use crate::error::FormsError;
use crate::map::Form8889Map;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::form8889::Form8889;
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::return_inputs::HdhpCoverage;
use btctax_core::Usd;

/// Logical Form 8889 columns: col 0 = MID, col 1 = AMOUNT.
const F8889_COL_MID: usize = 0;
const F8889_COL_AMOUNT: usize = 1;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF with
/// `xtask dump-fields`. This is the geometry ORACLE — deliberately code-side, NEVER read from the
/// (distrusted) map, so a map that points a line at the wrong column fails closed instead of
/// quietly printing a number in the wrong place.
///
/// ★ Only lines 9 and 10 sit in the MID cluster: the form insets them so line 11 can add them in
/// the amount column, exactly as Schedule SE insets 8a–8c for 8d.
const F8889_CLUSTERS: &[(f32, f32)] = &[(410.4, 481.6), (504.0, 576.0)];

/// Fill Form 8889 for `year` from the core-derived line chain, using the bundled map.
///
/// Returns the serialized PDF bytes, read back through the geometric verifier (a mis-mapped cell
/// FAILS CLOSED). There is no `Ok(None)` arm: the caller decides whether the form files, from
/// [`Form8889::must_file`], which reads the filer's own declaration.
pub fn fill_form_8889(
    lines: &Form8889,
    header: &ReturnHeader,
    year: i32,
) -> Result<Vec<u8>, FormsError> {
    let map = Form8889Map::for_year(year)?;
    fill_form_8889_with_map(lines, header, &map)
}

/// [`fill_form_8889`] against an explicit map — the seam the tests drive.
pub fn fill_form_8889_with_map(
    lines: &Form8889,
    header: &ReturnHeader,
    map: &Form8889Map,
) -> Result<Vec<u8>, FormsError> {
    // Load FIRST: `push_identity` reads each SSN cell's own /MaxLen to decide hyphenated-vs-digits.
    let mut doc = pdf::load(pdf::f8889_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;

    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    push_identity(
        &mut writes,
        &mut placements,
        &map.identity,
        &header.name_line,
        &header.taxpayer.ssn,
        &blank_fields,
    )?;

    // ★★★ THE THREE CHECKBOXES. Line 1's pair is MUTUALLY EXCLUSIVE and exactly one is checked —
    //     the form asks which coverage, and leaving both blank on a filed Form 8889 would be a
    //     line the IRS reads as unanswered on a form the filer signed. Line 17a is checked iff
    //     some part of line 16 met an exception, which is the form's own sentence.
    for (choice, on) in [
        (
            &map.check_1_self_only,
            lines.line1_coverage == HdhpCoverage::SelfOnly,
        ),
        (
            &map.check_1_family,
            lines.line1_coverage == HdhpCoverage::Family,
        ),
        (&map.check_17a, lines.line17a_exception_box),
    ] {
        if on {
            writes.push((
                choice.field.clone(),
                pdf::FieldValue::Check {
                    on: choice.on.clone(),
                },
            ));
            placements.push(FlatPlacement::check(
                choice.field.clone(),
                crate::cells::page_of(&choice.field),
            ));
        }
    }

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 22] = [
        (lines.line2, F8889_COL_AMOUNT),   // 2
        (lines.line3, F8889_COL_AMOUNT),   // 3
        (lines.line4, F8889_COL_AMOUNT),   // 4
        (lines.line5, F8889_COL_AMOUNT),   // 5
        (lines.line6, F8889_COL_AMOUNT),   // 6
        (lines.line7, F8889_COL_AMOUNT),   // 7
        (lines.line8, F8889_COL_AMOUNT),   // 8
        (lines.line9, F8889_COL_MID),      // 9  ← inset
        (lines.line10, F8889_COL_MID),     // 10 ← inset
        (lines.line11, F8889_COL_AMOUNT),  // 11
        (lines.line12, F8889_COL_AMOUNT),  // 12
        (lines.line13, F8889_COL_AMOUNT),  // 13 → Schedule 1 line 13
        (lines.line14a, F8889_COL_AMOUNT), // 14a
        (lines.line14b, F8889_COL_AMOUNT), // 14b
        (lines.line14c, F8889_COL_AMOUNT), // 14c
        (lines.line15, F8889_COL_AMOUNT),  // 15
        (lines.line16, F8889_COL_AMOUNT),  // 16 → Schedule 1 line 8f
        (lines.line17b, F8889_COL_AMOUNT), // 17b → Schedule 2 line 17c
        (lines.line18, F8889_COL_AMOUNT),  // 18
        (lines.line19, F8889_COL_AMOUNT),  // 19
        (lines.line20, F8889_COL_AMOUNT),  // 20 → Schedule 1 line 8f
        (lines.line21, F8889_COL_AMOUNT),  // 21 → Schedule 2 line 17d
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

    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // True read-back: re-parse the SERIALIZED output and verify geometry against the PDF's own rects.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F8889_CLUSTERS)?;
    Ok(bytes)
}
