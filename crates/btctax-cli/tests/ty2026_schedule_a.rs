//! ★★★ **TY2026 SCHEDULE A — WRONG FIGURE, OR WRONG LABEL?** (wave 5,
//! `design/agent-reports/BRIEF-wave5-schedule-a.md`; report
//! `design/agent-reports/REPORT-wave5-schedule-a.md`.)
//!
//! **THIS FILE VALIDATES NOTHING.** OpenTaxSolver 2026 does not exist until ~2027-01, taxcalc has no
//! validated TY2026 parameters, and no `f1040sa--2026` FINAL is archived — only a draft marked
//! *"DRAFT — evidence only, never transcribed as authority"*. Every TY2026 figure below is evidence
//! that a **mechanism reads a field**. None of it is evidence that a number is right.
//!
//! ## The question this file answers
//!
//! Tier A (`design/agent-reports/REPORT-stage2-A.md` §3) established that the printed layer **cannot
//! see the year**: `printed::schedule_a_lines(ar, line11_1040)` takes neither a year nor the params,
//! and `packet::assemble_printed_forms` is not passed the params at all. It did **not** establish
//! whether that yields a **wrong figure** or only a **wrong label**. For each of FR-185's six
//! collisions this file answers which, with the figure.
//!
//! ## THE SUBSTITUTION LIST — two items, both stated, neither a fabricated number
//!
//! | # | substituted | from | why it is legitimate |
//! |---|---|---|---|
//! | W5-1 | the **year** the chain is assembled at: TY2024 | the only year whose `FullReturnParams` are bundled (`BundledFullReturnTables::load()` inserts **2024 alone**) | `assemble_absolute` at 2026 ABORTS (Tier A's A-1, a `panic!` from `form6251_line1_rule`) and a test cannot bypass it without editing shipped source. The chain is then re-run with TY2026's own `ScheduleAParts` substituted into `AbsoluteReturn::schedule_a` — the one field `schedule_a_lines` reads for every Schedule A cell — so the TY2026 **parameters** do reach the printed cells. |
//! | W5-2 | a **TY2025** parameter set | `ty2026_full_return()` with line 5e's dollar figures replaced by the ones the **2025 extract itself prints**, parsed out of the text layer rather than typed | it exists only to be the CONTROL: the one line whose figures move while its number does not. Nothing else about it is used or claimed. |
//!
//! ★ There is **no** invented TY2026 arithmetic here and no guessed worksheet. The two documents the
//! TY2026 Schedule A depends on — the *Charitable Contribution Limitation Worksheet* (its line 6 is
//! Schedule A line 13) and the *Itemized Deductions Worksheet* (behind line 18's $384,350 screen) —
//! live in `i1040sca--2026` / `i1040gi--2026`, **neither archived**. Where a figure needs one, this
//! file names the gap and asserts nothing.
//!
//! ## Why it lives in `btctax-cli/tests/`
//!
//! Tier A's precedent, for its reason: the harness must read the **shipped** `ty2026_full_return()`
//! rather than a second copy, `btctax-core` does not depend on `btctax-adapters`, and adding that as
//! a dev-dep trips `repo_hygiene`'s pin rule — whose fix (a version) would break the
//! dependency-first publish order. **Zero manifest changes; no shipped source modified.**

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use btctax_adapters::tax_tables::ty2026_full_return;
use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::event::{BasisSource, DisposeKind};
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, LedgerState, Term};
use btctax_core::tax::charitable::apply_170b;
use btctax_core::tax::printed::{schedule_a_lines, ScheduleALines};
use btctax_core::tax::return_1040::{
    assemble_absolute, schedule_a_parts, AbsoluteReturn, ScheduleAParts,
};
use btctax_core::tax::return_inputs::{
    CarryProvenance, CharitableCarryItem, CharitableClass, CharitableGift, Form1099Div,
    Form1099Int, HouseholdHeader, Owner, Person, ReturnInputs, ScheduleAInputs, W2,
};
use btctax_core::tax::tables::{FullReturnParams, FullReturnTables, SaltLimitation, TaxTables};
use btctax_core::tax::testonly::{
    answer_all_live_declarations, form_1098_with_interest, reconcile_digital_asset_activity,
};
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use time::macros::date;

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §0 — THE EXTRACTS. Every TY2026 line fact below is READ OFF THE TEXT LAYER, never typed.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

const SA_2025: &str = "f1040sa--2025.txt";
const SA_2026: &str = "f1040sa--2026-DRAFT.txt";

fn extract(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../design/forms/extract")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Normalise one caption for comparison: collapse whitespace, drop `pdftotext`'s dot leaders, and drop
/// **exactly one** trailing answer-box id (the far-right column lands on the same physical line for a
/// one-line caption).
///
/// ★★ **"Exactly one" is load-bearing and was found by the loop being wrong.** A first draft popped
/// trailing ids repeatedly, which ate the caption's own operands: *"Add lines 8a through 8c  8e"*
/// became *"Add lines 8a through"* — and so did 2026's *"Add lines 8a through 8d  8e"*, so the two
/// revisions compared EQUAL and line 8e dropped out of the changed set. A checker that erases the very
/// numbers it is comparing is the `CLAUDE.md` blind-instrument shape; one pop is the answer-box, and
/// anything before it is the sentence.
fn norm(s: &str) -> String {
    let mut words: Vec<&str> = s.split_whitespace().filter(|w| *w != ".").collect();
    if let Some(last) = words.last() {
        let digits: String = last.chars().take_while(char::is_ascii_digit).collect();
        let rest = &last[digits.len()..];
        let is_box = !digits.is_empty()
            && digits.len() <= 2
            && (rest.is_empty()
                || (rest.len() == 1 && rest.starts_with(|c: char| c.is_ascii_lowercase())));
        if is_box && words.len() > 1 {
            words.pop();
        }
    }
    words.join(" ")
}

/// The caption at `id` **plus the three physical lines that follow it**, whitespace-collapsed — for the
/// later-clause facts the one-line [`captions`] map deliberately cannot see (`"check this box"` sits on
/// a continuation line on both revisions).
fn caption_block(text: &str, id: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let head = captions(text)
        .get(id)
        .unwrap_or_else(|| panic!("no caption for line {id}"))
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join(" ");
    let at = lines
        .iter()
        .position(|l| {
            l.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains(&head)
        })
        .unwrap_or_else(|| panic!("line {id}'s caption is not in the text layer"));
    lines[at..(at + 4).min(lines.len())]
        .join(" ")
        .split_whitespace()
        .filter(|w| *w != ".")
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every line id the schedule prints, mapped to the FIRST physical line of its own caption.
///
/// ★★ **WHAT THIS DERIVATION COVERS AND WHAT IT DOES NOT** (`CLAUDE.md`'s *derive the list* rule 3 —
/// an honest boundary is reviewable, a silent one is the defect):
///
/// * It reads the **left label column**: an `N` / `Na` token, or a bare sub-line letter in `a, b, c…`
///   order beneath the integer parent that introduced it, followed by whitespace and a capital letter
///   or `(`.
/// * The bare-letter branch exists **because of Tier A's A-7**: `pdftotext -layout` renders the 2025
///   sheet's sub-lines as a letter in its own column (`d Reserved for future use`) and the 2026
///   sheet's inline (`8d Mortgage insurance premiums`). Without it, `5a`–`5e` and `8a`–`8e` read as
///   *new in 2026* although both revisions print them, and a raw id-set difference over-reports.
///   Requiring the letter to be the expected NEXT one under the current parent is what makes the
///   branch safe against a stray `" a Word"` in prose.
/// * It captures only the **first** physical line of a caption, so a multi-line caption is compared on
///   its opening clause alone — enough to tell *"Carryover from prior year"* from *"Add lines 13 and
///   14"*, and not enough to catch a change buried in a later clause. Every later-clause fact this
///   file asserts is asserted by an explicit `contains` against the whole extract instead.
/// * Scanning starts at the form's own `SCHEDULE A` header, so the 2026 draft's coversheet prose
///   cannot contribute an id.
fn captions(text: &str) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let start = text
        .lines()
        .position(|l| l.contains("SCHEDULE A"))
        .expect("both extracts print the form's own `SCHEDULE A` header");
    let mut parent: Option<u32> = None;
    let mut next_letter: Option<char> = None;
    for raw in text.lines().skip(start) {
        let chars: Vec<char> = raw.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            if i > 0 && !chars[i - 1].is_whitespace() {
                i += 1;
                continue;
            }
            let start_i = i;
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let digits = j - start_i;
            let mut end = j;
            let mut id: Option<String> = None;
            let mut new_parent: Option<u32> = None;
            if (1..=2).contains(&digits) {
                if end < chars.len()
                    && chars[end].is_ascii_lowercase()
                    && (end + 1 >= chars.len() || !chars[end + 1].is_ascii_alphanumeric())
                {
                    end += 1;
                }
                let tok: String = chars[start_i..end].iter().collect();
                let num: String = tok.chars().take_while(char::is_ascii_digit).collect();
                if tok.len() == num.len() {
                    new_parent = num.parse().ok();
                }
                id = Some(tok);
            } else if digits == 0
                && chars[start_i].is_ascii_lowercase()
                && Some(chars[start_i]) == next_letter
                && (start_i + 1 >= chars.len() || chars[start_i + 1].is_whitespace())
            {
                if let Some(p) = parent {
                    end = start_i + 1;
                    id = Some(format!("{p}{}", chars[start_i]));
                }
            }
            let Some(id) = id else {
                i = if end > start_i { end } else { i + 1 };
                continue;
            };
            // …followed by whitespace and then a capital letter or an opening paren.
            let mut k = end;
            let mut spaces = 0;
            while k < chars.len() && chars[k] == ' ' {
                k += 1;
                spaces += 1;
            }
            if spaces >= 1 && k < chars.len() && (chars[k].is_ascii_uppercase() || chars[k] == '(')
            {
                let caption: String = chars[k..].iter().collect();
                out.entry(id).or_insert_with(|| norm(&caption));
                if let Some(p) = new_parent {
                    parent = Some(p);
                    next_letter = Some('a');
                } else if let Some(l) = next_letter {
                    next_letter = char::from_u32(l as u32 + 1);
                }
            }
            i = end.max(start_i + 1);
        }
    }
    out
}

/// Line 5e's dollar figures, in the order the sentence prints them: the cap, the MFS cap, the
/// §164(b)(7)(B) phase-out threshold, the MFS threshold. **Parsed, never typed** — the whole point of
/// the control is that its numbers come from the form.
fn salt_5e_figures(text: &str) -> Vec<Usd> {
    let idx = text
        .find("Enter the smaller of line 5d or")
        .expect("both revisions print line 5e's own sentence");
    let tail = &text[idx..];
    let end = tail.find("see instructions").unwrap_or(tail.len());
    let chars: Vec<char> = tail[..end].chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' {
            let mut j = i + 1;
            let mut n = String::new();
            while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == ',') {
                if chars[j].is_ascii_digit() {
                    n.push(chars[j]);
                }
                j += 1;
            }
            if !n.is_empty() {
                out.push(n.parse::<Usd>().expect("a dollar figure off the form"));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §1 — FR-185's SIX COLLISIONS, DERIVED FROM THE TWO TEXT LAYERS.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// ★★★ **The cascade is derived, not typed.** Two ids carry, *verbatim*, the caption a DIFFERENT id
/// carried on the 2025 sheet — the mechanical signature of a shift-by-one renumber, needing no
/// hand-written move table:
///
/// * 2026 **14** = 2025 **13** — *"Carryover from prior year"*
/// * 2026 **19** = 2025 **18** — *"If you elect to itemize deductions even though they are less than
///   your standard…"*
#[test]
fn the_schedule_a_renumber_cascade_is_derived_from_the_two_extracts() {
    let c25 = captions(&extract(SA_2025));
    let c26 = captions(&extract(SA_2026));

    // (a) Captions carried VERBATIM to a different id. Derived by searching 2025 for each 2026
    //     caption — nothing about "13" or "18" is typed into the search.
    let moved: BTreeMap<&str, &str> = c26
        .iter()
        .filter_map(|(id26, cap)| {
            c25.iter()
                .find(|(id25, c)| *c == cap && id25.as_str() != id26.as_str())
                .map(|(id25, _)| (id26.as_str(), id25.as_str()))
        })
        .collect();
    assert_eq!(
        moved,
        BTreeMap::from([("14", "13"), ("19", "18")]),
        "exactly two captions are carried VERBATIM to a new line number; got {moved:?}"
    );

    // (b) The shared ids whose caption changed at all.
    let changed: BTreeSet<&str> = c26
        .iter()
        .filter(|(id, cap)| c25.get(*id).is_some_and(|old| old != *cap))
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        changed,
        BTreeSet::from(["5e", "8d", "8e", "13", "14", "15", "16", "17", "18"]),
        "the derived changed-caption set: the six from 13 up are FR-185's collisions; 5e / 8d / 8e are \
         IN-PLACE changes on a number that did not move"
    );

    // (c) 19 is new — the sheet grew by one line.
    assert!(!c25.contains_key("19") && c26.contains_key("19"));

    // (d) The six collisions, transcribed AND checked back against the extract.
    for (id, cap25, cap26) in [
        (
            "13",
            "Carryover from prior year",
            "Enter the amount from line 6 of the Charitable Contribution",
        ),
        ("14", "Add lines 11 through 13", "Carryover from prior year"),
        (
            "15",
            "Casualty and theft loss(es) from a federally declared disaster (other than net qualified",
            "Add lines 13 and 14",
        ),
        (
            "16",
            "Other—from list in instructions. List type and amount:",
            "Casualty and theft loss(es) from a federally or state-declared disaster (other than net",
        ),
        (
            "17",
            "Add the amounts in the far right column for lines 4 through 16. Also, enter this amount",
            "Other itemized deductions (see instructions).",
        ),
        (
            "18",
            "If you elect to itemize deductions even though they are less than your standard",
            "Is the amount on Form 1040 or 1040-SR, line 11b, minus the amounts on lines 13a and",
        ),
    ] {
        assert_eq!(c25.get(id).map(String::as_str), Some(cap25), "2025 line {id}");
        assert_eq!(c26.get(id).map(String::as_str), Some(cap26), "2026 line {id}");
    }

    // (e) ★ FR-185's *"line 18 is the worst shape available"* — both halves read off the text.
    let t25 = extract(SA_2025);
    let t26 = extract(SA_2026);
    assert!(
        caption_block(&t25, "18").contains("check this box"),
        "2025's 18 is a CHECKBOX"
    );
    assert!(
        t26.contains("Also enter this amount on Form 1040 or 1040-SR, line 12e."),
        "2026's 18 is the line that feeds 1040 line 12e"
    );
    assert!(
        caption_block(&t26, "19").contains("check this box"),
        "2026's 19 is the checkbox"
    );

    // (f) ★ 17 became a PARENT with enumerated sub-lines; 2025's 16 had none.
    let subs26: BTreeSet<&str> = c26
        .keys()
        .filter(|k| k.starts_with("17") && k.len() > 2)
        .map(String::as_str)
        .collect();
    assert_eq!(
        subs26,
        BTreeSet::from([
            "17a", "17b", "17c", "17d", "17e", "17f", "17g", "17h", "17i", "17j", "17k", "17z"
        ]),
        "2026 line 17 enumerates 17a–17k plus the 17z subtotal"
    );
    assert!(
        c25.keys().all(|k| !(k.starts_with("16") && k.len() > 2)),
        "2025's line 16 has no sub-lines"
    );
}

/// ★★ **B1 — the derivation has been SEEN RED.** Plant the exact defect it exists to catch: undo the
/// renumber in the text the checker reads, by putting 2025's line-14 caption back at 14. The
/// verbatim-move detector must then stop finding `14 ← 13`.
#[test]
fn a_reverted_caption_is_no_longer_detected_as_a_move() {
    let c25 = captions(&extract(SA_2025));
    let original = extract(SA_2026);
    let planted = original.replace(
        "14     Carryover from prior year",
        "14     Add lines 11 through 13",
    );
    assert_ne!(planted, original, "the plant must actually edit the text");
    let c26 = captions(&planted);
    assert_eq!(
        c26.get("14").map(String::as_str),
        Some("Add lines 11 through 13"),
        "the plant landed"
    );
    let moved: Vec<&str> = c26
        .iter()
        .filter(|(id26, cap)| {
            c25.iter()
                .any(|(id25, c)| c == *cap && id25.as_str() != id26.as_str())
        })
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        moved,
        vec!["19"],
        "with 14 reverted only the 18→19 move remains — so the detector reads the text and not a \
         memory of it"
    );
}

/// ★★ **B1 for [`salt_5e_figures`] — seen RED on a planted figure.** The control's numbers are only
/// worth anything if the parser actually follows the document; plant a different cap in the text and it
/// must report the planted one.
#[test]
fn the_salt_figure_parser_follows_the_text_it_is_given() {
    let planted = extract(SA_2026).replace("$40,400", "$41,900");
    assert_ne!(
        planted,
        extract(SA_2026),
        "the plant must actually edit the text"
    );
    assert_eq!(
        salt_5e_figures(&planted),
        vec![dec!(41900), dec!(20200), dec!(505000), dec!(252500)],
        "only the planted figure moves, and it moves — so the parser reads the sheet"
    );
}

/// ★ **THE CONTROL, half one: line 5e keeps its number and moves every figure** — and the figures the
/// BUNDLED `ty2026_full_return()` carries are the ones the 2026 sheet prints. That cross-check is what
/// makes 5e usable as a control: it proves the params were transcribed from this document.
#[test]
fn line_5e_keeps_its_number_and_every_figure_moves() {
    let f25 = salt_5e_figures(&extract(SA_2025));
    let f26 = salt_5e_figures(&extract(SA_2026));
    assert_eq!(
        f25,
        vec![dec!(40000), dec!(20000), dec!(500000), dec!(250000)]
    );
    assert_eq!(
        f26,
        vec![dec!(40400), dec!(20200), dec!(505000), dec!(252500)]
    );

    let SaltLimitation::Worksheet2025 {
        line1_cap,
        line5_threshold,
        line5_threshold_mfs,
        ..
    } = ty2026_full_return().salt
    else {
        panic!("TY2026's SALT limitation is the worksheet shape, not a flat cap")
    };
    assert_eq!(
        (line1_cap, line5_threshold, line5_threshold_mfs),
        (f26[0], f26[2], f26[3]),
        "the bundled TY2026 params must carry the figures the 2026 sheet prints"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §2 — THE PRINTED CHAIN'S SHAPE, HELD BY THE COMPILER.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Which Schedule A cell a `ScheduleALines` field can express, and in what kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Money,
    Check,
    /// Text on the dotted lines beside a money cell, not a cell of its own.
    Dotted,
}

/// ★★★ **The cell set `ScheduleALines` can express — destructured with NO `..`, so adding a field does
/// not compile until it is classified here.** That is the compiler holding the list (`CLAUDE.md` rule
/// 2); a hand-typed cell list would go stale the day a `line15` lands, which is the exact day it must
/// not.
fn cells_the_struct_expresses(l: &ScheduleALines) -> BTreeMap<&'static str, Kind> {
    let ScheduleALines {
        line5a_is_sales_tax,
        line18_elects_smaller,
        line8_mixed_use_box,
        line1,
        line2,
        line3,
        line4,
        line5a,
        line5b,
        line5c,
        line5d,
        line5e,
        line7,
        line8a,
        line8b,
        line8c,
        line8b_payee,
        line8e,
        line9,
        line10,
        line11,
        line12,
        line13,
        line14,
        line17,
    } = l;
    // Every binding is consumed, so the destructure cannot silently ignore a field.
    let money: [(&'static str, &Usd); 21] = [
        ("1", line1),
        ("2", line2),
        ("3", line3),
        ("4", line4),
        ("5a", line5a),
        ("5b", line5b),
        ("5c", line5c),
        ("5d", line5d),
        ("5e", line5e),
        ("7", line7),
        ("8a", line8a),
        ("8b", line8b),
        ("8c", line8c),
        ("8e", line8e),
        ("9", line9),
        ("10", line10),
        ("11", line11),
        ("12", line12),
        ("13", line13),
        ("14", line14),
        ("17", line17),
    ];
    let checks: [(&'static str, &bool); 3] = [
        ("5", line5a_is_sales_tax),
        ("8", line8_mixed_use_box),
        ("18", line18_elects_smaller),
    ];
    let mut out: BTreeMap<&'static str, Kind> = BTreeMap::new();
    for (cell, _) in money {
        out.insert(cell, Kind::Money);
    }
    for (cell, _) in checks {
        out.insert(cell, Kind::Check);
    }
    let _: &Vec<String> = line8b_payee;
    out.insert("8b-dotted", Kind::Dotted);
    out
}

/// ★★★ **The printed chain takes NO year and NO params — and the compiler now says so.** A coercion to
/// an explicit `fn` type is the check: give `schedule_a_lines` a year, a `&FullReturnParams`, or a
/// revision selector and this file stops compiling. Prose cannot make that promise; a signature can.
#[test]
fn the_printed_schedule_a_chain_cannot_see_the_year() {
    let _: fn(&AbsoluteReturn, Usd) -> Option<ScheduleALines> = schedule_a_lines;
}

/// ★★★ **Which TY2026 cells the chain cannot express at all** — and the answer includes **15**, the
/// subtotal the 2026 total is figured from.
///
/// `CLAUDE.md`: *two blanks look identical on the printed page and are not the same thing.* 16 and
/// 17a–17z are blank **because this filer has no casualty loss and no gambling losses** — the common,
/// correct case. 15 is blank because **nothing ever populated it**, which is the defect class. 19 is
/// worse than blank: the §63(e) election the chain *does* compute is expressed at **18**, which on
/// this revision is money.
#[test]
fn the_ty2026_cells_the_struct_cannot_express_include_the_subtotal_the_total_needs() {
    let c26 = captions(&extract(SA_2026));
    let (_, lines) = sch_a_at(&ty2026_full_return());
    let expressed = cells_the_struct_expresses(&lines);

    let unexpressible: BTreeSet<&str> = c26
        .keys()
        .map(String::as_str)
        .filter(|id| !expressed.contains_key(*id))
        .collect();
    assert_eq!(
        unexpressible,
        BTreeSet::from([
            "15", "16", "17a", "17b", "17c", "17d", "17e", "17f", "17g", "17h", "17i", "17j",
            "17k", "17z", "19", "6", "8d"
        ]),
        "TY2026 ids `ScheduleALines` has no field for at all"
    );
    assert!(
        unexpressible.contains("15"),
        "★ 15 is *\"Add lines 13 and 14\"* — a REQUIRED subtotal of the 2026 total, and the struct \
         cannot hold it. That blank is not the inputs speaking; it is nothing having populated it."
    );

    // ★ And the inversion: 18 IS expressible — as a boolean, on the revision where it is the total.
    assert_eq!(
        expressed.get("18"),
        Some(&Kind::Check),
        "the chain expresses 18 as a checkbox; TY2026's 18 is the six-figure total"
    );
    assert!(
        c26["18"].starts_with("Is the amount on Form 1040"),
        "…and this is what 18 asks on the 2026 sheet"
    );
}

/// ★★ **B1 for the unexpressible-cell difference — seen RED on a planted omission.** Drop one cell
/// from the classification and it must appear in the set of things the struct cannot express. Without
/// this the set difference could be computed over an empty or mis-keyed map and still print a
/// plausible answer.
#[test]
fn dropping_a_cell_from_the_classification_makes_it_unexpressible() {
    let c26 = captions(&extract(SA_2026));
    let (_, lines) = sch_a_at(&ty2026_full_return());
    let mut planted = cells_the_struct_expresses(&lines);
    assert_eq!(
        planted.remove("13"),
        Some(Kind::Money),
        "the plant must remove a real entry"
    );
    let unexpressible: BTreeSet<&str> = c26
        .keys()
        .map(String::as_str)
        .filter(|id| !planted.contains_key(*id))
        .collect();
    assert!(
        unexpressible.contains("13"),
        "a cell removed from the classification must surface as unexpressible"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// §3 — THE OWNER'S PROFILE, DRIVEN THROUGH THE REAL CHAIN.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

fn person(first: &str, last: &str, ssn: &str) -> Person {
    Person {
        first_name: first.into(),
        last_name: last.into(),
        ssn: ssn.into(),
        occupation: "Engineer".into(),
        ..Default::default()
    }
}

/// The year the chain is ASSEMBLED at — substitution W5-1. TY2024 is the only year whose
/// `FullReturnParams` are bundled, and `assemble_absolute` at 2026 aborts (Tier A's A-1).
const ASSEMBLY_YEAR: i32 = 2024;

/// ★★ **THE OWNER'S SHAPE:** W-2 wages + Bitcoin dispositions + **itemized** Schedule A + Schedule B
/// over the $1,500 thresholds. **No Schedule C, no retirement.**
///
/// State withholding is set so line 5d lands **above every cap under test**, which is what makes 5e a
/// working control: TY2024's flat $10,000, TY2025's $40,000 and TY2026's $40,400 then produce three
/// different cells out of one fixture. MAGI stays below the $505,000 phase-out start, so the phase-out
/// is deliberately not in play — the cap alone moves.
fn owner_shape_household(carryover: Usd) -> (ReturnInputs, LedgerState) {
    let mut ri = ReturnInputs {
        tax_year: ASSEMBLY_YEAR,
        filing_status: FilingStatus::Mfj,
        header: HouseholdHeader {
            taxpayer: person("Alex", "Vale", "123-45-6789"),
            spouse: Some(person("Robin", "Vale", "987-65-4321")),
            address_street: "4 Larch Ln".into(),
            address_city: "Springfield".into(),
            address_state: "IL".into(),
            address_zip: "62704".into(),
            ..Default::default()
        },
        w2s: vec![W2 {
            owner: Owner::Taxpayer,
            employer: "SOFTWARE CO".into(),
            box1_wages: dec!(220000),
            box2_fed_withheld: dec!(44000),
            box3_ss_wages: dec!(220000),
            box4_ss_withheld: Usd::ZERO,
            box5_medicare_wages: dec!(220000),
            box6_medicare_withheld: dec!(3190),
            box17_state_tax_withheld: dec!(36000),
            ..Default::default()
        }],
        int_1099: vec![Form1099Int {
            payer: "CREDIT UNION".into(),
            box1_interest: dec!(4200),
            ..Default::default()
        }],
        div_1099: vec![Form1099Div {
            payer: "INDEX FUND".into(),
            box1a_ordinary: dec!(9000),
            box1b_qualified: dec!(7500),
            ..Default::default()
        }],
        foreign_accounts: Some(false),
        foreign_trust: Some(false),
        form_1098: vec![form_1098_with_interest(dec!(19000))],
        schedule_a: Some(ScheduleAInputs {
            medical: dec!(3000),
            salt_real_estate: dec!(9500),
            charitable: vec![CharitableGift {
                class: CharitableClass::Cash60,
                amount: dec!(6000),
            }],
            ..Default::default()
        }),
        has_income_exclusion: Some(false),
        other_out_of_scope_income: Some(false),
        charitable_cwa_obtained: Some(true),
        donations_had_restrictions: Some(false),
        filing_form_4952: Some(false),
        claiming_mortgage_interest_credit: Some(false),
        ..Default::default()
    };
    if carryover > Usd::ZERO {
        ri.charitable_carryover_in = vec![CharitableCarryItem {
            class: CharitableClass::Cash60,
            amount: carryover,
            origin_year: ASSEMBLY_YEAR - 2,
            provenance: CarryProvenance::default(),
        }];
    }
    let leg = |proceeds, basis, gain, term, acquired, sat| DisposalLeg {
        lot_id: LotId {
            origin_event_id: EventId::decision(9),
            split_sequence: 0,
        },
        sat,
        proceeds,
        basis,
        gain,
        term,
        basis_source: BasisSource::ExchangeProvided,
        gift_zone: None,
        acquired_at: acquired,
        lot_acquired_at: acquired,
        wallet: WalletId::SelfCustody {
            label: "cold".into(),
        },
        pseudo: false,
    };
    let state = LedgerState {
        disposals: vec![
            Disposal {
                event: EventId::decision(2),
                kind: DisposeKind::Sell,
                disposed_at: date!(2024 - 03 - 10),
                legs: vec![leg(
                    dec!(60000),
                    dec!(22000),
                    dec!(38000),
                    Term::LongTerm,
                    date!(2021 - 02 - 01),
                    50_000_000,
                )],
                fee_mini_disposition: false,
            },
            Disposal {
                event: EventId::decision(4),
                kind: DisposeKind::Sell,
                disposed_at: date!(2024 - 09 - 02),
                legs: vec![leg(
                    dec!(9000),
                    dec!(7000),
                    dec!(2000),
                    Term::ShortTerm,
                    date!(2024 - 01 - 05),
                    10_000_000,
                )],
                fee_mini_disposition: false,
            },
        ],
        ..Default::default()
    };
    answer_all_live_declarations(&mut ri);
    reconcile_digital_asset_activity(&mut ri, &state, ASSEMBLY_YEAR);
    (ri, state)
}

/// Assemble the owner's profile at [`ASSEMBLY_YEAR`] — the reachable, bundled year.
fn baseline(carryover: Usd) -> AbsoluteReturn {
    let (ri, state) = owner_shape_household(carryover);
    let ft = btctax_adapters::tax_tables::BundledFullReturnTables::load();
    let params = ft
        .full_return_for(ASSEMBLY_YEAR)
        .expect("TY2024's FullReturnParams are the bundled ones")
        .clone();
    let tt = btctax_adapters::tax_tables::BundledTaxTables::load();
    let table = tt
        .table_for(ASSEMBLY_YEAR)
        .expect("TY2024 TaxTable")
        .clone();
    assemble_absolute(&ri, &state, &params, &table, ASSEMBLY_YEAR)
}

/// The printed Schedule A the chain produces when `params` decide the parts, with no carryover.
fn sch_a_at(params: &FullReturnParams) -> (ScheduleAParts, ScheduleALines) {
    sch_a_with_carryover(params, Usd::ZERO)
}

/// ★★ Substitution W5-1's second half, and the only way past Tier A's A-1 wall without editing shipped
/// source. `AbsoluteReturn::schedule_a` is the one field `schedule_a_lines` reads for every Schedule A
/// cell; the AGI the parts are figured at is the engine's own, taken from the same assembled return,
/// so two runs differ in **nothing but the parameters**.
fn sch_a_with_carryover(
    params: &FullReturnParams,
    carryover: Usd,
) -> (ScheduleAParts, ScheduleALines) {
    let mut ar = baseline(carryover);
    let (ri, _) = owner_shape_household(carryover);
    let gifts = ri
        .schedule_a
        .as_ref()
        .map(|a| a.charitable.clone())
        .unwrap_or_default();
    let charitable = apply_170b(ar.agi, &gifts, &ri.charitable_carryover_in, ASSEMBLY_YEAR);
    let parts = schedule_a_parts(&ri, ar.agi, &charitable, params).expect("the fixture itemizes");
    ar.schedule_a = Some(parts.clone());
    let lines = schedule_a_lines(&ar, round_dollar(ar.agi)).expect("the fixture itemizes");
    (parts, lines)
}

/// The fixture really is the owner's shape, and it really reaches Schedule A.
#[test]
fn the_owner_profile_itemizes_and_files_schedule_b() {
    let ar = baseline(Usd::ZERO);
    assert!(
        ar.deduction_is_itemized,
        "the profile must ITEMIZE or Schedule A is never filed"
    );
    assert_eq!(ar.wages, dec!(220000));
    assert_eq!(ar.taxable_interest, dec!(4200));
    assert_eq!(ar.ordinary_dividends, dec!(9000));
    assert_eq!(
        ar.agi,
        dec!(273200),
        "wages + interest + dividends + $40,000 net gain"
    );
    assert!(
        ar.taxable_interest > dec!(1500) && ar.ordinary_dividends > dec!(1500),
        "Schedule B files on both halves"
    );
    let (_, state) = owner_shape_household(Usd::ZERO);
    assert_eq!(
        btctax_core::forms::form_8949(&state, ASSEMBLY_YEAR).len(),
        2,
        "two Bitcoin disposals"
    );
}

/// A TY2025 parameter set for the control — substitution W5-2. Only line 5e's figures differ from
/// TY2026's, and they are **parsed from the 2025 extract**, not typed.
fn ty2025_salt_control() -> FullReturnParams {
    let f25 = salt_5e_figures(&extract(SA_2025));
    let mut p = ty2026_full_return();
    let SaltLimitation::Worksheet2025 {
        ref mut line1_cap,
        ref mut line5_threshold,
        ref mut line5_threshold_mfs,
        ..
    } = p.salt
    else {
        panic!("TY2026's SALT limitation is the worksheet shape")
    };
    *line1_cap = f25[0];
    *line5_threshold = f25[2];
    *line5_threshold_mfs = f25[3];
    p
}

/// ★★★ **THE DISCRIMINATOR. A harness that cannot tell 5e from 13/14 is not measuring anything.**
///
/// One fixture, one AGI, three parameter sets. Line 5e — whose NUMBER did not move — produces three
/// different cells. The charity block — whose numbers DID move — is identical to the cent in all
/// three. So the instrument reads parameters, and the collisions are invisible to parameters by
/// construction.
#[test]
fn the_control_moves_and_the_charity_block_does_not() {
    let ft = btctax_adapters::tax_tables::BundledFullReturnTables::load();
    let ty2024 = ft.full_return_for(ASSEMBLY_YEAR).expect("bundled").clone();
    let (_, l24) = sch_a_at(&ty2024);
    let (_, l25) = sch_a_at(&ty2025_salt_control());
    let (_, l26) = sch_a_at(&ty2026_full_return());

    // 5d is above every cap, so 5e IS the cap in all three.
    assert_eq!(l24.line5d, dec!(45500));
    assert_eq!(
        (l24.line5e, l25.line5e, l26.line5e),
        (dec!(10000), dec!(40000), dec!(40400))
    );
    assert_eq!(
        (l24.line7, l25.line7, l26.line7),
        (dec!(10000), dec!(40000), dec!(40400))
    );

    // …and the six collisions do not move a cent between the three.
    for (a, b) in [(&l24, &l25), (&l25, &l26)] {
        assert_eq!(
            (a.line11, a.line12, a.line13, a.line14),
            (b.line11, b.line12, b.line13, b.line14)
        );
        assert_eq!(a.line18_elects_smaller, b.line18_elects_smaller);
    }
    assert_eq!(
        (l26.line11, l26.line12, l26.line13, l26.line14),
        (dec!(6000), Usd::ZERO, Usd::ZERO, dec!(6000)),
        "the charity block under TY2026 PARAMS, on the TY2025 line SET"
    );
    // Only the 5e delta reaches the total, which is the point: the total tracks the parameter and is
    // blind to the renumber.
    assert_eq!(l26.line17 - l24.line17, l26.line5e - l24.line5e);
}

/// ★★★ **COLLISIONS 13 AND 14 ARE A SWAP OF QUANTITIES — A WRONG FIGURE IN EACH CELL**, priced on a
/// filer WITH a prior-year carryover, which is where the brief's adverse hypothesis lives.
///
/// The chain puts the **carryover** in 13 and the **whole charity block** in 14. TY2026's 13 is the
/// *Charitable Contribution Limitation Worksheet*'s line 6 — a current-year quantity — and its 14 is
/// the carryover. So:
///
/// * cell **14** prints `$10,000` where the carryover `$4,000` belongs — **overstated by $6,000**, the
///   current-year gift total laundered under the heading *"Carryover from prior year"*;
/// * cell **13** prints `$4,000` where the worksheet's current-year figure belongs;
/// * cell **15**, *"Add lines 13 and 14"*, has no field and prints nothing — and `13 + 14` as the chain
///   fills them is `$14,000`, double-counting the carryover.
#[test]
fn collisions_13_and_14_are_a_swap_of_quantities_not_of_labels() {
    let carry = dec!(4000);
    let (parts, l) = sch_a_with_carryover(&ty2026_full_return(), carry);

    // What the engine computed, by name.
    assert_eq!(parts.charitable_cash_11, dec!(6000));
    assert_eq!(parts.charitable_noncash_12, Usd::ZERO);
    assert_eq!(parts.charitable_carryover_13, carry);
    assert_eq!(parts.charitable_14, dec!(10000));

    // Where the printed chain put it.
    assert_eq!(l.line13, carry, "cell 13 holds the CARRYOVER");
    assert_eq!(l.line14, dec!(10000), "cell 14 holds the TOTAL");

    // What TY2026 asks of those cells.
    let c26 = captions(&extract(SA_2026));
    assert_eq!(c26["14"], "Carryover from prior year");
    assert!(c26["13"].starts_with("Enter the amount from line 6 of the Charitable Contribution"));
    assert_eq!(c26["15"], "Add lines 13 and 14");

    // The size and direction of each cell's error, as arithmetic.
    assert_eq!(l.line14 - carry, dec!(6000), "cell 14 overstated by $6,000");
    assert_eq!(
        l.line13 + l.line14,
        dec!(14000),
        "★ 13 + 14 — which TY2026's line 15 adds — DOUBLE-COUNTS the carryover: $4,000 + $10,000 where \
         the schedule's own 15 should read $10,000. The chain has no 15, so this surfaces the moment a \
         TY2026 field map is written."
    );
}

/// ★★ **COLLISION 15 — nothing to print, and it is the line the 2026 total is figured from.**
#[test]
fn collision_15_has_no_field_and_the_total_depends_on_it() {
    let c26 = captions(&extract(SA_2026));
    assert_eq!(c26["15"], "Add lines 13 and 14");
    assert!(
        extract(SA_2026).contains("Add the amounts in far-right column for lines 4"),
        "2026's 18 adds the far-right column from line 4 up, which includes 15"
    );
    let (_, l) = sch_a_with_carryover(&ty2026_full_return(), dec!(4000));
    assert!(!cells_the_struct_expresses(&l).contains_key("15"));
    // The chain's line 14 IS the quantity 2026's line 15 wants — on the wrong number, with 13 and 14
    // both already claiming a different meaning underneath it.
    assert_eq!(l.line14, dec!(10000));
}

/// ★★★ **COLLISION 17 — a WRONG FIGURE of the whole itemized total.** The chain's `line17` is the
/// TOTAL (`4 + 7 + 10 + 14`) and feeds 1040 line 12; TY2026's 17 is the *"Other itemized deductions"*
/// parent, which this filer has none of.
#[test]
fn collision_17_puts_the_whole_total_where_other_itemized_deductions_belong() {
    let (parts, l) = sch_a_at(&ty2026_full_return());
    assert_eq!(l.line17, parts.total_17);
    assert_eq!(l.line17, l.line4 + l.line7 + l.line10 + l.line14);
    assert_eq!(
        l.line17,
        dec!(65400),
        "0 medical + 40,400 SALT + 19,000 interest + 6,000 charity"
    );

    let c26 = captions(&extract(SA_2026));
    assert_eq!(c26["17"], "Other itemized deductions (see instructions).");
    assert_eq!(c26["17z"], "Add lines 17a through 17k");
    // This filer has no gambling losses, no disaster loss, no claim-of-right repayment: 17a–17z are
    // CORRECTLY blank. That is what makes putting the total there a figure error and not a gap.
    assert_eq!(
        l.line4,
        Usd::ZERO,
        "medical is under the 7.5% floor on this fixture"
    );
}

/// ★★★ **COLLISION 18 — a BOOLEAN in the cell that holds the total.** FR-185's *"worst shape
/// available"*, measured: the chain's only `18` is `line18_elects_smaller: bool`, and it has no `19`.
#[test]
fn collision_18_is_a_checkbox_where_ty2026_prints_the_total() {
    let (_, l) = sch_a_at(&ty2026_full_return());
    // On this fixture itemizing genuinely WON, so the §63(e) election is correctly not made.
    assert!(!l.line18_elects_smaller);
    let ar = baseline(Usd::ZERO);
    assert!(ar
        .itemized_deduction
        .is_some_and(|it| it > ar.standard_deduction));

    let expressed = cells_the_struct_expresses(&l);
    assert_eq!(expressed.get("18"), Some(&Kind::Check));
    assert_eq!(expressed.get("19"), None, "the chain has no 19 at all");
    let t26 = extract(SA_2026);
    assert!(captions(&t26)["18"].starts_with("Is the amount on Form 1040"));
    assert!(caption_block(&t26, "19").contains("check this box"));
}

/// ★ **COLLISION 16 — a LABEL move, plus a widening a move table would miss (FR-187).** Casualty is
/// the one of the six that moves a cell this filer legitimately leaves blank, so on THIS household it
/// costs no figure. What it costs is eligibility: the 2026 sentence widened.
#[test]
fn collision_16_is_a_label_move_on_this_household_but_the_rule_widened() {
    let (_, l) = sch_a_at(&ty2026_full_return());
    let expressed = cells_the_struct_expresses(&l);
    assert!(!expressed.contains_key("15") && !expressed.contains_key("16"));
    let c25 = captions(&extract(SA_2025));
    let c26 = captions(&extract(SA_2026));
    assert!(c25["15"].contains("from a federally declared disaster"));
    assert!(c26["16"].contains("from a federally or state-declared disaster"));
    assert!(
        !c26["16"].contains("from a federally declared disaster"),
        "the 2026 clause is not the 2025 clause with a new number"
    );
}

/// ★★★ **THE BOTTOM LINE: the renumber alone does NOT move the total — a new FLOOR does, and the
/// direction is UNDERSTATEMENT.**
///
/// The chain's total is `4 + 7 + 10 + (11 + 12 + 13)`. TY2026's is `4 + 7 + 10 + 15 + 16 + 17z` with
/// `15 = 13 + 14`. Under the 2026 meanings — 13 the worksheet's current-year figure, 14 the carryover —
/// those two sums are the SAME quantity **if and only if** the *Charitable Contribution Limitation
/// Worksheet*'s line 6 equals the §170(b)-limited current-year total `charitable.rs` computes.
///
/// ★★ It does not, and the reason is statute rather than layout: Pub. L. 119-21 §70425 adds **§170(p)**,
/// a **0.5%-of-AGI floor** on an itemizer's charitable contributions for tax years beginning after
/// 2025 — already recorded in this repo at `design/OWNER_DECISIONS_2026-09-04.md:212`,
/// `design/direction/filing-readiness-lens-itemized.md:182` and
/// `design/agent-reports/RECON-schedule-a-ty2026.md:385`. A limitation worksheet appearing in the same
/// revision that stops summing gifts straight into the subtotal is where such a floor lands.
/// `charitable.rs` applies no floor, so the deduction is **overstated** and the tax **understated**.
///
/// This test states the floor's SIZE on the owner's profile and asserts nothing about the worksheet's
/// shape, which is unarchived.
#[test]
fn the_renumber_leaves_the_total_alone_but_the_new_charitable_floor_does_not() {
    let (_, l) = sch_a_with_carryover(&ty2026_full_return(), dec!(4000));
    let ar = baseline(dec!(4000));

    // The identity that makes the renumber alone harmless to the total.
    assert_eq!(
        l.line17,
        l.line4 + l.line7 + l.line10 + (l.line11 + l.line12 + l.line13)
    );
    assert_eq!(l.line14, l.line11 + l.line12 + l.line13);

    // The floor's size on this household — 0.5% of the engine's own AGI.
    let floor = dec!(0.005) * ar.agi;
    assert_eq!(ar.agi, dec!(273200));
    assert_eq!(floor, dec!(1366));
    assert!(
        floor < l.line14,
        "the floor bites: it is smaller than the charitable total it would reduce, so the whole \
         $1,366 is deduction btctax claims and §170(p) does not allow"
    );
}

/// Scan every `.rs` file under `crates/` (this harness excepted — it names the statute itself) for any
/// of `needles`, returning `<path>: <needle>` for each hit.
fn scan_crates(needles: &[&str]) -> Vec<String> {
    let mut hits = Vec::new();
    let mut stack = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).expect("crates/ is readable") {
            let e = e.expect("a readable dir entry");
            let p = e.path();
            if p.is_dir() {
                if !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("target"))
                {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs")
                && !p.file_name().is_some_and(|n| n == "ty2026_schedule_a.rs")
            {
                let t = std::fs::read_to_string(&p).unwrap_or_default();
                for needle in needles {
                    if t.contains(needle) {
                        hits.push(format!("{}: {needle}", p.display()));
                    }
                }
            }
        }
    }
    hits
}

/// ★★ **B1 for [`scan_crates`] — the POSITIVE control, and the reason the wall pin below is not
/// vacuous.** A scanner that walked the wrong directory, or read zero files, would report "no hits" and
/// look exactly like a clean tree. So it is first asked for something that is unquestionably present.
#[test]
fn the_source_scanner_actually_reads_files() {
    let present = scan_crates(&["§170(b)"]);
    assert!(
        present.len() >= 3,
        "the scanner must find the §170(b) ceilings it is pointed at; got {present:?}"
    );
    assert!(
        present.iter().any(|h| h.contains("charitable.rs")),
        "and `charitable.rs` must be among them: {present:?}"
    );
    assert!(
        scan_crates(&["a-string-no-source-file-contains-zzz"]).is_empty(),
        "…and it must find nothing when there is nothing"
    );
}

/// ★★ **WALL PIN: nothing in the workspace applies a charitable floor or a §68-style itemized
/// limitation.** Measured over the source rather than asserted from a doc — and it REDS the day either
/// lands, which is when every figure above must be re-run.
#[test]
fn no_charitable_floor_and_no_itemized_limitation_exists_anywhere() {
    let hits = scan_crates(&[
        "170(p)",
        "charitable_floor",
        "itemized_limit",
        "sec_68",
        "pease",
    ]);
    assert!(
        hits.is_empty(),
        "a floor or §68-style limitation now exists — re-run every figure in \
         `design/agent-reports/REPORT-wave5-schedule-a.md`:\n{}",
        hits.join("\n")
    );
}

/// ★★ **The label census for Schedule A is quoted at ONE year, and it is 2024.** So the census cannot
/// notice a 2026 renumber on its own; what reds is `xtask line-coverage`'s
/// `MAX_UNQUOTED_BUNDLED_YEARS` ratchet (12 today) the moment `forms/2026/f1040sa.map.toml` is
/// bundled. That ratchet names the `(form, year)` pair — it does not say which lines moved, and the
/// only way to satisfy it for one struct serving two revisions is FR-162's revision axis.
#[test]
fn the_schedule_a_label_census_is_quoted_at_a_single_year() {
    let (_, l) = sch_a_at(&ty2026_full_return());
    let cov = btctax_core::tax::line_coverage::cover_schedulealines(&l);
    assert!(!cov.0.is_empty());
    let years: BTreeSet<&str> = cov.0.iter().map(|r| r.year).collect();
    assert_eq!(
        years,
        BTreeSet::from(["2024"]),
        "one struct, one quoted revision — so a 2026 caption change is checked against a 2024 booklet"
    );
    let by_line: BTreeMap<&str, &str> = cov
        .0
        .iter()
        .map(|r| (r.line.as_str(), r.instruction))
        .collect();
    assert_eq!(by_line.get("13"), Some(&"Carryover from prior year"));
    assert_eq!(by_line.get("14"), Some(&"Add lines 11 through 13"));
    assert_eq!(
        by_line.get("17"),
        Some(&"Add the amounts in the far right column for lines 4 through 16.")
    );
    assert_eq!(by_line.get("15"), None, "no 15 is covered at all");
    assert_eq!(by_line.get("16"), None, "and no 16 either");
}
