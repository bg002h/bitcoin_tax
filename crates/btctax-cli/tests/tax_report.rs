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
use btctax_cli::{cmd, render, Session};
use btctax_core::{Carryforward, EventPayload, FilingStatus, InboundClass, IncomeKind, TaxProfile};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::datetime;

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

    let (outcome, advisory, sched_d, _gift, _se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome, advisory.as_deref());

    assert!(
        rendered.contains("TOTAL federal tax attributable to crypto (delta): 1747.50"),
        "expected total 1747.50 in rendered output:\n{rendered}"
    );

    // P2-B Task 3 (KAT — Computed): the RAW Schedule D part totals ride the same report path
    // (same projection). The fixture is a single LT sell (proceeds 50,000; basis 30,000;
    // gain 20,000), no ST. Computed year → netting note present; no "NOT COMPUTABLE" caveat.
    assert_eq!(sched_d.lt.proceeds, dec!(50000));
    assert_eq!(sched_d.lt.cost_basis, dec!(30000));
    assert_eq!(sched_d.lt.gain, dec!(20000));
    assert_eq!(sched_d.st.gain, dec!(0));
    let sd_text = render::render_schedule_d(2025, &sched_d, &outcome);
    assert!(
        sd_text.contains("§1222/§1211/§1212 netting + carryforward")
            && sd_text.contains("raw pre-netting"),
        "Schedule D text (Computed) must carry the netting note:\n{sd_text}"
    );
    assert!(
        !sd_text.contains("NOT COMPUTABLE"),
        "Schedule D text (Computed) must NOT carry the NotComputable caveat:\n{sd_text}"
    );
    assert!(
        sd_text.contains(
            "Part II (long-term):  proceeds 50000.00   cost basis 30000.00   gain 20000.00"
        ),
        "Schedule D text must show the LT part totals:\n{sd_text}"
    );
}

/// [P2-D Task 2 wiring] A business-mining year → `report_tax_year` surfaces the Schedule SE section
/// (components + total + §164(f) half + the dual-direction W-2 disclosure + the standalone note),
/// and the income-tax report's TOTAL is UNCHANGED by SE tax (D5 — SE is standalone).
#[test]
fn report_tax_year_renders_schedule_se_for_business_mining() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the unclassified Receive as $100,000 BUSINESS mining income (SE-eligible).
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    let class = InboundClass::Income {
        kind: IncomeKind::Mining,
        fmv: Some(dec!(100000.00)),
        business: true,
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        class,
        datetime!(2025-06-01 00:00:00 UTC),
    )
    .unwrap();

    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile()).unwrap();

    let (outcome, _advisory, _sched_d, _gift, se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let se = se.expect("Schedule SE section expected for a business-mining year");

    // Golden 1 components (Single, $100,000 business mining).
    assert!(se.contains("11451.40"), "SS component: {se}");
    assert!(se.contains("2678.15"), "Medicare component: {se}");
    assert!(se.contains("14129.55"), "total SE tax: {se}");
    assert!(se.contains("7064.78"), "§164(f) deductible half: {se}");
    // Dual-direction W-2 disclosure + standalone note.
    assert!(
        se.contains("OVERSTATED") && se.contains("UNDERSTATED"),
        "{se}"
    );
    assert!(se.contains("SEPARATE federal liability"), "{se}");
    // [Minor-2] expenses caveat — render must disclose that no Schedule C expenses are modeled.
    assert!(
        se.contains("Schedule C deductible business expenses are not modeled"),
        "expenses caveat must appear in Schedule SE render: {se}"
    );

    // [D5] the income-tax report TOTAL is UNCHANGED by SE tax — the $14,129.55 SE figure is NOT
    // added to total_federal_tax_attributable (the mining is taxed only as ordinary income there).
    // Golden (Single, OTI=40,000, MAGI_excl=60,000, $100,000 business mining, TY2025 real table):
    //   ordinary_delta = tax(140,000) − tax(40,000) = 26,447.00 − 4,561.50 = 21,885.50; NIIT=0.
    // This assertion FAILS if SE tax ($14,129.55) were ever folded in (would be 36,015.05).
    if let btctax_core::TaxOutcome::Computed(r) = &outcome {
        assert_eq!(
            r.total_federal_tax_attributable,
            dec!(21885.50),
            "[D5] total_federal_tax_attributable must be income-tax delta only (21,885.50), not including SE tax"
        );
    } else {
        panic!("computable (blocker resolved by classify)");
    }
    let it = render::render_tax_outcome(2025, &outcome, None);
    assert!(
        !it.contains("14129.55"),
        "SE tax must NOT appear in the income-tax report total (standalone, D5):\n{it}"
    );
}

/// [B-M1 Task 2] The Computed tax-report footer no longer carries the wrong-direction "understate"
/// disclosure; it accurately states the §1211(b) loss is applied to NII and flags the residual
/// crypto-lending-interest caveat.
#[test]
fn report_tax_year_footer_discloses_1211_loss_and_lending_interest_caveat() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile()).unwrap();

    let (outcome, advisory, _sched_d, _gift, _se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome, advisory.as_deref());

    // Wrong-direction language removed:
    assert!(
        !rendered.contains("can only ever understate"),
        "footer must not claim NIIT can only ever understate:\n{rendered}"
    );
    assert!(
        !rendered.contains("MAY UNDERSTATE"),
        "footer must not carry the wrong-direction MAY UNDERSTATE claim:\n{rendered}"
    );
    assert!(
        !rendered.contains("does not reduce NII"),
        "footer must not claim NII is not reduced by the §1211 loss:\n{rendered}"
    );
    // Accurate position present:
    assert!(
        rendered.contains("reduces NII by the §1211(b)-allowed net capital loss"),
        "footer must state the §1211(b) loss is applied to NII:\n{rendered}"
    );
    assert!(
        rendered.contains("crypto-lending interest"),
        "footer must flag the residual crypto-lending-interest understatement:\n{rendered}"
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

    let (outcome, advisory, _sched_d, _gift, _se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome, advisory.as_deref());

    // B-F1: all dollar figures are now fmt_money-formatted to exactly 2dp; assert the 2dp forms.
    assert!(
        rendered.contains("ordinary-rate tax (attributable): 0.00"),
        "ordinary-rate delta must be 0.00 (2dp) for a pure-LT vault:\n{rendered}"
    );
    assert!(
        rendered.contains("LTCG tax (attributable): 1747.50"),
        "LTCG must be 1747.50:\n{rendered}"
    );
    assert!(
        rendered.contains("NIIT (attributable): 0.00"),
        "NIIT must be 0.00 (2dp; MAGI below Single threshold):\n{rendered}"
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

    let (outcome, advisory, _sched_d, _gift, _se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome, advisory.as_deref());

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
///
/// KAT (NotComputable): Schedule D STILL shows the raw part totals AND carries the
/// "NOT COMPUTABLE / informational" caveat; the §1222/§1211/§1212 netting note is absent.
#[test]
fn report_tax_year_with_hard_blocker_says_not_computable() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Set a profile so the refusal is definitely from the hard blocker (not TaxProfileMissing).
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile()).unwrap();

    let (outcome, advisory, sched_d, _gift, _se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(2025, &outcome, advisory.as_deref());

    assert!(
        rendered.contains("NOT COMPUTABLE [TaxYearNotComputable]"),
        "hard blocker must render TaxYearNotComputable:\n{rendered}"
    );
    assert!(
        !rendered.contains("TOTAL federal tax attributable"),
        "must not print a computed total when hard blockers are present:\n{rendered}"
    );

    // KAT: Schedule D raw totals are shown AND the NotComputable caveat is present.
    // The fixture (buy + unclassified receive, no sell) has zero disposals → all totals are 0,
    // but the section header and caveat must still appear.
    let sd_text = render::render_schedule_d(2025, &sched_d, &outcome);
    assert!(
        sd_text.contains("Schedule D (raw pre-netting part totals)"),
        "Schedule D section header must appear even for a NotComputable year:\n{sd_text}"
    );
    assert!(
        sd_text.contains("Part I  (short-term)"),
        "Schedule D must show Part I totals even for a NotComputable year:\n{sd_text}"
    );
    assert!(
        sd_text.contains("Part II (long-term)"),
        "Schedule D must show Part II totals even for a NotComputable year:\n{sd_text}"
    );
    assert!(
        sd_text.contains("NOT COMPUTABLE"),
        "Schedule D must carry the NOT COMPUTABLE caveat for a NotComputable outcome:\n{sd_text}"
    );
    assert!(
        sd_text.contains("informational"),
        "Schedule D caveat must include 'informational':\n{sd_text}"
    );
    assert!(
        !sd_text.contains("§1222/§1211/§1212 netting"),
        "Schedule D must NOT show the netting note for a NotComputable outcome:\n{sd_text}"
    );
}

/// M4 (Task 10): when the declared 2026 `carryforward_in` ≠ 2025's computed `carryforward_out`,
/// `report --tax-year 2026` renders the advisory line and still exits 0 (non-gating).
///
/// Scenario derivation (hand-verified):
///   2025 vault: buy 1 BTC @ $50,000 on 2025-01-15 (ST); sell 1 BTC @ $40,000 on 2025-06-15 (ST).
///   → crypto_st = −$10,000; no LT; no carryforward-in declared in the 2025 profile.
///   net_1222(−10000, 0, 0, 0, 0, 3000):
///     st_net = −10000; lt_net = 0; both losses cross-net: no cross.
///     loss_deduction = min(10000, 3000) = 3000; absorbed_st = 3000; absorbed_lt = 0.
///     st_carry = 10000 − 3000 = 7000; lt_carry = 0.
///   carryforward_out TY2025 = { short: 7000, long: 0 }.
///   2026 profile declares carryforward_in = { short: 0, long: 0 } (deliberately wrong).
///   → Advisory fires: "does not match" is in rendered output.
///   2026 TaxTable is not bundled → main outcome is NotComputable(TaxTableMissing); exit 0.
#[test]
fn carryforward_mismatch_advisory_rendered() {
    // Synthetic Coinbase CSV: ST buy+sell in 2025 at a loss.
    let csv_dir = tempfile::tempdir().unwrap();
    let csv_path = csv_dir.path().join("coinbase_st_loss.csv");
    std::fs::write(
        &csv_path,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
st-buy,2025-01-15 12:00:00 UTC,Buy,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n\
st-sell,2025-06-15 12:00:00 UTC,Sell,BTC,1.00000000,USD,40000.00,40000.00,40000.00,0.00,,,\r\n",
    )
    .unwrap();
    let (_dir, vault) = make_vault_with(&csv_path);

    // 2025 profile: Single, OTI=100k, MAGI=100k, QD=0, carryforward_in=0/0.
    cmd::tax::set_profile(
        &vault,
        &pp(),
        2025,
        TaxProfile {
            filing_status: btctax_core::FilingStatus::Single,
            ordinary_taxable_income: dec!(100000),
            magi_excluding_crypto: dec!(100000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward::default(),
        },
    )
    .unwrap();

    // 2026 profile: declares carryforward_in={short:0, long:0} — wrong (should be {7000,0}).
    cmd::tax::set_profile(
        &vault,
        &pp(),
        2026,
        TaxProfile {
            filing_status: btctax_core::FilingStatus::Single,
            ordinary_taxable_income: dec!(100000),
            magi_excluding_crypto: dec!(100000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward::default(), // wrong: {0, 0}
        },
    )
    .unwrap();

    // report --tax-year 2026: main outcome is NotComputable (no TY2026 table); advisory fires.
    let (outcome, advisory, _sched_d, _gift, _se, _appraisal) =
        cmd::tax::report_tax_year(&vault, &pp(), 2026, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(2026, &outcome, advisory.as_deref());

    // Advisory must contain the mismatch message.
    assert!(
        rendered.contains("does not match"),
        "expected 'does not match' advisory in rendered output:\n{rendered}"
    );
    assert!(
        rendered.contains("ADVISORY (M4)"),
        "expected 'ADVISORY (M4)' label in rendered output:\n{rendered}"
    );
    // The advisory string must not be None.
    assert!(
        advisory.is_some(),
        "expected Some(advisory) for mismatched carryforward chain"
    );
    // Exit 0: report_tax_year returns Ok (no panic, no Err propagation).
    // (The ExitCode is driven by main.rs, not tested here, but Ok(()) == exit 0 for this path.)
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
