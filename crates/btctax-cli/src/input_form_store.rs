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
    // ★ P2-b: distinguish "no row" (→ false) from a real DB error (→ propagate). `.is_ok()` swallowed the
    // latter as "no draft"; mirror `parked_flag`'s correct QueryReturnedNoRows-vs-Err split.
    match conn.query_row(
        "SELECT 1 FROM return_inputs_draft WHERE year=?1",
        [year],
        |_| Ok(()),
    ) {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// ★ spec 1099-DA T9 — every year the DRAFT table holds a row for. `Session::broker_reporting_answers`
/// unions this with `return_inputs::years` so a year whose answers live ONLY in the draft (the
/// primary authoring path on a params-less year) is not invisible to every read-only surface.
pub fn draft_years(conn: &Connection) -> Result<Vec<i32>, CliError> {
    init_draft_table(conn)?;
    let mut st = conn.prepare("SELECT year FROM return_inputs_draft ORDER BY year")?;
    let rows = st.query_map([], |r| r.get::<_, i32>(0))?;
    let mut out = Vec::new();
    for y in rows {
        out.push(y?);
    }
    Ok(out)
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
    /// ★★ T4 fold, seam review M-3 — `Some(clause)` when the stale draft was **KEPT** rather than
    /// discarded: it holds work (T4/C-1), so [`load`] refuses it to a writer and [`load_for_read`]
    /// hands a read-only caller this note instead. `None` is the §6.3 discard the note was born for.
    pub kept_holdings: Option<String>,
}

impl std::fmt::Display for StaleNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kept_holdings {
            None => write!(
                f,
                "discarded a stale draft for {} (schema v{}, expected v{})",
                self.year, self.found, self.expected
            ),
            Some(held) => write!(
                f,
                "KEPT a stale draft for {} (schema v{}, expected v{}) holding {held} — this build \
                 cannot read it, so it was skipped, not deleted",
                self.year, self.found, self.expected
            ),
        }
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
            // ★★★ **T4 / R11 — §6.3's "regenerable" premise, tested rather than assumed.**
            //
            //     §6.3 discards a stale WIP draft silently because *"it is regenerable, so refusing
            //     would brick a resume for no benefit"*. That is true of crash-recovery scratch and
            //     false of an interview: `answer_log` records cannot be re-created by re-typing
            //     (`record_answer` writes one only when btctax itself asked), and on a year with no
            //     package the draft is the only store there is. So the discard is now narrowed to
            //     the drafts §6.3 was written about, and the rest fail closed like the parked half.
            // ★★★ FOLD C-1 — the same structural decision, so the two paths cannot diverge.
            if !draft_is_disposable(&d.ri) {
                return Err(CliError::StaleDraftHoldsInterview {
                    year,
                    found: d.version,
                    expected: SCHEMA_VERSION,
                    holdings: describe_draft(&d.ri),
                });
            }
            // ★ §6.3 / I-1: a stale WIP draft is regenerable — discard it and RETURN the note (never
            // eprintln! from a store read fn: plan 3's raw-mode TUI would swallow/garble it), fall through.
            delete_draft(conn, year)?;
            let note = StaleNote {
                year,
                found: d.version,
                expected: SCHEMA_VERSION,
                kept_holdings: None, // §6.3: this one really was discarded
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

/// ★★★ **T4 fold, seam review M-3 — the READ-ONLY resolution of a year's working return.**
///
/// [`load`] fails closed on a stale draft that holds work (C-1): a caller about to WRITE must not
/// proceed past a draft this build cannot read. A read-only caller is a different question — it
/// deletes nothing, and refusing it turns *"your draft is unreadable"* into *"you cannot look at
/// this year at all"*: a new hard failure on `report`, `income show-broker-answers`,
/// `export-irs-pdf`'s broker path and `income scrub`, reachable the first time `SCHEMA_VERSION`
/// moves.
///
/// So a read-only caller SKIPS the unreadable draft — **keeping** it — falls through to the
/// committed row, and is handed a [`StaleNote`] saying so. Every existing renderer of that note
/// already words it as *"skipped … Nothing was deleted"* (`cmd/tax.rs`'s scrub note,
/// `cmd/admin.rs`'s `stale_draft_note`), which is exactly true on this path and was only
/// approximately true on the §6.3 one.
pub fn load_for_read(
    conn: &Connection,
    year: i32,
) -> Result<(Loaded, Option<StaleNote>), CliError> {
    match load(conn, year) {
        Err(CliError::StaleDraftHoldsInterview {
            year,
            found,
            expected,
            holdings,
        }) => Ok((
            committed_or_fresh(conn, year)?,
            Some(StaleNote {
                year,
                found,
                expected,
                kept_holdings: Some(holdings),
            }),
        )),
        other => other,
    }
}

/// ★★★ spec 1099-DA T9 — **the year's WORKING return, for a reader that must not create a row.**
///
/// A thin wrapper over [`load`]: the §6.1 precedence every other reader uses, **a draft shadows the
/// committed row**, the §6.3 stale split unchanged (a stale WIP draft is discarded and the caller
/// gets the [`StaleNote`] to surface; a stale PARKED draft refuses with
/// [`CliError::StaleParkedDraft`] before the caller writes a byte).
///
/// Two things it does that `load` does not, and they are the whole point:
///
/// - **A PARKED draft carries NO live return.** `parked = 1` marks a return the filer switched back
///   to a raw tax-profile — testimony they WITHDREW. Filing from it would file withdrawn testimony,
///   so the accessor answers `None` and the caller falls to its no-answers arm.
/// - **It never creates anything.** No row is written and `ReturnInputs::default()` is never
///   persisted: the default carries `filing_status: Single`, testimony the filer never gave, which
///   `classify()`'s own exemption reason ("no default to launder") forbids. `resolve.rs` is
///   untouched by this file, so a draft still cannot shadow a stored `tax_profile`.
pub fn working_return(
    conn: &Connection,
    year: i32,
) -> Result<(Option<ReturnInputs>, Option<StaleNote>), CliError> {
    // ★ M-3: read-only — an unreadable draft is skipped and kept, never a hard failure.
    let (loaded, note) = load_for_read(conn, year)?;
    let ri = match loaded {
        // A parked draft is a WITHDRAWN return — not the working one (see above).
        Loaded::Draft { parked: true, .. } => None,
        Loaded::Draft { ri, parked: false } => Some(ri),
        Loaded::Committed(ri) => Some(ri),
        Loaded::Fresh => None,
    };
    Ok((ri, note))
}

/// ★ spec 1099-DA T9 — the **Form 1099-DA answers** projection of [`working_return`]: the ONE
/// resolution every reader of `broker_reporting` goes through, so the viewer's Box column, the TUI
/// export, `btctax export --csv` and the CLI export can never show or file two different answer
/// sets. `None` when no working return exists (or it is parked).
///
/// The one deliberate exception is the FULL return (`export-irs-pdf` arm (1)), which files from the
/// COMMITTED row by design — committing is possible on a params-bundled year, and nobody re-points
/// that arm here.
pub fn broker_answers(
    conn: &Connection,
    year: i32,
) -> Result<Option<btctax_core::BrokerReporting>, CliError> {
    Ok(working_return(conn, year)?.0.map(|ri| ri.broker_reporting))
}

/// The no-draft tail of [`load`]: the committed `return_inputs` row (if any) else `Fresh`.
fn committed_or_fresh(conn: &Connection, year: i32) -> Result<Loaded, CliError> {
    match crate::return_inputs::get(conn, year)? {
        Some(ri) => Ok(Loaded::Committed(ri)),
        None => Ok(Loaded::Fresh),
    }
}

/// ★★★ **T4 / `SPEC_interview.md` R11 — WHAT A WIP DRAFT HOLDS.**
///
/// §6.2 called a WIP draft *"regenerable crash-scratch"* and superseded it with a note. R11 makes
/// the draft the **Sep–Dec store for TY2026**: a year with no `FullReturnParams` cannot commit at
/// all, so the interview lives in the draft for months. This is the predicate that tells the two
/// apart — and it names what is at stake rather than answering a bare yes/no, because a refusal
/// that cannot say *what* it is protecting reads as an obstruction.
///
/// ★ The document counts are DERIVED from the census (`DocumentRow::ALL` × `declared_rows`), never
///   from a hand-list of `Vec` fields: a document type that gains a section is counted here the day
///   it gains one, with no edit to this file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DraftHoldings {
    /// `answer_log` records — **the unreproducible part**. `record_answer` writes one only when
    /// btctax itself put the question, and `income import` refuses to read records back (*"a record
    /// of an act this vault never observed"*), so a discarded log cannot be re-created by re-typing.
    pub answers: usize,
    /// Transcribed document rows, per census row that carries any — `("Form W-2", 2)`.
    pub documents: Vec<(&'static str, usize)>,
    /// Dependent rows.
    pub dependents: usize,
    /// Whether a Schedule A has been created.
    pub schedule_a: bool,
}

impl DraftHoldings {
    /// Nothing an interview put there — the draft is the disposable scratch §6.2 assumed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.answers == 0 && self.documents.is_empty() && self.dependents == 0 && !self.schedule_a
    }

    /// What the draft holds, as the clause a refusal (or a discard note) prints.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.answers > 0 {
            parts.push(format!("{} recorded answer(s)", self.answers));
        }
        for (doc, n) in &self.documents {
            parts.push(format!("{n} {doc} row(s)"));
        }
        if self.dependents > 0 {
            parts.push(format!("{} dependent(s)", self.dependents));
        }
        if self.schedule_a {
            parts.push("a Schedule A".to_string());
        }
        if parts.is_empty() {
            "nothing".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// ★★★ **T4 fold, seam review C-1 — THE DISPOSABLE PREDICATE, AND IT NAMES NO CATEGORY.**
///
/// A draft is disposable exactly when it is still the year's FRESH SEED: a tax year and a filing
/// status over `ReturnInputs::default()`. Anything else — one scalar, one answer, one row, one
/// struct that does not exist yet — is work, and work is never destroyed on a note.
///
/// ★★ **Why a comparison and not a list.** T4 shipped [`DraftHoldings::is_empty`] as the decision,
///    and it enumerates four categories. `ReturnInputs.broker_reporting` is not among them — and
///    this file's own header calls it *"the primary authoring path on a params-less year"* — so a
///    TY2026 draft holding the Form 1099-DA answers and a Schedule C reported *nothing* and was
///    destroyed, unconfirmed, by `income import`, `income clear`, `report --write-carryover` and
///    `load`'s stale discard. The failure direction of a list is OPEN: whatever is not on it is
///    disposable. A comparison against the seed fails CLOSED, so a field added to `ReturnInputs`
///    tomorrow is protected the day it is added and no one has to remember this file.
///
/// ★ The seed carries the filer's `tax_year` and `filing_status` rather than being bare
///   `Default::default()`: opening a year and choosing a status is not work, and treating it as
///   work would demand a confirmation for the crash-scratch §6.2 was actually written about.
///   Everything past that point is.
#[must_use]
pub fn draft_is_disposable(ri: &ReturnInputs) -> bool {
    *ri == ReturnInputs {
        tax_year: ri.tax_year,
        filing_status: ri.filing_status,
        ..Default::default()
    }
}

/// The clause a refusal or a discard note prints for `ri` — [`DraftHoldings::describe`], with the
/// fallback the structural predicate makes necessary.
///
/// ★★ [`draft_is_disposable`] can say *"this holds work"* about a field [`DraftHoldings`] does not
///    count, and a filer must never be told a draft holds **nothing** at the moment they are being
///    asked to confirm destroying it. So a non-disposable draft with no itemised category is
///    described as *"work not otherwise itemised"* — the honest answer, and the one that stays true
///    as `ReturnInputs` grows.
#[must_use]
pub fn describe_draft(ri: &ReturnInputs) -> String {
    let held = draft_holdings(ri);
    let unitemised = !draft_is_disposable(ri) && held.is_empty();
    match (held.is_empty(), unitemised) {
        (true, true) => "work not otherwise itemised".to_string(),
        (true, false) => "nothing".to_string(),
        _ => held.describe(),
    }
}

/// [`DraftHoldings`] for one return.
///
/// ★ This is the MESSAGE's input, not the decision's — see [`draft_is_disposable`], which is what
///   `coherence_check` and `load` key on.
#[must_use]
pub fn draft_holdings(ri: &ReturnInputs) -> DraftHoldings {
    use btctax_core::tax::document_census::{declared_rows, DocumentRow};
    DraftHoldings {
        answers: ri.answer_log.len(),
        documents: DocumentRow::ALL
            .iter()
            .filter_map(|row| match declared_rows(ri, *row) {
                Some(n) if n > 0 => Some((row.designation(), n)),
                _ => None,
            })
            .collect(),
        dependents: ri.header.dependents.len(),
        schedule_a: ri.schedule_a.is_some(),
    }
}

/// What [`coherence_check`] found, and therefore what [`coherence_clear`] may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftCoherence {
    /// No draft row for the year — the writer proceeds unchanged.
    Absent,
    /// A WIP draft holding nothing an interview put there: §6.2's disposable crash-scratch.
    Disposable,
    /// A WIP draft holding work, whose discard the caller has explicitly CONFIRMED
    /// (`--discard-draft` on the CLI; a payload-confirm in the TUI). Carries the rendered clause
    /// naming what will be lost — see [`describe_draft`], which is where the *"work not otherwise
    /// itemised"* fallback lives.
    ConfirmedDiscard(String),
}

/// §6.2 draft-coherence, READ HALF: what an authoritative committed-row write is about to
/// supersede, refusing where it must not.
///
/// The four writers of the committed `return_inputs` row (`income import`, `income answer`,
/// carryover write-back, `income clear`) are ignorant of the draft. A stale draft would then
/// silently shadow the freshly-written committed row at the next `load` (§6.1 precedence); a PARKED
/// draft — the sole copy of a screened return (C-1) — could be clobbered out of existence; and
/// since R11 a WIP draft may be months of interview.
///
/// - **no draft** → [`DraftCoherence::Absent`].
/// - **parked** (`parked = 1`) → [`CliError::ParkedDraftBlocksWrite`] (C-1, fail closed).
/// - **WIP holding an interview**, `discard_draft = false` → [`CliError::NonTrivialDraftBlocksWrite`]
///   (T4/R11), naming what the draft holds and the remedy.
/// - **WIP holding an interview**, `discard_draft = true` → [`DraftCoherence::ConfirmedDiscard`].
/// - **any other WIP** → [`DraftCoherence::Disposable`].
///
/// ★ It DELETES NOTHING. That is the point of the split: every refusal a write can raise is raised
///   before the write's own screens run, and the destructive half ([`coherence_clear`]) happens
///   only on the path that actually writes.
pub fn coherence_check(
    conn: &Connection,
    year: i32,
    discard_draft: bool,
) -> Result<DraftCoherence, CliError> {
    match parked_flag(conn, year)? {
        None => Ok(DraftCoherence::Absent),
        Some(true) => Err(CliError::ParkedDraftBlocksWrite { year }), // ★ C-1: never clobber the sole copy
        Some(false) => {
            let Some(d) = get_draft_row(conn, year)? else {
                return Ok(DraftCoherence::Absent);
            };
            // ★★★ FOLD C-1 — the DECISION is [`draft_is_disposable`], a comparison against the
            //     year's fresh seed. It reads no category list, so a draft holding something this
            //     file has never heard of (the Form 1099-DA answers were the shipped instance) is
            //     protected. `describe_draft` renders the message, and only the message.
            if draft_is_disposable(&d.ri) {
                return Ok(DraftCoherence::Disposable);
            }
            let clause = describe_draft(&d.ri);
            if discard_draft {
                Ok(DraftCoherence::ConfirmedDiscard(clause))
            } else {
                Err(CliError::NonTrivialDraftBlocksWrite {
                    year,
                    holdings: clause,
                })
            }
        }
    }
}

/// §6.2 draft-coherence, WRITE HALF: delete the draft [`coherence_check`] admitted, saying what was
/// superseded. The delete is in-memory on `conn`; the writer's own `save()` persists it together
/// with the committed write, so a writer that fails before saving destroys nothing.
pub fn coherence_clear(
    conn: &Connection,
    year: i32,
    checked: &DraftCoherence,
) -> Result<(), CliError> {
    match checked {
        DraftCoherence::Absent => Ok(()),
        DraftCoherence::Disposable => {
            // The pre-T4 note, unchanged for the drafts §6.2 was written about.
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
        DraftCoherence::ConfirmedDiscard(clause) => {
            eprintln!("{}", discard_note(year, clause));
            delete_draft(conn, year)?;
            Ok(())
        }
    }
}

/// ★ R11: a confirmed discard NAMES WHAT WAS LOST. A destruction that does not say what it
/// destroyed is the same silence the refusal exists to break — so the sentence is a function, held
/// by [`tests::the_confirmed_discard_note_names_what_was_lost`], rather than an `eprintln!` nobody
/// can assert on.
#[must_use]
pub fn discard_note(year: i32, clause: &str) -> String {
    format!("note: discarded the {year} work-in-progress draft as requested; it held {clause}.")
}

/// [`coherence_check`] then [`coherence_clear`] — the shape a writer wants when its own screens
/// cannot refuse between the two.
///
/// ★ M-1: callers invoke this RIGHT AFTER `Session::open`, before any committed-row read or write —
/// else the two writers that early-return on an absent committed row (`answer`, write-back) would
/// exit with a generic "no inputs" message before the parked-refuse is ever reached (a parked year
/// has no committed row).
pub fn coherence_clear_or_refuse(
    conn: &Connection,
    year: i32,
    discard_draft: bool,
) -> Result<(), CliError> {
    let checked = coherence_check(conn, year, discard_draft)?;
    coherence_clear(conn, year, &checked)
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

/// Run in-memory `mutate` writes then `save()`, rolling the WHOLE session back to the pre-write image on
/// ANY failure — a `mutate` error OR a `save` error (P2-d). Previously only a `save` failure restored, so a
/// mid-write `set`/`delete` error returned early leaving the long-lived in-memory `Session` partially
/// mutated (and diverged from disk). The snapshot is taken here, before `mutate` touches the conn.
fn mutate_and_save(
    sess: &mut Session,
    mutate: impl FnOnce(&Connection) -> Result<(), CliError>,
) -> Result<(), CliError> {
    let snap = sess.snapshot()?;
    let mutated = mutate(sess.conn());
    let outcome = mutated.and_then(|()| sess.save());
    if let Err(e) = outcome {
        sess.restore(&snap)?; // atomic: never leave an in-memory/disk split or a partial in-memory write
        return Err(e);
    }
    Ok(())
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
///   `sess.save()` via `mutate_and_save` (which rolls back on ANY failure) so there is never an
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
    // ★★★ **A TIER BOUNDARY, RECORDED** (T6 seam review M-4). This runs `screen_inputs` and NOT
    //     `screen_compute_dependent`, because this site holds no `LedgerState` — the TUI's commit
    //     path carries a `ReturnInputs` and a `Session`, and projecting the vault here to run one
    //     more screen would make committing depend on a ledger R9 promises may still be unresolved
    //     ("authoring proceeds in parallel with an unresolved ledger"). So a filer CAN commit
    //     `digital_asset_activity = Some(false)` against a ledger full of disposals, and first meets
    //     the refusal at `report` / `export-irs-pdf`.
    //
    // ★★ The deferral is a DECISION, not an oversight, and it is paid for on the surface that does
    //    hold the ledger: `btctax_cli::step0::step0_panel` raises that exact contradiction as a Step
    //    0 row — printed by `income answer` and on the TUI's tax-inputs entry screen — by calling
    //    the SAME `screen_digital_asset_answer` the export refuses on. The gate did not move; the
    //    silence did.
    if let Some(refusal) = screen_inputs(ri, table, params) {
        return Ok(CommitOutcome::Refused(refusal)); // fail-closed: writes nothing
    }
    mutate_and_save(sess, |conn| {
        crate::return_inputs::set(conn, year, ri)?;
        delete_draft(conn, year)?;
        Ok(())
    })?;
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
    mutate_and_save(sess, |conn| {
        set_draft_row(conn, year, &ri, true)?; // ★ stash FIRST (parked=1)
        crate::return_inputs::delete(conn, year)?; // ★ N-1: in-session delete, NOT `income clear`
        Ok(())
    }) // ★ atomic (D-6): a failed park (mutate OR save) never loses the committed row
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
/// Runs the delete + save through `mutate_and_save`, which restores the pre-write snapshot on ANY failure
/// (the delete OR the save; mirrors `park_to_profile`'s atomicity), so a failed discard never leaves an
/// in-memory/disk split or a partial in-memory delete.
pub fn discard_blocked_draft(sess: &mut Session, year: i32) -> Result<(), CliError> {
    // ★ T4/R11: the affordance now covers BOTH drafts `load` refuses to open — a PARKED committed
    //   return (C-1) and a STALE one holding an interview. It still refuses a readable WIP draft:
    //   that one is editable in the form, so a blanket delete button would be a footgun.
    let Some(row) = get_draft_row(sess.conn(), year)? else {
        return Err(CliError::Usage(format!(
            "year {year} has no draft to discard"
        )));
    };
    if !row.parked && row.version == SCHEMA_VERSION {
        return Err(CliError::Usage(format!(
            "year {year}'s draft is readable — open the tax-inputs form for {year} to edit or \
             finish it; this affordance discards only a draft this build cannot open"
        )));
    }
    mutate_and_save(sess, |conn| {
        delete_draft(conn, year)?;
        Ok(())
    })
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

    /// ★★★ **§G-16 — `delete_draft` must actually destroy the draft's bytes.**
    ///
    /// The draft row holds the filer's SSNs, DOBs and every superseded income figure they typed and
    /// then discarded. Before the fix, `delete_draft`'s plain `DELETE` freed the row's pages without
    /// overwriting them, `db_to_bytes` serialized **every page including free ones**, and `save()`
    /// encrypted that — so discarded identity data rode into every subsequent vault generation
    /// **indefinitely**, while the code read as a deletion.
    ///
    /// ★ The assertion searches the SERIALIZED IMAGE for raw bytes. A row-level query would report
    /// the draft gone and prove nothing at all — which is exactly how this survived review.
    #[test]
    fn delete_draft_leaves_no_trace_of_the_discarded_values_in_the_image() {
        let (_dir, path, pp) = tmp_vault();
        // ★ A sentinel that could only have come from the draft. Deliberately NOT shaped like real
        // identity data: the test needs a unique byte string, not a realistic one, and an
        // SSN-shaped literal would (correctly) trip `scripts/pii-scan-generic.sh` — which is exactly
        // what happened on the first attempt at this test.
        let sentinel = "ZZ-G16-DISCARDED-DRAFT-CANARY";
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            foreign_country_names: sentinel.to_string(),
            ..Default::default()
        };

        let mut sess = Session::open(&path, &pp).unwrap();
        save_draft(&mut sess, 2024, &ri).unwrap();
        let before = sess.snapshot().unwrap();
        assert!(
            contains(&before, sentinel.as_bytes()),
            "the sentinel must be in the image BEFORE the delete, or this test proves nothing"
        );

        assert!(
            delete_draft(sess.conn(), 2024).unwrap(),
            "the draft row existed"
        );
        sess.save().unwrap();
        let after = sess.snapshot().unwrap();

        assert!(
            !contains(&after, sentinel.as_bytes()),
            "§G-16: the discarded draft value SURVIVES the delete in the serialized image \
             ({} bytes). `delete_draft` must destroy the bytes, not just unlink the row — otherwise \
             every later vault generation carries the filer's discarded SSNs and DOBs.",
            after.len()
        );
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
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

    /// P2-d: a `mutate` error (not only a `save` error) must roll the in-memory DB back. The closure writes
    /// a draft row, THEN fails before save; after `mutate_and_save` returns Err, that write must be GONE —
    /// restored, not left partially applied in the long-lived session. (Kills the restore-only-on-save
    /// mutant: without the fix the draft write survives the mutate error.)
    #[test]
    fn mutate_and_save_rolls_back_the_in_memory_write_on_a_mutate_error() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        assert!(
            !draft_exists(sess.conn(), 2024).unwrap(),
            "precondition: no draft yet"
        );

        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        let r = mutate_and_save(&mut sess, |conn| {
            set_draft_row(conn, 2024, &ri, false)?; // in-memory write ...
            Err(CliError::Usage("boom".to_string())) // ... then fail BEFORE save
        });
        assert!(r.is_err(), "the mutate error must propagate");

        assert!(
            !draft_exists(sess.conn(), 2024).unwrap(),
            "P2-d: a mutate error must restore the pre-write image; the draft write must NOT survive"
        );
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
                expected: SCHEMA_VERSION,
                kept_holdings: None,
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

    /// ★★★ **T1's kill — THE VERSION-2 SPLIT, both halves.** A v2 WIP draft is regenerable
    /// crash-scratch, so it is DISCARDED and the fact comes back as a [`StaleNote`] the caller can
    /// surface; a v2 PARKED draft may be the sole copy of a screened return (C-1), so it REFUSES and
    /// the row survives. Both are pinned to the literal 2 — the specific predecessor the interview's
    /// provenance schema replaced — rather than to `SCHEMA_VERSION - 1`, which would follow the
    /// constant on the next bump and stop testing anything.
    #[test]
    fn a_version_2_wip_draft_is_discarded_with_a_note_and_a_version_2_parked_draft_refuses() {
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        let j = serde_json::to_string(&ri).unwrap();
        conn.execute(
            "INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(2026,?1,2,0)",
            [&j],
        )
        .unwrap();
        let (loaded, note) = load(&conn, 2026).unwrap();
        assert!(matches!(loaded, Loaded::Fresh));
        assert_eq!(
            note,
            Some(StaleNote {
                year: 2026,
                found: 2,
                expected: 3,
                kept_holdings: None,
            }),
            "a v2 WIP draft must be discarded WITH the note (never silently)"
        );
        assert!(
            !draft_exists(&conn, 2026).unwrap(),
            "the v2 WIP draft is gone"
        );

        conn.execute(
            "INSERT INTO return_inputs_draft(year,inputs_json,schema_version,parked) VALUES(2027,?1,2,1)",
            [&j],
        )
        .unwrap();
        assert!(
            matches!(
                load(&conn, 2027),
                Err(CliError::StaleParkedDraft {
                    year: 2027,
                    found: 2,
                    expected: 3
                })
            ),
            "a v2 PARKED draft must REFUSE — it may hold carryover that exists nowhere else"
        );
        assert!(
            draft_exists(&conn, 2027).unwrap(),
            "the refused parked draft must survive, not be destroyed"
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
        coherence_clear_or_refuse(&conn, 2024, false).unwrap();
        assert!(
            !draft_exists(&conn, 2024).unwrap(),
            "coherence clears a WIP draft"
        );
        // parked draft → refused, preserved, message names both exits
        set_draft_row(&conn, 2025, &ri, true).unwrap();
        let err = coherence_clear_or_refuse(&conn, 2025, false).unwrap_err();
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
        coherence_clear_or_refuse(&conn, 2030, false).unwrap();
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
    fn discard_blocked_draft_only_deletes_a_draft_the_build_cannot_open() {
        let (_dir, path, pp) = tmp_vault();
        let mut sess = Session::open(&path, &pp).unwrap();
        let ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        // a WIP draft is NOT discardable via this path
        set_draft_row(sess.conn(), 2024, &ri, false).unwrap();
        assert!(
            discard_blocked_draft(&mut sess, 2024).is_err(),
            "won't delete a WIP behind 'discard parked'"
        );
        assert!(draft_exists(sess.conn(), 2024).unwrap());
        // a parked draft IS discardable
        set_draft_row(sess.conn(), 2024, &ri, true).unwrap();
        discard_blocked_draft(&mut sess, 2024).unwrap();
        assert!(!draft_exists(sess.conn(), 2024).unwrap());
    }

    /// ★★★ **T4/R11 — `draft_holdings` counts what an interview put there, DERIVED from the
    ///     census, and `describe()` names it.**
    ///
    /// The derivation is the point: the document counts come from `DocumentRow::ALL` ×
    /// `declared_rows`, so a type that gains a section is counted the day it gains one. A hand-list
    /// of `Vec` fields is exactly the shape that goes stale silently — and a draft wrongly called
    /// disposable is destroyed by a note.
    #[test]
    fn draft_holdings_counts_answers_documents_dependents_and_a_schedule_a() {
        use btctax_core::tax::document_census::{declared_rows, DocumentRow};
        use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
        use btctax_core::tax::questions::{QuestionId, FORM_QUESTIONS};
        use btctax_core::tax::return_inputs::{Dependent, ScheduleAInputs, W2};

        // (a) a bare return holds nothing — this is the disposable crash-scratch §6.2 was written about
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Mfj,
            ..Default::default()
        };
        assert!(
            draft_holdings(&ri).is_empty(),
            "a filing status alone is not an interview"
        );
        assert_eq!(draft_holdings(&ri).describe(), "nothing");

        // (b) one recorded answer is enough — it is the unreproducible part
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::ForeignTrust)
            .expect("the foreign-trust declaration is in the registry");
        record_answer(
            &mut ri,
            AnswerKey::Question(q.id),
            q.prompt,
            time::macros::date!(2026 - 09 - 01),
            AnswerState::Given,
        );
        let h = draft_holdings(&ri);
        assert!(!h.is_empty());
        assert_eq!(h.answers, 1);
        assert!(
            h.describe().contains("1 recorded answer(s)"),
            "{}",
            h.describe()
        );

        // (c) transcribed documents, a dependent and a Schedule A each count, and each is NAMED
        ri.w2s.push(W2 {
            employer: "ACME".into(),
            ..Default::default()
        });
        ri.header.dependents.push(Dependent::default());
        ri.schedule_a = Some(ScheduleAInputs::default());
        let h = draft_holdings(&ri);
        assert_eq!(h.documents, vec![("Form W-2", 1)]);
        assert_eq!(h.dependents, 1);
        assert!(h.schedule_a);
        let d = h.describe();
        for want in [
            "1 recorded answer(s)",
            "1 Form W-2 row(s)",
            "1 dependent(s)",
            "a Schedule A",
        ] {
            assert!(d.contains(want), "describe() must name {want}: {d}");
        }

        // (d) the DERIVATION, asserted rather than assumed: every countable census row is reachable
        //     here, so a new document section cannot be silently uncounted.
        let countable: Vec<DocumentRow> = DocumentRow::ALL
            .iter()
            .copied()
            .filter(|r| declared_rows(&ReturnInputs::default(), *r).is_some())
            .collect();
        assert!(
            countable.contains(&DocumentRow::Int1099),
            "premise: the census knows how to count 1099-INT rows"
        );
        for row in countable {
            let mut r = ReturnInputs::default();
            match row {
                DocumentRow::W2 => r.w2s.push(W2::default()),
                DocumentRow::Int1099 => r.int_1099.push(Default::default()),
                DocumentRow::Div1099 => r.div_1099.push(Default::default()),
                DocumentRow::B1099 => r.b_1099.push(Default::default()),
                DocumentRow::G1099 => r.g_1099.push(Default::default()),
                // ★ T5 — the 1098-E became countable when `Form1098E` replaced the
                //   `sch1.student_loan_interest_paid` scalar.
                DocumentRow::Form1098e => r.form_1098e.push(Default::default()),
                // ★ T16 — the two HSA information returns became countable with Form 8889.
                DocumentRow::Sa1099 => r.sa_1099.push(Default::default()),
                DocumentRow::Sa5498 => r.sa_5498.push(Default::default()),
                other => panic!("a new countable census row ({other:?}) needs a case here"),
            }
            assert!(
                !draft_holdings(&r).is_empty(),
                "{row:?}: a transcribed document makes a draft non-disposable"
            );
        }
    }

    /// ★★★ **T4/R11 — the coherence CHECK refuses without confirmation, deletes nothing, and the
    ///     CLEAR is what deletes.** The split is what makes "a refusal writes nothing" true of the
    ///     draft as well as of the committed row.
    #[test]
    fn coherence_check_refuses_a_draft_holding_an_interview_and_deletes_nothing() {
        use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
        use btctax_core::tax::questions::{QuestionId, FORM_QUESTIONS};
        let conn = Connection::open_in_memory().unwrap();
        init_draft_table(&conn).unwrap();
        let mut ri = ReturnInputs {
            filing_status: FilingStatus::Single,
            ..Default::default()
        };
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::ForeignTrust)
            .unwrap();
        record_answer(
            &mut ri,
            AnswerKey::Question(q.id),
            q.prompt,
            time::macros::date!(2026 - 09 - 01),
            AnswerState::Given,
        );
        set_draft_row(&conn, 2026, &ri, false).unwrap();

        let err = coherence_check(&conn, 2026, false).unwrap_err();
        assert!(
            matches!(err, CliError::NonTrivialDraftBlocksWrite { year: 2026, .. }),
            "{err:?}"
        );
        assert!(
            err.to_string().contains("1 recorded answer(s)"),
            "the refusal names what is at stake: {err}"
        );
        assert!(
            draft_exists(&conn, 2026).unwrap(),
            "the CHECK deletes nothing"
        );

        // Confirmed: the check admits it, and only the CLEAR removes it.
        let checked = coherence_check(&conn, 2026, true).unwrap();
        assert!(matches!(checked, DraftCoherence::ConfirmedDiscard(_)));
        assert!(draft_exists(&conn, 2026).unwrap(), "still not deleted");
        coherence_clear(&conn, 2026, &checked).unwrap();
        assert!(!draft_exists(&conn, 2026).unwrap());

        // A disposable draft — the year's fresh seed — needs no confirmation, exactly as before T4.
        set_draft_row(&conn, 2026, &ReturnInputs::default(), false).unwrap();
        assert_eq!(
            coherence_check(&conn, 2026, false).unwrap(),
            DraftCoherence::Disposable
        );
        coherence_clear_or_refuse(&conn, 2026, false).unwrap();
        assert!(!draft_exists(&conn, 2026).unwrap());
    }

    /// The confirmed discard's note names the year and every kind of thing that was lost.
    #[test]
    fn the_confirmed_discard_note_names_what_was_lost() {
        use btctax_core::tax::provenance::{record_answer, AnswerKey, AnswerState};
        use btctax_core::tax::questions::{QuestionId, FORM_QUESTIONS};
        use btctax_core::tax::return_inputs::W2;
        let mut ri = ReturnInputs::default();
        let q = FORM_QUESTIONS
            .iter()
            .find(|q| q.id == QuestionId::ForeignTrust)
            .unwrap();
        record_answer(
            &mut ri,
            AnswerKey::Question(q.id),
            q.prompt,
            time::macros::date!(2026 - 09 - 01),
            AnswerState::Given,
        );
        ri.w2s.push(W2::default());
        let note = discard_note(2026, &describe_draft(&ri));
        assert!(note.contains("2026"), "{note}");
        assert!(note.contains("1 recorded answer(s)"), "{note}");
        assert!(note.contains("1 Form W-2 row(s)"), "{note}");
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
