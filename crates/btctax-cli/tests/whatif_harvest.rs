//! CLI integration tests for `what-if harvest` (task #43, phase P2).
//!
//! `what-if harvest` is READ-ONLY: it calls `btctax_core::whatif::harvest` (the segment-walk optimizer,
//! clone-fold-discard) and never `session.save()`. These tests assert the NON-NEGOTIABLE non-persistence
//! invariant end-to-end (`vault.pgp` BYTE-IDENTICAL before and after) for BOTH the `zero-ltcg` and the
//! `tax=$0` sanity targets, plus the ad-hoc-profile path and the rendered disclosures.
//!
//! PRIVACY: all fixtures synthetic, written into tempdirs; no user file is ever read. Real bundled 2025
//! tax tables (so N* itself is not asserted — the INVARIANTS are). Deterministic (NFR4).
use btctax_cli::{cmd, render};
use btctax_core::persistence::load_all;
use btctax_core::whatif::HarvestStatus;
use btctax_core::{FilingStatus, WalletId};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::date;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn coinbase_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "default".into(),
    }
}
fn make_vault_with(csv: &Path) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv.to_path_buf()]).unwrap();
    (dir, vault)
}
fn event_count(vault: &Path) -> usize {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    load_all(s.conn()).unwrap().len()
}
/// Two buy lots: low-basis LT (A, $30k, 2024-01-15) + high-basis (B, $80k, 2025-03-01). No sell.
fn write_two_lots_csv(dir: &Path) -> PathBuf {
    let p = dir.join("harvest_two_lots.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lt-buy,2024-01-15 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
ht-buy,2025-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,80000.00,80000.00,80000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}
fn adhoc_single(income: rust_decimal::Decimal) -> cmd::whatif::AdhocProfile {
    cmd::whatif::AdhocProfile {
        filing_status: FilingStatus::Single,
        income,
        magi: None, // defaults to income (caveat set)
        cf_long: dec!(0),
    }
}

/// NON-NEGOTIABLE: `what-if harvest --target zero-ltcg` writes NOTHING — the vault is byte-identical and
/// the event count is unchanged. The ad-hoc income $0 gives a meaningful 0%-bracket answer.
#[test]
fn harvest_zero_ltcg_never_persists() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    let bytes_before = std::fs::read(&vault).unwrap();
    let events_before = event_count(&vault);

    let outcome = cmd::whatif::harvest(
        &vault,
        &pp(),
        coinbase_wallet(),
        date!(2025 - 08 - 01),
        Some(dec!(120000)), // per-BTC price → both lots are gains
        cmd::whatif::parse_harvest_target("zero-ltcg").unwrap(),
        Some(adhoc_single(dec!(0))),
    )
    .expect("what-if harvest computes");

    // A sensible answer (some gain fits the 0% bracket) + the ad-hoc MAGI caveat.
    assert!(outcome.report.n_sat > 0, "some BTC fits the 0% bracket");
    assert!(matches!(
        outcome.report.status,
        HarvestStatus::Found | HarvestStatus::NotBinding
    ));
    assert!(outcome.magi_caveat, "omitting --magi sets the caveat");

    let rendered = render::render_whatif_harvest(&outcome.report, outcome.magi_caveat);
    assert!(rendered.contains("What-if HARVEST"), "header:\n{rendered}");
    assert!(
        rendered.contains("sell up to"),
        "answer headline:\n{rendered}"
    );
    assert!(
        rendered.contains("Tax decision-support only"),
        "not-investment-advice footer:\n{rendered}"
    );

    // THE INVARIANT: vault byte-identical, no event appended.
    assert_eq!(
        bytes_before,
        std::fs::read(&vault).unwrap(),
        "what-if harvest must leave vault.pgp BYTE-IDENTICAL"
    );
    assert_eq!(events_before, event_count(&vault), "no event appended");
}

/// The `tax=$0` sanity target is also fully read-only (vault byte-identical), via the STORED profile.
#[test]
fn harvest_tax_zero_never_persists() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Stored profile (Single, ord $0 → a positive zero-tax room).
    cmd::tax::set_profile(&vault, &pp(), 2025, {
        btctax_core::TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: dec!(0),
            magi_excluding_crypto: dec!(0),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: btctax_core::Carryforward::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        }
    }, false)
    .unwrap();

    let bytes_before = std::fs::read(&vault).unwrap();
    let events_before = event_count(&vault);

    let outcome = cmd::whatif::harvest(
        &vault,
        &pp(),
        coinbase_wallet(),
        date!(2025 - 08 - 01),
        Some(dec!(120000)),
        cmd::whatif::parse_harvest_target("tax=$0").unwrap(),
        None, // stored profile
    )
    .expect("what-if harvest tax=$0 computes");

    assert!(outcome.report.n_sat > 0, "some BTC adds zero federal tax");
    assert!(!outcome.magi_caveat, "stored profile → no ad-hoc caveat");

    assert_eq!(
        bytes_before,
        std::fs::read(&vault).unwrap(),
        "tax=$0 harvest must leave vault.pgp BYTE-IDENTICAL"
    );
    assert_eq!(events_before, event_count(&vault), "no event appended");
}
