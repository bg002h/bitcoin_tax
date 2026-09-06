//! Schedule D fill: the per-BOX total rows **1b / 2 / 3** (short-term) and **8b / 9 / 10**
//! (long-term), the nets on **7** and **15**, **16** (total), **17** (Part III's first routing
//! question) and the QOF Yes/No question (SP1 answers **No**). Lines 7/15 are pure arithmetic over
//! the box rows printed above them, so filling 16 while leaving 7/15 blank would be
//! self-inconsistent — we fill all of them.
//!
//! **★ spec 1099-DA T8 — one line per Form 8949 BOX, from the ROUTED rows.** The page-set and the
//! schedule are aggregated from the SAME `Form8949Row`s (`btctax_core::schedule_d_by_box`), never
//! re-derived from the ledger, so they cannot disagree; see [`fill_schedule_d_totals`] for the
//! box→line table in the form's own words.
//!
//! **★ Line 17 is answered here, and lines 18–22 are not.** The distinction is what the question can
//! be answered FROM. Line 17 — *"Are lines 15 and 16 both gains?"* — reads two lines that are PRINTED
//! on this very page, so it introduces no fact the form does not already assert; it is the same kind
//! of derivation as line 16 = line 7 + line 15, and the single definition lives in core
//! ([`btctax_core::tax::printed::schedule_d_line17`]), shared with the full return so the two can
//! never disagree. Lines 18/19 (28%-rate gain, unrecaptured §1250) are blank here because nothing was
//! ever ASKED, not because they are zero; line 20's *"…and you are not filing **Form 4952**?"* is a
//! fact about the filer that no btctax input carries; line 21 needs the §1211 ceiling by filing
//! status; and line 22 needs Form 1040 line 3a (qualified dividends), which the crypto slice never
//! sees. Answering any of those would be testimony the filer never gave — the CLI prints a notice
//! naming them instead.
//!
//! On the 2025 revision Schedule D line 3 reads "Box C **or Box I**" and line 10 "Box F **or Box L**",
//! so the digital-asset Box I/L totals from Form 8949 flow straight onto these lines; on the pre-2025
//! revisions the same lines read "Box C checked" / "Box F checked" and carry the securities-box
//! totals. This module fills all three revisions (2017/2024/2025) — and the TY2017 map binds no
//! 1b/2/8b/9 rows at all, which is correct: no pre-2025 revision has them, and a pre-2025 slice
//! routes every row to C/F.

use crate::error::FormsError;
use crate::map::{AmountCols, ScheduleDMap};
use crate::verify::{column_x_bands, in_band, no_unmapped_filled, Geo, Placement};
use crate::{fmt_money, pdf};
use btctax_core::{Form8949Box, ScheduleDPart, ScheduleDTotals};
use std::collections::BTreeMap;

/// Amount-column indices as ordered left→right in the Part I grid: d=0, e=1, g=2, h=3.
const SD_COL_D: usize = 0;
const SD_COL_E: usize = 1;
const SD_COL_H: usize = 3;

/// A part is "active" (worth a Schedule D line) iff it has any proceeds/cost/gain.
fn active(p: &btctax_core::ScheduleDPart) -> bool {
    !p.proceeds.is_zero() || !p.cost_basis.is_zero() || !p.gain.is_zero()
}

fn push_amount_line(
    line: &AmountCols,
    proceeds: &str,
    cost: &str,
    gain: &str,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    // (d) proceeds, (e) cost, (h) gain — (g) adjustment stays blank (crypto models no adjustment).
    for (fqn, value, col) in [
        (&line.proceeds_d, proceeds, SD_COL_D),
        (&line.cost_e, cost, SD_COL_E),
        (&line.gain_h, gain, SD_COL_H),
    ] {
        writes.push((fqn.clone(), pdf::FieldValue::Text(value.to_string())));
        placements.push(Placement {
            fqn: fqn.clone(),
            geo: Geo::Data { row: 0, col },
        });
    }
}

/// Sum the per-box totals of one Schedule D LINE's box set (spec 1099-DA T8).
fn box_group(
    by_box: &BTreeMap<Form8949Box, ScheduleDPart>,
    boxes: &[Form8949Box],
) -> ScheduleDPart {
    let mut t = ScheduleDPart::default();
    for b in boxes {
        if let Some(p) = by_box.get(b) {
            t.proceeds += p.proceeds;
            t.cost_basis += p.cost_basis;
            t.gain += p.gain;
        }
    }
    t
}

/// A per-box row this revision's map does not bind. Mirrors `schedule_d_full::need`: the fill
/// REFUSES rather than dropping a total off the page — a box group with rows whose line has no
/// cells is exactly the "nothing ever populated it" blank the census rule forbids.
fn need<'a>(
    cell: &'a Option<AmountCols>,
    line: &str,
    boxes: &str,
    year: i32,
) -> Result<&'a AmountCols, FormsError> {
    cell.as_ref().ok_or_else(|| {
        FormsError::Geometry(format!(
            "the TY{year} Schedule D map has no `{line}` — but this year's Form 8949 carries a {boxes} \
             page-set whose total belongs on that line. Filling without it would drop the total off \
             the filed page; bind the row in the year's `schedule_d.map.toml` (spec 1099-DA T8)."
        ))
    })
}

// ★★★ spec 1099-DA R6 fold (M-1) — THE SIX BOX GROUPS, WRITTEN ONCE.
//
// They are shared by [`partition_or_refuse`] and by the fill below, so a box dropped from a group
// cannot be dropped from only one of the two and go unnoticed.
//
// The form's captions name TWELVE boxes (1b = "Box A or Box G", 2 = "Box B or Box H", 8b = "Box D or
// Box J", 9 = "Box E or Box K") but `Form8949Box` has only EIGHT variants: A, B, D and E are the
// 1099-B **securities** boxes, which btctax never emits (`btctax_core::Form8949Box`'s own doc: "We
// NEVER auto-assign the 1099-reported boxes") and which therefore have no variant to group. So the
// six groups are exhaustive over the type — which `partition_or_refuse` PROVES on every fill rather
// than assuming, since a box in no group would be aggregated by `schedule_d_by_box` and then dropped
// off the filed page, invisibly (a missing Schedule D line is a normal blank).
const G_LINE1B: &[Form8949Box] = &[Form8949Box::G];
const G_LINE2: &[Form8949Box] = &[Form8949Box::H];
const G_LINE3: &[Form8949Box] = &[Form8949Box::C, Form8949Box::I];
const G_LINE8B: &[Form8949Box] = &[Form8949Box::J];
const G_LINE9: &[Form8949Box] = &[Form8949Box::K];
const G_LINE10: &[Form8949Box] = &[Form8949Box::F, Form8949Box::L];
const BOX_GROUPS: [(&str, &[Form8949Box]); 6] = [
    ("line1b", G_LINE1B),
    ("line2", G_LINE2),
    ("line3", G_LINE3),
    ("line8b", G_LINE8B),
    ("line9", G_LINE9),
    ("line10", G_LINE10),
];

/// ★ spec 1099-DA R6 fold (M-1) — every box WITH ROWS lands on exactly one Schedule D line.
///
/// `btctax_core::schedule_d_by_box` keys on whatever `row.box_` holds, so a box carried by NO group
/// would be summed and then silently dropped, and a box carried by TWO would print its total twice.
/// Both refuse here, naming the box — never a wrong or a vanished figure on a filed page.
///
/// Split out from [`fill_schedule_d_totals`] — the same shape as the CLI's `first_unresolved_map`
/// kill pattern — so a negative test can plant a group list with a box missing, which the real
/// `BOX_GROUPS` never is.
fn partition_or_refuse(
    by_box: &BTreeMap<Form8949Box, ScheduleDPart>,
    groups: &[(&str, &[Form8949Box])],
    year: i32,
) -> Result<(), FormsError> {
    for b in by_box.keys() {
        let lines: Vec<&str> = groups
            .iter()
            .filter(|(_, g)| g.contains(b))
            .map(|(line, _)| *line)
            .collect();
        if lines.len() != 1 {
            return Err(FormsError::Geometry(format!(
                "the TY{year} Schedule D box→line table carries Form 8949 Box {b:?} on {n} of its six \
                 lines ({lines:?}), not exactly one — this year's rows include that box, so its total \
                 would be {verb} the filed page. Fix the box groups (spec 1099-DA T8).",
                n = lines.len(),
                verb = if lines.is_empty() {
                    "dropped off"
                } else {
                    "printed twice on"
                },
            )));
        }
    }
    Ok(())
}

/// Fill Schedule D from the year's part totals and the ROUTED rows' per-box totals, and return the
/// serialized bytes, read back through a geometric + no-unmapped verifier (a mis-mapped amount
/// column fails closed).
///
/// ★ spec 1099-DA T8 — the BOX → LINE table, in the words of the 2025 form itself:
///
/// - line **1b** — *"Totals for all transactions reported on Form(s) 8949 with **Box A** or **Box G**
///   checked"* (A is a securities box btctax never emits; G is the digital-asset one it does)
/// - line **2** — *"… with Box B or Box H checked"*
/// - line **3** — *"… with Box C or Box I checked"* — the not-reported box: **C** before TY2025 and
///   **I** from TY2025, so a TY2017/TY2024 slice keeps its WHOLE Part I total on line 3
/// - line **8b** — *"… Box D or Box J"*, **9** — *"… Box E or Box K"*, **10** — *"… Box F or Box L"*
///
/// The same pairing `btctax_core::tax::printed::schedule_d_lines` encodes for the FULL return
/// (`&[B::I, B::C]` / `&[B::L, B::F]`), so the slice and the full return cannot put one box's total
/// on two different lines. A box group with no rows writes NOTHING; a box group WITH rows whose map
/// row is unbound REFUSES ([`need`]) rather than dropping the total.
///
/// `by_box` comes from `btctax_core::schedule_d_by_box` over the SAME rows the Form 8949 page-sets
/// were printed from, so the schedule and the page-set behind it cannot disagree; `totals` supplies
/// the two part nets (lines 7/15) and line 16, which are sums over the whole part either way.
pub fn fill_schedule_d_totals(
    totals: &ScheduleDTotals,
    by_box: &BTreeMap<Form8949Box, ScheduleDPart>,
    map: &ScheduleDMap,
) -> Result<Vec<u8>, FormsError> {
    let mut writes: Vec<(String, pdf::FieldValue)> = Vec::new();
    let mut placements: Vec<Placement> = Vec::new();
    // These are just amount-column bands checks, so reuse Geo::Data{row:0} (row band is ignored for
    // Schedule D — only the column band matters; see verify_schedule_d below).

    let st_active = active(&totals.st);
    let lt_active = active(&totals.lt);
    let y = map.year;

    // ★ M-1 — before a single cell is written: every box these rows carry has exactly one line.
    partition_or_refuse(by_box, &BOX_GROUPS, y)?;

    // ── Part I, top to bottom: 1b (A|G), 2 (B|H), 3 (C|I), then the net on line 7. ──
    for (cell, line, boxes, label) in [
        (&map.line1b, "line1b", G_LINE1B, "Box A/G"),
        (&map.line2, "line2", G_LINE2, "Box B/H"),
    ] {
        let g = box_group(by_box, boxes);
        if !active(&g) {
            continue; // a box group with no rows writes NOTHING — the form's normal blank
        }
        push_amount_line(
            need(cell, line, label, y)?,
            &fmt_money(g.proceeds),
            &fmt_money(g.cost_basis),
            &fmt_money(g.gain),
            &mut writes,
            &mut placements,
        );
    }
    // Line 3 (Box C **or Box I**) — the map binds it on every revision, so no `need` here.
    let st_not_reported = box_group(by_box, G_LINE3);
    if active(&st_not_reported) {
        push_amount_line(
            &map.line3,
            &fmt_money(st_not_reported.proceeds),
            &fmt_money(st_not_reported.cost_basis),
            &fmt_money(st_not_reported.gain),
            &mut writes,
            &mut placements,
        );
    }
    if st_active {
        // Line 7 (net short-term — the sum of the Part I box rows printed above).
        push_h_line(
            &map.line7_h,
            &fmt_money(totals.st.gain),
            &mut writes,
            &mut placements,
        );
    }

    // ── Part II: 8b (D|J), 9 (E|K), 10 (F|L), then the net on line 15. ──
    for (cell, line, boxes, label) in [
        (&map.line8b, "line8b", G_LINE8B, "Box D/J"),
        (&map.line9, "line9", G_LINE9, "Box E/K"),
    ] {
        let g = box_group(by_box, boxes);
        if !active(&g) {
            continue;
        }
        push_amount_line(
            need(cell, line, label, y)?,
            &fmt_money(g.proceeds),
            &fmt_money(g.cost_basis),
            &fmt_money(g.gain),
            &mut writes,
            &mut placements,
        );
    }
    let lt_not_reported = box_group(by_box, G_LINE10);
    if active(&lt_not_reported) {
        push_amount_line(
            &map.line10,
            &fmt_money(lt_not_reported.proceeds),
            &fmt_money(lt_not_reported.cost_basis),
            &fmt_money(lt_not_reported.gain),
            &mut writes,
            &mut placements,
        );
    }
    if lt_active {
        // Line 15 (net long-term — the sum of the Part II box rows printed above).
        push_h_line(
            &map.line15_h,
            &fmt_money(totals.lt.gain),
            &mut writes,
            &mut placements,
        );
    }
    if st_active || lt_active {
        // Line 16 = line 7 + line 15.
        let total = totals.st.gain + totals.lt.gain;
        push_h_line(
            &map.line16_h,
            &fmt_money(total),
            &mut writes,
            &mut placements,
        );
        // Line 17 — "Are lines 15 and 16 both gains?", read off the two lines just printed. `None`
        // means the form's OWN routing skips line 17 (line 16 a loss, or zero), so it stays blank.
        // The 2017/2024/2025 maps all carry the pair; a map without it simply does not answer.
        if let (Some(answer), Some(pair)) = (
            btctax_core::tax::printed::schedule_d_line17(totals.lt.gain, total),
            &map.line17,
        ) {
            let choice = if answer { &pair.yes } else { &pair.no };
            writes.push((
                choice.field.clone(),
                pdf::FieldValue::Check {
                    on: choice.on.clone(),
                },
            ));
            placements.push(Placement {
                fqn: choice.field.clone(),
                geo: Geo::Check,
            });
        }
    }
    // Answer the QOF question — No — on years that HAVE one (2024/2025). The 2017 Schedule D predates
    // Qualified Opportunity Funds (2019), so its map omits the field and nothing is written.
    if let Some(qof_no) = &map.qof_no {
        writes.push((
            qof_no.field.clone(),
            pdf::FieldValue::Check {
                on: qof_no.on.clone(),
            },
        ));
        placements.push(Placement {
            fqn: qof_no.field.clone(),
            geo: Geo::Check,
        });
    }

    let mut doc = pdf::load(pdf::schedule_d_pdf(map.year)?)?;
    let index = pdf::index(&pdf::collect_fields(&doc)?);
    pdf::drop_xfa_and_set_needappearances(&mut doc)?;
    pdf::apply_writes(&mut doc, &index, &writes)?;
    pdf::strip_nondeterminism(&mut doc);
    let bytes = pdf::save(&mut doc)?;

    // Read back the serialized output.
    let check = pdf::load(&bytes)?;
    let fields = pdf::collect_fields(&check)?;
    verify_schedule_d(&check, &fields, &placements, &map.table_token)?;
    Ok(bytes)
}

fn push_h_line(
    fqn: &str,
    value: &str,
    writes: &mut Vec<(String, pdf::FieldValue)>,
    placements: &mut Vec<Placement>,
) {
    writes.push((fqn.to_string(), pdf::FieldValue::Text(value.to_string())));
    placements.push(Placement {
        fqn: fqn.to_string(),
        geo: Geo::Total { col: SD_COL_H }, // single h-column amount (Total variant = "amount column")
    });
}

/// Geometric + no-unmapped read-back for Schedule D. The amount-column x-bands (d,e,g,h) are
/// re-derived from the Part I grid; each written value must land in the column its logical line
/// demands. Row/line y is not a uniform grid on Schedule D, so only the column band is geometric
/// here — enough to catch a swapped-amount-column map — plus the no-unmapped guard.
pub fn verify_schedule_d(
    doc: &lopdf::Document,
    fields: &[pdf::Field],
    placements: &[Placement],
    table_token: &str,
) -> Result<(), FormsError> {
    let bands = column_x_bands(fields, 0, table_token)?;
    let index: std::collections::HashMap<&str, &pdf::Field> =
        fields.iter().map(|f| (f.fqn.as_str(), f)).collect();
    for p in placements {
        let field = index
            .get(p.fqn.as_str())
            .ok_or_else(|| FormsError::MapFieldMissing(p.fqn.clone()))?;
        let col = match &p.geo {
            Geo::Data { col, .. } => *col,
            Geo::Total { col } => *col,
            Geo::Check => continue,
        };
        // Line 16 lives on page 2 (its own h column); skip the page-1 band check for it but keep it
        // in the no-unmapped set.
        if p.fqn.contains("Page2") {
            continue;
        }
        let cx = field
            .cx()
            .ok_or_else(|| FormsError::Geometry(format!("{}: no /Rect", p.fqn)))?;
        let band = *bands
            .get(col)
            .ok_or_else(|| FormsError::Geometry(format!("column {col} out of range")))?;
        if !in_band(cx, band) {
            return Err(FormsError::Geometry(format!(
                "{}: x-center {cx:.1} not in amount column {col} band {band:?} (mis-mapped column)",
                p.fqn
            )));
        }
    }
    no_unmapped_filled(doc, fields, placements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Every `Form8949Box` variant. Its completeness is not asserted from a hand-written list (which
    /// goes stale silently) but from [`box_index`]'s EXHAUSTIVE match plus the permutation check in
    /// [`all_boxes_is_every_variant`]: a new variant does not compile until it is given an index, and
    /// a variant missing from this array shows up as a gap in the sorted indices.
    const ALL_BOXES: [Form8949Box; 8] = [
        Form8949Box::C,
        Form8949Box::F,
        Form8949Box::I,
        Form8949Box::L,
        Form8949Box::G,
        Form8949Box::H,
        Form8949Box::J,
        Form8949Box::K,
    ];

    fn box_index(b: Form8949Box) -> usize {
        use Form8949Box as B;
        match b {
            B::C => 0,
            B::F => 1,
            B::I => 2,
            B::L => 3,
            B::G => 4,
            B::H => 5,
            B::J => 6,
            B::K => 7,
        }
    }

    #[test]
    fn all_boxes_is_every_variant() {
        let mut seen: Vec<usize> = ALL_BOXES.iter().map(|b| box_index(*b)).collect();
        seen.sort_unstable();
        assert_eq!(
            seen,
            (0..ALL_BOXES.len()).collect::<Vec<_>>(),
            "ALL_BOXES must hold each `Form8949Box` exactly once — otherwise the partition test \
             below silently stops covering a box"
        );
    }

    fn part() -> ScheduleDPart {
        ScheduleDPart {
            proceeds: dec!(100),
            cost_basis: dec!(60),
            gain: dec!(40),
        }
    }

    /// ★ spec 1099-DA R6 fold (M-1) KILL — a box carried by NO group refuses, naming it; a box
    /// carried by TWO refuses too; and the REAL [`BOX_GROUPS`] accept **every** variant.
    ///
    /// All three halves are the test. Without the last, a guard that refused everything would pass
    /// the first two; without the first two, the guard could be a no-op and still pass the last.
    #[test]
    fn a_box_in_no_group_refuses_and_the_real_groups_partition_every_box() {
        // The plant: line 3's group has lost Box C (exactly the R6 review's PLANT B).
        let dropped: [(&str, &[Form8949Box]); 6] = [
            ("line1b", G_LINE1B),
            ("line2", G_LINE2),
            ("line3", &[Form8949Box::I]),
            ("line8b", G_LINE8B),
            ("line9", G_LINE9),
            ("line10", G_LINE10),
        ];
        let c_rows = BTreeMap::from([(Form8949Box::C, part())]);
        let err = partition_or_refuse(&c_rows, &dropped, 2024)
            .expect_err("a box in no group must REFUSE, never be dropped off the page");
        let msg = err.to_string();
        assert!(
            msg.contains("Box C") && msg.contains("dropped off"),
            "the refusal names the box and the harm: {msg}"
        );

        // …and a box in TWO groups is refused as well (it would print its total twice).
        // (spelled out, not `G_LINE3`, so this half stays a two-line plant even when the real
        //  group is mutated — the third half below is what a mutated `G_LINE3` must red.)
        let doubled: [(&str, &[Form8949Box]); 6] = [
            ("line1b", G_LINE1B),
            ("line2", G_LINE2),
            ("line3", &[Form8949Box::C, Form8949Box::I]),
            ("line8b", G_LINE8B),
            ("line9", G_LINE9),
            ("line10", &[Form8949Box::F, Form8949Box::L, Form8949Box::C]),
        ];
        let msg = partition_or_refuse(&c_rows, &doubled, 2024)
            .expect_err("a box on two lines must REFUSE")
            .to_string();
        assert!(
            msg.contains("Box C") && msg.contains("printed twice on"),
            "the refusal names the box and the harm: {msg}"
        );

        // The real table: every variant lands on exactly one line.
        for b in ALL_BOXES {
            partition_or_refuse(&BTreeMap::from([(b, part())]), &BOX_GROUPS, 2024).unwrap_or_else(
                |e| panic!("the real BOX_GROUPS must carry {b:?} on exactly one line — {e}"),
            );
        }
    }
}
