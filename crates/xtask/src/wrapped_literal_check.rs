//! ★★★ **FR-108 — a sentence with a HOLE in it: the wrap indentation baked into a string literal.**
//!
//! A literal written to *look* wrapped in the source, but left on one physical line with the
//! next line's indentation still inside the quotes, prints to the filer with a six-plus-space gap
//! in the middle of a sentence:
//!
//! ```text
//! Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC box 1,
//!                  1099-MISC box 3 or 1099-K box 1a.
//! ```
//!
//! Measured 2026-09-07 over `crates/**/*.rs`: **34** single-source-line literals over 120 characters
//! carrying a run of six or more interior spaces, across 13 files. Six of them reach a person
//! verbatim — two Schedule 1-A trade-or-business REFUSAL details, an advisory, the `income scrub`
//! stale-draft note, and two Form 8995-A field help strings a filer reads while typing. Never a
//! wrong figure, but it is in the exit sentences this product's fail-closed posture rests on.
//!
//! ★★ **THE MECHANISM FR-108 NAMES IS WRONG, AND THE CORRECTION MATTERS FOR THE FIX.** The entry
//! blames a Rust `\`-newline continuation *"which eats the newline and KEEPS the next source line's
//! indentation"*. It does not: a continuation skips the newline **and the next line's leading
//! whitespace**, which is why every correctly-wrapped literal in this repo puts the space *before*
//! the backslash. Measured with `rustc` (2026-09-08):
//!
//! ```text
//! let a = "alpha \        ⇒ [alpha beta]
//!          beta";
//! let b = "alpha\         ⇒ [alphabeta]
//!          beta";
//! ```
//!
//! So a continuation is the CURE, not the disease. The defect is a literal that was never wrapped
//! at all — written (or joined) as one physical line with the wrap indentation typed in as real
//! spaces. That is why this check looks for a run of spaces on **one source line**: a literal that
//! spans lines is, by construction, already using the escape that removes them.
//!
//! ## What this covers
//!
//! A finding is a string literal that is **all four** of:
//!
//! 1. **opened and closed on the same source line** — a literal that already wraps has had its
//!    whitespace removed by the continuation escape, so it cannot carry this defect;
//! 2. **longer than 120 characters** including its quotes — the threshold FR-108 measured. A short
//!    literal with a run of spaces is a *column template* (`"  Adjustments (L10):        {}"`), and
//!    those are the overwhelming majority: at 120 the tree has 29 candidates, at no length limit it
//!    has 113, and the 84 extra are alignment, indentation prefixes and code generation;
//! 3. **free of an embedded newline escape (`\n`)** — a literal that prints more than one line is a
//!    table, a code sample or an aligned key/value block, where a run of spaces is the point
//!    (`"  fmv_at_gift:       {fmv}\n    donor_basis: …"`). This exclusion is DERIVED rather than
//!    listed, and it is exactly right on the measured tree: it is what separates the 29 real
//!    manglings from the 5 legitimate alignments, with no file named anywhere in this module;
//! 4. carrying a **run of six or more spaces between two non-space characters**. Six because the
//!    gap is a source indentation, and Rust source is indented in fours — a two- or three-space run
//!    is sentence typography, not a wrap.
//!
//! ## What this deliberately does NOT cover
//!
//! - **Comments and doc comments**, which are skipped outright: an aligned table in a `///` block
//!   is documentation, and reformatting one is not this check's business.
//! - **Raw strings** (`r"…"`, `r#"…"#`). A raw string cannot use the continuation escape at all, so
//!   a wrapped one holds a real newline and every space in it was typed on purpose.
//! - **Literals of 120 characters or fewer**, and **runs of five spaces or fewer** — see above.
//!   Both are blind spots on purpose, and both are stated so nobody mistakes silence for coverage.
//! - **The printed result.** This reads source, not output: a gap assembled at runtime from two
//!   `format!` arguments is invisible here.
//!
//! There is **no per-site allow list and no escape comment**, on purpose. FR-99 is this repo's
//! dominant defect class — a hand-written list standing beside a set that grows — and an exemption
//! list is that shape with a laundering path attached. If a legitimate case ever reds, the fix is to
//! widen one of the four derived conditions above, with the case as its evidence.

use std::path::{Path, PathBuf};

/// A run of this many spaces or more, between two non-space characters, is a wrap gap.
const MIN_RUN: usize = 6;
/// Literals at or below this length are column templates, not sentences (FR-108's own threshold).
const MIN_LEN: usize = 120;
/// The walk must see at least this many files. A check that scans nothing passes by finding nothing.
const FILE_FLOOR: usize = 200;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/xtask -> repo root")
        .to_path_buf()
}

/// Every `.rs` file under a directory, sorted.
fn rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// One ordinary (non-raw) string literal, as it appears in the source.
#[derive(Debug, PartialEq, Eq)]
pub struct Literal {
    /// 1-based line of the opening quote.
    pub line: usize,
    /// 1-based line of the closing quote (equal to `line` for a single-line literal).
    pub end_line: usize,
    /// The literal INCLUDING both quotes, verbatim from the source.
    pub text: String,
}

/// ★★★ **Every ordinary string literal in a Rust source, with comments, char literals and raw
///     strings skipped.**
///
/// Hand-rolled rather than regex, and the reason is the three things a regex over `"…"` gets wrong,
/// each of which would put this checker in the class it exists to catch — an instrument reporting
/// success over a region it cannot see:
///
/// - a `"` inside a `//` or `/* */` comment (block comments **nest** in Rust) opens a phantom
///   literal that swallows the rest of the file;
/// - a `"` inside a CHAR literal (`'"'`) does the same, and this repo has them;
/// - a raw string's `#` delimiters mean its contents are not escaped at all.
///
/// A lifetime (`'a`) is not a char literal and must not start one; the two are told apart by
/// looking for the closing quote where a char literal would have it.
#[must_use]
pub fn string_literals(src: &str) -> Vec<Literal> {
    let b: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    while i < b.len() {
        let c = b[i];
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        // ── a comment ─────────────────────────────────────────────────────────────────────────
        if c == '/' && i + 1 < b.len() && b[i + 1] == '/' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < b.len() && depth > 0 {
                if b[i] == '\n' {
                    line += 1;
                } else if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
                    depth += 1;
                    i += 1;
                } else if b[i] == '*' && i + 1 < b.len() && b[i + 1] == '/' {
                    depth -= 1;
                    i += 1;
                }
                i += 1;
            }
            continue;
        }
        // ── a raw string (`r"…"`, `r#"…"#`, `br#"…"#`) — skipped whole, see the module doc ──────
        if (c == 'r' || c == 'b') && !(i > 0 && ident(b[i - 1])) {
            let mut j = i;
            if b[j] == 'b' && j + 1 < b.len() && b[j + 1] == 'r' {
                j += 1;
            }
            if b[j] == 'r' {
                let mut k = j + 1;
                let mut hashes = 0usize;
                while k < b.len() && b[k] == '#' {
                    hashes += 1;
                    k += 1;
                }
                if k < b.len() && b[k] == '"' {
                    k += 1;
                    // Scan for the closing `"` followed by `hashes` `#`s.
                    while k < b.len() {
                        if b[k] == '\n' {
                            line += 1;
                        } else if b[k] == '"' {
                            let closed = (1..=hashes).all(|h| b.get(k + h) == Some(&'#'));
                            if closed {
                                k += hashes + 1;
                                break;
                            }
                        }
                        k += 1;
                    }
                    i = k;
                    continue;
                }
            }
        }
        // ── a char literal, told apart from a lifetime ────────────────────────────────────────
        if c == '\'' {
            let mut k = i + 1;
            if k < b.len() && b[k] == '\\' {
                k += 2;
                while k < b.len() && b[k] != '\'' && b[k] != '\n' {
                    k += 1;
                }
            } else {
                k += 1;
            }
            if k < b.len() && b[k] == '\'' {
                i = k + 1;
                continue;
            }
            // A lifetime — nothing to skip.
            i += 1;
            continue;
        }
        // ── an ordinary string literal (`"…"`, `b"…"`) ────────────────────────────────────────
        if c == '"' {
            let start = i;
            let start_line = line;
            let mut k = i + 1;
            while k < b.len() {
                if b[k] == '\\' {
                    if b.get(k + 1) == Some(&'\n') {
                        line += 1;
                    }
                    k += 2;
                    continue;
                }
                if b[k] == '\n' {
                    line += 1;
                }
                if b[k] == '"' {
                    break;
                }
                k += 1;
            }
            let end = k.min(b.len().saturating_sub(1));
            out.push(Literal {
                line: start_line,
                end_line: line,
                text: b[start..=end].iter().collect(),
            });
            i = end + 1;
            continue;
        }
        i += 1;
    }
    out
}

/// The 0-based character index at which `s` carries a run of `MIN_RUN`+ spaces between two
/// non-space characters, if it does.
#[must_use]
pub fn wrap_gap(s: &str) -> Option<usize> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < c.len() {
        if c[i] == ' ' {
            let start = i;
            while i < c.len() && c[i] == ' ' {
                i += 1;
            }
            let run = i - start;
            let before_is_text = start > 0 && c[start - 1] != ' ';
            let after_is_text = i < c.len() && c[i] != ' ';
            if run >= MIN_RUN && before_is_text && after_is_text {
                return Some(start);
            }
            continue;
        }
        i += 1;
    }
    None
}

/// The findings in ONE source, as `label:line — snippet` strings. Pure, so the kill test can drive
/// it on a planted source without touching the tree.
#[must_use]
pub fn findings_in(label: &str, src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for lit in string_literals(src) {
        if lit.line != lit.end_line {
            continue;
        }
        if lit.text.chars().count() <= MIN_LEN {
            continue;
        }
        if lit.text.contains("\\n") {
            continue;
        }
        // ★ The gap is looked for in the BODY, never across the quotes. Padding at either end of a
        //   literal is deliberate (a fixed-width cell, a right-aligned column); only a run BETWEEN
        //   two pieces of the text is a wrap that was never wrapped.
        let body = &lit.text[1..lit.text.len() - 1];
        let Some(at) = wrap_gap(body) else {
            continue;
        };
        // The gap in context: 30 characters either side, so the finding names the sentence.
        let c: Vec<char> = body.chars().collect();
        let lo = at.saturating_sub(30);
        let hi = (at + 40).min(c.len());
        let snippet: String = c[lo..hi].iter().collect();
        out.push(format!(
            "{label}:{} — a {}-space gap inside a one-line literal: …{}…",
            lit.line,
            c[at..].iter().take_while(|ch| **ch == ' ').count(),
            snippet.replace('\n', "\\n")
        ));
    }
    out
}

/// Every `crates/**/*.rs` file, as `(repo-relative label, source)`.
fn sources() -> Vec<(String, String)> {
    let root = repo_root();
    rs_files(&root.join("crates"))
        .into_iter()
        .map(|p| {
            let rel = p
                .strip_prefix(&root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            (rel, src)
        })
        .collect()
}

/// FR-108, over the committed tree.
///
/// # Errors
/// The findings, one per line — or a walk that saw too few files to be believed.
pub fn run() -> Result<String, String> {
    let files = sources();
    if files.len() < FILE_FLOOR {
        return Err(format!(
            "the walk found only {} files under crates/ — a check that scans nothing passes by \
             finding nothing",
            files.len()
        ));
    }
    let mut findings = Vec::new();
    for (label, src) in &files {
        findings.extend(findings_in(label, src));
    }
    if findings.is_empty() {
        Ok(format!(
            "wrapped literals: {} sources scanned, no one-line literal over {MIN_LEN} chars \
             carries a {MIN_RUN}-space gap",
            files.len()
        ))
    } else {
        Err(format!(
            "FR-108 — {} string literal(s) print with the wrap indentation still inside the \
             quotes. Wrap them with a `\\`-newline continuation, which removes the next line's \
             leading whitespace (so the space goes BEFORE the backslash):\n  {}",
            findings.len(),
            findings.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check, against the real tree. It must be silent.
    #[test]
    fn no_committed_literal_carries_its_wrap_indentation() {
        match run() {
            Ok(s) => println!("{s}"),
            Err(e) => panic!("{e}"),
        }
    }

    /// ★★★ **B1 — the check watched RED on the exact defect it exists to catch, and green on each
    ///     near miss beside it.**
    ///
    /// The plant is `return_refuse.rs:2498` as it stood at `db591e10`, verbatim: the Schedule 1-A
    /// trade-or-business tips refusal, one of the six FR-108 sentences a filer reads.
    ///
    /// ★★ **B1a — the fixture is asserted to PRESENT the case.** A planted literal that quietly
    ///    drifted under 120 characters, or whose gap shrank to five spaces, would make every
    ///    assertion below pass for the wrong reason — a kill test that cannot kill. So the plant's
    ///    own two properties are measured first, from the fixture, before it is used as one.
    #[test]
    fn the_check_reds_on_a_baked_in_wrap_and_not_on_its_near_misses() {
        // ── THE PLANT — one physical line, the wrap indentation typed in as real spaces ────────
        let planted = concat!(
            "        let detail = \"Schedule 1-A line 5 asks for qualified tips reported on Form ",
            "1099-NEC box 1,                  1099-MISC box 3 or 1099-K box 1a. btctax has no ",
            "input for those forms.\";\n"
        );
        // B1a — the fixture actually presents the case.
        let lit = &string_literals(planted)[0];
        assert_eq!(
            lit.line, lit.end_line,
            "the plant must be on ONE source line"
        );
        assert!(
            lit.text.chars().count() > MIN_LEN,
            "the plant is only {} chars — under the {MIN_LEN} threshold it would pass for the \
             wrong reason",
            lit.text.chars().count()
        );
        assert!(
            wrap_gap(&lit.text[1..lit.text.len() - 1]).is_some(),
            "the plant carries no {MIN_RUN}-space gap, so it is not the defect"
        );
        assert!(
            !lit.text.contains("\\n"),
            "the plant must carry no newline escape, or the table exclusion answers for it"
        );

        let hits = findings_in("planted.rs", planted);
        assert_eq!(
            hits.len(),
            1,
            "the planted wrap must be a finding: {hits:?}"
        );
        assert!(
            hits[0].contains("planted.rs:1") && hits[0].contains("18-space gap"),
            "the finding must name the site and the size of the gap: {}",
            hits[0]
        );

        // ── THE NEAR MISSES. A checker that reds on everything is deleted by the next person who
        //    trips it, so each of these must stay silent. ──────────────────────────────────────
        let green = |what: &str, src: &str| {
            let h = findings_in("near.rs", src);
            assert!(h.is_empty(), "{what} must stay green, got {h:?}");
        };
        // 1. The SAME sentence, wrapped the way Rust actually wraps — this is the fix, and the
        //    check must accept it or the fix does not converge.
        green(
            "a real `\\`-newline continuation",
            concat!(
                "        let detail = \"Schedule 1-A line 5 asks for qualified tips reported on \\\n",
                "                      Form 1099-NEC box 1, 1099-MISC box 3 or 1099-K box 1a. \\\n",
                "                      btctax has no input for those forms.\";\n"
            ),
        );
        // 2. An ALIGNED key/value block — the run of spaces IS the content (draw_edit.rs:979).
        green(
            "an aligned multi-line display block",
            concat!(
                "            format!(\"  as: GiftReceived\\n\\n    fmv_at_gift:       {fmv}   ",
                "(REQUIRED)\\n    donor_basis:       {basis}\\n    donor_acquired_at: {date}\")\n"
            ),
        );
        // 3. A short COLUMN TEMPLATE — the overwhelming majority of runs in this tree.
        green(
            "a column template under the length threshold",
            "    writeln!(s, \"  Adjustments (L10):        {}\", fmt_money(f.line10));\n",
        );
        // 4. The identical gap inside a COMMENT, which this check does not police.
        green(
            "a long comment with a run of spaces",
            "// Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC box 1,      \
             1099-MISC box 3 or 1099-K box 1a, and this comment is well over the threshold.\n",
        );
        // 5. A RAW string, which cannot use the continuation escape, so its spaces were typed on
        //    purpose.
        green(
            "a raw string",
            "    let s = r\"Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC \
             box 1,                  1099-MISC box 3 or 1099-K box 1a.\";\n",
        );
        // 6. A five-space run in a long literal — sentence typography, not a source indent.
        green(
            "a five-space run",
            "    let s = \"Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC \
             box 1,     1099-MISC box 3 or 1099-K box 1a, well past the length threshold.\";\n",
        );
        // 7. TRAILING and LEADING runs, which are padding rather than a gap in a sentence.
        green(
            "a run at the edge of the literal",
            "    let s = \"Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC \
             box 1, 1099-MISC box 3 or 1099-K box 1a, well past the threshold.            \";\n",
        );
    }

    /// ★★★ **The three shapes that would make the SCANNER blind, each watched not blinding it.**
    ///
    /// This is the half a regex over `"…"` gets wrong, and the failure would be silent in the worst
    /// direction: a phantom literal opened by a quote inside a comment or a char literal swallows
    /// the rest of the file, so every real mangling after it goes unreported and the check still
    /// says OK. Each case below plants a REAL finding *after* the hazard and asserts it is still
    /// seen.
    #[test]
    fn a_quote_inside_a_comment_a_char_literal_or_a_raw_string_does_not_blind_the_scan() {
        let mangled = concat!(
            "\"Schedule 1-A line 5 asks for qualified tips reported on Form 1099-NEC box 1,",
            "                  1099-MISC box 3 or 1099-K box 1a. btctax has no input for those.\""
        );
        for (what, hazard) in [
            ("a line comment", "// a quote \" in a comment\n".to_string()),
            (
                "a nested block comment",
                "/* outer /* inner \" */ still */\n".to_string(),
            ),
            ("a char literal", "let q = '\"';\n".to_string()),
            (
                "a raw string with hashes",
                "let r = r#\"a \" inside\"#;\n".to_string(),
            ),
            ("a lifetime", "fn f<'a>(s: &'a str) {}\n".to_string()),
            (
                "an escaped quote",
                "let e = \"a \\\" inside\";\n".to_string(),
            ),
        ] {
            let src = format!("{hazard}let d = {mangled};\n");
            let hits = findings_in("h.rs", &src);
            assert_eq!(
                hits.len(),
                1,
                "{what} blinded the scan — the mangling after it was not seen: {hits:?}"
            );
        }
    }

    /// The walk is derived from the tree, and it must be able to say so: a scan of nothing is an
    /// error, not a pass.
    #[test]
    fn the_walk_enumerates_the_tree_and_refuses_to_pass_on_nothing() {
        assert!(
            sources().len() >= FILE_FLOOR,
            "the file walk has stopped being populated"
        );
        assert!(
            sources().iter().all(|(l, _)| l.starts_with("crates/")),
            "every scanned label must be repo-relative"
        );
    }
}
