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
/// Dispatches on `app.screen` FIRST so that `Screen::Unlock` gets full text-input priority:
/// `q` and other printable chars are appended to the passphrase buffer; only `Esc` quits.
/// This means passphrases containing `q`, `t`, or any other letter/digit/symbol work correctly.
///
/// # Screen dispatch
/// - **Unlock**: `Esc` → quit; `Tab`/`BackTab` → ignored (no tab bar on this screen);
///   `Enter` → attempt open; `Backspace` → pop last char;
///   any `Char` (including `q`) → append to passphrase buffer.
/// - **Locked**: `r` → retry (back to Unlock); `q`/`Esc` → quit.
/// - **Viewer**: `q`/`Esc` → quit; `Tab` → next tab; `BackTab` → prev tab
///   (full tab keybindings added in later tasks).
fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Screen dispatch FIRST — so Unlock never accidentally fires global quit/tab keys.
    match app.screen {
        Screen::Unlock => match key.code {
            // Only Esc quits from the Unlock screen — 'q' and all other chars go to buffer.
            KeyCode::Esc => app.should_quit = true,
            // Tab / BackTab are ignored: no tab bar on Unlock; must not cycle or consume.
            KeyCode::Tab | KeyCode::BackTab => {}
            // Enter submits the passphrase buffer.
            KeyCode::Enter => app.do_unlock(),
            // Backspace removes the last character.
            KeyCode::Backspace => app.unlock.pop_char(),
            // ALL printable chars — including 'q' and every letter/digit/symbol — go to buffer.
            KeyCode::Char(c) => {
                // Clear the previous error when the user starts typing again.
                app.unlock.error = None;
                app.unlock.push_char(c);
            }
            _ => {}
        },
        Screen::Locked => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('r') => {
                // Retry: return to Unlock screen.
                app.screen = Screen::Unlock;
                app.unlock.error = None;
            }
            _ => {}
        },
        Screen::Viewer => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            KeyCode::Tab => app.tab = app.tab.next(),
            KeyCode::BackTab => app.tab = app.tab.prev(),
            _ => {} // additional viewer keys added in Tasks 3–4
        },
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
        // 'q' quits on the Viewer screen (regression guard for global quit on non-Unlock screens).
        let mut app = new_app();
        app.screen = Screen::Viewer;
        assert!(!app.should_quit);
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn esc_sets_should_quit() {
        // Esc quits on both Unlock and Viewer; the Unlock screen is the default.
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
        // Tab cycles tabs on Screen::Viewer (regression guard — Tab is ignored on Unlock).
        let mut app = new_app();
        app.screen = Screen::Viewer;
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
        // BackTab cycles tabs on Screen::Viewer (regression guard — BackTab is ignored on Unlock).
        let mut app = new_app();
        app.screen = Screen::Viewer;
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

    // ── handle_key: Unlock screen — full text-input priority ─────────────────
    //
    // The bug was: global 'q' / Tab / BackTab fired BEFORE screen dispatch, so on Unlock
    // pressing 'q' quit the app and Tab cycled tabs instead of going into the passphrase.
    // After the fix, screen dispatch is FIRST; Unlock gets all printable chars.

    #[test]
    fn q_on_unlock_screen_appends_to_buffer_not_quit() {
        let mut app = new_app();
        // Screen::Unlock is the default — 'q' must go to the passphrase buffer.
        assert_eq!(app.screen, Screen::Unlock);
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(
            !app.should_quit,
            "'q' on Unlock must NOT quit (only Esc quits from Unlock)"
        );
        assert_eq!(
            app.unlock.buffer.chars().count(),
            1,
            "'q' on Unlock must be appended to the passphrase buffer"
        );
    }

    #[test]
    fn char_input_on_unlock_screen_appends_various_chars_including_q() {
        let mut app = new_app();
        // All of these are valid passphrase characters; none should quit or be swallowed.
        for c in ['q', 'a', '1', '!', 'z', 'Q'] {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        assert_eq!(app.unlock.buffer.chars().count(), 6);
        assert!(!app.should_quit, "no char key must quit from Unlock screen");
    }

    #[test]
    fn esc_on_unlock_screen_quits() {
        let mut app = new_app();
        assert_eq!(app.screen, Screen::Unlock);
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(app.should_quit, "Esc must quit from Unlock screen");
    }

    #[test]
    fn tab_on_unlock_screen_is_ignored() {
        let mut app = new_app();
        let initial_tab = app.tab;
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(
            app.tab, initial_tab,
            "Tab on Unlock must not cycle tabs (no tab bar on Unlock)"
        );
        assert!(
            app.unlock.buffer.is_empty(),
            "Tab on Unlock must not append to the passphrase buffer"
        );
        assert!(!app.should_quit, "Tab on Unlock must not quit");
    }

    #[test]
    fn backtab_on_unlock_screen_is_ignored() {
        let mut app = new_app();
        let initial_tab = app.tab;
        handle_key(&mut app, press(KeyCode::BackTab));
        assert_eq!(
            app.tab, initial_tab,
            "BackTab on Unlock must not cycle tabs"
        );
        assert!(
            app.unlock.buffer.is_empty(),
            "BackTab must not touch the buffer"
        );
        assert!(!app.should_quit, "BackTab on Unlock must not quit");
    }

    #[test]
    fn enter_on_unlock_screen_calls_do_unlock_clears_buffer() {
        // Use a well-formed but nonexistent vault path (PathBuf::new() has no file component
        // and triggers a debug_assert in btctax-store's paths.rs sidecar-key computation).
        let mut app = App::new(PathBuf::from("/nonexistent/vault.pgp"));
        app.unlock.push_char('p');
        app.unlock.push_char('a');
        app.unlock.push_char('s');
        assert_eq!(app.unlock.buffer.len(), 3);
        handle_key(&mut app, press(KeyCode::Enter));
        // do_unlock consumed buffer via mem::take — buffer must be empty regardless of outcome.
        assert!(
            app.unlock.buffer.is_empty(),
            "Enter on Unlock must call do_unlock (buffer emptied by mem::take)"
        );
        // Vault not found → error is set and app does NOT quit.
        assert!(!app.should_quit, "Enter on Unlock must not quit the app");
        assert!(
            app.unlock.error.is_some(),
            "Enter on Unlock with nonexistent vault must set an error"
        );
    }

    // ── handle_key: Viewer / Locked regression guards ────────────────────────

    #[test]
    fn q_on_viewer_screen_quits() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        handle_key(&mut app, press(KeyCode::Char('q')));
        assert!(app.should_quit, "'q' on Viewer must still quit");
    }

    #[test]
    fn tab_on_viewer_screen_cycles_forward() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        assert_eq!(app.tab, Tab::Holdings);
        handle_key(&mut app, press(KeyCode::Tab));
        assert_eq!(
            app.tab,
            Tab::Disposals,
            "Tab on Viewer must cycle to next tab"
        );
    }
}
