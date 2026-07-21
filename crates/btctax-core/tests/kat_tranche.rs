//! KATs for the conservative-filing `DeclareTranche` core (Phase 1).
//!
//! See `design/conservative-filing/{SPEC,IMPLEMENTATION_PLAN}.md`. A tranche is undocumented BTC
//! declared at $0 basis (the IRS fallback), tagged `BasisSource::EstimatedConservative`, homed at
//! `acquired_at = window_end`; filing-ready (NOT pseudo). PRIVACY: synthetic values only.

use btctax_core::event::*;
use btctax_core::forms::how_acquired_from;
use btctax_core::forms::{form_8949, Form8949Box, Form8949Part};
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::resolve::{resolve, sort_canonical, Op};
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::{BlockerKind, Term};
use btctax_core::voidable_decisions;
use btctax_core::Form8283HowAcquired;
use btctax_core::LotMethod;
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

// ── Task 4 fixtures: self-custody wallet, import sale, confirmed self-transfer ─────────────────────
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, w: &WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: p,
    }
}
fn sell_ev(
    rf: &str,
    ts: time::OffsetDateTime,
    w: &WalletId,
    sat: i64,
    proceeds: i64,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: rust_decimal::Decimal::from(proceeds),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
/// A confirmed self-transfer: TransferOut (from) + TransferIn (to) + a TransferLink decision.
fn self_transfer(
    out_rf: &str,
    in_rf: &str,
    from: &WalletId,
    to: &WalletId,
    sat: i64,
    ts: time::OffsetDateTime,
    link_seq: u64,
) -> Vec<LedgerEvent> {
    vec![
        imp(
            out_rf,
            ts,
            from,
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            in_rf,
            ts,
            to,
            EventPayload::TransferIn(TransferIn {
                sat,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            link_seq,
            ts,
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new(out_rf)),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Coinbase,
                    SourceRef::new(in_rf),
                )),
            }),
        ),
    ]
}

/// Task 4 (D-8 + D-6/G-4): a PRE-2025 tranche survives the 2025 Path-A reconstruction with its tag, and a
/// 2025 disposal of it carries `EstimatedConservative`, derives LONG-term, and files Part II / Box L (the
/// TY2025 no-1099-DA digital-asset box, inherited from the merged box fix) with `box_needs_review`.
#[test]
fn tranche_tag_survives_2025_path_a_seed_and_reaches_a_2025_disposal_leg() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2015 - 01 - 01),
        date!(2015 - 12 - 31),
    );
    let sell = sell_ev(
        "S25",
        datetime!(2025-06-01 00:00 UTC),
        &w,
        100_000_000,
        60_000,
    ); // > 1yr after window_end
    let st = project(&[t, sell], &prices(), &cfg());
    let leg = st
        .disposals
        .iter()
        .filter(|d| d.disposed_at.year() == 2025)
        .flat_map(|d| &d.legs)
        .find(|l| l.wallet == w)
        .expect("a 2025 disposal leg");
    assert_eq!(
        leg.basis_source,
        BasisSource::EstimatedConservative,
        "tag survives Path-A seed (D-8)"
    );
    assert_eq!(
        leg.term,
        Term::LongTerm,
        "term DERIVED from window_end (G-4), not assumed"
    );
    let rows = form_8949(&st, 2025);
    let row = rows
        .iter()
        .find(|r| r.cost_basis == dec!(0))
        .expect("the $0 tranche row");
    assert_eq!(row.part, Form8949Part::LongTerm, "LT → Part II (D-6/G-4)");
    assert_eq!(row.box_, Form8949Box::L, "TY2025 no-1099-DA LT → Box L");
    assert!(
        row.box_needs_review,
        "exchange-sold tranche → box_needs_review (reclass to K/H if 1099-DA)"
    );
}

/// Task 4 (G-4 other direction): a tranche sold < 1yr after window_end is SHORT-term → Part I / Box I,
/// never silently long-term.
#[test]
fn a_short_term_tranche_disposal_is_box_i_never_hard_coded_long_term() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 02 - 01),
    );
    let sell = sell_ev(
        "S25",
        datetime!(2025-09-01 00:00 UTC),
        &w,
        100_000_000,
        60_000,
    ); // < 1yr
    let st = project(&[t, sell], &prices(), &cfg());
    let row = form_8949(&st, 2025)
        .into_iter()
        .find(|r| r.cost_basis == dec!(0))
        .unwrap();
    assert_eq!(row.part, Form8949Part::ShortTerm);
    assert_eq!(row.box_, Form8949Box::I, "TY2025 no-1099-DA ST → Box I");
}

/// Task 4 (tax M-1): the holding period is "LT iff window_end > 1yr before disposal" — pinned at the
/// EXACT boundary through the tranche wiring. Sale exactly one year after window_end is SHORT-term
/// (strict `>`, §1222 / Pub 544 day-after); one day later is LONG-term.
#[test]
fn holding_period_boundary_is_iff_exactly_one_year() {
    let w = exch();
    let we = date!(2025 - 03 - 01);
    let mk = |sell_ts: time::OffsetDateTime| {
        let t = tranche_ev(1, &w, 100_000_000, date!(2025 - 03 - 01), we);
        let s = sell_ev("S", sell_ts, &w, 100_000_000, 60_000);
        let st = project(&[t, s], &prices(), &cfg());
        st.disposals
            .iter()
            .flat_map(|d| &d.legs)
            .find(|l| l.wallet == w)
            .unwrap()
            .term
    };
    assert_eq!(
        mk(datetime!(2026-03-01 00:00 UTC)),
        Term::ShortTerm,
        "exactly one year after window_end is SHORT-term"
    );
    assert_eq!(
        mk(datetime!(2026-03-02 00:00 UTC)),
        Term::LongTerm,
        "one day past a year is LONG-term"
    );
}

/// Task 4 (tax N-2): D-6 is year-aware BOTH directions — a tranche disposed in a PRE-2025 tax year files
/// the securities Box C (ST) / F (LT), not the digital-asset I/L.
#[test]
fn a_pre_2025_tranche_disposal_files_the_securities_boxes_c_f() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2015 - 01 - 01),
        date!(2015 - 12 - 31),
    );
    let sell = sell_ev(
        "S20",
        datetime!(2020-06-01 00:00 UTC),
        &w,
        100_000_000,
        40_000,
    ); // pre-2025, > 1yr → LT
    let st = project(&[t, sell], &prices(), &cfg());
    let row = form_8949(&st, 2020)
        .into_iter()
        .find(|r| r.cost_basis == dec!(0))
        .unwrap();
    assert_eq!(
        row.box_,
        Form8949Box::F,
        "pre-2025 LT tranche → securities Box F, not digital-asset L"
    );
}

/// Task 4 (D-8 / arch r2 New-2): a relocated tranche keeps its tag — exactly the Exchange→SelfCustody
/// move P8 recommends — so the disposal leg keeps P3's dip advisory and P7's mandatory disclosure.
#[test]
fn tranche_tag_survives_self_transfer_relocation() {
    let ex = exch();
    let sc = cold();
    // Post-2025 tranche (directly in the Exchange pool, no transition) → isolate the relocation exemption.
    let t = tranche_ev(
        1,
        &ex,
        50_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 02 - 01),
    );
    let mut evs = vec![t];
    evs.extend(self_transfer(
        "XO",
        "XI",
        &ex,
        &sc,
        50_000_000,
        datetime!(2025-03-01 00:00 UTC),
        9,
    ));
    let st = project(&evs, &prices(), &cfg());
    let lot = st
        .lots
        .iter()
        .find(|l| l.wallet == sc)
        .expect("a relocated lot in self-custody");
    assert_eq!(
        lot.basis_source,
        BasisSource::EstimatedConservative,
        "tag survives relocation (D-8)"
    );
}

// ── Task 5: safe-harbor allocation fixture (mirrors tests/transition.rs) ───────────────────────────
fn alloc_ev(
    seq: u64,
    attested: bool,
    pre2025_method: LotMethod,
    lots: Vec<AllocLot>,
) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2024-12-01 00:00 UTC),
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots,
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ActualPosition,
            timely_allocation_attested: attested,
            pre2025_method,
        }),
    )
}
fn alloc_lot(w: &WalletId, sat: i64, basis: i64, acq: time::Date) -> AllocLot {
    AllocLot {
        wallet: w.clone(),
        sat,
        usd_basis: rust_decimal::Decimal::from(basis),
        acquired_at: acq,
        dual_loss_basis: None,
        donor_acquired_at: None,
    }
}

/// Task 5 (D-8 projection-time backstop / arch r3 New-1): an allocation whose totals WOULD conserve over
/// a pre-2025 residue containing a $0 EstimatedConservative tranche is DENIED effectiveness (Hard
/// SafeHarborUnconservable → inert → Path A), so the tranche survives instead of being silently discarded
/// by a Path-B seed. Independent of declaration order.
#[test]
fn allocation_that_would_conserve_over_a_tranche_residue_is_kept_inert_and_tag_survives() {
    let w = exch();
    // Pre-2025 tranche alone → the Universal residue is exactly {tranche: 100M sat, $0}.
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2015 - 01 - 01),
        date!(2015 - 12 - 31),
    );
    // An allocation whose ONE lot matches that residue on totals (100M sat, $0 basis) — so WITHOUT the
    // backstop it would go effective (Path B) and silently discard the tranche.
    let a = alloc_ev(
        2,
        true,
        LotMethod::Hifo,
        vec![alloc_lot(&w, 100_000_000, 0, date!(2015 - 12 - 31))],
    );
    let st = project(&[t, a], &prices(), &cfg());
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SafeHarborUnconservable),
        "the allocation is denied effectiveness by the D-8 backstop"
    );
    let lot = st
        .lots
        .iter()
        .find(|l| l.basis_source == BasisSource::EstimatedConservative)
        .expect("tranche survives via Path A (not discarded by Path B)");
    assert!(lot.remaining_sat > 0);
}

/// Task 7 (D-5): a filed tranche is NOT pseudo — its lot is real (`pseudo=false`) and the projection's
/// `pseudo_active()` stays false, so a tranche year exports CLEAN (no `[PSEUDO]` watermark, no
/// attestation gate). Contrast the pseudo-reconcile path, whose synthetic defaults DO trip the gate.
#[test]
fn a_filed_tranche_projection_is_not_pseudo_active() {
    let w = exch();
    let ev = tranche_ev(
        1,
        &w,
        50_000_000,
        date!(2020 - 01 - 01),
        date!(2020 - 12 - 31),
    );
    let st = project(&[ev], &prices(), &cfg());
    // Non-vacuous: the tranche lot really is in the projection...
    let lot = st
        .lots
        .iter()
        .find(|l| l.basis_source == BasisSource::EstimatedConservative)
        .expect("a tranche lot");
    assert!(!lot.pseudo, "a filed tranche lot is real, not pseudo (D-5)");
    // ...and it does not activate pseudo mode (the export-refusal signal, state.rs).
    assert!(
        !st.pseudo_active(),
        "a real tranche never activates pseudo mode → clean, non-watermarked export (D-5)"
    );
}
