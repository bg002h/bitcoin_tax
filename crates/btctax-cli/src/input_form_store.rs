//! Per-year `return_inputs_draft(year, inputs_json, schema_version, parked)` side-table — the input form's
//! crash-recovery scratch, INVISIBLE to `resolve.rs`. `parked = 1` marks a parked committed return (C-1).
//!
//! Plan-2 task 1 built the table + low-level row I/O; task 2 added `save_draft` (the autosave primitive);
//! task 3 adds `load` (the read path: draft ⇒ committed ⇒ fresh, with the §6.3 stale split). Every low-level
//! item now has a non-test caller — `set_draft_row`/`parked_flag` via `save_draft`, and `DraftRow`/
//! `get_draft_row` via `load` — so none carries a `#[allow(dead_code)]` any longer.
use crate::return_inputs::SCHEMA_VERSION;
use crate::{CliError, Session};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::return_refuse::{screen_inputs, Refusal};
use btctax_core::tax::tables::{FullReturnParams, TaxTable};
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

/// The working return for a year, resolved through the §6.1 precedence: a draft shadows the committed row.
///
/// - `Draft { ri, parked }` — a version-current draft exists (the crash-recovery scratch wins over committed).
/// - `Committed(ri)` — no draft; the committed `return_inputs` row is the working return.
/// - `Fresh` — neither exists; start a blank return.
pub enum Loaded {
    Draft { ri: ReturnInputs, parked: bool },
    Committed(ReturnInputs),
    Fresh,
}

/// Resolve the working return for `year` (§6.1 precedence + §6.3 stale split).
///
/// A draft row takes precedence over the committed row. If the draft is at a schema version this build does
/// not read (`d.version != SCHEMA_VERSION`), the split is by `parked`:
///
/// - **WIP** (`parked = 0`) → the draft is regenerable, so **DISCARD** it: delete the stale row (an
///   in-memory delete the caller's next `save_draft` persists — this is a read path, no `sess.save()` here)
///   and fall through to committed/Fresh, noting the discard on stderr.
/// - **parked** (`parked = 1`) → it may hold carryover that exists ONLY in the draft (C-1), so **REFUSE**
///   with [`CliError::StaleParkedDraft`] (fail closed) rather than destroy irreplaceable data.
///
/// A version-current draft yields `Draft { ri, parked }`. With no draft, the committed row (if any) is
/// `Committed`, else `Fresh`.
pub fn load(conn: &Connection, year: i32) -> Result<Loaded, CliError> {
    if let Some(d) = get_draft_row(conn, year)? {
        if d.version != SCHEMA_VERSION {
            if d.parked {
                return Err(CliError::StaleParkedDraft {
                    year,
                    found: d.version,
                    expected: SCHEMA_VERSION,
                });
            }
            // ★ §6.3: a stale WIP draft is regenerable — discard-with-note, fall through.
            eprintln!(
                "note: discarded a stale draft for {year} (schema v{} vs v{SCHEMA_VERSION}).",
                d.version
            );
            delete_draft(conn, year)?;
        } else {
            return Ok(Loaded::Draft {
                ri: d.ri,
                parked: d.parked,
            });
        }
    }
    match crate::return_inputs::get(conn, year)? {
        Some(ri) => Ok(Loaded::Committed(ri)),
        None => Ok(Loaded::Fresh),
    }
}

/// The outcome of a [`commit`] attempt.
///
/// - `Committed` — the return screened CLEAN; the committed `return_inputs` row was written and the draft
///   deleted.
/// - `Refused(refusal)` — [`screen_inputs`] tripped a fail-closed guard; NOTHING was written (the year is
///   never poisoned at `resolve`, and the draft is left intact for the user to fix).
/// - `NoTables` — the year has no full-return tables/params (v1 bundles TY2024 only — I-11); NOTHING was
///   written.
pub enum CommitOutcome {
    Committed,
    Refused(Refusal),
    NoTables,
}

/// Screen `ri`, and ONLY if it passes write the committed row and delete the draft (SPEC §5.7).
///
/// The write is all-or-nothing:
///
/// - No `table`/`params` for the year → [`CommitOutcome::NoTables`] (the TY2024-only gate, I-11) — writes
///   nothing.
/// - [`screen_inputs`] returns `Some(refusal)` → [`CommitOutcome::Refused`] — writes nothing, so a refused
///   commit never poisons the year at `resolve` and the draft remains for the user to fix.
/// - Clean → snapshot the in-memory DB, `return_inputs::set` the committed row, `delete_draft`, then
///   `sess.save()` to reach disk. If the save fails, RESTORE the snapshot so there is never an
///   in-memory/disk split (the committed row + draft-deletion are rolled back together, I-7).
///
/// Takes `&mut Session` for `save()`; reads through the SAME session's `conn()` — never opens a second
/// Session (N-1).
pub fn commit(
    sess: &mut Session,
    year: i32,
    ri: &ReturnInputs,
    table: Option<&TaxTable>,
    params: Option<&FullReturnParams>,
) -> Result<CommitOutcome, CliError> {
    let (Some(table), Some(params)) = (table, params) else {
        return Ok(CommitOutcome::NoTables); // I-11: no tables for this year → write nothing
    };
    if let Some(refusal) = screen_inputs(ri, table, params) {
        return Ok(CommitOutcome::Refused(refusal)); // fail-closed: writes nothing
    }
    let snap = sess.snapshot()?;
    crate::return_inputs::set(sess.conn(), year, ri)?;
    delete_draft(sess.conn(), year)?;
    if let Err(e) = sess.save() {
        sess.restore(&snap)?; // atomic: never leave an in-memory/disk split
        return Err(e);
    }
    Ok(CommitOutcome::Committed)
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
    fn load_precedence_draft_then_committed_then_fresh() {
        let conn = Connection::open_in_memory().unwrap();
        crate::return_inputs::init_table(&conn).unwrap();
        init_draft_table(&conn).unwrap();
        // Fresh
        assert!(matches!(load(&conn, 2024).unwrap(), Loaded::Fresh));
        // Committed only
        let cri = ReturnInputs { filing_status: FilingStatus::HoH, ..Default::default() };
        crate::return_inputs::set(&conn, 2024, &cri).unwrap();
        assert!(matches!(load(&conn, 2024).unwrap(), Loaded::Committed(r) if r.filing_status == FilingStatus::HoH));
        // Draft shadows committed
        let dri = ReturnInputs { filing_status: FilingStatus::Mfj, ..Default::default() };
        set_draft_row(&conn, 2024, &dri, false).unwrap();
        assert!(matches!(load(&conn, 2024).unwrap(), Loaded::Draft { ri, parked: false } if ri.filing_status == FilingStatus::Mfj));
    }

    #[test]
    fn load_discards_stale_wip_but_refuses_stale_parked() {
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let ri = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
        let j = serde_json::to_string(&ri).unwrap();
        // stale WIP (parked=0) at an old version → discarded, falls through to Fresh, row is GONE
        conn.execute("INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(2024,?1,0,0)", [&j]).unwrap();
        assert!(matches!(load(&conn, 2024).unwrap(), Loaded::Fresh));
        assert!(!draft_exists(&conn, 2024).unwrap(), "stale WIP is discarded");
        // stale PARKED (parked=1) → REFUSE, row PRESERVED
        conn.execute("INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(2025,?1,0,1)", [&j]).unwrap();
        assert!(matches!(load(&conn, 2025), Err(CliError::StaleParkedDraft { year: 2025, found: 0, .. })));
        assert!(draft_exists(&conn, 2025).unwrap(), "stale parked is preserved, not discarded");
    }

    /// The canonical screen-clean return: a minimal Single filer that is not a dependent and has answered
    /// every always-live declaration (mirrors `resolve.rs`'s fixture / the `answer.rs`
    /// `every_live_question_can_actually_be_answered_and_clears_the_screen` test). No income is needed —
    /// `screen_inputs` only checks the input-screenable rows, so this passes the screen cleanly.
    fn clean_screened_ri() -> ReturnInputs {
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            header: btctax_core::tax::testonly::not_a_dependent(),
            ..Default::default()
        };
        btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
        ri
    }

    #[test]
    fn commit_non2024_is_notables_and_writes_nothing() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        set_draft_row(sess.conn(), 2099, &ri, false).unwrap();
        // no full-return params for 2099 → NoTables
        let out = commit(&mut sess, 2099, &ri, None, None).unwrap();
        assert!(matches!(out, CommitOutcome::NoTables));
        assert!(
            !crate::return_inputs::exists(sess.conn(), 2099).unwrap(),
            "NoTables writes no committed row"
        );
        assert!(
            draft_exists(sess.conn(), 2099).unwrap(),
            "NoTables leaves the draft"
        );
    }

    #[test]
    fn commit_clean_sets_row_and_deletes_draft_refused_writes_nothing() {
        use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
        use btctax_core::tax::tables::FullReturnTables;
        use btctax_core::TaxTables;
        let (_dir, path, pp) = tmp_vault();
        let tables = BundledTaxTables::load(); // ★ I-B: load() returns Self, NOT Result — no `?`, no `.unwrap()`
        let fr = BundledFullReturnTables::load();
        let (t, p) = (
            tables.table_for(2024).unwrap(),
            fr.full_return_for(2024).unwrap(),
        ); // these DO return Option
        let mut sess = Session::open(&path, &pp).unwrap();
        // A screen-clean minimal return (all live declarations answered, no income).
        let clean = clean_screened_ri();
        set_draft_row(sess.conn(), 2024, &clean, false).unwrap();
        assert!(matches!(
            commit(&mut sess, 2024, &clean, Some(t), Some(p)).unwrap(),
            CommitOutcome::Committed
        ));
        assert!(
            crate::return_inputs::exists(sess.conn(), 2024).unwrap(),
            "clean commit writes the row"
        );
        assert!(
            !draft_exists(sess.conn(), 2024).unwrap(),
            "clean commit deletes the draft"
        );
        // A refused return (unanswered live declarations) writes nothing.
        let refused = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        }; // ~5 live None decls
        set_draft_row(sess.conn(), 2024, &refused, false).unwrap();
        assert!(matches!(
            commit(&mut sess, 2024, &refused, Some(t), Some(p)).unwrap(),
            CommitOutcome::Refused(_)
        ));
        // the committed 2024 row is still the earlier `clean` one; the refused draft remains
        assert!(
            crate::return_inputs::exists(sess.conn(), 2024).unwrap(),
            "a refused commit does not delete the earlier committed row"
        );
        assert!(
            draft_exists(sess.conn(), 2024).unwrap(),
            "a refused commit leaves the draft"
        );
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
