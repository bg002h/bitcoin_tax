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

use crate::edit::form::{filing_status_field, live_fields, live_sections, TaxInputsFormState};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::Usd;
use btctax_input_form::{
    apply, parse, parse_ip_pin, parse_ssn, ApplyError, Edit, Field, FieldId, FieldKind, FieldValue,
    ParseError, RowAddr, SectionKind, SetError,
};

// ── Focused-field projection ─────────────────────────────────────────────────────────────────────────

/// The currently-focused editable [`Field`], or `None` when the selected section exposes no editable field
/// (a repeating group, or an absent optional-singleton — those get their own row/create keys in Task 5).
/// On a `None` working copy the ONLY field is the filing-status choice (NI-2).
pub fn focused_field(form: &TaxInputsFormState) -> Option<&'static Field> {
    let Some(ri) = form.working.as_ref() else {
        return Some(filing_status_field());
    };
    let sections = live_sections(ri);
    let sel = form.section_idx.min(sections.len().checked_sub(1)?);
    let section = *sections.get(sel)?;
    match section.kind {
        SectionKind::Repeating { .. } => None,
        SectionKind::OptionalSingleton { present, .. } if !present(ri) => None,
        _ => live_fields(section, ri).get(form.field_focus).copied(),
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
    matches!(k, FieldKind::Enum(_) | FieldKind::TriState | FieldKind::Bool)
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
    form.buf.set(&seed);
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

// ── Internals ────────────────────────────────────────────────────────────────────────────────────────

/// Apply an already-built `Edit` to `form.working`. On `Ok`: clear the error and re-clamp focus (a
/// materialization or a section create/delete changes the live set). On `Err`: surface it in `form.error`
/// and mutate nothing (never a panic — a bad `RowAddr`/`WrongKind`/`Immutable` is a clean error).
fn apply_edit(form: &mut TaxInputsFormState, edit: Edit) -> bool {
    match apply(&mut form.working, edit) {
        Ok(()) => {
            form.error = None;
            clamp_focus(form);
            true
        }
        Err(e) => {
            form.error = Some(apply_error_msg(e));
            false
        }
    }
}

/// Re-clamp `section_idx`/`field_focus` into the CURRENT live set after a successful apply.
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
    let n_fields = live_fields(sections[form.section_idx], ri).len();
    form.field_focus = form.field_focus.min(n_fields.saturating_sub(1));
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
