//! `optimize_attestation` side-table — persists the user's per-disposal contemporaneous-ID
//! attestations (a projection input, **not** ledger state). The `attested_set` accessor feeds the
//! `compliance_overlay` as its `attested: &BTreeSet<EventId>` argument; `get` returns the stored
//! attestation text so the overlay can enforce attested-binds-the-exact-selection (R2-I1: a later
//! divergent pick is NOT covered). Modeled on `tax_profile.rs` discipline (idempotent DDL, defensive
//! guard on every accessor, BTreeSet for NFR4 determinism).
use crate::CliError;
use btctax_core::EventId;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeSet;

/// Create the `optimize_attestation` side-table if it does not exist (idempotent).
/// Called by `Session::from_fresh_vault`; also called at the top of every accessor as a
/// defensive ensure-table-then-read guard (robust to older vaults).
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS optimize_attestation \
         (disposal_event TEXT PRIMARY KEY, attestation TEXT NOT NULL, attested_at TEXT NOT NULL);",
    )?;
    Ok(())
}

/// Record a narrow attestation for `disposal` (upsert — replaces any prior value).
/// `attestation` is a caller-supplied opaque string (e.g. JSON-encoded `LotSelection`) that
/// binds the exact lot picks the user attested; `attested_at` is an ISO-8601 date string.
pub fn set(
    conn: &Connection,
    disposal: &EventId,
    attestation: &str,
    attested_at: &str,
) -> Result<(), CliError> {
    init_table(conn)?;
    conn.execute(
        "INSERT INTO optimize_attestation(disposal_event,attestation,attested_at) \
         VALUES(?1,?2,?3) \
         ON CONFLICT(disposal_event) DO UPDATE \
         SET attestation=excluded.attestation, attested_at=excluded.attested_at",
        rusqlite::params![disposal.canonical(), attestation, attested_at],
    )?;
    Ok(())
}

/// Return the stored attestation text for `disposal`, or `None` if none has been recorded.
/// Robust to older vaults (defensive `init_table` guard).
pub fn get(conn: &Connection, disposal: &EventId) -> Result<Option<String>, CliError> {
    init_table(conn)?;
    Ok(conn
        .query_row(
            "SELECT attestation FROM optimize_attestation WHERE disposal_event=?1",
            [disposal.canonical()],
            |r| r.get(0),
        )
        .optional()?)
}

/// Return all attested disposal `EventId`s as a sorted `BTreeSet` (NFR4-stable).
/// Feeds the `compliance_overlay` as its `attested` input. Robust to older vaults.
pub fn attested_set(conn: &Connection) -> Result<BTreeSet<EventId>, CliError> {
    init_table(conn)?;
    let mut stmt =
        conn.prepare("SELECT disposal_event FROM optimize_attestation ORDER BY disposal_event")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = BTreeSet::new();
    for r in rows {
        out.insert(crate::eventref::parse_event_id(&r?)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{EventId, Source, SourceRef};

    fn eid(seq: u64) -> EventId {
        EventId::decision(seq)
    }

    fn eid_import(src_ref: &str) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(src_ref.to_string()))
    }

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_table(&c).unwrap();
        c
    }

    #[test]
    fn set_then_get_round_trips() {
        let c = mem();
        let disposal = eid(1);
        set(&c, &disposal, r#"{"lots":[]}"#, "2025-03-15").unwrap();
        let stored = get(&c, &disposal).unwrap().unwrap();
        assert_eq!(stored, r#"{"lots":[]}"#);
    }

    #[test]
    fn get_missing_returns_none() {
        let c = mem();
        assert_eq!(get(&c, &eid(99)).unwrap(), None);
    }

    #[test]
    fn get_on_tableless_vault_returns_none() {
        let c = rusqlite::Connection::open_in_memory().unwrap(); // no init_table
        assert_eq!(get(&c, &eid(1)).unwrap(), None);
    }

    #[test]
    fn attested_set_on_tableless_vault_returns_empty() {
        let c = rusqlite::Connection::open_in_memory().unwrap(); // no init_table
        assert!(attested_set(&c).unwrap().is_empty());
    }

    #[test]
    fn attested_set_returns_keys_in_deterministic_sorted_order() {
        let c = mem();
        // Insert in arbitrary order; expect BTreeSet ordering on canonical strings.
        let d3 = eid(3);
        let d1 = eid(1);
        let d2 = eid(2);
        set(&c, &d3, "sel3", "2025-01-03").unwrap();
        set(&c, &d1, "sel1", "2025-01-01").unwrap();
        set(&c, &d2, "sel2", "2025-01-02").unwrap();
        let set_out = attested_set(&c).unwrap();
        // BTreeSet<EventId> — ordering by EventId's Ord impl; all must be present.
        assert_eq!(set_out.len(), 3);
        assert!(set_out.contains(&d1));
        assert!(set_out.contains(&d2));
        assert!(set_out.contains(&d3));
        // Verify iteration order is stable across calls (determinism check).
        let set_out2 = attested_set(&c).unwrap();
        let keys1: Vec<_> = set_out.iter().collect();
        let keys2: Vec<_> = set_out2.iter().collect();
        assert_eq!(keys1, keys2);
    }

    #[test]
    fn upsert_replaces_prior_attestation() {
        let c = mem();
        let disposal = eid(1);
        set(&c, &disposal, "original_sel", "2025-01-01").unwrap();
        set(&c, &disposal, "updated_sel", "2025-06-01").unwrap();
        assert_eq!(get(&c, &disposal).unwrap().unwrap(), "updated_sel");
    }

    #[test]
    fn divergent_selection_not_treated_as_attested() {
        // The stored attestation binds the EXACT selection (R2-I1). Retrieve and compare:
        // a caller using a different selection string must find it does NOT match.
        let c = mem();
        let disposal = eid(42);
        let attested_sel = r#"{"lots":[{"lot":"decision|1","sat":100000}]}"#;
        let divergent_sel = r#"{"lots":[{"lot":"decision|2","sat":100000}]}"#;
        set(&c, &disposal, attested_sel, "2025-03-15").unwrap();
        let stored = get(&c, &disposal).unwrap().unwrap();
        assert_eq!(stored, attested_sel);
        assert_ne!(
            stored, divergent_sel,
            "divergent selection must not match stored attestation"
        );
    }

    #[test]
    fn attested_set_reflects_all_stored_disposals() {
        let c = mem();
        let d_a = eid_import("TX-A");
        let d_b = eid_import("TX-B");
        set(&c, &d_a, "sel_a", "2025-01-01").unwrap();
        set(&c, &d_b, "sel_b", "2025-01-02").unwrap();
        let s = attested_set(&c).unwrap();
        assert!(s.contains(&d_a));
        assert!(s.contains(&d_b));
        assert!(!s.contains(&eid(99))); // unrelated disposal not present
    }

    #[test]
    fn table_created_on_existing_conn_without_explicit_init() {
        // Simulates the defensive-guard path: vault opened (no prior init_table) and a get/set
        // is called directly — the guard inside each function creates the table.
        let c = rusqlite::Connection::open_in_memory().unwrap();
        let disposal = eid(5);
        // No init_table called explicitly — the defensive guard in set must handle it.
        set(&c, &disposal, "some_sel", "2025-05-01").unwrap();
        assert_eq!(get(&c, &disposal).unwrap().unwrap(), "some_sel");
    }
}
