//! ★★★ **FR-200(b) — Form 8283 Section A: conformance against the form's own text layer, and the
//! overflow that must never lose a gift.**
//!
//! Two things live here that cannot live in `btctax-core`:
//!
//! 1. **The column census is MEASURED, not claimed.** `design/forms/extract/` is outside the published
//!    `btctax-core` crate — an `include_str!` reaching out of a crate root ships a tarball that builds
//!    in the workspace and is broken for everyone else, *with exit 0*. So the table lives in core as
//!    data ([`btctax_core::tax::form8283_section_a::SECTION_A_COLUMNS`]) and the checking lives here,
//!    where a test may read the repo tree. Same split as `capital_loss_carryover`'s.
//!
//! 2. **The FIFTH GIFT.** Section A prints four rows (A–D) and says *"If you need more space, attach a
//!    statement"*. btctax overflows onto additional copies of the form instead — *"Attach one or more
//!    Forms 8283"* sanctions it — and the property this file holds is that all five gifts reach paper.
//!
//! Every assertion below was observed RED on a planted defect before it was trusted (B1); each plant is
//! named in the test's own doc comment, and every plant is a mutation of the TRANSCRIPTION or the
//! EMITTER rather than of the checker (FR-235).

use btctax_core::forms::{Form8283HowAcquired, Form8283Row, Form8283Section};
use btctax_core::tax::form8283_section_a::{
    section_a_row, DateAcquiredByDonor, NoncashGiftProperty, NoncashPropertyKind,
    SectionAColumnsEfg, SectionARow, SECTION_A_COLUMNS,
};
use btctax_core::tax::printed::form_8283_printed;
use btctax_core::tax::testonly::kitchen_sink_header;
use btctax_forms::testonly::{collect_fields, load, text_value};
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use time::macros::date;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/btctax-forms -> workspace root")
        .to_path_buf()
}

fn form_extract(year: i32) -> String {
    let p = workspace_root()
        .join("design/forms/extract")
        .join(format!("f8283--{year}.txt"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
//  THE DERIVATION — Section A's columns, read off the text layer
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// **Every `(letter, heading)` Section A prints, derived from `pdftotext -layout` output.**
///
/// A heading wraps over several physical lines, and `-layout` keeps each column inside its own
/// character range — so a heading is reassembled by slicing every following line to the SAME x-span
/// until that span goes blank. Nothing here is hand-listed: the block is delimited by the form's own
/// `Section A.` / `Section B.` headings, and a header line is any line carrying two or more `(x) `
/// markers immediately followed by a capital letter.
///
/// ★ That last rule is what separates a header line from the form's `Note:` sentence, which also spells
/// `(e), (f), and (g)` — there the markers are followed by a comma or a period, never by a capital.
/// Stating the rule is the point: a hand-list of "the two header lines" would break on a revision that
/// re-wrapped them.
fn derived_columns(text: &str) -> BTreeMap<char, String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("Section A."))
        .expect("the extract prints a `Section A.` heading");
    let end = lines
        .iter()
        .position(|l| l.starts_with("Section B."))
        .expect("the extract prints a `Section B.` heading");
    assert!(start < end, "Section A precedes Section B");
    let block = &lines[start..end];

    /// ★★ Positions are CHAR indices, not byte offsets, and that is load-bearing: the (g) heading
    ///    contains `Donor’s` — U+2019, three bytes — so a byte offset taken from a marker and then fed
    ///    to `chars().skip(..)` drifts two characters per apostrophe and splices the (h) heading into
    ///    the (g) column. Measured: it produced `"(g) Donor’s cost        (h or adjusted basis"`.
    fn markers(l: &str) -> Vec<(usize, char)> {
        let cs: Vec<char> = l.chars().collect();
        let mut out = Vec::new();
        for i in 0..cs.len() {
            // `(x) Y` where x is a..=i and Y is an upper-case letter.
            if cs[i] == '('
                && i + 4 < cs.len()
                && ('a'..='i').contains(&cs[i + 1])
                && cs[i + 2] == ')'
                && cs[i + 3] == ' '
                && cs[i + 4].is_ascii_uppercase()
            {
                out.push((i, cs[i + 1]));
            }
        }
        out
    }

    let mut out: BTreeMap<char, String> = BTreeMap::new();
    for (i, line) in block.iter().enumerate() {
        let hits = markers(line);
        if hits.len() < 2 {
            continue;
        }
        for (k, (x, letter)) in hits.iter().enumerate() {
            let next = hits.get(k + 1).map_or(usize::MAX, |(nx, _)| *nx);
            let mut parts: Vec<String> = Vec::new();
            for (j, l) in block.iter().enumerate().skip(i) {
                let seg: String = l
                    .chars()
                    .skip(*x)
                    .take(next.saturating_sub(*x))
                    .collect::<String>()
                    .trim()
                    .to_string();
                if j > i && seg.is_empty() {
                    break;
                }
                if !seg.is_empty() {
                    parts.push(seg);
                }
            }
            let prev = out.insert(*letter, parts.join(" "));
            assert!(
                prev.is_none(),
                "column ({letter}) appears in two header lines — the derivation's marker rule has \
                 stopped separating header lines from prose"
            );
        }
    }
    out
}

/// Collapse runs of whitespace — `pdftotext -layout` pads columns, and a heading reassembled across
/// wrapped lines carries the joins.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// ★★★ **THE CONFORMANCE CHECK, in both directions.** For every supported year:
///
/// * every column the FORM prints is in [`SECTION_A_COLUMNS`] — a column added by a revision reds;
/// * every column in the table is on the form — a table entry for a column that no longer exists reds;
/// * each letter's heading matches the form's, VERBATIM modulo whitespace — a heading swapped between
///   two letters reds, which is the Form 6251 line-33 class.
///
/// **Planted-defect check (B1), all four observed RED before this was trusted** — and each plant is a
/// mutation of the TRANSCRIPTION, never of the checker:
///
/// | plant in `SECTION_A_COLUMNS` | what reds |
/// |---|---|
/// | swap the `heading` strings of `'e'` and `'f'` | *"column (e): the table says … and the form prints …"* |
/// | delete the `'b'` entry (the vehicle column) | *"the FORM prints Section A column(s) the table does not account for: ['b']"* |
/// | add an `'x'` entry | *"the table names Section A column(s) the form does not print: ['x']"* |
/// | shorten `(g) Donor's cost or adjusted basis` to `(g) Donor's cost` | the heading mismatch for `'g'` |
#[test]
fn every_section_a_column_of_the_form_is_accounted_for() {
    let claimed: BTreeMap<char, &str> = SECTION_A_COLUMNS
        .iter()
        .map(|c| (c.letter, c.heading))
        .collect();
    assert_eq!(
        claimed.len(),
        SECTION_A_COLUMNS.len(),
        "two entries claim the same column letter"
    );

    for year in btctax_forms::SUPPORTED_YEARS {
        let derived = derived_columns(&form_extract(*year));
        assert!(
            derived.len() >= 9,
            "TY{year}: the derivation found only {} Section A columns — it has stopped reading the \
             form, which would make every assertion below vacuous",
            derived.len()
        );

        let on_form: BTreeSet<char> = derived.keys().copied().collect();
        let in_table: BTreeSet<char> = claimed.keys().copied().collect();
        let missing: Vec<char> = on_form.difference(&in_table).copied().collect();
        assert!(
            missing.is_empty(),
            "TY{year}: the FORM prints Section A column(s) the table does not account for: \
             {missing:?}. Every column needs a determinate provenance — collected, carve-out, or a \
             named refusal."
        );
        let phantom: Vec<char> = in_table.difference(&on_form).copied().collect();
        assert!(
            phantom.is_empty(),
            "TY{year}: the table names Section A column(s) the form does not print: {phantom:?}"
        );

        for (letter, form_heading) in &derived {
            let want = claimed[letter];
            assert_eq!(
                squash(want),
                squash(form_heading),
                "TY{year} column ({letter}): the table says {want:?} and the form prints \
                 {form_heading:?}. A real sentence attached to the WRONG column is the Form 6251 \
                 line-33 class."
            );
        }
    }
}

/// ★★ **The Section A column set is IDENTICAL on every supported revision** — asserted, because
/// `form8283_section_a`'s module doc says so and one transcription serves both. If a future revision
/// re-letters a column, this reds before the test above has to guess which year is authoritative.
#[test]
fn the_section_a_column_set_is_identical_on_every_supported_revision() {
    let mut seen: Option<(i32, BTreeMap<char, String>)> = None;
    for year in btctax_forms::SUPPORTED_YEARS {
        let d = derived_columns(&form_extract(*year));
        match &seen {
            None => seen = Some((*year, d)),
            Some((first_year, first)) => assert_eq!(
                &d, first,
                "TY{year}'s Section A columns differ from TY{first_year}'s — one transcription can no \
                 longer serve both revisions, and `SECTION_A_COLUMNS` needs a per-revision split"
            ),
        }
    }
    assert!(seen.is_some(), "SUPPORTED_YEARS is empty");
}

/// ★★★ **THE FOUR-ROW CAPACITY IS DERIVED FROM THE FORM, never typed.** Section A's row labels are
/// `A`, `B`, `C`, `D` — printed twice, once on the (a)-(c) table and once on the (d)-(i) table — so the
/// capacity is the number of DISTINCT labels, and the emitter's is the number of rows the year's map
/// enumerates. The two must agree, or the overflow paginates against the wrong denominator.
///
/// **Planted-defect check (B1), observed RED:** delete one `{ donee = …, … }` entry from
/// `[section_a].rows` in `crates/btctax-forms/forms/2024/f8283.map.toml` and this reds with *"the FORM
/// prints 4 Section A row(s) … but the map enumerates 3"*.
#[test]
fn the_section_a_row_capacity_matches_the_forms_own_row_labels() {
    for year in btctax_forms::SUPPORTED_YEARS {
        let text = form_extract(*year);
        let lines: Vec<&str> = text.lines().collect();
        let start = lines
            .iter()
            .position(|l| l.starts_with("Section A."))
            .expect("Section A heading");
        let end = lines
            .iter()
            .position(|l| l.starts_with("Section B."))
            .expect("Section B heading");
        // A row label is a line of nothing but leading whitespace and one capital letter.
        let labels: BTreeSet<char> = lines[start..end]
            .iter()
            .filter_map(|l| {
                let t = l.trim();
                let mut cs = t.chars();
                match (cs.next(), cs.next()) {
                    (Some(c), None) if c.is_ascii_uppercase() && l.starts_with("  ") => Some(c),
                    _ => None,
                }
            })
            .collect();
        assert_eq!(
            labels,
            "ABCD".chars().collect::<BTreeSet<char>>(),
            "TY{year}: the Section A row labels read off the form are {labels:?}"
        );

        let map = btctax_forms::testonly::Form8283Map::for_year(*year).expect("a bundled 8283 map");
        assert_eq!(
            labels.len(),
            map.section_a.rows.len(),
            "TY{year}: the FORM prints {} Section A row(s) ({labels:?}) but the map enumerates {} — \
             the overflow paginates against the map, so a disagreement silently drops or invents a row",
            labels.len(),
            map.section_a.rows.len()
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
//  THE FIFTH GIFT
// ══════════════════════════════════════════════════════════════════════════════════════════════════

fn bag(n: usize) -> NoncashGiftProperty {
    NoncashGiftProperty {
        kind: NoncashPropertyKind::ClothingOrHouseholdItem {
            good_used_condition_or_better: true,
        },
        donee_name_and_address: format!("Goodwill {n}, 1 Main St, Springfield IL"),
        description_and_condition: format!("BAGMARKER{n} used adult clothing, good used condition"),
        date_of_contribution: date!(2024 - 11 - 30),
        date_acquired_by_donor: Some(DateAcquiredByDonor::Various),
        how_acquired_by_donor: Some("Purchase".into()),
        cost_or_adjusted_basis: Some(dec!(2400)),
        fair_market_value: dec!(600),
        method_used_to_determine_fmv: "Thrift shop value".into(),
    }
}

fn row_of(p: &NoncashGiftProperty) -> SectionARow {
    section_a_row(p.fair_market_value, Some(p))
        .expect("a $600 bag in good used condition is a Section A row")
}

/// Every text value written into the emitted PDF's fields, across every page of every merged copy.
fn filled_values(bytes: &[u8]) -> Vec<String> {
    let doc = load(bytes).expect("the emitted PDF loads");
    let fields = collect_fields(&doc).expect("the AcroForm is readable");
    fields
        .iter()
        .filter_map(|f| text_value(&doc, f.id))
        .collect()
}

fn fill(printed: &btctax_core::tax::printed::Printed8283Rows) -> Vec<u8> {
    btctax_forms::fill_form_8283_full(printed, &kitchen_sink_header(), 2024)
        .expect("the fill succeeds")
        .expect("rows are not nothing")
}

/// ★★★ **THE ONE THAT MATTERS MOST: a FIFTH gift must not disappear.**
///
/// Five $600 bags are five Section A rows and the form prints four per copy, so the emitter must
/// produce a SECOND physical copy carrying the fifth. The assertion is over the emitted PDF's own
/// fields, not over a row count in memory — a count is exactly what a dropped row would still satisfy.
///
/// **Planted-defect check (B1), observed RED:** in `form8283.rs`, change
/// `let n_copies = section_a.len().div_ceil(cap_a);` to `let n_copies = 1;` — the plausible "one page
/// is enough" edit — and this reds with *"gift 5 (BAGMARKER5) reached no page"*. Chunking with
/// `.take(cap_a)` over the whole list reds identically.
#[test]
fn a_fifth_noncash_gift_reaches_a_second_copy_and_is_not_dropped() {
    let props: Vec<NoncashGiftProperty> = (1..=5).map(bag).collect();
    let rows: Vec<SectionARow> = props.iter().map(row_of).collect();
    let printed = form_8283_printed(&[], Some(false), &rows)
        .expect("five noncash Section A rows produce a Form 8283 even with NO ledger donation");
    assert_eq!(
        printed.section_a_noncash().len(),
        5,
        "premise: all five rows reach the printed packet"
    );
    assert!(
        printed.rows().is_empty(),
        "premise: not a satoshi was donated — this is the FR-200(b) shape"
    );

    let values = filled_values(&fill(&printed));
    for n in 1..=5 {
        let marker = format!("BAGMARKER{n}");
        assert!(
            values.iter().any(|v| v.contains(&marker)),
            "gift {n} ({marker}) reached no page of the emitted Form 8283. Section A prints four rows \
             per copy; the fifth must continue onto another copy, never vanish. Values on paper: \
             {values:?}"
        );
    }
}

/// ★★ …and the fifth gift's own columns travel with it. A row that reached paper carrying a
/// NEIGHBOUR's donee is a different defect from a dropped row, and a marker-presence check cannot see
/// it.
#[test]
fn the_fifth_gifts_own_columns_travel_with_it() {
    let props: Vec<NoncashGiftProperty> = (1..=5).map(bag).collect();
    let rows: Vec<SectionARow> = props.iter().map(row_of).collect();
    let printed = form_8283_printed(&[], Some(false), &rows).expect("a Form 8283");
    let values = filled_values(&fill(&printed));
    assert!(
        values.iter().any(|v| v.contains("Goodwill 5")),
        "the fifth row's column (a) must be the FIFTH donee, not a neighbour's: {values:?}"
    );
    // Column (e) is the form's own word for a group of similar items acquired on various dates.
    assert!(
        values.iter().any(|v| v == "Various"),
        "column (e) prints i8283's own word: {values:?}"
    );
    assert!(
        values.iter().any(|v| v == "11/30/2024"),
        "column (d) prints the contribution date: {values:?}"
    );
}

/// ★★★ **THE CARVE-OUT ON PAPER: an omitted (e)(f)(g) writes NOTHING, not a zero.**
///
/// A $200 bag needs no columns (e), (f) or (g) — and a `0` in column (g) would be testimony the filer
/// never gave, about a basis the form did not ask for, on a substantiation form signed under §6065. The
/// distinction is invisible in a row count and invisible to both oracles; only the emitted page shows it.
///
/// **Planted-defect check (B1), observed RED:** replace the
/// `SectionAColumnsEfg::NotRequiredDeductionAtOrUnderFiveHundred => {}` arm in `form8283.rs` with one
/// that pushes `Usd::ZERO` into column (g) — the shape a "just print a zero" edit takes — and this reds
/// with *"column (g) printed a zero for an item the form does not ask it of"*.
#[test]
fn an_omitted_column_efg_writes_nothing_rather_than_a_zero() {
    let small = NoncashGiftProperty {
        date_acquired_by_donor: None,
        how_acquired_by_donor: None,
        cost_or_adjusted_basis: None,
        fair_market_value: dec!(200),
        ..bag(1)
    };
    let row = section_a_row(dec!(200), Some(&small)).expect("a $200 bag files");
    assert_eq!(
        row.cols_efg,
        SectionAColumnsEfg::NotRequiredDeductionAtOrUnderFiveHundred,
        "premise: the carve-out applies"
    );

    let printed = form_8283_printed(&[], Some(false), &[row]).expect("a Form 8283");
    let values = filled_values(&fill(&printed));

    assert!(
        values.iter().any(|v| v.contains("BAGMARKER1")),
        "premise: the row itself printed"
    );
    assert!(
        values.iter().any(|v| v == "200"),
        "column (h) fair market value is REQUIRED and printed: {values:?}"
    );
    assert!(
        !values.iter().any(|v| v == "0" || v == "-0-"),
        "column (g) printed a zero for an item the form does not ask it of. \"you do not have to \
         complete columns (e), (f), and (g)\" is not \"the basis is zero\", and a hardcoded zero is \
         indistinguishable from a computed one on the page. Values: {values:?}"
    );
    assert!(
        !values.iter().any(|v| v == "Various" || v == "Purchase"),
        "columns (e) and (f) must be blank too, not only (g): {values:?}"
    );
}

/// A $600 bag DOES print all three, so the test above is not passing because nothing ever writes them.
#[test]
fn a_completed_column_efg_does_print_all_three() {
    let row = row_of(&bag(1));
    let printed = form_8283_printed(&[], Some(false), &[row]).expect("a Form 8283");
    let values = filled_values(&fill(&printed));
    assert!(values.iter().any(|v| v == "Various"), "(e): {values:?}");
    assert!(values.iter().any(|v| v == "Purchase"), "(f): {values:?}");
    assert!(values.iter().any(|v| v == "2400"), "(g): {values:?}");
}

/// ★★★ **A MIXED YEAR NEEDS BOTH SECTIONS.** A year with over $5,000 of donated bitcoin (Section B)
/// and a $600 bag of clothes (Section A) must print the clothes in SECTION A — not into Section B's
/// property table, under a qualified-appraisal declaration nobody made.
///
/// **Planted-defect check (B1) — and the FIRST plant was blind, which is recorded here rather than
/// quietly replaced.**
///
/// The plant that was tried first restored the old single-section read (`let section = rows.iter()
/// .find_map(|r| r.section).unwrap_or(A)`, one `match section`) **while leaving the separate Section A
/// row list in place** — and all nine tests in this file stayed green, because the noncash rows never
/// travelled through the ledger's partition in that version. It was not a kill.
///
/// The plant that DOES red is the one that reproduces the old code's actual consequence: make the
/// noncash rows conditional on the ledger's single section, i.e. wrap
/// `section_a.extend(noncash_section_a…)` in `if ledger_b.is_empty()`. Observed RED with
/// *"the Section A gift's description must be on paper: [… BTCMARKER 1.00000000 BTC …]"* — the Section B
/// crypto leg printed and the $600 bag of clothes printed nowhere at all.
///
/// ★ **What that measurement also says about `partition_by_section`**: no input `form_8283()` can
/// produce exercises its MIXED-ledger branch, because the ledger's section is uniform across a year.
/// The branch is defensive, and stated as such at its definition. What this test kills is the property
/// that matters today — the noncash Section A list is independent of whatever section the ledger is in.
#[test]
fn a_section_b_crypto_year_with_a_noncash_gift_prints_both_sections() {
    let leg = Form8283Row {
        section: Some(Form8283Section::B),
        description: "BTCMARKER 1.00000000 BTC".into(),
        how_acquired: Form8283HowAcquired::Purchased,
        date_acquired: date!(2021 - 03 - 01),
        date_contributed: date!(2024 - 07 - 04),
        cost_basis: dec!(1200),
        fmv: dec!(60000),
        claimed_deduction: Some(dec!(60000)),
        fmv_method: "qualified appraisal".into(),
        donee: "Habitat".into(),
        appraiser: "A. Praiser".into(),
        needs_review: false,
        details: None,
    };
    let row = row_of(&bag(1));
    let printed = form_8283_printed(std::slice::from_ref(&leg), Some(false), &[row])
        .expect("one Section B donation and one Section A gift");
    let values = filled_values(&fill(&printed));
    assert!(
        values.iter().any(|v| v.contains("BAGMARKER1")),
        "the Section A gift's description must be on paper: {values:?}"
    );
    assert!(
        values.iter().any(|v| v.contains("Goodwill 1")),
        "…in column (a) of SECTION A, which Section B has no column for — Section B's donee is a Part \
         V identity block, so a clothes row swept into Section B loses its donee entirely: {values:?}"
    );
    assert!(
        values.iter().any(|v| v.contains("BTCMARKER")),
        "…and the Section B crypto donation is still on paper: {values:?}"
    );
}

/// A year with no donation of any kind files no Form 8283 — the `None` arm, so the test above is not
/// green merely because something always emits bytes.
#[test]
fn no_donations_and_no_noncash_gifts_file_no_form_8283() {
    assert!(
        form_8283_printed(&[], Some(false), &[]).is_none(),
        "nothing donated, nothing attached"
    );
}
