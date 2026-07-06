//! SP3b Known-Answer Tests: the TY2017 full packet (the OLD pre-TCJA forms) + the engine changes the
//! 2017 revisions require — Box C/F, the dollars+cents `MoneyPair` cell, the per-year Schedule D grid
//! token / QOF-optional / DA-optional / pre-filled-exempt config, and the Rev. 12-2014 Form 8283
//! ("j Other", no digital-asset box). Every fill is read back through the SAME geometric,
//! map-independent oracle that FAILS CLOSED. The 2024 + 2025 suites (sp3.rs / kats.rs / sp2.rs) are
//! the regression guard.

mod common;
use common::*;

use btctax_core::{
    DonationDetails, Form8283HowAcquired, Form8283Row, Form8283Section, SeTaxResult, Usd,
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

const WAGE_BASE_2017: Usd = dec!(127200);

fn se_100k() -> SeTaxResult {
    // TY2017 §1401 figures for $100k net SE (base = 92,350; SS 12.4%, Medicare 2.9%).
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
        appraisal_date: Some(date!(2017 - 06 - 01)),
        fmv_method_override: None,
    }
}

fn b_row(desc: &str, cost: Usd, fmv: Usd, carrier: bool) -> Form8283Row {
    Form8283Row {
        section: carrier.then_some(Form8283Section::B),
        description: desc.to_string(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2015 - 01 - 05),
        date_contributed: date!(2017 - 03 - 01),
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

fn a_row(desc: &str, cost: Usd, fmv: Usd) -> Form8283Row {
    Form8283Row {
        section: Some(Form8283Section::A),
        description: desc.to_string(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2016 - 07 - 15),
        date_contributed: date!(2017 - 02 - 10),
        cost_basis: cost,
        fmv,
        claimed_deduction: Some(fmv),
        fmv_method: String::new(),
        donee: "Local Food Bank".into(),
        appraiser: String::new(),
        needs_review: false,
        details: None,
    }
}

// ── ★ Box C/F (NOT I/L) — the pre-1099-DA revision, shared with 2024 ─────────────────────────────

#[test]
fn ty2017_and_2024_bitcoin_use_box_c_f() {
    // ★ Both 2017 and 2024 file BTC under Box C (short-term) / Box F (long-term), on-state /3.
    for year in [2017, 2024] {
        let bytes = btctax_forms::fill_form_8949(&mixed_rows(), year).unwrap();
        let (doc, fields) = fields_of(&bytes);
        assert_eq!(
            cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[2]").as_deref(),
            Some("3"),
            "year {year}: Box C must be /3"
        );
        assert_eq!(
            cb(&doc, &fields, "topmostSubform[0].Page2[0].c2_1[2]").as_deref(),
            Some("3"),
            "year {year}: Box F must be /3"
        );
        // Box A/B (the 1099-B boxes) stay off.
        assert_eq!(
            cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[0]"),
            None
        );
        assert_eq!(
            cb(&doc, &fields, "topmostSubform[0].Page1[0].c1_1[1]"),
            None
        );
    }
}

#[test]
fn ty2017_8949_14_rows() {
    let m = Form8949Map::ty2017();
    assert_eq!(m.rows_per_page, 14);
    assert_eq!(m.table_token, "Table_Line1");
    for p in &m.parts {
        assert_eq!(p.rows.len(), 14, "part {} must enumerate 14 rows", p.term);
    }
    // 14 rows → 1 copy (2 pages); a 15th paginates.
    let rows: Vec<_> = (0..14u32)
        .map(|i| {
            row(
                btctax_core::Form8949Part::ShortTerm,
                &format!("{i}.00000000 BTC"),
                dec!(100),
                dec!(50),
                false,
            )
        })
        .collect();
    let bytes = btctax_forms::fill_form_8949(&rows, 2017).unwrap();
    assert_eq!(
        load(&bytes).unwrap().get_pages().len(),
        2,
        "14 rows = 1 copy"
    );
}

// ── ★ MoneyPair (dollars + cents) ────────────────────────────────────────────────────────────────

#[test]
fn money_pair_splits_dollars_and_cents() {
    // The REAL 2-decimal/zero-pad formatter (not raw Decimal::to_string).
    assert_eq!(fmt_money_pair(dec!(127200)), ("127200".into(), "00".into()));
    assert_eq!(
        fmt_money_pair(dec!(45500.50)),
        ("45500".into(), "50".into())
    );
    assert_eq!(fmt_money_pair(dec!(100.05)), ("100".into(), "05".into()));
    assert_eq!(fmt_money_pair(dec!(11451.4)), ("11451".into(), "40".into()));
    assert_eq!(fmt_money_pair(dec!(0)), ("0".into(), "00".into()));

    // And a real 2017 SE fill lands the split across the dollars + cents widgets.
    let pdf = btctax_forms::fill_schedule_se(&se_100k(), Usd::ZERO, WAGE_BASE_2017, 2017)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    // Line 11 (Medicare 2,678.15) → f2_37 dollars "2678", f2_38 cents "15".
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page2[0].f2_37[0]").as_deref(),
        Some("2678")
    );
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page2[0].f2_38[0]").as_deref(),
        Some("15")
    );
}

// ── ★ Schedule SE — OLD §B long form ─────────────────────────────────────────────────────────────

#[test]
fn ty2017_schedule_se_long_form_section_b() {
    let pdf = btctax_forms::fill_schedule_se(&se_100k(), Usd::ZERO, WAGE_BASE_2017, 2017)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    let dc = |d: &str, c: &str| {
        (
            tv(&doc, &fields, &format!("topmostSubform[0].Page2[0].{d}[0]")),
            tv(&doc, &fields, &format!("topmostSubform[0].Page2[0].{c}[0]")),
        )
    };
    // Line 9 = wage base 127,200 − line 8d (0).
    assert_eq!(
        dc("f2_33", "f2_34"),
        (Some("127200".into()), Some("00".into()))
    );
    // Line 12 = SS + regular Medicare = 14,129.55 (NOT total; addl excluded since 2013).
    assert_eq!(
        dc("f2_39", "f2_40"),
        (Some("14129".into()), Some("55".into()))
    );
    // Line 13 (MID column) = deductible half 7,064.78.
    assert_eq!(
        dc("f2_41", "f2_42"),
        (Some("7064".into()), Some("78".into()))
    );
    // The SHORT form (page 1) is left entirely blank.
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_5[0]"),
        None
    );
}

#[test]
fn ty2017_se_below_400_floor_skips() {
    let mut se = se_100k();
    se.base = dec!(399.99);
    assert!(
        btctax_forms::fill_schedule_se(&se, Usd::ZERO, WAGE_BASE_2017, 2017)
            .unwrap()
            .is_none()
    );
}

#[test]
fn ty2017_se_prefilled_constants_are_exempt() {
    // The blank §B form pre-prints line 7 = 127,200/00 and line 14 = 5,200/00. After filling they
    // survive AND no_unmapped_filled does not trip on them.
    let pdf = btctax_forms::fill_schedule_se(&se_100k(), Usd::ZERO, WAGE_BASE_2017, 2017)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page2[0].Line7Dollars[0]").as_deref(),
        Some("127,200")
    );
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page2[0].Line14Dollars[0]").as_deref(),
        Some("5,200")
    );
    let m = ScheduleSeMap::ty2017();
    assert_eq!(
        m.prefilled_exempt.len(),
        4,
        "line 7 + line 14 dollars/cents"
    );
}

// ── ★ Form 1040 — line 13, NO Digital-Asset question ─────────────────────────────────────────────

#[test]
fn ty2017_1040_line13_no_da_question() {
    // ★ Cap gain lands on LINE 13 (dollars f1-_51 + cents f1_52); there is NO DA question.
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true, // ignored on 2017 (no DA question) — produce gates on capital activity.
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        2017,
    )
    .unwrap()
    .unwrap();
    assert!(f.filled_7a && !f.loss && !f.active_zero);
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1-_51[0]").as_deref(),
        Some("45500"),
        "line 13 dollars"
    );
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_52[0]").as_deref(),
        Some("50"),
        "line 13 cents"
    );
    // The map carries no DA fields.
    let m = Form1040Map::ty2017();
    assert!(!m.da_present && m.da_yes.is_none() && m.da_no.is_none());
}

#[test]
fn ty2017_1040_income_only_skips() {
    // Income-only 2017 (no capital disposals) → the 1040 is SKIPPED (no line-13 value; no note fills).
    let out = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: false,
            schedule_d_line16: Usd::ZERO,
        },
        2017,
    )
    .unwrap();
    assert!(out.is_none(), "no capital activity ⇒ no 2017 1040");
}

// ── ★ Schedule D — no QOF ────────────────────────────────────────────────────────────────────────

#[test]
fn ty2017_schedule_d_has_no_qof() {
    let m = ScheduleDMap::ty2017();
    assert!(
        m.qof_yes.is_none() && m.qof_no.is_none(),
        "2017 predates QOF"
    );
    assert_eq!(m.table_token, "TablePartI");
    let bytes = btctax_forms::fill_schedule_d(&totals_for(&mixed_rows()), 2017).unwrap();
    let (doc, fields) = fields_of(&bytes);
    // Line 3 (Box C total) d/h + line 7 + line 15 + line 16 land.
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page1[0].TablePartI[0].Line3[0].f1_011[0]"
        )
        .as_deref(),
        Some("36000.50")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page1[0].TablePartII[0].Line10[0].f1_027[0]"
        )
        .as_deref(),
        Some("60000")
    );
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page2[0].f2_001[0]").as_deref(),
        Some("45500.50"),
        "line 16 = 7 + 15"
    );
}

// ── ★ Form 8283 Rev. 12-2014 — "j Other" + printed note ──────────────────────────────────────────

#[test]
fn ty2017_8283_rev_2014_uses_j_other_with_note() {
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    let pdf = btctax_forms::fill_form_8283(&rows, 2017).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    // ★ "j Other" checkbox (p1-cb4[8]) on /9 — there is NO "k Digital assets" box on this revision.
    assert_eq!(
        cb(&doc, &fields, "topmostSubform[0].Page2[0].p1-cb4[8]").as_deref(),
        Some("9"),
        "j Other = /9"
    );
    // ★ the printed note is prepended to the first Section B row's (a) description.
    let desc = tv(
        &doc,
        &fields,
        "topmostSubform[0].Page2[0].Pt1Ln5Table[0].Line5A[1].p2-t3[0]",
    )
    .unwrap();
    assert!(
        desc.contains("digital asset") && desc.contains("1.00000000 BTC"),
        "note+desc: {desc:?}"
    );
    // Money is split dollars/cents: FMV 60,000 → dollars "60000" cents "00".
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page2[0].Pt1Ln5Table[0].Line5A[1].p2-t5[0]"
        )
        .as_deref(),
        Some("60000")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page2[0].Pt1Ln5Table[0].Line5A[1].p2-t6[0]"
        )
        .as_deref(),
        Some("00")
    );
    // Donee identity landed (Part IV); appraiser NAME has no field on this revision.
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page2[0].p2-t65[0]").as_deref(),
        Some("Test Charity")
    );
    assert!(Form8283Map::ty2017().section_b.appraiser_name.is_none());
}

#[test]
fn ty2017_8283_section_a_five_rows_and_overflow() {
    // Section A holds 5 rows on the Rev. 12-2014 form; a 6th overflows onto a 2nd copy.
    let caps = Form8283Map::ty2017();
    assert_eq!(caps.section_a.rows.len(), 5);
    assert_eq!(caps.section_b.rows.len(), 4);
    let rows: Vec<_> = (0..6)
        .map(|i| a_row(&format!("0.0{i} BTC"), dec!(1000), dec!(3000)))
        .collect();
    let pdf = btctax_forms::fill_form_8283(&rows, 2017).unwrap().unwrap();
    assert_eq!(
        load(&pdf).unwrap().get_pages().len(),
        4,
        "6 rows = 2 copies (2pp each)"
    );
    // A single Section A fill lands cost/fmv dollars+cents on row A.
    let one = btctax_forms::fill_form_8283(&[a_row("0.05 BTC", dec!(1500), dec!(3000))], 2017)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&one);
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page1[0].Pt1Tabble2[0].Line1A[0].p1-t18[0]"
        )
        .as_deref(),
        Some("3000"),
        "fmv dollars"
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page1[0].Pt1Tabble2[0].Line1A[0].p1-t19[0]"
        )
        .as_deref(),
        Some("00"),
        "fmv cents"
    );
}

// ── coverage + no-unmapped ───────────────────────────────────────────────────────────────────────

#[test]
fn map_2017_matches_bundled_pdf_fieldset() {
    let s = fieldset(F8949_PDF_2017);
    for n in f8949_2017_field_names() {
        assert!(s.contains(&n), "8949 map field absent: {n}");
    }
    let s = fieldset(SCHEDULE_D_PDF_2017);
    for n in schedule_d_2017_field_names() {
        assert!(s.contains(&n), "schedule_d map field absent: {n}");
    }
    let s = fieldset(SCHEDULE_SE_PDF_2017);
    let m = ScheduleSeMap::ty2017();
    for n in m
        .field_names()
        .into_iter()
        .chain(m.prefilled_exempt.iter().map(|x| x.as_str()))
    {
        assert!(s.contains(n), "SE map field absent: {n}");
    }
    let s = fieldset(F8283_PDF_2017);
    for n in Form8283Map::ty2017().field_names() {
        assert!(s.contains(n), "8283 map field absent: {n}");
    }
    let m = Form1040Map::ty2017();
    let s = fieldset(F1040_PDF_2017);
    for n in match &m.line7a {
        MoneyCell::Single(f) => vec![f.clone()],
        MoneyCell::Pair(p) => vec![p.dollars_field.clone(), p.cents_field.clone()],
    } {
        assert!(s.contains(&n), "1040 map field absent: {n}");
    }
}

#[test]
fn ty2017_no_unmapped_filled_all_forms() {
    // Every filled field on each 2017 form is one the map authorized (SE minus the pre-filled set).
    let check = |pdf: &[u8], allowed: std::collections::HashSet<String>| {
        let (doc, fields) = fields_of(pdf);
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
    };
    check(
        &btctax_forms::fill_form_8949(&mixed_rows(), 2017).unwrap(),
        f8949_2017_field_names().into_iter().collect(),
    );
    check(
        &btctax_forms::fill_schedule_d(&totals_for(&mixed_rows()), 2017).unwrap(),
        schedule_d_2017_field_names().into_iter().collect(),
    );
    let se = ScheduleSeMap::ty2017();
    let se_allowed: std::collections::HashSet<String> = se
        .field_names()
        .into_iter()
        .map(String::from)
        .chain(se.prefilled_exempt.iter().cloned())
        .collect();
    check(
        &btctax_forms::fill_schedule_se(&se_100k(), Usd::ZERO, WAGE_BASE_2017, 2017)
            .unwrap()
            .unwrap(),
        se_allowed,
    );
    check(
        &btctax_forms::fill_form_8283(
            &[b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)],
            2017,
        )
        .unwrap()
        .unwrap(),
        Form8283Map::ty2017()
            .field_names()
            .into_iter()
            .map(String::from)
            .collect(),
    );
    let f1040 = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        2017,
    )
    .unwrap()
    .unwrap();
    check(
        &f1040.pdf,
        [
            "topmostSubform[0].Page1[0].f1-_51[0]".to_string(),
            "topmostSubform[0].Page1[0].f1_52[0]".to_string(),
        ]
        .into_iter()
        .collect(),
    );
}

// ── ★ per-form geometric fault injection (a swapped 2017 map must FAIL CLOSED) ────────────────────

/// Swap the dollars ↔ cents members of a MoneyPair cell (a no-op on a single-field cell).
fn swap_pair(cell: &mut MoneyCell) {
    if let MoneyCell::Pair(p) = cell {
        std::mem::swap(&mut p.dollars_field, &mut p.cents_field);
    }
}

#[test]
fn fault_injected_2017_8949_column_swap_is_red() {
    let rows = mixed_rows();
    let (st, lt) = split_parts(&rows);
    let short = part_data(&st).unwrap();
    let long = part_data(&lt).unwrap();
    let mut map = Form8949Map::ty2017();
    let sp = map.parts.iter_mut().find(|p| p.term == "short").unwrap();
    for r in &mut sp.rows {
        r.swap(3, 4); // (d) proceeds ↔ (e) cost
    }
    let err = fill_8949_parts(&short, &long, &map).unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

#[test]
fn fault_injected_2017_schedule_d_column_swap_is_red() {
    let mut m = ScheduleDMap::ty2017();
    std::mem::swap(&mut m.line3.proceeds_d, &mut m.line3.gain_h); // (d) ↔ (h)
    let err = fill_schedule_d_totals(&totals_for(&mixed_rows()), &m).unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

#[test]
fn fault_injected_2017_se_moneypair_swap_is_red() {
    // ★ MoneyPair PAIR swap: line 11's dollars ↔ cents. The whole-dollars value lands in the narrow
    // cents widget → its center-x leaves the dollars column cluster → FAILS CLOSED.
    let mut m = ScheduleSeMap::ty2017();
    swap_pair(&mut m.line11);
    let err = fill_schedule_se_with_map(&se_100k(), Usd::ZERO, WAGE_BASE_2017, &m).unwrap_err();
    assert!(
        matches!(err, FormsError::Geometry(_)),
        "moneypair swap: {err:?}"
    );
}

#[test]
fn fault_injected_2017_se_cross_and_same_column_are_red() {
    // cross-column (12 amount ↔ 13 mid).
    let mut m = ScheduleSeMap::ty2017();
    std::mem::swap(&mut m.line12, &mut m.line13);
    let err = fill_schedule_se_with_map(&se_100k(), Usd::ZERO, WAGE_BASE_2017, &m).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(s) if s.contains("cluster") || s.contains("column")),
        "cross-column: {err:?}"
    );
    // same-column (10 ↔ 11) → ordinal-y descent breaks.
    let mut m = ScheduleSeMap::ty2017();
    std::mem::swap(&mut m.line10, &mut m.line11);
    let err = fill_schedule_se_with_map(&se_100k(), Usd::ZERO, WAGE_BASE_2017, &m).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(s) if s.contains("descent")),
        "same-column: {err:?}"
    );
}

#[test]
fn fault_injected_2017_1040_moneypair_swap_is_red() {
    // ★ line 13 dollars ↔ cents swap fails closed.
    let mut m = Form1040Map::ty2017();
    swap_pair(&mut m.line7a);
    let err = fill_1040_with_map(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        &m,
    )
    .unwrap_err();
    assert!(matches!(err, FormsError::Geometry(_)), "got {err:?}");
}

#[test]
fn fault_injected_2017_8283_moneypair_and_cross_column_are_red() {
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    // ★ MoneyPair PAIR swap: Section B row-A FMV dollars ↔ cents.
    let mut m = Form8283Map::ty2017();
    swap_pair(&mut m.section_b.rows[0].fmv);
    let err = fill_8283_with_map(&rows, &m).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(_)),
        "moneypair swap: {err:?}"
    );
    // cross-column: Section B FMV(c) ↔ cost(f) for every row.
    let mut m = Form8283Map::ty2017();
    for r in &mut m.section_b.rows {
        std::mem::swap(&mut r.fmv, &mut r.cost);
    }
    let err = fill_8283_with_map(&rows, &m).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(_)),
        "cross-column: {err:?}"
    );
}

// ── determinism (golden sha per (form, 2017)) ────────────────────────────────────────────────────

#[test]
fn ty2017_forms_are_byte_deterministic() {
    let f8949 = btctax_forms::fill_form_8949(&mixed_rows(), 2017).unwrap();
    assert_eq!(
        f8949,
        btctax_forms::fill_form_8949(&mixed_rows(), 2017).unwrap()
    );
    assert_eq!(
        hex(&Sha256::digest(&f8949)),
        GOLDEN_2017_F8949,
        "8949 changed"
    );

    let sd = btctax_forms::fill_schedule_d(&totals_for(&mixed_rows()), 2017).unwrap();
    assert_eq!(
        hex(&Sha256::digest(&sd)),
        GOLDEN_2017_SCHED_D,
        "schedule_d changed"
    );

    let se = btctax_forms::fill_schedule_se(&se_100k(), Usd::ZERO, WAGE_BASE_2017, 2017)
        .unwrap()
        .unwrap();
    assert_eq!(hex(&Sha256::digest(&se)), GOLDEN_2017_SE, "SE changed");

    let f8283 = btctax_forms::fill_form_8283(
        &[b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)],
        2017,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        hex(&Sha256::digest(&f8283)),
        GOLDEN_2017_8283,
        "8283 changed"
    );

    let f1040 = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        2017,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        hex(&Sha256::digest(&f1040.pdf)),
        GOLDEN_2017_1040,
        "1040 changed"
    );
}

const GOLDEN_2017_F8949: &str = "00e9d511e6a8225e5710884feb6c531570b424858a1de2c7891d02e532860f03";
const GOLDEN_2017_SCHED_D: &str =
    "ed25f3866fc34f4d5eaa593b78cb45f446454324f68176c1443832653341637b";
const GOLDEN_2017_SE: &str = "9203f018e8f351d28629b16f8406e0da88f0f6e40aa453915d72761511068a88";
const GOLDEN_2017_8283: &str = "274a4984683283ae4fbfb568b26c05c4caf9c1fe54912136729374122f0e6055";
const GOLDEN_2017_1040: &str = "56db10fa8e8dbe5cef1b4ba2a7de35088a74cd1e7c0fb0884ccb259cd1a2120a";

// ── helpers for coverage ─────────────────────────────────────────────────────────────────────────

fn f8949_2017_field_names() -> Vec<String> {
    let m = Form8949Map::ty2017();
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

fn schedule_d_2017_field_names() -> Vec<String> {
    let m = ScheduleDMap::ty2017();
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
    names
}
