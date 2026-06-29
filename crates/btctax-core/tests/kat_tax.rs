use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn wal() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn ev(src_ref: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: p,
    }
}
// Duplicated per M4: each tests/*.rs file is a separate crate; shared helpers require a common module.
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: p,
    }
}

#[test]
fn buy_then_sell_one_year_one_day_is_long_term() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = ev(
        "SELL",
        datetime!(2026-03-02 00:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 100_000,
            usd_proceeds: dec!(100.50),
            fee_usd: dec!(0.50),
            kind: DisposeKind::Sell,
        }),
    );
    let st = project(
        &[buy, sell],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.disposals.len(), 1);
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.term, Term::LongTerm);
    assert_eq!(leg.proceeds, dec!(100.00)); // 100.50 gross − 0.50 fee (TP2)
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.gain, dec!(40.00));
    assert!(st.holdings_by_wallet.is_empty());
}

#[test]
fn same_day_sell_is_short_term() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 09:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = ev(
        "SELL",
        datetime!(2025-03-01 17:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 40_000,
            usd_proceeds: dec!(30.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let st = project(
        &[buy, sell],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.disposals[0].legs[0].term, Term::ShortTerm);
    assert_eq!(st.holdings_by_wallet[&wal()], 60_000); // partial: 100k − 40k remains, same LotId
    assert_eq!(st.lots.len(), 1);
}

#[test]
fn income_creates_fmv_basis_lot_and_records_income() {
    let inc = ev(
        "INC",
        datetime!(2025-05-01 00:00:00 UTC),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: Some(dec!(50.00)),
            fmv_status: FmvStatus::PriceDataset,
            kind: IncomeKind::Interest,
            business: false,
        }),
    );
    let st = project(
        &[inc],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.income_recognized.len(), 1);
    assert_eq!(st.income_recognized[0].usd_fmv, dec!(50.00));
    assert_eq!(st.lots[0].usd_basis, dec!(50.00));
    assert_eq!(st.lots[0].basis_source, BasisSource::FmvAtIncome);
}

#[test]
fn income_missing_fmv_creates_lot_but_blocks_and_gates_downstream() {
    let inc = ev(
        "INC",
        datetime!(2025-05-01 00:00:00 UTC),
        EventPayload::Income(Income {
            sat: 100_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Mining,
            business: true,
        }),
    );
    let sell = ev(
        "SELL",
        datetime!(2025-06-01 00:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 100_000,
            usd_proceeds: dec!(70.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let st = project(
        &[inc, sell],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::FmvMissing)); // both the income AND the downstream disposal gate
    assert_eq!(st.holdings_by_wallet.get(&wal()), None); // sats existed for conservation, then disposed
    assert!(st.income_recognized.is_empty()); // no recognized income amount while FMV missing
}

#[test]
fn oversell_raises_uncovered_disposal_and_never_panics() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 10_000,
            usd_cost: dec!(6.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = ev(
        "SELL",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 50_000,
            usd_proceeds: dec!(40.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let st = project(
        &[buy, sell],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UncoveredDisposal));
    assert!(st.lots.iter().all(|l| l.remaining_sat >= 0)); // no negative remainder
}

// ── Task 8: transfers & reconciliation (TP7) ────────────────────────────────────────────────

#[test]
fn unclassified_transfer_out_moves_lots_to_pending() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let st = project(
        &[buy, out],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.pending_reconciliation.len(), 1);
    assert!(st.holdings_by_wallet.is_empty());
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnmatchedOutflows));
}

#[test]
fn transfer_link_relocates_lots_non_taxably_carrying_basis_and_hp() {
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 01:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    };
    let link = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Swan,
                SourceRef::new("IN"),
            )),
        }),
    );
    let st = project(
        &[buy, out, in_ev, link],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.disposals.is_empty() && st.removals.is_empty()); // non-taxable (TP7)
    assert_eq!(st.holdings_by_wallet[&cold], 100_000);
    assert_eq!(st.lots[0].acquired_at, time::macros::date!(2025 - 03 - 01)); // HP carries
    assert_eq!(st.lots[0].usd_basis, dec!(60.00));
    assert!(st.pending_reconciliation.is_empty());
    // No unexpected blockers: confirmed link must leave no unmatched-outflow, uncovered-disposal,
    // or unknown-basis-inbound noise.
    assert!(st.blockers.iter().all(|b| {
        b.kind != BlockerKind::UnmatchedOutflows
            && b.kind != BlockerKind::UncoveredDisposal
            && b.kind != BlockerKind::UnknownBasisInbound
    }));
    // Self-transfer does NOT increment sigma_in — only externally-sourced acquisitions count (FR9).
    assert_eq!(st.stats.sigma_in, 100_000); // from the BUY Acquire only
}

#[test]
fn unclassified_inbound_is_blocker_without_creating_a_lot() {
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Gemini, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    };
    let st = project(
        &[in_ev],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert!(st.lots.is_empty() && st.holdings_by_wallet.is_empty());
}

#[test]
fn classify_inbound_as_income_creates_fmv_lot() {
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Gemini, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    };
    let cls = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Gemini, SourceRef::new("IN")),
            as_: InboundClass::Income {
                kind: IncomeKind::Reward,
                fmv: Some(dec!(45.00)),
                business: false,
            },
        }),
    );
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.income_recognized[0].usd_fmv, dec!(45.00));
    assert_eq!(st.lots[0].usd_basis, dec!(45.00));
}

// ── M-1/M-2/N-1: GiftReceived baseline + malformed-link + sigma_in ──────────────────────────

/// GiftReceived with known donor_basis: carryover lot, GiftCarryover basis_source,
/// donor_acquired_at carried, sigma_in += sat. Baseline before Task 10 dual-basis overlay.
#[test]
fn gift_received_fold_creates_carryover_lot_and_counts_sigma_in() {
    let donor_date = time::macros::date!(2023 - 06 - 15);
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Gemini, SourceRef::new("GIFT")),
        utc_timestamp: datetime!(2025-07-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 50_000,
            src_addr: None,
            txid: None,
        }),
    };
    let cls = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Gemini, SourceRef::new("GIFT")),
            as_: InboundClass::GiftReceived {
                donor_basis: Some(dec!(25.00)),
                donor_acquired_at: Some(donor_date),
                fmv_at_gift: dec!(30.00),
            },
        }),
    );
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.lots.len(), 1);
    let lot = &st.lots[0];
    assert_eq!(lot.usd_basis, dec!(25.00));
    assert_eq!(lot.basis_source, BasisSource::GiftCarryover);
    assert_eq!(lot.donor_acquired_at, Some(donor_date));
    assert_eq!(lot.remaining_sat, 50_000);
    // No UnknownBasisInbound — donor_basis is known.
    assert!(st
        .blockers
        .iter()
        .all(|b| b.kind != BlockerKind::UnknownBasisInbound));
    assert_eq!(st.stats.sigma_in, 50_000); // GiftReceived counts as externally-sourced (FR9)
}

/// I-1: a TransferLink whose in-event has wallet:None must NOT silently discard the inbound sats.
/// Expected: DecisionConflict hard blocker on the link; UnknownBasisInbound on the in-event
/// (which surfaces as Op::UnknownInbound because it is NOT consumed); out-event falls to PendingOut.
#[test]
fn malformed_transfer_link_no_dest_wallet_raises_blocker_not_silent_drop() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    // TransferIn with wallet:None — no destination wallet can be resolved from this event.
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 01:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::TransferIn(TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    };
    let link = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Swan,
                SourceRef::new("IN"),
            )),
        }),
    );
    let st = project(
        &[buy, out, in_ev, link],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // Hard blocker: malformed link raises DecisionConflict.
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict),
        "expected DecisionConflict blocker for unroutable link"
    );
    // Inbound sats NOT silently Skipped: in-event becomes Op::UnknownInbound → UnknownBasisInbound fires.
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::UnknownBasisInbound),
        "expected UnknownBasisInbound blocker — inbound must not be silently dropped"
    );
    // No residual lots: buy's sats consumed into pending, in-event (UnknownInbound) creates no lot.
    assert!(st.lots.is_empty());
}

// ── Task 8 closures: I-2, M-3 ───────────────────────────────────────────────────────────────

/// I-2: GiftReceived with unknown donor_basis (None): blocker must fire, lot still created for
/// sat conservation with basis_pending=true and usd_basis=Usd::ZERO.
#[test]
fn gift_received_no_donor_basis_raises_unknown_basis_blocker() {
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Gemini, SourceRef::new("GIFT")),
        utc_timestamp: datetime!(2025-07-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 50_000,
            src_addr: None,
            txid: None,
        }),
    };
    let cls = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Gemini, SourceRef::new("GIFT")),
            as_: InboundClass::GiftReceived {
                donor_basis: None,
                donor_acquired_at: Some(time::macros::date!(2023 - 06 - 15)),
                fmv_at_gift: dec!(30.00),
            },
        }),
    );
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // UnknownBasisInbound blocker must fire.
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::UnknownBasisInbound),
        "expected UnknownBasisInbound blocker for gift with donor_basis=None"
    );
    // Lot is still created for sat conservation.
    assert_eq!(st.lots.len(), 1);
    let lot = &st.lots[0];
    assert!(lot.basis_pending, "basis_pending must be true");
    assert_eq!(lot.usd_basis, Usd::ZERO, "usd_basis must be ZERO");
    assert_eq!(lot.remaining_sat, 50_000);
    // Holdings reflect received sats.
    assert_eq!(st.holdings_by_wallet[&wal()], 50_000);
}

// ── Task 9: gift/donation outbound (TP10) — ReclassifyOutflow ───────────────────────────────

/// Gift out: Removal with zero recognized gain, per-lot basis, FMV-at-transfer, and ST/LT term.
/// No Disposal emitted (TP10: gift is a non-recognition event for the donor).
#[test]
fn gift_out_is_zero_gain_with_basis_fmv_and_term() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2026-06-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-06-15 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.disposals.is_empty());
    let leg = &st.removals[0].legs[0];
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.fmv_at_transfer, dec!(150.00));
    assert_eq!(leg.term, Term::LongTerm); // bought 2025-03-01, gifted 2026-06-01
    assert_eq!(st.removals[0].kind, RemovalKind::Gift);
}

/// Donation over $5k: appraisal_required flag passes through; Removal with zero recognized gain.
#[test]
fn donation_over_5k_flags_appraisal_required() {
    let buy = ev(
        "BUY",
        datetime!(2025-01-05 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(1000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2026-02-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-02-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Donate {
                appraisal_required: true,
            },
            principal_proceeds_or_fmv: dec!(60000.00),
            fee_usd: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.removals[0].appraisal_required);
    assert_eq!(st.removals[0].kind, RemovalKind::Donation);
    assert!(st.disposals.is_empty());
}

/// ReclassifyOutflow{as: Dispose} creates a real Disposal (proceeds−fee, basis, gain, ST/LT).
/// Reuses the existing Task 5 Dispose fold path.
#[test]
fn reclassify_outflow_as_dispose_creates_disposal_with_gain() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2026-06-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-06-15 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: Some(dec!(1.00)),
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.removals.is_empty());
    assert_eq!(st.disposals.len(), 1);
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.proceeds, dec!(149.00)); // 150.00 gross − 1.00 fee (TP2)
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.gain, dec!(89.00));
    assert_eq!(leg.term, Term::LongTerm); // bought 2025-03-01, sold 2026-06-01 (>1yr)
}

/// M-3: Two TransferLink decisions both targeting the same in-event: exactly one
/// DecisionConflict blocker on the duplicate (second link); in-event consumed only once
/// (first link wins, no double-consumption).
#[test]
fn duplicate_transfer_link_same_in_event_is_decision_conflict() {
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 01:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    };
    // First TransferLink: valid link to the in-event.
    let link1 = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Swan,
                SourceRef::new("IN"),
            )),
        }),
    );
    // Second TransferLink: duplicate targeting the same in-event.
    let link2 = dec_ev(
        2,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Swan,
                SourceRef::new("IN"),
            )),
        }),
    );
    let st = project(
        &[buy, out, in_ev, link1, link2],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // Exactly one DecisionConflict blocker for the duplicate.
    let decision_conflicts: Vec<_> = st
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::DecisionConflict)
        .collect();
    assert_eq!(
        decision_conflicts.len(),
        1,
        "expected exactly one DecisionConflict blocker for duplicate link"
    );
    // In-event consumed only once: holdings in cold wallet should reflect 100_000 (first link wins).
    assert_eq!(st.holdings_by_wallet[&cold], 100_000);
    // Single lot, transferred to cold wallet (basis carried).
    assert_eq!(st.lots.len(), 1);
    let lot = &st.lots[0];
    assert_eq!(lot.wallet, cold);
    assert_eq!(lot.remaining_sat, 100_000);
    assert_eq!(lot.usd_basis, dec!(60.00));
    // No disposals or removals: non-taxable transfer.
    assert!(st.disposals.is_empty() && st.removals.is_empty());
}

// ── Task 10: received-gift dual basis (TP11, §1015(a) + §1223(2) tacking) ───────────────────

fn gift_lot(
    donor_basis: Option<rust_decimal::Decimal>,
    donor_acq: Option<time::Date>,
    fmv_at_gift: rust_decimal::Decimal,
    recv: time::OffsetDateTime,
) -> Vec<LedgerEvent> {
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("GIN")),
        utc_timestamp: recv,
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 100_000,
            src_addr: None,
            txid: None,
        }),
    };
    let cls = dec_ev(
        1,
        datetime!(2026-12-31 00:00:00 UTC),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Swan, SourceRef::new("GIN")),
            as_: InboundClass::GiftReceived {
                donor_basis,
                donor_acquired_at: donor_acq,
                fmv_at_gift,
            },
        }),
    );
    vec![in_ev, cls]
}

fn sell(ts: time::OffsetDateTime, proceeds: rust_decimal::Decimal) -> LedgerEvent {
    ev(
        "S",
        ts,
        EventPayload::Dispose(Dispose {
            sat: 100_000,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}

/// TP11 case 1: FMV-at-gift ≥ donor basis → single carryover, no dual, §1223(2) tacking from donor_acquired_at.
#[test]
fn tp11_case_no_dual_basis_fmv_ge_donor_basis_tacks() {
    let mut evs = gift_lot(
        Some(dec!(40.00)),
        Some(time::macros::date!(2024 - 01 - 01)),
        dec!(60.00),
        datetime!(2025-06-01 00:00:00 UTC),
    );
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(80.00)));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.basis, dec!(40.00));
    assert_eq!(leg.gain, dec!(40.00));
    assert_eq!(leg.term, Term::LongTerm); // tacks from donor 2024-01-01
    assert_eq!(leg.gift_zone, None);
}

/// TP11 case 2: FMV-at-gift < donor basis → dual basis; proceeds > gain_basis → Gain zone, §1223(2) tacking.
#[test]
fn tp11_case_gain_zone_with_tacking() {
    let mut evs = gift_lot(
        Some(dec!(100.00)),
        Some(time::macros::date!(2024 - 01 - 01)),
        dec!(60.00), // dual: fmv < basis
        datetime!(2025-06-01 00:00:00 UTC),
    );
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(120.00))); // proceeds > gain basis (100)
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::Gain));
    assert_eq!(leg.basis, dec!(100.00));
    assert_eq!(leg.gain, dec!(20.00));
    assert_eq!(leg.term, Term::LongTerm); // tacks from donor 2024-01-01
}

/// TP11 case 3: proceeds < loss_basis (FMV-at-gift) → Loss zone; HP from gift date (no tacking on loss side).
#[test]
fn tp11_case_loss_zone_hp_from_gift_date() {
    let mut evs = gift_lot(
        Some(dec!(100.00)),
        Some(time::macros::date!(2024 - 01 - 01)),
        dec!(60.00), // dual
        datetime!(2025-06-01 00:00:00 UTC),
    );
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(40.00))); // proceeds < loss basis (60)
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::Loss));
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.gain, dec!(-20.00));
    assert_eq!(leg.term, Term::ShortTerm); // HP from gift date 2025-06-01
}

/// TP11 case 4: proceeds between loss_basis and gain_basis → NoGainNoLoss; gain = 0.
#[test]
fn tp11_case_middle_zone_zero_gain() {
    let mut evs = gift_lot(
        Some(dec!(100.00)),
        Some(time::macros::date!(2024 - 01 - 01)),
        dec!(60.00), // dual: loss=60, gain=100
        datetime!(2025-06-01 00:00:00 UTC),
    );
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(80.00))); // 60 <= 80 <= 100
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::NoGainNoLoss));
    assert_eq!(leg.gain, dec!(0));
}

/// TP11 GiftFmvFallback: donor_basis=None with a known donor_acquired_at and an available price →
/// basis = FMV of sat at donor acquisition date; basis_source = GiftFmvFallback; no blocker.
#[test]
fn tp11_unknown_donor_basis_uses_fmv_at_donor_acquisition_date() {
    let mut prices = StaticPrices::default();
    prices
        .0
        .insert(time::macros::date!(2023 - 03 - 15), dec!(28000.00)); // BTC/USD at donor acq date
    let mut evs = gift_lot(
        None,
        Some(time::macros::date!(2023 - 03 - 15)),
        dec!(60.00),
        datetime!(2025-06-01 00:00:00 UTC),
    );
    evs.push(sell(datetime!(2025-07-01 00:00:00 UTC), dec!(100.00)));
    let st = project(&evs, &prices, &ProjectionConfig::default());
    // 100_000 sat @ 28000/BTC = 28.00 basis (GiftFmvFallback)
    assert_eq!(st.disposals[0].legs[0].basis, dec!(28.00));
    assert_eq!(
        st.disposals[0].legs[0].basis_source,
        BasisSource::GiftFmvFallback
    );
}

/// TP11 unknown basis + unknown date: both indeterminate → UnknownBasisInbound blocker;
/// sat-bearing lot still created for conservation (basis_pending=true).
#[test]
fn tp11_unknown_donor_basis_and_date_creates_basis_pending_lot() {
    let evs = gift_lot(None, None, dec!(60.00), datetime!(2025-06-01 00:00:00 UTC));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert_eq!(st.lots[0].remaining_sat, 100_000); // sat-bearing lot exists (conservation)
    assert!(st.lots[0].basis_pending);
}

/// I-1 Task 9: Two ReclassifyOutflow decisions both targeting the same transfer_out_event:
/// exactly one DecisionConflict blocker on the duplicate (second decision); outflow classified
/// once by first decision (no double-processing).
#[test]
fn duplicate_reclassify_outflow_same_target_is_decision_conflict() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    // First ReclassifyOutflow: classify as GiftOut.
    let recl1 = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(100.00),
            fee_usd: None,
        }),
    );
    // Second ReclassifyOutflow: duplicate targeting the same transfer_out_event.
    let recl2 = dec_ev(
        2,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: None,
        }),
    );
    let st = project(
        &[buy, out, recl1, recl2],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // Exactly one DecisionConflict blocker for the duplicate.
    let decision_conflicts: Vec<_> = st
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::DecisionConflict)
        .collect();
    assert_eq!(
        decision_conflicts.len(),
        1,
        "expected exactly one DecisionConflict blocker for duplicate reclassify"
    );
    // Outflow classified once by first decision: one Removal created.
    assert_eq!(st.removals.len(), 1);
    let removal = &st.removals[0];
    // First decision (GiftOut) wins.
    assert_eq!(removal.kind, RemovalKind::Gift);
    assert_eq!(removal.legs[0].basis, dec!(60.00));
    assert_eq!(removal.legs[0].fmv_at_transfer, dec!(100.00));
    // No double-processing: only one removal, not two.
    assert!(st.disposals.is_empty());
}

// ── Task 11: self-transfer network fee (TP8) — default (c) and config (b) ──────────────────────

fn cfg_b() -> ProjectionConfig {
    ProjectionConfig {
        self_transfer_fee: btctax_core::project::FeeTreatment::TreatmentB,
        ..ProjectionConfig::default()
    }
}

/// TP8 default (c): fee-sats are NON-TAXABLE; their basis is re-homed onto the surviving relocated lot
/// so the destination holds the FULL $60.00 basis (C1) while holding only 99,800 principal sats.
#[test]
fn self_transfer_fee_default_c_is_non_taxable() {
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 99_800,
            fee_sat: Some(200),
            dest_addr: None,
            txid: None,
        }),
    );
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 01:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 99_800,
            src_addr: None,
            txid: None,
        }),
    };
    let link = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Swan,
                SourceRef::new("IN"),
            )),
        }),
    );
    let st = project(
        &[buy, out, in_ev, link],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.disposals.is_empty()); // (c): no recognition
    assert_eq!(st.holdings_by_wallet[&cold], 99_800); // 100_000 − 200 fee
                                                      // C1: the destination lot carries the FULL $60.00 basis (the 200 fee-sats' $0.12 re-homed, NOT dropped to $59.88).
    assert_eq!(st.lots.len(), 1);
    assert_eq!(st.lots[0].wallet, cold);
    assert_eq!(st.lots[0].remaining_sat, 99_800);
    assert_eq!(st.lots[0].usd_basis, dec!(60.00));
    assert_eq!(st.lots[0].basis_source, BasisSource::CarriedFromTransfer);
    assert_eq!(st.stats.fee_sats_consumed, 200); // FR9: fee-sats' sole conservation home
}

/// Gift/donation fee BY ANALOGY (§7.3): under (c) the fee-sats' basis is re-homed onto the removal leg,
/// so the donee's carried-over basis is the FULL $60.00 (not $59.88). C1 analogue.
#[test]
fn gift_out_fee_default_c_carries_full_basis_onto_the_removal() {
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2026-06-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 99_800,
            fee_sat: Some(200),
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-06-15 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.disposals.is_empty()); // (c): non-recognition on the fee
    assert_eq!(st.removals.len(), 1);
    let removal_basis: rust_decimal::Decimal = st.removals[0].legs.iter().map(|l| l.basis).sum();
    let removal_sat: i64 = st.removals[0].legs.iter().map(|l| l.sat).sum();
    assert_eq!(removal_sat, 99_800); // principal only; fee burned
    assert_eq!(removal_basis, dec!(60.00)); // FULL basis carries (200 fee-sats' $0.12 re-homed, not dropped)
    assert_eq!(st.stats.fee_sats_consumed, 200);
}

/// TP8 config (b): fee-sats ARE a taxable mini-disposition — a recognition record with `fee_mini_disposition=true`.
/// CONTRAST with (c): the fee basis rides the mini-disposition, so the destination lot is $59.88 NOT $60.00.
#[test]
fn self_transfer_fee_config_b_is_a_mini_disposition_recognition_record() {
    let mut prices = StaticPrices::default();
    prices
        .0
        .insert(time::macros::date!(2025 - 04 - 01), dec!(50000.00));
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let buy = ev(
        "BUY",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OUT",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 99_800,
            fee_sat: Some(200),
            dest_addr: None,
            txid: None,
        }),
    );
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-04-01 01:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 99_800,
            src_addr: None,
            txid: None,
        }),
    };
    let link = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::TransferLink(TransferLink {
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                Source::Swan,
                SourceRef::new("IN"),
            )),
        }),
    );
    let st = project(&[buy, out, in_ev, link], &prices, &cfg_b());
    let mini: Vec<_> = st
        .disposals
        .iter()
        .filter(|d| d.fee_mini_disposition)
        .collect();
    assert_eq!(mini.len(), 1); // recognition record for the 200 fee-sats
    assert_eq!(st.holdings_by_wallet[&cold], 99_800);
    // (b) CONTRAST with (c): the fee basis rides the mini-disposition, so the destination lot is the
    // principal-only basis $59.88 (NOT re-homed). The mini-disposition is excluded from FR9 Σdisposed.
    assert_eq!(
        st.lots.iter().find(|l| l.wallet == cold).unwrap().usd_basis,
        dec!(59.88)
    );
    assert_eq!(st.stats.fee_sats_consumed, 200);
}
