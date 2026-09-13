//! ★★★ **`xtask forms extract` — THE SUBCOMMAND 58 COMMITTED EXTRACTS ALREADY NAME.**
//!
//! **The gap this closes (FR-140 / port-rehearsal F8).** Measured over `design/forms/extract/`: **126**
//! text layers, **74** carrying a `# Regenerate:` line, **58** of those naming
//! `cargo run -p xtask -- forms extract` — a subcommand that did not exist. The remaining **52** carried
//! no header at all, so the one decision that matters about a text layer — whether it was extracted with
//! `-layout` — was unrecorded for them. The rehearsal had to recover it by trial for `f8995a--2025`.
//!
//! A text layer is not a convenience: it is *what every conformance check in this repo reads instead of
//! the PDF*, because the PDFs are gitignored and the suite runs offline. So "how was this file made"
//! is a provenance question about the authority itself, and the answer belongs in the file.
//!
//! ## The recipe, as the header records it
//!
//! ```text
//! # GENERATED — do not hand-edit. Text layer of design/forms/2024/f1040--2024.pdf
//! # sha256:0a7a54354283044c…  |  pdftotext -layout  |  form feeds -> LF
//! # Regenerate: cargo run -p xtask -- forms extract f1040--2024
//! #
//! ```
//!
//! Four fields, every one load-bearing:
//!
//! | field | why it is in the file |
//! |---|---|
//! | the source path | which document this is the text of — a bundled template and an archived PDF are different objects (`f8283--2024`) |
//! | `sha256:<16>…` | **which revision**. If the PDF on disk hashes differently the run REFUSES: the IRS revising a document in place is exactly what the notes warn about, and regenerating over it would launder a new authority into the tree |
//! | the flags | `-layout` or none. The instructions are two-column and `-layout` interleaves them; the forms need it to keep line labels beside their boxes. **Measured: 47 `-layout`, 27 none** |
//! | `form feeds -> LF` | ★ the field this had to ADD. Nothing recorded it, and it is not cosmetic |
//!
//! ★★ **The fourth field is a finding, not a design flourish.** `pdftotext` separates pages with `\f`.
//! Of the 126 committed extracts, **72 keep the form feeds and 54 have them replaced by `\n`** — and the
//! split does not follow the recorded flags, the tree, or the year. It cuts straight through the cohort
//! that named `forms extract`: of those 58, **54 are form-feed-stripped and 4 are not**. So the recorded
//! command was not merely missing, it was *insufficient*: running it as written would have rewritten
//! 54 authorities on their page boundaries. An absent field defaults to *keep them* (the null
//! transformation, which is what raw `pdftotext` prints), so a header that does not mention page breaks
//! still means exactly one thing.
//!
//! ## What it refuses, and why each refusal is not a nuisance
//!
//! * **No header** → `--adopt` first. Guessing the flags is how a wrong recipe becomes a committed fact.
//! * **PDF absent** → `forms fetch --restore`. Every gitignored authority is note-backed and restorable.
//! * **PDF hash ≠ the header's** → the document was revised. Archive the new revision *in addition*
//!   (`design/forms/README.md`); do not let a regeneration be how it replaces the old one.
//! * **`--check` and the output differs** → non-zero, with the byte counts. This is the mode a gate
//!   would use.
//!
//! ## `--adopt`: how a headerless extract earns a header
//!
//! It does **not** write down what someone believes. It tries each candidate recipe, requires **exactly
//! one to reproduce the committed body byte-for-byte**, and records that one. If none does, it refuses —
//! a text layer that `pdftotext` cannot reproduce from the archived PDF is a provenance claim nobody can
//! make, and the file needs a human, not a header.
//!
//! ★ `--adopt` deliberately never changes a body. Adopting a *revised* document is a different act with
//! a different review (the body changes, so every quotation drawn from it must be re-verified), and a
//! flag that did both would make the two indistinguishable in a diff.
//!
//! ## Offline
//!
//! `pdftotext` and the PDFs are operator-side. Every test here drives the pure functions
//! ([`parse_header`], [`compose`], [`regenerate`], [`discover`]) with in-process strings, so the suite
//! needs neither poppler nor a single archived byte — the same seam
//! `authority_refresh::compare` uses for the network.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// How the extract treats `pdftotext`'s page separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageBreaks {
    /// Exactly what `pdftotext` printed: `\f` between pages. The absence of the header field.
    FormFeed,
    /// Form feeds replaced by `\n`. Recorded as `form feeds -> LF`.
    Lf,
}

impl PageBreaks {
    /// The header spelling, or `""` for the null transformation.
    #[must_use]
    pub fn spelling(self) -> &'static str {
        match self {
            PageBreaks::FormFeed => "",
            PageBreaks::Lf => "  |  form feeds -> LF",
        }
    }

    fn apply(self, raw: &str) -> String {
        match self {
            PageBreaks::FormFeed => raw.to_string(),
            PageBreaks::Lf => raw.replace('\u{c}', "\n"),
        }
    }
}

/// The marker that spells out [`PageBreaks::Lf`] in a header.
pub const LF_MARKER: &str = "form feeds -> LF";

/// Everything the header records about how one text layer was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    /// Repo-relative path of the document this is the text of.
    pub source: String,
    /// The first 16 hex characters of the source's sha256, as the header prints them.
    pub sha16: String,
    /// `["-layout"]` or `[]`.
    pub flags: Vec<String>,
    pub page_breaks: PageBreaks,
    /// The header lines verbatim, so a regeneration reproduces them exactly.
    pub header: Vec<String>,
}

/// ★ The ONE spelling of a flag set, used to write a header, to print one, and to explain a refusal —
/// so a header this tool writes is always one it can read back.
#[must_use]
pub fn flag_spelling(flags: &[String]) -> String {
    if flags.is_empty() {
        "pdftotext (no flags)".to_string()
    } else {
        format!("pdftotext {}", flags.join(" "))
    }
}

/// Split a committed extract into its `#`-comment header and its body.
fn split(text: &str) -> (Vec<String>, String) {
    let lines: Vec<&str> = text.split('\n').collect();
    let n = lines.iter().take_while(|l| l.starts_with('#')).count();
    let header = lines[..n].iter().map(|s| (*s).to_string()).collect();
    (header, lines[n..].join("\n"))
}

/// ★★ Read a committed extract's own recipe. **Refuses rather than guessing** at any missing field.
pub fn parse_header(stem: &str, text: &str) -> Result<(Recipe, String), String> {
    let (header, body) = split(text);
    if header.is_empty() {
        return Err(format!(
            "{stem}.txt records nothing about how it was made — no `# GENERATED` header. REFUSING to \
             guess: the flags decide whether two-column instructions come out interleaved, and a \
             wrong guess would silently rewrite an archived authority.\n\
             ★ `cargo run -p xtask -- forms extract --adopt {stem}` derives the recipe by \
             reproducing this exact body from the archived PDF, and records only a recipe that works."
        ));
    }
    let source = header
        .iter()
        .find_map(|l| l.split_once("Text layer of "))
        .map(|(_, p)| p.trim().to_string())
        .ok_or_else(|| {
            format!("{stem}.txt has a header but it names no source document (`Text layer of …`)")
        })?;
    let recipe_line = header
        .iter()
        .find(|l| l.starts_with("# sha256:"))
        .ok_or_else(|| {
            format!("{stem}.txt records no `# sha256:<16>…  |  <command>` recipe line")
        })?;
    let sha16 = recipe_line
        .trim_start_matches("# sha256:")
        .split('…')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if sha16.len() != 16 || !sha16.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "{stem}.txt's recipe line does not record a 16-hex-digit digest prefix: {recipe_line:?}"
        ));
    }
    let after = recipe_line
        .split_once('|')
        .map(|(_, r)| r)
        .ok_or_else(|| format!("{stem}.txt's recipe line records no command: {recipe_line:?}"))?;
    let command = after.split('|').next().unwrap_or_default().trim();
    let flags: Vec<String> = match command {
        "pdftotext (no flags)" => Vec::new(),
        c => {
            let rest = c.strip_prefix("pdftotext ").ok_or_else(|| {
                format!("{stem}.txt's recipe names a command this tool does not run: {c:?}")
            })?;
            rest.split_whitespace().map(str::to_string).collect()
        }
    };
    let page_breaks = if recipe_line.contains(LF_MARKER) {
        PageBreaks::Lf
    } else {
        PageBreaks::FormFeed
    };
    Ok((
        Recipe {
            source,
            sha16,
            flags,
            page_breaks,
            header,
        },
        body,
    ))
}

/// The file the recipe says should be on disk, given what `pdftotext` printed.
#[must_use]
pub fn compose(recipe: &Recipe, raw: &str) -> String {
    let mut s = recipe.header.join("\n");
    s.push('\n');
    s.push_str(&recipe.page_breaks.apply(raw));
    s
}

/// What one regeneration found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The committed file is exactly what its own recipe reproduces.
    Unchanged,
    /// It is not — the regenerated text, for the caller to write or to refuse.
    Changed(String),
}

/// ★★★ **THE PURE REGENERATION.** `digest` is the sha256 of the source PDF, `raw` is what `pdftotext`
/// printed with the recorded flags. No filesystem, no poppler, no network — which is what lets the
/// planted-defect tests exist at all.
pub fn regenerate(
    stem: &str,
    committed: &str,
    digest: &str,
    raw: &str,
) -> Result<(Recipe, Verdict), String> {
    let (recipe, _body) = parse_header(stem, committed)?;
    if !digest.starts_with(&recipe.sha16) {
        return Err(format!(
            "{stem}: {} hashes to {}… but this text layer was made from {}….\n\
             ★ A different hash is not a corrupt download — it is the IRS REVISING the document. \
             REFUSING to regenerate: that would replace an audited authority with one nobody has \
             reviewed, silently, underneath every check that reads this text.\n\
             Archive the new revision IN ADDITION (design/forms/README.md), or \
             `forms fetch --restore` to put the archived one back.",
            recipe.source,
            &digest[..16.min(digest.len())],
            recipe.sha16
        ));
    }
    let produced = compose(&recipe, raw);
    let verdict = if produced == committed {
        Verdict::Unchanged
    } else {
        Verdict::Changed(produced)
    };
    Ok((recipe, verdict))
}

/// The candidate recipes `--adopt` tries, in preference order: the null page-break transformation
/// first, so a document with no page breaks at all gets the simpler header.
#[must_use]
pub fn candidates() -> Vec<(Vec<String>, PageBreaks)> {
    vec![
        (vec!["-layout".to_string()], PageBreaks::FormFeed),
        (vec!["-layout".to_string()], PageBreaks::Lf),
        (Vec::new(), PageBreaks::FormFeed),
        (Vec::new(), PageBreaks::Lf),
    ]
}

/// ★★★ **Derive a headerless extract's recipe BY REPRODUCTION.**
///
/// `raw_for` answers with what `pdftotext` printed for a given flag set. The first candidate whose
/// output equals the committed body wins; if none does, this **refuses**, because a text layer the
/// archived PDF cannot reproduce is a provenance claim that would be false the moment it was written.
pub fn discover(
    stem: &str,
    body: &str,
    raw_for: &dyn Fn(&[String]) -> Result<String, String>,
) -> Result<(Vec<String>, PageBreaks), String> {
    let mut tried = Vec::new();
    let mut raws: Vec<(Vec<String>, String)> = Vec::new();
    for (flags, _) in candidates() {
        if raws.iter().any(|(f, _)| *f == flags) {
            continue;
        }
        raws.push((flags.clone(), raw_for(&flags)?));
    }
    for (flags, pb) in candidates() {
        let raw = &raws
            .iter()
            .find(|(f, _)| *f == flags)
            .expect("every candidate's flags were run")
            .1;
        if pb.apply(raw) == *body {
            return Ok((flags, pb));
        }
        tried.push(format!(
            "{}{}: {} bytes",
            flag_spelling(&flags),
            match pb {
                PageBreaks::FormFeed => "",
                PageBreaks::Lf => " + form feeds -> LF",
            },
            pb.apply(raw).len()
        ));
    }
    Err(format!(
        "{stem}: no recipe reproduces the committed body ({} bytes). Tried:\n  {}\n\
         ★ REFUSING to write a header. A text layer `pdftotext` cannot reproduce from the archived \
         document is not a text layer of that document, and a header saying otherwise would be a \
         false provenance claim — the exact thing this header exists to prevent.",
        body.len(),
        tried.join("\n  ")
    ))
}

/// The stemless spelling **58 committed extracts carried**. It names the right tool and nothing else:
/// pasted verbatim it prints a usage message, because the tool cannot know which of the 126 text layers
/// the operator meant. That is better than the "no such subcommand" it used to print and still not an
/// answer, so `--adopt` completes it with the stem.
pub const STEMLESS_REGENERATE: &str = "# Regenerate: cargo run -p xtask -- forms extract";

/// ★ **The one narrow normalisation `--adopt` performs on a `# Regenerate:` line, and its boundary.**
///
/// Only the exactly-stemless `forms extract` spelling gains a stem. Every other value is left
/// **verbatim**, because it records a different and equally real route: 15 drafts name
/// `scripts/archive_drafts.py --relayout 2026` (the archiver that *fetched* them as well as extracting
/// them) and `f8283--2024` names a literal `pdftotext` invocation against a bundled template. Those are
/// provenance. Overwriting them with a command that merely also works would delete history.
#[must_use]
pub fn normalised_regenerate_line(line: &str, stem: &str) -> String {
    if line.trim_end() == STEMLESS_REGENERATE {
        format!("{STEMLESS_REGENERATE} {stem}")
    } else {
        line.to_string()
    }
}

/// The header `--adopt` writes once a recipe has been proven.
#[must_use]
pub fn adopted_header(
    stem: &str,
    source: &str,
    digest: &str,
    flags: &[String],
    page_breaks: PageBreaks,
) -> Vec<String> {
    let cmd = flag_spelling(flags);
    vec![
        format!("# GENERATED — do not hand-edit. Text layer of {source}"),
        format!(
            "# sha256:{}…  |  {cmd}{}",
            &digest[..16],
            page_breaks.spelling()
        ),
        format!("# Regenerate: cargo run -p xtask -- forms extract {stem}"),
        "#".to_string(),
    ]
}

// ─────────────────────────────── the operator-facing command ───────────────────────────────

fn extract_dir(root: &Path) -> PathBuf {
    root.join("design/forms/extract")
}

/// Every committed text layer, DERIVED from the tree rather than listed anywhere.
fn all_stems(root: &Path) -> Result<Vec<String>, String> {
    let dir = extract_dir(root);
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(stem) = name.strip_suffix(".txt") {
            out.push(stem.to_string());
        }
    }
    out.sort();
    Ok(out)
}

/// Run `pdftotext <flags> <pdf> -` and hand back stdout.
fn pdftotext(pdf: &Path, flags: &[String]) -> Result<String, String> {
    let out = std::process::Command::new("pdftotext")
        .args(flags)
        .arg(pdf)
        .arg("-")
        .output()
        .map_err(|e| format!("pdftotext did not run (is poppler-utils installed?): {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "pdftotext exited {:?} on {}: {}",
            out.status.code(),
            pdf.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("{} is not UTF-8: {e}", pdf.display()))
}

/// Where a headerless extract's source document is, found by glob and refused if ambiguous.
fn locate_source(root: &Path, stem: &str) -> Result<String, String> {
    let mut hits = Vec::new();
    for tree in ["design/forms", "crates/btctax-forms/forms"] {
        let base = root.join(tree);
        let Ok(years) = std::fs::read_dir(&base) else {
            continue;
        };
        for y in years.flatten() {
            if !y.path().is_dir() {
                continue;
            }
            let cand = y.path().join(format!("{stem}.pdf"));
            if cand.exists() {
                hits.push(
                    cand.strip_prefix(root)
                        .unwrap_or(&cand)
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    match hits.len() {
        1 => Ok(hits.remove(0)),
        0 => Err(format!(
            "{stem}: no archived document found (looked for <stem>.pdf under design/forms/<year>/ \
             and crates/btctax-forms/forms/<year>/). If it is gitignored, \
             `cargo run -p xtask -- forms fetch --restore --stem {stem}` puts it back."
        )),
        _ => Err(format!(
            "{stem}: {} documents share this name — {}. REFUSING to pick one; a text layer must say \
             which document it is the text OF.",
            hits.len(),
            hits.join(", ")
        )),
    }
}

fn sha256_of(p: &Path) -> Result<String, String> {
    let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// `xtask forms extract <stem> | --all [--check] | --adopt <stem>`.
pub fn run(args: &[String]) -> Result<(), String> {
    let root = crate::form_geometry::repo_root();
    let check_only = args.iter().any(|a| a == "--check");

    if let Some(stem) = args
        .iter()
        .position(|a| a == "--adopt")
        .and_then(|i| args.get(i + 1))
    {
        return adopt(&root, stem);
    }
    if args.iter().any(|a| a == "--adopt") {
        return Err("--adopt needs a stem: forms extract --adopt f8995a--2025".to_string());
    }

    let stems: Vec<String> = if args.iter().any(|a| a == "--all") {
        all_stems(&root)?
    } else {
        match args.iter().find(|a| !a.starts_with('-')) {
            Some(s) => vec![s.trim_end_matches(".txt").to_string()],
            None => {
                return Err(
                    "usage: cargo run -p xtask -- forms extract <stem> | --all [--check] | \
                     --adopt <stem>"
                        .to_string(),
                )
            }
        }
    };

    let (mut unchanged, mut written) = (0usize, 0usize);
    let mut problems: Vec<String> = Vec::new();
    for stem in &stems {
        let path = extract_dir(&root).join(format!("{stem}.txt"));
        let committed = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let recipe = match parse_header(stem, &committed) {
            Ok((r, _)) => r,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        let pdf = root.join(&recipe.source);
        if !pdf.exists() {
            problems.push(format!(
                "{stem}: {} is absent (gitignored) — \
                 `cargo run -p xtask -- forms fetch --restore --stem {stem}` restores it from its note",
                recipe.source
            ));
            continue;
        }
        let digest = match sha256_of(&pdf) {
            Ok(d) => d,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        let raw = match pdftotext(&pdf, &recipe.flags) {
            Ok(r) => r,
            Err(e) => {
                problems.push(format!("{stem}: {e}"));
                continue;
            }
        };
        match regenerate(stem, &committed, &digest, &raw) {
            Err(e) => problems.push(e),
            Ok((_, Verdict::Unchanged)) => unchanged += 1,
            Ok((_, Verdict::Changed(text))) => {
                if check_only {
                    problems.push(format!(
                        "{stem}: the committed text layer is NOT what its own recipe reproduces \
                         ({} bytes committed, {} regenerated). --check, so nothing was written.",
                        committed.len(),
                        text.len()
                    ));
                } else {
                    std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
                    println!(
                        "  ★ REWROTE {stem}.txt — {} bytes -> {} bytes. The text of an authority \
                         CHANGED; re-verify every quotation drawn from it.",
                        committed.len(),
                        text.len()
                    );
                    written += 1;
                }
            }
        }
    }
    println!(
        "forms extract: {} text layer(s) — {unchanged} reproduce byte-for-byte, {written} rewritten, \
         {} unresolved",
        stems.len(),
        problems.len()
    );
    for p in &problems {
        println!("  ★ {p}");
    }
    if problems.is_empty() {
        println!("forms extract: OK — every text layer asked is exactly what its own recorded recipe produces.");
        Ok(())
    } else {
        Err(format!(
            "{} of {} text layer(s) could not be verified.",
            problems.len(),
            stems.len()
        ))
    }
}

/// `forms extract --adopt <stem>` — make a text layer's header say, correctly, how to remake it.
///
/// It computes the header the file's **own bytes prove**, compares it to the header on disk, and writes
/// only if they differ. Two things can be missing or wrong:
///
/// 1. **The whole header** (the 52 the rehearsal found) — written from the proven recipe.
/// 2. **The recipe line** (the 54 whose form feeds had been replaced by `\n` with nothing recording
///    it) — replaced, every other header line kept verbatim.
///
/// and one thing is completed rather than corrected: a stemless `# Regenerate:` line gains its stem
/// (see [`normalised_regenerate_line`], which states exactly what it will not touch).
///
/// **One rule throughout: the body is never modified, and nothing is written that has not been
/// demonstrated to reproduce the committed body byte-for-byte.**
///
/// ★ The recorded recipe is tried **first**, so a recipe that already works is never swapped for a
/// different one that happens to work too. On a document with no page breaks both page-break settings
/// reproduce the body, and preferring the discovered one would strip a correct field out of a header.
fn adopt(root: &Path, stem: &str) -> Result<(), String> {
    let stem = stem.trim_end_matches(".txt");
    let path = extract_dir(root).join(format!("{stem}.txt"));
    let committed =
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (header, body) = split(&committed);

    let (source, recorded) = if header.is_empty() {
        (locate_source(root, stem)?, None)
    } else {
        let (recipe, _) = parse_header(stem, &committed)?;
        (recipe.source.clone(), Some(recipe))
    };
    let pdf = root.join(&source);
    if !pdf.exists() {
        return Err(format!(
            "{stem}: {source} is absent (gitignored) — \
             `cargo run -p xtask -- forms fetch --restore --stem {stem}` restores it from its note, \
             and a recipe can only be PROVEN against the document itself."
        ));
    }
    let digest = sha256_of(&pdf)?;

    // What recipe does this body actually prove? The recorded one if it still holds, else whichever
    // candidate reproduces it — and if none does, a refusal rather than a guess.
    let (flags, page_breaks) = match &recorded {
        Some(recipe) => {
            if !digest.starts_with(&recipe.sha16) {
                return Err(format!(
                    "{stem}: {source} hashes to {}… but this text layer records {}…. ★ REFUSING: that \
                     is a different revision of the document, and adopting a recipe against it would \
                     attach this body's provenance to a document it did not come from.",
                    &digest[..16],
                    recipe.sha16
                ));
            }
            let raw = pdftotext(&pdf, &recipe.flags)?;
            if compose(recipe, &raw) == committed {
                (recipe.flags.clone(), recipe.page_breaks)
            } else {
                discover(stem, &body, &|f: &[String]| pdftotext(&pdf, f))?
            }
        }
        None => discover(stem, &body, &|f: &[String]| pdftotext(&pdf, f))?,
    };

    let recipe_line = format!(
        "# sha256:{}…  |  {}{}",
        &digest[..16],
        flag_spelling(&flags),
        page_breaks.spelling()
    );
    let new_header: Vec<String> = if header.is_empty() {
        adopted_header(stem, &source, &digest, &flags, page_breaks)
    } else {
        header
            .iter()
            .map(|l| {
                if l.starts_with("# sha256:") {
                    recipe_line.clone()
                } else {
                    normalised_regenerate_line(l, stem)
                }
            })
            .collect()
    };
    let out = format!("{}\n{body}", new_header.join("\n"));
    if out == committed {
        println!(
            "forms extract --adopt {stem}: nothing to adopt — the header already records the recipe \
             ({}{}) that reproduces this file byte-for-byte.",
            flag_spelling(&flags),
            page_breaks.spelling()
        );
        return Ok(());
    }

    // ★★ VERIFIED BEFORE IT IS WRITTEN, not asserted afterwards: the body is untouched, and the file
    //    the new header describes is exactly the file about to be written.
    let (_, new_body) = split(&out);
    if new_body != body {
        return Err(format!(
            "{stem}: --adopt would have changed the body ({} bytes -> {} bytes). REFUSING — adopting \
             a recipe must never rewrite an authority.",
            body.len(),
            new_body.len()
        ));
    }
    let (check_recipe, _) = parse_header(stem, &out)?;
    let check_raw = pdftotext(&pdf, &check_recipe.flags)?;
    if compose(&check_recipe, &check_raw) != out {
        return Err(format!(
            "{stem}: the header --adopt derived does not reproduce the file it was derived from. \
             REFUSING to write a recipe that does not work."
        ));
    }

    std::fs::write(&path, &out).map_err(|e| format!("{}: {e}", path.display()))?;
    let changed: Vec<String> = new_header
        .iter()
        .enumerate()
        .filter(|(i, l)| header.get(*i) != Some(l))
        .map(|(_, l)| l.clone())
        .collect();
    println!(
        "forms extract --adopt {stem}: recorded what the committed body PROVES\n  \
         source  {source}\n  digest  {}…\n  command {}{}\n  body    {} bytes, UNCHANGED\n  header  {}",
        &digest[..16],
        flag_spelling(&flags),
        page_breaks.spelling(),
        body.len(),
        if header.is_empty() {
            "written (there was none)".to_string()
        } else {
            format!("{} line(s) changed: {}", changed.len(), changed.join(" ; "))
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str =
        "# GENERATED — do not hand-edit. Text layer of design/forms/2024/f1040--2024.pdf\n\
                          # sha256:0a7a54354283044c…  |  pdftotext -layout\n\
                          # Regenerate: cargo run -p xtask -- forms extract f1040--2024\n#\n";

    fn digest_of(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    #[test]
    fn a_header_parses_into_the_recipe_that_made_the_file() {
        let text = format!("{HEADER}Form 1040 U.S. Individual Income Tax Return 2024\n");
        let (r, body) = parse_header("f1040--2024", &text).expect("parses");
        assert_eq!(r.source, "design/forms/2024/f1040--2024.pdf");
        assert_eq!(r.sha16, "0a7a54354283044c");
        assert_eq!(r.flags, vec!["-layout".to_string()]);
        assert_eq!(r.page_breaks, PageBreaks::FormFeed);
        assert_eq!(body, "Form 1040 U.S. Individual Income Tax Return 2024\n");
        assert_eq!(
            compose(&r, &body),
            text,
            "compose(parse(x)) must be x, or a regeneration rewrites the header"
        );

        // ★ The other two recorded spellings, both live in the tree.
        let none = text.replace("pdftotext -layout", "pdftotext (no flags)");
        assert!(parse_header("x", &none).expect("parses").0.flags.is_empty());
        let lf = text.replace(
            "pdftotext -layout",
            "pdftotext -layout  |  form feeds -> LF",
        );
        assert_eq!(
            parse_header("x", &lf).expect("parses").0.page_breaks,
            PageBreaks::Lf
        );
    }

    /// ★★★ **B1 — the page-break field is watched DECIDING.** This is the field that had to be added:
    /// 54 of the 126 committed extracts have their form feeds replaced by `\n`, and nothing recorded
    /// it. Without the field, the recorded command (`pdftotext -layout`) regenerates those 54 files
    /// **with page breaks put back**, i.e. it rewrites an authority on every run while reporting
    /// success. Planted here by dropping the field from a header whose body needs it.
    #[test]
    fn dropping_the_page_break_field_rewrites_the_file_and_the_field_stops_it() {
        let raw = "page one\n\u{c}page two\n";
        let lf_header = HEADER.replace(
            "pdftotext -layout",
            "pdftotext -layout  |  form feeds -> LF",
        );
        let committed = format!("{lf_header}page one\n\npage two\n");
        let digest = "0a7a54354283044cdeadbeef";

        let (_, verdict) = regenerate("f1040--2024", &committed, digest, raw).expect("regenerates");
        assert_eq!(
            verdict,
            Verdict::Unchanged,
            "with the field recorded, the committed file is reproduced exactly"
        );

        // The plant: the same body, the same command, the field removed.
        let without = committed.replace("  |  form feeds -> LF", "");
        let (_, verdict) = regenerate("f1040--2024", &without, digest, raw).expect("regenerates");
        match verdict {
            Verdict::Changed(text) => assert!(
                text.contains('\u{c}'),
                "the regeneration must put the form feeds back — that is the defect being caught"
            ),
            Verdict::Unchanged => panic!(
                "an extract whose page breaks were replaced must NOT report `unchanged` when the \
                 recipe says to keep them — that is a silent rewrite of an authority"
            ),
        }
    }

    /// ★★★ **B1 — a revised document is REFUSED, not extracted over.**
    ///
    /// The header records which revision the text layer is of. If the PDF on disk is a different
    /// document, regenerating would replace an audited authority with an unaudited one *and report
    /// success*, because nothing downstream reads the PDF — they all read this text.
    #[test]
    fn a_pdf_that_is_not_the_recorded_revision_is_refused() {
        let text = format!("{HEADER}Form 1040 (2024)\n");
        let e = regenerate(
            "f1040--2024",
            &text,
            &digest_of(b"a different document"),
            "x",
        )
        .expect_err("a different document must REFUSE");
        assert!(e.contains("REFUSING to regenerate"), "{e}");
        assert!(
            e.contains("0a7a54354283044c"),
            "the refusal names the recorded revision: {e}"
        );
        // And the matching one does not refuse.
        let real = format!("{:x}", Sha256::digest(b"pretend this is f1040--2024.pdf"));
        let ok_header = HEADER.replace("0a7a54354283044c", &real[..16]);
        let ok = format!("{ok_header}Form 1040 (2024)\n");
        assert!(regenerate("f1040--2024", &ok, &real, "Form 1040 (2024)\n").is_ok());
    }

    /// ★★ A headerless extract is refused and pointed at `--adopt`; the flags are never guessed.
    #[test]
    fn a_headerless_extract_is_refused_rather_than_guessed_at() {
        let e = parse_header(
            "f8995a--2025",
            "8995-A Qualified Business Income Deduction\n",
        )
        .expect_err("no header must refuse");
        assert!(e.contains("REFUSING to guess"), "{e}");
        assert!(
            e.contains("--adopt f8995a--2025"),
            "the refusal names the remedy: {e}"
        );

        // ★ A header present but incomplete is also refused, each field by name.
        let no_sha =
            "# GENERATED — do not hand-edit. Text layer of design/forms/2025/x.pdf\n#\nbody\n";
        assert!(parse_header("x", no_sha)
            .expect_err("no recipe line")
            .contains("records no `# sha256:"));
        let no_src = "# sha256:0a7a54354283044c…  |  pdftotext -layout\n#\nbody\n";
        assert!(parse_header("x", no_src)
            .expect_err("no source")
            .contains("names no source document"));
    }

    /// ★★★ **B1 — `--adopt` is watched REFUSING a body the PDF cannot reproduce**, and watched
    /// picking the right recipe out of four when it can. Guessing here would write a false provenance
    /// claim into the file whose only job is to carry a true one.
    #[test]
    fn adopt_derives_the_recipe_by_reproduction_and_refuses_when_none_works() {
        let layout = "line 1   label\n\u{c}line 2   label\n";
        let plain = "line 1\nlabel\n\u{c}line 2\nlabel\n";
        let raw_for = |flags: &[String]| -> Result<String, String> {
            Ok(if flags.is_empty() {
                plain.to_string()
            } else {
                layout.to_string()
            })
        };

        // The committed body is the -layout output with its form feeds replaced.
        let body = layout.replace('\u{c}', "\n");
        let (flags, pb) = discover("x", &body, &raw_for).expect("one recipe reproduces it");
        assert_eq!(flags, vec!["-layout".to_string()]);
        assert_eq!(pb, PageBreaks::Lf);

        // The plain output, form feeds intact.
        let (flags, pb) = discover("x", plain, &raw_for).expect("the no-flags recipe");
        assert!(flags.is_empty());
        assert_eq!(pb, PageBreaks::FormFeed);

        // ★ THE PLANT: a body neither invocation produces — a hand-edited or differently-tooled file.
        let e = discover("x", "line 1 label\nline 2 label\n", &raw_for)
            .expect_err("an unreproducible body must REFUSE");
        assert!(e.contains("no recipe reproduces the committed body"), "{e}");
        assert!(e.contains("REFUSING to write a header"), "{e}");
        assert!(
            e.contains("pdftotext -layout") && e.contains("(no flags)"),
            "the refusal must show what it tried: {e}"
        );
    }

    /// ★★ The header `--adopt` writes is readable by [`parse_header`] and round-trips — otherwise the
    /// backfill would produce 52 files the tool itself cannot regenerate.
    #[test]
    fn an_adopted_header_round_trips_through_the_parser() {
        for (flags, pb) in candidates() {
            let digest = format!("{:x}", Sha256::digest(b"document"));
            let header = adopted_header(
                "f8995a--2025",
                "design/forms/2025/f8995a--2025.pdf",
                &digest,
                &flags,
                pb,
            );
            let body = "8995-A\n\u{c}page two\n";
            let text = format!("{}\n{}", header.join("\n"), pb.apply(body));
            let (r, got_body) = parse_header("f8995a--2025", &text).expect("adopted header parses");
            assert_eq!(r.flags, flags);
            assert_eq!(r.page_breaks, pb);
            assert_eq!(r.source, "design/forms/2025/f8995a--2025.pdf");
            assert_eq!(got_body, pb.apply(body));
            assert_eq!(
                regenerate("f8995a--2025", &text, &digest, body)
                    .expect("regenerates")
                    .1,
                Verdict::Unchanged,
                "an adopted file must immediately regenerate to itself ({flags:?}, {pb:?})"
            );
        }
    }

    /// ★★ **The `# Regenerate:` normaliser is watched leaving alone what it must not touch.**
    ///
    /// It completes the one stemless spelling and nothing else. The two values it must keep verbatim
    /// are real provenance — the draft archiver that *fetched* those 15 documents, and the literal
    /// `pdftotext` run against a bundled template — and a normaliser that overwrote them with a command
    /// that merely also works would delete how the file came to exist.
    #[test]
    fn the_regenerate_normaliser_completes_one_spelling_and_preserves_every_other() {
        assert_eq!(
            normalised_regenerate_line(STEMLESS_REGENERATE, "f1040--2024"),
            "# Regenerate: cargo run -p xtask -- forms extract f1040--2024"
        );
        // ★ Already complete: unchanged, so re-running --adopt is not a rewrite.
        let done = "# Regenerate: cargo run -p xtask -- forms extract f1040--2024";
        assert_eq!(normalised_regenerate_line(done, "f1040--2024"), done);
        // ★ THE PLANTS: the two other routes in the tree, which must survive byte-exact.
        for keep in [
            "# Regenerate: .venv/bin/python scripts/archive_drafts.py --relayout 2026",
            "# Regenerate: pdftotext -layout crates/btctax-forms/forms/2024/f8283.pdf \
             design/forms/extract/f8283--2024.txt",
            "# ★ DRAFT — evidence only, never transcribed as authority (design/ty2025/SPEC.md).",
            "# GENERATED — do not hand-edit. Text layer of design/forms/2024/f1040--2024.pdf",
        ] {
            assert_eq!(
                normalised_regenerate_line(keep, "f1040--2024"),
                keep,
                "the normaliser must leave this line alone"
            );
        }
    }

    /// ★★★ **Every committed text layer carries a recipe this tool can read** — the FR-140 gap, as a
    /// test that reds if a new extract lands without a header or with one nobody can parse.
    ///
    /// It reads the tree, so it cannot go stale as the corpus grows; and it needs no PDF, so it runs
    /// in CI's network-isolated job exactly as it runs here.
    #[test]
    fn every_committed_extract_records_a_readable_recipe() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("repo root")
            .to_path_buf();
        let stems = all_stems(&root).expect("the extract tree reads");
        assert!(
            stems.len() >= 126,
            "only {} extracts found — this test has stopped reading the tree",
            stems.len()
        );
        let mut headerless = Vec::new();
        let mut unparsable = Vec::new();
        for stem in &stems {
            let text = std::fs::read_to_string(extract_dir(&root).join(format!("{stem}.txt")))
                .expect("extract reads");
            match parse_header(stem, &text) {
                Ok((r, _)) => {
                    assert!(
                        !r.source.is_empty() && r.source.ends_with(".pdf"),
                        "{stem} names {:?} as its source",
                        r.source
                    );
                    // ★ A `# Regenerate:` line naming THIS tool must name the text layer it
                    //   regenerates. Pasted without a stem it prints a usage message — which is what
                    //   58 of these files did, and half of FR-140's finding.
                    for line in &r.header {
                        if line.starts_with("# Regenerate:") && line.contains("forms extract") {
                            assert!(
                                line.trim_end().ends_with(stem.as_str()),
                                "{stem}'s regeneration command does not name it, so pasting it \
                                 verbatim regenerates nothing: {line:?}"
                            );
                        }
                    }
                }
                Err(e) if e.contains("records nothing about how it was made") => {
                    headerless.push(stem.clone());
                }
                Err(e) => unparsable.push(e),
            }
        }
        assert!(
            headerless.is_empty(),
            "{} text layer(s) record NOTHING about how they were made, so nothing can regenerate \
             them and the `-layout` decision is lost: {headerless:?}",
            headerless.len()
        );
        assert!(unparsable.is_empty(), "{unparsable:#?}");
    }

    /// ★★ And the recipes are not all one value — a corpus where every file said the same thing would
    /// make the test above pass without the fields carrying any information.
    #[test]
    fn the_recorded_recipes_span_the_conventions_the_tree_actually_uses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("repo root")
            .to_path_buf();
        let (mut layout, mut plain, mut ff, mut lf) = (0usize, 0usize, 0usize, 0usize);
        for stem in all_stems(&root).expect("tree") {
            let text = std::fs::read_to_string(extract_dir(&root).join(format!("{stem}.txt")))
                .expect("reads");
            let Ok((r, body)) = parse_header(&stem, &text) else {
                continue;
            };
            if r.flags.is_empty() {
                plain += 1;
            } else {
                layout += 1;
            }
            match r.page_breaks {
                PageBreaks::FormFeed => ff += 1,
                PageBreaks::Lf => lf += 1,
            }
            // ★ The page-break field must agree with the file: a body with form feeds in it cannot
            //   have been produced by the LF recipe, and vice versa for a multi-page document.
            if r.page_breaks == PageBreaks::Lf {
                assert!(
                    !body.contains('\u{c}'),
                    "{stem} records `form feeds -> LF` and its body still has form feeds"
                );
            }
        }
        assert!(
            layout > 0 && plain > 0,
            "both flag conventions must be present: {layout}/{plain}"
        );
        assert!(
            ff > 0 && lf > 0,
            "both page-break conventions must be present: {ff}/{lf}"
        );
    }
}
