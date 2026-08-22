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
use btctax_cli::cmd::tax::TaxYearReport;
use btctax_cli::{cmd, eventref, render, Session};
use btctax_core::persistence::load_all;
use btctax_core::{
    form_8283, Carryforward, DonationDetails, EventPayload, FilingStatus, Form8283Section,
    InboundClass, IncomeKind, OutflowClass, SeTaxResult, TaxProfile,
};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime};

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
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
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

/// Like [`write_buy_receive`] but the Receive is in **2024** (a full-return-supported year).
fn write_buy_receive_2024(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_rcv24.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
hb-buy,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
hb-recv,2024-01-01 12:00:00 UTC,Receive,BTC,0.10000000,USD,44000.00,,,,,,\r\n",
    )
    .unwrap();
    p
}

/// Synthetic Coinbase CSV: 2023 buy + **2024** LT sell → $10,000 LT gain (a full-return-supported year).
/// Buy 1 BTC @ 30,000 (2023-01-01); sell 1 BTC @ 40,000 (2024-06-15) → LT gain 10,000.
fn write_lt_sell_2024(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_lt24.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lt-buy,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
lt-sell,2024-06-15 12:00:00 UTC,Sell,BTC,1.00000000,USD,40000.00,40000.00,40000.00,0.00,,,\r\n",
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

/// [pseudo-reconcile T3 / R0-M6] With pseudo mode ON and NO stored profile for the year, the CLI-layer
/// PLACEHOLDER profile (single filer, $0) is injected so the report computes — clearing `TaxProfileMissing`
/// ONLY. With the mode OFF and no profile, the same ledger reports `NotComputable(TaxProfileMissing)`.
#[test]
fn pseudo_mode_injects_placeholder_profile_clearing_tax_profile_missing() {
    use btctax_core::{BlockerKind, TaxOutcome};
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path()); // fully-real buy+sell, NO blockers
    let (_dir, vault) = make_vault_with(&csv);

    // Mode OFF + no profile ⇒ NotComputable(TaxProfileMissing).
    let TaxYearReport {
        outcome: out_off, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    match out_off {
        TaxOutcome::NotComputable(b) => assert_eq!(b.kind, BlockerKind::TaxProfileMissing),
        other => panic!("mode-off, no profile: expected TaxProfileMissing, got {other:?}"),
    }

    // Turn pseudo mode ON, still NO profile ⇒ the placeholder is injected ⇒ Computed.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let TaxYearReport {
        outcome: out_on,
        pseudo_contributed,
        advisory,
        ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    assert!(
        matches!(out_on, TaxOutcome::Computed(_)),
        "pseudo mode must inject a placeholder profile so the year computes: {out_on:?}"
    );
    // UX-P4-1 KAT (b) [G-I1]: this IS the PLACEHOLDER channel (count==0, no stored profile) — the
    // OR-predicate's second disjunct. Deleting the `PseudoPlaceholder` arm in report_tax_year (or reading a
    // pseudo-OFF view) must RED here: the figure would render clean while riding a fictional $0 profile.
    assert_eq!(
        pseudo_contributed,
        render::PseudoDisclosure::Placeholder,
        "pseudo-on + count==0 + no stored profile is the Placeholder channel"
    );
    let rendered =
        render::render_tax_outcome(2025, &out_on, advisory.as_deref(), pseudo_contributed);
    assert!(
        rendered.contains("estimated on a synthetic $0 placeholder profile"),
        "the placeholder-variant banner must lead the render:\n{rendered}"
    );
    assert!(
        rendered
            .lines()
            .any(|l| l.contains("TOTAL federal tax attributable") && l.contains("[PSEUDO]")),
        "the placeholder-channel TOTAL must also carry the [PSEUDO] suffix:\n{rendered}"
    );
}

/// Golden render test: Single, LT gain=20,000, OTI=40k, MAGI excl=60k → total=1,747.50.
/// Asserts the rendered output contains the expected TOTAL line.
#[test]
fn report_tax_year_renders_golden() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile(), false).unwrap();

    let TaxYearReport {
        outcome,
        advisory,
        schedule_d: sched_d,
        ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(
        2025,
        &outcome,
        advisory.as_deref(),
        render::PseudoDisclosure::None,
    );

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

/// [P2-D Task 2 wiring / Chunk A] A business-mining year → `report_tax_year` surfaces the
/// Schedule SE section (components + total + §164(f) half + [Chunk A] the $0-W-2 note + the
/// §164(f) advisory + the standalone note), and the income-tax report's TOTAL is UNCHANGED by
/// SE tax (D5 — SE is standalone).
/// [R0-I2 regression] Old OVERSTATED/UNDERSTATED text ABSENT; new $0 note PRESENT.
#[test]
fn report_tax_year_renders_schedule_se_for_business_mining() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the unclassified Receive as $100,000 BUSINESS mining income (SE-eligible).
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
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

    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile(), false).unwrap();

    let TaxYearReport {
        outcome,
        schedule_se: se,
        ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let se = se.expect("Schedule SE section expected for a business-mining year");

    // Golden 1 components (Single, $100,000 business mining, W-2 = $0).
    assert!(se.contains("11451.40"), "SS component: {se}");
    assert!(se.contains("2678.15"), "Medicare component: {se}");
    assert!(se.contains("14129.55"), "total SE tax: {se}");
    assert!(se.contains("7064.78"), "§164(f) deductible half: {se}");
    // [Chunk A / R0-I2] New $0-W-2 short note present; old OVERSTATED/UNDERSTATED GONE.
    assert!(
        se.contains("$0 W-2 wages"),
        "new $0-W-2 note must be present (profile has no W-2): {se}"
    );
    assert!(
        !se.contains("OVERSTATED"),
        "old OVERSTATED text must be absent (Chunk A regression): {se}"
    );
    assert!(
        !se.contains("UNDERSTATED"),
        "old UNDERSTATED text must be absent (Chunk A regression): {se}"
    );
    // [Chunk A / R0-I3] §164(f) advisory present.
    assert!(
        se.contains("NOT auto-coordinated"),
        "§164(f) advisory must appear: {se}"
    );
    // Standalone note.
    assert!(se.contains("SEPARATE federal liability"), "{se}");
    // [Chunk B] $0-expenses note — old "not modeled" caveat replaced by Chunk B disclosure.
    assert!(
        se.contains("no Schedule C expenses supplied"),
        "Chunk B $0-expenses note must appear in Schedule SE render: {se}"
    );
    assert!(
        !se.contains("not modeled"),
        "old 'not modeled' caveat must be absent (replaced by Chunk B): {se}"
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
    let it = render::render_tax_outcome(2025, &outcome, None, render::PseudoDisclosure::None);
    assert!(
        !it.contains("14129.55"),
        "SE tax must NOT appear in the income-tax report total (standalone, D5):\n{it}"
    );
}

/// [Chunk A / I4] Asymmetric-W-2 transposition guard (CLI path): profile w2_ss $150,000 /
/// w2_medicare $0 → the rendered Schedule SE shows ss $3,236.40 AND addl $0.00.
/// A swapped (w2_medicare, w2_ss) call order would flip both → ss $11,451.40 / addl $381.15.
/// Exercises the REAL cmd/tax.rs call site, not just the render unit KAT.
#[test]
fn chunk_a_asymmetric_w2_transposition_guard_cli_path() {
    use btctax_core::TaxProfile;
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the unclassified Receive as $100,000 BUSINESS mining income.
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(100000.00)),
            business: true,
        },
        datetime!(2025-06-01 00:00:00 UTC),
    )
    .unwrap();

    // Asymmetric profile: w2_ss_wages $150k, w2_medicare_wages $0.
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(150000),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, profile, false).unwrap();

    let TaxYearReport {
        schedule_se: se, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let se = se.expect("Schedule SE section expected");

    // ss must be $3,236.40 (reduced by w2_ss $150k) — NOT $11,451.40 (transposition).
    assert!(
        se.contains("3236.40"),
        "ss must be 3236.40 (w2_ss reduced cap): {se}"
    );
    assert!(
        !se.contains("11451.40"),
        "ss must NOT be 11451.40 (transposition would give this): {se}"
    );
    // addl must be $0.00 (threshold un-reduced, base < $200k) — NOT $381.15 (transposition).
    // Note: 0.00 appears in the Additional Medicare component line.
    assert!(
        se.contains("W-2 coordination applied"),
        "coordinated disclosure must appear: {se}"
    );
    assert!(
        !se.contains("381.15"),
        "addl must NOT be 381.15 (transposition would give this): {se}"
    );
}

/// [Chunk A / I1+M6] Export-path parity: the asymmetric profile (w2_ss $150k, w2_medicare $0)
/// produces schedule_se.csv figures that EQUAL the report figures (cmd/admin.rs call site).
/// If cmd/admin.rs defaulted W-2 to $0, the CSV would show $11,451.40 while the report shows
/// $3,236.40 — this test catches that divergence.
#[test]
fn chunk_a_export_parity_asymmetric_w2() {
    use btctax_core::TaxProfile;
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the Receive as $100,000 BUSINESS mining income.
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(100000.00)),
            business: true,
        },
        datetime!(2025-06-01 00:00:00 UTC),
    )
    .unwrap();

    // Asymmetric profile: w2_ss $150k, w2_medicare $0.
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(150000),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, profile, false).unwrap();

    // Get report figures (cmd/tax.rs call site).
    let TaxYearReport {
        schedule_se: se, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let se_text = se.expect("Schedule SE section expected");
    // Report shows reduced SS.
    assert!(
        se_text.contains("3236.40"),
        "report must show reduced ss $3,236.40: {se_text}"
    );

    // Get export figures (cmd/admin.rs call site) and compare with report.
    let export_dir = tempfile::tempdir().unwrap();
    cmd::admin::export_snapshot(&vault, &pp(), export_dir.path(), Some(2025), None).unwrap();
    let csv_path = export_dir.path().join("schedule_se.csv");
    assert!(csv_path.exists(), "schedule_se.csv must be written");
    let content = std::fs::read_to_string(&csv_path).unwrap();
    // The CSV ss_component must match the report ($3,236.40, not $11,451.40).
    assert!(
        content.contains("3236.40"),
        "schedule_se.csv ss_component must equal report figure $3,236.40 (not 11451.40): {content}"
    );
    assert!(
        !content.contains("11451.40"),
        "schedule_se.csv must NOT show the un-coordinated ss $11,451.40: {content}"
    );
}

/// [B-M1 Task 2 / Task 1 NII-interest D2] The Computed tax-report footer carries accurate NII
/// disclosures: §1211(b) loss applied; crypto-lending interest IS INCLUDED; mining/staking/etc.
/// remain excluded. The old "cannot yet isolate" / residual-understatement language is GONE.
///
/// [R0-N1 — SEMANTIC] `contains("crypto-lending interest")` alone would pass on BOTH old and
/// new text — must assert the NEW phrase ("is INCLUDED in NII") AND assert the old isolation
/// disclaimer is absent.
#[test]
fn report_tax_year_footer_discloses_1211_loss_and_interest_nii_included() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile(), false).unwrap();

    let TaxYearReport {
        outcome, advisory, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(
        2025,
        &outcome,
        advisory.as_deref(),
        render::PseudoDisclosure::None,
    );

    // B-M1 negatives (wrong-direction language must be absent):
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
    // [R0-N1] The old isolation disclaimer must be GONE:
    assert!(
        !rendered.contains("cannot yet isolate"),
        "footer must not say 'cannot yet isolate' — interest is now included:\n{rendered}"
    );
    // Accurate position present:
    assert!(
        rendered.contains("reduces NII by the §1211(b)-allowed net capital loss"),
        "footer must state the §1211(b) loss is applied to NII:\n{rendered}"
    );
    assert!(
        rendered.contains("crypto-lending interest"),
        "footer must reference crypto-lending interest:\n{rendered}"
    );
    // [R0-N1 SEMANTIC] The NEW phrase pins the correct state — passes ONLY on the new text:
    assert!(
        rendered.contains("is INCLUDED in NII"),
        "footer must state crypto-lending interest is INCLUDED in NII:\n{rendered}"
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
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile(), false).unwrap();

    let TaxYearReport {
        outcome, advisory, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(
        2025,
        &outcome,
        advisory.as_deref(),
        render::PseudoDisclosure::None,
    );

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

    let TaxYearReport {
        outcome, advisory, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(
        2025,
        &outcome,
        advisory.as_deref(),
        render::PseudoDisclosure::None,
    );

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

/// [full-return P2 / review R2-M2] `ReturnInputs` for a year v1 does NOT support (TY2025 — no bundled
/// full-return tables) must not silently compute a profile-less number: `report_tax_year` returns a
/// `Usage` error explaining the unsupported year and pointing at the real `income clear --year N` recovery.
#[test]
fn report_tax_year_with_return_inputs_for_unsupported_year_refuses_with_income_clear_hint() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let toml = dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\ntaxpayer_died_during_year = false\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2025, &toml, false).unwrap();

    let err = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap_err();
    match err {
        btctax_cli::CliError::Usage(msg) => {
            assert!(
                msg.contains("full-return"),
                "message should explain why: {msg}"
            );
            assert!(
                msg.contains("income clear --year 2025"),
                "message must point at the working recovery command: {msg}"
            );
        }
        other => panic!("expected a Usage error for the unsupported year, got {other:?}"),
    }

    // Recovery works: after `income clear`, the same year is no longer blocked (falls back to no-profile).
    assert!(cmd::tax::clear_return_inputs(&vault, &pp(), 2025).unwrap());
    let TaxYearReport { outcome, .. } =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    // No profile, no events ⇒ TaxProfileMissing (NOT the unsupported-year Usage error).
    assert!(matches!(outcome, btctax_core::TaxOutcome::NotComputable(_)));
}

/// [full-return P2 task 5] The headline: importing TY2024 `ReturnInputs` makes `report_tax_year` COMPUTE
/// (via the derived frozen profile) instead of refusing — no `tax-profile set` needed. With a real crypto
/// disposal in the ledger, the outcome is `Computed` and the derived Single profile drives the delta.
#[test]
fn report_tax_year_derives_and_computes_from_ty2024_return_inputs() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path()); // a fully-real buy+sell (no blockers)
    let (_dir, vault) = make_vault_with(&csv);

    // No `tax-profile set` — only full-return inputs. A modest wage household for TY2025 (the CSV year).
    let toml = _dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\nforeign_accounts = false\nforeign_trust = false\ndual_status_alien = false\nhas_income_exclusion = false\nother_out_of_scope_income = false\nfiling_form_4952 = false\n\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\ntaxpayer_died_during_year = false\n\n[sch1]\nhsa_activity = false\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\nbox1_wages = \"90000\"\nbox2_fed_withheld = \"12000\"\nbox5_medicare_wages = \"90000\"\n",
    )
    .unwrap();
    // The CSV disposal is in 2025, but v1 full-return tables are TY2024-only; import for 2024 to exercise
    // the derive+compute happy path (the ledger has no 2024 disposals → a clean profile-only computation).
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap();

    let TaxYearReport { outcome, .. } =
        cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap();
    // Derived a real profile ⇒ Computed (NOT NotComputable(TaxProfileMissing), NOT a Usage refusal).
    assert!(
        matches!(outcome, btctax_core::TaxOutcome::Computed(_)),
        "TY2024 ReturnInputs must derive+compute, got {outcome:?}"
    );
}

/// [full-return P2 task 2] The compute-dependent refuse rows gate `report_tax_year` fail-closed: a ledger
/// with SE-eligible business crypto income but `ReturnInputs` carrying no Schedule C must REFUSE (owner /
/// description unknowable) rather than compute a wrong number.
#[test]
fn report_tax_year_refuses_business_income_without_schedule_c() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the 2024 Receive as $100,000 BUSINESS mining income (SE-eligible).
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(100000.00)),
            business: true,
        },
        datetime!(2024-06-01 00:00:00 UTC),
    )
    .unwrap();

    // Full-return inputs for 2024 with NO Schedule C.
    let toml = _dir.path().join("inputs.toml");
    std::fs::write(&toml, "filing_status = \"Single\"\nforeign_accounts = false\nforeign_trust = false\ndual_status_alien = false\nhas_income_exclusion = false\nother_out_of_scope_income = false\nfiling_form_4952 = false\n\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\ntaxpayer_died_during_year = false\n\n[sch1]\nhsa_activity = false\n").unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap();

    let err = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap_err();
    match err {
        btctax_cli::CliError::Usage(msg) => {
            assert!(
                msg.contains("Schedule C"),
                "should name the missing Schedule C: {msg}"
            );
            assert!(
                msg.contains("income clear --year 2024"),
                "recovery hint: {msg}"
            );
        }
        other => {
            panic!("expected a Usage refusal for business-income-without-Schedule-C, got {other:?}")
        }
    }
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
    cmd::tax::set_profile(&vault, &pp(), 2025, single_40k_profile(), false).unwrap();

    let TaxYearReport {
        outcome,
        advisory,
        schedule_d: sched_d,
        ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(
        2025,
        &outcome,
        advisory.as_deref(),
        render::PseudoDisclosure::None,
    );

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

/// M4 (Task 10): when the declared 2027 `carryforward_in` ≠ 2026's computed `carryforward_out`,
/// `report --tax-year 2027` renders the advisory line. (2027 is NOT COMPUTABLE, so the BINARY now
/// exits 1 per UX-P4-10; this lib-level test asserts only that the advisory renders.)
///
/// (Re-pointed from 2026→2027 after TY2026 was bundled: 2026 now COMPUTES, so it can no longer be
/// the "unbundled → NotComputable" year. The WHOLE scenario shifts forward one year — the loss and
/// its carryforward must stay in the prior-year CSV, else the mismatch zeroes out and this fails.)
///
/// Scenario derivation (hand-verified):
///   2026 vault: buy 1 BTC @ $50,000 on 2026-01-15 (ST); sell 1 BTC @ $40,000 on 2026-06-15 (ST).
///   → crypto_st = −$10,000; no LT; no carryforward-in declared in the 2026 profile.
///   net_1222(−10000, 0, 0, 0, 0, 3000):
///     st_net = −10000; lt_net = 0; both losses cross-net: no cross.
///     loss_deduction = min(10000, 3000) = 3000; absorbed_st = 3000; absorbed_lt = 0.
///     st_carry = 10000 − 3000 = 7000; lt_carry = 0.
///   carryforward_out TY2026 = { short: 7000, long: 0 } (2026 IS bundled, so the prior year computes).
///   2027 profile declares carryforward_in = { short: 0, long: 0 } (deliberately wrong).
///   → Advisory fires: "does not match" is in rendered output.
///   2027 TaxTable is not bundled → main outcome is NotComputable(TaxTableMissing); binary exits 1 (UX-P4-10).
#[test]
fn carryforward_mismatch_advisory_rendered() {
    // Synthetic Coinbase CSV: ST buy+sell in 2026 at a loss.
    let csv_dir = tempfile::tempdir().unwrap();
    let csv_path = csv_dir.path().join("coinbase_st_loss.csv");
    std::fs::write(
        &csv_path,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
st-buy,2026-01-15 12:00:00 UTC,Buy,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n\
st-sell,2026-06-15 12:00:00 UTC,Sell,BTC,1.00000000,USD,40000.00,40000.00,40000.00,0.00,,,\r\n",
    )
    .unwrap();
    let (_dir, vault) = make_vault_with(&csv_path);

    // 2026 profile: Single, OTI=100k, MAGI=100k, QD=0, carryforward_in=0/0 (the prior/loss year).
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
            capital_loss_carryforward_in: Carryforward::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        },
        false,
    )
    .unwrap();

    // 2027 profile: declares carryforward_in={short:0, long:0} — wrong (should be {7000,0}).
    cmd::tax::set_profile(
        &vault,
        &pp(),
        2027,
        TaxProfile {
            filing_status: btctax_core::FilingStatus::Single,
            ordinary_taxable_income: dec!(100000),
            magi_excluding_crypto: dec!(100000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward::default(), // wrong: {0, 0}
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        },
        false,
    )
    .unwrap();

    // report --tax-year 2027: main outcome is NotComputable (no TY2027 table); advisory fires.
    let TaxYearReport {
        outcome, advisory, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2027, dec!(0)).unwrap();
    let rendered = render::render_tax_outcome(
        2027,
        &outcome,
        advisory.as_deref(),
        render::PseudoDisclosure::None,
    );

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
    // report_tax_year returns Ok (no panic, no Err propagation).
    // (The binary exit code is driven by main.rs: a NotComputable 2027 outcome exits 1 (UX-P4-10) —
    // not tested here; this lib-level test asserts only the advisory. See report_exit_code.rs.)
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

/// [R0-M3] Negative --prior-taxable-gifts WITHOUT --tax-year is rejected at the binary level.
///
/// The validation guard in `main.rs` runs BEFORE the `if let Some(y) = tax_year` branch, so
/// the rejection fires even on the display-only (no --tax-year) path.
///
/// **This test FAILS if the guard is moved INSIDE the `if let Some(y) = tax_year` block.**
/// Without `--tax-year`, the display path would be taken instead of the validation, and the
/// binary would exit 0 rather than 2 (usage error).
///
/// Pattern: `std::process::Command` over the compiled binary (same as `fr9_exit_code.rs`).
/// `CARGO_BIN_EXE_btctax` is set by Cargo for integration-test binaries.
#[test]
fn report_negative_prior_taxable_gifts_rejected_without_tax_year() {
    // Minimal vault: just init (no events needed — validation fires before any projection).
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Drive the REAL binary: `report --prior-taxable-gifts=-5` with NO `--tax-year`.
    let bin = env!("CARGO_BIN_EXE_btctax");
    let status = std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().expect("vault path is valid UTF-8"),
            "report",
            "--prior-taxable-gifts=-5",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("btctax binary must execute successfully");
    assert_ne!(
        status.code().unwrap_or(0),
        0,
        "btctax report --prior-taxable-gifts=-5 (no --tax-year) must exit non-zero; \
         if it exits 0 the validation guard has regressed into the --tax-year branch"
    );
}

/// Write a synthetic Coinbase CSV with an LT buy (2022) and a Send (2024) for the M2 seam test.
/// The Send becomes a pending TransferOut; once classified as Donate with FMV=$10,000 the LT lot
/// produces claimed_deduction = FMV = $10,000, which exceeds $5,000 → Section B.
fn write_lt_buy_send_2024(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_donate_seam.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
seam-buy,2022-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
seam-donate-send,2024-03-01 12:00:00 UTC,Send,BTC,0.50000000,USD,20000.00,,,,,,bc1qsyntheticcharity\r\n",
    )
    .unwrap();
    p
}

/// [M2] End-to-end seam: set-donation-details → session.donation_details() → form_8283 carrier.
///
/// Creates a Section-B-scale vault (LT buy 2022; Send classified as Donate FMV=$10,000 on
/// 2024-03-01; LT lot → claimed_deduction = FMV = $10,000 > $5,000 Section-B threshold).
/// Stores full §6695A details via `set_donation_details`, reads back via `session.donation_details()`,
/// drives `form_8283`, and asserts the carrier row:
///   - `needs_review == false`  (all §6695A fields present → `is_review_complete(B) == true`)
///   - `appraiser == "Test Appraiser Seam"`
///   - `donee == "Test Charity Seam"` (from DonationDetails.donee_name, not Removal.donee)
///   - `section == Some(Form8283Section::B)`
///
/// This locks the EventId canonical→reparse seam between the side-table and the form lookup:
/// the EventId stored in donation_details must match the removal's event in the projected state.
#[test]
fn e2e_donation_details_seam_form_8283_carrier() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_buy_send_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Find the pending TransferOut event (the Send) → get its canonical event ID.
    let send_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferOut(_)))
            .unwrap()
            .id
            .canonical()
    };

    // Classify the Send as Donate with FMV=$10,000 (LT lot → deduction=$10,000 > $5k Section B).
    let now = datetime!(2024-03-01 12:00:00 UTC);
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &send_ref,
        OutflowClass::Donate {
            appraisal_required: true,
        },
        dec!(10000),
        None,
        Some("Test Charity Seam".into()),
        now,
    )
    .unwrap();

    // Store full §6695A details (all Section-B completeness fields present).
    let details = DonationDetails {
        donee_name: "Test Charity Seam".into(),
        donee_address: Some("456 Charity Ave, Anytown USA".into()),
        donee_ein: Some("99-1234567".into()),
        appraiser_name: "Test Appraiser Seam".into(),
        appraiser_address: Some("789 Appraiser Blvd".into()),
        appraiser_tin: Some("111223333".into()),
        appraiser_ptin: Some("P09876543".into()),
        appraiser_qualifications: Some("Certified digital asset appraiser, 10 yrs".into()),
        appraisal_date: Some(date!(2024 - 02 - 15)),
        fmv_method_override: None,
    };
    cmd::reconcile::set_donation_details(&vault, &pp(), &send_ref, details).unwrap();

    // Read back via session.donation_details() → locks the EventId canonical→reparse seam.
    let session = Session::open(&vault, &pp()).unwrap();
    let stored_map = session.donation_details().unwrap();
    let send_id = eventref::parse_event_id(&send_ref).unwrap();
    let stored = stored_map
        .get(&send_id)
        .expect("donation details must be present after set");
    assert_eq!(
        stored.donee_name, "Test Charity Seam",
        "donee_name must survive the EventId canonical→reparse seam"
    );
    assert_eq!(
        stored.appraiser_name, "Test Appraiser Seam",
        "appraiser_name must survive the EventId canonical→reparse seam"
    );

    // Project state + call form_8283 → assert the carrier row.
    let (state, _cfg) = session.project().unwrap();
    let rows = form_8283(&state, 2024, &stored_map);
    let carrier = rows
        .iter()
        .find(|r| r.section.is_some())
        .expect("one carrier row with Some(section) must be present");

    assert!(
        !carrier.needs_review,
        "full §6695A fields must flip needs_review to false on the carrier row: {carrier:?}"
    );
    assert_eq!(
        carrier.appraiser, "Test Appraiser Seam",
        "appraiser must be populated from DonationDetails on the carrier"
    );
    assert_eq!(
        carrier.donee, "Test Charity Seam",
        "donee must come from DonationDetails.donee_name (not Removal.donee) on the carrier"
    );
    assert_eq!(
        carrier.section,
        Some(Form8283Section::B),
        "LT donation with FMV=$10,000 > $5,000 threshold must be Section B"
    );
}

/// [Chunk B / I2] Expensed-profile parity: profile with `schedule_c_expenses=$20,000` + business
/// mining income $100,000 → `report_tax_year` surfaces the expensed SE figures (ss $9,161.12,
/// total $11,303.64, gross/net breakout line) AND the `schedule_se.csv` export carries the SAME
/// figures (parity between the report and the admin CSV path).
///
/// Golden values (Single, mining $100,000, expenses $20,000, no W-2, TY2025 bundled table):
///   net_se = max(0, 100,000 − 20,000) = 80,000;
///   base = round_cents(80,000 × 0.9235) = 73,880.00;
///   ss = round_cents(12.4% × 73,880) = 9,161.12;
///   medicare = round_cents(2.9% × 73,880) = 2,142.52;
///   total = 9,161.12 + 2,142.52 = 11,303.64.
///
/// Parity contract: schedule_se.csv `ss_component` + `total_se_tax` must equal the report figures
/// — locked against divergent call sites in cmd/tax.rs vs cmd/admin.rs.
#[test]
fn chunkb_expensed_profile_report_and_csv_parity() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the unclassified Receive as $100,000 BUSINESS mining income.
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(100000.00)),
            business: true,
        },
        datetime!(2025-01-01 00:00:00 UTC),
    )
    .unwrap();

    // Profile: $20,000 Schedule C expenses.
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(20000),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, profile, false).unwrap();

    // ── Report path ────────────────────────────────────────────────────────────────────────────
    let TaxYearReport {
        schedule_se: se, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let se_text = se.expect("Schedule SE section expected with $20,000 expenses");

    // Breakout line: gross − expenses = net SE.
    assert!(
        se_text.contains("gross business income"),
        "breakout line must appear in report: {se_text}"
    );
    assert!(
        se_text.contains("100000.00"),
        "gross $100,000 must appear in breakout: {se_text}"
    );
    assert!(
        se_text.contains("20000.00"),
        "expenses $20,000 must appear in breakout: {se_text}"
    );
    assert!(
        se_text.contains("80000.00"),
        "net SE $80,000 must appear in breakout: {se_text}"
    );
    // Golden figures.
    assert!(
        se_text.contains("9161.12"),
        "SS component $9,161.12 must appear in report: {se_text}"
    );
    assert!(
        se_text.contains("11303.64"),
        "total SE $11,303.64 must appear in report: {se_text}"
    );

    // ── CSV export path ────────────────────────────────────────────────────────────────────────
    let export_dir = tempfile::tempdir().unwrap();
    cmd::admin::export_snapshot(&vault, &pp(), export_dir.path(), Some(2025), None).unwrap();
    let csv_path = export_dir.path().join("schedule_se.csv");
    assert!(
        csv_path.exists(),
        "schedule_se.csv must be written for expensed case"
    );
    let content = std::fs::read_to_string(&csv_path).unwrap();
    // CSV parity: same figures as the report.
    assert!(
        content.contains("9161.12"),
        "schedule_se.csv ss_component must match report ($9,161.12): {content}"
    );
    assert!(
        content.contains("11303.64"),
        "schedule_se.csv total_se_tax must match report ($11,303.64): {content}"
    );
}

/// [Chunk B / I2 — fully-expensed] Expenses ≥ gross → `report_tax_year` renders the
/// "fully expensed … no §1401 SE tax" line (not the wage-base-unavailable note) AND no
/// `schedule_se.csv` is written by the export path (se_result=None → CSV omitted).
///
/// Fixture: business Mining income $10,000, Schedule C expenses $15,000 → net_se = max(0,
/// 10,000 − 15,000) = 0 → `compute_se_tax` returns `None`. The render must show the
/// "fully expensed" line, NOT the "SS wage base unavailable" note.
#[test]
fn chunkb_fully_expensed_integration_no_se_tax_no_csv() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // Classify the Receive as $10,000 BUSINESS mining income (small enough to be fully expensed).
    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(10000.00)),
            business: true,
        },
        datetime!(2025-01-01 00:00:00 UTC),
    )
    .unwrap();

    // Profile: expenses $15,000 ≥ gross $10,000 → fully expensed.
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(15000),
    };
    cmd::tax::set_profile(&vault, &pp(), 2025, profile, false).unwrap();

    // ── Report path: "fully expensed" line present; wage-base note absent ──────────────────────
    let TaxYearReport {
        schedule_se: se, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    let se_text = se.expect("Schedule SE section expected (gross > 0) even when fully expensed");

    assert!(
        se_text.contains("fully expensed"),
        "report must show fully-expensed line: {se_text}"
    );
    assert!(
        se_text.contains("no \u{00a7}1401 SE tax"),
        "report must state no SE tax owed: {se_text}"
    );
    assert!(
        !se_text.contains("SS wage base unavailable"),
        "wage-base-unavailable note must be ABSENT (not a table-missing case): {se_text}"
    );

    // ── Export path: no schedule_se.csv written (se_result=None) ──────────────────────────────
    let export_dir = tempfile::tempdir().unwrap();
    cmd::admin::export_snapshot(&vault, &pp(), export_dir.path(), Some(2025), None).unwrap();
    let csv_path = export_dir.path().join("schedule_se.csv");
    assert!(
        !csv_path.exists(),
        "schedule_se.csv must NOT be written for fully-expensed case (no SE tax owed)"
    );
}

/// [Chunk B / Minor] Negative `--schedule-c-expenses` rejected at the binary level.
///
/// Calls `btctax tax-profile --year 2025 ... --schedule-c-expenses=-5` with all mandatory profile
/// fields supplied — ensuring the guard at the Schedule-C-expenses validation step (not a
/// "required field" guard) is what produces the error. Binary must exit non-zero.
///
/// Pattern: `std::process::Command` over the compiled binary (same as `fr9_exit_code.rs`).
/// `CARGO_BIN_EXE_btctax` is set by Cargo for integration-test binaries.
#[test]
fn tax_profile_negative_schedule_c_expenses_rejected() {
    // Minimal vault: just init (validation fires before any vault write).
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let bin = env!("CARGO_BIN_EXE_btctax");
    let status = std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().expect("vault path is valid UTF-8"),
            "tax-profile",
            "--year",
            "2025",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "40000",
            "--magi-excluding-crypto",
            "60000",
            "--qualified-dividends",
            "0",
            "--schedule-c-expenses=-5",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("btctax binary must execute successfully");
    assert_ne!(
        status.code().unwrap_or(0),
        0,
        "btctax tax-profile --schedule-c-expenses=-5 must exit non-zero; \
         if it exits 0 the negative-value guard has been removed"
    );
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// [burndown-3 D2] §6017 $400 filing-floor note KATs
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// [burndown-3 D2] §6017 floor note appears when base < $400 — including the exact case where
/// the PRE-factor profit exceeds $400: net_se $430 → base = round_cents(430 × 0.9235) = $397.10
/// (the raw product $397.1050 is an exact tie; house ROUND_HALF_EVEN resolves to the even cent
/// — $397.10, NOT $397.11). The §6017/§1402(b)(2) test is on the ×0.9235 base.
#[test]
fn schedule_se_6017_floor_note_present_below_400_base() {
    // Pin the ROUND_HALF_EVEN tie-break directly (R0-M1 claim).
    assert_eq!(
        btctax_core::conventions::round_cents(dec!(430.00) * dec!(0.9235)),
        dec!(397.10),
        "ROUND_HALF_EVEN: the exact tie 397.1050 resolves to the even cent"
    );

    let r = SeTaxResult {
        net_se: dec!(430.00),
        base: dec!(397.10),
        ss: dec!(49.24),
        medicare: dec!(11.52),
        addl: dec!(0),
        total: dec!(60.76),
        deductible_half: dec!(30.38),
    };
    let s = render::render_schedule_se(
        2025,
        Some(&r),
        dec!(430.00),
        true,
        dec!(0),
        dec!(0),
        dec!(0),
    )
    .expect("Some(r) arm always renders");
    assert!(
        s.contains("(§6017 filing floor)"),
        "§6017 note must be present when base < $400:\n{s}"
    );
    assert!(
        s.contains("397.10"),
        "the note must show the ×0.9235 base (397.10, ROUND_HALF_EVEN):\n{s}"
    );
    assert!(
        s.contains("§1402(j)(2)"),
        "the church-employee carve-out must be present:\n{s}"
    );
}

/// [burndown-3 D2] §6017 floor note is ABSENT when base ≥ $400 (boundary: exactly $400 is
/// "400 or more" — no note).
#[test]
fn schedule_se_6017_floor_note_absent_at_or_above_400_base() {
    let r = SeTaxResult {
        net_se: dec!(433.14),
        base: dec!(400.00), // exactly at the floor → §6017 filing required → NO note
        ss: dec!(49.60),
        medicare: dec!(11.60),
        addl: dec!(0),
        total: dec!(61.20),
        deductible_half: dec!(30.60),
    };
    let s = render::render_schedule_se(
        2025,
        Some(&r),
        dec!(433.14),
        true,
        dec!(0),
        dec!(0),
        dec!(0),
    )
    .expect("Some(r) arm always renders");
    assert!(
        !s.contains("§6017"),
        "no §6017 note at base == $400 (strict <):\n{s}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// [burndown-3 D3] Negative-flag binary tests — W-2 wage guards
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// [burndown-3 D3] Negative `--w2-ss-wages` rejected at the binary level.
///
/// Drives the REAL binary via `env!("CARGO_BIN_EXE_btctax")` and locks the
/// `--w2-ss-wages must not be negative` guard (main.rs:738-740, Chunk-A M-1 discharge).
#[test]
fn tax_profile_negative_w2_ss_wages_rejected() {
    // Minimal vault: just init (validation fires before any vault write).
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let bin = env!("CARGO_BIN_EXE_btctax");
    let status = std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().expect("vault path is valid UTF-8"),
            "tax-profile",
            "--year",
            "2025",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "40000",
            "--magi-excluding-crypto",
            "60000",
            "--qualified-dividends",
            "0",
            "--w2-ss-wages=-5",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("btctax binary must execute successfully");
    assert_ne!(
        status.code().unwrap_or(0),
        0,
        "btctax tax-profile --w2-ss-wages=-5 must exit non-zero; \
         if it exits 0 the negative-value guard has been removed"
    );
}

/// [burndown-3 D3] Negative `--w2-medicare-wages` rejected at the binary level.
///
/// Drives the REAL binary via `env!("CARGO_BIN_EXE_btctax")` and locks the
/// `--w2-medicare-wages must not be negative` guard (main.rs:746-750, Chunk-A M-1 discharge).
#[test]
fn tax_profile_negative_w2_medicare_wages_rejected() {
    // Minimal vault: just init (validation fires before any vault write).
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let bin = env!("CARGO_BIN_EXE_btctax");
    let status = std::process::Command::new(bin)
        .args([
            "--vault",
            vault.to_str().expect("vault path is valid UTF-8"),
            "tax-profile",
            "--year",
            "2025",
            "--filing-status",
            "single",
            "--ordinary-taxable-income",
            "40000",
            "--magi-excluding-crypto",
            "60000",
            "--qualified-dividends",
            "0",
            "--w2-medicare-wages=-5",
        ])
        .env("BTCTAX_PASSPHRASE", "pw")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("btctax binary must execute successfully");
    assert_ne!(
        status.code().unwrap_or(0),
        0,
        "btctax tax-profile --w2-medicare-wages=-5 must exit non-zero; \
         if it exits 0 the negative-value guard has been removed"
    );
}

/// §6 DUAL REPORT: a `ReturnInputs`-provenance TY2024 year renders the absolute filed 1040 (income →
/// total tax → refund/owed) side-by-side with the crypto delta, carrying the §6 "different questions /
/// not reconciled" labels + the §4.12 provenance line. A non-ReturnInputs (stored-profile) year does not.
#[test]
fn dual_report_renders_absolute_return_with_section_6_labels() {
    use btctax_core::tax::return_inputs::{Owner, ReturnInputs, W2};
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path()); // clean 2023 buy + 2024 LT sell (+$10,000 LTCG)
    let (_dir, vault) = make_vault_with(&csv);
    // ReturnInputs for 2024: Single, $80k wages, no Schedule A (standard deduction).
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(80000),
                    box3_ss_wages: dec!(80000),
                    box5_medicare_wages: dec!(80000),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    let TaxYearReport {
        dual_report: dual, ..
    } = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap();
    let dual = dual.expect("a ReturnInputs-provenance year must produce the §6 dual report");
    // Absolute filed-return lines.
    assert!(
        dual.contains("Absolute filed return (Form 1040) — tax year 2024"),
        "{dual}"
    );
    assert!(dual.contains("TOTAL TAX (L24)"), "{dual}");
    assert!(
        dual.contains("90000.00"),
        "AGI = 80,000 wages + 10,000 LTCG:\n{dual}"
    );
    // §4.12 provenance printed.
    assert!(dual.contains("Profile source: ReturnInputs"), "{dual}");
    // §6 never-reconcile labels + both figures side-by-side.
    assert!(
        dual.contains("DIFFERENT questions") && dual.contains("NOT reconciled"),
        "{dual}"
    );
    assert!(
        dual.contains("Absolute TOTAL TAX") && dual.contains("DELTA"),
        "{dual}"
    );

    // A stored-profile (non-ReturnInputs) year produces NO dual report (the delta-only path).
    let csv_dir2 = tempfile::tempdir().unwrap();
    let csv2 = write_lt_sell_2025(csv_dir2.path());
    let (_dir2, vault2) = make_vault_with(&csv2);
    cmd::tax::set_profile(&vault2, &pp(), 2025, single_40k_profile(), false).unwrap();
    let TaxYearReport {
        dual_report: dual2, ..
    } = cmd::tax::report_tax_year(&vault2, &pp(), 2025, dec!(0)).unwrap();
    assert!(
        dual2.is_none(),
        "a stored-profile year must not emit the dual report"
    );
}

/// ★★★ **K9 — rolling a carryover must never leave next year unfilable IN SILENCE.**
///
/// A nonzero capital-loss carryover-in makes three class-(A) declarations live on year Y+1 that were
/// not live before it: Form 6251 line 2k, and the Capital Loss Carryover Worksheet's two header
/// conditions. All three refuse when `None`. `--write-carryover` is therefore the ONLY write path in
/// btctax that can leave a COMMITTED row in a state `input_form_store` would have refused to create —
/// an invariant that held until (B) landed.
///
/// The command WARNS rather than refuses, and that is not softness: the row must exist for the answer
/// to be givable at all. Refusing would leave the filer with a written row, a refusal, and no command
/// that reaches it.
///
/// Mutations that MUST red:
///   (a) delete the re-screen ⇒ the note vanishes;
///   (b) point `carryforward_in_present` at `|_| false` ⇒ the fixture stops going unfilable, which
///       is the check that this test reads the REAL liveness predicate and not a hand-copy of it.
#[test]
fn rolling_a_carryover_never_leaves_next_year_unfilable_in_silence() {
    use btctax_core::tax::return_inputs::{Owner, ReturnInputs, W2};
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // 2024: $30,000 of wages and a $60,000 long-term loss carried in ⇒ a real carryover-out.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut y2024 = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(30000),
                box3_ss_wages: dec!(30000),
                box5_medicare_wages: dec!(30000),
                ..Default::default()
            }],
            ..Default::default()
        };
        y2024.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
            short: rust_decimal::Decimal::ZERO,
            long: dec!(60000),
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut y2024);
        btctax_cli::return_inputs::set(s.conn(), 2024, &y2024).unwrap();

        // 2025: a plain Single row that screens CLEAN today.
        // ★ It must STATE its tax year. `Default` gives 0, and a year-scoped question is correctly
        //   NOT live without one — so `answered()` would leave it blank and the stored row would
        //   already refuse, making the (before-clean, after-refusing) delta unreachable. The premise
        //   assertion below is what turns that into a visible failure instead of a silent pass.
        let y2025 = btctax_core::tax::testonly::answered(ReturnInputs {
            tax_year: 2025,
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        });
        btctax_cli::return_inputs::set(s.conn(), 2025, &y2025).unwrap();
        s.save().unwrap();
    }

    // PREMISE: the 2025 row screens CLEAN before the write, or the delta this test is about cannot
    // be produced and a silent pass is indistinguishable from a working warning.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let stored = btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap();
        assert!(
            btctax_core::tax::return_refuse::screen_inputs(
                &stored,
                &btctax_core::tax::testonly::ty2024_table(),
                &btctax_core::tax::testonly::ty2024_params(),
            )
            .is_none(),
            "premise: the 2025 row must be filable BEFORE the roll"
        );
    }

    let summary = cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();

    // PREMISE: a carryover really was written, or "it went unfilable" has no cause.
    let next = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
    };
    assert!(
        next.capital_loss_carryforward_in.long > rust_decimal::Decimal::ZERO,
        "premise: the roll must have put a nonzero carryover on the 2025 row: {:?}",
        next.capital_loss_carryforward_in
    );
    assert_eq!(
        next.carryover_includes_spouses_joint_loss, None,
        "premise: …and the worksheet's header declarations are unanswered on that row, which is the \
         state that makes it unfilable"
    );

    assert!(
        summary.contains("now needs an answer it did not need before"),
        "★ the filer must be TOLD: {summary}"
    );
    assert!(
        summary.contains("btctax income answer"),
        "★ …and told the command that fixes it: {summary}"
    );
    assert!(summary.contains("2025"), "★ …for the right year: {summary}");
}

/// ★★★ **K11 — the write-back summary names every carryover it wrote, and REDS ON AN ADDITION.**
///
/// The previous summary was a hand-written sentence naming two carryovers where the code wrote
/// three. A checker that reds only when something is REMOVED is that same hand-list wearing a test:
/// it cannot see the case that actually happened.
///
/// So the observed set is DERIVED — the year Y+1 row is serialized before and after the write-back and
/// diffed at the leaf level (the mutate-and-diff technique `spec::coverage` uses) — and compared
/// against a pinned table of `leaf path → the phrase the summary must carry`. A FIFTH assigned field
/// then produces a leaf path the table does not have, and the equality reds on the ADDITION.
///
/// Mutations that MUST red:
///   (a) assign a fifth field in `apply_carryover_writeback` without touching the summary ⇒ the
///       derived path set gains an entry the table lacks;
///   (b) drop any phrase from the summary ⇒ the second half.
#[test]
fn the_writeback_summary_names_every_carryover_it_wrote() {
    use btctax_core::tax::return_inputs::{Owner, ReturnInputs, W2};
    use std::collections::BTreeMap;

    /// `leaf path → the substring the summary must carry for it`. PINNED; the observed set is not.
    const EXPECTED: &[(&str, &str)] = &[
        ("charitable_carryover_in", "charitable carryover item(s)"),
        (
            "charitable_carryover_in_provenance",
            "charitable carryover item(s)",
        ),
        ("qbi.reit_ptp_carryforward_in", "QBI REIT/PTP carryforward"),
        (
            "qbi.reit_ptp_carryforward_in_provenance",
            "QBI REIT/PTP carryforward",
        ),
        ("qbi.qbi_carryforward_in", "QBI business-loss carryforward"),
        (
            "qbi.qbi_carryforward_in_provenance",
            "QBI business-loss carryforward",
        ),
        (
            "capital_loss_carryforward_in",
            "capital-loss carryover short",
        ),
        (
            "capital_loss_carryforward_in_provenance",
            "capital-loss carryover short",
        ),
    ];

    fn leaves(ri: &ReturnInputs) -> BTreeMap<String, serde_json::Value> {
        fn walk(
            v: &serde_json::Value,
            prefix: &str,
            out: &mut BTreeMap<String, serde_json::Value>,
        ) {
            match v {
                serde_json::Value::Object(m) => {
                    for (k, child) in m {
                        let p = if prefix.is_empty() {
                            k.clone()
                        } else {
                            format!("{prefix}.{k}")
                        };
                        walk(child, &p, out);
                    }
                }
                leaf => {
                    out.insert(prefix.to_string(), leaf.clone());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&serde_json::to_value(ri).unwrap(), "", &mut out);
        out
    }

    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // A 2024 return that produces ALL FOUR carryover-outs at once.
    let y2025_before;
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut y2024 = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            charitable_cwa_obtained: Some(true),
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(50000),
                box3_ss_wages: dec!(50000),
                box5_medicare_wages: dec!(50000),
                ..Default::default()
            }],
            schedule_a: Some(btctax_core::tax::return_inputs::ScheduleAInputs {
                charitable: vec![btctax_core::tax::return_inputs::CharitableGift {
                    class: btctax_core::tax::return_inputs::CharitableClass::Cash60,
                    amount: dec!(40000),
                }],
                ..Default::default()
            }),
            qbi: btctax_core::tax::return_inputs::QbiInputs {
                reit_ptp_carryforward_in: dec!(10000),
                qbi_carryforward_in: dec!(8000),
                ..Default::default()
            },
            ..Default::default()
        };
        y2024.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
            short: rust_decimal::Decimal::ZERO,
            long: dec!(60000),
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut y2024);
        btctax_cli::return_inputs::set(s.conn(), 2024, &y2024).unwrap();

        let seed = btctax_core::tax::testonly::answered(ReturnInputs {
            tax_year: 2025,
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        });
        btctax_cli::return_inputs::set(s.conn(), 2025, &seed).unwrap();
        s.save().unwrap();
    }
    // ★ Read the row back AS STORED. The persistence layer stamps `tax_year`, and diffing against
    //   the in-memory seed would attribute that to the write-back — a checker reporting a mutation
    //   its subject did not make gets widened until it is silent.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        y2025_before = btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap();
    }

    let summary = cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();
    let y2025_after = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
    };

    // ── The OBSERVED set: which leaves the write-back actually moved. ──────────────────────────────
    let before = leaves(&y2025_before);
    let after = leaves(&y2025_after);
    let mut changed: Vec<String> = after
        .iter()
        .filter(|(k, v)| before.get(*k) != Some(*v))
        .map(|(k, _)| {
            // A `Vec<Struct>` leaf reads as `charitable_carryover_in[0].amount`; collapse to the
            // field so the table is about FIELDS, not about how many items happened to be written.
            let base = k
                .split_once('[')
                .map(|(h, _)| h.to_string())
                .unwrap_or_else(|| k.clone());
            // …and a `Carryforward`'s two LIMBS are one field: which limb a given fixture happens
            // to move is an accident of the fixture, and the table is about fields.
            base.strip_suffix(".short")
                .or_else(|| base.strip_suffix(".long"))
                .map(str::to_string)
                .unwrap_or(base)
        })
        .collect();
    changed.sort();
    changed.dedup();

    assert!(
        !changed.is_empty(),
        "the write-back moved NOTHING — a derivation that observes nothing reports OK forever"
    );
    let expected: Vec<String> = {
        let mut v: Vec<String> = EXPECTED.iter().map(|(p, _)| p.to_string()).collect();
        v.sort();
        v.dedup();
        v
    };
    assert_eq!(
        changed, expected,
        "★★★ the set of fields the write-back ASSIGNS is derived, not declared. A field added to \
         `apply_carryover_writeback` lands here as an unexpected path and reds THIS assertion — \
         which is the half a deletion-only checker does not have, and the half that would have \
         caught 'the charitable and QBI-REIT/PTP carryovers' being two where the code wrote three."
    );

    // ── …and every one of them is NAMED in what the filer is shown. ───────────────────────────────
    for (path, phrase) in EXPECTED {
        assert!(
            summary.contains(phrase),
            "the summary must name {path} (\\\"{phrase}\\\"); got: {summary}"
        );
    }
}

/// §4 R3-M6 carryover write-back (P4.9): `report --tax-year Y --write-carryover` persists year Y's
/// computed charitable carryover-out as year (Y+1)'s carryover-in (stamped Computed), and refuses to
/// overwrite a user-entered next-year carryover without `--force`.
#[test]
fn carryover_write_back_round_trips_and_respects_user_precedence() {
    use btctax_core::tax::return_inputs::{
        CarryProvenance, CharitableCarryItem, CharitableClass, CharitableGift, Owner, ReturnInputs,
        ScheduleAInputs, W2,
    };
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path()); // clean 2024 ledger (+$10k LTCG)
    let (_dir, vault) = make_vault_with(&csv);
    // 2024 ReturnInputs: $50k wages + a $40k cash charitable gift that overflows the 60%-of-AGI ceiling.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(50000),
                    box3_ss_wages: dec!(50000),
                    box5_medicare_wages: dec!(50000),
                    ..Default::default()
                }],
                // §170(f)(8): this filer holds a contemporaneous written acknowledgment for the
                // $40,000 gift. Not covered by `answered()`, which walks the DECLARATION registry;
                // the CWA is a class-(B) skippable made mandatory by `screen_absolute`.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(ScheduleAInputs {
                    charitable: vec![CharitableGift {
                        class: CharitableClass::Cash60,
                        amount: dec!(40000),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    // I1: with NO 2025 row yet, the write-back must REFUSE (fabricating one would shadow a stored
    // tax-profile for 2025 in the §4.12 ladder and make that year uncomputable in v1).
    let err = cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap_err();
    assert!(
        format!("{err:?}").contains("no full-return inputs yet"),
        "must refuse to fabricate a 2025 row: {err:?}"
    );
    // Import 2025's inputs first (the required order), THEN write back.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2025,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    // Write-back → 2025 gets a computed charitable carryover-in.
    let summary = cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();
    assert!(summary.contains("written back to 2025"), "{summary}");
    let next = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .expect("2025 row written")
    };
    assert!(!next.charitable_carryover_in.is_empty());
    assert!(next
        .charitable_carryover_in
        .iter()
        .all(|c| c.provenance == CarryProvenance::Computed));

    // Now a USER-entered 2025 carryover must NOT be silently overwritten.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut y2025 = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        y2025.charitable_carryover_in = vec![CharitableCarryItem {
            class: CharitableClass::Cash60,
            amount: dec!(5000),
            origin_year: 2023,
            provenance: CarryProvenance::User,
        }];
        btctax_core::tax::testonly::answer_all_live_declarations(&mut y2025);
        btctax_cli::return_inputs::set(s.conn(), 2025, &y2025).unwrap();
        s.save().unwrap();
    }
    assert!(
        cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).is_err(),
        "must refuse to overwrite a user-entered carryover without --force"
    );
    // --force overwrites it.
    assert!(cmd::tax::write_back_carryover(&vault, &pp(), 2024, true).is_ok());
    let forced = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
    };
    assert!(forced
        .charitable_carryover_in
        .iter()
        .all(|c| c.provenance == CarryProvenance::Computed));
}

/// ★★★ **K10 — `income import` must not silently drop a COMPUTED carryover, and the QBI half is a
/// LIVE FAIL-OPEN AT HEAD.**
///
/// `income import` is a whole-blob upsert, so a re-import silently drops anything
/// `--write-carryover` put on the row. The existing preservation block covers the charitable list
/// and, nominally, QBI — but the QBI arm keys **entirely** on `reit_ptp_carryforward_in > 0` and then
/// restores the WHOLE `qbi` struct. So a filer with a Form 8995 line-3 business-loss carryforward and
/// **zero** REIT/PTP loses the computed `qbi_carryforward_in` on re-import.
///
/// ★★★ THAT DIRECTION IS FAIL-OPEN. `qbi.rs`'s line 4 is `(business_qbi − qbi_carryforward_in).max(0)`,
/// so dropping the carryforward INFLATES the §199A deduction and **UNDERSTATES the tax** — the exact
/// direction the block's own comment says it exists to prevent. It is not a defect this branch
/// introduced; case (b) below was watched RED on the unmodified tree before the fix was written.
///
/// Three cases:
///   (a) the CAPITAL-LOSS carryover survives, value AND provenance, and the note names it;
///   (b) the QBI BUSINESS-LOSS carryforward survives with REIT/PTP at zero (the live defect);
///   (c) the CHARITABLE provenance survives — it reverted to `User` even while the items survived.
///
/// ★ Keyed on the PROVENANCE TRANSITION, never on `is_zero()`: a computed zero is meaningful, and a
///   test keyed to the value could not tell "the TOML said nothing" from "the TOML said zero".
#[test]
fn income_import_preserves_a_computed_capital_loss_carryover_and_the_qbi_business_loss_one() {
    use btctax_core::tax::return_inputs::{CarryProvenance, Owner, QbiInputs, ReturnInputs, W2};
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        // 2024 produces a capital-loss carryover-out AND a QBI business-loss carryforward-out, with
        // REIT/PTP deliberately at ZERO — the shape the existing arm cannot see.
        let mut y2024 = ReturnInputs {
            tax_year: 2024,
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(30000),
                box3_ss_wages: dec!(30000),
                box5_medicare_wages: dec!(30000),
                ..Default::default()
            }],
            qbi: QbiInputs {
                reit_ptp_carryforward_in: rust_decimal::Decimal::ZERO,
                qbi_carryforward_in: dec!(30000),
                ..Default::default()
            },
            ..Default::default()
        };
        y2024.capital_loss_carryforward_in = btctax_core::tax::types::Carryforward {
            short: rust_decimal::Decimal::ZERO,
            long: dec!(60000),
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut y2024);
        btctax_cli::return_inputs::set(s.conn(), 2024, &y2024).unwrap();

        let y2025 = btctax_core::tax::testonly::answered(ReturnInputs {
            tax_year: 2025,
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        });
        btctax_cli::return_inputs::set(s.conn(), 2025, &y2025).unwrap();
        s.save().unwrap();
    }
    cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();

    // ── PREMISES: all three figures are really on the 2025 row, stamped Computed. ─────────────────
    let rolled = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
    };
    assert!(
        rolled.capital_loss_carryforward_in.long > rust_decimal::Decimal::ZERO
            && rolled.capital_loss_carryforward_in_provenance == CarryProvenance::Computed,
        "premise: a computed capital-loss carryover must be ON the row, or (a) is vacuous: {:?}",
        rolled.capital_loss_carryforward_in
    );
    assert!(
        rolled.qbi.qbi_carryforward_in > rust_decimal::Decimal::ZERO
            && rolled.qbi.reit_ptp_carryforward_in.is_zero(),
        "premise: a business-loss carryforward with ZERO REIT/PTP is the exact shape the old arm \
         could not see: {:?}",
        rolled.qbi
    );
    assert_eq!(
        rolled.charitable_carryover_in_provenance,
        CarryProvenance::Computed,
        "premise: the charitable provenance starts Computed"
    );

    // ── Re-import a TOML that supplies NO carryover at all. ───────────────────────────────────────
    let toml = csv_dir.path().join("2025.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\ntaxpayer_died_during_year = false\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2025, &toml, false).unwrap();
    let after = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
    };

    // (a) the capital-loss carryover survives, VALUE and PROVENANCE.
    assert_eq!(
        after.capital_loss_carryforward_in, rolled.capital_loss_carryforward_in,
        "★ (a) a computed capital-loss carryover must survive an import that supplies none — \
         dropping it would understate next year's Schedule D lines 6 and 14"
    );
    assert_eq!(
        after.capital_loss_carryforward_in_provenance,
        CarryProvenance::Computed,
        "★ (a) …and stay Computed, or the next write-back refuses to overwrite btctax's own figure"
    );

    // (b) THE LIVE FAIL-OPEN: the business-loss carryforward with REIT/PTP at zero.
    assert_eq!(
        after.qbi.qbi_carryforward_in, rolled.qbi.qbi_carryforward_in,
        "★★★ (b) losing the Form 8995 line-3 business-loss carryforward INFLATES the §199A \
         deduction and UNDERSTATES the tax — line 4 is `(business_qbi − qbi_carryforward_in).max(0)`. \
         This half was watched RED on the unmodified tree: the old arm keyed entirely on \
         `reit_ptp_carryforward_in > 0`, which is zero here."
    );
    assert_eq!(
        after.qbi.qbi_carryforward_in_provenance,
        CarryProvenance::Computed
    );

    // (c) the charitable PROVENANCE survives too — the items did, the stamp did not.
    assert_eq!(
        after.charitable_carryover_in_provenance,
        CarryProvenance::Computed,
        "★ (c) an empty charitable list has no per-item provenance, so the LIST-level stamp is the \
         only thing that distinguishes 'no carryover' from 'never asked'. It reverted to `User`."
    );
}

/// I2 (Fable P4.9 r1): re-importing year Y+1 must NOT silently drop the `Computed` carryover that
/// `--write-carryover` put there. For QBI that drop would be a FAIL-OPEN (losing the REIT/PTP loss
/// carryforward overstates the QBI deduction ⇒ understates tax). A TOML that supplies no carryover keeps
/// the computed one; a TOML that DOES supply one is the user's and wins (as `User`).
#[test]
fn import_preserves_a_computed_carryover() {
    use btctax_core::tax::return_inputs::{
        CarryProvenance, CharitableClass, CharitableGift, Owner, ReturnInputs, ScheduleAInputs, W2,
    };
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        // 2024: wages + an over-ceiling charitable gift → a carryover-out.
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(50000),
                    box3_ss_wages: dec!(50000),
                    box5_medicare_wages: dec!(50000),
                    ..Default::default()
                }],
                // §170(f)(8): this filer holds a contemporaneous written acknowledgment for the
                // $40,000 gift. Not covered by `answered()`, which walks the DECLARATION registry;
                // the CWA is a class-(B) skippable made mandatory by `screen_absolute`.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(ScheduleAInputs {
                    charitable: vec![CharitableGift {
                        class: CharitableClass::Cash60,
                        amount: dec!(40000),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .unwrap();
        // 2025 row must exist before the write-back (I1).
        btctax_cli::return_inputs::set(
            s.conn(),
            2025,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();

    // Re-import 2025 from a TOML that carries NO carryover — the computed one must SURVIVE.
    let toml = csv_dir.path().join("2025.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\ntaxpayer_died_during_year = false\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2025, &toml, false).unwrap();
    let after = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
    };
    assert!(
        !after.charitable_carryover_in.is_empty(),
        "a Computed carryover must survive an import that does not supply one"
    );
    assert!(after
        .charitable_carryover_in
        .iter()
        .all(|c| c.provenance == CarryProvenance::Computed));
}

/// P5: the full-return report surfaces the §3.4 conservative-omission advisories (CTC/ODC with dependents,
/// the forfeited §63(f) aged box when no DOB is on file, the EIC) + the FBAR disclosure — loudly, and
/// saying which direction the error runs (OVERSTATED). Non-gating: the report still computes.
#[test]
fn full_return_report_surfaces_conservative_omission_advisories() {
    use btctax_core::tax::return_inputs::{Dependent, Owner, ReturnInputs, W2};
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(40000),
                box3_ss_wages: dec!(40000),
                box5_medicare_wages: dec!(40000),
                ..Default::default()
            }],
            foreign_accounts: Some(true),           // → FBAR disclosure
            foreign_country_names: "Canada".into(), // 7b — required now that 7a is "Yes" (P9 §3.2)
            foreign_trust: Some(false),
            ..Default::default()
        };
        ri.header.dependents = vec![Dependent::default()]; // → CTC/ODC omission
                                                           // taxpayer.date_of_birth stays None → the §63(f) aged box is forfeited
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        btctax_cli::return_inputs::set(s.conn(), 2024, &ri).unwrap();
        s.save().unwrap();
    }
    let r = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap();
    let dual = r
        .dual_report
        .expect("full-return year must render the dual report");
    assert!(dual.contains("ADVISORIES"), "{dual}");
    assert!(dual.contains("CTC/ODC NOT COMPUTED"), "{dual}");
    assert!(dual.contains("OVERSTATED"), "{dual}");
    assert!(dual.contains("DATE OF BIRTH NOT ON FILE"), "{dual}");
    assert!(dual.contains("FBAR"), "{dual}");
    // AGI = 40k wages + 10k LTCG = 50k < the $70k EIC advisory ceiling, with earned income → EIC too.
    assert!(dual.contains("EIC NOT COMPUTED"), "{dual}");
}

/// ★ **D-8 acceptance, end to end: the bug reaches the rows that ALREADY have it, and the user can get out.**
///
/// This is the whole point of P8a, and it is the ONE test that exercises the real path a real person is
/// on. A vault written by the SHIPPED code carries `can_be_claimed_as_dependent_taxpayer: false` on every
/// row — a value nobody was ever asked for. Under the old code that silently bought the filer the full
/// basic standard deduction (and a free pass on the §1(g) kiddie-tax screen), and printed an unchecked box
/// on a filed 1040.
///
/// So the fix must do BOTH halves, and this asserts both:
///   1. the laundered `false` is NOT trusted — the year refuses rather than computing a wrong number; and
///   2. `income answer` gets them out of it — WITHOUT the TOML file, which the spec told them to delete.
///
/// A fix with only half 1 is a brick. A fix with only half 2 never reaches the people who have the bug.
#[test]
fn a_pre_d8_vault_refuses_until_answered_and_income_answer_is_the_way_out() {
    use btctax_cli::return_inputs;

    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // (a) A return imported the way the SHIPPED code did it: no dependent answer anywhere in the TOML,
    //     because the shipped code had nowhere to put one.
    let toml = _dir.path().join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\n\
         box1_wages = \"90000\"\nbox2_fed_withheld = \"12000\"\nbox5_medicare_wages = \"90000\"\n",
    )
    .unwrap();
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap();

    // (b) It REFUSES — loudly, as an error. It does not quietly hand back a number computed from a guess.
    let err = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("claim YOU as a dependent"),
        "the refusal must name the unanswered question: {msg}"
    );
    // ★ And it must tell them HOW to get out. A refusal with no exit is just a brick with better prose.
    assert!(
        msg.contains("income answer"),
        "the refusal must point at its own remedy: {msg}"
    );

    // (c) The user no longer has the TOML — they deleted it, as the plaintext-hygiene guidance says to.
    std::fs::remove_file(&toml).unwrap();

    // (d) `income answer` is the way out. It asks SEVEN mandatory declarations in registry order
    // (dependent-taxpayer, foreign-accounts, foreign-trust, HSA-activity, dual-status, §911/931/933
    // income-exclusion gate) — "n" to each — then bare Enter to skip the always-live SKIPPABLES:
    // §63(f) blindness, the date of birth, the §G-9 taxpayer-death gate (2026-07-31: downgraded from a
    // mandatory declaration, since silence already forgoes the addition), and the §G-21
    // donation-restrictions universal (2026-07-31: offered always, because the donations are in the
    // LEDGER and liveness cannot see them; it becomes MANDATORY only on a Section-B year, which this
    // household is not). The §G-9 DATE of death is not among them: it is live only once its gate
    // is answered "y".
    //
    // ★ 2026-08-21 — a SEVENTH mandatory declaration: Schedule D line 20 / Schedule A line 9's
    // "are you filing Form 4952?" (always live, because line 20 prints on every both-gains Schedule D
    // and that routing is a LEDGER fact liveness cannot see). And a FIFTH always-live skippable, the
    // §170(f)(8) contemporaneous-written-acknowledgment universal — offered always for the same
    // reason as the donation-restrictions one, mandatory only where a §170 deduction is claimed.
    //
    // ★ Seven "n" then bare Enters — the exact count is deliberate. A script that runs out fails with
    // "input ended before every question was answered", which is how this test noticed the interview
    // had grown at all.
    let mut keystrokes: &[u8] = b"n\nn\nn\nn\nn\nn\nn\n\n\n\n\n\n";
    let mut screen: Vec<u8> = Vec::new();
    cmd::answer::answer_return_inputs(&vault, &pp(), 2024, &mut keystrokes, &mut screen).unwrap();
    let screen = String::from_utf8(screen).unwrap();
    assert!(
        screen.contains("claim YOU as a dependent"),
        "it must actually ASK the question: {screen}"
    );
    assert!(
        !screen.contains("SPOUSE"),
        "a Single filer must not be asked about a spouse who does not exist: {screen}"
    );

    // (e) The year computes again — and the answer is on file as an ANSWER (`Some(false)`), not as the
    //     absence that the same `false` used to mean.
    let TaxYearReport { outcome, .. } =
        cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap();
    assert!(
        matches!(outcome, btctax_core::TaxOutcome::Computed(_)),
        "after answering, the year must compute: {outcome:?}"
    );
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    assert_eq!(
        return_inputs::get(s.conn(), 2024)
            .unwrap()
            .unwrap()
            .header
            .can_be_claimed_as_dependent_taxpayer,
        Some(false),
        "the answer must PERSIST as an answer — if it re-launders to None the year re-bricks on next read"
    );
}

/// `income answer` refuses a year that has no return. Answering questions about a return that does not
/// exist would MATERIALIZE a near-empty `ReturnInputs` row — which then takes precedence over the user's
/// `tax-profile` (the resolver ranks `ReturnInputs` first), silently replacing a working profile with an
/// empty return. A missing row is a mistake to report, not a shape to invent.
#[test]
fn income_answer_refuses_a_year_with_no_return() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2025(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    let mut keystrokes: &[u8] = b"n\n\n";
    let mut screen: Vec<u8> = Vec::new();
    let err = cmd::answer::answer_return_inputs(&vault, &pp(), 2024, &mut keystrokes, &mut screen)
        .unwrap_err();
    assert!(
        format!("{err}").contains("income import"),
        "the refusal must say how to create the return: {err}"
    );
    assert!(
        screen.is_empty(),
        "it must not ask anything before refusing"
    );
}

// ── P9 §2.6 — the stale-row REMEDY chain (IMPL r1 I-2) ───────────────────────────────────────────
//
// `p9_stale_row_refuses` (in the library) holds the version gate and the error TEXT. These hold the
// remedy's BEHAVIOUR: that `income import` over a stale row refuses (rather than silently overwriting and
// dropping the row's Computed carryover — the fail-open §2.6 adjudicated and REJECTED), that
// `clear` + `import` recovers to v2, and that the full chain restores the carryover `clear` discards.

/// A minimal, fully-answered import TOML (every always-live declaration answered).
fn answered_toml(dir: &Path) -> PathBuf {
    let toml = dir.join("inputs.toml");
    std::fs::write(
        &toml,
        "filing_status = \"Single\"\nforeign_accounts = false\nforeign_trust = false\n\
         dual_status_alien = false\nhas_income_exclusion = false\nother_out_of_scope_income = false\nfiling_form_4952 = false\n\n[header]\ncan_be_claimed_as_dependent_taxpayer = false\ntaxpayer_died_during_year = false\n\n\
         [sch1]\nhsa_activity = false\n\n[[w2s]]\nowner = \"taxpayer\"\nemployer = \"ACME\"\n\
         box1_wages = \"50000\"\nbox2_fed_withheld = \"6000\"\nbox5_medicare_wages = \"50000\"\n",
    )
    .unwrap();
    toml
}

/// Force an existing `return_inputs` row to a PRE-P9 schema version (as a real pre-P9 vault would hold).
fn stale_the_row(vault: &Path, year: i32) {
    let mut s = Session::open(vault, &pp()).unwrap();
    let n = s
        .conn()
        .execute(
            "UPDATE return_inputs SET schema_version = 1 WHERE year = ?1",
            [year],
        )
        .unwrap();
    assert_eq!(n, 1, "the row must exist to be staled");
    s.save().unwrap();
}

fn row_version(vault: &Path, year: i32) -> i64 {
    let s = Session::open(vault, &pp()).unwrap();
    s.conn()
        .query_row(
            "SELECT schema_version FROM return_inputs WHERE year = ?1",
            [year],
            |r| r.get(0),
        )
        .unwrap()
}

/// ★ IMPL r1 I-2, test (a). `income import` READS the existing row (the §4 R3-M6 carryover-preservation
/// read), so over a STALE row it must REFUSE — not silently overwrite. This is the test the reviewer's
/// mutation (`get(...).ok().flatten()` in `import_return_inputs`) kills: with the read swallowed, import
/// would overwrite the stale row at v2 and drop its Computed carryover with no error.
#[test]
fn import_over_a_stale_row_refuses() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    let toml_dir = tempfile::tempdir().unwrap();
    let toml = answered_toml(toml_dir.path());

    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap(); // a v2 row
    stale_the_row(&vault, 2024); // now it is pre-P9

    let err = cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap_err();
    assert!(
        matches!(err, btctax_cli::CliError::StaleReturnInputs { year: 2024, .. }),
        "import over a stale row must refuse (naming the remedy), not silently overwrite it: {err:?}"
    );
}

/// ★ IMPL r1 I-2, test (b). The named remedy WORKS: `income clear` (a bare DELETE — it never deserializes,
/// so it cannot itself refuse) then `income import` recovers the year, and the fresh row is stamped v2.
#[test]
fn clear_then_import_recovers_a_stale_row_to_v2() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    let toml_dir = tempfile::tempdir().unwrap();
    let toml = answered_toml(toml_dir.path());

    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap();
    stale_the_row(&vault, 2024);

    assert!(
        cmd::tax::clear_return_inputs(&vault, &pp(), 2024).unwrap(),
        "clear works on a stale row (it never deserializes)"
    );
    cmd::tax::import_return_inputs(&vault, &pp(), 2024, &toml, false).unwrap(); // no existing row now ⇒ succeeds
    assert_eq!(
        row_version(&vault, 2024),
        2,
        "the recovered row is stamped v2"
    );
}

/// ★ IMPL r1 I-2, test (c). THE FULL CHAIN restores what `clear` discards. Seed 2024, write a Computed
/// carryover onto 2025, stale BOTH rows, then run the whole remedy (clear+import each year, then re-run
/// `--write-carryover`) and assert 2025's Computed carryover is back — the "disclosure is not restoration"
/// guarantee (r6 I-1) that `clear` alone would silently drop.
#[test]
fn the_full_remedy_chain_restores_a_computed_carryover() {
    use btctax_core::tax::return_inputs::{
        CarryProvenance, CharitableClass, CharitableGift, ReturnInputs, ScheduleAInputs, W2,
    };
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    // 2024: a $40k cash gift that overflows the 60%-of-AGI ceiling ⇒ a charitable carryover to 2025.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    box1_wages: dec!(50000),
                    box5_medicare_wages: dec!(50000),
                    ..Default::default()
                }],
                // §170(f)(8): this filer holds a contemporaneous written acknowledgment for the
                // $40,000 gift. Not covered by `answered()`, which walks the DECLARATION registry;
                // the CWA is a class-(B) skippable made mandatory by `screen_absolute`.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(ScheduleAInputs {
                    charitable: vec![CharitableGift {
                        class: CharitableClass::Cash60,
                        amount: dec!(40000),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .unwrap();
        // 2025 row must exist for the write-back (I1).
        btctax_cli::return_inputs::set(
            s.conn(),
            2025,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    // Compute the carryover onto 2025.
    cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();
    let carry_before = {
        let s = Session::open(&vault, &pp()).unwrap();
        let n = btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap();
        assert!(
            n.charitable_carryover_in
                .iter()
                .any(|c| c.provenance == CarryProvenance::Computed),
            "precondition: 2025 carries a Computed carryover"
        );
        n.charitable_carryover_in.clone()
    };

    // Both rows go stale (a pre-P9 vault).
    stale_the_row(&vault, 2024);
    stale_the_row(&vault, 2025);

    // THE REMEDY, in order: clear + re-import each year (from the same inputs), THEN rebuild the carryover.
    let toml_dir = tempfile::tempdir().unwrap();
    let toml = answered_toml(toml_dir.path());
    for year in [2024, 2025] {
        cmd::tax::clear_return_inputs(&vault, &pp(), year).unwrap();
        cmd::tax::import_return_inputs(&vault, &pp(), year, &toml, false).unwrap();
    }
    // NOTE: the re-imported 2024 has no charitable gift (the minimal TOML), so to reproduce the carryover
    // the filer re-imports 2024's REAL inputs. Here we re-store them, then re-run the write-back.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    box1_wages: dec!(50000),
                    box5_medicare_wages: dec!(50000),
                    ..Default::default()
                }],
                // §170(f)(8): this filer holds a contemporaneous written acknowledgment for the
                // $40,000 gift. Not covered by `answered()`, which walks the DECLARATION registry;
                // the CWA is a class-(B) skippable made mandatory by `screen_absolute`.
                charitable_cwa_obtained: Some(true),
                schedule_a: Some(ScheduleAInputs {
                    charitable: vec![CharitableGift {
                        class: CharitableClass::Cash60,
                        amount: dec!(40000),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap();

    let carry_after = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::get(s.conn(), 2025)
            .unwrap()
            .unwrap()
            .charitable_carryover_in
    };
    assert_eq!(
        carry_after, carry_before,
        "the full remedy chain (clear + import + write-carryover) restores 2025's Computed carryover"
    );
}

/// UX-P4-1 surface 4 fixture: a TY2024 `ReturnInputs` vault (wages + a $40k charitable overflow → a real
/// carryover-out) whose ledger ALSO carries an unknown-basis 2024 Receive (`hb-recv`) — the pseudo trigger
/// (pseudo ON ⇒ a synthetic $0 lot ⇒ `pseudo_active()`; pseudo OFF ⇒ a Hard blocker ⇒ NotComputable delta).
/// A TY2025 row is present so "persists nothing" is a concrete before/after byte-compare.
fn fr2024_writeback_vault_with_pseudo_trigger() -> (tempfile::TempDir, PathBuf) {
    use btctax_core::tax::return_inputs::{
        CharitableClass, CharitableGift, Owner, ReturnInputs, ScheduleAInputs, W2,
    };
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_receive_2024(csv_dir.path()); // 2023 Buy + a 2024 unknown-basis Receive
    let (dir, vault) = make_vault_with(&csv);
    let mut s = Session::open(&vault, &pp()).unwrap();
    btctax_cli::return_inputs::set(
        s.conn(),
        2024,
        &btctax_core::tax::testonly::answered(ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                box1_wages: dec!(50000),
                box3_ss_wages: dec!(50000),
                box5_medicare_wages: dec!(50000),
                ..Default::default()
            }],
            // §170(f)(8): this filer holds a contemporaneous written acknowledgment for the
            // $40,000 gift. Not covered by `answered()`, which walks the DECLARATION registry;
            // the CWA is a class-(B) skippable made mandatory by `screen_absolute`.
            charitable_cwa_obtained: Some(true),
            schedule_a: Some(ScheduleAInputs {
                charitable: vec![CharitableGift {
                    class: CharitableClass::Cash60,
                    amount: dec!(40000),
                }],
                ..Default::default()
            }),
            ..Default::default()
        }),
    )
    .unwrap();
    btctax_cli::return_inputs::set(
        s.conn(),
        2025,
        &btctax_core::tax::testonly::answered(ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        }),
    )
    .unwrap();
    s.save().unwrap();
    (dir, vault)
}

fn get_2025_row(vault: &Path) -> Option<btctax_core::tax::return_inputs::ReturnInputs> {
    let s = Session::open(vault, &pp()).unwrap();
    btctax_cli::return_inputs::get(s.conn(), 2025).unwrap()
}

/// UX-P4-1 surface 4 clause 4a (SPEC §3.1) [T-C1]: `--write-carryover` on a PSEUDO-ACTIVE ledger REFUSES
/// fail-closed and persists NOTHING — else a deliberately-synthetic carryover launders into year+1's real
/// inputs, where next year's banner cannot fire. (★ fault-inject: drop the `pseudo_active()` gate in
/// `write_back_carryover` and this goes RED — the write would succeed.)
#[test]
fn write_carryover_refuses_on_a_pseudo_active_ledger_and_persists_nothing() {
    let (_dir, vault) = fr2024_writeback_vault_with_pseudo_trigger();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let before = get_2025_row(&vault);
    let err = cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap_err();
    assert!(
        format!("{err:?}").contains("pseudo-reconcile mode is contributing synthetic"),
        "expected the fail-closed pseudo refuse, got: {err:?}"
    );
    assert_eq!(
        before,
        get_2025_row(&vault),
        "a refused write-back must leave year+1 inputs byte-identical"
    );
}

/// UX-P4-1 surface 4 clause 4b (SPEC §3.1) [G2-NEW-4]: `--write-carryover` on a NOT-COMPUTABLE (Hard-blocked,
/// non-pseudo) ledger REFUSES and persists NOTHING — the same laundering class minus the pseudo mechanism.
/// (★ fault-inject: drop the NotComputable gate and this goes RED.)
#[test]
fn write_carryover_refuses_on_a_not_computable_ledger_and_persists_nothing() {
    let (_dir, vault) = fr2024_writeback_vault_with_pseudo_trigger();
    // pseudo OFF (default): the unknown-basis 2024 Receive is a Hard blocker ⇒ NotComputable crypto-delta.
    let before = get_2025_row(&vault);
    let err = cmd::tax::write_back_carryover(&vault, &pp(), 2024, false).unwrap_err();
    assert!(
        format!("{err:?}").contains("NOT COMPUTABLE"),
        "expected the fail-closed NotComputable refuse, got: {err:?}"
    );
    assert_eq!(
        before,
        get_2025_row(&vault),
        "a refused write-back must leave year+1 inputs byte-identical"
    );
}

/// UX-P4-1 surface 2 (SPEC §3.1) [T-I5]: on a pseudo-active `ReturnInputs`-provenance year, the §6
/// dual-report's filer-transcribed absolute totals ("TOTAL TAX (L24)" and "Absolute TOTAL TAX") carry the
/// `[PSEUDO]` suffix — so the most filing-authoritative scraped lines cannot be read clean. (★ fault-inject:
/// drop either `pseudo.suffix()` in `render_dual_report` and this goes RED.)
#[test]
fn pseudo_dual_report_absolute_totals_carry_pseudo_suffix() {
    let (_dir, vault) = fr2024_writeback_vault_with_pseudo_trigger();
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();
    let report = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0)).unwrap();
    assert_eq!(
        report.pseudo_contributed,
        render::PseudoDisclosure::Synthetic,
        "a ReturnInputs year with a synthetic lot is the Synthetic channel"
    );
    let dual = report
        .dual_report
        .expect("a ReturnInputs-provenance year must render the §6 dual report");
    let l24 = dual
        .lines()
        .find(|l| l.contains("TOTAL TAX (L24)"))
        .expect("a computed dual report has the L24 line");
    assert!(
        l24.contains("[PSEUDO]"),
        "the filed-return TOTAL TAX (L24) must be [PSEUDO]-suffixed under pseudo: {l24}"
    );
    let abs = dual
        .lines()
        .find(|l| l.contains("Absolute TOTAL TAX"))
        .expect("the §6 block has an 'Absolute TOTAL TAX' line");
    assert!(
        abs.contains("[PSEUDO]"),
        "the Absolute TOTAL TAX line must be [PSEUDO]-suffixed under pseudo: {abs}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// P6 — the §170(d)(1) charitable carryover-out must reach a HUMAN (FILING-READINESS-PLAN rank 9).
//
// `charitable_carryover_out` is computed on every full-return run, is correct, and is tested — and
// before this it had ZERO readers outside `apply_carryover_writeback`, which is reachable only from
// `report --write-carryover`, which itself ERRORS unless a year+1 row already exists. So a filer
// whose gift exceeded its §170(b) ceiling was never told a carryover existed at all.
//
// ★ This is not a rich-filer item. The ceiling is a FRACTION OF AGI, so the lower the income the
// lower the bar: at $0 AGI the ceiling is $0 and 100% of the gift carries forward, invisibly.

/// A 2024 full-return vault whose gift OVERFLOWS its §170(b)(1)(A) ceiling: $50,000 of wages plus a
/// $10,000 long-term crypto gain (AGI $60,000, so the 60% cash ceiling is $36,000) against a $40,000
/// cash gift ⇒ $4,000 carries to 2025. Same shape as
/// `carryover_write_back_round_trips_and_respects_user_precedence`'s fixture.
fn vault_with_a_gift_over_its_ceiling() -> (tempfile::TempDir, PathBuf) {
    use btctax_core::tax::return_inputs::{
        CharitableClass, CharitableGift, Owner, ReturnInputs, ScheduleAInputs, W2,
    };
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (dir, vault) = make_vault_with(&csv);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(50000),
                    box3_ss_wages: dec!(50000),
                    box5_medicare_wages: dec!(50000),
                    ..Default::default()
                }],
                schedule_a: Some(ScheduleAInputs {
                    charitable: vec![CharitableGift {
                        class: CharitableClass::Cash60,
                        amount: dec!(40000),
                    }],
                    ..Default::default()
                }),
                // ★ P4 (phase 2): an itemizing return with a gift >= $250 must state whether it
                //   holds the §170(f)(8) acknowledgment. The subject here is the CARRYOVER, not
                //   substantiation, so answer it rather than dodge it — a $40,000 gift with no CWA
                //   is a deduction the statute denies, and a fixture must not model an unlawful one.
                charitable_cwa_obtained: Some(true),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    // Keep the csv tempdir alive only as long as the import needed it; `dir` owns the vault.
    drop(csv_dir);
    (dir, vault)
}

/// ★ P6. `report --tax-year 2024` must PRINT the charitable carryover, per class and per origin
/// year. Mutation: delete the `render_charitable_carryover_out` call from `render_dual_report` and
/// this reds.
#[test]
fn a_gift_over_its_ceiling_prints_its_charitable_carryover_out_in_the_report() {
    let (_dir, vault) = vault_with_a_gift_over_its_ceiling();
    let dual = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0))
        .unwrap()
        .dual_report
        .expect("a ReturnInputs-provenance year renders the §6 dual report");

    assert!(
        dual.contains("Charitable carryover to 2025"),
        "the report must name the carryover and the year it lands in: {dual}"
    );
    assert!(
        dual.contains("4000.00"),
        "…with the amount ($60,000 AGI × 60% = $36,000 ceiling against a $40,000 gift): {dual}"
    );
    assert!(
        dual.contains("2024"),
        "…tagged by ORIGIN YEAR, because §170(d)(1) expires it after five: {dual}"
    );
    assert!(
        dual.contains("60%"),
        "…and by §170(b) CLASS, because each class has its own ceiling: {dual}"
    );
    assert!(
        dual.contains("--write-carryover"),
        "…and it must say how to roll it forward: {dual}"
    );
}

/// The other half of the pair — a gift that fits UNDER its ceiling prints NO carryover block. A line
/// that always appears tells the filer nothing, and a $0 carryover printed as a line is the
/// fabricated-testimony shape this repo refuses everywhere else.
#[test]
fn a_gift_within_its_ceiling_prints_no_charitable_carryover_line() {
    use btctax_core::tax::return_inputs::{
        CharitableClass, CharitableGift, Owner, ReturnInputs, ScheduleAInputs, W2,
    };
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_sell_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                w2s: vec![W2 {
                    owner: Owner::Taxpayer,
                    box1_wages: dec!(50000),
                    box3_ss_wages: dec!(50000),
                    box5_medicare_wages: dec!(50000),
                    ..Default::default()
                }],
                schedule_a: Some(ScheduleAInputs {
                    // $1,000 against a $36,000 ceiling — nothing carries.
                    charitable: vec![CharitableGift {
                        class: CharitableClass::Cash60,
                        amount: dec!(1000),
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
    let dual = cmd::tax::report_tax_year(&vault, &pp(), 2024, dec!(0))
        .unwrap()
        .dual_report
        .expect("a ReturnInputs-provenance year renders the §6 dual report");
    assert!(
        !dual.contains("Charitable carryover"),
        "a gift within its ceiling creates no carryover, so nothing may be printed: {dual}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★★★ F1 (phase-1 seam review) — M4 must not dispute the figure N1's advisory told the filer to
// enter. Until N1 there was one answer for "last year's carryforward-out"; N1 made the
// §1212(b)(2)(B) worksheet the correct one on a full-return year, and the two DIVERGE exactly on
// the floor household the worksheet exists for. Nobody touched M4, so the base tree was the second
// lane and no lane's mutations could see the contradiction.

/// A TY2024 vault holding one long-term crypto LOSS of $20,000 (buy 1 BTC @ $60k in 2022, sell @
/// $40k in 2024) — lane C's L4 shape.
fn write_lt_loss_2024(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_lt_loss24.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lt-buy,2022-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,60000.00,60000.00,60000.00,0.00,,,\r\n\
lt-sell,2024-06-15 12:00:00 UTC,Sell,BTC,1.00000000,USD,40000.00,40000.00,40000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Build the F1 scenario and return the rendered TY2025 advisory, if any.
///
/// 2024 is a `ReturnInputs` (full-return) year with NO wages and a $20,000 LT loss ⇒ taxable income
/// floors at zero, NOTHING of the §1211(b) $3,000 allowance is absorbed, and the worksheet carries
/// the WHOLE $20,000. The frozen delta engine still says $17,000 and structurally cannot say
/// otherwise. 2025 declares `carryforward_in.long = declared`.
fn f1_advisory_for_declared_carryforward(declared: rust_decimal::Decimal) -> Option<String> {
    use btctax_core::tax::return_inputs::ReturnInputs;
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_loss_2024(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_cli::return_inputs::set(
            s.conn(),
            2024,
            &btctax_core::tax::testonly::answered(ReturnInputs {
                filing_status: FilingStatus::Single,
                header: btctax_core::tax::testonly::not_a_dependent(),
                ..Default::default() // no wages — this is what puts TI at the floor
            }),
        )
        .unwrap();
        s.save().unwrap();
    }
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
            capital_loss_carryforward_in: Carryforward {
                short: dec!(0),
                long: declared,
            },
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        },
        false,
    )
    .unwrap();
    let TaxYearReport { advisory, .. } =
        cmd::tax::report_tax_year(&vault, &pp(), 2025, dec!(0)).unwrap();
    advisory
}

/// ★★★ **F1, the kill-test.** The filer obeys the N1 advisory and carries the WORKSHEET figure
/// ($20,000). M4 must be SILENT.
///
/// Before the fix M4 compared against the frozen engine's flat $17,000 and fired "verify your prior
/// return" — an instrument disputing the figure the product itself instructed them to enter, phrased
/// as an audit of it. A filer who obeys the audit "corrects" to $17,000 and permanently forfeits
/// $3,000 of capital loss.
///
/// B1 planted defect: pass `&prev.carryforward_out` (the flat figure) instead of the worksheet
/// authority in `cmd/tax.rs`, and this reds with "does not match prior-year carryforward_out".
#[test]
fn m4_is_silent_when_the_filer_carries_the_worksheet_figure_n1_told_them_to() {
    // The premise: the two figures really do diverge here, or this test proves nothing.
    let flat = f1_advisory_for_declared_carryforward(dec!(17000));
    assert!(
        flat.is_some_and(|a| a.contains("does not match")),
        "★ This assertion has TWO possible causes and they are not the same defect — do not assume \
         the first one. Either (a) `cmd/tax.rs` reverted to comparing against the frozen engine's \
         FLAT figure, so $17,000 now matches and F1 is back; or (b) the fixture stopped being the \
         floor case, so the worksheet and the flat rule agree and this test proves nothing either \
         way. Check the M4 call site in `cmd/tax.rs` FIRST — (a) is the regression this test exists \
         to catch, and it is the one that silently forfeits $3,000 of a filer's capital loss."
    );

    let advisory = f1_advisory_for_declared_carryforward(dec!(20000));
    assert_eq!(
        advisory, None,
        "★ M4 must not dispute the §1212(b)(2)(B) worksheet figure that N1's own advisory \
         instructed the filer to carry"
    );
}

/// ★ The other half: M4 must still DISPUTE an arbitrary figure. A consistency check that went silent
/// on everything would satisfy the test above and be worthless.
#[test]
fn m4_still_disputes_a_carryforward_that_matches_neither_authority() {
    let advisory = f1_advisory_for_declared_carryforward(dec!(19000));
    assert!(
        advisory.is_some_and(|a| a.contains("does not match")),
        "★ M4 must still fire on a figure that is neither the worksheet's nor the flat rule's"
    );
}
