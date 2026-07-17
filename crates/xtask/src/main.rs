//! xtask — developer tooling for the btctax workspace.
//!
//! `cargo run -p xtask -- docs` regenerates the committed man pages under `docs/man/`.
//! `cargo run -p xtask -- docs --pdf` additionally renders `docs/pdf/*.pdf` (requires `groff`).
//! `cargo run -p xtask -- check-isolation` asserts no HTTP client in the tax crates (#41 Part C).
//! `cargo run -p xtask -- dump-fields <pdf>` lists a PDF's AcroForm field names (map authoring).

mod check_isolation;
mod docs;
mod dump_fields;
mod examples;

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
                "usage: cargo run -p xtask -- <docs [--pdf] | examples | check-isolation | dump-fields <pdf>>"
            );
            std::process::exit(2);
        }
    }
}
