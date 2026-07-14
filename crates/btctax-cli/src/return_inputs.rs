//! Per-year `return_inputs(year, inputs_json)` side-table — the full-return v1 input surface (line
//! items + PII + payments). A projection input, **not** ledger state (NFR6). Mirrors `tax_profile.rs`
//! exactly (idempotent DDL, robust-to-older-vaults guard, typed error on bad JSON) — one JSON-encoded
//! [`ReturnInputs`] per tax year, stored inside the encrypted vault.
use crate::CliError;
use btctax_core::tax::return_inputs::ReturnInputs;
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeMap;

/// The current row schema. Bump when a stored blob needs a read-time fixup, and add the arm to
/// [`row_to_inputs`].
///
/// - **0** — pre-D-8. `can_be_claimed_as_dependent_*` were bare `bool`s, so EVERY row carries `false`
///   for them whether or not the filer was ever asked. Unanswered is indistinguishable from "No".
/// - **1** — the flags are tri-state `Option<bool>`; a stored value means the filer ANSWERED.
pub const SCHEMA_VERSION: i64 = 1;

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
/// here — `get` AND `all` — so a migration cannot be applied on one path and forgotten on the other.
///
/// **Version 0 → 1 (D-8).** The dependent flags were bare `bool`s: serde wrote `false` for a filer who
/// was never asked, and `false` is also what an answered "No" looks like. The two are indistinguishable
/// in the blob, so we must pick the SAFE reading — and `false` is the DANGEROUS one (it grants the full
/// basic standard deduction, skips the §1(g) kiddie-tax refusal, and prints an unchecked box on a filed
/// 1040). So a version-0 `false` becomes `None` ⇒ the year refuses until the filer answers.
///
/// A version-0 `true` is PRESERVED: nothing ever defaulted to `true`, so it can only have been typed.
fn row_to_inputs(year: i32, json: &str, version: i64) -> Result<ReturnInputs, CliError> {
    let mut ri: ReturnInputs =
        serde_json::from_str(json).map_err(|e| CliError::BadConfigValue {
            key: format!("return_inputs[{year}]"),
            value: format!("invalid JSON: {e}"),
        })?;
    if version < 1 {
        let unlaunder = |v: &mut Option<bool>| {
            if *v == Some(false) {
                *v = None;
            }
        };
        unlaunder(&mut ri.header.can_be_claimed_as_dependent_taxpayer);
        unlaunder(&mut ri.header.can_be_claimed_as_dependent_spouse);
    }
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
    let j = serde_json::to_string(ri).map_err(|e| CliError::BadConfigValue {
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
    let mut stmt = conn
        .prepare("SELECT year, inputs_json, schema_version FROM return_inputs ORDER BY year")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i32>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
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

#[cfg(test)]
mod p8a_migration_tests {
    use super::*;
    use btctax_core::tax::return_inputs::ReturnInputs;
    use rusqlite::Connection;

    /// A PRE-P8a vault: the old 2-column schema, holding a blob written by the SHIPPED code.
    ///
    /// The blob is built by SERIALIZING a real `ReturnInputs` and rewriting the flag keys — because a bare
    /// `bool` is always serialized, so every v0 row carries `false` for BOTH flags whether or not the filer
    /// was ever asked. Hand-writing the JSON would drift from the struct (and did: my first version put the
    /// flag at the top level instead of under `header`, so every assertion below passed VACUOUSLY against a
    /// defaulted header).
    ///
    /// ★ It rewrites **both** flags, and asserts each rewrite lands. The first version rewrote only the
    /// taxpayer key — so the SPOUSE half of the migration was held by no test at all, and deleting it left
    /// the whole suite green (Fable P8a r1 I2). A fixture that silently covers half of what it claims to
    /// cover is worse than no fixture: it reports success for the part it never touched.
    fn v0_blob(taxpayer: bool, spouse: bool) -> String {
        let j = serde_json::to_string(&ReturnInputs::default()).unwrap();
        let mut out = j.clone();
        for (key, v) in [
            ("can_be_claimed_as_dependent_taxpayer", taxpayer),
            ("can_be_claimed_as_dependent_spouse", spouse),
        ] {
            let before = out.clone();
            out = out.replace(&format!("\"{key}\":null"), &format!("\"{key}\":{v}"));
            assert_ne!(
                out, before,
                "the v0 rewrite must actually hit `{key}` — else the test that reads it is vacuous"
            );
        }
        out
    }

    /// A v0 vault whose row answers both flags the way the shipped `bool` did: `false`, unasked.
    fn old_schema_vault(year: i32, taxpayer: bool, spouse: bool) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE return_inputs (year INTEGER PRIMARY KEY, inputs_json TEXT NOT NULL);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO return_inputs(year,inputs_json) VALUES(?1,?2)",
            rusqlite::params![year, v0_blob(taxpayer, spouse)],
        )
        .unwrap();
        conn
    }

    fn old_schema_vault_with_unanswered_row(year: i32) -> Connection {
        old_schema_vault(year, false, false)
    }

    /// ★ The whole point. A pre-P8a row's `false` is INDISTINGUISHABLE from "never asked" — so it must
    /// load as `None` (⇒ the year refuses UNANSWERED), NOT as `Some(false)` (⇒ an answered "No", which
    /// would LAUNDER the guess into an answer and make it permanent).
    #[test]
    fn a_version_0_rows_unanswered_false_loads_as_none() {
        let conn = old_schema_vault_with_unanswered_row(2024);
        let ri = get(&conn, 2024).unwrap().expect("the row is there");
        assert_eq!(
            ri.header.can_be_claimed_as_dependent_taxpayer, None,
            "a version-0 row's `false` was never answered — it must load as None, not Some(false)"
        );
    }

    /// A stored `true` can only have been TYPED — nothing defaults to true. Preserve it.
    #[test]
    fn a_version_0_rows_true_is_preserved() {
        let conn = old_schema_vault(2024, true, true);
        let h = get(&conn, 2024).unwrap().unwrap().header;
        assert_eq!(
            h.can_be_claimed_as_dependent_taxpayer,
            Some(true),
            "nothing defaults to true — a stored true was typed, and must survive"
        );
        assert_eq!(
            h.can_be_claimed_as_dependent_spouse,
            Some(true),
            "...and the same is true of the SPOUSE flag"
        );
    }

    /// ★ **Fable P8a r1 I2.** The SPOUSE half of the migration had no test: deleting
    /// `unlaunder(&mut ri.header.can_be_claimed_as_dependent_spouse)` left 1729/1729 passing, because the
    /// fixture only ever rewrote the taxpayer key. Without this, every v0 MFJ/MFS row's spouse `false` is
    /// ratified as an answered "No" — `DependentSpouseStatusUnanswered` never fires for exactly the
    /// population that has the bug, and the 1040 prints the spouse box unchecked, unaffirmed.
    #[test]
    fn a_version_0_rows_unanswered_spouse_false_also_loads_as_none() {
        let conn = old_schema_vault_with_unanswered_row(2024);
        assert_eq!(
            get(&conn, 2024)
                .unwrap()
                .unwrap()
                .header
                .can_be_claimed_as_dependent_spouse,
            None,
            "a v0 spouse `false` was never answered either — it must not be ratified as a No"
        );
    }

    /// ★ THE ANSWER MUST STICK. The shipped upsert is `DO UPDATE SET inputs_json=excluded.inputs_json` —
    /// it names ONE column. If `set` does not stamp `schema_version = 1` in the DO-UPDATE branch too, the
    /// row stays version 0, the fixup RE-FIRES on the very next read, and the user's answer is silently
    /// laundered back to `None`. The bug would reconstitute itself out of its own fix.
    #[test]
    fn answering_false_on_a_version_0_row_sticks() {
        let conn = old_schema_vault_with_unanswered_row(2024);
        let mut ri = get(&conn, 2024).unwrap().unwrap();
        assert_eq!(ri.header.can_be_claimed_as_dependent_taxpayer, None);

        // The user answers: "no, nobody can claim me."
        ri.header.can_be_claimed_as_dependent_taxpayer = Some(false);
        set(&conn, 2024, &ri).unwrap();

        assert_eq!(
            get(&conn, 2024)
                .unwrap()
                .unwrap()
                .header
                .can_be_claimed_as_dependent_taxpayer,
            Some(false),
            "the user ANSWERED false — it must not be re-laundered to None on the next read"
        );
    }

    /// `all()` is the module's OTHER deserializer. It must migrate identically, or a reader sees the
    /// laundered flag.
    #[test]
    fn all_migrates_identically_to_get() {
        let conn = old_schema_vault_with_unanswered_row(2024);
        assert_eq!(
            all(&conn).unwrap()[&2024]
                .header
                .can_be_claimed_as_dependent_taxpayer,
            None,
            "`all()` must apply the same migration as `get()`"
        );
    }

    /// The DDL must be idempotent on an OLD vault. SQLite has no `ADD COLUMN IF NOT EXISTS`, and
    /// `init_table` runs on every command — so a bare ALTER errors `duplicate column name` on the second.
    #[test]
    fn an_old_schema_vault_opens_twice() {
        let conn = old_schema_vault_with_unanswered_row(2024);
        init_table(&conn).expect("first open migrates");
        init_table(&conn).expect("second open must NOT error `duplicate column name`");
        get(&conn, 2024).expect("and the row still reads");
    }
}
