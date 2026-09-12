//! §G-6 / design r2 §10 step 5 — **the OBBBA-era Form 6251 (Part I line 1 split into 1a/1b), held
//! against the form itself.**
//!
//! ## What is under test and what is NOT
//!
//! `LineSet::F6251_2025` was `Schema::Unwired` until 2026-09-11. It is wired now, and the honest
//! statement of why is in `Form6251ObbbaMap`'s own doc comment: **no packet reaches
//! this revision and for TY2025 none ever will** (owner ruling 2026-09-11 — TY2025 is not filed with
//! this software; `full_return_for(2025)` is a tested `None`). Its value is that `xtask form-delta`
//! measures the TY2025 → TY2026 AcroForm delta as **62 fields, 0 renamed, 0 moved**, so this is
//! TY2026's field map and emitter, written against a FINAL document a season early and exercised
//! here through the bundled TY2025 PDF. So the tests below are the *whole* exercise of this code
//! path; there is no packet-level backstop behind them.
//!
//! ## ★★★ The defect this file exists to catch
//!
//! Because every field name is identical across the two revisions, **a single struct carrying one
//! year's cross-reference would fill the other year's PDF and nothing would red.** The AMT base would
//! be taken from the wrong line of Schedule 1-A (line **37** in TY2025, line **43** in TY2026's
//! draft). That is the Form 6251 line-33 defect one line up: a cross-reference transcribed one line
//! off, which on that occasion inflated the tentative minimum tax by $200,000 on one vector. So the
//! year-varying cells live per revision in `btctax_forms::f6251_revision`, the compiler refuses a new
//! revision that states none, and [`the_year_varying_cells_are_verbatim_in_that_revisions_own_extract`]
//! refuses one that states another revision's.
//!
//! ## The line set is DERIVED FROM THE EXTRACT
//!
//! `CLAUDE.md`: *"a conformance KAT must enumerate the expected line set FROM the form's extracted
//! text, never from a range or a hand-written list."* [`printed_labels`] does exactly that, and
//! [`every_printed_line_is_accounted_for_in_its_own_revisions_map`] requires every printed label to be
//! either MAPPED or CENSUSED — never merely absent, because a blank the inputs chose and a blank
//! nobody ever populated are identical on paper.

use btctax_core::conventions::Usd;
use btctax_core::tax::form6251::{Form6251, Form6251Line1};
use btctax_forms::bundled::{map_text, Stem};
use btctax_forms::line_set::{schema, LineSet, Schema};
use btctax_forms::testonly::{
    collect_fields, f6251_pdf, fill_form_6251_obbba_with_map, index, load, parses_into_its_schema,
    text_value, Form6251Map, Form6251ObbbaMap,
};
use rust_decimal_macros::dec;
use std::collections::BTreeSet;
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────────────────────────
//  MEASUREMENT — every set below comes off the archived extract or out of the crate's own API.
// ──────────────────────────────────────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/btctax-forms -> workspace root")
        .to_path_buf()
}

fn extract_text(irs_stem: &str, year: i32) -> String {
    let p = workspace_root()
        .join("design/forms/extract")
        .join(format!("{irs_stem}--{year}.txt"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!(
            "the archived extract must be readable at {}: {e}",
            p.display()
        )
    })
}

/// ★ The shared normalisation: collapse whitespace runs (`pdftotext -layout` wraps mid-sentence) and
/// **drop standalone brace glyphs**.
///
/// The braces are not cosmetic. The form draws large `{`/`}` to group its bracketed rows and
/// `pdftotext` emits them as lone tokens INSIDE a sentence — line 6 comes out as *"…on lines 7, 9,
/// and } 11, and go to line 10"*. Without the filter a `contains` REJECTS the faithful quote and
/// ACCEPTS a truncated one ending *"…lines 7, 9, and"*, silently dropping **11** from the zero-out
/// set. Identical to `f6251_map.rs::norm` and `xtask::line_coverage_check::normalize` on purpose:
/// when two authorities on "is this quote verbatim?" normalised differently, the weaker one was
/// satisfied by a degraded citation and reported success.
fn norm(s: &str) -> String {
    s.split_whitespace()
        .filter(|t| *t != "{" && *t != "}")
        .collect::<Vec<_>>()
        .join(" ")
}

/// ★★★ **Every numbered line label the form PRINTS, read off the extract's own label column.**
///
/// The form prints its line number in a left-hand column and continues a long instruction on
/// following rows indented past it; a sub-line prints a bare letter (`b`, `c`, …) which belongs to
/// the last numbered parent (`2a` then `b` ⇒ `2b`).
///
/// ★★ **The label column is MEASURED, not typed.** `lo` is the leftmost indent at which any
/// label-shaped token appears in this file, and the column block is `lo..=lo+4` — five columns,
/// because the form right-aligns one- against two-digit numbers and indents sub-letters a little
/// further. Measured on all three archived revisions of this form: `f6251--2024` uses {1, 3, 5},
/// `f6251--2025` {1, 3, 4, 5}, `f6251--2026-DRAFT` {23, 25} — the draft's whole page is shifted 22
/// columns right, which is exactly why the threshold is derived from the file rather than fixed.
///
/// ★★★ **What this does NOT cover, stated rather than hidden:** a wrapped continuation row whose
/// first token happens to be label-shaped is rejected by the column window and nothing else. There
/// is exactly one on each revision of this form — extract row 67 of `f6251--2025` begins *"16 of
/// Schedule D (Form 1040)…"*, the tail of line 7's bullet, at column 10. If a future revision printed
/// a real label outside the window it would be DROPPED from this set, and that fails closed rather
/// than passing: the map would then account for a label the extract does not print, and
/// [`every_printed_line_is_accounted_for_in_its_own_revisions_map`] reds on that side of the
/// comparison.
fn printed_labels(extract: &str) -> Vec<String> {
    let candidate = |line: &str| -> Option<(usize, String)> {
        let indent = line.len() - line.trim_start().len();
        let tok = line.split_whitespace().next()?;
        // A label is either a bare sub-line letter (`b`, `c`, …) or one-to-two digits with at most
        // one trailing lowercase letter (`3`, `10`, `1a`, `2t`) — the form's own numbering, and
        // nothing wider. `16,` (a wrapped clause) and `2025` (the year) both fail it.
        let digits = tok.chars().take_while(char::is_ascii_digit).count();
        let tail = &tok[digits..];
        let label_shaped = if digits == 0 {
            tok.len() == 1 && tok.chars().all(|c| c.is_ascii_lowercase())
        } else {
            (1..=2).contains(&digits)
                && tail.len() <= 1
                && tail.chars().all(|c| c.is_ascii_lowercase())
        };
        label_shaped.then(|| (indent, tok.to_string()))
    };
    let rows: Vec<(usize, String)> = extract
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(candidate)
        .collect();
    let lo = rows
        .iter()
        .map(|(c, _)| *c)
        .min()
        .expect("the extract prints at least one numbered line");
    let mut out = Vec::new();
    let mut parent: Option<String> = None;
    for (col, tok) in rows {
        if col > lo + 4 {
            continue; // a wrapped continuation row, not the label column — see the doc comment
        }
        if tok.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            parent = Some(tok.chars().take_while(char::is_ascii_digit).collect());
            out.push(tok);
        } else if let Some(p) = &parent {
            out.push(format!("{p}{tok}"));
        }
    }
    out
}

/// Every `lineNN = "…"` binding in a map's text, as (printed label, FQN).
fn mapped(map: &str) -> Vec<(String, String)> {
    map.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let n = k.trim().strip_prefix("line")?;
            (!n.is_empty() && n.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .then(|| (n.to_string(), v.trim().trim_matches('"').to_string()))
        })
        .collect()
}

/// Every `line = "…"` a `[census]` entry names — the labels a decision leaves blank ON PURPOSE.
fn censused_labels(map: &str) -> BTreeSet<String> {
    map.lines()
        .filter_map(|l| l.split_once("line = \""))
        .filter_map(|(_, r)| r.split_once('"').map(|(v, _)| v.to_string()))
        .collect()
}

/// Every FQN a `[census]` entry keys on.
fn censused_fqns(map: &str) -> BTreeSet<&str> {
    map.lines()
        .filter(|l| l.starts_with("\"topmostSubform"))
        .filter_map(|l| l.split('"').nth(1))
        .collect()
}

/// ★★★ **Every revision of Form 6251 this build knows, DERIVED from the revision set** — never a
/// list of years, so a third revision is covered the day it is bound.
///
/// Keyed on the revision NAME's stem (`f6251/…`) rather than on a pair of `Schema` variants: the two
/// structs serving this form today ([`Form6251Map`], [`Form6251ObbbaMap`]) would be a two-element
/// hand-list beside a set that grows, which is the failure mode `CLAUDE.md` names.
fn every_f6251_revision() -> Vec<(LineSet, i32)> {
    let mut out: Vec<(LineSet, i32)> = LineSet::ALL
        .iter()
        .filter(|ls| ls.as_str().starts_with("f6251/"))
        .map(|ls| {
            let year = ls
                .as_str()
                .split_once('/')
                .and_then(|(_, y)| y.parse().ok())
                .expect("a revision name is <stem>/<year>");
            (*ls, year)
        })
        .collect();
    out.sort_by_key(|(_, y)| *y);
    assert!(
        out.len() >= 2,
        "this build must know at least the TY2024 and OBBBA revisions; found {out:?}"
    );
    out
}

/// The revisions on the OBBBA field map, derived from the ONE schema match.
fn obbba_revisions() -> Vec<(LineSet, i32)> {
    let out: Vec<(LineSet, i32)> = every_f6251_revision()
        .into_iter()
        .filter(|(ls, _)| schema(*ls) == Schema::Form6251ObbbaMap)
        .collect();
    assert!(
        !out.is_empty(),
        "no revision is wired to Form6251ObbbaMap — this whole file would be green having tested \
         nothing, which is the blind-instrument class it is written against"
    );
    out
}

// ──────────────────────────────────────────────────────────────────────────────────────────────────
//  PARSE — the revision that `deny_unknown_fields` was switched on for
// ──────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ **Kill 1, both directions.** The TY2025 map parses into the OBBBA struct, and the TY2024
/// struct REFUSES it by name.
///
/// This map is the reason every map struct carries `#[serde(deny_unknown_fields)]`: before that
/// attribute, `Form6251Map` discarded its `line1a` and `line1b` without a word and only the *absence*
/// of `line1` was loud — a vanished line failed closed and a RENAMED line half-vanished. The refusal
/// asserted below is that attribute doing its job, and it is asserted to NAME the field, because a
/// refusal that does not say what it refused sends the next reader hunting for a typo.
#[test]
fn the_obbba_map_parses_into_its_own_struct_and_the_ty2024_struct_refuses_it() {
    for (ls, year) in obbba_revisions() {
        let text = map_text(Stem::F6251, year).unwrap_or_else(|| {
            panic!(
                "{} names year {year}, whose map must be bundled",
                ls.as_str()
            )
        });

        Form6251ObbbaMap::parse(text)
            .unwrap_or_else(|e| panic!("{} must parse into Form6251ObbbaMap: {e}", ls.as_str()));

        let refused = Form6251Map::parse(text)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "{}: the TY2024 struct ACCEPTED the 1a/1b map. It prints a single line 1, so \
                     accepting this map means `deny_unknown_fields` is gone and line 1a has been \
                     silently discarded off a filed form.",
                    ls.as_str()
                )
            })
            .to_string();
        assert!(
            refused.contains("line1a"),
            "{}: the TY2024 struct's refusal must NAME the unknown field so the next reader is not \
             hunting a typo; got {refused:?}",
            ls.as_str()
        );

        // …and the derived dispatch agrees: the revision parses into the struct its own row names.
        assert_eq!(
            parses_into_its_schema(ls),
            Some(Ok(())),
            "{} must parse into the struct its `line_set` names",
            ls.as_str()
        );
    }
}

/// ★★ Kill 2 — a map missing a REQUIRED row field is a parse refusal, not a map with a default.
///
/// Mutated in memory, never on disk: the row keys are what the year-package table is *made of*, and a
/// default here would mean a bundled PDF nobody hash-pinned.
#[test]
fn a_map_missing_a_required_row_field_is_refused() {
    for (ls, year) in obbba_revisions() {
        let text = map_text(Stem::F6251, year).unwrap();
        for required in [
            "irs_stem",
            "versioning",
            "template_sha256",
            "instructions",
            "line_set",
        ] {
            let mutated: String = text
                .lines()
                .filter(|l| !l.trim_start().starts_with(required))
                .collect::<Vec<_>>()
                .join("\n");
            let e = Form6251ObbbaMap::parse(&mutated)
                .err()
                .unwrap_or_else(|| {
                    panic!(
                        "{}: a map with no `{required}` PARSED — a required row key silently \
                         defaulted",
                        ls.as_str()
                    )
                })
                .to_string();
            assert!(
                e.contains(required),
                "{}: dropping `{required}` must be refused BY NAME; got {e:?}",
                ls.as_str()
            );
        }
        // …and a mapped money line is equally required: `line1a` is the revision's whole identity.
        let no_1a: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("line1a"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            Form6251ObbbaMap::parse(&no_1a).is_err(),
            "{}: a map with no `line1a` must be refused — line 1a is the box this revision added",
            ls.as_str()
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────────────────────────
//  THE FIELD ASSIGNMENT — against the bundled PDF's own AcroForm
// ──────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ Every mapped FQN EXISTS on the blank form, and mapped ∪ censused ∪ identity **partitions**
/// the AcroForm.
///
/// A typo'd field name is invisible to a fill (`lopdf` writes nothing), so a whole line would
/// silently vanish from a filed return. And the partition is a disjoint union rather than a COUNT:
/// a count is satisfied by any bijection-shaped mistake — shifting the `3..11` block down one widget
/// keeps every count and every existence check green while printing line 3's figure in line 2t's box.
#[test]
fn every_mapped_field_exists_and_the_three_sets_partition_the_acroform() {
    for (ls, year) in obbba_revisions() {
        let text = map_text(Stem::F6251, year).unwrap();
        let doc = load(f6251_pdf(year).expect("the crate ships this year's f6251.pdf")).unwrap();
        let fields = collect_fields(&doc).unwrap();
        let present: BTreeSet<&str> = fields.iter().map(|f| f.fqn.as_str()).collect();

        let m = mapped(text);
        for (line, fqn) in &m {
            assert!(
                present.contains(fqn.as_str()),
                "{}: line {line} maps to {fqn}, which is NOT a field on the blank form",
                ls.as_str()
            );
        }
        let mapped_fqns: BTreeSet<&str> = m.iter().map(|(_, f)| f.as_str()).collect();
        assert_eq!(
            mapped_fqns.len(),
            m.len(),
            "{}: no FQN may be mapped to two lines — the values collapse into one /V",
            ls.as_str()
        );

        let censused = censused_fqns(text);
        let overlap: Vec<&&str> = mapped_fqns.intersection(&censused).collect();
        assert!(
            overlap.is_empty(),
            "{}: {overlap:?} are BOTH mapped and censused — a line pointed at a widget the map also \
             declares carries nothing. This is the shift a count cannot see.",
            ls.as_str()
        );

        // Keyed on `name`/`ssn`, NOT on "everything after `[identity]`": the census section FOLLOWS
        // it in the file (the header must come last so TOML does not swallow the `lineNN` keys), so a
        // positional read picks up every census FQN as identity and the partition silently widens.
        let identity: BTreeSet<&str> = text
            .lines()
            .filter(|l| {
                let k = l.split('=').next().unwrap_or("").trim();
                k == "name" || k == "ssn"
            })
            .filter_map(|l| l.split('"').nth(1))
            .collect();
        assert_eq!(identity.len(), 2, "{}: name + SSN", ls.as_str());

        let accounted: BTreeSet<&str> = mapped_fqns
            .union(&censused)
            .copied()
            .collect::<BTreeSet<&str>>()
            .union(&identity)
            .copied()
            .collect();
        let unaccounted: Vec<&str> = present.difference(&accounted).copied().collect();
        assert!(
            unaccounted.is_empty(),
            "{}: {unaccounted:?} exist on the blank form but are neither mapped, censused, nor \
             identity — a widget no decision reaches, which is invisible on the printed page and to \
             both oracles",
            ls.as_str()
        );
        assert_eq!(
            accounted.len(),
            fields.len(),
            "{}: mapped ∪ censused ∪ identity must be exactly the AcroForm",
            ls.as_str()
        );
    }
}

/// ★★★ **THE 1a INSET COLUMN and the inset trio** — the corroboration that makes the page-1
/// assignment more than an in-order zip.
///
/// Page 1's field names are not a uniform offset. Two independent geometric facts pin them:
///
/// 1. `line1a`'s widget is the ONLY money box outside the right-hand amount column (measured
///    x=[410.4, 481.6] against [504, 576]), because the form prints 1a in the inner column as a
///    sub-total feeding 1b. No other line on this form has that geometry.
/// 2. The form prints exactly THREE parenthesised boxes — 2b, 2f, 2s — and the PDF has exactly three
///    narrower widgets (w=64 against w=72). Under this assignment the two sets coincide; under an
///    off-by-N they do not. ★ These are NOT the TY2024 widgets (there the trio was
///    `f1_5`/`f1_9`/`f1_22`), which is why the trio is identified by WIDTH here rather than by a
///    carried-over name list.
#[test]
fn line_1a_sits_in_its_own_column_and_the_inset_trio_lands_on_the_parenthesised_lines() {
    for (ls, year) in obbba_revisions() {
        let text = map_text(Stem::F6251, year).unwrap();
        let map = Form6251ObbbaMap::parse(text).unwrap();
        let doc = load(f6251_pdf(year).unwrap()).unwrap();
        let fields = collect_fields(&doc).unwrap();
        let rect = |fqn: &str| -> [f32; 4] {
            fields
                .iter()
                .find(|f| f.fqn == fqn)
                .and_then(|f| f.rect)
                .unwrap_or_else(|| panic!("no rect for {fqn}"))
        };

        let r1a = rect(map.line1a.fields()[0]);
        let r1b = rect(map.line1b.fields()[0]);
        assert!(
            r1a[2] < 500.0,
            "{}: line 1a must sit in the INNER column (measured x2 = 481.6); got {r1a:?}",
            ls.as_str()
        );
        assert!(
            r1b[0] >= 500.0,
            "{}: line 1b is in the right-hand amount column; got {r1b:?}",
            ls.as_str()
        );
        // ★ …and 1a is still ABOVE 1b, so the column change does not break the descent the read-back
        //   verifies. Measured centre-y: 1a 654.0, 1b 630.0.
        assert!(
            (r1a[1] + r1a[3]) / 2.0 > (r1b[1] + r1b[3]) / 2.0,
            "{}: line 1a must print above line 1b",
            ls.as_str()
        );

        let width = |fqn: &str| -> f32 {
            let r = rect(fqn);
            r[2] - r[0]
        };
        let inset: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split('"').nth(1))
            .filter(|f| f.contains("Page1") && f.contains("f1_"))
            .filter(|f| width(f) < 70.0)
            .collect();
        assert_eq!(
            inset.len(),
            3,
            "{}: the form prints exactly three parenthesised boxes; got {inset:?}",
            ls.as_str()
        );
        // The MAPPED one of the three is line 2b; 2f and 2s are inside the censused 2c-2t range.
        assert!(
            inset.contains(&map.line2b.fields()[0]),
            "{}: line 2b is one of the three inset (parenthesised) boxes",
            ls.as_str()
        );
        let censused = censused_fqns(text);
        assert_eq!(
            inset.iter().filter(|f| censused.contains(**f)).count(),
            2,
            "{}: the other two inset boxes are 2f and 2s, which are censused",
            ls.as_str()
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────────────────────────
//  CONFORMANCE — the line set derived from the extract, and every quote verbatim
// ──────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ **Kill 5 — the printed line set is ENUMERATED FROM THE EXTRACT, and every label is accounted
/// for.**
///
/// `CLAUDE.md`: *"a conformance KAT must (a) enumerate the expected line set FROM the form's extracted
/// text, never from a range or a hand-written list, and (b) require every line to be accounted for —
/// mapped to a field or decision, or explicitly recorded as carrying none WITH A REASON."* Both
/// halves, both directions: a printed label with no decision is a line we forgot, and a decision
/// naming a label the form does not print is a decision about nothing.
///
/// ★ Run over **every** revision of this form, not just the new one — so the instrument is seen
/// discriminating on two different documents rather than tuned to one. Measured at the time of
/// writing: `f6251--2024` prints 59 labels = 41 mapped + 18 censused; `f6251--2025` prints **60** =
/// 42 mapped + 18 censused, the extra label being line 1's split. The 60 independently reproduces
/// `xtask label-census f6251--2025`'s *"60 entry line(s), 0 without a box"* by a completely different
/// method (text column vs widget geometry) — two methods, one answer.
#[test]
fn every_printed_line_is_accounted_for_in_its_own_revisions_map() {
    for (ls, year) in every_f6251_revision() {
        let text = map_text(Stem::F6251, year).unwrap();
        let extract = extract_text("f6251", year);
        let labels = printed_labels(&extract);
        let printed: BTreeSet<String> = labels.iter().cloned().collect();
        assert_eq!(
            labels.len(),
            printed.len(),
            "{}: the extract's label column yielded a DUPLICATE, so the reading is wrong rather \
             than the form: {labels:?}",
            ls.as_str()
        );
        assert!(
            printed.len() >= 59,
            "{}: only {} printed labels were read off {}-- the label column reading has gone blind, \
             which would make this whole check vacuously green",
            ls.as_str(),
            printed.len(),
            year
        );

        let mapped_labels: BTreeSet<String> = mapped(text).into_iter().map(|(l, _)| l).collect();
        let censused = censused_labels(text);
        let accounted: BTreeSet<String> = mapped_labels.union(&censused).cloned().collect();

        let forgotten: Vec<&String> = printed.difference(&accounted).collect();
        assert!(
            forgotten.is_empty(),
            "{}: the form PRINTS lines {forgotten:?} and the map neither fills them nor records a \
             census decision for them. A line nothing populated and a line the inputs left blank are \
             identical on the printed page; only the first is a defect, and this is it.",
            ls.as_str()
        );
        let invented: Vec<&String> = accounted.difference(&printed).collect();
        assert!(
            invented.is_empty(),
            "{}: the map accounts for lines {invented:?} that this revision's extract does not \
             print — a decision about a line that is not on the paper",
            ls.as_str()
        );
        // The partition is disjoint: a label is filled or declared blank, never both.
        let both: Vec<&String> = mapped_labels.intersection(&censused).collect();
        assert!(
            both.is_empty(),
            "{}: lines {both:?} are both mapped and censused",
            ls.as_str()
        );
    }
}

/// ★★★ Every quoted instruction in a map's comments is VERBATIM on the form. This is the standing
/// root cause, and this is the form it happened on.
///
/// The count is the guard against a quote that stops being *parsed* and therefore silently stops
/// being *verified* — the shape of failure that let `cite-check` report success while reading none of
/// the table it existed to protect.
#[test]
fn every_quoted_instruction_is_verbatim_on_its_own_revisions_form() {
    for (ls, year) in every_f6251_revision() {
        let text = map_text(Stem::F6251, year).unwrap();
        let form = norm(&extract_text("f6251", year));
        let mapped_labels: BTreeSet<String> = mapped(text).into_iter().map(|(l, _)| l).collect();
        let mut checked: BTreeSet<String> = BTreeSet::new();
        for l in text.lines() {
            let Some(rest) = l.trim_start().strip_prefix("# ") else {
                continue;
            };
            let Some((label, tail)) = rest.split_once(' ') else {
                continue;
            };
            let label_ok = !label.is_empty()
                && label.len() <= 3
                && label.chars().next().is_some_and(|c| c.is_ascii_digit())
                && label
                    .chars()
                    .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase());
            if !label_ok || !tail.starts_with('"') {
                continue;
            }
            let q = norm(
                tail.split_once('"')
                    .and_then(|(_, r)| r.rsplit_once('"').map(|(inner, _)| inner))
                    .unwrap_or(""),
            );
            assert!(
                form.contains(&q),
                "{}: line {label}'s quoted instruction is NOT verbatim on the form:\n      {q:?}",
                ls.as_str()
            );
            checked.insert(label.to_string());
        }
        // ★★ The count is DERIVED from the map's own mapped-line set, not typed: every filled line
        //    carries a quote, and a quote that stops parsing reds here instead of going unchecked.
        assert_eq!(
            checked,
            mapped_labels,
            "{}: every MAPPED line must carry a verbatim quote that this check actually parsed \
             (missing: {:?}; quoted but not mapped: {:?})",
            ls.as_str(),
            mapped_labels.difference(&checked).collect::<Vec<_>>(),
            checked.difference(&mapped_labels).collect::<Vec<_>>(),
        );
    }
}

/// ★★★ **THE LINE-33 PAIR, PINNED EXPLICITLY** on this revision too. Line 33 subtracts from **22**,
/// line 36 from **12** — four rows apart, same verb, and getting 33 wrong once inflated the tentative
/// minimum tax by $200,000 on one vector. The quotes come from the TEXT LAYER, not the rendered page.
#[test]
fn the_line_33_and_36_cross_references_name_different_lines() {
    for (_, year) in every_f6251_revision() {
        let extract = norm(&extract_text("f6251", year));
        assert!(
            extract.contains("Subtract line 32 from line 22"),
            "TY{year}: line 33 subtracts from line 22"
        );
        assert!(
            extract.contains("Subtract line 35 from line 12"),
            "TY{year}: line 36 subtracts from line 12"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────────────────────────
//  ★★★ THE §3 TRAP — the year-varying cells
// ──────────────────────────────────────────────────────────────────────────────────────────────────

/// ★★★ **Every revision on the OBBBA field map states its OWN year-varying cells.**
///
/// The compiler already refuses a revision with no arm in `f6251_revision::revision` (`E0004`,
/// observed). This closes the other leg: an arm written as `=> None` compiles, and would leave the
/// revision inheriting nothing — so it reds here instead. Derived from `LineSet::ALL` and the schema
/// match, so there is no list to extend.
#[test]
fn every_revision_on_this_schema_has_its_year_varying_cells() {
    for (ls, year) in obbba_revisions() {
        let r = btctax_forms::f6251_revision::revision(ls).unwrap_or_else(|| {
            panic!(
                "{} is wired to the OBBBA field map but states none of its own year-varying cells. \
                 Its line 1a would inherit another revision's Schedule 1-A line number — TY2025 \
                 prints line 37 and TY2026 line 43 — and nothing on the printed page, in the field \
                 map, in the geometry fixture or in either oracle would show it.",
                ls.as_str()
            )
        });
        assert_eq!(
            r.line_set,
            ls,
            "{}: a revision's cells must name it",
            ls.as_str()
        );
        assert_eq!(
            r.extract,
            format!("f6251--{year}"),
            "{}: the cells must be asserted against THIS revision's extract",
            ls.as_str()
        );
        assert!(
            !r.extract.contains("DRAFT"),
            "{}: a DRAFT extract is not an authority and must never be a revision's source",
            ls.as_str()
        );
        // The cross-references must be READABLE, not merely present: an unparseable sentence means
        // the AMT base's source line is unknown, and the emitter refuses on exactly this.
        assert!(
            r.schedule_1a_line().is_some(),
            "{}: line 1a's sentence carries no Schedule 1-A line number",
            ls.as_str()
        );
        assert!(
            r.mfs_threshold_printed().is_some(),
            "{}: line 4's parenthetical carries no MFS threshold",
            ls.as_str()
        );
        assert!(
            r.form_1040_capital_gain_line().is_some(),
            "{}: line 7's routing bullet carries no Form 1040 line",
            ls.as_str()
        );
    }
}

/// ★★★ **THE KILL THIS FILE EXISTS FOR: a revision's year-varying cells are verbatim in ITS OWN
/// extract.**
///
/// The two revisions' field maps are byte-identical in every FQN, so copying TY2025's cells onto
/// TY2026 compiles, parses, fills, reads back and prints a page that looks right — with the AMT base
/// taken from Schedule 1-A line **37** when TY2026 prints line **43**. Nothing else in this crate can
/// see that. This can, because the cells are compared against the extract of the document they claim
/// to transcribe.
///
/// ★ Observed RED on a planted defect (B1): changing TY2025's `line1a` to say "line 43" — exactly the
/// copy-from-the-sibling-revision mistake — fails here naming the sentence.
#[test]
fn the_year_varying_cells_are_verbatim_in_that_revisions_own_extract() {
    for (ls, year) in obbba_revisions() {
        let r = btctax_forms::f6251_revision::revision(ls).expect("cells exist");
        let form = norm(&extract_text("f6251", year));
        for (what, quote) in [
            ("line 1a", r.line1a),
            ("line 4", r.line4),
            (
                "line 7's Part III routing clause",
                r.line7_capital_gain_clause,
            ),
        ] {
            assert!(
                form.contains(&norm(quote)),
                "{}: {what}'s stated text is NOT verbatim in {}.txt — this is what a cell copied \
                 from a SIBLING REVISION looks like, and it is the only place that difference is \
                 visible:\n      {quote:?}",
                ls.as_str(),
                r.extract
            );
        }
        // ★★ …and the parsed cross-references are what the extract prints, not merely parseable.
        //    The Schedule 1-A line number is re-derived here from the extract itself so the
        //    comparison has two independent sides.
        let from_extract = form
            .split_once("Subtract Schedule 1-A (Form 1040), line ")
            .map(|(_, t)| {
                t.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
            })
            .and_then(|d| d.parse::<u32>().ok());
        assert_eq!(
            r.schedule_1a_line(),
            from_extract,
            "{}: the Schedule 1-A line number the cells state disagrees with the one the form \
             prints — the AMT base would come off the wrong line",
            ls.as_str()
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────────────────────────
//  FILL + READ-BACK
// ──────────────────────────────────────────────────────────────────────────────────────────────────

fn header() -> btctax_core::tax::packet::ReturnHeader {
    btctax_core::tax::testonly::kitchen_sink_header()
}

/// A **legitimately vouched** enhanced-senior-deduction subtotal: `amount` really is what this
/// schedule's line 37 prints, and the line number travels out of the schedule struct rather than out
/// of this file.
///
/// ★★ This is the only lawful route to a `SeniorDeductionSubtotal` — its fields are private to
/// `btctax_core::tax::schedule_1a`, so no fixture here can pair a figure with a Schedule 1-A line no
/// revision printed. The mismatch a kill test needs has its own deliberately ugly route
/// (`schedule_1a::testonly::forge_senior_deduction_subtotal_vouched_by_no_schedule`), used exactly
/// once in this file and nowhere in production — see
/// [`the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite`].
fn vouched_senior_subtotal(amount: Usd) -> btctax_core::tax::schedule_1a::SeniorDeductionSubtotal {
    use btctax_core::tax::schedule_1a::{Schedule1A, Schedule1aPartV};
    let sch = Schedule1A {
        part5: Schedule1aPartV {
            line37: Some(amount),
            ..Default::default()
        },
        ..Default::default()
    };
    Schedule1A::senior_deduction_subtotal(Some(&sch))
}

/// A filer who must attach the form and did NOT route to Part III: the "All others" flat 26/28 %
/// branch, with a $500 state-tax refund on line 2b (stored NEGATIVE, per i6251) and a Schedule 1-A
/// line-37 senior-deduction subtotal on line 1a.
///
/// ★ Built from `Default` and assigned because `Form6251` is `#[non_exhaustive]` outside its crate —
/// which is also the right shape: a fixture naming every field would go stale silently the moment
/// core gains one.
fn part_i_only_2025() -> Form6251 {
    let mut f = Form6251::default();
    f.line1 = Form6251Line1::Y2025 {
        line1a: dec!(15000),
        line1b: dec!(300000),
        // ★★ PROVENANCE, and the emitter compares its line against the revision's own printed
        //    sentence. Taken from the schedule struct that prints the subtotal, never typed here — so
        //    if a SECOND revision joins `obbba_revisions()` whose 1a cites line 43, the fills below
        //    REFUSE rather than printing this figure, and the fix is a per-revision fixture (core
        //    will by then have transcribed that revision's own Schedule 1-A), not a bumped literal.
        //    ★ It is now the compiler holding that and not this comment: the line number is
        //      unreachable except through the schedule accessor.
        senior_deduction: vouched_senior_subtotal(dec!(6000)),
    };
    f.line2a = dec!(10000);
    f.line2b = dec!(-500); // ★ core stores it negative; the box is parenthesised
    f.line3 = Usd::ZERO;
    f.line4 = dec!(309500); // 1b + 2a + 2b + 3 — "Combine lines 1b through 3", NOT 1a
    f.line5 = dec!(88100);
    f.line6 = dec!(221400);
    f.line7 = dec!(57564);
    f.line8 = Usd::ZERO;
    f.line9 = dec!(57564);
    f.line10 = dec!(55000);
    f.line11 = dec!(2564);
    f.part_iii_completed = false;
    f
}

fn tv(pdf: &[u8], fqn: &str) -> Option<String> {
    let doc = load(pdf).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let idx = index(&fields);
    text_value(&doc, idx.get(fqn)?.id)
}

/// ★★★ The fill lands every Part I/II figure in its own box and reads back off the SERIALIZED PDF.
///
/// Both line-1 boxes are written — `line1a` into the inner column and `line1b` into the amount
/// column — and the emitter's own geometric read-back (`verify_flat`) has already asserted the column
/// bands and the y-descent before these bytes existed; a failure there returns no PDF at all.
#[test]
fn the_obbba_fill_writes_both_line_1_boxes_and_reads_back() {
    for (ls, year) in obbba_revisions() {
        let map = Form6251ObbbaMap::for_year(year)
            .unwrap_or_else(|e| panic!("{} must resolve: {e}", ls.as_str()));
        let pdf = fill_form_6251_obbba_with_map(&part_i_only_2025(), &header(), &map).unwrap();

        assert_eq!(
            tv(&pdf, map.line1a.fields()[0]).as_deref(),
            Some("15000"),
            "{}: line 1a — the Schedule 1-A subtotal being subtracted out",
            ls.as_str()
        );
        assert_eq!(
            tv(&pdf, map.line1b.fields()[0]).as_deref(),
            Some("300000"),
            "{}: line 1b — and this, not 1a, is what line 4 combines",
            ls.as_str()
        );
        assert_eq!(tv(&pdf, map.line4.fields()[0]).as_deref(), Some("309500"));
        assert_eq!(tv(&pdf, map.line11.fields()[0]).as_deref(), Some("2564"));

        // ★★★ THE PARENTHESISED BOX TAKES A MAGNITUDE. The form prints `2b (   )`, so the parentheses
        //     supply the minus; writing the stored `-500` renders `(-500)`, which reads as a POSITIVE
        //     number on a return signed under 26 USC §6065.
        let l2b = tv(&pdf, map.line2b.fields()[0]);
        assert_eq!(l2b.as_deref(), Some("500"), "{}: line 2b", ls.as_str());
        assert!(!l2b.unwrap().starts_with('-'));

        // ★★★ PART III IS NOT WRITTEN WHEN THE FORM SAYS TO SKIP IT. *"Complete Part III only if you
        //     are required to do so by line 7…"* — lines 12-40 are plain `Usd` in core and zero on
        //     the un-routed path, so writing them would file TWENTY-NINE SWORN ZEROS on a page the
        //     filer was told not to complete. "Skip" is not "enter -0-"; this form says the latter
        //     elsewhere when that is what it means.
        for cell in [&map.line12, &map.line19, &map.line33, &map.line40] {
            assert_eq!(
                tv(&pdf, cell.fields()[0]),
                None,
                "{}: Part III must be ABSENT on the un-routed path",
                ls.as_str()
            );
        }
    }
}

/// ★★ **The revision gate refuses a TY2024-shaped line 1**, rather than leaving a numbered box blank.
///
/// The mirror of `Form6251Map`'s own refusal. Both directions matter: filling the 1a/1b PDF through
/// the single-line-1 map walks lines 2a..11 down one widget and lands **line 11, the AMT itself**, in
/// line 10's box; filling this map from a TY2024 chain would leave line 1a — or 1b — empty on a filed
/// form.
#[test]
fn the_obbba_fill_refuses_a_line_1_shape_it_has_no_cells_for() {
    for (ls, year) in obbba_revisions() {
        let map = Form6251ObbbaMap::for_year(year).unwrap();
        let mut f = part_i_only_2025();
        f.line1 = Form6251Line1::Y2024 {
            line1: dec!(300000),
        };
        let e = fill_form_6251_obbba_with_map(&f, &header(), &map)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "{}: a TY2024-shaped line 1 was ACCEPTED by the 1a/1b emitter — one of the two \
                     line-1 boxes would be blank on a filed form",
                    ls.as_str()
                )
            })
            .to_string();
        assert!(
            e.contains("1a/1b"),
            "{}: the refusal must say which revision's shape was expected; got {e:?}",
            ls.as_str()
        );
    }
}

/// ★★★ **Kill 4 — a wrong field name in the map is caught, not written into the void.**
///
/// `lopdf` silently writes nothing for a field that does not exist, so a typo'd FQN would drop a
/// whole numbered line off a filed return with no error anywhere. The map is mutated IN MEMORY (never
/// on disk) and the fill must refuse.
#[test]
fn a_map_naming_a_field_the_pdf_does_not_have_refuses_the_fill() {
    for (ls, year) in obbba_revisions() {
        let text = map_text(Stem::F6251, year).unwrap();
        let planted = text.replace(
            "line1a = \"topmostSubform[0].Page1[0].f1_3[0]\"",
            "line1a = \"topmostSubform[0].Page1[0].f1_999[0]\"",
        );
        assert_ne!(
            planted,
            text,
            "{}: the plant did not apply — this kill would be vacuously green",
            ls.as_str()
        );
        let map = Form6251ObbbaMap::parse(&planted).expect("a wrong FQN still parses as TOML");
        let e = fill_form_6251_obbba_with_map(&part_i_only_2025(), &header(), &map)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "{}: a map naming a nonexistent field FILLED SUCCESSFULLY — line 1a would be \
                     missing from the filed form with nothing red",
                    ls.as_str()
                )
            })
            .to_string();
        assert!(
            e.contains("f1_999"),
            "{}: the refusal must name the missing field; got {e:?}",
            ls.as_str()
        );
    }
}

/// ★★ **The geometry is the oracle, not the map**: a map that swaps two lines' widgets fills
/// "successfully" as far as `lopdf` is concerned and is caught only by the y-descent read-back.
///
/// Swapping lines 9 and 10 keeps every FQN existent, keeps the partition intact and keeps both values
/// in the right-hand column — and puts the tentative minimum tax in line 10's box. The read-back
/// rejects it because the ordinals no longer descend.
#[test]
fn a_map_with_two_lines_transposed_is_caught_by_the_read_back() {
    for (ls, year) in obbba_revisions() {
        let text = map_text(Stem::F6251, year).unwrap();
        let planted = text
            .replace(
                "line9 = \"topmostSubform[0].Page1[0].f1_31[0]\"",
                "line9 = \"topmostSubform[0].Page1[0].f1_32[0]\"",
            )
            .replace(
                "line10 = \"topmostSubform[0].Page1[0].f1_32[0]\"",
                "line10 = \"topmostSubform[0].Page1[0].f1_31[0]\"",
            );
        assert_ne!(
            planted,
            text,
            "{}: the transposition plant did not apply",
            ls.as_str()
        );
        let map = Form6251ObbbaMap::parse(&planted).expect("a transposed map still parses");
        let e = fill_form_6251_obbba_with_map(&part_i_only_2025(), &header(), &map)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "{}: lines 9 and 10 were TRANSPOSED and the fill succeeded — the tentative \
                     minimum tax would print in the AMT-offset box",
                    ls.as_str()
                )
            })
            .to_string();
        assert!(
            e.contains("descent"),
            "{}: the transposition must be caught by the y-descent read-back; got {e:?}",
            ls.as_str()
        );
    }
}

/// A filer whose line 7 DOES route to Part III, with cents on every reachable shape.
///
/// ★ Part III is filed as a UNIT or not at all, so `part_iii_completed` is what the form's own
/// routing instruction says — not a convenience flag. Values are distinct per line so a cell pointing
/// at its neighbour's widget is a value mismatch and not merely a count that still adds up.
fn part_iii_routed_2025() -> Form6251 {
    let mut f = part_i_only_2025();
    f.part_iii_completed = true;
    // Cents on the shapes that reach the page through different paths: Part I's inner column, the
    // parenthesised box, and Part III.
    f.line1 = Form6251Line1::Y2025 {
        line1a: dec!(15000.4913),
        line1b: dec!(300000.4913),
        senior_deduction: vouched_senior_subtotal(dec!(6000)),
    };
    f.line2b = dec!(-500.5044); // magnitude 501 after HALF-UP rounding
                                // Part III, lines 12-40. Distinct, ascending-by-line values with cents on several.
    let p3: [(&mut Usd, Usd); 29] = [
        (&mut f.line12, dec!(223800.1234)),
        (&mut f.line13, dec!(40000)),
        (&mut f.line14, dec!(1000)),
        (&mut f.line15, dec!(41000)),
        (&mut f.line16, dec!(41000)),
        (&mut f.line17, dec!(41000)),
        (&mut f.line18, dec!(232600)),
        (&mut f.line19, dec!(47025)),
        (&mut f.line20, dec!(30000)),
        (&mut f.line21, dec!(17025)),
        (&mut f.line22, dec!(182800)),
        (&mut f.line23, dec!(17025)),
        (&mut f.line24, dec!(17025)),
        (&mut f.line25, dec!(518900)),
        (&mut f.line26, dec!(47025)),
        (&mut f.line27, dec!(30000)),
        (&mut f.line28, dec!(77025)),
        (&mut f.line29, dec!(518900)),
        (&mut f.line30, dec!(77025)),
        (&mut f.line31, dec!(0)),
        (&mut f.line32, dec!(0)),
        (&mut f.line33, dec!(0)),
        (&mut f.line34, dec!(0)),
        (&mut f.line35, dec!(19050)),
        (&mut f.line36, dec!(163750)),
        (&mut f.line37, dec!(40937.5)),
        (&mut f.line38, dec!(60000)),
        (&mut f.line39, dec!(57564.2840)),
        (&mut f.line40, dec!(57564)),
    ];
    for (slot, v) in p3 {
        *slot = v;
    }
    f
}

/// ★★★ **PART III, ROUTED — and EVERY modelled money cell read back off the SERIALIZED PDF.**
///
/// **This is B3's "the fix already existed in the branch", carried across.** The identical guarantee
/// is nine files away for the sibling revision (`f6251_fill.rs`'s
/// `every_printed_figure_is_a_whole_dollar`, which routes Part III, iterates `map.money_cells()` and
/// asserts `checked == 41`). It was not carried to the revision that is TY2026's actual emitter.
/// Measured before this test existed: `grep -c "money_cells\|part_iii_completed = true"` over this
/// file returned **0** — so 29 of the 42 money cells, INCLUDING the line-33/36 pair that produced this
/// repo's standing transcription rule, were written by no test and read back by no test, and
/// `Form6251ObbbaMap::money_cells()` had no reader at all although its own doc comment says *"the
/// sweeps that read the FILLED page back … iterate this list"*.
///
/// **Why the counter is load-bearing, and not decoration.** The sweep skips a cell whose widget reads
/// back empty (`continue`), so without the count a cell that stopped being written would drop out
/// silently and the assertion would pass over 41 cells, then 40, reporting nothing. The count is the
/// difference between "every cell I found is a whole dollar" and "every modelled cell was found".
#[test]
fn part_iii_routed_writes_every_money_cell_as_a_whole_dollar_and_the_sweep_sees_all_of_them() {
    for (ls, year) in obbba_revisions() {
        let map = Form6251ObbbaMap::for_year(year).unwrap();
        let f = part_iii_routed_2025();
        let pdf = fill_form_6251_obbba_with_map(&f, &header(), &map)
            .unwrap_or_else(|e| panic!("{}: the routed Part III must fill: {e}", ls.as_str()));

        let doc = load(&pdf).unwrap();
        let fields = collect_fields(&doc).unwrap();
        let idx = index(&fields);
        // ★ Pinned separately from `checked` so the count's MEANING is unambiguous: 42 cells, one
        //   widget each, all 42 read back. Without this, "checked == 42" could also be 41 cells with
        //   one carrying two widgets — the same number standing for a different fact.
        assert_eq!(
            map.money_cells().len(),
            42,
            "{}: the map's own cell list",
            ls.as_str()
        );
        let mut checked = 0;
        for cell in map.money_cells() {
            for fqn in cell.fields() {
                let Some(v) = idx.get(fqn).and_then(|fl| text_value(&doc, fl.id)) else {
                    continue;
                };
                assert!(
                    !v.contains('.'),
                    "{}: {fqn} prints {v:?} — every line on a filed form is a whole dollar",
                    ls.as_str()
                );
                checked += 1;
            }
        }
        assert_eq!(
            checked,
            42,
            "{}: all forty-two modelled lines must be read back; a count that drifts means the \
             sweep stopped seeing cells and started passing vacuously",
            ls.as_str()
        );

        // ★★★ THE LINE-33/36 PAIR, on the page rather than in a doc comment. 33 subtracts from line
        //     **22** and 36 from line **12** — four rows apart, same verb — and the whole reason this
        //     form carries the repo's transcription rule is that they were once both "from line 12".
        //     Reaching them at all requires the routed path, which is why they were untested.
        assert_eq!(
            tv(&pdf, map.line33.fields()[0]).as_deref(),
            Some("0"),
            "{}: line 33",
            ls.as_str()
        );
        assert_eq!(
            tv(&pdf, map.line36.fields()[0]).as_deref(),
            Some("163750"),
            "{}: line 36 — a DIFFERENT box from line 33's, and a different figure",
            ls.as_str()
        );
        assert_eq!(
            tv(&pdf, map.line40.fields()[0]).as_deref(),
            Some("57564"),
            "{}: line 40 is what line 7 carries back to Part II",
            ls.as_str()
        );
        // …and the rounding is HALF-UP, not truncation: 500.5044 -> 501, 15000.4913 -> 15000.
        assert_eq!(tv(&pdf, map.line2b.fields()[0]).as_deref(), Some("501"));
        assert_eq!(tv(&pdf, map.line1a.fields()[0]).as_deref(), Some("15000"));
        assert_eq!(tv(&pdf, map.line37.fields()[0]).as_deref(), Some("40938"));
    }
}

/// ★★★ **THE JOIN, THROUGH THE REAL EMITTER — a figure read off a Schedule 1-A line this revision
/// does not cite REFUSES, rather than printing into line 1a's box.**
///
/// This is the computation-side twin of
/// [`the_year_varying_cells_are_verbatim_in_that_revisions_own_extract`], and until 2026-09-11 nothing
/// held it. The emitter resolved `revision.schedule_1a_line()` and **discarded the value** — an
/// existence check on the sentence, never a comparison — while on the core side the cross-reference
/// was spelled into a FIELD NAME (`schedule_1a_l37`), which no test, no `_`-free match and no extract
/// comparison can read. So the two halves of the most dangerous cell on this form were each pinned and
/// nothing joined them.
///
/// **What the plant is.** `37 → 43` is a COLLISION, not a renumber: the senior-deduction subtotal moved
/// to TY2026's line 43, and TY2026's line **37 was refilled** with *"Enter the amount from line 3"* —
/// modified AGI (`design/forms/extract/f1040s1a--2026-DRAFT.txt:213,222`). So a TY2026 arm reusing
/// core's `Y2025` shape against the TY2025 schedule compiles, fills, reads back and prints a page that
/// looks right, with a six-figure income where a ≤$6,000 deduction belongs and the AMT base overstated
/// by ≈MAGI — taxpayer-adverse, and invisible to the field map (same FQNs), the geometry (same rects),
/// the read-back (the value lands in the box it was sent to) and both oracles (they take it as INPUT).
///
/// ★ The planted line number is DERIVED from the revision's own printed one, never typed, so this kill
/// keeps meaning the same thing when a second revision joins `obbba_revisions()`.
#[test]
fn the_fill_refuses_a_figure_read_off_a_schedule_1a_line_this_revision_does_not_cite() {
    for (ls, year) in obbba_revisions() {
        let map = Form6251ObbbaMap::for_year(year).unwrap();
        let printed = map
            .revision()
            .and_then(|r| r.schedule_1a_line())
            .unwrap_or_else(|| panic!("{}: the revision must cite a line", ls.as_str()));
        // The other revision's line, whichever side of the collision this one is on.
        let other = if printed == 37 { 43 } else { 37 };

        let mut f = part_i_only_2025();
        f.line1 = Form6251Line1::Y2025 {
            line1a: dec!(15000),
            line1b: dec!(300000),
            // ★★★ **THE DELIBERATE FORGE, AND IT IS THE ONLY ONE IN THE WORKSPACE.** This kill's whole
            //     job is to present the mismatch the join exists to refuse, so the mismatch has to be
            //     constructible — sealing it away would make the guarantee unfalsifiable, which is the
            //     failure mode harness B1 was written against. The route is named for what it does and
            //     lives in a `testonly` module, so its appearance in a production seam reads as a
            //     defect on sight; `xtask::forge_reach_check` reds if it ever appears in one.
            //     ★ Honestly: this stands in for the accessor of a schedule revision that has not been
            //       transcribed yet. When TY2026's Schedule 1-A lands, `other` has a lawful source
            //       (that struct's own `SENIOR_DEDUCTION_SUBTOTAL_LINE = 43`) and this call goes away.
            senior_deduction:
                btctax_core::tax::schedule_1a::testonly::forge_senior_deduction_subtotal_vouched_by_no_schedule(
                    dec!(6000),
                    other,
                ),
        };
        let e = fill_form_6251_obbba_with_map(&f, &header(), &map)
            .err()
            .unwrap_or_else(|| {
                panic!(
                    "{}: this form cites Schedule 1-A line {printed} and the figure was read off \
                     line {other} — the fill SUCCEEDED, so the AMT base is taken from the wrong \
                     line of another revision's schedule and nothing on the printed page shows it",
                    ls.as_str()
                )
            })
            .to_string();
        assert!(
            e.contains(&format!("line {printed}")) && e.contains(&format!("line {other}")),
            "{}: the refusal must name BOTH lines — the whole defect is that they look \
             interchangeable; got {e:?}",
            ls.as_str()
        );

        // …and the agreeing figure still fills, so the guard is a comparison and not a blanket refusal.
        assert!(
            fill_form_6251_obbba_with_map(&part_i_only_2025(), &header(), &map).is_ok(),
            "{}: the real chain must still fill",
            ls.as_str()
        );
    }
}
