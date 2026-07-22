//! KATs for the Defensive Filing Wizard's DFW-D4 structured shortfall signal + triage classifier
//! (Task 5, Group B). `shortfalls`/`triage` are derived, read-only reads over the projected
//! `LedgerState` (+ the raw event log for `triage`'s pool/date lookup) — additive/derived, no filed-
//! number change. PRIVACY: synthetic values only.

use btctax_core::defensive::discovery::{shortfalls, triage, Triage};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::BlockerKind;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

// ── fixture harness (mirrors tests/kat_tranche.rs / tests/properties.rs) ──────────────────────────
fn wa() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "a".into(),
    }
}
fn wb() -> WalletId {
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
fn imp_no_wallet(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: p,
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
fn prices() -> StaticPrices {
    StaticPrices::default()
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default()
}

/// A confirmed self-transfer: TransferOut (from) + TransferIn (to) + a TransferLink decision.
/// `fee_sat` threads an on-chain fee through (None ⇒ no fee draw at all).
#[allow(clippy::too_many_arguments)]
fn self_transfer(
    out_rf: &str,
    in_rf: &str,
    from: &WalletId,
    to: &WalletId,
    sat: i64,
    fee_sat: Option<i64>,
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
                fee_sat,
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

/// A self-transfer with an EMPTY source pool (no prior Acquire) is entirely short on principal —
/// exactly one shortfall on the transfer event, correlated with NO open acquisition blocker, so it is
/// a clean `DeclareCandidate` (arch-I-2/DFW-D4).
#[test]
fn self_transfer_short_is_one_declare_candidate_of_short_sat() {
    let events = self_transfer(
        "XOUT1",
        "XIN1",
        &wa(),
        &wb(),
        50_000_000,
        None,
        datetime!(2025-03-01 00:00 UTC),
        1,
    );
    let st = project(&events, &prices(), &cfg());
    let triaged = triage(&events, &st);
    assert_eq!(triaged.len(), 1, "exactly one triage entry: {triaged:?}");
    match &triaged[0] {
        Triage::DeclareCandidate(s) => {
            assert_eq!(s.short_sat, 50_000_000);
            assert_eq!(s.fee_sat, 0);
        }
        other => panic!("expected DeclareCandidate, got {other:?}"),
    }
}

/// A `GiftOut` whose source `TransferOut` carries NO wallet (fold.rs "gift out without wallet")
/// never records a sat amount — no `Shortfall` is ever built for it, so it routes to `DataFix`, never
/// a `DeclareCandidate`.
#[test]
fn gift_out_without_wallet_yields_zero_declare_candidates() {
    let xfer = imp_no_wallet(
        "GOUT1",
        datetime!(2025-04-01 00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 10_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let reclass = dec_ev(
        1,
        datetime!(2025-04-02 00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("GOUT1")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(500.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let events = vec![xfer, reclass];
    let st = project(&events, &prices(), &cfg());
    let triaged = triage(&events, &st);
    assert!(
        triaged
            .iter()
            .all(|t| !matches!(t, Triage::DeclareCandidate(_))),
        "no DeclareCandidate for a without-wallet gift out: {triaged:?}"
    );
    assert!(
        triaged.iter().any(|t| matches!(t, Triage::DataFix(_))),
        "a without-wallet gift out routes to DataFix: {triaged:?}"
    );
}

/// A `Donate` whose source `TransferOut` carries NO wallet (fold.rs "donate without wallet") — same
/// shape as the gift-out case.
#[test]
fn donate_without_wallet_yields_zero_declare_candidates() {
    let xfer = imp_no_wallet(
        "DOUT1",
        datetime!(2025-05-01 00:00 UTC),
        EventPayload::TransferOut(TransferOut {
            sat: 10_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let reclass = dec_ev(
        1,
        datetime!(2025-05-02 00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("DOUT1")),
            as_: OutflowClass::Donate {
                appraisal_required: false,
            },
            principal_proceeds_or_fmv: dec!(500.00),
            fee_usd: None,
            donee: None,
        }),
    );
    let events = vec![xfer, reclass];
    let st = project(&events, &prices(), &cfg());
    let triaged = triage(&events, &st);
    assert!(
        triaged
            .iter()
            .all(|t| !matches!(t, Triage::DeclareCandidate(_))),
        "no DeclareCandidate for a without-wallet donate: {triaged:?}"
    );
    assert!(
        triaged.iter().any(|t| matches!(t, Triage::DataFix(_))),
        "a without-wallet donate routes to DataFix: {triaged:?}"
    );
}

/// A never-classified import (`Op::Unclassified`, no `ClassifyRaw`) never contributes sats to its
/// wallet's pool. A LATER disposal in the SAME pool that comes up short — with the unclassified row's
/// date `<=` the disposal's date — is behind an open acquisition question: DFW-D4 routes it to
/// `ResolveFirst`, never a bare `DeclareCandidate`.
#[test]
fn shortfall_behind_open_unclassified_is_resolve_first() {
    let w = wa();
    let unclassified = imp(
        "U1",
        datetime!(2025-01-05 00:00 UTC),
        &w,
        EventPayload::Unclassified(Unclassified {
            raw: "unrecognized row".into(),
        }),
    );
    let sell = imp(
        "S1",
        datetime!(2025-02-01 00:00 UTC),
        &w,
        EventPayload::Dispose(Dispose {
            sat: 10_000_000,
            usd_proceeds: dec!(500.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let events = vec![unclassified, sell];
    let st = project(&events, &prices(), &cfg());
    let triaged = triage(&events, &st);
    assert_eq!(triaged.len(), 1, "exactly one triage entry: {triaged:?}");
    match &triaged[0] {
        Triage::ResolveFirst { shortfall, blocker } => {
            assert_eq!(shortfall.short_sat, 10_000_000);
            assert_eq!(*blocker, BlockerKind::Unclassified);
        }
        other => panic!("expected ResolveFirst, got {other:?}"),
    }
}

/// A bare (unlinked, unreclassified) `TransferOut` folds as `Op::PendingOut` and ALWAYS co-emits an
/// advisory `UnmatchedOutflows` on its own event — the C-1 double-count guard (tax-I-1/arch-I-5): a
/// pending-out short is never a bare `DeclareCandidate` (a later `TransferLink` may reshape it), it is
/// always `ResolveFirst` via that co-emitted advisory.
#[test]
fn pending_out_short_routes_through_unmatched_outflows_first() {
    let w = wa();
    let xfer = imp(
        "P1",
        datetime!(2025-03-01 00:00 UTC),
        &w,
        EventPayload::TransferOut(TransferOut {
            sat: 20_000_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    );
    let events = vec![xfer];
    let st = project(&events, &prices(), &cfg());
    let triaged = triage(&events, &st);
    assert_eq!(triaged.len(), 1, "exactly one triage entry: {triaged:?}");
    match &triaged[0] {
        Triage::ResolveFirst { shortfall, blocker } => {
            assert_eq!(shortfall.short_sat, 20_000_000);
            assert_eq!(*blocker, BlockerKind::UnmatchedOutflows);
        }
        other => panic!("expected ResolveFirst, got {other:?}"),
    }
}

/// arch-I-2: a self-transfer short on BOTH principal (fold.rs's own "short by" site) AND its
/// on-chain fee (the shared `consume_fee` site) emits TWO raw records on the SAME event — `shortfalls`
/// aggregates them into ONE `Shortfall`: `short_sat` sums principal+fee, `fee_sat` sums the fee
/// component alone.
#[test]
fn principal_plus_fee_short_on_one_event_aggregate_to_one_shortfall() {
    let events = self_transfer(
        "XOUT2",
        "XIN2",
        &wa(),
        &wb(),
        50_000_000,
        Some(1_000),
        datetime!(2025-03-01 00:00 UTC),
        1,
    );
    let st = project(&events, &prices(), &cfg());
    let sf = shortfalls(&st);
    assert_eq!(sf.len(), 1, "one aggregate shortfall per event: {sf:?}");
    assert_eq!(sf[0].short_sat, 50_000_000 + 1_000);
    assert_eq!(sf[0].fee_sat, 1_000);
}

/// arch-I-2/tax-M-1: a PURE-fee short (principal fully covered, only the fee draw finds the pool
/// empty — `consume_fee`'s own site) has `fee_sat == short_sat`. A PURE-principal short (no fee
/// component at all) has `fee_sat == 0`.
#[test]
fn fee_only_short_has_fee_sat_equal_short_sat() {
    // Pure-fee short: an Acquire covers principal exactly, so the self-transfer's principal draw
    // fully succeeds (no principal shortfall) and leaves the pool empty for the fee draw.
    let acquire = imp(
        "A1",
        datetime!(2025-01-05 00:00 UTC),
        &wa(),
        EventPayload::Acquire(Acquire {
            sat: 50_000_000,
            usd_cost: dec!(1000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let mut fee_events = vec![acquire];
    fee_events.extend(self_transfer(
        "XOUT3",
        "XIN3",
        &wa(),
        &wb(),
        50_000_000,
        Some(1_000),
        datetime!(2025-02-01 00:00 UTC),
        1,
    ));
    let st = project(&fee_events, &prices(), &cfg());
    let sf = shortfalls(&st);
    assert_eq!(sf.len(), 1, "one aggregate shortfall: {sf:?}");
    assert_eq!(sf[0].short_sat, 1_000);
    assert_eq!(
        sf[0].fee_sat, 1_000,
        "a pure-fee short has fee_sat == short_sat"
    );

    // Pure-principal short: no fee component at all (fee_sat: None) — fee_sat must be 0.
    let principal_events = self_transfer(
        "XOUT4",
        "XIN4",
        &wa(),
        &wb(),
        30_000_000,
        None,
        datetime!(2025-03-01 00:00 UTC),
        2,
    );
    let st2 = project(&principal_events, &prices(), &cfg());
    let sf2 = shortfalls(&st2);
    assert_eq!(sf2.len(), 1, "one aggregate shortfall: {sf2:?}");
    assert_eq!(sf2[0].short_sat, 30_000_000);
    assert_eq!(sf2[0].fee_sat, 0, "a pure-principal short has fee_sat == 0");
}

/// Grep-guard (DFW-D4): `discovery.rs` must NEVER parse `Blocker.detail` — every classification comes
/// from `BlockerKind` + the raw event log (`LedgerEvent`), never a human-readable message string.
#[test]
fn shortfalls_never_parses_blocker_detail() {
    let src = include_str!("../src/defensive/discovery.rs");
    assert!(
        !src.contains(".detail"),
        "discovery.rs must never parse Blocker.detail — DFW-D4 correlates only via BlockerKind + \
         the raw event log, never a string-sniff of the human-readable message"
    );
}
