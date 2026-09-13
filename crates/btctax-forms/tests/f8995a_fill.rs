//! §G-28/B1a — filling Form 8995-A Part IV, read back off the serialized PDF.
//!
//! The map's own tests pin the FIELD ASSIGNMENT (`f8995a_map.rs`). These pin what the emitter WRITES
//! through it: the right figure in the right box, the parenthesized boxes as magnitudes, and the DPAD
//! line **blank** rather than zeroed.
//!
//! ── FR-142 (rehearsal F17) ──────────────────────────────────────────────────────────────────────
//! Every test here used to build `Form8995AMap::ty2024()` once. `packet.rs` dispatches on
//! `Form8995AMap::for_year(year)`, which is year-generic, so a newly ported year was WIRED (it
//! parses, it dispatches) without ever being FILLED by this suite — "right number in the right box"
//! stayed unproven for it. [`covered_years`] derives the set this file actually exercises from the
//! bundled glob, so a port inherits this read-back for free instead of silently sitting outside it.

use btctax_core::conventions::Usd;
use btctax_core::tax::qbi_a::{
    Form8995APartIToIii, Form8995APartIi, Form8995APartIii, Form8995APartIv, Form8995ARowA,
};
use btctax_forms::bundled::{Stem, BUNDLED};
use btctax_forms::testonly::{checkbox_on, collect_fields, index, load, text_value, Form8995AMap};
use btctax_forms::FormsError;
use rust_decimal_macros::dec;

fn header() -> btctax_core::tax::packet::ReturnHeader {
    btctax_core::tax::testonly::kitchen_sink_header()
}

/// Every tax year this build can both FILL and READ BACK as a [`Form8995AMap`] — DERIVED from the
/// bundled glob (`btctax_forms::bundled::BUNDLED`), filtered through the same schema dispatch
/// `packet.rs` uses (`Form8995AMap::for_year`), never a hand-typed list. A year whose `line_set`
/// fails to parse, or parses into a DIFFERENT schema (a renumbered form, `Schema::Unwired`), is
/// excluded rather than assumed — this is the measurement FR-142 asked for: "which years actually
/// share Form 8995-A's schema" is answered by which years survive this filter, not by a claim.
///
/// PANICS if the result is empty. A `for year in covered_years()` loop over zero years would make
/// every test below vacuously pass — silently covering nothing while reporting green — which is
/// worse than the gap FR-142 exists to close. See `the_covered_year_set_can_never_be_silently_empty`
/// for the planted-defect kill.
fn covered_years() -> Vec<i32> {
    let mut years: Vec<i32> = BUNDLED
        .iter()
        .filter(|(stem, _)| *stem == Stem::F8995a)
        .map(|(_, year)| *year)
        .filter(|&year| Form8995AMap::for_year(year).is_ok())
        .collect();
    years.sort_unstable();
    years.dedup();
    assert!(
        !years.is_empty(),
        "f8995a_fill::covered_years() is EMPTY — no bundled year parses as Form8995AMap. A loop over \
         nothing must never pass silently: if this reds, either the glob lost f8995a or its schema \
         moved on and this file needs a new type to test against, not a wider filter."
    );
    years
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

/// ★ FR-142's own instrument stays honest: an empty covered-year set must fail this assertion rather
/// than let every `for year in covered_years()` loop above skip silently. This does not call the
/// production `covered_years()` (that one is correctly non-empty today) — it re-states the same
/// guard directly so the guard itself is proven to fire on an empty input, independent of how many
/// years happen to be bundled when this runs.
#[test]
fn the_covered_year_set_can_never_be_silently_empty() {
    let empty: Vec<i32> = Vec::new();
    let result = std::panic::catch_unwind(|| {
        assert!(
            !empty.is_empty(),
            "f8995a_fill::covered_years() is EMPTY — no bundled year parses as Form8995AMap. A loop \
             over nothing must never pass silently: if this reds, either the glob lost f8995a or its \
             schema moved on and this file needs a new type to test against, not a wider filter."
        );
    });
    assert!(
        result.is_err(),
        "an empty covered-year set must panic the guard, not pass through it"
    );
}

/// ★★★ Every Part IV figure lands in its own box, read back off the SERIALIZED output — for every
/// year this build can fill as [`Form8995AMap`] (see [`covered_years`]).
#[test]
fn part_iv_writes_each_figure_to_its_own_line() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let pdf =
            btctax_forms::testonly::fill_form_8995a_with_map(&reit_only(), None, &header(), &map)
                .unwrap();

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
                "TY{year}: the box for {} should read {want}",
                fqn.fields()[0]
            );
        }
    }
}

/// ★★★ LINE 38 (DPAD) IS BLANK, NOT ZERO — the whole reason `push_money_opt` exists.
///
/// Its text is *"DPAD under section 199A(g) allocated from an agricultural or horticultural
/// cooperative. Don't enter more than line 33 minus line 37"* — a conditional entry with no `-0-`
/// clause. btctax fills no Schedule D (Form 8995-A), so no cooperative allocated anything, and a
/// printed `0` would swear the filer received an allocation of zero. Checked over every year in
/// [`covered_years`].
#[test]
fn the_dpad_line_carries_no_testimony() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let pdf =
            btctax_forms::testonly::fill_form_8995a_with_map(&reit_only(), None, &header(), &map)
                .unwrap();
        assert_eq!(
            tv(&pdf, map.line38.fields()[0]),
            None,
            "TY{year}: line 38 must be ABSENT from the filled form, not written as 0"
        );
        // …and line 39 still prints, so the blank is a deliberate omission rather than a truncated fill.
        assert_eq!(
            tv(&pdf, map.line39.fields()[0]).as_deref(),
            Some("800"),
            "TY{year}: line 39 must still print"
        );
    }
}

/// ★★ A negative in a PARENTHESIZED box fails closed. The form prints the minus sign, so `-1234` would
/// render as `(-1,234)` — a positive number on a filed return. Checked over every year in
/// [`covered_years`].
#[test]
fn a_negative_in_a_parenthesised_box_fails_closed() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        for (label, mut p) in [("29", reit_only()), ("40", reit_only())] {
            if label == "29" {
                p.line29 = dec!(-500);
            } else {
                p.line40 = dec!(-500);
            }
            let err = btctax_forms::testonly::fill_form_8995a_with_map(&p, None, &header(), &map)
                .expect_err("a negative magnitude must refuse");
            let FormsError::Geometry(m) = &err else {
                panic!("TY{year}: expected Geometry, got {err:?}")
            };
            assert!(
                m.contains(&format!("line {label}")) && m.contains("magnitude"),
                "TY{year}: the refusal must name the line and the convention: {m}"
            );
        }
    }
}

/// ★ A REIT/PTP loss carryforward prints as a POSITIVE MAGNITUDE in its parenthesized box. Checked
/// over every year in [`covered_years`].
#[test]
fn a_loss_carryforward_prints_as_a_magnitude() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let mut p = reit_only();
        p.line28 = Usd::ZERO;
        p.line29 = dec!(5000); // prior-year loss, magnitude
        p.line30 = Usd::ZERO; // combine ⇒ -5,000 ⇒ clamped
        p.line31 = Usd::ZERO;
        p.line32 = Usd::ZERO;
        p.line37 = Usd::ZERO;
        p.line39 = Usd::ZERO;
        p.line40 = dec!(5000); // carries forward, magnitude
        let pdf =
            btctax_forms::testonly::fill_form_8995a_with_map(&p, None, &header(), &map).unwrap();
        assert_eq!(
            tv(&pdf, map.line29.fields()[0]).as_deref(),
            Some("5000"),
            "TY{year}: line 29"
        );
        assert_eq!(
            tv(&pdf, map.line40.fields()[0]).as_deref(),
            Some("5000"),
            "TY{year}: line 40"
        );
        for s in [
            tv(&pdf, map.line29.fields()[0]),
            tv(&pdf, map.line40.fields()[0]),
        ] {
            assert!(
                !s.unwrap().starts_with('-'),
                "TY{year}: a parenthesized box never holds a minus sign — the form supplies it"
            );
        }
    }
}

// ── §G-28/B1b — Parts I–III ─────────────────────────────────────────────────────────────────────

/// A phased-in filer: QBI $100,581, no W-2 wages, TI-before-QBI $205,981 (inside the single range).
/// The figures are the `range/no-wages` vector `qbi_a` witnesses against Tax-Calculator.
fn phased_in() -> Form8995APartIToIii {
    Form8995APartIToIii {
        part_i: Form8995ARowA {
            col_a_name: "Bitcoin mining".into(),
            col_b_specified_service: false,
            col_c_aggregation: false,
            col_e_patron: false,
        },
        part_ii: Form8995APartIi {
            line2: dec!(100581),
            line3: dec!(20116),
            line4: Usd::ZERO,
            line5: Usd::ZERO,
            line6: Usd::ZERO,
            line7: Usd::ZERO,
            line8: Usd::ZERO,
            line9: Usd::ZERO,
            line10: Usd::ZERO,
            line11: Usd::ZERO,
            line12: Some(dec!(14471)),
            line13: dec!(14471),
            line14: None,
            line15: dec!(14471),
            line16: dec!(14471),
        },
        part_iii: Some(Form8995APartIii {
            line17: dec!(20116),
            line18: Usd::ZERO,
            line19: dec!(20116),
            line20: dec!(205981),
            line21: dec!(191950),
            line22: dec!(14031),
            line23: dec!(50000),
            line24_ratio: dec!(0.28062),
            line25: dec!(5645),
            line26: dec!(14471),
        }),
    }
}

/// ★★★ Every Part I, II and III figure lands in its own box, read back off the SERIALIZED output.
/// Checked over every year in [`covered_years`].
#[test]
fn parts_i_to_iii_write_each_figure_to_its_own_line() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let f = phased_in();
        let pdf = btctax_forms::testonly::fill_form_8995a_with_map(
            &reit_only(),
            Some(&f),
            &header(),
            &map,
        )
        .unwrap();

        // Part I row A — the business names itself, and the TIN is the PROPRIETOR's.
        assert_eq!(
            tv(&pdf, &map.part1_row_a.name).as_deref(),
            Some("Bitcoin mining"),
            "TY{year}: column (a) must name the trade or business"
        );
        assert!(
            tv(&pdf, &map.part1_row_a.tin).is_some(),
            "TY{year}: column (d) must carry the proprietor's TIN"
        );

        for (fqn, want) in [
            (&map.part2_col_a.line2, "100581"),
            (&map.part2_col_a.line3, "20116"),
            (&map.part2_col_a.line12, "14471"),
            (&map.part2_col_a.line13, "14471"),
            (&map.part2_col_a.line15, "14471"),
            (&map.part2_col_a.line16, "14471"),
            (&map.part3_col_a.line17, "20116"),
            (&map.part3_col_a.line19, "20116"),
            (&map.part3_col_a.line20, "205981"),
            (&map.part3_col_a.line21, "191950"),
            (&map.part3_col_a.line22, "14031"),
            (&map.part3_col_a.line23, "50000"),
            (&map.part3_col_a.line25, "5645"),
            (&map.part3_col_a.line26, "14471"),
        ] {
            assert_eq!(
                tv(&pdf, fqn.fields()[0]).as_deref(),
                Some(want),
                "TY{year}: the box for {} should read {want}",
                fqn.fields()[0]
            );
        }
    }
}

/// ★★★ LINE 24 PRINTS AS A PERCENTAGE — the ratio × 100, because the form suffixes the box with `%`.
///
/// The core struct holds the RATIO (0.28062), because that is what line 25 multiplies by. Printing
/// the ratio itself would put `0` in a box the form reads as a percentage — a 0% phase-in on a return
/// whose line 25 was figured at 28%, and the two would not reconcile on the page. Checked over every
/// year in [`covered_years`].
#[test]
fn the_phase_in_percentage_prints_scaled_by_a_hundred() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let pdf = btctax_forms::testonly::fill_form_8995a_with_map(
            &reit_only(),
            Some(&phased_in()),
            &header(),
            &map,
        )
        .unwrap();
        let printed = tv(&pdf, map.part3_col_a.line24.fields()[0])
            .unwrap_or_else(|| panic!("TY{year}: line 24 must print"));
        assert_eq!(
            printed, "28.062",
            "TY{year}: line 24 holds the ratio 0.28062 and must print as 28.062 percent"
        );
        // ★★ The exact assertion above already pins the normalization: 14031/50000 carries Decimal scale
        //    5, so the un-normalized product renders "28.06200" — arithmetically identical, wrong ink on a
        //    filed form. ★ A blanket "must not end in 0" would be FALSE for line 24 in general: a ratio of
        //    25000/50000 prints "50" and 50000/50000 prints "100", both correct.
    }
}

/// ★★ THE FAIL-CLOSED GUARD ON A NAMELESS BUSINESS — carried across from the Form 8995 emitter,
/// which has held the identical guard for the identical decision since P7.
///
/// Part II line 2 is this business's QBI; Part I column (a) is where it is named. A non-zero line 2
/// over a blank row A claims a §199A deduction for a business the return never names. Core refuses
/// it first, so this exists to keep that unreachable. Checked over every year in [`covered_years`].
#[test]
fn a_nameless_business_with_qbi_fails_closed() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let mut f = phased_in();
        f.part_i.col_a_name = "   ".into();
        let err = btctax_forms::testonly::fill_form_8995a_with_map(
            &reit_only(),
            Some(&f),
            &header(),
            &map,
        )
        .expect_err("a nameless business with QBI must refuse");
        let FormsError::Geometry(m) = &err else {
            panic!("TY{year}: expected Geometry, got {err:?}")
        };
        assert!(
            m.contains("line 2") && m.contains("never names"),
            "TY{year}: the refusal must name the line and the reason: {m}"
        );
        // …and a NAMED business with the same figures still fills, so the guard is not always-on.
        btctax_forms::testonly::fill_form_8995a_with_map(
            &reit_only(),
            Some(&phased_in()),
            &header(),
            &map,
        )
        .unwrap_or_else(|e| panic!("TY{year}: a named business must fill: {e:?}"));
    }
}

/// ★★★ LINE 14 IS BLANK ON EVERY FILED RETURN, and line 12 is blank OUTSIDE the phase-in range.
///
/// Both are *"if any"* conditional entries with no `-0-` clause. btctax fills no Schedule D (Form
/// 8995-A) because a patron refuses upstream, so there is no line 6 amount to enter; and outside the
/// range there is no line 26. A printed `0` on either would swear a figure was computed and came to
/// nothing — testimony the filer never gave. Checked over every year in [`covered_years`].
#[test]
fn the_conditional_part_ii_lines_carry_no_testimony() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        // Above the range: Part III skipped, so line 12 has no source.
        let mut f = phased_in();
        f.part_iii = None;
        f.part_ii.line12 = None;
        f.part_ii.line13 = f.part_ii.line11;
        f.part_ii.line15 = f.part_ii.line11;
        f.part_ii.line16 = f.part_ii.line11;
        let pdf = btctax_forms::testonly::fill_form_8995a_with_map(
            &reit_only(),
            Some(&f),
            &header(),
            &map,
        )
        .unwrap();
        assert_eq!(
            tv(&pdf, map.part2_col_a.line12.fields()[0]),
            None,
            "TY{year}: line 12 must be ABSENT when Part III was skipped, not written as 0"
        );
        assert_eq!(
            tv(&pdf, map.part2_col_a.line14.fields()[0]),
            None,
            "TY{year}: line 14 must be ABSENT — btctax fills no Schedule D (Form 8995-A)"
        );
        // …and Part III's own boxes are untouched, so the skip is a skip and not a blank fill.
        for cell in [&map.part3_col_a.line17, &map.part3_col_a.line24] {
            assert_eq!(
                tv(&pdf, cell.fields()[0]),
                None,
                "TY{year}: a skipped Part III writes nothing at all"
            );
        }
        // …while line 2 still prints, so the blanks are deliberate rather than a truncated fill.
        assert_eq!(
            tv(&pdf, map.part2_col_a.line2.fields()[0]).as_deref(),
            Some("100581"),
            "TY{year}: line 2 must still print"
        );
    }
}

/// ★★★ THE THREE PART I CHECKBOXES ARE WRITTEN ONLY WHEN TRUE.
///
/// An unchecked box is not written at all. Writing an "off" value would put a mark on the page the
/// filer did not make — and on a form where checking column (b) is the affirmative act, an explicit
/// off-state is not the same silence the form expects. Checked over every year in [`covered_years`].
#[test]
fn a_part_i_checkbox_is_written_only_when_it_is_checked() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let mut f = phased_in();
        f.part_i.col_b_specified_service = true;
        let pdf = btctax_forms::testonly::fill_form_8995a_with_map(
            &reit_only(),
            Some(&f),
            &header(),
            &map,
        )
        .unwrap();
        let doc = load(&pdf).unwrap();
        let fields = collect_fields(&doc).unwrap();
        let idx = index(&fields);
        let is_set = |fqn: &str| idx.get(fqn).and_then(|f| checkbox_on(&doc, f.id)).is_some();
        assert!(
            is_set(&map.part1_row_a.specified_service.field),
            "TY{year}: column (b) was answered YES and must be checked"
        );
        assert!(
            !is_set(&map.part1_row_a.aggregation.field),
            "TY{year}: column (c) is false — an unchecked box is not written"
        );
        assert!(
            !is_set(&map.part1_row_a.patron.field),
            "TY{year}: column (e) is false on every filed return — a patron refuses upstream"
        );
    }
}

/// ★★ A REIT/PTP-only filer files PART IV ALONE — i8995a's *"If you don't have QBI, and only have
/// REIT, PTP, skip Parts I through III and complete Part IV."* Passing `None` must leave Part I's
/// name box empty, not write an empty string into it. Checked over every year in [`covered_years`].
#[test]
fn a_reit_only_filer_writes_nothing_in_parts_i_to_iii() {
    for year in covered_years() {
        let map = Form8995AMap::for_year(year).unwrap();
        let pdf =
            btctax_forms::testonly::fill_form_8995a_with_map(&reit_only(), None, &header(), &map)
                .unwrap();
        for cell in [
            map.part1_row_a.name.as_str(),
            map.part1_row_a.tin.as_str(),
            map.part2_col_a.line2.fields()[0],
            map.part2_col_a.line16.fields()[0],
            map.part3_col_a.line20.fields()[0],
        ] {
            assert_eq!(
                tv(&pdf, cell),
                None,
                "TY{year}: {cell} must be untouched when Parts I-III are skipped"
            );
        }
        // Part IV still fills, so this is a skip and not a failed write.
        assert_eq!(
            tv(&pdf, map.line39.fields()[0]).as_deref(),
            Some("800"),
            "TY{year}: Part IV must still fill"
        );
    }
}
