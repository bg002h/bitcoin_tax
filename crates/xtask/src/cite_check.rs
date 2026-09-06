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

use std::collections::BTreeSet;
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
    let (archived, broken) = archived_form_years();
    if !broken.is_empty() {
        return Err(format!(
            "{} registry row(s) claim an extracted authority that is NOT on disk — coverage they \
             cannot back:\n  {}",
            broken.len(),
            broken.join("\n  ")
        ));
    }
    let excused = excused_form_years();
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

/// ★ Registry state as of 2026-07-29. Deliberately honest: only Schedule 1-A is fully wired, and the
/// `authority_coverage_may_only_improve` test below makes that visible instead of implicit.
pub const FORMS: &[FormAuthority] = &[FormAuthority {
    form: "f1040s1a",
    year: 2025,
    instructions: "i1040gi",
    instr_pages: Some((101, 110)),
    extract_stem: "schedule_1a_2025",
}];

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
/// `EMITTED_FORMS` it replaces listed **16** form basenames. The real surface is **18 stems / 37
/// (form, year) pairs**. The hand-list omitted `f1040s1` and `f8995a` outright — both of which
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

/// ★★ **THE RATCHET'S EXCUSE LIST, keyed on `(form, YEAR)`.** Every pair here is a form-year btctax can
/// print while holding no archived, extracted primary source, so its transcription is unverifiable by
/// `cite-check` and by the derive-the-decision-from-the-line tests. **The list may only SHRINK.**
///
/// ★ There is deliberately **no wildcard and no "all supported years" sentinel.** A sentinel would
/// re-open the exact hole this file just closed: adding TY2026 templates would be silently pre-excused
/// instead of reddening the ratchet. Every year is typed out, so a new tax year is a conscious edit.
///
/// ★ Years are the ones with a template on disk, not a range — e.g. `f8275` and `f8995a` are TY2024
/// only, `f1040s1a` is TY2025 only (and is the one pair that IS archived, so it does not appear here).
pub const AUTHORITY_NOT_YET_ARCHIVED: &[(&str, &[i32])] = &[
    ("f1040", &[2017, 2024, 2025]),
    ("f1040s1", &[2024]),
    ("f1040s2", &[2024, 2025]),
    ("f1040s3", &[2024, 2025]),
    ("f1040sa", &[2024, 2025]),
    ("f1040sb", &[2024, 2025]),
    ("f1040sc", &[2024, 2025]),
    ("f1040sd", &[2017, 2024, 2025]),
    ("f1040sse", &[2017, 2024, 2025]),
    ("f6251", &[2024, 2025]),
    ("f8275", &[2024]),
    ("f8283", &[2017, 2024, 2025]),
    ("f8949", &[2017, 2024, 2025]),
    ("f8959", &[2024, 2025]),
    ("f8960", &[2024, 2025]),
    ("f8995", &[2024, 2025]),
    ("f8995a", &[2024]),
];

/// The excuse list as `(form, year)` pairs.
pub fn excused_form_years() -> BTreeSet<FormYear> {
    AUTHORITY_NOT_YET_ARCHIVED
        .iter()
        .flat_map(|(form, years)| years.iter().map(move |y| ((*form).to_string(), *y)))
        .collect()
}

/// The `(form, year)` pairs [`FORMS`] actually holds an extracted authority for, and the rows whose
/// claim does not survive contact with the disk.
///
/// ★ **Skipping is not passing.** A registry row is coverage only if the committed extract it names is
/// really there; a row pointing at a fixture that has been renamed or deleted is returned in the second
/// element so it can FAIL by name, never quietly stop counting.
pub fn archived_form_years() -> (BTreeSet<FormYear>, Vec<String>) {
    let root = repo_root();
    let fixture = |stem: &str, suffix: &str| {
        root.join(format!(
            "crates/btctax-core/src/tax/fixtures/{stem}_{suffix}.txt"
        ))
    };
    let mut ok = BTreeSet::new();
    let mut broken = Vec::new();
    for f in FORMS {
        if f.extract_stem.is_empty() {
            continue;
        }
        let mut missing: Vec<String> = Vec::new();
        let form_txt = fixture(f.extract_stem, "form");
        if !form_txt.exists() {
            missing.push(form_txt.display().to_string());
        }
        if !f.instructions.is_empty() {
            let instr_txt = fixture(f.extract_stem, "instructions");
            if !instr_txt.exists() {
                missing.push(instr_txt.display().to_string());
            }
        }
        if missing.is_empty() {
            ok.insert((f.form.to_string(), f.year));
        } else {
            broken.push(format!("{}--{}: {}", f.form, f.year, missing.join(", ")));
        }
    }
    (ok, broken)
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
    /// has an archived, extracted primary source in [`FORMS`] or is explicitly listed in
    /// [`AUTHORITY_NOT_YET_ARCHIVED`]. The list may only shrink.
    ///
    /// Without this, adding a form emitter — or a new tax year's templates — is a silent regression:
    /// nothing compels anyone to archive the PDF that defines it, so the transcription becomes
    /// unverifiable and every downstream conformance test (the `cite-check` quotations, the
    /// derive-the-direction-from-the-line assertions, the label census) has nothing to check against
    /// and passes by finding nothing.
    ///
    /// ★ Both sides are DERIVED: the obligation set is read off the template directories, and the
    /// archived set off the registry plus the fixtures on disk. Nothing here is a hand-list except the
    /// excuses, and an excuse is an admission that is supposed to be typed by a human.
    #[test]
    fn authority_coverage_may_only_improve() {
        let emitted = emitted_form_years().expect("the emitting surface must be derivable");
        let (archived, broken) = archived_form_years();
        assert!(
            broken.is_empty(),
            "{} registry row(s) claim an extracted authority that is NOT on disk, so they are \
             counted as coverage they cannot back:\n  {}",
            broken.len(),
            broken.join("\n  ")
        );
        let excused = excused_form_years();
        let verdict = adjudicate_coverage(&emitted, &archived, &excused);

        // 1. Nothing may be BOTH archived and excused — a stale excuse is how a closed gap silently
        //    reopens for the next form.
        assert!(
            verdict.stale_excuses.is_empty(),
            "[{}] now HAS an archived authority for that YEAR — remove the year from \
             AUTHORITY_NOT_YET_ARCHIVED so the ratchet actually tightens",
            render(&verdict.stale_excuses)
        );

        // 2. Every emitted (form, year) is accounted for: archived, or consciously excused.
        assert!(
            verdict.unaccounted.is_empty(),
            "btctax can print [{}] with no archived primary source for THAT YEAR and no explicit \
             excuse. Every form follows one pattern and every one has an identically-numbered IRS \
             instructions document — archive the pair and extract it (`xtask extract-schedule-1a` is \
             the model), or add the year to AUTHORITY_NOT_YET_ARCHIVED with intent. Silence here \
             means a form-year whose transcription NOTHING can check.",
            render(&verdict.unaccounted)
        );

        // 3. Every excuse names a form-year we can actually print — otherwise the list rots into a
        //    wishlist, and a year that has been retired keeps pretending to be an open admission.
        assert!(
            verdict.phantom_excuses.is_empty(),
            "[{}] are excused but have no template on disk for that year",
            render(&verdict.phantom_excuses)
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
    /// basenames; the emitting surface is 18 stems / 37 `(form, year)` pairs. `f8995a` and `f1040s1`
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

    fn rows() -> Vec<(String, i32, btctax_forms::MapRow)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("btctax-forms/forms");
        let mut out = Vec::new();
        for y in std::fs::read_dir(&root).unwrap().flatten() {
            if !y.path().is_dir() {
                continue;
            }
            let year: i32 = y.file_name().to_string_lossy().parse().unwrap();
            for m in std::fs::read_dir(y.path()).unwrap().flatten() {
                let p = m.path();
                let name = p.file_name().unwrap().to_string_lossy().to_string();
                let Some(stem) = name.strip_suffix(".map.toml") else {
                    continue;
                };
                let row = btctax_forms::MapRow::read(&std::fs::read_to_string(&p).unwrap())
                    .unwrap_or_else(|e| panic!("{year}/{stem}: {e}"));
                out.push((stem.to_string(), year, row));
            }
        }
        out
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
        assert_eq!(
            from_rows.len(),
            37,
            "37 rows on disk today; a new year adds files, not a list"
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
    /// year, `design/forms/<year>/<instructions>--<year>.pdf` is a manifest entry. (TY2017 has no
    /// archive; those five rows are the `authority = "not-yet-archived"` rows and are skipped here
    /// for the same reason.)
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
