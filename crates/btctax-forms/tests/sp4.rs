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
                // FR-29: a promoted-basis leg is contrary to no NAMED rule, so (a)/(b) stay blank.
                rule: None,
                item_name: None,
                form: "8949".into(),
                line: "Part I \u{2014} column (e)".into(),
                description:
                    "basis estimated at the minimum daily closing price over the attested \
                    acquisition window (Cohan; the bearing-heavily minimum)"
                        .into(),
                amount: dec!(12345),
            },
            Part1Item {
                // FR-29: a promoted-basis leg is contrary to no NAMED rule, so (a)/(b) stay blank.
                rule: None,
                item_name: None,
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
/// Part II's numbered lines 2-6 — round 2 finding 4: NEVER written (the bundled PDF's XFA numbers
/// these beside Part I's rows 1-6; see `form8275.rs`'s module doc). Used only by the negative tests
/// below that pin they stay blank.
const PART_II_LINES_2_THROUGH_6: [&str; 5] = [
    "topmostSubform[0].Page1[0].p1-t81[0]",
    "topmostSubform[0].Page1[0].p1-t82[0]",
    "topmostSubform[0].Page1[0].p1-t83[0]",
    "topmostSubform[0].Page1[0].p1-t84[0]",
    "topmostSubform[0].Page1[0].p1-t85[0]",
];
const PART_IV_LINE1: &str = "topmostSubform[0].Page2[0].p2-t1[0]";
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

    // Part II narrative (201 characters, 782.5pt at 8pt Helvetica-Bold) does not fit Part II's own
    // line 1 (514.4pt usable, inset-adjusted) alone — it WRAPS, spilling to Part IV's line 1
    // (`PART_IV_LINE1`, NOT Part II's own line 2: round 2 finding 4 — Part II's numbered lines 2-6 are
    // never written, since the XFA numbers them beside Part I's rows). The Part IV line carries the
    // IRS-required cross-reference prefix. The exact split is a known-answer pin of `crate::wrap`'s
    // greedy word-wrap (T-f8275-part-ii-overflow).
    assert_eq!(
        tv(&doc, &fields, PART_II_LINE1).as_deref(),
        Some(
            "The taxpayer disposed of BTC acquired via an unverified peer-to-peer purchase; basis \
             was estimated using the daily low close over"
        )
    );
    assert_eq!(
        tv(&doc, &fields, PART_IV_LINE1).as_deref(),
        Some(
            "Part II, line 1 (continued): the attested acquisition window, consistent with Cohan v. \
             Commissioner."
        )
    );
    // Part II's numbered lines 2-6 stay BLANK — the narrative never lands there.
    for fqn in PART_II_LINES_2_THROUGH_6 {
        assert_eq!(
            tv(&doc, &fields, fqn),
            None,
            "{fqn}: Part II's numbered continuation lines must never be written (finding 4)"
        );
    }
    // No character lost: rejoining Part II line 1 + Part IV line 1 (cross-reference prefix stripped)
    // reproduces the full narrative.
    let part_iv_1 = tv(&doc, &fields, PART_IV_LINE1).unwrap();
    let part_iv_1_text = part_iv_1
        .strip_prefix("Part II, line 1 (continued): ")
        .expect("the first Part IV line used must carry the cross-reference prefix");
    let rejoined = format!(
        "{} {}",
        tv(&doc, &fields, PART_II_LINE1).unwrap(),
        part_iv_1_text
    );
    assert_eq!(rejoined, printed.part_ii);

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
                // FR-29: a promoted-basis leg is contrary to no NAMED rule, so (a)/(b) stay blank.
                rule: None,
                item_name: None,
                form: "R1FORM".into(),
                line: "R1ITEM".into(),
                description: "R1DESC".into(),
                amount: dec!(11111),
            },
            Part1Item {
                // FR-29: a promoted-basis leg is contrary to no NAMED rule, so (a)/(b) stay blank.
                rule: None,
                item_name: None,
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

/// ★ T-f8275-part-ii-overflow Step 3 (round 2 capacity: Part II's own line 1 + Part IV's 27 lines =
/// 28, not the round-1 33 — Part II's numbered lines 2-6 are no longer part of the wrap capacity, see
/// finding 4): a Part II narrative too long to wrap across all 28 available lines FAILS CLOSED with a
/// named [`FormsError::Overflow`] — mirroring the shipped Part I >6-row refusal — rather than clipping
/// past the last line. 900 repeats of "word " need 37 lines at 8pt Helvetica-Bold in these fields'
/// (inset-adjusted) widths; 28 are available.
#[test]
fn form_8275_part_ii_narrative_too_long_for_every_continuation_line_fails_closed() {
    let printed = Printed8275 {
        part_i: sample_printed().part_i,
        part_ii: "word ".repeat(900),
    };
    let err = btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024).unwrap_err();
    assert!(
        matches!(&err, FormsError::Overflow { part, rows, capacity }
            if *part == "Part II" && *capacity == 28 && *rows > 28),
        "expected a Part II Overflow with capacity 28 and rows > 28, got {err:?}"
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
const GOLDEN_8275_SHA256: &str = "c0b6fe3c12ed1aef74a9f5ee8c4f205a3546d508962b5900722d936135dae4c7";

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

/// ★ T-f8275-part-ii-overflow Step 2: pins the exact shape of the newly-mapped continuation lines —
/// 5 Part II lines (`p1-t81`..`p1-t85`) + 27 Part IV lines (`p2-t1`..`p2-t27`), 32 in total, plus the
/// pre-existing `part_ii_narrative` = 33 continuation fields overall. A change to either count is a
/// revision change to the bundled PDF and must be deliberate, not a silent drift.
#[test]
fn form_8275_map_carries_the_32_new_continuation_fields() {
    let map = Form8275Map::ty2024();
    assert_eq!(
        map.part_ii_continuation.len(),
        5,
        "Part II has 6 lines total: part_ii_narrative (1) + part_ii_continuation (5)"
    );
    assert_eq!(
        map.part_iv_continuation.len(),
        27,
        "Part IV (page 2) has 27 continuation lines"
    );
    assert_eq!(
        map.narrative_continuation_fields().len(),
        33,
        "1 (part_ii_narrative) + 5 (part_ii_continuation) + 27 (part_iv_continuation) = 33"
    );
    // Exact identity + order — every field named in `all_33_declared_continuation_fields()` (this test
    // file's own independent oracle list, Step 1) must appear, in the SAME order. This is the map's
    // DECLARED shape (all 33 continuation fields the bundled PDF carries) — NOT what the fill actually
    // WRITES today (only 28 of them; see `written_continuation_fields_in_order()` / finding 4).
    let got: Vec<String> = map
        .narrative_continuation_fields()
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(got, all_33_declared_continuation_fields());
}

/// The names `map` targets that do NOT exist in `pdf`'s AcroForm — the whole content of the
/// map-vs-PDF check below, factored out so the SAME comparison can be watched failing on a document
/// it should reject (`the_fieldset_check_can_fail_…`). A check nobody has seen discriminate is not a
/// check (B1).
fn map_fields_absent_from(pdf: &[u8], map: &Form8275Map) -> Vec<String> {
    let set = fieldset(pdf);
    map.field_names()
        .into_iter()
        .filter(|n| !set.contains(*n))
        .map(str::to_string)
        .collect()
}

#[test]
fn map_year_matches_bundled_pdf_fieldset_for_every_supported_year() {
    // ★ Each year against THAT YEAR'S OWN asset (`Form8275Map::bundled_pdf`), never against the 2024
    // literal. Form 8275 aliases one Rev. 10-2024 document to every supported year TODAY, so the two
    // spellings agree today — and they stop agreeing the moment a year bundles a different revision,
    // which is precisely the case this test exists to see. Written against `F8275_PDF_2024` it could
    // not: it read as year-general and was a 2024 re-check wearing a loop.
    let mut checked = Vec::new();
    for &year in btctax_forms::SUPPORTED_YEARS {
        let map = Form8275Map::for_year(year).unwrap();
        assert_eq!(map.year, year);
        let pdf = Form8275Map::bundled_pdf(year).unwrap_or_else(|e| {
            panic!("TY{year} is a SUPPORTED_YEAR with no bundled Form 8275: {e}")
        });
        let absent = map_fields_absent_from(pdf, &map);
        assert!(
            absent.is_empty(),
            "year {year}: 8275 map fields absent from TY{year}'s own bundled PDF: {absent:?}"
        );
        checked.push(year);
    }
    // Skipping is not passing: the loop must have covered every supported year, not zero of them.
    assert_eq!(
        checked,
        btctax_forms::SUPPORTED_YEARS.to_vec(),
        "the per-year fieldset check did not run for every SUPPORTED_YEAR"
    );
}

/// B1 kill-test for the check above: the same comparison, pointed at a *different* bundled IRS form,
/// must REPORT the mismatch. Without this the loop could be structurally incapable of failing (its
/// previous shape, against one hard-coded asset, effectively was).
#[test]
fn the_fieldset_check_can_fail_when_the_pdf_is_not_this_form() {
    let map = Form8275Map::ty2024();
    let absent = map_fields_absent_from(F8283_PDF_2024, &map);
    assert!(
        !absent.is_empty(),
        "the map-vs-PDF check found every Form 8275 field name inside Form 8283's AcroForm — it is \
         not comparing anything"
    );
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

// ── T-f8275-part-ii-overflow — Part II narrative overflow ──────────────────────────────────────────
//
// `crates/btctax-forms/src/form8275.rs` used to write the filer's ENTIRE Part II narrative into the
// ONE single-line 8pt `p1-t80[0]` field. That field has no `/MaxLen` (nothing at the PDF-data level
// truncates an over-long `/V`), so the write always "succeeds" — but the widget is a fixed-width,
// non-multiline, `DoNotScroll` box, so a viewer honoring its own geometry can only DISPLAY the portion
// that fits (about the first 137 characters at 8pt Helvetica-Bold in this field's 518.4pt width). The
// rest is silently invisible on the printed page: no error, no refusal, no truncation marker. This is
// the §1.6662-4(f) adequate-disclosure text — a fifth of a disclosure is not a disclosure.
//
// `verify_flat` cannot see this: this field carries no `/MaxLen`, and free placements skip the
// column/row geometry leg entirely (`crate::verify`'s doc comment). So the test below re-measures,
// independently of anything `btctax-forms` itself computes, whether what actually landed on each
// continuation field's own line would fit inside that field's own widget box at the SAME font the PDF
// itself declares (`/DA` = `/HelveticaLTStd-Bold 8.00 Tf`, confirmed against the bundled asset — see
// BUILD-REPORT.md). This is deliberately NOT `btctax_forms::wrap`'s own measurement — an independent
// second oracle, mirroring this crate's whole `verify.rs` ethos ("the map is what we distrust; the
// PDF's geometry is the oracle").
//
// ★ Round 2 (whole-branch two-lens review): Part II now writes ONLY its own line 1 — everything past
// that spills to Part IV with an IRS-required cross-reference prefix (finding 4/1) — and the width
// budget accounts for the renderer's text inset with a TIGHTENED, negative-allowance check (finding 3).

/// Helvetica-Bold glyph widths, ASCII printable range 0x20..=0x7E (`code - 0x20` indexed) —
/// independently re-extracted from the bundled `f8275.pdf`'s own embedded font
/// (`/DR/Font/HelveticaLTStd-Bold`, `/WinAnsiEncoding`, `/FirstChar 0`/`/LastChar 255`, full 256-entry
/// `/Widths`), the SAME authoritative source `crate::wrap`'s production table uses — see that module's
/// doc comment and `design/f8275-part-ii-overflow/BUILD-REPORT.md` for the extraction. This table is a
/// SEPARATE Rust array (not `use`d from `wrap.rs`) so a bug in the WRAPPING algorithm — off-by-one line
/// accounting, a dropped `EPS`, wrong budget math — has an independent check to fail against; the
/// numeric source (the asset's own font) is a published fact neither implementation can "cheat".
const ORACLE_HELV_BOLD_ASCII: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

/// Width, in PDF points, of `s` set 8pt Helvetica-Bold. Non-ASCII falls back to 1000/1000 em (this
/// font's own widest glyphs — em dash, ellipsis — so an unmodeled character can only widen the
/// estimate, never narrow it and hide a real overflow).
fn oracle_helv_bold_8pt_width(s: &str) -> f32 {
    let units: u32 = s
        .chars()
        .map(|c| {
            let code = c as u32;
            if c.is_ascii() && (0x20..=0x7E).contains(&code) {
                ORACLE_HELV_BOLD_ASCII[(code - 0x20) as usize] as u32
            } else {
                1000
            }
        })
        .sum();
    units as f32 * 8.0 / 1000.0
}

/// Independently-declared text inset (PDF points, PER SIDE) — mirrors `form8275.rs::TEXT_INSET_PTS`
/// (round 2 finding 3) but is its OWN constant, not `use`d from `src/`, for the same "independent
/// oracle" reason as the width table above.
const ORACLE_TEXT_INSET_PTS: f32 = 2.0;

/// Assert that `value`'s rendered width at 8pt Helvetica-Bold fits INSIDE `fqn`'s inset-adjusted usable
/// box — a NEGATIVE allowance (subtracting, not adding, a hair of slop) so this check is STRICTER than
/// exact equality: a value that lands EXACTLY on the inset boundary still fails, guarding against float
/// rounding making the test pass right at the edge while a real renderer clips by a hair (round 2
/// finding 3's "tighten it to the inset budget with a negative allowance").
fn assert_fits_inset_box(fqn: &str, value: &str, raw_box_width: f32) {
    let usable = raw_box_width - 2.0 * ORACLE_TEXT_INSET_PTS;
    let measured = oracle_helv_bold_8pt_width(value);
    assert!(
        measured <= usable - 0.05,
        "{fqn}: its written content measures {measured:.2}pt wide at 8pt Helvetica-Bold but the \
         field's INSET-adjusted usable width is only {usable:.2}pt ({raw_box_width:.2}pt raw minus \
         {:.1}pt inset per side) — a PDF viewer honoring text inset (PDF 32000-1 §12.7.4.3) would \
         silently clip roughly {:.2}pt of text (the field holds {} characters): {value:?}",
        ORACLE_TEXT_INSET_PTS,
        measured - usable,
        value.chars().count(),
    );
}

/// The 6 Part II lines (`p1-t80[0]` = [`PART_II_LINE1`], then `p1-t81[0]`..`p1-t85[0]`) followed by
/// the 27 page-2 Part IV lines (`p2-t1[0]`..`p2-t27[0]`) — 33 continuation fields total, in the
/// bundled PDF's own printed top-to-bottom reading order. This is the map's full DECLARED shape (see
/// `form_8275_map_carries_the_32_new_continuation_fields`) — not what the fill actually writes today;
/// for that, see [`written_continuation_fields_in_order`].
fn all_33_declared_continuation_fields() -> Vec<String> {
    let mut v = vec![PART_II_LINE1.to_string()];
    for n in 81..=85 {
        v.push(format!("topmostSubform[0].Page1[0].p1-t{n}[0]"));
    }
    for n in 1..=27 {
        v.push(format!("topmostSubform[0].Page2[0].p2-t{n}[0]"));
    }
    v
}

/// The 28 fields the fill can ACTUALLY write, in printed order: Part II's own line 1
/// (`PART_II_LINE1`) then the 27 page-2 Part IV lines (`p2-t1[0]`..`p2-t27[0]`) — round 2 finding 4:
/// Part II's numbered lines 2-6 are permanently excluded (the bundled PDF's XFA numbers them beside
/// Part I's rows 1-6; see `form8275.rs`'s module doc).
fn written_continuation_fields_in_order() -> Vec<String> {
    let mut v = vec![PART_II_LINE1.to_string()];
    for n in 1..=27 {
        v.push(format!("topmostSubform[0].Page2[0].p2-t{n}[0]"));
    }
    v
}

/// A realistic long Part II narrative — a filer explaining lost records writes paragraphs, not a
/// sentence. Well over 1500 characters (asserted below) and distinct from every other Part II fixture
/// in this workspace (none of which exceeds ~200 characters — see the design note).
fn long_part_ii_narrative() -> String {
    "The taxpayer disposed of Bitcoin that was originally acquired over several transactions \
        spanning approximately three years, during which the taxpayer used a combination of a hosted \
        exchange account that has since ceased operations, a small number of in-person cash purchases \
        from a now-unreachable counterparty, and at least one peer-to-peer transaction conducted \
        through a messaging application whose records were not retained. The exchange that held the \
        earliest lots suspended withdrawals and subsequently entered insolvency proceedings; repeated \
        requests to its claims administrator for historical trade confirmations and cost-basis \
        statements went unanswered, and the taxpayer has been unable to obtain contemporaneous \
        documentation of the exact purchase prices paid for those lots despite good-faith efforts \
        including searching personal email archives, bank and credit-card statements covering the \
        relevant period, and any cached web pages of the exchange's now-defunct account dashboard. \
        Because a substantial and unrecoverable portion of the original acquisition records is \
        unavailable through no fault of the taxpayer, basis for the disposed lots was estimated using \
        the daily low closing price over the taxpayer's best-documented estimate of the acquisition \
        window, consistent with the Cohan doctrine, and the estimate was limited so as never to report \
        a loss that a complete record might not support. The taxpayer maintains that this approach is \
        a reasonable, conservative substitute for records that cannot be reconstructed, and discloses \
        it here in the interest of full transparency with respect to the estimated basis reported on \
        the attached Form 8949, so that the position is examined on its merits rather than treated as \
        an undisclosed estimate."
        .to_string()
}

/// ★ T-f8275-part-ii-overflow **Step 1 (test-first — RED before the fix)**, tightened round 2.
///
/// Two invariants a real disclosure must satisfy, checked independently of `btctax-forms`'s own
/// wrapping arithmetic:
///
///  1. **No field's content overflows its own INSET-adjusted physical box.** Every WRITTEN field
///     (`written_continuation_fields_in_order()` — Part II line 1 + Part IV, NOT Part II's numbered
///     lines 2-6, which are never written; see finding 4) must measure `<=` its own inset-adjusted
///     usable width at 8pt Helvetica-Bold ([`assert_fits_inset_box`], negative allowance — round 2
///     finding 3). This is what a viewer honoring the widget's geometry AND its text inset can actually
///     show.
///  2. **No character is lost.** Concatenating Part II line 1 + every non-empty Part IV line (with the
///     cross-reference prefix stripped from the first) must reproduce the ENTIRE original narrative, in
///     order.
///
/// Pre-fix (round 1), `push_free` writes the WHOLE narrative unclipped into `p1-t80[0]`'s `/V` (there
/// is no `/MaxLen` on this field, so nothing truncates the STORED string) — so invariant 2 alone cannot
/// distinguish the defect (the data is all there). Invariant 1 is what catches it: a 1500+ character
/// narrative is many thousands of points wide at 8pt, vastly over `p1-t80[0]`'s inset-adjusted box, and
/// every OTHER continuation field is untouched (empty) because nothing maps or writes them yet.
///
/// MUTATION (verified true — full transcript in BUILD-REPORT.md): with the fix in place, reverting
/// `fill_form_8275_inner`'s Part II section to the shipped single `push_free(part_ii_narrative,
/// printed.part_ii)` (no wrap) turns THIS test RED again. See BUILD-REPORT.md for the round-2 re-run
/// (line numbers here drift with every edit — the report cites the mutation's OWN output, not this
/// doc comment).
#[test]
fn form_8275_part_ii_long_narrative_does_not_silently_clip() {
    let narrative = long_part_ii_narrative();
    assert!(
        narrative.chars().count() > 1500,
        "fixture premise: the narrative must exceed 1500 characters, got {}",
        narrative.chars().count()
    );

    let printed = Printed8275 {
        part_i: sample_printed().part_i,
        part_ii: narrative.clone(),
    };
    let pdf = btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("non-empty part_i");
    let (doc, fields) = fields_of(&pdf);

    let mut reconstructed = String::new();
    let mut any_nonempty = false;
    let mut seen_any_part_iv = false;
    for fqn in written_continuation_fields_in_order() {
        let Some(mut value) = tv(&doc, &fields, &fqn) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        any_nonempty = true;
        let field = fields
            .iter()
            .find(|f| f.fqn == fqn)
            .unwrap_or_else(|| panic!("{fqn} must be a real field in the bundled PDF"));
        let rect = field
            .rect
            .unwrap_or_else(|| panic!("{fqn} must carry a /Rect"));
        assert_fits_inset_box(&fqn, &value, rect[2] - rect[0]);

        // The FIRST Part IV line carries the cross-reference prefix — strip it before folding into the
        // reconstruction, which describes the ORIGINAL narrative's words, not this crate's own label.
        if fqn != PART_II_LINE1 && !seen_any_part_iv {
            seen_any_part_iv = true;
            value = value
                .strip_prefix("Part II, line 1 (continued): ")
                .unwrap_or_else(|| {
                    panic!("the first Part IV line used must carry the cross-reference prefix: {value:?}")
                })
                .to_string();
        }
        if !reconstructed.is_empty() {
            reconstructed.push(' ');
        }
        reconstructed.push_str(&value);
    }
    assert!(
        any_nonempty,
        "no continuation field carried any Part II content at all"
    );
    assert!(
        seen_any_part_iv,
        "fixture premise: this narrative must be long enough to spill into Part IV"
    );

    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        norm(&reconstructed),
        norm(&narrative),
        "the disclosure must carry EVERY word of the filer's Part II narrative, in order — a partial \
         disclosure is not a disclosure"
    );
}

/// ★ Round 2 finding 4: Part II's numbered lines 2-6 (`p1-t81[0]`..`p1-t85[0]`) must NEVER be written,
/// even for a long narrative that would, under the round-1 shape, have spread across several of them.
/// The bundled PDF's XFA numbers these beside Part I's rows 1-6 (`Line2PartII`..`Line6PartII`,
/// confirmed by decompressing the asset's `template` XFA packet — see BUILD-REPORT.md); claiming them
/// for an unrelated combined narrative would misattribute sentence fragments to Part I items they do
/// not explain.
#[test]
fn form_8275_never_writes_part_ii_numbered_lines_2_through_6() {
    let printed = Printed8275 {
        part_i: sample_printed().part_i,
        part_ii: long_part_ii_narrative(),
    };
    let pdf = btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("non-empty part_i");
    let (doc, fields) = fields_of(&pdf);
    for fqn in PART_II_LINES_2_THROUGH_6 {
        assert_eq!(
            tv(&doc, &fields, fqn),
            None,
            "{fqn}: Part II's numbered continuation lines must never be written"
        );
    }
}

/// ★ Round 2 finding 1: whenever the narrative spills to Part IV, the FIRST Part IV line used carries
/// the IRS-required cross-reference (Rev. 10-2024 Specific Instructions: "Include the corresponding
/// part and line number from page 1") and NO OTHER Part IV line does — an examiner reading page 2 cold
/// (captioned "Explanations (continued from Parts I **and/or** II)") can tell which without it, but
/// only the first line needs to say so.
#[test]
fn form_8275_part_iv_cross_reference_appears_only_on_the_first_used_part_iv_line() {
    let printed = Printed8275 {
        part_i: sample_printed().part_i,
        part_ii: long_part_ii_narrative(),
    };
    let pdf = btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("non-empty part_i");
    let (doc, fields) = fields_of(&pdf);

    let part_iv_fields: Vec<String> = (1..=27)
        .map(|n| format!("topmostSubform[0].Page2[0].p2-t{n}[0]"))
        .collect();
    let mut seen_first = false;
    for fqn in &part_iv_fields {
        let Some(value) = tv(&doc, &fields, fqn) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        if !seen_first {
            assert!(
                value.starts_with("Part II, line 1 (continued): "),
                "{fqn}: the FIRST used Part IV line must carry the cross-reference prefix: {value:?}"
            );
            seen_first = true;
        } else {
            assert!(
                !value.starts_with("Part II, line 1 (continued): "),
                "{fqn}: only the FIRST Part IV line may carry the cross-reference prefix: {value:?}"
            );
        }
    }
    assert!(seen_first, "fixture premise: must spill into Part IV");
}

/// ★ Round 2 finding 5: a blank line between two promoted tranches' narratives (how
/// `disclosure_8275` joins them — `btctax-core/src/tax/form8275.rs`, `.join("\n\n")`) is a HARD break
/// on the filed form, not just whitespace that collapses to a single space — two independent factual
/// accounts must never run together into one sentence. Exercised through the FULL fill (not just
/// `wrap.rs`'s own unit test) so the field-selection + width-computation plumbing is covered too.
#[test]
fn form_8275_paragraph_breaks_are_hard_breaks_not_collapsed_to_a_space() {
    let printed = Printed8275 {
        part_i: sample_printed().part_i,
        part_ii: "Tranche A: cash P2P purchase, no records.\n\n\
                   Tranche B: peer-to-peer trade, receipt lost."
            .to_string(),
    };
    let pdf = btctax_forms::fill_form_8275(&printed, &kitchen_sink_header(), 2024)
        .unwrap()
        .expect("non-empty part_i");
    let (doc, fields) = fields_of(&pdf);

    // Both paragraphs are short enough to EASILY share Part II's line 1 by character count alone —
    // the hard-break rule is the only thing stopping that.
    assert_eq!(
        tv(&doc, &fields, PART_II_LINE1).as_deref(),
        Some("Tranche A: cash P2P purchase, no records."),
        "the first paragraph must NOT be joined with the second on Part II's line 1"
    );
    assert_eq!(
        tv(&doc, &fields, PART_IV_LINE1).as_deref(),
        Some("Part II, line 1 (continued): Tranche B: peer-to-peer trade, receipt lost."),
        "the second paragraph must start its OWN line (Part IV's first), not continue the first"
    );
}

/// ★ Round 2 finding 6: Part IV's continuation lines are a physically ORDERED sequence the fill
/// assumes is top-to-bottom (`map.part_iv_continuation`'s array order) — `push_free_ordered` puts them
/// in a `FlatPlacement` descent group so `verify_flat` enforces that assumption. Fault-inject a map
/// whose `part_iv_continuation` SWAPS two entries (index 0 `p2-t1` topmost, with index 2 `p2-t3`
/// further down the page) — the fill would then write the FIRST wrapped line into a LOWER field and a
/// LATER line into a HIGHER one, breaking the top-to-bottom ordinal-y descent the geometric oracle
/// checks, and must fail closed.
#[test]
fn fault_injected_8275_part_iv_reordered_fields_breaks_descent_and_is_red() {
    let mut map = Form8275Map::ty2024();
    map.part_iv_continuation.swap(0, 2);
    let printed = Printed8275 {
        part_i: sample_printed().part_i,
        part_ii: long_part_ii_narrative(), // needs several Part IV lines, so the swap is exercised
    };
    let err = fill_8275_with_map(&printed, &kitchen_sink_header(), &map).unwrap_err();
    assert!(
        matches!(&err, FormsError::Geometry(msg) if msg.contains("descent")),
        "expected a Geometry error naming the broken descent, got {err:?}"
    );
}

// ── #9 — the Form 8275 alias states its own PRECONDITION, instead of resting on a year list ────────
//
// `Form8275Map::for_year` was the only one of sixteen map constructors that aliased rather than
// refused: `2017 | 2024 | 2025 => ty2024() with the year re-stamped`. Aliasing is right for a form the
// IRS versions by REVISION — but a year list asserts nothing checkable, so `| 2026` was a one-token
// edit that would hand a Rev. 12-2026 Form 8275 the Rev. 10-2024 field names. The constructor now asks
// the asset registry which years ship a Form 8275, and the bytes whether it is still THIS revision.

/// The nearest miss: the same document, one byte different — a stand-in for a new IRS revision, which
/// is the event no test can wait for.
fn one_byte_different_8275() -> Vec<u8> {
    let mut v = F8275_PDF_2024.to_vec();
    let i = v.len() / 2;
    v[i] ^= 0xff;
    assert_ne!(
        v.as_slice(),
        F8275_PDF_2024,
        "the plant did not change the bytes"
    );
    v
}

#[test]
fn the_8275_alias_is_licensed_by_the_asset_not_by_the_calendar() {
    // Positive control: the revision the map WAS transcribed from licenses the alias — for a year the
    // build has never heard of, because the licence is about the document, not the year.
    Form8275Map::alias_is_licensed_by(2026, F8275_PDF_2024)
        .expect("the Rev. 10-2024 asset must license the map that was transcribed from it");

    // Planted defect 1 — a DIFFERENT IRS form in the 8275 slot.
    let err = Form8275Map::alias_is_licensed_by(2026, F8283_PDF_2024)
        .expect_err("a different document must NOT license the 8275 map");
    let msg = err.to_string();
    assert!(
        matches!(err, FormsError::Structure(_)) && msg.contains("2026") && msg.contains("8275"),
        "expected a refusal naming the year and the form, got {msg}"
    );

    // Planted defect 2 — the nearest miss: this form, a different revision.
    let err = Form8275Map::alias_is_licensed_by(2026, &one_byte_different_8275())
        .expect_err("a changed Form 8275 revision must NOT license the Rev. 10-2024 map");
    assert!(
        matches!(err, FormsError::Structure(_)),
        "expected a Structure refusal, got {err:?}"
    );
}

#[test]
fn for_year_answers_exactly_what_the_bundled_asset_registry_answers() {
    // The expected set is DERIVED — `bundled_pdf` (i.e. `pdf::f8275_pdf`) is the single authority for
    // which years ship a Form 8275. The probe range only decides which years get asked. Re-introducing
    // a year literal in `map.rs` (`2017 | 2024 | 2025 | 2026 => …`) reds here on 2026.
    let (mut supported, mut refused) = (0usize, 0usize);
    for year in 2010..=2035 {
        let asset = Form8275Map::bundled_pdf(year).is_ok();
        let map = Form8275Map::for_year(year);
        assert_eq!(
            map.is_ok(),
            asset,
            "TY{year}: bundled_pdf says {asset} but Form8275Map::for_year says {} — the map has \
             re-acquired a year list of its own",
            map.is_ok()
        );
        if let Ok(m) = map {
            assert_eq!(m.year, year, "the alias must re-stamp the requested year");
            supported += 1;
        } else {
            refused += 1;
        }
    }
    // Skipping is not passing: a probe that never saw either verdict would prove nothing.
    assert!(
        supported > 0 && refused > 0,
        "the probe saw {supported} supported and {refused} refused years — it discriminates nothing"
    );
    // NOT asserted here: how many years that is. Which years ship a Form 8275 is the asset registry's
    // business, and `map_year_matches_bundled_pdf_fieldset_for_every_supported_year` already requires
    // every `SUPPORTED_YEAR` to have one. This test owns one question — that `for_year` has no opinion
    // of its own about years — and coupling it to a second constant only makes it red for other reasons.
}

// ── #11 — an unknown key in a committed map is a PARSE ERROR, not a silent drop ────────────────────
//
// Measured before the fix, on `forms/2025/f6251.map.toml` (the TY2025 line-1 split): a *missing*
// required `line1` was a loud parse error, while the *added* `line1a` / `line1b` were discarded in
// silence. So a vanished line was loud and a RENAMED line was dropped without a word — and the map
// structs now carry `#[serde(deny_unknown_fields)]`, with `[census]` modelled (`CensusDecision`)
// rather than swallowed as an unknown key.

#[test]
fn an_unknown_key_in_a_map_is_a_parse_error_not_a_silent_drop() {
    // Positive control — the committed map still parses.
    Form8275Map::parse(F8275_MAP_2024).expect("the committed Rev. 10-2024 map parses");

    // Planted defect: a line ADDED under a name the struct does not model (the silent half of a
    // rename). Anchored after `year = 2024` so it lands at the top level, not inside a table.
    let added = F8275_MAP_2024.replace(
        "\nyear = 2024\n",
        "\nyear = 2024\npart_ii_line7 = \"topmostSubform[0].Page1[0].p1-t86[0]\"\n",
    );
    assert_ne!(added, F8275_MAP_2024, "the plant did not apply");
    let msg = Form8275Map::parse(&added)
        .expect_err("an unmodelled key must be refused, not dropped")
        .to_string();
    assert!(
        msg.contains("unknown field") && msg.contains("part_ii_line7"),
        "the refusal must NAME the key that would have vanished, got: {msg}"
    );

    // Planted defect: the full rename shape — old key gone, new key present. The failure must name the
    // NEW key (the half that used to be invisible), not merely report the old one missing.
    let renamed = F8275_MAP_2024.replace("\npart_ii_narrative =", "\npart_ii_line1 =");
    assert_ne!(renamed, F8275_MAP_2024, "the rename plant did not apply");
    let msg = Form8275Map::parse(&renamed)
        .expect_err("a renamed line must be refused")
        .to_string();
    assert!(
        msg.contains("part_ii_line1"),
        "the refusal must name the renamed key, got: {msg}"
    );
}

#[test]
fn the_census_block_is_parsed_not_ignored() {
    // `deny_unknown_fields` must not have been bought by teaching the struct to swallow `[census]`:
    // the block is modelled, so every entry is really deserialized. The expected count is DERIVED from
    // the committed file's own text (the `[census]` section's FQN-keyed lines), never hand-counted.
    let in_census = F8275_MAP_2024
        .lines()
        .skip_while(|l| l.trim() != "[census]")
        .filter(|l| l.trim_start().starts_with('"'))
        .count();
    assert!(
        in_census > 0,
        "the f8275 map has no [census] entries to check"
    );

    let map = Form8275Map::ty2024();
    assert_eq!(
        map.census.len(),
        in_census,
        "the parsed census must hold every entry the file writes"
    );
    for (fqn, d) in &map.census {
        assert!(
            !d.line.is_empty() && !d.rule.is_empty() && !d.reason.is_empty(),
            "census entry {fqn} has an empty line/rule/reason"
        );
    }

    // Planted defect: a fourth key inside a census entry — a decision the model does not understand
    // must be loud too, for the same reason a map line must.
    let doctored = F8275_MAP_2024.replace("{ line = ", "{ evidence = \"see the PDF\", line = ");
    assert_ne!(doctored, F8275_MAP_2024, "the census plant did not apply");
    let msg = Form8275Map::parse(&doctored)
        .expect_err("an unmodelled census key must be refused")
        .to_string();
    assert!(
        msg.contains("unknown field") && msg.contains("evidence"),
        "the refusal must name the unmodelled census key, got: {msg}"
    );
}

/// The one key set `deny_unknown_fields` cannot be written on directly: `MoneyCell` is
/// `#[serde(untagged)]`, and serde does not accept the attribute there. MEASURED here rather than
/// assumed — the table *variant* (`MoneyPair`) carries it, and an untagged enum whose every variant
/// fails is an error rather than a default, so an extra key inside an inline money-pair table is loud
/// too. (TY2017's Schedule SE is the committed map that still uses dollars/cents pairs.)
#[test]
fn an_unknown_key_inside_an_inline_money_pair_is_refused_too() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("forms/2017/schedule_se.map.toml");
    let src = std::fs::read_to_string(&path).expect("the TY2017 Schedule SE map is committed");
    ScheduleSeMap::parse(&src).expect("the committed TY2017 Schedule SE map parses");

    let doctored = src.replacen(
        "{ dollars_field = ",
        "{ pennies_field = \"topmostSubform[0].Page2[0].f2_99[0]\", dollars_field = ",
        1,
    );
    assert_ne!(doctored, src, "the money-pair plant did not apply");
    let msg = ScheduleSeMap::parse(&doctored)
        .expect_err("an unmodelled key inside a money pair must be refused")
        .to_string();
    assert!(
        msg.contains("pennies_field") || msg.contains("did not match any variant"),
        "the refusal must be about the unmodelled money-pair key, got: {msg}"
    );
}
