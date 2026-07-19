//! UX-P4-4(a) I3 — binary-level WIRING KATs for the record-time value guards.
//!
//! Each sign guard is applied at a dispatch call site in `main.rs` (guard-per-flag, never in the
//! shared parser). A helper-level unit test cannot witness the WIRING: reverting a call site to the
//! unguarded `parse_usd_arg` / `parse_sell_arg` would leave the helper's own unit test green. These
//! tests drive the REAL binary with the `=`-form negative (which slips past clap's `-`-prefix
//! detection, the exact bypass the guards close) and assert the refusal NAMES the flag + the rule —
//! so reverting ANY one call site reds its row. Also covers the spec/plan-mandated cases the earlier
//! work left open: the `--sell=-1` `=`-form KAT (SPEC §3.3 acceptance) and the ad-hoc trio (PLAN
//! Step-1d: `--carryforward-in` refused < 0; `--income` / `--magi` negative ACCEPTED — a negative
//! AGI/MAGI is legitimate in an NOL year, so a blanket refuse would be a §1 false-refuse).
//!
//! PRIVACY: synthetic fixtures in tempdirs; no user file is read. Read-only refusals persist nothing.
use btctax_cli::cmd;
use btctax_store::Passphrase;
use std::path::{Path, PathBuf};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

const BIN: &str = env!("CARGO_BIN_EXE_btctax");

fn init_vault(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    vault
}

/// A vault holding 2 BTC in `exchange:coinbase:default` (two 1-BTC buys), for the accept-path KATs
/// that must actually reach the marginal computation.
fn lot_vault(dir: &Path) -> PathBuf {
    let csv = dir.join("buys.csv");
    std::fs::write(
        &csv,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
b1,2024-01-15 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
b2,2024-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,40000.00,40000.00,40000.00,0.00,,,\r\n",
    )
    .unwrap();
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    vault
}

/// Drive `btctax --vault <vault> <args...>`; return (exit_code, stderr).
fn run(vault: &Path, args: &[&str]) -> (i32, String) {
    let out = std::process::Command::new(BIN)
        .arg("--vault")
        .arg(vault.to_str().unwrap())
        .args(args)
        .env("BTCTAX_PASSPHRASE", "pw")
        .output()
        .expect("btctax binary must execute");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Assert `args` is REFUSED (non-zero exit) with a message naming `flag` AND `rule`.
fn assert_refused(vault: &Path, args: &[&str], flag: &str, rule: &str) {
    let (code, stderr) = run(vault, args);
    assert_ne!(code, 0, "{args:?} must be refused; stderr: {stderr}");
    assert!(
        stderr.contains(flag) && stderr.contains(rule),
        "the refusal for {args:?} must name {flag:?} + {rule:?}; stderr: {stderr}"
    );
}

/// Every refuse-<0 money flag across the reconcile / what-if / optimize dispatch surfaces refuses its
/// `=`-form negative with the flag-named message. Reverting any single call site to the unguarded
/// parser reds exactly its row (the message stops naming the flag).
#[test]
fn record_time_value_guards_are_wired_across_the_dispatch_surface() {
    let dir = tempfile::tempdir().unwrap();
    let v = init_vault(dir.path());

    // reconcile family (guard fires during dispatch, before the dummy ref is resolved) — refuse < 0.
    assert_refused(
        &v,
        &[
            "reconcile",
            "classify-inbound-income",
            "ref",
            "--kind",
            "reward",
            "--fmv=-1",
        ],
        "--fmv",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "reconcile",
            "classify-inbound-gift",
            "ref",
            "--fmv-at-gift=-1",
        ],
        "--fmv-at-gift",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "reconcile",
            "classify-inbound-gift",
            "ref",
            "--fmv-at-gift",
            "100",
            "--donor-basis=-1",
        ],
        "--donor-basis",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "reconcile",
            "classify-inbound-self-transfer",
            "ref",
            "--basis=-1",
        ],
        "--basis",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "reconcile",
            "reclassify-outflow",
            "ref",
            "--as-kind",
            "sell",
            "--amount=-1",
        ],
        "--amount",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "reconcile",
            "reclassify-outflow",
            "ref",
            "--as-kind",
            "sell",
            "--amount",
            "100",
            "--fee=-1",
        ],
        "--fee",
        ">= 0",
    );
    assert_refused(
        &v,
        &["reconcile", "set-fmv", "ref", "--fmv=-1"],
        "--fmv",
        ">= 0",
    );

    // what-if / optimize — `--sell` refuses <= 0 (a fictional-loss guard); the rest refuse < 0. The
    // guard fires during dispatch, before the (absent) pool is consulted, so an empty vault suffices.
    assert_refused(
        &v,
        &["what-if", "sell", "--sell=-1", "--wallet", "self:cold"],
        "--sell",
        "> 0",
    );
    assert_refused(
        &v,
        &["optimize", "consult", "--sell=-1", "--wallet", "self:cold"],
        "--sell",
        "> 0",
    );
    assert_refused(
        &v,
        &[
            "what-if",
            "sell",
            "--sell=1000",
            "--wallet",
            "self:cold",
            "--price=-1",
        ],
        "--price",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "optimize",
            "consult",
            "--sell=1000",
            "--wallet",
            "self:cold",
            "--proceeds=-1",
        ],
        "--proceeds",
        ">= 0",
    );
    assert_refused(
        &v,
        &[
            "what-if",
            "sell",
            "--sell=1000",
            "--wallet",
            "self:cold",
            "--filing-status",
            "single",
            "--income",
            "50000",
            "--carryforward-in=-1",
        ],
        "--carryforward-in",
        ">= 0",
    );
}

/// PLAN Step-1d ad-hoc trio ACCEPT side: a negative `--income` / `--magi` is a legitimate NOL-year
/// AGI/MAGI and must FLOW INTO the marginal computation, not be refused (a blanket sign guard here
/// would be a §1 false-refuse). Driven against a real 2-BTC pool so the what-if actually computes.
#[test]
fn adhoc_negative_income_and_magi_are_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let v = lot_vault(dir.path());

    // negative --income accepted (and the plan renders → exit 0).
    let (code, stderr) = run(
        &v,
        &[
            "what-if",
            "sell",
            "--sell=10000000",
            "--wallet",
            "exchange:coinbase:default",
            "--at",
            "2025-06-01",
            "--filing-status",
            "single",
            "--income=-5000",
        ],
    );
    assert_eq!(
        code, 0,
        "negative --income must be accepted + computed; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("--income must be"),
        "negative --income must NOT be sign-refused: {stderr}"
    );

    // negative --magi accepted (with a valid --income).
    let (code, stderr) = run(
        &v,
        &[
            "what-if",
            "sell",
            "--sell=10000000",
            "--wallet",
            "exchange:coinbase:default",
            "--at",
            "2025-06-01",
            "--filing-status",
            "single",
            "--income",
            "50000",
            "--magi=-5000",
        ],
    );
    assert_eq!(
        code, 0,
        "negative --magi must be accepted + computed; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("--magi must be"),
        "negative --magi must NOT be sign-refused: {stderr}"
    );
}
