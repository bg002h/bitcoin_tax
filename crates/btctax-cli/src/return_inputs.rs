//! Per-year `return_inputs(year, inputs_json)` side-table — the full-return v1 input surface (line
//! items + PII + payments). A projection input, **not** ledger state (NFR6). Mirrors `tax_profile.rs`
//! exactly (idempotent DDL, robust-to-older-vaults guard, typed error on bad JSON) — one JSON-encoded
//! [`ReturnInputs`] per tax year, stored inside the encrypted vault.
use crate::CliError;
use btctax_core::tax::return_inputs::ReturnInputs;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

/// Create the `return_inputs` side-table if it does not exist (idempotent — safe on any vault).
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS return_inputs \
         (year INTEGER PRIMARY KEY, inputs_json TEXT NOT NULL);",
    )?;
    Ok(())
}

/// Return the stored [`ReturnInputs`] for `year`, or `None` if none has been set.
/// Robust to older vaults (ensures the table first); typed error on malformed JSON.
pub fn get(conn: &Connection, year: i32) -> Result<Option<ReturnInputs>, CliError> {
    init_table(conn)?;
    let json: Option<String> = conn
        .query_row(
            "SELECT inputs_json FROM return_inputs WHERE year=?1",
            [year],
            |r| r.get(0),
        )
        .optional()?;
    match json {
        None => Ok(None),
        Some(j) => Ok(Some(serde_json::from_str(&j).map_err(|e| {
            CliError::BadConfigValue {
                key: format!("return_inputs[{year}]"),
                value: format!("invalid JSON: {e}"),
            }
        })?)),
    }
}

/// Persist `ri` as the [`ReturnInputs`] for `year` (upsert — replaces any prior value).
pub fn set(conn: &Connection, year: i32, ri: &ReturnInputs) -> Result<(), CliError> {
    init_table(conn)?;
    let j = serde_json::to_string(ri).map_err(|e| CliError::BadConfigValue {
        key: format!("return_inputs[{year}]"),
        value: e.to_string(),
    })?;
    conn.execute(
        "INSERT INTO return_inputs(year,inputs_json) VALUES(?1,?2) \
         ON CONFLICT(year) DO UPDATE SET inputs_json=excluded.inputs_json",
        rusqlite::params![year, j],
    )?;
    Ok(())
}

/// Whether a `ReturnInputs` exists for `year` (used by the `tax-profile set` guard — SPEC §4.12/D-4).
/// A `SELECT 1` existence probe (does not deserialize the blob — review M5).
pub fn exists(conn: &Connection, year: i32) -> Result<bool, CliError> {
    init_table(conn)?;
    let found: Option<i64> = conn
        .query_row("SELECT 1 FROM return_inputs WHERE year=?1", [year], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(found.is_some())
}

/// Delete the stored `ReturnInputs` for `year` (used by `income clear`). Returns `true` if a row existed.
pub fn delete(conn: &Connection, year: i32) -> Result<bool, CliError> {
    init_table(conn)?;
    let n = conn.execute("DELETE FROM return_inputs WHERE year=?1", [year])?;
    Ok(n > 0)
}

/// Return all stored inputs, sorted by year ascending.
pub fn all(conn: &Connection) -> Result<BTreeMap<i32, ReturnInputs>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare("SELECT year, inputs_json FROM return_inputs ORDER BY year")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (y, j) = row?;
        out.insert(
            y,
            serde_json::from_str(&j).map_err(|e| CliError::BadConfigValue {
                key: format!("return_inputs[{y}]"),
                value: e.to_string(),
            })?,
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::return_inputs::{Owner, W2};
    use btctax_core::FilingStatus;
    use rust_decimal_macros::dec;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        init_table(&c).unwrap();
        c
    }

    fn inputs() -> ReturnInputs {
        ReturnInputs {
            filing_status: FilingStatus::Mfj,
            w2s: vec![W2 {
                owner: Owner::Taxpayer,
                employer: "ACME".into(),
                box1_wages: dec!(82000),
                box2_fed_withheld: dec!(9100),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn set_then_get_round_trips() {
        let c = mem();
        set(&c, 2024, &inputs()).unwrap();
        assert_eq!(get(&c, 2024).unwrap().unwrap(), inputs());
        assert_eq!(get(&c, 2025).unwrap(), None);
        assert!(exists(&c, 2024).unwrap());
        assert!(!exists(&c, 2025).unwrap());
    }

    #[test]
    fn get_on_tableless_vault_is_ok_none() {
        let c = Connection::open_in_memory().unwrap(); // no init_table
        assert_eq!(get(&c, 2024).unwrap(), None);
    }

    #[test]
    fn bad_json_is_a_typed_error_not_a_panic() {
        let c = mem();
        c.execute(
            "INSERT INTO return_inputs(year,inputs_json) VALUES(2024,'not json')",
            [],
        )
        .unwrap();
        assert!(matches!(
            get(&c, 2024).unwrap_err(),
            CliError::BadConfigValue { .. }
        ));
    }

    #[test]
    fn all_returns_sorted_by_year() {
        let c = mem();
        set(&c, 2025, &inputs()).unwrap();
        set(&c, 2024, &inputs()).unwrap();
        assert_eq!(
            all(&c).unwrap().keys().copied().collect::<Vec<_>>(),
            vec![2024, 2025]
        );
    }

    #[test]
    fn delete_removes_the_row() {
        let c = mem();
        set(&c, 2024, &inputs()).unwrap();
        assert!(exists(&c, 2024).unwrap());
        assert!(delete(&c, 2024).unwrap()); // existed
        assert!(!exists(&c, 2024).unwrap());
        assert!(!delete(&c, 2024).unwrap()); // idempotent — nothing to remove
    }
}
