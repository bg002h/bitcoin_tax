//! Deterministic man-page generator for the `btctax` binaries.
//!
//! One roff page per clap command, recursing `Command::get_subcommands()` (clap_mangen's
//! single-root render only lists subcommand NAMES, never their args — so each subcommand needs
//! its OWN page). Pages are named git-style: `btctax.1`, `btctax-<sub>.1`, `btctax-<sub>-<sub>.1`.
//! The file-format long-help authored on the subcommand args (see `btctax_cli::cli`) rides along
//! automatically — zero drift with `--help`.
//!
//! Determinism: no `#[command(version)]` and no embedded date (clap_mangen defaults `date` to ""),
//! and the hand-authored roff below is static — so the committed `docs/man/*.1` are reproducible
//! (guarded by `gen_docs_is_deterministic`).

use btctax_cli::cli::Cli;
use clap::CommandFactory;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Repo root, resolved at compile time from this crate's location (`<root>/crates/xtask`) so the
/// generator is independent of the working directory it is invoked from.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask crate lives at <repo>/crates/xtask")
        .to_path_buf()
}

/// `<repo>/docs/man`.
pub fn man_dir() -> PathBuf {
    repo_root().join("docs/man")
}

/// `<repo>/docs/pdf` (build artifacts; git-ignored — gropdf embeds a timestamp so PDFs are not
/// byte-reproducible).
pub fn pdf_dir() -> PathBuf {
    repo_root().join("docs/pdf")
}

/// Deterministic `.TH` source/manual for every page.
const SOURCE: &str = "btctax";
const MANUAL: &str = "btctax manual";

/// Write every generated man page to `docs/man/`.
pub fn write_man_pages() -> std::io::Result<()> {
    let dir = man_dir();
    std::fs::create_dir_all(&dir)?;
    for (name, bytes) in render_generated_pages() {
        let path = dir.join(&name);
        std::fs::write(&path, &bytes)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// Render a PDF for EVERY committed `.1` in `docs/man/` (generated CLI pages + the hand-authored
/// TUI pages) into `docs/pdf/`, via `groff -k -man -T pdf`. Smoke-checks the `%PDF` magic on each
/// output (a build-step guard, not a unit test). Requires `groff` with the `pdf` device on PATH.
pub fn write_pdfs() -> std::io::Result<()> {
    let man = man_dir();
    let out_dir = pdf_dir();
    std::fs::create_dir_all(&out_dir)?;

    let mut names: Vec<String> = std::fs::read_dir(&man)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".1"))
        .collect();
    names.sort();

    for name in names {
        let src = man.join(&name);
        let stem = name.strip_suffix(".1").expect("filtered to *.1");
        let out = out_dir.join(format!("{stem}.pdf"));
        // -k runs preconv so the UTF-8 in the pages (§, em-dash, arrows) is handled.
        let result = std::process::Command::new("groff")
            .args(["-k", "-man", "-T", "pdf"])
            .arg(&src)
            .output()?;
        if !result.status.success() {
            return Err(std::io::Error::other(format!(
                "groff failed for {name}: {}",
                String::from_utf8_lossy(&result.stderr)
            )));
        }
        // Smoke-check: a valid PDF starts with the %PDF magic.
        if !result.stdout.starts_with(b"%PDF") {
            return Err(std::io::Error::other(format!(
                "groff output for {name} is not a PDF (missing %PDF magic)"
            )));
        }
        std::fs::write(&out, &result.stdout)?;
        println!("wrote {}", out.display());
    }
    Ok(())
}

/// Render every GENERATED man page (the root + one per subcommand, recursively), keyed by filename.
/// Does NOT include the hand-authored TUI pages (`btctax-tui.1` / `btctax-tui-edit.1`).
pub fn render_generated_pages() -> BTreeMap<String, Vec<u8>> {
    let root = Cli::command();
    let mut pages = BTreeMap::new();
    pages.insert("btctax.1".to_string(), render_root(&root));
    render_subtree(&root, "btctax", &mut pages);
    // #41 Part C: the SEPARATE online updater binary — its own single root page (no subcommands),
    // rendered from its clap doc-comments exactly like the tax pages (single-source docs).
    let updater = btctax_update_prices::Cli::command().disable_help_subcommand(true);
    let man = clap_mangen::Man::new(updater)
        .title("btctax-update-prices")
        .source(SOURCE)
        .manual(MANUAL);
    let mut buf = Vec::new();
    man.render(&mut buf)
        .expect("clap_mangen render is infallible for in-memory writer");
    pages.insert("btctax-update-prices.1".to_string(), buf);
    pages
}

/// Recurse the subcommand tree, rendering one standard page per command. `prefix` is the dashed
/// path so far (e.g. `btctax-reconcile`).
fn render_subtree(cmd: &clap::Command, prefix: &str, pages: &mut BTreeMap<String, Vec<u8>>) {
    for sub in cmd.get_subcommands() {
        // The auto-generated `help` pseudo-subcommand has no documentation value.
        if sub.get_name() == "help" {
            continue;
        }
        let dashed = format!("{prefix}-{}", sub.get_name());
        // Drop clap's auto `help` pseudo-subcommand so the page's SUBCOMMANDS list carries no
        // dangling `<cmd>-help(1)` cross-reference (there is no such page).
        let man = clap_mangen::Man::new(sub.clone().disable_help_subcommand(true))
            .title(dashed.clone())
            .source(SOURCE)
            .manual(MANUAL);
        let mut buf = Vec::new();
        man.render(&mut buf)
            .expect("clap_mangen render is infallible for in-memory writer");
        pages.insert(format!("{dashed}.1"), buf);
        render_subtree(sub, &dashed, pages);
    }
}

/// Render the ROOT `btctax.1`: the standard clap_mangen sections in conventional order, with the
/// hand-authored DESCRIPTION / FILES / EXAMPLES stitched in (roff). The file-format specifics live
/// on the SUBCOMMAND pages (single source of truth); the root only cross-links + overviews.
fn render_root(root: &clap::Command) -> Vec<u8> {
    let man = clap_mangen::Man::new(root.clone().disable_help_subcommand(true))
        .title("btctax")
        .source(SOURCE)
        .manual(MANUAL);
    let mut buf: Vec<u8> = Vec::new();
    man.render_title(&mut buf).unwrap();
    man.render_name_section(&mut buf).unwrap();
    man.render_synopsis_section(&mut buf).unwrap();
    buf.extend_from_slice(ROOT_DESCRIPTION.as_bytes());
    man.render_options_section(&mut buf).unwrap();
    man.render_subcommands_section(&mut buf).unwrap();
    buf.extend_from_slice(ROOT_FILES.as_bytes());
    buf.extend_from_slice(ROOT_EXAMPLES.as_bytes());
    buf
}

const ROOT_DESCRIPTION: &str = r#".SH DESCRIPTION
.B btctax
is an offline, single-user US Bitcoin tax ledger (Phase 1). It ingests exchange/wallet export
files into an append-only event log inside an encrypted vault, reconciles ambiguous movements
through explicit decision events, and projects holdings, realized disposals, income, and the
per-tax-year federal tax artifacts (Form 8949, Schedule D, Schedule SE, Form 8283).
.PP
All persistent state lives in a single passphrase-encrypted vault. The passphrase is read from the
.B BTCTAX_PASSPHRASE
environment variable when set (non-interactive/scripted use), otherwise from a secure prompt.
.PP
Most subcommands that reference an event do so by an EVENT REFERENCE (the canonical
.I EventId ) .
Event references are obtained from the
.B event
column of the projection CSVs written by
.B export-snapshot
(disposals.csv, removals.csv, income.csv), or from
.B report .
File and structured-argument FORMATS (the key backup, the export CSV set, the import-selections
CSV, the classify-raw JSON, the select-lots picks) are documented on each owning subcommand's page
(e.g.
.BR btctax-reconcile-import-selections (1)).
"#;

const ROOT_FILES: &str = r#".SH FILES
.TP
.B vault.pgp
The encrypted vault: the append-only event log + typed side-tables. The one file that holds all
state; never edited by hand. Path set by the global \fB--vault\fR option (default: vault.pgp).
.TP
.B vault.key
The passphrase-encrypted private key, sibling to the vault. Created by \fBinit\fR; back it up with
\fB--key-backup\fR / \fBbackup-key\fR (an ASCII-armored PGP PRIVATE KEY BLOCK).
.TP
.B <out>/snapshot.sqlite
Decrypted SQLite database written by \fBexport-snapshot\fR (the NFR2 plaintext exception).
.TP
.B <out>/*.csv
Projection CSVs written by \fBexport-snapshot\fR: lots.csv, disposals.csv, removals.csv,
income.csv (always); form8949.csv, schedule_d.csv, form8283.csv, schedule_se.csv (with
\fB--tax-year\fR). The \fBevent\fR column supplies the event references that \fBreconcile\fR
subcommands (e.g. \fBselect-lots\fR, \fBset-donation-details\fR) consume.
"#;

const ROOT_EXAMPLES: &str = r#".SH EXAMPLES
.PP
Create a vault and force a key backup:
.PP
.RS 4
.nf
btctax --vault vault.pgp init --key-backup key-backup.asc
.fi
.RE
.PP
Import exports, then review holdings and the current tax year:
.PP
.RS 4
.nf
btctax import coinbase.csv river.csv
btctax report
btctax report --tax-year 2025
.fi
.RE
.PP
Export the decrypted snapshot + projection CSVs (with the per-tax-year filing artifacts):
.PP
.RS 4
.nf
btctax export-snapshot --out ./snapshot --tax-year 2025
.fi
.RE
.PP
Pick the exact lots a disposal consumes (specific identification), using event references read
from the export CSVs:
.PP
.RS 4
.nf
btctax reconcile select-lots import|gemini|trade|T-2.O-2 \e
    --from import|coinbase|X#0:25000 --from import|river|Y#1:5000
.fi
.RE
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Read a committed page as UTF-8 (panicking with a helpful message if it is missing/stale).
    fn committed(name: &str) -> String {
        let path = man_dir().join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| {
            panic!("committed man page {name} missing ({e}); run `cargo run -p xtask -- docs`")
        });
        String::from_utf8(bytes).expect("man page is valid UTF-8")
    }

    /// C1-guard: every subcommand (recursively) has a committed page `docs/man/btctax[-<path>].1`.
    /// A new subcommand added without regenerating docs fails here.
    #[test]
    fn manpage_covers_every_subcommand() {
        fn walk(cmd: &clap::Command, prefix: &str, dir: &Path, missing: &mut Vec<String>) {
            for sub in cmd.get_subcommands() {
                if sub.get_name() == "help" {
                    continue;
                }
                let dashed = format!("{prefix}-{}", sub.get_name());
                let file = format!("{dashed}.1");
                if !dir.join(&file).exists() {
                    missing.push(file);
                }
                walk(sub, &dashed, dir, missing);
            }
        }
        let dir = man_dir();
        assert!(
            dir.join("btctax.1").exists(),
            "root btctax.1 must be committed"
        );
        let mut missing = Vec::new();
        walk(&Cli::command(), "btctax", &dir, &mut missing);
        assert!(
            missing.is_empty(),
            "subcommands missing a committed man page (regenerate with `cargo run -p xtask -- docs`): {missing:?}"
        );
    }

    /// C1: each file-format EXAMPLE appears on ITS OWN subcommand page (not the root). Tokens are
    /// hyphen-free because roff escapes `-` to `\-`; these tokens survive verbatim.
    #[test]
    fn file_format_examples_present_in_manpage() {
        assert!(
            committed("btctax-init.1").contains("BEGIN PGP PRIVATE KEY BLOCK"),
            "init page must carry the key-armor example"
        );
        assert!(
            committed("btctax-backup-key.1").contains("BEGIN PGP PRIVATE KEY BLOCK"),
            "backup-key page must carry the key-armor example"
        );
        assert!(
            committed("btctax-export-snapshot.1")
                .contains("event,kind,removed_at,lot,sat,basis,fmv_at_transfer"),
            "export-snapshot page must carry the projection CSV header"
        );
        assert!(
            committed("btctax-reconcile-import-selections.1")
                .contains("disposal_ref,origin_event_id,split_sequence,sat"),
            "import-selections page must carry the required CSV header"
        );
        assert!(
            committed("btctax-reconcile-classify-raw.1").contains(
                r#"{"Acquire":{"sat":2000000,"usd_cost":"1680.00","fee_usd":"5.00","basis_source":"ExchangeProvided"}}"#
            ),
            "classify-raw page must carry the JSON payload example"
        );
        assert!(
            committed("btctax-reconcile-select-lots.1").contains("import|coinbase|X#0:25000"),
            "select-lots page must carry the lot-pick example"
        );
    }

    /// verbatim_doc_comment guard: multi-line examples are NOT reflowed into one line — the armor
    /// BEGIN/END markers land on distinct lines, and the import-selections header + sample row on
    /// distinct lines.
    #[test]
    fn manpage_multiline_examples_survive() {
        let init = committed("btctax-init.1");
        let begin = init
            .lines()
            .position(|l| l.contains("BEGIN PGP PRIVATE KEY BLOCK"));
        let end = init
            .lines()
            .position(|l| l.contains("END PGP PRIVATE KEY BLOCK"));
        assert!(
            begin.is_some() && end.is_some() && begin != end,
            "armor BEGIN and END must survive on separate lines (begin={begin:?}, end={end:?})"
        );

        let imp = committed("btctax-reconcile-import-selections.1");
        let header = imp
            .lines()
            .position(|l| l.contains("disposal_ref,origin_event_id,split_sequence,sat"));
        let sample = imp.lines().position(|l| l.contains("import|gemini|trade"));
        assert!(
            header.is_some() && sample.is_some() && header != sample,
            "CSV header and sample row must survive on separate lines (header={header:?}, sample={sample:?})"
        );
    }

    /// The generator is deterministic (no dates / no `#[command(version)]`): two runs are
    /// byte-identical, AND the committed pages match a fresh generation (fails on stale docs).
    #[test]
    fn gen_docs_is_deterministic() {
        let a = render_generated_pages();
        let b = render_generated_pages();
        assert_eq!(a, b, "generation must be byte-identical across runs");

        let dir = man_dir();
        for (name, bytes) in &a {
            let on_disk = std::fs::read(dir.join(name)).unwrap_or_else(|e| {
                panic!("committed {name} missing ({e}); run `cargo run -p xtask -- docs`")
            });
            assert_eq!(
                &on_disk, bytes,
                "committed docs/man/{name} is STALE; regenerate with `cargo run -p xtask -- docs`"
            );
        }
    }

    /// Structural guard over EVERY committed page (generated + the hand-authored TUI pages):
    /// each has NAME + SYNOPSIS; the root btctax.1 has DESCRIPTION + FILES + EXAMPLES; and the
    /// hand-authored TUI pages document their tab set + keys (tui-edit lists `?`, `V`, `O`).
    #[test]
    fn manpages_have_required_sections() {
        let dir = man_dir();
        let mut pages: Vec<String> = std::fs::read_dir(&dir)
            .expect("docs/man exists")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".1"))
            .collect();
        pages.sort();
        assert!(
            pages.len() >= 40,
            "expected root + subcommands + 2 TUI pages, got {pages:?}"
        );

        for name in &pages {
            let page = committed(name);
            assert!(
                page.contains(".SH NAME"),
                "{name} is missing a NAME section"
            );
            assert!(
                page.contains(".SH SYNOPSIS"),
                "{name} is missing a SYNOPSIS section"
            );
        }

        // Root: the hand-authored DESCRIPTION / FILES / EXAMPLES stitched by render_root.
        let root = committed("btctax.1");
        for sec in [".SH DESCRIPTION", ".SH FILES", ".SH EXAMPLES"] {
            assert!(root.contains(sec), "root btctax.1 missing {sec}");
        }

        // Hand-authored TUI pages: tab set + keymap.
        for tui in ["btctax-tui.1", "btctax-tui-edit.1"] {
            let p = committed(tui);
            for tab in [
                "Holdings",
                "Disposals",
                "Income",
                "Tax",
                "Forms",
                "Compliance",
            ] {
                assert!(p.contains(tab), "{tui} must document the {tab} tab");
            }
        }
        let edit = committed("btctax-tui-edit.1");
        for key in ["\n.B ?", "\n.B V", "\n.B O"] {
            assert!(
                edit.contains(key),
                "btctax-tui-edit.1 must list the {key:?} action key (copy of the ? overlay)"
            );
        }
        // The tax-inputs mode entry key. Line-anchored on BOTH ends (`\n.B T\n`, not just
        // `\n.B T`): the page already contains `\n.B Tab / Shift\-Tab` (Navigation), which
        // contains `\n.B T` as a substring, so an unanchored match would pass vacuously and
        // guard nothing. `\n.B T\n` only matches a real `.B T` keymap entry on its own line.
        assert!(
            edit.contains("\n.B T\n"),
            "btctax-tui-edit.1 must list the `T` (tax-inputs mode) action key on its own \
             .B line"
        );
    }
}
