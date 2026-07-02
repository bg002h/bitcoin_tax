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
use edit::form::{cycle_filing_status, validate, MutationModalState, ProfileFormState};
use editor::{EditorApp, EditorScreen};
use ratatui::{backend::CrosstermBackend, widgets::TableState, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

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
/// **Dispatch order** (modal → form → screen — the R0-M4 lesson: Esc must never
/// fall through a modal to a quit arm):
/// 1. Mutation-modal dispatch — BEFORE form and screen dispatch.
/// 2. Form dispatch — BEFORE screen dispatch.
/// 3. Screen dispatch (Unlock / Locked / Browse).
///
/// # Screen dispatch
/// - **Unlock**: `Esc` → quit; `Tab`/`BackTab` → ignored (no tab bar); `Enter` →
///   attempt open; `Backspace` → pop char; any `Char` → append to buffer.
/// - **Locked**: `r` → retry (back to Unlock); `q`/`Esc` → quit.
/// - **Browse**: `q`/`Esc` → quit; `Tab` → next tab; `BackTab` → prev tab;
///   `←/→` → year change + reset selections; `↑/↓ j/k` → scroll;
///   `PgUp/PgDn` → page; `g/G` → top/bottom; `p` → tax-profile form.
pub fn handle_key(app: &mut EditorApp, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // ── 1. Mutation-modal dispatch — BEFORE form and screen dispatch ───────────
    if app.mutation_modal.is_some() {
        handle_modal_key(app, key);
        return;
    }

    // ── 2. Form dispatch — BEFORE screen dispatch ─────────────────────────────
    if app.profile_form.is_some() {
        handle_form_key(app, key);
        return;
    }

    // ── 3. Screen dispatch ────────────────────────────────────────────────────
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
}
