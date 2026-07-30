//! ⑤ step 2 — **the two mechanical WITNESSES**, and the rules for when they disagree.
//!
//! Design adjudicated by the ④ consult (`reviews/label-reader-strategy-fable-r1.md`, `hybrid`):
//!
//! > The FORM — the hash-pinned PDF and its extract — is the only authority; all three signals are
//! > witnesses to it, exactly as an oracle is.
//!
//! | witness | reads | answers |
//! |---|---|---|
//! | **W1 text** | the margin column of `pdftotext -bbox` words | *which line labels are printed, and where* |
//! | **W2 boxes** | AcroForm field geometry, y-flipped | *which rows carry an amount box* |
//!
//! Neither is authoritative alone, and the asymmetry is the point: W2 answers the question
//! `LABEL_READER.md` says the text layer **cannot** — *"distinguishing a heading from a label means
//! knowing whether the line has an amount box, which the text layer does not directly say."*
//!
//! ★★★ **THE COLUMN IS DERIVED, NEVER HARDCODED.** Schedule 1-A's labels sit at x=45, f1040's at
//! x≈96, f1040sa's at x≈97–102. A constant would be the `1..=38` trap in yet another costume. The
//! discriminator used here is a property **of the form itself**, not of any layout:
//!
//! > **line numbers increase monotonically down the page.**
//!
//! Body-text digits (`2d` at x=167, `2e` at x=129 — real false hits, measured) do not. So the label
//! column is the x-cluster whose numbers climb as y climbs; everything else is prose that happens to
//! contain a numeral.

use crate::form_geometry::{Geometry, Word};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a line turned out to be. Only [`Kind::Amount`] is expected to carry a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A normal line with an amount box.
    Amount,
    /// A numbered line that is a HEADING for its lettered sub-rows and carries no box of its own
    /// (Schedule 1-A lines 4, 14, 22). It is still a label and must still be accounted for.
    Heading,
    /// A line that asks for something other than money — a VIN, a name, a checkbox.
    NonMoney,
}

/// One adjudicated line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// `"1"`, `"2b"`, `"22a"` — the label as the form prints it, sub-letters resolved to their parent.
    pub label: String,
    pub page: u32,
    pub kind: Kind,
    /// Why this row is what it is. Required for anything that is not a plain `Amount`, because a
    /// bare classification nobody justified is exactly the "we forgot this line" case in disguise.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// A candidate label token lifted from the text layer.
#[derive(Debug, Clone)]
struct Tok {
    page: u32,
    x2: f64,
    y: f64,
    text: String,
}

fn is_numeric_label(s: &str) -> bool {
    let d = s.trim_end_matches(|c: char| c.is_ascii_lowercase());
    !d.is_empty() && d.len() <= 2 && d.chars().all(|c| c.is_ascii_digit()) && s.len() <= 3
}

fn is_bare_letter(s: &str) -> bool {
    s.len() == 1 && s.chars().all(|c| c.is_ascii_lowercase())
}

fn numeric_part(s: &str) -> Option<u32> {
    let d: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    d.parse().ok()
}

/// ★★ **Find the label column by MONOTONICITY, not by position.**
///
/// For each cluster of numeric-label tokens, order them down the page and measure the longest
/// non-decreasing run of their numeric values. The label column is the cluster with the longest such
/// run. This is form-independent: it works at x=45 and at x=96 without being told either.
///
/// ★★★ **CLUSTER ON THE RIGHT EDGE (`x2`), NOT THE LEFT.** Measured on Schedule 1-A: the margin
/// column is RIGHT-ALIGNED, so single digits start at x≈45.4 and double digits at x≈40.4 — two
/// left-edge clusters for ONE column. Clustering on `xMin` split them and let the two-digit cluster
/// win, yielding labels 10..38 and silently dropping 1..9. Their right edges coincide at x2≈50.4.
///
/// ★ That near-miss is the `LABEL_READER.md` whitespace lesson in a new costume: an unexamined
/// alignment assumption drops a contiguous block of lines while still returning a long, plausible
/// list. It was caught only because the ONE form with a known answer was the test.
///
/// Returns the cluster's right-edge x, or `None` if nothing looks like a numbered column at all —
/// which the caller must treat as a hard failure, never a quiet zero.
fn find_label_column(words: &[Word]) -> Option<f64> {
    let toks: Vec<Tok> = words
        .iter()
        .filter(|w| is_numeric_label(&w.text))
        .map(|w| Tok {
            page: w.page,
            x2: w.x2,
            y: w.y,
            text: w.text.clone(),
        })
        .collect();

    // 2pt buckets on the RIGHT edge, then merge neighbours: the same column wobbles by a fraction
    // of a point.
    let mut by_bucket: BTreeMap<i64, Vec<&Tok>> = BTreeMap::new();
    for t in &toks {
        by_bucket
            .entry((t.x2 / 2.0).round() as i64)
            .or_default()
            .push(t);
    }

    let mut best: Option<(usize, f64)> = None;
    for (&b, group) in &by_bucket {
        let mut merged: Vec<&Tok> = group.clone();
        if let Some(n) = by_bucket.get(&(b + 1)) {
            merged.extend(n.iter().copied());
        }
        if merged.len() < 3 {
            continue; // a column of one or two numerals is prose, not a form's spine
        }
        merged.sort_by(|a, c| {
            (a.page, a.y)
                .partial_cmp(&(c.page, c.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Longest non-decreasing run of the numeric values, read down the page.
        let vals: Vec<u32> = merged
            .iter()
            .filter_map(|t| numeric_part(&t.text))
            .collect();
        let (mut run, mut longest) = (1usize, 1usize);
        for w in vals.windows(2) {
            if w[1] >= w[0] {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 1;
            }
        }
        let x = merged.iter().map(|t| t.x2).fold(0.0_f64, f64::max);
        if best.is_none_or(|(l, _)| longest > l) {
            best = Some((longest, x));
        }
    }
    best.map(|(_, x)| x)
}

/// **W1 — the text witness.** Every label the form prints in its margin column, sub-letters resolved.
///
/// ★ Sub-letters (`b`, `c`, `d`, `e`) print BARE and slightly indented — measured on Schedule 1-A:
/// parents at x=45, sub-letters at x=50. So the reader carries the last numeric parent, and the
/// indentation is a second, geometric confirmation on top of the state machine.
pub fn witness_text(g: &Geometry) -> Result<Vec<(String, u32, f64)>, String> {
    let right = find_label_column(&g.words).ok_or_else(|| {
        "no numbered label column found — refusing to report zero labels".to_string()
    })?;

    // ★ Parents are matched on their RIGHT edge (the column is right-aligned). Sub-letters print
    // bare and hang just past that edge — measured: parent x2≈50.4, sub-letter `b` at x=50.0.
    let mut toks: Vec<Tok> = g
        .words
        .iter()
        .filter(|w| {
            // ★ A parent token SPANS the column's right edge rather than ending at it. The
            // NUMBER is right-aligned (x2≈`right`), but a letter suffix hangs past: `1` ends at
            // 50.4 while `2a` ends at ~55. Requiring x2≈right dropped `2a` outright, and the bare
            // sub-letters then inherited `1` as parent and came out as `1b`..`1e`. The left bound
            // keeps prose out of the band.
            let in_parent = is_numeric_label(&w.text)
                && w.x >= right - 20.0
                && w.x <= right + 1.0
                && w.x2 >= right - 6.0;
            let in_sub = is_bare_letter(&w.text) && w.x >= right - 2.0 && w.x <= right + 15.0;
            in_parent || in_sub
        })
        .map(|w| Tok {
            page: w.page,
            x2: w.x2,
            y: w.y,
            text: w.text.clone(),
        })
        .collect();
    toks.sort_by(|a, b| {
        (a.page, a.y)
            .partial_cmp(&(b.page, b.y))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ★★ ROW MERGE, and it is mechanical rather than a judgement call. On Schedule 1-A, `14`+`a`
    // and `36`+`a` are printed on the SAME y-row: the label is `14a`, and the bare `14` beside it is
    // not a separate line. `4` and `22`, by contrast, sit on rows of their own and ARE standalone
    // headings. Measured, not assumed — same-row is `|dy| < 3.0`.
    let mut merged: Vec<Tok> = Vec::new();
    for t in &toks {
        if let Some(prev) = merged.last() {
            if prev.page == t.page
                && (prev.y - t.y).abs() < 3.0
                && is_numeric_label(&prev.text)
                && is_bare_letter(&t.text)
            {
                let combined = format!("{}{}", prev.text, t.text);
                merged.pop();
                merged.push(Tok {
                    text: combined,
                    ..t.clone()
                });
                continue;
            }
        }
        merged.push(t.clone());
    }
    let toks = merged;

    let mut out: Vec<(String, u32, f64)> = Vec::new();
    let mut parent: Option<u32> = None;
    for t in &toks {
        if is_numeric_label(&t.text) {
            parent = numeric_part(&t.text);
            out.push((t.text.clone(), t.page, t.y));
        } else {
            // A bare letter with no parent yet is a stray glyph, not a label — dropping it silently
            // would be a guess, so it is reported to the caller as a hard error instead.
            let Some(p) = parent else {
                return Err(format!(
                    "bare sub-letter `{}` at page {} y={} has no numeric parent — the state machine \
                     is out of step and the label set cannot be trusted",
                    t.text, t.page, t.y
                ));
            };
            out.push((format!("{p}{}", t.text), t.page, t.y));
        }
    }
    out.dedup_by(|a, b| a.0 == b.0);
    Ok(out)
}

/// **W2 — the box witness.** The top-down y-span of every AcroForm box, per page.
///
/// ★ This is the signal the text layer cannot provide. A row with a box is an entry line; a numbered
/// row with none is a heading.
pub fn witness_boxes(g: &Geometry) -> Vec<(u32, f64, f64, String)> {
    let mut v: Vec<(u32, f64, f64, String)> = g
        .boxes
        .iter()
        .filter_map(|b| {
            let (top, bottom) = g.box_top_down_y(b)?;
            Some((b.page, top, bottom, b.name.clone()))
        })
        .collect();
    v.sort_by(|a, b| {
        (a.0, a.1)
            .partial_cmp(&(b.0, b.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

/// Does any box sit on the same printed row as a label at top-down `y`?
///
/// ★ The tolerance is a ROW, not a point: a label's baseline and its box's rect differ by a few
/// points, and the amount box for line *n* is vertically centred on line *n*'s text.
pub fn row_has_box(boxes: &[(u32, f64, f64, String)], page: u32, y: f64) -> bool {
    boxes
        .iter()
        .any(|(p, top, bottom, _)| *p == page && y + 12.0 >= *top && y <= *bottom + 12.0)
}

/// `cargo run -p xtask -- label-census <stem>` — run both witnesses and print the adjudicated rows.
///
/// ★ This is the human's view of the two witnesses side by side: every label the form prints, and
/// whether the AcroForm says it carries an amount box. Rows the witnesses disagree about are exactly
/// the ones a person must adjudicate against the rendered page.
pub fn run(stem: &str) -> Result<(), String> {
    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    let labels = witness_text(&g)?;
    let boxes = witness_boxes(&g);

    let rows: Vec<Row> = labels
        .iter()
        .map(|(label, page, y)| {
            let has_box = row_has_box(&boxes, *page, *y);
            Row {
                label: label.clone(),
                page: *page,
                kind: if has_box { Kind::Amount } else { Kind::Heading },
                note: if has_box {
                    String::new()
                } else {
                    "no AcroForm box on this row — heading, or a non-money entry".to_string()
                },
            }
        })
        .collect();

    println!("# {stem} — {} labels, {} boxes", rows.len(), boxes.len());
    for r in &rows {
        let mark = match r.kind {
            Kind::Amount => " ",
            Kind::Heading => "H",
            Kind::NonMoney => "N",
        };
        println!("  {mark} p{} {:<5} {}", r.page, r.label, r.note);
    }
    let headings = rows.iter().filter(|r| r.kind != Kind::Amount).count();
    println!(
        "# {} entry line(s), {headings} without a box — each of the latter needs a recorded reason",
        rows.len() - headings
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form_geometry;

    fn sch1a() -> Geometry {
        form_geometry::load(&form_geometry::repo_root(), "f1040s1a--2025")
            .expect("Schedule 1-A geometry fixture")
    }

    /// ★★★ **THE ANCHOR — the one form whose answer we know.**
    ///
    /// Schedule 1-A's label set is **48** (I 7, II 12, III 10, IV 10, V 8, VI 1), established by
    /// hand. The old leading-number regex found 45 and missed three distinct classes. This asserts
    /// the text witness recovers the ones the regex could not: lines 1 and 3 (a whitespace
    /// accident), the bare sub-letters `2b`–`2e`, and `22a`/`22b` — which the regex missed
    /// *entirely* because they print as a bare `a`/`b` with nothing after them.
    #[test]
    fn the_text_witness_recovers_the_labels_the_regex_missed() {
        let g = sch1a();
        let labels: Vec<String> = witness_text(&g)
            .expect("witness")
            .into_iter()
            .map(|(l, _, _)| l)
            .collect();

        for must in ["1", "2a", "2b", "2c", "2d", "2e", "3", "22a", "22b"] {
            assert!(
                labels.iter().any(|l| l == must),
                "label `{must}` missing — this is one of the classes the leading-number regex \
                 dropped, and recovering it is the whole point. Got: {labels:?}"
            );
        }
        // ★ The STANDALONE headings must be present — they are labels even with no amount box.
        // `14` and `36` are deliberately absent: they share a y-row with their first sub-letter and
        // merge to `14a`/`36a`, which is a measured fact about the form, not a dropped line.
        for heading in ["4", "22"] {
            assert!(
                labels.iter().any(|l| l == heading),
                "standalone heading `{heading}` missing; a heading is still a label"
            );
        }
        for merged in ["14a", "36a"] {
            assert!(
                labels.iter().any(|l| l == merged),
                "`{merged}` missing — the same-row merge did not happen"
            );
        }
    }

    /// ★★★ **THE COUNT — and the ONE open adjudication, left open on purpose.**
    ///
    /// The witness yields **50**. The hand-established figure in `LABEL_READER.md` is **48**. The
    /// delta is exactly the two STANDALONE HEADING rows, `4` and `22`:
    ///
    /// - `14`+`a` and `36`+`a` share a y-row, so they merge to `14a`/`36a` — mechanical, measured
    ///   (`|dy| < 3.0`), and it accounts for 2 of the original 4 over-counts.
    /// - `4` and `22` sit on rows of their OWN, carry no amount box, and head their lettered
    ///   sub-rows. Whether they are labels is a question about the FORM, not about the reader.
    ///
    /// ★★ **The reader is NOT tuned to reach 48**, and that restraint is the point. Adjusting an
    /// instrument until it agrees with an expectation is how false confidence is manufactured — the
    /// exact failure this census exists to prevent. Under the project's own doctrine a heading *is*
    /// a label that "encodes no decision" and must be recorded **with a reason**, which argues for
    /// 50; the hand count plainly counted entry-taking lines only, which argues for 48. That is an
    /// adjudication against the rendered page, and it is what the ledger is for.
    ///
    /// This test therefore pins the MECHANICAL result and names the disagreement, so the number
    /// cannot drift while the question is open.
    #[test]
    fn the_text_witness_yields_the_mechanical_label_set() {
        let g = sch1a();
        let labels: Vec<String> = witness_text(&g)
            .expect("witness")
            .into_iter()
            .map(|(l, _, _)| l)
            .collect();
        assert_eq!(
            labels.len(),
            50,
            "mechanical label set changed. Hand-established figure is 48; the known, OPEN delta is \
             the two standalone headings `4` and `22`. Got {}: {:?}",
            labels.len(),
            labels
        );
        // The merge must have consumed the bare parents, or the count is right by luck.
        assert!(
            !labels.contains(&"14".to_string()),
            "`14` shares its row with `a` and must merge"
        );
        assert!(
            !labels.contains(&"36".to_string()),
            "`36` shares its row with `a` and must merge"
        );
        assert!(labels.contains(&"14a".to_string()) && labels.contains(&"36a".to_string()));
        // ...and the standalone headings must survive, since they are the open question.
        assert!(labels.contains(&"4".to_string()) && labels.contains(&"22".to_string()));
    }

    /// ★★ **Zero labels is ALWAYS a hard failure** — `LABEL_READER.md`'s rule, and the reason a
    /// permissive reader is worse than none: a census with nothing to check reports conformance.
    #[test]
    fn a_form_with_no_label_column_is_a_hard_error_not_an_empty_list() {
        let g = Geometry {
            form: "empty".into(),
            pdf_sha256: String::new(),
            pages: vec![crate::form_geometry::Page {
                n: 1,
                width: 612.0,
                height: 792.0,
            }],
            words: vec![Word {
                page: 1,
                x: 100.0,
                y: 100.0,
                x2: 110.0,
                y2: 110.0,
                text: "Total".into(),
            }],
            boxes: vec![],
        };
        assert!(
            witness_text(&g).is_err(),
            "a form with no numbered column must ERROR, never return an empty label set"
        );
    }

    /// ★★★ **The column is DERIVED.** Same synthetic form at two different x positions must yield
    /// the same labels — this is what stops the reader from being tuned to Schedule 1-A and silently
    /// finding nothing on f1040, whose column sits ~50pt further right.
    #[test]
    fn the_label_column_is_found_wherever_it_sits() {
        let build = |x: f64| Geometry {
            form: "synthetic".into(),
            pdf_sha256: String::new(),
            pages: vec![crate::form_geometry::Page {
                n: 1,
                width: 612.0,
                height: 792.0,
            }],
            words: (1..=6)
                .map(|i| Word {
                    page: 1,
                    x,
                    y: 100.0 + f64::from(i) * 12.0,
                    x2: x + 5.0,
                    y2: 111.0 + f64::from(i) * 12.0,
                    text: i.to_string(),
                })
                .chain(std::iter::once(Word {
                    // Body-text numeral that must NOT be mistaken for the column.
                    page: 1,
                    x: x + 120.0,
                    y: 150.0,
                    x2: x + 130.0,
                    y2: 160.0,
                    text: "50".into(),
                }))
                .collect(),
            boxes: vec![],
        };
        for x in [45.0, 96.0, 108.0] {
            let got: Vec<String> = witness_text(&build(x))
                .expect("witness")
                .into_iter()
                .map(|(l, _, _)| l)
                .collect();
            assert_eq!(
                got,
                vec!["1", "2", "3", "4", "5", "6"],
                "column at x={x} was not found correctly"
            );
        }
    }

    /// ★★ **The box witness answers what the text layer cannot.** Lines 4, 14 and 22 are headings
    /// with no amount box; ordinary lines have one. If this inverted, every heading would be
    /// classified as an entry line and the census would demand a value for a line that has no box.
    #[test]
    fn the_box_witness_separates_headings_from_entry_lines() {
        let g = sch1a();
        let boxes = witness_boxes(&g);
        assert!(
            !boxes.is_empty(),
            "Schedule 1-A has 54 AcroForm fields; none were joined"
        );

        let labels = witness_text(&g).expect("witness");
        let at = |name: &str| labels.iter().find(|(l, _, _)| l == name).cloned();

        let (_, hp, hy) = at("4").expect("line 4 present");
        assert!(
            !row_has_box(&boxes, hp, hy),
            "line 4 is a HEADING and must carry no amount box"
        );
        let (_, ap, ay) = at("1").expect("line 1 present");
        assert!(
            row_has_box(&boxes, ap, ay),
            "line 1 is an entry line and must carry an amount box"
        );
    }
}
