//! FR-38, the DOORS: binary-level proof that each surface which can carry a structurally
//! impossible USD/sat value is actually refused — end to end, through the real `btctax` binary.
//!
//! The guard itself lives at `btctax_core::persistence::insert` (the workspace's sole
//! `INSERT INTO events`) and its seam tests are `btctax-core/tests/fr38_payload_polarity.rs`.
//! These are deliberately SEPARATE and deliberately per-door, because a unit test on the predicate
//! would pass while three of the four doors still bypassed it — which is exactly the FR-38 shape.
//!
//! Doors covered here:
//!  * `reconcile classify-raw --payload-json` — the door FR-38 was filed against.
//!  * `import` of a Coinbase CSV whose `Subtotal` uses the accounting-parentheses negative
//!    `(1,234.56)`. `btctax_adapters::parse::parse_usd` *deliberately* preserves that sign, and
//!    Coinbase (unlike Gemini) never `.abs()`es it — so a negative `usd_cost` reaches the payload.
//!    This is the largest door and the one FR-38 as filed did not mention.
//!  * `accept-conflict` reaches the seam only through the `ImportConflict` row written by a prior
//!    import; the import that would have written an impossible one is refused above.
//!
//! ★ B1 kill-test: delete the `check_payload_polarity(&ev.payload)?` line at the top of
//! `persistence::insert` and every `*_is_refused` test in this file goes RED.
use btctax_store::Passphrase;
use std::path::{Path, PathBuf};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
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
    (
        output.status.code().expect("process exits normally"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// A Coinbase CSV with one row of `ttype`, whose `Subtotal` cell is written verbatim as `subtotal`.
fn coinbase_csv(dir: &Path, name: &str, ttype: &str, subtotal: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(
        &p,
        format!(
            "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-1,2025-03-01 12:00:00 UTC,{ttype},BTC,0.05000000,USD,84000.00,\"{subtotal}\",,,,,\r\n"
        ),
    )
    .unwrap();
    p
}

/// A vault holding one `Unclassified` row (a Coinbase row of an unrecognised type), returning its
/// canonical event ref — the only legal `classify-raw` target.
fn vault_with_unclassified(dir: &Path) -> (PathBuf, String) {
    let vault = dir.join("vault.pgp");
    btctax_cli::cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let csv = coinbase_csv(dir, "cb_raw.csv", "Learning Reward", "42.00");
    btctax_cli::cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    let want = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .find(|e| matches!(e.payload, btctax_core::EventPayload::Unclassified(_)))
        .expect("an Unclassified event must exist")
        .id
        .canonical();
    (vault, want)
}

fn events_in(vault: &Path) -> Vec<btctax_core::LedgerEvent> {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    btctax_core::persistence::load_all(s.conn()).unwrap()
}

fn assert_refused(code: i32, stderr: &str, field: &str) {
    assert_ne!(code, 0, "an impossible value must be a REFUSAL: {stderr}");
    assert!(
        stderr.contains("refused") && stderr.contains(field),
        "the refusal must say it refused and name {field}: {stderr}"
    );
}

// ── D2: the CLI `classify-raw` JSON door — FR-38 as filed ────────────────────────────────────

#[test]
fn classify_raw_negative_usd_cost_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_unclassified(dir.path());
    let before = events_in(&vault).len();

    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-raw",
            &target,
            "--payload-json",
            r#"{"Acquire":{"sat":2000000,"usd_cost":"-1680.00","fee_usd":"5.00","basis_source":"ExchangeProvided"}}"#,
        ],
    );
    assert_refused(code, &stderr, "usd_cost");
    assert_eq!(
        events_in(&vault).len(),
        before,
        "a refused classify-raw must append nothing"
    );
}

#[test]
fn classify_raw_negative_fee_usd_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_unclassified(dir.path());
    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-raw",
            &target,
            "--payload-json",
            r#"{"Acquire":{"sat":2000000,"usd_cost":"1680.00","fee_usd":"-5.00","basis_source":"ExchangeProvided"}}"#,
        ],
    );
    assert_refused(code, &stderr, "fee_usd");
}

/// ★ Wider than FR-38 as filed: `sat` was unguarded on this door too (the CLI does a bare
/// `serde_json` parse, and the TUI a bare `t.parse::<i64>()`), and nothing in core refused a
/// negative `sat` on a payload. A negative `sat` folds no lot but is still fabricated testimony on
/// the ledger, so it travels with the same fix at the same seam.
#[test]
fn classify_raw_negative_sat_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_unclassified(dir.path());
    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-raw",
            &target,
            "--payload-json",
            r#"{"Acquire":{"sat":-2000000,"usd_cost":"1680.00","fee_usd":"5.00","basis_source":"ExchangeProvided"}}"#,
        ],
    );
    assert_refused(code, &stderr, "sat");
}

#[test]
fn classify_raw_negative_income_fmv_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_unclassified(dir.path());
    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-raw",
            &target,
            "--payload-json",
            r#"{"Income":{"sat":100000,"usd_fmv":"-84.00","fmv_status":"ManualEntry","kind":"Reward","business":false}}"#,
        ],
    );
    assert_refused(code, &stderr, "usd_fmv");
}

/// The not-over-refusing control: the SAME command with a legitimate payload still records, so the
/// guard is discriminating rather than merely failing. Zero is legal too (§0 basis is this app's
/// conservative default).
#[test]
fn classify_raw_zero_and_positive_usd_still_record() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, target) = vault_with_unclassified(dir.path());
    let before = events_in(&vault).len();
    let (code, stderr) = run(
        &vault,
        &[
            "reconcile",
            "classify-raw",
            &target,
            "--payload-json",
            r#"{"Acquire":{"sat":2000000,"usd_cost":"0","fee_usd":"0","basis_source":"ExchangeProvided"}}"#,
        ],
    );
    assert_eq!(code, 0, "a $0 basis is legitimate: {stderr}");
    assert_eq!(
        events_in(&vault).len(),
        before + 1,
        "the ClassifyRaw decision must have been appended"
    );
}

// ── D1: the CSV import door — the biggest one, and the one FR-38 did not name ────────────────

/// Coinbase writes accounting negatives as `(1,234.56)`, `parse_usd` preserves the sign on purpose,
/// and — unlike Gemini — the Coinbase adapter never `.abs()`es `usd_cost`. Before this fix the
/// negative basis landed in the vault silently. An inflated/negative basis moves gain the wrong
/// way, which is the direction this project treats as worst.
#[test]
fn import_of_a_coinbase_csv_with_an_accounting_negative_subtotal_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    btctax_cli::cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let csv = coinbase_csv(dir.path(), "cb_neg.csv", "Buy", "(1,234.56)");

    let (code, stderr) = run(&vault, &["import", csv.to_str().unwrap()]);
    assert_refused(code, &stderr, "usd_cost");
    assert!(
        events_in(&vault).is_empty(),
        "append_import_batch is atomic — a refused row rolls the WHOLE batch back"
    );
}

/// The atomicity claim, exercised: a good row and a bad row in one file leave NOTHING behind, so a
/// filer never ends up with a silently half-imported statement.
#[test]
fn one_impossible_row_rolls_the_whole_import_back() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    btctax_cli::cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let p = dir.path().join("cb_mixed.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-ok,2025-03-01 12:00:00 UTC,Buy,BTC,0.05000000,USD,84000.00,\"4200.00\",,,,,\r\n\
cb-bad,2025-03-02 12:00:00 UTC,Buy,BTC,0.05000000,USD,84000.00,\"(1,234.56)\",,,,,\r\n",
    )
    .unwrap();

    let (code, stderr) = run(&vault, &["import", p.to_str().unwrap()]);
    assert_refused(code, &stderr, "usd_cost");
    assert!(
        events_in(&vault).is_empty(),
        "the GOOD row must not survive a batch containing an impossible one: {stderr}"
    );
}

/// The import control: an ordinary Coinbase buy is unaffected — this is a record-time gate, not a
/// change of import behaviour.
#[test]
fn an_ordinary_coinbase_import_still_records() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    btctax_cli::cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let csv = coinbase_csv(dir.path(), "cb_ok.csv", "Buy", "4200.00");
    let (code, stderr) = run(&vault, &["import", csv.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(events_in(&vault).len(), 1);
}
