//! CLI integration tests for the `tax-profile` set/show command (Task 8).
//! Uses a temp vault + synthetic data only — PRIVACY: never reads ~/Documents/BitcoinTax/ReadOnly.
use btctax_cli::{cmd, tax_profile};
use btctax_core::{Carryforward, FilingStatus, TaxProfile};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn prof_2025() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Mfj,
        ordinary_taxable_income: dec!(120000),
        magi_excluding_crypto: dec!(130000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
    }
}

/// Set a profile for 2025, then show it — must round-trip exactly.
#[test]
fn set_then_show_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // set
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025()).unwrap();

    // show
    let got = cmd::tax::show_profile(&vault, &pp(), 2025).unwrap();
    assert_eq!(got, Some(prof_2025()));
}

/// show for a year with no stored profile → None.
#[test]
fn show_missing_year_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let got = cmd::tax::show_profile(&vault, &pp(), 2025).unwrap();
    assert_eq!(got, None);
}

/// Overwriting an existing profile upserts (replaces the old value).
#[test]
fn set_overwrites_previous_profile() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025()).unwrap();

    let updated = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(80000),
        magi_excluding_crypto: dec!(85000),
        qualified_dividends_and_other_pref_income: dec!(500),
        other_net_capital_gain: dec!(1000),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(200),
            long: dec!(300),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, updated.clone()).unwrap();

    let got = cmd::tax::show_profile(&vault, &pp(), 2025).unwrap();
    assert_eq!(got, Some(updated));
}

/// Multiple years are stored independently.
#[test]
fn multiple_years_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let p24 = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(90000),
        magi_excluding_crypto: dec!(95000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2024, p24.clone()).unwrap();
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025()).unwrap();

    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2024).unwrap(),
        Some(p24)
    );
    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2025).unwrap(),
        Some(prof_2025())
    );
    assert_eq!(cmd::tax::show_profile(&vault, &pp(), 2026).unwrap(), None);
}

/// `tax_profile::get` on a vault opened (not freshly created) still works (robust to missing DDL
/// call — the defensive `init_table` guard inside `get` creates the table if absent).
#[test]
fn get_on_open_vault_without_prior_init_table_call_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Open the vault and call the low-level `get` directly — no `Session::tax_profile` wrapper.
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    let result = tax_profile::get(s.conn(), 2025).unwrap();
    assert_eq!(result, None);
}
