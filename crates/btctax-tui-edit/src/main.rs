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
    bulk_checked_totals, cycle_alloc_method, cycle_basis_source, cycle_business_optional,
    cycle_classify_raw_variant, cycle_filing_status, cycle_income_kind, cycle_income_kind_optional,
    cycle_outflow_kind, filter_optimize_candidates, income_kind_display, is_revocable_payload,
    next_focus, optimize_basis_label, prev_focus, validate, validate_classify_inbound_gift,
    validate_classify_inbound_income, validate_classify_inbound_self_transfer,
    validate_classify_raw_acquire, validate_classify_raw_income, validate_donation_details,
    validate_reclassify_income, validate_reclassify_outflow, validate_select_lots,
    validate_set_fmv, AllocLotRow, BulkLinkFlowState, BulkLinkModalState, BulkLinkRowItem,
    BulkLinkStep, ClassifyInboundFlowState, ClassifyInboundModalState, ClassifyInboundStep,
    ClassifyRawFlowState, ClassifyRawModalState, ClassifyRawStep, ClassifyRawVariant, ConflictItem,
    DisposalKind, DisposalListItem, DonationListItem, FieldBuffer, FmvListItem, InEventItem,
    InboundListItem, InboundVariant, IncomeListItem, LinkMode, LinkTransferFlowState,
    LinkTransferModalState, LinkTransferStep, LotPickFormRow, MutationModalState,
    OptimizeAcceptFlowState, OptimizeAcceptModalState, OptimizeAcceptStep, OptimizeCandidateItem,
    OutflowKind, OutflowListItem, ProfileFormState, RawListItem, ReclassifyIncomeFlowState,
    ReclassifyIncomeModalState, ReclassifyIncomeStep, ReclassifyOutflowFlowState,
    ReclassifyOutflowModalState, ReclassifyOutflowStep, ResolveConflictFlowState,
    ResolveConflictModalState, ResolveConflictStep, ResolveKind, SafeHarborAllocateFlowState,
    SafeHarborAllocateModalState, SafeHarborAllocateStep, SafeHarborAttestFlowState,
    SafeHarborAttestStep, SelectLotsFlowState, SelectLotsModalState, SelectLotsStep,
    SetDonationDetailsFlowState, SetDonationDetailsModalState, SetDonationDetailsStep,
    SetFmvFlowState, SetFmvModalState, SetFmvStep, TargetList, TransferOutItem, VoidFlowState,
    VoidListItem, VoidModalState, VoidStep, WalletItem, FREETEXT_CAP,
};
use editor::{EditorApp, EditorScreen};
use ratatui::{backend::CrosstermBackend, widgets::TableState, Terminal};
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use btctax_core::conventions::TRANSITION_DATE;
use btctax_core::{
    BasisSource, BlockerKind, ClassifyInbound, DisposeKind, DonationDetails, EventId, EventPayload,
    Form8283Section, InboundClass, IncomeKind, ManualFmv, OutflowClass, Persistability,
    ReclassifyIncome, RemovalKind, TransferTarget,
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
/// fall through to a quit arm mid-flow or mid-modal) [N4]:
/// 1. Mutation-modal dispatch — BEFORE flow, form and screen dispatch.
/// 2. Classify-inbound-modal dispatch — BEFORE flow, form and screen dispatch.
/// 3. Reclassify-outflow-modal dispatch — BEFORE flow, form and screen dispatch.
/// 4. Reclassify-income-modal dispatch — BEFORE flow, form and screen dispatch.
/// 5. Set-fmv-modal dispatch — BEFORE flow, form and screen dispatch.
/// 6. Void-modal dispatch — BEFORE flow, form and screen dispatch.
/// 7. Select-lots-modal dispatch — BEFORE flow, form and screen dispatch.
/// 8. Set-donation-details-modal dispatch — BEFORE flow, form and screen dispatch.
/// 9. Flow dispatch — ANY open flow claims ALL keys at every step [R0-I2].
///    The attest flow (incl. its TypedWord step — no separate attest modal) is
///    handled entirely here [R0-M4].
/// 10. Form dispatch — BEFORE screen dispatch.
/// 11. Screen dispatch (Unlock / Locked / Browse).
///
/// At most one flow `Some` and at most one modal `Some` at any time.
///
/// # Screen dispatch
/// - **Unlock**: `Esc` → quit; `Tab`/`BackTab` → ignored (no tab bar); `Enter` →
///   attempt open; `Backspace` → pop char; any `Char` → append to buffer.
/// - **Locked**: `r` → retry (back to Unlock); `q`/`Esc` → quit.
/// - **Browse**: `q`/`Esc` → quit; `Tab` → next tab; `BackTab` → prev tab;
///   `←/→` → year change + reset selections; `↑/↓ j/k` → scroll;
///   `PgUp/PgDn` → page; `g/G` → top/bottom; `p` → tax-profile form;
///   `c` → classify-inbound flow; `o` → reclassify-outflow flow;
///   `r` → reclassify-income flow; `f` → set-fmv flow; `v` → void flow.
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

    // ── 4. Reclassify-income-modal dispatch — BEFORE flow, form, screen ───────
    if app.reclassify_income_modal.is_some() {
        handle_reclassify_income_modal_key(app, key);
        return;
    }

    // ── 5. Set-fmv-modal dispatch — BEFORE flow, form, screen ────────────────
    if app.set_fmv_modal.is_some() {
        handle_set_fmv_modal_key(app, key);
        return;
    }

    // ── 6. Void-modal dispatch — BEFORE flow, form, screen ───────────────────
    if app.void_modal.is_some() {
        handle_void_modal_key(app, key);
        return;
    }

    // ── 7. Select-lots-modal dispatch — BEFORE flow, form, screen ────────────
    if app.select_lots_modal.is_some() {
        handle_select_lots_modal_key(app, key);
        return;
    }

    // ── 8. Set-donation-details-modal dispatch — BEFORE flow, form, screen ───
    if app.set_donation_details_modal.is_some() {
        handle_set_donation_details_modal_key(app, key);
        return;
    }

    // ── Link-transfer-modal dispatch — BEFORE flow, form, screen ─────────────
    if app.link_transfer_modal.is_some() {
        handle_link_transfer_modal_key(app, key);
        return;
    }

    // ── Classify-raw-modal dispatch — BEFORE flow, form, screen ──────────────
    if app.classify_raw_modal.is_some() {
        handle_classify_raw_modal_key(app, key);
        return;
    }

    // ── Resolve-conflict-modal dispatch — BEFORE flow, form, screen ──────────
    if app.resolve_conflict_modal.is_some() {
        handle_resolve_conflict_modal_key(app, key);
        return;
    }

    // ── Optimize-accept-modal dispatch — BEFORE flow, form, screen ───────────
    if app.optimize_accept_modal.is_some() {
        handle_optimize_accept_modal_key(app, key);
        return;
    }

    // ── Safe-harbor-allocate-modal dispatch — BEFORE flow, form, screen ──────
    if app.safe_harbor_allocate_modal.is_some() {
        handle_safe_harbor_allocate_modal_key(app, key);
        return;
    }

    // ── Bulk-link-transfer-modal dispatch — BEFORE flow, form, screen ────────
    if app.bulk_link_modal.is_some() {
        handle_bulk_link_modal_key(app, key);
        return;
    }

    // ── 9. Flow dispatch — the FLOW Option (not the step) is the guard [R0-I2] ─
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
    if app.reclassify_income_flow.is_some() {
        handle_reclassify_income_flow_key(app, key);
        return;
    }
    if app.set_fmv_flow.is_some() {
        handle_set_fmv_flow_key(app, key);
        return;
    }
    if app.void_flow.is_some() {
        handle_void_flow_key(app, key);
        return;
    }
    if app.select_lots_flow.is_some() {
        handle_select_lots_flow_key(app, key);
        return;
    }
    if app.set_donation_details_flow.is_some() {
        handle_set_donation_details_flow_key(app, key);
        return;
    }
    if app.link_transfer_flow.is_some() {
        handle_link_transfer_flow_key(app, key);
        return;
    }
    if app.classify_raw_flow.is_some() {
        handle_classify_raw_flow_key(app, key);
        return;
    }
    if app.safe_harbor_allocate_flow.is_some() {
        handle_safe_harbor_allocate_flow_key(app, key);
        return;
    }
    if app.bulk_link_flow.is_some() {
        handle_bulk_link_flow_key(app, key);
        return;
    }
    if app.safe_harbor_attest_flow.is_some() {
        handle_safe_harbor_attest_flow_key(app, key);
        return;
    }
    if app.resolve_conflict_flow.is_some() {
        handle_resolve_conflict_flow_key(app, key);
        return;
    }
    if app.optimize_accept_flow.is_some() {
        handle_optimize_accept_flow_key(app, key);
        return;
    }

    // ── 10. Form dispatch — BEFORE screen dispatch ────────────────────────────
    if app.profile_form.is_some() {
        handle_form_key(app, key);
        return;
    }

    // ── 9. Screen dispatch ────────────────────────────────────────────────────
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
                KeyCode::Char('r') => open_reclassify_income_flow(app),
                KeyCode::Char('f') => open_set_fmv_flow(app),
                KeyCode::Char('v') => open_void_flow(app),
                KeyCode::Char('s') => open_select_lots_flow(app),
                KeyCode::Char('d') => open_set_donation_details_flow(app),
                KeyCode::Char('l') => open_link_transfer_flow(app),
                KeyCode::Char('u') => open_classify_raw_flow(app),
                KeyCode::Char('a') => open_safe_harbor_attest_flow(app),
                KeyCode::Char('A') => open_safe_harbor_allocate_flow(app),
                KeyCode::Char('b') => open_bulk_link_transfer_flow(app),
                KeyCode::Char('i') => open_resolve_conflict_flow(app),
                KeyCode::Char('z') => open_optimize_accept_flow(app),
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
                    app.on_persist_error(e);
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

impl EditorApp {
    /// The residue-latch status, if any mutating opener must refuse. `attest_save_failed` keeps its
    /// exact shipped wording (so `kat_e2e_attest_errlatch_chmod` stays green); `rollback_failed`
    /// reports the unrevertable-residue remedy. `None` when neither latch is set. [save-rollback]
    fn residue_latch_status(&self) -> Option<String> {
        if self.attest_save_failed {
            Some(
                "A failed attest save left unsaved decisions in memory — quit the editor \
                 (the unsaved attestation is discarded on quit), then retry via CLI: \
                 btctax reconcile safe-harbor-attest"
                    .to_string(),
            )
        } else if self.rollback_failed {
            Some(
                "CRITICAL: a save failed and could not be reverted — unsaved data is in memory. \
                 Quit the editor NOW (the vault on disk is unchanged); no in-editor action will \
                 save until you quit, then re-run the operation via the CLI."
                    .to_string(),
            )
        } else {
            None
        }
    }

    /// The SINGLE site that maps a `PersistError` to its editor effect [R0-I1]. Every rollback-flow
    /// Enter arm delegates here (after closing its own modal). `NoChange`/`RolledBack` → benign
    /// keep-open status (nothing persisted; safe to retry). `ResidueLive` → arm the `rollback_failed`
    /// latch, show the CRITICAL status, and close every mutation surface. `PersistError` has no
    /// `Display`, so a lazy `{e}` cannot bypass the `ResidueLive` arm.
    fn on_persist_error(&mut self, e: edit::persist::PersistError) {
        use edit::persist::PersistError::{NoChange, ResidueLive, RolledBack};
        match e {
            NoChange(err) | RolledBack(err) => {
                self.status = Some(format!(
                    "Save error: {err} — no changes were recorded; safe to retry."
                ));
            }
            ResidueLive(err) => {
                self.rollback_failed = true;
                self.status = Some(format!(
                    "CRITICAL: a save failed and could not be reverted ({err}) — unsaved data is in \
                     memory. Quit the editor NOW (the vault on disk is unchanged); no in-editor \
                     action will save until you quit, then re-run the operation via the CLI."
                ));
                self.close_all_mutation_surfaces();
            }
        }
    }

    /// Close every mutating flow/modal (at most one of each is ever open). Used by the `ResidueLive`
    /// arm so no open flow can trigger a further save while the residue latch is up.
    fn close_all_mutation_surfaces(&mut self) {
        self.profile_form = None;
        self.mutation_modal = None;
        self.classify_inbound_flow = None;
        self.classify_inbound_modal = None;
        self.reclassify_outflow_flow = None;
        self.reclassify_outflow_modal = None;
        self.reclassify_income_flow = None;
        self.reclassify_income_modal = None;
        self.set_fmv_flow = None;
        self.set_fmv_modal = None;
        self.void_flow = None;
        self.void_modal = None;
        self.select_lots_flow = None;
        self.select_lots_modal = None;
        self.set_donation_details_flow = None;
        self.set_donation_details_modal = None;
        self.link_transfer_flow = None;
        self.link_transfer_modal = None;
        self.classify_raw_flow = None;
        self.classify_raw_modal = None;
        self.safe_harbor_attest_flow = None;
        self.resolve_conflict_flow = None;
        self.resolve_conflict_modal = None;
        self.optimize_accept_flow = None;
        self.optimize_accept_modal = None;
        self.safe_harbor_allocate_flow = None;
        self.safe_harbor_allocate_modal = None;
        self.bulk_link_flow = None;
        self.bulk_link_modal = None;
    }
}

/// Open the tax-profile form for `selected_year`, pre-populated from the snapshot.
///
/// Pre-population (the `--show` equivalent): if `snapshot.profiles.get(&year)` is
/// `Some(p)`, every buffer is filled with the field's `Display` string and
/// `filing_status` is set from `p`.  Otherwise: `filing_status = Single`, all
/// buffers empty (required fields must be typed; optional empties → $0 at validation).
fn open_profile_form(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
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
                    app.on_persist_error(e);
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
        Some(ClassifyInboundStep::SelfTransferForm { .. }) => 4,
        None => return,
    };
    match step_kind {
        0 => handle_ci_list_key(app, key),
        1 => handle_ci_picker_key(app, key),
        2 => handle_ci_income_form_key(app, key),
        3 => handle_ci_gift_form_key(app, key),
        4 => handle_ci_self_transfer_form_key(app, key),
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
                        InboundVariant::GiftReceived => InboundVariant::SelfTransferMine,
                        InboundVariant::SelfTransferMine => InboundVariant::Income,
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
                    InboundVariant::SelfTransferMine => ClassifyInboundStep::SelfTransferForm {
                        item,
                        basis_buf: FieldBuffer::new(),
                        acquired_buf: FieldBuffer::new(),
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

/// SelfTransferForm step (Cycle A): two OPTIONAL text fields — basis (focus 0), acquired_at (focus 1).
/// Mirrors the gift-form handler; Enter validates → opens the (reused) confirmation modal; Esc → picker.
fn handle_ci_self_transfer_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let result = {
                match app.classify_inbound_flow.as_ref() {
                    Some(f) => match &f.step {
                        ClassifyInboundStep::SelfTransferForm {
                            item,
                            basis_buf,
                            acquired_buf,
                            ..
                        } => validate_classify_inbound_self_transfer(basis_buf, acquired_buf)
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
                        if let ClassifyInboundStep::SelfTransferForm { error, .. } = &mut flow.step
                        {
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
                    ClassifyInboundStep::SelfTransferForm { item, .. } => item.clone(),
                    _ => return,
                },
                None => return,
            };
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                flow.step = ClassifyInboundStep::VariantPicker {
                    item,
                    variant: InboundVariant::SelfTransferMine,
                };
            }
        }
        KeyCode::Tab | KeyCode::Down => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::SelfTransferForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(1);
                }
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::SelfTransferForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::SelfTransferForm {
                    basis_buf,
                    acquired_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => basis_buf.pop_char(),
                        1 => acquired_buf.pop_char(),
                        _ => {}
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                if let ClassifyInboundStep::SelfTransferForm {
                    basis_buf,
                    acquired_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => basis_buf.push_char(c),
                        1 => acquired_buf.push_char(c),
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
                    app.on_persist_error(e);
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

// ── Reclassify-income modal handler ──────────────────────────────────────────

fn handle_reclassify_income_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let modal_data = match app.reclassify_income_modal.as_ref() {
                Some(m) => {
                    let payload = EventPayload::ReclassifyIncome(ReclassifyIncome {
                        income_event: m.target_event.clone(),
                        business: m.new_business,
                        kind: m.new_kind,
                    });
                    let target_event = m.target_event.clone();
                    let new_business = m.new_business;
                    let new_kind = m.new_kind;
                    (payload, target_event, new_business, new_kind)
                }
                None => return,
            };
            let (payload, target_event, new_business, new_kind) = modal_data;

            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.reclassify_income_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_reclassify_income(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_reclassify_income_status(
                                &snap,
                                &target_event,
                                &decision_id,
                                new_business,
                                new_kind,
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
                    app.reclassify_income_modal = None;
                    app.reclassify_income_flow = None;
                }
                Err(e) => {
                    app.reclassify_income_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.reclassify_income_modal = None;
        }
        _ => {}
    }
}

// ── Set-fmv modal handler ─────────────────────────────────────────────────────

fn handle_set_fmv_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let modal_data = match app.set_fmv_modal.as_ref() {
                Some(m) => {
                    let payload = EventPayload::ManualFmv(ManualFmv {
                        event: m.target_event.clone(),
                        usd_fmv: m.usd_fmv,
                    });
                    let target_event = m.target_event.clone();
                    let usd_fmv = m.usd_fmv;
                    (payload, target_event, usd_fmv)
                }
                None => return,
            };
            let (payload, target_event, usd_fmv) = modal_data;

            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.set_fmv_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_set_fmv(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status =
                                derive_set_fmv_status(&snap, &target_event, &decision_id, usd_fmv);
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.set_fmv_modal = None;
                    app.set_fmv_flow = None;
                }
                Err(e) => {
                    app.set_fmv_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.set_fmv_modal = None;
        }
        _ => {}
    }
}

// ── Reclassify-income flow key dispatch ──────────────────────────────────────

fn handle_reclassify_income_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step_kind: u8 = match app.reclassify_income_flow.as_ref().map(|f| &f.step) {
        Some(ReclassifyIncomeStep::List) => 0,
        Some(ReclassifyIncomeStep::FieldForm { .. }) => 1,
        None => return,
    };
    match step_kind {
        0 => handle_ri_list_key(app, key),
        1 => handle_ri_field_form_key(app, key),
        _ => {}
    }
}

fn handle_ri_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .reclassify_income_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                if let Some(flow) = app.reclassify_income_flow.as_mut() {
                    flow.step = ReclassifyIncomeStep::FieldForm {
                        item,
                        business: None,
                        kind: None,
                        focus: 0,
                        error: None,
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.reclassify_income_flow = None;
        }
        _ => {}
    }
}

fn handle_ri_field_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let result = {
                match app.reclassify_income_flow.as_ref() {
                    Some(f) => match &f.step {
                        ReclassifyIncomeStep::FieldForm {
                            item,
                            business,
                            kind,
                            ..
                        } => validate_reclassify_income(item, *business, *kind)
                            .map(|_| (item.clone(), *business, *kind)),
                        _ => return,
                    },
                    None => return,
                }
            };
            match result {
                Ok((item, business, kind)) => {
                    app.reclassify_income_modal = Some(ReclassifyIncomeModalState {
                        target_event: item.income_event.clone(),
                        target_date: item.date,
                        target_sat: item.sat,
                        original_kind: item.kind,
                        original_business: item.business,
                        new_business: business.unwrap_or(false),
                        new_kind: kind,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.reclassify_income_flow.as_mut() {
                        if let ReclassifyIncomeStep::FieldForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                flow.step = ReclassifyIncomeStep::List;
            }
        }
        KeyCode::Tab => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                if let ReclassifyIncomeStep::FieldForm {
                    business,
                    kind,
                    focus,
                    ..
                } = &mut flow.step
                {
                    if *focus == 0 {
                        *business = cycle_business_optional(*business);
                    } else {
                        *kind = cycle_income_kind_optional(*kind);
                    }
                }
            }
        }
        KeyCode::Up => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                if let ReclassifyIncomeStep::FieldForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Down => {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                if let ReclassifyIncomeStep::FieldForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(1);
                }
            }
        }
        _ => {}
    }
}

// ── Set-fmv flow key dispatch ─────────────────────────────────────────────────

fn handle_set_fmv_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step_kind: u8 = match app.set_fmv_flow.as_ref().map(|f| &f.step) {
        Some(SetFmvStep::List) => 0,
        Some(SetFmvStep::FieldForm { .. }) => 1,
        None => return,
    };
    match step_kind {
        0 => handle_sfmv_list_key(app, key),
        1 => handle_sfmv_field_form_key(app, key),
        _ => {}
    }
}

fn handle_sfmv_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .set_fmv_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                if let Some(flow) = app.set_fmv_flow.as_mut() {
                    flow.step = SetFmvStep::FieldForm {
                        item,
                        usd_fmv_buf: FieldBuffer::new(),
                        error: None,
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.set_fmv_flow = None;
        }
        _ => {}
    }
}

fn handle_sfmv_field_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let result = {
                match app.set_fmv_flow.as_ref() {
                    Some(f) => match &f.step {
                        SetFmvStep::FieldForm {
                            item, usd_fmv_buf, ..
                        } => validate_set_fmv(item, usd_fmv_buf)
                            .map(|payload| (item.clone(), payload)),
                        _ => return,
                    },
                    None => return,
                }
            };
            match result {
                Ok((item, payload)) => {
                    let usd_fmv = match &payload {
                        EventPayload::ManualFmv(mf) => mf.usd_fmv,
                        _ => unreachable!(),
                    };
                    app.set_fmv_modal = Some(SetFmvModalState {
                        target_event: item.event.clone(),
                        target_date: item.date,
                        target_sat: item.sat,
                        target_kind: item.kind,
                        usd_fmv,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.set_fmv_flow.as_mut() {
                        if let SetFmvStep::FieldForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                flow.step = SetFmvStep::List;
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                if let SetFmvStep::FieldForm { usd_fmv_buf, .. } = &mut flow.step {
                    usd_fmv_buf.pop_char();
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                if let SetFmvStep::FieldForm { usd_fmv_buf, .. } = &mut flow.step {
                    usd_fmv_buf.push_char(c);
                }
            }
        }
        _ => {}
    }
}

// ── Void flow handlers ────────────────────────────────────────────────────────

/// Handle a key press while the void confirmation modal is open.
///
/// Enter-arm semantics [M1]:
/// - `Ok(id)` → re-project + derive status + close modal + close flow.
/// - `Err(e)` → close modal, flow stays at List, status "Save error: {e}".
///   (Void has no FieldForm, so "keep FieldForm open" does not apply.)
///
/// Esc → close modal only (back to List step; flow stays open).
fn handle_void_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Extract data from the modal before dropping the borrow.
            let (target_event_id, seq, payload_tag, inner_target) = match app.void_modal.as_ref() {
                Some(m) => (
                    m.target_event_id.clone(),
                    m.seq,
                    m.payload_tag,
                    m.inner_target.clone(),
                ),
                None => return,
            };

            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.void_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_void(session, target_event_id.clone(), now)
            };

            match save_result {
                Ok(void_decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_void_status(
                                &snap,
                                &void_decision_id,
                                &target_event_id,
                                inner_target.as_ref(),
                                payload_tag,
                                seq,
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
                    app.void_modal = None;
                    app.void_flow = None;
                }
                Err(e) => {
                    // [M1] On save error: close modal, flow stays at List.
                    app.void_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal → back to List step (flow stays open).
            app.void_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal — 'q' must NOT quit here).
        }
    }
}

/// Dispatch a key press to the void flow step handler.
///
/// The FLOW OPTION (not the step) is the guard: when we reach this function the
/// void_flow is always `Some`. VoidStep has only one variant (List).
fn handle_void_flow_key(app: &mut EditorApp, key: KeyEvent) {
    handle_void_list_key(app, key);
}

/// Handle keys at the void flow's List step.
///
/// Enter → open `VoidModalState` DIRECTLY (no FieldForm — spec D3.1).
/// Esc → close flow (back to Browse).
/// q → SWALLOWED (flow is blocking).
fn handle_void_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.void_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.void_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.void_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.void_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            // Transition DIRECTLY to modal (no FieldForm step).
            let selected = app
                .void_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                let is_safe_harbor = item.payload_tag == "SafeHarborAllocation";
                app.void_modal = Some(VoidModalState {
                    target_event_id: item.event_id.clone(),
                    seq: item.seq,
                    payload_tag: item.payload_tag,
                    target_summary: item.target_summary.clone(),
                    inner_target: item.inner_target.clone(),
                    is_safe_harbor,
                });
            }
        }
        KeyCode::Esc => {
            // Close flow; nothing written.
            app.void_flow = None;
        }
        KeyCode::Char('q') => {
            // SWALLOWED: flow is blocking; 'q' must NOT quit while a flow is open.
        }
        _ => {}
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
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
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
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
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
            // WB-I1 fix: canonical() already includes the "decision|" prefix, so format
            // as "{}" (not "decision|{}") to avoid the double-prefix "decision|decision|N".
            return format!(
                "Saved, but DecisionConflict fired on this decision — see Compliance; \
                 clear with Void flow (press 'v'), or quit the editor and run: btctax reconcile void {}",
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
                 to supply the FMV, void this decision (Void flow: press 'v'; or quit the \
                 editor and run: btctax reconcile void decision|{seq}) and re-classify with an FMV",
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
                 void this decision (Void flow: press 'v'; or quit the editor and run: btctax reconcile void \
                 decision|{seq}) and re-classify with donor basis or a donor date covered \
                 by the price dataset"
            );
        }
    }

    // No target-attributed blocker: clean success.
    let cls_desc = match as_ {
        InboundClass::Income { kind, .. } => {
            format!("Income({})", income_kind_display(*kind))
        }
        InboundClass::GiftReceived { .. } => "GiftReceived".to_string(),
        InboundClass::SelfTransferMine { .. } => "SelfTransferMine".to_string(),
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
            // WB-I1 fix: canonical() already includes the "decision|" prefix, so format
            // as "{}" (not "decision|{}") to avoid the double-prefix "decision|decision|N".
            return format!(
                "Saved, but DecisionConflict fired on this decision — see Compliance; \
                 clear with Void flow (press 'v'), or quit the editor and run: btctax reconcile void {}",
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

// ── Reclassify-income flow opener ─────────────────────────────────────────────

/// Open the reclassify-income flow from the Browse screen.
///
/// Applies the compound pre-filter (spec §Pre-filter verification, Claim C):
/// 1. Raw `EventPayload::Income` events only [WB-I4(a): raw only — ClassifyRaw'd
///    Unclassified events whose effective payload became Income are excluded;
///    under-inclusion only (safe direction); recorded in FOLLOWUPS].
/// 2. No non-voided `ReclassifyIncome` decision already targets it (a second
///    would fire Hard `DecisionConflict`; FIRST-WINS, resolve.rs pass-1e).
///
/// Display data derives date/sat/kind/business from the Income payload directly
/// (pre-override — the filter excludes already-reclassified events), enriched
/// with the `income_recognized` entry for fmv when present. `FmvMissing` events
/// (no income_recognized entry) render fmv as `"(pending)"`.
///
/// Empty filtered list → status "No reclassifiable income events"; flow NOT
/// opened [R0-M8].
fn open_reclassify_income_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    // [N3] Both sets are HOISTED once before the per-event filter (the 2a
    // precedent: `open_classify_inbound_flow`) — never rebuilt inside the closure.

    // Voided-decision set (VoidDecisionEvent targets).
    let voided: BTreeSet<&EventId> = snap
        .events
        .iter()
        .filter_map(|ev| {
            if let EventPayload::VoidDecisionEvent(v) = &ev.payload {
                Some(&v.target_event_id)
            } else {
                None
            }
        })
        .collect();

    // Income EventIds already targeted by a non-voided ReclassifyIncome
    // (a second would fire DecisionConflict; FIRST-WINS).
    let already_reclassified: BTreeSet<&EventId> = snap
        .events
        .iter()
        .filter(|ev| !voided.contains(&ev.id))
        .filter_map(|ev| {
            if let EventPayload::ReclassifyIncome(ri) = &ev.payload {
                Some(&ri.income_event)
            } else {
                None
            }
        })
        .collect();

    let mut items: Vec<IncomeListItem> = snap
        .events
        .iter()
        .filter_map(|e| {
            // Filter 1: raw Income payload only.
            let inc = match &e.payload {
                EventPayload::Income(inc) => inc,
                _ => return None,
            };
            // Filter 2: exclude if a non-voided ReclassifyIncome already targets it.
            if already_reclassified.contains(&e.id) {
                return None;
            }
            // FMV from income_recognized if present (clean FMV); None → "(pending)".
            let fmv = snap
                .state
                .income_recognized
                .iter()
                .find(|r| r.event == e.id)
                .map(|r| r.usd_fmv);
            let date = btctax_core::conventions::tax_date(e.utc_timestamp, e.original_tz);
            Some(IncomeListItem {
                income_event: e.id.clone(),
                date,
                sat: inc.sat,
                kind: inc.kind,
                business: inc.business,
                fmv,
                wallet: e.wallet.clone(),
            })
        })
        .collect();

    // Sort by date for deterministic display order.
    items.sort_by_key(|i| i.date);

    if items.is_empty() {
        // R0-M8: empty filtered list never opens a flow.
        app.status = Some("No reclassifiable income events".to_string());
        return;
    }

    app.reclassify_income_flow = Some(ReclassifyIncomeFlowState {
        list: TargetList::new(items),
        step: ReclassifyIncomeStep::List,
    });
}

// ── Set-FMV flow opener ───────────────────────────────────────────────────────

/// Open the set-fmv flow from the Browse screen.
///
/// Applies the filter from spec Claim D:
/// 1. `FmvMissing` blockers only.
/// 2. Blocker.event resolves to a raw `EventPayload::Income` event in snap.events
///    (ManualFmv pass-1d validates EFFECTIVE payload == Income; the raw filter
///    approximates this — same WB-I4(a) limitation as 2a. A TransferIn classified
///    as Income via ClassifyInbound is excluded — correct, because ManualFmv on a
///    TransferIn fires DecisionConflict; the remedy is void + re-classify).
///
/// No pre-filter for already-set FMVs: the list naturally empties when the
/// `FmvMissing` blocker clears after a successful persist + re-projection; a
/// second `ManualFmv` is NOT a conflict (latest-wins, resolve.rs:453–456).
///
/// Empty filtered list → status "No FMV-missing income events"; flow NOT
/// opened [R0-M8].
fn open_set_fmv_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

    let mut items: Vec<FmvListItem> = snap
        .state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::FmvMissing)
        .filter_map(|b| {
            let id = b.event.as_ref()?;
            // Raw Income payload check [WB-I4(a): raw only].
            let ev = ev_idx.get(id)?;
            let inc = match &ev.payload {
                EventPayload::Income(inc) => inc,
                _ => return None,
            };
            let date = btctax_core::conventions::tax_date(ev.utc_timestamp, ev.original_tz);
            Some(FmvListItem {
                event: id.clone(),
                date,
                sat: inc.sat,
                kind: inc.kind,
                wallet: ev.wallet.clone(),
            })
        })
        .collect();

    // Sort by date for deterministic display order.
    items.sort_by_key(|i| i.date);

    if items.is_empty() {
        // R0-M8: empty filtered list never opens a flow.
        app.status = Some("No FMV-missing income events".to_string());
        return;
    }

    app.set_fmv_flow = Some(SetFmvFlowState {
        list: TargetList::new(items),
        step: SetFmvStep::List,
    });
}

// ── Status derivers for reclassify-income and set-fmv ─────────────────────────

fn derive_reclassify_income_status(
    snap: &btctax_tui::app::Snapshot,
    _target_event: &EventId,
    decision_id: &EventId,
    new_business: bool,
    new_kind: Option<IncomeKind>,
) -> String {
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired on this decision — see Compliance; \
                 clear with Void flow (press 'v'), or quit the editor and run: btctax reconcile void {}",
                decision_id.canonical()
            );
        }
    }

    let effective_kind = new_kind.map(income_kind_display).unwrap_or("original");
    format!("Reclassified income: business={new_business}, kind={effective_kind}")
}

fn derive_set_fmv_status(
    snap: &btctax_tui::app::Snapshot,
    target_event: &EventId,
    decision_id: &EventId,
    usd_fmv: btctax_core::Usd,
) -> String {
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(target_event) {
            return format!(
                "FMV set but FmvMissing re-fired for this event — see Compliance; \
                 blocker detail: {}",
                b.detail
            );
        }
    }
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired on this decision — see Compliance; \
                 clear with Void flow (press 'v'), or quit the editor and run: btctax reconcile void {}",
                decision_id.canonical()
            );
        }
    }

    format!(
        "FMV set: {usd_fmv} for {} — FmvMissing blocker cleared",
        target_event.canonical()
    )
}

// ── Void flow opener ──────────────────────────────────────────────────────────

/// Compute the payload tag, target summary, inner_target, and is_safe_harbor flag
/// for a void list item from its payload (spec D3.1).
fn summarize_void_payload(payload: &EventPayload) -> (&'static str, String, Option<EventId>, bool) {
    use btctax_core::EventPayload;
    match payload {
        EventPayload::TransferLink(tl) => (
            "TransferLink",
            format!("out \u{2192} {}", tl.out_event.canonical()),
            Some(tl.out_event.clone()),
            false,
        ),
        EventPayload::ReclassifyOutflow(ro) => (
            "ReclassifyOutflow",
            format!("out {} as {:?}", ro.transfer_out_event.canonical(), ro.as_),
            Some(ro.transfer_out_event.clone()),
            false,
        ),
        EventPayload::ClassifyInbound(ci) => (
            "ClassifyInbound",
            format!("in {} as {:?}", ci.transfer_in_event.canonical(), ci.as_),
            Some(ci.transfer_in_event.clone()),
            false,
        ),
        EventPayload::ManualFmv(m) => (
            "ManualFmv",
            format!("fmv={} for {}", m.usd_fmv, m.event.canonical()),
            Some(m.event.clone()),
            false,
        ),
        EventPayload::ClassifyRaw(cr) => (
            "ClassifyRaw",
            format!("raw {}", cr.target.canonical()),
            Some(cr.target.clone()),
            false,
        ),
        EventPayload::MethodElection(me) => (
            "MethodElection",
            format!("method={:?} from {}", me.method, me.effective_from),
            None,
            false,
        ),
        EventPayload::LotSelection(ls) => (
            "LotSelection",
            format!("lots for {}", ls.disposal_event.canonical()),
            Some(ls.disposal_event.clone()),
            false,
        ),
        EventPayload::ReclassifyIncome(ri) => (
            "ReclassifyIncome",
            format!("income {} biz={}", ri.income_event.canonical(), ri.business),
            Some(ri.income_event.clone()),
            false,
        ),
        EventPayload::SafeHarborAllocation(a) => (
            "SafeHarborAllocation",
            format!("alloc {} lots as_of {}", a.lots.len(), a.as_of_date),
            None,
            true,
        ),
        _ => ("?", "?".to_string(), None, false),
    }
}

/// Open the void flow from the Browse screen.
///
/// Applies the filter from spec Claim E:
/// 1. Decision EventId only (EventId::Decision { .. }).
/// 2. Not already voided (target of an existing VoidDecisionEvent).
/// 3. Revocable payload (is_revocable_payload).
///
/// Non-revocable types (SupersedeImport, RejectImport, VoidDecisionEvent) are excluded.
/// Already-voided decisions are excluded for UX cleanliness (not conflict-prevention).
///
/// Empty filtered list → status "No revocable decisions to void"; flow NOT opened [R0-M8].
fn open_void_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    // Build voided set (IDs targeted by any VoidDecisionEvent).
    let voided: std::collections::BTreeSet<EventId> = snap
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

    // #7: exclude EFFECTIVE SafeHarborAllocations (irrevocable, §7.4). A confirmed void of an
    // effective allocation writes a permanent VoidDecisionEvent the engine rejects with
    // DecisionConflict (a damaging no-op). "Effective" = a non-voided SafeHarborAllocation on
    // whose id NEITHER SafeHarborTimebar NOR SafeHarborUnconservable fired (resolve.rs:865-921).
    // Inert allocations (timebarred OR unconservable) STAY voidable — voiding them applies
    // cleanly (transition.rs:403) — so they remain listed.
    let effective_alloc = |e: &btctax_core::LedgerEvent| {
        matches!(e.payload, EventPayload::SafeHarborAllocation(_)) && {
            let has = |k| {
                snap.state
                    .blockers
                    .iter()
                    .any(|b| b.kind == k && b.event.as_ref() == Some(&e.id))
            };
            !has(BlockerKind::SafeHarborTimebar) && !has(BlockerKind::SafeHarborUnconservable)
        }
    };

    let mut items: Vec<VoidListItem> = snap
        .events
        .iter()
        .filter(|e| matches!(e.id, EventId::Decision { .. }))
        .filter(|e| !voided.contains(&e.id))
        .filter(|e| is_revocable_payload(&e.payload))
        .filter(|e| !effective_alloc(e))
        .map(|e| {
            let seq = match &e.id {
                EventId::Decision { seq } => *seq,
                _ => 0,
            };
            let (payload_tag, target_summary, inner_target, _is_sha) =
                summarize_void_payload(&e.payload);
            VoidListItem {
                event_id: e.id.clone(),
                seq,
                payload_tag,
                target_summary,
                inner_target,
            }
        })
        .collect();

    // Sort by seq for deterministic display.
    items.sort_by_key(|i| i.seq);

    if items.is_empty() {
        app.status = Some("No revocable decisions to void".to_string());
        return;
    }

    app.void_flow = Some(VoidFlowState {
        list: TargetList::new(items),
        step: VoidStep::List,
    });
}

// ── Status deriver for void ───────────────────────────────────────────────────

/// Derive the status string from RE-PROJECTED blockers after a void save.
///
/// Three arms (spec D3.3):
/// 1. DecisionConflict attributed to `void_decision_id` → void REJECTED (e.g. effective
///    SafeHarborAllocation). Status must NOT start with "Voided" [I2].
/// 2. Returned-blocker attributed to `inner_target` → surfaced concretely [M5].
/// 3. Clean → generic "effects un-projected" message.
///
/// Cascade conflicts are NOT detected here — they are attributed to the ORPHANED
/// decision's id, not the void's. This is the deliberate surfacing limit (D3.1 [I1]).
fn derive_void_status(
    snap: &btctax_tui::app::Snapshot,
    void_decision_id: &EventId,
    _target_event_id: &EventId,
    inner_target: Option<&EventId>,
    payload_tag: &'static str,
    seq: u64,
) -> String {
    // Arm 1: conflict attributed to the void decision (void REJECTED).
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(void_decision_id) {
            return "Void saved, but DecisionConflict fired — the target decision \
                    remains in force (see Compliance)"
                .to_string();
        }
    }

    // Arm 2: returned blocker attributed to the inner target.
    if let Some(inner) = inner_target {
        for b in &snap.state.blockers {
            if b.event.as_ref() == Some(inner) {
                let blocker_kind = format!("{:?}", b.kind);
                return format!(
                    "Voided {payload_tag} decision|{seq} — {blocker_kind} returned for \
                     {} (see Compliance)",
                    inner.canonical()
                );
            }
        }
    }

    // Arm 3: clean.
    format!(
        "Voided {payload_tag} decision|{seq} — effects un-projected; \
         check Compliance for any returned blockers"
    )
}

// ── Select-lots flow: modal handler ──────────────────────────────────────────

/// Handle a key press while the select-lots confirmation modal is open.
///
/// Enter-arm (spec D1):
///   `Ok(id)` → re-project + `derive_select_lots_status` + close modal + close flow.
///   `Err(e)` → close modal, keep LotsForm open (buffers intact), status `"Save error: {e}"`.
///
/// Esc → close modal only (back to LotsForm; nothing written).
fn handle_select_lots_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Extract the validated payload from the modal before dropping the borrow.
            // (disposal_date/kind/principal are display-only modal fields — not needed here.)
            let (disposal_event, picks, pick_count, total_sat) =
                match app.select_lots_modal.as_ref() {
                    Some(m) => (
                        m.disposal_event.clone(),
                        m.picks.clone(),
                        m.pick_count,
                        m.total_sat,
                    ),
                    None => return,
                };

            let payload =
                btctax_core::EventPayload::LotSelection(btctax_core::event::LotSelection {
                    disposal_event: disposal_event.clone(),
                    lots: picks,
                });
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.select_lots_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_select_lots(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_select_lots_status(
                                &snap,
                                &disposal_event,
                                &decision_id,
                                pick_count,
                                total_sat,
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
                    app.select_lots_modal = None;
                    app.select_lots_flow = None;
                }
                Err(e) => {
                    // [M1] On save error: close modal, keep LotsForm open (buffers intact).
                    app.select_lots_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal → back to LotsForm (nothing written).
            app.select_lots_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal — 'q' must NOT quit here).
        }
    }
}

// ── Select-lots flow: flow key dispatcher ────────────────────────────────────

/// Dispatch to the correct sub-handler depending on `SelectLotsStep`.
fn handle_select_lots_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.select_lots_flow.as_ref() {
        Some(f) => match &f.step {
            SelectLotsStep::List => 0u8,
            SelectLotsStep::LotsForm { .. } => 1u8,
        },
        None => return,
    };
    match step {
        0 => handle_sl_list_key(app, key),
        _ => handle_sl_lots_form_key(app, key),
    }
}

/// Handle keys at the select-lots flow's List step.
///
/// Enter → transition to LotsForm (build lot rows from snap.state.lots).
/// Esc → close flow (back to Browse).
/// q → SWALLOWED (flow is blocking).
fn handle_sl_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            // Transition to LotsForm: clone selected item, build lot rows.
            let selected = app
                .select_lots_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            let snap = match app.snapshot.as_ref() {
                Some(s) => s,
                None => return,
            };

            if let Some(item) = selected {
                // Build lot rows and sort by acquired_at ASC.
                //
                // #2: feasibility-honest candidate-lot gate [R0-I1]. A PRE-2025 disposal consumes
                // from the (pre-boundary) Universal residue, so offer pre-2025 lots ACROSS wallets,
                // but EXCLUDE Path-B `SafeHarborAllocated` seed lots — their lot_ids never existed
                // in Universal, so the engine raises a hard LotSelectionInvalid (no method-order
                // fallback). Path-A `ReconstructedPerWallet` lots preserve their Universal lot_ids
                // and are feasible. Post-2025 disposals stay per-wallet (unchanged).
                let wallet_ref = item.wallet.as_ref();
                let rows: Vec<LotPickFormRow> = snap
                    .state
                    .lots
                    .iter()
                    .filter(|l| {
                        if item.date < TRANSITION_DATE {
                            l.acquired_at < TRANSITION_DATE
                                && l.basis_source != BasisSource::SafeHarborAllocated
                        } else {
                            wallet_ref.is_some_and(|w| &l.wallet == w)
                        }
                    })
                    .map(|l| LotPickFormRow {
                        lot_id: l.lot_id.clone(),
                        remaining_sat: l.remaining_sat,
                        acquired_at: l.acquired_at,
                        usd_basis: l.usd_basis,
                        pick_sat_buf: FieldBuffer::new(),
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .collect();

                // Sort by acquired_at ASC (oldest first — Specific-Id natural display order).
                let mut sorted_rows = rows;
                sorted_rows.sort_by_key(|r| r.acquired_at);

                if sorted_rows.is_empty() {
                    let wallet_str = match &item.wallet {
                        Some(w) => format!("{w:?}"),
                        None => "(no wallet)".to_string(),
                    };
                    // No per-step error on the List step — surface via the global status
                    // and stay on List (the flow's step is left unchanged).
                    app.status = Some(format!(
                        "No lots available for wallet {wallet_str}; check Holdings"
                    ));
                    return;
                }

                if let Some(flow) = app.select_lots_flow.as_mut() {
                    flow.step = SelectLotsStep::LotsForm {
                        item,
                        rows: sorted_rows,
                        cursor: 0,
                        error: None,
                    };
                }
            }
        }
        KeyCode::Esc => {
            // Close flow; nothing written.
            app.select_lots_flow = None;
        }
        KeyCode::Char('q') => {
            // SWALLOWED: flow is blocking; 'q' must NOT quit while a flow is open.
        }
        _ => {}
    }
}

/// Handle keys at the select-lots flow's LotsForm step.
///
/// 0..9 → push digit to focused row's pick_sat_buf.
/// Backspace → pop from focused row's pick_sat_buf.
/// Enter → validate → open select_lots_modal.
/// Esc → back to List step (one step per press — [I4]).
fn handle_sl_lots_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                if let SelectLotsStep::LotsForm { cursor, .. } = &mut flow.step {
                    *cursor = cursor.saturating_sub(1);
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                if let SelectLotsStep::LotsForm { rows, cursor, .. } = &mut flow.step {
                    *cursor = (*cursor + 1).min(rows.len().saturating_sub(1));
                }
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                if let SelectLotsStep::LotsForm { cursor, .. } = &mut flow.step {
                    *cursor = 0;
                }
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                if let SelectLotsStep::LotsForm { rows, cursor, .. } = &mut flow.step {
                    *cursor = rows.len().saturating_sub(1);
                }
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                if let SelectLotsStep::LotsForm { rows, cursor, .. } = &mut flow.step {
                    if let Some(row) = rows.get_mut(*cursor) {
                        row.pick_sat_buf.push_char(c);
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.select_lots_flow.as_mut() {
                if let SelectLotsStep::LotsForm { rows, cursor, .. } = &mut flow.step {
                    if let Some(row) = rows.get_mut(*cursor) {
                        row.pick_sat_buf.pop_char();
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Validate → open modal on success.
            let validation_result = app.select_lots_flow.as_ref().and_then(|flow| {
                if let SelectLotsStep::LotsForm { item, rows, .. } = &flow.step {
                    Some(validate_select_lots(item, rows).map(|payload| {
                        // Build modal state from validated payload.
                        let (disposal_event, picks) = match &payload {
                            btctax_core::EventPayload::LotSelection(ls) => {
                                (ls.disposal_event.clone(), ls.lots.clone())
                            }
                            _ => unreachable!("validate_select_lots always returns LotSelection"),
                        };
                        let pick_count = picks.len();
                        let total_sat: btctax_core::Sat = picks.iter().map(|p| p.sat).sum();
                        SelectLotsModalState {
                            disposal_event,
                            disposal_date: item.date,
                            disposal_kind: item.kind,
                            principal_sat: item.principal_sat,
                            picks,
                            pick_count,
                            total_sat,
                        }
                    }))
                } else {
                    None
                }
            });

            match validation_result {
                Some(Ok(modal)) => {
                    app.select_lots_modal = Some(modal);
                }
                Some(Err(msg)) => {
                    if let Some(flow) = app.select_lots_flow.as_mut() {
                        if let SelectLotsStep::LotsForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
                None => {}
            }
        }
        KeyCode::Esc => {
            // Back to List step (one step per press — [I4]).
            if let Some(flow) = app.select_lots_flow.as_mut() {
                flow.step = SelectLotsStep::List;
            }
        }
        KeyCode::Char('q') => {
            // SWALLOWED: flow is blocking.
        }
        _ => {}
    }
}

// ── Set-donation-details flow: modal handler ──────────────────────────────────

/// Handle a key press while the set-donation-details confirmation modal is open.
///
/// Enter-arm (spec D2):
///   `Ok(())` → re-project snapshot + derive status + close modal + close flow.
///   `Err(e)` → close modal, keep FieldForm open (buffers intact), status `"Save error: {e}"`.
///
/// Esc → close modal only (back to FieldForm; nothing written).
fn handle_set_donation_details_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (event_id, details) = match app.set_donation_details_modal.as_ref() {
                Some(m) => (m.event_id.clone(), m.details.clone()),
                None => return,
            };

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.set_donation_details_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_donation_details(session, &event_id, &details)
            };

            match save_result {
                Ok(()) => {
                    // Re-project.
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_donation_details_status(&event_id, &details);
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.set_donation_details_modal = None;
                    app.set_donation_details_flow = None;
                }
                Err(e) => {
                    // [M1] On save error: close modal, keep FieldForm open.
                    app.set_donation_details_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal → back to FieldForm (nothing written).
            app.set_donation_details_modal = None;
        }
        _ => {
            // All other keys swallowed.
        }
    }
}

// ── Set-donation-details flow: flow key dispatcher ───────────────────────────

/// Dispatch to the correct sub-handler depending on `SetDonationDetailsStep`.
fn handle_set_donation_details_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.set_donation_details_flow.as_ref() {
        Some(f) => match &f.step {
            SetDonationDetailsStep::List => 0u8,
            SetDonationDetailsStep::FieldForm { .. } => 1u8,
        },
        None => return,
    };
    match step {
        0 => handle_dd_list_key(app, key),
        _ => handle_dd_field_form_key(app, key),
    }
}

/// Handle keys at the donation-details flow's List step.
///
/// Enter → open FieldForm (pre-populated from item.existing_details if present).
/// Esc → close flow (back to Browse).
/// q → SWALLOWED (flow is blocking).
fn handle_dd_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .set_donation_details_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();

            if let Some(item) = selected {
                // Pre-populate buffers from existing_details if present.
                // #6: the 6 FREE-TEXT fields accept CLI-parity length (FREETEXT_CAP = 512);
                // the 4 STRUCTURED fields (ein/tin/ptin/date) keep FIELD_CAP = 64 (fixed-format;
                // a 512-char EIN is nonsense + a render hazard). `.set(existing…)` respects the cap.
                let mut donee_name_buf = FieldBuffer::with_cap(FREETEXT_CAP);
                let mut donee_address_buf = FieldBuffer::with_cap(FREETEXT_CAP);
                let mut donee_ein_buf = FieldBuffer::new();
                let mut appraiser_name_buf = FieldBuffer::with_cap(FREETEXT_CAP);
                let mut appraiser_address_buf = FieldBuffer::with_cap(FREETEXT_CAP);
                let mut appraiser_tin_buf = FieldBuffer::new();
                let mut appraiser_ptin_buf = FieldBuffer::new();
                let mut appraiser_qualifications_buf = FieldBuffer::with_cap(FREETEXT_CAP);
                let mut appraisal_date_buf = FieldBuffer::new();
                let mut fmv_method_override_buf = FieldBuffer::with_cap(FREETEXT_CAP);

                if let Some(existing) = &item.existing_details {
                    donee_name_buf.set(&existing.donee_name);
                    if let Some(v) = &existing.donee_address {
                        donee_address_buf.set(v);
                    }
                    if let Some(v) = &existing.donee_ein {
                        donee_ein_buf.set(v);
                    }
                    appraiser_name_buf.set(&existing.appraiser_name);
                    if let Some(v) = &existing.appraiser_address {
                        appraiser_address_buf.set(v);
                    }
                    if let Some(v) = &existing.appraiser_tin {
                        appraiser_tin_buf.set(v);
                    }
                    if let Some(v) = &existing.appraiser_ptin {
                        appraiser_ptin_buf.set(v);
                    }
                    if let Some(v) = &existing.appraiser_qualifications {
                        appraiser_qualifications_buf.set(v);
                    }
                    if let Some(v) = &existing.appraisal_date {
                        appraisal_date_buf.set(&v.to_string());
                    }
                    if let Some(v) = &existing.fmv_method_override {
                        fmv_method_override_buf.set(v);
                    }
                }

                if let Some(flow) = app.set_donation_details_flow.as_mut() {
                    flow.step = SetDonationDetailsStep::FieldForm {
                        item,
                        donee_name_buf,
                        donee_address_buf,
                        donee_ein_buf,
                        appraiser_name_buf,
                        appraiser_address_buf,
                        appraiser_tin_buf,
                        appraiser_ptin_buf,
                        appraiser_qualifications_buf,
                        appraisal_date_buf,
                        fmv_method_override_buf,
                        focus: 0,
                        error: None,
                    };
                }
            }
        }
        KeyCode::Esc => {
            // Close flow; nothing written.
            app.set_donation_details_flow = None;
        }
        KeyCode::Char('q') => {
            // SWALLOWED: flow is blocking.
        }
        _ => {}
    }
}

/// Handle keys at the donation-details flow's FieldForm step.
///
/// ↑/↓ → move focus. Printable chars → push to focused buf. Backspace → pop.
/// Enter → validate → open modal. Esc → back to List step [I4].
#[allow(clippy::too_many_lines)]
fn handle_dd_field_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                if let SetDonationDetailsStep::FieldForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Down => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                if let SetDonationDetailsStep::FieldForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(9);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                if let SetDonationDetailsStep::FieldForm {
                    focus,
                    donee_name_buf,
                    donee_address_buf,
                    donee_ein_buf,
                    appraiser_name_buf,
                    appraiser_address_buf,
                    appraiser_tin_buf,
                    appraiser_ptin_buf,
                    appraiser_qualifications_buf,
                    appraisal_date_buf,
                    fmv_method_override_buf,
                    ..
                } = &mut flow.step
                {
                    let bufs: [&mut FieldBuffer; 10] = [
                        donee_name_buf,
                        donee_address_buf,
                        donee_ein_buf,
                        appraiser_name_buf,
                        appraiser_address_buf,
                        appraiser_tin_buf,
                        appraiser_ptin_buf,
                        appraiser_qualifications_buf,
                        appraisal_date_buf,
                        fmv_method_override_buf,
                    ];
                    if let Some(b) = bufs.into_iter().nth(*focus) {
                        b.pop_char();
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                if let SetDonationDetailsStep::FieldForm {
                    focus,
                    donee_name_buf,
                    donee_address_buf,
                    donee_ein_buf,
                    appraiser_name_buf,
                    appraiser_address_buf,
                    appraiser_tin_buf,
                    appraiser_ptin_buf,
                    appraiser_qualifications_buf,
                    appraisal_date_buf,
                    fmv_method_override_buf,
                    ..
                } = &mut flow.step
                {
                    let bufs: [&mut FieldBuffer; 10] = [
                        donee_name_buf,
                        donee_address_buf,
                        donee_ein_buf,
                        appraiser_name_buf,
                        appraiser_address_buf,
                        appraiser_tin_buf,
                        appraiser_ptin_buf,
                        appraiser_qualifications_buf,
                        appraisal_date_buf,
                        fmv_method_override_buf,
                    ];
                    if let Some(b) = bufs.into_iter().nth(*focus) {
                        b.push_char(c);
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Validate → open modal on success.
            let validation_result = app.set_donation_details_flow.as_ref().and_then(|flow| {
                if let SetDonationDetailsStep::FieldForm {
                    item,
                    donee_name_buf,
                    donee_address_buf,
                    donee_ein_buf,
                    appraiser_name_buf,
                    appraiser_address_buf,
                    appraiser_tin_buf,
                    appraiser_ptin_buf,
                    appraiser_qualifications_buf,
                    appraisal_date_buf,
                    fmv_method_override_buf,
                    ..
                } = &flow.step
                {
                    Some(
                        validate_donation_details(
                            donee_name_buf,
                            donee_address_buf,
                            donee_ein_buf,
                            appraiser_name_buf,
                            appraiser_address_buf,
                            appraiser_tin_buf,
                            appraiser_ptin_buf,
                            appraiser_qualifications_buf,
                            appraisal_date_buf,
                            fmv_method_override_buf,
                        )
                        .map(|details| SetDonationDetailsModalState {
                            event_id: item.event_id.clone(),
                            event_date: item.date,
                            total_sat: item.total_sat,
                            details,
                        }),
                    )
                } else {
                    None
                }
            });

            match validation_result {
                Some(Ok(modal)) => {
                    app.set_donation_details_modal = Some(modal);
                }
                Some(Err(msg)) => {
                    if let Some(flow) = app.set_donation_details_flow.as_mut() {
                        if let SetDonationDetailsStep::FieldForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
                None => {}
            }
        }
        KeyCode::Esc => {
            // Back to List step (one step per press — [I4]).
            if let Some(flow) = app.set_donation_details_flow.as_mut() {
                flow.step = SetDonationDetailsStep::List;
            }
        }
        // Note: 'q' as a Char lands in the Char(c) arm above and is pushed to the focused
        // buffer. The flow guard ensures 'q' never reaches a Browse quit arm [I4].
        _ => {}
    }
}

// ── Select-lots flow: opener ──────────────────────────────────────────────────

/// Open the select-lots flow from the Browse screen.
///
/// Applies the compound pre-filter (spec §Claim F):
/// 1. Voided-decision set: IDs targeted by VoidDecisionEvent.
/// 2. Already-selected set: disposal_event IDs of non-voided LotSelection decisions.
/// 3. Disposals (sell/spend) excluding fee_mini_disposition and already-selected.
/// 4. Removals (gift/donation — BOTH kinds) excluding already-selected.
///
/// Per-item wallet sourced from `events_by_id(snap)[&event].wallet` [R0-I1]:
/// NOT from DisposalLeg (would miss Gift/Donate rows where RemovalLeg has no wallet).
///
/// Empty filtered list → status "No method-honoring disposals available for lot
/// selection (select-lots pre-filter)"; flow NOT opened [R0-M8].
fn open_select_lots_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

    // Build voided set (IDs targeted by any VoidDecisionEvent).
    let voided: std::collections::BTreeSet<&EventId> = snap
        .events
        .iter()
        .filter_map(|e| {
            if let EventPayload::VoidDecisionEvent(v) = &e.payload {
                Some(&v.target_event_id)
            } else {
                None
            }
        })
        .collect();

    // Build already-selected set: disposal_event IDs of non-voided LotSelection decisions.
    let already_selected: std::collections::BTreeSet<&EventId> = snap
        .events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| {
            if let EventPayload::LotSelection(ls) = &e.payload {
                Some(&ls.disposal_event)
            } else {
                None
            }
        })
        .collect();

    // #3: pre-filter events carrying an UncoveredDisposal blocker out of the MERGED list
    // (disposals + removals + self-transfers). Selecting lots can NEVER cure under-coverage
    // (the pool is short); the disposal stays actionable in Compliance, whose UncoveredDisposal
    // blocker names the real remedy (add the missing acquisition).
    let uncovered: std::collections::BTreeSet<&EventId> = snap
        .state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::UncoveredDisposal)
        .filter_map(|b| b.event.as_ref())
        .collect();

    // #1: reconstruct SelfTransfers — a TransferOut projects to Op::SelfTransfer iff a non-voided
    // TransferLink names it with a resolvable destination wallet (resolve.rs:201-216). Mirror the
    // engine's pass-1 link build (resolve.rs:486-527): iterate non-voided TransferLinks by
    // decision_seq ASC [R0-M2] and FIRST-WINS on a duplicate out_event; dedup in-events; skip an
    // in-event that is missing or has no wallet (use `ev_idx.get`, never index [R0-M2]).
    let mut transfer_links: Vec<(u64, &btctax_core::event::TransferLink)> = snap
        .events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| match (&e.id, &e.payload) {
            (EventId::Decision { seq }, EventPayload::TransferLink(tl)) => Some((*seq, tl)),
            _ => None,
        })
        .collect();
    transfer_links.sort_by_key(|(seq, _)| *seq);

    let mut linked_outs: std::collections::BTreeSet<EventId> = std::collections::BTreeSet::new();
    let mut consumed_ins: std::collections::BTreeSet<EventId> = std::collections::BTreeSet::new();
    for (_seq, tl) in &transfer_links {
        if linked_outs.contains(&tl.out_event) {
            continue; // duplicate out_event → first (lowest-seq) wins
        }
        match &tl.in_event_or_wallet {
            TransferTarget::Wallet(_) => {
                linked_outs.insert(tl.out_event.clone());
            }
            TransferTarget::InEvent(in_id) => {
                if consumed_ins.contains(in_id) {
                    continue; // in-event already consumed by an earlier link
                }
                if ev_idx.get(in_id).and_then(|e| e.wallet.as_ref()).is_none() {
                    continue; // in-event missing or has no resolvable destination wallet
                }
                consumed_ins.insert(in_id.clone());
                linked_outs.insert(tl.out_event.clone());
            }
        }
    }

    // Disposals (sell / spend).
    let disposal_items: Vec<DisposalListItem> = snap
        .state
        .disposals
        .iter()
        .filter(|d| !d.fee_mini_disposition)
        .filter(|d| !already_selected.contains(&d.event))
        .filter(|d| !uncovered.contains(&d.event))
        .map(|d| {
            let principal_sat: btctax_core::Sat = d.legs.iter().map(|l| l.sat).sum();
            // Wallet sourced from the raw LedgerEvent [R0-I1].
            let wallet = ev_idx.get(&d.event).and_then(|e| e.wallet.clone());
            // Disposal.kind is DisposeKind {Sell, Spend} (state.rs:38-40).
            // Gift/Donate removals come from snap.state.removals, not snap.state.disposals.
            let kind = match d.kind {
                DisposeKind::Sell => DisposalKind::Sell,
                DisposeKind::Spend => DisposalKind::Spend,
            };
            DisposalListItem {
                disposal_event: d.event.clone(),
                date: d.disposed_at,
                kind,
                principal_sat,
                wallet,
            }
        })
        .collect();

    // Removals (gift / donation — BOTH kinds — [R0-Claim F]).
    let removal_items: Vec<DisposalListItem> = snap
        .state
        .removals
        .iter()
        .filter(|r| !already_selected.contains(&r.event))
        .filter(|r| !uncovered.contains(&r.event))
        .map(|r| {
            let principal_sat: btctax_core::Sat = r.legs.iter().map(|l| l.sat).sum();
            // Wallet sourced from the raw LedgerEvent [R0-I1].
            let wallet = ev_idx.get(&r.event).and_then(|e| e.wallet.clone());
            let kind = match r.kind {
                RemovalKind::Gift => DisposalKind::Gift,
                RemovalKind::Donation => DisposalKind::Donate,
            };
            DisposalListItem {
                disposal_event: r.event.clone(),
                date: r.removed_at,
                kind,
                principal_sat,
                wallet,
            }
        })
        .collect();

    // #1: self-transfer rows from the raw TransferOut events whose id ∈ linked_outs.
    let self_transfer_items: Vec<DisposalListItem> = linked_outs
        .iter()
        .filter(|out_id| !already_selected.contains(*out_id))
        .filter(|out_id| !uncovered.contains(*out_id))
        .filter_map(|out_id| {
            let e = ev_idx.get(out_id)?;
            let transfer_out = match &e.payload {
                EventPayload::TransferOut(t) => t,
                _ => return None,
            };
            // `date` sourced as at main.rs (tax_date of the raw event).
            let date = btctax_core::conventions::tax_date(e.utc_timestamp, e.original_tz);
            Some(DisposalListItem {
                disposal_event: e.id.clone(),
                date,
                kind: DisposalKind::SelfTransfer,
                // principal_sat = TransferOut.sat (NOT minus fee — matches honoring_principal).
                principal_sat: transfer_out.sat,
                // SOURCE wallet — correct for the candidate-lot filter.
                wallet: e.wallet.clone(),
            })
        })
        .collect();

    // Merge and sort by date DESC (most recent first — matching the display tabs).
    let mut items: Vec<DisposalListItem> =
        [disposal_items, removal_items, self_transfer_items].concat();
    items.sort_by_key(|item| std::cmp::Reverse(item.date));

    if items.is_empty() {
        app.status = Some(
            "No method-honoring disposals available for lot selection (select-lots pre-filter)"
                .to_string(),
        );
        return;
    }

    app.select_lots_flow = Some(SelectLotsFlowState {
        list: TargetList::new(items),
        step: SelectLotsStep::List,
    });
}

// ── Set-donation-details flow: opener ────────────────────────────────────────

/// Open the set-donation-details flow from the Browse screen.
///
/// Applies the pre-filter from spec §Claim G:
/// - Only `snap.state.removals` entries where `r.kind == RemovalKind::Donation`.
/// - No "already-complete" exclusion: re-setting is always valid (last-write-wins).
/// - `existing_details` sourced from `snap.donation_details` [R0-I3] (NEVER `conn(`).
///
/// Empty filtered list → status "No donation removals found (donate a TransferOut first
/// via reclassify-outflow)"; flow NOT opened [R0-M8].
fn open_set_donation_details_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let mut items: Vec<DonationListItem> = snap
        .state
        .removals
        .iter()
        .filter(|r| r.kind == RemovalKind::Donation)
        .map(|r| {
            let total_sat: btctax_core::Sat = r.legs.iter().map(|l| l.sat).sum();
            // existing_details from snap.donation_details — NEVER conn( [R0-I3].
            let existing_details = snap.donation_details.get(&r.event).cloned();
            DonationListItem {
                event_id: r.event.clone(),
                date: r.removed_at,
                total_sat,
                donee: r.donee.clone(),
                existing_details,
            }
        })
        .collect();

    // Sort by date DESC.
    items.sort_by_key(|item| std::cmp::Reverse(item.date));

    if items.is_empty() {
        app.status = Some(
            "No donation removals found (donate a TransferOut first via reclassify-outflow)"
                .to_string(),
        );
        return;
    }

    app.set_donation_details_flow = Some(SetDonationDetailsFlowState {
        list: TargetList::new(items),
        step: SetDonationDetailsStep::List,
    });
}

// ── Status derivers ───────────────────────────────────────────────────────────

/// Derive the status string from RE-PROJECTED state after a select-lots save.
///
/// Three arms (spec D1):
/// 1. `DecisionConflict` attributed to `decision_id` → NEITHER applies; method-order fallback.
///    Status ends with `"(see Compliance)"` [R0-N3 nit sweep].
/// 2. `LotSelectionInvalid` with `event == disposal_event` → engine rejected the selection.
/// 3. Clean → success summary with pick_count and total_sat.
fn derive_select_lots_status(
    snap: &btctax_tui::app::Snapshot,
    disposal_event: &EventId,
    decision_id: &EventId,
    pick_count: usize,
    total_sat: btctax_core::Sat,
) -> String {
    // Arm 1: DecisionConflict attributed to the SECOND selection's decision_id.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired — neither selection applies (method order \
                 governs); clear with Void flow (press 'v'), or quit the editor and run: \
                 btctax reconcile void {} (see Compliance)",
                decision_id.canonical()
            );
        }
    }

    // Arm 2: LotSelectionInvalid attributed to the disposal_event.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::LotSelectionInvalid && b.event.as_ref() == Some(disposal_event) {
            return "LotSelection saved but invalid — see Compliance for detail; the disposal \
                    falls back to method order. Correct via Void flow (press 'v') then re-select."
                .to_string();
        }
    }

    // Arm 3: clean.
    format!(
        "Lot selection recorded for {} — {pick_count} lot(s), {total_sat} sat; \
         check Compliance for §1.1012-1(j) contemporaneity.",
        disposal_event.canonical()
    )
}

/// Derive the status string from the IN-HAND validated details (spec D2).
///
/// Uses `details.is_review_complete(Form8283Section::B)` directly (last-write-wins
/// guarantees the value just written IS the stored value — no side-table re-load,
/// no `conn(` in main.rs [R0-I3(c)]).
fn derive_donation_details_status(event_id: &EventId, details: &DonationDetails) -> String {
    if details.is_review_complete(Form8283Section::B) {
        format!(
            "Details saved for {} — Section B complete (§6695A fields present)",
            event_id.canonical()
        )
    } else {
        format!(
            "Details saved for {} — Section A complete on presence; add appraiser \
             TIN/PTIN + appraisal date + qualifications + donee EIN for Section B completeness",
            event_id.canonical()
        )
    }
}

// ── Link-transfer flow (chunk 4a, D1) ────────────────────────────────────────

/// Open the link-transfer flow from the Browse screen (chunk 4a, D1).
///
/// Three pre-filtered sets, all built at open (the flow owns all three lists [R0-I2]):
/// - out-list: `snap.state.pending_reconciliation` (inherently post-filtered — the unlinked,
///   unreconciled TransferOuts; shared with reclassify-outflow, mutually-exclusive resolutions).
/// - in-list: `TransferIn` events whose raw `LedgerEvent.wallet.is_some()` (engine requires a
///   resolvable dest wallet) minus those already targeted by a non-voided `TransferLink::InEvent`.
/// - wallet-list: ALL distinct `snap.events[].wallet` Some-values [R0-I2] (NOT just
///   `holdings_by_wallet` keys — a zero-balance destination wallet must be offerable).
///
/// Empty out-list → status "No pending outbound transfers to link"; flow NOT opened [R0-M8].
fn open_link_transfer_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

    // Voided-decision set (for the consumed-in filter).
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

    // In-events already targeted by a non-voided TransferLink::InEvent (adding a second would fire
    // DecisionConflict; FIRST-WINS). Mirrors open_classify_inbound_flow's already_classified.
    let consumed_ins: BTreeSet<EventId> = snap
        .events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| {
            if let EventPayload::TransferLink(tl) = &e.payload {
                if let TransferTarget::InEvent(in_id) = &tl.in_event_or_wallet {
                    return Some(in_id.clone());
                }
            }
            None
        })
        .collect();

    // Out-list (step 1): pending_reconciliation (post-filtered by the engine).
    let mut out_items: Vec<TransferOutItem> = snap
        .state
        .pending_reconciliation
        .iter()
        .map(|pt| {
            let ev = ev_idx.get(&pt.event);
            let date = ev
                .map(|e| btctax_core::conventions::tax_date(e.utc_timestamp, e.original_tz))
                .unwrap_or_else(|| {
                    btctax_core::conventions::tax_date(
                        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
                        time::UtcOffset::UTC,
                    )
                });
            let wallet = ev.and_then(|e| e.wallet.clone());
            TransferOutItem {
                transfer_out_event: pt.event.clone(),
                date,
                principal_sat: pt.principal_sat,
                wallet,
            }
        })
        .collect();
    out_items.sort_by_key(|i| i.date);

    if out_items.is_empty() {
        // R0-M8: empty out-list never opens a flow.
        app.status = Some("No pending outbound transfers to link".to_string());
        return;
    }

    // In-list (step 2, InEvent mode): TransferIn with a resolvable wallet, not already consumed.
    let mut in_items: Vec<InEventItem> = snap
        .events
        .iter()
        .filter_map(|e| {
            let ti = match &e.payload {
                EventPayload::TransferIn(ti) => ti,
                _ => return None,
            };
            let wallet = e.wallet.clone()?; // engine requires a resolvable dest wallet
            if consumed_ins.contains(&e.id) {
                return None;
            }
            let date = btctax_core::conventions::tax_date(e.utc_timestamp, e.original_tz);
            Some(InEventItem {
                in_event: e.id.clone(),
                date,
                sat: ti.sat,
                wallet,
            })
        })
        .collect();
    in_items.sort_by_key(|i| i.date);

    // Wallet-list (step 2, Wallet mode): ALL distinct snap.events[].wallet Some-values [R0-I2].
    // A BTreeSet dedups AND sorts (WalletId: Ord) for stable display.
    let wallet_set: BTreeSet<btctax_core::WalletId> = snap
        .events
        .iter()
        .filter_map(|e| e.wallet.clone())
        .collect();
    let wallet_items: Vec<WalletItem> = wallet_set
        .into_iter()
        .map(|wallet| WalletItem { wallet })
        .collect();

    app.link_transfer_flow = Some(LinkTransferFlowState {
        out_list: TargetList::new(out_items),
        step: LinkTransferStep::OutList,
        in_list: TargetList::new(in_items),
        wallet_list: TargetList::new(wallet_items),
    });
}

/// The active mode of an open TargetPick step (defaults to InEvent defensively).
fn current_link_mode(flow: &LinkTransferFlowState) -> LinkMode {
    match &flow.step {
        LinkTransferStep::TargetPick { mode, .. } => *mode,
        _ => LinkMode::InEvent,
    }
}

/// Dispatch to the correct sub-handler depending on `LinkTransferStep`.
fn handle_link_transfer_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.link_transfer_flow.as_ref() {
        Some(f) => match &f.step {
            LinkTransferStep::OutList => 0u8,
            LinkTransferStep::TargetPick { .. } => 1u8,
        },
        None => return,
    };
    match step {
        0 => handle_lt_out_list_key(app, key),
        _ => handle_lt_target_pick_key(app, key),
    }
}

/// Handle keys at the link-transfer flow's OutList step.
///
/// Enter → transition to TargetPick { out: selected, mode: InEvent }.
/// Esc → close flow (back to Browse). q → SWALLOWED (flow is blocking).
fn handle_lt_out_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                flow.out_list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                flow.out_list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                flow.out_list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                flow.out_list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .link_transfer_flow
                .as_ref()
                .and_then(|f| f.out_list.selected())
                .cloned();
            if let Some(out) = selected {
                if let Some(flow) = app.link_transfer_flow.as_mut() {
                    flow.step = LinkTransferStep::TargetPick {
                        out,
                        mode: LinkMode::InEvent,
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.link_transfer_flow = None;
        }
        KeyCode::Char('q') => {
            // SWALLOWED: flow is blocking; 'q' must NOT quit while a flow is open.
        }
        _ => {}
    }
}

/// Handle keys at the link-transfer flow's TargetPick step.
///
/// Tab/BackTab → toggle mode (InEvent ⇄ Wallet). ↑/↓/g/G → nav the ACTIVE list.
/// Enter → build the `TransferTarget` from the active list's selection → open link_transfer_modal.
/// Esc → back to OutList (one step per press). q → SWALLOWED.
fn handle_lt_target_pick_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Tab | KeyCode::BackTab => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                if let LinkTransferStep::TargetPick { mode, .. } = &mut flow.step {
                    *mode = match mode {
                        LinkMode::InEvent => LinkMode::Wallet,
                        LinkMode::Wallet => LinkMode::InEvent,
                    };
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                match current_link_mode(flow) {
                    LinkMode::InEvent => flow.in_list.scroll_up(),
                    LinkMode::Wallet => flow.wallet_list.scroll_up(),
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                match current_link_mode(flow) {
                    LinkMode::InEvent => flow.in_list.scroll_down(),
                    LinkMode::Wallet => flow.wallet_list.scroll_down(),
                }
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                match current_link_mode(flow) {
                    LinkMode::InEvent => flow.in_list.go_top(),
                    LinkMode::Wallet => flow.wallet_list.go_top(),
                }
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                match current_link_mode(flow) {
                    LinkMode::InEvent => flow.in_list.go_bottom(),
                    LinkMode::Wallet => flow.wallet_list.go_bottom(),
                }
            }
        }
        KeyCode::Enter => {
            let modal = app.link_transfer_flow.as_ref().and_then(|flow| {
                let out = match &flow.step {
                    LinkTransferStep::TargetPick { out, .. } => out,
                    _ => return None,
                };
                let (target, target_label) = match current_link_mode(flow) {
                    LinkMode::InEvent => {
                        let item = flow.in_list.selected()?;
                        (
                            TransferTarget::InEvent(item.in_event.clone()),
                            format!(
                                "TransferIn {} (wallet {})",
                                item.in_event.canonical(),
                                crate::edit::form::wallet_label(&item.wallet)
                            ),
                        )
                    }
                    LinkMode::Wallet => {
                        let item = flow.wallet_list.selected()?;
                        (
                            TransferTarget::Wallet(item.wallet.clone()),
                            format!("wallet {}", crate::edit::form::wallet_label(&item.wallet)),
                        )
                    }
                };
                Some(LinkTransferModalState {
                    out_event: out.transfer_out_event.clone(),
                    out_date: out.date,
                    out_sat: out.principal_sat,
                    target,
                    target_label,
                })
            });
            // If the active list is empty (no selection), Enter is a no-op.
            if let Some(modal) = modal {
                app.link_transfer_modal = Some(modal);
            }
        }
        KeyCode::Esc => {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                flow.step = LinkTransferStep::OutList;
            }
        }
        KeyCode::Char('q') => {
            // SWALLOWED: flow is blocking.
        }
        _ => {}
    }
}

/// Handle a key press while the link-transfer confirmation modal is open (chunk 4a, D1).
///
/// Enter → build the TransferLink payload → persist_link_transfer → re-project + status + close.
///   `Err(e)` → close modal, route through `on_persist_error` (benign → keep TargetPick open).
/// Esc → close modal only (back to TargetPick; nothing written).
fn handle_link_transfer_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (out_event, target, target_label) = match app.link_transfer_modal.as_ref() {
                Some(m) => (
                    m.out_event.clone(),
                    m.target.clone(),
                    m.target_label.clone(),
                ),
                None => return,
            };

            let payload =
                btctax_core::EventPayload::TransferLink(btctax_core::event::TransferLink {
                    out_event: out_event.clone(),
                    in_event_or_wallet: target,
                });
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.link_transfer_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_link_transfer(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_link_transfer_status(
                                &snap,
                                &out_event,
                                &decision_id,
                                &target_label,
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
                    app.link_transfer_modal = None;
                    app.link_transfer_flow = None;
                }
                Err(e) => {
                    app.link_transfer_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            // Cancel: close modal → back to TargetPick (nothing written).
            app.link_transfer_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal — 'q' must NOT quit here).
        }
    }
}

/// Derive the status string from RE-PROJECTED state after a link-transfer save (chunk 4a, D1).
///
/// Two arms (spec D1):
/// 1. `DecisionConflict` attributed to `decision_id` (duplicate link — effectively unreachable
///    given the exclusive lock + the up-front pre-filter; a defensive arm [R0-M3]).
/// 2. Clean → the non-taxable self-transfer success framing.
fn derive_link_transfer_status(
    snap: &btctax_tui::app::Snapshot,
    out_event: &EventId,
    decision_id: &EventId,
    target_label: &str,
) -> String {
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired — the link was not applied; clear with Void flow \
                 (press 'v'), or quit the editor and run: btctax reconcile void {}",
                decision_id.canonical()
            );
        }
    }
    format!(
        "Self-transfer link recorded for {} → {target_label}; the TransferOut is now a \
         non-taxable relocation.",
        out_event.canonical()
    )
}

// ── Classify-raw flow (chunk 4a, D2) ─────────────────────────────────────────

/// Open the classify-raw flow from the Browse screen (chunk 4a, D2).
///
/// Pre-filter: events carrying `BlockerKind::Unclassified` whose payload is
/// `EventPayload::Unclassified`, minus those already targeted by a non-voided `ClassifyRaw`
/// (a second classification of one target → `DecisionConflict`; FIRST-WINS). Same shape as
/// `open_classify_inbound_flow`'s filter keyed on `Unclassified`.
///
/// Empty filtered list → status "No unclassified raw imports"; flow NOT opened [R0-M8].
fn open_classify_raw_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

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

    // Targets already classified by a non-voided ClassifyRaw (a second → DecisionConflict).
    let already_classified: BTreeSet<EventId> = snap
        .events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| {
            if let EventPayload::ClassifyRaw(cr) = &e.payload {
                Some(cr.target.clone())
            } else {
                None
            }
        })
        .collect();

    let mut items: Vec<RawListItem> = snap
        .state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::Unclassified)
        .filter_map(|b| {
            let target = b.event.as_ref()?;
            let ev = ev_idx.get(target)?;
            // Raw-only: the target's RAW payload must be Unclassified.
            let raw = match &ev.payload {
                EventPayload::Unclassified(u) => u.raw.clone(),
                _ => return None,
            };
            if already_classified.contains(target) {
                return None;
            }
            let date = btctax_core::conventions::tax_date(ev.utc_timestamp, ev.original_tz);
            Some(RawListItem {
                target: target.clone(),
                date,
                raw,
                wallet: ev.wallet.clone(),
            })
        })
        .collect();
    items.sort_by_key(|i| i.date);

    if items.is_empty() {
        // R0-M8: empty filtered list never opens a flow.
        app.status = Some("No unclassified raw imports".to_string());
        return;
    }

    app.classify_raw_flow = Some(ClassifyRawFlowState {
        list: TargetList::new(items),
        step: ClassifyRawStep::List,
    });
}

/// Dispatch to the correct sub-handler depending on `ClassifyRawStep`.
fn handle_classify_raw_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.classify_raw_flow.as_ref() {
        Some(f) => match &f.step {
            ClassifyRawStep::List => 0u8,
            ClassifyRawStep::VariantPicker { .. } => 1u8,
            ClassifyRawStep::AcquireForm { .. } => 2u8,
            ClassifyRawStep::IncomeForm { .. } => 3u8,
        },
        None => return,
    };
    match step {
        0 => handle_cr_list_key(app, key),
        1 => handle_cr_picker_key(app, key),
        2 => handle_cr_acquire_form_key(app, key),
        _ => handle_cr_income_form_key(app, key),
    }
}

/// List step: Enter → variant picker (initial Acquire). Esc → close flow. q → swallowed.
fn handle_cr_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .classify_raw_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                if let Some(flow) = app.classify_raw_flow.as_mut() {
                    flow.step = ClassifyRawStep::VariantPicker {
                        item,
                        variant: ClassifyRawVariant::Acquire, // initial
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.classify_raw_flow = None;
        }
        _ => {
            // All other keys (including 'q') swallowed while flow is open [R0-I2].
        }
    }
}

/// Variant-picker step: Acquire ↔ Income via Tab; Enter → the per-variant sub-form.
fn handle_cr_picker_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::VariantPicker { variant, .. } = &mut flow.step {
                    *variant = cycle_classify_raw_variant(*variant);
                }
            }
        }
        KeyCode::Enter => {
            let Some(flow) = app.classify_raw_flow.as_mut() else {
                return;
            };
            let old_step = std::mem::replace(&mut flow.step, ClassifyRawStep::List);
            if let ClassifyRawStep::VariantPicker { item, variant } = old_step {
                flow.step = match variant {
                    ClassifyRawVariant::Acquire => ClassifyRawStep::AcquireForm {
                        item,
                        sat_buf: FieldBuffer::new(),
                        usd_cost_buf: FieldBuffer::new(),
                        fee_buf: FieldBuffer::new(),
                        basis_source: BasisSource::ExchangeProvided, // default PICK
                        focus: 0,
                        error: None,
                    },
                    ClassifyRawVariant::Income => ClassifyRawStep::IncomeForm {
                        item,
                        sat_buf: FieldBuffer::new(),
                        fmv_buf: FieldBuffer::new(),
                        kind: IncomeKind::Mining, // initial
                        business: false,
                        focus: 0,
                        error: None,
                    },
                };
            }
        }
        KeyCode::Esc => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.step = ClassifyRawStep::List;
            }
        }
        _ => {
            // All other keys (including 'q') swallowed [R0-I2].
        }
    }
}

/// Acquire-form step: sat/usd_cost/fee text, basis_source picker (Tab on row 3), submit.
fn handle_cr_acquire_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let result = match app.classify_raw_flow.as_ref() {
                Some(f) => match &f.step {
                    ClassifyRawStep::AcquireForm {
                        item,
                        sat_buf,
                        usd_cost_buf,
                        fee_buf,
                        basis_source,
                        ..
                    } => {
                        validate_classify_raw_acquire(sat_buf, usd_cost_buf, fee_buf, *basis_source)
                            .map(|built| (item.clone(), built))
                    }
                    _ => return,
                },
                None => return,
            };
            match result {
                Ok((item, built)) => {
                    app.classify_raw_modal = Some(ClassifyRawModalState {
                        target: item.target.clone(),
                        raw: item.raw.clone(),
                        built,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.classify_raw_flow.as_mut() {
                        if let ClassifyRawStep::AcquireForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            let item = match app.classify_raw_flow.as_ref() {
                Some(f) => match &f.step {
                    ClassifyRawStep::AcquireForm { item, .. } => item.clone(),
                    _ => return,
                },
                None => return,
            };
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.step = ClassifyRawStep::VariantPicker {
                    item,
                    variant: ClassifyRawVariant::Acquire,
                };
            }
        }
        KeyCode::Tab => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::AcquireForm {
                    basis_source,
                    focus,
                    ..
                } = &mut flow.step
                {
                    if *focus == 3 {
                        *basis_source = cycle_basis_source(*basis_source);
                    } else {
                        *focus = (*focus + 1).min(3);
                    }
                }
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::AcquireForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Down => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::AcquireForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(3);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::AcquireForm {
                    sat_buf,
                    usd_cost_buf,
                    fee_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => sat_buf.pop_char(),
                        1 => usd_cost_buf.pop_char(),
                        2 => fee_buf.pop_char(),
                        _ => {}
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::AcquireForm {
                    sat_buf,
                    usd_cost_buf,
                    fee_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => sat_buf.push_char(c),
                        1 => usd_cost_buf.push_char(c),
                        2 => fee_buf.push_char(c),
                        // focus==3 (basis_source): Tab cycles (handled above); other chars swallowed.
                        _ => {}
                    }
                }
            }
        }
        _ => {
            // All unmatched keys (including 'q' at a picker row) swallowed [R0-I2].
        }
    }
}

/// Income-form step: sat/usd_fmv text, kind picker (Tab on row 2), business toggle (Space), submit.
fn handle_cr_income_form_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let result = match app.classify_raw_flow.as_ref() {
                Some(f) => match &f.step {
                    ClassifyRawStep::IncomeForm {
                        item,
                        sat_buf,
                        fmv_buf,
                        kind,
                        business,
                        ..
                    } => validate_classify_raw_income(sat_buf, fmv_buf, *kind, *business)
                        .map(|built| (item.clone(), built)),
                    _ => return,
                },
                None => return,
            };
            match result {
                Ok((item, built)) => {
                    app.classify_raw_modal = Some(ClassifyRawModalState {
                        target: item.target.clone(),
                        raw: item.raw.clone(),
                        built,
                    });
                }
                Err(msg) => {
                    if let Some(flow) = app.classify_raw_flow.as_mut() {
                        if let ClassifyRawStep::IncomeForm { error, .. } = &mut flow.step {
                            *error = Some(msg);
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            let item = match app.classify_raw_flow.as_ref() {
                Some(f) => match &f.step {
                    ClassifyRawStep::IncomeForm { item, .. } => item.clone(),
                    _ => return,
                },
                None => return,
            };
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                flow.step = ClassifyRawStep::VariantPicker {
                    item,
                    variant: ClassifyRawVariant::Income,
                };
            }
        }
        KeyCode::Tab => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::IncomeForm { kind, focus, .. } = &mut flow.step {
                    if *focus == 2 {
                        *kind = cycle_income_kind(*kind);
                    } else {
                        *focus = (*focus + 1).min(3);
                    }
                }
            }
        }
        KeyCode::BackTab | KeyCode::Up => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::IncomeForm { focus, .. } = &mut flow.step {
                    *focus = focus.saturating_sub(1);
                }
            }
        }
        KeyCode::Down => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::IncomeForm { focus, .. } = &mut flow.step {
                    *focus = (*focus + 1).min(3);
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::IncomeForm {
                    business, focus, ..
                } = &mut flow.step
                {
                    if *focus == 3 {
                        *business = !*business;
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::IncomeForm {
                    sat_buf,
                    fmv_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => sat_buf.pop_char(),
                        1 => fmv_buf.pop_char(),
                        _ => {}
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                if let ClassifyRawStep::IncomeForm {
                    sat_buf,
                    fmv_buf,
                    focus,
                    ..
                } = &mut flow.step
                {
                    match *focus {
                        0 => sat_buf.push_char(c),
                        1 => fmv_buf.push_char(c),
                        // focus==2 (kind): Tab cycles; focus==3 (business): Space toggles.
                        _ => {}
                    }
                }
            }
        }
        _ => {
            // All unmatched keys (including 'q' at a picker/toggle row) swallowed [R0-I2].
        }
    }
}

/// Static "Acquire"/"Income" tag for a built classify-raw payload (modal + status).
fn classify_raw_variant_label(built: &EventPayload) -> &'static str {
    match built {
        EventPayload::Acquire(_) => "Acquire",
        EventPayload::Income(_) => "Income",
        _ => "imported",
    }
}

/// Handle a key press while the classify-raw confirmation modal is open (chunk 4a, D2).
///
/// Enter → build `ClassifyRaw{target, as_: Box::new(built)}` → persist_classify_raw → re-project +
///   status + close. `Err(e)` → close modal, route through `on_persist_error`.
/// Esc → close modal only (back to the sub-form; nothing written).
fn handle_classify_raw_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (target, built) = match app.classify_raw_modal.as_ref() {
                Some(m) => (m.target.clone(), m.built.clone()),
                None => return,
            };
            let variant_label = classify_raw_variant_label(&built);

            let payload = btctax_core::EventPayload::ClassifyRaw(btctax_core::event::ClassifyRaw {
                target: target.clone(),
                as_: Box::new(built),
            });
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.classify_raw_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_classify_raw(session, payload, now)
            };

            match save_result {
                Ok(decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_classify_raw_status(
                                &snap,
                                &target,
                                &decision_id,
                                variant_label,
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
                    app.classify_raw_modal = None;
                    app.classify_raw_flow = None;
                }
                Err(e) => {
                    app.classify_raw_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.classify_raw_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal).
        }
    }
}

/// Derive the status string from RE-PROJECTED state after a classify-raw save (chunk 4a, D2).
///
/// Three arms (spec D2):
/// 1. `DecisionConflict` attributed to `decision_id` (duplicate classify) → clear-with-void.
/// 2. Clean and the target's `Unclassified` blocker is gone → success.
/// 3. Clean but a NEW blocker attributes to the target — for the scoped Income/Acquire variants
///    this is `FmvMissing` (Income with an empty `usd_fmv` → `Missing`) [R0-M2].
fn derive_classify_raw_status(
    snap: &btctax_tui::app::Snapshot,
    target: &EventId,
    decision_id: &EventId,
    variant_label: &str,
) -> String {
    // Arm 1: DecisionConflict attributed to the decision_id.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired — the classification was not applied; clear with \
                 Void flow (press 'v'), or quit the editor and run: btctax reconcile void {}",
                decision_id.canonical()
            );
        }
    }

    // Arm 3: a NEW blocker attributes to the target (FmvMissing for the scoped variants).
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(target) {
            return format!("Classified, but {:?} now applies — see Compliance.", b.kind);
        }
    }

    // Arm 2: clean — the Unclassified blocker is cleared.
    format!(
        "Classified {} as {variant_label}; the Unclassified blocker is cleared.",
        target.canonical()
    )
}

// ── Safe-harbor-attest flow ───────────────────────────────────────────────────

/// Open the safe-harbor-attest flow from the Browse screen.
///
/// # Step 0 — latch check [R0-C1]
/// If `attest_save_failed` → the latch status; return. (Every mutating opener carries
/// the same guard; here it also protects the direct-call path.)
///
/// # Step 1 — pre-flight (spec Claim H, [R0-I5])
/// ONE `session.load_events_and_project()` call — NEVER the cached snap. The session
/// sees any unsaved in-memory residue, so the already-attested arm is the
/// defense-in-depth guard against the [R0-C1] double-batch. Arms, in order:
/// 1. zero live allocations → "No allocation to attest …", return.
/// 2. 2+ live allocations → "Multiple live allocations present …", return.
/// 3. `prior.timely_allocation_attested` → "Allocation already attested …", return.
/// 4. `SafeHarborUnconservable` on `prior_id` → "Allocation fails conservation …", return.
/// 5. NO `SafeHarborTimebar` on `prior_id` (already-effective) → "Allocation already
///    effective …", return.
/// 6. `SafeHarborTimebar` present → open the flow at the Info step.
fn open_safe_harbor_attest_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    // No-op when the snapshot is missing (mirrors every other opener).
    if app.snapshot.is_none() {
        return;
    }
    let session = match app.session.as_ref() {
        Some(s) => s,
        None => return,
    };
    let (events, state, _cfg) = match session.load_events_and_project() {
        Ok(t) => t,
        Err(e) => {
            app.status = Some(format!("Pre-flight load error: {e}"));
            return;
        }
    };

    // Build voided set; collect live (non-voided) SafeHarborAllocation events.
    let voided: std::collections::BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| {
            if let EventPayload::VoidDecisionEvent(v) = &e.payload {
                Some(v.target_event_id.clone())
            } else {
                None
            }
        })
        .collect();
    let live: Vec<(EventId, btctax_core::event::SafeHarborAllocation)> = events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .filter_map(|e| {
            if let EventPayload::SafeHarborAllocation(a) = &e.payload {
                Some((e.id.clone(), a.clone()))
            } else {
                None
            }
        })
        .collect();

    // Arms 1–2: live-allocation count.
    let (prior_id, prior_alloc) = match live.len() {
        0 => {
            app.status = Some(
                "No allocation to attest — quit the editor, then run: \
                 btctax reconcile safe-harbor-allocate"
                    .to_string(),
            );
            return;
        }
        1 => live.into_iter().next().expect("len == 1"),
        _ => {
            app.status = Some(
                "Multiple live allocations present — void the stale one (press 'v') \
                 before attesting"
                    .to_string(),
            );
            return;
        }
    };

    // Arm 3: already attested (defense-in-depth against the C1 double-batch:
    // the session-sourced load sees in-memory residue).
    if prior_alloc.timely_allocation_attested {
        app.status = Some("Allocation already attested — nothing to attest".to_string());
        return;
    }

    // Arms 4–5: blocker checks from the freshly-projected state.
    let blocked_with = |k: BlockerKind| {
        state
            .blockers
            .iter()
            .any(|b| b.event.as_ref() == Some(&prior_id) && b.kind == k)
    };
    if blocked_with(BlockerKind::SafeHarborUnconservable) {
        app.status = Some(
            "Allocation fails conservation — attestation cannot cure it; quit the \
             editor, then re-run: btctax reconcile safe-harbor-allocate"
                .to_string(),
        );
        return;
    }
    if !blocked_with(BlockerKind::SafeHarborTimebar) {
        app.status = Some("Allocation already effective — no attestation needed".to_string());
        return;
    }

    // Arm 6: SafeHarborTimebar present → open the flow at the Info step.
    app.safe_harbor_attest_flow = Some(SafeHarborAttestFlowState {
        prior_id,
        prior_alloc,
        step: SafeHarborAttestStep::Info,
    });
}

/// Dispatch a key press to the correct step handler while the attest flow is open.
///
/// Attest flow has only two steps: Info and TypedWord.
/// No separate modal — TypedWord IS the gate [R0-M4].
fn handle_safe_harbor_attest_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.safe_harbor_attest_flow.as_ref() {
        Some(f) => match &f.step {
            SafeHarborAttestStep::Info => 0u8,
            SafeHarborAttestStep::TypedWord { .. } => 1u8,
        },
        None => return,
    };
    match step {
        0 => handle_attest_info_key(app, key),
        _ => handle_attest_typed_word_key(app, key),
    }
}

/// Handle a key press while the attest-flow Info step is active.
///
/// Enter → advance to TypedWord step.
/// Esc → close flow entirely.
/// All other keys (incl. `q`) → swallowed (never fall through to Browse quit).
fn handle_attest_info_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.safe_harbor_attest_flow = None;
        }
        KeyCode::Enter => {
            if let Some(flow) = app.safe_harbor_attest_flow.as_mut() {
                flow.step = SafeHarborAttestStep::TypedWord {
                    buf: FieldBuffer::new(),
                    error: None,
                };
            }
        }
        _ => {}
    }
}

/// Handle a key press while the attest-flow TypedWord step is active.
///
/// All printable chars (incl. `q`) are consumed by the buffer — never fall through.
/// Backspace removes last char from buf.
/// Enter → validate "ATTEST"; on match calls `persist_safe_harbor_attest`:
///   `Ok((void_id, attest_id))` → re-project + derive_attest_status + close flow.
///   `Err(e)` → set `app.attest_save_failed = true` [R0-C1] + Err status + close flow.
/// On wrong typed word → set error on step but PRESERVE buf (do NOT clear) [R0-I7].
/// Esc → back to the Info step (one step per press — [I4]: TypedWord → Info → close).
fn handle_attest_typed_word_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(flow) = app.safe_harbor_attest_flow.as_mut() {
                flow.step = SafeHarborAttestStep::Info;
            }
            return;
        }
        KeyCode::Backspace => {
            if let Some(SafeHarborAttestFlowState {
                step: SafeHarborAttestStep::TypedWord { buf, .. },
                ..
            }) = app.safe_harbor_attest_flow.as_mut()
            {
                buf.pop_char();
            }
            return;
        }
        KeyCode::Char(c) => {
            if let Some(SafeHarborAttestFlowState {
                step: SafeHarborAttestStep::TypedWord { buf, error },
                ..
            }) = app.safe_harbor_attest_flow.as_mut()
            {
                buf.push_char(c);
                *error = None;
            }
            return;
        }
        KeyCode::Enter => {}
        _ => return,
    }

    // Enter: validate typed word.
    let (typed, prior_id, prior_alloc) = match app.safe_harbor_attest_flow.as_ref() {
        Some(SafeHarborAttestFlowState {
            step: SafeHarborAttestStep::TypedWord { buf, .. },
            prior_id,
            prior_alloc,
        }) => (
            buf.buf.as_str().trim().to_string(),
            prior_id.clone(),
            prior_alloc.clone(),
        ),
        _ => return,
    };

    if typed != "ATTEST" {
        // Wrong word: set the spec error, buffer PRESERVED (the user corrects with
        // Backspace) [R0-I7].
        if let Some(SafeHarborAttestFlowState {
            step: SafeHarborAttestStep::TypedWord { error, .. },
            ..
        }) = app.safe_harbor_attest_flow.as_mut()
        {
            *error = Some("type ATTEST (all caps) to confirm".to_string());
        }
        return;
    }

    // Correct word — call persist.
    let now = time::OffsetDateTime::now_utc();
    let save_result = match app.session.as_mut() {
        Some(s) => edit::persist::persist_safe_harbor_attest(s, prior_id, prior_alloc, now),
        None => return,
    };

    app.safe_harbor_attest_flow = None;

    match save_result {
        Ok((_void_id, attest_id)) => {
            let new_snap = {
                let session = app.session.as_ref().unwrap();
                btctax_tui::unlock::build_snapshot(session)
            };
            match new_snap {
                Ok((snap, _)) => {
                    let status = derive_attest_status(&snap, &attest_id);
                    app.snapshot = Some(snap);
                    app.status = Some(status);
                }
                Err(e) => {
                    app.status = Some(format!(
                        "Attested but re-projection failed ({e}) — restart to refresh"
                    ));
                }
            }
        }
        Err(e) => {
            app.attest_save_failed = true;
            app.status = Some(format!(
                "Save error: {e} — quit the editor now (the unsaved attestation is \
                 discarded on quit), then run: btctax reconcile safe-harbor-attest"
            ));
        }
    }
}

/// Derive the status string for a completed safe-harbor attest (spec D3 derive_attest_status).
///
/// Four arms in PRIORITY order, keyed ONLY to `new_attest_id` [R0-M10]: the voided
/// prior allocation keeps firing SafeHarborTimebar on ITS id every projection (stale
/// Advisory, harmless — allocation-targeted voids never enter the engine's `voided`
/// set). Never widen an arm to "no timebar anywhere". Blocker kinds outside the three
/// arms fall through to the clean arm (spec: "no timebar, no unconservable, no
/// conflict on new_attest_id").
fn derive_attest_status(snap: &btctax_tui::app::Snapshot, new_attest_id: &EventId) -> String {
    let has = |k: BlockerKind| {
        snap.state
            .blockers
            .iter()
            .any(|b| b.kind == k && b.event.as_ref() == Some(new_attest_id))
    };

    // Arm 1: conservation failed on the re-attested allocation (defensive).
    if has(BlockerKind::SafeHarborUnconservable) {
        return "ATTEST FAILED: allocation fires SafeHarborUnconservable — see Compliance; \
                the prior void and re-append both landed; quit the editor, then repair via CLI"
            .to_string();
    }

    // Arm 2: re-attested allocation still time-barred (unexpected).
    if has(BlockerKind::SafeHarborTimebar) {
        return "ATTEST SAVED but SafeHarborTimebar re-fired — check Compliance; \
                the allocation may not have cured the time-bar"
            .to_string();
    }

    // Arm 3: conflict on the new allocation (edge case).
    if has(BlockerKind::DecisionConflict) {
        return "ATTEST SAVED but DecisionConflict fired — check Compliance; vault \
                integrity may be affected; quit and run: btctax verify"
            .to_string();
    }

    // Arm 4: clean.
    format!(
        "Allocation attested (IRREVOCABLE, §7.4) — {}; quit and run btctax verify to confirm effectiveness",
        new_attest_id.canonical()
    )
}

// ── Safe-harbor-allocate flow (chunk 5, D1/D2/D4/D5/D6) ───────────────────────

/// Open the safe-harbor-allocate flow (`A`) — the CREATION counterpart to attest (`a`). Six
/// eligibility steps (spec D1), in order. The residue math is delegated ENTIRELY to
/// `Session::safe_harbor_residue` — no inline DB access (`conn(`/`load_all`/`project`) here. Of those,
/// only `conn(` is a KAT-G1 persist-only token; reads (`load_all`/`project`) are not gated.
fn open_safe_harbor_allocate_flow(app: &mut EditorApp) {
    // 1. Latch: refuse while a prior save left unrevertable residue.
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    // 2. Snapshot must be present (mirrors every other opener).
    if app.snapshot.is_none() {
        return;
    }

    // 3+4. Config gate (pre-2025 method declared+attested) and the pre-2025 residue — BOTH read the
    // HELD session. The residue helper reads config ONCE and returns the method it was computed under
    // [R0-M1/G5]; we thread that RETURNED method unchanged through flow→modal→persist.
    let session = match app.session.as_ref() {
        Some(s) => s,
        None => return,
    };
    let cfg = match session.config() {
        Ok(c) => c,
        Err(e) => {
            app.status = Some(format!("Pre-flight config error: {e}"));
            return;
        }
    };
    if !cfg.pre2025_method_attested {
        app.status = Some(
            "Declare your filed pre-2025 method first — quit the editor, then run: \
             btctax config --set-pre2025-method <m> --attest-pre2025-method"
                .to_string(),
        );
        return;
    }
    let (lots, pre2025_method) = match session.safe_harbor_residue() {
        Ok(t) => t,
        Err(e) => {
            app.status = Some(format!("Pre-flight residue error: {e}"));
            return;
        }
    };
    if lots.is_empty() {
        app.status = Some(
            "No pre-2025 lots to allocate (Path A applies; safe harbor unnecessary)".to_string(),
        );
        return;
    }

    // 5. No existing LIVE allocation (TUI-added guard the CLI lacks — prevents the chunk-3 "Multiple
    // live allocations present" tangle). Scan the cached snapshot's events for a non-voided
    // SafeHarborAllocation.
    let snap = app.snapshot.as_ref().unwrap();
    let voided: std::collections::BTreeSet<&EventId> = snap
        .events
        .iter()
        .filter_map(|e| {
            if let EventPayload::VoidDecisionEvent(v) = &e.payload {
                Some(&v.target_event_id)
            } else {
                None
            }
        })
        .collect();
    let live_exists = snap
        .events
        .iter()
        .filter(|e| !voided.contains(&e.id))
        .any(|e| matches!(e.payload, EventPayload::SafeHarborAllocation(_)));
    if live_exists {
        app.status = Some(
            "An allocation already exists — attest it with 'a', or void it with 'v' \
             before creating a new one"
                .to_string(),
        );
        return;
    }

    // 6. Open the flow at Preview. Totals + display rows are computed ONCE from the residue (the
    // `method` toggle is method-INDEPENDENT — G3 — so it never recomputes these).
    let total_sat: btctax_core::Sat = lots.iter().map(|l| l.sat).sum();
    let total_basis: btctax_core::Usd = lots.iter().map(|l| l.usd_basis).sum();
    let rows: Vec<AllocLotRow> = lots
        .iter()
        .map(|l| AllocLotRow {
            wallet: l.wallet.clone(),
            sat: l.sat,
            usd_basis: l.usd_basis,
            acquired_at: l.acquired_at,
            dual_loss_basis: l.dual_loss_basis,
            donor_acquired_at: l.donor_acquired_at,
        })
        .collect();
    app.safe_harbor_allocate_flow = Some(SafeHarborAllocateFlowState {
        lots,
        total_sat,
        total_basis,
        method: btctax_core::AllocMethod::ActualPosition,
        pre2025_method,
        list: TargetList::new(rows),
        step: SafeHarborAllocateStep::Preview,
    });
}

/// Capture the allocate Preview into the confirmation modal (revocable framing; NOT typed-word).
fn open_safe_harbor_allocate_modal(app: &mut EditorApp) {
    let modal = match app.safe_harbor_allocate_flow.as_ref() {
        Some(f) => SafeHarborAllocateModalState {
            lots: f.lots.clone(),
            total_sat: f.total_sat,
            total_basis: f.total_basis,
            method: f.method,
            pre2025_method: f.pre2025_method,
            lot_count: f.lots.len(),
        },
        None => return,
    };
    app.safe_harbor_allocate_modal = Some(modal);
}

/// Handle a key press while the allocate Preview is open (the flow's only step; spec D2).
///
/// `Tab`/`←`/`→` → cycle `method` (the recorded tag ONLY — the residue is method-independent, G3);
/// `↑`/`k`/`↓`/`j`/`g`/`G` scroll the lot list; `Enter` → open the confirm modal; `Esc` → close flow;
/// all else (incl. `q`) swallowed.
fn handle_safe_harbor_allocate_flow_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            if let Some(flow) = app.safe_harbor_allocate_flow.as_mut() {
                flow.method = cycle_alloc_method(flow.method);
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.safe_harbor_allocate_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.safe_harbor_allocate_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.safe_harbor_allocate_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.safe_harbor_allocate_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => open_safe_harbor_allocate_modal(app),
        KeyCode::Esc => {
            app.safe_harbor_allocate_flow = None;
        }
        _ => {
            // All other keys (including 'q') swallowed while the flow is open.
        }
    }
}

/// Handle a key press while the allocate confirmation modal is open (chunk 5, D4/D5).
///
/// On `Enter`: `persist_safe_harbor_allocate` (single append via `save_or_rollback`, no latch) →
/// re-project + `derive_allocate_status` + close modal & flow; on `Err(e)` close the modal and route
/// through `on_persist_error`. On `Esc`: close the modal only (back to Preview; nothing written). All
/// other keys are swallowed (blocking modal).
fn handle_safe_harbor_allocate_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (lots, method, pre2025_method) = match app.safe_harbor_allocate_modal.as_ref() {
                Some(m) => (m.lots.clone(), m.method, m.pre2025_method),
                None => return,
            };
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.safe_harbor_allocate_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_safe_harbor_allocate(
                    session,
                    lots,
                    method,
                    pre2025_method,
                    now,
                )
            };

            match save_result {
                Ok(new_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_allocate_status(&snap, &new_id);
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.safe_harbor_allocate_modal = None;
                    app.safe_harbor_allocate_flow = None;
                }
                Err(e) => {
                    app.safe_harbor_allocate_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.safe_harbor_allocate_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal).
        }
    }
}

/// Derive the status string for a completed safe-harbor-allocate (spec D6). Four arms in PRIORITY
/// order, keyed to `new_id` [R0-M10 discipline], mirroring `derive_attest_status`.
fn derive_allocate_status(snap: &btctax_tui::app::Snapshot, new_id: &EventId) -> String {
    let has = |k: BlockerKind| {
        snap.state
            .blockers
            .iter()
            .any(|b| b.kind == k && b.event.as_ref() == Some(new_id))
    };

    // Arm 1: conservation failed on the new allocation (defensive — the residue conserves by
    // construction, but fail closed).
    if has(BlockerKind::SafeHarborUnconservable) {
        return "Created, but SafeHarborUnconservable fired — see Compliance; void ('v') and re-run."
            .to_string();
    }

    // Arm 2: DecisionConflict on the new id OR the `event: None` "multiple effective" conflict
    // (resolve.rs) [R0-N2: the event:None read is a DELIBERATE exception to the new_id-only discipline
    // — the multiple-effective conflict has no single owning id; defensive, normally unreachable given
    // the step-5 guard].
    let multiple_effective = snap
        .state
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::DecisionConflict && b.event.is_none());
    if has(BlockerKind::DecisionConflict) || multiple_effective {
        return "Created, but conflicts with an existing effective allocation — void one ('v') \
                (see Compliance)."
            .to_string();
    }

    // Arm 3: SafeHarborTimebar on the new id — the EXPECTED arm (every fresh allocation at the current
    // date is timebarred → inert → voidable; G2).
    if has(BlockerKind::SafeHarborTimebar) {
        return "Allocation created (REVOCABLE, timebarred) — attest with 'a' to make it effective, \
                or void with 'v'."
            .to_string();
    }

    // Arm 4: clean — no timebar (unreachable for a fresh allocation at the current date, G2; kept
    // correct, NOT dead code). An immediately-effective allocation can no longer be voided (G1:
    // voidability tracks effectiveness).
    "Allocation created and EFFECTIVE (Path B) — it can no longer be voided; attest with 'a' to \
     lock §7.4."
        .to_string()
}

// ── Bulk link-transfer flow (bulk-link-transfer D3) ──────────────────────────

/// Open the bulk link-transfer flow from Browse (bulk-link-transfer D3).
///
/// Latch → snapshot → `pending_reconciliation` non-empty (else status). The dest pick-list (the full
/// `snap.events` wallet union) and the filter choices (distinct source wallets + years among the
/// pending outs) are read from `snap` DIRECTLY — KAT-G1-clean, like `open_link_transfer_flow`. Only
/// the PRICED preview (step 2→3) routes through `Session::bulk_link_transfer_plan` [R0-M4].
fn open_bulk_link_transfer_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };
    if snap.state.pending_reconciliation.is_empty() {
        app.status = Some("No pending outbound transfers to bulk-link".to_string());
        return;
    }
    let ev_idx = events_by_id(snap);

    // Dest pick-list: ALL distinct snap.events wallets (a dest may only ever appear inbound — so the
    // full event-wallet union, NOT just pending-out source wallets). BTreeSet dedups + sorts.
    let wallet_set: BTreeSet<btctax_core::WalletId> = snap
        .events
        .iter()
        .filter_map(|e| e.wallet.clone())
        .collect();
    let wallet_items: Vec<btctax_core::WalletId> = wallet_set.into_iter().collect();

    // Filter choices, from the enriched (date, source_wallet) of each pending out.
    let mut source_set: BTreeSet<btctax_core::WalletId> = BTreeSet::new();
    let mut year_set: BTreeSet<i32> = BTreeSet::new();
    for pt in &snap.state.pending_reconciliation {
        if let Some(ev) = ev_idx.get(&pt.event) {
            if let Some(w) = &ev.wallet {
                source_set.insert(w.clone());
            }
            let d = btctax_core::conventions::tax_date(ev.utc_timestamp, ev.original_tz);
            year_set.insert(d.year());
        }
    }
    let mut source_choices: Vec<Option<btctax_core::WalletId>> = vec![None]; // Any
    source_choices.extend(source_set.into_iter().map(Some));
    let mut year_choices: Vec<Option<i32>> = vec![None]; // All
    year_choices.extend(year_set.into_iter().map(Some));

    app.bulk_link_flow = Some(BulkLinkFlowState {
        step: BulkLinkStep::DestPick,
        wallet_list: TargetList::new(wallet_items),
        dest_buf: FieldBuffer::new(),
        dest: None,
        source_choices,
        source_idx: 0,
        year_choices,
        year_idx: 0,
        filter_focus: 0,
        preview: TargetList::new(Vec::new()),
        error: None,
    });
}

/// Dispatch to the correct sub-handler depending on `BulkLinkStep`.
fn handle_bulk_link_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.bulk_link_flow.as_ref() {
        Some(f) => match f.step {
            BulkLinkStep::DestPick => 0u8,
            BulkLinkStep::DestType => 1,
            BulkLinkStep::Filter => 2,
            BulkLinkStep::Preview => 3,
        },
        None => return,
    };
    match step {
        0 => handle_bulk_dest_pick_key(app, key),
        1 => handle_bulk_dest_type_key(app, key),
        2 => handle_bulk_filter_key(app, key),
        _ => handle_bulk_preview_key(app, key),
    }
}

/// Step 1 — destination pick-list. `k/j/g/G` scroll; `n` → typed-destination entry [Fork B]; Enter →
/// pick the highlighted wallet → Filter; Esc → close flow; `q` swallowed.
fn handle_bulk_dest_pick_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.wallet_list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.wallet_list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.wallet_list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.wallet_list.go_bottom();
            }
        }
        KeyCode::Char('n') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.dest_buf.set("");
                f.error = None;
                f.step = BulkLinkStep::DestType;
            }
        }
        KeyCode::Enter => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                if let Some(w) = f.wallet_list.selected().cloned() {
                    f.dest = Some(w);
                    f.error = None;
                    f.step = BulkLinkStep::Filter;
                }
            }
        }
        KeyCode::Esc => {
            app.bulk_link_flow = None;
        }
        KeyCode::Char('q') => {}
        _ => {}
    }
}

/// Step 1b — typed destination [Fork B]. Free text parsed by `eventref::parse_wallet_id` (the same
/// call `--to-wallet` uses), so a never-seen cold wallet (`self:cold-wallet`) is reachable. Enter →
/// parse: Ok → Filter; Err → error + stay. Esc → back to the pick-list. All chars (incl. `q`) type.
fn handle_bulk_dest_type_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let parsed = app
                .bulk_link_flow
                .as_ref()
                .map(|f| btctax_cli::eventref::parse_wallet_id(f.dest_buf.buf.trim()));
            if let Some(res) = parsed {
                match res {
                    Ok(w) => {
                        if let Some(f) = app.bulk_link_flow.as_mut() {
                            f.dest = Some(w);
                            f.error = None;
                            f.step = BulkLinkStep::Filter;
                        }
                    }
                    Err(e) => {
                        if let Some(f) = app.bulk_link_flow.as_mut() {
                            f.error = Some(format!("{e}"));
                        }
                    }
                }
            }
        }
        KeyCode::Esc => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.error = None;
                f.step = BulkLinkStep::DestPick;
            }
        }
        KeyCode::Backspace => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.dest_buf.pop_char();
            }
        }
        KeyCode::Char(c) => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.dest_buf.push_char(c);
            }
        }
        _ => {}
    }
}

/// Step 2 — filter. `k/j`/`↑/↓` move focus (source-wallet ⇄ time-frame); `←/→` cycle the focused
/// choice; Enter → recompute the PRICED plan → Preview; Esc → back to dest pick; `q` swallowed.
fn handle_bulk_filter_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.filter_focus = f.filter_focus.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.filter_focus = (f.filter_focus + 1).min(1);
            }
        }
        KeyCode::Left => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                if f.filter_focus == 0 {
                    let n = f.source_choices.len();
                    f.source_idx = (f.source_idx + n - 1) % n;
                } else {
                    let n = f.year_choices.len();
                    f.year_idx = (f.year_idx + n - 1) % n;
                }
            }
        }
        KeyCode::Right => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                if f.filter_focus == 0 {
                    let n = f.source_choices.len();
                    f.source_idx = (f.source_idx + 1) % n;
                } else {
                    let n = f.year_choices.len();
                    f.year_idx = (f.year_idx + 1) % n;
                }
            }
        }
        KeyCode::Enter => bulk_recompute_preview(app),
        KeyCode::Esc => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.error = None;
                f.step = BulkLinkStep::DestPick;
            }
        }
        KeyCode::Char('q') => {}
        _ => {}
    }
}

/// Recompute the priced plan from the current dest + filter selections and transition to Preview
/// (all rows checked). Empty plan → stay on Filter with an explanatory error. This is the ONLY
/// Session-helper call in the flow (KAT-G1: the opener reads `snap` directly).
fn bulk_recompute_preview(app: &mut EditorApp) {
    let (dest, filter) = match app.bulk_link_flow.as_ref() {
        Some(f) => {
            let dest = match f.dest.clone() {
                Some(d) => d,
                None => return,
            };
            let from_wallet = f.source_choices.get(f.source_idx).cloned().flatten();
            let frame = match f.year_choices.get(f.year_idx).copied().flatten() {
                Some(y) => btctax_cli::Frame::Year(y),
                None => btctax_cli::Frame::All,
            };
            (dest, btctax_cli::BulkFilter { frame, from_wallet })
        }
        None => return,
    };
    let plan = match app.session.as_ref() {
        Some(s) => s.bulk_link_transfer_plan(filter, dest),
        None => return,
    };
    match plan {
        Ok(plan) => {
            let items: Vec<BulkLinkRowItem> = plan
                .included
                .iter()
                .map(|r| BulkLinkRowItem {
                    out_event: r.out_event.clone(),
                    date: r.date,
                    source_wallet: r.source_wallet.clone(),
                    principal_sat: r.principal_sat,
                    usd_value: r.usd_value,
                    basis_usd: r.basis_usd,
                    checked: true,
                })
                .collect();
            if let Some(f) = app.bulk_link_flow.as_mut() {
                if items.is_empty() {
                    f.error = Some("No pending outbound transfers match this filter".to_string());
                } else {
                    f.error = None;
                    f.preview = TargetList::new(items);
                    f.step = BulkLinkStep::Preview;
                }
            }
        }
        Err(e) => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.error = Some(format!("Plan error: {e}"));
            }
        }
    }
}

/// Step 3 — per-row exclude checklist. `k/j/g/G` scroll; `Space`/`x` toggles the row's exclusion;
/// Enter → confirm modal over the CHECKED rows (refuse if none); Esc → back to Filter; `q` swallowed.
fn handle_bulk_preview_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.preview.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.preview.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.preview.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.preview.go_bottom();
            }
        }
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                if let Some(i) = f.preview.table_state.selected() {
                    if let Some(item) = f.preview.items.get_mut(i) {
                        item.checked = !item.checked;
                    }
                }
            }
        }
        KeyCode::Enter => open_bulk_link_modal(app),
        KeyCode::Esc => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.error = None;
                f.step = BulkLinkStep::Filter;
            }
        }
        KeyCode::Char('q') => {}
        _ => {}
    }
}

/// Capture the CHECKED preview rows into the confirmation modal. Empty selection → refuse (stay on
/// Preview with a "Nothing selected" error), never open the modal.
fn open_bulk_link_modal(app: &mut EditorApp) {
    let modal = {
        let f = match app.bulk_link_flow.as_ref() {
            Some(f) => f,
            None => return,
        };
        let dest = match f.dest.clone() {
            Some(d) => d,
            None => return,
        };
        let (count, total_sat, floor, missing) = bulk_checked_totals(&f.preview.items);
        if count == 0 {
            None
        } else {
            let out_events: Vec<EventId> = f
                .preview
                .items
                .iter()
                .filter(|i| i.checked)
                .map(|i| i.out_event.clone())
                .collect();
            Some(BulkLinkModalState {
                dest,
                out_events,
                count,
                total_sat,
                total_usd_value_floor: floor,
                missing_price_count: missing,
            })
        }
    };
    match modal {
        Some(m) => app.bulk_link_modal = Some(m),
        None => {
            if let Some(f) = app.bulk_link_flow.as_mut() {
                f.error = Some("Nothing selected — check at least one row".to_string());
            }
        }
    }
}

/// Handle a key press while the bulk-link confirmation modal is open (explicit confirm; NOT typed).
///
/// Enter → `persist_bulk_link_transfer` (batch append + single save, mid-batch rollback [R0-I1]) →
/// re-project + `derive_bulk_link_status` + close; `Err(e)` → close modal, route `on_persist_error`.
/// Esc → close modal only (back to Preview; nothing written). All else swallowed (blocking modal).
fn handle_bulk_link_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (out_events, dest) = match app.bulk_link_modal.as_ref() {
                Some(m) => (m.out_events.clone(), m.dest.clone()),
                None => return,
            };
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.bulk_link_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_bulk_link_transfer(
                    session,
                    out_events,
                    dest.clone(),
                    now,
                )
            };

            match save_result {
                Ok(n) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_bulk_link_status(&snap, n, &dest);
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.bulk_link_modal = None;
                    app.bulk_link_flow = None;
                }
                Err(e) => {
                    app.bulk_link_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.bulk_link_modal = None;
        }
        _ => {}
    }
}

/// Derive the post-apply status from RE-PROJECTED state (bulk-link-transfer D3). No blocker arm is
/// normally reachable (each append is the shape the single flow uses; a failed save rolls back clean
/// via `on_persist_error`).
fn derive_bulk_link_status(
    snap: &btctax_tui::app::Snapshot,
    n: usize,
    dest: &btctax_core::WalletId,
) -> String {
    let remaining = snap.state.pending_reconciliation.len();
    format!(
        "Linked {n} outflow(s) to {} as self-transfers ({remaining} pending outbound remain).",
        crate::edit::form::wallet_label(dest)
    )
}

// ── Resolve-conflict flow (chunk 4b, D3) ─────────────────────────────────────

/// One-line human summary of an imported payload (resolve-conflict list + modal). Covers the common
/// imported variants; anything else falls back to a compact debug form.
fn import_payload_summary(p: &EventPayload) -> String {
    match p {
        EventPayload::Acquire(a) => format!("Acquire {} sat, cost {}", a.sat, a.usd_cost),
        EventPayload::Income(i) => {
            let fmv = i
                .usd_fmv
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(no fmv)".to_string());
            format!("Income {} sat @ {}", i.sat, fmv)
        }
        EventPayload::Dispose(d) => {
            format!("Dispose {} sat, proceeds {}", d.sat, d.usd_proceeds)
        }
        EventPayload::TransferIn(t) => format!("TransferIn {} sat", t.sat),
        EventPayload::TransferOut(t) => format!("TransferOut {} sat", t.sat),
        EventPayload::Unclassified(u) => {
            format!(
                "Unclassified: {}",
                u.raw.chars().take(40).collect::<String>()
            )
        }
        other => format!("{other:?}"),
    }
}

/// Open the resolve-conflict flow from the Browse screen (chunk 4b, D3).
///
/// Pre-filter: events carrying `BlockerKind::ImportConflict` (Hard; fires ONLY while UNRESOLVED —
/// resolve.rs:386-401), so no extra exclusion is needed (inherently post-filtered). The blocker's
/// `.event` is the `ImportConflict` EventId; its payload names the `target` import event and the
/// `new_payload` proposed to supersede it. The two summaries are computed here (the CURRENT payload
/// lives at the TARGET id, a SEPARATE event; the NEW payload rides the conflict).
///
/// Empty filtered list → status "No unresolved import conflicts"; flow NOT opened [R0-M8].
fn open_resolve_conflict_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    let snap = match app.snapshot.as_ref() {
        Some(s) => s,
        None => return,
    };

    let ev_idx = events_by_id(snap);

    let mut items: Vec<ConflictItem> = snap
        .state
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::ImportConflict)
        .filter_map(|b| {
            let conflict_id = b.event.as_ref()?;
            let conflict_ev = ev_idx.get(conflict_id)?;
            let conflict = match &conflict_ev.payload {
                EventPayload::ImportConflict(c) => c,
                _ => return None,
            };
            let date = btctax_core::conventions::tax_date(
                conflict_ev.utc_timestamp,
                conflict_ev.original_tz,
            );
            // CURRENT payload lives at the TARGET id (a separate event, conflict_event != target).
            let current_summary = ev_idx
                .get(&conflict.target)
                .map(|e| import_payload_summary(&e.payload))
                .unwrap_or_else(|| "(target not found)".to_string());
            let new_summary = import_payload_summary(&conflict.new_payload);
            let new_fingerprint = conflict
                .new_fingerprint
                .0
                .chars()
                .take(8)
                .collect::<String>();
            Some(ConflictItem {
                conflict_event: conflict_id.clone(),
                target: conflict.target.clone(),
                date,
                new_fingerprint,
                current_summary,
                new_summary,
            })
        })
        .collect();
    items.sort_by_key(|i| i.date);

    if items.is_empty() {
        // R0-M8: empty filtered list never opens a flow.
        app.status = Some("No unresolved import conflicts".to_string());
        return;
    }

    app.resolve_conflict_flow = Some(ResolveConflictFlowState {
        list: TargetList::new(items),
        step: ResolveConflictStep::List,
    });
}

/// Dispatch to the correct sub-handler depending on `ResolveConflictStep`.
fn handle_resolve_conflict_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.resolve_conflict_flow.as_ref() {
        Some(f) => match &f.step {
            ResolveConflictStep::List => 0u8,
            ResolveConflictStep::Choose { .. } => 1u8,
        },
        None => return,
    };
    match step {
        0 => handle_rc_list_key(app, key),
        _ => handle_rc_choose_key(app, key),
    }
}

/// List step: Enter → Choose (default Accept). Esc → close flow. q → swallowed.
fn handle_rc_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .resolve_conflict_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                    flow.step = ResolveConflictStep::Choose {
                        conflict: item,
                        kind: ResolveKind::Accept, // default
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.resolve_conflict_flow = None;
        }
        _ => {
            // All other keys (including 'q') swallowed while flow is open [R0-I2].
        }
    }
}

/// Choose step: ←/→ or h/l toggle Accept ⇄ Reject (in-flow — NOT Browse `a`); Enter → modal;
/// Esc → back to List; q → swallowed.
fn handle_rc_choose_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Tab => {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                if let ResolveConflictStep::Choose { kind, .. } = &mut flow.step {
                    *kind = match *kind {
                        ResolveKind::Accept => ResolveKind::Reject,
                        ResolveKind::Reject => ResolveKind::Accept,
                    };
                }
            }
        }
        KeyCode::Enter => {
            if let Some(flow) = app.resolve_conflict_flow.as_ref() {
                if let ResolveConflictStep::Choose { conflict, kind } = &flow.step {
                    app.resolve_conflict_modal = Some(ResolveConflictModalState {
                        conflict_event: conflict.conflict_event.clone(),
                        target: conflict.target.clone(),
                        kind: *kind,
                        old_summary: conflict.current_summary.clone(),
                        new_summary: conflict.new_summary.clone(),
                    });
                }
            }
        }
        KeyCode::Esc => {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                flow.step = ResolveConflictStep::List;
            }
        }
        _ => {
            // All other keys (including 'q') swallowed [R0-I2].
        }
    }
}

/// Handle a key press while the resolve-conflict confirmation modal is open (chunk 4b, D3).
///
/// Enter → `persist_resolve_conflict(session, conflict_event, kind, now)` → re-project + status +
///   close. `Err(e)` → close modal, route through `on_persist_error`.
/// Esc → close modal only (back to the Choose step; nothing written).
fn handle_resolve_conflict_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (conflict_event, kind) = match app.resolve_conflict_modal.as_ref() {
                Some(m) => (m.conflict_event.clone(), m.kind),
                None => return,
            };
            let now = time::OffsetDateTime::now_utc();

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.resolve_conflict_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_resolve_conflict(
                    session,
                    conflict_event.clone(),
                    kind,
                    now,
                )
            };

            match save_result {
                Ok(_decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status =
                                derive_resolve_conflict_status(&snap, &conflict_event, kind);
                            app.snapshot = Some(snap);
                            app.status = Some(status);
                        }
                        Err(e) => {
                            app.status = Some(format!(
                                "Saved but re-projection failed ({e}) — restart to refresh"
                            ));
                        }
                    }
                    app.resolve_conflict_modal = None;
                    app.resolve_conflict_flow = None;
                }
                Err(e) => {
                    app.resolve_conflict_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.resolve_conflict_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal).
        }
    }
}

/// Derive the status string from RE-PROJECTED state after a resolve-conflict save (chunk 4b, D3).
///
/// The pre-filter removes already-resolved conflicts and a failed save rolls back clean, so no
/// `DecisionConflict` retry arm is reachable. On success the target's `ImportConflict` blocker
/// clears; a defensive re-check reports the (unreachable) case where it somehow persists.
fn derive_resolve_conflict_status(
    snap: &btctax_tui::app::Snapshot,
    conflict_event: &EventId,
    kind: ResolveKind,
) -> String {
    let verb = match kind {
        ResolveKind::Accept => "accepted",
        ResolveKind::Reject => "rejected",
    };
    let still_unresolved =
        snap.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::ImportConflict && b.event.as_ref() == Some(conflict_event)
        });
    if still_unresolved {
        return format!(
            "Resolution recorded for {} but the import-conflict blocker persists — see Compliance.",
            conflict_event.canonical()
        );
    }
    format!(
        "Conflict {} {verb}; import-conflict resolved.",
        conflict_event.canonical()
    )
}

// ── Optimize-accept flow (chunk 4b, D4) ──────────────────────────────────────

/// Open the optimize-accept flow from the Browse screen (chunk 4b, D4).
///
/// Opener = a READ-ONLY optimizer RECOMPUTE (KAT-G1-clean) via the additive
/// `Session::optimize_proposal(year, now)` — never trusts a stale proposal (NFR4), never opens a
/// second `Session` (a second open would deadlock on the held VaultLock; `cmd::optimize::accept` is
/// forbidden). On `Err(e)` shows the consult remedy and no-opens.
///
/// Pre-filter (`filter_optimize_candidates`): keep rows where `proposed != current`,
/// `persistable != ForbiddenBroker2027`, AND the disposal has NO live `LotSelection` (the MANDATORY
/// duplicate guard). Empty filtered list → status + NO open [R0-M3].
fn open_optimize_accept_flow(app: &mut EditorApp) {
    if let Some(s) = app.residue_latch_status() {
        app.status = Some(s);
        return;
    }
    if app.snapshot.is_none() {
        return;
    }
    let year = app.selected_year;
    let now = time::OffsetDateTime::now_utc();

    // READ-ONLY recompute via the additive btctax-cli helper.
    let proposal = {
        let session = match app.session.as_ref() {
            Some(s) => s,
            None => return,
        };
        match session.optimize_proposal(year, now) {
            Ok(p) => p,
            Err(e) => {
                app.status = Some(format!(
                    "{e} — quit the editor and run: btctax optimize consult"
                ));
                return;
            }
        }
    };

    // Duplicate guard: disposal_events of non-voided LotSelection decisions (owned set for the helper).
    let (items, delta, approximate) = {
        let snap = app.snapshot.as_ref().unwrap();
        let voided: std::collections::BTreeSet<&EventId> = snap
            .events
            .iter()
            .filter_map(|e| {
                if let EventPayload::VoidDecisionEvent(v) = &e.payload {
                    Some(&v.target_event_id)
                } else {
                    None
                }
            })
            .collect();
        let already_selected: std::collections::BTreeSet<EventId> = snap
            .events
            .iter()
            .filter(|e| !voided.contains(&e.id))
            .filter_map(|e| {
                if let EventPayload::LotSelection(ls) = &e.payload {
                    Some(ls.disposal_event.clone())
                } else {
                    None
                }
            })
            .collect();
        (
            filter_optimize_candidates(&proposal.per_disposal, &already_selected),
            proposal.delta,
            proposal.approximate,
        )
    };

    if items.is_empty() {
        // R0-M3: empty filtered list never opens a flow.
        app.status = Some("No persistable optimizer improvements available".to_string());
        return;
    }

    app.optimize_accept_flow = Some(OptimizeAcceptFlowState {
        list: TargetList::new(items),
        step: OptimizeAcceptStep::List,
        delta,
        approximate,
    });
}

/// Build the optimize-accept confirmation modal from a chosen candidate.
fn open_optimize_accept_modal(
    app: &mut EditorApp,
    item: OptimizeCandidateItem,
    attestation: Option<String>,
) {
    let pick_count = item.picks.len();
    let total_sat: btctax_core::Sat = item.picks.iter().map(|p| p.sat).sum();
    let basis_label = optimize_basis_label(item.persistable);
    app.optimize_accept_modal = Some(OptimizeAcceptModalState {
        disposal: item.disposal,
        picks: item.picks,
        pick_count,
        total_sat,
        attestation,
        basis_label,
    });
}

/// Dispatch to the correct sub-handler depending on `OptimizeAcceptStep`.
fn handle_optimize_accept_flow_key(app: &mut EditorApp, key: KeyEvent) {
    let step = match app.optimize_accept_flow.as_ref() {
        Some(f) => match &f.step {
            OptimizeAcceptStep::List => 0u8,
            OptimizeAcceptStep::AttestText { .. } => 1u8,
        },
        None => return,
    };
    match step {
        0 => handle_oa_list_key(app, key),
        _ => handle_oa_attest_text_key(app, key),
    }
}

/// List step: Enter → branch on persistability (`ContemporaneousNow` → modal; `NeedsAttestation` →
/// attestation-text step). Esc → close flow. q → swallowed.
fn handle_oa_list_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                flow.list.scroll_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                flow.list.scroll_down();
            }
        }
        KeyCode::Char('g') => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                flow.list.go_top();
            }
        }
        KeyCode::Char('G') => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                flow.list.go_bottom();
            }
        }
        KeyCode::Enter => {
            let selected = app
                .optimize_accept_flow
                .as_ref()
                .and_then(|f| f.list.selected())
                .cloned();
            if let Some(item) = selected {
                match item.persistable {
                    Persistability::ContemporaneousNow => {
                        open_optimize_accept_modal(app, item, None);
                    }
                    Persistability::NeedsAttestation => {
                        if let Some(flow) = app.optimize_accept_flow.as_mut() {
                            flow.step = OptimizeAcceptStep::AttestText {
                                item,
                                buf: FieldBuffer::with_cap(FREETEXT_CAP),
                                error: None,
                            };
                        }
                    }
                    Persistability::ForbiddenBroker2027 => {
                        // Unreachable — pre-filtered out of the candidate list. Defensive no-op.
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.optimize_accept_flow = None;
        }
        _ => {
            // All other keys (including 'q') swallowed while flow is open [R0-I2].
        }
    }
}

/// Attestation-text step (NeedsAttestation only): free-text entry of the contemporaneous-ID
/// statement. Enter → non-empty required → modal. Esc → back to List. All printable chars (incl.
/// 'q') are consumed by the buffer.
fn handle_oa_attest_text_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                flow.step = OptimizeAcceptStep::List;
            }
        }
        KeyCode::Backspace => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                if let OptimizeAcceptStep::AttestText { buf, .. } = &mut flow.step {
                    buf.pop_char();
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                if let OptimizeAcceptStep::AttestText { buf, error, .. } = &mut flow.step {
                    buf.push_char(c);
                    *error = None;
                }
            }
        }
        KeyCode::Enter => {
            // [R0-M4] "empty" = len==0 (checked before trimming). Store the text VERBATIM (CLI parity).
            let (item, is_empty, text) = match app.optimize_accept_flow.as_ref() {
                Some(f) => match &f.step {
                    OptimizeAcceptStep::AttestText { item, buf, .. } => {
                        (item.clone(), buf.is_empty(), buf.buf.clone())
                    }
                    _ => return,
                },
                None => return,
            };
            if is_empty {
                if let Some(flow) = app.optimize_accept_flow.as_mut() {
                    if let OptimizeAcceptStep::AttestText { error, .. } = &mut flow.step {
                        *error = Some(
                            "attestation text is required (the contemporaneous-ID statement)"
                                .to_string(),
                        );
                    }
                }
                return;
            }
            open_optimize_accept_modal(app, item, Some(text));
        }
        _ => {
            // Non-text keys swallowed.
        }
    }
}

/// Handle a key press while the optimize-accept confirmation modal is open (chunk 4b, D4).
///
/// Enter → `persist_optimize_accept(session, disposal, picks, attestation, made, now)` (dual-write:
///   LotSelection + optional attest row) → re-project + status + close. `Err(e)` → close modal,
///   route through `on_persist_error`.
/// Esc → close modal only (back to the prior step; nothing written).
fn handle_optimize_accept_modal_key(app: &mut EditorApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let (disposal, picks, attestation, basis_label) =
                match app.optimize_accept_modal.as_ref() {
                    Some(m) => (
                        m.disposal.clone(),
                        m.picks.clone(),
                        m.attestation.clone(),
                        m.basis_label,
                    ),
                    None => return,
                };
            let pick_count = picks.len();
            let attested = attestation.is_some();
            let now = time::OffsetDateTime::now_utc();
            let made = btctax_core::conventions::tax_date(now, time::UtcOffset::UTC);

            let save_result = {
                let session = match app.session.as_mut() {
                    Some(s) => s,
                    None => {
                        app.optimize_accept_modal = None;
                        return;
                    }
                };
                crate::edit::persist::persist_optimize_accept(
                    session,
                    disposal.clone(),
                    picks,
                    attestation,
                    made,
                    now,
                )
            };

            match save_result {
                Ok(decision_id) => {
                    let new_snap = {
                        let session = app.session.as_ref().unwrap();
                        btctax_tui::unlock::build_snapshot(session)
                    };
                    match new_snap {
                        Ok((snap, _)) => {
                            let status = derive_optimize_accept_status(
                                &snap,
                                &disposal,
                                &decision_id,
                                pick_count,
                                basis_label,
                                attested,
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
                    app.optimize_accept_modal = None;
                    app.optimize_accept_flow = None;
                }
                Err(e) => {
                    app.optimize_accept_modal = None;
                    app.on_persist_error(e);
                }
            }
        }
        KeyCode::Esc => {
            app.optimize_accept_modal = None;
        }
        _ => {
            // All other keys swallowed (blocking modal).
        }
    }
}

/// Derive the status string from RE-PROJECTED state after an optimize-accept save (chunk 4b, D4).
///
/// Three arms (spec D4): (1) `DecisionConflict` on `decision_id` (duplicate LotSelection — only via a
/// failed-save race) → NEITHER-applies/method-order (reuses the select-lots arm-1 wording); (2)
/// `LotSelectionInvalid` for the disposal → saved-but-invalid; (3) clean → recorded summary
/// (+ "; attestation recorded" when attested).
fn derive_optimize_accept_status(
    snap: &btctax_tui::app::Snapshot,
    disposal: &EventId,
    decision_id: &EventId,
    pick_count: usize,
    basis_label: &str,
    attested: bool,
) -> String {
    // Arm 1: DecisionConflict attributed to the decision_id.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(decision_id) {
            return format!(
                "Saved, but DecisionConflict fired — neither selection applies (method order \
                 governs); clear with Void flow (press 'v'), or quit the editor and run: \
                 btctax reconcile void {} (see Compliance)",
                decision_id.canonical()
            );
        }
    }

    // Arm 2: LotSelectionInvalid attributed to the disposal.
    for b in &snap.state.blockers {
        if b.kind == BlockerKind::LotSelectionInvalid && b.event.as_ref() == Some(disposal) {
            return "Optimizer selection saved but invalid — see Compliance; void ('v') and retry."
                .to_string();
        }
    }

    // Arm 3: clean.
    let mut s = format!(
        "Optimizer selection recorded for {} — {pick_count} lot(s); {basis_label}",
        disposal.canonical()
    );
    if attested {
        s.push_str("; attestation recorded");
    }
    s.push('.');
    s
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
            // wallet MUST be set: fold.rs Op::IncomeInbound and Op::GiftReceived fire
            // FmvMissing and return early when wallet is None, so no lot or income record
            // is ever created. This mirrors the requirement documented in seed_transfer_out_vault.
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: ti_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
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
        // (5) [save-rollback] the failed save left NO residue: the in-memory log is reverted to pre.
        let mid_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();
        assert_eq!(
            mid_len, pre_len,
            "S2: rollback must revert the in-memory append (no residue after a failed save)"
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

        // [save-rollback] capture status before drop(app).
        let status_after_retry = app.status.clone().unwrap_or_default();

        // Retry outcome: the failed save left NO residue, so the retry appends EXACTLY ONE decision
        // (not two) and fires NO DecisionConflict — this SUPERSEDES the old R0-I1 residue+conflict
        // behavior (an intentional supersession, not a regression).
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
            1,
            "S2: retry after a rolled-back save must append EXACTLY ONE decision (no residue); got: {}",
            new_decisions.len()
        );

        // The single row round-trips to the ClassifyInbound::Income(Mining) payload.
        let p0: EventPayload = serde_json::from_str(&new_decisions[0].payload_json).unwrap();
        assert!(
            matches!(
                &p0,
                EventPayload::ClassifyInbound(ci)
                    if matches!(&ci.as_, InboundClass::Income { kind: IncomeKind::Mining, .. })
            ),
            "S2: payload must be ClassifyInbound::Income(Mining); got: {:?}",
            p0
        );

        // Clean retry: NO DecisionConflict anywhere, and the success status does not mention one.
        let snap_session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&snap_session).unwrap();
        assert!(
            !snap
                .state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::DecisionConflict),
            "S2: a clean retry must fire NO DecisionConflict; blockers: {:?}",
            snap.state.blockers
        );
        assert!(
            !status_after_retry.contains("DecisionConflict"),
            "S2: clean-retry status must not mention DecisionConflict; got: {status_after_retry:?}"
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

        // Release the vault lock before the CLI read-back call.
        drop(session2);

        // Step 4 (spec KAT-E2E-CI): CLI read-back via cmd::inspect::verify.
        // UnknownBasisInbound for the classified event must be absent from the hard-blocker list.
        let vr = btctax_cli::cmd::inspect::verify(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let ubi_absent = !vr.hard.iter().any(|b| {
            b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
        });
        assert!(
            ubi_absent,
            "E2E-CI step 4: cmd::inspect::verify must NOT list UnknownBasisInbound for the \
             classified TransferIn; hard blockers: {:?}",
            vr.hard
        );
    }

    // ── KAT-CI-ST — self-transfer-in variant Tab-cycle + form transition ─────

    /// Cycle A (Task 3): the picker Tab-cycles Income → GiftReceived → SelfTransferMine → Income,
    /// and Enter on the SelfTransferMine picker opens the SelfTransferForm step.
    #[test]
    fn kat_ci_st_picker_cycles_and_opens_self_transfer_form() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-ci-st-cycle";

        seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        handle_key(&mut app, press(KeyCode::Char('c'))); // open flow (List)
        handle_key(&mut app, press(KeyCode::Enter)); // List → VariantPicker (Income)
                                                     // Tab twice: Income → GiftReceived → SelfTransferMine.
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Tab));
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::VariantPicker {
                    variant: InboundVariant::SelfTransferMine,
                    ..
                })
            ),
            "two Tabs must reach SelfTransferMine"
        );
        // Third Tab wraps back to Income.
        handle_key(&mut app, press(KeyCode::Tab));
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::VariantPicker {
                    variant: InboundVariant::Income,
                    ..
                })
            ),
            "third Tab must wrap to Income"
        );
        // Back to SelfTransferMine, then Enter → SelfTransferForm.
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::SelfTransferForm { .. })
            ),
            "Enter on SelfTransferMine picker must open SelfTransferForm"
        );
        // Esc → back to VariantPicker (SelfTransferMine retained).
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::VariantPicker {
                    variant: InboundVariant::SelfTransferMine,
                    ..
                })
            ),
            "Esc on SelfTransferForm returns to the picker on SelfTransferMine"
        );
    }

    /// KAT-E2E-ST — full self-transfer-in flow with DEFAULT basis: creates a non-taxable lot, clears
    /// `UnknownBasisInbound`, and fires the `SelfTransferInboundZeroBasis` advisory. Also asserts the
    /// modal renders the self-transfer copy, and 'q' in the (empty) basis field is swallowed.
    #[test]
    fn kat_e2e_ci_classify_inbound_self_transfer_default_basis() {
        use btctax_core::persistence::load_all_ordered;
        use ratatui::{backend::TestBackend, Terminal};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-st-pass";

        let ti_id = seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // Seed produces UnknownBasisInbound.
        assert!(
            app.snapshot
                .as_ref()
                .unwrap()
                .state
                .blockers
                .iter()
                .any(|b| {
                    b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
                }),
            "E2E-ST: seed must produce UnknownBasisInbound"
        );
        let pre_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();

        // c → Enter (List→Picker) → Tab,Tab (→ SelfTransferMine) → Enter (→ SelfTransferForm)
        handle_key(&mut app, press(KeyCode::Char('c')));
        handle_key(&mut app, press(KeyCode::Enter));
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.classify_inbound_flow.as_ref().map(|f| &f.step),
                Some(ClassifyInboundStep::SelfTransferForm { .. })
            ),
            "E2E-ST: must be on SelfTransferForm"
        );

        // 'q' at the (text) basis field is swallowed (inserted, not quit) [R2-N1]; backspace it out.
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(!app.should_quit, "E2E-ST: 'q' in basis field must not quit");
        assert!(app.classify_inbound_flow.is_some());
        handle_key(&mut app, press(KeyCode::Backspace));

        // Leave BOTH fields empty (defaults). Enter → modal.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.classify_inbound_modal.is_some(),
            "E2E-ST: Enter on empty SelfTransferForm must open the modal (fields optional)"
        );

        // Modal renders the self-transfer copy + the canonical target id.
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains(&ti_id.canonical()),
            "E2E-ST: modal must show the canonical EventId; rendered: {rendered}"
        );
        assert!(
            rendered.contains("SelfTransferMine") || rendered.contains("my own coins"),
            "E2E-ST: modal must identify the self-transfer classification; rendered: {rendered}"
        );

        // Confirm → save + re-project.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.classify_inbound_modal.is_none(),
            "E2E-ST: modal closes after confirm"
        );
        assert!(
            app.classify_inbound_flow.is_none(),
            "E2E-ST: flow closes after confirm"
        );

        // Exactly one decision row appended.
        let post_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();
        assert_eq!(
            post_len,
            pre_len + 1,
            "E2E-ST: exactly one decision appended"
        );

        // Reopen + project → UnknownBasisInbound gone; a non-taxable lot exists; advisory present.
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();
        assert!(
            !snap.state.blockers.iter().any(|b| {
                b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
            }),
            "E2E-ST: UnknownBasisInbound must be cleared"
        );
        assert_eq!(snap.state.lots.len(), 1, "E2E-ST: a lot is created");
        assert_eq!(
            snap.state.lots[0].usd_basis,
            rust_decimal_macros::dec!(0),
            "E2E-ST: default basis is $0"
        );
        assert!(
            !snap.state.lots[0].basis_pending,
            "E2E-ST: $0 basis never gates"
        );
        assert!(
            snap.state.income_recognized.is_empty(),
            "E2E-ST: non-taxable"
        );
        assert!(
            snap.state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::SelfTransferInboundZeroBasis),
            "E2E-ST: the zero-basis advisory must fire for the default-basis path"
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
        assert!(
            status.contains("'v'"),
            "FMV-MISSING: status must mention \"'v'\" (TUI void flow hint); got: {status}"
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
        assert!(
            status.contains("'v'"),
            "GIFT-UNK: status must mention \"'v'\" (TUI void flow hint); got: {status}"
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
        assert!(
            status.contains("'v'"),
            "PRICE-GAP: status must mention \"'v'\" (TUI void flow hint); got: {status}"
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
        use btctax_core::Usd;
        use ratatui::{backend::TestBackend, Terminal};
        use rust_decimal_macros::dec;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-ro-pass";

        // WB-I3 fix: use lot-seeded helper so the Dispose arm produces a covered Disposal
        // (spec step 3 requires Disposal{kind=Sell, proceeds=640.00} in state.disposals).
        // seed_transfer_out_vault_with_lots seeds an Acquire lot (500_000 sat, 1 day before
        // TransferOut) so FIFO yields a fully covered single-leg Disposal.
        let to_id = seed_transfer_out_vault_with_lots(&vault, &key, pp_str, 500_000);

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

        // WB-I3 step 3: Disposal with kind=Sell must appear in state.disposals.
        // The lot-seeded vault provides 500_000 sat; the Acquire covers the full TransferOut,
        // so the projection produces a single-leg Disposal with proceeds = entered 640.00.
        let disposal = snap.state.disposals.iter().find(|d| d.event == to_id);
        assert!(
            disposal.is_some(),
            "E2E-RO: Disposal must appear in state.disposals after reclassify-sell \
             (lot-seeded vault ensures full coverage)"
        );
        let d = disposal.unwrap();
        assert_eq!(
            d.kind,
            DisposeKind::Sell,
            "E2E-RO: Disposal.kind must be Sell"
        );
        // Single lot covers all 500_000 sat; no fee → net proceeds == gross proceeds == 640.00.
        let total_proceeds: Usd = d.legs.iter().map(|l| l.proceeds).sum();
        assert_eq!(
            total_proceeds,
            dec!(640.00),
            "E2E-RO: Disposal total net proceeds must equal the entered 640.00; got: {total_proceeds}"
        );

        // Check the decision row on disk.
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

        // Release the vault lock before the CLI read-back call.
        drop(session2);

        // Step 4 (spec KAT-E2E-RO): CLI read-back via cmd::inspect::report.
        // A fresh cmd::inspect::report projection must show the Sell Disposal.
        let cli_state =
            btctax_cli::cmd::inspect::report(&vault, &Passphrase::new(pp_str.into()), None)
                .unwrap();
        let cli_disposal = cli_state.disposals.iter().find(|d| d.event == to_id);
        assert!(
            cli_disposal.is_some() && cli_disposal.unwrap().kind == DisposeKind::Sell,
            "E2E-RO step 4: cmd::inspect::report must project a Sell Disposal for the reclassified outflow; \
             got: {:?}", cli_disposal
        );
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
        // (5) [save-rollback] no residue: the in-memory log is reverted to pre after the failed save.
        let mid_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();
        assert_eq!(
            mid_len, pre_len,
            "S2-RO: rollback must revert the in-memory append (no residue after a failed save)"
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

        // [save-rollback] capture status before drop(app).
        let status_after_retry = app.status.clone().unwrap_or_default();

        // Retry outcome: the failed save left NO residue, so the retry appends EXACTLY ONE decision
        // (not two) and fires NO DecisionConflict — SUPERSEDES the old R0-I1 residue+conflict behavior.
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
            1,
            "S2-RO: retry after a rolled-back save must append EXACTLY ONE decision (no residue); got: {}",
            new_decisions.len()
        );

        // The single row round-trips to the ReclassifyOutflow(Sell) payload.
        let p0: EventPayload = serde_json::from_str(&new_decisions[0].payload_json).unwrap();
        assert!(
            matches!(
                &p0,
                EventPayload::ReclassifyOutflow(ro)
                    if matches!(&ro.as_, OutflowClass::Dispose { kind: DisposeKind::Sell })
            ),
            "S2-RO: payload must be ReclassifyOutflow(Sell); got: {:?}",
            p0
        );

        // Clean retry: NO DecisionConflict anywhere, and the success status does not mention one.
        let snap_session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&snap_session).unwrap();
        assert!(
            !snap
                .state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::DecisionConflict),
            "S2-RO: a clean retry must fire NO DecisionConflict; blockers: {:?}",
            snap.state.blockers
        );
        assert!(
            !status_after_retry.contains("DecisionConflict"),
            "S2-RO: clean-retry status must not mention DecisionConflict; got: {status_after_retry:?}"
        );
    }

    // ── Seed helpers for reclassify-income and set-fmv tests ─────────────────

    /// Seed a vault with an Income event (FmvStatus::PriceDataset, Reward, 100_000 sat,
    /// fmv=$30_000) plus a MethodElection, and return the income EventId.
    fn seed_income_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> btctax_core::EventId {
        use btctax_core::event::{EventPayload, FmvStatus, Income, IncomeKind, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let income_id = EventId::import(Source::River, SourceRef::new("e2e-income-1"));
        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: income_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: Some(dec!(30_000)),
                    fmv_status: FmvStatus::PriceDataset,
                    kind: IncomeKind::Reward,
                    business: false,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                // Valid standing order: effective_from >= made-date (2025-05-23) and
                // >= TRANSITION_DATE (2025-01-01) — a back-dated election fires the
                // Hard MethodElectionBackdated blocker and would gate compute_tax_year.
                effective_from: date!(2025 - 06 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            btctax_core::persistence::append_decision(session.conn(), p, now, UtcOffset::UTC, None)
                .unwrap();
            session.save().unwrap();
        }
        income_id
    }

    /// Seed a vault with an Income event (FmvStatus::Missing, Staking, 100_000 sat, no fmv)
    /// plus a MethodElection, and return the income EventId.
    fn seed_income_fmv_missing_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> btctax_core::EventId {
        use btctax_core::event::{EventPayload, FmvStatus, Income, IncomeKind, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let income_id = EventId::import(Source::River, SourceRef::new("e2e-fmv-miss-1"));
        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: income_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: None,
                    fmv_status: FmvStatus::Missing,
                    kind: IncomeKind::Staking,
                    business: false,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            let now = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();
            let p = EventPayload::MethodElection(btctax_core::event::MethodElection {
                // Valid standing order: effective_from >= made-date (2025-05-23) and
                // >= TRANSITION_DATE (2025-01-01) — a back-dated election fires the
                // Hard MethodElectionBackdated blocker and would gate compute_tax_year.
                effective_from: date!(2025 - 06 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            btctax_core::persistence::append_decision(session.conn(), p, now, UtcOffset::UTC, None)
                .unwrap();
            session.save().unwrap();
        }
        income_id
    }

    // ── KAT-C2c — cancel path: reclassify-income bytes unchanged; q swallowed ──
    //
    // Spec sequence [I4 — one step back per Esc press]:
    // r → List; Enter → FieldForm; Tab business→true; Enter → modal;
    // Esc → modal closes (FieldForm still open); Esc → back to List;
    // Esc → flow closes. 'q' swallowed at EVERY flow step + at the modal.

    #[test]
    fn kat_c2c_cancel_path_vault_bytes_unchanged_reclassify_income() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2c-ri-pass";

        seed_income_vault(&vault, &key, pp_str);
        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // 'r' → open RI flow at List
            handle_key(&mut app, press(KeyCode::Char('r')));
            assert!(app.reclassify_income_flow.is_some(), "C2c: flow must open");

            // 'q' at List is swallowed [R0-I2]
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2c: 'q' at List must be swallowed");
            assert!(
                app.reclassify_income_flow.is_some(),
                "C2c: flow must remain open after 'q' at List"
            );

            // Enter → select first item (move to FieldForm)
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.reclassify_income_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyIncomeStep::FieldForm { .. })
                ),
                "C2c: must enter FieldForm after Enter on list"
            );

            // 'q' at FieldForm is swallowed (both rows are pickers — no text buffer)
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2c: 'q' at FieldForm must be swallowed");
            assert!(app.reclassify_income_flow.is_some());

            // Tab to set business=true (otherwise Enter would error)
            handle_key(&mut app, press(KeyCode::Tab));

            // Enter → validate → modal opens
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.reclassify_income_modal.is_some(),
                "C2c: modal must open after valid Enter"
            );

            // 'q' while modal open is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2c: 'q' at modal must be swallowed");
            assert!(
                app.reclassify_income_modal.is_some(),
                "C2c: modal must stay open after 'q'"
            );

            // Esc → cancel modal (no write); FieldForm still open [I4]
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.reclassify_income_modal.is_none(),
                "C2c: modal must close on Esc"
            );
            assert!(
                matches!(
                    app.reclassify_income_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyIncomeStep::FieldForm { .. })
                ),
                "C2c: Esc on modal must keep FieldForm open"
            );

            // Esc → close FieldForm, back to List [I4 — one step back per press]
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.reclassify_income_flow.as_ref().map(|f| &f.step),
                    Some(ReclassifyIncomeStep::List)
                ),
                "C2c: Esc on FieldForm must go back to List"
            );

            // 'q' at List (again) still swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2c: 'q' at List (second time) must be swallowed"
            );

            // Esc → close flow
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.reclassify_income_flow.is_none(), "C2c: flow must close");
            assert!(!app.should_quit, "C2c: Esc on List must NOT quit");
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2c: vault bytes must be unchanged after Esc cancel path"
        );
    }

    // ── KAT-C2d — cancel path: set-fmv bytes unchanged; q swallowed ──────────
    //
    // Spec sequence [I4 — one step back per Esc press]:
    // f → List; Enter → FieldForm; type FMV; Enter → modal;
    // Esc → modal closes (FieldForm still open); Esc → back to List;
    // Esc → flow closes. 'q' swallowed at each step ('q' at the text field
    // inserts into the buffer per the 2a R2-N1 discipline — it never quits).

    #[test]
    fn kat_c2d_cancel_path_vault_bytes_unchanged_set_fmv() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2d-sfmv-pass";

        seed_income_fmv_missing_vault(&vault, &key, pp_str);
        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // 'f' → open Set-FMV flow at List
            handle_key(&mut app, press(KeyCode::Char('f')));
            assert!(app.set_fmv_flow.is_some(), "C2d: flow must open");

            // 'q' at List is swallowed [R0-I2]
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2d: 'q' at List must be swallowed");
            assert!(
                app.set_fmv_flow.is_some(),
                "C2d: flow must remain open after 'q' at List"
            );

            // Enter → select first item (move to FieldForm)
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.set_fmv_flow.as_ref().map(|f| &f.step),
                    Some(SetFmvStep::FieldForm { .. })
                ),
                "C2d: must enter FieldForm after Enter on list"
            );

            // 'q' at FieldForm inserts into the text buffer (does NOT quit) [R2-N1]
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit,
                "C2d: 'q' at FieldForm must be swallowed (not quit) [R2-N1]"
            );
            assert!(app.set_fmv_flow.is_some());
            // Backspace out the 'q' before typing the FMV.
            handle_key(&mut app, press(KeyCode::Backspace));

            // Type a valid FMV
            type_str(&mut app, "45.00");

            // Enter → validate → modal opens
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.set_fmv_modal.is_some(),
                "C2d: modal must open after valid Enter"
            );

            // 'q' while modal open is swallowed
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2d: 'q' at modal must be swallowed");
            assert!(
                app.set_fmv_modal.is_some(),
                "C2d: modal must stay open after 'q'"
            );

            // Esc → cancel modal (no write); FieldForm still open [I4]
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.set_fmv_modal.is_none(), "C2d: modal must close on Esc");
            assert!(
                matches!(
                    app.set_fmv_flow.as_ref().map(|f| &f.step),
                    Some(SetFmvStep::FieldForm { .. })
                ),
                "C2d: Esc on modal must keep FieldForm open"
            );

            // Esc → close FieldForm, back to List [I4]
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.set_fmv_flow.as_ref().map(|f| &f.step),
                    Some(SetFmvStep::List)
                ),
                "C2d: Esc on FieldForm must go back to List"
            );

            // Esc → close flow
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.set_fmv_flow.is_none(), "C2d: flow must close");
            assert!(!app.should_quit, "C2d: Esc on List must NOT quit");
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2d: vault bytes must be unchanged after Esc cancel path"
        );
    }

    // ── KAT-E2E-RI — end-to-end reclassify-income (business flip, kind kept) ──
    //
    // Spec steps: seed Income{Reward, business:false} → confirm IncomeRecord
    // projects {Reward, false} → drive r → Tab business to true, leave kind None
    // (keep original) → modal shows "business: true (was false)" + "keep original"
    // → save → re-project: IncomeRecord{Reward, true}; the pre-filter now excludes
    // the event from the r list (flow won't re-open; R0-M8 status).

    #[test]
    fn kat_e2e_ri_reclassify_income_happy_path() {
        use btctax_core::persistence::load_all_ordered;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-ri-pass";

        let income_id = seed_income_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // 1. Verify seed: IncomeRecord{Reward, business:false} in initial snapshot.
        let snap = app.snapshot.as_ref().unwrap();
        let ir_before = snap
            .state
            .income_recognized
            .iter()
            .find(|r| r.event == income_id)
            .expect("E2E-RI: income record must exist before");
        assert!(!ir_before.business, "E2E-RI: seed has business=false");
        assert_eq!(
            ir_before.kind,
            IncomeKind::Reward,
            "E2E-RI: seed has kind=Reward"
        );

        // 2. 'r' → open RI flow; list shows the event.
        handle_key(&mut app, press(KeyCode::Char('r')));
        assert!(
            app.reclassify_income_flow.is_some(),
            "E2E-RI: flow must open"
        );
        {
            let flow = app.reclassify_income_flow.as_ref().unwrap();
            assert_eq!(flow.list.items.len(), 1, "E2E-RI: list must show 1 event");
            assert_eq!(
                flow.list.items[0].income_event, income_id,
                "E2E-RI: list item must be the seeded Income event"
            );
        }

        // Enter → select first item (FieldForm)
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.reclassify_income_flow.as_ref().map(|f| &f.step),
                Some(ReclassifyIncomeStep::FieldForm { .. })
            ),
            "E2E-RI: must enter FieldForm"
        );

        // Tab on business (focus=0): None → true.  Leave kind as None (keep original).
        handle_key(&mut app, press(KeyCode::Tab));

        // Enter → validate → modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_income_modal.is_some(),
            "E2E-RI: modal must open"
        );

        // Modal render: shows "business: true    (was false)" and "kind: keep original".
        let backend = ratatui::backend::TestBackend::new(100, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("business: true"),
            "E2E-RI: modal must show 'business: true'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("(was false)"),
            "E2E-RI: modal must show '(was false)'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("keep original"),
            "E2E-RI: modal must show 'keep original' for kind=None; rendered: {rendered}"
        );

        // Enter on modal → save + re-project
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_income_modal.is_none(),
            "E2E-RI: modal must close"
        );
        assert!(
            app.reclassify_income_flow.is_none(),
            "E2E-RI: flow must close"
        );

        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.starts_with("Reclassified income"),
            "E2E-RI: status must start with 'Reclassified income'; got: {status}"
        );
        assert!(
            status.contains("business=true"),
            "E2E-RI: status must say business=true; got: {status}"
        );
        assert!(
            status.contains("kind=original"),
            "E2E-RI: status must say kind=original (kept); got: {status}"
        );

        // 3. Re-projected snapshot: IncomeRecord{Reward, business:true}; the
        // original business:false record is GONE (override applies; single record).
        {
            let snap_after = app.snapshot.as_ref().unwrap();
            let recs: Vec<_> = snap_after
                .state
                .income_recognized
                .iter()
                .filter(|r| r.event == income_id)
                .collect();
            assert_eq!(
                recs.len(),
                1,
                "E2E-RI: exactly one income record for the target after reclassify"
            );
            assert!(recs[0].business, "E2E-RI: business must be true after");
            assert_eq!(
                recs[0].kind,
                IncomeKind::Reward,
                "E2E-RI: kind must stay Reward (kind=None keeps original)"
            );
        }

        // 4. The event no longer appears in the 'r' list (non-voided
        // ReclassifyIncome pre-filter excludes it) → flow won't open [R0-M8].
        handle_key(&mut app, press(KeyCode::Char('r')));
        assert!(
            app.reclassify_income_flow.is_none(),
            "E2E-RI: 'r' must NOT re-open the flow (event pre-filtered)"
        );
        assert_eq!(
            app.status.as_deref(),
            Some("No reclassifiable income events"),
            "E2E-RI: status must be the R0-M8 empty-list message"
        );

        // 5. On-disk log: the ReclassifyIncome decision round-trips.
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap2, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();
        let ir_after = snap2
            .state
            .income_recognized
            .iter()
            .find(|r| r.event == income_id)
            .expect("E2E-RI: income record must exist after reopen");
        assert!(ir_after.business, "E2E-RI: business=true persists on disk");
        assert_eq!(
            ir_after.kind,
            IncomeKind::Reward,
            "E2E-RI: kind=Reward persists on disk"
        );

        let events = load_all_ordered(session2.conn()).unwrap();
        let dec_rows: Vec<_> = events.iter().filter(|r| r.kind == "decision").collect();
        // 1 MethodElection + 1 ReclassifyIncome = 2 decisions
        assert_eq!(dec_rows.len(), 2, "E2E-RI: must have 2 decision rows");
        let stored: btctax_core::EventPayload =
            serde_json::from_str(&dec_rows[1].payload_json).unwrap();
        assert!(
            matches!(
                &stored,
                btctax_core::EventPayload::ReclassifyIncome(ri)
                    if ri.income_event == income_id
                    && ri.business
                    && ri.kind.is_none()
            ),
            "E2E-RI: stored payload must be ReclassifyIncome{{business:true, kind:None}}; got: {:?}",
            stored
        );
    }

    // ── KAT-E2E-RI-SE — Interest → Mining flip moves BOTH NIIT and SE [I3] ───
    //
    // Fixture (spec D5, figures transcribed from the core reclassify_income.rs
    // KATs — niit_profile() + the ±$380 derivation + the P2-D SE math):
    //   Income{kind: Interest, business: false, fmv: $10,000} @ 2025-03-01;
    //   profile: Single, ordinary_taxable_income=$0, magi_excluding_crypto=$205,000
    //   (above the Single $200,000 §1411 threshold — NIIT non-vacuous).
    //
    // Before:  interest_nii=$10,000 → niit = round_cents(3.8% × $10,000) = $380.00
    //          (exact); se = None (Interest is SE-EXCLUDED per §1402(a)(2)).
    // After (business=true, kind=Mining):
    //          niit = $0 (Interest left NII; delta −$380.00 exact);
    //          se = Some{base $9,235.00, ss $1,145.14, medicare $267.82,
    //                    total $1,412.96, deductible_half $706.48}.
    // The TUI status is the CLEAN success string — NO tax figure in the status
    // (blocker-derived-status discipline; figures asserted on TaxResult only).

    #[test]
    fn kat_e2e_ri_se_reclassify_income_se_exposure_changes() {
        use btctax_adapters::BundledTaxTables;
        use btctax_core::event::{EventPayload, FmvStatus, Income, IncomeKind, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::tax::compute::compute_tax_year;
        use btctax_core::tax::se::compute_se_tax;
        use btctax_core::tax::types::{FilingStatus, TaxOutcome, TaxProfile};
        use btctax_core::Carryforward;
        use btctax_core::EventId;
        use btctax_core::TaxTables;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-ri-se-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();
        let income_id = EventId::import(Source::River, SourceRef::new("e2e-se-1"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let wallet = Some(btctax_core::WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![LedgerEvent {
                id: income_id.clone(),
                // 2025-03-01 12:00:00 UTC — same calendar date as the core KAT fixture.
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_830_400).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: Some(dec!(10_000)),
                    fmv_status: FmvStatus::PriceDataset,
                    kind: IncomeKind::Interest,
                    business: false,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            session.save().unwrap();
        }

        let tables = BundledTaxTables::load();
        let year = 2025;
        // niit_profile() from the core KAT fixture (reclassify_income.rs).
        let profile = TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: dec!(0),
            magi_excluding_crypto: dec!(205000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(0),
                long: dec!(0),
            },
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        };

        // ── 1. BEFORE reclassify: niit = $380.00 exact; SE = None ────────────
        let session_before =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap_before, _) = btctax_tui::unlock::build_snapshot(&session_before).unwrap();
        let before_outcome =
            compute_tax_year(&[], &snap_before.state, year, Some(&profile), &tables);
        let TaxOutcome::Computed(before_r) = before_outcome else {
            panic!("E2E-RI-SE: engine B must be computable BEFORE; got: {before_outcome:?}");
        };
        assert_eq!(
            before_r.niit,
            dec!(380.00),
            "E2E-RI-SE: before — Interest NII $10,000 over the $200k threshold must \
             yield niit exactly $380.00"
        );
        let se_before = compute_se_tax(
            &snap_before.state,
            year,
            FilingStatus::Single,
            tables.table_for(year).unwrap(),
            dec!(0),
            dec!(0),
            dec!(0),
        );
        assert!(
            se_before.is_none(),
            "E2E-RI-SE: before — Interest is SE-excluded (§1402(a)(2)); got: {:?}",
            se_before
        );
        drop(session_before);

        // ── 2. Drive the TUI flow: business=true, kind=Mining ────────────────
        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('r')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → FieldForm
        handle_key(&mut app, press(KeyCode::Tab)); // business: None → true
        handle_key(&mut app, press(KeyCode::Down)); // focus → kind
        handle_key(&mut app, press(KeyCode::Tab)); // kind: None → Mining
        handle_key(&mut app, press(KeyCode::Enter)); // validate → modal
        assert!(app.reclassify_income_modal.is_some(), "SE: modal must open");

        // Modal render: "business: true (was false)" and "kind: mining (was interest)".
        let backend = ratatui::backend::TestBackend::new(100, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("business: true"),
            "E2E-RI-SE: modal must show 'business: true'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("(was false)"),
            "E2E-RI-SE: modal must show '(was false)'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("mining (was interest)"),
            "E2E-RI-SE: modal must show 'mining (was interest)'; rendered: {rendered}"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save + re-project
        assert!(
            app.reclassify_income_modal.is_none(),
            "SE: modal must close after save"
        );

        // Status is the CLEAN success string — no tax figure appears in it.
        assert_eq!(
            app.status.as_deref(),
            Some("Reclassified income: business=true, kind=mining"),
            "E2E-RI-SE: status must be the clean blocker-derived string (no figures)"
        );

        // ── 3. AFTER reclassify (exact asserts) ──────────────────────────────
        let snap_after = app.snapshot.as_ref().unwrap();
        let TaxOutcome::Computed(after_r) =
            compute_tax_year(&[], &snap_after.state, year, Some(&profile), &tables)
        else {
            panic!("E2E-RI-SE: engine B must be computable AFTER");
        };
        assert_eq!(
            after_r.niit,
            dec!(0),
            "E2E-RI-SE: after — Mining is not NII; niit must be exactly $0"
        );
        assert_eq!(
            before_r.niit - after_r.niit,
            dec!(380.00),
            "E2E-RI-SE: NIIT delta must be exactly −$380.00 (Interest left NII)"
        );

        let se = compute_se_tax(
            &snap_after.state,
            year,
            FilingStatus::Single,
            tables.table_for(year).unwrap(),
            dec!(0),
            dec!(0),
            dec!(0),
        )
        .expect("E2E-RI-SE: Mining+business must produce SE tax after reclassify");
        // Core-KAT hand-derived figures for fmv=$10,000, Single, no W-2:
        assert_eq!(se.base, dec!(9235.00), "E2E-RI-SE: SE base $9,235.00");
        assert_eq!(se.ss, dec!(1145.14), "E2E-RI-SE: SS $1,145.14");
        assert_eq!(se.medicare, dec!(267.82), "E2E-RI-SE: Medicare $267.82");
        assert_eq!(se.total, dec!(1412.96), "E2E-RI-SE: total $1,412.96");
        assert_eq!(
            se.deductible_half,
            dec!(706.48),
            "E2E-RI-SE: deductible_half $706.48"
        );
    }

    // ── KAT-E2E-FMV — set-fmv happy path clears FmvMissing blocker ───────────

    #[test]
    fn kat_e2e_fmv_set_fmv_clears_blocker() {
        use btctax_core::persistence::load_all_ordered;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-fmv-pass";

        let income_id = seed_income_fmv_missing_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // 1. Verify seed: FmvMissing blocker present; income_recognized has NO
        // entry for the target (no FMV yet).
        {
            let snap = app.snapshot.as_ref().unwrap();
            let has_missing =
                snap.state.blockers.iter().any(|b| {
                    b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(&income_id)
                });
            assert!(
                has_missing,
                "E2E-FMV: FmvMissing blocker must be present before set-fmv"
            );
            assert!(
                !snap
                    .state
                    .income_recognized
                    .iter()
                    .any(|r| r.event == income_id),
                "E2E-FMV: income_recognized must NOT contain the target before set-fmv"
            );
        }

        // 'f' → open flow; list shows the event.
        handle_key(&mut app, press(KeyCode::Char('f')));
        assert!(app.set_fmv_flow.is_some(), "E2E-FMV: flow must open");
        {
            let flow = app.set_fmv_flow.as_ref().unwrap();
            assert_eq!(flow.list.items.len(), 1, "E2E-FMV: list must show 1 event");
            assert_eq!(
                flow.list.items[0].event, income_id,
                "E2E-FMV: list item must be the FmvMissing Income event"
            );
        }

        // Enter → select first item (FieldForm)
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.set_fmv_flow.as_ref().map(|f| &f.step),
                Some(SetFmvStep::FieldForm { .. })
            ),
            "E2E-FMV: must enter FieldForm"
        );

        // Type FMV
        type_str(&mut app, "45.00");

        // Enter → modal
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.set_fmv_modal.is_some(), "E2E-FMV: modal must open");

        // 2. Modal render: shows usd_fmv 45.00 and the target canonical id.
        let backend = ratatui::backend::TestBackend::new(100, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("45.00"),
            "E2E-FMV: modal must show usd_fmv 45.00; rendered: {rendered}"
        );
        assert!(
            rendered.contains(&income_id.canonical()),
            "E2E-FMV: modal must show the target canonical id; rendered: {rendered}"
        );

        // Enter → confirm
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.set_fmv_modal.is_none(), "E2E-FMV: modal must close");
        assert!(app.set_fmv_flow.is_none(), "E2E-FMV: flow must close");

        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.starts_with("FMV set"),
            "E2E-FMV: status must start with 'FMV set'; got: {status}"
        );
        assert!(
            status.contains("FmvMissing blocker cleared"),
            "E2E-FMV: status must say the blocker cleared; got: {status}"
        );

        // 3. Re-projected state: FmvMissing GONE; income_recognized has the entry
        // {usd_fmv: 45.00, kind: Staking, business: false}; the lot materializes
        // at usd_basis 45.00 (NOT basis_pending).
        {
            use rust_decimal_macros::dec;
            let snap_after = app.snapshot.as_ref().unwrap();
            let still_missing =
                snap_after.state.blockers.iter().any(|b| {
                    b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(&income_id)
                });
            assert!(
                !still_missing,
                "E2E-FMV: FmvMissing blocker must be cleared after set-fmv"
            );

            let ir = snap_after
                .state
                .income_recognized
                .iter()
                .find(|r| r.event == income_id)
                .expect("E2E-FMV: income_recognized must contain the target after set-fmv");
            assert_eq!(ir.usd_fmv, dec!(45.00), "E2E-FMV: usd_fmv must be 45.00");
            assert_eq!(
                ir.kind,
                IncomeKind::Staking,
                "E2E-FMV: kind must stay Staking"
            );
            assert!(!ir.business, "E2E-FMV: business must stay false");

            // 4. The lot has usd_basis = 45.00 (not basis_pending).
            let lot = snap_after
                .state
                .lots
                .iter()
                .find(|l| l.original_sat == 100_000)
                .expect("E2E-FMV: the income lot must materialize after set-fmv");
            assert_eq!(
                lot.usd_basis,
                dec!(45.00),
                "E2E-FMV: lot.usd_basis must equal the supplied FMV"
            );
            assert!(
                !lot.basis_pending,
                "E2E-FMV: lot must NOT be basis_pending after set-fmv"
            );
        }

        // The event is no longer in the 'f' list (FmvMissing cleared) → the flow
        // won't re-open [R0-M8].
        handle_key(&mut app, press(KeyCode::Char('f')));
        assert!(
            app.set_fmv_flow.is_none(),
            "E2E-FMV: 'f' must NOT re-open the flow (blocker cleared)"
        );
        assert_eq!(
            app.status.as_deref(),
            Some("No FMV-missing income events"),
            "E2E-FMV: status must be the R0-M8 empty-list message"
        );

        // Verify ManualFmv decision was appended.
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let events = load_all_ordered(session2.conn()).unwrap();
        let dec_rows: Vec<_> = events.iter().filter(|r| r.kind == "decision").collect();
        // 1 MethodElection + 1 ManualFmv = 2 decisions
        assert_eq!(
            dec_rows.len(),
            2,
            "E2E-FMV: must have 2 decision rows; got: {}",
            dec_rows.len()
        );
        let stored: btctax_core::EventPayload =
            serde_json::from_str(&dec_rows[1].payload_json).unwrap();
        assert!(
            matches!(
                &stored,
                btctax_core::EventPayload::ManualFmv(mf)
                    if mf.event == income_id
                    && mf.usd_fmv == rust_decimal_macros::dec!(45.00)
            ),
            "E2E-FMV: stored payload must be ManualFmv; got: {:?}",
            stored
        );
    }

    // ── KAT-E2E-FMV-REPOINT — second set-fmv overrides; NO conflict ──────────
    //
    // Spec D5: after the FIRST set-fmv the FmvMissing blocker clears, so the
    // event leaves the 'f' list (pinned in KAT-E2E-FMV). The re-point is
    // therefore tested at the unit level: `persist_set_fmv` called twice on the
    // same event. LATEST-WINS (resolve.rs:453–456): on-disk log grows by 2
    // ManualFmv rows, NO DecisionConflict, income_recognized reflects the
    // SECOND FMV. This proves the "no pre-filter for already-set FMVs" claim
    // is safe.

    #[test]
    fn kat_e2e_fmv_repoint_second_set_fmv_no_conflict() {
        use btctax_core::persistence::load_all_ordered;
        use btctax_core::ManualFmv;
        use rust_decimal_macros::dec;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-fmv-repoint-pass";

        let income_id = seed_income_fmv_missing_vault(&vault, &key, pp_str);

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();

        // First set-fmv: 45.00.
        let now1 = time::OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();
        let payload1 = EventPayload::ManualFmv(ManualFmv {
            event: income_id.clone(),
            usd_fmv: dec!(45.00),
        });
        crate::edit::persist::persist_set_fmv(&mut session, payload1, now1).unwrap();

        // Second set-fmv (the re-point): 90.00 — LATEST-WINS, no conflict.
        let now2 = time::OffsetDateTime::from_unix_timestamp(1_748_003_000).unwrap();
        let payload2 = EventPayload::ManualFmv(ManualFmv {
            event: income_id.clone(),
            usd_fmv: dec!(90.00),
        });
        crate::edit::persist::persist_set_fmv(&mut session, payload2, now2).unwrap();

        // On-disk log == pre + 2 ManualFmv rows.
        drop(session);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 2,
            "REPOINT: on-disk log must grow by exactly 2 rows"
        );
        let first: btctax_core::EventPayload =
            serde_json::from_str(&post[pre.len()].payload_json).unwrap();
        let second: btctax_core::EventPayload =
            serde_json::from_str(&post[pre.len() + 1].payload_json).unwrap();
        assert!(
            matches!(
                &first,
                btctax_core::EventPayload::ManualFmv(mf)
                    if mf.event == income_id && mf.usd_fmv == dec!(45.00)
            ),
            "REPOINT: first appended row must be ManualFmv(45.00); got: {first:?}"
        );
        assert!(
            matches!(
                &second,
                btctax_core::EventPayload::ManualFmv(mf)
                    if mf.event == income_id && mf.usd_fmv == dec!(90.00)
            ),
            "REPOINT: second appended row must be ManualFmv(90.00); got: {second:?}"
        );

        // Re-projected state: NO DecisionConflict anywhere (latest-wins is not a
        // conflict); income_recognized reflects the SECOND FMV; FmvMissing gone.
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();
        assert!(
            !snap
                .state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::DecisionConflict),
            "REPOINT: no DecisionConflict may fire on a ManualFmv re-point; blockers: {:?}",
            snap.state.blockers
        );
        assert!(
            !snap
                .state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(&income_id)),
            "REPOINT: FmvMissing must stay cleared after the re-point"
        );
        let ir = snap
            .state
            .income_recognized
            .iter()
            .find(|r| r.event == income_id)
            .expect("REPOINT: income_recognized must contain the target");
        assert_eq!(
            ir.usd_fmv,
            dec!(90.00),
            "REPOINT: income_recognized must reflect the SECOND FMV (latest-wins)"
        );
    }

    // ── KAT-RI-REQUIRED-BUSINESS — key-driven: submit blocked without a choice ─
    //
    // Spec D5: on the reclassify-income FieldForm the `business` field is
    // REQUIRED-EXPLICIT — initial None renders "---" + "[required]"; Enter with
    // None sets the error and does NOT open the modal; Tab chooses true; Enter
    // then opens the modal.

    #[test]
    fn kat_ri_required_business_submit_blocked_without_choice() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-ri-req-biz-pass";

        seed_income_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        handle_key(&mut app, press(KeyCode::Char('r')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → FieldForm (business=None)

        // Initial render: "---" and "[required]" marker for the unset business row.
        let backend = ratatui::backend::TestBackend::new(100, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("---"),
            "RI-REQ: form must render '---' for unset business; rendered: {rendered}"
        );
        assert!(
            rendered.contains("[required]"),
            "RI-REQ: form must render the '[required]' marker; rendered: {rendered}"
        );

        // Enter with business=None → error; modal must NOT open.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_income_modal.is_none(),
            "RI-REQ: modal must NOT open while business is unset"
        );
        {
            let flow = app.reclassify_income_flow.as_ref().unwrap();
            let ReclassifyIncomeStep::FieldForm {
                error, business, ..
            } = &flow.step
            else {
                panic!("RI-REQ: must still be at FieldForm");
            };
            assert!(business.is_none(), "RI-REQ: business must still be None");
            let err = error.as_deref().unwrap_or("");
            assert!(
                err.contains("business is required"),
                "RI-REQ: error must say 'business is required'; got: {err}"
            );
        }

        // Tab → business = Some(true); Enter → modal opens (no error).
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.reclassify_income_modal.is_some(),
            "RI-REQ: modal must open once business is chosen"
        );
    }

    // ── KAT-RS-1..4 — remedy-string unit tests [M2] ──────────────────────────
    //
    // Each arm of derive_classify_inbound_status / derive_reclassify_outflow_status
    // must contain BOTH "'v'" (TUI void flow hint) AND "btctax reconcile void"
    // (CLI fallback). Nothing is deleted — all existing pins survive.

    fn make_synthetic_snapshot_with_conflict(
        event: btctax_core::EventId,
    ) -> btctax_tui::app::Snapshot {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_core::state::{Blocker, LedgerState};
        use std::collections::BTreeMap;
        let mut state = LedgerState::default();
        state.blockers.push(Blocker {
            kind: BlockerKind::DecisionConflict,
            event: Some(event),
            detail: "synthetic conflict".to_string(),
        });
        btctax_tui::app::Snapshot {
            events: vec![],
            state,
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
        }
    }

    fn make_synthetic_snapshot_with_blocker(
        kind: BlockerKind,
        event: btctax_core::EventId,
    ) -> btctax_tui::app::Snapshot {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_core::state::{Blocker, LedgerState};
        use std::collections::BTreeMap;
        let mut state = LedgerState::default();
        state.blockers.push(Blocker {
            kind,
            event: Some(event),
            detail: "synthetic blocker".to_string(),
        });
        btctax_tui::app::Snapshot {
            events: vec![],
            state,
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
        }
    }

    // KAT-RS-1: derive_classify_inbound_status — DecisionConflict arm.
    #[test]
    fn kat_rs_1_classify_inbound_status_conflict_arm_names_void_flow() {
        use btctax_core::{event::InboundClass, EventId, IncomeKind};
        let decision_id = EventId::Decision { seq: 42 };
        let target_event = EventId::Decision { seq: 1 }; // dummy
        let as_ = InboundClass::Income {
            kind: IncomeKind::Staking,
            fmv: None,
            business: false,
        };
        let snap = make_synthetic_snapshot_with_conflict(decision_id.clone());
        let status = derive_classify_inbound_status(&snap, &target_event, &decision_id, &as_);
        assert!(
            status.contains("'v'"),
            "RS-1: status must contain \"'v'\"; got: {status}"
        );
        assert!(
            status.contains("btctax reconcile void"),
            "RS-1: status must contain 'btctax reconcile void'; got: {status}"
        );
        assert!(
            status.contains("quit the editor"),
            "RS-1: status must name 'quit the editor' first (VaultLock audit); got: {status}"
        );
    }

    // KAT-RS-2: derive_classify_inbound_status — FmvMissing arm.
    #[test]
    fn kat_rs_2_classify_inbound_status_fmv_missing_arm_names_void_flow() {
        use btctax_core::{event::InboundClass, EventId, IncomeKind};
        let decision_id = EventId::Decision { seq: 5 };
        let target_event = EventId::Decision { seq: 3 }; // dummy target
        let as_ = InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: None,
            business: false,
        };
        let snap =
            make_synthetic_snapshot_with_blocker(BlockerKind::FmvMissing, target_event.clone());
        let status = derive_classify_inbound_status(&snap, &target_event, &decision_id, &as_);
        assert!(
            status.contains("'v'"),
            "RS-2: status must contain \"'v'\"; got: {status}"
        );
        assert!(
            status.contains("btctax reconcile void"),
            "RS-2: status must contain 'btctax reconcile void'; got: {status}"
        );
        assert!(
            status.contains("quit the editor"),
            "RS-2: status must name 'quit the editor' first (VaultLock audit); got: {status}"
        );
    }

    // KAT-RS-3: derive_classify_inbound_status — UnknownBasisInbound arm.
    #[test]
    fn kat_rs_3_classify_inbound_status_unknown_basis_arm_names_void_flow() {
        use btctax_core::{event::InboundClass, EventId, IncomeKind};
        let decision_id = EventId::Decision { seq: 7 };
        let target_event = EventId::Decision { seq: 4 };
        let as_ = InboundClass::Income {
            kind: IncomeKind::Mining,
            fmv: None,
            business: false,
        };
        let snap = make_synthetic_snapshot_with_blocker(
            BlockerKind::UnknownBasisInbound,
            target_event.clone(),
        );
        let status = derive_classify_inbound_status(&snap, &target_event, &decision_id, &as_);
        assert!(
            status.contains("'v'"),
            "RS-3: status must contain \"'v'\"; got: {status}"
        );
        assert!(
            status.contains("btctax reconcile void"),
            "RS-3: status must contain 'btctax reconcile void'; got: {status}"
        );
        assert!(
            status.contains("quit the editor"),
            "RS-3: status must name 'quit the editor' first (VaultLock audit); got: {status}"
        );
    }

    // KAT-RS-4: derive_reclassify_outflow_status — DecisionConflict arm.
    #[test]
    fn kat_rs_4_reclassify_outflow_status_conflict_arm_names_void_flow() {
        use btctax_core::EventId;
        let decision_id = EventId::Decision { seq: 11 };
        let target_event = EventId::Decision { seq: 1 };
        let snap = make_synthetic_snapshot_with_conflict(decision_id.clone());
        let status = derive_reclassify_outflow_status(&snap, &target_event, &decision_id, "sell");
        assert!(
            status.contains("'v'"),
            "RS-4: status must contain \"'v'\"; got: {status}"
        );
        assert!(
            status.contains("btctax reconcile void"),
            "RS-4: status must contain 'btctax reconcile void'; got: {status}"
        );
        assert!(
            status.contains("quit the editor"),
            "RS-4: status must name 'quit the editor' first (VaultLock audit); got: {status}"
        );
    }

    // KAT-RS-5: derive_reclassify_income_status — DecisionConflict arm.
    #[test]
    fn kat_rs_5_reclassify_income_status_conflict_arm_names_void_flow() {
        use btctax_core::EventId;
        let decision_id = EventId::Decision { seq: 13 };
        let target_event = EventId::Decision { seq: 2 };
        let snap = make_synthetic_snapshot_with_conflict(decision_id.clone());
        let status = derive_reclassify_income_status(
            &snap,
            &target_event,
            &decision_id,
            true,
            Some(IncomeKind::Staking),
        );
        assert!(
            status.contains("'v'"),
            "RS-5: status must contain \"'v'\"; got: {status}"
        );
        assert!(
            status.contains("btctax reconcile void"),
            "RS-5: status must contain 'btctax reconcile void'; got: {status}"
        );
        assert!(
            status.contains("quit the editor"),
            "RS-5: status must name 'quit the editor' first (VaultLock audit); got: {status}"
        );
    }

    // KAT-RS-6: derive_set_fmv_status — DecisionConflict arm.
    #[test]
    fn kat_rs_6_set_fmv_status_conflict_arm_names_void_flow() {
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        let decision_id = EventId::Decision { seq: 17 };
        let target_event = EventId::Decision { seq: 6 };
        let snap = make_synthetic_snapshot_with_conflict(decision_id.clone());
        let status = derive_set_fmv_status(&snap, &target_event, &decision_id, dec!(1234));
        assert!(
            status.contains("'v'"),
            "RS-6: status must contain \"'v'\"; got: {status}"
        );
        assert!(
            status.contains("btctax reconcile void"),
            "RS-6: status must contain 'btctax reconcile void'; got: {status}"
        );
        assert!(
            status.contains("quit the editor"),
            "RS-6: status must name 'quit the editor' first (VaultLock audit); got: {status}"
        );
    }

    // ── KAT-C2e — cancel-path void flow (bytes unchanged) ────────────────────

    #[test]
    fn kat_c2e_cancel_path_vault_bytes_unchanged_void() {
        use btctax_core::event::{EventPayload, MethodElection};
        use btctax_core::persistence::append_decision;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2e-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed a MethodElection decision so the void list is non-empty.
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let p = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            append_decision(
                session.conn(),
                p,
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // v → void flow opens at List step.
            handle_key(&mut app, press(KeyCode::Char('v')));
            assert!(app.void_flow.is_some(), "C2e: flow must open on 'v'");
            assert!(app.void_modal.is_none(), "C2e: modal must not be open yet");

            // 'q' at List step is swallowed.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(app.void_flow.is_some(), "'q' must not close the void flow");
            assert!(
                !app.should_quit,
                "'q' must not quit while void flow is open"
            );

            // Enter → modal opens DIRECTLY (no FieldForm).
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.void_modal.is_some(),
                "C2e: Enter must open modal directly"
            );

            // 'q' at modal step is swallowed.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(app.void_modal.is_some(), "'q' must not close void modal");
            assert!(!app.should_quit, "'q' must not quit while modal is open");

            // Esc → modal closes; flow still open at List step.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.void_modal.is_none(), "C2e: Esc must close modal");
            assert!(
                app.void_flow.is_some(),
                "C2e: flow must stay open after modal Esc"
            );

            // Esc again → flow closes.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.void_flow.is_none(), "C2e: second Esc must close flow");
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2e: vault bytes must be unchanged after cancel path"
        );

        // Complement: confirmed path writes (Enter → Enter).
        {
            let mut app = open_app(&vault, pp_str);
            handle_key(&mut app, press(KeyCode::Char('v')));
            handle_key(&mut app, press(KeyCode::Enter)); // modal opens
            assert!(app.void_modal.is_some(), "C2e-complement: modal must open");
            handle_key(&mut app, press(KeyCode::Enter)); // confirm
            assert!(
                app.void_modal.is_none(),
                "C2e-complement: modal must close after confirm"
            );
            assert!(
                app.void_flow.is_none(),
                "C2e-complement: flow must close after confirm"
            );
        }
        let bytes_written = std::fs::read(&vault).unwrap();
        assert_ne!(
            bytes_before, bytes_written,
            "C2e-complement: vault bytes must change after confirmed void"
        );
    }

    // ── KAT-S2b — save-error path for set-fmv (chmod; latest-wins retry) ─────
    //
    // KAT-S2b justification: the ONLY new retry detail in chunk 2b is LATEST-WINS
    // (set-fmv retry yields +2 rows, NO conflict, second FMV governs). This KAT pins that.

    #[cfg(unix)]
    #[test]
    fn kat_s2b_save_error_path_set_fmv_chmod() {
        use btctax_core::persistence::load_all_ordered;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-s2b-pass";

        let income_id = seed_income_fmv_missing_vault(&vault, &key, pp_str);
        let _ = income_id;

        // Root-skip guard (same as KAT-S1 / KAT-S2).
        {
            let test_file = dir.path().join("root_check.txt");
            std::fs::write(&test_file, b"x").unwrap();
            let mut perms = std::fs::metadata(&test_file).unwrap().permissions();
            perms.set_mode(0o000);
            std::fs::set_permissions(&test_file, perms).unwrap();
            let readable = std::fs::read(&test_file).is_ok();
            if readable {
                eprintln!("KAT-S2b: skipping — chmod 0o000 did not deny reads (running as root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();
        let mut app = open_app(&vault, pp_str);
        let pre_count = {
            let session = app.session.as_ref().unwrap();
            load_all_ordered(session.conn()).unwrap().len()
        };

        // Navigate to the set-fmv modal.
        handle_key(&mut app, press(KeyCode::Char('f')));
        assert!(app.set_fmv_flow.is_some(), "S2b: set-fmv flow must open");
        handle_key(&mut app, press(KeyCode::Enter)); // list → FieldForm
        assert!(
            matches!(
                app.set_fmv_flow.as_ref().unwrap().step,
                SetFmvStep::FieldForm { .. }
            ),
            "S2b: must be at FieldForm"
        );
        type_str(&mut app, "45.00");
        handle_key(&mut app, press(KeyCode::Enter)); // FieldForm → modal
        assert!(app.set_fmv_modal.is_some(), "S2b: modal must open");

        // Break the vault parent dir: chmod 0o500 → atomic save will fail.
        let parent = dir.path();
        let orig_perms = std::fs::metadata(parent).unwrap().permissions();
        let mut no_write = orig_perms.clone();
        no_write.set_mode(0o500);
        std::fs::set_permissions(parent, no_write).unwrap();

        // Enter → save fails.
        handle_key(&mut app, press(KeyCode::Enter));

        // Restore perms before any asserts that might panic.
        std::fs::set_permissions(parent, orig_perms.clone()).unwrap();

        // Assert: modal closed; FieldForm still open; status contains "Save error".
        assert!(
            app.set_fmv_modal.is_none(),
            "S2b: modal must close after save error"
        );
        assert!(
            matches!(
                app.set_fmv_flow.as_ref().unwrap().step,
                SetFmvStep::FieldForm { .. }
            ),
            "S2b: FieldForm must stay open after save error"
        );
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("Save error"),
            "S2b: status must say 'Save error'; got: {status}"
        );
        let bytes_after_fail = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after_fail,
            "S2b: vault bytes must be unchanged after failed save"
        );
        // [save-rollback] no residue: the in-memory log is reverted to pre after the failed save.
        let mid = load_all_ordered(app.session.as_ref().unwrap().conn()).unwrap();
        assert_eq!(
            mid.len(),
            pre_count,
            "S2b: rollback must revert the in-memory append (no residue after a failed save)"
        );

        // Re-submit (retry): the rolled-back save left nothing; the retry appends ONE ManualFmv row.
        handle_key(&mut app, press(KeyCode::Enter)); // FieldForm → modal (re-open)
        assert!(
            app.set_fmv_modal.is_some(),
            "S2b: modal must re-open for retry"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm retry

        // Assert: exactly ONE ManualFmv row appended (rollback → no residue); no FmvMissing; no conflict.
        let session = app.session.as_ref().unwrap();
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre_count + 1,
            "S2b: retry after a rolled-back save must append EXACTLY ONE ManualFmv row (no residue)"
        );

        use btctax_core::EventPayload;
        let new_decisions: Vec<_> = post[pre_count..].iter().collect();
        assert_eq!(new_decisions.len(), 1, "S2b: exactly 1 new row");
        let p: EventPayload = serde_json::from_str(&new_decisions[0].payload_json).unwrap();
        assert!(
            matches!(p, EventPayload::ManualFmv(_)),
            "S2b: the new row must be ManualFmv"
        );

        // Re-project and check.
        let snap = app.snapshot.as_ref().unwrap();
        let has_fmv_missing = snap
            .state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::FmvMissing);
        assert!(
            !has_fmv_missing,
            "S2b: FmvMissing must be GONE after retry (latest-wins)"
        );
        let has_conflict = snap
            .state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict);
        assert!(
            !has_conflict,
            "S2b: no DecisionConflict after retry (latest-wins — no conflict)"
        );
        let status_retry = app.status.as_deref().unwrap_or("");
        assert!(
            status_retry.contains("FMV set") || status_retry.contains("FmvMissing blocker cleared"),
            "S2b: retry status must be clean-success; got: {status_retry}"
        );
    }

    // ── KAT-E2E-VOID-ROUNDTRIP ───────────────────────────────────────────────
    //
    // Full remedy loop: classify → void → blocker returns → re-classify cleanly.

    #[test]
    fn kat_e2e_void_roundtrip_classify_void_reclassify() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-roundtrip-pass";

        let ti_id = seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // 1. TransferIn is in the c list; UnknownBasisInbound fires.
        {
            let snap = app.snapshot.as_ref().unwrap();
            let has_ubi = snap.state.blockers.iter().any(|b| {
                b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
            });
            assert!(
                has_ubi,
                "VOID-RT: TransferIn must have UnknownBasisInbound initially"
            );
        }

        // 2. Classify-inbound as Income(Staking, fmv=200) via TUI.
        handle_key(&mut app, press(KeyCode::Char('c')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker
        handle_key(&mut app, press(KeyCode::Enter)); // picker → Income form (Income is initial)
        type_str(&mut app, "200.00"); // fmv
        handle_key(&mut app, press(KeyCode::Enter)); // form → modal
        assert!(
            app.classify_inbound_modal.is_some(),
            "VOID-RT: modal must open"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // confirm
        assert!(
            app.classify_inbound_modal.is_none(),
            "VOID-RT: modal must close"
        );

        // Capture the ClassifyInbound decision id.
        let classify_id = {
            use btctax_core::persistence::load_all_ordered;
            let session = app.session.as_ref().unwrap();
            let events = load_all_ordered(session.conn()).unwrap();
            let ci_row = events
                .iter()
                .rfind(|r| r.decision_seq.is_some())
                .unwrap()
                .clone();
            btctax_core::EventId::Decision {
                seq: ci_row.decision_seq.unwrap() as u64,
            }
        };

        // 3. TransferIn is now excluded from c list.
        handle_key(&mut app, press(KeyCode::Char('c')));
        assert!(
            app.classify_inbound_flow
                .as_ref()
                .map(|f| f.list.items.iter().all(|i| i.blocker_event != ti_id))
                .unwrap_or(true),
            "VOID-RT: classified TransferIn must be excluded from c list"
        );
        // Close the flow.
        handle_key(&mut app, press(KeyCode::Esc));

        // 4. Void the ClassifyInbound via v flow.
        handle_key(&mut app, press(KeyCode::Char('v')));
        assert!(app.void_flow.is_some(), "VOID-RT: void flow must open");
        // Find the ClassifyInbound in the list.
        let void_flow = app.void_flow.as_ref().unwrap();
        let ci_idx = void_flow
            .list
            .items
            .iter()
            .position(|i| i.event_id == classify_id);
        assert!(
            ci_idx.is_some(),
            "VOID-RT: ClassifyInbound must be in void list"
        );
        // Navigate to it (it should already be selected or we scroll to it).
        for _ in 0..ci_idx.unwrap() {
            handle_key(&mut app, press(KeyCode::Down));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.void_modal.is_some(), "VOID-RT: void modal must open");
        handle_key(&mut app, press(KeyCode::Enter)); // confirm void
        assert!(app.void_modal.is_none(), "VOID-RT: void modal must close");
        assert!(
            app.void_flow.is_none(),
            "VOID-RT: void flow must close after confirm"
        );

        // 5. Re-project: UnknownBasisInbound returns; TransferIn back in c list.
        {
            let snap = app.snapshot.as_ref().unwrap();
            let ubi_returned = snap.state.blockers.iter().any(|b| {
                b.kind == BlockerKind::UnknownBasisInbound && b.event.as_ref() == Some(&ti_id)
            });
            assert!(
                ubi_returned,
                "VOID-RT: UnknownBasisInbound must return after void"
            );
        }

        // 6. Re-classify as Income(Mining, fmv=250) — should succeed with no conflict.
        handle_key(&mut app, press(KeyCode::Char('c')));
        assert!(
            app.classify_inbound_flow.is_some(),
            "VOID-RT: c must re-open flow"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // list → picker
        handle_key(&mut app, press(KeyCode::Enter)); // picker → Income form
        type_str(&mut app, "250.00");
        handle_key(&mut app, press(KeyCode::Enter)); // form → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            !status.contains("DecisionConflict"),
            "VOID-RT: re-classify must not conflict; got: {status}"
        );

        // 7. The old (voided) ClassifyInbound must NOT appear in the void list.
        handle_key(&mut app, press(KeyCode::Char('v')));
        if let Some(flow) = app.void_flow.as_ref() {
            assert!(
                flow.list.items.iter().all(|i| i.event_id != classify_id),
                "VOID-RT: voided ClassifyInbound must not appear in void list"
            );
        }
        if app.void_flow.is_some() {
            handle_key(&mut app, press(KeyCode::Esc));
        }
    }

    // ── KAT-E2E-VOID-RECLASSIFY-INCOME ──────────────────────────────────────

    #[test]
    fn kat_e2e_void_reclassify_income_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-ri-pass";

        let income_id = seed_income_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // 1. Income event appears in r list.
        handle_key(&mut app, press(KeyCode::Char('r')));
        assert!(
            app.reclassify_income_flow.is_some(),
            "VOID-RI: r flow must open"
        );
        handle_key(&mut app, press(KeyCode::Esc));

        // 2. Reclassify-income: business=true.
        handle_key(&mut app, press(KeyCode::Char('r')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → FieldForm
        handle_key(&mut app, press(KeyCode::Tab)); // business → Some(true)
        handle_key(&mut app, press(KeyCode::Enter)); // form → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        let ri_decision_id = {
            use btctax_core::persistence::load_all_ordered;
            let session = app.session.as_ref().unwrap();
            let events = load_all_ordered(session.conn()).unwrap();
            let last_decision = events.iter().rfind(|r| r.decision_seq.is_some()).unwrap();
            btctax_core::EventId::Decision {
                seq: last_decision.decision_seq.unwrap() as u64,
            }
        };

        // Check event excluded from r list.
        handle_key(&mut app, press(KeyCode::Char('r')));
        let excluded = app
            .reclassify_income_flow
            .as_ref()
            .map(|f| f.list.items.iter().all(|i| i.income_event != income_id))
            .unwrap_or(true);
        assert!(
            excluded,
            "VOID-RI: reclassified income must be excluded from r list"
        );
        if app.reclassify_income_flow.is_some() {
            handle_key(&mut app, press(KeyCode::Esc));
        }

        // 3. Void the ReclassifyIncome via v flow.
        handle_key(&mut app, press(KeyCode::Char('v')));
        assert!(app.void_flow.is_some(), "VOID-RI: void flow must open");
        let ri_idx = app
            .void_flow
            .as_ref()
            .unwrap()
            .list
            .items
            .iter()
            .position(|i| i.event_id == ri_decision_id);
        assert!(
            ri_idx.is_some(),
            "VOID-RI: ReclassifyIncome must be in void list"
        );
        for _ in 0..ri_idx.unwrap() {
            handle_key(&mut app, press(KeyCode::Down));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        // 3b. IncomeRecord-restoration: post-void snapshot reverts to original values.
        // The reclassify set business=true; after void the record must restore to the
        // seeded business=false, kind=Reward (IncomeKind unchanged by kind=None reclassify).
        {
            let snap = app.snapshot.as_ref().unwrap();
            let ir = snap
                .state
                .income_recognized
                .iter()
                .find(|r| r.event == income_id)
                .expect("VOID-RI: income record must exist in post-void snapshot");
            assert!(
                !ir.business,
                "VOID-RI: business must revert to original false after void"
            );
            assert_eq!(
                ir.kind,
                IncomeKind::Reward,
                "VOID-RI: kind must remain original Reward after void"
            );
        }

        // 4. Re-project: income event back in r list.
        handle_key(&mut app, press(KeyCode::Char('r')));
        let back_in_list = app
            .reclassify_income_flow
            .as_ref()
            .map(|f| f.list.items.iter().any(|i| i.income_event == income_id))
            .unwrap_or(false);
        assert!(
            back_in_list,
            "VOID-RI: income event must be back in r list after void"
        );
        handle_key(&mut app, press(KeyCode::Esc));

        // 5. Re-reclassify: business=true, kind=Mining → no conflict.
        handle_key(&mut app, press(KeyCode::Char('r')));
        handle_key(&mut app, press(KeyCode::Enter)); // list → FieldForm
        handle_key(&mut app, press(KeyCode::Tab)); // business → Some(true)
        handle_key(&mut app, press(KeyCode::Down)); // focus → kind
        handle_key(&mut app, press(KeyCode::Tab)); // kind → Mining
        handle_key(&mut app, press(KeyCode::Enter)); // form → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            !status.contains("DecisionConflict"),
            "VOID-RI: re-reclassify must not conflict; got: {status}"
        );
    }

    // ── KAT-E2E-VOID-CASCADE ─────────────────────────────────────────────────
    //
    // Pins: ClassifyRaw → void → orphaned ManualFmv fires conflict → void it → clean.

    #[test]
    fn kat_e2e_void_cascade_orphaned_manual_fmv() {
        use btctax_core::event::{
            ClassifyRaw, EventPayload, FmvStatus, Income, IncomeKind, LedgerEvent, ManualFmv,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::append_decision;
        use btctax_core::{EventId, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-cascade-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed an Unclassified event with a wallet.
        let unclassified_id = EventId::import(Source::River, SourceRef::new("unclass-cascade"));
        let wallet = Some(WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        let ts = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
        let cr_id: EventId;
        let mf_id: EventId;
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            // Import the Unclassified event.
            let batch = vec![LedgerEvent {
                id: unclassified_id.clone(),
                utc_timestamp: ts,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Unclassified(btctax_core::Unclassified {
                    raw: "cascade-unclassified".to_string(),
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();

            // ClassifyRaw: as Income with fmv=None.
            let cr_payload = EventPayload::ClassifyRaw(ClassifyRaw {
                target: unclassified_id.clone(),
                as_: Box::new(EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: None,
                    fmv_status: FmvStatus::Missing,
                    kind: IncomeKind::Reward,
                    business: false,
                })),
            });
            cr_id = append_decision(
                session.conn(),
                cr_payload,
                OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();

            // ManualFmv targeting the Unclassified event (now effectively Income).
            let mf_payload = EventPayload::ManualFmv(ManualFmv {
                event: unclassified_id.clone(),
                usd_fmv: dec!(100),
            });
            mf_id = append_decision(
                session.conn(),
                mf_payload,
                OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();

            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);

        // 1. Pre-state: no blockers (clean — ClassifyRaw + ManualFmv resolve cleanly).
        {
            let snap = app.snapshot.as_ref().unwrap();
            assert!(
                snap.state.blockers.is_empty(),
                "CASCADE: pre-state must be clean; got: {:?}",
                snap.state.blockers
            );
        }

        // 2. v list shows BOTH ClassifyRaw and ManualFmv.
        handle_key(&mut app, press(KeyCode::Char('v')));
        assert!(app.void_flow.is_some(), "CASCADE: v flow must open");
        {
            let flow = app.void_flow.as_ref().unwrap();
            let has_cr = flow.list.items.iter().any(|i| i.event_id == cr_id);
            let has_mf = flow.list.items.iter().any(|i| i.event_id == mf_id);
            assert!(has_cr, "CASCADE: ClassifyRaw must be in void list");
            assert!(has_mf, "CASCADE: ManualFmv must be in void list");
        }

        // 3. TUI-void the ClassifyRaw.
        let cr_idx = app
            .void_flow
            .as_ref()
            .unwrap()
            .list
            .items
            .iter()
            .position(|i| i.event_id == cr_id)
            .unwrap();
        for _ in 0..cr_idx {
            handle_key(&mut app, press(KeyCode::Down));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // modal
                                                     // Assert consequence note is in the modal.
        {
            let modal = app.void_modal.as_ref().unwrap();
            // is_safe_harbor must be false for ClassifyRaw.
            assert!(
                !modal.is_safe_harbor,
                "CASCADE: ClassifyRaw is not SafeHarbor"
            );
        }
        handle_key(&mut app, press(KeyCode::Enter)); // confirm
        let void_status = app.status.as_deref().unwrap_or("").to_string();

        // 4. Re-project: ManualFmv now orphaned → DecisionConflict attributed to mf_id.
        {
            let snap = app.snapshot.as_ref().unwrap();
            let has_mf_conflict = snap.state.blockers.iter().any(|b| {
                b.kind == BlockerKind::DecisionConflict && b.event.as_ref() == Some(&mf_id)
            });
            assert!(
                has_mf_conflict,
                "CASCADE: ManualFmv must fire DecisionConflict after ClassifyRaw voided"
            );
        }

        // 5. The void status was NOT about the cascade conflict (surfacing limit D3.1 [I1]).
        assert!(
            !void_status.starts_with("Void saved, but DecisionConflict"),
            "CASCADE: void status must not say 'Void saved, but DecisionConflict'; \
             cascade conflicts attributed to orphan not void; got: {void_status}"
        );

        // 6. Drive v again: orphaned ManualFmv IS in the list.
        handle_key(&mut app, press(KeyCode::Char('v')));
        assert!(
            app.void_flow.is_some(),
            "CASCADE: v must re-open after cascade"
        );
        {
            let flow = app.void_flow.as_ref().unwrap();
            let has_mf = flow.list.items.iter().any(|i| i.event_id == mf_id);
            assert!(
                has_mf,
                "CASCADE: orphaned ManualFmv must be in void list for remedy"
            );
        }

        // 7. Void the ManualFmv → DecisionConflict gone.
        let mf_idx = app
            .void_flow
            .as_ref()
            .unwrap()
            .list
            .items
            .iter()
            .position(|i| i.event_id == mf_id)
            .unwrap();
        for _ in 0..mf_idx {
            handle_key(&mut app, press(KeyCode::Down));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        let snap = app.snapshot.as_ref().unwrap();
        let has_conflict = snap
            .state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::DecisionConflict);
        assert!(
            !has_conflict,
            "CASCADE: DecisionConflict must be gone after voiding orphaned ManualFmv"
        );
    }

    // ── KAT-VOID-CONFLICT-ARM ────────────────────────────────────────────────
    //
    // Synthetic unit KAT: the VOID-REJECTED status arm (I2) — must not start with
    // "Voided".

    #[test]
    fn kat_void_conflict_arm_rejected_void_string() {
        use btctax_core::EventId;
        let void_decision_id = EventId::Decision { seq: 99 };
        let target_event_id = EventId::Decision { seq: 10 };
        let snap = make_synthetic_snapshot_with_conflict(void_decision_id.clone());
        let status = derive_void_status(
            &snap,
            &void_decision_id,
            &target_event_id,
            None,
            "SafeHarborAllocation",
            99,
        );
        assert!(
            status.contains("Void saved, but DecisionConflict fired"),
            "VOID-CONFLICT: status must contain 'Void saved, but DecisionConflict fired'; got: {status}"
        );
        assert!(
            status.contains("the target decision remains in force"),
            "VOID-CONFLICT: status must say 'the target decision remains in force'; got: {status}"
        );
        assert!(
            !status.starts_with("Voided"),
            "VOID-CONFLICT: status must NOT start with 'Voided' (void was rejected); got: {status}"
        );
    }

    // ── KAT-VOID-RETRY ───────────────────────────────────────────────────────
    //
    // Idempotent re-void: +2 inert rows, no conflict [M1].

    #[test]
    fn kat_void_retry_idempotent_two_void_rows_no_conflict() {
        use btctax_core::event::{EventPayload, MethodElection};
        use btctax_core::persistence::{append_decision, load_all_ordered};
        use btctax_core::EventId;
        use btctax_store::Passphrase;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-retry-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Seed a MethodElection decision.
        let me_id: EventId;
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let p = EventPayload::MethodElection(MethodElection {
                effective_from: date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            me_id = append_decision(
                session.conn(),
                p,
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let mut session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let pre = load_all_ordered(session.conn()).unwrap();

        let now1 = OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap();
        let now2 = OffsetDateTime::from_unix_timestamp(1_748_002_000).unwrap();

        // Call persist_void TWICE on the same target.
        crate::edit::persist::persist_void(&mut session, me_id.clone(), now1).unwrap();
        crate::edit::persist::persist_void(&mut session, me_id.clone(), now2).unwrap();

        // Assert: on-disk has pre + 2 VoidDecisionEvent rows.
        let post = load_all_ordered(session.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre.len() + 2,
            "VOID-RETRY: must have pre + 2 rows"
        );
        for row in &post[pre.len()..] {
            let p: EventPayload = serde_json::from_str(&row.payload_json).unwrap();
            match p {
                EventPayload::VoidDecisionEvent(v) => {
                    assert_eq!(
                        v.target_event_id, me_id,
                        "VOID-RETRY: both void rows must target the original MethodElection"
                    );
                }
                other => panic!("VOID-RETRY: expected VoidDecisionEvent, got {other:?}"),
            }
        }

        // Drop session to release the vault lock before opening a second session.
        drop(session);

        // Assert: no new blocker (idempotent BTreeSet insert).
        let snap_session =
            btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let (snap, _) = btctax_tui::unlock::build_snapshot(&snap_session).unwrap();
        assert!(
            snap.state.blockers.is_empty(),
            "VOID-RETRY: no new blocker after idempotent re-void; got: {:?}",
            snap.state.blockers
        );
    }

    // ── KAT-VOID-EXCLUSIONS ──────────────────────────────────────────────────
    //
    // Void list correctly excludes non-revocable + already-voided decisions.

    #[test]
    fn kat_void_exclusions_non_revocable_and_already_voided_absent() {
        use btctax_core::event::{
            ClassifyInbound, EventPayload, MethodElection, ReclassifyOutflow, RejectImport,
            SupersedeImport, VoidDecisionEvent,
        };
        use btctax_core::event::{DisposeKind, OutflowClass};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use btctax_core::{EventId, InboundClass, IncomeKind, WalletId};
        use btctax_store::Passphrase;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-exclusions-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let now = |secs: i64| OffsetDateTime::from_unix_timestamp(secs).unwrap();
        let tz = UtcOffset::UTC;

        // Import a fake TransferIn event to be the target for ClassifyInbound/SupersedeImport.
        let fake_ti_id = EventId::import(Source::River, SourceRef::new("excl-ti"));
        let fake_to_id = EventId::import(Source::River, SourceRef::new("excl-to"));
        let fake_conflict_id = EventId::import(Source::River, SourceRef::new("excl-conflict"));

        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();

            // Seed dummy import batch (for import-id targets).
            use btctax_core::event::{LedgerEvent, TransferIn, TransferOut};
            let wallet = Some(WalletId::Exchange {
                provider: "River".to_string(),
                account: "main".to_string(),
            });
            let batch = vec![
                LedgerEvent {
                    id: fake_ti_id.clone(),
                    utc_timestamp: now(1_748_000_000),
                    original_tz: tz,
                    wallet: wallet.clone(),
                    payload: EventPayload::TransferIn(TransferIn {
                        sat: 100_000,
                        src_addr: None,
                        txid: None,
                    }),
                },
                LedgerEvent {
                    id: fake_to_id.clone(),
                    utc_timestamp: now(1_748_000_100),
                    original_tz: tz,
                    wallet: wallet.clone(),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: 50_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
                LedgerEvent {
                    id: fake_conflict_id.clone(),
                    utc_timestamp: now(1_748_000_200),
                    original_tz: tz,
                    wallet: wallet.clone(),
                    payload: EventPayload::TransferIn(TransferIn {
                        sat: 1_000,
                        src_addr: None,
                        txid: None,
                    }),
                },
            ];
            append_import_batch(session.conn(), &batch).unwrap();

            // 1. SupersedeImport — non-revocable.
            append_decision(
                session.conn(),
                EventPayload::SupersedeImport(SupersedeImport {
                    conflict_event: fake_conflict_id.clone(),
                }),
                now(1_748_001_000),
                tz,
                None,
            )
            .unwrap();

            // 2. RejectImport — non-revocable.
            append_decision(
                session.conn(),
                EventPayload::RejectImport(RejectImport {
                    conflict_event: fake_conflict_id.clone(),
                }),
                now(1_748_001_100),
                tz,
                None,
            )
            .unwrap();

            // 3. ClassifyInbound (to be voided → already-voided).
            let ci_id = append_decision(
                session.conn(),
                EventPayload::ClassifyInbound(ClassifyInbound {
                    transfer_in_event: fake_ti_id.clone(),
                    as_: InboundClass::Income {
                        kind: IncomeKind::Staking,
                        fmv: Some(dec!(100)),
                        business: false,
                    },
                }),
                now(1_748_001_200),
                tz,
                None,
            )
            .unwrap();

            // 4. VoidDecisionEvent targeting the ClassifyInbound → ClassifyInbound becomes already-voided.
            let void_ci_id = append_decision(
                session.conn(),
                EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                    target_event_id: ci_id.clone(),
                }),
                now(1_748_001_300),
                tz,
                None,
            )
            .unwrap();
            // void_ci_id is a VoidDecisionEvent — non-revocable in the void list.
            let _ = void_ci_id;

            // 5. Non-voided ReclassifyOutflow — must appear in void list.
            append_decision(
                session.conn(),
                EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                    transfer_out_event: fake_to_id.clone(),
                    as_: OutflowClass::Dispose {
                        kind: DisposeKind::Sell,
                    },
                    principal_proceeds_or_fmv: dec!(5000),
                    fee_usd: None,
                    donee: None,
                }),
                now(1_748_001_400),
                tz,
                None,
            )
            .unwrap();

            // 6. Non-voided MethodElection — must appear in void list.
            append_decision(
                session.conn(),
                EventPayload::MethodElection(MethodElection {
                    effective_from: date!(2024 - 01 - 01),
                    method: btctax_core::LotMethod::Fifo,
                }),
                now(1_748_001_500),
                tz,
                None,
            )
            .unwrap();

            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);

        // Drive v.
        handle_key(&mut app, press(KeyCode::Char('v')));
        let void_flow = app
            .void_flow
            .as_ref()
            .expect("VOID-EXCL: v must open a flow");
        let items = &void_flow.list.items;

        // Assert: exactly 2 items (ReclassifyOutflow + MethodElection).
        assert_eq!(
            items.len(),
            2,
            "VOID-EXCL: void list must contain exactly 2 items (RO + ME); got {}: {:?}",
            items.len(),
            items.iter().map(|i| i.payload_tag).collect::<Vec<_>>()
        );

        let tags: Vec<&str> = items.iter().map(|i| i.payload_tag).collect();
        assert!(
            tags.contains(&"ReclassifyOutflow"),
            "VOID-EXCL: ReclassifyOutflow must be in void list"
        );
        assert!(
            tags.contains(&"MethodElection"),
            "VOID-EXCL: MethodElection must be in void list"
        );

        // SupersedeImport, RejectImport, VoidDecisionEvent, already-voided ClassifyInbound
        // must all be absent.
        assert!(
            !tags.contains(&"SupersedeImport"),
            "VOID-EXCL: SupersedeImport must NOT be in void list"
        );
        assert!(
            !tags.contains(&"RejectImport"),
            "VOID-EXCL: RejectImport must NOT be in void list"
        );
        assert!(
            !tags.contains(&"VoidDecisionEvent"),
            "VOID-EXCL: VoidDecisionEvent must NOT be in void list"
        );
        assert!(
            !tags.contains(&"ClassifyInbound"),
            "VOID-EXCL: already-voided ClassifyInbound must NOT be in void list"
        );
    }

    // ── Task 1 KAT helpers ────────────────────────────────────────────────────

    /// Seed vault: Acquire + TransferOut + ReclassifyOutflow(Donate) → Donation removal.
    /// Returns (acquire_id, transfer_out_id, wallet).
    fn seed_donate_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
        principal_sat: btctax_core::Sat,
    ) -> (btctax_core::EventId, btctax_core::EventId) {
        use btctax_core::event::{
            Acquire, BasisSource, EventPayload, LedgerEvent, OutflowClass, ReclassifyOutflow,
            TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();

        let wallet = Some(btctax_core::WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        let acq_id = EventId::import(Source::River, SourceRef::new("donate-acq-1"));
        let to_id = EventId::import(Source::River, SourceRef::new("donate-to-1"));

        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_747_913_600).unwrap();
            let t1 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            let batch = vec![
                LedgerEvent {
                    id: acq_id.clone(),
                    utc_timestamp: t0,
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::Acquire(Acquire {
                        // Acquire 2× so FIFO leaves remaining_sat == principal_sat after donation.
                        // This ensures snap.state.lots is non-empty when select-lots opens.
                        sat: principal_sat * 2,
                        usd_cost: dec!(50000),
                        fee_usd: dec!(0),
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: to_id.clone(),
                    utc_timestamp: t1,
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: principal_sat,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();

            let t2 = OffsetDateTime::from_unix_timestamp(1_748_100_000).unwrap();
            let ro = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: to_id.clone(),
                as_: OutflowClass::Donate {
                    appraisal_required: false,
                },
                principal_proceeds_or_fmv: dec!(20000),
                fee_usd: None,
                donee: None,
            });
            btctax_core::persistence::append_decision(session.conn(), ro, t2, UtcOffset::UTC, None)
                .unwrap();
            // MethodElection so fold has a method.
            let me = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: time::macros::date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            btctax_core::persistence::append_decision(session.conn(), me, t2, UtcOffset::UTC, None)
                .unwrap();
            session.save().unwrap();
        }

        (acq_id, to_id)
    }

    /// Seed vault: TWO Acquire lots + TransferOut + ReclassifyOutflow(Sell).
    /// Returns (lot_a_id, lot_b_id, transfer_out_id).
    fn seed_two_lot_sell_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> (
        btctax_core::EventId,
        btctax_core::EventId,
        btctax_core::EventId,
    ) {
        use btctax_core::event::{
            Acquire, BasisSource, DisposeKind, EventPayload, LedgerEvent, OutflowClass,
            ReclassifyOutflow, TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();

        let wallet = Some(btctax_core::WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        let lot_a_id = EventId::import(Source::River, SourceRef::new("lot-a-1"));
        let lot_b_id = EventId::import(Source::River, SourceRef::new("lot-b-1"));
        let to_id = EventId::import(Source::River, SourceRef::new("sell-to-1"));

        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            let ta = OffsetDateTime::from_unix_timestamp(1_740_000_000).unwrap(); // lot A — earlier
            let tb = OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap(); // lot B — later
            let tc = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(); // sell
            let td = OffsetDateTime::from_unix_timestamp(1_748_100_000).unwrap(); // decisions
            let batch = vec![
                LedgerEvent {
                    id: lot_a_id.clone(),
                    utc_timestamp: ta,
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 1_000_000,
                        usd_cost: dec!(30000),
                        fee_usd: dec!(0),
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: lot_b_id.clone(),
                    utc_timestamp: tb,
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 1_000_000,
                        usd_cost: dec!(50000),
                        fee_usd: dec!(0),
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: to_id.clone(),
                    utc_timestamp: tc,
                    original_tz: UtcOffset::UTC,
                    wallet: wallet.clone(),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: 500_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();

            // ReclassifyOutflow → Sell.
            let ro = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: to_id.clone(),
                as_: OutflowClass::Dispose {
                    kind: DisposeKind::Sell,
                },
                principal_proceeds_or_fmv: dec!(30000),
                fee_usd: None,
                donee: None,
            });
            btctax_core::persistence::append_decision(session.conn(), ro, td, UtcOffset::UTC, None)
                .unwrap();
            // MethodElection (FIFO — so lot A is consumed by default).
            let me = EventPayload::MethodElection(btctax_core::event::MethodElection {
                effective_from: time::macros::date!(2024 - 01 - 01),
                method: btctax_core::LotMethod::Fifo,
            });
            btctax_core::persistence::append_decision(session.conn(), me, td, UtcOffset::UTC, None)
                .unwrap();
            session.save().unwrap();
        }

        (lot_a_id, lot_b_id, to_id)
    }

    // ── KAT-C2f — cancel-path bytes-unchanged (select-lots) ──────────────────

    #[test]
    fn kat_c2f_cancel_path_vault_bytes_unchanged_select_lots() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2f-pass";

        // Seed: 2 lots (1M sat each) + TransferOut 500K + ReclassifyOutflow(Sell) + MethodElection.
        // FIFO consumes 500K from lot A → lot A remaining = 500K, lot B remaining = 1M.
        // snap.state.lots is non-empty so LotsForm can open.
        let (_lot_a, _lot_b, _to_id) = seed_two_lot_sell_vault(&vault, &key, pp_str);

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // ── s → flow opens at List step ──────────────────────────────────
            handle_key(&mut app, press(KeyCode::Char('s')));
            assert!(
                app.select_lots_flow.is_some(),
                "C2f: select_lots_flow must open on 's'"
            );

            // 'q' at List step is SWALLOWED [I4].
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2f: 'q' at List must be swallowed");
            assert!(
                app.select_lots_flow.is_some(),
                "C2f: flow must still be open after swallowed 'q'"
            );

            // Enter → LotsForm.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.select_lots_flow.as_ref().map(|f| &f.step),
                    Some(SelectLotsStep::LotsForm { .. })
                ),
                "C2f: Enter must transition to LotsForm"
            );

            // 'q' at LotsForm is SWALLOWED.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2f: 'q' at LotsForm must be swallowed");

            // Type "500000" → pick_sat_buf for lot A (cursor=0).
            // Disposal principal is 500K sat; lot A remaining = 500K (FIFO consumed 500K of 1M).
            for c in "500000".chars() {
                handle_key(&mut app, press(KeyCode::Char(c)));
            }

            // Enter → modal opens (picks == principal).
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.select_lots_modal.is_some(),
                "C2f: Enter on valid picks must open select_lots_modal"
            );

            // 'q' at modal is SWALLOWED.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2f: 'q' at modal must be swallowed");

            // Esc → modal closes; LotsForm stays open with buffer intact.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.select_lots_modal.is_none(), "C2f: Esc must close modal");
            assert!(
                matches!(
                    app.select_lots_flow.as_ref().map(|f| &f.step),
                    Some(SelectLotsStep::LotsForm { .. })
                ),
                "C2f: LotsForm must still be open after Esc-close-modal"
            );
            // Buffer intact.
            {
                let flow = app.select_lots_flow.as_ref().unwrap();
                if let SelectLotsStep::LotsForm { rows, .. } = &flow.step {
                    assert_eq!(
                        rows[0].pick_sat_buf.buf, "500000",
                        "C2f: buffer must be intact after modal Esc"
                    );
                }
            }

            // Esc from LotsForm → back to List.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.select_lots_flow.as_ref().map(|f| &f.step),
                    Some(SelectLotsStep::List)
                ),
                "C2f: Esc from LotsForm must go back to List"
            );

            // Esc from List → flow closes.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.select_lots_flow.is_none(),
                "C2f: Esc from List must close flow"
            );
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2f: vault bytes must be UNCHANGED on full cancel path"
        );
    }

    // ── KAT-C2g — cancel-path bytes-unchanged (set-donation-details) ─────────

    #[test]
    fn kat_c2g_cancel_path_vault_bytes_unchanged_set_donation_details() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2g-pass";

        seed_donate_vault(&vault, &key, pp_str, 500_000);

        let bytes_before = std::fs::read(&vault).unwrap();

        {
            let mut app = open_app(&vault, pp_str);

            // ── d → flow opens at List step ──────────────────────────────────
            handle_key(&mut app, press(KeyCode::Char('d')));
            assert!(
                app.set_donation_details_flow.is_some(),
                "C2g: set_donation_details_flow must open on 'd'"
            );

            // 'q' at List is SWALLOWED.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2g: 'q' at List must be swallowed");

            // Enter → FieldForm.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.set_donation_details_flow.as_ref().map(|f| &f.step),
                    Some(SetDonationDetailsStep::FieldForm { .. })
                ),
                "C2g: Enter must transition to FieldForm"
            );

            // 'q' at FieldForm is pushed to donee_name_buf (NOT swallowed as quit) [I4].
            // We skip the 'q'-swallow check here since 'q' is a valid text char for the field.

            // Type donee_name.
            type_str(&mut app, "Test Charity");

            // Move focus to appraiser_name (field 3, index 3).
            for _ in 0..3 {
                handle_key(&mut app, press(KeyCode::Down));
            }
            type_str(&mut app, "Jane Appraiser");

            // Enter → modal opens.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.set_donation_details_modal.is_some(),
                "C2g: Enter on valid FieldForm must open set_donation_details_modal"
            );

            // Esc → modal closes; FieldForm stays open.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.set_donation_details_modal.is_none(),
                "C2g: Esc must close modal"
            );
            assert!(
                matches!(
                    app.set_donation_details_flow.as_ref().map(|f| &f.step),
                    Some(SetDonationDetailsStep::FieldForm { .. })
                ),
                "C2g: FieldForm must still be open after modal Esc"
            );

            // Esc from FieldForm → back to List.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.set_donation_details_flow.as_ref().map(|f| &f.step),
                    Some(SetDonationDetailsStep::List)
                ),
                "C2g: Esc from FieldForm must go back to List"
            );

            // Esc from List → flow closes.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.set_donation_details_flow.is_none(),
                "C2g: Esc from List must close flow"
            );
        }

        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2g: vault bytes must be UNCHANGED on full cancel path"
        );
    }

    // ── KAT-S3a — save-error path for select-lots (chmod) ────────────────────

    #[test]
    #[cfg(unix)]
    fn kat_s3a_save_error_path_select_lots_chmod() {
        use btctax_core::persistence::load_all_ordered;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-s3a-pass";

        // Seed: 2 lots (1M sat each) + TransferOut 500K + ReclassifyOutflow(Sell) + MethodElection.
        // FIFO takes 500K from lot A → remaining = 500K; snap.state.lots is non-empty.
        let (_lot_a, _lot_b, _to_id) = seed_two_lot_sell_vault(&vault, &key, pp_str);

        // Root-skip guard.
        {
            let probe = dir.path().join("probe.tmp");
            let perms = std::fs::Permissions::from_mode(0o500);
            std::fs::set_permissions(dir.path(), perms).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("KAT-S3a: skipping — chmod 0o500 did not deny writes (running as root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();
        let pre_event_count = {
            let session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            load_all_ordered(session.conn()).unwrap().len()
        };

        let mut app = open_app(&vault, pp_str);

        // Drive s → LotsForm → pick 500000 → modal.
        // Lot A (cursor=0) has 500K remaining; disposal principal is 500K. Pick exact.
        handle_key(&mut app, press(KeyCode::Char('s')));
        handle_key(&mut app, press(KeyCode::Enter)); // List → LotsForm
        for c in "500000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.select_lots_modal.is_some(), "S3a: modal must open");

        // Make vault's parent dir read-only.
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();

        // Confirm → should fail.
        handle_key(&mut app, press(KeyCode::Enter));

        // Restore before any assertions that might panic and leave dir locked.
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        // Assertions: modal closed, LotsForm still open, buffer intact, status "Save error".
        assert!(
            app.select_lots_modal.is_none(),
            "S3a: modal must be closed after save failure"
        );
        assert!(
            matches!(
                app.select_lots_flow.as_ref().map(|f| &f.step),
                Some(SelectLotsStep::LotsForm { .. })
            ),
            "S3a: LotsForm must still be open after save failure"
        );
        {
            let flow = app.select_lots_flow.as_ref().unwrap();
            if let SelectLotsStep::LotsForm { rows, .. } = &flow.step {
                assert_eq!(
                    rows[0].pick_sat_buf.buf, "500000",
                    "S3a: buffer must be intact after save failure"
                );
            }
        }
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error"))
                .unwrap_or(false),
            "S3a: status must contain 'Save error'; got: {:?}",
            app.status
        );
        let bytes_mid = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_mid,
            "S3a: vault bytes must be unchanged after failed save"
        );
        // [save-rollback] no residue: the in-memory log is reverted to pre after the failed save.
        let mid_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();
        assert_eq!(
            mid_len, pre_event_count,
            "S3a: rollback must revert the in-memory append (no residue after a failed save)"
        );

        // Retry → clean single LotSelection (the rolled-back first attempt left no residue).
        handle_key(&mut app, press(KeyCode::Enter)); // validate again → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → retry save
        assert!(
            app.select_lots_modal.is_none(),
            "S3a: modal must be closed after retry save"
        );
        assert!(
            app.select_lots_flow.is_none(),
            "S3a: flow must be closed after retry"
        );

        // Assert: exactly ONE LotSelection appended (rollback → no residue), NO DecisionConflict.
        let status_after_retry = app.status.clone();
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre_event_count + 1,
            "S3a: retry after a rolled-back save must append EXACTLY ONE LotSelection (no residue); pre={pre_event_count}"
        );

        // The single tail is a LotSelection.
        let tail = &post[pre_event_count];
        let p: btctax_core::EventPayload =
            serde_json::from_str(&tail.payload_json).expect("tail must deserialize");
        assert!(
            matches!(p, btctax_core::EventPayload::LotSelection(_)),
            "S3a: the new tail must be LotSelection"
        );

        // Re-project: NO DecisionConflict (the failed-save residue that would have conflicted was
        // reverted; a lone valid selection applies cleanly).
        let (snap, _) = btctax_tui::unlock::build_snapshot(&session2).unwrap();
        assert!(
            !snap
                .state
                .blockers
                .iter()
                .any(|b| b.kind == btctax_core::BlockerKind::DecisionConflict),
            "S3a: a clean retry must fire NO DecisionConflict; blockers: {:?}",
            snap.state.blockers
        );
        assert!(
            !status_after_retry
                .as_deref()
                .unwrap_or("")
                .contains("DecisionConflict"),
            "S3a: clean-retry status must not mention DecisionConflict; got: {status_after_retry:?}"
        );
    }

    // ── save-rollback: residue-latch producer + consumer ─────────────────────

    /// [R0-I1] PRODUCER test: `on_persist_error` is the SINGLE site that arms `rollback_failed`.
    /// The runtime trigger (a restore OOM) is not inducible, so hand-build each `PersistError`
    /// variant and assert the effect directly.
    #[test]
    fn kat_on_persist_error_residue_live_arms_latch() {
        use edit::persist::PersistError;

        // ResidueLive → arm the latch + CRITICAL status.
        let mut app = EditorApp::new(std::path::PathBuf::from("/nonexistent"));
        app.on_persist_error(PersistError::ResidueLive(btctax_cli::CliError::Usage(
            "induced".to_string(),
        )));
        assert!(
            app.rollback_failed,
            "ResidueLive must arm the rollback_failed latch"
        );
        let s = app.status.as_deref().unwrap_or("");
        assert!(
            s.contains("could not be reverted") && s.contains("Quit the editor NOW"),
            "ResidueLive status must be the CRITICAL residue message; got: {s:?}"
        );

        // NoChange / RolledBack → benign, no latch, "safe to retry".
        for benign in [
            PersistError::NoChange(btctax_cli::CliError::Usage("x".into())),
            PersistError::RolledBack(btctax_cli::CliError::Usage("y".into())),
        ] {
            let mut a = EditorApp::new(std::path::PathBuf::from("/nonexistent"));
            a.on_persist_error(benign);
            assert!(!a.rollback_failed, "benign arms must NOT arm the latch");
            assert!(
                a.status
                    .as_deref()
                    .unwrap_or("")
                    .contains("no changes were recorded; safe to retry"),
                "benign status must be the safe-to-retry message; got: {:?}",
                a.status
            );
        }
    }

    /// `residue_latch_status` precedence: attest wording (verbatim — ERRLATCH regression guard) wins
    /// over rollback wording; `None` when neither latch is set.
    #[test]
    fn kat_residue_latch_status_precedence() {
        let mut a = EditorApp::new(std::path::PathBuf::from("/x"));
        assert!(a.residue_latch_status().is_none(), "no latch → None");
        a.rollback_failed = true;
        assert!(
            a.residue_latch_status()
                .unwrap()
                .contains("could not be reverted"),
            "rollback_failed → CRITICAL residue wording"
        );
        a.attest_save_failed = true; // attest takes precedence
        assert!(
            a.residue_latch_status()
                .unwrap()
                .contains("failed attest save"),
            "attest_save_failed must keep its verbatim wording (ERRLATCH regression guard)"
        );
    }

    /// CONSUMER test: while `rollback_failed` is set, EVERY mutating opener (p/c/o/r/f/v/s/d/a)
    /// refuses with the CRITICAL residue status and opens no flow. Mirrors the attest ERRLATCH loop.
    #[test]
    fn kat_rollback_failed_latch_refuses_all_openers() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-rollback-latch-pass";
        seed_transfer_in_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);
        app.rollback_failed = true;

        for k in ['p', 'c', 'o', 'r', 'f', 'v', 's', 'd', 'a'] {
            app.status = None;
            handle_key(&mut app, press(KeyCode::Char(k)));
            assert!(
                app.profile_form.is_none()
                    && app.classify_inbound_flow.is_none()
                    && app.reclassify_outflow_flow.is_none()
                    && app.reclassify_income_flow.is_none()
                    && app.set_fmv_flow.is_none()
                    && app.void_flow.is_none()
                    && app.select_lots_flow.is_none()
                    && app.set_donation_details_flow.is_none()
                    && app.safe_harbor_attest_flow.is_none(),
                "rollback latch: opener '{k}' must open no mutating flow"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("could not be reverted"))
                    .unwrap_or(false),
                "rollback latch: opener '{k}' must show the CRITICAL residue status; got: {:?}",
                app.status
            );
        }
    }

    // ── KAT-E2E-SL — end-to-end select-lots (discriminating seed) ─────────────

    #[test]
    fn kat_e2e_sl_select_lots_happy_path_discriminating_seed() {
        use ratatui::{backend::TestBackend, Terminal};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-sl-pass";

        let (lot_a_id, lot_b_id, to_id) = seed_two_lot_sell_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // 1. Confirm disposal is in projected state.
        let snap = app.snapshot.as_ref().unwrap();
        let disposal_present = snap.state.disposals.iter().any(|d| d.event == to_id);
        assert!(
            disposal_present,
            "E2E-SL: TransferOut/sell must appear in snap.state.disposals"
        );

        // Under FIFO, lot A (earlier) is consumed.
        let uses_lot_a = snap.state.disposals.iter().any(|d| {
            d.event == to_id && d.legs.iter().any(|l| l.lot_id.origin_event_id == lot_a_id)
        });
        assert!(
            uses_lot_a,
            "E2E-SL: FIFO must consume lot A before select-lots"
        );

        // 2. Drive s → list → Enter → LotsForm.
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert!(
            app.select_lots_flow.is_some(),
            "E2E-SL: flow must open on 's'"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // List → LotsForm
        assert!(
            matches!(
                app.select_lots_flow.as_ref().map(|f| &f.step),
                Some(SelectLotsStep::LotsForm { .. })
            ),
            "E2E-SL: Enter must transition to LotsForm"
        );

        // LotsForm shows BOTH lots (sorted by acquired_at ASC = A first, B second).
        let rows_len = app.select_lots_flow.as_ref().and_then(|f| {
            if let SelectLotsStep::LotsForm { rows, .. } = &f.step {
                Some(rows.len())
            } else {
                None
            }
        });
        assert_eq!(rows_len, Some(2), "E2E-SL: LotsForm must show 2 lots");

        // Navigate to lot B (index 1) and type "500000".
        handle_key(&mut app, press(KeyCode::Down)); // cursor → lot B (index 1)
        for c in "500000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }

        // Enter → modal.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.select_lots_modal.is_some(),
            "E2E-SL: Enter must open modal"
        );

        // Render and check modal content.
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| crate::draw_edit::draw(f, &mut app))
            .unwrap();
        let rendered = rendered_text(&terminal);
        assert!(
            rendered.contains("sell") || rendered.contains("(sell)"),
            "E2E-SL: modal must show disposal kind 'sell'; rendered: {rendered}"
        );
        assert!(
            rendered.contains("500000"),
            "E2E-SL: modal must show 500000 sat; rendered: {rendered}"
        );

        // Confirm → save + re-project.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.select_lots_modal.is_none(),
            "E2E-SL: modal must close after confirm"
        );
        assert!(
            app.select_lots_flow.is_none(),
            "E2E-SL: flow must close after confirm"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Lot selection"))
                .unwrap_or(false),
            "E2E-SL: status must contain 'Lot selection'; got: {:?}",
            app.status
        );

        // 3. Re-project: no LotSelectionInvalid; disposal now consumes lot B.
        let snap2 = app.snapshot.as_ref().unwrap();
        let no_invalid = !snap2.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::LotSelectionInvalid && b.event.as_ref() == Some(&to_id)
        });
        assert!(
            no_invalid,
            "E2E-SL: no LotSelectionInvalid after valid selection"
        );

        let uses_lot_b = snap2.state.disposals.iter().any(|d| {
            d.event == to_id && d.legs.iter().any(|l| l.lot_id.origin_event_id == lot_b_id)
        });
        assert!(
            uses_lot_b,
            "E2E-SL: re-projected disposal must consume lot B (non-FIFO specific-ID overrides FIFO)"
        );

        // 4. The disposal NO LONGER appears in the 's' list (already-selected pre-filter).
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert!(
            app.select_lots_flow.is_none(),
            "E2E-SL: flow must NOT open — no eligible disposals after selection (pre-filter)"
        );
        assert!(
            app.status.is_some(),
            "E2E-SL: status must be set when no eligible disposals"
        );
    }

    // ── KAT-E2E-SL-DONATE — select-lots through a Donate removal ─────────────

    #[test]
    fn kat_e2e_sl_donate_wallet_sourced_from_raw_event() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-sl-donate-pass";

        // Seed: Acquire (wallet W) + TransferOut + ReclassifyOutflow(Donate).
        let (_, to_id) = seed_donate_vault(&vault, &key, pp_str, 500_000);

        let mut app = open_app(&vault, pp_str);

        // 1. The donation removal appears in projected state.
        let snap = app.snapshot.as_ref().unwrap();
        let removal_present = snap.state.removals.iter().any(|r| r.event == to_id);
        assert!(
            removal_present,
            "E2E-SL-DONATE: Donation removal must appear in snap.state.removals"
        );

        // 2. Drive s → list shows the donate removal.
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert!(
            app.select_lots_flow.is_some(),
            "E2E-SL-DONATE: flow must open"
        );

        // Verify the list contains the donate removal and its wallet comes from raw LedgerEvent.
        let flow = app.select_lots_flow.as_ref().unwrap();
        let donate_item = flow
            .list
            .items
            .iter()
            .find(|item| item.disposal_event == to_id);
        assert!(
            donate_item.is_some(),
            "E2E-SL-DONATE: dispose list must contain the donate removal"
        );
        let donate_item = donate_item.unwrap();
        assert_eq!(
            donate_item.kind,
            DisposalKind::Donate,
            "E2E-SL-DONATE: kind must be Donate"
        );
        // Wallet comes from raw LedgerEvent (RemovalLeg has no wallet field [R0-I1]).
        assert!(
            donate_item.wallet.is_some(),
            "E2E-SL-DONATE: wallet must be sourced from raw LedgerEvent (not None)"
        );

        // Enter → LotsForm with wallet-W lots.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.select_lots_flow.as_ref().map(|f| &f.step),
                Some(SelectLotsStep::LotsForm { .. })
            ),
            "E2E-SL-DONATE: Enter must open LotsForm"
        );

        // Pick the full principal (500000 sat).
        for c in "500000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(
            app.select_lots_modal.is_some(),
            "E2E-SL-DONATE: modal must open"
        );

        // Confirm → save.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.select_lots_modal.is_none(),
            "E2E-SL-DONATE: modal must close"
        );
        assert!(
            app.select_lots_flow.is_none(),
            "E2E-SL-DONATE: flow must close"
        );

        // Re-project: no LotSelectionInvalid.
        let snap2 = app.snapshot.as_ref().unwrap();
        let no_invalid = !snap2.state.blockers.iter().any(|b| {
            b.kind == BlockerKind::LotSelectionInvalid && b.event.as_ref() == Some(&to_id)
        });
        assert!(
            no_invalid,
            "E2E-SL-DONATE: no LotSelectionInvalid after valid donation selection"
        );
    }

    // ── KAT-E2E-SL-VOID — select-lots + void round-trip ─────────────────────

    #[test]
    fn kat_e2e_sl_void_lot_selection_re_appears_in_list() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-sl-void-pass";

        let (lot_a_id, _lot_b_id, to_id) = seed_two_lot_sell_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // Step 1: select lots (lot B, non-FIFO).
        handle_key(&mut app, press(KeyCode::Char('s')));
        handle_key(&mut app, press(KeyCode::Enter)); // → LotsForm
        handle_key(&mut app, press(KeyCode::Down)); // cursor to lot B
        for c in "500000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save

        assert!(
            app.select_lots_flow.is_none(),
            "SL-VOID: flow must close after save"
        );

        // Get the LotSelection decision_id from the status (we need the id for void).
        let snap_after_select = app.snapshot.as_ref().unwrap();
        let selection_decision_id = snap_after_select
            .events
            .iter()
            .rev()
            .find(|e| matches!(&e.payload, btctax_core::EventPayload::LotSelection(_)))
            .map(|e| e.id.clone())
            .expect("SL-VOID: LotSelection decision must exist after save");

        // Step 2: void the LotSelection.
        handle_key(&mut app, press(KeyCode::Char('v'))); // open void flow
        assert!(app.void_flow.is_some(), "SL-VOID: void flow must open");
        // Find and select the LotSelection in the void list.
        let lot_sel_idx = app
            .void_flow
            .as_ref()
            .unwrap()
            .list
            .items
            .iter()
            .position(|item| item.event_id == selection_decision_id);
        assert!(
            lot_sel_idx.is_some(),
            "SL-VOID: LotSelection must be in void list"
        );
        // Navigate to it.
        let target_idx = lot_sel_idx.unwrap();
        for _ in 0..target_idx {
            handle_key(&mut app, press(KeyCode::Down));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → void modal
        assert!(app.void_modal.is_some(), "SL-VOID: void modal must open");
        handle_key(&mut app, press(KeyCode::Enter)); // confirm void
        assert!(
            app.void_flow.is_none(),
            "SL-VOID: void flow must close after confirm"
        );

        // Step 3: disposal re-appears in select-lots list.
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert!(
            app.select_lots_flow.is_some(),
            "SL-VOID: disposal must re-appear in select-lots list after void"
        );

        // Verify FIFO restored (lot A consumed again after voiding the specific-ID).
        let snap_after_void = app.snapshot.as_ref().unwrap();
        let uses_lot_a_again = snap_after_void.state.disposals.iter().any(|d| {
            d.event == to_id && d.legs.iter().any(|l| l.lot_id.origin_event_id == lot_a_id)
        });
        assert!(
            uses_lot_a_again,
            "SL-VOID: after voiding LotSelection, disposal must revert to FIFO (lot A)"
        );

        // Close flow.
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(
            app.select_lots_flow.is_none(),
            "SL-VOID: Esc must close flow"
        );

        // Verify optimize_attestation was cleared (chunk-2b persist_void side-effect).
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let attest = btctax_cli::optimize_attest::get(session2.conn(), &to_id).unwrap();
        assert!(
            attest.is_none(),
            "SL-VOID: optimize_attestation must be None for the disposal (no attest was set)"
        );
    }

    // ── KAT-E2E-DD — end-to-end set-donation-details (completeness progression)

    #[test]
    fn kat_e2e_dd_donation_details_completeness_progression() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-dd-pass";

        let (_, to_id) = seed_donate_vault(&vault, &key, pp_str, 500_000);

        let mut app = open_app(&vault, pp_str);

        // Step 1: donation appears in list with "(none)" completeness.
        handle_key(&mut app, press(KeyCode::Char('d')));
        assert!(
            app.set_donation_details_flow.is_some(),
            "E2E-DD: flow must open"
        );
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            let item = flow.list.items.iter().find(|i| i.event_id == to_id);
            assert!(item.is_some(), "E2E-DD: donation must appear in list");
            assert_eq!(
                item.unwrap().completeness_str(),
                "(none)",
                "E2E-DD: initial completeness must be (none)"
            );
        }

        // Step 2: Enter → FieldForm; fill only required fields.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.set_donation_details_flow.as_ref().map(|f| &f.step),
                Some(SetDonationDetailsStep::FieldForm { .. })
            ),
            "E2E-DD: Enter must open FieldForm"
        );

        // Fill donee_name (field 0 — already focused).
        type_str(&mut app, "Community Foundation");
        // Move to appraiser_name (field 3).
        for _ in 0..3 {
            handle_key(&mut app, press(KeyCode::Down));
        }
        type_str(&mut app, "Jane Appraiser");

        // Enter → modal.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.set_donation_details_modal.is_some(),
            "E2E-DD: modal must open"
        );

        // Confirm → save.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.set_donation_details_modal.is_none(),
            "E2E-DD: modal must close"
        );
        assert!(
            app.set_donation_details_flow.is_none(),
            "E2E-DD: flow must close"
        );

        // Step 3: status = "Section A complete on presence".
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("Section A complete on presence"),
            "E2E-DD: status must say 'Section A complete on presence'; got: {status}"
        );

        // Step 4: re-open; list shows "present" completeness; FieldForm pre-populated.
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('d')));
        assert!(
            app.set_donation_details_flow.is_some(),
            "E2E-DD: flow must re-open"
        );
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            let item = flow.list.items.iter().find(|i| i.event_id == to_id);
            assert!(
                item.is_some(),
                "E2E-DD: donation must appear in list on re-open"
            );
            assert_eq!(
                item.unwrap().completeness_str(),
                "present",
                "E2E-DD: completeness must be 'present' after initial save"
            );
        }

        handle_key(&mut app, press(KeyCode::Enter)); // → FieldForm
                                                     // Verify pre-populated: donee_name should be "Community Foundation".
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            if let SetDonationDetailsStep::FieldForm {
                donee_name_buf,
                appraiser_name_buf,
                ..
            } = &flow.step
            {
                assert_eq!(
                    donee_name_buf.buf.trim(),
                    "Community Foundation",
                    "E2E-DD: donee_name_buf must be pre-populated"
                );
                assert_eq!(
                    appraiser_name_buf.buf.trim(),
                    "Jane Appraiser",
                    "E2E-DD: appraiser_name_buf must be pre-populated"
                );
            }
        }

        // Add fields for Section B completeness: appraiser_tin (5), appraisal_date (8),
        // appraiser_qualifications (7), donee_ein (2).
        // Navigate to appraiser_tin (field 5).
        for _ in 0..5 {
            handle_key(&mut app, press(KeyCode::Down));
        }
        type_str(&mut app, "987654321"); // appraiser_tin

        // Navigate to appraiser_qualifications (field 7 = 2 more Down).
        handle_key(&mut app, press(KeyCode::Down)); // → 6
        handle_key(&mut app, press(KeyCode::Down)); // → 7
        type_str(&mut app, "certified bitcoin appraiser");

        // Navigate to appraisal_date (field 8).
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "2025-05-20");

        // Navigate to donee_ein (field 2) — go back to top first.
        handle_key(&mut app, press(KeyCode::Up)); // → 7
        handle_key(&mut app, press(KeyCode::Up)); // → 6
        handle_key(&mut app, press(KeyCode::Up)); // → 5
        handle_key(&mut app, press(KeyCode::Up)); // → 4
        handle_key(&mut app, press(KeyCode::Up)); // → 3
        handle_key(&mut app, press(KeyCode::Up)); // → 2
        type_str(&mut app, "12-3456789"); // donee_ein

        // Enter → modal.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.set_donation_details_modal.is_some(),
            "E2E-DD step4: modal must open"
        );

        // Confirm → save.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.set_donation_details_modal.is_none(),
            "E2E-DD step4: modal must close"
        );
        assert!(
            app.set_donation_details_flow.is_none(),
            "E2E-DD step4: flow must close"
        );

        // Step 5: status = "Section B complete".
        let status2 = app.status.as_deref().unwrap_or("");
        assert!(
            status2.contains("Section B complete"),
            "E2E-DD step5: status must say 'Section B complete'; got: {status2}"
        );

        // List now shows "B-complete".
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('d')));
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            let item = flow.list.items.iter().find(|i| i.event_id == to_id);
            assert_eq!(
                item.unwrap().completeness_str(),
                "B-complete",
                "E2E-DD: completeness must be 'B-complete' after Section B save"
            );
        }
        handle_key(&mut app, press(KeyCode::Esc));
    }

    // ── KAT-FREETEXT-CAP (#6) — a >64-char free-text field round-trips ────────
    //
    // The donation FREE-TEXT buffers use `FieldBuffer::with_cap(FREETEXT_CAP=512)`, so a
    // 200-char appraiser_qualifications is NOT truncated (the CLI is unbounded). Type it,
    // save, reload → assert the full 200 chars round-trip.

    #[test]
    fn kat_freetext_cap_long_qualifications_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-freetext-cap-pass";

        let (_, to_id) = seed_donate_vault(&vault, &key, pp_str, 500_000);
        let long_qual = "A".repeat(200); // > FIELD_CAP (64), < FREETEXT_CAP (512)

        let mut app = open_app(&vault, pp_str);

        // d → List → Enter → FieldForm.
        handle_key(&mut app, press(KeyCode::Char('d')));
        handle_key(&mut app, press(KeyCode::Enter));

        // donee_name (field 0, focused) — REQUIRED.
        type_str(&mut app, "Community Foundation");
        // appraiser_name (field 3) — REQUIRED.
        for _ in 0..3 {
            handle_key(&mut app, press(KeyCode::Down));
        }
        type_str(&mut app, "Jane Appraiser");
        // appraiser_qualifications (field 7) — free-text, 200 chars.
        for _ in 0..4 {
            handle_key(&mut app, press(KeyCode::Down));
        }
        type_str(&mut app, &long_qual);

        // The free-text buffer must hold ALL 200 chars (not truncated at 64).
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            if let SetDonationDetailsStep::FieldForm {
                appraiser_qualifications_buf,
                ..
            } = &flow.step
            {
                assert_eq!(
                    appraiser_qualifications_buf.buf.len(),
                    200,
                    "FREETEXT-CAP: free-text buffer must hold all 200 chars pre-save"
                );
            } else {
                panic!("FREETEXT-CAP: expected FieldForm step");
            }
        }

        // Enter → modal → Enter → save.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.set_donation_details_modal.is_some(),
            "FREETEXT-CAP: modal must open"
        );
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.set_donation_details_flow.is_none(),
            "FREETEXT-CAP: flow must close after save"
        );

        // Reload: d → List → Enter → FieldForm; qualifications must round-trip in FULL.
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('d')));
        handle_key(&mut app, press(KeyCode::Enter));
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            if let SetDonationDetailsStep::FieldForm {
                appraiser_qualifications_buf,
                ..
            } = &flow.step
            {
                assert_eq!(
                    appraiser_qualifications_buf.buf.trim(),
                    long_qual,
                    "FREETEXT-CAP: 200-char qualifications must round-trip fully after reload"
                );
            } else {
                panic!("FREETEXT-CAP: expected FieldForm step on reload");
            }
        }
        // Also confirm the persisted side-table value is the full 200 chars.
        let snap = app.snapshot.as_ref().unwrap();
        let details = snap.donation_details.get(&to_id).unwrap();
        assert_eq!(
            details.appraiser_qualifications.as_deref(),
            Some(long_qual.as_str()),
            "FREETEXT-CAP: persisted qualifications must be the full 200 chars"
        );
        handle_key(&mut app, press(KeyCode::Esc));
    }

    // ── KAT-STRUCTURED-CAP (#6) — a STRUCTURED field still caps at 64 ─────────
    //
    // donee_ein is a fixed-format field: it keeps `FieldBuffer::new()` (FIELD_CAP=64).
    // Typing 100 chars must cap at 64 (a 512-char EIN is nonsense + a render hazard).

    #[test]
    fn kat_structured_cap_donee_ein_caps_at_64() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-structured-cap-pass";

        seed_donate_vault(&vault, &key, pp_str, 500_000);

        let mut app = open_app(&vault, pp_str);

        // d → List → Enter → FieldForm.
        handle_key(&mut app, press(KeyCode::Char('d')));
        handle_key(&mut app, press(KeyCode::Enter));

        // Navigate to donee_ein (field 2) and type 100 chars.
        for _ in 0..2 {
            handle_key(&mut app, press(KeyCode::Down));
        }
        type_str(&mut app, &"9".repeat(100));

        let flow = app.set_donation_details_flow.as_ref().unwrap();
        if let SetDonationDetailsStep::FieldForm { donee_ein_buf, .. } = &flow.step {
            assert_eq!(
                donee_ein_buf.buf.len(),
                64,
                "STRUCTURED-CAP: donee_ein must cap at FIELD_CAP (64), not FREETEXT_CAP"
            );
        } else {
            panic!("STRUCTURED-CAP: expected FieldForm step");
        }
    }

    // ── KAT-V-DD-4 — pre-population drives the PRODUCTION List→FieldForm mapping ─
    //
    // Round-1 whole-branch review [I1]: the prior form.rs `kat_v_dd_4_...` re-implemented
    // the 10-field pre-population mapping IN the test body (coverage theatre — dropping a
    // production optional-field pre-population passed uncaught, risking a last-write-wins
    // upsert of `None` over a stored field). This real-path version stores a full 10-field
    // DonationDetails, drives `d` → List → Enter → FieldForm (the production pre-population
    // at the List→FieldForm transition, main.rs), asserts EACH of the 10 buffers equals the
    // stored value, then Enter → modal to assert the validated `details` round-trip.

    #[test]
    fn kat_v_dd_4_pre_population_drives_real_path() {
        use time::macros::date;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-v-dd-4-pass";

        let (_, to_id) = seed_donate_vault(&vault, &key, pp_str, 500_000);

        // Store a FULL 10-field DonationDetails via the production side-table writer,
        // then drop the session (releasing the vault lock before open_app).
        let details = DonationDetails {
            donee_name: "Community Foundation".to_owned(),
            donee_address: Some("123 Charity Lane".to_owned()),
            donee_ein: Some("12-3456789".to_owned()),
            appraiser_name: "Jane Appraiser".to_owned(),
            appraiser_address: Some("456 Appraise Ave".to_owned()),
            appraiser_tin: Some("987654321".to_owned()),
            appraiser_ptin: Some("P01234567".to_owned()),
            appraiser_qualifications: Some("certified bitcoin appraiser".to_owned()),
            appraisal_date: Some(date!(2025 - 05 - 20)),
            fmv_method_override: Some("qualified appraisal".to_owned()),
        };
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            crate::edit::persist::persist_donation_details(&mut session, &to_id, &details).unwrap();
        }

        // Open the editor; snap.donation_details now carries the stored details.
        let mut app = open_app(&vault, pp_str);

        // d → List: the list item must carry existing_details from snap.donation_details.
        handle_key(&mut app, press(KeyCode::Char('d')));
        {
            let flow = app
                .set_donation_details_flow
                .as_ref()
                .expect("KAT-V-DD-4: flow must open");
            let item = flow
                .list
                .items
                .iter()
                .find(|i| i.event_id == to_id)
                .expect("KAT-V-DD-4: donation must appear in list");
            assert_eq!(
                item.existing_details.as_ref(),
                Some(&details),
                "KAT-V-DD-4: list item must carry the stored details from snap.donation_details"
            );
        }

        // Enter → FieldForm: runs the production pre-population mapping.
        handle_key(&mut app, press(KeyCode::Enter));

        // Assert EACH of the 10 buffers equals the stored value — a dropped or swapped
        // production pre-population line fails HERE (the [I1] regression guard).
        {
            let flow = app.set_donation_details_flow.as_ref().unwrap();
            let SetDonationDetailsStep::FieldForm {
                donee_name_buf,
                donee_address_buf,
                donee_ein_buf,
                appraiser_name_buf,
                appraiser_address_buf,
                appraiser_tin_buf,
                appraiser_ptin_buf,
                appraiser_qualifications_buf,
                appraisal_date_buf,
                fmv_method_override_buf,
                ..
            } = &flow.step
            else {
                panic!("KAT-V-DD-4: Enter must open FieldForm");
            };
            assert_eq!(
                donee_name_buf.buf.trim(),
                "Community Foundation",
                "donee_name"
            );
            assert_eq!(
                donee_address_buf.buf.trim(),
                "123 Charity Lane",
                "donee_address"
            );
            assert_eq!(donee_ein_buf.buf.trim(), "12-3456789", "donee_ein");
            assert_eq!(
                appraiser_name_buf.buf.trim(),
                "Jane Appraiser",
                "appraiser_name"
            );
            assert_eq!(
                appraiser_address_buf.buf.trim(),
                "456 Appraise Ave",
                "appraiser_address"
            );
            assert_eq!(appraiser_tin_buf.buf.trim(), "987654321", "appraiser_tin");
            assert_eq!(appraiser_ptin_buf.buf.trim(), "P01234567", "appraiser_ptin");
            assert_eq!(
                appraiser_qualifications_buf.buf.trim(),
                "certified bitcoin appraiser",
                "appraiser_qualifications"
            );
            assert_eq!(
                appraisal_date_buf.buf.trim(),
                "2025-05-20",
                "appraisal_date"
            );
            assert_eq!(
                fmv_method_override_buf.buf.trim(),
                "qualified appraisal",
                "fmv_method_override"
            );
        }

        // Enter → modal: the pre-populated buffers round-trip through the REAL validator
        // back to the exact stored details (retains the prior round-trip assertion, now
        // over production-populated buffers — strictly stronger).
        handle_key(&mut app, press(KeyCode::Enter));
        let modal = app
            .set_donation_details_modal
            .as_ref()
            .expect("KAT-V-DD-4: Enter on a valid FieldForm must open the modal");
        assert_eq!(
            modal.details, details,
            "KAT-V-DD-4: validated modal details must round-trip to the stored details"
        );
    }

    // ── Safe-harbor-attest flow KATs ─────────────────────────────────────────

    /// Seed a vault with one TIMEBARRED SafeHarborAllocation
    /// (`timely_allocation_attested: false`). Returns the allocation's EventId.
    ///
    /// Method = ProRata: an unattested ProRata allocation is timebarred
    /// UNCONDITIONALLY (resolve.rs pass-3 factored bar: `¬attested ∧ (past-bar ∨
    /// ProRata)`), independent of the made-date. An unattested ActualPosition
    /// allocation made before 2026-04-15 with no 2025 dispositions would instead be
    /// ALREADY EFFECTIVE — the pre-flight would refuse it (arm 5) and the flow would
    /// never open.
    fn seed_safe_harbor_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> btctax_core::EventId {
        use btctax_core::event::{AllocMethod, EventPayload, SafeHarborAllocation};
        use btctax_core::persistence::append_decision;
        use btctax_core::LotMethod;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();

        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
        let prior_id = append_decision(
            session.conn(),
            EventPayload::SafeHarborAllocation(SafeHarborAllocation {
                lots: vec![],
                as_of_date: date!(2025 - 01 - 01),
                method: AllocMethod::ProRata,
                timely_allocation_attested: false,
                pre2025_method: LotMethod::Fifo,
            }),
            t0,
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        session.save().unwrap();
        prior_id
    }

    // ── KAT-C2h — cancel-path bytes-unchanged (safe-harbor-attest) ────────────
    //
    // Spec D5: seed a valid timebarred allocation. `a` → Info; `Enter` → TypedWord;
    // type partial word "ATT"; `Enter` → error shown, TypedWord stays open; `Esc` →
    // back to Info [I4]; `Esc` → flow closes. `q` swallowed at each step.
    // bytes_after == bytes_before. (Complement: KAT-E2E-ATTEST writes.)

    #[test]
    fn kat_c2h_cancel_path_vault_bytes_unchanged_attest() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2h-pass";

        seed_safe_harbor_vault(&vault, &key, pp_str);
        let bytes_before = std::fs::read(&vault).unwrap();

        let mut app = open_app(&vault, pp_str);

        // 'a' → flow opens at Info step.
        handle_key(&mut app, press(KeyCode::Char('a')));
        assert!(
            matches!(
                app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAttestStep::Info)
            ),
            "C2h: flow must open at Info step"
        );

        // 'q' swallowed at Info step (flow is blocking).
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(!app.should_quit, "C2h: 'q' at Info must NOT quit");
        assert!(
            app.safe_harbor_attest_flow.is_some(),
            "C2h: 'q' at Info must NOT close the flow"
        );

        // Enter → TypedWord step.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAttestStep::TypedWord { .. })
            ),
            "C2h: Enter at Info must advance to TypedWord"
        );

        // Type partial word "ATT"; Enter → error shown, TypedWord stays open.
        type_str(&mut app, "ATT");
        handle_key(&mut app, press(KeyCode::Enter));
        match app.safe_harbor_attest_flow.as_ref().map(|f| &f.step) {
            Some(SafeHarborAttestStep::TypedWord { buf, error }) => {
                assert_eq!(buf.buf.as_str(), "ATT", "C2h: buffer must be preserved");
                assert_eq!(
                    error.as_deref(),
                    Some("type ATTEST (all caps) to confirm"),
                    "C2h: wrong-word error must match spec text"
                );
            }
            other => panic!("C2h: expected TypedWord after partial-word Enter; got {other:?}"),
        }

        // 'q' swallowed at TypedWord step (goes to the buffer, never quits).
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(!app.should_quit, "C2h: 'q' at TypedWord must NOT quit");
        assert!(
            app.safe_harbor_attest_flow.is_some(),
            "C2h: 'q' at TypedWord must NOT close the flow"
        );

        // Esc → back to Info step (one step per press — [I4]).
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(
            matches!(
                app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAttestStep::Info)
            ),
            "C2h: Esc at TypedWord must step back to Info (not close the flow)"
        );

        // Esc → flow closes.
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(
            app.safe_harbor_attest_flow.is_none(),
            "C2h: Esc at Info must close the flow"
        );
        assert!(!app.should_quit, "C2h: cancel path must never quit the app");

        // Vault bytes unchanged — nothing was written.
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2h: cancel path must leave vault bytes unchanged"
        );
        assert!(
            !app.attest_save_failed,
            "C2h: cancel path must never set the latch"
        );
    }

    // ── KAT-E2E-ATTEST-PREFLIGHT — all 4 failure arms + positive control ──────
    //
    // Spec D5: drive `a` with vaults covering each pre-flight failure arm:
    // 1. no allocation, 2. already-attested, 3. unconservable, 4. already-effective.
    // 5. positive control: timebarred allocation → flow opens at Info step.

    #[test]
    fn kat_e2e_attest_preflight_failure_arms_and_positive_control() {
        use btctax_core::event::{AllocMethod, EventPayload, SafeHarborAllocation};
        use btctax_core::persistence::append_decision;
        use btctax_core::LotMethod;
        use rust_decimal_macros::dec;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        // Helper: init a vault and append one SafeHarborAllocation.
        let seed_with = |vault: &std::path::Path,
                         key: &std::path::Path,
                         pp: &str,
                         alloc: SafeHarborAllocation| {
            btctax_cli::cmd::init::run(vault, &Passphrase::new(pp.into()), key).unwrap();
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp.into())).unwrap();
            let t0 = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
            append_decision(
                session.conn(),
                EventPayload::SafeHarborAllocation(alloc),
                t0,
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        };

        // ── Arm 1: no allocation ─────────────────────────────────────────────
        {
            let dir = tempfile::tempdir().unwrap();
            let vault = dir.path().join("vault.pgp");
            let key = dir.path().join("key.asc");
            let pp = "preflight-arm1";
            btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp.into()), &key).unwrap();

            let mut app = open_app(&vault, pp);
            handle_key(&mut app, press(KeyCode::Char('a')));
            assert!(
                app.safe_harbor_attest_flow.is_none(),
                "PREFLIGHT arm 1: flow must NOT open"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("No allocation to attest"))
                    .unwrap_or(false),
                "PREFLIGHT arm 1: status must say 'No allocation to attest'; got: {:?}",
                app.status
            );
        }

        // ── Arm 2: already attested (seed attested=true directly) ────────────
        {
            let dir = tempfile::tempdir().unwrap();
            let vault = dir.path().join("vault.pgp");
            let key = dir.path().join("key.asc");
            let pp = "preflight-arm2";
            seed_with(
                &vault,
                &key,
                pp,
                SafeHarborAllocation {
                    lots: vec![],
                    as_of_date: date!(2025 - 01 - 01),
                    method: AllocMethod::ActualPosition,
                    timely_allocation_attested: true,
                    pre2025_method: LotMethod::Fifo,
                },
            );

            let mut app = open_app(&vault, pp);
            handle_key(&mut app, press(KeyCode::Char('a')));
            assert!(
                app.safe_harbor_attest_flow.is_none(),
                "PREFLIGHT arm 2: flow must NOT open"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("Allocation already attested"))
                    .unwrap_or(false),
                "PREFLIGHT arm 2: status must say 'Allocation already attested'; got: {:?}",
                app.status
            );
        }

        // ── Arm 3: unconservable (allocation lists sat the vault does not hold) ──
        {
            let dir = tempfile::tempdir().unwrap();
            let vault = dir.path().join("vault.pgp");
            let key = dir.path().join("key.asc");
            let pp = "preflight-arm3";
            seed_with(
                &vault,
                &key,
                pp,
                SafeHarborAllocation {
                    lots: vec![btctax_core::event::AllocLot {
                        wallet: btctax_core::WalletId::Exchange {
                            provider: "River".to_string(),
                            account: "main".to_string(),
                        },
                        sat: 100_000,
                        usd_basis: dec!(1000),
                        acquired_at: date!(2024 - 06 - 01),
                        dual_loss_basis: None,
                        donor_acquired_at: None,
                    }],
                    as_of_date: date!(2025 - 01 - 01),
                    method: AllocMethod::ProRata,
                    timely_allocation_attested: false,
                    pre2025_method: LotMethod::Fifo,
                },
            );

            let mut app = open_app(&vault, pp);
            handle_key(&mut app, press(KeyCode::Char('a')));
            assert!(
                app.safe_harbor_attest_flow.is_none(),
                "PREFLIGHT arm 3: flow must NOT open"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("Allocation fails conservation"))
                    .unwrap_or(false),
                "PREFLIGHT arm 3: status must say 'Allocation fails conservation'; got: {:?}",
                app.status
            );
        }

        // ── Arm 4: already effective (unattested ActualPosition, made before the
        //    2026-04-15 due date, no 2025 dispositions → NOT timebarred; empty lots
        //    on an empty vault conserve → EFFECTIVE) ────────────────────────────
        {
            let dir = tempfile::tempdir().unwrap();
            let vault = dir.path().join("vault.pgp");
            let key = dir.path().join("key.asc");
            let pp = "preflight-arm4";
            seed_with(
                &vault,
                &key,
                pp,
                SafeHarborAllocation {
                    lots: vec![],
                    as_of_date: date!(2025 - 01 - 01),
                    method: AllocMethod::ActualPosition,
                    timely_allocation_attested: false,
                    pre2025_method: LotMethod::Fifo,
                },
            );

            let mut app = open_app(&vault, pp);
            handle_key(&mut app, press(KeyCode::Char('a')));
            assert!(
                app.safe_harbor_attest_flow.is_none(),
                "PREFLIGHT arm 4: flow must NOT open"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("Allocation already effective"))
                    .unwrap_or(false),
                "PREFLIGHT arm 4: status must say 'Allocation already effective'; got: {:?}",
                app.status
            );
        }

        // ── Positive control: timebarred allocation → flow opens at Info step ──
        {
            let dir = tempfile::tempdir().unwrap();
            let vault = dir.path().join("vault.pgp");
            let key = dir.path().join("key.asc");
            let pp = "preflight-pos";
            let prior_id = seed_safe_harbor_vault(&vault, &key, pp);

            let mut app = open_app(&vault, pp);
            handle_key(&mut app, press(KeyCode::Char('a')));
            assert!(
                matches!(
                    app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                    Some(SafeHarborAttestStep::Info)
                ),
                "PREFLIGHT positive control: flow must open at Info step"
            );
            let flow = app.safe_harbor_attest_flow.as_ref().unwrap();
            assert_eq!(
                flow.prior_id, prior_id,
                "PREFLIGHT positive control: flow.prior_id must match seeded allocation"
            );

            // Esc at Info closes the flow.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.safe_harbor_attest_flow.is_none(),
                "PREFLIGHT positive control: Esc at Info must close flow"
            );
        }
    }

    // ── KAT-E2E-ATTEST — happy path: type ATTEST, vault updated ──────────────

    #[test]
    fn kat_e2e_attest_happy_path() {
        use btctax_core::event::{EventPayload, SafeHarborAllocation};
        use btctax_core::persistence::load_all_ordered;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-attest-happy";

        let prior_id = seed_safe_harbor_vault(&vault, &key, pp_str);

        let bytes_before = std::fs::read(&vault).unwrap();
        let pre_count = {
            let s = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            load_all_ordered(s.conn()).unwrap().len()
        };
        assert_eq!(pre_count, 1, "ATTEST-HAPPY: pre must have 1 event");

        let mut app = open_app(&vault, pp_str);

        // 'a' → Info step.
        handle_key(&mut app, press(KeyCode::Char('a')));
        assert!(
            matches!(
                app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAttestStep::Info)
            ),
            "ATTEST-HAPPY: must open at Info step"
        );

        // Spec step 2: the Info display mentions "IRREVOCABLE" and the prior
        // allocation's canonical id.
        {
            use ratatui::{backend::TestBackend, Terminal};
            let backend = TestBackend::new(100, 30);
            let mut terminal = Terminal::new(backend).unwrap();
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
                rendered.contains("IRREVOCABLE"),
                "ATTEST-HAPPY: Info step must render 'IRREVOCABLE'"
            );
            assert!(
                rendered.contains(&prior_id.canonical()),
                "ATTEST-HAPPY: Info step must render the prior allocation's canonical id"
            );
        }

        // Enter → TypedWord step.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAttestStep::TypedWord { .. })
            ),
            "ATTEST-HAPPY: Enter at Info must advance to TypedWord"
        );

        // Spec step 3: type "ATTES" (incomplete) → Enter → error shown, TypedWord
        // still open, buffer PRESERVED [R0-I7].
        type_str(&mut app, "ATTES");
        handle_key(&mut app, press(KeyCode::Enter));
        match app.safe_harbor_attest_flow.as_ref().map(|f| &f.step) {
            Some(SafeHarborAttestStep::TypedWord { buf, error }) => {
                assert_eq!(
                    buf.buf.as_str(),
                    "ATTES",
                    "ATTEST-HAPPY: buffer must be preserved after incomplete word"
                );
                assert_eq!(
                    error.as_deref(),
                    Some("type ATTEST (all caps) to confirm"),
                    "ATTEST-HAPPY: incomplete word must show the spec error"
                );
            }
            other => panic!("ATTEST-HAPPY: expected TypedWord after 'ATTES'+Enter; got {other:?}"),
        }

        // Spec step 4: type "T" (completing "ATTEST" in the preserved buffer) → Enter → save.
        type_str(&mut app, "T");
        handle_key(&mut app, press(KeyCode::Enter));

        // Flow must be closed.
        assert!(
            app.safe_harbor_attest_flow.is_none(),
            "ATTEST-HAPPY: flow must be closed after successful attest"
        );

        // Latch must NOT be set.
        assert!(
            !app.attest_save_failed,
            "ATTEST-HAPPY: attest_save_failed must be false on success"
        );

        // Status is the clean arm: "Allocation attested (IRREVOCABLE, §7.4) — {id}; …".
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Allocation attested (IRREVOCABLE, §7.4)"))
                .unwrap_or(false),
            "ATTEST-HAPPY: status must be the clean attest arm; got: {:?}",
            app.status
        );

        // Vault must have changed (save happened).
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_ne!(
            bytes_before, bytes_after,
            "ATTEST-HAPPY: vault bytes must change after successful attest"
        );

        // Verify in-memory (app still holds the session — can't open a second session).
        let post_events = {
            let session = app.session.as_ref().unwrap();
            load_all_ordered(session.conn()).unwrap()
        };
        assert_eq!(
            post_events.len(),
            pre_count + 2,
            "ATTEST-HAPPY: post must have pre+2 events (void + re-attest)"
        );

        // The last event must be SafeHarborAllocation with timely_allocation_attested=true.
        let last: btctax_core::event::EventPayload =
            serde_json::from_str(&post_events.last().unwrap().payload_json).unwrap();
        match &last {
            EventPayload::SafeHarborAllocation(SafeHarborAllocation {
                timely_allocation_attested,
                ..
            }) => {
                assert!(
                    timely_allocation_attested,
                    "ATTEST-HAPPY: last event must have timely_allocation_attested=true"
                );
            }
            other => panic!("ATTEST-HAPPY: last event must be SafeHarborAllocation; got {other:?}"),
        }

        // Snapshot is rebuilt; NO SafeHarborTimebar attributed to the NEW allocation id.
        // Do NOT assert "no timebar anywhere" [R0-M10]: the voided PRIOR keeps firing a
        // stale Advisory SafeHarborTimebar on ITS id every projection.
        assert!(
            app.snapshot.is_some(),
            "ATTEST-HAPPY: snapshot must be rebuilt after attest"
        );
        {
            let new_attest_id = EventId::Decision {
                seq: post_events.last().unwrap().decision_seq.unwrap() as u64,
            };
            let snap = app.snapshot.as_ref().unwrap();
            assert!(
                !snap.state.blockers.iter().any(|b| {
                    b.kind == BlockerKind::SafeHarborTimebar
                        && b.event.as_ref() == Some(&new_attest_id)
                }),
                "ATTEST-HAPPY: NO SafeHarborTimebar may be attributed to the NEW allocation id"
            );
        }

        // Already-attested guard: pressing 'a' again must yield "Already attested" status.
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('a')));
        assert!(
            app.safe_harbor_attest_flow.is_none(),
            "ATTEST-HAPPY: flow must NOT open when already attested"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("already attested"))
                .unwrap_or(false),
            "ATTEST-HAPPY: 'a' after attest must yield 'already attested' status; got: {:?}",
            app.status
        );

        // prior_id must be in voided set.
        let void_event = &post_events[pre_count];
        let void_payload: EventPayload = serde_json::from_str(&void_event.payload_json).unwrap();
        match &void_payload {
            EventPayload::VoidDecisionEvent(v) => {
                assert_eq!(
                    v.target_event_id, prior_id,
                    "ATTEST-HAPPY: void must target prior_id"
                );
            }
            other => {
                panic!("ATTEST-HAPPY: event at pre.len() must be VoidDecisionEvent; got {other:?}")
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Chunk 5 — safe-harbor-ALLOCATE (`A`) flow: opener/eligibility + status + E2E
    // ══════════════════════════════════════════════════════════════════════════

    /// Seed a vault for the allocate opener. `with_pre2025_lot` adds a pre-2025 Acquire (River,
    /// 2024-06-01, 0.20 BTC) so the residue is non-empty; `attest_method` declares+attests FIFO (the
    /// config gate); `prior_alloc` pre-appends a live SafeHarborAllocation (step-5 guard fixture).
    fn seed_allocate_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
        with_pre2025_lot: bool,
        attest_method: bool,
        prior_alloc: Option<btctax_core::event::SafeHarborAllocation>,
    ) {
        use btctax_core::event::{Acquire, EventPayload, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use rust_decimal::Decimal;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        {
            let mut session =
                btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
            if with_pre2025_lot {
                let batch = vec![LedgerEvent {
                    id: EventId::import(Source::River, SourceRef::new("alloc-pre-lot")),
                    // 2024-06-01 (pre-2025 → PoolKey::Universal).
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_717_200_000).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: Some(river_wallet()),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 20_000_000,
                        usd_cost: Decimal::from(8550),
                        fee_usd: Decimal::ZERO,
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                }];
                append_import_batch(session.conn(), &batch).unwrap();
            }
            if let Some(a) = prior_alloc {
                append_decision(
                    session.conn(),
                    EventPayload::SafeHarborAllocation(a),
                    OffsetDateTime::from_unix_timestamp(1_741_100_000).unwrap(),
                    UtcOffset::UTC,
                    None,
                )
                .unwrap();
            }
            session.save().unwrap();
        }
        if attest_method {
            btctax_cli::cmd::admin::set_pre2025_method(
                vault,
                &Passphrase::new(pp_str.into()),
                btctax_core::LotMethod::Fifo,
                true,
            )
            .unwrap();
        }
    }

    /// Count non-voided-independent SafeHarborAllocation events currently in the HELD session's log.
    fn count_allocations(app: &EditorApp) -> usize {
        use btctax_core::persistence::load_all;
        let session = app.session.as_ref().unwrap();
        load_all(session.conn())
            .unwrap()
            .iter()
            .filter(|e| matches!(e.payload, EventPayload::SafeHarborAllocation(_)))
            .count()
    }

    // ── KAT-ALLOCATE-REFUSE-UNATTESTED (D1 step 3) ───────────────────────────
    #[test]
    fn kat_allocate_refuses_when_pre2025_method_unattested() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-alloc-unattested";
        // Pre-2025 lot present, but pre2025_method NOT attested.
        seed_allocate_vault(&vault, &key, pp_str, true, false, None);

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('A')));

        assert!(
            app.safe_harbor_allocate_flow.is_none(),
            "UNATTESTED: flow must NOT open when pre2025_method is unattested"
        );
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("Declare your filed pre-2025 method first")
                && status.contains("--attest-pre2025-method"),
            "UNATTESTED: status must direct to the CLI attest remedy; got: {status:?}"
        );
        assert_eq!(
            count_allocations(&app),
            0,
            "UNATTESTED: nothing may be appended"
        );
    }

    // ── KAT-ALLOCATE-REFUSE-LIVE-EXISTS (D1 step 5) ──────────────────────────
    #[test]
    fn kat_allocate_refuses_when_live_allocation_exists() {
        use btctax_core::event::{AllocMethod, SafeHarborAllocation};
        use btctax_core::LotMethod;
        use time::macros::date;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-alloc-live-exists";

        // Attested method + a pre-2025 lot (residue non-empty) + a pre-existing live allocation.
        let prior = SafeHarborAllocation {
            lots: vec![],
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ProRata,
            timely_allocation_attested: false,
            pre2025_method: LotMethod::Fifo,
        };
        seed_allocate_vault(&vault, &key, pp_str, true, true, Some(prior));

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('A')));

        assert!(
            app.safe_harbor_allocate_flow.is_none(),
            "LIVE-EXISTS: flow must NOT open when a live allocation already exists"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("An allocation already exists"),
            "LIVE-EXISTS: status must direct to attest/void; got: {:?}",
            app.status
        );
        assert_eq!(
            count_allocations(&app),
            1,
            "LIVE-EXISTS: no SECOND allocation may be appended (step-5 refuses)"
        );
    }

    // ── KAT-ALLOCATE-NOOP-EMPTY-RESIDUE (D1 step 4) ──────────────────────────
    #[test]
    fn kat_allocate_noop_when_residue_empty() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-alloc-empty";
        // Attested method but NO pre-2025 lot → the residue is empty.
        seed_allocate_vault(&vault, &key, pp_str, false, true, None);

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('A')));

        assert!(
            app.safe_harbor_allocate_flow.is_none(),
            "EMPTY: flow must NOT open when there is no pre-2025 residue"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("No pre-2025 lots to allocate"),
            "EMPTY: status must be the Path-A-applies message; got: {:?}",
            app.status
        );
        assert_eq!(count_allocations(&app), 0, "EMPTY: nothing appended");
    }

    // ── KAT-ALLOCATE-LATCH-REFUSES (D1 step 1) ───────────────────────────────
    #[test]
    fn kat_allocate_latch_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-alloc-latch";
        seed_allocate_vault(&vault, &key, pp_str, true, true, None);

        let mut app = open_app(&vault, pp_str);
        app.rollback_failed = true; // arm the unrevertable-residue latch
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('A')));

        assert!(
            app.safe_harbor_allocate_flow.is_none(),
            "LATCH: flow must NOT open while rollback_failed is set"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("could not be reverted"))
                .unwrap_or(false),
            "LATCH: status must be the CRITICAL residue latch; got: {:?}",
            app.status
        );
        assert_eq!(count_allocations(&app), 0, "LATCH: nothing appended");
    }

    // ── KAT-ALLOCATE-STATUS-TIMEBARRED (D6 arm 3) ────────────────────────────
    //
    // derive_allocate_status keyed to a timebarred allocation id → arm 3. Uses the ProRata
    // unattested seed (timebarred UNCONDITIONALLY, independent of the wall-clock date), so the arm
    // is exercised deterministically.
    #[test]
    fn kat_allocate_status_timebarred() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-alloc-status-tb";
        let alloc_id = seed_safe_harbor_vault(&vault, &key, pp_str);

        let app = open_app(&vault, pp_str);
        let snap = app.snapshot.as_ref().unwrap();

        // Sanity: the seed allocation is timebarred (inert), not effective.
        assert!(
            snap.state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::SafeHarborTimebar
                    && b.event.as_ref() == Some(&alloc_id)),
            "STATUS-TB: the seed allocation must fire SafeHarborTimebar on its id"
        );

        let status = derive_allocate_status(snap, &alloc_id);
        assert_eq!(
            status,
            "Allocation created (REVOCABLE, timebarred) — attest with 'a' to make it effective, \
             or void with 'v'.",
            "STATUS-TB: a timebarred fresh allocation must yield the arm-3 status"
        );
    }

    // ── KAT-E2E-ALLOCATE-THEN-ATTEST (Task 3) ────────────────────────────────
    //
    // A (create REVOCABLE allocation) → a (attest) → EFFECTIVE (Path B). At the current date the
    // created allocation is timebarred (G2: now > TY2025_RETURN_DUE 2026-04-15), so the attest flow
    // opens and cures it; NO SafeHarborTimebar remains on the attested id.
    #[test]
    fn kat_e2e_allocate_then_attest() {
        use btctax_core::event::{EventPayload, SafeHarborAllocation};
        use btctax_core::persistence::load_all_ordered;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-alloc-attest";
        // Pre-2025 lot (0.20 BTC) + attested FIFO → residue non-empty, config gate open.
        seed_allocate_vault(&vault, &key, pp_str, true, true, None);

        let mut app = open_app(&vault, pp_str);

        // A → Preview → Enter → modal → Enter → create.
        handle_key(&mut app, press(KeyCode::Char('A')));
        assert!(
            matches!(
                app.safe_harbor_allocate_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAllocateStep::Preview)
            ),
            "ALLOC-ATTEST: flow must open at Preview"
        );
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.safe_harbor_allocate_modal.is_some(),
            "ALLOC-ATTEST: Enter must open the confirm modal"
        );
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.safe_harbor_allocate_flow.is_none() && app.safe_harbor_allocate_modal.is_none(),
            "ALLOC-ATTEST: a clean create must close the flow AND modal"
        );

        // The created allocation is REVOCABLE + timebarred (arm 3) at the current date.
        let alloc_id = {
            let session = app.session.as_ref().unwrap();
            let events = load_all_ordered(session.conn()).unwrap();
            let last = events.last().unwrap();
            let payload: EventPayload = serde_json::from_str(&last.payload_json).unwrap();
            assert!(
                matches!(&payload, EventPayload::SafeHarborAllocation(a) if !a.timely_allocation_attested),
                "ALLOC-ATTEST: the tail must be an unattested SafeHarborAllocation"
            );
            EventId::Decision {
                seq: last.decision_seq.unwrap() as u64,
            }
        };
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("REVOCABLE, timebarred"),
            "ALLOC-ATTEST: create status must be arm 3 (timebarred); got: {:?}",
            app.status
        );

        // a → attest the created allocation.
        handle_key(&mut app, press(KeyCode::Char('a')));
        assert!(
            matches!(
                app.safe_harbor_attest_flow.as_ref().map(|f| &f.step),
                Some(SafeHarborAttestStep::Info)
            ),
            "ALLOC-ATTEST: attest flow must open at Info (the created allocation is timebarred)"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // → TypedWord
        type_str(&mut app, "ATTEST");
        handle_key(&mut app, press(KeyCode::Enter)); // → attest save

        assert!(
            app.safe_harbor_attest_flow.is_none() && !app.attest_save_failed,
            "ALLOC-ATTEST: attest must succeed (flow closed, no latch)"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Allocation attested (IRREVOCABLE, §7.4)"))
                .unwrap_or(false),
            "ALLOC-ATTEST: status must be the clean attest arm; got: {:?}",
            app.status
        );

        // The attested allocation is EFFECTIVE: no SafeHarborTimebar on the NEW attested id +
        // Path-B seed lots installed.
        let post = {
            let session = app.session.as_ref().unwrap();
            load_all_ordered(session.conn()).unwrap()
        };
        let attest_id = EventId::Decision {
            seq: post.last().unwrap().decision_seq.unwrap() as u64,
        };
        let last: EventPayload = serde_json::from_str(&post.last().unwrap().payload_json).unwrap();
        assert!(
            matches!(&last, EventPayload::SafeHarborAllocation(SafeHarborAllocation { timely_allocation_attested, .. }) if *timely_allocation_attested),
            "ALLOC-ATTEST: the tail must be the attested allocation"
        );
        // Effective = neither the time-bar NOR the conservation blocker fires on the attested id.
        let snap = app.snapshot.as_ref().unwrap();
        let on_attest = |k: BlockerKind| {
            snap.state
                .blockers
                .iter()
                .any(|b| b.kind == k && b.event.as_ref() == Some(&attest_id))
        };
        assert!(
            !on_attest(BlockerKind::SafeHarborTimebar),
            "ALLOC-ATTEST: NO SafeHarborTimebar may be attributed to the attested id (effective)"
        );
        assert!(
            !on_attest(BlockerKind::SafeHarborUnconservable),
            "ALLOC-ATTEST: the residue must conserve (no SafeHarborUnconservable on the attested id)"
        );
        // The pre-attest allocation was voided by the attest batch (no stray live unattested residue).
        let _ = alloc_id;
    }

    // ── KAT-E2E-ALLOCATE-THEN-VOID (Task 3) ──────────────────────────────────
    //
    // A (create REVOCABLE allocation) → v (void). The created allocation is inert (timebarred) at the
    // current date, so #7 keeps it in the void list; voiding it applies CLEANLY (no DecisionConflict).
    #[test]
    fn kat_e2e_allocate_then_void() {
        use btctax_core::event::{EventPayload, VoidDecisionEvent};
        use btctax_core::persistence::load_all_ordered;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-alloc-void";
        seed_allocate_vault(&vault, &key, pp_str, true, true, None);

        let mut app = open_app(&vault, pp_str);

        // A → Preview → Enter → modal → Enter → create.
        handle_key(&mut app, press(KeyCode::Char('A')));
        handle_key(&mut app, press(KeyCode::Enter));
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.safe_harbor_allocate_flow.is_none(),
            "ALLOC-VOID: create must close the flow"
        );

        let alloc_id = {
            let session = app.session.as_ref().unwrap();
            let events = load_all_ordered(session.conn()).unwrap();
            EventId::Decision {
                seq: events.last().unwrap().decision_seq.unwrap() as u64,
            }
        };

        // v → the inert allocation is listed (#7 keeps inert allocations voidable).
        handle_key(&mut app, press(KeyCode::Char('v')));
        {
            let void_flow = app
                .void_flow
                .as_ref()
                .expect("ALLOC-VOID: void flow must open");
            assert!(
                void_flow
                    .list
                    .items
                    .iter()
                    .any(|i| i.event_id == alloc_id && i.payload_tag == "SafeHarborAllocation"),
                "ALLOC-VOID: the inert created allocation must be listed by the void flow (#7)"
            );
            let idx = void_flow
                .list
                .items
                .iter()
                .position(|i| i.event_id == alloc_id)
                .unwrap();
            app.void_flow
                .as_mut()
                .unwrap()
                .list
                .table_state
                .select(Some(idx));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → void modal
        assert!(
            app.void_modal.is_some(),
            "ALLOC-VOID: Enter must open the void modal"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // → void save

        assert!(
            app.void_flow.is_none() && app.void_modal.is_none(),
            "ALLOC-VOID: a clean void must close the flow AND modal"
        );

        // Voided CLEANLY: NO DecisionConflict.
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !snap
                .state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::DecisionConflict),
            "ALLOC-VOID: voiding an inert allocation must NOT raise DecisionConflict; blockers: {:?}",
            snap.state.blockers
        );

        // A VoidDecisionEvent targets the created allocation.
        let events = {
            let session = app.session.as_ref().unwrap();
            load_all_ordered(session.conn()).unwrap()
        };
        assert!(
            events.iter().any(|e| matches!(
                serde_json::from_str::<EventPayload>(&e.payload_json).unwrap(),
                EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id })
                    if target_event_id == alloc_id
            )),
            "ALLOC-VOID: a VoidDecisionEvent must target the created allocation"
        );
    }

    // ── KAT-E2E-ATTEST-WRONGWORD — wrong word: error shown, buf preserved ─────

    #[test]
    fn kat_e2e_attest_wrong_word_preserves_buf() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-wrongword";

        seed_safe_harbor_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        handle_key(&mut app, press(KeyCode::Char('a')));
        handle_key(&mut app, press(KeyCode::Enter)); // → TypedWord

        // Type wrong word.
        type_str(&mut app, "attest");
        handle_key(&mut app, press(KeyCode::Enter));

        // Flow must still be open, error must be set, buf must be preserved.
        assert!(
            app.safe_harbor_attest_flow.is_some(),
            "WRONGWORD: flow must stay open on wrong word"
        );
        match app.safe_harbor_attest_flow.as_ref().map(|f| &f.step) {
            Some(SafeHarborAttestStep::TypedWord { buf, error }) => {
                assert_eq!(
                    buf.buf.as_str(),
                    "attest",
                    "WRONGWORD: buf must be preserved; got: {:?}",
                    buf.buf
                );
                assert_eq!(
                    error.as_deref(),
                    Some("type ATTEST (all caps) to confirm"),
                    "WRONGWORD: case-sensitivity error must match spec text exactly"
                );
            }
            other => panic!("WRONGWORD: expected TypedWord step; got {other:?}"),
        }

        // Correct word clears error and saves.
        // Need to clear buf first (backspace x6).
        for _ in 0..6 {
            handle_key(&mut app, press(KeyCode::Backspace));
        }
        type_str(&mut app, "ATTEST");
        handle_key(&mut app, press(KeyCode::Enter));

        assert!(
            app.safe_harbor_attest_flow.is_none(),
            "WRONGWORD: flow must close after correct word"
        );
        assert!(
            !app.attest_save_failed,
            "WRONGWORD: latch must not be set after correct attest"
        );
    }

    // ── KAT-E2E-ATTEST-VOID — after attest the void list is EMPTY (#7 pre-filter) ──
    //
    // REWRITTEN for #7 (INTENTIONAL SUPERSESSION — flag for the whole-diff reviewer):
    // the prior version pinned the §7.4 doomed-void TRAP (asserted the attested alloc IS
    // listed, and that voiding it yields "remains in force"). After #7's effective-allocation
    // pre-filter that path is TUI-UNREACHABLE: post-attest the newly-attested allocation is
    // EFFECTIVE (excluded by the pre-filter) and the prior is already voided (excluded by the
    // voided-set scan) → the void list is EMPTY. So the flow does NOT open and the status is
    // the empty-list message. Engine coverage of the §7.4 irrevocability guard is NOT lost —
    // it is still pinned by crates/btctax-core/tests/transition.rs:365.

    #[test]
    fn kat_e2e_attest_void_list_empty_after_attest() {
        use btctax_core::persistence::load_all_ordered;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-attest-void";

        // The prior (unattested ProRata) allocation — voided by the attest below.
        let _prior_id = seed_safe_harbor_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // Attest first.
        handle_key(&mut app, press(KeyCode::Char('a')));
        handle_key(&mut app, press(KeyCode::Enter)); // Info → TypedWord
        type_str(&mut app, "ATTEST");
        handle_key(&mut app, press(KeyCode::Enter)); // save
        assert!(
            app.safe_harbor_attest_flow.is_none(),
            "VOID-EMPTY: flow must be closed after attest"
        );

        // The NEW attested allocation's id = tail decision row.
        let new_attest_id = {
            let session = app.session.as_ref().unwrap();
            let rows = load_all_ordered(session.conn()).unwrap();
            EventId::Decision {
                seq: rows.last().unwrap().decision_seq.unwrap() as u64,
            }
        };

        // Precondition sanity: post-attest the new allocation is EFFECTIVE (no timebar /
        // unconservable on its id) and the prior is voided — so both are excluded from the
        // void list (effective-alloc pre-filter + voided-set scan respectively).
        {
            let snap = app.snapshot.as_ref().unwrap();
            let on_new = |k: BlockerKind| {
                snap.state
                    .blockers
                    .iter()
                    .any(|b| b.kind == k && b.event.as_ref() == Some(&new_attest_id))
            };
            assert!(
                !on_new(BlockerKind::SafeHarborTimebar)
                    && !on_new(BlockerKind::SafeHarborUnconservable),
                "VOID-EMPTY: the newly attested allocation must be effective"
            );
        }

        // Now open void flow: 'v' → the list is EMPTY → flow does NOT open + empty-list status.
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('v')));
        assert!(
            app.void_flow.is_none(),
            "VOID-EMPTY: void flow must NOT open (effective alloc excluded, prior voided)"
        );
        assert_eq!(
            app.status.as_deref(),
            Some("No revocable decisions to void"),
            "VOID-EMPTY: status must be the empty-list message"
        );
    }

    // ── KAT-VOID-EFFECTIVE-PREFILTER-MIXED (#7) — effective alloc excluded, other listed ──
    //
    // Seed an EFFECTIVE SafeHarborAllocation (empty → conservable vs the empty pre-2025
    // Universal snapshot; attested → not timebarred) PLUS one other revocable decision
    // (a MethodElection) so the flow opens. Assert the other decision IS listed and the
    // effective allocation is NOT.

    #[test]
    fn kat_void_effective_prefilter_mixed() {
        use btctax_core::event::{AllocMethod, EventPayload, MethodElection, SafeHarborAllocation};
        use btctax_core::persistence::append_decision;
        use btctax_core::LotMethod;
        use time::{macros::date, OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-mixed-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        let (alloc_id, me_id) = {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            // Decision 1: an EFFECTIVE allocation.
            let alloc_id = append_decision(
                session.conn(),
                EventPayload::SafeHarborAllocation(SafeHarborAllocation {
                    lots: vec![],
                    as_of_date: date!(2025 - 01 - 01),
                    method: AllocMethod::ActualPosition,
                    timely_allocation_attested: true,
                    pre2025_method: LotMethod::Fifo,
                }),
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            // Decision 2: another revocable decision so the flow opens.
            let me_id = append_decision(
                session.conn(),
                EventPayload::MethodElection(MethodElection {
                    effective_from: date!(2024 - 01 - 01),
                    method: LotMethod::Fifo,
                }),
                OffsetDateTime::from_unix_timestamp(1_748_001_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
            (alloc_id, me_id)
        };

        let mut app = open_app(&vault, pp_str);

        // Sanity: the allocation is effective (no timebar / unconservable on its id).
        {
            let snap = app.snapshot.as_ref().unwrap();
            let on_alloc = |k: BlockerKind| {
                snap.state
                    .blockers
                    .iter()
                    .any(|b| b.kind == k && b.event.as_ref() == Some(&alloc_id))
            };
            assert!(
                !on_alloc(BlockerKind::SafeHarborTimebar)
                    && !on_alloc(BlockerKind::SafeHarborUnconservable),
                "VOID-MIXED: the allocation must be effective (Path B)"
            );
        }

        // v → flow opens (the MethodElection keeps the list non-empty).
        handle_key(&mut app, press(KeyCode::Char('v')));
        let void_flow = app
            .void_flow
            .as_ref()
            .expect("VOID-MIXED: flow must open (other revocable decision present)");
        assert!(
            void_flow.list.items.iter().any(|i| i.event_id == me_id),
            "VOID-MIXED: the MethodElection must be listed"
        );
        assert!(
            !void_flow.list.items.iter().any(|i| i.event_id == alloc_id),
            "VOID-MIXED: the effective allocation must NOT be listed"
        );
    }

    // ── KAT-VOID-INERT-ALLOC-LISTED (#7) — a timebarred alloc REMAINS voidable ──
    //
    // An unattested past-bar ProRata allocation is timebarred (inert) → voiding it applies
    // cleanly (transition.rs:403) → it must REMAIN in the void list (the pre-filter excludes
    // ONLY effective allocations).

    #[test]
    fn kat_void_inert_alloc_listed() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-void-inert-pass";

        // Unattested ProRata empty allocation → timebarred (inert).
        let alloc_id = seed_safe_harbor_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // Sanity: the allocation IS timebarred (inert).
        {
            let snap = app.snapshot.as_ref().unwrap();
            assert!(
                snap.state.blockers.iter().any(|b| {
                    b.kind == BlockerKind::SafeHarborTimebar && b.event.as_ref() == Some(&alloc_id)
                }),
                "VOID-INERT: the allocation must be timebarred (inert)"
            );
        }

        handle_key(&mut app, press(KeyCode::Char('v')));
        let void_flow = app
            .void_flow
            .as_ref()
            .expect("VOID-INERT: flow must open (an inert allocation is voidable)");
        assert!(
            void_flow
                .list
                .items
                .iter()
                .any(|i| i.event_id == alloc_id && i.payload_tag == "SafeHarborAllocation"),
            "VOID-INERT: the timebarred allocation must REMAIN in the void list"
        );

        // [WB-M1] The inert-allocation void path is now the ONLY reachable place to E2E-assert the
        // void modal's is_safe_harbor flag (the ATTEST-VOID rewrite removed the effective-alloc path).
        // Open the modal on the (sole, cursor-0) allocation and assert the Path-B warning flag is set.
        handle_key(&mut app, press(KeyCode::Enter));
        let modal = app
            .void_modal
            .as_ref()
            .expect("VOID-INERT: Enter must open the void modal for the inert allocation");
        assert!(
            modal.is_safe_harbor,
            "VOID-INERT: a SafeHarborAllocation void modal must set is_safe_harbor (Path-B warning)"
        );
    }

    // ── KAT-E2E-ATTEST-ERRLATCH — save error sets latch, blocks all openers ──

    #[cfg(unix)]
    #[test]
    fn kat_e2e_attest_errlatch_chmod() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-errlatch";

        seed_safe_harbor_vault(&vault, &key, pp_str);

        // Root-skip guard.
        {
            let probe = dir.path().join("probe.tmp");
            let perms = std::fs::Permissions::from_mode(0o500);
            std::fs::set_permissions(dir.path(), perms).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("KAT-E2E-ERRLATCH: skipping — chmod 0o500 did not deny writes (root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();

        let mut app = open_app(&vault, pp_str);

        // 'a' → Info → TypedWord → "ATTEST".
        handle_key(&mut app, press(KeyCode::Char('a')));
        handle_key(&mut app, press(KeyCode::Enter)); // Info → TypedWord
        type_str(&mut app, "ATTEST");

        // Make vault's parent dir read-only before confirming.
        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();

        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save fails

        // Restore BEFORE any assertions that might panic.
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        // [R0-C1] Latch must be set.
        assert!(
            app.attest_save_failed,
            "ERRLATCH: attest_save_failed must be true after save error"
        );

        // Flow must be closed.
        assert!(
            app.safe_harbor_attest_flow.is_none(),
            "ERRLATCH: flow must be closed after save error"
        );

        // Status is the quit-first remedy (spec D3 Err arm).
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error")
                    && s.contains(
                        "quit the editor now (the unsaved attestation is discarded on quit)"
                    )
                    && s.contains("btctax reconcile safe-harbor-attest"))
                .unwrap_or(false),
            "ERRLATCH: status must be the quit-first Err remedy; got: {:?}",
            app.status
        );

        // Vault bytes must be unchanged.
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "ERRLATCH: vault bytes must be unchanged after failed save"
        );

        // While the latch is set, EVERY mutating opener (p/c/o/r/f/v/s/d/a) must refuse
        // with the latch status — this is the piggy-back guard: with every mutating opener
        // latched shut, no later session.save() can flush the in-memory Void+Attest residue.
        // [round-1 whole-branch review TF-M1: cover all 9 openers, not just a/f/p, so a
        // future opener added without the guard is caught here.]
        for k in ['p', 'c', 'o', 'r', 'f', 'v', 's', 'd', 'a'] {
            app.status = None;
            handle_key(&mut app, press(KeyCode::Char(k)));
            assert!(
                app.profile_form.is_none()
                    && app.classify_inbound_flow.is_none()
                    && app.reclassify_outflow_flow.is_none()
                    && app.reclassify_income_flow.is_none()
                    && app.set_fmv_flow.is_none()
                    && app.void_flow.is_none()
                    && app.select_lots_flow.is_none()
                    && app.set_donation_details_flow.is_none()
                    && app.safe_harbor_attest_flow.is_none(),
                "ERRLATCH: opener '{k}' must open no mutating flow while the latch is set"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("failed attest save"))
                    .unwrap_or(false),
                "ERRLATCH: opener '{k}' must show the latch status; got: {:?}",
                app.status
            );
        }

        // Verify vault disk bytes still equal bytes_before (session holds lock; read file directly).
        let bytes_after2 = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after2,
            "ERRLATCH: vault bytes must remain unchanged after failed save"
        );

        // Defense-in-depth [R0-I5]: even BYPASSING the latch, the session-sourced
        // pre-flight sees the in-memory already-attested residue and refuses via the
        // "already attested" arm, appending NOTHING.
        {
            use btctax_core::persistence::load_all_ordered;
            let len_before = {
                let session = app.session.as_ref().unwrap();
                load_all_ordered(session.conn()).unwrap().len()
            };
            app.attest_save_failed = false; // bypass the latch deliberately
            app.status = None;
            open_safe_harbor_attest_flow(&mut app);
            assert!(
                app.safe_harbor_attest_flow.is_none(),
                "ERRLATCH defense-in-depth: flow must NOT open on in-memory residue"
            );
            assert!(
                app.status
                    .as_deref()
                    .map(|s| s.contains("Allocation already attested"))
                    .unwrap_or(false),
                "ERRLATCH defense-in-depth: pre-flight must refuse via the \
                 'already attested' arm; got: {:?}",
                app.status
            );
            let len_after = {
                let session = app.session.as_ref().unwrap();
                load_all_ordered(session.conn()).unwrap().len()
            };
            assert_eq!(
                len_before, len_after,
                "ERRLATCH defense-in-depth: the refused pre-flight must append NOTHING"
            );
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // Task 3 (select-lots cluster: #1 SelfTransfer, #2 pre-2025 gate, #3 uncovered)
    // ══════════════════════════════════════════════════════════════════════════
    //
    // Epoch anchors (UTC): 2024-06-01 = 1_717_200_000, 2024-07-01 = 1_719_792_000,
    // 2024-08-01 = 1_722_470_400 (all pre-2025 → PoolKey::Universal); 2025-02-01 =
    // 1_738_368_000, 2025-03-01 = 1_740_787_200 (post-2025 → PoolKey::Wallet). The
    // 2025-01-01 boundary is TRANSITION_DATE (= 1_735_689_600).

    fn river_wallet() -> btctax_core::WalletId {
        btctax_core::WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        }
    }
    fn kraken_wallet() -> btctax_core::WalletId {
        btctax_core::WalletId::Exchange {
            provider: "Kraken".to_string(),
            account: "main".to_string(),
        }
    }

    /// #1: Acquire 1M (River, 2025-02) + TransferOut 500K (River, 2025-03) + a non-voided
    /// TransferLink(out=to, dest=Wallet(Kraken)) → a covered SelfTransfer of 500K.
    /// Returns (to_id, transfer_link_decision_id).
    fn seed_self_transfer_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> (btctax_core::EventId, btctax_core::EventId) {
        use btctax_core::event::{Acquire, EventPayload, LedgerEvent, TransferLink, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let acq_id = EventId::import(Source::River, SourceRef::new("st-acq-1"));
        let to_id = EventId::import(Source::River, SourceRef::new("st-to-1"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let batch = vec![
            LedgerEvent {
                id: acq_id,
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_368_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(river_wallet()),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(50000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: to_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_787_200).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(river_wallet()),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 500_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
        ];
        append_import_batch(session.conn(), &batch).unwrap();
        let tl_id = append_decision(
            session.conn(),
            EventPayload::TransferLink(TransferLink {
                out_event: to_id.clone(),
                in_event_or_wallet: TransferTarget::Wallet(kraken_wallet()),
            }),
            OffsetDateTime::from_unix_timestamp(1_740_800_000).unwrap(),
            UtcOffset::UTC,
            None,
        )
        .unwrap();
        session.save().unwrap();
        (to_id, tl_id)
    }

    // ── KAT-SELFTRANSFER-SELECTABLE (#1) ─────────────────────────────────────
    #[test]
    fn kat_selftransfer_selectable() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-st-sel-pass";

        let (to_id, _tl_id) = seed_self_transfer_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // s → the SelfTransfer row is listed with principal = TransferOut.sat + source wallet.
        handle_key(&mut app, press(KeyCode::Char('s')));
        {
            let flow = app
                .select_lots_flow
                .as_ref()
                .expect("ST-SEL: select-lots flow must open");
            let item = flow
                .list
                .items
                .iter()
                .find(|i| i.disposal_event == to_id)
                .expect("ST-SEL: the self-transfer must be listed");
            assert_eq!(
                item.kind,
                DisposalKind::SelfTransfer,
                "ST-SEL: kind must be SelfTransfer"
            );
            assert_eq!(
                item.principal_sat, 500_000,
                "ST-SEL: principal_sat must equal TransferOut.sat (fee excluded)"
            );
            assert_eq!(
                item.wallet.as_ref(),
                Some(&river_wallet()),
                "ST-SEL: wallet must be the SOURCE wallet"
            );
        }

        // Enter → LotsForm (post-2025 date → per-wallet; source wallet has the 500K remainder).
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.select_lots_flow.as_ref().map(|f| &f.step),
                Some(SelectLotsStep::LotsForm { .. })
            ),
            "ST-SEL: LotsForm must open (source wallet has a lot)"
        );

        // Pick the whole 500K remainder → conserves principal → clean save (arm 3).
        for c in "500000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.select_lots_modal.is_some(), "ST-SEL: modal must open");
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save
        assert!(
            app.select_lots_flow.is_none(),
            "ST-SEL: a clean save must close the flow"
        );
        let status = app.status.as_deref().unwrap_or("");
        assert!(
            status.contains("Lot selection recorded"),
            "ST-SEL: clean save must yield the arm-3 status; got: {status}"
        );
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !snap.state.blockers.iter().any(|b| matches!(
                b.kind,
                BlockerKind::DecisionConflict | BlockerKind::LotSelectionInvalid
            )),
            "ST-SEL: the selection must apply cleanly; blockers: {:?}",
            snap.state.blockers
        );
    }

    // ── KAT-SELFTRANSFER-VOIDED-LINK-ABSENT (#1) ─────────────────────────────
    #[test]
    fn kat_selftransfer_voided_link_absent() {
        use btctax_core::event::{EventPayload, VoidDecisionEvent};
        use btctax_core::persistence::append_decision;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-st-void-pass";

        let (_to_id, tl_id) = seed_self_transfer_vault(&vault, &key, pp_str);

        // Void the TransferLink → the TransferOut is no longer a self-transfer.
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            append_decision(
                session.conn(),
                EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                    target_event_id: tl_id,
                }),
                OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert!(
            app.select_lots_flow.is_none(),
            "ST-VOID: a voided TransferLink → no self-transfer → empty list → flow must NOT open"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("No method-honoring disposals available"),
            "ST-VOID: status must be the empty-list message; got: {:?}",
            app.status
        );
    }

    /// #2/#3 pre-2025 seed builder. Adds Acquire lots (each `(wallet, ts, sat, usd_cost)`), a
    /// pre-2025 TransferOut(500K, River, 2024-08) reclassified to Sell, and — when `trigger_2025`
    /// — a post-2025 Acquire (River, 2025-02, `trigger_2025` sat, $trigger/20) to fire the §7.4
    /// boundary seed. Optionally appends an effective SafeHarborAllocation (`alloc`). Returns
    /// (to_id, Option<alloc_id>).
    #[allow(clippy::type_complexity)]
    fn seed_pre2025_disposal_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
        acquires: &[(btctax_core::WalletId, i64, i64, i64)],
        trigger_2025: Option<(i64, i64)>,
        alloc: Option<btctax_core::event::SafeHarborAllocation>,
    ) -> (btctax_core::EventId, Option<btctax_core::EventId>) {
        use btctax_core::event::{
            Acquire, DisposeKind, EventPayload, LedgerEvent, OutflowClass, ReclassifyOutflow,
            TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use rust_decimal::Decimal;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let to_id = EventId::import(Source::River, SourceRef::new("p25-to-1"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();

        let mut batch: Vec<LedgerEvent> = Vec::new();
        for (i, (wallet, ts, sat, usd_cost)) in acquires.iter().enumerate() {
            batch.push(LedgerEvent {
                id: EventId::import(Source::River, SourceRef::new(format!("p25-acq-{i}"))),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(*ts).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(wallet.clone()),
                payload: EventPayload::Acquire(Acquire {
                    sat: *sat,
                    usd_cost: Decimal::from(*usd_cost),
                    fee_usd: Decimal::ZERO,
                    basis_source: BasisSource::ExchangeProvided,
                }),
            });
        }
        // Pre-2025 TransferOut (2024-08) in River.
        batch.push(LedgerEvent {
            id: to_id.clone(),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_722_470_400).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(river_wallet()),
            payload: EventPayload::TransferOut(TransferOut {
                sat: 500_000,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            }),
        });
        if let Some((sat, usd_cost)) = trigger_2025 {
            batch.push(LedgerEvent {
                id: EventId::import(Source::River, SourceRef::new("p25-acq-2025")),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_368_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(river_wallet()),
                payload: EventPayload::Acquire(Acquire {
                    sat,
                    usd_cost: Decimal::from(usd_cost),
                    fee_usd: Decimal::ZERO,
                    basis_source: BasisSource::ExchangeProvided,
                }),
            });
        }
        append_import_batch(session.conn(), &batch).unwrap();

        // Reclassify the TransferOut → Sell (a Dispose).
        append_decision(
            session.conn(),
            EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: to_id.clone(),
                as_: OutflowClass::Dispose {
                    kind: DisposeKind::Sell,
                },
                principal_proceeds_or_fmv: Decimal::from(20000),
                fee_usd: None,
                donee: None,
            }),
            OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap(),
            UtcOffset::UTC,
            None,
        )
        .unwrap();

        let alloc_id = alloc.map(|a| {
            append_decision(
                session.conn(),
                EventPayload::SafeHarborAllocation(a),
                OffsetDateTime::from_unix_timestamp(1_741_100_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap()
        });
        session.save().unwrap();
        (to_id, alloc_id)
    }

    // ── KAT-PRE2025-CROSSWALLET-LOTS (#2) ────────────────────────────────────
    #[test]
    fn kat_pre2025_crosswallet_lots() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p25-xw-pass";

        // Pre-2025 lots in River (2024-06) and Kraken (2024-07); pre-2025 disposal in River;
        // a post-2025 Acquire triggers the Path-A boundary seed (lots → ReconstructedPerWallet).
        let acq_river = EventId::import(
            btctax_core::identity::Source::River,
            btctax_core::identity::SourceRef::new("p25-acq-0"),
        );
        let acq_kraken = EventId::import(
            btctax_core::identity::Source::River,
            btctax_core::identity::SourceRef::new("p25-acq-1"),
        );
        let acq_2025 = EventId::import(
            btctax_core::identity::Source::River,
            btctax_core::identity::SourceRef::new("p25-acq-2025"),
        );
        let (to_id, _) = seed_pre2025_disposal_vault(
            &vault,
            &key,
            pp_str,
            &[
                (river_wallet(), 1_717_200_000, 1_000_000, 30000),
                (kraken_wallet(), 1_719_792_000, 1_000_000, 40000),
            ],
            Some((200_000, 10000)),
            None,
        );

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('s')));
        {
            let flow = app
                .select_lots_flow
                .as_ref()
                .expect("XW: select-lots flow must open");
            let idx = flow
                .list
                .items
                .iter()
                .position(|i| i.disposal_event == to_id)
                .expect("XW: the pre-2025 disposal must be listed");
            app.select_lots_flow
                .as_mut()
                .unwrap()
                .list
                .table_state
                .select(Some(idx));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → LotsForm
        {
            let flow = app.select_lots_flow.as_ref().unwrap();
            let origins: Vec<btctax_core::EventId> = match &flow.step {
                SelectLotsStep::LotsForm { rows, .. } => rows
                    .iter()
                    .map(|r| r.lot_id.origin_event_id.clone())
                    .collect(),
                _ => panic!("XW: LotsForm must open"),
            };
            assert!(
                origins.contains(&acq_river),
                "XW: the River pre-2025 lot must be offered"
            );
            assert!(
                origins.contains(&acq_kraken),
                "XW: the Kraken pre-2025 lot must be offered CROSS-WALLET (pre-2025 Universal)"
            );
            assert!(
                !origins.contains(&acq_2025),
                "XW: the post-2025 lot must NOT be offered for a pre-2025 disposal"
            );
        }

        // Pick the Kraken lot (rows sorted acquired_at ASC → River=0, Kraken=1) for 500K → clean.
        handle_key(&mut app, press(KeyCode::Down)); // cursor → 1 (Kraken)
        for c in "500000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.select_lots_modal.is_some(), "XW: modal must open");
        handle_key(&mut app, press(KeyCode::Enter)); // save
        assert!(
            app.select_lots_flow.is_none(),
            "XW: a clean cross-wallet selection must close the flow"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("Lot selection recorded"),
            "XW: clean arm-3 status expected; got: {:?}",
            app.status
        );
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !snap.state.blockers.iter().any(|b| matches!(
                b.kind,
                BlockerKind::DecisionConflict | BlockerKind::LotSelectionInvalid
            )),
            "XW: the engine must accept the cross-wallet pre-2025 pick; blockers: {:?}",
            snap.state.blockers
        );
    }

    // ── KAT-PRE2025-PATHB-SEEDLOTS-EXCLUDED (#2) ─────────────────────────────
    #[test]
    fn kat_pre2025_pathb_seedlots_excluded() {
        use btctax_core::event::{AllocLot, AllocMethod, SafeHarborAllocation};
        use btctax_core::LotMethod;
        use rust_decimal::Decimal;
        use time::macros::date;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p25-pathb-pass";

        // Pre-2025 residue after the 500K disposal = 500K sat / $15000 basis (from 1M / $30000).
        // The attested allocation LISTS exactly that residue → effective → Path B installs
        // SafeHarborAllocated seed lots (acquired 2024-06, < TRANSITION_DATE).
        let alloc = SafeHarborAllocation {
            lots: vec![AllocLot {
                wallet: river_wallet(),
                sat: 500_000,
                usd_basis: Decimal::from(15000),
                acquired_at: date!(2024 - 06 - 01),
                dual_loss_basis: None,
                donor_acquired_at: None,
            }],
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ActualPosition,
            timely_allocation_attested: true,
            pre2025_method: LotMethod::Fifo,
        };
        let (to_id, alloc_id) = seed_pre2025_disposal_vault(
            &vault,
            &key,
            pp_str,
            &[(river_wallet(), 1_717_200_000, 1_000_000, 30000)],
            Some((100_000, 5000)),
            Some(alloc),
        );
        let alloc_id = alloc_id.unwrap();

        let mut app = open_app(&vault, pp_str);

        // Sanity: the allocation is EFFECTIVE (Path B) and the pool carries SafeHarborAllocated seeds.
        {
            let snap = app.snapshot.as_ref().unwrap();
            let on_alloc = |k: BlockerKind| {
                snap.state
                    .blockers
                    .iter()
                    .any(|b| b.kind == k && b.event.as_ref() == Some(&alloc_id))
            };
            assert!(
                !on_alloc(BlockerKind::SafeHarborTimebar)
                    && !on_alloc(BlockerKind::SafeHarborUnconservable),
                "PATHB: the allocation must be effective; blockers: {:?}",
                snap.state.blockers
            );
            assert!(
                snap.state
                    .lots
                    .iter()
                    .any(|l| l.basis_source == BasisSource::SafeHarborAllocated),
                "PATHB: Path-B seed lots must be present"
            );
        }

        handle_key(&mut app, press(KeyCode::Char('s')));
        {
            let flow = app
                .select_lots_flow
                .as_ref()
                .expect("PATHB: flow must open (the disposal is a candidate)");
            let idx = flow
                .list
                .items
                .iter()
                .position(|i| i.disposal_event == to_id)
                .expect("PATHB: the pre-2025 disposal must be listed");
            app.select_lots_flow
                .as_mut()
                .unwrap()
                .list
                .table_state
                .select(Some(idx));
        }
        app.status = None;
        handle_key(&mut app, press(KeyCode::Enter)); // → "No lots available" (all lots excluded)
        assert!(
            matches!(
                app.select_lots_flow.as_ref().map(|f| &f.step),
                Some(SelectLotsStep::List)
            ),
            "PATHB: the flow must stay on List (no feasible lot to offer)"
        );
        assert!(
            app.select_lots_modal.is_none(),
            "PATHB: no LotsForm/modal must open"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("No lots available"),
            "PATHB: Path-B seed lots are excluded → 'No lots available'; got: {:?}",
            app.status
        );
    }

    // ── KAT-POST2025-WALLET-SCOPED (#2 regression) ───────────────────────────
    #[test]
    fn kat_post2025_wallet_scoped() {
        use btctax_core::event::{Acquire, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::event::{DisposeKind, OutflowClass, ReclassifyOutflow};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use rust_decimal::Decimal;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-post25-ws-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();
        let acq_river = EventId::import(Source::River, SourceRef::new("post-acq-river"));
        let acq_kraken = EventId::import(Source::River, SourceRef::new("post-acq-kraken"));
        let to_id = EventId::import(Source::River, SourceRef::new("post-to"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![
                LedgerEvent {
                    id: acq_river.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_368_000).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: Some(river_wallet()),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 1_000_000,
                        usd_cost: Decimal::from(50000),
                        fee_usd: Decimal::ZERO,
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: acq_kraken.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_454_400).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: Some(kraken_wallet()),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 1_000_000,
                        usd_cost: Decimal::from(60000),
                        fee_usd: Decimal::ZERO,
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: to_id.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_787_200).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: Some(river_wallet()),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: 500_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            append_import_batch(session.conn(), &batch).unwrap();
            append_decision(
                session.conn(),
                EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                    transfer_out_event: to_id.clone(),
                    as_: OutflowClass::Dispose {
                        kind: DisposeKind::Sell,
                    },
                    principal_proceeds_or_fmv: Decimal::from(30000),
                    fee_usd: None,
                    donee: None,
                }),
                OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('s')));
        {
            let flow = app.select_lots_flow.as_ref().expect("WS: flow must open");
            let idx = flow
                .list
                .items
                .iter()
                .position(|i| i.disposal_event == to_id)
                .expect("WS: the post-2025 disposal must be listed");
            app.select_lots_flow
                .as_mut()
                .unwrap()
                .list
                .table_state
                .select(Some(idx));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → LotsForm
        {
            let flow = app.select_lots_flow.as_ref().unwrap();
            let origins: Vec<btctax_core::EventId> = match &flow.step {
                SelectLotsStep::LotsForm { rows, .. } => rows
                    .iter()
                    .map(|r| r.lot_id.origin_event_id.clone())
                    .collect(),
                _ => panic!("WS: LotsForm must open"),
            };
            assert!(
                origins.contains(&acq_river),
                "WS: the disposal's own-wallet (River) lot must be offered"
            );
            assert!(
                !origins.contains(&acq_kraken),
                "WS: a post-2025 disposal must NOT offer another wallet's (Kraken) lot"
            );
        }
    }

    // ── KAT-PRE2025-EXCLUDES-POST2025-LOT (#2) ───────────────────────────────
    #[test]
    fn kat_pre2025_excludes_post2025_lot() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-p25-excl-pass";

        let acq_river = EventId::import(
            btctax_core::identity::Source::River,
            btctax_core::identity::SourceRef::new("p25-acq-0"),
        );
        let acq_2025 = EventId::import(
            btctax_core::identity::Source::River,
            btctax_core::identity::SourceRef::new("p25-acq-2025"),
        );
        // Pre-2025 lot (River, 2024-06) + pre-2025 disposal; a post-2025 Acquire is BOTH the
        // Path-A boundary trigger AND the 2025-acquired lot that must be excluded.
        let (to_id, _) = seed_pre2025_disposal_vault(
            &vault,
            &key,
            pp_str,
            &[(river_wallet(), 1_717_200_000, 1_000_000, 30000)],
            Some((500_000, 20000)),
            None,
        );

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('s')));
        {
            let flow = app.select_lots_flow.as_ref().expect("EXCL: flow must open");
            let idx = flow
                .list
                .items
                .iter()
                .position(|i| i.disposal_event == to_id)
                .expect("EXCL: the pre-2025 disposal must be listed");
            app.select_lots_flow
                .as_mut()
                .unwrap()
                .list
                .table_state
                .select(Some(idx));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → LotsForm
        let flow = app.select_lots_flow.as_ref().unwrap();
        let origins: Vec<btctax_core::EventId> = match &flow.step {
            SelectLotsStep::LotsForm { rows, .. } => rows
                .iter()
                .map(|r| r.lot_id.origin_event_id.clone())
                .collect(),
            _ => panic!("EXCL: LotsForm must open (the pre-2025 lot is offered)"),
        };
        assert!(
            origins.contains(&acq_river),
            "EXCL: the pre-2025 lot must be offered"
        );
        assert!(
            !origins.contains(&acq_2025),
            "EXCL: the 2025-acquired lot must NOT be offered for a pre-2025 disposal"
        );
    }

    // ── KAT-UNCOVERED-EXCLUDED (#3) ──────────────────────────────────────────
    #[test]
    fn kat_uncovered_excluded() {
        use btctax_core::event::{Acquire, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::event::{DisposeKind, OutflowClass, ReclassifyOutflow};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use rust_decimal::Decimal;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-uncov-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();
        let acq_id = EventId::import(Source::River, SourceRef::new("uncov-acq"));
        let to_id = EventId::import(Source::River, SourceRef::new("uncov-to"));
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            // Acquire only 300K but dispose 500K → PARTIAL coverage: the Disposal IS recorded
            // (consumed non-empty) AND an UncoveredDisposal blocker fires on the disposal event.
            let batch = vec![
                LedgerEvent {
                    id: acq_id,
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_368_000).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: Some(river_wallet()),
                    payload: EventPayload::Acquire(Acquire {
                        sat: 300_000,
                        usd_cost: Decimal::from(10000),
                        fee_usd: Decimal::ZERO,
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                },
                LedgerEvent {
                    id: to_id.clone(),
                    utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_787_200).unwrap(),
                    original_tz: UtcOffset::UTC,
                    wallet: Some(river_wallet()),
                    payload: EventPayload::TransferOut(TransferOut {
                        sat: 500_000,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                },
            ];
            append_import_batch(session.conn(), &batch).unwrap();
            append_decision(
                session.conn(),
                EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                    transfer_out_event: to_id.clone(),
                    as_: OutflowClass::Dispose {
                        kind: DisposeKind::Sell,
                    },
                    principal_proceeds_or_fmv: Decimal::from(20000),
                    fee_usd: None,
                    donee: None,
                }),
                OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);

        // Sanity: the disposal IS recorded (partial coverage) but carries an UncoveredDisposal.
        {
            let snap = app.snapshot.as_ref().unwrap();
            assert!(
                snap.state.disposals.iter().any(|d| d.event == to_id),
                "UNCOV: the partially-covered disposal must be recorded in state.disposals"
            );
            assert!(
                snap.state.blockers.iter().any(|b| {
                    b.kind == BlockerKind::UncoveredDisposal && b.event.as_ref() == Some(&to_id)
                }),
                "UNCOV: an UncoveredDisposal blocker must fire on the disposal event"
            );
        }

        handle_key(&mut app, press(KeyCode::Char('s')));
        assert!(
            app.select_lots_flow.is_none(),
            "UNCOV: the under-covered disposal is the only candidate and is pre-filtered → \
             the flow must NOT open"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("No method-honoring disposals available"),
            "UNCOV: status must be the empty-list message; got: {:?}",
            app.status
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // chunk 4a — Task 1: link-transfer (`l`) KATs
    // ══════════════════════════════════════════════════════════════════════════

    /// Seed: Acquire(river, 1M) + TransferOut(river, 500K) + `extra` import events.
    /// Returns the pending TransferOut id.
    fn seed_link_out_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
        extra: &[btctax_core::event::LedgerEvent],
    ) -> btctax_core::EventId {
        use btctax_core::event::{Acquire, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let acq_id = EventId::import(Source::River, SourceRef::new("lt-acq-1"));
        let to_id = EventId::import(Source::River, SourceRef::new("lt-to-1"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let mut batch = vec![
            LedgerEvent {
                id: acq_id,
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_368_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(river_wallet()),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(50000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: to_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_787_200).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(river_wallet()),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 500_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
        ];
        batch.extend_from_slice(extra);
        btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
        session.save().unwrap();
        to_id
    }

    // ── KAT-E2E-LT-WALLET — link the TransferOut to a wallet → SelfTransfer ────
    #[test]
    fn kat_e2e_lt_wallet_target() {
        use btctax_core::event::{Acquire, EventPayload, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-wallet-pass";

        // Extra: a kraken Acquire (100K) so kraken is offerable in the wallet-list (union of
        // event wallets) AND carries a 100K starting holding → the relocation is observable.
        let kraken_acq = LedgerEvent {
            id: EventId::import(Source::River, SourceRef::new("lt-kraken-acq")),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_738_300_000).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(kraken_wallet()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(6000),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        };
        let to_id = seed_link_out_vault(&vault, &key, pp_str, &[kraken_acq]);

        let mut app = open_app(&vault, pp_str);
        assert!(
            app.snapshot
                .as_ref()
                .unwrap()
                .state
                .pending_reconciliation
                .iter()
                .any(|pt| pt.event == to_id),
            "LT-W: seed must produce a pending TransferOut"
        );

        // l → out-list → Enter → TargetPick(InEvent) → Tab → Wallet mode.
        handle_key(&mut app, press(KeyCode::Char('l')));
        assert!(
            app.link_transfer_flow.is_some(),
            "LT-W: flow must open on 'l'"
        );
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.link_transfer_flow.as_ref().map(|f| &f.step),
                Some(LinkTransferStep::TargetPick {
                    mode: LinkMode::InEvent,
                    ..
                })
            ),
            "LT-W: Enter must open TargetPick(InEvent)"
        );
        handle_key(&mut app, press(KeyCode::Tab));
        assert!(
            matches!(
                app.link_transfer_flow.as_ref().map(|f| &f.step),
                Some(LinkTransferStep::TargetPick {
                    mode: LinkMode::Wallet,
                    ..
                })
            ),
            "LT-W: Tab must toggle to Wallet mode"
        );
        {
            let flow = app.link_transfer_flow.as_ref().unwrap();
            // WalletId Ord: "Kraken" < "River" → kraken is index 0 (already selected).
            assert_eq!(
                flow.wallet_list.items[0].wallet,
                kraken_wallet(),
                "LT-W: kraken must be the first (Ord) offerable wallet"
            );
        }
        // Enter → modal → confirm.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.link_transfer_modal.is_some(), "LT-W: modal must open");
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.link_transfer_modal.is_none() && app.link_transfer_flow.is_none(),
            "LT-W: confirm must close modal + flow"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("Self-transfer link recorded"),
            "LT-W: success status; got: {:?}",
            app.status
        );

        // In-memory re-projection: out resolved, non-taxable, lots relocated to kraken.
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !snap
                .state
                .pending_reconciliation
                .iter()
                .any(|pt| pt.event == to_id),
            "LT-W: the TransferOut must be resolved (gone from pending_reconciliation)"
        );
        assert!(
            !snap.state.disposals.iter().any(|d| d.event == to_id),
            "LT-W: a SelfTransfer is non-taxable — no Disposal recorded"
        );
        assert_eq!(
            snap.state.holdings_by_wallet.get(&kraken_wallet()).copied(),
            Some(600_000),
            "LT-W: kraken must hold 100K + relocated 500K (Op::SelfTransfer relocation)"
        );

        // Direct SelfTransfer confirmation: the out now reconstructs as a select-lots SelfTransfer row.
        handle_key(&mut app, press(KeyCode::Char('s')));
        let sl = app
            .select_lots_flow
            .as_ref()
            .expect("LT-W: select-lots must open (the SelfTransfer is method-honoring)");
        let row = sl
            .list
            .items
            .iter()
            .find(|i| i.disposal_event == to_id)
            .expect("LT-W: the out must reconstruct as a SelfTransfer row");
        assert_eq!(
            row.kind,
            DisposalKind::SelfTransfer,
            "LT-W: the linked out projects to Op::SelfTransfer"
        );
    }

    // ── KAT-E2E-LT-INEVENT — link the TransferOut to a TransferIn event ────────
    #[test]
    fn kat_e2e_lt_in_event_target() {
        use btctax_core::event::{EventPayload, LedgerEvent, TransferIn};
        use btctax_core::identity::{Source, SourceRef};
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-inevent-pass";

        let ti_id = EventId::import(Source::River, SourceRef::new("lt-ti-1"));
        let ti = LedgerEvent {
            id: ti_id.clone(),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_800_000).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(kraken_wallet()),
            payload: EventPayload::TransferIn(TransferIn {
                sat: 500_000,
                src_addr: None,
                txid: None,
            }),
        };
        let to_id = seed_link_out_vault(&vault, &key, pp_str, &[ti]);

        let mut app = open_app(&vault, pp_str);

        // l → out-list → Enter → TargetPick(InEvent); the TransferIn is in the in-list.
        handle_key(&mut app, press(KeyCode::Char('l')));
        handle_key(&mut app, press(KeyCode::Enter));
        {
            let flow = app.link_transfer_flow.as_ref().unwrap();
            assert!(
                flow.in_list.items.iter().any(|i| i.in_event == ti_id),
                "LT-IN: the TransferIn (resolvable wallet) must be in the in-list"
            );
        }
        // Enter selects the in-event (index 0) → modal → confirm.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.link_transfer_modal.is_some(), "LT-IN: modal must open");
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.link_transfer_flow.is_none(),
            "LT-IN: confirm must close the flow"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("Self-transfer link recorded"),
            "LT-IN: success status; got: {:?}",
            app.status
        );

        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !snap
                .state
                .pending_reconciliation
                .iter()
                .any(|pt| pt.event == to_id),
            "LT-IN: the TransferOut must be resolved"
        );
        assert_eq!(
            snap.state.holdings_by_wallet.get(&kraken_wallet()).copied(),
            Some(500_000),
            "LT-IN: the 500K relocates to the in-event's wallet (kraken)"
        );
    }

    // ── KAT-C2-LT — cancel-path: q swallowed each step, Esc steps back, bytes unchanged ──
    #[test]
    fn kat_c2_lt_cancel_path_bytes_unchanged() {
        use btctax_core::event::{EventPayload, LedgerEvent, TransferIn};
        use btctax_core::identity::{Source, SourceRef};
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-cancel-pass";

        // A TransferIn so the InEvent list is non-empty (exercise both modes).
        let ti = LedgerEvent {
            id: EventId::import(Source::River, SourceRef::new("lt-c-ti")),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_800_000).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(kraken_wallet()),
            payload: EventPayload::TransferIn(TransferIn {
                sat: 500_000,
                src_addr: None,
                txid: None,
            }),
        };
        let _to_id = seed_link_out_vault(&vault, &key, pp_str, &[ti]);

        let bytes_before = std::fs::read(&vault).unwrap();
        {
            let mut app = open_app(&vault, pp_str);

            handle_key(&mut app, press(KeyCode::Char('l')));
            assert!(app.link_transfer_flow.is_some(), "C2-LT: flow opens on 'l'");

            // 'q' swallowed at OutList.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit && app.link_transfer_flow.is_some(),
                "C2-LT: 'q' swallowed at OutList"
            );

            // Enter → TargetPick.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                matches!(
                    app.link_transfer_flow.as_ref().map(|f| &f.step),
                    Some(LinkTransferStep::TargetPick { .. })
                ),
                "C2-LT: Enter opens TargetPick"
            );
            // 'q' swallowed at TargetPick.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit && app.link_transfer_flow.is_some(),
                "C2-LT: 'q' swallowed at TargetPick"
            );
            // Tab twice (InEvent→Wallet→InEvent).
            handle_key(&mut app, press(KeyCode::Tab));
            handle_key(&mut app, press(KeyCode::Tab));

            // Enter → modal.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(
                app.link_transfer_modal.is_some(),
                "C2-LT: Enter opens the modal"
            );
            // 'q' swallowed at modal.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit && app.link_transfer_modal.is_some(),
                "C2-LT: 'q' swallowed at modal"
            );
            // Esc closes modal → back to TargetPick.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.link_transfer_modal.is_none()
                    && matches!(
                        app.link_transfer_flow.as_ref().map(|f| &f.step),
                        Some(LinkTransferStep::TargetPick { .. })
                    ),
                "C2-LT: Esc closes modal → TargetPick"
            );
            // Esc → back to OutList.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.link_transfer_flow.as_ref().map(|f| &f.step),
                    Some(LinkTransferStep::OutList)
                ),
                "C2-LT: Esc steps back to OutList"
            );
            // Esc → close flow.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.link_transfer_flow.is_none(),
                "C2-LT: Esc closes the flow"
            );
        }
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2-LT: the cancel path must write NOTHING"
        );
    }

    // ── KAT-S3-LT — save-error path (chmod) → rollback, retry clean, no residue ──
    #[test]
    #[cfg(unix)]
    fn kat_s3_lt_save_error_chmod() {
        use btctax_core::persistence::load_all_ordered;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-s3-pass";

        // No extras: the only offerable wallet is river (a river→river self-transfer is a valid link).
        let _to_id = seed_link_out_vault(&vault, &key, pp_str, &[]);

        // Root-skip guard.
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("KAT-S3-LT: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();
        let pre_event_count = {
            let session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            load_all_ordered(session.conn()).unwrap().len()
        };

        let mut app = open_app(&vault, pp_str);

        // l → TargetPick → Tab (Wallet mode) → Enter (select river) → modal.
        handle_key(&mut app, press(KeyCode::Char('l')));
        handle_key(&mut app, press(KeyCode::Enter));
        handle_key(&mut app, press(KeyCode::Tab));
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.link_transfer_modal.is_some(), "S3-LT: modal must open");

        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save fails
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            app.link_transfer_modal.is_none(),
            "S3-LT: modal closes after a save failure"
        );
        assert!(
            matches!(
                app.link_transfer_flow.as_ref().map(|f| &f.step),
                Some(LinkTransferStep::TargetPick { .. })
            ),
            "S3-LT: TargetPick stays open after a save failure"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error"))
                .unwrap_or(false),
            "S3-LT: status must contain 'Save error'; got: {:?}",
            app.status
        );
        let bytes_mid = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_mid,
            "S3-LT: vault bytes unchanged after failed save"
        );
        let mid_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();
        assert_eq!(mid_len, pre_event_count, "S3-LT: rollback → no residue");

        // Retry → clean single append.
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → retry save
        assert!(
            app.link_transfer_flow.is_none(),
            "S3-LT: flow closes after a clean retry"
        );
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre_event_count + 1,
            "S3-LT: retry appends EXACTLY one TransferLink (no residue)"
        );
    }

    // ── KAT-LT-DUP — the defensive DecisionConflict status arm ────────────────
    #[test]
    fn kat_lt_duplicate_link_decision_conflict_arm() {
        use btctax_core::event::{EventPayload, TransferLink};
        use time::OffsetDateTime;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-dup-pass";

        // seed_self_transfer_vault already links to_id → kraken (a non-voided TransferLink).
        let (to_id, _tl_id) = seed_self_transfer_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);

        // The out is already linked → resolved → NOT in pending → the flow does not open.
        handle_key(&mut app, press(KeyCode::Char('l')));
        assert!(
            app.link_transfer_flow.is_none(),
            "LT-DUP: an already-linked out is pre-filtered out (empty out-list → no flow)"
        );

        // Directly persist a SECOND (duplicate) link on the same out — the defensive scenario the
        // pre-filter guards against. The engine must fire DecisionConflict on the second decision.
        let now = OffsetDateTime::now_utc();
        let payload = EventPayload::TransferLink(TransferLink {
            out_event: to_id.clone(),
            in_event_or_wallet: TransferTarget::Wallet(river_wallet()),
        });
        let decision_id = {
            let session = app.session.as_mut().unwrap();
            crate::edit::persist::persist_link_transfer(session, payload, now).unwrap()
        };
        let (snap, _) = {
            let session = app.session.as_ref().unwrap();
            btctax_tui::unlock::build_snapshot(session).unwrap()
        };
        assert!(
            snap.state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::DecisionConflict
                    && b.event.as_ref() == Some(&decision_id)),
            "LT-DUP: a duplicate out_event link must fire DecisionConflict"
        );
        let status = derive_link_transfer_status(&snap, &to_id, &decision_id, "wallet River/main");
        assert!(
            status.contains("DecisionConflict") && status.contains("void"),
            "LT-DUP: the deriver must return the clear-with-void arm; got: {status}"
        );
    }

    // ── KAT-LT-PREFILTER — in-list excludes consumed in-events; wallet-list unions events ──
    #[test]
    fn kat_lt_prefilter_in_list_excludes_consumed() {
        use btctax_core::event::{
            EventPayload, LedgerEvent, TransferIn, TransferLink, TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-pref-pass";

        // A second TransferOut B (300K) and a TransferIn ti (kraken). Then link A → InEvent(ti).
        let to_b = EventId::import(Source::River, SourceRef::new("lt-to-b"));
        let ti_id = EventId::import(Source::River, SourceRef::new("lt-pref-ti"));
        let extras = vec![
            LedgerEvent {
                id: to_b.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_790_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(river_wallet()),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 300_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
            LedgerEvent {
                id: ti_id.clone(),
                utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_800_000).unwrap(),
                original_tz: UtcOffset::UTC,
                wallet: Some(kraken_wallet()),
                payload: EventPayload::TransferIn(TransferIn {
                    sat: 500_000,
                    src_addr: None,
                    txid: None,
                }),
            },
        ];
        let to_a = seed_link_out_vault(&vault, &key, pp_str, &extras);

        // Link A → InEvent(ti) directly (consumes ti; resolves A).
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            btctax_core::persistence::append_decision(
                session.conn(),
                EventPayload::TransferLink(TransferLink {
                    out_event: to_a.clone(),
                    in_event_or_wallet: TransferTarget::InEvent(ti_id.clone()),
                }),
                OffsetDateTime::from_unix_timestamp(1_740_900_000).unwrap(),
                UtcOffset::UTC,
                None,
            )
            .unwrap();
            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('l')));
        {
            let flow = app
                .link_transfer_flow
                .as_ref()
                .expect("LT-PRE: flow must open (B pending)");
            // out-list has B (A resolved → excluded).
            assert!(
                flow.out_list
                    .items
                    .iter()
                    .any(|i| i.transfer_out_event == to_b),
                "LT-PRE: the still-pending out B must be listed"
            );
            assert!(
                !flow
                    .out_list
                    .items
                    .iter()
                    .any(|i| i.transfer_out_event == to_a),
                "LT-PRE: the already-linked out A must be excluded"
            );
            // in-list excludes ti (consumed by A's link).
            assert!(
                !flow.in_list.items.iter().any(|i| i.in_event == ti_id),
                "LT-PRE: the consumed in-event must be excluded from the in-list"
            );
            // wallet-list unions ALL event wallets (river + kraken).
            let wallets: Vec<_> = flow.wallet_list.items.iter().map(|w| &w.wallet).collect();
            assert!(
                wallets.contains(&&river_wallet()) && wallets.contains(&&kraken_wallet()),
                "LT-PRE: the wallet-list must union all event wallets; got: {wallets:?}"
            );
        }
    }

    // ── KAT-LT-WALLET-UNION — a zero-balance destination wallet is offerable [R0-I2] ──
    #[test]
    fn kat_lt_wallet_list_includes_zero_balance_wallet() {
        use btctax_core::event::{EventPayload, LedgerEvent, TransferIn};
        use btctax_core::identity::{Source, SourceRef};
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-lt-union-pass";

        // An UNCLASSIFIED TransferIn on kraken → kraken appears in events but has NO basis/holding.
        let ti = LedgerEvent {
            id: EventId::import(Source::River, SourceRef::new("lt-union-ti")),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_800_000).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(kraken_wallet()),
            payload: EventPayload::TransferIn(TransferIn {
                sat: 500_000,
                src_addr: None,
                txid: None,
            }),
        };
        let _to_id = seed_link_out_vault(&vault, &key, pp_str, &[ti]);

        let mut app = open_app(&vault, pp_str);
        // kraken has ZERO holdings (unclassified TransferIn creates no lot) …
        assert!(
            !app.snapshot
                .as_ref()
                .unwrap()
                .state
                .holdings_by_wallet
                .contains_key(&kraken_wallet()),
            "LT-UNION: precondition — kraken is a zero-balance wallet (not in holdings_by_wallet)"
        );

        handle_key(&mut app, press(KeyCode::Char('l')));
        handle_key(&mut app, press(KeyCode::Enter)); // → TargetPick
        handle_key(&mut app, press(KeyCode::Tab)); // → Wallet mode
        let flow = app.link_transfer_flow.as_ref().unwrap();
        assert!(
            flow.wallet_list
                .items
                .iter()
                .any(|w| w.wallet == kraken_wallet()),
            "LT-UNION: a zero-balance destination wallet MUST be offerable (union of events, \
             NOT holdings_by_wallet) [R0-I2]"
        );
    }

    // ══════════════════════════════════════════════════════════════════════════
    // chunk 4a — Task 2: classify-raw (`u`) KATs
    // ══════════════════════════════════════════════════════════════════════════

    /// Seed a vault with a single Unclassified import event (with a wallet) → the projection
    /// carries a `BlockerKind::Unclassified` blocker on it. Returns the target EventId.
    fn seed_unclassified_vault(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> btctax_core::EventId {
        use btctax_core::event::{EventPayload, LedgerEvent, Unclassified};
        use btctax_core::identity::{Source, SourceRef};
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let raw_id = EventId::import(Source::River, SourceRef::new("cr-raw-1"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let batch = vec![LedgerEvent {
            id: raw_id.clone(),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_787_200).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(river_wallet()),
            payload: EventPayload::Unclassified(Unclassified {
                raw: "river csv row: 0.005 BTC in, memo=?".into(),
            }),
        }];
        btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
        session.save().unwrap();
        raw_id
    }

    fn unclassified_blocker_present(snap: &btctax_tui::app::Snapshot, target: &EventId) -> bool {
        snap.state
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::Unclassified && b.event.as_ref() == Some(target))
    }

    // ── KAT-E2E-CR-ACQUIRE — classify a raw row as Acquire → blocker cleared ──
    #[test]
    fn kat_e2e_cr_acquire() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-cr-acq-pass";

        let target = seed_unclassified_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);
        assert!(
            unclassified_blocker_present(app.snapshot.as_ref().unwrap(), &target),
            "CR-ACQ: the seed must carry an Unclassified blocker"
        );

        // u → List → Enter → VariantPicker(Acquire) → Enter → AcquireForm.
        handle_key(&mut app, press(KeyCode::Char('u')));
        assert!(app.classify_raw_flow.is_some(), "CR-ACQ: flow opens on 'u'");
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.classify_raw_flow.as_ref().map(|f| &f.step),
                Some(ClassifyRawStep::VariantPicker {
                    variant: ClassifyRawVariant::Acquire,
                    ..
                })
            ),
            "CR-ACQ: Enter opens VariantPicker(Acquire)"
        );
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.classify_raw_flow.as_ref().map(|f| &f.step),
                Some(ClassifyRawStep::AcquireForm { .. })
            ),
            "CR-ACQ: Enter opens AcquireForm"
        );

        // sat (focus 0) → Down → usd_cost (focus 1). Leave fee empty; basis_source default.
        type_str(&mut app, "500000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "300.00");
        // Enter → modal → confirm.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.classify_raw_modal.is_some(), "CR-ACQ: modal opens");
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.classify_raw_flow.is_none() && app.classify_raw_modal.is_none(),
            "CR-ACQ: confirm closes modal + flow"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("Unclassified blocker is cleared"),
            "CR-ACQ: clean-arm status; got: {:?}",
            app.status
        );

        // Re-projection: the Unclassified blocker is gone; the ClassifyRaw round-trips as Acquire.
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !unclassified_blocker_present(snap, &target),
            "CR-ACQ: the Unclassified blocker must be cleared after classify"
        );
        let cr = snap
            .events
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::ClassifyRaw(cr) if cr.target == target => Some(cr),
                _ => None,
            })
            .expect("CR-ACQ: a ClassifyRaw decision must exist");
        match &*cr.as_ {
            EventPayload::Acquire(a) => {
                assert_eq!(a.sat, 500_000, "CR-ACQ: sat");
                assert_eq!(
                    a.usd_cost,
                    rust_decimal_macros::dec!(300.00),
                    "CR-ACQ: usd_cost"
                );
                assert_eq!(a.fee_usd, rust_decimal::Decimal::ZERO, "CR-ACQ: fee → $0");
                assert_eq!(
                    a.basis_source,
                    BasisSource::ExchangeProvided,
                    "CR-ACQ: default basis_source"
                );
            }
            other => panic!("CR-ACQ: expected Acquire, got {other:?}"),
        }
    }

    // ── KAT-E2E-CR-INCOME — classify a raw row as Income (typed FMV) ──────────
    #[test]
    fn kat_e2e_cr_income() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-cr-inc-pass";

        let target = seed_unclassified_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        // u → List → Enter → VariantPicker → Tab → Income → Enter → IncomeForm.
        handle_key(&mut app, press(KeyCode::Char('u')));
        handle_key(&mut app, press(KeyCode::Enter));
        handle_key(&mut app, press(KeyCode::Tab)); // Acquire → Income
        assert!(
            matches!(
                app.classify_raw_flow.as_ref().map(|f| &f.step),
                Some(ClassifyRawStep::VariantPicker {
                    variant: ClassifyRawVariant::Income,
                    ..
                })
            ),
            "CR-INC: Tab cycles to Income"
        );
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            matches!(
                app.classify_raw_flow.as_ref().map(|f| &f.step),
                Some(ClassifyRawStep::IncomeForm { .. })
            ),
            "CR-INC: Enter opens IncomeForm"
        );

        // sat (focus 0) → Down → usd_fmv (focus 1) typed → fmv_status=ManualEntry.
        type_str(&mut app, "250000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "180.00");
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.classify_raw_modal.is_some(), "CR-INC: modal opens");
        handle_key(&mut app, press(KeyCode::Enter)); // confirm
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("Unclassified blocker is cleared"),
            "CR-INC: clean-arm status (typed FMV → no FmvMissing); got: {:?}",
            app.status
        );

        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            !unclassified_blocker_present(snap, &target),
            "CR-INC: Unclassified blocker cleared"
        );
        let cr = snap
            .events
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::ClassifyRaw(cr) if cr.target == target => Some(cr),
                _ => None,
            })
            .expect("CR-INC: a ClassifyRaw decision must exist");
        match &*cr.as_ {
            EventPayload::Income(i) => {
                assert_eq!(i.sat, 250_000, "CR-INC: sat");
                assert_eq!(
                    i.usd_fmv,
                    Some(rust_decimal_macros::dec!(180.00)),
                    "CR-INC: usd_fmv"
                );
                assert_eq!(
                    i.fmv_status,
                    btctax_core::FmvStatus::ManualEntry,
                    "CR-INC: typed FMV → ManualEntry"
                );
                assert_eq!(i.kind, IncomeKind::Mining, "CR-INC: default kind");
                assert!(!i.business, "CR-INC: default business=false");
            }
            other => panic!("CR-INC: expected Income, got {other:?}"),
        }
    }

    // ── KAT-CR-FMV-MISSING — Income with an EMPTY usd_fmv → FmvMissing arm ─────
    #[test]
    fn kat_cr_income_empty_fmv_missing_arm() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-cr-fmv-pass";

        let target = seed_unclassified_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        handle_key(&mut app, press(KeyCode::Char('u')));
        handle_key(&mut app, press(KeyCode::Enter));
        handle_key(&mut app, press(KeyCode::Tab)); // → Income
        handle_key(&mut app, press(KeyCode::Enter)); // → IncomeForm
                                                     // sat only; LEAVE usd_fmv empty → fmv_status = Missing → FmvMissing blocker.
        type_str(&mut app, "250000");
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm

        // Round-trip: fmv_status = Missing, usd_fmv = None.
        let snap = app.snapshot.as_ref().unwrap();
        let cr = snap
            .events
            .iter()
            .find_map(|e| match &e.payload {
                EventPayload::ClassifyRaw(cr) if cr.target == target => Some(cr),
                _ => None,
            })
            .expect("CR-FMV: a ClassifyRaw decision must exist");
        match &*cr.as_ {
            EventPayload::Income(i) => {
                assert_eq!(i.usd_fmv, None, "CR-FMV: empty → None");
                assert_eq!(
                    i.fmv_status,
                    btctax_core::FmvStatus::Missing,
                    "CR-FMV: empty → Missing"
                );
            }
            other => panic!("CR-FMV: expected Income, got {other:?}"),
        }
        // The Unclassified blocker cleared, but a FmvMissing blocker now attributes to the target.
        assert!(
            !unclassified_blocker_present(snap, &target),
            "CR-FMV: the Unclassified blocker must be cleared"
        );
        assert!(
            snap.state
                .blockers
                .iter()
                .any(|b| b.kind == BlockerKind::FmvMissing && b.event.as_ref() == Some(&target)),
            "CR-FMV: an empty-FMV Income must fire FmvMissing on the target"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or("")
                .contains("FmvMissing now applies"),
            "CR-FMV: status arm 3 must name FmvMissing; got: {:?}",
            app.status
        );
    }

    // ── KAT-CR-VARIANTS — only Acquire + Income are offered ───────────────────
    #[test]
    fn kat_cr_only_acquire_income_offered() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-cr-variants-pass";

        let _target = seed_unclassified_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        handle_key(&mut app, press(KeyCode::Char('u')));
        handle_key(&mut app, press(KeyCode::Enter)); // → VariantPicker(Acquire)
                                                     // Tab cycles through EXACTLY two variants (Acquire → Income → Acquire).
        let variant = |app: &EditorApp| match app.classify_raw_flow.as_ref().map(|f| &f.step) {
            Some(ClassifyRawStep::VariantPicker { variant, .. }) => Some(*variant),
            _ => None,
        };
        assert_eq!(variant(&app), Some(ClassifyRawVariant::Acquire));
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(variant(&app), Some(ClassifyRawVariant::Income));
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(
            variant(&app),
            Some(ClassifyRawVariant::Acquire),
            "CR-VAR: only Acquire + Income are offered (unsupported variants absent)"
        );
    }

    // ── KAT-C2-CR — cancel path: q swallowed each step, Esc steps back, bytes unchanged ──
    #[test]
    fn kat_c2_cr_cancel_path_bytes_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-cr-cancel-pass";

        let _target = seed_unclassified_vault(&vault, &key, pp_str);
        let bytes_before = std::fs::read(&vault).unwrap();
        {
            let mut app = open_app(&vault, pp_str);

            handle_key(&mut app, press(KeyCode::Char('u')));
            assert!(app.classify_raw_flow.is_some(), "C2-CR: flow opens");
            // 'q' swallowed at List.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit && app.classify_raw_flow.is_some(),
                "C2-CR: 'q' swallowed at List"
            );

            handle_key(&mut app, press(KeyCode::Enter)); // → VariantPicker
                                                         // 'q' swallowed at VariantPicker.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit && app.classify_raw_flow.is_some(),
                "C2-CR: 'q' swallowed at VariantPicker"
            );

            handle_key(&mut app, press(KeyCode::Enter)); // → AcquireForm
            assert!(
                matches!(
                    app.classify_raw_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyRawStep::AcquireForm { .. })
                ),
                "C2-CR: Enter opens AcquireForm"
            );
            // Fill valid fields and open the modal.
            type_str(&mut app, "500000");
            handle_key(&mut app, press(KeyCode::Down));
            type_str(&mut app, "300.00");
            handle_key(&mut app, press(KeyCode::Enter)); // → modal
            assert!(app.classify_raw_modal.is_some(), "C2-CR: modal opens");
            // 'q' swallowed at modal.
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(
                !app.should_quit && app.classify_raw_modal.is_some(),
                "C2-CR: 'q' swallowed at modal"
            );
            // Esc closes modal → back to AcquireForm.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.classify_raw_modal.is_none()
                    && matches!(
                        app.classify_raw_flow.as_ref().map(|f| &f.step),
                        Some(ClassifyRawStep::AcquireForm { .. })
                    ),
                "C2-CR: Esc closes modal → AcquireForm"
            );
            // Esc → back to VariantPicker.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.classify_raw_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyRawStep::VariantPicker { .. })
                ),
                "C2-CR: Esc steps back to VariantPicker"
            );
            // Esc → back to List.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                matches!(
                    app.classify_raw_flow.as_ref().map(|f| &f.step),
                    Some(ClassifyRawStep::List)
                ),
                "C2-CR: Esc steps back to List"
            );
            // Esc → close flow.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(
                app.classify_raw_flow.is_none(),
                "C2-CR: Esc closes the flow"
            );
        }
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "C2-CR: the cancel path writes NOTHING"
        );
    }

    // ── KAT-S3-CR — save-error path (chmod) → rollback, retry clean, no residue ──
    #[test]
    #[cfg(unix)]
    fn kat_s3_cr_save_error_chmod() {
        use btctax_core::persistence::load_all_ordered;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-cr-s3-pass";

        let _target = seed_unclassified_vault(&vault, &key, pp_str);

        // Root-skip guard.
        {
            let probe = dir.path().join("probe.tmp");
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
            let can_write = std::fs::write(&probe, b"x").is_ok();
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
            if can_write {
                eprintln!("KAT-S3-CR: skipping — chmod did not deny writes (root?)");
                return;
            }
        }

        let bytes_before = std::fs::read(&vault).unwrap();
        let pre_event_count = {
            let session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            load_all_ordered(session.conn()).unwrap().len()
        };

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('u')));
        handle_key(&mut app, press(KeyCode::Enter)); // → VariantPicker(Acquire)
        handle_key(&mut app, press(KeyCode::Enter)); // → AcquireForm
        type_str(&mut app, "500000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "300.00");
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.classify_raw_modal.is_some(), "S3-CR: modal must open");

        let parent = vault.parent().unwrap();
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → save fails
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

        assert!(
            app.classify_raw_modal.is_none(),
            "S3-CR: modal closes on save failure"
        );
        assert!(
            matches!(
                app.classify_raw_flow.as_ref().map(|f| &f.step),
                Some(ClassifyRawStep::AcquireForm { .. })
            ),
            "S3-CR: AcquireForm stays open after a save failure"
        );
        assert!(
            app.status
                .as_deref()
                .map(|s| s.contains("Save error"))
                .unwrap_or(false),
            "S3-CR: status must contain 'Save error'; got: {:?}",
            app.status
        );
        let bytes_mid = std::fs::read(&vault).unwrap();
        assert_eq!(
            bytes_before, bytes_mid,
            "S3-CR: vault bytes unchanged after failed save"
        );
        let mid_len = load_all_ordered(app.session.as_ref().unwrap().conn())
            .unwrap()
            .len();
        assert_eq!(mid_len, pre_event_count, "S3-CR: rollback → no residue");

        // Retry → clean single append.
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm → retry save
        assert!(
            app.classify_raw_flow.is_none(),
            "S3-CR: flow closes after clean retry"
        );
        drop(app);
        let session2 = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let post = load_all_ordered(session2.conn()).unwrap();
        assert_eq!(
            post.len(),
            pre_event_count + 1,
            "S3-CR: retry appends EXACTLY one ClassifyRaw (no residue)"
        );
    }

    // ── KAT-RENDER-CR — the classify-raw overlays render at every step (no panic) ──
    #[test]
    fn kat_render_classify_raw_smoke() {
        use ratatui::{backend::TestBackend, Terminal};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-render-cr-pass";

        let _target = seed_unclassified_vault(&vault, &key, pp_str);
        let mut app = open_app(&vault, pp_str);

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        handle_key(&mut app, press(KeyCode::Char('u')));
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("Classify Raw"),
            "CR-RENDER: list overlay"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // VariantPicker(Acquire)
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("variant picker"),
            "CR-RENDER: variant picker overlay"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // AcquireForm
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("basis_source"),
            "CR-RENDER: Acquire form overlay"
        );

        // Fill and open the modal.
        type_str(&mut app, "500000");
        handle_key(&mut app, press(KeyCode::Down));
        type_str(&mut app, "300.00");
        handle_key(&mut app, press(KeyCode::Enter)); // modal
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("classify-raw"),
            "CR-RENDER: confirm modal overlay"
        );

        // Income form path renders too.
        handle_key(&mut app, press(KeyCode::Esc)); // close modal → AcquireForm
        handle_key(&mut app, press(KeyCode::Esc)); // → VariantPicker
        handle_key(&mut app, press(KeyCode::Tab)); // → Income
        handle_key(&mut app, press(KeyCode::Enter)); // IncomeForm
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("business"),
            "CR-RENDER: Income form overlay"
        );
    }

    // ── KAT-RENDER-LT — the link-transfer overlays render at every step (no panic) ──
    // (Back-fills render coverage for the Task-1 flow; the Task-1 E2E asserts behaviour
    // via handle_key + re-projection, not the draw path.)
    #[test]
    fn kat_render_link_transfer_smoke() {
        use btctax_core::event::{EventPayload, LedgerEvent, TransferIn};
        use btctax_core::identity::{Source, SourceRef};
        use ratatui::{backend::TestBackend, Terminal};
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-render-lt-pass";

        let ti = LedgerEvent {
            id: EventId::import(Source::River, SourceRef::new("lt-render-ti")),
            utc_timestamp: OffsetDateTime::from_unix_timestamp(1_740_800_000).unwrap(),
            original_tz: UtcOffset::UTC,
            wallet: Some(kraken_wallet()),
            payload: EventPayload::TransferIn(TransferIn {
                sat: 500_000,
                src_addr: None,
                txid: None,
            }),
        };
        let _to_id = seed_link_out_vault(&vault, &key, pp_str, &[ti]);
        let mut app = open_app(&vault, pp_str);

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        handle_key(&mut app, press(KeyCode::Char('l')));
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("Link Transfer"),
            "LT-RENDER: out-list overlay"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // TargetPick(InEvent)
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("InEvent"),
            "LT-RENDER: target-pick InEvent overlay"
        );

        handle_key(&mut app, press(KeyCode::Tab)); // Wallet mode
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("Wallet"),
            "LT-RENDER: target-pick Wallet overlay"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // modal
        terminal.draw(|f| draw_edit::draw(f, &mut app)).unwrap();
        assert!(
            rendered_text(&terminal).contains("link-transfer"),
            "LT-RENDER: confirm modal overlay"
        );
    }

    // ── Resolve-conflict flow (chunk 4b, D3) ─────────────────────────────────

    /// Seed a vault with ONE unresolved ImportConflict on an Acquire (usd_cost 30000 → 50000).
    fn seed_conflict_vault(vault: &std::path::Path, key: &std::path::Path, pp_str: &str) {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::append_import_batch;
        use btctax_core::{EventId, WalletId};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let target = EventId::import(Source::River, SourceRef::new("rc-e2e"));
        let wallet = Some(WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        });
        let ts = OffsetDateTime::from_unix_timestamp(1_740_000_000).unwrap();
        let acq = |usd: rust_decimal::Decimal| {
            vec![LedgerEvent {
                id: target.clone(),
                utc_timestamp: ts,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 100_000,
                    usd_cost: usd,
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            }]
        };
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        append_import_batch(session.conn(), &acq(dec!(30000))).unwrap();
        session.save().unwrap();
        append_import_batch(session.conn(), &acq(dec!(50000))).unwrap();
        session.save().unwrap();
    }

    // ── KAT-C2-RC — cancel-path bytes-unchanged (resolve-conflict) ───────────
    #[test]
    fn kat_c2_rc_cancel_path_vault_bytes_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-c2rc-pass";
        seed_conflict_vault(&vault, &key, pp_str);

        let bytes_before = std::fs::read(&vault).unwrap();
        {
            let mut app = open_app(&vault, pp_str);

            // i → flow opens at List.
            handle_key(&mut app, press(KeyCode::Char('i')));
            assert!(app.resolve_conflict_flow.is_some(), "C2-RC: 'i' opens flow");

            // q swallowed (flow stays open, editor does not quit).
            handle_key(&mut app, press(KeyCode::Char('q')));
            assert!(!app.should_quit, "C2-RC: q must not quit mid-flow");
            assert!(app.resolve_conflict_flow.is_some(), "C2-RC: q swallowed");

            // Enter → Choose step.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(matches!(
                app.resolve_conflict_flow.as_ref().map(|f| &f.step),
                Some(ResolveConflictStep::Choose { .. })
            ));

            // toggle Accept ⇄ Reject.
            handle_key(&mut app, press(KeyCode::Right));
            assert!(matches!(
                app.resolve_conflict_flow.as_ref().map(|f| &f.step),
                Some(ResolveConflictStep::Choose {
                    kind: ResolveKind::Reject,
                    ..
                })
            ));
            handle_key(&mut app, press(KeyCode::Left));

            // Enter → modal.
            handle_key(&mut app, press(KeyCode::Enter));
            assert!(app.resolve_conflict_modal.is_some(), "C2-RC: modal opens");

            // Esc → modal closed, back to Choose (flow open).
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.resolve_conflict_modal.is_none());
            assert!(matches!(
                app.resolve_conflict_flow.as_ref().map(|f| &f.step),
                Some(ResolveConflictStep::Choose { .. })
            ));

            // Esc → List; Esc → close.
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(matches!(
                app.resolve_conflict_flow.as_ref().map(|f| &f.step),
                Some(ResolveConflictStep::List)
            ));
            handle_key(&mut app, press(KeyCode::Esc));
            assert!(app.resolve_conflict_flow.is_none(), "C2-RC: flow closed");
        }
        let bytes_after = std::fs::read(&vault).unwrap();
        assert_eq!(bytes_before, bytes_after, "C2-RC: cancel writes nothing");
    }

    // ── KAT-E2E-RC-ACCEPT — full 'i' accept path adopts new payload, clears blocker ──
    #[test]
    fn kat_e2e_rc_accept() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-rc-a-pass";
        seed_conflict_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('i'))); // List
        handle_key(&mut app, press(KeyCode::Enter)); // Choose (default Accept)
        handle_key(&mut app, press(KeyCode::Enter)); // modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm & save

        assert!(app.resolve_conflict_modal.is_none() && app.resolve_conflict_flow.is_none());
        let status = app.status.clone().unwrap_or_default();
        assert!(
            status.contains("accepted") && status.contains("import-conflict resolved"),
            "E2E-RC-ACCEPT: status must confirm resolution; got {status:?}"
        );
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            snap.state
                .blockers
                .iter()
                .all(|b| b.kind != BlockerKind::ImportConflict),
            "E2E-RC-ACCEPT: ImportConflict blocker must clear"
        );
        let lot = snap
            .state
            .lots
            .iter()
            .find(|l| l.original_sat == 100_000)
            .unwrap();
        assert_eq!(
            lot.usd_basis,
            rust_decimal_macros::dec!(50000),
            "E2E-RC-ACCEPT: lot must adopt the NEW basis 50000"
        );
    }

    // ── KAT-E2E-RC-REJECT — full 'i' reject path keeps original, clears blocker ──
    #[test]
    fn kat_e2e_rc_reject() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-rc-r-pass";
        seed_conflict_vault(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('i'))); // List
        handle_key(&mut app, press(KeyCode::Enter)); // Choose (Accept)
        handle_key(&mut app, press(KeyCode::Right)); // toggle → Reject
        handle_key(&mut app, press(KeyCode::Enter)); // modal
        handle_key(&mut app, press(KeyCode::Enter)); // confirm & save

        let status = app.status.clone().unwrap_or_default();
        assert!(
            status.contains("rejected") && status.contains("import-conflict resolved"),
            "E2E-RC-REJECT: status must confirm resolution; got {status:?}"
        );
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            snap.state
                .blockers
                .iter()
                .all(|b| b.kind != BlockerKind::ImportConflict),
            "E2E-RC-REJECT: ImportConflict blocker must clear"
        );
        let lot = snap
            .state
            .lots
            .iter()
            .find(|l| l.original_sat == 100_000)
            .unwrap();
        assert_eq!(
            lot.usd_basis,
            rust_decimal_macros::dec!(30000),
            "E2E-RC-REJECT: lot must keep the ORIGINAL basis 30000"
        );
    }

    // ── Optimize-accept flow (chunk 4b, D4) ──────────────────────────────────

    /// A single-lot candidate item for the flow-step tests.
    fn oa_item(persistable: Persistability) -> OptimizeCandidateItem {
        use btctax_core::identity::{LotId, Source, SourceRef};
        OptimizeCandidateItem {
            disposal: EventId::import(Source::River, SourceRef::new("oa-item-disp")),
            wallet: btctax_core::WalletId::Exchange {
                provider: "River".into(),
                account: "main".into(),
            },
            date: time::macros::date!(2025 - 05 - 23),
            persistable,
            picks: vec![btctax_core::LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::River, SourceRef::new("oa-item-lot")),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        }
    }

    fn oa_flow_app(persistable: Persistability) -> EditorApp {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        app.optimize_accept_flow = Some(OptimizeAcceptFlowState {
            list: TargetList::new(vec![oa_item(persistable)]),
            step: OptimizeAcceptStep::List,
            delta: rust_decimal_macros::dec!(-1234),
            approximate: false,
        });
        app
    }

    // ── KAT-OA-CONTEMP-SKIP — ContemporaneousNow → modal directly (no text step) ──
    #[test]
    fn kat_oa_contemporaneous_skips_text_step() {
        let mut app = oa_flow_app(Persistability::ContemporaneousNow);
        handle_key(&mut app, press(KeyCode::Enter)); // pick
        assert!(
            app.optimize_accept_modal.is_some(),
            "OA-CONTEMP: modal opens directly"
        );
        let m = app.optimize_accept_modal.as_ref().unwrap();
        assert!(m.attestation.is_none(), "OA-CONTEMP: no attestation");
        assert_eq!(m.basis_label, "Contemporaneous");
        // Flow stayed at List (never entered AttestText).
        assert!(matches!(
            app.optimize_accept_flow.as_ref().map(|f| &f.step),
            Some(OptimizeAcceptStep::List)
        ));
    }

    // ── KAT-OA-NEEDS-ATTEST — NeedsAttestation → text step; empty rejected, text → modal ──
    #[test]
    fn kat_oa_needs_attestation_requires_nonempty_text() {
        let mut app = oa_flow_app(Persistability::NeedsAttestation);
        handle_key(&mut app, press(KeyCode::Enter)); // pick → AttestText
        assert!(app.optimize_accept_modal.is_none(), "no modal yet");
        assert!(matches!(
            app.optimize_accept_flow.as_ref().map(|f| &f.step),
            Some(OptimizeAcceptStep::AttestText { .. })
        ));

        // Empty text → Enter → error, still no modal.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.optimize_accept_modal.is_none(),
            "OA-NEEDS: empty text must NOT open the modal"
        );
        let has_error = matches!(
            app.optimize_accept_flow.as_ref().map(|f| &f.step),
            Some(OptimizeAcceptStep::AttestText { error: Some(_), .. })
        );
        assert!(has_error, "OA-NEEDS: empty submit sets an error");

        // Type text → Enter → modal opens, attested.
        type_str(&mut app, "contemporaneous-id");
        handle_key(&mut app, press(KeyCode::Enter));
        let m = app
            .optimize_accept_modal
            .as_ref()
            .expect("OA-NEEDS: non-empty text opens the modal");
        assert_eq!(m.attestation.as_deref(), Some("contemporaneous-id"));
        assert_eq!(m.basis_label, "AttestedRecording");
    }

    // ── KAT-E2E-OA-Z — full 'z' path via Session::optimize_proposal (attested) ──
    fn oa_profile_2025() -> btctax_core::TaxProfile {
        use rust_decimal_macros::dec;
        btctax_core::TaxProfile {
            filing_status: btctax_core::FilingStatus::Single,
            ordinary_taxable_income: dec!(100000),
            magi_excluding_crypto: dec!(100000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: btctax_core::Carryforward {
                short: dec!(0),
                long: dec!(0),
            },
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        }
    }

    /// Seed a computable 2025 year: two same-wallet 2025 lots (A cheap $30k, B dearer $50k) + a 500k
    /// sale (FIFO baseline consumes cheaper lot A → higher gain; the optimizer prefers dearer lot B →
    /// a persistable proposed row). NO back-dated MethodElection (default FIFO config keeps 2025
    /// computable — a MethodElection effective before its own made-date would fire
    /// `MethodElectionBackdated`). Returns the sale's disposal EventId.
    fn oa_seed_computable_sell(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> btctax_core::EventId {
        use btctax_core::event::{
            Acquire, BasisSource, DisposeKind, EventPayload, LedgerEvent, OutflowClass,
            ReclassifyOutflow, TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::{EventId, WalletId};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let wallet = Some(WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        let lot_a = EventId::import(Source::River, SourceRef::new("oa-lot-a"));
        let lot_b = EventId::import(Source::River, SourceRef::new("oa-lot-b"));
        let to_id = EventId::import(Source::River, SourceRef::new("oa-sell"));

        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let ta = OffsetDateTime::from_unix_timestamp(1_739_000_000).unwrap(); // 2025-02 lot A
        let tb = OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap(); // 2025-03 lot B
        let tc = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(); // 2025-05 sell
        let td = OffsetDateTime::from_unix_timestamp(1_748_100_000).unwrap(); // decisions
        let batch = vec![
            LedgerEvent {
                id: lot_a.clone(),
                utc_timestamp: ta,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(30000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: lot_b.clone(),
                utc_timestamp: tb,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(50000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: to_id.clone(),
                utc_timestamp: tc,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 500_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
        ];
        btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
        let ro = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: to_id.clone(),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(30000),
            fee_usd: None,
            donee: None,
        });
        btctax_core::persistence::append_decision(session.conn(), ro, td, UtcOffset::UTC, None)
            .unwrap();
        session.save().unwrap();
        to_id
    }

    #[test]
    fn kat_e2e_oa_z_attested_persists_lotselection_and_attest_row() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-oa-pass";

        let to_id = oa_seed_computable_sell(&vault, &key, pp_str);

        // Set a 2025 profile so `optimize_year` computes (else YearNotComputable → no open).
        {
            let mut s = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            btctax_cli::tax_profile::set(s.conn(), 2025, &oa_profile_2025()).unwrap();
            s.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);
        app.selected_year = 2025;

        // z → opener recomputes via Session::optimize_proposal; the optimizer prefers dearer lot B
        // (lower gain) over FIFO's lot A → a persistable proposed row for the 2025 sale.
        handle_key(&mut app, press(KeyCode::Char('z')));
        let flow = app
            .optimize_accept_flow
            .as_ref()
            .expect("OA-Z: flow must open with a persistable candidate");
        assert!(
            flow.list.items.iter().any(|c| c.disposal == to_id),
            "OA-Z: the 2025 sale must be a candidate"
        );
        // Already-executed (made 2026 > sale 2025), own-books exchange → NeedsAttestation.
        assert!(
            matches!(
                flow.list
                    .items
                    .iter()
                    .find(|c| c.disposal == to_id)
                    .unwrap()
                    .persistable,
                Persistability::NeedsAttestation
            ),
            "OA-Z: an already-executed own-books disposal needs attestation"
        );

        handle_key(&mut app, press(KeyCode::Enter)); // pick → AttestText
        type_str(&mut app, "attest-2025");
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        assert!(app.optimize_accept_modal.is_some(), "OA-Z: modal opens");
        handle_key(&mut app, press(KeyCode::Enter)); // confirm & save

        assert!(app.optimize_accept_modal.is_none() && app.optimize_accept_flow.is_none());
        let status = app.status.clone().unwrap_or_default();
        assert!(
            status.contains("Optimizer selection recorded")
                && status.contains("attestation recorded"),
            "OA-Z: status must confirm the attested recording; got {status:?}"
        );

        // The attestation row is co-persisted for the disposal.
        let att =
            btctax_cli::optimize_attest::get(app.session.as_ref().unwrap().conn(), &to_id).unwrap();
        assert_eq!(
            att.as_deref(),
            Some("attest-2025"),
            "OA-Z: attest row co-persisted with the LotSelection"
        );

        // The re-projected snapshot carries a LotSelection targeting the sale.
        let snap = app.snapshot.as_ref().unwrap();
        assert!(
            snap.events.iter().any(|e| matches!(
                &e.payload,
                btctax_core::EventPayload::LotSelection(ls) if ls.disposal_event == to_id
            )),
            "OA-Z: a LotSelection for the sale must be persisted"
        );
    }

    // ── Bulk link-transfer TUI KATs (bulk-link-transfer D3) ──────────────────

    /// Seed a vault with an Acquire lot + TWO pending TransferOuts from the SAME wallet
    /// (River/main), on distinct 2025 dates. Returns `(o1 @ 2025-03-01, o2 @ 2025-06-15)`.
    fn seed_bulk_one_wallet(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> (btctax_core::EventId, btctax_core::EventId) {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::macros::datetime;
        use time::UtcOffset;

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let wallet = Some(btctax_core::WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        });
        let acq = EventId::import(Source::River, SourceRef::new("bulk-acq"));
        let o1 = EventId::import(Source::River, SourceRef::new("bulk-o1"));
        let o2 = EventId::import(Source::River, SourceRef::new("bulk-o2"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let out = |sat: i64| {
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            })
        };
        let batch = vec![
            LedgerEvent {
                id: acq,
                utc_timestamp: datetime!(2024-12-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(30000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: o1.clone(),
                utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: out(100_000),
            },
            LedgerEvent {
                id: o2.clone(),
                utc_timestamp: datetime!(2025-06-15 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet,
                payload: out(50_000),
            },
        ];
        btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
        session.save().unwrap();
        (o1, o2)
    }

    /// Seed a vault with pending outs from TWO wallets. Returns `(river_out, coinbase_out)`.
    fn seed_bulk_two_wallets(
        vault: &std::path::Path,
        key: &std::path::Path,
        pp_str: &str,
    ) -> (btctax_core::EventId, btctax_core::EventId) {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent, TransferOut};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::macros::datetime;
        use time::UtcOffset;

        btctax_cli::cmd::init::run(vault, &Passphrase::new(pp_str.into()), key).unwrap();
        let river = Some(btctax_core::WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        });
        let coinbase = Some(btctax_core::WalletId::Exchange {
            provider: "Coinbase".into(),
            account: "main".into(),
        });
        let acq_r = EventId::import(Source::River, SourceRef::new("bulk-acq-r"));
        let acq_c = EventId::import(Source::Coinbase, SourceRef::new("bulk-acq-c"));
        let river_out = EventId::import(Source::River, SourceRef::new("bulk-out-r"));
        let coinbase_out = EventId::import(Source::Coinbase, SourceRef::new("bulk-out-c"));
        let mut session =
            btctax_cli::Session::open(vault, &Passphrase::new(pp_str.into())).unwrap();
        let out = |sat: i64| {
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat: None,
                dest_addr: None,
                txid: None,
            })
        };
        let acq = |sat: i64| {
            EventPayload::Acquire(Acquire {
                sat,
                usd_cost: dec!(20000),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            })
        };
        let batch = vec![
            LedgerEvent {
                id: acq_r,
                utc_timestamp: datetime!(2024-12-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: river.clone(),
                payload: acq(500_000),
            },
            LedgerEvent {
                id: acq_c,
                utc_timestamp: datetime!(2024-12-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: coinbase.clone(),
                payload: acq(500_000),
            },
            LedgerEvent {
                id: river_out.clone(),
                utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: river,
                payload: out(100_000),
            },
            LedgerEvent {
                id: coinbase_out.clone(),
                utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: coinbase,
                payload: out(80_000),
            },
        ];
        btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
        session.save().unwrap();
        (river_out, coinbase_out)
    }

    /// `b` on a vault with NO pending outbound transfers refuses (no flow opened, status set).
    #[test]
    fn kat_bulk_refuses_when_no_pending() {
        use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent};
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::EventId;
        use rust_decimal_macros::dec;
        use time::macros::datetime;
        use time::UtcOffset;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-nopending";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();
        {
            let mut session =
                btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
            let batch = vec![LedgerEvent {
                id: EventId::import(Source::River, SourceRef::new("acq-only")),
                utc_timestamp: datetime!(2024-12-01 12:00:00 UTC),
                original_tz: UtcOffset::UTC,
                wallet: Some(btctax_core::WalletId::Exchange {
                    provider: "River".into(),
                    account: "main".into(),
                }),
                payload: EventPayload::Acquire(Acquire {
                    sat: 100_000,
                    usd_cost: dec!(3000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            }];
            btctax_core::persistence::append_import_batch(session.conn(), &batch).unwrap();
            session.save().unwrap();
        }

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('b')));
        assert!(
            app.bulk_link_flow.is_none(),
            "no pending outs → flow must NOT open"
        );
        assert_eq!(
            app.status.as_deref(),
            Some("No pending outbound transfers to bulk-link")
        );
    }

    /// Unchecking a preview row omits it from the confirm modal's appended batch.
    #[test]
    fn kat_bulk_per_row_exclude_drops_row() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-exclude";
        let (o1, o2) = seed_bulk_one_wallet(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('b'))); // DestPick
        handle_key(&mut app, press(KeyCode::Char('n'))); // typed dest
        type_str(&mut app, "self:cold");
        handle_key(&mut app, press(KeyCode::Enter)); // → Filter
        assert!(matches!(
            app.bulk_link_flow.as_ref().map(|f| &f.step),
            Some(BulkLinkStep::Filter)
        ));
        handle_key(&mut app, press(KeyCode::Enter)); // Filter (All/Any) → Preview
        {
            let f = app.bulk_link_flow.as_ref().unwrap();
            assert!(matches!(f.step, BulkLinkStep::Preview));
            assert_eq!(f.preview.items.len(), 2, "both outs included, all checked");
            assert!(f.preview.items.iter().all(|i| i.checked));
            assert_eq!(
                f.preview.items[0].out_event, o1,
                "row 0 sorted-by-date is o1"
            );
        }
        // Exclude row 0 (o1) → only o2 remains checked.
        handle_key(&mut app, press(KeyCode::Char(' ')));
        assert!(!app.bulk_link_flow.as_ref().unwrap().preview.items[0].checked);
        handle_key(&mut app, press(KeyCode::Enter)); // → confirm modal
        let m = app
            .bulk_link_modal
            .as_ref()
            .expect("confirm modal must open over checked rows");
        assert_eq!(m.count, 1, "one row excluded → one remains");
        assert_eq!(
            m.out_events,
            vec![o2],
            "excluded o1 is absent from the batch"
        );
    }

    /// A same-wallet row (source == dest) is skipped — absent from the preview/batch.
    #[test]
    fn kat_bulk_same_wallet_row_absent() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-samewallet";
        let (river_out, coinbase_out) = seed_bulk_two_wallets(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('b'))); // DestPick
                                                         // Wallet union sorted: [Coinbase/main, River/main]. Pick River/main (idx 1) as dest.
        handle_key(&mut app, press(KeyCode::Down));
        handle_key(&mut app, press(KeyCode::Enter)); // → Filter (dest = River/main)
        handle_key(&mut app, press(KeyCode::Enter)); // Filter (All/Any) → Preview
        let f = app.bulk_link_flow.as_ref().unwrap();
        assert!(matches!(f.step, BulkLinkStep::Preview));
        assert_eq!(
            f.preview.items.len(),
            1,
            "the River/main out (source == dest) is skipped; only the Coinbase out remains"
        );
        assert_eq!(f.preview.items[0].out_event, coinbase_out);
        assert!(
            f.preview.items.iter().all(|i| i.out_event != river_out),
            "the same-wallet row must be ABSENT from the preview"
        );
    }

    /// [Fork B] Typing a never-seen cold wallet (`self:cold-wallet`) yields a `SelfCustody`
    /// destination the batch links to.
    #[test]
    fn kat_bulk_typed_dest_cold_wallet() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-bulk-typeddest";
        let (o1, o2) = seed_bulk_one_wallet(&vault, &key, pp_str);
        let cold = btctax_core::WalletId::SelfCustody {
            label: "cold-wallet".into(),
        };

        let mut app = open_app(&vault, pp_str);
        handle_key(&mut app, press(KeyCode::Char('b'))); // DestPick
        handle_key(&mut app, press(KeyCode::Char('n'))); // typed dest
        type_str(&mut app, "self:cold-wallet");
        handle_key(&mut app, press(KeyCode::Enter)); // parse → Filter
        assert_eq!(
            app.bulk_link_flow.as_ref().and_then(|f| f.dest.clone()),
            Some(cold.clone()),
            "typed cold wallet parses to a SelfCustody destination"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // Filter → Preview
        assert_eq!(
            app.bulk_link_flow.as_ref().unwrap().preview.items.len(),
            2,
            "cold-wallet is no source → nothing skipped"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // → confirm modal
        let m = app.bulk_link_modal.as_ref().expect("modal opens");
        assert_eq!(
            m.dest, cold,
            "the batch links to the never-seen cold wallet"
        );
        assert_eq!(m.out_events, vec![o1, o2]);
    }

    // ── Bulk link-transfer E2E round-trips (bulk-link-transfer Task 3) ────────

    /// E2E: `b` → typed dest → filter → exclude one → confirm → APPLY. The included out projects
    /// as `Op::SelfTransfer` (relocated to the dest wallet, absent from pending); the excluded out
    /// stays pending.
    #[test]
    fn kat_e2e_bulk_link_then_selftransfer() {
        use btctax_core::{EventPayload, TransferTarget};
        use std::collections::BTreeSet;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-bulk-st";
        let (o1, o2) = seed_bulk_one_wallet(&vault, &key, pp_str);
        let cold = btctax_core::WalletId::SelfCustody {
            label: "cold".into(),
        };

        let mut app = open_app(&vault, pp_str);
        {
            let snap = app.snapshot.as_ref().unwrap();
            let pend: BTreeSet<_> = snap
                .state
                .pending_reconciliation
                .iter()
                .map(|p| p.event.clone())
                .collect();
            assert!(
                pend.contains(&o1) && pend.contains(&o2),
                "both outs start pending"
            );
        }

        // b → n → self:cold → Filter → Preview.
        handle_key(&mut app, press(KeyCode::Char('b')));
        handle_key(&mut app, press(KeyCode::Char('n')));
        type_str(&mut app, "self:cold");
        handle_key(&mut app, press(KeyCode::Enter)); // → Filter
        handle_key(&mut app, press(KeyCode::Enter)); // → Preview
                                                     // Exclude o1 (row 0, sorted by date), keep o2.
        handle_key(&mut app, press(KeyCode::Char(' ')));
        handle_key(&mut app, press(KeyCode::Enter)); // → confirm modal
        assert!(app.bulk_link_modal.is_some());
        handle_key(&mut app, press(KeyCode::Enter)); // APPLY (persist + re-project)

        assert!(app.bulk_link_flow.is_none() && app.bulk_link_modal.is_none());
        let snap = app.snapshot.as_ref().unwrap();
        let pend: BTreeSet<_> = snap
            .state
            .pending_reconciliation
            .iter()
            .map(|p| p.event.clone())
            .collect();
        assert_eq!(
            pend.len(),
            1,
            "one out linked, one excluded remains pending"
        );
        assert!(
            pend.contains(&o1) && !pend.contains(&o2),
            "excluded o1 stays pending; included o2 left pending (self-transferred)"
        );
        let links: Vec<_> = snap
            .events
            .iter()
            .filter_map(|e| match &e.payload {
                EventPayload::TransferLink(tl) => Some(tl.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(links.len(), 1, "exactly one TransferLink appended");
        assert_eq!(links[0].out_event, o2);
        assert_eq!(
            links[0].in_event_or_wallet,
            TransferTarget::Wallet(cold.clone())
        );
        assert!(
            snap.state.holdings_by_wallet.contains_key(&cold),
            "the self-transfer relocated o2's lot to the dest wallet (Op::SelfTransfer)"
        );
        assert!(
            app.status
                .as_deref()
                .unwrap_or_default()
                .contains("Linked 1 outflow"),
            "status reports the applied count; got {:?}",
            app.status
        );
    }

    /// E2E: bulk-link BOTH outs, then void ONE bulk-created link via the `v` flow — it voids
    /// cleanly (the out returns to pending; no DecisionConflict).
    #[test]
    fn kat_e2e_bulk_link_then_void() {
        use btctax_core::EventPayload;
        use std::collections::BTreeSet;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "kat-e2e-bulk-void";
        let (o1, o2) = seed_bulk_one_wallet(&vault, &key, pp_str);

        let mut app = open_app(&vault, pp_str);
        // Bulk-link BOTH (no exclude).
        handle_key(&mut app, press(KeyCode::Char('b')));
        handle_key(&mut app, press(KeyCode::Char('n')));
        type_str(&mut app, "self:cold");
        handle_key(&mut app, press(KeyCode::Enter)); // → Filter
        handle_key(&mut app, press(KeyCode::Enter)); // → Preview
        handle_key(&mut app, press(KeyCode::Enter)); // → modal
        handle_key(&mut app, press(KeyCode::Enter)); // APPLY
        assert!(
            app.snapshot
                .as_ref()
                .unwrap()
                .state
                .pending_reconciliation
                .is_empty(),
            "both outs linked → nothing pending"
        );

        // Find the TransferLink decision id for o1.
        let link_id = {
            let snap = app.snapshot.as_ref().unwrap();
            snap.events
                .iter()
                .find(
                    |e| matches!(&e.payload, EventPayload::TransferLink(tl) if tl.out_event == o1),
                )
                .map(|e| e.id.clone())
                .expect("a TransferLink for o1 must exist")
        };

        // Void it via the `v` flow.
        app.status = None;
        handle_key(&mut app, press(KeyCode::Char('v')));
        assert!(app.void_flow.is_some(), "void flow must open");
        let idx = app
            .void_flow
            .as_ref()
            .unwrap()
            .list
            .items
            .iter()
            .position(|it| it.event_id == link_id)
            .expect("the bulk link must be listed as a revocable decision");
        for _ in 0..idx {
            handle_key(&mut app, press(KeyCode::Down));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // → void modal
        assert!(app.void_modal.is_some(), "void modal must open");
        handle_key(&mut app, press(KeyCode::Enter)); // confirm void
        assert!(app.void_flow.is_none(), "void flow closes after confirm");

        // After void: o1 returns to pending; o2 stays self-transferred; the void is clean.
        let snap = app.snapshot.as_ref().unwrap();
        let pend: BTreeSet<_> = snap
            .state
            .pending_reconciliation
            .iter()
            .map(|p| p.event.clone())
            .collect();
        assert!(pend.contains(&o1), "voided link → o1 returns to pending");
        assert!(!pend.contains(&o2), "o2 remains self-transferred");
        assert!(
            snap.state
                .blockers
                .iter()
                .all(|b| b.kind != BlockerKind::DecisionConflict),
            "voiding a bulk link is clean (no DecisionConflict)"
        );
    }
}
