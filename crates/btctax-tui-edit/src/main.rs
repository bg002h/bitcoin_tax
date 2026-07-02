//! btctax-tui-edit entry point, event loop, and key dispatch.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! Terminal lifecycle: identical to the viewer's (raw mode + alt screen; TerminalGuard
//! RAII + panic hook; `restore_terminal` called explicitly for belt-and-suspenders).
//! This module performs no writes.

mod draw_edit;
mod edit;
mod editor;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use edit::form::{
    cycle_filing_status, cycle_income_kind, cycle_outflow_kind, income_kind_display, next_focus,
    prev_focus, validate, validate_classify_inbound_gift, validate_classify_inbound_income,
    validate_reclassify_outflow, ClassifyInboundFlowState, ClassifyInboundModalState,
    ClassifyInboundStep, FieldBuffer, InboundListItem, InboundVariant, MutationModalState,
    OutflowKind, OutflowListItem, ProfileFormState, ReclassifyOutflowFlowState,
    ReclassifyOutflowModalState, ReclassifyOutflowStep, TargetList,
};
use editor::{EditorApp, EditorScreen};
use ratatui::{backend::CrosstermBackend, widgets::TableState, Terminal};
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use btctax_core::{
    BlockerKind, ClassifyInbound, DisposeKind, EventId, EventPayload, InboundClass, IncomeKind,
    OutflowClass,
};
use btctax_tui::app::Tab;
use btctax_tui::{restore_terminal, setup_panic_hook, TerminalGuard};

// ── Argument parsing ──────────────────────────────────────────────────────────

/// Parse the vault path from CLI arguments.
///
/// Mirrors the viewer's `parse_vault_path` — accepts `--vault <path>` or a
/// bare positional argument; falls back to `~/Documents/BitcoinTax/vault.pgp`.
fn parse_vault_path() -> PathBuf {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--vault" {
            if i + 1 < args.len() {
                return PathBuf::from(&args[i + 1]);
            }
        } else if !args[i].starts_with('-') {
            return PathBuf::from(&args[i]);
        }
        i += 1;
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Documents/BitcoinTax/vault.pgp"))
        .unwrap_or_else(|| PathBuf::from("vault.pgp"))
}

// ── Key dispatch ──────────────────────────────────────────────────────────────

/// Map a key press to an `EditorApp` state transition.
///
/// Only KEY PRESS events are acted on (release/repeat ignored).
///
/// **Dispatch order** (modal → flow → form → screen — the R0-M4 lesson: Esc must never
/// fall through to a quit arm mid-flow or mid-modal):
/// 1. Mutation-modal dispatch — BEFORE flow, form and screen dispatch.
/// 2. Classify-inbound-modal dispatch — BEFORE flow, form and screen dispatch.
/// 3. Reclassify-outflow-modal dispatch — BEFORE flow, form and screen dispatch.
/// 4. Flow dispatch — ANY open flow claims ALL keys at every step [R0-I2].
/// 5. Form dispatch — BEFORE screen dispatch.
/// 6. Screen dispatch (Unlock / Locked / Browse).
///
/// # Screen dispatch
/// - **Unlock**: `Esc` → quit; `Tab`/`BackTab` → ignored (no tab bar); `Enter` →
///   attempt open; `Backspace` → pop char; any `Char` → append to buffer.
/// - **Locked**: `r` → retry (back to Unlock); `q`/`Esc` → quit.
/// - **Browse**: `q`/`Esc` → quit; `Tab` → next tab; `BackTab` → prev tab;
///   `←/→` → year change + reset selections; `↑/↓ j/k` → scroll;
///   `PgUp/PgDn` → page; `g/G` → top/bottom; `p` → tax-profile form;
///   `c` → classify-inbound flow; `o` → reclassify-outflow flow.
pub fn handle_key(app: &mut EditorApp, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // ── 1. Mutation-modal dispatch — BEFORE everything else ───────────────────
    if app.mutation_modal.is_some() {
        handle_modal_key(app, key);
        return;
    }

    // ── 2. Classify-inbound-modal dispatch — BEFORE flow, form, screen ────────
    if app.classify_inbound_modal.is_some() {
        handle_classify_inbound_modal_key(app, key);
        return;
    }

    // ── 3. Reclassify-outflow-modal dispatch — BEFORE flow, form, screen ──────
    if app.reclassify_outflow_modal.is_some() {
        handle_reclassify_outflow_modal_key(app, key);
        return;
    }

    // ── 4. Flow dispatch — the FLOW Option (not the step) is the guard [R0-I2] ─
    //    Every step of an open flow is claimed here; 'q' and Esc can never
    //    fall through to a Browse quit arm mid-flow.
    if app.classify_inbound_flow.is_some() {
        handle_classify_inbound_flow_key(app, key);
        return;
    }
    if app.reclassify_outflow_flow.is_some() {
        handle_reclassify_outflow_flow_key(app, key);
        return;
    }

    // ── 5. Form dispatch — BEFORE screen dispatch ─────────────────────────────
    if app.profile_form.is_some() {
        handle_form_key(app, key);
        return;
    }

    // ── 6. Screen dispatch ────────────────────────────────────────────────────
    match app.screen {
        EditorScreen::Unlock => match key.code {
            // Only Esc quits from Unlock — 'q' and all printable chars go to buffer.
            KeyCode::Esc => app.should_quit = true,
            // Tab / BackTab ignored: no tab bar on Unlock screen.
            KeyCode::Tab | KeyCode::BackTab => {}
            KeyCode::Enter => app.do_unlock(),
            KeyCode::Backspace => app.unlock.pop_char(),
            KeyCode::Char(c) => {
                app.unlock.error = None;
                app.unlock.push_char(c);
            }
            _ => {}
        },
        EditorScreen::Locked => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('r') => {
                app.screen = EditorScreen::Unlock;
                app.unlock.error = None;
            }
            _ => {}
        },
        EditorScreen::Browse => {
            // Clear status on any key press ([N5]: modal/form keys never reach here,
            // so the status set by modal Enter/Esc is not instantly cleared).
            app.status = None;
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                KeyCode::Tab => app.tab = app.tab.next(),
                KeyCode::BackTab => app.tab = app.tab.prev(),
                KeyCode::Up | KeyCode::Char('k') => scroll_up(app),
                KeyCode::Down | KeyCode::Char('j') => scroll_down(app),
                KeyCode::PageUp => page_up(app),
                KeyCode::PageDown => page_down(app),
                KeyCode::Char('g') => go_top(app),
                KeyCode::Char('G') => go_bottom(app),
                KeyCode::Left => {
                    app.selected_year -= 1;
                    reset_selections(app);
                }
                KeyCode::Right => {
                    app.selected_year += 1;
                    reset_selections(app);
                }
                KeyCode::Char('p') => open_profile_form(app),
                KeyCode::Char('c') => open_classify_inbound_flow(app),
                KeyCode::Char('o') => open_reclassify_outflow_flow(app),
                _ => {}
            }
        }
    }
}

/// Handle a key press while the mutation-confirmation modal is open.
///
/// Dispatch order: modal → form → screen. All keys NOT matched here are swallowed
/// (the modal is blocking — `q` must NOT quit while the modal is open).
fn handle_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Extract payload before dropping the modal borrow.
            let (year, profile) = match app.mutation_modal.as_ref() {
                Some(m) => (m.year, m.profile.clone()),
                None => return,
            };

            // Persist: borrows session mutably. Block scope ends the borrow before
            // we access other fields.
            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.mutation_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_tax_profile(session, year, &profile)
            };

            match save_result {
                Ok(()) => {
                    // Re-project: borrows session immutably; block scope ends before
                    // we mutate app.snapshot.
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            app.snapshot = Some(snap);
                            app.status = Some(format!("Saved tax profile for {year}"));
                        }
                        Err(e) => {
                            // Save succeeded but re-projection failed (near-impossible).
                            // Keep old snapshot; inform user to restart.
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.mutation_modal = None;
                    app.profile_form = None;
                }
                Err(e) => {
                    // [R0-M1] Failed-save semantics: close modal, keep form (buffers intact),
                    // set error status. Do NOT re-project (vault unchanged on disk).
                    app.mutation_modal = None;
                    app.status = Some(format!("Save error: {e}"));
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal only — back to form; nothing written.
            app.mutation_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal — 'q' must NOT quit here).
        }
    }
}

/// Handle a key press while the profile form is open.
///
/// Tab cycles `FilingStatus` when focus==0 (the filing-status row);
/// on other rows, Tab moves focus down. Tab NEVER inserts text.
fn handle_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Validate, then open modal on success.
            // Extract before dropping the borrow so we can set mutation_modal.
            let result = app
                .profile_form
                .as_ref()
                .map(|f| validate(f).map(|p| (f.year, p)));
            match result {
                Some(Ok((year, profile))) => {
                    app.mutation_modal = Some(MutationModalState { year, profile });
                }
                Some(Err(msg)) => {
                    if let Some(f) = app.profile_form.as_mut() {
                        f.error = Some(msg);
                    }
                }
                None => {}
            }
        }
        KeyCode::Esc => {
            // Close form; nothing written.
            app.profile_form = None;
        }
        KeyCode::Tab => {
            if let Some(form) = app.profile_form.as_mut() {
                if form.focus == 0 {
                    // Cycle filing status
                    form.filing_status = cycle_filing_status(form.filing_status);
                } else {
                    // Move focus down (Tab never inserts text)
                    form.focus = (form.focus + 1).min(9);
                }
            }
        }
        KeyCode::BackTab => {
            if let Some(form) = app.profile_form.as_mut() {
                form.focus = form.focus.saturating_sub(1);
            }
        }
        KeyCode::Up => {
            if let Some(form) = app.profile_form.as_mut() {
                form.focus = form.focus.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let Some(form) = app.profile_form.as_mut() {
                form.focus = (form.focus + 1).min(9);
            }
        }
        KeyCode::Backspace => {
            if let Some(form) = app.profile_form.as_mut() {
                if form.focus > 0 {
                    form.fields[form.focus - 1].pop_char();
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(form) = app.profile_form.as_mut() {
                if form.focus > 0 {
                    form.fields[form.focus - 1].push_char(c);
                }
            }
        }
        _ => {}
    }
}

/// Open the tax-profile form for `selected_year`, pre-populated from the snapshot.
///
/// Pre-population (the `--show` equivalent): if `snapshot.profiles.get(&year)` is
/// `Some(p)`, every buffer is filled with the field's `Display` string and
/// `filing_status` is set from `p`.  Otherwise: `filing_status = Single`, all
/// buffers empty (required fields must be typed; optional empties → $0 at validation).
fn open_profile_form(app: &mut EditorApp) {
    if app.snapshot.is_none() {
        return;
    }
    let year = app.selected_year;
    let mut form = ProfileFormState::new(year);

    if let Some(snap) = app.snapshot.as_ref() {
        if let Some(profile) = snap.profiles.get(&year) {
            form.filing_status = profile.filing_status;
            form.fields[0].set(&profile.ordinary_taxable_income.to_string());
            form.fields[1].set(&profile.magi_excluding_crypto.to_string());
            form.fields[2].set(
                &profile
                    .qualified_dividends_and_other_pref_income
                    .to_string(),
            );
            form.fields[3].set(&profile.other_net_capital_gain.to_string());
            form.fields[4].set(&profile.capital_loss_carryforward_in.short.to_string());
            form.fields[5].set(&profile.capital_loss_carryforward_in.long.to_string());
            form.fields[6].set(&profile.w2_ss_wages.to_string());
            form.fields[7].set(&profile.w2_medicare_wages.to_string());
            form.fields[8].set(&profile.schedule_c_expenses.to_string());
        }
    }

    app.profile_form = Some(form);
}

// ── Classify-inbound modal handler ────────────────────────────────────────────

/// Handle a key press while the classify-inbound confirmation modal is open.
///
/// Same blocking pattern as `handle_modal_key`: all unmatched keys are swallowed;
/// `q` does NOT quit; `Esc` closes modal only (back to field form).
///
/// Enter-arm semantics (identical to chunk-1 D4 / R0-M1 pattern):
/// - `Ok(id)` → re-project, D4-step-2 blocker-derived status, close modal + flow.
/// - `Err(e)` → close modal, keep form open (buffers intact), "Save error: {e}".
fn handle_classify_inbound_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Extract the payload from the modal (before dropping the borrow).
            let modal = match app.classify_inbound_modal.as_ref() {
                Some(m) => {
                    let payload = EventPayload::ClassifyInbound(ClassifyInbound {
                        transfer_in_event: m.target_event.clone(),
                        as_: m.as_.clone(),
                    });
                    (payload, m.target_event.clone(), m.as_.clone())
                }
                None => return,
            };
            let (payload, target_event, as_) = modal;

            // Capture now at Enter-press (not inside persist fn) for determinism.
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.classify_inbound_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_classify_inbound(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    // Re-project: borrows session immutably in its own block.
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            // D4 step-2: derive status from re-projected blockers [R0-I5].
                            let status = derive_classify_inbound_status(
                                &snap,
                                &target_event,
                                &decision_id,
                                &as_,
                            );
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.classify_inbound_modal = None;
                    app.classify_inbound_flow = None;
                }
                Err(e) => {
                    // Failed-save semantics [R0-M1]: close modal, keep form (buffers intact).
                    app.classify_inbound_modal = None;
                    app.status = Some(format!("Save error: {e}"));
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal only → back to the field form.
            app.classify_inbound_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal — 'q' must NOT quit here).
        }
    }
}

// ── Classify-inbound flow key dispatch ────────────────────────────────────────

/// Dispatch the key to the appropriate step handler.
///
/// The FLOW OPTION (not the step) is the guard in `handle_key` [R0-I2]: when we
/// reach this function the flow is always `Some`.  The step discriminant is
/// read here to fan out to step-specific handlers.
fn handle_classify_inbound_flow_key(app: &mut EditorApp, key: KeyEvent) {
    // Determine the step discriminant via a non-destructuring borrow.
    let step_kind: u8 = match app.classify_inbound_flow.as_ref().map(|f| &f.step) {
        Some(ClassifyInboundStep::List) => 0,
        Some(ClassifyInboundStep::VariantPicker { .. }) => 1,
        Some(ClassifyInboundStep::IncomeForm { .. }) => 2,
        Some(ClassifyInboundStep::GiftForm { .. }) => 3,
        None => return,
    };
    match step_kind {
        0 => handle_ci_list_key(app, key),
        1 => handle_ci_picker_key(app, key),
        2 => handle_ci_income_form_key(app, key),
        3 => handle_ci_gift_form_key(app, key),
        _ => {}
    }
}

/// List step: scroll and select.
fn handle_ci_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            // Transition to variant picker with the selected item.
            let selected = app
                .classify_inbound_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                if let Some(flow) = app.classify_inbound_flow.as_mut() {
                    flow.step = ClassifyInboundStep::VariantPicker {
                        item,
                        variant: InboundVariant::Income, // initial selection
                    };
                }
            }
            // If nothing selected (defensive; list is non-empty by contract), swallow.
        }
        KeyCode::Esc => {
            // Close the flow — back to Browse; nothing written.
            app.classify_inbound_flow = None;
        }
        _ => {
            // All other keys (including 'q') swallowed while flow is open [R0-I2].
        }
    }
}

/// Variant-picker step: Income ↔ GiftReceived via Tab.
fn handle_ci_picker_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::VariantPicker { variant, .. } = &mut flow.step {
                    *variant = match *variant {
                        InboundVariant::Income => InboundVariant::GiftReceived,
                        InboundVariant::GiftReceived => InboundVariant::Income,
                    };
                }
            }
        }
        KeyCode::Enter => {
            // Move to the per-variant field form.  Use mem::replace to take ownership.
            let Some(flow) = app.classify_inbound_flow.as_mut() else {
                return;
            };
            let old_step = std::mem::replace(&mut flow.step, ClassifyInboundStep::List);
            if let ClassifyInboundStep::VariantPicker { item, variant } = old_step {
                flow.step = match variant {
                    InboundVariant::Income => ClassifyInboundStep::IncomeForm {
                        item,
                        kind: IncomeKind::Mining, // initial [R0-M3]
                        fmv_buf: FieldBuffer::new(),
                        business: false,
                        focus: 0,
                        error: None,
                    },
                    InboundVariant::GiftReceived => ClassifyInboundStep::GiftForm {
                        item,
                        fmv_at_gift_buf: FieldBuffer::new(),
                        donor_basis_buf: FieldBuffer::new(),
                        donor_acquired_at_buf: FieldBuffer::new(),
                        focus: 0,
                        error: None,
                    },
                };
            }
            // If step wasn't VariantPicker (shouldn't happen), step is now List (placeholder).
        }
        KeyCode::Esc => {
            // Back to the list step.
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.step = ClassifyInboundStep::List;
            }
        }
        _ => {
            // All other keys (including 'q') swallowed [R0-I2].
        }
    }
}

/// Income-form step: kind picker (Tab), fmv text, business toggle (Space), submit.
fn handle_ci_income_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Validate → open modal on success; set error on failure.
            let result = {
                match app.classify_inbound_flow.as_ref() {
                    Some(f) => match &f.step {
                        ClassifyInboundStep::IncomeForm {
                            item,
                            kind,
                            fmv_buf,
                            business,
                            ..
                        } => validate_classify_inbound_income(*kind, fmv_buf, *business)
                            .map(|cls| (item.clone(), cls)),
                        _ => return,
                    },
                    None => return,
                }
            };
            match result {
                Ok((item, cls)) => {
                    app.classify_inbound_modal = Some(ClassifyInboundModalState {
                        target_event: item.blocker_event.clone(),
                        target_date: item.date,
                        target_sat: item.sat,
                        as_: cls,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.classify_inbound_flow.as_mut() {
                        if let ClassifyInboundStep::IncomeForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            // Back to variant picker (retaining the item).
            let item = match app.classify_inbound_flow.as_ref() {
                Some(f) => match &f.step {
                    ClassifyInboundStep::IncomeForm { item, .. } => item.clone(),
                    _ => return,
                },
                None => return,
            };
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.step = ClassifyInboundStep::VariantPicker {
                    item,
                    variant: InboundVariant::Income,
                };
            }
        }
        KeyCode::Tab => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm { kind, focus, .. } = &mut flow.step {
                    if *focus == 0 {
                        // Tab on kind row cycles the IncomeKind variant.
                        *kind = cycle_income_kind(*kind);
                    } else {
                        // Tab on other rows moves focus down.
                        *focus = (*focus + 1).min(2);
                    }
                }
            }
        }
        KeyCode::BackTab => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Up => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Down => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(2);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm { fmv_buf, focus, .. } = &mut flow.step {
                    if *focus == 1 {
                        fmv_buf.pop_char();
                    }
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm {
                    business, focus, ..
                } = &mut flow.step
                {
                    if *focus == 2 {
                        *business = !*business;
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::IncomeForm { fmv_buf, focus, .. } = &mut flow.step {
                    if *focus == 1 {
                        fmv_buf.push_char(c);
                    }
                    // focus==0 (kind): Tab cycles kind (handled above); 'q' inserts into
                    // fmv_buf (text focus) or is swallowed here — does NOT quit [R2-N1].
                    // focus==2 (business): Space toggles; other chars swallowed.
                }
            }
        }
        _ => {
            // All unmatched keys (including 'q' at non-text focus) swallowed [R0-I2].
        }
    }
}

/// Gift-form step: three optional fields (fmv_at_gift required), submit.
fn handle_ci_gift_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let result = {
                match app.classify_inbound_flow.as_ref() {
                    Some(f) => match &f.step {
                        ClassifyInboundStep::GiftForm {
                            item,
                            fmv_at_gift_buf,
                            donor_basis_buf,
                            donor_acquired_at_buf,
                            ..
                        } => validate_classify_inbound_gift(
                            fmv_at_gift_buf,
                            donor_basis_buf,
                            donor_acquired_at_buf,
                        )
                        .map(|cls| (item.clone(), cls)),
                        _ => return,
                    },
                    None => return,
                }
            };
            match result {
                Ok((item, cls)) => {
                    app.classify_inbound_modal = Some(ClassifyInboundModalState {
                        target_event: item.blocker_event.clone(),
                        target_date: item.date,
                        target_sat: item.sat,
                        as_: cls,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.classify_inbound_flow.as_mut() {
                        if let ClassifyInboundStep::GiftForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            // Back to variant picker (retaining the item).
            let item = match app.classify_inbound_flow.as_ref() {
                Some(f) => match &f.step {
                    ClassifyInboundStep::GiftForm { item, .. } => item.clone(),
                    _ => return,
                },
                None => return,
            };
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.step = ClassifyInboundStep::VariantPicker {
                    item,
                    variant: InboundVariant::GiftReceived,
                };
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::GiftForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(2);
                }
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::GiftForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::GiftForm {
                    fmv_at_gift_buf,
                    donor_basis_buf,
                    donor_acquired_at_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => fmv_at_gift_buf.pop_char(),
                        1 => donor_basis_buf.pop_char(),
                        2 => donor_acquired_at_buf.pop_char(),
                        _ => {}
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::GiftForm {
                    fmv_at_gift_buf,
                    donor_basis_buf,
                    donor_acquired_at_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => fmv_at_gift_buf.push_char(c),
                        1 => donor_basis_buf.push_char(c),
                        2 => donor_acquired_at_buf.push_char(c),
                        _ => {}
                    }
                    // 'q' at any text focus inserts into the buffer — does NOT quit [R2-N1].
                }
            }
        }
        _ => {
            // All unmatched keys swallowed [R0-I2].
        }
    }
}

// ── Reclassify-outflow modal handler ─────────────────────────────────────────

/// Handle a key press while the reclassify-outflow confirmation modal is open.
///
/// Same blocking pattern as `handle_classify_inbound_modal_key`.
/// Enter-arm semantics:
/// - `Ok(id)` → re-project, D4-step-2 blocker-derived status, close modal + flow.
/// - `Err(e)` → close modal, keep field form open (buffers intact), "Save error: {e}".
fn handle_reclassify_outflow_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Extract the payload from the modal before dropping the borrow.
            let modal_data = match app.reclassify_outflow_modal.as_ref() {
                Some(m) => {
                    let payload = EventPayload::ReclassifyOutflow(m.payload.clone());
                    let target_event = m.target_event.clone();
                    let kind_str = outflow_kind_str(&m.payload.as_);
                    (payload, target_event, kind_str)
                }
                None => return,
            };
            let (payload, target_event, kind_str) = modal_data;

            // Capture now at Enter-press (not inside persist fn) for determinism.
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.reclassify_outflow_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_reclassify_outflow(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    // Re-project: borrows session immutably in its own block.
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            // D4 step-2: derive status from re-projected blockers [R0-I5].
                            let status = derive_reclassify_outflow_status(
                                &snap,
                                &target_event,
                                &decision_id,
                                &kind_str,
                            );
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.reclassify_outflow_modal = None;
                    app.reclassify_outflow_flow = None;
                }
                Err(e) => {
                    // Failed-save semantics [R0-M1]: close modal, keep form (buffers intact).
                    app.reclassify_outflow_modal = None;
                    app.status = Some(format!("Save error: {e}"));
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal only → back to the field form.
            app.reclassify_outflow_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal — 'q' must NOT quit here).
        }
    }
}

/// Return a display string for the outflow class (for status messages).
fn outflow_kind_str(as_: &OutflowClass) -> String {
    match as_ {
        OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        } => "sell".to_string(),
        OutflowClass::Dispose {
            kind: DisposeKind::Spend,
        } => "spend".to_string(),
        OutflowClass::GiftOut => "gift".to_string(),
        OutflowClass::Donate { .. } => "donate".to_string(),
    }
}

// ── Reclassify-outflow flow key dispatch ──────────────────────────────────────

/// Dispatch the key to the appropriate step handler.
///
/// The FLOW OPTION (not the step) is the guard in `handle_key` [R0-I2].
fn handle_reclassify_outflow_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step_kind: u8 = match app.reclassify_outflow_flow.as_ref().map(|f| &f.step) {
        Some(ReclassifyOutflowStep::List) => 0,
        Some(ReclassifyOutflowStep::KindPicker { .. }) => 1,
        Some(ReclassifyOutflowStep::FieldForm { .. }) => 2,
        None => return,
    };
    match step_kind {
        0 => handle_ro_list_key(app, key),
        1 => handle_ro_kind_picker_key(app, key),
        2 => handle_ro_field_form_key(app, key),
        _ => {}
    }
}

/// List step: scroll and select.
fn handle_ro_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            // Transition to kind picker with the selected item.
            let selected = app
                .reclassify_outflow_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                    flow.step = ReclassifyOutflowStep::KindPicker {
                        item,
                        kind: OutflowKind::Sell, // initial selection
                    };
                }
            }
            // If nothing selected (defensive; list is non-empty by contract), swallow.
        }
        KeyCode::Esc => {
            // Close the flow — back to Browse; nothing written.
            app.reclassify_outflow_flow = None;
        }
        _ => {
            // All other keys (including 'q') swallowed while flow is open [R0-I2].
        }
    }
}

/// Kind-picker step: sell / spend / gift / donate via Tab.
fn handle_ro_kind_picker_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                if let ReclassifyOutflowStep::KindPicker { kind, .. } = &mut flow.step {
                    *kind = cycle_outflow_kind(*kind);
                }
            }
        }
        KeyCode::Enter => {
            // Move to the field form. Use mem::replace to take ownership.
            let Some(flow) = app.reclassify_outflow_flow.as_mut() else {
                return;
            };
            let old_step = std::mem::replace(&mut flow.step, ReclassifyOutflowStep::List);
            if let ReclassifyOutflowStep::KindPicker { item, kind } = old_step {
                flow.step = ReclassifyOutflowStep::FieldForm {
                    item,
                    kind,
                    amount_buf: FieldBuffer::new(),
                    fee_buf: FieldBuffer::new(),
                    appraisal: false,
                    donee_buf: FieldBuffer::new(),
                    focus: 0,
                    error: None,
                };
            }
        }
        KeyCode::Esc => {
            // Back to the list step.
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                flow.step = ReclassifyOutflowStep::List;
            }
        }
        _ => {
            // All other keys (including 'q') swallowed [R0-I2].
        }
    }
}

/// Field-form step: amount, fee, appraisal (donate), donee (gift/donate), submit.
fn handle_ro_field_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Validate → open modal on success; set error on failure.
            let result = {
                match app.reclassify_outflow_flow.as_ref() {
                    Some(f) => match &f.step {
                        ReclassifyOutflowStep::FieldForm {
                            item,
                            kind,
                            amount_buf,
                            fee_buf,
                            appraisal,
                            donee_buf,
                            ..
                        } => validate_reclassify_outflow(
                            item, *kind, amount_buf, fee_buf, *appraisal, donee_buf,
                        )
                        .map(|payload| (item.clone(), payload)),
                        _ => return,
                    },
                    None => return,
                }
            };
            match result {
                Ok((item, payload)) => {
                    app.reclassify_outflow_modal = Some(ReclassifyOutflowModalState {
                        target_event: item.transfer_out_event.clone(),
                        target_date: item.date,
                        principal_sat: item.principal_sat,
                        payload,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                        if let ReclassifyOutflowStep::FieldForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            // Back to kind picker (retaining the item + kind).
            let (item, kind) = match app.reclassify_outflow_flow.as_ref() {
                Some(f) => match &f.step {
                    ReclassifyOutflowStep::FieldForm { item, kind, .. } => (item.clone(), *kind),
                    _ => return,
                },
                None => return,
            };
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                flow.step = ReclassifyOutflowStep::KindPicker { item, kind };
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                if let ReclassifyOutflowStep::FieldForm { focus, kind, .. } = &mut flow.step {
                    *focus = next_focus(*focus, *kind);
                }
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                if let ReclassifyOutflowStep::FieldForm { focus, kind, .. } = &mut flow.step {
                    *focus = prev_focus(*focus, *kind);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                if let ReclassifyOutflowStep::FieldForm {
                    amount_buf,
                    fee_buf,
                    donee_buf,
                    focus,
                    kind,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => amount_buf.pop_char(),
                        1 => fee_buf.pop_char(),
                        // row 2 (appraisal) is a toggle; Backspace swallowed
                        3 if matches!(*kind, OutflowKind::Gift | OutflowKind::Donate) => {
                            donee_buf.pop_char()
                        }
                        _ => {}
                    }
                }
            }
        }
        KeyCode::Char(' ') => {
            // Toggle appraisal when focus == 2 and kind == Donate.
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                if let ReclassifyOutflowStep::FieldForm {
                    appraisal,
                    focus,
                    kind,
                    ..
                } = &mut flow.step
                {
                    if *focus == 2 && *kind == OutflowKind::Donate {
                        *appraisal = !*appraisal;
                    }
                    // Space at text rows inserts into the relevant buffer (donee, or
                    // swallowed at amount/fee — spaces are allowed by FIELD_CAP discipline).
                    // For amount/fee buffers, space would produce a non-parseable decimal;
                    // the user gets a parse error at submit — consistent with [R0-N1] approach.
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                if let ReclassifyOutflowStep::FieldForm {
                    amount_buf,
                    fee_buf,
                    donee_buf,
                    focus,
                    kind,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => amount_buf.push_char(c),
                        1 => fee_buf.push_char(c),
                        // row 2 (appraisal): Space toggles (handled above); other chars swallowed.
                        3 if matches!(*kind, OutflowKind::Gift | OutflowKind::Donate) => {
                            donee_buf.push_char(c)
                        }
                        _ => {
                            // All other chars (including 'q' at non-text focus) swallowed [R0-I2].
                        }
                    }
                }
            }
        }
        _ => {
            // All unmatched keys (including 'q' at non-text focus) swallowed [R0-I2].
        }
    }
}

// ── Classify-inbound flow opener ──────────────────────────────────────────────

/// Build an `events_by_id` lookup table from the snapshot's raw event list.
fn events_by_id(
    snap: &btctax_tui::app::Snapshot,
) -> std::collections::BTreeMap<&EventId, &btctax_core::LedgerEvent> {
    snap.events.iter().map(|e| (&e.id, e)).collect()
}

/// Open the classify-inbound flow from the Browse screen.
///
/// Applies the compound pre-filter (spec §Pre-filter verification, Claim A):
/// 1. `UnknownBasisInbound` blockers only.
/// 2. Blocker.event resolves to a raw `TransferIn` event in snap.events [R0-M2: raw only].
/// 3. No non-voided `ClassifyInbound` decision in snap.events already targets it.
///
/// Empty filtered list → status "No unclassified inbound transfers"; flow NOT opened [R0-M8].
fn open_classify_inbound_flow(app: &mut EditorApp) {
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

    // Pre-compute the set of voided event ids (for filter 3's voided-ClassifyInbound check).
    let voided: BTreeSet<EventId> = snap
        .events
        .iter()
        .filter_map(|e| {
            if let EventPayload::VoidDecisionEvent(v) = &e.payload {
                Some(v.target_event_id.clone())
            } else {
                None
            }
        })
        .collect();

    // Apply filter 3: build the set of TransferIn EventIds already targeted by a non-voided
    // ClassifyInbound decision (adding a second would fire DecisionConflict; FIRST-WINS).
    let already_classified: BTreeSet<EventId> = snap
        .events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| {
            if let EventPayload::ClassifyInbound(ci) = &e.payload {
                Some(ci.transfer_in_event.clone())
            } else {
                None
            }
        })
        .collect();

    // Apply filters 1 + 2 + 3 in one pass.
    let mut items: Vec<InboundListItem> = snap
        .state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UnknownBasisInbound)
        .filter_map(|b| {
            let ti_id = b.event.as_ref()?;
            // Filter 2: raw TransferIn payload check [R0-M2: raw only, effective-payload
            // limitation documented in spec + FOLLOWUPS].
            let ev = ev_idx.get(ti_id)?;
            if !matches!(ev.payload, EventPayload::TransferIn(_)) {
                return None;
            }
            // Filter 3: no non-voided ClassifyInbound already targets this TransferIn.
            if already_classified.contains(ti_id) {
                return None;
            }
            // Build the display item.
            let sat = match &ev.payload {
                EventPayload::TransferIn(ti) => ti.sat,
                _ => unreachable!(),
            };
            let date = btctax_core::conventions::tax_date(ev.utc_timestamp, ev.original_tz);
            Some(InboundListItem {
                blocker_event: ti_id.clone(),
                date,
                sat,
                wallet: ev.wallet.clone(),
                detail: b.detail.clone(),
            })
        })
        .collect();

    // Sort by date for deterministic display order.
    items.sort_by_key(|i| i.date);

    if items.is_empty() {
        // R0-M8: empty filtered list never opens a flow.
        app.status = Some("No unclassified inbound transfers".to_string());
        return;
    }

    app.classify_inbound_flow = Some(ClassifyInboundFlowState {
        list: TargetList::new(items),
        step: ClassifyInboundStep::List,
    });
}

// ── Reclassify-outflow flow opener ────────────────────────────────────────────

/// Open the reclassify-outflow flow from the Browse screen.
///
/// Sources `snap.state.pending_reconciliation` (Claim B: inherently post-filtered —
/// only unreclassified, unlinked TransferOuts). No additional client-side filter required.
///
/// Empty list → status "No pending outbound transfers"; flow NOT opened [R0-M8].
fn open_reclassify_outflow_flow(app: &mut EditorApp) {
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

    let mut items: Vec<OutflowListItem> = snap
        .state
        .pending_reconciliation
        .iter()
        .map(|pt| {
            let ev = ev_idx.get(&pt.event);
            let date = ev
                .map(|e| btctax_core::conventions::tax_date(e.utc_timestamp, e.original_tz))
                .unwrap_or_else(|| {
                    // Defensive: event not found in snap (shouldn't happen).
                    btctax_core::conventions::tax_date(
                        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
                        time::UtcOffset::UTC,
                    )
                });
            let wallet = ev.and_then(|e| e.wallet.clone());
            OutflowListItem {
                transfer_out_event: pt.event.clone(),
                date,
                principal_sat: pt.principal_sat,
                wallet,
            }
        })
        .collect();

    // Sort by date for deterministic display order.
    items.sort_by_key(|i| i.date);

    if items.is_empty() {
        // R0-M8: empty list never opens a flow.
        app.status = Some("No pending outbound transfers".to_string());
        return;
    }

    app.reclassify_outflow_flow = Some(ReclassifyOutflowFlowState {
        list: TargetList::new(items),
        step: ReclassifyOutflowStep::List,
    });
}

// ── Post-persist status derivation ────────────────────────────────────────────

/// Derive the status string from the RE-PROJECTED blockers after a classify-inbound save.
///
/// The status is NEVER keyed on the payload shape — it is derived from the new
/// `snap.state.blockers` [R0-I5].  The `decision_id` is the returned `EventId` of the
/// just-appended decision (used only for the `DecisionConflict` check; the TransferIn
/// `target_event` is the event attributed to `FmvMissing` and `UnknownBasisInbound`).
fn derive_classify_inbound_status(
    snap: &btctax_tui::app::Snapshot,
    target_event: &EventId,
    decision_id: &EventId,
    as_: &InboundClass,
) -> String {
    // Decision-attributed DecisionConflict check (failed-save-retry duplicate [R0-I1]).
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired on this decision — see Compliance; \
                 clear with CLI: btctax reconcile void decision|{}",
                decision_id.canonical()
            );
        }
    }

    // FmvMissing attributed to the target TransferIn event.
    // [R0-I4]: no set-fmv suggestion — void + re-classify is the only remedy.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(target_event) {
            // Find the decision seq from the decision_id for the void CLI command.
            let seq = match decision_id {
                EventId::Decision { seq } => *seq,
                _ => 0,
            };
            return format!(
                "Classified as Income({kind}) but FMV missing — FmvMissing blocker fired; \
                 to supply the FMV, void this decision (CLI: btctax reconcile void \
                 decision|{seq}) and re-classify with an FMV",
                kind = match as_ {
                    InboundClass::Income { kind, .. } => income_kind_display(*kind),
                    _ => "?",
                }
            );
        }
    }

    // UnknownBasisInbound re-fired for the target TransferIn (gift case 3 or 4) [R0-I5].
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(target_event) {
            let seq = match decision_id {
                EventId::Decision { seq } => *seq,
                _ => 0,
            };
            return format!(
                "Gift recorded but basis unknown — UnknownBasisInbound re-fired; \
                 void this decision (CLI: btctax reconcile void decision|{seq}) \
                 and re-classify with donor basis or a donor date covered by the price dataset"
            );
        }
    }

    // No target-attributed blocker: clean success.
    let cls_desc = match as_ {
        InboundClass::Income { kind, .. } => {
            format!("Income({})", income_kind_display(*kind))
        }
        InboundClass::GiftReceived { .. } => "GiftReceived".to_string(),
    };
    format!("Classified inbound as {cls_desc}")
}

/// Derive the status string from the RE-PROJECTED blockers after a reclassify-outflow save.
///
/// The status is NEVER keyed on the payload shape — it is derived from the new
/// `snap.state.blockers` [R0-I5]. `decision_id` is the returned `EventId` of the
/// just-appended decision (for the `DecisionConflict` check); `target_event` is the
/// `transfer_out_event` (for the `UncoveredDisposal` check).
fn derive_reclassify_outflow_status(
    snap: &btctax_tui::app::Snapshot,
    target_event: &EventId,
    decision_id: &EventId,
    kind_str: &str,
) -> String {
    // Decision-attributed DecisionConflict check (failed-save-retry duplicate [R0-I1]).
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired on this decision — see Compliance; \
                 clear with CLI: btctax reconcile void decision|{}",
                decision_id.canonical()
            );
        }
    }

    // UncoveredDisposal attributed to the target TransferOut event [R0-M6].
    // May also fire for the PendingOut arm before reclassification (pre-existing shortfall);
    // after reclassify it fires from the Dispose/GiftOut/Donate consume paths.
    // The re-projected state will show UncoveredDisposal if the lot pool is short.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::UncoveredDisposal && b.event.as_ref() == Some(target_event) {
            return format!(
                "Reclassified outflow as {kind_str} — WARNING: UncoveredDisposal blocker fired; \
                 check Holdings"
            );
        }
    }

    // No target-attributed blocker: clean success.
    format!("Reclassified outflow as {kind_str}")
}

// ── Scroll helpers ────────────────────────────────────────────────────────────

/// Return the active `TableState` for the currently focused tab (if the tab has one).
fn active_state(app: &mut EditorApp) -> Option<&mut TableState> {
    match app.tab {
        Tab::Holdings => Some(&mut app.holdings_state),
        Tab::Disposals => Some(&mut app.disposals_state),
        Tab::Income => Some(&mut app.income_state),
        Tab::Forms => Some(&mut app.forms_state),
        _ => None,
    }
}

/// Number of selectable data rows for the active tab (TOTAL row excluded, same as viewer).
fn active_row_count(app: &EditorApp) -> usize {
    let Some(snap) = app.snapshot.as_ref() else {
        return 0;
    };
    match app.tab {
        Tab::Holdings => snap.state.lots.len(),
        Tab::Disposals => {
            let yr = app.selected_year;
            snap.state
                .disposals
                .iter()
                .filter(|d| d.disposed_at.year() == yr)
                .map(|d| d.legs.len())
                .sum::<usize>()
        }
        Tab::Income => {
            let yr = app.selected_year;
            snap.state
                .income_recognized
                .iter()
                .filter(|r| r.recognized_at.year() == yr)
                .count()
        }
        Tab::Forms => {
            let yr = app.selected_year;
            btctax_core::form_8949(&snap.state, yr).len()
        }
        _ => 0,
    }
}

fn scroll_up(app: &mut EditorApp) {
    let Some(state) = active_state(app) else {
        return;
    };
    let next = match state.selected() {
        Some(i) if i > 0 => Some(i - 1),
        Some(_) => Some(0),
        None => None,
    };
    state.select(next);
}

fn scroll_down(app: &mut EditorApp) {
    let count = active_row_count(app);
    if count == 0 {
        return;
    }
    let Some(state) = active_state(app) else {
        return;
    };
    let next = match state.selected() {
        Some(i) => Some((i + 1).min(count - 1)),
        None => Some(0),
    };
    state.select(next);
}

fn page_up(app: &mut EditorApp) {
    const PAGE: usize = 10;
    let Some(state) = active_state(app) else {
        return;
    };
    let next = state.selected().map(|i| i.saturating_sub(PAGE));
    state.select(next);
}

fn page_down(app: &mut EditorApp) {
    const PAGE: usize = 10;
    let count = active_row_count(app);
    if count == 0 {
        return;
    }
    let Some(state) = active_state(app) else {
        return;
    };
    let next = match state.selected() {
        Some(i) => Some((i + PAGE).min(count - 1)),
        None => Some(PAGE.min(count - 1)),
    };
    state.select(next);
}

fn go_top(app: &mut EditorApp) {
    let count = active_row_count(app);
    if count == 0 {
        return;
    }
    if let Some(state) = active_state(app) {
        state.select(Some(0));
    }
}

fn go_bottom(app: &mut EditorApp) {
    let count = active_row_count(app);
    if count == 0 {
        return;
    }
    if let Some(state) = active_state(app) {
        state.select(Some(count - 1));
    }
}

fn reset_selections(app: &mut EditorApp) {
    app.holdings_state.select(None);
    app.disposals_state.select(None);
    app.income_state.select(None);
    app.forms_state.select(None);
}

// ── Run loop ──────────────────────────────────────────────────────────────────

/// The main event loop. Runs until `app.should_quit` is set.
fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    vault_path: PathBuf,
) -> io::Result<()> {
    let mut app = EditorApp::new(vault_path);

    // `BTCTAX_PASSPHRASE` fast-path: open immediately without displaying the unlock prompt.
    app.try_env_passphrase();

    while !app.should_quit {
        terminal.draw(|f| draw_edit::draw(f, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key);
            }
        }
    }
    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    // Install panic hook BEFORE enabling raw mode.
    setup_panic_hook();

    let vault_path = parse_vault_path();

    enable_raw_mode()?;
    // RAII guard: Drop calls restore_terminal() regardless of how this scope exits.
    let _guard = TerminalGuard::new();

    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, vault_path);

    // Explicit call is redundant (guard's Drop covers it) but kept for clarity;
    // restore_terminal() is idempotent.
    restore_terminal();

    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_store::Passphrase;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers};
    use editor::{EditorApp, EditorScreen};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    // ── Helper: type characters into the focused buffer ──────────────────────

    fn type_str(app: &mut EditorApp, s: &str) {
        for c in s.chars() {
            handle_key(app, press(KeyCode::Char(c)));
        }
    }

    // ── KAT-U1 — unlock parity ───────────────────────────────────────────────

    #[test]
    fn kat_u1_correct_passphrase_transitions_to_browse_with_session_and_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "u1-correct-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(
            app.screen,
            EditorScreen::Browse,
            "correct passphrase must transition to Browse"
        );
        assert!(app.session.is_some(), "session must be held after unlock");
        assert!(
            app.snapshot.is_some(),
            "snapshot must be populated after unlock"
        );
        assert!(
            app.unlock.buffer.is_empty(),
            "buffer must be cleared after unlock (mem::take)"
        );
        let snap = app.snapshot.as_ref().unwrap();
        let _ = &snap.events;
        let _ = &snap.state;
        let _ = &snap.profiles;
    }

    #[test]
    fn kat_u1_wrong_passphrase_stays_on_unlock_with_error_and_no_session() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");

        btctax_cli::cmd::init::run(&vault, &Passphrase::new("correct".into()), &key).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in "wrong-pass".chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(app.screen, EditorScreen::Unlock);
        assert_eq!(app.unlock.error.as_deref(), Some("incorrect passphrase"));
        assert!(app.session.is_none());
        assert!(app.snapshot.is_none());
        assert!(app.unlock.buffer.is_empty());
    }

    #[test]
    fn kat_u1_locked_vault_transitions_to_locked_screen_with_no_session() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "u1-lock-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let _holder = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(app.screen, EditorScreen::Locked);
        assert!(app.session.is_none());
    }

    // ── Lock-exclusivity KAT ─────────────────────────────────────────────────

    #[test]
    fn lock_exclusivity_editor_session_blocks_concurrent_open() {
        use btctax_tui::unlock::{open_session, SessionOpenOutcome};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "excl-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();
        assert_eq!(app.screen, EditorScreen::Browse);
        assert!(app.session.is_some());

        let outcome2 = open_session(&vault, Passphrase::new(pp_str.into()));
        assert!(matches!(outcome2, SessionOpenOutcome::Locked));
    }

    // ── EDITOR visual markers ─────────────────────────────────────────────────

    #[test]
    fn unlock_screen_carries_editor_marker_in_rendered_buffer() {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        assert_eq!(app.screen, EditorScreen::Unlock);

        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            rendered.contains("EDITOR"),
            "Unlock screen must contain 'EDITOR' marker; rendered:\n{rendered}"
        );
    }

    // ── Browse tabs smoke test ────────────────────────────────────────────────

    #[test]
    fn browse_tabs_smoke_all_six_tabs_render_without_panic() {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_tui::app::Snapshot;
        use ratatui::{backend::TestBackend, Terminal};
        use std::collections::BTreeMap;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let snap = Snapshot {
            events: vec![],
            state: btctax_core::state::LedgerState::default(),
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
        };

        let mut app = EditorApp::new(PathBuf::from("/smoke/vault.pgp"));
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snap);
        app.selected_year = 2025;

        for tab in Tab::ALL {
            app.tab = tab;
            terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        }

        let rendered: String = terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            rendered.contains("EDITOR"),
            "Browse screen must contain '[EDITOR]' marker; rendered:\n{rendered}"
        );
    }

    // ── handle_key: regression guards ────────────────────────────────────────

    #[test]
    fn q_on_browse_sets_should_quit() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        assert!(!app.should_quit);
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(app.should_quit, "'q' on Browse must quit");
    }

    #[test]
    fn esc_on_unlock_sets_should_quit() {
        let mut app = EditorApp::new(PathBuf::new());
        assert_eq!(app.screen, EditorScreen::Unlock);
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(app.should_quit, "Esc on Unlock must quit");
    }

    #[test]
    fn q_on_unlock_appends_to_buffer_not_quit() {
        let mut app = EditorApp::new(PathBuf::new());
        assert_eq!(app.screen, EditorScreen::Unlock);
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(!app.should_quit, "'q' on Unlock must NOT quit");
        assert_eq!(app.unlock.buffer.len(), 1);
    }

    #[test]
    fn tab_on_browse_cycles_forward() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        assert_eq!(app.tab, Tab::Holdings);
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Disposals);
    }

    #[test]
    fn backtab_on_browse_cycles_backward() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        assert_eq!(app.tab, Tab::Holdings);
        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Compliance);
    }

    #[test]
    fn r_on_locked_returns_to_unlock() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Locked;
        handle_key(&mut app, press(KeyCode::Char('r')));
        assert_eq!(app.screen, EditorScreen::Unlock);
    }

    #[test]
    fn tab_on_unlock_is_ignored() {
        let mut app = EditorApp::new(PathBuf::new());
        let initial_tab = app.tab;
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, initial_tab);
        assert!(app.unlock.buffer.is_empty());
    }

    #[test]
    fn key_release_is_ignored() {
        let mut app = EditorApp::new(PathBuf::new());
        let mut release_q = press(KeyCode::Char('q'));
        release_q.kind = KeyEventKind::Release;
        handle_key(&mut app, release_q);
        assert!(!app.should_quit);
    }

    #[test]
    fn left_right_on_browse_changes_selected_year() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        let initial = app.selected_year;
        handle_key(&mut app, press(KeyCode::Left));
        assert_eq!(app.selected_year, initial - 1);
        handle_key(&mut app, press(KeyCode::Right));
        assert_eq!(app.selected_year, initial);
    }

    // ── Modal: q is swallowed while modal is open ────────────────────────────

    #[test]
    fn q_while_modal_open_is_swallowed_not_quit() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        use btctax_core::{Carryforward, FilingStatus, TaxProfile};
        use rust_decimal_macros::dec;
        app.mutation_modal = Some(MutationModalState {
            year: 2025,
            profile: TaxProfile {
                filing_status: FilingStatus::Single,
                ordinary_taxable_income: dec!(100000),
                magi_excluding_crypto: dec!(100000),
                qualified_dividends_and_other_pref_income: dec!(0),
                other_net_capital_gain: dec!(0),
                capital_loss_carryforward_in: Carryforward::default(),
                w2_ss_wages: dec!(0),
                w2_medicare_wages: dec!(0),
                schedule_c_expenses: dec!(0),
            },
        });
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(
            !app.should_quit,
            "'q' while modal open must be swallowed, not trigger quit"
        );
        assert!(
            app.mutation_modal.is_some(),
            "modal must stay open after 'q'"
        );
    }

    // ── Modal: Esc closes modal only, leaves form open ───────────────────────

    #[test]
    fn esc_while_modal_open_closes_modal_only() {
        use btctax_core::{Carryforward, FilingStatus, TaxProfile};
        use rust_decimal_macros::dec;

        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        app.profile_form = Some(ProfileFormState::new(2025));
        app.mutation_modal = Some(MutationModalState {
            year: 2025,
            profile: TaxProfile {
                filing_status: FilingStatus::Single,
                ordinary_taxable_income: dec!(100000),
                magi_excluding_crypto: dec!(100000),
                qualified_dividends_and_other_pref_income: dec!(0),
                other_net_capital_gain: dec!(0),
                capital_loss_carryforward_in: Carryforward::default(),
                w2_ss_wages: dec!(0),
                w2_medicare_wages: dec!(0),
                schedule_c_expenses: dec!(0),
            },
        });

        handle_key(&mut app, press(KeyCode::Esc));

        assert!(
            app.mutation_modal.is_none(),
            "Esc on modal must close the modal"
        );
        assert!(
            app.profile_form.is_some(),
            "Esc on modal must NOT close the form — form must stay open"
        );
        assert!(
            !app.should_quit,
            "Esc on modal must NOT quit the application"
        );
        assert!(
            app.status.is_none(),
            "no status must be set on Esc (cancel path)"
        );
    }

    // ── Form: Esc closes form, nothing written ───────────────────────────────

    #[test]
    fn esc_while_form_open_closes_form_not_quit() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        app.profile_form = Some(ProfileFormState::new(2025));

        handle_key(&mut app, press(KeyCode::Esc));

        assert!(
            app.profile_form.is_none(),
            "Esc on form must close the form"
        );
        assert!(!app.should_quit, "Esc on form must NOT quit");
    }

    // ── Form: Enter with invalid data shows error ────────────────────────────

    #[test]
    fn enter_with_empty_form_sets_validation_error() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        app.profile_form = Some(ProfileFormState::new(2025));

        handle_key(&mut app, press(KeyCode::Enter));

        assert!(
            app.mutation_modal.is_none(),
            "invalid form must not open modal"
        );
        let form = app.profile_form.as_ref().unwrap();
        assert!(
            form.error.is_some(),
            "invalid form must set an error message"
        );
    }

    // ── KAT-F1: pre-population from existing profile ─────────────────────────

    #[test]
    fn kat_f1_p_opens_form_prepopulated_from_existing_profile() {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_core::{Carryforward, FilingStatus, TaxProfile};
        use btctax_tui::app::Snapshot;
        use rust_decimal_macros::dec;
        use std::collections::BTreeMap;

        let profile = TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: dec!(120000),
            magi_excluding_crypto: dec!(130000),
            qualified_dividends_and_other_pref_income: dec!(5000),
            other_net_capital_gain: dec!(1000),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(500),
                long: dec!(250),
            },
            w2_ss_wages: dec!(80000),
            w2_medicare_wages: dec!(85000),
            schedule_c_expenses: dec!(3000),
        };

        let mut profiles = BTreeMap::new();
        profiles.insert(2025, profile.clone());

        let snap = Snapshot {
            events: vec![],
            state: btctax_core::state::LedgerState::default(),
            cli_config: CliConfig::default(),
            profiles,
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
        };

        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snap);
        app.selected_year = 2025;

        // Press 'p' to open the form
        handle_key(&mut app, press(KeyCode::Char('p')));

        let form = app
            .profile_form
            .as_ref()
            .expect("form must be open after 'p'");
        assert_eq!(form.year, 2025);
        assert_eq!(
            form.filing_status,
            FilingStatus::Mfj,
            "filing_status must be pre-populated"
        );
        assert_eq!(
            form.fields[0].buf, "120000",
            "ordinary_taxable_income must be pre-populated"
        );
        assert_eq!(form.fields[1].buf, "130000", "magi must be pre-populated");
        assert_eq!(form.fields[2].buf, "5000", "qd must be pre-populated");
        assert_eq!(form.fields[3].buf, "1000", "oncg must be pre-populated");
        assert_eq!(form.fields[4].buf, "500", "cf_short must be pre-populated");
        assert_eq!(form.fields[5].buf, "250", "cf_long must be pre-populated");
        assert_eq!(form.fields[6].buf, "80000", "w2_ss must be pre-populated");
        assert_eq!(
            form.fields[7].buf, "85000",
            "w2_medicare must be pre-populated"
        );
        assert_eq!(
            form.fields[8].buf, "3000",
            "schedule_c must be pre-populated"
        );
    }

    // ── KAT-C1: cancel-path vault bytes unchanged ────────────────────────────

    #[test]
    fn kat_c1_cancel_path_vault_bytes_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c1-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            // Open editor session
            let mut app = EditorApp::new(vault.clone());
            for c in pp_str.chars() {
                app.unlock.push_char(c);
            }
            app.do_unlock();
            assert_eq!(app.screen, EditorScreen::Browse, "must open to Browse");

            // Press 'p' → form opens
            handle_key(&mut app, press(KeyCode::Char('p')));
            assert!(app.profile_form.is_some(), "form must open after 'p'");

            // Fill the 3 required fields
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "120000");
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "130000");
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "5000");

            // Enter → modal opens (valid form)
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.mutation_modal.is_some(),
                "modal must open after Enter on valid form"
            );

            // Assert: 'q' while modal is open is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "'q' in modal must be swallowed");
            assert!(
                app.mutation_modal.is_some(),
                "modal must stay open after 'q'"
            );

            // Esc → modal closes (back to form, nothing written)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.mutation_modal.is_none(), "Esc must close modal");
            assert!(
                app.profile_form.is_some(),
                "form must stay open after modal Esc"
            );
            assert!(app.status.is_none(), "no status must be set on cancel path");

            // Esc → form closes
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.profile_form.is_none(), "Esc must close form");

            // 'q' → quit
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(app.should_quit);
            // app drops here, releasing the session (VaultLock)
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "KAT-C1: vault must be byte-identical after cancel path"
        );
    }

    #[test]
    fn kat_c1_complement_confirmed_mutation_changes_vault_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c1-comp-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = EditorApp::new(vault.clone());
            for c in pp_str.chars() {
                app.unlock.push_char(c);
            }
            app.do_unlock();
            assert_eq!(app.screen, EditorScreen::Browse);

            // p → form
            handle_key(&mut app, press(KeyCode::Char('p')));
            // Fill required fields
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "120000");
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "130000");
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "5000");
            // Enter → modal
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(app.mutation_modal.is_some());
            // Enter → confirm + save
            handle_key(&mut app, press(KeyCode::Enter));
            // After confirm: modal closed, form closed, status set
            assert!(app.mutation_modal.is_none());
            assert!(app.profile_form.is_none());
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("Saved"))
                    .unwrap_or(false),
                "status must say Saved; got: {:?}",
                app.status
            );
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_ne!(
            bytes_before, bytes_after,
            "KAT-C1 complement: vault bytes must differ after confirmed mutation"
        );
    }

    // ── KAT-S1: save-error path (unix chmod) ────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn kat_s1_save_error_path_chmod_parent() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-s1-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Root-skip guard: try writing a file into the dir; skip if it succeeds after chmod
        {
            let test_file = dir.path().join("probe.tmp");
            let perms = std::fs::Permissions::from_mode(0o500);
            std::fs::set_permissions(dir.path(), perms.clone()).unwrap();
            let can_write = std::fs::write(&test_file, b"x").is_ok();
            // Restore immediately
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("KAT-S1: skipping — chmod 0o500 did not deny writes (running as root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();
        assert_eq!(app.screen, EditorScreen::Browse);

        // Open form + fill required fields
        handle_key(&mut app, press(KeyCode::Char('p')));
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "120000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "130000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "5000");
        // Open modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.mutation_modal.is_some(), "modal must be open");

        // Make vault's parent dir read-only (0o500) so atomic_write's .tmp creation fails
        let parent = vault.parent().unwrap();
        let lock_perms = std::fs::Permissions::from_mode(0o500);
        std::fs::set_permissions(parent, lock_perms).unwrap();

        // Confirm — should fail
        handle_key(&mut app, press(KeyCode::Enter));

        // (1) modal must be closed
        assert!(
            app.mutation_modal.is_none(),
            "KAT-S1: modal must be closed after save failure"
        );
        // (2) form must still be open with buffers intact
        {
            let form = app
                .profile_form
                .as_ref()
                .expect("KAT-S1: form must still be open after save failure");
            assert_eq!(
                form.fields[0].buf, "120000",
                "KAT-S1: form buffer must be intact after save failure"
            );
        }
        // (3) status must contain "Save error"
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error"))
                .unwrap_or(false),
            "KAT-S1: status must contain 'Save error'; got: {:?}",
            app.status
        );
        // (4) vault bytes unchanged
        let bytes_mid = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_mid,
            "KAT-S1: vault must be byte-identical after save failure"
        );

        // Restore permissions
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        // Retry succeeds (idempotent upsert re-runs + save)
        handle_key(&mut app, press(KeyCode::Enter)); // re-open modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm
        assert!(
            app.mutation_modal.is_none(),
            "KAT-S1: retry: modal must close after successful save"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Saved"))
                .unwrap_or(false),
            "KAT-S1: retry: status must say Saved; got: {:?}",
            app.status
        );

        // Profile round-trips
        let stored = app
            .session
            .as_ref()
            .unwrap()
            .tax_profile(2025)
            .unwrap()
            .unwrap();
        use btctax_core::FilingStatus;
        use rust_decimal_macros::dec;
        assert_eq!(stored.filing_status, FilingStatus::Single);
        assert_eq!(stored.ordinary_taxable_income, dec!(120000));

        // Event log still unchanged (side-table upsert)
        let events_after =
            btctax_core::persistence::load_all_ordered(app.session.as_ref().unwrap().conn())
                .unwrap();
        assert!(
            events_after.is_empty(),
            "KAT-S1: event log must remain empty (side-table upsert)"
        );
    }

    // ── KAT-F3: confirm-flow end-to-end ─────────────────────────────────────

    #[test]
    fn kat_f3_confirm_flow_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-f3-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();
        assert_eq!(app.screen, EditorScreen::Browse);

        // p → form
        handle_key(&mut app, press(KeyCode::Char('p')));
        // Fill required fields (focus 0 = filing_status, Down → focus 1)
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "120000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "130000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "5000");

        // Enter → validate → modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.mutation_modal.is_some(),
            "Enter on valid form must open modal"
        );

        // Enter on modal → persist + re-project
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.mutation_modal.is_none(),
            "modal must close after confirm"
        );
        assert!(app.profile_form.is_none(), "form must close after confirm");
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Saved"))
                .unwrap_or(false),
            "status must say Saved; got: {:?}",
            app.status
        );

        // Profile round-trips from the held session
        let stored = app
            .session
            .as_ref()
            .unwrap()
            .tax_profile(2025)
            .unwrap()
            .expect("KAT-F3: profile must be stored");
        use btctax_core::FilingStatus;
        use rust_decimal_macros::dec;
        assert_eq!(stored.filing_status, FilingStatus::Single);
        assert_eq!(stored.ordinary_taxable_income, dec!(120000));
        assert_eq!(stored.magi_excluding_crypto, dec!(130000));
        assert_eq!(stored.qualified_dividends_and_other_pref_income, dec!(5000));

        // Re-projected snapshot reflects the stored profile
        let snap_profile = app
            .snapshot
            .as_ref()
            .unwrap()
            .profiles
            .get(&2025)
            .expect("KAT-F3: snapshot.profiles must include the stored profile");
        assert_eq!(
            snap_profile, &stored,
            "KAT-F3: re-projected snapshot profile must match stored"
        );
    }

    // ── KAT-F4: CLI parity ──────────────────────────────────────────────────

    #[test]
    fn kat_f4_cli_parity_editor_profile_readable_by_cmd_tax_show() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-f4-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Editor flow
        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();
        assert_eq!(app.screen, EditorScreen::Browse);

        handle_key(&mut app, press(KeyCode::Char('p')));
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "120000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "130000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "5000");
        handle_key(&mut app, press(KeyCode::Enter));
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app
            .status
            .as_deref()
            .map(|s| s.contains("Saved"))
            .unwrap_or(false));

        // Save year before dropping app (default is 2025)
        let year = app.selected_year;

        // Drop editor session so CLI can open the same vault
        drop(app);

        // CLI parity: read back via cmd::tax::show_profile
        let cli_profile =
            btctax_cli::cmd::tax::show_profile(&vault, &Passphrase::new(pp_str.into()), year)
                .unwrap()
                .expect("KAT-F4: CLI must be able to read the profile set by the editor");

        use btctax_core::FilingStatus;
        use rust_decimal_macros::dec;
        assert_eq!(cli_profile.filing_status, FilingStatus::Single);
        assert_eq!(cli_profile.ordinary_taxable_income, dec!(120000));
        assert_eq!(cli_profile.magi_excluding_crypto, dec!(130000));
        assert_eq!(
            cli_profile.qualified_dividends_and_other_pref_income,
            dec!(5000)
        );
    }

    // ── Helper: seed a TransferIn vault and return the transfer's EventId ──────

    fn seed_transfer_in_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> btctax_core::EventId {
        use btctax_core::event::{EventPayload, LedgerEvent, TransferIn};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let ti_id = EventId::import(Source::River, SourceRef::new("test-ti-1"));
        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![LedgerEvent {
                id: ti_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: None,
                payload: EventPayload::TransferIn(TransferIn {
                    sat: 500_000,
                    src_addr: None,
                    txid: None,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            session.save().unwrap();
        }
        ti_id
    }

    // Helper: unlock app from vault
    fn open_app(vault: &std::path::Path, pp_str: &str) -> EditorApp {
        let mut app = EditorApp::new(vault.to_path_buf());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();
        assert_eq!(app.screen, EditorScreen::Browse, "must open to Browse");
        app
    }

    // Helper: collect a string from a TestBackend terminal buffer
    fn rendered_text(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect()
    }

    // ── KAT-C2a — cancel-path vault bytes unchanged (classify-inbound) ────────

    #[test]
    fn kat_c2a_cancel_path_vault_bytes_unchanged_classify_inbound() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2a-pass";

        seed_transfer_in_vault(&vault, &key, pp_str);

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // ── c → flow opens at List step ──────────────────────────────────
            handle_key(&mut app, press(KeyCode::Char('c')));
            assert!(
                app.classify_inbound_flow.is_some(),
                "C2a: flow must open on 'c'"
            );

            // 'q' at List step is swallowed (R0-I2 / R2-N1)
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2a: 'q' at List step must be swallowed, not quit"
            );
            assert!(
                app.classify_inbound_flow.is_some(),
                "C2a: flow must remain open after 'q' at List"
            );

            // Enter → variant picker
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.classify_inbound_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyInboundStep::VariantPicker { .. })
                ),
                "C2a: Enter on List must transition to VariantPicker"
            );

            // 'q' at VariantPicker is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2a: 'q' at VariantPicker must be swallowed"
            );
            assert!(app.classify_inbound_flow.is_some());

            // Tab → GiftReceived
            handle_key(&mut app, press(KeyCode::Tab));
            assert!(
                matches!(
                    app.classify_inbound_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyInboundStep::VariantPicker {
                        variant: InboundVariant::GiftReceived,
                        ..
                    })
                ),
                "C2a: Tab on VariantPicker must cycle to GiftReceived"
            );

            // Enter → GiftForm
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.classify_inbound_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyInboundStep::GiftForm { .. })
                ),
                "C2a: Enter on VariantPicker(Gift) must open GiftForm"
            );

            // 'q' at GiftForm (text focus 0) inserts into fmv_at_gift_buf [R2-N1],
            // but does NOT quit and does NOT close the flow.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2a: 'q' at GiftForm must be swallowed (not quit) [R2-N1]"
            );
            assert!(app.classify_inbound_flow.is_some());
            // Backspace out the 'q' before the fmv_at_gift submit.
            handle_key(&mut app, press(KeyCode::Backspace));

            // Type a valid fmv_at_gift value.
            type_str(&mut app, "500.00");

            // Enter → modal opens
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.classify_inbound_modal.is_some(),
                "C2a: Enter on valid GiftForm must open classify_inbound_modal"
            );

            // 'q' while modal open is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2a: 'q' while CI modal open must be swallowed"
            );
            assert!(app.classify_inbound_modal.is_some());

            // Esc → modal closes (back to GiftForm)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.classify_inbound_modal.is_none(),
                "C2a: Esc on CI modal must close the modal"
            );
            assert!(
                matches!(
                    app.classify_inbound_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyInboundStep::GiftForm { .. })
                ),
                "C2a: Esc on CI modal must keep GiftForm open"
            );
            assert!(!app.should_quit, "C2a: Esc on modal must NOT quit");

            // Esc → GiftForm closes (back to VariantPicker)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.classify_inbound_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyInboundStep::VariantPicker { .. })
                ),
                "C2a: Esc on GiftForm must go back to VariantPicker"
            );

            // 'q' at VariantPicker still swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2a: 'q' at VariantPicker (second time) must be swallowed"
            );

            // Esc → VariantPicker closes (back to List)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.classify_inbound_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyInboundStep::List)
                ),
                "C2a: Esc on VariantPicker must go back to List"
            );

            // 'q' at List is still swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2a: 'q' at List (second time) must be swallowed"
            );

            // Esc → flow closes
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.classify_inbound_flow.is_none(),
                "C2a: Esc on List must close the flow"
            );
            assert!(!app.should_quit, "C2a: Esc on List must NOT quit");

            // 'q' in Browse (after flow closes) → quit
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(app.should_quit, "C2a: 'q' after flow closes must quit");
        }

        // Vault must be byte-identical (cancel path — nothing written).
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "KAT-C2a: vault must be byte-identical after full cancel path"
        );
    }

    // ── KAT-S2 — save-error path for classify-inbound (chmod; unix) ──────────

    #[cfg(unix)]
    #[test]
    fn kat_s2_save_error_path_classify_inbound_chmod() {
        use btctax_core::event::{EventPayload, InboundClass, IncomeKind};
        use btctax_core::persistence::load_all_ordered;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-s2-pass";

        seed_transfer_in_vault(&vault, &key, pp_str);

        // Root-skip guard (same pattern as KAT-S1).
        {
            let test_file = dir.path().join("probe.tmp");
            let perms = std::fs::Permissions::from_mode(0o500);
            std::fs::set_permissions(dir.path(), perms).unwrap();
            let can_write = std::fs::write(&test_file, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("KAT-S2: skipping — chmod 0o500 did not deny writes (running as root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();

        let mut app = open_app(&vault, pp_str);

        // Capture pre-state row count.
        let pre_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();

        // Navigate to the CI income modal: c → Enter (list) → Enter (picker=Income)
        // → Tab (kind=Staking) → focus 1 → type FMV → Enter (opens modal).
        handle_key(&mut app, press(KeyCode::Char('c')));
        assert!(app.classify_inbound_flow.is_some(), "S2: flow must open");
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker
        handle_key(&mut app, press(KeyCode::Enter)); // picker → IncomeForm (kind=Mining)
                                                     // Move focus to fmv field (focus 1)
        handle_key(&mut app, press(KeyCode::Down));
        // Type FMV
        type_str(&mut app, "30000.00");
        // Enter → opens CI modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.classify_inbound_modal.is_some(),
            "S2: CI modal must be open"
        );

        // Make vault's parent dir read-only (0o500) → save will fail.
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();

        // Press Enter on modal → save fails.
        handle_key(&mut app, press(KeyCode::Enter));

        // (1) modal must be closed.
        assert!(
            app.classify_inbound_modal.is_none(),
            "S2: CI modal must close after save failure"
        );
        // (2) flow must still be open with the IncomeForm intact.
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::IncomeForm { .. })
            ),
            "S2: IncomeForm must remain open after save failure (buffers intact)"
        );
        // (3) status must contain "Save error".
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error"))
                .unwrap_or(false),
            "S2: status must contain 'Save error'; got: {:?}",
            app.status
        );
        // (4) vault bytes unchanged.
        let bytes_mid = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_mid,
            "S2: vault must be byte-identical after save failure"
        );

        // Restore permissions.
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        // Retry: re-submit the form → modal → confirm → save succeeds.
        handle_key(&mut app, press(KeyCode::Enter)); // re-open modal (IncomeForm still open)
        assert!(
            app.classify_inbound_modal.is_some(),
            "S2: retry: CI modal must re-open on Enter"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → N+2 appended

        // Flow + modal closed on successful save.
        assert!(
            app.classify_inbound_modal.is_none(),
            "S2: retry: modal must close after successful save"
        );
        assert!(
            app.classify_inbound_flow.is_none(),
            "S2: retry: flow must close after successful save"
        );

        // Assert TRUE retry outcome: on-disk log == pre + 2 decision rows [R0-I1].
        let post_disk = {
            drop(app);
            let session2 =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            load_all_ordered(session2.conn()).unwrap()
        };
        let new_decisions: Vec<_> = post_disk
            .iter()
            .skip(pre_len)
            .filter(|r| r.kind == "decision")
            .collect();
        assert_eq!(
            new_decisions.len(),
            2,
            "S2: retry: on-disk log must have EXACTLY 2 new decision rows (N+1 from failed-save + N+2 from retry); got: {}",
            new_decisions.len()
        );

        // Both rows' payloads round-trip to the identical ClassifyInbound payload.
        let p0: EventPayload = serde_json::from_str(&new_decisions[0].payload_json).unwrap();
        let p1: EventPayload = serde_json::from_str(&new_decisions[1].payload_json).unwrap();
        assert_eq!(p0, p1, "S2: both retry rows must have identical payload");
        assert!(
            matches!(
                &p0,
                EventPayload::ClassifyInbound(ci)
                    if matches!(&ci.as_, InboundClass::Income { kind: IncomeKind::Mining, .. })
            ),
            "S2: payload must be ClassifyInbound::Income(Mining); got: {:?}",
            p0
        );

        // The re-projected state after the retry must contain a DecisionConflict
        // attributed to the retry decision's EventId (FIRST-WINS).
        let retry_seq = new_decisions[1]
            .decision_seq
            .expect("retry decision must have decision_seq") as u64;
        let retry_id = btctax_core::EventId::Decision { seq: retry_seq };
        let snap_session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&snap_session).unwrap();
        let has_conflict = snap.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(&retry_id)
        });
        assert!(
            has_conflict,
            "S2: re-projected state must contain DecisionConflict for retry decision {retry_id:?}"
        );
    }

    // ── KAT-E2E-CI — end-to-end classify-inbound (Income with FMV) ───────────

    #[test]
    fn kat_e2e_ci_classify_inbound_income_with_fmv() {
        use btctax_core::event::InboundClass;
        use btctax_core::persistence::load_all_ordered;
        use ratatui::{backend::TestBackend, Terminal};
        use rust_decimal_macros::dec;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-ci-pass";

        let ti_id = seed_transfer_in_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // 1. Confirm seed produces UnknownBasisInbound in the projected state.
        let snap_before = app.snapshot.as_ref().unwrap();
        let has_ubi = snap_before.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
        });
        assert!(
            has_ubi,
            "E2E-CI: seed must produce UnknownBasisInbound blocker"
        );

        // 2. Key-drive the full flow.
        handle_key(&mut app, press(KeyCode::Char('c'))); // open flow
        assert!(
            app.classify_inbound_flow.is_some(),
            "E2E-CI: flow must open"
        );

        // List → Enter → VariantPicker (Income initial)
        handle_key(&mut app, press(KeyCode::Enter));
        // VariantPicker (Income) → Enter → IncomeForm (kind=Mining initial)
        handle_key(&mut app, press(KeyCode::Enter));

        // Tab on kind row → Staking (exercises picker)
        handle_key(&mut app, press(KeyCode::Tab));
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::IncomeForm {
                    kind: IncomeKind::Staking,
                    focus: 0,
                    ..
                })
            ),
            "E2E-CI: one Tab on kind must yield Staking"
        );

        // Move focus to fmv field, type FMV
        handle_key(&mut app, press(KeyCode::Down)); // focus → 1
        type_str(&mut app, "45.50");

        // Enter → CI modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.classify_inbound_modal.is_some(),
            "E2E-CI: Enter on valid IncomeForm must open CI modal"
        );

        // Check modal content via render.
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        let ti_canonical = ti_id.canonical();
        assert!(
            rendered.contains(&ti_canonical),
            "E2E-CI: modal must show canonical EventId; rendered: {rendered}"
        );
        assert!(
            rendered.contains("staking"),
            "E2E-CI: modal must show kind 'staking'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("45.50"),
            "E2E-CI: modal must show FMV 45.50; rendered: {rendered}"
        );

        // Enter on modal → save + re-project.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.classify_inbound_modal.is_none(),
            "E2E-CI: modal must close after confirm"
        );
        assert!(
            app.classify_inbound_flow.is_none(),
            "E2E-CI: flow must close after confirm"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Staking") || s.contains("staking"))
                .unwrap_or(false),
            "E2E-CI: status must contain kind; got: {:?}",
            app.status
        );

        // 3. Reopen + project → UnknownBasisInbound gone; IncomeRecord + Lot present.
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();

        let ubi_gone = !snap.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
        });
        assert!(
            ubi_gone,
            "E2E-CI: UnknownBasisInbound must be gone after classification"
        );

        let income_rec = snap
            .state
            .income_recognized
            .iter()
            .find(|r| r.event == ti_id);
        assert!(
            income_rec.is_some(),
            "E2E-CI: IncomeRecord must be present for the classified TransferIn"
        );
        let ir = income_rec.unwrap();
        assert_eq!(
            ir.kind,
            IncomeKind::Staking,
            "E2E-CI: IncomeRecord.kind must be Staking"
        );
        assert_eq!(
            ir.usd_fmv,
            dec!(45.50),
            "E2E-CI: IncomeRecord.usd_fmv must be 45.50"
        );

        let lot = snap.state.lots.iter().find(|l| {
            // The lot's basis comes from the Income path; look for the sat count.
            l.original_sat == 500_000
        });
        assert!(
            lot.is_some(),
            "E2E-CI: a Lot with 500_000 sat must be present after classification"
        );

        // 4. Check the event log has the new decision.
        let events = load_all_ordered(session2.conn()).unwrap();
        let decision_rows: Vec<_> = events.iter().filter(|r| r.kind == "decision").collect();
        assert_eq!(
            decision_rows.len(),
            1,
            "E2E-CI: exactly one decision row must be appended"
        );
        let stored_payload: btctax_core::EventPayload =
            serde_json::from_str(&decision_rows[0].payload_json).unwrap();
        assert!(
            matches!(
                &stored_payload,
                btctax_core::EventPayload::ClassifyInbound(ci)
                    if ci.transfer_in_event == ti_id
                    && matches!(&ci.as_, InboundClass::Income { kind: IncomeKind::Staking, .. })
            ),
            "E2E-CI: stored payload must be ClassifyInbound(Staking); got: {:?}",
            stored_payload
        );
    }

    // ── KAT-E2E-FMV-MISSING — classify-inbound Income without FMV ────────────

    #[test]
    fn kat_e2e_fmv_missing_classify_inbound_income_no_fmv() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-fmv-miss-pass";

        let ti_id = seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // c → Enter → Enter (Income, no Tab) → Enter (fmv_buf empty) → modal → Enter
        handle_key(&mut app, press(KeyCode::Char('c')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker
        handle_key(&mut app, press(KeyCode::Enter)); // picker → IncomeForm (Mining)
                                                     // Leave fmv_buf EMPTY (focus stays at 0, kind=Mining).
        handle_key(&mut app, press(KeyCode::Enter)); // validates OK (fmv optional) → modal
        assert!(
            app.classify_inbound_modal.is_some(),
            "FMV-MISSING: modal must open with empty fmv"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save + re-project

        assert!(
            app.classify_inbound_modal.is_none(),
            "FMV-MISSING: modal must close after confirm"
        );
        assert!(
            app.classify_inbound_flow.is_none(),
            "FMV-MISSING: flow must close after confirm"
        );

        // Status must contain "FmvMissing" AND "void" (R0-I4 remedy).
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("FmvMissing") || status.contains("FMV missing"),
            "FMV-MISSING: status must mention FmvMissing; got: {status}"
        );
        assert!(
            status.contains("void"),
            "FMV-MISSING: status must mention 'void' (the CLI remedy); got: {status}"
        );
        // Must NOT suggest set-fmv (R0-I4).
        assert!(
            !status.contains("set-fmv"),
            "FMV-MISSING: status must NOT suggest set-fmv (R0-I4); got: {status}"
        );

        // Re-project: FmvMissing blocker is present; lot with basis_pending.
        let snap = app.snapshot.as_ref().unwrap();
        let has_fmv_missing = snap
            .state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(&ti_id));
        assert!(
            has_fmv_missing,
            "FMV-MISSING: re-projected state must have FmvMissing blocker"
        );

        let lot = snap.state.lots.iter().find(|l| l.original_sat == 500_000);
        assert!(
            lot.is_some(),
            "FMV-MISSING: lot must be created even without FMV (basis_pending)"
        );
        assert!(
            lot.unwrap().basis_pending,
            "FMV-MISSING: lot must have basis_pending=true when FMV missing"
        );
    }

    // ── KAT-E2E-GIFT-UNKNOWN — both donor fields empty ────────────────────────

    #[test]
    fn kat_e2e_gift_unknown_both_donor_fields_empty() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-gift-unk-pass";

        let _ti_id = seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // c → Enter → Tab(Gift) → Enter → fmv_at_gift → Enter → modal → Enter
        handle_key(&mut app, press(KeyCode::Char('c')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker (Income initial)
        handle_key(&mut app, press(KeyCode::Tab)); // cycle to GiftReceived
        handle_key(&mut app, press(KeyCode::Enter)); // picker → GiftForm
        type_str(&mut app, "300.00"); // fmv_at_gift (required); donor fields empty
        handle_key(&mut app, press(KeyCode::Enter)); // validates → modal
        assert!(
            app.classify_inbound_modal.is_some(),
            "GIFT-UNK: modal must open"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        assert!(app.classify_inbound_modal.is_none());
        assert!(app.classify_inbound_flow.is_none());

        // Status must contain "UnknownBasisInbound" (or "basis unknown") AND "void".
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("UnknownBasisInbound") || status.contains("basis unknown"),
            "GIFT-UNK: status must mention UnknownBasisInbound; got: {status}"
        );
        assert!(
            status.contains("void"),
            "GIFT-UNK: status must mention 'void'; got: {status}"
        );

        // Re-projected: original UBI gone (classified); new UBI fires (gift case 4).
        let snap = app.snapshot.as_ref().unwrap();
        let new_ubi = snap
            .state
            .blockers
            .iter()
            .find(|b| b.kind == BlockerKind::UnknownBasisInbound);
        assert!(
            new_ubi.is_some(),
            "GIFT-UNK: UnknownBasisInbound must re-fire after gift with no donor info"
        );

        // The classify-inbound list (c) must NOT show this TransferIn again
        // (it has a ClassifyInbound decision → pre-filtered out).
        // We verify by re-opening the flow and checking it either shows empty status
        // or opens without this ti_id.
        //
        // Since we already saved the first CI decision, the flow's pre-filter's
        // "already_classified" set will contain ti_id → it won't appear in items.
        // The easiest check: close app, rebuild snapshot fresh, re-run filter logic.
        //
        // For KAT purposes: simply re-open the classify-inbound flow and assert
        // "No unclassified inbound transfers" status (the only TransferIn is now classified).
        drop(app);
        let session3 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let mut app3 = EditorApp::new(vault.clone());
        app3.session = Some(session3);
        let (snap3, _) =
            btctax_tui::unlock::build_snapshot(app3.session.as_ref().unwrap()).unwrap();
        app3.snapshot = Some(snap3);
        app3.screen = EditorScreen::Browse;

        handle_key(&mut app3, press(KeyCode::Char('c')));
        assert!(
            app3.classify_inbound_flow.is_none(),
            "GIFT-UNK: c must not open flow (TransferIn already classified)"
        );
        assert!(
            app3.status
                .as_deref()
                .map(|s| s.contains("No unclassified") || s.contains("no unclassified"))
                .unwrap_or(false),
            "GIFT-UNK: c must set 'No unclassified inbound transfers' status; got: {:?}",
            app3.status
        );
    }

    // ── KAT-E2E-GIFT-PRICE-GAP — donor date outside bundled price dataset ─────

    #[test]
    fn kat_e2e_gift_price_gap_donor_date_outside_price_dataset() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-price-gap-pass";

        let _ti_id = seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // c → Enter → Tab(Gift) → Enter → GiftForm
        handle_key(&mut app, press(KeyCode::Char('c')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker
        handle_key(&mut app, press(KeyCode::Tab)); // cycle to GiftReceived
        handle_key(&mut app, press(KeyCode::Enter)); // picker → GiftForm

        // fmv_at_gift
        type_str(&mut app, "500.00");
        // Tab to donor_basis (leave empty)
        handle_key(&mut app, press(KeyCode::Tab));
        // Tab to donor_acquired_at
        handle_key(&mut app, press(KeyCode::Tab));
        // Type a date OUTSIDE the bundled price dataset (e.g. 1990-01-01).
        type_str(&mut app, "1990-01-01");

        handle_key(&mut app, press(KeyCode::Enter)); // validates → modal
        assert!(
            app.classify_inbound_modal.is_some(),
            "PRICE-GAP: modal must open"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        assert!(app.classify_inbound_modal.is_none());
        assert!(app.classify_inbound_flow.is_none());

        // Status must come from RE-PROJECTED blockers, not payload shape [R0-I5].
        // Since donor_acquired_at=1990-01-01 is outside the price dataset, the fold
        // fires UnknownBasisInbound (gift case 3, fold.rs:913–927).
        // Status must mention "UnknownBasisInbound" (or "basis unknown") AND "void".
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("UnknownBasisInbound") || status.contains("basis unknown"),
            "PRICE-GAP: status must come from re-projected UnknownBasisInbound [R0-I5]; got: {status}"
        );
        assert!(
            status.contains("void"),
            "PRICE-GAP: status must mention 'void'; got: {status}"
        );

        // Re-projected: UnknownBasisInbound re-fired (gift case 3).
        let snap = app.snapshot.as_ref().unwrap();
        let ubi_refired = snap
            .state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::UnknownBasisInbound);
        assert!(
            ubi_refired,
            "PRICE-GAP: UnknownBasisInbound must re-fire for out-of-range donor date"
        );
    }

    // ── KAT-E2E-GIFT-DUAL — gift happy path with dual basis ──────────────────

    #[test]
    fn kat_e2e_gift_dual_basis_fmv_less_than_donor_basis() {
        use rust_decimal_macros::dec;
        use time::macros::date;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-gift-dual-pass";

        let ti_id = seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // c → Enter → Tab(Gift) → Enter → GiftForm
        handle_key(&mut app, press(KeyCode::Char('c')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker
        handle_key(&mut app, press(KeyCode::Tab)); // cycle to GiftReceived
        handle_key(&mut app, press(KeyCode::Enter)); // picker → GiftForm

        // fmv_at_gift = 400 (LESS than donor_basis = 500 → dual basis case 2)
        type_str(&mut app, "400.00");
        // Tab to donor_basis
        handle_key(&mut app, press(KeyCode::Tab));
        type_str(&mut app, "500.00");
        // Tab to donor_acquired_at
        handle_key(&mut app, press(KeyCode::Tab));
        // Date that IS in the bundled price dataset (2024-01-01 should be covered).
        type_str(&mut app, "2024-01-01");

        handle_key(&mut app, press(KeyCode::Enter)); // validate → modal
        assert!(
            app.classify_inbound_modal.is_some(),
            "GIFT-DUAL: modal must open"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save

        assert!(app.classify_inbound_modal.is_none());
        assert!(app.classify_inbound_flow.is_none());

        // Clean success status (no new blocker for this event).
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("GiftReceived") || status.contains("gift"),
            "GIFT-DUAL: status must mention GiftReceived; got: {status}"
        );
        // No UnknownBasisInbound or FmvMissing.
        assert!(
            !status.contains("UnknownBasisInbound") && !status.contains("FmvMissing"),
            "GIFT-DUAL: status must be clean success; got: {status}"
        );

        // Re-project: no new UnknownBasisInbound for ti_id; dual-basis lot present.
        let snap = app.snapshot.as_ref().unwrap();
        let ubi_for_target = snap.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
        });
        assert!(
            !ubi_for_target,
            "GIFT-DUAL: must have no UBI for the target after dual-basis gift"
        );

        // Check the gift lot's dual-basis fields.
        // The lot is created in fold.rs gift case 2: fmv_at_gift(400) < donor_basis(500).
        // usd_basis = donor_basis (500), dual_loss_basis = Some(fmv_at_gift = 400).
        let gift_lot = snap.state.lots.iter().find(|l| l.original_sat == 500_000);
        assert!(gift_lot.is_some(), "GIFT-DUAL: gift lot must be present");
        let lot = gift_lot.unwrap();
        assert_eq!(
            lot.usd_basis,
            dec!(500.00),
            "GIFT-DUAL: lot.usd_basis must equal donor_basis (500.00)"
        );
        assert_eq!(
            lot.dual_loss_basis,
            Some(dec!(400.00)),
            "GIFT-DUAL: lot.dual_loss_basis must equal fmv_at_gift (400.00)"
        );
        assert_eq!(
            lot.donor_acquired_at,
            Some(date!(2024 - 01 - 01)),
            "GIFT-DUAL: lot.donor_acquired_at must be carried through"
        );
    }

    // ── E2E happy path: the TUI Tax tab now computes ─────────────────────────
    //
    // Spec Task-3 E2E: set a profile via the full key-driven flow → the Tax tab
    // switches from "NOT COMPUTABLE [TaxProfileMissing]" to a computed report.
    // (Reopen + CLI read-back are covered by KAT-F4; this closes the loop on the
    // re-projected snapshot actually feeding compute_tax_year.)

    #[test]
    fn e2e_tax_tab_computes_after_profile_set() {
        use ratatui::{backend::TestBackend, Terminal};

        fn rendered_text(terminal: &Terminal<TestBackend>) -> String {
            terminal
                .backend()
                .buffer()
                .clone()
                .content()
                .iter()
                .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                .collect()
        }

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "e2e-tax-tab-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();
        assert_eq!(app.screen, EditorScreen::Browse);
        assert_eq!(app.selected_year, 2025, "empty ledger defaults to 2025");

        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        // BEFORE: no profile for 2025 → the Tax tab is NOT COMPUTABLE.
        app.tab = Tab::Tax;
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let before = rendered_text(&terminal);
        assert!(
            before.contains("NOT COMPUTABLE"),
            "Tax tab must be NOT COMPUTABLE before a profile is set; rendered:\n{before}"
        );

        // Full key-driven flow: p → fill required fields → Enter → Enter (confirm).
        handle_key(&mut app, press(KeyCode::Char('p')));
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "120000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "130000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "5000");
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.mutation_modal.is_some(), "modal must open");
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Saved"))
                .unwrap_or(false),
            "status must say Saved; got: {:?}",
            app.status
        );

        // AFTER: the re-projected snapshot feeds compute_tax_year → the Tax tab computes.
        app.tab = Tab::Tax;
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let after = rendered_text(&terminal);
        assert!(
            !after.contains("NOT COMPUTABLE"),
            "Tax tab must compute after the profile is set; rendered:\n{after}"
        );
        assert!(
            after.contains("TOTAL federal tax attributable"),
            "Tax tab must show the computed report after the profile is set; rendered:\n{after}"
        );
    }

    // ── Helper: seed a TransferOut vault and return the TransferOut EventId ───

    fn seed_transfer_out_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
        principal_sat: i64,
    ) -> btctax_core::EventId {
        use btctax_core::event::{EventPayload, LedgerEvent, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let to_id = EventId::import(Source::River, SourceRef::new("test-to-1"));
        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            // wallet MUST be set: fold.rs Op::PendingOut fires UncoveredDisposal and returns
            // early when wallet is None, so the event never reaches pending_reconciliation.
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: to_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: EventPayload::TransferOut(TransferOut {
                    sat: principal_sat,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            session.save().unwrap();
        }
        to_id
    }

    /// Seed a vault with an Acquire lot (ensuring pool coverage) PLUS a TransferOut.
    ///
    /// Used by DONATE and GIFTOUT tests that verify `snap.state.removals` — which the core
    /// only populates when `consumed.is_empty() == false` in the GiftOut/Donate fold arm.
    /// The Acquire and TransferOut share the same wallet; the Acquire timestamp is 1 day
    /// before the TransferOut so FIFO yields the Acquire lot first.
    fn seed_transfer_out_vault_with_lots(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
        principal_sat: i64,
    ) -> btctax_core::EventId {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();

        let wallet = Some(btctax_core::WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        // Acquire event 1 day before TransferOut.
        let acq_id = EventId::import(Source::River, SourceRef::new("test-acq-1"));
        let to_id = EventId::import(Source::River, SourceRef::new("test-to-1"));
        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![
                LedgerEvent {
                    id: acq_id.clone(),
                    // 1 day before TransferOut (1_748_000_000 - 86400 = 1_747_913_600)
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_747_913_600).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::Acquire(Acquire {
                        sat: principal_sat,
                        usd_cost: dec!(50000.00),
                        fee_usd: dec!(0.00),
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: to_id.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet,
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: principal_sat,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            session.save().unwrap();
        }
        to_id
    }

    // ── KAT-C2b — cancel-path vault bytes unchanged (reclassify-outflow) ─────

    #[test]
    fn kat_c2b_cancel_path_vault_bytes_unchanged_reclassify_outflow() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2b-pass";

        seed_transfer_out_vault(&vault, &key, pp_str, 500_000);

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // ── o → flow opens at List step ──────────────────────────────────
            handle_key(&mut app, press(KeyCode::Char('o')));
            assert!(
                app.reclassify_outflow_flow.is_some(),
                "C2b: flow must open on 'o'"
            );

            // 'q' at List step is swallowed [R0-I2]
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2b: 'q' at List step must be swallowed, not quit"
            );
            assert!(
                app.reclassify_outflow_flow.is_some(),
                "C2b: flow must remain open after 'q' at List"
            );

            // Enter → KindPicker (Sell initial)
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyOutflowStep::KindPicker {
                        kind: OutflowKind::Sell,
                        ..
                    })
                ),
                "C2b: Enter on List must transition to KindPicker(Sell)"
            );

            // 'q' at KindPicker is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2b: 'q' at KindPicker must be swallowed");
            assert!(app.reclassify_outflow_flow.is_some());

            // Enter → FieldForm (Sell)
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyOutflowStep::FieldForm {
                        kind: OutflowKind::Sell,
                        ..
                    })
                ),
                "C2b: Enter on KindPicker must open FieldForm(Sell)"
            );

            // 'q' at FieldForm (text focus 0) inserts into amount_buf [R2-N1];
            // does NOT quit and does NOT close the flow.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2b: 'q' at FieldForm must be swallowed (not quit) [R2-N1]"
            );
            assert!(app.reclassify_outflow_flow.is_some());
            // Backspace out the 'q'
            handle_key(&mut app, press(KeyCode::Backspace));

            // Type a valid amount value
            type_str(&mut app, "640.00");

            // Enter → modal opens
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.reclassify_outflow_modal.is_some(),
                "C2b: Enter on valid FieldForm must open reclassify_outflow_modal"
            );

            // 'q' while modal open is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2b: 'q' while RO modal open must be swallowed"
            );
            assert!(app.reclassify_outflow_modal.is_some());

            // Esc → modal closes (back to FieldForm)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.reclassify_outflow_modal.is_none(),
                "C2b: Esc on RO modal must close the modal"
            );
            assert!(
                matches!(
                    app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyOutflowStep::FieldForm { .. })
                ),
                "C2b: Esc on RO modal must keep FieldForm open"
            );
            assert!(!app.should_quit, "C2b: Esc on modal must NOT quit");

            // Esc → FieldForm closes (back to KindPicker)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyOutflowStep::KindPicker { .. })
                ),
                "C2b: Esc on FieldForm must go back to KindPicker"
            );

            // 'q' at KindPicker still swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2b: 'q' at KindPicker (second time) must be swallowed"
            );

            // Esc → KindPicker closes (back to List)
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyOutflowStep::List)
                ),
                "C2b: Esc on KindPicker must go back to List"
            );

            // 'q' at List is still swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2b: 'q' at List (second time) must be swallowed"
            );

            // Esc → flow closes
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.reclassify_outflow_flow.is_none(),
                "C2b: Esc on List must close the flow"
            );
            assert!(!app.should_quit, "C2b: Esc on List must NOT quit");

            // 'q' in Browse (after flow closes) → quit
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(app.should_quit, "C2b: 'q' after flow closes must quit");
        }

        // Vault must be byte-identical (cancel path — nothing written).
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "KAT-C2b: vault must be byte-identical after full cancel path"
        );
    }

    // ── KAT-E2E-RO — end-to-end reclassify-outflow (Sell) ───────────────────

    #[test]
    fn kat_e2e_ro_reclassify_outflow_sell() {
        use btctax_core::persistence::load_all_ordered;
        use ratatui::{backend::TestBackend, Terminal};
        use rust_decimal_macros::dec;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-ro-pass";

        let to_id = seed_transfer_out_vault(&vault, &key, pp_str, 500_000);

        let mut app = open_app(&vault, pp_str);

        // 1. Confirm seed produces a pending_reconciliation entry.
        let snap_before = app.snapshot.as_ref().unwrap();
        let has_pending = snap_before
            .state
            .pending_reconciliation
            .iter()
            .any(|pt| pt.event == to_id);
        assert!(
            has_pending,
            "E2E-RO: seed must produce pending_reconciliation entry"
        );

        // 2. Key-drive: o → list → Enter → KindPicker (Sell initial) → Enter → FieldForm.
        handle_key(&mut app, press(KeyCode::Char('o')));
        assert!(
            app.reclassify_outflow_flow.is_some(),
            "E2E-RO: flow must open"
        );

        // List → Enter → KindPicker (Sell)
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                Some(ReclassifyOutflowStep::KindPicker {
                    kind: OutflowKind::Sell,
                    ..
                })
            ),
            "E2E-RO: Enter on List must open KindPicker(Sell)"
        );

        // KindPicker (Sell) → Enter → FieldForm(Sell)
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                Some(ReclassifyOutflowStep::FieldForm {
                    kind: OutflowKind::Sell,
                    ..
                })
            ),
            "E2E-RO: Enter on KindPicker must open FieldForm(Sell)"
        );

        // Assert the amount label is "gross proceeds" for sell [R0-I3].
        // We check the rendered form in the terminal.
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("gross proceeds"),
            "E2E-RO: FieldForm for sell must show 'gross proceeds' label [R0-I3]; rendered: {rendered}"
        );

        // Type amount.
        type_str(&mut app, "640.00");

        // Enter → modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_outflow_modal.is_some(),
            "E2E-RO: Enter on valid FieldForm must open RO modal"
        );

        // Check modal content.
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        let to_canonical = to_id.canonical();
        assert!(
            rendered.contains(&to_canonical),
            "E2E-RO: modal must show canonical EventId; rendered: {rendered}"
        );
        assert!(
            rendered.contains("sell"),
            "E2E-RO: modal must show kind 'sell'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("640"),
            "E2E-RO: modal must show amount 640; rendered: {rendered}"
        );

        // Enter on modal → save + re-project.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_outflow_modal.is_none(),
            "E2E-RO: modal must close after confirm"
        );
        assert!(
            app.reclassify_outflow_flow.is_none(),
            "E2E-RO: flow must close after confirm"
        );
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("sell") || status.contains("Sell"),
            "E2E-RO: status must mention sell; got: {status}"
        );

        // 3. Reopen + project → pending_reconciliation entry gone; Disposal appears.
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();

        let still_pending = snap
            .state
            .pending_reconciliation
            .iter()
            .any(|pt| pt.event == to_id);
        assert!(
            !still_pending,
            "E2E-RO: target must be gone from pending_reconciliation after reclassify"
        );

        let disposal = snap.state.disposals.iter().find(|d| d.event == to_id);
        // Note: disposal may be present as a Disposal (or UncoveredDisposal blocker
        // if pool is short). The test vault has no lots, so UncoveredDisposal will fire.
        // We verify the pending entry is gone and check the decision row.
        let events = load_all_ordered(session2.conn()).unwrap();
        let decision_rows: Vec<_> = events.iter().filter(|r| r.kind == "decision").collect();
        assert_eq!(
            decision_rows.len(),
            1,
            "E2E-RO: exactly one decision row must be appended"
        );
        let stored_payload: btctax_core::EventPayload =
            serde_json::from_str(&decision_rows[0].payload_json).unwrap();
        assert!(
            matches!(
                &stored_payload,
                btctax_core::EventPayload::ReclassifyOutflow(ro)
                    if ro.transfer_out_event == to_id
                    && matches!(&ro.as_, OutflowClass::Dispose { kind: DisposeKind::Sell })
                    && ro.principal_proceeds_or_fmv == dec!(640.00)
            ),
            "E2E-RO: stored payload must be ReclassifyOutflow(Sell, 640.00); got: {:?}",
            stored_payload
        );
        let _ = disposal; // present or not depends on lot coverage; not the critical assertion
    }

    // ── KAT-E2E-UNCOVERED — reclassify-outflow with UncoveredDisposal ─────────

    #[test]
    fn kat_e2e_uncovered_reclassify_outflow_uncovered_disposal() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-uncovered-pass";

        // Seed a TransferOut with a large sat count (no lots in vault → always uncovered).
        let to_id = seed_transfer_out_vault(&vault, &key, pp_str, 5_000_000);

        let mut app = open_app(&vault, pp_str);

        // Assert the PRE-STATE: the uncovered PendingOut already fires UncoveredDisposal
        // before reclassification [R0-M6].
        let snap_before = app.snapshot.as_ref().unwrap();
        let pre_uncovered =
            snap_before.state.blockers.iter().any(|b| {
                b.kind == BlockerKind::UncoveredDisposal && b.event.as_ref() == Some(&to_id)
            });
        assert!(
            pre_uncovered,
            "E2E-UNCOVERED: pre-state must have UncoveredDisposal for the pending TransferOut [R0-M6]"
        );

        // Reclassify as sell: o → Enter → Enter → type amount → Enter → modal → Enter.
        handle_key(&mut app, press(KeyCode::Char('o')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → KindPicker
        handle_key(&mut app, press(KeyCode::Enter)); // KindPicker → FieldForm(Sell)
        type_str(&mut app, "1000.00");
        handle_key(&mut app, press(KeyCode::Enter)); // FieldForm → modal
        assert!(
            app.reclassify_outflow_modal.is_some(),
            "UNCOVERED: modal must open"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        assert!(app.reclassify_outflow_modal.is_none());
        assert!(app.reclassify_outflow_flow.is_none());

        // Status must contain "UncoveredDisposal".
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("UncoveredDisposal"),
            "E2E-UNCOVERED: status must contain 'UncoveredDisposal'; got: {status}"
        );

        // Re-projected: UncoveredDisposal still present after reclassification
        // (now from the Dispose consume path instead of PendingOut).
        let snap = app.snapshot.as_ref().unwrap();
        let post_uncovered = snap
            .state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::UncoveredDisposal);
        assert!(
            post_uncovered,
            "E2E-UNCOVERED: UncoveredDisposal must still be present after reclassify (lot pool short)"
        );

        // The pending entry must be gone.
        let still_pending = snap
            .state
            .pending_reconciliation
            .iter()
            .any(|pt| pt.event == to_id);
        assert!(
            !still_pending,
            "E2E-UNCOVERED: target must be gone from pending_reconciliation after reclassify"
        );
    }

    // ── KAT-E2E-DONATE — end-to-end reclassify-outflow (Donate with appraisal + donee) ─

    #[test]
    fn kat_e2e_donate_reclassify_outflow_donate_appraisal_donee() {
        use ratatui::{backend::TestBackend, Terminal};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-donate-pass";

        // Use the lot-seeded helper: GiftOut/Donate fold arm only pushes Removal when
        // consumed.is_empty() == false, which requires pre-existing lots in the pool.
        let to_id = seed_transfer_out_vault_with_lots(&vault, &key, pp_str, 100_000);
        let mut app = open_app(&vault, pp_str);

        // o → List → Enter → KindPicker → Tab×3 → Donate → Enter → FieldForm(Donate)
        handle_key(&mut app, press(KeyCode::Char('o')));
        handle_key(&mut app, press(KeyCode::Enter)); // List → KindPicker(Sell)
                                                     // Cycle: Sell → Spend → Gift → Donate
        handle_key(&mut app, press(KeyCode::Tab)); // Spend
        handle_key(&mut app, press(KeyCode::Tab)); // Gift
        handle_key(&mut app, press(KeyCode::Tab)); // Donate
        assert!(
            matches!(
                app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                Some(ReclassifyOutflowStep::KindPicker {
                    kind: OutflowKind::Donate,
                    ..
                })
            ),
            "DONATE: 3 Tabs must reach Donate"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // KindPicker → FieldForm(Donate)

        // Assert amount label is "FMV" for donate.
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("FMV"),
            "DONATE: FieldForm for donate must show 'FMV' label; rendered: {rendered}"
        );

        // Fill amount
        type_str(&mut app, "5000.00");

        // Tab to fee (skip), Tab to appraisal (focus 2 for donate)
        handle_key(&mut app, press(KeyCode::Tab)); // focus → 1 (fee)
        handle_key(&mut app, press(KeyCode::Tab)); // focus → 2 (appraisal, donate only)

        // Toggle appraisal via Space
        handle_key(&mut app, press(KeyCode::Char(' ')));
        // Tab to donee (focus 3)
        handle_key(&mut app, press(KeyCode::Tab)); // focus → 3 (donee)
        type_str(&mut app, "Community Foundation");

        // Enter → modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_outflow_modal.is_some(),
            "DONATE: Enter on valid FieldForm must open RO modal"
        );

        // Check modal content: appraisal_required: true AND donee shown [R0-I7].
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("appraisal"),
            "DONATE: modal must show appraisal field; rendered: {rendered}"
        );
        assert!(
            rendered.contains("true"),
            "DONATE: modal must show appraisal_required: true; rendered: {rendered}"
        );
        assert!(
            rendered.contains("Community Foundation") || rendered.contains("Community"),
            "DONATE: modal must show donee; rendered: {rendered}"
        );

        // Confirm
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.reclassify_outflow_modal.is_none());
        assert!(app.reclassify_outflow_flow.is_none());

        // Re-projected: Removal with kind=Donation, appraisal_required=true, donee=Some.
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();

        let removal = snap.state.removals.iter().find(|r| r.event == to_id);
        assert!(
            removal.is_some(),
            "DONATE: Removal must be present for the donate TransferOut"
        );
        let r = removal.unwrap();
        assert_eq!(
            r.kind,
            btctax_core::RemovalKind::Donation,
            "DONATE: Removal.kind must be Donation"
        );
        assert!(
            r.appraisal_required,
            "DONATE: appraisal_required must be true"
        );
        assert!(
            r.donee
                .as_deref()
                .map(|d| d.contains("Community"))
                .unwrap_or(false),
            "DONATE: donee must be Some containing 'Community'; got: {:?}",
            r.donee
        );
    }

    // ── KAT-E2E-GIFTOUT-DONEE — gift-path modal shows donee [R0-I7] ──────────

    #[test]
    fn kat_e2e_giftout_donee_modal_shows_donee_no_appraisal() {
        use ratatui::{backend::TestBackend, Terminal};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-giftout-pass";

        // Use the lot-seeded helper: GiftOut fold arm only pushes Removal when
        // consumed.is_empty() == false, which requires pre-existing lots in the pool.
        let to_id = seed_transfer_out_vault_with_lots(&vault, &key, pp_str, 100_000);
        let mut app = open_app(&vault, pp_str);

        // o → List → Enter → KindPicker → Tab×2 → Gift → Enter → FieldForm(Gift)
        handle_key(&mut app, press(KeyCode::Char('o')));
        handle_key(&mut app, press(KeyCode::Enter)); // List → KindPicker(Sell)
        handle_key(&mut app, press(KeyCode::Tab)); // Spend
        handle_key(&mut app, press(KeyCode::Tab)); // Gift
        assert!(
            matches!(
                app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                Some(ReclassifyOutflowStep::KindPicker {
                    kind: OutflowKind::Gift,
                    ..
                })
            ),
            "GIFTOUT: 2 Tabs must reach Gift"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // KindPicker → FieldForm(Gift)

        // Fill amount (FMV for gift)
        type_str(&mut app, "640.00");

        // Tab to fee (focus 1)
        handle_key(&mut app, press(KeyCode::Tab)); // focus → 1 (fee), leave empty
                                                   // Tab to donee (focus 3; focus 2/appraisal skipped for gift)
        handle_key(&mut app, press(KeyCode::Tab)); // focus → 3 (donee, skipping appraisal)
        type_str(&mut app, "Alice");

        // Enter → modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_outflow_modal.is_some(),
            "GIFTOUT: Enter on valid FieldForm must open RO modal"
        );

        // Assert modal shows donee value AND does NOT show appraisal_required [R0-I7].
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("Alice"),
            "GIFTOUT: modal must show donee 'Alice' [R0-I7]; rendered: {rendered}"
        );
        // appraisal_required must NOT be shown for gift.
        assert!(
            !rendered.contains("appraisal_required"),
            "GIFTOUT: modal must NOT show appraisal_required for gift [R0-I7]; rendered: {rendered}"
        );

        // Confirm
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.reclassify_outflow_modal.is_none());
        assert!(app.reclassify_outflow_flow.is_none());

        // Re-projected: Removal with kind=Gift, donee=Some("Alice").
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();

        let removal = snap.state.removals.iter().find(|r| r.event == to_id);
        assert!(
            removal.is_some(),
            "GIFTOUT: Removal must be present for the gift TransferOut"
        );
        let r = removal.unwrap();
        assert_eq!(
            r.kind,
            btctax_core::RemovalKind::Gift,
            "GIFTOUT: Removal.kind must be Gift"
        );
        assert_eq!(
            r.donee,
            Some("Alice".to_string()),
            "GIFTOUT: Removal.donee must be Some('Alice')"
        );
    }

    // ── KAT-S2-RO — save-error path for reclassify-outflow (chmod; unix) [R2-N3] ─

    #[cfg(unix)]
    #[test]
    fn kat_s2_ro_save_error_path_reclassify_outflow_chmod() {
        use btctax_core::event::{EventPayload, OutflowClass};
        use btctax_core::persistence::load_all_ordered;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-s2-ro-pass";

        seed_transfer_out_vault(&vault, &key, pp_str, 500_000);

        // Root-skip guard (same pattern as KAT-S1 / KAT-S2).
        {
            let test_file = dir.path().join("probe.tmp");
            let perms = std::fs::Permissions::from_mode(0o500);
            std::fs::set_permissions(dir.path(), perms).unwrap();
            let can_write = std::fs::write(&test_file, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!(
                    "KAT-S2-RO: skipping — chmod 0o500 did not deny writes (running as root?)"
                );
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();

        let mut app = open_app(&vault, pp_str);

        // Capture pre-state row count.
        let pre_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();

        // Navigate to the RO sell modal: o → Enter (list) → Enter (picker=Sell)
        // → type amount → Enter (opens modal).
        handle_key(&mut app, press(KeyCode::Char('o')));
        assert!(
            app.reclassify_outflow_flow.is_some(),
            "S2-RO: flow must open"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // list → KindPicker
        handle_key(&mut app, press(KeyCode::Enter)); // KindPicker → FieldForm(Sell)
        type_str(&mut app, "640.00");
        handle_key(&mut app, press(KeyCode::Enter)); // FieldForm → modal
        assert!(
            app.reclassify_outflow_modal.is_some(),
            "S2-RO: RO modal must be open"
        );

        // Make vault's parent dir read-only (0o500) → save will fail.
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();

        // Press Enter on modal → save fails.
        handle_key(&mut app, press(KeyCode::Enter));

        // (1) modal must be closed.
        assert!(
            app.reclassify_outflow_modal.is_none(),
            "S2-RO: RO modal must close after save failure"
        );
        // (2) flow must still be open with the FieldForm intact.
        assert!(
            matches!(
                app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
                Some(ReclassifyOutflowStep::FieldForm { .. })
            ),
            "S2-RO: FieldForm must remain open after save failure (buffers intact)"
        );
        // (3) status must contain "Save error".
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error"))
                .unwrap_or(false),
            "S2-RO: status must contain 'Save error'; got: {:?}",
            app.status
        );
        // (4) vault bytes unchanged.
        let bytes_mid = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_mid,
            "S2-RO: vault must be byte-identical after save failure"
        );

        // Restore permissions.
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        // Retry: re-submit the form → modal → confirm → save succeeds.
        handle_key(&mut app, press(KeyCode::Enter)); // re-open modal (FieldForm still open)
        assert!(
            app.reclassify_outflow_modal.is_some(),
            "S2-RO: retry: RO modal must re-open on Enter"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → N+2 appended

        // Flow + modal closed on successful save.
        assert!(
            app.reclassify_outflow_modal.is_none(),
            "S2-RO: retry: modal must close after successful save"
        );
        assert!(
            app.reclassify_outflow_flow.is_none(),
            "S2-RO: retry: flow must close after successful save"
        );

        // Assert TRUE retry outcome: on-disk log == pre + 2 decision rows [R0-I1].
        let post_disk = {
            drop(app);
            let session2 =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            load_all_ordered(session2.conn()).unwrap()
        };
        let new_decisions: Vec<_> = post_disk
            .iter()
            .skip(pre_len)
            .filter(|r| r.kind == "decision")
            .collect();
        assert_eq!(
            new_decisions.len(),
            2,
            "S2-RO: retry: on-disk log must have EXACTLY 2 new decision rows \
             (N+1 from failed-save + N+2 from retry); got: {}",
            new_decisions.len()
        );

        // Both rows' payloads round-trip to the identical ReclassifyOutflow payload.
        let p0: EventPayload = serde_json::from_str(&new_decisions[0].payload_json).unwrap();
        let p1: EventPayload = serde_json::from_str(&new_decisions[1].payload_json).unwrap();
        assert_eq!(p0, p1, "S2-RO: both retry rows must have identical payload");
        assert!(
            matches!(
                &p0,
                EventPayload::ReclassifyOutflow(ro)
                    if matches!(&ro.as_, OutflowClass::Dispose { kind: DisposeKind::Sell })
            ),
            "S2-RO: payload must be ReclassifyOutflow(Sell); got: {:?}",
            p0
        );

        // The re-projected state must contain a DecisionConflict
        // attributed to the retry decision's EventId (FIRST-WINS).
        let retry_seq = new_decisions[1]
            .decision_seq
            .expect("retry decision must have decision_seq") as u64;
        let retry_id = btctax_core::EventId::Decision { seq: retry_seq };
        let snap_session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&snap_session).unwrap();
        let has_conflict = snap.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(&retry_id)
        });
        assert!(
            has_conflict,
            "S2-RO: re-projected state must contain DecisionConflict for retry decision {retry_id:?}"
        );
    }
}
