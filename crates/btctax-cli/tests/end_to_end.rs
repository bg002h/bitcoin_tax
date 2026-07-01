mod fixtures;
use btctax_cli::{cmd, render, Session};
use btctax_core::{DisposeKind, OutflowClass};
use btctax_store::Passphrase;
use time::macros::datetime;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

#[test]
fn full_lifecycle_init_import_verify_reconcile_report() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let now = datetime!(2026-02-01 12:00:00 UTC);

    // init
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // import synthetic Coinbase (Buy + Sell + Send→pending)
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();

    // verify: the Send is pending (advisory), conservation balances, no hard blockers
    let v1 = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(v1.conservation.balanced);
    assert_eq!(v1.pending, 1);
    assert!(!v1.has_hard_blockers());

    // reconcile: reclassify the pending Send as a Sell (discover its eventref from the projection)
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (st, _) = s.project().unwrap();
        st.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        },
        btctax_cli::eventref::parse_usd_arg("2050.00").unwrap(),
        Some(btctax_cli::eventref::parse_usd_arg("2.50").unwrap()),
        None,
        now,
    )
    .unwrap();

    // report: two 2025 disposals now (the original Sell + the reclassified Send)
    let state = cmd::inspect::report(&vault, &pp(), Some(2025)).unwrap();
    assert_eq!(state.disposals.len(), 2);
    let text = render::render_report(&state, Some(2025));
    assert!(text.contains("Disposals (year 2025)"));

    // verify again: nothing pending; still no hard blockers
    let v2 = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert_eq!(v2.pending, 0);
    assert!(!v2.has_hard_blockers());
}
