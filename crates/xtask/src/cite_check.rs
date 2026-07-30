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

use std::fs;
use std::path::{Path, PathBuf};

/// Repo root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
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
fn normalise(s: &str) -> String {
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
        for pair in quotes.chunks_exact(2) {
            let span = &line[pair[0] + 1..pair[1]];
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

    // ★ Report AUTHORITY COVERAGE every run, not only in a test. btctax emits ~16 forms and had the
    // defining PDF archived for 4 — a gap that was completely invisible because nothing ever counted it.
    // Every form follows the same pattern and every one has an identically-numbered IRS instructions
    // document, so an unarchived form is a form whose transcription NOTHING can check.
    let archived: Vec<&str> = FORMS
        .iter()
        .filter(|f| !f.extract_stem.is_empty())
        .map(|f| f.form)
        .collect();
    println!(
        "cite-check: authority archived + extracted for {}/{} emitted forms ({}); {} awaiting archive",
        archived.len(),
        EMITTED_FORMS.len(),
        archived.join(", "),
        AUTHORITY_NOT_YET_ARCHIVED.len()
    );
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
            format!("design/amt-form6251/{}--{}.pdf", f.form, f.year),
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
            format!("design/amt-form6251/{}--{}.pdf", f.instructions, f.year),
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

/// Every form btctax **emits**, by IRS basename — derived from `btctax-forms`' modules.
///
/// ★ This is the left-hand side of the coverage question the registry answers: we emit these, so we owe
/// an archived authority for each.
pub const EMITTED_FORMS: &[&str] = &[
    "f1040", "f1040s1a", "f1040sa", "f1040sb", "f1040sc", "f1040sd", "f1040sse", "f1040s2",
    "f1040s3", "f6251", "f8949", "f8275", "f8283", "f8959", "f8960", "f8995",
];

/// ★★ **THE RATCHET.** btctax emits 16 forms and has the authority archived for a handful. That gap is
/// real and is not closed by this commit — but it must never GROW, and it must never be invisible.
///
/// Every form listed here is one we emit while holding no archived, extracted primary source, so its
/// transcription is unverifiable by `cite-check` and by the derive-the-decision-from-the-line tests. The
/// list may only SHRINK. Adding a form to `EMITTED_FORMS` without either archiving its authority or
/// consciously extending this list is a compile-free, silent regression — so the test below makes it a
/// test failure instead.
pub const AUTHORITY_NOT_YET_ARCHIVED: &[&str] = &[
    "f1040", "f1040sa", "f1040sb", "f1040sc", "f1040sd", "f1040sse", "f1040s2", "f1040s3", "f6251",
    "f8949", "f8275", "f8283", "f8959", "f8960", "f8995",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

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

    /// ★★ **THE AUTHORITY RATCHET.** Every form btctax emits either has an archived, extracted primary
    /// source in [`FORMS`] or is explicitly listed in [`AUTHORITY_NOT_YET_ARCHIVED`]. The list may only
    /// shrink.
    ///
    /// Without this, adding a form emitter is a silent regression: nothing compels anyone to archive the
    /// PDF that defines it, so the transcription becomes unverifiable and every downstream conformance
    /// test — the `cite-check` quotations, the derive-the-direction-from-the-line assertions, the label
    /// census — simply has nothing to check against and passes by finding nothing.
    #[test]
    fn authority_coverage_may_only_improve() {
        let archived: BTreeSet<&str> = FORMS
            .iter()
            .filter(|f| !f.extract_stem.is_empty())
            .map(|f| f.form)
            .collect();
        let excused: BTreeSet<&str> = AUTHORITY_NOT_YET_ARCHIVED.iter().copied().collect();

        // 1. Nothing may be BOTH archived and excused — that is a stale excuse, and a stale excuse list
        //    is how a closed gap silently reopens for the next form.
        let both: Vec<&&str> = archived.intersection(&excused).collect();
        assert!(
            both.is_empty(),
            "{both:?} now HAS an archived authority — remove it from AUTHORITY_NOT_YET_ARCHIVED so the \
             ratchet actually tightens"
        );

        // 2. Every emitted form is accounted for: archived, or consciously excused.
        let unaccounted: Vec<&&str> = EMITTED_FORMS
            .iter()
            .filter(|f| !archived.contains(**f) && !excused.contains(**f))
            .collect();
        assert!(
            unaccounted.is_empty(),
            "btctax emits {unaccounted:?} with no archived primary source and no explicit excuse. Every \
             form we support follows one pattern and every one has an identically-numbered IRS \
             instructions document — archive the pair and extract it (`xtask extract-schedule-1a` is the \
             model), or add it to AUTHORITY_NOT_YET_ARCHIVED with intent. Silence here means a form whose \
             transcription NOTHING can check."
        );

        // 3. Every excuse names a form we actually emit — otherwise the list rots into a wishlist.
        let phantom: Vec<&&str> = AUTHORITY_NOT_YET_ARCHIVED
            .iter()
            .filter(|f| !EMITTED_FORMS.contains(f))
            .collect();
        assert!(
            phantom.is_empty(),
            "{phantom:?} are excused but not emitted"
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
