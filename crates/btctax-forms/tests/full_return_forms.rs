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
use btctax_core::tax::printed::{
    Form1040Lines, Schedule1Lines, Schedule2Lines, Schedule3Lines, ScheduleALines, ScheduleBLines,
    ScheduleBRow, ScheduleCLines, ScheduleDLines, ScheduleDRouting,
};
use btctax_core::tax::qbi::form_8995_lines;
use btctax_core::tax::se::SeTaxResult;
use btctax_core::tax::testonly::kitchen_sink_header;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;
use btctax_forms::testonly::*;
use btctax_forms::{
    Form8959Map, Form8960Map, Form8995Map, FormsError, Schedule3Map, ScheduleAMap, ScheduleCMap,
};
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

/// Is a checkbox ON in a filled PDF, by fully-qualified field name?
fn box_on(pdf: &[u8], fqn: &str) -> bool {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    fields
        .iter()
        .find(|f| f.fqn == fqn)
        .and_then(|f| checkbox_on(&doc, f.id))
        .is_some()
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
    let pdf = btctax_forms::fill_form_8959(&lines, &kitchen_sink_header(), 2024)
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

    let pdf = btctax_forms::fill_form_8959(&lines, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("the form must still be filed to claim the 25c credit");
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_26[0]").as_deref(),
        Some("325")
    );

    // …and with neither tax nor over-withholding, there is genuinely nothing to file.
    let nothing = form_8959_lines(FilingStatus::Single, dec!(150000), dec!(2175), None);
    assert!(
        btctax_forms::fill_form_8959(&nothing, &kitchen_sink_header(), 2024)
            .unwrap()
            .is_none()
    );
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
    let pdf = btctax_forms::fill_form_8960(&lines, &kitchen_sink_header(), 2024).unwrap();

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
    let lines =
        form_8995_lines("", Usd::ZERO, dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();
    let pdf = btctax_forms::fill_form_8995(&lines, &kitchen_sink_header(), 2024).unwrap();

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
    let lines =
        form_8995_lines("", Usd::ZERO, dec!(10000), dec!(15000), dec!(100000), Usd::ZERO).unwrap();
    let pdf = btctax_forms::fill_form_8995(&lines, &kitchen_sink_header(), 2024).unwrap();

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
    let mut lines =
        form_8995_lines("", Usd::ZERO, dec!(10000), dec!(15000), dec!(100000), Usd::ZERO).unwrap();
    lines.line17 = dec!(-5000); // what a naive "carryforward is a loss ⇒ negative" refactor would do
    let err = fill_form_8995_with_map(&lines, &kitchen_sink_header(), &Form8995Map::ty2024())
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
    let err = fill_form_8959_with_map(&lines, &kitchen_sink_header(), &map)
        .expect_err("a cross-column swap must fail closed");
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
    let err = fill_form_8960_with_map(&lines, &kitchen_sink_header(), &map)
        .expect_err("a same-column swap must fail closed");
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
    let l95 =
        form_8995_lines("", Usd::ZERO, dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();

    for _ in 0..2 {
        let a = btctax_forms::fill_form_8959(&l59, &kitchen_sink_header(), 2024)
            .unwrap()
            .unwrap();
        let b = btctax_forms::fill_form_8959(&l59, &kitchen_sink_header(), 2024)
            .unwrap()
            .unwrap();
        assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)), "8959");

        let a = btctax_forms::fill_form_8960(&l60, &kitchen_sink_header(), 2024).unwrap();
        let b = btctax_forms::fill_form_8960(&l60, &kitchen_sink_header(), 2024).unwrap();
        assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)), "8960");

        let a = btctax_forms::fill_form_8995(&l95, &kitchen_sink_header(), 2024).unwrap();
        let b = btctax_forms::fill_form_8995(&l95, &kitchen_sink_header(), 2024).unwrap();
        assert_eq!(hex(&Sha256::digest(&a)), hex(&Sha256::digest(&b)), "8995");
    }
}

/// Full-return v1 is TY2024-only — every other year is refused, not silently filled with the wrong
/// revision's field names.
#[test]
fn full_return_forms_refuse_unsupported_years() {
    let l95 =
        form_8995_lines("", Usd::ZERO, dec!(10000), Usd::ZERO, dec!(100000), dec!(20000)).unwrap();
    for year in [2017, 2023, 2025] {
        assert!(matches!(
            btctax_forms::fill_form_8995(&l95, &kitchen_sink_header(), year),
            Err(FormsError::UnsupportedYear(_))
        ));
    }
}

// ────────────────────────────── Schedule 2 / Schedule 3 ───────────────────────────────────────

/// Schedule 2 carries the three taxes v1 computes, and **Part I stays blank** — line 1a (excess
/// APTC) has no input and would refuse if it did, and line 2 (AMT) is $0 by construction because the
/// return is refused outright if the Form 6251 screen trips. A 0 printed there would be a lie.
///
/// Line 21 is on **page 2**, so this also exercises the per-page descent grouping.
#[test]
fn schedule_2_fills_part_ii_and_leaves_part_i_blank() {
    let lines = Schedule2Lines {
        line4: dec!(29871),
        line11: dec!(693),
        line12: dec!(1406),
        line21: dec!(31970), // 29,871 + 693 + 1,406 — sums the PRINTED lines
    };
    let pdf = btctax_forms::fill_schedule_2(&lines, &kitchen_sink_header(), 2024).unwrap();

    let g = |fqn: &str| tv(&pdf, fqn);
    assert_eq!(g("form1[0].Page1[0].f1_14[0]").as_deref(), Some("29871")); // L4
    assert_eq!(g("form1[0].Page1[0].f1_21[0]").as_deref(), Some("693")); // L11
    assert_eq!(g("form1[0].Page1[0].f1_22[0]").as_deref(), Some("1406")); // L12
    assert_eq!(g("form1[0].Page2[0].f2_25[0]").as_deref(), Some("31970")); // L21 — PAGE 2

    // Part I must be BLANK — not zero.
    for p1 in ["f1_03[0]", "f1_11[0]", "f1_12[0]", "f1_13[0]"] {
        let fqn = format!("form1[0].Page1[0].{p1}");
        assert_eq!(g(&fqn), None, "{fqn} (Schedule 2 Part I) must be blank");
    }
}

/// Schedule 3 carries the FTC and the excess-SS credit. Every other Part I credit is a §3.4
/// conservative omission and must be BLANK — a 0 would tell the filer we considered and rejected it.
#[test]
fn schedule_3_fills_ftc_and_excess_ss_and_leaves_omitted_credits_blank() {
    let lines = Schedule3Lines {
        line1: dec!(287),
        line8: dec!(287),
        line10: dec!(4000), // the extension payment — the line whose absence made the filer pay twice
        line11: dec!(1235),
        line15: dec!(5235), // "Add lines 9 through 12 and 14"
    };
    let pdf = btctax_forms::fill_schedule_3(&lines, &kitchen_sink_header(), 2024).unwrap();

    let g = |fqn: &str| tv(&pdf, fqn);
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_03[0]").as_deref(),
        Some("287")
    ); // L1 FTC
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_26[0]").as_deref(),
        Some("287")
    ); // L8 → 1040 L20
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_28[0]").as_deref(),
        Some("4000")
    ); // L10 extension payment
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_29[0]").as_deref(),
        Some("1235")
    ); // L11 excess SS
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_39[0]").as_deref(),
        Some("5235")
    ); // L15 = "Add lines 9 through 12 and 14" ⇒ 4000 + 1235 → 1040 L31

    // The conservatively-omitted credits: education (L3), dependent-care (L2), saver's (L4),
    // residential-energy (L5a/5b), adoption (L6c). All BLANK.
    for omitted in ["f1_04[0]", "f1_05[0]", "f1_06[0]", "f1_07[0]", "f1_08[0]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{omitted}");
        assert_eq!(
            g(&fqn),
            None,
            "{fqn} (conservatively omitted credit) must be blank"
        );
    }
    // …and line 6e is the ReadOnly "Reserved for future use" widget — never written.
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_13[0]"),
        None,
        "L6e is reserved/ReadOnly"
    );
}

/// ★ The extension payment reaches the FILED PAGE (Fable ARCH-P6.3a D1). Schedule 3 line 10 is "Amount
/// paid with request for extension to file", and it flows to 1040 L31 via line 15. Its absence from the
/// printed chain meant a filer who had ALREADY paid $4,000 with their extension would be told on the
/// filed return to pay it a second time.
#[test]
fn schedule_3_prints_the_extension_payment_on_line_10() {
    let lines = Schedule3Lines {
        line1: Usd::ZERO,
        line8: Usd::ZERO,
        line10: dec!(4000),
        line11: Usd::ZERO,
        line15: dec!(4000),
    };
    let pdf = btctax_forms::fill_schedule_3(&lines, &kitchen_sink_header(), 2024).unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_28[0]").as_deref(),
        Some("4000"),
        "L10 — the payment the filer already made"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_39[0]").as_deref(),
        Some("4000"),
        "L15 carries it to 1040 L31"
    );
}

/// Same-column swap on Schedule 3 (L1 and L15 are both AMOUNT, far apart in y) → the descent leg of
/// the oracle catches it and the fill FAILS CLOSED.
#[test]
fn schedule_3_same_column_swap_fails_closed() {
    let lines = Schedule3Lines {
        line1: dec!(287),
        line8: dec!(287),
        line10: dec!(4000), // the extension payment — the line whose absence made the filer pay twice
        line11: dec!(1235),
        line15: dec!(5235), // "Add lines 9 through 12 and 14"
    };
    let mut map = Schedule3Map::ty2024();
    std::mem::swap(&mut map.line1, &mut map.line15);
    let err = fill_schedule_3_with_map(&lines, &kitchen_sink_header(), &map)
        .expect_err("a same-column swap must fail closed");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
}

// ───────────────────────────────────── Schedule A ─────────────────────────────────────────────

fn sch_a_lines() -> ScheduleALines {
    // AGI 100,000 ⇒ 7.5% floor 7,500; medical 10,000 ⇒ 2,500 allowed.
    // SALT 8,000 + 4,000 + 500 = 12,500 ⇒ capped at 10,000. Mortgage 12,000.
    // Charitable 1,000 cash + 2,000 noncash + 500 carryover = 3,500. Total 28,000.
    ScheduleALines {
        line5a_is_sales_tax: false,
        line18_elects_smaller: false,
        line1: dec!(10000),
        line2: dec!(100000),
        line3: dec!(7500),
        line4: dec!(2500),
        line5a: dec!(8000),
        line5b: dec!(4000),
        line5c: dec!(500),
        line5d: dec!(12500),
        line5e: dec!(10000),
        line7: dec!(10000),
        line8a: dec!(12000),
        line8e: dec!(12000),
        line10: dec!(12000),
        line11: dec!(1000),
        line12: dec!(2000),
        line13: dec!(500),
        line14: dec!(3500),
        line17: dec!(28000),
    }
}

#[test]
fn schedule_a_fills_the_printed_chain_and_reads_back() {
    let pdf = btctax_forms::fill_schedule_a(&sch_a_lines(), &kitchen_sink_header(), 2024).unwrap();
    let g = |fqn: &str| tv(&pdf, fqn);

    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_3[0]").as_deref(),
        Some("10000")
    ); // L1
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_4[0]").as_deref(),
        Some("100000")
    ); // L2 ★ AGI-inline
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_5[0]").as_deref(),
        Some("7500")
    ); // L3 floor
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_6[0]").as_deref(),
        Some("2500")
    ); // L4
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_10[0]").as_deref(),
        Some("12500")
    ); // L5d
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_11[0]").as_deref(),
        Some("10000")
    ); // L5e — capped
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_22[0]").as_deref(),
        Some("12000")
    ); // L8e
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_28[0]").as_deref(),
        Some("3500")
    ); // L14
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_34[0]").as_deref(),
        Some("28000")
    ); // L17 → 1040 L12

    // ★ Line 8d (f1_21) is the IRS's own ReadOnly "Reserved for future use" widget — never written.
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_21[0]"),
        None,
        "L8d is reserved/ReadOnly"
    );
    // Unmodeled lines stay BLANK: 6 (other taxes), 8b/8c, 9 (investment interest), 15, 16.
    for blank in [
        "f1_14[0]", "f1_19[0]", "f1_20[0]", "f1_23[0]", "f1_29[0]", "f1_33[0]",
    ] {
        let fqn = format!("topmostSubform[0].Page1[0].{blank}");
        assert_eq!(g(&fqn), None, "{fqn} (unmodeled) must be blank");
    }
}

/// ★ The AGI-inline column. Line 2 (`f1_4`, x ≈ [331,403]) is in NEITHER the MID nor the AMOUNT
/// cluster — its box sits inline with the printed sentence, 86pt left of MID, and it is the *same
/// width* as a MID box, so neither a MID column check nor a width heuristic would catch a swap. Only
/// its own tight cluster does. Swapping it with line 1 (a MID cell) must FAIL CLOSED — otherwise the
/// AGI would print into the medical-expenses box and the 7.5% floor would be taken on the wrong
/// number.
#[test]
fn schedule_a_agi_inline_column_swap_fails_closed() {
    let mut map = ScheduleAMap::ty2024();
    std::mem::swap(&mut map.line1, &mut map.line2);
    let err = fill_schedule_a_with_map(&sch_a_lines(), &kitchen_sink_header(), &map)
        .expect_err("swapping the AGI-inline cell with a MID cell must fail closed");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
}

// ───────────────────────────────────── Schedule 1 ─────────────────────────────────────────────

/// Schedule 1 carries the additional income (Part I, page 1) and the adjustments (Part II, page 2).
/// This also exercises the per-page descent grouping across a real two-page form.
#[test]
fn schedule_1_fills_both_parts_across_two_pages() {
    let lines = Schedule1Lines {
        line1: dec!(1200),   // taxable state refund
        line3: dec!(40000),  // crypto Schedule C net
        line7: dec!(3000),   // unemployment
        line8v: dec!(5000),  // non-business crypto ordinary income
        line9: dec!(5000),   // total other income (8a-8z) = 8v
        line10: dec!(49200), // 1,200 + 40,000 + 3,000 + 5,000 → 1040 L8
        line15: dec!(2825),  // half of SE tax
        line18: dec!(150),   // early-withdrawal penalty
        line21: dec!(2500),  // student-loan interest
        line26: dec!(5475),  // 2,825 + 150 + 2,500 → 1040 L10
    };
    let pdf = btctax_forms::fill_schedule_1(&lines, &kitchen_sink_header(), 2024).unwrap();
    let g = |fqn: &str| tv(&pdf, fqn);

    // Part I — page 1.
    assert_eq!(g("form1[0].Page1[0].f1_04[0]").as_deref(), Some("1200")); // L1
    assert_eq!(g("form1[0].Page1[0].f1_07[0]").as_deref(), Some("40000")); // L3
    assert_eq!(g("form1[0].Page1[0].f1_11[0]").as_deref(), Some("3000")); // L7
    assert_eq!(g("form1[0].Page1[0].f1_33[0]").as_deref(), Some("5000")); // L8v ★ digital assets
    assert_eq!(g("form1[0].Page1[0].f1_38[0]").as_deref(), Some("49200")); // L10 → 1040 L8

    // Part II — page 2.
    assert_eq!(g("form1[0].Page2[0].f2_05[0]").as_deref(), Some("2825")); // L15
    assert_eq!(g("form1[0].Page2[0].f2_08[0]").as_deref(), Some("150")); // L18
    assert_eq!(g("form1[0].Page2[0].f2_13[0]").as_deref(), Some("2500")); // L21
    assert_eq!(g("form1[0].Page2[0].f2_31[0]").as_deref(), Some("5475")); // L26 → 1040 L10

    // ★ Line 22 (f2_14) is the IRS's ReadOnly "Reserved for future use" widget — never written.
    // It sits BETWEEN line 21 and line 23, so a suffix-walker that skipped it would misalign
    // everything below.
    assert_eq!(
        g("form1[0].Page2[0].f2_14[0]"),
        None,
        "L22 is reserved/ReadOnly"
    );

    // Unrepresentable income stays BLANK: line 5 is Schedule E, line 6 is Schedule F.
    assert_eq!(
        g("form1[0].Page1[0].f1_09[0]"),
        None,
        "L5 (Schedule E) must be blank"
    );
    assert_eq!(
        g("form1[0].Page1[0].f1_10[0]"),
        None,
        "L6 (Schedule F) must be blank"
    );
    // …and the non-money fields in the money band are never touched (a date on 2b).
    assert_eq!(
        g("form1[0].Page1[0].f1_06[0]"),
        None,
        "L2b is a DATE field, not money"
    );
}

// ───────────────────────────────────── Schedule C ─────────────────────────────────────────────

#[test]
fn schedule_c_fills_the_printed_chain_and_reads_back() {
    // $60,000 of crypto mining gross, $8,000 of expenses ⇒ $52,000 net profit.
    let lines = ScheduleCLines {
        line_a_business: "Bitcoin mining".into(),
        line_b_naics: "518210".into(),
        line_f_accrual: false,
        line1: dec!(60000),
        line3: dec!(60000),
        line5: dec!(60000),
        line7: dec!(60000),
        line28: dec!(8000),
        line29: dec!(52000),
        line31: dec!(52000),
    };
    let pdf = btctax_forms::fill_schedule_c(&lines, &kitchen_sink_header(), 2024).unwrap();
    let g = |fqn: &str| tv(&pdf, fqn);

    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_10[0]").as_deref(),
        Some("60000")
    ); // L1
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_12[0]").as_deref(),
        Some("60000")
    ); // L3
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_14[0]").as_deref(),
        Some("60000")
    ); // L5
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_16[0]").as_deref(),
        Some("60000")
    ); // L7
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_41[0]").as_deref(),
        Some("8000")
    ); // L28
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_42[0]").as_deref(),
        Some("52000")
    ); // L29

    // ★ THE LINE-31 TRAP. Line 31's GUTTER label is at y≈144.5, but its AMOUNT BOX is at y≈120.5 —
    // two printed rows lower, because the line carries two bullet rows of instructions. Correlating
    // on the gutter label would map line 31 to the wrong widget, and line 31 is the figure that feeds
    // BOTH Schedule 1 line 3 AND Schedule SE line 2: a mis-map there is wrong income and wrong SE tax.
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_46[0]").as_deref(),
        Some("52000"),
        "line 31 must land in the box at y=120 (f1_46), NOT the one near its gutter label"
    );
    // f1_45 is line 30 (business use of home) — out of scope, and it must stay BLANK. If line 31 had
    // been mapped by its gutter label it would very plausibly have landed here instead.
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_45[0]"),
        None,
        "L30 (home office) is out of scope and must be blank"
    );

    // Unmodeled: returns (L2), cost of goods sold (L4), other income (L6) — BLANK, never 0.
    for blank in ["f1_11[0]", "f1_13[0]", "f1_15[0]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{blank}");
        assert_eq!(g(&fqn), None, "{fqn} (unmodeled) must be blank");
    }
    // Part II's individual expense lines stay BLANK — v1 has one flat total, and writing 0 into each
    // of the twenty lines would assert we found no advertising, no insurance, no legal fees.
    for expense in ["Lines18-27[0].f1_28[0]", "Lines18-27[0].f1_33[0]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{expense}");
        assert_eq!(g(&fqn), None, "{fqn} (itemized expense line) must be blank");
    }
}

/// Schedule C's money column is x ≈ [475, 576] — its own, shared with no other form. Its cells sit
/// OUTSIDE the [504, 576] band every other schedule uses, so a filler that reused the common cluster
/// constant would reject every Schedule C cell. This pins that the right cluster is in force.
#[test]
fn schedule_c_same_column_swap_fails_closed() {
    let lines = ScheduleCLines {
        line_a_business: "Bitcoin mining".into(),
        line_b_naics: "518210".into(),
        line_f_accrual: false,
        line1: dec!(60000),
        line3: dec!(60000),
        line5: dec!(60000),
        line7: dec!(60000),
        line28: dec!(8000),
        line29: dec!(52000),
        line31: dec!(52000),
    };
    let mut map = ScheduleCMap::ty2024();
    std::mem::swap(&mut map.line1, &mut map.line31); // same column, y-order inverted
    let err = fill_schedule_c_with_map(&lines, &kitchen_sink_header(), &map)
        .expect_err("a same-column swap must fail closed on the descent leg");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
}

// ───────────────────────────────────── Schedule B ─────────────────────────────────────────────

fn row(payer: &str, amount: Usd) -> ScheduleBRow {
    ScheduleBRow {
        payer: payer.to_string(),
        amount,
    }
}

fn sch_b(part1: Vec<ScheduleBRow>, part2: Vec<ScheduleBRow>, fa: bool, ft: bool) -> ScheduleBLines {
    let line2: Usd = part1.iter().map(|r| r.amount).sum();
    let line6: Usd = part2.iter().map(|r| r.amount).sum();
    ScheduleBLines {
        line7b_countries: String::new(),
        part1_rows: part1,
        line2,
        line4: line2,
        part2_rows: part2,
        line6,
        foreign_accounts_7a: fa,
        foreign_trust_8: ft,
    }
}

/// Schedule B lists its payers by name and totals the PRINTED rows, so the form adds up against its
/// own list. Row 1 of BOTH tables has a different parent subform than every other row, so this also
/// pins that those two FQNs resolve.
#[test]
fn schedule_b_lists_payers_and_totals_the_printed_rows() {
    let lines = sch_b(
        vec![row("Ally Bank", dec!(1200)), row("US Treasury", dec!(800))],
        vec![
            row("Vanguard VTSAX", dec!(3400)),
            row("Fidelity FXAIX", dec!(600)),
        ],
        false,
        false,
    );
    let pdf = btctax_forms::fill_schedule_b(&lines, &kitchen_sink_header(), 2024).unwrap();
    let g = |fqn: &str| tv(&pdf, fqn);

    // ★ Part I row 1's payer lives under Line1_ReadOrder — a parent no other row has.
    assert_eq!(
        g("topmostSubform[0].Page1[0].Line1_ReadOrder[0].f1_03[0]").as_deref(),
        Some("Ally Bank")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_04[0]").as_deref(),
        Some("1200")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_05[0]").as_deref(),
        Some("US Treasury")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_06[0]").as_deref(),
        Some("800")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_31[0]").as_deref(),
        Some("2000")
    ); // L2
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_33[0]").as_deref(),
        Some("2000")
    ); // L4 → 1040 2b

    // ★ Part II row 1's payer lives under ReadOrderControl — a DIFFERENT wrapper again.
    assert_eq!(
        g("topmostSubform[0].Page1[0].ReadOrderControl[0].f1_34[0]").as_deref(),
        Some("Vanguard VTSAX")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_35[0]").as_deref(),
        Some("3400")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_36[0]").as_deref(),
        Some("Fidelity FXAIX")
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_64[0]").as_deref(),
        Some("4000")
    ); // L6 → 1040 3b

    // Unused rows stay blank; line 3 (Form 8815) is unmodeled and stays blank.
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_07[0]"),
        None,
        "unused row stays blank"
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_32[0]"),
        None,
        "L3 (Form 8815) is unmodeled"
    );
}

/// ★ Part III is TRANSCRIBED, never decided. Lines 7a and 8 carry the filer's OWN answers (the return
/// is refused upstream if they were left unanswered). The unnumbered FBAR sub-question under 7a
/// (`c1_2`) and line 7b's country list are left BLANK: v1 has no input for them, and the FbarFinCen
/// advisory tells the filer in terms that they must decide it themselves. An incomplete Part III is
/// the honest output; a guessed one would not be.
#[test]
fn schedule_b_part3_transcribes_the_filers_own_answers_and_never_guesses_the_fbar() {
    let yes = btctax_forms::fill_schedule_b(
        &sch_b(vec![], vec![], true, false),
        &kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&yes).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());

    // 7a = YES (c1_1[0], on-state "1"); 8 = NO (c1_3[1], on-state "2").
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_1[0]"].id).as_deref(),
        Some("1"),
        "7a answered YES"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_3[1]"].id).as_deref(),
        Some("2"),
        "8 answered NO"
    );
    // ★ The FBAR sub-question is NEVER answered — neither box is set.
    for fbar in ["c1_2[0]", "c1_2[1]"] {
        let fqn = format!("topmostSubform[0].Page1[0].{fbar}");
        assert_eq!(
            checkbox_on(&doc, idx[fqn.as_str()].id),
            None,
            "{fqn}: v1 never answers the FBAR sub-question"
        );
    }
    // …nor line 7b's country list (free text, NOT a Yes/No pair).
    assert_eq!(tv(&yes, "topmostSubform[0].Page1[0].f1_65[0]"), None);

    // The opposite answers flip the boxes — the filer's declaration is what lands on the form.
    let no = btctax_forms::fill_schedule_b(
        &sch_b(vec![], vec![], false, true),
        &kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc2 = load(&no).unwrap();
    let idx2 = index(&collect_fields(&doc2).unwrap());
    assert_eq!(
        checkbox_on(&doc2, idx2["topmostSubform[0].Page1[0].c1_1[1]"].id).as_deref(),
        Some("2"),
        "7a answered NO"
    );
    assert_eq!(
        checkbox_on(&doc2, idx2["topmostSubform[0].Page1[0].c1_3[0]"].id).as_deref(),
        Some("1"),
        "8 answered YES"
    );
}

/// ★ Overflow FAILS CLOSED. Part I holds 14 payers and Part II 15 (the asymmetry is real). Truncating
/// a longer list would leave a form whose printed rows do not add up to its own line 2 — or, if the
/// total were taken from the visible rows instead, a return that UNDERSTATES interest income.
#[test]
fn schedule_b_refuses_more_payers_than_the_form_has_rows() {
    let fifteen: Vec<ScheduleBRow> = (0..15)
        .map(|i| row(&format!("Bank {i}"), dec!(100)))
        .collect();
    let err = btctax_forms::fill_schedule_b(
        &sch_b(fifteen, vec![], false, false),
        &kitchen_sink_header(),
        2024,
    )
    .expect_err("15 interest payers must not fit in 14 rows");
    // A CAPACITY refusal, not a placement failure (`p6-schedule-b-capacity-error-variant`): the cells
    // are mapped correctly, there are simply more payers than rows. Typing it as `Overflow` lets the
    // CLI say "file Schedule B by hand" and lets the all-or-nothing packet name what refused.
    assert!(
        matches!(
            err,
            FormsError::Overflow {
                part: "Schedule B Part I",
                rows: 15,
                capacity: 14
            }
        ),
        "expected a capacity refusal, got {err:?}"
    );

    // …but exactly 14 fits, and 15 dividend payers fit Part II (which genuinely has one more row).
    let fourteen: Vec<ScheduleBRow> = (0..14)
        .map(|i| row(&format!("Bank {i}"), dec!(100)))
        .collect();
    let fifteen_div: Vec<ScheduleBRow> = (0..15)
        .map(|i| row(&format!("Fund {i}"), dec!(200)))
        .collect();
    let pdf = btctax_forms::fill_schedule_b(
        &sch_b(fourteen, fifteen_div, false, false),
        &kitchen_sink_header(),
        2024,
    )
    .expect("14 interest + 15 dividend payers is exactly the form's capacity");
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_31[0]").as_deref(),
        Some("1400")
    ); // L2
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_64[0]").as_deref(),
        Some("3000")
    ); // L6
}

// ────────────────────────── Schedule D (the FULL-RETURN fill) ─────────────────────────────────

fn sd(
    st: Usd,
    lt: Usd,
    st_cf: Usd,
    lt_cf: Usd,
    distr: Usd,
    routing: ScheduleDRouting,
) -> ScheduleDLines {
    ScheduleDLines {
        line3_d: dec!(50000),
        line3_e: dec!(45000),
        line3_h: st + st_cf,
        line6: st_cf,
        line7: st,
        line10_d: dec!(80000),
        line10_e: dec!(70000),
        line10_h: lt + lt_cf - distr,
        line13: distr,
        line14: lt_cf,
        line15: lt,
        line16: st + lt,
        routing,
    }
}

/// ★ The three lines the CRYPTO-SLICE Schedule D omits — 13 (1099-DIV box-2a capital-gain
/// distributions) and 6/14 (capital-loss carryovers) — all appear on the full-return form. Their
/// absence is exactly the defect the P5-C1 refusal covers, and this filler is what retires it.
/// Lines 6 and 14 are PAREN boxes ⇒ positive magnitudes.
#[test]
fn schedule_d_full_fills_the_lines_the_crypto_slice_omits() {
    let lines = sd(
        dec!(1000),
        dec!(15000),
        dec!(2000), // line 6 — ST carryover
        dec!(500),  // line 14 — LT carryover
        dec!(3000), // line 13 — capital gain distributions
        ScheduleDRouting::BothGains,
    );
    let pdf = btctax_forms::fill_schedule_d_full(&lines, &kitchen_sink_header(), 2024).unwrap();
    let g = |fqn: &str| tv(&pdf, fqn);

    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_21[0]").as_deref(),
        Some("2000")
    ); // L6  ★ PAREN
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_41[0]").as_deref(),
        Some("3000")
    ); // L13
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_42[0]").as_deref(),
        Some("500")
    ); // L14 ★ PAREN
    assert_eq!(
        g("topmostSubform[0].Page2[0].f2_01[0]").as_deref(),
        Some("16000")
    ); // L16

    // Neither paren cell may carry a minus sign — the form supplies it.
    for paren in ["f1_21[0]", "f1_42[0]"] {
        let v = g(&format!("topmostSubform[0].Page1[0].{paren}")).unwrap();
        assert!(
            !v.starts_with('-'),
            "{paren} renders inside ( ) — never a minus sign: {v}"
        );
    }
}

/// ★ SPEC §7.2 path 1 — BOTH GAINS: line 17 = Yes, lines 18/19 = 0, line 20 = Yes → QDCGT.
/// Lines 21 and 22 are NOT completed, and the KAT asserts they are genuinely untouched.
#[test]
fn schedule_d_full_routing_both_gains() {
    let pdf = btctax_forms::fill_schedule_d_full(
        &sd(
            dec!(5000),
            dec!(20000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            ScheduleDRouting::BothGains,
        ),
        &kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&pdf).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());

    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_1[0]"].id).as_deref(),
        Some("1"),
        "L17 = Yes"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_02[0]").as_deref(),
        Some("0"),
        "L18 = 0"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_03[0]").as_deref(),
        Some("0"),
        "L19 = 0"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_2[0]"].id).as_deref(),
        Some("1"),
        "L20 = Yes → QDCGT"
    );
    // 21 and 22 are NOT completed on this branch.
    assert_eq!(
        tv(
            &pdf,
            "topmostSubform[0].Page2[0].TagCorrectingSubform[0].f2_04[0]"
        ),
        None,
        "L21 untouched"
    );
    for c in ["c2_3[0]", "c2_3[1]"] {
        let fqn = format!("topmostSubform[0].Page2[0].{c}");
        assert_eq!(
            checkbox_on(&doc, idx[fqn.as_str()].id),
            None,
            "L22 untouched"
        );
    }
}

/// ★ SPEC §7.2 path 2 — SHORT-TERM GAIN / LONG-TERM LOSS (the common crypto year): line 17 = No ⇒
/// skip 18–21 ⇒ line 22, which routes to QDCGT iff there are qualified dividends.
#[test]
fn schedule_d_full_routing_short_gain_long_loss() {
    let pdf = btctax_forms::fill_schedule_d_full(
        &sd(
            dec!(30000),
            dec!(-4000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            ScheduleDRouting::ShortGainLongLoss { line22_yes: true },
        ),
        &kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&pdf).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());

    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_1[1]"].id).as_deref(),
        Some("2"),
        "L17 = No"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_3[0]"].id).as_deref(),
        Some("1"),
        "L22 = Yes"
    );
    // 18, 19, 20 and 21 are SKIPPED — writing a 0 into 18/19 here would answer a question the form
    // told the filer to skip.
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_02[0]"),
        None,
        "L18 skipped"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_03[0]"),
        None,
        "L19 skipped"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_2[0]"].id),
        None,
        "L20 skipped"
    );
    assert_eq!(
        tv(
            &pdf,
            "topmostSubform[0].Page2[0].TagCorrectingSubform[0].f2_04[0]"
        ),
        None,
        "L21 skipped"
    );
}

/// ★ SPEC §7.2 path 3 — NET LOSS: skip 17–20; line 21 carries the §1211(b) offset as a POSITIVE
/// MAGNITUDE (the form pre-prints the parentheses); line 22 is still answered.
#[test]
fn schedule_d_full_routing_net_loss() {
    let pdf = btctax_forms::fill_schedule_d_full(
        &sd(
            dec!(-10000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            ScheduleDRouting::NetLoss {
                line21: dec!(3000),
                line22_yes: false,
            },
        ),
        &kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&pdf).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());

    let l21 = tv(
        &pdf,
        "topmostSubform[0].Page2[0].TagCorrectingSubform[0].f2_04[0]",
    )
    .unwrap();
    assert_eq!(l21, "3000", "the §1211(b) cap");
    assert!(
        !l21.starts_with('-'),
        "★ L21 renders inside ( ) — a minus here would print a GAIN"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_3[1]"].id).as_deref(),
        Some("2"),
        "L22 = No"
    );
    // 17 through 20 are skipped.
    for c in ["c2_1[0]", "c2_1[1]", "c2_2[0]", "c2_2[1]"] {
        let fqn = format!("topmostSubform[0].Page2[0].{c}");
        assert_eq!(
            checkbox_on(&doc, idx[fqn.as_str()].id),
            None,
            "{fqn} skipped on a net loss"
        );
    }
}

/// ★ SPEC §7.2 path 4 — ZERO: 1040 line 7 is -0-; skip 17–21; line 22 is still answered. The branch
/// easiest to forget, and the one that silently routes the whole tax computation if it is wrong.
#[test]
fn schedule_d_full_routing_zero() {
    let pdf = btctax_forms::fill_schedule_d_full(
        &sd(
            dec!(4000),
            dec!(-4000),
            Usd::ZERO,
            Usd::ZERO,
            Usd::ZERO,
            ScheduleDRouting::Zero { line22_yes: true },
        ),
        &kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&pdf).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_01[0]").as_deref(),
        Some("0"),
        "L16 = 0"
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page2[0].c2_3[0]"].id).as_deref(),
        Some("1"),
        "L22 = Yes"
    );
    for c in ["c2_1[0]", "c2_1[1]", "c2_2[0]", "c2_2[1]"] {
        let fqn = format!("topmostSubform[0].Page2[0].{c}");
        assert_eq!(
            checkbox_on(&doc, idx[fqn.as_str()].id),
            None,
            "{fqn} skipped when L16 = 0"
        );
    }
}

/// The paren guard fails closed if a negative ever reaches line 6/14/21 — the thing standing between
/// a future refactor ("a carryover is a loss, so it's negative") and a filed form that reads a capital
/// LOSS carryover as a GAIN.
#[test]
fn schedule_d_full_refuses_a_negative_in_a_parenthesized_cell() {
    let mut lines = sd(
        dec!(1000),
        dec!(15000),
        dec!(2000),
        dec!(500),
        dec!(3000),
        ScheduleDRouting::BothGains,
    );
    lines.line14 = dec!(-500);
    let err = fill_schedule_d_full_with_map(
        &lines,
        &kitchen_sink_header(),
        &btctax_forms::ScheduleDMap::ty2024(),
    )
    .expect_err("a negative in a paren box must fail closed");
    assert!(format!("{err}").contains("line 14"), "{err}");
}

// ─────────────────────────── Form 1040 (the FULL-RETURN fill) ─────────────────────────────────

fn f1040() -> Form1040Lines {
    Form1040Lines {
        line1a: dec!(120000),
        line2a: dec!(1234),
        line1z: dec!(120000),
        line2b: dec!(2000),
        line3a: dec!(3000), // ★ SUBLINE column — the preferential slice
        line3b: dec!(4000),
        line7: dec!(25000),
        line8: dec!(5000),
        line9: dec!(156000),
        line10: dec!(3000),
        line11: dec!(153000),
        line12: dec!(14600),
        line13: dec!(800),
        line14: dec!(15400),
        line15: dec!(137600),
        line16: dec!(26000),
        line17: Usd::ZERO,
        line18: dec!(26000),
        line19: Usd::ZERO,
        line20: dec!(287),
        line21: dec!(287),
        line22: dec!(25713),
        line23: dec!(1406),
        line24: dec!(27119),
        line25a: dec!(24000),
        line25b: dec!(300),
        line25c: dec!(180),
        line25d: dec!(24480),
        line26: dec!(500),
        line31: dec!(1235),
        line32: dec!(1235),
        line33: dec!(26215),
        line34: Usd::ZERO,
        line37: dec!(904),
        digital_asset_yes: true,
    }
}

#[test]
fn form_1040_full_fills_every_line_and_reads_back() {
    let pdf = btctax_forms::fill_form_1040_full(
        &f1040(),
        &kitchen_sink_header(),
        FilingStatus::Single,
        2024,
    )
    .unwrap();
    let g = |fqn: &str| tv(&pdf, fqn);

    // Page 1 — income → taxable income.
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_41[0]").as_deref(),
        Some("120000")
    ); // L1z
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_43[0]").as_deref(),
        Some("2000")
    ); // L2b
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_44[0]").as_deref(),
        Some("3000")
    ); // L3a ★ SUBLINE
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_45[0]").as_deref(),
        Some("4000")
    ); // L3b
    assert_eq!(
        g("topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].f1_52[0]").as_deref(),
        Some("25000")
    ); // L7
       // ★ f1_57 is LINE 12 on the 2024 form (it is line 1z on the 2025 one — SPEC §7.4).
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_57[0]").as_deref(),
        Some("14600"),
        "L12 = f1_57 on TY2024"
    );
    assert_eq!(
        g("topmostSubform[0].Page1[0].f1_60[0]").as_deref(),
        Some("137600")
    ); // L15 taxable income

    // Page 2 — tax → total tax → payments → owed.
    assert_eq!(
        g("topmostSubform[0].Page2[0].f2_10[0]").as_deref(),
        Some("27119")
    ); // L24 TOTAL TAX
    assert_eq!(
        g("topmostSubform[0].Page2[0].f2_13[0]").as_deref(),
        Some("180")
    ); // L25c (8959 L24)
    assert_eq!(
        g("topmostSubform[0].Page2[0].f2_22[0]").as_deref(),
        Some("26215")
    ); // L33 TOTAL PAYMENTS
    assert_eq!(
        g("topmostSubform[0].Page2[0].f2_28[0]").as_deref(),
        Some("904")
    ); // L37 AMOUNT OWED

    // The direct-deposit block is NEVER filled — a refund arrives as a paper check, and the
    // RefundByPaperCheck advisory says so.
    assert_eq!(
        g("topmostSubform[0].Page2[0].RoutingNo[0].f2_25[0]"),
        None,
        "35b routing untouched"
    );
    assert_eq!(
        g("topmostSubform[0].Page2[0].AccountNo[0].f2_26[0]"),
        None,
        "35c/d account untouched"
    );
    // Lines 27-30 (EIC, additional CTC, AOC) are §3.4 conservative omissions — BLANK, never 0.
    for omitted in ["f2_16[0]", "f2_17[0]", "f2_18[0]", "f2_19[0]"] {
        let fqn = format!("topmostSubform[0].Page2[0].{omitted}");
        assert_eq!(g(&fqn), None, "{fqn} (conservative omission) must be blank");
    }

    // The Digital-Asset question is answered YES; "No" is never checked.
    let doc = load(&pdf).unwrap();
    let idx = index(&collect_fields(&doc).unwrap());
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_5[0]"].id).as_deref(),
        Some("1")
    );
    assert_eq!(
        checkbox_on(&doc, idx["topmostSubform[0].Page1[0].c1_5[1]"].id),
        None,
        "never 'No'"
    );
}

/// ★★ **THE FILING-STATUS NAME COLLISION.** Two distinct fields are both called `c1_3[0]` (Single,
/// and Head of household) and two are both called `c1_3[1]` (MFJ, and QSS) — distinguished ONLY by
/// their parent subform. A map keyed on the leaf name would silently check the WRONG FILING STATUS,
/// which changes the standard deduction, every bracket and every threshold on the return.
///
/// This drives all five statuses and asserts, for each, that the RIGHT fully-qualified field carries
/// the RIGHT on-state and that no OTHER filing-status box is set. The distinct on-states
/// (1=Single, 2=HoH, 3=MFJ, 4=MFS, 5=QSS) corroborate the mapping independently of the field names.
#[test]
fn form_1040_full_filing_status_boxes_do_not_collide() {
    const SINGLE: &str = "topmostSubform[0].Page1[0].FilingStatus_ReadOrder[0].c1_3[0]";
    const HOH: &str = "topmostSubform[0].Page1[0].c1_3[0]"; // ← same LEAF name as Single!
    const MFJ: &str = "topmostSubform[0].Page1[0].FilingStatus_ReadOrder[0].c1_3[1]";
    const MFS: &str = "topmostSubform[0].Page1[0].FilingStatus_ReadOrder[0].c1_3[2]";
    const QSS: &str = "topmostSubform[0].Page1[0].c1_3[1]"; // ← same LEAF name as MFJ!
    let all = [SINGLE, HOH, MFJ, MFS, QSS];

    for (status, want_field, want_on) in [
        (FilingStatus::Single, SINGLE, "1"),
        (FilingStatus::HoH, HOH, "2"),
        (FilingStatus::Mfj, MFJ, "3"),
        (FilingStatus::Mfs, MFS, "4"),
        (FilingStatus::Qss, QSS, "5"),
    ] {
        let pdf = btctax_forms::fill_form_1040_full(&f1040(), &kitchen_sink_header(), status, 2024)
            .unwrap();
        let doc = load(&pdf).unwrap();
        let idx = index(&collect_fields(&doc).unwrap());

        assert_eq!(
            checkbox_on(&doc, idx[want_field].id).as_deref(),
            Some(want_on),
            "{status:?} must set {want_field} to on-state {want_on}"
        );
        for other in all {
            if other != want_field {
                assert_eq!(
                    checkbox_on(&doc, idx[other].id),
                    None,
                    "{status:?} must NOT set {other} — exactly one filing-status box may be checked"
                );
            }
        }
    }
}

/// 1040 line 7 on a loss year carries a LEADING MINUS (SPEC §3.2) — unlike Schedule D's lines
/// 6/14/21, which are parenthesized boxes carrying magnitudes. Two conventions, two forms that
/// reference each other.
#[test]
fn form_1040_full_line7_loss_prints_a_leading_minus() {
    let mut lines = f1040();
    lines.line7 = dec!(-3000); // the §1211(b)-limited loss
    let pdf = btctax_forms::fill_form_1040_full(
        &lines,
        &kitchen_sink_header(),
        FilingStatus::Single,
        2024,
    )
    .unwrap();
    let v = tv(
        &pdf,
        "topmostSubform[0].Page1[0].Line4a-11_ReadOrder[0].f1_52[0]",
    )
    .unwrap();
    assert_eq!(v, "-3000");
    assert!(
        v.starts_with('-'),
        "1040 L7 is signed with a LEADING MINUS, not parentheses"
    );
}

/// Same-column swap on the 1040 (lines 9 and 15 are both AMOUNT, far apart in y) → the descent leg of
/// the oracle catches it and the fill FAILS CLOSED.
#[test]
fn form_1040_full_same_column_swap_fails_closed() {
    let mut map = btctax_forms::Form1040Map::ty2024();
    std::mem::swap(&mut map.line9, &mut map.line15);
    let err =
        fill_form_1040_full_with_map(&f1040(), &kitchen_sink_header(), FilingStatus::Single, &map)
            .expect_err("a same-column swap must fail closed");
    assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
}

// ── /MaxLen: the comb-cell guard (P6.2) ─────────────────────────────────────────────────────────

/// ★ The 1040's SSN cells are **9-character COMB cells** — the PDF says so itself (`/MaxLen 9`, comb
/// flag set), and that answers the hyphens-or-digits question from the primary source rather than by
/// guessing: a formatted `123-45-6789` is ELEVEN characters and would be silently truncated by a
/// viewer (or splayed across the wrong comb teeth). We write the nine bare digits.
///
/// This KAT pins the fact, so a future year whose form changed its cell width cannot slip through.
#[test]
fn the_1040_ssn_cells_are_nine_character_comb_cells() {
    let pdf = std::fs::read("forms/2024/f1040.pdf").unwrap();
    let doc = load(&pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let by = |fqn: &str| {
        fields
            .iter()
            .find(|f| f.fqn == fqn)
            .unwrap_or_else(|| panic!("{fqn} exists"))
            .max_len
    };
    // Taxpayer SSN, spouse SSN, and every dependent SSN: nine characters, no room for hyphens.
    assert_eq!(by("topmostSubform[0].Page1[0].f1_06[0]"), Some(9));
    assert_eq!(by("topmostSubform[0].Page1[0].f1_09[0]"), Some(9));
    assert_eq!(
        by("topmostSubform[0].Page1[0].Table_Dependents[0].Row1[0].f1_21[0]"),
        Some(9)
    );
    // A name cell is NOT length-capped — only the combs are.
    assert_eq!(by("topmostSubform[0].Page1[0].f1_04[0]"), None);
}

// ── Identity headers (p6-form-identity-header, P6.2) ────────────────────────────────────────────

/// ★ Every schedule carries the taxpayer's name and SSN. Without them the money lines are right and
/// the form is still not FILABLE — an unnamed Schedule C is not a return.
///
/// The SSN is written **hyphenated** here because these cells declare `/MaxLen 11`, and it is written
/// as bare digits on the 1040 because that form's cells declare `/MaxLen 9`. Same value, two
/// renderings, each decided by the form itself — never extrapolated from a sibling form.
#[test]
fn every_schedule_carries_the_name_and_ssn_header() {
    let h = kitchen_sink_header();
    let f8959 = form_8959_lines(
        FilingStatus::Mfj,
        dec!(280000),
        dec!(4240),
        Some(&se_mining_60k_mfj()),
    );
    let pdf = fill_form_8959_with_map(&f8959, &h, &Form8959Map::ty2024())
        .unwrap()
        .expect("the form is required");

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_1[0]").as_deref(),
        Some("John Doe & Jane Doe"),
        "the joint name line"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_2[0]").as_deref(),
        Some("123-45-6789"),
        "hyphenated — this cell is /MaxLen 11"
    );
}

/// ★ The GATING KAT (`p6-aged-blind-checkboxes-missing`). A nonstandard standard deduction is validated
/// by the IRS by COUNTING the checked §63(f) boxes — so a 1040 whose L12 carries the aged/blind addition
/// with ZERO boxes ticked fails the Service's own arithmetic cross-check. Here: taxpayer aged + blind,
/// spouse blind ⇒ THREE boxes, and all three must be on the filed page.
#[test]
fn the_1040_prints_the_aged_blind_boxes_its_line_12_depends_on() {
    use btctax_core::tax::packet::ReturnHeader;
    use btctax_core::tax::return_inputs::{Person, ReturnInputs};

    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Mfj,
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "John".into(),
        last_name: "Doe".into(),
        ssn: "123456789".into(),
        date_of_birth: Some(time::macros::date!(1955 - 03 - 02)), // 65+
        blind: true,
        ..Default::default()
    };
    ri.header.spouse = Some(Person {
        first_name: "Jane".into(),
        last_name: "Doe".into(),
        ssn: "987654321".into(),
        blind: true,
        ..Default::default()
    });
    let h = ReturnHeader::build(&ri, 2024).unwrap();
    assert_eq!(h.aged_blind.count(), 3, "the fixture claims three boxes");

    let pdf = btctax_forms::fill_form_1040_full(&f1040(), &h, FilingStatus::Mfj, 2024).unwrap();
    let on = |fqn: &str| box_on(&pdf, fqn);

    assert!(
        on("topmostSubform[0].Page1[0].c1_9[0]"),
        "taxpayer born before 1960"
    );
    assert!(on("topmostSubform[0].Page1[0].c1_10[0]"), "taxpayer blind");
    assert!(
        !on("topmostSubform[0].Page1[0].c1_11[0]"),
        "spouse is NOT aged"
    );
    assert!(on("topmostSubform[0].Page1[0].c1_12[0]"), "spouse blind");
}

/// The 1040 carries the taxpayer's identity: names, SSN (BARE digits — these cells are /MaxLen 9 combs,
/// unlike the schedules' /MaxLen 11), address, and the dependents rows.
#[test]
fn the_1040_prints_names_ssns_address_and_dependents() {
    let h = kitchen_sink_header();
    let pdf = btctax_forms::fill_form_1040_full(&f1040(), &h, FilingStatus::Mfj, 2024).unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_04[0]").as_deref(),
        Some("John")
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_05[0]").as_deref(),
        Some("Doe")
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_06[0]").as_deref(),
        Some("123456789"),
        "bare digits — this cell is a 9-character comb"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_09[0]").as_deref(),
        Some("987654321")
    );
    assert_eq!(
        tv(
            &pdf,
            "topmostSubform[0].Page1[0].Address_ReadOrder[0].f1_10[0]"
        )
        .as_deref(),
        Some("100 Main St")
    );
    // The dependent's row: name, SSN (digits), relationship — and NO credit box (v1 omits CTC/ODC).
    assert_eq!(
        tv(
            &pdf,
            "topmostSubform[0].Page1[0].Table_Dependents[0].Row1[0].f1_20[0]"
        )
        .as_deref(),
        Some("Sam Doe")
    );
    assert_eq!(
        tv(
            &pdf,
            "topmostSubform[0].Page1[0].Table_Dependents[0].Row1[0].f1_21[0]"
        )
        .as_deref(),
        Some("111223333")
    );
    assert!(
        !box_on(&pdf, "topmostSubform[0].Page1[0].Table_Dependents[0].Row1[0].c1_14[0]"),
        "the CTC box stays UNCHECKED — v1's L19 is zero, and a ticked credit box beside a zero credit \
         is a form contradicting itself"
    );
}

/// More dependents than the form physically holds REFUSES rather than printing the first four. The
/// IRS's own remedy is a continuation statement, which is a synthetic page generator v1 does not have
/// (same posture as Schedule B's >14-payer refusal). Printing four of five would silently file a return
/// that misstates the household.
#[test]
fn more_dependents_than_the_form_holds_fails_closed() {
    use btctax_core::tax::packet::ReturnHeader;
    use btctax_core::tax::return_inputs::{Dependent, Person, ReturnInputs};

    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "John".into(),
        last_name: "Doe".into(),
        ssn: "123456789".into(),
        ..Default::default()
    };
    ri.header.dependents = (0..5)
        .map(|i| Dependent {
            name: format!("Kid {i}"),
            ssn: format!("11122333{i}"),
            relationship: "Child".into(),
            ..Default::default()
        })
        .collect();
    let h = ReturnHeader::build(&ri, 2024).unwrap();

    let err = btctax_forms::fill_form_1040_full(&f1040(), &h, FilingStatus::Single, 2024)
        .expect_err("five dependents must not silently become four");
    assert!(
        matches!(
            err,
            FormsError::Overflow {
                rows: 5,
                capacity: 4,
                ..
            }
        ),
        "expected a capacity refusal, got {err:?}"
    );
}

// ── The ARCH-P6.3a Q7 sweep: each captured input now reaches its CELL ────────────────────────────

/// The three 1040 header items that reached the arithmetic (or the vault) but never the page: line 2a
/// (tax-exempt interest — the IRS document-matches 1099-INT box 8), the occupation cells, and the
/// **IP PIN**, whose absence gets a paper return REJECTED when one was issued.
#[test]
fn the_1040_prints_tax_exempt_interest_the_occupations_and_the_ip_pin() {
    use btctax_core::tax::packet::ReturnHeader;
    use btctax_core::tax::return_inputs::{Person, ReturnInputs};

    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "Pat".into(),
        last_name: "Roe".into(),
        ssn: "222334444".into(),
        occupation: "Teacher".into(),
        ..Default::default()
    };
    ri.header.ip_pin = Some("123456".into());
    let h = ReturnHeader::build(&ri, 2024).unwrap();

    let mut lines = f1040();
    lines.line2a = dec!(1234);
    let pdf = btctax_forms::fill_form_1040_full(&lines, &h, FilingStatus::Single, 2024).unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_42[0]").as_deref(),
        Some("1234"),
        "line 2a — tax-exempt interest"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_33[0]").as_deref(),
        Some("Teacher"),
        "the occupation cell"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_34[0]").as_deref(),
        Some("123456"),
        "★ the IP PIN — a paper return that omits an ISSUED one is rejected"
    );
}

/// Schedule A's two ELECTION checkboxes. Core honoured both in the arithmetic; the form showed neither.
/// Without the line-18 box especially, the Service's math-error unit may "correct" a §63(e) return back
/// to the standard deduction — silently undoing the filer's own election.
#[test]
fn schedule_a_prints_the_sales_tax_and_force_itemize_election_boxes() {
    let mut lines = sch_a_lines();
    lines.line5a_is_sales_tax = true;
    lines.line18_elects_smaller = true;
    let pdf = btctax_forms::fill_schedule_a(&lines, &kitchen_sink_header(), 2024).unwrap();

    assert!(
        box_on(&pdf, "topmostSubform[0].Page1[0].c1_1[0]"),
        "the §164(b)(5) sales-tax election"
    );
    assert!(
        box_on(
            &pdf,
            "topmostSubform[0].Page1[0].Line18_ReadOrder[0].c1_3[0]"
        ),
        "the §63(e) itemize-below-the-standard election"
    );

    // …and an ordinary itemizing return checks neither.
    let plain = sch_a_lines();
    let pdf = btctax_forms::fill_schedule_a(&plain, &kitchen_sink_header(), 2024).unwrap();
    assert!(!box_on(&pdf, "topmostSubform[0].Page1[0].c1_1[0]"));
    assert!(!box_on(
        &pdf,
        "topmostSubform[0].Page1[0].Line18_ReadOrder[0].c1_3[0]"
    ));
}

/// Schedule C's lines A, B and F — captured expressly for those cells. A Schedule C with a blank
/// "Principal business or profession" is incomplete on its face.
#[test]
fn schedule_c_prints_its_business_naics_and_accounting_method() {
    let lines = ScheduleCLines {
        line_a_business: "Bitcoin mining".into(),
        line_b_naics: "518210".into(),
        line_f_accrual: false, // Cash
        line1: dec!(60000),
        line3: dec!(60000),
        line5: dec!(60000),
        line7: dec!(60000),
        line28: dec!(8000),
        line29: dec!(52000),
        line31: dec!(52000),
    };
    let pdf = btctax_forms::fill_schedule_c(&lines, &kitchen_sink_header(), 2024).unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_3[0]").as_deref(),
        Some("Bitcoin mining"),
        "line A"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].BComb[0].f1_4[0]").as_deref(),
        Some("518210"),
        "line B — the NAICS code (a 6-character comb)"
    );
    assert!(
        box_on(&pdf, "topmostSubform[0].Page1[0].c1_1[0]"),
        "line F — Cash, the captured method"
    );
    assert!(
        !box_on(&pdf, "topmostSubform[0].Page1[0].c1_1[1]"),
        "…and NOT accrual"
    );
}

// ── Full-return Schedule SE (ARCH-P6.3a D5/D6) ──────────────────────────────────────────────────

/// ★ The full-return Schedule SE prints WHOLE DOLLARS and is headed by the **proprietor** — "Name of
/// person with self-employment income" — not the return's joint name line. On a joint return with a
/// spouse-owned business that is the SPOUSE, with the SPOUSE's SSN; filing it under the taxpayer would
/// attribute the self-employment tax to the wrong person.
///
/// Its line 12 is what Schedule 2 line 4 carries, so the two must be the same integer.
#[test]
fn the_full_return_schedule_se_prints_whole_dollars_under_the_proprietors_name() {
    use btctax_core::tax::packet::ReturnHeader;
    use btctax_core::tax::printed::ScheduleSeLines;
    use btctax_core::tax::return_inputs::{Owner, Person, ReturnInputs, ScheduleCInputs};

    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Mfj,
        schedule_c: Some(ScheduleCInputs {
            owner: Owner::Spouse, // ★ the SPOUSE owns the business
            ..Default::default()
        }),
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "John".into(),
        last_name: "Doe".into(),
        ssn: "123456789".into(),
        ..Default::default()
    };
    ri.header.spouse = Some(Person {
        first_name: "Jane".into(),
        last_name: "Roe".into(),
        ssn: "987654321".into(),
        ..Default::default()
    });
    let h = ReturnHeader::build(&ri, 2024).unwrap();

    let lines = ScheduleSeLines {
        line2: dec!(52000),
        line3: dec!(52000),
        line4a: dec!(48022),
        line4c: dec!(48022),
        line6: dec!(48022),
        line8a: dec!(90000),
        line8d: dec!(90000),
        line9: dec!(78600),
        line10: dec!(5955),
        line11: dec!(1393),
        line12: dec!(7348), // = the PRINTED 10 + 11 → Schedule 2 line 4
        line13: dec!(3674),
    };
    let pdf = btctax_forms::fill_schedule_se_full(&lines, &h, 2024).unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_1[0]").as_deref(),
        Some("Jane Roe"),
        "★ the PROPRIETOR — the spouse who owns the business, not the joint name line"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_2[0]").as_deref(),
        Some("987-65-4321"),
        "…and the PROPRIETOR's SSN"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_21[0]").as_deref(),
        Some("7348"),
        "line 12 — whole dollars, and the figure Schedule 2 line 4 carries"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_22[0]").as_deref(),
        Some("3674"),
        "line 13 → Schedule 1 line 15"
    );
    // No cents anywhere on the filed page — the §3.1 election is all-or-nothing.
    for fqn in [
        "topmostSubform[0].Page1[0].f1_19[0]",
        "topmostSubform[0].Page1[0].f1_20[0]",
    ] {
        let v = tv(&pdf, fqn).unwrap_or_default();
        assert!(!v.contains('.'), "{fqn} printed cents: {v:?}");
    }
}

// ── Full-return Form 8949 + the CROSS-PDF byte oracle (ARCH-P6.3a D2/D3) ────────────────────────

/// ★ The cross-PDF oracle: Schedule D's line-3(d) **cell text** equals Form 8949's Part I column-(d)
/// **total cell text** — read back out of two separately serialized PDFs.
///
/// This is the one leg no other test covers: the core KATs prove the CHAIN composes, and the read-back
/// KATs prove each form transcribes its own chain, but only this proves the composition SURVIVED
/// transcription into the two documents a filer actually staples together.
///
/// The fixture is deliberately discriminating (KAT-9, one level deeper): rows of $100.50 and $200.50
/// print 101 + 201 = **302**, while re-rounding the exact aggregate (301.00) would print 301.
#[test]
fn schedule_d_line3_cell_text_equals_the_8949s_printed_column_total() {
    use btctax_core::forms::{Form8949Box, Form8949Part, Form8949Row};
    use btctax_core::identity::WalletId;
    use btctax_core::tax::printed::form_8949_printed;

    let row = |proceeds: Usd, basis: Usd| Form8949Row {
        part: Form8949Part::ShortTerm,
        box_: Form8949Box::C,
        box_needs_review: false,
        description: "1.00000000 BTC".into(),
        date_acquired: time::macros::date!(2024 - 01 - 02),
        date_sold: time::macros::date!(2024 - 05 - 01),
        proceeds,
        cost_basis: basis,
        adjustment_code: String::new(),
        adjustment_amount: Usd::ZERO,
        gain: proceeds - basis,
        wallet: WalletId::SelfCustody { label: "w".into() },
        disposition_kind: btctax_core::event::DisposeKind::Sell,
    };
    let printed = form_8949_printed(&[row(dec!(100.50), Usd::ZERO), row(dec!(200.50), Usd::ZERO)])
        .expect("there are rows");
    assert_eq!(
        printed.st_totals.proceeds_d,
        dec!(302),
        "Σ of the PRINTED rows"
    );

    // Fill the 8949…
    let pdf_8949 = btctax_forms::fill_8949_full(&printed, &kitchen_sink_header(), 2024).unwrap();
    let map_8949 = btctax_forms::Form8949Map::ty2024();
    let part1 = map_8949
        .parts
        .iter()
        .find(|p| p.term == "short")
        .expect("Part I");
    let total_8949 =
        tv(&pdf_8949, &part1.totals.proceeds_d).expect("the Part I (d) total cell is filled");

    // …and the Schedule D that CITES it.
    let mut ar_sd = sd(
        dec!(302),
        Usd::ZERO,
        Usd::ZERO,
        Usd::ZERO,
        Usd::ZERO,
        ScheduleDRouting::BothGains,
    );
    ar_sd.line3_d = printed.st_totals.proceeds_d;
    ar_sd.line3_e = printed.st_totals.cost_e;
    ar_sd.line3_h = printed.st_totals.gain_h;
    let pdf_sd = btctax_forms::fill_schedule_d_full(&ar_sd, &kitchen_sink_header(), 2024).unwrap();
    let map_sd = btctax_forms::ScheduleDMap::ty2024();
    let cell_sd = tv(&pdf_sd, &map_sd.line3.proceeds_d).expect("Schedule D line 3(d) is filled");

    assert_eq!(
        cell_sd, total_8949,
        "★ the filed Schedule D and the filed Form 8949 must carry the SAME characters in the cells \
         that cite each other"
    );
    assert_eq!(
        cell_sd, "302",
        "…and it is the sum of the PRINTED rows, not round(exact)"
    );
}

/// The full-return Form 8283 carries the FILER's identity ("Name(s) shown on your income tax return")
/// and whole-dollar money columns. The crypto slice writes neither — its 8283 rides beside a return
/// btctax did not produce — and that difference is exactly why the two paths stay separate.
#[test]
fn the_full_return_8283_names_the_filer_and_prints_whole_dollars() {
    use btctax_core::forms::{Form8283HowAcquired, Form8283Row, Form8283Section};
    use btctax_core::tax::printed::form_8283_printed;

    let row = Form8283Row {
        section: Some(Form8283Section::A),
        description: "0.50000000 BTC".into(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: time::macros::date!(2021 - 03 - 01),
        date_contributed: time::macros::date!(2024 - 07 - 04),
        cost_basis: dec!(1200.49),
        fmv: dec!(30000.50),
        claimed_deduction: Some(dec!(30000.50)),
        fmv_method: String::new(),
        donee: "Habitat".into(),
        appraiser: String::new(),
        needs_review: false,
        details: None,
    };
    let printed = form_8283_printed(&[row]).expect("there is a donation");
    let pdf = btctax_forms::fill_form_8283_full(&printed, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("a donation ⇒ an 8283");

    assert_eq!(
        tv(&pdf, "Form8283[0].Page1[0].f1_01[0]").as_deref(),
        Some("John Doe & Jane Doe"),
        "the filer's name line"
    );
    assert_eq!(
        tv(&pdf, "Form8283[0].Page1[0].f1_02[0]").as_deref(),
        Some("123-45-6789"),
        "…and their identifying number (this cell is /MaxLen 11 ⇒ hyphenated)"
    );
    // The money columns are whole dollars — no cents anywhere on the filed page.
    assert_eq!(printed.rows()[0].cost_basis, dec!(1200));
    assert_eq!(
        printed.rows()[0].fmv,
        dec!(30001),
        "30,000.50 rounds at the cell"
    );
}

// ── fill_full_return — the assembled packet (P6.3b) ─────────────────────────────────────────────

/// ★ The packet is ALL-OR-NOTHING. If any member filler refuses, ZERO bytes come back: a 1040 whose
/// line 2b cites a Schedule B that is not attached is a wrong return, so partial emission is a
/// fail-OPEN. Here Schedule B overflows (15 payers, 14 rows) and the WHOLE packet refuses — not just
/// Schedule B.
#[test]
fn the_packet_is_all_or_nothing_when_a_member_filler_refuses() {
    use btctax_core::tax::packet::assemble_printed_return;
    use btctax_core::tax::return_1040::assemble_absolute;
    use btctax_core::tax::return_inputs::Form1099Int;
    use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};

    let (mut ri, state) = kitchen_sink_household();
    // 15 interest payers — one more than Schedule B Part I can hold.
    ri.int_1099 = (0..15)
        .map(|i| Form1099Int {
            payer: format!("Bank {i}"),
            box1_interest: dec!(200),
            ..Default::default()
        })
        .collect();

    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
    let pr = assemble_printed_return(
        &ri,
        &state,
        &std::collections::BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2024,
    )
    .unwrap();

    let err = btctax_forms::fill_full_return(&pr, 2024)
        .expect_err("an overflowing Schedule B must refuse the WHOLE packet");
    assert!(
        matches!(
            err,
            FormsError::Overflow {
                part: "Schedule B Part I",
                ..
            }
        ),
        "the packet names WHICH form refused: {err:?}"
    );
}

/// The kitchen-sink household files EVERY form, and the packet comes back in IRS **Attachment Sequence
/// No.** order — the filer's stapling order, printed on the forms themselves (Sch 1 = 01, Sch 2 = 02,
/// Sch 3 = 03, Sch A = 07, Sch B = 08, Sch C = 09, Sch D = 12, 8949 = 12A, Sch SE = 17, 8995 = 55,
/// 8959 = 71, 8960 = 72, 8283 = 155), with the 1040 itself first.
#[test]
fn the_packet_emits_every_required_form_in_attachment_sequence_order() {
    use btctax_core::tax::packet::assemble_printed_return;
    use btctax_core::tax::return_1040::assemble_absolute;
    use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};

    let (ri, state) = kitchen_sink_household();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
    let pr = assemble_printed_return(
        &ri,
        &state,
        &std::collections::BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2024,
    )
    .unwrap();

    let packet = btctax_forms::fill_full_return(&pr, 2024).unwrap();
    let names: Vec<&str> = packet.iter().map(|f| f.name.as_str()).collect();

    assert_eq!(
        names,
        vec![
            "f1040",
            "f1040s1",
            "f1040s2",
            "f1040s3",
            "f1040sa",
            "f1040sb",
            "f1040sc",
            "schedule_d",
            "f8949",
            "schedule_se",
            "f8995",
            "f8959",
            "f8960",
        ],
        "the 1040 first, then ascending Attachment Sequence No."
    );
    for form in &packet {
        assert!(!form.bytes.is_empty(), "{} produced no bytes", form.name);
    }
}

/// A plain W-2 household files a 1040 and NOTHING else — the packet's `None` arms are as load-bearing
/// as its `Some` ones (a blank Schedule C stapled to a return with no business is a wrong return).
#[test]
fn a_w2_only_household_files_a_1040_and_nothing_else() {
    use btctax_core::tax::packet::assemble_printed_return;
    use btctax_core::tax::return_1040::assemble_absolute;
    use btctax_core::tax::testonly::{ty2024_params, ty2024_table, w2_only_household};

    let (ri, state) = w2_only_household();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
    let pr = assemble_printed_return(
        &ri,
        &state,
        &std::collections::BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2024,
    )
    .unwrap();

    let packet = btctax_forms::fill_full_return(&pr, 2024).unwrap();
    let names: Vec<&str> = packet.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["f1040"]);
}

/// ★ The REPORT and the FILED PDF carry the same "amount you owe" (ARCH-P6 Q3).
///
/// Line 37 is not an analytical figure — it is an instruction to write a check. A tool that says
/// $12,345.67 in the terminal and prints $12,347 on the filed form has produced TWO authoritative
/// answers to "what do I pay". So the report renders the PRINTED chain, and this KAT reads the figure
/// back out of the actual PDF to prove they are the same characters.
#[test]
fn the_reports_amount_owed_is_the_figure_printed_on_the_filed_1040() {
    use btctax_core::tax::packet::assemble_printed_forms;
    use btctax_core::tax::return_1040::assemble_absolute;
    use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};

    let (ri, state) = kitchen_sink_household();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &ty2024_table(), 2024);
    let printed = assemble_printed_forms(
        &ri,
        &state,
        &std::collections::BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2024,
    );

    // The figure the report prints (it renders `printed.f1040`, not `ar`).
    let reported = printed.f1040.line37;

    // …and the figure on the filed page.
    let pdf = btctax_forms::fill_form_1040_full(
        &printed.f1040,
        &kitchen_sink_header(),
        FilingStatus::Mfj,
        2024,
    )
    .unwrap();
    let map = btctax_forms::Form1040Map::ty2024();
    let fqn = match map.line37.as_ref().expect("line 37 is mapped") {
        MoneyCell::Single(f) => f.clone(),
        MoneyCell::Pair(p) => p.dollars_field.clone(),
    };
    let cell = tv(&pdf, &fqn).expect("line 37 is filled");

    assert_eq!(
        cell,
        reported.to_string(),
        "★ the terminal and the filed form must not give two different answers to 'what do I pay'"
    );
    // …and it is a whole dollar, per the §3.1 election.
    assert!(
        !cell.contains('.'),
        "the filed figure carries no cents: {cell:?}"
    );
}

// ── Fable P6 gate review r1 — the folded findings, pinned ────────────────────────────────────────

/// **I1.** 1040 line 1a prints. Without it the filed 1z sat above an EMPTY operand column, and the
/// form's own "Add lines 1a through 1h" summed blanks to 0 ≠ 1z — on the line the Service
/// document-matches against your W-2s.
#[test]
fn the_1040_prints_line_1a_under_the_1z_that_adds_it_up() {
    let mut lines = f1040();
    lines.line1a = dec!(120000);
    lines.line1z = dec!(120000);
    let pdf =
        btctax_forms::fill_form_1040_full(&lines, &kitchen_sink_header(), FilingStatus::Mfj, 2024)
            .unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_32[0]").as_deref(),
        Some("120000"),
        "L1a — Σ W-2 box 1"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_41[0]").as_deref(),
        Some("120000"),
        "L1z — 'Add lines 1a through 1h', which now has an operand to add"
    );
}

/// **I3.** The packet's Form 8949 is NAMED — on BOTH pages. It is a two-page detail attachment and each
/// page carries its own "Name(s) shown on return" + SSN header. An unnamed 8949 is not filable.
#[test]
fn the_full_return_8949_is_named_on_both_pages() {
    use btctax_core::forms::{Form8949Box, Form8949Part, Form8949Row};
    use btctax_core::identity::WalletId;
    use btctax_core::tax::printed::form_8949_printed;

    let row = |part| Form8949Row {
        part,
        box_: if part == Form8949Part::ShortTerm {
            Form8949Box::C
        } else {
            Form8949Box::F
        },
        box_needs_review: false,
        description: "1.00000000 BTC".into(),
        date_acquired: time::macros::date!(2020 - 01 - 02),
        date_sold: time::macros::date!(2024 - 05 - 01),
        proceeds: dec!(30000),
        cost_basis: dec!(10000),
        adjustment_code: String::new(),
        adjustment_amount: Usd::ZERO,
        gain: dec!(20000),
        wallet: WalletId::SelfCustody { label: "w".into() },
        disposition_kind: btctax_core::event::DisposeKind::Sell,
    };
    // Both parts, so both pages of the form are in play.
    let printed =
        form_8949_printed(&[row(Form8949Part::ShortTerm), row(Form8949Part::LongTerm)]).unwrap();
    let pdf = btctax_forms::fill_8949_full(&printed, &kitchen_sink_header(), 2024).unwrap();

    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_1[0]").as_deref(),
        Some("John Doe & Jane Doe"),
        "page 1 name"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page1[0].f1_2[0]").as_deref(),
        Some("123-45-6789"),
        "page 1 SSN (/MaxLen 11 ⇒ hyphenated)"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_1[0]").as_deref(),
        Some("John Doe & Jane Doe"),
        "★ page 2 carries its OWN header — this is the page that was missed"
    );
    assert_eq!(
        tv(&pdf, "topmostSubform[0].Page2[0].f2_2[0]").as_deref(),
        Some("123-45-6789"),
        "page 2 SSN"
    );
}

/// **I4.** The full-return Schedule D answers the QOF question — "Did you dispose of any investment(s)
/// in a qualified opportunity fund…?" — exactly as the crypto slice always has. A mandatory header
/// question left blank on identical ledger knowledge (bitcoin-only, no QOF) is an incomplete form.
#[test]
fn the_full_return_schedule_d_answers_the_qof_question() {
    let lines = sd(
        dec!(5000),
        dec!(20000),
        Usd::ZERO,
        Usd::ZERO,
        Usd::ZERO,
        ScheduleDRouting::BothGains,
    );
    let pdf = btctax_forms::fill_schedule_d_full(&lines, &kitchen_sink_header(), 2024).unwrap();

    assert!(
        box_on(&pdf, "topmostSubform[0].Page1[0].c1_1[1]"),
        "the QOF question is answered NO, as the slice answers it"
    );
}
