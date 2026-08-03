//! §G-6 — the Form 6251 map, held against the form itself.
//!
//! Four checks, the same shape that pinned Form 8995-A: every FQN exists on the blank PDF, every
//! line sits in the column the form prints it in, y descends within each page, and every quoted
//! instruction is verbatim in the extracted text.
//!
//! ★★★ This form is the reason the transcription rule exists. Line 33 reads *"Subtract line 32 from
//! line 22"* and line 36 reads *"Subtract line 35 from line 12"* — four rows apart, same verb — and
//! transcribing 33 from the rendered page once produced "from line 12", taxing the ordinary slice
//! twice and inflating the tentative minimum tax by $200,000 on one vector.

use btctax_forms::testonly::{collect_fields, f6251_pdf, load};
use std::collections::BTreeSet;

const MAP: &str = include_str!("../forms/2024/f6251.map.toml");
const FORM_TEXT: &str = include_str!("../../../design/forms/extract/f6251--2024.txt");

/// Every `lineNN = "…"` in the map, as (line label, FQN).
fn mapped() -> Vec<(String, String)> {
    MAP.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let n = k.trim().strip_prefix("line")?;
            (!n.is_empty() && n.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .then(|| (n.to_string(), v.trim().trim_matches('"').to_string()))
        })
        .collect()
}

/// ★★★ Every mapped FQN must EXIST on the blank PDF. A typo'd field name is invisible to a fill —
/// `lopdf` simply writes nothing — so a whole line would silently vanish from a filed return.
#[test]
fn every_mapped_field_exists_in_the_blank_form() {
    let doc = load(f6251_pdf(2024).expect("the crate ships f6251.pdf")).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let present: BTreeSet<&str> = fields.iter().map(|f| f.fqn.as_str()).collect();

    let m = mapped();
    assert_eq!(
        m.len(),
        41,
        "core models 41 of the form's 59 numbered boxes; 2c-2t are censused: {m:?}"
    );
    for (line, fqn) in &m {
        assert!(
            present.contains(fqn.as_str()),
            "line {line} maps to {fqn}, which is NOT a field on the blank form"
        );
    }
    // ★★★ …and the three sets PARTITION the AcroForm — a disjoint union, not a matching count.
    //
    //     r1 found this was a COUNT: `mapped + censused + identity == fields.len()`. That is satisfied
    //     by any bijection-shaped mistake. Concretely, shifting the `3..11` block DOWN one widget
    //     (line 3 → `f1_23`, … line 11 → `f1_31`) keeps the count at 61, keeps every FQN existent,
    //     keeps y descending, and keeps 2b inset — it passes all four tests while printing line 3's
    //     figure in line 2t's box and leaving `f1_32` blank on a filed return.
    //
    //     ★ The same repo has been bitten by count-not-partition before (a `BTreeSet` built from
    //       `1..=38` when the label set was 48). A count answers "how many"; only a partition answers
    //       "which", and "which" is the question a field map exists to answer.
    let censused: BTreeSet<&str> = MAP
        .lines()
        .filter(|l| l.starts_with("\"topmostSubform"))
        .filter_map(|l| l.split('"').nth(1))
        .collect();
    assert_eq!(censused.len(), 18, "lines 2c-2t are censused");

    let mapped: BTreeSet<&str> = m.iter().map(|(_, f)| f.as_str()).collect();
    assert_eq!(mapped.len(), m.len(), "no FQN is mapped to two lines");

    let overlap: Vec<&&str> = mapped.intersection(&censused).collect();
    assert!(
        overlap.is_empty(),
        "{overlap:?} are BOTH mapped and censused — a line pointed at a widget the map also declares \
         carries nothing. This is the shift the old count could not see."
    );

    // ★ Keyed on `name`/`ssn`, NOT on "everything after `[identity]`" — the census section follows it
    //   in the file (the header must come last so TOML does not swallow the `lineNN` keys), so a
    //   positional read picks up all 18 census FQNs as identity and the partition silently widens.
    let identity: BTreeSet<&str> = MAP
        .lines()
        .filter(|l| {
            let k = l.split('=').next().unwrap_or("").trim();
            k == "name" || k == "ssn"
        })
        .filter_map(|l| l.split('"').nth(1))
        .collect();
    assert_eq!(identity.len(), 2, "name + SSN");

    let accounted: BTreeSet<&str> = mapped
        .union(&censused)
        .copied()
        .collect::<BTreeSet<&str>>()
        .union(&identity)
        .copied()
        .collect();
    let unaccounted: Vec<&str> = present.difference(&accounted).copied().collect();
    assert!(
        unaccounted.is_empty(),
        "{unaccounted:?} exist on the blank form but are neither mapped, censused, nor identity — a \
         widget no decision reaches, which is invisible on the printed page and to both oracles"
    );
    assert_eq!(
        accounted.len(),
        fields.len(),
        "mapped ∪ censused ∪ identity must be exactly the AcroForm"
    );
}

/// ★★★ THE INSET TRIO — the corroboration that makes the page-1 assignment more than an in-order zip.
///
/// Page 1's field names are NOT a uniform offset: `f1_3` is line 1, `f1_4`..`f1_23` are the twenty
/// sub-lines 2a..2t, `f1_24` is line 3. The form prints exactly THREE parenthesised boxes — 2b, 2f,
/// 2s — and the PDF has exactly three narrower widgets (w=64 against w=72). Under this assignment the
/// two sets coincide; under an off-by-N they do not, which is what pins the offset.
#[test]
fn the_three_inset_widgets_land_on_the_three_parenthesised_lines() {
    let doc = load(f6251_pdf(2024).unwrap()).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let width = |fqn: &str| -> f32 {
        fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.rect.map(|r| r[2] - r[0]))
            .unwrap_or_else(|| panic!("no rect for {fqn}"))
    };
    // Only 2b is MAPPED (2f and 2s are inside the censused 2c-2t range), so the inset check runs over
    // the map for 2b and over the census for the other two.
    let inset: Vec<&str> = MAP
        .lines()
        .filter_map(|l| l.split('"').nth(1))
        .filter(|f| f.contains("Page1") && f.contains("f1_"))
        .filter(|f| width(f) < 70.0)
        .collect();
    assert_eq!(
        inset.len(),
        3,
        "the form prints exactly three parenthesised boxes; got {inset:?}"
    );
    for f in &inset {
        assert!(
            ["f1_5[0]", "f1_9[0]", "f1_22[0]"]
                .iter()
                .any(|w| f.ends_with(w)),
            "{f} is inset but is not one of the three parenthesised widgets"
        );
    }
    // ★ …and the mapped one of the three is line 2b.
    let l2b = mapped()
        .into_iter()
        .find(|(l, _)| l == "2b")
        .expect("2b is mapped");
    assert!(l2b.1.ends_with("f1_5[0]"), "line 2b is the first inset box");
}

/// ★★ y must DESCEND as the line number ascends, WITHIN each page. Catches a transposition the column
/// check cannot see. Per page, because Part III starts on page 2 where y resets to the top.
#[test]
fn the_lines_descend_each_page_in_order() {
    let doc = load(f6251_pdf(2024).unwrap()).unwrap();
    let fields = collect_fields(&doc).unwrap();
    let y_of = |fqn: &str| -> f32 {
        fields
            .iter()
            .find(|f| f.fqn == fqn)
            .and_then(|f| f.rect.map(|r| (r[1] + r[3]) / 2.0))
            .unwrap()
    };
    let key = |l: &str| -> (u32, u32) {
        let digits: String = l.chars().take_while(|c| c.is_ascii_digit()).collect();
        let letter = l.chars().find(|c| c.is_ascii_alphabetic()).unwrap_or(' ');
        (digits.parse().unwrap(), letter as u32)
    };
    for page in ["Page1", "Page2"] {
        let mut m: Vec<(String, String)> = mapped()
            .into_iter()
            .filter(|(_, f)| f.contains(page))
            .collect();
        m.sort_by_key(|(l, _)| key(l));
        for w in m.windows(2) {
            assert!(
                y_of(&w[0].1) > y_of(&w[1].1),
                "{page}: line {} sits at or below line {} ({} vs {}) — the map is transposed",
                w[0].0,
                w[1].0,
                y_of(&w[0].1),
                y_of(&w[1].1)
            );
        }
    }
}

/// ★★★ Every quoted instruction is VERBATIM on the form. This is the standing root cause, and this is
/// the form it happened on.
#[test]
fn every_quoted_instruction_is_verbatim_on_the_form() {
    // ★★★ STANDALONE BRACE GLYPHS ARE DROPPED, and this is not a convenience. The form draws large
    //     `{` / `}` braces to group its bracketed rows, and `pdftotext` emits them as lone tokens
    //     INSIDE a sentence: line 6's own text comes out as *"…on lines 7, 9, and } 11, and go to
    //     line 10"*. A `contains` over the raw layer therefore rejects the FAITHFUL quote and accepts
    //     a truncated one ending "…lines 7, 9, and" — silently dropping 11 from the zero-out set,
    //     which is the survey's trap #1 and precisely the failure this whole check exists to prevent.
    //
    //     ★ A lone `{`/`}` between spaces is a glyph, never instruction text — so this is a
    //       mechanical normalisation of the LAYOUT, not a per-line exception list.
    let norm = |s: &str| {
        s.split_whitespace()
            .filter(|t| *t != "{" && *t != "}")
            .collect::<Vec<_>>()
            .join(" ")
    };
    let form = norm(FORM_TEXT);
    let mut checked = 0;
    for l in MAP.lines() {
        let t = l.trim_start();
        let Some(rest) = t.strip_prefix("# ") else {
            continue;
        };
        let Some((label, tail)) = rest.split_once(' ') else {
            continue;
        };
        // A quoted instruction opens `<label> "` where the label is a line number, optionally lettered.
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
            "line {label}'s quoted instruction is NOT verbatim on the form:\n      {q:?}"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 41,
        "all FORTY-ONE mapped instructions must be checked — the count is the guard against a quote \
         that stops being parsed and therefore silently stops being verified"
    );
}

/// ★★★ THE LINE-33 PAIR, PINNED EXPLICITLY. Line 33 subtracts from **22**, line 36 from **12** — four
/// rows apart, same verb, and getting 33 wrong once inflated the tentative minimum tax by $200,000.
#[test]
fn the_line_33_and_36_cross_references_name_different_lines() {
    let by = |n: &str| -> String {
        MAP.lines()
            .find(|l| l.trim_start().starts_with(&format!("# {n} \"")))
            .unwrap_or_else(|| panic!("no quote for line {n}"))
            .to_string()
    };
    assert!(
        by("33").contains("Subtract line 32 from line 22"),
        "line 33 subtracts from line 22: {}",
        by("33")
    );
    assert!(
        by("36").contains("Subtract line 35 from line 12"),
        "line 36 subtracts from line 12: {}",
        by("36")
    );
}
