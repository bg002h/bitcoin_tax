//! **Form 1040-V fill KATs** (spec `SPEC_form_4868_1040v.md`, task T4) — the payment voucher, read
//! back from the SERIALIZED PDF rather than from the struct that wrote it.
//!
//! The voucher's own trap, measured in T1 and worth restating where the filler lives: **four of Form
//! 4868's field names also exist in this form's AcroForm** (`f1_11`…`f1_12`…`f1_14`, the 4868's
//! money lines, are the voucher's city/state/ZIP row). So "every mapped field exists in the PDF" is
//! necessary and NOT sufficient for this pair — a 4868 map applied to the voucher would resolve
//! every name and print a balance due across the address block. Only the geometric read-back tells
//! them apart, and it is exercised below on a planted defect.
//!
//! Nothing here hardcodes a field name: every FQN comes from the committed map, and the
//! never-written set comes from the map's own `[census]` keys plus `address_apt`.

use btctax_core::tax::packet::{assemble_printed_return, PrintedReturn};
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;
use btctax_forms::fill_form_1040v;
use btctax_forms::testonly::{
    collect_fields, fill_form_1040v_with_map, load, text_value, Field, Form1040VMap,
};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

/// The kitchen-sink TY2024 household, printed. MFJ, with a spouse.
fn kitchen_sink() -> PrintedReturn {
    let (ri, state) = kitchen_sink_household();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &ty2024_params(), &table, 2024);
    let details: BTreeMap<_, _> = BTreeMap::new();
    assemble_printed_return(
        &ri,
        &state,
        &details,
        &ar,
        &table,
        2024,
        &[],
        btctax_core::InformationReturnRegime::NONE,
    )
    .expect("the kitchen sink assembles a printed TY2024 return")
}

fn readback(bytes: &[u8]) -> (lopdf::Document, Vec<Field>) {
    let doc = load(bytes).expect("the filled 1040-V re-parses");
    let fields = collect_fields(&doc).expect("…and its AcroForm is readable");
    (doc, fields)
}

/// The text a named field carries in the output, or `None` when it is BLANK.
fn text_of(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields
        .iter()
        .find(|f| f.fqn == fqn)
        .unwrap_or_else(|| panic!("{fqn} is not a field of the bundled Form 1040-V"));
    text_value(doc, f.id).filter(|s| !s.is_empty())
}

/// ★ Box 3 is the whole reason the page exists — *"Line 3. Enter the amount you are paying by check
/// or money"* *"order."* — and the identity boxes are what let the Service match the check to the
/// return: *"Line 4. Enter your name(s) and address exactly as shown on"* *"your return."*
#[test]
fn the_voucher_prints_the_amount_and_the_returns_own_identity() {
    let pr = kitchen_sink();
    let map = Form1040VMap::ty2024();
    let bytes = fill_form_1040v(&pr, 2024, dec!(1234)).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(
        text_of(&doc, &fields, &map.box3_amount).as_deref(),
        Some("1234"),
        "box 3 carries the amount being paid"
    );
    assert_eq!(
        text_of(&doc, &fields, &map.box1_ssn).as_deref(),
        Some("123-45-6789"),
        "box 1 is the SSN shown FIRST on the return, hyphenated (the cell's /MaxLen is 11)"
    );
    assert_eq!(
        text_of(&doc, &fields, &map.box4_first_name).as_deref(),
        Some(pr.header.taxpayer.first_name.as_str())
    );
    assert_eq!(
        text_of(&doc, &fields, &map.box4_last_name).as_deref(),
        Some(pr.header.taxpayer.last_name.as_str())
    );
    for (what, fqn) in [
        ("street", &map.address_street),
        ("city", &map.address_city),
        ("state", &map.address_state),
        ("ZIP", &map.address_zip),
    ] {
        assert!(
            text_of(&doc, &fields, fqn).is_some(),
            "the {what} cell must come back filled — the Service matches the check by address"
        );
    }
}

/// ★ KILL — box 2 and the spouse name row are JOINT-RETURN rows: *"If a joint return, SSN shown
/// second"* *"on your return."*
///
/// The discriminating pair, exactly as on the 4868: ONE household, ONE header carrying a spouse, TWO
/// filing statuses. `ReturnHeader.spouse` is `Some` on MFS too (spec I-1), so a filler gated on the
/// spouse's presence would print a joint return's SSN on a separate return's voucher.
#[test]
fn box_2_and_the_spouse_name_row_are_filled_on_mfj_and_blank_on_mfs() {
    let map = Form1040VMap::ty2024();

    let mut joint = kitchen_sink();
    joint.filing_status = FilingStatus::Mfj;
    assert!(
        joint.header.spouse.is_some(),
        "premise: the fixture header carries a spouse"
    );
    let (doc, fields) = readback(&fill_form_1040v(&joint, 2024, dec!(1234)).unwrap());
    assert_eq!(
        text_of(&doc, &fields, &map.box2_spouse_ssn).as_deref(),
        Some("987-65-4321")
    );
    assert!(text_of(&doc, &fields, &map.spouse_first_name).is_some());
    assert!(text_of(&doc, &fields, &map.spouse_last_name).is_some());

    let mut separate = joint.clone();
    separate.filing_status = FilingStatus::Mfs;
    assert!(
        separate.header.spouse.is_some(),
        "★ the header STILL carries a spouse — only the status changed"
    );
    let (doc, fields) = readback(&fill_form_1040v(&separate, 2024, dec!(1234)).unwrap());
    assert_eq!(
        text_of(&doc, &fields, &map.box2_spouse_ssn),
        None,
        "MFS ⇒ box 2 blank"
    );
    assert_eq!(text_of(&doc, &fields, &map.spouse_first_name), None);
    assert_eq!(text_of(&doc, &fields, &map.spouse_last_name), None);
    assert!(
        text_of(&doc, &fields, &map.box1_ssn).is_some(),
        "…while box 1 is still the filer's own SSN"
    );
}

/// ★ KILL — the cells this build never populates stay BLANK: the three foreign-address boxes (the
/// map's `[census]`) and `Apt. no.` (`ReturnHeader` has no apartment field). Both blanks are
/// CORRECT; what would be a defect is inventing an apartment number by splitting the street string,
/// or writing a foreign country the filer never gave.
///
/// The population is the map's own census keys plus `address_apt`, never a hand-list.
#[test]
fn the_foreign_boxes_and_the_apartment_cell_stay_blank_on_both_years() {
    for year in [2024, 2025] {
        let map = Form1040VMap::for_year(year).expect("both bundled years wire");
        let pr = kitchen_sink();
        let (doc, fields) = readback(&fill_form_1040v(&pr, year, dec!(1234)).unwrap());

        assert_eq!(
            map.census.len(),
            3,
            "premise: TY{year}'s census records the three foreign cells"
        );
        for fqn in map.census.keys() {
            assert_eq!(
                text_of(&doc, &fields, fqn),
                None,
                "TY{year}: {fqn} is censused as never-filled but came back FILLED"
            );
        }
        assert_eq!(
            text_of(&doc, &fields, &map.address_apt),
            None,
            "TY{year}: `Apt. no.` is a RECORDED blank — nothing in ReturnHeader can populate it"
        );
    }
}

/// ★ KILL — a voucher for $0, a negative amount, or an amount with cents is REFUSED with no bytes.
///
/// Each is a distinct wrong: $0 is not a payment (the export writes no voucher on a return that owes
/// nothing), a negative is not a payment either, and cents on a voucher whose return prints whole
/// dollars (§3.1) makes the two disagree about what is owed. The command refuses all three as well;
/// this is the second, structural gate.
#[test]
fn a_zero_negative_or_fractional_amount_is_refused() {
    let pr = kitchen_sink();

    let zero = fill_form_1040v(&pr, 2024, Usd::ZERO).expect_err("a $0 voucher must be refused");
    assert!(
        zero.to_string().contains("is not a payment"),
        "the refusal says why: {zero}"
    );

    let negative =
        fill_form_1040v(&pr, 2024, dec!(-5)).expect_err("a negative voucher must be refused");
    assert!(
        negative.to_string().contains("cannot be negative"),
        "{negative}"
    );

    let cents = fill_form_1040v(&pr, 2024, dec!(1234.56)).expect_err("cents must be refused");
    let msg = cents.to_string();
    assert!(
        msg.contains("1040-V") && msg.contains("line 3") && msg.contains("whole dollars"),
        "the refusal names the form, the box and the remedy: {msg}"
    );
}

/// Both revisions fill from their own map and template. The 15 field names and rects are identical
/// across TY2024 and TY2025 (T1 diffed them), so this is a wiring check rather than a geometry one —
/// and it is the check that reds if either year's `line_set` stops resolving to `Form1040VMap`.
#[test]
fn both_bundled_years_fill_the_voucher() {
    let pr = kitchen_sink();
    for year in [2024, 2025] {
        let map = Form1040VMap::for_year(year).expect("wired");
        let (doc, fields) = readback(&fill_form_1040v(&pr, year, dec!(4321)).unwrap());
        assert_eq!(
            text_of(&doc, &fields, &map.box3_amount).as_deref(),
            Some("4321")
        );
        assert!(text_of(&doc, &fields, &map.box1_ssn).is_some());
    }
}

/// ★★ KILL (harness B1) — the geometric read-back FAILS THE FILL CLOSED when box 3 is pointed at a
/// cell outside the amount column.
///
/// This is the guarantee that actually separates the two new maps. Planted on a COPY of the
/// committed TOML: box 3 is repointed at `f1_11`, the voucher's CITY cell — which is also the name
/// of Form 4868's line 4, the exact collision T1 measured. An existence check passes it; the
/// geometry oracle does not. The clean map is filled in the same test, so a checker that reds on
/// everything cannot pass either.
#[test]
fn box_3_pointed_at_the_city_cell_fails_the_fill_closed() {
    let pr = kitchen_sink();
    let src = btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F1040v, 2025)
        .expect("the TY2025 Form 1040-V map is bundled");

    let clean = Form1040VMap::parse(src).expect("the committed map parses");
    assert!(
        fill_form_1040v_with_map(&pr, dec!(1234), &clean).is_ok(),
        "premise: the COMMITTED map fills — otherwise this test reds on everything"
    );

    let doctored_src = src.replace(
        "box3_amount       = \"topmostSubform[0].Page1[0].f1_3[0]\"",
        "box3_amount       = \"topmostSubform[0].Page1[0].f1_11[0]\"",
    );
    assert!(
        doctored_src != src,
        "the plant must actually change the map"
    );
    let doctored = Form1040VMap::parse(&doctored_src).expect("the doctored map still parses");
    assert!(
        btctax_forms::testonly::collect_fields(
            &load(Form1040VMap::bundled_pdf(2025).unwrap()).unwrap()
        )
        .unwrap()
        .iter()
        .any(|f| f.fqn == doctored.box3_amount),
        "★ the planted target EXISTS in this PDF — that is the point: an existence check would pass it"
    );

    let err = fill_form_1040v_with_map(&pr, dec!(1234), &doctored)
        .expect_err("the amount landing in the city cell must fail closed");
    let msg = err.to_string();
    assert!(
        msg.contains("mis-mapped") || msg.contains("geometric"),
        "the geometric oracle must be what refuses it: {msg}"
    );
}
