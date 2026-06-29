mod fixtures;
use btctax_cli::{cmd, Session};
use btctax_core::{
    DisposeKind, EventPayload, InboundClass, IncomeKind, OutflowClass, TransferTarget,
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
