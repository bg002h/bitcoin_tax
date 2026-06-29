//! Binary-level FR9 exit-code regression tests.
//!
//! These tests assert the *process* exit code of the `btctax verify` subcommand by running the
//! compiled binary through `std::process::Command`. That is the gap the lib-level tests in
//! `verify_report.rs` do not cover: they assert `report.has_hard_blockers()` but do not observe
//! the actual exit code the OS receives. If the mapping
//!
//!   `if report.has_hard_blockers() { return Ok(ExitCode::from(1)) }`
//!
//! in `main.rs` were removed, `fr9_hard_blocker_exits_one` would fail (observed exit 0 ≠ 1).
use btctax_cli::cmd;
use btctax_store::Passphrase;
use std::path::Path;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// A single-row BTC Buy — no pending reconciliation, no unclassified inbound → 0 hard blockers.
fn write_clean_csv(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("fr9_clean.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
fr9-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n",
    )
    .unwrap();
    p
}

/// A BTC Buy + an unclassified Receive with no ClassifyInbound decision → hard `UnknownBasisInbound`
/// blocker (§7.3). Without a decision event to resolve the inbound, `project()` emits the hard
/// blocker and `has_hard_blockers()` returns true → binary must exit 1.
fn write_blocker_csv(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("fr9_blocker.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
fr9-buy2,2025-04-01 12:00:00 UTC,Buy,BTC,0.05000000,USD,85000.00,4250.00,4265.00,15.00,,,\r\n\
fr9-recv,2025-05-01 12:00:00 UTC,Receive,BTC,0.02000000,USD,90000.00,,,,,,\r\n",
    )
    .unwrap();
    p
}

/// Run `btctax --vault <vault> verify` via the compiled binary and return the process exit code.
///
/// `CARGO_BIN_EXE_btctax` is set by Cargo for integration-test binaries; it is the path to the
/// freshly built `btctax` executable. `BTCTAX_PASSPHRASE` is the env seam from `main.rs`.
fn run_verify(vault: &Path) -> i32 {
    let bin = env!("CARGO_BIN_EXE_btctax");
    std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().expect("vault path is valid UTF-8"),
            "verify",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        // Suppress binary stdout/stderr in test output unless the test fails.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("btctax binary must execute successfully")
        .code()
        .expect("btctax process must exit normally (not via signal)")
}

/// FR9 clean path: a vault with only a BTC Buy (no unclassified inbound) must exit 0.
///
/// This confirms that a user with a reconciled vault does not receive a spurious non-zero exit.
#[test]
fn fr9_clean_vault_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_clean_csv(dir.path())]).unwrap();

    let code = run_verify(&vault);
    assert_eq!(
        code, 0,
        "btctax verify must exit 0 when there are no hard blockers"
    );
}

/// FR9 hard-blocker path: a vault with an unclassified Receive (no ClassifyInbound decision)
/// produces a hard `UnknownBasisInbound` blocker → `btctax verify` must exit 1.
///
/// This test FAILS if the `if report.has_hard_blockers() { return Ok(ExitCode::from(1)) }` mapping
/// in `main.rs` is removed — it would observe exit 0 instead of the required exit 1.
#[test]
fn fr9_hard_blocker_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_blocker_csv(dir.path())]).unwrap();

    let code = run_verify(&vault);
    assert_eq!(
        code, 1,
        "btctax verify must exit 1 when has_hard_blockers() is true (FR9 process contract)"
    );
}
