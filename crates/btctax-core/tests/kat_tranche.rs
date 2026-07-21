//! KATs for the conservative-filing `DeclareTranche` core (Phase 1).
//!
//! See `design/conservative-filing/{SPEC,IMPLEMENTATION_PLAN}.md`. A tranche is undocumented BTC
//! declared at $0 basis (the IRS fallback), tagged `BasisSource::EstimatedConservative`, homed at
//! `acquired_at = window_end`; filing-ready (NOT pseudo). PRIVACY: synthetic values only.

use btctax_core::event::*;
use btctax_core::forms::how_acquired_from;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::Form8283HowAcquired;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

// ── fixture harness (mirrors tests/method_election.rs) ─────────────────────────────────────────────
fn exch() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: p,
    }
}
/// A DeclareTranche decision event. `utc_timestamp` is the CREATION time (here a fixed 2026 stamp) —
/// deliberately unrelated to `window_end`, to prove the fold homes the lot at window_end regardless.
fn tranche_ev(seq: u64, w: &WalletId, sat: i64, ws: time::Date, we: time::Date) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-01-01 00:00 UTC),
        EventPayload::DeclareTranche(DeclareTranche {
            sat,
            wallet: w.clone(),
            window_start: ws,
            window_end: we,
        }),
    )
}
fn prices() -> StaticPrices {
    StaticPrices::default()
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default()
}

/// Task 1 (tax min-6): `EstimatedConservative` is NOT an 8949 column; on Form 8283 (donation) it needs
/// manual review — an LT tranche donation → FMV; an ST-held tranche donation → deduction limited to
/// basis = $0 (§170(e)(1)(A)).
#[test]
fn estimated_conservative_donor_field_is_review() {
    assert_eq!(
        how_acquired_from(BasisSource::EstimatedConservative),
        Form8283HowAcquired::Review
    );
}

/// Task 2 (D-1/D-1a/D-2): a `DeclareTranche` folds (via the reused `Op::Acquire` arm) to exactly the
/// D-1 lot — $0 basis, `EstimatedConservative`, `acquired_at = window_end`, declared wallet, NOT pseudo.
#[test]
fn declare_tranche_folds_to_zero_basis_estimated_conservative_lot_homed_at_window_end() {
    let w = exch();
    let ev = tranche_ev(
        1,
        &w,
        50_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let st = project(&[ev], &prices(), &cfg());
    let lot = st
        .lots
        .iter()
        .find(|l| l.wallet == w)
        .expect("a tranche lot");
    assert_eq!(lot.usd_basis, dec!(0), "tranche basis is $0 (G-2/D-7)");
    assert_eq!(lot.basis_source, BasisSource::EstimatedConservative);
    assert_eq!(
        lot.acquired_at,
        date!(2018 - 12 - 31),
        "acquired_at = window_end (D-2), decoupled from the 2026 creation timestamp"
    );
    assert_eq!(lot.original_sat, 50_000_000);
    assert!(!lot.pseudo, "a tranche is filing-ready, NOT pseudo (D-5)");
}
