//! **Form 4868 fill KATs** (spec `SPEC_form_4868_1040v.md`, task T2) — every guarantee the extension
//! application makes, read back from the SERIALIZED PDF rather than from the struct that wrote it.
//!
//! Two things this file is careful about, because they are where the class of defect this repo keeps
//! finding lives:
//!
//! 1. **A printed `0` and a blank are different speech acts.** Lines 4 and 6 carry the form's own
//!    `-0-` clause and must print a zero; line 5 has none and must be BLANK when it is zero. On the
//!    page they look nothing alike, but in a struct they are the same `Usd::ZERO`, so every one of
//!    these is asserted against the PDF's field value.
//! 2. **An unchecked box and a never-written box print identically.** Line 8 is therefore
//!    mutation-verified: the "absent ⇒ blank" half is worthless on its own, since a filler that
//!    never wrote the box at all would pass it.
//!
//! Nothing here hardcodes a field name: every FQN comes from the committed map, and the
//! never-written set comes from the map's own `[census]` keys rather than a hand-list that would rot.

use btctax_core::tax::packet::{assemble_printed_return, PrintedReturn};
use btctax_core::tax::printed::Schedule3Lines;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{kitchen_sink_household, ty2024_params, ty2024_table};
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;
use btctax_forms::testonly::{
    checkbox_on, collect_fields, fill_form_4868_with_map, load, text_value, Field, Form4868Map,
};
use btctax_forms::{fill_form_4868, Form4868Choices};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

/// The kitchen-sink TY2024 household, printed. MFJ, with a spouse, and it records a $500 extension
/// payment — so it exercises the Schedule 3 line 10 default without any doctoring.
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

/// The same household with the three Part II inputs set outright: 1040 line 24, 1040 line 33, and
/// the printed Schedule 3 (`None` = the common case, no extension payment and no other credit).
fn shaped(line24: Usd, line33: Usd, sch_3: Option<Schedule3Lines>) -> PrintedReturn {
    let mut pr = kitchen_sink();
    pr.forms.f1040.line24 = line24;
    pr.forms.f1040.line33 = line33;
    pr.forms.sch_3 = sch_3;
    pr
}

/// A printed Schedule 3 carrying `line10` (extension payment) and `line11` (excess Social Security).
fn sch3(line10: Usd, line11: Usd) -> Schedule3Lines {
    Schedule3Lines {
        line1: Usd::ZERO,
        line8: Usd::ZERO,
        line10,
        line11,
        line15: line10 + line11,
    }
}

/// Read every field of a filled 4868 back out of its own serialized bytes.
fn readback(bytes: &[u8]) -> (lopdf::Document, Vec<Field>) {
    let doc = load(bytes).expect("the filled 4868 re-parses");
    let fields = collect_fields(&doc).expect("…and its AcroForm is readable");
    (doc, fields)
}

/// The text a named field carries in the output, or `None` when it is BLANK. An empty string reads
/// as blank too: a `""` write and an untouched cell are the same mark on the page.
fn text_of(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields
        .iter()
        .find(|f| f.fqn == fqn)
        .unwrap_or_else(|| panic!("{fqn} is not a field of the bundled Form 4868"));
    text_value(doc, f.id).filter(|s| !s.is_empty())
}

/// The on-state a named checkbox carries, or `None` when it is not checked.
fn check_of(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields
        .iter()
        .find(|f| f.fqn == fqn)
        .unwrap_or_else(|| panic!("{fqn} is not a field of the bundled Form 4868"));
    checkbox_on(doc, f.id)
}

// ── The Part II line chain ───────────────────────────────────────────────────────────────────────

/// ★ KILL — *"Balance due. Subtract line 5 from line 4."* … *"If line 5 is more than line 4, enter
/// -0-."* A return whose payments exceed its tax owes nothing, and the form says to say so with a
/// printed zero — NOT a blank, and certainly not a negative balance due.
#[test]
fn payments_above_the_tax_print_an_explicit_zero_on_line_6() {
    let pr = shaped(dec!(100), dec!(900), None);
    let map = Form4868Map::ty2024();
    let bytes = fill_form_4868(&pr, 2024, Form4868Choices::default()).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(text_of(&doc, &fields, &map.line4).as_deref(), Some("100"));
    assert_eq!(text_of(&doc, &fields, &map.line5).as_deref(), Some("900"));
    assert_eq!(
        text_of(&doc, &fields, &map.line6).as_deref(),
        Some("0"),
        "line 5 > line 4 ⇒ the form's own -0- clause, printed"
    );
}

/// ★ KILL — *"If you expect this amount to be zero, enter -0-."* Line 4 is never blank. A blank
/// line 4 would leave the Service with no estimate at all, and the instructions warn that an
/// unreasonable estimate makes the extension *"null and void"* — an absent one is worse.
#[test]
fn a_zero_line_4_prints_a_zero_and_a_zero_line_5_prints_a_blank() {
    let pr = shaped(Usd::ZERO, Usd::ZERO, None);
    let map = Form4868Map::ty2024();
    let bytes = fill_form_4868(&pr, 2024, Form4868Choices::default()).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(
        text_of(&doc, &fields, &map.line4).as_deref(),
        Some("0"),
        "line 4 carries a -0- clause ⇒ a printed zero"
    );
    assert_eq!(
        text_of(&doc, &fields, &map.line5),
        None,
        "★ line 5 carries NO -0- clause ⇒ a zero is BLANK. A printed 0 here would swear that total \
         payments were zero; a blank says nothing, which is what the form asks for."
    );
    assert_eq!(
        text_of(&doc, &fields, &map.line6).as_deref(),
        Some("0"),
        "line 6 carries a -0- clause"
    );
    assert_eq!(
        text_of(&doc, &fields, &map.line7),
        None,
        "nothing is being paid ⇒ line 7 blank"
    );
}

/// ★ KILL — the line-7 DEFAULT with no recorded extension payment: pay the balance due.
#[test]
fn line_7_defaults_to_line_6_when_the_return_records_no_extension_payment() {
    let pr = shaped(dec!(9000), dec!(6500), None);
    let map = Form4868Map::ty2024();
    let bytes = fill_form_4868(&pr, 2024, Form4868Choices::default()).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(text_of(&doc, &fields, &map.line5).as_deref(), Some("6500"));
    assert_eq!(text_of(&doc, &fields, &map.line6).as_deref(), Some("2500"));
    assert_eq!(
        text_of(&doc, &fields, &map.line7).as_deref(),
        Some("2500"),
        "no payment recorded ⇒ line 7 = line 6"
    );
}

/// ★ KILL — the line-7 DEFAULT when the return ALREADY records an extension payment (spec I-6): the
/// PRINTED Schedule 3 line 10 wins, even where it differs from line 6. A filer who recorded the
/// payment first and printed second gets back the number they entered, not a second one.
#[test]
fn line_7_defaults_to_the_printed_schedule_3_line_10_when_it_is_positive() {
    // Line 33 ($6,500) already CONTAINS the $500 extension payment, so line 5 excludes it (= $6,000)
    // and line 6 is $3,000 — deliberately different from the $500 recorded payment.
    let pr = shaped(dec!(9000), dec!(6500), Some(sch3(dec!(500), Usd::ZERO)));
    let map = Form4868Map::ty2024();
    let bytes = fill_form_4868(&pr, 2024, Form4868Choices::default()).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(text_of(&doc, &fields, &map.line5).as_deref(), Some("6000"));
    assert_eq!(text_of(&doc, &fields, &map.line6).as_deref(), Some("3000"));
    assert_eq!(
        text_of(&doc, &fields, &map.line7).as_deref(),
        Some("500"),
        "the recorded extension payment is the default for line 7, NOT line 6"
    );
}

/// ★ KILL — *"Don't include on line 5 the amount you're paying with this"* *"Form 4868."* An
/// excess-Social-Security credit puts a Schedule 3 on the return with line 10 = $0; line 5 must then
/// be the WHOLE of line 33. Subtracting "Schedule 3 exists" instead of "Schedule 3 line 10" is the
/// mistake this pins, and it would understate payments (and so OVERSTATE the balance due) by the
/// whole §6413(c) credit.
#[test]
fn an_excess_ss_credit_alone_leaves_line_5_at_the_whole_of_line_33() {
    let pr = shaped(dec!(9000), dec!(6500), Some(sch3(Usd::ZERO, dec!(300))));
    let map = Form4868Map::ty2024();
    let bytes = fill_form_4868(&pr, 2024, Form4868Choices::default()).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(
        text_of(&doc, &fields, &map.line5).as_deref(),
        Some("6500"),
        "line 10 is zero ⇒ nothing is excluded"
    );
    assert_eq!(text_of(&doc, &fields, &map.line6).as_deref(), Some("2500"));
}

/// ★ KILL — the same Schedule 3 carrying BOTH: only line 10 comes out of line 5. The excess-SS
/// credit is a payment the filer already made and it stays counted.
#[test]
fn a_schedule_3_with_both_credits_excludes_only_the_extension_payment_from_line_5() {
    let pr = shaped(dec!(9000), dec!(6500), Some(sch3(dec!(500), dec!(300))));
    let map = Form4868Map::ty2024();
    let bytes = fill_form_4868(&pr, 2024, Form4868Choices::default()).expect("fills");
    let (doc, fields) = readback(&bytes);

    assert_eq!(
        text_of(&doc, &fields, &map.line5).as_deref(),
        Some("6000"),
        "6500 − 500 (the extension payment) — the $300 excess-SS credit is NOT excluded"
    );
}

/// ★ KILL — `--pay` refusals, in the FILLER (the command refuses them too; fail closed twice).
/// A negative payment is not a payment; a payment with cents contradicts the form's own
/// all-or-nothing rounding rule beside four whole-dollar lines. Neither is rounded on the filer's
/// behalf: the number on a signed application must be the one they chose.
#[test]
fn a_negative_or_fractional_pay_is_refused_with_no_bytes() {
    let pr = shaped(dec!(9000), dec!(6500), None);

    let cents = fill_form_4868(
        &pr,
        2024,
        Form4868Choices {
            pay: Some(dec!(2500.75)),
            out_of_country: false,
        },
    )
    .expect_err("a payment with cents must be refused");
    let msg = cents.to_string();
    assert!(
        msg.contains("4868") && msg.contains("line 7") && msg.contains("whole dollars"),
        "the refusal names the form, the line and the remedy: {msg}"
    );

    let negative = fill_form_4868(
        &pr,
        2024,
        Form4868Choices {
            pay: Some(dec!(-1)),
            out_of_country: false,
        },
    )
    .expect_err("a negative payment must be refused");
    assert!(
        negative.to_string().contains("cannot be negative"),
        "{negative}"
    );

    // …and a payment ABOVE line 6 is ALLOWED: paying ahead of an estimate is the filer's choice.
    let bytes = fill_form_4868(
        &pr,
        2024,
        Form4868Choices {
            pay: Some(dec!(5000)),
            out_of_country: false,
        },
    )
    .expect("paying more than line 6 is not a refusal");
    let (doc, fields) = readback(&bytes);
    let map = Form4868Map::ty2024();
    assert_eq!(text_of(&doc, &fields, &map.line7).as_deref(), Some("5000"));
}

// ── Part I ──────────────────────────────────────────────────────────────────────────────────────

/// ★ KILL — line 3 is a JOINT-RETURN line. *"If you plan to file a joint return, enter on line 2 the
/// social security"* … *"the other SSN to be shown on the joint return."*
///
/// The discriminating pair: ONE household, ONE header carrying a spouse, TWO filing statuses. That
/// is the whole point — `ReturnHeader.spouse` is `Some` on MFS as well (spec I-1), so a filler gated
/// on the spouse's presence rather than on the filing status would pass an MFS-only test and still
/// print a joint return's SSN.
#[test]
fn the_spouse_ssn_is_printed_on_mfj_and_blank_on_mfs_from_the_same_header() {
    let map = Form4868Map::ty2024();

    let mut joint = shaped(dec!(9000), dec!(6500), None);
    joint.filing_status = FilingStatus::Mfj;
    assert!(
        joint.header.spouse.is_some(),
        "premise: the fixture header carries a spouse"
    );
    let (doc, fields) =
        readback(&fill_form_4868(&joint, 2024, Form4868Choices::default()).unwrap());
    assert_eq!(
        text_of(&doc, &fields, &map.spouse_ssn).as_deref(),
        Some("987-65-4321"),
        "MFJ ⇒ line 3 carries the second SSN, hyphenated (the cell's /MaxLen is 11)"
    );

    let mut separate = joint.clone();
    separate.filing_status = FilingStatus::Mfs;
    assert!(
        separate.header.spouse.is_some(),
        "★ the header STILL carries a spouse — only the status changed"
    );
    let (doc, fields) =
        readback(&fill_form_4868(&separate, 2024, Form4868Choices::default()).unwrap());
    assert_eq!(
        text_of(&doc, &fields, &map.spouse_ssn),
        None,
        "MFS ⇒ line 3 blank; printing it would claim a joint return"
    );
    assert!(
        text_of(&doc, &fields, &map.taxpayer_ssn).is_some(),
        "…while line 2 is still filled"
    );
}

/// ★ The identity cells the geometric oracle CANNOT check — `free` placements catch stray writes,
/// never missing ones — so the read-back asserts them non-empty by name. An unnamed extension
/// application is the failure mode `push_identity`'s own doc comment warns about.
#[test]
fn the_name_address_and_taxpayer_ssn_cells_all_come_back_filled() {
    let pr = shaped(dec!(9000), dec!(6500), None);
    let map = Form4868Map::ty2024();
    let (doc, fields) = readback(&fill_form_4868(&pr, 2024, Form4868Choices::default()).unwrap());

    for (what, fqn) in [
        ("name line", &map.name_line),
        ("street", &map.address_street),
        ("city", &map.address_city),
        ("state", &map.address_state),
        ("ZIP", &map.address_zip),
        ("taxpayer SSN", &map.taxpayer_ssn),
    ] {
        assert!(
            text_of(&doc, &fields, fqn).is_some(),
            "Part I's {what} cell must come back filled"
        );
    }
    assert_eq!(
        text_of(&doc, &fields, &map.name_line).as_deref(),
        Some(pr.header.name_line.as_str()),
        "the extension carries the RETURN's own name line, verbatim"
    );
}

/// ★ KILL — every cell the map's `[census]` accounts for stays BLANK: the three fiscal-year header
/// cells (btctax is calendar-year only), line 9's Form 1040-NR box, and the page-3 confirmation cell.
///
/// The population is the map's own census keys, never a hand-list — so a cell moved from the census
/// into the map, or a new census entry, is covered the day it lands.
#[test]
fn every_censused_cell_stays_blank_on_both_years() {
    for year in [2024, 2025] {
        let map = Form4868Map::for_year(year).expect("both bundled years wire");
        let pr = shaped(dec!(9000), dec!(6500), Some(sch3(dec!(500), dec!(300))));
        let bytes = fill_form_4868(
            &pr,
            year,
            Form4868Choices {
                pay: Some(dec!(1000)),
                out_of_country: true,
            },
        )
        .expect("fills");
        let (doc, fields) = readback(&bytes);

        assert!(
            !map.census.is_empty(),
            "premise: the {year} map records its unfilled cells"
        );
        for fqn in map.census.keys() {
            let f = fields.iter().find(|f| &f.fqn == fqn).expect("census FQN");
            let filled = if f.is_button {
                checkbox_on(&doc, f.id).is_some()
            } else {
                text_value(&doc, f.id).is_some_and(|s| !s.is_empty())
            };
            assert!(
                !filled,
                "TY{year}: {fqn} is censused as never-filled but came back FILLED"
            );
        }
    }
}

// ── Line 8, the one assertion the filer makes ───────────────────────────────────────────────────

/// ★★ KILL (mutation-verified) — `out_of_country` ⇒ `c1_1` CHECKED; absent ⇒ blank.
///
/// The "absent ⇒ blank" half is worthless alone: a filler that never wrote line 8 at all would pass
/// it, and on the printed page an unchecked box and a never-written box are the same mark. Only the
/// checked half discriminates, which is why both are asserted here in one test, from one map, and
/// why the state is read back out of the SERIALIZED PDF (`/V`, else `/AS`) rather than from the
/// struct that wrote it.
#[test]
fn out_of_country_checks_line_8_and_its_absence_leaves_the_box_unwritten() {
    let pr = shaped(dec!(9000), dec!(6500), None);
    let map = Form4868Map::ty2024();

    let (doc, fields) = readback(
        &fill_form_4868(
            &pr,
            2024,
            Form4868Choices {
                pay: None,
                out_of_country: true,
            },
        )
        .unwrap(),
    );
    assert_eq!(
        check_of(&doc, &fields, &map.line8.field).as_deref(),
        Some(map.line8.on.as_str()),
        "line 8 must come back CHECKED in the PDF's own on-state, not merely 'set' in a struct"
    );

    let (doc, fields) = readback(
        &fill_form_4868(
            &pr,
            2024,
            Form4868Choices {
                pay: None,
                out_of_country: false,
            },
        )
        .unwrap(),
    );
    assert_eq!(
        check_of(&doc, &fields, &map.line8.field),
        None,
        "silence FORGOES the extra two months and asserts nothing — the box is never written"
    );
}

// ── Both bundled years, and the geometric read-back ─────────────────────────────────────────────

/// Both revisions fill, from their OWN map and their OWN template. The two 4868 maps differ in the
/// Part I subform name (`Part1_ReadOrder` in 2024, `PartI_ReadOrder` in 2025), so a fill that
/// reached for the other year's map would fail closed on a missing field — which is the point.
#[test]
fn both_bundled_years_fill_the_same_chain_from_their_own_map() {
    let pr = shaped(dec!(9000), dec!(6500), Some(sch3(dec!(500), Usd::ZERO)));
    for year in [2024, 2025] {
        let map = Form4868Map::for_year(year).expect("wired");
        let bytes = fill_form_4868(&pr, year, Form4868Choices::default()).expect("fills");
        let (doc, fields) = readback(&bytes);
        assert_eq!(text_of(&doc, &fields, &map.line4).as_deref(), Some("9000"));
        assert_eq!(text_of(&doc, &fields, &map.line5).as_deref(), Some("6000"));
        assert_eq!(text_of(&doc, &fields, &map.line6).as_deref(), Some("3000"));
        assert_eq!(text_of(&doc, &fields, &map.line7).as_deref(), Some("500"));
        assert!(text_of(&doc, &fields, &map.name_line).is_some());
    }
}

/// ★★ KILL (harness B1) — the geometric read-back is not decoration: a map that points a Part II
/// money line at a Part I identity cell FAILS THE FILL CLOSED, with no bytes returned.
///
/// Planted on a COPY of the committed TOML (the committed map is never touched): line 6's FQN is
/// repointed at the wide name-line cell, which sits far outside the money column's x-cluster. The
/// clean map is filled in the same test, so a checker that reds on everything cannot pass either.
#[test]
fn a_money_line_pointed_at_an_identity_cell_fails_the_fill_closed() {
    let pr = shaped(dec!(9000), dec!(6500), None);
    let src = btctax_forms::bundled::map_text(btctax_forms::bundled::Stem::F4868, 2025)
        .expect("the TY2025 Form 4868 map is bundled");

    let clean = Form4868Map::parse(src).expect("the committed map parses");
    assert!(
        fill_form_4868_with_map(&pr, Form4868Choices::default(), &clean).is_ok(),
        "premise: the COMMITTED map fills — otherwise this test reds on everything"
    );

    let doctored_src = src.replace(
        "line6 = \"topmostSubform[0].Page1[0].f1_13[0]\"",
        "line6 = \"topmostSubform[0].Page1[0].PartI_ReadOrder[0].f1_4[0]\"",
    );
    // `assert!`, not `assert_ne!`: the operands here are whole map files, and a broken plant should
    // print one line rather than two 6 KB TOML dumps.
    assert!(
        doctored_src != src,
        "the plant must actually change the map"
    );
    let doctored = Form4868Map::parse(&doctored_src).expect("the doctored map still parses");

    let err = fill_form_4868_with_map(&pr, Form4868Choices::default(), &doctored)
        .expect_err("a money line in the name cell must fail closed");
    let msg = err.to_string();
    assert!(
        msg.contains("mis-mapped") || msg.contains("geometric"),
        "the geometric oracle must be what refuses it: {msg}"
    );
}
