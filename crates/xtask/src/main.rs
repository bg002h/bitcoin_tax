//! xtask — developer tooling for the btctax workspace.
//!
//! `cargo run -p xtask -- docs` regenerates the committed man pages under `docs/man/`.
//! `cargo run -p xtask -- docs --pdf` additionally renders `docs/pdf/*.pdf` (requires `groff`).
//! `cargo run -p xtask -- check-isolation` asserts no HTTP client in the tax crates (#41 Part C).

mod check_isolation;
mod docs;

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
        Some("check-isolation") => {
            if let Err(e) = check_isolation::run() {
                eprintln!("xtask check-isolation: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: cargo run -p xtask -- <docs [--pdf] | check-isolation>");
            std::process::exit(2);
        }
    }
}
