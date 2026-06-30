//! Per-year `tax_profile(year, profile_json)` side-table — a projection input, **not** ledger
//! state (NFR6). Stores one JSON-encoded `TaxProfile` per tax year. Modeled on `config.rs`'s
//! `cli_config` side-table discipline (idempotent DDL, robust-to-older-vaults guard, typed error
//! on bad JSON). Called by `Session::from_fresh_vault` and as a defensive guard at the top of
//! every read/write (same pattern as `config::init_config_table` / `config::read_config`).
use crate::CliError;
use btctax_core::TaxProfile;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

/// Create the `tax_profile` side-table if it does not exist.
/// `CREATE TABLE IF NOT EXISTS` makes this idempotent — safe to call on any vault (old, new, or
/// restored from snapshot). Called by `Session::from_fresh_vault`; also called at the top of
/// every `get`/`set`/`all` as a defensive ensure-table-then-read guard (same guard as
/// `read_config`, `config.rs:77`).
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tax_profile \
         (year INTEGER PRIMARY KEY, profile_json TEXT NOT NULL);",
    )?;
    Ok(())
}

/// Return the stored `TaxProfile` for `year`, or `None` if none has been set.
///
/// Robust to older vaults (calls `init_table` first so "no such table" is never returned).
/// Returns `CliError::BadConfigValue` if the stored JSON is malformed.
pub fn get(conn: &Connection, year: i32) -> Result<Option<TaxProfile>, CliError> {
    // Defensive ensure-table-then-read (same guard as `read_config`, config.rs:77).
    init_table(conn)?;
    let json: Option<String> = conn
        .query_row(
            "SELECT profile_json FROM tax_profile WHERE year=?1",
            [year],
            |r| r.get(0),
        )
        .optional()?;
    match json {
        None => Ok(None),
        Some(j) => Ok(Some(serde_json::from_str(&j).map_err(|e| {
            CliError::BadConfigValue {
                key: format!("tax_profile[{year}]"),
                value: format!("invalid JSON: {e}"),
            }
        })?)),
    }
}

/// Persist `p` as the `TaxProfile` for `year` (upsert — replaces any prior value).
pub fn set(conn: &Connection, year: i32, p: &TaxProfile) -> Result<(), CliError> {
    init_table(conn)?;
    let j = serde_json::to_string(p).map_err(|e| CliError::BadConfigValue {
        key: format!("tax_profile[{year}]"),
        value: e.to_string(),
    })?;
    conn.execute(
        "INSERT INTO tax_profile(year,profile_json) VALUES(?1,?2) \
         ON CONFLICT(year) DO UPDATE SET profile_json=excluded.profile_json",
        rusqlite::params![year, j],
    )?;
    Ok(())
}

/// Return all stored profiles, sorted by year ascending.
pub fn all(conn: &Connection) -> Result<BTreeMap<i32, TaxProfile>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare("SELECT year, profile_json FROM tax_profile ORDER BY year")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (y, j) = row?;
        out.insert(
            y,
            serde_json::from_str(&j).map_err(|e| CliError::BadConfigValue {
                key: format!("tax_profile[{y}]"),
                value: e.to_string(),
            })?,
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{Carryforward, FilingStatus, TaxProfile};
    use rust_decimal_macros::dec;

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_table(&c).unwrap();
        c
    }

    fn prof() -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: dec!(120000),
            magi_excluding_crypto: dec!(130000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(0),
                long: dec!(0),
            },
        }
    }

    #[test]
    fn set_then_get_round_trips() {
        let c = mem();
        set(&c, 2025, &prof()).unwrap();
        assert_eq!(get(&c, 2025).unwrap().unwrap(), prof());
        assert_eq!(get(&c, 2024).unwrap(), None);
    }

    #[test]
    fn get_on_tableless_vault_is_ok_none() {
        let c = rusqlite::Connection::open_in_memory().unwrap(); // no init_table
        assert_eq!(get(&c, 2025).unwrap(), None);
    }

    #[test]
    fn bad_json_is_a_typed_error_not_a_panic() {
        let c = mem();
        c.execute(
            "INSERT INTO tax_profile(year,profile_json) VALUES(2025,'not json')",
            [],
        )
        .unwrap();
        assert!(matches!(
            get(&c, 2025).unwrap_err(),
            CliError::BadConfigValue { .. }
        ));
    }

    #[test]
    fn all_returns_sorted_by_year() {
        let c = mem();
        set(&c, 2026, &prof()).unwrap();
        set(&c, 2025, &prof()).unwrap();
        assert_eq!(
            all(&c).unwrap().keys().copied().collect::<Vec<_>>(),
            vec![2025, 2026]
        );
    }
}
