//! ★★★ **TY2026 REHEARSAL — the EMITTER half (stage 2 Tier A).**
//!
//! **VALIDATES NOTHING.** It asks one question: *how far into the emitter can a TY2026 packet get?*
//! The compute half is `btctax-core/tests/ty2026_rehearsal.rs`.
//!
//! ★ The year set and the stem set are both **derived** — `bundled::bundled_years()` and
//! `Stem::ALL` — so a year package landing for 2026, or a new fillable form, changes this file's
//! answer without anyone editing it.

use btctax_forms::bundled::{bundled_years, periodic_template, years_sentence, Stem};

/// ★★ **TY2026 IS A BUNDLED YEAR WITH ZERO TEMPLATES, AND THE TWO SENSES ARE ALREADY SEPARATE.**
///
/// `bundled_years()` is the glob of `forms/<year>/`, and `forms/2026/` exists — it holds `YEAR.toml`
/// and nothing else. So `bundled_years()` **does** contain 2026 while not one form can be filled for
/// it. That is a real trap and the crate already avoids it: `years_sentence()` is built from
/// `TEMPLATE_YEARS`, with the reason written down — *"a `preparing` year with only its record (TY2026
/// since spec 1099-DA T0) is bundled but cannot fill anything, and a refusal that named it would send
/// the filer to a year with zero forms."*
///
/// This test pins both halves, because a future `bundled_years()`-based message would reintroduce the
/// trap and nothing else would notice.
#[test]
fn ty2026_is_a_bundled_year_that_can_fill_nothing() {
    let years = bundled_years();
    assert!(
        years.contains(&2026),
        "forms/2026/ carries a YEAR.toml, so the package year set includes it; got {years:?}"
    );
    let sentence = years_sentence();
    assert!(
        !sentence.contains("2026"),
        "the FILLABLE-years sentence must not name TY2026: {sentence:?}"
    );
    assert!(
        sentence.contains("2024") && sentence.contains("2025"),
        "the fillable years are 2024 and 2025: {sentence:?}"
    );

    // Zero own templates for 2026, derived over `Stem::ALL`: anything that resolves at all resolves
    // from an EARLIER year's file, which is the periodic alias, never a TY2026 revision.
    for stem in Stem::ALL {
        if let Some((_, served_from)) = periodic_template(*stem, 2026) {
            assert_ne!(
                served_from,
                2026,
                "{} resolved from a TY2026 file — a TY2026 template landed",
                stem.file_stem()
            );
        }
    }
}

/// ★★ **THE TWO FORMS THAT *DO* REACH TY2026, and why that is not good news on its own.**
///
/// `periodic_template` serves a revision-dated form from the newest archived year until a later
/// revision is archived, so Form 8275 (Rev. 10-2024) and Form 8283 (Rev. 12-2025) resolve for 2026
/// **because 2026 is in `BUNDLED_YEARS`**. The prediction file counts them as *not* blockers, and that
/// is right — a periodic form genuinely has no annual revision. What it means for the rehearsal is
/// that a TY2026 packet is not uniformly blocked at the map layer: two forms would fill and nineteen
/// would not, so the thing that stops an incoherent two-page packet is the CLI's export gate
/// (`year_readiness::slice_can_print`), not the map layer.
///
/// ★ The probe is over `Stem::ALL`, so the *set* of periodic stems is derived rather than typed.
#[test]
fn only_the_periodic_forms_reach_ty2026() {
    let reaching: Vec<&str> = Stem::ALL
        .iter()
        .filter(|s| periodic_template(**s, 2026).is_some())
        .map(|s| s.file_stem())
        .collect();
    assert_eq!(
        reaching,
        vec!["f8275", "f8283"],
        "the periodic set changed — say so in the report rather than editing this list"
    );
    let blocked = Stem::ALL.len() - reaching.len();
    assert_eq!(
        blocked,
        19,
        "Stem::ALL is {} forms; {blocked} of them cannot be filled for TY2026",
        Stem::ALL.len()
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ THE BUNDLED TEMPLATE MUST BE THE DOCUMENT ITS DIRECTORY YEAR NAMES.
//
// Tier B measured a TY2026 packet writing at exit 0 with **2024 printed on every face** — Schedule A
// line 5e reading "smaller of line 5d or $10,000" while carrying $31,000. Nothing in the tree caught
// it, and this is why: there are THREE content joins over a bundled template and not one of them is
// keyed to the directory year.
//
//   * `map_rows.rs` kill 2 — `sha256(pdf) == row.template_sha256`. The row lives beside the PDF and is
//     regenerated with it, so a mis-filed pair agrees with itself.
//   * `map_rows.rs` kill 3 — the hash must be in `design/forms/MANIFEST.json`'s authority set.
//     `manifest_authority_hashes` is a FLAT set over every non-DRAFT entry of every year, so a TY2024
//     PDF's hash IS an authority.
//   * `map_rows.rs` kill 4 — the printed *Attachment Sequence No.* must match the row's. Schedule A is
//     sequence 07 in both years, so this compares a year-STABLE number.
//
// ★★ And the guards that DO currently stop it are keyed to the ABSENCE of a 2026 extract, so they stop
//    firing in January — exactly when the mistake becomes possible.
//
// The join below is per-document and per-year: every archived extract records the SHA-256 of the PDF
// its text layer was taken from (`# sha256:…`), so a bundled template whose hash is that year's
// extract's hash IS the document we transcribed, and its printed revision is that year's BY
// CONSTRUCTION. Coverage is total today — 38 of 38 bundled templates — and it does not weaken in
// January: when `f1040sa--2026.txt` lands, the join demands the bundled 2026 template be that file's
// source.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

use btctax_forms::MapRow;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// The bundled PDF is the document the year's extract was taken from.
    Is,
    /// The bundled PDF is NOT that document — a mis-filed revision, Tier B's defect.
    IsNot,
    /// No archived extract for this (stem, year), so nothing holds the revision. Named, never skipped.
    NoExtract,
}

/// The pure decision, so the kill can be planted without touching a committed file.
fn verdict(pdf_sha256: &str, recorded_prefix: Option<&str>) -> Verdict {
    match recorded_prefix {
        None => Verdict::NoExtract,
        // The extract's header truncates the hash for legibility, so the join is a PREFIX test — and
        // a short prefix must not pass vacuously.
        Some(p) if p.len() >= 16 && pdf_sha256.starts_with(p) => Verdict::Is,
        Some(_) => Verdict::IsNot,
    }
}

/// The `# sha256:…` an archived extract records for the PDF its text layer came from.
fn recorded_prefix(extract_root: &Path, irs_stem: &str, year: i32) -> Option<String> {
    let p = extract_root.join(format!("{irs_stem}--{year}.txt"));
    let text = std::fs::read_to_string(p).ok()?;
    let line = text.lines().take(6).find(|l| l.contains("sha256:"))?;
    let after = line.split("sha256:").nth(1)?;
    let hex: String = after
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    (!hex.is_empty()).then_some(hex)
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

/// Every committed `(year, crate stem, map path)` under a forms root, derived from the glob.
fn every_map(forms_root: &Path) -> Vec<(i32, String, PathBuf)> {
    let mut out = Vec::new();
    for y in std::fs::read_dir(forms_root).expect("forms/").flatten() {
        let Ok(year) = y.file_name().to_string_lossy().parse::<i32>() else {
            continue;
        };
        for f in std::fs::read_dir(y.path())
            .expect("forms/<year>/")
            .flatten()
        {
            let name = f.file_name().to_string_lossy().into_owned();
            if let Some(stem) = name.strip_suffix(".map.toml") {
                out.push((year, stem.to_string(), f.path()));
            }
        }
    }
    out.sort();
    out
}

/// ★★★ **THE LIVE HALF — 38 of 38, and the two failure classes are NAMED rather than skipped.**
///
/// `irs_stem` comes from the map ROW (`schedule_d` gives `f1040sd`), so the alias is derived from the
/// committed artifact and not typed here.
#[test]
fn every_bundled_template_is_the_document_its_year_archived() {
    let ws = workspace();
    let forms_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("forms");
    let extract_root = ws.join("design/forms/extract");

    let mut is = 0usize;
    let mut is_not = Vec::new();
    let mut no_extract = Vec::new();
    for (year, stem, map) in every_map(&forms_root) {
        let text = std::fs::read_to_string(&map).expect("map text");
        let row = MapRow::read(&text).unwrap_or_else(|e| panic!("{year}/{stem}: {e}"));
        let pdf_path = map.with_extension("").with_extension("pdf");
        let bytes = std::fs::read(&pdf_path).unwrap_or_else(|e| panic!("{year}/{stem}: {e}"));
        let sha = format!("{:x}", Sha256::digest(&bytes));
        let rec = recorded_prefix(&extract_root, &row.irs_stem, year);
        match verdict(&sha, rec.as_deref()) {
            Verdict::Is => is += 1,
            Verdict::IsNot => is_not.push(format!("{year}/{stem} ({})", row.irs_stem)),
            Verdict::NoExtract => no_extract.push(format!("{year}/{stem} ({})", row.irs_stem)),
        }
    }

    assert!(
        is_not.is_empty(),
        "these bundled templates are NOT the document their directory year archived — a mis-filed \
         revision would print the wrong year on every face: {is_not:?}"
    );
    assert!(
        no_extract.is_empty(),
        "these bundled templates have no archived extract, so NOTHING holds their printed revision — \
         name them here rather than letting the join pass vacuously: {no_extract:?}"
    );
    assert_eq!(
        is, 38,
        "coverage changed: {is} bundled templates joined to their year's extract. A NEW pair is fine \
         — update this count. A SMALLER one means a template lost its archive."
    );
}

/// ★ **B1 — the join observed discriminating, on the two REAL documents Tier B's defect confuses.**
///
/// The plant is not synthetic: it takes the genuine TY2024 Schedule A template and asks the join
/// whether it is the document TY2025 archived. It must say no. Then the same bytes against their own
/// year must say yes, so the kill cannot be passing because the function always refuses.
#[test]
fn filing_the_2024_schedule_a_under_2025_is_refused() {
    let ws = workspace();
    let extract_root = ws.join("design/forms/extract");
    let forms_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("forms");

    let pdf_2024 = std::fs::read(forms_root.join("2024/f1040sa.pdf")).expect("the 2024 template");
    let sha_2024 = format!("{:x}", Sha256::digest(&pdf_2024));

    let rec_2025 = recorded_prefix(&extract_root, "f1040sa", 2025).expect("the 2025 extract");
    let rec_2024 = recorded_prefix(&extract_root, "f1040sa", 2024).expect("the 2024 extract");
    assert_ne!(
        rec_2024, rec_2025,
        "the two revisions must be different files"
    );

    assert_eq!(
        verdict(&sha_2024, Some(&rec_2025)),
        Verdict::IsNot,
        "the 2024 Schedule A filed under 2025 must be REFUSED"
    );
    assert_eq!(
        verdict(&sha_2024, Some(&rec_2024)),
        Verdict::Is,
        "and the same bytes under their own year must be ACCEPTED, or the refusal above is vacuous"
    );
    assert_eq!(verdict(&sha_2024, None), Verdict::NoExtract);
    // A too-short prefix must not pass vacuously — the header truncates, and a 4-char prefix would
    // match far too much.
    assert_eq!(verdict(&sha_2024, Some(&sha_2024[..4])), Verdict::IsNot);
}
