//! §G-6 — fill **Form 6251** (Alternative Minimum Tax—Individuals).
//!
//! ## Why this exists
//!
//! btctax has COMPUTED Form 6251 since v0.14.0 but could not FILE it, so any return where the form
//! must be attached was refused (`RefuseReason::AmtScreenTriggered`). i6251 p.1, *Who Must File*,
//! condition 1: *"Form 6251, line 7, is greater than line 10."* That is the attach test — **not**
//! `amt > 0`, because when line 7 exceeds line 10 the AMT foreign tax credit is figured, so the AMT
//! can be $0 while the form is still required.
//!
//! ## ★★★ TWO SIGN CONVENTIONS, AND THEY ARE NOT INTERCHANGEABLE
//!
//! | lines | the form prints | so the emitter writes |
//! |---|---|---|
//! | **2b, 2f, 2s** | a literal `(   )` pair | a POSITIVE MAGNITUDE — the parentheses supply the minus |
//! | 2c, 2d, 2k, 2l, 2n–2r, 3 | nothing | a LITERAL MINUS, straight from the stored value |
//! | 1 | nothing | likewise; line 1 may legitimately be negative |
//!
//! Every figure written here is [`Form6251::printed`] — **whole dollars per line** (SPEC §3.1),
//! rounded once at this boundary and never in the computation, so Form 6251 line 11 and Schedule 2
//! line 2 agree by construction.
//!
//! `Form6251::line2b` is stored **negative** — i6251 line 2b: *"Enter the total as a negative
//! amount"* — so writing it raw renders `(-500)` on the page, which reads as a positive number on a
//! filed return. [`assert_paren_magnitudes`] fails closed if a magnitude ever arrives signed.
//!
//! ## ★★★ PART III IS FILED AS A UNIT, OR NOT AT ALL
//!
//! The form's own gate: *"Complete Part III only if you are required to do so by line 7 or by the
//! Foreign Earned Income Tax Worksheet in the instructions."* Lines 12–40 are ordinary `Usd` in core
//! and are zero on the un-routed path, so writing them unconditionally would file **twenty-nine sworn
//! zeros** on a page the form told the filer not to complete. `Form6251::part_iii_completed` carries
//! the gate, and nothing here writes Part III without it. "Skip" is not an instruction to enter zero —
//! this form says *"enter -0-"* elsewhere when that is what it means.
//!
//! ## What stays out
//!
//! Part I lines **2c–2t** are not modelled by core and are CENSUSED as `gap`, not `unmodeled`. Every
//! one is an ADD-BACK, so a laundered zero understates tax rather than forgoing a benefit. They are
//! reachable only through the §G-22 out-of-scope refusal, whose limb (b) names an ISO exercise
//! explicitly — necessary because an ISO exercise is not *income* for the regular tax, so the income
//! half of that question could never have caught it.

use crate::cells::{push_identity, push_money};
use crate::error::FormsError;
use crate::map::{Form6251Map, MoneyCell};
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::form6251::Form6251;
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::Usd;

/// The one AMOUNT column, at x ≈ [504,576]. The three parenthesised boxes are INSET to [508,572], so
/// they share the column's centre (540.0) and band with it.
const COL_AMOUNT: usize = 0;
const F6251_CLUSTERS: &[(f32, f32)] = &[(504.0, 576.0)];

/// Page 1 (Parts I and II) — one descent run.
const GRP_P1: u32 = 0;
/// Page 2 (Part III) — its own run; y resets at the page break, so a single group would be
/// satisfiable by a wrong ordering across it.
const GRP_P2: u32 = 1;

/// ★★ The three parenthesised boxes hold a POSITIVE MAGNITUDE. The form prints the minus sign, so a
/// stored `-500` would render as `(-500)` — a positive number on a filed return. Core stores line 2b
/// negative on purpose (i6251: *"Enter the total as a negative amount"*), so the conversion happens
/// here, once, and this fails closed if a magnitude ever arrives signed anyway.
fn assert_paren_magnitudes(v: &[(&str, Usd)]) -> Result<(), FormsError> {
    for (line, x) in v {
        if *x < Usd::ZERO {
            return Err(FormsError::Geometry(format!(
                "Form 6251 line {line} is a PARENTHESIZED box holding {x}: the form's own parentheses \
                 supply the minus sign, so a negative value renders as a positive number on a filed \
                 return. Pass the magnitude."
            )));
        }
    }
    Ok(())
}

/// Fill Form 6251 and read the result back geometrically.
pub fn fill_form_6251_with_map(
    f: &Form6251,
    header: &ReturnHeader,
    map: &Form6251Map,
) -> Result<Vec<u8>, FormsError> {
    // ★★★ WHOLE DOLLARS, at the very top, so nothing below can reach a cell unrounded (SPEC §3.1).
    //     Core computes in cents; a filed Form 6251 reading `471037.2840` is not a return. `printed()`
    //     destructures exhaustively, so a line added to the form cannot slip past this.
    let f = &f.printed();

    // ★ The three magnitudes, converted ONCE here. `line2b` is the only one core stores signed today;
    //   2f and 2s have no core field yet (they are inside the censused 2c-2t range) and are therefore
    //   never written — but the conversion is expressed for all three so adding them cannot forget it.
    let l2b = f.line2b.abs();
    assert_paren_magnitudes(&[("2b", l2b)])?;

    // ★★★ LINE 1 IS YEAR-SHAPED, and the TY2024 map has ONE cell for it. TY2025 splits it into 1a/1b
    //     (60 boxes, and line 4 becomes "combine lines 1b through 3"), so a TY2025 chain cannot be
    //     written through this map at all — and must say so rather than silently drop 1a.
    let line1 = match f.line1 {
        btctax_core::tax::form6251::Form6251Line1::Y2024 { line1 } => line1,
        // ★ `#[non_exhaustive]`, so the catch-all is required and is also the right behaviour: any
        //   future year shape this TY2024 map has no cells for must REFUSE, never silently drop a
        //   sub-line.
        _ => {
            return Err(FormsError::Geometry(
                "Form 6251 line 1 is not in its TY2024 shape, but this is the TY2024 map, which \
                 prints a single line 1. The 2025 revision splits it into 1a/1b (60 numbered boxes, \
                 and a line 4 that combines lines 1b through 3) — that needs its own map, not this \
                 one, and filling this one would drop a sub-line off a filed form."
                    .into(),
            ));
        }
    };

    let mut doc = pdf::load(pdf::f6251_pdf(map.year)?)?;
    let blank_fields = pdf::collect_fields(&doc)?;
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── Parts I and II, top to bottom. ──────────────────────────────────────────────────────────
    //
    // ★ The descent ordinal is the field's POSITION IN THIS LIST, and the list is in the form's own
    //   printed order — so `verify_flat` reads the filled PDF back and rejects any cell whose y does
    //   not descend with its neighbours. Lines 2c-2t are absent from the list because core does not
    //   model them; their widgets stay blank and are censused.
    let p1: [(&MoneyCell, Usd); 12] = [
        (&map.line1, line1),
        (&map.line2a, f.line2a),
        (&map.line2b, l2b), // ★ PARENTHESISED — magnitude
        (&map.line3, f.line3),
        (&map.line4, f.line4),
        (&map.line5, f.line5),
        (&map.line6, f.line6),
        (&map.line7, f.line7),
        (&map.line8, f.line8),
        (&map.line9, f.line9),
        (&map.line10, f.line10),
        (&map.line11, f.line11),
    ];
    for (ord, (cell, value)) in p1.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_AMOUNT,
            Some((GRP_P1, ord as u32)),
        );
    }

    // ── Part III — as a UNIT, and only when the form's own gate says to complete it. ─────────────
    if f.part_iii_completed {
        let p3: [(&MoneyCell, Usd); 29] = [
            (&map.line12, f.line12),
            (&map.line13, f.line13),
            (&map.line14, f.line14),
            (&map.line15, f.line15),
            (&map.line16, f.line16),
            (&map.line17, f.line17),
            (&map.line18, f.line18),
            (&map.line19, f.line19),
            (&map.line20, f.line20),
            (&map.line21, f.line21),
            (&map.line22, f.line22),
            (&map.line23, f.line23),
            (&map.line24, f.line24),
            (&map.line25, f.line25),
            (&map.line26, f.line26),
            (&map.line27, f.line27),
            (&map.line28, f.line28),
            (&map.line29, f.line29),
            (&map.line30, f.line30),
            (&map.line31, f.line31),
            (&map.line32, f.line32),
            (&map.line33, f.line33),
            (&map.line34, f.line34),
            (&map.line35, f.line35),
            (&map.line36, f.line36),
            (&map.line37, f.line37),
            (&map.line38, f.line38),
            (&map.line39, f.line39),
            (&map.line40, f.line40),
        ];
        for (ord, (cell, value)) in p3.iter().enumerate() {
            push_money(
                &mut writes,
                &mut placements,
                cell,
                *value,
                COL_AMOUNT,
                Some((GRP_P2, ord as u32)),
            );
        }
    }

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
    verify_flat(&check, &fields, &placements, F6251_CLUSTERS)?;
    Ok(bytes)
}
