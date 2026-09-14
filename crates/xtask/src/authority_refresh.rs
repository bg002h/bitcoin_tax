//! ★★★ **`xtask authority-refresh --check` — DOES IRS.GOV STILL SERVE WHAT THE NOTES CLAIM?**
//! (Seam review finding I3; `design/SPEC_interview.md` §7 row T2, whose kill list ends *"the archive
//! round-trip hash matches the note"*.)
//!
//! **The gap this closes.** The archive's whole design rests on a note: the PDF is gitignored, and
//! the note's URL + sha256 is what makes the committed text layer reproducible. Every note carries
//! the right instinct —
//!
//! > ★ A DIFFERENT hash does not mean a corrupt download — it means the IRS REVISED this document.
//! > That is a change to the authority: review it, never silently absorb it.
//!
//! — and until this command there was **no reader**. `authority_manifest`'s tests hash the *local*
//! copy, which in a fresh worktree does not exist at all. Nothing compared a note to what irs.gov
//! serves today, which is why the archive could sit a whole tax year behind the spec's target while
//! every instrument printed OK: three archived documents had already been revised, and six annual
//! editions superseded, before the build that archived them landed.
//!
//! **Why it is on demand and not in the suite.** It needs the network, and a test that needs the
//! network is a test that fails on a plane and gets deleted. `make check` and `cargo nextest` never
//! run it. What IS in the suite is [`compare`], the pure drift comparison, with its planted kill.
//!
//! **Two questions, because a stale archive has two shapes.**
//!
//! 1. **Drift** — the bytes at a note's own URL no longer hash to what the note records. That is the
//!    IRS revising a document in place, which is what happened to Form 1098 (Rev. April 2025
//!    replacing Rev. January 2022 at the same moving `irs-pdf/` URL).
//! 2. **A newer edition** — the note's URL still serves the same bytes, but `irs-prior` has begun
//!    serving `<stem>--<archived+1>.pdf`. That is what a *superseded* annual edition looks like:
//!    nothing about the archived document changes, and it simply stops being the paper a filer
//!    holds. Only this probe can see it.
//!
//! `--from-dir <dir>` reads `<dir>/<basename>` instead of fetching, so the command can be exercised
//! with no network at all.

use crate::authority_manifest::{self, Entry, Storage};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What one note's re-fetch said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The bytes at the URL still hash to the note's sha256.
    Same,
    /// The URL answered with different bytes — **the IRS revised the document**.
    Drift { got_sha: String, got_bytes: u64 },
    /// The URL could not be read at all.
    Unreachable(String),
}

/// ★★ **THE PURE COMPARISON**, so the kill needs neither the network nor the archive. `fetch`
/// returns the bytes the note's URL serves now.
pub fn compare(
    entries: &[Entry],
    fetch: &dyn Fn(&Entry) -> Result<Vec<u8>, String>,
) -> Vec<(String, Verdict)> {
    let mut out = Vec::new();
    for e in entries {
        if e.storage != Storage::Note || e.url.is_empty() {
            continue;
        }
        let verdict = match fetch(e) {
            Err(why) => Verdict::Unreachable(why),
            Ok(bytes) => {
                let got_sha = format!("{:x}", Sha256::digest(&bytes));
                if got_sha == e.sha256 && bytes.len() as u64 == e.bytes {
                    Verdict::Same
                } else {
                    Verdict::Drift {
                        got_sha,
                        got_bytes: bytes.len() as u64,
                    }
                }
            }
        };
        out.push((e.path.clone(), verdict));
    }
    out
}

fn curl_bytes(url: &str) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("curl")
        .args(["-sSL", "-A", "btctax-archive", "--max-time", "120", url])
        .output()
        .map_err(|e| format!("curl did not run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "curl exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if out.stdout.is_empty() {
        return Err("curl returned 0 bytes".into());
    }
    Ok(out.stdout)
}

fn curl_status(url: &str) -> Result<u32, String> {
    let out = std::process::Command::new("curl")
        .args([
            "-sI",
            "-A",
            "btctax-archive",
            "--max-time",
            "60",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            url,
        ])
        .output()
        .map_err(|e| format!("curl did not run: {e}"))?;
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .map_err(|e| format!("curl printed no status: {e}"))
}

/// The highest edition year archived for each information-return stem — the probe's starting point.
fn newest_archived_edition(entries: &[Entry]) -> BTreeMap<String, u32> {
    let mut newest: BTreeMap<String, u32> = BTreeMap::new();
    for e in entries {
        let name = e.path.rsplit('/').next().unwrap_or_default();
        let Some((stem, rest)) = name.split_once("--") else {
            continue;
        };
        if !crate::archive_check::is_information_return_stem(stem) {
            continue;
        }
        let Ok(year) = rest.trim_end_matches(".pdf").parse::<u32>() else {
            continue; // a `-DRAFT` edition is not a revision of record
        };
        let slot = newest.entry(stem.to_string()).or_insert(year);
        *slot = (*slot).max(year);
    }
    newest
}

/// `xtask authority-refresh --check [--from-dir <dir>]`.
/// A document this repo KNOWS it is missing and needs — the third question `authority-refresh` asks.
///
/// ★★★ **Why this exists at all.** The two probes above can only see documents that are ALREADY
/// archived: `newest_archived_edition` is, in its own words, *"the probe's starting point"*, so a
/// document that was never archived has no starting point and is invisible. FR-181 says *"archive both
/// the moment the IRS posts them"* — an action with **no instrument**, depending on somebody
/// remembering to check, which is the one thing this repo's doctrine says a guarantee may not rest on.
/// This turns that remembered action into a command that fails on the day it becomes possible.
///
/// ★★ **And why [`Wanted::face`] is the load-bearing field rather than the URL.** A 200 is NOT arrival.
/// Measured 2026-09-14: `https://www.irs.gov/pub/irs-dft/f1040--dft.pdf` answers **200** and is the
/// **TY2025** draft — its own face reads *"1040 U.S. Individual Income Tax Return 2025"* and its footer
/// *"Form 1040 (2025)"*. A probe keyed to the status code alone would have fired the day it was written,
/// every run, forever — and the repair for an instrument that cries wolf is always to mute it. So this
/// one reads the document's OWN printed year and only reports arrival when the page says the year we
/// asked for.
pub struct Wanted {
    /// The archive stem the document would take, e.g. `f1040`.
    pub stem: &'static str,
    /// The tax year wanted.
    pub year: u32,
    /// Every URL worth trying, final first then draft. A draft counts as arrival — FR-181 says
    /// *"drafts if that is what exists"* — but only if its face carries [`Wanted::year`].
    pub urls: &'static [&'static str],
    /// The string that must appear in the document's TEXT LAYER for it to be the wanted year. This is
    /// the whole honesty of the probe; see the type note.
    pub face: &'static str,
    /// What is blocked while this is missing, so the failure explains itself.
    pub blocks: &'static str,
}

/// ★ The list is SHORT and each row names what it blocks, because a wanted list that accumulates
/// aspirations becomes noise and then gets muted. A row leaves this table the moment the document is
/// archived — at which point the two probes above take over and this one stops being the thing that
/// watches it.
pub const WANTED: &[Wanted] = &[
    Wanted {
        stem: "f1040",
        year: 2026,
        urls: &[
            "https://www.irs.gov/pub/irs-prior/f1040--2026.pdf",
            "https://www.irs.gov/pub/irs-dft/f1040--dft.pdf",
        ],
        face: "U.S. Individual Income Tax Return 2026",
        blocks: "FR-181 — the 1040 is the form the whole packet is built around, and TY2026 has \
                 neither a final nor a draft; every TY2026 claim citing it is unverified",
    },
    Wanted {
        stem: "i1040gi",
        year: 2026,
        urls: &[
            "https://www.irs.gov/pub/irs-prior/i1040gi--2026.pdf",
            "https://www.irs.gov/pub/irs-dft/i1040gi--dft.pdf",
        ],
        face: "2026 Returns",
        blocks: "FR-181 — the Itemized Deductions Worksheet and the Social Security Benefits \
                 Worksheet both live in i1040gi, so every quote from either is unverifiable for TY2026",
    },
];

/// The text layer of `bytes`, via `pdftotext -layout`, or `None` if it cannot be read.
///
/// ★ Deliberately tolerant: a wanted-document probe that HARD-FAILS on a malformed download would turn
/// a network hiccup into a red gate, and the whole point of this probe is that it is quiet until the
/// day it matters.
fn text_layer(bytes: &[u8]) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("btctax-wanted-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok()?;
    let pdf = dir.join("probe.pdf");
    std::fs::write(&pdf, bytes).ok()?;
    let out = std::process::Command::new("pdftotext")
        .arg("-layout")
        .arg(&pdf)
        .arg("-")
        .output()
        .ok()?;
    let _ = std::fs::remove_dir_all(&dir);
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Whether a fetched text layer is the wanted year's document.
///
/// ★ Split out of [`wanted_has_arrived`] for one reason: it is the only part with a defect worth a
/// test, and it is the only part that can be tested without the network. See
/// `a_served_document_of_the_wrong_year_has_not_arrived`.
#[must_use]
pub fn face_matches(text: &str, w: &Wanted) -> bool {
    text.contains(w.face)
}

/// Whether a wanted document is being served AND its own face carries the wanted year.
///
/// Returns the URL that satisfied it, or `None`.
#[must_use]
pub fn wanted_has_arrived(w: &Wanted) -> Option<String> {
    for url in w.urls {
        if curl_status(url) != Ok(200) {
            continue;
        }
        let Ok(bytes) = curl_bytes(url) else { continue };
        if text_layer(&bytes).is_some_and(|t| face_matches(&t, w)) {
            return Some((*url).to_string());
        }
    }
    None
}

pub fn run(from_dir: Option<PathBuf>) -> Result<(), String> {
    let root = crate::form_geometry::repo_root();
    let entries = authority_manifest::load(&root)?;
    let offline = from_dir.clone();
    let fetch = move |e: &Entry| -> Result<Vec<u8>, String> {
        match &offline {
            Some(dir) => {
                let name = e.path.rsplit('/').next().unwrap_or_default();
                std::fs::read(dir.join(name)).map_err(|err| format!("{}: {err}", dir.display()))
            }
            None => curl_bytes(&e.url),
        }
    };
    let results = compare(&entries, &fetch);
    let (mut same, mut drift, mut unreachable) = (0usize, Vec::new(), Vec::new());
    for (path, verdict) in &results {
        match verdict {
            Verdict::Same => same += 1,
            Verdict::Drift { got_sha, got_bytes } => {
                let e = entries.iter().find(|e| &e.path == path).expect("entry");
                drift.push(format!(
                    "{path}\n      note   {} ({} bytes)\n      live   {got_sha} ({got_bytes} \
                     bytes)\n      url    {}",
                    e.sha256, e.bytes, e.url
                ));
            }
            Verdict::Unreachable(why) => unreachable.push(format!("{path}: {why}")),
        }
    }
    println!(
        "authority-refresh: {} note(s) re-fetched — {same} unchanged, {} DRIFTED, {} unreachable",
        results.len(),
        drift.len(),
        unreachable.len()
    );
    for d in &drift {
        println!("  ★ REVISED: {d}");
    }
    for u in &unreachable {
        println!("  ? unreachable: {u}");
    }

    // ── The second question: has a NEWER edition appeared beside the archived one? ───────────────
    let mut newer = Vec::new();
    if from_dir.is_none() {
        for (stem, year) in newest_archived_edition(&entries) {
            let next = year + 1;
            let url = format!("https://www.irs.gov/pub/irs-prior/{stem}--{next}.pdf");
            match curl_status(&url) {
                Ok(200) => newer.push(format!("{stem}--{next} ({url})")),
                Ok(_) => {}
                Err(e) => unreachable.push(format!("{url}: {e}")),
            }
        }
        println!(
            "authority-refresh: probed {} information-return stem(s) for a newer edition — {} found",
            newest_archived_edition(&entries).len(),
            newer.len()
        );
        for n in &newer {
            println!("  ★ A NEWER EDITION EXISTS: {n}");
        }
    } else {
        println!("authority-refresh: --from-dir, so the newer-edition probe was skipped");
    }

    // ── The third question: has a document we KNOW we are missing been posted? ──────────────────
    let mut arrived = Vec::new();
    if from_dir.is_none() {
        for w in WANTED {
            if let Some(url) = wanted_has_arrived(w) {
                arrived.push(format!("{}--{} ({url}) — {}", w.stem, w.year, w.blocks));
            }
        }
        println!(
            "authority-refresh: probed {} WANTED document(s) not yet archived — {} have arrived",
            WANTED.len(),
            arrived.len()
        );
        for a in &arrived {
            println!("  ★ A WANTED DOCUMENT HAS ARRIVED: {a}");
        }
    }

    if drift.is_empty() && newer.is_empty() && arrived.is_empty() {
        println!("authority-refresh: OK — every note still matches irs.gov, no newer edition is served, and no wanted document has been posted");
        Ok(())
    } else if drift.is_empty() && newer.is_empty() {
        Err(format!(
            "{} WANTED document(s) have been posted and are not archived yet. ★ This is GOOD NEWS \
             that must not be silent: archive each one (drafts included — FR-181 says \"drafts if \
             that is what exists\"), give it a census entry per edition, then DELETE its row from \
             `WANTED` so the ordinary drift and newer-edition probes take over watching it.",
            arrived.len()
        ))
    } else {
        Err(format!(
            "{} note(s) drifted and {} newer edition(s) exist. ★ A different hash is not a corrupt \
             download — it is the IRS REVISING the authority. Archive the new revision IN ADDITION \
             (design/forms/README.md), give it a census entry per edition, and let \
             `box_census::revision_in_force` decide which tax year it governs. Never overwrite the \
             old note in place.",
            drift.len(),
            newer.len()
        ))
    }
}

/// Repo-root helper for the tests.
#[cfg(test)]
fn repo_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority_manifest::Kind;

    fn entry(path: &str, bytes: &[u8]) -> Entry {
        Entry {
            path: path.into(),
            kind: Kind::Form,
            storage: Storage::Note,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            bytes: bytes.len() as u64,
            url: format!("https://www.irs.gov/pub/irs-prior/{path}"),
            extract: String::new(),
        }
    }

    /// ★★★ **B1 — the drift check watched RED on a note whose document the IRS revised.**
    ///
    /// The plant is the exact event the notes warn about and nothing could see: the URL answers, and
    /// answers with different bytes. If this ever returns [`Verdict::Same`] the command has become a
    /// reachability check wearing a hash's clothes.
    #[test]
    fn a_revised_document_reds_and_an_unchanged_one_does_not() {
        let unchanged = entry("f1099int--2024.pdf", b"the bytes the note recorded");
        let revised = entry("f1099g--2024.pdf", b"the bytes the note recorded");
        let entries = vec![unchanged.clone(), revised.clone()];

        let live = |e: &Entry| -> Result<Vec<u8>, String> {
            if e.path.contains("f1099g") {
                // The IRS revised this one in place — one byte is all it takes.
                Ok(b"the bytes the note recorded, plus a December 2026 revision".to_vec())
            } else {
                Ok(b"the bytes the note recorded".to_vec())
            }
        };
        let got = compare(&entries, &live);
        assert_eq!(got.len(), 2, "both notes must be asked: {got:?}");
        assert_eq!(got[0].1, Verdict::Same, "the unchanged note must pass");
        match &got[1].1 {
            Verdict::Drift { got_sha, got_bytes } => {
                assert_ne!(*got_sha, revised.sha256);
                assert_ne!(*got_bytes, revised.bytes);
            }
            other => panic!("a revised document must RED, and it did not: {other:?}"),
        }

        // ★ And an unreachable URL is its own verdict, never silently "same".
        let dead = |_: &Entry| -> Result<Vec<u8>, String> { Err("curl exited 22".into()) };
        assert!(matches!(
            compare(&entries, &dead)[0].1,
            Verdict::Unreachable(_)
        ));
    }

    /// ★ A `Committed` entry is not re-fetched: its bytes are in the tree, and
    /// `authority_manifest`'s own test already hashes them.
    #[test]
    fn only_note_backed_entries_are_re_fetched() {
        let mut committed = entry("26USC_s1211.html", b"law");
        committed.storage = Storage::Committed;
        let mut urlless = entry("f8949--2024.pdf", b"x");
        urlless.url = String::new();
        let entries = vec![committed, urlless, entry("fw2--2025.pdf", b"y")];
        let got = compare(&entries, &|_: &Entry| Ok(b"y".to_vec()));
        assert_eq!(
            got.len(),
            1,
            "only the note-backed, URL-bearing entry: {got:?}"
        );
        assert!(got[0].0.ends_with("fw2--2025.pdf"));
    }

    /// ★★ The newer-edition probe starts from the HIGHEST archived edition of each information
    /// return, read out of the manifest — not from a hand-list of years.
    #[test]
    fn the_probe_starts_from_the_newest_archived_edition() {
        let entries = authority_manifest::load(&repo_root()).expect("manifest loads");
        let newest = newest_archived_edition(&entries);
        assert_eq!(newest.get("fw2"), Some(&2026), "{newest:?}");
        assert_eq!(newest.get("f1099g"), Some(&2026), "{newest:?}");
        assert_eq!(newest.get("f1099int"), Some(&2024), "{newest:?}");
        assert_eq!(newest.get("f1098"), Some(&2025), "{newest:?}");
        assert!(
            !newest.contains_key("f6251"),
            "only the information-return series is probed: {newest:?}"
        );
    }
    /// ★★★ **B1 — the probe observed DISCRIMINATING, on the real document that makes a status-only
    /// probe wrong.**
    ///
    /// The plant is not synthetic and does not touch the network. `f1040--dft.pdf` answers **200**
    /// today and is the **TY2025** draft; the two strings below are copied from its actual text layer
    /// (measured 2026-09-14). So:
    ///
    /// * asked for 2026 — the year [`WANTED`] really asks for — it must say NOT arrived, even though a
    ///   document is being served at that URL. **A status-only probe fails here**, which is the defect.
    /// * asked for 2025 — the year that document really is — it must say arrived. Without this half the
    ///   test would pass on a `face_matches` hard-wired to `false`, which is [B1a]: the fixture and the
    ///   plant are the other half of the checker.
    ///
    /// ★ Per FR-235 the plant is not in the checker's vocabulary: it is a property of a real IRS
    /// document, not a rephrasing of the predicate.
    #[test]
    fn a_served_document_of_the_wrong_year_has_not_arrived() {
        // Verbatim from the text layer of https://www.irs.gov/pub/irs-dft/f1040--dft.pdf, 2026-09-14.
        let served = "                             1040 U.S. Individual Income Tax Return 2025\n\
                      Form 1040 (2025)\n";

        let asked_2026 = Wanted {
            stem: "f1040",
            year: 2026,
            urls: &[],
            face: "U.S. Individual Income Tax Return 2026",
            blocks: "test",
        };
        assert!(
            !face_matches(served, &asked_2026),
            "a document IS served at the wanted URL, but its face says 2025 — reporting this as \
             arrival is the status-only defect, and it would fire on every run forever"
        );

        let asked_2025 = Wanted {
            face: "U.S. Individual Income Tax Return 2025",
            ..asked_2026
        };
        assert!(
            face_matches(served, &asked_2025),
            "the same text, asked for the year it really is, must count as arrival — otherwise this \
             test passes on a predicate hard-wired to false and measures nothing"
        );
    }

    /// ★ Every `WANTED` row must state what it blocks, or the failure cannot explain itself and the
    /// next reader mutes it.
    #[test]
    fn every_wanted_row_names_what_it_blocks_and_where_to_look() {
        assert!(
            !WANTED.is_empty(),
            "an empty wanted list makes the probe vacuous"
        );
        for w in WANTED {
            assert!(
                !w.blocks.trim().is_empty(),
                "{}: no `blocks` reason",
                w.stem
            );
            assert!(!w.urls.is_empty(), "{}: nowhere to look", w.stem);
            assert!(
                w.face.contains(&w.year.to_string()),
                "{}: the face proof {:?} does not mention {} — then it cannot tell that year's \
                 document from another year's, which is the whole point",
                w.stem,
                w.face,
                w.year
            );
        }
    }
    /// ★★★ **B1 — this command must have a RUNNER, and the test exists because for its whole life it
    /// did not.**
    ///
    /// Nothing invoked `authority-refresh --check`: no CI job, no `make` target. That is precisely how
    /// FR-242 happened — two newer IRS editions were served, this command detects exactly that, and it
    /// was simply never run. Adding a third probe (`WANTED`) into an uninvoked command would have made
    /// it a better green-and-blind instrument, which is the dominant defect shape in this repo's ledger.
    ///
    /// ★ The assertion is on the WORKFLOW FILES, derived rather than hand-listed: any workflow that
    /// invokes the subcommand satisfies it, so the runner can be renamed or moved without a false red.
    /// ★★ What it deliberately does NOT claim: that GitHub actually ran the job, that the schedule
    /// fired, or that the job is a required check. A test cannot observe CI, and pretending otherwise
    /// would be the same false promise one level up — the `#[ignore]` reason that said *"run in CI"*
    /// while no job existed.
    #[test]
    fn some_workflow_actually_runs_this_command() {
        let dir = repo_root().join(".github/workflows");
        let mut invokers = Vec::new();
        for e in std::fs::read_dir(&dir).expect("the workflows directory exists") {
            let path = e.expect("a readable dir entry").path();
            if path.extension().is_some_and(|x| x == "yml" || x == "yaml") {
                let body = std::fs::read_to_string(&path).expect("a readable workflow");
                if body.contains("authority-refresh --check") {
                    invokers.push(path.file_name().unwrap().to_string_lossy().into_owned());
                }
            }
        }
        assert!(
            !invokers.is_empty(),
            "no workflow in .github/workflows invokes `authority-refresh --check`, so nothing asks \
             whether an archived note drifted, a newer edition appeared, or a WANTED document was \
             posted. FR-242 is what that costs: the command worked and was never run."
        );
    }
}
