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

use crate::cells::{push_identity, push_money};
use crate::error::FormsError;
use crate::map::ScheduleAMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::ScheduleALines;
use btctax_core::Usd;

/// Logical Schedule A columns.
const COL_AGI_INLINE: usize = 0;
const COL_MID: usize = 1;
const COL_AMOUNT: usize = 2;

/// Hand-pinned column-x clusters, measured from the blank TY2024 PDF. Code-side oracle — never taken
/// from the (distrusted) map. The AGI-inline band is deliberately tight: it is the whole reason a
/// mis-mapped line 2 fails closed instead of silently printing the AGI in the wrong box.
const SCHEDULE_A_CLUSTERS: &[(f32, f32)] = &[
    (331.0, 403.0),
    (417.0, 489.0),
    (504.0, 576.0),
    (245.0, 266.0),
];

/// ★★★ T9 — line 8b's DOTTED LINES, a fourth cluster far to the LEFT of every money column.
///
/// Measured with `xtask dump-fields` on the bundled blanks rather than read off the render:
/// TY2024's two rows are `f1_17` @ y=348 and `f1_18` @ y=336, both x = [115.2, 396.0] (x-centre
/// 255.6); TY2025's one merged 24pt box is `Line8b_ReadOrder[0].f1_16[0]` at x = [122.4, 381.6]
/// (x-centre 252.0). `verify_flat` bands the x-CENTRE, so the cluster is [245, 266] — well clear of
/// the AGI-inline band at [331, 403], which is the nearest money column to its right.
const COL_PAYEE: usize = 3;

/// The descent group the dotted lines belong to. The money cells are group 0 (one strictly
/// descending chain); the payee rows are their own chain, because they sit BETWEEN two money cells
/// in y and would otherwise break the money chain's monotonicity.
const GRP_PAYEE: u32 = 1;

/// The Schedule A instruction's own words for a line 8b block that does not fit on the form:
/// *"identify the person by attaching a statement to your paper return and printing "See attached"
/// to the right of line 8b."* (`design/forms/extract/i1040sca--2025.txt:1126-1131`.)
const SEE_ATTACHED: &str = "See attached";

/// ★★★ **THE LINE 8b RECIPIENTS THE YEAR'S FORM CANNOT PRINT — the ONE decision, read twice.**
///
/// Empty in the ordinary case (every recipient fits, and the form carries their identities). When it
/// is **not** empty the emitter prints [`SEE_ATTACHED`] instead, and *every* recipient's identity is
/// off the page — not just the ones past the last dotted line — because the escape replaces the
/// whole block with one string. So this returns them **all**: they are all on the statement.
///
/// ★★ **Why it is a function and not an inline `len()` comparison** (the T9 seam review's I-1). The
///    printed *"See attached"* asserts, on a page the filer signs under §6065, that a statement is
///    attached — and T9 shipped it with nothing anywhere producing, requesting or naming that
///    statement. The packet manifest's *"COMPLETE BY HAND"* block, which frames itself as the closed
///    list of marks btctax deliberately did not make, has to name it; and the manifest's condition
///    and the emitter's condition must be the SAME condition, or the packet can print the assertion
///    while the manifest stays silent. `btctax_forms::schedule_a_line8b_overflow` is the manifest's
///    door onto this function.
///
/// ★ `map.line8b_payee` is empty only for a year whose map does not carry the cells at all; nothing
///   is printed then, so nothing overflows.
pub(crate) fn line8b_overflow<'a>(lines: &'a ScheduleALines, map: &ScheduleAMap) -> &'a [String] {
    if map.line8b_payee.is_empty() || lines.line8b_payee.len() <= map.line8b_payee.len() {
        &[]
    } else {
        &lines.line8b_payee
    }
}

/// Fill Schedule A from the core-derived printed chain. The serialized bytes are read back through
/// the geometric verifier (a mis-mapped cell FAILS CLOSED).
pub fn fill_schedule_a_with_map(
    lines: &ScheduleALines,
    header: &ReturnHeader,
    map: &ScheduleAMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Parallel to `map.lines()` — printed reading order, strictly descending y on page 1.
    let plan: [(Usd, usize); 21] = [
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
        (lines.line8b, COL_MID),       // 8b mortgage interest NOT on a Form 1098
        (lines.line8c, COL_MID),       // 8c points not on a Form 1098
        (lines.line8e, COL_MID),       // 8e add 8a-8c
        (lines.line9, COL_MID),        // 9  investment interest (§163(d))
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

    // ★★★ T9 / R8 — LINE 8b's DOTTED LINES. The instruction makes the identity part of the line:
    //     *"write that person's name, identifying number, and address on the dotted lines next to
    //     line 8b"*, with its own Caution — *"If you don't show the required information about the
    //     recipient … you may have to pay a $50 penalty."* So a printed 8b amount without this
    //     block is an incomplete line, not a tidier one.
    //
    // ★★ When there are MORE recipients than the form has dotted lines, the escape is the
    //    instruction's own, printed on this very line: *"identify the person by attaching a
    //    statement to your paper return and printing \u{201c}See attached\u{201d} to the right of line 8b."*
    //    The emitter chooses between two strings the form supplies; it composes neither.
    if !lines.line8b_payee.is_empty() && !map.line8b_payee.is_empty() {
        // ★ ONE condition, shared with the packet manifest — see `line8b_overflow`.
        let printed: Vec<String> = if line8b_overflow(lines, map).is_empty() {
            lines.line8b_payee.clone()
        } else {
            vec![SEE_ATTACHED.to_string()]
        };
        for (ord, (fqn, text)) in map.line8b_payee.iter().zip(printed).enumerate() {
            writes.push((fqn.clone(), pdf::FieldValue::Text(text)));
            placements.push(FlatPlacement::cell(
                fqn.clone(),
                crate::cells::page_of(fqn),
                COL_PAYEE,
                GRP_PAYEE,
                ord as u32,
            ));
        }
    }

    let mut doc = pdf::load(pdf::schedule_a_pdf(map.year)?)?;
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
    // ★ The two ELECTIONS core already honoured but the form never showed (ARCH-P6.3a Q7):
    // §164(b)(5) sales-tax-instead-of-income-tax, and §63(e) itemize-below-the-standard-deduction.
    // Without the latter the Service's math-error unit may "correct" the return to the standard.
    // ★ P9 §2.7: the line-8 §163(h)(3)(F) mixed-use-mortgage DECLARATION — core zeroed 8a; without this
    // box a $0 line 8a beside an unchecked box is an unaffirmed statement.
    for (choice, on) in [
        (&map.check_5a_sales_tax, lines.line5a_is_sales_tax),
        (&map.check_8_mixed_use, lines.line8_mixed_use_box),
        (&map.check_18_elects_smaller, lines.line18_elects_smaller),
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

    let index = pdf::index(&blank_fields);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, SCHEDULE_A_CLUSTERS)?;
    Ok(bytes)
}
