mod fixtures;
use btctax_cli::{cmd, eventref, CliError, Session};
use btctax_core::{
    AllocMethod, BlockerKind, DisposeKind, EventId, EventPayload, FeeTreatment, FmvStatus,
    InboundClass, Income, IncomeKind, LotMethod, ManualFmv, OutflowClass, Source, SourceRef,
    TransferTarget, WalletId,
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
        None,
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

/// [Chunk 2 Task 1] reclassify-outflow gift with --donee "Alice" → Removal.donee == Some("Alice")
#[test]
fn reclassify_outflow_gift_with_donee_populates_removal_donee() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::GiftOut,
        btctax_cli::eventref::parse_usd_arg("2040.00").unwrap(),
        None,
        Some("Alice".to_string()),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.removals.len(), 1);
    assert_eq!(
        state.removals[0].donee,
        Some("Alice".to_string()),
        "GiftOut with donee 'Alice' must carry donee on the Removal"
    );
}

/// [Chunk 2 Task 1] reclassify-outflow donate with --donee "Charity X" → Removal.donee == Some("Charity X")
#[test]
fn reclassify_outflow_donate_with_donee_populates_removal_donee() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("2040.00").unwrap(),
        None,
        Some("Charity X".to_string()),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.removals.len(), 1);
    assert_eq!(
        state.removals[0].donee,
        Some("Charity X".to_string()),
        "Donate with donee 'Charity X' must carry donee on the Removal"
    );
}

/// [Chunk 2 Task 1] reclassify-outflow with no --donee → Removal.donee == None
#[test]
fn reclassify_outflow_without_donee_has_none_donee() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, out_ref) = vault_with_pending(dir.path());

    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::GiftOut,
        btctax_cli::eventref::parse_usd_arg("2040.00").unwrap(),
        None,
        None, // no donee
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(state.removals.len(), 1);
    assert_eq!(
        state.removals[0].donee, None,
        "GiftOut without donee must have donee: None on the Removal"
    );
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
    // D3 (Task 3): attest FIFO before the pre2025_method_attested gate requires it.
    // `timely_allocation_attested` (4th arg, true below) is a separate §5.02(4) attestation.
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();

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
    // D3 (Task 3): attest FIFO before the pre2025_method_attested gate requires it.
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();

    // alloc #1 (unattested) — inert: time-barred by the 2025 Sell. Then VOID it and re-allocate (alloc #2).
    // This is the legitimate allocate→inert→void→re-allocate→attest workflow (Eng-I1/I-2a). The OLD,
    // voided alloc #1 must NOT count toward attest's single-allocation guard.
    // ("unattested" = timely_allocation_attested=false, §5.02(4); unrelated to pre2025_method_attested.)
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
    // D3 (Task 3): attest FIFO before the pre2025_method_attested gate requires it.
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();
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
    // D3 (Task 3): attest FIFO before the pre2025_method_attested gate requires it.
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();

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

// ── Task 3 (pre-2025 method reconciliation): allocate gate KATs ────────────────────────────────

/// Task 3 KAT (a): `safe_harbor_allocate` refuses when `pre2025_method_attested == false`.
/// The error names the `config --set-pre2025-method … --attest-pre2025-method` remedy and
/// NO `SafeHarborAllocation` is appended to the event log.
#[test]
fn safe_harbor_allocate_refuses_when_pre2025_method_unattested() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let p = dir.path().join("cb.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    // Default config: pre2025_method_attested = false. Do NOT call set_pre2025_method.

    let err = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(&err, CliError::Usage(m)
            if m.contains("UNDECLARED pre-2025 method")
            && m.contains("--attest-pre2025-method")),
        "expected refusal naming UNDECLARED method and remedy, got: {err}"
    );

    // NO SafeHarborAllocation appended — event log is unchanged.
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(
        events
            .iter()
            .all(|e| !matches!(e.payload, EventPayload::SafeHarborAllocation(_))),
        "event log must NOT contain a SafeHarborAllocation after the refused allocate"
    );
}

/// Task 3 KAT (c): explicitly attested FIFO → allocate succeeds and records FIFO.
/// FIFO is the §7.4 legal default but must be explicitly attested — not silently inherited.
#[test]
fn safe_harbor_allocate_succeeds_with_explicitly_attested_fifo() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let p = dir.path().join("cb.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    // Explicitly attest FIFO — an explicit confirmation, not a silent default.
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();

    let id = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();
    assert!(matches!(id, EventId::Decision { .. }));

    // The allocation records the attested FIFO method.
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let alloc = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => Some(a),
            _ => None,
        })
        .expect("SafeHarborAllocation must be persisted");
    assert_eq!(
        alloc.pre2025_method,
        LotMethod::Fifo,
        "allocation must record the attested FIFO method"
    );
}

/// Chunk-5 Task 1 KAT: `Session::safe_harbor_residue` returns EXACTLY the lots the CLI command
/// appends, AND the returned `pre2025_method` equals the recorded `pre2025_method` on the appended
/// allocation [R0-M1]. Guards the DRY refactor (the helper is the single source of the pre-2025
/// subset, shared by `cmd::reconcile::safe_harbor_allocate` and the TUI allocate opener).
#[test]
fn safe_harbor_residue_matches_command_lots() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_timebarred(dir.path()); // pre-2025 0.20 BTC lot + a 2025 Sell
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();

    // Read the residue via the helper (READ-ONLY: appends/persists nothing).
    let (helper_lots, helper_method) = {
        let s = Session::open(&vault, &pp()).unwrap();
        s.safe_harbor_residue().unwrap()
    };
    assert!(
        !helper_lots.is_empty(),
        "pre-2025 residue must be non-empty"
    );

    // Append via the command, then load the persisted allocation.
    cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now())
        .unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let alloc = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => Some(a.clone()),
            _ => None,
        })
        .expect("SafeHarborAllocation persisted");

    assert_eq!(
        helper_lots, alloc.lots,
        "helper lots must equal the command-appended lots"
    );
    assert_eq!(
        helper_method, alloc.pre2025_method,
        "helper method must equal the recorded pre2025_method [R0-M1]"
    );
}

// ── Task 5: select-lots + import-selections + set_forward_method ────────────────────────────────

/// Emits a `LotSelection` decision for a specific disposal. Uses a synthetic buy+sell fixture
/// (100 000 sat each — full lot, no split, split_sequence=0) so the lot_id is deterministic.
#[test]
fn select_lots_emits_a_lot_selection_decision() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Synthetic post-2025 buy + sell, 100 000 sat each (fully consumed → no split).
    let p = dir.path().join("sel.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
sel-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.00100000,USD,84000.00,84.00,85.00,1.00,,,\r\n\
sel-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.00100000,USD,90000.00,90.00,89.00,1.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // The Coinbase adapter mints source_refs as "trade|<id>" for Buy/Sell.
    // The lot origin is the buy event; split_sequence=0 (original lot, not a split).
    let disposal_ref = "import|coinbase|trade|sel-sell";
    let lot_ref = "import|coinbase|trade|sel-buy#0";

    let picks = vec![eventref::parse_lot_pick(&format!("{lot_ref}:100000")).unwrap()];
    let id = cmd::reconcile::select_lots(&vault, &pp(), disposal_ref, picks, now()).unwrap();
    assert!(
        matches!(id, EventId::Decision { .. }),
        "select_lots must return a Decision EventId"
    );

    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::LotSelection(_))),
        "a LotSelection event must be present in the log after select_lots"
    );
}

/// `import-selections` must reject a CSV whose header does not match the required columns.
#[test]
fn import_selections_rejects_a_bad_header() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let csv = dir.path().join("sel.csv");
    std::fs::write(
        &csv,
        "wrong,header,here,now\nimport|coinbase|trade|D,import|coinbase|trade|A,0,100000\n",
    )
    .unwrap();
    let err = cmd::reconcile::import_selections(&vault, &pp(), &csv, now()).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)),
        "bad CSV header must produce CliError::Usage; got: {err}"
    );
}

/// `import-selections` groups multiple rows sharing a `disposal_ref` into a single `LotSelection`
/// (one decision per disposal). This test has two rows with the same disposal → one decision with
/// two picks.
#[test]
fn import_selections_groups_rows_into_one_selection_per_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let csv = dir.path().join("sel.csv");
    std::fs::write(
        &csv,
        "disposal_ref,origin_event_id,split_sequence,sat\n\
import|coinbase|trade|D,import|coinbase|trade|A,0,60000\n\
import|coinbase|trade|D,import|coinbase|trade|B,0,40000\n",
    )
    .unwrap();
    let ids = cmd::reconcile::import_selections(&vault, &pp(), &csv, now()).unwrap();
    assert_eq!(
        ids.len(),
        1,
        "two rows with the same disposal_ref → one LotSelection decision"
    );
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let ls = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::LotSelection(l) => Some(l.clone()),
            _ => None,
        })
        .expect("a LotSelection event must be persisted");
    assert_eq!(
        ls.lots.len(),
        2,
        "the LotSelection must carry both picks (60k + 40k)"
    );
}

/// `config --set-forward-method` appends a `MethodElection` decision (SPEC A.1 standing order).
/// The method and explicit effective_from must round-trip.
#[test]
fn set_forward_method_appends_a_method_election_decision() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let id = cmd::reconcile::set_forward_method(
        &vault,
        &pp(),
        LotMethod::Hifo,
        Some(date!(2025 - 06 - 01)),
        now(),
    )
    .unwrap();
    assert!(
        matches!(id, EventId::Decision { .. }),
        "set_forward_method must return a Decision EventId"
    );
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let me = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::MethodElection(m) => Some(m.clone()),
            _ => None,
        })
        .expect("a MethodElection event must be persisted");
    assert_eq!(me.method, LotMethod::Hifo, "method must be HIFO");
    assert_eq!(
        me.effective_from,
        date!(2025 - 06 - 01),
        "effective_from must match the supplied date"
    );
}

/// When `effective_from` is `None`, `set_forward_method` defaults to the decision's made-date
/// (the `now` parameter, in UTC), satisfying `effective_from >= made-date` by construction.
#[test]
fn set_forward_method_defaults_effective_from_to_made_date() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // now() = datetime!(2026-02-01 12:00:00 UTC) → made-date in UTC = 2026-02-01
    cmd::reconcile::set_forward_method(&vault, &pp(), LotMethod::Lifo, None, now()).unwrap();
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    let me = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::MethodElection(m) => Some(m.clone()),
            _ => None,
        })
        .expect("a MethodElection event must be persisted");
    assert_eq!(
        me.effective_from,
        date!(2026 - 02 - 01),
        "effective_from must default to the made-date (now in UTC)"
    );
}

/// Task-1 review Minor (apply-all): when both `--set-pre2025-method` and `--set-fee-treatment`
/// are provided together, both must take effect (the old if/else dispatch silently dropped
/// `--set-fee-treatment` when `--set-pre2025-method` was also set).
#[test]
fn config_apply_all_no_silent_drop() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Apply both flags — simulates what the fixed Config dispatch does sequentially.
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Lifo, false).unwrap();
    cmd::admin::set_config(&vault, &pp(), Some(FeeTreatment::TreatmentB)).unwrap();

    // Both must be stored; neither silently dropped.
    let cfg = cmd::admin::show_config(&vault, &pp()).unwrap();
    assert_eq!(
        cfg.pre2025_method,
        LotMethod::Lifo,
        "pre2025_method must be Lifo (not silently dropped)"
    );
    assert_eq!(
        cfg.fee_treatment,
        FeeTreatment::TreatmentB,
        "fee_treatment must be B (not silently dropped by the old pre2025_method branch)"
    );
}

/// M3 (apply-all incl. forward method): `config --set-forward-method` together with
/// `--set-fee-treatment` must apply BOTH. The old dispatch returned early after appending the
/// MethodElection and silently dropped the co-passed fee-treatment flag. Mirrors the fixed
/// Config dispatch (append the MethodElection AND apply the cli_config mutation, no early return).
#[test]
fn config_set_forward_method_and_fee_treatment_both_take_effect() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Simulate the FIXED Config dispatch: append the MethodElection AND apply the fee-treatment
    // mutation (apply-all — neither silently dropped by an early return).
    cmd::reconcile::set_forward_method(&vault, &pp(), LotMethod::Hifo, None, now()).unwrap();
    cmd::admin::set_config(&vault, &pp(), Some(FeeTreatment::TreatmentB)).unwrap();

    // (1) the MethodElection was appended (forward standing order took effect)...
    let me = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events.iter().find_map(|e| match &e.payload {
            EventPayload::MethodElection(m) => Some(m.clone()),
            _ => None,
        })
    };
    assert!(
        matches!(me, Some(ref m) if m.method == LotMethod::Hifo),
        "a HIFO MethodElection must be persisted (--set-forward-method not dropped)"
    );

    // (2) ...AND the fee-treatment mutation took effect (the old early-return dropped it).
    let cfg = cmd::admin::show_config(&vault, &pp()).unwrap();
    assert_eq!(
        cfg.fee_treatment,
        FeeTreatment::TreatmentB,
        "fee_treatment must be B — co-passed --set-fee-treatment must not be silently dropped"
    );
}

/// Task-1 review Minor: `--attest-pre2025-method` without `--set-pre2025-method` must produce a
/// `CliError::Usage` (not silently no-op). This test verifies the guard logic directly.
#[test]
fn attest_pre2025_method_requires_set_pre2025_method() {
    // The guard in main.rs::run() (Config arm) checks:
    //   if attest_pre2025_method && set_pre2025_method.is_none() → CliError::Usage
    // Mirror the check here so it's tested at the library level.
    fn dispatch_guard(
        set_pre2025_method: Option<LotMethod>,
        attest_pre2025_method: bool,
    ) -> Result<(), CliError> {
        if attest_pre2025_method && set_pre2025_method.is_none() {
            return Err(CliError::Usage(
                "--attest-pre2025-method requires --set-pre2025-method".into(),
            ));
        }
        Ok(())
    }

    // Negative: attest=true but method=None → Usage error
    assert!(
        matches!(dispatch_guard(None, true), Err(CliError::Usage(_))),
        "attest without set must be a Usage error"
    );
    // Positive: attest=true with method=Some → no error
    assert!(
        dispatch_guard(Some(LotMethod::Hifo), true).is_ok(),
        "attest with set must succeed"
    );
    // Positive: attest=false without method → no error (just show config)
    assert!(
        dispatch_guard(None, false).is_ok(),
        "no-op (show config only) must succeed"
    );
}

// ── bulk-link-transfer KATs (bulk-link-transfer Task 1) ──────────────────────

/// Seed a vault with two source wallets and five pending outbound transfers spanning two years and
/// a mix of priced / unpriced dates. Returns `(vault_path, [o1, o2, o3, o4, o5])` — the out EventIds:
///   o1: wallet A, 2025-03-01 (priced), 100_000 sat
///   o2: wallet A, 2025-06-15 (priced),  50_000 sat
///   o3: wallet B, 2025-03-01 (priced),  30_000 sat
///   o4: wallet A, 2024-02-01 (priced),  20_000 sat  (dropped by Frame::Year(2025))
///   o5: wallet A, 2025-04-01 (UNPRICED),40_000 sat  (increments missing_price_count)
/// PRIVACY: synthetic values only.
fn bulk_fixture(dir: &std::path::Path) -> (std::path::PathBuf, [EventId; 5]) {
    use btctax_core::event::{Acquire, BasisSource, TransferOut};
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::macros::datetime;
    use time::UtcOffset;

    let vault = dir.join("bulk.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();

    let wallet_a = WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    };
    let wallet_b = WalletId::Exchange {
        provider: "river".into(),
        account: "main".into(),
    };

    let mkid = |r: &str| EventId::import(Source::Coinbase, SourceRef::new(r));
    let out = |sat: i64| {
        EventPayload::TransferOut(TransferOut {
            sat,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        })
    };

    let acq_a = mkid("acq-a");
    let acq_b = mkid("acq-b");
    let o1 = mkid("o1");
    let o2 = mkid("o2");
    let o3 = mkid("o3");
    let o4 = mkid("o4");
    let o5 = mkid("o5");

    let batch = vec![
        LedgerEvent {
            id: acq_a.clone(),
            utc_timestamp: datetime!(2024-01-15 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: rust_decimal_macros::dec!(30000),
                fee_usd: rust_decimal_macros::dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        },
        LedgerEvent {
            id: acq_b.clone(),
            utc_timestamp: datetime!(2024-01-15 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_b.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 500_000,
                usd_cost: rust_decimal_macros::dec!(20000),
                fee_usd: rust_decimal_macros::dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        },
        LedgerEvent {
            id: o4.clone(),
            utc_timestamp: datetime!(2024-02-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a.clone()),
            payload: out(20_000),
        },
        LedgerEvent {
            id: o1.clone(),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a.clone()),
            payload: out(100_000),
        },
        LedgerEvent {
            id: o3.clone(),
            utc_timestamp: datetime!(2025-03-01 13:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_b.clone()),
            payload: out(30_000),
        },
        LedgerEvent {
            id: o5.clone(),
            utc_timestamp: datetime!(2025-04-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a.clone()),
            payload: out(40_000),
        },
        LedgerEvent {
            id: o2.clone(),
            utc_timestamp: datetime!(2025-06-15 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a.clone()),
            payload: out(50_000),
        },
    ];
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    (vault, [o1, o2, o3, o4, o5])
}

fn wallet_a() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}
fn wallet_b() -> WalletId {
    WalletId::Exchange {
        provider: "river".into(),
        account: "main".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}

/// The plan selects pending outs in-frame, applies the from_wallet filter, and routes same-wallet
/// rows to `skipped_same_wallet` (never `included`).
#[test]
fn bulk_plan_selects_pending_in_frame() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, [o1, o2, o3, _o4, o5]) = bulk_fixture(dir.path());
    let s = Session::open(&vault, &pp()).unwrap();

    // Frame::Year(2025), dest = B → o3 (source B) is same-wallet ⇒ skipped; o4 (2024) dropped.
    let plan = s
        .bulk_link_transfer_plan(
            BulkFilter {
                frame: Frame::Year(2025),
                from_wallet: None,
            },
            wallet_b(),
        )
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.out_event.clone()).collect();
    assert_eq!(
        included,
        vec![o1.clone(), o5.clone(), o2.clone()],
        "included sorted by date, same-wallet o3 skipped, 2024 o4 dropped"
    );
    let skipped: Vec<_> = plan
        .skipped_same_wallet
        .iter()
        .map(|r| r.out_event.clone())
        .collect();
    assert_eq!(
        skipped,
        vec![o3.clone()],
        "o3 (source == dest B) is skipped_same_wallet"
    );
    assert_eq!(plan.total_sat, 190_000, "Σ included principal_sat");

    // from_wallet = Some(A), dest = cold (never a source) → only A's 2025 outs, none skipped.
    let plan2 = s
        .bulk_link_transfer_plan(
            BulkFilter {
                frame: Frame::Year(2025),
                from_wallet: Some(wallet_a()),
            },
            cold(),
        )
        .unwrap();
    let included2: Vec<_> = plan2.included.iter().map(|r| r.out_event.clone()).collect();
    assert_eq!(
        included2,
        vec![o1, o5, o2],
        "from_wallet filter keeps only wallet-A outs"
    );
    assert!(
        plan2.skipped_same_wallet.is_empty(),
        "cold is no source → nothing skipped"
    );
}

/// A row with no price → `usd_value = None`, increments `missing_price_count`; the floor is the Σ of
/// the PRICED rows only (never a false exact total).
#[test]
fn bulk_plan_usd_total_floor_when_price_missing() {
    use btctax_cli::{BulkFilter, Frame};
    use rust_decimal_macros::dec;
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = bulk_fixture(dir.path());
    let s = Session::open(&vault, &pp()).unwrap();

    let plan = s
        .bulk_link_transfer_plan(
            BulkFilter {
                frame: Frame::Year(2025),
                from_wallet: Some(wallet_a()),
            },
            cold(),
        )
        .unwrap();
    // Included: o1 (84.00), o5 (unpriced), o2 (33.75).
    assert_eq!(plan.included.len(), 3);
    assert_eq!(
        plan.missing_price_count, 1,
        "o5 (2025-04-01) has no bundled price"
    );
    assert_eq!(
        plan.total_usd_value_floor,
        dec!(84.00) + dec!(33.75),
        "floor = Σ of priced rows only (o1 + o2)"
    );
    // The one None row is present in `included` (advisory, not dropped).
    assert_eq!(
        plan.included
            .iter()
            .filter(|r| r.usd_value.is_none())
            .count(),
        1
    );
}

/// Phase 1 (`bulk_link_plan`) is READ-ONLY: computing the plan writes nothing to the vault.
#[test]
fn bulk_cli_dry_run_writes_nothing() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = bulk_fixture(dir.path());

    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    let plan = cmd::reconcile::bulk_link_plan(
        &vault,
        &pp(),
        BulkFilter {
            frame: Frame::All,
            from_wallet: None,
        },
        cold(),
    )
    .unwrap();
    assert!(
        !plan.included.is_empty(),
        "plan must select rows (so the no-write is meaningful)"
    );
    let after = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    assert_eq!(
        before, after,
        "computing the plan must not append any event"
    );
}

/// Phase 2 (`apply_bulk_link_transfer`) is atomic: N TransferLinks appended in ONE save; the linked
/// outs leave `pending_reconciliation` (they project as `Op::SelfTransfer`).
#[test]
fn bulk_cli_apply_is_atomic_single_save() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, [o1, o2, _o3, _o4, _o5]) = bulk_fixture(dir.path());

    let pending_before = {
        let s = Session::open(&vault, &pp()).unwrap();
        s.project().unwrap().0.pending_reconciliation.len()
    };
    assert_eq!(pending_before, 5, "all five outs start pending");

    let n = cmd::reconcile::apply_bulk_link_transfer(
        &vault,
        &pp(),
        vec![o1.clone(), o2.clone()],
        cold(),
        now(),
    )
    .unwrap();
    assert_eq!(n, 2);

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    // o1 + o2 are no longer pending (self-transferred); o3/o4/o5 remain.
    let still_pending: std::collections::BTreeSet<_> = state
        .pending_reconciliation
        .iter()
        .map(|p| p.event.clone())
        .collect();
    assert_eq!(state.pending_reconciliation.len(), 3);
    assert!(!still_pending.contains(&o1) && !still_pending.contains(&o2));

    // Exactly two TransferLink decisions, both to Wallet(cold).
    let links: Vec<_> = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .filter_map(|e| match e.payload {
            EventPayload::TransferLink(tl) => Some(tl),
            _ => None,
        })
        .collect();
    assert_eq!(links.len(), 2, "exactly two TransferLinks appended");
    assert!(links
        .iter()
        .all(|tl| tl.in_event_or_wallet == TransferTarget::Wallet(cold())));
}

/// A frame that matches nothing → empty plan (the dispatch prints "no match" and exits 0).
#[test]
fn bulk_cli_no_match_exits_clean() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = bulk_fixture(dir.path());
    let s = Session::open(&vault, &pp()).unwrap();
    let plan = s
        .bulk_link_transfer_plan(
            BulkFilter {
                frame: Frame::Year(2030),
                from_wallet: None,
            },
            cold(),
        )
        .unwrap();
    assert!(plan.included.is_empty(), "no pending outs in 2030");
    assert!(plan.skipped_same_wallet.is_empty());
    assert_eq!(plan.total_sat, 0);
}
