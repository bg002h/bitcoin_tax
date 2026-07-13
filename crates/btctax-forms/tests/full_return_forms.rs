//! P6 full-return form KATs: Forms 8959, 8960, 8995 (TY2024).
//!
//! The star, as everywhere in this crate, is the **map-independent geometric read-back**: every fill
//! re-parses its own SERIALIZED bytes and verifies each cell against the blank PDF's own widget
//! rects (column-x cluster + ordinal-y descent). These KATs exercise that oracle by FAULT-INJECTING
//! a corrupted map and asserting the fill FAILS CLOSED — a mis-mapped cell must never produce a PDF.
//!
//! They also pin the values actually written, read back by fully-qualified field name, because
//! placement being right says nothing about the number being right.

use btctax_core::tax::other_taxes::{form_8959_lines, form_8960_lines};
use btctax_core::tax::qbi::form_8995_lines;
use btctax_core::tax::se::SeTaxResult;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;
use btctax_forms::testonly::*;
use btctax_forms::{Form8959Map, Form8960Map, Form8995Map, FormsError};
use rust_decimal_macros::dec;
use sha2::{Digest, Sha256};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Read a text value out of a filled PDF by fully-qualified field name.
fn tv(pdf: &[u8], fqn: &str) -> Option<String> {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let f = fields.iter().find(|f| f.fqn == fqn)?;
    text_value(&doc, f.id)
}

/// The deep/02 example-2 household: MFJ, $280,000 W-2 Medicare wages, $60,000 of mining.
fn se_mining_60k_mfj() -> SeTaxResult {
    SeTaxResult {
        net_se: dec!(60000),
        base: dec!(55410.00),
        ss: dec!(0.00),
        medicare: dec!(1606.89),
        addl: dec!(498.69),
        total: dec!(2105.58),
        deductible_half: dec!(803.44),
    }
}

// ─────────────────────────────────────── Form 8959 ────────────────────────────────────────────

#[test]
fn f8959_fills_the_printed_chain_and_reads_back() {
    let se = se_mining_60k_mfj();
    let lines = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));
    let pdf = btctax_forms::fill_form_8959(&lines, 2024)
        .unwrap()
        .expect("this household owes Additional Medicare Tax");

    let g = |fqn: &str| tv(&pdf, fqn);
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_3[0]").as_deref(),
        Some("280000")
    ); // L1
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_7[0]").as_deref(),
        Some("250000")
    ); // L5 threshold
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_9[0]").as_deref(),
        Some("270")
    ); // L7
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_10[0]").as_deref(),
        Some("55410")
    ); // L8
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_15[0]").as_deref(),
        Some("499")
    ); // L13
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_20[0]").as_deref(),
        Some("769")
    ); // L18 = 270+499
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_26[0]").as_deref(),
        Some("180")
    ); // L24

    // Part III (RRTA) is UNMODELED and must be BLANK — never a misleading 0.
    for rrta in ["f1_16[0]", "f1_17[0]", "f1_18[0]", "f1_19[0]", "f1_25[0]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{rrta}");
        assert_eq!(g(&fqn), None, "{fqn} (RRTA/unmodeled) must be blank");
    }
}

/// ★ The skip rule's non-obvious half: a filer who owes NO Additional Medicare Tax can still have had
/// some OVER-withheld (each employer withholds on its own wages over $200k, blind to a spouse or a
/// second job), and that excess is a CREDIT on 1040 line 25c. Skipping the form on line 18 alone
/// would silently forfeit it.
#[test]
fn f8959_is_produced_for_withholding_even_with_no_tax_owed() {
    // Single, $150,000 wages (under the $200,000 threshold ⇒ no tax), but $2,500 of Medicare withheld
    // against a 1.45% regular amount of $2,175 ⇒ $325 over-withheld.
    let lines = form_8959_lines(FilingStatus::Single, dec!(150000), dec!(2500), None);
    assert_eq!(
        lines.line18,
        Usd::ZERO,
        "no Additional Medicare Tax is owed"
    );
    assert_eq!(lines.line24, dec!(325), "but $325 was over-withheld");

    let pdf = btctax_forms::fill_form_8959(&lines, 2024)
        .unwrap()
        .expect("the form must still be filed to claim the 25c credit");
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_26[0]").as_deref(),
        Some("325")
    );

    // …and with neither tax nor over-withholding, there is genuinely nothing to file.
    let nothing = form_8959_lines(FilingStatus::Single, dec!(150000), dec!(2175), None);
    assert!(btctax_forms::fill_form_8959(&nothing, 2024)
        .unwrap()
        .is_none());
}

// ─────────────────────────────────────── Form 8960 ────────────────────────────────────────────

#[test]
fn f8960_fills_the_printed_chain_and_reads_back() {
    // Single: interest 5,000 + dividends 10,000 + L7 20,000 + crypto lending 2,000 = NII 37,000;
    // MAGI 300,000 ⇒ over 100,000 ⇒ line 16 = 37,000 ⇒ line 17 = 3.8% × 37,000 = 1,406.
    let lines = form_8960_lines(
        FilingStatus::Single,
        dec!(5000),
        dec!(10000),
        dec!(20000),
        dec!(2000),
        dec!(300000),
    )
    .expect("NIIT is owed");
    let pdf = btctax_forms::fill_form_8960(&lines, 2024).unwrap();

    let g = |fqn: &str| tv(&pdf, fqn);
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_3[0]").as_deref(),
        Some("5000")
    ); // L1
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_4[0]").as_deref(),
        Some("10000")
    ); // L2
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_9[0]").as_deref(),
        Some("20000")
    ); // L5a
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_14[0]").as_deref(),
        Some("2000")
    ); // L7
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_15[0]").as_deref(),
        Some("37000")
    ); // L8
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_24[0]").as_deref(),
        Some("200000")
    ); // L14 threshold
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_26[0]").as_deref(),
        Some("37000")
    ); // L16
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_27[0]").as_deref(),
        Some("1406")
    ); // L17

    // Part III's ESTATES AND TRUSTS branch must be blank on an individual return.
    for et in ["f1_28[0]", "f1_30[0]", "f1_33[0]", "f1_34[0]", "f1_35[0]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{et}");
        assert_eq!(g(&fqn), None, "{fqn} (estates/trusts) must be blank");
    }
    // …as must Schedule E (4a-4c) and the CFC/PFIC line 6 — unmodeled, never a misleading 0.
    for un in ["f1_6[0]", "f1_7[0]", "f1_8[0]", "f1_13[0]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{un}");
        assert_eq!(g(&fqn), None, "{fqn} (unmodeled) must be blank");
    }
}

// ─────────────────────────────────────── Form 8995 ────────────────────────────────────────────

#[test]
fn f8995_fills_the_printed_chain_and_reads_back() {
    // $10,000 REIT dividends, no carryforward; TI-before-QBI 100,000; net capital gain 20,000.
    let lines = form_8995_lines(dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();
    let pdf = btctax_forms::fill_form_8995(&lines, 2024).unwrap();

    let g = |fqn: &str| tv(&pdf, fqn);
    assert_eq!(
        g("topmostSubform[0].Page1[0].ReadOrderSubForm[0].f1_18[0]").as_deref(),
        Some("0")
    ); // L2 — printed at zero: the form ADDS it
    assert_eq!(
        g("topmostSubform[0].Page1[0].Line6_ReadOrder[0].f1_22[0]").as_deref(),
        Some("10000")
    ); // L6
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_25[0]").as_deref(),
        Some("2000")
    ); // L9
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_30[0]").as_deref(),
        Some("16000")
    ); // L14
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_31[0]").as_deref(),
        Some("2000")
    ); // L15 deduction

    // The trade/business table (rows 1i-1v) and line 3 must be BLANK — v1 has no business QBI.
    for t in [
        "Table[0].Ln1A_Row1[0].f1_3[0]",
        "Table[0].Ln1E_Row5[0].f1_17[0]",
    ] {
        let fqn = format!("topmostSubform[0].Page1[0].{t}");
        assert_eq!(g(&fqn), None, "{fqn} (QBI table) must be blank");
    }
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_19[0]"),
        None,
        "L3 must be blank"
    );
}

/// ★ The parenthesized-box hazard, end to end. A prior-year REIT/PTP loss carryforward LARGER than
/// this year's REIT dividends must print on lines 7 and 17 as POSITIVE MAGNITUDES — the form's own
/// `(   )` supplies the minus sign. A negative would render as `(-5,000)`: a POSITIVE number.
#[test]
fn f8995_loss_carryforward_prints_positive_magnitudes() {
    let lines = form_8995_lines(dec!(10000), dec!(15000), dec!(100000), Usd::ZERO).unwrap();
    let pdf = btctax_forms::fill_form_8995(&lines, 2024).unwrap();

    let g = |fqn: &str| tv(&pdf, fqn);
    let l7 = g("topmostSubform[0].Page1[0].f1_23[0]").unwrap();
    let l17 = g("topmostSubform[0].Page1[0].f1_33[0]").unwrap();
    assert_eq!(l7, "15000");
    assert_eq!(l17, "5000");
    assert!(
        !l7.starts_with('-'),
        "line 7 renders inside ( ) — never a minus sign"
    );
    assert!(
        !l17.starts_with('-'),
        "line 17 renders inside ( ) — never a minus sign"
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_31[0]").as_deref(),
        Some("0")
    ); // no deduction
}

/// The paren guard FAILS CLOSED if a negative ever reaches a parenthesized cell (it cannot today —
/// the core chain floors them — but the guard is the thing standing between a future refactor and a
/// silently sign-flipped return).
#[test]
fn f8995_refuses_a_negative_in_a_parenthesized_cell() {
    let mut lines = form_8995_lines(dec!(10000), dec!(15000), dec!(100000), Usd::ZERO).unwrap();
    lines.line17 = dec!(-5000); // what a naive "carryforward is a loss ⇒ negative" refactor would do
    let err = fill_form_8995_with_map(&lines, &Form8995Map::ty2024())
        .expect_err("a negative in a paren box must fail closed");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
    assert!(format!("{err}").contains("line 17"));
}

// ──────────────────────── The geometric oracle: fault injection ───────────────────────────────

/// Swap two cells ACROSS columns on Form 8959 (line 7 is AMOUNT, line 8 is MID). The oracle bands
/// each cell's x-center against its column cluster, so the corrupted map must FAIL CLOSED — no PDF.
#[test]
fn f8959_cross_column_swap_fails_closed() {
    let se = se_mining_60k_mfj();
    let lines = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));

    let mut map = Form8959Map::ty2024();
    std::mem::swap(&mut map.line7, &mut map.line8);
    let err =
        fill_form_8959_with_map(&lines, &map).expect_err("a cross-column swap must fail closed");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
}

/// Swap two cells WITHIN a column on Form 8960 (lines 13 and 15 are both MID). The column check
/// passes, so this is caught only by the ordinal-y DESCENT leg of the oracle — the second half of the
/// map-independent check, and the one a column-only oracle would miss.
#[test]
fn f8960_same_column_swap_fails_closed_on_descent() {
    let lines = form_8960_lines(
        FilingStatus::Single,
        dec!(5000),
        dec!(10000),
        dec!(20000),
        dec!(2000),
        dec!(300000),
    )
    .unwrap();

    let mut map = Form8960Map::ty2024();
    std::mem::swap(&mut map.line13, &mut map.line15); // both MID; y-order now inverted
    let err =
        fill_form_8960_with_map(&lines, &map).expect_err("a same-column swap must fail closed");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
}

// ─────────────────────────────── Determinism / golden hashes ──────────────────────────────────

/// Every fill is byte-deterministic (no timestamps, no object-id churn) — the precondition for the
/// golden-SHA regression net, and for a filer being able to diff two runs.
#[test]
fn full_return_form_fills_are_byte_deterministic() {
    let se = se_mining_60k_mfj();
    let l59 = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));
    let l60 = form_8960_lines(
        FilingStatus::Single,
        dec!(5000),
        dec!(10000),
        dec!(20000),
        dec!(2000),
        dec!(300000),
    )
    .unwrap();
    let l95 = form_8995_lines(dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();

    for _ in 0..2 {
        let a = btctax_forms::fill_form_8959(&l59, 2024).unwrap().unwrap();
        let b = btctax_forms::fill_form_8959(&l59, 2024).unwrap().unwrap();
        assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)), "8959");

        let a = btctax_forms::fill_form_8960(&l60, 2024).unwrap();
        let b = btctax_forms::fill_form_8960(&l60, 2024).unwrap();
        assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)), "8960");

        let a = btctax_forms::fill_form_8995(&l95, 2024).unwrap();
        let b = btctax_forms::fill_form_8995(&l95, 2024).unwrap();
        assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)), "8995");
    }
}

/// Full-return v1 is TY2024-only — every other year is refused, not silently filled with the wrong
/// revision's field names.
#[test]
fn full_return_forms_refuse_unsupported_years() {
    let l95 = form_8995_lines(dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();
    for year in [2017, 2023, 2025] {
        assert!(matches!(
            btctax_forms::fill_form_8995(&l95, year),
            Err(FormsError::UnsupportedYear(_))
        ));
    }
}
