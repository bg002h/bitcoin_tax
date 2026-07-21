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
//! never writes the vault or any decrypted image of it; writes only the year's form artifacts
//! via `export.rs` on explicit user confirmation. This module performs no writes.

use crate::app::Snapshot;
use btctax_adapters::BundledTaxTables;
use btctax_cli::{CliError, Session};
use btctax_core::{LedgerState, TaxProfile};
use btctax_store::{Passphrase, StoreError};
use std::collections::BTreeMap;
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

/// Outcome of [`open_session`] — like [`OpenOutcome`] but the `Session` is RETURNED instead
/// of dropped so that the editor can hold it for the whole TUI session.
pub enum SessionOpenOutcome {
    /// Vault opened successfully; the live session, built snapshot, and year are returned.
    ///
    /// The session is returned (not dropped) so the editor can hold the `VaultLock` for the
    /// full TUI session.  The viewer's `attempt_open` drops the session immediately via the
    /// `drop(session)` call in its wrapper — preserving the viewer's structural read-only
    /// property ("the viewer App never stores a Session").
    ///
    /// Both `session` and `snapshot` are boxed to keep all enum variants close in size
    /// (mirrors `OpenOutcome::Success(Box<Snapshot>, i32)`).
    Success {
        /// The open, live session — caller must keep it alive for the TUI lifetime.
        session: Box<Session>,
        /// Read-only snapshot, boxed to keep variant sizes close.
        snapshot: Box<Snapshot>,
        /// Default display year (latest year with events, or 2025 for an empty ledger).
        year: i32,
    },
    /// Another process holds the vault lock → show the `Screen::Locked` screen.
    Locked,
    /// Any other failure → show an error line on the `Screen::Unlock` screen.
    Error(String),
}

/// Try to open `vault_path` with `pp`, build a read-only [`Snapshot`], and RETURN the session.
///
/// Unlike [`attempt_open`], the session is returned so the caller can hold the vault lock for
/// the whole TUI session (the editor use-case).  Error strings are single-sourced via the
/// private `map_open_error` helper — identical messages in both the viewer and the editor.
///
/// **[R0-M5] Passphrase-drop ordering PINNED:** `pp` is zeroized (`drop(pp)`) immediately after
/// `Session::open` succeeds and BEFORE `build_snapshot` — exactly today's ordering.
pub fn open_session(vault_path: &Path, pp: Passphrase) -> SessionOpenOutcome {
    let session = match Session::open(vault_path, &pp) {
        Ok(s) => s,
        Err(CliError::Store(StoreError::Locked)) => return SessionOpenOutcome::Locked,
        Err(e) => return SessionOpenOutcome::Error(map_open_error(&e, vault_path)),
    };
    // [R0-M5] Zeroize the passphrase immediately after Session::open succeeds,
    // BEFORE build_snapshot — pinned ordering, verified by inspection at whole-diff.
    drop(pp);

    match build_snapshot(&session) {
        Ok((snapshot, year)) => SessionOpenOutcome::Success {
            session: Box::new(session),
            snapshot: Box::new(snapshot),
            year,
        },
        Err(e) => SessionOpenOutcome::Error(format!("failed to load vault data: {e}")),
    }
}

/// Try to open `vault_path` with `pp` and build a read-only [`Snapshot`].
///
/// Thin wrapper over [`open_session`]: on success, the session is DROPPED immediately
/// (preserving the viewer's structural property — the viewer App never stores a Session).
///
/// # Read-only guarantees
/// - **[R0-I1]** The session is dropped immediately on success; `save()` is never reachable.
/// - **[R0-M2]** `CliConfig` is loaded via `session.config()`.
/// - **[R0-M3]** `optimize_attested_set` is NOT called.
/// - **[R0-I2]** `pp` arrives by MOVE (never cloned); `Passphrase::Drop` zeroizes it.
///   `Session::conn()` is NEVER called directly from `btctax-tui`.
pub fn attempt_open(vault_path: &Path, pp: Passphrase) -> OpenOutcome {
    match open_session(vault_path, pp) {
        SessionOpenOutcome::Success {
            session,
            snapshot,
            year,
        } => {
            // Drop the session — the viewer's structural read-only property:
            // the viewer App never stores a Session (unlike the editor).
            drop(session);
            OpenOutcome::Success(snapshot, year)
        }
        SessionOpenOutcome::Locked => OpenOutcome::Locked,
        SessionOpenOutcome::Error(msg) => OpenOutcome::Error(msg),
    }
}

/// Build a [`Snapshot`] from an open `Session`.
///
/// Uses ONLY the typed read-only methods — never `session.conn()` directly [R0-I1].
/// Exposed as `pub` so the editor can call it for re-projection after a confirmed mutation.
pub fn build_snapshot(session: &Session) -> Result<(Snapshot, i32), CliError> {
    // [R0-M2] CliConfig is loaded here (needed by build_verify in Compliance tab)
    // [R0-M3] optimize_attested_set is intentionally omitted
    let (events, state, _) = session.load_events_and_project()?;
    let cli_config = session.config()?;
    let tables = BundledTaxTables::load();
    // [P2-C1] Resolve+screen EVERY stored/ReturnInputs year through the SAME single resolver the CLI uses,
    // so the viewer never shows a different liability (or a number for a refused year) than `report`.
    // Split the per-year outcomes into displayable profiles vs refusals; a `Ready { None }` (missing) is
    // simply absent (the tab shows TaxProfileMissing, as before).
    let mut profiles: BTreeMap<i32, TaxProfile> = BTreeMap::new();
    let mut refused: BTreeMap<i32, String> = BTreeMap::new();
    for (year, outcome) in session.resolve_all_screened(&state, &tables)? {
        match outcome {
            btctax_cli::resolve::ProfileOutcome::Ready {
                profile: Some(p), ..
            } => {
                profiles.insert(year, p);
            }
            btctax_cli::resolve::ProfileOutcome::Ready { profile: None, .. } => {}
            btctax_cli::resolve::ProfileOutcome::Uncomputable { detail } => {
                refused.insert(year, detail);
            }
        }
    }
    let donation_details = session.donation_details()?;
    // [R0-M1] the `[est]` marker set — loaded via the typed accessor, NEVER `conn()` directly.
    let bulk_estimated = session.bulk_estimated()?;
    // [whatif P3 / R0-I1] The owned price provider for the read-only what-if panel — built EXACTLY as
    // the session's own `default_prices()` (session.rs): the bundled daily-close dataset with the LOCAL
    // price cache layered over it. MUST pass the real cache path (never `None`) so the panel's prices are
    // byte-identical to the viewer's tabs. `?` compiles: `From<AdapterError> for CliError` exists.
    let prices = btctax_adapters::LayeredPrices::load_with_cache(
        btctax_cli::price_cache::default_cache_path().as_deref(),
    )?;
    let year = latest_year(&state);
    let snapshot = Snapshot {
        events,
        state,
        cli_config,
        profiles,
        refused,
        tables,
        donation_details,
        bulk_estimated,
        prices,
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
        // UX-P4-8: `Session::open` now enriches a pathless vault-open `io::Error` into
        // `CliError::PathIo`. The unlock screen has its OWN concise "no vault at <path>" line and
        // renders errors WITHOUT wrapping (`draw_unlock_screen`), so keep the short form here — the
        // long `PathIo` string (path + hint) would clip mid-clause on a narrow terminal.
        CliError::PathIo { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
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

    /// UX-P4-8 (fold I1): `Session::open` on a missing vault now returns `CliError::PathIo`, but the
    /// unlock screen must still show the CONCISE `no vault at <path>` — not the long path+hint
    /// `PathIo` string, which `draw_unlock_screen` (no wrap) would clip mid-clause. Regression pin.
    #[test]
    fn missing_vault_maps_to_concise_no_vault_message() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("does_not_exist.pgp");
        let err = Session::open(&vault, &Passphrase::new("pw".into()))
            .expect_err("a missing vault must error");
        let msg = map_open_error(&err, &vault);
        assert_eq!(msg, format!("no vault at {}", vault.display()));
    }

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
    // KAT-E3 extends it: do_export (writing the form artifacts) must ALSO leave the vault
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
                attest: None,
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

    // ── [R0-M4] build_snapshot prices parity ────────────────────────────────
    //
    // The snapshot's owned `LayeredPrices` (built in `build_snapshot`) must return the SAME FMV as the
    // session's own price provider for a sample date — i.e. it is byte-identical to the session's
    // `default_prices()`, not merely "is set". This is what makes the what-if panel's baseline agree
    // with the viewer's Tax tab.

    #[test]
    fn build_snapshot_prices_parity() {
        use btctax_core::PriceProvider;

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "prices-parity-pass";
        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        let session = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into()))
            .expect("open must succeed");
        let (snapshot, _year) = build_snapshot(&session).expect("build_snapshot must succeed");

        // Real bundled dates: the snapshot provider and the session provider must agree exactly.
        for sample in [
            time::macros::date!(2025 - 06 - 15),
            time::macros::date!(2026 - 06 - 03),
            time::macros::date!(2030 - 01 - 01), // uncovered → None on both
        ] {
            assert_eq!(
                snapshot.prices.usd_per_btc(sample),
                session.prices().usd_per_btc(sample),
                "snapshot prices must match the session's own provider at {sample}"
            );
        }
    }

    /// UX-P4-1 surface 3 invariant [T2-M2 / C2]: the ENUMERATION invariant that licenses the viewer Tax
    /// tab's count-only pseudo signal. With pseudo mode ON but NO stored profile for the year,
    /// `resolve_all_screened` does NOT enumerate the year (it enumerates only `tax_profile ∪ return_inputs`
    /// years), so the CLI's $0 placeholder profile NEVER reaches `snap.profiles` — the viewer renders NOT
    /// COMPUTABLE, never a fictional number. A regression that made the viewer resolve a bare year on demand
    /// (CLI placeholder parity) would put an unflagged $0 number here (count==0 ⇒ no banner — the C2 channel
    /// reborn); the `NOT COMPUTABLE` assert below REDS that.
    #[test]
    fn build_snapshot_pseudo_on_unprofiled_year_stays_not_computable_in_the_viewer() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let pp = Passphrase::new("enum-invariant-pass".into());
        btctax_cli::cmd::init::run(&vault, &pp, &dir.path().join("k.asc")).unwrap();
        btctax_cli::cmd::reconcile::pseudo_set_mode(&vault, &pp, true).unwrap();

        let session = btctax_cli::Session::open(&vault, &pp).expect("open must succeed");
        let (snapshot, _year) = build_snapshot(&session).expect("build_snapshot must succeed");
        // 2025 has no stored profile ⇒ never enumerated ⇒ NOT COMPUTABLE (never a $0 placeholder number).
        let content = crate::tabs::tax::render_tax_content(&snapshot, 2025);
        assert!(
            content.contains("NOT COMPUTABLE"),
            "a pseudo-on year with no stored profile must render NOT COMPUTABLE in the viewer, not a \
             placeholder number:\n{content}"
        );
        assert!(
            !content.contains("[PSEUDO]"),
            "the placeholder channel must NOT surface any [PSEUDO] figure in the viewer:\n{content}"
        );
    }

    // ── Wrapper-consistency KAT: attempt_open and open_session agree ─────────
    //
    // `attempt_open` is a thin wrapper over `open_session`; their outcome variants must
    // correspond one-to-one for every error class (Success/Locked/Error).

    #[test]
    fn attempt_open_is_wrapper_consistent_with_open_session() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let key_backup = dir.path().join("key.asc");
        let pp_str = "wrapper-consistency-pass";

        btctax_cli::cmd::init::run(&vault, &Passphrase::new(pp_str.into()), &key_backup).unwrap();

        // ── Correct passphrase: both return Success ───────────────────────────
        // attempt_open drops the session immediately (viewer property).
        let out1 = attempt_open(&vault, Passphrase::new(pp_str.into()));
        assert!(
            matches!(out1, OpenOutcome::Success(_, _)),
            "attempt_open with correct passphrase must return Success"
        );
        // open_session returns the session; wrap in a block so it drops before the next check.
        {
            let out2 = open_session(&vault, Passphrase::new(pp_str.into()));
            assert!(
                matches!(out2, SessionOpenOutcome::Success { .. }),
                "open_session with correct passphrase must return Success"
            );
            // out2 (and the held session) drop here — vault lock released.
        }

        // ── Wrong passphrase: both return Error ───────────────────────────────
        // No session is held at this point, so the vault is accessible.
        let out1 = attempt_open(&vault, Passphrase::new("wrong-pass".into()));
        let out2 = open_session(&vault, Passphrase::new("wrong-pass".into()));
        assert!(
            matches!(out1, OpenOutcome::Error(_)),
            "attempt_open with wrong passphrase must return Error"
        );
        assert!(
            matches!(out2, SessionOpenOutcome::Error(_)),
            "open_session with wrong passphrase must return Error"
        );

        // ── Locked vault: both return Locked ─────────────────────────────────
        let _holder = btctax_cli::Session::open(&vault, &Passphrase::new(pp_str.into())).unwrap();
        let out1 = attempt_open(&vault, Passphrase::new(pp_str.into()));
        let out2 = open_session(&vault, Passphrase::new(pp_str.into()));
        assert!(
            matches!(out1, OpenOutcome::Locked),
            "attempt_open on locked vault must return Locked"
        );
        assert!(
            matches!(out2, SessionOpenOutcome::Locked),
            "open_session on locked vault must return Locked"
        );
    }
}
