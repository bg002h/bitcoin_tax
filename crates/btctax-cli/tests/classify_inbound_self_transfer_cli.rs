//! Binary-level CLI tests for `reconcile classify-inbound-self-transfer` (Cycle A, Task 2).
//!
//! Drives the real `btctax reconcile classify-inbound-self-transfer` subcommand via
//! `std::process::Command` to verify the dispatch builds `InboundClass::SelfTransferMine` and calls
//! the UNCHANGED `cmd::reconcile::classify_inbound`: defaults ($0 basis + receipt-date HP + honest
//! advisory), the `--basis`/`--acquired` overrides (no advisory, adjustable HP), and the wrong-target
//! (non-TransferIn) bad-target path.
//!
//! Test precedent: `reclassify_income_cli.rs` (same `CARGO_BIN_EXE_btctax` + `BTCTAX_PASSPHRASE` pattern).
use btctax_cli::{cmd, Session};
use btctax_core::{BlockerKind, EventPayload};
use btctax_store::Passphrase;
use std::path::Path;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// A Coinbase CSV with a single Receive (→ a raw `TransferIn`, the self-transfer receiving side).
fn coinbase_receive_csv(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("cb_recv.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n",
    )
    .unwrap();
    p
}

/// A Coinbase CSV with a single Buy (→ an `Acquire`, a NON-TransferIn event for the bad-target case).
fn coinbase_buy_csv(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("cb_buy.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Build a vault from `csv`, returning `(vault_path, first-event-of-`kind` canonical ref)`.
fn vault_with(
    dir: &Path,
    csv: std::path::PathBuf,
    want_transfer_in: bool,
) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let want_ref = events
        .iter()
        .find(|e| {
            if want_transfer_in {
                matches!(e.payload, EventPayload::TransferIn(_))
            } else {
                matches!(e.payload, EventPayload::Acquire(_))
            }
        })
        .expect("target event must exist")
        .id
        .canonical();
    (vault, want_ref)
}

/// Run `btctax --vault <vault> reconcile classify-inbound-self-transfer <args...>`; returns (exit, stderr).
fn run_self_transfer(vault: &Path, args: &[&str]) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let output = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().unwrap())
        .arg("reconcile")
        .arg("classify-inbound-self-transfer")
        .args(args)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().expect("process exits normally");
    (code, stderr)
}

/// Defaults: no `--basis` / `--acquired` → creates a $0-basis lot, clears `UnknownBasisInbound`, and
/// fires the honest `SelfTransferInboundZeroBasis` advisory.
#[test]
fn classify_inbound_self_transfer_defaults_creates_lot_and_fires_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with(dir.path(), coinbase_receive_csv(dir.path()), true);

    let (code, stderr) = run_self_transfer(&vault, &[&in_ref]);
    assert_eq!(code, 0, "defaults must exit 0; stderr: {stderr}");

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.lots.len(), 1, "a non-taxable lot is created");
    assert_eq!(state.lots[0].usd_basis, rust_decimal_macros::dec!(0));
    assert!(
        !state.lots[0].basis_pending,
        "$0 basis is computable — never pending"
    );
    assert!(state.income_recognized.is_empty(), "non-taxable: no income");
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::UnknownBasisInbound),
        "UnknownBasisInbound must be cleared"
    );
    assert!(
        state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SelfTransferInboundZeroBasis),
        "the zero-basis advisory must fire when --basis is omitted"
    );
}

/// `--basis` + `--acquired`: real cost + old date → basis set, NO advisory, and the supplied
/// acquisition date rides onto the lot (a 2015 date → long-term while landing in the 2025 wallet pool).
#[test]
fn classify_inbound_self_transfer_with_basis_and_acquired_has_no_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with(dir.path(), coinbase_receive_csv(dir.path()), true);

    let (code, stderr) = run_self_transfer(
        &vault,
        &[&in_ref, "--basis", "1234.56", "--acquired", "2015-01-02"],
    );
    assert_eq!(code, 0, "with overrides must exit 0; stderr: {stderr}");

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.lots.len(), 1);
    assert_eq!(state.lots[0].usd_basis, rust_decimal_macros::dec!(1234.56));
    assert_eq!(
        state.lots[0].acquired_at,
        time::macros::date!(2015 - 01 - 02)
    );
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::SelfTransferInboundZeroBasis),
        "a supplied basis must NOT fire the advisory"
    );
}

/// Wrong target: pointing the subcommand at a NON-TransferIn event (an Acquire) surfaces the existing
/// bad-target path — a `DecisionConflict` blocker (variant-agnostic; the CLI only appends).
#[test]
fn classify_inbound_self_transfer_wrong_target_is_decision_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, acquire_ref) = vault_with(dir.path(), coinbase_buy_csv(dir.path()), false);

    let (code, stderr) = run_self_transfer(&vault, &[&acquire_ref]);
    // The CLI append itself succeeds (exit 0) — the engine adjudicates the bad target in projection.
    assert_eq!(code, 0, "append succeeds; stderr: {stderr}");

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(
        state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict),
        "classifying a non-TransferIn event must raise DecisionConflict (bad-target path)"
    );
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::SelfTransferInboundZeroBasis),
        "no lot / no advisory for an excluded bad-target decision"
    );
}
