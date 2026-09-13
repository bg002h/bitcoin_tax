//! ★ Design r2 §5 — **the binding is a build script, and it decides NOTHING.**
//!
//! It globs `forms/<year>/` under this crate's own manifest directory (never anywhere else — the
//! publishing trap: an `include_*!` that reaches outside the crate ships a broken tarball with exit
//! 0) and emits `bundled.rs` into `OUT_DIR`: one `include_bytes!` per template, one `include_str!`
//! per map, the list of years found, and the list of `(Stem, year)` pairs. Every judgment about a
//! form lives in the map's ROW (`MapRow`, design r2 §4), written by a human; every filler is code.
//!
//! The `Stem` enum is HAND-WRITTEN in `src/bundled.rs` (a new FORM needs a filler and is correctly a
//! code change). This script derives the variant name from the file stem by one rule; a stem with no
//! variant makes the generated code fail to compile, which is the intended signal: "you added a form
//! without adding its filler". A new YEAR of an existing form is files under a directory and nothing
//! else.
//!
//! ★ **It also generates [`LineSet`]** (`line_set_generated.rs` in `OUT_DIR`, included by
//! `src/line_set.rs`): the closed set of line-set revisions, plus its `parse`, its `as_str` and its
//! `ALL`. Those four are pure transcription of ONE string that is already on disk — the `line_set`
//! field of each map's ROW — and hand-typing them beside a glob that grows is exactly the defect
//! `CLAUDE.md`'s *"derive the list, or make the compiler hold it"* names. Measured on the
//! `f8995a/2025` port rehearsal (`design/agent-reports/REPORT-rehearse-port-f8995a-2025.md` F10,
//! `FOLLOWUPS.md` FR-141): **7 hand-edits in `line_set.rs` + `f6251_revision.rs` per new
//! `(stem, year)`**, six of which decided nothing. The ONE that decides — `line_set::schema`, naming
//! the struct that transcribes a revision — stays hand-written and stays an `_`-free match over this
//! generated enum, so a new row still cannot compile until a human says which struct parses it.
//!
//! ★★ Two of those seven were worse than redundant. `ALL` is ITERATED by every gate that walks the
//! revision set (`tests/line_set_wiring.rs`, `tests/f6251_obbba.rs`,
//! `tests/supported_years_cross_product.rs`) and nothing held it to the glob — so measured 2026-09-12,
//! a port that added the variant and forgot the `ALL` entry reds **nothing**, and that revision is
//! simply never measured by any of them again.
//!
//! What it refuses (a build error, not a silent skip): a year directory that is not four digits; a
//! `.pdf` with no `.map.toml` beside it or a map with no PDF — location is status (§3), and an
//! unpaired file is a form the crate would list and could not fill; a map whose header does not carry
//! exactly one readable `line_set` — the revision a map transcribes is not optional, and a map whose
//! revision this script cannot read would drop that revision out of the set every gate iterates.
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn variant_name(stem: &str) -> String {
    // `f1040s1a` → `F1040s1a`; `schedule_d` → `ScheduleD`; `schedule_se` → `ScheduleSe`.
    stem.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// The `line_set` of ONE map, read out of its header — the only field of a map this script reads.
///
/// ★ Deliberately a line scan and not a TOML parse: this crate has no `[build-dependencies]`, and the
/// script must go on deciding nothing. The STRICTNESS is what makes a scan safe rather than clever —
/// **exactly one** `line_set = "…"` assignment, **in the header** (before the first `[section]`), or it
/// is a build error. The real TOML parse still happens at runtime in `MapRow::read`, and the two are
/// cross-checked by every gate that resolves a row's revision: a value this scan read wrong would not
/// parse back into a variant, and the form would report a `dispatch` gap.
fn line_set_of(path: &Path) -> String {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("build.rs: cannot read {}: {e}", path.display()));
    let mut in_header = true;
    let mut found: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            in_header = false;
        }
        let Some(rest) = t.strip_prefix("line_set") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue; // `line_set_something = …` — a different key, not this one.
        };
        let value = rest
            .trim_start()
            .strip_prefix('"')
            .and_then(|r| r.split('"').next())
            .unwrap_or_else(|| {
                panic!(
                    "build.rs: {}: `line_set` is not a quoted string — the revision a map \
                     transcribes is what every gate iterates on, and it must be readable",
                    path.display()
                )
            });
        if !in_header {
            panic!(
                "build.rs: {}: `line_set` appears after a `[section]` header — the ROW is the map's \
                 top-level header (design r2 §4), and a `line_set` nested in a table is not the row's",
                path.display()
            );
        }
        found.push(value.to_string());
    }
    match found.len() {
        1 => found.pop().unwrap(),
        0 => panic!(
            "build.rs: {}: the header carries no `line_set = \"<stem>/<year>\"`. Every map names the \
             LINE-SET REVISION it transcribes (design r2 §4); without it that revision is absent from \
             `LineSet`, and every gate that walks `LineSet::ALL` skips this map in silence.",
            path.display()
        ),
        n => panic!(
            "build.rs: {}: `line_set` is assigned {n} times in the header — one map transcribes one \
             revision, and two assignments make the generated variant whichever the scan saw last",
            path.display()
        ),
    }
}

/// The `LineSet` variant name for a row string: `"f8995a/2024"` -> `F8995a_2024`.
///
/// Refuses anything that is not `<stem>/<four-digit year>` — the shape `tests/map_rows.rs` holds every
/// committed row to. A row this cannot read is a build error, never a mangled variant name.
fn line_set_variant(ls: &str, path: &Path) -> String {
    let (stem, year) = ls.split_once('/').unwrap_or_else(|| {
        panic!(
            "build.rs: {}: `line_set = \"{ls}\"` is not `<stem>/<year>`",
            path.display()
        )
    });
    let ok_stem = !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    let ok_year = year.len() == 4 && year.chars().all(|c| c.is_ascii_digit());
    if !(ok_stem && ok_year) {
        panic!(
            "build.rs: {}: `line_set = \"{ls}\"` is not `<stem>/<four-digit year>` — the shape every \
             committed row carries",
            path.display()
        );
    }
    format!("{}_{year}", variant_name(stem))
}

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let forms = manifest_dir.join("forms");
    println!("cargo:rerun-if-changed=forms");

    // year → stem → (has_pdf, has_map)
    let mut found: BTreeMap<i32, BTreeMap<String, (bool, bool)>> = BTreeMap::new();
    let mut year_records: Vec<(i32, PathBuf)> = Vec::new();
    let mut year_dirs: Vec<PathBuf> = std::fs::read_dir(&forms)
        .unwrap_or_else(|e| panic!("build.rs: cannot read {}: {e}", forms.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    year_dirs.sort();
    for dir in &year_dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let year: i32 = match (name.len(), name.parse::<i32>()) {
            (4, Ok(y)) => y,
            _ => panic!(
                "build.rs: `forms/{name}/` is not a four-digit tax year — the year package's location IS its \
                 status (design r2 §3), and a directory that is not a year is a form outside every gate"
            ),
        };
        println!("cargo:rerun-if-changed={}", dir.display());
        // ★ Design r2 §6 — the YEAR RECORD. A year directory without one is a year with no declared
        //   intent (no expected form set, no due date, no oracle, no regime) — refused at build.
        let record = dir.join("YEAR.toml");
        if !record.is_file() {
            panic!(
                "build.rs: forms/{name}/ has no YEAR.toml — every bundled year declares itself (design r2 §6: \
                 status, return_due, forms_expected, forms_absent, tables, oracles, prices_through, \
                 information_returns). Location is status; a year without a record is a form outside every gate."
            );
        }
        year_records.push((year, record));
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        files.sort();
        for f in files {
            println!("cargo:rerun-if-changed={}", f.display());
            let fname = f.file_name().unwrap().to_string_lossy().to_string();
            let entry = found.entry(year).or_default();
            if let Some(stem) = fname.strip_suffix(".map.toml") {
                entry.entry(stem.to_string()).or_default().1 = true;
            } else if let Some(stem) = fname.strip_suffix(".pdf") {
                entry.entry(stem.to_string()).or_default().0 = true;
            }
        }
    }

    let mut out = String::new();
    writeln!(
        out,
        "// GENERATED by build.rs from the glob of `forms/<year>/` — do not edit."
    )
    .unwrap();
    writeln!(
        out,
        "// Every judgment about a form is in its map's ROW; this file only binds files."
    )
    .unwrap();
    let mut pairs: Vec<(i32, String)> = Vec::new();
    for (year, stems) in &found {
        for (stem, (has_pdf, has_map)) in stems {
            if !(*has_pdf && *has_map) {
                panic!(
                    "build.rs: forms/{year}/{stem} has {} — a template without a map (or a map without a \
                     template) is a form the crate would list and could not fill. Location is status: \
                     both files, or neither.",
                    if *has_pdf { "a .pdf but no .map.toml" } else { "a .map.toml but no .pdf" }
                );
            }
            pairs.push((*year, stem.clone()));
        }
    }

    // ★ Design r2 §4 + FR-141 — THE REVISION SET, derived from the rows instead of typed beside them.
    //   Ordered by the glob (year, then stem) exactly as `pairs` is, and DEDUPED: two maps naming one
    //   `line_set` is the many-to-one collapse design r2 §4 leaves open (a constants-only year reusing
    //   its predecessor's revision), and it must be one variant carried by two files, never two.
    let mut revisions: Vec<(String, String, Vec<String>)> = Vec::new(); // (line_set, variant, maps)
    for (year, stem) in &pairs {
        let map_path = forms
            .join(year.to_string())
            .join(format!("{stem}.map.toml"));
        let ls = line_set_of(&map_path);
        let variant = line_set_variant(&ls, &map_path);
        let carried_by = format!("forms/{year}/{stem}.map.toml");
        match revisions.iter_mut().find(|(seen, _, _)| *seen == ls) {
            Some((_, _, maps)) => maps.push(carried_by),
            None => revisions.push((ls, variant, vec![carried_by])),
        }
    }
    // Two rows cannot generate the SAME variant name from DIFFERENT strings — `f1040_s1/2024` and
    // `f1040s1/2024` both spell `F1040s1_2024` under `variant_name`'s one rule, and the generated enum
    // would not compile. It would not compile for the right reason, but with a message about a
    // duplicate variant rather than about two rows; say which rows, here.
    for i in 0..revisions.len() {
        for j in (i + 1)..revisions.len() {
            if revisions[i].1 == revisions[j].1 {
                panic!(
                    "build.rs: `line_set = \"{}\"` ({}) and `line_set = \"{}\"` ({}) both spell the \
                     variant `{}` — two revisions the generated enum cannot tell apart",
                    revisions[i].0,
                    revisions[i].2.join(", "),
                    revisions[j].0,
                    revisions[j].2.join(", "),
                    revisions[i].1,
                );
            }
        }
    }
    // template()
    writeln!(
        out,
        "\n/// The bundled PDF for `(stem, year)`, or `None` when this build carries no such file."
    )
    .unwrap();
    writeln!(
        out,
        "pub fn template(stem: Stem, year: i32) -> Option<&'static [u8]> {{"
    )
    .unwrap();
    writeln!(out, "    match (stem, year) {{").unwrap();
    for (year, stem) in &pairs {
        let p: &Path = &forms.join(year.to_string()).join(format!("{stem}.pdf"));
        writeln!(
            out,
            "        (Stem::{}, {year}) => Some(include_bytes!({:?})),",
            variant_name(stem),
            p.display().to_string()
        )
        .unwrap();
    }
    writeln!(out, "        _ => None,\n    }}\n}}").unwrap();
    // map_text()
    writeln!(out, "\n/// The committed map (TOML text) for `(stem, year)`, or `None` when this build carries none.").unwrap();
    writeln!(
        out,
        "pub fn map_text(stem: Stem, year: i32) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(out, "    match (stem, year) {{").unwrap();
    for (year, stem) in &pairs {
        let p: &Path = &forms
            .join(year.to_string())
            .join(format!("{stem}.map.toml"));
        writeln!(
            out,
            "        (Stem::{}, {year}) => Some(include_str!({:?})),",
            variant_name(stem),
            p.display().to_string()
        )
        .unwrap();
    }
    writeln!(out, "        _ => None,\n    }}\n}}").unwrap();
    // years + pairs
    let years: Vec<String> = found.keys().map(|y| y.to_string()).collect();
    writeln!(out, "\n/// Every tax year with at least one bundled form — DERIVED from `forms/`, replacing `SUPPORTED_YEARS`.").unwrap();
    writeln!(
        out,
        "pub const BUNDLED_YEARS: &[i32] = &[{}];",
        years.join(", ")
    )
    .unwrap();
    // ★ A `preparing` year may bundle its RECORD and no template at all (TY2026 since spec 1099-DA
    //   T0). Everything that means "a year btctax can FILL" — the cluster guards, the cross-product
    //   matrix, cite-check's template obligations, the era preset — keys on this list, not on
    //   BUNDLED_YEARS: a year with zero templates has zero obligations of that kind.
    let template_years: Vec<String> = found
        .iter()
        .filter(|(_, stems)| stems.values().any(|(pdf, map)| *pdf || *map))
        .map(|(y, _)| y.to_string())
        .collect();
    writeln!(out, "\n/// Every tax year with at least one bundled TEMPLATE — the years the crate can fill. A `preparing` year with only a `YEAR.toml` is in `BUNDLED_YEARS` and not here.")
        .unwrap();
    writeln!(
        out,
        "pub const TEMPLATE_YEARS: &[i32] = &[{}];",
        template_years.join(", ")
    )
    .unwrap();
    writeln!(
        out,
        "\n/// Every `(Stem, year)` this build binds — the row SET, as the glob found it."
    )
    .unwrap();
    writeln!(out, "pub const BUNDLED: &[(Stem, i32)] = &[").unwrap();
    for (year, stem) in &pairs {
        writeln!(out, "    (Stem::{}, {year}),", variant_name(stem)).unwrap();
    }
    writeln!(out, "];").unwrap();

    // year_record_text()
    writeln!(out, "\n/// The committed `forms/<year>/YEAR.toml` text — the year's DECLARATION (design r2 §6).").unwrap();
    writeln!(
        out,
        "pub fn year_record_text(year: i32) -> Option<&'static str> {{"
    )
    .unwrap();
    writeln!(out, "    match year {{").unwrap();
    for (year, path) in &year_records {
        writeln!(
            out,
            "        {year} => Some(include_str!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    writeln!(out, "        _ => None,\n    }}\n}}").unwrap();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let dest = out_dir.join("bundled.rs");
    std::fs::write(&dest, out)
        .unwrap_or_else(|e| panic!("build.rs: cannot write {}: {e}", dest.display()));

    // ── `LineSet` — the closed set of revisions, its parse, its as_str and its ALL. ──
    let mut ls_out = String::new();
    writeln!(
        ls_out,
        "// GENERATED by build.rs from the `line_set` row of every `forms/<year>/*.map.toml` — do not edit."
    )
    .unwrap();
    writeln!(
        ls_out,
        "// The revision set IS the set of rows (design r2 §4). The one thing a NEW row still costs a"
    )
    .unwrap();
    writeln!(
        ls_out,
        "// human is the `line_set::schema` arm naming the struct that transcribes it, which does not"
    )
    .unwrap();
    writeln!(
        ls_out,
        "// compile until it is written: FR-141 removed the transcription, not the judgment."
    )
    .unwrap();
    writeln!(ls_out, "\n/// The closed set of line-set revisions this build knows — one variant per distinct `line_set`").unwrap();
    writeln!(
        ls_out,
        "/// row, DERIVED from the glob of `forms/<year>/*.map.toml` (FR-141)."
    )
    .unwrap();
    writeln!(ls_out, "#[allow(non_camel_case_types)]").unwrap();
    writeln!(
        ls_out,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]"
    )
    .unwrap();
    writeln!(ls_out, "pub enum LineSet {{").unwrap();
    for (ls, variant, maps) in &revisions {
        writeln!(
            ls_out,
            "    /// `{:?}` — the revision transcribed by {}.",
            ls,
            maps.iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(" and ")
        )
        .unwrap();
        writeln!(ls_out, "    {variant},").unwrap();
    }
    writeln!(ls_out, "}}").unwrap();
    writeln!(ls_out, "\nimpl LineSet {{").unwrap();
    writeln!(ls_out, "    /// Parse a row's `line_set` string. `None` is a revision this build does not know — a refusal.").unwrap();
    writeln!(
        ls_out,
        "    pub fn parse(s: &str) -> Option<LineSet> {{\n        match s {{"
    )
    .unwrap();
    for (ls, variant, _) in &revisions {
        writeln!(ls_out, "            {:?} => Some(LineSet::{variant}),", ls).unwrap();
    }
    writeln!(ls_out, "            _ => None,\n        }}\n    }}").unwrap();
    writeln!(ls_out, "\n    /// The row string this variant names.").unwrap();
    writeln!(
        ls_out,
        "    pub const fn as_str(self) -> &'static str {{\n        match self {{"
    )
    .unwrap();
    for (ls, variant, _) in &revisions {
        writeln!(ls_out, "            LineSet::{variant} => {:?},", ls).unwrap();
    }
    writeln!(ls_out, "        }}\n    }}").unwrap();
    writeln!(ls_out, "\n    /// Every variant, in the glob's own order — the list every gate that walks the revision set").unwrap();
    writeln!(ls_out, "    /// iterates. DERIVED: a bundled map cannot be missing from it (FR-141), which it silently could be").unwrap();
    writeln!(ls_out, "    /// while it was hand-written.").unwrap();
    writeln!(ls_out, "    pub const ALL: &'static [LineSet] = &[").unwrap();
    for (_, variant, _) in &revisions {
        writeln!(ls_out, "        LineSet::{variant},").unwrap();
    }
    writeln!(ls_out, "    ];").unwrap();
    writeln!(ls_out, "}}").unwrap();
    let ls_dest = out_dir.join("line_set_generated.rs");
    std::fs::write(&ls_dest, ls_out)
        .unwrap_or_else(|e| panic!("build.rs: cannot write {}: {e}", ls_dest.display()));
}
