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

/// The delta, as data. `None` for a label map means that side has no geometry fixture — reported,
/// never silently treated as "no change".
pub struct Delta {
    pub added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
    pub common: BTreeSet<String>,
    /// Fields present in BOTH whose printed line label moved: name -> (old label, new label).
    pub label_moved: BTreeMap<String, (String, String)>,
    pub labels_available: bool,
}

pub fn compute(old: &str, new: &str) -> Result<Delta, String> {
    let a = field_set(old)?;
    let b = field_set(new)?;
    let common: BTreeSet<String> = a.intersection(&b).cloned().collect();

    let (la, lb) = (
        crate::label_reader::label_join_public(old).ok(),
        crate::label_reader::label_join_public(new).ok(),
    );
    let mut label_moved = BTreeMap::new();
    let labels_available = la.is_some() && lb.is_some();
    if let (Some(la), Some(lb)) = (la, lb) {
        for f in &common {
            let (x, y) = (la.get(f), lb.get(f));
            if let (Some(x), Some(y)) = (x, y) {
                if x != y && x != "?" && y != "?" {
                    label_moved.insert(f.clone(), (x.clone(), y.clone()));
                }
            }
        }
    }
    Ok(Delta {
        added: b.difference(&a).cloned().collect(),
        removed: a.difference(&b).cloned().collect(),
        common,
        label_moved,
        labels_available,
    })
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
    for f in d.added.iter().take(8) {
        println!("    + {f}");
    }
    for f in d.removed.iter().take(8) {
        println!("    - {f}");
    }
    if !d.labels_available {
        println!(
            "  ★ LINE->LABEL DRIFT NOT CHECKED — one side has no geometry fixture. Generate with \
             `xtask extract-geometry <stem>`. This is the axis a name check cannot see, so an \
             unchecked run is NOT a clean one."
        );
        return Ok(());
    }
    if d.label_moved.is_empty() {
        println!("  line->label: no field changed the printed line it sits beside");
    } else {
        println!(
            "  ★★ {} field(s) KEPT THEIR NAME but now sit beside a different printed line — a map \
             carried forward unchanged would write to the wrong line of a signed return:",
            d.label_moved.len()
        );
        for (f, (x, y)) in d.label_moved.iter().take(20) {
            println!("    {f}: line {x} -> {y}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
