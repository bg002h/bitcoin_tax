//! **The line-keyed inverse transcriber** (`testonly::extract_lines`) — P7 / follow-up `p6-r1-n1`.
//!
//! Every read-back in this crate until now went through a fully-qualified AcroForm leaf name:
//!
//! ```ignore
//! assert_eq!(tv(&pdf, "topmostSubform[0].Page1[0].f1_3[0]").as_deref(), Some("280000")); // L1
//! ```
//!
//! The `// L1` is the part a reader needs and the part the compiler cannot check. `extract_lines`
//! inverts the committed map instead: it hands back what the filled PDF actually SAYS, keyed by the
//! logical line, so a test can assert `lines["line1"] == "280000"` and mean it.
//!
//! It is deliberately generic over forms — it walks the map TOML rather than any typed `*Map` struct
//! — because its consumer (the P7 packet round-trip) must transcribe **every** form in the packet
//! without knowing which one it is holding.

use btctax_core::tax::other_taxes::form_8959_lines;
use btctax_core::tax::printed::{Form1040Lines, ScheduleBLines, ScheduleBRow};
use btctax_core::tax::se::SeTaxResult;
use btctax_core::tax::testonly::kitchen_sink_header;
use btctax_core::tax::types::FilingStatus;
use btctax_core::Usd;
use btctax_forms::testonly::{extract_lines, F1040_MAP_2024, F8959_MAP_2024, SCHEDULE_B_MAP_2024};
use rust_decimal_macros::dec;

/// The deep/02 example-2 household: MFJ, $280,000 W-2 Medicare wages, $60,000 of mining — the same
/// fixture `full_return_forms.rs` pins by leaf name, so the two read-backs are describing one PDF.
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

/// The transcriber returns the FILLED cells, keyed by the map's logical line names.
#[test]
fn extract_lines_reads_a_filled_form_back_by_line_number() {
    let se = se_mining_60k_mfj();
    let lines = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));
    let pdf = btctax_forms::fill_form_8959(&lines, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("this household owes Additional Medicare Tax");

    let got = extract_lines(&pdf, F8959_MAP_2024).expect("the filled 8959 transcribes");

    // The same figures full_return_forms.rs pins by leaf name — but said in the form's own language.
    assert_eq!(got.get("line1").map(String::as_str), Some("280000"));
    assert_eq!(got.get("line5").map(String::as_str), Some("250000"));
    assert_eq!(got.get("line7").map(String::as_str), Some("270"));
    assert_eq!(got.get("line8").map(String::as_str), Some("55410"));
    assert_eq!(got.get("line13").map(String::as_str), Some("499"));
    assert_eq!(got.get("line18").map(String::as_str), Some("769"));
}

/// Nested groups come back dotted, so the identity block is reachable too — the packet round-trip
/// needs to prove every schedule carries the filer's name and SSN.
#[test]
fn extract_lines_descends_into_nested_groups() {
    let se = se_mining_60k_mfj();
    let lines = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));
    let pdf = btctax_forms::fill_form_8959(&lines, &kitchen_sink_header(), 2024)
        .unwrap()
        .unwrap();

    let got = extract_lines(&pdf, F8959_MAP_2024).unwrap();
    assert!(
        got.contains_key("identity.name"),
        "the identity group must transcribe as `identity.name`, got keys: {:?}",
        got.keys().collect::<Vec<_>>()
    );
    assert!(got.contains_key("identity.ssn"));
}

/// ★ An OFF checkbox must be ABSENT from the transcript, not present.
///
/// This is what makes the transcriber usable as a statement about what the return SAYS. The Digital
/// Asset question is a Yes/No PAIR of widgets and exactly one of them is on; a transcriber that
/// emitted both would make `assert_eq!(got["da_yes"], "1")` pass on a return that also said No. The
/// fixture answers **Yes**, so `da_no` must not appear at all.
#[test]
fn extract_lines_omits_the_off_half_of_a_checkbox_pair() {
    let pdf =
        btctax_forms::fill_form_1040_full(&f1040(), &kitchen_sink_header(), FilingStatus::Single, 2024)
            .unwrap();

    let got = extract_lines(&pdf, F1040_MAP_2024).unwrap();
    assert_eq!(
        got.get("da_yes").map(String::as_str),
        Some("1"),
        "the fixture answers YES to the digital-asset question"
    );
    assert!(
        !got.contains_key("da_no"),
        "the NO box is off; transcribing it would make the Yes/No pair unfalsifiable"
    );
}

/// A repeating table's UNUSED rows must be absent — and the used ones must transcribe by index.
///
/// Schedule B carries **14** interest-payer row slots. A household with two payers fills two and
/// leaves twelve blank, and those twelve are the only genuinely-unwritten money cells anywhere in the
/// packet: on the 1040 proper every mapped cell is written (line 35a is filled with an explicit ZERO,
/// which is a statement, not a blank).
///
/// This also pins the ARRAY path — `part1_rows[0].payer` — which the packet round-trip needs in order
/// to read the 8949's rows back.
#[test]
fn extract_lines_indexes_table_rows_and_omits_the_unused_ones() {
    let lines = sch_b(
        vec![row("ORACLE BANK", dec!(1200)), row("SECOND BANK", dec!(300))],
        vec![],
        false,
        false,
    );
    let pdf = btctax_forms::fill_schedule_b(&lines, &kitchen_sink_header(), 2024).unwrap();

    let got = extract_lines(&pdf, SCHEDULE_B_MAP_2024).unwrap();

    assert_eq!(
        got.get("part1_rows[0].payer").map(String::as_str),
        Some("ORACLE BANK")
    );
    assert_eq!(
        got.get("part1_rows[0].amount").map(String::as_str),
        Some("1200")
    );
    assert_eq!(
        got.get("part1_rows[1].payer").map(String::as_str),
        Some("SECOND BANK")
    );
    // Slots 2..14 are mapped, real widgets — and this filer has no third payer. They are blank on the
    // paper and must be blank in the transcript.
    assert!(
        !got.contains_key("part1_rows[2].payer"),
        "row 2 is an unused slot; transcribing it would invent a payer"
    );
    assert!(!got.contains_key("part1_rows[13].amount"));
}

/// The map's METADATA keys are not cells and must not be transcribed as if they were.
///
/// `form = "f8959"` and `year = 2024` sit in the same TOML table as the line cells. A transcriber
/// that treated every string as a field name would emit `form` as a line — and the round-trip would
/// then be asserting on the map's own header rather than on the PDF.
#[test]
fn extract_lines_ignores_map_metadata() {
    let se = se_mining_60k_mfj();
    let lines = form_8959_lines(FilingStatus::Mfj, dec!(280000), dec!(4240), Some(&se));
    let pdf = btctax_forms::fill_form_8959(&lines, &kitchen_sink_header(), 2024)
        .unwrap()
        .unwrap();

    let got = extract_lines(&pdf, F8959_MAP_2024).unwrap();
    assert!(!got.contains_key("form"), "`form` is metadata, not a cell");
    assert!(!got.contains_key("year"), "`year` is metadata, not a cell");
}
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

// ───────────────── Fable P7 r2 I1 + I2 — Form 8995's Part I row 1i ─────────────────

/// ★ **The business TIN is the PROPRIETOR's, not the primary taxpayer's.**
///
/// A spouse-owned business files under the SPOUSE's name and SSN even on a joint return — Schedule C
/// and Schedule SE already do this via `ReturnHeader::proprietor`. Form 8995 hardcoded
/// `header.taxpayer`, so an MFJ return whose spouse ran the mining business filed a Schedule C under
/// the spouse's SSN and an 8995 claiming the §199A deduction for a business whose TIN it reported as
/// the *taxpayer's* — a business TIN matching no Schedule C in the same packet.
#[test]
fn form_8995_row_1i_carries_the_proprietors_tin_not_the_taxpayers() {
    use btctax_core::tax::return_inputs::{Owner, Person, ScheduleCInputs};
    use btctax_core::tax::packet::ReturnHeader;

    let mut ri = btctax_core::tax::return_inputs::ReturnInputs {
        filing_status: FilingStatus::Mfj,
        ..Default::default()
    };
    ri.header.taxpayer = Person {
        first_name: "Primary".into(),
        last_name: "Filer".into(),
        ssn: "111111111".into(),
        ..Default::default()
    };
    ri.header.spouse = Some(Person {
        first_name: "Spouse".into(),
        last_name: "Filer".into(),
        ssn: "222222222".into(),
        ..Default::default()
    });
    // ★ The SPOUSE runs the business.
    ri.schedule_c = Some(ScheduleCInputs {
        owner: Owner::Spouse,
        business_description: "Bitcoin mining".into(),
        ..Default::default()
    });
    let header = ReturnHeader::build(&ri, 2024).unwrap();

    let lines = btctax_core::tax::qbi::form_8995_lines(
        "Bitcoin mining",
        dec!(55761), // business QBI
        Usd::ZERO,
        Usd::ZERO,
        dec!(81161), // TI before QBI
        Usd::ZERO,
    )
    .expect("there is QBI");
    let pdf = btctax_forms::fill_form_8995(&lines, &header, 2024).unwrap();
    let got = extract_lines(&pdf, btctax_forms::testonly::F8995_MAP_2024).unwrap();

    assert_eq!(
        got.get("row1_tin").map(String::as_str),
        Some("222-22-2222"),
        "row 1i(b) is the BUSINESS's TIN — the spouse owns it, so it is the SPOUSE's SSN. Printing the \
         primary taxpayer's would report a TIN that matches no Schedule C in the packet."
    );
    assert_eq!(
        got.get("row1_business").map(String::as_str),
        Some("Bitcoin mining")
    );
}

/// ★ **A non-zero line 2 over an unnamed business FAILS CLOSED.**
///
/// The r1 fix keyed the row off the business NAME, which made the whole defect conditional on
/// `business_description` — a `#[serde(default)]` free-text field that nothing validated. An import
/// omitting it produced a blank row under a non-zero line 2: the very defect r1 raised, re-created.
/// Core now REFUSES an unnamed Schedule C; this is the second line of defence, because a form claiming
/// a deduction for a business it cannot name must never be produced at all.
#[test]
fn form_8995_refuses_to_file_a_qbi_total_for_an_unnamed_business() {
    let lines = btctax_core::tax::qbi::form_8995_lines(
        "", // no name — as an import omitting `business_description` would give
        dec!(55761),
        Usd::ZERO,
        Usd::ZERO,
        dec!(81161),
        Usd::ZERO,
    )
    .expect("there is QBI");
    assert!(
        lines.line2 > Usd::ZERO,
        "the fixture must have a non-zero line 2, else this test is vacuous"
    );

    let err = btctax_forms::fill_form_8995(&lines, &kitchen_sink_header(), 2024)
        .expect_err("a QBI total over an unnamed business must not produce a PDF");
    let msg = err.to_string();
    assert!(
        msg.contains("EMPTY column") || msg.contains("never names"),
        "the refusal must say WHY: got {msg}"
    );
}

/// A REIT-only Form 8995 leaves Part I BLANK. The other half of the contract.
///
/// r2 caught the golden-packet test that *claimed* to pin this asserting only that the whole form was
/// absent, and pointing at "unit KATs" that did not exist. Inventing a trade or business for a filer
/// whose only QBI is REIT dividends would name a business they do not have.
#[test]
fn form_8995_with_only_reit_dividends_leaves_part_i_blank() {
    let lines = btctax_core::tax::qbi::form_8995_lines(
        "",         // no trade or business…
        Usd::ZERO,  // …and no business QBI
        dec!(10000), // just REIT dividends
        Usd::ZERO,
        dec!(100000),
        dec!(20000),
    )
    .expect("REIT dividends are QBI");

    assert_eq!(lines.line2, Usd::ZERO, "no business ⇒ line 2 is zero");
    let pdf = btctax_forms::fill_form_8995(&lines, &kitchen_sink_header(), 2024).unwrap();
    let got = extract_lines(&pdf, btctax_forms::testonly::F8995_MAP_2024).unwrap();

    for cell in ["row1_business", "row1_tin", "row1_qbi"] {
        assert!(
            !got.contains_key(cell),
            "{cell} must be BLANK — this filer has no trade or business, and the form must not name one"
        );
    }
    assert_eq!(
        got.get("line6").map(String::as_str),
        Some("10000"),
        "the REIT leg still files"
    );
}
