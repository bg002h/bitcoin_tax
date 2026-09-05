//! ★★ **Every committed field map is checked against its own PDF — and the SET IS DERIVED.**
//!
//! The audit of the TY2025 map batch (`design/agent-reports/2026-09-05-ty2025-map-AUDIT.md`, M-3)
//! found ten freshly-committed maps that **no compiled consumer loaded**. `map_2025_matches_bundled_
//! pdf_fieldset` in `kats.rs` names its fixtures by hand — 8949 and Schedule D — so it could not
//! reach them, and "N fields mapped" stayed an unexecuted claim in a doc comment.
//!
//! That is this repo's dominant defect shape twice over: a checker green because it never RAN, and an
//! expected set written as a HAND-LIST instead of derived from the source. So this test walks
//! `crates/btctax-forms/forms/` on the filesystem, and every `.map.toml` it finds is checked. A new
//! map is covered the moment it is committed, with nobody remembering to add it.

use btctax_forms::testonly::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn forms_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("forms")
}

/// Every `(year, stem, map_path, pdf_path)` on disk. Derived, never hand-listed.
fn every_committed_map() -> Vec<(String, String, PathBuf, PathBuf)> {
    let mut out = Vec::new();
    let mut years: Vec<_> = std::fs::read_dir(forms_root())
        .expect("forms/ must exist")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    years.sort();
    for y in years {
        let dir = forms_root().join(&y);
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().ends_with(".map.toml"))
            .collect();
        entries.sort();
        for map in entries {
            let stem = map
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches(".map.toml")
                .to_string();
            let pdf = dir.join(format!("{stem}.pdf"));
            out.push((y.clone(), stem, map, pdf));
        }
    }
    out
}

/// Every string in the TOML that is shaped like an AcroForm fully-qualified field name.
/// Deliberately structural rather than key-driven: the maps nest differently per form, and a
/// key-driven walk would silently skip the shapes it did not anticipate.
fn field_names(v: &toml::Value, out: &mut BTreeSet<String>) {
    match v {
        toml::Value::String(s) => {
            // An AcroForm FQN in these maps always contains a `[` index and a `.` separator.
            if s.contains('[') && s.contains('.') && !s.contains(' ') {
                out.insert(s.clone());
            }
        }
        toml::Value::Array(a) => a.iter().for_each(|x| field_names(x, out)),
        toml::Value::Table(t) => t.values().for_each(|x| field_names(x, out)),
        _ => {}
    }
}

/// ★★★ The check the batch never had: every field a map names must EXIST in that form's PDF.
///
/// This is what makes a map artifact a claim the compiler and the suite can refute. A ported map
/// whose year renamed its fields parses fine, reads right, and writes into boxes that do not exist —
/// measured on this very batch, the IRS renamed 18 of Form 8995's 33 fields for TY2025 while the
/// printed form did not change at all.
#[test]
fn every_committed_map_field_exists_in_its_own_pdf() {
    let maps = every_committed_map();
    assert!(
        maps.len() >= 20,
        "the walk found only {} maps — forms/ is not being read correctly",
        maps.len()
    );

    let mut checked_fields = 0usize;
    let mut missing: Vec<String> = Vec::new();

    for (year, stem, map_path, pdf_path) in &maps {
        assert!(
            pdf_path.exists(),
            "{year}/{stem}.map.toml has no sibling {stem}.pdf — a map with no form is unfillable"
        );
        let text = std::fs::read_to_string(map_path).unwrap();
        let parsed: toml::Value = toml::from_str(&text)
            .unwrap_or_else(|e| panic!("{year}/{stem}.map.toml is not valid TOML: {e}"));

        let mut names = BTreeSet::new();
        field_names(&parsed, &mut names);
        assert!(
            !names.is_empty(),
            "{year}/{stem}.map.toml names no AcroForm fields at all — the map is empty or the \
             extractor is blind to its shape (either is a defect, and a silent one)"
        );

        let bytes = std::fs::read(pdf_path).unwrap();
        let doc = load(&bytes).unwrap();
        let present: BTreeSet<String> = collect_fields(&doc)
            .unwrap()
            .into_iter()
            .map(|f| f.fqn)
            .collect();

        for n in &names {
            checked_fields += 1;
            if !present.contains(n) {
                missing.push(format!("{year}/{stem}: {n}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "{} mapped field(s) do not exist in their own PDF — a fill would write nowhere:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
    assert!(
        checked_fields > 400,
        "only {checked_fields} field references checked; the walk is not reaching the maps"
    );
    eprintln!(
        "checked {checked_fields} field references across {} committed maps",
        maps.len()
    );
}

/// ★ A map must not name the same PDF field for two different lines. Two lines sharing one box is
/// the ISO 32000 same-name trap: both values collapse into one `/V` and one of them is silently lost.
/// Rows of a repeating grid legitimately repeat a *shape*, not a field name, so this holds for all.
#[test]
fn no_committed_map_points_two_lines_at_the_same_pdf_field() {
    let mut offenders: Vec<String> = Vec::new();
    for (year, stem, map_path, _) in every_committed_map() {
        let text = std::fs::read_to_string(&map_path).unwrap();
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        // Count with multiplicity by walking the raw text: `field_names` de-duplicates by design.
        for line in text.lines() {
            let l = line.trim();
            if l.starts_with('#') {
                continue;
            }
            if let Some(q0) = l.find('"') {
                if let Some(q1) = l[q0 + 1..].find('"') {
                    let s = &l[q0 + 1..q0 + 1 + q1];
                    if s.contains('[') && s.contains('.') && !s.contains(' ') {
                        *counts.entry(s.to_string()).or_default() += 1;
                    }
                }
            }
        }
        for (f, n) in counts {
            if n > 1 {
                offenders.push(format!("{year}/{stem}: {f} named {n}x"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a PDF field is named by more than one line — the values collapse into one /V:\n  {}",
        offenders.join("\n  ")
    );
}

/// ★★★ **The year hardcode, and why a refusal is the only safe default.**
///
/// The map audit found `packet.rs` calling `Form6251Map::ty2024()` and `Form8995AMap::ty2024()`
/// with `year` already in scope two lines away. That is a wrong-number path waiting to be reached,
/// not a missing feature: TY2025 split Form 6251 line 1 into 1a/1b and shifted the page-1 field
/// names, so a TY2024 map applied to a TY2025 PDF writes 2a into 1b's box and walks everything
/// below down one — landing **line 11, the AMT itself**, in line 10's box. Nothing errors. The
/// paper looks right and the figure is wrong.
///
/// So an unmapped year must REFUSE rather than fall back to a neighbouring year's geometry.
#[test]
fn a_year_with_no_bundled_6251_map_refuses_instead_of_reusing_another_years_geometry() {
    use btctax_forms::testonly::{Form6251Map, Form8995AMap};

    assert!(
        Form6251Map::for_year(2024).is_ok(),
        "TY2024 is bundled and must resolve"
    );
    for unmapped in [2023, 2025, 2026] {
        assert!(
            Form6251Map::for_year(unmapped).is_err(),
            "TY{unmapped} has no bundled 6251 map and must REFUSE — silently reusing another \
             year's field names puts the AMT in the wrong box"
        );
        assert!(
            Form8995AMap::for_year(unmapped).is_err(),
            "TY{unmapped} has no bundled 8995-A map and must refuse for the same reason"
        );
    }
}
