//! T2 KATs: > 11-row pagination (rename-per-copy, per-copy totals) + the DRAFT estimate watermark.

mod common;
use common::*;

use btctax_core::{Form8949Part, Form8949Row};
use btctax_forms::testonly::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// 26 short-term + 13 long-term rows → ⌈26/11⌉=3 and ⌈13/11⌉=2 → 3 physical copies (6 pages).
fn big_fixture() -> Vec<Form8949Row> {
    let mut rows = Vec::new();
    for i in 0..26u32 {
        rows.push(row(
            Form8949Part::ShortTerm,
            &format!("{i}.00000000 BTC"),
            dec!(100) * Decimal::from(i + 1),
            dec!(50),
            false,
        ));
    }
    for i in 0..13u32 {
        rows.push(row(
            Form8949Part::LongTerm,
            &format!("{i}.00000000 BTC"),
            dec!(200),
            dec!(50),
            false,
        ));
    }
    rows
}

fn values_ending(doc: &lopdf::Document, fields: &[Field], suffix: &str) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.fqn.ends_with(suffix))
        .filter_map(|f| text_value(doc, f.id))
        .collect()
}

#[test]
fn eleven_rows_per_page() {
    let bytes = btctax_forms::fill_form_8949(&big_fixture(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    assert_eq!(doc.get_pages().len(), 6, "3 copies × 2 pages");

    // Each physical Part I page holds AT MOST 11 rows: the col-a descriptor appears once per filled
    // row. Copy 0 fills all 11 (row 11 = f1_83 present); the last copy fills the 4-row remainder.
    let fields = collect_fields(&doc).unwrap();
    let row11_a = values_ending(&doc, &fields, "Row11[0].f1_83[0]"); // last row's col-a
    assert_eq!(
        row11_a.len(),
        2,
        "row 11 is filled on the two FULL Part I copies only (26 = 11+11+4)"
    );
}

#[test]
fn overflow_renames_fields_per_copy_no_shared_value() {
    let bytes = btctax_forms::fill_form_8949(&big_fixture(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let fields = collect_fields(&doc).unwrap();

    // The three Part I copies each carry their own Row-1 col-a value — proof the copies do NOT share
    // one /V (the ISO 32000 same-name trap the rename defeats).
    let mut r1 = values_ending(&doc, &fields, "Row1[0].f1_03[0]");
    r1.sort();
    r1.dedup();
    assert_eq!(
        r1,
        vec![
            "0.00000000 BTC".to_string(),
            "11.00000000 BTC".to_string(),
            "22.00000000 BTC".to_string()
        ],
        "each copy's Row 1 shows its own first row"
    );

    // And every field's fully-qualified name is unique (no duplicate FQNs across copies).
    let mut fqns: Vec<&str> = fields.iter().map(|f| f.fqn.as_str()).collect();
    let total = fqns.len();
    fqns.sort_unstable();
    fqns.dedup();
    assert_eq!(
        fqns.len(),
        total,
        "all field names are unique after renaming"
    );
}

#[test]
fn each_copy_has_its_own_totals() {
    let bytes = btctax_forms::fill_form_8949(&big_fixture(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let fields = collect_fields(&doc).unwrap();
    // Part I proceeds totals (f1_91) — one per copy, all distinct.
    let mut totals = values_ending(&doc, &fields, ".f1_91[0]");
    totals.sort();
    // copy0 rows 1..11 = 100*(1..11)=6600; copy1 rows 12..22 = 100*(12..22)=18700; copy2 = 100*(23..26)=9800.
    assert_eq!(totals, vec!["18700", "6600", "9800"]);
    assert_eq!(
        totals
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
}

// ── DRAFT estimate watermark ─────────────────────────────────────────────────────────────────────

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn pseudo_fill_is_watermarked() {
    let clean = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let stamped = btctax_forms::stamp_draft_watermark(&clean).unwrap();
    assert!(
        contains(&stamped, b"ESTIMATE, NOT FOR FILING"),
        "stamped output must carry the DRAFT watermark text"
    );
    // Still a valid, XFA-free PDF with the same page count.
    let doc = load(&stamped).unwrap();
    assert!(!pdf_has_xfa(&doc).unwrap());
    assert_eq!(
        doc.get_pages().len(),
        load(&clean).unwrap().get_pages().len()
    );
}

#[test]
fn real_fill_is_clean() {
    let clean = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    assert!(
        !contains(&clean, b"ESTIMATE, NOT FOR FILING"),
        "a real-ledger fill must NOT be watermarked"
    );
}
