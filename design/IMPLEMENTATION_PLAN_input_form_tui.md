# Input-Form TUI (plan 3 of 4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "tax inputs" editing mode to `btctax-tui-edit` — a renderer over the `btctax-input-form` `FormSpec`/`Edit` engine (plan 1) that drives the `input_form_store` (plan 2), so a technical user creates and edits a full v1-subset `ReturnInputs` return interactively (live per-field validation, autosave, screen-clean commit, the non-destructive tax-profile toggle) without hand-editing TOML. Spec §9A.

**Architecture:** A new flow on `EditorApp`, mirroring the existing tax-profile form flow (`p` key) exactly: one `Option<TaxInputsFormState>` field, one dispatch line in `handle_key`, an `open_tax_inputs_form` opener, a `draw_tax_inputs_form` renderer, key handlers, and a payload-confirm modal. The flow holds a `Working = Option<ReturnInputs>` (the engine's type; `None` until a filing status is chosen — NI-2), a `FieldBuffer` for the field being edited, and the current `(section_index, RowAddr)`. It **never names a `ReturnInputs` field** — all field access goes through `form_spec()` accessors. Persistence is `input_form_store::{load, save_draft, commit, park_to_profile, discard_parked_draft, active_source, shadows_profile}` on the held `Session`.

**Tech Stack:** Rust, `ratatui` 0.29 (the crate's only render layer; `TestBackend` for snapshot tests), `crossterm` key events, the `btctax-input-form` engine, the `btctax-cli` `input_form_store`.

## Global Constraints

- **Add the engine dependency:** `btctax-tui-edit/Cargo.toml` does NOT yet depend on `btctax-input-form`. Add `btctax-input-form = { path = "../btctax-input-form", version = "0.5.0" }` (a one-line, no-cycle workspace addition). It becomes reachable as `btctax_input_form::{form_spec, apply, Working, parse, parse_ssn, parse_ip_pin, attribute, Field, Section, SectionKind, FieldKind, FieldValue, FieldId, SectionId, RowAddr, Edit, Anchor, SecretView}`.
- **Mirror the tax-profile form flow END-TO-END** (the recon's template): state struct like `ProfileFormState` (`edit/form.rs:114`); `FieldBuffer` (`form.rs:36`, pre-allocated, no realloc) for text entry; modal state carries the VALIDATED payload never raw buffers; render like `draw_profile_form` (`draw_edit.rs:470`) + `draw_mutation_modal` (`draw_edit.rs:552`) using `centered_rect` (`draw_edit.rs:1956`) + `Clear`; the `TargetList<T>` (`form.rs:239`) for any selectable list; routing via one `Option<TaxInputsFormState>` field + a dispatch early-return in `handle_key` (`main.rs:126`) BEFORE the Browse `match`, and an opener bound to a free Browse key.
- **The `EditorApp` invariant:** at most one flow `Some` and one modal `Some` at a time. The opener must not clobber an open flow; the dispatch chain checks modals before flows.
- **`EditorScreen`** stays `{ Unlock, Locked, Browse }` — the tax-inputs form is a flow reached from `Browse`, NOT a new screen. It reuses `app.selected_year` (changed via `[`/`]` in Browse; no year-picker exists).
- **NI-2 (from the engine, honored by the renderer):** on a `None` working copy present ONLY the filing-status choice; the first `SetField{FilingStatus, Choice}` materializes the return, then the rest of the sections appear. Never construct a `ReturnInputs` directly — go through `apply`.
- **Autosave reaches disk (I-7):** `save_draft(sess, year, ri)` calls `Vault::save`; it is expensive (re-encrypts the whole vault), so debounce it (on section-exit and short idle — NOT per keystroke). Only a materialized `Some(ReturnInputs)` writes a draft.
- **Secrets never surface digits:** `Secret` fields render masked (`***-**-1234` when set, a bullet count during no-echo entry). `get` returns `SecretView` (presence), `set` accepts only `SecretEntry` (built by `parse_ssn`/`parse_ip_pin`). The editor-side masker is NEW (no existing one); model it on `UnlockState` (`btctax-tui/unlock.rs:30`) + `draw_unlock_screen` (`btctax-tui/draw.rs:36`, `"●".repeat(...)`).
- **P2-a (from plan-2 review):** when `load` returns `Err(StaleParkedDraft)`, the mode MUST make the 'X' discard-parked affordance reachable (a confirmed delete via `discard_parked_draft`) — else a stale parked draft is undiscardable in-app.
- **Commit gate (plan 2):** `s` runs `input_form_store::commit(sess, year, &ri, table, params)`. Get `table`/`params` via `BundledTaxTables::load().table_for(year)` / `BundledFullReturnTables::load().full_return_for(year)` (both `Option`; `full_return_for` is `Some` only for 2024 — a non-2024 year yields `CommitOutcome::NoTables`).
- **FROZEN — never edit:** `crates/btctax-core/src/tax/{types,compute,se}.rs`. Do not modify the `btctax-input-form` engine or the `input_form_store` module — this plan CONSUMES them (if you find a genuine gap, STOP and report it, don't patch the engine here).
- **Gate per task:** `make check` (~7s; fast suite + clippy `-D warnings`). TDD: write the failing (snapshot or key-driven) test, watch it fail, implement, watch it pass, commit. **Mutation-check** each behavioral guard. Snapshot tests use `TestBackend` + buffer-flatten + `assert!(rendered.contains(...))` (recon §3; template `draw_edit.rs:5264`); key-driven tests use `press(code)` (`main.rs:9016`) + `type_str(app, s)` (`main.rs:9027`) + assertions on `app.tax_inputs_form`/`app.status`. Fish shell: quote globs; heredoc for `git commit -F -`.

## File Structure

- **Modify** `crates/btctax-tui-edit/Cargo.toml` — add the `btctax-input-form` dep.
- **Modify** `crates/btctax-tui-edit/src/edit/form.rs` — add `TaxInputsFormState` + `TaxInputsModalState` + `validate`/navigation helpers.
- **Modify** `crates/btctax-tui-edit/src/draw_edit.rs` — add `draw_tax_inputs_form` (3-region layout) + `draw_tax_inputs_modal` (payload-confirm).
- **Modify** `crates/btctax-tui-edit/src/main.rs` (or `editor.rs`) — the `Option<TaxInputsFormState>` field on `EditorApp`, the dispatch early-return, the opener, the key handlers, the commit/toggle/discard wiring.
- **Create (maybe)** `crates/btctax-tui-edit/src/edit/tax_inputs.rs` — if the flow's non-render logic (navigation, apply-dispatch, the secret masker) grows large, factor it here rather than bloating `main.rs`. Decide during Task 2.

Interfaces produced: a self-contained editor flow; no cross-crate API. Tasks build on each other's `TaxInputsFormState`.

---

### Task 1: Engine dep + `TaxInputsFormState` skeleton + opener (load → Working) + the Fresh filing-status-only screen

**Files:** Modify `Cargo.toml`, `edit/form.rs`, `main.rs`/`editor.rs`, `draw_edit.rs`.

**Interfaces:**
- Produces: `pub struct TaxInputsFormState { pub year: i32, pub working: btctax_input_form::Working, pub section_idx: usize, pub addr: btctax_input_form::RowAddr, pub buf: FieldBuffer, pub editing: bool, pub error: Option<String>, pub parked: bool, pub stale_note: Option<btctax_cli::input_form_store::StaleNote>, pub discard_offered: bool }` (fields grow over tasks; start minimal — `year`, `working`, `section_idx`, `error`, `stale_note`, `parked`, `discard_offered`), plus `pub tax_inputs_form: Option<TaxInputsFormState>` on `EditorApp`, `open_tax_inputs_form(app)`, and a dispatch line + Browse key binding.

- [ ] **Step 1: Add the dep, wire the field + a minimal opener + a render stub, and write a key-driven test that opening on a fresh year yields a `None` working copy.** First (RED needs it to compile): add the Cargo dep; add `pub tax_inputs_form: Option<TaxInputsFormState>` to `EditorApp` (init `None` in `EditorApp::new`); add the `TaxInputsFormState` struct (minimal fields); add a `handle_key` dispatch early-return `if app.tax_inputs_form.is_some() { handle_tax_inputs_key(app, key); return; }` placed with the other flow gates (before the Browse `match`, after modals); bind a FREE Browse key (grep the Browse `match key.code` at `main.rs:397-452` for an unused letter — e.g. `KeyCode::Char('T')` if free; if not, pick another and note it) to `open_tax_inputs_form(app)`; add a stub `handle_tax_inputs_key` (Esc closes the flow) and a stub `draw_tax_inputs_form`.

```rust
// key-driven test (main.rs tests)
#[test]
fn open_tax_inputs_on_fresh_year_has_none_working() {
    let (mut app, _dir) = unlocked_app_on_empty_vault(2024);   // helper: mirror kat_c1_... setup (main.rs:10057)
    handle_key(&mut app, press(KeyCode::Char('T')));           // the chosen open key
    let f = app.tax_inputs_form.as_ref().expect("form opened");
    assert!(f.working.is_none(), "NI-2: a fresh year starts with no ReturnInputs until filing status is chosen");
    assert!(f.stale_note.is_none());
}
```
(Build `unlocked_app_on_empty_vault` by extracting the unlock-on-tempdir-vault setup the existing `kat_c1_cancel_path_vault_bytes_unchanged` test uses — `main.rs:10057` — into a small helper, or inline it.)

- [ ] **Step 2: Run to verify it fails** — `cargo test -p btctax-tui-edit open_tax_inputs_on_fresh_year_has_none_working` → FAIL (form doesn't open / field missing).

- [ ] **Step 3: Implement `open_tax_inputs_form`** — reuse `app.selected_year`. Call `input_form_store::load(app.session().conn(), year)`:
  - `Ok((Loaded::Fresh, note))` → `working = None`, `parked = false`.
  - `Ok((Loaded::Committed(ri), note))` → `working = Some(ri)`, `parked = false`.
  - `Ok((Loaded::Draft{ri, parked}, note))` → `working = Some(ri)`, `parked`.
  - carry `stale_note = note` (Task 2 renders it in the status line).
  - `Err(CliError::StaleParkedDraft{..})` → **P2-a:** do NOT open a normal editing form; open the flow in a `discard_offered = true` state that presents ONLY the stale-parked message + an 'X' to `discard_parked_draft` and Esc to back out (Task 8 wires the actual discard; here just set the flag + store the error text in `error`). This makes the stale parked draft discardable in-app.
  - other `Err(e)` → set `app.status` to the error, do NOT open the flow.
  Set `app.tax_inputs_form = Some(TaxInputsFormState{ year, working, section_idx: 0, error: None, stale_note, parked, discard_offered, ..})`. Guard with the `residue_latch_status()` check the other openers use (`main.rs:702` pattern) if applicable.

- [ ] **Step 4: Run green** (`cargo test -p btctax-tui-edit open_tax_inputs_on_fresh_year_has_none_working`), then `make check` green.

- [ ] **Step 5: Commit** — `git commit -m "feat(input-form tui): tax-inputs flow skeleton + opener (load->Working) (plan 3 task 1)"`

---

### Task 2: The 3-region render (section list · field pane · status line) + live-section recompute + section/field navigation

**Files:** Modify `draw_edit.rs`, `main.rs`, `edit/form.rs`.

**Interfaces:** Consumes `form_spec()`, `TaxInputsFormState`. Produces `draw_tax_inputs_form(frame, area, form)` (3 regions) + navigation in `handle_tax_inputs_key` (↑/↓ field focus, ←/→ or Tab section) + `live_sections(form)`/`live_fields(section, ri)` helpers.

- [ ] **Step 1: Write the failing snapshot tests** (mirror `draw_edit.rs:5264` `TestBackend` pattern): (a) on a `None` working copy the render shows ONLY the filing-status choice (assert the buffer contains "Filing status" / "Single" and does NOT contain a W-2 section title); (b) on a materialized `Single` return the left pane lists the live sections in §9A order (`ReturnOptions → Taxpayer → Address → Dependents → W-2s → Schedule A? → Payments → Declarations → Skippables`; Spouse hidden on Single), the right pane shows the selected section's live fields as `label  [value]`, and the bottom shows the active source + a key legend.

```rust
#[test]
fn tax_inputs_renders_only_filing_status_when_fresh() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let form = TaxInputsFormState::fresh(2024);                 // working = None
    let area = terminal.get_frame().area();
    terminal.draw(|f| draw_tax_inputs_form(f, area, &form)).unwrap();
    let r = flatten(terminal.backend().buffer());               // the recon's buffer-flatten helper
    assert!(r.contains("Filing status"));
    assert!(!r.contains("W-2"), "no other section is offered until filing status is chosen (NI-2)");
}
```

- [ ] **Step 2: Run fail. Step 3: implement `draw_tax_inputs_form`.** Split `area` into left (section list), right (field pane), bottom (status) with a `Layout` (mirror the profile-form `Layout`/`centered_rect` usage). **Live sections:** iterate `form_spec()`, keep those whose `Section` is live given `form.working` — a `Singleton` is always shown; an `OptionalSingleton` shows if `present(ri)` OR is offerable; a `Repeating` always shows (with its rows); when `working` is `None`, show ONLY `ReturnOptions`'s `FilingStatus` field. Compute per-section status glyph: `✓` all live fields set / `…` incomplete / `!` a screen refusal attributes here (Task 7 wires `!`; here `✓`/`…`). **Field pane:** for the selected section, iterate its live `Field`s (`field.live(ri)`), render `label  [value]  ‹error›` where value = the `get` accessor's `FieldValue` rendered per kind (Money → `$x`, TriState → `yes/no/—`, Enum → the choice, Secret → the `SecretView` masked/`(unset)`, Date → `YYYY-MM-DD`/`—`), focus-styled like the profile form. **Status line:** `active source: full return|tax-profile` (Task 8) + the screen status + key legend; if `form.stale_note.is_some()`, show its `Display` (e.g. "discarded a stale draft…"). Navigation in `handle_tax_inputs_key`: `Up/Down` move field focus within the section; `Left/Right`/`Tab` move `section_idx` across live sections (clamp); recompute live sections each keypress (they change after edits).

- [ ] **Step 4: Run green; `make check` green.**
- [ ] **Step 5: Mutation-check** the NI-2 render gate: make the render show all sections even when `working.is_none()` → `tax_inputs_renders_only_filing_status_when_fresh` fails on the `!contains("W-2")` assertion; restore via cp-backup + touch.
- [ ] **Step 6: Commit** — `feat(input-form tui): 3-region render + live-section nav (plan 3 task 2)`

---

### Task 3: Field editing — the edit buffer, `parse`, and `apply` per kind (Money/Text/TriState/Enum/Bool/Date), incl. the filing-status materialization

**Files:** Modify `main.rs`/`edit/tax_inputs.rs`, `edit/form.rs`.

**Interfaces:** Consumes `parse`, `apply`, `FieldKind`/`FieldValue`/`Edit`. Produces the edit keymap: `Enter` edits the focused field; a per-kind commit-of-edit path that `parse`s the buffer and `apply`s a `SetField`; TriState/Enum/Bool cycle in place; inline error on parse failure.

- [ ] **Step 1: Write failing key-driven tests:** (a) on a `None` working copy, choosing a filing status (cycle the `FilingStatus` Enum to `Mfj` and confirm) materializes `working = Some(ri)` with `filing_status == Mfj` and the other sections appear; (b) typing `50000` into a Money field and confirming makes `get` return `Money($50000)` (drive via `type_str` + Enter, assert via the section's `get` accessor or a re-render containing `$50,000`); (c) an invalid Money entry (`abc`) sets `form.error` (a `ParseError` surfaced) and does NOT apply.

```rust
#[test]
fn choosing_filing_status_materializes_then_sections_appear() {
    let mut form = TaxInputsFormState::fresh(2024);
    // focus is on FilingStatus; cycle to Mfj and confirm (the Enum-cycle key, e.g. Enter/Space)
    tax_inputs_apply_edit(&mut form, /* set FilingStatus to */ "Mfj");   // the flow's edit-commit entry
    assert!(form.working.is_some());
    assert_eq!(form.working.as_ref().unwrap().filing_status, FilingStatus::Mfj);
}
```

- [ ] **Step 2: Run fail. Step 3: implement.** `Enter` on a focused field: for `Money/Text/Date/Secret` (Secret is Task 4) enter edit mode (`editing = true`, seed `buf` from the current value for Money/Text/Date), and a second `Enter` commits: `parse(field.kind, buf.as_str())` (Date/Money/Text/Enum) → on `Ok(value)` build `Edit::SetField{ id: field.id, addr: form.addr.clone(), value }` and `apply(&mut form.working, edit)` → on `Ok` clear error + exit edit mode; on `Err(ParseError)`/`Err(ApplyError)` set `form.error` (rendered inline). For `TriState` a key cycles `never→yes→no→never` via `SetField{TriState(None|Some(true)|Some(false))}` (or `ClearField` for `None`); `Enum` (`FilingStatus`/`ItemizeElection`/`W2Owner`/`CharClass`) cycles/selects among `FieldKind::Enum(options)` → `SetField{Choice(name)}`; `Bool` toggles `SetField{Bool(!cur)}`. **The `FilingStatus` case on a `None` working copy is the materialization** — `apply` handles it (NI-2); the renderer just sends the edit. After any successful `apply`, recompute live sections/fields (a materialization or a `DeleteSection` changes them).

- [ ] **Step 4: green; `make check`. Step 5: mutation-check** the parse-error guard (make the commit ignore a `parse` `Err` and apply a default → the invalid-Money test fails). **Step 6: commit** — `feat(input-form tui): per-kind field editing via parse+apply (plan 3 task 3)`

---

### Task 4: Secret fields — no-echo masked entry (SSN / IP-PIN) + masked render

**Files:** Modify `main.rs`/`edit/tax_inputs.rs`, `draw_edit.rs`, `edit/form.rs`.

**Interfaces:** Consumes `parse_ssn`/`parse_ip_pin`, `SecretView`. Produces a no-echo entry mode for `Secret` fields + a masker for the rendered value.

- [ ] **Step 1: Failing tests:** (a) a Secret field's rendered value shows the `SecretView` masked form (`***-**-1234` when set) and NEVER the raw digits — assert the render for a set SSN contains `***-**-` and does NOT contain the middle digits; (b) entering an SSN in no-echo mode shows a bullet count, not the typed digits; (c) committing a valid 9-digit SSN applies a `SecretEntry` (the underlying field is set — assert via `get` → `Secret(SecretView::Set{..})`), and an invalid SSN (`123`) sets `form.error` via `parse_ssn`'s `BadSsn`.

- [ ] **Step 2: fail. Step 3: implement.** A `Secret` field's edit is no-echo: keystrokes push to `buf` but the field pane renders `"●".repeat(buf.chars().count())` during entry (model on `draw_unlock_screen` `btctax-tui/draw.rs:51`). On commit: choose `parse_ssn` for `TpSsn`/`SpSsn`/`DepSsn` and `parse_ip_pin` for `IpPin` (by `FieldId`) → on `Ok(FieldValue::SecretEntry(_))` `apply(SetField)`; on `Err(BadSsn|BadIpPin)` set `form.error`. The DISPLAY (not editing) value uses the `get` accessor's `SecretView`: `Empty → "(unset)"`, `Set{masked} → masked`. Never render `SecretEntry` or raw digits.

- [ ] **Step 4: green; `make check`. Step 5: mutation-check** the no-leak: make the entry render `buf` verbatim → test (b) fails (digits visible); restore. **Step 6: commit** — `feat(input-form tui): secret no-echo masked entry (plan 3 task 4)`

---

### Task 5: Repeating + optional sections — add/remove row, create/delete section, row navigation

**Files:** Modify `main.rs`/`edit/tax_inputs.rs`, `edit/form.rs`, `draw_edit.rs`.

**Interfaces:** Consumes `Edit::{AddRow, RemoveRow, CreateSection, DeleteSection}`, `SectionKind`. Produces: `a` add row / `d` remove row (remove = payload-confirm) for `Repeating`; `c` create / `x` delete for `OptionalSingleton`; `RowAddr` navigation into rows (incl. `W2Box12` nested at depth 2).

- [ ] **Step 1: Failing key-driven tests:** (a) `a` on `W2s` adds a row (`apply(AddRow{W2s, addr})` → the W-2 list grows, a new row is focusable); (b) `c` on `ScheduleA` creates it (fields appear), `x` deletes it and `itemize_election` resets to `Auto` (I-10 — assert via `get`); (c) `d` on a W-2 row removes it (via a confirm); (d) a nested `W2Box12` add uses parent `[w2_i]` and the removed addr is `[w2_i, box12_i]`.

- [ ] **Step 2: fail. Step 3: implement.** In a `Repeating` section, render rows via a `TargetList`-style selectable list (recon §4; `form.rs:239`) with the current row highlighted; `a` → `apply(AddRow{ section, parent: current_parent_addr })`; `d` → open a small payload-confirm ("remove W-2 #2?") whose Enter → `apply(RemoveRow{ section, addr })`. In an `OptionalSingleton`, `c` → `apply(CreateSection{section})`, `x` → `apply(DeleteSection{section})` (the engine's ScheduleA delete resets `itemize_election`). Navigation: entering a `Repeating`/nested section pushes an index onto `form.addr`; `Up/Down` within rows changes the last index; leaving pops it. Guard arity via the engine's `apply` (it already fails closed on a malformed `RowAddr` — a returned `ApplyError` becomes `form.error`, never a panic). NOTE: `x` (delete optional section) is DISTINCT from `X` (discard parked draft, Task 8) — different keys, per §9A.

- [ ] **Step 4: green; `make check`. Step 5: mutation-check** the I-10 reset assertion (it's the engine's, but pin that `x` on ScheduleA routes a `DeleteSection` — make `x` a no-op → test (b) fails). **Step 6: commit** — `feat(input-form tui): repeating/optional section edits + row nav (plan 3 task 5)`

---

### Task 6: Autosave — debounced `save_draft` (I-7) on section-exit and short idle

**Files:** Modify `main.rs`/`editor.rs`, `edit/tax_inputs.rs`.

**Interfaces:** Consumes `input_form_store::save_draft`. Produces a debounced autosave: after a mutating edit, mark the flow dirty; flush `save_draft(sess, year, ri)` on section change and on a short idle tick (NOT per keystroke). Only a `Some(working)` writes.

- [ ] **Step 1: Failing test** (key-driven): make an edit, trigger a section change (the autosave point), and assert a draft row now exists on disk for the year (`input_form_store::load` on a fresh `Session`, or `draft_exists`) with the edited value — proving `save_draft` (and thus `Vault::save`) ran. Also assert that a `None` working copy writes NO draft.

- [ ] **Step 2: fail. Step 3: implement.** Add `dirty: bool` to `TaxInputsFormState`; set it on every successful mutating `apply`. Flush points: on `Left/Right`/`Tab` section change, and on the editor's idle/tick path if one exists (grep `editor.rs` for a tick/poll timeout; if the crate polls with a timeout, flush on timeout when `dirty`; else flush on section change + on flow close). Flush = `if form.dirty { if let Some(ri) = &form.working { input_form_store::save_draft(app.session_mut(), year, ri)?; form.dirty = false; } }`. `save_draft` preserves `parked` (NI-1, plan 2). Route any error through the existing `on_persist_error` mapping (`main.rs:627`). **Do NOT autosave a `None` working copy** (nothing chosen yet).

- [ ] **Step 4: green; `make check`. Step 5: mutation-check** the debounce/flush: remove the section-change flush → the autosave test fails (no draft on disk). **Step 6: commit** — `feat(input-form tui): debounced autosave via save_draft (plan 3 task 6)`

---

### Task 7: Commit (`s`) — payload-confirm modal → `commit`; refused jumps to the §7 anchor

**Files:** Modify `main.rs`/`edit/tax_inputs.rs`, `draw_edit.rs`, `edit/form.rs`.

**Interfaces:** Consumes `input_form_store::commit`, `attribute`, `shadows_profile`, `BundledTaxTables`/`BundledFullReturnTables`. Produces a `TaxInputsModalState` (payload-confirm) + the commit flow: `s` → build the confirm modal (names the filing status, the row it replaces, and — if `shadows_profile` — the shadow + all-zero warning) → Enter → `commit` → on `Refused` jump focus to the `attribute`d anchor + show the refusal; on `NoTables` show "year {y} has no full-return tables"; on `Committed` set status + clear the flow's dirty/draft state.

- [ ] **Step 1: Failing tests:** (a) a screen-refused return (bare `Single`, unanswered declarations) → `s` → `commit` returns `Refused`; the flow jumps `section_idx`/focus to the section that `attribute(refusal.reason)` names and shows the refusal text (assert `app.status`/`form.error` contains the refusal + the focus moved to the anchored section); (b) a screen-clean return → `s` opens the payload-confirm modal naming the filing status; Enter → `Committed` (assert a committed `return_inputs` row now exists AND the draft is gone); (c) a non-2024 year → `NoTables` (nothing written).

- [ ] **Step 2: fail. Step 3: implement.** `s` requires `working.is_some()` (else "choose a filing status first"). Build the confirm modal: `TaxInputsModalState { year, filing_status, summary: String, shadows: bool }` — the summary names the filing status (I-9), the sections present (n W-2s, Schedule A?, n dependents), and if `shadows_profile(conn, year)` the shadow + all-zero warning ("your tax-profile estimate stays saved and unused"; a declarations-only return commits ≈ $0). Modal Enter → `let table = BundledTaxTables::load().table_for(year); let params = BundledFullReturnTables::load().full_return_for(year); commit(sess, year, ri, table, params)`:
  - `CommitOutcome::Committed` → status "committed {year} as {filing_status}"; clear `dirty`; the flow may close or reload as `Committed`.
  - `CommitOutcome::Refused(refusal)` → `let anchors = attribute(&refusal.reason);` focus the first `Anchor::Field(id)`/`Anchor::Section(id)` that maps to a live section (set `section_idx` + field focus); `Anchor::NotInForm{note}` → show the note. Set `form.error`/status to `refusal.detail`.
  - `CommitOutcome::NoTables` → status "year {year} has no full-return tables (2024 only)".
  Esc on the modal cancels (writes nothing). Render the modal via `draw_tax_inputs_modal` (mirror `draw_mutation_modal` `draw_edit.rs:552`), showing the summary + `[Enter] commit  [Esc] cancel`.

- [ ] **Step 4: green; `make check`. Step 5: mutation-check** two guards: (a) make `Refused` NOT jump focus → test (a)'s focus assertion fails; (b) make the modal Enter skip `commit` → test (b)'s committed-row assertion fails. **Step 6: commit** — `feat(input-form tui): commit flow + payload-confirm + refusal anchor jump (plan 3 task 7)`

---

### Task 8: Toggle (`t`) + discard parked (`X`) — `park_to_profile` / re-commit / `discard_parked_draft`, active-source display, P2-a

**Files:** Modify `main.rs`/`edit/tax_inputs.rs`, `draw_edit.rs`.

**Interfaces:** Consumes `input_form_store::{active_source, park_to_profile, discard_parked_draft, load, commit}`. Produces: `t` toggles the source (park / re-commit, offered only from a clean/committed state); `X` discards a parked draft (confirmed); the active-source status; and the P2-a stale-parked discard path.

- [ ] **Step 1: Failing tests:** (a) with a committed full return active (`active_source == FullReturn`) and no WIP, `t` → confirm → `park_to_profile` runs (assert the committed row is gone and a `parked=1` draft exists; the status now shows `active source: tax-profile`); (b) with `active_source == TaxProfile` and a parked draft, `t` ("use full return") reloads the parked draft into `working` and re-commit is available; (c) `X` on a year with a parked draft → confirm → `discard_parked_draft` (the parked draft is gone); (d) **P2-a:** a flow opened in the `discard_offered` state (from a `StaleParkedDraft` load error, Task 1) → `X` → confirm → `discard_parked_draft` succeeds and the stale parked draft is gone.

- [ ] **Step 2: fail. Step 3: implement.** Status line shows `active_source(conn, year)`. `t`:
  - `FullReturn` active + not `dirty` (clean/committed) → confirm modal "use tax-profile (park this return)?" → `park_to_profile(sess, year)` (refuses if a divergent WIP exists — surface its `Usage` error). Offer ONLY from a clean state (if `dirty`, "save or commit first").
  - `TaxProfile` active with a parked draft → "use full return" → `load` the parked draft into `working` (it becomes editable; a subsequent `s` re-commits, which consumes `parked`).
  `X` (distinct from `x`): shown only when a parked draft exists (or in the `discard_offered` state) → a payload-showing confirm → `discard_parked_draft(sess, year)`. **P2-a:** in the `discard_offered` state (opened from `StaleParkedDraft`), `X` is the reachable escape — after a successful discard, close the flow (the stale parked draft is gone; the year falls back to committed/profile/fresh on reopen).

- [ ] **Step 4: green; `make check`. Step 5: mutation-check** the clean-state gate (`t` on a dirty flow must refuse park) + the `X`-only-when-parked gate. **Step 6: commit** — `feat(input-form tui): source toggle + discard parked (P2-a) (plan 3 task 8)`

---

### Task 9: Snapshot/KAT coverage — the §9A/§10 representative states + a quit-warns-on-unsaved check

**Files:** Modify `draw_edit.rs` (tests), `main.rs` (tests).

**Interfaces:** No new production code (or only trivial glue). Produces the spec §10 TUI snapshot set + key-driven KATs.

- [ ] **Step 1: Write the snapshot/KAT tests** (all should pass against Tasks 1-8; if one reveals a gap, fix the owning task's code and note it):
  - **empty year** — the fresh filing-status-only screen renders (from Task 2, re-pinned as a named §9A KAT).
  - **a two-W-2 MFJ return** — build the working copy via `apply` edits (Mfj + two AddRow{W2s} + box1 values), render, assert both W-2s + the MFJ status + Spouse section present.
  - **a screen-refused SALT state** — a return that triggers a SALT refusal; `s`; assert the refusal text + the focus jumped to the Schedule-A anchor (the `SaSaltUseSalesTax`/`salt_sales_tax_amount` fields).
  - **the commit modal** — a clean return; `s`; assert the modal names the filing status + the payload summary.
  - **the toggle prompt** — `t` on a committed return; assert the "use tax-profile" confirm names the shadow/park consequence.
  - **quit warns on an unsaved-draft divergence** — with `dirty`, `q` warns (but the draft is already autosaved, per §9A) — assert the warning appears and quit is safe.
  - **the renderer never names a `ReturnInputs` field** (§9A/§13): a grep-style test or a code assertion that `draw_tax_inputs_form` uses only `form_spec()` labels — at minimum, assert a rendered field uses the `Field.label`, not a struct field name.

- [ ] **Step 2: Run** — most pass; for any that fail, fix the owning task's code (a real gap the KAT caught) and re-run. **Step 3: `make check` green.**
- [ ] **Step 4: Commit** — `test(input-form tui): §9A/§10 snapshot + KAT coverage (plan 3 task 9)`

---

## Self-Review notes (controller)

- **Spec coverage:** §9A layout (T2) · Fresh-filing-status-only + NI-2 (T1/T2/T3) · per-kind editing + parse (T3) · Secret no-echo masked (T4) · repeating/optional + I-10 (T5) · autosave I-7 debounced (T6) · commit + payload-confirm + refusal-anchor jump §7 (T7) · toggle park/re-commit + discard + active-source §9 (T8) · §10 snapshot set (T9). P2-a stale-parked discard reachability: T1 (open in discard_offered) + T8 (X wired).
- **Consumes only** plan-1 engine + plan-2 store; does NOT modify either (if a gap appears, STOP and report — don't patch the engine from the TUI).
- **Deferred (out of plan 3):** docs (plan 4); the web renderer (a future second renderer over the same seam); the deferred sections (Schedule C/QBI/1099s).
- **Right-sizing:** T1-T8 each end with an independently-testable deliverable + a review; T9 is the coverage sweep. If T3 or T7 grows too large mid-implementation, its implementer should report DONE_WITH_CONCERNS and I'll split — don't silently truncate.
