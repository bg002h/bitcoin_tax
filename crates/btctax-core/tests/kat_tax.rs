use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{conservation_report, project, FeeTreatment, ProjectionConfig};
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
            donee: None,
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
            donee: None,
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
            donee: None,
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
            donee: None,
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
            donee: None,
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
            donee: None,
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

/// M1 regression — cross-lot (c) fee: principal exhausts a NORMAL lot; fee crosses into a
/// DUAL-BASIS received-gift lot. Survivor (`relocated.last()`) is the NORMAL lot.
///
/// Setup:
///   Lot A (FIFO first, normal buy): 500 sats @ $30.00 in wal().
///   Lot B (FIFO second, dual-basis gift): 500 sats, donor_basis=$100, fmv_at_gift=$60 in wal().
///   Transfer: 500 principal (exhausts Lot A exactly) + 200 fee (crosses into Lot B).
///   Fee carry from Lot B: gain_basis=$40 (=$100×200/500), loss_basis=Some($24) (=$60×200/500).
///
/// Assertions:
///   (1) C1 intact: survivor usd_basis = $30 + $40 = $70 (full gain-basis carries).
///   (2) dual_loss_basis stays None — NORMAL lot NOT promoted to §1015(a) dual-basis.
///   (3) later sale routes through normal (gift_zone=None) single-basis path, not four-zone logic.
#[test]
fn self_transfer_fee_c_cross_lot_normal_survivor_stays_non_dual() {
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };

    // Lot A: normal purchased lot, 500 sats @ $30.00 total, into wal() on 2025-01-01 (FIFO slot 0).
    let buy = ev(
        "BUY",
        datetime!(2025-01-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 500,
            usd_cost: dec!(30.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );

    // Lot B: dual-basis received-gift lot, 500 sats into wal() on 2025-02-01 (FIFO slot 1).
    // donor_basis=$100 > fmv_at_gift=$60 → dual; usd_basis=$100, dual_loss_basis=Some($60).
    let gift_in = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("GIN")),
        utc_timestamp: datetime!(2025-02-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 500,
            src_addr: None,
            txid: None,
        }),
    };
    let classify_gift = dec_ev(
        1,
        datetime!(2026-12-31 00:00:00 UTC),
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Swan, SourceRef::new("GIN")),
            as_: InboundClass::GiftReceived {
                donor_basis: Some(dec!(100.00)),
                donor_acquired_at: Some(time::macros::date!(2024 - 01 - 01)),
                fmv_at_gift: dec!(60.00), // fmv < donor_basis → dual-basis lot
            },
        }),
    );

    // Self-transfer: 500 principal (exhausts Lot A exactly) + 200 fee (crosses into Lot B).
    // After principal: relocated=[Lot A (500 sats, $30)]. Fee: 200 from Lot B → carry gain=$40, loss=Some($24).
    let out = ev(
        "OUT",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 500,
            fee_sat: Some(200),
            dest_addr: None,
            txid: None,
        }),
    );
    let in_ev = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("IN")),
        utc_timestamp: datetime!(2025-03-01 01:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::TransferIn(TransferIn {
            sat: 500,
            src_addr: None,
            txid: None,
        }),
    };
    let link = dec_ev(
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

    // Phase 1: check survivor lot state (no sale yet).
    let evs_no_sale = [
        buy.clone(),
        gift_in.clone(),
        classify_gift.clone(),
        out.clone(),
        in_ev.clone(),
        link.clone(),
    ];
    let st = project(
        &evs_no_sale,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.disposals.is_empty(), "(c): no recognition event on fee");
    let cold_lot = st
        .lots
        .iter()
        .find(|l| l.wallet == cold)
        .expect("survivor lot must exist in cold wallet after transfer");
    // (1) C1: gain_basis carries fully — Lot A $30.00 + fee contribution ($100×200/500) = $40.00 → $70.00.
    assert_eq!(
        cold_lot.usd_basis,
        dec!(70.00),
        "C1: full gain-basis must carry onto survivor (Lot A $30 + fee $40)"
    );
    // (2) dual_loss_basis stays None — NORMAL lot must NOT be promoted to §1015(a) dual-basis.
    assert_eq!(
        cold_lot.dual_loss_basis, None,
        "survivor must remain non-dual (§1015 misclassification guard)"
    );

    // Phase 2: sell all survivor sats; verify disposal uses normal (single-basis) path.
    let sale = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("SELL")),
        utc_timestamp: datetime!(2027-01-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::Dispose(Dispose {
            sat: 500,
            usd_proceeds: dec!(200.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };
    let evs_with_sale = [buy, gift_in, classify_gift, out, in_ev, link, sale];
    let st2 = project(
        &evs_with_sale,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let real_disposals: Vec<_> = st2
        .disposals
        .iter()
        .filter(|d| !d.fee_mini_disposition)
        .collect();
    assert_eq!(
        real_disposals.len(),
        1,
        "exactly one real disposal from the sale"
    );
    let leg = &real_disposals[0].legs[0];
    // (3) Normal single-basis path: gift_zone must be None (NOT routed through §1015 four-zone logic).
    assert_eq!(
        leg.gift_zone, None,
        "survivor sale must use normal (non-dual) path, not §1015(a) four-zone logic"
    );
    assert_eq!(
        leg.basis,
        dec!(70.00),
        "basis must be the carried gain_basis (C1)"
    );
    assert_eq!(
        leg.gain,
        dec!(130.00),
        "gain = proceeds $200 − basis $70 via normal path"
    );
}

// ── I-1 fix: ReclassifyOutflow{Dispose} with on-chain fee_sat — whole-branch Important ──────
// The fee-sats MUST be consumed (not left in holdings), conservation must balance honestly,
// and the TP8 treatment (c/b) applies to the disposal just as it does for gift/SelfTransfer.

/// I-1 (c): TransferOut{sat=99_800, fee_sat=Some(200)} reclassified as Dispose.
/// Under (c): fee-sats consumed; their basis re-homed onto the last disposal leg so the
/// reported basis = full $60.00 (not $59.88), holdings = 0, conservation balanced.
#[test]
fn reclassify_dispose_fee_sat_treatment_c_conservation_honest() {
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
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: Some(dec!(1.00)),
            donee: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );

    // (1) fee-sats consumed — holdings NOT overstated.
    assert!(
        st.holdings_by_wallet.is_empty(),
        "holdings must be empty: fee-sats consumed, not stranded in the pool"
    );
    assert_eq!(
        st.stats.fee_sats_consumed, 200,
        "FR9: 200 fee-sats in sole conservation home"
    );

    // (2) conservation honest — balanced and no uncovered disposal.
    let report = conservation_report(&st);
    assert!(!report.has_uncovered, "no uncovered disposals");
    assert!(
        report.balanced,
        "conservation must be balanced (fee-sats consumed, not phantom-held): {report:?}"
    );
    assert_eq!(report.sigma_in, 100_000);
    assert_eq!(report.sigma_disposed, 99_800); // principal disposal legs only
    assert_eq!(report.sigma_fee_sats, 200); // fee-sat sole conservation home
    assert_eq!(report.sigma_held, 0);

    // (3) TP8 (c): fee-sat basis rolled onto last disposal leg.
    // Buy: 100k sats @ $60.00. Principal 99_800 → raw basis $59.88 (pro-rata).
    // Fee 200 sats → carry.gain_basis = $0.12, re-homed → leg.basis = $60.00.
    // Net proceeds: $150.00 − $1.00 fee_usd = $149.00. Gain: $149.00 − $60.00 = $89.00.
    assert_eq!(st.disposals.len(), 1);
    assert!(!st.disposals[0].fee_mini_disposition);
    let leg = &st.disposals[0].legs[0];
    assert_eq!(
        leg.basis,
        dec!(60.00),
        "(c): fee-sat basis ($0.12) re-homed onto last disposal leg → full $60.00 basis"
    );
    assert_eq!(
        leg.proceeds,
        dec!(149.00),
        "net proceeds = $150.00 gross − $1.00 fee_usd"
    );
    assert_eq!(
        leg.gain,
        dec!(89.00),
        "gain = $149.00 − $60.00 (fee basis re-homed)"
    );
}

/// I-1 (b): same TransferOut reclassified as Dispose under TreatmentB.
/// Under (b): fee-sats become a mini-disposition recognition record; conservation balanced;
/// principal disposal leg keeps principal-only basis ($59.88, not re-homed).
#[test]
fn reclassify_dispose_fee_sat_treatment_b_mini_disposition() {
    let mut prices = StaticPrices::default();
    prices
        .0
        .insert(time::macros::date!(2026 - 06 - 01), dec!(50000.00));
    let cfg = ProjectionConfig {
        self_transfer_fee: FeeTreatment::TreatmentB,
        ..ProjectionConfig::default()
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
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: Some(dec!(1.00)),
            donee: None,
        }),
    );
    let st = project(&[buy, out, recl], &prices, &cfg);

    // (b): fee-sats are a taxable mini-disposition recognition record.
    let mini: Vec<_> = st
        .disposals
        .iter()
        .filter(|d| d.fee_mini_disposition)
        .collect();
    assert_eq!(
        mini.len(),
        1,
        "(b): exactly one fee mini-disposition for the 200 fee-sats"
    );

    // Conservation balanced (fee-sats in fee_sats_consumed, NOT double-counted in sigma_disposed).
    let report = conservation_report(&st);
    assert!(
        report.balanced,
        "conservation balanced under (b): {report:?}"
    );
    assert_eq!(report.sigma_fee_sats, 200);
    assert_eq!(report.sigma_disposed, 99_800); // principal disposal legs only (mini excluded)
    assert!(st.holdings_by_wallet.is_empty());

    // (b) CONTRAST with (c): basis NOT re-homed onto principal disposal leg.
    let principal_disposal: &_ = st
        .disposals
        .iter()
        .find(|d| !d.fee_mini_disposition)
        .expect("one real disposal");
    let leg = &principal_disposal.legs[0];
    assert_eq!(
        leg.basis,
        dec!(59.88),
        "(b): principal leg basis stays at $59.88 (fee basis rode the mini-disposition)"
    );
}

// ── Task slug: §170(f)(11)(C) qualified-appraisal advisory KATs ──────────────────────────────

// Helper: build a single-lot Donate via pre/post reclassify-outflow→donate.
// `buy_ts`: acquisition timestamp; `out_ts`: TransferOut timestamp; `recl_ts`: decision timestamp;
// `sat`: sats for buy+out; `cost`: buy usd_cost; `fmv`: donation FMV; `appraisal_required`: manual flag.
fn donate_single(
    buy_ts: time::OffsetDateTime,
    out_ts: time::OffsetDateTime,
    recl_ts: time::OffsetDateTime,
    sat: i64,
    cost: Usd,
    fmv: Usd,
    appraisal_required: bool,
) -> Vec<LedgerEvent> {
    vec![
        ev(
            "B",
            buy_ts,
            EventPayload::Acquire(Acquire {
                sat,
                usd_cost: cost,
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "O",
            out_ts,
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            1,
            recl_ts,
            EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("O")),
                as_: OutflowClass::Donate { appraisal_required },
                principal_proceeds_or_fmv: fmv,
                fee_usd: None,
                donee: None,
            }),
        ),
    ]
}

/// (a) LT $60k FMV / $5k basis → FLAGGED (proxy = FMV $60k; the case the AND rule missed).
/// The proxy for a LT lot is FMV (the LT-capital-gain deduction), not basis.
#[test]
fn qualified_appraisal_lt_60k_fmv_5k_basis_flagged() {
    // Buy 2025-01-05 (LT when donated 2026-03-01, since 2026-03-01 > one_year_after(2025-01-05)=2026-01-05).
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(5000.00),  // basis = $5k
        dec!(60000.00), // FMV = $60k
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let notes: Vec<_> = st
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::QualifiedAppraisalNote)
        .collect();
    assert_eq!(
        notes.len(),
        1,
        "LT $60k FMV / $5k basis must emit QualifiedAppraisalNote (proxy = FMV $60k > $5k)"
    );
    // Verify the detail contains the proxy and key statutory references.
    let detail = &notes[0].detail;
    assert!(
        detail.contains("60000.00"),
        "detail must name the deduction proxy; got: {detail}"
    );
    assert!(
        detail.contains("§170(f)(11)(C)"),
        "detail must cite §170(f)(11)(C); got: {detail}"
    );
    assert!(
        detail.contains("CCA 202302012"),
        "detail must cite CCA 202302012; got: {detail}"
    );
    assert!(
        detail.contains("§170(e)"),
        "detail must cite §170(e); got: {detail}"
    );
    assert!(
        detail.contains("§1221(a)(1)"),
        "detail must cite §1221(a)(1) (character-framed caveat); got: {detail}"
    );
    assert!(
        detail.contains("§170(f)(11)(F)"),
        "detail must cite §170(f)(11)(F) (aggregation caveat); got: {detail}"
    );
    // Removal still created; no disposal; no basis/gain change.
    assert_eq!(st.removals.len(), 1);
    assert_eq!(st.removals[0].kind, RemovalKind::Donation);
    assert!(st.disposals.is_empty());
}

/// (b) ST $10k FMV / $2k basis → NOT flagged (proxy = basis $2k ≤ $5k).
/// For a ST lot the §170(e)-reduced deduction is basis, not FMV.
#[test]
fn qualified_appraisal_st_10k_fmv_2k_basis_not_flagged() {
    // Buy 2025-01-05 → donate 2025-06-01 (ST: 2025-06-01 < 2026-01-05 = one_year_after).
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2025-06-01 00:00:00 UTC),
        datetime!(2025-06-02 00:00:00 UTC),
        100_000_000,
        dec!(2000.00),  // basis = $2k
        dec!(10000.00), // FMV = $10k (irrelevant for ST — proxy uses basis)
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "ST $10k FMV / $2k basis must NOT emit QualifiedAppraisalNote (proxy = basis $2k ≤ $5k)"
    );
}

/// (c) Mixed LT+ST legs: sum > $5k → flagged; sum ≤ $5k → not flagged.
/// Two lots: Lot A (LT, small), Lot B (ST, larger). FIFO consumes A first.
#[test]
fn qualified_appraisal_mixed_legs_above_threshold_flagged() {
    // Lot A: 100_000_000 sats, $5k cost, bought 2025-01-05 → LT when donated 2026-03-01.
    // Lot B: 100_000_000 sats, $2k cost, bought 2025-12-01 → ST when donated 2026-03-01.
    // Total: 200_000_000 sats, FMV = $100k → A's FMV = $50k (LT), B's basis = $2k (ST).
    // Proxy = $50k + $2k = $52k > $5k → FLAGGED.
    let lot_a_buy = ev(
        "BA",
        datetime!(2025-01-05 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(5000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let lot_b_buy = ev(
        "BB",
        datetime!(2025-12-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(2000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OC",
        datetime!(2026-03-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 200_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-03-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OC")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(100000.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[lot_a_buy, lot_b_buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::QualifiedAppraisalNote),
        "mixed LT+ST summing >$5k must emit QualifiedAppraisalNote"
    );
}

#[test]
fn qualified_appraisal_mixed_legs_below_threshold_not_flagged() {
    // Lot A: 1_000_000 sats, $50 cost, LT. Lot B: 9_000_000 sats, $450 cost, ST.
    // FMV = $1k → A's FMV = $100 (LT), B's basis = $450 (ST). Proxy = $550 < $5k → NOT FLAGGED.
    let lot_a_buy = ev(
        "BA",
        datetime!(2025-01-05 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 1_000_000,
            usd_cost: dec!(50.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let lot_b_buy = ev(
        "BB",
        datetime!(2025-12-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 9_000_000,
            usd_cost: dec!(450.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OD",
        datetime!(2026-03-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 10_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-03-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OD")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(1000.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[lot_a_buy, lot_b_buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "mixed LT+ST summing ≤$5k must NOT emit QualifiedAppraisalNote"
    );
}

/// (d) Boundary: exactly $5,000.00 → NOT flagged (strict >); $5,000.01 → flagged.
/// LT lot: proxy = FMV. At-threshold ($5k) must NOT fire; one-cent-over ($5k.01) MUST fire.
#[test]
fn qualified_appraisal_boundary_exactly_5000_not_flagged() {
    // LT donation, FMV = exactly $5,000.00 → proxy = $5,000.00 (NOT > $5,000) → NOT FLAGGED.
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(1000.00),
        dec!(5000.00), // FMV = exactly threshold
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "FMV = exactly $5,000.00 must NOT emit (strict >; exactly-at-threshold is not over)"
    );
}

#[test]
fn qualified_appraisal_boundary_5000_01_flagged() {
    // LT donation, FMV = $5,000.01 → proxy = $5,000.01 (> $5,000) → FLAGGED.
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(1000.00),
        dec!(5000.01), // FMV = one cent over threshold
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::QualifiedAppraisalNote),
        "FMV = $5,000.01 must emit QualifiedAppraisalNote (strict > threshold)"
    );
}

/// (e) QualifiedAppraisalNote is Advisory; a year whose only blocker is this note still
/// yields TaxOutcome::Computed(..) — the advisory MUST NOT gate compute_tax_year.
#[test]
fn qualified_appraisal_note_is_advisory_and_does_not_gate_compute() {
    // Direct severity check (Task 1 KAT mirror).
    assert_eq!(
        BlockerKind::QualifiedAppraisalNote.severity(),
        Severity::Advisory,
        "QualifiedAppraisalNote must be Advisory (never Hard)"
    );

    // Build a state with only a QualifiedAppraisalNote advisory (LT donation, no Hard blockers).
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(5000.00),
        dec!(60000.00),
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // Verify: only advisory blockers, none Hard.
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind.severity() != Severity::Hard),
        "donation-only projection must have no Hard blockers"
    );
    assert_eq!(
        st.blockers
            .iter()
            .filter(|b| b.kind == BlockerKind::QualifiedAppraisalNote)
            .count(),
        1,
        "exactly one QualifiedAppraisalNote advisory expected"
    );

    // Verify compute_tax_year returns Computed for the donation year (2026) despite the advisory.
    use btctax_core::compute_tax_year;
    use btctax_core::{
        Carryforward, FilingStatus, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxOutcome,
        TaxProfile, TaxTable,
    };
    use std::collections::BTreeMap;
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![OrdinaryBracket {
                lower: dec!(0),
                rate: dec!(0.10),
            }],
        },
    );
    let mut ltcg_map = BTreeMap::new();
    ltcg_map.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(40000),
            max_fifteen: dec!(400000),
        },
    );
    let mut tables: BTreeMap<i32, TaxTable> = BTreeMap::new();
    tables.insert(
        2026,
        TaxTable {
            year: 2026,
            source: "SYNTHETIC",
            ordinary,
            ltcg: ltcg_map,
            gift_annual_exclusion: dec!(19000),
            ss_wage_base: dec!(176100),
            gift_lifetime_exclusion: dec!(13_990_000),
        },
    );
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(0),
        magi_excluding_crypto: dec!(0),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    let outcome = compute_tax_year(&events, &st, 2026, Some(&profile), &tables);
    assert!(
        matches!(outcome, TaxOutcome::Computed(_)),
        "Advisory QualifiedAppraisalNote must not gate compute_tax_year; got: {outcome:?}"
    );
}

/// (f) Two qualifying donations → TWO QualifiedAppraisalNote blockers (per-event, not single-fire).
#[test]
fn qualified_appraisal_two_qualifying_donations_emit_two_notes() {
    // Buy 200_000_000 sats (2 BTC) → two separate LT donations of 100_000_000 each.
    let buy = ev(
        "B",
        datetime!(2025-01-05 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 200_000_000,
            usd_cost: dec!(10000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out1 = ev(
        "O1",
        datetime!(2026-03-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let out2 = ev(
        "O2",
        datetime!(2026-03-02 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl1 = dec_ev(
        1,
        datetime!(2026-03-10 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("O1")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(60000.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let recl2 = dec_ev(
        2,
        datetime!(2026-03-10 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("O2")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(60000.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[buy, out1, out2, recl1, recl2],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let note_count = st
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::QualifiedAppraisalNote)
        .count();
    assert_eq!(
        note_count, 2,
        "two qualifying donations must emit TWO QualifiedAppraisalNote blockers (per-event, not single-fire)"
    );
}

/// (g) GiftOut (non-donation removal) never emits QualifiedAppraisalNote.
#[test]
fn qualified_appraisal_gift_out_never_emits_note() {
    let buy = ev(
        "B",
        datetime!(2025-01-05 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(5000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OG",
        datetime!(2026-03-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    // Reclassify as GiftOut (not Donate), FMV = $60k (would be >$5k if it were a donation).
    let recl = dec_ev(
        1,
        datetime!(2026-03-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OG")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(60000.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "GiftOut must NEVER emit QualifiedAppraisalNote (only Donate arm emits it)"
    );
    assert_eq!(st.removals[0].kind, RemovalKind::Gift);
}

/// (h) Decoupling from the user's `appraisal_required` bool.
/// Case 1: proxy>$5k with appraisal_required=false STILL emits the advisory (computed independently).
#[test]
fn qualified_appraisal_proxy_over_5k_with_flag_false_still_emits() {
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(5000.00),
        dec!(60000.00),
        false, // appraisal_required = false
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::QualifiedAppraisalNote),
        "proxy >$5k with appraisal_required=false MUST still emit QualifiedAppraisalNote (independent cross-check)"
    );
    // The manual flag is persisted as-is; the advisory is the independent cross-check.
    assert!(!st.removals[0].appraisal_required);
}

/// (h) Decoupling from the user's `appraisal_required` bool.
/// Case 2: proxy≤$5k with appraisal_required=true does NOT emit (proxy is the sole gate).
#[test]
fn qualified_appraisal_proxy_under_5k_with_flag_true_does_not_emit() {
    // ST donation → proxy = basis = $2k ≤ $5k → NOT FLAGGED even though appraisal_required=true.
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2025-06-01 00:00:00 UTC), // ST
        datetime!(2025-06-02 00:00:00 UTC),
        100_000_000,
        dec!(2000.00), // basis = $2k; proxy = basis = $2k for ST
        dec!(60000.00),
        true, // appraisal_required = true
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "proxy ≤$5k with appraisal_required=true must NOT emit (proxy is the sole gate, not the manual flag)"
    );
    // The manual flag is persisted as-is.
    assert!(st.removals[0].appraisal_required);
}

// ── P2-A: §170(e) claimed_deduction KATs ─────────────────────────────────────────────────────

/// (P2-A-a1) LT-only donation: claimed_deduction = FMV.
/// §170(e)(1)(A): LT capital-gain property → FMV (no reduction; the would-be gain is LTCG).
#[test]
fn claimed_deduction_lt_only_equals_fmv() {
    // Buy 2025-01-05 → donate 2026-03-01 (LT; >1yr). basis=$5k, FMV=$60k.
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(5000.00),  // basis
        dec!(60000.00), // FMV
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(
        st.removals[0].claimed_deduction,
        Some(dec!(60000.00)),
        "LT-only donation: claimed_deduction must equal FMV $60k (no §170(e) reduction)"
    );
}

/// (P2-A-a2) ST-appreciated donation (basis < FMV): claimed_deduction = basis = min(fmv, basis).
/// §170(e)(1)(A): the ST gain would not be LTCG → deduction reduced from FMV to basis.
#[test]
fn claimed_deduction_st_appreciated_equals_basis() {
    // Buy 2025-01-05 → donate 2025-06-01 (ST; <1yr). basis=$2k, FMV=$10k.
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2025-06-01 00:00:00 UTC),
        datetime!(2025-06-02 00:00:00 UTC),
        100_000_000,
        dec!(2000.00),  // basis < FMV → appreciated ST
        dec!(10000.00), // FMV
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(
        st.removals[0].claimed_deduction,
        Some(dec!(2000.00)),
        "ST-appreciated (basis $2k < FMV $10k): claimed_deduction must = basis $2k = min(10k,2k)"
    );
}

/// (P2-A-a3) ST-DEPRECIATED donation (basis > FMV): claimed_deduction = FMV, NOT basis. [R0-C1 lock]
/// §170(e)(1)(A): no would-be gain (loss property) → no reduction → deduction = FMV.
/// min(FMV, basis) correctly yields FMV when basis > FMV.
/// The old proxy used `basis` unconditionally for ST, which OVER-stated for depreciated property.
/// This KAT locks the ST-depreciated behavior:
///   basis=$8k / fmv=$3k → deduction=$3k → trigger does NOT fire (old basis $8k WOULD have).
#[test]
fn claimed_deduction_st_depreciated_equals_fmv_not_basis() {
    // Buy 2025-06-01 → donate 2025-12-01 (ST; <1yr). basis=$8k (high), FMV=$3k (depreciated).
    let events = donate_single(
        datetime!(2025-06-01 00:00:00 UTC),
        datetime!(2025-12-01 00:00:00 UTC),
        datetime!(2025-12-02 00:00:00 UTC),
        100_000_000,
        dec!(8000.00), // basis > FMV → depreciated ST
        dec!(3000.00), // FMV = $3k (below basis)
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // Deduction = min(3000, 8000) = 3000 (FMV), not basis.
    assert_eq!(
        st.removals[0].claimed_deduction,
        Some(dec!(3000.00)),
        "ST-DEPRECIATED (basis $8k > FMV $3k): claimed_deduction must = FMV $3k, NOT basis $8k"
    );
    // ST-depreciated lock: $3k deduction ≤ $5k threshold → trigger must NOT fire.
    // (The old basis-proxy of $8k WOULD have triggered — that was a false positive now fixed.)
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "ST-depreciated deduction $3k (≤ $5k) must NOT trigger QualifiedAppraisalNote \
         (old proxy used basis $8k and would have fired — corrected to min(fmv,basis)=$3k)"
    );
}

/// (P2-A-a4) Mixed LT+ST legs: claimed_deduction = LT→FMV + ST→min(FMV,basis).
/// Same two-lot setup as qualified_appraisal_mixed_legs_above_threshold_flagged.
/// LT lot ($5k basis, $50k FMV portion) → $50k; ST lot ($2k basis, $50k FMV portion) → min($50k,$2k) = $2k.
/// Total claimed_deduction = $52k.
#[test]
fn claimed_deduction_mixed_lt_plus_st() {
    let lot_a_buy = ev(
        "BA",
        datetime!(2025-01-05 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(5000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let lot_b_buy = ev(
        "BB",
        datetime!(2025-12-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(2000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let out = ev(
        "OC",
        datetime!(2026-03-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 200_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        1,
        datetime!(2026-03-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OC")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(100000.00), // total FMV $100k → pro-rata $50k each
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[lot_a_buy, lot_b_buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // LT leg ($50k FMV) → $50k; ST leg ($2k basis, $50k FMV) → min($50k,$2k) = $2k.
    assert_eq!(
        st.removals[0].claimed_deduction,
        Some(dec!(52000.00)),
        "mixed LT+ST: claimed_deduction must = LT-FMV $50k + ST-min($50k,$2k) = $52k"
    );
}

/// (P2-A-b) Gift removal: claimed_deduction = None (gift is not a charitable deduction).
#[test]
fn gift_claimed_deduction_is_none() {
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
            donee: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.removals.len(), 1);
    assert_eq!(st.removals[0].kind, RemovalKind::Gift);
    assert_eq!(
        st.removals[0].claimed_deduction, None,
        "Gift removal must have claimed_deduction = None (not a charitable deduction)"
    );
}

/// (P2-A-c) Appraisal trigger fires off stored claimed_deduction: LT $60k → flagged.
/// Adapts existing appraisal KAT to also assert on the stored field.
#[test]
fn appraisal_trigger_lt_60k_fires_and_claimed_deduction_stored() {
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(5000.00),
        dec!(60000.00),
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // Trigger fires (stored deduction $60k > $5k threshold).
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::QualifiedAppraisalNote),
        "LT $60k claimed_deduction must fire QualifiedAppraisalNote"
    );
    // Stored field = FMV $60k (LT).
    assert_eq!(st.removals[0].claimed_deduction, Some(dec!(60000.00)));
}

/// (P2-A-c) Appraisal trigger: ST $10k/$2k appreciated → NOT flagged.
/// Stored claimed_deduction = min($10k, $2k) = $2k ≤ $5k.
#[test]
fn appraisal_trigger_st_appreciated_not_fired_and_claimed_deduction_stored() {
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2025-06-01 00:00:00 UTC),
        datetime!(2025-06-02 00:00:00 UTC),
        100_000_000,
        dec!(2000.00),
        dec!(10000.00),
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "ST appreciated $2k claimed_deduction must NOT fire QualifiedAppraisalNote"
    );
    assert_eq!(st.removals[0].claimed_deduction, Some(dec!(2000.00)));
}

/// (P2-A-c) Boundary: exactly $5,000.00 → NOT flagged (strict >).
#[test]
fn appraisal_trigger_boundary_exactly_5000_not_flagged_with_stored_deduction() {
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(1000.00),
        dec!(5000.00), // FMV = exactly threshold; LT → claimed_deduction = $5k
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::QualifiedAppraisalNote),
        "claimed_deduction exactly $5,000.00 must NOT fire (strict >)"
    );
    assert_eq!(st.removals[0].claimed_deduction, Some(dec!(5000.00)));
}

/// (P2-A-c) Boundary: $5,000.01 → flagged.
#[test]
fn appraisal_trigger_boundary_5000_01_flagged_with_stored_deduction() {
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(1000.00),
        dec!(5000.01), // FMV = one cent over; LT → claimed_deduction = $5,000.01
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::QualifiedAppraisalNote),
        "claimed_deduction $5,000.01 must fire QualifiedAppraisalNote (strict > threshold)"
    );
    assert_eq!(st.removals[0].claimed_deduction, Some(dec!(5000.01)));
}

/// (P2-A-d) Detail text: says "Claimed deduction $X" (not "proxy"), retains all caveats:
/// dealer/inventory (§1221(a)(1)), donee-type/private foundation (§170(e)(1)(B)(ii)) [NEW],
/// aggregation (§170(f)(11)(F)), CCA 202302012.
#[test]
fn appraisal_detail_named_claimed_deduction_with_all_caveats() {
    let events = donate_single(
        datetime!(2025-01-05 00:00:00 UTC),
        datetime!(2026-03-01 00:00:00 UTC),
        datetime!(2026-03-02 00:00:00 UTC),
        100_000_000,
        dec!(5000.00),
        dec!(60000.00),
        false,
    );
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let note = st
        .blockers
        .iter()
        .find(|b| b.kind == BlockerKind::QualifiedAppraisalNote)
        .expect("QualifiedAppraisalNote must be emitted for LT $60k donation");
    let detail = &note.detail;
    // Must say "Claimed deduction $..." (exact, not proxy).
    assert!(
        detail.contains("Claimed deduction $60000.00"),
        "detail must open with 'Claimed deduction $60000.00'; got: {detail}"
    );
    // Statutory citations.
    assert!(
        detail.contains("§170(f)(11)(C)"),
        "must cite §170(f)(11)(C); got: {detail}"
    );
    assert!(
        detail.contains("CCA 202302012"),
        "must cite CCA 202302012; got: {detail}"
    );
    assert!(
        detail.contains("§170(e)"),
        "must cite §170(e); got: {detail}"
    );
    assert!(
        detail.contains("§1221(a)(1)"),
        "must cite §1221(a)(1) (dealer/inventory caveat); got: {detail}"
    );
    assert!(
        detail.contains("§170(f)(11)(F)"),
        "must cite §170(f)(11)(F) (aggregation caveat); got: {detail}"
    );
    // NEW donee-type caveat: private foundation + §170(e)(1)(B)(ii) [R0-I1].
    assert!(
        detail.contains("private foundation"),
        "detail must mention 'private foundation' (donee-type caveat); got: {detail}"
    );
    assert!(
        detail.contains("§170(e)(1)(B)(ii)"),
        "detail must cite §170(e)(1)(B)(ii) (donee-type reduction rule); got: {detail}"
    );
}

// ── P2-B Task 1: DisposalLeg.acquired_at (zone-aware) + wallet KATs ─────────────────────────────

/// KAT (a): ordinary (non-gift) disposal — acquired_at == the consumed lot's gain_hp_start
/// (which equals acquired_at on a purchased lot, since donor_acquired_at is None).
/// Also verifies wallet matches the exchange wallet the lot was consumed from.
#[test]
fn task1_kat_a_ordinary_disposal_acquired_at_equals_purchase_date() {
    let purchase_date = time::macros::date!(2025 - 03 - 01);
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
    let sell_ev = ev(
        "SELL",
        datetime!(2026-06-01 00:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 100_000,
            usd_proceeds: dec!(100.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let st = project(
        &[buy, sell_ev],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.disposals.len(), 1);
    let leg = &st.disposals[0].legs[0];
    // (a) acquired_at == purchase date (gain_hp_start == acquired_at when donor_acquired_at is None)
    assert_eq!(
        leg.acquired_at, purchase_date,
        "ordinary leg acquired_at must equal the purchase (gain_hp_start) date"
    );
    // consistent with term (2025-03-01 → 2026-06-01 is > 1 year → LT)
    assert_eq!(leg.term, Term::LongTerm);
    // (d) wallet: exchange wallet matches the consuming lot's wallet
    assert_eq!(
        leg.wallet,
        wal(),
        "wallet must match the consuming lot's exchange wallet"
    );
    // existing gain math unchanged
    assert_eq!(leg.gain, dec!(40.00));
}

/// KAT (b): §1223(2) tacked gift (Gain zone) — acquired_at == donor's tacked date (NOT gift date).
/// Dual-basis lot (fmv_at_gift=$60 < donor_basis=$100); proceeds=$120 > gain_basis=$100 → Gain zone.
/// Tacking: acquired_at = gain_hp_start = donor_acquired_at = 2024-01-01 → LT (consistent with term).
#[test]
fn task1_kat_b_gain_zone_gift_acquired_at_equals_donor_date() {
    let donor_date = time::macros::date!(2024 - 01 - 01);
    let gift_date = time::macros::date!(2025 - 06 - 01);
    let mut evs = gift_lot(
        Some(dec!(100.00)),                 // donor_basis
        Some(donor_date),                   // donor_acquired_at
        dec!(60.00),                        // fmv_at_gift < donor_basis → dual
        datetime!(2025-06-01 00:00:00 UTC), // received
    );
    evs.push(sell(
        datetime!(2025-07-01 00:00:00 UTC),
        dec!(120.00), // proceeds > gain_basis (100) → Gain zone
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::Gain));
    // (b) acquired_at == donor's tacked date, NOT the gift date
    assert_eq!(
        leg.acquired_at, donor_date,
        "gain-zone leg acquired_at must equal donor's tacked date ({donor_date}), not gift date ({gift_date})"
    );
    // consistent with term (tacked from 2024-01-01 → LT by 2025-07-01)
    assert_eq!(leg.term, Term::LongTerm);
    // existing basis/gain math unchanged
    assert_eq!(leg.basis, dec!(100.00));
    assert_eq!(leg.gain, dec!(20.00));
}

/// KAT (c): [R0-C1 LOCK] §1015 dual-basis LOSS-zone gift — acquired_at == GIFT DATE (loss_hp_start),
/// NOT the donor's tacked date. Loss basis does NOT tack (Pub 551 / §1015(a)).
/// Dual-basis lot (fmv_at_gift=$60 < donor_basis=$100); proceeds=$40 < loss_basis=$60 → Loss zone.
/// acquired_at = loss_hp_start = gift date 2025-06-01 → ST (consistent with term).
#[test]
fn task1_kat_c_loss_zone_gift_acquired_at_equals_gift_date_not_donor_date() {
    let donor_date = time::macros::date!(2024 - 01 - 01);
    let gift_date = time::macros::date!(2025 - 06 - 01);
    let mut evs = gift_lot(
        Some(dec!(100.00)),                 // donor_basis
        Some(donor_date),                   // donor_acquired_at
        dec!(60.00),                        // fmv_at_gift < donor_basis → dual (loss_basis = $60)
        datetime!(2025-06-01 00:00:00 UTC), // received — gift date = loss_hp_start
    );
    evs.push(sell(
        datetime!(2025-07-01 00:00:00 UTC),
        dec!(40.00), // proceeds < loss_basis (60) → Loss zone
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.gift_zone, Some(GiftZone::Loss));
    // [R0-C1] acquired_at MUST be the gift date (loss_hp_start), NOT the donor's tacked date
    assert_eq!(
        leg.acquired_at, gift_date,
        "loss-zone leg acquired_at must equal gift date ({gift_date}), not donor date ({donor_date}); \
         loss basis does not tack [R0-C1]"
    );
    // MUST NOT be the donor's date (this is the Critical the task exists to prevent)
    assert_ne!(
        leg.acquired_at, donor_date,
        "loss-zone leg acquired_at must NEVER be the donor date — that would contradict the leg's term [R0-C1]"
    );
    // consistent with term (HP from gift date 2025-06-01 → 2025-07-01 is < 1 year → ST)
    assert_eq!(
        leg.term,
        Term::ShortTerm,
        "loss-zone leg term must be ShortTerm (HP from gift date, not tacked)"
    );
    // existing basis/gain math unchanged
    assert_eq!(leg.basis, dec!(60.00));
    assert_eq!(leg.gain, dec!(-20.00));
}

/// KAT (d): wallet field matches the consuming lot's wallet for both exchange and self-custody.
#[test]
fn task1_kat_d_wallet_matches_consuming_lot_wallet() {
    // Exchange wallet disposal (uses wal() = Exchange { provider: "cb", account: "m" })
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
    let sell_ev = ev(
        "SELL",
        datetime!(2025-06-01 00:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 100_000,
            usd_proceeds: dec!(80.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let st = project(
        &[buy, sell_ev],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(
        st.disposals[0].legs[0].wallet,
        wal(),
        "exchange-wallet disposal leg must carry the exchange WalletId"
    );

    // Self-custody wallet disposal
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let buy_cold = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("BUY_COLD")),
        utc_timestamp: datetime!(2025-04-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::Acquire(Acquire {
            sat: 50_000,
            usd_cost: dec!(30.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };
    let sell_cold = LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new("SELL_COLD")),
        utc_timestamp: datetime!(2025-07-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold.clone()),
        payload: EventPayload::Dispose(Dispose {
            sat: 50_000,
            usd_proceeds: dec!(50.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };
    let st2 = project(
        &[buy_cold, sell_cold],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(
        st2.disposals[0].legs[0].wallet, cold,
        "self-custody disposal leg must carry the SelfCustody WalletId"
    );
}

// ── P2-C Task 1: RemovalLeg.acquired_at (removals recognize no gain/loss → always gain_hp_start) ──

/// KAT (a): ordinary (purchased-lot) donation — the removal leg's `acquired_at` == the lot's
/// gain_hp_start (== purchase date when donor_acquired_at is None) and is CONSISTENT with `term`.
/// Removals recognize no gain/loss (TP10), so there is no loss-zone branching: acquired_at can never
/// contradict term.
#[test]
fn task1_removalleg_kat_a_ordinary_donation_acquired_at_equals_hp_start() {
    let purchase_date = time::macros::date!(2025 - 03 - 01);
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
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let st = project(
        &[buy, out, recl],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.removals.len(), 1);
    let leg = &st.removals[0].legs[0];
    // acquired_at == purchase date (gain_hp_start == acquired_at when donor_acquired_at is None)
    assert_eq!(
        leg.acquired_at, purchase_date,
        "ordinary donation leg acquired_at must equal the purchase (gain_hp_start) date"
    );
    // consistent with term (2025-03-01 → 2026-06-01 is > 1 year → LT)
    assert_eq!(
        leg.term,
        Term::LongTerm,
        "acquired_at must never contradict term"
    );
}

/// KAT (b): gift-received-then-donated — the removal leg's `acquired_at` == the TACKED donor
/// acquisition date (§1223), NOT the gift-received date, and is CONSISTENT with `term`. A carryover
/// gift (fmv_at_gift ≥ donor_basis) tacks the donor's holding period on the gain side; a removal has
/// only the gain side, so acquired_at == gain_hp_start == donor_acquired_at.
#[test]
fn task1_removalleg_kat_b_gift_received_then_donated_acquired_at_equals_donor_date() {
    let donor_date = time::macros::date!(2024 - 01 - 01);
    let gift_date = time::macros::date!(2025 - 06 - 01);
    // fmv_at_gift ($60) ≥ donor_basis ($40) → single carryover lot, tacks from donor_date.
    let mut evs = gift_lot(
        Some(dec!(40.00)),
        Some(donor_date),
        dec!(60.00),
        datetime!(2025-06-01 00:00:00 UTC),
    );
    let out = ev(
        "OUT",
        datetime!(2026-02-01 00:00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let recl = dec_ev(
        2, // gift_lot uses decision seq 1 for ClassifyInbound; use 2 here
        datetime!(2026-02-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(150.00),
            fee_usd: None,
            donee: None,
        }),
    );
    evs.push(out);
    evs.push(recl);
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.removals.len(), 1);
    let leg = &st.removals[0].legs[0];
    // acquired_at == donor's tacked date (§1223), NOT the gift-received date.
    assert_eq!(
        leg.acquired_at, donor_date,
        "gift-received-then-donated leg acquired_at must equal the tacked donor date ({donor_date}), \
         not the gift date ({gift_date})"
    );
    assert_ne!(leg.acquired_at, gift_date);
    // consistent with term (tacked from 2024-01-01 → 2026-02-01 is > 1 year → LT)
    assert_eq!(
        leg.term,
        Term::LongTerm,
        "acquired_at (tacked donor date) must be consistent with term"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════
// Cycle A — inbound self-transfer-in (SPEC_self_transfer_inbound). A TransferIn that is the
// receiving side of an unmatched self-transfer, classified as "my own coins": a NON-taxable
// receipt that CREATES a fresh origin lot (basis default $0 conservative, acquired_at default =
// receipt date). Invariants 1–8 + serde/duplicate/void/pre-2025/wallet-missing corners.
// ══════════════════════════════════════════════════════════════════════════════════════════════

/// A raw `TransferIn` (self-transfer receiving side) — `wallet` overridable for the missing-wallet corner.
fn stx_in(
    src_ref: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    wallet: Option<WalletId>,
) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet,
        payload: EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        }),
    }
}

/// A `ClassifyInbound::SelfTransferMine` decision targeting the raw `TransferIn` named `src_ref`.
fn classify_self(
    seq: u64,
    ts: time::OffsetDateTime,
    src_ref: &str,
    basis: Option<Usd>,
    acquired_at: Option<time::Date>,
) -> LedgerEvent {
    dec_ev(
        seq,
        ts,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
            as_: InboundClass::SelfTransferMine { basis, acquired_at },
        }),
    )
}

/// Invariant 1 — conservative basis: `{basis:None}` → lot `usd_basis == 0`, `basis_source ==
/// SelfTransferInbound`; a later Sell at proceeds P → gain == P (MAX gain, never under-reports).
#[test]
fn self_transfer_in_default_basis_is_zero_and_max_gain() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    let sale = sell(datetime!(2025-06-01 00:00:00 UTC), dec!(100.00));
    let st = project(
        &[in_ev, cls, sale],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // The created lot (post-sale it is fully consumed, so inspect the disposal leg for basis/gain).
    assert_eq!(st.disposals.len(), 1);
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.basis, dec!(0.00)); // conservative $0
    assert_eq!(leg.gain, dec!(100.00)); // max gain: proceeds − 0
    assert_eq!(leg.basis_source, BasisSource::SelfTransferInbound);
}

/// Invariant 2 — adjustable + advisory keys on `None`, not the numeric value:
///   `{basis:Some(v)}`   → basis v, NO advisory.
///   `{basis:Some(0)}`   → basis 0, NO advisory (attested zero-cost is silent).
#[test]
fn self_transfer_in_supplied_basis_has_no_advisory() {
    // Some(50): real cost, no advisory.
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        "IN",
        Some(dec!(50.00)),
        None,
    );
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.lots.len(), 1);
    assert_eq!(st.lots[0].usd_basis, dec!(50.00));
    assert_eq!(st.lots[0].basis_source, BasisSource::SelfTransferInbound);
    assert!(
        !st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SelfTransferInboundZeroBasis),
        "supplied basis must NOT fire the zero-basis advisory"
    );

    // Some(0): attested zero cost — basis 0 but STILL no advisory (flag keys on None).
    let in0 = stx_in(
        "IN0",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls0 = classify_self(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        "IN0",
        Some(dec!(0)),
        None,
    );
    let st0 = project(
        &[in0, cls0],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st0.lots[0].usd_basis, dec!(0.00));
    assert!(
        !st0.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::SelfTransferInboundZeroBasis),
        "attested Some(0) must be silent — advisory keys on None, not usd_basis == 0"
    );
}

/// Invariant 3a — conservative holding period: `{acquired_at:None}` on receipt date D → lot
/// `acquired_at == D`; a <1yr-later disposal is Short-Term (conservative until proven long).
#[test]
fn self_transfer_in_hp_defaults_to_receipt_date_short_term() {
    let d = time::macros::date!(2025 - 04 - 01);
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    let sale = sell(datetime!(2025-06-01 00:00:00 UTC), dec!(100.00)); // ~2 months later
    let st = project(
        &[in_ev, cls, sale],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.acquired_at, d); // gain_hp_start == lot's own acquired_at (no tacking)
    assert_eq!(leg.term, Term::ShortTerm);
}

/// Invariant 3b — pool/HP ORTHOGONALITY: a 2026 receipt with a real 2013 `acquired_at` lands in the
/// 2026 **Wallet** pool (keyed on the RECEIPT date) yet is Long-Term (HP from 2013). Proven jointly:
/// the 2026-dated sale (Wallet pool) FINDS the lot (would be an uncovered disposal if it were mis-keyed
/// into Universal) AND the disposal is Long-Term.
#[test]
fn self_transfer_in_supplied_old_date_is_long_term_in_wallet_pool() {
    let old = time::macros::date!(2013 - 05 - 01);
    let in_ev = stx_in(
        "IN",
        datetime!(2026-01-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(
        1,
        datetime!(2026-01-02 00:00:00 UTC),
        "IN",
        Some(dec!(10.00)),
        Some(old),
    );
    let sale = sell(datetime!(2026-03-01 00:00:00 UTC), dec!(100.00));
    let st = project(
        &[in_ev, cls, sale],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(
        st.disposals.len(),
        1,
        "the 2026 Wallet-pool sale must find the lot"
    );
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.acquired_at, old);
    assert_eq!(leg.term, Term::LongTerm);
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::UncoveredDisposal),
        "lot must be in the 2026 Wallet pool (receipt date), not Universal (acquired_at)"
    );
}

/// Invariant 4 — NON-taxable: no `IncomeRecord`, no Disposal/Removal for the receipt itself,
/// nothing on any form. (Only the created lot; no recognition event.)
#[test]
fn self_transfer_in_is_non_taxable() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.lots.len(), 1, "one non-taxable origin lot created");
    assert!(
        st.income_recognized.is_empty(),
        "self-transfer-in recognizes NO income"
    );
    assert!(st.disposals.is_empty());
    assert!(st.removals.is_empty());
}

/// Invariant 5 — NEVER basis_pending / NEVER gated: `{basis:None}` → lot `basis_pending == false`;
/// a later disposal computes a real gain with NO `FmvMissing` gate. (Gotcha G1.)
#[test]
fn self_transfer_in_is_never_basis_pending_or_gated() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    // Lot-only projection: basis_pending must be false even for the defaulted $0 basis.
    let st_lot = project(
        &[in_ev.clone(), cls.clone()],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        !st_lot.lots[0].basis_pending,
        "$0 basis is computable — NEVER pending (G1)"
    );

    // Disposal must NOT be gated by FmvMissing.
    let sale = sell(datetime!(2025-07-01 00:00:00 UTC), dec!(80.00));
    let st = project(
        &[in_ev, cls, sale],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.disposals.len(), 1);
    assert_eq!(st.disposals[0].legs[0].gain, dec!(80.00));
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::FmvMissing),
        "a self-transfer-in disposal must NEVER raise FmvMissing"
    );
}

/// Invariant 6 — the honest flag is ADVISORY and NON-gating: `SelfTransferInboundZeroBasis.severity()
/// == Advisory`; a projection whose ONLY blocker is this one carries NO Hard blocker, so
/// `compute_tax_year` returns `Computed` (its sole blocker gate is `first_hard_blocker`, severity==Hard).
#[test]
fn self_transfer_in_zero_basis_blocker_is_advisory_and_non_gating() {
    use btctax_core::tax::compute::compute_tax_year;
    use btctax_core::tax::tables::{LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable};
    use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
    use std::collections::BTreeMap;

    assert_eq!(
        BlockerKind::SelfTransferInboundZeroBasis.severity(),
        Severity::Advisory,
        "the zero-basis honesty flag must be Advisory, never Hard"
    );

    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    // The advisory fires...
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::SelfTransferInboundZeroBasis));
    // ...and it is the ONLY blocker, and it is NOT Hard.
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind.severity() != Severity::Hard),
        "no Hard blocker may arise from a self-transfer-in — compute must not be gated"
    );

    // And compute_tax_year actually returns Computed (not NotComputable) for the receipt year.
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![OrdinaryBracket {
                lower: dec!(0),
                rate: dec!(0.10),
            }],
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
    let mut tables: BTreeMap<i32, TaxTable> = BTreeMap::new();
    tables.insert(
        2025,
        TaxTable {
            year: 2025,
            source: "SYNTHETIC",
            ordinary,
            ltcg,
            gift_annual_exclusion: dec!(19000),
            ss_wage_base: dec!(176100),
            gift_lifetime_exclusion: dec!(13_990_000),
        },
    );
    let profile = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(0),
        magi_excluding_crypto: dec!(0),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    assert!(
        matches!(
            compute_tax_year(&[], &st, 2025, Some(&profile), &tables),
            TaxOutcome::Computed(_)
        ),
        "a vault whose only blocker is the zero-basis advisory must still compute the year"
    );
}

/// Invariant 7 — OUTSIDE FIFO but sellable: a `LotSelection` targeting the self-transfer-in event is
/// rejected (`LotSelectionInvalid`) — a lot-CREATING op is not method-honoring — yet the created lot
/// participates normally in FIFO when later SOLD.
#[test]
fn self_transfer_in_is_outside_fifo_but_sellable() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        "IN",
        Some(dec!(20.00)),
        None,
    );
    // A LotSelection naming the self-transfer-in TransferIn as its "disposal" — non-honoring.
    let bad_sel = dec_ev(
        2,
        datetime!(2026-01-02 00:00:00 UTC),
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new("IN")),
            lots: vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("IN")),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        }),
    );
    let sale = sell(datetime!(2026-06-01 00:00:00 UTC), dec!(100.00));
    let st = project(
        &[in_ev, cls, bad_sel, sale],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::LotSelectionInvalid),
        "a LotSelection cannot target a lot-creating self-transfer-in"
    );
    // The lot still sells normally under FIFO fallback (basis 20 → gain 80).
    assert_eq!(st.disposals.len(), 1);
    assert_eq!(st.disposals[0].legs[0].basis, dec!(20.00));
    assert_eq!(st.disposals[0].legs[0].gain, dec!(80.00));
}

/// Invariant 8 — FR9 conservation: `sigma_in` increments by the received sats and the report balances.
#[test]
fn self_transfer_in_conserves_sigma_in() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.stats.sigma_in, 100_000);
    let r = conservation_report(&st);
    assert!(r.balanced, "{r:?}");
    assert_eq!(r.sigma_held, 100_000);
}

/// serde: a `ClassifyInbound::SelfTransferMine` (with basis + acquired_at) round-trips through the
/// canonical vault encoding unchanged.
#[test]
fn self_transfer_in_classify_round_trips_serde() {
    let ci = ClassifyInbound {
        transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("IN")),
        as_: InboundClass::SelfTransferMine {
            basis: Some(dec!(12.34)),
            acquired_at: Some(time::macros::date!(2019 - 08 - 07)),
        },
    };
    let payload = EventPayload::ClassifyInbound(ci);
    let json = serde_json::to_string(&payload).unwrap();
    let back: EventPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(payload, back);

    // Also the defaults form (both None).
    let ci2 = ClassifyInbound {
        transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("IN2")),
        as_: InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        },
    };
    let p2 = EventPayload::ClassifyInbound(ci2);
    let back2: EventPayload = serde_json::from_str(&serde_json::to_string(&p2).unwrap()).unwrap();
    assert_eq!(p2, back2);
}

/// Duplicate `ClassifyInbound` first-wins still holds for the new variant: the second decision is a
/// `DecisionConflict` and the FIRST classification governs.
#[test]
fn duplicate_classify_inbound_self_transfer_first_wins() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    // First: basis $10 (wins). Second: basis $99 (excluded → DecisionConflict).
    let first = classify_self(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        "IN",
        Some(dec!(10.00)),
        None,
    );
    let second = classify_self(
        2,
        datetime!(2026-01-02 00:00:00 UTC),
        "IN",
        Some(dec!(99.00)),
        None,
    );
    let st = project(
        &[in_ev, first, second],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st.lots.len(), 1);
    assert_eq!(
        st.lots[0].usd_basis,
        dec!(10.00),
        "first classification wins"
    );
    assert_eq!(
        st.blockers
            .iter()
            .filter(|b| b.kind == BlockerKind::DecisionConflict)
            .count(),
        1,
        "exactly one DecisionConflict for the duplicate"
    );
}

/// Voiding the self-transfer classification RE-EXPOSES the raw `TransferIn` as `UnknownInbound`
/// (Hard `UnknownBasisInbound`, no lot) — the decision is fully reversible.
#[test]
fn void_classify_inbound_self_transfer_re_exposes_unknown_inbound() {
    let in_ev = stx_in(
        "IN",
        datetime!(2025-04-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(1, datetime!(2026-01-01 00:00:00 UTC), "IN", None, None);
    let void = dec_ev(
        2,
        datetime!(2026-02-01 00:00:00 UTC),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: EventId::decision(1),
        }),
    );
    let st = project(
        &[in_ev, cls, void],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.lots.is_empty(), "voided classification creates no lot");
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::SelfTransferInboundZeroBasis),
        "voided classification fires no self-transfer advisory"
    );
}

/// [R0-N2] A PRE-2025 receipt self-transfer-in folds through the **Universal** pool (pool keyed on the
/// pre-transition receipt date) and conserves; a post-2025 sale (reconstructed per-wallet) consumes it.
#[test]
fn pre_2025_self_transfer_in_conserves_through_universal_pool() {
    // Receipt on 2024-06-01 (pre-TRANSITION_DATE 2025-01-01) → Universal pool.
    let in_ev = stx_in(
        "IN",
        datetime!(2024-06-01 00:00:00 UTC),
        100_000,
        Some(wal()),
    );
    let cls = classify_self(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        "IN",
        Some(dec!(15.00)),
        None,
    );
    // Lot-only: conserves; lot present with acquired_at == receipt (2024-06-01).
    let st_lot = project(
        &[in_ev.clone(), cls.clone()],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(st_lot.lots.len(), 1);
    assert_eq!(
        st_lot.lots[0].acquired_at,
        time::macros::date!(2024 - 06 - 01)
    );
    let r0 = conservation_report(&st_lot);
    assert!(
        r0.balanced,
        "pre-2025 self-transfer-in must conserve: {r0:?}"
    );

    // With a 2026 sale: the Universal lot reconstructs into the per-wallet pool and is consumed LT.
    let sale = sell(datetime!(2026-03-01 00:00:00 UTC), dec!(100.00));
    let st = project(
        &[in_ev, cls, sale],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(
        st.disposals.len(),
        1,
        "the pre-2025 lot must be sellable post-transition"
    );
    assert_eq!(st.disposals[0].legs[0].term, Term::LongTerm);
    let r = conservation_report(&st);
    assert!(r.balanced, "{r:?}");
}

/// [R0-M2 / G5] The wallet-MISSING corner: a self-transfer-in `TransferIn` with `wallet: None` has
/// nowhere to create the lot → emit a Hard `UnknownBasisInbound` (a self-transfer message), NOT the
/// income-path `FmvMissing`, and create NO lot. Must not panic.
#[test]
fn self_transfer_in_without_wallet_emits_hard_unknown_basis_not_fmv_missing() {
    let in_ev = stx_in("IN", datetime!(2025-04-01 00:00:00 UTC), 100_000, None);
    let cls = classify_self(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        "IN",
        Some(dec!(10.00)),
        None,
    );
    let st = project(
        &[in_ev, cls],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st.lots.is_empty(), "no wallet → no lot");
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::UnknownBasisInbound),
        "wallet-missing self-transfer-in must raise Hard UnknownBasisInbound"
    );
    assert!(
        st.blockers.iter().all(|b| b.kind != BlockerKind::FmvMissing),
        "must NOT copy the IncomeInbound FmvMissing guard (semantically wrong for a non-income receipt)"
    );
}
