//! ★★★ **`SUPPORTED_YEARS` × the forms this build ships** — the cross-product nothing checked.
//!
//! [`btctax_forms::SUPPORTED_YEARS`] is a three-element list that reads as a warranty: *this build
//! bundles IRS forms for these years*. Before this file, **nothing in the workspace asserted anything
//! about it.** Grep it and the only hit outside a doc comment is its own definition and a hand-typed
//! sentence in `error.rs` 57 lines below. So the constant asserted more than it checked, in both
//! directions at once:
//!
//! - a year could be LISTED with no bundled asset, no map, no archive note (the refusal would name a
//!   year the build cannot fill), and
//! - a year could be BUNDLED on disk and never listed (its assets compile in, unreachable).
//!
//! **What TY2017 turned out to be, and what was done about it.** Measured by this file: TY2017 was
//! genuinely wired — five forms, five maps, bound by `build.rs` and served by `Schema` arms in all
//! five map types — so a TY2017 fill really did emit a filled IRS PDF. What it had **none** of was
//! the authority chain every other shipped year has: **0** provenance notes, **0** `MANIFEST.json`
//! entries, **0** committed extracts, **0** geometry fixtures and **0** `[census]` sections. Nothing
//! hash-pinned the five bundled PDFs to an irs.gov document, nothing could re-derive them, and no
//! gate asked whether the TY2017 maps accounted for every field on the TY2017 pages. **TY2017 was
//! supported in fill, unsupported in evidence** — and this file is the instrument that measured it.
//!
//! ★★ **On 2026-09-06 the owner ruled "S9 drop"** (`design/ROADMAP_STATUS.md` §0a, `FOLLOWUPS.md`
//! FR-61): the TY2017 form package was deleted rather than evidenced. That is the OTHER legal way to
//! close a row of this matrix, and the one this file always named — see the paragraph on narrowing
//! below.
//!
//! ## Both axes are DERIVED, never hand-listed
//!
//! - **years** — read from `SUPPORTED_YEARS` itself; that is the point of the file.
//! - **forms** — the union of `forms/<year>/*.pdf` stems over every bundled year directory. A new
//!   form or a new year directory enters the matrix by existing, not by being remembered here.
//! - **the join to the archive is by CONTENT** — each bundled PDF is sha256'd and looked up in the
//!   `# sha256` line of the committed `design/forms/*/<stem>--<year>.pdf.txt` notes. That is what lets
//!   the check span the two spellings of the registry (`schedule_d` in the crate, `f1040sd` in the
//!   archive) **without a fifth hand-written stem table** — the very drift `CLAUDE.md` §B3 and the
//!   port report's R16 name. A bundled asset that no note describes has no provenance, full stop.
//!
//! ## The record is a RATCHET, and it is deliberately not green-by-narrowing
//!
//! 21 of the 37 bundled (year, form) cells were missing at least one supporting artifact when this
//! was written, 54 obligations unmet in total. Since then the four Form 4868 / Form 1040-V cells
//! (2026-09-06) landed complete on all eight and added no row, and the five TY2017 cells left the
//! same day with their package (S9). **Measured at that commit: 8 of the 36 bundled cells carry a
//! gap, 11 obligations unmet** — down from 13 cells / 36 obligations, i.e. exactly the five TY2017
//! rows and their 25. Those are recorded in [`KNOWN_GAPS`] **cell by cell, artifact by
//! artifact** — measured, never estimated. A gap that appears anywhere else fails. A gap that *closes*
//! also fails, with instructions to delete the line: an excuse register that may be edited in either
//! direction records nothing.
//!
//! ★ **Narrowing `SUPPORTED_YEARS` cannot make this file green — only deleting the PACKAGE can.**
//! Dropping 2017 from `SUPPORTED_YEARS` alone leaves its rows unmatched by a bundled cell (this file
//! fails) *and* makes `forms/2017/` a bundled-but-unsupported directory
//! ([`supported_years_and_bundled_year_directories_agree`] fails). The two legal exits are to record
//! the gap in the open, or to remove the year's files entirely — which is what S9 did on
//! 2026-09-06. Both are visible in the diff; neither is a quiet edit to a list.

use btctax_forms::line_set::LineSet;
use btctax_forms::testonly::{
    parses_into_its_schema, Form1040Map, Form1040VMap, Form4868Map, Form6251Map, Form6251ObbbaMap,
    Form8275Map, Form8283Map, Form8889Map, Form8949Map, Form8959Map, Form8960Map, Form8995AMap,
    Form8995Map, Schedule1Map, Schedule2Map, Schedule3Map, ScheduleAMap, ScheduleBMap,
    ScheduleCMap, ScheduleDMap, ScheduleSeMap,
};
use btctax_forms::{FormsError, SUPPORTED_YEARS};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------
// THE RECORD — measured 2026-09-05, shrink-only.
// ---------------------------------------------------------------------------------------------

/// The supporting artifacts a shipped (year, form) cell is expected to have. Every one of these is a
/// thing some *other* instrument in this repo reads; a cell missing one is a cell that instrument
/// cannot see.
///
/// | obligation | what it means | who reads it |
/// |---|---|---|
/// | `map` | `forms/<y>/<stem>.map.toml` sits beside the PDF | every filler |
/// | `wired` | the build script bound both files — `bundled::template` and `bundled::map_text` are `Some` (design r2 §5; before step 3 this was a grep of `src/` for the `include_*!` lines) | the build |
/// | `dispatch` | the form's `Map::for_year(<y>)` returns `Ok` | every caller |
/// | `note` | a committed `design/forms/<y>/…pdf.txt` whose `# sha256` IS this asset's | re-derivation |
/// | `manifest` | that sha256 appears in `design/forms/MANIFEST.json` | the provenance record |
/// | `extract` | a non-empty `design/forms/extract/<archive stem>.txt` | every citation assertion |
/// | `geometry` | `design/forms/geometry/<archive stem>.json` | the label reader / line→label joins |
/// | `census` | the map carries a `[census]` section | `field_census.rs`'s 100 % accounting gate |
const OBLIGATIONS: [&str; 8] = [
    "map", "wired", "dispatch", "note", "manifest", "extract", "geometry", "census",
];

/// ★ **The honest matrix.** Every bundled (year, form) cell that is missing at least one obligation,
/// with the exact set. Measured — `git log -1` the file that produced these, not a hand count.
///
/// Reading it: TY2024 is complete
/// but for Form 8283, whose bundled Rev. 12-2023 asset matches **no** committed note — its
/// `design/forms/extract/f8283--2024.txt` exists with nothing pinning it to the bytes we ship; TY2025
/// has 15 committed forms of which **10 are unreachable** (asset and map committed, never included,
/// never dispatched — the inverse defect: a *prepared* year refused) and 5 more that are reachable but
/// carry no `[census]`.
const KNOWN_GAPS: &[(i32, &str, &[&str])] = &[
    // The five TY2017 rows (13 cells / 36 obligations → 8 / 11 — the whole authority chain, 25
    // obligations) were removed 2026-09-06 when S9 dropped the TY2017 form package: the cells they
    // excused no longer exist. Counts MEASURED off this list at that commit, not derived.
    // ── TY2024 — the reference year; one asset stands outside the archive. ──────────────────────
    (2024, "f8283", &["note", "manifest", "extract", "geometry"]),
    // ── TY2025 — step 5 wired eight of the ten on 2026-09-05 and `f6251/2025` on 2026-09-11 (its
    //    own struct, `Form6251ObbbaMap`, for the 1a/1b split). ONE remains bound but not DISPATCHED:
    //    `f1040s1a`, which has no struct and under the owner's 2026-09-11 ruling will never need one
    //    for TY2025. Five more are dispatched but carry no `[census]`. "wired" left this record at
    //    step 3: the glob binds every file, so it can no longer be missing. ──────────────────────────
    (2025, "f1040", &["census"]),
    (2025, "f1040s1a", &["dispatch"]),
    (2025, "f8283", &["census"]),
    (2025, "f8949", &["census"]),
    (2025, "schedule_d", &["census"]),
    (2025, "schedule_se", &["census"]),
];

/// (year, form) cells the crate **resolves from another year's asset** — no PDF and no map of their
/// own on disk, yet `Map::for_year` returns `Ok`.
///
/// ★ Form 8275 is revision-versioned, not tax-year-versioned, so this aliasing is deliberate and
/// documented (`map.rs::Form8275Map::for_year`). Since design r2 step 3 the alias is
/// `bundled::periodic_template`: a BUNDLED year with no file of its own is served by the newest
/// bundled revision whose own row says `versioning = { periodic = … }` — no year list anywhere, so
/// `| 2026` is not an edit that exists. It is pinned here anyway, because an alias that nobody
/// records is indistinguishable from a year silently borrowing a neighbouring year's geometry,
/// which is the exact failure `Form6251Map`'s doc comment exists to describe — and because a NEW
/// bundled year with no 8275 of its own will appear here, loudly, the day it is added.
const KNOWN_ALIASES: &[(i32, &str)] = &[(2025, "f8275")]; // (2017, "f8275") dropped with the TY2017 package (S9, 2026-09-06)

/// Bundled year directories that `SUPPORTED_YEARS` does not list. Since spec 1099-DA T0 this is
/// exactly the `preparing` years with a record and no template (TY2026): `SUPPORTED_YEARS` is
/// `bundled::TEMPLATE_YEARS`, `BUNDLED_YEARS` also carries record-only years, and a year in the
/// second but not the first must be a `preparing` record with `forms_expected = []` — held by
/// `tests/year_record.rs` and `field_census.rs`, so this ratchet stays informational. What makes a new year loud now:
/// `build.rs` refuses a year directory without `YEAR.toml`; `bundled::tests::
/// bundled_years_are_the_year_directories` pins the year list to the directories; and
/// `tests/year_record.rs` holds the new year's declaration to its glob. Kept as the record of the
/// old rule and as the place a deliberately-unshipped year would be named.
const BUNDLED_BUT_NOT_SUPPORTED: &[i32] = &[2026]; // TY2026: a `preparing` record with forms_expected = [] (spec 1099-DA T0) — bundled, nothing to fill

/// How many forms each supported year bundles. Pinned so that **deleting** an asset is as loud as
/// adding one — a cell with no recorded gap vanishing from the matrix would otherwise be silent.
// 2026-09-06: 2024 17 → 19 and 2025 15 → 17 — the Form 4868 and Form 1040-V rows (spec 4868/1040-V
// T1). The `(2017, 5)` row was removed the same day: S9 dropped the TY2017 package (owner ruling).
const BUNDLED_FORMS_PER_YEAR: &[(i32, usize)] = &[(2024, 20), (2025, 18)];

/// Bundled stems for which this build ships **no map type at all**, so nothing can parse the
/// committed `*.map.toml`. Recorded rather than skipped (`CLAUDE.md`: *skipping is not passing*).
///
/// ★ `f1040s1a` — Schedule 1-A, created by Pub. L. 119-21 for TY2025. `forms/2025/f1040s1a.map.toml`
/// and its PDF are committed and 100 % censused, and `map.rs` has no `Schedule1AMap`: there is no type
/// to give a `for_year` arm to. Found by this file; **not fixed here** (it is a build task, not a
/// test's business).
const STEMS_WITH_NO_MAP_TYPE: &[&str] = &["f1040s1a"];

// ---------------------------------------------------------------------------------------------
// MEASUREMENT — every set below comes off the filesystem or out of the crate's own API.
// ---------------------------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn forms_root() -> PathBuf {
    crate_root().join("forms")
}

fn archive_root() -> PathBuf {
    crate_root()
        .join("../..")
        .canonicalize()
        .expect("workspace root resolves")
        .join("design/forms")
}

/// Every year directory under `forms/` — **derived from the filesystem**, never a hand-list and never
/// a range (`CLAUDE.md`: *enumerate the line set FROM the form*; the same rule applies to years).
fn bundled_years() -> Vec<i32> {
    let mut years: Vec<i32> = std::fs::read_dir(forms_root())
        .expect("crates/btctax-forms/forms/ must exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_string_lossy().parse::<i32>().ok())
        .collect();
    years.sort_unstable();
    assert!(
        !years.is_empty(),
        "no year directories under forms/ — the walk is broken, not the repo"
    );
    years
}

/// The union of every form stem committed in any bundled year.
fn bundled_stems() -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    for year in bundled_years() {
        for entry in std::fs::read_dir(forms_root().join(year.to_string()))
            .expect("year directory reads")
            .filter_map(Result::ok)
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".pdf") {
                stems.insert(stem.to_string());
            }
        }
    }
    assert!(
        !stems.is_empty(),
        "no bundled PDFs found — the walk is broken"
    );
    stems
}

/// Does this build resolve a MAP for `(stem, year)`? `None` means the build ships no map type for the
/// stem at all, which is a different fact from "this year is unsupported" and is recorded, not hidden.
///
/// ★ The `_ => None` arm is what makes a newly committed form visible: a stem on disk that reaches it
/// is caught by [`every_bundled_stem_is_probed_or_recorded`].
fn map_resolves(stem: &str, year: i32) -> Option<bool> {
    let ok = match stem {
        "f1040" => Form1040Map::for_year(year).is_ok(),
        "f1040s1" => Schedule1Map::for_year(year).is_ok(),
        "f1040s2" => Schedule2Map::for_year(year).is_ok(),
        "f1040s3" => Schedule3Map::for_year(year).is_ok(),
        "f1040sa" => ScheduleAMap::for_year(year).is_ok(),
        "f1040sb" => ScheduleBMap::for_year(year).is_ok(),
        "f1040sc" => ScheduleCMap::for_year(year).is_ok(),
        "f1040v" => Form1040VMap::for_year(year).is_ok(),
        "f4868" => Form4868Map::for_year(year).is_ok(),
        // ★★★ TWO STRUCTS SERVE THIS STEM, and asking only one is how this instrument went green
        //     without traversing the revision it should have been reporting on. `f6251/2025` is a
        //     DIFFERENT revision (Part I line 1 split into 1a/1b) with its own struct, so
        //     `Form6251Map::for_year(2025)` is `Err` by design — and this arm therefore recorded a
        //     `dispatch` gap for a map that dispatches perfectly well. The derived cross-check that
        //     keeps this arm honest is `the_per_stem_dispatch_agrees_with_the_derived_dispatch`
        //     below; a third revision-struct appearing without being added here reds there.
        "f6251" => Form6251Map::for_year(year).is_ok() || Form6251ObbbaMap::for_year(year).is_ok(),
        "f8275" => Form8275Map::for_year(year).is_ok(),
        "f8283" => Form8283Map::for_year(year).is_ok(),
        "f8949" => Form8949Map::for_year(year).is_ok(),
        "f8889" => Form8889Map::for_year(year).is_ok(),
        "f8959" => Form8959Map::for_year(year).is_ok(),
        "f8960" => Form8960Map::for_year(year).is_ok(),
        "f8995" => Form8995Map::for_year(year).is_ok(),
        "f8995a" => Form8995AMap::for_year(year).is_ok(),
        "schedule_d" => ScheduleDMap::for_year(year).is_ok(),
        "schedule_se" => ScheduleSeMap::for_year(year).is_ok(),
        _ => return None,
    };
    Some(ok)
}

/// sha256 → archive stem (`f1040sd--2024`), read from the committed provenance notes. This is the
/// content join: no filename convention, no stem translation table.
fn note_index() -> BTreeMap<String, String> {
    let mut index = BTreeMap::new();
    let mut notes_seen = 0usize;
    for dir in std::fs::read_dir(archive_root())
        .expect("design/forms/ must exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
    {
        for entry in std::fs::read_dir(dir.path())
            .expect("archive year directory reads")
            .filter_map(Result::ok)
        {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(stem) = name.strip_suffix(".pdf.txt") else {
                continue;
            };
            notes_seen += 1;
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            for line in text.lines() {
                let t = line.trim();
                let Some(rest) = t.strip_prefix("# sha256") else {
                    continue;
                };
                let sha = rest.trim();
                if sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    index.insert(sha.to_string(), stem.to_string());
                }
            }
        }
    }
    assert!(
        notes_seen > 0 && !index.is_empty(),
        "read {notes_seen} note file(s) and parsed {} sha256 line(s) — the JOIN is broken, which \
         would report every asset as unprovenanced. Fix the reader, do not record the gaps.",
        index.len()
    );
    index
}

fn manifest_text() -> String {
    let text =
        std::fs::read_to_string(archive_root().join("MANIFEST.json")).expect("MANIFEST.json reads");
    assert!(text.len() > 100, "MANIFEST.json is empty or truncated");
    text
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CellState {
    /// No asset of its own, and the crate refuses the year. The honest fail-closed state.
    Absent,
    /// No asset of its own, and yet `Map::for_year` returns `Ok` — another year's asset, restamped.
    Aliased,
    /// The year bundles this form. Carries the obligations it is MISSING (empty = fully supported).
    Bundled(BTreeSet<&'static str>),
}

/// The whole matrix: `SUPPORTED_YEARS` × every bundled stem.
fn measure() -> BTreeMap<(i32, String), CellState> {
    let notes = note_index();
    let manifest = manifest_text();
    // Liveness (steps-2/3 review Q6): the `wired` reader consults the generated bindings; a build
    // that bound nothing would measure every cell unwired and this file would report a defect that
    // is really a blind instrument. The old reader asserted `include_bytes!` appeared in src/.
    // 41 → 36 on 2026-09-06: S9 dropped the five TY2017 (pdf, map) pairs.
    assert!(
        btctax_forms::bundled::BUNDLED.len() >= 36,
        "the generated bindings hold {} pairs — the reader has gone blind",
        btctax_forms::bundled::BUNDLED.len()
    );
    let mut matrix = BTreeMap::new();

    for &year in SUPPORTED_YEARS {
        for stem in bundled_stems() {
            let dir = forms_root().join(year.to_string());
            let pdf = dir.join(format!("{stem}.pdf"));
            let map_toml = dir.join(format!("{stem}.map.toml"));
            let dispatch = map_resolves(&stem, year).unwrap_or(false);

            if !pdf.exists() {
                let state = if dispatch {
                    CellState::Aliased
                } else {
                    CellState::Absent
                };
                matrix.insert((year, stem), state);
                continue;
            }

            let bytes = std::fs::read(&pdf).expect("bundled PDF reads");
            let sha = format!("{:x}", Sha256::digest(&bytes));
            let archive_stem = notes.get(&sha);

            let mut have: BTreeMap<&'static str, bool> = BTreeMap::new();
            have.insert("map", map_toml.exists());
            // ★ Since design r2 step 3 the binding IS the glob: a file on disk is bound by build.rs
            //   or the build fails. `wired` is therefore measured from the generated bindings, and
            //   the interesting obligation a bundled-but-unparsed map still lacks is `dispatch`.
            have.insert(
                "wired",
                btctax_forms::bundled::Stem::from_file_stem(&stem).is_some_and(|st| {
                    btctax_forms::bundled::template(st, year).is_some()
                        && btctax_forms::bundled::map_text(st, year).is_some()
                }),
            );
            have.insert("dispatch", dispatch);
            have.insert("note", archive_stem.is_some());
            have.insert("manifest", manifest.contains(&sha));
            have.insert(
                "extract",
                archive_stem.is_some_and(|a| {
                    std::fs::metadata(archive_root().join(format!("extract/{a}.txt")))
                        .is_ok_and(|m| m.len() > 0)
                }),
            );
            have.insert(
                "geometry",
                archive_stem.is_some_and(|a| {
                    std::fs::metadata(archive_root().join(format!("geometry/{a}.json")))
                        .is_ok_and(|m| m.len() > 0)
                }),
            );
            have.insert(
                "census",
                std::fs::read_to_string(&map_toml)
                    .map(|t| t.lines().any(|l| l.trim() == "[census]"))
                    .unwrap_or(false),
            );

            let missing: BTreeSet<&'static str> = OBLIGATIONS
                .iter()
                .copied()
                .filter(|o| !have[o])
                .collect::<BTreeSet<_>>();
            assert_eq!(
                have.len(),
                OBLIGATIONS.len(),
                "an obligation was named but never measured"
            );
            matrix.insert((year, stem), CellState::Bundled(missing));
        }
    }
    matrix
}

// ---------------------------------------------------------------------------------------------
// THE VERDICTS — pure functions over the measurement, so the planted defects need no repo edit.
// ---------------------------------------------------------------------------------------------

fn year_verdict(supported: &[i32], bundled: &[i32], recorded: &[i32]) -> Result<(), String> {
    let bundled_set: BTreeSet<i32> = bundled.iter().copied().collect();
    let supported_set: BTreeSet<i32> = supported.iter().copied().collect();

    let claimed_without_assets: Vec<i32> =
        supported_set.difference(&bundled_set).copied().collect();
    if !claimed_without_assets.is_empty() {
        return Err(format!(
            "SUPPORTED_YEARS lists {claimed_without_assets:?}, and forms/ has no directory for \
             them. The constant is a warranty read by `UnsupportedYear`'s message and by every \
             caller that branches on it: a listed year with no bundled asset refuses every fill \
             while claiming to be supported."
        ));
    }

    let unclaimed: Vec<i32> = bundled_set
        .difference(&supported_set)
        .copied()
        .filter(|y| !recorded.contains(y))
        .collect();
    if !unclaimed.is_empty() {
        return Err(format!(
            "forms/ bundles {unclaimed:?}, which SUPPORTED_YEARS does not list and \
             BUNDLED_BUT_NOT_SUPPORTED does not record. A year's assets must not land silently: \
             either ship the year, or record it as prepared-but-not-shipped and say why."
        ));
    }

    let stale: Vec<i32> = recorded
        .iter()
        .copied()
        .filter(|y| !bundled_set.contains(y) || supported_set.contains(y))
        .collect();
    if !stale.is_empty() {
        return Err(format!(
            "BUNDLED_BUT_NOT_SUPPORTED still records {stale:?}, which is now either shipped or \
             gone. Delete the line — a record that survives its own reason is how a closed gap \
             reopens."
        ));
    }
    Ok(())
}

fn count_verdict(
    matrix: &BTreeMap<(i32, String), CellState>,
    pinned: &[(i32, usize)],
) -> Result<(), String> {
    let mut measured: BTreeMap<i32, usize> = BTreeMap::new();
    for ((year, _), state) in matrix {
        if matches!(state, CellState::Bundled(_)) {
            *measured.entry(*year).or_default() += 1;
        }
    }
    let pinned_map: BTreeMap<i32, usize> = pinned.iter().copied().collect();
    if measured != pinned_map {
        return Err(format!(
            "bundled forms per year measured {measured:?}, recorded {pinned_map:?}. A form gained \
             or LOST its bundled asset — the loss is the dangerous direction, and it is invisible \
             in a gap register that only lists what is missing."
        ));
    }
    Ok(())
}

fn matrix_verdict(
    matrix: &BTreeMap<(i32, String), CellState>,
    known_gaps: &[(i32, &str, &[&str])],
    known_aliases: &[(i32, &str)],
) -> Result<(), String> {
    let recorded: BTreeMap<(i32, String), BTreeSet<&str>> = known_gaps
        .iter()
        .map(|(y, s, g)| ((*y, (*s).to_string()), g.iter().copied().collect()))
        .collect();
    if recorded.len() != known_gaps.len() {
        return Err("KNOWN_GAPS names the same (year, form) twice".to_string());
    }
    for (_, _, gaps) in known_gaps {
        if let Some(bad) = gaps.iter().find(|g| !OBLIGATIONS.contains(g)) {
            return Err(format!(
                "KNOWN_GAPS records {bad:?}, which is not one of the obligations {OBLIGATIONS:?} — \
                 a misspelt excuse excuses nothing and hides the real gap"
            ));
        }
    }

    let aliases: BTreeSet<(i32, String)> = known_aliases
        .iter()
        .map(|(y, s)| (*y, (*s).to_string()))
        .collect();

    let mut errors: Vec<String> = Vec::new();

    for (cell, state) in matrix {
        let (year, stem) = cell;
        match state {
            CellState::Bundled(missing) => {
                let recorded_gaps = recorded.get(cell);
                match (missing.is_empty(), recorded_gaps) {
                    (true, None) => {}
                    (true, Some(g)) => errors.push(format!(
                        "({year}, {stem}) is recorded as missing {g:?} and is now complete. The \
                         register is SHRINK-ONLY: delete the line."
                    )),
                    (false, None) => errors.push(format!(
                        "({year}, {stem}) is missing {missing:?} and NOTHING records it. This is \
                         the defect the file exists for: a shipped form whose provenance, extract, \
                         geometry or census is simply absent, invisible on the emitted page."
                    )),
                    (false, Some(g)) => {
                        let g: BTreeSet<&str> = g.iter().copied().collect();
                        let m: BTreeSet<&str> = missing.iter().copied().collect();
                        if g != m {
                            let new: Vec<&&str> = m.difference(&g).collect();
                            let closed: Vec<&&str> = g.difference(&m).collect();
                            errors.push(format!(
                                "({year}, {stem}) records {g:?} but measures {m:?} — newly \
                                 missing {new:?}, newly present {closed:?}. Record it exactly or \
                                 close it; never widen the excuse."
                            ));
                        }
                    }
                }
            }
            CellState::Aliased => {
                if !aliases.contains(cell) {
                    errors.push(format!(
                        "({year}, {stem}) bundles NO asset of its own, yet the crate resolves it — \
                         another year's map, restamped with this year. Unrecorded aliasing is how a \
                         year silently inherits a neighbouring year's geometry."
                    ));
                }
            }
            CellState::Absent => {
                if aliases.contains(cell) {
                    errors.push(format!(
                        "({year}, {stem}) is recorded as an alias and now refuses. Delete the line."
                    ));
                }
                if recorded.contains_key(cell) {
                    errors.push(format!(
                        "KNOWN_GAPS records ({year}, {stem}), which this build does not bundle at \
                         all. A gap row must name a shipped cell — otherwise removing the year \
                         from SUPPORTED_YEARS would quietly satisfy the register."
                    ));
                }
            }
        }
    }

    for cell in recorded.keys() {
        if !matrix.contains_key(cell) {
            errors.push(format!(
                "KNOWN_GAPS records {cell:?}, which is not in the matrix at all — the year is no \
                 longer in SUPPORTED_YEARS, or the form is no longer bundled anywhere. Removing a \
                 year does not discharge its gaps."
            ));
        }
    }
    for cell in &aliases {
        if !matrix.contains_key(cell) {
            errors.push(format!(
                "KNOWN_ALIASES records {cell:?}, which is not in the matrix at all."
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// The matrix, rendered — printed on every failure so the reader sees the state, not just the delta.
fn render(matrix: &BTreeMap<(i32, String), CellState>) -> String {
    let mut out = format!("\n{:<6}{:<14}{}\n", "year", "form", "state");
    for ((year, stem), state) in matrix {
        let s = match state {
            CellState::Absent => "absent (refuses)".to_string(),
            CellState::Aliased => "ALIASED to another year's asset".to_string(),
            CellState::Bundled(m) if m.is_empty() => "bundled, complete".to_string(),
            CellState::Bundled(m) => format!("bundled, MISSING {m:?}"),
        };
        out.push_str(&format!("{year:<6}{stem:<14}{s}\n"));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// THE TESTS
// ---------------------------------------------------------------------------------------------

#[test]
fn supported_years_and_bundled_year_directories_agree() {
    let bundled = bundled_years();
    if let Err(e) = year_verdict(SUPPORTED_YEARS, &bundled, BUNDLED_BUT_NOT_SUPPORTED) {
        panic!("{e}\n\nSUPPORTED_YEARS = {SUPPORTED_YEARS:?}\nforms/ = {bundled:?}");
    }
}

#[test]
fn every_bundled_stem_is_probed_or_recorded() {
    let unprobed: BTreeSet<String> = bundled_stems()
        .into_iter()
        .filter(|s| map_resolves(s, 2024).is_none())
        .collect();
    let recorded: BTreeSet<String> = STEMS_WITH_NO_MAP_TYPE
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        unprobed, recorded,
        "a bundled form reaches the `_ => None` arm of `map_resolves` without being recorded in \
         STEMS_WITH_NO_MAP_TYPE (or a recorded stem now has a map type). Every committed form must \
         be measured or named; an unmeasured one is counted as fine by this whole file."
    );
}

/// ★★★ **THE GATE.** `SUPPORTED_YEARS` × every bundled form, against the recorded state.
#[test]
fn the_cross_product_matrix_matches_the_recorded_gaps() {
    let matrix = measure();
    if let Err(e) = matrix_verdict(&matrix, KNOWN_GAPS, KNOWN_ALIASES) {
        panic!("{e}\n{}", render(&matrix));
    }
    if let Err(e) = count_verdict(&matrix, BUNDLED_FORMS_PER_YEAR) {
        panic!("{e}\n{}", render(&matrix));
    }

    // The register may only shrink, and its size is the headline number: say it out loud so a
    // shrinking run is visible in the log rather than only in a diff.
    let gapped = matrix
        .values()
        .filter(|s| matches!(s, CellState::Bundled(m) if !m.is_empty()))
        .count();
    let unmet: usize = matrix
        .values()
        .map(|s| match s {
            CellState::Bundled(m) => m.len(),
            _ => 0,
        })
        .sum();
    println!(
        "{} of {} bundled (year, form) cells carry a gap; {unmet} obligations unmet",
        gapped,
        matrix
            .values()
            .filter(|s| matches!(s, CellState::Bundled(_)))
            .count()
    );
}

/// Every committed `*.map.toml` has the PDF it maps beside it. A map without its page is a cell the
/// matrix would classify as absent while the file sits there looking prepared.
#[test]
fn every_committed_map_has_its_bundled_pdf() {
    let mut orphans = Vec::new();
    for year in bundled_years() {
        let dir = forms_root().join(year.to_string());
        for entry in std::fs::read_dir(&dir)
            .expect("year directory reads")
            .filter_map(Result::ok)
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".map.toml") {
                if !dir.join(format!("{stem}.pdf")).exists() {
                    orphans.push(format!("{year}/{stem}"));
                }
            }
        }
    }
    assert!(
        orphans.is_empty(),
        "committed maps with no bundled PDF: {orphans:?}"
    );
}

/// The refusal a filer actually reads must name the years this build actually ships.
///
/// ★ Before this, the sentence in `error.rs` was a literal 57 lines below the constant and **no test
/// red when `SUPPORTED_YEARS` changed** — the same shape as the six stale refusal descriptions folded
/// in `3d01b5e3`. The requested year is rendered as `-1` precisely so the only four-digit numbers left
/// in the message are the claim itself.
#[test]
fn the_unsupported_year_refusal_names_exactly_the_supported_years() {
    let msg = FormsError::UnsupportedYear(-1).to_string();

    for year in SUPPORTED_YEARS {
        assert!(
            msg.contains(&year.to_string()),
            "SUPPORTED_YEARS lists {year} and the refusal a filer sees does not mention it: {msg:?}"
        );
    }

    let mut claimed: BTreeSet<i32> = BTreeSet::new();
    let digits: Vec<char> = msg.chars().collect();
    for i in 0..digits.len() {
        if digits[i..].len() >= 4 && digits[i..i + 4].iter().all(char::is_ascii_digit) {
            let before_ok = i == 0 || !digits[i - 1].is_ascii_digit();
            let after_ok = i + 4 == digits.len() || !digits[i + 4].is_ascii_digit();
            if before_ok && after_ok {
                claimed.insert(digits[i..i + 4].iter().collect::<String>().parse().unwrap());
            }
        }
    }
    let supported: BTreeSet<i32> = SUPPORTED_YEARS.iter().copied().collect();
    assert_eq!(
        claimed, supported,
        "the UnsupportedYear message names {claimed:?} and this build supports {supported:?}. The \
         refusal fires correctly and names the wrong year: {msg:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// B1 — seen-red-once. Every verdict above, watched rejecting the exact defect it exists to catch.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_gate_reds_on_every_planted_defect() {
    let cell = |y: i32, s: &str| (y, s.to_string());
    let bundled = |gaps: &[&'static str]| CellState::Bundled(gaps.iter().copied().collect());

    // — year_verdict —
    // A year LISTED as supported with no assets behind it (adding 2026 to the constant first).
    assert!(
        year_verdict(&[2024, 2026], &[2024], &[]).is_err(),
        "a supported year with no forms/<year>/ must fail"
    );
    // A year whose assets landed but which nothing lists (committing forms/2026/ first).
    assert!(
        year_verdict(&[2024], &[2024, 2026], &[]).is_err(),
        "a bundled year absent from SUPPORTED_YEARS must fail"
    );
    // …and recording it is the only way through.
    assert!(year_verdict(&[2024], &[2024, 2026], &[2026]).is_ok());
    // A record that outlived its reason.
    assert!(
        year_verdict(&[2024, 2026], &[2024, 2026], &[2026]).is_err(),
        "a shipped year still recorded as unsupported must fail"
    );
    assert!(year_verdict(&[2024], &[2024], &[]).is_ok());

    // — matrix_verdict —
    let complete: BTreeMap<(i32, String), CellState> =
        [(cell(2024, "f1040"), bundled(&[]))].into_iter().collect();
    assert!(matrix_verdict(&complete, &[], &[]).is_ok());

    // A NEW gap on a cell nothing records — the headline defect.
    let regressed: BTreeMap<(i32, String), CellState> =
        [(cell(2024, "f1040"), bundled(&["census"]))]
            .into_iter()
            .collect();
    let err = matrix_verdict(&regressed, &[], &[]).unwrap_err();
    assert!(
        err.contains("census") && err.contains("NOTHING records it"),
        "{err}"
    );

    // The ratchet accepts it once recorded, and ONLY at that exact set.
    assert!(matrix_verdict(&regressed, &[(2024, "f1040", &["census"])], &[]).is_ok());
    assert!(
        matrix_verdict(&regressed, &[(2024, "f1040", &["census", "note"])], &[]).is_err(),
        "a wider excuse than the measurement must fail"
    );
    let wider: BTreeMap<(i32, String), CellState> = [(
        cell(2024, "f1040"),
        bundled(&["census", "note", "geometry"]),
    )]
    .into_iter()
    .collect();
    assert!(
        matrix_verdict(&wider, &[(2024, "f1040", &["census"])], &[]).is_err(),
        "a gap growing beyond its record must fail"
    );

    // A gap that CLOSED must also fail, so the record cannot rot into a false description.
    assert!(
        matrix_verdict(&complete, &[(2024, "f1040", &["census"])], &[]).is_err(),
        "a closed gap still on the register must fail"
    );

    // A misspelt obligation excuses nothing.
    assert!(
        matrix_verdict(&regressed, &[(2024, "f1040", &["cenus"])], &[]).is_err(),
        "an obligation name that is not in OBLIGATIONS must fail"
    );

    // Aliasing: unrecorded fails, recorded passes, stale record fails.
    let aliased: BTreeMap<(i32, String), CellState> = [(cell(2026, "f8275"), CellState::Aliased)]
        .into_iter()
        .collect();
    assert!(
        matrix_verdict(&aliased, &[], &[]).is_err(),
        "an unrecorded alias must fail — this is the `| 2026` one-token edit"
    );
    assert!(matrix_verdict(&aliased, &[], &[(2026, "f8275")]).is_ok());
    let refuses: BTreeMap<(i32, String), CellState> = [(cell(2026, "f8275"), CellState::Absent)]
        .into_iter()
        .collect();
    assert!(
        matrix_verdict(&refuses, &[], &[(2026, "f8275")]).is_err(),
        "an alias record that no longer aliases must fail"
    );

    // ★ The anti-dodge: dropping a year from SUPPORTED_YEARS must NOT discharge its gaps.
    let dropped: BTreeMap<(i32, String), CellState> =
        [(cell(2024, "f1040"), bundled(&[]))].into_iter().collect();
    let err = matrix_verdict(
        &dropped,
        &[(
            2017,
            "f8949",
            &["note", "manifest", "extract", "geometry", "census"],
        )],
        &[],
    )
    .unwrap_err();
    assert!(err.contains("Removing a year does not discharge"), "{err}");

    // A recorded gap pointing at a cell the build refuses.
    let absent: BTreeMap<(i32, String), CellState> = [(cell(2017, "f6251"), CellState::Absent)]
        .into_iter()
        .collect();
    assert!(
        matrix_verdict(&absent, &[(2017, "f6251", &["census"])], &[]).is_err(),
        "a gap row naming an unbundled cell must fail"
    );

    // — count_verdict — a deleted asset is as loud as a new one.
    let two: BTreeMap<(i32, String), CellState> = [
        (cell(2024, "f1040"), bundled(&[])),
        (cell(2024, "f8949"), bundled(&[])),
    ]
    .into_iter()
    .collect();
    assert!(count_verdict(&two, &[(2024, 2)]).is_ok());
    assert!(
        count_verdict(&two, &[(2024, 3)]).is_err(),
        "a lost bundled asset must fail"
    );
    assert!(
        count_verdict(&two, &[(2024, 1)]).is_err(),
        "an unrecorded new bundled asset must fail"
    );
}

/// ★★★ **The per-stem dispatch table agrees with the DERIVED one** — the guard on `map_resolves`.
///
/// `map_resolves` is a hand-written `match` from stem to *one* `Map::for_year`, and it is the reason
/// this file recorded a `dispatch` gap for `f6251/2025` right up to the day that revision was wired:
/// the stem had acquired a **second** struct and the arm still asked the first one. A recorded gap
/// that is not a gap is the quietest kind of wrong instrument — it reports a defect that does not
/// exist and, symmetrically, would hide one that does.
///
/// So the arm is cross-checked against `btctax_forms::testonly::parses_into_its_schema`, which is
/// exhaustive over `Schema` and therefore cannot fall behind a new struct: **if a revision's own row
/// names a struct and that struct accepts its bundled map, the stem must dispatch.** A third Form 6251
/// revision-struct added without extending the arm reds here.
///
/// ★ Scoped to revisions whose name is `<stem>/<year>` for a stem this file probes — which is every
/// committed row (`map_rows.rs` holds that convention). Form 8275's periodic ALIAS is deliberately not
/// reached: it has no `f8275/2025` revision at all, which is why `KNOWN_ALIASES` exists.
#[test]
fn the_per_stem_dispatch_agrees_with_the_derived_dispatch() {
    let mut checked = 0usize;
    for ls in LineSet::ALL {
        let (stem, year) = ls.as_str().split_once('/').expect("<stem>/<year>");
        let year: i32 = year.parse().expect("a year");
        let derived = parses_into_its_schema(*ls);
        let per_stem = map_resolves(stem, year);
        match derived {
            Some(Ok(())) => {
                assert_eq!(
                    per_stem,
                    Some(true),
                    "{}: its own row names a struct that ACCEPTS its bundled map, but \
                     `map_resolves` says {per_stem:?}. The stem's arm is asking a struct that is \
                     not this revision's — which records a `dispatch` gap for a map that dispatches.",
                    ls.as_str()
                );
                checked += 1;
            }
            Some(Err(e)) => panic!(
                "{}: the struct its row names REFUSES its own bundled map: {e}",
                ls.as_str()
            ),
            // No struct named (`Unwired`). It must NOT dispatch, or the record is fiction the other
            // way round.
            None => {
                assert_ne!(
                    per_stem,
                    Some(true),
                    "{}: no struct parses this revision, yet `map_resolves` reports it dispatching",
                    ls.as_str()
                );
            }
        }
    }
    assert!(
        checked >= 37,
        "only {checked} revisions were cross-checked — the reader has gone blind and this test is \
         vacuously green"
    );
}
