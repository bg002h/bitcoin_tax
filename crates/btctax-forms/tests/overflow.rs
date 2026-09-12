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

/// The two stamps are independent and compose: a pseudo-reconciled crypto-slice 1040 carries BOTH,
/// on opposite diagonals. Each says a different thing — "these numbers are fictional" and "this form
/// is two cells of a return" — so neither may swallow the other.
#[test]
fn the_two_watermarks_compose_without_swallowing_each_other() {
    let clean = btctax_forms::fill_form_8949(&mixed_rows(), 2025).unwrap();
    let both = btctax_forms::stamp_draft_watermark(
        &btctax_forms::stamp_partial_worksheet_watermark(&clean).unwrap(),
    )
    .unwrap();
    assert!(contains(&both, b"ESTIMATE, NOT FOR FILING"));
    assert!(contains(&both, b"NOT A COMPLETE FORM 1040"));
    // Order-independent: the reverse composition carries both too.
    let reversed = btctax_forms::stamp_partial_worksheet_watermark(
        &btctax_forms::stamp_draft_watermark(&clean).unwrap(),
    )
    .unwrap();
    assert!(contains(&reversed, b"ESTIMATE, NOT FOR FILING"));
    assert!(contains(&reversed, b"NOT A COMPLETE FORM 1040"));
    // Still a valid, XFA-free PDF with the same page count.
    let doc = load(&both).unwrap();
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

// ── FR-113's guard: ROW CONSERVATION across every page of every copy ─────────────────────────────

/// 20 short-term + 19 long-term rows on the 14-row 2024 grid → ⌈20/14⌉=2 and ⌈19/14⌉=2 → 2 copies
/// (4 pages), carrying **39 rows** — the journey-walk shape FR-113's fix must not disturb.
///
/// ★ Derived, not typed: the counts come from `map.rows_per_page`, so a revision that changes the
///   grid keeps producing a 2-copy overflow instead of silently collapsing to one page.
fn thirty_nine_rows(cap: usize) -> Vec<Form8949Row> {
    let mut rows = Vec::new();
    for i in 0..(cap + 6) {
        rows.push(row(
            Form8949Part::ShortTerm,
            &format!("ST-{i:03} BTC"),
            dec!(100) * Decimal::from(i + 1),
            dec!(50),
            false,
        ));
    }
    for i in 0..(cap + 5) {
        rows.push(row(
            Form8949Part::LongTerm,
            &format!("LT-{i:03} BTC"),
            dec!(200) * Decimal::from(i + 1),
            dec!(50),
            false,
        ));
    }
    rows
}

/// Every col-(a) descriptor cell the MERGED document carries a value in, across every page of every
/// copy — the emitted row set, read back off the PDF.
///
/// ★ The cells are **derived from the map** (`parts[].rows[][0]`), never a hand-written list of
///   field names: a revision that renames a cell or changes the grid re-derives the census, which is
///   the only way this stays a conservation check rather than a spot check. `merge_copies` renames
///   each copy's ROOT component, so the comparison is on the template-stable path below the root.
fn emitted_descriptions(doc: &lopdf::Document, fields: &[Field], map: &Form8949Map) -> Vec<String> {
    let below_root = |fqn: &str| match fqn.split_once('.') {
        Some((_root, rest)) => rest.to_string(),
        None => fqn.to_string(),
    };
    let col_a: Vec<String> = map
        .parts
        .iter()
        .flat_map(|p| p.rows.iter().map(|r| below_root(&r[0])))
        .collect();
    let mut out: Vec<String> = fields
        .iter()
        .filter(|f| col_a.iter().any(|suffix| f.fqn.ends_with(suffix)))
        .filter_map(|f| text_value(doc, f.id))
        .filter(|v| !v.trim().is_empty())
        .collect();
    out.sort();
    out
}

/// ★★★ **ROW CONSERVATION — the one Form 8949 pagination invariant that is a WRONG RETURN when it
/// breaks.** Every row entered is emitted exactly once, counted over every page of every copy.
///
/// Written **before** FR-113 regrouped the pages, because that fix is arithmetic on a filed form
/// dressed as a cosmetic one: a dropped or double-counted 8949 row understates or overstates the
/// return, and no page-order improvement is worth risking it. The count is the journey walk's own
/// measurement — *"39 rows over 4 pages … none was dropped, verified by counting emitted rows
/// against those entered"* — turned from a one-off observation into a standing check.
///
/// Mutation (B1 — seen RED, 2026-09-12): drop the last row of every chunk in `fill_form_8949`'s
/// `pages()` (`out.push(chunk[..chunk.len().saturating_sub(1)].to_vec())`) →
/// *"every row entered must reach a page: 39 entered, 35 emitted"*, `left: 35 right: 39`.
#[test]
fn every_entered_8949_row_is_emitted_exactly_once_across_all_pages() {
    let map = Form8949Map::ty2024();
    let rows = thirty_nine_rows(map.rows_per_page);
    let bytes = btctax_forms::fill_form_8949(&rows, 2024).unwrap();
    let doc = load(&bytes).unwrap();
    // The premise this guard is scoped to: a real multi-copy overflow, not one page.
    assert_eq!(doc.get_pages().len(), 4, "2 copies × 2 pages");

    let fields = collect_fields(&doc).unwrap();
    let emitted = emitted_descriptions(&doc, &fields, &map);
    let mut entered: Vec<String> = rows.iter().map(|r| r.description.clone()).collect();
    entered.sort();

    assert_eq!(
        emitted.len(),
        entered.len(),
        "every row entered must reach a page: {} entered, {} emitted",
        entered.len(),
        emitted.len()
    );
    // Multiset equality: a row emitted TWICE while another was dropped also fails here.
    assert_eq!(
        emitted, entered,
        "the emitted row set must be exactly the entered one — no drop, no duplicate"
    );
}

/// Which page index (0-based, document order) each emitted col-(a) cell of `part` sits on.
///
/// The widget→page join is read from each page's `/Annots`, and a field found on NO page is an
/// assertion failure rather than a silent omission — an instrument that cannot place a cell must say
/// so instead of reporting an empty set as agreement (`design/HARNESS.md` class β).
fn pages_carrying_rows(doc: &lopdf::Document, fields: &[Field], part: &PartMap) -> Vec<usize> {
    let below_root = |fqn: &str| match fqn.split_once('.') {
        Some((_root, rest)) => rest.to_string(),
        None => fqn.to_string(),
    };
    let col_a: Vec<String> = part.rows.iter().map(|r| below_root(&r[0])).collect();
    let page_annots: Vec<Vec<lopdf::ObjectId>> = doc
        .get_pages()
        .into_values()
        .map(|pid| {
            let annots = doc
                .get_dictionary(pid)
                .ok()
                .and_then(|d| d.get(b"Annots").ok())
                // `/Annots` may be an indirect reference to the array — a version of this helper
                // that skipped the deref saw every page as annotation-free and would have called a
                // blind read agreement.
                .and_then(|o| doc.dereference(o).ok())
                .and_then(|(_, o)| o.as_array().ok());
            annots
                .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
                .unwrap_or_default()
        })
        .collect();
    let mut out = Vec::new();
    for f in fields.iter().filter(|f| {
        col_a.iter().any(|s| f.fqn.ends_with(s))
            && text_value(doc, f.id).is_some_and(|v| !v.trim().is_empty())
    }) {
        let page = page_annots
            .iter()
            .position(|annots| annots.contains(&f.id))
            .unwrap_or_else(|| panic!("{} is on no page's /Annots", f.fqn));
        out.push(page);
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// ★★ **FR-113 — Part I is GROUPED before Part II across the whole document.** A filer assembling a
/// paper packet by hand used to meet short-term and long-term pages alternating (copy 1 Part I, copy
/// 1 Part II, copy 2 Part I, …), because each physical copy is a two-page form and the copies were
/// concatenated. The IRS does not require grouping; a human flipping the stack does.
///
/// This is a **permutation of the page tree only** — same pages, same fields, same values, same
/// count. Row conservation is held separately by
/// [`every_entered_8949_row_is_emitted_exactly_once_across_all_pages`], which is the invariant that
/// would make this a wrong return rather than an untidy one.
///
/// Seen RED on the pre-fix order (2026-09-12): Part I pages `[0, 2]`, Part II pages `[1, 3]`.
#[test]
fn every_part_i_page_precedes_every_part_ii_page() {
    let map = Form8949Map::ty2024();
    let short = map.part("short").expect("Part I");
    let long = map.part("long").expect("Part II");
    // The premise: within ONE physical copy the form itself puts Part I on page 1, Part II on page 2.
    assert_eq!(
        (short.page, long.page),
        (0, 1),
        "the 8949 is Part I then Part II"
    );

    let rows = thirty_nine_rows(map.rows_per_page);
    let bytes = btctax_forms::fill_form_8949(&rows, 2024).unwrap();
    let doc = load(&bytes).unwrap();
    assert_eq!(doc.get_pages().len(), 4, "2 copies × 2 pages");
    let fields = collect_fields(&doc).unwrap();

    let st = pages_carrying_rows(&doc, &fields, short);
    let lt = pages_carrying_rows(&doc, &fields, long);
    assert_eq!(st.len(), 2, "two Part I pages carry rows: {st:?}");
    assert_eq!(lt.len(), 2, "two Part II pages carry rows: {lt:?}");
    assert!(
        st.iter().max() < lt.iter().min(),
        "every Part I page must precede every Part II page — Part I on {st:?}, Part II on {lt:?}"
    );
}

/// ★★ **FR-113 and row conservation on the FULL-RETURN path.** `fill_8949_full_with_map` is a second
/// implementation of the same chunk-and-merge rule (its own `pages()`, its own copy loop), so a test
/// on the crypto-slice path holds nothing about it — the shape `CLAUDE.md`'s "derive the list, or make
/// the compiler hold it" is about, here in its two-code-paths form. Both properties are asserted
/// together because the fixture is the expensive part.
///
/// Seen RED on the pre-fix order (2026-09-12): Part I pages `[0, 2]`, Part II pages `[1, 3]`.
#[test]
fn the_full_return_8949_groups_its_parts_and_conserves_every_row() {
    use btctax_core::tax::printed::form_8949_printed;

    let map = Form8949Map::ty2024();
    let rows = thirty_nine_rows(map.rows_per_page);
    let printed = form_8949_printed(&rows).expect("there are rows");
    let pdf = btctax_forms::fill_8949_full(
        &printed,
        &btctax_core::tax::testonly::kitchen_sink_header(),
        2024,
    )
    .expect("39 legs must FILE, not refuse");

    let doc = load(&pdf).unwrap();
    assert_eq!(doc.get_pages().len(), 4, "2 copies × 2 pages");
    let fields = collect_fields(&doc).unwrap();

    // Grouped: every Part I page before every Part II page.
    let st = pages_carrying_rows(&doc, &fields, map.part("short").expect("Part I"));
    let lt = pages_carrying_rows(&doc, &fields, map.part("long").expect("Part II"));
    assert!(
        st.iter().max() < lt.iter().min(),
        "every Part I page must precede every Part II page — Part I on {st:?}, Part II on {lt:?}"
    );

    // Conserved: the emitted col-(a) descriptors are exactly the ones entered.
    let emitted = emitted_descriptions(&doc, &fields, &map);
    let mut entered: Vec<String> = rows.iter().map(|r| r.description.clone()).collect();
    entered.sort();
    assert_eq!(
        emitted, entered,
        "every full-return row entered must be emitted exactly once"
    );
}
