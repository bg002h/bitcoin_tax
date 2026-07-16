//! **Oracle-sweep T4 — the forms-side on-paper read-back.**
//!
//! The double-oracle sweep (T6) reads a FILLED PDF and compares the figures in the boxes against what
//! two independent tax engines computed. But the paper does not print signed integers: the 1040 signs
//! line 7 with a LEADING MINUS (SPEC §3.2), while Schedule D's own lines 6/14/21 are PARENTHESIZED
//! boxes — the form pre-prints the parentheses, so the cell carries a POSITIVE MAGNITUDE that MEANS a
//! negative. Reading either cell as its literal string would compare `"3000"` against `-3000` and call
//! a correct return wrong (or, worse, call a sign-flipped return right).
//!
//! [`common::on_paper_signed`] is the sign table that turns a filled cell back into the signed integer
//! it represents, per its §6.3 convention; [`common::cell_or_zero`] reads a "blank" cell under an
//! explicit regime. This file is their contract test — anchored on a real filled return
//! (`single_capital_loss_capped`, whose §1211(b)-limited loss puts −3000 on both cells) plus unit
//! coverage of every variant and the parse-discipline boundary.

mod common;
use common::{cell_or_zero, form, on_paper_signed, packet, Blank, Sign};

use btctax_core::tax::testonly::golden_households;
use btctax_forms::testonly::{extract_lines, F1040_MAP_2024, SCHEDULE_D_MAP_2024};
use std::collections::BTreeMap;

/// ★ The anchor: a capped capital loss is −3000 on the 1040 (leading minus) AND on Schedule D
/// (parenthesized magnitude), and the two sign conventions must read back to the SAME signed integer.
#[test]
fn line7_is_signed_and_schedule_d_is_parenthesized_magnitude() {
    let h = golden_households()
        .into_iter()
        .find(|h| h.name == "single_capital_loss_capped")
        .unwrap();
    let pkt = packet(&h);

    let f1040 = extract_lines(&form(&pkt, "f1040").bytes, F1040_MAP_2024).unwrap();
    // 1040 line 7 is on paper as the literal string "-3000" — a leading minus (SPEC §3.2).
    assert_eq!(
        on_paper_signed(&f1040, "line7a", Sign::Leading),
        Some(-3000),
        "1040 line 7 signs a capital loss with a LEADING MINUS; it must read back as −3000"
    );

    let sd = extract_lines(&form(&pkt, "schedule_d").bytes, SCHEDULE_D_MAP_2024).unwrap();
    // Schedule D line 21 is on paper as the bare magnitude "3000" inside a pre-printed paren box.
    assert_eq!(
        on_paper_signed(&sd, "line21", Sign::ParenMagnitude),
        Some(-3000),
        "Schedule D line 21 is a POSITIVE MAGNITUDE in a parenthesized box; it MEANS −3000"
    );
}

// ── helper unit coverage ────────────────────────────────────────────────────────────────────────

fn cells(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn absent_key_reads_none() {
    let c = cells(&[]);
    assert_eq!(on_paper_signed(&c, "line7a", Sign::Leading), None);
    assert_eq!(on_paper_signed(&c, "line7a", Sign::ParenMagnitude), None);
    assert_eq!(on_paper_signed(&c, "line7a", Sign::Unsigned), None);
}

#[test]
fn leading_reads_the_sign_verbatim() {
    let c = cells(&[("neg", "-3000"), ("pos", "42000")]);
    assert_eq!(on_paper_signed(&c, "neg", Sign::Leading), Some(-3000));
    assert_eq!(on_paper_signed(&c, "pos", Sign::Leading), Some(42000));
}

#[test]
fn paren_magnitude_negates_the_magnitude() {
    let c = cells(&[("mag", "3000"), ("zero", "0")]);
    assert_eq!(on_paper_signed(&c, "mag", Sign::ParenMagnitude), Some(-3000));
    // Zero is its own negation — a paren box that happens to carry 0 reads back as 0, not −0.
    assert_eq!(on_paper_signed(&c, "zero", Sign::ParenMagnitude), Some(0));
}

#[test]
fn unsigned_reads_as_is() {
    let c = cells(&[("v", "18000")]);
    assert_eq!(on_paper_signed(&c, "v", Sign::Unsigned), Some(18000));
}

#[test]
#[should_panic(expected = "not-a-number")]
fn a_present_but_unparseable_cell_panics_with_the_raw_string() {
    // Parse discipline (§6.3): a present cell that is not an integer is a BUG in the filler or the map,
    // never a silent None. The panic must carry the raw string so the failure names what it saw.
    let c = cells(&[("v", "not-a-number")]);
    let _ = on_paper_signed(&c, "v", Sign::Unsigned);
}

#[test]
fn present_zero_regime_accepts_a_printed_zero() {
    let c = cells(&[("line17", "0")]);
    assert_eq!(cell_or_zero(&c, "line17", Blank::PresentZero), 0);
}

#[test]
#[should_panic(expected = "line17")]
fn present_zero_regime_rejects_an_absent_cell() {
    // PresentZero is the dropped-line detector: an ABSENT cell means the filler stopped writing the
    // line, which must fail loudly rather than default to 0.
    let c = cells(&[]);
    let _ = cell_or_zero(&c, "line17", Blank::PresentZero);
}

#[test]
#[should_panic(expected = "5")]
fn present_zero_regime_rejects_a_nonzero_cell() {
    let c = cells(&[("line17", "5")]);
    let _ = cell_or_zero(&c, "line17", Blank::PresentZero);
}

#[test]
fn absent_is_zero_regime_reads_absent_as_zero_and_present_as_itself() {
    let c = cells(&[("present", "42")]);
    assert_eq!(cell_or_zero(&c, "absent", Blank::AbsentIsZero), 0);
    assert_eq!(cell_or_zero(&c, "present", Blank::AbsentIsZero), 42);
}
