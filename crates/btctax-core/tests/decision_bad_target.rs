//! D1 — bad-target validation KATs for ReclassifyOutflow, ClassifyInbound, and ManualFmv.
//!
//! All fixtures are SYNTHETIC. No real user data is read.
//!
//! Covered:
//!  - ReclassifyOutflow: missing target → DecisionConflict, no disposal/removal created.
//!  - ReclassifyOutflow: wrong-type target (Income) → DecisionConflict, income unaffected.
//!  - ClassifyInbound: missing target → DecisionConflict, no lot created.
//!  - ClassifyInbound: wrong-type target (TransferOut) → DecisionConflict, no lot.
//!  - ManualFmv: missing target → DecisionConflict, original FMV unchanged.
//!  - ManualFmv: wrong-type target (TransferIn) → DecisionConflict, no income.
//!  - ReclassifyOutflow valid happy path → removal created, no DecisionConflict.
//!  - ClassifyInbound valid happy path → lot created, no DecisionConflict.
//!  - ManualFmv valid happy path (FmvMissing Income) → FMV override applied.
//!  - ManualFmv latest-seq-wins preserved (no duplicate blocker).
//!  - Void remedy: bad ReclassifyOutflow + void → no DecisionConflict after void.

use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

// ── Fixture helpers ────────────────────────────────────────────────────────────────────────────

fn cb_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}

fn gem_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "gemini".into(),
        account: "main".into(),
    }
}

/// Build an imported LedgerEvent (Source::Coinbase by default).
fn ev(
    ref_str: &str,
    ts: time::OffsetDateTime,
    wallet: WalletId,
    payload: EventPayload,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(ref_str)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wallet),
        payload,
    }
}

/// Build an imported LedgerEvent with an explicit source.
fn ev_src(
    source: Source,
    ref_str: &str,
    ts: time::OffsetDateTime,
    wallet: WalletId,
    payload: EventPayload,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(source, SourceRef::new(ref_str)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wallet),
        payload,
    }
}

/// Build a decision LedgerEvent.
fn dec_ev(seq: u64, ts: time::OffsetDateTime, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload,
    }
}

// ── Canonical timestamps ───────────────────────────────────────────────────────────────────────

fn ts_event() -> time::OffsetDateTime {
    datetime!(2025-03-01 12:00:00 UTC)
}
fn ts_event2() -> time::OffsetDateTime {
    datetime!(2025-04-01 12:00:00 UTC)
}
fn ts_decision() -> time::OffsetDateTime {
    datetime!(2025-06-01 00:00:00 UTC)
}

// ── Helper: count DecisionConflict blockers ────────────────────────────────────────────────────

fn count_decision_conflicts(st: &LedgerState) -> usize {
    st.blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::DecisionConflict)
        .count()
}

// ── Test 1: ReclassifyOutflow — missing target ─────────────────────────────────────────────────

/// ReclassifyOutflow pointing at a non-existent event → exactly 1 DecisionConflict;
/// no disposal or removal created; the real TransferOut is unaffected (→ PendingOut).
#[test]
fn reclassify_outflow_missing_target_yields_blocker() {
    // Provide a real TransferOut with holdings to back it.
    let buy = ev(
        "OUT-1-BUY",
        ts_event(),
        cb_wallet(),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT-1",
        ts_event2(),
        cb_wallet(),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    // Decision targets "OUT-BOGUS" which does not exist.
    let bogus_id = EventId::import(Source::Coinbase, SourceRef::new("OUT-BOGUS"));
    let bad = dec_ev(
        1,
        ts_decision(),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: bogus_id,
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(50.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[buy, out, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        1,
        "missing target must yield exactly 1 DecisionConflict: {:?}",
        st.blockers
    );
    assert!(st.disposals.is_empty(), "no disposal expected");
    assert!(st.removals.is_empty(), "no removal expected");
}

// ── Test 2: ReclassifyOutflow — wrong-type target (Income event) ──────────────────────────────

/// ReclassifyOutflow pointing at an Income event → exactly 1 DecisionConflict;
/// no disposal or removal created; the income is unaffected.
#[test]
fn reclassify_outflow_wrong_type_target_income_yields_blocker() {
    let income = ev(
        "INCOME-1",
        ts_event(),
        cb_wallet(),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000.00)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    );
    // Decision targets "INCOME-1" — an Income event, not a TransferOut.
    let income_id = EventId::import(Source::Coinbase, SourceRef::new("INCOME-1"));
    let bad = dec_ev(
        1,
        ts_decision(),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: income_id,
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(50.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[income, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        1,
        "wrong-type target must yield exactly 1 DecisionConflict: {:?}",
        st.blockers
    );
    assert!(st.disposals.is_empty(), "no disposal expected");
    assert!(st.removals.is_empty(), "no removal expected");
    assert_eq!(st.income_recognized.len(), 1, "income must be unaffected");
}

// ── Test 3: ClassifyInbound — missing target ──────────────────────────────────────────────────

/// ClassifyInbound pointing at a non-existent event → exactly 1 DecisionConflict;
/// no lot created from the decision; the real TransferIn creates an UnknownBasisInbound.
#[test]
fn classify_inbound_missing_target_yields_blocker() {
    let tin = ev(
        "IN-1",
        ts_event(),
        cb_wallet(),
        EventPayload::TransferIn(TransferIn {
            sat: 50_000,
            src_addr: None,
            txid: None,
        }),
    );
    // Decision targets "IN-BOGUS" which does not exist.
    let bogus_id = EventId::import(Source::Coinbase, SourceRef::new("IN-BOGUS"));
    let bad = dec_ev(
        1,
        ts_decision(),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: bogus_id,
            as_: InboundClass::GiftReceived {
                donor_basis: Some(dec!(25.00)),
                donor_acquired_at: None,
                fmv_at_gift: dec!(30.00),
            },
        }),
    );
    let st = project(
        &[tin, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // Filter specifically for DecisionConflict (UnknownBasisInbound is also expected for the TransferIn).
    assert_eq!(
        count_decision_conflicts(&st),
        1,
        "missing target must yield exactly 1 DecisionConflict: {:?}",
        st.blockers
    );
    // No lot created from the bad ClassifyInbound.
    assert!(
        st.lots.is_empty(),
        "no lot expected from bad ClassifyInbound"
    );
    assert!(st.income_recognized.is_empty(), "no income expected");
}

// ── Test 4: ClassifyInbound — wrong-type target (TransferOut event) ───────────────────────────

/// ClassifyInbound pointing at a TransferOut event → exactly 1 DecisionConflict;
/// no income lot created.
#[test]
fn classify_inbound_wrong_type_target_transferout_yields_blocker() {
    // Provide a real TransferOut with holdings to back it.
    let buy = ev(
        "OUT-2-BUY",
        ts_event(),
        cb_wallet(),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT-2",
        ts_event2(),
        cb_wallet(),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    // Decision targets "OUT-2" — a TransferOut event, not a TransferIn.
    let out_id = EventId::import(Source::Coinbase, SourceRef::new("OUT-2"));
    let bad = dec_ev(
        1,
        ts_decision(),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: out_id,
            as_: InboundClass::GiftReceived {
                donor_basis: Some(dec!(25.00)),
                donor_acquired_at: None,
                fmv_at_gift: dec!(30.00),
            },
        }),
    );
    let st = project(
        &[buy, out, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        1,
        "wrong-type target must yield exactly 1 DecisionConflict: {:?}",
        st.blockers
    );
    assert!(st.income_recognized.is_empty(), "no income expected");
    // The bad ClassifyInbound targeting a TransferOut produces no income lot.
    // (The existing lots from the buy are consumed by the pending TransferOut.)
    assert!(
        st.lots.is_empty(),
        "no income lot expected from bad ClassifyInbound"
    );
}

// ── Test 5: ManualFmv — missing target ────────────────────────────────────────────────────────

/// ManualFmv pointing at a non-existent event → exactly 1 DecisionConflict;
/// the original Income FMV is unchanged.
#[test]
fn manual_fmv_missing_target_yields_blocker() {
    let income = ev(
        "INCOME-2",
        ts_event(),
        cb_wallet(),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000.00)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    );
    // Decision targets "INCOME-BOGUS" which does not exist.
    let bogus_id = EventId::import(Source::Coinbase, SourceRef::new("INCOME-BOGUS"));
    let bad = dec_ev(
        1,
        ts_decision(),
        EventPayload::ManualFmv(ManualFmv {
            event: bogus_id,
            usd_fmv: dec!(99999.00),
        }),
    );
    let st = project(
        &[income, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        1,
        "missing target must yield exactly 1 DecisionConflict: {:?}",
        st.blockers
    );
    // Original FMV ($10,000) is unchanged — the bad ManualFmv was excluded.
    assert_eq!(st.income_recognized.len(), 1);
    assert_eq!(
        st.income_recognized[0].usd_fmv,
        dec!(10000.00),
        "original FMV must be unchanged"
    );
}

// ── Test 6: ManualFmv — wrong-type target (TransferIn event) ──────────────────────────────────

/// ManualFmv pointing at a TransferIn event → exactly 1 DecisionConflict;
/// no income recognized (TransferIn without ClassifyInbound → UnknownInbound, not income).
#[test]
fn manual_fmv_wrong_type_target_transferin_yields_blocker() {
    let tin = ev(
        "IN-2",
        ts_event(),
        cb_wallet(),
        EventPayload::TransferIn(TransferIn {
            sat: 50_000,
            src_addr: None,
            txid: None,
        }),
    );
    // Decision targets "IN-2" — a TransferIn, not an Income event.
    let in_id = EventId::import(Source::Coinbase, SourceRef::new("IN-2"));
    let bad = dec_ev(
        1,
        ts_decision(),
        EventPayload::ManualFmv(ManualFmv {
            event: in_id,
            usd_fmv: dec!(99999.00),
        }),
    );
    let st = project(
        &[tin, bad],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // Filter for DecisionConflict specifically (UnknownBasisInbound also present from the TransferIn).
    assert_eq!(
        count_decision_conflicts(&st),
        1,
        "wrong-type target must yield exactly 1 DecisionConflict: {:?}",
        st.blockers
    );
    assert!(
        st.income_recognized.is_empty(),
        "no income expected (TransferIn → UnknownInbound, not income)"
    );
}

// ── Test 7: ReclassifyOutflow valid happy path ─────────────────────────────────────────────────

/// Valid ReclassifyOutflow (GiftOut) against a real TransferOut → removal created;
/// no DecisionConflict.
#[test]
fn reclassify_outflow_valid_happy_path() {
    let buy = ev(
        "ACQ-H",
        ts_event(),
        cb_wallet(),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT-H",
        ts_event2(),
        cb_wallet(),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let out_id = EventId::import(Source::Coinbase, SourceRef::new("OUT-H"));
    let recl = dec_ev(
        1,
        ts_decision(),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: out_id,
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(50.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        0,
        "valid GiftOut must not produce a DecisionConflict: {:?}",
        st.blockers
    );
    assert_eq!(st.removals.len(), 1, "gift removal must be created");
    assert!(st.disposals.is_empty(), "must not be a disposal");
}

// ── Test 8: ClassifyInbound valid happy path ──────────────────────────────────────────────────

/// Valid ClassifyInbound (GiftReceived) against a real TransferIn → gift lot created;
/// no DecisionConflict; no income recognized.
#[test]
fn classify_inbound_valid_happy_path() {
    let tin = ev_src(
        Source::Gemini,
        "IN-H",
        ts_event(),
        gem_wallet(),
        EventPayload::TransferIn(TransferIn {
            sat: 50_000,
            src_addr: None,
            txid: None,
        }),
    );
    let in_id = EventId::import(Source::Gemini, SourceRef::new("IN-H"));
    let cls = dec_ev(
        1,
        ts_decision(),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: in_id,
            as_: InboundClass::GiftReceived {
                donor_basis: Some(dec!(25.00)),
                donor_acquired_at: None,
                fmv_at_gift: dec!(30.00),
            },
        }),
    );
    let st = project(
        &[tin, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        0,
        "valid ClassifyInbound must not produce a DecisionConflict: {:?}",
        st.blockers
    );
    assert_eq!(st.lots.len(), 1, "gift lot must be created");
    assert!(st.income_recognized.is_empty(), "gift is not income");
}

// ── Test 9: ManualFmv valid happy path (FmvMissing Income) ────────────────────────────────────

/// Valid ManualFmv against an Income event with usd_fmv=None/FmvStatus::Missing →
/// FMV override applied; no FmvMissing or DecisionConflict blocker.
#[test]
fn manual_fmv_valid_happy_path_fmv_missing() {
    let income = ev(
        "INCOME-H",
        ts_event(),
        cb_wallet(),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Reward,
            business: false,
        }),
    );
    let income_id = EventId::import(Source::Coinbase, SourceRef::new("INCOME-H"));
    let fmv_dec = dec_ev(
        1,
        ts_decision(),
        EventPayload::ManualFmv(ManualFmv {
            event: income_id,
            usd_fmv: dec!(9999.00),
        }),
    );
    let st = project(
        &[income, fmv_dec],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    assert_eq!(
        count_decision_conflicts(&st),
        0,
        "valid ManualFmv must not produce a DecisionConflict: {:?}",
        st.blockers
    );
    // ManualFmv override applied: FmvMissing blocker should not fire.
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::FmvMissing),
        "FmvMissing must not fire when ManualFmv override is applied: {:?}",
        st.blockers
    );
    assert_eq!(st.income_recognized.len(), 1, "income must be recognized");
    assert_eq!(
        st.income_recognized[0].usd_fmv,
        dec!(9999.00),
        "FMV override must be applied"
    );
}

// ── Test 10: ManualFmv latest-seq-wins preserved ──────────────────────────────────────────────

/// Two ManualFmv decisions for the same valid Income event → NO duplicate blocker;
/// the later (higher seq) value wins.
#[test]
fn manual_fmv_latest_seq_wins_no_duplicate_blocker() {
    let income = ev(
        "INCOME-LW",
        ts_event(),
        cb_wallet(),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Reward,
            business: false,
        }),
    );
    let income_id = EventId::import(Source::Coinbase, SourceRef::new("INCOME-LW"));
    // First decision (seq=1): FMV = $1,000.
    let first = dec_ev(
        1,
        ts_decision(),
        EventPayload::ManualFmv(ManualFmv {
            event: income_id.clone(),
            usd_fmv: dec!(1000.00),
        }),
    );
    // Second decision (seq=2): FMV = $2,000 (later → should win).
    let second = dec_ev(
        2,
        ts_decision(),
        EventPayload::ManualFmv(ManualFmv {
            event: income_id,
            usd_fmv: dec!(2000.00),
        }),
    );
    let st = project(
        &[income, first, second],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // NO DecisionConflict blocker — ManualFmv allows latest-seq-wins without a conflict.
    assert_eq!(
        count_decision_conflicts(&st),
        0,
        "ManualFmv latest-seq-wins must NOT produce a DecisionConflict: {:?}",
        st.blockers
    );
    assert_eq!(st.income_recognized.len(), 1);
    // Later value ($2,000) wins.
    assert_eq!(
        st.income_recognized[0].usd_fmv,
        dec!(2000.00),
        "LATER ManualFmv value must win"
    );
}

// ── Test 11: Void remedy ──────────────────────────────────────────────────────────────────────

/// A bad ReclassifyOutflow (targeting a wrong-type Income event) can be remedied by voiding
/// the decision → after void, no DecisionConflict blocker remains; no disposal or removal.
#[test]
fn void_remedy_clears_bad_reclassify_outflow() {
    let income = ev(
        "INCOME-V",
        ts_event(),
        cb_wallet(),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(10000.00)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Reward,
            business: false,
        }),
    );
    // Also provide a real TransferOut (with holdings) to ensure no unrelated blockers from
    // the outflow path — though we'll filter for DecisionConflict specifically anyway.
    let buy = ev(
        "OUT-V-BUY",
        ts_event(),
        cb_wallet(),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT-V",
        ts_event2(),
        cb_wallet(),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    // Bad decision: seq=1, targeting INCOME-V (wrong type for ReclassifyOutflow).
    let income_id = EventId::import(Source::Coinbase, SourceRef::new("INCOME-V"));
    let bad_decision_id = EventId::decision(1);
    let bad = LedgerEvent {
        id: bad_decision_id.clone(),
        utc_timestamp: ts_decision(),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: income_id,
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(50.00),
            fee_usd: None,
            donee: None,
        }),
    };
    // Void the bad decision: seq=2.
    let void_it = dec_ev(
        2,
        ts_decision(),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: bad_decision_id,
        }),
    );

    let st = project(
        &[income, buy, out, bad, void_it],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // After void, no DecisionConflict from the bad ReclassifyOutflow.
    assert_eq!(
        count_decision_conflicts(&st),
        0,
        "voided bad decision must not leave a DecisionConflict: {:?}",
        st.blockers
    );
    assert!(st.disposals.is_empty(), "no disposal expected");
    assert!(st.removals.is_empty(), "no removal expected");
}
