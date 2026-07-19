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
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        },
    );
    match row {
        Ok((json, version, parked)) => {
            // ★ I-A: CliError has NO From<serde_json::Error> — map explicitly like return_inputs.rs:66-69
            // (a bad blob is a typed error, not a `?`-panic). Do NOT use `?` on serde here.
            let ri: ReturnInputs =
                serde_json::from_str(&json).map_err(|e| CliError::BadConfigValue {
                    key: format!("return_inputs_draft[{year}]"),
                    value: format!("invalid JSON: {e}"),
                })?;
            Ok(Some(DraftRow {
                ri,
                version,
                parked: parked != 0,
            }))
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

pub(crate) fn delete_draft(conn: &Connection, year: i32) -> Result<bool, CliError> {
    init_draft_table(conn)?;
    Ok(conn.execute("DELETE FROM return_inputs_draft WHERE year=?1", [year])? > 0)
}

pub fn draft_exists(conn: &Connection, year: i32) -> Result<bool, CliError> {
    init_draft_table(conn)?;
    Ok(conn
        .query_row(
            "SELECT 1 FROM return_inputs_draft WHERE year=?1",
            [year],
            |_| Ok(()),
        )
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

/// The §6.3 fact that [`load`] discarded a stale work-in-progress draft (schema `found` — a version this
/// build does not read — vs `expected`). Returned ALONGSIDE `Loaded` so the caller can surface it: a store
/// read fn must not `eprintln!` the note itself, because its only future caller is plan 3's raw-mode,
/// alternate-screen TUI, where stderr is invisible/screen-corrupting (I-1). `pub` so plan 3 can render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleNote {
    pub year: i32,
    pub found: i64,
    pub expected: i64,
}

impl std::fmt::Display for StaleNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "discarded a stale draft for {} (schema v{}, expected v{})",
            self.year, self.found, self.expected
        )
    }
}

/// Resolve the working return for `year` (§6.1 precedence + §6.3 stale split).
///
/// A draft row takes precedence over the committed row. If the draft is at a schema version this build does
/// not read (`d.version != SCHEMA_VERSION`), the split is by `parked`:
///
/// - **WIP** (`parked = 0`) → the draft is regenerable, so **DISCARD** it: delete the stale row (an
///   in-memory delete the caller's next `save_draft` persists — this is a read path, no `sess.save()` here)
///   and fall through to committed/Fresh, RETURNING a [`StaleNote`] so the caller (not this store fn) can
///   surface the discard (I-1).
/// - **parked** (`parked = 1`) → it may hold carryover that exists ONLY in the draft (C-1), so **REFUSE**
///   with [`CliError::StaleParkedDraft`] (fail closed) rather than destroy irreplaceable data.
///
/// A version-current draft yields `Draft { ri, parked }`. With no draft, the committed row (if any) is
/// `Committed`, else `Fresh`. The second tuple element is `Some(StaleNote)` ONLY on the stale-WIP discard
/// path; every other path returns `None`.
pub fn load(conn: &Connection, year: i32) -> Result<(Loaded, Option<StaleNote>), CliError> {
    if let Some(d) = get_draft_row(conn, year)? {
        if d.version != SCHEMA_VERSION {
            if d.parked {
                return Err(CliError::StaleParkedDraft {
                    year,
                    found: d.version,
                    expected: SCHEMA_VERSION,
                });
            }
            // ★ §6.3 / I-1: a stale WIP draft is regenerable — discard it and RETURN the note (never
            // eprintln! from a store read fn: plan 3's raw-mode TUI would swallow/garble it), fall through.
            delete_draft(conn, year)?;
            let note = StaleNote {
                year,
                found: d.version,
                expected: SCHEMA_VERSION,
            };
            return Ok((committed_or_fresh(conn, year)?, Some(note)));
        } else {
            return Ok((
                Loaded::Draft {
                    ri: d.ri,
                    parked: d.parked,
                },
                None,
            ));
        }
    }
    Ok((committed_or_fresh(conn, year)?, None))
}

/// The no-draft tail of [`load`]: the committed `return_inputs` row (if any) else `Fresh`.
fn committed_or_fresh(conn: &Connection, year: i32) -> Result<Loaded, CliError> {
    match crate::return_inputs::get(conn, year)? {
        Some(ri) => Ok(Loaded::Committed(ri)),
        None => Ok(Loaded::Fresh),
    }
}

/// §6.2 draft-coherence: reconcile that year's input-form draft with an authoritative committed-row write.
///
/// The four writers of the committed `return_inputs` row (`income import`, `income answer`, carryover
/// write-back, `income clear`) are ignorant of the crash-recovery draft. A stale draft would then silently
/// shadow the freshly-written committed row at the next `load` (§6.1 precedence), and a PARKED draft — the
/// sole copy of a screened return (C-1) — could be clobbered out of existence. This helper closes both:
///
/// - **no draft** → `Ok(())` (a no-op; the writer proceeds unchanged).
/// - **WIP** (`parked = 0`) → the draft is regenerable crash-scratch, so the write SUPERSEDES it:
///   `delete_draft` (noting on stderr only if it held a non-trivial return, i.e. `!= default`). The delete
///   is in-memory on `conn`; the writer's own `s.save()` persists it together with the committed write.
/// - **parked** (`parked = 1`) → REFUSE with [`CliError::ParkedDraftBlocksWrite`] (fail closed) BEFORE any
///   committed-row mutation — never silently destroy irreplaceable data.
///
/// ★ M-1: callers invoke this RIGHT AFTER `Session::open`, before any committed-row read or write — else
/// the two writers that early-return on an absent committed row (`answer`, write-back) would exit with a
/// generic "no inputs" message before the parked-refuse is ever reached (a parked year has no committed
/// row). Takes `&Connection` (read + a conditional in-memory delete); a parked `Err` propagates via `?`.
pub fn coherence_clear_or_refuse(conn: &Connection, year: i32) -> Result<(), CliError> {
    match parked_flag(conn, year)? {
        None => Ok(()),
        Some(true) => Err(CliError::ParkedDraftBlocksWrite { year }), // ★ C-1: never clobber the sole copy
        Some(false) => {
            if let Some(d) = get_draft_row(conn, year)? {
                if d.ri != ReturnInputs::default() {
                    eprintln!(
                        "note: superseding a work-in-progress draft for {year} with this write."
                    );
                }
            }
            delete_draft(conn, year)?;
            Ok(())
        }
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
    if table.year != year || params.year != year {
        // ★ I-11 is per-YEAR, not per-call: tables for a DIFFERENT year would `screen_inputs`-pass and
        // write a committed row for a table-less `year`, poisoning it at resolve. Write nothing.
        return Ok(CommitOutcome::NoTables);
    }
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

/// Park the committed return for `year` into its draft (the "switch to tax-profile" toggle — C-1).
///
/// Stashes the committed [`ReturnInputs`] row into the draft table with `parked = 1`, then deletes the
/// committed row, so the year resolves through the [`crate::tax_profile`] again at `resolve.rs` precedence
/// — the full return is not lost, it is preserved as the sole `parked` copy (D-6) and reinstated by a later
/// `use full return`. The `tax_profile` itself is never touched; it simply stops being shadowed.
///
/// Refuses (writing nothing) when:
///
/// - **no committed row** → [`CliError::Usage`] (there is nothing to park).
/// - **a WIP draft already occupies the slot** (`parked = 0`) → [`CliError::Usage`]. ★ M-4: this refuses
///   ANY work-in-progress draft, not only one that diverges from the committed row. The store cannot
///   cheaply tell "divergent" apart, the draft slot is one-per-year, and stashing would clobber the user's
///   in-progress edit — so refusing a same-valued WIP costs nothing and the conservatism is intentional.
///
/// The stash and the delete are ordered stash-FIRST and committed together by a single `sess.save()`: on
/// save failure `restore(&snap)` rolls BOTH back, so a failed park never loses the committed row's SSNs
/// (D-6 atomicity, I-7). The delete is the in-session [`crate::return_inputs::delete`] on THIS session's
/// `conn()` — never the `income clear` command, which would open a second `Session` and deadlock against
/// the held lock (N-1). Takes `&mut Session` for `save()`.
pub fn park_to_profile(sess: &mut Session, year: i32) -> Result<(), CliError> {
    let Some(ri) = crate::return_inputs::get(sess.conn(), year)? else {
        return Err(CliError::Usage(format!(
            "no committed return to park for {year}"
        )));
    };
    if parked_flag(sess.conn(), year)? == Some(false) {
        // ★ clean-state gate (§9 / M-4): a WIP draft owns the one-per-year slot; parking would clobber it.
        return Err(CliError::Usage(format!(
            "year {year} has a work-in-progress draft; finish or discard it before switching to the tax-profile"
        )));
    }
    let snap = sess.snapshot()?;
    set_draft_row(sess.conn(), year, &ri, true)?; // ★ stash FIRST (parked=1)
    crate::return_inputs::delete(sess.conn(), year)?; // ★ N-1: in-session delete, NOT `income clear`
    if let Err(e) = sess.save() {
        sess.restore(&snap)?; // ★ atomic (D-6): a failed park never loses the committed row
        return Err(e);
    }
    Ok(())
}

/// The source the input form is CURRENTLY displaying/editing for `year` — the toggle's read side.
///
/// Mirrors `resolve.rs`'s precedence: a committed full return always wins over a `tax_profile`, which
/// wins over nothing at all. This is a display hint for plan 3 (the TUI), not a resolver decision.
pub enum ActiveSource {
    FullReturn,
    TaxProfile,
    Neither,
}

/// Which source is active for `year`, mirroring `resolve.rs` precedence (committed full return, then
/// `tax_profile`, then neither).
///
/// ★ M-5: this uses [`crate::return_inputs::exists`] (a cheap `SELECT 1`), so a schema-STALE committed
/// row still reports `FullReturn` — that is correct, not a bug to "fix" to `get`. The stale row IS the
/// active source (it is what shadows the `tax_profile` and what `resolve.rs` would refuse to compute
/// from); reporting `Neither`/`TaxProfile` here would be a false display, hiding the row that is actually
/// blocking the toggle.
pub fn active_source(conn: &Connection, year: i32) -> Result<ActiveSource, CliError> {
    if crate::return_inputs::exists(conn, year)? {
        return Ok(ActiveSource::FullReturn);
    }
    if crate::tax_profile::years(conn)?.contains(&year) {
        return Ok(ActiveSource::TaxProfile);
    }
    Ok(ActiveSource::Neither)
}

/// Whether a `tax_profile` exists for `year` — the TUI's commit-time shadow warning (§9 create-row
/// amendment): committing a full return for a year that also has a `tax_profile` leaves the profile
/// in place but no longer active, so the form warns before writing.
///
/// `tax_profile.rs` has no cheaper `exists` probe; `years(conn)?.contains(&year)` is the correct one
/// (confirmed against current source — it does not deserialize any profile blob).
pub fn shadows_profile(conn: &Connection, year: i32) -> Result<bool, CliError> {
    Ok(crate::tax_profile::years(conn)?.contains(&year))
}

/// Discard the parked draft for `year` — the 'X' path (§9A/M-2), the ONLY deleter of a `parked = 1` row.
///
/// The TUI owns the confirmation modal; this fn is the confirmed action. It REFUSES
/// ([`CliError::Usage`]) unless the year's draft is parked, so it can never be reached to delete a
/// work-in-progress draft (or a year with no draft at all) behind a "discard parked draft" affordance —
/// the parked check is the entire safety property of this function.
///
/// Snapshots before the delete and restores on a failed `sess.save()` (mirrors `park_to_profile`'s
/// atomicity), so a failed discard never leaves an in-memory/disk split.
pub fn discard_parked_draft(sess: &mut Session, year: i32) -> Result<(), CliError> {
    if parked_flag(sess.conn(), year)? != Some(true) {
        // never delete a WIP draft (or a non-existent one) behind this affordance
        return Err(CliError::Usage(format!(
            "year {year} has no parked draft to discard"
        )));
    }
    let snap = sess.snapshot()?;
    delete_draft(sess.conn(), year)?;
    if let Err(e) = sess.save() {
        sess.restore(&snap)?;
        return Err(e);
    }
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
    fn load_precedence_draft_then_committed_then_fresh() {
        let conn = Connection::open_in_memory().unwrap();
        crate::return_inputs::init_table(&conn).unwrap();
        init_draft_table(&conn).unwrap();
        // Fresh
        let (loaded, note) = load(&conn, 2024).unwrap();
        assert!(matches!(loaded, Loaded::Fresh));
        assert!(note.is_none(), "no stale discard on a fresh year");
        // Committed only
        let cri = ReturnInputs {
            filing_status: FilingStatus::HoH,
            ..Default::default()
        };
        crate::return_inputs::set(&conn, 2024, &cri).unwrap();
        let (loaded, note) = load(&conn, 2024).unwrap();
        assert!(matches!(loaded, Loaded::Committed(r) if r.filing_status == FilingStatus::HoH));
        assert!(note.is_none(), "no stale discard on the committed path");
        // Draft shadows committed
        let dri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        set_draft_row(&conn, 2024, &dri, false).unwrap();
        let (loaded, note) = load(&conn, 2024).unwrap();
        assert!(
            matches!(loaded, Loaded::Draft { ri, parked: false } if ri.filing_status == FilingStatus::Mfj)
        );
        assert!(
            note.is_none(),
            "no stale discard on a version-current draft"
        );
    }

    #[test]
    fn load_discards_stale_wip_but_refuses_stale_parked() {
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        let j = serde_json::to_string(&ri).unwrap();
        // stale WIP (parked=0) at an old version → discarded, falls through to Fresh, row is GONE,
        // and the discard fact is RETURNED as a StaleNote (I-1: never eprintln!'d from this store fn).
        conn.execute("INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(2024,?1,0,0)", [&j]).unwrap();
        let (loaded, note) = load(&conn, 2024).unwrap();
        assert!(matches!(loaded, Loaded::Fresh));
        assert_eq!(
            note,
            Some(StaleNote {
                year: 2024,
                found: 0,
                expected: SCHEMA_VERSION
            }),
            "the stale-WIP discard returns the note (not an eprintln!)"
        );
        assert!(
            !draft_exists(&conn, 2024).unwrap(),
            "stale WIP is discarded"
        );
        // stale PARKED (parked=1) → REFUSE, row PRESERVED
        conn.execute("INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(2025,?1,0,1)", [&j]).unwrap();
        assert!(matches!(
            load(&conn, 2025),
            Err(CliError::StaleParkedDraft {
                year: 2025,
                found: 0,
                ..
            })
        ));
        assert!(
            draft_exists(&conn, 2025).unwrap(),
            "stale parked is preserved, not discarded"
        );
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

    /// ★ I-3 — `commit` gates I-11 per-YEAR, not per-call: passing the (only) 2024 tables with `year = 2025`
    /// would `screen_inputs`-pass, but writing a committed row for a table-less year poisons it at resolve.
    /// The year-consistency guard returns `NoTables` and writes NOTHING.
    #[test]
    fn commit_refuses_tables_for_a_different_year_and_writes_nothing() {
        use btctax_adapters::{BundledFullReturnTables, BundledTaxTables};
        use btctax_core::tax::tables::FullReturnTables;
        use btctax_core::TaxTables;
        let (_dir, path, pp) = tmp_vault();
        let tables = BundledTaxTables::load(); // ★ I-B: load() returns Self, not Result
        let fr = BundledFullReturnTables::load();
        let (t2024, p2024) = (
            tables.table_for(2024).unwrap(),
            fr.full_return_for(2024).unwrap(),
        );
        let mut sess = Session::open(&path, &pp).unwrap();
        let clean = clean_screened_ri();
        // 2024 tables passed with year = 2025 → the year↔table mismatch is caught before any write.
        let out = commit(&mut sess, 2025, &clean, Some(t2024), Some(p2024)).unwrap();
        assert!(
            matches!(out, CommitOutcome::NoTables),
            "tables for a different year → NoTables, not a committed write"
        );
        assert!(
            !crate::return_inputs::exists(sess.conn(), 2025).unwrap(),
            "the table-less year is never poisoned with a committed row"
        );
    }

    /// ★ §6.2 — an authoritative committed-row write CLEARS a WIP draft but REFUSES a parked one. A WIP
    /// draft is regenerable crash-scratch, so the write supersedes it; a parked draft is the SOLE copy of a
    /// screened return (C-1), so it is never silently destroyed — the write is refused, naming both exits.
    #[test]
    fn coherence_clears_wip_but_refuses_parked() {
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        // WIP draft → cleared
        set_draft_row(&conn, 2024, &ri, false).unwrap();
        coherence_clear_or_refuse(&conn, 2024).unwrap();
        assert!(
            !draft_exists(&conn, 2024).unwrap(),
            "coherence clears a WIP draft"
        );
        // parked draft → refused, preserved, message names both exits
        set_draft_row(&conn, 2025, &ri, true).unwrap();
        let err = coherence_clear_or_refuse(&conn, 2025).unwrap_err();
        assert!(matches!(
            err,
            CliError::ParkedDraftBlocksWrite { year: 2025 }
        ));
        let msg = err.to_string();
        assert!(
            msg.contains("use full return") && msg.contains("discard parked draft"),
            "M-d: names both exits"
        );
        assert!(
            draft_exists(&conn, 2025).unwrap(),
            "a parked draft is never silently destroyed"
        );
        // no draft → Ok
        coherence_clear_or_refuse(&conn, 2030).unwrap();
    }

    #[test]
    fn park_stashes_then_deletes_committed_atomically() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        crate::return_inputs::set(sess.conn(), 2024, &ri).unwrap();
        park_to_profile(&mut sess, 2024).unwrap();
        // committed row gone; draft holds it with parked=1
        assert!(
            !crate::return_inputs::exists(sess.conn(), 2024).unwrap(),
            "park deletes the committed row"
        );
        let d = get_draft_row(sess.conn(), 2024).unwrap().unwrap();
        assert!(
            d.parked && d.ri.filing_status == FilingStatus::Mfj,
            "park stashes the row as parked"
        );
        // survives disk (I-7)
        drop(sess);
        let s2 = Session::open(&path, &pp).unwrap();
        assert!(get_draft_row(s2.conn(), 2024).unwrap().unwrap().parked);
        assert!(
            !crate::return_inputs::exists(s2.conn(), 2024).unwrap(),
            "the committed-row DELETE also reached disk, not just the stash"
        );
    }

    #[test]
    fn park_refuses_without_committed_row_and_on_any_wip() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        assert!(park_to_profile(&mut sess, 2024).is_err(), "nothing to park");
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        crate::return_inputs::set(sess.conn(), 2024, &ri).unwrap();
        set_draft_row(sess.conn(), 2024, &ri, false).unwrap(); // a WIP draft occupies the slot
        assert!(
            park_to_profile(&mut sess, 2024).is_err(),
            "clean-state gate: won't clobber a WIP draft"
        );
        assert!(
            crate::return_inputs::exists(sess.conn(), 2024).unwrap(),
            "a refused park leaves the committed row"
        );
    }

    /// Mirrors `tax_profile::tests::prof()` (that fixture is private to its own module) — a minimal
    /// well-formed `TaxProfile` sufficient to make `tax_profile::years` report the year as present.
    fn sample_profile() -> btctax_core::TaxProfile {
        use btctax_core::{Carryforward, TaxProfile};
        use rust_decimal_macros::dec;
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
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        }
    }

    #[test]
    fn active_source_follows_resolve_precedence() {
        let conn = Connection::open_in_memory().unwrap();
        crate::return_inputs::init_table(&conn).unwrap();
        crate::tax_profile::init_table(&conn).unwrap();
        assert!(matches!(
            active_source(&conn, 2024).unwrap(),
            ActiveSource::Neither
        ));
        crate::tax_profile::set(&conn, 2024, &sample_profile()).unwrap();
        assert!(matches!(
            active_source(&conn, 2024).unwrap(),
            ActiveSource::TaxProfile
        ));
        assert!(shadows_profile(&conn, 2024).unwrap());
        // committed return_inputs wins
        crate::return_inputs::set(&conn, 2024, &ReturnInputs::default()).unwrap();
        assert!(matches!(
            active_source(&conn, 2024).unwrap(),
            ActiveSource::FullReturn
        ));
    }

    #[test]
    fn discard_parked_draft_only_deletes_a_parked_row() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        // a WIP draft is NOT discardable via this path
        set_draft_row(sess.conn(), 2024, &ri, false).unwrap();
        assert!(
            discard_parked_draft(&mut sess, 2024).is_err(),
            "won't delete a WIP behind 'discard parked'"
        );
        assert!(draft_exists(sess.conn(), 2024).unwrap());
        // a parked draft IS discardable
        set_draft_row(sess.conn(), 2024, &ri, true).unwrap();
        discard_parked_draft(&mut sess, 2024).unwrap();
        assert!(!draft_exists(sess.conn(), 2024).unwrap());
    }

    #[test]
    fn draft_row_set_get_delete_roundtrip_with_parked() {
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
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
