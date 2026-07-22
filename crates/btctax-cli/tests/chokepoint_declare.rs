//! Task 2 (Defensive Filing Wizard, sub-project-2 P-A) — characterization + behavior tests for the
//! extracted DECLARE chokepoint (`btctax_cli::chokepoint`): `plan_declare` / `apply_declare`.
//!
//! Step 1 pins the SHIPPED `cmd::tranche::declare_tranche` behavior BEFORE the refactor — a `$0` declare
//! succeeds; an allocation-conflicting pre-2025 declare refuses. These two tests must stay GREEN across
//! the chokepoint extraction (Steps 3-4): the refactor is behavior-PRESERVING for the `target_shortfall =
//! None` (CLI free-form) path.
//!
//! Step 5 exercises the NEW `target_shortfall = Some(id)` clearance shadow directly against
//! `chokepoint::plan_declare`/`apply_declare`: (b) a same-day candidate fails clearance, a day-before
//! candidate clears (the mutation pair); the pseudo-off KAT (arch-I-5); and the cleared-row KAT.
//!
//! PRIVACY: synthetic values in a tempdir; no real user file is ever read.

use btctax_cli::{chokepoint, cmd, CliError, Session};
use btctax_core::event::{
    AllocMethod, Dispose, DisposeKind, EventPayload, SafeHarborAllocation, TransferIn,
};
use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
use btctax_core::persistence::{append_decision, append_import_batch, load_all};
use btctax_core::{LedgerEvent, LotMethod};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::path::{Path, PathBuf};
use time::macros::{date, datetime};
use time::UtcOffset;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    datetime!(2026 - 01 - 01 0:00 UTC)
}
fn wallet() -> WalletId {
    WalletId::SelfCustody {
        label: "chokepoint-t2".into(),
    }
}

fn decl_count(session: &Session) -> usize {
    load_all(session.conn())
        .unwrap()
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::DeclareTranche(_)))
        .count()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Step 1 — characterization: pin the SHIPPED `cmd::tranche::declare_tranche` behavior BEFORE the refactor.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ Step 1(a): a `$0` declare on an empty vault succeeds — appends one `DeclareTranche` decision that
/// folds to a $0-basis `EstimatedConservative` lot homed at `window_end`, in the declared wallet.
#[test]
fn shipped_declare_tranche_zero_basis_declare_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let id = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        wallet(),
        date!(2020 - 01 - 01),
        date!(2020 - 12 - 31),
        now(),
    )
    .expect("a $0 declare on an empty vault must succeed");
    assert!(matches!(id, EventId::Decision { .. }));

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let lot = state
        .lots
        .iter()
        .find(|l| l.wallet == wallet())
        .expect("a tranche lot in the declared wallet");
    assert_eq!(lot.usd_basis, btctax_core::Usd::ZERO, "tranche basis is $0");
    assert_eq!(
        lot.basis_source,
        btctax_core::BasisSource::EstimatedConservative
    );
    assert_eq!(
        lot.acquired_at,
        date!(2020 - 12 - 31),
        "homed at window_end"
    );
}

/// A vault holding ONE hand-crafted, non-voided `SafeHarborAllocation` decision. `guard_tranche_vs_
/// allocation`/`in_force_allocation_exists` are PURE event-EXISTENCE scans (they never inspect the
/// allocation's computed effectiveness), so a minimal hand-crafted event exercises the SAME guard the
/// full real allocate flow would (`declare_tranche_cli.rs::vault_effective_alloc` runs the full CSV-
/// import + `safe_harbor_allocate` pipeline for a richer end-to-end fixture; this one isolates just the
/// guard this task's refactor touches).
fn vault_with_a_safe_harbor_allocation(dir: &Path) -> PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let mut s = Session::open(&vault, &pp()).unwrap();
    append_decision(
        s.conn(),
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots: vec![],
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ActualPosition,
            timely_allocation_attested: true,
            pre2025_method: LotMethod::Fifo,
        }),
        now(),
        UtcOffset::UTC,
        None,
    )
    .unwrap();
    s.save().unwrap();
    vault
}

/// ★ Step 1(b): an allocation-conflicting PRE-2025 declare refuses (D-8's mutual exclusion) — appends
/// NOTHING (fail-closed).
#[test]
fn shipped_declare_tranche_refuses_under_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_with_a_safe_harbor_allocation(dir.path());

    let err = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        wallet(),
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)),
        "expected Usage; got {err}"
    );
    assert!(
        err.to_string().to_lowercase().contains("allocation"),
        "the refusal must name the allocation collision: {err}"
    );

    let s = Session::open(&vault, &pp()).unwrap();
    assert_eq!(
        decl_count(&s),
        0,
        "the refused tranche must NOT be appended (fail-closed)"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Step 5(a) — the CLI `None` path via the NEW `plan_declare`/`apply_declare`: shipped semantics preserved.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ Step 5(a): `target_shortfall = None` is NOT refused on a targets-no-shortfall declare — the shipped
/// gate set only (no clearance shadow runs at all).
#[test]
fn none_path_targets_no_shortfall_is_not_refused() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let mut session = Session::open(&vault, &pp()).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();

    let plan = chokepoint::plan_declare(
        &events,
        session.prices(),
        &cfg,
        50_000_000,
        wallet(),
        date!(2020 - 01 - 01),
        date!(2020 - 12 - 31),
        None,
        now(),
    )
    .expect(
        "a targets-no-shortfall declare (None) must NOT be refused — shipped semantics preserved",
    );

    let id = chokepoint::apply_declare(&mut session, plan, now()).unwrap();
    assert!(matches!(id, EventId::Decision { .. }));

    let (state, _) = session.project().unwrap();
    let lot = state
        .lots
        .iter()
        .find(|l| l.wallet == wallet())
        .expect("a tranche lot in the declared wallet");
    assert_eq!(lot.usd_basis, btctax_core::Usd::ZERO);
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Step 5(b)/arch-I-5 — the `Some(id)` target-scoped clearance shadow.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A wallet with a single Sell (`Dispose`) at `disposal_date`, `sat` sat, and NO other lots anywhere in
/// that wallet — a full, sat-for-sat `UncoveredDisposal` shortfall on the Sell's own `EventId`. Returns
/// (vault, the Sell's `EventId`).
fn vault_with_uncovered_disposal(
    dir: &Path,
    disposal_date: time::Date,
    sat: i64,
) -> (PathBuf, EventId) {
    let vault = dir.join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();
    let sell_id = EventId::import(Source::Coinbase, SourceRef::new("SELL"));
    let sell = LedgerEvent {
        id: sell_id.clone(),
        utc_timestamp: disposal_date.midnight().assume_utc(),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload: EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: dec!(1_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };
    append_import_batch(s.conn(), &[sell]).unwrap();
    s.save().unwrap();
    (vault, sell_id)
}

/// ★ Step 5(b): a candidate whose `window_end == disposal date` does NOT clear — a decision's synthetic
/// acquisition sorts AFTER a same-instant import (`resolve.rs:~1312`), so the tranche's lot is not yet in
/// the pool when the same-day disposal folds. Refuses via `Refusal::Coverage`; appends nothing.
#[test]
fn some_path_candidate_at_disposal_date_fails_clearance() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, sell_id) =
        vault_with_uncovered_disposal(dir.path(), date!(2026 - 06 - 01), 50_000_000);

    let session = Session::open(&vault, &pp()).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();

    let err = chokepoint::plan_declare(
        &events,
        session.prices(),
        &cfg,
        50_000_000,
        wallet(),
        date!(2020 - 01 - 01),
        date!(2026 - 06 - 01), // == disposal date
        Some(sell_id),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, chokepoint::Refusal::Coverage(_)),
        "a same-day candidate must refuse via Refusal::Coverage: {err:?}"
    );

    assert_eq!(
        decl_count(&session),
        0,
        "a plan-time refusal must not have appended anything"
    );
}

/// ★ Step 5(b) mutation: the SAME candidate, `window_end` moved to the day BEFORE the disposal, clears
/// (proves the boundary is exactly "strictly before", not an off-by-one elsewhere).
#[test]
fn some_path_candidate_before_disposal_date_clears() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, sell_id) =
        vault_with_uncovered_disposal(dir.path(), date!(2026 - 06 - 01), 50_000_000);

    let session = Session::open(&vault, &pp()).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();

    let plan = chokepoint::plan_declare(
        &events,
        session.prices(),
        &cfg,
        50_000_000,
        wallet(),
        date!(2020 - 01 - 01),
        date!(2026 - 05 - 31), // strictly BEFORE the disposal date
        Some(sell_id),
        now(),
    )
    .expect("a candidate window_end strictly before the disposal date must clear");
    assert!(matches!(plan.payload, EventPayload::DeclareTranche(_)));
}

/// ★ arch-I-5 (cleared-row KAT): a candidate that DOES clear → `apply_declare` removes the shortfall —
/// re-projecting the REAL (applied) vault afterward shows NO `UncoveredDisposal` blocker remains on the
/// targeted disposal.
#[test]
fn apply_declare_clears_the_uncovereddisposal_blocker_on_the_targeted_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let (vault, sell_id) =
        vault_with_uncovered_disposal(dir.path(), date!(2026 - 06 - 01), 50_000_000);

    let mut session = Session::open(&vault, &pp()).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();

    // Sanity: the shortfall is really there before the tranche.
    let (before, _) = session.project().unwrap();
    assert!(
        before
            .blockers
            .iter()
            .any(|b| b.kind == btctax_core::BlockerKind::UncoveredDisposal
                && b.event.as_ref() == Some(&sell_id)),
        "fixture must start with a real shortfall on the Sell: {:?}",
        before.blockers
    );

    let plan = chokepoint::plan_declare(
        &events,
        session.prices(),
        &cfg,
        50_000_000,
        wallet(),
        date!(2020 - 01 - 01),
        date!(2026 - 05 - 31),
        Some(sell_id.clone()),
        now(),
    )
    .unwrap();
    chokepoint::apply_declare(&mut session, plan, now()).unwrap();

    let (after, _) = session.project().unwrap();
    assert!(
        !after
            .blockers
            .iter()
            .any(|b| b.kind == btctax_core::BlockerKind::UncoveredDisposal
                && b.event.as_ref() == Some(&sell_id)),
        "the applied tranche must clear the targeted shortfall: {:?}",
        after.blockers
    );
}

/// ★ arch-I-5: the clearance shadow forces `pseudo_reconcile = false` — a pseudo `SelfTransferMine{$0}`
/// default must NOT mask a real shortfall. Fixture: an UNRESOLVED `TransferIn` (sat matches the shortfall
/// exactly) dated BEFORE a Sell with no other lots in the same wallet; pseudo mode is turned ON in the
/// STORED config (mirroring a dashboard user who left pseudo-reconcile active). The candidate itself does
/// NOT cover anything (its `window_end` is strictly AFTER the disposal) — so IF the clearance shadow used
/// the caller's cfg as-is (pseudo ON), the TransferIn's synthetic `SelfTransferMine{$0}` default would
/// fully cover the Sell's shortfall, producing a FALSE clearance pass. Forcing pseudo off correctly still
/// refuses. Mutation-verified (dev-time): commenting out `plan_declare`'s
/// `honest_cfg.pseudo_reconcile = false;` line flips this test from `Err` to `Ok`.
#[test]
fn clearance_shadow_forces_pseudo_off_a_pseudo_selftransfer_cannot_mask_a_real_shortfall() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    let mut s = Session::create(&vault, &pp()).unwrap();

    let transfer_in = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("TIN")),
        utc_timestamp: datetime!(2026 - 01 - 01 0:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 50_000_000,
            src_addr: None,
            txid: None,
        }),
    };
    let sell_id = EventId::import(Source::Coinbase, SourceRef::new("SELL"));
    let sell = LedgerEvent {
        id: sell_id.clone(),
        utc_timestamp: datetime!(2026 - 06 - 01 0:00 UTC),
        original_tz: UtcOffset::UTC,
        wallet: Some(wallet()),
        payload: EventPayload::Dispose(Dispose {
            sat: 50_000_000,
            usd_proceeds: dec!(1_000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };
    append_import_batch(s.conn(), &[transfer_in, sell]).unwrap();
    s.save().unwrap();
    drop(s); // release the vault lock before pseudo_set_mode re-opens it

    // Turn pseudo-reconcile ON in the STORED config — this is what the clearance shadow must override.
    cmd::reconcile::pseudo_set_mode(&vault, &pp(), true).unwrap();

    let session = Session::open(&vault, &pp()).unwrap();
    let events = load_all(session.conn()).unwrap();
    let cfg = session.config().unwrap().to_projection();
    assert!(
        cfg.pseudo_reconcile,
        "fixture precondition: pseudo must be ON in the caller's cfg"
    );

    // A candidate that does NOT itself cover anything (window_end strictly AFTER the disposal).
    let err = chokepoint::plan_declare(
        &events,
        session.prices(),
        &cfg,
        1,
        wallet(),
        date!(2026 - 07 - 01),
        date!(2026 - 12 - 31),
        Some(sell_id),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, chokepoint::Refusal::Coverage(_)),
        "a pseudo SelfTransferMine{{$0}} default must NOT mask a real shortfall: {err:?}"
    );
}
