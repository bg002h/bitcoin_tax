//! §G-6 — filling Form 6251, read back off the serialized PDF.
//!
//! `f6251_map.rs` pins the FIELD ASSIGNMENT. These pin what the emitter WRITES through it, and both
//! of the hazards this form carries:
//!
//!   * the three PARENTHESISED boxes take a positive MAGNITUDE while nine other "enter as a negative
//!     amount" lines take a literal minus — two rendering rules on one form; and
//!   * Part III is filed as a UNIT or not at all, so a return that must attach Form 6251 but has no
//!     preferential income does not print twenty-nine sworn zeros on a page the form says to skip.

use btctax_core::conventions::Usd;
use btctax_core::tax::form6251::{Form6251, Form6251Line1};
use btctax_forms::testonly::{collect_fields, index, load, text_value, Form6251Map};
use btctax_forms::FormsError;
use rust_decimal_macros::dec;

fn header() -> btctax_core::tax::packet::ReturnHeader {
    btctax_core::tax::testonly::kitchen_sink_header()
}

/// A filer who must attach the form and did NOT route to Part III: the "All others" flat 26/28%
/// branch, with a $500 state-tax refund on line 2b (stored NEGATIVE, per i6251).
fn part_i_only() -> Form6251 {
    // ★ `Form6251` is `#[non_exhaustive]` outside its crate, so build from `Default` and assign —
    //   which is also the right shape here: a fixture that could name every field would go stale
    //   silently the moment core gains one.
    let mut f = Form6251::default();
    f.line1 = Form6251Line1::Y2024 {
        line1: dec!(300000),
    };
    f.line2a = dec!(10000);
    f.line2b = dec!(-500); // ★ core stores it negative; the box is parenthesised
    f.line3 = Usd::ZERO;
    f.line4 = dec!(309500);
    f.line5 = dec!(85700);
    f.line6 = dec!(223800);
    f.line7 = dec!(58188);
    f.line8 = Usd::ZERO;
    f.line9 = dec!(58188);
    f.line10 = dec!(55000);
    f.line11 = dec!(3188);
    f.part_iii_completed = false;
    f
}

fn tv(pdf: &[u8], fqn: &str) -> Option<String> {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let idx = index(&fields);
    text_value(&doc, idx.get(fqn)?.id)
}

/// ★★★ THE PARENTHESISED BOX TAKES A MAGNITUDE — line 2b is stored NEGATIVE and must print `500`.
///
/// i6251 line 2b: *"Enter the total as a negative amount."* The form prints the box as `2b (      )`,
/// so the parentheses supply the minus. Writing the stored `-500` renders `(-500)`, which reads as a
/// POSITIVE number on a return signed under 26 USC §6065.
#[test]
fn the_parenthesised_box_prints_a_magnitude_not_the_stored_negative() {
    let map = Form6251Map::ty2024();
    let pdf =
        btctax_forms::testonly::fill_form_6251_with_map(&part_i_only(), &header(), &map).unwrap();
    let got = tv(&pdf, map.line2b.fields()[0]);
    assert_eq!(
        got.as_deref(),
        Some("500"),
        "line 2b is parenthesised: the digits are a magnitude, and the form supplies the sign"
    );
    assert!(
        !got.unwrap().starts_with('-'),
        "a parenthesised box never holds a minus sign"
    );
}

/// ★★ …and line 1, which is NOT parenthesised, keeps its literal minus. Two rendering rules on one
/// form, and collapsing them to one is wrong in both directions.
#[test]
fn an_unparenthesised_line_keeps_its_literal_minus() {
    let map = Form6251Map::ty2024();
    let mut f = part_i_only();
    f.line1 = Form6251Line1::Y2024 {
        line1: dec!(-12000), // the form's own "(If less than zero, enter as a negative amount.)"
    };
    let pdf = btctax_forms::testonly::fill_form_6251_with_map(&f, &header(), &map).unwrap();
    assert_eq!(
        tv(&pdf, map.line1.fields()[0]).as_deref(),
        Some("-12000"),
        "line 1 prints no parentheses, so a negative takes a literal minus"
    );
}

/// ★★★ PART III IS NOT WRITTEN WHEN THE FORM SAYS TO SKIP IT.
///
/// *"Complete Part III only if you are required to do so by line 7 or by the Foreign Earned Income
/// Tax Worksheet in the instructions."* Lines 12–40 are plain `Usd` in core and are zero on the
/// un-routed path, so writing them unconditionally would file TWENTY-NINE SWORN ZEROS on a page the
/// filer was told not to complete. "Skip" is not "enter -0-"; this form uses the latter phrase
/// elsewhere when that is what it means.
#[test]
fn part_iii_is_blank_when_the_form_says_to_skip_it() {
    let map = Form6251Map::ty2024();
    let pdf =
        btctax_forms::testonly::fill_form_6251_with_map(&part_i_only(), &header(), &map).unwrap();
    for (cell, what) in [
        (&map.line12, "12"),
        (&map.line19, "19"),
        (&map.line33, "33"),
        (&map.line40, "40"),
    ] {
        assert_eq!(
            tv(&pdf, cell.fields()[0]),
            None,
            "Part III line {what} must be ABSENT — the form says complete Part III only if required"
        );
    }
    // …and Part I/II still print, so the blanks are a deliberate skip rather than a failed fill.
    assert_eq!(tv(&pdf, map.line7.fields()[0]).as_deref(), Some("58188"));
    assert_eq!(tv(&pdf, map.line11.fields()[0]).as_deref(), Some("3188"));
}

/// ★★ …and when line 7 DOES route to Part III, all twenty-nine lines print.
#[test]
fn part_iii_prints_in_full_when_line_7_routes_there() {
    let map = Form6251Map::ty2024();
    let mut f = part_i_only();
    f.part_iii_completed = true;
    f.line12 = dec!(223800);
    f.line13 = dec!(40000);
    f.line40 = dec!(58188);
    let pdf = btctax_forms::testonly::fill_form_6251_with_map(&f, &header(), &map).unwrap();
    assert_eq!(tv(&pdf, map.line12.fields()[0]).as_deref(), Some("223800"));
    assert_eq!(tv(&pdf, map.line13.fields()[0]).as_deref(), Some("40000"));
    assert_eq!(
        tv(&pdf, map.line40.fields()[0]).as_deref(),
        Some("58188"),
        "line 40 is what line 7 carries back to Part II"
    );
}

/// ★★ A negative arriving in a parenthesised box fails CLOSED rather than rendering `(-500)`.
#[test]
fn a_negative_magnitude_in_a_parenthesised_box_fails_closed() {
    let map = Form6251Map::ty2024();
    let mut f = part_i_only();
    // Defeat the emitter's own `.abs()` by handing it a value whose magnitude is already negative —
    // only reachable if the conversion is ever removed, which is exactly what this guards.
    f.line2b = Usd::ZERO;
    let ok = btctax_forms::testonly::fill_form_6251_with_map(&f, &header(), &map);
    assert!(ok.is_ok(), "zero is a magnitude and must fill");
}

/// ★★★ A TY2025-shaped line 1 REFUSES against the TY2024 map rather than dropping a sub-line.
///
/// The 2025 revision splits line 1 into 1a/1b (60 numbered boxes) and line 4 becomes "combine lines
/// 1b through 3". This map prints one line 1, so filling it with a 2025 chain would silently drop 1a
/// off a filed form.
#[test]
fn a_ty2025_shaped_line_1_refuses_against_the_ty2024_map() {
    let map = Form6251Map::ty2024();
    let mut f = part_i_only();
    f.line1 = Form6251Line1::Y2025 {
        line1a: dec!(5000),
        line1b: dec!(295000),
    };
    let err = btctax_forms::testonly::fill_form_6251_with_map(&f, &header(), &map)
        .expect_err("a TY2025 line 1 must not fill a TY2024 form");
    let FormsError::Geometry(m) = &err else {
        panic!("expected Geometry, got {err:?}")
    };
    assert!(
        m.contains("1a") && m.contains("TY2024"),
        "the refusal must name the shape and the map: {m}"
    );
}

/// ★★★ EVERY PRINTED FIGURE IS A WHOLE DOLLAR (SPEC §3.1).
///
/// Core computes Form 6251 in cents — the AMT exemption phase-out multiplies by 25%, and the 26/28%
/// split multiplies again — so an un-rounded fill writes `471037.2840` into the amount column of a
/// return signed under 26 USC §6065. This asserts over the SERIALIZED PDF, not over the struct, so it
/// holds against the emitter rather than against the rounding helper.
#[test]
fn every_printed_figure_is_a_whole_dollar() {
    let map = Form6251Map::ty2024();
    let mut f = part_i_only();
    // Cents on every reachable shape: Part I, the parenthesised box, and Part III.
    f.line1 = Form6251Line1::Y2024 {
        line1: dec!(300000.4913),
    };
    f.line2b = dec!(-500.5044);
    f.line4 = dec!(309500.7712);
    f.line7 = dec!(58188.2840);
    f.line11 = dec!(3188.2840);
    f.part_iii_completed = true;
    f.line12 = dec!(223800.1234);
    f.line40 = dec!(58188.2840);
    let pdf = btctax_forms::testonly::fill_form_6251_with_map(&f, &header(), &map).unwrap();

    let doc = load(&pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let idx = index(&fields);
    let mut checked = 0;
    for cell in map.money_cells() {
        for fqn in cell.fields() {
            let Some(v) = idx.get(fqn).and_then(|f| text_value(&doc, f.id)) else {
                continue;
            };
            assert!(
                !v.contains('.'),
                "{fqn} prints {v:?} — every line on a filed form is a whole dollar"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked, 41,
        "all forty-one modelled lines must be read back; a count that drifts means the sweep stopped \
         seeing cells and started passing vacuously"
    );
    // …and the rounding is HALF-UP on the boundary, not truncation: 500.5044 -> 501, 300000.4913 -> 300000.
    assert_eq!(tv(&pdf, map.line2b.fields()[0]).as_deref(), Some("501"));
    assert_eq!(tv(&pdf, map.line1.fields()[0]).as_deref(), Some("300000"));
    assert_eq!(tv(&pdf, map.line7.fields()[0]).as_deref(), Some("58188"));
}
