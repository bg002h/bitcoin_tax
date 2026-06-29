mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_core::{
    BlockerKind, DisposeKind, EventPayload, InboundClass, IncomeKind, OutflowClass, TransferTarget,
};
use btctax_store::Passphrase;
use time::macros::datetime;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026-02-01 12:00:00 UTC) // fixed decision clock (NFR4 deterministic tests)
}

fn coinbase_with_receive(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("cb_recv.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n").unwrap();
    p
}

/// Import the buy/sell/send fixture and return (vault_path, the TransferOut's canonical eventref).
fn vault_with_pending(dir: &std::path::Path) -> (std::path::PathBuf, String) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let file = fixtures::coinbase_buy_sell_send(dir);
    cmd::import::run(&vault, &pp(), &[file]).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let out_ref = state.pending_reconciliation[0].event.canonical();
    (vault, out_ref)
}

#[test]
fn classify_inbound_income_resolves_unknown_basis() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_receive(dir.path())]).unwrap();

    let in_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .unwrap()
            .id
            .canonical()
    };

    let class = InboundClass::Income {
        kind: IncomeKind::Reward,
        fmv: Some(btctax_cli::eventref::parse_usd_arg("4200.00").unwrap()),
        business: false,
    };
    cmd::reconcile::classify_inbound(&vault, &pp(), &in_ref, class, now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    // The classified inbound is recognized income; no unknown-basis blocker remains.
    assert_eq!(state.income_recognized.len(), 1);
    assert!(state
        .blockers
        .iter()
        .all(|b| b.kind != btctax_core::BlockerKind::UnknownBasisInbound));
}

#[test]
fn link_transfer_clears_pending_and_relocates_lots() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    let target =
        TransferTarget::Wallet(btctax_cli::eventref::parse_wallet_id("self:cold").unwrap());
    let id = cmd::reconcile::link_transfer(&vault, &pp(), &out_ref, target, now()).unwrap();
    assert!(matches!(id, btctax_core::EventId::Decision { seq: 1 }));

    // Re-project: the TransferOut is no longer pending (it became a self-transfer; TP7).
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.pending_reconciliation.is_empty());
    // The decision is persisted as a TransferLink.
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e.payload, EventPayload::TransferLink(_))));
}

#[test]
fn reclassify_outflow_to_sell_creates_a_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        },
        btctax_cli::eventref::parse_usd_arg("2000.00").unwrap(),
        Some(btctax_cli::eventref::parse_usd_arg("3.00").unwrap()),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.pending_reconciliation.is_empty()); // outflow resolved
    assert_eq!(state.disposals.len(), 2); // the fixture Sell + the reclassified Send
}

#[test]
fn reclassify_outflow_to_gift_creates_a_removal() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::GiftOut,
        btctax_cli::eventref::parse_usd_arg("2040.00").unwrap(),
        None,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.pending_reconciliation.is_empty()); // outflow resolved
    assert_eq!(state.removals.len(), 1); // GiftOut → Removal, zero gain
}

#[test]
fn reclassify_outflow_to_donate_creates_a_removal_with_appraisal_flag() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: true,
        },
        btctax_cli::eventref::parse_usd_arg("2040.00").unwrap(),
        None,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.pending_reconciliation.is_empty()); // outflow resolved
    assert_eq!(state.removals.len(), 1);
    assert!(state.removals[0].appraisal_required);
}

#[test]
fn void_drops_a_revocable_decision() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());
    let link = cmd::reconcile::link_transfer(
        &vault,
        &pp(),
        &out_ref,
        TransferTarget::Wallet(btctax_cli::eventref::parse_wallet_id("self:cold").unwrap()),
        now(),
    )
    .unwrap();

    // Void the link by its decision eventref; the outflow returns to pending.
    cmd::reconcile::void(&vault, &pp(), &link.canonical(), now()).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.pending_reconciliation.len(), 1);
}

#[test]
fn set_fmv_appends_a_manual_fmv_decision() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _out_ref) = vault_with_pending(dir.path());
    // Target the Buy event (any event id parses); the decision is appended + persisted.
    let target = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::Acquire(_)))
            .unwrap()
            .id
            .canonical()
    };
    let id = cmd::reconcile::set_fmv(
        &vault,
        &pp(),
        &target,
        btctax_cli::eventref::parse_usd_arg("123.45").unwrap(),
        now(),
    )
    .unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(events
        .iter()
        .any(|e| e.id == id && matches!(e.payload, EventPayload::ManualFmv(_))));
}

// ── Task 12: classify-raw + accept/reject-conflict ──────────────────────────

fn coinbase_with_order(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("cb_order.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-ord,2025-03-01 12:00:00 UTC,Order,BTC,0.01000000,USD,84000.00,840.00,845.00,5.00,,,\r\n").unwrap();
    p
}

/// A second Coinbase CSV with the same ID `cb-ord` but different amounts → ImportConflict on re-import.
fn coinbase_with_order_v2(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("cb_order_v2.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-ord,2025-03-01 12:00:00 UTC,Order,BTC,0.01000000,USD,84000.00,840.00,860.00,20.00,,,\r\n").unwrap();
    p
}

#[test]
fn classify_raw_resolves_an_unclassified_row() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_order(dir.path())]).unwrap();

    let target = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::Unclassified(_)))
            .unwrap()
            .id
            .canonical()
    };
    // Supply an Acquire payload as JSON (EventPayload is Deserialize).
    let json = r#"{"Acquire":{"sat":1000000,"usd_cost":"845.00","fee_usd":"5.00","basis_source":"ComputedFromCost"}}"#;
    cmd::reconcile::classify_raw(&vault, &pp(), &target, json, now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    // No Unclassified blocker remains; a lot now exists.
    assert!(state
        .blockers
        .iter()
        .all(|b| b.kind != btctax_core::BlockerKind::Unclassified));
    assert_eq!(state.lots.len(), 1);
}

#[test]
fn classify_raw_rejects_decision_payload() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_order(dir.path())]).unwrap();

    let target = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::Unclassified(_)))
            .unwrap()
            .id
            .canonical()
    };
    // A decision payload (ManualFmv) must be rejected by the is_imported guard.
    let bad_json = r#"{"ManualFmv":{"event":"d:1","usd_fmv":"100.00"}}"#;
    let err = cmd::reconcile::classify_raw(&vault, &pp(), &target, bad_json, now());
    assert!(err.is_err(), "expected Err for non-imported payload");
}

#[test]
fn accept_conflict_clears_import_conflict_blocker() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // First import creates the Acquire (Order→Acquire via classify-raw, but here we import a Buy
    // to get a clean initial import, then re-import the same ID with different data).
    cmd::import::run(&vault, &pp(), &[coinbase_with_order(dir.path())]).unwrap();
    // Re-import the same source_ref with different amounts → ImportConflict.
    cmd::import::run(&vault, &pp(), &[coinbase_with_order_v2(dir.path())]).unwrap();

    // Verify the ImportConflict blocker exists.
    let conflict_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::ImportConflict(_)))
            .expect("ImportConflict must exist after re-import with changed content")
            .id
            .canonical()
    };

    cmd::reconcile::accept_conflict(&vault, &pp(), &conflict_ref, now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::ImportConflict),
        "ImportConflict blocker must be cleared after accept"
    );
}

#[test]
fn reject_conflict_clears_import_conflict_blocker() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_order(dir.path())]).unwrap();
    cmd::import::run(&vault, &pp(), &[coinbase_with_order_v2(dir.path())]).unwrap();

    let conflict_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .find(|e| matches!(e.payload, EventPayload::ImportConflict(_)))
            .expect("ImportConflict must exist after re-import with changed content")
            .id
            .canonical()
    };

    cmd::reconcile::reject_conflict(&vault, &pp(), &conflict_ref, now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::ImportConflict),
        "ImportConflict blocker must be cleared after reject"
    );
}
