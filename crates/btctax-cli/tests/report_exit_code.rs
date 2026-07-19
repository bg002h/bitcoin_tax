//! Binary-level exit-code contract for `report --tax-year` (UX-P4-10).
//!
//! `report --tax-year Y` returns **exit 1** when the outcome is `TaxOutcome::NotComputable` (it ran
//! but produced NO filing-ready number — mirrors `verify`'s non-zero), and **exit 0** for a rendered
//! report. The TWO deliberate exit-0 NON-triggers (SPEC §3.5), both pinned here: (1) a **pseudo-active**
//! report — the outcome is `Computed(placeholder)`, the banner is the signal; (2) a **dual-report whose
//! ABSOLUTE total is refused** by `screen_absolute` while the crypto **delta** still computes — the
//! exit code keys ONLY on the delta `outcome`, never the absolute refusal. (`run_to_exit` maps any
//! `Err` to 2, so the contract keys on NON-ZERO.)
use btctax_cli::cmd;
use btctax_cli::cmd::tax::TaxYearReport;
use btctax_core::{Carryforward, FilingStatus, TaxProfile, Usd};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// A single 2025 BTC Buy — reconciled, no disposal, no unclassified inbound.
fn write_buy_2025(dir: &Path) -> PathBuf {
    let p = dir.join("buy2025.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
rx-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n",
    )
    .unwrap();
    p
}

/// A 2025 Buy + an unclassified Receive (no ClassifyInbound) → hard `UnknownBasisInbound`; under
/// pseudo mode the basis + profile are synthesized so the year becomes `Computed(placeholder)`.
fn write_buy_receive_2025(dir: &Path) -> PathBuf {
    let p = dir.join("buy_recv2025.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
rx-buy2,2025-04-01 12:00:00 UTC,Buy,BTC,0.05000000,USD,85000.00,4250.00,4265.00,15.00,,,\r\n\
rx-recv,2025-05-01 12:00:00 UTC,Receive,BTC,0.02000000,USD,90000.00,,,,,,\r\n",
    )
    .unwrap();
    p
}

/// A single 2024 BTC Buy — a computable (zero-crypto-tax) TY2024 year, no disposal/pending.
fn write_buy_2024(dir: &Path) -> PathBuf {
    let p = dir.join("buy2024.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
rx-buy24,2024-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,60000.00,6000.00,6000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

fn single_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: Usd::ZERO,
        other_net_capital_gain: Usd::ZERO,
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: Usd::ZERO,
        w2_medicare_wages: Usd::ZERO,
        schedule_c_expenses: Usd::ZERO,
    }
}

/// Run `btctax --vault <vault> report --tax-year <year>` via the compiled binary; return the exit code.
fn run_report(vault: &Path, year: &str) -> i32 {
    let bin = env!("CARGO_BIN_EXE_btctax");
    std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().expect("vault path is valid UTF-8"),
            "report",
            "--tax-year",
            year,
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("btctax binary must execute")
        .code()
        .expect("btctax process must exit normally (not via signal)")
}

/// NotComputable (no tax profile → `TaxProfileMissing`) ⇒ exit 1 (no filing-ready number).
#[test]
fn report_notcomputable_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_buy_2025(dir.path())]).unwrap();
    // No profile set → report_tax_year = NotComputable(TaxProfileMissing).
    assert_eq!(
        run_report(&vault, "2025"),
        1,
        "a NotComputable report must exit 1 (mirrors verify)"
    );
}

/// A Computed report (profile present, bundled table) ⇒ exit 0.
#[test]
fn report_computed_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_buy_2025(dir.path())]).unwrap();
    cmd::tax::set_profile(&vault, &pp(), 2025, single_profile(), false).unwrap();
    assert_eq!(
        run_report(&vault, "2025"),
        0,
        "a Computed report must exit 0"
    );
}

/// A hard-blocker year (unclassified inbound, no pseudo, no profile) is `NotComputable` too — the
/// kind that most directly mirrors `verify` — and must exit 1. Pins the hard-blocker kind, distinct
/// from `TaxProfileMissing`.
#[test]
fn report_hard_blocker_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_buy_receive_2025(dir.path())]).unwrap();
    // No pseudo, no profile → the unclassified Receive is a hard blocker → NotComputable → exit 1.
    assert_eq!(
        run_report(&vault, "2025"),
        1,
        "a hard-blocker (NotComputable) report must exit 1"
    );
}

/// A pseudo-active report is a deliberate exit-0 NON-trigger (SPEC §3.5): the outcome is
/// `Computed(placeholder)`, the banner is the signal — the exit code must stay 0.
#[test]
fn report_pseudo_active_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_buy_receive_2025(dir.path())]).unwrap();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    assert_eq!(
        run_report(&vault, "2025"),
        0,
        "a pseudo-active (Computed placeholder) report must exit 0 — the banner is the signal"
    );
}

/// SPEC §3.5 non-trigger 2: a dual-report whose ABSOLUTE 1040 total is REFUSED (`screen_absolute`)
/// but whose crypto DELTA still computes must exit 0 — the exit code keys on the delta `outcome`, not
/// the absolute refusal. Setup: a TY2024 ReturnInputs return with $0 wages (taxable income clamps to
/// 0) + a capital-loss carryforward trips screen_absolute (c) (§1211/§1212 worksheet unmodeled) → the
/// ABSOLUTE is NOT COMPUTABLE; the delta (no 2024 disposal → zero crypto tax) Computes.
#[test]
fn report_dual_report_absolute_refused_delta_computed_exits_zero() {
    use btctax_cli::{return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[write_buy_2024(dir.path())]).unwrap();
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let ri = btctax_core::tax::testonly::answered(ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(1000),
                long: Usd::ZERO,
            },
            ..Default::default()
        });
        return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }

    // Lib-level: prove we actually hit the intended state (absolute refused, delta computed).
    let TaxYearReport {
        outcome,
        dual_report,
        ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap();
    assert!(
        matches!(outcome, btctax_core::TaxOutcome::Computed(_)),
        "the crypto DELTA must Compute"
    );
    let dual = dual_report.expect("a ReturnInputs-provenance year renders the dual report");
    assert!(
        dual.contains("NOT COMPUTABLE"),
        "the ABSOLUTE return must be refused by screen_absolute:\n{dual}"
    );

    // Binary-level: the absolute refusal is NOT an exit-1 trigger.
    assert_eq!(
        run_report(&vault, "2024"),
        0,
        "a dual-report absolute-refused-but-delta-computed report must exit 0 (SPEC §3.5)"
    );
}
