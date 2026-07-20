//! Tax-profile form state, field buffers, validation, and the mutation-modal payload.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! This module performs NO writes — it only holds form state and validates input.

use btctax_core::{
    AllocLot, AllocMethod, BasisSource, Carryforward, DisposalProposal, DisposeKind,
    DonationDetails, EventId, FilingStatus, FmvStatus, Form8283Section, InboundClass, IncomeKind,
    LotId, LotMethod, LotPick, ManualFmv, OutflowClass, Persistability, ReclassifyIncome,
    ReclassifyOutflow, Sat, TaxDate, TaxProfile, TransferTarget, Usd, WalletId,
};
use ratatui::widgets::TableState;
use std::str::FromStr;

/// Maximum byte-length of a money field buffer (64 chars is ample for any Decimal).
pub const FIELD_CAP: usize = 64;

/// Byte-length cap for donation **free-text** fields (donee/appraiser name+address,
/// appraiser qualifications, fmv-method override). [R0-N1] A generous BOUND for TUI
/// rendering — the CLI (`Option<String>`) is unbounded; 512 covers realistic
/// addresses / multi-clause qualifications while keeping the buffer render-safe.
pub const FREETEXT_CAP: usize = 512;

/// A single text input buffer.
///
/// Follows the `UnlockState` push/pop discipline (unlock.rs:42–63 — the only
/// text-input precedent): pre-allocated to its `cap`, never reallocates.
/// Rendered **plaintext** (not masked — these are not secrets).
///
/// `cap` is per-instance: money/structured fields keep `FIELD_CAP`; donation
/// free-text fields use `FREETEXT_CAP` (see `with_cap`).
#[derive(Debug)]
pub struct FieldBuffer {
    pub buf: String,
    cap: usize,
}

impl FieldBuffer {
    pub fn new() -> Self {
        Self::with_cap(FIELD_CAP)
    }

    /// Construct a buffer with an explicit byte-length cap (pre-allocated, never reallocates).
    pub fn with_cap(cap: usize) -> Self {
        Self {
            buf: String::with_capacity(cap),
            cap,
        }
    }

    /// Push one character, silently ignoring input past this buffer's `cap`.
    pub fn push_char(&mut self, c: char) {
        if self.buf.len() + c.len_utf8() <= self.cap {
            self.buf.push(c);
        }
    }

    /// Remove the last character (backspace). No-op when empty.
    pub fn pop_char(&mut self) {
        self.buf.pop();
    }

    /// Set the buffer content, respecting this buffer's `cap`.
    pub fn set(&mut self, s: &str) {
        self.buf.clear();
        for c in s.chars() {
            self.push_char(c);
        }
    }

    /// True when byte-length is 0.
    ///
    /// [R0-M4] "empty" = len==0, checked BEFORE any trimming. Whitespace-only is NOT empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// The buffer's current content as a `&str` (for `parse`).
    pub fn as_str(&self) -> &str {
        &self.buf
    }
}

impl Default for FieldBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Field ordering within `ProfileFormState::fields[0..=8]`:
///
/// 0 = ordinary_taxable_income        (REQUIRED)
/// 1 = magi_excluding_crypto          (REQUIRED)
/// 2 = qualified_dividends_and_other_pref_income (REQUIRED)
/// 3 = other_net_capital_gain         (optional, default 0)
/// 4 = carryforward_short             (optional, default 0)
/// 5 = carryforward_long              (optional, default 0)
/// 6 = w2_ss_wages                    (optional, default 0, must be ≥ 0)
/// 7 = w2_medicare_wages              (optional, default 0, must be ≥ 0)
/// 8 = schedule_c_expenses            (optional, default 0, must be ≥ 0)
pub const FIELD_LABELS: [&str; 9] = [
    "ordinary_taxable_income *req",
    "magi_excluding_crypto *req",
    "qualified_dividends_and_other_pref_income *req",
    "other_net_capital_gain",
    "capital_loss_carryforward_in.short",
    "capital_loss_carryforward_in.long",
    "w2_ss_wages (≥0)",
    "w2_medicare_wages (≥0)",
    "schedule_c_expenses (≥0)",
];

/// Live state for the tax-profile form.
///
/// `focus == 0` = filing_status (cycled via Tab); `focus == 1..=9` = money fields.
pub struct ProfileFormState {
    pub year: i32,
    pub filing_status: FilingStatus,
    pub fields: [FieldBuffer; 9],
    pub focus: usize,
    pub error: Option<String>,
}

impl ProfileFormState {
    pub fn new(year: i32) -> Self {
        Self {
            year,
            filing_status: FilingStatus::Single,
            fields: std::array::from_fn(|_| FieldBuffer::new()),
            focus: 0,
            error: None,
        }
    }
}

/// Payload for the per-mutation confirmation modal.
///
/// Contains the VALIDATED profile (not raw buffers) — what will be written, verbatim.
pub struct MutationModalState {
    pub year: i32,
    pub profile: TaxProfile,
}

/// Live state for the "tax inputs" editing flow — a renderer over the `btctax-input-form`
/// engine that drives the `btctax-cli::input_form_store` (plan 3).
///
/// The flow holds a [`btctax_input_form::Working`] (`Option<ReturnInputs>`; `None` until a filing
/// status is chosen — NI-2) and never names a `ReturnInputs` field directly (all access goes through
/// the engine's `form_spec()` accessors, later tasks). It NEVER constructs a `ReturnInputs` — only
/// `apply` materializes one.
///
/// Task 1 carries the minimal skeleton: the opener's `load`-resolved `working`/`parked`/`stale_note`,
/// the current section/row cursor, an inline `error`, and the P2-a `discard_offered` flag. The edit
/// buffer, per-kind editing, autosave/commit, and the source toggle land in later tasks.
pub struct TaxInputsFormState {
    /// The tax year being edited (reuses `EditorApp::selected_year`).
    pub year: i32,
    /// The working return: `None` until a filing status materializes it (NI-2).
    pub working: btctax_input_form::Working,
    /// Index into the live section list (left-pane cursor). Task 2 renders/navigates it.
    pub section_idx: usize,
    /// Index into the SELECTED section's live-field list (field-pane cursor). Task 2 navigates it
    /// (`Up`/`Down`); reset to 0 on a section change (the new section has different fields).
    pub field_focus: usize,
    /// The current row address (`[]` for singletons; `[w2_i]`/`[w2_i, box12_i]` for rows). Task 5.
    pub addr: btctax_input_form::RowAddr,
    /// ★ Task 3: `true` while the focused text-kind field (Money/Text/Date) is being edited — the
    /// `buf` is capturing keystrokes. A second `Enter` commits (parse+apply); `Esc` cancels. Cycle
    /// kinds (Enum/TriState/Bool) never set this — they apply in place on the keypress.
    pub editing: bool,
    /// ★ Task 3: the reused, pre-allocated raw-text edit buffer (`FieldBuffer`, no realloc). Seeded
    /// from the focused field's current value on edit-entry; `parse`d on commit.
    pub buf: FieldBuffer,
    /// Inline error surfaced under the field pane (parse/apply/store failures). `None` when clean.
    pub error: Option<String>,
    /// ★ Task 6 (autosave, I-7): `true` after a SUCCESSFUL mutating `apply` (field/shape edit or the
    /// filing-status materialization) and cleared when the draft is flushed to disk via `save_draft`. The
    /// debounce latch — the flow flushes at flush points ONLY (section change, idle tick, flow close, `q`)
    /// when this is set, NEVER per keystroke.
    pub dirty: bool,
    /// Whether this working copy came from a PARKED committed return (NI-1). Carried across edits.
    pub parked: bool,
    /// The §6.3 stale-WIP-discard note from `load`, if any — Task 2 renders it in the status line.
    pub stale_note: Option<btctax_cli::input_form_store::StaleNote>,
    /// ★ P2-a: `true` when `load` refused a stale PARKED draft (`CliError::StaleParkedDraft`). In this
    /// state the flow renders ONLY the stale-parked message + an 'X' to discard (Task 8) / Esc to back
    /// out — NOT a normal editing form — so the undiscardable parked draft becomes discardable in-app.
    pub discard_offered: bool,
    /// ★ Task 8: the cached `active source: …` label shown in the status line — `"full return"` /
    /// `"tax-profile"` / `"(none)"`, mapped by [`active_source_label`] from `input_form_store::active_source`
    /// (via the `edit::persist::form_active_source` seam). Set at OPEN and refreshed after the one store
    /// mutation that changes the active source while the flow stays open — a `park_to_profile` (commit and
    /// discard both close the flow; autosave/reinstate never change the active source). The render is a pure
    /// `fn(form)` (the profile-form template), and while the flow is open it is the sole store writer under
    /// the exclusive vault lock, so the cache is always consistent with disk after each handler.
    pub active_source_label: &'static str,
    /// ★ Task 5: a staged `RemoveRow` awaiting the payload-confirm ("remove W-2 #2?"). `Some` while the
    /// confirm modal is open — Enter applies it, Esc clears it. It carries the VALIDATED row address (never
    /// a raw cursor), so a later cursor move cannot re-target the delete.
    pub pending_remove: Option<PendingRemove>,
    /// ★ Task-5 fix (nested drill-down): `None` at a section's OWN level; `Some(nested_id)` while descended
    /// INTO a nested repeating group (`W2Box12` under a W-2 row, `ScheduleACharitable` under Schedule A).
    /// It is the ONE extra nav bit that disambiguates "viewing W-2 row `[w2_i]`'s fields" (descent `None`,
    /// `addr = [w2_i]`) from "browsing the box-12 list under `[w2_i]`" (descent `Some(W2Box12)`,
    /// `addr = [w2_i]`) — same `form.addr`, different pane. At a nested sub-list `form.addr` is the group's
    /// PARENT path; at a nested sub-row it is that row's path (one deeper). See `edit/tax_inputs.rs`.
    pub descent: Option<btctax_input_form::SectionId>,
    /// ★ Task 7/8: the payload-confirm modal (`s` commit · `t` park · `X` discard-parked). `Some` while
    /// it is open — Enter runs the confirmed action (dispatched by [`TaxInputsModalState::kind`]), Esc
    /// cancels (writes nothing). NESTED here (review M5), dispatched in `handle_tax_inputs_key` BEFORE the
    /// field keymap. `s` requires `working.is_some()` (else the status "choose a filing status first").
    pub modal: Option<TaxInputsModalState>,
    /// ★ I-4 (SPEC §9A): the SECTION a screen refusal is attributed to — set by [`focus_refusal`] when the
    /// last `commit` returned `Refused` and its anchor resolved to a LIVE in-form section, CLEARED on the
    /// next successful `apply` (the model changed → the refusal is stale) and subsumed by the flow close on
    /// a clean commit. Drives the `!` glyph on that section (left pane) and the status line's `1 issue:
    /// <section>` segment (else `screens clean, except what report computes`). Stores a `SectionId` — a
    /// FormSpec section key, NEVER a `ReturnInputs` leaf (the never-name-a-leaf seam holds).
    pub refused_section: Option<btctax_input_form::SectionId>,
}

/// ★ Task 8: which confirmed action a [`TaxInputsModalState`] gates — one nested modal field serves all
/// three payload-confirms (the `EditorApp` invariant: at most one modal `Some`), with the flow's Enter
/// dispatching by this tag.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxInputsModalKind {
    /// `s` — screen + write the committed `return_inputs` row (Task 7).
    Commit,
    /// `t` — park the committed full return into a `parked = 1` draft and switch to the tax-profile.
    ParkToProfile,
    /// `X` — discard the year's `parked = 1` draft (the ONLY deleter of a parked row).
    DiscardParked,
}

/// ★ Task 7: the COMMIT payload-confirm modal (`s` → this → Enter runs `commit`). A NESTED field on
/// [`TaxInputsFormState`] (`form.modal`), dispatched inside `handle_tax_inputs_key` BEFORE the field
/// keymap — NOT a separate top-level `EditorApp` `Option` (review M5).
///
/// Carries the SUMMARY strings shown before the write — built once at `s`-press from the working return
/// via the `FilingStatus` `get` ACCESSOR ([`filing_status_label`]/`commit_summary`), NEVER `ri.filing_status`
/// (review M6 — Task 9 pins "never names a `ReturnInputs` field"). It holds no raw `ReturnInputs`: the
/// modal's Enter re-reads (clones) the flow's live `working`, so the confirmed payload is what is on screen.
pub struct TaxInputsModalState {
    /// ★ Task 8: which confirmed action this modal gates (commit · park · discard) — the flow's Enter
    /// dispatches on it. `Commit` for the Task-7 flow; `ParkToProfile`/`DiscardParked` for the `t`/`X` toggle.
    pub kind: TaxInputsModalKind,
    /// The tax year being committed.
    pub year: i32,
    /// The chosen filing status label (`"Single"`/`"Mfj"`/…), read via the accessor — the status message
    /// on a clean commit ("committed {year} as {filing_status_label}").
    pub filing_status_label: String,
    /// The multi-line payload summary: the filing status, the sections present (n W-2s, Schedule A?, n
    /// dependents), and — when `shadows` — the shadow + all-zero warning.
    pub summary: String,
    /// Whether a raw `tax_profile` is shadowed by this commit (`shadows_profile(conn, year)`).
    pub shadows: bool,
}

/// A staged repeating-row removal awaiting its payload-confirm (Task 5). Built from the CURRENT row cursor
/// at `d`-press and frozen here, so the confirmed `RemoveRow` deletes exactly the row the prompt named.
pub struct PendingRemove {
    /// The repeating section the row belongs to (`W2s`/`Dependents`/…).
    pub section: btctax_input_form::SectionId,
    /// The full row address to remove (`[w2_i]`; `[w2_i, box12_i]` for the nested box-12 group).
    pub addr: btctax_input_form::RowAddr,
    /// The human-readable payload the confirm shows ("remove W-2 #2?").
    pub label: String,
}

impl TaxInputsFormState {
    /// A fresh flow for `year` with no working return (NI-2: `working = None`). The renderer shows
    /// ONLY the filing-status choice until an `apply` materializes the return. Test/opener helper.
    pub fn fresh(year: i32) -> Self {
        Self {
            year,
            working: None,
            section_idx: 0,
            field_focus: 0,
            addr: btctax_input_form::RowAddr::default(),
            editing: false,
            buf: FieldBuffer::new(),
            error: None,
            dirty: false,
            parked: false,
            stale_note: None,
            discard_offered: false,
            active_source_label: active_source_label(
                &btctax_cli::input_form_store::ActiveSource::Neither,
            ),
            pending_remove: None,
            descent: None,
            modal: None,
            refused_section: None,
        }
    }
}

/// The `active source: …` status-line label for an [`btctax_cli::input_form_store::ActiveSource`] (Task 8):
/// `"full return"` / `"tax-profile"` / `"(none)"`. Used by the opener + the `t`-park handler to cache
/// [`TaxInputsFormState::active_source_label`], which the render prints (keeping the render a pure `fn(form)`).
pub fn active_source_label(a: &btctax_cli::input_form_store::ActiveSource) -> &'static str {
    use btctax_cli::input_form_store::ActiveSource;
    match a {
        ActiveSource::FullReturn => "full return",
        ActiveSource::TaxProfile => "tax-profile",
        ActiveSource::Neither => "(none)",
    }
}

// ── Live-section / live-field projection over the FormSpec (shared by the renderer + the nav) ──────────
//
// ★ All access goes through `form_spec()` accessors — never a `ReturnInputs` struct field (spec §9A/§13,
// Task 9 tests it). The Spouse-visibility + nested-skip rules are TUI-side knowledge the engine's
// `OptionalSingleton` does not carry (review I-2).

/// The `FilingStatus` field, located by its stable `FieldId` in `form_spec()`.
pub fn filing_status_field() -> &'static btctax_input_form::Field {
    btctax_input_form::form_spec()
        .iter()
        .flat_map(|s| s.fields.iter())
        .find(|f| f.id == btctax_input_form::FieldId::FilingStatus)
        .expect("FilingStatus field is present in form_spec()")
}

/// The chosen filing status as its stable enum name (`"Single"`/`"Mfj"`/…), read via the accessor —
/// NEVER `ri.filing_status` (spec §9A/§13).
fn filing_status_name(ri: &btctax_core::tax::return_inputs::ReturnInputs) -> Option<String> {
    match (filing_status_field().get)(ri, &btctax_input_form::RowAddr::default()) {
        Some(btctax_input_form::FieldValue::Choice(c)) => Some(c),
        _ => None,
    }
}

/// The commit modal's `filing_status_label` — the chosen filing status as its stable label, read via the
/// [`filing_status_field`] `get` accessor, NEVER `ri.filing_status` (review M6 — spec §9A/§13; Task 9
/// pins it). A materialized working copy always has one; the `(unset)` fallback is belt-and-suspenders.
pub fn filing_status_label(ri: &btctax_core::tax::return_inputs::ReturnInputs) -> String {
    filing_status_name(ri).unwrap_or_else(|| "(unset)".to_string())
}

/// Whether the Spouse section is OFFERED for this return — MFJ/MFS/QSS only (hidden on Single/HoH).
/// TUI-side gate (review I-2); reads the filing status via the accessor.
fn spouse_offered(ri: &btctax_core::tax::return_inputs::ReturnInputs) -> bool {
    matches!(
        filing_status_name(ri).as_deref(),
        Some("Mfj") | Some("Mfs") | Some("Qss")
    )
}

/// Is this section shown as a top-level left-pane entry for `ri`?
/// - `W2Box12` / `ScheduleACharitable`: logically nested (Task 5 renders them within their parent's
///   rows) → skipped here.
/// - `Spouse`: offered only for MFJ/MFS/QSS.
/// - every other section (`Singleton`, `Repeating`, `ScheduleA`) is always shown.
fn section_is_live(
    section: &btctax_input_form::Section,
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
) -> bool {
    use btctax_input_form::SectionId;
    match section.id {
        SectionId::W2Box12 | SectionId::ScheduleACharitable => false,
        SectionId::Spouse => spouse_offered(ri),
        _ => true,
    }
}

/// The sections a renderer lists for this working return, in spec §9A order.
pub fn live_sections(
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
) -> Vec<&'static btctax_input_form::Section> {
    btctax_input_form::form_spec()
        .iter()
        .filter(|s| section_is_live(s, ri))
        .collect()
}

/// The live fields of a section for this return (`field.live(ri)`).
pub fn live_fields(
    section: &'static btctax_input_form::Section,
    ri: &btctax_core::tax::return_inputs::ReturnInputs,
) -> Vec<&'static btctax_input_form::Field> {
    section.fields.iter().filter(|f| (f.live)(ri)).collect()
}

/// Cycle through the 5 `FilingStatus` variants in declaration order.
/// Tab from the last wraps back to the first.
pub fn cycle_filing_status(fs: FilingStatus) -> FilingStatus {
    match fs {
        FilingStatus::Single => FilingStatus::Mfj,
        FilingStatus::Mfj => FilingStatus::Mfs,
        FilingStatus::Mfs => FilingStatus::HoH,
        FilingStatus::HoH => FilingStatus::Qss,
        FilingStatus::Qss => FilingStatus::Single,
    }
}

/// Cycle the safe-harbor allocation `AllocMethod` (mirrors `cycle_filing_status`). The toggle changes
/// ONLY the recorded `method` tag — the displayed residue lots are method-INDEPENDENT (gotcha G3):
/// both `ActualPosition` and `ProRata` seed from the SAME per-wallet actuals; the method affects only
/// the engine's timebar/effectiveness rule, never the lot list. ProRata cross-wallet redistribution is
/// NOT implemented (core O4) — the tag records the election; it does not redistribute basis.
pub fn cycle_alloc_method(m: AllocMethod) -> AllocMethod {
    match m {
        AllocMethod::ActualPosition => AllocMethod::ProRata,
        AllocMethod::ProRata => AllocMethod::ActualPosition,
    }
}

/// Validate the form and return a `TaxProfile` or an error string.
///
/// Mirrors the CLI's clap-side rules (main.rs:688–760) EXACTLY:
/// - Rule 1: filing_status always valid (structural)
/// - Rules 2–4: required fields (empty = len-0 → "... is required"; else parse)
/// - Rules 5–7: optional (empty → 0; else parse; negatives accepted — CLI parity)
/// - Rules 8–10: optional (empty → 0; else parse; negative → error)
///
/// [R0-M4] "empty" = byte-len 0, checked BEFORE trimming. Whitespace-only → parse error.
pub fn validate(form: &ProfileFormState) -> Result<TaxProfile, String> {
    let oti = parse_required(&form.fields[0], "ordinary-taxable-income")?;
    let magi = parse_required(&form.fields[1], "magi-excluding-crypto")?;
    let qd = parse_required(&form.fields[2], "qualified-dividends-and-other-pref-income")?;

    let oncg = parse_optional(&form.fields[3])?;
    let cf_short = parse_optional(&form.fields[4])?;
    let cf_long = parse_optional(&form.fields[5])?;

    let w2_ss = parse_optional(&form.fields[6])?;
    if w2_ss.is_sign_negative() {
        return Err("w2-ss-wages must not be negative".to_string());
    }
    let w2_medicare = parse_optional(&form.fields[7])?;
    if w2_medicare.is_sign_negative() {
        return Err("w2-medicare-wages must not be negative".to_string());
    }
    let sce = parse_optional(&form.fields[8])?;
    if sce.is_sign_negative() {
        return Err("schedule-c-expenses must not be negative".to_string());
    }

    Ok(TaxProfile {
        filing_status: form.filing_status,
        ordinary_taxable_income: oti,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: qd,
        other_net_capital_gain: oncg,
        capital_loss_carryforward_in: Carryforward {
            short: cf_short,
            long: cf_long,
        },
        w2_ss_wages: w2_ss,
        w2_medicare_wages: w2_medicare,
        schedule_c_expenses: sce,
    })
}

/// Parse a REQUIRED field: byte-len-0 → "name is required"; else Decimal::from_str(trim).
fn parse_required(buf: &FieldBuffer, name: &str) -> Result<Usd, String> {
    if buf.is_empty() {
        return Err(format!("{name} is required"));
    }
    let trimmed = buf.buf.trim();
    Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed}"))
}

/// Parse an OPTIONAL field: byte-len-0 → 0; else Decimal::from_str(trim).
fn parse_optional(buf: &FieldBuffer) -> Result<Usd, String> {
    if buf.is_empty() {
        return Ok(Usd::ZERO);
    }
    let trimmed = buf.buf.trim();
    Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed}"))
}

// ── TargetList widget (shared by classify-inbound and reclassify-outflow) ─────

/// Selectable list of actionable targets rendered as a `ratatui` Table.
///
/// Callers (flow-open code) guarantee `items` is non-empty — an empty filtered
/// list never opens a flow [R0-M8]. The render's defensive "no items" placeholder
/// and Enter-swallow are belt-and-suspenders; they are unreachable under the
/// flow-open rule and carry no KAT.
pub struct TargetList<T> {
    pub items: Vec<T>,
    pub table_state: TableState,
}

impl<T> TargetList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut table_state = TableState::default();
        if !items.is_empty() {
            table_state.select(Some(0));
        }
        Self { items, table_state }
    }

    pub fn selected(&self) -> Option<&T> {
        self.table_state.selected().and_then(|i| self.items.get(i))
    }

    pub fn scroll_up(&mut self) {
        let next = match self.table_state.selected() {
            Some(i) if i > 0 => Some(i - 1),
            Some(_) => Some(0),
            None => None,
        };
        self.table_state.select(next);
    }

    pub fn scroll_down(&mut self) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let next = match self.table_state.selected() {
            Some(i) => Some((i + 1).min(count - 1)),
            None => Some(0),
        };
        self.table_state.select(next);
    }

    pub fn go_top(&mut self) {
        if !self.items.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn go_bottom(&mut self) {
        let count = self.items.len();
        if count > 0 {
            self.table_state.select(Some(count - 1));
        }
    }
}

// ── Display data types for list items ─────────────────────────────────────────

/// Pre-computed display data for a classify-inbound list row.
#[derive(Clone)]
pub struct InboundListItem {
    /// The TransferIn event targeted by the `UnknownBasisInbound` blocker.
    pub blocker_event: EventId,
    /// Calendar date (tax timezone) of the TransferIn event.
    pub date: TaxDate,
    /// Principal sat from the TransferIn payload.
    pub sat: Sat,
    /// Wallet of the TransferIn event (None → displayed as "(no wallet)").
    pub wallet: Option<WalletId>,
    /// Blocker detail string.
    pub detail: String,
}

/// Pre-computed display data for a reclassify-outflow list row.
/// Added here per the spec Task-1 file list; fully used in Task 2 [R0-N3].
#[allow(dead_code)]
#[derive(Clone)]
pub struct OutflowListItem {
    /// The `PendingTransfer.event` EventId.
    pub transfer_out_event: EventId,
    /// Calendar date (tax timezone) of the TransferOut event.
    pub date: TaxDate,
    /// Principal sat from `PendingTransfer.principal_sat`.
    pub principal_sat: Sat,
    /// Wallet of the TransferOut event.
    pub wallet: Option<WalletId>,
}

// ── Classify-inbound flow types ───────────────────────────────────────────────

/// Which variant the picker is showing for classify-inbound step 2.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InboundVariant {
    Income,
    GiftReceived,
    /// Cycle A: "my own coins" — a non-taxable inbound self-transfer.
    SelfTransferMine,
}

/// Step in the classify-inbound flow.
pub enum ClassifyInboundStep {
    List,
    VariantPicker {
        item: InboundListItem,
        variant: InboundVariant,
    },
    IncomeForm {
        item: InboundListItem,
        /// Current IncomeKind selection; initial: Mining [R0-M3].
        kind: IncomeKind,
        fmv_buf: FieldBuffer,
        /// Default: false (CLI parity).
        business: bool,
        /// 0 = kind (Tab-cycles), 1 = fmv (text), 2 = business (Space toggle).
        focus: usize,
        error: Option<String>,
    },
    GiftForm {
        item: InboundListItem,
        fmv_at_gift_buf: FieldBuffer,
        donor_basis_buf: FieldBuffer,
        donor_acquired_at_buf: FieldBuffer,
        /// 0 = fmv_at_gift, 1 = donor_basis, 2 = donor_acquired_at.
        focus: usize,
        error: Option<String>,
    },
    /// Cycle A: inbound self-transfer ("my own coins"). Both fields optional — empty basis defaults to
    /// $0 (conservative) and fires the honest advisory; empty acquired defaults to 1yr+1day before receipt (long-term).
    SelfTransferForm {
        item: InboundListItem,
        basis_buf: FieldBuffer,
        acquired_buf: FieldBuffer,
        /// 0 = basis (USD, optional), 1 = acquired_at (YYYY-MM-DD, optional).
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the classify-inbound flow.  Owns its target list.
pub struct ClassifyInboundFlowState {
    /// Owned list — no standalone list field on EditorApp [R0-I2].
    pub list: TargetList<InboundListItem>,
    pub step: ClassifyInboundStep,
}

/// Payload for the classify-inbound confirmation modal.
pub struct ClassifyInboundModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    /// The VALIDATED classification — what will be persisted.
    pub as_: InboundClass,
}

// ── Helper: IncomeKind cycling and display ────────────────────────────────────

/// Cycle through the 5 `IncomeKind` variants in declaration order (event.rs:29–35).
/// Mining → Staking → Interest → Airdrop → Reward → Mining.
pub fn cycle_income_kind(kind: IncomeKind) -> IncomeKind {
    match kind {
        IncomeKind::Mining => IncomeKind::Staking,
        IncomeKind::Staking => IncomeKind::Interest,
        IncomeKind::Interest => IncomeKind::Airdrop,
        IncomeKind::Airdrop => IncomeKind::Reward,
        IncomeKind::Reward => IncomeKind::Mining,
    }
}

/// Return the lowercase display tag for an `IncomeKind` (matches CLI render convention).
pub fn income_kind_display(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::Mining => "mining",
        IncomeKind::Staking => "staking",
        IncomeKind::Interest => "interest",
        IncomeKind::Airdrop => "airdrop",
        IncomeKind::Reward => "reward",
    }
}

// ── Classify-inbound validation ───────────────────────────────────────────────

/// UX-P4-4(a) both-surfaces: parse a REQUIRED non-negative USD field. Same `Usd::from_str`
/// (Decimal) semantics as before — plus the per-flag sign guard the CLI applies via
/// `eventref::parse_nonneg_usd_arg`. No legitimate negative cost basis / FMV / fee / proceeds exists
/// (§1012; §1016 floors adjustments at zero), so a `-5000` that would ride into gain math (gain >
/// proceeds) and onto a filed form is refused HERE, at the TUI record surface, exactly as on the CLI.
/// `s` is the already-trimmed field text; `label` names the field in the refusal.
fn parse_nonneg_usd(label: &str, s: &str) -> Result<Usd, String> {
    let v = Usd::from_str(s).map_err(|_| format!("bad USD {s:?}"))?;
    if v < Usd::ZERO {
        return Err(format!("{label} must be >= 0 (got {v})"));
    }
    Ok(v)
}

/// UX-P4-4(b) both-surfaces: refuse an acquisition date STRICTLY after the receipt date — coins
/// cannot have been acquired after they arrived. Same-day is allowed (a same-day acquire→receive is
/// legitimate). `receipt` is the TransferIn's tax-tz calendar date, carried on the flow's list item.
fn check_acquired_not_after_receipt(
    label: &str,
    acquired: Option<TaxDate>,
    receipt: TaxDate,
) -> Result<(), String> {
    if let Some(d) = acquired {
        if d > receipt {
            return Err(format!(
                "{label} {d} is after the receipt date {receipt}; coins cannot be acquired after \
                 they are received (same-day is allowed)"
            ));
        }
    }
    Ok(())
}

/// Validate the Income variant of the classify-inbound form.
///
/// `kind` is always structurally valid (picker).  `fmv_buf` is optional:
/// empty (byte-len 0) → `None`; non-empty → `parse_usd_arg(trim)`.
/// [R0-M4] whitespace-only is NOT empty.
///
/// Returns the validated `InboundClass::Income` or an error string.
pub fn validate_classify_inbound_income(
    kind: IncomeKind,
    fmv_buf: &FieldBuffer,
    business: bool,
) -> Result<InboundClass, String> {
    let fmv = if fmv_buf.is_empty() {
        None
    } else {
        Some(parse_nonneg_usd("fmv", fmv_buf.buf.trim())?)
    };
    Ok(InboundClass::Income {
        kind,
        fmv,
        business,
    })
}

/// Validate the GiftReceived variant of the classify-inbound form.
///
/// `fmv_at_gift_buf` is REQUIRED (empty → "fmv-at-gift is required").
/// `donor_basis_buf` and `donor_acquired_at_buf` are optional.
///
/// Date format: YYYY-MM-DD (`parse_date_arg` semantics — `Date::parse(trim, "[year]-[month]-[day]")`).
/// [R0-M4] whitespace-only is NOT empty.
///
/// Returns the validated `InboundClass::GiftReceived` or an error string.
pub fn validate_classify_inbound_gift(
    receipt: TaxDate,
    fmv_at_gift_buf: &FieldBuffer,
    donor_basis_buf: &FieldBuffer,
    donor_acquired_at_buf: &FieldBuffer,
) -> Result<InboundClass, String> {
    if fmv_at_gift_buf.is_empty() {
        return Err("fmv-at-gift is required".to_string());
    }
    let fmv_at_gift = parse_nonneg_usd("fmv-at-gift", fmv_at_gift_buf.buf.trim())?;

    let donor_basis = if donor_basis_buf.is_empty() {
        None
    } else {
        Some(parse_nonneg_usd("donor-basis", donor_basis_buf.buf.trim())?)
    };

    let donor_acquired_at = if donor_acquired_at_buf.is_empty() {
        None
    } else {
        let t = donor_acquired_at_buf.buf.trim();
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        Some(time::Date::parse(t, fmt).map_err(|e| format!("bad date {t:?}: {e}"))?)
    };
    check_acquired_not_after_receipt("donor-acquired", donor_acquired_at, receipt)?;

    Ok(InboundClass::GiftReceived {
        donor_basis,
        donor_acquired_at,
        fmv_at_gift,
    })
}

/// Validate the SelfTransferMine variant of the classify-inbound form (Cycle A).
///
/// Both fields are OPTIONAL:
///   `basis_buf`    — empty → `None` ($0 default + honest advisory in the fold); else `parse_usd_arg(trim)`.
///   `acquired_buf` — empty → `None` (receipt-date default); else `parse_date_arg(trim)` (YYYY-MM-DD).
/// [R0-M4] whitespace-only is NOT empty (it parse-errors, never silently `None`). An explicit `0` basis
/// parses to `Some(0)` — an attested zero cost, honored WITHOUT the advisory (the flag keys on `None`).
///
/// Returns the validated `InboundClass::SelfTransferMine` or an error string.
pub fn validate_classify_inbound_self_transfer(
    receipt: TaxDate,
    basis_buf: &FieldBuffer,
    acquired_buf: &FieldBuffer,
) -> Result<InboundClass, String> {
    let basis = if basis_buf.is_empty() {
        None
    } else {
        Some(parse_nonneg_usd("basis", basis_buf.buf.trim())?)
    };

    let acquired_at = if acquired_buf.is_empty() {
        None
    } else {
        let t = acquired_buf.buf.trim();
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        Some(time::Date::parse(t, fmt).map_err(|e| format!("bad date {t:?}: {e}"))?)
    };
    check_acquired_not_after_receipt("acquired", acquired_at, receipt)?;

    Ok(InboundClass::SelfTransferMine { basis, acquired_at })
}

// ── Reclassify-outflow flow types ────────────────────────────────────────────

/// Which outflow kind is selected in the reclassify-outflow kind picker.
///
/// Tab cycles: Sell → Spend → Gift → Donate → Sell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutflowKind {
    Sell,
    Spend,
    Gift,
    Donate,
}

/// Cycle through the 4 `OutflowKind` variants in picker order.
/// Sell → Spend → Gift → Donate → Sell.
pub fn cycle_outflow_kind(kind: OutflowKind) -> OutflowKind {
    match kind {
        OutflowKind::Sell => OutflowKind::Spend,
        OutflowKind::Spend => OutflowKind::Gift,
        OutflowKind::Gift => OutflowKind::Donate,
        OutflowKind::Donate => OutflowKind::Sell,
    }
}

/// Step in the reclassify-outflow flow.
pub enum ReclassifyOutflowStep {
    List,
    KindPicker {
        item: OutflowListItem,
        /// Initial: Sell.
        kind: OutflowKind,
    },
    FieldForm {
        item: OutflowListItem,
        kind: OutflowKind,
        /// Gross proceeds (sell/spend) or FMV (gift/donate); REQUIRED.
        amount_buf: FieldBuffer,
        /// Network fee; optional.
        fee_buf: FieldBuffer,
        /// Appraisal required toggle; donate only (default false).
        appraisal: bool,
        /// Donee free-form label; optional; gift and donate only.
        donee_buf: FieldBuffer,
        /// Focus: 0=amount, 1=fee, 2=appraisal (donate only), 3=donee (gift/donate only).
        /// Hidden rows are skipped in focus cycling.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the reclassify-outflow flow. Owns its target list [R0-I2].
pub struct ReclassifyOutflowFlowState {
    /// Owned list — no standalone list field on EditorApp.
    pub list: TargetList<OutflowListItem>,
    pub step: ReclassifyOutflowStep,
}

/// Payload for the reclassify-outflow confirmation modal.
pub struct ReclassifyOutflowModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub principal_sat: Sat,
    /// The VALIDATED payload — what will be persisted.
    pub payload: ReclassifyOutflow,
}

/// Return the label for the `amount` field based on the outflow kind.
///
/// [R0-I3]: gross proceeds for sell AND spend; FMV for gift/donate.
pub fn amount_label(kind: OutflowKind) -> &'static str {
    match kind {
        OutflowKind::Sell | OutflowKind::Spend => "gross proceeds (USD)",
        OutflowKind::Gift | OutflowKind::Donate => "FMV (USD)",
    }
}

/// Compute the maximum focus index for the reclassify-outflow field form.
///
/// Focus order: 0=amount, 1=fee, 2=appraisal (donate only), 3=donee (gift/donate only).
/// Hidden rows are skipped: for sell/spend max focus is 1; for gift max is 3 (skipping 2);
/// for donate max is 3.
pub fn max_focus_for_kind(kind: OutflowKind) -> usize {
    match kind {
        OutflowKind::Sell | OutflowKind::Spend => 1,
        OutflowKind::Gift | OutflowKind::Donate => 3,
    }
}

/// Step to the next visible focus row (skipping hidden rows).
pub fn next_focus(focus: usize, kind: OutflowKind) -> usize {
    let max = max_focus_for_kind(kind);
    let next = (focus + 1).min(max);
    // Row 2 (appraisal) is only shown for donate; skip for gift.
    if next == 2 && kind == OutflowKind::Gift {
        3.min(max)
    } else {
        next
    }
}

/// Step to the previous visible focus row (skipping hidden rows).
pub fn prev_focus(focus: usize, kind: OutflowKind) -> usize {
    if focus == 0 {
        return 0;
    }
    let prev = focus - 1;
    // Row 2 (appraisal) is only shown for donate; skip for gift.
    if prev == 2 && kind == OutflowKind::Gift {
        1
    } else {
        prev
    }
}

// ── Reclassify-outflow validation ────────────────────────────────────────────

/// Validate the reclassify-outflow field form and build a `ReclassifyOutflow` payload.
///
/// `amount` is REQUIRED (empty byte-len-0 → "amount is required"; whitespace-only → parse error).
/// `fee` is optional (empty → `None`). `appraisal` is a toggle bool (always valid).
/// `donee` is optional free-form text; empty → `None`; non-empty → `Some(trim().to_owned())`.
/// [R0-M4] whitespace-only is NOT empty.
/// [R0-I3] `amount` is gross proceeds for sell/spend; FMV for gift/donate — same field, different label.
///
/// Returns the validated `ReclassifyOutflow` or an error string.
pub fn validate_reclassify_outflow(
    item: &OutflowListItem,
    kind: OutflowKind,
    amount_buf: &FieldBuffer,
    fee_buf: &FieldBuffer,
    appraisal: bool,
    donee_buf: &FieldBuffer,
) -> Result<ReclassifyOutflow, String> {
    // amount: REQUIRED
    if amount_buf.is_empty() {
        return Err("amount is required".to_string());
    }
    let principal_proceeds_or_fmv = parse_nonneg_usd("amount", amount_buf.buf.trim())?;

    // fee: optional
    let fee_usd = if fee_buf.is_empty() {
        None
    } else {
        Some(parse_nonneg_usd("fee", fee_buf.buf.trim())?)
    };

    // donee: optional free-form; trimmed + capped at FIELD_CAP
    let donee = if donee_buf.is_empty() {
        None
    } else {
        Some(donee_buf.buf.trim().to_owned())
    };

    // Build OutflowClass
    let as_ = match kind {
        OutflowKind::Sell => OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        },
        OutflowKind::Spend => OutflowClass::Dispose {
            kind: DisposeKind::Spend,
        },
        OutflowKind::Gift => OutflowClass::GiftOut,
        OutflowKind::Donate => OutflowClass::Donate {
            appraisal_required: appraisal,
        },
    };

    Ok(ReclassifyOutflow {
        transfer_out_event: item.transfer_out_event.clone(),
        as_,
        principal_proceeds_or_fmv,
        fee_usd,
        donee,
    })
}

// ── Reclassify-income flow types ─────────────────────────────────────────────

/// Pre-computed display data for a reclassify-income list row.
#[derive(Clone)]
pub struct IncomeListItem {
    pub income_event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: IncomeKind,
    pub business: bool,
    /// FMV from income_recognized if present; None if FmvMissing.
    pub fmv: Option<Usd>,
    pub wallet: Option<WalletId>,
}

/// Pre-computed display data for a set-fmv list row.
#[derive(Clone)]
pub struct FmvListItem {
    pub event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: IncomeKind,
    pub wallet: Option<WalletId>,
}

/// Step in the reclassify-income flow.
pub enum ReclassifyIncomeStep {
    List,
    FieldForm {
        item: IncomeListItem,
        /// 3-state: None = not chosen (REQUIRED-EXPLICIT).
        business: Option<bool>,
        /// None = keep original.
        kind: Option<IncomeKind>,
        /// 0 = business, 1 = kind.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the reclassify-income flow.
pub struct ReclassifyIncomeFlowState {
    pub list: TargetList<IncomeListItem>,
    pub step: ReclassifyIncomeStep,
}

/// Payload for the reclassify-income confirmation modal.
pub struct ReclassifyIncomeModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub original_kind: IncomeKind,
    pub original_business: bool,
    pub new_business: bool,
    pub new_kind: Option<IncomeKind>,
}

/// Step in the set-fmv flow.
pub enum SetFmvStep {
    List,
    FieldForm {
        item: FmvListItem,
        usd_fmv_buf: FieldBuffer,
        error: Option<String>,
    },
}

/// Full state for the set-fmv flow.
pub struct SetFmvFlowState {
    pub list: TargetList<FmvListItem>,
    pub step: SetFmvStep,
}

/// Payload for the set-fmv confirmation modal.
pub struct SetFmvModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub target_kind: IncomeKind,
    pub usd_fmv: Usd,
}

// ── Reclassify-income validation ─────────────────────────────────────────────

/// Validate the reclassify-income field form.
///
/// `business`: None → "business is required (press Tab to choose true or false)".
/// `kind`: always valid (None = keep original).
///
/// Returns the validated `EventPayload::ReclassifyIncome(…)` or an error string.
pub fn validate_reclassify_income(
    item: &IncomeListItem,
    business: Option<bool>,
    kind: Option<IncomeKind>,
) -> Result<btctax_core::EventPayload, String> {
    let b = match business {
        None => return Err("business is required (press Tab to choose true or false)".to_string()),
        Some(b) => b,
    };
    Ok(btctax_core::EventPayload::ReclassifyIncome(
        ReclassifyIncome {
            income_event: item.income_event.clone(),
            business: b,
            kind,
        },
    ))
}

/// Validate the set-fmv field form.
///
/// `usd_fmv_buf`: empty (len==0) → "usd-fmv is required"; non-empty → parse.
///
/// Returns the validated `EventPayload::ManualFmv(…)` or an error string.
pub fn validate_set_fmv(
    item: &FmvListItem,
    usd_fmv_buf: &FieldBuffer,
) -> Result<btctax_core::EventPayload, String> {
    if usd_fmv_buf.is_empty() {
        return Err("usd-fmv is required".to_string());
    }
    let usd_fmv = parse_nonneg_usd("usd-fmv", usd_fmv_buf.buf.trim())?;
    Ok(btctax_core::EventPayload::ManualFmv(ManualFmv {
        event: item.event.clone(),
        usd_fmv,
    }))
}

/// Cycle through IncomeKind with an extra None state (None = keep original).
///
/// None → Mining → Staking → Interest → Airdrop → Reward → None.
pub fn cycle_income_kind_optional(kind: Option<IncomeKind>) -> Option<IncomeKind> {
    match kind {
        None => Some(IncomeKind::Mining),
        Some(IncomeKind::Mining) => Some(IncomeKind::Staking),
        Some(IncomeKind::Staking) => Some(IncomeKind::Interest),
        Some(IncomeKind::Interest) => Some(IncomeKind::Airdrop),
        Some(IncomeKind::Airdrop) => Some(IncomeKind::Reward),
        Some(IncomeKind::Reward) => None,
    }
}

/// Cycle through the business 3-state (None → true → false → None).
pub fn cycle_business_optional(b: Option<bool>) -> Option<bool> {
    match b {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
}

// ── Method-election flow types (§A.5(a) per-account cost-basis method) ─────────

/// Cycle through the lot method (Fifo → Hifo → Lifo → Fifo). The chosen method is REQUIRED — there is
/// no None state (unlike the reclassify-income pickers): the flow seeds the picker to the account's
/// currently-resolved method, so Enter always has a concrete method.
pub fn cycle_lot_method(m: LotMethod) -> LotMethod {
    match m {
        LotMethod::Fifo => LotMethod::Hifo,
        LotMethod::Hifo => LotMethod::Lifo,
        LotMethod::Lifo => LotMethod::Fifo,
    }
}

/// Human-readable method label for the list/modal (FIFO/HIFO/LIFO).
pub fn lot_method_label(m: LotMethod) -> &'static str {
    match m {
        LotMethod::Fifo => "FIFO",
        LotMethod::Hifo => "HIFO",
        LotMethod::Lifo => "LIFO",
    }
}

/// Pre-computed display data for a method-election account row: an Exchange account, its
/// currently-resolved method, and whether that method is an explicit per-account election (`scoped`)
/// or inherited from a global election / the FIFO default.
#[derive(Clone)]
pub struct MethodElectionListItem {
    pub wallet: WalletId,
    pub current: LotMethod,
    pub scoped: bool,
}

/// Step in the method-election flow.
pub enum MethodElectionStep {
    List,
    Choose {
        item: MethodElectionListItem,
        /// The method being elected — seeded to `item.current`; Tab cycles it. Always concrete.
        method: LotMethod,
        error: Option<String>,
    },
}

/// Full state for the method-election flow.
pub struct MethodElectionFlowState {
    pub list: TargetList<MethodElectionListItem>,
    pub step: MethodElectionStep,
}

/// Payload for the method-election confirmation ("attest") modal.
pub struct MethodElectionModalState {
    pub wallet: WalletId,
    pub method: LotMethod,
}

// ── Void flow types ───────────────────────────────────────────────────────────

/// Pre-computed display data for a void-decision list row.
#[derive(Clone)]
pub struct VoidListItem {
    /// The decision EventId being offered for void.
    pub event_id: EventId,
    /// Sequence number (for display in the Seq column).
    pub seq: u64,
    /// Static payload type tag ("TransferLink", "ClassifyInbound", etc.).
    pub payload_tag: &'static str,
    /// Human-readable target summary computed at list-open time.
    pub target_summary: String,
    /// The decision's OWN inner target event (used by `derive_void_status`'s
    /// returned-blocker check). None for MethodElection and SafeHarborAllocation.
    pub inner_target: Option<EventId>,
}

/// Step in the void flow. Only `List` — Enter goes DIRECTLY to `VoidModalState`.
pub enum VoidStep {
    List,
}

/// Full state for the void flow. Owns its target list [R0-I2 discipline].
pub struct VoidFlowState {
    /// Owned target list — no standalone list field on EditorApp.
    pub list: TargetList<VoidListItem>,
    pub step: VoidStep,
}

/// Payload for the void confirmation modal.
pub struct VoidModalState {
    /// The decision EventId being voided.
    pub target_event_id: EventId,
    pub seq: u64,
    pub payload_tag: &'static str,
    pub target_summary: String,
    /// Carried from VoidListItem for derive_void_status [M5].
    pub inner_target: Option<EventId>,
    /// True when the target is SafeHarborAllocation — triggers the conditional warning.
    pub is_safe_harbor: bool,
}

// The voidable-decision predicate now lives in btctax-core (`btctax_core::is_revocable_payload`) so
// the CLI (`Session::bulk_void_plan`) and the TUI share ONE copy (SPEC_bulk_void Task 1). Callers
// reference it directly via `btctax_core::is_revocable_payload` (was `edit::form::is_revocable_payload`).

// ── Select-lots flow types ────────────────────────────────────────────────────

/// Display kind for a disposal-list row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisposalKind {
    Sell,
    Spend,
    Gift,
    Donate,
    /// #1: a TransferOut projected to `Op::SelfTransfer` by a non-voided TransferLink.
    /// Reconstructed in-TUI (not in `snap.state.disposals`/`removals`); `principal_sat` =
    /// `TransferOut.sat` (fee excluded — matches `honoring_principal`, resolve.rs:211-214).
    SelfTransfer,
}

/// Pre-computed display data for a select-lots disposal list row.
///
/// `wallet` is ALWAYS sourced from the raw `LedgerEvent.wallet` via `events_by_id` [R0-I1].
/// `RemovalLeg` has no wallet field — Gift/Donate rows would have no wallet source otherwise.
#[derive(Clone)]
pub struct DisposalListItem {
    pub disposal_event: EventId,
    pub date: TaxDate,
    pub kind: DisposalKind,
    /// Principal sat = Σ legs.sat (from Disposal or Removal).
    pub principal_sat: Sat,
    /// From the raw `LedgerEvent.wallet` via `events_by_id`.
    pub wallet: Option<WalletId>,
}

/// A single lot-pick row in the LotsForm.
pub struct LotPickFormRow {
    pub lot_id: LotId,
    pub remaining_sat: Sat,
    pub acquired_at: TaxDate,
    pub usd_basis: Usd,
    /// Editable sat amount (digits only; initially empty = 0).
    pub pick_sat_buf: FieldBuffer,
}

impl LotPickFormRow {
    /// Parse pick_sat_buf as i64; returns 0 when buffer is empty.
    pub fn pick_sat(&self) -> Result<Sat, String> {
        if self.pick_sat_buf.is_empty() {
            return Ok(0);
        }
        self.pick_sat_buf.buf.trim().parse::<i64>().map_err(|e| {
            format!(
                "bad sat in row {}: {e}",
                self.lot_id.origin_event_id.canonical()
            )
        })
    }
}

/// Step in the select-lots flow.
pub enum SelectLotsStep {
    List,
    LotsForm {
        item: DisposalListItem,
        rows: Vec<LotPickFormRow>,
        /// Focused row index.
        cursor: usize,
        error: Option<String>,
    },
}

/// Full state for the select-lots flow.
pub struct SelectLotsFlowState {
    /// Owned by the flow [R0-I2 discipline].
    pub list: TargetList<DisposalListItem>,
    pub step: SelectLotsStep,
}

/// Payload for the select-lots confirmation modal.
pub struct SelectLotsModalState {
    pub disposal_event: EventId,
    pub disposal_date: TaxDate,
    pub disposal_kind: DisposalKind,
    pub principal_sat: Sat,
    /// Validated picks (non-zero only).
    pub picks: Vec<btctax_core::LotPick>,
    pub pick_count: usize,
    /// Σ picks.sat (== principal_sat by construction).
    pub total_sat: Sat,
}

/// Validate the LotsForm at Enter-press.
///
/// Returns `EventPayload::LotSelection(…)` or an error string.
///
/// Validation rules (spec D1):
/// 1. Parse every `pick_sat_buf` → error on any non-integer.
/// 2. Collect rows with `pick_sat > 0` → `Vec<LotPick>`.
/// 3. If no picks (all zero) → `"pick at least one lot"`.
/// 4. `Σ picked_sat` must equal `item.principal_sat`.
pub fn validate_select_lots(
    item: &DisposalListItem,
    rows: &[LotPickFormRow],
) -> Result<btctax_core::EventPayload, String> {
    // Step 1: parse every buffer, capping each pick at its row's at-disposal Remaining.
    let mut total: Sat = 0;
    let mut picks: Vec<btctax_core::LotPick> = Vec::new();
    for row in rows {
        let sat = row.pick_sat()?;
        // Step 1b [SL-r2-b / review r2 M-1]: per-row cap. `remaining_sat` IS the at-disposal availability
        // (the form is built from `available_lots_before`), so a pick above it is a lot the engine would
        // reject — refuse it here rather than persist a doomed selection that fails `selection_feasible`.
        if sat > row.remaining_sat {
            return Err(format!(
                "picked {sat} sat on a lot with only {} sat available; reduce it",
                row.remaining_sat
            ));
        }
        if sat > 0 {
            total += sat;
            picks.push(btctax_core::LotPick {
                lot: row.lot_id.clone(),
                sat,
            });
        }
    }

    // Step 3: at least one pick required.
    if picks.is_empty() {
        return Err("pick at least one lot".to_string());
    }

    // Step 4: principal conservation.
    if total != item.principal_sat {
        return Err(format!(
            "picked {total} sat != disposal principal {} sat; adjust to match exactly",
            item.principal_sat
        ));
    }

    Ok(btctax_core::EventPayload::LotSelection(
        btctax_core::event::LotSelection {
            disposal_event: item.disposal_event.clone(),
            lots: picks,
        },
    ))
}

// ── Set-donation-details flow types ──────────────────────────────────────────

/// Pre-computed display data for a set-donation-details list row.
#[derive(Clone)]
pub struct DonationListItem {
    pub event_id: EventId,
    pub date: TaxDate,
    /// Σ removal.legs.iter().map(|l| l.sat).sum().
    pub total_sat: Sat,
    /// From `Removal.donee` (free-form label, if any).
    pub donee: Option<String>,
    /// From `snap.donation_details.get(&event_id).cloned()` [R0-I3].
    pub existing_details: Option<DonationDetails>,
}

impl DonationListItem {
    /// Return the completeness string for the Completeness column.
    pub fn completeness_str(&self) -> &'static str {
        match &self.existing_details {
            None => "(none)",
            Some(d) if d.is_review_complete(Form8283Section::B) => "B-complete",
            Some(_) => "present",
        }
    }
}

/// Step in the set-donation-details flow.
// FieldForm has 10 FieldBuffer fields by design; boxing the variant is not worth the
// refactor cost given TUI-only usage (stack frames are short-lived).
#[allow(clippy::large_enum_variant)]
pub enum SetDonationDetailsStep {
    List,
    FieldForm {
        item: DonationListItem,
        donee_name_buf: FieldBuffer,
        donee_address_buf: FieldBuffer,
        donee_ein_buf: FieldBuffer,
        appraiser_name_buf: FieldBuffer,
        appraiser_address_buf: FieldBuffer,
        appraiser_tin_buf: FieldBuffer,
        appraiser_ptin_buf: FieldBuffer,
        appraiser_qualifications_buf: FieldBuffer,
        appraisal_date_buf: FieldBuffer,
        fmv_method_override_buf: FieldBuffer,
        /// 0..=9 focus index.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the set-donation-details flow.
pub struct SetDonationDetailsFlowState {
    pub list: TargetList<DonationListItem>,
    pub step: SetDonationDetailsStep,
}

/// Payload for the set-donation-details confirmation modal.
pub struct SetDonationDetailsModalState {
    pub event_id: EventId,
    pub event_date: TaxDate,
    pub total_sat: Sat,
    /// The VALIDATED details payload.
    pub details: DonationDetails,
}

/// Labels for the 10 donation-details fields (focus index 0..=9).
pub const DONATION_FIELD_LABELS: [&str; 10] = [
    "donee_name (REQUIRED)",
    "donee_address",
    "donee_ein",
    "appraiser_name (REQUIRED)",
    "appraiser_address",
    "appraiser_tin",
    "appraiser_ptin",
    "appraiser_qualifications",
    "appraisal_date (YYYY-MM-DD)",
    "fmv_method_override",
];

/// Validate the donation-details FieldForm at Enter-press.
///
/// Returns `DonationDetails` or an error string.
///
/// Validation rules (spec D2):
/// 1. `donee_name`: REQUIRED (empty → error).
/// 2. `appraiser_name`: REQUIRED (empty → error).
/// 3. `appraisal_date`: if non-empty → `parse_date_arg(trim)` (YYYY-MM-DD → error on bad format).
/// 4. All other optionals: empty → `None`.
#[allow(clippy::too_many_arguments)]
pub fn validate_donation_details(
    donee_name_buf: &FieldBuffer,
    donee_address_buf: &FieldBuffer,
    donee_ein_buf: &FieldBuffer,
    appraiser_name_buf: &FieldBuffer,
    appraiser_address_buf: &FieldBuffer,
    appraiser_tin_buf: &FieldBuffer,
    appraiser_ptin_buf: &FieldBuffer,
    appraiser_qualifications_buf: &FieldBuffer,
    appraisal_date_buf: &FieldBuffer,
    fmv_method_override_buf: &FieldBuffer,
) -> Result<DonationDetails, String> {
    // donee_name: REQUIRED
    if donee_name_buf.is_empty() {
        return Err("donee-name is required".to_string());
    }
    let donee_name = donee_name_buf.buf.trim().to_owned();

    // appraiser_name: REQUIRED
    if appraiser_name_buf.is_empty() {
        return Err("appraiser-name is required".to_string());
    }
    let appraiser_name = appraiser_name_buf.buf.trim().to_owned();

    // Optional string fields.
    let opt_str = |buf: &FieldBuffer| -> Option<String> {
        if buf.is_empty() {
            None
        } else {
            let t = buf.buf.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_owned())
            }
        }
    };

    // appraisal_date: optional; if non-empty must parse as YYYY-MM-DD.
    let appraisal_date = if appraisal_date_buf.is_empty() {
        None
    } else {
        let t = appraisal_date_buf.buf.trim();
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        Some(time::Date::parse(t, fmt).map_err(|e| format!("bad date {t:?}: {e}"))?)
    };

    Ok(DonationDetails {
        donee_name,
        donee_address: opt_str(donee_address_buf),
        donee_ein: opt_str(donee_ein_buf),
        appraiser_name,
        appraiser_address: opt_str(appraiser_address_buf),
        appraiser_tin: opt_str(appraiser_tin_buf),
        appraiser_ptin: opt_str(appraiser_ptin_buf),
        appraiser_qualifications: opt_str(appraiser_qualifications_buf),
        appraisal_date,
        fmv_method_override: opt_str(fmv_method_override_buf),
    })
}

// ── Safe-harbor-attest flow types ─────────────────────────────────────────────

/// Step in the safe-harbor-attest flow.
///
/// TypedWord is the FINAL gate — no separate modal exists for this flow [R0-M4].
// TypedWord carries FieldBuffer which is not large; no boxing needed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum SafeHarborAttestStep {
    /// Step 1: displays allocation details + IRREVOCABLE warnings.
    Info,
    /// Step 2: user must type "ATTEST" (all-caps) to confirm.
    TypedWord {
        buf: FieldBuffer,
        error: Option<String>,
    },
}

/// Full state for the safe-harbor-attest flow.
///
/// No separate modal field on EditorApp — TypedWord is the gate [R0-M4].
pub struct SafeHarborAttestFlowState {
    /// EventId of the live (non-voided, timebarred) SafeHarborAllocation being re-attested.
    pub prior_id: btctax_core::EventId,
    /// The allocation payload (cloned from the pre-flight load — for display and re-append).
    pub prior_alloc: btctax_core::event::SafeHarborAllocation,
    pub step: SafeHarborAttestStep,
}

// ── Link-transfer flow types (chunk 4a, D1) ──────────────────────────────────

/// Human-readable one-line label for a `WalletId` (used in the modal target line
/// and the wallet pick-list). Mirrors the `provider/account` render convention.
pub fn wallet_label(w: &WalletId) -> String {
    match w {
        WalletId::Exchange { provider, account } => format!("{provider}/{account}"),
        WalletId::SelfCustody { label } => format!("self:{label}"),
    }
}

/// Pre-computed display data for a link-transfer step-1 (out-list) row.
///
/// Sourced from `snap.state.pending_reconciliation` (the same inherently-post-filtered
/// source reclassify-outflow uses): exactly the unlinked, unreconciled TransferOuts.
#[derive(Clone)]
pub struct TransferOutItem {
    /// The `PendingTransfer.event` EventId (the raw TransferOut).
    pub transfer_out_event: EventId,
    /// Calendar date (tax timezone) of the TransferOut event.
    pub date: TaxDate,
    /// Principal sat from `PendingTransfer.principal_sat`.
    pub principal_sat: Sat,
    /// Wallet of the TransferOut event (SOURCE wallet).
    pub wallet: Option<WalletId>,
}

/// Pre-computed display data for a link-transfer step-2 in-event (InEvent mode) row.
///
/// Only `TransferIn` events whose raw `LedgerEvent.wallet.is_some()` (the engine requires a
/// resolvable destination wallet) AND not already targeted by a non-voided `TransferLink::InEvent`.
#[derive(Clone)]
pub struct InEventItem {
    /// The TransferIn event id.
    pub in_event: EventId,
    /// Calendar date (tax timezone) of the TransferIn event.
    pub date: TaxDate,
    /// Principal sat from the TransferIn payload.
    pub sat: Sat,
    /// Destination wallet (guaranteed `Some` by the pre-filter).
    pub wallet: WalletId,
}

/// Pre-computed display data for a link-transfer step-2 wallet (Wallet mode) row.
///
/// ALL distinct `snap.events[].wallet` Some-values [R0-I2] — NOT just `holdings_by_wallet`
/// keys (which would hide a zero-balance destination wallet, the primary Wallet-target use case).
#[derive(Clone)]
pub struct WalletItem {
    pub wallet: WalletId,
}

/// Which target mode the step-2 picker is showing (Tab cycles InEvent ⇄ Wallet).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LinkMode {
    InEvent,
    Wallet,
}

/// Step in the link-transfer flow.
pub enum LinkTransferStep {
    OutList,
    TargetPick {
        out: TransferOutItem,
        mode: LinkMode,
    },
}

/// Full state for the link-transfer flow. Owns all three target lists [R0-I2 discipline].
pub struct LinkTransferFlowState {
    /// Step-1 list: pending TransferOuts.
    pub out_list: TargetList<TransferOutItem>,
    pub step: LinkTransferStep,
    /// Step-2 InEvent-mode list (built once at open).
    pub in_list: TargetList<InEventItem>,
    /// Step-2 Wallet-mode list (built once at open).
    pub wallet_list: TargetList<WalletItem>,
}

/// Payload for the link-transfer confirmation modal.
pub struct LinkTransferModalState {
    pub out_event: EventId,
    pub out_date: TaxDate,
    pub out_sat: Sat,
    /// The VALIDATED target — what will be persisted (InEvent(id) or Wallet(w)).
    pub target: TransferTarget,
    /// Human-readable target label (shown in the modal).
    pub target_label: String,
}

// ── Match-self-transfers flow types (self-transfer-passthrough C3) ────────────

/// The confirm action for a matched self-transfer pair. Mirrors `btctax_cli::MatchAction`; kept as a
/// local btctax_core-only enum so `form.rs` stays free of the CLI dependency. The user's choice IS the
/// determination (DROP = same-wallet passthrough → `SelfTransferPassthrough`; RELOCATE = cross-wallet →
/// `TransferLink`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MatchPairAction {
    Drop,
    Relocate,
}
impl MatchPairAction {
    pub fn toggle(self) -> Self {
        match self {
            Self::Drop => Self::Relocate,
            Self::Relocate => Self::Drop,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Drop => "DROP (passthrough — both legs dropped, non-taxable)",
            Self::Relocate => "RELOCATE (link — coins move to the destination wallet)",
        }
    }
}

/// One proposed self-transfer match row (mirrors `btctax_cli::MatchProposal` + the suggested action as
/// a local enum). Built at open from `Session::self_transfer_match_plan`.
#[derive(Clone)]
pub struct MatchSelfTransferItem {
    pub in_event: EventId,
    pub out_event: EventId,
    pub in_date: TaxDate,
    pub out_date: TaxDate,
    pub in_wallet: Option<WalletId>,
    pub out_wallet: Option<WalletId>,
    pub in_sat: Sat,
    pub out_principal_sat: Sat,
    pub usd_value: Option<Usd>,
    /// The topology-derived suggestion (same-wallet ⇒ Drop, cross-wallet ⇒ Relocate).
    pub suggested: MatchPairAction,
    pub ambiguous: bool,
    pub txid_match: bool,
}

/// Full state for the match-self-transfers flow: a single-step list of proposed pairs.
pub struct MatchSelfTransfersFlowState {
    pub list: TargetList<MatchSelfTransferItem>,
}

/// Payload for the match-self-transfers confirmation modal. `action` starts at the suggested action and
/// is toggled DROP↔RELOCATE by the user (the choice is the determination). NEVER auto-applied.
pub struct MatchSelfTransfersModalState {
    pub in_event: EventId,
    pub out_event: EventId,
    pub in_sat: Sat,
    pub out_principal_sat: Sat,
    pub in_wallet: Option<WalletId>,
    pub out_wallet: Option<WalletId>,
    pub action: MatchPairAction,
    pub ambiguous: bool,
}

// ── Classify-raw flow types (chunk 4a, D2) ───────────────────────────────────

/// Pre-computed display data for a classify-raw list row.
///
/// Events carrying `BlockerKind::Unclassified` whose payload is `EventPayload::Unclassified`,
/// minus those already targeted by a non-voided `ClassifyRaw`.
#[derive(Clone)]
pub struct RawListItem {
    /// The Unclassified event id (the ClassifyRaw target — its EventId is preserved).
    pub target: EventId,
    /// Calendar date (tax timezone) of the raw event.
    pub date: TaxDate,
    /// The raw import text (`Unclassified.raw`).
    pub raw: String,
    /// Wallet of the raw event (kept by the classified effective payload).
    pub wallet: Option<WalletId>,
}

/// Which imported variant the classify-raw picker is building (Tab cycles).
///
/// **Scoped to Acquire + Income this cycle** — the two that dominate raw-row classification.
/// Dispose/TransferOut/TransferIn/Unclassified are a CLI-only parity FOLLOWUP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifyRawVariant {
    Acquire,
    Income,
}

/// Cycle the classify-raw variant (Acquire ⇄ Income).
pub fn cycle_classify_raw_variant(v: ClassifyRawVariant) -> ClassifyRawVariant {
    match v {
        ClassifyRawVariant::Acquire => ClassifyRawVariant::Income,
        ClassifyRawVariant::Income => ClassifyRawVariant::Acquire,
    }
}

/// Cycle through the 8 user-selectable `BasisSource` variants in declaration order (event.rs:16-26).
/// `SelfTransferInbound` is a system-assigned source (the inbound self-transfer fold), NOT a manual
/// classify-raw choice, so it is deliberately OUTSIDE the ring: it never appears as a cycle output, and
/// if an edited lot somehow carries it, one Tab exits to `ExchangeProvided`.
pub fn cycle_basis_source(bs: BasisSource) -> BasisSource {
    match bs {
        BasisSource::ExchangeProvided => BasisSource::ComputedFromCost,
        BasisSource::ComputedFromCost => BasisSource::FmvAtIncome,
        BasisSource::FmvAtIncome => BasisSource::CarriedFromTransfer,
        BasisSource::CarriedFromTransfer => BasisSource::GiftCarryover,
        BasisSource::GiftCarryover => BasisSource::GiftFmvFallback,
        BasisSource::GiftFmvFallback => BasisSource::SafeHarborAllocated,
        BasisSource::SafeHarborAllocated => BasisSource::ReconstructedPerWallet,
        BasisSource::ReconstructedPerWallet => BasisSource::ExchangeProvided,
        BasisSource::SelfTransferInbound => BasisSource::ExchangeProvided, // off-ring defensive exit
    }
}

/// Lowercase display tag for a `BasisSource`.
pub fn basis_source_display(bs: BasisSource) -> &'static str {
    match bs {
        BasisSource::ExchangeProvided => "exchange-provided",
        BasisSource::ComputedFromCost => "computed-from-cost",
        BasisSource::FmvAtIncome => "fmv-at-income",
        BasisSource::CarriedFromTransfer => "carried-from-transfer",
        BasisSource::GiftCarryover => "gift-carryover",
        BasisSource::GiftFmvFallback => "gift-fmv-fallback",
        BasisSource::SafeHarborAllocated => "safe-harbor-allocated",
        BasisSource::ReconstructedPerWallet => "reconstructed-per-wallet",
        BasisSource::SelfTransferInbound => "self-transfer-inbound",
    }
}

/// Step in the classify-raw flow.
// AcquireForm / IncomeForm carry several FieldBuffers; boxing is not worth it (TUI-only).
#[allow(clippy::large_enum_variant)]
pub enum ClassifyRawStep {
    List,
    VariantPicker {
        item: RawListItem,
        variant: ClassifyRawVariant,
    },
    AcquireForm {
        item: RawListItem,
        sat_buf: FieldBuffer,
        usd_cost_buf: FieldBuffer,
        fee_buf: FieldBuffer,
        /// A required BasisSource PICK (default ExchangeProvided); Tab cycles on the picker row.
        basis_source: BasisSource,
        /// 0=sat, 1=usd_cost, 2=fee, 3=basis_source.
        focus: usize,
        error: Option<String>,
    },
    IncomeForm {
        item: RawListItem,
        sat_buf: FieldBuffer,
        /// Optional; typed → fmv_status=ManualEntry, empty → None + Missing.
        fmv_buf: FieldBuffer,
        kind: IncomeKind,
        business: bool,
        /// 0=sat, 1=usd_fmv, 2=kind, 3=business.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the classify-raw flow. Owns its target list.
pub struct ClassifyRawFlowState {
    pub list: TargetList<RawListItem>,
    pub step: ClassifyRawStep,
}

/// Payload for the classify-raw confirmation modal.
pub struct ClassifyRawModalState {
    /// The Unclassified target event id.
    pub target: EventId,
    /// The raw import text (shown in the modal).
    pub raw: String,
    /// The BUILT imported payload (Acquire/Income) — boxed into `ClassifyRaw.as_` at persist.
    pub built: btctax_core::EventPayload,
}

/// Parse a REQUIRED sat field: empty (len==0) → "name is required"; else parse i64.
fn parse_required_sat(buf: &FieldBuffer, name: &str) -> Result<Sat, String> {
    if buf.is_empty() {
        return Err(format!("{name} is required"));
    }
    let t = buf.buf.trim();
    t.parse::<i64>().map_err(|_| format!("bad sat {t:?}"))
}

/// Validate the classify-raw Acquire form → `EventPayload::Acquire(…)` built DIRECTLY (NOT via
/// `InboundClass` [R0-I1]).
///
/// `sat`/`usd_cost` REQUIRED; `fee_usd` optional → $0; `basis_source` is the required PICK.
/// NO acquired-at field — the effective event keeps the TARGET's timestamp (resolve.rs) [R0-I1].
pub fn validate_classify_raw_acquire(
    sat_buf: &FieldBuffer,
    usd_cost_buf: &FieldBuffer,
    fee_buf: &FieldBuffer,
    basis_source: BasisSource,
) -> Result<btctax_core::EventPayload, String> {
    let sat = parse_required_sat(sat_buf, "sat")?;
    if usd_cost_buf.is_empty() {
        return Err("usd-cost is required".to_string());
    }
    let uc = usd_cost_buf.buf.trim();
    let usd_cost = Usd::from_str(uc).map_err(|_| format!("bad USD {uc:?}"))?;
    let fee_usd = if fee_buf.is_empty() {
        Usd::ZERO
    } else {
        let t = fee_buf.buf.trim();
        Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?
    };
    Ok(btctax_core::EventPayload::Acquire(
        btctax_core::event::Acquire {
            sat,
            usd_cost,
            fee_usd,
            basis_source,
        },
    ))
}

/// Validate the classify-raw Income form → `EventPayload::Income(…)` built DIRECTLY (NOT via
/// `InboundClass` [R0-I1]).
///
/// `sat` REQUIRED; `usd_fmv` optional: typed → `Some` + `fmv_status=ManualEntry`, empty → `None` +
/// `fmv_status=Missing` (resolve.rs discards `usd_fmv` when `Missing`; the empty case fires a
/// `FmvMissing` blocker surfaced by status arm 3). `kind` PICK; `business` toggle.
pub fn validate_classify_raw_income(
    sat_buf: &FieldBuffer,
    fmv_buf: &FieldBuffer,
    kind: IncomeKind,
    business: bool,
) -> Result<btctax_core::EventPayload, String> {
    let sat = parse_required_sat(sat_buf, "sat")?;
    let (usd_fmv, fmv_status) = if fmv_buf.is_empty() {
        (None, FmvStatus::Missing)
    } else {
        let t = fmv_buf.buf.trim();
        let v = Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?;
        (Some(v), FmvStatus::ManualEntry)
    };
    Ok(btctax_core::EventPayload::Income(
        btctax_core::event::Income {
            sat,
            usd_fmv,
            fmv_status,
            kind,
            business,
        },
    ))
}

// ── Resolve-conflict flow types (chunk 4b, D3) ───────────────────────────────

/// Pre-computed display data for a resolve-conflict list row.
///
/// Sourced from events carrying `BlockerKind::ImportConflict` — the blocker's `.event` is the
/// `ImportConflict` event id (`conflict_event`), whose payload names the `target` import event and
/// the `new_payload` proposed to supersede it. The two summaries are computed at open time (against
/// the CURRENT `target` payload vs the conflict's `new_payload`), so the flow/modal render no live
/// event lookups.
#[derive(Clone)]
pub struct ConflictItem {
    /// The `ImportConflict` event id — the resolution target (`SupersedeImport`/`RejectImport`
    /// carry this as `conflict_event`).
    pub conflict_event: EventId,
    /// The TARGET import event id whose payload the conflict proposes to supersede.
    pub target: EventId,
    /// Calendar date (tax tz) of the conflict event.
    pub date: TaxDate,
    /// Short `new_fingerprint` string (table column).
    pub new_fingerprint: String,
    /// One-line summary of the TARGET's CURRENT payload (kept on reject; replaced on accept).
    pub current_summary: String,
    /// One-line summary of the conflict's NEW payload (adopted on accept).
    pub new_summary: String,
}

/// Accept vs Reject branch of the resolve-conflict flow (an in-flow toggle — NOT Browse `a`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveKind {
    /// `SupersedeImport` — adopt `new_payload` onto the target id.
    Accept,
    /// `RejectImport` — keep the original; discard the new payload.
    Reject,
}

/// Step in the resolve-conflict flow.
pub enum ResolveConflictStep {
    /// Step 1: pick a conflict.
    List,
    /// Step 2: accept/reject choice for the picked conflict (`←/→` or `h/l` toggles).
    Choose {
        conflict: ConflictItem,
        kind: ResolveKind,
    },
}

/// Full state for the resolve-conflict flow. Owns its list [R0-I2 discipline].
pub struct ResolveConflictFlowState {
    pub list: TargetList<ConflictItem>,
    pub step: ResolveConflictStep,
}

/// Payload for the resolve-conflict confirmation modal.
///
/// Shows BOTH sides (the target's CURRENT payload vs the conflict's NEW payload) and the
/// NON-REVOCABLE warning. `SupersedeImport`/`RejectImport` are excluded from `is_revocable_payload`;
/// a later void fires `DecisionConflict`, so the decision cannot be undone in-editor.
pub struct ResolveConflictModalState {
    pub conflict_event: EventId,
    pub target: EventId,
    pub kind: ResolveKind,
    pub old_summary: String,
    pub new_summary: String,
}

// ── Optimize-accept flow types (chunk 4b, D4) ────────────────────────────────

/// Pre-computed display data for one optimize-accept candidate disposal (a per-disposal proposal row
/// that survived the pre-filter). NO per-disposal Δtax [R0-I1]: `DisposalProposal` carries no
/// per-disposal delta, and the year-level `OptimizeProposal.delta` is shown at the flow level only.
#[derive(Clone)]
pub struct OptimizeCandidateItem {
    /// The disposal EventId the proposed selection targets.
    pub disposal: EventId,
    /// Wallet of the disposal (table column).
    pub wallet: WalletId,
    /// Calendar date (tax tz) of the disposal.
    pub date: TaxDate,
    /// §C.2 gate verdict — drives the step-2 branch (`ContemporaneousNow` → modal;
    /// `NeedsAttestation` → attestation-text step). `ForbiddenBroker2027` is pre-filtered out.
    pub persistable: Persistability,
    /// The optimizer's tax-minimizing pick (the `LotSelection.lots` to persist).
    pub picks: Vec<LotPick>,
}

/// Step in the optimize-accept flow.
// AttestText carries a FieldBuffer + a cloned item; boxing is not worth it (TUI-only).
#[allow(clippy::large_enum_variant)]
pub enum OptimizeAcceptStep {
    /// Step 1: pick a proposed disposal.
    List,
    /// Step 2 (NeedsAttestation only): type the contemporaneous-ID attestation (non-empty required).
    AttestText {
        item: OptimizeCandidateItem,
        buf: FieldBuffer,
        error: Option<String>,
    },
}

/// Full state for the optimize-accept flow. Owns its candidate list [R0-I2 discipline].
pub struct OptimizeAcceptFlowState {
    pub list: TargetList<OptimizeCandidateItem>,
    pub step: OptimizeAcceptStep,
    /// Whole-year `optimized − baseline` figure (≤ 0) — the flow-level banner [R0-I1].
    pub delta: Usd,
    /// `true` ⇔ the proposal is only APPROXIMATE (banner caveat).
    pub approximate: bool,
}

/// Payload for the optimize-accept confirmation modal.
pub struct OptimizeAcceptModalState {
    pub disposal: EventId,
    /// The proposed `LotSelection.lots`.
    pub picks: Vec<LotPick>,
    pub pick_count: usize,
    /// Σ picks.sat.
    pub total_sat: Sat,
    /// `None` → `Contemporaneous` (basis label; no attest row); `Some(text)` → `AttestedRecording`
    /// (the attestation text co-persisted alongside the LotSelection).
    pub attestation: Option<String>,
    /// §A.5 basis label (`"Contemporaneous"` / `"AttestedRecording"`).
    pub basis_label: &'static str,
}

// ── Safe-harbor-allocate flow types (chunk 5, D2) ────────────────────────────

/// Pre-computed display row for one pre-2025 residue `AllocLot` in the allocate Preview table.
/// Carries the AllocLot's display-relevant fields; the residue is method-INDEPENDENT (G3), so these
/// rows are computed ONCE at open and never recomputed when the `method` toggle changes.
#[derive(Clone)]
pub struct AllocLotRow {
    pub wallet: WalletId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
    /// §1015(a) LOSS basis (FMV-at-gift); `Some` only for a dual-basis gift lot.
    pub dual_loss_basis: Option<Usd>,
    /// §1223(2) donor acquisition date (tacking); `Some` only for a gift lot.
    pub donor_acquired_at: Option<TaxDate>,
}

/// Step in the safe-harbor-allocate flow. There is exactly ONE step (the Preview); the confirmation
/// is a SEPARATE modal (`SafeHarborAllocateModalState`) — creation is REVOCABLE, so NO typed-word gate
/// (contrast the attest flow, whose TypedWord step IS the gate).
pub enum SafeHarborAllocateStep {
    Preview,
}

/// Full state for the safe-harbor-allocate flow (chunk 5, D2).
///
/// The ONLY user input is `method` (an `AllocMethod` toggle). `lots`/`total_sat`/`total_basis` are the
/// residue computed ONCE at open via `Session::safe_harbor_residue`; the `method` toggle changes ONLY
/// the recorded tag, never the displayed lots (G3). `pre2025_method` is the method the residue was
/// computed under, threaded UNCHANGED through flow→modal→persist (G5 immutability).
pub struct SafeHarborAllocateFlowState {
    pub lots: Vec<AllocLot>,
    pub total_sat: Sat,
    pub total_basis: Usd,
    pub method: AllocMethod,
    pub pre2025_method: LotMethod,
    pub list: TargetList<AllocLotRow>,
    pub step: SafeHarborAllocateStep,
}

/// Payload for the safe-harbor-allocate confirmation modal (revocable framing; NOT typed-word).
pub struct SafeHarborAllocateModalState {
    pub lots: Vec<AllocLot>,
    pub total_sat: Sat,
    pub total_basis: Usd,
    pub method: AllocMethod,
    pub pre2025_method: LotMethod,
    pub lot_count: usize,
}

// ── Bulk link-transfer flow types (bulk-link-transfer D3) ────────────────────

/// One preview-checklist row: an enriched pending out + its `checked` (included) flag. Every row
/// starts CHECKED; `Space`/`x` toggles `checked` (exclude). Mirrors `session::BulkLinkRow` fields plus
/// the UI flag.
#[derive(Clone)]
pub struct BulkLinkRowItem {
    pub out_event: EventId,
    pub date: TaxDate,
    pub source_wallet: Option<WalletId>,
    pub principal_sat: Sat,
    /// Advisory FMV (`fmv_of`); `None` on missing price → footer floor + "(N unavailable)".
    pub usd_value: Option<Usd>,
    pub basis_usd: Usd,
    pub checked: bool,
}

/// Step in the bulk link-transfer flow (four steps on the `TargetList` substrate).
pub enum BulkLinkStep {
    /// 1 — pick the destination from the event-wallet union (or press `n` to type one).
    DestPick,
    /// 1b — free-text destination entry [Fork B]: `parse_wallet_id` reaches a never-seen cold wallet.
    DestType,
    /// 2 — source-wallet (Any/each) + time-frame (All/each-year) toggles.
    Filter,
    /// 3 — per-row exclude checklist over the PRICED plan; `Space`/`x` toggles a row.
    Preview,
}

/// Full state for the bulk link-transfer flow. The dest pick-list + filter choices are read from the
/// snapshot at open (KAT-G1-clean, like `open_link_transfer_flow`); only the PRICED preview (step 2→3)
/// routes through the `Session::bulk_link_transfer_plan` helper [R0-M4].
pub struct BulkLinkFlowState {
    pub step: BulkLinkStep,
    // Step 1 — destination.
    /// The DISTINCT wallets across ALL `snap.events` (a dest may only ever appear inbound).
    pub wallet_list: TargetList<WalletId>,
    /// Typed-destination entry buffer [Fork B].
    pub dest_buf: FieldBuffer,
    /// The chosen destination, set on leaving step 1.
    pub dest: Option<WalletId>,
    // Step 2 — filter choices (derived from the pending outs at open).
    /// `[None = Any, Some(w), …]` — Any + each distinct pending-out source wallet.
    pub source_choices: Vec<Option<WalletId>>,
    pub source_idx: usize,
    /// `[None = All, Some(y), …]` — All + each distinct year present in the pending outs.
    pub year_choices: Vec<Option<i32>>,
    pub year_idx: usize,
    /// 0 = source-wallet row, 1 = time-frame row.
    pub filter_focus: usize,
    // Step 3 — preview checklist over the priced plan.
    pub preview: TargetList<BulkLinkRowItem>,
    /// Cross-step transient error (bad typed dest / empty plan / nothing selected).
    pub error: Option<String>,
}

/// Payload for the bulk link-transfer confirmation modal (explicit confirm; NOT typed-word — the op
/// is reversible per-`v`). Captures the CHECKED rows only.
pub struct BulkLinkModalState {
    pub dest: WalletId,
    pub out_events: Vec<EventId>,
    pub count: usize,
    pub total_sat: Sat,
    pub total_usd_value_floor: Usd,
    pub missing_price_count: usize,
}

/// Live footer/modal totals over the CHECKED rows: `(count, Σ sat, Σ priced USD floor, missing count)`.
pub fn bulk_checked_totals(items: &[BulkLinkRowItem]) -> (usize, Sat, Usd, usize) {
    let mut count = 0usize;
    let mut sat: Sat = 0;
    let mut floor: Usd = Usd::ZERO;
    let mut missing = 0usize;
    for it in items.iter().filter(|i| i.checked) {
        count += 1;
        sat += it.principal_sat;
        match it.usd_value {
            Some(v) => floor += v,
            None => missing += 1,
        }
    }
    (count, sat, floor, missing)
}

/// Render the USD floor for the bulk preview/modal: exact `$X` when nothing is unpriced, else
/// `≥ $X (N unavailable)` [R0-I2].
pub fn bulk_usd_floor_label(floor: Usd, missing: usize) -> String {
    if missing == 0 {
        format!("${floor}")
    } else {
        format!("\u{2265} ${floor} ({missing} unavailable)")
    }
}

// ── Bulk classify-inbound-self-transfer flow types (bulk-classify-inbound-self-transfer D3) ──

/// One preview-checklist row: an enriched pending unknown-basis inbound + its `checked` (included)
/// flag. Every row starts CHECKED; `Space`/`x` toggles `checked` (exclude). Mirrors `session::BulkStiRow`
/// fields plus the UI flag.
#[derive(Clone)]
pub struct BulkStiRowItem {
    pub in_event: EventId,
    pub date: TaxDate,
    pub wallet: Option<WalletId>,
    pub sat: Sat,
    /// The USD FMV being GIVEN $0 basis (`fmv_of`); `None` on missing price → footer floor + "(N unavailable)".
    pub usd_fmv: Option<Usd>,
    pub checked: bool,
}

/// Step in the bulk STI flow (two steps — NO destination pick; an inbound IS the receiving leg).
pub enum BulkStiStep {
    /// 1 — receiving-wallet (Any/each) + time-frame (All/each-year) toggles.
    Filter,
    /// 2 — per-row exclude checklist over the PRICED plan; `Space`/`x` toggles a row.
    Preview,
}

/// Full state for the bulk STI flow. The filter choices are read from the snapshot at open
/// (KAT-G1-clean, like `open_classify_inbound_flow`); only the PRICED preview (step 1→2) routes through
/// the `Session::bulk_self_transfer_in_plan` helper.
pub struct BulkStiFlowState {
    pub step: BulkStiStep,
    /// `[None = Any, Some(w), …]` — Any + each distinct receiving wallet of the candidates.
    pub wallet_choices: Vec<Option<WalletId>>,
    pub wallet_idx: usize,
    /// `[None = All, Some(y), …]` — All + each distinct year present in the candidates.
    pub year_choices: Vec<Option<i32>>,
    pub year_idx: usize,
    /// 0 = receiving-wallet row, 1 = time-frame row.
    pub filter_focus: usize,
    /// Preview checklist over the priced plan.
    pub preview: TargetList<BulkStiRowItem>,
    /// Cross-step transient error (empty plan / nothing selected).
    pub error: Option<String>,
}

/// Payload for the bulk STI confirmation modal (explicit confirm; NOT typed-word — each classification
/// is voidable per-`v`). Captures the CHECKED rows only.
pub struct BulkStiModalState {
    pub in_events: Vec<EventId>,
    pub count: usize,
    pub total_sat: Sat,
    pub total_usd_fmv_floor: Usd,
    pub missing_price_count: usize,
}

/// Live footer/modal totals over the CHECKED rows: `(count, Σ sat, Σ priced USD floor, missing count)`.
pub fn bulk_sti_checked_totals(items: &[BulkStiRowItem]) -> (usize, Sat, Usd, usize) {
    let mut count = 0usize;
    let mut sat: Sat = 0;
    let mut floor: Usd = Usd::ZERO;
    let mut missing = 0usize;
    for it in items.iter().filter(|i| i.checked) {
        count += 1;
        sat += it.sat;
        match it.usd_fmv {
            Some(v) => floor += v,
            None => missing += 1,
        }
    }
    (count, sat, floor, missing)
}

// ── Bulk classify-inbound-income flow types (bulk-classify-inbound-income, Cycle 4) ──

/// One preview-checklist row: an enriched pending unknown-basis inbound + its `checked` flag. Every row
/// starts CHECKED; `Space`/`x` toggles `checked` (exclude). Mirrors `session::BulkIncomeRow` + the UI
/// flag. [#a] `fmv` is the RESOLVED auto-value (never `None` — the `None` rows are excluded upstream);
/// it is BOTH the income recognized AND the lot basis for this row.
#[derive(Clone)]
pub struct BulkIncomeRowItem {
    pub in_event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub fmv: Usd,
    pub checked: bool,
}

/// Step in the bulk classify-income flow (two steps — the uniform kind/business + wallet/year filter,
/// then the per-row exclude checklist).
pub enum BulkIncomeStep {
    /// 1 — income-kind + business-flag + receiving-wallet + time-frame toggles.
    Filter,
    /// 2 — per-row exclude checklist over the PRICED plan; `Space`/`x` toggles a row.
    Preview,
}

/// Full state for the bulk classify-income flow. The wallet/year filter choices are read from the
/// snapshot at open (KAT-G1-clean, like the STI flow); only the PRICED preview (step 1→2) routes
/// through the `Session::bulk_classify_income_plan` helper.
pub struct BulkIncomeFlowState {
    pub step: BulkIncomeStep,
    /// The UNIFORM income kind for the batch; `←/→` cycles it (`cycle_income_kind`). Starts `Mining`.
    pub kind: IncomeKind,
    /// The UNIFORM business flag for the batch; `←/→` toggles it. Starts `false`.
    pub business: bool,
    /// `[None = Any, Some(w), …]` — Any + each distinct receiving wallet of the candidates.
    pub wallet_choices: Vec<Option<WalletId>>,
    pub wallet_idx: usize,
    /// `[None = All, Some(y), …]` — All + each distinct year present in the candidates.
    pub year_choices: Vec<Option<i32>>,
    pub year_idx: usize,
    /// 0 = kind, 1 = business, 2 = receiving-wallet, 3 = time-frame.
    pub filter_focus: usize,
    /// Preview checklist over the priced plan.
    pub preview: TargetList<BulkIncomeRowItem>,
    /// [#a] From the last recomputed plan: candidates dropped for a MISSING price (surfaced, not
    /// silently dropped — the user learns N inbounds could not be auto-valued as income).
    pub excluded_missing_price: usize,
    /// Cross-step transient error (empty plan / nothing selected).
    pub error: Option<String>,
}

/// Payload for the bulk classify-income confirmation modal (explicit confirm; NOT typed-word — each
/// classification is voidable per-`v`, matching the STI tier). Captures the CHECKED rows' per-row
/// `ClassifyInbound{Income{kind, Some(fmv), business}}` decisions + the preview totals.
pub struct BulkIncomeModalState {
    pub payloads: Vec<btctax_core::EventPayload>,
    pub count: usize,
    pub total_sat: Sat,
    /// Σ per-row auto-FMV over the CHECKED rows — the total income being recognized (prominent).
    pub total_income_usd: Usd,
    /// [#a] Carried forward from the plan: inbounds NOT auto-valuable (surfaced in the modal note).
    pub excluded_missing_price: usize,
    pub kind: IncomeKind,
    pub business: bool,
}

/// Live footer/modal totals over the CHECKED rows: `(count, Σ sat, Σ income USD)`. Every checked row is
/// priced (the plan excluded the unpriced ones), so the income total is always a real number.
pub fn bulk_income_checked_totals(items: &[BulkIncomeRowItem]) -> (usize, Sat, Usd) {
    let mut count = 0usize;
    let mut sat: Sat = 0;
    let mut income: Usd = Usd::ZERO;
    for it in items.iter().filter(|i| i.checked) {
        count += 1;
        sat += it.sat;
        income += it.fmv;
    }
    (count, sat, income)
}

// ── Bulk reclassify-outflow flow types (bulk-reclassify-outflow, Cycle 5 — the LAST) ──

/// Toggle the batch-wide disposition kind (Sell ↔ Spend). The ONLY two kinds in scope — gift/donate
/// are excluded (they need a per-row donee + appraisal FMV, so a uniform bulk substitution is wrong).
pub fn cycle_dispose_kind(kind: DisposeKind) -> DisposeKind {
    match kind {
        DisposeKind::Sell => DisposeKind::Spend,
        DisposeKind::Spend => DisposeKind::Sell,
    }
}

/// One preview-checklist row: an enriched pending outflow + its `checked` flag. Every row starts
/// CHECKED; `Space`/`x` toggles `checked` (exclude). Mirrors `session::BulkReclassifyOutflowRow`.
/// [#a] `fmv` is the RESOLVED auto-value (never `None` — the `None` rows are excluded upstream); it is
/// the ESTIMATED proceeds. `estimated_gain = round_cents(fmv − basis_usd)` (never double-counted).
#[derive(Clone)]
pub struct BulkReclassifyOutflowRowItem {
    pub out_event: EventId,
    pub date: TaxDate,
    pub principal_sat: Sat,
    pub fmv: Usd,
    pub basis_usd: Usd,
    pub estimated_gain: Usd,
    pub checked: bool,
}

/// Step in the bulk reclassify-outflow flow (two steps — the uniform kind + source-wallet/frame
/// filter, then the per-row exclude checklist).
pub enum BulkReclassifyOutflowStep {
    /// 1 — dispose-kind (Sell/Spend) + source-wallet + time-frame toggles.
    Filter,
    /// 2 — per-row exclude checklist over the PRICED plan; `Space`/`x` toggles a row.
    Preview,
}

/// Full state for the bulk reclassify-outflow flow. The wallet/year filter choices are read from the
/// snapshot at open (KAT-G1-clean, like the income flow); only the PRICED preview (step 1→2) routes
/// through the `Session::bulk_reclassify_outflow_plan` helper.
pub struct BulkReclassifyOutflowFlowState {
    pub step: BulkReclassifyOutflowStep,
    /// The UNIFORM disposition kind for the batch; `←/→` toggles it (`cycle_dispose_kind`). Starts `Sell`.
    pub kind: DisposeKind,
    /// `[None = Any, Some(w), …]` — Any + each distinct SOURCE wallet of the candidates.
    pub wallet_choices: Vec<Option<WalletId>>,
    pub wallet_idx: usize,
    /// `[None = All, Some(y), …]` — All + each distinct year present in the candidates.
    pub year_choices: Vec<Option<i32>>,
    pub year_idx: usize,
    /// 0 = kind, 1 = source-wallet, 2 = time-frame.
    pub filter_focus: usize,
    /// Preview checklist over the priced plan.
    pub preview: TargetList<BulkReclassifyOutflowRowItem>,
    /// [#a] From the last recomputed plan: candidates dropped for a MISSING price (surfaced, not
    /// silently dropped — a Sell with fabricated proceeds would be a SILENT misreport).
    pub excluded_missing_price: usize,
    /// Cross-step transient error (empty plan / nothing selected).
    pub error: Option<String>,
}

/// Payload for the bulk reclassify-outflow confirmation modal (explicit confirm; NOT typed-word — each
/// `ReclassifyOutflow` is voidable per-`v`, the REVOCABLE tier + a prominent ESTIMATED warning).
/// Captures the CHECKED rows' `(out_event, fmv)` pairs (the persist builds the `ReclassifyOutflow` +
/// side-table mark) + the preview totals + the batch-wide kind.
pub struct BulkReclassifyOutflowModalState {
    /// The CHECKED rows as `(out_event, resolved fmv proceeds)` — the persist builds the decision +
    /// `bulk_estimated::mark` per row.
    pub rows: Vec<(EventId, Usd)>,
    pub count: usize,
    pub total_sat: Sat,
    /// Σ per-row auto-FMV over the CHECKED rows — the total ESTIMATED proceeds (prominent).
    pub total_proceeds_usd: Usd,
    /// Σ per-row basis over the CHECKED rows.
    pub total_basis_usd: Usd,
    /// Σ per-row estimated gain over the CHECKED rows — the total ESTIMATED gain (prominent).
    pub total_estimated_gain: Usd,
    /// [#a] Carried forward from the plan: outflows NOT auto-valuable (surfaced in the modal note).
    pub excluded_missing_price: usize,
    pub kind: DisposeKind,
}

/// Live footer/modal totals over the CHECKED rows: `(count, Σ sat, Σ proceeds, Σ basis, Σ gain)`. Every
/// checked row is priced (the plan excluded the unpriced ones), so the totals are always real numbers.
pub fn bulk_reclassify_outflow_checked_totals(
    items: &[BulkReclassifyOutflowRowItem],
) -> (usize, Sat, Usd, Usd, Usd) {
    let mut count = 0usize;
    let mut sat: Sat = 0;
    let mut proceeds: Usd = Usd::ZERO;
    let mut basis: Usd = Usd::ZERO;
    let mut gain: Usd = Usd::ZERO;
    for it in items.iter().filter(|i| i.checked) {
        count += 1;
        sat += it.principal_sat;
        proceeds += it.fmv;
        basis += it.basis_usd;
        gain += it.estimated_gain;
    }
    (count, sat, proceeds, basis, gain)
}

// ── Bulk resolve-conflict flow types (bulk-resolve-conflict D3) ──────────────

/// One preview-checklist row: a flagged import conflict + its `checked` (included) flag. Every row
/// starts CHECKED; `Space`/`x` toggles `checked` (exclude). The `current`/`new` summaries are computed
/// at open time (via `import_payload_summary` over the plan's STRUCTURED payloads) so the flow/modal
/// render no live event lookups.
#[derive(Clone)]
pub struct BulkResolveRowItem {
    /// The `ImportConflict` event id — the resolution target (`SupersedeImport`/`RejectImport` carry
    /// this as `conflict_event`).
    pub conflict_event: EventId,
    /// The TARGET import event id whose payload the conflict proposes to supersede.
    pub target: EventId,
    /// Calendar date (tax tz) of the conflict event.
    pub date: TaxDate,
    /// One-line summary of the TARGET's CURRENT payload (kept on reject; replaced on accept).
    pub current_summary: String,
    /// One-line summary of the conflict's NEW payload (adopted on accept).
    pub new_summary: String,
    /// Short `new_fingerprint` disambiguator (table column).
    pub new_fingerprint: String,
    pub checked: bool,
}

/// Step in the bulk resolve-conflict flow (two steps — NO filter; the batch-wide accept/reject toggle
/// is the ONLY param, per-row exclude is the precision tool).
pub enum BulkResolveStep {
    /// 1 — batch-wide Accept/Reject toggle (`←/→`); the ONLY "param".
    Choose,
    /// 2 — per-row exclude checklist over the live conflicts; `Space`/`x` toggles a row.
    Preview,
}

/// Full state for the bulk resolve-conflict flow. The candidate rows are the live `ImportConflict`
/// blockers (from `Session::bulk_resolve_conflict_plan`); `kind` is the batch-wide accept/reject choice.
pub struct BulkResolveFlowState {
    pub step: BulkResolveStep,
    /// The batch-wide Accept/Reject choice (the ONLY param; the shipped `ResolveKind`).
    pub kind: ResolveKind,
    /// Preview checklist over the live conflicts (all start checked).
    pub preview: TargetList<BulkResolveRowItem>,
    /// Cross-step transient error (nothing selected).
    pub error: Option<String>,
}

/// Payload for the bulk resolve-conflict confirmation modal — Tier-B NON-REVOCABLE (NOT typed-word;
/// `SupersedeImport`/`RejectImport` are excluded from `is_revocable_payload`, so a wrong accept/reject
/// CANNOT be voided). Captures the CHECKED rows' conflict-event ids + the count + the action.
pub struct BulkResolveModalState {
    pub kind: ResolveKind,
    pub conflict_events: Vec<EventId>,
    pub count: usize,
}

/// The count of CHECKED rows in a bulk resolve-conflict preview (footer + modal gate).
pub fn bulk_resolve_checked_count(items: &[BulkResolveRowItem]) -> usize {
    items.iter().filter(|i| i.checked).count()
}

// ── Bulk-void flow types (bulk-void D3) ──────────────────────────────────────

/// The persist-layer target of one void in a bulk-void batch: the decision id to void + its
/// precomputed side-effect keys. Precomputed ONCE by `open_bulk_void_flow` from the snapshot (mirrors
/// `Session::bulk_void_plan`), so `persist_bulk_void` never re-loads the log per row.
/// - `disposal_to_clear`: a `LotSelection` target → `Some(ls.disposal_event)` (clear its optimizer
///   attestation on void), else `None`.
/// - `reclass_out_to_clear` [R0-I1]: a `ReclassifyOutflow` target → `Some(ro.transfer_out_event)`
///   (clear its `bulk_estimated` flag on void — else a stale `[est]` survives a void + re-reclassify),
///   else `None`.
#[derive(Clone)]
pub struct VoidTarget {
    pub target_event_id: EventId,
    pub disposal_to_clear: Option<EventId>,
    pub reclass_out_to_clear: Option<EventId>,
}

/// One preview-checklist row for the bulk-void sweep: a voidable decision + its `checked` (included)
/// flag. Every row starts CHECKED; `Space`/`x` toggles `checked` (exclude). The tag/summary are
/// computed at open time via `summarize_void_payload` so the flow/modal render no live event lookups.
#[derive(Clone)]
pub struct BulkVoidRowItem {
    /// The Decision event id to void.
    pub target_event_id: EventId,
    /// `decision|seq` sequence number (table column + deterministic sort).
    pub seq: u64,
    /// Payload tag (`summarize_void_payload`) — e.g. "ClassifyInbound", "LotSelection".
    pub payload_tag: &'static str,
    /// One-line summary of what the void UNDOES (the inner target).
    pub target_summary: String,
    /// Precomputed side-effect target: a `LotSelection` → `Some(ls.disposal_event)` (re-exposes the
    /// disposal to the default method + clears its optimizer attestation on void); else `None`.
    pub disposal_to_clear: Option<EventId>,
    /// [R0-I1] Precomputed side-effect target: a `ReclassifyOutflow` → `Some(ro.transfer_out_event)`
    /// (clears its `bulk_estimated` flag on void); else `None`.
    pub reclass_out_to_clear: Option<EventId>,
    pub checked: bool,
}

/// Full state for the bulk-void flow (a single per-row-exclude checklist step — NO batch-wide param;
/// void is single-valued). The candidate rows are the shared `voidable_decisions` predicate output.
pub struct BulkVoidFlowState {
    /// Preview checklist over the voidable decisions (all start checked).
    pub preview: TargetList<BulkVoidRowItem>,
    /// Cross-step transient error (nothing selected).
    pub error: Option<String>,
}

/// Payload for the bulk-void confirmation modal — Tier-B NON-REVOCABLE + high blast-radius (red border,
/// prominent warning, NOT a typed-word — Tier-C is reserved for the §7.4 attest batch). Captures the
/// CHECKED rows' `VoidTarget`s + the count + how many are `LotSelection` voids (the blast radius).
pub struct BulkVoidModalState {
    pub targets: Vec<VoidTarget>,
    pub count: usize,
    /// How many of the checked voids are `LotSelection`s that re-expose disposals + clear attestations.
    pub lot_selection_count: usize,
}

/// The count of CHECKED rows in a bulk-void preview (footer + modal gate).
pub fn bulk_void_checked_count(items: &[BulkVoidRowItem]) -> usize {
    items.iter().filter(|i| i.checked).count()
}

/// The count of CHECKED rows that are `LotSelection` voids (blast-radius line — `disposal_to_clear` is
/// `Some` exactly for `LotSelection` targets).
pub fn bulk_void_lot_selection_checked_count(items: &[BulkVoidRowItem]) -> usize {
    items
        .iter()
        .filter(|i| i.checked && i.disposal_to_clear.is_some())
        .count()
}

/// The §A.5 basis label for a `Persistability`. `ForbiddenBroker2027` is never persisted (pre-filtered),
/// so it maps to a placeholder that is unreachable at the modal.
pub fn optimize_basis_label(p: Persistability) -> &'static str {
    match p {
        Persistability::ContemporaneousNow => "Contemporaneous",
        Persistability::NeedsAttestation => "AttestedRecording",
        Persistability::ForbiddenBroker2027 => "Forbidden",
    }
}

/// Pre-filter the optimizer proposal's per-disposal rows into persistable candidates (chunk 4b, D4).
///
/// Keeps a row iff ALL hold:
/// - `proposed_selection != current_selection` (a no-change row is the CLI "already optimal" skip);
/// - `persistable != ForbiddenBroker2027` (2027+ broker lots NEVER persist — no attestation cures);
/// - the disposal has NO live (non-voided) `LotSelection` (`already_selected`) — else the append is a
///   DUPLICATE ⇒ `DecisionConflict` NEITHER-applies (the MANDATORY duplicate guard).
pub fn filter_optimize_candidates(
    per_disposal: &[DisposalProposal],
    already_selected: &std::collections::BTreeSet<EventId>,
) -> Vec<OptimizeCandidateItem> {
    per_disposal
        .iter()
        .filter(|d| d.proposed_selection != d.current_selection)
        .filter(|d| d.persistable != Persistability::ForbiddenBroker2027)
        .filter(|d| !already_selected.contains(&d.disposal))
        .map(|d| OptimizeCandidateItem {
            disposal: d.disposal.clone(),
            wallet: d.wallet.clone(),
            date: d.date,
            persistable: d.persistable,
            picks: d.proposed_selection.clone(),
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// A receipt date safely AFTER every acquisition date used in these tests, so the UX-P4-4(b)
    /// acquired-after-receipt guard does not fire on the pre-existing happy-path cases.
    fn any_receipt() -> TaxDate {
        time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap()
    }

    fn make_valid_form() -> ProfileFormState {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("120000");
        f.fields[1].set("130000");
        f.fields[2].set("0");
        f
    }

    // ── KAT-V1: FilingStatus cycles through all 5 variants and wraps ─────────

    #[test]
    fn kat_v1_filing_status_cycles_five_times_returns_to_start() {
        let mut fs = FilingStatus::Single;
        let start = fs;
        for _ in 0..5 {
            fs = cycle_filing_status(fs);
        }
        assert_eq!(fs, start, "5 cycles must return to Single");
    }

    #[test]
    fn kat_v1_all_variants_reachable_in_cycle() {
        let mut seen = std::collections::HashSet::new();
        let mut fs = FilingStatus::Single;
        for _ in 0..5 {
            seen.insert(format!("{fs:?}"));
            fs = cycle_filing_status(fs);
        }
        assert_eq!(seen.len(), 5, "all 5 variants must be reachable");
    }

    // ── KAT-V2..V4: required fields ─────────────────────────────────────────

    #[test]
    fn kat_v2_empty_ordinary_taxable_income_is_required_error() {
        let f = ProfileFormState::new(2025);
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("ordinary-taxable-income") && err.contains("required"),
            "got: {err}"
        );
    }

    #[test]
    fn kat_v3_empty_magi_is_required_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("100000"); // fill required[0]
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("magi-excluding-crypto") && err.contains("required"),
            "got: {err}"
        );
    }

    #[test]
    fn kat_v4_empty_qualified_dividends_is_required_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("100000");
        f.fields[1].set("100000");
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("qualified-dividends") && err.contains("required"),
            "got: {err}"
        );
    }

    // ── KAT-V5..V7: empty optional fields default to 0 ──────────────────────

    #[test]
    fn kat_v5_empty_other_net_capital_gain_defaults_to_zero() {
        let f = make_valid_form();
        let p = validate(&f).unwrap();
        assert_eq!(p.other_net_capital_gain, Usd::ZERO);
    }

    #[test]
    fn kat_v6_empty_carryforward_defaults_to_zero() {
        let f = make_valid_form();
        let p = validate(&f).unwrap();
        assert_eq!(p.capital_loss_carryforward_in.short, Usd::ZERO);
        assert_eq!(p.capital_loss_carryforward_in.long, Usd::ZERO);
    }

    #[test]
    fn kat_v7_empty_optional_defaults_to_zero() {
        let f = make_valid_form();
        let p = validate(&f).unwrap();
        assert_eq!(p.w2_ss_wages, Usd::ZERO);
        assert_eq!(p.w2_medicare_wages, Usd::ZERO);
        assert_eq!(p.schedule_c_expenses, Usd::ZERO);
    }

    // ── KAT-V8..V10: negative optional non-negative fields → error ───────────

    #[test]
    fn kat_v8_negative_w2_ss_wages_is_rejected() {
        let mut f = make_valid_form();
        f.fields[6].set("-1");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("w2-ss-wages"), "got: {err}");
    }

    #[test]
    fn kat_v9_negative_w2_medicare_wages_is_rejected() {
        let mut f = make_valid_form();
        f.fields[7].set("-1");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("w2-medicare-wages"), "got: {err}");
    }

    #[test]
    fn kat_v10_negative_schedule_c_expenses_is_rejected() {
        let mut f = make_valid_form();
        f.fields[8].set("-1");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("schedule-c-expenses"), "got: {err}");
    }

    // ── KAT-V8b..V10b: fields 2–7 accept negatives (CLI parity) ────────────

    #[test]
    fn kat_v8b_negative_values_accepted_for_required_and_optional_fields() {
        // Required fields accept negative (CLI parity: no negativity check for 2-4)
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("-50000"); // ordinary_taxable_income: negative accepted
        f.fields[1].set("-1000");
        f.fields[2].set("-500");
        f.fields[3].set("-100"); // other_net_capital_gain: negative accepted
        f.fields[4].set("-50"); // carryforward_short: negative accepted
        f.fields[5].set("-50"); // carryforward_long: negative accepted
        let p = validate(&f).unwrap();
        assert_eq!(p.ordinary_taxable_income, dec!(-50000));
        assert_eq!(p.other_net_capital_gain, dec!(-100));
        assert_eq!(p.capital_loss_carryforward_in.short, dec!(-50));
    }

    // ── KAT-V11: whitespace-only buffers ────────────────────────────────────

    #[test]
    fn kat_v11_whitespace_only_optional_is_parse_error_not_zero() {
        let mut f = make_valid_form();
        f.fields[3].set("  "); // other_net_capital_gain — whitespace-only
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only optional must be a parse error, not 0; got: {err}"
        );
    }

    #[test]
    fn kat_v11_whitespace_only_required_is_parse_error_not_required_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("  "); // ordinary_taxable_income — whitespace-only
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only required must be parse error, not 'required'; got: {err}"
        );
        assert!(
            !err.contains("required"),
            "must not say 'required' for whitespace-only; got: {err}"
        );
    }

    #[test]
    fn kat_v11_len_zero_required_is_required_error() {
        let f = ProfileFormState::new(2025); // buffers all empty (len==0)
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("required"),
            "len-0 must be 'required' error; got: {err}"
        );
    }

    #[test]
    fn kat_v11_len_zero_optional_is_zero() {
        let f = make_valid_form(); // optional buffers are len-0
        let p = validate(&f).unwrap();
        assert_eq!(p.other_net_capital_gain, Usd::ZERO);
    }

    // ── KAT-V-CI-9: IncomeKind cycles in declaration order, initial = Mining ──

    #[test]
    fn kat_v_ci_9_income_kind_cycles_five_variants_wraps_to_mining() {
        let mut kind = IncomeKind::Mining; // initial [R0-M3]
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Staking);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Interest);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Airdrop);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Reward);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Mining, "5 cycles must wrap to Mining");
    }

    // ── KAT-V-CI-1: Income fmv empty → None (valid, no error) ───────────────

    #[test]
    fn kat_v_ci_1_income_fmv_empty_gives_none() {
        let result =
            validate_classify_inbound_income(IncomeKind::Mining, &FieldBuffer::new(), false);
        let cls = result.unwrap();
        if let InboundClass::Income { fmv, .. } = cls {
            assert!(fmv.is_none(), "empty fmv_buf must produce fmv=None");
        } else {
            panic!("expected Income variant");
        }
    }

    // ── KAT-V-CI-2: Income fmv valid decimal → parses correctly ──────────────

    #[test]
    fn kat_v_ci_2_income_fmv_valid_decimal_parses() {
        use rust_decimal_macros::dec;
        let mut buf = FieldBuffer::new();
        buf.set("45.50");
        let result = validate_classify_inbound_income(IncomeKind::Staking, &buf, false);
        let cls = result.unwrap();
        if let InboundClass::Income {
            fmv,
            kind,
            business,
        } = cls
        {
            assert_eq!(fmv, Some(dec!(45.50)));
            assert_eq!(kind, IncomeKind::Staking);
            assert!(!business);
        } else {
            panic!("expected Income variant");
        }
    }

    // ── KAT-V-CI-3: Income fmv non-numeric → parse error "bad USD…" ──────────

    #[test]
    fn kat_v_ci_3_income_fmv_nonnumeric_is_parse_error() {
        let mut buf = FieldBuffer::new();
        buf.set("abc");
        let err = validate_classify_inbound_income(IncomeKind::Mining, &buf, false).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "non-numeric fmv must produce 'bad USD' error; got: {err}"
        );
    }

    // ── KAT-V-CI-4: Income fmv whitespace-only → parse error (not None) ──────

    #[test]
    fn kat_v_ci_4_income_fmv_whitespace_only_is_parse_error_not_none() {
        let mut buf = FieldBuffer::new();
        buf.set("   "); // whitespace-only: is_empty()==false [R0-M4]
        let err = validate_classify_inbound_income(IncomeKind::Mining, &buf, false).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only fmv must be a parse error, not None; got: {err}"
        );
    }

    // ── KAT-V-CI-5: GiftReceived fmv_at_gift empty → "fmv-at-gift is required" ─

    #[test]
    fn kat_v_ci_5_gift_fmv_at_gift_empty_is_required_error() {
        let err = validate_classify_inbound_gift(
            any_receipt(),
            &FieldBuffer::new(),
            &FieldBuffer::new(),
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("fmv-at-gift") && err.contains("required"),
            "empty fmv_at_gift must produce 'required' error; got: {err}"
        );
    }

    // ── KAT-V-CI-6: GiftReceived fmv_at_gift valid → parses correctly ─────────

    #[test]
    fn kat_v_ci_6_gift_fmv_at_gift_valid_parses() {
        use rust_decimal_macros::dec;
        let mut buf = FieldBuffer::new();
        buf.set("500.00");
        let cls = validate_classify_inbound_gift(
            any_receipt(),
            &buf,
            &FieldBuffer::new(),
            &FieldBuffer::new(),
        )
        .unwrap();
        if let InboundClass::GiftReceived {
            fmv_at_gift,
            donor_basis,
            donor_acquired_at,
        } = cls
        {
            assert_eq!(fmv_at_gift, dec!(500.00));
            assert!(donor_basis.is_none());
            assert!(donor_acquired_at.is_none());
        } else {
            panic!("expected GiftReceived variant");
        }
    }

    // ── KAT-V-CI-7: GiftReceived donor_acquired_at valid YYYY-MM-DD → parses ──

    #[test]
    fn kat_v_ci_7_gift_donor_acquired_at_valid_parses() {
        use time::macros::date;
        let mut fmv_buf = FieldBuffer::new();
        fmv_buf.set("500.00");
        let mut date_buf = FieldBuffer::new();
        date_buf.set("2022-04-01");
        let cls =
            validate_classify_inbound_gift(any_receipt(), &fmv_buf, &FieldBuffer::new(), &date_buf)
                .unwrap();
        if let InboundClass::GiftReceived {
            donor_acquired_at, ..
        } = cls
        {
            assert_eq!(donor_acquired_at, Some(date!(2022 - 04 - 01)));
        } else {
            panic!("expected GiftReceived variant");
        }
    }

    // ── KAT-V-CI-8: GiftReceived donor_acquired_at bad format → "bad date…" ───

    #[test]
    fn kat_v_ci_8_gift_donor_acquired_at_bad_format_is_error() {
        let mut fmv_buf = FieldBuffer::new();
        fmv_buf.set("500.00");
        let mut date_buf = FieldBuffer::new();
        date_buf.set("not-a-date");
        let err =
            validate_classify_inbound_gift(any_receipt(), &fmv_buf, &FieldBuffer::new(), &date_buf)
                .unwrap_err();
        assert!(
            err.contains("bad date"),
            "bad date format must produce 'bad date' error; got: {err}"
        );
    }

    // ── KAT-V-CI-ST: SelfTransferMine validator (Cycle A, Task 3) ────────────

    /// Both fields empty → `SelfTransferMine { basis: None, acquired_at: None }` (the conservative
    /// defaults path; the fold applies $0 + receipt-date and fires the honest advisory).
    #[test]
    fn kat_v_ci_st_1_both_empty_gives_none_none() {
        let cls = validate_classify_inbound_self_transfer(
            any_receipt(),
            &FieldBuffer::new(),
            &FieldBuffer::new(),
        )
        .unwrap();
        if let InboundClass::SelfTransferMine { basis, acquired_at } = cls {
            assert!(basis.is_none(), "empty basis_buf → None");
            assert!(acquired_at.is_none(), "empty acquired_buf → None");
        } else {
            panic!("expected SelfTransferMine variant");
        }
    }

    /// Both fields supplied → parsed basis + acquisition date.
    #[test]
    fn kat_v_ci_st_2_supplied_fields_parse() {
        use rust_decimal_macros::dec;
        use time::macros::date;
        let mut basis = FieldBuffer::new();
        basis.set("1234.56");
        let mut acq = FieldBuffer::new();
        acq.set("2015-01-02");
        let cls = validate_classify_inbound_self_transfer(any_receipt(), &basis, &acq).unwrap();
        if let InboundClass::SelfTransferMine { basis, acquired_at } = cls {
            assert_eq!(basis, Some(dec!(1234.56)));
            assert_eq!(acquired_at, Some(date!(2015 - 01 - 02)));
        } else {
            panic!("expected SelfTransferMine variant");
        }
    }

    /// An explicit `0` basis → `Some(0)` (attested zero-cost — the fold honors it WITHOUT the advisory).
    #[test]
    fn kat_v_ci_st_3_explicit_zero_basis_is_some_zero() {
        use rust_decimal_macros::dec;
        let mut basis = FieldBuffer::new();
        basis.set("0");
        let cls =
            validate_classify_inbound_self_transfer(any_receipt(), &basis, &FieldBuffer::new())
                .unwrap();
        if let InboundClass::SelfTransferMine { basis, .. } = cls {
            assert_eq!(basis, Some(dec!(0)), "explicit 0 → Some(0), NOT None");
        } else {
            panic!("expected SelfTransferMine variant");
        }
    }

    /// [R0-M4] Whitespace-only basis is NOT empty → a parse error (not silently None).
    #[test]
    fn kat_v_ci_st_4_whitespace_only_basis_is_parse_error() {
        let mut basis = FieldBuffer::new();
        basis.set("   ");
        let err =
            validate_classify_inbound_self_transfer(any_receipt(), &basis, &FieldBuffer::new())
                .unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace basis → parse error; got: {err}"
        );
    }

    /// Bad date format → "bad date" error.
    #[test]
    fn kat_v_ci_st_5_bad_date_is_error() {
        let mut acq = FieldBuffer::new();
        acq.set("not-a-date");
        let err = validate_classify_inbound_self_transfer(any_receipt(), &FieldBuffer::new(), &acq)
            .unwrap_err();
        assert!(err.contains("bad date"), "bad date → error; got: {err}");
    }

    // ── UX-P4-4 folds: I1 (negative money, BOTH surfaces) + I2 (acquired>receipt, BOTH surfaces) ──
    // The CLI refuses these at record time; these KATs prove the TUI validators — the sibling record
    // surface named by SPEC:223 "negative basis refused on BOTH surfaces" — refuse them too. Each dies
    // under mutation (neuter `parse_nonneg_usd` / `check_acquired_not_after_receipt`).

    /// I1 — income `fmv < 0` refused.
    #[test]
    fn ux_p4_4_income_negative_fmv_refused() {
        let mut fmv = FieldBuffer::new();
        fmv.set("-5000");
        let err = validate_classify_inbound_income(IncomeKind::Reward, &fmv, false).unwrap_err();
        assert!(err.contains("fmv") && err.contains(">= 0"), "got: {err}");
    }

    /// I1 — gift `fmv-at-gift < 0` and `donor-basis < 0` refused.
    #[test]
    fn ux_p4_4_gift_negative_money_refused() {
        let mut neg = FieldBuffer::new();
        neg.set("-5000");
        let mut fmv = FieldBuffer::new();
        fmv.set("100");
        let err = validate_classify_inbound_gift(
            any_receipt(),
            &neg,
            &FieldBuffer::new(),
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("fmv-at-gift") && err.contains(">= 0"),
            "got: {err}"
        );
        let err = validate_classify_inbound_gift(any_receipt(), &fmv, &neg, &FieldBuffer::new())
            .unwrap_err();
        assert!(
            err.contains("donor-basis") && err.contains(">= 0"),
            "got: {err}"
        );
    }

    /// I1 — self-transfer `basis < 0` refused; `0` still allowed (attested zero-cost).
    #[test]
    fn ux_p4_4_self_transfer_negative_basis_refused_zero_allowed() {
        let mut neg = FieldBuffer::new();
        neg.set("-5000");
        let err = validate_classify_inbound_self_transfer(any_receipt(), &neg, &FieldBuffer::new())
            .unwrap_err();
        assert!(err.contains("basis") && err.contains(">= 0"), "got: {err}");
        let mut zero = FieldBuffer::new();
        zero.set("0");
        assert!(
            validate_classify_inbound_self_transfer(any_receipt(), &zero, &FieldBuffer::new())
                .is_ok(),
            "zero basis (attested zero-cost) must still be allowed"
        );
    }

    /// I1 — reclassify-outflow `amount < 0` and `fee < 0` refused.
    #[test]
    fn ux_p4_4_reclassify_outflow_negative_money_refused() {
        let item = dummy_outflow_item();
        let mut neg = FieldBuffer::new();
        neg.set("-5000");
        let mut amt = FieldBuffer::new();
        amt.set("640");
        let err = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &neg,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(err.contains("amount") && err.contains(">= 0"), "got: {err}");
        let err = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amt,
            &neg,
            false,
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(err.contains("fee") && err.contains(">= 0"), "got: {err}");
    }

    /// I1 — set-fmv `usd-fmv < 0` refused.
    #[test]
    fn ux_p4_4_set_fmv_negative_refused() {
        let item = dummy_fmv_item();
        let mut neg = FieldBuffer::new();
        neg.set("-5000");
        let err = validate_set_fmv(&item, &neg).unwrap_err();
        assert!(
            err.contains("usd-fmv") && err.contains(">= 0"),
            "got: {err}"
        );
    }

    /// I2 — self-transfer `acquired` strictly AFTER the receipt refused; same-day allowed.
    #[test]
    fn ux_p4_4_self_transfer_acquired_after_receipt_refused_same_day_ok() {
        use time::macros::date;
        let receipt = date!(2025 - 03 - 01);
        let mut after = FieldBuffer::new();
        after.set("2025-03-02");
        let err = validate_classify_inbound_self_transfer(receipt, &FieldBuffer::new(), &after)
            .unwrap_err();
        assert!(
            err.contains("acquired") && err.contains("2025-03-01") && err.contains("receipt"),
            "got: {err}"
        );
        let mut same = FieldBuffer::new();
        same.set("2025-03-01");
        assert!(
            validate_classify_inbound_self_transfer(receipt, &FieldBuffer::new(), &same).is_ok(),
            "same-day acquired must be allowed"
        );
    }

    /// I2 — gift `donor-acquired` strictly AFTER the receipt refused.
    #[test]
    fn ux_p4_4_gift_donor_acquired_after_receipt_refused() {
        use time::macros::date;
        let receipt = date!(2025 - 03 - 01);
        let mut fmv = FieldBuffer::new();
        fmv.set("100");
        let mut after = FieldBuffer::new();
        after.set("2025-03-02");
        let err =
            validate_classify_inbound_gift(receipt, &fmv, &FieldBuffer::new(), &after).unwrap_err();
        assert!(
            err.contains("donor-acquired") && err.contains("2025-03-01") && err.contains("receipt"),
            "got: {err}"
        );
    }

    // ── Parse failure: non-numeric ───────────────────────────────────────────

    #[test]
    fn non_numeric_required_field_is_parse_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("abc");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("bad USD"), "got: {err}");
    }

    // ── Full valid form round-trips ──────────────────────────────────────────

    #[test]
    fn valid_form_produces_correct_tax_profile() {
        let mut f = ProfileFormState::new(2025);
        f.filing_status = FilingStatus::Mfj;
        f.fields[0].set("120000");
        f.fields[1].set("130000");
        f.fields[2].set("5000");
        f.fields[3].set("1000");
        f.fields[4].set("500");
        f.fields[5].set("250");
        f.fields[6].set("80000");
        f.fields[7].set("85000");
        f.fields[8].set("3000");
        let p = validate(&f).unwrap();
        assert_eq!(p.filing_status, FilingStatus::Mfj);
        assert_eq!(p.ordinary_taxable_income, dec!(120000));
        assert_eq!(p.magi_excluding_crypto, dec!(130000));
        assert_eq!(p.qualified_dividends_and_other_pref_income, dec!(5000));
        assert_eq!(p.other_net_capital_gain, dec!(1000));
        assert_eq!(p.capital_loss_carryforward_in.short, dec!(500));
        assert_eq!(p.capital_loss_carryforward_in.long, dec!(250));
        assert_eq!(p.w2_ss_wages, dec!(80000));
        assert_eq!(p.w2_medicare_wages, dec!(85000));
        assert_eq!(p.schedule_c_expenses, dec!(3000));
    }

    // ── Helper: build a minimal OutflowListItem for validation tests ──────────

    fn dummy_outflow_item() -> OutflowListItem {
        use btctax_core::{
            identity::{Source, SourceRef},
            EventId,
        };
        use time::{OffsetDateTime, UtcOffset};
        OutflowListItem {
            transfer_out_event: EventId::import(Source::River, SourceRef::new("test-ro-1")),
            date: btctax_core::conventions::tax_date(
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
            ),
            principal_sat: 1_000_000,
            wallet: None,
        }
    }

    // ── KAT-V-RO-1: amount empty → "amount is required" ─────────────────────

    #[test]
    fn kat_v_ro_1_amount_empty_is_required_error() {
        let item = dummy_outflow_item();
        let err = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &FieldBuffer::new(),
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("amount") && err.contains("required"),
            "empty amount must produce 'required' error; got: {err}"
        );
    }

    // ── KAT-V-RO-2: amount valid decimal → parses correctly ──────────────────

    #[test]
    fn kat_v_ro_2_amount_valid_decimal_parses() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert_eq!(ro.principal_proceeds_or_fmv, dec!(640.00));
        assert!(matches!(
            ro.as_,
            OutflowClass::Dispose {
                kind: DisposeKind::Sell
            }
        ));
        assert!(ro.fee_usd.is_none());
        assert!(ro.donee.is_none());
    }

    // ── KAT-V-RO-3: amount whitespace-only → parse error (not "required") ────

    #[test]
    fn kat_v_ro_3_amount_whitespace_only_is_parse_error_not_required() {
        let item = dummy_outflow_item();
        let mut buf = FieldBuffer::new();
        buf.set("   "); // whitespace-only: is_empty()==false [R0-M4]
        let err = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only amount must be a parse error; got: {err}"
        );
        assert!(
            !err.contains("required"),
            "whitespace-only amount must NOT say 'required'; got: {err}"
        );
    }

    // ── KAT-V-RO-4: fee empty → None (no error) ──────────────────────────────

    #[test]
    fn kat_v_ro_4_fee_empty_gives_none() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(ro.fee_usd.is_none(), "empty fee must produce None");
    }

    // ── KAT-V-RO-5: fee valid → parses correctly ─────────────────────────────

    #[test]
    fn kat_v_ro_5_fee_valid_parses() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");
        let mut fee_buf = FieldBuffer::new();
        fee_buf.set("2.50");
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amount_buf,
            &fee_buf,
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert_eq!(ro.fee_usd, Some(dec!(2.50)));
    }

    // ── KAT-V-RO-6: appraisal toggle default false; Space toggles ────────────

    #[test]
    fn kat_v_ro_6_appraisal_toggle_default_false_then_true() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");

        // Default: appraisal=false
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Donate,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(matches!(
            ro.as_,
            OutflowClass::Donate {
                appraisal_required: false
            }
        ));

        // Toggled: appraisal=true
        let ro2 = validate_reclassify_outflow(
            &item,
            OutflowKind::Donate,
            &amount_buf,
            &FieldBuffer::new(),
            true,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(matches!(
            ro2.as_,
            OutflowClass::Donate {
                appraisal_required: true
            }
        ));
    }

    // ── KAT-V-RO-7: donee empty → None; non-empty → Some(trimmed) ────────────

    #[test]
    fn kat_v_ro_7_donee_empty_is_none_nonempty_is_some_trimmed() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");

        // Empty → None
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Gift,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(ro.donee.is_none(), "empty donee must produce None");

        // Non-empty (with leading/trailing spaces) → Some(trimmed)
        let mut donee_buf = FieldBuffer::new();
        donee_buf.set("  Alice  ");
        let ro2 = validate_reclassify_outflow(
            &item,
            OutflowKind::Gift,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &donee_buf,
        )
        .unwrap();
        assert_eq!(
            ro2.donee,
            Some("Alice".to_string()),
            "non-empty donee must be Some(trimmed)"
        );
    }

    // ── KAT-V-RO-8: OutflowKind Tab cycles: sell → spend → gift → donate → sell ─

    #[test]
    fn kat_v_ro_8_outflow_kind_cycles_four_variants_wraps_to_sell() {
        let mut kind = OutflowKind::Sell; // initial
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Spend);
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Gift);
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Donate);
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Sell, "4 cycles must wrap to Sell");
    }

    // ── Helper: build a minimal IncomeListItem for RI validation tests ────────

    fn dummy_income_item() -> IncomeListItem {
        use btctax_core::{
            identity::{Source, SourceRef},
            EventId,
        };
        use time::{OffsetDateTime, UtcOffset};
        IncomeListItem {
            income_event: EventId::import(Source::River, SourceRef::new("test-ri-1")),
            date: btctax_core::conventions::tax_date(
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
            ),
            sat: 100_000,
            kind: IncomeKind::Reward,
            business: false,
            fmv: None,
            wallet: None,
        }
    }

    // ── Helper: build a minimal FmvListItem for FMV validation tests ──────────

    fn dummy_fmv_item() -> FmvListItem {
        use btctax_core::{
            identity::{Source, SourceRef},
            EventId,
        };
        use time::{OffsetDateTime, UtcOffset};
        FmvListItem {
            event: EventId::import(Source::River, SourceRef::new("test-fmv-1")),
            date: btctax_core::conventions::tax_date(
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
            ),
            sat: 100_000,
            kind: IncomeKind::Staking,
            wallet: None,
        }
    }

    // ── KAT-RI-REQUIRED-BUSINESS: business=None → Err "business is required" ─

    #[test]
    fn kat_ri_required_business_none_is_error() {
        let item = dummy_income_item();
        let err = validate_reclassify_income(&item, None, None).unwrap_err();
        assert!(
            err.contains("business is required"),
            "None business must produce 'business is required' error; got: {err}"
        );
    }

    // ── KAT-V-RI-1: business=None → blocked ─────────────────────────────────

    #[test]
    fn kat_v_ri_1_business_none_is_blocked() {
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, None, None);
        assert!(result.is_err(), "None business must be blocked");
        let err = result.unwrap_err();
        assert!(
            err.contains("required"),
            "error must say 'required'; got: {err}"
        );
    }

    // ── KAT-V-RI-2: business=Some(true), kind=None → valid ──────────────────

    #[test]
    fn kat_v_ri_2_business_true_kind_none_is_valid() {
        use btctax_core::EventPayload;
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, Some(true), None);
        let payload = result.unwrap();
        if let EventPayload::ReclassifyIncome(ri) = payload {
            assert!(ri.business, "business must be true");
            assert!(ri.kind.is_none(), "kind must be None (keep original)");
        } else {
            panic!("expected ReclassifyIncome");
        }
    }

    // ── KAT-V-RI-3: business=Some(false), kind=Some(Mining) → valid ──────────

    #[test]
    fn kat_v_ri_3_business_false_kind_mining_is_valid() {
        use btctax_core::EventPayload;
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, Some(false), Some(IncomeKind::Mining));
        let payload = result.unwrap();
        if let EventPayload::ReclassifyIncome(ri) = payload {
            assert!(!ri.business, "business must be false");
            assert_eq!(ri.kind, Some(IncomeKind::Mining));
        } else {
            panic!("expected ReclassifyIncome");
        }
    }

    // ── KAT-V-RI-4: business=Some(true), kind=Some(Reward) → valid ──────────

    #[test]
    fn kat_v_ri_4_business_true_kind_reward_is_valid() {
        use btctax_core::EventPayload;
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, Some(true), Some(IncomeKind::Reward));
        let payload = result.unwrap();
        if let EventPayload::ReclassifyIncome(ri) = payload {
            assert!(ri.business);
            assert_eq!(ri.kind, Some(IncomeKind::Reward));
        } else {
            panic!("expected ReclassifyIncome");
        }
    }

    // ── KAT-V-FMV-1: empty buf → Err "usd-fmv is required" ──────────────────

    #[test]
    fn kat_v_fmv_1_empty_buf_is_required_error() {
        let item = dummy_fmv_item();
        let err = validate_set_fmv(&item, &FieldBuffer::new()).unwrap_err();
        assert!(
            err.contains("usd-fmv") && err.contains("required"),
            "empty buf must produce 'usd-fmv is required' error; got: {err}"
        );
    }

    // ── KAT-V-FMV-2: "45.00" → valid, usd_fmv=45.00 ─────────────────────────

    #[test]
    fn kat_v_fmv_2_valid_decimal_parses() {
        use btctax_core::EventPayload;
        use rust_decimal_macros::dec;
        let item = dummy_fmv_item();
        let mut buf = FieldBuffer::new();
        buf.set("45.00");
        let payload = validate_set_fmv(&item, &buf).unwrap();
        if let EventPayload::ManualFmv(mf) = payload {
            assert_eq!(mf.usd_fmv, dec!(45.00));
        } else {
            panic!("expected ManualFmv");
        }
    }

    // ── KAT-V-FMV-3: whitespace-only → parse error, NOT "required" [R0-M4] ───

    #[test]
    fn kat_v_fmv_3_whitespace_only_is_parse_error_not_required() {
        let item = dummy_fmv_item();
        let mut buf = FieldBuffer::new();
        buf.set("   "); // whitespace-only: is_empty()==false [R0-M4]
        let err = validate_set_fmv(&item, &buf).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only usd-fmv must be a parse error; got: {err}"
        );
        assert!(
            !err.contains("required"),
            "whitespace-only usd-fmv must NOT say 'required'; got: {err}"
        );
    }

    // ── Extra: non-numeric usd-fmv → parse error ─────────────────────────────

    #[test]
    fn set_fmv_non_numeric_is_parse_error() {
        let item = dummy_fmv_item();
        let mut buf = FieldBuffer::new();
        buf.set("abc");
        let err = validate_set_fmv(&item, &buf).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "non-numeric must produce 'bad USD' error; got: {err}"
        );
    }

    // ── Cycle pins: business 3-state and optional-kind 6-state ───────────────
    //
    // KAT-RI-REQUIRED-BUSINESS cycle pins (spec D5): Tab cycles
    // None → Some(true) → Some(false) → None.

    #[test]
    fn kat_ri_business_optional_cycles_three_states_and_wraps() {
        let mut b: Option<bool> = None; // initial: not chosen (REQUIRED-EXPLICIT)
        b = cycle_business_optional(b);
        assert_eq!(b, Some(true));
        b = cycle_business_optional(b);
        assert_eq!(b, Some(false));
        b = cycle_business_optional(b);
        assert_eq!(b, None, "3 cycles must wrap back to None");
        b = cycle_business_optional(b);
        assert_eq!(b, Some(true), "4th cycle lands on Some(true) again");
    }

    // Optional-kind picker: None → Mining → Staking → Interest → Airdrop →
    // Reward → None (spec D1: None = keep original).

    #[test]
    fn kat_ri_kind_optional_cycles_six_states_and_wraps() {
        let mut k: Option<IncomeKind> = None; // initial: keep original
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Mining));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Staking));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Interest));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Airdrop));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Reward));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, None, "6 cycles must wrap back to None (keep original)");
    }

    // ── KAT-V-RO-9: amount label is "gross proceeds" for sell/spend; "FMV" for gift/donate ─

    #[test]
    fn kat_v_ro_9_amount_label_per_kind() {
        assert!(
            amount_label(OutflowKind::Sell).contains("gross proceeds"),
            "sell must have 'gross proceeds' label; got: {}",
            amount_label(OutflowKind::Sell)
        );
        assert!(
            amount_label(OutflowKind::Spend).contains("gross proceeds"),
            "spend must have 'gross proceeds' label [R0-I3]; got: {}",
            amount_label(OutflowKind::Spend)
        );
        assert!(
            amount_label(OutflowKind::Gift).contains("FMV"),
            "gift must have 'FMV' label; got: {}",
            amount_label(OutflowKind::Gift)
        );
        assert!(
            amount_label(OutflowKind::Donate).contains("FMV"),
            "donate must have 'FMV' label; got: {}",
            amount_label(OutflowKind::Donate)
        );
    }

    // ── KAT-V-SL-1..3 — select-lots validation ───────────────────────────────

    fn dummy_disposal_item(principal_sat: Sat) -> DisposalListItem {
        DisposalListItem {
            disposal_event: btctax_core::EventId::Decision { seq: 1 },
            date: time::macros::date!(2025 - 09 - 15),
            kind: DisposalKind::Sell,
            principal_sat,
            wallet: None,
        }
    }

    fn dummy_lot_row(lot_sat: Sat, pick: &str) -> LotPickFormRow {
        let mut buf = FieldBuffer::new();
        buf.set(pick);
        LotPickFormRow {
            lot_id: btctax_core::LotId {
                origin_event_id: btctax_core::EventId::Decision { seq: 99 },
                split_sequence: 0,
            },
            remaining_sat: lot_sat,
            acquired_at: time::macros::date!(2024 - 01 - 01),
            usd_basis: rust_decimal_macros::dec!(30000),
            pick_sat_buf: buf,
        }
    }

    /// KAT-V-SL-1: all picks zero → "pick at least one lot".
    #[test]
    fn kat_v_sl_1_all_picks_zero_is_required_error() {
        let item = dummy_disposal_item(100_000);
        let rows = vec![
            dummy_lot_row(60_000, ""),  // empty = 0
            dummy_lot_row(40_000, "0"), // explicit zero
        ];
        let err = validate_select_lots(&item, &rows).unwrap_err();
        assert!(
            err.contains("pick at least one lot"),
            "all-zero picks must produce 'pick at least one lot'; got: {err}"
        );
    }

    /// KAT-V-SL-2: Σ picked_sat < principal_sat → error.
    #[test]
    fn kat_v_sl_2_underpick_is_principal_mismatch_error() {
        let item = dummy_disposal_item(100_000);
        let rows = vec![dummy_lot_row(100_000, "50000")]; // 50k < 100k
        let err = validate_select_lots(&item, &rows).unwrap_err();
        assert!(
            err.contains("50000 sat") && err.contains("100000 sat"),
            "underpick must mention totals; got: {err}"
        );
        assert!(
            !err.contains("pick at least"),
            "error must NOT say 'pick at least one lot' for underpick; got: {err}"
        );
    }

    /// KAT-V-SL-3: Σ picked_sat == principal_sat → valid; builds correct LotPick list.
    #[test]
    fn kat_v_sl_3_exact_pick_is_valid_builds_lot_pick_list() {
        let item = dummy_disposal_item(100_000);
        let lot_id = btctax_core::LotId {
            origin_event_id: btctax_core::EventId::Decision { seq: 42 },
            split_sequence: 1,
        };
        let mut buf = FieldBuffer::new();
        buf.set("100000");
        let rows = vec![LotPickFormRow {
            lot_id: lot_id.clone(),
            remaining_sat: 100_000,
            acquired_at: time::macros::date!(2024 - 06 - 01),
            usd_basis: rust_decimal_macros::dec!(50000),
            pick_sat_buf: buf,
        }];
        let payload = validate_select_lots(&item, &rows).unwrap();
        match payload {
            btctax_core::EventPayload::LotSelection(ls) => {
                assert_eq!(ls.disposal_event, item.disposal_event);
                assert_eq!(ls.lots.len(), 1);
                assert_eq!(ls.lots[0].lot, lot_id);
                assert_eq!(ls.lots[0].sat, 100_000);
            }
            other => panic!("expected LotSelection, got {other:?}"),
        }
    }

    /// KAT-V-SL-4 (SL-r2-b / review r2 M-1): a per-row pick exceeding that row's Remaining must be rejected
    /// AT FORM VALIDATION — even when Σ still equals the principal. Without a per-row cap the form persists a
    /// pick the engine rejects (`selection_feasible` → hard `LotSelectionInvalid` → tax NotComputable). Here
    /// row-A has 30k available but is picked for 80k; row-B (70k avail) picks 20k; Σ = 100k = principal, so
    /// the principal-conservation check alone would (wrongly) pass.
    #[test]
    fn kat_v_sl_4_per_row_overdraw_is_rejected_even_when_sum_matches() {
        let item = dummy_disposal_item(100_000);
        let rows = vec![
            dummy_lot_row(30_000, "80000"), // 80k picked > 30k available — infeasible
            dummy_lot_row(70_000, "20000"), // 20k ≤ 70k — fine
        ];
        let err = validate_select_lots(&item, &rows).unwrap_err();
        assert!(
            err.contains("80000") && err.contains("30000"),
            "the per-row cap error must name the over-picked amount and the row's Remaining; got: {err}"
        );
        assert!(
            !err.contains("pick at least"),
            "must NOT be the all-zero error; got: {err}"
        );
    }

    // ── KAT-V-DD-1..3 — set-donation-details validation ──────────────────────

    fn empty_bufs() -> [FieldBuffer; 10] {
        std::array::from_fn(|_| FieldBuffer::new())
    }

    fn call_validate_dd(bufs: &[FieldBuffer; 10]) -> Result<DonationDetails, String> {
        validate_donation_details(
            &bufs[0], &bufs[1], &bufs[2], &bufs[3], &bufs[4], &bufs[5], &bufs[6], &bufs[7],
            &bufs[8], &bufs[9],
        )
    }

    /// KAT-V-DD-1: donee_name empty → "donee-name is required".
    #[test]
    fn kat_v_dd_1_donee_name_empty_is_required_error() {
        let bufs = empty_bufs();
        let err = call_validate_dd(&bufs).unwrap_err();
        assert!(
            err.contains("donee-name") && err.contains("required"),
            "empty donee_name must say 'donee-name is required'; got: {err}"
        );
    }

    /// KAT-V-DD-2: appraiser_name empty → "appraiser-name is required".
    #[test]
    fn kat_v_dd_2_appraiser_name_empty_is_required_error() {
        let mut bufs = empty_bufs();
        bufs[0].set("Community Foundation"); // fill donee_name
        let err = call_validate_dd(&bufs).unwrap_err();
        assert!(
            err.contains("appraiser-name") && err.contains("required"),
            "empty appraiser_name must say 'appraiser-name is required'; got: {err}"
        );
    }

    /// KAT-V-DD-3: appraisal_date non-empty with bad format → parse error.
    #[test]
    fn kat_v_dd_3_bad_appraisal_date_format_is_error() {
        let mut bufs = empty_bufs();
        bufs[0].set("Community Foundation");
        bufs[3].set("Jane Appraiser");
        bufs[8].set("not-a-date"); // appraisal_date = bad format
        let err = call_validate_dd(&bufs).unwrap_err();
        assert!(
            err.contains("bad date"),
            "bad appraisal_date must produce 'bad date' error; got: {err}"
        );
    }

    // KAT-V-DD-4 (pre-population round-trip) is implemented in `main.rs` as
    // `kat_v_dd_4_pre_population_drives_real_path`. The prior version here
    // re-implemented the production List→FieldForm pre-population mapping IN the test
    // body — round-1 whole-branch review [I1] found it to be coverage theatre (dropping
    // a production optional-field pre-population passed uncaught). The real-path version
    // drives `d` → List → Enter → FieldForm so a wiring regression in any of the 10
    // fields fails, then Enter → modal to assert the validator round-trip.

    // ── KAT-OA-FILTER — optimize-accept pre-filter (chunk 4b, D4) ────────────
    //
    // filter_optimize_candidates keeps ONLY rows where proposed != current AND persistable !=
    // ForbiddenBroker2027 AND the disposal has NO live LotSelection. Constructs `DisposalProposal`s
    // by hand (all pub) to pin ForbiddenBroker2027-excluded + live-LotSelection-excluded + no-change
    // excluded, mirroring the spec's optimize-accept KAT bullets.
    #[test]
    fn kat_oa_filter_excludes_nochange_forbidden_and_already_selected() {
        use btctax_core::identity::{LotId, Source, SourceRef};
        use btctax_core::project::ComplianceStatus;
        use btctax_core::{DisposalProposal, EventId, LotPick, Persistability, WalletId};
        use time::macros::date;

        let wallet = WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        };
        let d = |tag: &str| EventId::import(Source::River, SourceRef::new(tag));
        let pick = |tag: &str, sat: i64| LotPick {
            lot: LotId {
                origin_event_id: d(tag),
                split_sequence: 0,
            },
            sat,
        };
        let row = |disp: EventId, cur: Vec<LotPick>, prop: Vec<LotPick>, p: Persistability| {
            DisposalProposal {
                disposal: disp,
                wallet: wallet.clone(),
                date: date!(2025 - 05 - 23),
                current_selection: cur,
                proposed_selection: prop,
                status: ComplianceStatus::NonCompliant,
                persistable: p,
            }
        };

        let a = d("oa-A"); // ContemporaneousNow, changed → KEPT
        let b = d("oa-B"); // no-change → EXCLUDED
        let c = d("oa-C"); // ForbiddenBroker2027, changed → EXCLUDED
        let sel = d("oa-D"); // NeedsAttestation, changed, already-selected → EXCLUDED
        let e = d("oa-E"); // NeedsAttestation, changed → KEPT

        let per_disposal = vec![
            row(
                a.clone(),
                vec![pick("lotA1", 100)],
                vec![pick("lotA2", 100)],
                Persistability::ContemporaneousNow,
            ),
            row(
                b.clone(),
                vec![pick("lotB", 100)],
                vec![pick("lotB", 100)],
                Persistability::ContemporaneousNow,
            ),
            row(
                c.clone(),
                vec![pick("lotC1", 100)],
                vec![pick("lotC2", 100)],
                Persistability::ForbiddenBroker2027,
            ),
            row(
                sel.clone(),
                vec![pick("lotD1", 100)],
                vec![pick("lotD2", 100)],
                Persistability::NeedsAttestation,
            ),
            row(
                e.clone(),
                vec![pick("lotE1", 100)],
                vec![pick("lotE2", 100)],
                Persistability::NeedsAttestation,
            ),
        ];

        let already_selected: std::collections::BTreeSet<EventId> = [sel.clone()].into();
        let kept = filter_optimize_candidates(&per_disposal, &already_selected);
        let kept_ids: Vec<EventId> = kept.iter().map(|k| k.disposal.clone()).collect();

        assert!(kept_ids.contains(&a), "ContemporaneousNow changed row kept");
        assert!(kept_ids.contains(&e), "NeedsAttestation changed row kept");
        assert!(!kept_ids.contains(&b), "no-change row excluded");
        assert!(!kept_ids.contains(&c), "ForbiddenBroker2027 excluded");
        assert!(
            !kept_ids.contains(&sel),
            "already-selected (live LotSelection) excluded"
        );
        assert_eq!(kept.len(), 2, "exactly A and E survive");

        // The kept ContemporaneousNow row carries its proposed picks + verdict.
        let a_item = kept.iter().find(|k| k.disposal == a).unwrap();
        assert_eq!(a_item.persistable, Persistability::ContemporaneousNow);
        assert_eq!(a_item.picks, vec![pick("lotA2", 100)]);
        assert_eq!(optimize_basis_label(a_item.persistable), "Contemporaneous");
        let e_item = kept.iter().find(|k| k.disposal == e).unwrap();
        assert_eq!(
            optimize_basis_label(e_item.persistable),
            "AttestedRecording"
        );
    }
}
