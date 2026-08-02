//! §G-28/B1a — fill **Form 8995-A Part IV**.
//!
//! Above the §199A(e)(2) threshold the simplified Form 8995 no longer applies, so a filer there must
//! file this form even when the arithmetic is identical. i8995a scopes the case:
//!
//! > *"You must complete Part I if you have QBI from a qualified trade, business, or aggregation. **If
//! > you don't have QBI, and only have REIT, PTP, skip Parts I through III and complete Part IV.**"*
//!
//! ★★★ **This emitter and the refusal narrowing are ONE unit.** Relaxing `QbiAboveThreshold` without
//! this would print the SIMPLIFIED Form 8995 for a filer the instructions require to use 8995-A — a
//! wrong form on a filed return, which is worse than the refusal it replaced.

use crate::cells::{push_identity, push_money, push_money_opt};
use crate::error::FormsError;
use crate::map::Form8995AMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::qbi_a::Form8995APartIv;
use btctax_core::Usd;

/// Logical columns, matching the map's header: MID x ≈ [410,482], AMOUNT x ≈ [504,576].
const COL_MID: usize = 0;
const COL_AMOUNT: usize = 1;
const F8995A_CLUSTERS: &[(f32, f32)] = &[(410.0, 482.0), (504.0, 576.0)];

/// The descent group for Part IV — one column-agnostic run down page 2.
const GRP_PART_IV: u32 = 0;

/// ★★ The parenthesized boxes hold a POSITIVE MAGNITUDE. The form prints the minus sign, so writing
/// `-1234` renders as `(-1,234)` — a positive number on a filed return. `Form8995APartIv` sources both
/// from `Form8995Lines`, which already guarantees magnitudes, and this fails closed if that ever slips.
fn assert_paren_magnitudes(p: &Form8995APartIv) -> Result<(), FormsError> {
    for (line, v) in [("29", p.line29), ("40", p.line40)] {
        if v < Usd::ZERO {
            return Err(FormsError::Geometry(format!(
                "Form 8995-A line {line} is a PARENTHESIZED box holding {v}: the form's own \
                 parentheses supply the minus sign, so a negative value renders as a positive number \
                 on a filed return. Store the magnitude."
            )));
        }
    }
    Ok(())
}

/// Fill Form 8995-A (Part IV) and read the result back geometrically.
pub fn fill_form_8995a_with_map(
    p: &Form8995APartIv,
    header: &ReturnHeader,
    map: &Form8995AMap,
) -> Result<Vec<u8>, FormsError> {
    assert_paren_magnitudes(p)?;

    let mut doc = pdf::load(pdf::f8995a_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // Part IV, top to bottom. The column each line prints in is the map's own documented partition —
    // seven MID, seven AMOUNT — and `verify_flat` reads it back against the PDF's rects.
    let plan: [(&crate::map::MoneyCell, Usd, usize); 13] = [
        (&map.line27, p.line27, COL_MID),
        (&map.line28, p.line28, COL_MID),
        (&map.line29, p.line29, COL_MID), // ★ paren — magnitude
        (&map.line30, p.line30, COL_MID),
        (&map.line31, p.line31, COL_MID),
        (&map.line32, p.line32, COL_AMOUNT),
        (&map.line33, p.line33, COL_MID),
        (&map.line34, p.line34, COL_MID),
        (&map.line35, p.line35, COL_AMOUNT),
        (&map.line36, p.line36, COL_AMOUNT),
        (&map.line37, p.line37, COL_AMOUNT),
        (&map.line39, p.line39, COL_AMOUNT),
        (&map.line40, p.line40, COL_AMOUNT), // ★ paren — magnitude
    ];
    for (ord, (cell, value, col)) in plan.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            *col,
            Some((GRP_PART_IV, ord as u32)),
        );
    }

    // ★★★ LINE 38 (DPAD) IS LEFT BLANK, and `push_money_opt` is how that is expressible. Its own text
    // is *"DPAD under section 199A(g) allocated from an agricultural or horticultural cooperative.
    // Don't enter more than line 33 minus line 37"* — a CONDITIONAL entry with no `-0-` clause, and
    // btctax fills no Schedule D (Form 8995-A), so no cooperative has allocated anything. A printed
    // `0` would swear the filer received an allocation of zero. It has no descent ordinal because it
    // writes nothing; line 39 sits below it either way.
    push_money_opt(
        &mut writes,
        &mut placements,
        &map.line38,
        p.line38,
        COL_AMOUNT,
        None,
    );

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
    verify_flat(&check, &fields, &placements, F8995A_CLUSTERS)?;
    Ok(bytes)
}
