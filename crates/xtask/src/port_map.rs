//! `cargo run -p xtask -- port-map <stem> <prior-tag> <new-tag>` — a CANDIDATE field map for a new
//! revision, derived from the prior year's map and refusing every cell it cannot prove.
//!
//! ## Why this exists
//!
//! In January a year's forms are ported at once, under time pressure, for the year that is actually
//! filed. Porting **one** form by hand (f1040, TY2024 → TY2025, 2026-09-20/21) hit four traps, and not
//! one of them is visible to a carry-forward of last year's map:
//!
//! | trap | what it looked like |
//! |---|---|
//! | an FQN exists but is a DIFFERENT cell | TY2024's `taxpayer_ssn` is a `/MaxLen 2` box on TY2025 |
//! | a checkbox's on-state ORDER changed | MFS is `3` on TY2025 and `4` on TY2024 |
//! | the page break MOVED | the descent oracle compared a page-1 `y` with a page-2 `y` |
//! | printed labels RENUMBERED | AGI at `11a`, the deduction at `12e`, QBI at `13a` |
//!
//! The first would have printed a truncated SSN; the second would have filed the return under a
//! different filing status, taking every bracket and phase-out with it. Both were caught by a guard
//! firing at fill time, which is to say: by luck of having a fill to run.
//!
//! ## What it does, and what it refuses to do
//!
//! Every cell of the prior map is re-pointed through [`crate::form_delta::pair_fields`] — the suffix
//! ladder already used by `form-delta` — and then **verified five ways** before it is emitted:
//!
//! 1. **the printed LABEL** beside the new widget equals the cell's own key, because this repo requires
//!    a key to BE the label (`label_reader::every_mapped_line_lands_on_its_own_printed_label`);
//! 2. **the COLUMN band** matches the prior cell's, because a line's sub-line and amount widgets share
//!    one printed label and the label check cannot tell them apart;
//! 3. **`/MaxLen` compatibility** — a cell that held nine digits must still hold nine;
//! 4. **the on-state is one the new widget DECLARES**, for every checkbox;
//! 5. **the PAGE**, reported rather than enforced: the emitter derives descent groups from the page now,
//!    so a moved break is information, not an error.
//!
//! A cell failing any check is **not emitted**. It is listed with the measurement that killed it, for a
//! human to resolve against the form. This tool proposes; it never decides.
//!
//! ★★ **It is not a golden regenerator.** Its output is a CANDIDATE to review, and
//! `tests::the_f1040_ty2025_port_reproduces_the_hand_written_map` holds it to the one port that was done
//! by hand — a known answer, not its own previous output.

use std::collections::{BTreeMap, BTreeSet};

/// The three amount columns of the 1040 family, as centre-x bands (`form1040_full`'s own).
const BANDS: [(&str, f64, f64); 3] = [
    ("SUBLINE", 252.0, 324.0),
    ("MID", 410.0, 482.0),
    ("AMOUNT", 504.0, 576.0),
];

fn band_of(x: f64) -> String {
    BANDS
        .iter()
        .find(|(_, lo, hi)| x >= lo - 1.0 && x <= hi + 1.0)
        .map_or_else(|| format!("other({x:.0})"), |(n, _, _)| (*n).to_string())
}

/// One cell of a map: its key, the FQN it names, and the on-state when it is a checkbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub key: String,
    pub fqn: String,
    pub on: Option<String>,
}

/// Every `key = "fqn"` and `key = { field = "fqn", on = "N" }` in a committed map, in file order.
///
/// ★ Deliberately a TEXT scan rather than a `toml` parse: the keys of a map are per-revision (TY2025
/// spells AGI `line11a`), so a typed parse would silently normalise the very thing being ported. The
/// first quoted span is the FQN — a trailing `# comment` on the same line is common in these files and
/// broke an earlier reader that used `trim_matches('"')`.
#[must_use]
pub fn cells_of(map_text: &str) -> Vec<Cell> {
    let mut out = Vec::new();
    for line in map_text.lines() {
        let t = line.trim_start();
        if t.starts_with('#') || t.starts_with('[') {
            continue;
        }
        let Some((k, v)) = t.split_once('=') else {
            continue;
        };
        let key = k.trim();
        if key.is_empty() || !key.chars().next().is_some_and(char::is_alphabetic) {
            continue;
        }
        let v = v.trim();
        let Some(fqn) = v.split('"').nth(1) else {
            continue;
        };
        if !fqn.starts_with("topmostSubform") {
            continue;
        }
        let on = v.split("on = \"").nth(1).and_then(|r| r.split('"').next());
        out.push(Cell {
            key: key.to_string(),
            fqn: fqn.to_string(),
            on: on.map(str::to_string),
        });
    }
    out
}

/// What a widget looks like on one revision: where it sits, how much it holds, what it declares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shape {
    pub page: u32,
    pub band: String,
    pub max_len: Option<usize>,
    pub on_states: BTreeSet<String>,
}

/// Read every widget's shape from a bundled template PDF.
pub fn shapes(pdf: &std::path::Path) -> Result<BTreeMap<String, Shape>, String> {
    let bytes = std::fs::read(pdf).map_err(|e| format!("{}: {e}", pdf.display()))?;
    let doc = btctax_forms::testonly::load(&bytes).map_err(|e| format!("load: {e}"))?;
    let fields =
        btctax_forms::testonly::collect_fields(&doc).map_err(|e| format!("fields: {e}"))?;
    let mut out = BTreeMap::new();
    for f in &fields {
        let (page, band) = f.rect.map_or((0, String::new()), |r| {
            (
                u32::from(f.fqn.contains("Page2")) + 1,
                band_of(f64::from(r[0])),
            )
        });
        out.insert(
            f.fqn.clone(),
            Shape {
                page,
                band,
                max_len: f.max_len,
                on_states: if f.is_button {
                    btctax_forms::testonly::button_on_states(&doc, f.id)
                        .into_iter()
                        .collect()
                } else {
                    BTreeSet::new()
                },
            },
        );
    }
    Ok(out)
}

/// A cell the tool would not emit, and the measurement that killed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub key: String,
    pub candidate: String,
    pub why: String,
}

/// The result of a port: cells that passed every check, cells refused, and the page moves.
#[derive(Debug, Default)]
pub struct Port {
    pub emitted: Vec<Cell>,
    pub refused: Vec<Refused>,
    /// `key: prior page → new page`, reported because the emitter derives descent groups from the page.
    pub page_moves: Vec<String>,
}

/// Derive a candidate map from `prior`'s cells.
///
/// ★★★ **THE PRINTED LABEL IS THE IDENTITY; THE FQN IS AN IMPLEMENTATION DETAIL.** The first version of
/// this paired cells through [`crate::form_delta::pair_fields`], the suffix ladder `form-delta` uses.
/// That pairs by FQN NAME, and it produced **0 of 102** usable cells: TY2025 inserted widgets above the
/// income block, so `f1_47` names line 4b on TY2024 and line 1a on TY2025. The ladder is right for
/// COUNTING what changed — its actual job — and wrong for deciding what a cell means.
///
/// So a line cell is carried by `(printed label, column band)`: the new widget is the one this form
/// prints the SAME label beside, in the SAME column. That transfers every line the revision did not
/// renumber, and refuses the ones it did — `11 → 11a` cannot be inferred, it has to be read, which is
/// exactly the judgement a tool must not make.
///
/// ★★ Cells with no printed line label — the identity block, the filing-status radios, the dependents
/// grid — are refused by design. By hand they were resolved from column x, `/MaxLen`, row order and the
/// caption above; none of those is a single fact this tool could check, and a wrong identity cell prints
/// somebody's SSN in a date box. They are named in the report so the count of remaining work is honest.
pub fn port(
    prior: &[Cell],
    old_shapes: &BTreeMap<String, Shape>,
    new_shapes: &BTreeMap<String, Shape>,
    old_labels: &BTreeMap<String, String>,
    new_labels: &BTreeMap<String, String>,
) -> Port {
    // The new revision indexed the way a port needs it: (label, band) -> the widgets that match.
    let mut by_label_band: BTreeMap<(String, String), Vec<&String>> = BTreeMap::new();
    for (fqn, label) in new_labels {
        if let Some(sh) = new_shapes.get(fqn) {
            by_label_band
                .entry((label.clone(), sh.band.clone()))
                .or_default()
                .push(fqn);
        }
    }

    let mut p = Port::default();
    for c in prior {
        let Some(was) = old_shapes.get(&c.fqn) else {
            p.refused.push(Refused {
                key: c.key.clone(),
                candidate: String::new(),
                why: format!("{} is not a widget of the prior template", c.fqn),
            });
            continue;
        };
        // Only LINE cells have a printed label to carry. Everything else is the identity block's.
        let Some(want_label) = c
            .key
            .strip_prefix("line")
            .filter(|r| r.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        else {
            p.refused.push(Refused {
                key: c.key.clone(),
                candidate: String::new(),
                why: "not a numbered line, so it has no printed label to carry: the identity block, \
                      the filing-status radios and the dependents grid are resolved from column x, \
                      /MaxLen, row order and the caption ABOVE the widget. No single fact checks them, \
                      and a wrong one prints an SSN in a date box. Read the form."
                    .into(),
            });
            continue;
        };
        // ★ Sanity: the prior cell's own key must match the prior form's label, or the premise is gone.
        if let Some(printed_before) = old_labels.get(&c.fqn) {
            if printed_before != "?"
                && !crate::label_reader::label_matches(want_label, printed_before)
            {
                p.refused.push(Refused {
                    key: c.key.clone(),
                    candidate: String::new(),
                    why: format!(
                        "the PRIOR form prints {printed_before:?} beside {}, not {want_label:?}. The \
                         prior map disagrees with the prior form, so nothing can be carried from it.",
                        c.fqn
                    ),
                });
                continue;
            }
        }
        // ★ The same one-way tolerance on the NEW side: a key may carry a sub-letter the form omits.
        let hits: Vec<&String> = by_label_band
            .iter()
            .filter(|((label, band), _)| {
                *band == was.band && crate::label_reader::label_matches(want_label, label)
            })
            .flat_map(|(_, v)| v.iter().copied())
            .collect();
        match hits.as_slice() {
            [one] => {
                let now = &new_shapes[*one];
                if was.max_len.is_some() && was.max_len != now.max_len {
                    p.refused.push(Refused {
                        key: c.key.clone(),
                        candidate: (*one).clone(),
                        why: format!(
                            "/MaxLen {:?} → {:?}. The label and column agree and the CAPACITY does \
                             not; this is how a nine-digit SSN meets a two-character box.",
                            was.max_len, now.max_len
                        ),
                    });
                    continue;
                }
                if let Some(on) = &c.on {
                    if !now.on_states.contains(on) {
                        p.refused.push(Refused {
                            key: c.key.clone(),
                            candidate: (*one).clone(),
                            why: format!(
                                "on-state {on:?} is not declared by that widget, which offers {:?}. An \
                                 undeclared on-state renders as UNCHECKED while the write reports \
                                 success.",
                                now.on_states
                            ),
                        });
                        continue;
                    }
                }
                if was.page != now.page {
                    p.page_moves
                        .push(format!("{}: page {} → {}", c.key, was.page, now.page));
                }
                p.emitted.push(Cell {
                    key: c.key.clone(),
                    fqn: (*one).clone(),
                    on: c.on.clone(),
                });
            }
            many if !many.is_empty() => p.refused.push(Refused {
                key: c.key.clone(),
                candidate: String::new(),
                why: format!(
                    "{} widgets carry label {want_label:?} in the {} column: {many:?}. Ambiguous is \
                     not resolvable by rule.",
                    many.len(),
                    was.band
                ),
            }),
            _ => p.refused.push(Refused {
                key: c.key.clone(),
                candidate: String::new(),
                why: format!(
                    "the new revision prints no {want_label:?} in the {} column. Either the line was \
                     RENUMBERED — this revision spells AGI `11a` where the prior spells it `11`, and \
                     that cannot be inferred, only read — or it is gone.",
                    was.band
                ),
            }),
        }
    }
    p
}

/// Render a port as TOML plus a refusal report.
#[must_use]
pub fn render(stem: &str, prior_tag: &str, p: &Port) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# CANDIDATE map for {stem}, derived from {prior_tag} by `xtask port-map`.\n\
         # ★ REVIEW EVERY LINE against the form. This tool proposes; it never decides.\n\
         # {} cell(s) passed all five checks; {} refused; {} page move(s).\n",
        p.emitted.len(),
        p.refused.len(),
        p.page_moves.len()
    ));
    for m in &p.page_moves {
        s.push_str(&format!("# page move: {m}\n"));
    }
    for c in &p.emitted {
        match &c.on {
            Some(on) => s.push_str(&format!(
                "{} = {{ field = \"{}\", on = \"{on}\" }}\n",
                c.key, c.fqn
            )),
            None => s.push_str(&format!("{} = \"{}\"\n", c.key, c.fqn)),
        }
    }
    if !p.refused.is_empty() {
        s.push_str("\n# ── REFUSED — resolve each against the form, then add it by hand. ──\n");
        for r in &p.refused {
            s.push_str(&format!("# {} -> {}\n#   {}\n", r.key, r.candidate, r.why));
        }
    }
    s
}

/// `port-map <stem> <prior-tag> <new-tag>`.
pub fn run(stem: &str, prior_tag: &str, new_tag: &str) -> Result<(), String> {
    let root = crate::form_geometry::repo_root();
    let year_of = |tag: &str| tag.split('-').next().unwrap_or(tag).to_string();
    let map_path = root.join(format!(
        "crates/btctax-forms/forms/{}/{stem}.map.toml",
        year_of(prior_tag)
    ));
    let prior_text =
        std::fs::read_to_string(&map_path).map_err(|e| format!("{}: {e}", map_path.display()))?;
    let pdf = |tag: &str| {
        root.join(format!(
            "crates/btctax-forms/forms/{}/{stem}.pdf",
            year_of(tag)
        ))
    };
    let (old_sh, new_sh) = (shapes(&pdf(prior_tag))?, shapes(&pdf(new_tag))?);
    let old_labels = crate::label_reader::label_join(&format!("{stem}--{prior_tag}"))?;
    let new_labels = crate::label_reader::label_join(&format!("{stem}--{new_tag}"))?;
    let p = port(
        &cells_of(&prior_text),
        &old_sh,
        &new_sh,
        &old_labels,
        &new_labels,
    );
    print!("{}", render(stem, prior_tag, &p));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★★ **THE KNOWN ANSWER: the tool must agree with the one port done by hand.**
    ///
    /// f1040 TY2024 → TY2025 was ported cell by cell against the form on 2026-09-20/21. That makes it a
    /// known answer rather than a golden the tool could regenerate to match itself, which is the whole
    /// reason this test is possible and the reason it was worth doing the hand port first.
    ///
    /// The bar is **zero disagreements on the cells it emits** — not full coverage. A tool that emitted
    /// everything would have to guess at the four cells below, and guessing is what it exists not to do.
    #[test]
    fn the_f1040_ty2025_port_reproduces_the_hand_written_map() {
        let root = crate::form_geometry::repo_root();
        let read = |y: i32| {
            std::fs::read_to_string(
                root.join(format!("crates/btctax-forms/forms/{y}/f1040.map.toml")),
            )
            .expect("committed map")
        };
        let pdf = |y: i32| root.join(format!("crates/btctax-forms/forms/{y}/f1040.pdf"));
        let (old_sh, new_sh) = (shapes(&pdf(2024)).unwrap(), shapes(&pdf(2025)).unwrap());
        let ol = crate::label_reader::label_join("f1040--2024").unwrap();
        let nl = crate::label_reader::label_join("f1040--2025").unwrap();
        let p = port(&cells_of(&read(2024)), &old_sh, &new_sh, &ol, &nl);

        let hand: BTreeMap<String, String> = cells_of(&read(2025))
            .into_iter()
            .map(|c| (c.key, c.fqn))
            .collect();
        assert!(
            p.emitted.len() >= 35,
            "only {} cells emitted — if the tool has stopped resolving, this test is measuring almost \
             nothing",
            p.emitted.len()
        );
        let mut wrong = Vec::new();
        for c in &p.emitted {
            match hand.get(&c.key) {
                Some(h) if *h == c.fqn => {}
                Some(h) => wrong.push(format!("{}: tool {} vs hand {h}", c.key, c.fqn)),
                None => wrong.push(format!("{}: tool {} vs hand ABSENT", c.key, c.fqn)),
            }
        }
        assert!(
            wrong.is_empty(),
            "the tool disagrees with the hand-written TY2025 map on {} cell(s). One of the two is \
             wrong, and a filed return depends on which:\n  {}",
            wrong.len(),
            wrong.join("\n  ")
        );
        // ★ And it must NOT have invented the four a human had to read: three renumbered lines
        //   (11 → 11a, 12 → 12e, 13 → 13a) and one line TY2025 added outright (6d).
        for key in ["line11a", "line12e", "line13a", "line6d"] {
            assert!(
                !p.emitted.iter().any(|c| c.key == key),
                "{key} must be REFUSED — a renumbered or new line cannot be inferred from last year's \
                 map, and emitting it would be the tool deciding"
            );
        }
        // ★ The page move that broke the descent oracle is REPORTED, not swallowed.
        assert!(
            p.page_moves.iter().any(|m| m.starts_with("line14:")),
            "the page move for line14 must be reported: {:?}",
            p.page_moves
        );
    }

    fn shape(page: u32, band: &str, max_len: Option<usize>, on: &[&str]) -> Shape {
        Shape {
            page,
            band: band.to_string(),
            max_len,
            on_states: on.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// One prior cell, one candidate widget, and whatever shapes/labels the caller wants to plant.
    fn one(
        key: &str,
        on: Option<&str>,
        was: Shape,
        now: Shape,
        printed_before: &str,
        printed_after: &str,
    ) -> Port {
        let prior = vec![Cell {
            key: key.to_string(),
            fqn: "topmostSubform[0].Page1[0].old[0]".into(),
            on: on.map(str::to_string),
        }];
        let mut olds = BTreeMap::new();
        olds.insert("topmostSubform[0].Page1[0].old[0]".to_string(), was);
        let mut news = BTreeMap::new();
        news.insert("topmostSubform[0].Page1[0].new[0]".to_string(), now);
        let mut ol = BTreeMap::new();
        ol.insert(
            "topmostSubform[0].Page1[0].old[0]".to_string(),
            printed_before.to_string(),
        );
        let mut nl = BTreeMap::new();
        nl.insert(
            "topmostSubform[0].Page1[0].new[0]".to_string(),
            printed_after.to_string(),
        );
        port(&prior, &olds, &news, &ol, &nl)
    }

    /// ★★★ **B1 — each of the four traps the hand port actually hit, planted.**
    ///
    /// Every one is a real measurement from f1040 TY2024 → TY2025, not an invented case: the `/MaxLen`
    /// shrink is TY2024's `taxpayer_ssn` FQN meeting TY2025's two-character date box; the undeclared
    /// on-state is the filing-status radio renumbered `mfs 4 → 3`; the column move is a sub-line cell
    /// pointed at its own pair's amount widget; the renumbering is `11 → 11a`.
    #[test]
    fn every_trap_the_hand_port_hit_is_refused() {
        // (1) capacity: nine digits into a two-character box.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", Some(9), &[]),
            shape(1, "AMOUNT", Some(2), &[]),
            "1a",
            "1a",
        );
        assert!(p.emitted.is_empty(), "a /MaxLen shrink must be refused");
        assert!(
            p.refused[0].why.contains("/MaxLen"),
            "and named as capacity: {}",
            p.refused[0].why
        );

        // (2) an on-state the new widget does not declare — the filing-status renumbering.
        let p = one(
            "line1a",
            Some("4"),
            shape(1, "AMOUNT", None, &["4"]),
            shape(1, "AMOUNT", None, &["3"]),
            "1a",
            "1a",
        );
        assert!(
            p.emitted.is_empty(),
            "an undeclared on-state must be refused"
        );
        assert!(
            p.refused[0].why.contains("on-state"),
            "and named as such: {}",
            p.refused[0].why
        );

        // (3) the column moved — the trap the label check structurally cannot see.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "MID", None, &[]),
            "1a",
            "1a",
        );
        assert!(p.emitted.is_empty(), "a column move must be refused");

        // (4) the line was RENUMBERED: nothing carries label `11` on the new revision.
        let p = one(
            "line11",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "AMOUNT", None, &[]),
            "11",
            "11a",
        );
        assert!(p.emitted.is_empty(), "a renumbered line must be refused");
        assert!(
            p.refused[0].why.contains("RENUMBERED"),
            "and say so, because that is the one case a human must read: {}",
            p.refused[0].why
        );

        // …and the control: same label, same column, same capacity ⇒ EMITTED. Without this the four
        // above are satisfied by a tool that refuses everything.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", Some(9), &[]),
            shape(1, "AMOUNT", Some(9), &[]),
            "1a",
            "1a",
        );
        assert_eq!(
            p.emitted.len(),
            1,
            "an unchanged cell must carry: {:?}",
            p.refused
        );
        assert!(p.page_moves.is_empty());

        // A PAGE move carries, and is reported — the emitter derives descent groups from the page.
        let p = one(
            "line1a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(2, "AMOUNT", None, &[]),
            "1a",
            "1a",
        );
        assert_eq!(p.emitted.len(), 1, "a page move is not a refusal");
        assert_eq!(p.page_moves.len(), 1, "but it must be reported");
    }

    /// ★★★ **The one-way label tolerance is the SHARED predicate's, not a second opinion.**
    ///
    /// The first version of this tool compared labels strictly and refused TY2024's `line7a` against a
    /// printed `7` — a false positive against a convention `label_reader::label_matches` already
    /// encodes. This pins both directions so the two can never drift apart.
    #[test]
    fn the_label_tolerance_is_one_way_and_shared_with_the_gate() {
        // A key MAY carry a sub-letter the form omits: there is only one line 7 to mean.
        let p = one(
            "line7a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "AMOUNT", None, &[]),
            "7",
            "7a",
        );
        assert_eq!(
            p.emitted.len(),
            1,
            "line7a over a printed 7: {:?}",
            p.refused
        );
        // ★★ And the tolerance must apply on the NEW side too — the direction a strict comparison
        //    would break. A key `line7a` against a new form that prints plain `7` is the TY2025 → a
        //    hypothetical TY2026 case, and it is also what TY2024's own map needs today. Planting
        //    strictness had to red SOMETHING, and until this case existed it red nothing.
        let p = one(
            "line7a",
            None,
            shape(1, "AMOUNT", None, &[]),
            shape(1, "AMOUNT", None, &[]),
            "7a",
            "7",
        );
        assert_eq!(
            p.emitted.len(),
            1,
            "line7a must carry onto a form printing plain `7`: {:?}",
            p.refused
        );
        // It may NOT lack a letter the form HAS: 11a and 11b both exist, so `line11` says neither.
        assert!(crate::label_reader::label_matches("7a", "7"));
        assert!(!crate::label_reader::label_matches("11", "11a"));
    }

    /// ★★★ **AN AMBIGUOUS LABEL IS REFUSED, NEVER PICKED.**
    ///
    /// Two widgets carrying one label in one column is the 2a/2b shape — the form prints `2a` once over a
    /// pair — and picking the first would put a sub-line figure in the amount column of a signed return.
    /// Planting `[one] => …` as `[one, ..] => …` (take the first of many) red NOTHING until this test
    /// existed, which is the whole reason it does.
    #[test]
    fn two_widgets_with_one_label_in_one_column_are_refused() {
        let prior = vec![Cell {
            key: "line2a".into(),
            fqn: "topmostSubform[0].Page1[0].old[0]".into(),
            on: None,
        }];
        let mut olds = BTreeMap::new();
        olds.insert(
            "topmostSubform[0].Page1[0].old[0]".to_string(),
            shape(1, "AMOUNT", None, &[]),
        );
        let mut news = BTreeMap::new();
        let mut nl = BTreeMap::new();
        for n in ["a", "b"] {
            let fqn = format!("topmostSubform[0].Page1[0].new_{n}[0]");
            news.insert(fqn.clone(), shape(1, "AMOUNT", None, &[]));
            nl.insert(fqn, "2a".to_string());
        }
        let mut ol = BTreeMap::new();
        ol.insert(
            "topmostSubform[0].Page1[0].old[0]".to_string(),
            "2a".to_string(),
        );
        let p = port(&prior, &olds, &news, &ol, &nl);
        assert!(
            p.emitted.is_empty(),
            "two candidates must be refused, not picked from: {:?}",
            p.emitted
        );
        assert!(
            p.refused[0].why.contains("Ambiguous"),
            "and named as ambiguity: {}",
            p.refused[0].why
        );
    }
}
