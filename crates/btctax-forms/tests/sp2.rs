//! SP2 Known-Answer Tests: Form 8283 + Schedule SE + Form 1040 cap-gains, TY2025.
//!
//! The star is again the **geometric, map-independent read-back** — now the SP2 flat oracle
//! (column-x cluster + ordinal-y descent + same-y `/Btn` pair). Every fill runs it internally and
//! FAILS CLOSED; these KATs exercise both oracle legs (a cross-column swap AND a same-column swap on
//! Schedule SE) plus the named correctness KATs from the spec.

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

/// Read a text value by fully-qualified name.
fn tv(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields.iter().find(|f| f.fqn == fqn)?;
    text_value(doc, f.id)
}
/// Read a checkbox on-state by fully-qualified name.
fn cb(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields.iter().find(|f| f.fqn == fqn)?;
    checkbox_on(doc, f.id)
}

// ── Schedule SE ──────────────────────────────────────────────────────────────────────────────────

/// The $300,000 Single golden (matches btctax-core se.rs c1_lock): ss = 21,836.40, medicare = 8,034.45,
/// addl = 693.45. Line 12 = ss + medicare = **29,870.85** (NOT total 30,564.30).
fn se_300k() -> SeTaxResult {
    SeTaxResult {
        net_se: dec!(300000),
        base: dec!(277050.00),
        ss: dec!(21836.40),
        medicare: dec!(8034.45),
        addl: dec!(693.45),
        total: dec!(30564.30),
        deductible_half: dec!(14935.42),
    }
}

const SE_L12: &str = "topmostSubform[0].Page1[0].f1_21[0]";
const SE_L13: &str = "topmostSubform[0].Page1[0].f1_22[0]";
const SE_L10: &str = "topmostSubform[0].Page1[0].f1_19[0]";
const SE_L11: &str = "topmostSubform[0].Page1[0].f1_20[0]";
const SS_WAGE_BASE_2025: Usd = dec!(176100);

#[test]
fn schedule_se_line12_equals_ss_plus_medicare() {
    let pdf = btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .expect("SE tax above the $400 floor");
    let (doc, fields) = fields_of(&pdf);
    assert_eq!(tv(&doc, &fields, SE_L12).as_deref(), Some("29870.85"));
    assert_eq!(tv(&doc, &fields, SE_L13).as_deref(), Some("14935.42"));
}

#[test]
fn schedule_se_line12_excludes_addl_medicare() {
    // ★ C1 lock: line 12 must be ss+medicare (29,870.85), NEVER the SeTaxResult.total (30,564.30),
    // which folds in the 0.9% Additional Medicare Tax — a Form 8959 item, not on Schedule SE.
    let pdf = btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    let l12 = tv(&doc, &fields, SE_L12);
    assert_eq!(l12.as_deref(), Some("29870.85"));
    assert_ne!(
        l12.as_deref(),
        Some("30564.30"),
        "line 12 must NOT include addl Medicare"
    );
}

#[test]
fn schedule_se_full_chain_is_self_consistent() {
    // The whole filled chain (Golden-1 $100k Single, no W-2): 2/3=net_se, 4a/4c/6=base, 8a/8d=0,
    // 9=176,100, 10=ss, 11=medicare, 12=ss+medicare, 13=deductible_half.
    let se = SeTaxResult {
        net_se: dec!(100000),
        base: dec!(92350.00),
        ss: dec!(11451.40),
        medicare: dec!(2678.15),
        addl: dec!(0.00),
        total: dec!(14129.55),
        deductible_half: dec!(7064.78),
    };
    let pdf = btctax_forms::fill_schedule_se(&se, Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    let g = |fqn: &str| tv(&doc, &fields, fqn);
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_5[0]").as_deref(),
        Some("100000")
    ); // line 2
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_6[0]").as_deref(),
        Some("100000")
    ); // line 3
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_7[0]").as_deref(),
        Some("92350.00")
    ); // line 4a
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_9[0]").as_deref(),
        Some("92350.00")
    ); // line 4c
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_12[0]").as_deref(),
        Some("92350.00")
    ); // line 6
       // line 8a (MID col) = 0; line 9 = 176,100.
    assert_eq!(
        g("topmostSubform[0].Page1[0].Line8a_ReadOrder[0].f1_14[0]").as_deref(),
        Some("0")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_18[0]").as_deref(),
        Some("176100")
    ); // line 9
    assert_eq!(g(SE_L10).as_deref(), Some("11451.40")); // line 10 = ss
    assert_eq!(g(SE_L11).as_deref(), Some("2678.15")); // line 11 = medicare
    assert_eq!(g(SE_L12).as_deref(), Some("14129.55")); // line 12 = ss+medicare
    assert_eq!(g(SE_L13).as_deref(), Some("7064.78")); // line 13
}

#[test]
fn schedule_se_skipped_below_400_floor() {
    // ★ I2: net SE earnings (line 4c = base) below $400 → no SE tax owed → the form is NOT written.
    let tiny = SeTaxResult {
        net_se: dec!(420),
        base: dec!(387.87), // 420 × 0.9235, below $400 → STOP
        ss: dec!(48.10),
        medicare: dec!(11.25),
        addl: dec!(0.00),
        total: dec!(59.35),
        deductible_half: dec!(29.68),
    };
    let out = btctax_forms::fill_schedule_se(&tiny, Usd::ZERO, SS_WAGE_BASE_2025, 2025).unwrap();
    assert!(
        out.is_none(),
        "Schedule SE must be skipped below the $400 floor"
    );
}

#[test]
fn schedule_se_w2_above_wage_base_skips_8b_to_10() {
    // W-2 SS wages ≥ the wage base → per the form, skip lines 8b–10 (8d/9/10 blank), ss = 0.
    let se = SeTaxResult {
        net_se: dec!(100000),
        base: dec!(92350.00),
        ss: dec!(0.00),
        medicare: dec!(2678.15),
        addl: dec!(0.00),
        total: dec!(2678.15),
        deductible_half: dec!(1339.08),
    };
    let pdf = btctax_forms::fill_schedule_se(&se, dec!(180000), SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    let (doc, fields) = fields_of(&pdf);
    // 8a filled with the W-2 wages; 8d/9/10 blank; 11/12 present.
    assert_eq!(
        tv(
            &doc,
            &fields,
            "topmostSubform[0].Page1[0].Line8a_ReadOrder[0].f1_14[0]"
        )
        .as_deref(),
        Some("180000")
    );
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_17[0]"),
        None
    ); // 8d blank
    assert_eq!(
        tv(&doc, &fields, "topmostSubform[0].Page1[0].f1_18[0]"),
        None
    ); // 9 blank
    assert_eq!(tv(&doc, &fields, SE_L10), None); // 10 blank (ss == 0)
    assert_eq!(tv(&doc, &fields, SE_L12).as_deref(), Some("2678.15")); // 12 = medicare only
}

#[test]
fn fault_injected_se_cross_column_swap_12_13_is_red() {
    // ★ [R0-M2] CROSS-column swap (line 12 amount ↔ line 13 mid) — caught by the column-x leg.
    let mut map = ScheduleSeMap::ty2025();
    std::mem::swap(&mut map.line12, &mut map.line13);
    let err =
        fill_schedule_se_with_map(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, &map).unwrap_err();
    match err {
        FormsError::Geometry(m) => assert!(
            m.contains("cluster") || m.contains("column"),
            "cross-column swap must fail via column-x, got: {m}"
        ),
        other => panic!("expected Geometry column error, got {other:?}"),
    }
}

#[test]
fn fault_injected_se_same_column_swap_10_11_is_red() {
    // ★ [R0-M2] SAME-column swap (line 10 ↔ line 11, both amount) — column-x passes, caught by the
    // ordinal-y descent leg.
    let mut map = ScheduleSeMap::ty2025();
    std::mem::swap(&mut map.line10, &mut map.line11);
    let err =
        fill_schedule_se_with_map(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, &map).unwrap_err();
    match err {
        FormsError::Geometry(m) => assert!(
            m.contains("descent"),
            "same-column swap must fail via ordinal-y descent, got: {m}"
        ),
        other => panic!("expected Geometry descent error, got {other:?}"),
    }
}

#[test]
fn schedule_se_is_byte_deterministic() {
    let a = btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    let b = btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    assert_eq!(a, b, "same (data, form) must be byte-identical");
    assert_eq!(
        hex(&Sha256::digest(&a)),
        GOLDEN_SE_SHA256,
        "SE fill changed — if intentional, update GOLDEN_SE_SHA256"
    );
}
const GOLDEN_SE_SHA256: &str = "1b50266ab4f63a682b439bce4c940b1be42fbbe6e063295118cee78d5f7dcc07";

// ── Form 1040 cap-gains ──────────────────────────────────────────────────────────────────────────

const F1040_7A: &str = "topmostSubform[0].Page1[0].f1_70[0]";
const F1040_7B: &str = "topmostSubform[0].Page1[0].f1_71[0]";
const F1040_DA_YES: &str = "topmostSubform[0].Page1[0].c1_10[0]";
const F1040_DA_NO: &str = "topmostSubform[0].Page1[0].c1_10[1]";

#[test]
fn form_1040_line7a_gain_equals_schedule_d_line16() {
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(45500.50),
        },
        2025,
    )
    .unwrap()
    .unwrap();
    assert!(f.filled_7a && !f.loss && !f.active_zero);
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(tv(&doc, &fields, F1040_7A).as_deref(), Some("45500.50"));
    assert_eq!(
        cb(&doc, &fields, F1040_DA_YES).as_deref(),
        Some("1"),
        "DA = YES"
    );
    assert_eq!(cb(&doc, &fields, F1040_DA_NO), None, "DA No never filled");
}

#[test]
fn form_1040_line7a_active_zero_is_dash_zero() {
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: Usd::ZERO,
        },
        2025,
    )
    .unwrap()
    .unwrap();
    assert!(f.filled_7a && f.active_zero);
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(tv(&doc, &fields, F1040_7A).as_deref(), Some("-0-"));
}

#[test]
fn form_1040_line7a_loss_is_blank_with_notice() {
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(-3000),
        },
        2025,
    )
    .unwrap()
    .unwrap();
    assert!(!f.filled_7a && f.loss, "net loss → 7a BLANK + loss notice");
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(tv(&doc, &fields, F1040_7A), None, "7a blank on a loss");
    assert_eq!(cb(&doc, &fields, F1040_DA_YES).as_deref(), Some("1"));
}

#[test]
fn form_1040_7a_blank_when_schedule_d_inactive() {
    // ★ I★1: income-only / donation-only year — DA = YES but Schedule D is INACTIVE → 7a BLANK
    // (never stamp "-0-" against a blank Schedule D line 16).
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: false,
            schedule_d_line16: Usd::ZERO,
        },
        2025,
    )
    .unwrap()
    .unwrap();
    assert!(!f.filled_7a && !f.active_zero && !f.loss);
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(
        tv(&doc, &fields, F1040_7A),
        None,
        "7a must be blank when Schedule D inactive"
    );
    assert_eq!(
        cb(&doc, &fields, F1040_DA_YES).as_deref(),
        Some("1"),
        "DA still YES"
    );
}

#[test]
fn form_1040_da_yes_iff_reportable_activity() {
    // No reportable activity → skip the WHOLE 1040 (never fill "No").
    let none = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: false,
            schedule_d_active: false,
            schedule_d_line16: Usd::ZERO,
        },
        2025,
    )
    .unwrap();
    assert!(none.is_none(), "no reportable activity → no 1040 at all");

    // Reportable activity → 1040 present, DA = YES.
    let some = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: false,
            schedule_d_line16: Usd::ZERO,
        },
        2025,
    )
    .unwrap();
    assert!(some.is_some());
}

#[test]
fn form_1040_7b_checkboxes_untouched() {
    let f = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(1000),
        },
        2025,
    )
    .unwrap()
    .unwrap();
    let (doc, fields) = fields_of(&f.pdf);
    assert_eq!(
        tv(&doc, &fields, F1040_7B),
        None,
        "7b stays untouched (Schedule D is attached)"
    );
}

#[test]
fn fault_injected_1040_da_yes_no_swap_is_red() {
    // Swap the map's Yes/No entries → the same-y-pair oracle catches it (Yes must be the LEFT member).
    let mut map = Form1040Map::ty2025();
    std::mem::swap(&mut map.da_yes, &mut map.da_no);
    let err = fill_1040_with_map(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(1000),
        },
        &map,
    )
    .unwrap_err();
    assert!(
        matches!(err, FormsError::Geometry(_)),
        "Yes/No swap must fail closed, got {err:?}"
    );
}

#[test]
fn form_1040_is_byte_deterministic() {
    let inp = Form1040Inputs {
        da_yes: true,
        schedule_d_active: true,
        schedule_d_line16: dec!(45500.50),
    };
    let a = btctax_forms::fill_form_1040_capgains(&inp, 2025)
        .unwrap()
        .unwrap();
    let b = btctax_forms::fill_form_1040_capgains(&inp, 2025)
        .unwrap()
        .unwrap();
    assert_eq!(a.pdf, b.pdf);
    assert_eq!(
        hex(&Sha256::digest(&a.pdf)),
        GOLDEN_1040_SHA256,
        "1040 fill changed — update golden"
    );
}
const GOLDEN_1040_SHA256: &str = "d13d087581a342aee9f785ec084e1288f002ff941c0ad86f6bdc106f05e56d1b";

// ── Form 8283 ────────────────────────────────────────────────────────────────────────────────────

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
        appraisal_date: Some(date!(2025 - 06 - 01)),
        fmv_method_override: None,
    }
}

/// Build a Section-B donation row. `carrier` marks the first-leg carrier (section/deduction/details).
fn b_row(desc: &str, cost: Usd, fmv: Usd, carrier: bool) -> Form8283Row {
    Form8283Row {
        section: carrier.then_some(Form8283Section::B),
        description: desc.to_string(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2023 - 01 - 05),
        date_contributed: date!(2025 - 03 - 01),
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

const K_DIGITAL: &str = "Form8283[0].Page1[0].Lines2i-l[0].c1_6[2]";
const L_OTHER: &str = "Form8283[0].Page1[0].Lines2i-l[0].c1_6[3]";
const F_SECURITIES: &str = "Form8283[0].Page1[0].Lines2d-h[0].c1_6[2]";

#[test]
fn form_8283_section_b_checks_digital_assets_box() {
    // ★ MUST check "k Digital assets" (Lines2i-l[0].c1_6[2] on-state /11) — not "l Other" or "f Securities".
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    let pdf = btctax_forms::fill_form_8283(&rows, 2025).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    assert_eq!(
        cb(&doc, &fields, K_DIGITAL).as_deref(),
        Some("11"),
        "k Digital assets = /11"
    );
    assert_eq!(cb(&doc, &fields, L_OTHER), None, "l Other must stay OFF");
    assert_eq!(
        cb(&doc, &fields, F_SECURITIES),
        None,
        "f Securities must stay OFF"
    );
    // Row data landed: description, appraised FMV, cost, deduction.
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
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line3_ColsA-C[0].Row3A[0].f1_44[0]"
        )
        .as_deref(),
        Some("60000")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line3_ColsD-I[0].Row3A[0].f1_56[0]"
        )
        .as_deref(),
        Some("60000"),
        "amount claimed as deduction"
    );
    // Part IV appraiser + Part V donee IDENTITY filled (page 2).
    assert_eq!(
        tv(&doc, &fields, "Form8283[0].Page2[0].f2_13[0]").as_deref(),
        Some("Test Appraiser")
    );
    assert_eq!(
        tv(&doc, &fields, "Form8283[0].Page2[0].f2_19[0]").as_deref(),
        Some("Test Charity")
    );
    assert_eq!(
        tv(&doc, &fields, "Form8283[0].Page2[0].f2_20[0]").as_deref(),
        Some("12-3456789"),
        "donee EIN"
    );
    // Date acquired uses the (mo., yr.) format.
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line3_ColsD-I[0].Row3A[0].f1_51[0]"
        )
        .as_deref(),
        Some("01/2023")
    );
}

#[test]
fn form_8283_leaves_other_party_decls_blank() {
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    let pdf = btctax_forms::fill_form_8283(&rows, 2025).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    // Part II restriction questions 5a/5b/5c.
    for q in [
        "Form8283[0].Page2[0].c2_1[0]",
        "Form8283[0].Page2[0].c2_1[1]",
        "Form8283[0].Page2[0].c2_2[0]",
        "Form8283[0].Page2[0].c2_3[0]",
    ] {
        assert_eq!(
            cb(&doc, &fields, q),
            None,
            "restriction Q {q} must be blank"
        );
    }
    // Part III taxpayer statement text.
    assert_eq!(tv(&doc, &fields, "Form8283[0].Page2[0].f2_12[0]"), None);
    // Part V donee acknowledgment: receipt date + "unrelated use?" + authorized sig title.
    assert_eq!(
        tv(&doc, &fields, "Form8283[0].Page2[0].f2_18[0]"),
        None,
        "receipt date blank"
    );
    assert_eq!(
        cb(&doc, &fields, "Form8283[0].Page2[0].c2_4[0]"),
        None,
        "unrelated-use blank"
    );
    assert_eq!(cb(&doc, &fields, "Form8283[0].Page2[0].c2_4[1]"), None);
    assert_eq!(
        tv(&doc, &fields, "Form8283[0].Page2[0].f2_23[0]"),
        None,
        "authorized title blank"
    );
    // Header block (originally-reported entity) blank.
    assert_eq!(tv(&doc, &fields, "Form8283[0].Page1[0].f1_3[0]"), None);
}

#[test]
fn form_8283_none_when_no_donations() {
    assert!(btctax_forms::fill_form_8283(&[], 2025).unwrap().is_none());
}

#[test]
fn form_8283_overflow_pages() {
    // 4 Section-B legs (one carrier + 3 more) exceed the 3-row Section B → 2 form copies (4 pages).
    let rows = vec![
        b_row("1.00000000 BTC", dec!(20000), dec!(60000), true),
        b_row("2.00000000 BTC", dec!(10000), dec!(30000), false),
        b_row("3.00000000 BTC", dec!(5000), dec!(15000), false),
        b_row("4.00000000 BTC", dec!(1000), dec!(3000), false),
    ];
    let pdf = btctax_forms::fill_form_8283(&rows, 2025).unwrap().unwrap();
    let doc = load(&pdf).unwrap();
    assert_eq!(doc.get_pages().len(), 4, "2 copies × 2 pages");
}

#[test]
fn each_8283_copy_renamed_no_shared_value() {
    let rows = vec![
        b_row("1.00000000 BTC", dec!(20000), dec!(60000), true),
        b_row("2.00000000 BTC", dec!(10000), dec!(30000), false),
        b_row("3.00000000 BTC", dec!(5000), dec!(15000), false),
        b_row("4.00000000 BTC", dec!(1000), dec!(3000), false),
    ];
    let pdf = btctax_forms::fill_form_8283(&rows, 2025).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    // Every field's fully-qualified name is unique (no duplicate FQNs across the merged copies).
    let mut fqns: Vec<&str> = fields.iter().map(|f| f.fqn.as_str()).collect();
    let total = fqns.len();
    fqns.sort_unstable();
    fqns.dedup();
    assert_eq!(
        fqns.len(),
        total,
        "all field names unique after per-copy renaming"
    );
    // The two copies each carry their own Row-3A description (BTC 1 on copy 0, BTC 4 on copy 1).
    let descs: Vec<String> = fields
        .iter()
        .filter(|f| f.fqn.ends_with("Row3A[0].f1_42[0]"))
        .filter_map(|f| text_value(&doc, f.id))
        .collect();
    assert!(descs.contains(&"1.00000000 BTC".to_string()));
    assert!(descs.contains(&"4.00000000 BTC".to_string()));
}

#[test]
fn form_8283_section_a_fills_line1_table() {
    // A Section-A (≤ $5,000) donation fills the page-1 Line 1 table (no page-2 appraiser/donee).
    let row = Form8283Row {
        section: Some(Form8283Section::A),
        description: "0.05000000 BTC".into(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2024 - 07 - 15),
        date_contributed: date!(2025 - 02 - 10),
        cost_basis: dec!(1500),
        fmv: dec!(3000),
        claimed_deduction: Some(dec!(3000)),
        fmv_method: String::new(),
        donee: "Local Food Bank".into(),
        appraiser: String::new(),
        needs_review: false,
        details: None,
    };
    let pdf = btctax_forms::fill_form_8283(&[row], 2025).unwrap().unwrap();
    let (doc, fields) = fields_of(&pdf);
    // (a) donee, (c) description, (d) date of contribution (full), (e) date acquired (mo/yr), (h) FMV.
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsA-C[0].Row1A[0].f1_5[0]"
        )
        .as_deref(),
        Some("Local Food Bank")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsA-C[0].Row1A[0].f1_7[0]"
        )
        .as_deref(),
        Some("0.05000000 BTC")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsD-I[0].Row1A[0].f1_17[0]"
        )
        .as_deref(),
        Some("02/10/2025")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsD-I[0].Row1A[0].f1_18[0]"
        )
        .as_deref(),
        Some("07/2024")
    );
    assert_eq!(
        tv(
            &doc,
            &fields,
            "Form8283[0].Page1[0].Table_Line1_ColsD-I[0].Row1A[0].f1_21[0]"
        )
        .as_deref(),
        Some("3000")
    );
    // The k-digital-assets box is a Section B control — never touched for Section A.
    assert_eq!(cb(&doc, &fields, K_DIGITAL), None);
}

#[test]
fn form_8283_is_byte_deterministic() {
    let rows = vec![b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)];
    let a = btctax_forms::fill_form_8283(&rows, 2025).unwrap().unwrap();
    let b = btctax_forms::fill_form_8283(&rows, 2025).unwrap().unwrap();
    assert_eq!(a, b);
    assert_eq!(
        hex(&Sha256::digest(&a)),
        GOLDEN_8283_SHA256,
        "8283 fill changed — update golden"
    );
}
const GOLDEN_8283_SHA256: &str = "6832c7607ff2eb233bf9c95cdf36af5338c0636f86d2c053366a44325bd76e8d";

// ── XFA drop + watermark + map↔PDF coverage (all three forms) ─────────────────────────────────────

#[test]
fn sp2_outputs_have_no_xfa() {
    let se = btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    assert!(!pdf_has_xfa(&load(&se).unwrap()).unwrap());
    let f8283 = btctax_forms::fill_form_8283(
        &[b_row("1.00000000 BTC", dec!(20000), dec!(60000), true)],
        2025,
    )
    .unwrap()
    .unwrap();
    assert!(!pdf_has_xfa(&load(&f8283).unwrap()).unwrap());
    let f1040 = btctax_forms::fill_form_1040_capgains(
        &Form1040Inputs {
            da_yes: true,
            schedule_d_active: true,
            schedule_d_line16: dec!(1000),
        },
        2025,
    )
    .unwrap()
    .unwrap();
    assert!(!pdf_has_xfa(&load(&f1040.pdf).unwrap()).unwrap());
}

#[test]
fn sp2_watermark_stamps_every_form() {
    let se = btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2025)
        .unwrap()
        .unwrap();
    let stamped = btctax_forms::stamp_draft_watermark(&se).unwrap();
    let needle = b"ESTIMATE, NOT FOR FILING";
    assert!(
        stamped.windows(needle.len()).any(|w| w == needle),
        "SE watermark text present"
    );
    assert!(!pdf_has_xfa(&load(&stamped).unwrap()).unwrap());
}

#[test]
fn map_2025_matches_bundled_pdf_fieldset() {
    let se_set = fieldset(SCHEDULE_SE_PDF_2025);
    for name in ScheduleSeMap::ty2025().field_names() {
        assert!(
            se_set.contains(name),
            "SE map field absent from PDF: {name}"
        );
    }
    let f8283_set = fieldset(F8283_PDF_2025);
    for name in Form8283Map::ty2025().field_names() {
        assert!(
            f8283_set.contains(name),
            "8283 map field absent from PDF: {name}"
        );
    }
    let m = Form1040Map::ty2025();
    let f1040_set = fieldset(F1040_PDF_2025);
    let mut f1040_names: Vec<String> = m.line7a.fields().iter().map(|s| s.to_string()).collect();
    f1040_names.push(m.da_yes.as_ref().unwrap().field.clone());
    f1040_names.push(m.da_no.as_ref().unwrap().field.clone());
    for name in &f1040_names {
        assert!(
            f1040_set.contains(name.as_str()),
            "1040 map field absent from PDF: {name}"
        );
    }
}

#[test]
fn unsupported_year_rejected_for_sp2_forms() {
    // 2023 is not bundled (this build ships 2024 + 2025).
    assert!(matches!(
        btctax_forms::fill_schedule_se(&se_300k(), Usd::ZERO, SS_WAGE_BASE_2025, 2023).unwrap_err(),
        FormsError::UnsupportedYear(2023)
    ));
    assert!(matches!(
        btctax_forms::fill_form_8283(&[b_row("1.00000000 BTC", dec!(1), dec!(2), true)], 2023)
            .unwrap_err(),
        FormsError::UnsupportedYear(2023)
    ));
    assert!(matches!(
        btctax_forms::fill_form_1040_capgains(
            &Form1040Inputs {
                da_yes: true,
                schedule_d_active: false,
                schedule_d_line16: Usd::ZERO
            },
            2023
        )
        .unwrap_err(),
        FormsError::UnsupportedYear(2023)
    ));
}
