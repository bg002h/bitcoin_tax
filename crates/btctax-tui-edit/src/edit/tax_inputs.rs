//! Task 3: per-kind field editing for the tax-inputs flow — the edit buffer, `parse`, and `apply`
//! dispatch (Money/Text/TriState/Enum/Bool/Date), incl. the NI-2 filing-status materialization.
//!
//! This layer is ENGINE-only: every mutation goes through `btctax_input_form::apply`, which mutates the
//! in-memory `Working` (`Option<ReturnInputs>`). There is NO store write here — autosave (the disk flush
//! via `save_draft`) lands in Task 6. It NEVER constructs a `ReturnInputs` and NEVER names a `ReturnInputs`
//! leaf field: it reads current values via `field.get` and materializes via `apply` (spec §9A/§13).
//!
//! **The chosen edit keymap** (documented once here; the key handler in `main.rs` calls into this module):
//! - `Enter` on a focused text-kind field (`Money`/`Text`/`Date`) → open the edit buffer (seeded from the
//!   current value via `get`); a second `Enter` commits (`parse` → `apply(SetField)`); `Esc` cancels.
//! - `Enter` or `Space` on a cycle kind (`Enum`/`TriState`/`Bool`) → cycle/toggle IN PLACE (apply on the
//!   keypress, no buffer). Enum cycles the options; TriState `never→yes→no→never`; Bool toggles.
//! - `Secret` is skipped here (no-echo masked entry is Task 4).

use crate::edit::form::{
    filing_status_field, filing_status_label, live_fields, live_sections, PendingRemove,
    TaxInputsFormState,
};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::return_refuse::RefuseReason;
use btctax_core::Usd;
use btctax_input_form::{
    apply, parse, parse_ip_pin, parse_ssn, Anchor, ApplyError, Edit, Field, FieldId, FieldKind,
    FieldValue, ParseError, RowAddr, Section, SectionId, SectionKind, SetError,
};

// ── Pane projection (the field-cursor fold, Task-2 Minor) ──────────────────────────────────────────────
//
// The renderer draws ONE of these panes for the selected section, and the field cursor advances ONLY over
// the CURRENTLY-drawn pane's items — never an invisible cursor on a row-list / `[create]` pane. Both the
// render (`draw_edit`) and the nav (`main.rs`) derive the pane from `(section_idx, form.addr)`, so the two
// can never disagree.

/// What the field pane draws for the selected section (Task 5). The `usize` is the number of navigable
/// items — the field cursor (`field_focus`) is clamped to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// A singleton (or a PRESENT optional-singleton): its live fields. `usize` = # live fields.
    Fields(usize),
    /// An ABSENT optional-singleton: the `[create]` affordance. No navigable items.
    Create,
    /// A repeating section's row LIST (no row entered yet — `form.addr` at the container path). `usize` =
    /// # rows; the cursor selects a row.
    RowList(usize),
    /// INSIDE a repeating row (`form.addr` = the row path): that row's live fields. `usize` = # live fields.
    RowFields(usize),
}

impl Pane {
    /// The number of navigable items in this pane — the clamp for the field/row cursor (the fold: a
    /// row-list navigates ROWS, a `[create]` navigates NOTHING, a fields pane navigates FIELDS).
    pub fn navigable(self) -> usize {
        match self {
            Pane::Fields(n) | Pane::RowFields(n) | Pane::RowList(n) => n,
            Pane::Create => 0,
        }
    }
}

/// The selected top-level [`Section`] for the current `section_idx`, or `None` when there is no live
/// section yet (a `None` working copy has none until a filing status materializes it).
fn selected_section(ri: &ReturnInputs, section_idx: usize) -> Option<&'static Section> {
    let sections = live_sections(ri);
    let sel = section_idx.min(sections.len().checked_sub(1)?);
    sections.get(sel).copied()
}

/// The [`Pane`] drawn for the selected section — the single source of truth the render and the nav share.
/// On a `None` working copy the only pane is the filing-status choice (one navigable field, NI-2).
pub fn active_pane(form: &TaxInputsFormState) -> Pane {
    let Some(ri) = form.working.as_ref() else {
        return Pane::Fields(1); // NI-2: only the filing-status field
    };
    // ★ Task-5 fix: a nested drill-down (`descent` set) projects the NESTED group's pane — its sub-list when
    // `form.addr` is the parent path, or a sub-row's fields when it is one level deeper. `descent` is the
    // extra bit that tells "box-12 list under [w2_i]" (RowList) apart from "W-2 row [w2_i] fields".
    if let Some(nested_id) = form.descent {
        let nested = nested_section(nested_id);
        let SectionKind::Repeating { len, .. } = nested.kind else {
            return Pane::Fields(0); // unreachable: descent targets are Repeating groups
        };
        return if form.addr.0.len() == nested_parent_depth(nested_id) {
            Pane::RowList(len(ri, &form.addr))
        } else {
            Pane::RowFields(live_fields(nested, ri).len())
        };
    }
    let Some(section) = selected_section(ri, form.section_idx) else {
        return Pane::Fields(0);
    };
    // A parent-fields pane with a nested child appends ONE synthetic "… (n) →" drill entry (a navigable item).
    let synth = nested_child_here(form).is_some() as usize;
    match section.kind {
        SectionKind::Singleton => Pane::Fields(live_fields(section, ri).len()),
        SectionKind::OptionalSingleton { present, .. } => {
            if present(ri) {
                Pane::Fields(live_fields(section, ri).len() + synth)
            } else {
                Pane::Create
            }
        }
        // A repeating section: `form.addr` empty ⇒ the row LIST; non-empty ⇒ INSIDE a row (that row's
        // fields, read/written at `form.addr`). Entering a row pushed the index; leaving pops it.
        SectionKind::Repeating { len, .. } => {
            if form.addr.0.is_empty() {
                Pane::RowList(len(ri, &RowAddr::default()))
            } else {
                Pane::RowFields(live_fields(section, ri).len() + synth)
            }
        }
    }
}

/// The number of navigable items in the currently-drawn pane (the field-cursor clamp).
pub fn navigable_count(form: &TaxInputsFormState) -> usize {
    active_pane(form).navigable()
}

// ── Focused-field projection ─────────────────────────────────────────────────────────────────────────

/// The currently-focused editable [`Field`], or `None` when the drawn pane is a row LIST or a `[create]`
/// affordance (those get their own row/create keys). On a `None` working copy the ONLY field is the
/// filing-status choice (NI-2). Inside a repeating row the field is read/written at `form.addr` (the row).
pub fn focused_field(form: &TaxInputsFormState) -> Option<&'static Field> {
    let Some(ri) = form.working.as_ref() else {
        return Some(filing_status_field());
    };
    // ★ Task-5 fix: inside a nested drill-down a focused field exists only at a sub-row (RowFields), read
    // from the NESTED section at `form.addr`; the sub-list level has no focused field (it navigates rows).
    if let Some(nested_id) = form.descent {
        return match active_pane(form) {
            Pane::RowFields(_) => live_fields(nested_section(nested_id), ri)
                .get(form.field_focus)
                .copied(),
            _ => None,
        };
    }
    let section = selected_section(ri, form.section_idx)?;
    match active_pane(form) {
        // The synthetic drill entry sits at index == live-field count; `.get` yields `None` there (it is
        // not an editable field — `Enter` on it drills in instead, via `on_nested_drill_entry`).
        Pane::Fields(_) | Pane::RowFields(_) => {
            live_fields(section, ri).get(form.field_focus).copied()
        }
        Pane::Create | Pane::RowList(_) => None,
    }
}

/// The focused field's [`FieldKind`] (for the key handler's Enter/Space dispatch).
pub fn focused_kind(form: &TaxInputsFormState) -> Option<FieldKind> {
    focused_field(form).map(|f| f.kind)
}

/// A text-entry kind: `Enter` opens the edit buffer, a second `Enter` commits.
pub fn is_text_kind(k: FieldKind) -> bool {
    matches!(k, FieldKind::Money | FieldKind::Text | FieldKind::Date)
}

/// A cycle-in-place kind: `Enter`/`Space` cycles/toggles and applies on the keypress.
pub fn is_cycle_kind(k: FieldKind) -> bool {
    matches!(
        k,
        FieldKind::Enum(_) | FieldKind::TriState | FieldKind::Bool
    )
}

/// A no-echo secret-entry kind (SSN / IP-PIN): `Enter` opens the buffer, keystrokes are MASKED to bullets
/// (never echoed), a second `Enter` commits via `parse_ssn`/`parse_ip_pin`. Distinct from `is_text_kind`
/// because a Secret is never seeded from its value and never renders its buffer.
pub fn is_secret_kind(k: FieldKind) -> bool {
    matches!(k, FieldKind::Secret)
}

// ── Edit-buffer entry / commit (text kinds) ──────────────────────────────────────────────────────────

/// Open the edit buffer on the focused text-kind field: seed `buf` from the current value (via `get`), set
/// `editing`, clear any stale error. A no-op if no editable field is focused.
pub fn begin_edit(form: &mut TaxInputsFormState) {
    let Some(field) = focused_field(form) else {
        return;
    };
    let seed = seed_string(field, form.working.as_ref(), &form.addr);
    form.buf.seed(&seed); // ★ P3-e: never truncate an already-accepted stored value on re-edit
    form.editing = true;
    form.error = None;
}

/// The edit-commit entry (also the direct-set entry the tests drive): `parse` `raw` for the focused field's
/// kind, then `apply` a `SetField`. Returns `true` on a clean apply (error cleared, focus re-clamped);
/// `false` on a parse/apply error (surfaced in `form.error`, nothing mutated — never a panic). The
/// `FilingStatus` case on a `None` working copy is the NI-2 materialization — handled entirely by `apply`.
pub fn tax_inputs_apply_edit(form: &mut TaxInputsFormState, raw: &str) -> bool {
    let Some(field) = focused_field(form) else {
        form.error = Some("no editable field is focused".to_string());
        return false;
    };
    let (id, kind) = (field.id, field.kind);
    // ★ The parse-error guard (spec §5.7): a bad value is rejected HERE — we build the `Edit` from the
    // parser's `Ok`, never from the raw text, so a `ParseError` never reaches `apply`. A `Secret` field
    // (Task 4) is parsed by the DEDICATED entry point chosen from its `FieldId` (`parse_ssn` for
    // `TpSsn`/`SpSsn`/`DepSsn`, `parse_ip_pin` for `IpPin`) — the generic `parse` refuses `Secret` on
    // purpose (it can't know which). Either way the digits leave `raw` only inside an opaque `SecretEntry`.
    let parsed = if matches!(kind, FieldKind::Secret) {
        parse_secret(id, raw)
    } else {
        parse(kind, raw)
    };
    let value = match parsed {
        Ok(v) => v,
        Err(e) => {
            form.error = Some(parse_error_msg(e));
            return false;
        }
    };
    apply_edit(
        form,
        Edit::SetField {
            id,
            addr: form.addr.clone(),
            value,
        },
    )
}

/// Cycle/toggle the focused cycle-kind field (`Enum`/`TriState`/`Bool`) in place, applying the next value.
/// The next value is computed from the CURRENT value read via `field.get` — never a constructed leaf. On a
/// `None` working copy the only cycle field is `FilingStatus` (Enum): the first cycle materializes the first
/// option (NI-2), handled by `apply`.
pub fn cycle_focused(form: &mut TaxInputsFormState) {
    let Some(field) = focused_field(form) else {
        return;
    };
    let (id, kind) = (field.id, field.kind);
    let current = form
        .working
        .as_ref()
        .and_then(|ri| (field.get)(ri, &form.addr));
    let edit = match kind {
        FieldKind::Enum(options) => Edit::SetField {
            id,
            addr: form.addr.clone(),
            value: FieldValue::Choice(next_enum(options, current.as_ref()).to_string()),
        },
        FieldKind::TriState => next_tristate_edit(id, &form.addr, current.as_ref()),
        FieldKind::Bool => {
            let cur = matches!(current, Some(FieldValue::Bool(true)));
            Edit::SetField {
                id,
                addr: form.addr.clone(),
                value: FieldValue::Bool(!cur),
            }
        }
        // A non-cycle kind should never reach here (the key handler dispatches by kind); ignore defensively.
        FieldKind::Money | FieldKind::Text | FieldKind::Date | FieldKind::Secret => return,
    };
    apply_edit(form, edit);
}

// ── Shape edits: add/remove row (Repeating) · create/delete section (OptionalSingleton) ────────────────
//
// Every shape edit goes through `apply(&mut form.working, Edit::…)` (via `apply_edit`) — the flow NEVER
// mutates `working` directly and never names a `ReturnInputs` leaf. A malformed `RowAddr` (or a create/
// delete on the wrong section kind) is the engine's fail-closed `ApplyError` → `form.error`, never a panic.

/// The selected section as a `Repeating` group, with its container path (`form.addr`) and row count.
/// `None` when the selected section is not a repeating group. The container path is `form.addr` — `[]` at a
/// top-level repeating section (W-2s/Dependents), one level shallower than a row of the group.
fn selected_repeating(form: &TaxInputsFormState) -> Option<(SectionId, usize)> {
    let ri = form.working.as_ref()?;
    // ★ Task-5 fix: inside a nested drill-down the ACTIVE repeating group is the nested section, counted at
    // `form.addr` (its parent path). So `a`/`d` target the box-12/charitable list, NOT the parent W-2/section.
    if let Some(nested_id) = form.descent {
        let nested = nested_section(nested_id);
        return match nested.kind {
            SectionKind::Repeating { len, .. } => Some((nested.id, len(ri, &form.addr))),
            _ => None,
        };
    }
    let section = selected_section(ri, form.section_idx)?;
    if let SectionKind::Repeating { len, .. } = section.kind {
        Some((section.id, len(ri, &form.addr)))
    } else {
        None
    }
}

/// The selected section as an `OptionalSingleton`, with whether it is currently present.
/// `None` when the selected section is not an optional-singleton.
fn selected_optional(form: &TaxInputsFormState) -> Option<(SectionId, bool)> {
    // ★ Task-5 fix: inside a nested drill-down, `c`/`x` must NOT act on the parent optional-singleton
    // (otherwise `x` while browsing the charitable list would delete Schedule A out from under it).
    if form.descent.is_some() {
        return None;
    }
    let ri = form.working.as_ref()?;
    let section = selected_section(ri, form.section_idx)?;
    if let SectionKind::OptionalSingleton { present, .. } = section.kind {
        Some((section.id, present(ri)))
    } else {
        None
    }
}

/// `a` — add a row to the selected repeating group: `AddRow{ section, parent: form.addr }` (the container
/// path). On success the cursor moves to the newly-added last row so it is immediately focusable. A no-op
/// (returns `false`) when the selected section is not a repeating group, or the row list is not the drawn
/// pane (`form.addr` non-empty ⇒ we are inside a row, not the list).
pub fn add_row(form: &mut TaxInputsFormState) -> bool {
    if !matches!(active_pane(form), Pane::RowList(_)) {
        return false;
    }
    let Some((section, _)) = selected_repeating(form) else {
        return false;
    };
    let ok = apply_edit(
        form,
        Edit::AddRow {
            section,
            parent: form.addr.clone(),
        },
    );
    if ok {
        if let Some((_, rows)) = selected_repeating(form) {
            form.field_focus = rows.saturating_sub(1);
        }
    }
    ok
}

/// `d` — stage the payload-confirm for removing the CURRENTLY-selected row (the modal's Enter then calls
/// [`confirm_remove`]). Freezes the row's address NOW so a later cursor move cannot re-target the delete.
/// A no-op when the row list is not the drawn pane or the group is empty.
pub fn stage_remove_selected(form: &mut TaxInputsFormState) {
    let Some((section, rows)) = selected_repeating(form) else {
        return;
    };
    if rows == 0 || !matches!(active_pane(form), Pane::RowList(_)) {
        return;
    }
    let row = form.field_focus.min(rows - 1);
    let mut addr = form.addr.clone();
    addr.0.push(row);
    form.pending_remove = Some(PendingRemove {
        section,
        addr,
        label: remove_label(section, row),
    });
}

/// The confirm-modal Enter: apply the staged `RemoveRow` and clear the staging. Returns `true` on a clean
/// remove. A malformed/stale address surfaces as `form.error` (never a panic) and the staging is cleared.
pub fn confirm_remove(form: &mut TaxInputsFormState) -> bool {
    let Some(pr) = form.pending_remove.take() else {
        return false;
    };
    apply_edit(
        form,
        Edit::RemoveRow {
            section: pr.section,
            addr: pr.addr,
        },
    )
}

/// The confirm-modal Esc: drop the staged removal without touching the working copy.
pub fn cancel_remove(form: &mut TaxInputsFormState) {
    form.pending_remove = None;
}

/// `c` — create the selected optional-singleton (`CreateSection{ section }`). A no-op when the selected
/// section is not an optional-singleton, or it is already present.
pub fn create_selected_section(form: &mut TaxInputsFormState) -> bool {
    let Some((section, present)) = selected_optional(form) else {
        return false;
    };
    if present {
        return false;
    }
    apply_edit(form, Edit::CreateSection { section })
}

/// `x` — delete the selected optional-singleton (`DeleteSection{ section }`). For `ScheduleA` the engine's
/// `delete` also resets `itemize_election` to `Auto` (I-10) — we route through it, never re-implement it.
/// DISTINCT from `X` (discard a parked draft, Task 8). A no-op when the section is not present.
pub fn delete_selected_section(form: &mut TaxInputsFormState) -> bool {
    let Some((section, present)) = selected_optional(form) else {
        return false;
    };
    if !present {
        return false;
    }
    apply_edit(form, Edit::DeleteSection { section })
}

// ── Row navigation (the `form.addr` push/pop) ──────────────────────────────────────────────────────────

/// `Enter` on a row-list pane — enter the selected row: push its index onto `form.addr` (so per-row field
/// editing, Task 3, targets that row) and reset the field cursor to the row's first field. A no-op unless
/// the drawn pane is a non-empty row list.
pub fn enter_selected_row(form: &mut TaxInputsFormState) {
    if let Pane::RowList(rows) = active_pane(form) {
        if rows > 0 {
            let row = form.field_focus.min(rows - 1);
            form.addr.0.push(row);
            form.field_focus = 0;
        }
    }
}

/// `Left`/`Esc` inside a row — leave it: pop the last index off `form.addr` and restore the cursor to the
/// row we were in (so the caller lands back on the row list at the same row). `true` when a level was
/// popped (there was a row to leave), `false` at the top level.
pub fn leave_row(form: &mut TaxInputsFormState) -> bool {
    // ★ Task-5 fix: inside a nested drill-down, back-navigation pops WITHIN the nested context first — a
    // sub-row's fields → the sub-list (pop the row index), then the sub-list → the parent's fields (clear
    // `descent`, land on the synthetic drill entry). Only THEN does the top-level pop/close apply.
    if let Some(nested_id) = form.descent {
        if form.addr.0.len() > nested_parent_depth(nested_id) {
            let row = form.addr.0.pop().unwrap_or(0);
            form.field_focus = row;
        } else {
            form.descent = None;
            form.field_focus = parent_live_fields_len(form);
        }
        return true;
    }
    if let Some(row) = form.addr.0.pop() {
        form.field_focus = row;
        true
    } else {
        false
    }
}

// ── Nested-group drill-down (Task-5 fix) ───────────────────────────────────────────────────────────────
//
// Two v1 groups are addressed at DEPTH 2 by the engine but are NOT top-level sections (`section_is_live`
// skips them): `W2Box12` (a W-2's box-12 lines) and `ScheduleACharitable` (Schedule-A gifts). The filer
// reaches them by DESCENDING from the parent's fields pane via a synthetic "… (n) →" entry. `form.descent`
// is the one extra nav bit that disambiguates the descent from the parent's own row-fields (same `addr`).
//
// Address depths (confirmed against the engine's `Repeating` accessors in `sections.rs`):
//   • `W2Box12`             — parent `[w2_i]` (depth 1); a row `[w2_i, box12_i]` (depth 2).
//   • `ScheduleACharitable` — parent `[]` (depth 0); a gift `[charitable_i]` (depth 1).

/// The nested repeating child section reachable from a parent section, if any.
fn nested_child_of(id: SectionId) -> Option<SectionId> {
    match id {
        SectionId::W2s => Some(SectionId::W2Box12),
        SectionId::ScheduleA => Some(SectionId::ScheduleACharitable),
        _ => None,
    }
}

/// The PARENT-address depth of a nested repeating group — the number of indices in its parent path (one
/// less than a row's own depth). `W2Box12` sits under a W-2 row (`[w2_i]`, depth 1); `ScheduleACharitable`
/// sits under the optional Schedule-A singleton (`[]`, depth 0).
fn nested_parent_depth(id: SectionId) -> usize {
    match id {
        SectionId::W2Box12 => 1,
        _ => 0,
    }
}

/// Look up a nested section's `&'static Section` (its `Repeating` accessors + live fields).
pub fn nested_section(id: SectionId) -> &'static Section {
    btctax_input_form::form_spec()
        .iter()
        .find(|s| s.id == id)
        .expect("nested section is present in form_spec()")
}

/// The nested repeating child whose synthetic "… (n) →" drill entry the CURRENT parent-fields pane carries,
/// if any. `Some` only at the parent level (`descent == None`) when the parent is showing its OWN fields:
/// inside a W-2 row (`W2s → W2Box12`) or a PRESENT Schedule A (`ScheduleA → ScheduleACharitable`). `None`
/// everywhere else — a row list, a `[create]`, or already inside a nested group.
pub fn nested_child_here(form: &TaxInputsFormState) -> Option<SectionId> {
    if form.descent.is_some() {
        return None;
    }
    let ri = form.working.as_ref()?;
    let section = selected_section(ri, form.section_idx)?;
    let child = nested_child_of(section.id)?;
    let shows_fields = match section.kind {
        SectionKind::Repeating { .. } => !form.addr.0.is_empty(), // inside a row (not the row list)
        SectionKind::OptionalSingleton { present, .. } => present(ri),
        _ => false,
    };
    shows_fields.then_some(child)
}

/// The number of live fields of the currently-selected PARENT section — the index of its synthetic drill
/// entry (appended after the fields), and where the cursor lands when popping back out of a nested group.
fn parent_live_fields_len(form: &TaxInputsFormState) -> usize {
    form.working
        .as_ref()
        .and_then(|ri| selected_section(ri, form.section_idx).map(|s| live_fields(s, ri).len()))
        .unwrap_or(0)
}

/// Is the field cursor on the synthetic nested-group drill entry (the last item of a parent-fields pane
/// that has a nested child)? The gate for the drill-in key — `Enter` on "Box 12 entries (n) →" /
/// "Charitable gifts (n) →".
pub fn on_nested_drill_entry(form: &TaxInputsFormState) -> bool {
    nested_child_here(form).is_some() && form.field_focus == parent_live_fields_len(form)
}

/// Drill INTO the nested repeating group whose synthetic entry the cursor is on. Sets the `descent` marker
/// and lands on the group's sub-list — `form.addr` STAYS the parent path (`[w2_i]` for box-12, `[]` for
/// charitable), which is exactly that group's `Repeating::len`/`add` parent. A no-op (returns `false`) off
/// the synthetic entry — the mutation-checked guard: neuter it and no box-12/charitable entry is reachable.
pub fn enter_nested_group(form: &mut TaxInputsFormState) -> bool {
    if !on_nested_drill_entry(form) {
        return false;
    }
    let Some(child) = nested_child_here(form) else {
        return false;
    };
    form.descent = Some(child);
    form.field_focus = 0;
    true
}

/// The payload string the remove-confirm shows, e.g. "remove W-2 #2?".
fn remove_label(section: SectionId, row: usize) -> String {
    let noun = match section {
        SectionId::W2s => "W-2",
        SectionId::Dependents => "dependent",
        SectionId::W2Box12 => "box-12 entry",
        SectionId::ScheduleACharitable => "charitable gift",
        _ => "row",
    };
    format!("remove {noun} #{}?", row + 1)
}

// ── Internals ────────────────────────────────────────────────────────────────────────────────────────

/// Apply an already-built `Edit` to `form.working`. On `Ok`: clear the error and re-clamp focus (a
/// materialization or a section create/delete changes the live set). On `Err`: surface it in `form.error`
/// and mutate nothing (never a panic — a bad `RowAddr`/`WrongKind`/`Immutable` is a clean error).
fn apply_edit(form: &mut TaxInputsFormState, edit: Edit) -> bool {
    match apply(&mut form.working, edit) {
        Ok(()) => {
            form.error = None;
            // ★ I-4: a successful mutating apply changes the model — any recorded screen refusal is now
            // stale, so clear the `!` attribution (it re-arms only on the next refused commit).
            form.refused_section = None;
            // ★ Task 6 (autosave, I-7): a successful mutating apply marks the flow dirty; the disk flush is
            // DEBOUNCED to the flow's flush points (section change / idle tick / flow close / `q`), never here.
            form.dirty = true;
            clamp_focus(form);
            true
        }
        Err(e) => {
            form.error = Some(apply_error_msg(e));
            false
        }
    }
}

/// Re-clamp `section_idx`/`field_focus` into the CURRENT live set after a successful apply. The field
/// cursor clamps to the CURRENTLY-DRAWN pane's navigable count (rows for a row list, fields for a fields
/// pane, 0 for a `[create]`) — the Task-2 Minor fold, so the cursor never lands off the drawn pane.
fn clamp_focus(form: &mut TaxInputsFormState) {
    let Some(ri) = form.working.as_ref() else {
        return;
    };
    let sections = live_sections(ri);
    if sections.is_empty() {
        form.section_idx = 0;
        form.field_focus = 0;
        return;
    }
    form.section_idx = form.section_idx.min(sections.len() - 1);
    let n = navigable_count(form);
    form.field_focus = form.field_focus.min(n.saturating_sub(1));
}

/// The raw editable string to seed the buffer with for a text-kind field, from its current value via `get`.
/// A zero Money / an empty Text / an unset Date seeds an EMPTY buffer (clean entry); a set value seeds its
/// re-parseable text (Money → the plain decimal, no `$`; Date → `YYYY-MM-DD`).
fn seed_string(field: &Field, ri: Option<&ReturnInputs>, addr: &RowAddr) -> String {
    let Some(ri) = ri else {
        return String::new();
    };
    match (field.get)(ri, addr) {
        Some(FieldValue::Money(m)) if m != Usd::ZERO => m.to_string(),
        Some(FieldValue::Text(s)) => s,
        Some(FieldValue::Date(Some(d))) => d.to_string(),
        _ => String::new(),
    }
}

/// The next Enum option after the current choice (wrapping). A `None`/unknown current (a `None` working
/// copy, or a value not among the options) picks the FIRST option.
fn next_enum(options: &[&'static str], current: Option<&FieldValue>) -> &'static str {
    let cur_name = match current {
        Some(FieldValue::Choice(c)) => Some(c.as_str()),
        _ => None,
    };
    match cur_name.and_then(|name| options.iter().position(|o| *o == name)) {
        Some(i) => options[(i + 1) % options.len()],
        None => options[0],
    }
}

/// The TriState cycle step `never → yes → no → never`. The `None` step uses `ClearField` (the engine's
/// un-answer path); every `Err` from `apply` (e.g. a registry-delegating field that rejects the clear on an
/// absent parent) is surfaced as `form.error`, never a panic.
fn next_tristate_edit(id: FieldId, addr: &RowAddr, current: Option<&FieldValue>) -> Edit {
    let cur = match current {
        Some(FieldValue::TriState(t)) => *t,
        _ => None,
    };
    match cur {
        None => Edit::SetField {
            id,
            addr: addr.clone(),
            value: FieldValue::TriState(Some(true)),
        },
        Some(true) => Edit::SetField {
            id,
            addr: addr.clone(),
            value: FieldValue::TriState(Some(false)),
        },
        Some(false) => Edit::ClearField {
            id,
            addr: addr.clone(),
        },
    }
}

/// Parse a `Secret` field's raw entry via the entry point selected by `FieldId`: `parse_ssn` for the
/// SSN fields (`TpSsn`/`SpSsn`/`DepSsn`), `parse_ip_pin` for `IpPin`. On success the canonical digits are
/// wrapped in an opaque `SecretEntry` (masked `Debug`); on failure a `BadSsn`/`BadIpPin` is surfaced (never
/// a panic, never a leak). Any other `FieldId` is not a `Secret` in the spec — refuse rather than guess.
fn parse_secret(id: FieldId, raw: &str) -> Result<FieldValue, ParseError> {
    match id {
        FieldId::TpSsn | FieldId::SpSsn | FieldId::DepSsn => parse_ssn(raw),
        FieldId::IpPin => parse_ip_pin(raw),
        _ => Err(ParseError::BadSsn),
    }
}

/// A one-line, human-readable message for a `ParseError` (rendered inline under the field pane).
fn parse_error_msg(e: ParseError) -> String {
    match e {
        ParseError::NotANumber => "not a number".to_string(),
        ParseError::Negative => "must not be negative".to_string(),
        ParseError::BadDate => "bad date (expected YYYY-MM-DD)".to_string(),
        ParseError::BadSsn => "bad SSN".to_string(),
        ParseError::BadIpPin => "bad IP PIN".to_string(),
        ParseError::NotAChoice => "not a valid choice".to_string(),
    }
}

// ── Task 7: the commit payload-confirm summary + the refusal → anchor → focus jump ─────────────────────

/// A top-level section's `&'static Section` by id (its accessors). Panics only on a stable-id typo.
fn section_by_id(id: SectionId) -> &'static Section {
    btctax_input_form::form_spec()
        .iter()
        .find(|s| s.id == id)
        .expect("section id is present in form_spec()")
}

/// The row count of a Repeating section for `ri`, via its `Repeating::len` accessor (NEVER a `ReturnInputs`
/// leaf). `0` for a non-repeating section (defensive — callers pass repeating ids).
fn repeating_len(ri: &ReturnInputs, id: SectionId) -> usize {
    match section_by_id(id).kind {
        SectionKind::Repeating { len, .. } => len(ri, &RowAddr::default()),
        _ => 0,
    }
}

/// Whether an OptionalSingleton section is present for `ri`, via its `present` accessor (NEVER a leaf).
fn optional_present(ri: &ReturnInputs, id: SectionId) -> bool {
    match section_by_id(id).kind {
        SectionKind::OptionalSingleton { present, .. } => present(ri),
        _ => false,
    }
}

/// The commit modal's multi-line payload summary (Task 7): the filing status (read via the accessor, never
/// `ri.filing_status`), the sections present (n W-2s, whether a Schedule A, n dependents — all via the
/// engine's `Repeating::len` / `OptionalSingleton::present` accessors, never a leaf), and — when a raw
/// `tax_profile` is shadowed (`shadows`) — the shadow + all-zero warning (§9 create-row amendment).
pub fn commit_summary(ri: &ReturnInputs, shadows: bool) -> String {
    let fs = filing_status_label(ri);
    let w2s = repeating_len(ri, SectionId::W2s);
    let deps = repeating_len(ri, SectionId::Dependents);
    let sched_a = if optional_present(ri, SectionId::ScheduleA) {
        "yes"
    } else {
        "no"
    };
    let mut s = format!(
        "filing status: {fs}\n{w2s} W-2(s)  ·  Schedule A: {sched_a}  ·  {deps} dependent(s)"
    );
    if shadows {
        s.push_str(
            "\n\na tax-profile estimate exists for this year — it stays saved and unused once this full \
             return commits (your tax-profile estimate stays saved and unused). A declarations-only \
             return (no income entered) commits \u{2248} $0.",
        );
    }
    s
}

/// The outcome of [`focus_refusal`]: focus moved to an in-form anchor, or the refusal points only OUTSIDE
/// the v1 form (a `NotInForm` note to surface), or nothing was focusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalFocus {
    /// Focus moved to the first in-form anchor (a live field/section).
    Moved,
    /// No in-form anchor was focusable; the refusal points outside the v1 form — surface this note.
    NotInForm(&'static str),
    /// Nothing to focus (no working copy; should not happen on a refused commit).
    None,
}

/// The section shown at the TOP level for an anchor's section: a nested group maps to its parent
/// (`W2Box12 → W2s`, `ScheduleACharitable → ScheduleA`), everything else is itself. `live_sections` never
/// lists the two nested groups, so those anchors focus the parent that owns them.
fn top_level_section(id: SectionId) -> SectionId {
    match id {
        SectionId::W2Box12 => SectionId::W2s,
        SectionId::ScheduleACharitable => SectionId::ScheduleA,
        other => other,
    }
}

/// The section that OWNS a field in `form_spec()` (the section whose `fields` contains `id`), if any.
fn owning_section_of_field(id: FieldId) -> Option<SectionId> {
    btctax_input_form::form_spec()
        .iter()
        .find(|s| s.fields.iter().any(|f| f.id == id))
        .map(|s| s.id)
}

/// Resolve an `Anchor::Section(id)` to its live-section index (mapping a nested group to its parent), or
/// `None` when that section is not a live top-level entry for `ri`.
fn resolve_section_anchor(ri: &ReturnInputs, id: SectionId) -> Option<usize> {
    let top = top_level_section(id);
    live_sections(ri).iter().position(|s| s.id == top)
}

/// Resolve an `Anchor::Field(id)` to a `(section_idx, field_focus)` in the CURRENT live layout, or `None`
/// when the field's top-level section is not live. Prefers the field's own live index; falls back to the
/// section (field 0) when the field itself is not currently a live field.
fn resolve_field_anchor(ri: &ReturnInputs, id: FieldId) -> Option<(usize, usize)> {
    let owner = owning_section_of_field(id)?;
    let sidx = resolve_section_anchor(ri, owner)?;
    let sections = live_sections(ri);
    let fidx = live_fields(sections[sidx], ri)
        .iter()
        .position(|f| f.id == id)
        .unwrap_or(0);
    Some((sidx, fidx))
}

/// ★ Task 7: jump focus to the FIRST in-form anchor `attribute(reason)` names — the refused-commit remedy
/// (SPEC §7). Sets `section_idx` + `field_focus` (clearing the row path + nested `descent`) to the first
/// `Field`/`Section` anchor that maps to a LIVE section/field. A `NotInForm` anchor moves nothing; its note
/// is returned so the caller surfaces it. The mutation-checked behavior: neuter the move and a refused
/// filer is stranded on whatever field they pressed `s` from (test (a)'s focus assertion fails).
pub fn focus_refusal(form: &mut TaxInputsFormState, reason: &RefuseReason) -> RefusalFocus {
    // Compute the target under an immutable borrow of `working`, THEN mutate `form` (disjoint borrows).
    // ★ I-4: alongside the focus target, capture the LIVE top-level section it lands on (a `SectionId`,
    // never a leaf) so the `!` glyph + `1 issue: <section>` status can attribute the refusal.
    let (target, section_id, not_in_form) = {
        let Some(ri) = form.working.as_ref() else {
            return RefusalFocus::None;
        };
        let mut target: Option<(usize, usize)> = None;
        let mut note: Option<&'static str> = None;
        for anchor in btctax_input_form::attribute(reason) {
            match anchor {
                Anchor::Field(id) => {
                    if let Some(t) = resolve_field_anchor(ri, id) {
                        target = Some(t);
                        break;
                    }
                }
                Anchor::Section(id) => {
                    if let Some(sidx) = resolve_section_anchor(ri, id) {
                        target = Some((sidx, 0));
                        break;
                    }
                }
                Anchor::NotInForm { note: n } => {
                    note.get_or_insert(n);
                }
            }
        }
        let section_id = target.map(|(sidx, _)| live_sections(ri)[sidx].id);
        (target, section_id, note)
    };
    match target {
        Some((sidx, fidx)) => {
            form.section_idx = sidx;
            form.field_focus = fidx;
            form.addr = RowAddr::default();
            form.descent = None;
            // ★ I-4: attribute the `!` glyph to the section the focus jumped to.
            form.refused_section = section_id;
            RefusalFocus::Moved
        }
        None => {
            // ★ I-4: no in-form anchor to attribute — carry no `!` glyph (the note/status still surfaces).
            form.refused_section = None;
            match not_in_form {
                Some(n) => RefusalFocus::NotInForm(n),
                None => RefusalFocus::None,
            }
        }
    }
}

/// A one-line, human-readable message for an `ApplyError`.
fn apply_error_msg(e: ApplyError) -> String {
    match e {
        // On the flow this can only arise from an edit that isn't the filing-status choice on a `None`
        // working copy — the renderer only offers `FilingStatus` there, so this is a belt-and-suspenders map.
        ApplyError::NotChosenYet | ApplyError::WrongFirstEdit => {
            "choose a filing status first".to_string()
        }
        ApplyError::NoSuchSection => "no such section".to_string(),
        ApplyError::SetError(SetError::WrongKind) => "wrong value for this field".to_string(),
        ApplyError::SetError(SetError::NoSuchRow) => "no such row".to_string(),
        ApplyError::SetError(SetError::Immutable) => "this field cannot be cleared".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::tax::types::FilingStatus;
    use btctax_input_form::SectionId;
    use rust_decimal_macros::dec;

    /// Materialize a Single working return (via the edit-commit entry, never a constructed `ReturnInputs`)
    /// and focus the Payments → PayEstimated singleton Money field.
    fn form_focused_on_pay_estimated() -> TaxInputsFormState {
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        focus_field(&mut form, SectionId::Payments, FieldId::PayEstimated);
        form
    }

    /// Point `section_idx`/`field_focus` at the given section+field (by stable id).
    fn focus_field(form: &mut TaxInputsFormState, sec: SectionId, fld: FieldId) {
        let ri = form.working.as_ref().unwrap();
        let sections = live_sections(ri);
        let s = sections.iter().position(|x| x.id == sec).unwrap();
        let f = live_fields(sections[s], ri)
            .iter()
            .position(|x| x.id == fld)
            .unwrap();
        form.section_idx = s;
        form.field_focus = f;
    }

    /// Read the focused field's current value via the `get` accessor (never a leaf name).
    fn focused_value(form: &TaxInputsFormState) -> Option<FieldValue> {
        let field = focused_field(form)?;
        (field.get)(form.working.as_ref()?, &form.addr)
    }

    /// (a) NI-2 materialization: choosing a filing status on a `None` working copy materializes the return
    /// with that status, and the rest of the sections then appear.
    #[test]
    fn choosing_filing_status_materializes_then_sections_appear() {
        let mut form = TaxInputsFormState::fresh(2024);
        // focus is on FilingStatus; set it to Mfj via the flow's edit-commit entry.
        assert!(tax_inputs_apply_edit(&mut form, "Mfj"));
        assert!(
            form.working.is_some(),
            "the filing-status choice materializes the return (NI-2)"
        );
        assert_eq!(
            form.working.as_ref().unwrap().filing_status,
            FilingStatus::Mfj
        );
        let sections = live_sections(form.working.as_ref().unwrap());
        assert!(
            sections.len() > 1,
            "materialization reveals the rest of the sections"
        );
        assert!(
            sections.iter().any(|s| s.id == SectionId::Spouse),
            "Spouse is offered once Mfj is chosen"
        );
        assert!(form.error.is_none());
    }

    /// (b) A valid Money commit round-trips through `get`.
    #[test]
    fn money_edit_roundtrips_via_get() {
        let mut form = form_focused_on_pay_estimated();
        assert!(tax_inputs_apply_edit(&mut form, "50000"));
        assert_eq!(focused_value(&form), Some(FieldValue::Money(dec!(50000))));
        assert!(form.error.is_none());
    }

    /// (c) An invalid Money entry surfaces a `ParseError` in `form.error` and applies NOTHING (the field
    /// keeps its prior value). The prior value is a known non-zero, so a mutant that "applies a default"
    /// instead of honoring the parse error is caught on the value AND the error/return assertions.
    #[test]
    fn invalid_money_sets_error_and_does_not_apply() {
        let mut form = form_focused_on_pay_estimated();
        assert!(tax_inputs_apply_edit(&mut form, "12345"));
        assert_eq!(focused_value(&form), Some(FieldValue::Money(dec!(12345))));

        assert!(
            !tax_inputs_apply_edit(&mut form, "abc"),
            "a bad Money entry must not commit"
        );
        assert!(form.error.is_some(), "the ParseError is surfaced inline");
        assert_eq!(
            focused_value(&form),
            Some(FieldValue::Money(dec!(12345))),
            "the bad entry did not mutate the field"
        );
    }

    /// The Enum cycle advances in place; on a `None` working copy the first cycle materializes the first
    /// option (Single), the next advances (Mfj) — reading current via `get`, never a constructed value.
    #[test]
    fn enum_cycle_advances_filing_status_in_place() {
        let mut form = TaxInputsFormState::fresh(2024);
        cycle_focused(&mut form);
        assert_eq!(
            form.working.as_ref().unwrap().filing_status,
            FilingStatus::Single
        );
        cycle_focused(&mut form);
        assert_eq!(
            form.working.as_ref().unwrap().filing_status,
            FilingStatus::Mfj
        );
    }

    /// The TriState cycle walks `never → yes → no → never`, using `ClearField` for the None step (which the
    /// engine honors for a live registry-delegating tri-state). No panic on any step.
    #[test]
    fn tristate_cycles_never_yes_no_never_via_clearfield() {
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        focus_field(&mut form, SectionId::Skippables, FieldId::BlindTaxpayer);

        let read = |form: &TaxInputsFormState| match focused_value(form) {
            Some(FieldValue::TriState(t)) => t,
            other => panic!("expected a TriState, got {other:?}"),
        };
        assert_eq!(read(&form), None, "starts un-answered (never)");
        cycle_focused(&mut form);
        assert_eq!(read(&form), Some(true), "never → yes");
        cycle_focused(&mut form);
        assert_eq!(read(&form), Some(false), "yes → no");
        cycle_focused(&mut form);
        assert_eq!(read(&form), None, "no → never (via ClearField)");
        assert!(form.error.is_none());
    }

    /// (c) Task 4 — a valid 9-digit SSN commits a `SecretEntry` (chosen by `FieldId` → `parse_ssn`): the
    /// field reads back SET and MASKED via `get`, never digits, and no error.
    #[test]
    fn secret_ssn_commit_sets_field_masked_via_get() {
        use btctax_input_form::SecretView;
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        focus_field(&mut form, SectionId::Taxpayer, FieldId::TpSsn);
        assert!(
            tax_inputs_apply_edit(&mut form, "123456789"),
            "a valid 9-digit SSN commits"
        );
        match focused_value(&form) {
            Some(FieldValue::Secret(SecretView::Set { masked })) => {
                assert!(
                    masked.starts_with("***-**-"),
                    "the read-back is masked, never digits"
                );
                assert!(!masked.contains("12345"), "the middle digits never surface");
            }
            other => panic!("expected a Set SecretView, got {other:?}"),
        }
        assert!(form.error.is_none());
    }

    /// (c-bis) Task 4 — the parser is chosen by `FieldId`: a 6-digit IP PIN commits via `parse_ip_pin`.
    /// The SAME string is `BadSsn` under `parse_ssn` (wrong length), so a mis-dispatch would REJECT it —
    /// this asserts the `IpPin` → `parse_ip_pin` selection specifically.
    #[test]
    fn secret_ip_pin_commit_uses_parse_ip_pin_not_ssn() {
        use btctax_input_form::SecretView;
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        focus_field(&mut form, SectionId::Taxpayer, FieldId::IpPin);
        assert!(
            tax_inputs_apply_edit(&mut form, "112233"),
            "a 6-digit IP PIN commits via parse_ip_pin (BadSsn under parse_ssn)"
        );
        assert!(matches!(
            focused_value(&form),
            Some(FieldValue::Secret(SecretView::Set { .. }))
        ));
        assert!(form.error.is_none());
    }

    /// Point `section_idx` at a section (by id), resetting the row cursor to its root (`addr = []`).
    fn focus_section(form: &mut TaxInputsFormState, sec: SectionId) {
        let ri = form.working.as_ref().unwrap();
        let sections = live_sections(ri);
        form.section_idx = sections.iter().position(|x| x.id == sec).unwrap();
        form.addr = RowAddr::default();
        form.field_focus = 0;
    }

    /// The engine's row count for a repeating section (via its `Repeating::len` accessor — never a leaf).
    fn row_count(form: &TaxInputsFormState, sec: SectionId, parent: &RowAddr) -> usize {
        let ri = form.working.as_ref().unwrap();
        let section = btctax_input_form::form_spec()
            .iter()
            .find(|s| s.id == sec)
            .unwrap();
        match section.kind {
            SectionKind::Repeating { len, .. } => len(ri, parent),
            _ => panic!("{sec:?} is not a repeating section"),
        }
    }

    /// Read a field's value via its `get` accessor at `addr` (never a leaf).
    fn get_at(form: &TaxInputsFormState, id: FieldId, addr: &RowAddr) -> Option<FieldValue> {
        let ri = form.working.as_ref().unwrap();
        let field = btctax_input_form::form_spec()
            .iter()
            .flat_map(|s| s.fields.iter())
            .find(|f| f.id == id)
            .unwrap();
        (field.get)(ri, addr)
    }

    /// (Task 5, d) The nested `W2Box12` group is addressed at DEPTH 2: `AddRow` parents on `[w2_i]` and
    /// `RemoveRow` targets `[w2_i, box12_i]`. A malformed (too-shallow) address is the engine's fail-closed
    /// `ApplyError` → `form.error`, never a panic, never a mutation.
    #[test]
    fn nested_box12_uses_depth2_addr_and_bad_addr_errors_no_panic() {
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        // Add a W-2 (row 0) via the flow's row-list `add_row`.
        focus_section(&mut form, SectionId::W2s);
        assert!(add_row(&mut form));
        assert_eq!(row_count(&form, SectionId::W2s, &RowAddr::default()), 1);

        // box-12 add: parent = [0] (the W-2 row). The nested group grows under that W-2.
        assert!(apply_edit(
            &mut form,
            Edit::AddRow {
                section: SectionId::W2Box12,
                parent: RowAddr(vec![0]),
            }
        ));
        assert_eq!(row_count(&form, SectionId::W2Box12, &RowAddr(vec![0])), 1);
        // A box-12 row reads back at DEPTH 2: [w2_i, box12_i] = [0, 0].
        assert!(get_at(&form, FieldId::Box12Code, &RowAddr(vec![0, 0])).is_some());

        // box-12 remove: addr = [0, 0].
        assert!(apply_edit(
            &mut form,
            Edit::RemoveRow {
                section: SectionId::W2Box12,
                addr: RowAddr(vec![0, 0]),
            }
        ));
        assert_eq!(row_count(&form, SectionId::W2Box12, &RowAddr(vec![0])), 0);

        // ★ A malformed box-12 `AddRow` (empty parent — too shallow for a depth-2 group) is a clean error,
        // never a panic and never a mutation.
        assert!(!apply_edit(
            &mut form,
            Edit::AddRow {
                section: SectionId::W2Box12,
                parent: RowAddr::default(),
            }
        ));
        assert!(form.error.is_some(), "a bad RowAddr surfaces as form.error");
        assert_eq!(row_count(&form, SectionId::W2Box12, &RowAddr(vec![0])), 0);
    }

    /// (Task 5) Entering a repeating row pushes its index onto `form.addr` so per-row field editing
    /// (Task 3) targets THAT row's address; leaving pops it. A `SetField` while inside W-2 row 0 writes at
    /// `[0]` and reads back there.
    #[test]
    fn entering_a_row_edits_fields_at_the_row_addr() {
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        focus_section(&mut form, SectionId::W2s);
        assert!(add_row(&mut form)); // one W-2, still at the row LIST (addr [])
        assert!(form.addr.0.is_empty(), "add_row leaves us at the row list");
        assert!(matches!(active_pane(&form), Pane::RowList(1)));

        // Enter the row → addr = [0], the field cursor resets to the row's first field.
        enter_selected_row(&mut form);
        assert_eq!(form.addr, RowAddr(vec![0]));
        assert!(matches!(active_pane(&form), Pane::RowFields(_)));

        // Focus Box1Wages within the row and commit $50,000 — it writes at the row addr [0].
        let ri = form.working.as_ref().unwrap();
        let section = live_sections(ri)
            .into_iter()
            .find(|s| s.id == SectionId::W2s)
            .unwrap();
        form.field_focus = live_fields(section, ri)
            .iter()
            .position(|f| f.id == FieldId::Box1Wages)
            .unwrap();
        assert!(tax_inputs_apply_edit(&mut form, "50000"));
        assert_eq!(
            get_at(&form, FieldId::Box1Wages, &RowAddr(vec![0])),
            Some(FieldValue::Money(dec!(50000))),
            "the per-row edit landed at the row address [0]"
        );

        // Leaving pops the index; we're back at the row list on the row we edited.
        assert!(leave_row(&mut form));
        assert!(form.addr.0.is_empty());
        assert_eq!(form.field_focus, 0);
    }

    /// P3-e (integration): `begin_edit` seeds the FULL stored value into `form.buf`, even when it exceeds
    /// `FIELD_CAP`. A CLI/imported occupation can be > 64 chars (Text parse is unbounded); re-opening its
    /// edit buffer must NOT truncate it — truncation would silently re-commit the shortened value. Kills the
    /// `seed`→`set` WIRING mutant (under `set` the buffer caps at 64); the unit KAT alone did not.
    #[test]
    fn begin_edit_seeds_a_long_stored_text_without_truncating_it() {
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));

        // Focus the taxpayer occupation (a Text field) and store a > 64-char value, as CLI/import can.
        focus_section(&mut form, SectionId::Taxpayer);
        form.field_focus = {
            let ri = form.working.as_ref().unwrap();
            let sec = live_sections(ri)
                .into_iter()
                .find(|s| s.id == SectionId::Taxpayer)
                .unwrap();
            live_fields(sec, ri)
                .iter()
                .position(|f| f.id == FieldId::TpOccupation)
                .unwrap()
        };
        let long = "Consultant, ".repeat(8); // 96 chars > FIELD_CAP (64)
        assert!(tax_inputs_apply_edit(&mut form, &long));

        // Re-open the edit buffer on that field — it must seed the FULL stored value.
        begin_edit(&mut form);
        assert_eq!(
            form.buf.as_str(),
            long,
            "P3-e: begin_edit must seed the full stored value; `set` would truncate the tail at FIELD_CAP"
        );
    }

    /// (Task 5) The field-cursor fold: the navigable count matches the DRAWN pane — a repeating row list
    /// navigates ROWS (not the section's 13 W-2 fields), an absent optional-singleton navigates NOTHING.
    #[test]
    fn navigable_count_matches_the_drawn_pane() {
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));

        // W-2s row list: 0 rows → 0 navigable; after two adds → 2 (rows, NOT the 13 W-2 fields).
        focus_section(&mut form, SectionId::W2s);
        assert_eq!(navigable_count(&form), 0);
        assert!(add_row(&mut form));
        assert!(add_row(&mut form));
        assert_eq!(
            navigable_count(&form),
            2,
            "the row list navigates ROWS, not fields"
        );
        assert!(matches!(active_pane(&form), Pane::RowList(2)));

        // Schedule A absent → the [create] pane navigates NOTHING.
        focus_section(&mut form, SectionId::ScheduleA);
        assert_eq!(navigable_count(&form), 0);
        assert!(matches!(active_pane(&form), Pane::Create));
    }

    /// (d) Task 4 — an invalid SSN (`123`) surfaces `BadSsn` in `form.error` and applies NOTHING: the field
    /// stays unset (`Empty`), never a partial or leaked value.
    #[test]
    fn invalid_ssn_sets_error_and_does_not_apply() {
        use btctax_input_form::SecretView;
        let mut form = TaxInputsFormState::fresh(2024);
        assert!(tax_inputs_apply_edit(&mut form, "Single"));
        focus_field(&mut form, SectionId::Taxpayer, FieldId::TpSsn);
        assert!(
            !tax_inputs_apply_edit(&mut form, "123"),
            "a 3-digit SSN must not commit"
        );
        assert!(form.error.is_some(), "BadSsn is surfaced inline");
        assert_eq!(
            focused_value(&form),
            Some(FieldValue::Secret(SecretView::Empty)),
            "the bad entry did not set the field"
        );
    }
}
