//! **Every quoted claim in a design document, checked against the manual.**
//!
//! ## Why this exists
//!
//! This project's review loop kept producing a specific low-value finding: *"section X disagrees with
//! section Y."* `CLAUDE.md` records the evidence — five plan-review rounds went 5C/12I → 2C/8I → 0C/4I
//! → 0C/4I → 0C/2I, with rounds 3-5 finding mostly "edited §X, forgot §Y" — and concludes that once
//! findings look like that, the artifact has stopped being where the risk lives.
//!
//! But that whole class is **mechanically resolvable, because we have the instruction manual.** An IRS
//! form and its instructions are the single authority; a document does not need to agree with *itself*,
//! it needs to agree with *the form*. So instead of a reviewer reading two sections and comparing them,
//! this test compares each of them to the primary source — and two sections that both match the manual
//! cannot disagree with each other.
//!
//! ## What it checks
//!
//! Every span in a design document that is *presented as a quotation from the form or its instructions*
//! must appear **verbatim** in the committed text-layer extract:
//!
//! - markdown blockquote lines (`> …`), which is how the specs carry the long instruction passages; and
//! - inline emphasised quotations (`*"…"*` / `**"…"**`), which is how they carry single sentences.
//!
//! A span that is not found is either a paraphrase presented as a quote, or a real misreading of the
//! form. Both are defects, and the second is the class that ships wrong numbers.
//!
//! ## ★★ What it does NOT hold — measured, not assumed
//!
//! **It verifies that a quotation appears SOMEWHERE in the manual, not that it appears at the line it is
//! attributed to.** Found by mutation: changing S-1's line-11 quotation from *"decrease … to the next
//! lower whole number"* to *"increase … to the next higher whole number"* — inverting the rounding
//! direction, the single most dangerous fact in this form — **survives**, because line 28 really does say
//! that. Attributing a real sentence to the wrong line is the residual gap, and it is tracked as
//! `FOLLOWUPS.md` §G-10.
//!
//! So this catches: invented quotations, paraphrases presented as citations, composites with bracketed
//! alternatives, silent truncation, and any drift between a document and the archived PDF. It does not
//! catch a correctly-quoted sentence pointed at the wrong line — for that, the guard is T1's
//! `parts_two_and_three_floor_the_step_count_while_part_four_ceils`, which asserts the *behaviour* at a
//! fractional step and dies to that mutation. **Prose citation checking and executable KATs cover
//! different halves; neither is a substitute.**
//!
//! ## What it deliberately does NOT check
//!
//! Prose that is *about* the form (our own analysis, cross-references, task ordering). Only material
//! that claims to *be* the form's words is in scope — asserting more would make the test a style gate
//! and it would be turned off.
//!
//! ★ The extracts are committed fixtures generated from the archived PDFs by
//! `cargo run -p xtask -- extract-schedule-1a`, so this runs with no `pdftotext` at test time and the
//! PDF hash in each fixture header pins provenance.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Repo root, from this crate's manifest directory.
pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

/// Normalise for comparison: the extract wraps lines mid-sentence, hyphenates across breaks, and uses
/// typographic punctuation, none of which is a substantive difference. Markdown emphasis inside a
/// quotation is ours, not the form's, so it comes out too.
///
/// ★ This is deliberately lossy in exactly the ways a PDF text layer is noisy, and **not** lossy about
/// digits, dollar amounts, or line numbers — the things a misreading gets wrong.
pub(crate) fn normalise(s: &str) -> String {
    let mut t = s.to_string();
    for (from, to) in [
        ('\u{2019}', '\''),
        ('\u{2018}', '\''),
        ('\u{201C}', '"'),
        ('\u{201D}', '"'),
        ('\u{2014}', '-'),
        ('\u{2013}', '-'),
        ('\u{00A0}', ' '),
    ] {
        t = t.replace(from, &to.to_string());
    }
    // Markdown emphasis, quote marks and bullets are OURS, not the form's — a quotation carries the
    // delimiting `"` and a re-flowed list may carry `•`, and neither appears that way in the extract.
    // ★ Forgetting the quote marks made every quotation that included one fail to match, which is how
    // this function's first draft reported 19 false positives.
    t = t.replace(['*', '`', '"', '\u{2022}'], " ");
    // ★ The IRS PDFs render CAUTION / TIP / ! icons as text, and `pdftotext` drops the label INTO the
    // sentence it decorates — the extract really contains "Form CAUTION 1099-MISC" and, worse, "final
    // regu-\nTIP lations". Typography, not prose, so it comes out.
    //
    // ★★ This MUST run before de-hyphenation: a label landing inside a hyphenated word gets glued to the
    // stem ("reguTIP lations") and is then no longer a standalone token to strip. Ordering bug, found by
    // the checker failing on exactly one span out of 32.
    t = strip_icon_labels(&t);

    // De-hyphenate across a line break: "self-em-\nployment" -> "self-employment".
    let mut out = String::with_capacity(t.len());
    let bytes: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '-' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_whitespace() {
                j += 1;
            }
            if j > i + 1 && j < bytes.len() {
                // A hyphen followed by whitespace: drop both (the PDF broke a word here).
                i = j;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    // ★ Drop punctuation on BOTH sides. Stripping `**` from mid-sentence emphasis leaves a space before
    // the following period ("2024 ." vs "2024."), which is not a substantive difference but defeats a
    // substring match — the second-largest source of false positives in this checker's first draft.
    // Digits, `$` and `%` are KEPT, because a dollar amount or a line number is exactly what a
    // misreading gets wrong.
    let kept: String = out
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '$' || c == '%' || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect();
    kept.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Remove the CAUTION / TIP / `!` icon labels the IRS PDFs render as text.
///
/// Two forms, both real in these extracts:
/// - **standalone**, dropped into the middle of a sentence — `"Form CAUTION 1099-MISC, and Form 1099-K"`;
/// - ★ **glued to a word the icon split** — `"final reguTIP lations"`, where the label landed between
///   `regu` and `lations`. Token filtering cannot see this one, which is why it is handled by position:
///   a label immediately following a lowercase letter is mid-word and takes its trailing space with it.
///
/// Matched CASE-SENSITIVELY and never when the next character continues the word (so an all-caps `TIPS`
/// survives intact — otherwise this would silently rewrite it to `S` and quietly stop matching).
fn strip_icon_labels(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let chars: Vec<char> = t.chars().collect();
    let mut i = 0;
    'outer: while i < chars.len() {
        for label in ["CAUTION", "TIP"] {
            let n = label.chars().count();
            if chars[i..].starts_with(&label.chars().collect::<Vec<_>>()[..]) {
                let next = chars.get(i + n).copied();
                // Not a label if the word continues (TIPS, CAUTIONARY).
                if next.is_some_and(|c| c.is_alphanumeric() && c.is_uppercase() || c == 's') {
                    break;
                }
                let glued_midword = out.chars().last().is_some_and(|c| c.is_lowercase());
                let standalone = out.chars().last().is_none_or(char::is_whitespace);
                if glued_midword || standalone {
                    i += n;
                    // Take the single following space with it, rejoining the split word.
                    if chars.get(i) == Some(&' ') {
                        i += 1;
                    }
                    continue 'outer;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Pull every span that is presented as a quotation from the form or its instructions.
///
/// Returns `(line_number, span)` so a failure names the line to fix.
fn quoted_spans(doc: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut blockquote: Option<(usize, String)> = None;
    for (idx, raw) in doc.lines().enumerate() {
        let line = raw.trim_start();
        if let Some(rest) = line.strip_prefix('>') {
            let rest = rest.trim();
            match blockquote.as_mut() {
                // A blank `>` line separates two quotations rather than continuing one.
                Some(_) if rest.is_empty() => {
                    if let Some(q) = blockquote.take() {
                        out.push(q);
                    }
                }
                Some((_, acc)) => {
                    acc.push(' ');
                    acc.push_str(rest);
                }
                None if !rest.is_empty() => blockquote = Some((idx + 1, rest.to_string())),
                None => {}
            }
            continue;
        }
        if let Some(q) = blockquote.take() {
            out.push(q);
        }
    }
    if let Some(q) = blockquote.take() {
        out.push(q);
    }
    out.extend(inline_quotations(doc));
    out.extend(plain_quotations(doc));
    out
}

/// Plain `"…"` quotations that carry no markdown emphasis — the form's words sitting in a **table cell**
/// or a parenthetical.
///
/// ★★ **Why this pass exists at all: without it the checker was decoration for the most dangerous fact in
/// this form.** S-1's rounding-direction table quotes lines 11 and 28 as plain `"…"`, and a mutation that
/// flipped "decrease" to "increase" there — inverting the rounding — SURVIVED, because the first draft
/// only looked at `*"…"*` spans and blockquotes.
///
/// Scanned **per line** and skipped when the line's quotes are unbalanced or the span carries markdown or
/// code metacharacters. Sequential pairing across the whole document drifts as soon as one stray quote
/// appears in an inline code span, and then captures our own prose as if it were the form's.
fn plain_quotations(doc: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (idx, line) in doc.lines().enumerate() {
        if line.trim_start().starts_with('>') {
            continue; // blockquotes already handled, and they legitimately span lines
        }
        // ★★ ATTRIBUTION IS REQUIRED, rather than exclusion being enumerated. A plain `"…"` is checked
        // only when its line claims the span comes from the form — "line 11:", "the instructions say",
        // "Caution:". Inverting it this way is the difference between a rule and a growing blocklist: the
        // first draft excluded known self-citations and kept finding new ones (our own prompt wording, a
        // spec principle, a paraphrase of §5.6b), because *our* documents quote *themselves* constantly.
        let lower = line.to_lowercase();
        const ATTRIBUTION: [&str; 7] = [
            "line ",
            "instruction",
            "i1040",
            "caution",
            "worksheet",
            "the form",
            "f1040",
        ];
        if !ATTRIBUTION.iter().any(|m| lower.contains(m)) {
            continue;
        }
        if FOREIGN_SOURCES
            .iter()
            .any(|src| lower.contains(&src.to_lowercase()))
        {
            continue;
        }
        // A statute quotation is authorised by the U.S. Code, not by this extract. The form paraphrases
        // the statute constantly ("for each $1,000" vs "for each $1,000 or portion thereof" — which is
        // exactly the distinction S-1 turns on), so checking one against the other would be wrong.
        if line.contains('§') {
            continue;
        }
        let quotes: Vec<usize> = line
            .char_indices()
            .filter(|(_, c)| *c == '"')
            .map(|(i, _)| i)
            .collect();
        if !quotes.len().is_multiple_of(2) {
            continue; // unbalanced: part of a multi-line quotation, handled by `inline_quotations`
        }
        // ★ `as_chunks::<2>()`, not `chunks_exact(2)` — required by clippy 1.98
        //   (`chunks_exact_to_as_chunks`), and strictly better here: the pair arrives as a fixed-size
        //   `[usize; 2]`, so the two ends are DESTRUCTURED and named rather than reached by
        //   bounds-checked index. The discarded `.1` remainder is provably empty — the
        //   `is_multiple_of(2)` guard above already returned early on an odd count — which is what
        //   makes that guard load-bearing rather than decorative.
        for &[open, close] in quotes.as_chunks::<2>().0 {
            let span = &line[open + 1..close];
            // Emphasis INSIDE the span is fine — `normalise` strips it, and S-1's rounding table marks
            // the load-bearing word bold ("the next **lower** whole number"), so excluding `*` here
            // skipped precisely the span this pass exists for. Inline CODE is ours, not the form's.
            if span.contains('`') || span.len() < 12 {
                continue;
            }
            out.push((idx + 1, span.to_string()));
        }
    }
    out
}

/// Inline emphasised quotations — `*"…"*` / `**"…"**` — scanned over the WHOLE document.
///
/// ★ Deliberately not line-by-line: one of these routinely spans a line break, which leaves the quote
/// count on each line ODD and mis-pairs every quotation after it, capturing our own prose as if it were
/// the form's words. Anchoring on the `*"` … `"*` delimiters instead of on bare `"` also means an
/// ordinary quoted phrase in our prose is not mistaken for a citation.
fn inline_quotations(doc: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = doc.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'"' {
            let start = i + 2;
            let mut j = start;
            while j + 1 < bytes.len() && !(bytes[j] == b'"' && bytes[j + 1] == b'*') {
                j += 1;
            }
            if j + 1 < bytes.len() {
                let span = &doc[start..j];
                let line = doc[..start].matches('\n').count() + 1;
                // A citation of one of OUR documents is not authorised by this extract.
                let ctx_start = doc[..start].rfind('\n').map_or(0, |p| p + 1);
                let ctx_end = doc[j..]
                    .find('\n')
                    .map_or(doc.len(), |p| (j + p).min(doc.len()));
                let ctx = &doc[ctx_start..ctx_end];
                if !FOREIGN_SOURCES
                    .iter()
                    .any(|src| ctx.to_lowercase().contains(&src.to_lowercase()))
                {
                    out.push((line, span.to_string()));
                }
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// A quoted span shorter than this is not worth checking — a two-word fragment matches by accident and
/// a failure would be noise, not signal.
const MIN_WORDS: usize = 6;

/// A source named on the same line that is **not** the IRS form, so the span is a quotation of something
/// else and this extract cannot authorise it.
const FOREIGN_SOURCES: [&str; 12] = [
    "comment",
    // ★ A quotation attributed to a SOURCE FILE is our own code or comment, not the IRS. Caught by the
    // checker itself: quoting return_1040.rs's doc comment tripped the "the form" attribution marker.
    ".rs:",
    ".rs`",
    "CLAUDE.md",
    "STANDARD_WORKFLOW",
    "FOLLOWUPS",
    "recon",
    "B2's own",
    "doc comment saying",
    "spec §3",
    "§4.1",
    "r1 draft",
];

/// Split an elided quotation into the fragments it actually asserts.
///
/// ★ A quotation containing `…` claims that each side appears verbatim, **not** that the joined string
/// does — so checking the join is simply the wrong assertion, and it is the one that produced most of
/// this checker's first-draft noise. Splitting is also what makes a re-flowed bulleted list checkable:
/// each bullet is its own contiguous span in the source even when we present them together.
fn fragments(span: &str) -> Vec<String> {
    let mut t = span.replace('\u{2026}', "|").replace("...", "|");
    t = t.replace('\u{2022}', "|");
    // A numbered list re-flowed into one paragraph is N contiguous spans in the source, not one: the
    // source breaks them across lines and often re-orders the surrounding prose.
    for n in 1..=9 {
        t = t.replace(&format!(" {n}. "), " |");
    }
    t.split('|')
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect()
}

/// Does this span carry at least one fragment long enough to check?
fn checkable(span: &str) -> bool {
    fragments(span)
        .iter()
        .any(|f| normalise(f).split_whitespace().count() >= MIN_WORDS)
}

/// Check one document against one or more extracts. Returns the spans that were NOT found.
pub fn unverified_quotations(doc: &str, extracts: &[String]) -> Vec<(usize, String)> {
    let haystacks: Vec<String> = extracts.iter().map(|e| normalise(e)).collect();
    let mut bad = Vec::new();
    for (line, span) in quoted_spans(doc) {
        // Every fragment of an elided quotation must appear; fragments too short to be distinctive are
        // skipped rather than guessed at. ★ Report the FAILING FRAGMENT, not the whole span — a long
        // quotation with one wrong clause should point at the clause.
        for part in fragments(&span) {
            let n = normalise(&part);
            if n.split_whitespace().count() < MIN_WORDS {
                continue;
            }
            if !haystacks.iter().any(|h| h.contains(&n)) {
                bad.push((line, part));
            }
        }
    }
    bad
}

/// The Schedule 1-A documents and the extracts that authorise them.
fn schedule_1a_docs() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let root = repo_root();
    (
        vec![
            root.join("design/ty2025/SPEC_schedule_1a.md"),
            root.join("design/ty2025/IMPLEMENTATION_PLAN_schedule_1a.md"),
        ],
        vec![
            root.join("crates/btctax-core/src/tax/fixtures/schedule_1a_2025_form.txt"),
            root.join("crates/btctax-core/src/tax/fixtures/schedule_1a_2025_instructions.txt"),
        ],
    )
}

/// The design corpus is filed one directory per tax year, `design/ty<YYYY>`. Return them, newest last.
fn design_year_dirs() -> Result<Vec<(i32, PathBuf)>, String> {
    let root = repo_root().join("design");
    let mut out = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| format!("cannot read {}: {e}", root.display()))? {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", root.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(year) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("ty"))
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };
        out.push((year, path));
    }
    out.sort();
    Ok(out)
}

/// Tax years whose design corpus exists on disk but which **no checked document comes from**.
///
/// ★★ The other half of the year-transition failure, and the one the coverage ratchet cannot see:
/// [`schedule_1a_docs`] names two TY2025 documents and two TY2025 extracts by hand. Standing up
/// `design/ty2026/` therefore adds a corpus that `cite-check` silently does not read — it would go on
/// printing *"51 quotations, all verbatim"* about last year's documents. Skipping is not passing, so a
/// year with no checked document is reported by name rather than quietly not counted.
///
/// Pure in its inputs, so the planted case can be exercised without creating a directory.
fn unchecked_design_years(docs: &[PathBuf], year_dirs: &[(i32, PathBuf)]) -> Vec<i32> {
    year_dirs
        .iter()
        .filter(|(_, dir)| !docs.iter().any(|d| d.starts_with(dir)))
        .map(|(year, _)| *year)
        .collect()
}

/// `cargo run -p xtask -- cite-check` — the same check the test runs, as a command, so it can be run
/// while editing a design document instead of only at test time.
pub fn run() -> Result<(), String> {
    let (docs, extract_paths) = schedule_1a_docs();
    let extracts: Vec<String> = extract_paths
        .iter()
        .map(|p| fs::read_to_string(p).map_err(|e| format!("cannot read {}: {e}", p.display())))
        .collect::<Result<_, _>>()?;

    let mut failures = 0;
    let mut checked = 0;
    for doc_path in &docs {
        let doc = fs::read_to_string(doc_path)
            .map_err(|e| format!("cannot read {}: {e}", doc_path.display()))?;
        let total = quoted_spans(&doc)
            .iter()
            .filter(|(_, s)| checkable(s))
            .count();
        checked += total;
        let bad = unverified_quotations(&doc, &extracts);
        let name = doc_path
            .strip_prefix(repo_root())
            .unwrap_or(doc_path)
            .display();
        if bad.is_empty() {
            println!("cite-check: {name} — {total}/{total} quotations verbatim in the extract");
        } else {
            for (line, span) in &bad {
                let short: String = span.chars().take(140).collect();
                println!("cite-check: {name}:{line}: NOT IN THE EXTRACT\n    {short}");
            }
            failures += bad.len();
        }
    }
    if failures > 0 {
        return Err(format!(
            "{failures} of {checked} quoted span(s) do not appear in the archived text layer. Either \
             the quotation is a PARAPHRASE presented as a quote, or it is a MISREADING of the form — \
             fix the document against the extract, never the extract against the document."
        ));
    }
    println!("cite-check: OK — {checked} quotations, all verbatim.");

    // ★ A tax year whose design corpus nothing reads is an instrument that has fallen off the year.
    let unchecked = unchecked_design_years(&docs, &design_year_dirs()?);
    if !unchecked.is_empty() {
        return Err(format!(
            "design/ty{:?} exist(s) but NO document from them is checked — `schedule_1a_docs()` still \
             names TY2025 documents and TY2025 extracts by hand, so cite-check is reporting success \
             about last year's corpus. Point it at the new year's documents and extracts.",
            unchecked
        ));
    }

    // ★ Report AUTHORITY COVERAGE every run, not only in a test — and report it as `(form, YEAR)`
    // pairs, because a prior-year archive is not coverage for a current-year revision.
    let emitted = emitted_form_years()?;
    let (archived, broken) = archived_form_years()?;
    if !broken.is_empty() {
        return Err(format!(
            "{} map row(s) carry no `authority` header — so they assert their primary source is \
             archived — and the manifest join does not back that claim:\n  {}",
            broken.len(),
            broken.join("\n  ")
        ));
    }
    let excused = excused_form_years()?;
    let verdict = adjudicate_coverage(&emitted, &archived, &excused);
    println!(
        "cite-check: authority archived + extracted for {}/{} emitted (form, year) pairs [{}]; \
         {} excused, {} unaccounted",
        archived.intersection(&emitted).count(),
        emitted.len(),
        render(&archived.iter().cloned().collect::<Vec<_>>()),
        excused.len(),
        verdict.unaccounted.len()
    );
    // ★ The command FAILS on the same conditions the test does. A reporting-only coverage line is an
    //   instrument that cannot fail, and this repo's dominant defect is exactly that shape.
    if !verdict.unaccounted.is_empty()
        || !verdict.stale_excuses.is_empty()
        || !verdict.phantom_excuses.is_empty()
    {
        return Err(format!(
            "authority coverage is not accounted for — unaccounted: [{}]; stale excuses: [{}]; \
             phantom excuses: [{}]. See `authority_coverage_may_only_improve`.",
            render(&verdict.unaccounted),
            render(&verdict.stale_excuses),
            render(&verdict.phantom_excuses)
        ));
    }
    Ok(())
}

/// `cargo run -p xtask -- extract-schedule-1a` — regenerate the committed text-layer extracts from the
/// archived PDFs.
///
/// ★ Two different `pdftotext` invocations, and the difference matters: the FORM needs `-layout` to keep
/// each line's amount boxes on that line, while the INSTRUCTION pages are 3-column and `-layout`
/// interleaves the columns into text that is neither readable nor matchable. Getting this backwards is
/// how an extract silently stops containing the sentences you are checking against.
pub fn extract() -> Result<(), String> {
    let root = repo_root();
    // ★ Driven by the FORMS registry, not a hardcoded pair: every form follows the same pattern, so
    // adding one is a table entry. The two `pdftotext` invocations differ and the difference matters —
    // `-layout` keeps a FORM's amount boxes on their own line, but interleaves 3-column INSTRUCTION pages
    // into text that is neither readable nor matchable.
    let mut jobs: Vec<(String, String, Vec<String>, &str)> = Vec::new();
    for f in FORMS {
        if f.extract_stem.is_empty() {
            continue;
        }
        jobs.push((
            format!("design/forms/{}/{}--{}.pdf", f.year, f.form, f.year),
            format!(
                "crates/btctax-core/src/tax/fixtures/{}_form.txt",
                f.extract_stem
            ),
            vec!["-layout".to_string()],
            "THE MANUAL, as a checkable fixture",
        ));
        if f.instructions.is_empty() {
            continue;
        }
        let mut flags: Vec<String> = Vec::new();
        if let Some((first, last)) = f.instr_pages {
            flags.extend([
                "-f".to_string(),
                first.to_string(),
                "-l".to_string(),
                last.to_string(),
            ]);
        }
        jobs.push((
            format!("design/forms/{}/{}--{}.pdf", f.year, f.instructions, f.year),
            format!(
                "crates/btctax-core/src/tax/fixtures/{}_instructions.txt",
                f.extract_stem
            ),
            flags,
            "THE INSTRUCTIONS, as a checkable fixture",
        ));
    }
    for (pdf, out, flags, title) in &jobs {
        let pdf_path = root.join(pdf);
        let bytes = fs::read(&pdf_path).map_err(|e| format!("cannot read {pdf}: {e}"))?;
        let hash = sha256_prefix(&bytes);
        let mut cmd = std::process::Command::new("pdftotext");
        cmd.args(flags.iter()).arg(&pdf_path).arg("-");
        let got = cmd
            .output()
            .map_err(|e| format!("pdftotext failed (is poppler-utils installed?): {e}"))?;
        if !got.status.success() {
            return Err(format!("pdftotext exited {:?} on {pdf}", got.status.code()));
        }
        let body = String::from_utf8_lossy(&got.stdout);
        let mut text = format!(
            "# GENERATED — do not hand-edit. {title}.\n#\n# Source: {pdf}  sha256:{hash}…\n\
             # Command: pdftotext {} <pdf> -\n#\n\
             # ★ Extracted from the TEXT LAYER, never a rendered page: a rendered 12 and 22 differ by a\n\
             # few pixels, and that exact confusion once put \"Subtract line 32 from line 12\" where\n\
             # Form 6251 says line 22, inflating a tentative minimum tax by $200,000.\n\
             #   cargo run -p xtask -- extract-schedule-1a\n#\n",
            flags.join(" ")
        );
        for line in body.lines() {
            text.push_str(line.trim_end());
            text.push('\n');
        }
        fs::write(root.join(out), &text).map_err(|e| format!("cannot write {out}: {e}"))?;
        println!(
            "extract-schedule-1a: {out} ({} lines, sha256:{hash}…)",
            text.lines().count()
        );
    }
    Ok(())
}

/// First 8 hex chars of the SHA-256, matching how the specs cite these PDFs. Hand-rolled to avoid adding
/// a dependency to dev-only tooling.
fn sha256_prefix(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = bytes.to_vec();
    let bitlen = (bytes.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(v[i]);
        }
    }
    format!("{:08x}", h[0])
}

// ── The form registry: every form we support follows the SAME pattern ─────────────────────────────

/// **Every IRS form btctax emits, paired with the authority that defines it.**
///
/// ★★ The generalisation that makes this file worth having (user, 2026-07-29): *"Every form we ever
/// support will follow this exact pattern. And all have simple instructions in the form or the
/// identically numbered instructions document available from the IRS."*
///
/// The IRS naming is **mechanical**: form `fNNNN` has instructions `iNNNN` (`f6251`→`i6251`,
/// `f1040sa`→`i1040sca`), with the 1040 family's general instructions `i1040gi` carrying the schedules
/// that have no standalone booklet. So adding a form is a **table entry plus a transcription**, never a
/// bespoke project: archive the pair, extract the text layer, enumerate the label set *from the extract*,
/// one field per label with the instruction text verbatim as its doc comment, derive each decision
/// (direction, constant, cross-reference) from the line's own words, and classify each label's provenance.
///
/// `instr_pages` is `None` when the instructions are a standalone booklet (extract all of it) and
/// `Some((first, last))` when they are a section of a larger one, as Schedule 1-A is of `i1040gi`.
pub struct FormAuthority {
    /// IRS form basename, e.g. `"f1040s1a"` — also the `design/` filename stem.
    pub form: &'static str,
    pub year: i32,
    /// IRS instructions basename. `""` for the handful of forms the IRS publishes with the instructions
    /// printed ON the form itself (Form 8275 is one) — those are self-authorising.
    pub instructions: &'static str,
    pub instr_pages: Option<(u32, u32)>,
    /// Committed text-layer extract stem under `crates/btctax-core/src/tax/fixtures/`, or `""` if not
    /// yet extracted.
    pub extract_stem: &'static str,
}

/// ★★ **THIS IS NO LONGER THE AUTHORITY RATCHET'S INPUT (FR-138, 2026-09-12).** It is now only the
/// driver for `extract-schedule-1a`, which regenerates the SECOND extract root
/// (`crates/btctax-core/src/tax/fixtures/`) that design r2 §9 plans to retire. The ratchet reads the
/// year-package table's rows and `design/forms/MANIFEST.json` instead — see [`archived_form_years`] —
/// because keying coverage on *this* const meant a form-year could be archived, extracted, hashed and
/// manifested and still be counted as uncovered. Rows are held to their map rows by
/// `the_forms_const_row_agrees_with_its_map_row` until the root retires and the const goes with it.
///
/// ★ **Form 4868 and Form 1040-V are their own instructions documents** — the IRS publishes no
/// `i4868` and no `i1040v`, so `instructions` names the form itself and `instr_pages` is the range
/// each map row MEASURED from its extract's form-feed page breaks ([1,4] and [1,2]).
/// `the_forms_const_row_agrees_with_its_map_row` holds these five rows to those declarations, so a
/// page range cannot drift here without reddening.
pub const FORMS: &[FormAuthority] = &[
    // ★★★ T16 / FR-76 — Form 8889 reached STEP 2 the day it was transcribed, which is the point of
    //     the ladder: the transcription in `btctax_core::tax::form8889` is checked line by line
    //     against these fixtures by `xtask line-coverage`, so the form is not merely archived but
    //     CHECKABLE. `instr_pages` is `None` because the whole i8889 booklet is the authority (its
    //     Line-N blocks are what `line-coverage`'s `FilerRecords` rows quote from).
    FormAuthority {
        form: "f8889",
        year: 2024,
        instructions: "i8889",
        instr_pages: None,
        extract_stem: "f8889_2024",
    },
    FormAuthority {
        form: "f8889",
        year: 2025,
        instructions: "i8889",
        instr_pages: None,
        extract_stem: "f8889_2025",
    },
    FormAuthority {
        form: "f1040s1a",
        year: 2025,
        instructions: "i1040gi",
        instr_pages: Some((101, 110)),
        extract_stem: "schedule_1a_2025",
    },
    FormAuthority {
        form: "f1040v",
        year: 2024,
        instructions: "f1040v",
        instr_pages: Some((1, 2)),
        extract_stem: "f1040v_2024",
    },
    FormAuthority {
        form: "f1040v",
        year: 2025,
        instructions: "f1040v",
        instr_pages: Some((1, 2)),
        extract_stem: "f1040v_2025",
    },
    FormAuthority {
        form: "f4868",
        year: 2024,
        instructions: "f4868",
        instr_pages: Some((1, 4)),
        extract_stem: "f4868_2024",
    },
    FormAuthority {
        form: "f4868",
        year: 2025,
        instructions: "f4868",
        instr_pages: Some((1, 4)),
        extract_stem: "f4868_2025",
    },
];

// ── The emitting surface: (form, YEAR), derived — never a hand-list ───────────────────────────────

/// One authority obligation: an IRS form basename and the **tax year of the revision we print**.
///
/// ★★ **The year is half the key, and dropping it is how an instrument reports success while checking
/// the wrong document.** A form's lines are renumbered, added and deleted between revisions — the
/// TY2026 Schedule 1-A draft keeps 10 of the TY2025 revision's 219 AcroForm field names — so an archive of the
/// TY2025 booklet says nothing whatever about a TY2026 transcription. This ratchet used to collect
/// `archived` as a set of form basenames with `FormAuthority::year` discarded, so the one archived row
/// (`f1040s1a`, 2025) would have discharged the TY2026 obligation for the same form, and `cite-check`
/// would have gone on verifying a TY2026 spec against the TY2025 extract while printing coverage.
pub type FormYear = (String, i32);

/// Where btctax keeps the fillable blanks it embeds — one directory per tax year.
const TEMPLATE_ROOT: &str = "crates/btctax-forms/forms";

/// Template stems that are **not** already the IRS basename, and what each one means.
///
/// ★ Deliberately tiny and deliberately TOTAL. Every other stem must already be an IRS basename
/// (`f` followed by a digit); anything else is an **error**, never a skip. A stem this table cannot
/// translate would drop a form out of the obligation set silently, which is the exact class of failure
/// this module exists to prevent.
const STEM_ALIASES: &[(&str, &str)] = &[("schedule_d", "f1040sd"), ("schedule_se", "f1040sse")];

/// Translate a template stem into the IRS basename that names its authority PDF
/// (`design/forms/<year>/<basename>--<year>.pdf`).
pub fn irs_basename(stem: &str) -> Result<&str, String> {
    if let Some((_, irs)) = STEM_ALIASES.iter().find(|(s, _)| *s == stem) {
        return Ok(irs);
    }
    let mut c = stem.chars();
    if c.next() == Some('f') && c.next().is_some_and(|d| d.is_ascii_digit()) {
        return Ok(stem);
    }
    Err(format!(
        "template stem {stem:?} is neither an IRS basename (`fNNNN…`) nor listed in STEM_ALIASES. \
         Refusing to guess: an untranslated stem silently leaves that form out of the authority \
         obligation set, which is a form whose transcription nothing checks."
    ))
}

/// **Every `(form, year)` btctax can put on paper — read off the filesystem, never enumerated by hand.**
///
/// The emitting surface is `crates/btctax-forms/forms/<year>/<stem>.pdf` plus its `<stem>.map.toml`:
/// an embedded blank and a field map together **are** a transcription of that revision's grid, so each
/// pair is a form-year whose primary source we owe. Deriving it here is what makes the year transition
/// fail **closed** — dropping a TY2026 template directory into the tree creates a new obligation per
/// form and reds the ratchet until each is archived or consciously excused.
///
/// ★ Measured 2026-09-05, and the reason this is a function and not a `const`: the hand-written
/// `EMITTED_FORMS` it replaces listed **16** form basenames. The real surface was **18 stems / 37
/// (form, year) pairs** then, went to **20 stems / 41** when the Form 4868 and Form 1040-V rows
/// landed (2026-09-06), and is **20 stems / 36** since S9 dropped the five TY2017 pairs the same
/// day (no stem was lost: all five have a 2024 and/or 2025 revision). The hand-list omitted
/// `f1040s1` and `f8995a` outright — both of which
/// `btctax-forms/src/packet.rs` pushes into the filed packet — so the ratchet passed on those two
/// forms by finding nothing to check.
pub fn emitted_form_years() -> Result<BTreeSet<FormYear>, String> {
    let root = repo_root().join(TEMPLATE_ROOT);
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .map_err(|e| format!("cannot read {}: {e}", root.display()))?
        .map(|e| {
            e.map(|e| e.path())
                .map_err(|e| format!("{}: {e}", root.display()))
        })
        .collect::<Result<_, _>>()?;
    dirs.sort();

    let mut out: BTreeSet<FormYear> = BTreeSet::new();
    let mut year_dirs = 0usize;
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let Some(year) = dir
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };
        year_dirs += 1;

        let mut pdfs: BTreeSet<String> = BTreeSet::new();
        let mut maps: BTreeSet<String> = BTreeSet::new();
        for entry in
            fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        {
            let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            // `.map.toml` FIRST — otherwise nothing, but a stem ending in `.map` would be invented.
            if let Some(stem) = name.strip_suffix(".map.toml") {
                maps.insert(stem.to_string());
            } else if let Some(stem) = name.strip_suffix(".pdf") {
                pdfs.insert(stem.to_string());
            }
        }
        // ★ A blank with no map, or a map with no blank, is a half-transcription. Fail rather than
        //   pick one side: which side we picked would decide, silently, whether an obligation exists.
        let lonely: Vec<&String> = pdfs.symmetric_difference(&maps).collect();
        if !lonely.is_empty() {
            return Err(format!(
                "{}: {lonely:?} has a blank without a field map or a map without a blank — the \
                 emitting surface is ambiguous there, so the authority obligation cannot be derived",
                dir.display()
            ));
        }
        for stem in &pdfs {
            out.insert((irs_basename(stem)?.to_string(), year));
        }
    }

    // ★★ Guard the guard. A derivation that finds nothing would make every check below pass by
    //    finding nothing — the dominant defect shape in this repo. The 1040 itself is pushed
    //    unconditionally by `packet.rs` for every supported year, so its absence means the walk broke,
    //    not that the product changed.
    if year_dirs == 0 || out.is_empty() || !out.iter().any(|(f, _)| f == "f1040") {
        return Err(format!(
            "the emitting surface came back as {} pair(s) across {year_dirs} year directory(ies) under \
             {} and does not contain f1040 — the derivation is broken, and a broken derivation makes \
             the authority ratchet pass vacuously",
            out.len(),
            root.display()
        ));
    }
    Ok(out)
}

/// **Every bundled map's ROW — design r2 §4's year-package table, read off the glob.**
///
/// ★ The row SET *is* the glob: a `.map.toml` file IS a row, so there is no central list to forget one
/// from. This is the non-`cfg(test)` reader because the authority ratchet also runs as a command, and a
/// coverage line that only reports is an instrument that cannot fail.
///
/// ★ A row whose `year` key disagrees with the directory it lives in is a **refusal**, not a skip: every
/// path this module builds (`design/forms/<year>/<stem>--<year>.pdf`) is keyed on the year, so a
/// disagreement would silently look up the wrong revision's archive.
pub fn map_rows() -> Result<Vec<(String, i32, btctax_forms::MapRow)>, String> {
    let root = repo_root().join(TEMPLATE_ROOT);
    let mut year_dirs: Vec<PathBuf> = fs::read_dir(&root)
        .map_err(|e| format!("cannot read {}: {e}", root.display()))?
        .map(|e| {
            e.map(|e| e.path())
                .map_err(|e| format!("{}: {e}", root.display()))
        })
        .collect::<Result<_, _>>()?;
    year_dirs.sort();

    let mut out = Vec::new();
    for dir in year_dirs {
        if !dir.is_dir() {
            continue;
        }
        let Some(year) = dir
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
            .map(|e| {
                e.map(|e| e.path())
                    .map_err(|e| format!("{}: {e}", dir.display()))
            })
            .collect::<Result<_, _>>()?;
        files.sort();
        for path in files {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let Some(stem) = name.strip_suffix(".map.toml") else {
                continue;
            };
            let stem = stem.to_string();
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let row = btctax_forms::MapRow::read(&text)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if row.year != year {
                return Err(format!(
                    "{}: the row says year = {} but it lives in forms/{year}/ — every archive path \
                     this module builds is keyed on the year, so the two may not disagree",
                    path.display(),
                    row.year
                ));
            }
            out.push((stem, year, row));
        }
    }

    // ★★ Guard the guard. A walk that found nothing would make every check built on it pass by
    //    finding nothing — the dominant defect shape in this repo. The 1040 is bundled for every
    //    supported year, so its absence means the walk broke, not that the product changed.
    if out.is_empty() || !out.iter().any(|(_, _, r)| r.irs_stem == "f1040") {
        return Err(format!(
            "the year-package table came back as {} row(s) under {} and does not contain f1040 — the \
             walk is broken, and a broken walk makes the authority ratchet pass vacuously",
            out.len(),
            root.display()
        ));
    }
    Ok(out)
}

/// The ONE excuse a map row may carry, and the prefix it must be spelled with
/// (`btctax_forms::MapRow::authority`, design r2 §9; `btctax-forms/tests/map_rows.rs` holds the same
/// spelling from the other side).
const EXCUSE_PREFIX: &str = "not-yet-archived: ";

/// The REASON a row's `authority` value gives, or `None` when the value is not an excuse at all.
///
/// ★ One function, so the thing that decides is the thing the kill plants against. An `authority` slot
/// that accepted any string would accept an admission nobody has to justify — and the 31 hand-typed
/// pairs this replaced carried no reason whatever.
fn excuse_reason(authority: &str) -> Option<&str> {
    authority
        .strip_prefix(EXCUSE_PREFIX)
        .map(str::trim)
        .filter(|r| !r.is_empty())
}

/// ★★ **THE RATCHET'S EXCUSE SET, keyed on `(form, YEAR)` and READ OFF THE ROWS THAT CARRY THE REASON.**
/// Every pair here is a form-year btctax can print while holding no archived, extracted primary source,
/// so its transcription is unverifiable by `cite-check` and by the derive-the-decision-from-the-line
/// tests. **The set may only SHRINK**, and its shrink-only human declaration is
/// `btctax-forms/tests/map_rows.rs::EXCUSED`, which pins it *exactly* — both directions — and refuses
/// any `authority` value not spelled [`EXCUSE_PREFIX`].
///
/// ★★★ **FR-138 (rehearsal F5, 2026-09-12) — this used to be a hand-typed `AUTHORITY_NOT_YET_ARCHIVED`
/// const of 31 bare `(form, years)` pairs, and 30 of the 31 were FALSE.** The ratchet's notion of
/// "archived" was *"[`FORMS`] has a row **and** a duplicate extract pair exists under
/// `crates/btctax-core/src/tax/fixtures/`"*, so a form-year whose PDF was archived with a URL, a
/// sha256, a `MANIFEST.json` entry and both committed text layers was still reported `unaccounted` —
/// and the cheapest way to make the ratchet green was to write *"not-yet-archived"* about a document
/// that was archived. **A gate whose cheapest discharge is a false statement does not merely fail to
/// catch things; it rewards writing something untrue into a committed file.** Re-pointing the join at
/// `design/forms/MANIFEST.json` plus the `design/forms/extract/` convention took the excuse set from
/// **31 pairs to 1** and created **no** archive to do it: 37 of the 38 emitted pairs were *already*
/// archived and extracted, and the ratchet could not see a single one of them.
///
/// ★ There is deliberately **no wildcard and no "all supported years" sentinel**, and none is now
/// expressible: an excuse is a key on one map file, so a new tax year cannot be pre-excused in bulk —
/// adding TY2026 templates reddens the ratchet per form until each is archived or its own row says why
/// not, *with a reason*. The 31 pairs this replaced carried no reason at all.
///
/// ★ `phantom_excuses` (excused but not emitted) is **unreachable from real data by construction** now,
/// because an excuse comes from a `.map.toml` and [`emitted_form_years`] refuses a map without its
/// blank. That is a strengthening, not a blind spot — the arm is still exercised on planted sets by
/// `a_prior_year_archive_does_not_discharge_a_new_year_obligation`.
pub fn excused_form_years() -> Result<BTreeSet<FormYear>, String> {
    let mut out = BTreeSet::new();
    for (stem, year, row) in map_rows()? {
        let Some(excuse) = row.authority.as_deref() else {
            continue;
        };
        // ★ An arbitrary string may not excuse. `authority` is the excuse SLOT, and a slot that
        //   accepts anything is a slot that accepts a sentence nobody has to justify.
        if excuse_reason(excuse).is_none() {
            return Err(format!(
                "forms/{year}/{stem}.map.toml: `authority = {excuse:?}` — the only excuse this \
                 ratchet accepts is `\"{EXCUSE_PREFIX}<reason>\"` with a non-empty reason, because an \
                 admission with no reason is indistinguishable from a shrug"
            ));
        }
        out.insert((row.irs_stem.clone(), year));
    }
    Ok(out)
}

/// **What it takes for a `(form, year)` to hold an archived, extracted primary source.**
///
/// One notion, built from the two conventions the rest of the repo already uses, so there is no second
/// registry to keep in step:
///
/// 1. the row's own PDF **and** its instructions' PDF *for that year* are entries in
///    `design/forms/MANIFEST.json` — which is what carries the URL, sha256 and byte count, i.e. the
///    provenance that makes a file an *archive* rather than a text file someone typed;
/// 2. each entry is IRS-final, never a draft (`authority_manifest::Entry::is_authority`);
/// 3. each entry records its text layer at the conventional path
///    `design/forms/extract/<stem>--<year>.txt` — the path [`btctax_forms::MapRow`]'s own doc comment
///    calls *"derived by convention, never stored"*; and
/// 4. that file is on disk.
///
/// ★ Every input is COMMITTED, so the ratchet runs in an isolated worktree. The archived PDFs
/// themselves are gitignored and absent there (the port rehearsal measured **0 of 125**), which is why
/// the join is to the manifest's recorded hash and extract rather than to the bytes.
///
/// ★★ **WHAT THIS DOES NOT CLAIM, stated so nobody reads it as more.** "Archived and extracted" means
/// *the authority exists to check a transcription against* — it does **not** mean any instrument has
/// checked one. That distinction is the whole difference between this ratchet and the coverage
/// instruments that consume the same extracts (`xtask line-coverage`, `census_join`,
/// `btctax-forms/tests/map_rows.rs`), and conflating the two would turn a precondition into a false
/// completeness claim. `design/forms/extract/` is the surface all of them read, which is why it — and
/// not the legacy `crates/btctax-core/src/tax/fixtures/` root design r2 §9 retires — is what this joins
/// to.
struct Archive {
    root: PathBuf,
    by_path: BTreeMap<String, crate::authority_manifest::Entry>,
}

impl Archive {
    fn load(root: &Path) -> Result<Self, String> {
        let by_path = crate::authority_manifest::load(root)?
            .into_iter()
            .map(|e| (e.path.clone(), e))
            .collect();
        Ok(Self {
            root: root.to_path_buf(),
            by_path,
        })
    }

    /// The authority documents one row names, each as `(PDF path, conventional extract path)`.
    ///
    /// ★ ONE document when the form **is** its own instructions — the IRS publishes no `i4868` and no
    /// `i1040v`, and both map rows say so by naming themselves in `instructions`. Emitting the same
    /// path twice would double every problem message for those two forms.
    fn documents(row: &btctax_forms::MapRow) -> Vec<(String, String)> {
        let doc = |stem: &str| {
            (
                format!("design/forms/{}/{stem}--{}.pdf", row.year, row.year),
                format!("design/forms/extract/{stem}--{}.txt", row.year),
            )
        };
        let form = doc(&row.irs_stem);
        let instructions = doc(&row.instructions);
        if instructions.0 == form.0 {
            vec![form]
        } else {
            vec![form, instructions]
        }
    }

    /// Why this row does **not** hold an archived, extracted primary source. Empty = archived.
    ///
    /// ★ Every arm names the document and says what is wrong with it, because the message a January
    /// operator reads decides whether they go and archive something or go and re-check an archive that
    /// is already complete. The message this replaced said *"btctax can print [f8995a--2025] with no
    /// archived primary source for THAT YEAR"* about a form whose PDF, note, URL, sha256, manifest
    /// entry and text layer were all committed — the real gap was `i8995a--2025`, which it never named.
    fn problems_for(&self, row: &btctax_forms::MapRow) -> Vec<String> {
        let mut out = Vec::new();
        for (pdf, extract) in Self::documents(row) {
            let Some(entry) = self.by_path.get(&pdf) else {
                out.push(format!(
                    "{pdf} is not in design/forms/MANIFEST.json — archive the document with its URL \
                     and sha256, then `xtask authority-manifest --regen`"
                ));
                continue;
            };
            if !entry.is_authority() {
                out.push(format!(
                    "{pdf} is a DRAFT in the manifest ({}) — a draft is evidence, never authority to \
                     transcribe a figure or a line number from",
                    entry.url
                ));
                continue;
            }
            if entry.extract != extract {
                out.push(format!(
                    "{pdf} records extract {:?}, not the conventional {extract} — every consumer of \
                     the text layer resolves that path by convention, so a manifest entry pointing \
                     elsewhere is coverage nothing reads",
                    entry.extract
                ));
                continue;
            }
            if !self.root.join(&extract).exists() {
                out.push(format!(
                    "{pdf} records extract {extract}, which is NOT on disk"
                ));
            }
        }
        out
    }
}

/// The `(form, year)` pairs that hold an archived, extracted primary source, and the rows whose claim
/// does not survive contact with the disk.
///
/// ★ **Skipping is not passing.** A row that carries no `authority` header is *asserting* its primary
/// source is archived, so a failed join is returned in the second element and FAILS by name — never
/// quietly stops counting. A row that carries `authority = "not-yet-archived: …"` is neither archived
/// nor broken: it is the excuse, and [`excused_form_years`] returns it.
pub fn archived_form_years() -> Result<(BTreeSet<FormYear>, Vec<String>), String> {
    let root = repo_root();
    let archive = Archive::load(&root)?;
    let mut ok = BTreeSet::new();
    let mut broken = Vec::new();
    for (stem, year, row) in map_rows()? {
        if row.authority.is_some() {
            continue;
        }
        let problems = archive.problems_for(&row);
        if problems.is_empty() {
            ok.insert((row.irs_stem.clone(), year));
        } else {
            broken.push(format!("forms/{year}/{stem}: {}", problems.join("; ")));
        }
    }
    Ok((ok, broken))
}

/// The three ways authority coverage can be wrong.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CoverageVerdict {
    /// Archived AND excused — a stale excuse, which is how a closed gap silently reopens.
    pub stale_excuses: Vec<FormYear>,
    /// Emitted, not archived, not excused — a form-year whose transcription nothing can check.
    pub unaccounted: Vec<FormYear>,
    /// Excused but not emitted — the excuse list rotting into a wishlist.
    pub phantom_excuses: Vec<FormYear>,
}

/// Adjudicate coverage. **Pure**, taking all three sets as arguments, so the ratchet's own logic can be
/// exercised against *planted* inputs — a year-mismatched archive, a stale excuse — instead of only
/// against whatever the repository happens to contain today.
pub fn adjudicate_coverage(
    emitted: &BTreeSet<FormYear>,
    archived: &BTreeSet<FormYear>,
    excused: &BTreeSet<FormYear>,
) -> CoverageVerdict {
    let accounted: BTreeSet<FormYear> = archived.union(excused).cloned().collect();
    CoverageVerdict {
        stale_excuses: archived.intersection(excused).cloned().collect(),
        unaccounted: emitted.difference(&accounted).cloned().collect(),
        phantom_excuses: excused.difference(emitted).cloned().collect(),
    }
}

/// Render a `(form, year)` list the way the archive names it on disk: `f6251--2025`.
fn render(pairs: &[FormYear]) -> String {
    pairs
        .iter()
        .map(|(f, y)| format!("{f}--{y}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★ **THE POINT OF THIS FILE.** Every quotation in the Schedule 1-A spec and plan is verbatim
    /// from the archived form or instructions. This is the mechanical answer to the "§X disagrees with
    /// §Y" review finding: two sections that each match the manual cannot disagree with each other, so
    /// the class stops needing a reviewer at all.
    #[test]
    fn every_quotation_in_the_schedule_1a_documents_is_verbatim_from_the_manual() {
        let (docs, extract_paths) = schedule_1a_docs();
        let extracts: Vec<String> = extract_paths
            .iter()
            .map(|p| {
                fs::read_to_string(p)
                    .unwrap_or_else(|e| panic!("extract fixture missing: {} ({e})", p.display()))
            })
            .collect();
        let mut all_bad = Vec::new();
        let mut checked = 0;
        for doc_path in &docs {
            let doc = fs::read_to_string(doc_path)
                .unwrap_or_else(|e| panic!("design doc missing: {} ({e})", doc_path.display()));
            checked += quoted_spans(&doc)
                .iter()
                .filter(|(_, s)| checkable(s))
                .count();
            for (line, span) in unverified_quotations(&doc, &extracts) {
                all_bad.push(format!(
                    "{}:{line}: {}",
                    doc_path.file_name().unwrap().to_string_lossy(),
                    span.chars().take(120).collect::<String>()
                ));
            }
        }
        assert!(
            all_bad.is_empty(),
            "{} quoted span(s) are not in the archived text layer — a paraphrase presented as a \
             quotation, or a misreading of the form:\n  {}",
            all_bad.len(),
            all_bad.join("\n  ")
        );
        // ★ Guard the guard: if the extractor stops finding quotations, this test passes vacuously.
        assert!(
            checked >= 20,
            "only {checked} quotations found — the span extractor has probably broken, which would \
             make this test pass by finding nothing"
        );
    }

    /// ★★ **THE AUTHORITY RATCHET, keyed on `(form, YEAR)`.** Every form-year btctax can print either
    /// holds an archived, extracted primary source — its PDF and its instructions' PDF are
    /// `design/forms/MANIFEST.json` entries naming committed text layers — or its own map row says why
    /// not, with a reason. The excuse set may only shrink.
    ///
    /// Without this, adding a form emitter — or a new tax year's templates — is a silent regression:
    /// nothing compels anyone to archive the PDF that defines it, so the transcription becomes
    /// unverifiable and every downstream conformance test (the `cite-check` quotations, the
    /// derive-the-direction-from-the-line assertions, the label census) has nothing to check against
    /// and passes by finding nothing.
    ///
    /// ★ **All three sides are now DERIVED** (FR-138): the obligation set off the template
    /// directories, the archived set off the manifest join, and the excuse set off the `authority`
    /// header of the row it excuses. Nothing here is a hand-list. Its shrink-only human declaration is
    /// `btctax-forms/tests/map_rows.rs::EXCUSED`, which pins the header set exactly.
    #[test]
    fn authority_coverage_may_only_improve() {
        let emitted = emitted_form_years().expect("the emitting surface must be derivable");
        let (archived, broken) =
            archived_form_years().expect("the authority join must be derivable");
        assert!(
            broken.is_empty(),
            "{} map row(s) carry no `authority` header — so they assert their primary source is \
             archived — and the manifest join does not back that claim:\n  {}",
            broken.len(),
            broken.join("\n  ")
        );
        let excused = excused_form_years().expect("the excuse set must be derivable");
        let verdict = adjudicate_coverage(&emitted, &archived, &excused);

        // 1. Nothing may be BOTH archived and excused — a stale excuse is how a closed gap silently
        //    reopens for the next form.
        assert!(
            verdict.stale_excuses.is_empty(),
            "[{}] now HAS an archived authority for that YEAR — delete the `authority` header from \
             that map row (and from map_rows.rs::EXCUSED) so the ratchet actually tightens",
            render(&verdict.stale_excuses)
        );

        // 2. Every emitted (form, year) is accounted for: archived, or consciously excused.
        assert!(
            verdict.unaccounted.is_empty(),
            "btctax can print [{}] with no archived primary source for THAT YEAR and no explicit \
             excuse. Every form follows one pattern and every one has an identically-numbered IRS \
             instructions document — archive the PAIR (the form and its instructions), extract both \
             text layers to design/forms/extract/, and regen the manifest; or write \
             `authority = \"{EXCUSE_PREFIX}<reason>\"` into that map row. Silence here means a \
             form-year whose transcription NOTHING can check.",
            render(&verdict.unaccounted)
        );

        // 3. Every excuse names a form-year we can actually print — otherwise the list rots into a
        //    wishlist, and a year that has been retired keeps pretending to be an open admission.
        //    ★ Since FR-138 this is unreachable from real data BY CONSTRUCTION (an excuse is a key on
        //    a `.map.toml`, and `emitted_form_years` refuses a map with no blank beside it), which is
        //    why the arm's kill lives on planted sets in
        //    `a_prior_year_archive_does_not_discharge_a_new_year_obligation`.
        assert!(
            verdict.phantom_excuses.is_empty(),
            "[{}] are excused but have no template on disk for that year",
            render(&verdict.phantom_excuses)
        );
    }

    /// ★★ **THE RATCHET MAY ONLY SHRINK — planted on the REAL sets, one pair at a time.**
    ///
    /// `authority_coverage_may_only_improve`'s first assertion is that nothing is both archived and
    /// excused. That assertion is what stops a closed gap from silently reopening: writing
    /// `authority = "not-yet-archived: …"` back into a map row whose archive exists would otherwise be
    /// a green edit that quietly withdraws the coverage.
    ///
    /// It had **no kill**. The sibling below plants on synthetic sets and exercises `unaccounted`
    /// and `phantom_excuses`; the `stale_excuses` arm was asserted by the ratchet and observed by
    /// nothing — an instrument nobody had watched discriminate (B1). Written 2026-09-06 with the
    /// four Form 4868 / Form 1040-V pairs that just left the list, which is exactly the class of
    /// edit it has to catch.
    ///
    /// ★ Both directions, and the plant is enumerated FROM the archived set itself rather than
    /// hand-listed, so it covers whatever is archived at the time and cannot rot into naming a row
    /// that no longer exists. FR-138 moved it off [`FORMS`] — which held 7 of the 38 pairs — and onto
    /// the join, so the plant count is now the coverage count and is asserted against it rather than
    /// against a literal.
    #[test]
    fn re_excusing_an_archived_pair_reds_the_ratchet() {
        let emitted = emitted_form_years().expect("the emitting surface must be derivable");
        let (archived, broken) =
            archived_form_years().expect("the authority join must be derivable");
        assert!(
            broken.is_empty(),
            "premise: every row that claims an archive has one: {broken:?}"
        );
        let excused = excused_form_years().expect("the excuse set must be derivable");

        // CONTROL: today's real sets have no stale excuse. Without this half a checker that
        // reported everything as stale would pass the plants below.
        assert!(
            adjudicate_coverage(&emitted, &archived, &excused)
                .stale_excuses
                .is_empty(),
            "premise: nothing is currently both archived and excused"
        );

        // …and the four pairs this test was written for really are on the archived side now.
        for pair in [
            ("f4868".to_string(), 2024),
            ("f4868".to_string(), 2025),
            ("f1040v".to_string(), 2024),
            ("f1040v".to_string(), 2025),
        ] {
            assert!(
                archived.contains(&pair),
                "premise: {pair:?} joins to a manifest entry with its extract on disk"
            );
            assert!(
                !excused.contains(&pair),
                "premise: {pair:?}'s map row carries no `authority` header"
            );
        }

        // PLANT, one archived pair at a time: re-excuse it and the ratchet must NAME it.
        let mut planted = 0usize;
        for pair in &archived {
            let pair = pair.clone();
            let mut widened = excused.clone();
            widened.insert(pair.clone());
            let verdict = adjudicate_coverage(&emitted, &archived, &widened);
            assert_eq!(
                verdict.stale_excuses,
                vec![pair.clone()],
                "re-excusing {pair:?} — which HAS an archived, extracted authority — must be \
                 reported as a stale excuse; the ratchet may only shrink"
            );
            // and re-excusing it may not be laundered as some other verdict
            assert!(
                verdict.unaccounted.is_empty() && verdict.phantom_excuses.is_empty(),
                "{pair:?}: the stale-excuse arm is the one that must fire, not another: {verdict:?}"
            );
            planted += 1;
        }
        // ★ Guard the guard, DERIVED: every archived pair must have been planted, and there must be
        //   some. A loop that ran zero times would pass every assertion inside it, and a literal here
        //   would be the FR-150 shape — a number that moves on every port, pinned by hand.
        assert_eq!(planted, archived.len());
        // ★ And the partition is exact, which is what makes the count above non-vacuous without a
        //   literal: `broken` is empty (asserted at the top), so every emitted pair is either archived
        //   or excused and nothing is both. Written as addition, never subtraction — a `usize`
        //   subtraction here would turn a set-relation defect into an overflow panic.
        assert_eq!(
            planted + excused.len(),
            emitted.len(),
            "{planted} archived + {} excused ≠ {} emitted — the join has stopped seeing coverage that \
             exists, or an excuse has escaped the emitting surface",
            excused.len(),
            emitted.len()
        );
    }

    /// ★★★ **FR-138's KILL — an ARCHIVED form-year is accepted with nobody writing "not-yet-archived"
    /// about it, and an UNARCHIVED one still reds.**
    ///
    /// The defect this closes (rehearsal F5, 2026-09-12): `archived_form_years()` counted a
    /// `(form, year)` as archived only if the hand-written [`FORMS`] const had a row for it **and** a
    /// duplicate extract pair existed under `crates/btctax-core/src/tax/fixtures/`. A form-year with a
    /// committed PDF, a provenance note carrying its URL and sha256, a `MANIFEST.json` entry and both
    /// text layers was therefore `unaccounted`, and the cheapest way to make the ratchet green was to
    /// write *"not-yet-archived"* about a document that was archived.
    ///
    /// ★★ Both halves are required, and the second is the one that makes this a kill rather than a
    /// relaxation: **a gate that stops complaining is not a gate.** So the plant is run in both
    /// directions on REAL archives, with nothing synthetic on the manifest side:
    ///
    /// | plant | the row | what the tree really holds | verdict |
    /// |---|---|---|---|
    /// | (a) | Schedule 1 (`f1040s1`) ported to TY2025 | `f1040s1--2025` + `i1040gi--2025`, both manifested with their extracts | **archived** |
    /// | (b) | Form 8995-A (`f8995a`) ported to TY2025 — the rehearsal's own case | `f8995a--2025` yes; **`i8995a--2025` no entry, no extract** | **reds, naming the instructions** |
    ///
    /// ★ (b) is also the sharpest correction to the old message. It said *"btctax can print
    /// [f8995a--2025] with no archived primary source for THAT YEAR"* — of a form whose own PDF, note,
    /// URL, sha256, manifest entry and text layer are all committed. The real gap is the instructions
    /// booklet, which the old ratchet could not name because it never looked at one.
    #[test]
    fn an_archived_form_year_is_accepted_and_an_unarchived_one_still_reds() {
        let root = repo_root();
        let archive = Archive::load(&root).expect("MANIFEST.json loads");
        let rows = map_rows().expect("the year-package table derives");
        // A real committed row, re-pointed at another year — exactly the edit runbook step 21 makes.
        let ported = |stem: &str, year: i32| {
            let (_, _, mut row) = rows
                .iter()
                .find(|(s, _, _)| s == stem)
                .cloned()
                .unwrap_or_else(|| panic!("premise: a {stem} row exists to port"));
            row.year = year;
            row.line_set = format!("{stem}/{year}");
            // ★ NOBODY WRITES AN EXCUSE. That is the entire point of the plant.
            row.authority = None;
            row
        };

        // PLANT (a) — genuinely archived. The ratchet must accept it with no excuse anywhere.
        let schedule_1 = ported("f1040s1", 2025);
        assert_eq!(
            archive.problems_for(&schedule_1),
            Vec::<String>::new(),
            "f1040s1--2025 and i1040gi--2025 are both manifested with their text layers committed, so \
             a TY2025 Schedule 1 row is ARCHIVED — the old join could not say so for any form without \
             a FORMS row and a second extract pair, and the only cheap discharge was a false sentence"
        );
        assert!(
            schedule_1.authority.is_none(),
            "the plant must not have smuggled in an excuse"
        );

        // PLANT (b) — genuinely NOT archived, and it must still red, by name, on the right document.
        let form_8995a = ported("f8995a", 2025);
        let problems = archive.problems_for(&form_8995a);
        assert_eq!(
            problems.len(),
            1,
            "exactly one of the two documents is missing; got {problems:?}"
        );
        assert!(
            problems[0].contains("design/forms/2025/i8995a--2025.pdf")
                && problems[0].contains("not in design/forms/MANIFEST.json"),
            "the INSTRUCTIONS are the gap and the message must say so: {problems:?}"
        );
        assert!(
            !problems[0].contains("f8995a--2025.pdf"),
            "the form's own archive IS complete and must not be blamed: {problems:?}"
        );

        // ★ CONTROL, so a `problems_for` that always returned empty could not pass (a): the same row
        //   pointed at a year the archive does not reach at all reds on BOTH documents.
        let nowhere = ported("f1040s1", 2099);
        assert_eq!(
            archive.problems_for(&nowhere).len(),
            2,
            "a year with no archive at all must red on the form AND its instructions"
        );
    }

    /// ★★ **FR-138 — the excuse SLOT may not accept an arbitrary sentence.**
    ///
    /// `authority` is the one excuse the join takes, and `btctax_forms::MapRow` spells it
    /// `"not-yet-archived: <reason>"`. A slot that accepts anything is a slot that accepts an admission
    /// nobody has to justify — and the 31 pairs this replaced carried no reason at all.
    #[test]
    fn an_excuse_must_be_spelled_as_an_excuse_with_a_reason() {
        let real = excused_form_years().expect("the excuse set derives");
        assert!(
            !real.is_empty(),
            "premise: at least one row carries an excuse today, else the plants below prove nothing"
        );
        // Every committed excuse really does read `not-yet-archived: <reason>`, through the same
        // function `excused_form_years` decides with.
        let carried: Vec<String> = map_rows()
            .expect("rows")
            .into_iter()
            .filter_map(|(_, _, r)| r.authority)
            .collect();
        for a in &carried {
            assert!(
                excuse_reason(a).is_some(),
                "every committed excuse must read `{EXCUSE_PREFIX}<reason>`: {a:?}"
            );
        }
        // PLANT: four ways a sentence is not an excuse, adjudicated by the live decider.
        for bad in [
            "archived",                    // the opposite claim
            "not-yet-archived: ",          // the prefix with no reason
            "not-yet-archived",            // no reason and not even the separator
            "  not-yet-archived: pending", // the prefix is not where it must be
        ] {
            assert!(
                excuse_reason(bad).is_none(),
                "{bad:?} must NOT satisfy the excuse spelling `excused_form_years` enforces"
            );
        }
        // CONTROL: the real spelling IS accepted, so a decider that rejected everything could not
        // pass the plants above.
        assert_eq!(
            excuse_reason("not-yet-archived: no manifest entry"),
            Some("no manifest entry")
        );
    }

    /// ★★ **THE PLANTED DEFECT THIS FILE WAS FIXED FOR: a prior-year archive is not coverage.**
    ///
    /// The ratchet used to collect `archived` as a `BTreeSet<&str>` of `f.form`, throwing
    /// [`FormAuthority::year`] away. With one registry row — `f1040s1a`, TY2025 — that made the TY2026
    /// obligation for the same form look discharged, and `cite-check` would have gone on verifying a
    /// TY2026 document against the TY2025 extract while printing coverage. The TY2026 draft of that
    /// very form keeps 10 of its 219 TY2025 AcroForm field names, so "same form, different year" is not a detail.
    ///
    /// **This test reds if the year is dropped from the key again**, in either direction: the mismatch
    /// case must be reported, and the matching case must not be — so it cannot be satisfied by a
    /// comparison that simply rejects everything.
    #[test]
    fn a_prior_year_archive_does_not_discharge_a_new_year_obligation() {
        let pair = |f: &str, y: i32| (f.to_string(), y);
        let emitted: BTreeSet<FormYear> = [pair("f1040s1a", 2026)].into_iter().collect();
        let none: BTreeSet<FormYear> = BTreeSet::new();

        // PLANT: the archive is the TY2025 revision; the obligation is the TY2026 one.
        let stale: BTreeSet<FormYear> = [pair("f1040s1a", 2025)].into_iter().collect();
        let verdict = adjudicate_coverage(&emitted, &stale, &none);
        assert_eq!(
            verdict.unaccounted,
            vec![pair("f1040s1a", 2026)],
            "a TY2025 archive discharged a TY2026 obligation — the coverage key has lost the YEAR, \
             which is how cite-check ends up verifying a document against the wrong booklet"
        );

        // CONTROL: the SAME year IS coverage. Without this half, a key that matched nothing at all
        // would pass the assertion above.
        let right: BTreeSet<FormYear> = [pair("f1040s1a", 2026)].into_iter().collect();
        assert_eq!(
            adjudicate_coverage(&emitted, &right, &none),
            CoverageVerdict::default(),
            "an archive of the SAME form-year must discharge the obligation"
        );

        // And a prior-year EXCUSE is likewise not an excuse for a later year.
        let stale_excuse: BTreeSet<FormYear> = [pair("f1040s1a", 2025)].into_iter().collect();
        let verdict = adjudicate_coverage(&emitted, &none, &stale_excuse);
        assert_eq!(verdict.unaccounted, vec![pair("f1040s1a", 2026)]);
        assert_eq!(
            verdict.phantom_excuses,
            vec![pair("f1040s1a", 2025)],
            "an excuse for a year we no longer print must be reported, not silently carried"
        );
    }

    /// ★★ **R16 — the hand-list omitted forms btctax really prints.** `EMITTED_FORMS` listed 16 form
    /// basenames; the emitting surface is 20 stems / 36 `(form, year)` pairs (18 / 37 when this was
    /// written; the Form 4868 and Form 1040-V rows landed 2026-09-06 and the five TY2017 rows left
    /// the same day, S9). `f8995a` and `f1040s1`
    /// were both absent, and `packet.rs` pushes both into the filed packet — so the ratchet passed on
    /// them by finding nothing, and no instrument checked either transcription.
    ///
    /// The fix is by construction, not by adding two strings: the surface is now READ, so the same
    /// omission cannot be made again. This test reds if the derivation stops seeing them.
    #[test]
    fn the_emitting_surface_is_derived_and_carries_the_year() {
        let emitted = emitted_form_years().expect("the emitting surface must be derivable");
        let forms: BTreeSet<&str> = emitted.iter().map(|(f, _)| f.as_str()).collect();

        for missed in ["f8995a", "f1040s1"] {
            assert!(
                forms.contains(missed),
                "{missed} is pushed by btctax-forms/src/packet.rs but is not in the derived emitting \
                 surface — the derivation has stopped seeing a form we print, which is exactly the \
                 R16 defect the hand-list had"
            );
        }

        // ★ Measured, not assumed: Form 8615 is NOT the same shape as 8995-A. btctax never emits it —
        //   there is no f8615 template and no map — because the §1(g) case is REFUSED and disclosed on
        //   Form 8275 instead (`btctax-core/src/tax/form8275.rs`, "Form 8615 not filed"). Its absence
        //   from the obligation set is correct. If btctax ever does print it, this assertion reds and
        //   the fix is to archive f8615/i8615 for that year, not to delete the line.
        assert!(
            !forms.contains("f8615"),
            "btctax now embeds a Form 8615 template — archive f8615--<year> and i8615--<year> and \
             remove this assertion"
        );

        // Aliases are translated, not skipped: the packet stems `schedule_d`/`schedule_se` are the IRS
        // basenames `f1040sd`/`f1040sse` on the authority side.
        for irs in ["f1040sd", "f1040sse"] {
            assert!(
                forms.contains(irs),
                "{irs} must reach the obligation set under its IRS name"
            );
        }
        assert!(
            !forms.iter().any(|f| f.starts_with("schedule_")),
            "a raw template stem leaked into the obligation set — it would never match an archived \
             authority filename and would look like a permanent gap"
        );

        // ★ An untranslatable stem must ERROR, never be quietly dropped.
        assert!(irs_basename("dependents_statement").is_err());
        assert_eq!(irs_basename("f8995a"), Ok("f8995a"));
        assert_eq!(irs_basename("schedule_d"), Ok("f1040sd"));
    }

    /// ★ The year dimension has to agree with the crate that decides which years btctax will fill.
    /// A template directory with no [`btctax_forms::SUPPORTED_YEARS`] entry is dead weight nothing can
    /// print; a supported year with no template directory is a year the emitter will fail on at
    /// runtime — and, worse here, a year that creates NO authority obligation at all, so the ratchet
    /// would report full coverage for a year it has never looked at.
    #[test]
    fn the_template_years_are_exactly_the_supported_years() {
        let emitted = emitted_form_years().expect("the emitting surface must be derivable");
        let template_years: BTreeSet<i32> = emitted.iter().map(|(_, y)| *y).collect();
        let supported: BTreeSet<i32> = btctax_forms::SUPPORTED_YEARS.iter().copied().collect();
        assert_eq!(
            template_years, supported,
            "the tax years with embedded templates and btctax_forms::SUPPORTED_YEARS have drifted. \
             Adding a year to SUPPORTED_YEARS without its templates leaves that year with zero \
             authority obligations, so this ratchet reports success for a year it never examined."
        );
    }

    /// ★★ **A new tax year's design corpus must not be silently unread.** The quotation pass is
    /// pointed at a hand-named pair of TY2025 documents and a hand-named pair of TY2025 extracts, so
    /// creating `design/ty2026/` adds a corpus `cite-check` does not look at — and it would keep
    /// printing "all verbatim" about the previous year while the retargeted spec drifted freely.
    ///
    /// This reds the moment a year directory exists with no document in the checked set.
    #[test]
    fn every_design_year_on_disk_has_a_document_in_the_checked_set() {
        let (docs, _) = schedule_1a_docs();
        let dirs = design_year_dirs().expect("design/ is readable");
        assert!(
            !dirs.is_empty(),
            "no design/ty<YYYY> directory found — the walk is broken, and a broken walk makes this \
             check pass by finding nothing"
        );
        assert!(
            unchecked_design_years(&docs, &dirs).is_empty(),
            "design year(s) {:?} have a corpus on disk that cite-check never reads: it still names \
             TY2025 documents and TY2025 extracts by hand in `schedule_1a_docs()`",
            unchecked_design_years(&docs, &dirs)
        );

        // ★ PLANTED: a year directory with no checked document must be REPORTED, not skipped. Without
        //   this half the assertion above is satisfied by any list, including an empty one.
        let root = repo_root();
        let planted = vec![
            (2025, root.join("design/ty2025")),
            (2026, root.join("design/ty2026")),
        ];
        assert_eq!(
            unchecked_design_years(&docs, &planted),
            vec![2026],
            "a design year with no checked document must be named"
        );
    }

    /// ★ Mutation-proofing the checker itself: a paraphrase MUST be rejected. Without this, a
    /// `normalise` that collapsed too much (or a `contains` that always matched) would let every
    /// misquote through and the test above would be decoration.
    #[test]
    fn a_paraphrase_is_rejected_and_the_real_sentence_is_accepted() {
        let extract = "Divide line 27 by $1,000. If the resulting number isn't a whole number, \
                       increase the result to the next higher whole number."
            .to_string();
        // The real sentence, wrapped and emphasised the way a design doc carries it.
        let good = "> Divide line 27 by $1,000. If the resulting number isn't a whole\n\
                    > number, **increase** the result to the next **higher** whole number.";
        assert!(unverified_quotations(good, std::slice::from_ref(&extract)).is_empty());
        // A paraphrase that means the same thing but is not the form's words.
        let paraphrase = "> Divide line 27 by $1,000 and round the result up to a whole number.";
        assert_eq!(
            unverified_quotations(paraphrase, std::slice::from_ref(&extract)).len(),
            1
        );
        // ★ The dangerous case: one word changed, in the direction that flips the rounding.
        let wrong = "> Divide line 27 by $1,000. If the resulting number isn't a whole number, \
                     decrease the result to the next lower whole number.";
        assert_eq!(
            unverified_quotations(wrong, &[extract]).len(),
            1,
            "a quotation with the rounding direction flipped must NOT verify"
        );
    }
}

/// ★★ Design r2 §10 step 1 — **the row set IS the emitting surface, both ways.** `emitted_form_years()`
/// derives `(IRS basename, year)` from the glob of bundled templates; each map's ROW carries
/// `irs_stem`. The two are joined through `irs_stem` — NOT the crate stem — because this function
/// keys on the IRS basename and `schedule_d`/`schedule_se` would otherwise red on six non-defects
/// (fold review F4). And `irs_stem` must equal `irs_basename(stem)` on every row, which is what lets
/// `STEM_ALIASES` retire into the header (design r2 §9).
#[cfg(test)]
mod map_row_tests {
    use super::*;
    use std::collections::BTreeSet;

    /// ★ ONE reader, shared with the authority ratchet (FR-138). This used to be a second walk of the
    /// same glob with its own `unwrap`s; two readers of one file format is how a checker ends up
    /// reading something the live path does not.
    fn rows() -> Vec<(String, i32, btctax_forms::MapRow)> {
        map_rows().expect("the year-package table must be derivable")
    }

    fn row_set() -> BTreeSet<FormYear> {
        rows()
            .iter()
            .map(|(_, y, r)| (r.irs_stem.clone(), *y))
            .collect()
    }

    /// THE check, shared by the green test and the plant (step-1 review P4): both directions of the
    /// symmetric difference between the rows' `(irs_stem, year)` set and the emitting surface.
    fn two_way_diff(
        from_rows: &BTreeSet<FormYear>,
        emitted: &BTreeSet<FormYear>,
    ) -> (Vec<FormYear>, Vec<FormYear>) {
        (
            from_rows.difference(emitted).cloned().collect(),
            emitted.difference(from_rows).cloned().collect(),
        )
    }

    /// ★★ **The DECLARED row set, derived from each year's `YEAR.toml` — FR-150's replacement for a
    /// hardcoded `38`.**
    ///
    /// `forms_expected` is design r2 §6's year record: *"the forms this year INTENDS to bundle —
    /// runbook step 1's output, committed"*, written by a human as the first step of a port and bound
    /// to the glob in **both** directions by `btctax-forms/tests/year_record.rs`
    /// (`YearRecord::glob_problems` reports "expected but not bundled" and "bundled but not
    /// expected"). So it is an independently-authored declaration of the same set, and the row count
    /// derives from it instead of from a literal that a new year moves.
    fn declared_set(years: &BTreeSet<i32>) -> BTreeSet<FormYear> {
        let mut out = BTreeSet::new();
        for year in years {
            let record = btctax_forms::year_record::YearRecord::for_year(*year)
                .unwrap_or_else(|| panic!("forms/{year}/YEAR.toml is bundled"));
            for stem in &record.forms_expected {
                out.insert((
                    irs_basename(stem)
                        .unwrap_or_else(|e| panic!("{year}: forms_expected has {stem:?}: {e}"))
                        .to_string(),
                    *year,
                ));
            }
        }
        out
    }

    #[test]
    fn the_row_set_equals_the_emitting_surface_both_ways_through_irs_stem() {
        let from_rows = row_set();
        let emitted = emitted_form_years().expect("the emitting surface derives");
        let (only_rows, only_emitted) = two_way_diff(&from_rows, &emitted);
        assert!(
            only_rows.is_empty() && only_emitted.is_empty(),
            "row set ≠ emitting surface — rows without a template: {only_rows:?}; templates \
             without a row: {only_emitted:?}"
        );

        // ★★ FR-150 — the count DERIVES from the year records, not from a hand-typed number. The
        //    literal this replaced was `38`, and its own failure message explained why the number
        //    would move ("a new year adds files, not a list") and then pinned it anyway. A third
        //    independently-authored view of the same set is what makes the equality mean something:
        //    templates on disk (`emitted`), map rows on disk (`from_rows`), and what each year
        //    DECLARES it intends to bundle (`YEAR.toml`).
        let declared = declared_set(&emitted.iter().map(|(_, y)| *y).collect());
        let (only_declared, only_rows) = two_way_diff(&declared, &from_rows);
        assert!(
            only_declared.is_empty() && only_rows.is_empty(),
            "the year records and the map rows disagree — declared in YEAR.toml with no map row: \
             {only_declared:?}; map row no YEAR.toml declares: {only_rows:?}. `forms_expected` is \
             runbook step 1's committed output; if a form is genuinely gone from a year, that \
             declaration is where it leaves."
        );
        // ★ Guard the guard, STRUCTURALLY rather than with a floor: a `declared` that came back empty
        //   cannot pass, because `from_rows` cannot be empty — `map_rows()` refuses a walk that finds
        //   no rows or no f1040 — so the difference would name every row. That is why this test needs
        //   no count at all, which was the whole of FR-150.
    }

    /// ★★★ **FR-150's KILL — what the hand-typed `38` caught, the derivation must still catch.**
    ///
    /// The literal it replaced was `assert_eq!(from_rows.len(), 38, "38 rows on disk today … a new year
    /// adds files, not a list")` — a failure message that explains why the number will move and then
    /// pins it anyway. Deriving the expectation is only an improvement if it still reds on the one case
    /// the count could see and the symmetric row/template diff could not: **a template and its map
    /// deleted together**, which leaves both derived sets smaller and their difference empty.
    ///
    /// ★ The plant is on the derived sets, one pair at a time, the same way
    /// `a_row_pointing_at_no_template_is_caught_in_both_directions` plants — because the *inputs* are
    /// files this test may not write, and a planted input is the only honest alternative to a planted
    /// file. Both directions, so a comparison that rejected everything could not satisfy it.
    #[test]
    fn a_silently_deleted_form_and_an_undeclared_one_are_both_caught() {
        let emitted = emitted_form_years().expect("the emitting surface derives");
        let declared = declared_set(&emitted.iter().map(|(_, y)| *y).collect());
        let rows = row_set();

        // CONTROL: today's real sets agree. Without this the plants below prove nothing.
        let (only_declared, only_rows) = two_way_diff(&declared, &rows);
        assert!(only_declared.is_empty() && only_rows.is_empty());

        // PLANT (a) — THE CASE THE LITERAL `38` EXISTED FOR. A form's template and its map vanish
        // together; `row_set` and `emitted` shrink in step, so their diff stays empty. The year record
        // still declares it, and the derived expectation names it.
        let victim = ("f6251".to_string(), 2025);
        assert!(rows.contains(&victim), "premise: {victim:?} has a map row");
        let mut deleted = rows.clone();
        deleted.remove(&victim);
        let (only_declared, only_rows) = two_way_diff(&declared, &deleted);
        assert_eq!(
            only_declared,
            vec![victim.clone()],
            "a form deleted from the tree while YEAR.toml still declares it must be NAMED — this is \
             exactly what the hardcoded count caught, and it must survive the derivation"
        );
        assert!(only_rows.is_empty());

        // PLANT (b) — the other direction: a map row nothing declares. A porter who copies a template
        // and writes a map but never touches runbook step 1's `YEAR.toml`.
        let mut undeclared = rows.clone();
        undeclared.insert(("f8615".to_string(), 2025));
        let (only_declared, only_rows) = two_way_diff(&declared, &undeclared);
        assert!(only_declared.is_empty());
        assert_eq!(
            only_rows,
            vec![("f8615".to_string(), 2025)],
            "a bundled map no year record declares must be NAMED"
        );
    }

    #[test]
    fn every_rows_irs_stem_is_what_irs_basename_would_derive() {
        for (stem, year, row) in rows() {
            assert_eq!(
                row.irs_stem,
                irs_basename(&stem).unwrap(),
                "{year}/{stem}: the header's irs_stem and STEM_ALIASES disagree"
            );
        }
    }

    /// B1 — the SAME `two_way_diff` the green test runs, observed red in each direction on a plant.
    #[test]
    fn a_row_pointing_at_no_template_is_caught_in_both_directions() {
        let emitted = emitted_form_years().unwrap();
        let mut planted_rows = row_set();
        planted_rows.insert(("f9999".to_string(), 2024));
        let (only_rows, only_emitted) = two_way_diff(&planted_rows, &emitted);
        assert_eq!(only_rows, vec![("f9999".to_string(), 2024)]);
        assert!(only_emitted.is_empty());
        let mut planted_emitted = emitted.clone();
        planted_emitted.insert(("f1040s1".to_string(), 2025));
        let (only_rows, only_emitted) = two_way_diff(&row_set(), &planted_emitted);
        assert!(only_rows.is_empty());
        assert_eq!(only_emitted, vec![("f1040s1".to_string(), 2025)]);
    }

    /// P6 — the row's `instructions` names a document the archive HOLDS: for every row of an archived
    /// year, `design/forms/<year>/<instructions>--<year>.pdf` is a manifest entry. (Rows carrying
    /// `authority = "not-yet-archived"` are skipped: their template is not in the manifest either.
    /// Until S9 dropped it on 2026-09-06 that was TY2017's five rows plus `2024/f8283`; it is now
    /// `2024/f8283` alone.)
    #[test]
    fn every_archived_rows_instructions_stem_is_a_manifest_entry() {
        let manifest = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(Path::parent)
                .unwrap()
                .join("design/forms/MANIFEST.json"),
        )
        .unwrap();
        let mut checked = 0;
        for (stem, year, row) in rows() {
            if row.authority.is_some() {
                continue;
            }
            let path = format!("design/forms/{year}/{}--{year}.pdf", row.instructions);
            assert!(
                manifest.contains(&format!("\"path\": \"{path}\"")),
                "{year}/{stem}: instructions = {:?} but {path} is not in MANIFEST.json",
                row.instructions
            );
            checked += 1;
        }
        assert!(checked >= 30, "only {checked} rows checked");
    }

    /// P6 — until `FORMS` retires into the rows, the one row it duplicates must agree with it.
    #[test]
    fn the_forms_const_row_agrees_with_its_map_row() {
        for fa in FORMS {
            let (_, _, row) = rows()
                .into_iter()
                .find(|(stem, year, _)| stem == fa.form && *year == fa.year)
                .unwrap_or_else(|| {
                    panic!("FORMS names {}/{} but no such row exists", fa.year, fa.form)
                });
            assert_eq!(row.instructions, fa.instructions, "{}/{}", fa.year, fa.form);
            assert_eq!(
                row.instr_pages.map(|p| (p[0], p[1])),
                fa.instr_pages,
                "{}/{}",
                fa.year,
                fa.form
            );
        }
    }
}
