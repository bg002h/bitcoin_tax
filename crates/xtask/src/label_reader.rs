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
//! > **the label column is the one that accounts for the most printed lines.**
//!
//! Body-text digits (`2d` at x=167, `2e` at x=129 — real false hits, measured) account for almost
//! none, so prose loses by construction rather than by a threshold.
//!
//! ★★★ **The discriminator used to be MONOTONICITY — "line numbers increase down the page" — and it
//! shipped a silent drop.** A form prints each line number at least TWICE: in the margin, and again
//! in the gutter beside the amount box. Both columns climb, so monotonicity cannot choose between
//! them; the tie was being broken by token count, which systematically favours the gutter (it
//! spells out `2a`…`2t` where the margin abbreviates to a bare `b`, `c`, `d`). And the gutter is
//! structurally incomplete — a heading has no box, and a form with two money columns puts some
//! gutter labels ~95pt further left. On `f6251--2025` that silently dropped **line 1a, the seam to
//! Schedule 1-A**, and reported 59 tidy labels starting at `1b`. See [`witness_text`].

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

/// ★★ **Every plausible label column, found by CLUSTERING rather than by position.**
///
/// A candidate is any x-cluster of three or more numeric-label tokens. Nothing here decides *the*
/// column — that is [`witness_text`]'s job and it decides on the labels a column yields. This is
/// form-independent: it works at x=45 and at x=96 without being told either.
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
/// ★★★ **This returns ALL candidates, not one — and that change IS the 1a fix.** A form prints its
/// line numbers at least twice (margin, and again in the gutter beside the box), so "the" column is
/// a choice, and the choice must be made on the *labels a column yields*, not on how many tokens it
/// happens to contain. See [`witness_text`].
///
/// Returned left-to-right. An empty result means nothing looks like a numbered column at all —
/// which the caller must treat as a hard failure, never a quiet zero.
fn candidate_columns(words: &[Word]) -> Vec<f64> {
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

    let mut out: Vec<f64> = Vec::new();
    for (&b, group) in &by_bucket {
        let mut merged: Vec<&Tok> = group.clone();
        if let Some(n) = by_bucket.get(&(b + 1)) {
            merged.extend(n.iter().copied());
        }
        if merged.len() < 3 {
            continue; // a column of one or two numerals is prose, not a form's spine
        }
        // ★ NO monotonicity gate here, and that omission is MEASURED, not an oversight. Form 8949
        // prints only lines `1` and `2` per page, so its real label column's longest non-decreasing
        // run is 2 — a `run >= 3` admission test was tried and refused both f8949--2024 and
        // f8949--2025 outright, turning a 4-label census into a hard error. Body-text numerals are
        // rejected downstream instead, by yielding almost no labels: across all 32 archived forms,
        // admitting every cluster changed nothing but restoring f8949.
        let x = merged.iter().map(|t| t.x2).fold(0.0_f64, f64::max);
        if !out.iter().any(|prev| (prev - x).abs() < 0.5) {
            out.push(x);
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

/// The tokens one candidate column owns, in reading order, with same-row parents and sub-letters
/// already merged.
///
/// ★ Parents are matched on their RIGHT edge (the column is right-aligned). Sub-letters print bare
/// and hang just past that edge — measured: parent x2≈50.4, sub-letter `b` at x=50.0.
fn column_tokens(words: &[Word], right: f64) -> Vec<Tok> {
    let mut toks: Vec<Tok> = words
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
    merged
}

/// Resolve a column's tokens into labels, carrying the last numeric parent across bare sub-letters.
///
/// Returns the labels **and** the orphan complaint, rather than one or the other, so the same state
/// machine can both SCORE a candidate (where an orphan is just a token that yields no label) and
/// EXTRACT the winner (where an orphan is a hard error the caller must not swallow). Two copies of
/// this loop, one lenient and one strict, is exactly how the two would drift apart.
fn resolve(toks: &[Tok]) -> (Vec<(String, u32, f64)>, Option<String>) {
    let mut out: Vec<(String, u32, f64)> = Vec::new();
    let mut parent: Option<u32> = None;
    let mut orphan: Option<String> = None;
    for t in toks {
        if is_numeric_label(&t.text) {
            parent = numeric_part(&t.text);
            out.push((t.text.clone(), t.page, t.y));
        } else if let Some(p) = parent {
            out.push((format!("{p}{}", t.text), t.page, t.y));
        } else if orphan.is_none() {
            // A bare letter with no parent yet is a stray glyph, not a label — dropping it silently
            // would be a guess, so it is reported to the caller instead.
            orphan = Some(format!(
                "bare sub-letter `{}` at page {} y={} has no numeric parent — the state machine \
                 is out of step and the label set cannot be trusted",
                t.text, t.page, t.y
            ));
        }
    }
    out.dedup_by(|a, b| a.0 == b.0);
    (out, orphan)
}

/// **W1 — the text witness.** Every label the form prints in its margin column, sub-letters resolved.
///
/// ★ Sub-letters (`b`, `c`, `d`, `e`) print BARE and slightly indented — measured on Schedule 1-A:
/// parents at x=45, sub-letters at x=50. So the reader carries the last numeric parent, and the
/// indentation is a second, geometric confirmation on top of the state machine.
///
/// ★★★ **WHICH column, and why the old answer silently dropped Form 6251 line 1a.** An IRS form
/// prints each line's number **twice** — once in the left margin, once again in the gutter beside
/// the amount box — so there is always more than one monotone column, and picking the wrong one is
/// a silent, plausible-looking loss rather than an error. The old rule picked *the column with the
/// longest run of tokens*, and lost on both counts:
///
/// | | |
/// |---|---|
/// | **the margin is under-counted** | it is bucketed on the token's RIGHT edge, and a suffix hangs past it — `1a` ends at x2=55.28 where `2`, `3`… end at 50.40, so `1a` fell out of its own column's bucket |
/// | **the gutter is over-counted** | the margin abbreviates sub-rows to a bare `b`, `c`, `d`; the gutter spells out `2a`…`2t` in full. On Form 6251 that is 20 gutter tokens against 2 margin ones for the same 20 lines |
///
/// Measured on `f6251--2025`: margin 39 tokens, gutter 58. The gutter won — and the gutter is
/// **structurally incomplete**, because a form with two money columns puts some labels ~95pt further
/// left (line 1a's gutter label sits at x2≈404, alone). Result: a census of 59 labels starting at
/// `1b`, with `0 without a box` and no complaint. Line 1a is the seam to Schedule 1-A.
///
/// ★★ **So score a column by the LABELS IT YIELDS, not by the tokens it contains.** That is the
/// quantity the reader exists to produce, and it is immune to both distortions above: sub-letters
/// count once whether spelled out or abbreviated, and a column that omits a line scores lower for
/// omitting it. The margin can therefore never lose to a gutter that repeats a subset of it — the
/// margin prints every line, including the headings that have no box and can never appear in any
/// gutter at all. Ties go LEFTMOST, because a gutter is a repetition of a margin and never the
/// reverse.
pub fn witness_text(g: &Geometry) -> Result<Vec<(String, u32, f64)>, String> {
    let candidates = candidate_columns(&g.words);
    let mut best: Option<(usize, f64)> = None;
    for right in candidates {
        let labels = resolve(&column_tokens(&g.words, right)).0;
        let distinct: std::collections::BTreeSet<&str> =
            labels.iter().map(|(l, _, _)| l.as_str()).collect();
        // `>` not `>=`: candidates arrive left-to-right, so a tie keeps the leftmost.
        if best.is_none_or(|(n, _)| distinct.len() > n) {
            best = Some((distinct.len(), right));
        }
    }
    let (_, right) = best.ok_or_else(|| {
        "no numbered label column found — refusing to report zero labels".to_string()
    })?;

    let (labels, orphan) = resolve(&column_tokens(&g.words, right));
    match orphan {
        Some(e) => Err(e),
        None => Ok(labels),
    }
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

/// Does the row belonging to a label carry an amount box?
///
/// ★★★ **A ROW IS A SPAN, NOT A POINT — and a fixed tolerance is wrong.** The first version used
/// `y ± 12pt` and reported that Schedule 1-A **line 4a has no amount box**. Opening the PDF showed it
/// plainly does. The label `a` sits at the TOP of a three-line instruction paragraph while its box
/// aligns with the LAST line, ~36pt below, so the tolerance could never reach it.
///
/// ★ Note the failure mode: it produced a *plausible* answer (4a joined the two real headings 4 and
/// 22 in the "no box" list) that only looking at the page could refute. Two more lines of tolerance
/// would have hidden it again on some other form.
///
/// The correct model: a label owns the vertical span from its own `y` down to the next label's `y`
/// on the same page. Any box in that span is its box. That is exactly how the form reads.
/// ★★★ **A box belongs to the LAST label at or above its CENTRE.** Three models were tried; the
/// first two were refuted by the rendered page, and each fix would have hidden the other:
///
/// | model | what it got wrong |
/// |---|---|
/// | `y ± 12pt` fixed tolerance | reported **4a as box-less**. Its label sits at the top of a three-line paragraph, its box ~36pt below at the foot — out of reach of any fixed window. |
/// | span from label to next label, testing the box's **top** | reported **22 as HAVING a box**. Boxes are vertically centred on their row, so the 22a VIN box's top edge (159.0) sits a fraction above label `a` (161) and bleeds into line 22's span. |
/// | nearest label to the box centre | reported **11, 19, 28 as box-less**. On a two-line paragraph the box sits nearer the NEXT label than its own, so the next label steals it. |
///
/// ★ Using the box's **centre** fixes the bleed (a centre is unambiguously inside its own row), and
/// "last label at or above" fixes the theft (a box can never be claimed by a label printed below
/// it). Both failures were only visible because the form was opened and read.
pub fn assign_boxes(
    labels: &[(String, u32, f64)],
    boxes: &[(u32, f64, f64, String)],
) -> Vec<usize> {
    let mut counts = vec![0usize; labels.len()];
    for (bp, top, bottom, _) in boxes {
        let centre = (top + bottom) / 2.0;
        // The last label on this page whose y is at or above the box's centre. The small epsilon
        // covers a box centred a hair above its own label's text baseline.
        let owner = labels
            .iter()
            .enumerate()
            .filter(|(_, (_, lp, ly))| lp == bp && *ly <= centre + 2.0)
            .max_by(|(_, (_, _, a)), (_, (_, _, b))| a.total_cmp(b))
            .map(|(i, _)| i);
        if let Some(i) = owner {
            counts[i] += 1;
        }
    }
    counts
}

/// `cargo run -p xtask -- label-census <stem>` — run both witnesses and print the adjudicated rows.
///
/// ★ This is the human's view of the two witnesses side by side: every label the form prints, and
/// whether the AcroForm says its row carries an amount box. Rows the witnesses disagree about are
/// exactly the ones a person must adjudicate against the rendered page — which is how line 4a's
/// missing box was caught and how lines 4 and 22 were confirmed as genuine headings.
pub fn run(stem: &str) -> Result<(), String> {
    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    let labels = witness_text(&g)?;
    let boxes = witness_boxes(&g);
    let counts = assign_boxes(&labels, &boxes);

    let rows: Vec<Row> = labels
        .iter()
        .enumerate()
        .map(|(i, (label, page, _y))| {
            let has_box = counts[i] > 0;
            Row {
                label: label.clone(),
                page: *page,
                kind: if has_box { Kind::Amount } else { Kind::Heading },
                note: if has_box {
                    String::new()
                } else {
                    "no AcroForm box in this row's span — heading, or a non-money entry".to_string()
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

/// The DIRECTORY year for a form stem: `f6251--2025` → `2025`, `f6251--2026-DRAFT` → `2026`.
///
/// ★★ **`-DRAFT` is stripped for the DIRECTORY and kept everywhere else.** The marker stays in the
/// FILENAME on purpose — it is one of the three signals `authority_manifest::Entry::is_draft` reads,
/// and R20 is precisely that a draft under a clean stem is indistinguishable from a final. The year
/// directory is the YEAR: `design/forms/2026/` exists, `design/forms/2026-DRAFT/` never has.
///
/// ★★★ **This existed in `form_geometry::extract` and NOT here, and the asymmetry was the whole
/// defect.** [`proof`] parsed its own year with a bare `rsplit("--")`, so every archived draft
/// resolved to a directory that does not exist. Measured 2026-09-05 over the 16 committed
/// `*-DRAFT` geometry fixtures: `label-proof` was **OK=0 FAILED=16**, 15 of them looking in
/// `design/forms/2026-DRAFT/` and blaming *"not present (gitignored; re-fetch…)"* on a PDF that is
/// on disk — a message that sends the reader to re-download a file they already have. This is the
/// human-in-the-loop instrument whose own banner reads *"a defect the machines could not see"*, so
/// it was the one instrument a machine could not cover for.
///
/// ★★ **And there is no fallback year, where there used to be one that could not fire.** The old
/// expression was `stem.rsplit("--").next().unwrap_or("2025")`, but `rsplit` always yields at least
/// one item — `"f1040".rsplit("--").next()` is `Some("f1040")` — so `"2025"` was DEAD code reading
/// as a deliberate fallback-to-2025 policy, in one of the two sites most likely to be copied when a
/// new stem-consuming command is added (the other is `form_geometry.rs:194`, still to fix). A stem
/// that is not `<form>--<year>` is now a REFUSAL that names the shape it wanted.
pub(crate) fn stem_year(stem: &str) -> Result<&str, String> {
    let bad = || {
        format!(
            "`{stem}` is not a form stem: expected `<form>--<year>`, e.g. `f6251--2025` or \
             `f6251--2026-DRAFT`"
        )
    };
    let (form, year) = stem.rsplit_once("--").ok_or_else(bad)?;
    let year = year.trim_end_matches("-DRAFT");
    if form.is_empty() || year.is_empty() {
        return Err(bad());
    }
    Ok(year)
}

/// `cargo run -p xtask -- label-proof <stem>` — **the human-readable proof of the label→box join.**
///
/// ★★★ **Owner's idea, 2026-07-30, and it closes the residual risk the ④ consult named:** *"a
/// trailing sub-letter heading with no box and no sequence signature would evade both witnesses."*
/// Two mechanical witnesses can agree with each other and still both be wrong; a person looking at
/// the printed page cannot be fooled the same way.
///
/// Every AcroForm box on a BLANK form is filled with **the label this census assigned it** — not
/// `1, 2, 3…`. That distinction is the whole value:
///
/// - line 22a's boxes print `22a`. If the join is wrong you see `23` sitting in 22a's box, and the
///   error is obvious at a glance instead of buried in coordinates.
/// - a box no label claimed prints **`?`** — that is BOX-WITH-NO-LABEL, a dropped line, made visible.
/// - a line whose box stays blank is a heading (4, 22) or a missed box.
///
/// ★ It fills through the SHIPPED writer (`btctax_forms::apply_writes`), so the render also exercises
/// the path the real emitter uses, rather than a second implementation that could differ from it.
///
/// ★★ This is DIAGNOSTIC output, never a filed artifact: it writes to a scratch path and prints
/// where. Nothing here goes near a return.
pub fn proof(stem: &str, out_path: &str) -> Result<(), String> {
    use btctax_forms::testonly as bf;

    // ★ First, before anything else can fail on a malformed stem with a less useful message.
    let year = stem_year(stem)?;
    let root = crate::form_geometry::repo_root();
    let g = crate::form_geometry::load(&root, stem)?;
    let labels = witness_text(&g)?;
    let boxes = witness_boxes(&g);

    // Which label owns each box — the SAME assignment the census uses, so the render cannot flatter
    // the checker by computing the join a second, kinder way.
    let mut owner_of: Vec<Option<usize>> = Vec::with_capacity(boxes.len());
    for (bp, top, bottom, _) in &boxes {
        let centre = (top + bottom) / 2.0;
        owner_of.push(
            labels
                .iter()
                .enumerate()
                .filter(|(_, (_, lp, ly))| lp == bp && *ly <= centre + 2.0)
                .max_by(|(_, (_, _, a)), (_, (_, _, b))| a.total_cmp(b))
                .map(|(i, _)| i),
        );
    }

    let pdf = root.join(format!("design/forms/{year}/{stem}.pdf"));
    let bytes = std::fs::read(&pdf).map_err(|e| {
        format!(
            "{} not present (gitignored; re-fetch from its .pdf.txt note): {e}",
            pdf.display()
        )
    })?;
    let mut doc = bf::load(&bytes).map_err(|e| format!("parse {}: {e}", pdf.display()))?;
    bf::drop_xfa_and_set_needappearances(&mut doc)
        .map_err(|e| format!("preparing appearances: {e}"))?;
    let fields = bf::collect_fields(&doc).map_err(|e| format!("collect fields: {e}"))?;
    let idx = bf::index(&fields);

    let mut writes = Vec::new();
    let mut unclaimed = 0usize;
    for (b, owner) in boxes.iter().zip(&owner_of) {
        let text = match owner {
            Some(i) => labels[*i].0.clone(),
            None => {
                unclaimed += 1;
                "?".to_string()
            }
        };
        if idx.contains_key(&b.3) {
            writes.push((b.3.clone(), bf::FieldValue::Text(text)));
        }
    }
    bf::apply_writes(&mut doc, &idx, &writes).map_err(|e| format!("writing values: {e}"))?;
    bf::strip_nondeterminism(&mut doc);
    let out = bf::save(&mut doc).map_err(|e| format!("saving: {e}"))?;
    std::fs::write(out_path, &out).map_err(|e| format!("write {out_path}: {e}"))?;

    let claimed: std::collections::BTreeSet<&str> = owner_of
        .iter()
        .flatten()
        .map(|i| labels[*i].0.as_str())
        .collect();
    let boxless: Vec<&str> = labels
        .iter()
        .map(|(l, _, _)| l.as_str())
        .filter(|l| !claimed.contains(l))
        .collect();

    println!("label-proof: wrote {out_path}");
    println!(
        "  {} boxes filled with their assigned label; {unclaimed} printed `?` (BOX-WITH-NO-LABEL)",
        writes.len()
    );
    println!("  {} label(s) with no box: {boxless:?}", boxless.len());
    println!(
        "  ★ Open it. Every box should show the line it belongs to. A `?`, a blank that should"
    );
    println!(
        "    have a value, or a label in the wrong box is a defect the machines could not see."
    );
    Ok(())
}

/// `cargo run -p xtask -- label-boxes <stem>` — emit `FQN<TAB>label<TAB>page` for every AcroForm box.
///
/// ★ This is the **address map**: the field census keys on the FQN (exhaustive, authoritative, needs
/// no parsing), and the label is what makes a 244-row census readable by a person. `22a` means
/// something; `Line22b[0].f2_06[0]` does not.
///
/// A box no label claims emits `?` — that is BOX-WITH-NO-LABEL, and it is a hard finding, not a
/// blank cell.
pub fn boxes_tsv(stem: &str) -> Result<(), String> {
    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    let labels = witness_text(&g)?;
    let boxes = witness_boxes(&g);
    for (bp, top, bottom, name) in &boxes {
        let centre = (top + bottom) / 2.0;
        let label = labels
            .iter()
            .filter(|(_, lp, ly)| lp == bp && *ly <= centre + 2.0)
            .max_by(|(_, _, a), (_, _, b)| a.total_cmp(b))
            .map(|(l, _, _)| l.as_str())
            .unwrap_or("?");
        println!("{name}\t{label}\t{bp}");
    }
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

    fn f6251() -> Geometry {
        form_geometry::load(&form_geometry::repo_root(), "f6251--2025")
            .expect("Form 6251 (2025) geometry fixture")
    }

    /// ★★★ **THE KILL-TEST for the two-money-column drop (B1), and it is SYNTHETIC on purpose.**
    ///
    /// The form under test is the *shape* of the defect, not a particular form. IRS forms print each
    /// line's number **twice**: once in the left margin, once again in the right-hand gutter beside
    /// the amount box. The gutter is not one column — a form with an inner and an outer money column
    /// prints some gutter labels ~95pt further left. Here line **1a**'s gutter label sits in the
    /// inner column (x2≈404) while every other line's sits in the outer one (x2≈498).
    ///
    /// Two independent facts then conspire, and either alone is survivable:
    ///
    /// 1. the margin cluster is keyed on the token's RIGHT edge, so the suffixed token `1a`
    ///    (x2=55.28) falls out of its own column's bucket (`2`, `3`… end at 50.40) and the margin is
    ///    under-counted by exactly the lines that print an inline sub-letter;
    /// 2. the winning-column metric counts TOKENS, and the gutter spells out every sub-letter
    ///    (`2a`…`2e`) that the margin abbreviates to a bare letter — so the gutter always out-scores
    ///    the margin on a form with many sub-lettered lines.
    ///
    /// The gutter then wins, and **every line whose box is in the inner column disappears** — with
    /// the census still printing a long, monotone, entirely plausible list. That is the repo's
    /// "green and blind instrument" class exactly.
    ///
    /// ★ Coordinates are the MEASURED ones from `f6251--2025` (margin parents x=45.67/x2=50.40,
    /// sub-letters x=50.40, outer gutter x=488.24/x2=498.16, 1a's inner gutter x=394.80/x2=404.40),
    /// so this is the real geometry with the prose removed, not an invented layout.
    #[test]
    fn a_line_whose_gutter_label_sits_in_the_inner_money_column_is_not_dropped() {
        let mut words: Vec<Word> = Vec::new();
        let mut put = |x: f64, y: f64, x2: f64, t: &str| {
            words.push(Word {
                page: 1,
                x,
                y,
                x2,
                y2: y + 8.0,
                text: t.into(),
            });
        };

        // Line 1a — margin parent printed as ONE suffixed token; its gutter label is inset.
        put(45.67, 100.0, 55.28, "1a");
        put(394.80, 100.0, 404.40, "1a");
        // Line 1b — bare sub-letter in the margin, full label in the outer gutter.
        put(50.40, 112.0, 55.59, "b");
        put(488.24, 112.0, 498.16, "1b");
        // Line 2a — margin prints `2` and `a` as two tokens on ONE row.
        put(45.67, 124.0, 50.40, "2");
        put(50.40, 124.0, 55.28, "a");
        put(488.24, 124.0, 498.16, "2a");
        // Lines 2b–2e — bare sub-letters in the margin, spelled out in the gutter.
        for (i, sub) in ["b", "c", "d", "e"].iter().enumerate() {
            let y = 136.0 + 12.0 * i as f64;
            put(50.40, y, 55.59, sub);
            put(488.24, y, 498.16, &format!("2{sub}"));
        }
        // Lines 3–8 — plain numbered lines, printed identically in both columns.
        for n in 3..=8u32 {
            let y = 184.0 + 12.0 * f64::from(n - 3);
            put(45.67, y, 50.40, &n.to_string());
            put(488.24, y, 498.16, &n.to_string());
        }

        let g = Geometry {
            form: "synthetic-two-money-columns".into(),
            pdf_sha256: String::new(),
            pages: vec![crate::form_geometry::Page {
                n: 1,
                width: 612.0,
                height: 792.0,
            }],
            words,
            boxes: vec![],
        };

        let labels: Vec<String> = witness_text(&g)
            .expect("witness")
            .into_iter()
            .map(|(l, _, _)| l)
            .collect();

        assert_eq!(
            labels,
            vec!["1a", "1b", "2a", "2b", "2c", "2d", "2e", "3", "4", "5", "6", "7", "8"],
            "the reader locked onto the right-hand gutter and dropped every line whose amount box \
             sits in the INNER money column. `1a` is the whole set of them here. Got: {labels:?}"
        );
    }

    /// ★★★ **The live defect, on the form it was found on.** TY2025 split Form 6251's Part I line 1
    /// into `1a`/`1b` because Pub. L. 119-21 created Schedule 1-A, and **line 1a is the seam**: it
    /// reads *"Subtract Schedule 1-A (Form 1040), line 37, from Form 1040, 1040-SR, or 1040-NR,
    /// line 14"*. The census reported 59 labels beginning at `1b` — 1a silently gone, with `0
    /// without a box` and no complaint.
    ///
    /// ★ Transcribed from the TEXT LAYER (`design/forms/extract/f6251--2025.txt`), never the
    /// rendered page.
    #[test]
    fn form_6251_part_i_begins_at_1a_the_schedule_1a_seam() {
        let g = f6251();
        let labels: Vec<String> = witness_text(&g)
            .expect("witness")
            .into_iter()
            .map(|(l, _, _)| l)
            .collect();

        assert_eq!(
            labels.first().map(String::as_str),
            Some("1a"),
            "Part I opens at line 1a — the Schedule 1-A seam. Got: {:?}",
            &labels[..labels.len().min(6)]
        );
        assert!(
            labels.iter().any(|l| l == "1b"),
            "line 1b must survive alongside 1a: {labels:?}"
        );

        // The form's own Part I spine, read off the text layer: 1a, 1b, 2a–2t, 3, 4.
        let mut want: Vec<String> = vec!["1a".into(), "1b".into()];
        want.extend(('a'..='t').map(|c| format!("2{c}")));
        want.push("3".into());
        want.push("4".into());
        assert_eq!(
            labels[..want.len()],
            want[..],
            "Part I's printed label sequence does not match the form"
        );

        // ★ MEASURED, not assumed: the recovered set is the old 59 plus the dropped 1a.
        assert_eq!(labels.len(), 60, "printed label count changed: {labels:?}");
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

    /// ★★★ **THE COUNT — and the 50-vs-48 question, RESOLVED against the rendered page.**
    ///
    /// Both numbers were right; they were counting different things.
    ///
    /// | | |
    /// |---|---|
    /// | **50** | printed line labels — what the text witness enumerates |
    /// | **48** | of those, the ones that TAKE AN ENTRY — the hand-established figure |
    /// | **2** | headings with no box of their own: line **4** (heads 4a–4c) and line **22** (heads the VIN table, whose columns are (i)/(ii)/(iii) and whose rows 22a/22b line 23 then adds) |
    ///
    /// ★★ Nothing was tuned to make these agree. The witnesses were fixed against **the form**
    /// — three box-assignment models were tried and the first two were refuted by opening the PDF —
    /// and 48 fell out. That is the difference between an instrument that agrees with an expectation
    /// and one that is right: had the reader been nudged to 48 labels, the two headings would have
    /// vanished from the census entirely, which is precisely the "we forgot this line" defect it
    /// exists to catch.
    #[test]
    fn the_witnesses_resolve_the_50_vs_48_question() {
        let g = sch1a();
        let labels = witness_text(&g).expect("witness");
        let boxes = witness_boxes(&g);
        let counts = assign_boxes(&labels, &boxes);

        assert_eq!(labels.len(), 50, "printed label count changed: {labels:?}");

        let entry: Vec<&str> = labels
            .iter()
            .zip(&counts)
            .filter(|(_, c)| **c > 0)
            .map(|((l, _, _), _)| l.as_str())
            .collect();
        let headings: Vec<&str> = labels
            .iter()
            .zip(&counts)
            .filter(|(_, c)| **c == 0)
            .map(|((l, _, _), _)| l.as_str())
            .collect();

        assert_eq!(
            entry.len(),
            48,
            "the hand-established figure is 48 ENTRY lines; got {}: {entry:?}",
            entry.len()
        );
        assert_eq!(
            headings,
            vec!["4", "22"],
            "exactly lines 4 and 22 head their sub-rows and take no entry of their own"
        );
    }

    /// ★★ **Zero labels is ALWAYS a hard failure**    /// ★★ **Zero labels is ALWAYS a hard failure** — `LABEL_READER.md`'s rule, and the reason a
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
        let counts = assign_boxes(&labels, &boxes);
        let has = |name: &str| {
            let i = labels
                .iter()
                .position(|(l, _, _)| l == name)
                .expect("label present");
            counts[i] > 0
        };

        // ★ ADJUDICATED AGAINST THE RENDERED PAGE 2026-07-30. Lines 4 and 22 are instruction
        // paragraphs heading their lettered sub-rows and carry no box of their own; 22 heads the VIN
        // table whose columns are (i)/(ii)/(iii). Everything else here takes an entry.
        assert!(
            !has("4"),
            "line 4 is a HEADING and must carry no amount box"
        );
        assert!(
            !has("22"),
            "line 22 heads the VIN table and must carry no amount box"
        );
        assert!(
            has("1"),
            "line 1 is an entry line and must carry an amount box"
        );
        // ★★ The regression this span model exists for: 4a's box sits ~36pt below its label, at the
        // foot of a three-line paragraph. A fixed tolerance reported it box-less, which the PDF
        // refutes at a glance.
        assert!(
            has("4a"),
            "line 4a HAS an amount box — the row span must reach it"
        );
        assert!(has("22a") && has("22b"), "the VIN grid rows carry boxes");
    }

    /// ★★ **B1 — the planted defect is the one that shipped.** Delete `.trim_end_matches("-DRAFT")`
    /// from [`stem_year`] and this reds on the first `*-DRAFT` fixture it reads; delete the refusal
    /// and the last three assertions red. The stems are READ OFF `design/forms/geometry/`, never
    /// listed here, so a fixture committed tomorrow is covered with nobody remembering.
    ///
    /// ★ It asserts the DIRECTORY, not the PDF: `design/forms/**/*.pdf` is gitignored, so a test
    /// that reached for the file itself would be red in CI and green here — the exact shape this
    /// repo keeps finding.
    #[test]
    fn every_committed_geometry_stem_resolves_to_a_year_directory_that_exists() {
        let root = crate::form_geometry::repo_root();
        let mut drafts = 0usize;
        let mut seen = 0usize;
        for e in std::fs::read_dir(root.join("design/forms/geometry")).expect("geometry fixtures") {
            let f = e
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned();
            let Some(stem) = f.strip_suffix(".json") else {
                continue;
            };
            seen += 1;
            let year = stem_year(stem)
                .unwrap_or_else(|e| panic!("committed fixture `{stem}` is not a form stem: {e}"));
            assert!(
                year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()),
                "`{stem}` resolved to year directory `{year}` — `-DRAFT` belongs to the FILENAME, \
                 never to the directory"
            );
            assert!(
                root.join(format!("design/forms/{year}")).is_dir(),
                "`{stem}` resolves to design/forms/{year}/, which is not a directory"
            );
            if stem.ends_with("-DRAFT") {
                drafts += 1;
            }
        }
        // ★ NON-VACUITY, not a count ratchet. What must hold is that the `-DRAFT` strip is
        //   exercised by a real committed fixture; the fixture population itself moves whenever a
        //   form is archived or re-extracted, and a hard count here would red on that instead of on
        //   the defect. (Measured 2026-09-05: 47 fixtures, 15 of them drafts.)
        assert!(
            drafts >= 1 && seen >= drafts,
            "{seen} geometry fixture(s), {drafts} of them `-DRAFT`. With no draft fixture this test \
             exercises the `-DRAFT` strip on nothing and witnesses nothing."
        );
        assert!(
            !root.join("design/forms/2026-DRAFT").exists(),
            "design/forms/2026-DRAFT/ exists, so the pre-fix `rsplit(\"--\")` would now resolve to \
             a real directory and this test would stop witnessing anything"
        );

        // The refusal that replaced a fallback which could never fire.
        assert!(
            stem_year("f1040").is_err(),
            "a stem with no `--` must REFUSE"
        );
        assert!(
            stem_year("--2025").is_err(),
            "an empty form part must REFUSE"
        );
        assert!(
            stem_year("f1040--").is_err(),
            "an empty year part must REFUSE"
        );
        assert_eq!(stem_year("f6251--2026-DRAFT").as_deref(), Ok("2026"));
        assert_eq!(stem_year("f6251--2025").as_deref(), Ok("2025"));
    }
}

/// FQN → the printed line label beside that box, for one archived form. The same join `boxes_tsv`
/// prints, returned instead of printed so a conformance check can consume it.
///
/// ★★★ **This exists because "the field name is present" is NOT the check.** Measured on the real
/// artifacts: applying the TY2024 Form 6251 map to the TY2025 PDF leaves **0 of 61 field names
/// absent** — every existence check passes — while TY2025 *added* one field on page 1 and walked
/// everything below it down, with **zero renames**. TY2024's `line11`, the AMT itself, then prints
/// in the TY2025 form's **line-10 box**. A wrong number on signed testimony, and the exists-check is
/// structurally blind to it, because a rename is not what happened.
/// Every numbered-label WORD the page prints — `(label, page, top-down y, x2)` — raw, not column-
/// resolved. Used by the x-aware join for the one case a column reader cannot see: a label printed
/// INLINE immediately before its box on a row that also carries the margin label (the 1040's `2b`
/// at x≈489 beside the `2a`/`2b` row; Form 6251's `1a` at x≈395 before its box). A union over
/// resolved label COLUMNS was tried first and admitted prose numbers ("Form 1040, line 14") that
/// happen to align into a column — a raw word with a small gap to the box is the right candidate.
pub fn inline_label_words(g: &Geometry) -> Vec<(String, u32, f64, f64)> {
    g.words
        .iter()
        .filter(|w| is_numeric_label(&w.text))
        .map(|w| (w.text.clone(), w.page, w.y, w.x2))
        .collect()
}

/// Every box with its LEFT edge — `(page, top, bottom, left x, name)` — for the x-aware join.
pub fn witness_boxes_x(g: &Geometry) -> Vec<(u32, f64, f64, f64, String)> {
    g.boxes
        .iter()
        .filter_map(|b| {
            let (top, bottom) = g.box_top_down_y(b)?;
            Some((b.page, top, bottom, b.x, b.name.clone()))
        })
        .collect()
}

/// The line→box join: for every AcroForm box, the printed line label that governs it.
///
/// ★ x-AWARE (2026-09-06): within the box's own row (label centre inside the box's vertical span,
/// ±2pt), the label nearest to the LEFT of the box wins, across every label column — so the 1040's
/// `2b` cell joins to "2b" and its `2a` cell to "2a". Only when no label shares the row does the
/// older rule apply: the lowest label at or above the box centre in the primary column (a multi-line
/// item whose number prints on the first line, the box on a later one).
pub fn label_join(stem: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    let g = crate::form_geometry::load(&crate::form_geometry::repo_root(), stem)?;
    let primary = witness_text(&g)?;
    let inline = inline_label_words(&g);
    let boxes = witness_boxes_x(&g);
    let mut out = std::collections::BTreeMap::new();
    for (bp, top, bottom, left, name) in &boxes {
        let centre = (top + bottom) / 2.0;
        // "In the row" = the label's own vertical extent lies inside the box's span (labels are
        // ~9.6pt tall; rows are ~12pt apart, so the NEXT row's label top sits just below the box
        // bottom and must NOT qualify — a ±2pt band on the label's top edge admitted it and produced
        // a uniform off-by-one on first measurement) — AND the label ends within 12pt of the box's
        // left edge: an inline label is printed right before its box; a prose number is not.
        let in_row = inline
            .iter()
            .filter(|(_, lp, ly, lx2)| {
                lp == bp
                    && *ly >= top - 2.0
                    && *ly + 4.0 <= *bottom
                    && *lx2 <= left + 1.0
                    && left - *lx2 <= 12.0
            })
            .max_by(|(_, _, _, a), (_, _, _, b)| a.total_cmp(b))
            .map(|(l, _, _, _)| l.as_str());
        let label = in_row.unwrap_or_else(|| {
            primary
                .iter()
                .filter(|(_, lp, ly)| lp == bp && *ly <= centre + 2.0)
                .max_by(|(_, _, a), (_, _, b)| a.total_cmp(b))
                .map(|(l, _, _)| l.as_str())
                .unwrap_or("?")
        });
        out.insert(name.clone(), label.to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod map_label_join_tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn forms_root() -> PathBuf {
        crate::form_geometry::repo_root().join("crates/btctax-forms/forms")
    }

    /// `lineN = "<FQN>"` bindings from a committed map. Only the plain per-line ones — repeating
    /// grids address rows positionally and carry no single printed label.
    fn line_bindings(path: &std::path::Path) -> BTreeMap<String, String> {
        let text = std::fs::read_to_string(path).unwrap();
        let mut out = BTreeMap::new();
        for l in text.lines() {
            let l = l.trim();
            if l.starts_with('#') {
                continue;
            }
            if let Some((lhs, rhs)) = l.split_once('=') {
                let key = lhs.trim();
                if !key.starts_with("line") {
                    continue;
                }
                let line = &key["line".len()..];
                // Only NUMBERED lines carry a printed label (`line7a`, `line22b`); a named cell
                // (`line_a_business`, Schedule C's `line_b_naics`) is not a line on the page and
                // `label_matches` would compare its empty first token to nothing.
                if !line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    continue;
                }
                // ★ The FIRST quoted token on the right-hand side — not "the whole RHS is one quoted
                //   string". Every binding that carried a trailing `# comment` used to be dropped
                //   SILENTLY here: TY2025's Schedule A contributed ZERO of its nineteen bindings to
                //   the join while the year's floor stayed green (steps-4/5 review R2).
                let rhs = rhs.trim();
                let v = rhs
                    .strip_prefix('"')
                    .and_then(|s| s.split_once('"'))
                    .map(|(v, _)| v);
                if let Some(v) = v {
                    if v.contains('[') && v.contains('.') {
                        out.insert(line.to_string(), v.to_string());
                    }
                }
            }
        }
        out
    }

    /// Does a map key's line part name the same line the form printed beside the box?
    ///
    /// Exact match is the rule. Two NARROW normalisations are allowed, each because the schema and
    /// the printed page legitimately disagree about granularity — not to make failures go away:
    ///
    /// * a `_suffix` on the key (`7b_countries`) is btctax naming two cells of one printed line;
    /// * a trailing LETTER on the key where the form prints only the number (`7a` vs `7`) is the
    ///   1040's capital-gain line, printed once and modelled as a sub-line.
    ///
    /// ★ Only a trailing *letter* is stripped, never a digit — `line11` must not be accepted
    /// against a printed `1`, which is precisely the off-by-one this check exists to catch.
    fn label_matches(key: &str, printed: &str) -> bool {
        let base = key.split('_').next().unwrap_or(key);
        if base == printed {
            return true;
        }
        let trimmed = base.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        !trimmed.is_empty() && trimmed != base && trimmed == printed
    }

    /// sha256 of a byte slice, lowercase hex.
    fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The `pdf_sha256` a committed geometry fixture records, without parsing its megabyte of words.
    ///
    /// ★ It VALIDATES the field it lifts (64 hex characters) and errors by name otherwise, rather
    /// than returning an empty string that would silently match nothing.
    fn fixture_pdf_sha256(path: &std::path::Path) -> Result<String, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let v = text
            .split("\"pdf_sha256\":\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .ok_or_else(|| format!("{} records no pdf_sha256", path.display()))?;
        if v.len() != 64 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{} records a malformed pdf_sha256", path.display()));
        }
        Ok(v.to_string())
    }

    /// One map on disk, and the geometry fixture that observed **the very PDF it was written
    /// against** — or the reason there is none.
    struct MapOnDisk {
        form: String,
        year: String,
        path: PathBuf,
        /// `Ok(stem)`, or `Err(reason)` — never a silently-absent join.
        stem: Result<String, String>,
    }

    /// Every map on disk, paired to its geometry by the **sha256 of the bundled PDF**, never by name.
    ///
    /// ★★★ **The name join was wrong, and it hid Schedule D.** This built the stem as
    /// `format!("{form}--{year}")` using the MAP's spelling, while the fixtures are committed under
    /// the IRS's: the maps are `schedule_d` and `schedule_se`, the fixtures are `f1040sd--2024` and
    /// `f1040sse--2025`. `crate::form_geometry::load` then failed on a fixture that does not exist,
    /// the map was counted as *unwitnessed*, and the count was `eprintln!`'d. Measured 2026-09-05:
    /// **30 line→label joins whose fixtures were on disk the whole time** — and **Schedule D is
    /// where btctax's capital gain lands**, so no committed test joined any of its lines to its
    /// printed label, in any year.
    ///
    /// ★★ **Hashing is not a workaround for the name — it is a stronger join.** A name match pairs
    /// a map with *a* fixture for that year; the hash pairs it with the fixture observed from
    /// **byte-identical bytes**, which is the claim the map's own header makes (*"a byte-for-byte
    /// copy of design/forms/2025/f8959--2025.pdf"*) and which nothing checked. A revised PDF
    /// bundled under an unchanged name now reports itself instead of joining to stale geometry.
    /// Measured 2026-09-05: of the 37 bundled map PDFs, 31 match exactly one fixture and none
    /// matches two; the 6 that match none are TY2017's five and `2024/f8283.pdf`, all named below.
    ///
    /// ★ A map with no match is carried as an `Err` reason and gated in
    /// [`every_mapped_line_lands_on_its_own_printed_label`], never dropped.
    fn every_map() -> Vec<MapOnDisk> {
        // sha256 → fixture stems observed from that PDF.
        let mut by_hash: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let geom_dir = crate::form_geometry::repo_root().join("design/forms/geometry");
        let mut fixtures: Vec<PathBuf> = std::fs::read_dir(&geom_dir)
            .expect("design/forms/geometry exists")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        fixtures.sort();
        for f in &fixtures {
            let stem = f
                .file_stem()
                .expect("a .json has a stem")
                .to_string_lossy()
                .into_owned();
            let h = fixture_pdf_sha256(f).unwrap_or_else(|e| panic!("{e}"));
            by_hash.entry(h).or_default().push(stem);
        }

        let mut out = Vec::new();
        let mut years: Vec<_> = std::fs::read_dir(forms_root())
            .expect("forms/ exists")
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        years.sort();
        for y in years {
            let dir = forms_root().join(&y);
            let mut maps: Vec<_> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.to_string_lossy().ends_with(".map.toml"))
                .collect();
            maps.sort();
            for m in maps {
                let form = m
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .trim_end_matches(".map.toml")
                    .to_string();
                let pdf = dir.join(format!("{form}.pdf"));
                let stem = match std::fs::read(&pdf) {
                    Err(e) => Err(format!("no bundled PDF at {}: {e}", pdf.display())),
                    Ok(bytes) => {
                        let h = sha256_hex(&bytes);
                        match by_hash.get(&h).map(Vec::as_slice) {
                            None => Err(format!(
                                "no geometry fixture was observed from {} (sha256:{}…) — generate \
                                 one with `xtask extract-geometry <stem>`",
                                pdf.display(),
                                &h[..12]
                            )),
                            Some([one]) => Ok(one.clone()),
                            Some(many) => Err(format!(
                                "{} matches {} fixtures ({many:?}); the join is ambiguous",
                                pdf.display(),
                                many.len()
                            )),
                        }
                    }
                };
                out.push(MapOnDisk {
                    form,
                    year: y.clone(),
                    path: m,
                    stem,
                });
            }
        }
        out
    }

    /// What one year's maps actually got checked, MEASURED — never assumed from the year existing.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct YearReach {
        year: String,
        maps: usize,
        /// `<form> — <why>`, one per map this check could not reach. Named, never counted.
        unwitnessed: Vec<String>,
        /// line→label joins actually compared for this year.
        joins: usize,
    }

    /// The recorded reach floor for ONE tax year.
    ///
    /// ★★★ **The years are NOT enumerated here.** They are read off the maps directory, and the
    /// audit runs in BOTH directions: a year on disk with no entry below fails ("a new tax year
    /// arrived with no recorded floor"), and an entry below with no maps on disk fails ("the year
    /// this floor guards has vanished"). A one-directional check keyed to a list reds on nothing
    /// when an item is dropped — that exact defect has shipped in this repo.
    struct YearFloor {
        year: &'static str,
        min_joins: usize,
        max_unwitnessed: usize,
        /// Why a permissive floor is the right one. Required whenever this floor tolerates zero
        /// joins or an unreachable map — a recorded "no" needs a reason, or it is just an omission.
        why: &'static str,
    }

    /// ★ RATCHETS. Raise them when coverage grows; never lower one to make a red go away.
    const YEAR_FLOORS: &[YearFloor] = &[
        YearFloor {
            year: "2017",
            min_joins: 0,
            max_unwitnessed: 5,
            why: "TY2017 is the historical corpus year. None of its five forms is archived under \
                  design/forms/2017/, so no geometry fixture can be generated and no line->label \
                  join is reachable. Nothing emits a TY2017 return.",
        },
        YearFloor {
            year: "2024",
            // 99 → 235 on 2026-09-06: the binding parser dropped every `line = "…" # comment` (R2),
            // and the join gained the x-aware in-row rule. Measured, not estimated.
            min_joins: 235,
            max_unwitnessed: 1,
            why: "f8283 — design/forms/ holds i8283--2024 (the instructions) but not the form, so \
                  there is no PDF to extract geometry from.",
        },
        YearFloor {
            year: "2025",
            // 82 → 193 on 2026-09-06 (same fix as 2024). Every TY2025 map contributes; the per-map
            // zero-join check below is what would say otherwise.
            min_joins: 193,
            max_unwitnessed: 0,
            why: "every TY2025 form is archived with geometry; nothing is unreachable",
        },
    ];

    /// Both directions, and every failure names what it is about.
    fn audit_year_reach(observed: &[YearReach], floors: &[YearFloor]) -> Result<(), String> {
        let mut bad: Vec<String> = Vec::new();
        for o in observed {
            match floors.iter().find(|f| f.year == o.year) {
                None => bad.push(format!(
                    "{}: {} map(s) on disk and {} join(s) checked, but NO coverage floor is \
                     recorded. A new tax year must not arrive silently — record a floor with the \
                     measured numbers, or with 0 and the reason 0 is right.",
                    o.year, o.maps, o.joins
                )),
                Some(f) => {
                    if o.joins < f.min_joins {
                        bad.push(format!(
                            "{}: only {} line->label join(s) checked, floor is {}. Coverage for \
                             this year FELL — a geometry fixture or a map binding has gone missing.",
                            o.year, o.joins, f.min_joins
                        ));
                    }
                    if o.unwitnessed.len() > f.max_unwitnessed {
                        bad.push(format!(
                            "{}: {} map(s) this check cannot reach, allowance is {}:\n      {}",
                            o.year,
                            o.unwitnessed.len(),
                            f.max_unwitnessed,
                            o.unwitnessed.join("\n      ")
                        ));
                    }
                }
            }
        }
        for f in floors {
            if !observed.iter().any(|o| o.year == f.year) {
                bad.push(format!(
                    "{}: a coverage floor is recorded, but no maps for that year are on disk. \
                     Either the year was deleted (and this floor with it), or forms_root() is \
                     pointing somewhere else.",
                    f.year
                ));
            }
            if (f.min_joins == 0 || f.max_unwitnessed > 0) && f.why.trim().is_empty() {
                bad.push(format!(
                    "{}: a floor that tolerates zero joins or an unreachable map must record WHY.",
                    f.year
                ));
            }
        }
        if bad.is_empty() {
            Ok(())
        } else {
            Err(bad.join("\n  "))
        }
    }

    /// ★★★ **A mapped line must land on the box the form prints that line's number beside.**
    ///
    /// The existence check in `btctax-forms/tests/map_pdf_conformance.rs` is necessary and NOT
    /// sufficient, and the gap is not hypothetical — it is measured on the committed artifacts:
    /// applying the TY2024 Form 6251 map to the TY2025 PDF leaves **0 of 61 field names absent**,
    /// so every existence assertion passes, while **12 of 41 mapped lines land on the wrong printed
    /// label** — TY2024's `line11`, the AMT itself, in the TY2025 form's **line-10 box**. TY2025
    /// added one field on page 1 and walked everything below it down, with **zero renames**. A
    /// rename check cannot see a renumber.
    ///
    /// ★★★ **Coverage is asserted PER YEAR, and that is the whole point of this rewrite.** It used
    /// to be one global `checked >= 151` across every year at once, with the unreachable maps
    /// `eprintln!`'d and never asserted. A TY2026 whose geometry was never generated then
    /// contributes **0 joins** and the gate stays green on 2024+2025 coverage — the product fails
    /// closed, the instrument failed open. Per-year floors plus the both-directions year audit make
    /// a new year's arrival, and an old year's disappearance, both failures.
    #[test]
    fn every_mapped_line_lands_on_its_own_printed_label() {
        let mut wrong: Vec<String> = Vec::new();
        let mut reach: Vec<YearReach> = Vec::new();
        let mut per_map_zero: Vec<String> = Vec::new();
        let mut unlabelled = 0usize;

        for m in every_map() {
            if reach.last().map(|r| r.year.as_str()) != Some(m.year.as_str()) {
                reach.push(YearReach {
                    year: m.year.clone(),
                    maps: 0,
                    unwitnessed: Vec::new(),
                    joins: 0,
                });
            }
            let r = reach.last_mut().expect("just pushed");
            r.maps += 1;
            let stem = match &m.stem {
                Ok(s) => s.clone(),
                Err(why) => {
                    r.unwitnessed.push(format!("{} — {why}", m.form));
                    continue;
                }
            };
            let join = match label_join(&stem) {
                Ok(j) => j,
                Err(why) => {
                    r.unwitnessed.push(format!("{} — {stem}: {why}", m.form));
                    continue;
                }
            };
            let bindings = line_bindings(&m.path);
            let (mut map_joined, mut map_unboxed) = (0usize, 0usize);
            for (line, fqn) in &bindings {
                let Some(got) = join.get(fqn) else {
                    map_unboxed += 1; // the FQN is not a box the geometry fixture holds — counted, not silent
                    continue;
                };
                if got == "?" {
                    unlabelled += 1;
                    continue; // the reader could not witness a label for this box
                }
                r.joins += 1;
                map_joined += 1;
                if !label_matches(line, got) {
                    wrong.push(format!(
                        "{}/{}: map says line {line} -> {fqn}, but the form prints \"{got}\" beside that box",
                        m.year, m.form
                    ));
                }
            }
            // ★ Per-map reach, printed on every run: a map with bindings that contributes ZERO joins
            //   is the shape R2 found — the aggregate floor cannot see it, this line can.
            eprintln!(
                "  {}/{}: {} numbered bindings, {} joined, {} not a geometry box",
                m.year,
                m.form,
                bindings.len(),
                map_joined,
                map_unboxed
            );
            if !bindings.is_empty() && map_joined == 0 {
                per_map_zero.push(format!(
                    "{}/{} ({} bindings, 0 joined)",
                    m.year,
                    m.form,
                    bindings.len()
                ));
            }
        }

        // ★ The unreachable maps are printed BY NAME on every run, not only on failure. A count
        //   is what the old `eprintln!` gave, and a count is exactly what cannot distinguish "this
        //   form has no archived PDF" from "we forgot to extract its geometry".
        for r in &reach {
            eprintln!(
                "{}: {} map(s), {} join(s) checked, {} unreachable",
                r.year,
                r.maps,
                r.joins,
                r.unwitnessed.len()
            );
            for u in &r.unwitnessed {
                eprintln!("      NOT WITNESSED {u}");
            }
        }
        eprintln!("{unlabelled} binding(s) landed on a box the reader could not label (`?`)");

        assert!(
            wrong.is_empty(),
            "{} mapped line(s) land on a box the form labels differently — a filled value would \
             print on the WRONG LINE of a signed return:\n  {}",
            wrong.len(),
            wrong.join("\n  ")
        );
        assert!(
            per_map_zero.is_empty(),
            "maps with line bindings and ZERO joins — the join never examined them (R2): {per_map_zero:?}"
        );
        if let Err(e) = audit_year_reach(&reach, YEAR_FLOORS) {
            panic!("per-year coverage of the line->label join is not what was recorded:\n  {e}");
        }
    }

    /// ★★ **B1 for the per-year audit — the planted defects are synthetic, and deliberately so.**
    ///
    /// [`audit_year_reach`] is a pure function over a measured census precisely so its kill test can
    /// plant the four failures without mutating the shared tree: a new year's maps landing with no
    /// floor, a recorded year vanishing, coverage falling, and a geometry fixture going missing. A
    /// manual one-off mutation proves it once; this proves it on every run, forever.
    #[test]
    fn the_per_year_audit_reds_on_a_new_year_a_lost_year_lost_coverage_and_a_lost_fixture() {
        let floors = &[YearFloor {
            year: "2025",
            min_joins: 100,
            max_unwitnessed: 0,
            why: "",
        }];
        let good = |joins: usize, unwitnessed: Vec<String>| YearReach {
            year: "2025".to_string(),
            maps: 15,
            unwitnessed,
            joins,
        };
        audit_year_reach(&[good(100, vec![])], floors).expect("the calibrated case must PASS");

        // (1) TY2026's maps land, its geometry was never generated: 0 joins, no floor.
        let e = audit_year_reach(
            &[
                good(100, vec![]),
                YearReach {
                    year: "2026".to_string(),
                    maps: 12,
                    unwitnessed: vec!["f6251 — no geometry fixture".to_string()],
                    joins: 0,
                },
            ],
            floors,
        )
        .expect_err("a year with maps and no recorded floor must FAIL");
        assert!(e.contains("2026") && e.contains("NO coverage floor"), "{e}");

        // (2) the guarded year disappears — the direction a per-year list is blind to.
        let e = audit_year_reach(&[], floors).expect_err("a vanished year must FAIL");
        assert!(
            e.contains("2025") && e.contains("no maps for that year"),
            "{e}"
        );

        // (3) coverage falls below the recorded floor.
        let e = audit_year_reach(&[good(99, vec![])], floors)
            .expect_err("coverage below the floor must FAIL");
        assert!(e.contains("Coverage for this year FELL"), "{e}");

        // (4) a geometry fixture goes missing, so a map stops being checked.
        let e = audit_year_reach(&[good(100, vec!["schedule_d — gone".to_string()])], floors)
            .expect_err("an unreachable map must FAIL");
        assert!(
            e.contains("cannot reach") && e.contains("schedule_d"),
            "{e}"
        );

        // (5) a permissive floor with no recorded reason.
        let e = audit_year_reach(
            &[good(100, vec![])],
            &[YearFloor {
                year: "2025",
                min_joins: 0,
                max_unwitnessed: 0,
                why: "  ",
            }],
        )
        .expect_err("a zero floor with no reason must FAIL");
        assert!(e.contains("must record WHY"), "{e}");
    }

    /// ★★★ **B1 for the hash join — Schedule D, by name, in the test that was blind to it.**
    ///
    /// The planted defect is the shipped one: build the stem from the MAP's spelling. This asserts
    /// that the fixture the old code looked for **does not exist** (so the old path reached
    /// nothing), that the hash join finds the IRS-spelled fixture instead, and that Schedule D's
    /// bindings actually join — the form btctax's capital gain lands on.
    #[test]
    fn schedule_d_joins_through_the_pdf_hash_because_its_map_name_is_not_its_irs_stem() {
        let root = crate::form_geometry::repo_root();
        let maps = every_map();
        let mut checked = 0usize;
        for form in ["schedule_d", "schedule_se"] {
            for year in ["2024", "2025"] {
                let m = maps
                    .iter()
                    .find(|m| m.form == form && m.year == year)
                    .unwrap_or_else(|| panic!("{year}/{form}.map.toml is on disk"));
                let stem = m
                    .stem
                    .as_ref()
                    .unwrap_or_else(|e| panic!("{year}/{form} must join to a fixture: {e}"));
                assert_ne!(
                    stem,
                    &format!("{form}--{year}"),
                    "the map spelling must not be the fixture spelling, or this test proves nothing"
                );
                assert!(
                    !crate::form_geometry::geometry_path(&root, &format!("{form}--{year}"))
                        .exists(),
                    "design/forms/geometry/{form}--{year}.json exists, so the OLD name join would \
                     have worked and this test no longer witnesses the defect"
                );
                let join = label_join(stem).expect("the hash-joined fixture loads");
                let bound = line_bindings(&m.path);
                assert!(!bound.is_empty(), "{year}/{form} binds no plain lines");
                for (line, fqn) in bound {
                    if let Some(got) = join.get(&fqn) {
                        if got != "?" {
                            checked += 1;
                            assert!(
                                label_matches(&line, got),
                                "{year}/{form}: map says line {line} -> {fqn}, but the form prints \
                                 \"{got}\" beside that box"
                            );
                        }
                    }
                }
            }
        }
        assert!(
            checked >= 30,
            "measured 2026-09-05: 30 Schedule D / Schedule SE line->label joins were unreachable \
             under the name join and are reachable under the hash join; only {checked} arrived"
        );
    }

    /// ★★ **B1 — the kill test, and its planted defect is REAL HISTORY rather than an invention.**
    ///
    /// The exact near-miss the check above exists to prevent: the TY2024 Form 6251 map against the
    /// TY2025 form. Every field name still exists, so an existence check is green; the join is not.
    /// If this ever stops reporting wrong bindings, the join check has gone blind.
    #[test]
    fn the_join_check_reds_on_a_wrong_year_map_that_an_existence_check_would_pass() {
        let join = label_join("f6251--2025").expect("TY2025 Form 6251 geometry fixture");
        let m2024 = line_bindings(&forms_root().join("2024/f6251.map.toml"));
        let m2025 = line_bindings(&forms_root().join("2025/f6251.map.toml"));

        let count_wrong = |m: &BTreeMap<String, String>| {
            m.iter()
                .filter(|(line, fqn)| {
                    join.get(*fqn)
                        .is_some_and(|g| g != "?" && !label_matches(line, g))
                })
                .count()
        };

        assert_eq!(
            count_wrong(&m2025),
            0,
            "the COMMITTED TY2025 map must land every line on its own label, or the check is \
             calibrated wrong and its red below means nothing"
        );
        assert!(
            count_wrong(&m2024) >= 10,
            "the TY2024 map applied to the TY2025 form must be caught landing lines on the wrong \
             labels (measured: 12 of 41). If this is now 0, the join check has gone blind — and an \
             existence check passes on this exact input, with 0 of 61 field names absent."
        );
    }
}

/// The same join, for `form_delta`. A thin alias so the doc above stays the single explanation.
pub fn label_join_public(stem: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    label_join(stem)
}
