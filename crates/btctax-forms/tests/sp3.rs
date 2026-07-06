//! SP3a Known-Answer Tests: the TY2024 full packet + the engine changes (Box C/F, the per-year 8949
//! grid token, the DA-question **adjacency** oracle) — all with the SAME geometric, map-independent
//! read-back that fails closed. The 2025 suite (kats.rs / sp2.rs) is the regression guard.

mod common;
use common::*;

use btctax_core::{
    DonationDetails, Form8283HowAcquired, Form8283Row, Form8283Section, ScheduleDPart,
    ScheduleDTotals, SeTaxResult, Usd,
};
use btctax_forms::testonly::*;
use btctax_forms::{Form1040Inputs, FormsError};
use rust_decimal_macros::dec;
use sha2::{Digest, Sha256};
use time::macros::date;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn fields_of(pdf: &[u8]) -> (lopdf::Document, Vec<Field>) {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    (doc, fields)
}
fn fieldset(pdf: &[u8]) -> std::collections::HashSet<String> {
    collect_fields(&load(pdf).unwrap())
        .unwrap()
        .into_iter()
        .map(|f| f.fqn)
        .collect()
}
fn tv(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields.iter().find(|f| f.fqn == fqn)?;
    text_value(doc, f.id)
}
fn cb(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields.iter().find(|f| f.fqn == fqn)?;
    checkbox_on(doc, f.id)
}

const WAGE_BASE_2024: Usd = dec!(168600);

// ── ★ Box C/F (NOT I/L) — the pre-1099-DA revision ───────────────────────────────────────────────

#[test]
fn ty2024_bitcoin_uses_box_c_f() {
    // ★ 2024 files BTC under Box C (short-term) / Box F (long-term), on-state /3 — NOT Box I/L.
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2024).unwrap();
    let (doc, fields) = fields_of(&bytes);
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[2]").as_deref(),
        Some("3"),
        "Box C (short-term) must be /3"
    );
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page2[0].c2_1[2]").as_deref(),
        Some("3"),
        "Box F (long-term) must be /3"
    );
    // Box A/B (the 1099-B boxes) must stay off.
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[0]"),
        None
    );
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[1]"),
        None
    );
}

#[test]
fn ty2025_still_i_l() {
    // Regression: the 2025 (1099-DA) revision still files BTC under Box I/L (c1_1[5]/c2_1[5] on /6).
    let bytes = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let (doc, fields) = fields_of(&bytes);
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[5]").as_deref(),
        Some("6")
    );
    // Box C on the 2025 form must NOT be checked.
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[2]"),
        None
    );
}

#[test]
fn ty2024_8949_14_rows() {
    // ★ 2024 grid holds 14 rows/part (vs 11 in 2025) — map DATA, and the per-year grid token.
    let m = Form8949Map::ty2024();
    assert_eq!(m.rows_per_page, 14);
    assert_eq!(m.table_token, "Table_Line1");
    for p in &m.parts {
        assert_eq!(p.rows.len(), 14, "part {} must enumerate 14 rows", p.term);
        for row in &p.rows {
            assert_eq!(row.len(), 8);
        }
    }
    // A 14-row Part I fills without overflow; a 15th would paginate.
    let mut rows = Vec::new();
    for i in 0..14u32 {
        rows.push(row(
            btctax_core::Form8949Part::ShortTerm,
            &format!("{i}.00000000 BTC"),
            dec!(100),
            dec!(50),
            false,
        ));
    }
    let bytes = btctax_forms::fill_form_8949(&rows, 2024).unwrap();
    assert_eq!(
        load(&bytes).unwrap().get_pages().len(),
        2,
        "14 rows = 1 copy"
    );
}

// ── ★ [R0-C2] DA-question oracle: select by horizontal ADJACENCY, not top-most-y ─────────────────

#[test]
fn da_pair_selected_by_adjacency_not_topmost() {
    // ★ On the 2024 1040 the TOP-MOST same-y {/1,/2} /Btn row is the FILING-STATUS row
    // (Single c1_3[0] @ x≈107 vs MFJ c1_3[0] @ x≈373, ~266pt apart) — NOT the DA pair. The adjacency
    // oracle must skip it and return the DA pair (c1_5[0]/c1_5[1], ~36pt apart).
    let (doc, fields) = fields_of(F1040_PDF_2024);
    let (yes, no) = topmost_yes_no_pair(&doc, &fields, 0).unwrap();
    assert_eq!(
        yes, "topmostSubform[0].Page1[0].c1_5[0]",
        "DA 'Yes' must be the adjacent pair's LEFT box, not the filing-status row"
    );
    assert_eq!(no, "topmostSubform[0].Page1[0].c1_5[1]");
    // Explicitly: the filing-status boxes were NOT chosen.
    assert_ne!(
        yes,
        "topmostSubform[0].Page1[0].FilingStatus_ReadOrder[0].c1_3[0]"
    );
    assert_ne!(no, "topmostSubform[0].Page1[0].c1_3[0]");
}

#[test]
fn ty2025_da_still_correct() {
    // Regression: on the 2025 1040 the DA pair (c1_10) is BOTH the top-most {/1,/2} 2-widget row AND
    // adjacent — so the adjacency change picks the same boxes as before.
    let (doc, fields) = fields_of(F1040_PDF_2025);
    let (yes, no) = topmost_yes_no_pair(&doc, &fields, 0).unwrap();
    assert_eq!(yes, "topmostSubform[0].Page1[0].c1_10[0]");
    assert_eq!(no, "topmostSubform[0].Page1[0].c1_10[1]");
}

// ── ★ 2024 full packet fills ─────────────────────────────────────────────────────────────────────

#[test]
fn ty2024_1040_fills_da_and_line7() {
    // ★ Cap-gain lands on LINE 7 (2024's single field, not 7a/7b), and the DA question = YES.
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        2024,
    )
    .unwrap()
    .unwrap();
    assert!(f.filled_7a && !f.loss && !f.active_zero);
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].f1_52[0]"
        )
        .as_deref(),
        Some("45500.50"),
        "line 7 capital gain"
    );
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_5[0]").as_deref(),
        Some("1"),
        "DA question = YES (2024 c1_5)"
    );
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_5[1]"),
        None,
        "DA No never filled"
    );
}

#[test]
fn ty2024_schedule_d_fills_lines_and_line16_f2_01() {
    let totals = ScheduleDTotals {
        st: ScheduleDPart {
            proceeds: dec!(36000.50),
            cost_basis: dec!(30500),
            gain: dec!(5500.50),
        },
        lt: ScheduleDPart {
            proceeds: dec!(60000),
            cost_basis: dec!(20000),
            gain: dec!(40000),
        },
    };
    let bytes = btctax_forms::fill_schedule_d(&totals, 2024).unwrap();
    let (doc, fields) = fields_of(&bytes);
    let v = |fqn: &str| tv(&doc, &fields, fqn);
    // Line 3 (ST total) d/h.
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
    // Line 10 (LT total) d.
    assert_eq!(
        v("topmostSubform[0].Page1[0].Table_PartII[0].Row10[0].f1_35[0]").as_deref(),
        Some("60000")
    );
    // Line 15 net LT.
    assert_eq!(
        v("topmostSubform[0].Page1[0].f1_43[0]").as_deref(),
        Some("40000")
    );
    // ★ Line 16 = 7 + 15 on the 2024-specific field f2_01 (NOT 2025's f2_1).
    assert_eq!(
        v("topmostSubform[0].Page2[0].f2_01[0]").as_deref(),
        Some("45500.50")
    );
    // QOF answered No.
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[1]").as_deref(),
        Some("2")
    );
}

#[test]
fn ty2024_schedule_se_long_chain() {
    // The 2024 unified SE is field-identical to 2025; wage base $168,600 threads onto line 9.
    let se = SeTaxResult {
        net_se: dec!(100000),
        base: dec!(92350.00),
        ss: dec!(11451.40),
        medicare: dec!(2678.15),
        addl: dec!(0.00),
        total: dec!(14129.55),
        deductible_half: dec!(7064.78),
    };
    let pdf = btctax_forms::fill_schedule_se(&se, Usd::ZERO, WAGE_BASE_2024, 2024)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    // Line 9 = wage base − line 8d (0) = 168,600.
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_18[0]").as_deref(),
        Some("168600"),
        "line 9 = 2024 wage base"
    );
    // Line 12 = ss + medicare (NOT total).
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_21[0]").as_deref(),
        Some("14129.55")
    );
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_22[0]").as_deref(),
        Some("7064.78"),
        "line 13 = deductible half"
    );
}

#[test]
fn ty2024_8283_rev_2023_digital_assets_box() {
    // ★ The 2024 8283 (Rev. 12-2023) has "k Digital assets" (Lines2i-l[0].c1_6[2] on /11) — checked
    // for BTC, same field+state as 2025.
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    let pdf = btctax_forms::fill_form_8283(&rows, 2024).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    assert_eq!(
        cb(&doc, &fields, "Form8283[0].Page1[0].Lines2i-l[0].c1_6[2]").as_deref(),
        Some("11"),
        "k Digital assets = /11"
    );
    // Row data + page-2 identity landed (using the rev2023 field names).
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line3_ColsA-C[0].Row3A[0].f1_42[0]"
        )
        .as_deref(),
        Some("1.00000000 BTC")
    );
    assert_eq!(
        tv(&doc, &fields, "Form8283[0].Page2[0].f2_19[0]").as_deref(),
        Some("Test Charity")
    );
}

#[test]
fn ty2024_8283_section_a_uses_rev2023_padded_names() {
    // Section A on Rev. 12-2023 uses the zero-padded col-A/C names (f1_05/f1_07) — a fill lands there.
    let row = Form8283Row {
        section: Some(Form8283Section::A),
        description: "0.05000000 BTC".into(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2024 - 07 - 15),
        date_contributed: date!(2024 - 02 - 10),
        cost_basis: dec!(1500),
        fmv: dec!(3000),
        claimed_deduction: Some(dec!(3000)),
        fmv_method: String::new(),
        donee: "Local Food Bank".into(),
        appraiser: String::new(),
        needs_review: false,
        details: None,
    };
    let pdf = btctax_forms::fill_form_8283(&[row], 2024).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsA-C[0].Row1A[0].f1_05[0]"
        )
        .as_deref(),
        Some("Local Food Bank")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsA-C[0].Row1A[0].f1_07[0]"
        )
        .as_deref(),
        Some("0.05000000 BTC")
    );
}

// ── ★ per-form fault injection (a swapped map must FAIL CLOSED on the 2024 forms) ────────────────

#[test]
fn fault_injected_2024_8949_column_swap_is_red() {
    // ★ cross-column: swap (d) proceeds ↔ (e) cost for every short-term row.
    let rows = mixed_rows();
    let (st, lt) = split_parts(&rows);
    let short = part_data(&st).unwrap();
    let long = part_data(&lt).unwrap();
    let mut map = Form8949Map::ty2024();
    let sp = map.parts.iter_mut().find(|p| p.term == "short").unwrap();
    for r in &mut sp.rows {
        r.swap(3, 4);
    }
    let err = fill_8949_parts(&short, &long, &map).unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

#[test]
fn fault_injected_2024_8949_row_swap_is_red() {
    // ★ same-column: swap Row1 ↔ Row2 (proceeds column) → the value lands in the wrong ROW band.
    let rows = mixed_rows();
    let (st, lt) = split_parts(&rows);
    let short = part_data(&st).unwrap();
    let long = part_data(&lt).unwrap();
    let mut map = Form8949Map::ty2024();
    let sp = map.parts.iter_mut().find(|p| p.term == "short").unwrap();
    let (r1, r2) = (sp.rows[0][3].clone(), sp.rows[1][3].clone());
    sp.rows[0][3] = r2;
    sp.rows[1][3] = r1;
    let err = fill_8949_parts(&short, &long, &map).unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

#[test]
fn fault_injected_2024_se_cross_and_same_column_are_red() {
    // cross-column (12 amount ↔ 13 mid).
    let mut m = ScheduleSeMap::ty2024();
    std::mem::swap(&mut m.line12, &mut m.line13);
    let err = fill_schedule_se_with_map(&se_100k(), Usd::ZERO, WAGE_BASE_2024, &m).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(s) if s.contains("cluster") || s.contains("column")),
        "cross-column swap: {err:?}"
    );
    // same-column (10 ↔ 11, both amount) → ordinal-y descent breaks.
    let mut m = ScheduleSeMap::ty2024();
    std::mem::swap(&mut m.line10, &mut m.line11);
    let err = fill_schedule_se_with_map(&se_100k(), Usd::ZERO, WAGE_BASE_2024, &m).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(s) if s.contains("descent")),
        "same-column swap: {err:?}"
    );
}

#[test]
fn fault_injected_2024_1040_da_swap_is_red() {
    let mut m = Form1040Map::ty2024();
    std::mem::swap(&mut m.da_yes.field, &mut m.da_no.field);
    std::mem::swap(&mut m.da_yes.on, &mut m.da_no.on);
    let err = fill_1040_with_map(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(1000),
        },
        &m,
    )
    .unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

#[test]
fn fault_injected_2024_8283_cross_column_is_red() {
    // Swap Section-B desc(a) ↔ fmv(c) for every row — a cross-column x-cluster mismatch.
    let mut m = Form8283Map::ty2024();
    for r in &mut m.section_b.rows {
        std::mem::swap(&mut r.desc, &mut r.fmv);
    }
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    let err = fill_8283_with_map(&rows, &m).unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

// ── no-unmapped + map↔PDF coverage + determinism (per 2024 form) ─────────────────────────────────

#[test]
fn ty2024_no_unmapped_filled() {
    // Every filled field on each 2024 form is one the map authorized.
    let f8949 = btctax_forms::fill_form_8949(&mixed_rows(), 2024).unwrap();
    let (doc, fields) = fields_of(&f8949);
    let allowed: std::collections::HashSet<String> = f8949_2024_field_names().into_iter().collect();
    for f in &fields {
        let filled = if f.is_button {
            checkbox_on(&doc, f.id).is_some()
        } else {
            text_value(&doc, f.id).is_some_and(|s| !s.is_empty())
        };
        if filled {
            assert!(allowed.contains(&f.fqn), "unmapped filled field: {}", f.fqn);
        }
    }
}

#[test]
fn map_2024_matches_bundled_pdf_fieldset() {
    // Every field the 2024 maps target must exist in the corresponding bundled 2024 PDF.
    let s = fieldset(F8949_PDF_2024);
    for n in f8949_2024_field_names() {
        assert!(s.contains(&n), "8949 map field absent: {n}");
    }
    let s = fieldset(SCHEDULE_D_PDF_2024);
    for n in schedule_d_2024_field_names() {
        assert!(s.contains(&n), "schedule_d map field absent: {n}");
    }
    let s = fieldset(SCHEDULE_SE_PDF_2024);
    for n in ScheduleSeMap::ty2024().field_names() {
        assert!(s.contains(n), "SE map field absent: {n}");
    }
    let s = fieldset(F8283_PDF_2024);
    for n in Form8283Map::ty2024().field_names() {
        assert!(s.contains(n), "8283 map field absent: {n}");
    }
    let m = Form1040Map::ty2024();
    let s = fieldset(F1040_PDF_2024);
    for n in [&m.line7a, &m.da_yes.field, &m.da_no.field] {
        assert!(s.contains(n), "1040 map field absent: {n}");
    }
}

#[test]
fn ty2024_forms_are_byte_deterministic() {
    let f8949 = btctax_forms::fill_form_8949(&mixed_rows(), 2024).unwrap();
    assert_eq!(
        f8949,
        btctax_forms::fill_form_8949(&mixed_rows(), 2024).unwrap()
    );
    assert_eq!(
        hex(&Sha256::digest(&f8949)),
        GOLDEN_2024_F8949,
        "8949 changed"
    );

    let sd = btctax_forms::fill_schedule_d(&totals_for(&mixed_rows()), 2024).unwrap();
    assert_eq!(
        hex(&Sha256::digest(&sd)),
        GOLDEN_2024_SCHED_D,
        "schedule_d changed"
    );

    let se = btctax_forms::fill_schedule_se(&se_100k(), Usd::ZERO, WAGE_BASE_2024, 2024)
        .unwrap()
        .unwrap();
    assert_eq!(hex(&Sha256::digest(&se)), GOLDEN_2024_SE, "SE changed");

    let f8283 = btctax_forms::fill_form_8283(
        &[b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)],
        2024,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        hex(&Sha256::digest(&f8283)),
        GOLDEN_2024_8283,
        "8283 changed"
    );

    let f1040 = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        2024,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        hex(&Sha256::digest(&f1040.pdf)),
        GOLDEN_2024_1040,
        "1040 changed"
    );
}

const GOLDEN_2024_F8949: &str = "42fca4b11085c9181e40d1bd70046285c6254658438ccc2c78b6cb5e957b8373";
const GOLDEN_2024_SCHED_D: &str =
    "3e7a0074fb159467378ff560d18d5fe883662f43ce961548be03e5a5d988d4c1";
const GOLDEN_2024_SE: &str = "d41317e29ebdf8bbf4391d0306d0d770cb7aec22d9cb7d339ca926aae1af19b1";
const GOLDEN_2024_8283: &str = "9fef09fcc42d50c6989131cf3f173720dcaf5578b7b2a71ddb7033f7f5607f0e";
const GOLDEN_2024_1040: &str = "216f946bbde8557056700af2558a400461ad90d2da47b4efa0e5ad939ede758c";

// ── helpers ──────────────────────────────────────────────────────────────────────────────────────

fn se_100k() -> SeTaxResult {
    SeTaxResult {
        net_se: dec!(100000),
        base: dec!(92350.00),
        ss: dec!(11451.40),
        medicare: dec!(2678.15),
        addl: dec!(0.00),
        total: dec!(14129.55),
        deductible_half: dec!(7064.78),
    }
}

fn full_details() -> DonationDetails {
    DonationDetails {
        donee_name: "Test Charity".into(),
        donee_address: Some("123 Main St, Anytown USA".into()),
        donee_ein: Some("12-3456789".into()),
        appraiser_name: "Test Appraiser".into(),
        appraiser_address: Some("456 Appraiser Ave".into()),
        appraiser_tin: Some("987-65-4321".into()),
        appraiser_ptin: Some("P01234567".into()),
        appraiser_qualifications: Some("Certified bitcoin appraiser".into()),
        appraisal_date: Some(date!(2024 - 06 - 01)),
        fmv_method_override: None,
    }
}

fn b_row(desc: &str, cost: Usd, fmv: Usd, carrier: bool) -> Form8283Row {
    Form8283Row {
        section: carrier.then_some(Form8283Section::B),
        description: desc.to_string(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2022 - 01 - 05),
        date_contributed: date!(2024 - 03 - 01),
        cost_basis: cost,
        fmv,
        claimed_deduction: carrier.then_some(dec!(60000)),
        fmv_method: if carrier {
            "qualified appraisal".into()
        } else {
            String::new()
        },
        donee: if carrier {
            "Test Charity".into()
        } else {
            String::new()
        },
        appraiser: if carrier {
            "Test Appraiser".into()
        } else {
            String::new()
        },
        needs_review: false,
        details: carrier.then(full_details),
    }
}

fn f8949_2024_field_names() -> Vec<String> {
    let m = Form8949Map::ty2024();
    let mut names = Vec::new();
    for p in &m.parts {
        names.push(p.box_field.clone());
        names.push(p.totals.proceeds_d.clone());
        names.push(p.totals.cost_e.clone());
        names.push(p.totals.adj_g.clone());
        names.push(p.totals.gain_h.clone());
        for r in &p.rows {
            names.extend(r.iter().cloned());
        }
    }
    names
}

fn schedule_d_2024_field_names() -> Vec<String> {
    let m = ScheduleDMap::ty2024();
    let a = |c: &AmountCols| {
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
    names.push(m.qof_yes.field.clone());
    names.push(m.qof_no.field.clone());
    names
}
