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

/// ★★ **The TYPED fieldset guard for the two rows spec 4868/1040-V T1 landed** — sp2/sp3 style, but
/// per YEAR and against THAT year's own bundled asset.
///
/// [`every_committed_map_field_exists_in_its_own_pdf`] above already walks these maps as TEXT. This
/// one asks the different question: does the *struct* the filler will read — `Form4868Map` /
/// `Form1040VMap`, whose `field_names()` is what a fill enumerates — name only fields the bundled PDF
/// actually has? A binding the struct forgot to model would pass the text walk and be invisible here
/// by absence, which is why `field_names()` is length-checked too: 12 of the 4868's 17 boxes and 12
/// of the 1040-V's 15 are bound, the rest are `[census]` entries.
#[test]
fn the_4868_and_1040v_maps_name_only_fields_their_own_bundled_pdf_carries() {
    use btctax_forms::testonly::{Form1040VMap, Form4868Map};
    let present = |bytes: &[u8]| -> BTreeSet<String> {
        collect_fields(&load(bytes).unwrap())
            .unwrap()
            .into_iter()
            .map(|f| f.fqn)
            .collect()
    };
    let mut checked = 0usize;
    for year in [2024, 2025] {
        let m = Form4868Map::for_year(year).unwrap_or_else(|e| panic!("TY{year} f4868 map: {e}"));
        assert_eq!(m.year, year);
        let set = present(Form4868Map::bundled_pdf(year).unwrap());
        assert_eq!(set.len(), 17, "TY{year} Form 4868 has 17 AcroForm fields");
        let names = m.field_names();
        assert_eq!(names.len(), 12, "TY{year} f4868 binds 12 of the 17 boxes");
        for n in &names {
            assert!(
                set.contains(*n),
                "TY{year} f4868: {n} is not a field of its own PDF"
            );
            checked += 1;
        }

        let v = Form1040VMap::for_year(year).unwrap_or_else(|e| panic!("TY{year} f1040v map: {e}"));
        assert_eq!(v.year, year);
        let set = present(Form1040VMap::bundled_pdf(year).unwrap());
        assert_eq!(set.len(), 15, "TY{year} Form 1040-V has 15 AcroForm fields");
        let names = v.field_names();
        assert_eq!(names.len(), 12, "TY{year} f1040v binds 12 of the 15 boxes");
        for n in &names {
            assert!(
                set.contains(*n),
                "TY{year} f1040v: {n} is not a field of its own PDF"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 48, "2 forms x 2 years x 12 bindings");
}

/// ★★★ B1 for the guard above — and the plant turned out to be REAL, not hypothetical.
///
/// The same comparison pointed at the OTHER form's bundled PDF must report the mismatch. It does,
/// but only for **8 of the 4868's 12 bindings**: `f1_11`…`f1_14` — Form 4868's lines 4, 5, 6 and 7,
/// the money — are spelled `topmostSubform[0].Page1[0].f1_NN[0]` on BOTH forms, so those four names
/// exist in the Form 1040-V AcroForm too. Only the seven `PartI_ReadOrder`-prefixed cells and the
/// `c1_1` checkbox are absent. **A fieldset/existence check is therefore necessary and NOT
/// sufficient** for these two rows — a 4868 map applied to the voucher would write the balance due
/// into the voucher's name and address boxes with every field name resolving — which is exactly why
/// the line→label join (`xtask`'s `every_mapped_line_lands_on_its_own_printed_label`) witnesses the
/// 4868's five numbered lines against the geometry fixture of its own PDF. The counts below are
/// measured and pinned in both directions, so a future revision that renames its way into or out of
/// the collision is loud.
#[test]
fn the_4868_1040v_fieldset_guard_reds_when_the_pdf_is_the_other_form() {
    use btctax_forms::testonly::{Form1040VMap, Form4868Map};
    let present = |bytes: &[u8]| -> BTreeSet<String> {
        collect_fields(&load(bytes).unwrap())
            .unwrap()
            .into_iter()
            .map(|f| f.fqn)
            .collect()
    };
    let voucher = present(Form1040VMap::bundled_pdf(2025).unwrap());
    let ext_map = Form4868Map::ty2025();
    let absent: Vec<&str> = ext_map
        .field_names()
        .into_iter()
        .filter(|n| !voucher.contains(*n))
        .collect();
    assert_eq!(
        absent.len(),
        8,
        "measured 2026-09-06: 8 of the 4868's 12 bindings are absent from the 1040-V's AcroForm \
         (the seven PartI_ReadOrder cells and c1_1); the four that are NOT absent are f1_11..f1_14, \
         the 4868's lines 4-7, whose unprefixed names the voucher also carries. Found {absent:?}"
    );

    let extension = present(Form4868Map::bundled_pdf(2025).unwrap());
    let v_map = Form1040VMap::ty2025();
    let absent: Vec<&str> = v_map
        .field_names()
        .into_iter()
        .filter(|n| !extension.contains(*n))
        .collect();
    assert_eq!(
        absent.len(),
        9,
        "measured 2026-09-06: 9 of the voucher's 12 bindings are absent from Form 4868's AcroForm; \
         the three that are not are f1_11/f1_12/f1_13 — the same collision seen from the other \
         side. Found {absent:?}"
    );
}
