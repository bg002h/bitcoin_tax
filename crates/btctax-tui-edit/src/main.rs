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
/// **Dispatch order** (established here; Task 3 inserts modal → form before screen):
/// 1. [Task 3: mutation-modal dispatch — BEFORE screen dispatch]
/// 2. [Task 3: form dispatch — BEFORE screen dispatch]
/// 3. Screen dispatch (Unlock / Locked / Browse)
///
/// # Screen dispatch
/// - **Unlock**: `Esc` → quit; `Tab`/`BackTab` → ignored (no tab bar); `Enter` →
///   attempt open; `Backspace` → pop char; any `Char` → append to buffer.
/// - **Locked**: `r` → retry (back to Unlock); `q`/`Esc` → quit.
/// - **Browse**: `q`/`Esc` → quit; `Tab` → next tab; `BackTab` → prev tab;
///   `←/→` → year change + reset selections; `↑/↓ j/k` → scroll;
///   `PgUp/PgDn` → page; `g/G` → top/bottom.
///   (`p` tax-profile form wired in Task 3.)
pub fn handle_key(app: &mut EditorApp, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // ── [Task 3: mutation-modal dispatch here — BEFORE screen dispatch] ───────
    // ── [Task 3: form dispatch here — BEFORE screen dispatch] ────────────────

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
            // Clear status on any key press (Task 3 ensures modal keys never reach here).
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
                // `p` (tax-profile edit form) is wired in Task 3; stub: no-op.
                _ => {}
            }
        }
    }
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

    // ── KAT-U1 — unlock parity ───────────────────────────────────────────────
    //
    // Verifies editor unlock paths match the viewer's (single-sourced via open_session):
    //   correct passphrase → Browse + session + snapshot
    //   wrong passphrase   → Unlock + error + no session
    //   locked vault       → Locked + no session

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

        assert_eq!(
            app.screen,
            EditorScreen::Unlock,
            "wrong passphrase must stay on Unlock"
        );
        assert_eq!(
            app.unlock.error.as_deref(),
            Some("incorrect passphrase"),
            "error must be set to 'incorrect passphrase'"
        );
        assert!(
            app.session.is_none(),
            "session must be None on wrong passphrase"
        );
        assert!(
            app.snapshot.is_none(),
            "snapshot must be None on wrong passphrase"
        );
        assert!(
            app.unlock.buffer.is_empty(),
            "buffer must be cleared after failed unlock"
        );
    }

    #[test]
    fn kat_u1_locked_vault_transitions_to_locked_screen_with_no_session() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp_str = "u1-lock-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key).unwrap();

        // Hold the vault lock via a raw Session::open (test-region exception for cmd::init::run;
        // here we use Session::open which is NOT a vault-creating constructor).
        let _holder = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();

        let mut app = EditorApp::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(
            app.screen,
            EditorScreen::Locked,
            "locked vault must show Locked screen"
        );
        assert!(
            app.session.is_none(),
            "session must be None when vault is locked by another holder"
        );
    }

    // ── Lock-exclusivity KAT ─────────────────────────────────────────────────
    //
    // While the editor holds its live session (and thus the VaultLock), a second
    // attempt to open the same vault via open_session returns SessionOpenOutcome::Locked.

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
        assert_eq!(
            app.screen,
            EditorScreen::Browse,
            "first open must succeed and transition to Browse"
        );
        assert!(
            app.session.is_some(),
            "editor must hold the session (VaultLock)"
        );

        // Second attempt while the editor holds the session → Locked
        let outcome2 = open_session(&vault, Passphrase::new(pp_str.into()));
        assert!(
            matches!(outcome2, SessionOpenOutcome::Locked),
            "second open while editor holds the session must return Locked"
        );
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

    // ── Browse tabs smoke test (TestBackend) ──────────────────────────────────
    //
    // Renders all six tabs via the viewer's App-free render fns with a minimal
    // Snapshot; asserts no panic and that the [EDITOR] marker is present.

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

        // Smoke all six tabs — none must panic.
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
        assert_eq!(
            app.unlock.buffer.len(),
            1,
            "'q' on Unlock must go to the passphrase buffer"
        );
    }

    #[test]
    fn tab_on_browse_cycles_forward() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        assert_eq!(app.tab, Tab::Holdings);
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(
            app.tab,
            Tab::Disposals,
            "Tab on Browse must cycle to next tab"
        );
    }

    #[test]
    fn backtab_on_browse_cycles_backward() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        assert_eq!(app.tab, Tab::Holdings);
        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(
            app.tab,
            Tab::Compliance,
            "BackTab on Browse must wrap to last tab"
        );
    }

    #[test]
    fn r_on_locked_returns_to_unlock() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Locked;
        handle_key(&mut app, press(KeyCode::Char('r')));
        assert_eq!(
            app.screen,
            EditorScreen::Unlock,
            "r on Locked must go to Unlock"
        );
    }

    #[test]
    fn tab_on_unlock_is_ignored() {
        let mut app = EditorApp::new(PathBuf::new());
        let initial_tab = app.tab;
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, initial_tab, "Tab on Unlock must not cycle tabs");
        assert!(
            app.unlock.buffer.is_empty(),
            "Tab on Unlock must not touch the passphrase buffer"
        );
    }

    #[test]
    fn key_release_is_ignored() {
        let mut app = EditorApp::new(PathBuf::new());
        let mut release_q = press(KeyCode::Char('q'));
        release_q.kind = KeyEventKind::Release;
        handle_key(&mut app, release_q);
        assert!(!app.should_quit, "key release must not trigger dispatch");
    }

    #[test]
    fn left_right_on_browse_changes_selected_year() {
        let mut app = EditorApp::new(PathBuf::new());
        app.screen = EditorScreen::Browse;
        let initial = app.selected_year;

        handle_key(&mut app, press(KeyCode::Left));
        assert_eq!(app.selected_year, initial - 1, "Left must decrement year");

        handle_key(&mut app, press(KeyCode::Right));
        assert_eq!(app.selected_year, initial, "Right must increment year back");
    }
}
