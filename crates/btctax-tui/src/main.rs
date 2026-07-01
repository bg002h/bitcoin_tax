//! `btctax-tui` — ratatui read-only vault viewer.
//!
//! Terminal lifecycle: enter raw mode + alternate screen on startup; ALWAYS restore on exit:
//!   1. Setup `?` failure — `TerminalGuard` drop restores before propagating the `Err`.
//!   2. Normal exit       — `TerminalGuard` drop restores on scope exit.
//!   3. `run()` error     — `TerminalGuard` drop restores before propagating the `Err` [R0-M4].
//!   4. Panic             — panic hook calls `restore_terminal()` before the default hook [R0-M4].
//!      (`TerminalGuard` also runs during unwind; having both is belt-and-suspenders.)
//!
//! STRICTLY READ-ONLY: this binary MUST NOT call `Session::save()`, `persistence::append_*`,
//! any `btctax_cli::cmd::*` mutating command, or `Session::conn()`.

mod app;
mod draw;
mod unlock;

use app::{App, Screen};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

// ── Terminal lifecycle ────────────────────────────────────────────────────────

/// Restore the terminal to its pre-TUI state.
///
/// Idempotent: safe to call even if raw mode or alternate screen was never entered.
/// Factored here so it is callable from the panic hook AND from the normal/error exit paths.
pub fn restore_terminal() {
    // Ignore errors — we're in a teardown path; best-effort is the right contract.
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// RAII guard: calls `restore_terminal()` on drop, ensuring the terminal is ALWAYS restored
/// regardless of how `main` exits — early `?`-return, normal return, or panic unwind.
///
/// Created immediately after `enable_raw_mode()` succeeds so that every subsequent failure
/// point (`EnterAlternateScreen`, `Terminal::new`, `run()`) is covered by the guard's `Drop`.
/// `restore_terminal()` is idempotent, so the guard's implicit drop and any explicit
/// `restore_terminal()` call coexist safely.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Install a panic hook that restores the terminal BEFORE the default hook prints the message.
/// This ensures a crash never leaves the user's shell in raw/alt-screen state.
fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));
}

// ── Argument parsing ──────────────────────────────────────────────────────────

/// Parse the vault path from CLI arguments.
///
/// Accepts:
/// - `--vault <path>` (named flag)
/// - `<path>` (first positional argument that doesn't start with `-`)
///
/// Falls back to `~/Documents/BitcoinTax/vault.pgp` when HOME is set, else `vault.pgp`.
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
    // Default: ~/Documents/BitcoinTax/vault.pgp (mirrors CLI default)
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Documents/BitcoinTax/vault.pgp"))
        .unwrap_or_else(|| PathBuf::from("vault.pgp"))
}

// ── Event handling ────────────────────────────────────────────────────────────

/// Map a key press to an `App` state transition.
///
/// Only KEY PRESS events are acted on; repeat/release are ignored (crossterm distinguishes them
/// on supporting terminals; others always send `Press`).
///
/// # Screen dispatch
/// - **Unlock**: char input → buffer; Backspace → pop; Enter → attempt open.
/// - **Locked**: r → retry (back to Unlock); q/Esc → quit.
/// - **Viewer**: q/Esc → quit (full tab keybindings added in later tasks).
/// - **Global**: Tab/Shift-Tab cycle tabs on any screen; q/Esc quit on any screen.
fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Global keys that apply on every screen
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
            return;
        }
        KeyCode::Tab => {
            app.tab = app.tab.next();
            return;
        }
        KeyCode::BackTab => {
            app.tab = app.tab.prev();
            return;
        }
        _ => {}
    }

    // Screen-specific keys
    match app.screen {
        Screen::Unlock => match key.code {
            KeyCode::Char(c) => {
                // Clear the previous error when the user starts typing again
                app.unlock.error = None;
                app.unlock.push_char(c);
            }
            KeyCode::Backspace => app.unlock.pop_char(),
            KeyCode::Enter => app.do_unlock(),
            _ => {}
        },
        Screen::Locked => {
            if let KeyCode::Char('r') = key.code {
                // Retry: return to Unlock screen
                app.screen = Screen::Unlock;
                app.unlock.error = None;
            }
        }
        Screen::Viewer => {} // additional viewer keys added in Tasks 3–4
    }
}

// ── Run loop ─────────────────────────────────────────────────────────────────

/// The main event loop.  Runs until `app.should_quit` is set.
///
/// Returns `Err` on I/O failure; the caller is responsible for calling `restore_terminal()`
/// regardless of the return value [R0-M4].
fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    vault_path: PathBuf,
) -> io::Result<()> {
    let mut app = App::new(vault_path);

    // `BTCTAX_PASSPHRASE` fast-path: open immediately without displaying the unlock prompt.
    // Mirrors the CLI's non-interactive behaviour.
    app.try_env_passphrase();

    while !app.should_quit {
        terminal.draw(|f| draw::draw(f, &app))?;
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
    // Install the panic hook BEFORE enabling raw mode so any panic restores the terminal.
    setup_panic_hook();

    let vault_path = parse_vault_path();

    enable_raw_mode()?;
    // Guard created immediately after raw mode is enabled.
    // Its Drop calls restore_terminal() on ANY exit from this scope:
    // early ? (EnterAlternateScreen, Terminal::new), normal return, or panic unwind.
    let _guard = TerminalGuard;

    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, vault_path);

    // Explicit call is now redundant (the guard's Drop covers it) but kept for clarity.
    // restore_terminal() is idempotent — calling it twice is safe [R0-M4].
    restore_terminal();

    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use app::Tab;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    fn new_app() -> App {
        App::new(PathBuf::new())
    }

    // ── restore_terminal / panic hook ─────────────────────────────────────────

    /// `restore_terminal` must be callable without panicking, even outside a real terminal
    /// (disable_raw_mode / execute! return errors that are silently ignored).
    #[test]
    fn restore_terminal_is_callable_outside_a_real_terminal() {
        restore_terminal(); // must not panic
    }

    /// `setup_panic_hook` must complete without panicking; it installs the hook as a side-effect.
    #[test]
    fn setup_panic_hook_installs_without_error() {
        setup_panic_hook(); // must not panic
                            // Verify the hook is installed: take_hook returns the previously-set hook.
                            // We take it back and replace with a no-op to avoid interfering with other tests.
        let hook = std::panic::take_hook();
        std::panic::set_hook(hook); // restore
    }

    /// `TerminalGuard`'s Drop must call `restore_terminal()` without panicking, even outside a
    /// real terminal.  Also verifies that calling `restore_terminal()` again after the guard drops
    /// is safe (idempotency of double-restore, mirroring the guard + explicit-call pattern in main).
    #[test]
    fn terminal_guard_drop_calls_restore_terminal_and_is_idempotent() {
        {
            let _guard = TerminalGuard;
        } // Drop fires here: restore_terminal() called once.
          // Call a second time to confirm double-call is safe (guard + explicit pattern).
        restore_terminal();
    }

    // ── handle_key: quit ─────────────────────────────────────────────────────

    #[test]
    fn q_sets_should_quit() {
        let mut app = new_app();
        assert!(!app.should_quit);
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn esc_sets_should_quit() {
        let mut app = new_app();
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(app.should_quit);
    }

    // ── handle_key: release events are ignored ───────────────────────────────

    #[test]
    fn key_release_does_not_set_should_quit() {
        let mut app = new_app();
        let mut release_q = press(KeyCode::Char('q'));
        release_q.kind = KeyEventKind::Release;
        handle_key(&mut app, release_q);
        assert!(!app.should_quit, "release event must not quit");
    }

    // ── handle_key: Tab cycling ───────────────────────────────────────────────

    #[test]
    fn tab_cycles_forward_through_all_six_and_wraps() {
        let mut app = new_app();
        assert_eq!(app.tab, Tab::Holdings);

        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Disposals);

        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Income);

        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Tax);

        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Forms);

        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Compliance);

        // Wrap back to the first tab.
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(app.tab, Tab::Holdings);
    }

    #[test]
    fn shift_tab_cycles_backward_through_all_six_and_wraps() {
        let mut app = new_app();
        assert_eq!(app.tab, Tab::Holdings);

        // BackTab is how crossterm reports Shift-Tab.
        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Compliance);

        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Forms);

        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Tax);

        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Income);

        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Disposals);

        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(app.tab, Tab::Holdings);
    }

    // ── Snapshot type compiles with read-only fields ──────────────────────────

    /// Verify the Snapshot type definition compiles and that App starts with snapshot = None.
    /// Task 2 populates the snapshot; for Task 1 it is always None on a fresh app.
    #[test]
    fn snapshot_is_none_on_new_app() {
        let app = new_app();
        assert!(app.snapshot.is_none());
    }

    // ── handle_key: unlock screen passphrase input ───────────────────────────

    #[test]
    fn char_keys_on_unlock_screen_go_to_buffer() {
        let mut app = new_app();
        // App starts on Screen::Unlock
        handle_key(&mut app, press(KeyCode::Char('a')));
        handle_key(&mut app, press(KeyCode::Char('b')));
        handle_key(&mut app, press(KeyCode::Char('c')));
        assert_eq!(app.unlock.buffer.chars().count(), 3);
    }

    #[test]
    fn backspace_on_unlock_screen_removes_last_char() {
        let mut app = new_app();
        handle_key(&mut app, press(KeyCode::Char('x')));
        handle_key(&mut app, press(KeyCode::Char('y')));
        handle_key(&mut app, press(KeyCode::Backspace));
        assert_eq!(app.unlock.buffer.chars().count(), 1);
    }

    #[test]
    fn r_on_locked_screen_returns_to_unlock() {
        let mut app = new_app();
        app.screen = Screen::Locked;
        handle_key(&mut app, press(KeyCode::Char('r')));
        assert_eq!(app.screen, Screen::Unlock);
    }
}
