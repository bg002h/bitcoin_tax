//! xtask — developer tooling for the btctax workspace.
//!
//! `cargo run -p xtask -- docs` regenerates the committed man pages under `docs/man/`.
//! `cargo run -p xtask -- docs --pdf` additionally renders `docs/pdf/*.pdf` (requires `groff`).
//! `cargo run -p xtask -- check-isolation` asserts no HTTP client in the tax crates (#41 Part C).
//! `cargo run -p xtask -- dump-fields <pdf>` lists a PDF's AcroForm field names (map authoring).
//! `cargo run -p xtask -- cite-check` verifies every quotation in the Schedule 1-A design docs against
//! the committed IRS text-layer extracts. `… -- extract-schedule-1a` regenerates those extracts.

mod archive_check;
mod authority_conflicts;
mod authority_manifest;
mod authority_refresh;
mod box_census;
/// N1 — asserts `btctax_core::tax::capital_loss_carryover` is verbatim and complete against the 2025
/// Schedule D instructions' text layer.
///
/// `#[cfg(test)]` because it has no operator-facing mode: unlike `line_coverage_check`, whose `run()`
/// prints a coverage REPORT worth reading, this one answers a yes/no question and the answer belongs
/// in the suite, where `make check` asks it on every commit rather than when someone remembers to.
#[cfg(test)]
mod capital_loss_carryover_check;
mod census_join;
mod check_isolation;
mod cite_check;
mod dependents_grid;
mod docs;
mod dump_fields;
mod examples;
mod form_delta;
mod form_geometry;
mod harness_check;
mod label_reader;
mod line_coverage_check;
mod package_check;
mod prompt_check;
mod r15_stop_list;
/// Half 1a of the Schedule 1-A conformance KAT — the 48 entry labels, adjudicated by `label_reader`'s
/// two witnesses over the committed geometry and compared to `Schedule1A`'s own leaves.
///
/// `#[cfg(test)]` because it has no operator-facing mode, exactly like `capital_loss_carryover_check`:
/// it answers a yes/no question, and the answer belongs in the suite.
#[cfg(test)]
mod schedule_1a_membership;
mod verdict_reach;
/// FR-108 — a filer-facing sentence with the wrap indentation still inside its quotes.
mod wrapped_literal_check;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("docs") => {
            if let Err(e) = docs::write_man_pages() {
                eprintln!("xtask docs: {e}");
                std::process::exit(1);
            }
            if args.iter().any(|a| a == "--pdf") {
                if let Err(e) = docs::write_pdfs() {
                    eprintln!("xtask docs --pdf: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("examples") => {
            examples::run();
        }
        Some("subcommand-coverage") => {
            examples::run_coverage();
        }
        Some("authority-conflicts") => {
            if let Err(e) = authority_conflicts::run() {
                eprintln!("xtask authority-conflicts: {e}");
                std::process::exit(1);
            }
        }
        // ★★★ FR-29 / SPEC §9 G7 — the Form 8615 prompts and help strings against the extracts they
        //     are transcribed from, both directions. See `prompt_check`'s module doc.
        Some("prompt-check") => {
            if let Err(e) = prompt_check::run() {
                eprintln!("xtask prompt-check: a prompt no longer matches the form:\n{e}");
                std::process::exit(1);
            }
        }
        Some("cite-check") => {
            if let Err(e) = cite_check::run() {
                eprintln!("xtask cite-check: {e}");
                std::process::exit(1);
            }
        }
        Some("extract-schedule-1a") => {
            if let Err(e) = cite_check::extract() {
                eprintln!("xtask extract-schedule-1a: {e}");
                std::process::exit(1);
            }
        }
        Some("archive-check") => {
            if let Err(e) = archive_check::run() {
                eprintln!("xtask archive-check: {e}");
                std::process::exit(1);
            }
        }
        Some("verdict-reach") => {
            if let Err(e) = verdict_reach::run() {
                eprintln!("xtask verdict-reach: {e}");
                std::process::exit(1);
            }
        }
        Some("classify-path") => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- classify-path <path>");
                std::process::exit(2);
            };
            if let Err(e) = archive_check::classify_path(path) {
                eprintln!("xtask classify-path: {e}");
                std::process::exit(1);
            }
        }
        Some("authority-manifest") => {
            if args.iter().any(|a| a == "--regen") {
                match authority_manifest::regen(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .and_then(std::path::Path::parent)
                        .expect("workspace root"),
                ) {
                    Ok(n) => println!("authority-manifest: regenerated {n} entries"),
                    Err(e) => {
                        eprintln!("xtask authority-manifest --regen: {e}");
                        std::process::exit(1);
                    }
                }
            } else if let Err(e) = authority_manifest::run() {
                eprintln!("xtask authority-manifest: {e}");
                std::process::exit(1);
            }
        }
        Some("form-delta") => {
            let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
                eprintln!(
                    "usage: cargo run -p xtask -- form-delta <old-stem> <new-stem>\n\
                     e.g. form-delta f6251--2025 f6251--2026-DRAFT"
                );
                std::process::exit(2);
            };
            if let Err(e) = form_delta::run(a, b) {
                eprintln!("xtask form-delta: {e}");
                std::process::exit(1);
            }
        }
        Some("port-status") => {
            let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
                eprintln!(
                    "usage: cargo run -p xtask -- port-status <prior-tag> <new-tag>\n\
                     e.g. port-status 2025 2026-DRAFT   (prints the work list's two tables from the emitting surface)"
                );
                std::process::exit(2);
            };
            match form_delta::port_status(a, b) {
                Ok(s) => print!("{s}"),
                Err(e) => {
                    eprintln!("xtask port-status: {e}");
                    std::process::exit(1);
                }
            }
        }
        // ★ ON DEMAND ONLY — it needs the network, and `make check` must stay offline. See the
        //   module doc: the suite tests the pure comparison, never the fetch.
        Some("authority-refresh") => {
            if !args.iter().any(|a| a == "--check") {
                eprintln!(
                    "usage: cargo run -p xtask -- authority-refresh --check [--from-dir <dir>]"
                );
                std::process::exit(2);
            }
            let from_dir = args
                .iter()
                .position(|a| a == "--from-dir")
                .and_then(|i| args.get(i + 1))
                .map(std::path::PathBuf::from);
            if let Err(e) = authority_refresh::run(from_dir) {
                eprintln!("xtask authority-refresh: {e}");
                std::process::exit(1);
            }
        }
        Some("extract-geometry") => {
            let Some(stem) = args.get(1) else {
                eprintln!(
                    "usage: cargo run -p xtask -- extract-geometry <stem>   e.g. f1040s1a--2025"
                );
                std::process::exit(2);
            };
            if let Err(e) = form_geometry::extract(stem) {
                eprintln!("xtask extract-geometry: {e}");
                std::process::exit(1);
            }
        }
        Some("label-census") => {
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- label-census <stem>   e.g. f1040s1a--2025");
                std::process::exit(2);
            };
            if let Err(e) = label_reader::run(stem) {
                eprintln!("xtask label-census: {e}");
                std::process::exit(1);
            }
        }
        Some("dependents-grid") => {
            // ★★★ T8 / R6: MEASURE the TY2025+ Dependents grid off the form and print the map
            //     section, so `forms/<year>/f1040.map.toml` is generated rather than typed.
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- dependents-grid <stem>   e.g. f1040--2025");
                std::process::exit(2);
            };
            if let Err(e) = dependents_grid::run(stem) {
                eprintln!("xtask dependents-grid: {e}");
                std::process::exit(1);
            }
        }
        Some("label-boxes") => {
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- label-boxes <stem>");
                std::process::exit(2);
            };
            if let Err(e) = label_reader::boxes_tsv(stem) {
                eprintln!("xtask label-boxes: {e}");
                std::process::exit(1);
            }
        }
        Some("label-proof") => {
            let Some(stem) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- label-proof <stem> [out.pdf]");
                std::process::exit(2);
            };
            let out = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("/tmp/{stem}-label-proof.pdf"));
            if let Err(e) = label_reader::proof(stem, &out) {
                eprintln!("xtask label-proof: {e}");
                std::process::exit(1);
            }
        }
        Some("harness-check") => {
            if let Err(e) = harness_check::run() {
                eprintln!("xtask harness-check: {e}");
                std::process::exit(1);
            }
        }
        Some("line-coverage") => {
            // ★ §G-11: every printed money field declares which IRS instruction production it
            //   transcribes, checked verbatim against design/forms/extract/.
            match line_coverage_check::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask line-coverage: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("census-join") => {
            // ★★★ SPEC_interview R2.2 / T3: every `unmodeled` census entry on the seven forms the
            //   interview reaches is ANNOUNCED or REFUSED, never silent — with the DIRECTION rule
            //   (no Advisory may cover a line whose omission understates tax) and the REACH check
            //   (a covering question's prompt must NAME the line).
            match census_join::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask census-join: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("stop-list") => {
            // ★★★ SPEC_interview R15: the stop list as executable requirements — no `serde_json`
            //   reflection in the typed seam, no progress/remaining field, no ledger question in a
            //   return registry.
            match r15_stop_list::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask stop-list: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("wrapped-literals") => {
            // ★★★ FR-108: a string literal left on one source line with the next line's
            //   indentation typed in as real spaces prints a gap in the middle of a sentence.
            //   See `wrapped_literal_check`'s module doc for what it covers and what it does not.
            match wrapped_literal_check::run() {
                Ok(msg) => println!("{msg}"),
                Err(e) => {
                    eprintln!("xtask wrapped-literals: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("box-census") => {
            // ★ SPEC_interview R4/I2: every box an archived information return PRINTS carries exactly
            //   one recorded decision, the caption checked verbatim against design/forms/extract/.
            if let Err(e) = box_census::run() {
                eprintln!("xtask box-census: {e}");
                std::process::exit(1);
            }
        }
        Some("check-isolation") => {
            if let Err(e) = check_isolation::run() {
                eprintln!("xtask check-isolation: {e}");
                std::process::exit(1);
            }
        }
        Some("dump-fields") => {
            let Some(path) = args.get(1) else {
                eprintln!("usage: cargo run -p xtask -- dump-fields <pdf>");
                std::process::exit(2);
            };
            if let Err(e) = dump_fields::run(path) {
                eprintln!("xtask dump-fields: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!(
                "usage: cargo run -p xtask -- <docs [--pdf] | examples | subcommand-coverage | \
                 check-isolation | line-coverage | census-join | stop-list | box-census | cite-check | prompt-check | authority-conflicts | harness-check | archive-check | authority-manifest [--regen] | authority-refresh --check | extract-geometry <stem> | label-census <stem> | label-proof <stem> | label-boxes <stem> | dependents-grid <stem> | \
                 classify-path <path> | \
                 extract-schedule-1a | dump-fields <pdf> | form-delta <old> <new> | \
                 port-status <prior-tag> <new-tag>>"
            );
            std::process::exit(2);
        }
    }
}
