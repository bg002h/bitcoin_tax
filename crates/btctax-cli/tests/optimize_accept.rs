//! CLI integration tests for `optimize accept` (Task 10 / §C.2 gated persistence + revocation).
//!
//! This is the COMPLIANCE-CRITICAL write path — the only Mode-1 path that persists. The §1.1012-1(j)
//! boundary is load-bearing here:
//!   * a genuinely-contemporaneous pick (made ≤ sale) persists freely (→ Contemporaneous);
//!   * an already-executed disposal persists ONLY behind a NARROW per-disposal `--attest` scoped to a
//!     single `--disposal` (→ AttestedRecording); the app NEVER auto-attests a post-hoc selection;
//!   * a 2027+ broker-held pick is CATEGORICALLY refused (own-books insufficient; no attestation cures);
//!   * the proposed `LotSelection` decision + the attestation side-table row are co-persisted ATOMICALLY
//!     (one `session.save()`), so persisted == attested == new baseline (R2-I1 holds on a later re-run);
//!   * revocation reuses `reconcile void` on the returned decision id.
//!
//! PRIVACY: all fixtures are synthetic and written into tempdirs; the suite NEVER reads
//! `~/Documents/BitcoinTax/ReadOnly` or any user file. Exact Decimal/i64 only (NFR5); deterministic
//! `now` injected (NFR4).
use btctax_cli::{cmd, render};
use btctax_core::persistence::load_all;
use btctax_core::{
    Carryforward, ComplianceStatus, EventId, EventPayload, FilingStatus, LotPick, LtcgBreakpoints,
    OrdinaryBracket, OrdinarySchedule, TaxProfile, TaxTable, TaxTables,
};
use btctax_store::Passphrase;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use time::{macros::datetime, OffsetDateTime};

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}

/// Single filer profile for any TY: OTI=100k, MAGI=100k, QD=0 (solidly 24% ordinary in 2025).
fn single_100k_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(100000),
        magi_excluding_crypto: dec!(100000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
    }
}

/// Initialize vault + import one CSV; return `(tempdir, vault_path)`.
fn make_vault_with(csv: &Path) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();
    cmd::import::run(&vault, &pp(), &[csv.to_path_buf()]).unwrap();
    (dir, vault)
}

/// Count events in the vault.
fn event_count(vault: &Path) -> usize {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    load_all(s.conn()).unwrap().len()
}

/// Read back the `LotSelection.lots` of a specific decision event (the persisted pick).
fn lotselection_lots(vault: &Path, decision: &EventId) -> Vec<LotPick> {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    let events = load_all(s.conn()).unwrap();
    let e = events
        .iter()
        .find(|e| &e.id == decision)
        .expect("decision event must be present");
    match &e.payload {
        EventPayload::LotSelection(ls) => ls.lots.clone(),
        other => panic!("expected a LotSelection decision, got {other:?}"),
    }
}

/// The stored attestation text for a disposal, or `None`.
fn attestation_of(vault: &Path, disposal: &EventId) -> Option<String> {
    let s = btctax_cli::Session::open(vault, &pp()).unwrap();
    btctax_cli::optimize_attest::get(s.conn(), disposal).unwrap()
}

// ── Injectable synthetic tax tables (for the 2027-broker end-to-end refusal) ────────────────────────
// The bundled tables cover TY2025 only, so a 2027 disposal is `YearNotComputable` under the public
// `accept`. `accept_with_tables` lets the test inject a later year's table to drive the disposal to a
// real `ForbiddenBroker2027` verdict and assert the categorical refusal end-to-end.
struct OneTable(TaxTable);
impl TaxTables for OneTable {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        (year == self.0.year).then_some(&self.0)
    }
}

fn synth_tables(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.10),
                },
                OrdinaryBracket {
                    lower: dec!(50000),
                    rate: dec!(0.22),
                },
                OrdinaryBracket {
                    lower: dec!(90000),
                    rate: dec!(0.32),
                },
            ],
        },
    );
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(40000),
            max_fifteen: dec!(400000),
        },
    );
    OneTable(TaxTable {
        year,
        source: "SYNTHETIC",
        ordinary,
        ltcg,
    })
}

// ── CSV fixtures ────────────────────────────────────────────────────────────────────────────────

/// Coinbase: pre-2025 low-basis LT lot + 2025 high-basis ST lot + a 2025 sell.
///   Lot A: 1 BTC @ $30,000 (2023-01-01) — LT by 2025-06-01.
///   Lot B: 1 BTC @ $80,000 (2025-01-02) — ST on 2025-06-01.
///   Sell:  1 BTC @ $50,000 (2025-06-01).
/// FIFO baseline → Lot A (+$20k LT gain). Optimizer HIFO → Lot B (−$30k loss). proposed != current.
fn write_tax_saving_csv(dir: &Path) -> PathBuf {
    let p = dir.join("opt_saving.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
opt-buy-lt,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
opt-buy-st,2025-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,80000.00,80000.00,80000.00,0.00,,,\r\n\
opt-sell,2025-06-01 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

/// Coinbase (Exchange = broker): two lots + a 2027 sell (so sale year ≥ 2027).
///   Lot A: 1 BTC @ $20,000 (2025-01-02). Lot B: 1 BTC @ $90,000 (2026-01-02).
///   Sell:  1 BTC @ $50,000 (2027-06-01).
/// FIFO → Lot A (+$30k gain). Optimizer → Lot B (−$40k loss). proposed != current → reaches the gate,
/// where a 2027+ broker pick is ForbiddenBroker2027.
fn write_broker_2027_csv(dir: &Path) -> PathBuf {
    let p = dir.join("opt_broker27.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,\
Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
b27-buy-a,2025-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,20000.00,20000.00,20000.00,0.00,,,\r\n\
b27-buy-b,2026-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,90000.00,90000.00,90000.00,0.00,,,\r\n\
b27-sell,2027-06-01 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n",
    )
    .unwrap();
    p
}

// The 2025 self-custody/broker sale is on 2025-06-01; these injected `now`s straddle it.
const BEFORE_SALE: OffsetDateTime = datetime!(2025-01-01 12:00:00 UTC); // made ≤ sale → Contemporaneous
const AFTER_SALE: OffsetDateTime = datetime!(2026-01-01 12:00:00 UTC); // made > sale → already-executed

// ── accept-mutates (Contemporaneous; no --attest) ─────────────────────────────────────────────────

/// made ≤ sale → `ContemporaneousNow`: a bare `accept` persists the optimum's `LotSelection`
/// (event +1), writes NO attestation, the persisted picks == the displayed proposal (determinism),
/// and a re-run reports the disposal `Contemporaneous`.
#[test]
fn accept_persists_contemporaneous_without_attestation() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    // run (read-only) captures the displayed proposal for the determinism comparison.
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, BEFORE_SALE).unwrap();
    let disposal = proposal.per_disposal[0].disposal.clone();
    let displayed = proposal.per_disposal[0].proposed_selection.clone();
    assert_ne!(
        displayed, proposal.per_disposal[0].current_selection,
        "fixture must offer an improving pick (proposed != current)"
    );

    let before = event_count(&vault);
    let out = cmd::optimize::accept(&vault, &pp(), 2025, None, None, BEFORE_SALE).unwrap();

    assert_eq!(
        out.persisted.len(),
        1,
        "the contemporaneous pick must persist"
    );
    assert_eq!(out.persisted[0].2, "Contemporaneous");
    assert!(out.skipped.is_empty(), "nothing should be skipped");
    assert_eq!(
        event_count(&vault),
        before + 1,
        "exactly one LotSelection appended"
    );

    // Determinism: the persisted picks == the run-displayed proposed selection.
    let (_, decision, _) = &out.persisted[0];
    assert_eq!(
        lotselection_lots(&vault, decision),
        displayed,
        "accept must persist exactly the selection run displayed (NFR4)"
    );

    // No attestation written for a contemporaneous pick.
    assert_eq!(attestation_of(&vault, &disposal), None);

    // Re-run: persisted selection is now the baseline; the optimum is unchanged → Contemporaneous.
    let re = cmd::optimize::run(&vault, &pp(), 2025, BEFORE_SALE).unwrap();
    let row = &re.per_disposal[0];
    assert_eq!(
        row.proposed_selection, row.current_selection,
        "no-change row"
    );
    assert_eq!(row.status, ComplianceStatus::Contemporaneous);

    // Render summary is informative.
    let rendered = render::render_accept_outcome(&out);
    assert!(rendered.contains("PERSISTED"));
    assert!(rendered.contains("Contemporaneous"));
}

// ── accept refuses without attestation (already-executed) ──────────────────────────────────────────

/// made > sale (post-hoc) → `NeedsAttestation`: a bare `accept` (no `--attest`) persists NOTHING for
/// it, the skip reason mentions `--attest`, the event count is unchanged, and no attestation is written.
#[test]
fn accept_refuses_already_executed_without_attestation() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let proposal = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    let disposal = proposal.per_disposal[0].disposal.clone();

    let before = event_count(&vault);
    let out = cmd::optimize::accept(&vault, &pp(), 2025, None, None, AFTER_SALE).unwrap();

    assert!(
        out.persisted.is_empty(),
        "post-hoc pick must NOT persist without --attest"
    );
    assert_eq!(out.skipped.len(), 1);
    assert!(
        out.skipped[0].1.contains("--attest"),
        "skip reason must direct the user to --attest: {}",
        out.skipped[0].1
    );
    assert_eq!(event_count(&vault), before, "no event appended");
    assert_eq!(
        attestation_of(&vault, &disposal),
        None,
        "no attestation row"
    );
}

// ── attested → AttestedRecording (R2-I1 positive end-to-end + atomic co-persist) ───────────────────

/// The same already-executed disposal with `--disposal <ref> --attest "..."` co-persists the
/// `LotSelection` AND an attestation row (event +1 AND attestation present — atomicity). A re-run's
/// proposed pick then equals the now-persisted (current) selection, so the disposal is in `unchanged`
/// and the overlay reports `AttestedRecording`.
#[test]
fn accept_attested_persists_and_upgrades_to_attested_recording() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let proposal = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    let disposal = proposal.per_disposal[0].disposal.clone();
    let displayed = proposal.per_disposal[0].proposed_selection.clone();

    let before = event_count(&vault);
    let attest = "I identified these units at the time of sale in my books";
    let out = cmd::optimize::accept(
        &vault,
        &pp(),
        2025,
        Some(&disposal.canonical()),
        Some(attest),
        AFTER_SALE,
    )
    .unwrap();

    assert_eq!(out.persisted.len(), 1);
    assert_eq!(out.persisted[0].2, "AttestedRecording");

    // Atomic co-persist: the LotSelection event AND the attestation row are BOTH present.
    assert_eq!(event_count(&vault), before + 1, "LotSelection appended");
    let (_, decision, _) = &out.persisted[0];
    assert_eq!(lotselection_lots(&vault, decision), displayed);
    assert_eq!(
        attestation_of(&vault, &disposal).as_deref(),
        Some(attest),
        "attestation must round-trip"
    );

    // R2-I1 positive: re-run → proposed == current (D ∈ unchanged) → AttestedRecording.
    let re = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    let row = &re.per_disposal[0];
    assert_eq!(
        row.proposed_selection, row.current_selection,
        "R2-I1: the upgrade requires D ∈ unchanged (proposed == persisted current)"
    );
    assert_eq!(
        row.status,
        ComplianceStatus::AttestedRecording,
        "attested ∧ unchanged ∧ self-envelope → AttestedRecording"
    );
}

// ── R2-I1 divergent end-to-end: a divergent later baseline stays NonCompliant ──────────────────────

/// After accept+attest persists pick P1 (the optimum) for D, `void` the persisted decision. The
/// attestation side-table row SURVIVES the void (void touches only the ledger), but the in-force
/// baseline reverts to FIFO — so a re-run proposes P1 while current is the FIFO lot → D ∉ unchanged →
/// the (now inert) attestation does NOT launder it → stays NonCompliant. This is the R2-I1 boundary
/// end-to-end: an attestation binds ONLY the exact persisted-and-in-force selection, never a divergent
/// baseline.
#[test]
fn accept_then_divergent_baseline_stays_noncompliant() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    // Capture the FIFO baseline pick (Lot A) and the disposal id.
    let proposal = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    let disposal = proposal.per_disposal[0].disposal.clone();
    let fifo_pick = proposal.per_disposal[0].current_selection.clone(); // Lot A
    let opt_pick = proposal.per_disposal[0].proposed_selection.clone(); // Lot B
    assert_ne!(fifo_pick, opt_pick);

    // accept+attest persists P1 = the optimum (Lot B) + attestation.
    let out = cmd::optimize::accept(
        &vault,
        &pp(),
        2025,
        Some(&disposal.canonical()),
        Some("attested P1"),
        AFTER_SALE,
    )
    .unwrap();
    let (_, decision, _) = out.persisted[0].clone();

    // Sanity: the persisted selection is now in force AND it is AttestedRecording.
    let in_force = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    assert_eq!(in_force.per_disposal[0].current_selection, opt_pick);
    assert_eq!(
        in_force.per_disposal[0].status,
        ComplianceStatus::AttestedRecording
    );

    // Void the persisted decision — the attestation row survives but the selection is revoked.
    cmd::reconcile::void(&vault, &pp(), &decision.canonical(), AFTER_SALE).unwrap();
    assert_eq!(
        attestation_of(&vault, &disposal).as_deref(),
        Some("attested P1"),
        "void does not clear the attestation side-table row (it becomes inert)"
    );

    // Re-run: current == FIFO (Lot A), proposed == Lot B (the optimum) → divergent → D ∉ unchanged.
    let re = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    let row = &re.per_disposal[0];
    assert_eq!(
        row.current_selection, fifo_pick,
        "baseline reverts to FIFO after void"
    );
    assert_ne!(
        row.proposed_selection, row.current_selection,
        "proposed (P1) diverges from current (FIFO) → D ∉ unchanged"
    );
    assert_eq!(
        row.status,
        ComplianceStatus::NonCompliant,
        "R2-I1: a lingering attestation must NOT launder a divergent/reverted baseline"
    );
}

// ── refuse 2027+ broker-held (categorical; even with --attest) ─────────────────────────────────────

/// A 2027 `Exchange` (broker) disposal, post-hoc → `ForbiddenBroker2027`: NEVER persisted, even with
/// `--attest`; the skip reason cites the 2027+ broker rule; no event appended; no attestation written.
#[test]
fn accept_refuses_2027_broker_held_even_with_attestation() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_broker_2027_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2027, single_100k_profile()).unwrap();
    let tables = synth_tables(2027);
    let now_post_hoc: OffsetDateTime = datetime!(2028-01-01 12:00:00 UTC);

    let before = event_count(&vault);

    // First, a plain accept (no --attest): the 2027 broker disposal is skipped with the broker rule.
    let out =
        cmd::optimize::accept_with_tables(&vault, &pp(), 2027, None, None, now_post_hoc, &tables)
            .unwrap();
    assert!(
        out.persisted.is_empty(),
        "2027 broker pick must never persist"
    );
    assert_eq!(out.skipped.len(), 1);
    let (disposal, reason) = out.skipped[0].clone();
    assert!(
        reason.contains("2027+ broker-held") && reason.contains("own-books"),
        "skip reason must cite the 2027+ broker rule: {reason}"
    );
    assert_eq!(event_count(&vault), before, "no event appended");

    // Now WITH --attest scoped to that disposal: still categorically refused (no attestation can cure).
    let out2 = cmd::optimize::accept_with_tables(
        &vault,
        &pp(),
        2027,
        Some(&disposal.canonical()),
        Some("I tried to attest a 2027 broker pick"),
        now_post_hoc,
        &tables,
    )
    .unwrap();
    assert!(
        out2.persisted.is_empty(),
        "even --attest must not persist a 2027 broker pick"
    );
    assert_eq!(out2.skipped.len(), 1);
    assert!(out2.skipped[0].1.contains("2027+ broker-held"));
    assert_eq!(
        event_count(&vault),
        before,
        "no event appended even with --attest"
    );
    assert_eq!(
        attestation_of(&vault, &disposal),
        None,
        "no attestation row written for a refused 2027 broker pick"
    );
}

// ── blanket-attest guard (R2-M5/R0-M5 — fires BEFORE any append) ──────────────────────────────────

/// `accept --attest "..."` WITHOUT `--disposal` → `Err(Usage(... "no blanket attestation" ...))`, and
/// the event count is UNCHANGED. The guard is hoisted ABOVE the loop, so even a `ContemporaneousNow`
/// disposal (here `now` is before the sale) is NOT appended before the guard fires — no partial writes.
#[test]
fn accept_blanket_attest_is_rejected_before_any_write() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let proposal = cmd::optimize::run(&vault, &pp(), 2025, BEFORE_SALE).unwrap();
    let disposal = proposal.per_disposal[0].disposal.clone();

    let before = event_count(&vault);
    // BEFORE_SALE ⇒ the disposal is ContemporaneousNow: absent the hoisted guard it WOULD be appended.
    let err =
        cmd::optimize::accept(&vault, &pp(), 2025, None, Some("blanket"), BEFORE_SALE).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("no blanket attestation"),
        "blanket attest must be rejected: {msg}"
    );
    assert_eq!(
        event_count(&vault),
        before,
        "guard above the loop ⇒ NO append before it fires (no partial writes)"
    );
    assert_eq!(attestation_of(&vault, &disposal), None);
}

// ── void revokes ──────────────────────────────────────────────────────────────────────────────────

/// After persisting a Contemporaneous selection, `reconcile void` on the returned decision id revokes
/// it: the disposal no longer reports the persisted selection — its baseline returns to FIFO.
#[test]
fn void_revokes_a_persisted_selection() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    // Capture the FIFO baseline (Lot A) for the post-void comparison.
    let pre = cmd::optimize::run(&vault, &pp(), 2025, BEFORE_SALE).unwrap();
    let fifo_pick = pre.per_disposal[0].current_selection.clone();
    let opt_pick = pre.per_disposal[0].proposed_selection.clone();

    let out = cmd::optimize::accept(&vault, &pp(), 2025, None, None, BEFORE_SALE).unwrap();
    let (_, decision, _) = out.persisted[0].clone();

    // After accept: baseline follows the persisted optimum (Lot B); no-change row.
    let after_accept = cmd::optimize::run(&vault, &pp(), 2025, BEFORE_SALE).unwrap();
    assert_eq!(after_accept.per_disposal[0].current_selection, opt_pick);

    // Void the persisted decision.
    cmd::reconcile::void(&vault, &pp(), &decision.canonical(), BEFORE_SALE).unwrap();

    // After void: baseline returns to FIFO (Lot A) — the selection is revoked.
    let after_void = cmd::optimize::run(&vault, &pp(), 2025, BEFORE_SALE).unwrap();
    assert_eq!(
        after_void.per_disposal[0].current_selection, fifo_pick,
        "void must restore the prior (FIFO) baseline"
    );
    assert_ne!(
        after_void.per_disposal[0].current_selection,
        after_accept.per_disposal[0].current_selection,
        "the persisted selection is no longer in force"
    );
}

// ── determinism: recompute matches; only the targeted disposal is touched ──────────────────────────

/// `accept` recomputes the SAME deterministic optimum as `run` (two identical runs agree), and a
/// `--disposal` scope touches ONLY that disposal.
#[test]
fn accept_recompute_is_deterministic_and_disposal_scoped() {
    let csv_dir = tempfile::tempdir().unwrap();
    let csv = write_tax_saving_csv(csv_dir.path());
    let (_dir, vault) = make_vault_with(&csv);
    cmd::tax::set_profile(&vault, &pp(), 2025, single_100k_profile()).unwrap();

    let a = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    let b = cmd::optimize::run(&vault, &pp(), 2025, AFTER_SALE).unwrap();
    assert_eq!(
        a.per_disposal[0].proposed_selection, b.per_disposal[0].proposed_selection,
        "two recomputes must be byte-identical (NFR4)"
    );

    // A --disposal scope that matches no disposal persists nothing and skips nothing.
    let before = event_count(&vault);
    let bogus = EventId::decision(99_999);
    let out = cmd::optimize::accept(
        &vault,
        &pp(),
        2025,
        Some(&bogus.canonical()),
        None,
        AFTER_SALE,
    )
    .unwrap();
    assert!(
        out.persisted.is_empty() && out.skipped.is_empty(),
        "no disposal matched the scope"
    );
    assert_eq!(
        event_count(&vault),
        before,
        "an unmatched scope appends nothing"
    );
}
