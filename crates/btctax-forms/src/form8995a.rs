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

use crate::cells::{page_of, push_identity, push_money, push_money_opt};
use crate::error::FormsError;
use crate::map::Form8995AMap;
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::qbi_a::{Form8995APartIToIii, Form8995APartIv};
use btctax_core::Usd;

/// Logical columns, matching the map's header.
///
/// ★ Part IV prints in MID / AMOUNT; Parts II and III print in the three-business grid's **column A**;
/// Part III lines 20-24 print in the single `Ln` entry box, which sits well LEFT of column A. Keeping
/// that last band separate is what makes a drift onto the form's read-only `_RO` column mirrors fail
/// the read-back instead of landing silently.
const COL_MID: usize = 0;
const COL_AMOUNT: usize = 1;
const COL_A: usize = 2;
const COL_LN: usize = 3;
const F8995A_CLUSTERS: &[(f32, f32)] = &[
    (410.0, 482.0),
    (504.0, 576.0),
    (360.0, 432.0),
    (266.0, 338.0),
];

/// The descent group for Part IV — one column-agnostic run down page 2.
const GRP_PART_IV: u32 = 0;
/// Part II — one run down page 1, lines 2 to 16.
const GRP_PART_II: u32 = 1;
/// Part III — one run down page 2, lines 17 to 26. ★ Its own group, not Part IV's: the two share a
/// page and Part III sits ABOVE Part IV, so a single group would be satisfiable by a wrong ordering.
const GRP_PART_III: u32 = 2;

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
    parts_i_to_iii: Option<&Form8995APartIToIii>,
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

    // ── Parts I-III (§G-28/B1b) — present exactly when there is a qualified trade or business. A
    //    REIT/PTP-only filer, and a filer whose SSTB §199A(d)(3) excluded, file Part IV alone: that is
    //    i8995a's own "If you don't have QBI, and only have REIT, PTP, skip Parts I through III".
    if let Some(f) = parts_i_to_iii {
        // ★★★ FAIL CLOSED ON A NAMELESS TRADE OR BUSINESS — the identical guard `form8995.rs` carries
        //     for the identical decision. Part II line 2 is this business's QBI and Part I row A is
        //     where it is named; a non-zero line 2 over a blank row A claims a §199A deduction for a
        //     business the return never names. Core refuses it first
        //     (`ScheduleCNoBusinessDescription`), so this is unreachable today and exists to STAY
        //     unreachable — it was missing here purely because nobody held the two emitters at once,
        //     which is the branch's own recurring failure mode.
        if f.part_ii.line2 > Usd::ZERO && f.part_i.col_a_name.trim().is_empty() {
            return Err(FormsError::Geometry(
                "Form 8995-A Part II line 2 is non-zero but the trade or business has no name: \
                 Part I column (a) is \"Trade, business, or aggregation name\", so this would file a \
                 qualified business income component for a business the return never names."
                    .into(),
            ));
        }
        // Part I row A. The name and TIN are wide free-text cells (geometry-exempt, page-checked);
        // the three checkboxes are written only when TRUE — an unchecked box is not written at all,
        // because writing an "off" value is a mark the filer did not make.
        let a = &f.part_i;
        writes.push((
            map.part1_row_a.name.clone(),
            pdf::FieldValue::Text(a.col_a_name.clone()),
        ));
        placements.push(FlatPlacement::free(
            map.part1_row_a.name.clone(),
            page_of(&map.part1_row_a.name),
        ));
        // ★★ Column (d) is the PROPRIETOR's SSN, not the return's primary taxpayer's — a
        //    spouse-owned business files under the spouse's, even on a joint return, and an 8995-A
        //    naming a TIN that matches no Schedule C in the same packet is exactly the mismatch IRS
        //    matching flags. Form 8995 row 1i(b) resolves it the same way.
        //
        // ★ Rendered through `render_ssn` against THIS cell's own `/MaxLen` rather than a hardcoded
        //   format: TY2024's column (d) declares 11 (hyphenated), but hardcoding that would write
        //   eleven characters into a nine-character comb if a later revision narrows the widget.
        let proprietor = header.proprietor.as_ref().ok_or_else(|| {
            FormsError::Geometry(
                "Form 8995-A Part I lists a trade or business but the return names no proprietor to \
                 file it under"
                    .into(),
            )
        })?;
        let tin_max_len = blank_fields
            .iter()
            .find(|f| f.fqn == map.part1_row_a.tin)
            .ok_or_else(|| FormsError::MapFieldMissing(map.part1_row_a.tin.clone()))?
            .max_len;
        writes.push((
            map.part1_row_a.tin.clone(),
            pdf::FieldValue::Text(crate::cells::render_ssn(&proprietor.ssn, tin_max_len)?),
        ));
        placements.push(FlatPlacement::free(
            map.part1_row_a.tin.clone(),
            page_of(&map.part1_row_a.tin),
        ));
        for (c, on) in [
            (
                &map.part1_row_a.specified_service,
                a.col_b_specified_service,
            ),
            (&map.part1_row_a.aggregation, a.col_c_aggregation),
            (&map.part1_row_a.patron, a.col_e_patron),
        ] {
            if on {
                writes.push((c.field.clone(), pdf::FieldValue::Check { on: c.on.clone() }));
                placements.push(FlatPlacement::check(c.field.clone(), page_of(&c.field)));
            }
        }

        // Part II, column A, lines 2-16 top to bottom.
        let ii = &f.part_ii;
        let m2 = &map.part2_col_a;
        let unconditional: [(&crate::map::MoneyCell, Usd); 13] = [
            (&m2.line2, ii.line2),
            (&m2.line3, ii.line3),
            (&m2.line4, ii.line4),
            (&m2.line5, ii.line5),
            (&m2.line6, ii.line6),
            (&m2.line7, ii.line7),
            (&m2.line8, ii.line8),
            (&m2.line9, ii.line9),
            (&m2.line10, ii.line10),
            (&m2.line11, ii.line11),
            (&m2.line13, ii.line13),
            (&m2.line15, ii.line15),
            (&m2.line16, ii.line16),
        ];
        // ★ The descent ordinal is the LINE NUMBER, not the position in this array — lines 12 and 14
        //   are conditional and may be absent, and a positional ordinal would then renumber every
        //   line below them and make the descent check pass on a shifted fill.
        let ord_of = |cell: &crate::map::MoneyCell| -> u32 {
            for (n, c) in (2u32..=16).zip([
                &m2.line2, &m2.line3, &m2.line4, &m2.line5, &m2.line6, &m2.line7, &m2.line8,
                &m2.line9, &m2.line10, &m2.line11, &m2.line12, &m2.line13, &m2.line14, &m2.line15,
                &m2.line16,
            ]) {
                if std::ptr::eq(c, cell) {
                    return n;
                }
            }
            unreachable!("every Part II cell is in the line list")
        };
        for (cell, value) in unconditional {
            push_money(
                &mut writes,
                &mut placements,
                cell,
                value,
                COL_A,
                Some((GRP_PART_II, ord_of(cell))),
            );
        }
        // ★★ L12 "Enter the amount from line 26, IF ANY" and L14 "…from Schedule D (Form 8995-A),
        //    line 6, IF ANY" are CONDITIONAL entries with no `-0-` clause. Outside the phase-in range
        //    there is no line 26; btctax fills no Schedule D because a patron refuses. Both stay
        //    BLANK, and `push_money_opt` is how that is expressible — a printed `0` on line 12 would
        //    swear a phased-in reduction was figured and came to nothing.
        push_money_opt(
            &mut writes,
            &mut placements,
            &m2.line12,
            ii.line12,
            COL_A,
            Some((GRP_PART_II, 12)),
        );
        push_money_opt(
            &mut writes,
            &mut placements,
            &m2.line14,
            ii.line14,
            COL_A,
            Some((GRP_PART_II, 14)),
        );

        // Part III — written only when the form's own gate says to complete it.
        if let Some(iii) = f.part_iii.as_ref() {
            let m3 = &map.part3_col_a;
            for (cell, value, col, ord) in [
                (&m3.line17, iii.line17, COL_A, 17u32),
                (&m3.line18, iii.line18, COL_A, 18),
                (&m3.line19, iii.line19, COL_A, 19),
                (&m3.line20, iii.line20, COL_LN, 20),
                (&m3.line21, iii.line21, COL_LN, 21),
                (&m3.line22, iii.line22, COL_LN, 22),
                (&m3.line23, iii.line23, COL_LN, 23),
                (&m3.line25, iii.line25, COL_A, 25),
                (&m3.line26, iii.line26, COL_A, 26),
            ] {
                push_money(
                    &mut writes,
                    &mut placements,
                    cell,
                    value,
                    col,
                    Some((GRP_PART_III, ord)),
                );
            }
            // ★★★ L24 IS A PERCENTAGE, not a dollar amount. `Form8995APartIii` stores the RATIO
            //     (line 22 ÷ line 23), because that is what line 25 multiplies by; the form prints
            //     it into a box it suffixes with `%`, so the printed figure is ratio × 100. Scaling
            //     here, once, at the single point where the ratio becomes ink, is what keeps the
            //     arithmetic and the printing from disagreeing.
            push_money(
                &mut writes,
                &mut placements,
                &m3.line24,
                //     ★ `.normalize()` strips the Decimal SCALE's trailing zeros: 14031/50000 has
                //       scale 5, so the raw product renders "28.06200 %" on a filed form. The value
                //       is identical; only the ink changes.
                (iii.line24_ratio * rust_decimal::Decimal::from(100)).normalize(),
                COL_LN,
                Some((GRP_PART_III, 24)),
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
    verify_flat(&check, &fields, &placements, F8995A_CLUSTERS)?;
    Ok(bytes)
}
