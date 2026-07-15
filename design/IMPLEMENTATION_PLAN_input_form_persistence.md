# Input-Form Persistence (plan 2 of 4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `btctax-cli` persistence layer for the input form — a `return_inputs_draft` table, `load`/`save_draft`/`commit`, the draft-coherence rule across the committed-row writers, and the non-destructive tax-profile toggle — so the TUI (plan 3) can autosave a mid-entry return, commit only a screened blob, and switch sources without data loss.

**Architecture:** A new module `crates/btctax-cli/src/input_form_store.rs` mirroring the existing `return_inputs.rs` side-table pattern. It depends on **core types only** (`ReturnInputs`, `screen_inputs`, `TaxTable`, `FullReturnParams`, `Refusal`) plus `Session`/`Connection` — **NOT** the `btctax-input-form` engine crate. Read/table fns take `conn: &Connection` (from the caller's already-held `Session::conn()`); the disk-reaching mutations (`save_draft`/`commit`/`park_to_profile`/`discard_parked_draft`) take `sess: &mut Session` to call `sess.save()` (I-7), using `snapshot()`/`restore()` for atomic rollback. `resolve.rs` is **never touched** — drafts are invisible to the resolver by construction (it reads only `return_inputs` + `tax_profile`).

**Tech Stack:** Rust, `rusqlite` (SQLite side-table in the encrypted vault), the `Session`/`Vault` PGP-encrypted store.

## Global Constraints

- **New module `crates/btctax-cli/src/input_form_store.rs`**; wire it into `crates/btctax-cli/src/lib.rs` (`pub mod input_form_store;`). Mirror `return_inputs.rs`'s conventions exactly (idempotent `init_table` called first by every fn; `PRIMARY KEY(year)`; upsert via `ON CONFLICT(year) DO UPDATE`; `all`/`years` avoid deserializing so one corrupt blob can't brick enumeration).
- **Depends on `btctax-core` + the store, NOT on `btctax-input-form`.** The store works with a materialized `&ReturnInputs`; the engine's `apply`/`Working` belong to the TUI (plan 3).
- **Never open a second `Session`** (N-1: `VaultLock::acquire` is non-blocking `try_lock_exclusive`, so a nested open errors `Locked` against itself). Every fn takes `conn: &Connection` or `sess: &mut Session` from the caller.
- **Disk (I-7):** `save_draft`/`commit`/`park_to_profile`/`discard_parked_draft` MUST call `sess.save()` (which re-encrypts + atomic-writes). Use the established `edit/persist.rs` pattern: `let snap = sess.snapshot()?;` before the writes, and `if let Err(e) = sess.save() { sess.restore(&snap)?; return Err(e); }` — a failed save never leaves an in-memory/disk split (critical for the park stash, where the SSNs exist nowhere else — D-6).
- **`resolve.rs` UNCHANGED** (precedence: committed `return_inputs` → `tax_profile` → pseudo → Missing; a refused `return_inputs` row returns early with `profile: None` and does NOT fall through to `tax_profile`). The new draft table must stay out of `resolve_core` entirely.
- **FROZEN — never edit:** `crates/btctax-core/src/tax/{types,compute,se}.rs`. `screen_inputs` unchanged.
- **`parked` semantics (C-1):** `parked = 0` = disposable WIP; `parked = 1` = a parked committed return (its sole copy) — protected like a committed row (coherence-writes REFUSE it; stale-version REFUSES-and-reimports it; only an explicit confirmed `discard_parked_draft` or a re-`commit` removes it).
- **Commit is TY2024-only (I-11):** `FullReturnParams` exist only for TY2024 (`BundledFullReturnTables` bundles only 2024). `commit` on a year lacking params returns `NoTables` and writes nothing (never commits unscreened, which would poison the year at resolve).
- **Gate per task:** `make check` (~7s; the fast suite + clippy `-D warnings`), NOT `cargo test --workspace`. TDD: write the failing test, watch it fail, implement, watch it pass, commit. **Mutation-check each guard** (delete it → a named test fails → restore; use a `cp` backup + `touch`, NEVER `git checkout` on uncommitted work).
- **Fish shell:** quote globs; use a heredoc for `git commit -F -`.
- **`CliError` (`crates/btctax-cli/src/lib.rs:29-93`)** gets two new struct variants (Task 3, Task 5), each with a `thiserror` `#[error(...)]` message that NAMES the remedy, following `StaleReturnInputs`'s shape.

---

## File Structure

- **Create** `crates/btctax-cli/src/input_form_store.rs` (~250 non-test lines): the `return_inputs_draft` table + `DraftRow` low-level I/O + `Loaded`/`load` + `save_draft` + `CommitOutcome`/`commit` + coherence helper + `park_to_profile`/`discard_parked_draft` + toggle info fns (`active_source`, `shadows_profile`).
- **Modify** `crates/btctax-cli/src/lib.rs`: `pub mod input_form_store;` + two `CliError` variants (`StaleParkedDraft`, `ParkedDraftBlocksWrite`).
- **Modify** `crates/btctax-cli/src/cmd/tax.rs`: insert the coherence call in `import_return_inputs` (:98), `write_back_carryover` (:461, on `year+1`), `clear_return_inputs` (:165).
- **Modify** `crates/btctax-cli/src/cmd/answer.rs`: insert the coherence call in `answer_return_inputs` (:205).

Interfaces produced (consumed by plan 3, the TUI):
```rust
pub enum Loaded { Draft { ri: ReturnInputs, parked: bool }, Committed(ReturnInputs), Fresh }
pub enum CommitOutcome { Committed, Refused(Refusal), NoTables }
pub enum ActiveSource { FullReturn, TaxProfile, Neither }
pub fn init_draft_table(conn: &Connection) -> Result<(), CliError>;
pub fn load(conn: &Connection, year: i32) -> Result<Loaded, CliError>;
pub fn save_draft(sess: &mut Session, year: i32, ri: &ReturnInputs) -> Result<(), CliError>;
pub fn commit(sess: &mut Session, year: i32, ri: &ReturnInputs,
              table: Option<&TaxTable>, params: Option<&FullReturnParams>) -> Result<CommitOutcome, CliError>;
pub fn coherence_clear_or_refuse(conn: &Connection, year: i32) -> Result<(), CliError>;
pub fn park_to_profile(sess: &mut Session, year: i32) -> Result<(), CliError>;
pub fn discard_parked_draft(sess: &mut Session, year: i32) -> Result<(), CliError>;
pub fn active_source(conn: &Connection, year: i32) -> Result<ActiveSource, CliError>;
pub fn shadows_profile(conn: &Connection, year: i32) -> Result<bool, CliError>;
```

---

### Task 1: The `return_inputs_draft` table + low-level `DraftRow` I/O

**Files:**
- Create: `crates/btctax-cli/src/input_form_store.rs`
- Modify: `crates/btctax-cli/src/lib.rs` (add `pub mod input_form_store;`)

**Interfaces:**
- Consumes: `rusqlite::Connection`, `btctax_core::tax::return_inputs::ReturnInputs`, `crate::return_inputs::SCHEMA_VERSION`, `crate::CliError`.
- Produces: `init_draft_table`, a private `DraftRow { ri: ReturnInputs, version: i64, parked: bool }`, `get_draft_row(conn, year) -> Result<Option<DraftRow>, CliError>` (returns the RAW version — it does NOT gate on `SCHEMA_VERSION`; `load` (Task 3) decides discard-vs-refuse), `set_draft_row(conn, year, ri, parked) -> Result<(), CliError>`, `delete_draft(conn, year) -> Result<bool, CliError>`, `draft_exists(conn, year) -> Result<bool, CliError>`, `parked_flag(conn, year) -> Result<Option<bool>, CliError>` (None if no row).

- [ ] **Step 1: Write the failing test** (in `input_form_store.rs` `#[cfg(test)]`)

```rust
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
```
(Imports: `use super::*; use rusqlite::Connection; use btctax_core::tax::return_inputs::ReturnInputs; use btctax_core::tax::types::FilingStatus;`)

- [ ] **Step 2: Run to verify it fails** — `cargo test -p btctax-cli --lib input_form_store::` → FAIL (module/fns missing).

- [ ] **Step 3: Implement the table + low-level I/O.** Mirror `return_inputs.rs` exactly:

```rust
//! Per-year `return_inputs_draft(year, inputs_json, schema_version, parked)` side-table — the input form's
//! crash-recovery scratch, INVISIBLE to `resolve.rs`. `parked = 1` marks a parked committed return (C-1).
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

pub(crate) struct DraftRow { pub ri: ReturnInputs, pub version: i64, pub parked: bool }

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
            let ri: ReturnInputs = serde_json::from_str(&json)?;
            Ok(Some(DraftRow { ri, version, parked: parked != 0 }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub(crate) fn set_draft_row(conn: &Connection, year: i32, ri: &ReturnInputs, parked: bool) -> Result<(), CliError> {
    init_draft_table(conn)?;
    let j = serde_json::to_string(ri)?;
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
    Ok(conn.query_row("SELECT 1 FROM return_inputs_draft WHERE year=?1", [year], |_| Ok(())).is_ok())
}

pub(crate) fn parked_flag(conn: &Connection, year: i32) -> Result<Option<bool>, CliError> {
    init_draft_table(conn)?;
    match conn.query_row("SELECT parked FROM return_inputs_draft WHERE year=?1", [year], |r| r.get::<_, i64>(0)) {
        Ok(p) => Ok(Some(p != 0)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
```
Add `pub mod input_form_store;` to `lib.rs` (near `pub mod return_inputs;`). Note: `serde_json` and `CliError`'s `#[from] rusqlite::Error` / serde handling already exist (mirror how `return_inputs.rs` maps a bad blob — check its exact `serde_json` error mapping and match it).

- [ ] **Step 4: Run to verify it passes** — `cargo test -p btctax-cli --lib input_form_store::draft_row_set_get_delete_roundtrip_with_parked` → PASS. Then `make check` → all green.

- [ ] **Step 5: Commit** — `git commit -m "feat(input-form persistence): return_inputs_draft table + low-level DraftRow I/O (plan 2 task 1)"`

---

### Task 2: `save_draft` — read-modify-write preserving `parked`, reaching disk (I-7)

**Files:**
- Modify: `crates/btctax-cli/src/input_form_store.rs`

**Interfaces:**
- Consumes: `crate::Session` (`sess.conn()`, `sess.save()`), Task-1 fns.
- Produces: `pub fn save_draft(sess: &mut Session, year: i32, ri: &ReturnInputs) -> Result<(), CliError>` — reads the existing row's `parked` flag (default `false` if absent), upserts with that flag PRESERVED (NI-1), then `sess.save()` (reaches disk).

- [ ] **Step 1: Write the failing tests.** Use an on-disk temp vault so the disk round-trip is real (mirror how `return_inputs.rs` tests build a `Session` — grep its tests for the `Session::create`/temp-path helper and reuse it).

```rust
#[test]
fn save_draft_preserves_parked_and_reaches_disk() {
    let (path, pp) = tmp_vault();                 // helper: creates an empty vault, returns (path, passphrase)
    let ri_a = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
    {
        let mut sess = Session::open(&path, &pp).unwrap();
        // seed a PARKED draft directly, then save_draft an edit — parked must survive
        set_draft_row(sess.conn(), 2024, &ri_a, true).unwrap();
        let ri_b = ReturnInputs { filing_status: FilingStatus::Mfj, ..Default::default() };
        save_draft(&mut sess, 2024, &ri_b).unwrap();
        assert_eq!(parked_flag(sess.conn(), 2024).unwrap(), Some(true), "NI-1: parked survives an edit");
    }
    // reopen a fresh Session — proves save_draft reached disk (I-7), not just the in-memory conn
    let sess2 = Session::open(&path, &pp).unwrap();
    let row = get_draft_row(sess2.conn(), 2024).unwrap().unwrap();
    assert_eq!(row.ri.filing_status, FilingStatus::Mfj);
    assert!(row.parked);
}

#[test]
fn save_draft_on_fresh_year_is_unparked() {
    let (path, pp) = tmp_vault();
    let mut sess = Session::open(&path, &pp).unwrap();
    save_draft(&mut sess, 2024, &ReturnInputs::default()).unwrap();
    assert_eq!(parked_flag(sess.conn(), 2024).unwrap(), Some(false));
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test -p btctax-cli --lib input_form_store::save_draft_` → FAIL.

- [ ] **Step 3: Implement.**

```rust
pub fn save_draft(sess: &mut Session, year: i32, ri: &ReturnInputs) -> Result<(), CliError> {
    let parked = parked_flag(sess.conn(), year)?.unwrap_or(false);   // ★ NI-1: preserve; default WIP
    set_draft_row(sess.conn(), year, ri, parked)?;
    sess.save()?;                                                    // ★ I-7: reach disk
    Ok(())
}
```

- [ ] **Step 4: Run green** — the two tests pass; `make check` green.

- [ ] **Step 5: Mutation-check the NI-1 guard** — `cp input_form_store.rs input_form_store.rs.bak`; change `parked_flag(...).unwrap_or(false)` to a hard `false`; `cargo test -p btctax-cli --lib input_form_store::save_draft_preserves_parked` → the test FAILS; `mv input_form_store.rs.bak input_form_store.rs; touch input_form_store.rs`.

- [ ] **Step 6: Commit** — `git commit -m "feat(input-form persistence): save_draft preserves parked + reaches disk (plan 2 task 2)"`

---

### Task 3: `load` — Draft{parked} ⇒ Committed ⇒ Fresh, with the §6.3 stale split

**Files:**
- Modify: `crates/btctax-cli/src/input_form_store.rs`
- Modify: `crates/btctax-cli/src/lib.rs` (add `CliError::StaleParkedDraft { year, found, expected }`)

**Interfaces:**
- Consumes: Task-1 fns, `crate::return_inputs::{get, SCHEMA_VERSION}`.
- Produces: `pub enum Loaded { Draft { ri: ReturnInputs, parked: bool }, Committed(ReturnInputs), Fresh }` and `pub fn load(conn: &Connection, year: i32) -> Result<Loaded, CliError>`.

**Precedence + stale rule (§6.1/§6.3):** a draft row takes precedence over the committed row. If the draft's `version != SCHEMA_VERSION`: `parked = 0` (WIP) → **DISCARD** (delete the stale draft, warn, fall through to committed/Fresh); `parked = 1` → **REFUSE** `Err(StaleParkedDraft{..})` (it may hold irreplaceable carryover — C-1). A version-current draft → `Draft{ri, parked}`. No draft → `return_inputs::get` → `Committed(ri)`; else `Fresh`.

- [ ] **Step 1: Write the failing tests.**

```rust
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
```

- [ ] **Step 2: Run to verify it fails** — FAIL (`load`/`Loaded`/`StaleParkedDraft` missing).

- [ ] **Step 3: Implement.** Add to `lib.rs` after `StaleReturnInputs`:
```rust
#[error("year {year}'s parked full return is schema v{found} but this build expects v{expected}; \
         an upgrade changed the input format. Its data lives only in the draft — do not discard it. \
         Re-run on the app version that wrote it, or export it there first.")]
StaleParkedDraft { year: i32, found: i64, expected: i64 },
```
Then in `input_form_store.rs`:
```rust
pub enum Loaded { Draft { ri: ReturnInputs, parked: bool }, Committed(ReturnInputs), Fresh }

pub fn load(conn: &Connection, year: i32) -> Result<Loaded, CliError> {
    if let Some(d) = get_draft_row(conn, year)? {
        if d.version != SCHEMA_VERSION {
            if d.parked {
                return Err(CliError::StaleParkedDraft { year, found: d.version, expected: SCHEMA_VERSION });
            }
            // ★ §6.3: a stale WIP draft is regenerable — discard-with-note, fall through.
            eprintln!("note: discarded a stale draft for {year} (schema v{} vs v{SCHEMA_VERSION}).", d.version);
            delete_draft(conn, year)?;
        } else {
            return Ok(Loaded::Draft { ri: d.ri, parked: d.parked });
        }
    }
    match crate::return_inputs::get(conn, year)? {
        Some(ri) => Ok(Loaded::Committed(ri)),
        None => Ok(Loaded::Fresh),
    }
}
```

- [ ] **Step 4: Run green**; `make check` green.

- [ ] **Step 5: Mutation-check the parked-vs-WIP split** — change `if d.parked {` to `if false {` (so a stale parked draft would be discarded instead of refused) → `load_discards_stale_wip_but_refuses_stale_parked` FAILS on the `StaleParkedDraft` assertion; restore via cp-backup + touch.

- [ ] **Step 6: Commit** — `git commit -m "feat(input-form persistence): load precedence + §6.3 stale split (plan 2 task 3)"`

---

### Task 4: `commit` — screen → set → delete-draft; `NoTables` for non-2024; refused writes nothing

**Files:**
- Modify: `crates/btctax-cli/src/input_form_store.rs`

**Interfaces:**
- Consumes: `btctax_core::tax::return_refuse::{screen_inputs, Refusal}`, `btctax_core::tax::tables::{TaxTable, FullReturnParams}`, `crate::return_inputs`, Task-1 fns, `Session` (`conn`, `snapshot`, `restore`, `save`).
- Produces: `pub enum CommitOutcome { Committed, Refused(Refusal), NoTables }` and `pub fn commit(sess: &mut Session, year: i32, ri: &ReturnInputs, table: Option<&TaxTable>, params: Option<&FullReturnParams>) -> Result<CommitOutcome, CliError>`.

**Behavior:** if `table` or `params` is `None` → `NoTables` (write nothing — I-11). Else `screen_inputs(ri, table, params)`: `Some(refusal)` → `Refused(refusal)` (write nothing); `None` → snapshot, `return_inputs::set(conn, year, ri)`, `delete_draft(conn, year)`, `sess.save()` (restore snapshot on save failure) → `Committed`.

- [ ] **Step 1: Write the failing tests.** For the params, load the real bundled tables (grep `BundledTaxTables`/`BundledFullReturnTables` usage in `resolve.rs`/tests for the exact load + lookup call — e.g. `BundledTaxTables::load()?.table_for(2024)` and `BundledFullReturnTables::load()?.full_return_for(2024)`).

```rust
#[test]
fn commit_non2024_is_notables_and_writes_nothing() {
    let (path, pp) = tmp_vault();
    let mut sess = Session::open(&path, &pp).unwrap();
    let ri = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
    set_draft_row(sess.conn(), 2099, &ri, false).unwrap();
    // no full-return params for 2099 → NoTables
    let out = commit(&mut sess, 2099, &ri, None, None).unwrap();
    assert!(matches!(out, CommitOutcome::NoTables));
    assert!(!crate::return_inputs::exists(sess.conn(), 2099).unwrap(), "NoTables writes no committed row");
    assert!(draft_exists(sess.conn(), 2099).unwrap(), "NoTables leaves the draft");
}

#[test]
fn commit_clean_sets_row_and_deletes_draft_refused_writes_nothing() {
    let (path, pp) = tmp_vault();
    let tables = BundledTaxTables::load().unwrap();
    let fr = BundledFullReturnTables::load().unwrap();
    let (t, p) = (tables.table_for(2024).unwrap(), fr.full_return_for(2024).unwrap());
    let mut sess = Session::open(&path, &pp).unwrap();
    // A screen-clean minimal return: choose the fixture that `every_live_question_can_actually_be_answered`
    // uses (all 8 declarations answered, no income) — build it the same way that test does. Call it `clean`.
    let clean = clean_screened_ri();          // helper mirroring the no-brick fixture
    set_draft_row(sess.conn(), 2024, &clean, false).unwrap();
    assert!(matches!(commit(&mut sess, 2024, &clean, Some(t), Some(p)).unwrap(), CommitOutcome::Committed));
    assert!(crate::return_inputs::exists(sess.conn(), 2024).unwrap(), "clean commit writes the row");
    assert!(!draft_exists(sess.conn(), 2024).unwrap(), "clean commit deletes the draft");
    // A refused return (an unanswered declaration) writes nothing
    let refused = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() }; // 5 live None decls
    set_draft_row(sess.conn(), 2024, &refused, false).unwrap();
    assert!(matches!(commit(&mut sess, 2024, &refused, Some(t), Some(p)).unwrap(), CommitOutcome::Refused(_)));
    // the committed 2024 row is still the earlier `clean` one; the refused draft remains
    assert!(draft_exists(sess.conn(), 2024).unwrap(), "a refused commit leaves the draft");
}
```
(If constructing `clean_screened_ri()` is non-trivial, port the exact fixture from `answer.rs`'s `every_live_question_can_actually_be_answered_and_clears_the_screen` test — it is the canonical screen-clean return.)

- [ ] **Step 2: Run to verify it fails** — FAIL.

- [ ] **Step 3: Implement.**

```rust
use btctax_core::tax::return_refuse::{screen_inputs, Refusal};
use btctax_core::tax::tables::{FullReturnParams, TaxTable};

pub enum CommitOutcome { Committed, Refused(Refusal), NoTables }

pub fn commit(sess: &mut Session, year: i32, ri: &ReturnInputs,
              table: Option<&TaxTable>, params: Option<&FullReturnParams>) -> Result<CommitOutcome, CliError> {
    let (Some(table), Some(params)) = (table, params) else { return Ok(CommitOutcome::NoTables) }; // I-11
    if let Some(refusal) = screen_inputs(ri, table, params) {
        return Ok(CommitOutcome::Refused(refusal));                  // writes nothing
    }
    let snap = sess.snapshot()?;
    crate::return_inputs::set(sess.conn(), year, ri)?;
    delete_draft(sess.conn(), year)?;
    if let Err(e) = sess.save() { sess.restore(&snap)?; return Err(e); }   // atomic: no in-mem/disk split
    Ok(CommitOutcome::Committed)
}
```

- [ ] **Step 4: Run green**; `make check` green.

- [ ] **Step 5: Mutation-check** — (a) change the `Refused` early-return to fall through → the refused-writes-nothing assertion fails; restore. (b) delete the `delete_draft` line → the clean-commit-deletes-draft assertion fails; restore. Both via cp-backup + touch.

- [ ] **Step 6: Commit** — `git commit -m "feat(input-form persistence): commit — screen/set/delete-draft, NoTables (plan 2 task 4)"`

---

### Task 5: The §6.2 coherence rule — helper + wire into the four committed-row writers

**Files:**
- Modify: `crates/btctax-cli/src/input_form_store.rs` (the helper)
- Modify: `crates/btctax-cli/src/lib.rs` (`CliError::ParkedDraftBlocksWrite { year }`)
- Modify: `crates/btctax-cli/src/cmd/tax.rs` (`import_return_inputs`:98, `write_back_carryover`:461, `clear_return_inputs`:165)
- Modify: `crates/btctax-cli/src/cmd/answer.rs` (`answer_return_inputs`:205)

**Interfaces:**
- Produces: `pub fn coherence_clear_or_refuse(conn: &Connection, year: i32) -> Result<(), CliError>` — no draft → `Ok`; `parked = 0` WIP → `delete_draft` (warn if it deserializes to a non-trivial return); `parked = 1` → `Err(ParkedDraftBlocksWrite { year })`.

**RULE (§6.2):** an authoritative committed-row write CLEARS that year's WIP draft but REFUSES a parked one. The four writers call `coherence_clear_or_refuse` **immediately BEFORE** their `return_inputs::set`/`delete` (fail-fast: a parked-draft `Err` aborts the command before any committed-row mutation — no wasted in-memory write to discard), and before the shared `s.save()`. A WIP draft is deleted in the same `conn`, so the subsequent `set` + `s.save()` persist both changes together. `write_back_carryover` writes `year + 1`, so it checks coherence on `year + 1`.

- [ ] **Step 1: Write the failing tests** (in `input_form_store.rs`).

```rust
#[test]
fn coherence_clears_wip_but_refuses_parked() {
    let conn = Connection::open_in_memory().unwrap();
    init_draft_table(&conn).unwrap();
    let ri = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
    // WIP draft → cleared
    set_draft_row(&conn, 2024, &ri, false).unwrap();
    coherence_clear_or_refuse(&conn, 2024).unwrap();
    assert!(!draft_exists(&conn, 2024).unwrap(), "coherence clears a WIP draft");
    // parked draft → refused, preserved, message names both exits
    set_draft_row(&conn, 2025, &ri, true).unwrap();
    let err = coherence_clear_or_refuse(&conn, 2025).unwrap_err();
    assert!(matches!(err, CliError::ParkedDraftBlocksWrite { year: 2025 }));
    let msg = err.to_string();
    assert!(msg.contains("use full return") && msg.contains("discard parked draft"), "M-d: names both exits");
    assert!(draft_exists(&conn, 2025).unwrap(), "a parked draft is never silently destroyed");
    // no draft → Ok
    coherence_clear_or_refuse(&conn, 2030).unwrap();
}
```
Also add ONE integration test that `income import` refuses on a parked draft (build a Session, seed a `parked=1` draft for 2024 via `set_draft_row`, run the import path, assert `Err(ParkedDraftBlocksWrite)` and the committed row was NOT written). If wiring an end-to-end `income import` in a unit test is heavy, instead assert the writer calls the helper by placing the helper call and adding a focused test on `import_return_inputs`'s public entry with a seeded parked draft — pick whichever the existing `tax.rs` tests support.

- [ ] **Step 2: Run to verify it fails** — FAIL.

- [ ] **Step 3: Implement the helper + variant, then wire the four sites.**
```rust
// lib.rs, after StaleReturnInputs:
#[error("year {year} holds a parked full return — in the form, 'use full return' to re-commit it, or \
         'discard parked draft' (a confirmed delete) to drop it; then re-run this command.")]
ParkedDraftBlocksWrite { year: i32 },
```
```rust
// input_form_store.rs:
pub fn coherence_clear_or_refuse(conn: &Connection, year: i32) -> Result<(), CliError> {
    match parked_flag(conn, year)? {
        None => Ok(()),
        Some(true) => Err(CliError::ParkedDraftBlocksWrite { year }),   // ★ C-1
        Some(false) => {
            if let Some(d) = get_draft_row(conn, year)? {               // warn on discarding a non-trivial WIP
                if d.ri != ReturnInputs::default() {
                    eprintln!("note: superseding a work-in-progress draft for {year} with this write.");
                }
            }
            delete_draft(conn, year)?;
            Ok(())
        }
    }
}
```
Wire (each: insert the call **immediately before** the existing `return_inputs::set`/`delete`; grep for the write site rather than trusting a line number, since plan 1 shifted `answer.rs`):
- `tax.rs` `import_return_inputs` (before `return_inputs::set(s.conn(), year, &ri)?;`): `crate::input_form_store::coherence_clear_or_refuse(s.conn(), year)?;`
- `answer.rs` `answer_return_inputs` (before its `return_inputs::set(...)?;`): same, on `year`.
- `tax.rs` `write_back_carryover` (before `return_inputs::set(s.conn(), year + 1, &updated)?;`): `crate::input_form_store::coherence_clear_or_refuse(s.conn(), year + 1)?;`
- `tax.rs` `clear_return_inputs` (before `return_inputs::delete(s.conn(), year)?;`): same, on `year`.

Note: `ReturnInputs` must be `PartialEq` for the `!= default` warn (it is — used across the codebase; verify).

- [ ] **Step 4: Run green**; `make check` green (the four writers still pass their existing tests, now also clearing/refusing drafts).

- [ ] **Step 5: Mutation-check** — change `Some(true) => Err(...)` to `Some(true) => Ok(())` → `coherence_clears_wip_but_refuses_parked` FAILS on the parked assertion; restore. Then confirm at least one writer's wiring is load-bearing: remove the `coherence_clear_or_refuse` call from `import_return_inputs` → the import-refuses-on-parked integration test fails; restore.

- [ ] **Step 6: Commit** — `git commit -m "feat(input-form persistence): §6.2 draft-coherence rule across the committed-row writers (plan 2 task 5)"`

---

### Task 6: `park_to_profile` — atomic stash → in-session delete (C-1, N-1) + clean-state gate

**Files:**
- Modify: `crates/btctax-cli/src/input_form_store.rs`

**Interfaces:**
- Produces: `pub fn park_to_profile(sess: &mut Session, year: i32) -> Result<(), CliError>` — stashes the committed row into its draft with `parked = 1`, THEN deletes the committed row (in-session `return_inputs::delete`, NOT `income clear` — N-1), atomically (snapshot/restore). Refuses (writes nothing) if there is no committed row, or if a divergent WIP draft already occupies the slot (clean-state gate, §9).

**Behavior:** (1) `return_inputs::get(conn, year)` — `None` → `Err(Usage("no committed return to park"))`. (2) clean-state gate: if a `parked = 0` draft exists → `Err(Usage("finish or discard the work-in-progress draft first"))` (parking would clobber it). (3) `snapshot`; `set_draft_row(conn, year, &ri, parked=true)`; `return_inputs::delete(conn, year)`; `sess.save()` (restore on failure).

- [ ] **Step 1: Write the failing tests.**
```rust
#[test]
fn park_stashes_then_deletes_committed_atomically() {
    let (path, pp) = tmp_vault();
    let mut sess = Session::open(&path, &pp).unwrap();
    let ri = ReturnInputs { filing_status: FilingStatus::Mfj, ..Default::default() };
    crate::return_inputs::set(sess.conn(), 2024, &ri).unwrap();
    park_to_profile(&mut sess, 2024).unwrap();
    // committed row gone; draft holds it with parked=1
    assert!(!crate::return_inputs::exists(sess.conn(), 2024).unwrap(), "park deletes the committed row");
    let d = get_draft_row(sess.conn(), 2024).unwrap().unwrap();
    assert!(d.parked && d.ri.filing_status == FilingStatus::Mfj, "park stashes the row as parked");
    // survives disk (I-7)
    drop(sess);
    let s2 = Session::open(&path, &pp).unwrap();
    assert!(get_draft_row(s2.conn(), 2024).unwrap().unwrap().parked);
}

#[test]
fn park_refuses_without_committed_row_and_on_divergent_wip() {
    let (path, pp) = tmp_vault();
    let mut sess = Session::open(&path, &pp).unwrap();
    assert!(park_to_profile(&mut sess, 2024).is_err(), "nothing to park");
    let ri = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
    crate::return_inputs::set(sess.conn(), 2024, &ri).unwrap();
    set_draft_row(sess.conn(), 2024, &ri, false).unwrap();  // a WIP draft occupies the slot
    assert!(park_to_profile(&mut sess, 2024).is_err(), "clean-state gate: won't clobber a WIP draft");
    assert!(crate::return_inputs::exists(sess.conn(), 2024).unwrap(), "a refused park leaves the committed row");
}
```

- [ ] **Step 2: Run to verify it fails** — FAIL.

- [ ] **Step 3: Implement.**
```rust
pub fn park_to_profile(sess: &mut Session, year: i32) -> Result<(), CliError> {
    let Some(ri) = crate::return_inputs::get(sess.conn(), year)? else {
        return Err(CliError::Usage(format!("no committed return to park for {year}")));
    };
    if parked_flag(sess.conn(), year)? == Some(false) {                 // clean-state gate (§9)
        return Err(CliError::Usage(format!(
            "year {year} has a work-in-progress draft; finish or discard it before switching to the tax-profile")));
    }
    let snap = sess.snapshot()?;
    set_draft_row(sess.conn(), year, &ri, true)?;                       // stash FIRST (parked=1)
    crate::return_inputs::delete(sess.conn(), year)?;                   // ★ N-1: in-session delete, not `income clear`
    if let Err(e) = sess.save() { sess.restore(&snap)?; return Err(e); } // ★ atomic: a failed stash never loses the row
    Ok(())
}
```

- [ ] **Step 4: Run green**; `make check` green.

- [ ] **Step 5: Mutation-check** — reorder to delete-before-stash (swap the two lines) is NOT a good mutation (still passes on success); instead, mutation-check the clean-state gate: change `== Some(false)` to `== Some(true)` → `park_refuses_...on_divergent_wip` FAILS; restore. And delete the clean-state gate block entirely → same test fails; restore.

- [ ] **Step 6: Commit** — `git commit -m "feat(input-form persistence): park_to_profile — atomic stash→delete + clean-state gate (plan 2 task 6)"`

---

### Task 7: Toggle info + `discard_parked_draft` (the sole parked-row deleter)

**Files:**
- Modify: `crates/btctax-cli/src/input_form_store.rs`

**Interfaces:**
- Produces:
  - `pub enum ActiveSource { FullReturn, TaxProfile, Neither }` + `pub fn active_source(conn: &Connection, year: i32) -> Result<ActiveSource, CliError>` — committed `return_inputs` present → `FullReturn`; else `tax_profile` present → `TaxProfile`; else `Neither` (mirrors `resolve.rs` precedence; the form's active-source display).
  - `pub fn shadows_profile(conn: &Connection, year: i32) -> Result<bool, CliError>` — `tax_profile::exists(conn, year)` (the TUI warns on commit when this is true — §9 create-row amendment).
  - `pub fn discard_parked_draft(sess: &mut Session, year: i32) -> Result<(), CliError>` — the 'X' path (§9A/M-2): the ONLY deleter of a `parked = 1` row (a confirmed delete; the confirm modal is the TUI's). Refuses if the year's draft is not parked (so it can never delete a WIP behind a "discard parked" affordance). Deletes + `sess.save()`.

- [ ] **Step 1: Write the failing tests.**
```rust
#[test]
fn active_source_follows_resolve_precedence() {
    let conn = Connection::open_in_memory().unwrap();
    crate::return_inputs::init_table(&conn).unwrap();
    crate::tax_profile::init_table(&conn).unwrap();
    assert!(matches!(active_source(&conn, 2024).unwrap(), ActiveSource::Neither));
    // (build a TaxProfile the way tax_profile tests do) → TaxProfile
    crate::tax_profile::set(&conn, 2024, &sample_profile()).unwrap();
    assert!(matches!(active_source(&conn, 2024).unwrap(), ActiveSource::TaxProfile));
    assert!(shadows_profile(&conn, 2024).unwrap());
    // committed return_inputs wins
    crate::return_inputs::set(&conn, 2024, &ReturnInputs::default()).unwrap();
    assert!(matches!(active_source(&conn, 2024).unwrap(), ActiveSource::FullReturn));
}

#[test]
fn discard_parked_draft_only_deletes_a_parked_row() {
    let (path, pp) = tmp_vault();
    let mut sess = Session::open(&path, &pp).unwrap();
    let ri = ReturnInputs { filing_status: FilingStatus::Single, ..Default::default() };
    // a WIP draft is NOT discardable via this path
    set_draft_row(sess.conn(), 2024, &ri, false).unwrap();
    assert!(discard_parked_draft(&mut sess, 2024).is_err(), "won't delete a WIP behind 'discard parked'");
    assert!(draft_exists(sess.conn(), 2024).unwrap());
    // a parked draft IS discardable
    set_draft_row(sess.conn(), 2024, &ri, true).unwrap();
    discard_parked_draft(&mut sess, 2024).unwrap();
    assert!(!draft_exists(sess.conn(), 2024).unwrap());
}
```

- [ ] **Step 2: Run to verify it fails** — FAIL.

- [ ] **Step 3: Implement.**
```rust
pub enum ActiveSource { FullReturn, TaxProfile, Neither }

pub fn active_source(conn: &Connection, year: i32) -> Result<ActiveSource, CliError> {
    if crate::return_inputs::exists(conn, year)? { return Ok(ActiveSource::FullReturn); }
    if crate::tax_profile::years(conn)?.contains(&year) { return Ok(ActiveSource::TaxProfile); }
    Ok(ActiveSource::Neither)
}

pub fn shadows_profile(conn: &Connection, year: i32) -> Result<bool, CliError> {
    Ok(crate::tax_profile::years(conn)?.contains(&year))
}

pub fn discard_parked_draft(sess: &mut Session, year: i32) -> Result<(), CliError> {
    if parked_flag(sess.conn(), year)? != Some(true) {              // never delete a WIP behind this affordance
        return Err(CliError::Usage(format!("year {year} has no parked draft to discard")));
    }
    let snap = sess.snapshot()?;
    delete_draft(sess.conn(), year)?;
    if let Err(e) = sess.save() { sess.restore(&snap)?; return Err(e); }
    Ok(())
}
```
(If `tax_profile` has an `exists` fn, use it instead of `years().contains`; check `tax_profile.rs` and prefer the cheapest probe.)

- [ ] **Step 4: Run green**; `make check` green.

- [ ] **Step 5: Mutation-check** — change `!= Some(true)` to `== Some(true)` (invert the parked gate) → `discard_parked_draft_only_deletes_a_parked_row` FAILS; restore.

- [ ] **Step 6: Commit** — `git commit -m "feat(input-form persistence): toggle info (active_source/shadows_profile) + discard_parked_draft (plan 2 task 7)"`

---

## Self-Review notes (controller)

- **Spec coverage:** §6.1 draft table (T1) · §6.1 commit gate + NoTables (T4) · §6.2 coherence rule + 4 writers (T5) · §6.3 stale split (T3) · §5.7 `load`/`save_draft`/`commit`/`park_to_profile` signatures (T2/T3/T4/T6) · NI-1 parked round-trip (T2/T3) · §9 toggle: park (T6), active-source + re-commit-via-`commit` + discard (T7). §9 re-commit ("use full return") needs no new fn — it is `load` the parked draft → `commit` (which deletes the draft, dropping `parked`); the TUI (plan 3) wires the key.
- **Deferred to plan 3 (TUI):** the payload-confirm modals, the shadow/all-zero warning copy, the key bindings, the active-source display, secret no-echo. Plan 2 provides the store primitives + the info fns those need.
- **`resolve.rs` untouched** — verified the draft table is referenced only in `input_form_store` + the 4 writers' coherence call; `resolve_core` never sees it.
- **Type consistency:** `Loaded`, `CommitOutcome`, `ActiveSource`, `save_draft`/`commit`/`park_to_profile`/`load`/`discard_parked_draft` signatures match §5.7 and are the exact names plan 3 will consume.
