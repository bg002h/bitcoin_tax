//! Known-Answer Tests for the btctax-forms engine (SP1-T1).
//!
//! The star is the **geometric, map-independent read-back**: a fill whose map swaps two columns is
//! caught because the value lands in the wrong x-band (`fault_injected_column_swap_is_red`). Every
//! fill already runs this read-back internally and fails closed; these KATs exercise both arms.

mod common;
use common::*;

use btctax_core::{Form8949Box, Form8949Part, ScheduleDPart, ScheduleDTotals};
use btctax_forms::testonly::*;
use btctax_forms::FormsError;
use sha2::{Digest, Sha256};

// ── Field-name helpers over the committed maps ───────────────────────────────────────────────────

fn f8949_map_field_names() -> Vec<String> {
    let m = Form8949Map::ty2025();
    let mut names = Vec::new();
    for p in &m.parts {
        names.push(p.box_field.clone());
        names.extend(p.boxes.values().map(|c| c.field.clone())); // spec 1099-DA T3: G/H/I, J/K/L
        names.push(p.totals.proceeds_d.clone());
        names.push(p.totals.cost_e.clone());
        names.push(p.totals.adj_g.clone());
        names.push(p.totals.gain_h.clone());
        for row in &p.rows {
            names.extend(row.iter().cloned());
        }
    }
    names
}

fn schedule_d_map_field_names() -> Vec<String> {
    let m = ScheduleDMap::ty2025();
    let a = |c: &btctax_forms::testonly::AmountCols| {
        vec![
            c.proceeds_d.clone(),
            c.cost_e.clone(),
            c.adj_g.clone(),
            c.gain_h.clone(),
        ]
    };
    let mut names = a(&m.line3);
    names.extend(a(&m.line10));
    // spec 1099-DA T4 — the per-box rows, read OFF THE MAP
    for c in [&m.line1b, &m.line2, &m.line8b, &m.line9]
        .into_iter()
        .flatten()
    {
        names.extend(a(c));
    }
    names.push(m.line7_h.clone());
    names.push(m.line15_h.clone());
    names.push(m.line16_h.clone());
    names.push(m.qof_yes.as_ref().unwrap().field.clone());
    names.push(m.qof_no.as_ref().unwrap().field.clone());
    // Part III line 17's Yes/No pair — read OFF THE MAP, never hand-listed.
    if let Some(p) = &m.line17 {
        names.push(p.yes.field.clone());
        names.push(p.no.field.clone());
    }
    names
}

fn fieldset(pdf: &[u8]) -> std::collections::HashSet<String> {
    let doc = load(pdf).unwrap();
    collect_fields(&doc)
        .unwrap()
        .into_iter()
        .map(|f| f.fqn)
        .collect()
}

// ── XFA (★ R0-C1) ────────────────────────────────────────────────────────────────────────────────

#[test]
fn output_has_no_xfa() {
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    assert!(
        !pdf_has_xfa(&doc).unwrap(),
        "the /XFA layer must be removed"
    );
    let sd = btctax_forms::fill_schedule_d(
        &totals_for(&mixed_rows()),
        &btctax_core::schedule_d_by_box(&mixed_rows()),
        2025,
    )
    .unwrap();
    assert!(!pdf_has_xfa(&load(&sd).unwrap()).unwrap());
}

#[test]
fn filled_value_persists_after_xfa_drop() {
    // A /V-only fill on an XFA hybrid opens blank in Acrobat; after dropping /XFA the classic
    // /V must still be present on reload.
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    // Row 1, column (a) description of the first short-term row.
    let a1 = &idx["topmostSubform[0].Page1[0].Table_Line1_Part1[0].Row1[0].f1_03[0]"];
    assert_eq!(text_value(&doc, a1.id).as_deref(), Some("0.53000000 BTC"));
}

// ── Digital-asset box (★ R0-C2): Box I / Box L, NOT C / F ────────────────────────────────────────

#[test]
fn ty2025_bitcoin_uses_box_i_and_l() {
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    // Box I (short-term digital assets) = c1_1[5] on-state /6.
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[5]"].id).as_deref(),
        Some("6"),
        "Box I must be checked for short-term BTC"
    );
    // Box L (long-term) = c2_1[5] on-state /6.
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_1[5]"].id).as_deref(),
        Some("6"),
        "Box L must be checked for long-term BTC"
    );
    // Box C ("other than digital asset transactions") and Box F must NOT be checked.
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[2]"].id),
        None,
        "Box C must stay OFF — BTC is a digital asset"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_1[2]"].id),
        None,
        "Box F must stay OFF"
    );
}

// ── Geometric read-back (★ R0-I3) ────────────────────────────────────────────────────────────────

#[test]
fn filled_values_land_in_expected_geometry() {
    // The fill's INTERNAL geometric read-back already passed (fill returned Ok). Additionally show a
    // concrete value landed in its geometrically-correct column: row-1 proceeds (col d) must sit in
    // the (d) x-band 273.6..337.65.
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    let d1 = &idx["topmostSubform[0].Page1[0].Table_Line1_Part1[0].Row1[0].f1_06[0]"];
    assert_eq!(text_value(&doc, d1.id).as_deref(), Some("30000.50"));
    let cx = d1.cx().unwrap();
    assert!(
        (273.6..=337.65).contains(&cx),
        "row-1 proceeds x-center {cx} must be in column (d) band"
    );
}

#[test]
fn fault_injected_column_swap_is_red() {
    // ★ Corrupt the map — swap the (d) proceeds and (e) cost columns for every short-term row — and
    // confirm the geometric read-back catches it (fails closed). This is the core safety property:
    // the map is what we distrust.
    let rows = mixed_rows();
    let (st, lt) = split_parts(&rows);
    let short = part_data(&st).unwrap();
    let long = part_data(&lt).unwrap();

    let mut map = Form8949Map::ty2025();
    let sp = map.parts.iter_mut().find(|p| p.term == "short").unwrap();
    for row in &mut sp.rows {
        row.swap(3, 4); // swap columns (d) and (e)
    }
    let err = fill_8949_parts(&short, &long, &map).unwrap_err();
    assert!(
        matches!(err, FormsError::Geometry(_)),
        "swapped columns must fail the geometric read-back, got {err:?}"
    );
}

#[test]
fn no_unmapped_field_filled() {
    // Positive: after a correct fill, every field that carries a value is a name the maps authorize.
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let allowed: std::collections::HashSet<String> = f8949_map_field_names().into_iter().collect();
    for f in &fields {
        let filled = if f.is_button {
            checkbox_on(&doc, f.id).is_some()
        } else {
            text_value(&doc, f.id).is_some_and(|s| !s.is_empty())
        };
        if filled {
            assert!(
                allowed.contains(&f.fqn),
                "unexpected filled field outside the map: {}",
                f.fqn
            );
        }
    }

    // Negative: the guard fires when a filled field is not authorized (empty placement set).
    let err = no_unmapped_filled(&doc, &fields, &[]).unwrap_err();
    assert!(matches!(err, FormsError::UnmappedField(_)), "got {err:?}");
}

// ── Map ↔ PDF coverage ───────────────────────────────────────────────────────────────────────────

#[test]
fn map_2025_matches_bundled_pdf_fieldset() {
    let set = fieldset(
        btctax_forms::bundled::template(btctax_forms::bundled::Stem::F8949, 2025).unwrap(),
    );
    for name in f8949_map_field_names() {
        assert!(
            set.contains(&name),
            "8949 map field absent from PDF: {name}"
        );
    }
    let sd_set = fieldset(
        btctax_forms::bundled::template(btctax_forms::bundled::Stem::ScheduleD, 2025).unwrap(),
    );
    for name in schedule_d_map_field_names() {
        assert!(
            sd_set.contains(&name),
            "schedule_d map field absent from PDF: {name}"
        );
    }
}

#[test]
fn rows_per_page_is_map_data() {
    let m = Form8949Map::ty2025();
    assert_eq!(m.rows_per_page, 11, "TY2025 8949 is 11 rows/part/page");
    for p in &m.parts {
        assert_eq!(
            p.rows.len(),
            m.rows_per_page,
            "part {} must enumerate exactly {} rows",
            p.term,
            m.rows_per_page
        );
        for row in &p.rows {
            assert_eq!(row.len(), 8, "each 8949 data row has 8 columns (a..h)");
        }
    }
}

// ── Determinism (golden) ─────────────────────────────────────────────────────────────────────────

#[test]
fn fill_is_byte_deterministic() {
    let rows = mixed_rows();
    let a = btctax_forms::fill_form_8949(&rows, 2025).unwrap();
    let b = btctax_forms::fill_form_8949(&rows, 2025).unwrap();
    assert_eq!(a, b, "same (data, form) must produce byte-identical output");

    // Golden: pin the content hash of the canonical fixture (lopdf 0.36.0, Cargo.lock-pinned).
    let hash = hex(&Sha256::digest(&a));
    assert_eq!(
        hash, GOLDEN_F8949_SHA256,
        "fill output changed — if intentional, update GOLDEN_F8949_SHA256"
    );
}

const GOLDEN_F8949_SHA256: &str =
    "d981a64548a9a2971ed35cd953e1dbda643665242e252cf3721012800b580906";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ── Schedule D ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn schedule_d_fills_3_7_10_15_16_and_qof() {
    let totals = ScheduleDTotals {
        st: ScheduleDPart {
            proceeds: rust_decimal_macros::dec!(36000.50),
            cost_basis: rust_decimal_macros::dec!(30500),
            gain: rust_decimal_macros::dec!(5500.50),
        },
        lt: ScheduleDPart {
            proceeds: rust_decimal_macros::dec!(60000),
            cost_basis: rust_decimal_macros::dec!(20000),
            gain: rust_decimal_macros::dec!(40000),
        },
    };
    let bytes =
        btctax_forms::fill_schedule_d(&totals, &not_reported_by_box(&totals), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    let v = |fqn: &str| text_value(&doc, idx[fqn].id);
    // Line 3 (ST total) d/e/h.
    assert_eq!(
        v("topmostSubform[0].Page1[0].Table_PartI[0].Row3[0].f1_15[0]").as_deref(),
        Some("36000.50")
    );
    assert_eq!(
        v("topmostSubform[0].Page1[0].Table_PartI[0].Row3[0].f1_18[0]").as_deref(),
        Some("5500.50")
    );
    // Line 7 net ST.
    assert_eq!(
        v("topmostSubform[0].Page1[0].f1_22[0]").as_deref(),
        Some("5500.50")
    );
    // Line 10 (LT total) d/h.
    assert_eq!(
        v("topmostSubform[0].Page1[0].Table_PartII[0].Row10[0].f1_35[0]").as_deref(),
        Some("60000")
    );
    // Line 15 net LT.
    assert_eq!(
        v("topmostSubform[0].Page1[0].f1_43[0]").as_deref(),
        Some("40000")
    );
    // Line 16 = 7 + 15.
    assert_eq!(
        v("topmostSubform[0].Page2[0].f2_1[0]").as_deref(),
        Some("45500.50")
    );
    // QOF answered No (c1_1[1], on-state /2); Yes must be off.
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[1]"].id).as_deref(),
        Some("2")
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[0]"].id),
        None
    );
    // Line 17 = Yes (both 15 and 16 are gains). See `schedule_d_line17_is_derived_on_every_revision`
    // for the whole routing table; this pins that the flagship KAT's own form carries the answer.
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_1[0]"].id).as_deref(),
        Some("1")
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_1[1]"].id),
        None
    );
}

/// Schedule D **line 17** — *"Are lines 15 and 16 both gains?"* — is DERIVED from the two lines
/// printed above it, on every revision the crypto slice supports, and is left BLANK in exactly the
/// two cases the form's own routing skips it (line 16 a loss; line 16 zero).
///
/// ★★ **The on-states are DUMPED, not assumed, and they DIFFER between revisions**: 2024/2025 use
/// `"1"`/`"2"` while **2017 uses `"Yes"`/`"No"`**. Asserting the literal per-year on-state here is what
/// makes an on-state copied by analogy fail — a wrong on-state writes an OFF box, i.e. an unanswered
/// line 17 that looks filled to any test that only checks "some value was written".
///
/// ★ Lines 18-22 are deliberately NOT answered, and that is asserted too: line 20's *"…and you are
/// not filing Form 4952?"* is a fact no btctax input carries, so a Yes there would be testimony the
/// filer never gave.
#[test]
fn schedule_d_line17_is_derived_on_every_revision() {
    use rust_decimal_macros::dec;
    let part = |proceeds: rust_decimal::Decimal,
                cost: rust_decimal::Decimal,
                gain: rust_decimal::Decimal| ScheduleDPart {
        proceeds,
        cost_basis: cost,
        gain,
    };
    // (label, ST part, LT part, expected line-17 answer — `None` ⇒ the form skips line 17).
    let cases: [(&str, ScheduleDPart, ScheduleDPart, Option<bool>); 5] = [
        (
            "both gains ⇒ Yes",
            part(dec!(36000), dec!(30500), dec!(5500)),
            part(dec!(60000), dec!(20000), dec!(40000)),
            Some(true),
        ),
        (
            "L16 gain, L15 LOSS ⇒ No",
            part(dec!(80000), dec!(30000), dec!(50000)),
            part(dec!(10000), dec!(20000), dec!(-10000)),
            Some(false),
        ),
        (
            "L15 printed as an active ZERO — zero is not a gain ⇒ No",
            part(dec!(80000), dec!(30000), dec!(50000)),
            part(dec!(20000), dec!(20000), dec!(0)),
            Some(false),
        ),
        (
            "L16 is a LOSS — the form says skip 17 through 20 ⇒ blank",
            part(dec!(10000), dec!(60000), dec!(-50000)),
            part(dec!(30000), dec!(20000), dec!(10000)),
            None,
        ),
        (
            "L16 is ZERO — the form says skip 17 through 21 ⇒ blank",
            part(dec!(40000), dec!(30000), dec!(10000)),
            part(dec!(10000), dec!(20000), dec!(-10000)),
            None,
        ),
    ];

    // The per-revision Yes/No on-states, DUMPED from each bundled PDF with `xtask dump-fields`.
    let expected_on = |year: i32, yes: bool| -> &'static str {
        match (year, yes) {
            (2017, true) => "Yes",
            (2017, false) => "No",
            (_, true) => "1",
            (_, false) => "2",
        }
    };

    for year in [2017, 2024, 2025] {
        let map = ScheduleDMap::for_year(year).unwrap();
        let pair = map
            .line17
            .as_ref()
            .unwrap_or_else(|| panic!("the {year} Schedule D map must carry line 17"));
        for (label, st, lt, expected) in &cases {
            let totals = ScheduleDTotals { st: *st, lt: *lt };
            let bytes = btctax_forms::fill_schedule_d(&totals, &not_reported_by_box(&totals), year)
                .unwrap();
            let doc = load(&bytes).unwrap();
            let idx = index(&collect_fields(&doc).unwrap());
            let yes = checkbox_on(&doc, idx[pair.yes.field.as_str()].id);
            let no = checkbox_on(&doc, idx[pair.no.field.as_str()].id);
            match expected {
                Some(true) => {
                    assert_eq!(
                        yes.as_deref(),
                        Some(expected_on(year, true)),
                        "{year} {label}: line 17 Yes must carry this revision's OWN on-state"
                    );
                    assert_eq!(no, None, "{year} {label}: line 17 No must be off");
                }
                Some(false) => {
                    assert_eq!(
                        no.as_deref(),
                        Some(expected_on(year, false)),
                        "{year} {label}: line 17 No must carry this revision's OWN on-state"
                    );
                    assert_eq!(yes, None, "{year} {label}: line 17 Yes must be off");
                }
                None => {
                    assert_eq!(
                        (yes, no),
                        (None, None),
                        "{year} {label}: the form's own routing SKIPS line 17 — a checked box here \
                         is an answer to a question the form never asked"
                    );
                }
            }
        }
    }
}

/// ★★★ B1 (`design/HARNESS.md`) — THE PLANTED DEFECT, and the reason `apply_writes` now checks the
/// widget's own `/AP` `/N` keys.
///
/// A checkbox renders from its appearance dictionary. Write an `/AS` with no matching key and the box
/// draws **NOTHING** — blank on every filed copy — while `/V` and `/AS` read back as exactly the value
/// you asked for. It is the second row of CLAUDE.md's provenance table (*"nothing ever populated it"*)
/// wearing the costume of the first (*"the inputs say so"*): invisible on the page, invisible to both
/// oracles, and invisible to any test that reads the field value back.
///
/// It is reachable from ONE transposition — a map whose `yes`/`no` FIELD names are swapped while the
/// on-states stay put. That is finding G-19b of the 2026-07-30 review, which reached it by mutating
/// the 2025 map and watching `schedule_d_line17_is_derived_on_every_revision` stay GREEN.
///
/// This test plants that exact mutation and asserts the fill now FAILS CLOSED. It is the answer to
/// B1's one reviewable question — *which test reds when this checker is removed?* — this one.
#[test]
fn a_swapped_yes_no_map_fails_closed_instead_of_rendering_a_blank_box() {
    let totals = totals_for(&mixed_rows()); // both gains ⇒ line 17 is answered
    let by_box = btctax_core::schedule_d_by_box(&mixed_rows());
    for year in [2017, 2024, 2025] {
        let mut map = ScheduleDMap::for_year(year).unwrap();
        let pair = map.line17.as_mut().unwrap();
        // Swap ONLY the field names. Each on-state stays with its original index, so "Yes" is now
        // written to the widget that can only render "No" — and vice versa.
        std::mem::swap(&mut pair.yes.field, &mut pair.no.field);
        // `expect_err` would print the whole PDF on failure; name the outcome instead.
        let msg = match fill_schedule_d_totals(&totals, &by_box, &map) {
            Ok(_) => panic!(
                "{year}: a swapped Yes/No map FILLED. It writes an on-state the widget cannot \
                 render, so line 17 comes out BLANK on the filed form while reading back as checked."
            ),
            Err(e) => format!("{e}"),
        };
        assert!(
            msg.contains("is not one this widget declares"),
            "{year}: the refusal must name the on-state mismatch, got: {msg}"
        );
    }

    // ★ The other half of the guarantee: the UNSWAPPED map still fills. A guard that rejected
    // everything would also pass the assertions above.
    for year in [2017, 2024, 2025] {
        let map = ScheduleDMap::for_year(year).unwrap();
        fill_schedule_d_totals(&totals, &by_box, &map)
            .unwrap_or_else(|e| panic!("{year}: the correct map must still fill — {e}"));
    }
}

/// The crypto slice answers line 17 and **stops**: lines 18-22 stay blank on every revision, because
/// each needs a fact it never collects (18/19 were never asked, 20 also asks about Form 4952, 21
/// needs the §1211 ceiling by filing status, 22 needs Form 1040 line 3a).
#[test]
fn schedule_d_crypto_slice_leaves_lines_18_through_22_blank() {
    let totals = totals_for(&mixed_rows()); // both gains — the branch that reaches line 18
    for year in [2017, 2024, 2025] {
        let bytes = btctax_forms::fill_schedule_d(
            &totals,
            &btctax_core::schedule_d_by_box(&mixed_rows()),
            year,
        )
        .unwrap();
        let doc = load(&bytes).unwrap();
        let fields = collect_fields(&doc).unwrap();
        let map = ScheduleDMap::for_year(year).unwrap();
        let line17 = map.line17.as_ref().unwrap();
        // Every page-2 field EXCEPT line 16's amount and line 17's own pair must be untouched.
        let allowed = [
            map.line16_h.as_str(),
            line17.yes.field.as_str(),
            line17.no.field.as_str(),
        ];
        for f in fields.iter().filter(|f| f.fqn.contains("Page2")) {
            if allowed.contains(&f.fqn.as_str()) {
                continue;
            }
            let idx = index(&collect_fields(&doc).unwrap());
            assert_eq!(
                text_value(&doc, idx[f.fqn.as_str()].id),
                None,
                "{year}: Schedule D {} carries a value — the crypto slice must stop after line 17",
                f.fqn
            );
            assert_eq!(
                checkbox_on(&doc, idx[f.fqn.as_str()].id),
                None,
                "{year}: Schedule D {} is checked — the crypto slice must stop after line 17",
                f.fqn
            );
        }
    }
}

/// Every text value of a merged Form 8949 whose fully-qualified name ends in `suffix` — one per
/// COPY, because `overflow::merge_copies` uniquifies each copy by renaming only the root component.
fn values_ending(pdf: &[u8], suffix: &str) -> Vec<String> {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    fields
        .iter()
        .filter(|f| f.fqn.ends_with(suffix))
        .filter_map(|f| text_value(&doc, f.id))
        .collect()
}

/// A short-term **G + I** mix plus one long-term **L** row: two Part I box groups, so Form 8949
/// prints one page-set per box and Schedule D must carry each on its OWN line (spec 1099-DA T8).
fn g_and_i_rows() -> Vec<btctax_core::Form8949Row> {
    let mut rows = vec![
        row(
            Form8949Part::ShortTerm,
            "0.53000000 BTC",
            rust_decimal_macros::dec!(30000.50),
            rust_decimal_macros::dec!(25000),
            true,
        ),
        row(
            Form8949Part::ShortTerm,
            "0.10000000 BTC",
            rust_decimal_macros::dec!(6000),
            rust_decimal_macros::dec!(5500),
            false,
        ),
        row(
            Form8949Part::LongTerm,
            "1.00000000 BTC",
            rust_decimal_macros::dec!(60000),
            rust_decimal_macros::dec!(20000),
            false,
        ),
    ];
    // The exchange row answered `basis_matches` routes to G; the self-custody row keeps I.
    rows[0].box_ = btctax_core::Form8949Box::G;
    rows
}

/// ★ spec 1099-DA T8 — the PDF-vs-8949 half, PER BOX GROUP. (Renamed from
/// `schedule_d_totals_match_form8949_and_csv`, which never opened a CSV: the third artifact was a
/// `Decimal::to_string()` of the same in-memory total, so the "and csv" in its name asserted
/// nothing. The real three-artifact cross-check lives in `btctax-cli`, where the CSV is written —
/// spec 1099-DA T8, I-9/I-13.)
///
/// Each Schedule D box line's column (d) must equal that box's Form 8949 page-set total, and the
/// part total must equal the sum of the groups.
#[test]
fn schedule_d_box_lines_match_the_form8949_page_set_totals() {
    let rows = g_and_i_rows();
    let totals = totals_for(&rows);
    let by_box = btctax_core::schedule_d_by_box(&rows);

    let sd = btctax_forms::fill_schedule_d(&totals, &by_box, 2025).unwrap();
    let sdoc = load(&sd).unwrap();
    let sidx = index(&collect_fields(&sdoc).unwrap());
    let cell = |fqn: &str| text_value(&sdoc, sidx[fqn].id);
    let map = ScheduleDMap::ty2025();

    // ── Schedule D: 1b = the G group, 3 = the I group, 10 = the L group. Read OFF THE MAP. ──
    let g = by_box[&btctax_core::Form8949Box::G];
    let i = by_box[&btctax_core::Form8949Box::I];
    let l = by_box[&btctax_core::Form8949Box::L];
    assert_eq!(
        cell(&map.line1b.as_ref().unwrap().proceeds_d).as_deref(),
        Some(g.proceeds.to_string().as_str()),
        "line 1b (d) = the Box G page-set's proceeds"
    );
    assert_eq!(
        cell(&map.line3.proceeds_d).as_deref(),
        Some(i.proceeds.to_string().as_str()),
        "line 3 (d) = the Box I page-set's proceeds"
    );
    assert_eq!(
        cell(&map.line10.proceeds_d).as_deref(),
        Some(l.proceeds.to_string().as_str()),
        "line 10 (d) = the Box L page-set's proceeds"
    );

    // ── …and each equals the total the 8949 PAGE-SET for that box prints. ──
    let f8949 = btctax_forms::fill_form_8949(&rows, 2025).unwrap();
    let mut part_i_totals = values_ending(&f8949, "Page1[0].f1_91[0]");
    part_i_totals.sort();
    let mut want = vec![g.proceeds.to_string(), i.proceeds.to_string()];
    want.sort();
    assert_eq!(
        part_i_totals, want,
        "one Part I page-set per box, each totalling its OWN rows"
    );

    // ── The part total cross-foots: 1b(d) + 3(d) = the Part I total. ──
    assert_eq!(
        g.proceeds + i.proceeds,
        totals.st.proceeds,
        "the box groups partition Part I"
    );
    assert_eq!(
        totals.st.proceeds,
        sum_part(&rows, Form8949Part::ShortTerm).proceeds
    );
}

/// ★ spec 1099-DA T8 KILL — a live-regime `basis_matches` slice: the G total lands on line **1b**
/// and line **3 is BLANK**. Before T8 the whole Part I total went to line 3 whatever box the rows
/// carried, which put a broker-reported total under the schedule's *"Box C or Box I"* heading — a
/// filed page asserting the transactions were NOT reported to the IRS when they were.
#[test]
fn a_basis_matches_slice_puts_the_g_total_on_line_1b_and_leaves_line_3_blank() {
    let mut rows = g_and_i_rows();
    rows.remove(1); // drop the self-custody I row: Part I is G-only
    let totals = totals_for(&rows);
    let by_box = btctax_core::schedule_d_by_box(&rows);
    let sd = btctax_forms::fill_schedule_d(&totals, &by_box, 2025).unwrap();
    let doc = load(&sd).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    let map = ScheduleDMap::ty2025();
    let l1b = map.line1b.as_ref().unwrap();
    assert_eq!(
        text_value(&doc, idx[l1b.proceeds_d.as_str()].id).as_deref(),
        Some(totals.st.proceeds.to_string().as_str()),
        "the G total belongs on line 1b (\"Box A or Box G checked\")"
    );
    assert_eq!(
        text_value(&doc, idx[map.line3.proceeds_d.as_str()].id),
        None,
        "line 3 is \"Box C or Box I\" — with no I rows it must be BLANK, not zero"
    );
    assert_eq!(
        text_value(&doc, idx[map.line3.gain_h.as_str()].id),
        None,
        "…in every column"
    );
    // The Form 8949 behind it carries a G page-set.
    let f8949 = btctax_forms::fill_form_8949(&rows, 2025).unwrap();
    let fdoc = load(&f8949).unwrap();
    let fidx = index(&collect_fields(&fdoc).unwrap());
    let part_i = Form8949Map::ty2025();
    let st = part_i.parts.iter().find(|p| p.term == "short").unwrap();
    let g_cell = st.boxes.get("G").expect("the 2025 map binds Box G");
    assert!(
        checkbox_on(&fdoc, fidx[g_cell.field.as_str()].id).is_some(),
        "the page-set behind line 1b is boxed G"
    );
}

/// ★ spec 1099-DA T8 KILL — a **TY2024** slice still puts its whole Part I total on line 3 and its
/// whole Part II total on line 10, and leaves 1b blank. The pre-2025 revisions have no digital-asset
/// boxes at all (every row is C/F), and line 3 reads *"Box C checked"* there (I-6).
///
/// ★★ The fixture is RE-BOXED to **C/F**, not `mixed_rows()`'s TY2025 I/L defaults, and that is the
/// whole point of the test (R6 build review **I-2**). With I/L rows the `2024` argument selects only
/// the map REVISION, so this test proved the 2024 map binds line 3 — never that a TY2024 slice's Box
/// **C** total reaches it. Both pre-2025 halves of the pairing were consequently unkilled: deleting
/// `B::C` from line 3's group, and separately `B::F` from line 10's, each left the whole suite green.
#[test]
fn a_pre_2025_slice_keeps_the_whole_part_i_total_on_line_3() {
    // The pre-2025 not-reported boxes: C (short-term) and F (long-term).
    let rows: Vec<_> = mixed_rows()
        .into_iter()
        .map(|mut r| {
            r.box_ = if r.part == Form8949Part::ShortTerm {
                Form8949Box::C
            } else {
                Form8949Box::F
            };
            r
        })
        .collect();
    let totals = totals_for(&rows);
    let by_box = btctax_core::schedule_d_by_box(&rows);
    let sd = btctax_forms::fill_schedule_d(&totals, &by_box, 2024).unwrap();
    let doc = load(&sd).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    let map = ScheduleDMap::ty2024();
    assert_eq!(
        text_value(&doc, idx[map.line3.proceeds_d.as_str()].id).as_deref(),
        Some(totals.st.proceeds.to_string().as_str()),
        "the WHOLE Part I total (Box C) on line 3"
    );
    assert_eq!(
        text_value(&doc, idx[map.line10.proceeds_d.as_str()].id).as_deref(),
        Some(totals.lt.proceeds.to_string().as_str()),
        "the WHOLE Part II total (Box F) on line 10"
    );
    assert_eq!(
        text_value(
            &doc,
            idx[map.line1b.as_ref().unwrap().proceeds_d.as_str()].id
        ),
        None,
        "line 1b is BLANK on a slice with no Box G rows"
    );
}

/// ★★★ spec 1099-DA T8 KILL (B1) — an UNBOUND per-box row with rows to print REFUSES; the same map
/// with no rows for that box FILLS CLEAN.
///
/// Both halves are the test. Without the second, a filler that refused everything would pass the
/// first; without the first, a filler that silently dropped the G total off the page would pass the
/// second — and a dropped total is invisible on the filed page, which is the whole class this
/// guards (`CLAUDE.md`'s provenance table, row 2).
#[test]
fn an_unbound_box_row_with_rows_to_print_refuses_and_without_them_fills() {
    let mut map = ScheduleDMap::ty2025();
    map.line1b = None; // the plant: this revision no longer binds line 1b

    let g_rows = g_and_i_rows();
    let err = fill_schedule_d_totals(
        &totals_for(&g_rows),
        &btctax_core::schedule_d_by_box(&g_rows),
        &map,
    )
    .expect_err("a routed G group with no line-1b cells must REFUSE, never drop the total");
    let msg = format!("{err}");
    assert!(
        msg.contains("line1b") && msg.contains("Box A/G"),
        "the refusal names the unbound row and the box whose total needs it: {msg}"
    );

    // …and the SAME map fills clean when no G rows exist.
    let plain = mixed_rows();
    fill_schedule_d_totals(
        &totals_for(&plain),
        &btctax_core::schedule_d_by_box(&plain),
        &map,
    )
    .expect("no Box G rows ⇒ line 1b is simply blank, and the fill proceeds");
}

#[test]
fn schedule_d_line3_10_accept_i_l() {
    // Schedule D line 3 text is "Box C or Box I" and line 10 "Box F or Box L" — the Box I/L 8949
    // totals flow onto exactly these lines. Confirm both are populated from an I/L (digital-asset)
    // fill with no error (the geometric read-back accepts them).
    let totals = totals_for(&mixed_rows());
    let bytes = btctax_forms::fill_schedule_d(
        &totals,
        &btctax_core::schedule_d_by_box(&mixed_rows()),
        2025,
    )
    .unwrap();
    let doc = load(&bytes).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    assert!(text_value(
        &doc,
        idx["topmostSubform[0].Page1[0].Table_PartI[0].Row3[0].f1_15[0]"].id
    )
    .is_some());
    assert!(text_value(
        &doc,
        idx["topmostSubform[0].Page1[0].Table_PartII[0].Row10[0].f1_35[0]"].id
    )
    .is_some());
}

// ── [I5] broker-reported advisory ────────────────────────────────────────────────────────────────

#[test]
fn rows_possibly_broker_reported_counts_exchange_rows() {
    // The mixed fixture has one exchange (box_needs_review) row.
    assert_eq!(
        btctax_forms::rows_possibly_broker_reported(&mixed_rows()),
        1
    );
}

// ── Year guard ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn unsupported_year_is_rejected() {
    // 2023 is not bundled (this build ships 2024 + 2025).
    let err = btctax_forms::fill_form_8949(&mixed_rows(), 2023).unwrap_err();
    assert!(
        matches!(err, FormsError::UnsupportedYear(2023)),
        "got {err:?}"
    );
}
