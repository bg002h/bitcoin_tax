//! `cargo run -p xtask -- check-isolation` — [R0-r2 M-B] the network-isolation gate.
//!
//! Asserts that NO HTTP client (`ureq`) nor its TLS stack (`rustls`) appears in the NORMAL dependency
//! tree of ANY tax crate (btctax-cli / -tui / -tui-edit / -core / -adapters) — the tax binaries must
//! stay offline/deterministic/private — AND that it IS present in `btctax-update-prices` (so the check
//! is never vacuous). Runs `cargo tree` (a hermetic, offline metadata read of the locked graph), so this
//! lives as an xtask/CI step, NOT a non-hermetic `#[test]`.
use std::process::Command;

/// The tax crates that MUST carry no network dependency.
const TAX_CRATES: &[&str] = &[
    "btctax-cli",
    "btctax-tui",
    "btctax-tui-edit",
    "btctax-core",
    "btctax-adapters",
];
/// Crate names whose presence in a tax crate's tree is a violation (the HTTP client + its TLS stack).
const FORBIDDEN: &[&str] = &["ureq", "rustls"];

pub fn run() -> Result<(), String> {
    for pkg in TAX_CRATES {
        let tree = cargo_tree(pkg)?;
        for dep in FORBIDDEN {
            if tree_has_crate(&tree, dep) {
                return Err(format!(
                    "ISOLATION VIOLATION: `{dep}` is in the normal dependency tree of tax crate \
                     `{pkg}`. The tax binaries must link NO HTTP client — network code belongs ONLY in \
                     btctax-update-prices (#41 Part C)."
                ));
            }
        }
    }
    // Positive control: the updater DOES link ureq, else the whole check is vacuous.
    let updater = cargo_tree("btctax-update-prices")?;
    if !tree_has_crate(&updater, "ureq") {
        return Err(
            "btctax-update-prices no longer links `ureq` — the isolation check would be vacuous; \
             restore the HTTP client or update this gate."
                .into(),
        );
    }
    println!(
        "cargo-tree isolation OK: no {FORBIDDEN:?} in {TAX_CRATES:?}; ureq present in \
         btctax-update-prices."
    );
    Ok(())
}

/// `cargo tree -p <pkg> -e normal --prefix none` — normal (non-dev, non-build) edges of the locked graph.
fn cargo_tree(pkg: &str) -> Result<String, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args(["tree", "-p", pkg, "-e", "normal", "--prefix", "none"])
        .output()
        .map_err(|e| format!("running `cargo tree -p {pkg}`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`cargo tree -p {pkg}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// True when `dep` appears as a crate node in `tree`. With `--prefix none`, each line begins with the
/// crate NAME (e.g. `ureq v2.10.1`); match the first whitespace-delimited token exactly.
fn tree_has_crate(tree: &str, dep: &str) -> bool {
    tree.lines()
        .any(|line| line.split_whitespace().next() == Some(dep))
}
