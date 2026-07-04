//! `bulk_estimated_proceeds` side-table — flags each disposal whose proceeds were derived from an
//! auto-FMV (daily-close market value) by the bulk-reclassify-outflow path, rather than a real
//! user-entered sale price. Keyed by the `EventId::canonical()` string of the `transfer_out_event`,
//! which IS the eventual `Disposal.event` (fold pushes `Disposal{event: eff.id.clone()}` using the
//! ORIGINAL TransferOut id — so the Disposals-tab join lands). Modeled on `donation_details.rs` /
//! `optimize_attest.rs` discipline (idempotent DDL, defensive `init_table` guard on every accessor).
//!
//! **Stores ONLY the flag + a `date_marked` provenance stamp — NEVER the estimate numbers** [R0-M3].
//! The Disposals tab renders the EXACT fold-computed `leg.proceeds/basis/gain` plus an `[est]` marker;
//! a stored preview figure must never override the exact numbers. The flag lives OUTSIDE the
//! append-only event log entirely (btctax-core is unchanged; no serde risk).
use crate::CliError;
use btctax_core::EventId;
use rusqlite::Connection;
use std::collections::BTreeMap;

/// Create the `bulk_estimated_proceeds` side-table if it does not exist (idempotent).
///
/// `CREATE TABLE IF NOT EXISTS` makes this safe to call on any vault — new, old, or restored from
/// snapshot — without errors. Called by `Session::from_fresh_vault`; also called at the top of every
/// `mark`/`clear`/`all` as a defensive ensure-table-then-read guard (robust to older vaults that
/// predate this table, mirroring the `donation_details`/`optimize_attest` back-compat pattern).
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS bulk_estimated_proceeds \
         (out_event TEXT PRIMARY KEY, date_marked TEXT NOT NULL);",
    )?;
    Ok(())
}

/// Flag `out_event`'s disposal as estimated-FMV proceeds (upsert — last-write-wins). `date_marked`
/// is an ISO-8601 provenance stamp (the batch made-date). Idempotent DDL guard first.
pub fn mark(conn: &Connection, out_event: &EventId, date_marked: &str) -> Result<(), CliError> {
    init_table(conn)?;
    conn.execute(
        "INSERT INTO bulk_estimated_proceeds(out_event,date_marked) VALUES(?1,?2) \
         ON CONFLICT(out_event) DO UPDATE SET date_marked=excluded.date_marked",
        rusqlite::params![out_event.canonical(), date_marked],
    )?;
    Ok(())
}

/// Delete the `bulk_estimated_proceeds` row for `out_event` (idempotent — no-op if absent).
///
/// Called when the `ReclassifyOutflow` decision for `out_event` is VOIDED, so a re-reclassify via
/// single `o` (which may carry a REAL price) does not inherit a stale `[est]` marker. Clearing an
/// absent row is `Ok`, so single-`o` reclassifies that were never flagged are unaffected.
pub fn clear(conn: &Connection, out_event: &EventId) -> Result<(), CliError> {
    init_table(conn)?;
    conn.execute(
        "DELETE FROM bulk_estimated_proceeds WHERE out_event=?1",
        [out_event.canonical()],
    )?;
    Ok(())
}

/// Return all flagged disposals as a `BTreeMap<EventId, String>` (value = `date_marked`), keyed by the
/// `transfer_out_event` / `Disposal.event` (NFR4-stable deterministic order). Defensive DDL guard first.
pub fn all(conn: &Connection) -> Result<BTreeMap<EventId, String>, CliError> {
    init_table(conn)?;
    let mut stmt = conn
        .prepare("SELECT out_event, date_marked FROM bulk_estimated_proceeds ORDER BY out_event")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (ev_str, date_marked) = row?;
        out.insert(crate::eventref::parse_event_id(&ev_str)?, date_marked);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{EventId, Source, SourceRef};

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_table(&c).unwrap();
        c
    }

    fn eid(label: &str) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(label.to_string()))
    }

    #[test]
    fn mark_then_all_contains_the_key() {
        let c = mem();
        let e = eid("out|bulk-est-1");
        mark(&c, &e, "2026-07-03").unwrap();
        let m = all(&c).unwrap();
        assert_eq!(m.get(&e).map(String::as_str), Some("2026-07-03"));
    }

    #[test]
    fn clear_removes_the_flag_and_is_idempotent() {
        let c = mem();
        let e = eid("out|bulk-est-clear");
        mark(&c, &e, "2026-07-03").unwrap();
        assert!(all(&c).unwrap().contains_key(&e));
        clear(&c, &e).unwrap();
        assert!(!all(&c).unwrap().contains_key(&e));
        // Idempotent: clearing an absent row is Ok.
        clear(&c, &e).unwrap();
        assert!(!all(&c).unwrap().contains_key(&e));
    }

    #[test]
    fn clear_absent_key_is_ok() {
        let c = mem();
        // Never marked — clear is a harmless no-op (covers the single-`o`-never-flagged path).
        clear(&c, &eid("out|never-marked")).unwrap();
        assert!(all(&c).unwrap().is_empty());
    }

    #[test]
    fn mark_upserts_last_write_wins() {
        let c = mem();
        let e = eid("out|bulk-est-upsert");
        mark(&c, &e, "2026-01-01").unwrap();
        mark(&c, &e, "2026-07-03").unwrap();
        assert_eq!(
            all(&c).unwrap().get(&e).map(String::as_str),
            Some("2026-07-03")
        );
    }

    #[test]
    fn all_on_tableless_vault_returns_empty() {
        let c = rusqlite::Connection::open_in_memory().unwrap(); // no init_table
        assert!(all(&c).unwrap().is_empty());
    }

    #[test]
    fn defensive_guard_in_mark_creates_table() {
        let c = rusqlite::Connection::open_in_memory().unwrap(); // no init_table
        let e = eid("out|guard");
        mark(&c, &e, "2026-07-03").unwrap();
        assert!(all(&c).unwrap().contains_key(&e));
    }

    #[test]
    fn all_returns_btreemap_in_deterministic_order() {
        let c = mem();
        let e1 = eid("out|alpha");
        let e2 = eid("out|beta");
        mark(&c, &e2, "2026-07-03").unwrap();
        mark(&c, &e1, "2026-07-02").unwrap();
        let m = all(&c).unwrap();
        assert_eq!(m.len(), 2);
        assert!(m.contains_key(&e1) && m.contains_key(&e2));
    }
}
