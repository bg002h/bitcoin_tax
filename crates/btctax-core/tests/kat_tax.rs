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
    assert!(st
        .blockers
        .iter()
        .all(|b| b.kind != BlockerKind::UnknownBasisInbound)); // dest TransferIn consumed
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
