use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::persistence;
use rust_decimal_macros::dec;
use time::macros::{datetime, offset};

fn acq(source_ref: &str, cost: rust_decimal::Decimal) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(source_ref)),
        utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange {
            provider: "coinbase".into(),
            account: "main".into(),
        }),
        payload: EventPayload::Acquire(Acquire {
            sat: 100_000,
            usd_cost: cost,
            fee_usd: dec!(1.00),
            basis_source: BasisSource::ExchangeProvided,
        }),
    }
}

#[test]
fn round_trips_the_event_set() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.00)), acq("B", dec!(61.00))])
        .unwrap();
    let loaded = persistence::load_all(&conn).unwrap();
    assert_eq!(loaded.len(), 2);
}

#[test]
fn re_import_identical_is_idempotent() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.00))]).unwrap();
    // cosmetic variation: trailing-zero scale must NOT create a dup (same fingerprint).
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.0))]).unwrap();
    assert_eq!(persistence::load_all(&conn).unwrap().len(), 1);
}

#[test]
fn changed_row_appends_exactly_one_conflict_idempotently() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(60.00))]).unwrap();
    persistence::append_import_batch(&conn, &[acq("A", dec!(99.00))]).unwrap(); // changed
    persistence::append_import_batch(&conn, &[acq("A", dec!(99.00))]).unwrap(); // same change again
    let loaded = persistence::load_all(&conn).unwrap();
    let conflicts = loaded
        .iter()
        .filter(|e| matches!(e.payload, EventPayload::ImportConflict(_)))
        .count();
    assert_eq!(conflicts, 1); // one conflict total; the original Acquire is untouched
    assert_eq!(loaded.len(), 2);
}

#[test]
fn decisions_get_monotonic_seq_and_decision_event_ids() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&conn).unwrap();
    let id1 = persistence::append_decision(
        &conn,
        EventPayload::RejectImport(RejectImport {
            conflict_event: EventId::decision(0),
        }),
        datetime!(2026-01-01 00:00:00 UTC),
        offset!(+00:00),
        None,
    )
    .unwrap();
    let id2 = persistence::append_decision(
        &conn,
        EventPayload::RejectImport(RejectImport {
            conflict_event: EventId::decision(0),
        }),
        datetime!(2026-01-02 00:00:00 UTC),
        offset!(+00:00),
        None,
    )
    .unwrap();
    assert_eq!(id1, EventId::decision(1));
    assert_eq!(id2, EventId::decision(2));
}
