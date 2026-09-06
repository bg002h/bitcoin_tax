//! **`xtask form-delta <old-stem> <new-stem>` — exactly what changed between two revisions of a form.**
//!
//! ★★★ **This exists to make draft→final a DIFF rather than a rebuild.** The TY2026 drafts are
//! archived now; the finals land Nov 2026 – Jan 2027. Everything built against a draft — field maps,
//! line numbering, struct shapes — has to be re-verified when the final arrives, and the difference
//! between "re-verify" and "redo" is whether the work list is computed or remembered.
//!
//! ★★ It reports the TWO axes independently, because they fail independently and the second is the
//! one that is normally missed:
//!
//! * **field-name churn** — names added, removed, or renamed. Loud, and a name-existence check
//!   already catches it.
//! * **line→label drift** — a field whose NAME is unchanged but which now sits beside a different
//!   printed line number. Measured on the real artifacts: TY2025's Form 6251 renamed **zero** page-1
//!   fields and still moved **12 of 41** mapped lines, because one added field walked everything
//!   below it down. An existence check passes on that with 0 of 61 names absent, and the AMT prints
//!   in line 10's box.
//!
//! Calibrated on pairs where BOTH sides are real: `f6251--2024` → `f6251--2025` (0 renames, 12 label
//! moves) and `f8995--2024` → `f8995--2025` (18 renames, 0 label moves). Those two are opposite
//! shapes, which is why both are pinned in the tests.
//!
//! ★★★ **AND THE VERDICT REPORTS ITS OWN EVIDENCE.** The label axis used to silently drop every
//! field it could not read — no box in the geometry, or a box no printed label claims (`?`) — and
//! then print *"no field changed the printed line it sits beside"* over whatever was left, including
//! nothing. That is the cleanest possible verdict from zero comparisons, and it is the product's
//! organising failure in miniature: the FORM fails closed, the INSTRUMENT fails open.
//!
//! Measured 2026-09-05 over the 13 archived `--2025` → `--2026-DRAFT` pairs: **604 of 664 common
//! fields were actually compared**, so 60 fields were being folded into verdicts that said nothing
//! about them. The extremes:
//!
//! | pair | common | actually compared | what it printed then | what it prints now |
//! |---|---|---|---|---|
//! | `f8949--2025` → `f8949--2026-DRAFT` | 202 | **186** | "no field changed" | 186 compared, none moved, **16 named as unwitnessed** |
//! | `f1040sc--2025` → `f1040sc--2026-DRAFT` | 59 | **39** | "8 fields moved" | 8 moved **of 39 compared**, 20 named |
//! | `f8960--2025` → `f8960--2026-DRAFT` | 38 | **33** | "no field changed" | 33 compared, none moved, **5 named** |
//! | `f6251--2024` → `f6251--2025` | 61 | **59** | "30 fields moved" | 30 moved of 59 compared, 2 named |
//!
//! So a "no change" claim now always carries the count it rests on, an unwitnessed field is named
//! with the reason it could not be read, and **zero comparisons is its own verdict**
//! ([`LabelVerdict::Unwitnessed`]) that exits non-zero — never a clean one.
//!
//! ★ The counts above are what the tool prints today; re-run it rather than trusting this table,
//! which is a dated measurement and not the authority. `f1040` is absent from it because
//! `design/forms/2026/f1040--2026-DRAFT.pdf` was withdrawn the same day (it was the TY2025 form).

use std::collections::{BTreeMap, BTreeSet};

/// Where a form's PDF lives: the bundled template if there is one, else the archived authority copy.
fn pdf_for(stem: &str) -> Option<std::path::PathBuf> {
    let root = crate::form_geometry::repo_root();
    let (form, year) = stem.split_once("--")?;
    let year_dir = year.trim_end_matches("-DRAFT");
    let bundled = root.join(format!("crates/btctax-forms/forms/{year_dir}/{form}.pdf"));
    if bundled.is_file() {
        return Some(bundled);
    }
    let archived = root.join(format!("design/forms/{year_dir}/{stem}.pdf"));
    archived.is_file().then_some(archived)
}

fn field_set(stem: &str) -> Result<BTreeSet<String>, String> {
    let pdf = pdf_for(stem).ok_or_else(|| format!("no PDF found for {stem}"))?;
    let bytes = std::fs::read(&pdf).map_err(|e| format!("{}: {e}", pdf.display()))?;
    let doc = btctax_forms::testonly::load(&bytes).map_err(|e| format!("{stem}: {e:?}"))?;
    Ok(btctax_forms::testonly::collect_fields(&doc)
        .map_err(|e| format!("{stem}: {e:?}"))?
        .into_iter()
        .map(|f| f.fqn)
        .collect())
}

/// The label `label_reader` gives a box that **no printed line label claims** — its
/// BOX-WITH-NO-LABEL marker (`label_reader::boxes_tsv`: *"A box no label claims emits `?` — that is
/// BOX-WITH-NO-LABEL, and it is a hard finding, not a blank cell."*).
///
/// It is a finding, never a value, so it can never be compared against another one. What it must
/// never do is vanish: `"?" == "?"` is not "this line did not move".
const NO_LABEL: &str = "?";

/// Why one common field contributed **no evidence** to the line→label axis.
///
/// ★★ Each of these used to be an `if let` that fell through to nothing, which made a field the tool
/// *could not read* indistinguishable from a field it read and found unmoved. A checker that cannot
/// tell "this box encodes no printed line" from "we never looked at this box" is not a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unwitnessed {
    /// One side has no geometry fixture at all, so the axis never ran for any field.
    GeometryFixtureMissing,
    /// Both sides have geometry, and neither records a box by this name.
    BoxAbsentBothSides,
    /// The new side records a box by this name; the old side's geometry does not.
    BoxAbsentOld,
    /// The old side records a box by this name; the new side's geometry does not.
    BoxAbsentNew,
    /// The box exists on both sides and no printed line label claims it on either — the header
    /// name/SSN boxes, checkboxes, and address rows are the honest members of this class.
    UnlabelledBothSides,
    /// The new side's box carries a printed label; the old side's is BOX-WITH-NO-LABEL.
    UnlabelledOld,
    /// The old side's box carries a printed label; the new side's is BOX-WITH-NO-LABEL.
    UnlabelledNew,
}

impl Unwitnessed {
    /// A sentence a person can act on. Printed beside the field name, never in place of it.
    pub fn why(self) -> &'static str {
        match self {
            Unwitnessed::GeometryFixtureMissing => {
                "one side has NO geometry fixture — run `xtask extract-geometry <stem>`"
            }
            Unwitnessed::BoxAbsentBothSides => "neither side's geometry records a box by this name",
            Unwitnessed::BoxAbsentOld => "the OLD side's geometry records no box by this name",
            Unwitnessed::BoxAbsentNew => "the NEW side's geometry records no box by this name",
            Unwitnessed::UnlabelledBothSides => {
                "BOX-WITH-NO-LABEL on both sides — no printed line claims this box"
            }
            Unwitnessed::UnlabelledOld => "BOX-WITH-NO-LABEL on the OLD side",
            Unwitnessed::UnlabelledNew => "BOX-WITH-NO-LABEL on the NEW side",
        }
    }
}

/// Can this one field's two labels be compared at all? `Ok` is evidence; `Err` says why there is
/// none. There is deliberately no third answer, so a caller cannot drop a field by forgetting it.
fn witness(old: Option<&str>, new: Option<&str>) -> Result<(String, String), Unwitnessed> {
    let (o, n) = match (old, new) {
        (None, None) => return Err(Unwitnessed::BoxAbsentBothSides),
        (None, Some(_)) => return Err(Unwitnessed::BoxAbsentOld),
        (Some(_), None) => return Err(Unwitnessed::BoxAbsentNew),
        (Some(o), Some(n)) => (o, n),
    };
    match (o == NO_LABEL, n == NO_LABEL) {
        (true, true) => Err(Unwitnessed::UnlabelledBothSides),
        (true, false) => Err(Unwitnessed::UnlabelledOld),
        (false, true) => Err(Unwitnessed::UnlabelledNew),
        (false, false) => Ok((o.to_string(), n.to_string())),
    }
}

/// The line→label axis, with its own evidence attached.
///
/// The invariant that makes it a check rather than a summary: **`compared + unwitnessed.len()`
/// equals the number of common fields, always.** Every field is either evidence or a named gap.
pub struct LabelAxis {
    /// How many common fields yielded a printed label on BOTH sides. The verdict rests on exactly
    /// this many observations, and **zero is not clean**.
    pub compared: usize,
    /// Of those compared, the ones whose label changed: name → (old label, new label).
    pub moved: BTreeMap<String, (String, String)>,
    /// Every common field that yielded no comparison, by name, with the reason.
    pub unwitnessed: BTreeMap<String, Unwitnessed>,
}

/// Compare two label maps over a field set. Pure — no filesystem — so the zero-evidence case can be
/// tested without waiting for a form to develop one.
pub fn label_axis(
    common: &BTreeSet<String>,
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> LabelAxis {
    let mut axis = LabelAxis {
        compared: 0,
        moved: BTreeMap::new(),
        unwitnessed: BTreeMap::new(),
    };
    for f in common {
        match witness(
            old.get(f).map(String::as_str),
            new.get(f).map(String::as_str),
        ) {
            Ok((o, n)) => {
                axis.compared += 1;
                if o != n {
                    axis.moved.insert(f.clone(), (o, n));
                }
            }
            Err(u) => {
                axis.unwitnessed.insert(f.clone(), u);
            }
        }
    }
    axis
}

/// What the line→label axis is entitled to claim, given what it actually managed to read.
///
/// ★★★ The two evidence-free outcomes are **separate variants**, not an empty `moved` map, because
/// an empty map is what "nothing moved" and "nothing was looked at" both used to look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelVerdict {
    /// The axis never ran: one side has no geometry fixture.
    NoFixture { unwitnessed: usize },
    /// The axis ran and resolved a printed label on BOTH sides for **zero** common fields.
    Unwitnessed { unwitnessed: usize },
    /// `compared` bindings were actually compared and none of them moved.
    Unchanged { compared: usize, unwitnessed: usize },
    /// `compared` bindings were actually compared and `moved` of them changed line.
    Moved {
        compared: usize,
        moved: usize,
        unwitnessed: usize,
    },
}

impl LabelVerdict {
    /// How many field bindings this verdict rests on. **A "no change" claim with zero here is the
    /// absence of evidence, not evidence of absence.**
    pub fn compared(self) -> usize {
        match self {
            LabelVerdict::NoFixture { .. } | LabelVerdict::Unwitnessed { .. } => 0,
            LabelVerdict::Unchanged { compared, .. } | LabelVerdict::Moved { compared, .. } => {
                compared
            }
        }
    }

    /// How many common fields this verdict could not read, and therefore says nothing about.
    pub fn unwitnessed(self) -> usize {
        match self {
            LabelVerdict::NoFixture { unwitnessed }
            | LabelVerdict::Unwitnessed { unwitnessed }
            | LabelVerdict::Unchanged { unwitnessed, .. }
            | LabelVerdict::Moved { unwitnessed, .. } => unwitnessed,
        }
    }

    /// Whether this run is entitled to be read as a checked one. False for both evidence-free
    /// verdicts, and it is what `run` turns into a non-zero exit.
    pub fn is_witnessed(self) -> bool {
        self.compared() > 0
    }
}

/// The delta, as data. `None` for a label map means that side has no geometry fixture — reported,
/// never silently treated as "no change".
pub struct Delta {
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
    pub common: BTreeSet<String>,
    /// Fields present in BOTH whose printed line label moved: name -> (old label, new label).
    pub label_moved: BTreeMap<String, (String, String)>,
    pub labels_available: bool,
    /// How many common fields yielded a printed label on BOTH sides — the evidence
    /// [`Delta::label_moved`] being empty is or is not entitled to rest on.
    pub label_compared: usize,
    /// Every common field that yielded none, by name, with the reason it could not be read.
    pub label_unwitnessed: BTreeMap<String, Unwitnessed>,
}

impl Delta {
    /// The label axis's verdict, with its evidence count welded to it.
    pub fn label_verdict(&self) -> LabelVerdict {
        let unwitnessed = self.label_unwitnessed.len();
        if !self.labels_available {
            return LabelVerdict::NoFixture { unwitnessed };
        }
        if self.label_compared == 0 {
            return LabelVerdict::Unwitnessed { unwitnessed };
        }
        if self.label_moved.is_empty() {
            LabelVerdict::Unchanged {
                compared: self.label_compared,
                unwitnessed,
            }
        } else {
            LabelVerdict::Moved {
                compared: self.label_compared,
                moved: self.label_moved.len(),
                unwitnessed,
            }
        }
    }
}

pub fn compute(old: &str, new: &str) -> Result<Delta, String> {
    let a = field_set(old)?;
    let b = field_set(new)?;
    let common: BTreeSet<String> = a.intersection(&b).cloned().collect();

    let (la, lb) = (
        crate::label_reader::label_join_public(old).ok(),
        crate::label_reader::label_join_public(new).ok(),
    );
    let labels_available = la.is_some() && lb.is_some();
    let axis = match (la, lb) {
        (Some(la), Some(lb)) => label_axis(&common, &la, &lb),
        // The axis did not run. Every common field is still ACCOUNTED FOR — as a named gap, so the
        // `compared + unwitnessed == common` invariant holds here too and nothing is dropped.
        _ => LabelAxis {
            compared: 0,
            moved: BTreeMap::new(),
            unwitnessed: common
                .iter()
                .map(|f| (f.clone(), Unwitnessed::GeometryFixtureMissing))
                .collect(),
        },
    };
    Ok(Delta {
        added: b.difference(&a).cloned().collect(),
        removed: a.difference(&b).cloned().collect(),
        common,
        label_moved: axis.moved,
        labels_available,
        label_compared: axis.compared,
        label_unwitnessed: axis.unwitnessed,
    })
}

/// Print a capped list without hiding the cap — a truncated list that does not say it was truncated
/// is the same defect as a verdict that does not say it was unwitnessed.
fn print_capped(items: impl ExactSizeIterator<Item = String>, cap: usize, prefix: &str) {
    let n = items.len();
    for s in items.take(cap) {
        println!("    {prefix}{s}");
    }
    if n > cap {
        println!("    {prefix}… and {} more (not shown)", n - cap);
    }
}

fn print_unwitnessed(d: &Delta) {
    if d.label_unwitnessed.is_empty() {
        return;
    }
    let mut by_reason: BTreeMap<Unwitnessed, Vec<&str>> = BTreeMap::new();
    for (f, u) in &d.label_unwitnessed {
        by_reason.entry(*u).or_default().push(f.as_str());
    }
    println!(
        "  ★ {} of {} common field(s) yielded NO comparable label. The verdict above says nothing \
         about these, and they are listed rather than counted because a skipped field is not a \
         passed one:",
        d.label_unwitnessed.len(),
        d.common.len()
    );
    for (u, fs) in &by_reason {
        println!("    {} — {}:", fs.len(), u.why());
        // `GeometryFixtureMissing` is the one reason that is not about the field: it is identical for
        // every common field and the actionable item is the missing fixture, already named in the
        // banner. Every other reason is per-field and every field is printed.
        if *u == Unwitnessed::GeometryFixtureMissing {
            continue;
        }
        for f in fs {
            println!("      {f}");
        }
    }
}

pub fn run(old: &str, new: &str) -> Result<(), String> {
    let d = compute(old, new)?;
    println!("form-delta {old} -> {new}");
    println!(
        "  fields: {} common, {} added, {} removed",
        d.common.len(),
        d.added.len(),
        d.removed.len()
    );
    print_capped(d.added.iter().cloned(), 8, "+ ");
    print_capped(d.removed.iter().cloned(), 8, "- ");

    let v = d.label_verdict();
    match v {
        LabelVerdict::NoFixture { .. } => println!(
            "  ★ LINE->LABEL DRIFT NOT CHECKED — one side has no geometry fixture. Generate with \
             `xtask extract-geometry <stem>`. This is the axis a name check cannot see, so an \
             unchecked run is NOT a clean one."
        ),
        LabelVerdict::Unwitnessed { .. } => println!(
            "  ★★ LINE->LABEL DRIFT UNWITNESSED — 0 of {} common field(s) yielded a printed label \
             on BOTH sides. Nothing was compared, so \"no change\" here would be the absence of \
             evidence, not evidence of absence.",
            d.common.len()
        ),
        LabelVerdict::Unchanged { compared, .. } => println!(
            "  line->label: {compared} of {} common field(s) COMPARED; none of them changed the \
             printed line it sits beside",
            d.common.len()
        ),
        LabelVerdict::Moved {
            compared, moved, ..
        } => {
            println!(
                "  ★★ {moved} of {compared} COMPARED field(s) ({} common) KEPT THEIR NAME but now \
                 sit beside a different printed line — a map carried forward unchanged would write \
                 to the wrong line of a signed return:",
                d.common.len()
            );
            print_capped(
                d.label_moved
                    .iter()
                    .map(|(f, (x, y))| format!("{f}: line {x} -> {y}"))
                    .collect::<Vec<_>>()
                    .into_iter(),
                20,
                "",
            );
        }
    }
    print_unwitnessed(&d);

    // ★★ The exit status is DERIVED from the verdict rather than decided arm by arm, so neither
    // evidence-free verdict can exit 0 by someone forgetting a `return` in a fifth branch.
    if v.is_witnessed() {
        Ok(())
    } else {
        Err(format!(
            "{old} -> {new}: the line->label axis compared {} of {} common field(s) ({} \
             unwitnessed); this run is NOT evidence that no line moved",
            v.compared(),
            d.common.len(),
            v.unwitnessed()
        ))
    }
}

#[cfg(test)]
mod tests {
    /// ★ B1 for the label-join fold review L3: `design/TY2026_WORK_LIST.md` is this tool's OUTPUT,
    /// and it decayed silently when the reader behind the "lines that moved" column changed (four
    /// cells, including the only claim that Form 6251 moves a line). Every numeric row of the
    /// committed table is recomputed here at HEAD; every excused row ("Not listed") must be excused
    /// for a REAL reason (no pair at HEAD); and every stem with a map in any bundled year must
    /// appear in one table or the other — the doc's own "wrong denominator" warning, made a check.
    #[test]
    fn the_committed_work_list_matches_form_delta_at_head() {
        let doc = std::fs::read_to_string(
            crate::form_geometry::repo_root().join("design/TY2026_WORK_LIST.md"),
        )
        .unwrap();
        let mut compared: Vec<String> = Vec::new();
        let mut excused: Vec<String> = Vec::new();
        let mut wrong: Vec<String> = Vec::new();
        for l in doc.lines() {
            let Some((form, cells)) = parse_work_list_row(l) else {
                continue;
            };
            let pair = super::compute(&format!("{form}--2025"), &format!("{form}--2026-DRAFT"));
            match (cells, pair) {
                (Some(cells), Ok(d)) => {
                    compared.push(form.clone());
                    let got = (
                        d.common.len(),
                        d.added.len(),
                        d.removed.len(),
                        d.label_moved.len(),
                    );
                    if got != cells {
                        wrong.push(format!(
                            "{form}: table says {cells:?}, form-delta at HEAD says {got:?}"
                        ));
                    }
                }
                (Some(_), Err(e)) => {
                    wrong.push(format!("{form}: a numeric row, but no pair at HEAD ({e})"))
                }
                (None, Err(_)) => excused.push(form.clone()),
                (None, Ok(_)) => wrong.push(format!(
                    "{form}: excused as having no pair, but form-delta computes one"
                )),
            }
        }
        let mut stems: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let forms = crate::form_geometry::repo_root().join("crates/btctax-forms/forms");
        for year in std::fs::read_dir(&forms).unwrap().flatten() {
            let Ok(maps) = std::fs::read_dir(year.path()) else {
                continue;
            };
            for m in maps.flatten() {
                let p = m.path();
                if !p.to_string_lossy().ends_with(".map.toml") {
                    continue;
                }
                let text = std::fs::read_to_string(&p).unwrap();
                if let Some(v) = text.lines().find_map(|l| l.trim().strip_prefix("irs_stem")) {
                    stems.insert(
                        v.trim()
                            .trim_start_matches('=')
                            .trim()
                            .trim_matches('"')
                            .to_string(),
                    );
                }
            }
        }
        let listed: std::collections::BTreeSet<String> =
            compared.iter().chain(&excused).cloned().collect();
        let missing: Vec<&String> = stems.iter().filter(|s| !listed.contains(*s)).collect();
        assert!(
            stems.len() >= 18,
            "expected every emitted stem's irs_stem across the bundled maps, found {stems:?}"
        );
        assert!(
            missing.is_empty(),
            "stems with a map in a bundled year but NO row in either work-list table: {missing:?}"
        );
        eprintln!(
            "work list: {} compared, {} excused, {} stems on the emitting surface",
            compared.len(),
            excused.len(),
            stems.len()
        );
        assert!(
            wrong.is_empty(),
            "work list rows that no longer match the tool:\n  {}",
            wrong.join("\n  ")
        );
        // the plant: a cell off by one is a disagreement, so the comparison above can fail
        let (f, c) = parse_work_list_row("| `f6251` | 62 | 0 | 0 | 1 | port |").unwrap();
        let d = super::compute("f6251--2025", "f6251--2026-DRAFT").unwrap();
        assert_eq!(f, "f6251");
        assert_ne!(
            Some((
                d.common.len(),
                d.added.len(),
                d.removed.len(),
                d.label_moved.len()
            )),
            c
        );
    }

    /// A row of either work-list table: `| \`form\` | common | added | removed | moved | shape |`
    /// → `(form, Some(cells))`; a "Not listed" row (`| \`form\` | emitted? | … |`) → `(form, None)`.
    #[allow(clippy::type_complexity)]
    fn parse_work_list_row(l: &str) -> Option<(String, Option<(usize, usize, usize, usize)>)> {
        let cells: Vec<&str> = l.split('|').map(str::trim).collect();
        if cells.len() < 6 || !cells[1].starts_with('`') {
            return None;
        }
        let form = cells[1].trim_matches('`').to_string();
        let n = |i: usize| cells.get(i).and_then(|c| c.parse::<usize>().ok());
        let numeric = match (n(2), n(3), n(4), n(5)) {
            (Some(a), Some(b), Some(c), Some(d)) => Some((a, b, c, d)),
            _ => None,
        };
        Some((form, numeric))
    }

    use super::*;

    /// ★★★ **Calibrated on the pair that motivated the whole tool.**
    ///
    /// TY2025's Form 6251 split line 1 into 1a/1b: **one field added, ZERO renamed**, and every
    /// field from 2b down then sat beside a different printed line. A name-existence check passes
    /// on this with 0 of 61 names absent, and TY2024's `line11` — the AMT itself — lands in the
    /// TY2025 form's line-10 box.
    ///
    /// If this ever reports no label movement, `form-delta`'s second axis has gone blind and
    /// draft→final stops being a diff.
    #[test]
    fn form_delta_sees_the_6251_renumber_that_renamed_nothing() {
        let d = compute("f6251--2024", "f6251--2025").expect("both revisions are archived");
        assert_eq!(d.added.len(), 1, "TY2025 added exactly one page-1 field");
        assert_eq!(d.removed.len(), 0, "and renamed NOTHING — that is the trap");
        assert!(
            d.labels_available,
            "both sides need a geometry fixture, or the label axis is silently unchecked"
        );
        assert!(
            d.label_moved.len() >= 12,
            "the added field walks every line below it down; measured 30 of 61 fields moved, got {}",
            d.label_moved.len()
        );
        // The specific cascade, spot-checked rather than merely counted.
        let f = "topmostSubform[0].Page1[0].f1_10[0]";
        assert_eq!(
            d.label_moved.get(f).map(|(a, b)| (a.as_str(), b.as_str())),
            Some(("2g", "2f")),
            "the 2b-onward cascade must be visible field by field, not just as a count"
        );
    }

    /// ★★ **The OPPOSITE shape, pinned because a tool calibrated on one failure mode is calibrated
    /// on none.** TY2025's Form 8995 renamed 18 of 33 fields while the printed form did not change
    /// at all — loud on the name axis, silent on the label axis. Together with the 6251 case these
    /// two bracket the space: rename-without-renumber, and renumber-without-rename.
    #[test]
    fn form_delta_sees_the_8995_rename_that_moved_no_lines() {
        let d = compute("f8995--2024", "f8995--2025").expect("both revisions are archived");
        assert_eq!(d.added.len(), 18, "18 new spellings");
        assert_eq!(d.removed.len(), 18, "…for 18 retired ones — a pure rename");
        assert_eq!(d.common.len(), 15, "15 spellings survived");
        assert!(d.labels_available, "both sides need geometry");
        assert!(
            d.label_moved.is_empty(),
            "the printed form did not change, so no surviving field may have moved a line: {:?}",
            d.label_moved
        );
    }

    /// ★ The TY2026 drafts are archived and geometry-extracted, so the tool is ready for the finals.
    /// This asserts the pipeline is wired end to end TODAY rather than discovering in January that
    /// a fixture is missing.
    #[test]
    fn the_ty2026_drafts_are_ready_to_be_diffed_against_their_finals() {
        let d = compute("f6251--2025", "f6251--2026-DRAFT")
            .expect("the TY2026 draft is archived and its geometry extracted");
        assert!(
            d.labels_available,
            "the TY2026 draft must have a geometry fixture, or the day the final lands the label \
             axis is unchecked and the port is guesswork"
        );
        assert!(
            !d.common.is_empty(),
            "the two revisions must share fields, or one side failed to load"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The evidence axis. These are pure — no fixture, no PDF — so the zero-comparison case can be
    // exercised on demand instead of waiting for a real form to develop one.
    // ---------------------------------------------------------------------------------------

    fn m(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    fn s(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    fn delta_from(
        common: BTreeSet<String>,
        old: &BTreeMap<String, String>,
        new: &BTreeMap<String, String>,
    ) -> Delta {
        let axis = label_axis(&common, old, new);
        Delta {
            added: BTreeSet::new(),
            removed: BTreeSet::new(),
            common,
            label_moved: axis.moved,
            labels_available: true,
            label_compared: axis.compared,
            label_unwitnessed: axis.unwitnessed,
        }
    }

    /// ★★★ **THE PLANTED DEFECT THIS FILE EXISTS TO KEEP DEAD: a clean verdict from zero
    /// comparisons.**
    ///
    /// Every common field here has a box on both sides and no printed label claims any of them —
    /// `?`, `label_reader`'s BOX-WITH-NO-LABEL. The old code skipped each pair silently, found
    /// `label_moved` empty, and printed *"no field changed the printed line it sits beside"*. That
    /// is the cleanest possible verdict from no evidence at all, and it is what
    /// `form-delta f8959--2025 f8959--2026-DRAFT` was reporting.
    ///
    /// Restore either half of the old behaviour — let `witness` compare two `?`s, or let
    /// `label_verdict` fall through to `Unchanged` when `label_compared == 0` — and this reds.
    #[test]
    fn zero_comparisons_is_unwitnessed_and_never_a_clean_verdict() {
        let common = s(&["f1_1[0]", "f1_2[0]", "f1_3[0]"]);
        let old = m(&[("f1_1[0]", "?"), ("f1_2[0]", "?"), ("f1_3[0]", "?")]);
        let new = m(&[("f1_1[0]", "?"), ("f1_2[0]", "?"), ("f1_3[0]", "?")]);
        let d = delta_from(common, &old, &new);

        assert_eq!(
            d.label_verdict(),
            LabelVerdict::Unwitnessed { unwitnessed: 3 },
            "0 comparisons must be its OWN verdict — an empty `label_moved` is what \"nothing \
             moved\" and \"nothing was looked at\" both look like"
        );
        assert!(
            !d.label_verdict().is_witnessed(),
            "an evidence-free run must not be readable as a checked one"
        );
        assert_eq!(
            d.label_verdict().compared(),
            0,
            "the verdict must carry the count it rests on"
        );
        assert!(
            d.label_moved.is_empty(),
            "and it is still true that nothing was OBSERVED to move — that was never the lie"
        );
    }

    /// ★★ A clean verdict must state how many bindings it rests on, and name what it could not read.
    ///
    /// Two fields are comparable and unmoved; one is BOX-WITH-NO-LABEL on the new side only. The
    /// honest report is *"2 compared, none moved, 1 unwitnessed by name"* — never *"no change"*.
    /// Plant: drop the `Err(u) => unwitnessed.insert(..)` arm in [`label_axis`] and this reds on the
    /// missing name; make `compared` a constant and it reds on the count.
    #[test]
    fn a_clean_verdict_carries_its_evidence_count_and_names_the_gaps() {
        let common = s(&["a", "b", "c"]);
        let old = m(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let new = m(&[("a", "1"), ("b", "2"), ("c", "?")]);
        let d = delta_from(common, &old, &new);

        assert_eq!(
            d.label_verdict(),
            LabelVerdict::Unchanged {
                compared: 2,
                unwitnessed: 1
            },
            "the clean verdict must report 2 comparisons, not 3 and not silence"
        );
        assert_eq!(
            d.label_unwitnessed.get("c"),
            Some(&Unwitnessed::UnlabelledNew),
            "the field that could not be read must be named, with the reason — a count alone \
             cannot be acted on"
        );
        assert!(d.label_moved.is_empty());
    }

    /// ★★ A real move must stay a real move, and must be reported as a fraction of what was
    /// actually compared rather than of what happened to be present.
    #[test]
    fn a_move_among_unwitnessed_fields_is_still_a_move() {
        let common = s(&["a", "b", "c", "d"]);
        let old = m(&[("a", "?"), ("b", "5"), ("c", "6")]);
        let new = m(&[("a", "?"), ("b", "5"), ("c", "7"), ("d", "9")]);
        let d = delta_from(common, &old, &new);

        assert_eq!(
            d.label_verdict(),
            LabelVerdict::Moved {
                compared: 2,
                moved: 1,
                unwitnessed: 2
            },
            "`a` is unlabelled on both sides and `d` has no box on the old side; 2 of 4 were \
             actually compared and 1 of those moved"
        );
        assert_eq!(
            d.label_unwitnessed.get("d"),
            Some(&Unwitnessed::BoxAbsentOld),
            "a field the OLD geometry does not record must be named as such, not skipped"
        );
    }

    /// ★★ **A missing geometry fixture is unwitnessed too, and every common field still has to be
    /// accounted for.** `compute` builds this arm when either side's `label_join` fails, and `run`
    /// derives its non-zero exit from `is_witnessed()` — so the banner that already said *"an
    /// unchecked run is NOT a clean one"* now also EXITS like it means it.
    ///
    /// Reproduced end to end on a real artifact the day this was written: `f8275r--2025` has a PDF
    /// and no geometry, and `form-delta f8275r--2025 f8275r--2025` reports 102 of 102 common fields
    /// unwitnessed and exits 1 (it exited **0** before). That stem is not asserted here — extracting
    /// its geometry is legitimate work that must not red this test.
    ///
    /// Plant: make `compute`'s fixture-missing arm return an empty `unwitnessed` map, or make
    /// `is_witnessed` true for `NoFixture`, and this reds.
    #[test]
    fn a_missing_geometry_fixture_is_unwitnessed_not_clean() {
        let common = s(&["a", "b"]);
        let d = Delta {
            added: BTreeSet::new(),
            removed: BTreeSet::new(),
            label_moved: BTreeMap::new(),
            labels_available: false,
            label_compared: 0,
            label_unwitnessed: common
                .iter()
                .map(|f| (f.clone(), Unwitnessed::GeometryFixtureMissing))
                .collect(),
            common,
        };
        assert_eq!(
            d.label_verdict(),
            LabelVerdict::NoFixture { unwitnessed: 2 },
            "an axis that never ran must be its own verdict, and must still account for every field"
        );
        assert!(
            !d.label_verdict().is_witnessed(),
            "`run` turns this into a non-zero exit; if it reads as witnessed the tool goes back to \
             exiting 0 on a run that checked nothing"
        );
        assert_eq!(
            d.label_compared + d.label_unwitnessed.len(),
            d.common.len(),
            "the accounting invariant holds on this arm too — it is not exempt for being an error \
             path"
        );
    }

    /// ★★★ **The accounting invariant: every common field is either evidence or a named gap.**
    /// There is no third bucket, which is what makes the compared count meaningful — a field cannot
    /// be dropped by being forgotten.
    ///
    /// Checked on synthetic inputs covering every [`Unwitnessed`] variant *and* on the real
    /// artifacts, because an invariant that only holds on made-up data is a tautology.
    #[test]
    fn every_common_field_is_either_compared_or_named_as_unwitnessed() {
        // Synthetic: one of each outcome.
        let common = s(&[
            "ok",
            "moved",
            "noboxboth",
            "noboxold",
            "noboxnew",
            "qq",
            "q1",
            "q2",
        ]);
        let old = m(&[
            ("ok", "1"),
            ("moved", "2"),
            ("noboxnew", "3"),
            ("qq", "?"),
            ("q1", "?"),
            ("q2", "4"),
        ]);
        let new = m(&[
            ("ok", "1"),
            ("moved", "3"),
            ("noboxold", "5"),
            ("qq", "?"),
            ("q1", "6"),
            ("q2", "?"),
        ]);
        let d = delta_from(common.clone(), &old, &new);
        assert_eq!(
            d.label_compared + d.label_unwitnessed.len(),
            common.len(),
            "compared {} + unwitnessed {} must account for all {} common fields",
            d.label_compared,
            d.label_unwitnessed.len(),
            common.len()
        );
        let reasons: BTreeSet<Unwitnessed> = d.label_unwitnessed.values().copied().collect();
        assert_eq!(
            reasons,
            [
                Unwitnessed::BoxAbsentBothSides,
                Unwitnessed::BoxAbsentOld,
                Unwitnessed::BoxAbsentNew,
                Unwitnessed::UnlabelledBothSides,
                Unwitnessed::UnlabelledOld,
                Unwitnessed::UnlabelledNew,
            ]
            .into_iter()
            .collect::<BTreeSet<_>>(),
            "each way of failing to read a label must be distinguishable in the report"
        );

        // Real: the calibration pairs must obey the same accounting.
        for (a, b) in [
            ("f6251--2024", "f6251--2025"),
            ("f8995--2024", "f8995--2025"),
            ("f6251--2025", "f6251--2026-DRAFT"),
        ] {
            let d = compute(a, b).unwrap_or_else(|e| panic!("{a} -> {b}: {e}"));
            assert_eq!(
                d.label_compared + d.label_unwitnessed.len(),
                d.common.len(),
                "{a} -> {b}: compared {} + unwitnessed {} != {} common",
                d.label_compared,
                d.label_unwitnessed.len(),
                d.common.len()
            );
            assert!(
                d.label_verdict().is_witnessed(),
                "{a} -> {b} is a calibration pair; if its axis stops witnessing anything the \
                 calibration is gone: {:?}",
                d.label_verdict()
            );
        }
    }

    /// ★★ **The real-artifact census, enumerated FROM THE FILESYSTEM** — never a hand-list, which is
    /// the failure mode that produced most of the defects in this repo.
    ///
    /// Every archived `*--2026-DRAFT` geometry fixture is paired with its `--2025` counterpart and
    /// the whole set is walked. What this gates is `form_delta`'s own contract and nothing else:
    ///
    /// * every pair whose axis ran is **self-accounting** (compared + unwitnessed == common), and
    /// * **no pair anywhere yields a clean verdict backed by zero comparisons.**
    ///
    /// Pairs whose PDF or geometry is missing are named in the failure text of the vacuity guard
    /// rather than gated here — fixture coverage is `label_reader`'s gate, not this one — but they
    /// can never come back as `Unchanged`, which is the thing this file is responsible for.
    #[test]
    fn no_archived_pair_reports_a_clean_verdict_from_zero_comparisons() {
        let geom = crate::form_geometry::repo_root().join("design/forms/geometry");
        let mut drafts: Vec<String> = std::fs::read_dir(&geom)
            .unwrap_or_else(|e| panic!("{}: {e}", geom.display()))
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .filter(|s| s.ends_with("--2026-DRAFT"))
            .collect();
        drafts.sort();
        assert!(
            !drafts.is_empty(),
            "no TY2026 draft geometry found under {} — this test would otherwise pass vacuously",
            geom.display()
        );

        let mut witnessed = Vec::new();
        let mut blind = Vec::new();
        for draft in &drafts {
            let form = draft.split("--").next().expect("stem has a `--`");
            let prior = format!("{form}--2025");
            let d = match compute(&prior, draft) {
                Ok(d) => d,
                // No PDF on one side: the pair cannot be diffed at all. Named, not silently dropped.
                Err(e) => {
                    blind.push(format!("{prior} -> {draft}: {e}"));
                    continue;
                }
            };
            assert_eq!(
                d.label_compared + d.label_unwitnessed.len(),
                d.common.len(),
                "{prior} -> {draft}: compared {} + unwitnessed {} != {} common — a field was \
                 dropped rather than accounted for",
                d.label_compared,
                d.label_unwitnessed.len(),
                d.common.len()
            );
            match d.label_verdict() {
                LabelVerdict::Unchanged { compared, .. } | LabelVerdict::Moved { compared, .. } => {
                    assert!(
                        compared > 0,
                        "{prior} -> {draft}: a verdict about drift may not be reported with zero \
                         comparisons behind it"
                    );
                    witnessed.push(format!("{prior} -> {draft}: {compared} compared"));
                }
                v @ (LabelVerdict::NoFixture { .. } | LabelVerdict::Unwitnessed { .. }) => {
                    blind.push(format!("{prior} -> {draft}: {v:?}"));
                }
            }
        }
        assert!(
            !witnessed.is_empty(),
            "not one archived 2025 -> 2026-DRAFT pair produced a witnessed label verdict, so this \
             test proved nothing. Unwitnessed pairs were: {blind:#?}"
        );
    }
}
