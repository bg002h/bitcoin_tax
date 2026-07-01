//! `donation_details` side-table — persists Form 8283 Section-B appraiser + structured-donee
//! metadata for each donation event. Keyed by the donation's `EventId::canonical()` string.
//! Modeled on `optimize_attest.rs` discipline (idempotent DDL, defensive `init_table` guard on
//! every accessor, JSON storage via serde_json — same as `tax_profile.rs`). The data is
//! POST-HOC form-completion metadata; it does NOT enter the fold or projection.
use crate::CliError;
use btctax_core::{DonationDetails, EventId};
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

/// Create the `donation_details` side-table if it does not exist (idempotent).
///
/// `CREATE TABLE IF NOT EXISTS` makes this safe to call on any vault — new, old, or restored
/// from snapshot — without errors. Called by `Session::from_fresh_vault`; also called at the
/// top of every `get`/`set`/`all` as a defensive ensure-table-then-read guard (robust to older
/// vaults that predate Chunk 3b, mirroring the `optimize_attest` back-compat pattern).
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS donation_details \
         (donation_event TEXT PRIMARY KEY, details_json TEXT NOT NULL);",
    )?;
    Ok(())
}

/// Return the stored `DonationDetails` for `event`, or `None` if none has been set.
///
/// Robust to older vaults (calls `init_table` first so "no such table" is never returned).
/// Returns `CliError::BadConfigValue` if the stored JSON is malformed.
pub fn get(conn: &Connection, event: &EventId) -> Result<Option<DonationDetails>, CliError> {
    init_table(conn)?;
    let json: Option<String> = conn
        .query_row(
            "SELECT details_json FROM donation_details WHERE donation_event=?1",
            [event.canonical()],
            |r| r.get(0),
        )
        .optional()?;
    match json {
        None => Ok(None),
        Some(j) => Ok(Some(serde_json::from_str(&j).map_err(|e| {
            CliError::BadConfigValue {
                key: format!("donation_details[{}]", event.canonical()),
                value: format!("invalid JSON: {e}"),
            }
        })?)),
    }
}

/// Persist `details` for `event` (upsert — replaces any prior value; last-write-wins).
///
/// Idempotent DDL guard first. JSON via serde_json (mirrors `tax_profile::set`).
pub fn set(conn: &Connection, event: &EventId, details: &DonationDetails) -> Result<(), CliError> {
    init_table(conn)?;
    let j = serde_json::to_string(details).map_err(|e| CliError::BadConfigValue {
        key: format!("donation_details[{}]", event.canonical()),
        value: e.to_string(),
    })?;
    conn.execute(
        "INSERT INTO donation_details(donation_event,details_json) VALUES(?1,?2) \
         ON CONFLICT(donation_event) DO UPDATE SET details_json=excluded.details_json",
        rusqlite::params![event.canonical(), j],
    )?;
    Ok(())
}

/// Return all stored `DonationDetails`, keyed by the donation `EventId` (NFR4-stable
/// `BTreeMap` — deterministic iteration order). Defensive `init_table` guard first.
pub fn all(conn: &Connection) -> Result<BTreeMap<EventId, DonationDetails>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare(
        "SELECT donation_event, details_json \
         FROM donation_details ORDER BY donation_event",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (ev_str, j) = row?;
        let eid = crate::eventref::parse_event_id(&ev_str)?;
        let details: DonationDetails =
            serde_json::from_str(&j).map_err(|e| CliError::BadConfigValue {
                key: format!("donation_details[{ev_str}]"),
                value: format!("invalid JSON: {e}"),
            })?;
        out.insert(eid, details);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{EventId, Source, SourceRef};
    use time::macros::date;

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_table(&c).unwrap();
        c
    }

    fn eid(label: &str) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(label.to_string()))
    }

    /// Full details — all optional fields populated. PRIVACY: no real PII.
    fn full_details() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: Some("123 Main St, Anytown USA".into()),
            donee_ein: Some("12-3456789".into()),
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: Some("456 Appraiser Ave".into()),
            appraiser_tin: Some("987654321".into()),
            appraiser_ptin: Some("P01234567".into()),
            appraiser_qualifications: Some("Certified bitcoin property appraiser".into()),
            appraisal_date: Some(date!(2025 - 06 - 01)),
            fmv_method_override: Some("qualified appraisal".into()),
        }
    }

    /// Minimal details — only required fields; all optionals are None.
    fn minimal_details() -> DonationDetails {
        DonationDetails {
            donee_name: "Test Charity".into(),
            donee_address: None,
            donee_ein: None,
            appraiser_name: "Test Appraiser".into(),
            appraiser_address: None,
            appraiser_tin: None,
            appraiser_ptin: None,
            appraiser_qualifications: None,
            appraisal_date: None,
            fmv_method_override: None,
        }
    }

    /// Full round-trip: all optional fields populated (incl. appraisal_date) survive JSON.
    #[test]
    fn set_then_get_round_trips_full_details() {
        let c = mem();
        let e = eid("out|test-donation-full");
        set(&c, &e, &full_details()).unwrap();
        let stored = get(&c, &e).unwrap().unwrap();
        assert_eq!(stored, full_details());
    }

    /// Minimal round-trip: all optional fields are None (serde default).
    #[test]
    fn set_then_get_round_trips_minimal_details() {
        let c = mem();
        let e = eid("out|test-donation-minimal");
        set(&c, &e, &minimal_details()).unwrap();
        let stored = get(&c, &e).unwrap().unwrap();
        assert_eq!(stored, minimal_details());
    }

    /// A missing key returns None (not an error).
    #[test]
    fn get_missing_returns_none() {
        let c = mem();
        assert_eq!(get(&c, &eid("out|no-such-event")).unwrap(), None);
    }

    /// Back-compat: a vault that has NO donation_details table (an "old" vault) → `get` returns
    /// None (the defensive `init_table` guard creates the table transparently on first access).
    #[test]
    fn get_on_tableless_vault_returns_none() {
        // No init_table call — simulates an old vault opened for the first time post-Chunk-3b.
        let c = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(get(&c, &eid("out|test")).unwrap(), None);
    }

    /// Back-compat: `all()` on a tableless vault returns an empty map (not an error).
    #[test]
    fn all_on_tableless_vault_returns_empty_map() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        assert!(all(&c).unwrap().is_empty());
    }

    /// `init_table` is idempotent — calling it multiple times must not error.
    #[test]
    fn init_table_is_idempotent() {
        let c = mem(); // already called init_table
        init_table(&c).unwrap();
        init_table(&c).unwrap();
        assert!(all(&c).unwrap().is_empty());
    }

    /// Upsert: a second `set` for the same key replaces the prior value.
    #[test]
    fn upsert_replaces_prior_details() {
        let c = mem();
        let e = eid("out|donation-upsert");
        set(&c, &e, &minimal_details()).unwrap();
        set(&c, &e, &full_details()).unwrap();
        let stored = get(&c, &e).unwrap().unwrap();
        assert_eq!(stored, full_details());
    }

    /// Defensive-guard path: no explicit `init_table` called, but `set` + `get` still work.
    #[test]
    fn defensive_guard_in_set_creates_table() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        let e = eid("out|guard-test");
        // No init_table — the defensive guard inside `set` must create the table.
        set(&c, &e, &minimal_details()).unwrap();
        assert_eq!(get(&c, &e).unwrap().unwrap(), minimal_details());
    }

    /// `all()` returns a `BTreeMap` ordered by EventId (NFR4 determinism).
    #[test]
    fn all_returns_btreemap_in_deterministic_order() {
        let c = mem();
        let e1 = eid("out|donation-alpha");
        let e2 = eid("out|donation-beta");
        // Insert in reverse order; BTreeMap must impose deterministic order.
        set(&c, &e2, &full_details()).unwrap();
        set(&c, &e1, &minimal_details()).unwrap();
        let m = all(&c).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&e1).unwrap(), &minimal_details());
        assert_eq!(m.get(&e2).unwrap(), &full_details());
    }
}
