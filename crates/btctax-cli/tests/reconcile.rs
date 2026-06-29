mod fixtures;
use btctax_cli::{cmd, CliError, Session};
use btctax_core::{
    AllocMethod, BlockerKind, DisposeKind, EventId, EventPayload, FmvStatus, InboundClass, Income,
    IncomeKind, ManualFmv, OutflowClass, Source, SourceRef, TransferTarget, WalletId,
};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use time::macros::{date, datetime};

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

/// Strengthened test for `set-fmv`. The original test targeted an Acquire event, but
/// ManualFmv is only applied by `build_op` in the `EventPayload::Income` arm (resolve.rs). This
/// test uses a SYNTHETIC Income event with `FmvStatus::Missing` and `usd_fmv: None`, appended
/// directly via `append_import_batch`. It verifies that:
///   1. The `FmvMissing` blocker is PRESENT before set-fmv.
///   2. After set-fmv, the blocker is CLEARED and income is recognized at the manual FMV.
#[test]
fn set_fmv_clears_fmv_missing_blocker_and_recognizes_income() {
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Append a synthetic River Income event with FmvStatus::Missing (no bundled price on
    // 2025-04-01 — the dataset only has a few fixed dates). The adapter path for River Income
    // with a missing price sets fmv_status=Missing and usd_fmv=None; here we create it directly.
    let income_id = EventId::import(Source::River, SourceRef::new("river-income-001"));
    let income_event = LedgerEvent {
        id: income_id.clone(),
        utc_timestamp: datetime!(2025-04-01 12:00:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(WalletId::Exchange {
            provider: "river".into(),
            account: "main".into(),
        }),
        payload: EventPayload::Income(Income {
            sat: 50_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Interest,
            business: false,
        }),
    };
    // Open a mutable session, append the synthetic event, and save.
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        append_import_batch(s.conn(), &[income_event]).unwrap();
        s.save().unwrap();
    }

    // BEFORE set-fmv: assert FmvMissing blocker is present.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::FmvMissing),
            "FmvMissing blocker must be present before set-fmv: {:?}",
            state.blockers
        );
        assert!(
            state.income_recognized.is_empty(),
            "income must NOT be recognized while FMV is missing"
        );
    }

    // Apply set-fmv targeting the Income event's id.
    let manual_fmv = btctax_cli::eventref::parse_usd_arg("4200.00").unwrap();
    let decision_id =
        cmd::reconcile::set_fmv(&vault, &pp(), &income_id.canonical(), manual_fmv, now()).unwrap();

    // Verify the ManualFmv decision was persisted.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        assert!(
            events
                .iter()
                .any(|e| e.id == decision_id && matches!(e.payload, EventPayload::ManualFmv(_))),
            "ManualFmv decision must be in the event log"
        );
    }

    // AFTER set-fmv: assert FmvMissing blocker is CLEARED and income recognized at manual FMV.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .blockers
                .iter()
                .all(|b| b.kind != BlockerKind::FmvMissing),
            "FmvMissing blocker must be CLEARED after set-fmv: {:?}",
            state.blockers
        );
        assert_eq!(
            state.income_recognized.len(),
            1,
            "income must be recognized at the manual FMV"
        );
        assert_eq!(
            state.income_recognized[0].usd_fmv, manual_fmv,
            "income FMV must equal the manual value supplied to set-fmv"
        );
        assert_eq!(
            state.income_recognized[0].kind,
            IncomeKind::Interest,
            "income kind must be preserved"
        );
    }
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
    // Build a real decision payload (ManualFmv) and serialize it to guarantee valid JSON that
    // parses correctly. This proves the is_imported guard rejects it, not a JSON parse error.
    let decision = EventPayload::ManualFmv(ManualFmv {
        event: EventId::decision(1),
        usd_fmv: dec!(100.00),
    });
    let bad_json = serde_json::to_string(&decision).unwrap();
    // Verify is_imported() returns false for this decision variant (the guard's condition).
    assert!(!decision.is_imported(), "ManualFmv must not be imported");
    // Call classify_raw with the decision payload and assert the error is the guard's message.
    let err = cmd::reconcile::classify_raw(&vault, &pp(), &target, &bad_json, now())
        .unwrap_err()
        .to_string();
    // Assert the guard's specific error message, not a JSON parse error (which would not contain "imported").
    assert!(
        err.contains("imported"),
        "expected is_imported guard error, got: {}",
        err
    );
}

#[test]
fn classify_raw_rejects_malformed_json() {
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
    // Malformed JSON should trigger the parse-error path, distinct from the is_imported guard.
    let malformed = "not json";
    let err = cmd::reconcile::classify_raw(&vault, &pp(), &target, malformed, now())
        .unwrap_err()
        .to_string();
    // Parse errors mention "bad --payload-json", not the guard's "imported" message.
    assert!(
        err.contains("bad --payload-json"),
        "expected parse error, got: {}",
        err
    );
}

#[test]
fn accept_conflict_clears_import_conflict_blocker() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // First import creates the Acquire (Order→Acquire via classify-raw, but here we import an Order
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

// ── Task 13: safe-harbor allocate + attest ──────────────────────────────────

#[test]
fn safe_harbor_allocate_seeds_full_pre2025_residue_even_after_a_2025_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // I-1 REGRESSION: a pre-2025 Buy (0.20 BTC) PLUS a 2025 Sell (0.05 BTC) that consumes part of that
    // 2024-vintage lot in FIFO. The post-2025-disposal `state.lots` would show only 0.15 BTC remaining,
    // but the engine's conservation guard compares the allocation to the *pre-2025-only* Universal residue
    // (the full 0.20 BTC at 2025-01-01). So the allocation MUST seed the full 0.20 BTC, not 0.15 — else it
    // trips the hard `SafeHarborUnconservable` blocker (the bug this fix closes).
    let p = dir.path().join("cb.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-sell,2025-06-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,90000.00,4500.00,4490.00,10.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    let id = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        true,
        now(),
    )
    .unwrap();
    assert!(matches!(id, btctax_core::EventId::Decision { .. }));

    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let alloc = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => Some(a.clone()),
            _ => None,
        })
        .expect("allocation persisted");
    assert_eq!(alloc.lots.len(), 1);
    // Seeds the FULL pre-2025 residue (0.20 BTC = 20_000_000 sat), NOT the 0.15 BTC post-Sell remainder.
    assert_eq!(alloc.lots[0].sat, 20_000_000);
    assert!(alloc.timely_allocation_attested);
    assert_eq!(alloc.as_of_date, btctax_core::conventions::TRANSITION_DATE);
    // Conservation is the engine's call; the seed equals the Universal residue → no hard safe-harbor blocker.
    let (state, _) = s.project().unwrap();
    assert!(state
        .blockers
        .iter()
        .all(|b| b.kind != btctax_core::BlockerKind::SafeHarborUnconservable));
}

/// Build a vault with a pre-2025 lot + a 2025 disposition (so an unattested allocation is TIME-BARRED:
/// its 2026 made-date is after the first-2025-disposition prong of the §5.02(4) ActualPosition bar).
fn vault_timebarred(dir: &std::path::Path) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let p = dir.join("cb.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-sell,2025-06-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,90000.00,4500.00,4490.00,10.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    vault
}

#[test]
fn safe_harbor_attest_cures_a_timebarred_allocation_excluding_voided_priors() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_timebarred(dir.path());

    // alloc #1 (unattested) — inert: time-barred by the 2025 Sell. Then VOID it and re-allocate (alloc #2).
    // This is the legitimate allocate→inert→void→re-allocate→attest workflow (Eng-I1/I-2a). The OLD,
    // voided alloc #1 must NOT count toward attest's single-allocation guard.
    let a1 = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();
    cmd::reconcile::void(&vault, &pp(), &a1.canonical(), now()).unwrap();
    let _a2 = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();

    // attest is NOT blocked by the voided alloc #1; it cures the time-bar on the single LIVE allocation.
    cmd::reconcile::safe_harbor_attest(&vault, &pp(), now())
        .unwrap_or_else(|e| panic!("attest should succeed: {e}"));

    // Path B is now effective: the boundary seed produced SafeHarborAllocated lots; no hard blocker.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(state.lots.iter().any(|l| matches!(
        l.basis_source,
        btctax_core::BasisSource::SafeHarborAllocated
    )));
    assert!(state
        .blockers
        .iter()
        .all(|b| b.kind != btctax_core::BlockerKind::SafeHarborUnconservable));
}

#[test]
fn safe_harbor_attest_refuses_an_already_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // A pre-2025 lot with NO 2025 disposition: an unattested allocation is ALREADY EFFECTIVE (made-date
    // precedes the only bar prong, the 2026-04-15 return-due date) → Path B with no attestation.
    let p = dir.path().join("cb_pre.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now())
        .unwrap();

    // I-2(b)/N-2: attest must REFUSE (and advise `verify`) rather than append a void-of-effective.
    let err = cmd::reconcile::safe_harbor_attest(&vault, &pp(), now()).unwrap_err();
    assert!(
        matches!(&err, CliError::Usage(m) if m.contains("already effective") && m.contains("verify"))
    );

    // The log was NOT mutated (no doomed Void appended): still exactly one allocation, zero voids.
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::SafeHarborAllocation(_)))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::VoidDecisionEvent(_)))
            .count(),
        0
    );
}

// ── Slug 1, Task B: safe_harbor_allocate must carry §1015(a) dual basis ────────────────────────

/// `safe_harbor_allocate` must carry the §1015(a) dual basis fields (`dual_loss_basis`,
/// `donor_acquired_at`) from the pre-2025 projection's residue lots into the emitted AllocLot.
/// Under the old code the CLI dropped these fields, collapsing the lot to single-basis.
///
/// Scenario: pre-2025 GiftReceived lot with donor (gain) basis = $100 and FMV-at-gift = $40
/// (FMV < donor → dual). Expected AllocLot: usd_basis=$100, dual_loss_basis=Some($40),
/// donor_acquired_at=Some(2021-01-01). [R0-I2: loss basis is FMV-at-gift, not donor basis.]
#[test]
fn safe_harbor_allocate_carries_gift_dual_basis() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Import a pre-2025 Coinbase Receive (2024-06-01, 100_000 sat = 0.00100000 BTC).
    let p = dir.path().join("cb_gift.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-gift-recv,2024-06-01 12:00:00 UTC,Receive,BTC,0.00100000,USD,40000.00,,,,,bc1qsender,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Find the TransferIn event id.
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

    // Classify as GiftReceived: donor (gain) basis = $100, FMV-at-gift (LOSS basis) = $40.
    // FMV-at-gift $40 < donor basis $100 → dual basis (§1015(a)); donor_acquired_at for tacking.
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::GiftReceived {
            donor_basis: Some(dec!(100.00)),
            donor_acquired_at: Some(date!(2021 - 01 - 01)),
            fmv_at_gift: dec!(40.00),
        },
        now(),
    )
    .unwrap();

    // Allocate via Path B. No 2025 disposition → made-date 2026-02-01 < return-due 2026-04-15
    // → timely without attestation → effective.
    cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now())
        .unwrap();

    // Load the persisted SafeHarborAllocation and assert the AllocLot carries the dual basis.
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let alloc = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => Some(a.clone()),
            _ => None,
        })
        .expect("SafeHarborAllocation must be persisted");

    assert_eq!(alloc.lots.len(), 1, "one lot in the allocation");
    let lot = &alloc.lots[0];

    // usd_basis = GAIN basis = donor carryover basis (§1015(a)).
    assert_eq!(
        lot.usd_basis,
        dec!(100.00),
        "usd_basis (gain basis) must be $100 (donor carryover); got {}",
        lot.usd_basis
    );
    // dual_loss_basis = LOSS basis = FMV-at-gift = $40. Must NOT be None. [R0-I2]
    assert_eq!(
        lot.dual_loss_basis,
        Some(dec!(40.00)),
        "dual_loss_basis must be Some($40) (FMV-at-gift LOSS basis); got {:?}",
        lot.dual_loss_basis
    );
    // donor_acquired_at carries through for §1223(2) tacking on the gain side.
    assert_eq!(
        lot.donor_acquired_at,
        Some(date!(2021 - 01 - 01)),
        "donor_acquired_at must be Some(2021-01-01) for §1223(2) tacking; got {:?}",
        lot.donor_acquired_at
    );
}
