//! Known-Answer Tests for the btctax-forms engine (SP1-T1).
//!
//! The star is the **geometric, map-independent read-back**: a fill whose map swaps two columns is
//! caught because the value lands in the wrong x-band (`fault_injected_column_swap_is_red`). Every
//! fill already runs this read-back internally and fails closed; these KATs exercise both arms.

mod common;
use common::*;

use btctax_core::{Form8949Part, ScheduleDPart, ScheduleDTotals};
use btctax_forms::testonly::*;
use btctax_forms::FormsError;
use sha2::{Digest, Sha256};

// ── Field-name helpers over the committed maps ───────────────────────────────────────────────────

fn f8949_map_field_names() -> Vec<String> {
    let m = Form8949Map::ty2025();
    let mut names = Vec::new();
    for p in &m.parts {
        names.push(p.box_field.clone());
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
    names.push(m.line7_h.clone());
    names.push(m.line15_h.clone());
    names.push(m.line16_h.clone());
    names.push(m.qof_yes.as_ref().unwrap().field.clone());
    names.push(m.qof_no.as_ref().unwrap().field.clone());
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
    let sd = btctax_forms::fill_schedule_d(&totals_for(&mixed_rows()), 2025).unwrap();
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
    let set = fieldset(F8949_PDF_2025);
    for name in f8949_map_field_names() {
        assert!(
            set.contains(&name),
            "8949 map field absent from PDF: {name}"
        );
    }
    let sd_set = fieldset(SCHEDULE_D_PDF_2025);
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
    let bytes = btctax_forms::fill_schedule_d(&totals, 2025).unwrap();
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
}

#[test]
fn schedule_d_totals_match_form8949_and_csv() {
    // The Form 8949 per-part totals, the Schedule D line 3/10 amounts, and the CSV values are all the
    // exact `Decimal::to_string()` of the same summed legs — so they must be byte-identical.
    let rows = mixed_rows();
    let totals = totals_for(&rows);

    // Schedule D line 3 = ST total; line 10 = LT total.
    let sd = btctax_forms::fill_schedule_d(&totals, 2025).unwrap();
    let sdoc = load(&sd).unwrap();
    let sidx = index(&collect_fields(&sdoc).unwrap());
    let sd_line3_proceeds = text_value(
        &sdoc,
        sidx["topmostSubform[0].Page1[0].Table_PartI[0].Row3[0].f1_15[0]"].id,
    );

    // Form 8949 Part I totals row (f1_91 = proceeds).
    let f8949 = btctax_forms::fill_form_8949(&rows, 2025).unwrap();
    let fdoc = load(&f8949).unwrap();
    let fidx = index(&collect_fields(&fdoc).unwrap());
    let f8949_st_proceeds = text_value(&fdoc, fidx["topmostSubform[0].Page1[0].f1_91[0]"].id);

    // The "CSV" value is the identical Decimal Display of the same total.
    let csv_st_proceeds = totals.st.proceeds.to_string();

    assert_eq!(sd_line3_proceeds.as_deref(), Some(csv_st_proceeds.as_str()));
    assert_eq!(f8949_st_proceeds.as_deref(), Some(csv_st_proceeds.as_str()));
    // And the ST total equals the sum of the ST rows (no drift between the two projections).
    assert_eq!(
        totals.st.proceeds,
        sum_part(&rows, Form8949Part::ShortTerm).proceeds
    );
}

#[test]
fn schedule_d_line3_10_accept_i_l() {
    // Schedule D line 3 text is "Box C or Box I" and line 10 "Box F or Box L" — the Box I/L 8949
    // totals flow onto exactly these lines. Confirm both are populated from an I/L (digital-asset)
    // fill with no error (the geometric read-back accepts them).
    let totals = totals_for(&mixed_rows());
    let bytes = btctax_forms::fill_schedule_d(&totals, 2025).unwrap();
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
