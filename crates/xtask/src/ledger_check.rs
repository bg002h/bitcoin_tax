//! `FOLLOWUPS.md` vs the source tree: does the ledger still call something open that the code says is
//! fixed?
//!
//! ★★★ **Why this exists, measured 2026-09-14.** The ledger advertised **six open CRITICALs** and every
//! one of them was already fixed — only the headers were stale. FR-29, FR-30, FR-31, FR-32, FR-33 and
//! FR-102, each closed in code, each still reading as a live funds-safety defect on the shelf a reader
//! goes to when asking *"what is unsafe right now?"*
//!
//! That is the *instrument that cries wolf* failure applied to the artifact whose whole job is to say
//! what is unsafe, and it is not hypothetical harm: it produced a wrong recommendation about what to
//! work next, and two of the six claim the defect **understates tax** (FR-31 doubles basis, FR-32
//! fabricates a loss). Acting on a stale one means editing correct funds-sensitive code with no defect
//! to guide you.
//!
//! ★★ **The signal, and why it is this one.** Closure markers inside an entry's body do not work — they
//! sit anywhere in a 40-line entry, and a sibling's tick reads as the parent's. What *did* find all six
//! is a **cross-artifact disagreement**: the ledger calls an entry open and gating, while
//! `crates/**/*.rs` cites its `FR-<n>` by name. Code that names a follow-up usually names it because it
//! *did something about it*.
//!
//! ★ **What this does NOT claim**, stated here rather than discovered later:
//!
//! 1. It is **not** proof of closure. A source comment may cite an `FR-<n>` as a *pending hazard* — all
//!    six entries pinned in [`EXPECTED_CITED_AND_OPEN`] are exactly that shape. So this check reds on a
//!    **change** to the disagreement set, not on disagreement itself.
//! 2. It reads only `FR-<n>` identifiers. An entry with no `FR-` number, or one whose fix cites it in
//!    prose instead of code, is invisible here.
//! 3. It cannot tell a fix from a mention. Adjudicating a newly-flagged entry means reading the source
//!    site — which is the work this check exists to *trigger*, not to replace.

use std::collections::BTreeSet;

/// The entries that are legitimately **open** while the source cites their `FR-<n>` — the residue this
/// check must not red on, with the reason each is here.
///
/// ★ Every row is a TY2026-port item whose source citation is a *warning about the port*, not a fix. A
/// new arrival is the thing to look at; these six were each read and confirmed on 2026-09-14.
///
/// ★★ A row LEAVES this list when its entry is closed — that is the point. It is a pin on a shrinking
/// residue, not a permanent exemption, and it must never be widened to quiet a red without reading the
/// source site the way the six below were read.
pub const EXPECTED_CITED_AND_OPEN: &[(&str, &str)] = &[
    (
        "FR-162",
        "a revision axis for the compute-side transcription structs — the source names the axis it does \
         not yet have; owning phase is AFTER the first TY2026 port, deliberately",
    ),
    (
        "FR-185",
        "six line-number collisions on TY2026 Schedule A — cited where the collisions are, as a hazard \
         for whoever wires that form",
    ),
    (
        "FR-186",
        "the §68 gate's $384,350 already exists in the codebase as a DIFFERENT quantity — cited at the \
         other quantity precisely so nobody reuses it",
    ),
    (
        "FR-214",
        "Form 6251's TY2026 text cites \"Form 1040 line 7a\" — cited as the only in-tree evidence for a \
         renumbering nobody can confirm until the TY2026 1040 exists (FR-181)",
    ),
    (
        "FR-219",
        "Schedule A line 5e's SALT cap and phase-out both move for TY2026 on a line number that did not \
         — cited at the line as a trap",
    ),
    (
        "FR-250",
        "the golden corpus has no retirement or charitable household — cited by `tax::public_vectors` \
         as the REASON that module exists (a published corpus supplies the witness the goldens cannot), \
         not as a fix. The corpus gap itself is untouched and still open",
    ),
    (
        "FR-220",
        "Schedule 1 line 14's eligibility widened to the intelligence community — cited at the line, \
         pending the port",
    ),
];

/// True if a ledger header carries a closure marker, i.e. is not claiming to be open.
#[must_use]
pub fn header_is_closed(header: &str) -> bool {
    header.contains('✅')
        || header.contains('⏳')
        || header.contains("CLOSED")
        || header.contains("RECORDED + MITIGATED")
}

/// True if a ledger header claims a **gating** severity — the only ones whose staleness misleads about
/// safety. A stale Minor costs a reader a moment; a stale Critical costs a recommendation.
#[must_use]
pub fn header_is_gating(header: &str) -> bool {
    let h = header.to_ascii_lowercase();
    h.contains("critical") || h.contains("important")
}

/// The `FR-<n>` identifier a ledger header is about, if it has one.
#[must_use]
pub fn header_fr(header: &str) -> Option<String> {
    let i = header.find("FR-")?;
    let rest = &header[i + 3..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (!digits.is_empty()).then(|| format!("FR-{digits}"))
}

/// Every `FR-<n>` whose ledger header is **open and gating**.
///
/// ★ Top-level entries only (a line starting `- **`). A nested sub-bullet is part of its parent's
/// argument, not an entry with its own severity.
#[must_use]
pub fn open_gating_frs(followups: &str) -> BTreeSet<String> {
    followups
        .lines()
        .filter(|l| l.starts_with("- **"))
        .filter(|l| !header_is_closed(l) && header_is_gating(l))
        .filter_map(header_fr)
        .collect()
}

/// Every `FR-<n>` cited anywhere in the given source text.
#[must_use]
pub fn frs_cited_in(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let b = source.as_bytes();
    let mut i = 0;
    while let Some(p) = source[i..].find("FR-") {
        let start = i + p;
        let digits: String = source[start + 3..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if !digits.is_empty() {
            // ★ Reject `FR-12x`-style false hits by requiring the match to end at a non-digit, which
            //   `take_while` already guarantees, and to start at a word boundary.
            let boundary = start == 0 || !b[start - 1].is_ascii_alphanumeric();
            if boundary {
                out.insert(format!("FR-{digits}"));
            }
        }
        i = start + 3;
    }
    out
}

/// The disagreement: open-and-gating in the ledger, yet cited by name in the source.
#[must_use]
pub fn disagreements(followups: &str, source: &str) -> BTreeSet<String> {
    let cited = frs_cited_in(source);
    open_gating_frs(followups)
        .into_iter()
        .filter(|fr| cited.contains(fr))
        .collect()
}

/// Report the ledger/source disagreement set, and fail if it has MOVED from [`EXPECTED_CITED_AND_OPEN`].
///
/// ★ The same assertion the test makes, exposed as a command so it can be run and read directly — the
/// repo's other checkers (`cite-check`, `archive-check`) work the same way. A checker only a test can
/// reach is harder to consult when a red needs adjudicating.
pub fn run() -> Result<(), String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .ok_or("crates/xtask -> repo root")?
        .to_path_buf();
    let followups = std::fs::read_to_string(root.join("FOLLOWUPS.md"))
        .map_err(|e| format!("FOLLOWUPS.md: {e}"))?;

    let mut source = String::new();
    let mut stack = vec![root.join("crates")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                if !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("target")
                {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|x| x == "rs") {
                if let Ok(t) = std::fs::read_to_string(&path) {
                    source.push_str(&t);
                    source.push('\n');
                }
            }
        }
    }

    let got = disagreements(&followups, &source);
    let want: BTreeSet<String> = EXPECTED_CITED_AND_OPEN
        .iter()
        .map(|(fr, _)| (*fr).to_string())
        .collect();

    println!(
        "ledger-check: {} entry/entries are open+gating in FOLLOWUPS.md while the source cites them by \
         name; {} expected",
        got.len(),
        want.len()
    );
    for fr in &got {
        let why = EXPECTED_CITED_AND_OPEN
            .iter()
            .find(|(p, _)| p == fr)
            .map_or("★ NOT EXPECTED — read the source site", |(_, w)| *w);
        println!("  {fr}: {why}");
    }
    println!(
        "ledger-check — WHAT THIS DOES NOT CLAIM: a source citation is not proof of closure (a comment \
         may name a PENDING hazard), only `FR-<n>` identifiers are read, and an entry whose fix is \
         recorded only in prose is invisible here."
    );

    if got == want {
        println!("ledger-check: OK — the disagreement set is exactly the expected residue");
        return Ok(());
    }
    let new: Vec<_> = got.difference(&want).cloned().collect();
    let gone: Vec<_> = want.difference(&got).cloned().collect();
    Err(format!(
        "the disagreement set MOVED. New: {new:?} — the code cites these while the ledger still calls \
         them open and gating; read each source site and either mark the header closed or pin the row \
         with its reason. Closed: {gone:?} — delete those rows from EXPECTED_CITED_AND_OPEN."
    ))
}

#[cfg(test)]
mod tests {

    /// ★★★ **THE TEETH for the four published acceptance vectors: FR-255 step 2 must stay OPEN while
    /// they are unconsumed.**
    ///
    /// `btctax_core::tax::public_vectors` holds four TY2024 vectors transcribed from TaxCalcBench — the
    /// only independent witness the retirement feature has, since `FR-250` records that the 107-household
    /// golden corpus contains no retirement income at all and `C-3` showed the two oracles AGREE while
    /// both wrong on the MFS-lived-with branch.
    ///
    /// T14.5 added Form 1040 lines 4a/4b/5a/5b, which retired that module's original tripwire (*"reds the
    /// moment the retirement lines exist"*). Its instruction — drive each case, compare the printed lines
    /// — cannot be followed yet: the corpus's raw inputs are not committed, so building inputs that
    /// reproduce the expected outputs would derive the expectation from the thing under test (FR-230).
    /// The MeF→`ReturnInputs` translator is FR-255 step 2.
    ///
    /// ★★ This test lives in `xtask` because `btctax-core` cannot read `FOLLOWUPS.md`: an `include_str!`
    /// escaping the crate root ships a broken crates.io tarball with exit 0, and
    /// `repo_hygiene::no_published_crate_includes_a_file_outside_its_own_root` refuses it. The crate that
    /// already reads the ledger is the one that should hold a claim about the ledger.
    #[test]
    fn fr255_step_2_is_open_while_these_vectors_are_unconsumed() {
        let ledger =
            std::fs::read_to_string(repo_root().join("FOLLOWUPS.md")).expect("FOLLOWUPS.md");
        let vectors = std::fs::read_to_string(
            repo_root().join("crates/btctax-core/src/tax/public_vectors.rs"),
        )
        .expect("public_vectors.rs");

        // The lines exist, so the vectors are no longer waiting on them.
        let printed =
            std::fs::read_to_string(repo_root().join("crates/btctax-core/src/tax/printed.rs"))
                .expect("printed.rs");
        assert!(
            printed.contains("pub line4b: Usd,"),
            "Form 1040 line 4b is gone from printed.rs — this test's premise has changed"
        );

        // ★ "Unconsumed" is checked structurally: no test anywhere drives a vector against a computed
        //   line. The marker is the vectors module naming the compute, which a real KAT would have to do.
        let consumed = vectors.contains("form1099r::line_4b")
            || vectors.contains("form1099r::line_5b")
            || vectors.contains("assemble_absolute");
        if consumed {
            // Someone wired them. Then FR-255 step 2 has been discharged and this guard is finished —
            // deleting it is the correct action, and the message says so rather than making them guess.
            panic!(
                "`public_vectors` now references the compute, so the vectors ARE consumed. Confirm the \
                 KAT compares the printed 4a/4b/5a/5b against each vector, mark FR-255 step 2 CLOSED, \
                 and DELETE this test — its whole purpose was to keep that from being forgotten."
            );
        }
        assert!(
            ledger.contains("FR-255") && ledger.contains("step 2"),
            "the vectors are still unconsumed but FOLLOWUPS.md no longer records FR-255 step 2 as the \
             task that consumes them. A published third witness that nothing reads, with nothing in the \
             ledger pointing at it, is how it becomes four decorative tables."
        );
    }
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("crates/xtask -> repo root")
            .to_path_buf()
    }

    /// Concatenate every `.rs` under `crates/`, which is where a fix records the follow-up it closed.
    fn all_source() -> String {
        fn walk(dir: &std::path::Path, out: &mut String) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    // Skip build artifacts: they contain generated copies that would double-count.
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    if name.starts_with("target") {
                        continue;
                    }
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    if let Ok(s) = std::fs::read_to_string(&p) {
                        out.push_str(&s);
                        out.push('\n');
                    }
                }
            }
        }
        let mut out = String::new();
        walk(&repo_root().join("crates"), &mut out);
        out
    }

    /// ★★★ **The check itself.** Reds when the set of "ledger says open-and-gating, source cites it"
    /// entries CHANGES — either a new one appears (look at it) or a pinned one closes (remove it).
    #[test]
    fn the_ledger_and_the_source_agree_about_what_is_open() {
        let followups = std::fs::read_to_string(repo_root().join("FOLLOWUPS.md"))
            .expect("FOLLOWUPS.md is readable");
        let got = disagreements(&followups, &all_source());
        let want: BTreeSet<String> = EXPECTED_CITED_AND_OPEN
            .iter()
            .map(|(fr, _)| (*fr).to_string())
            .collect();
        assert_eq!(
            got, want,
            "the ledger and the source disagree about what is open, and the set MOVED.\n\
             \n\
             A NEW entry here means the code cites a follow-up the ledger still calls open and \
             gating. Read the source site: if it records a FIX, mark the ledger header closed — six \
             CRITICALs were stale exactly this way on 2026-09-14. If it records a PENDING HAZARD, add \
             the row to EXPECTED_CITED_AND_OPEN with that reason.\n\
             \n\
             A MISSING entry means a pinned follow-up was closed — delete its row. Do NOT widen this \
             list to quiet a red without reading the source site."
        );
    }

    /// ★★ **B1 — the check observed discriminating, and in both directions.**
    ///
    /// ★ Per FR-235 the plant is not in the checker's vocabulary: the checker tests a header for a
    /// closure marker and a severity word, and these fixtures vary the *ledger/source relationship*
    /// instead — an entry the source names versus one it does not.
    #[test]
    fn an_open_gating_entry_the_source_calls_fixed_is_reported() {
        let ledger = "\
- **FR-901 ★★★ CRITICAL — a defect the code has since fixed.**\n\
  body line that says nothing about closure\n\
- **✅ CLOSED — FR-902 ★★★ CRITICAL — already marked.**\n\
  body\n\
- **FR-903 — a Minor with no gating severity.**\n\
  body\n\
- **FR-904 ★★★ CRITICAL — open and NOT cited anywhere in the source.**\n\
  body\n";
        let source = "// ★★★ FR-901: fixed here.\n// see FR-902 too\n// and FR-903\n";

        let got = disagreements(ledger, source);
        assert_eq!(
            got,
            ["FR-901".to_string()].into_iter().collect::<BTreeSet<_>>(),
            "only FR-901 qualifies: FR-902's header is already marked closed, FR-903 is not gating, \
             and FR-904 is gating and open but the source never names it"
        );

        // ★★ B1a — the negative half, so the test cannot pass on a `disagreements` hard-wired to
        //    return everything or nothing. Each exclusion above is checked as an exclusion.
        assert!(
            !got.contains("FR-902"),
            "a closed header must not be flagged"
        );
        assert!(
            !got.contains("FR-903"),
            "a non-gating header must not be flagged"
        );
        assert!(
            !got.contains("FR-904"),
            "an uncited entry must not be flagged — otherwise this reports every open Critical and \
             is a severity census, not a disagreement check"
        );
    }

    /// ★ Every pinned row must carry a reason, or the next reader cannot tell a considered exemption
    /// from one added to make a red go away.
    #[test]
    fn every_pinned_row_says_why_it_is_open() {
        assert!(
            !EXPECTED_CITED_AND_OPEN.is_empty(),
            "an empty pin makes the check assert the disagreement set is empty, which is a different \
             and much stronger claim than this check can support"
        );
        for (fr, why) in EXPECTED_CITED_AND_OPEN {
            assert!(header_fr(fr).is_some(), "{fr} is not an FR-<n> identifier");
            assert!(
                why.len() > 40,
                "{fr}: the reason is too short to be a reason — say what the source site actually \
                 records, since that is the judgement being pinned"
            );
        }
    }

    /// ★ `FR-` scanning must not run digits together across a boundary, or `FR-1` would match `FR-102`
    /// and the two artifacts would appear to agree when they do not.
    #[test]
    fn fr_identifiers_are_read_whole() {
        let cited = frs_cited_in("FR-102 and FR-1 and xFR-7 and FR-33.");
        assert!(cited.contains("FR-102"));
        assert!(cited.contains("FR-1"));
        assert!(
            cited.contains("FR-33"),
            "a trailing period must not swallow the number"
        );
        assert!(
            !cited.contains("FR-7"),
            "xFR-7 is not a citation — a word-boundary is required, or any identifier ending in FR- \
             would inject false agreement"
        );
    }
}
