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
fn imp(src_ref: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(src_ref)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
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

#[test]
fn unresolved_import_conflict_blocks_and_keeps_original() {
    let buy = imp(
        "A",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let new = EventPayload::Acquire(Acquire {
        sat: 100_000,
        usd_cost: dec!(99.00),
        fee_usd: dec!(0),
        basis_source: BasisSource::ExchangeProvided,
    });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let conflict = LedgerEvent {
        id: EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp),
        utc_timestamp: datetime!(2025-03-02 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::ImportConflict(ImportConflict {
            target: EventId::import(Source::Coinbase, SourceRef::new("A")),
            new_payload: Box::new(new),
            new_fingerprint: fp,
        }),
    };
    let st = project(
        &[buy, conflict],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::ImportConflict));
    assert_eq!(st.lots[0].usd_basis, dec!(60.00)); // original kept until resolved
}

#[test]
fn supersede_applies_new_payload_to_same_target_id() {
    let buy = imp(
        "A",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let new = EventPayload::Acquire(Acquire {
        sat: 100_000,
        usd_cost: dec!(99.00),
        fee_usd: dec!(0),
        basis_source: BasisSource::ExchangeProvided,
    });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let cid = EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp);
    let conflict = LedgerEvent {
        id: cid.clone(),
        utc_timestamp: datetime!(2025-03-02 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::ImportConflict(ImportConflict {
            target: EventId::import(Source::Coinbase, SourceRef::new("A")),
            new_payload: Box::new(new),
            new_fingerprint: fp,
        }),
    };
    let sup = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::SupersedeImport(SupersedeImport {
            conflict_event: cid,
        }),
    );
    let st = project(
        &[buy, conflict, sup],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st
        .blockers
        .iter()
        .all(|b| b.kind != BlockerKind::ImportConflict)); // resolved
    assert_eq!(st.lots[0].usd_basis, dec!(99.00)); // new payload applied, same lot origin id
}

#[test]
fn void_of_supersede_is_a_decision_conflict_not_a_drop() {
    // SupersedeImport is non-revocable; voiding it must NOT silently drop it.
    let buy = imp(
        "A",
        datetime!(2025-03-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(60.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let new = EventPayload::Acquire(Acquire {
        sat: 100_000,
        usd_cost: dec!(99.00),
        fee_usd: dec!(0),
        basis_source: BasisSource::ExchangeProvided,
    });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let cid = EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp);
    let conflict = LedgerEvent {
        id: cid.clone(),
        utc_timestamp: datetime!(2025-03-02 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::ImportConflict(ImportConflict {
            target: EventId::import(Source::Coinbase, SourceRef::new("A")),
            new_payload: Box::new(new),
            new_fingerprint: fp,
        }),
    };
    let sup = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::SupersedeImport(SupersedeImport {
            conflict_event: cid,
        }),
    );
    let void = dec_ev(
        2,
        datetime!(2026-02-01 00:00:00 UTC),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: EventId::decision(1),
        }),
    );
    let st = project(
        &[buy, conflict, sup, void],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::DecisionConflict));
    assert_eq!(st.lots[0].usd_basis, dec!(99.00)); // supersede still in force
}

#[test]
fn late_supersede_rewrites_an_earlier_year_deterministically() {
    // A 2026 SupersedeImport rewrites a 2022 Acquire's basis; result independent of event order.
    let buy = imp(
        "A",
        datetime!(2022-06-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: dec!(20.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let new = EventPayload::Acquire(Acquire {
        sat: 100_000,
        usd_cost: dec!(25.00),
        fee_usd: dec!(0),
        basis_source: BasisSource::ExchangeProvided,
    });
    let fp = btctax_core::persistence::fingerprint(&new).unwrap();
    let cid = EventId::conflict(Source::Coinbase, SourceRef::new("A"), &fp);
    let conflict = LedgerEvent {
        id: cid.clone(),
        utc_timestamp: datetime!(2022-06-02 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: EventPayload::ImportConflict(ImportConflict {
            target: EventId::import(Source::Coinbase, SourceRef::new("A")),
            new_payload: Box::new(new),
            new_fingerprint: fp,
        }),
    };
    let sup = dec_ev(
        1,
        datetime!(2026-01-01 00:00:00 UTC),
        EventPayload::SupersedeImport(SupersedeImport {
            conflict_event: cid,
        }),
    );
    let s1 = project(
        &[buy.clone(), conflict.clone(), sup.clone()],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let s2 = project(
        &[sup, buy, conflict],
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert_eq!(s1, s2);
    assert_eq!(s1.lots[0].usd_basis, dec!(25.00));
}
