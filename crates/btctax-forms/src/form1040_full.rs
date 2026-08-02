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
    let p1: [(&MoneyCell, Usd); 13] = [
        // ★ 1a must print BEFORE 1z — the form says "Add lines 1a through 1h", and a filled 1z above a
        // blank 1a does not add up (Fable P6 r1 I1).
        (need(&map.line1a, "line1a", y)?, lines.line1a),
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
    let p2_amount: [(&MoneyCell, Usd); 13] = [
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
    // ★★★ §G-24 — LINES 34 AND 37 ARE MUTUALLY EXCLUSIVE, AND THE FORM SAYS BLANK, NOT ZERO.
    //
    // L34: "If line 33 is more than line 24, subtract line 24 from line 33. This is the amount you
    //       overpaid" — a CONDITION, and no "-0-" clause anywhere on the line.
    // L37: "Subtract line 33 from line 24. This is the amount you owe." — no clamp, no condition.
    //
    // `printed.rs` computes both with `.max(Usd::ZERO)`, so before this gate EVERY OWING RETURN swore
    // "you overpaid $0" and EVERY REFUND RETURN swore "you owe $0" — figures the filer never gave, on
    // lines the form leaves empty. Neither moves the tax; both fabricate testimony under §6065, which
    // is the defect §G-11 exists to name and this is its first real instance on paper.
    //
    // ★ The gate is the FORM'S OWN COMPARISON, not a negation of one arm — which is why the
    // exactly-even case (payments == tax) leaves BOTH blank: both conditions fail, and the form asks
    // for neither. A `!= ZERO` test would have got that case wrong in the same direction as the bug.
    //
    // ★ Fixed HERE rather than by retyping the leaf: `Usd` cannot express blank — that is §G-11 P0b's
    // `LineEntry` — but the writer can simply decline to write. When P0b lands this becomes a
    // `LineEntry::Blank` and the comparison moves to the constructor.
    // ★★★ r6 I-1 — RESTORE THE FAIL-CLOSED GEOMETRY THIS BRANCH TOOK AWAY.
    //
    // Before the gate, 34/35a/37 all wrote into one descent group, so `verify.rs`'s ordinal-y check
    // caught a 34↔37 or 35a↔37 map swap MAP-INDEPENDENTLY and returned `Err(Geometry(..))`. Making the
    // two arms mutually exclusive means the group never holds both again — so that swap now returns
    // `Ok` and only a value assertion in one KAT notices. The r6 review demonstrated it by planting
    // both swaps against the pre- and post-change writers.
    //
    // ★ The cost is entirely in the NEXT map: TY2024's is correct and pinned by literal FQN in a test,
    // but that is a test-time guard on one committed asset, not the production guarantee two doc
    // comments in this crate still promise. TY2025's full-return map does not exist yet.
    //
    // So assert the ordering from the BLANK PDF's own geometry, before either branch runs. It holds on
    // whichever arm fires — and on neither — because it never looks at what we wrote.
    //
    // ★★ B1 — the kill-tests are STANDING, in `full_return_forms.rs`:
    //
    //     form_1040_full_refund_owe_block_swap_34_37_fails_closed
    //     form_1040_full_refund_owe_block_swap_35a_37_fails_closed
    //     form_1040_full_refund_owe_block_across_pages_fails_closed
    //
    // ★★★ An earlier version of this comment claimed *"this guard sits on the production path of every
    // `fill_form_1040_full`, so the whole 1040 KAT suite is its kill-test"*. **That was wrong, and it is
    // the exact conflation B1 exists to keep apart.** The KAT suite reds when the MAP is broken *while
    // the guard is present*; it does not red when the GUARD is broken, because the committed map is
    // correct. Review r7 deleted this whole block and ran `-p btctax-forms`: 233 passed either way.
    // Nothing observed the difference — so the guarantee did not exist, by this repo's own definition.
    {
        let cy_of = |cell: &MoneyCell, what: &str| -> Result<(usize, f32), FormsError> {
            // A pair's dollars field carries the row's y just as a single does.
            let fqn = cell.fields()[0];
            let cy = blank_fields
                .iter()
                .find(|f| f.fqn == fqn)
                .and_then(|f| f.cy())
                .ok_or_else(|| {
                    FormsError::Geometry(format!(
                        "1040 {what} maps to {fqn}, which has no rectangle in the blank form"
                    ))
                })?;
            Ok((page_of(fqn), cy))
        };
        // ★ These three cells are REQUIRED in every full-return map, on every return — including the
        //   exactly-even return that writes none of them. That is deliberate (fail closed), but it is a
        //   real coupling and it lands on an ELECTION: line 35a is *"Amount of line 34 you want refunded
        //   to you"*, which §G-11 records as a choice btctax makes without asking. Unmapping it — the
        //   obvious way to stop making that election, and what the map already does for 35b/35c/35d and
        //   36 — would otherwise refuse EVERY 1040 in the product with a message about geometry. So the
        //   refusal names the line and says what to do instead.
        fn need_mapped<'a>(
            cell: &'a Option<MoneyCell>,
            what: &str,
        ) -> Result<&'a MoneyCell, FormsError> {
            cell.as_ref().ok_or_else(|| {
                FormsError::Geometry(format!(
                    "1040 {what} is not mapped, and the refund/owe ordering guard requires all three \
                     of lines 34/35a/37 on every return. If {what} was unmapped deliberately (e.g. to \
                     stop electing line 35a for the filer), this guard must be taught that — do not \
                     delete it, or a 34/37 swap goes back to printing the amount OWED into the box \
                     captioned OVERPAID."
                ))
            })
        }
        let (p34, y34) = cy_of(need_mapped(&map.line34, "line 34")?, "line 34")?;
        let (p35a, y35a) = cy_of(need_mapped(&map.line35a, "line 35a")?, "line 35a")?;
        let (p37, y37) = cy_of(need_mapped(&map.line37, "line 37")?, "line 37")?;
        // ★ Same page FIRST. Raw PDF y is only comparable within a page, so an ordering test alone is
        //   page-blind: a `line37` aimed at a page-1 amount cell (page 1's money column runs to low y)
        //   satisfies `y34 > y35a > y37` and sails through. `verify_flat` cannot catch it either — for
        //   money cells the expected page comes from `page_of(fqn)`, i.e. from the map's own string, so
        //   that leg is tautological across pages.
        if !(p34 == p35a && p35a == p37) {
            return Err(FormsError::Geometry(format!(
                "1040 refund/owe block is mis-mapped: lines 34/35a/37 land on pages \
                 {p34}/{p35a}/{p37}. They are one block on one page, and y is only comparable within \
                 a page — so a cross-page mis-map would defeat the ordering check below."
            )));
        }
        // The form prints them in this order down the page, so their centres strictly descend.
        // ★ `EPS` rather than a bare `>`, matching `verify.rs`'s descent leg: two money cells within a
        //   point of each other are the same printed row, not an ordering.
        if !(y34 > y35a + crate::verify::EPS && y35a > y37 + crate::verify::EPS) {
            return Err(FormsError::Geometry(format!(
                "1040 refund/owe block is mis-mapped: lines 34/35a/37 sit at y \
                 {y34:.1}/{y35a:.1}/{y37:.1}, which is not strictly descending. The form prints \
                 'amount you overpaid', then 'refunded to you', then 'amount you owe' — a swap here \
                 would print the amount OWED into the box captioned OVERPAID."
            )));
        }
    }

    let overpaid = lines.line33 > lines.line24;
    let owed = lines.line24 > lines.line33;
    if overpaid {
        for (ord, cell) in [
            need(&map.line34, "line34", y)?,
            need(&map.line35a, "line35a", y)?, // 35a = the overpayment refunded
        ]
        .into_iter()
        .enumerate()
        {
            push_money(
                &mut writes,
                &mut placements,
                cell,
                lines.line34,
                COL_AMOUNT,
                Some((GRP_P2_AMOUNT, (p2_amount.len() + ord) as u32)),
            );
        }
    }
    // Line 37 (amount owed) sits below 35a and is still the AMOUNT column.
    if owed {
        push_money(
            &mut writes,
            &mut placements,
            need(&map.line37, "line37", y)?,
            lines.line37,
            COL_AMOUNT,
            Some((GRP_P2_AMOUNT, (p2_amount.len() + 2) as u32)),
        );
    }

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
