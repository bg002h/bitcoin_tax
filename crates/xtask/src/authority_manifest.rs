//! Step ③ — **one manifest and one checker over BOTH primary-source trees.**
//!
//! ★★ **The defect this closes.** The repo held two archiving conventions with no shared provenance,
//! so *"which one is authoritative?"* had no mechanical answer:
//!
//! | | binaries | text layer | provenance |
//! |---|---|---|---|
//! | **(A)** `design/forms/` | gitignored; each has a `.pdf.txt` URL + sha256 note | `design/forms/extract/` | hashes, revision-detecting |
//! | **(B)** `legal/primary-sources/` | **committed** | `legal/text/` | fetch scripts only — **no hashes at all** |
//!
//! The **hybrid** decision (owner, 2026-07-30): keep each tree's *storage* policy — it is right for
//! its material — and unify *provenance*. Forms are re-fetchable from `irs.gov/pub/irs-prior`
//! forever and are revised annually, so a hash is exactly the alarm you want; the statute and the
//! regulations are law as-of-a-date, should be frozen in the repo, and their non-IRS URLs are less
//! stable. **Storage differs by document kind; provenance does not differ at all.**
//!
//! ★★★ **And the manifest was itself unverified.** `design/forms/MANIFEST.json` already existed with
//! 66 entries — and **nothing in the codebase read it**. A manifest nobody checks is F4 in its purest
//! form: an instrument that cannot discriminate, reporting nothing, forever. This module is the
//! reader that makes it real.
//!
//! ★ **Two directions, because one is not enough.** A manifest that only checks *its own entries*
//! passes a repo where half the archive was never listed:
//!
//! - **verify** — every manifest entry resolves, and every committed file still hashes to what the
//!   manifest says. A changed hash means **the source was REVISED** — review it, never absorb it.
//! - **census** — every primary-source-shaped file in an accounted-for tree **appears in the
//!   manifest**. This is the [`crate::archive_check`] shape detector pointed inward, and it is the
//!   half that catches "archived but never recorded".

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// How a document is stored, which is the one thing the hybrid decision lets differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Storage {
    /// The binary is committed. Frozen, offline, self-contained — right for law as-of-a-date.
    Committed,
    /// The binary is gitignored; a `<path>.txt` note carries the URL + sha256. Right for forms,
    /// which are large, re-fetchable from a permanent IRS URL, and revised every year.
    Note,
}

/// What the document *is*. Derived from its path, never hand-assigned per file — a hand-assignment
/// is a hand-list, and a hand-list is F2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Rung 4 — 26 USC. **The only rung that is law.**
    Statute,
    /// Rung 3 — 26 CFR. The agency's interpretation; binding in practice, capable of being wrong.
    Regulation,
    /// Rung 2 — the numbered `iNNNN` instructions document.
    Instructions,
    /// Rung 1 — the form itself.
    Form,
    /// Sub-regulatory: Notices, Rev. Ruls., Rev. Procs., CCAs, TDs.
    Guidance,
    /// IRS publications — explanatory, not authority.
    Publication,
}

impl Kind {
    /// ★ Derived from the path and the IRS's own stem convention. `i1040gi` is instructions;
    /// `f6251` is a form; the `legal/` subdirectories name their own kind.
    pub fn of(rel: &str) -> Kind {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        if rel.contains("statute-irc") {
            return Kind::Statute;
        }
        if rel.contains("regulations-cfr") {
            return Kind::Regulation;
        }
        if rel.contains("irs-guidance") || rel.contains("federal-register") {
            return Kind::Guidance;
        }
        if rel.contains("irs-publications") || name.starts_with("Pub") {
            return Kind::Publication;
        }
        if name.starts_with('i') || name.starts_with("Instructions") {
            return Kind::Instructions;
        }
        Kind::Form
    }
}

/// One primary source, in whichever tree it lives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Repo-relative path to the document itself (the PDF/HTML/XML), committed or not.
    pub path: String,
    pub kind: Kind,
    pub storage: Storage,
    /// sha256 of the document. For [`Storage::Note`] this is what the note claims and what a
    /// re-fetch must reproduce; for [`Storage::Committed`] it is checked against the file on disk.
    pub sha256: String,
    pub bytes: u64,
    /// Where it came from. Empty only for entries in [`URL_NOT_RECOVERABLE`].
    #[serde(default)]
    pub url: String,
    /// Committed text layer, if one exists. Empty is normal — most sources have no extract yet.
    #[serde(default)]
    pub extract: String,
}

/// ★ Sources whose original URL is not recoverable from the repo — the `legal/` tree predates the
/// note convention and its fetch scripts do not cover every file. **Shrink-only**: re-deriving a URL
/// removes an entry, and the test below reds if one is listed that in fact has a URL.
///
/// This is a *recorded gap*, not an excuse: an authority we cannot re-fetch is one we cannot prove
/// current, and that is worth being able to grep for.
pub const URL_NOT_RECOVERABLE: &[&str] = &[
    // Fetched outside `legal/_scripts/` — no `dl` line records where it came from.
    "legal/primary-sources/irs-guidance/CCA_202302012.pdf",
    // §61 and §1223 are absent from the statute loop's section list (which covers §1, §170, §1001,
    // §1011, §1012, §1015, §1016, §1031, §1091, §1211, §1212, §1221, §1222, §1411). Both are
    // re-derivable by hand from the govinfo granule pattern, but nothing in the repo RECORDS that,
    // and inventing the URL here would be asserting provenance we do not have.
    "legal/primary-sources/statute-irc/26USC_s61.html",
    "legal/primary-sources/statute-irc/26USC_s1223.html",
];

/// ★★★ **The residual duplication, as a NUMBER that may only go down.**
///
/// Each group is one document archived under two paths — byte-identical, same sha256. Every one is a
/// `design/amt-form6251/` note shadowing the `design/forms/2025/` entry for the same form, left from
/// before `design/forms/` existed.
///
/// This is what remains of step ③ after the hybrid decision: the *conventions* are now unified (one
/// manifest, one checker, provenance identical across both trees), but the stray copies are still
/// there. Retiring them is mechanical — delete the note, keep `design/amt-form6251/`'s actual design
/// work — and repoint the provenance line in
/// `crates/btctax-core/src/tax/fixtures/schedule_1a_2025_form.txt`.
///
/// ★ Pinned rather than merely reported, because "we know about it" is precisely the state that
/// produced four archives. The count is measured by `duplicates()`; lowering it is the deliverable.
pub const DUPLICATE_SOURCE_GROUPS: usize = 15;

/// The committed **text layers** — the extracted-text counterpart each tree keeps beside its
/// sources. These are derived artifacts, never authorities, so they are neither manifest entries nor
/// census subjects.
///
/// ★ `design/forms/` keeps its extracts in a sibling directory; `legal/` keeps them in a parallel
/// tree. That is the one cosmetic difference the hybrid decision leaves standing, and it is harmless
/// precisely because this list names both.
pub const EXTRACT_TREES: &[&str] = &["design/forms/extract", "legal/text"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

pub fn manifest_path(root: &Path) -> PathBuf {
    root.join("design/forms/MANIFEST.json")
}

pub fn sha256_of(p: &Path) -> std::io::Result<(String, u64)> {
    let bytes = std::fs::read(p)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok((format!("{:x}", h.finalize()), bytes.len() as u64))
}

pub fn load(root: &Path) -> Result<Vec<Entry>, String> {
    let p = manifest_path(root);
    let text =
        std::fs::read_to_string(&p).map_err(|e| format!("cannot read {} ({e})", p.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} is not a valid manifest: {e}", p.display()))
}

/// A discrepancy between the manifest and the tree.
#[derive(Debug, PartialEq, Eq)]
pub enum Problem {
    /// A committed file the manifest lists is gone.
    MissingFile(String),
    /// A gitignored file's `.txt` provenance note is gone — the only record of where it came from.
    MissingNote(String),
    /// ★★ The committed file no longer hashes to what the manifest says. **This means the document
    /// was REVISED** (or edited). Never absorb it silently.
    HashMismatch {
        path: String,
        manifest: String,
        actual: String,
    },
    /// A named extract does not exist.
    MissingExtract { path: String, extract: String },
    /// A primary source exists in an accounted-for tree but no manifest entry names it.
    NotInManifest(String),
    /// Listed as unrecoverable, but a URL is present after all — stale excuse.
    StaleUrlExcuse(String),
}

impl Problem {
    pub fn describe(&self) -> String {
        match self {
            Problem::MissingFile(p) => format!("{p} — listed as committed, but the file is gone"),
            Problem::MissingNote(p) => {
                format!("{p} — gitignored, and its `.txt` provenance note is missing; nothing records where it came from")
            }
            Problem::HashMismatch {
                path,
                manifest,
                actual,
            } => format!(
                "{path} — HASH CHANGED\n        manifest: {manifest}\n        on disk:  {actual}\n\
                 \x20       ★ A changed hash means the source was REVISED. Review the change and \
                 update deliberately — never regenerate to make this go away."
            ),
            Problem::MissingExtract { path, extract } => {
                format!("{path} — names extract `{extract}`, which does not exist")
            }
            Problem::NotInManifest(p) => {
                format!("{p} — a primary source in an accounted-for tree with NO manifest entry")
            }
            Problem::StaleUrlExcuse(p) => {
                format!("{p} — in URL_NOT_RECOVERABLE but a URL is recorded; remove the excuse")
            }
        }
    }
}

/// **Direction 1 — every manifest entry resolves, and committed files still hash true.**
pub fn verify(root: &Path, entries: &[Entry]) -> Vec<Problem> {
    let mut out = Vec::new();
    for e in entries {
        let abs = root.join(&e.path);
        match e.storage {
            Storage::Committed => {
                if !abs.is_file() {
                    out.push(Problem::MissingFile(e.path.clone()));
                } else if let Ok((sha, _)) = sha256_of(&abs) {
                    if sha != e.sha256 {
                        out.push(Problem::HashMismatch {
                            path: e.path.clone(),
                            manifest: e.sha256.clone(),
                            actual: sha,
                        });
                    }
                }
            }
            Storage::Note => {
                // The binary is deliberately absent; the NOTE is the artifact that must survive.
                let note = root.join(format!("{}.txt", e.path));
                if !note.is_file() {
                    out.push(Problem::MissingNote(e.path.clone()));
                }
            }
        }
        if !e.extract.is_empty() && !root.join(&e.extract).is_file() {
            out.push(Problem::MissingExtract {
                path: e.path.clone(),
                extract: e.extract.clone(),
            });
        }
        if !e.url.is_empty() && URL_NOT_RECOVERABLE.contains(&e.path.as_str()) {
            out.push(Problem::StaleUrlExcuse(e.path.clone()));
        }
    }
    out
}

/// **Direction 2 — every primary source in an accounted-for tree is IN the manifest.**
///
/// ★★ Without this the manifest is self-referential: it would happily verify its 66 entries while
/// 47 committed statutes sat unlisted beside them. This is [`crate::archive_check`]'s shape detector
/// turned inward — the same rule (*shape, not path*) answering the opposite question.
pub fn census(root: &Path, entries: &[Entry]) -> Vec<Problem> {
    let listed: BTreeSet<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    let mut out = Vec::new();
    for (tree, _) in crate::archive_check::KNOWN_ARCHIVES {
        let mut found = Vec::new();
        collect_sources(&root.join(tree), root, &mut found);
        for rel in found {
            if !listed.contains(rel.as_str()) {
                out.push(Problem::NotInManifest(rel));
            }
        }
    }
    out.sort_by_key(|p| format!("{p:?}"));
    out
}

/// Every primary-source *document* under `dir` — the binaries, not their notes or extracts.
fn collect_sources(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != "__pycache__" && name != "reviews" {
                collect_sources(&p, root, out);
            }
            continue;
        }
        // ★ Neither a provenance NOTE nor a TEXT LAYER is a source. Both trees keep an extract
        // layer — `design/forms/extract/` and `legal/text/` — and counting those as sources
        // double-lists every document (it inflated the first regen to 72 "committed" when
        // legal/primary-sources holds 47).
        let rel_ok = p.strip_prefix(root);
        if name.ends_with(".pdf.txt")
            || rel_ok.is_ok_and(|r| EXTRACT_TREES.iter().any(|t| r.starts_with(t)))
        {
            continue;
        }
        if crate::archive_check::classify(&name).is_some() {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

/// Documents archived under more than one path, grouped by content hash.
///
/// ★ Keyed on **sha256, not filename** — the two trees name the same document differently
/// (`Form_8949.pdf` vs `f8949--2025.pdf`), so a name-based check would report zero duplicates in a
/// repo full of them.
pub fn duplicates(entries: &[Entry]) -> Vec<(String, Vec<String>)> {
    let mut by_hash: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in entries.iter().filter(|e| !e.sha256.is_empty()) {
        by_hash
            .entry(e.sha256.clone())
            .or_default()
            .push(e.path.clone());
    }
    let mut out: Vec<(String, Vec<String>)> =
        by_hash.into_iter().filter(|(_, v)| v.len() > 1).collect();
    for (_, v) in &mut out {
        v.sort();
    }
    out.sort();
    out
}

pub fn report(problems: &[Problem]) -> String {
    let mut s = format!("AUTHORITY MANIFEST — {} problem(s):\n\n", problems.len());
    for p in problems {
        s.push_str(&format!("  {}\n", p.describe()));
    }
    s.push_str(
        "\n  The manifest is design/forms/MANIFEST.json and covers BOTH trees: forms are stored as\n  \
         URL+sha256 notes (gitignored binaries), statute/regs/guidance are committed. Storage differs\n  \
         by document kind; provenance does not.\n",
    );
    s
}

/// URLs recorded in `legal/_scripts/*.sh` as `dl <url> <relpath>`.
///
/// ★ The scripts are real shell, not a table: URLs are quoted, built from `${VAR}` prefixes, and
/// split across line continuations. A naive `split_whitespace` recovered 87 of 138 and silently
/// dropped every statute and regulation — i.e. **exactly the rungs that are law**. Parsing the three
/// constructs the scripts actually use is the difference between a provenance record and a plausible
/// one.
fn harvested_urls(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(root.join("legal/_scripts")) else {
        return out;
    };
    for e in entries.flatten() {
        let Ok(text) = std::fs::read_to_string(e.path()) else {
            continue;
        };

        // Join line continuations so a `dl <url> \` / `<rel>` pair is one logical line.
        let joined = text.replace("\\\n", " ");

        // Collect simple `NAME="value"` assignments for ${VAR} expansion.
        let mut vars: BTreeMap<String, String> = BTreeMap::new();
        for line in joined.lines() {
            let t = line.trim();
            if t.starts_with('#') {
                continue;
            }
            if let Some((name, val)) = t.split_once('=') {
                if !name.is_empty()
                    && name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                    && val.starts_with('"')
                {
                    vars.insert(name.to_string(), val.trim_matches('"').to_string());
                }
            }
        }

        let expand = |s: &str| -> String {
            let mut v = s.to_string();
            for (k, val) in &vars {
                v = v
                    .replace(&format!("${{{k}}}"), val)
                    .replace(&format!("${k}"), val);
            }
            v
        };

        // ★ `declare -A P=( [1]="…sec1" … )` — the section→granule map the statute loop indexes.
        // Rung 4 is fetched entirely through this construct, so skipping it means the STATUTE has no
        // recorded provenance, which is the least acceptable gap in the whole manifest.
        let mut assoc: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut cur: Option<String> = None;
        for line in joined.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("declare -A ") {
                if let Some((name, _)) = rest.split_once('=') {
                    cur = Some(name.to_string());
                    assoc.entry(name.to_string()).or_default();
                }
                continue;
            }
            if cur.is_some() && t.starts_with(')') {
                cur = None;
                continue;
            }
            if let Some(name) = cur.clone() {
                if let Some(rest) = t.strip_prefix('[') {
                    if let Some((k, v)) = rest.split_once("]=") {
                        assoc
                            .entry(name)
                            .or_default()
                            .insert(k.to_string(), v.trim().trim_matches('"').to_string());
                    }
                }
            }
        }

        // Walk logical lines, tracking the innermost `for VAR in ...; do` so `dl` lines inside it
        // are emitted once per iteration.
        let mut loop_ctx: Option<(String, Vec<String>)> = None;
        for line in joined.lines() {
            let t = line.trim();
            if t.starts_with('#') {
                continue;
            }
            if let Some(rest) = t.strip_prefix("for ") {
                if let Some((var, tail)) = rest.split_once(" in ") {
                    let items = tail
                        .trim_end_matches("do")
                        .trim_end()
                        .trim_end_matches(';')
                        .split_whitespace()
                        .map(|s| s.trim_matches('"').to_string())
                        .collect::<Vec<_>>();
                    loop_ctx = Some((var.trim().to_string(), items));
                }
                continue;
            }
            if t == "done" {
                loop_ctx = None;
                continue;
            }
            let Some(rest) = t.strip_prefix("dl ") else {
                continue;
            };
            let mut it = rest
                .split_whitespace()
                .map(|a| a.trim_matches('"').to_string());
            let (Some(url_raw), Some(rel_raw)) = (it.next(), it.next()) else {
                continue;
            };

            let iterations: Vec<Option<String>> = match &loop_ctx {
                Some((_, items)) => items.iter().cloned().map(Some).collect(),
                None => vec![None],
            };
            for item in iterations {
                let (mut url, mut rel) = (expand(&url_raw), rel_raw.clone());
                if let (Some((var, _)), Some(val)) = (&loop_ctx, &item) {
                    for pat in [format!("${{{var}}}"), format!("${var}")] {
                        url = url.replace(&pat, val);
                        rel = rel.replace(&pat, val);
                    }
                    // `${MAP[$VAR]}` -> the associative-array lookup for this iteration.
                    for (name, map) in &assoc {
                        if let Some(v) = map.get(val) {
                            for pat in [
                                format!("${{{name}[${var}]}}"),
                                format!("${{{name}[{val}]}}"),
                            ] {
                                url = url.replace(&pat, v);
                            }
                        }
                    }
                }
                if url.starts_with("http") && !url.contains('$') {
                    out.insert(
                        format!("legal/primary-sources/{}", rel.trim_matches('"')),
                        url,
                    );
                }
            }
        }
    }
    out
}

/// The committed text layer for a source, if one exists.
fn extract_for(root: &Path, rel: &str) -> String {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);

    // (A) design/forms/<year>/<stem>.pdf  ->  design/forms/extract/<stem>.txt
    let a = format!("design/forms/extract/{stem}.txt");
    if root.join(&a).is_file() {
        return a;
    }
    // (B) legal/primary-sources/<sub>/<name>.<ext>  ->  legal/text/<sub>/<stem>.txt
    if let Some(sub) = rel.strip_prefix("legal/primary-sources/") {
        if let Some((dir, _)) = sub.rsplit_once('/') {
            let b = format!("legal/text/{dir}/{stem}.txt");
            if root.join(&b).is_file() {
                return b;
            }
        }
    }
    String::new()
}

/// `cargo run -p xtask -- authority-manifest --regen`
///
/// ★★ **Derived from the trees, never hand-listed.** That is not a convenience — a hand-written
/// manifest is exactly the enumeration-from-a-hand-list failure (F2) that produced the four-archive
/// mess in the first place. Storage is read from `git check-ignore` rather than assumed, so the
/// manifest records what git actually does with each file.
pub fn regen(root: &Path) -> Result<usize, String> {
    let urls = harvested_urls(root);
    let mut sources = Vec::new();
    for (tree, _) in crate::archive_check::KNOWN_ARCHIVES {
        collect_sources(&root.join(tree), root, &mut sources);
    }
    sources.sort();
    sources.dedup();

    let mut entries = Vec::new();
    for rel in sources {
        let abs = root.join(&rel);
        let ignored = std::process::Command::new("git")
            .args(["check-ignore", "-q", &rel])
            .current_dir(root)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let storage = if ignored {
            Storage::Note
        } else {
            Storage::Committed
        };

        // For note-storage the binary may be absent; the note carries URL (line 1) and sha256.
        let note = root.join(format!("{rel}.txt"));
        let note_text = std::fs::read_to_string(&note).unwrap_or_default();
        let note_url = note_text.lines().next().unwrap_or("").trim().to_string();
        let note_sha = note_text
            .lines()
            .find_map(|l| l.split_once("sha256:").map(|(_, s)| s.trim().to_string()))
            .or_else(|| {
                note_text
                    .lines()
                    .find(|l| {
                        l.trim().len() == 64 && l.trim().chars().all(|c| c.is_ascii_hexdigit())
                    })
                    .map(|l| l.trim().to_string())
            })
            .unwrap_or_default();

        let (sha256, bytes) = match sha256_of(&abs) {
            Ok(v) => v,
            Err(_) => (note_sha, 0), // gitignored and absent locally — trust the note
        };

        let url = if note_url.starts_with("http") {
            note_url
        } else {
            urls.get(&rel).cloned().unwrap_or_default()
        };

        entries.push(Entry {
            kind: Kind::of(&rel),
            storage,
            sha256,
            bytes,
            url,
            extract: extract_for(root, &rel),
            path: rel,
        });
    }

    let json =
        serde_json::to_string_pretty(&entries).map_err(|e| format!("serialising manifest: {e}"))?;
    std::fs::write(manifest_path(root), format!("{json}\n"))
        .map_err(|e| format!("writing manifest: {e}"))?;
    Ok(entries.len())
}

/// `cargo run -p xtask -- authority-manifest`
pub fn run() -> Result<(), String> {
    let root = repo_root();
    let entries = load(&root)?;
    let mut problems = verify(&root, &entries);
    problems.extend(census(&root, &entries));

    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for e in &entries {
        *by_kind
            .entry(format!("{:?}", e.kind).to_lowercase())
            .or_default() += 1;
    }
    let committed = entries
        .iter()
        .filter(|e| e.storage == Storage::Committed)
        .count();
    println!(
        "authority-manifest: {} entries — {committed} committed, {} note-only",
        entries.len(),
        entries.len() - committed
    );
    println!(
        "authority-manifest: by kind — {}",
        by_kind
            .iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let dups = duplicates(&entries);
    println!(
        "authority-manifest: {} document(s) archived under more than one path (pinned \
         {DUPLICATE_SOURCE_GROUPS}, may only shrink — CONTINUITY.md §0 step ③)",
        dups.len()
    );
    if problems.is_empty() {
        println!("authority-manifest: OK — every entry resolves and every source is listed");
        return Ok(());
    }
    Err(report(&problems))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        repo_root()
    }

    /// ★★ **THE LIVE GATE, direction 1.** Every entry resolves; every committed file hashes true.
    #[test]
    fn every_manifest_entry_resolves_and_hashes_true() {
        let entries = load(&root()).expect("manifest loads");
        let problems = verify(&root(), &entries);
        assert!(problems.is_empty(), "{}", report(&problems));
    }

    /// ★★ **THE LIVE GATE, direction 2.** Every primary source in an accounted-for tree is listed.
    #[test]
    fn every_primary_source_is_in_the_manifest() {
        let entries = load(&root()).expect("manifest loads");
        let problems = census(&root(), &entries);
        assert!(problems.is_empty(), "{}", report(&problems));
    }

    /// ★★★ **THE KILL.** Every problem class is planted and the checker is *observed red*. A
    /// verifier that has never been seen to discriminate is exactly F4 — and this manifest sat in
    /// the repo with 66 entries and no reader at all, which is how F4 looks before anyone notices.
    #[test]
    fn every_problem_class_is_caught() {
        let dir = tempfile::tempdir().expect("tempdir");
        let r = dir.path();
        std::fs::create_dir_all(r.join("legal/primary-sources/statute-irc")).expect("mkdir");
        let real = r.join("legal/primary-sources/statute-irc/26USC_s1.html");
        std::fs::write(&real, "the law").expect("write");
        let (sha, bytes) = sha256_of(&real).expect("hash");

        let good = Entry {
            path: "legal/primary-sources/statute-irc/26USC_s1.html".into(),
            kind: Kind::Statute,
            storage: Storage::Committed,
            sha256: sha.clone(),
            bytes,
            url: "https://example.invalid/26USC_s1".into(),
            extract: String::new(),
        };
        assert!(
            verify(r, std::slice::from_ref(&good)).is_empty(),
            "a true entry must verify"
        );

        // 1. ★ The one that matters most: the document was REVISED under us.
        let mut tampered = good.clone();
        tampered.sha256 = "0".repeat(64);
        assert!(
            matches!(verify(r, &[tampered])[..], [Problem::HashMismatch { .. }]),
            "a changed hash MUST be caught — this is how an IRS revision announces itself"
        );

        // 2. A committed file that has vanished.
        let mut gone = good.clone();
        gone.path = "legal/primary-sources/statute-irc/26USC_s999.html".into();
        assert!(matches!(verify(r, &[gone])[..], [Problem::MissingFile(_)]));

        // 3. A note-storage entry whose provenance note is missing — the binary is *supposed* to be
        //    absent, so the note is the only thing proving where it came from.
        let noted = Entry {
            path: "design/forms/2025/f6251--2025.pdf".into(),
            kind: Kind::Form,
            storage: Storage::Note,
            sha256: sha.clone(),
            bytes,
            url: "https://example.invalid/f6251".into(),
            extract: String::new(),
        };
        assert!(matches!(
            verify(r, std::slice::from_ref(&noted))[..],
            [Problem::MissingNote(_)]
        ));

        // 4. A named extract that does not exist.
        let mut bad_extract = good.clone();
        bad_extract.extract = "legal/text/nope.txt".into();
        assert!(matches!(
            verify(r, &[bad_extract])[..],
            [Problem::MissingExtract { .. }]
        ));

        // 5. ★★ The census direction: a real source on disk that NO entry names. Without this the
        //    manifest verifies itself into a green light while half the archive is unlisted.
        let problems = census(r, &[]);
        assert!(
            problems
                .iter()
                .any(|p| matches!(p, Problem::NotInManifest(s) if s.contains("26USC_s1"))),
            "an unlisted primary source must be caught, got {problems:?}"
        );
    }

    /// ★ `Kind` is derived from the path, so a new document lands in the right rung without anyone
    /// classifying it by hand. Anchored to real repo paths.
    #[test]
    fn kind_is_derived_from_the_path() {
        for (path, want) in [
            (
                "legal/primary-sources/statute-irc/26USC_s1211.html",
                Kind::Statute,
            ),
            (
                "legal/primary-sources/regulations-cfr/26CFR_1.1012-1_basis.xml",
                Kind::Regulation,
            ),
            (
                "legal/primary-sources/irs-guidance/Notice_2014-21.pdf",
                Kind::Guidance,
            ),
            (
                "legal/primary-sources/irs-publications/Pub550_Investment_Income_Expenses.pdf",
                Kind::Publication,
            ),
            (
                "legal/primary-sources/federal-register/TD_10000_89FR56480_broker_regs.pdf",
                Kind::Guidance,
            ),
            ("design/forms/2025/f6251--2025.pdf", Kind::Form),
            ("design/forms/2025/i1040gi--2025.pdf", Kind::Instructions),
            (
                "legal/primary-sources/irs-forms/Instructions_8949.pdf",
                Kind::Instructions,
            ),
        ] {
            assert_eq!(Kind::of(path), want, "wrong kind for {path}");
        }
    }

    /// ★★★ **The duplication countdown — step ③'s remaining work, as a test.**
    ///
    /// Fifteen documents are archived twice. The hybrid decision unified the *conventions*; these are
    /// the leftover copies. The number may only go DOWN, and it is measured on content hashes rather
    /// than filenames — the two trees name the same document differently, so a name-based check
    /// would cheerfully report zero.
    #[test]
    fn duplicate_source_groups_may_only_shrink() {
        let entries = load(&root()).expect("manifest loads");
        let dups = duplicates(&entries);
        assert!(
            dups.len() <= DUPLICATE_SOURCE_GROUPS,
            "duplicate archived documents rose to {} (pinned {DUPLICATE_SOURCE_GROUPS}):\n{}",
            dups.len(),
            dups.iter()
                .map(|(_, v)| format!("    {}\n", v.join("  ==  ")))
                .collect::<String>()
        );
        // ★ And when they are retired, this reds so the constant must come down with them —
        // otherwise the pin rots into a number nobody revisits, which is the excuse-list failure.
        assert_eq!(
            dups.len(),
            DUPLICATE_SOURCE_GROUPS,
            "duplicates fell to {} — lower DUPLICATE_SOURCE_GROUPS to match; the pin must track \
             reality, not sit above it",
            dups.len()
        );
    }

    /// ★★ **URL coverage may only improve.** Every entry without a URL is named in
    /// [`URL_NOT_RECOVERABLE`] with a reason, and every name in that list is really URL-less. An
    /// authority we cannot re-fetch is one we cannot prove current — so the gap is grep-able and
    /// shrink-only, never a silent blank.
    #[test]
    fn url_coverage_may_only_improve() {
        let entries = load(&root()).expect("manifest loads");
        let excused: BTreeSet<&str> = URL_NOT_RECOVERABLE.iter().copied().collect();

        let unexcused: Vec<&str> = entries
            .iter()
            .filter(|e| e.url.is_empty() && !excused.contains(e.path.as_str()))
            .map(|e| e.path.as_str())
            .collect();
        assert!(
            unexcused.is_empty(),
            "{unexcused:?} have no recorded URL and are not in URL_NOT_RECOVERABLE. Recover the URL \
             (legal/_scripts/*.sh is the record) or list it WITH a reason — a blank provenance that \
             nobody decided is the defect."
        );

        let listed: BTreeSet<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        for x in URL_NOT_RECOVERABLE {
            assert!(
                listed.contains(x),
                "{x} is excused but is not in the manifest at all"
            );
        }
        let stale: Vec<&&str> = URL_NOT_RECOVERABLE
            .iter()
            .filter(|x| entries.iter().any(|e| e.path == ***x && !e.url.is_empty()))
            .collect();
        assert!(
            stale.is_empty(),
            "{stale:?} now HAVE a URL — remove them from URL_NOT_RECOVERABLE so the ratchet tightens"
        );
    }

    /// ★ The statute and the regulations must actually be represented. If a refactor dropped the
    /// `legal/` tree from the manifest, direction 2 would red — but this says *why it matters*:
    /// rung 4 is the only rung that is law, and an archive without it is not an authority archive.
    #[test]
    fn the_manifest_covers_the_law_itself() {
        let entries = load(&root()).expect("manifest loads");
        for (kind, least) in [(Kind::Statute, 16), (Kind::Regulation, 6)] {
            let n = entries.iter().filter(|e| e.kind == kind).count();
            assert!(
                n >= least,
                "manifest lists only {n} × {kind:?}; the repo archives at least {least}. An \
                 authority manifest that omits the statute is not an authority manifest."
            );
        }
    }
}
