//! Conservative-filing Task 6 — the friendly RECORD-TIME mutual-exclusion refusal between a
//! `DeclareTranche` and an in-force `SafeHarborAllocation` (D-8 UX layer). The engine backstop
//! (Task 5, `SafeHarborUnconservable` over a live tranche residue) is the GUARANTEE; this is the
//! early, friendly error surfaced at the append sites.
//!
//! Scoping (tax r1 I-2): only a PRE-2025 tranche (`window_end < TRANSITION_DATE`) collides with the
//! pre-2025 Universal residue an allocation reconstructs. A `window_end ≥ 2025` tranche folds straight
//! into a per-wallet post-transition pool and records CLEANLY even beside an effective allocation
//! (else P7's mandatory disclosure for the mixed-records filer is permanently foreclosed).
//!
//! PRIVACY: synthetic Coinbase fixtures in tempdirs; no user file is read.
use btctax_cli::{cmd, CliError, Session};
use btctax_core::{EventPayload, LotMethod, WalletId};
use btctax_store::Passphrase;
use std::path::Path;
use time::macros::date;

fn pp() -> Passphrase {
    Passphrase::new("pw".into())
}
fn now() -> time::OffsetDateTime {
    time::macros::datetime!(2026 - 01 - 01 0:00 UTC)
}

const HEADER: &str = "\r\nTransactions\r\nUser,x\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n";

/// The tranche is declared in its own self-custody-ish wallet; the guard keys on EXISTENCE, not on
/// wallet matching, so it need not equal the imported Coinbase lot's wallet.
fn tranche_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "cold".into(),
        account: "vault".into(),
    }
}

fn count<P: Fn(&EventPayload) -> bool>(vault: &Path, pred: P) -> usize {
    let s = Session::open(vault, &pp()).unwrap();
    btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .iter()
        .filter(|e| pred(&e.payload))
        .count()
}

/// A vault with a single pre-2025 documented buy (0.20 BTC) — the allocatable residue, no allocation
/// and no tranche yet.
fn vault_pre2025_buy(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let p = dir.join("cb.csv");
    std::fs::write(
        &p,
        format!(
            "{HEADER}cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    vault
}

/// A vault with an EFFECTIVE allocation: a pre-2025 buy with NO 2025 disposition makes an unattested
/// `ActualPosition` allocation already effective (Path B). Recipe from
/// `reconcile::bulk_void_plan_omits_effective_allocation`.
fn vault_effective_alloc(dir: &Path) -> std::path::PathBuf {
    let vault = vault_pre2025_buy(dir);
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();
    cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        btctax_core::AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();
    vault
}

/// A vault with an INERT allocation: a pre-2025 buy + a 2025 Sell time-bars an unattested
/// `ActualPosition` allocation (§5.02(4)) → inert but NON-VOIDED. Needed so Step-5(b)'s effective-only
/// mutation can go RED (arch r2 New-3: inert allocations are still in-force for this guard).
fn vault_inert_alloc(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    let p = dir.join("cb.csv");
    std::fs::write(
        &p,
        format!(
            "{HEADER}cb-pre,2024-01-15 12:00:00 UTC,Buy,BTC,0.20000000,USD,42500.00,8500.00,8550.00,50.00,,,\r\n\
cb-sell,2025-06-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,90000.00,4500.00,4490.00,10.00,,,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[p]).unwrap();
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();
    let alloc = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        btctax_core::AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap();
    // Sanity: the allocation is INERT (a SafeHarborTimebar blocker sits on its id).
    {
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _) = s.project().unwrap();
        assert!(
            state
                .blockers
                .iter()
                .any(|b| b.event.as_ref() == Some(&alloc)
                    && b.kind == btctax_core::BlockerKind::SafeHarborTimebar),
            "fixture must produce an INERT (time-barred) allocation"
        );
    }
    vault
}

// ── (a) pre-2025 tranche refused under an EFFECTIVE allocation ──────────────────────────────────

#[test]
fn pre2025_tranche_refused_under_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_effective_alloc(dir.path());

    let err = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
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
        err.to_string().to_lowercase().contains("safe-harbor")
            || err.to_string().to_lowercase().contains("allocation"),
        "the refusal must name the allocation collision: {err}"
    );
    // (e) refusal appends NO event.
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::DeclareTranche(_))),
        0,
        "the refused tranche must NOT be appended (fail-closed)"
    );
}

// ── (a2) pre-2025 tranche refused under an INERT allocation (kills the effective-only mutation) ──

#[test]
fn pre2025_tranche_refused_under_inert_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_inert_alloc(dir.path());

    let err = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)),
        "an INERT (non-voided) allocation is still in-force for this guard: {err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::DeclareTranche(_))),
        0,
    );
}

// ── (c) allocation refused under a pre-2025 tranche ──────────────────────────────────────────────

#[test]
fn allocation_refused_under_a_pre2025_tranche() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_pre2025_buy(dir.path());

    // Record a pre-2025 tranche first (no allocation exists → accepted).
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
        now(),
    )
    .unwrap();
    cmd::admin::set_pre2025_method(&vault, &pp(), LotMethod::Fifo, true).unwrap();

    let err = cmd::reconcile::safe_harbor_allocate(
        &vault,
        &pp(),
        btctax_core::AllocMethod::ActualPosition,
        false,
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)) && err.to_string().to_lowercase().contains("tranche"),
        "the refusal must name the tranche collision: {err}"
    );
    // (e) refusal appends NO event.
    assert_eq!(
        count(&vault, |p| matches!(
            p,
            EventPayload::SafeHarborAllocation(_)
        )),
        0,
        "the refused allocation must NOT be appended (fail-closed)"
    );
}

// ── (d) a ≥2025-window tranche records CLEANLY beside an effective allocation (foreclosure guard) ─

#[test]
fn post2025_tranche_records_cleanly_beside_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_effective_alloc(dir.path());

    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2025 - 06 - 01),
        date!(2025 - 12 - 31),
        now(),
    )
    .expect(
        "a ≥2025 tranche must record cleanly beside an effective allocation (P7 not foreclosed)",
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::DeclareTranche(_))),
        1,
        "the ≥2025 tranche must be appended",
    );
}

// ── (f) safe_harbor_residue omits tranche sats as allocatable ────────────────────────────────────

#[test]
fn safe_harbor_residue_omits_tranche_sats() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_pre2025_buy(dir.path());

    // A pre-2025 tranche (0.50 BTC) recorded beside the documented 0.20-BTC buy.
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (lots, _method) = s.safe_harbor_residue().unwrap();
    let total_sat: i64 = lots.iter().map(|l| l.sat).sum();
    assert_eq!(
        total_sat, 20_000_000,
        "the allocatable residue must be the documented buy only (0.20 BTC); the $0 tranche is NOT \
         allocatable pre-2025 residue"
    );
    assert!(
        lots.iter().all(|l| l.usd_basis > btctax_core::Usd::ZERO),
        "no $0 tranche lot may appear in the allocatable residue"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Task 7 — the `declare-tranche` verb record path: input validation + clean (non-pseudo) export (D-5)
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A bare vault (no imports) — the tranche record path opens/appends on its own.
fn empty_vault(dir: &Path) -> std::path::PathBuf {
    let vault = dir.join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.join("k.asc")).unwrap();
    vault
}

/// (Task 7 a) the verb's record path appends a tranche that folds to the D-1 lot: $0 basis,
/// `EstimatedConservative`, homed at `window_end`, in the declared wallet, NOT pseudo.
#[test]
fn declare_tranche_records_and_folds_to_zero_basis_lot() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());

    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2020 - 01 - 01),
        date!(2020 - 12 - 31),
        now(),
    )
    .unwrap();

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    let lot = state
        .lots
        .iter()
        .find(|l| l.wallet == tranche_wallet())
        .expect("a tranche lot in the declared wallet");
    assert_eq!(
        lot.usd_basis,
        btctax_core::Usd::ZERO,
        "tranche basis is $0 (D-7)"
    );
    assert_eq!(
        lot.basis_source,
        btctax_core::BasisSource::EstimatedConservative
    );
    assert_eq!(
        lot.acquired_at,
        date!(2020 - 12 - 31),
        "homed at window_end (D-2)"
    );
    assert!(!lot.pseudo, "a filed tranche is NOT pseudo (D-5)");
    assert!(
        !state.pseudo_active(),
        "a real tranche never activates pseudo mode (D-5)"
    );
}

/// (Task 7 c) a `sat <= 0` tranche is REFUSED at record time — a non-positive sat would bump
/// `stats.sigma_in` by a non-positive amount (`fold.rs`), corrupting Σ-conservation. No event appended.
#[test]
fn declare_tranche_refuses_nonpositive_sat() {
    for bad in [0_i64, -1] {
        let dir = tempfile::tempdir().unwrap();
        let vault = empty_vault(dir.path());
        let err = cmd::tranche::declare_tranche(
            &vault,
            &pp(),
            bad,
            tranche_wallet(),
            date!(2020 - 01 - 01),
            date!(2020 - 12 - 31),
            now(),
        )
        .unwrap_err();
        assert!(
            matches!(err, CliError::Usage(_)) && err.to_string().contains("> 0"),
            "sat {bad} must be refused: {err}"
        );
        assert_eq!(
            count(&vault, |p| matches!(p, EventPayload::DeclareTranche(_))),
            0,
            "the refused tranche must NOT be appended (fail-closed)"
        );
    }
}

/// (Task 7 d) `window_start > window_end` is REFUSED (an undefined P5/P7 window). No event appended.
#[test]
fn declare_tranche_refuses_inverted_window() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    let err = cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2020 - 12 - 31),
        date!(2020 - 01 - 01),
        now(),
    )
    .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)) && err.to_string().to_lowercase().contains("window"),
        "an inverted window must be refused: {err}"
    );
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::DeclareTranche(_))),
        0,
    );
}

/// (Task 7 warn-not-refuse) a FUTURE `window_end` is ACCEPTED (it merely strands the lot; conservative
/// but confusing). The verb warns on stderr; the record path itself does NOT refuse.
#[test]
fn declare_tranche_accepts_future_window_end() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    // `now()` is 2026-01-01; a 2027 window_end is in the future.
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2027 - 01 - 01),
        date!(2027 - 12 - 31),
        now(),
    )
    .expect("a future window_end warns but does not refuse");
    assert_eq!(
        count(&vault, |p| matches!(p, EventPayload::DeclareTranche(_))),
        1,
    );
}

/// (Task 7 b) a year with a filed tranche exports CLEAN: `export_snapshot` with NO attestation returns
/// Ok (no `AttestationRequired`), because a real tranche never activates pseudo mode (D-5).
#[test]
fn filed_tranche_year_exports_clean() {
    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2020 - 01 - 01),
        date!(2020 - 12 - 31),
        now(),
    )
    .unwrap();

    let out = dir.path().join("export_out");
    cmd::admin::export_snapshot(&vault, &pp(), &out, Some(2020), None)
        .expect("a filed-tranche year must export clean with NO attestation (not pseudo, D-5)");
}
