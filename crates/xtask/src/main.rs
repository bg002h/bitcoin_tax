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
mod check_isolation;
mod cite_check;
mod docs;
mod dump_fields;
mod examples;
mod form_geometry;
mod harness_check;
mod label_reader;

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
                 check-isolation | cite-check | authority-conflicts | harness-check | archive-check | authority-manifest [--regen] | extract-geometry <stem> | label-census <stem> | label-proof <stem> | label-boxes <stem> | \
                 classify-path <path> | \
                 extract-schedule-1a | dump-fields <pdf>>"
            );
            std::process::exit(2);
        }
    }
}
