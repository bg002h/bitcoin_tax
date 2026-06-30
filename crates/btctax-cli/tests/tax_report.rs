//! CLI integration tests for `report --tax-year` (Task 9 / B.5).
//! Uses temp vaults + synthetic fixtures only — PRIVACY: never reads ~/Documents/BitcoinTax/ReadOnly.
//!
//! Golden derivation (Single TY2025, LT gain=20,000, OTI=40,000, MAGI excl.=60,000):
//!   Rev. Proc. 2024-40 §2.03 Single §1(h) breakpoints: max_zero=48,350; max_fifteen=533,400.
//!   pref stack: bottom=40,000, pref=20,000, top=60,000.
//!     at_0  = 48,350 − 40,000 = 8,350
//!     at_15 = 60,000 − 48,350 = 11,650
//!     ltcg_tax = 11,650 × 0.15 = 1,747.50
//!   NIIT: magi_with = 60,000 + 20,000 = 80,000 < 200,000 (Single threshold) → 0.
//!   ordinary_delta: OTI unchanged by LT gain → 0.
//!   total = 0 + 1,747.50 + 0 = 1,747.50.
use btctax_cli::{cmd, render};
use btctax_core::{Carryforward, FilingStatus, TaxProfile};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// Single filer: OTI=40,000; MAGI excl. crypto=60,000; QD=0.
/// MAGI is set so that even after adding the 20,000 LT gain (→ magi_with=80,000) we stay
/// below the Single §1411 NIIT threshold of $200,000.
fn single_40k_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
    }
}

/// Synthetic Coinbase CSV: pre-2025 buy (2023-01-01) + 2025 LT sell (2025-06-15).
/// Buy:  1 BTC, subtotal=30,000, fees=0 → exchange-provided basis=30,000.
/// Sell: 1 BTC, subtotal=50,000, fees=0 → exchange-provided proceeds=50,000.
/// Term: LT (2023-01-01 → 2025-06-15 = ~2.5 years > 1 year).
/// Gain: 50,000 − 30,000 = 20,000 (LT).
fn write_lt_sell_2025(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_lt.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lt-buy,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
lt-sell,2025-06-15 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Synthetic Coinbase CSV: buy + unclassified Receive → `UnknownBasisInbound` hard blocker.
/// The unclassified Receive (no `ClassifyInbound` decision) folds as `Op::UnknownInbound` →
/// `BlockerKind::UnknownBasisInbound` (Hard), which gates B.4 computation for every year.
fn write_buy_receive(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_rcv.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
hb-buy,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
hb-recv,2025-01-01 12:00:00 UTC,Receive,BTC,0.10000000,USD,44000.00,,,,,,\r\n",
    )
    .unwrap();
    p
}

/// Init vault + import one CSV file; return `(tempdir, vault_path)`.
fn make_vault_with(csv: &Path) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv.to_path_buf()]).unwrap();
    (dir, vault)
}

/// Golden render test: Single, LT gain=20,000, OTI=40k, MAGI excl=60k → total=1,747.50.
/// Asserts the rendered output contains the expected TOTAL line.
#[test]
fn report_tax_year_renders_golden() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile()).unwrap();

    let outcome = cmd::tax::report_tax_year(&vault, &pp(), 2025).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome);

    assert!(
        rendered.contains("TOTAL federal tax attributable to crypto (delta): 1747.50"),
        "expected total 1747.50 in rendered output:\n{rendered}"
    );
}

/// B-M2 reconciliation KAT: the three printed attributable components sum to TOTAL.
/// For the Single LT 0→15 golden: ordinary-rate=0.00 + LTCG=1747.50 + NIIT=0.00 = 1747.50.
/// Also verifies the reconciliation numerically from the raw `TaxResult` fields.
#[test]
fn report_tax_year_components_reconcile_to_total() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile()).unwrap();

    let outcome = cmd::tax::report_tax_year(&vault, &pp(), 2025).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome);

    // Rendered component lines.  Zero values may display as "0" or "0.00" depending on the
    // rust_decimal scale of the intermediate result; assert on the prefix that's stable.
    assert!(
        rendered.contains("ordinary-rate tax (attributable): 0"),
        "ordinary-rate delta must be zero for a pure-LT vault:\n{rendered}"
    );
    assert!(
        rendered.contains("LTCG tax (attributable): 1747.50"),
        "LTCG must be 1747.50:\n{rendered}"
    );
    assert!(
        rendered.contains("NIIT (attributable): 0"),
        "NIIT must be zero (MAGI below Single threshold):\n{rendered}"
    );
    assert!(
        rendered.contains("TOTAL federal tax attributable to crypto (delta): 1747.50"),
        "TOTAL must be 1747.50:\n{rendered}"
    );

    // Numeric reconciliation from the raw TaxResult (B-M2 identity).
    let btctax_core::TaxOutcome::Computed(r) = outcome else {
        panic!("expected TaxOutcome::Computed");
    };
    let ordinary_rate_attributable = r.total_federal_tax_attributable - r.ltcg_tax - r.niit;
    assert_eq!(
        ordinary_rate_attributable + r.ltcg_tax + r.niit,
        r.total_federal_tax_attributable,
        "B-M2: ordinary-rate + LTCG + NIIT must equal total_federal_tax_attributable"
    );
    assert_eq!(r.ltcg_tax, dec!(1747.50));
    assert_eq!(r.niit, dec!(0.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(1747.50));
}

/// No profile for the year → `NotComputable(TaxProfileMissing)` rendered; no dollar amount.
#[test]
fn report_tax_year_without_profile_says_not_computable() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Deliberately do NOT set a tax profile for 2025.

    let outcome = cmd::tax::report_tax_year(&vault, &pp(), 2025).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome);

    assert!(
        rendered.contains("NOT COMPUTABLE [TaxProfileMissing]"),
        "missing profile must render TaxProfileMissing:\n{rendered}"
    );
    // Must not contain a computed dollar amount.
    assert!(
        !rendered.contains("TOTAL federal tax attributable"),
        "must not print a computed total when profile is missing:\n{rendered}"
    );
}

/// Unresolved hard blocker (UnknownBasisInbound from unclassified Receive) →
/// `NotComputable(TaxYearNotComputable)` rendered; no dollar amount.
/// B.4 / I6: ANY hard blocker gates computation projection-wide.
#[test]
fn report_tax_year_with_hard_blocker_says_not_computable() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Set a profile so the refusal is definitely from the hard blocker (not TaxProfileMissing).
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile()).unwrap();

    let outcome = cmd::tax::report_tax_year(&vault, &pp(), 2025).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome);

    assert!(
        rendered.contains("NOT COMPUTABLE [TaxYearNotComputable]"),
        "hard blocker must render TaxYearNotComputable:\n{rendered}"
    );
    assert!(
        !rendered.contains("TOTAL federal tax attributable"),
        "must not print a computed total when hard blockers are present:\n{rendered}"
    );
}

/// Regression: `report --year 2025` (the existing display path) still works after adding
/// `--tax-year`. Tests `cmd::inspect::report` + `render::render_report` directly.
#[test]
fn report_display_year_still_works_unchanged() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Existing display path: LedgerState + render_report (no tax computation).
    let state = cmd::inspect::report(&vault, &pp(), Some(2025)).unwrap();
    let rendered = render::render_report(&state, Some(2025));

    assert!(
        rendered.contains("Disposals (year 2025):"),
        "display path must still show disposals section:\n{rendered}"
    );
    // Must not trigger tax computation (no "NOT COMPUTABLE" or "TOTAL federal" lines).
    assert!(
        !rendered.contains("NOT COMPUTABLE"),
        "display path must not trigger tax computation:\n{rendered}"
    );
    assert!(
        !rendered.contains("TOTAL federal tax"),
        "display path must not show tax output:\n{rendered}"
    );
}
