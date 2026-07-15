//! Per-year `return_inputs_draft(year, inputs_json, schema_version, parked)` side-table — the input form's
//! crash-recovery scratch, INVISIBLE to `resolve.rs`. `parked = 1` marks a parked committed return (C-1).
//!
//! This is plan-2 task 1: the table + low-level row I/O only. `DraftRow`/`get_draft_row`/`set_draft_row`/
//! `parked_flag` are `pub(crate)` and have no callers yet — task 3 (`load`/`save_draft`/`commit`) and task
//! 4 (the park/unpark toggle) are the consumers. `#[allow(dead_code)]` on those items until then; the
//! roundtrip test below is the only caller in this task.
#![allow(dead_code)]
use crate::return_inputs::SCHEMA_VERSION;
use crate::CliError;
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

pub(crate) struct DraftRow {
    pub ri: ReturnInputs,
    pub version: i64,
    pub parked: bool,
}

/// The RAW draft row — does NOT gate on `SCHEMA_VERSION` (Task 3 `load` decides discard-vs-refuse per §6.3).
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

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use btctax_core::tax::types::FilingStatus;
    use rusqlite::Connection;

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
