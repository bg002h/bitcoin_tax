mod fixtures;
use btctax_cli::{cmd, render};
use btctax_store::Passphrase;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn report_shows_lots_and_year_filtered_disposals() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_sell_send(dir.path());
    cmd::import::run(&vault, &pp(), &[file]).unwrap();

    let state = cmd::inspect::report(&vault, &pp(), Some(2025)).unwrap();
    // The Buy minus the Sell leaves a remaining lot; the Sell is a 2025 disposal.
    assert!(!state.lots.is_empty());
    assert_eq!(state.disposals.len(), 1);

    let text = render::render_report(&state, Some(2025));
    assert!(text.contains("Holdings"));
    assert!(text.contains("Disposals"));

    // A year with no realized events renders the sections empty (no panic, no disposals listed).
    let none = render::render_report(&state, Some(1999));
    assert!(none.contains("Disposals (year 1999): none"));
}
