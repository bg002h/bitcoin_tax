//! KATs for the conservative-filing `DeclareTranche` core (Phase 1).
//!
//! See `design/conservative-filing/{SPEC,IMPLEMENTATION_PLAN}.md`. A tranche is undocumented BTC
//! declared at $0 basis (the IRS fallback), tagged `BasisSource::EstimatedConservative`, homed at
//! `acquired_at = window_end`; filing-ready (NOT pseudo). PRIVACY: synthetic values only.

use btctax_core::event::*;
use btctax_core::forms::how_acquired_from;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::resolve::{resolve, sort_canonical, Op};
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::voidable_decisions;
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

fn void_ev(seq: u64, target: EventId) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2026-01-02 00:00 UTC),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: target,
        }),
    )
}

/// Task 3 (D-1a-c): a `DeclareTranche` folds as an `Op`, never `Op::Skip` (the build_op arm exists).
/// Observed on the resolved timeline (build_op is private).
#[test]
fn declare_tranche_yields_an_op_never_skip() {
    let w = exch();
    let ev = tranche_ev(
        1,
        &w,
        50_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let res = resolve(&[ev], &prices(), &cfg());
    let eff = res
        .timeline
        .iter()
        .find(|e| e.id == EventId::decision(1))
        .expect("the tranche has a timeline Eff");
    assert!(
        !matches!(eff.op, Op::Skip),
        "a DeclareTranche must fold as an Op, never Skip (D-1a-c)"
    );
}

/// Task 3 (D-1a-d): a VOIDED tranche folds nothing — the admit honors `voided`.
#[test]
fn voided_declare_tranche_folds_no_lot() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        50_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let v = void_ev(2, EventId::decision(1));
    let st = project(&[t, v], &prices(), &cfg());
    assert!(
        st.lots.iter().all(|l| l.wallet != w),
        "a voided tranche folds nothing (D-1a-d)"
    );
}

/// Task 3 (arch I-2): a tranche is listed as a voidable decision on the PRODUCT surface
/// (`voidable_decisions` is the single source of truth for bulk-void + both TUI void flows) — not only
/// void-able by the engine. Without the `is_revocable_payload` arm the product treats it as permanent.
#[test]
fn a_tranche_is_listed_as_a_voidable_decision_on_the_product_surface() {
    let w = exch();
    let events = vec![tranche_ev(
        7,
        &w,
        50_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    )];
    let voidable = voidable_decisions(&events, &[]);
    assert!(
        voidable.iter().any(|e| e.id == EventId::decision(7)),
        "a tranche is sweep-voidable (D-1a-d)"
    );
}

/// Task 3 (★ D-1a-b / arch r1 I-1 + r3 NEW-I-1): two legitimately-additive same-window tranches (seqs 2
/// and 10) order by NUMERIC seq in the CANONICAL timeline. Canonical order is `sort_canonical`'s output,
/// applied by the fold pipeline (fold.rs) — `resolve()` returns the timeline UNSORTED, so the KAT composes
/// `sort_canonical` explicitly. Reverting the constant src_ref (→ "10" < "2") or the numeric id key (→
/// stable push order) misorders these → RED.
#[test]
fn two_same_window_tranches_are_ordered_by_seq_in_the_canonical_timeline() {
    let w = exch();
    let a = tranche_ev(
        2,
        &w,
        10_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let b = tranche_ev(
        10,
        &w,
        20_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let mut res = resolve(&[b, a], &prices(), &cfg()); // pushed OUT of seq order
    sort_canonical(&mut res.timeline);
    let seqs: Vec<u64> = res
        .timeline
        .iter()
        .filter_map(|e| match e.id {
            EventId::Decision { seq } => Some(seq),
            _ => None,
        })
        .collect();
    assert_eq!(
        seqs,
        vec![2, 10],
        "same-window tranche Effs are canonically ordered by numeric seq (D-1a-b)"
    );
}

/// Task 3 (D-1a-d): two same-window tranches are ADDITIVE — two lots, not a duplicate-conflict.
/// (Observable lot order comes from `finalize`/`LotId`, not `sort_canonical` — so this pins additivity +
/// the observable order, not the sort fix, which the timeline KAT above owns.)
#[test]
fn two_same_window_tranches_are_additive_not_a_duplicate_conflict() {
    let w = exch();
    let a = tranche_ev(
        2,
        &w,
        10_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let b = tranche_ev(
        10,
        &w,
        20_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let st = project(&[b, a], &prices(), &cfg());
    let lots: Vec<_> = st.lots.iter().filter(|l| l.wallet == w).collect();
    assert_eq!(
        lots.len(),
        2,
        "two same-window tranches are additive (D-1a-d)"
    );
    assert_eq!(lots[0].lot_id.origin_event_id, EventId::decision(2));
    assert_eq!(lots[1].lot_id.origin_event_id, EventId::decision(10));
}
