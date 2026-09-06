//! ★★ **Design r2 §10 step 1 — the ROW of the year-package table, and its four self-contained kills.**
//!
//! Every `forms/<year>/<stem>.map.toml` IS a row (design r2 §4); the row SET is the glob, never a
//! list. This file walks that glob and holds, for every committed map:
//!
//! 1. the row parses with its required keys (and a map missing one is REFUSED — kill 1);
//! 2. `template_sha256` is the sha256 of the bundled PDF beside it (kill 2);
//! 3. that hash joins `design/forms/MANIFEST.json` on an AUTHORITY entry — or the row carries
//!    `authority = "not-yet-archived: …"`, the ONLY excuse, on exactly the six rows the design names
//!    and no seventh (kill 3);
//! 4. `attachment_sequence` equals the "Attachment Sequence No." printed on the archived extract,
//!    absent exactly on the 1040 (kill 4) — and `btctax_forms::attachment_sequence` (the packet's
//!    stapling order) agrees with every row, which is how the Form 8283 155→36 renumber was found.
//!
//! Every kill was observed RED on a planted defect before it was trusted (B1): each `check_*` helper
//! takes a root, and the `a_planted_*` tests build a tempdir copy with exactly one defect.
use btctax_forms::{AnnualTag, MapRow, Versioning};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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

/// Every `(year, stem, map path)` under `<root>/<year>/`. Derived, never hand-listed.
fn every_map(forms_root: &Path) -> Vec<(i32, String, PathBuf)> {
    let mut out = Vec::new();
    let mut years: Vec<PathBuf> = std::fs::read_dir(forms_root)
        .expect("forms root")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    years.sort();
    for y in years {
        let year: i32 = y
            .file_name()
            .unwrap()
            .to_string_lossy()
            .parse()
            .expect("year dir");
        let mut maps: Vec<PathBuf> = std::fs::read_dir(&y)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().ends_with(".map.toml"))
            .collect();
        maps.sort();
        for m in maps {
            let stem = m
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches(".map.toml")
                .to_string();
            out.push((year, stem, m));
        }
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// The manifest's AUTHORITY hashes: every entry whose path is not a `-DRAFT` and whose URL is not
/// under `irs-dft`. Read from the committed JSON, never re-derived.
fn manifest_authority_hashes(workspace: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(workspace.join("design/forms/MANIFEST.json"))
        .expect("design/forms/MANIFEST.json");
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let entries = v
        .get("entries")
        .and_then(|e| e.as_array())
        .cloned()
        .or_else(|| v.as_array().cloned())
        .expect("manifest entries");
    entries
        .iter()
        .filter(|e| {
            let path = e.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let url = e.get("url").and_then(|p| p.as_str()).unwrap_or("");
            !path.contains("-DRAFT") && !url.contains("/irs-dft/")
        })
        .filter_map(|e| e.get("sha256").and_then(|s| s.as_str()).map(String::from))
        .collect()
}

/// The "Attachment Sequence No." a text layer prints, or `None` when it prints none.
fn printed_sequence(extract: &str) -> Option<String> {
    for line in extract.lines() {
        if let Some(i) = line.find("Sequence No") {
            let rest = &line[i + "Sequence No".len()..];
            let tok: String = rest
                .trim_start_matches(['.', ' '])
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if !tok.is_empty() && tok.chars().next().unwrap().is_ascii_digit() {
                return Some(tok);
            }
        }
    }
    None
}

#[derive(Debug, PartialEq, Eq)]
enum RowProblem {
    Unparseable {
        year: i32,
        stem: String,
        why: String,
    },
    HashMismatch {
        year: i32,
        stem: String,
    },
    NotInManifest {
        year: i32,
        stem: String,
    },
    SequenceMismatch {
        year: i32,
        stem: String,
        row: Option<String>,
        printed: Option<String>,
    },
}

/// The four checks, over any forms root (so a test can plant a defect in a tempdir copy).
fn check_rows(
    forms_root: &Path,
    authority_hashes: &BTreeSet<String>,
    extract_root: &Path,
) -> (Vec<MapRow>, Vec<RowProblem>) {
    let mut rows = Vec::new();
    let mut problems = Vec::new();
    for (year, stem, map) in every_map(forms_root) {
        let text = std::fs::read_to_string(&map).unwrap();
        let row = match MapRow::read(&text) {
            Ok(r) => r,
            Err(e) => {
                problems.push(RowProblem::Unparseable {
                    year,
                    stem,
                    why: e.to_string(),
                });
                continue;
            }
        };
        assert_eq!(
            row.form, stem,
            "{year}/{stem}: the row's `form` must be the file's stem"
        );
        assert_eq!(
            row.year, year,
            "{year}/{stem}: the row's `year` must be its directory"
        );
        // kill 2 — the hash IS the bundled PDF's.
        let pdf = std::fs::read(map.with_extension("").with_extension("pdf")).unwrap();
        if sha256_hex(&pdf) != row.template_sha256 {
            problems.push(RowProblem::HashMismatch {
                year,
                stem: stem.clone(),
            });
        }
        // kill 3 — the manifest join, excused ONLY by `authority`.
        if row.authority.is_none() && !authority_hashes.contains(&row.template_sha256) {
            problems.push(RowProblem::NotInManifest {
                year,
                stem: stem.clone(),
            });
        }
        // kill 4 — the printed sequence number, from the archived extract.
        let extract = extract_root.join(format!("{}--{}.txt", row.irs_stem, year));
        if row.authority.is_none() {
            let printed = std::fs::read_to_string(&extract)
                .ok()
                .and_then(|t| printed_sequence(&t));
            if printed != row.attachment_sequence {
                problems.push(RowProblem::SequenceMismatch {
                    year,
                    stem: stem.clone(),
                    row: row.attachment_sequence.clone(),
                    printed,
                });
            }
        }
        rows.push(row);
    }
    (rows, problems)
}

/// The six rows the design names (r2 §4, §10 step 1) — a shrink-only pin. A seventh excuse reds;
/// removing one means an authority was archived and the row's `authority` key must go.
const EXCUSED: &[(i32, &str)] = &[
    (2017, "f1040"),
    (2017, "f8283"),
    (2017, "f8949"),
    (2017, "schedule_d"),
    (2017, "schedule_se"),
    (2024, "f8283"),
];

#[test]
fn every_committed_map_has_a_row_that_parses_and_all_four_kills_are_green() {
    let ws = workspace_root();
    let (rows, problems) = check_rows(
        &crate_root().join("forms"),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    assert!(rows.len() >= 37, "the walk found only {} rows", rows.len());
    assert!(problems.is_empty(), "row problems:\n  {:#?}", problems);
    // The excuse set is exactly the six, no more, no fewer.
    let excused: BTreeSet<(i32, String)> = rows
        .iter()
        .filter(|r| r.authority.is_some())
        .map(|r| (r.year, r.form.clone()))
        .collect();
    let expected: BTreeSet<(i32, String)> =
        EXCUSED.iter().map(|(y, s)| (*y, s.to_string())).collect();
    assert_eq!(
        excused, expected,
        "the `authority = \"not-yet-archived\"` excuse may only SHRINK, and only these six carry it"
    );
    for r in &rows {
        assert!(
            r.authority
                .as_deref()
                .is_none_or(|a| a.starts_with("not-yet-archived: ")),
            "{}/{}: `authority` must read `not-yet-archived: <reason>`",
            r.year,
            r.form
        );
        // line_set shape, and the 1040 exception.
        assert_eq!(
            r.line_set,
            format!("{}/{}", r.form, r.year),
            "{}/{}: step 1 writes `<stem>/<year>`; collapsing revisions is later work",
            r.year,
            r.form
        );
        assert_eq!(
            r.attachment_sequence.is_none(),
            r.form == "f1040",
            "{}/{}: only the 1040 carries no sequence number",
            r.year,
            r.form
        );
        // irs_stem: the two aliases and nothing else.
        let expected_irs = match r.form.as_str() {
            "schedule_d" => "f1040sd",
            "schedule_se" => "f1040sse",
            other => other,
        };
        assert_eq!(r.irs_stem, expected_irs, "{}/{}", r.year, r.form);
        // versioning: periodic exactly where the form prints a revision date.
        let periodic = matches!(r.versioning, Versioning::Periodic { .. });
        assert_eq!(
            periodic,
            matches!(r.form.as_str(), "f8275" | "f8283"),
            "{}/{}: only Forms 8275 and 8283 are revision-dated",
            r.year,
            r.form
        );
        if !periodic {
            assert_eq!(r.versioning, Versioning::Annual(AnnualTag::Annual));
        }
    }
    // extract_override: only the one row with a second extract root.
    let overrides: Vec<_> = rows
        .iter()
        .filter(|r| r.extract_override.is_some())
        .map(|r| (r.year, r.form.as_str()))
        .collect();
    assert_eq!(overrides, vec![(2025, "f1040s1a")]);
    let pages: Vec<_> = rows
        .iter()
        .filter(|r| r.instr_pages.is_some())
        .map(|r| (r.year, r.form.as_str(), r.instr_pages.unwrap()))
        .collect();
    assert_eq!(pages, vec![(2025, "f1040s1a", [101, 110])]);
}

/// ★ The consumer-side kill: the packet's stapling order (`packet.rs`) must agree with the form's
/// own printed number on EVERY row. This is what caught Form 8283 (155 on Rev. 12-2023, 36 on
/// Rev. 12-2025) being pushed as "155" for TY2025.
#[test]
fn packet_sequences_agree_with_every_map_row() {
    let ws = workspace_root();
    let (rows, _) = check_rows(
        &crate_root().join("forms"),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    let mut drift = Vec::new();
    for r in &rows {
        let packet = btctax_forms::attachment_sequence(&r.form, r.year).map(String::from);
        if packet != r.attachment_sequence {
            drift.push(format!(
                "{}/{}: packet says {:?}, the form prints {:?}",
                r.year, r.form, packet, r.attachment_sequence
            ));
        }
    }
    assert!(
        drift.is_empty(),
        "stapling-order drift:\n  {}",
        drift.join("\n  ")
    );
}

/// B1 — kill 1: a row missing a REQUIRED key is refused by the typed struct (deny_unknown_fields is
/// on, and `line_set` has no default), and by the projection reader.
#[test]
fn a_map_missing_a_required_row_key_is_refused() {
    let text = std::fs::read_to_string(crate_root().join("forms/2024/f8959.map.toml")).unwrap();
    let without: String = text
        .lines()
        .filter(|l| !l.starts_with("line_set"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        btctax_forms::testonly::Form8959Map::parse(&without).is_err(),
        "the typed parse must REFUSE a map without `line_set`"
    );
    assert!(
        MapRow::read(&without).is_err(),
        "the row reader must refuse it too"
    );
    // …and an unknown top-level key is refused by the typed parse (the row reader ignores it — it
    // has to, the bindings are unknown to it).
    let with_typo = text.replace("line_set", "line_sett");
    assert!(btctax_forms::testonly::Form8959Map::parse(&with_typo).is_err());
}

/// Build a tempdir copy of one (year, stem) so a defect can be planted without touching the tree.
fn plant(year: i32, stem: &str, edit: impl Fn(String) -> String) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = crate_root().join(format!("forms/{year}"));
    let dst = dir.path().join(format!("{year}"));
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::copy(
        src.join(format!("{stem}.pdf")),
        dst.join(format!("{stem}.pdf")),
    )
    .unwrap();
    let text = std::fs::read_to_string(src.join(format!("{stem}.map.toml"))).unwrap();
    std::fs::write(dst.join(format!("{stem}.map.toml")), edit(text)).unwrap();
    dir
}

/// B1 — kill 2 observed red: a flipped hash is reported, and only it.
#[test]
fn a_planted_hash_mismatch_is_reported() {
    let ws = workspace_root();
    let dir = plant(2024, "f8959", |t| {
        let line = t
            .lines()
            .find(|l| l.starts_with("template_sha256"))
            .unwrap()
            .to_string();
        t.replace(&line, "template_sha256     = \"0000000000000000000000000000000000000000000000000000000000000000\"")
    });
    let (_, problems) = check_rows(
        dir.path(),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    // The flipped hash also fails the manifest join — both are the same planted defect.
    assert!(
        problems.contains(&RowProblem::HashMismatch {
            year: 2024,
            stem: "f8959".into()
        }),
        "{problems:?}"
    );
}

/// B1 — kill 3 observed red: strip the excuse from an excused row and the join reds.
#[test]
fn a_planted_missing_excuse_is_reported() {
    let ws = workspace_root();
    let dir = plant(2024, "f8283", |t| {
        t.lines()
            .filter(|l| !l.starts_with("authority"))
            .collect::<Vec<_>>()
            .join("\n")
    });
    let (_, problems) = check_rows(
        dir.path(),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    assert!(
        problems.contains(&RowProblem::NotInManifest {
            year: 2024,
            stem: "f8283".into()
        }),
        "{problems:?}"
    );
}

/// B1 — kill 4 observed red: a sequence number that is not what the form prints.
#[test]
fn a_planted_wrong_sequence_number_is_reported() {
    let ws = workspace_root();
    let dir = plant(2025, "f8283", |t| {
        t.replace(
            "attachment_sequence = \"36\"",
            "attachment_sequence = \"155\"",
        )
    });
    let (_, problems) = check_rows(
        dir.path(),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    assert!(
        problems.contains(&RowProblem::SequenceMismatch {
            year: 2025,
            stem: "f8283".into(),
            row: Some("155".into()),
            printed: Some("36".into()),
        }),
        "{problems:?}"
    );
}

/// The printed-sequence reader on the two layouts the extracts use (inline, and split across the
/// masthead columns), plus the 1040's none.
#[test]
fn printed_sequence_reader_handles_both_layouts() {
    assert_eq!(
        printed_sequence("Form 8949  Attachment Sequence No. 12A\n"),
        Some("12A".into())
    );
    assert_eq!(
        printed_sequence("Internal Revenue Service   Go to …            Sequence No. 02\n"),
        Some("02".into())
    );
    assert_eq!(
        printed_sequence("Form 1040  U.S. Individual Income Tax Return\n"),
        None
    );
}

#[allow(dead_code)]
fn _rows_by_key(rows: &[MapRow]) -> BTreeMap<(i32, String), &MapRow> {
    rows.iter().map(|r| ((r.year, r.form.clone()), r)).collect()
}
