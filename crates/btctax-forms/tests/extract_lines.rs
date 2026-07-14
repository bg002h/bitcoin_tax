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
