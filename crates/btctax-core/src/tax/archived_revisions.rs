//! Every archived revision of one IRS form family, **derived from the extract directory**.
//!
//! ## Why this module exists
//!
//! A transcription gate reads an archived extract and asserts the captions it transcribed are
//! verbatim in it. Which revisions it reads decides whether it is an instrument or a restatement, and
//! there are two ways to answer that:
//!
//! ```text
//! for ed in ["2024", "2025"] { … }        // a list beside a set that grows
//! for (year, text) in of("f1099r") { … }  // the set itself
//! ```
//!
//! The first is correct the day it is typed. Archiving `f1099r--2026.txt` — the event the whole
//! year-port exists to handle — widens the set beneath it, the list does not know, **and nothing
//! reds**. Measured 2026-09-20 by planting exactly that: a `f1099r--2026.txt` printing none of the
//! captions `form1099r.rs` transcribes was archived, and the suite ran **3989/3989 green**.
//!
//! Worse, `xtask blockers` reported all four such families as safe, in these words:
//!
//! > `f1099r` is read as a PER-REVISION family: the module derives its revision set from
//! > `design/forms/extract/`, so archiving `f1099r--2026` REDS the suite — it does not silently
//! > clear, and it does not silently pass
//!
//! It classified on the extract path being a `format!` with a `{parameter}`, which proves the path is
//! parameterised and says nothing about where the parameters come from. So the families most likely
//! to absorb a revision silently were the ones the instrument named as protected.
//!
//! ★★ **And the rule was already written in this repo.** `state_local_refund.rs::archived_revisions`
//! carries it verbatim — *"Read out of the extract DIRECTORY, never from a list of years typed
//! here"* — with the T8 defect shape named. Three other modules violated it, because no reviewer
//! ever held two of those files at once. That is the B3 field-of-view failure, not ignorance.
//!
//! ## What it does NOT cover
//!
//! It answers *which revisions exist*. It does not read them: a caller that fetches the set and then
//! asserts nothing still passes. The non-vacuity floor below is the only thing standing between a
//! moved directory and a gate that measures an empty loop.

/// The workspace root, from this crate's manifest directory.
#[cfg(test)]
pub(crate) fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/btctax-core -> workspace root")
        .to_path_buf()
}

/// Every archived revision of `stem`, as `(year, text)`, oldest first.
///
/// Panics if fewer than `floor` revisions are found — a gate looping over an empty or shrunken set
/// reports success while measuring nothing, which is the failure this module is about. Pass the
/// number the caller's assertions actually need to be meaningful.
#[cfg(test)]
pub(crate) fn of(stem: &str, floor: usize) -> Vec<(i32, String)> {
    let dir = repo_root().join("design/forms/extract");
    let prefix = format!("{stem}--");
    let mut out: Vec<(i32, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
        .map(|e| e.expect("a readable directory entry").path())
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_string();
            // ★ `parse()` rather than a length check: this is what excludes `--2026-DRAFT`, which is
            //   an archived draft and not a revision anyone may transcribe against (FR-58).
            let year: i32 = name
                .strip_prefix(&prefix)?
                .strip_suffix(".txt")?
                .parse()
                .ok()?;
            let text = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()));
            Some((year, text))
        })
        .collect();
    out.sort_by_key(|(y, _)| *y);
    assert!(
        out.len() >= floor,
        "only {} archived `{stem}` revision(s) in {}, and the caller needs at least {floor} for its \
         assertions to mean anything. Either the extract directory moved, or the stem is misspelled \
         — both of which make every check below measure an empty loop and report success.",
        out.len(),
        dir.display()
    );
    out
}
