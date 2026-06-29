mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_store::Passphrase;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn import_appends_btc_events_and_reports_fr2_counts() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let file = fixtures::coinbase_single_buy(dir.path());
    let (reports, import) = cmd::import::run(&vault, &pp(), &[file]).unwrap();

    // One Coinbase group; the ETH row is dropped (no BTC leg, FR2); the Buy is appended.
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].dropped_no_btc, 1);
    assert_eq!(import.appended, 1);
    assert_eq!(import.conflicts, 0);

    // Idempotent re-import (FR1): same rows → zero new appends, zero conflicts.
    let file2 = fixtures::coinbase_single_buy(dir.path());
    let (_r2, import2) = cmd::import::run(&vault, &pp(), &[file2]).unwrap();
    assert_eq!(import2.appended, 0);
    assert_eq!(import2.duplicates, 1);
    assert_eq!(import2.conflicts, 0);

    // The appended Acquire is visible to the projection.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.lots.len(), 1);
}
