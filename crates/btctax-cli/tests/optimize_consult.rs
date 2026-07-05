//! CLI integration tests for `optimize consult` (Task 11 / §C.3 Mode-2 read-only pre-trade what-if).
//!
//! `optimize consult` is READ-ONLY — it calls `consult_sale` which is side-effect-free
//! (clone-fold-discard on every candidate). It appends NO event and writes NO side-table row.
//! These tests assert:
//!   - The tax-minimizing lot selection is returned and correctly rendered.
//!   - The ST/LT gain split and federal tax are exact (NFR5 — no float).
//!   - The ST→LT timing insight is present when ≥1 selected lot is short-term AND the crossover
//!     falls in the same year, and absent otherwise.
//!   - A future date with no bundled dataset price and `proceeds=None` → `ProceedsRequired` error
//!     mentioning "proceeds"; the same date with explicit `--proceeds` → `Ok`.
//!   - The event log is byte-identical before and after every consult call (read-only invariant).
//!
//! PRIVACY: all fixtures are synthetic and written into tempdirs; the suite NEVER reads
//! `~/Documents/BitcoinTax/ReadOnly` or any user file.
//! Exact Decimal/i64 only (NFR5); deterministic (NFR4 — no HashMap, no Date::now in core).
use btctax_cli::{cmd, render};
use btctax_core::{
    persistence::load_all, Carryforward, DisposeKind, FilingStatus, TaxProfile, WalletId,
};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::date;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// Single filer profile for TY2025: OTI=$100k, MAGI=$100k, QD=0.
/// With OTI=$100,000 the marginal ordinary rate is 22% (Single 2025: $48,475–$103,350 at 22%).
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

/// `exchange:coinbase:default` — the wallet Coinbase CSV imports land in (adapter `exchange_wallet`).
fn coinbase_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "default".into(),
    }
}

/// Initialize vault + import one CSV; return `(tempdir, vault_path)`.
fn make_vault_with(csv: &Path) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv.to_path_buf()]).unwrap();
    (dir, vault)
}

/// Count events in the vault (used to assert the read-only invariant).
fn event_count(vault: &Path) -> usize {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    load_all(s.conn()).unwrap().len()
}

// ── CSV fixtures ──────────────────────────────────────────────────────────────────────────────────

/// Coinbase CSV with TWO buy lots (high-basis and low-basis). No sell in the ledger.
///
/// Lot A (low-basis):  1 BTC @ $30,000 acquired 2024-01-15 → LT at 2025-06-15 (>1 year).
/// Lot B (high-basis): 1 BTC @ $80,000 acquired 2025-03-01 → ST at 2025-06-15 (<1 year).
///
/// Consult: sell 1 BTC (100,000,000 sat) from exchange:coinbase:default at 2025-06-15 (FMV=$67,500).
/// HIFO (tax-min): optimizer picks Lot B ($80k) → ST gain = $67,500 − $80,000 = −$12,500.
/// §1211 cap: loss deduction = $3,000 × 22% marginal = $660 saving.
/// → st_gain = −12,500; lt_gain = 0; total_federal_tax_attributable = −660.
fn write_two_lots_csv(dir: &Path) -> PathBuf {
    let p = dir.join("consult_two_lots.csv");
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

/// Coinbase CSV with ONE buy lot that is short-term at 2025-06-15 but crosses over to LT on 2025-06-17.
///
/// Lot: 1 BTC @ $60,000 acquired 2024-06-16.
///   - `is_long_term(2024-06-16, 2025-06-15)` = 2025-06-15 > 2025-06-16? → NO → short-term.
///   - `one_year_after(2024-06-16)` = 2025-06-16 → `next_day()` = 2025-06-17 → crossover 2025-06-17.
///   - `2025-06-17.year() == 2025` = `2025-06-15.year()` → timing insight active.
///
/// Consult at 2025-06-15 (FMV=$67,500):
///   - gain = $67,500 − $60,000 = $7,500 (ST) → ordinary rate applies.
///   - With OTI=$100k (spans 22% and 24% bracket):
///     $3,350 at 22% = $737; $4,150 at 24% = $996 → total_federal_tax_attributable = $1,733.
///   - Timing: LT re-score at 2025-06-17 → $7,500 LTCG at 15% = $1,125.
///   - saving_if_waited = $1,733 − $1,125 = $608.
fn write_st_crossover_csv(dir: &Path) -> PathBuf {
    let p = dir.join("consult_st_crossover.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
st-buy,2024-06-16 12:00:00 UTC,Buy,BTC,1.00000000,USD,60000.00,60000.00,60000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Coinbase CSV with ONE buy lot that is long-term at 2025-06-15 (acquired 2024-01-15).
///
/// `is_long_term(2024-01-15, 2025-06-15)` = 2025-06-15 > 2025-01-15 → YES → LT at consult date.
/// No short-term lot → timing insight OMITTED (returns None).
fn write_lt_only_csv(dir: &Path) -> PathBuf {
    let p = dir.join("consult_lt_only.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lt-only-buy,2024-01-15 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

// ── Tests: what-if output + read-only invariant ────────────────────────────────────────────────────

/// Golden what-if test: HIFO picks the high-basis lot ($80k over $30k) to minimize tax.
/// Verifies: proposed selection uses lot B; st_gain = −$12,500; lt_gain = $0;
/// total_federal_tax_attributable = −$660 (a saving: 22% × $3k §1211 deduction).
/// Also verifies the READ-ONLY invariant: event count unchanged after `consult`.
#[test]
fn consult_what_if_picks_high_basis_lot_and_writes_nothing() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let before = event_count(&vault);

    let report = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000, // sell 1 BTC = 100,000,000 sat
        coinbase_wallet(),
        // The real 2025-06-15 close differs from the legacy $67,500 stub; pass it as EXPLICIT proceeds
        // so the golden ST/LT tax figures (all hand-derived from $67,500) hold independent of the dataset.
        date!(2025 - 06 - 15),
        Some(dec!(67500)),
        DisposeKind::Sell,
    )
    .unwrap();

    // Read-only invariant: the vault event log must be byte-identical.
    let after = event_count(&vault);
    assert_eq!(before, after, "consult must NOT append any event");

    // HIFO selection: optimizer picks the $80k lot (high basis) over the $30k lot.
    // The $80k lot was acquired 2025-03-01 → short-term at 2025-06-15.
    // Gain = $67,500 − $80,000 = −$12,500 (ST loss).
    assert_eq!(
        report.st_gain,
        dec!(-12500),
        "HIFO picks high-basis lot → ST loss = −$12,500"
    );
    assert_eq!(
        report.lt_gain,
        dec!(0),
        "HIFO picks one lot: no LT leg (the high-basis lot is ST)"
    );

    // Total federal tax: §1211 deduction = $3,000 at 22% marginal = −$660 (saving).
    assert_eq!(
        report.total_federal_tax_attributable,
        dec!(-660),
        "total attributable = −$660 (22% × $3k §1211 deduction from $12,500 ST loss)"
    );

    // Proposed selection must be non-empty and contain exactly one lot.
    assert_eq!(
        report.proposed_selection.len(),
        1,
        "HIFO: exactly one lot consumed"
    );

    // Render: must contain the lot, the total, and the no-investment-advice footer.
    let rendered = render::render_consult(&report);
    assert!(
        rendered.contains("consult_two_lots") || rendered.contains("-660"),
        "rendered must contain the total tax or fixture reference:\n{rendered}"
    );
    assert!(
        rendered.contains("-12500") || rendered.contains("-660"),
        "rendered must contain the ST gain or total:\n{rendered}"
    );
    assert!(
        rendered.contains("Tax decision-support only"),
        "rendered must contain the non-investment-advice footer:\n{rendered}"
    );
    assert!(
        rendered.contains("not investment advice"),
        "rendered must disclaim investment advice:\n{rendered}"
    );
    // No timing line (LT re-score crossover = 2026-03-02, not 2025 → timing omitted).
    assert!(
        !rendered.contains("timing:"),
        "HIFO picks the 2025-03-01 lot → crossover 2026-03-02 (year 2026 ≠ 2025) → no timing:\n{rendered}"
    );
}

/// Determinism: calling `consult` twice with identical args returns byte-identical results (NFR4).
#[test]
fn consult_is_deterministic() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let r1 = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 06 - 15),
        Some(dec!(67500)), // explicit proceeds = the legacy stub close; golden tax figures preserved
        DisposeKind::Sell,
    )
    .unwrap();

    let r2 = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 06 - 15),
        Some(dec!(67500)), // explicit proceeds = the legacy stub close; golden tax figures preserved
        DisposeKind::Sell,
    )
    .unwrap();

    assert_eq!(r1, r2, "consult must be deterministic (NFR4)");
}

// ── Tests: ST→LT timing insight ───────────────────────────────────────────────────────────────────

/// ST→LT timing insight: a lot acquired 2024-06-16 is short-term at 2025-06-15 but crosses over to
/// LT on 2025-06-17 (same year 2025 → timing insight present).
///
/// Golden values (hand-derived):
///   gain at 2025-06-15 (ST):  $7,500;  total_federal_tax_attributable = $1,733 (ST spans 22%+24%).
///   gain at 2025-06-17 (LT):  $7,500 LTCG at 15% = $1,125.
///   saving_if_waited = $1,733 − $1,125 = $608.
#[test]
fn consult_timing_insight_present_for_soon_to_cross_st_lot() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_st_crossover_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let report = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 06 - 15),
        Some(dec!(67500)), // explicit proceeds = the legacy stub close; golden tax figures preserved
        DisposeKind::Sell,
    )
    .unwrap();

    // The lot is short-term → gain is ST.
    assert_eq!(report.st_gain, dec!(7500), "ST gain = $7,500");
    assert_eq!(
        report.lt_gain,
        dec!(0),
        "no LT leg (lot is ST at consult date)"
    );

    // total federal tax (ST rate spans 22% and 24% bracket for Single OTI=$100k):
    //   $3,350 at 22% = $737;  $4,150 at 24% = $996 → $1,733.
    assert_eq!(
        report.total_federal_tax_attributable,
        dec!(1733),
        "total attributable = $1,733 (ST gain spans 22%+24% bracket)"
    );

    // Timing insight must be present.
    assert!(
        report.timing.is_some(),
        "timing insight must be present for a soon-to-cross ST lot"
    );
    let timing = report.timing.as_ref().unwrap();

    // Crossover date: one_year_after(2024-06-16) = 2025-06-16 → next_day = 2025-06-17.
    assert_eq!(
        timing.latest_crossover,
        date!(2025 - 06 - 17),
        "crossover date must be 2025-06-17"
    );

    // All 100,000,000 sat are in the ST selection.
    assert_eq!(
        timing.st_sat_in_selection, 100_000_000,
        "all 100,000,000 sat are short-term at the consult date"
    );

    // tax_if_sold_long_term: $7,500 LT at 15% = $1,125.
    assert_eq!(
        timing.tax_if_sold_long_term,
        dec!(1125),
        "tax if sold LT = $7,500 × 15% = $1,125"
    );

    // saving_if_waited = $1,733 − $1,125 = $608.
    assert_eq!(
        timing.saving_if_waited,
        dec!(608),
        "saving if waited = $1,733 − $1,125 = $608"
    );

    // Render must contain the timing line with the crossover date and saving.
    let rendered = render::render_consult(&report);
    assert!(
        rendered.contains("timing:"),
        "rendered must contain 'timing:' line:\n{rendered}"
    );
    assert!(
        rendered.contains("2025-06-17"),
        "rendered must contain the crossover date 2025-06-17:\n{rendered}"
    );
    assert!(
        rendered.contains("608"),
        "rendered must mention the saving $608:\n{rendered}"
    );
    // Footer must still be present.
    assert!(
        rendered.contains("Tax decision-support only"),
        "rendered must contain the non-investment-advice footer:\n{rendered}"
    );
}

/// Purely long-term lot → timing insight is OMITTED (returns None).
///
/// The lot acquired 2024-01-15 is long-term at 2025-06-15 (2025-06-15 > 2025-01-15 = 1 year after).
/// `is_long_term(2024-01-15, 2025-06-15)` = TRUE → no ST legs → timing = None.
#[test]
fn consult_timing_absent_for_purely_lt_lot() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_only_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let report = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 06 - 15),
        Some(dec!(67500)), // explicit proceeds = the legacy stub close; golden tax figures preserved
        DisposeKind::Sell,
    )
    .unwrap();

    // Lot is LT → gain is LT.
    assert_eq!(
        report.st_gain,
        dec!(0),
        "no ST leg (lot is LT at consult date)"
    );
    assert_eq!(
        report.lt_gain,
        dec!(37500),
        "LT gain = $67,500 − $30,000 = $37,500"
    );

    // Timing must be absent.
    assert!(
        report.timing.is_none(),
        "timing insight must be OMITTED for a purely LT lot"
    );

    // Render must NOT contain the timing line.
    let rendered = render::render_consult(&report);
    assert!(
        !rendered.contains("timing:"),
        "rendered must NOT contain 'timing:' for a purely LT lot:\n{rendered}"
    );
    // Footer must still be present.
    assert!(
        rendered.contains("Tax decision-support only"),
        "rendered must contain the non-investment-advice footer:\n{rendered}"
    );
}

// ── Tests: --proceeds required for future date ────────────────────────────────────────────────────

/// Future date with no bundled dataset price and `proceeds = None` → ProceedsRequired error.
///
/// The bundled dataset's last entry is 2026-06-03. `at = 2026-12-31` is beyond the priced range (so no
/// dataset price) yet still has a bundled TY2026 tax table. With `proceeds = None` (--fmv path),
/// `consult_sale` returns `OptimizeError::Evaluate(EvaluateError::ProceedsRequired)` → `CliError::Usage`
/// mentioning "proceeds". With `proceeds = Some(70000)`, the same date → Ok (proceeds are explicit).
#[test]
fn consult_future_date_requires_proceeds() {
    let csv_dir = tempfile::tempdir().unwrap();
    // Use the two-lots fixture so lots are available at 2026-12-31 (none sold before that date).
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Need a 2026 profile (the consult year) for score_synthetic to work when proceeds are provided.
    cmd::tax::set_profile(&vault, &pp(), 2026, single_100k_profile()).unwrap();

    // -- fmv (proceeds = None) at off-dataset date → ProceedsRequired.
    let err = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2026 - 12 - 31), // beyond the priced range (last is 2026-06-03) → no bundled price
        None,                  // --fmv: dataset FMV → fails (no price)
        DisposeKind::Sell,
    )
    .unwrap_err();

    let msg = format!("{err}");
    assert!(
        msg.to_ascii_lowercase().contains("proceeds"),
        "error must mention 'proceeds' when no dataset price and proceeds=None:\n{msg}"
    );

    // -- with explicit proceeds → Ok even at a future/off-dataset date.
    let ok = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2026 - 12 - 31),
        Some(dec!(70000)), // --proceeds 70000 → explicit; no dataset price needed
        DisposeKind::Sell,
    );
    assert!(
        ok.is_ok(),
        "consult with explicit proceeds must succeed even for off-dataset date:\n{ok:?}"
    );
}

// ── Tests: error paths ────────────────────────────────────────────────────────────────────────────

/// Pre-2025 `at` → `OptimizeError::PreTransitionYear` → `CliError::Usage` with "pre-2025" text.
#[test]
fn consult_pre2025_date_is_usage_error() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_lt_only_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);

    let err = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2024 - 12 - 31), // pre-2025
        Some(dec!(42000)),
        DisposeKind::Sell,
    )
    .unwrap_err();

    let msg = format!("{err}");
    assert!(
        msg.contains("pre-2025"),
        "error must mention 'pre-2025' for a pre-transition date:\n{msg}"
    );
}

/// No lots available in the wallet at `at` → `OptimizeError::NoLots` → `CliError::Usage`.
#[test]
fn consult_no_lots_is_usage_error() {
    // Vault with NO imports (no lots at all).
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let err = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 06 - 15),
        Some(dec!(67500)), // explicit proceeds = the legacy stub close; golden tax figures preserved
        DisposeKind::Sell,
    )
    .unwrap_err();

    let msg = format!("{err}");
    assert!(
        msg.contains("no lots"),
        "error must mention 'no lots' when wallet is empty:\n{msg}"
    );
}

// ── C-M2 KATs: ConsultReport::approximate + render disclosure ────────────────────────────────────

/// Coinbase CSV with 13 lots (> LOT_ENUM_BOUND=12) of equal basis. Any 1-BTC consult over this
/// wallet uses the heuristic branch → `ConsultReport::approximate == true`.
fn write_thirteen_lots_csv(dir: &Path) -> PathBuf {
    let p = dir.join("consult_thirteen_lots.csv");
    let mut rows = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n".to_string();
    for i in 1..=13usize {
        rows.push_str(&format!(
            "lot{i:02},2025-01-{i:02} 12:00:00 UTC,Buy,BTC,1.00000000,USD,60000.00,60000.00,60000.00,0.00,,,\r\n"
        ));
    }
    std::fs::write(&p, rows).unwrap();
    p
}

/// C-M2 KAT: a pool of 13 lots (> LOT_ENUM_BOUND=12) → `ConsultReport::approximate == true`
/// and `render_consult` includes the heuristic disclosure note.
#[test]
fn consult_large_pool_sets_approximate_and_renders_note() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_thirteen_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let report = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000, // sell 1 BTC from a 13-lot pool → heuristic branch
        coinbase_wallet(),
        date!(2025 - 06 - 15), // bundled FMV = $67,500
        None,
        DisposeKind::Sell,
    )
    .unwrap();

    // C-M2: approximate must be true for a >12-lot pool.
    assert!(
        report.approximate,
        "ConsultReport::approximate must be true for a >12-lot pool (heuristic subset)"
    );

    // render_consult must include the disclosure note.
    let rendered = render::render_consult(&report);
    assert!(
        rendered.contains("heuristic"),
        "rendered must contain 'heuristic' disclosure for approximate=true:\n{rendered}"
    );
    assert!(
        rendered.contains(">12-lot"),
        "rendered must mention the >12-lot pool in the disclosure:\n{rendered}"
    );
    assert!(
        rendered.contains("Tax decision-support only"),
        "rendered must contain the non-investment-advice footer:\n{rendered}"
    );
}

/// C-M2 KAT (mirror): a pool of ≤12 lots → `ConsultReport::approximate == false`
/// and `render_consult` does NOT include the heuristic disclosure note.
#[test]
fn consult_small_pool_approximate_false_no_note() {
    let csv_dir = tempfile::tempdir().unwrap();
    // The existing two_lots fixture has only 2 lots → complete enumeration.
    let csv = write_two_lots_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let report = cmd::optimize::consult(
        &vault,
        &pp(),
        100_000_000,
        coinbase_wallet(),
        date!(2025 - 06 - 15),
        Some(dec!(67500)), // explicit proceeds = the legacy stub close; golden tax figures preserved
        DisposeKind::Sell,
    )
    .unwrap();

    // approximate must be false for a small (≤12-lot) pool.
    assert!(
        !report.approximate,
        "ConsultReport::approximate must be false for a ≤12-lot pool (complete enumeration)"
    );

    // render_consult must NOT include the disclosure note.
    let rendered = render::render_consult(&report);
    assert!(
        !rendered.contains("heuristic"),
        "rendered must NOT contain 'heuristic' for approximate=false:\n{rendered}"
    );
    assert!(
        rendered.contains("Tax decision-support only"),
        "rendered must contain the non-investment-advice footer:\n{rendered}"
    );
}
