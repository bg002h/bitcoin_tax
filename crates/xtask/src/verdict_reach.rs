//! **Does this verdict change a filed number, or gate a filed artifact?**
//!
//! ★★★ Built 2026-09-05, and it exists because of one measured defect. The crypto disposition audit
//! found that `ComplianceStatus` — the engine's per-disposal identification verdict, the thing that
//! decides whether a lot selection was timely under Reg. §1.1012-1(j) — **has no reader on the
//! filing path.** It occurs in `optimize.rs` and its own definition, and nowhere else. So btctax
//! computes "this disposal is NonCompliant", files it anyway at HIFO, and the repo's own KAT
//! measures the cost: a $95 sale reports **gain $5 where FIFO gives $45**.
//!
//! §1.1012-1(j)(3)'s deemed-FIFO consequence lived in **three doc comments and no code**.
//!
//! ★★ **The auditor named this instrument against its own interest.** Its closing note: *"a
//! mechanical sweep of every consumer of `ComplianceStatus`, `BlockerKind` and
//! `pending_reconciliation` asking 'does this change a filed number or gate a filed artifact' would
//! have found three of the four in minutes."* Three of four, in minutes, versus one agent-hour of
//! reading. That is the whole argument for a checker over a review — and it is
//! [`crate::cite_check`]'s argument too.
//!
//! ## The class, stated so it outlives the instance
//!
//! This repo has a memory named **"a figure with no reader"**: an unread computed value is not
//! thereby correct. `total_tax` was once short by the whole AMT with every test green, because two
//! chains existed and nothing compared them. A VERDICT is the sharpest case of that class, because a
//! verdict's only purpose is to be acted on. A verdict nobody reads is worse than no verdict: it
//! creates the appearance of a control.
//!
//! ## What counts as reach
//!
//! A verdict type **reaches** the filing path if its identifier appears in any
//! [`FILING_PATH`] file, outside that file's trailing `#[cfg(test)]` module.
//!
//! ★ **HONEST LIMIT, stated because a gate that hides its blind spot is worse than no gate.** Reach
//! is TEXTUAL. It proves the name is mentioned where a filed number is built; it does NOT prove the
//! value is branched on, nor that the branch is correct. A verdict threaded into `printed.rs` and
//! then ignored in a `let _ =` would pass. This catches the *absence* of a reader, which is the
//! defect actually observed — it does not audit the reader's quality. `#[cfg(test)]` stripping
//! assumes the Rust convention that the test module is last in the file.

use std::path::{Path, PathBuf};

/// Modules whose output becomes a FILED artifact — a number on a form, or a refusal that stops a
/// form being produced. A verdict that never appears here cannot be changing what the filer signs.
///
/// ★ Deliberately short. Widening this list is how the check would be quietly defeated: adding
/// `optimize.rs` here would "fix" the very defect that motivated the instrument, so the entries
/// carry their reason and [`every_filing_path_module_exists`] proves they are real.
pub const FILING_PATH: &[(&str, &str)] = &[
    (
        "crates/btctax-core/src/tax/",
        "the return computation — every 1040 and schedule line is built here",
    ),
    (
        "crates/btctax-core/src/project/fold.rs",
        "the fold that produces the disposals Schedule D and Form 8949 report",
    ),
    (
        "crates/btctax-core/src/project/resolve.rs",
        "resolution decides what BECOMES a disposal — FR-31 and FR-33 are both defects here that \
         change a filed number, so a verdict read only in resolve.rs IS read on the filing path",
    ),
    (
        "crates/btctax-forms/src/",
        "the PDF emitters — what physically lands on the signed paper",
    ),
];

/// Verdict types that legitimately never reach a filed number, each with the reason.
///
/// ★★ **SHRINK-ONLY, and every entry is an admission.** This is the `AUTHORITY_NOT_YET_ARCHIVED`
/// pattern: a recorded gap is not an excuse, it is a thing a future reader can grep for. An entry
/// here says "this verdict is advisory by design" — and [`no_exemption_is_stale`] reds if the named
/// type stops existing, so a closed gap cannot silently reopen.
pub const ADVISORY_ONLY: &[(&str, &str)] = &[
    (
        "ComplianceStatus",
        "★★★ NOT ADVISORY — THIS IS THE DEFECT (FOLLOWUPS.md FR-34). It is listed here ONLY so the \
         gate is not permanently red, which `design/HARNESS.md` says is noise that gets muted. The \
         engine computes a per-disposal identification verdict under Reg. §1.1012-1(j) and files \
         the return anyway: a no-election disposal computes at HIFO while `disposal_compliance` \
         calls that same disposal NonCompliant, and the row reaches only `verify`. The repo's own \
         KAT measures it — a $95 sale, gain $5 under HIFO versus $45 under FIFO. DELETE THIS ENTRY \
         when FR-34 lands; do not let it become furniture.",
    ),
    (
        "ApproxReason",
        "Optimizer diagnostics — why a search was approximate. Advisory by construction: nothing \
         in `optimize.rs` becomes a filed artifact without passing through `optimize accept`, \
         which is separately attested.",
    ),
    (
        "HarvestStatus",
        "`whatif.rs` — the hypothetical harvest tool. Verified 2026-09-05: no `whatif::` symbol \
         appears anywhere under `tax/` or `btctax-forms/`, so nothing it decides can reach paper.",
    ),
    (
        "SellStatus",
        "`whatif.rs` — same tool, same verification. A what-if sale is not a disposal.",
    ),
    (
        "TrancheStatus",
        "Confined to `defensive/` — verified 2026-09-05 that the name appears in no other core or \
         forms module. The tranche state drives the guided dashboard; the Form 8275 it can produce \
         is built from the accepted RESULT, not from this status.",
    ),
];

/// ★★ **The count may only go DOWN.** Four of the five entries above are advisory by design; the
/// fifth is a recorded defect. A NEW verdict with no reader is exactly what this instrument exists
/// to stop, so growth is the one direction that must not happen silently.
pub const UNREACHED_PIN: usize = 5;

/// A verdict enum, and where it is defined.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Verdict {
    pub name: String,
    pub defined_in: String,
}

/// Enum names that look like a determination rather than a datum.
///
/// ★ Shape, not a hand-list — the same rule `archive_check` applies to primary sources, and for the
/// same reason: a hand-list passes the case it forgot. Deliberately OVER-BROAD. A false positive
/// costs one [`ADVISORY_ONLY`] line with a reason; a false negative is a verdict nobody reads.
fn looks_like_a_verdict(name: &str) -> bool {
    ["Status", "Verdict", "Compliance", "Kind", "Reason"]
        .iter()
        .any(|s| name.ends_with(s))
}

/// Everything after the first `#[cfg(test)]` — by Rust convention the trailing test module.
fn without_tests(src: &str) -> &str {
    match src.find("#[cfg(test)]") {
        Some(i) => &src[..i],
        None => src,
    }
}

fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every verdict-shaped `pub enum` defined under `crates/btctax-core/src`.
pub fn verdicts(root: &Path) -> Vec<Verdict> {
    let mut files = Vec::new();
    walk_rs(&root.join("crates/btctax-core/src"), &mut files);
    files.sort();

    let mut out = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        let rel = f
            .strip_prefix(root)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        for line in without_tests(&text).lines() {
            let t = line.trim();
            let Some(rest) = t.strip_prefix("pub enum ") else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && looks_like_a_verdict(&name) {
                out.push(Verdict {
                    name,
                    defined_in: rel.clone(),
                });
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Does `name` appear in any filing-path file (outside its trailing test module), other than where
/// the verdict is defined?
fn reaches_filing_path(root: &Path, v: &Verdict) -> bool {
    let mut files = Vec::new();
    for (p, _) in FILING_PATH {
        let abs = root.join(p);
        if abs.is_dir() {
            walk_rs(&abs, &mut files);
        } else if abs.is_file() {
            files.push(abs);
        }
    }
    for f in files {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        // ★ Skip the DEFINITION LINE, not the defining file. The first cut skipped the whole file
        //   and so reported `SkippableKind` — defined in `tax/questions.rs` and consumed 40 lines
        //   below, both on the filing path — as having no reader. Declaring a verdict is not
        //   reading it; using it in the same module IS.
        let def = format!("pub enum {}", v.name);
        for line in without_tests(&text).lines() {
            if rel == v.defined_in && line.trim_start().starts_with(&def) {
                continue;
            }
            if line.contains(&v.name) {
                return true;
            }
        }
    }
    false
}

/// Verdicts that reach no filed number and carry no recorded reason.
pub fn unreached(root: &Path) -> Vec<Verdict> {
    verdicts(root)
        .into_iter()
        .filter(|v| !ADVISORY_ONLY.iter().any(|(n, _)| *n == v.name))
        .filter(|v| !reaches_filing_path(root, v))
        .collect()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

pub fn report(v: &[Verdict]) -> String {
    let mut s = String::from(
        "VERDICTS WITH NO READER ON THE FILING PATH — each computes a determination that changes \
         nothing the filer signs:\n\n",
    );
    for x in v {
        s.push_str(&format!("  {} (defined in {})\n", x.name, x.defined_in));
    }
    s.push_str("\n  The filing path is:\n");
    for (p, why) in FILING_PATH {
        s.push_str(&format!("    {p} — {why}\n"));
    }
    s.push_str(
        "\n  ★ This is the shape that let `ComplianceStatus` mark a disposal NonCompliant while the\n  \
         return filed it anyway at HIFO — gain $5 where FIFO gives $45, on the repo's own KAT.\n\n  \
         Either give the verdict a reader (make it change a number or raise a refusal), or add it\n  \
         to ADVISORY_ONLY **with the reason it is advisory**. Do not widen FILING_PATH to make this\n  \
         pass — that defeats the instrument.\n",
    );
    s
}

/// `cargo run -p xtask -- verdict-reach`
///
/// ★★ **One implementation, two call sites** — the [`crate::archive_check`] pattern, so the rule
/// cannot drift between the command a human runs and the test `make check` runs.
pub fn run() -> Result<(), String> {
    let root = repo_root();
    let all = verdicts(&root);
    let bad = unreached(&root);
    println!(
        "verdict-reach: {} verdict-shaped enum(s) in btctax-core; {} exempted with a reason; \
         {} reaching no filed number",
        all.len(),
        ADVISORY_ONLY.len(),
        bad.len()
    );
    ratchet(&root)
}

/// The live ratchet: today's exempted set may shrink, never grow.
pub fn ratchet(root: &Path) -> Result<(), String> {
    let bad = unreached(root);
    if !bad.is_empty() {
        return Err(report(&bad));
    }
    if ADVISORY_ONLY.len() > UNREACHED_PIN {
        return Err(format!(
            "ADVISORY_ONLY grew to {} (pinned {UNREACHED_PIN}). A new verdict with no reader on \
             the filing path is the defect this instrument exists to catch — give it a reader \
             instead of an excuse.",
            ADVISORY_ONLY.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// ★★ **THE KILL (B1).** A verdict that no filing-path file mentions must be caught. Planted in
    /// a temp tree so it tests the RULE, not this repo's current contents — a check that only ever
    /// sees a passing repo has never been observed discriminating.
    #[test]
    fn a_verdict_with_no_reader_on_the_filing_path_is_caught() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let core = root.join("crates/btctax-core/src");
        let tax = root.join("crates/btctax-core/src/tax");
        fs::create_dir_all(&tax).expect("mkdir");

        // A verdict the filing path DOES read.
        fs::write(core.join("seen.rs"), "pub enum ReadStatus { A, B }\n").expect("w");
        fs::write(
            tax.join("printed.rs"),
            "fn f(x: ReadStatus) -> u8 { match x { _ => 1 } }\n",
        )
        .expect("w");
        assert!(
            unreached(root).is_empty(),
            "a verdict the filing path mentions must NOT be flagged: {:?}",
            unreached(root)
        );

        // ── The defect: a verdict nothing on the filing path mentions. ──
        fs::write(core.join("orphan.rs"), "pub enum GhostStatus { X }\n").expect("w");
        let found = unreached(root);
        assert_eq!(
            found.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
            vec!["GhostStatus"],
            "a verdict with no filing-path reader must be caught — got {found:?}"
        );

        // ── And the recorded-reason escape must work, or the ratchet is unusable. ──
        // (Exercised by construction: ADVISORY_ONLY filtering happens before the reach test; see
        //  `an_exemption_suppresses_exactly_one_verdict`.)
    }

    /// ★ The exemption list must suppress the named verdict and ONLY that one — an exemption that
    /// silenced everything would make the gate vacuous while reporting success.
    #[test]
    fn an_exemption_suppresses_exactly_one_verdict() {
        // Pure-logic check on the filter, independent of the filesystem.
        let all = vec![
            Verdict {
                name: "GhostStatus".into(),
                defined_in: "a.rs".into(),
            },
            Verdict {
                name: "OtherStatus".into(),
                defined_in: "b.rs".into(),
            },
        ];
        let exempt = [("GhostStatus", "advisory by design")];
        let left: Vec<_> = all
            .into_iter()
            .filter(|v| !exempt.iter().any(|(n, _)| *n == v.name))
            .collect();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].name, "OtherStatus");
    }

    /// ★ The shape must actually fire, and must not fire on a plain datum.
    #[test]
    fn the_verdict_shape_fires_on_determinations_and_not_on_data() {
        for y in [
            "ComplianceStatus",
            "BlockerKind",
            "RefuseReason",
            "TrancheStatus",
        ] {
            assert!(
                looks_like_a_verdict(y),
                "`{y}` must be treated as a verdict"
            );
        }
        for n in ["Usd", "ReturnInputs", "Lot", "Disposal"] {
            assert!(!looks_like_a_verdict(n), "`{n}` is a datum, not a verdict");
        }
    }

    /// ★★ **THE LIVE GATE.** Every verdict in this repository either reaches a filed number or
    /// carries a recorded reason, and the exempted set may only shrink.
    #[test]
    fn this_repo_has_no_unreached_verdict() {
        if let Err(e) = ratchet(&repo_root()) {
            panic!("{e}");
        }
    }

    /// ★★★ **The instrument must still catch the defect it was BUILT for.** `ComplianceStatus` is
    /// exempted only to keep the gate off permanent red — it is FR-34, not a design choice. If it
    /// silently stopped being flagged as unreached, the exemption would have become furniture and
    /// nobody would notice. This asserts the underlying condition, beneath the exemption.
    #[test]
    fn compliance_status_is_still_unread_on_the_filing_path() {
        let root = repo_root();
        let cs = verdicts(&root)
            .into_iter()
            .find(|v| v.name == "ComplianceStatus")
            .expect("ComplianceStatus is still defined");
        assert!(
            !reaches_filing_path(&root, &cs),
            "ComplianceStatus now READS on the filing path — FR-34 may be fixed. If so, delete its \
             ADVISORY_ONLY entry, lower UNREACHED_PIN, and INVERT this test. Do not leave an \
             exemption standing for a defect that no longer exists."
        );
    }

    /// ★ Every filing-path entry must be real. A path that no longer exists is an excuse that
    /// silently shrinks the search — the same staleness `every_accounted_for_tree_still_exists`
    /// guards for the archive ratchet.
    #[test]
    fn every_filing_path_module_exists() {
        let root = repo_root();
        for (p, _) in FILING_PATH {
            let abs = root.join(p);
            assert!(
                abs.exists(),
                "FILING_PATH names `{p}`, which does not exist — remove it or fix it, but do not \
                 leave the check searching a path that is gone"
            );
        }
    }

    /// ★ No exemption may name a verdict that no longer exists.
    #[test]
    fn no_exemption_is_stale() {
        let root = repo_root();
        let live = verdicts(&root);
        for (n, _) in ADVISORY_ONLY {
            assert!(
                live.iter().any(|v| v.name == *n),
                "ADVISORY_ONLY exempts `{n}`, which is no longer defined — delete the entry so the \
                 ratchet tightens"
            );
        }
    }
}
