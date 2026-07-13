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
        schedule_c_expenses: dec!(0),
    }
}

/// Set a profile for 2025, then show it — must round-trip exactly.
#[test]
fn set_then_show_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // set
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();

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

    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();

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
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, updated.clone(), false).unwrap();

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
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2024, p24.clone(), false).unwrap();
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();

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

/// D-4 guard (SPEC §4.12): once full-return `ReturnInputs` exist for a year, a raw `tax-profile set`
/// for that year is REFUSED (would be silently ignored by `resolve_profile`) — unless `--force`.
/// `income clear` removes the inputs and re-opens the raw path. This is the two-sources-of-truth
/// guard end to end (import → refuse → force → clear → allow).
#[test]
fn set_profile_is_refused_while_return_inputs_exist_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Seed full-return inputs for 2024 via the TOML import path.
    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"82000\"\nbox2_fed_withheld = \"9100\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml).unwrap();

    // A raw tax-profile for the SAME year is refused (Usage), and nothing is stored.
    let err = cmd::tax::set_profile(&vault, &pp(), 2024, prof_2025(), false).unwrap_err();
    assert!(matches!(err, btctax_cli::CliError::Usage(_)));
    assert_eq!(cmd::tax::show_profile(&vault, &pp(), 2024).unwrap(), None);

    // A DIFFERENT year is unaffected (the guard is per-year).
    cmd::tax::set_profile(&vault, &pp(), 2025, prof_2025(), false).unwrap();
    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2025).unwrap(),
        Some(prof_2025())
    );

    // --force overrides the guard and stores the raw profile anyway.
    cmd::tax::set_profile(&vault, &pp(), 2024, prof_2025(), true).unwrap();
    assert_eq!(
        cmd::tax::show_profile(&vault, &pp(), 2024).unwrap(),
        Some(prof_2025())
    );

    // `income clear` removes the inputs; afterward the un-forced path is allowed again.
    assert!(cmd::tax::clear_return_inputs(&vault, &pp(), 2024).unwrap());
    assert!(!cmd::tax::clear_return_inputs(&vault, &pp(), 2024).unwrap()); // idempotent
    cmd::tax::set_profile(&vault, &pp(), 2024, prof_2025(), false).unwrap();
}

/// Vault-level `income import` → `income show` round trip (review M6): imports a TOML with a cleartext
/// SSN, then `income show` must return the stored inputs as JSON with the SSN REDACTED (I5) — cleartext
/// PII must never reach stdout. `show` on an unset year is `None`.
#[test]
fn income_import_then_show_redacts_pii_at_the_vault_level() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    assert_eq!(
        cmd::tax::show_return_inputs(&vault, &pp(), 2024).unwrap(),
        None
    );

    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n\n[header.taxpayer]\nfirst_name = \"Pat\"\nlast_name = \"Doe\"\nssn = \"123-45-6789\"\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"82000\"\nbox2_fed_withheld = \"9100\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml).unwrap();

    let shown = cmd::tax::show_return_inputs(&vault, &pp(), 2024)
        .unwrap()
        .expect("inputs were imported");
    assert!(
        shown.contains("***-**-6789"),
        "SSN must be redacted: {shown}"
    );
    assert!(
        !shown.contains("123-45-6789"),
        "cleartext SSN must never appear: {shown}"
    );
    assert!(shown.contains("82000")); // non-PII figures are shown verbatim
}

/// [P2 review M-r3-4 / N3] `resolve_all_screened` maps a corrupt side-table row to a per-year refusal
/// (Uncomputable), NOT a whole-vault brick — the read-only viewer must still open, with other years
/// resolving normally. Pins the availability behavior N3 introduced.
#[test]
fn resolve_all_screened_maps_a_corrupt_year_to_a_refusal_not_a_brick() {
    use btctax_cli::{resolve::ProfileOutcome, return_inputs, Session};
    use btctax_core::tax::return_inputs::ReturnInputs;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Valid full-return inputs for 2024 + a CORRUPT `return_inputs` blob for 2023 (one bad side-table row).
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        return_inputs::set(
            s.conn(),
            2024,
            &ReturnInputs {
                filing_status: FilingStatus::Single,
                ..Default::default()
            },
        )
        .unwrap();
        s.conn()
            .execute(
                "INSERT INTO return_inputs(year, inputs_json) VALUES (2023, 'not json')",
                [],
            )
            .unwrap();
        s.save().unwrap();
    }

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _cfg) = s.project().unwrap();
    let tables = btctax_adapters::BundledTaxTables::load();
    let resolved = s.resolve_all_screened(&state, &tables).unwrap();

    // 2023 (corrupt) → per-year Uncomputable; 2024 (valid) → Ready. The viewer is NOT bricked.
    assert!(
        matches!(
            resolved.get(&2023),
            Some(ProfileOutcome::Uncomputable { .. })
        ),
        "a corrupt 2023 blob must become a per-year refusal, not fail the whole enumeration"
    );
    assert!(
        matches!(resolved.get(&2024), Some(ProfileOutcome::Ready { .. })),
        "the valid 2024 year must still resolve"
    );
}
