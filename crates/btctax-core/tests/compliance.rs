//! §A.5 `DisposalCompliance` projection tests (Task 7).
//!
//! Covers: custody mapping (self-custody vs. broker); the 2025-2026 own-books envelope vs. the
//! 2027+ broker-communication requirement; made-date <= time-of-sale -> Contemporaneous;
//! made-date > time-of-sale -> NonCompliant (§1.1012-1(j) no post-hoc); standing-order coverage;
//! and determinism (decision_seq ordering, load-order independent).
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::{disposal_compliance, ComplianceStatus, LotMethod};
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

// ── Test fixtures ────────────────────────────────────────────────────────────────────────────────

fn cb() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}

fn ev_in(rf: &str, ts: time::OffsetDateTime, wal: &WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wal.clone()),
        payload: p,
    }
}

fn buy_in(
    rf: &str,
    ts: time::OffsetDateTime,
    wal: &WalletId,
    sat: i64,
    cost: rust_decimal::Decimal,
) -> LedgerEvent {
    ev_in(
        rf,
        ts,
        wal,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}

fn sell_in(
    rf: &str,
    ts: time::OffsetDateTime,
    wal: &WalletId,
    sat: i64,
    proceeds: rust_decimal::Decimal,
) -> LedgerEvent {
    ev_in(
        rf,
        ts,
        wal,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
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

fn election(seq: u64, made: time::OffsetDateTime, eff: time::Date, m: LotMethod) -> LedgerEvent {
    dec_ev(
        seq,
        made,
        EventPayload::MethodElection(MethodElection {
            effective_from: eff,
            method: m,
        }),
    )
}

fn lot_selection(
    seq: u64,
    made: time::OffsetDateTime,
    disposal_ref: &str,
    picks: Vec<LotPick>,
) -> LedgerEvent {
    dec_ev(
        seq,
        made,
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new(disposal_ref)),
            lots: picks,
        }),
    )
}

fn pid(rf: &str) -> LotId {
    LotId {
        origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        split_sequence: 0,
    }
}

/// Run the projection and return the compliance status of the single post-2025 disposal.
fn status_of(evs: &[LedgerEvent]) -> ComplianceStatus {
    let st = project(evs, &StaticPrices::default(), &ProjectionConfig::default());
    let dc = disposal_compliance(evs, &st);
    assert_eq!(
        dc.len(),
        1,
        "expected exactly one post-2025 disposal-compliance entry"
    );
    dc[0].status.clone()
}

// ── Tests ────────────────────────────────────────────────────────────────────────────────────────

/// §A.5(a): a `MethodElection` that is in-force at the time of sale confers `StandingOrder`.
/// Self-custody wallet, 2025 disposal, election effective 2025-06 (before the sale in 2025-07).
#[test]
fn standing_order_status_self_custody() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(50.00),
        ),
        election(
            1,
            datetime!(2025-05-01 00:00:00 UTC),
            date!(2025 - 06 - 01),
            LotMethod::Hifo,
        ),
        sell_in(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(70.00),
        ),
    ];
    assert!(
        matches!(status_of(&evs), ComplianceStatus::StandingOrder { effective_from } if effective_from == date!(2025-06-01)),
        "expected StandingOrder {{ effective_from: 2025-06-01 }}"
    );
}

/// §A.5(b): a `LotSelection` whose made-date is on the same day as the disposal is Contemporaneous.
#[test]
fn contemporaneous_status_when_selection_made_before_sale() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(50.00),
        ),
        sell_in(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(70.00),
        ),
        lot_selection(
            1,
            datetime!(2025-07-01 00:00:00 UTC),
            "D",
            vec![LotPick {
                lot: pid("A"),
                sat: 100_000,
            }],
        ),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::Contemporaneous);
}

/// §1.1012-1(j) no post-hoc: a `LotSelection` whose made-date (2025-09-01) is AFTER the disposal
/// (2025-07-01) is NonCompliant.
#[test]
fn post_hoc_selection_is_noncompliant() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(50.00),
        ),
        sell_in(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(70.00),
        ),
        lot_selection(
            1,
            datetime!(2025-09-01 00:00:00 UTC),
            "D",
            vec![LotPick {
                lot: pid("A"),
                sat: 100_000,
            }],
        ),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
}

/// Self-custody sell with no election and no selection: FIFO is the defensible fall-through but
/// the identification basis is absent → NonCompliant.
#[test]
fn noncompliant_when_no_basis() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(50.00),
        ),
        sell_in(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(70.00),
        ),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
}

/// Exchange (broker) wallet, 2027 disposal: own-books identification is insufficient under the
/// broker-communication rule → NonCompliant, even with an in-force election and contemporaneous
/// selection.
#[test]
fn broker_2027_plus_is_noncompliant_even_with_election() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cb(),
            100_000,
            dec!(50.00),
        ),
        election(
            1,
            datetime!(2025-05-01 00:00:00 UTC),
            date!(2025 - 06 - 01),
            LotMethod::Hifo,
        ),
        sell_in(
            "D",
            datetime!(2027-03-01 00:00:00 UTC),
            &cb(),
            100_000,
            dec!(70.00),
        ),
        lot_selection(
            2,
            datetime!(2027-03-01 00:00:00 UTC),
            "D",
            vec![LotPick {
                lot: pid("A"),
                sat: 100_000,
            }],
        ),
    ];
    assert_eq!(status_of(&evs), ComplianceStatus::NonCompliant);
}

/// Exchange (broker) wallet, 2026 disposal, in-force own-books election: the 2025-2026 relief
/// envelope applies → StandingOrder.
#[test]
fn broker_2026_own_books_election_is_standing_order() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cb(),
            100_000,
            dec!(50.00),
        ),
        election(
            1,
            datetime!(2025-05-01 00:00:00 UTC),
            date!(2025 - 06 - 01),
            LotMethod::Hifo,
        ),
        sell_in(
            "D",
            datetime!(2026-03-01 00:00:00 UTC),
            &cb(),
            100_000,
            dec!(70.00),
        ),
    ];
    assert!(
        matches!(status_of(&evs), ComplianceStatus::StandingOrder { .. }),
        "expected StandingOrder for broker 2026 disposal with in-force election"
    );
}

/// NFR4 determinism: reversing the slice order of events must not change the compliance result
/// (decision_seq ordering, not slice order, governs).
#[test]
fn determinism_load_order_independent() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(50.00),
        ),
        election(
            1,
            datetime!(2025-05-01 00:00:00 UTC),
            date!(2025 - 06 - 01),
            LotMethod::Hifo,
        ),
        sell_in(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(70.00),
        ),
    ];
    let mut evs_rev = evs.clone();
    evs_rev.reverse();

    let st1 = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let st2 = project(
        &evs_rev,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let dc1 = disposal_compliance(&evs, &st1);
    let dc2 = disposal_compliance(&evs_rev, &st2);

    assert_eq!(dc1.len(), dc2.len(), "entry counts must match");
    assert_eq!(
        dc1[0].status, dc2[0].status,
        "compliance status must be identical regardless of slice order"
    );
}

/// Custody mapping: `WalletId::SelfCustody` is own-books in all years — a 2027 self-custody
/// disposal with an in-force election remains `StandingOrder` (no broker-communication constraint).
#[test]
fn self_custody_2027_with_election_is_standing_order() {
    let evs = vec![
        buy_in(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(50.00),
        ),
        election(
            1,
            datetime!(2025-05-01 00:00:00 UTC),
            date!(2025 - 06 - 01),
            LotMethod::Lifo,
        ),
        sell_in(
            "D",
            datetime!(2027-04-01 00:00:00 UTC),
            &cold(),
            100_000,
            dec!(90.00),
        ),
    ];
    assert!(
        matches!(status_of(&evs), ComplianceStatus::StandingOrder { .. }),
        "self-custody 2027 disposal with in-force election must be StandingOrder"
    );
}
