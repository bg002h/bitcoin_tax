//! `btctax-tui` — ratatui vault viewer with owner-only form-CSV export.
//!
//! This crate provides both a binary (`btctax-tui`) and a library.  The library
//! exposes the viewer's reusable read-only surface so that the sibling editor crate
//! (`btctax-tui-edit`) can share the unlock flow, `Snapshot`, and tab renderers
//! without duplicating code or types.
//!
//! # Public surface (Task 1 / D1)
//! - `app::{Screen, Tab, Snapshot}` — viewer navigation state and read-only projection.
//! - `unlock::{UnlockState, PASSPHRASE_CAP, attempt_open, OpenOutcome,
//!             open_session, SessionOpenOutcome, build_snapshot, latest_year}`
//! - `draw::draw_unlock_screen` — App-free unlock-screen renderer.
//! - `tabs::{holdings,disposals,income,tax,forms,compliance}::render` — App-free tab renderers.
//! - `restore_terminal`, `TerminalGuard`, `setup_panic_hook` — terminal lifecycle.
//! - `run_viewer` — entry point for the viewer binary.
//!
//! # Internal surface (NOT reachable from the editor)
//! - `app::App`, `export::ExportConfirmState`, `draw::draw`, `handle_key` — viewer internals.
//!
//! Terminal lifecycle: enter raw mode + alternate screen on startup; ALWAYS restore on exit:
//!   1. Setup `?` failure — `TerminalGuard` drop restores before propagating the `Err`.
//!   2. Normal exit       — `TerminalGuard` drop restores on scope exit.
//!   3. `run()` error     — `TerminalGuard` drop restores before propagating the `Err` [R0-M4].
//!   4. Panic             — panic hook calls `restore_terminal()` before the default hook [R0-M4].
//!      (`TerminalGuard` also runs during unwind; having both is belt-and-suspenders.)
//!
//! never writes the vault or any decrypted image of it; writes only the four form CSVs
//! via `export.rs` on explicit user confirmation. This module performs no writes.

// ── Module declarations ───────────────────────────────────────────────────────

/// Application state: `Screen`, `Tab`, `Snapshot` (pub), `App` (pub(crate)).
pub mod app;
pub mod clock;
/// Terminal rendering (pub for `draw_unlock_screen`; `draw::draw` is pub(crate)).
pub mod draw;
/// Form CSV export (crate-internal; `ExportConfirmState` not in external surface).
pub(crate) mod export;
/// Shared display-only column-sort primitives (`Dir`/`ViewSort`/cursor helpers + `stable_sort_by`).
pub mod sort;
/// Per-tab renderers: each tab exposes a `pub fn render` + a `pub(crate) fn draw` wrapper.
pub mod tabs;
/// Vault-open logic and unlock screen state.
pub mod unlock;
/// The read-only what-if planner panel (`w`): reuses `btctax_core::whatif::{sell,harvest}`.
pub mod whatif_panel;

use app::{App, Screen, Tab};
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
/// regardless of how the entry point exits — early `?`-return, normal return, or panic unwind.
///
/// Created immediately after `enable_raw_mode()` succeeds so that every subsequent failure
/// point (`EnterAlternateScreen`, `Terminal::new`, `run()`) is covered by the guard's `Drop`.
/// `restore_terminal()` is idempotent, so the guard's implicit drop and any explicit
/// `restore_terminal()` call coexist safely.
pub struct TerminalGuard;

impl TerminalGuard {
    /// Create a new `TerminalGuard`.
    ///
    /// Equivalent to `TerminalGuard` (unit-struct literal), provided as a named
    /// constructor for clarity in calling code.
    pub fn new() -> Self {
        TerminalGuard
    }
}

impl Default for TerminalGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Install a panic hook that restores the terminal BEFORE the default hook prints the message.
/// This ensures a crash never leaves the user's shell in raw/alt-screen state.
pub fn setup_panic_hook() {
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
/// **Modal priority [R0-M4]:** When `app.export_modal` is `Some`, the modal dispatch runs
/// FIRST and consumes the key entirely (returning early). This prevents `Esc` from reaching
/// the Viewer arm (which currently quits the app) while the modal is open.
///
/// Dispatches on `app.screen` FIRST (after modal) so that `Screen::Unlock` gets full
/// text-input priority: `q` and other printable chars are appended to the passphrase buffer;
/// only `Esc` quits. This means passphrases containing `q`, `t`, or any other
/// letter/digit/symbol work correctly.
///
/// # Screen dispatch
/// - **Unlock**: `Esc` → quit; `Tab`/`BackTab` → ignored (no tab bar on this screen);
///   `Enter` → attempt open; `Backspace` → pop last char;
///   any `Char` (including `q`) → append to passphrase buffer.
/// - **Locked**: `r` → retry (back to Unlock); `q`/`Esc` → quit.
/// - **Viewer**: `q`/`Esc` → quit; `Tab` → next tab; `BackTab` → prev tab;
///   `e` → open export confirmation modal (no-op if no snapshot).
pub(crate) fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // ── Modal dispatch — BEFORE screen dispatch [R0-M4] ──────────────────────
    // When the export confirmation modal is open it consumes the key entirely (the modal is
    // blocking; 'q' never quits while it is open). Two modes, decided at modal-open time:
    //   • plain confirm (not pseudo-active): only Enter/Esc are acted on;
    //   • typed-word attest (pseudo-active) [R0-C1]: printable chars edit the phrase buffer.
    if app.export_modal.is_some() {
        handle_export_modal_key(app, key);
        return; // Modal consumed the key — skip screen dispatch.
    }

    // ── What-if panel dispatch — panel-first while open [whatif P3 / R0-M3] ───
    // The read-only planner overlay takes focus + gets keys FIRST (like the export modal): printable
    // chars edit its sub-fields, so 'q' never quits while it is open. Esc closes it. It NEVER writes.
    if app.whatif.is_some() {
        handle_whatif_key(app, key);
        return; // Panel consumed the key — skip screen dispatch.
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
        Screen::Viewer => {
            // Clear export status on any non-modal key press [D4 footer spec].
            app.export_status = None;
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
                // Column cursor: h/← move left, l/→ move right (was tax year — the year MOVED to
                // the [ / ] keys below so s/h/l are free for sorting).
                KeyCode::Left | KeyCode::Char('h') => move_cursor_left(app),
                KeyCode::Right | KeyCode::Char('l') => move_cursor_right(app),
                // Sort the focused column: toggle asc↔desc; focusing a NEW column sorts ascending.
                KeyCode::Char('s') => sort_focused(app),
                // Tax year prev / next. Holdings is NOT year-scoped, so this is a no-op there
                // (year still steps on Disposals/Income) [R0-N-2].
                KeyCode::Char('[') => {
                    app.selected_year -= 1;
                    reset_selections(app);
                }
                KeyCode::Char(']') => {
                    app.selected_year += 1;
                    reset_selections(app);
                }
                // [whatif P3] Open the read-only what-if planner overlay. No-op with no snapshot
                // [M5 whatif_panel_w_noop_before_snapshot] — the panel needs the read-only snapshot.
                KeyCode::Char('w') => {
                    if let Some(snap) = app.snapshot.as_ref() {
                        let now = app.clock.now();
                        app.whatif =
                            Some(whatif_panel::WhatIfPanel::new(snap, app.selected_year, now));
                    }
                }
                // [D4] Export keybinding: open the confirmation modal.
                // No-op when no snapshot is loaded [KAT-E8].
                KeyCode::Char('e') => {
                    if let Some(snap) = app.snapshot.as_ref() {
                        let export_now = app.clock.now();
                        let out_dir = export::export_dir_for(&app.vault_path, export_now);
                        let files = export::compute_files(snap, app.selected_year);
                        // [sub-3 / R0-C1] When the ledger is PSEUDO-ACTIVE the modal becomes a
                        // typed-word attest gate (the exact ATTEST_PHRASE required before export);
                        // a fully-real ledger gets today's plain Enter/Esc confirm.
                        let attest = if snap.state.pseudo_active() {
                            Some(export::AttestInput::default())
                        } else {
                            None
                        };
                        app.export_modal = Some(export::ExportConfirmState {
                            year: app.selected_year,
                            out_dir,
                            files,
                            export_now,
                            attest,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

// ── Export modal dispatch ──────────────────────────────────────────────────────

/// Dispatch a key while the export confirmation modal is open.
///
/// Two modes, decided at modal-open time from `snap.state.pseudo_active()`:
///  - **plain confirm** (`attest == None`, not pseudo-active): today's behavior — `Enter` runs the
///    export, `Esc` cancels, every other key is swallowed.
///  - **typed-word attest** (`attest == Some`, pseudo-active) [R0-C1]: the user must type the exact
///    `btctax_cli::ATTEST_PHRASE` before the export runs. Printable chars / `Backspace` edit the
///    buffer (clearing any prior error); `Enter` validates via the shared `require_attestation`
///    exact-compare — a wrong phrase sets an error and does NOT export (buffer PRESERVED); `Esc`
///    cancels. Mirrors tui-edit's SafeHarborAttest TypedWord gate.
fn handle_export_modal_key(app: &mut App, key: KeyEvent) {
    let is_attest = app
        .export_modal
        .as_ref()
        .is_some_and(|m| m.attest.is_some());

    if !is_attest {
        // Plain confirm (not pseudo-active).
        match key.code {
            KeyCode::Enter => run_export(app),
            KeyCode::Esc => app.export_modal = None, // cancel — writes nothing; does NOT quit.
            _ => {}                                  // swallowed ('q' does not quit).
        }
        return;
    }

    // Typed-word attest (pseudo-active) [R0-C1].
    match key.code {
        KeyCode::Char(c) => {
            if let Some(a) = app.export_modal.as_mut().and_then(|m| m.attest.as_mut()) {
                a.buf.push(c);
                a.error = None;
            }
        }
        KeyCode::Backspace => {
            if let Some(a) = app.export_modal.as_mut().and_then(|m| m.attest.as_mut()) {
                a.buf.pop();
                a.error = None;
            }
        }
        KeyCode::Esc => app.export_modal = None, // cancel — writes nothing.
        KeyCode::Enter => {
            let typed = app
                .export_modal
                .as_ref()
                .and_then(|m| m.attest.as_ref())
                .map(|a| a.buf.clone())
                .unwrap_or_default();
            // Shared exact-compare (trimmed, case-sensitive) — the SAME gate the CLI uses.
            if btctax_cli::require_attestation(Some(&typed)).is_ok() {
                run_export(app);
            } else if let Some(a) = app.export_modal.as_mut().and_then(|m| m.attest.as_mut()) {
                // Wrong phrase: NO export; keep the modal open, PRESERVE the buffer, show the error.
                a.error = Some(format!(
                    "type '{}' exactly to attest and export",
                    btctax_cli::ATTEST_PHRASE
                ));
            }
        }
        _ => {} // swallowed.
    }
}

/// Take the export modal and run the export, recording the one-line status. Shared by both modal
/// modes (plain confirm and the attested typed-word path). Clears `export_modal` (takes it).
fn run_export(app: &mut App) {
    let Some(modal) = app.export_modal.take() else {
        return;
    };
    if let Some(snap) = app.snapshot.as_ref() {
        match export::do_export(snap, &modal) {
            Ok(dir) => {
                app.export_status = Some(format!("Exported to {}", dir.display()));
            }
            Err(e) => {
                app.export_status = Some(format!("Export error: {e}"));
            }
        }
    }
}

// ── What-if panel dispatch ──────────────────────────────────────────────────────

/// Dispatch a key while the read-only what-if planner overlay is open [whatif P3].
///
/// Panel-first: printable chars edit the focused sub-field (so `q` never quits while it is open).
/// `Esc` closes it; `Tab`/`s`/`h` switch Sell⇄Harvest; `↑`/`↓` (+`BackTab`) move field focus;
/// `←`/`→` cycle the wallet picker; `Enter` computes (EXPLICIT — never per-keystroke [R0-M2]).
///
/// The panel calls ONLY the non-persisting `btctax_core::whatif::{sell,harvest}` and reads `snapshot`;
/// it NEVER writes the vault. `handle_key`'s read-only invariant is preserved: this touches only the
/// `whatif` UI field (+ reads the read-only snapshot on Enter).
fn handle_whatif_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Esc closes the panel — writes nothing, does NOT quit.
        KeyCode::Esc => app.whatif = None,
        // Sell ⇄ Harvest toggle + explicit selectors ('s' Sell / 'h' Harvest).
        KeyCode::Tab => {
            if let Some(p) = app.whatif.as_mut() {
                p.toggle_mode();
            }
        }
        KeyCode::Char('s') => {
            if let Some(p) = app.whatif.as_mut() {
                p.set_mode(whatif_panel::WhatIfMode::Sell);
            }
        }
        KeyCode::Char('h') => {
            if let Some(p) = app.whatif.as_mut() {
                p.set_mode(whatif_panel::WhatIfMode::Harvest);
            }
        }
        // Field focus: ↑/BackTab previous, ↓ next.
        KeyCode::Up | KeyCode::BackTab => {
            if let Some(p) = app.whatif.as_mut() {
                p.focus_prev();
            }
        }
        KeyCode::Down => {
            if let Some(p) = app.whatif.as_mut() {
                p.focus_next();
            }
        }
        // Wallet picker (only cycles when the Wallet field is focused).
        KeyCode::Left => {
            if let Some(p) = app.whatif.as_mut() {
                p.wallet_prev();
            }
        }
        KeyCode::Right => {
            if let Some(p) = app.whatif.as_mut() {
                p.wallet_next();
            }
        }
        // EXPLICIT compute — needs the read-only snapshot + the selected year.
        KeyCode::Enter => {
            if let Some(snap) = app.snapshot.as_ref() {
                if let Some(p) = app.whatif.as_mut() {
                    p.compute(snap, app.selected_year);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(p) = app.whatif.as_mut() {
                p.backspace();
            }
        }
        // All other printable chars edit the focused text field.
        KeyCode::Char(c) => {
            if let Some(p) = app.whatif.as_mut() {
                p.push_char(c);
            }
        }
        _ => {}
    }
}

// ── Column-sort helpers ─────────────────────────────────────────────────────────

/// Return the active `(ViewSort, cursor, column_count)` triple for the currently focused row view.
///
/// Only the three row views (Holdings/Disposals/Income) are sortable; other tabs return `None` and
/// the cursor/sort keys are a no-op there.
fn active_sort(app: &mut App) -> Option<(&mut sort::ViewSort, &mut usize, usize)> {
    match app.tab {
        Tab::Holdings => Some((
            &mut app.holdings_sort,
            &mut app.holdings_cursor,
            tabs::holdings::COLUMN_COUNT,
        )),
        Tab::Disposals => Some((
            &mut app.disposals_sort,
            &mut app.disposals_cursor,
            tabs::disposals::COLUMN_COUNT,
        )),
        Tab::Income => Some((
            &mut app.income_sort,
            &mut app.income_cursor,
            tabs::income::COLUMN_COUNT,
        )),
        _ => None,
    }
}

/// Move the focused row view's column cursor one column left (clamped at 0).
fn move_cursor_left(app: &mut App) {
    if let Some((_, cursor, _)) = active_sort(app) {
        *cursor = sort::cursor_left(*cursor);
    }
}

/// Move the focused row view's column cursor one column right (clamped at the last column).
fn move_cursor_right(app: &mut App) {
    if let Some((_, cursor, count)) = active_sort(app) {
        *cursor = sort::cursor_right(*cursor, count);
    }
}

/// Sort the focused row view by its cursor column (`s`): toggle direction if it is already the sort
/// column, else focus the new column ascending. Display-only — never mutates ledger state.
fn sort_focused(app: &mut App) {
    if let Some((view_sort, cursor, _)) = active_sort(app) {
        let c = *cursor;
        sort::apply_sort_key(view_sort, c);
    }
}

// ── Scroll helpers ────────────────────────────────────────────────────────────

/// Return the active `TableState` reference for the currently focused tab.
///
/// Holdings, Disposals, Income, and Forms tabs have a `TableState`.
/// Other tabs return `None` and scroll is a no-op.
fn active_state(app: &mut App) -> Option<&mut ratatui::widgets::TableState> {
    match app.tab {
        Tab::Holdings => Some(&mut app.holdings_state),
        Tab::Disposals => Some(&mut app.disposals_state),
        Tab::Income => Some(&mut app.income_state),
        Tab::Forms => Some(&mut app.forms_state),
        _ => None,
    }
}

/// Number of **selectable** data rows for the active tab.
///
/// **[Minor B] TOTAL row is excluded** — the TOTAL row is always rendered but NEVER selectable.
/// Returning `lots.len()` (not `lots.len() + 1`) means `go_bottom` caps at index `lots.len() - 1`,
/// which is the last DATA row, not the TOTAL row at `lots.len()`.
fn active_row_count(app: &App) -> usize {
    let Some(snap) = app.snapshot.as_ref() else {
        return 0;
    };
    match app.tab {
        Tab::Holdings => snap.state.lots.len(), // data rows only — TOTAL not selectable
        Tab::Disposals => {
            let yr = app.selected_year;
            snap.state
                .disposals
                .iter()
                .filter(|d| d.disposed_at.year() == yr)
                .map(|d| d.legs.len())
                .sum::<usize>()
            // no +1 for TOTAL
        }
        Tab::Income => {
            let yr = app.selected_year;
            snap.state
                .income_recognized
                .iter()
                .filter(|r| r.recognized_at.year() == yr)
                .count()
            // no +1 for TOTAL
        }
        Tab::Forms => {
            let yr = app.selected_year;
            btctax_core::form_8949(&snap.state, yr).len() // 8949 rows (no TOTAL row in 8949)
        }
        _ => 0,
    }
}

/// Move selection up by 1 row.  No-op when at the top or no rows.
pub(crate) fn scroll_up(app: &mut App) {
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

/// Move selection down by 1 row.  Selects index 0 when nothing is selected.
pub(crate) fn scroll_down(app: &mut App) {
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

/// Move selection up by 10 rows (page up).
fn page_up(app: &mut App) {
    const PAGE: usize = 10;
    let Some(state) = active_state(app) else {
        return;
    };
    let next = state.selected().map(|i| i.saturating_sub(PAGE));
    state.select(next);
}

/// Move selection down by 10 rows (page down).
fn page_down(app: &mut App) {
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

/// Move selection to the first row.
fn go_top(app: &mut App) {
    let count = active_row_count(app);
    if count == 0 {
        return;
    }
    if let Some(state) = active_state(app) {
        state.select(Some(0));
    }
}

/// Move selection to the last row.
fn go_bottom(app: &mut App) {
    let count = active_row_count(app);
    if count == 0 {
        return;
    }
    if let Some(state) = active_state(app) {
        state.select(Some(count - 1));
    }
}

/// Reset all table selections to `None` (e.g. after a year change).
fn reset_selections(app: &mut App) {
    app.holdings_state.select(None);
    app.disposals_state.select(None);
    app.income_state.select(None);
    app.forms_state.select(None);
}

// ── Run loop ─────────────────────────────────────────────────────────────────

/// The main event loop.  Runs until `app.should_quit` is set.
///
/// Returns `Err` on I/O failure; the caller is responsible for calling `restore_terminal()`
/// regardless of the return value [R0-M4].
fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    vault_path: PathBuf,
    clock: clock::Clock,
) -> io::Result<()> {
    let mut app = App::new(vault_path);
    app.clock = clock; // SPEC §3.4 — resolved from BTCTAX_NOW in run_viewer, before raw mode

    // `BTCTAX_PASSPHRASE` fast-path: open immediately without displaying the unlock prompt.
    // Mirrors the CLI's non-interactive behaviour.
    app.try_env_passphrase();

    while !app.should_quit {
        terminal.draw(|f| draw::draw(f, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key);
            }
        }
    }
    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the viewer binary.
///
/// Installs the panic hook, parses the vault path, enters raw/alt-screen mode,
/// runs the event loop, and always restores the terminal on exit.
///
/// Called by `main.rs`'s `fn main()`.
pub fn run_viewer() -> io::Result<()> {
    // Install the panic hook BEFORE enabling raw mode so any panic restores the terminal.
    setup_panic_hook();

    let vault_path = parse_vault_path();

    // Resolve the deterministic clock (SPEC §3.4) BEFORE raw mode: a malformed BTCTAX_NOW exits 2 like the
    // CLI, and an active override prints the stderr banner now (the alt-screen would hide it once raw mode
    // is on). Unset ⇒ Clock::Wall ⇒ byte-identical to the pre-seam behavior.
    let clock = match clock::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    if matches!(clock, clock::Clock::Pinned(_)) {
        eprintln!("{}", clock::OVERRIDE_BANNER);
    }

    enable_raw_mode()?;
    // Guard created immediately after raw mode is enabled.
    // Its Drop calls restore_terminal() on ANY exit from this scope:
    // early ? (EnterAlternateScreen, Terminal::new), normal return, or panic unwind.
    let _guard = TerminalGuard;

    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, vault_path, clock);

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

    /// [R0-M-3] The tax year now steps on `[` / `]` (MOVED off `←`/`→`, which move the column
    /// cursor). Also verifies arrows no longer change the year.
    #[test]
    fn tax_year_moves_to_bracket_keys() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        app.tab = Tab::Disposals; // a year-scoped tab
        let initial_year = app.selected_year;

        handle_key(&mut app, press(KeyCode::Char('[')));
        assert_eq!(
            app.selected_year,
            initial_year - 1,
            "'[' must decrement selected_year"
        );

        handle_key(&mut app, press(KeyCode::Char(']')));
        assert_eq!(
            app.selected_year, initial_year,
            "']' must increment selected_year back"
        );

        // Arrows NO LONGER change the year — they move the column cursor.
        handle_key(&mut app, press(KeyCode::Left));
        handle_key(&mut app, press(KeyCode::Right));
        assert_eq!(
            app.selected_year, initial_year,
            "←/→ must NOT change the year anymore (they move the column cursor)"
        );
    }

    /// The column cursor moves with h/l and ←/→, clamped at both ends per view.
    #[test]
    fn cursor_moves_h_l_and_arrows() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        app.tab = Tab::Holdings;
        // Start at the default sort column (Acquired = col 1).
        assert_eq!(app.holdings_cursor, 1);

        handle_key(&mut app, press(KeyCode::Char('l')));
        assert_eq!(app.holdings_cursor, 2, "'l' moves cursor right");
        handle_key(&mut app, press(KeyCode::Right));
        assert_eq!(app.holdings_cursor, 3, "'→' moves cursor right");
        handle_key(&mut app, press(KeyCode::Char('h')));
        assert_eq!(app.holdings_cursor, 2, "'h' moves cursor left");
        handle_key(&mut app, press(KeyCode::Left));
        assert_eq!(app.holdings_cursor, 1, "'←' moves cursor left");

        // Clamp left at 0.
        for _ in 0..5 {
            handle_key(&mut app, press(KeyCode::Char('h')));
        }
        assert_eq!(app.holdings_cursor, 0, "cursor clamps at 0");
        // Clamp right at COLUMN_COUNT-1 (6 holdings columns → last index 5).
        for _ in 0..20 {
            handle_key(&mut app, press(KeyCode::Char('l')));
        }
        assert_eq!(app.holdings_cursor, 5, "cursor clamps at last column");
    }

    /// `s` toggles the sort direction when re-pressed on the same column; focusing a NEW column
    /// sorts ascending first.
    #[test]
    fn s_toggles_direction_on_repeat() {
        use sort::Dir;
        let mut app = new_app();
        app.screen = Screen::Viewer;
        app.tab = Tab::Holdings;
        // Cursor starts on Acquired (col 1) = the default sort column, ascending.
        assert_eq!(app.holdings_sort, tabs::holdings::DEFAULT_SORT);

        handle_key(&mut app, press(KeyCode::Char('s')));
        assert_eq!(app.holdings_sort.col, 1);
        assert_eq!(app.holdings_sort.dir, Dir::Desc, "same col toggles to desc");
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert_eq!(app.holdings_sort.dir, Dir::Asc, "toggles back to asc");

        // Move to a new column and sort → ascending first.
        handle_key(&mut app, press(KeyCode::Char('l'))); // col 2 (BTC)
        handle_key(&mut app, press(KeyCode::Char('s')));
        assert_eq!(app.holdings_sort.col, 2, "new column becomes the sort key");
        assert_eq!(
            app.holdings_sort.dir,
            Dir::Asc,
            "new column sorts ascending"
        );
    }

    /// Sort state is per-view: sorting on Holdings does not touch Disposals/Income sort state.
    #[test]
    fn sort_state_is_per_view() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        app.tab = Tab::Holdings;
        let disposals_before = app.disposals_sort;
        let income_before = app.income_sort;

        handle_key(&mut app, press(KeyCode::Char('s'))); // toggle Holdings
        handle_key(&mut app, press(KeyCode::Char('l')));
        handle_key(&mut app, press(KeyCode::Char('s')));

        assert_ne!(
            app.holdings_sort,
            tabs::holdings::DEFAULT_SORT,
            "Holdings sort changed"
        );
        assert_eq!(
            app.disposals_sort, disposals_before,
            "Disposals sort untouched"
        );
        assert_eq!(app.income_sort, income_before, "Income sort untouched");
    }

    /// [R0-N-2] Holdings is not year-scoped, so `[`/`]` still step `selected_year` but the view is
    /// unaffected; asserts the key does not panic and the year field still changes (a shared field).
    #[test]
    fn holdings_year_keys_are_noop_on_view() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        app.tab = Tab::Holdings;
        let before = app.selected_year;
        // The key is accepted (year field steps), but Holdings ignores year when rendering — so the
        // holdings sort/cursor state is what matters and remains untouched.
        handle_key(&mut app, press(KeyCode::Char(']')));
        assert_eq!(app.selected_year, before + 1);
        assert_eq!(
            app.holdings_sort,
            tabs::holdings::DEFAULT_SORT,
            "year keys never disturb the Holdings sort"
        );
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

    // ── KAT-E8 — 'e' on Viewer with no snapshot is a no-op ──────────────────

    /// KAT-E8: pressing `e` when `app.snapshot.is_none()` must NOT open the modal.
    #[test]
    fn e8_e_key_no_snapshot_is_noop() {
        let mut app = new_app();
        app.screen = Screen::Viewer;
        // snapshot is None (never unlocked)
        assert!(app.snapshot.is_none());

        handle_key(&mut app, press(KeyCode::Char('e')));

        assert!(
            app.export_modal.is_none(),
            "'e' with no snapshot must be a no-op — export_modal must stay None"
        );
    }

    // ── KAT-E2 — Esc-cancel writes nothing + modal-priority asserts [R0-M4] ──

    /// KAT-E2: Esc closes the modal without writing anything and without quitting.
    /// Additionally verifies that `q` while the modal is open is swallowed.
    #[test]
    fn e2_esc_cancel_writes_nothing_and_q_is_swallowed() {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_core::state::LedgerState;
        use std::collections::BTreeMap;

        let dir = tempfile::tempdir().unwrap();
        let vault_path = dir.path().join("vault.pgp");

        // Build a minimal Snapshot (no income, no profiles — just enough to have a snapshot).
        let snap = app::Snapshot {
            events: vec![],
            state: LedgerState::default(),
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            refused: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
        };

        let mut test_app = App::new(vault_path);
        test_app.screen = Screen::Viewer;
        test_app.selected_year = 2025;
        test_app.snapshot = Some(snap);

        // Press 'e' → modal opens.
        handle_key(&mut test_app, press(KeyCode::Char('e')));
        assert!(
            test_app.export_modal.is_some(),
            "export_modal must be Some after 'e'"
        );

        // Snapshot the expected out_dir before we press any more keys.
        let out_dir = test_app.export_modal.as_ref().unwrap().out_dir.clone();

        // Additional case [R0-M4]: 'q' while modal open → swallowed (no quit, modal stays).
        handle_key(&mut test_app, press(KeyCode::Char('q')));
        assert!(
            !test_app.should_quit,
            "'q' while modal open must NOT quit the app"
        );
        assert!(
            test_app.export_modal.is_some(),
            "modal must still be open after 'q' (key is swallowed)"
        );

        // Press Esc → modal closes, nothing written, no quit.
        handle_key(&mut test_app, press(KeyCode::Esc));
        assert!(
            test_app.export_modal.is_none(),
            "export_modal must be None after Esc"
        );
        assert!(
            !test_app.should_quit,
            "Esc on modal must NOT quit the app [R0-M4]"
        );
        assert!(
            test_app.export_status.is_none(),
            "export_status must be None after cancel (no write occurred)"
        );

        // The output directory must NOT exist (no writes, not even the dir creation).
        assert!(
            !out_dir.exists(),
            "export dir must NOT exist after Esc cancel — no writes occurred"
        );
    }

    // ── KAT-E1 — Confirmation flow (unit, temp vault) ────────────────────────

    /// KAT-E1: full confirm flow — `e` opens modal with correct files, Enter executes
    /// the export, `export_status` contains "Exported to", output dir + CSVs exist.
    #[test]
    fn e1_confirmation_flow_with_se_income() {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_core::{
            event::IncomeKind,
            identity::{EventId, Source, SourceRef},
            state::{IncomeRecord, LedgerState},
            Carryforward, FilingStatus, TaxProfile,
        };
        use btctax_store::Passphrase;
        use rust_decimal::Decimal;
        use std::collections::BTreeMap;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        // Create a real vault so the vault_path's parent exists for the export dir.
        btctax_cli::cmd::init::run(&vault, &Passphrase::new("e1-pass".into()), &key).unwrap();

        // Synthetic Snapshot with business mining income + TaxProfile for 2025.
        let mut state = LedgerState::default();
        state.income_recognized.push(IncomeRecord {
            event: EventId::import(Source::Coinbase, SourceRef::new("e1-mining")),
            recognized_at: time::Date::from_calendar_date(2025, time::Month::March, 1).unwrap(),
            sat: 100_000_000,
            usd_fmv: Decimal::from(50_000i64),
            kind: IncomeKind::Mining,
            business: true,
            pseudo: false,
        });
        let mut profiles = BTreeMap::new();
        profiles.insert(
            2025,
            TaxProfile {
                filing_status: FilingStatus::Single,
                ordinary_taxable_income: Decimal::from(50_000i64),
                magi_excluding_crypto: Decimal::from(50_000i64),
                qualified_dividends_and_other_pref_income: Decimal::ZERO,
                other_net_capital_gain: Decimal::ZERO,
                capital_loss_carryforward_in: Carryforward::default(),
                w2_ss_wages: Decimal::ZERO,
                w2_medicare_wages: Decimal::ZERO,
                schedule_c_expenses: Decimal::ZERO,
            },
        );
        let snap = app::Snapshot {
            events: vec![],
            state,
            cli_config: CliConfig::default(),
            profiles,
            refused: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
        };

        let mut test_app = App::new(vault);
        test_app.screen = Screen::Viewer;
        test_app.selected_year = 2025;
        test_app.snapshot = Some(snap);

        // Press 'e' → modal opens.
        handle_key(&mut test_app, press(KeyCode::Char('e')));
        assert!(
            test_app.export_modal.is_some(),
            "export_modal must be Some after 'e'"
        );

        {
            let modal = test_app.export_modal.as_ref().unwrap();
            assert!(
                modal.files.contains(&"form8949.csv"),
                "files must include form8949.csv"
            );
            assert!(
                modal.files.contains(&"schedule_se.csv"),
                "files must include schedule_se.csv (SE income + profile present)"
            );
            assert_eq!(modal.year, 2025, "modal year must be 2025");
        }

        let out_dir = test_app.export_modal.as_ref().unwrap().out_dir.clone();

        // Press Enter → export executes.
        handle_key(&mut test_app, press(KeyCode::Enter));
        assert!(
            test_app.export_modal.is_none(),
            "export_modal must be None after Enter"
        );
        assert!(
            test_app
                .export_status
                .as_deref()
                .is_some_and(|s| s.contains("Exported to")),
            "export_status must contain 'Exported to'; got: {:?}",
            test_app.export_status
        );

        // Output dir and all expected CSVs must exist.
        assert!(
            out_dir.exists(),
            "export dir must exist after successful export"
        );
        assert!(
            out_dir.join("form8949.csv").exists(),
            "form8949.csv must exist"
        );
        assert!(
            out_dir.join("schedule_d.csv").exists(),
            "schedule_d.csv must exist"
        );
        assert!(
            out_dir.join("form8283.csv").exists(),
            "form8283.csv must exist"
        );
        assert!(
            out_dir.join("schedule_se.csv").exists(),
            "schedule_se.csv must exist (SE income present + profile)"
        );
    }

    // ── sub-3 / R0-C1 — viewer export attestation gate ───────────────────────

    use btctax_adapters::BundledTaxTables;
    use btctax_cli::CliConfig;
    use btctax_core::state::LedgerState;
    use btctax_store::Passphrase;
    use std::collections::BTreeMap;

    /// Build a real (empty) vault so the export dir's parent exists + `do_export` can write, then
    /// return `(tempdir, vault_path)`. The tempdir must be kept alive for the vault to persist.
    fn export_vault(pass: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pass.into()), &key).unwrap();
        (dir, vault)
    }

    fn viewer_app_with_state(vault: PathBuf, state: LedgerState) -> App {
        let snap = app::Snapshot {
            events: vec![],
            state,
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            refused: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
        };
        let mut a = App::new(vault);
        a.screen = Screen::Viewer;
        a.selected_year = 2025;
        a.snapshot = Some(snap);
        a
    }

    /// [sub-3 / R0-C1] While PSEUDO-ACTIVE, the viewer `e` export is a TYPED-WORD attest modal:
    /// Esc cancels (nothing written); an empty/wrong phrase is REFUSED (nothing written, error set);
    /// only typing the EXACT `ATTEST_PHRASE` exports. This is the accidental-filing bypass sub-3 closes.
    #[test]
    fn viewer_pseudo_active_export_requires_typed_phrase() {
        let (_dir, vault) = export_vault("pseudo-attest-pw");

        // Pseudo-active state (a synthetic default contributes).
        let state = LedgerState {
            pseudo_synthetic_count: 1,
            ..LedgerState::default()
        };
        assert!(state.pseudo_active(), "fixture must be pseudo-active");
        let mut app = viewer_app_with_state(vault, state);

        // 'e' opens a TYPED-WORD attest modal (NOT the plain confirm).
        handle_key(&mut app, press(KeyCode::Char('e')));
        assert!(
            app.export_modal
                .as_ref()
                .is_some_and(|m| m.attest.is_some()),
            "pseudo-active export must open the typed-word attest modal"
        );
        let out_dir = app.export_modal.as_ref().unwrap().out_dir.clone();

        // (1) Esc cancels — nothing written, no quit.
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(app.export_modal.is_none(), "Esc cancels the attest modal");
        assert!(!app.should_quit, "Esc on modal must not quit");
        assert!(!out_dir.exists(), "Esc must write nothing");

        // Reopen for the phrase flow.
        handle_key(&mut app, press(KeyCode::Char('e')));
        assert!(app.export_modal.is_some());

        // (2) Empty buffer + Enter → refused: modal stays open, an error is set, nothing written.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.export_modal.is_some(),
            "empty-phrase Enter must NOT export (modal stays open)"
        );
        assert!(
            app.export_modal
                .as_ref()
                .unwrap()
                .attest
                .as_ref()
                .unwrap()
                .error
                .is_some(),
            "empty phrase must set the error"
        );
        assert!(!out_dir.exists(), "empty phrase writes nothing");

        // (3) WRONG phrase (case differs) + Enter → refused, buffer PRESERVED, error set.
        for c in "i attest this is true".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.export_modal.is_some(),
            "wrong phrase must NOT export (modal stays open)"
        );
        {
            let a = app.export_modal.as_ref().unwrap().attest.as_ref().unwrap();
            assert_eq!(
                a.buf, "i attest this is true",
                "wrong phrase preserves buffer"
            );
            assert!(a.error.is_some(), "wrong phrase sets the error");
        }
        assert!(!out_dir.exists(), "wrong phrase writes nothing");
        assert!(
            app.export_status.is_none(),
            "no export status while still refused"
        );

        // (4) Correct the phrase: backspace the whole buffer, type the EXACT phrase, Enter → exports.
        for _ in 0.."i attest this is true".chars().count() {
            handle_key(&mut app, press(KeyCode::Backspace));
        }
        for c in btctax_cli::ATTEST_PHRASE.chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(
            app.export_modal.is_none(),
            "the exact phrase closes the modal"
        );
        assert!(
            app.export_status
                .as_deref()
                .is_some_and(|s| s.contains("Exported to")),
            "the exact phrase exports; got status {:?}",
            app.export_status
        );
        assert!(
            out_dir.join("form8949.csv").exists(),
            "form8949.csv must be written after a correct attestation"
        );
    }

    /// [sub-3] While NOT pseudo-active, the viewer `e` export is TODAY's plain Enter/Esc confirm —
    /// no phrase required (a fully-real ledger is never gated).
    #[test]
    fn viewer_not_pseudo_active_export_plain_confirm() {
        let (_dir, vault) = export_vault("plain-confirm-pw");

        let state = LedgerState::default(); // pseudo_synthetic_count == 0 → not pseudo-active
        assert!(!state.pseudo_active());
        let mut app = viewer_app_with_state(vault, state);

        handle_key(&mut app, press(KeyCode::Char('e')));
        assert!(
            app.export_modal
                .as_ref()
                .is_some_and(|m| m.attest.is_none()),
            "a fully-real ledger opens the PLAIN confirm modal (no attest)"
        );
        let out_dir = app.export_modal.as_ref().unwrap().out_dir.clone();

        // Plain Enter exports immediately — no phrase needed.
        handle_key(&mut app, press(KeyCode::Enter));
        assert!(app.export_modal.is_none(), "plain Enter closes the modal");
        assert!(
            app.export_status
                .as_deref()
                .is_some_and(|s| s.contains("Exported to")),
            "plain confirm exports; got status {:?}",
            app.export_status
        );
        assert!(
            out_dir.join("form8949.csv").exists(),
            "form8949.csv must be written on plain confirm"
        );
    }

    // ── whatif P3 — the read-only what-if planner overlay ────────────────────

    /// [M5] `w` on the Unlock screen (no snapshot) is a no-op: it does NOT open the panel (and, as a
    /// printable char, it lands in the passphrase buffer). `w` on the Viewer with no snapshot is a
    /// pure no-op. Neither path panics.
    #[test]
    fn whatif_panel_w_noop_before_snapshot() {
        // Unlock screen: 'w' is a passphrase char, never opens the panel.
        let mut app = new_app();
        assert_eq!(app.screen, Screen::Unlock);
        handle_key(&mut app, press(KeyCode::Char('w')));
        assert!(
            app.whatif.is_none(),
            "'w' on Unlock must NOT open the what-if panel"
        );
        assert_eq!(
            app.unlock.buffer.chars().count(),
            1,
            "'w' on Unlock goes to the passphrase buffer"
        );

        // Viewer screen with no snapshot: pure no-op.
        let mut app2 = new_app();
        app2.screen = Screen::Viewer;
        assert!(app2.snapshot.is_none());
        handle_key(&mut app2, press(KeyCode::Char('w')));
        assert!(
            app2.whatif.is_none(),
            "'w' with no snapshot must not open the panel"
        );
    }

    /// [★ whatif_panel_never_persists] The NON-NEGOTIABLE gate: open the panel, drive it through a
    /// SELL and a HARVEST compute, then close it — and the vault file is BYTE-IDENTICAL afterward. The
    /// panel calls only the clone-fold-discard `whatif::{sell,harvest}` + reads the snapshot; it never
    /// writes. (An empty vault yields NoLots refusals — the compute paths still execute, still write
    /// nothing.)
    #[test]
    fn whatif_panel_never_persists() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key = dir.path().join("key.asc");
        let pp = "whatif-nopersist-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp.into()), &key).unwrap();

        let bytes_before = std::fs::read(&vault).expect("vault readable before");

        let outcome = crate::unlock::attempt_open(&vault, Passphrase::new(pp.into()));
        let (snapshot, year) = match outcome {
            crate::unlock::OpenOutcome::Success(s, y) => (s, y),
            _ => panic!("open must succeed for the non-persistence KAT"),
        };
        let mut app = App::new(vault.clone());
        app.screen = Screen::Viewer;
        app.selected_year = year;
        app.snapshot = Some(*snapshot);

        // Open the panel with 'w'.
        handle_key(&mut app, press(KeyCode::Char('w')));
        assert!(app.whatif.is_some(), "'w' opens the what-if panel");

        // Drive a SELL: focus the amount, type a BTC decimal, move to the price, type a price, compute.
        handle_key(&mut app, press(KeyCode::Down)); // At -> Amount
        for c in "0.5".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Down)); // Amount -> Wallet
        handle_key(&mut app, press(KeyCode::Down)); // Wallet -> Price
        for c in "30000".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // EXPLICIT compute (NoLots on an empty vault)

        // Toggle to HARVEST and compute the default target.
        handle_key(&mut app, press(KeyCode::Tab));
        assert!(
            matches!(
                app.whatif.as_ref().unwrap().mode,
                crate::whatif_panel::WhatIfMode::Harvest
            ),
            "Tab toggles to HARVEST"
        );
        handle_key(&mut app, press(KeyCode::Enter)); // compute harvest

        // Close the panel.
        handle_key(&mut app, press(KeyCode::Esc));
        assert!(app.whatif.is_none(), "Esc closes the panel");
        assert!(!app.should_quit, "Esc on the panel must not quit the app");

        drop(app);
        let bytes_after = std::fs::read(&vault).expect("vault readable after");
        assert_eq!(
            bytes_before, bytes_after,
            "[★] the vault file MUST be byte-identical after opening + driving the what-if panel"
        );
    }

    /// `handle_key_still_only_mutates_ui` — the app.rs:120 read-only invariant, EXTENDED to the panel:
    /// driving the panel through a full sell/harvest/toggle/close lifecycle mutates ONLY the `whatif`
    /// UI field — never the read-only `snapshot` (events + projected state are unchanged).
    #[test]
    fn handle_key_still_only_mutates_ui() {
        let (_dir, vault) = export_vault("whatif-ui-invariant-pass");
        let mut app = viewer_app_with_state(vault, LedgerState::default());

        let events_before = app.snapshot.as_ref().unwrap().events.clone();
        let lots_before = app.snapshot.as_ref().unwrap().state.lots.len();

        // Full panel lifecycle: open, type, compute, toggle both ways, close.
        handle_key(&mut app, press(KeyCode::Char('w')));
        handle_key(&mut app, press(KeyCode::Down)); // -> Amount
        for c in "0.25".chars() {
            handle_key(&mut app, press(KeyCode::Char(c)));
        }
        handle_key(&mut app, press(KeyCode::Enter)); // compute sell
        handle_key(&mut app, press(KeyCode::Char('h'))); // -> Harvest
        handle_key(&mut app, press(KeyCode::Enter)); // compute harvest
        handle_key(&mut app, press(KeyCode::Char('s'))); // -> Sell
        handle_key(&mut app, press(KeyCode::Esc)); // close

        let snap = app.snapshot.as_ref().unwrap();
        assert_eq!(
            snap.events, events_before,
            "handle_key must NOT mutate ledger events (read-only invariant)"
        );
        assert_eq!(
            snap.state.lots.len(),
            lots_before,
            "handle_key must NOT mutate projected ledger state"
        );
        assert!(app.whatif.is_none(), "the panel is closed");
        assert!(
            !app.should_quit,
            "panel keys (incl. 's'/'h') must not quit while planning"
        );
    }
}
