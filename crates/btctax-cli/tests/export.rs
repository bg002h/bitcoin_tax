mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_core::OutflowClass;
use btctax_store::Passphrase;
use csv::Reader;
use std::fs::File;
use time::macros::datetime;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn export_snapshot_writes_sqlite_and_csvs_and_backup_key() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    let out = dir.path().join("export");
    let sqlite = cmd::admin::export_snapshot(&vault, &pp(), &out).unwrap();
    assert!(sqlite.exists(), "snapshot.sqlite (store)");

    let lots_path = out.join("lots.csv");
    let disposals_path = out.join("disposals.csv");
    let removals_path = out.join("removals.csv");
    let income_path = out.join("income.csv");

    assert!(lots_path.exists());
    assert!(disposals_path.exists());
    assert!(removals_path.exists());
    assert!(income_path.exists());

    // Strengthen: read back lots.csv and verify content (not just existence).
    // coinbase_buy_sell_send creates:
    //   - Buy 0.10000000 BTC (10,000,000 sat)
    //   - Sell 0.02000000 BTC (2,000,000 sat)
    //   - Send 0.03000000 BTC (3,000,000 sat)
    // Expected remaining: 10,000,000 - 2,000,000 - 3,000,000 = 5,000,000 sat
    let mut reader = Reader::from_reader(File::open(&lots_path).unwrap());
    let records: Vec<_> = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read lots.csv records");

    assert!(
        !records.is_empty(),
        "lots.csv must contain at least one data record"
    );

    // Verify that at least one lot has the expected remaining_sat value.
    // The remaining_sat column is at index 4 (0-indexed).
    let expected_remaining_sat = "5000000";
    let found_expected = records
        .iter()
        .any(|rec| rec.get(4) == Some(expected_remaining_sat));

    assert!(
        found_expected,
        "Expected to find a lot with remaining_sat={}, but none was found in lots.csv",
        expected_remaining_sat
    );

    // Strengthen: verify that disposals.csv uses stable tag strings (not Debug repr).
    // coinbase_buy_sell_send: Buy 2025-03-01, Sell 2025-06-15 — less than 1 year → short-term.
    // The `term` column (index 8) must read "short", not the Debug form "ShortTerm".
    let mut dreader = Reader::from_reader(File::open(&disposals_path).unwrap());
    let disposal_records: Vec<_> = dreader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read disposals.csv records");
    assert!(
        !disposal_records.is_empty(),
        "disposals.csv must contain at least one data record (the Sell event)"
    );
    // term column is at index 8 (0-indexed): event,kind,disposed_at,lot,sat,proceeds,basis,gain,term,gift_zone
    let term_col = disposal_records[0].get(8).expect("term column missing");
    assert!(
        term_col == "short" || term_col == "long",
        "term column must use stable tag ('short'/'long'), got: {:?}",
        term_col
    );
    // The fixture sell is short-term (bought and sold in 2025, within a year).
    assert_eq!(
        term_col, "short",
        "fixture sell (Mar→Jun 2025) must be short-term, got: {:?}",
        term_col
    );

    // Verify basis_source column in lots.csv uses stable tags (not Debug repr).
    // The Buy is via Coinbase adapter → BasisSource::ExchangeProvided → tag "exchange".
    // Re-read lots.csv (reader was already consumed above; open fresh).
    let mut lreader2 = Reader::from_reader(File::open(&lots_path).unwrap());
    let lot_records: Vec<_> = lreader2
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to re-read lots.csv records");
    // basis_source column is at index 6: origin_event,split,wallet,acquired_at,remaining_sat,usd_basis,basis_source,basis_pending
    let bs_col = lot_records[0].get(6).expect("basis_source column missing");
    assert_eq!(
        bs_col, "exchange",
        "basis_source column must use stable tag 'exchange' (not Debug 'ExchangeProvided'), got: {:?}",
        bs_col
    );

    let key = dir.path().join("backup.asc");
    cmd::admin::backup_key(&vault, &pp(), &key).unwrap();
    assert!(key.exists());
}

/// (P2-A Minor fix KAT) Multi-leg donation: removals.csv must show `claimed_deduction` on the
/// FIRST leg row only; subsequent leg rows carry an empty cell so `SUM()` over the column equals
/// the correct per-donation total without double-counting.
///
/// Setup: two lots consumed by one Donation reclassification → `make_removal_legs` yields 2 legs.
///   Lot A: LT (2024-01-01), 1 BTC, basis $5,000 → FMV pro-rata $50,000 → deduction $50,000.
///   Lot B: ST (2025-12-01), 1 BTC, basis $2,000 → FMV pro-rata $50,000 → min($50k,$2k) = $2,000.
///   Total claimed_deduction = $52,000.
///
/// Before fix: both rows carried "52000.00" → SUM = $104,000 (double-counted).
/// After fix:  row 0 = "52000.00", row 1 = "" → SUM = $52,000 (correct).
#[test]
fn removals_csv_multi_leg_donation_no_double_count() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-04-01 12:00:00 UTC);

    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_two_lot_donation(dir.path())],
    )
    .unwrap();

    // The 2-BTC Send is pending; retrieve its event ref for reclassification.
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert_eq!(
            state.pending_reconciliation.len(),
            1,
            "Send must be pending before reclassification"
        );
        state.pending_reconciliation[0].event.canonical()
    };

    // Reclassify as Donate with total FMV $100,000 (2 BTC × $50k each, pro-rata by sat).
    // FIFO consumes both lots: LT leg ($50k FMV → $50k deduction) + ST leg ($2k basis,
    // $50k FMV → min($50k,$2k) = $2k deduction).  Total claimed_deduction = $52,000.
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("100000.00").unwrap(),
        None,
        now,
    )
    .unwrap();

    let out = dir.path().join("export");
    cmd::admin::export_snapshot(&vault, &pp(), &out).unwrap();

    let removals_path = out.join("removals.csv");
    assert!(removals_path.exists(), "removals.csv must exist");

    // claimed_deduction column index 8:
    // event(0), kind(1), removed_at(2), lot(3), sat(4), basis(5), fmv_at_transfer(6), term(7),
    // claimed_deduction(8).
    const DED_COL: usize = 8;

    let mut reader = Reader::from_reader(File::open(&removals_path).unwrap());
    let records: Vec<_> = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to read removals.csv records");

    assert_eq!(
        records.len(),
        2,
        "expected 2 removal leg rows (one per lot consumed by the donation); got {}",
        records.len()
    );

    let row0_ded = records[0]
        .get(DED_COL)
        .expect("row 0 claimed_deduction missing");
    let row1_ded = records[1]
        .get(DED_COL)
        .expect("row 1 claimed_deduction missing");

    // First leg: full per-donation deduction total.
    assert_eq!(
        row0_ded, "52000.00",
        "first leg row must carry the full claimed_deduction ($52,000); got {row0_ded:?}"
    );

    // Subsequent legs: empty cell so a naive SUM() does not double-count.
    assert!(
        row1_ded.is_empty(),
        "second leg row must have an empty claimed_deduction (not {row1_ded:?}); \
         a non-empty value means the deduction is double-counted in a spreadsheet SUM()"
    );

    // Invariant: SUM over the column == single donation total.
    let sum: f64 = records
        .iter()
        .filter_map(|r| r.get(DED_COL))
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f64>()
                .expect("non-empty claimed_deduction must be numeric")
        })
        .sum();
    assert!(
        (sum - 52_000.0_f64).abs() < 0.01,
        "SUM of claimed_deduction column must equal $52,000 (no double-count); got {sum}"
    );
}

/// CLI-I1 fix: exported CSVs are owner-only (0o600) even when the out-dir PRE-EXISTS.
/// The old `Writer::from_path` path created files at the process umask (typically 0o644 —
/// world-readable). The fix routes each CSV through `fsperms::open_owner_only` (0o600 on
/// Unix create-or-truncate) so the mode is hardened regardless of umask or dir pre-existence.
#[cfg(unix)]
#[test]
fn csv_exports_are_owner_only_on_pre_existing_dir() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // Create the out-dir BEFORE the export so it pre-exists — this is the hole the fix closes.
    let out = dir.path().join("exports_pre");
    std::fs::create_dir_all(&out).unwrap();

    cmd::admin::export_snapshot(&vault, &pp(), &out).unwrap();

    for name in ["lots.csv", "disposals.csv", "removals.csv", "income.csv"] {
        let path = out.join(name);
        assert!(path.exists(), "{name} must exist");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "{name} must be owner-only (0o600), got {:#o}",
            mode & 0o777
        );
    }
}
