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

    let key = dir.path().join("backup.asc");
    cmd::admin::backup_key(&vault, &pp(), &key).unwrap();
    assert!(key.exists());
}
