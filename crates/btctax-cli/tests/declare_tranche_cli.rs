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
use btctax_core::conservative::Coverage;
use btctax_core::{
    Acknowledgment, DeclareTranche, EventId, EventPayload, FloorMethod, LedgerEvent, LotMethod,
    PromoteTranche, WalletId,
};
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

    // (tax review r1 Minor) Non-poisoning: a ≥2025 tranche folds into a post-transition per-wallet pool,
    // NOT the pre-2025 Universal residue, so it does NOT deny the allocation effectiveness. Assert the
    // allocation stays EFFECTIVE (no SafeHarborUnconservable/Timebar blocker) AND the tranche coexists.
    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    assert!(
        !state.blockers.iter().any(|b| matches!(
            b.kind,
            btctax_core::BlockerKind::SafeHarborUnconservable
                | btctax_core::BlockerKind::SafeHarborTimebar
        )),
        "a ≥2025 tranche must NOT poison the effective allocation (Path B preserved): {:?}",
        state.blockers
    );
    // Path B is DIRECTLY pinned (not just blocker-absence): the effective allocation seeds the pre-2025
    // documented lot with `SafeHarborAllocated` — so a regression flipping this filer to Path A (even one
    // that emits no blocker) goes RED here, foreclosing the Rev-Proc-2024-28 flexibility strip.
    assert!(
        state
            .lots
            .iter()
            .any(|l| l.basis_source == btctax_core::BasisSource::SafeHarborAllocated),
        "the pre-2025 residue must be SEEDED under the effective allocation (Path B), not reconstructed"
    );
    assert!(
        state.lots.iter().any(|l| l.basis_source
            == btctax_core::BasisSource::EstimatedConservative
            && l.remaining_sat == 50_000_000),
        "the ≥2025 tranche coexists as its own EstimatedConservative lot"
    );
}

// ── (f) safe_harbor_residue REFUSES the opener when a pre-2025 tranche exists (T16 follow-up) ─────────

#[test]
fn safe_harbor_residue_refuses_when_a_pre2025_tranche_exists() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_pre2025_buy(dir.path());

    // A pre-2025 tranche (0.50 BTC) beside the documented 0.20-BTC buy makes a safe-harbor allocation
    // mutually-exclusive (D-8). The residue opener therefore REFUSES (rather than displaying a residue
    // that a pre-2025 disposal could skew — arch r1 Minor 3 / tax r1 Minor 4). Matches the record-time
    // allocation guard; the TUI opener surfaces this Err as its pre-flight status.
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
    let err = s
        .safe_harbor_residue()
        .expect_err("a pre-2025 tranche must refuse the allocate-flow opener");
    assert!(
        format!("{err}").contains("mutually exclusive"),
        "the refusal names the D-8 mutual exclusion: {err}"
    );
}

// ── (g) a PROMOTED pre-2025 tranche still refuses the allocation guard (approach-B Task 3, BG-D1) ──
//
// NB placement: `guard_allocation_vs_tranche` lives HERE (btctax-cli) — it maps a refusal to a
// `CliError`, a cli-only concern. `pre2025_tranche_exists` itself MOVED to
// `btctax_core::tranche_guard` (Defensive Filing Wizard Task 5, C-2) so core callers can read it
// without a cli→core dependency inversion; the guard calls the core predicate. This KAT still cannot
// literally live beside the other four Task-3 by-construction KATs in
// `crates/btctax-core/tests/kat_promote.rs` as the task brief's file listing suggested, since the
// GUARD (the thing this KAT exercises) is still cli-only. Both functions are PURE over
// `&[LedgerEvent]` (no vault needed).

/// A PromoteTranche decision event promoting `target` to `filed_basis` (mirrors the equivalent
/// `promote_ev` helper in `kat_promote.rs`; duplicated here since the two crates' test binaries can't
/// share private fixture code).
fn promote_ev(seq: u64, target: EventId, filed_basis: rust_decimal::Decimal) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: now(),
        original_tz: time::UtcOffset::UTC,
        wallet: None,
        payload: EventPayload::PromoteTranche(PromoteTranche {
            target,
            method: FloorMethod::WindowLowClose,
            filed_basis,
            coverage: Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I understand and accept the risk".into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
    }
}

/// Task 3 (BG-D1 / arch r2 M-4, tax r2 — verified shared predicate): `pre2025_tranche_exists`
/// (`btctax_core::tranche_guard`, moved from btctax-cli in Defensive Filing Wizard Task 5 / C-2) is
/// keyed on the mere PRESENCE of a non-voided pre-2025 `DeclareTranche` — it never inspects the lot's
/// basis or looks for a `PromoteTranche` decision at all. So a PROMOTED (>$0 filed) tranche still
/// refuses a safe-harbor allocation at record time, exactly like an un-promoted ($0) one.
/// `guard_allocation_vs_tranche` is the ONE chokepoint for all four allocation append sites (CLI
/// `safe_harbor_allocate`/`safe_harbor_attest`, TUI `persist_safe_harbor_allocate`/
/// `persist_safe_harbor_attest`) AND the TUI opener's pre-flight consult (`session.rs:692`, via the
/// same core `pre2025_tranche_exists`) — exercising the shared predicate here covers all of them by
/// construction.
#[test]
fn a_promoted_tranche_still_refuses_a_safe_harbor_allocation_at_record_time() {
    let tranche = LedgerEvent {
        id: EventId::decision(1),
        utc_timestamp: now(),
        original_tz: time::UtcOffset::UTC,
        wallet: None,
        payload: EventPayload::DeclareTranche(DeclareTranche {
            sat: 50_000_000,
            wallet: tranche_wallet(),
            window_start: date!(2018 - 01 - 01),
            window_end: date!(2018 - 12 - 31),
        }),
    };
    let promote = promote_ev(2, EventId::decision(1), rust_decimal::Decimal::from(12_000));
    let events = vec![tranche, promote];
    assert!(
        cmd::tranche::guard_allocation_vs_tranche(&events).is_err(),
        "a promoted pre-2025 tranche still blocks a safe-harbor allocation (D-8, tag-keyed)"
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

/// (review r1 Minor) The CLI `safe_harbor_attest` ATTEST-site guard is exercised: with a pre-2025
/// tranche on file, attest refuses with the TRANCHE message — the guard fires BEFORE the "no allocation
/// to attest" path. Without the guard this returns the "no allocation" error instead → the assertion on
/// the tranche wording is what kills the attest-guard mutation (untested-guard discipline).
#[test]
fn attest_refused_under_a_pre2025_tranche() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_pre2025_buy(dir.path());
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

    let err = cmd::reconcile::safe_harbor_attest(&vault, &pp(), now()).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)) && err.to_string().to_lowercase().contains("tranche"),
        "attest must refuse with the TRANCHE message (guard fires before 'no allocation'): {err}"
    );
}

/// (P3 / Task 9 — surfacing KAT) The tranche dip advisory REACHES `report_tax_year` (the user-visible
/// half): a pre-2025 tranche disposed in 2020 populates `TaxYearReport.tranche_advisory` with the
/// dip text + the basis as filed. Dropping the report-field wiring makes this RED (surfacing mutation).
#[test]
fn tranche_dip_advisory_reaches_the_tax_year_report() {
    use btctax_core::{Carryforward, FilingStatus, TaxProfile};
    use rust_decimal_macros::dec;

    let dir = tempfile::tempdir().unwrap();
    let vault = empty_vault(dir.path());
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        100_000_000,
        tranche_wallet(),
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
        now(),
    )
    .unwrap();
    // A 2020 Coinbase Sell of 0.5 BTC consumes the tranche (pre-2025 Universal pool, HIFO — only lot).
    let csv = dir.path().join("sell.csv");
    std::fs::write(
        &csv,
        format!(
            "{HEADER}cb-sell,2020-06-01 12:00:00 UTC,Sell,BTC,0.50000000,USD,80000.00,40000.00,40000.00,0.00,,,\r\n"
        ),
    )
    .unwrap();
    cmd::import::run(&vault, &pp(), &[csv]).unwrap();
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(40000),
        magi_excluding_crypto: dec!(60000),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    cmd::tax::set_profile(&vault, &pp(), 2020, profile, false).unwrap();

    let report = cmd::tax::report_tax_year(&vault, &pp(), 2020, dec!(0)).unwrap();
    let adv = report
        .tranche_advisory
        .expect("the tranche dip advisory must reach the tax-year report (surfacing)");
    assert!(
        adv.to_lowercase().contains("undocumented"),
        "the dip advisory text must surface: {adv}"
    );
    assert!(
        adv.contains("$0"),
        "basis as filed ($0) must surface: {adv}"
    );
}

/// P8 Nit — `wallet_is_known` recognizes a wallet referenced by a prior import (`e.wallet`) OR a prior
/// tranche declaration (payload), and reports a never-referenced wallet as a phantom (→ WARN, not refuse).
#[test]
fn wallet_is_known_covers_imports_and_tranche_payloads_and_flags_phantoms() {
    use btctax_core::event::{Acquire, DeclareTranche};
    use btctax_core::identity::{EventId, Source, SourceRef};
    use btctax_core::{BasisSource, LedgerEvent};
    use rust_decimal_macros::dec;
    use time::macros::date;
    let imported = WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    };
    let declared = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let phantom = WalletId::SelfCustody {
        label: "typo".into(),
    };
    let import = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("x")),
        utc_timestamp: now(),
        original_tz: time::UtcOffset::UTC,
        wallet: Some(imported.clone()),
        payload: EventPayload::Acquire(Acquire {
            sat: 1,
            usd_cost: dec!(1),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };
    let tranche = LedgerEvent {
        id: EventId::decision(1),
        utc_timestamp: now(),
        original_tz: time::UtcOffset::UTC,
        wallet: None,
        payload: EventPayload::DeclareTranche(DeclareTranche {
            sat: 1,
            wallet: declared.clone(),
            window_start: date!(2018 - 01 - 01),
            window_end: date!(2018 - 12 - 31),
        }),
    };
    let evs = vec![import, tranche];
    assert!(
        cmd::tranche::wallet_is_known(&evs, &imported),
        "an import's e.wallet is known"
    );
    assert!(
        cmd::tranche::wallet_is_known(&evs, &declared),
        "a tranche payload's wallet is known"
    );
    assert!(
        !cmd::tranche::wallet_is_known(&evs, &phantom),
        "a never-referenced wallet is a phantom (→ warn)"
    );
}

/// (a2, T16 arch r1 Minor) The PRODUCT path is already protected: `reconcile void` REFUSES voiding an
/// effective allocation (§7.4 irrevocable), so the "dangling void on an effective allocation" the review
/// worried about is NOT product-reachable — it needs a hand-crafted raw `VoidDecisionEvent`. (The review's
/// "product-reachable via reconcile void" premise was thus imprecise; documented here.)
#[test]
fn reconcile_void_refuses_voiding_an_effective_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_effective_alloc(dir.path());
    let alloc_id = effective_alloc_id(&vault);
    let err = cmd::reconcile::void(&vault, &pp(), &alloc_id.canonical(), now()).unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)) && err.to_string().to_lowercase().contains("effective"),
        "voiding an effective allocation is refused as irrevocable: {err}"
    );
}

/// (a3, T16 review r2 / I-1) A HAND-CRAFTED raw void of an EFFECTIVE allocation (bypassing `reconcile
/// void`'s §7.4 refusal above) followed by a pre-2025 tranche: the record-time guard ADMITS the tranche
/// (a voided allocation is not in force — the ENGINE resolves the void), and the projection resolves
/// SAFELY — the D-8 backstop denies the allocation effectiveness, the §7.4 retirement pass retires it,
/// and Path A governs so the tranche SURVIVES. Crucially: NO silent Path-B discard (the allocation never
/// seeds `SafeHarborAllocated` lots), which is what the SPEC's "denied effectiveness, tag survives"
/// promises even for this hand-crafted corner.
#[test]
fn handcrafted_void_of_effective_alloc_then_tranche_admits_and_survives_via_path_a() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_effective_alloc(dir.path());
    let alloc_id = effective_alloc_id(&vault);
    // Hand-craft a raw void of the effective allocation (bypasses reconcile-void's refusal).
    {
        let mut s = Session::open(&vault, &pp()).unwrap();
        btctax_core::persistence::append_decision(
            s.conn(),
            EventPayload::VoidDecisionEvent(btctax_core::event::VoidDecisionEvent {
                target_event_id: alloc_id,
            }),
            now(),
            time::UtcOffset::UTC,
            None,
        )
        .unwrap();
        s.save().unwrap();
    }
    // ADMITTED — the voided allocation is not in force.
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
    let (state, _) = s.project().unwrap();
    // The tranche survives via Path A — and the allocation NEVER seeds a Path-B `SafeHarborAllocated` lot
    // (no silent discard). The year is computable (no Hard SafeHarborUnconservable left on the voided alloc).
    assert!(
        state.lots.iter().any(|l| l.basis_source
            == btctax_core::BasisSource::EstimatedConservative
            && l.remaining_sat > 0),
        "the tranche survives via Path A (tag intact)"
    );
    assert!(
        !state
            .lots
            .iter()
            .any(|l| l.basis_source == btctax_core::BasisSource::SafeHarborAllocated),
        "no Path-B seed lot — the tranche is never silently discarded (SPEC D-8)"
    );
    assert!(
        !state
            .blockers
            .iter()
            .any(|b| b.kind == btctax_core::BlockerKind::SafeHarborUnconservable),
        "no Hard SafeHarborUnconservable left on the retired allocation: {:?}",
        state.blockers
    );
}

/// The persisted `SafeHarborAllocation`'s decision id (for voiding it).
fn effective_alloc_id(vault: &Path) -> btctax_core::EventId {
    let s = Session::open(vault, &pp()).unwrap();
    btctax_core::persistence::load_all(s.conn())
        .unwrap()
        .into_iter()
        .find(|e| matches!(e.payload, EventPayload::SafeHarborAllocation(_)))
        .map(|e| e.id)
        .expect("an allocation is on file")
}

/// (I-1, T16 review r1) The SUPPORTED flow must NOT brick the vault: void an INERT allocation, then
/// declare a pre-2025 tranche (the guard ADMITS it — the voided-inert allocation is not in force). Before
/// the fix the D-8 backstop re-evaluated the voided allocation and pushed a PERMANENT Hard
/// SafeHarborUnconservable → every year NotComputable, with no clearing move. Now Path A governs, the
/// tranche lot survives, and no Hard blocker is emitted.
#[test]
fn void_inert_alloc_then_declare_pre2025_tranche_keeps_the_year_computable() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault_inert_alloc(dir.path());
    let alloc_id = effective_alloc_id(&vault);
    cmd::reconcile::void(&vault, &pp(), &alloc_id.canonical(), now()).unwrap(); // inert ⇒ voidable
    cmd::tranche::declare_tranche(
        &vault,
        &pp(),
        50_000_000,
        tranche_wallet(),
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
        now(),
    )
    .unwrap(); // ADMITTED (voided-inert allocation is not in force)

    let s = Session::open(&vault, &pp()).unwrap();
    let (state, _) = s.project().unwrap();
    // No Hard blocker of ANY kind survives on the retired allocation ⇒ the year is computable (r2-N-1: pin
    // computability directly, not just the absence of the specific Unconservable).
    assert!(
        !state
            .blockers
            .iter()
            .any(|b| b.kind.severity() == btctax_core::Severity::Hard),
        "no Hard blocker survives the void-inert-then-declare flow ⇒ every year computes (I-1): {:?}",
        state.blockers
    );
    assert!(
        state.lots.iter().any(|l| l.basis_source
            == btctax_core::BasisSource::EstimatedConservative
            && l.remaining_sat > 0),
        "the tranche lot survives via Path A (tag intact)"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★ T2-M2 follow-up (FOLLOWUPS.md, Task 9): the shipped phantom-wallet stderr warning
// (`cmd/tranche.rs::declare_tranche`, `eprintln!` AFTER `plan_declare` succeeds) is preserved
// byte-for-byte by the Defensive Filing Wizard's chokepoint extraction (Task 2), but no test pinned
// its actual EMISSION. `eprintln!` cannot be intercepted in-process, so this spawns the REAL `btctax`
// binary (mirrors `chokepoint_parity.rs`'s own subprocess convention) and captures its real stderr.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Run `btctax --vault <vault> reconcile declare-tranche <args...>`; returns (exit, stderr).
fn run_declare(vault: &Path, args: &[&str]) -> (i32, String) {
    let bin = env!("CARGO_BIN_EXE_btctax");
    let mut c = std::process::Command::new(bin);
    c.arg("--vault")
        .arg(vault.to_str().unwrap())
        .arg("reconcile")
        .arg("declare-tranche");
    for a in args {
        c.arg(a);
    }
    c.env("BTCTAX_PASSPHRASE", "pw");
    let out = c.output().expect("btctax binary must execute");
    (
        out.status.code().expect("exits normally"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// (a) A never-before-referenced `--wallet` on an otherwise-valid declare EMITS the shipped
/// phantom-wallet warning verbatim, on a SUCCESSFUL (exit 0) run.
#[test]
fn phantom_wallet_warning_is_emitted_verbatim_on_a_successful_declare() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let (code, stderr) = run_declare(
        &vault,
        &[
            "--amount",
            "0.5",
            "--wallet",
            "self:phantom",
            "--window-start",
            "2020-01-01",
            "--window-end",
            "2020-12-31",
        ],
    );
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stderr.contains("phantom wallet"),
        "the shipped phantom-wallet warning must be emitted verbatim on success: {stderr:?}"
    );
    assert!(
        stderr.contains("self:phantom"),
        "the warning must name the offending --wallet: {stderr:?}"
    );
}

/// (b) A REFUSED declare (a non-positive `--amount`) never reaches the wallet-check — it is SILENT
/// (the `eprintln!` sits AFTER `plan_declare`'s own `?`, so a refused plan never runs it).
#[test]
fn phantom_wallet_warning_is_silent_on_a_refused_declare() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault.pgp");
    cmd::init::run(&vault, &pp(), &dir.path().join("k.asc")).unwrap();

    let (code, stderr) = run_declare(
        &vault,
        &[
            "--amount",
            "0",
            "--wallet",
            "self:phantom",
            "--window-start",
            "2020-01-01",
            "--window-end",
            "2020-12-31",
        ],
    );
    assert_ne!(code, 0, "a non-positive amount must be refused");
    assert!(
        !stderr.contains("phantom wallet"),
        "a refused declare must NEVER reach the phantom-wallet warning: {stderr:?}"
    );
}
