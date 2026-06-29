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

#[test]
fn verify_reports_conservation_and_advisory_pending_no_hard_blockers() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_sell_send(dir.path());
    cmd::import::run(&vault, &pp(), &[file]).unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    // The Send sits in pending_reconciliation (advisory unmatched_outflows); conservation still balances.
    assert!(
        report.conservation.balanced,
        "Σpending closes the FR9 identity"
    );
    assert_eq!(report.pending, 1);
    assert!(report.hard.is_empty(), "no hard blockers -> exit 0");
    assert!(!report.has_hard_blockers());

    let text = render::render_verify(&report);
    assert!(text.contains("Conservation"));
    assert!(text.contains("Path A")); // default 2025 transition, no allocation
}

#[test]
fn verify_surfaces_hard_blocker_and_signals_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_receive(dir.path());
    cmd::import::run(&vault, &pp(), &[file]).unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    // The Receive without a ClassifyInbound decision → Op::UnknownInbound → hard blocker.
    assert!(
        !report.hard.is_empty(),
        "unclassified TransferIn → hard UnknownBasisInbound blocker"
    );
    assert!(
        report.has_hard_blockers(),
        "has_hard_blockers() true → caller maps to non-zero exit (§7.1)"
    );
    assert!(
        report.unknown_basis_inbounds > 0,
        "unknown_basis_inbounds counter reflects the hard blocker count"
    );
}

#[test]
fn config_set_fee_treatment_b_persists_and_affects_projection_config() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let before = cmd::admin::show_config(&vault, &pp()).unwrap();
    assert_eq!(before.fee_treatment, btctax_core::FeeTreatment::TreatmentC); // default (c)

    let after =
        cmd::admin::set_config(&vault, &pp(), Some(btctax_core::FeeTreatment::TreatmentB)).unwrap();
    assert_eq!(after.fee_treatment, btctax_core::FeeTreatment::TreatmentB);

    // Reopen: persisted across sessions; projection picks it up.
    let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
    assert_eq!(
        s.config().unwrap().fee_treatment,
        btctax_core::FeeTreatment::TreatmentB
    );
}
