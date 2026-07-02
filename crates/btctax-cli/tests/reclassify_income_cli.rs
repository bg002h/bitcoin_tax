//! Binary-level CLI-parse tests for `reconcile reclassify-income --business`.
//!
//! These tests drive the real `btctax reconcile reclassify-income` subcommand via
//! `std::process::Command` to verify the I-1 fix: `--business` now requires an explicit value
//! (`true` or `false`) and is required — bare `--business` and omitting it entirely both produce
//! a clap argument error before any vault I/O occurs.
//!
//! Test precedent: `config_dispatch.rs` (same `CARGO_BIN_EXE_btctax` + `BTCTAX_PASSPHRASE` pattern).
use btctax_cli::cmd;
use btctax_store::Passphrase;
use std::path::Path;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// Import a minimal River CSV containing one Income row so that a valid income event ref exists.
fn import_river_income(vault: &Path) -> String {
    let dir = vault.parent().expect("vault must have a parent directory");
    let csv_path = dir.join("river_income.csv");
    // Minimal River universal CSV (§9.1 confirmed 8-col shape, CRLF): one Income row.
    // Date 2025-03-01 is in the bundled dataset at $84,000 → FmvStatus::PriceDataset.
    std::fs::write(
        &csv_path,
        "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
         2025-03-01 12:00:00,,,0.00100000,BTC,,,Income\r\n",
    )
    .unwrap();
    cmd::import::run(vault, &pp(), &[csv_path]).unwrap();
    // Retrieve the canonical event ref for the imported income event.
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    events
        .iter()
        .find_map(|e| match &e.payload {
            btctax_core::EventPayload::Income(_) => Some(e.id.canonical()),
            _ => None,
        })
        .expect("imported River income event must exist")
}

/// Run `btctax --vault <vault> reconcile reclassify-income <args...>` via the compiled binary.
/// Returns `(exit_code, stderr_text)`.
fn run_reclassify_income(vault: &Path, args: &[&str]) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let output = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().expect("vault path is valid UTF-8"))
        .arg("reconcile")
        .arg("reclassify-income")
        .args(args)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute successfully");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output
        .status
        .code()
        .expect("btctax process must exit normally (not via signal)");
    (code, stderr)
}

/// I-1 — explicit-value accepted: `--business true` must parse and the command must succeed.
/// Drives the real binary with a vault containing a valid Income event ref.
#[test]
fn reclassify_income_business_true_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let income_ref = import_river_income(&vault);

    let (code, stderr) = run_reclassify_income(&vault, &[&income_ref, "--business", "true"]);
    assert_eq!(
        code, 0,
        "reclassify-income <ref> --business true must exit 0; stderr: {stderr}"
    );
}

/// I-1 — bare flag rejected: `--business` without a value must produce a non-zero exit.
/// Clap emits the error before any vault I/O (vault path does not need to exist).
#[test]
fn reclassify_income_bare_business_flag_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp"); // never created — clap fails before vault access

    // `--business` with no following value: clap should refuse and exit non-zero.
    let (code, stderr) = run_reclassify_income(&vault, &["some-event-ref", "--business"]);
    assert_ne!(
        code, 0,
        "reclassify-income <ref> --business (bare, no value) must exit non-zero; stderr: {stderr}"
    );
}

/// I-1 — omitted `--business` rejected: omitting the required `--business` arg must error.
/// Clap emits the "required argument not provided" error before any vault I/O.
#[test]
fn reclassify_income_omitted_business_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp"); // never created — clap fails before vault access

    // No `--business` at all: clap should refuse (required arg missing) and exit non-zero.
    let (code, stderr) = run_reclassify_income(&vault, &["some-event-ref"]);
    assert_ne!(
        code, 0,
        "reclassify-income <ref> (no --business) must exit non-zero; stderr: {stderr}"
    );
}
