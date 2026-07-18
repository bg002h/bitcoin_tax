//! `xtask examples` — the CLI verbatim-I/O examples generator (SPEC §7, Artifact 1).
//!
//! Runs the freshly-built `btctax` binary against synthetic vaults under a pinned, deterministic
//! environment (SPEC §3.3) and captures `$ cmd` + verbatim stdout (+ exit code, + labelled stderr where
//! relevant) into a single whole-file Markdown golden. The golden is a pure function of
//! `(repo tree, binary, synthetic inputs)`; a `regen == committed` test (examples_golden.rs) gates it.
//!
//! The generator NEVER runs a stale binary: `built_btctax()` compiles `btctax` via a nested `cargo build`
//! (I3 — `CARGO_BIN_EXE_btctax` is not set for xtask), so freshness holds by construction.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// SPEC §3.3 pinned environment (minus `BTCTAX_NOW`/`HOME`, which are set per invocation).
const PINNED_ENV: &[(&str, &str)] = &[("TZ", "UTC"), ("LC_ALL", "C"), ("LANG", "C")];
/// The synthetic passphrase every captured journey uses (front-matter discloses the interactive prompt
/// a real user would see instead).
const PASSPHRASE: &str = "pw";

/// The workspace root (two levels up from `crates/xtask`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root above crates/xtask")
        .to_path_buf()
}

/// Compile `btctax` and return the path to the fresh debug binary. Honors `CARGO_TARGET_DIR`.
pub fn built_btctax() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let ws = workspace_root();
    let status = Command::new(&cargo)
        .current_dir(&ws)
        .args(["build", "-p", "btctax-cli", "--bin", "btctax"])
        .status()
        .expect("spawn `cargo build -p btctax-cli`");
    assert!(status.success(), "cargo build -p btctax-cli --bin btctax failed");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| ws.join("target"));
    target.join("debug").join("btctax")
}

/// The `btctax-cli` crate version, read from its `Cargo.toml` (the binary has no `--version`; SPEC §7).
/// A release bump reds `regen == committed` until the golden is regenerated.
fn btctax_cli_version() -> String {
    let toml = std::fs::read_to_string(workspace_root().join("crates/btctax-cli/Cargo.toml"))
        .expect("read btctax-cli/Cargo.toml");
    for line in toml.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("version") {
            if let Some(v) = rest.split('"').nth(1) {
                return v.to_string();
            }
        }
    }
    panic!("no version in btctax-cli/Cargo.toml");
}

/// One captured command in a journey. `display` is what the reader sees after `$ `; `args` is what the
/// real binary runs (display is bare `btctax …`, execution is the resolved binary — display-faithful,
/// execution-pinned). `now` pins `BTCTAX_NOW`; `show_stderr` adds a labelled `stderr:` block.
struct Cmd<'a> {
    args: &'a [&'a str],
    now: Option<&'a str>,
    show_stderr: bool,
}

/// Run one command in `cwd` under the pinned environment; return `(stdout, stderr, exit_code)`.
fn capture(bin: &Path, cwd: &Path, cmd: &Cmd) -> (String, String, i32) {
    let mut c = Command::new(bin);
    c.current_dir(cwd)
        .env("BTCTAX_PASSPHRASE", PASSPHRASE)
        .env("BTCTAX_PRICE_CACHE", cwd.join("no-such-price-cache.csv"))
        .env("HOME", cwd);
    for (k, v) in PINNED_ENV {
        c.env(k, v);
    }
    if let Some(now) = cmd.now {
        c.env("BTCTAX_NOW", now);
    }
    c.args(cmd.args);
    let out = c.output().expect("spawn btctax");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// Emit one `$ btctax …` block (command + verbatim stdout + exit-code marker, + labelled stderr when
/// `show_stderr`) into `md`.
fn emit(md: &mut String, bin: &Path, cwd: &Path, cmd: &Cmd) {
    let shown = format!("btctax {}", cmd.args.join(" "));
    let (stdout, stderr, code) = capture(bin, cwd, cmd);
    md.push_str("```console\n");
    let _ = writeln!(md, "$ {shown}");
    md.push_str(&stdout);
    if !stdout.ends_with('\n') && !stdout.is_empty() {
        md.push('\n');
    }
    if code != 0 {
        let _ = writeln!(md, "[exit {code}]");
    }
    md.push_str("```\n");
    if cmd.show_stderr && !stderr.is_empty() {
        md.push_str("\nstderr:\n```console\n");
        md.push_str(&stderr);
        if !stderr.ends_with('\n') {
            md.push('\n');
        }
        md.push_str("```\n");
    }
}

/// The front matter: the pinned-env convention + the honest passphrase sentence + the version pin.
fn front_matter(md: &mut String) {
    let _ = writeln!(md, "---");
    let _ = writeln!(md, "title: btctax — worked examples");
    let _ = writeln!(md, "btctax-version: {}", btctax_cli_version());
    let _ = writeln!(md, "---");
    md.push('\n');
    md.push_str(
        "<!-- GENERATED by `cargo run -p xtask -- examples`; do NOT edit by hand. A `regen == committed`\n\
         test gates this file; CI re-diffs it. Every block below is the real `btctax` binary run against\n\
         synthetic vaults. -->\n\n\
         All examples run under a pinned, deterministic environment: `BTCTAX_PASSPHRASE=pw`, `TZ=UTC`,\n\
         `LC_ALL=C`, a nonexistent `BTCTAX_PRICE_CACHE`, a scrubbed `HOME`, and (where a decision is\n\
         recorded) a fixed `BTCTAX_NOW`. A real user is prompted for the passphrase interactively rather\n\
         than passing `BTCTAX_PASSPHRASE`.\n\n",
    );
}

/// Write a committed synthetic corpus into `cwd/name` (so a journey's `import` sees a real file).
fn write_corpus(cwd: &Path, name: &str, content: &str) {
    std::fs::write(cwd.join(name), content).expect("write corpus");
}

/// A no-`now` non-stderr command (the common case).
fn plain<'a>(args: &'a [&'a str]) -> Cmd<'a> {
    Cmd { args, now: None, show_stderr: false }
}

// ── Synthetic corpora (embedded as consts with explicit CRLF — committed .csv files are force-LF'd by
//    .gitattributes and would break the Coinbase parser, so we follow the fixtures.rs pattern) ─────
const J1_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.02000000,USD,67500.00,1350.00,1340.00,10.00,,,\r\n";

/// Generate the whole-file golden by running `bin` across every journey. Pure function of
/// `(repo tree, binary, synthetic inputs)`.
pub fn generate(bin: &Path) -> String {
    let mut md = String::new();
    front_matter(&mut md);

    md.push_str("## btctax at a glance\n\nThe top-level command surface:\n\n");
    let dir = tempfile::tempdir().expect("tempdir");
    emit(&mut md, bin, dir.path(), &plain(&["--help"]));

    journey_j1(&mut md, bin);

    md
}

/// J1 — single-buyer happy path: init → import → verify → set a tax profile → report → export.
fn journey_j1(md: &mut String, bin: &Path) {
    md.push_str(
        "\n## J1 — a single buyer, start to finish\n\n\
         Alice buys 0.1 BTC, sells 0.02, and wants her 2025 numbers. Create an encrypted vault (a key\n\
         backup is mandatory), import the exchange CSV, and check the ledger balances:\n\n",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = dir.path();
    write_corpus(cwd, "coinbase.csv", J1_CSV);
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "init", "--key-backup", "key-backup.asc"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "import", "coinbase.csv"]));
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "verify"]));
    md.push_str(
        "\nThe year's tax is *not computable* until a tax profile is set (btctax refuses to guess your\n\
         bracket). Set one, then the report computes:\n\n",
    );
    emit(
        md, bin, cwd,
        &plain(&[
            "--vault", "v.pgp", "tax-profile", "--year", "2025", "--filing-status", "single",
            "--ordinary-taxable-income", "100000", "--magi-excluding-crypto", "100000",
            "--qualified-dividends", "0",
        ]),
    );
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "report", "--tax-year", "2025"]));
    md.push_str("\nExport the reconciled snapshot (CSVs + a decrypted SQLite) and fill the IRS forms:\n\n");
    emit(md, bin, cwd, &plain(&["--vault", "v.pgp", "export-snapshot", "--out", "snapshot", "--tax-year", "2025"]));
    emit(
        md, bin, cwd,
        &Cmd {
            args: &["--vault", "v.pgp", "export-irs-pdf", "--out", "irs", "--tax-year", "2025", "--forms", "f8949,schedule-d"],
            now: None,
            show_stderr: true, // the NOT-AUTHORISED notice + 1099-DA caveat are on stderr and matter
        },
    );
}

/// Regenerate the committed golden to stdout (`cargo run -p xtask -- examples`).
pub fn run() {
    let bin = built_btctax();
    print!("{}", generate(&bin));
}

#[cfg(test)]
#[cfg(unix)] // journey stdout can embed joined paths; byte-exact goldens are gated on unix (I4)
mod tests {
    use super::*;

    #[test]
    fn generate_is_deterministic_and_captures_help() {
        let bin = built_btctax();
        let a = generate(&bin);
        let b = generate(&bin);
        assert_eq!(a, b, "generate() must be byte-deterministic");
        assert!(a.contains("$ btctax --help"), "must show the verbatim command");
        assert!(a.contains("Usage: btctax"), "must capture the real help output");
        assert!(a.contains(&format!("btctax-version: {}", btctax_cli_version())), "front-matter version");
    }
}
