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

/// `f1040--2024.pdf`, `i1040gi--2024.pdf.txt`, `f8949--2025.pdf` — the IRS's own stem
/// convention: `f`/`i`/`p` + at least three digits, **or** the W-series spelling `f`/`i` + `w` +
/// at least one digit (`fw2--2025.pdf`, `iw2w3--2025.pdf`).
///
/// ★ Same standard as `human_readable_form` below: these must be files the repo actually HOLDS.
/// `f8949.pdf` and `p550.pdf` used to be listed here and are in no tree — note that the `p`
/// arm is still load-bearing for the *shape* (it matches `Pub525_…` style stems only via
/// `irs_guidance`'s `PUB` prefix, not here), so it is kept, not pruned.
///
/// ★★★ **THE W-SERIES ARM WAS ADDED 2026-09-06, AND IT WAS A SILENT HOLE.** The IRS does not number
/// every form `NNNN`: the wage and withholding series is `W-2`, `W-3`, `W-4`, `W-9`, and their
/// filenames are `fw2`, `iw2w3`, `fw9`. Under the digits-only rule `classify("fw2--2025.pdf")`
/// returned `None`, so `collect_sources` skipped the file entirely — the W-2 and its instructions
/// could be fetched, noted, extracted and committed, and **`authority-manifest` would still print
/// "OK — every entry resolves and every source is listed"** while listing neither. Measured on the
/// first regen of the interview T2 archive: 14 documents added to the tree, **12** reached the
/// manifest. That is precisely the "archived but never recorded" half the manifest census exists to
/// catch, defeated by the shape detector one layer below it.
///
/// ★ The arm is deliberately narrow — `w` then **at least one digit**, so `fwd.txt` or `index.html`
/// still classify as `None`. See `the_w_series_stems_are_primary_sources`.
fn irs_stem(name: &str) -> bool {
    if !ext_is_document(name) {
        return false;
    }
    let mut c = name.chars();
    match c.next() {
        Some('f') | Some('i') | Some('p') => {}
        _ => return false,
    }
    let rest: Vec<char> = c.collect();
    if rest.iter().take_while(|ch| ch.is_ascii_digit()).count() >= 3 {
        return true;
    }
    // The W-series: `fw2`, `iw2w3`, `fw9`. `w` followed by at least one digit.
    matches!(rest.first(), Some('w')) && rest.get(1).is_some_and(char::is_ascii_digit)
}

/// ★★★ **Is this stem one of the W / 1098 / 1099 INFORMATION RETURNS?** — the series the interview
/// transcribes boxes from, and the join `box_census` derives its censused document set through.
///
/// ★ It is a reading of how the IRS numbers the series, not a list of the seven documents we happen
/// to hold: strip the `f`/`i` prefix, then the remainder is the wage series (`w` + a digit: `fw2`,
/// `iw2w3`) or the 1098/1099 series (`f1098e`, `i1099int`). A hand-list would have to be edited to
/// admit the 1099-NEC, 1099-R or 1099-DA, and forgetting to is exactly the silence I2 found — the
/// review deleted a whole archived form from the census and every test still reported success.
#[must_use]
pub fn is_information_return_stem(stem: &str) -> bool {
    let rest = match stem.as_bytes().first() {
        Some(b'f') | Some(b'i') => &stem[1..],
        _ => return false,
    };
    rest.starts_with("1098")
        || rest.starts_with("1099")
        || (rest.starts_with('w') && rest[1..].starts_with(|c: char| c.is_ascii_digit()))
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

/// `SSA_COLA_Determinations_2026.pdf` — a **Social Security Administration** determination published
/// in the Federal Register, which is where the OASDI contribution-and-benefit base (the SS wage base)
/// is actually set.
///
/// ★★ **Its own shape rather than a sixth prefix on `irs_guidance`, because it is not IRS guidance.**
/// The SS wage base is the one figure in `tax_tables.rs` that no revenue procedure contains — the IRS
/// procedures set brackets and breakpoints, the SSA sets the wage base — so the document class is
/// genuinely different and a reader who found it filed under "irs-guidance" would be misled about
/// which agency's determination governs.
///
/// ★ Added 2026-09-05 when four of these were archived to close the shipped-tables ratchet. Before
/// that, `classify` returned `None` for them and the manifest walker skipped them SILENTLY: the
/// files sat in an accounted-for tree, unrecorded, and the entry count simply did not move. That is
/// the shape of a gap this registry exists to make impossible.
fn ssa_determination(name: &str) -> bool {
    ext_is_document(name) && name.to_ascii_uppercase().starts_with("SSA_")
}

/// `PLAW-119publ21_OBBBA.pdf` — a **Public Law** (session law) from govinfo's `PLAW-` package
/// series: the AMENDING text of a statute, as distinct from the codified section it amends.
///
/// ★★ Its own shape, not a third keyword on `usc_or_cfr`, because a Public Law is not a codified
/// section: it is what CHANGED the section, and a reader who only held the USCODE-2024 granule of
/// §55 would read a pre-OBBBA phase-out rate as current law. The two documents answer different
/// questions ("what does §55 say?" vs "what did Pub. L. 119-21 do to it, and from when?").
///
/// ★ Added 2026-09-05 with the first one (FR-47). Before this shape, `classify` returned `None`
/// for it and `authority_manifest`'s walker would have skipped it SILENTLY — the same gap the
/// `ssa-determination` shape closed for one document class. `every_document_under_primary_sources_has_a_shape`
/// now closes the CLASS: a new kind of document cannot be archived unshaped.
fn public_law(name: &str) -> bool {
    ext_is_document(name) && name.to_ascii_uppercase().starts_with("PLAW-")
}

/// `Form_1099-DA.pdf`, `Instructions_1099-DA.pdf` — the human-readable convention
/// `legal/primary-sources/` uses, which is exactly why shape-matching needs more than the IRS stem:
/// the two archives name the SAME documents differently, and a detector that knew only one
/// convention would see only one archive.
///
/// ★★ **These examples must be documents the repo actually HOLDS.** They used to read
/// `Form_8949.pdf` / `Instructions_Schedule_D.pdf` / `Schedule_D_1040.pdf`, all three of which were
/// deleted in the 2026-09-04 reconciliation — leaving the shape's own illustration pointing at
/// nothing, in the instrument whose failure message is "the detector has gone blind".
/// ★ `FR-23` schedules the two 1099-DA files for retirement too. When that lands, this shape has no
/// subject left in the tree and the honest move is to retire the shape with it — NOT to leave a
/// witness that only proves a string literal matches a string matcher.
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
        name: "ssa-determination",
        matches: ssa_determination,
        witness: "SSA_COLA_Determinations_2026.pdf",
    },
    Shape {
        name: "public-law",
        matches: public_law,
        witness: "PLAW-119publ21_OBBBA.pdf",
    },
    Shape {
        name: "human-readable-form",
        matches: human_readable_form,
        // ★ Must be a file the repo still HOLDS — see the shape's doc comment. `Form_8949.pdf`
        //   was the witness until 2026-09-04 deleted it.
        witness: "Form_1099-DA.pdf",
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
        "Convention (B): 42 binaries COMMITTED (not gitignored), no manifest, no hashes, fetched by \
         legal/_scripts/. Holds the rungs design/forms lacks — 17 × 26 USC + 1 Public Law + 1 OLRC prelim (rung 4, THE \
         LAW) and 6 × 26 CFR (rung 3) — so it CANNOT simply be deleted.",
    ),
    (
        "legal/text",
        "The TEXT LAYER of legal/primary-sources — 20 real extracts (pdftotext output, ~10 KB each), \
         100% overlap, zero unique documents. Not a separate archive: it is (B)'s equivalent of \
         design/forms/extract/, kept in a parallel tree instead of beside its sources.",
    ),
];

// ★★★ **RETIRED 2026-09-04 — `ARCHIVE_RECONCILIATION_REVIEW_BY` and its test are gone, because
// their SUBJECT is gone.** This block is the record; it documents no item because there is no
// longer an item to document.
//
// The constant carried a date by which the archive duplication had to be re-decided, and past it
// `the_archive_reconciliation_is_not_past_its_review_by` failed the suite — the same mechanism
// `AUTHORITY_CONFLICTS.md` uses, for the same reason: a known defect with no deadline is
// indistinguishable from a forgotten one. Set 2026-08-13, reset twice, discharged 2026-09-04 when
// both residual duplicate groups were resolved (`DUPLICATE_SOURCE_GROUPS` 7 → 0).
//
// ★★ **Why deleting it is not the mute the doc warned against.** The warning was against pushing
// the DATE out while the duplication stood — converting a deadline into decoration. Here the
// duplication was retired first, and the guard that replaces the date is strictly stronger:
// `authority_manifest::duplicate_source_groups_may_only_shrink` is pinned at **0** and reds on any
// duplicate the instant one appears, with no date for anyone to renew; `strays()` plus
// `the_archive_count_may_only_shrink` still red on a new tree. A dated test whose subject no longer
// exists is itself the decoration — so retiring it applies the rule rather than evading it.
//
// ★ The RESET LOG is kept verbatim below. Its whole point was visibility without reading git
// history, and that is worth more now than it was while the gate was live: it is the only place the
// two extensions, and the reason each was granted, can be read at a glance.
//
// ## ★★★ RESET LOG — every extension, who decided it, and why
//
// **The log exists because a date nobody records re-pushing is not a deadline, it is furniture.**
// The gate's whole claim is that the four archives cannot become permanent by inattention; a reset
// is a legitimate move, but an *unlogged* reset defeats it silently and the next reader cannot tell
// a considered deferral from a reflex. Two entries here is a decision; five is the gate being
// routed around, and that must be visible without reading git history.
//
// | # | from | to | date | who | why |
// |---|---|---|---|---|---|
// | 1 | 2026-08-13 | 2026-08-28 | 2026-08-20 | owner | The gate fired as designed and blocked all commits (pre-commit runs `make check`). The owner deferred the reconciliation itself to a window when **model usage is expected to be more available** — the remaining work is two genuine decisions, not cleanup, and neither should ride along behind other work. Nothing about the duplication changed; only when it is decided. |
// | 2 | 2026-08-28 | 2026-09-11 | 2026-09-04 | owner | **DECIDED, not deferred — and the last entry.** Both residual groups resolve and the tickle retires with them. Group A: retire `design/forms/periodic/` — no code resolves through it, its notes name a text layer that does not exist (`extract/f8275.txt`), its `irs-pdf/` URLs are the moving ones, and the year directory already holds a revision it cannot (`extract/f8283--2024.txt` = Rev. 12-2023 vs `periodic/f8283.pdf` = Rev. 12-2025). Group B: delete (B)'s five `irs-forms/` form copies per the hybrid rule (forms are note+sha256 in (A)); the only reader of `legal/text/irs-forms/` is MANIFEST.json's own `extract` field, regenerated here, and its instruction extracts are `-layout` column-interleaved — the wrong text layer to transcribe from. What changed since #1: the window it waited for has arrived (nothing in flight since the 2026-08-30 push). This date is the EXECUTION deadline for landing that diff; when it lands, `DUPLICATE_SOURCE_GROUPS` is 0 and this constant, its test and the `run()` branch are deleted, leaving this table as the record. There is no #3. |
//
//
// ★★ **ROW 2 IS DISCHARGED.** It is written in the future tense because it was written before the
// work; everything it promised did happen, on 2026-09-04, four commits later and seven days
// before its own deadline. Read it as the record of a decision taken, not a plan outstanding.
//
// ★ What the two extensions bought, settled: Group A retired `design/forms/periodic/`; Group B
// deleted (B)'s five `irs-forms/` form copies (905,833 bytes) per the hybrid rule. The full
// reasoning, and the shape that will legitimately red the pin one day, is on
// `authority_manifest::DUPLICATE_SOURCE_GROUPS`.

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

/// Every DOCUMENT-extension file under `tree` that no shape claims — the files the manifest walker
/// would drop on the floor. Extract layers are not documents in this sense and are not walked here.
pub fn unshaped(tree: &Path) -> Vec<PathBuf> {
    fn go(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                go(&p, out);
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if ext_is_document(&name) && classify(&name).is_none() {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    go(tree, &mut out);
    out.sort();
    out
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
/// Reports the ratchet: **no NEW archive**. The tickle that once formed its second half was retired
/// 2026-09-04 with its subject — duplicates are now pinned at 0 by `authority-manifest`, which reds
/// on any duplicate immediately rather than on a date somebody must renew.
pub fn run() -> Result<(), String> {
    let found = strays(&repo_root());
    if !found.is_empty() {
        return Err(report(&found));
    }

    println!(
        "archive-check: no primary source outside the {} accounted-for tree(s)\n\
         archive-check: {} accounted-for archive(s) — hybrid, decided 2026-07-30; duplicates \
         reconciled 2026-09-04 and pinned at 0 by `authority-manifest`",
        KNOWN_ARCHIVES.len() + NOT_AN_ARCHIVE.len(),
        KNOWN_ARCHIVES.len(),
    );
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

    /// ★★ **The class-closing guard: nothing under `legal/primary-sources/` may be unshaped.** The
    /// manifest walker (`authority_manifest.rs`) keeps only files `classify` claims, so an archived
    /// document no shape matches is not "unclassified" — it is ABSENT from the manifest, unhashed,
    /// unrecorded, and the entry count simply does not move. That happened once (four SSA notices,
    /// 2026-09-05) and was fixed by adding a shape; this test is what makes the NEXT new document
    /// class red instead of vanish.
    #[test]
    fn every_document_under_primary_sources_has_a_shape() {
        let found = unshaped(&repo_root().join("legal/primary-sources"));
        assert!(
            found.is_empty(),
            "archived documents that NO shape claims — the manifest walker would skip these silently; \
             add a `Shape` (with a witness) rather than renaming the file to fit one: {found:?}"
        );
    }

    /// B1 — the guard above observed RED on a planted defect: a document-extension file whose name
    /// fits no shape, inside an accounted-for tree.
    #[test]
    fn an_unshaped_document_in_an_accounted_tree_is_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tree = dir.path().join("legal/primary-sources/statute-irc");
        fs::create_dir_all(&tree).expect("mkdir");
        fs::write(tree.join("26USC_s55.html"), "x").expect("write"); // shaped: usc-or-cfr
        fs::write(tree.join("Session_Law_2025.pdf"), "x").expect("write"); // unshaped
        let found = unshaped(&dir.path().join("legal/primary-sources"));
        assert_eq!(
            found.len(),
            1,
            "exactly the unshaped document must be reported, not its shaped neighbour: {found:?}"
        );
        assert!(found[0].ends_with("Session_Law_2025.pdf"));
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

    /// ★★★ **THE KILL for the W-series arm (B1, seen red on the un-widened matcher).** Planted by
    /// reverting `irs_stem` to `f`/`i`/`p` + three digits, this test reds with
    /// *"`fw2--2025.pdf` is an IRS primary source (the W-2), left None: expected Some(\"irs-stem\")"*
    /// — and it is not decorative, because with the arm missing the whole W-2 pair is invisible to
    /// `collect_sources`, so `authority-manifest` prints OK on an archive that lists neither.
    ///
    /// ★ Both halves are asserted in one test on purpose: the positive half alone is satisfied by a
    /// matcher that returns `Some` for everything, and the negative half alone is satisfied by the
    /// old digits-only matcher. Neither one, on its own, watches the instrument *discriminate*.
    #[test]
    fn the_w_series_stems_are_primary_sources() {
        for name in [
            "fw2--2025.pdf",
            "fw2--2025.pdf.txt",
            "iw2w3--2025.pdf",
            "iw2w3--2025.pdf.txt",
            "fw9.pdf",
        ] {
            assert_eq!(
                classify(name),
                Some("irs-stem"),
                "`{name}` is an IRS primary source (the W-series); a `None` here makes it invisible \
                 to the manifest census while `authority-manifest` still prints OK"
            );
        }
        // The arm must not swallow ordinary `f*`/`i*` documents that carry no IRS stem.
        for name in [
            "forward.txt",
            "index.html",
            "issues.txt",
            "fw.pdf",
            "iw.pdf",
        ] {
            assert_eq!(
                classify(name),
                None,
                "`{name}` must not classify as a primary source — the W arm requires a digit"
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
        // ★ Two assertions here, versus the single one in
        //   `authority_manifest::duplicate_source_groups_may_only_shrink`. Both are deliberate and
        //   the difference is the PIN, not taste: this one pins 3, where a rise and a fall are both
        //   representable, so the pair buys two tailored messages at no cost in vacuity. That one
        //   pins 0, where `usize` makes a fall unrepresentable — so a `<=` half there is
        //   always-true and was removed as dead weight. Read them together; neither is the "right"
        //   convention on its own.
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
