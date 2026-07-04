//! xtask — developer tooling for the btctax workspace.
//!
//! `cargo run -p xtask -- docs` regenerates the committed man pages under `docs/man/`.
//! (PDF generation is wired in by Task 4.)

mod docs;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("docs") => {
            if let Err(e) = docs::write_man_pages() {
                eprintln!("xtask docs: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: cargo run -p xtask -- docs");
            std::process::exit(2);
        }
    }
}
