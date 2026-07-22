//! SP4 Known-Answer Tests: Form 8275 (Disclosure Statement, Rev. 10-2024).
//!
//! The star, as with SP2, is the **geometric, map-independent read-back** (`verify_flat`) — but 8275
//! is FREE-TEXT (no money grid), so every write is `push_free`/`FlatPlacement::free`: page-checked,
//! `/MaxLen`-checked, and inside the no-unmapped set, with no column-x cluster or ordinal-y descent to
//! assert. The fault-injection KAT below demonstrates the ONE geometric trap this form's structure
//! actually offers: column (e) "Line No." is a genuine 3-character comb cell, so a map that (mis)points
//! a wide free-text cell (like the Cohan description) at it overflows and fails closed.
//!
//! ★ Year coverage (arch r1 I-6 / tax r1 M-7): Form 8275 is REVISION-versioned, not tax-year-versioned
//! — the ONE bundled Rev. 10-2024 asset + map is aliased to EVERY `SUPPORTED_YEAR` (2017/2024/2025).
//! The per-year fill KAT below pins this for both non-2024 years.

use btctax_core::tax::form8275::Part1Item;
use btctax_core::tax::printed::Printed8275;
use btctax_core::tax::testonly::kitchen_sink_header;
use btctax_forms::testonly::*;
use btctax_forms::FormsError;
use rust_decimal_macros::dec;
use sha2::{Digest, Sha256};

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

fn tv(doc: &lopdf::Document, fields: &[Field], fqn: &str) -> Option<String> {
    let f = fields.iter().find(|f| f.fqn == fqn)?;
    text_value(doc, f.id)
}

/// A 2-item disclosure: one short-term leg (no loss clamp), one long-term leg (BG-D4 loss-clamp
/// suffix present) — matches the two `Part1Item.line` shapes `disclosure_8275` actually emits.
fn sample_printed() -> Printed8275 {
    Printed8275 {
        part_i: vec![
            Part1Item {
                form: "8949".into(),
                line: "Part I \u{2014} column (e)".into(),
                description:
                    "basis estimated at the minimum daily closing price over the attested \
                    acquisition window (Cohan; the bearing-heavily minimum)"
                        .into(),
                amount: dec!(12345),
            },
            Part1Item {
                form: "8949".into(),
                line: "Part II \u{2014} column (e)".into(),
                description:
                    "basis estimated at the minimum daily closing price over the attested \
                    acquisition window (Cohan; the bearing-heavily minimum); limited so as not to \
                    report a loss from the estimate"
                        .into(),
                amount: dec!(6789),
            },
        ],
        part_ii: "The taxpayer disposed of BTC acquired via an unverified peer-to-peer purchase; \
            basis was estimated using the daily low close over the attested acquisition window, \
            consistent with Cohan v. Commissioner."
            .into(),
    }
}

const ROW1_ITEM: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line1[0].p1-t4[0]";
const ROW1_DESC: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line1[0].#subform[0].p1-t5[0]";
const ROW1_FORM_SCHEDULE: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line1[0].p1-t7[0]";
const ROW1_AMOUNT: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line1[0].p1-t9[0]";
const ROW1_CITATION_A: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line1[0].p1-t3[0]";
const ROW1_LINE_NO_E: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line1[0].p1-t8[0]";
const ROW2_ITEM: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line2[0].p1-t11[0]";
const ROW2_DESC: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line2[0].#subform[0].p1-t12[0]";
const ROW2_FORM_SCHEDULE: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line2[0].p1-t14[0]";
const ROW2_AMOUNT: &str = "topmostSubform[0].Page1[0].Table_Part1[0].Line2[0].p1-t16[0]";
const PART_II_LINE1: &str = "topmostSubform[0].Page1[0].p1-t80[0]";
const IDENTITY_NAME: &str = "topmostSubform[0].Page1[0].p1-t1[0]";
const IDENTITY_SSN: &str = "topmostSubform[0].Page1[0].p1-t2[0]";

#[test]
fn form_8275_fills_part_i_part_ii_and_identity() {
    let printed = sample_printed();
    let header = kitchen_sink_header();
    let pdf = btctax_forms::fill_form_8275(&printed, &header, 2024)
        .unwrap()
        .expect("non-empty part_i");
    let (doc, fields) = fields_of(&pdf);

    // Row 1: (b) item ← line, (c) desc ← description, (d) form_schedule ← "Form {form}", (f) amount.
    assert_eq!(
        tv(&doc, &fields, ROW1_ITEM).as_deref(),
        Some("Part I \u{2014} column (e)")
    );
    assert_eq!(
        tv(&doc, &fields, ROW1_DESC).as_deref(),
        Some(printed.part_i[0].description.as_str())
    );
    assert_eq!(
        tv(&doc, &fields, ROW1_FORM_SCHEDULE).as_deref(),
        Some("Form 8949")
    );
    assert_eq!(tv(&doc, &fields, ROW1_AMOUNT).as_deref(), Some("12345"));

    // Row 2.
    assert_eq!(
        tv(&doc, &fields, ROW2_ITEM).as_deref(),
        Some("Part II \u{2014} column (e)")
    );
    assert_eq!(tv(&doc, &fields, ROW2_AMOUNT).as_deref(), Some("6789"));

    // Part II narrative — written whole to the single free-text field.
    assert_eq!(
        tv(&doc, &fields, PART_II_LINE1).as_deref(),
        Some(printed.part_ii.as_str())
    );

    // Filer identity.
    assert_eq!(
        tv(&doc, &fields, IDENTITY_NAME).as_deref(),
        Some("John Doe & Jane Doe")
    );
    assert_eq!(
        tv(&doc, &fields, IDENTITY_SSN).as_deref(),
        Some("123-45-6789")
    );

    // Column (a) [citation] and (e) [Line No.] are never written — no applicable data.
    assert_eq!(tv(&doc, &fields, ROW1_CITATION_A), None);
    assert_eq!(tv(&doc, &fields, ROW1_LINE_NO_E), None);
}

/// ★ T15 review Minor-2: the free-text `verify_flat` oracle checks page + `/MaxLen` + no-unmapped, so a
/// map that SWAPPED two wide same-page cells (item↔desc, form_schedule↔item, or a row1↔row2 reorder)
/// would fail-closed-silently — verify_flat can't tell a wide cell's neighbour apart. This per-field
/// SENTINEL readback closes that: EVERY writable Part-I field (both rows' item/desc/form/amount) gets a
/// DISTINCT value, and we read each back BY FIELD NAME and assert its own sentinel landed there — so any
/// map swap between two written fields reds this (the existing fill KAT missed row-2 desc/form_schedule
/// and shared "Form 8949" across both rows' form cells, so a form-cell swap survived it).
#[test]
fn form_8275_lands_each_part_i_field_in_its_own_widget_no_swap() {
    let printed = Printed8275 {
        part_i: vec![
            Part1Item {
                form: "R1FORM".into(),
                line: "R1ITEM".into(),
                description: "R1DESC".into(),
                amount: dec!(11111),
            },
            Part1Item {
                form: "R2FORM".into(),
                line: "R2ITEM".into(),
                description: "R2DESC".into(),
                amount: dec!(22222),
            },
        ],
        part_ii: "R_PARTII_NARRATIVE".into(),
    };
    let pdf = btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("non-empty part_i");
    let (doc, fields) = fields_of(&pdf);

    // Each distinct sentinel lands in its OWN widget — a swap of any two would cross the values.
    for (fqn, want) in [
        (ROW1_ITEM, "R1ITEM"),
        (ROW1_DESC, "R1DESC"),
        (ROW1_FORM_SCHEDULE, "Form R1FORM"),
        (ROW1_AMOUNT, "11111"),
        (ROW2_ITEM, "R2ITEM"),
        (ROW2_DESC, "R2DESC"),
        (ROW2_FORM_SCHEDULE, "Form R2FORM"),
        (ROW2_AMOUNT, "22222"),
        (PART_II_LINE1, "R_PARTII_NARRATIVE"),
    ] {
        assert_eq!(
            tv(&doc, &fields, fqn).as_deref(),
            Some(want),
            "field {fqn} must hold its own sentinel {want:?} (a map swap would cross it)"
        );
    }
}

#[test]
fn form_8275_none_when_part_i_is_empty() {
    let printed = Printed8275 {
        part_i: vec![],
        part_ii: "unused".into(),
    };
    assert!(
        btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024)
            .unwrap()
            .is_none(),
        "an empty Part I means nothing to disclose — the form must not be written"
    );
}

#[test]
fn fault_injected_8275_desc_mapped_to_maxlen3_line_no_cell_is_red() {
    // ★ Simulates a swapped-column map: (c) Detailed Description lands on (e) Line No. — a genuine
    // 3-character comb cell. The long Cohan explanation cannot fit, so verify_flat's /MaxLen leg FAILS
    // CLOSED (no PDF bytes are returned). This is the ONE geometric trap 8275's free-text layout
    // actually offers (no column-x cluster / descent to swap, since every write here is `push_free`).
    let mut map = Form8275Map::ty2024();
    map.rows[0].desc = ROW1_LINE_NO_E.to_string();
    let err = fill_8275_with_map(&sample_printed(), &kitchen_sink_header(), &map).unwrap_err();
    assert!(
        matches!(&err, FormsError::CellOverflow { max_len, fqn, .. }
            if *max_len == 3 && fqn == ROW1_LINE_NO_E),
        "expected CellOverflow at the /MaxLen 3 cell, got {err:?}"
    );
}

#[test]
fn form_8275_is_byte_deterministic() {
    let a = btctax_forms::fill_form_8275(&sample_printed(), &kitchen_sink_header(), 2024)
        .unwrap()
        .unwrap();
    let b = btctax_forms::fill_form_8275(&sample_printed(), &kitchen_sink_header(), 2024)
        .unwrap()
        .unwrap();
    assert_eq!(a, b, "same (data, form) must be byte-identical");
    assert_eq!(
        hex(&Sha256::digest(&a)),
        GOLDEN_8275_SHA256,
        "8275 fill changed — if intentional, update GOLDEN_8275_SHA256"
    );
}
const GOLDEN_8275_SHA256: &str = "aa70a5c55901586ec04e5659ff70038c06d8bef38d034ff47987b381893d5032";

#[test]
fn form_8275_fills_for_every_supported_non_2024_year() {
    // ★ arch r1 I-6 / tax r1 M-7: Form 8275 is REVISION-versioned, not tax-year-versioned — the SAME
    // bundled asset + map is aliased to 2017 AND 2025 (not just 2024), so a promoted disposal filed in
    // either year still gets a real fillable disclosure.
    let printed = sample_printed();
    let header = kitchen_sink_header();
    let pdf_2024 = btctax_forms::fill_form_8275(&printed, &header, 2024)
        .unwrap()
        .unwrap();
    let pdf_2025 = btctax_forms::fill_form_8275(&printed, &header, 2025)
        .unwrap()
        .expect("2025 must also produce a filled 8275 (aliased revision)");
    let pdf_2017 = btctax_forms::fill_form_8275(&printed, &header, 2017)
        .unwrap()
        .expect("2017 must also produce a filled 8275 (aliased revision)");

    // Since the SAME underlying asset/map content is used for all three years (only the map's `year`
    // metadata tag differs, which is never itself written to the PDF), the output must be BYTE
    // IDENTICAL across all three — the strongest possible pin of "aliased to every year".
    assert_eq!(
        pdf_2024, pdf_2025,
        "2024 and 2025 fills must be byte-identical"
    );
    assert_eq!(
        pdf_2024, pdf_2017,
        "2024 and 2017 fills must be byte-identical"
    );

    // Spot-check 2025 and 2017 actually carry the data (not just "didn't error").
    for pdf in [&pdf_2025, &pdf_2017] {
        let (doc, fields) = fields_of(pdf);
        assert_eq!(tv(&doc, &fields, ROW1_AMOUNT).as_deref(), Some("12345"));
        assert_eq!(
            tv(&doc, &fields, IDENTITY_NAME).as_deref(),
            Some("John Doe & Jane Doe")
        );
    }
}

#[test]
fn map_year_matches_bundled_pdf_fieldset_for_every_supported_year() {
    // Form 8275 aliases the SAME bundled PDF to every SUPPORTED_YEAR, so this asserts the map's field
    // names all exist in that one asset, once per year the map is stamped for.
    for &year in btctax_forms::SUPPORTED_YEARS {
        let map = Form8275Map::for_year(year).unwrap();
        assert_eq!(map.year, year);
        let set = fieldset(F8275_PDF_2024);
        for name in map.field_names() {
            assert!(
                set.contains(name),
                "year {year}: 8275 map field absent from the bundled PDF: {name}"
            );
        }
    }
}

#[test]
fn unsupported_year_rejected_for_form_8275() {
    let err =
        btctax_forms::fill_form_8275(&sample_printed(), &kitchen_sink_header(), 2023).unwrap_err();
    assert!(
        matches!(err, FormsError::UnsupportedYear(2023)),
        "got {err:?}"
    );
}
