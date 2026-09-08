//! ★★★ **T8 / R6 — THE TY2025+ DEPENDENTS GRID, DERIVED FROM THE FORM AND NEVER TYPED.**
//!
//! Form 1040 (2025) re-parted the Dependents block: TY2024 printed four dependent **rows** with four
//! columns; TY2025 prints four dependent **columns** with seven rows — *"(1) First name"*,
//! *"(2) Last name"*, *"(3) SSN"*, *"(4) Relationship"*, *"(5) Check if lived with you more than half
//! of 2025 — (a) Yes / (b) And in the U.S."*, *"(6) Check if — Full-time student / Permanently and
//! totally disabled"*, *"(7) Credits — Child tax credit / Credit for other dependents"*
//! (`design/forms/extract/f1040--2025.txt:38-54`).
//!
//! Forty AcroForm cells plus the *more than four dependents* box, none of whose names is guessable
//! (`…Table_Dependents[0].Row6[0].Dependent3[0].c1_25[0]`). **Typing them into the map is the defect
//! this module exists to prevent** — one transposed `c1_2x` prints *permanently and totally disabled*
//! where the filer said *full-time student*, and it is invisible on the emitted page, invisible to
//! both oracles, and invisible to the field census (which only asks whether a name is *accounted for*,
//! never whether it is in the *right slot*).
//!
//! ★★★ **Why this is not `label_reader::boxes_tsv`.** It was tried first, and it reports `?` — a
//! BOX-WITH-NO-LABEL — for **all forty-one**. `witness_text`'s label tokens are bare numerals
//! (`is_numeric_label`), and this grid numbers its rows `(1)`…`(7)` **in parentheses**. That is a
//! genuine finding about the shared reader, not a reason to type the names in: the grid's addressing
//! is derived here from the same two witnesses the label reader uses (the text layer's own tokens,
//! and AcroForm geometry y-flipped), with the parenthesised label column derived rather than pinned.
//! Widening `is_numeric_label` itself would re-key every other form's census on a token class it has
//! never seen, which is the wrong blast radius for one form's grid.
//!
//! ★ **What is DERIVED and what is TRANSCRIBED, stated plainly.** The *address* of every cell — which
//! AcroForm field sits at (row label, sub-row, dependent, position) — is derived from geometry alone.
//! The *meaning* of each slot — that the second position in row (6) is *"Permanently and totally
//! disabled"* — is a transcription, and [`SLOT_CAPTIONS`] is checked verbatim against the extract by
//! [`tests::every_slot_caption_is_the_forms_own_words`]. A derivation with no captions would be an
//! address book; a caption table with no derivation would be the typing this replaces.

use crate::form_geometry::{self, Box_, Geometry};
use std::collections::BTreeMap;

/// One cell of the grid, named for the row the form prints it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    /// Row (1).
    FirstName,
    /// Row (2).
    LastName,
    /// Row (3).
    Ssn,
    /// Row (4).
    Relationship,
    /// Row (5)(a).
    LivedWithYou,
    /// Row (5)(b).
    LivedWithYouInUs,
    /// Row (6), first position.
    FullTimeStudent,
    /// Row (6), second position.
    PermanentlyAndTotallyDisabled,
    /// Row (7), first position.
    ChildTaxCredit,
    /// Row (7), second position.
    CreditForOtherDependents,
}

impl Slot {
    /// The key this slot carries in `f1040.map.toml`'s `[[dependents_grid.columns]]` tables.
    pub fn map_key(self) -> &'static str {
        match self {
            Slot::FirstName => "first_name",
            Slot::LastName => "last_name",
            Slot::Ssn => "ssn",
            Slot::Relationship => "relationship",
            Slot::LivedWithYou => "lived_with_you",
            Slot::LivedWithYouInUs => "lived_with_you_in_us",
            Slot::FullTimeStudent => "full_time_student",
            Slot::PermanentlyAndTotallyDisabled => "permanently_and_totally_disabled",
            Slot::ChildTaxCredit => "child_tax_credit",
            Slot::CreditForOtherDependents => "credit_for_other_dependents",
        }
    }
}

/// **The slot table**: the printed row label, the sub-row index within that label (rows (5)(a) and
/// (5)(b) are two y-bands under one label; every other label has one), the position index within the
/// band (rows (6) and (7) print two boxes side by side), and the caption the form prints beside it.
///
/// ★ The caption is what [`tests::every_slot_caption_is_the_forms_own_words`] asserts against
///   `design/forms/extract/f1040--2025.txt`, so a slot that drifts from the form reds. The `(row,
///   band, position)` triple is what the derivation below *measures*.
pub const SLOT_CAPTIONS: &[(Slot, &str, usize, usize, &str, &str)] = &[
    (Slot::FirstName, "1", 0, 0, "First name", F1040),
    (Slot::LastName, "2", 0, 0, "Last name", F1040),
    (Slot::Ssn, "3", 0, 0, "SSN", F1040),
    (Slot::Relationship, "4", 0, 0, "Relationship", F1040),
    (Slot::LivedWithYou, "5", 0, 0, "Check if lived", F1040),
    (Slot::LivedWithYouInUs, "5", 1, 0, "And in the U.S.", F1040),
    // ★ Rows (6) and (7) are sourced from the INSTRUCTIONS, and the reason is mechanical: the form
    //   prints these two captions stacked over three lines in each of four side-by-side columns, so
    //   `pdftotext -layout` interleaves them (*"Full-time  Permanently  Full-time  Permanently…"*) and
    //   no complete phrase survives contiguously in the form's own text layer. The flowchart prints
    //   each phrase whole, and it is the document that decides the box.
    (
        Slot::FullTimeStudent,
        "6",
        0,
        0,
        "Full-time student",
        I1040GI,
    ),
    (
        Slot::PermanentlyAndTotallyDisabled,
        "6",
        0,
        1,
        "Permanently and totally disabled",
        I1040GI,
    ),
    (Slot::ChildTaxCredit, "7", 0, 0, "Child tax credit", I1040GI),
    (
        Slot::CreditForOtherDependents,
        "7",
        0,
        1,
        "Credit for other dependents",
        I1040GI,
    ),
];

const F1040: &str = "design/forms/extract/f1040--2025.txt";
const I1040GI: &str = "design/forms/extract/i1040gi--2025.txt";

/// The derived grid: the *more than four dependents* box, and one column per printed dependent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedGrid {
    /// *"If more than four dependents, see instructions and check here"* — the ONE box in the grid's
    /// y-range that sits in no dependent column.
    pub more_than_four: String,
    /// Four columns, left to right, each a complete slot → FQN map.
    pub columns: Vec<BTreeMap<Slot, String>>,
}

/// A word token that is a parenthesised row label — `(1)` … `(9)`.
fn parenthesised_number(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('(')?.strip_suffix(')')?;
    (inner.len() == 1 && inner.chars().all(|c| c.is_ascii_digit())).then_some(inner)
}

/// ★★★ **THE DERIVATION.** Pure over one [`Geometry`], so a planted defect can reach it (B1).
///
/// 1. **The dependent columns** are the `Dependent 1` … `Dependent 4` headings' x anchors, read off
///    the text layer. Four is measured, never assumed: fewer or more is an error, because the count
///    is what `DEPENDENTS_GRID_ROWS` has to agree with.
/// 2. **The row labels** are the parenthesised tokens `(1)`…`(7)` in the column that carries the most
///    of them — the same *"the label column is the one that accounts for the most printed lines"*
///    discriminator `label_reader` uses, and for the same reason: a hardcoded x is the `1..=38` trap
///    in another costume.
/// 3. **A box belongs to the row label** with the greatest y at or above the box's own vertical
///    centre (`label_reader::boxes_tsv`'s rule, verbatim), and **to the dependent column** whose
///    anchor is nearest — unless it is farther than half the column pitch, which is how the *more
///    than four dependents* box identifies itself rather than being named here.
/// 4. **Within a label**, distinct y-bands are the sub-rows in printed order and, within a band,
///    x order is the position. `(row, band, position)` then names the slot through [`SLOT_CAPTIONS`].
pub fn derive(g: &Geometry) -> Result<DerivedGrid, String> {
    // ── 1. The dependent columns. ────────────────────────────────────────────────────────────────
    let mut anchors: Vec<(f64, f64)> = Vec::new(); // (x, y) of each "Dependent" heading
    for (i, w) in g.words.iter().enumerate() {
        if w.page != 1 || w.text != "Dependent" {
            continue;
        }
        // The ordinal is the very next token on the same line.
        let ordinal = g.words.get(i + 1).filter(|n| {
            n.page == w.page && (n.y - w.y).abs() < 2.0 && n.text.parse::<u32>().is_ok()
        });
        if let Some(n) = ordinal {
            anchors.push((w.x, w.y));
            let _ = n;
        }
    }
    // Only the grid's own headings, which all sit on one printed line.
    if anchors.is_empty() {
        return Err("no `Dependent N` headings found on page 1".into());
    }
    let heading_y = anchors[0].1;
    anchors.retain(|(_, y)| (y - heading_y).abs() < 2.0);
    anchors.sort_by(|a, b| a.0.total_cmp(&b.0));
    if anchors.len() != 4 {
        return Err(format!(
            "the grid must print four dependent columns; measured {}",
            anchors.len()
        ));
    }
    let xs: Vec<f64> = anchors.iter().map(|(x, _)| *x).collect();
    let pitch = (xs[3] - xs[0]) / 3.0;

    // ── 2. The row-label column: the x-cluster carrying the most parenthesised numbers. ──────────
    let mut by_x: BTreeMap<i64, Vec<(String, f64)>> = BTreeMap::new();
    for w in &g.words {
        if w.page != 1 {
            continue;
        }
        if let Some(n) = parenthesised_number(&w.text) {
            by_x.entry(w.x.round() as i64)
                .or_default()
                .push((n.to_string(), w.y));
        }
    }
    let (_, labels) = by_x
        .into_iter()
        .max_by_key(|(_, v)| v.len())
        .ok_or("no parenthesised row labels on page 1")?;
    if labels.len() < 7 {
        return Err(format!(
            "the label column accounts for only {} rows; the grid prints seven",
            labels.len()
        ));
    }
    let first_label_y = labels.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let last_label_y = labels.iter().map(|(_, y)| *y).fold(0.0_f64, f64::max);

    // ── 3. Every box inside the grid's y-range, keyed by (label, band, column, x). ───────────────
    struct Cell<'a> {
        label: String,
        y: f64,
        x: f64,
        column: Option<usize>,
        b: &'a Box_,
    }
    let mut cells: Vec<Cell<'_>> = Vec::new();
    let mut more_than_four: Option<&Box_> = None;
    for b in &g.boxes {
        let Some((top, bottom)) = g.box_top_down_y(b) else {
            continue;
        };
        let centre_y = (top + bottom) / 2.0;
        // The grid's own vertical extent: from its first label to just past its last row of boxes.
        if b.page != 1 || centre_y < first_label_y - 4.0 || centre_y > last_label_y + 14.0 {
            continue;
        }
        let Some((label, _)) = labels
            .iter()
            .filter(|(_, ly)| *ly <= centre_y + 2.0)
            .max_by(|(_, a), (_, c)| a.total_cmp(c))
        else {
            return Err(format!("{}: no row label at or above it", b.name));
        };
        let centre_x = (b.x + b.x2) / 2.0;
        let (nearest, dist) = xs
            .iter()
            .enumerate()
            .map(|(i, x)| (i, (centre_x - x).abs()))
            .min_by(|a, c| a.1.total_cmp(&c.1))
            .expect("four anchors");
        let column = (dist <= pitch / 2.0).then_some(nearest);
        if column.is_none() {
            if more_than_four.replace(b).is_some() {
                return Err(format!(
                    "two boxes in the grid sit in no dependent column; the second is {}",
                    b.name
                ));
            }
            continue;
        }
        cells.push(Cell {
            label: label.clone(),
            y: centre_y,
            x: centre_x,
            column,
            b,
        });
    }
    let more_than_four = more_than_four
        .ok_or("no *more than four dependents* box: every grid box sat in a dependent column")?
        .name
        .clone();

    // ── 4. Bands within a label, then positions within a band. ───────────────────────────────────
    let mut bands: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for c in &cells {
        let key = (c.y * 2.0).round() as i64; // half-point buckets: distinct printed rows differ by ≥ 8pt
        let v = bands.entry(c.label.clone()).or_default();
        if !v.iter().any(|b| (b - key).abs() <= 8) {
            v.push(key);
        }
    }
    for v in bands.values_mut() {
        v.sort_unstable();
    }

    let mut columns: Vec<BTreeMap<Slot, String>> = vec![BTreeMap::new(); 4];
    for c in &cells {
        let key = (c.y * 2.0).round() as i64;
        let band = bands[&c.label]
            .iter()
            .position(|b| (b - key).abs() <= 8)
            .expect("every cell's band was recorded");
        let col = c.column.expect("uncolumned cells were taken out above");
        // Position within (label, band, column): x order among the cells sharing all three.
        let position = cells
            .iter()
            .filter(|o| {
                o.label == c.label
                    && o.column == c.column
                    && ((o.y * 2.0).round() as i64 - key).abs() <= 8
            })
            .filter(|o| o.x < c.x)
            .count();
        let slot = SLOT_CAPTIONS
            .iter()
            .find(|(_, l, bd, p, ..)| *l == c.label && *bd == band && *p == position)
            .map(|(s, ..)| *s)
            .ok_or_else(|| {
                format!(
                    "{}: measured row ({}) band {band} position {position}, which no slot claims",
                    c.b.name, c.label
                )
            })?;
        if let Some(prev) = columns[col].insert(slot, c.b.name.clone()) {
            return Err(format!(
                "dependent {} slot {slot:?} is claimed twice: {prev} and {}",
                col + 1,
                c.b.name
            ));
        }
    }
    for (i, col) in columns.iter().enumerate() {
        if col.len() != SLOT_CAPTIONS.len() {
            return Err(format!(
                "dependent {} has {} of {} slots filled: {:?}",
                i + 1,
                col.len(),
                SLOT_CAPTIONS.len(),
                col.keys().collect::<Vec<_>>()
            ));
        }
    }
    Ok(DerivedGrid {
        more_than_four,
        columns,
    })
}

/// The ON-STATE of one checkbox, **measured from the PDF** — never typed. Row (7)'s two positions are
/// two widgets of ONE field with on-states `/1` and `/2`, so a hand-typed `on = "1"` on the second
/// would silently never check.
pub fn on_state(stem: &str, fqn: &str) -> Result<String, String> {
    let (form, year) = split_stem(stem)?;
    let pdf = form_geometry::repo_root()
        .join("crates/btctax-forms/forms")
        .join(year)
        .join(format!("{form}.pdf"));
    let bytes = std::fs::read(&pdf).map_err(|e| format!("read {}: {e}", pdf.display()))?;
    let doc = btctax_forms::testonly::load(&bytes).map_err(|e| format!("{e}"))?;
    let fields = btctax_forms::testonly::collect_fields(&doc).map_err(|e| format!("{e}"))?;
    let f = fields
        .iter()
        .find(|f| f.fqn == fqn)
        .ok_or_else(|| format!("{fqn} is not an AcroForm field of {}", pdf.display()))?;
    let states = btctax_forms::testonly::button_on_states(&doc, f.id);
    match states.len() {
        1 => Ok(states[0].clone()),
        _ => Err(format!(
            "{fqn} declares {} on-states {states:?}; a checkbox widget must declare exactly one",
            states.len()
        )),
    }
}

/// `f1040--2025` → `("f1040", "2025")`.
fn split_stem(stem: &str) -> Result<(&str, &str), String> {
    stem.split_once("--")
        .ok_or_else(|| format!("{stem} is not a `<form>--<year>` stem"))
}

/// `cargo run -p xtask -- dependents-grid <stem>` — print the derived `[dependents_grid]` section, so
/// the committed map is GENERATED and the test below is what holds it there.
///
/// ★ Rows (1)–(4) are printed COMMENTED OUT. They are the identity block's cells, this build writes
///   no TY2025 identity block, and a mapped cell nothing writes is the *"blank because nothing
///   populated it"* defect with a map entry in front of it. They stay unaccounted on the field
///   census's `UNCENSUSED` register, which is the honest home for a cell with no decision yet.
pub fn run(stem: &str) -> Result<(), String> {
    let g = form_geometry::load(&form_geometry::repo_root(), stem)?;
    let grid = derive(&g)?;
    println!("[dependents_grid]");
    println!(
        "more_than_four_dependents = {{ field = \"{}\", on = \"{}\" }}",
        grid.more_than_four,
        on_state(stem, &grid.more_than_four)?
    );
    for (i, col) in grid.columns.iter().enumerate() {
        println!("\n# Dependent {}", i + 1);
        println!("[[dependents_grid.columns]]");
        for (slot, fqn) in col {
            match slot {
                // ★★★ NO QUOTES on the commented names, and it is load-bearing rather than
                //     cosmetic: `field_census.rs::map_and_census` text-scans every quoted span in the
                //     file — comments included — so a commented-out `"…[0]"` would be counted as
                //     MAPPED and would shrink the UNCENSUSED register by cells nothing writes. That
                //     is precisely the silent-omission defect the register exists to make loud.
                Slot::FirstName | Slot::LastName | Slot::Ssn | Slot::Relationship => {
                    println!(
                        "# (derived; NOT mapped — identity block) {} = {fqn}",
                        slot.map_key()
                    );
                }
                _ => println!(
                    "{} = {{ field = \"{fqn}\", on = \"{}\" }}",
                    slot.map_key(),
                    on_state(stem, fqn)?
                ),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cite_check::normalise;

    fn f1040_2025() -> Geometry {
        form_geometry::load(&form_geometry::repo_root(), "f1040--2025")
            .expect("the TY2025 Form 1040 geometry fixture")
    }

    /// ★★★ **THE GATE — the committed map's grid IS the derivation, cell for cell.**
    ///
    /// Every FQN in `forms/2025/f1040.map.toml`'s `[dependents_grid]` is compared against the one
    /// measured off the form. A transposed pair, a stale name after an IRS revision, or a hand-edit
    /// reds here — and nowhere else, because a wrong-but-existing FQN passes the field census, passes
    /// `map_pdf_conformance`, and prints a checked box in the wrong place.
    #[test]
    fn the_committed_ty2025_grid_map_is_the_one_measured_off_the_form() {
        let derived = derive(&f1040_2025()).expect("the grid derives");
        let map = btctax_forms::testonly::Form1040Map::for_year(2025).expect("the TY2025 1040 map");
        let grid = map
            .dependents_grid
            .as_ref()
            .expect("the TY2025 map declares [dependents_grid]");
        assert_eq!(
            grid.more_than_four_dependents.field, derived.more_than_four,
            "the *more than four dependents* box"
        );
        assert_eq!(grid.columns.len(), derived.columns.len(), "column count");
        for (i, (committed, measured)) in grid.columns.iter().zip(&derived.columns).enumerate() {
            let n = i + 1;
            for (slot, fqn) in measured {
                let got = match slot {
                    // ★ Rows (1)–(4) are DERIVED but deliberately not mapped — see `run`'s note.
                    //   Asserted absent below, so "not mapped" stays a decision rather than drift.
                    Slot::FirstName | Slot::LastName | Slot::Ssn | Slot::Relationship => continue,
                    Slot::LivedWithYou => &committed.lived_with_you,
                    Slot::LivedWithYouInUs => &committed.lived_with_you_in_us,
                    Slot::FullTimeStudent => &committed.full_time_student,
                    Slot::PermanentlyAndTotallyDisabled => {
                        &committed.permanently_and_totally_disabled
                    }
                    Slot::ChildTaxCredit => &committed.child_tax_credit,
                    Slot::CreditForOtherDependents => &committed.credit_for_other_dependents,
                };
                assert_eq!(&got.field, fqn, "dependent {n}, {slot:?}");
                // ★★ The ON-STATE is measured too. Row (7)'s two positions are two widgets of ONE
                //    field, on `/1` and `/2`; a copied `on = "1"` on the second would print nothing
                //    at all and look exactly like a filer with no credit.
                assert_eq!(
                    got.on,
                    on_state("f1040--2025", fqn).expect("the on-state reads"),
                    "dependent {n}, {slot:?}: on-state"
                );
            }
        }
    }

    /// ★★★ **B1 — the derivation watched going RED on the defect it exists to catch.**
    ///
    /// The plant is the one a human typing forty names actually makes: two boxes swapped inside one
    /// dependent's row (6). It is invisible on the emitted PDF (both are empty checkboxes), invisible
    /// to the field census (both names are accounted for), and it prints *permanently and totally
    /// disabled* for a filer who said *full-time student*.
    #[test]
    fn a_transposed_pair_reds() {
        let mut g = f1040_2025();
        let good = derive(&g).expect("the grid derives");
        // Swap the x of dependent 1's two row-(6) boxes — the geometry a transposition would need.
        let a = good.columns[0][&Slot::FullTimeStudent].clone();
        let b = good.columns[0][&Slot::PermanentlyAndTotallyDisabled].clone();
        let (mut ax, mut ax2) = (0.0, 0.0);
        for bx in &g.boxes {
            if bx.name == a {
                ax = bx.x;
                ax2 = bx.x2;
            }
        }
        let (mut bx_, mut bx2) = (0.0, 0.0);
        for bx in &g.boxes {
            if bx.name == b {
                bx_ = bx.x;
                bx2 = bx.x2;
            }
        }
        for bx in &mut g.boxes {
            if bx.name == a {
                bx.x = bx_;
                bx.x2 = bx2;
            } else if bx.name == b {
                bx.x = ax;
                bx.x2 = ax2;
            }
        }
        let swapped = derive(&g).expect("the swapped geometry still derives");
        assert_ne!(
            swapped.columns[0][&Slot::FullTimeStudent],
            good.columns[0][&Slot::FullTimeStudent],
            "a transposed pair must change the derivation — otherwise the gate above is blind"
        );
        assert_eq!(
            swapped.columns[0][&Slot::FullTimeStudent],
            b,
            "and it must change it to the OTHER box, not to nothing"
        );
    }

    /// ★★★ **B1, the second class — a DROPPED row.** Deleting one dependent's row-(7) boxes must be
    /// an error, not a shorter map: a missing slot is the *"we forgot this line"* defect, and a
    /// derivation that silently returns nine slots would let the map lose a credit box.
    #[test]
    fn a_missing_cell_reds() {
        let mut g = f1040_2025();
        let victim = derive(&g).expect("derives").columns[2][&Slot::ChildTaxCredit].clone();
        g.boxes.retain(|b| b.name != victim);
        let e = derive(&g).expect_err("a grid missing a cell must not derive");
        assert!(
            e.contains("has 9 of 10 slots filled"),
            "the error must name the incomplete column: {e}"
        );
    }

    /// ★ **The captions are the FORM'S OWN WORDS**, checked verbatim against the extract — the half
    /// the geometric derivation cannot do. Without it the slot table would be ten plausible names
    /// nobody checked, which is exactly the shape `prompt_check` exists to refuse elsewhere.
    #[test]
    fn every_slot_caption_is_the_forms_own_words() {
        let mut hay: BTreeMap<&str, String> = BTreeMap::new();
        for (slot, .., caption, extract) in SLOT_CAPTIONS {
            let hay = hay
                .entry(*extract)
                .or_insert_with(|| {
                    let path = form_geometry::repo_root().join(extract);
                    normalise(
                        &std::fs::read_to_string(&path)
                            .unwrap_or_else(|e| panic!("cannot read {extract}: {e}")),
                    )
                })
                .clone();
            let needle = normalise(caption);
            assert!(
                hay.contains(&needle),
                "{slot:?}'s caption is not in {extract}: {caption:?}"
            );
        }
    }
}
