//! A3 — **the primary-source shape detector.** (`design/HARNESS.md` r2, class α.)
//!
//! ★★ **Catches F1**, observed 2026-07-30: `design/forms/` was built as a primary-source archive
//! without checking whether one existed. `legal/primary-sources/` already held 16 × 26 USC, 6 × 26 CFR,
//! 11 guidance documents and 7 forms — including forms that overlap `design/forms/` directly. The
//! memory saying *"before deriving or building, grep for what already exists"* had been written hours
//! earlier, that same day.
//!
//! **One implementation, two call sites** — the whole point, so the rule cannot drift between them:
//!
//! | call site | when | effect |
//! |---|---|---|
//! | [`run`], via the test below | `make check` | reds if a primary source appears in a NEW tree |
//! | `scripts/hooks/on-write.sh` → `xtask classify-path` | at `Write` time | denies the file before it is created |
//!
//! ★★ **SHAPE, NOT PATH — this is the load-bearing rule.** A hand-list of known archive directories
//! would be F2 (*false completeness*) committed **inside the harness**: it passes a new archive at a
//! new location, which is precisely the case it exists to catch. So classification is by *filename
//! shape*, and the shapes below were read off the archives that actually exist rather than invented —
//! the same discipline the label census demands of form lines.
//!
//! ★★★ **It paid for itself on its first run.** `CONTINUITY.md` said there were two archives. The walk
//! found **four**: `design/amt-form6251/` (byte-identical duplicates of `design/forms/2025/`) and
//! `legal/text/` (a 25-file extract layer of `legal/primary-sources/`, 100% overlap) had never been
//! named anywhere. A path hand-list would have found neither.
//!
//! ★ **The shapes are deliberately over-broad.** A false positive costs one ratchet entry with a
//! reason; a false negative is a silent second archive, which is the entire failure. Fail loud.

use std::path::{Path, PathBuf};

/// A recognisable primary-source document shape, named so failures can say *why* a file matched.
pub struct Shape {
    pub name: &'static str,
    pub matches: fn(&str) -> bool,
    /// A real filename from one of the archives, so the shape is anchored to observed data and the
    /// test below can prove the shape still fires.
    pub witness: &'static str,
}

fn ext_is_document(n: &str) -> bool {
    // `.pdf.txt` is the committed text layer, which is as much a primary source as the PDF.
    [".pdf", ".txt", ".html", ".htm", ".xml"]
        .iter()
        .any(|e| n.ends_with(e))
}

/// `f1040--2024.pdf`, `i1040gi--2024.pdf.txt`, `f8949.pdf`, `p550.pdf` — the IRS's own stem
/// convention: `f`/`i`/`p` + at least three digits.
fn irs_stem(name: &str) -> bool {
    if !ext_is_document(name) {
        return false;
    }
    let mut c = name.chars();
    match c.next() {
        Some('f') | Some('i') | Some('p') => {}
        _ => return false,
    }
    c.take_while(|ch| ch.is_ascii_digit()).count() >= 3
}

/// `26USC_s1211.html`, `26CFR_1.1012-1_basis.xml` — statute and regulation, rungs 4 and 3.
fn usc_or_cfr(name: &str) -> bool {
    if !ext_is_document(name) {
        return false;
    }
    let u = name.to_ascii_uppercase();
    (u.contains("USC") || u.contains("CFR")) && u.chars().any(|c| c.is_ascii_digit())
}

/// `Notice_2014-21.pdf`, `RevRul_2019-24.pdf`, `RevProc_2024-28.pdf`, `CCA_202124008.pdf`,
/// `TD_10000_89FR56480_broker_regs.pdf` — sub-regulatory guidance.
fn irs_guidance(name: &str) -> bool {
    if !ext_is_document(name) {
        return false;
    }
    let u = name.to_ascii_uppercase();
    ["NOTICE_", "REVRUL_", "REVPROC_", "CCA_", "TD_", "PUB"]
        .iter()
        .any(|p| u.starts_with(p))
}

/// `Form_8949.pdf`, `Instructions_Schedule_D.pdf`, `Schedule_D_1040.pdf` — the human-readable
/// convention `legal/primary-sources/` uses, which is exactly why shape-matching needs more than the
/// IRS stem: the two archives name the SAME documents differently, and a detector that knew only one
/// convention would see only one archive.
fn human_readable_form(name: &str) -> bool {
    if !ext_is_document(name) {
        return false;
    }
    let u = name.to_ascii_uppercase();
    ["FORM_", "INSTRUCTIONS_", "SCHEDULE_"]
        .iter()
        .any(|p| u.starts_with(p))
}

pub const SHAPES: &[Shape] = &[
    Shape {
        name: "irs-stem",
        matches: irs_stem,
        witness: "f1040--2024.pdf",
    },
    Shape {
        name: "usc-or-cfr",
        matches: usc_or_cfr,
        witness: "26USC_s1211.html",
    },
    Shape {
        name: "irs-guidance",
        matches: irs_guidance,
        witness: "Notice_2014-21.pdf",
    },
    Shape {
        name: "human-readable-form",
        matches: human_readable_form,
        witness: "Form_8949.pdf",
    },
];

/// Classify a filename. `None` = not a primary source.
pub fn classify(name: &str) -> Option<&'static str> {
    SHAPES.iter().find(|s| (s.matches)(name)).map(|s| s.name)
}

/// ★★ **THE RATCHET — it may only SHRINK.**
///
/// Every tree here holds primary-source-shaped files. **Down from FOUR to THREE on 2026-07-30**:
/// `design/amt-form6251/` was retired as an archive — its 8 duplicate form-notes were deleted, its 2
/// unique files moved to `design/forms/2026/`, and it is now purely a design directory. What remains
/// is the (A)/(B) split the hybrid decision deliberately keeps, plus (B)'s text layer.
///
/// ★ **This list deliberately does NOT decide which tree wins.** That is the reconciliation, and it is
/// the owner's call. What the list DOES do is make a **fifth** archive impossible to add silently —
/// which is the forward-looking half of F1, and the half a "go read CONTINUITY.md" note cannot enforce.
///
/// ★★ Why a ratchet rather than a red suite: `design/HARNESS.md` r1 said this check should simply red
/// today. With A2 wired that would block **every commit** until the reconciliation landed, and a gate
/// that stands permanently red is not a gate — it is noise that gets muted, which is the failure mode
/// the whole document is written against. The in-repo model is `AUTHORITY_NOT_YET_ARCHIVED`.
/// ★★★ **MEASURED 2026-07-30 BY THIS CHECK, ON ITS FIRST RUN — and the count was FOUR, not two.**
/// `CONTINUITY.md` §4 recorded "two archives". That number was written from memory rather than from a
/// walk of the tree, which is **F2 (enumerating from a hand-list instead of the source) inside the
/// continuity document itself** — the same defect, one level up, in the very note warning about it.
/// Every entry below was found by `strays()`, not recalled.
pub const KNOWN_ARCHIVES: &[(&str, &str)] = &[
    (
        "design/forms",
        "URL-note + sha256 + committed text layer; machine-checked by `xtask cite-check`. The most \
         complete convention, and the likeliest survivor of the reconciliation.",
    ),
    (
        "legal/primary-sources",
        "Convention (B): 47 binaries COMMITTED (not gitignored), no manifest, no hashes, fetched by \
         legal/_scripts/. Holds the rungs design/forms lacks — 16 × 26 USC (rung 4, THE LAW) and \
         6 × 26 CFR (rung 3) — so it CANNOT simply be deleted.",
    ),
    (
        "legal/text",
        "The TEXT LAYER of legal/primary-sources — 25 real extracts (pdftotext output, ~10 KB each), \
         100% overlap, zero unique documents. Not a separate archive: it is (B)'s equivalent of \
         design/forms/extract/, kept in a parallel tree instead of beside its sources.",
    ),
];

/// ★★★ **The date the four-archive duplication must be re-decided by.**
///
/// The ratchet stops a FIFTH archive; this stops the existing four from becoming permanent furniture.
/// Past this date `the_archive_reconciliation_is_not_past_its_review_by` fails the suite — the same
/// mechanism `AUTHORITY_CONFLICTS.md` uses, for the same reason: a known defect with no deadline is
/// indistinguishable from a forgotten one.
///
/// Set 2026-08-13 (two weeks after the four were discovered on 2026-07-30). Reconciliation is step ③
/// in `CONTINUITY.md` §0, i.e. the very next piece of work after this harness.
pub const ARCHIVE_RECONCILIATION_REVIEW_BY: &str = "2026-08-13";

/// Trees that legitimately hold form-shaped files that are **not** an archive.
pub const NOT_AN_ARCHIVE: &[(&str, &str)] = &[
    (
        "crates/btctax-forms/forms",
        "RUNTIME ASSETS — the fillable AcroForm templates the emitter writes into, plus their .map.toml \
         field maps. Shipped in the crate; not a source of authority.",
    ),
    (
        "crates/btctax-core/src/tax/fixtures",
        "in-crate text-layer fixture read by `include_str!` (which must not escape its crate).",
    ),
];

/// Directories never worth walking.
const SKIP: &[&str] = &[
    ".git",
    // ★★★ A git WORKTREE is a second checkout of the same commit, so every file in it is already
    //     accounted for at its canonical path. Walking it reports the ENTIRE archive as strays — 130+
    //     of them on the first isolated review — which is worse than useless: an alarm that fires on
    //     nothing teaches people to ignore it, and this one exists precisely because a real overlap
    //     went unnoticed. Worktrees are where isolated reviewers run, so this is not a corner case.
    ".claude",
    "target",
    "target-clippy",
    ".venv",
    "node_modules",
    "__pycache__",
];

/// A primary-source-shaped file in a tree that is neither a known archive nor a declared non-archive.
#[derive(Debug)]
pub struct Stray {
    pub path: PathBuf,
    pub shape: &'static str,
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<Stray>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !SKIP.contains(&name.as_str()) {
                walk(&p, root, out);
            }
            continue;
        }
        let Ok(rel) = p.strip_prefix(root) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        let accounted = KNOWN_ARCHIVES
            .iter()
            .chain(NOT_AN_ARCHIVE.iter())
            .any(|(tree, _)| rel_s.starts_with(&format!("{tree}/")));
        if accounted {
            continue;
        }
        if let Some(shape) = classify(&name) {
            out.push(Stray {
                path: rel.to_path_buf(),
                shape,
            });
        }
    }
}

/// Every primary-source-shaped file outside every accounted-for tree.
pub fn strays(root: &Path) -> Vec<Stray> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

pub fn report(strays: &[Stray]) -> String {
    let mut s = String::from(
        "A NEW PRIMARY-SOURCE ARCHIVE is appearing — primary-source-shaped files were found \
         outside every accounted-for tree:\n\n",
    );
    for st in strays {
        // The witness anchors the shape to a real filename, so the message says WHY this matched
        // rather than only that it did.
        let eg = SHAPES
            .iter()
            .find(|sh| sh.name == st.shape)
            .map(|sh| sh.witness)
            .unwrap_or("?");
        s.push_str(&format!(
            "  {}   [shape: {}, like {eg}]\n",
            st.path.display(),
            st.shape
        ));
    }
    s.push_str("\n  Already accounted for:\n");
    for (tree, why) in KNOWN_ARCHIVES {
        s.push_str(&format!("    {tree}/ — {why}\n"));
    }
    for (tree, why) in NOT_AN_ARCHIVE {
        s.push_str(&format!("    {tree}/ — {why}\n"));
    }
    s.push_str(
        "\n  ★ On 2026-07-30 `design/forms/` was built from scratch as a primary-source archive while\n  \
         `legal/primary-sources/` already held the same material — hours after writing down the rule\n  \
         \"before deriving or building, grep for what already exists\". THIS is that check.\n\n  \
         If the file belongs in an existing archive, put it there. If it genuinely needs a new tree,\n  \
         add it to KNOWN_ARCHIVES with a reason — deliberately, not by accident.\n",
    );
    s
}

/// `cargo run -p xtask -- archive-check`
///
/// Reports both halves of the gate: **no NEW archive** (the ratchet) and **the existing four are not
/// past their review-by** (the tickle). The second is what stops "recorded" from quietly meaning
/// "permanent".
pub fn run() -> Result<(), String> {
    let found = strays(&repo_root());
    if !found.is_empty() {
        return Err(report(&found));
    }

    let now = crate::authority_conflicts::today();
    let overdue = ARCHIVE_RECONCILIATION_REVIEW_BY < now.as_str();
    println!(
        "archive-check: no primary source outside the {} accounted-for tree(s)\n\
         archive-check: {} archive(s) pending reconciliation (CONTINUITY.md §0 step ③), \
         review-by {ARCHIVE_RECONCILIATION_REVIEW_BY}{}",
        KNOWN_ARCHIVES.len() + NOT_AN_ARCHIVE.len(),
        KNOWN_ARCHIVES.len(),
        if overdue { "  ** OVERDUE **" } else { "" }
    );
    if overdue {
        return Err(format!(
            "the four-archive reconciliation is PAST its review-by \
             ({ARCHIVE_RECONCILIATION_REVIEW_BY}, today {now}). Re-decide it — retire a tree, or set \
             a new date with a reason. Do NOT simply push the date out."
        ));
    }
    Ok(())
}

/// `cargo run -p xtask -- classify-path <path>` — the hook's call site.
///
/// Prints the tree the path would belong to, or `ok` if it is not a primary source / is already inside
/// an accounted-for tree. Exit 1 (via `Err`) means **deny the write**.
pub fn classify_path(raw: &str) -> Result<(), String> {
    let root = repo_root();
    let p = PathBuf::from(raw);
    let rel = p.strip_prefix(&root).unwrap_or(&p);
    let rel_s = rel.to_string_lossy().replace('\\', "/");
    let name = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let Some(shape) = classify(&name) else {
        println!("ok — not a primary-source shape");
        return Ok(());
    };
    for (tree, _) in KNOWN_ARCHIVES.iter().chain(NOT_AN_ARCHIVE.iter()) {
        if rel_s.starts_with(&format!("{tree}/")) {
            println!("ok — inside accounted-for tree {tree}/");
            return Ok(());
        }
    }
    Err(format!(
        "`{rel_s}` matches primary-source shape `{shape}` but is outside every accounted-for tree.\n\n\
         {}\n  Put it in an existing archive, or extend KNOWN_ARCHIVES deliberately.",
        KNOWN_ARCHIVES
            .iter()
            .chain(NOT_AN_ARCHIVE.iter())
            .map(|(t, w)| format!("    {t}/ — {w}\n"))
            .collect::<String>()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// ★★ **The shapes must actually fire on the archives that exist.** Anchored to a witness filename
    /// per shape, so a refactor that quietly broke `irs_stem` could not leave this passing — which is
    /// F4 (a checker blind to the case it was written for) in its natural habitat.
    #[test]
    fn every_shape_fires_on_its_witness() {
        for s in SHAPES {
            assert_eq!(
                classify(s.witness),
                Some(s.name),
                "shape `{}` no longer matches its own witness `{}` — the detector has gone blind",
                s.name,
                s.witness
            );
        }
    }

    /// The negative half. Without it a `classify` returning `Some` unconditionally would pass the test
    /// above and flag every file in the repo.
    #[test]
    fn ordinary_repo_files_are_not_primary_sources() {
        for name in [
            "main.rs",
            "Cargo.toml",
            "README.md",
            "CONTINUITY.md",
            "lib.rs",
            "MANIFEST.json",
            "schedule_d.map.toml",
            "pre-commit",
            "settings.json",
            "LABEL_READER.md",
        ] {
            assert_eq!(
                classify(name),
                None,
                "`{name}` must not classify as a primary source"
            );
        }
    }

    /// ★★ **THE KILL — a third archive is caught wherever it is put.** This is the F1 recurrence, and
    /// it is planted at a path nobody has thought of, because a path hand-list would pass it.
    #[test]
    fn a_third_archive_at_a_novel_path_is_caught() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        // A clean tree with only the accounted-for trees populated.
        for (tree, _) in KNOWN_ARCHIVES.iter().chain(NOT_AN_ARCHIVE.iter()) {
            let d = root.join(tree);
            fs::create_dir_all(&d).expect("mkdir");
            fs::write(d.join("f1040--2024.pdf"), "x").expect("write");
        }
        assert!(
            strays(root).is_empty(),
            "files inside accounted-for trees must not be flagged: {:?}",
            strays(root)
        );

        // Now the failure: a new archive somewhere entirely different. Each of the four shapes, at a
        // path no hand-list would contain.
        let novel = root.join("docs/authority/irs");
        fs::create_dir_all(&novel).expect("mkdir");
        for s in SHAPES {
            fs::write(novel.join(s.witness), "x").expect("write");
        }
        let found = strays(root);
        assert_eq!(
            found.len(),
            SHAPES.len(),
            "every shape must be caught at a novel path — got {found:?}"
        );
    }

    /// ★★ **THE LIVE GATE.** This repository must have no primary source outside the accounted-for
    /// trees. It passes today, and it reds the moment a third archive starts.
    #[test]
    fn this_repo_has_no_unaccounted_primary_source() {
        let found = strays(&repo_root());
        assert!(found.is_empty(), "{}", report(&found));
    }

    /// ★ The ratchet's entries must be real. An archive listed here that no longer exists is a stale
    /// excuse, and a stale excuse is how a closed gap silently reopens.
    #[test]
    fn every_accounted_for_tree_still_exists() {
        let root = repo_root();
        for (tree, _) in KNOWN_ARCHIVES.iter().chain(NOT_AN_ARCHIVE.iter()) {
            assert!(
                root.join(tree).is_dir(),
                "`{tree}/` is listed as an accounted-for tree but does not exist — remove it so the \
                 ratchet tightens"
            );
        }
    }

    /// ★★★ **THE TICKLE — the answer to "does the ratchet neuter the gate?"**
    ///
    /// It partly does, and this is the part that puts the pressure back. `design/HARNESS.md` r1 said
    /// this check should stand RED until the archives were reconciled. Turning it green behind a
    /// ratchet keeps a **new** archive impossible — proven, a fifth one reds `strays()` and the Write
    /// hook denies it — but it removes all pressure on the **four that already exist**, and a recorded
    /// defect with no deadline is how step ③ sits untouched for a month.
    ///
    /// So the duplication carries a **date**, on the same mechanism `AUTHORITY_CONFLICTS.md` uses for
    /// a believed statute/reg disagreement: past `review-by`, **the suite fails**. Neglecting the
    /// reconciliation stays a legitimate choice — it is real work and the owner may have better uses
    /// for the time — but it must be *a choice*, dated and revisited, never an omission nobody made.
    ///
    /// ★ **Do not simply push the date out.** That converts the tickle into the decoration it exists
    /// to prevent. Re-decide, and record what changed.
    #[test]
    fn the_archive_reconciliation_is_not_past_its_review_by() {
        let now = crate::authority_conflicts::today();
        assert!(
            ARCHIVE_RECONCILIATION_REVIEW_BY >= now.as_str(),
            "The four-archive reconciliation (CONTINUITY.md §0 step ③) is PAST its review-by \
             ({ARCHIVE_RECONCILIATION_REVIEW_BY}, today {now}).\n\n{}\n\
             Re-decide it: retire a tree, or set a new date WITH a reason. The whole value of the \
             date is that it is not silently extended.",
            KNOWN_ARCHIVES
                .iter()
                .map(|(t, w)| format!("    {t}/ — {w}\n"))
                .collect::<String>()
        );
    }

    /// ★★ **The duplication is RECORDED, not forgotten — and the recorded number is MEASURED.**
    ///
    /// **THREE** archives is the post-reconciliation state (A `design/forms/` + B
    /// `legal/primary-sources/` + B's text layer `legal/text/`). It was FOUR before step ③; the
    /// stray `design/amt-form6251/` notes were retired, and the doc here still narrated four long
    /// after the tree stopped having four — corrected 2026-07-31.
    ///
    /// ★ The number is a MEASUREMENT (a walk of the tree found it), never a plan. Lowering it is the
    /// deliverable; raising it is what A3 exists to prevent.
    ///
    /// ★★ **TWO-SIDED, and the second side is the point.** The old assertion was `<= 3` alone, which
    /// catches a new archive but is SILENT when one is retired — exactly the direction this ratchet
    /// exists to reward. A retired tree would leave a stale `3` here reading as "still three to go",
    /// and `every_accounted_for_tree_still_exists` would red on the *path* while this said nothing
    /// about the *count*. `assert_eq!` makes the constant come down with the tree, which is what its
    /// two siblings (`the_two_lists_partition_every_form`, the duplicate-group pin) already do.
    #[test]
    fn the_archive_count_may_only_shrink() {
        assert!(
            KNOWN_ARCHIVES.len() <= 3,
            "KNOWN_ARCHIVES has grown to {} — a NEW archive is exactly what A3 exists to prevent. \
             Three is the post-reconciliation state (A + B + B's text layer); the only legitimate \
             direction for this number is DOWN.",
            KNOWN_ARCHIVES.len()
        );
        assert_eq!(
            KNOWN_ARCHIVES.len(),
            3,
            "KNOWN_ARCHIVES is {} — if an archive was RETIRED, lower this constant in the same commit. \
             A shrink that leaves the pin at 3 is progress the ratchet cannot see, and the next reader \
             is told there are still three trees to reconcile when there are not.",
            KNOWN_ARCHIVES.len()
        );
    }

    /// The hook's call site, both directions.
    #[test]
    fn classify_path_denies_outside_and_allows_inside() {
        assert!(classify_path("design/forms/2024/f1040--2024.pdf").is_ok());
        assert!(classify_path("crates/btctax-forms/forms/2025/f8949.pdf").is_ok());
        assert!(classify_path("crates/xtask/src/main.rs").is_ok());
        assert!(
            classify_path("docs/authority/f6251--2025.pdf").is_err(),
            "a primary source at a novel path must be DENIED at write time"
        );
        assert!(classify_path("notes/26USC_s55.html").is_err());
    }
}
