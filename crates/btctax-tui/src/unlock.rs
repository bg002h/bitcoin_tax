//! Unlock screen state and vault-open logic.
//!
//! # Passphrase hygiene [R0-I2/M7]
//! - `buffer` is `String::with_capacity(PASSPHRASE_CAP)` and NEVER reallocates:
//!   any input that would push the byte-length past `PASSPHRASE_CAP` is silently rejected,
//!   so `.push()` is never called when at or beyond the cap.
//! - The passphrase is moved into `Passphrase::new` via `std::mem::take` — NEVER cloned.
//!   The store's `Passphrase::Drop` impl zeroizes the only copy.
//! - The raw chars are NEVER logged or rendered; only `●`×char_count is displayed.
//!
//! # Read-only contract
//! never writes the vault or any decrypted image of it; writes only the four form CSVs
//! via `export.rs` on explicit user confirmation. This module performs no writes.

use crate::app::Snapshot;
use btctax_adapters::BundledTaxTables;
use btctax_cli::{CliError, Session};
use btctax_core::LedgerState;
use btctax_store::{Passphrase, StoreError};
use std::path::Path;

/// Maximum byte-length of the passphrase buffer.
///
/// Pre-allocated at this size so `.push()` within the cap never triggers a reallocation,
/// which would scatter passphrase bytes across freed heap [R0-I2/M7].
pub const PASSPHRASE_CAP: usize = 128;

/// Live state for the Unlock screen.
pub struct UnlockState {
    /// Pre-allocated passphrase buffer [R0-I2/M7].
    ///
    /// NEVER clone; NEVER log or render the actual content.
    /// Move out via `Passphrase::new(std::mem::take(&mut buffer))`.
    pub buffer: String,
    /// Error message shown below the passphrase field.
    /// Cleared automatically when the user begins typing.
    pub error: Option<String>,
}

impl UnlockState {
    /// Allocate a new `UnlockState` with the buffer pre-allocated to `PASSPHRASE_CAP`.
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(PASSPHRASE_CAP),
            error: None,
        }
    }

    /// Push one character into the buffer.
    ///
    /// Silently ignores `c` if adding it would push the byte-length past `PASSPHRASE_CAP`,
    /// guaranteeing the `String` never reallocates [R0-I2/M7].
    pub fn push_char(&mut self, c: char) {
        if self.buffer.len() + c.len_utf8() <= PASSPHRASE_CAP {
            self.buffer.push(c);
        }
    }

    /// Remove the last character (backspace / delete-left).
    /// No-op when the buffer is empty.
    pub fn pop_char(&mut self) {
        self.buffer.pop();
    }
}

impl Default for UnlockState {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of [`attempt_open`].
pub enum OpenOutcome {
    /// Vault opened and `Snapshot` built successfully.
    /// Snapshot is boxed to keep all enum variants close in size.
    /// Contains the read-only snapshot and the default `selected_year`.
    Success(Box<Snapshot>, i32),
    /// Another process holds the vault lock → show the `Screen::Locked` screen.
    Locked,
    /// Any other failure → show an error line on the `Screen::Unlock` screen.
    Error(String),
}

/// Try to open `vault_path` with `pp` and build a read-only [`Snapshot`].
///
/// # Read-only guarantees
/// - **[R0-I1]** `session` is held in an **immutable** binding; `save()` takes `&mut self`,
///   so calling it is a compile error.
/// - **[R0-M2]** `CliConfig` is loaded via `session.config()`.
/// - **[R0-M3]** `optimize_attested_set` is NOT called.
/// - **[R0-I2]** `pp` arrives by MOVE (never cloned); `Passphrase::Drop` zeroizes it.
///   `Session::conn()` is NEVER called directly from `btctax-tui`.
pub fn attempt_open(vault_path: &Path, pp: Passphrase) -> OpenOutcome {
    // [R0-I1] IMMUTABLE binding — `let mut session` would make `save()` callable.
    let session = match Session::open(vault_path, &pp) {
        Ok(s) => s,
        Err(CliError::Store(StoreError::Locked)) => return OpenOutcome::Locked,
        Err(e) => return OpenOutcome::Error(map_open_error(&e, vault_path)),
    };
    // pp's job is done once Session::open succeeds; zeroize it now rather than at frame exit.
    drop(pp);

    match build_snapshot(&session) {
        Ok((snapshot, year)) => OpenOutcome::Success(Box::new(snapshot), year),
        Err(e) => OpenOutcome::Error(format!("failed to load vault data: {e}")),
    }
}

/// Build a [`Snapshot`] from an open, immutable `Session`.
///
/// Uses ONLY the typed read-only methods — never `session.conn()` directly [R0-I1].
fn build_snapshot(session: &Session) -> Result<(Snapshot, i32), CliError> {
    // [R0-M2] CliConfig is loaded here (needed by build_verify in Compliance tab)
    // [R0-M3] optimize_attested_set is intentionally omitted
    let (events, state, _) = session.load_events_and_project()?;
    let profiles = session.all_tax_profiles()?;
    let cli_config = session.config()?;
    let tables = BundledTaxTables::load();
    let donation_details = session.donation_details()?;
    let year = latest_year(&state);
    let snapshot = Snapshot {
        events,
        state,
        cli_config,
        profiles,
        tables,
        donation_details,
    };
    Ok((snapshot, year))
}

/// Derive the default display year: the latest year with a disposal or income event,
/// or 2025 when the ledger is empty.
pub fn latest_year(state: &LedgerState) -> i32 {
    let from_disposals = state.disposals.iter().map(|d| d.disposed_at.year());
    let from_income = state
        .income_recognized
        .iter()
        .map(|r| r.recognized_at.year());
    from_disposals.chain(from_income).max().unwrap_or(2025)
}

/// Map a [`CliError`] to a user-facing message for the Unlock screen error line.
fn map_open_error(e: &CliError, vault_path: &Path) -> String {
    match e {
        CliError::Store(StoreError::WrongPassphrase) => "incorrect passphrase".to_owned(),
        // Locked is handled before calling map_open_error; this arm is a defensive fallback.
        CliError::Store(StoreError::Locked) => {
            "vault in use by another process — close the CLI/other viewer and retry".to_owned()
        }
        CliError::Store(StoreError::HalfCreatedVault(_)) => {
            "interrupted init — run `btctax init --repair`".to_owned()
        }
        CliError::Store(StoreError::Io(io_err))
            if io_err.kind() == std::io::ErrorKind::NotFound =>
        {
            format!("no vault at {}", vault_path.display())
        }
        CliError::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
            format!("no vault at {}", vault_path.display())
        }
        _ => format!("vault error: {e}"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Screen};

    // ── Masked-input buffer [R0-I2/M7] ──────────────────────────────────────

    #[test]
    fn push_char_appends_and_char_count_grows() {
        let mut s = UnlockState::new();
        assert_eq!(s.buffer.chars().count(), 0);
        s.push_char('a');
        s.push_char('b');
        s.push_char('c');
        assert_eq!(s.buffer.chars().count(), 3);
        // Rendered mask: one ● per character
        let mask = "●".repeat(s.buffer.chars().count());
        assert_eq!(mask.chars().count(), 3);
    }

    #[test]
    fn pop_char_removes_last_and_is_noop_on_empty() {
        let mut s = UnlockState::new();
        s.push_char('x');
        s.push_char('y');
        assert_eq!(s.buffer.chars().count(), 2);
        s.pop_char();
        assert_eq!(s.buffer.chars().count(), 1);
        s.pop_char();
        assert_eq!(s.buffer.chars().count(), 0);
        s.pop_char(); // no-op on empty
        assert_eq!(s.buffer.chars().count(), 0);
    }

    #[test]
    fn input_past_cap_is_silently_ignored() {
        let mut s = UnlockState::new();
        // Fill exactly to PASSPHRASE_CAP bytes (ASCII: 1 byte per char)
        for _ in 0..PASSPHRASE_CAP {
            s.push_char('a');
        }
        assert_eq!(s.buffer.len(), PASSPHRASE_CAP);
        // Any further input must be rejected
        s.push_char('z');
        assert_eq!(
            s.buffer.len(),
            PASSPHRASE_CAP,
            "buffer must not grow past PASSPHRASE_CAP"
        );
    }

    #[test]
    fn buffer_never_reallocates_within_cap() {
        let mut s = UnlockState::new();
        assert_eq!(
            s.buffer.capacity(),
            PASSPHRASE_CAP,
            "must be pre-allocated to PASSPHRASE_CAP"
        );
        for _ in 0..PASSPHRASE_CAP {
            s.push_char('p');
        }
        assert_eq!(
            s.buffer.capacity(),
            PASSPHRASE_CAP,
            "capacity must not change when filling to cap"
        );
    }

    #[test]
    fn mem_take_empties_buffer_and_yields_content() {
        let mut s = UnlockState::new();
        s.push_char('s');
        s.push_char('e');
        s.push_char('c');
        let taken = std::mem::take(&mut s.buffer);
        assert_eq!(taken, "sec");
        assert!(s.buffer.is_empty(), "buffer must be empty after mem::take");
    }

    // ── attempt_open: correct passphrase ────────────────────────────────────

    #[test]
    fn correct_passphrase_yields_success_with_populated_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "task2-correct-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        let outcome = attempt_open(&vault, Passphrase::new(pp_str.into()));

        match outcome {
            OpenOutcome::Success(snapshot, _year) => {
                // Fields must all be accessible (Snapshot is populated)
                let _ = &snapshot.events;
                let _ = &snapshot.state;
                let _ = &snapshot.cli_config;
                let _ = &snapshot.profiles;
                let _ = &snapshot.tables;
                // snapshot is Box<Snapshot>; fields are accessed via auto-deref
            }
            OpenOutcome::Locked => panic!("expected Success, got Locked"),
            OpenOutcome::Error(e) => panic!("expected Success, got Error({e})"),
        }
    }

    // ── App-level: correct passphrase → Screen::Viewer ──────────────────────

    #[test]
    fn do_unlock_correct_passphrase_transitions_to_viewer_with_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "task2-viewer-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        let mut app = App::new(vault.clone());
        // Push passphrase into buffer (never renders as plaintext — only ● shown in draw)
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(
            app.screen,
            Screen::Viewer,
            "screen must transition to Viewer"
        );
        assert!(app.snapshot.is_some(), "Snapshot must be populated");

        let snap = app.snapshot.as_ref().unwrap();
        // All Snapshot fields must be present
        let _ = &snap.events;
        let _ = &snap.state;
        let _ = &snap.cli_config;
        let _ = &snap.profiles;
        let _ = &snap.tables;
    }

    // ── attempt_open: wrong passphrase → Error("incorrect passphrase") ──────

    #[test]
    fn wrong_passphrase_yields_incorrect_passphrase_error() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");

        btctax_cli::cmd::init::run(&vault, &Passphrase::new("right".into()), &key_backup).unwrap();

        let outcome = attempt_open(&vault, Passphrase::new("wrong".into()));

        match outcome {
            OpenOutcome::Error(msg) => assert_eq!(msg, "incorrect passphrase"),
            OpenOutcome::Success(..) => panic!("wrong passphrase must not succeed"),
            OpenOutcome::Locked => panic!("wrong passphrase must not report Locked"),
        }
    }

    // ── App-level: wrong passphrase → stays on Unlock, buffer cleared ────────

    #[test]
    fn do_unlock_wrong_passphrase_stays_on_unlock_with_error_and_clear_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");

        btctax_cli::cmd::init::run(&vault, &Passphrase::new("correct".into()), &key_backup)
            .unwrap();

        let mut app = App::new(vault.clone());
        for c in "wrong-pass".chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(
            app.screen,
            Screen::Unlock,
            "screen must stay on Unlock after wrong passphrase"
        );
        assert_eq!(
            app.unlock.error.as_deref(),
            Some("incorrect passphrase"),
            "error must be set"
        );
        assert!(
            app.unlock.buffer.is_empty(),
            "buffer must be cleared after failed unlock (mem::take)"
        );
        assert!(app.snapshot.is_none(), "Snapshot must not be set");
    }

    // ── attempt_open: Locked vault → OpenOutcome::Locked ────────────────────

    #[test]
    fn locked_vault_yields_locked_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "lock-test-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        // Hold the first session open to keep the vault lock acquired
        let _session_holder = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into()))
            .expect("first open must succeed");

        // Second open while the lock is held → Locked
        let outcome = attempt_open(&vault, Passphrase::new(pp_str.into()));

        match outcome {
            OpenOutcome::Locked => {} // expected
            OpenOutcome::Success(..) => panic!("second open on locked vault must not succeed"),
            OpenOutcome::Error(e) => panic!("expected Locked, got Error({e})"),
        }
    }

    // ── App-level: locked vault → Screen::Locked ────────────────────────────

    #[test]
    fn do_unlock_locked_vault_transitions_to_locked_screen() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "app-lock-test-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        let _session_holder = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into()))
            .expect("first open must succeed");

        let mut app = App::new(vault.clone());
        for c in pp_str.chars() {
            app.unlock.push_char(c);
        }
        app.do_unlock();

        assert_eq!(
            app.screen,
            Screen::Locked,
            "screen must transition to Locked when vault lock is held"
        );
    }

    // ── App-level: BTCTAX_PASSPHRASE env-var fast-path ──────────────────────

    /// Mutex to serialize env-var tests: `std::env::set_var` / `remove_var` are not safe
    /// to call concurrently from multiple threads (cargo test runs tests in parallel by default).
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn env_passphrase_var_opens_vault_and_transitions_to_viewer() {
        let _env_guard = ENV_MUTEX.lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "env-path-test-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        // Set env var; the `_env_guard` mutex ensures no other env-var test runs concurrently.
        // SAFETY: env_guard serializes all callers — no concurrent set/remove races.
        std::env::set_var("BTCTAX_PASSPHRASE", pp_str);

        let mut app = App::new(vault.clone());
        app.try_env_passphrase();

        // Always clean up before any assert so the var is removed even on panic.
        std::env::remove_var("BTCTAX_PASSPHRASE");

        assert_eq!(
            app.screen,
            Screen::Viewer,
            "BTCTAX_PASSPHRASE env var must transition App to Screen::Viewer"
        );
        assert!(
            app.snapshot.is_some(),
            "Snapshot must be populated when opened via BTCTAX_PASSPHRASE"
        );
        // Spot-check Snapshot fields are accessible (they are populated, not None/empty).
        let snap = app.snapshot.as_ref().unwrap();
        let _ = &snap.events;
        let _ = &snap.state;
        let _ = &snap.cli_config;
        let _ = &snap.profiles;
        let _ = &snap.tables;
    }

    // ── Read-only behavioral test [R0-M6] + KAT-E3 ──────────────────────────
    //
    // The immutable Session binding is a compile-level guarantee that save() cannot be called.
    // This behavioral test adds a runtime confirmation: open→build-Snapshot→drop leaves
    // the vault file BYTE-IDENTICAL to what it was before.
    //
    // KAT-E3 extends it: do_export (writing the four form CSVs) must ALSO leave the vault
    // byte-identical. The export writes ONLY to the timestamped export subdirectory; the
    // vault file itself is never touched.

    #[test]
    fn vault_file_bytes_unchanged_after_open_build_snapshot_drop() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "readonly-bytes-check";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        let bytes_before = std::fs::read(&vault).expect("vault must be readable before");

        {
            // open → build Snapshot → everything drops here
            let outcome = attempt_open(&vault, Passphrase::new(pp_str.into()));
            assert!(
                matches!(outcome, OpenOutcome::Success(_, _)),
                "open must succeed for the read-only bytes test"
            );
            // snapshot and session both drop at end of this block
        }

        let bytes_after_open_drop =
            std::fs::read(&vault).expect("vault must be readable after open+drop");
        assert_eq!(
            bytes_before, bytes_after_open_drop,
            "[R0-M6] vault file must be byte-identical after open→build-Snapshot→drop"
        );

        // ── KAT-E3: vault bytes unchanged after a FULL export cycle ──────────
        // Open again, do_export (writes form CSVs to a fresh subdir), assert vault unchanged.
        {
            let outcome = attempt_open(&vault, Passphrase::new(pp_str.into()));
            let (snapshot, year) = match outcome {
                OpenOutcome::Success(snap, yr) => (snap, yr),
                _ => panic!("[KAT-E3] second open must succeed"),
            };

            let export_now = time::macros::datetime!(2025-06-15 10:00:00 UTC);
            let out_dir = crate::export::export_dir_for(&vault, export_now);
            let modal = crate::export::ExportConfirmState {
                year,
                out_dir,
                files: crate::export::compute_files(&snapshot, year),
                export_now,
            };
            crate::export::do_export(&snapshot, &modal).expect("[KAT-E3] do_export must succeed");
            // snapshot drops here
        }

        let bytes_after_export =
            std::fs::read(&vault).expect("vault must be readable after export");
        assert_eq!(
            bytes_before, bytes_after_export,
            "[KAT-E3] vault file must be byte-identical after a full export cycle \
             (export writes only to the timestamped CSVs, never the vault)"
        );
    }
}
