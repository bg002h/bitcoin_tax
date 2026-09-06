//! ★★ **S9 — the TY2017 FORM PACKAGE is gone, and nothing quietly still expects it.**
//!
//! Owner ruling 2026-09-06 (`design/ROADMAP_STATUS.md` §0a S9, `FOLLOWUPS.md` FR-61): TY2017 was
//! *supported in fill, unsupported in evidence* — five bundled PDFs and five maps with **zero**
//! provenance notes, MANIFEST entries, extracts, geometry fixtures or `[census]` sections
//! (measured by `tests/supported_years_cross_product.rs`, which is the instrument that found it).
//! The ruling was to DELETE the package rather than archive an authority chain for a year nobody
//! files.
//!
//! What was deliberately KEPT is the TY2017 `TaxTable`
//! (`btctax-adapters::tax_tables::ty2017`, pinned to Rev. Proc. 2016-55), so
//! `report --tax-year 2017` still computes the crypto delta for a filer with 2017 events. Deleting
//! forms is not the same as deleting a year, and the two are easy to conflate — hence the KAT for
//! that in `btctax-cli/tests/tax_report.rs`.
//!
//! ★ **Why a whole file for this.** A removal is the one change whose defect is an ABSENCE: every
//! surviving reference to the deleted directory would be a path that resolves to nothing, a hand-list that
//! silently lost a member, or a comment promising a year the build no longer ships. The gates that
//! caught the *live* references are the ones that already existed (`bundled.rs`, `map_rows.rs`,
//! `field_census.rs`, `year_record.rs`, `supported_years_cross_product.rs`). This file holds the
//! two things none of them can see: that the directory is really gone, and that no source or test
//! anywhere in the workspace still spells its path.

use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// KILL 1 — the year directory is gone, and the build no longer binds a single TY2017 asset.
///
/// Both halves matter: `build.rs` globs the directory, so the directory's absence is what makes the
/// bindings empty — and a stale `OUT_DIR` would show up here as bindings for files that are not on
/// disk.
#[test]
fn no_ty2017_form_package_is_on_disk_or_bound() {
    assert!(
        !crate_root().join("forms/2017").exists(),
        "crates/btctax-forms/forms/2017/ is back — S9 deleted it (owner ruling 2026-09-06)"
    );
    assert!(
        !btctax_forms::bundled::bundled_years().contains(&2017),
        "2017 is a bundled year again: {:?}",
        btctax_forms::bundled::bundled_years()
    );
    assert!(
        !btctax_forms::SUPPORTED_YEARS.contains(&2017),
        "2017 is a supported (fillable) year again: {:?}",
        btctax_forms::SUPPORTED_YEARS
    );
    let bound: Vec<String> = btctax_forms::bundled::BUNDLED
        .iter()
        .filter(|(_, y)| *y == 2017)
        .map(|(s, y)| format!("{y}/{s}"))
        .collect();
    assert!(
        bound.is_empty(),
        "the build still binds TY2017 assets: {bound:?}"
    );

    // Every stem, both accessors — a template OR a map text surviving for 2017 is the same defect.
    for stem in btctax_forms::bundled::Stem::ALL {
        assert!(
            btctax_forms::bundled::template(*stem, 2017).is_none(),
            "{stem}: a TY2017 template is still bound"
        );
        assert!(
            btctax_forms::bundled::map_text(*stem, 2017).is_none(),
            "{stem}: a TY2017 map is still bound"
        );
    }
    // …including through the PERIODIC alias, which serves bundled years that lack their own file.
    // Form 8275 and Form 8283 are the periodic forms; 2017 is not a bundled year, so neither may
    // reach it. (`bundled::periodic_template` gates on `BUNDLED_YEARS` for exactly this reason.)
    for stem in [
        btctax_forms::bundled::Stem::F8275,
        btctax_forms::bundled::Stem::F8283,
    ] {
        assert!(
            btctax_forms::bundled::periodic_template(stem, 2017).is_none(),
            "{stem}: the periodic alias still serves TY2017"
        );
    }
}

/// KILL 2 — the five TY2017 line-set revisions no longer PARSE.
///
/// `LineSet::parse` is the refusal that keeps an unknown `line_set` string out of a map row. If the
/// variants had been left behind, a re-committed TY2017 map would parse cleanly into a struct with no
/// PDF behind it — the quietest possible way for the package to come back.
#[test]
fn no_ty2017_line_set_revision_parses() {
    for s in [
        "f1040/2017",
        "f8283/2017",
        "f8949/2017",
        "schedule_d/2017",
        "schedule_se/2017",
    ] {
        assert!(
            btctax_forms::line_set::LineSet::parse(s).is_none(),
            "{s} still parses — the TY2017 LineSet variants outlived their package"
        );
    }
    assert!(
        btctax_forms::line_set::LineSet::ALL
            .iter()
            .all(|ls| !ls.as_str().ends_with("/2017")),
        "a /2017 revision is still in LineSet::ALL"
    );
}

/// KILL 3 — **no CODE anywhere in the workspace still names the `forms/2017` path.**
///
/// This is the one an absence-shaped change most needs and that no other gate can give: a path
/// literal in a test, a fixture reading a deleted file, a `join("forms/2017")` that now resolves to
/// nothing. `bundled.rs` proves the BINDINGS are empty; only a text walk proves nobody reads the
/// directory by hand. The walk is over `crates/` (`.rs` and `.toml`), derived from the filesystem
/// rather than a list of files to remember.
///
/// ★ **Comment lines are exempt, deliberately.** Several comments record what was MEASURED off
/// `forms/2017/f1040.pdf` and `forms/2017/schedule_se.pdf` before the drop — the evidence for why
/// the geometry-band panics refuse a wildcard, which cannot be re-derived now that the PDFs are
/// gone. A checker that forced those to be deleted would be destroying the record in order to pass,
/// which is the opposite of the point. What is forbidden is a path a program can FOLLOW.
///
/// ★ Also deliberately NOT over `design/` — the design corpus is a historical record, and the
/// reports and specs describing the TY2017 package are supposed to keep describing it.
#[test]
fn no_code_in_the_workspace_names_the_deleted_forms_2017_path() {
    // ★ B1 liveness, inline: the predicate must accept the defect it exists to catch and reject the
    // comment it exists to tolerate. Without this pair the walk below could be scanning nothing.
    assert!(
        names_the_path(r#"    let p = crate_root().join("forms/2017/f1040.map.toml");"#),
        "the predicate does not catch a live path literal — this gate would pass on anything"
    );
    assert!(
        !names_the_path("//       measured off `forms/2017/f1040.pdf` before the drop"),
        "the predicate must tolerate a comment recording the pre-drop measurement"
    );

    let this_file = Path::new(file!()).file_name().unwrap().to_owned();
    let mut hits = Vec::new();
    let mut files = 0usize;
    walk(&workspace_root().join("crates"), &mut |p: &Path| {
        let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
            return;
        };
        if ext != "rs" && ext != "toml" {
            return;
        }
        // This file names the path in its own predicate and its own assertions.
        if p.file_name() == Some(this_file.as_os_str()) {
            return;
        }
        files += 1;
        let Ok(text) = std::fs::read_to_string(p) else {
            return;
        };
        for (n, line) in text.lines().enumerate() {
            if names_the_path(line) {
                hits.push(format!("{}:{}: {}", p.display(), n + 1, line.trim()));
            }
        }
    });
    // ★ Liveness again, on the WALK: one that reached nothing would report success for the same
    // reason a blind checker does. Measured 2026-09-06: 388 `.rs`/`.toml` files under `crates/`
    // besides this one (389 including it). The floor is well under that so ordinary churn does not
    // red it, and well over zero so a broken walk does.
    assert!(
        files >= 300,
        "the walk saw only {files} source files — it has gone blind, and a blind grep passes"
    );
    assert!(
        hits.is_empty(),
        "these are CODE naming the deleted TY2017 form package (a comment recording the pre-drop \
         measurement is fine; a path a program can follow is not):\n  {}",
        hits.join("\n  ")
    );
}

/// True when `line` names the deleted directory as code rather than as prose. Comment lines — `//`,
/// `///`, `//!`, a TOML `#`, a `*` continuation — are prose.
fn names_the_path(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("//") || t.starts_with('#') || t.starts_with('*') {
        return false;
    }
    line.contains("forms/2017")
}

fn walk(dir: &Path, f: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            // `target/` is build output, never source.
            if p.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            walk(&p, f);
        } else {
            f(&p);
        }
    }
}
