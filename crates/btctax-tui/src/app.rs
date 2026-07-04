//! Core application state: `Screen`, `Tab`, `Snapshot`, and `App`.
//!
//! never writes the vault or any decrypted image of it; writes only the four form CSVs
//! via `export.rs` on explicit user confirmation. This module performs no writes.

use crate::export::ExportConfirmState;
use crate::unlock;
use crate::unlock::UnlockState;
use btctax_adapters::BundledTaxTables;
use btctax_cli::CliConfig;
use btctax_core::{DonationDetails, EventId, LedgerEvent, LedgerState, TaxProfile};
use btctax_store::Passphrase;
use ratatui::widgets::TableState;
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
pub struct Snapshot {
    pub events: Vec<LedgerEvent>,
    pub state: LedgerState,
    pub cli_config: CliConfig,
    pub profiles: BTreeMap<i32, TaxProfile>,
    pub tables: BundledTaxTables,
    pub donation_details: BTreeMap<EventId, DonationDetails>,
    /// Disposals flagged as estimated-FMV proceeds by the bulk-reclassify-outflow path (Cycle 5),
    /// keyed by `transfer_out_event` (== `Disposal.event`); value = the `date_marked` provenance
    /// stamp. The Disposals tab renders an `[est]` marker on flagged rows; the Compliance tab shows
    /// an advisory count. Loaded via `Session::bulk_estimated()` [R0-M1], never `conn()` directly.
    pub bulk_estimated: BTreeMap<EventId, String>,
}

/// Top-level application state.
///
/// `handle_key` mutates ONLY UI navigation fields (`screen`, `tab`, `should_quit`,
/// `selected_year`, `unlock`, `export_modal`, `export_status`). It NEVER mutates ledger data.
///
/// Kept `pub(crate)` so the viewer's internal surface does not leak into the editor crate —
/// the editor uses `btctax_tui::unlock::open_session` and `tabs::*::render` instead.
pub(crate) struct App {
    /// Path to the encrypted vault file (from CLI arg or default).
    pub vault_path: PathBuf,
    /// Unlock screen state (passphrase buffer + error).
    pub unlock: UnlockState,
    pub screen: Screen,
    pub tab: Tab,
    pub should_quit: bool,
    /// Populated after a successful `Session::open` + `build_snapshot`.
    pub snapshot: Option<Snapshot>,
    /// Tax year currently displayed in year-scoped tabs (Disposals/Income/Tax/Forms).
    /// Set to the latest year present in disposals/income after unlock; defaults to 2025.
    pub selected_year: i32,
    /// Per-tab table scroll states (mutated by scroll helpers in main.rs).
    pub holdings_state: TableState,
    pub disposals_state: TableState,
    pub income_state: TableState,
    /// Scroll/selection state for the Form 8949 table in the Forms tab.
    pub forms_state: TableState,
    /// Export confirmation modal state. `Some` while the modal is open; `None` otherwise.
    /// Set by the `e` keybinding; cleared on Enter (execute) or Esc (cancel).
    pub export_modal: Option<ExportConfirmState>,
    /// One-line export status shown in the footer after a completed export or error.
    /// Cleared on the next non-modal key press.
    pub export_status: Option<String>,
}

impl App {
    pub(crate) fn new(vault_path: PathBuf) -> Self {
        App {
            vault_path,
            unlock: UnlockState::new(),
            screen: Screen::Unlock,
            tab: Tab::Holdings,
            should_quit: false,
            snapshot: None,
            selected_year: 2025,
            holdings_state: TableState::default(),
            disposals_state: TableState::default(),
            income_state: TableState::default(),
            forms_state: TableState::default(),
            export_modal: None,
            export_status: None,
        }
    }

    // ── Unlock flow ───────────────────────────────────────────────────────────

    /// Consume the passphrase buffer (via `mem::take` — never cloned [R0-I2]) and attempt
    /// to open the vault.  Updates `screen`/`snapshot`/`selected_year`/`unlock.error`
    /// according to the outcome.
    pub(crate) fn do_unlock(&mut self) {
        // Move passphrase out of buffer — NEVER clone [R0-I2/M7].
        // The taken `String` is moved into `Passphrase::new`; `self.unlock.buffer` is left empty.
        let pp = Passphrase::new(std::mem::take(&mut self.unlock.buffer));
        let outcome = unlock::attempt_open(&self.vault_path, pp);
        self.apply_open_outcome(outcome);
    }

    /// Apply an [`unlock::OpenOutcome`] to `App` state.
    pub(crate) fn apply_open_outcome(&mut self, outcome: unlock::OpenOutcome) {
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
    pub(crate) fn try_env_passphrase(&mut self) {
        if let Ok(pp_str) = std::env::var("BTCTAX_PASSPHRASE") {
            // pp_str is moved into Passphrase::new — never cloned, never logged [R0-I2].
            let pp = Passphrase::new(pp_str);
            let outcome = unlock::attempt_open(&self.vault_path, pp);
            self.apply_open_outcome(outcome);
        }
    }
}
