//! Core application state: `Screen`, `Tab`, `Snapshot`, and `App`.
//!
//! STRICTLY READ-ONLY: this module MUST NOT call `Session::save()`, `persistence::append_*`,
//! any `btctax_cli::cmd::*` mutating command, or `Session::conn()`.

use btctax_adapters::BundledTaxTables;
use btctax_cli::CliConfig;
use btctax_core::{LedgerEvent, LedgerState, ProjectionConfig, TaxProfile};
use std::collections::BTreeMap;

/// Which top-level screen is active.
// `Locked` and `Viewer` are constructed in Task 2+; suppress the lint for the Task-1 skeleton.
#[allow(dead_code)]
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

/// READ-ONLY snapshot loaded once at unlock (Task 2).
///
/// All fields are read-only projections of vault data; NONE are ever mutated after construction.
///
/// [R0-M2] `cli_config` is included because `btctax_cli::render::build_verify` needs it.
/// [R0-M3] `optimize_attested_set` is intentionally OMITTED — the viewer tabs do not consume it.
// All fields are consumed in Tasks 2–4; suppress the Task-1 dead-code lint.
#[allow(dead_code)]
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
/// `selected_year`). It NEVER mutates ledger data.
pub struct App {
    pub screen: Screen,
    pub tab: Tab,
    pub should_quit: bool,
    /// Populated in Task 2 after a successful `Session::open`.
    #[allow(dead_code)] // read in Task 2+
    pub snapshot: Option<Snapshot>,
    /// Tax year currently displayed in year-scoped tabs (Disposals/Income/Tax/Forms).
    /// Defaults to 2025; Task 2 sets it to the latest year present in the snapshot.
    #[allow(dead_code)] // read in Tasks 3–4
    pub selected_year: i32,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            screen: Screen::Unlock,
            tab: Tab::Holdings,
            should_quit: false,
            snapshot: None,
            selected_year: 2025,
        }
    }
}
