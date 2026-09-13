//! ★★★ **`xtask forms fetch --restore` — PUT THE GITIGNORED AUTHORITY PDFs BACK, from the notes.**
//!
//! **The gap this closes (FR-139 / port-rehearsal F6).** `design/TY2026_PORT_REPORT.md`'s runbook tags
//! step 4 — *"add/refresh the `MANIFEST.json` entry"* — **M**, for mechanical. It is not mechanical in
//! the workflow this repo actually uses. `authority-manifest --regen` refuses, correctly and loudly, in
//! any tree that does not hold every gitignored authority PDF:
//!
//! > REFUSING to regenerate: it would drop 124 document(s) from MANIFEST.json without a word.
//!
//! An isolated worktree holds **0** of them; so does a fresh clone; so does CI. Until this command
//! there was no committed way to get them back — the rehearsal discharged step 4 by copying 125 PDFs
//! out of the main tree, which a January port could do only because that tree happened to have them.
//! Every `label-proof`, `label-census`, `extract-geometry`, `dependents-grid` and `forms extract`
//! invocation has the same precondition.
//!
//! **It is not a new source of truth.** Every fact it needs is already committed, in the provenance
//! note beside each absent PDF:
//!
//! ```text
//! https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf
//!
//! # f8995a--2025.pdf — IRS primary source, NOT committed …
//! # sha256  3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa
//! # bytes   117129
//! ```
//!
//! So the restore is a **fetch plus the note's own verification**, and the note already prints the two
//! commands a human would run by hand. This does it over the whole archive instead of one at a time.
//!
//! ★★ **A hash mismatch is a REFUSAL, never a download.** The notes say why, and it is the reason this
//! command writes nothing it has not verified first:
//!
//! > ★ A DIFFERENT hash does not mean a corrupt download — it means the IRS REVISED this document.
//! > That is a change to the authority: review it, never silently absorb it.
//!
//! Writing a differently-hashed PDF to the archive path would put a document the repo never audited
//! underneath every conformance check that reads it. So the bytes are hashed **before** they are
//! written, and a mismatch leaves the path absent and the run non-zero.
//!
//! ★★★ **OFFLINE-SAFE BY CONSTRUCTION, because CI runs a network-isolation job.** `--from-dir <dir>`
//! reads `<dir>/<basename>` instead of fetching — the same seam `authority-refresh --check --from-dir`
//! uses — and every test in this module drives [`restore`] with an in-process closure, so the suite
//! never opens a socket. The network path exists only behind an explicit operator invocation.

use crate::authority_manifest::{self, Storage};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One absent-or-present authority document, with everything its note records about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restorable {
    /// Repo-relative path of the document itself (gitignored).
    pub source: String,
    /// Repo-relative path of the note the facts below were read from.
    pub note: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}

impl Restorable {
    /// The filename a `--from-dir` directory is expected to hold.
    #[must_use]
    pub fn basename(&self) -> &str {
        self.source.rsplit('/').next().unwrap_or(&self.source)
    }
}

/// What happened to one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Already on disk, and its bytes hash to the note's sha256.
    Present,
    /// Fetched, verified against the note, written.
    Restored,
    /// ★★ On disk, but NOT the document the note describes. Left alone — overwriting is a decision.
    OnDiskWrongHash { got_sha: String, got_bytes: u64 },
    /// ★★ Fetched and **refused**: the bytes are not the document the note describes.
    Mismatch { got_sha: String, got_bytes: u64 },
    /// The source could not be read at all (no network, no such file in `--from-dir`).
    Unavailable(String),
}

impl Outcome {
    /// ★ `None` = the archive is whole for this document. `Some(reason)` = it is not, spelled for the
    /// operator. One function decides both, so the summary count and the printed list cannot disagree.
    #[must_use]
    pub fn failure(&self) -> Option<String> {
        match self {
            Outcome::Present | Outcome::Restored => None,
            Outcome::OnDiskWrongHash { got_sha, got_bytes } => Some(format!(
                "ON DISK but NOT the archived document — {got_sha} ({got_bytes} bytes). Left in \
                 place: replacing it is a decision, not a restore."
            )),
            Outcome::Mismatch { got_sha, got_bytes } => Some(format!(
                "REFUSED — the source served {got_sha} ({got_bytes} bytes), the note records a \
                 different document. ★ A different hash is not a corrupt download; it is the IRS \
                 REVISING the authority. Nothing was written."
            )),
            Outcome::Unavailable(why) => Some(format!("unavailable — {why}")),
        }
    }
}

/// ★ Read one provenance note. **Refuses rather than guessing** — a note missing its URL or its digest
/// cannot be restored from, and inventing either is how an unverified document enters the archive.
pub fn parse_note(note_rel: &str, text: &str) -> Result<(String, String, u64), String> {
    let url = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("http"))
        .unwrap_or_default()
        .to_string();
    if url.is_empty() {
        return Err(format!(
            "{note_rel} records no URL, so nothing can re-fetch it. (`authority_manifest::\
             URL_NOT_RECOVERABLE` is where a genuinely unrecoverable source is declared.)"
        ));
    }
    let sha = authority_manifest::sha256_in_note(text).ok_or_else(|| {
        format!(
            "{note_rel} records no sha256, so a fetch could not be verified. REFUSING — an \
             unverifiable document must not reach the archive path."
        )
    })?;
    let bytes = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("# bytes"))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .ok_or_else(|| format!("{note_rel} records no byte count"))?;
    Ok((url, sha, bytes))
}

/// ★★ **DERIVED from the notes on disk**, never a list typed here: every `<source>.txt` in every known
/// archive tree whose sibling name classifies as a primary source.
///
/// The same walk `authority_manifest::notes_without_binaries` does, minus its "is the binary absent"
/// filter — because `--restore` must be able to say *"present and hash-correct"* about a document it
/// did not need to fetch, which is half of what makes the report trustworthy.
pub fn enumerate(root: &Path) -> Result<Vec<Restorable>, String> {
    let mut out: Vec<Restorable> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for (tree, _) in crate::archive_check::KNOWN_ARCHIVES {
        let mut notes = Vec::new();
        collect_notes(&root.join(tree), root, &mut notes);
        for note_rel in notes {
            let source = note_rel.trim_end_matches(".txt").to_string();
            let text = match std::fs::read_to_string(root.join(&note_rel)) {
                Ok(t) => t,
                Err(e) => {
                    problems.push(format!("{note_rel}: {e}"));
                    continue;
                }
            };
            match parse_note(&note_rel, &text) {
                Ok((url, sha256, bytes)) => out.push(Restorable {
                    source,
                    note: note_rel,
                    url,
                    sha256,
                    bytes,
                }),
                Err(e) => problems.push(e),
            }
        }
    }
    if !problems.is_empty() {
        return Err(format!(
            "{} note(s) cannot be restored from:\n  {}",
            problems.len(),
            problems.join("\n  ")
        ));
    }
    out.sort_by(|a, b| a.source.cmp(&b.source));
    Ok(out)
}

/// The note walk. A `*.txt` beside a primary source is a note; the extract trees hold text layers, not
/// notes, and are skipped by name.
fn collect_notes(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "__pycache__" && name != "reviews" {
                collect_notes(&p, root, out);
            }
            continue;
        }
        let Some(stem) = name.strip_suffix(".txt") else {
            continue;
        };
        let Ok(rel) = p.strip_prefix(root) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        if authority_manifest::EXTRACT_TREES
            .iter()
            .any(|t| rel_s.starts_with(t))
        {
            continue;
        }
        if crate::archive_check::classify(stem).is_some() {
            out.push(rel_s);
        }
    }
}

/// The three seams [`restore`] is driven through. Named so the tests can substitute an in-process
/// closure for each: what is already at the archive path, what the source serves now, and the write.
pub type ReadLocal<'a> = &'a dyn Fn(&Restorable) -> Option<Vec<u8>>;
/// What the note's URL — or a `--from-dir` copy — serves now.
pub type Fetch<'a> = &'a dyn Fn(&Restorable) -> Result<Vec<u8>, String>;
/// Put VERIFIED bytes on disk. Nothing else may call this.
pub type Write<'a> = &'a mut dyn FnMut(&Restorable, &[u8]) -> Result<(), String>;

/// ★★★ **THE PURE RESTORE**, so the kill needs neither the network nor the archive.
///
/// `read_local` answers with the bytes already at the archive path (`None` = absent), `fetch` with the
/// bytes the note's URL — or a `--from-dir` copy — serves now, and `write` puts verified bytes on disk.
/// **Nothing is written before its digest is compared**, which is the whole safety property.
pub fn restore(
    items: &[Restorable],
    read_local: ReadLocal<'_>,
    fetch: Fetch<'_>,
    write: Write<'_>,
) -> Vec<(String, Outcome)> {
    let mut out = Vec::new();
    for it in items {
        let outcome = match read_local(it) {
            Some(bytes) => {
                let (got_sha, got_bytes) = digest(&bytes);
                if got_sha == it.sha256 && got_bytes == it.bytes {
                    Outcome::Present
                } else {
                    Outcome::OnDiskWrongHash { got_sha, got_bytes }
                }
            }
            None => match fetch(it) {
                Err(why) => Outcome::Unavailable(why),
                Ok(bytes) => {
                    let (got_sha, got_bytes) = digest(&bytes);
                    if got_sha != it.sha256 || got_bytes != it.bytes {
                        // ★★★ REFUSE. The note says a different hash means the IRS REVISED the
                        //     document; absorbing it silently is what must never happen.
                        Outcome::Mismatch { got_sha, got_bytes }
                    } else {
                        match write(it, &bytes) {
                            Ok(()) => Outcome::Restored,
                            Err(e) => Outcome::Unavailable(e),
                        }
                    }
                }
            },
        };
        out.push((it.source.clone(), outcome));
    }
    out
}

fn digest(bytes: &[u8]) -> (String, u64) {
    (format!("{:x}", Sha256::digest(bytes)), bytes.len() as u64)
}

/// ★★ **Two committed records of one fact must agree.** The notes and `MANIFEST.json` both record the
/// sha256 of every note-backed document, and the manifest is *derived* from the notes — so a
/// disagreement means one of them is stale and nothing on disk can say which. Fail closed: a restore
/// that picked one would put an unaudited document under every check that reads it.
///
/// A note the manifest does not list yet is not a disagreement (that is what `--regen` is for).
pub fn disagreements_with_manifest(root: &Path, items: &[Restorable]) -> Vec<String> {
    let Ok(entries) = authority_manifest::load(root) else {
        return Vec::new(); // no manifest to disagree with; `authority-manifest` reports that.
    };
    let by_path: BTreeMap<&str, &authority_manifest::Entry> = entries
        .iter()
        .filter(|e| e.storage == Storage::Note)
        .map(|e| (e.path.as_str(), e))
        .collect();
    let mut out = Vec::new();
    for it in items {
        let Some(e) = by_path.get(it.source.as_str()) else {
            continue;
        };
        if e.sha256 != it.sha256 || e.bytes != it.bytes {
            out.push(format!(
                "{}\n      note      {} ({} bytes)  [{}]\n      MANIFEST  {} ({} bytes)",
                it.source, it.sha256, it.bytes, it.note, e.sha256, e.bytes
            ));
        }
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

/// `xtask forms fetch --restore [--from-dir <dir>] [--stem <stem>]`.
pub fn run(args: &[String]) -> Result<(), String> {
    if !args.iter().any(|a| a == "--restore") {
        return Err(
            "usage: cargo run -p xtask -- forms fetch --restore [--from-dir <dir>] [--stem <stem>]\n\
             \x20      --restore  put every gitignored authority document back from its own note\n\
             \x20      --from-dir read <dir>/<basename> instead of fetching (OFFLINE; what CI and the\n\
             \x20                 tests use — there is no network in this repo's validation surface)\n\
             \x20      --stem     restrict to documents whose filename starts with <stem>"
                .to_string(),
        );
    }
    let root = crate::form_geometry::repo_root();
    let from_dir = flag_value(args, "--from-dir").map(PathBuf::from);
    let stem = flag_value(args, "--stem");

    let all = enumerate(&root)?;
    let items: Vec<Restorable> = match &stem {
        Some(s) => all
            .into_iter()
            .filter(|i| i.basename().starts_with(s.as_str()))
            .collect(),
        None => all,
    };
    if items.is_empty() {
        return Err(format!(
            "no note-backed document matches {:?}",
            stem.unwrap_or_default()
        ));
    }

    let disagree = disagreements_with_manifest(&root, &items);
    if !disagree.is_empty() {
        return Err(format!(
            "REFUSING to restore: {} document(s) whose note and MANIFEST.json record DIFFERENT \
             bytes. Two committed records of one fact disagree, so nothing here can say which \
             document the repo audited — repair that first (`authority-manifest --regen` in a tree \
             that holds the binaries), never let a restore pick a side.\n  ★ {}",
            disagree.len(),
            disagree.join("\n  ★ ")
        ));
    }

    let read_local =
        |it: &Restorable| -> Option<Vec<u8>> { std::fs::read(root.join(&it.source)).ok() };
    let offline = from_dir.clone();
    let fetch = move |it: &Restorable| -> Result<Vec<u8>, String> {
        match &offline {
            Some(dir) => std::fs::read(dir.join(it.basename()))
                .map_err(|e| format!("{}: {e}", dir.join(it.basename()).display())),
            None => curl_bytes(&it.url),
        }
    };
    let mut wrote = 0usize;
    let mut write = |it: &Restorable, bytes: &[u8]| -> Result<(), String> {
        let dest = root.join(&it.source);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&dest, bytes).map_err(|e| format!("{}: {e}", dest.display()))?;
        wrote += 1;
        Ok(())
    };

    let results = restore(&items, &read_local, &fetch, &mut write);
    let mut present = 0usize;
    let mut restored = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (path, outcome) in &results {
        match outcome.failure() {
            None if *outcome == Outcome::Present => present += 1,
            None => restored += 1,
            Some(why) => failures.push(format!("{path}: {why}")),
        }
    }
    println!(
        "forms fetch --restore: {} document(s) — {present} already present and hash-correct, \
         {restored} restored, {} failed{}",
        results.len(),
        failures.len(),
        match &from_dir {
            Some(d) => format!(" (offline, from {})", d.display()),
            None => String::new(),
        }
    );
    for f in &failures {
        println!("  ★ {f}");
    }
    if failures.is_empty() {
        println!(
            "forms fetch --restore: OK — every note-backed authority document is on disk and hashes \
             to what its note records. `authority-manifest --regen`, `label-proof`, `label-census`, \
             `extract-geometry` and `forms extract` can all run now."
        );
        Ok(())
    } else {
        Err(format!(
            "{} of {} document(s) are not in the archive. Nothing unverified was written.",
            failures.len(),
            results.len()
        ))
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, bytes: &[u8]) -> Restorable {
        let (sha, n) = digest(bytes);
        Restorable {
            source: format!("design/forms/2025/{name}"),
            note: format!("design/forms/2025/{name}.txt"),
            url: format!("https://www.irs.gov/pub/irs-prior/{name}"),
            sha256: sha,
            bytes: n,
        }
    }

    /// ★★★ **B1 — the restore is watched REFUSING a document the note does not describe, and
    /// accepting the one it does.**
    ///
    /// The plant is the event the notes warn about in capitals: the URL answers, and answers with
    /// different bytes, because the IRS revised the document in place. If the wrong document were
    /// written, every conformance check downstream would be reading an authority nobody audited —
    /// and they would all still print OK, because they read the extract and the PDF, never the note.
    ///
    /// The `write` callback records what it was asked to write, so this also pins the stronger claim:
    /// **the refused document is not written at all**, rather than written and then complained about.
    #[test]
    fn a_document_the_note_does_not_describe_is_refused_and_never_written() {
        let good = item("f8995a--2025.pdf", b"%PDF the archived TY2025 Form 8995-A");
        let revised = item("f6251--2025.pdf", b"%PDF the archived TY2025 Form 6251");
        let items = vec![good.clone(), revised.clone()];

        let absent = |_: &Restorable| -> Option<Vec<u8>> { None };
        let live = |it: &Restorable| -> Result<Vec<u8>, String> {
            if it.source.contains("f6251") {
                Ok(b"%PDF the TY2025 Form 6251, Rev. February 2026".to_vec())
            } else {
                Ok(b"%PDF the archived TY2025 Form 8995-A".to_vec())
            }
        };
        let mut written: Vec<String> = Vec::new();
        let mut write = |it: &Restorable, _: &[u8]| -> Result<(), String> {
            written.push(it.source.clone());
            Ok(())
        };

        let got = restore(&items, &absent, &live, &mut write);
        assert_eq!(got.len(), 2, "both documents must be attempted: {got:?}");
        assert_eq!(
            got[0].1,
            Outcome::Restored,
            "the matching document restores"
        );
        match &got[1].1 {
            Outcome::Mismatch { got_sha, got_bytes } => {
                assert_ne!(*got_sha, revised.sha256);
                assert_ne!(*got_bytes, revised.bytes);
            }
            other => panic!("a revised document must be REFUSED, and it was not: {other:?}"),
        }
        assert_eq!(
            written,
            vec![good.source.clone()],
            "the refused document must never reach the archive path"
        );
        assert!(
            got[1].1.failure().is_some(),
            "a refusal must make the run non-zero"
        );

        // ★ An unreachable source is its own outcome, never a silent success.
        let dead = |_: &Restorable| -> Result<Vec<u8>, String> { Err("curl exited 6".into()) };
        let mut noop = |_: &Restorable, _: &[u8]| -> Result<(), String> { Ok(()) };
        assert!(matches!(
            restore(&items, &absent, &dead, &mut noop)[0].1,
            Outcome::Unavailable(_)
        ));
    }

    /// ★★ A document already on disk is verified, not assumed — and a wrong one is reported without
    /// being overwritten, because replacing an archived authority is a decision a human makes.
    #[test]
    fn a_document_already_on_disk_is_hashed_rather_than_trusted() {
        let it = item("fw2--2025.pdf", b"%PDF the archived TY2025 Form W-2");
        let right = |_: &Restorable| -> Option<Vec<u8>> {
            Some(b"%PDF the archived TY2025 Form W-2".to_vec())
        };
        let wrong = |_: &Restorable| -> Option<Vec<u8>> { Some(b"%PDF something else".to_vec()) };
        let never = |_: &Restorable| -> Result<Vec<u8>, String> {
            panic!("a document on disk must not be re-fetched")
        };
        let mut write = |_: &Restorable, _: &[u8]| -> Result<(), String> {
            panic!("an on-disk document must never be overwritten by a restore")
        };
        let items = vec![it];
        assert_eq!(
            restore(&items, &right, &never, &mut write)[0].1,
            Outcome::Present
        );
        assert!(matches!(
            restore(&items, &wrong, &never, &mut write)[0].1,
            Outcome::OnDiskWrongHash { .. }
        ));
    }

    /// ★★ A note missing either half of its provenance is REFUSED, not guessed at.
    #[test]
    fn a_note_with_no_url_or_no_digest_is_refused() {
        let whole = "https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf\n\n\
                     # sha256  3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa\n\
                     # bytes   117129\n";
        let (url, sha, bytes) = parse_note("n.txt", whole).expect("a whole note parses");
        assert!(url.ends_with("f8995a--2025.pdf"));
        assert_eq!(sha.len(), 64);
        assert_eq!(bytes, 117_129);

        let no_url = whole.replace("https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf", "");
        let e = parse_note("n.txt", &no_url).expect_err("no URL must refuse");
        assert!(e.contains("records no URL"), "{e}");

        let no_sha = whole.replace(
            "3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa",
            "(not recorded)",
        );
        let e = parse_note("n.txt", &no_sha).expect_err("no sha256 must refuse");
        assert!(e.contains("records no sha256"), "{e}");
        assert!(
            e.contains("REFUSING"),
            "the refusal must say it is refusing: {e}"
        );
    }

    /// ★★★ **The disagreement guard, watched RED.** The note and `MANIFEST.json` each record the
    /// sha256 of every note-backed document; the manifest is derived from the notes, so if they split,
    /// one is stale and nothing on disk can say which. Planted by editing the note's copy.
    #[test]
    fn a_note_that_disagrees_with_the_manifest_stops_the_restore() {
        let root = repo_root();
        let real = enumerate(&root).expect("the notes enumerate");
        assert!(
            disagreements_with_manifest(&root, &real).is_empty(),
            "the committed notes and MANIFEST.json must agree today"
        );
        let mut tampered = real.clone();
        let victim = tampered
            .iter_mut()
            .find(|i| i.source.contains("f8995a--2025"))
            .expect("f8995a--2025 is archived");
        let source = victim.source.clone();
        victim.sha256 = "0".repeat(64);
        let found = disagreements_with_manifest(&root, &tampered);
        assert_eq!(
            found.len(),
            1,
            "exactly the planted split must red: {found:?}"
        );
        assert!(found[0].contains(&source), "{:?}", found[0]);
    }

    /// ★★ **Two independent derivations of "which documents are note-backed" must agree** — this
    /// module's walk of the notes on disk, and `MANIFEST.json`'s `storage: "note"` entries. A note the
    /// walk misses is a document `--restore` would silently leave absent.
    #[test]
    fn the_note_walk_covers_every_note_backed_manifest_entry() {
        let root = repo_root();
        let items = enumerate(&root).expect("the notes enumerate");
        let walked: std::collections::BTreeSet<&str> =
            items.iter().map(|i| i.source.as_str()).collect();
        let entries = authority_manifest::load(&root).expect("manifest loads");
        let note_backed: Vec<&str> = entries
            .iter()
            .filter(|e| e.storage == Storage::Note && !e.url.is_empty())
            .map(|e| e.path.as_str())
            .collect();
        assert!(
            note_backed.len() > 100,
            "only {} note-backed entries — this test would pass vacuously",
            note_backed.len()
        );
        let missed: Vec<&&str> = note_backed
            .iter()
            .filter(|p| !walked.contains(*p))
            .collect();
        assert!(
            missed.is_empty(),
            "the note walk misses {} manifest entry(ies), which `--restore` would leave absent \
             while printing OK: {missed:?}",
            missed.len()
        );
    }

    /// ★★★ **END TO END, OFFLINE, in a tempdir**: a tree holding a note and no binary, restored from a
    /// local directory — then the same restore refused when that directory serves a different
    /// document. This is the shape a fresh clone, an isolated worktree and CI are all in.
    #[test]
    fn an_absent_binary_is_restored_from_a_local_directory_and_a_wrong_one_is_not() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        std::fs::create_dir_all(root.join("design/forms/2025")).expect("mkdir");
        let served = tmp.path().join("served");
        std::fs::create_dir_all(&served).expect("mkdir");

        let body = b"%PDF-1.7 the archived document";
        let (sha, n) = digest(body);
        std::fs::write(
            root.join("design/forms/2025/f9999--2025.pdf.txt"),
            format!(
                "https://www.irs.gov/pub/irs-prior/f9999--2025.pdf\n\n\
                 # f9999--2025.pdf — IRS primary source, NOT committed.\n\
                 # sha256  {sha}\n# bytes   {n}\n"
            ),
        )
        .expect("write note");

        let items = enumerate(root).expect("enumerate");
        assert_eq!(items.len(), 1, "one note, one restorable: {items:?}");
        assert!(
            !root.join(&items[0].source).exists(),
            "the binary is absent"
        );

        // The offline source serves the right document.
        std::fs::write(served.join("f9999--2025.pdf"), body).expect("write served");
        let read_local =
            |it: &Restorable| -> Option<Vec<u8>> { std::fs::read(root.join(&it.source)).ok() };
        let from_dir = |it: &Restorable| -> Result<Vec<u8>, String> {
            std::fs::read(served.join(it.basename())).map_err(|e| e.to_string())
        };
        let mut write = |it: &Restorable, bytes: &[u8]| -> Result<(), String> {
            std::fs::write(root.join(&it.source), bytes).map_err(|e| e.to_string())
        };
        let got = restore(&items, &read_local, &from_dir, &mut write);
        assert_eq!(got[0].1, Outcome::Restored, "{got:?}");
        assert_eq!(
            std::fs::read(root.join(&items[0].source)).expect("restored"),
            body,
            "the restored bytes must be the archived document"
        );
        // ★ Second run: present, hash-verified, not re-fetched.
        let never = |_: &Restorable| -> Result<Vec<u8>, String> {
            panic!("must not re-fetch a present file")
        };
        assert_eq!(
            restore(&items, &read_local, &never, &mut write)[0].1,
            Outcome::Present
        );

        // ★★ And the refusal, end to end: the offline source now serves a revised document, and the
        //    archive path keeps the audited one.
        std::fs::remove_file(root.join(&items[0].source)).expect("rm");
        std::fs::write(
            served.join("f9999--2025.pdf"),
            b"%PDF-1.7 Rev. February 2026",
        )
        .expect("write served");
        let got = restore(&items, &read_local, &from_dir, &mut write);
        assert!(
            matches!(got[0].1, Outcome::Mismatch { .. }),
            "a revised document must be refused: {got:?}"
        );
        assert!(
            !root.join(&items[0].source).exists(),
            "REFUSED, so nothing may have been written"
        );
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crates/xtask -> repo root")
            .to_path_buf()
    }
}
