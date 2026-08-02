//! §G-28/B1a — filling Form 8995-A Part IV, read back off the serialized PDF.
//!
//! The map's own tests pin the FIELD ASSIGNMENT (`f8995a_map.rs`). These pin what the emitter WRITES
//! through it: the right figure in the right box, the parenthesized boxes as magnitudes, and the DPAD
//! line **blank** rather than zeroed.

use btctax_core::conventions::Usd;
use btctax_core::tax::qbi_a::Form8995APartIv;
use btctax_forms::testonly::{collect_fields, index, load, text_value, Form8995AMap};
use btctax_forms::FormsError;
use rust_decimal_macros::dec;

fn header() -> btctax_core::tax::packet::ReturnHeader {
    btctax_core::tax::testonly::kitchen_sink_header()
}

/// A REIT-only filer above the §199A threshold: $4,000 of REIT dividends, taxable income $250,000,
/// no net capital gain. Deduction = 20% × 4,000 = $800, well under the 20%-of-TI limitation.
fn reit_only() -> Form8995APartIv {
    Form8995APartIv {
        line27: Usd::ZERO,
        line28: dec!(4000),
        line29: Usd::ZERO,
        line30: dec!(4000),
        line31: dec!(800),
        line32: dec!(800),
        line33: dec!(250000),
        line34: Usd::ZERO,
        line35: dec!(250000),
        line36: dec!(50000),
        line37: dec!(800),
        line38: None,
        line39: dec!(800),
        line40: Usd::ZERO,
    }
}

/// Read one field's value back off a serialized PDF, by fully-qualified name.
fn tv(pdf: &[u8], fqn: &str) -> Option<String> {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let idx = index(&fields);
    text_value(&doc, idx.get(fqn)?.id)
}

/// ★★★ Every Part IV figure lands in its own box, read back off the SERIALIZED output.
#[test]
fn part_iv_writes_each_figure_to_its_own_line() {
    let map = Form8995AMap::ty2024();
    let pdf =
        btctax_forms::testonly::fill_form_8995a_with_map(&reit_only(), &header(), &map).unwrap();

    for (fqn, want) in [
        (&map.line28, "4000"),
        (&map.line30, "4000"),
        (&map.line31, "800"),
        (&map.line32, "800"),
        (&map.line33, "250000"),
        (&map.line36, "50000"),
        (&map.line37, "800"),
        (&map.line39, "800"),
    ] {
        let got = tv(&pdf, fqn.fields()[0]);
        assert_eq!(
            got.as_deref(),
            Some(want),
            "the box for {} should read {want}",
            fqn.fields()[0]
        );
    }
}

/// ★★★ LINE 38 (DPAD) IS BLANK, NOT ZERO — the whole reason `push_money_opt` exists.
///
/// Its text is *"DPAD under section 199A(g) allocated from an agricultural or horticultural
/// cooperative. Don't enter more than line 33 minus line 37"* — a conditional entry with no `-0-`
/// clause. btctax fills no Schedule D (Form 8995-A), so no cooperative allocated anything, and a
/// printed `0` would swear the filer received an allocation of zero.
#[test]
fn the_dpad_line_carries_no_testimony() {
    let map = Form8995AMap::ty2024();
    let pdf =
        btctax_forms::testonly::fill_form_8995a_with_map(&reit_only(), &header(), &map).unwrap();
    assert_eq!(
        tv(&pdf, map.line38.fields()[0]),
        None,
        "line 38 must be ABSENT from the filled form, not written as 0"
    );
    // …and line 39 still prints, so the blank is a deliberate omission rather than a truncated fill.
    assert_eq!(tv(&pdf, map.line39.fields()[0]).as_deref(), Some("800"));
}

/// ★★ A negative in a PARENTHESIZED box fails closed. The form prints the minus sign, so `-1234` would
/// render as `(-1,234)` — a positive number on a filed return.
#[test]
fn a_negative_in_a_parenthesised_box_fails_closed() {
    let map = Form8995AMap::ty2024();
    for (label, mut p) in [("29", reit_only()), ("40", reit_only())] {
        if label == "29" {
            p.line29 = dec!(-500);
        } else {
            p.line40 = dec!(-500);
        }
        let err = btctax_forms::testonly::fill_form_8995a_with_map(&p, &header(), &map)
            .expect_err("a negative magnitude must refuse");
        let FormsError::Geometry(m) = &err else {
            panic!("expected Geometry, got {err:?}")
        };
        assert!(
            m.contains(&format!("line {label}")) && m.contains("magnitude"),
            "the refusal must name the line and the convention: {m}"
        );
    }
}

/// ★ A REIT/PTP loss carryforward prints as a POSITIVE MAGNITUDE in its parenthesized box.
#[test]
fn a_loss_carryforward_prints_as_a_magnitude() {
    let map = Form8995AMap::ty2024();
    let mut p = reit_only();
    p.line28 = Usd::ZERO;
    p.line29 = dec!(5000); // prior-year loss, magnitude
    p.line30 = Usd::ZERO; // combine ⇒ -5,000 ⇒ clamped
    p.line31 = Usd::ZERO;
    p.line32 = Usd::ZERO;
    p.line37 = Usd::ZERO;
    p.line39 = Usd::ZERO;
    p.line40 = dec!(5000); // carries forward, magnitude
    let pdf = btctax_forms::testonly::fill_form_8995a_with_map(&p, &header(), &map).unwrap();
    assert_eq!(tv(&pdf, map.line29.fields()[0]).as_deref(), Some("5000"));
    assert_eq!(tv(&pdf, map.line40.fields()[0]).as_deref(), Some("5000"));
    for s in [
        tv(&pdf, map.line29.fields()[0]),
        tv(&pdf, map.line40.fields()[0]),
    ] {
        assert!(
            !s.unwrap().starts_with('-'),
            "a parenthesized box never holds a minus sign — the form supplies it"
        );
    }
}
