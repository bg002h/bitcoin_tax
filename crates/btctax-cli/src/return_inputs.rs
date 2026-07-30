//! Per-year `return_inputs(year, inputs_json)` side-table — the full-return v1 input surface (line
//! items + PII + payments). A projection input, **not** ledger state (NFR6). Mirrors `tax_profile.rs`
//! exactly (idempotent DDL, robust-to-older-vaults guard, typed error on bad JSON) — one JSON-encoded
//! [`ReturnInputs`] per tax year, stored inside the encrypted vault.
use crate::CliError;
use btctax_core::tax::return_inputs::ReturnInputs;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

/// The current row schema.
///
/// - **0** — pre-D-8. `can_be_claimed_as_dependent_*` were bare `bool`s (unanswered indistinguishable
///   from "No").
/// - **1** — those flags became tri-state `Option<bool>`.
/// - **2** — P9: `Person.blind` and `ScheduleAInputs.salt_use_sales_tax` became tri-state; `hsa_present`
///   was renamed `hsa_activity` (a *different* question); `dual_status_alien` and the mixed-use-mortgage
///   box were added.
///
/// ★ **P9 §2.6 — there is no migration.** The owner confirmed no real tax data has ever been entered, so a
/// row at any version other than the current one **REFUSES** (`row_to_inputs`) rather than being read or
/// per-key unlaundered. A version check cannot forget a key; a hand-written unlaunder list can, and did
/// (Fable r4 I-4). Retire this the moment real data exists — the first real return needs true migrations,
/// and prior-year carryforwards are exactly what a filer cannot reconstruct (FOLLOWUPS, release gate).
pub const SCHEMA_VERSION: i64 = 2;

/// Create the `return_inputs` side-table if it does not exist, and bring an OLDER vault's table up to the
/// current schema. Idempotent — it runs on every `get`/`set`, so it must be safe to call repeatedly.
pub fn init_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS return_inputs \
         (year INTEGER PRIMARY KEY, inputs_json TEXT NOT NULL, schema_version INTEGER NOT NULL DEFAULT 0);",
    )?;
    // A vault created BEFORE this column existed still has the 2-column table — `CREATE TABLE IF NOT
    // EXISTS` is a no-op on it and adds nothing. SQLite has no `ADD COLUMN IF NOT EXISTS`, and this runs
    // on every command, so: attempt the ALTER and tolerate EXACTLY the already-applied error. Any other
    // error is real and propagates.
    if let Err(e) = conn.execute_batch(
        "ALTER TABLE return_inputs ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 0;",
    ) {
        let msg = e.to_string();
        if !msg.contains("duplicate column name") {
            return Err(e.into());
        }
    }
    Ok(())
}

/// ★ The ONE read boundary. Every path that turns a stored blob into a [`ReturnInputs`] goes through
/// here — `get` AND `all` — so the version gate cannot be applied on one path and forgotten on the other.
///
/// **P9 §2.6 — refuse-and-reimport, not migrate.** A row whose `version` is anything other than the
/// current [`SCHEMA_VERSION`] REFUSES (`StaleReturnInputs`). This is fail-closed in both directions: an
/// OLDER row would deserialize its now-`Option` fields' stored `false` as `Some(false)` — a never-asked
/// default ratified as the filer's answer, the D-8 laundering — and a NEWER row would be half-read. The
/// remedy (named in the error) is `income clear` → `income import` → `report --write-carryover`; `clear`
/// discards the row's computed carryover, so the rebuild step is not optional. There is no per-key
/// migration to forget a key (Fable r4 I-4), because there is no real data to migrate yet.
fn row_to_inputs(year: i32, json: &str, version: i64) -> Result<ReturnInputs, CliError> {
    if version != SCHEMA_VERSION {
        return Err(CliError::StaleReturnInputs {
            year,
            found: version,
            expected: SCHEMA_VERSION,
        });
    }
    let mut ri: ReturnInputs =
        serde_json::from_str(json).map_err(|e| CliError::BadConfigValue {
            key: format!("return_inputs[{year}]"),
            value: format!("invalid JSON: {e}"),
        })?;
    // ★★ §G-15 — THE ROW KEY IS AUTHORITATIVE. Stamping here (the ONE read boundary) means an
    // in-memory `tax_year` can never disagree with the row it came from, and a row written before the
    // field existed loads with the right year instead of `0`. Every year-scoped liveness predicate
    // therefore reads a value that is true by construction rather than by discipline.
    ri.tax_year = year;
    Ok(ri)
}

/// Return the stored [`ReturnInputs`] for `year`, or `None` if none has been set.
/// Robust to older vaults (ensures the table first); typed error on malformed JSON.
pub fn get(conn: &Connection, year: i32) -> Result<Option<ReturnInputs>, CliError> {
    init_table(conn)?;
    let json: Option<(String, i64)> = conn
        .query_row(
            "SELECT inputs_json, schema_version FROM return_inputs WHERE year=?1",
            [year],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match json {
        None => Ok(None),
        Some((j, v)) => Ok(Some(row_to_inputs(year, &j, v)?)),
    }
}

/// Persist `ri` as the [`ReturnInputs`] for `year` (upsert — replaces any prior value).
pub fn set(conn: &Connection, year: i32, ri: &ReturnInputs) -> Result<(), CliError> {
    init_table(conn)?;
    // ★★ §G-15 — refuse a DISAGREEMENT rather than silently preferring one side. `0` is "not
    // stated" and is stamped from the key; any other mismatch means the caller believes this is a
    // different year's return, and writing it under `year` would file one year's answers as
    // another's.
    if ri.tax_year != 0 && ri.tax_year != year {
        return Err(CliError::BadConfigValue {
            key: format!("return_inputs[{year}].tax_year"),
            value: format!(
                "these inputs are for tax year {} but are being stored under {year} — refusing, \
                 because storing one year's answers as another's would misattribute the filer's \
                 testimony",
                ri.tax_year
            ),
        });
    }
    let stamped = ReturnInputs {
        tax_year: year,
        ..ri.clone()
    };
    let j = serde_json::to_string(&stamped).map_err(|e| CliError::BadConfigValue {
        key: format!("return_inputs[{year}]"),
        value: e.to_string(),
    })?;
    // ★ The DO-UPDATE branch MUST stamp the version too. The shipped upsert named `inputs_json` alone —
    // so writing an answer onto a version-0 row would have left it at version 0, the read-time fixup would
    // RE-FIRE on the very next `get`, and the filer's answered `false` would be laundered straight back to
    // `None`. The bug would have reconstituted itself out of its own fix.
    conn.execute(
        "INSERT INTO return_inputs(year,inputs_json,schema_version) VALUES(?1,?2,?3) \
         ON CONFLICT(year) DO UPDATE SET inputs_json=excluded.inputs_json, \
                                         schema_version=excluded.schema_version",
        rusqlite::params![year, j, SCHEMA_VERSION],
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

/// The years that have stored inputs, ascending — WITHOUT deserializing any blob, so a single corrupt
/// row cannot break enumeration (review N3: one bad blob must not brick the read-only viewer).
pub fn years(conn: &Connection) -> Result<Vec<i32>, CliError> {
    init_table(conn)?;
    let mut stmt = conn.prepare("SELECT year FROM return_inputs ORDER BY year")?;
    let rows = stmt.query_map([], |r| r.get::<_, i32>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Return all stored inputs, sorted by year ascending.
pub fn all(conn: &Connection) -> Result<BTreeMap<i32, ReturnInputs>, CliError> {
    init_table(conn)?;
    let mut stmt =
        conn.prepare("SELECT year, inputs_json, schema_version FROM return_inputs ORDER BY year")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i32>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (y, j, v) = row?;
        // Same read boundary as `get` — NOT a second `from_str` (that is how the two paths drift apart).
        out.insert(y, row_to_inputs(y, &j, v)?);
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
            // ★ §G-15 — a real return always HAS a year, so the shared fixture states one. The two
            // guard tests below override it deliberately (`0` = not stated, and a disagreeing year).
            tax_year: 2024,
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

    /// ★★★ **§G-15 — the row key is authoritative on READ.**
    ///
    /// A row written before `tax_year` existed deserializes to `0`. Stamping at the one read boundary
    /// means every year-scoped liveness predicate sees a true year by construction, not by whoever
    /// remembered to set it. Without this, `HasIncomeExclusion`'s year gate would read `0` on every
    /// legacy row and silently go not-live for a return that should be asked.
    #[test]
    fn the_row_key_stamps_the_tax_year_on_read() {
        let c = mem();
        // Simulate a row written before the field existed: JSON with no `tax_year` at all.
        let mut ri = inputs();
        ri.tax_year = 0;
        let j = serde_json::to_string(&ri).unwrap();
        let j = j.replace("\"tax_year\":0,", "");
        assert!(
            !j.contains("tax_year"),
            "the fixture must genuinely lack the field"
        );
        c.execute(
            "INSERT INTO return_inputs(year,inputs_json,schema_version) VALUES(?1,?2,?3)",
            rusqlite::params![2024, j, SCHEMA_VERSION],
        )
        .unwrap();

        let got = get(&c, 2024).unwrap().expect("row exists");
        assert_eq!(
            got.tax_year, 2024,
            "the read boundary must stamp the year from the ROW KEY — a legacy row otherwise loads \
             as year 0 and every year-scoped question silently mis-scopes"
        );
    }

    /// ★★★ **§G-15 — a year DISAGREEMENT is refused, not silently resolved.**
    ///
    /// Storing one year's answers under another year would misattribute the filer's testimony: the
    /// same `Some(false)` means "no foreign accounts in 2024" or "…in 2025", and they are different
    /// statements. `0` is "not stated" and is stamped; anything else that disagrees refuses.
    #[test]
    fn storing_one_years_inputs_under_another_year_refuses() {
        let c = mem();
        let mut ri = inputs();
        ri.tax_year = 2025;

        let err = set(&c, 2024, &ri).expect_err("a 2025 return stored under 2024 must refuse");
        let msg = err.to_string();
        assert!(
            msg.contains("2025") && msg.contains("2024"),
            "the refusal must name BOTH years so the caller can see which is wrong: {msg}"
        );

        // The neutral case still works, and the stored JSON is stamped self-describing.
        let mut unset = inputs();
        unset.tax_year = 0;
        set(&c, 2024, &unset).expect("an unstated year is stamped from the key");
        assert_eq!(get(&c, 2024).unwrap().unwrap().tax_year, 2024);

        // And an agreeing year is accepted.
        let mut agree = inputs();
        agree.tax_year = 2024;
        set(&c, 2024, &agree).expect("an agreeing year stores");
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
        // At the CURRENT schema version (so the stale-row gate passes and we reach the JSON parse): a
        // malformed blob must be a typed error, not a panic. (A row omitting `schema_version` defaults to
        // 0 and would refuse as stale — a different, correct path tested in `p9_stale_row_refuses`.)
        c.execute(
            "INSERT INTO return_inputs(year,inputs_json,schema_version) VALUES(2024,'not json',?1)",
            [SCHEMA_VERSION],
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
        // ★ §G-15 — each year's inputs must STATE that year; storing one year's answers under
        // another now refuses, which is the point. `for_year` makes the intent explicit.
        let for_year = |y| ReturnInputs {
            tax_year: y,
            ..inputs()
        };
        set(&c, 2025, &for_year(2025)).unwrap();
        set(&c, 2024, &for_year(2024)).unwrap();
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

#[cfg(test)]
mod p9_stale_row_refuses {
    //! ★ P9 §2.6 — there is NO migration. A stored row whose `schema_version` is not the current one
    //! REFUSES (`StaleReturnInputs`) rather than being silently read or per-key unlaundered. The owner
    //! confirmed no real data has ever been entered, so refuse-and-reimport is lawful — and a version check
    //! cannot forget a key, unlike the hand-written unlaunder list this replaces (whose `blind ×2`
    //! mutation-check went vacuous — Fable r4 I-4). This module replaces the old `p8a_migration_tests`,
    //! which tested the now-deleted v0→v1 unlaunder.
    use super::*;
    use rusqlite::Connection;

    /// A vault holding a row at an OLD schema version, in the current 3-column table.
    fn vault_with_row_at_version(year: i32, version: i64) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_table(&conn).unwrap();
        // A well-formed blob (valid JSON) — the point is the VERSION, not a parse failure.
        let blob = serde_json::to_string(&ReturnInputs::default()).unwrap();
        conn.execute(
            "INSERT INTO return_inputs(year,inputs_json,schema_version) VALUES(?1,?2,?3)",
            rusqlite::params![year, blob, version],
        )
        .unwrap();
        conn
    }

    /// ★ THE POINT. A pre-P9 row (v0, the pre-D-8 schema) must REFUSE — never be read. Its `blind`/`salt`
    /// bools would deserialize to `Some(false)` (a never-asked default ratified as an answer), which is the
    /// D-8 laundering; refusing is the fail-closed reading.
    #[test]
    fn a_version_0_row_refuses_stale() {
        let conn = vault_with_row_at_version(2024, 0);
        assert!(
            matches!(get(&conn, 2024), Err(CliError::StaleReturnInputs { year: 2024, found: 0, expected }) if expected == SCHEMA_VERSION),
            "a v0 row must refuse as stale, naming the version"
        );
    }

    /// A v1 row (the post-D-8, pre-P9 schema) is equally stale — the `blind`/`salt` type flips landed in P9.
    #[test]
    fn a_version_1_row_refuses_stale() {
        let conn = vault_with_row_at_version(2024, 1);
        assert!(
            matches!(
                get(&conn, 2024),
                Err(CliError::StaleReturnInputs { found: 1, .. })
            ),
            "a v1 row must refuse as stale"
        );
    }

    /// ★ `all()` is the module's OTHER deserializer — it must refuse identically, or a reader (the TUI's
    /// per-year resolution, `income show --all`) sees a laundered row `get` would have refused.
    #[test]
    fn all_refuses_a_stale_row_identically_to_get() {
        let conn = vault_with_row_at_version(2024, 1);
        assert!(
            matches!(
                all(&conn),
                Err(CliError::StaleReturnInputs { found: 1, .. })
            ),
            "`all()` must apply the same version gate as `get()`"
        );
    }

    /// ★ FORWARD guard (r3 Nit-2): a row written by a NEWER build (version > current) is also stale — the
    /// same `!=` covers it. P9 creates the first-ever version skew, and a half-read future row is exactly
    /// the class this spec closes.
    #[test]
    fn a_future_version_row_refuses_too() {
        let conn = vault_with_row_at_version(2024, SCHEMA_VERSION + 1);
        assert!(
            matches!(get(&conn, 2024), Err(CliError::StaleReturnInputs { .. })),
            "a future-version row must refuse, not be half-read"
        );
    }

    /// A row at the CURRENT version reads normally — the gate is exact, not a blanket refusal.
    #[test]
    fn a_current_version_row_reads() {
        let conn = vault_with_row_at_version(2024, SCHEMA_VERSION);
        assert!(
            get(&conn, 2024).unwrap().is_some(),
            "a current-version row must read"
        );
    }

    /// The stale-row refusal's message names all THREE remedy commands, in order (§2.6 / r6 I-1): `clear`
    /// discards the computed carryover, so `import` alone is not a complete recovery. Mutation: drop the
    /// rebuild clause from the `#[error(...)]` string ⇒ this fails.
    #[test]
    fn the_stale_message_names_the_full_three_command_remedy() {
        let msg = CliError::StaleReturnInputs {
            year: 2024,
            found: 1,
            expected: 2,
        }
        .to_string();
        assert!(msg.contains("income clear 2024"), "names clear");
        assert!(msg.contains("income import"), "names import");
        assert!(
            msg.contains("--write-carryover"),
            "names the rebuild — disclosure is not restoration (r6 I-1)"
        );
        assert!(
            msg.contains("2023"),
            "the rebuild targets the PRIOR year (year-1)"
        );
    }
}
