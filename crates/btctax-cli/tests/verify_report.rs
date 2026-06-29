mod fixtures;
use btctax_cli::{cmd, render};
use btctax_core::{AllocMethod, BasisSource, BlockerKind, EventPayload};
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

/// CLI-I2 fix: after the attest happy-path (allocate-unattested → timebar advisory → void →
/// re-allocate-unattested → attest), the engine is on effective Path B. The OLD safe_harbor_status
/// checked `timebar` before `effective_path_b` and therefore mis-reported "time-barred" because the
/// stale SafeHarborTimebar advisory (from the now-voided inert allocation) was still in state.blockers.
/// The fix: check for SafeHarborAllocated lots first; report "Path B effective" when they are present.
#[test]
fn verify_reports_path_b_effective_not_time_barred_after_attest_happy_path() {
    use time::macros::datetime;
    let now = datetime!(2026-02-01 12:00:00 UTC);

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Pre-2025 Buy + 2025 Sell: unattested allocation is time-barred (made in 2026 is after the
    // first-2025-disposition prong of the ActualPosition bar).
    let p = dir.path().join("cb.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-sell,2025-06-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,90000.00,4500.00,4490.00,10.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Step 1: allocate (unattested) → inert due to time-bar.
    let a1 = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now,
    )
    .unwrap();

    // Step 2: void the inert allocation.
    cmd::reconcile::void(&vault, &pp(), &a1.canonical(), now).unwrap();

    // Step 3: re-allocate (unattested) — still inert, still time-barred.
    cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now)
        .unwrap();

    // Step 4: attest — voids prior, appends attested copy; now effective Path B.
    cmd::reconcile::safe_harbor_attest(&vault, &pp(), now).unwrap();

    // Sanity: the projection has SafeHarborAllocated lots and a stale SafeHarborTimebar advisory.
    // Open a temporary session in a block so the lock is released before cmd::inspect::verify.
    let (has_allocated_lots, has_stale_timebar, attested_alloc_exists) = {
        let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
        let events = btctax_core::persistence::load_all(s.conn()).unwrap();
        let (state, _) = s.project().unwrap();
        let has_allocated = state.lots.iter().any(|l| {
            matches!(
                l.basis_source,
                btctax_core::BasisSource::SafeHarborAllocated
            )
        });
        let has_timebar = state
            .blockers
            .iter()
            .any(|b| b.kind == btctax_core::BlockerKind::SafeHarborTimebar);
        let attested = events.iter().any(|e| match &e.payload {
            EventPayload::SafeHarborAllocation(a) => a.timely_allocation_attested,
            _ => false,
        });
        (has_allocated, has_timebar, attested)
        // `s` is dropped here, releasing the vault lock.
    };
    assert!(
        has_allocated_lots,
        "effective Path B: SafeHarborAllocated lots must exist after attest"
    );
    assert!(
        has_stale_timebar,
        "stale SafeHarborTimebar advisory must remain (from voided inert allocation)"
    );
    assert!(
        attested_alloc_exists,
        "an attested SafeHarborAllocation must exist in the event log"
    );

    // The fix: verify reports Path B effective, NOT time-barred.
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(
        report.safe_harbor.contains("effective"),
        "safe_harbor must say 'effective' after attest, got: {:?}",
        report.safe_harbor
    );
    assert!(
        !report.safe_harbor.contains("time-barred"),
        "safe_harbor must NOT say 'time-barred' after attest, got: {:?}",
        report.safe_harbor
    );

    // render_verify also carries the correct string through.
    let text = render::render_verify(&report);
    assert!(
        text.contains("Path B safe-harbor allocation is effective"),
        "rendered verify must show 'Path B safe-harbor allocation is effective', got:\n{text}"
    );
}

/// Fix for CLI-I3: `safe_harbor_status` went dark (reported "time-barred → Path A") when
/// ALL Path-B allocated lots were fully consumed (remaining_sat==0 → filtered by `finalize`).
/// The old code only looked at `state.lots`; the fix also checks disposal/removal legs which
/// retain the `SafeHarborAllocated` basis_source even after all lots are consumed.
#[test]
fn safe_harbor_status_remains_effective_when_all_path_b_lots_are_consumed() {
    use time::macros::datetime;
    let now = datetime!(2026-02-01 12:00:00 UTC);

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Pre-2025 Buy 0.10 BTC + 2025 Sell ALL 0.10 BTC.
    // After Path B seeding at 2025-01-01, the Sell fully consumes every SafeHarborAllocated lot.
    // state.lots will be empty; state.disposals legs will carry SafeHarborAllocated basis_source.
    let p = dir.path().join("cb.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.10000000,USD,42500.00,4250.00,4275.00,25.00,,,\r\n\
cb-sell-all,2025-06-15 12:00:00 UTC,Sell,BTC,0.10000000,USD,67500.00,6750.00,6740.00,10.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Allocate (unattested) → inert: made 2026-02-01 is after the 2025-06-15 ActualPosition bar.
    let a1 = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        AllocMethod::ActualPosition,
        false,
        now,
    )
    .unwrap();
    // Void alloc #1 — a stale SafeHarborTimebar advisory for a1 remains in state.blockers.
    cmd::reconcile::void(&vault, &pp(), &a1.canonical(), now).unwrap();
    // Re-allocate (still unattested, still inert).
    cmd::reconcile::safe_harbor_allocate(&vault, &pp(), AllocMethod::ActualPosition, false, now)
        .unwrap();
    // Attest → voids alloc #2, appends attested alloc #3; effective Path B.
    cmd::reconcile::safe_harbor_attest(&vault, &pp(), now).unwrap();

    // Sanity checks: no SafeHarborAllocated lots remain (all consumed); stale timebar is present;
    // disposal legs carry SafeHarborAllocated.
    {
        let s = btctax_cli::Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .lots
                .iter()
                .all(|l| l.basis_source != BasisSource::SafeHarborAllocated),
            "all SafeHarborAllocated lots must be fully consumed (remaining_sat==0 → filtered)"
        );
        assert!(
            state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::SafeHarborTimebar),
            "stale SafeHarborTimebar advisory must remain (from voided alloc #1)"
        );
        assert!(
            state.disposals.iter().any(|d| d
                .legs
                .iter()
                .any(|l| l.basis_source == BasisSource::SafeHarborAllocated)),
            "disposal legs must carry SafeHarborAllocated (the consumed seed lots)"
        );
    }

    // The fix: verify reports Path B effective, NOT time-barred.
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(
        report.safe_harbor.contains("effective"),
        "safe_harbor_status must report 'effective' even when all allocated lots are consumed, \
         got: {:?}",
        report.safe_harbor
    );
    assert!(
        !report.safe_harbor.contains("time-barred"),
        "safe_harbor_status must NOT say 'time-barred' after a successful attest, \
         got: {:?}",
        report.safe_harbor
    );
    let text = render::render_verify(&report);
    assert!(
        text.contains("Path B safe-harbor allocation is effective"),
        "render_verify text must show 'Path B … effective', got:\n{text}"
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
