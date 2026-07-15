//! Per-year `return_inputs_draft(year, inputs_json, schema_version, parked)` side-table — the input form's
//! crash-recovery scratch, INVISIBLE to `resolve.rs`. `parked = 1` marks a parked committed return (C-1).
//!
//! Plan-2 task 1 built the table + low-level row I/O; task 2 adds `save_draft` (the autosave primitive).
//! `set_draft_row`/`parked_flag` now have a non-test caller (`save_draft`), so they carry no `dead_code`
//! allow. `DraftRow`/`get_draft_row` are still only read by tests (task 3's `load` is their first non-test
//! consumer), so they each carry a per-item `#[allow(dead_code)]` until then — per the codebase's per-item
//! convention (see `tests/fixtures.rs`), not a broad module-level allow.
use crate::return_inputs::SCHEMA_VERSION;
use crate::{CliError, Session};
use btctax_core::tax::return_inputs::ReturnInputs;
use rusqlite::Connection;

/// Create the draft side-table if absent. Idempotent; called first by every fn (safe on an older vault).
pub fn init_draft_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS return_inputs_draft (\
             year INTEGER PRIMARY KEY, inputs_json TEXT NOT NULL, \
             schema_version INTEGER NOT NULL DEFAULT 0, parked INTEGER NOT NULL DEFAULT 0)",
        [],
    )?;
    Ok(())
}

#[allow(dead_code)] // read only by tests until task 3's `load`
pub(crate) struct DraftRow {
    pub ri: ReturnInputs,
    pub version: i64,
    pub parked: bool,
}

/// The RAW draft row — does NOT gate on `SCHEMA_VERSION` (Task 3 `load` decides discard-vs-refuse per §6.3).
#[allow(dead_code)] // called only by tests until task 3's `load`
pub(crate) fn get_draft_row(conn: &Connection, year: i32) -> Result<Option<DraftRow>, CliError> {
    init_draft_table(conn)?;
    let row = conn.query_row(
        "SELECT inputs_json, schema_version, parked FROM return_inputs_draft WHERE year=?1",
        [year],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)),
    );
    match row {
        Ok((json, version, parked)) => {
            // ★ I-A: CliError has NO From<serde_json::Error> — map explicitly like return_inputs.rs:66-69
            // (a bad blob is a typed error, not a `?`-panic). Do NOT use `?` on serde here.
            let ri: ReturnInputs = serde_json::from_str(&json).map_err(|e| CliError::BadConfigValue {
                key: format!("return_inputs_draft[{year}]"),
                value: format!("invalid JSON: {e}"),
            })?;
            Ok(Some(DraftRow { ri, version, parked: parked != 0 }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub(crate) fn set_draft_row(
    conn: &Connection,
    year: i32,
    ri: &ReturnInputs,
    parked: bool,
) -> Result<(), CliError> {
    init_draft_table(conn)?;
    // ★ I-A: map serde explicitly (no From<serde_json::Error> on CliError) — mirror return_inputs.rs:92-95.
    let j = serde_json::to_string(ri).map_err(|e| CliError::BadConfigValue {
        key: format!("return_inputs_draft[{year}]"),
        value: format!("could not serialize: {e}"),
    })?;
    conn.execute(
        "INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(?1,?2,?3,?4) \
         ON CONFLICT(year) DO UPDATE SET inputs_json=?2, schema_version=?3, parked=?4",
        rusqlite::params![year, j, SCHEMA_VERSION, parked as i64],
    )?;
    Ok(())
}

pub fn delete_draft(conn: &Connection, year: i32) -> Result<bool, CliError> {
    init_draft_table(conn)?;
    Ok(conn.execute("DELETE FROM return_inputs_draft WHERE year=?1", [year])? > 0)
}

pub fn draft_exists(conn: &Connection, year: i32) -> Result<bool, CliError> {
    init_draft_table(conn)?;
    Ok(conn
        .query_row("SELECT 1 FROM return_inputs_draft WHERE year=?1", [year], |_| Ok(()))
        .is_ok())
}

pub(crate) fn parked_flag(conn: &Connection, year: i32) -> Result<Option<bool>, CliError> {
    init_draft_table(conn)?;
    match conn.query_row(
        "SELECT parked FROM return_inputs_draft WHERE year=?1",
        [year],
        |r| r.get::<_, i64>(0),
    ) {
        Ok(p) => Ok(Some(p != 0)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Autosave a mid-entry return to the draft table (the input form's crash-recovery scratch).
///
/// Read-modify-write that PRESERVES the row's `parked` flag (NI-1): a parked committed return stays
/// parked across edits. Reads the existing flag (default `false` — a fresh year is WIP, not parked),
/// upserts `ri` with that flag, then `sess.save()` re-encrypts and atomically writes the vault to disk
/// (I-7) — nothing survives a crash until `save()` returns. Takes `&mut Session` for `save()`; reads
/// through the SAME session's `conn()` (never opens a second Session — N-1).
pub fn save_draft(sess: &mut Session, year: i32, ri: &ReturnInputs) -> Result<(), CliError> {
    let parked = parked_flag(sess.conn(), year)?.unwrap_or(false); // ★ NI-1: preserve; default WIP
    set_draft_row(sess.conn(), year, ri, parked)?;
    sess.save()?; // ★ I-7: reach disk
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;
    use btctax_store::Passphrase;
    use rusqlite::Connection;

    fn pp() -> Passphrase {
        Passphrase::new("test-pass".into())
    }

    /// Shared temp-vault fixture (M-3). MUST return the `TempDir` guard — if it drops, the temp
    /// dir (and the vault file inside it) is deleted before any later `Session::open`. `Session::create`
    /// is dropped inside the block so the store single-instance lock is released and `open` can
    /// re-acquire it (N-1). Later tasks' tests (T4/T6/T7) reuse this helper.
    fn tmp_vault() -> (tempfile::TempDir, std::path::PathBuf, Passphrase) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.pgp");
        {
            let _ = Session::create(&path, &pp()).unwrap(); // create + drop releases the lock
        }
        (dir, path, pp())
    }

    #[test]
    fn save_draft_preserves_parked_and_reaches_disk() {
        let (_dir, path, pp) = tmp_vault();
        let ri_a = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        {
            let mut sess = Session::open(&path, &pp).unwrap();
            // seed a PARKED draft directly, then save_draft an edit — parked must survive.
            set_draft_row(sess.conn(), 2024, &ri_a, true).unwrap();
            let ri_b = ReturnInputs {
                filing_status: FilingStatus::Mfj,
                ..Default::default()
            };
            save_draft(&mut sess, 2024, &ri_b).unwrap();
            assert_eq!(
                parked_flag(sess.conn(), 2024).unwrap(),
                Some(true),
                "NI-1: parked survives an edit"
            );
        }
        // reopen a fresh Session — proves save_draft reached disk (I-7), not just the in-memory conn.
        let sess2 = Session::open(&path, &pp).unwrap();
        let row = get_draft_row(sess2.conn(), 2024).unwrap().unwrap();
        assert_eq!(row.ri.filing_status, FilingStatus::Mfj);
        assert!(row.parked);
    }

    #[test]
    fn save_draft_on_fresh_year_is_unparked() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        save_draft(&mut sess, 2024, &ReturnInputs::default()).unwrap();
        assert_eq!(parked_flag(sess.conn(), 2024).unwrap(), Some(false));
    }

    #[test]
    fn draft_row_set_get_delete_roundtrip_with_parked() {
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let ri = ReturnInputs { filing_status: FilingStatus::Mfj, ..Default::default() };
        // WIP row
        set_draft_row(&conn, 2024, &ri, false).unwrap();
        let got = get_draft_row(&conn, 2024).unwrap().unwrap();
        assert_eq!(got.ri.filing_status, FilingStatus::Mfj);
        assert_eq!(got.version, SCHEMA_VERSION);
        assert!(!got.parked);
        assert_eq!(parked_flag(&conn, 2024).unwrap(), Some(false));
        // upgrade to parked
        set_draft_row(&conn, 2024, &ri, true).unwrap();
        assert!(get_draft_row(&conn, 2024).unwrap().unwrap().parked);
        // delete
        assert!(delete_draft(&conn, 2024).unwrap());
        assert!(get_draft_row(&conn, 2024).unwrap().is_none());
        assert!(!delete_draft(&conn, 2024).unwrap()); // idempotent
    }
}
