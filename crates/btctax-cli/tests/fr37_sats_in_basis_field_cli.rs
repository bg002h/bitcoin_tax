//! FR-37 (FOLLOWUPS.md, 2026-09-05 crypto basis audit): the "sats typed into a dollars field"
//! advisory (UX-P4-4(d), `reconcile.rs::sats_as_dollars_advisory`) had exactly ONE call site —
//! `--amount` on `reclassify-outflow`, where the mistake OVERSTATES tax (the safe direction).
//! `--basis` / `--donor-basis` / `--fmv-at-gift` / `--fmv` got only the `>= 0` sign check
//! (UX-P4-4a), so a sats-shaped value recorded SILENTLY — and every one of these fields is
//! basis-bearing: `--donor-basis` and `--fmv-at-gift` feed the §1015(a) dual-basis computation
//! (gain-basis / loss-basis respectively — see `btctax_core::AllocLot` doc comments), and `--fmv`
//! (on both `classify-inbound-income` and the later-correction `set-fmv`) becomes the acquired
//! lot's cost basis via `BasisSource::FmvAtIncome`. Inflating any of these UNDERSTATES a future
//! gain — the direction this project treats as worst.
//!
//! Binary-level CLI tests, same harness pattern as `classify_inbound_self_transfer_cli.rs`
//! (`std::process::Command` against the real `btctax` binary, capturing stderr).
use btctax_cli::{cmd, Session};
use btctax_core::EventPayload;
use btctax_store::Passphrase;
use std::path::Path;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// A Coinbase CSV with a single Receive of 0.05 BTC on 2025-03-01 (→ a raw `TransferIn`). The
/// bundled dataset prices that date at $84,000/BTC, so the market value of this receipt is
/// $4,200 — the SAME fixture `classify_inbound_self_transfer_cli.rs` uses for its FR-37 case.
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

/// Build a vault from `csv`, returning `(vault_path, first-TransferIn-event canonical ref)`.
fn vault_with_transfer_in(dir: &Path, csv: std::path::PathBuf) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let want_ref = events
        .iter()
        .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
        .expect("a TransferIn event must exist")
        .id
        .canonical();
    (vault, want_ref)
}

/// A minimal River universal CSV (§9.1 confirmed 8-col shape, CRLF) with one NATIVE Income row of
/// 0.001 BTC on 2025-03-01 (bundled-dataset price $84,000/BTC ⇒ market value $84). Same fixture
/// shape as `reclassify_income_cli.rs::import_river_income`. `set-fmv`'s `ManualFmv` can ONLY
/// target a raw `EventPayload::Income` event (enforced by `guard_decision_conflict` upstream) — a
/// `TransferIn` later classified as income via `classify-inbound-income` does NOT change its raw
/// payload type, so it is the WRONG fixture for `set-fmv`; a native Income import is required.
fn river_income_csv(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("river_income.csv");
    std::fs::write(
        &p,
        "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
         2025-03-01 12:00:00,,,0.00100000,BTC,,,Income\r\n",
    )
    .unwrap();
    p
}

/// Build a vault from a native-Income `csv`, returning `(vault_path, Income-event canonical ref)`.
fn vault_with_income(dir: &Path, csv: std::path::PathBuf) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let want_ref = events
        .iter()
        .find(|e| matches!(e.payload, EventPayload::Income(_)))
        .expect("a native Income event must exist")
        .id
        .canonical();
    (vault, want_ref)
}

/// Run `btctax --vault <vault> <args...>`; returns (exit_code, stderr).
fn run(vault: &Path, args: &[&str]) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let output = std::process::Command::new(bin)
        .arg("--vault")
        .arg(vault.to_str().unwrap())
        .args(args)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().expect("process exits normally");
    (code, stderr)
}

fn assert_sats_warning(stderr: &str, flag: &str) {
    assert!(
        stderr.contains("warning")
            && stderr.contains(flag)
            && stderr.to_lowercase().contains("sats"),
        "a sats-shaped {flag} must warn, naming {flag} + the sats mistake: {stderr}"
    );
}

/// `--donor-basis` (FR-37, NAMED): the donor's carried-over GAIN basis (§1015(a)) gets only the
/// `>= 0` sign check today. Typing the sats count (5,000,000) for a 0.05 BTC gift is >100x its
/// $4,200 market value at receipt — the classic mistake, now caught. (★ fault-inject: remove the
/// `--donor-basis` call site from `classify_inbound`'s advisory block and this goes RED.)
#[test]
fn classify_inbound_gift_large_donor_basis_is_flagged_as_likely_sats() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_transfer_in(dir.path(), coinbase_receive_csv(dir.path()));

    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-inbound-gift",
            &in_ref,
            "--fmv-at-gift",
            "4200",
            "--donor-basis",
            "5000000",
        ],
    );
    assert_eq!(code, 0, "an advisory is non-fatal; stderr: {stderr}");
    assert_sats_warning(&stderr, "--donor-basis");
}

/// `--fmv-at-gift` (required field, same command as `--donor-basis`): the §1015(a) LOSS-basis
/// reference. Leaving it unchecked while fixing `--donor-basis` on the SAME decision would be a
/// half-fix — an inflated `--fmv-at-gift` disables the dual-basis loss cap in favor of the (larger)
/// donor basis, which can only make an allowed loss bigger, i.e. understate tax. (★ fault-inject:
/// remove the `--fmv-at-gift` call site and this goes RED.)
#[test]
fn classify_inbound_gift_large_fmv_at_gift_is_flagged_as_likely_sats() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_transfer_in(dir.path(), coinbase_receive_csv(dir.path()));

    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-inbound-gift",
            &in_ref,
            "--fmv-at-gift",
            "5000000",
        ],
    );
    assert_eq!(code, 0, "an advisory is non-fatal; stderr: {stderr}");
    assert_sats_warning(&stderr, "--fmv-at-gift");
}

/// Plausible gift values (both near the $4,200 market value) fire neither advisory.
#[test]
fn classify_inbound_gift_plausible_values_have_no_sats_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_transfer_in(dir.path(), coinbase_receive_csv(dir.path()));

    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-inbound-gift",
            &in_ref,
            "--fmv-at-gift",
            "4200",
            "--donor-basis",
            "1000",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        !stderr.contains("--donor-basis") && !stderr.contains("--fmv-at-gift"),
        "plausible gift values must not warn: {stderr}"
    );
}

/// `--fmv` on `classify-inbound-income`: becomes the acquired lot's cost basis
/// (`BasisSource::FmvAtIncome`), so it is basis-bearing exactly like `--donor-basis`, one
/// indirection removed. (★ fault-inject: remove the `--fmv` (Income) call site and this goes RED.)
#[test]
fn classify_inbound_income_large_fmv_is_flagged_as_likely_sats() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, in_ref) = vault_with_transfer_in(dir.path(), coinbase_receive_csv(dir.path()));

    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-inbound-income",
            &in_ref,
            "--kind",
            "reward",
            "--fmv",
            "5000000",
        ],
    );
    assert_eq!(code, 0, "an advisory is non-fatal; stderr: {stderr}");
    assert_sats_warning(&stderr, "--fmv");
}

/// `set-fmv --fmv`: the later-correction path for a NATIVE Income event's FMV (`ManualFmv`, which
/// can only target an `EventPayload::Income` — enforced upstream by `guard_decision_conflict`), so
/// it shares the exact same basis-bearing risk (`BasisSource::FmvAtIncome`) as `--fmv` on
/// `classify-inbound-income`. The fixture's 0.001 BTC prices at $84 on 2025-03-01. (★ fault-inject:
/// remove the `set_fmv` call site and this goes RED.)
#[test]
fn set_fmv_large_value_is_flagged_as_likely_sats() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, income_ref) = vault_with_income(dir.path(), river_income_csv(dir.path()));

    let (code, stderr) = run(
        &vault,
        &["reconcile", "set-fmv", &income_ref, "--fmv", "5000000"],
    );
    assert_eq!(code, 0, "an advisory is non-fatal; stderr: {stderr}");
    assert_sats_warning(&stderr, "--fmv");
}
