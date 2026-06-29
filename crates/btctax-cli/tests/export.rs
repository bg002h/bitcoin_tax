mod fixtures;
use btctax_cli::cmd;
use btctax_store::Passphrase;
use csv::Reader;
use std::fs::File;

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
