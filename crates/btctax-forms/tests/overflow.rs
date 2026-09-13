//! T2 KATs: > 11-row pagination (rename-per-copy, per-copy totals) + the DRAFT estimate watermark.

mod common;
use common::*;

use btctax_core::{Form8949Part, Form8949Row};
use btctax_forms::testonly::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// 26 short-term + 13 long-term rows → ⌈26/11⌉=3 Part I pages and ⌈13/11⌉=2 Part II pages on the
/// TY2025 grid — **five** filed pages, on three physical template copies.
const BIG_ST: usize = 26;
const BIG_LT: usize = 13;
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

/// ★★ **FR-218 changed the expected page count here, and the instruction is the reason.** This asserted
/// `6` — three template copies × two pages — which is one page more than the filer has anything to put
/// on: 26 short-term rows fill three Part I pages, 13 long-term rows fill two Part II pages, so copy
/// 3's Part II page was filed blank. `i8949` (`design/forms/extract/i8949--2024.txt:422-424`): *"You
/// don't need to complete and file an entire copy of Form 8949 (Parts I and II) if you can check a
/// single box to describe all your transactions. In that case, complete and file **either Part I or
/// II**…"* The new expectation is derived from the fixture and the revision's grid, not from the
/// emitter's output.
#[test]
fn eleven_rows_per_page() {
    let cap = Form8949Map::ty2025().rows_per_page;
    let bytes = btctax_forms::fill_form_8949(&big_fixture(), 2025).unwrap();
    let doc = load(&bytes).unwrap();
    let (st_pages, lt_pages) = (BIG_ST.div_ceil(cap), BIG_LT.div_ceil(cap));
    assert_eq!(
        (st_pages, lt_pages),
        (3, 2),
        "premise: the fixture spans three Part I pages and two Part II pages on the {cap}-row grid"
    );
    assert_eq!(
        doc.get_pages().len(),
        st_pages + lt_pages,
        "|Part I pages| + |Part II pages| — never max(|ST|,|LT|) copies of BOTH parts (FR-218)"
    );

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

// ── ★★★ FR-218 — a part with NO rows is not a blank page; it is NO page ──────────────────────────
//
// Found by the OWNER, on paper, printing the S8b packet: *"the first page of 8949 is printed twice."*
// Pages 1 and 2 of that 4-page form had identical text layers. The cause was that the copy count was
// `max(|ST pages|, |LT pages|)` and each copy then filled BOTH parts, so a part with zero rows still
// emitted a page — once per copy, each carrying the filer's name and SSN.
//
// ★★ **Nothing this project owns could catch it, which is why the kill below is STRUCTURAL rather than
// visual.** The PDF is byte-stable so the golden passed; every filled field read back so the read-back
// verifier passed; both oracles agreed on every figure; and the two Part II pages' subtotals rolled up
// to Schedule D exactly. A test that looks at VALUES cannot see this defect at all — the blank page's
// cells are correctly blank. What distinguishes it is that the PAGE EXISTS, which is a fact about the
// page tree and the widget annotations, not about any value.
//
// The authority (`design/forms/extract/i8949--2024.txt:422-424`): *"You don't need to complete and file
// an entire copy of Form 8949 (Parts I and II) if you can check a single box to describe all your
// transactions. In that case, complete and file **either Part I or II** and check the box that
// describes the transactions."*

/// `n` rows in `part`: proceeds `100 × (i+1)`, basis 50.
fn legs(part: Form8949Part, n: usize) -> Vec<Form8949Row> {
    (0..n)
        .map(|i| {
            row(
                part,
                &format!("{part:?}-{i:03} BTC"),
                dec!(100) * Decimal::from(i + 1),
                dec!(50),
                false,
            )
        })
        .collect()
}

/// Every field name a `PartMap` claims, relative to each copy's ROOT component (which the merge renames
/// per copy) — the data grid, the line-2 totals row, and the part's box checkboxes.
///
/// ★ Derived from the map, never a hand-written list of FQNs: a revision that renames a cell or moves a
/// part to another page re-derives this, which is what keeps the page classifier below a measurement.
fn part_cells(p: &PartMap) -> Vec<String> {
    let below = |f: &str| {
        f.split_once('.')
            .map(|(_, r)| r.to_string())
            .unwrap_or_else(|| f.to_string())
    };
    let mut v: Vec<String> = p.rows.iter().flatten().map(|f| below(f)).collect();
    for f in [
        &p.totals.proceeds_d,
        &p.totals.cost_e,
        &p.totals.adj_g,
        &p.totals.gain_h,
    ] {
        v.push(below(f));
    }
    v.push(below(&p.box_field));
    for b in p.boxes.values() {
        v.push(below(&b.field));
    }
    v
}

/// Each page's `/Annots` widget ids, in document page order. `/Annots` may be an indirect reference to
/// the array; a version of this helper that skipped the deref would see every page as annotation-free —
/// and would then report an empty page set as agreement.
fn annots_per_page(doc: &lopdf::Document) -> Vec<Vec<lopdf::ObjectId>> {
    doc.get_pages()
        .into_values()
        .map(|pid| {
            doc.get_dictionary(pid)
                .ok()
                .and_then(|d| d.get(b"Annots").ok())
                .and_then(|o| doc.dereference(o).ok())
                .and_then(|(_, o)| o.as_array().ok())
                .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
                .unwrap_or_default()
        })
        .collect()
}

/// ★★★ Which PART each page of the emitted document belongs to, in page order — decided by whose mapped
/// widgets sit on that page, **not** by any value. A blank Part I page classifies as `"short"` exactly
/// as a populated one does, which is what makes this instrument able to see FR-218 at all.
///
/// A page carrying no part's widgets, or two parts', is an assertion failure rather than a silent
/// verdict: an instrument that cannot classify a page must say so instead of reporting agreement
/// (`design/HARNESS.md` class β).
fn part_of_each_page(doc: &lopdf::Document, fields: &[Field], map: &Form8949Map) -> Vec<String> {
    let by_part: Vec<(String, Vec<String>)> = map
        .parts
        .iter()
        .map(|p| (p.term.clone(), part_cells(p)))
        .collect();
    annots_per_page(doc)
        .iter()
        .enumerate()
        .map(|(i, annots)| {
            let mut hits: Vec<String> = by_part
                .iter()
                .filter(|(_, cells)| {
                    fields
                        .iter()
                        .any(|f| annots.contains(&f.id) && cells.iter().any(|c| f.fqn.ends_with(c)))
                })
                .map(|(term, _)| term.clone())
                .collect();
            assert_eq!(
                hits.len(),
                1,
                "page {i} must carry exactly ONE part's widgets — got {hits:?}. This classifier \
                 cannot report a verdict it did not measure."
            );
            hits.remove(0)
        })
        .collect()
}

/// ★★★ **FR-218 — B1, and the planted defect is the blank page itself.** The emitted page sequence must
/// be exactly `|ST pages|` Part I pages then `|LT pages|` Part II pages: **zero** Part I pages for a
/// long-term-only filer, **zero** Part II pages for a short-term-only filer, and both for a mixed one.
///
/// Both emitters are checked, because they are two independent implementations of the same rule
/// (`fill_form_8949` for the crypto slice, `fill_8949_full` for the packet) and a test on one holds
/// nothing about the other.
#[test]
fn a_part_with_no_rows_files_no_page_at_all() {
    let map = Form8949Map::ty2024();
    let cap = map.rows_per_page;
    // ★ The multi-page shapes are DERIVED from the revision's grid, so a revision that regrids keeps
    //   testing them instead of silently collapsing to one page per part.
    let cases: [(&str, usize, usize, &[&str]); 6] = [
        ("long-term only, one page", 0, 3, &["long"]),
        ("long-term only, two pages", 0, cap + 1, &["long", "long"]),
        ("short-term only, one page", 3, 0, &["short"]),
        (
            "short-term only, two pages",
            cap + 1,
            0,
            &["short", "short"],
        ),
        (
            "the owner's shape: one Part I page, two Part II pages",
            3,
            cap + 1,
            &["short", "long", "long"],
        ),
        (
            "mixed, two pages each",
            cap + 1,
            cap + 1,
            &["short", "short", "long", "long"],
        ),
    ];
    for (label, n_st, n_lt, expected) in cases {
        let mut rows = legs(Form8949Part::ShortTerm, n_st);
        rows.extend(legs(Form8949Part::LongTerm, n_lt));
        // The premise: the fixture really spans the page counts the expectation names.
        assert_eq!(
            (n_st.div_ceil(cap), n_lt.div_ceil(cap)),
            (
                expected.iter().filter(|t| **t == "short").count(),
                expected.iter().filter(|t| **t == "long").count()
            ),
            "{label}: premise — ⌈rows/{cap}⌉ per part must be the expected page set"
        );

        let slice = btctax_forms::fill_form_8949(&rows, 2024).expect("the slice 8949 fills");
        let printed =
            btctax_core::tax::printed::form_8949_printed(&rows).expect("the fixture has rows");
        let full = btctax_forms::fill_8949_full(
            &printed,
            &btctax_core::tax::testonly::kitchen_sink_header(),
            2024,
        )
        .expect("the full-return 8949 fills");
        for (path, pdf) in [("slice", &slice), ("full return", &full)] {
            let doc = load(pdf).unwrap();
            let fields = collect_fields(&doc).unwrap();
            assert_eq!(
                part_of_each_page(&doc, &fields, &map),
                expected,
                "{label} ({path}): the filed page sequence must be exactly the parts that have rows \
                 — i8949 says \"complete and file either Part I or II\""
            );
        }
    }
}

/// ★★★ **FR-218 — the full-return 8949 puts the filer's NAME AND SSN on filed pages only.**
///
/// The blank pages were not anonymous: the identity header is per-page, so a long-term-only filer's two
/// blank Part I pages each carried their name and SSN — a page of sworn testimony that asserts nothing,
/// filed twice. This counts the identity cells per part and requires the count to equal that part's
/// filed page count, so it reds both on a blank page that exists and on a filed page left unnamed.
#[test]
fn the_full_return_8949_names_only_the_pages_it_files() {
    let map = Form8949Map::ty2024();
    let cap = map.rows_per_page;
    let rows = legs(Form8949Part::LongTerm, cap + 1);
    let printed = btctax_core::tax::printed::form_8949_printed(&rows).expect("there are rows");
    let pdf = btctax_forms::fill_8949_full(
        &printed,
        &btctax_core::tax::testonly::kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let pages = part_of_each_page(&doc, &fields, &map);
    assert_eq!(
        pages,
        vec!["long", "long"],
        "premise: two Part II pages only"
    );

    let below = |f: &str| f.split_once('.').map(|(_, r)| r.to_string()).unwrap();
    for (term, cells) in [
        (
            "short",
            map.identity_page1.as_ref().expect("page 1 identity"),
        ),
        (
            "long",
            map.identity_page2.as_ref().expect("page 2 identity"),
        ),
    ] {
        let filed = pages.iter().filter(|t| *t == term).count();
        for (what, fqn) in [("name", &cells.name), ("SSN", &cells.ssn)] {
            let found = values_ending(&doc, &fields, &below(fqn)).len();
            assert_eq!(
                found, filed,
                "the {term} part files {filed} page(s) but carries {found} {what} cell(s) — a page \
                 the filer does not file must not carry their identity (FR-218)"
            );
        }
    }
}

/// ★★★ **FR-218 must not disturb the SCHEDULE D ROLL-UP.** The form's line 2 says *"Enter each total
/// here"*, so every filed page totals only its own rows; Schedule D lines 3 and 10 cite *"Totals for
/// all transactions reported on **Form(s) 8949**"*, plural, and therefore the SUM of those per-page
/// totals. The owner's own packet was the worked example — two Part II pages, controller-verified:
/// 352655 + 468635 = 821290 (d), 86361 + 102520 = 188881 (e), 266294 + 366115 = 632409 (h). Page
/// surgery is exactly the kind of change that could break that roll-up silently, because a dropped page
/// has no rows and so contributes 0 to every column.
///
/// Asserted on columns (d), (e) and (h) of BOTH parts, over a fixture whose every part spans two pages,
/// so the sum is over more than one page in every case.
#[test]
fn every_filed_pages_line2_totals_sum_to_the_parts_schedule_d_total() {
    let map = Form8949Map::ty2024();
    let cap = map.rows_per_page;
    let mut rows = legs(Form8949Part::ShortTerm, cap + 2);
    rows.extend(legs(Form8949Part::LongTerm, cap + 3));
    let printed = btctax_core::tax::printed::form_8949_printed(&rows).expect("there are rows");
    let pdf = btctax_forms::fill_8949_full(
        &printed,
        &btctax_core::tax::testonly::kitchen_sink_header(),
        2024,
    )
    .unwrap();
    let doc = load(&pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    // The premise: every part really does span more than one page, or a "sum" over one page proves
    // nothing about the roll-up.
    assert_eq!(
        part_of_each_page(&doc, &fields, &map),
        vec!["short", "short", "long", "long"],
        "premise: two Part I pages and two Part II pages"
    );

    let below = |f: &str| f.split_once('.').map(|(_, r)| r.to_string()).unwrap();
    let sum_cells = |fqn: &str| -> i64 {
        let vals = values_ending(&doc, &fields, &below(fqn));
        assert!(
            vals.len() >= 2,
            "{fqn}: expected one line-2 total per filed page, got {vals:?}"
        );
        vals.iter()
            .map(|v| {
                v.parse::<i64>()
                    .unwrap_or_else(|e| panic!("{fqn} = {v:?}: {e}"))
            })
            .sum()
    };
    for (term, part, totals) in [
        ("short", map.part("short").unwrap(), printed.st_totals),
        ("long", map.part("long").unwrap(), printed.lt_totals),
    ] {
        for (col, fqn, expected) in [
            ("(d) proceeds", &part.totals.proceeds_d, totals.proceeds_d),
            ("(e) cost", &part.totals.cost_e, totals.cost_e),
            ("(h) gain", &part.totals.gain_h, totals.gain_h),
        ] {
            assert_eq!(
                Decimal::from(sum_cells(fqn)),
                expected,
                "{term} {col}: Σ the filed pages' line-2 totals must equal the figure Schedule D \
                 cites for that part"
            );
        }
    }
}
