//! ★★ **Design r2 §10 step 1 — the ROW of the year-package table, and its four self-contained kills.**
//!
//! Every `forms/<year>/<stem>.map.toml` IS a row (design r2 §4); the row SET is the glob, never a
//! list. This file walks that glob and holds, for every committed map:
//!
//! 1. the row parses with its required keys (and a map missing one is REFUSED — kill 1);
//! 2. `template_sha256` is the sha256 of the bundled PDF beside it (kill 2);
//! 3. that hash joins `design/forms/MANIFEST.json` on an AUTHORITY entry — or the row carries
//!    `authority = "not-yet-archived: …"`, the ONLY excuse, on exactly the one row the design names
//!    and no second (kill 3);
//! 4. `attachment_sequence` equals the "Attachment Sequence No." printed on the archived extract,
//!    absent exactly on the rows whose extract prints none (kill 4) — and
//!    `btctax_forms::attachment_sequence` (the packet's stapling order) agrees with every row, which
//!    is how the Form 8283 155→36 renumber was found.
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
    /// No archived extract exists to read the printed number from — the value is held only by the row
    /// agreeing with `packet::attachment_sequence`. Named separately (step-1 review P1) rather than
    /// laundered through the MANIFEST excuse: NO committed row today (the five TY2017 rows that
    /// carried it went with the S9 drop of the TY2017 package, owner ruling 2026-09-06), pinned
    /// shrink-only. `a_row_with_no_extract_is_reported_as_unverifiable` keeps the branch observed.
    SequenceUnverifiable {
        year: i32,
        stem: String,
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
        // kill 4 — the printed sequence number, from the archived extract. Keyed to ITS OWN
        // mechanism (does an extract exist?), never to the manifest excuse: `2024/f8283` is excused
        // from the manifest join and yet has an extract printing 155, and that extract is what holds
        // its row (step-1 review P1). A row with no extract at all is a separately-named gap.
        let extract = extract_root.join(format!("{}--{}.txt", row.irs_stem, year));
        match std::fs::read_to_string(&extract) {
            Ok(text) => {
                let printed = printed_sequence(&text);
                if printed != row.attachment_sequence {
                    problems.push(RowProblem::SequenceMismatch {
                        year,
                        stem: stem.clone(),
                        row: row.attachment_sequence.clone(),
                        printed,
                    });
                }
            }
            Err(_) => problems.push(RowProblem::SequenceUnverifiable {
                year,
                stem: stem.clone(),
            }),
        }
        rows.push(row);
    }
    (rows, problems)
}

/// The rows the design names (r2 §4, §10 step 1) — a shrink-only pin. A new excuse reds;
/// removing one means an authority was archived and the row's `authority` key must go.
///
/// **EMPTY since 2026-09-06** (S9, owner ruling): the five entries here were the five TY2017
/// templates, and `design/forms/` never carried a 2017 archive to verify them against. Dropping the
/// TY2017 form package took the rows with it, so every committed row's sequence number is now read
/// off an archived extract. The strongest possible state of a shrink-only pin — and it can only
/// grow again by someone editing this list, which is the point.
const SEQUENCE_UNVERIFIABLE: &[(i32, &str)] = &[];

/// 6 → 1 on 2026-09-06 (S9): the five TY2017 rows carried the manifest excuse and are gone.
const EXCUSED: &[(i32, &str)] = &[(2024, "f8283")];

#[test]
fn every_committed_map_has_a_row_that_parses_and_all_four_kills_are_green() {
    let ws = workspace_root();
    let (rows, problems) = check_rows(
        &crate_root().join("forms"),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    // 37 → 41 on 2026-09-06 (the four Form 4868 / Form 1040-V rows, spec 4868/1040-V T1), then
    // 41 → 36 the same day: S9 dropped the five TY2017 rows with their form package.
    assert!(rows.len() >= 36, "the walk found only {} rows", rows.len());
    // The rows whose sequence number NO extract can verify — none, since S9. Shrink-only.
    let unverifiable: Vec<(i32, String)> = problems
        .iter()
        .filter_map(|p| match p {
            RowProblem::SequenceUnverifiable { year, stem } => Some((*year, stem.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        unverifiable,
        SEQUENCE_UNVERIFIABLE
            .iter()
            .map(|(y, s)| (*y, s.to_string()))
            .collect::<Vec<_>>(),
        "the sequence-unverifiable set must stay EMPTY; archive an authority to shrink it"
    );
    let problems: Vec<&RowProblem> = problems
        .iter()
        .filter(|p| !matches!(p, RowProblem::SequenceUnverifiable { .. }))
        .collect();
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
        "the `authority = \"not-yet-archived\"` excuse may only SHRINK, and only this one carries it"
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
        // ★ R1 / spec I-3, then seam review N-1: this read `is_none() == (form == "f1040")` — a
        //   HAND-LIST of one wearing an equality — and then a hand-list of three. Both spellings
        //   ENUMERATED the outcome someone happened to see. The predicate is now COMPUTED from the
        //   same archived extract kill 4 reads: `attachment_sequence` is absent exactly when the
        //   form's own printed page carries no sequence number. A fourth stem quietly losing its
        //   number reds from the FORM, with no list to remember to extend.
        //
        //   A row whose extract does not exist at all has nothing here to check against. Such rows
        //   are skipped by NAME from `SEQUENCE_UNVERIFIABLE`, whose membership is itself asserted
        //   above and is shrink-only — never by "the read failed", which would let a wrong path turn
        //   this into a check that silently never runs. That list is EMPTY since S9 dropped the five
        //   TY2017 rows, so today this branch runs on every committed row.
        let extract = ws
            .join("design/forms/extract")
            .join(format!("{}--{}.txt", r.irs_stem, r.year));
        if !SEQUENCE_UNVERIFIABLE.contains(&(r.year, r.form.as_str())) {
            let text = std::fs::read_to_string(&extract).unwrap_or_else(|e| {
                panic!(
                    "{}/{}: its archived extract must be READABLE at {} — the sequence shape is \
                     computed from the form, never skipped: {e}",
                    r.year,
                    r.form,
                    extract.display()
                )
            });
            assert_eq!(
                r.attachment_sequence.is_none(),
                printed_sequence(&text).is_none(),
                "{}/{}: the sequence number is absent exactly on the rows whose extract prints none — \
                 the 1040 itself, Form 4868 (mailed separately: \"Don\u{2019}t attach a copy of Form 4868 \
                 to your return.\") and Form 1040-V (\"Do not staple or attach this voucher to your \
                 payment or return.\")",
                r.year,
                r.form
            );
        }
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
    // ★ R1 / spec I-3: a SET, not one pinned row. `f1040s1a` is a section of the `i1040gi` booklet;
    //   the four 4868 / 1040-V rows name pages inside the FORM itself, which is its own instructions
    //   document (the IRS publishes no i4868 / i1040v). Each range was measured from the committed
    //   extract's form-feed page breaks: `f4868--<year>.txt` has 4 form feeds and prints instruction
    //   text on all four pages (page 1 above the DETACH HERE rule, pages 2–4 in full);
    //   `f1040v--<year>.txt` has 2 and prints instruction text on both (page 1's upper half carries
    //   "How To Fill in Form 1040-V", page 2 the payment methods and mailing addresses).
    assert_eq!(
        pages,
        vec![
            (2024, "f1040v", [1, 2]),
            (2024, "f4868", [1, 4]),
            (2025, "f1040s1a", [101, 110]),
            (2025, "f1040v", [1, 2]),
            (2025, "f4868", [1, 4]),
        ]
    );
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

/// B1 — kill 4 observed red ON A MANIFEST-EXCUSED ROW that has an extract: the exemption is the
/// manifest's, not this kill's (step-1 review P1). `2024/f8283` prints 155.
#[test]
fn a_planted_wrong_sequence_number_is_reported_even_on_a_manifest_excused_row() {
    let ws = workspace_root();
    let dir = plant(2024, "f8283", |t| {
        t.replace(
            "attachment_sequence = \"155\"",
            "attachment_sequence = \"999\"",
        )
    });
    let (_, problems) = check_rows(
        dir.path(),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    assert!(
        problems.contains(&RowProblem::SequenceMismatch {
            year: 2024,
            stem: "f8283".into(),
            row: Some("999".into()),
            printed: Some("155".into()),
        }),
        "{problems:?}"
    );
}

/// Copy one committed map into a tempdir under a DIFFERENT year, rewriting the `year` and `line_set`
/// keys so `check_rows`'s "the row's `year` must be its directory" assertion still holds. This is how
/// a defect is planted on a (year, stem) pair that has **no archived extract** now that every
/// committed row has one — before S9 the test below planted `2017/f8949`, which had no 2017 archive
/// only because the TY2017 package happened to ship unarchived.
fn plant_as(
    src_year: i32,
    dst_year: i32,
    stem: &str,
    edit: impl Fn(String) -> String,
) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = crate_root().join(format!("forms/{src_year}"));
    let dst = dir.path().join(format!("{dst_year}"));
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::copy(
        src.join(format!("{stem}.pdf")),
        dst.join(format!("{stem}.pdf")),
    )
    .unwrap();
    let raw = std::fs::read_to_string(src.join(format!("{stem}.map.toml"))).unwrap();
    let year_line = raw
        .lines()
        .find(|l| l.trim_start().starts_with("year") && l.contains(&src_year.to_string()))
        .unwrap_or_else(|| panic!("{src_year}/{stem}.map.toml has no `year` key"))
        .to_string();
    let text = raw
        .replace(
            &year_line,
            &year_line.replace(&src_year.to_string(), &dst_year.to_string()),
        )
        .replace(&format!("{stem}/{src_year}"), &format!("{stem}/{dst_year}"));
    std::fs::write(dst.join(format!("{stem}.map.toml")), edit(text)).unwrap();
    dir
}

/// B1 — a row with NO extract is reported as unverifiable, not silently passed (P1).
///
/// Re-pointed 2026-09-06 (S9): this planted `2017/f8949` because `design/forms/` carried no 2017
/// archive. With the TY2017 package dropped, EVERY committed row has an extract, so the kill plants
/// a real map at a year that has none — the mechanism, not a year that happened to be unarchived.
#[test]
fn a_row_with_no_extract_is_reported_as_unverifiable() {
    let ws = workspace_root();
    let dir = plant_as(2025, 1999, "f8949", |t| t);
    assert!(
        !ws.join("design/forms/extract/f8949--1999.txt").exists(),
        "the planted year must have NO archived extract, or this kill proves nothing"
    );
    let (_, problems) = check_rows(
        dir.path(),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    assert!(
        problems.contains(&RowProblem::SequenceUnverifiable {
            year: 1999,
            stem: "f8949".into()
        }),
        "{problems:?}"
    );
}

/// P5 — `AnnualTag`'s stated guarantee: a typo is a parse refusal, not a free string.
#[test]
fn a_mistyped_versioning_is_refused() {
    let text = std::fs::read_to_string(crate_root().join("forms/2024/f8959.map.toml")).unwrap();
    assert!(MapRow::read(&text.replace("\"annual\"", "\"anual\"")).is_err());
    let periodic = std::fs::read_to_string(crate_root().join("forms/2024/f8275.map.toml")).unwrap();
    assert!(periodic.contains("periodic = "), "the 8275 row is periodic");
    assert!(MapRow::read(&periodic.replace("periodic = ", "periodc = ")).is_err());
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

/// ★★ **B1 — kill 4 observed red on a row whose form prints NO sequence number.** This is the kill
/// that had to exist before `map_rows.rs:298` could be widened from *"absent iff form == f1040"* to
/// *"absent exactly on the rows whose extract prints none"* (R1 / spec I-3): the widened shape check
/// is a hand-list of three, and what actually holds the value is this comparison against the archived
/// extract. Planting a sequence number on the 4868 — which prints none, because it is mailed
/// separately rather than attached — must be reported as a mismatch.
#[test]
fn a_planted_sequence_number_on_a_form_that_prints_none_is_reported() {
    let ws = workspace_root();
    let dir = plant(2025, "f4868", |t| {
        t.replace(
            "line_set        = \"f4868/2025\"",
            "line_set        = \"f4868/2025\"\nattachment_sequence = \"99\"",
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
            stem: "f4868".into(),
            row: Some("99".into()),
            printed: None,
        }),
        "{problems:?}"
    );
    // …and the same row unplanted is clean, or the plant proves nothing.
    let clean = plant(2025, "f4868", |t| t);
    let (_, problems) = check_rows(
        clean.path(),
        &manifest_authority_hashes(&ws),
        &ws.join("design/forms/extract"),
    );
    assert!(problems.is_empty(), "{problems:?}");
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
