//! CLI integration tests for `what-if sell` (task #43, phase P1).
//!
//! `what-if sell` is READ-ONLY: it calls `btctax_core::whatif::sell` (clone-fold-discard) and never
//! `session.save()`. These tests assert the NON-NEGOTIABLE non-persistence invariant end-to-end (the
//! `vault.pgp` file is BYTE-IDENTICAL before and after any sell), plus the ad-hoc-profile path and the
//! [R0-M4] `--magi`-defaults-to-`--income` caveat.
//!
//! PRIVACY: all fixtures are synthetic, written into tempdirs; no user file is ever read. Exact
//! Decimal/i64 (NFR5); deterministic (NFR4).
use btctax_cli::{cmd, render};
use btctax_core::persistence::load_all;
use btctax_core::whatif::SellStatus;
use btctax_core::{Carryforward, FilingStatus, LotMethod, TaxProfile, WalletId};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::date;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn single_100k_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(100000),
        magi_excluding_crypto: dec!(100000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    }
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
    let p = dir.join("whatif_two_lots.csv");
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

/// NON-NEGOTIABLE: `what-if sell` writes NOTHING — the `vault.pgp` file is byte-identical before and
/// after, and the event count is unchanged. (HIFO picks the $80k lot → a loss; the report still returns.)
#[test]
fn whatif_never_persists() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let bytes_before = std::fs::read(&vault).unwrap();
    let events_before = event_count(&vault);

    let outcome = cmd::whatif::sell(
        &vault,
        &pp(),
        100_000_000, // 1 BTC
        coinbase_wallet(),
        date!(2025 - 08 - 01),
        Some(dec!(67500)),     // explicit per-BTC price
        Some(LotMethod::Hifo), // HIFO → the $80k lot → a loss
        None,                  // use the stored profile
    )
    .expect("what-if sell computes");

    // The report is sensible (HIFO consumed the high-basis lot → a loss).
    assert_eq!(outcome.report.lots.len(), 1, "one lot consumed");
    assert_eq!(outcome.report.lots[0].basis, dec!(80000));
    assert_eq!(outcome.report.status, SellStatus::Loss);
    assert!(
        !outcome.magi_caveat,
        "stored profile → no ad-hoc MAGI caveat"
    );

    // The renderer surfaces the marginal headline + the §1212 carryforward disclosure + the footer.
    let rendered = render::render_whatif_sell(&outcome.report, outcome.magi_caveat);
    assert!(
        rendered.contains("marginal federal tax (this sale):"),
        "render must headline the marginal tax:\n{rendered}"
    );
    assert!(
        rendered.contains("\u{00a7}1212:"),
        "a loss sale must show the §1212 carryforward disclosure:\n{rendered}"
    );
    assert!(
        rendered.contains("Tax decision-support only"),
        "render must carry the not-investment-advice footer:\n{rendered}"
    );

    // THE INVARIANT: the vault is byte-identical and no event was appended.
    let bytes_after = std::fs::read(&vault).unwrap();
    assert_eq!(
        bytes_before, bytes_after,
        "what-if sell must leave vault.pgp BYTE-IDENTICAL (no save())"
    );
    assert_eq!(
        events_before,
        event_count(&vault),
        "no event may be appended by a what-if"
    );
}

/// The ad-hoc (non-persisted) profile path: a vault with NO stored profile still plans via the inline
/// `--filing-status`/`--income` profile, and [R0-M4] omitting `--magi` sets the caveat flag (defaulted
/// to income, never $0). The vault stays byte-identical.
#[test]
fn whatif_adhoc_profile_magi_defaults_and_never_persists() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv); // NO tax-profile set for 2025

    let bytes_before = std::fs::read(&vault).unwrap();

    let outcome = cmd::whatif::sell(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 08 - 01),
        Some(dec!(120000)), // per-BTC price → the low-basis lot is a gain
        Some(LotMethod::Fifo),
        Some(cmd::whatif::AdhocProfile {
            filing_status: FilingStatus::Single,
            income: dec!(120000),
            magi: None, // [R0-M4] defaults to income → caveat set
            cf_long: dec!(0),
        }),
    )
    .expect("ad-hoc profile lets the plan run with no stored profile");

    assert!(
        outcome.magi_caveat,
        "omitting --magi must set the caveat (defaulted to income)"
    );
    assert_eq!(outcome.report.lots.len(), 1);

    let bytes_after = std::fs::read(&vault).unwrap();
    assert_eq!(
        bytes_before, bytes_after,
        "the ad-hoc path is still read-only (vault byte-identical)"
    );
}
