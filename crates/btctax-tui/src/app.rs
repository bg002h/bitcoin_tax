//! Core application state: `Screen`, `Tab`, `Snapshot`, and `App`.
//!
//! STRICTLY READ-ONLY: this module MUST NOT call `Session::save()`, `persistence::append_*`,
//! any `btctax_cli::cmd::*` mutating command, or `Session::conn()`.

use crate::unlock;
use crate::unlock::UnlockState;
use btctax_adapters::BundledTaxTables;
use btctax_cli::CliConfig;
use btctax_core::{LedgerEvent, LedgerState, ProjectionConfig, TaxProfile};
use btctax_store::Passphrase;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Which top-level screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Unlock,
    Locked,
    Viewer,
}

/// The six viewer tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Holdings,
    Disposals,
    Income,
    Tax,
    Forms,
    Compliance,
}

impl Tab {
    /// Ordered list of all tabs (for tab-bar rendering and cycling).
    pub const ALL: [Tab; 6] = [
        Tab::Holdings,
        Tab::Disposals,
        Tab::Income,
        Tab::Tax,
        Tab::Forms,
        Tab::Compliance,
    ];

    /// Human-readable tab title shown in the tab bar and content block.
    pub fn title(self) -> &'static str {
        match self {
            Tab::Holdings => "Holdings",
            Tab::Disposals => "Disposals",
            Tab::Income => "Income",
            Tab::Tax => "Tax",
            Tab::Forms => "Forms",
            Tab::Compliance => "Compliance",
        }
    }

    /// Zero-based index into `Tab::ALL` (used to drive the `Tabs` widget selection).
    pub fn index(self) -> usize {
        match self {
            Tab::Holdings => 0,
            Tab::Disposals => 1,
            Tab::Income => 2,
            Tab::Tax => 3,
            Tab::Forms => 4,
            Tab::Compliance => 5,
        }
    }

    /// Advance to the next tab, wrapping around.
    pub fn next(self) -> Self {
        match self {
            Tab::Holdings => Tab::Disposals,
            Tab::Disposals => Tab::Income,
            Tab::Income => Tab::Tax,
            Tab::Tax => Tab::Forms,
            Tab::Forms => Tab::Compliance,
            Tab::Compliance => Tab::Holdings,
        }
    }

    /// Go back to the previous tab, wrapping around.
    pub fn prev(self) -> Self {
        match self {
            Tab::Holdings => Tab::Compliance,
            Tab::Disposals => Tab::Holdings,
            Tab::Income => Tab::Disposals,
            Tab::Tax => Tab::Income,
            Tab::Forms => Tab::Tax,
            Tab::Compliance => Tab::Forms,
        }
    }
}

/// READ-ONLY snapshot loaded once at unlock.
///
/// All fields are read-only projections of vault data; NONE are ever mutated after construction.
///
/// [R0-M2] `cli_config` is included because `btctax_cli::render::build_verify` needs it.
/// [R0-M3] `optimize_attested_set` is intentionally OMITTED — the viewer tabs do not consume it.
#[allow(dead_code)] // fields are consumed in Tasks 3–4
pub struct Snapshot {
    pub events: Vec<LedgerEvent>,
    pub state: LedgerState,
    pub config: ProjectionConfig,
    pub cli_config: CliConfig,
    pub profiles: BTreeMap<i32, TaxProfile>,
    pub tables: BundledTaxTables,
}

/// Top-level application state.
///
/// `handle_key` mutates ONLY UI navigation fields (`screen`, `tab`, `should_quit`,
/// `selected_year`, `unlock`). It NEVER mutates ledger data.
pub struct App {
    /// Path to the encrypted vault file (from CLI arg or default).
    pub vault_path: PathBuf,
    /// Unlock screen state (passphrase buffer + error).
    pub unlock: UnlockState,
    pub screen: Screen,
    pub tab: Tab,
    pub should_quit: bool,
    /// Populated after a successful `Session::open` + `build_snapshot`.
    #[allow(dead_code)] // read in Tasks 3–4
    pub snapshot: Option<Snapshot>,
    /// Tax year currently displayed in year-scoped tabs (Disposals/Income/Tax/Forms).
    /// Set to the latest year present in disposals/income after unlock; defaults to 2025.
    #[allow(dead_code)] // read in Tasks 3–4
    pub selected_year: i32,
}

impl App {
    pub fn new(vault_path: PathBuf) -> Self {
        App {
            vault_path,
            unlock: UnlockState::new(),
            screen: Screen::Unlock,
            tab: Tab::Holdings,
            should_quit: false,
            snapshot: None,
            selected_year: 2025,
        }
    }

    // ── Unlock flow ───────────────────────────────────────────────────────────

    /// Consume the passphrase buffer (via `mem::take` — never cloned [R0-I2]) and attempt
    /// to open the vault.  Updates `screen`/`snapshot`/`selected_year`/`unlock.error`
    /// according to the outcome.
    pub fn do_unlock(&mut self) {
        // Move passphrase out of buffer — NEVER clone [R0-I2/M7].
        // The taken `String` is moved into `Passphrase::new`; `self.unlock.buffer` is left empty.
        let pp = Passphrase::new(std::mem::take(&mut self.unlock.buffer));
        let outcome = unlock::attempt_open(&self.vault_path, pp);
        self.apply_open_outcome(outcome);
    }

    /// Apply an [`unlock::OpenOutcome`] to `App` state.
    pub fn apply_open_outcome(&mut self, outcome: unlock::OpenOutcome) {
        match outcome {
            unlock::OpenOutcome::Success(snapshot, year) => {
                self.snapshot = Some(*snapshot);
                self.selected_year = year;
                self.screen = Screen::Viewer;
                self.unlock.error = None;
            }
            unlock::OpenOutcome::Locked => {
                self.screen = Screen::Locked;
            }
            unlock::OpenOutcome::Error(msg) => {
                // Buffer already emptied by mem::take in do_unlock.
                self.unlock.error = Some(msg);
            }
        }
    }

    /// `BTCTAX_PASSPHRASE` fast-path: if the env var is set, open directly without a prompt.
    ///
    /// Mirrors the CLI's non-interactive behaviour.  Called once at startup before the event loop.
    pub fn try_env_passphrase(&mut self) {
        if let Ok(pp_str) = std::env::var("BTCTAX_PASSPHRASE") {
            // pp_str is moved into Passphrase::new — never cloned, never logged [R0-I2].
            let pp = Passphrase::new(pp_str);
            let outcome = unlock::attempt_open(&self.vault_path, pp);
            self.apply_open_outcome(outcome);
        }
    }
}
