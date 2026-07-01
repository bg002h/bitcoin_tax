mod fixtures;
use btctax_cli::{cmd, render, Session};
use btctax_core::{AllocMethod, BasisSource, BlockerKind, EventPayload, OutflowClass};
use btctax_store::Passphrase;
use time::macros::{date, datetime};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

fn now() -> time::OffsetDateTime {
    datetime!(2026-02-01 12:00:00 UTC)
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
    // D3 (Task 3): attest FIFO (pre2025_method_attested) before the allocate gate.
    // `timely_allocation_attested` (4th arg, false below) is a separate §5.02(4) attestation.
    cmd::admin::set_pre2025_method(&vault, &pp(), btctax_core::LotMethod::Fifo, true).unwrap();

    // Step 1: allocate (unattested = timely_allocation_attested=false) → inert due to time-bar.
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
    // D3 (Task 3): attest FIFO (pre2025_method_attested) before the allocate gate.
    cmd::admin::set_pre2025_method(&vault, &pp(), btctax_core::LotMethod::Fifo, true).unwrap();

    // Allocate (unattested = timely_allocation_attested=false) → inert: made 2026-02-01 is after the 2025-06-15 ActualPosition bar.
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

// ── Task 8 tests ────────────────────────────────────────────────────────────────────────────────

/// Task 8: `verify` surfaces the declared `pre2025_method` and whether it is attested.
/// `render_verify` must include the uppercase method name (e.g. "HIFO") and the word "attested".
#[test]
fn verify_reports_declared_method_and_attestation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::admin::set_pre2025_method(&vault, &pp(), btctax_core::LotMethod::Hifo, true).unwrap();
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert_eq!(report.declared_pre2025_method, btctax_core::LotMethod::Hifo);
    assert!(report.pre2025_method_attested);
    let text = render::render_verify(&report);
    assert!(
        text.contains("HIFO") && text.contains("attested"),
        "render_verify must include 'HIFO' and 'attested', got:\n{text}"
    );
}

/// Task 8: `verify` reports the standing-order history (election count), the `LotSelection` count,
/// and a non-empty per-disposal compliance vector when disposals exist.
#[test]
fn verify_lists_election_history_and_selection_count_and_compliance() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    // Post-2025 buy + sell (+ send): exactly one 2025 disposal; cb-sell consumes one lot fully ->
    // a single leg.
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();
    // Forward standing order (MethodElection) effective 2025-06-01.
    cmd::reconcile::set_forward_method(
        &vault,
        &pp(),
        btctax_core::LotMethod::Hifo,
        Some(date!(2025 - 06 - 01)),
        now(),
    )
    .unwrap();
    // Read the 2025 disposal eventref + the lot/sat its single leg consumes; record a
    // contemporaneous selection.
    let (disposal_ref, lot_ref, principal) = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        let leg = &state.disposals[0].legs[0];
        (
            state.disposals[0].event.canonical(),
            format!(
                "{}#{}",
                leg.lot_id.origin_event_id.canonical(),
                leg.lot_id.split_sequence
            ),
            leg.sat,
        )
    };
    let picks =
        vec![btctax_cli::eventref::parse_lot_pick(&format!("{lot_ref}:{principal}")).unwrap()];
    cmd::reconcile::select_lots(&vault, &pp(), &disposal_ref, picks, now()).unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert_eq!(
        report.elections.len(),
        1,
        "one MethodElection decision expected"
    );
    assert_eq!(
        report.selection_count, 1,
        "one LotSelection decision expected"
    );
    assert!(
        !report.compliance.is_empty(),
        "per-disposal compliance must be non-empty (post-2025 sell exists)"
    );
}

/// Task 8: a `LotSelection` with a principal mismatch (1 sat != disposal principal) fires a hard
/// `LotSelectionInvalid` blocker; `verify` partitions it into `report.hard` + signals non-zero (FR9).
#[test]
fn verify_partitions_lot_selection_invalid_as_hard() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(
        &vault,
        &pp(),
        &[fixtures::coinbase_buy_sell_send(dir.path())],
    )
    .unwrap();
    let (disposal_ref, lot_ref) = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        let leg = &state.disposals[0].legs[0];
        (
            state.disposals[0].event.canonical(),
            format!(
                "{}#{}",
                leg.lot_id.origin_event_id.canonical(),
                leg.lot_id.split_sequence
            ),
        )
    };
    // pick 1 sat — deliberately != the disposal principal → conservation violation →
    // hard LotSelectionInvalid.
    let picks = vec![btctax_cli::eventref::parse_lot_pick(&format!("{lot_ref}:1")).unwrap()];
    cmd::reconcile::select_lots(&vault, &pp(), &disposal_ref, picks, now()).unwrap();
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(
        report
            .hard
            .iter()
            .any(|b| b.kind == btctax_core::BlockerKind::LotSelectionInvalid),
        "LotSelectionInvalid must appear in report.hard; hard blockers: {:?}",
        report.hard
    );
    assert!(
        report.has_hard_blockers(),
        "has_hard_blockers() must be true → non-zero exit (FR9)"
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

// ── Task 4 (pre-2025 method reconciliation): verify consistency KATs ───────────────────────────

/// Task 4 KAT — ATTESTED vault: `verify` shows "DECLARED + ATTESTED" advisory + "attested: true"
/// with NO "have NOT declared" warning. Requires a pre-2025 disposal so Pre2025MethodNote fires
/// (the note fires on the first pre-2025 Dispose/GiftOut/Donate, not on a Buy or post-2025 event).
#[test]
fn verify_consistency_attested_vault_shows_declared_attested_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Pre-2025 Buy + pre-2025 Sell to trigger Pre2025MethodNote (fires on pre-2025 Dispose).
    let p = dir.path().join("cb_pre_dispose.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-pre-buy,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-pre-sell,2024-06-15 12:00:00 UTC,Sell,BTC,0.05000000,USD,50000.00,2500.00,2490.00,10.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Attest FIFO: declare the filed pre-2025 method.
    cmd::admin::set_pre2025_method(&vault, &pp(), btctax_core::LotMethod::Fifo, true).unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    let text = render::render_verify(&report);

    // "attested: true" must appear in the Pre-2025 method line (render.rs outputs this field).
    assert!(
        text.contains("attested: true"),
        "render_verify must contain 'attested: true' for an attested vault, got:\n{text}"
    );
    // Pre2025MethodNote advisory detail must contain "DECLARED + ATTESTED" (attested branch, D2).
    assert!(
        report
            .advisory
            .iter()
            .any(|b| b.kind == BlockerKind::Pre2025MethodNote
                && b.detail.contains("DECLARED + ATTESTED")),
        "advisory Pre2025MethodNote detail must contain 'DECLARED + ATTESTED' when attested; \
         advisories: {:?}",
        report.advisory
    );
    // No "have NOT declared" warning in rendered text (only the attested branch fires).
    assert!(
        !text.contains("have NOT declared"),
        "attested vault must NOT show 'have NOT declared' in render_verify, got:\n{text}"
    );
}

/// Task 4 KAT — UNATTESTED vault: `verify` shows "have NOT declared" advisory + "attested: false"
/// with NO "DECLARED + ATTESTED" text. Requires a pre-2025 disposal so Pre2025MethodNote fires.
#[test]
fn verify_consistency_unattested_vault_shows_have_not_declared_advisory() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Same pre-2025 Buy + pre-2025 Sell fixture as the attested KAT above.
    let p = dir.path().join("cb_pre_dispose.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-pre-buy,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-pre-sell,2024-06-15 12:00:00 UTC,Sell,BTC,0.05000000,USD,50000.00,2500.00,2490.00,10.00,,,\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    // Default config: pre2025_method_attested = false (NOT attested). Do NOT call set_pre2025_method.

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    let text = render::render_verify(&report);

    // "attested: false" must appear in the Pre-2025 method line.
    assert!(
        text.contains("attested: false"),
        "render_verify must contain 'attested: false' for an unattested vault, got:\n{text}"
    );
    // Pre2025MethodNote advisory detail must contain "have NOT declared" (unattested branch, D2).
    assert!(
        report
            .advisory
            .iter()
            .any(|b| b.kind == BlockerKind::Pre2025MethodNote
                && b.detail.contains("have NOT declared")),
        "advisory Pre2025MethodNote detail must contain 'have NOT declared' when unattested; \
         advisories: {:?}",
        report.advisory
    );
    // No "DECLARED + ATTESTED" text in the rendered output.
    assert!(
        !text.contains("DECLARED + ATTESTED"),
        "unattested vault must NOT show 'DECLARED + ATTESTED' in render_verify, got:\n{text}"
    );
}

// ── Task-3 verify KAT: §170(f)(11)(C) QualifiedAppraisalNote surfaced under verify ────────────

/// Task-3 KAT (large): a vault with a >$5k-proxy LT donation shows QualifiedAppraisalNote
/// under verify's Advisory blockers with the deduction/threshold/§170 detail.
/// Setup: Buy 2025-01-05 0.10 BTC → Send 2026-01-30 (LT: >1yr after 2025-01-05) → Donate FMV=$90k.
/// LT proxy = FMV = $90,000 > $5,000 → FLAGGED.
#[test]
fn verify_donation_over_5k_proxy_shows_qualified_appraisal_note() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Write a synthetic Coinbase CSV: Buy 0.10 BTC on 2025-01-05, Send 0.10 BTC on 2026-01-30.
    // 2026-01-30 vs one_year_after(2025-01-05) = 2026-01-05: 2026-01-30 > 2026-01-05 → LT.
    let p = dir.path().join("cb_appr.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-appr-buy,2025-01-05 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-appr-send,2026-01-30 12:00:00 UTC,Send,BTC,0.10000000,USD,90000.00,,,,,,bc1qsyntheticdest\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Get the TransferOut ref from pending and reclassify as Donate with FMV = $90,000.
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert_eq!(
            state.pending_reconciliation.len(),
            1,
            "Send must be pending before reclassification"
        );
        state.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("90000.00").unwrap(), // FMV = $90k → LT proxy > $5k
        None,
        now(),
    )
    .unwrap();

    // verify must surface QualifiedAppraisalNote in advisory blockers.
    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    let note = report
        .advisory
        .iter()
        .find(|b| b.kind == BlockerKind::QualifiedAppraisalNote);
    assert!(
        note.is_some(),
        "QualifiedAppraisalNote must appear in report.advisory for LT $90k donation; \
         advisories: {:?}",
        report.advisory
    );
    // Verify the detail references the key statutory items.
    let detail = &note.unwrap().detail;
    assert!(
        detail.contains("90000.00"),
        "detail must name the deduction proxy ($90k); got: {detail}"
    );
    assert!(
        detail.contains("§170(f)(11)(C)"),
        "detail must cite §170(f)(11)(C); got: {detail}"
    );
    assert!(
        detail.contains("CCA 202302012"),
        "detail must cite CCA 202302012 (crypto-specific point); got: {detail}"
    );
    assert!(
        detail.contains("§170(e)"),
        "detail must cite §170(e) (character caveat); got: {detail}"
    );
    assert!(
        detail.contains("§170(f)(11)(F)"),
        "detail must cite §170(f)(11)(F) (aggregation caveat); got: {detail}"
    );
    // Advisory must not gate: no hard blockers, report is computable.
    assert!(
        report.hard.is_empty(),
        "QualifiedAppraisalNote must not create hard blockers; hard: {:?}",
        report.hard
    );
    // render_verify includes advisory in output.
    let text = render::render_verify(&report);
    assert!(
        text.contains("QualifiedAppraisalNote"),
        "render_verify must render QualifiedAppraisalNote advisory; got:\n{text}"
    );
}

/// Task-3 KAT (small): a vault with a small (<$5k-proxy) ST donation shows NO QualifiedAppraisalNote.
/// Setup: Buy 2025-07-01 0.01 BTC ($510 cost) → Send 2026-01-15 (ST: <1yr) → Donate FMV=$1k.
/// ST proxy = basis = $510 < $5,000 → NOT FLAGGED.
#[test]
fn verify_donation_under_5k_proxy_shows_no_qualified_appraisal_note() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // Buy 2025-07-01, Send 2026-01-15: 2026-01-15 vs one_year_after(2025-07-01)=2026-07-01 → ST.
    // Basis = $510 → ST proxy = basis = $510 < $5,000 → NOT FLAGGED.
    let p = dir.path().join("cb_small_appr.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
cb-small-buy,2025-07-01 12:00:00 UTC,Buy,BTC,0.01000000,USD,50000.00,500.00,510.00,10.00,,,\r\n\
cb-small-send,2026-01-15 12:00:00 UTC,Send,BTC,0.01000000,USD,50000.00,,,,,,bc1qsyntheticdest\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Reclassify as Donate with FMV = $1,000 (FMV doesn't matter for ST — proxy = basis = $510).
    let out_ref = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        state.pending_reconciliation[0].event.canonical()
    };
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &out_ref,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("1000.00").unwrap(), // FMV; ST → proxy = basis = $510
        None,
        now(),
    )
    .unwrap();

    let report = cmd::inspect::verify(&vault, &pp()).unwrap();
    assert!(
        !report
            .advisory
            .iter()
            .any(|b| b.kind == BlockerKind::QualifiedAppraisalNote),
        "small ST donation (proxy = basis $510 < $5k) must NOT show QualifiedAppraisalNote; \
         advisories: {:?}",
        report.advisory
    );
}

// ── P2-A Task 2 KATs: render_report charitable total + donation header + CSV column ─────────────

/// (P2-A-T2-a) render_report per-year charitable-deduction total:
/// - Two donations in year 2026 → total = their sum.
/// - A prior-year donation (2025) is excluded from the year=2026 total.
/// - The total line carries "BEFORE §170(b) AGI limits / carryover" qualifier [R0-I2].
/// - The total line says "Schedule A itemized".
#[test]
fn render_report_charitable_total_year_filter_and_qualifier() {
    use btctax_core::{EventId, LedgerState, Removal, RemovalKind, Source, SourceRef};
    use rust_decimal_macros::dec;
    use time::macros::date;

    // Three donations: one in 2025 (prior year), two in 2026.
    let make_removal = |src_ref: &str, date: time::Date, amount| Removal {
        event: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
        kind: RemovalKind::Donation,
        removed_at: date,
        legs: vec![],
        appraisal_required: false,
        donor_acquired_at: None,
        claimed_deduction: Some(amount),
    };
    let mut state = LedgerState::default();
    state
        .removals
        .push(make_removal("D-2025", date!(2025 - 12 - 15), dec!(8000.00))); // prior year
    state.removals.push(make_removal(
        "D-2026a",
        date!(2026 - 03 - 01),
        dec!(10000.00),
    )); // year 2026
    state.removals.push(make_removal(
        "D-2026b",
        date!(2026 - 06 - 01),
        dec!(6000.00),
    )); // year 2026

    // Year 2026: total = $10,000 + $6,000 = $16,000.
    let text_2026 = render::render_report(&state, Some(2026));
    assert!(
        text_2026.contains("16000.00"),
        "year=2026 charitable total must be $16,000.00; got:\n{text_2026}"
    );
    // The prior-year $8k must NOT be included: $8k+$10k+$6k = $24k must not appear.
    assert!(
        !text_2026.contains("24000.00"),
        "year=2026 total must NOT include 2025 donation (prior-year excluded); got:\n{text_2026}"
    );
    // [R0-I2] Label carries the §170(b) AGI limit qualifier.
    assert!(
        text_2026.contains("BEFORE §170(b) AGI limits / carryover"),
        "total line must say 'BEFORE §170(b) AGI limits / carryover'; got:\n{text_2026}"
    );
    assert!(
        text_2026.contains("Schedule A itemized"),
        "total line must say 'Schedule A itemized'; got:\n{text_2026}"
    );

    // Year 2025: only the $8k donation.
    let text_2025 = render::render_report(&state, Some(2025));
    assert!(
        text_2025.contains("8000.00"),
        "year=2025 charitable total must be $8,000.00; got:\n{text_2025}"
    );
}

/// (P2-A-T2-b) render_report donation header shows [claimed deduction $X].
/// Gifts show no claimed-deduction annotation.
#[test]
fn render_report_donation_header_shows_claimed_deduction() {
    use btctax_core::{EventId, LedgerState, Removal, RemovalKind, Source, SourceRef};
    use rust_decimal_macros::dec;
    use time::macros::date;

    let mut state = LedgerState::default();
    // Donation with claimed_deduction = $10k.
    state.removals.push(Removal {
        event: EventId::import(Source::Coinbase, SourceRef::new("DON")),
        kind: RemovalKind::Donation,
        removed_at: date!(2026 - 03 - 01),
        legs: vec![],
        appraisal_required: false,
        donor_acquired_at: None,
        claimed_deduction: Some(dec!(10000.00)),
    });
    // Gift with claimed_deduction = None.
    state.removals.push(Removal {
        event: EventId::import(Source::Coinbase, SourceRef::new("GIFT")),
        kind: RemovalKind::Gift,
        removed_at: date!(2026 - 04 - 01),
        legs: vec![],
        appraisal_required: false,
        donor_acquired_at: None,
        claimed_deduction: None,
    });

    let text = render::render_report(&state, None);
    // Donation header must carry the claimed-deduction annotation.
    assert!(
        text.contains("claimed deduction"),
        "donation header must show 'claimed deduction'; got:\n{text}"
    );
    assert!(
        text.contains("10000.00"),
        "donation header must show the deduction amount $10,000.00; got:\n{text}"
    );
    // Gift header must NOT carry a claimed-deduction annotation.
    // We verify the gift line does NOT contain "claimed deduction".
    // Since the donation line also has it, we check per-line.
    let gift_line = text
        .lines()
        .find(|l| l.contains("gift"))
        .expect("gift line must appear in report");
    assert!(
        !gift_line.contains("claimed deduction"),
        "gift header must NOT show 'claimed deduction'; got: {gift_line}"
    );
}

/// (P2-A-T2-c) CSV removals.csv has `claimed_deduction` column.
/// Donation row: non-empty (the §170(e) deduction amount). Gift row: empty string.
/// Uses a full vault with an LT donation reclassification.
#[test]
fn csv_removals_has_claimed_deduction_column() {
    use csv::Reader;
    use std::fs::File;

    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    // CSV: Buy 0.10 BTC 2025-01-05 (LT by 2026), Send A 2026-03-01 (→ Donate LT $10k),
    //      Send B 2026-04-01 (→ GiftOut LT).
    // Note: only 0.05 BTC for donation + 0.05 BTC for gift = 0.10 BTC total.
    let p = dir.path().join("cb_don_gift.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,\
Recipient Address\r\n\
csv-buy,2025-01-05 12:00:00 UTC,Buy,BTC,0.10000000,USD,50000.00,5000.00,5050.00,50.00,,,\r\n\
csv-send-a,2026-03-01 12:00:00 UTC,Send,BTC,0.05000000,USD,60000.00,,,,,,bc1qdonation\r\n\
csv-send-b,2026-04-01 12:00:00 UTC,Send,BTC,0.05000000,USD,62000.00,,,,,,bc1qgiftout\r\n",
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();

    // Identify the two pending TransferOuts and reclassify them.
    let (ref_a, ref_b) = {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert_eq!(
            state.pending_reconciliation.len(),
            2,
            "two Sends must be pending"
        );
        let refs: Vec<_> = state
            .pending_reconciliation
            .iter()
            .map(|p| p.event.canonical())
            .collect();
        // The CSV sends are ordered by timestamp: send-a first, send-b second.
        (refs[0].clone(), refs[1].clone())
    };
    // Send A → Donate, FMV = $10,000. (LT: 2025-01-05 → 2026-03-01; >1yr → LT; basis ≈ $2525 pro-rata)
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &ref_a,
        OutflowClass::Donate {
            appraisal_required: false,
        },
        btctax_cli::eventref::parse_usd_arg("10000.00").unwrap(),
        None,
        now(),
    )
    .unwrap();
    // Send B → GiftOut, FMV = $12,000.
    cmd::reconcile::reclassify_outflow(
        &vault,
        &pp(),
        &ref_b,
        OutflowClass::GiftOut,
        btctax_cli::eventref::parse_usd_arg("12000.00").unwrap(),
        None,
        now(),
    )
    .unwrap();

    // Export.
    let out = dir.path().join("export_p2a");
    let session = Session::open(&vault, &pp()).unwrap();
    let (state, _) = session.project().unwrap();
    btctax_cli::render::write_csv_exports(&out, &state).unwrap();

    // Read removals.csv and check the `claimed_deduction` column.
    let mut rdr = Reader::from_reader(File::open(out.join("removals.csv")).unwrap());
    // Header check: `claimed_deduction` must be the 9th column (index 8, 0-based).
    let headers: Vec<String> = rdr
        .headers()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(
        headers.contains(&"claimed_deduction".to_string()),
        "removals.csv must have 'claimed_deduction' header; headers: {headers:?}"
    );
    let cd_idx = headers
        .iter()
        .position(|h| h == "claimed_deduction")
        .unwrap();

    let records: Vec<csv::StringRecord> = rdr
        .records()
        .collect::<Result<Vec<_>, _>>()
        .expect("removals.csv records must be readable");
    assert!(
        !records.is_empty(),
        "removals.csv must have at least one data row"
    );

    // Donation row: claimed_deduction must be non-empty (the LT FMV = $10,000).
    let donation_rows: Vec<_> = records
        .iter()
        .filter(|r| r.get(1) == Some("donation"))
        .collect();
    assert!(
        !donation_rows.is_empty(),
        "must have at least one donation row"
    );
    for row in &donation_rows {
        let cd = row
            .get(cd_idx)
            .expect("claimed_deduction column must exist");
        assert!(
            !cd.is_empty(),
            "donation row claimed_deduction must be non-empty; row: {row:?}"
        );
        // LT donation: claimed_deduction = FMV = $10,000 (the full total for this lot).
        assert!(
            cd.contains("10000"),
            "LT donation claimed_deduction must contain '10000'; got: {cd}"
        );
    }

    // Gift row: claimed_deduction must be empty.
    let gift_rows: Vec<_> = records
        .iter()
        .filter(|r| r.get(1) == Some("gift"))
        .collect();
    assert!(!gift_rows.is_empty(), "must have at least one gift row");
    for row in &gift_rows {
        let cd = row
            .get(cd_idx)
            .expect("claimed_deduction column must exist");
        assert!(
            cd.is_empty(),
            "gift row claimed_deduction must be empty; got: {cd}"
        );
    }
}
