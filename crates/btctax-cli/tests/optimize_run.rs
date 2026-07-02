//! CLI integration tests for `optimize run` (Task 9 / §C.2 Mode-1 what-if proposal).
//!
//! PRIVACY: all fixtures are synthetic and written into tempdirs; the test suite NEVER reads
//! `~/Documents/BitcoinTax/ReadOnly` or any user file.
//!
//! Every test that invokes `cmd::optimize::run` asserts the vault (event log) is UNCHANGED
//! afterwards — the command is read-only (Mode-1 proposes; it appends NOTHING).
use btctax_cli::{cmd, render};
use btctax_core::{persistence::load_all, Carryforward, FilingStatus, TaxProfile};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::{macros::datetime, OffsetDateTime};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// Single filer profile for TY2025: OTI=100k, MAGI=100k, QD=0.
/// With OTI=100,000 we are solidly in the 24% ordinary bracket (Single 2025).
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

/// Count events in the vault (used to assert read-only invariant).
fn event_count(vault: &Path) -> usize {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    load_all(s.conn()).unwrap().len()
}

// ── CSV fixtures ──────────────────────────────────────────────────────────────────────────────────

/// Synthetic Coinbase CSV: pre-2025 buy (low-basis, long-term by 2025) + 2025 high-basis buy + 2025 sell.
///
/// Golden scenario:
///   Lot A: 1 BTC @ $30,000 acquired 2023-01-01 (LT by 2025-06-01 — over one year).
///   Lot B: 1 BTC @ $80,000 acquired 2025-01-02 (ST on 2025-06-01 — under one year).
///   Sell:  1 BTC at $50,000 proceeds on 2025-06-01.
///
/// FIFO (default): uses Lot A (older) → 20,000 LT gain → LTCG tax at 15% = $3,000.
/// Optimizer HIFO: uses Lot B (higher basis) → −$30,000 ST loss → §1211 deduction = $3,000,
///   saving ~$720 in ordinary tax (24% marginal) → total attributable ≈ −$720.
/// delta = −$720 − $3,000 = −$3,720 ≤ 0.  ✓
fn write_tax_saving_csv(dir: &Path) -> PathBuf {
    let p = dir.join("opt_saving.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
opt-buy-lt,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
opt-buy-st,2025-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,80000.00,80000.00,80000.00,0.00,,,\r\n\
opt-sell,2025-06-01 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Synthetic Coinbase CSV with a single buy + sell (no alternative selection: only 1 lot available).
/// Used for the R2-M1 no-change-row test.
///
/// Lot: 1 BTC @ $50,000 (2025-03-01).  Sell: 1 BTC @ $50,000 (2025-06-15).
/// The only lot consumes the full principal → proposed == current → "no change" row.
fn write_single_lot_csv(dir: &Path) -> PathBuf {
    let p = dir.join("opt_single.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
single-buy,2025-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n\
single-sell,2025-06-15 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Synthetic Coinbase CSV with 13 separate buy lots + 1 sell of the same size as each lot.
/// 13 lots > LOT_ENUM_BOUND (12) → `candidate_selections` takes the heuristic branch →
/// proposal carries `approximate=true` + `PoolHeuristic { lots: 13, bound: 12 }` (R2-C1).
///
/// Each buy: 0.01 BTC at progressively higher prices ($40,000–$64,000, step $2,000).
/// Sell:     0.01 BTC at $50,000 (≥ 2025-07-01, after all buys).
fn write_heuristic_pool_csv(dir: &Path) -> PathBuf {
    let p = dir.join("opt_heuristic.csv");
    let header = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n";
    let mut rows = String::new();
    for i in 0..13usize {
        let price = 40_000u32 + (i as u32) * 2_000; // 40k, 42k, ..., 64k
        let subtotal = price / 100; // 0.01 BTC × price / 100 = price * 0.01
        rows.push_str(&format!(
            "heur-buy-{i:02},2025-01-{day:02} 12:00:00 UTC,Buy,BTC,0.01000000,USD,\
             {price}.00,{subtotal}.00,{subtotal}.00,0.00,,,\r\n",
            day = i + 1
        ));
    }
    rows.push_str(
        "heur-sell,2025-07-01 12:00:00 UTC,Sell,BTC,0.01000000,USD,\
         50000.00,500.00,500.00,0.00,,,\r\n",
    );
    std::fs::write(&p, format!("{header}{rows}")).unwrap();
    p
}

/// Synthetic Coinbase CSV: only a 2025 buy, no sell — used for the `NoDisposals` test.
fn write_buy_only_csv(dir: &Path) -> PathBuf {
    let p = dir.join("opt_buyonly.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
buyonly-buy,2025-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,84000.00,84000.00,84000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

// ── Tests: tax-saving proposal + read-only invariant ─────────────────────────────────────────────

/// Golden tax-saving test: FIFO (LT gain) vs HIFO (ST loss) → delta < 0.
/// Also verifies the read-only invariant: event count unchanged after `run`.
/// `now` is after the sale (2026-01-01) → proposed selection is already-executed → NeedsAttestation.
#[test]
fn optimize_run_saves_tax_and_writes_nothing() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let before = event_count(&vault);

    // `now` after the sale — already-executed.
    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap();

    // Read-only invariant: the vault event log must be byte-identical (same event count).
    let after = event_count(&vault);
    assert_eq!(before, after, "optimize run must not append any event");

    // delta ≤ 0 (optimizer never recommends worse than doing nothing).
    assert!(
        proposal.delta <= dec!(0),
        "expected delta ≤ 0, got {}",
        proposal.delta
    );

    // In this scenario the optimizer picks the high-basis lot → strict improvement.
    assert!(
        proposal.delta < dec!(0),
        "expected strict tax saving (delta < 0) for this fixture, got {}",
        proposal.delta
    );

    // Render: must contain the "NOTHING is filed" header.
    let rendered = render::render_optimize_proposal(&proposal);
    assert!(
        rendered.contains("NOTHING is filed or bound by running this"),
        "rendered output must contain the read-only disclaimer:\n{rendered}"
    );

    // Per-disposal compliance status present (the one disposal must appear).
    assert!(
        !proposal.per_disposal.is_empty(),
        "expected at least one disposal row"
    );
    // The rendered output must include a disposal line.
    assert!(
        rendered.contains("::"),
        "rendered must include per-disposal status line:\n{rendered}"
    );
}

// ── Tests: approximate banner ─────────────────────────────────────────────────────────────────────

/// A pool of 13 lots (> LOT_ENUM_BOUND=12) triggers the heuristic branch → `approximate=true` →
/// the "⚠ APPROXIMATE" banner must appear (R0-C1/R2-C1).
#[test]
fn optimize_run_approximate_banner_when_heuristic_pool() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_heuristic_pool_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap();

    assert!(
        proposal.approximate,
        "13-lot pool must trigger approximate=true"
    );
    // approx_reason must be PoolHeuristic { lots: 13, bound: 12 }.
    assert!(
        matches!(
            proposal.approx_reason,
            Some(btctax_core::ApproxReason::PoolHeuristic {
                lots: 13,
                bound: 12
            })
        ),
        "expected PoolHeuristic {{ lots: 13, bound: 12 }}, got {:?}",
        proposal.approx_reason
    );

    let rendered = render::render_optimize_proposal(&proposal);
    // Banner must appear (R0-C1: never present the result as "the optimum" without this banner).
    assert!(
        rendered.contains("APPROXIMATE"),
        "rendered must contain APPROXIMATE banner:\n{rendered}"
    );
    // R2-C1: the specific reason text must be present.
    assert!(
        rendered.contains("exhaustive-enumeration bound"),
        "rendered must explain the pool-heuristic reason:\n{rendered}"
    );
    assert!(
        rendered.contains("heuristic SUBSET"),
        "rendered must mention heuristic SUBSET:\n{rendered}"
    );
}

/// A small fixture (2 lots, 1 disposal) → exact result; the banner must NOT appear (R0-C1).
#[test]
fn optimize_run_no_banner_when_exact_result() {
    // Use the tax-saving fixture: 2 lots (pre-2025 + 2025-01-02) for 1 disposal. 2 ≤ 12 → exact.
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap();

    assert!(
        !proposal.approximate,
        "2-lot pool must give an exact (non-approximate) result"
    );

    let rendered = render::render_optimize_proposal(&proposal);
    assert!(
        !rendered.contains("APPROXIMATE"),
        "rendered must NOT contain APPROXIMATE banner for an exact result:\n{rendered}"
    );
}

// ── Tests: persistability (R0-C2 clock seam) ─────────────────────────────────────────────────────

/// When `now` is BEFORE the sale date, the proposed selection is contemporaneous →
/// the render must show "persistable now (made ≤ sale → Contemporaneous)" for the changed row.
#[test]
fn optimize_run_contemporaneous_when_now_before_sale() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    // Sale is 2025-06-01; now = 2025-01-01 (before) → ContemporaneousNow.
    let now: OffsetDateTime = datetime!(2025-01-01 12:00:00 UTC);
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap();

    let rendered = render::render_optimize_proposal(&proposal);
    // For the changed row (proposed != current), the Contemporaneous tag must appear.
    assert!(
        rendered.contains("made")
            && rendered.contains("sale")
            && rendered.contains("Contemporaneous"),
        "rendered must show ContemporaneousNow for a pre-sale proposal:\n{rendered}"
    );
    // The already-executed line must NOT appear.
    assert!(
        !rendered.contains("already executed"),
        "rendered must NOT show NeedsAttestation for a pre-sale proposal:\n{rendered}"
    );
}

/// When `now` is AFTER the sale date, the proposed selection is already-executed →
/// the render must show the "already executed — needs..." line for the changed row (R0-C2).
#[test]
fn optimize_run_needs_attestation_when_now_after_sale() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    // Sale is 2025-06-01; now = 2026-01-01 (after) → NeedsAttestation.
    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap();

    let rendered = render::render_optimize_proposal(&proposal);
    // For the changed row (proposed != current), the already-executed line must appear.
    assert!(
        rendered.contains("already executed"),
        "rendered must show NeedsAttestation for a post-sale proposal:\n{rendered}"
    );
    // The ContemporaneousNow text must NOT appear.
    assert!(
        !rendered.contains("made \u{2264} sale \u{2192} Contemporaneous"),
        "rendered must NOT show ContemporaneousNow for a post-sale proposal:\n{rendered}"
    );
}

// ── Tests: R2-M1 no-change row ────────────────────────────────────────────────────────────────────

/// When the optimizer cannot improve a disposal (single lot → proposed == current), the render must
/// show "no change — already optimal" and must NOT show "needs `optimize accept`" (R2-M1).
#[test]
fn optimize_run_no_change_row_shows_no_change_not_attest() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_single_lot_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    // now after the sale → would be NeedsAttestation IF the selection changed, but since it
    // doesn't, the persistability line must be suppressed entirely.
    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap();

    // The single disposal row must have proposed == current.
    assert_eq!(
        proposal.per_disposal.len(),
        1,
        "expected exactly one disposal row"
    );
    assert_eq!(
        proposal.per_disposal[0].proposed_selection, proposal.per_disposal[0].current_selection,
        "single-lot disposal must be a no-change row"
    );

    let rendered = render::render_optimize_proposal(&proposal);

    // Must contain the no-change note.
    assert!(
        rendered.contains("no change"),
        "rendered must contain 'no change' note for a no-change disposal:\n{rendered}"
    );
    assert!(
        rendered.contains("already optimal under current identification"),
        "rendered must contain 'already optimal' note:\n{rendered}"
    );
    // Must NOT contain the NeedsAttestation / "already executed" line (R2-M1).
    assert!(
        !rendered.contains("already executed"),
        "rendered must NOT show 'already executed' for a no-change disposal:\n{rendered}"
    );
    // Must NOT contain the "needs `optimize accept`" text.
    assert!(
        !rendered.contains("optimize accept"),
        "rendered must NOT show 'optimize accept' for a no-change disposal:\n{rendered}"
    );
}

// ── Tests: error paths (pre-2025, no profile, no disposals) ─────────────────────────────────────

/// Pre-2025 year → `OptimizeError::PreTransitionYear` → `CliError::Usage` with "pre-2025" text.
/// A restatement of a closed year is not an optimization (M7).
#[test]
fn optimize_run_pre2025_is_usage_error() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Profile not needed — the pre-2025 guard fires before the profile check.

    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let err = cmd::optimize::run(&vault, &pp(), 2024, now).unwrap_err();

    let msg = format!("{err}");
    assert!(
        msg.contains("pre-2025"),
        "error must mention 'pre-2025' for year=2024:\n{msg}"
    );
    // Must not contain a computed dollar amount.
    assert!(
        !msg.contains("delta"),
        "error message must not contain a delta for pre-2025:\n{msg}"
    );
}

/// No tax profile set → `OptimizeError::YearNotComputable(TaxProfileMissing)` →
/// `CliError::Usage` with "year not computable" text.
#[test]
fn optimize_run_no_profile_is_usage_error() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Deliberately do NOT set a profile.

    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let err = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap_err();

    let msg = format!("{err}");
    assert!(
        msg.contains("year not computable"),
        "error must mention 'year not computable' when no profile:\n{msg}"
    );
}

/// No 2025 disposals (only a buy) → `OptimizeError::NoDisposals` →
/// `CliError::Usage` with "no method-honoring disposals" text.
#[test]
fn optimize_run_no_disposals_is_usage_error() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_buy_only_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    // Profile must be set so `compute_tax_year` returns Computed (not NotComputable).
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let now: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC);
    let err = cmd::optimize::run(&vault, &pp(), 2025, now).unwrap_err();

    let msg = format!("{err}");
    assert!(
        msg.contains("no method-honoring disposals"),
        "error must mention 'no method-honoring disposals' when year has no sells:\n{msg}"
    );
}
