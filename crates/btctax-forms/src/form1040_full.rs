//! Form 1040 — the **FULL-RETURN** fill: every line, not just the capital-gain cluster.
//!
//! This is not `form1040::fill_form_1040_capgains`, which writes only line 7 and the Digital-Asset
//! question for the crypto-slice export. That one stays; for a year with no `ReturnInputs` it is what
//! the filer wants.
//!
//! **This module does no tax arithmetic.** Every cell is transcribed from [`Form1040Lines`], which
//! core derives — and which carries each schedule's **printed** figure, so the 1040 ties out against
//! its own attachments to the dollar.
//!
//! **★ Three x-clusters.** The lettered "a" sub-lines (2a, 3a) sit at x ≈ [252, 324] — neither the
//! MID column (25a–25c, 31) at [410, 482] nor the AMOUNT column at [504, 576]. Line 3a is the one
//! v1 writes there, and it is the preferential-rate slice: putting it in the wrong box would report
//! qualified dividends as something else entirely.
//!
//! **★ `f1_57` is line 12 on the 2024 form and line 1z on the 2025 one** (SPEC §7.4). The maps are
//! per-(form, year) for exactly this reason.
//!
//! **★★ The 5-way filing-status group is a NAME COLLISION.** Two distinct fields are both called
//! `c1_3[0]` (Single, and Head of household) and two are both called `c1_3[1]` (MFJ, and QSS) —
//! distinguished ONLY by their parent subform. A map keyed on the leaf name checks the wrong filing
//! status, which changes the standard deduction, every bracket and every threshold on the return.
//! The on-states (1=Single, 2=HoH, 3=MFJ, 4=MFS, 5=QSS) are distinct, so they independently
//! corroborate the mapping, and [`fill_form_1040_full_with_map`] asserts both.
//!
//! **Line 7 is signed with a LEADING MINUS** (SPEC §3.2), unlike Schedule D's parenthesized boxes.

use crate::cells::{page_of, push_money, render_ssn};
use crate::error::FormsError;
use crate::map::{CheckChoice, Form1040HeaderCells, Form1040Map, MoneyCell};
use crate::pdf;
use crate::verify::{verify_flat, FlatPlacement};
use btctax_core::tax::packet::ReturnHeader;
use btctax_core::tax::printed::Form1040Lines;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;

/// Logical Form 1040 columns.
const COL_SUBLINE: usize = 0; // the lettered "a" sub-lines (2a, 3a)
const COL_MID: usize = 1; // 25a, 25b, 25c, 31
const COL_AMOUNT: usize = 2; // everything else

const F1040_CLUSTERS: &[(f32, f32)] = &[(252.0, 324.0), (410.0, 482.0), (504.0, 576.0)];

/// Descent groups — per page AND per column (a page-2 y is not comparable with a page-1 y, and a
/// sub-line cell shares its row's y with the amount cell beside it).
const GRP_P1_AMOUNT: u32 = 0;
const GRP_P1_SUBLINE: u32 = 1;
const GRP_P2_AMOUNT: u32 = 2;
const GRP_P2_MID: u32 = 3;

fn need<'a, T>(cell: &'a Option<T>, what: &str, year: i32) -> Result<&'a T, FormsError> {
    cell.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            "the TY{year} Form 1040 map has no `{what}` — the full-return fill needs it. Full-return \
             v1 is TY2024-only."
        ))
    })
}

/// The filing-status box + on-state for `status`. **Fully-qualified names only** — the leaf names
/// collide (Single/HoH both `c1_3[0]`; MFJ/QSS both `c1_3[1]`).
fn filing_status_box(
    map: &Form1040Map,
    status: FilingStatus,
    year: i32,
) -> Result<&CheckChoice, FormsError> {
    let fs = need(&map.filing_status, "filing_status", year)?;
    Ok(match status {
        FilingStatus::Single => &fs.single,
        FilingStatus::HoH => &fs.hoh,
        FilingStatus::Mfj => &fs.mfj,
        FilingStatus::Mfs => &fs.mfs,
        FilingStatus::Qss => &fs.qss,
    })
}

/// Fill the FULL-RETURN Form 1040. The serialized bytes are read back through the geometric verifier
/// (a mis-mapped cell FAILS CLOSED).
pub fn fill_form_1040_full_with_map(
    lines: &Form1040Lines,
    header: &ReturnHeader,
    status: FilingStatus,
    map: &Form1040Map,
) -> Result<Vec<u8>, FormsError> {
    let y = map.year;
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<FlatPlacement> = Vec::new();

    // ── The identity block. A 1040 with the right money and no NAME is not a return. ─────────────
    let h = map.header.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            "the TY{y} 1040 map has no [header] block — a full return cannot file an unnamed 1040"
        ))
    })?;
    let blank = pdf::load(pdf::f1040_pdf(y)?)?;
    let blank_fields = pdf::collect_fields(&blank)?;
    push_header_block(
        &mut writes,
        &mut placements,
        h,
        header,
        status,
        &blank_fields,
    )?;

    // ── Page 1, AMOUNT column, top to bottom. Line 7 carries a LEADING MINUS on a loss year. ────
    let p1: [(&MoneyCell, Usd); 12] = [
        (need(&map.line1z, "line1z", y)?, lines.line1z),
        (need(&map.line2b, "line2b", y)?, lines.line2b),
        (need(&map.line3b, "line3b", y)?, lines.line3b),
        (&map.line7a, lines.line7), // the existing crypto-slice cell IS line 7 on the 2024 form
        (need(&map.line8, "line8", y)?, lines.line8),
        (need(&map.line9, "line9", y)?, lines.line9),
        (need(&map.line10, "line10", y)?, lines.line10),
        (need(&map.line11, "line11", y)?, lines.line11),
        (need(&map.line12, "line12", y)?, lines.line12),
        (need(&map.line13, "line13", y)?, lines.line13),
        (need(&map.line14, "line14", y)?, lines.line14),
        (need(&map.line15, "line15", y)?, lines.line15),
    ];
    for (ord, (cell, value)) in p1.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_AMOUNT,
            Some((GRP_P1_AMOUNT, ord as u32)),
        );
    }
    // Line 2a — tax-exempt interest, the SUBLINE column, one printed row ABOVE 3a (so ordinal 0).
    push_money(
        &mut writes,
        &mut placements,
        need(&map.line2a, "line2a", y)?,
        lines.line2a,
        COL_SUBLINE,
        Some((GRP_P1_SUBLINE, 0)),
    );

    // ★ Line 3a — the SUBLINE column. The preferential-rate slice; a wrong box misreports it.
    push_money(
        &mut writes,
        &mut placements,
        need(&map.line3a, "line3a", y)?,
        lines.line3a,
        COL_SUBLINE,
        Some((GRP_P1_SUBLINE, 1)),
    );

    // ── Page 2, AMOUNT column. ──────────────────────────────────────────────────────────────────
    let p2_amount: [(&MoneyCell, Usd); 15] = [
        (need(&map.line16, "line16", y)?, lines.line16),
        (need(&map.line17, "line17", y)?, lines.line17),
        (need(&map.line18, "line18", y)?, lines.line18),
        (need(&map.line19, "line19", y)?, lines.line19),
        (need(&map.line20, "line20", y)?, lines.line20),
        (need(&map.line21, "line21", y)?, lines.line21),
        (need(&map.line22, "line22", y)?, lines.line22),
        (need(&map.line23, "line23", y)?, lines.line23),
        (need(&map.line24, "line24", y)?, lines.line24),
        (need(&map.line25d, "line25d", y)?, lines.line25d),
        (need(&map.line26, "line26", y)?, lines.line26),
        (need(&map.line32, "line32", y)?, lines.line32),
        (need(&map.line33, "line33", y)?, lines.line33),
        (need(&map.line34, "line34", y)?, lines.line34),
        (need(&map.line35a, "line35a", y)?, lines.line34), // 35a = the overpayment refunded
    ];
    for (ord, (cell, value)) in p2_amount.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_AMOUNT,
            Some((GRP_P2_AMOUNT, ord as u32)),
        );
    }
    // Line 37 (amount owed) sits below 35a and is still the AMOUNT column.
    push_money(
        &mut writes,
        &mut placements,
        need(&map.line37, "line37", y)?,
        lines.line37,
        COL_AMOUNT,
        Some((GRP_P2_AMOUNT, p2_amount.len() as u32)),
    );

    // ── Page 2, MID column. ─────────────────────────────────────────────────────────────────────
    let p2_mid: [(&MoneyCell, Usd); 4] = [
        (need(&map.line25a, "line25a", y)?, lines.line25a),
        (need(&map.line25b, "line25b", y)?, lines.line25b),
        (need(&map.line25c, "line25c", y)?, lines.line25c),
        (need(&map.line31, "line31", y)?, lines.line31),
    ];
    for (ord, (cell, value)) in p2_mid.iter().enumerate() {
        push_money(
            &mut writes,
            &mut placements,
            cell,
            *value,
            COL_MID,
            Some((GRP_P2_MID, ord as u32)),
        );
    }

    // ── The 5-way filing-status box (fully-qualified name + its distinct on-state). ─────────────
    let fs = filing_status_box(map, status, y)?;
    writes.push((
        fs.field.clone(),
        pdf::FieldValue::Check { on: fs.on.clone() },
    ));
    placements.push(FlatPlacement::check(fs.field.clone(), 0));

    // ── The Digital-Asset question. btctax answers "Yes" or leaves it to the filer — never "No". ─
    if lines.digital_asset_yes {
        let da = map
            .da_yes
            .as_ref()
            .ok_or_else(|| FormsError::Geometry(format!("the TY{y} 1040 map has no `da_yes`")))?;
        writes.push((
            da.field.clone(),
            pdf::FieldValue::Check { on: da.on.clone() },
        ));
        placements.push(FlatPlacement::check(da.field.clone(), 0));
    }

    let mut doc = pdf::load(pdf::f1040_pdf(y)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_flat(&check, &fields, &placements, F1040_CLUSTERS)?;
    Ok(bytes)
}

/// Write the 1040's identity block: names, SSNs, address, the checkbox row, and the dependents table.
///
/// Every cell is a [`FlatPlacement::free`] (or `check`) — geometry-exempt, since none of them sits in a
/// money column, but still page-checked and inside the no-unmapped set. `free` catches STRAY writes,
/// not MISSING ones, so the KATs assert each cell reads back non-empty: an unnamed return is the exact
/// failure this block exists to prevent, and the geometric oracle cannot see it.
#[allow(clippy::too_many_arguments)]
fn push_header_block(
    w: &mut Vec<(String, pdf::FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cells: &Form1040HeaderCells,
    header: &ReturnHeader,
    status: FilingStatus,
    blank_fields: &[pdf::Field],
) -> Result<(), FormsError> {
    let max_len_of = |fqn: &str| -> Option<usize> {
        blank_fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.max_len)
    };
    let text = |w: &mut Vec<(String, pdf::FieldValue)>,
                p: &mut Vec<FlatPlacement>,
                fqn: &str,
                value: &str| {
        if value.is_empty() {
            return; // an empty cell is left BLANK, never written with ""
        }
        w.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
        p.push(FlatPlacement::free(fqn.to_string(), page_of(fqn)));
    };
    let check = |w: &mut Vec<(String, pdf::FieldValue)>,
                 p: &mut Vec<FlatPlacement>,
                 c: &CheckChoice,
                 on: bool| {
        if !on {
            return; // an unchecked box is simply not written
        }
        w.push((c.field.clone(), pdf::FieldValue::Check { on: c.on.clone() }));
        p.push(FlatPlacement::check(c.field.clone(), page_of(&c.field)));
    };

    // Names + SSNs. The SSN rendering follows the CELL's own /MaxLen (9 here ⇒ bare digits).
    let t = &header.taxpayer;
    text(w, p, &cells.taxpayer_first, &t.first_name);
    text(w, p, &cells.taxpayer_last, &t.last_name);
    text(
        w,
        p,
        &cells.taxpayer_ssn,
        &render_ssn(&t.ssn, max_len_of(&cells.taxpayer_ssn))?,
    );
    if let Some(sp) = &header.spouse {
        text(w, p, &cells.spouse_first, &sp.first_name);
        text(w, p, &cells.spouse_last, &sp.last_name);
        text(
            w,
            p,
            &cells.spouse_ssn,
            &render_ssn(&sp.ssn, max_len_of(&cells.spouse_ssn))?,
        );
        // "If you checked the MFS box, enter the name of your spouse" — MFS only. (On HoH/QSS that same
        // cell wants the qualifying CHILD's name, which v1 does not capture, so it stays blank.)
        if status == FilingStatus::Mfs {
            text(w, p, &cells.mfs_spouse_name, &sp.full_name());
        }
    }

    // The signature block (page 2): occupations, and the IP PIN — whose absence gets a paper return
    // REJECTED when one was issued. The spouse's IP PIN is not captured, so that cell stays blank.
    text(w, p, &cells.occupation_taxpayer, &t.occupation);
    if let Some(sp) = &header.spouse {
        text(w, p, &cells.occupation_spouse, &sp.occupation);
    }
    if let Some(pin) = &header.ip_pin {
        text(w, p, &cells.ip_pin, pin.digits());
    }

    text(w, p, &cells.address_street, &header.address_street);
    text(w, p, &cells.address_city, &header.address_city);
    text(w, p, &cells.address_state, &header.address_state);
    text(w, p, &cells.address_zip, &header.address_zip);

    check(
        w,
        p,
        &cells.presidential_taxpayer,
        header.presidential_fund_taxpayer,
    );
    check(
        w,
        p,
        &cells.presidential_spouse,
        header.presidential_fund_spouse,
    );
    check(
        w,
        p,
        &cells.claimed_dependent_taxpayer,
        header.claimed_as_dependent_taxpayer,
    );
    check(
        w,
        p,
        &cells.claimed_dependent_spouse,
        header.claimed_as_dependent_spouse,
    );
    check(w, p, &cells.mfs_spouse_itemizes, header.mfs_spouse_itemizes);

    // ★ The §63(f) boxes. These must agree with L12 or the return fails the IRS's own arithmetic
    // cross-check; core derives the count ONCE (`AgedBlindBoxes`) and L12 consumes that same count.
    let ab = header.aged_blind;
    check(w, p, &cells.taxpayer_aged, ab.taxpayer_aged);
    check(w, p, &cells.taxpayer_blind, ab.taxpayer_blind);
    check(w, p, &cells.spouse_aged, ab.spouse_aged);
    check(w, p, &cells.spouse_blind, ab.spouse_blind);

    // Dependents. More than the form physically holds REFUSES: the IRS's own remedy is to check
    // `more_than_four_dependents` and attach a continuation statement, which is a synthetic page
    // generator v1 does not have (the same posture as Schedule B's >14-payer refusal, SPEC §7.4).
    // Printing only the first four would file a return that misstates the household — silently.
    if header.dependents.len() > cells.dependent_rows.len() {
        return Err(FormsError::Overflow {
            part: "the 1040 dependents table",
            rows: header.dependents.len(),
            capacity: cells.dependent_rows.len(),
        });
    }
    for (d, row) in header.dependents.iter().zip(&cells.dependent_rows) {
        text(w, p, &row.name, &d.name);
        text(w, p, &row.ssn, &render_ssn(&d.ssn, max_len_of(&row.ssn))?);
        text(w, p, &row.relationship, &d.relationship);
        // row.ctc / row.odc are deliberately NOT checked — v1 omits the credit (L19 = 0).
    }
    Ok(())
}
