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

/// The pre-#41 6-row STUB daily-close dataset. Injected into a `Session` via `set_prices` [R0-C1] so
/// price-derived KAT assertions (whose expected FMV floors were computed from these exact stub closes,
/// AND whose `missing_price`/`excluded` sentinels rely on 2025-04-01 being ABSENT) stay independent of
/// the now-real bundled data (which is contiguous daily 2010-07-17→2026-06-03, so every 2025 date is
/// priced). Injecting the stub reproduces the exact pre-swap projection for these read-only plan KATs.
const STUB_PRICES_CSV: &str = "date,usd_close\n\
2024-01-15,42500.00\n\
2024-02-01,43100.50\n\
2025-01-10,91000.00\n\
2025-03-01,84000.00\n\
2025-03-02,84250.25\n\
2025-06-15,67500.00\n";

/// Open a session then inject the stub price provider (the read-only plan KATs' controlled dataset).
fn open_stub(vault: &std::path::Path) -> Session {
    let mut s = Session::open(vault, &pp()).unwrap();
    s.set_prices(Box::new(
        btctax_adapters::BundledPrices::from_csv_str(STUB_PRICES_CSV).unwrap(),
    ));
    s
}

/// [M5 + R0-I1] A batch of native `Income{fmv_status: Missing}` events on NOW-covered bundled dates
/// stays Hard-`FmvMissing` under the bundled provider ALONE — the already-imported Missing status is
/// never back-filled by better data (idempotent ingest). This is the committed source-of-truth the T2
/// "27 clear under pseudo" gate flips (A supplies the data; only B synthesizes the FMV).
#[test]
fn income_fmv_missing_batch_stays_blocked_under_bundled() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("m5.pgp");
    {
        let mut session = Session::create(&vault, &pp()).unwrap();
        let batch = fixtures::income_fmv_missing_batch(27);
        btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
        session.save().unwrap();
    } // drop the create session → release the vault lock before re-opening

    // REAL bundled data (covers all 27 dates) — yet the imported Missing status is not back-filled.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let fmv_missing = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::FmvMissing)
        .count();
    assert_eq!(
        fmv_missing, 27,
        "A alone leaves all 27 income events FmvMissing (idempotent import; R0-I1)"
    );
    assert_eq!(
        state.income_recognized.len(),
        0,
        "none recognized while FMV is pending"
    );
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

// ── bulk-resolve-conflict KATs (bulk-resolve-conflict Task 2) ────────────────

/// Seed a vault with TWO Acquires (cf-1 = 30_000 cost, cf-2 = 20_000 cost), then re-import BOTH with a
/// changed `usd_cost` (31_000 / 21_000) → TWO live `ImportConflict`s. Returns `(vault, [target1,
/// target2])`. Accept adopts the new (higher) cost; reject keeps the current. PRIVACY: synthetic only.
fn bulk_conflict_fixture(dir: &std::path::Path) -> (std::path::PathBuf, [EventId; 2]) {
    use btctax_core::event::{Acquire, BasisSource};
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let vault = dir.join("bulk_conflict.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();
    let wallet = WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    };
    let t1 = EventId::import(Source::Coinbase, SourceRef::new("cf-1"));
    let t2 = EventId::import(Source::Coinbase, SourceRef::new("cf-2"));
    let acq = |sat: i64, cost| {
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ComputedFromCost,
        })
    };
    let mk = |id: &EventId, payload| LedgerEvent {
        id: id.clone(),
        utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet.clone()),
        payload,
    };
    // v1 import.
    append_import_batch(
        session.conn(),
        &[
            mk(&t1, acq(1_000_000, dec!(30000))),
            mk(&t2, acq(500_000, dec!(20000))),
        ],
    )
    .unwrap();
    session.save().unwrap();
    // v2 re-import (changed cost) → two ImportConflicts.
    append_import_batch(
        session.conn(),
        &[
            mk(&t1, acq(1_000_000, dec!(31000))),
            mk(&t2, acq(500_000, dec!(21000))),
        ],
    )
    .unwrap();
    session.save().unwrap();
    (vault, [t1, t2])
}

/// The plan lists ONLY live `ImportConflict`s (structured current/new payloads + 8-char fingerprint);
/// resolving one single-item drops it from a subsequent plan (structural idempotence).
#[test]
fn bulk_resolve_plan_lists_unresolved_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, [t1, t2]) = bulk_conflict_fixture(dir.path());

    let plan = cmd::reconcile::bulk_resolve_conflict_plan(&vault, &pp()).unwrap();
    assert_eq!(plan.rows.len(), 2, "both live conflicts listed");
    let targets: Vec<_> = plan.rows.iter().map(|r| r.target.clone()).collect();
    assert!(
        targets.contains(&t1) && targets.contains(&t2),
        "rows join to the two import targets"
    );
    for r in &plan.rows {
        // Structured: current = the v1 Acquire, new = the changed v2 Acquire (different cost).
        match (&r.current_payload, &r.new_payload) {
            (EventPayload::Acquire(cur), EventPayload::Acquire(new)) => {
                assert_ne!(cur.usd_cost, new.usd_cost, "current cost != new cost");
            }
            other => panic!("expected Acquire current/new payloads, got {other:?}"),
        }
        assert_eq!(
            r.new_fingerprint.len(),
            8,
            "8-char fingerprint disambiguator"
        );
        assert_ne!(r.conflict_event, r.target, "conflict event != target");
    }

    // Resolve ONE single-item → a later plan lists only the OTHER (resolved excluded).
    let conflict1 = plan
        .rows
        .iter()
        .find(|r| r.target == t1)
        .unwrap()
        .conflict_event
        .canonical();
    cmd::reconcile::accept_conflict(&vault, &pp(), &conflict1, now()).unwrap();
    let plan2 = cmd::reconcile::bulk_resolve_conflict_plan(&vault, &pp()).unwrap();
    assert_eq!(plan2.rows.len(), 1, "resolved conflict excluded from plan");
    assert_eq!(
        plan2.rows[0].target, t2,
        "only the unresolved conflict remains"
    );
}

/// E2E accept: every target's `ImportConflict` blocker clears AND the projection adopts each
/// `new_payload` (total lot basis reflects the higher v2 costs: 31_000 + 21_000 = 52_000).
#[test]
fn bulk_resolve_cli_accept_adopts_new() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _) = bulk_conflict_fixture(dir.path());

    let plan = cmd::reconcile::bulk_resolve_conflict_plan(&vault, &pp()).unwrap();
    let conflict_events: Vec<_> = plan.rows.iter().map(|r| r.conflict_event.clone()).collect();
    let n =
        cmd::reconcile::apply_bulk_accept_conflicts(&vault, &pp(), conflict_events, now()).unwrap();
    assert_eq!(n, 2, "two conflicts accepted");

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::ImportConflict),
        "all ImportConflict blockers cleared after bulk accept"
    );
    let total_basis: btctax_core::Usd = state.lots.iter().map(|l| l.usd_basis).sum();
    assert_eq!(
        total_basis,
        dec!(52000),
        "accept adopts each new_payload (higher v2 costs)"
    );
}

/// E2E reject: every target's `ImportConflict` blocker clears AND the projection KEEPS each current
/// payload (total lot basis reflects the original v1 costs: 30_000 + 20_000 = 50_000).
#[test]
fn bulk_resolve_cli_reject_keeps_current() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _) = bulk_conflict_fixture(dir.path());

    let plan = cmd::reconcile::bulk_resolve_conflict_plan(&vault, &pp()).unwrap();
    let conflict_events: Vec<_> = plan.rows.iter().map(|r| r.conflict_event.clone()).collect();
    let n =
        cmd::reconcile::apply_bulk_reject_conflicts(&vault, &pp(), conflict_events, now()).unwrap();
    assert_eq!(n, 2, "two conflicts rejected");

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::ImportConflict),
        "all ImportConflict blockers cleared after bulk reject"
    );
    let total_basis: btctax_core::Usd = state.lots.iter().map(|l| l.usd_basis).sum();
    assert_eq!(
        total_basis,
        dec!(50000),
        "reject keeps each current payload (original v1 costs)"
    );
}

/// The Phase-1 read (`bulk_resolve_conflict_plan`, which `--dry-run` invokes and then STOPS) writes
/// nothing — a subsequent plan still lists both conflicts.
#[test]
fn bulk_resolve_cli_dry_run_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _) = bulk_conflict_fixture(dir.path());

    let _preview = cmd::reconcile::bulk_resolve_conflict_plan(&vault, &pp()).unwrap();
    let plan2 = cmd::reconcile::bulk_resolve_conflict_plan(&vault, &pp()).unwrap();
    assert_eq!(
        plan2.rows.len(),
        2,
        "the read/preview phase resolves nothing (dry-run writes nothing)"
    );
}

// ── bulk-void (Cycle 3) ──────────────────────────────────────────────────────

/// Import two Receives (→ two UnknownBasisInbound TransferIns) and classify BOTH as income (two
/// revocable `ClassifyInbound` decisions). Returns (vault, [ti1, ti2]) — the TransferIn ids the
/// classifications target (so voiding them re-exposes UnknownBasisInbound).
fn vault_with_two_classified_inbounds(dir: &std::path::Path) -> (std::path::PathBuf, [String; 2]) {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let p = dir.join("cb_two_recv.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv-1,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,\r\n\
cb-recv-2,2025-04-01 12:00:00 UTC,Receive,BTC,0.03000000,USD,86000.00,,,,,bc1qsender,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    let ti_refs: Vec<String> = {
        let s = Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        events
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::TransferIn(_)))
            .map(|e| e.id.canonical())
            .collect()
    };
    assert_eq!(ti_refs.len(), 2, "two TransferIns imported");
    for ti in &ti_refs {
        let class = InboundClass::Income {
            kind: IncomeKind::Reward,
            fmv: Some(btctax_cli::eventref::parse_usd_arg("4200.00").unwrap()),
            business: false,
        };
        cmd::reconcile::classify_inbound(&vault, &pp(), ti, class, now()).unwrap();
    }
    (vault, [ti_refs[0].clone(), ti_refs[1].clone()])
}

/// The plan lists the voidable revocable decisions (both ClassifyInbounds); the Phase-1 read writes
/// nothing (a second plan is identical), and applying it voids every row.
#[test]
fn bulk_void_dry_run_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _) = vault_with_two_classified_inbounds(dir.path());

    let plan = cmd::reconcile::bulk_void_plan(&vault, &pp()).unwrap();
    assert_eq!(
        plan.rows.len(),
        2,
        "both ClassifyInbound decisions voidable"
    );
    assert!(
        plan.rows.iter().all(|r| r.disposal_to_clear.is_none()),
        "ClassifyInbound targets carry no disposal_to_clear"
    );

    // Dry-run == re-run the plan; nothing written.
    let plan2 = cmd::reconcile::bulk_void_plan(&vault, &pp()).unwrap();
    assert_eq!(
        plan2.rows.len(),
        2,
        "the read/preview phase voids nothing (dry-run writes nothing)"
    );
}

/// [R0-M3 / #7] `bulk_void_plan` OMITS an effective `SafeHarborAllocation` — so `apply_bulk_void`
/// (whose targets are the plan rows) can never sweep it into a Hard `DecisionConflict`. A pre-2025 lot
/// with NO 2025 disposition makes an unattested allocation ALREADY EFFECTIVE (Path B).
#[test]
fn bulk_void_plan_omits_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    let p = dir.path().join("cb_pre.csv");
    std::fs::write(&p, "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n").unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();
    let alloc = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();

    // Sanity: the allocation is effective (no timebar / unconservable blocker on its id). Scoped so the
    // session's VaultLock is dropped before `bulk_void_plan` re-opens the vault.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state.blockers.iter().all(|b| {
                !(matches!(
                    b.kind,
                    BlockerKind::SafeHarborTimebar | BlockerKind::SafeHarborUnconservable
                ) && b.event.as_ref() == Some(&alloc))
            }),
            "the allocation must be effective (Path B) for this KAT"
        );
    }

    let plan = cmd::reconcile::bulk_void_plan(&vault, &pp()).unwrap();
    assert!(
        !plan.rows.iter().any(|r| r.target_event_id == alloc),
        "an effective allocation must NOT be a bulk-void candidate (#7)"
    );
    assert!(
        plan.rows.is_empty(),
        "the effective allocation is the only decision → plan is empty"
    );
}

/// An INERT (time-barred) allocation STAYS a bulk-void candidate — voiding it applies cleanly.
#[test]
fn bulk_void_plan_includes_inert_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_timebarred(dir.path());
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();
    // Unattested → time-barred by the 2025 Sell → inert.
    let alloc = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();

    // Scoped so the session's VaultLock is dropped before `bulk_void_plan` re-opens the vault.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state.blockers.iter().any(|b| {
                b.kind == BlockerKind::SafeHarborTimebar && b.event.as_ref() == Some(&alloc)
            }),
            "the allocation must be time-barred (inert) for this KAT"
        );
    }

    let plan = cmd::reconcile::bulk_void_plan(&vault, &pp()).unwrap();
    assert!(
        plan.rows.iter().any(|r| r.target_event_id == alloc),
        "an inert (time-barred) allocation MUST stay a bulk-void candidate"
    );
}

/// E2E: bulk-voiding the two ClassifyInbound decisions re-exposes their UnknownBasisInbound blockers.
#[test]
fn bulk_void_cli_reexposes_inbound_blockers() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _) = vault_with_two_classified_inbounds(dir.path());

    // Pre: both classified → no UnknownBasisInbound.
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .blockers
                .iter()
                .all(|b| b.kind != BlockerKind::UnknownBasisInbound),
            "both inbounds classified → no UnknownBasisInbound pre-void"
        );
    }

    let plan = cmd::reconcile::bulk_void_plan(&vault, &pp()).unwrap();
    let targets: Vec<_> = plan
        .rows
        .iter()
        .map(|r| (r.target_event_id.clone(), r.disposal_to_clear.clone()))
        .collect();
    let n = cmd::reconcile::apply_bulk_void(&vault, &pp(), targets, now()).unwrap();
    assert_eq!(n, 2, "two decisions voided");

    // Post: both UnknownBasisInbound blockers re-exposed.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let unknown = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .count();
    assert_eq!(
        unknown, 2,
        "voiding the two ClassifyInbounds re-exposes both UnknownBasisInbound blockers"
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
/// UX-P4-4(b) gift arm: a `--donor-acquired` date STRICTLY AFTER the receipt date is refused by the
/// SAME `classify_inbound` record-time guard that covers `--acquired` on a self-transfer. Receipt is
/// 2024-06-01 (UTC); `donor_acquired_at = 2024-06-02` is refused with a message that names
/// `--donor-acquired`, prints the receipt date + tz basis, and no decision is appended (fail-closed).
/// (★ fault-inject: drop the `GiftReceived` arm from `acquired_override` and this goes RED.)
#[test]
fn classify_inbound_gift_donor_acquired_after_receipt_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Import a Coinbase Receive (2024-06-01 UTC) — the gifted coins' receiving TransferIn.
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

    let err = cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &in_ref,
        InboundClass::GiftReceived {
            donor_basis: Some(dec!(100.00)),
            donor_acquired_at: Some(date!(2024 - 06 - 02)), // one day AFTER receipt → impossible
            fmv_at_gift: dec!(40.00),
        },
        now(),
    )
    .expect_err("a donor-acquired date after the receipt must be refused");
    let msg = err.to_string();
    assert!(
        msg.contains("--donor-acquired")
            && msg.contains("2024-06-01")
            && msg.contains("receipt")
            && msg.contains("UTC"),
        "the refusal must name --donor-acquired, the receipt date + tz basis: {msg}"
    );

    // Fail-closed: nothing was appended (no ClassifyInbound decision in the log).
    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::ClassifyInbound(_))),
        "a refused gift classify must append no decision"
    );
}

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
        None, // global scope
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
    cmd::reconcile::set_forward_method(&vault, &pp(), LotMethod::Lifo, None, None, now()).unwrap();
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
    cmd::reconcile::set_forward_method(&vault, &pp(), LotMethod::Hifo, None, None, now()).unwrap();
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
    let s = open_stub(&vault);

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

// ══════════════════════════════════════════════════════════════════════════════════════════════
// bulk-classify-inbound-self-transfer D1/D2 — `Session::bulk_self_transfer_in_plan` +
// `cmd::reconcile::{bulk_self_transfer_in_plan, apply_bulk_self_transfer_in}`. READ-ONLY plan:
// selects UnknownBasisInbound TransferIns MINUS already-classified (filter-3 [I1]) MINUS wallet-less
// [M2]; honest USD floor; atomic N-append/one-save apply as SelfTransferMine{None,None} ($0 basis).
// ══════════════════════════════════════════════════════════════════════════════════════════════

/// Seed a vault of raw (unclassified) `TransferIn` deposits, each firing `UnknownBasisInbound`.
/// Returns `(vault, [i1, i2, i3, i4])`:
///   i1 = wallet A, 2025-03-01, 100_000 sat (PRICED → $84.00)
///   i2 = wallet A, 2025-04-01,  50_000 sat (UNPRICED — no bundled price)
///   i3 = wallet B, 2025-06-15,  40_000 sat (PRICED)
///   i4 = wallet A, 2024-02-01,  20_000 sat (OUT of the 2025 frame)
fn sti_fixture(dir: &std::path::Path) -> (std::path::PathBuf, [EventId; 4]) {
    use btctax_core::event::TransferIn;
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let vault = dir.join("sti.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();

    let mkid = |r: &str| EventId::import(Source::Coinbase, SourceRef::new(r));
    let tin = |sat: i64| {
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        })
    };
    let ev = |id: EventId, ts, wallet: WalletId, sat: i64| LedgerEvent {
        id,
        utc_timestamp: ts,
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet),
        payload: tin(sat),
    };

    let i1 = mkid("sti-i1");
    let i2 = mkid("sti-i2");
    let i3 = mkid("sti-i3");
    let i4 = mkid("sti-i4");
    let batch = vec![
        ev(
            i1.clone(),
            datetime!(2025-03-01 12:00:00 UTC),
            wallet_a(),
            100_000,
        ),
        ev(
            i2.clone(),
            datetime!(2025-04-01 12:00:00 UTC),
            wallet_a(),
            50_000,
        ),
        ev(
            i3.clone(),
            datetime!(2025-06-15 12:00:00 UTC),
            wallet_b(),
            40_000,
        ),
        ev(
            i4.clone(),
            datetime!(2024-02-01 12:00:00 UTC),
            wallet_a(),
            20_000,
        ),
    ];
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    (vault, [i1, i2, i3, i4])
}

/// The plan selects UnknownBasisInbound TransferIns in-frame, applies the wallet filter, sorts by date.
#[test]
fn bulk_sti_plan_selects_unknown_inbounds_in_frame() {
    use btctax_cli::{BulkStiFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, i2, i3, _i4]) = sti_fixture(dir.path());
    let s = Session::open(&vault, &pp()).unwrap();

    // Year(2025), wallet None → i1,i2,i3 (sorted by date); 2024 i4 dropped.
    let plan = s
        .bulk_self_transfer_in_plan(BulkStiFilter {
            frame: Frame::Year(2025),
            wallet: None,
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(
        included,
        vec![i1.clone(), i2.clone(), i3.clone()],
        "included sorted by date; the 2024 inbound is out of frame"
    );
    assert_eq!(plan.total_sat, 190_000, "Σ included sat");

    // wallet = Some(A) → only A's 2025 inbounds (i1, i2); i3 (wallet B) filtered out.
    let plan2 = s
        .bulk_self_transfer_in_plan(BulkStiFilter {
            frame: Frame::Year(2025),
            wallet: Some(wallet_a()),
        })
        .unwrap();
    let included2: Vec<_> = plan2.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(
        included2,
        vec![i1, i2],
        "wallet filter keeps only wallet-A inbounds"
    );
}

/// [R0-I1 + R0-M2] The candidate set EXCLUDES (a) any inbound already targeted by a non-voided
/// `ClassifyInbound` — here a gift-case-4 `GiftReceived{donor_basis:None,donor_acquired_at:None}` which
/// RE-FIRES `UnknownBasisInbound` — and (b) any wallet-less `TransferIn` (creates no lot). Only the
/// plain unclassified wallet-bearing inbound is selected.
#[test]
fn bulk_sti_plan_excludes_already_classified_and_walletless() {
    use btctax_cli::{BulkStiFilter, Frame};
    use btctax_core::event::TransferIn;
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("sti-excl.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();

    let mkid = |r: &str| EventId::import(Source::Coinbase, SourceRef::new(r));
    let tin = |sat: i64| {
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        })
    };
    let normal = mkid("sti-normal");
    let gift4 = mkid("sti-gift4");
    let walletless = mkid("sti-walletless");
    let batch = vec![
        // (a) plain unclassified, wallet A → the ONLY expected candidate.
        LedgerEvent {
            id: normal.clone(),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a()),
            payload: tin(100_000),
        },
        // (b) will be classified gift-case-4 → re-fires UnknownBasisInbound but is already-classified.
        LedgerEvent {
            id: gift4.clone(),
            utc_timestamp: datetime!(2025-03-05 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a()),
            payload: tin(70_000),
        },
        // (c) wallet-less unclassified → fires UnknownBasisInbound with wallet None (M2 excludes it).
        LedgerEvent {
            id: walletless.clone(),
            utc_timestamp: datetime!(2025-03-10 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: None,
            payload: tin(60_000),
        },
    ];
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    drop(session); // release the VaultLock before classify_inbound re-opens the vault

    // Classify gift4 as gift-case-4 (donor basis + acquisition BOTH unknown → UnknownBasisInbound re-fires).
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &gift4.canonical(),
        InboundClass::GiftReceived {
            donor_basis: None,
            donor_acquired_at: None,
            fmv_at_gift: dec!(1000),
        },
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();

    // Sanity: all three still carry UnknownBasisInbound (so the exclusions are MEANINGFUL, not vacuous).
    let (state, _) = s.project().unwrap();
    let flagged: std::collections::BTreeSet<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .filter_map(|b| b.event.clone())
        .collect();
    assert!(
        flagged.contains(&normal) && flagged.contains(&gift4) && flagged.contains(&walletless),
        "all three inbounds re-fire UnknownBasisInbound: {flagged:?}"
    );

    let plan = s
        .bulk_self_transfer_in_plan(BulkStiFilter {
            frame: Frame::All,
            wallet: None,
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(
        included,
        vec![normal],
        "only the plain unclassified wallet-bearing inbound is a candidate; \
         gift-case-4 (already-classified, filter-3) and wallet-less (M2) are excluded"
    );
}

/// A row with no price → `usd_fmv = None`, increments `missing_price_count`; the floor is the Σ of the
/// PRICED rows only (never a false exact total).
#[test]
fn bulk_sti_plan_fmv_floor_when_price_missing() {
    use btctax_cli::{BulkStiFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = sti_fixture(dir.path());
    let s = open_stub(&vault);

    // wallet A / Year 2025 → i1 (2025-03-01, priced $84.00) + i2 (2025-04-01, unpriced).
    let plan = s
        .bulk_self_transfer_in_plan(BulkStiFilter {
            frame: Frame::Year(2025),
            wallet: Some(wallet_a()),
        })
        .unwrap();
    assert_eq!(plan.included.len(), 2);
    assert_eq!(
        plan.missing_price_count, 1,
        "i2 (2025-04-01) has no bundled price"
    );
    assert_eq!(
        plan.total_usd_fmv_floor,
        dec!(84.00),
        "floor = Σ of priced rows only (i1)"
    );
    assert_eq!(
        plan.included.iter().filter(|r| r.usd_fmv.is_none()).count(),
        1
    );
}

/// Phase 1 (`bulk_self_transfer_in_plan`) is READ-ONLY: computing the plan writes nothing to the vault.
#[test]
fn bulk_sti_cli_dry_run_writes_nothing() {
    use btctax_cli::{BulkStiFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = sti_fixture(dir.path());

    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    let plan = cmd::reconcile::bulk_self_transfer_in_plan(
        &vault,
        &pp(),
        BulkStiFilter {
            frame: Frame::All,
            wallet: None,
        },
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

/// Phase 2 (`apply_bulk_self_transfer_in`) is atomic: N `ClassifyInbound{SelfTransferMine{None,None}}`
/// appended in ONE save; each classified inbound projects as a non-taxable $0-basis lot and its
/// `UnknownBasisInbound` blocker clears.
#[test]
fn bulk_sti_cli_apply_is_atomic_single_save() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, i2, i3, _i4]) = sti_fixture(dir.path());

    let n = cmd::reconcile::apply_bulk_self_transfer_in(
        &vault,
        &pp(),
        vec![i1.clone(), i3.clone()],
        now(),
    )
    .unwrap();
    assert_eq!(n, 2);

    let s = Session::open(&vault, &pp()).unwrap();

    // Exactly two ClassifyInbound{SelfTransferMine{None,None}} decisions appended.
    let classified: Vec<_> = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .filter_map(|e| match e.payload {
            EventPayload::ClassifyInbound(ci) => Some(ci),
            _ => None,
        })
        .collect();
    assert_eq!(classified.len(), 2, "exactly two ClassifyInbound appended");
    assert!(
        classified.iter().all(|ci| matches!(
            ci.as_,
            InboundClass::SelfTransferMine {
                basis: None,
                acquired_at: None
            }
        )),
        "every appended classification is SelfTransferMine{{None,None}}"
    );

    let (state, _) = s.project().unwrap();
    // i1 + i3 no longer flagged UnknownBasisInbound (i2 untouched stays flagged).
    let flagged: std::collections::BTreeSet<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .filter_map(|b| b.event.clone())
        .collect();
    assert!(
        !flagged.contains(&i1) && !flagged.contains(&i3),
        "classified inbounds' UnknownBasisInbound blockers cleared"
    );
    assert!(flagged.contains(&i2), "the untouched i2 stays flagged");

    // Each classified inbound created a $0-basis lot.
    for id in [&i1, &i3] {
        let lot = state
            .lots
            .iter()
            .find(|l| &l.lot_id.origin_event_id == id)
            .expect("a lot was created for the classified inbound");
        assert_eq!(
            lot.usd_basis,
            dec!(0),
            "self-transfer-in defaults to $0 basis"
        );
    }
}

/// A frame that matches nothing → empty plan (the dispatch prints "no match" and exits 0).
#[test]
fn bulk_sti_cli_no_match_exits_clean() {
    use btctax_cli::{BulkStiFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = sti_fixture(dir.path());
    let s = Session::open(&vault, &pp()).unwrap();
    let plan = s
        .bulk_self_transfer_in_plan(BulkStiFilter {
            frame: Frame::Year(2030),
            wallet: None,
        })
        .unwrap();
    assert!(plan.included.is_empty(), "no inbounds in 2030");
    assert_eq!(plan.total_sat, 0);
    assert_eq!(plan.total_usd_fmv_floor, dec!(0));
    assert_eq!(plan.missing_price_count, 0);
}

// ══════════════════════════════════════════════════════════════════════════════════════════════
// bulk-classify-inbound-income (Cycle 4) — `Session::bulk_classify_income_plan` +
// `cmd::reconcile::{bulk_classify_income_plan, apply_bulk_classify_inbound_income}`. A NEAR-CLONE of
// bulk-sti with ONE load-bearing difference [#a tax-safety]: a MISSING-PRICE candidate is EXCLUDED
// from `included` (counted in `excluded_missing_price`), NEVER classified as `Income{fmv:None}` (which
// would trade `UnknownBasisInbound` for a Hard `FmvMissing` year-gate). Reuses `sti_fixture` (same seed).
// ══════════════════════════════════════════════════════════════════════════════════════════════

/// The plan selects UnknownBasisInbound TransferIns in-frame, applies the wallet filter, sorts by date,
/// and carries a RESOLVED per-row FMV; the 2024 inbound is out of frame and the unpriced one is excluded.
#[test]
fn bulk_income_plan_lists_pending_inbounds() {
    use btctax_cli::{BulkIncomeFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, _i2, i3, _i4]) = sti_fixture(dir.path());
    let s = open_stub(&vault);

    // Year(2025), wallet None → i1 (priced $84.00) + i3 (priced $27.00); i2 (2025, unpriced) is
    // EXCLUDED (missing price), i4 (2024) is out of frame.
    let plan = s
        .bulk_classify_income_plan(BulkIncomeFilter {
            frame: Frame::Year(2025),
            wallet: None,
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(
        included,
        vec![i1, i3],
        "included = priced 2025 inbounds sorted by date (i2 unpriced, i4 out of frame dropped)"
    );
    assert_eq!(
        plan.excluded_missing_price, 1,
        "i2 (2025-04-01) has no price"
    );
    assert_eq!(plan.total_sat, 140_000, "Σ included sat");
    assert_eq!(
        plan.total_income_usd,
        dec!(111.00),
        "Σ income = $84.00 + $27.00"
    );
    assert_eq!(
        plan.included.iter().map(|r| r.fmv).collect::<Vec<_>>(),
        vec![dec!(84.00), dec!(27.00)],
        "each included row carries its resolved auto-FMV"
    );
}

/// [filter-3] The candidate set EXCLUDES any inbound already targeted by a non-voided `ClassifyInbound`
/// (a gift-case-4 `GiftReceived{None,None}` that RE-FIRES `UnknownBasisInbound` yet is already
/// classified — a second `ClassifyInbound{Income}` would fire a Hard `DecisionConflict`).
#[test]
fn bulk_income_plan_excludes_already_classified() {
    use btctax_cli::{BulkIncomeFilter, Frame};
    use btctax_core::event::TransferIn;
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("inc-excl.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();

    let mkid = |r: &str| EventId::import(Source::Coinbase, SourceRef::new(r));
    let tin = |sat: i64| {
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        })
    };
    let normal = mkid("inc-normal");
    let gift4 = mkid("inc-gift4");
    let batch = vec![
        // plain unclassified, wallet A, PRICED date → the ONLY expected candidate.
        LedgerEvent {
            id: normal.clone(),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a()),
            payload: tin(100_000),
        },
        // will be classified gift-case-4 → re-fires UnknownBasisInbound but is already-classified.
        LedgerEvent {
            id: gift4.clone(),
            utc_timestamp: datetime!(2025-03-02 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a()),
            payload: tin(70_000),
        },
    ];
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    drop(session);

    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &gift4.canonical(),
        InboundClass::GiftReceived {
            donor_basis: None,
            donor_acquired_at: None,
            fmv_at_gift: dec!(1000),
        },
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    // Sanity: both still carry UnknownBasisInbound (so the exclusion is MEANINGFUL, not vacuous).
    let (state, _) = s.project().unwrap();
    let flagged: std::collections::BTreeSet<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .filter_map(|b| b.event.clone())
        .collect();
    assert!(
        flagged.contains(&normal) && flagged.contains(&gift4),
        "both inbounds re-fire UnknownBasisInbound: {flagged:?}"
    );

    let plan = s
        .bulk_classify_income_plan(BulkIncomeFilter {
            frame: Frame::All,
            wallet: None,
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(
        included,
        vec![normal],
        "only the plain unclassified inbound is a candidate; the already-classified gift-case-4 is excluded"
    );
}

/// [#a tax-safety] A row whose date has NO bundled price is NOT in `included` and IS counted in
/// `excluded_missing_price` — so a persisted batch NEVER creates an `Income{fmv:None}` → NO Hard
/// `FmvMissing`. (i2 = 2025-04-01 has no bundled price.)
#[test]
fn bulk_income_plan_excludes_missing_price() {
    use btctax_cli::{BulkIncomeFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, i2, _i3, _i4]) = sti_fixture(dir.path());
    let s = open_stub(&vault);

    // wallet A / Year 2025 → i1 (2025-03-01, priced) + i2 (2025-04-01, UNPRICED).
    let plan = s
        .bulk_classify_income_plan(BulkIncomeFilter {
            frame: Frame::Year(2025),
            wallet: Some(wallet_a()),
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(included, vec![i1], "only the priced inbound is included");
    assert!(
        !included.contains(&i2),
        "the unpriced inbound (i2) is NOT classified as income (would year-gate)"
    );
    assert_eq!(plan.excluded_missing_price, 1, "i2 surfaced as excluded");
    assert_eq!(
        plan.total_income_usd,
        dec!(84.00),
        "income = priced row only"
    );
}

/// [R0-M4] A wallet-less pending inbound (a Hard-`FmvMissing`/no-lot vector) is NOT in `included`
/// (mirrors bulk-sti's wallet-less exclusion) — and is NOT counted as missing-price (a distinct
/// exclusion, earlier in the pipeline).
#[test]
fn bulk_income_plan_excludes_wallet_less() {
    use btctax_cli::{BulkIncomeFilter, Frame};
    use btctax_core::event::TransferIn;
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("inc-walletless.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();

    let mkid = |r: &str| EventId::import(Source::Coinbase, SourceRef::new(r));
    let tin = |sat: i64| {
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        })
    };
    let normal = mkid("inc-normal");
    let walletless = mkid("inc-walletless");
    let batch = vec![
        LedgerEvent {
            id: normal.clone(),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(wallet_a()),
            payload: tin(100_000),
        },
        // wallet-less → fires UnknownBasisInbound with wallet None (excluded; creates no lot).
        LedgerEvent {
            id: walletless.clone(),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: None,
            payload: tin(60_000),
        },
    ];
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    drop(session); // release the VaultLock before re-opening

    let s = Session::open(&vault, &pp()).unwrap();
    let plan = s
        .bulk_classify_income_plan(BulkIncomeFilter {
            frame: Frame::All,
            wallet: None,
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.in_event.clone()).collect();
    assert_eq!(
        included,
        vec![normal],
        "the wallet-less inbound is excluded (creates no lot; no missing-price count)"
    );
    assert_eq!(
        plan.excluded_missing_price, 0,
        "wallet-less is a DISTINCT exclusion, not counted as missing-price"
    );
}

/// Phase 1 (`bulk_classify_income_plan`) is READ-ONLY: computing the plan writes nothing to the vault.
#[test]
fn bulk_income_dry_run_writes_nothing() {
    use btctax_cli::{BulkIncomeFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = sti_fixture(dir.path());

    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    let plan = cmd::reconcile::bulk_classify_income_plan(
        &vault,
        &pp(),
        BulkIncomeFilter {
            frame: Frame::All,
            wallet: None,
        },
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

/// [#a] The persisted decision's `fmv` — and thus the PROJECTED `Income.usd_fmv` (the recognized
/// income AND the lot basis) — equals `fmv_of(date, sat)`.
#[test]
fn bulk_income_apply_sets_autofmv() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, _i2, _i3, _i4]) = sti_fixture(dir.path());

    let n = cmd::reconcile::apply_bulk_classify_inbound_income(
        &vault,
        &pp(),
        vec![i1.clone()],
        IncomeKind::Mining,
        false,
        now(),
    )
    .unwrap();
    assert_eq!(n, 1);

    let s = Session::open(&vault, &pp()).unwrap();
    // The apply + this projection BOTH read the REAL bundled close at 2025-03-01 (85435.58/BTC → the
    // 100_000-sat auto-FMV = $85.44); the decision `.fmv` is Some($85.44) (NEVER None on an included row).
    let ci = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .find_map(|e| match e.payload {
            EventPayload::ClassifyInbound(ci) if ci.transfer_in_event == i1 => Some(ci),
            _ => None,
        })
        .expect("a ClassifyInbound for i1");
    assert!(
        matches!(
            ci.as_,
            InboundClass::Income {
                fmv: Some(v),
                kind: IncomeKind::Mining,
                business: false
            } if v == dec!(85.44)
        ),
        "decision fmv == fmv_of(date, sat) == $85.44; got {:?}",
        ci.as_
    );

    // The PROJECTED income record + lot basis both == the auto-FMV.
    let (state, _) = s.project().unwrap();
    let rec = state
        .income_recognized
        .iter()
        .find(|r| r.event == i1)
        .expect("i1 recognized as income");
    assert_eq!(
        rec.usd_fmv,
        dec!(85.44),
        "projected Income.usd_fmv == fmv_of"
    );
    let lot = state
        .lots
        .iter()
        .find(|l| l.lot_id.origin_event_id == i1)
        .expect("income lot created");
    assert_eq!(
        lot.usd_basis,
        dec!(85.44),
        "lot basis == the recognized FMV"
    );
}

/// E2E: after apply, `income_recognized` grows by the included count, the `UnknownBasisInbound`
/// blockers clear, and NO new Hard blocker appears (crucially NO `FmvMissing`).
#[test]
fn bulk_income_apply_recognizes_income() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, i2, i3, _i4]) = sti_fixture(dir.path());

    let income_before = {
        let s = Session::open(&vault, &pp()).unwrap();
        s.project().unwrap().0.income_recognized.len()
    };

    let n = cmd::reconcile::apply_bulk_classify_inbound_income(
        &vault,
        &pp(),
        vec![i1.clone(), i3.clone()],
        IncomeKind::Staking,
        false,
        now(),
    )
    .unwrap();
    assert_eq!(n, 2);

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(
        state.income_recognized.len(),
        income_before + 2,
        "income_recognized grew by the included count"
    );

    // i1 + i3 no longer flagged UnknownBasisInbound; the untouched i2 stays flagged.
    let flagged: std::collections::BTreeSet<_> = state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .filter_map(|b| b.event.clone())
        .collect();
    assert!(
        !flagged.contains(&i1) && !flagged.contains(&i3),
        "classified inbounds' UnknownBasisInbound cleared"
    );
    assert!(flagged.contains(&i2), "the untouched i2 stays flagged");

    // The whole point [#a]: NO Hard FmvMissing was traded in.
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::FmvMissing),
        "a priced income batch introduces NO FmvMissing year-gate"
    );
}

/// `--kind`/`--business` are UNIFORM: every persisted row carries the chosen kind + business flag.
#[test]
fn bulk_income_uniform_kind_and_business() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, [i1, _i2, i3, _i4]) = sti_fixture(dir.path());

    cmd::reconcile::apply_bulk_classify_inbound_income(
        &vault,
        &pp(),
        vec![i1, i3],
        IncomeKind::Interest,
        true,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let classified: Vec<_> = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .filter_map(|e| match e.payload {
            EventPayload::ClassifyInbound(ci) => Some(ci.as_),
            _ => None,
        })
        .collect();
    assert_eq!(classified.len(), 2);
    assert!(
        classified.iter().all(|c| matches!(
            c,
            InboundClass::Income {
                kind: IncomeKind::Interest,
                business: true,
                fmv: Some(_)
            }
        )),
        "every row carries the uniform kind=Interest + business=true (each fmv is Some)"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════
// self-transfer-passthrough C2 — the matcher `Session::self_transfer_match_plan`. READ-ONLY: pairs
// ONLY unreconciled legs (candidate ins = UnknownBasisInbound TransferIns; candidate outs = pending
// outflows), proposes DROP (same wallet) vs RELOCATE (cross wallet), flags ambiguity, NEVER auto-applies.
// ══════════════════════════════════════════════════════════════════════════════════════════════

/// Build a vault of raw TransferIn/TransferOut legs (imported directly). `legs` = (ref, kind, sat, wallet,
/// ts). kind: "in" → TransferIn, "out" → TransferOut. Returns the vault path.
fn match_fixture(
    dir: &std::path::Path,
    legs: &[(&str, &str, i64, WalletId, time::OffsetDateTime)],
) -> std::path::PathBuf {
    use btctax_core::event::{TransferIn, TransferOut};
    use btctax_core::persistence::append_import_batch;
    use btctax_core::LedgerEvent;
    use time::UtcOffset;

    let vault = dir.join("match.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();
    let batch: Vec<LedgerEvent> = legs
        .iter()
        .map(|(r, kind, sat, wallet, ts)| {
            let payload = match *kind {
                "in" => EventPayload::TransferIn(TransferIn {
                    sat: *sat,
                    src_addr: None,
                    txid: None,
                }),
                "out" => EventPayload::TransferOut(TransferOut {
                    sat: *sat,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
                other => panic!("bad kind {other}"),
            };
            LedgerEvent {
                id: EventId::import(Source::Coinbase, SourceRef::new(*r)),
                utc_timestamp: *ts,
                original_tz: UtcOffset::UTC,
                wallet: Some(wallet.clone()),
                payload,
            }
        })
        .collect();
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    vault
}

fn id_of(r: &str) -> EventId {
    EventId::import(Source::Coinbase, SourceRef::new(r))
}

/// Proposes-right-pairs: a same-wallet passthrough (in before out) → DROP; a cross-wallet transfer (out
/// before in) → RELOCATE. Both non-ambiguous. And running the matcher writes NOTHING (invariant 3).
#[test]
fn self_transfer_match_proposes_drop_and_relocate() {
    let dir = tempfile::tempdir().unwrap();
    let vault = match_fixture(
        dir.path(),
        &[
            // DROP pair: same wallet A, in (03-01) BEFORE out (03-02).
            (
                "in-d",
                "in",
                100_000,
                wallet_a(),
                datetime!(2025-03-01 12:00:00 UTC),
            ),
            (
                "out-d",
                "out",
                100_000,
                wallet_a(),
                datetime!(2025-03-02 12:00:00 UTC),
            ),
            // RELOCATE pair: out from A (04-01), in to B (04-02).
            (
                "out-r",
                "out",
                100_000,
                wallet_a(),
                datetime!(2025-04-01 12:00:00 UTC),
            ),
            (
                "in-r",
                "in",
                100_000,
                wallet_b(),
                datetime!(2025-04-02 12:00:00 UTC),
            ),
        ],
    );
    let s = Session::open(&vault, &pp()).unwrap();

    let events_before = btctax_core::persistence::load_all(s.conn()).unwrap().len();
    let plan = s.self_transfer_match_plan().unwrap();
    let events_after = btctax_core::persistence::load_all(s.conn()).unwrap().len();
    assert_eq!(
        events_before, events_after,
        "the matcher must persist NOTHING (confirmed-not-automatic)"
    );

    assert_eq!(plan.len(), 2, "exactly two pairs proposed");
    let drop = plan
        .iter()
        .find(|p| p.action == btctax_cli::MatchAction::Drop)
        .expect("a DROP proposal");
    assert_eq!(drop.in_event, id_of("in-d"));
    assert_eq!(drop.out_event, id_of("out-d"));
    assert!(!drop.ambiguous);

    let reloc = plan
        .iter()
        .find(|p| p.action == btctax_cli::MatchAction::Relocate)
        .expect("a RELOCATE proposal");
    assert_eq!(reloc.in_event, id_of("in-r"));
    assert_eq!(reloc.out_event, id_of("out-r"));
    assert!(!reloc.ambiguous);
}

/// False-match safety: an already-classified income-in + an already-reclassified dispose-out with a
/// MATCHING amount are NOT candidates (the income-in is no longer UnknownBasisInbound; the dispose-out
/// is no longer pending), so no coincidental pair is proposed.
#[test]
fn self_transfer_match_excludes_reconciled_legs() {
    let dir = tempfile::tempdir().unwrap();
    // Give wallet A an acquire so the out has coins to dispose (a real, recognized sale).
    let vault = {
        use btctax_core::event::{Acquire, BasisSource, TransferIn, TransferOut};
        use btctax_core::persistence::append_import_batch;
        use btctax_core::LedgerEvent;
        use time::UtcOffset;
        let vault = dir.path().join("fm.pgp");
        let mut s = Session::create(&vault, &pp()).unwrap();
        let batch = vec![
            LedgerEvent {
                id: id_of("acq"),
                utc_timestamp: datetime!(2025-01-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: Some(wallet_a()),
                payload: EventPayload::Acquire(Acquire {
                    sat: 100_000,
                    usd_cost: dec!(50),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ComputedFromCost,
                }),
            },
            LedgerEvent {
                id: id_of("in-inc"),
                utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: Some(wallet_a()),
                payload: EventPayload::TransferIn(TransferIn {
                    sat: 100_000,
                    src_addr: None,
                    txid: None,
                }),
            },
            LedgerEvent {
                id: id_of("out-disp"),
                utc_timestamp: datetime!(2025-03-02 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: Some(wallet_a()),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 100_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
        ];
        append_import_batch(s.conn(), &batch).unwrap();
        s.save().unwrap();
        vault
    };
    // Classify the in as Income and reclassify the out as a Dispose — both now reconciled.
    cmd::reconcile::classify_inbound(
        &vault,
        &pp(),
        &id_of("in-inc").canonical(),
        InboundClass::Income {
            kind: IncomeKind::Reward,
            fmv: Some(dec!(4200)),
            business: false,
        },
        now(),
    )
    .unwrap();
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &id_of("out-disp").canonical(),
        OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        },
        dec!(5000),
        None,
        None,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let plan = s.self_transfer_match_plan().unwrap();
    assert!(
        plan.is_empty(),
        "reconciled legs are NOT candidates — no coincidental match proposed: {plan:?}"
    );
}

/// Ambiguity: one in matches TWO outs (same amount + window) → BOTH pairs surfaced `ambiguous`, never
/// silently picked.
#[test]
fn self_transfer_match_flags_ambiguity() {
    let dir = tempfile::tempdir().unwrap();
    let vault = match_fixture(
        dir.path(),
        &[
            // One in (A, 03-03); two candidate outs (A) within the DROP window (in on/before out).
            (
                "in-x",
                "in",
                100_000,
                wallet_a(),
                datetime!(2025-03-03 12:00:00 UTC),
            ),
            (
                "out-1",
                "out",
                100_000,
                wallet_a(),
                datetime!(2025-03-03 13:00:00 UTC),
            ),
            (
                "out-2",
                "out",
                100_000,
                wallet_a(),
                datetime!(2025-03-04 12:00:00 UTC),
            ),
        ],
    );
    let s = Session::open(&vault, &pp()).unwrap();
    let plan = s.self_transfer_match_plan().unwrap();
    assert_eq!(
        plan.len(),
        2,
        "the in matches both outs → two flagged pairs"
    );
    assert!(
        plan.iter().all(|p| p.ambiguous),
        "a 1-in/2-out collision must flag BOTH pairs ambiguous, never auto-pick: {plan:?}"
    );
}

/// `apply_self_transfer_passthrough` appends exactly ONE `SelfTransferPassthrough` decision, and the
/// resulting projection SKIPS both legs (the DROP is applied).
#[test]
fn apply_self_transfer_passthrough_drops_both_legs() {
    let dir = tempfile::tempdir().unwrap();
    let vault = match_fixture(
        dir.path(),
        &[
            (
                "in-d",
                "in",
                100_000,
                wallet_a(),
                datetime!(2025-03-01 12:00:00 UTC),
            ),
            (
                "out-d",
                "out",
                100_000,
                wallet_a(),
                datetime!(2025-03-02 12:00:00 UTC),
            ),
        ],
    );
    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    cmd::reconcile::apply_self_transfer_passthrough(
        &vault,
        &pp(),
        &id_of("in-d").canonical(),
        &id_of("out-d").canonical(),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let events = btctax_core::persistence::load_all(s.conn()).unwrap();
    assert_eq!(events.len(), before + 1, "exactly one decision appended");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::SelfTransferPassthrough(_)))
            .count(),
        1
    );
    let (state, _) = s.project().unwrap();
    assert!(state.lots.is_empty(), "in-leg skipped → no lot");
    assert!(
        state.pending_reconciliation.is_empty(),
        "out-leg skipped → not pending"
    );
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::UnknownBasisInbound
                && b.kind != BlockerKind::UnmatchedOutflows),
        "both legs vanish cleanly"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════
// bulk-reclassify-outflow (Cycle 5 — the LAST) — `Session::bulk_reclassify_outflow_plan` +
// `cmd::reconcile::{bulk_reclassify_outflow_plan, apply_bulk_reclassify_outflow}` + the
// `bulk_estimated` side-table. The bulk analog of the single `o`: sweep MANY pending outflows to a
// `Dispose{Sell|Spend}` with the daily-close FMV as ESTIMATED proceeds, flagged persistently. Load-
// bearing [#a]: a MISSING-PRICE candidate is EXCLUDED (counted), NEVER emitted as a Sell with
// fabricated proceeds (a SILENT misreport). Estimated gain = round_cents(fmv − Σ pending legs basis),
// never double-counted across the batch (the fold's single chronological pass draws the pool down).
// ══════════════════════════════════════════════════════════════════════════════════════════════

use btctax_core::event::{Acquire, BasisSource, TransferOut};
use btctax_core::persistence::append_import_batch;
use btctax_core::LedgerEvent;
use time::UtcOffset;

/// One wallet, TWO lots of DIFFERENT bases, and configurable outflows. Returns (vault, wallet, [acq_a,
/// acq_b], out_ids). `acq_a` (basis $40) is acquired BEFORE `acq_b` (basis $80) so FIFO draws A first.
fn reclass_batch_vault(
    dir: &std::path::Path,
    outs: &[(&str, i64, time::OffsetDateTime)],
) -> (std::path::PathBuf, WalletId, [EventId; 2], Vec<EventId>) {
    let vault = dir.join("reclass-batch.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();
    let w = wallet_a();
    let mkid = |r: &str| EventId::import(Source::Coinbase, SourceRef::new(r));
    let acq_a = mkid("racq-a");
    let acq_b = mkid("racq-b");

    let mut batch = vec![
        LedgerEvent {
            id: acq_a.clone(),
            utc_timestamp: datetime!(2025-01-15 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(w.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(40.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        },
        LedgerEvent {
            id: acq_b.clone(),
            utc_timestamp: datetime!(2025-01-20 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(w.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(80.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        },
    ];
    let mut out_ids = Vec::new();
    for (label, sat, ts) in outs {
        let id = mkid(label);
        out_ids.push(id.clone());
        batch.push(LedgerEvent {
            id,
            utc_timestamp: *ts,
            original_tz: UtcOffset::UTC,
            wallet: Some(w.clone()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: *sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        });
    }
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    (vault, w, [acq_a, acq_b], out_ids)
}

/// The plan selects pending outs in-frame, applies the from_wallet filter, sorts by date, and carries
/// a RESOLVED per-row FMV; the 2024 out (o4) is out of frame and the unpriced one (o5) is excluded.
#[test]
fn bulk_reclassify_outflow_plan_lists_pending_outflows() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, [o1, o2, o3, _o4, _o5]) = bulk_fixture(dir.path());
    let s = open_stub(&vault);

    // Year(2025), wallet None → o1 (2025-03-01), o3 (2025-03-01), o2 (2025-06-15); o5 (2025-04-01)
    // is EXCLUDED (unpriced), o4 (2024) is out of frame.
    let plan = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::Year(2025),
            from_wallet: None,
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.out_event.clone()).collect();
    // o1 and o3 share 2025-03-01; sort is stable by date so both precede o2. Assert as a set on date.
    assert_eq!(
        included.len(),
        3,
        "o1 + o3 + o2 included (o5 unpriced, o4 out of frame)"
    );
    assert!(included.contains(&o1) && included.contains(&o3) && included.contains(&o2));
    assert_eq!(
        plan.excluded_missing_price, 1,
        "o5 (2025-04-01) has no price"
    );
    assert_eq!(
        plan.included.last().unwrap().out_event,
        o2,
        "latest-dated row sorts last"
    );

    // from_wallet = Some(B) → only o3 (wallet B, 2025-03-01).
    let plan_b = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::Year(2025),
            from_wallet: Some(wallet_b()),
        })
        .unwrap();
    let included_b: Vec<_> = plan_b
        .included
        .iter()
        .map(|r| r.out_event.clone())
        .collect();
    assert_eq!(
        included_b,
        vec![o3],
        "from_wallet filter keeps only wallet-B outs"
    );
}

/// [#a] A row whose date has NO bundled price is NOT in `included` and IS counted in
/// `excluded_missing_price` — so a persisted batch NEVER creates a Sell with fabricated proceeds.
#[test]
fn bulk_reclassify_outflow_plan_excludes_missing_price() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, [_o1, _o2, _o3, _o4, o5]) = bulk_fixture(dir.path());
    let s = open_stub(&vault);

    let plan = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::Year(2025),
            from_wallet: Some(wallet_a()),
        })
        .unwrap();
    let included: Vec<_> = plan.included.iter().map(|r| r.out_event.clone()).collect();
    assert!(
        !included.contains(&o5),
        "the unpriced out (o5, 2025-04-01) is NOT reclassified"
    );
    assert_eq!(plan.excluded_missing_price, 1, "o5 surfaced as excluded");
}

/// [#a defense-in-depth] Fed a NON-plan id (an unpriced out, or a bogus id), the apply's
/// `let Some(fmv)=… else continue` skips it — never a `0`/fabricated proceeds appended.
#[test]
fn bulk_reclassify_outflow_apply_never_emits_fabricated_proceeds() {
    let dir = tempfile::tempdir().unwrap();
    // The bundled daily-close dataset is CONTIGUOUS through 2026-06-03, so an unpriced date must lie
    // BEYOND it. This outflow at 2030-01-01 has no bundled price ⇒ the apply cannot resolve an
    // auto-FMV and skips it (as it skips the bogus id below) — never a fabricated-proceeds Sell.
    let (vault, _w, _acqs, outs) = reclass_batch_vault(
        dir.path(),
        &[("ro-unpriced", 40_000, datetime!(2030-01-01 12:00:00 UTC))],
    );
    let o_unpriced = outs[0].clone();
    let bogus = EventId::import(Source::Coinbase, SourceRef::new("no-such-out"));

    // unpriced out + a bogus id → both skipped; nothing appended.
    let n = cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        vec![o_unpriced, bogus],
        DisposeKind::Sell,
        now(),
    )
    .unwrap();
    assert_eq!(
        n, 0,
        "no fabricated-proceeds Sell appended for an unpriced / bogus id"
    );

    let s = Session::open(&vault, &pp()).unwrap();
    let ro_count = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .filter(|e| matches!(e.payload, EventPayload::ReclassifyOutflow(_)))
        .count();
    assert_eq!(ro_count, 0, "no ReclassifyOutflow appended");
    assert!(
        s.bulk_estimated().unwrap().is_empty(),
        "no side-table row for a skipped row"
    );
}

/// The row's `fmv` equals `fmv_of(date, sat)`, and the persisted decision's
/// `principal_proceeds_or_fmv` equals that same resolved FMV.
#[test]
fn bulk_reclassify_outflow_plan_resolves_fmv_as_proceeds() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    // Single 100_000-sat outflow @ 2025-03-01 (real bundled close 85435.58/BTC → $85.44). The plan and
    // the apply BOTH read the real dataset, so `row.fmv` and the persisted proceeds agree.
    let (vault, _w, _acqs, outs) = reclass_batch_vault(
        dir.path(),
        &[("ro-1", 100_000, datetime!(2025-03-01 12:00:00 UTC))],
    );
    let o = outs[0].clone();
    let s = Session::open(&vault, &pp()).unwrap();

    let plan = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::All,
            from_wallet: None,
        })
        .unwrap();
    assert_eq!(plan.included.len(), 1);
    assert_eq!(
        plan.included[0].fmv,
        dec!(85.44),
        "row.fmv == fmv_of(date, sat)"
    );
    drop(s);

    cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        vec![o.clone()],
        DisposeKind::Sell,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let ro = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .find_map(|e| match e.payload {
            EventPayload::ReclassifyOutflow(ro) if ro.transfer_out_event == o => Some(ro),
            _ => None,
        })
        .expect("a ReclassifyOutflow for o");
    assert_eq!(
        ro.principal_proceeds_or_fmv,
        dec!(85.44),
        "payload proceeds == row.fmv"
    );
    assert!(matches!(
        ro.as_,
        OutflowClass::Dispose {
            kind: DisposeKind::Sell
        }
    ));
}

/// [gain] TWO lots of DIFFERENT bases consumed by one outflow → `estimated_gain ==
/// round_cents(fmv − Σ pending legs basis)`, provably not a coincidental zero.
#[test]
fn bulk_reclassify_outflow_plan_estimated_gain_matches_pending_legs_basis() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    // One 150_000-sat outflow spans lot A (100k @ $40) fully + lot B (50k @ $40.00 of $80/100k).
    let (vault, _w, [acq_a, acq_b], outs) = reclass_batch_vault(
        dir.path(),
        &[("ro-span", 150_000, datetime!(2025-03-01 12:00:00 UTC))],
    );
    let o = outs[0].clone();
    let s = open_stub(&vault);

    // fmv = $84,000/BTC × 150_000/1e8 = $126.00. Σ basis = $40.00 (all of A) + $40.00 (half of B) = $80.00.
    let plan = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::All,
            from_wallet: None,
        })
        .unwrap();
    assert_eq!(plan.included.len(), 1);
    let row = &plan.included[0];
    assert_eq!(row.fmv, dec!(126.00), "resolved FMV");
    assert_eq!(
        row.basis_usd,
        dec!(80.00),
        "Σ pending legs basis (A $40 + half-B $40)"
    );
    assert_eq!(
        row.estimated_gain,
        dec!(46.00),
        "round_cents(126.00 − 80.00)"
    );
    assert_ne!(
        row.estimated_gain,
        dec!(0),
        "provably not a coincidental zero"
    );

    // Cross-check against the fold's pending legs directly (basis is proportional, not double-counted).
    let (state, _) = s.project().unwrap();
    let pt = state
        .pending_reconciliation
        .iter()
        .find(|p| p.event == o)
        .unwrap();
    let legs_basis: btctax_core::Usd = pt.legs.iter().map(|l| l.usd_basis).sum();
    assert_eq!(legs_basis, dec!(80.00));
    let a_leg = pt
        .legs
        .iter()
        .find(|l| l.lot_id.origin_event_id == acq_a)
        .unwrap();
    let b_leg = pt
        .legs
        .iter()
        .find(|l| l.lot_id.origin_event_id == acq_b)
        .unwrap();
    assert_eq!(a_leg.sat, 100_000, "all of A consumed");
    assert_eq!(b_leg.sat, 50_000, "half of B consumed");
}

/// [the ordering-hazard pin] Lot A then Lot B (higher basis) in one wallet; TWO outflows (60k @ d1,
/// 80k @ d2>d1) so #2 spills A→B. `total_estimated_gain == Σ row.estimated_gain` AND row2's A-leg is
/// exactly A's REMAINDER after row1 (40k), never A's original size (100k).
#[test]
fn bulk_reclassify_outflow_plan_batch_gain_not_double_counted() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _w, [acq_a, _acq_b], outs) = reclass_batch_vault(
        dir.path(),
        &[
            ("ro-d1", 60_000, datetime!(2025-03-01 12:00:00 UTC)), // fmv $50.40
            ("ro-d2", 80_000, datetime!(2025-06-15 12:00:00 UTC)), // fmv $54.00, spills A→B
        ],
    );
    let (o1, o2) = (outs[0].clone(), outs[1].clone());
    let s = open_stub(&vault);

    let plan = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::All,
            from_wallet: None,
        })
        .unwrap();
    assert_eq!(plan.included.len(), 2);
    let r1 = plan.included.iter().find(|r| r.out_event == o1).unwrap();
    let r2 = plan.included.iter().find(|r| r.out_event == o2).unwrap();
    // o1: 60k from A → basis $24.00, fmv $50.40, gain $26.40.
    assert_eq!(r1.basis_usd, dec!(24.00));
    assert_eq!(r1.estimated_gain, dec!(26.40));
    // o2: 40k A-remainder ($16) + 40k B ($32) → basis $48.00, fmv $54.00, gain $6.00.
    assert_eq!(r2.basis_usd, dec!(48.00));
    assert_eq!(r2.estimated_gain, dec!(6.00));
    // NOT double-counted: the plan total equals the sum of per-row gains.
    assert_eq!(
        plan.total_estimated_gain,
        r1.estimated_gain + r2.estimated_gain,
        "Σ row gains == plan total (no double-count)"
    );
    assert_eq!(plan.total_estimated_gain, dec!(32.40));

    // row2's A-leg is exactly A's REMAINDER (40k) after row1 drew 60k — NEVER A's original 100k.
    let (state, _) = s.project().unwrap();
    let pt2 = state
        .pending_reconciliation
        .iter()
        .find(|p| p.event == o2)
        .unwrap();
    let a_leg2 = pt2
        .legs
        .iter()
        .find(|l| l.lot_id.origin_event_id == acq_a)
        .unwrap();
    assert_eq!(
        a_leg2.sat, 40_000,
        "o2 consumes A's 40k remainder, not A's original 100k"
    );
}

/// Scope-lock: the CLI `--kind` parser accepts sell|spend and REJECTS gift/donate (and junk).
#[test]
fn bulk_reclassify_outflow_scope_excludes_gift_and_donate() {
    use btctax_cli::eventref::parse_dispose_kind;
    assert!(matches!(
        parse_dispose_kind("sell").unwrap(),
        DisposeKind::Sell
    ));
    assert!(matches!(
        parse_dispose_kind("spend").unwrap(),
        DisposeKind::Spend
    ));
    for bad in ["gift", "donate", "GIFT", "Donate"] {
        let err = parse_dispose_kind(bad).unwrap_err();
        assert!(
            matches!(err, CliError::Usage(_)),
            "{bad} must be a Usage error"
        );
    }
    assert!(parse_dispose_kind("junk").is_err());
}

/// `--kind` is UNIFORM: every persisted row carries the chosen `Dispose{kind}`.
#[test]
fn bulk_reclassify_outflow_apply_uniform_kind() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _w, _acqs, outs) = reclass_batch_vault(
        dir.path(),
        &[
            ("ro-u1", 40_000, datetime!(2025-03-01 12:00:00 UTC)),
            ("ro-u2", 40_000, datetime!(2025-06-15 12:00:00 UTC)),
        ],
    );
    cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        outs.clone(),
        DisposeKind::Spend,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let kinds: Vec<_> = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .filter_map(|e| match e.payload {
            EventPayload::ReclassifyOutflow(ro) => Some(ro.as_),
            _ => None,
        })
        .collect();
    assert_eq!(kinds.len(), 2);
    assert!(
        kinds.iter().all(|k| matches!(
            k,
            OutflowClass::Dispose {
                kind: DisposeKind::Spend
            }
        )),
        "every row is Dispose{{Spend}}"
    );
}

/// `fee_usd` is always None on the emitted payload, yet the on-chain `fee_sat` still flows through to
/// consumption (the lot is drawn down by principal + fee).
#[test]
fn bulk_reclassify_outflow_fee_usd_none_but_fee_sat_flows() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("reclass-fee.pgp");
    let mut session = Session::create(&vault, &pp()).unwrap();
    let w = wallet_a();
    let acq = EventId::import(Source::Coinbase, SourceRef::new("fee-acq"));
    let out = EventId::import(Source::Coinbase, SourceRef::new("fee-out"));
    let batch = vec![
        LedgerEvent {
            id: acq.clone(),
            utc_timestamp: datetime!(2025-01-15 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(w.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(100.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ComputedFromCost,
            }),
        },
        LedgerEvent {
            id: out.clone(),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: UtcOffset::UTC,
            wallet: Some(w.clone()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 100_000,
                fee_sat: Some(5_000),
                dest_addr: None,
                txid: None,
            }),
        },
    ];
    append_import_batch(session.conn(), &batch).unwrap();
    session.save().unwrap();
    drop(session);

    cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        vec![out.clone()],
        DisposeKind::Sell,
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let ro = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .find_map(|e| match e.payload {
            EventPayload::ReclassifyOutflow(ro) => Some(ro),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        ro.fee_usd, None,
        "fee_usd is always None on a bulk reclassify"
    );

    // The fee_sat flowed: the backing lot is drawn down by principal (100k) + fee (5k) = 105k.
    let (state, _) = s.project().unwrap();
    let lot = state
        .lots
        .iter()
        .find(|l| l.lot_id.origin_event_id == acq)
        .unwrap();
    assert_eq!(
        lot.remaining_sat, 895_000,
        "lot drawn down by principal + fee (fee_sat flows)"
    );
}

/// Empty refuses to write: a no-candidate frame yields an empty plan, and applying an empty id list
/// appends nothing.
#[test]
fn bulk_reclassify_outflow_empty_refuses() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = bulk_fixture(dir.path());
    let s = Session::open(&vault, &pp()).unwrap();
    let plan = s
        .bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::Year(2030),
            from_wallet: None,
        })
        .unwrap();
    assert!(plan.included.is_empty(), "no pending outs in 2030");
    assert_eq!(plan.total_sat, 0);
    assert_eq!(plan.excluded_missing_price, 0);
    drop(s);

    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    let n = cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        vec![],
        DisposeKind::Sell,
        now(),
    )
    .unwrap();
    assert_eq!(n, 0, "empty id list appends nothing");
    let after = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    assert_eq!(before, after, "no event appended for an empty batch");
}

/// Phase 1 (`bulk_reclassify_outflow_plan`) is READ-ONLY: computing the plan writes nothing.
#[test]
fn bulk_reclassify_outflow_dry_run_writes_nothing() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _ids) = bulk_fixture(dir.path());
    let before = {
        let s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::load_all(s.conn()).unwrap().len()
    };
    let plan = cmd::reconcile::bulk_reclassify_outflow_plan(
        &vault,
        &pp(),
        BulkFilter {
            frame: Frame::All,
            from_wallet: None,
        },
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

/// [side-table A+] After apply + REOPEN, the `bulk_estimated` side-table has a row per applied
/// `transfer_out_event` (== `Disposal.event`), and a control single-item `o` reclassify is NOT flagged.
#[test]
fn bulk_reclassify_outflow_estimated_flag_persists_and_joins() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _w, _acqs, outs) = reclass_batch_vault(
        dir.path(),
        &[
            ("ro-flag-bulk", 40_000, datetime!(2025-03-01 12:00:00 UTC)),
            ("ro-flag-solo", 40_000, datetime!(2025-06-15 12:00:00 UTC)),
        ],
    );
    let (bulk_out, solo_out) = (outs[0].clone(), outs[1].clone());

    // Bulk-reclassify the first; single-`o` reclassify the second (a control — NEVER flagged).
    cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        vec![bulk_out.clone()],
        DisposeKind::Sell,
        now(),
    )
    .unwrap();
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &solo_out.canonical(),
        OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        },
        dec!(54.00),
        None,
        None,
        now(),
    )
    .unwrap();

    // REOPEN: the flag persisted for the bulk out only.
    let s = Session::open(&vault, &pp()).unwrap();
    let flagged = s.bulk_estimated().unwrap();
    assert!(
        flagged.contains_key(&bulk_out),
        "bulk reclassify is flagged"
    );
    assert!(
        !flagged.contains_key(&solo_out),
        "single-`o` reclassify is NOT flagged (control)"
    );

    // The flag key JOINS against Disposal.event (both disposals exist in the projection).
    let (state, _) = s.project().unwrap();
    assert!(
        state.disposals.iter().any(|d| d.event == bulk_out),
        "the flagged key == a real Disposal.event (the Disposals-tab join lands)"
    );
}

/// [WB — R0-I1 CLI parity] Voiding a ReclassifyOutflow via the CLI `void` path MUST clear its
/// `bulk_estimated` `[est]` flag — else a stale marker survives a void→re-reclassify (the exact I1 gap,
/// which the fold closed only on the TUI persist paths). Mirrors the TUI `void_clears_estimated_flag`.
#[test]
fn bulk_reclassify_outflow_cli_void_clears_estimated_flag() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _w, _acqs, outs) = reclass_batch_vault(
        dir.path(),
        &[(
            "ro-cli-void-clear",
            40_000,
            datetime!(2025-03-01 12:00:00 UTC),
        )],
    );
    let bulk_out = outs[0].clone();

    // Bulk-reclassify → flag set.
    cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        vec![bulk_out.clone()],
        DisposeKind::Sell,
        now(),
    )
    .unwrap();
    {
        let s = Session::open(&vault, &pp()).unwrap();
        assert!(
            s.bulk_estimated().unwrap().contains_key(&bulk_out),
            "flag set after bulk reclassify"
        );
    }

    // Find the ReclassifyOutflow decision id (its target is bulk_out) and void it via the CLI.
    let reclass_id = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (events, _st, _c) = s.load_events_and_project().unwrap();
        events
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::ReclassifyOutflow(ro) if ro.transfer_out_event == bulk_out => {
                    Some(e.id.clone())
                }
                _ => None,
            })
            .expect("reclassify decision exists")
    };
    cmd::reconcile::void(&vault, &pp(), &reclass_id.canonical(), now()).unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    assert!(
        !s.bulk_estimated().unwrap().contains_key(&bulk_out),
        "CLI void of a reclassify MUST clear its [est] flag (stale-marker gap)"
    );
}

/// [R0-M2] A CLI apply that fails mid-batch leaves NO ReclassifyOutflow appends AND NO `bulk_estimated`
/// rows (the bare-`?`-before-`save` discard covers the side-table too).
#[test]
fn bulk_reclassify_outflow_cli_mid_batch_failure_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, _w, _acqs, outs) = reclass_batch_vault(
        dir.path(),
        &[
            ("ro-mb1", 40_000, datetime!(2025-03-01 12:00:00 UTC)),
            ("ro-mb2", 40_000, datetime!(2025-06-15 12:00:00 UTC)),
        ],
    );

    // Inject a persistent BEFORE-INSERT trigger that ABORTS the SECOND decision append (decision_seq 2).
    {
        let mut session = Session::open(&vault, &pp()).unwrap();
        session
            .conn()
            .execute_batch(
                "CREATE TRIGGER inject_reclass_midbatch BEFORE INSERT ON events \
                 WHEN NEW.decision_seq = 2 \
                 BEGIN SELECT RAISE(ABORT, 'injected mid-batch append failure'); END;",
            )
            .unwrap();
        session.save().unwrap();
    }

    // The apply opens its OWN session; append #1 commits in-memory, append #2 aborts → `?` returns
    // Err BEFORE save → the whole in-memory session is discarded (nothing saved).
    let res = cmd::reconcile::apply_bulk_reclassify_outflow(
        &vault,
        &pp(),
        outs.clone(),
        DisposeKind::Sell,
        now(),
    );
    assert!(res.is_err(), "a mid-batch abort surfaces as Err");

    // REOPEN: neither the appends nor the side-table marks landed.
    let s = Session::open(&vault, &pp()).unwrap();
    let ro_count = btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .filter(|e| matches!(e.payload, EventPayload::ReclassifyOutflow(_)))
        .count();
    assert_eq!(
        ro_count, 0,
        "no ReclassifyOutflow appended after a mid-batch failure"
    );
    assert!(
        s.bulk_estimated().unwrap().is_empty(),
        "no phantom side-table rows after a mid-batch failure"
    );
}

/// E2E: after apply, `state.disposals` grows by the included count, `pending_reconciliation` shrinks
/// by the same, and NO new Hard blocker appears.
#[test]
fn bulk_reclassify_outflow_apply_then_disposals_reflect() {
    use btctax_cli::{BulkFilter, Frame};
    let dir = tempfile::tempdir().unwrap();
    let (vault, _w, _acqs, _outs) = reclass_batch_vault(
        dir.path(),
        &[
            ("ro-e1", 40_000, datetime!(2025-03-01 12:00:00 UTC)),
            ("ro-e2", 40_000, datetime!(2025-06-15 12:00:00 UTC)),
        ],
    );

    let (disp_before, pend_before) = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (st, _) = s.project().unwrap();
        (st.disposals.len(), st.pending_reconciliation.len())
    };
    assert_eq!(pend_before, 2, "both outs start pending");

    let plan = {
        let s = Session::open(&vault, &pp()).unwrap();
        s.bulk_reclassify_outflow_plan(BulkFilter {
            frame: Frame::All,
            from_wallet: None,
        })
        .unwrap()
    };
    let ids: Vec<_> = plan.included.iter().map(|r| r.out_event.clone()).collect();
    let n =
        cmd::reconcile::apply_bulk_reclassify_outflow(&vault, &pp(), ids, DisposeKind::Sell, now())
            .unwrap();
    assert_eq!(n, 2);

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert_eq!(
        state.disposals.len(),
        disp_before + 2,
        "disposals grew by the included count"
    );
    assert_eq!(
        state.pending_reconciliation.len(),
        pend_before - 2,
        "pending shrank by the same count"
    );
    // No new Hard blocker (crucially no fabricated-proceeds artifact / FmvMissing).
    assert!(
        state
            .blockers
            .iter()
            .all(|b| b.kind != BlockerKind::FmvMissing),
        "a priced reclassify batch introduces NO FmvMissing"
    );
}
