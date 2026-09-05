//! FR-31 — a `--to-wallet` `TransferLink` and an inbound booked AT that wallet can be the SAME
//! movement, booked TWICE: the link relocates the real lot into the destination, and the inbound
//! creates a fresh ORIGIN lot for the coins that just arrived there. Pool doubled, basis doubled,
//! `conservation_report().balanced == true`, and (before this guard) zero blockers.
//!
//! All fixtures are SYNTHETIC. No real user data is read.
//!
//! The guard under test is `resolve`'s `[FR-31]` double-booking check. It REFUSES (Hard
//! `BlockerKind::SelfTransferDoubleBooked`) and never picks a pairing — the owner's self-transfer
//! policy is "matched pairs are CONFIRMED, not auto", so software may not decide which in-leg a
//! `--to-wallet` link meant. It names both events and points at the precise form.
//!
//! Covered:
//!  - variant D (`--to-wallet` + `SelfTransferMine{basis: Some}`)  → the pure understatement case.
//!  - variant C (`--to-wallet` + `SelfTransferMine{basis: None}`)  → the bulk-classify path.
//!  - variant B (`--to-wallet` + UNCLASSIFIED inbound)             → the routing trap.
//!  - inbound classified Income / GiftReceived at the destination  → same mechanism, same refusal.
//!  - DISCRIMINATION (must NOT fire): the `--to-event` form; an amount that cannot be this
//!    movement; a date outside the window; an inbound at a different wallet; an untracked
//!    destination; an in-leg already consumed by another link.

use btctax_core::conventions::{Sat, Usd};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::conservation::conservation_report;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

// ── Fixture helpers ────────────────────────────────────────────────────────────────────────────

fn exchange() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}

fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}

fn other() -> WalletId {
    WalletId::SelfCustody {
        label: "other".into(),
    }
}

fn imp(
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

fn dec_ev(seq: u64, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: datetime!(2026-01-15 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload,
    }
}

fn out_id() -> EventId {
    EventId::import(Source::Coinbase, SourceRef::new("OUT-1"))
}
fn in_id() -> EventId {
    EventId::import(Source::Coinbase, SourceRef::new("IN-1"))
}

const ONE_BTC: Sat = 100_000_000;

/// buy 1 BTC for $50,000 on the exchange; withdraw it on 2025-03-01; it arrives at `in_wallet`
/// `in_sat` sats later that day (unless `in_ts` says otherwise).
fn base_events(in_wallet: WalletId, in_sat: Sat, in_ts: time::OffsetDateTime) -> Vec<LedgerEvent> {
    vec![
        imp(
            "BUY-1",
            datetime!(2025-02-01 12:00:00 UTC),
            exchange(),
            EventPayload::Acquire(Acquire {
                sat: ONE_BTC,
                usd_cost: dec!(50000.00),
                fee_usd: dec!(0.00),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "OUT-1",
            datetime!(2025-03-01 12:00:00 UTC),
            exchange(),
            EventPayload::TransferOut(TransferOut {
                sat: ONE_BTC,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            "IN-1",
            in_ts,
            in_wallet,
            EventPayload::TransferIn(TransferIn {
                sat: in_sat,
                src_addr: None,
                txid: None,
            }),
        ),
    ]
}

fn link_to_wallet(seq: u64, w: WalletId) -> LedgerEvent {
    dec_ev(
        seq,
        EventPayload::TransferLink(TransferLink {
            out_event: out_id(),
            in_event_or_wallet: TransferTarget::Wallet(w),
        }),
    )
}

fn classify_in(seq: u64, as_: InboundClass) -> LedgerEvent {
    dec_ev(
        seq,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: in_id(),
            as_,
        }),
    )
}

fn run(events: &[LedgerEvent]) -> LedgerState {
    project(
        events,
        &StaticPrices(Default::default()),
        &ProjectionConfig::default(),
    )
}

fn double_book_blockers(st: &LedgerState) -> Vec<&Blocker> {
    st.blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::SelfTransferDoubleBooked)
        .collect()
}

fn sigma_basis(st: &LedgerState) -> Usd {
    st.lots.iter().map(|l| l.usd_basis).sum()
}

// ── The defect: it fires, and it is HARD ───────────────────────────────────────────────────────

/// Variant D — the pure understatement case. `--to-wallet self:cold` relocates the real $50,000 lot
/// into `cold`, and the arriving deposit is separately classified `SelfTransferMine` with the SAME
/// $50,000 basis. Result before the guard: 2 lots, 2 BTC held, **$100,000 basis**, FR9
/// `balanced: true`, **zero blockers**. Doubled basis shrinks gain ⇒ understates tax.
#[test]
fn variant_d_wallet_link_plus_stated_basis_is_refused() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: Some(dec!(50000.00)),
            acquired_at: Some(date!(2025 - 02 - 01)),
        },
    ));
    let st = run(&evs);

    // The measured doubling is still in the state — the guard REFUSES, it does not silently pick a
    // pairing and rewrite the ledger. What must never happen is that this reaches a filed number.
    assert_eq!(st.lots.len(), 2, "the doubling itself is unchanged");
    assert_eq!(sigma_basis(&st), dec!(100000.00));
    assert!(
        conservation_report(&st).balanced,
        "FR9 is sat-only and CANNOT see this — the phantom lot bumps sigma_in and sigma_held \
         equally. That is precisely why the guard cannot live in conservation.rs."
    );

    let found = double_book_blockers(&st);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one SelfTransferDoubleBooked blocker, got {:?}",
        st.blockers
    );
    assert_eq!(found[0].event.as_ref(), Some(&in_id()));
    assert!(
        found[0].detail.contains(&out_id().canonical()),
        "the blocker must NAME the other leg so the filer can resolve it: {}",
        found[0].detail
    );
    assert!(
        found[0].detail.contains("--to-event"),
        "the blocker must name the precise form: {}",
        found[0].detail
    );
    assert_eq!(
        BlockerKind::SelfTransferDoubleBooked.severity(),
        Severity::Hard
    );
    // This is the exact predicate `compute_tax_year` step (1) uses to refuse a year.
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind.severity() == Severity::Hard));
}

/// Variant C — the BULK path. `bulk-classify-inbound-self-transfer` writes
/// `SelfTransferMine { basis: None }`; the pool doubles (2 BTC held where 1 exists) and the only
/// blockers left are advisories that REASSURE the filer the treatment is conservative.
#[test]
fn variant_c_wallet_link_plus_bulk_zero_basis_is_refused() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: None,
            acquired_at: None,
        },
    ));
    let st = run(&evs);

    assert_eq!(st.lots.len(), 2);
    assert_eq!(
        st.lots.iter().map(|l| l.remaining_sat).sum::<Sat>(),
        2 * ONE_BTC,
        "2 BTC held where 1 exists"
    );
    assert_eq!(double_book_blockers(&st).len(), 1, "{:?}", st.blockers);
}

/// Variant B — the ROUTING TRAP. The unconsumed in-leg keeps a Hard `UnknownBasisInbound`, which is
/// the exact selection key of `bulk-classify-inbound-self-transfer`. No lot is doubled YET, but the
/// tool's own blocker is walking the filer into variant C. The guard names both legs here too.
#[test]
fn variant_b_wallet_link_with_unclassified_inbound_names_both_legs() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    let st = run(&evs);

    assert_eq!(
        st.lots.len(),
        1,
        "not doubled yet — this is the trap, not the loss"
    );
    let found = double_book_blockers(&st);
    assert_eq!(found.len(), 1, "{:?}", st.blockers);
    assert!(found[0].detail.contains(&out_id().canonical()));
}

/// The mechanism is "an ORIGIN lot is booked at the wallet a link relocates into" — it is not
/// keyed to the `SelfTransferMine` variant we happened to observe. An inbound the filer calls
/// INCOME at that wallet is the same double-book (relocated lot + income lot).
#[test]
fn an_income_classified_inbound_at_the_destination_is_refused_too() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: Some(dec!(50000.00)),
            business: false,
        },
    ));
    let st = run(&evs);
    assert_eq!(double_book_blockers(&st).len(), 1, "{:?}", st.blockers);
}

/// Same for a received gift.
#[test]
fn a_gift_classified_inbound_at_the_destination_is_refused_too() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::GiftReceived {
            donor_basis: Some(dec!(10000.00)),
            donor_acquired_at: Some(date!(2024 - 01 - 01)),
            fmv_at_gift: dec!(50000.00),
        },
    ));
    let st = run(&evs);
    assert_eq!(double_book_blockers(&st).len(), 1, "{:?}", st.blockers);
}

// ── DISCRIMINATION: the guard must stay silent on every one of these ───────────────────────────

/// The CORRECT form. `--to-event` consumes the in-leg (`Op::Skip`), so there is one lot and nothing
/// to refuse. If this reds, the guard has made the sound path unusable.
#[test]
fn the_to_event_form_is_clean() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(dec_ev(
        1,
        EventPayload::TransferLink(TransferLink {
            out_event: out_id(),
            in_event_or_wallet: TransferTarget::InEvent(in_id()),
        }),
    ));
    let st = run(&evs);
    assert_eq!(st.lots.len(), 1);
    assert_eq!(sigma_basis(&st), dec!(50000.00));
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// An arrival at the destination that CANNOT be this movement: 0.1 BTC against a 1 BTC withdrawal
/// is far outside the amount tolerance. A filer who genuinely received unrelated coins at a wallet
/// they also transfer into must not be blocked.
#[test]
fn an_inbound_of_a_different_amount_is_not_flagged() {
    let mut evs = base_events(cold(), ONE_BTC / 10, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: Some(dec!(5000.00)),
            acquired_at: Some(date!(2025 - 01 - 01)),
        },
    ));
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// An arrival of the same size months later is a different movement.
#[test]
fn an_inbound_outside_the_window_is_not_flagged() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-07-04 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: Some(dec!(50000.00)),
            acquired_at: Some(date!(2025 - 01 - 01)),
        },
    ));
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// An arrival BEFORE the withdrawal is not the other leg of a relocation.
#[test]
fn an_inbound_preceding_the_withdrawal_is_not_flagged() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-02-20 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: Some(dec!(50000.00)),
            acquired_at: Some(date!(2025 - 01 - 01)),
        },
    ));
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// The arrival is at a DIFFERENT wallet than the one the link relocates into.
#[test]
fn an_inbound_at_another_wallet_is_not_flagged() {
    let mut evs = base_events(other(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: Some(dec!(50000.00)),
            acquired_at: Some(date!(2025 - 01 - 01)),
        },
    ));
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// The whole point of `--to-wallet`: an UNTRACKED destination, whose deposits are not imported.
/// There is no in-leg at all, so there is nothing to double-book.
#[test]
fn a_link_to_an_untracked_wallet_is_clean() {
    let evs = vec![
        imp(
            "BUY-1",
            datetime!(2025-02-01 12:00:00 UTC),
            exchange(),
            EventPayload::Acquire(Acquire {
                sat: ONE_BTC,
                usd_cost: dec!(50000.00),
                fee_usd: dec!(0.00),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "OUT-1",
            datetime!(2025-03-01 12:00:00 UTC),
            exchange(),
            EventPayload::TransferOut(TransferOut {
                sat: ONE_BTC,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        link_to_wallet(1, cold()),
    ];
    let st = run(&evs);
    assert_eq!(st.lots.len(), 1);
    assert_eq!(sigma_basis(&st), dec!(50000.00));
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// Voiding the offending link clears the refusal — the blocker is resolvable, not a dead end.
#[test]
fn voiding_the_wallet_link_clears_the_refusal() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(link_to_wallet(1, cold()));
    evs.push(classify_in(
        2,
        InboundClass::SelfTransferMine {
            basis: Some(dec!(50000.00)),
            acquired_at: Some(date!(2025 - 02 - 01)),
        },
    ));
    evs.push(dec_ev(
        3,
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: EventId::decision(1),
        }),
    ));
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// TWO movements into the same wallet: the first pair named precisely with `--to-event`, the second
/// recorded with `--to-wallet` (its own arrival never imported). The already-consumed deposit is
/// relocated ONTO, not booked — it mints no origin lot — so it must not be flagged against the
/// second link, even though it matches that link's out-leg on amount and date.
#[test]
fn a_deposit_already_consumed_by_another_link_is_not_flagged() {
    let evs = vec![
        imp(
            "BUY-1",
            datetime!(2025-02-01 12:00:00 UTC),
            exchange(),
            EventPayload::Acquire(Acquire {
                sat: 2 * ONE_BTC,
                usd_cost: dec!(100000.00),
                fee_usd: dec!(0.00),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        imp(
            "OUT-1",
            datetime!(2025-03-01 12:00:00 UTC),
            exchange(),
            EventPayload::TransferOut(TransferOut {
                sat: ONE_BTC,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            "IN-1",
            datetime!(2025-03-01 13:00:00 UTC),
            cold(),
            EventPayload::TransferIn(TransferIn {
                sat: ONE_BTC,
                src_addr: None,
                txid: None,
            }),
        ),
        imp(
            "OUT-2",
            datetime!(2025-03-01 14:00:00 UTC),
            exchange(),
            EventPayload::TransferOut(TransferOut {
                sat: ONE_BTC,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        ),
        // OUT-1 → IN-1, named precisely: IN-1 lands in `consumed_ins`.
        dec_ev(
            1,
            EventPayload::TransferLink(TransferLink {
                out_event: out_id(),
                in_event_or_wallet: TransferTarget::InEvent(in_id()),
            }),
        ),
        // OUT-2 → the wallet. IN-1 matches its amount and date, but is already consumed.
        dec_ev(
            2,
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT-2")),
                in_event_or_wallet: TransferTarget::Wallet(cold()),
            }),
        ),
    ];
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}

/// A deposit dropped by a confirmed `SelfTransferPassthrough` folds to `Op::Skip` and mints no
/// origin lot either, so it is likewise not a double-book against an unrelated `--to-wallet` link.
#[test]
fn a_deposit_dropped_by_a_passthrough_is_not_flagged() {
    let mut evs = base_events(cold(), ONE_BTC, datetime!(2025-03-01 13:00:00 UTC));
    evs.push(imp(
        "OUT-2",
        datetime!(2025-03-02 09:00:00 UTC),
        cold(),
        EventPayload::TransferOut(TransferOut {
            sat: ONE_BTC,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    ));
    evs.push(link_to_wallet(1, cold()));
    evs.push(dec_ev(
        2,
        EventPayload::SelfTransferPassthrough(SelfTransferPassthrough {
            in_event: in_id(),
            out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT-2")),
        }),
    ));
    let st = run(&evs);
    assert!(double_book_blockers(&st).is_empty(), "{:?}", st.blockers);
}
