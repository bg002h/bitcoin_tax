//! Editor application state: `EditorScreen`, `EditorApp`.
//!
//! # Guarantee
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! # VaultLock exclusivity
//! `Session::open` acquires the store's single-instance lock for the vault's lifetime
//! (session.rs:53–58, vault.rs:137–142). While the editor runs, the CLI (or a viewer)
//! **cannot** open the vault — it gets `StoreError::Locked`. Conversely, the editor
//! shows the `Locked` screen when something else holds the lock. There is no
//! concurrent-writer case to reason about; this is the only safe lifecycle for a
//! session-holding TUI editor.

use crate::edit::form::{
    ClassifyInboundFlowState, ClassifyInboundModalState, MutationModalState, ProfileFormState,
    ReclassifyIncomeFlowState, ReclassifyIncomeModalState, ReclassifyOutflowFlowState,
    ReclassifyOutflowModalState, SelectLotsFlowState, SelectLotsModalState,
    SetDonationDetailsFlowState, SetDonationDetailsModalState, SetFmvFlowState, SetFmvModalState,
    VoidFlowState, VoidModalState,
};
use btctax_cli::Session;
use btctax_store::Passphrase;
use btctax_tui::{
    app::{Snapshot, Tab},
    unlock::{open_session, SessionOpenOutcome, UnlockState},
};
use ratatui::widgets::TableState;
use std::path::PathBuf;

/// Which top-level screen is active in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorScreen {
    #[default]
    Unlock,
    Locked,
    Browse,
}

/// Top-level editor application state.
///
/// `handle_key` mutates ONLY UI navigation fields until a confirmed mutation fires
/// `edit::persist::persist_tax_profile` (the ONE place mutations are permitted, per D3).
///
/// # VaultLock exclusivity
/// The `session` field holds the live `Session` — and therefore the vault's `VaultLock` —
/// for the entire TUI session. The CLI and viewer cannot open the same vault concurrently
/// (`StoreError::Locked`). There is no concurrent-writer case.
pub struct EditorApp {
    /// Path to the encrypted vault file.
    pub vault_path: PathBuf,
    /// Unlock screen state (passphrase buffer + error).
    ///
    /// Same masked-input discipline as the viewer's `UnlockState`:
    /// pre-allocated, never-reallocating buffer; move via `mem::take`; never clone.
    pub unlock: UnlockState,
    pub screen: EditorScreen,
    pub tab: Tab,
    pub should_quit: bool,
    /// The LIVE session — held for the whole TUI session.
    ///
    /// # VaultLock exclusivity (documented concurrency story)
    /// Holding this `Session` keeps `VaultLock` acquired for the editor's lifetime.
    /// The CLI or another viewer/editor cannot open the same vault while we hold this
    /// (they get `StoreError::Locked`). Conversely, this field is `None` before unlock
    /// and we show `EditorScreen::Locked` when `open_session` returns `SessionOpenOutcome::Locked`.
    /// There is no concurrent-writer scenario: the lock is exclusive and file-level.
    ///
    /// `Some` iff `snapshot` is `Some`.
    pub session: Option<Session>,
    /// Read-only snapshot, re-projected after every confirmed mutation.
    /// Built by `btctax_tui::unlock::build_snapshot(&session)` — same fn as the viewer.
    pub snapshot: Option<Snapshot>,
    /// Tax year currently displayed in year-scoped tabs (Disposals / Income / Tax / Forms).
    /// Set to the latest year present in disposals/income after unlock; defaults to 2025.
    pub selected_year: i32,
    /// Per-tab table scroll states (mutated by scroll helpers in `main.rs`).
    pub holdings_state: TableState,
    pub disposals_state: TableState,
    pub income_state: TableState,
    pub forms_state: TableState,
    /// The tax-profile form. `Some` while the form is open.
    /// Pre-populated from the selected year's existing profile when present.
    pub profile_form: Option<ProfileFormState>,
    /// The per-mutation confirmation modal. `Some` while awaiting Enter/Esc.
    ///
    /// Modal dispatch precedes form and screen dispatch (the R0-M4 lesson —
    /// Esc must never fall through to a quit arm).
    pub mutation_modal: Option<MutationModalState>,
    /// Full classify-inbound flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: modal → classify_inbound_modal → classify_inbound_flow
    /// → profile form → screen.  Guarantees `q` / Esc never fall through to a
    /// quit arm while the flow (or its modal) is blocking.
    pub classify_inbound_flow: Option<ClassifyInboundFlowState>,
    /// Classify-inbound confirmation modal.  `Some` while awaiting Enter/Esc.
    /// Always set via the `IncomeForm` / `GiftForm` Enter path in the flow.
    pub classify_inbound_modal: Option<ClassifyInboundModalState>,
    /// Full reclassify-outflow flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: modal → reclassify_outflow_modal → reclassify_outflow_flow
    /// → classify-inbound modal → classify-inbound flow → form → screen.
    /// State invariant: at most one flow `Some` at any time; at most one modal `Some`.
    pub reclassify_outflow_flow: Option<ReclassifyOutflowFlowState>,
    /// Reclassify-outflow confirmation modal.  `Some` while awaiting Enter/Esc.
    pub reclassify_outflow_modal: Option<ReclassifyOutflowModalState>,
    /// Full reclassify-income flow state.  `Some` while the flow is open.
    pub reclassify_income_flow: Option<ReclassifyIncomeFlowState>,
    /// Reclassify-income confirmation modal.  `Some` while awaiting Enter/Esc.
    pub reclassify_income_modal: Option<ReclassifyIncomeModalState>,
    /// Full set-fmv flow state.  `Some` while the flow is open.
    pub set_fmv_flow: Option<SetFmvFlowState>,
    /// Set-fmv confirmation modal.  `Some` while awaiting Enter/Esc.
    pub set_fmv_modal: Option<SetFmvModalState>,
    /// Full void flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: void_modal (layer 6) → void_flow (flow layer) → ...
    pub void_flow: Option<VoidFlowState>,
    /// Void confirmation modal.  `Some` while awaiting Enter/Esc.
    pub void_modal: Option<VoidModalState>,
    /// Full select-lots flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: select_lots_modal (layer 7) → select_lots_flow (flow layer, layer 9) → ...
    pub select_lots_flow: Option<SelectLotsFlowState>,
    /// Select-lots confirmation modal.  `Some` while awaiting Enter/Esc.
    pub select_lots_modal: Option<SelectLotsModalState>,
    /// Full set-donation-details flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: set_donation_details_modal (layer 8) → set_donation_details_flow (flow layer) → ...
    pub set_donation_details_flow: Option<SetDonationDetailsFlowState>,
    /// Set-donation-details confirmation modal.  `Some` while awaiting Enter/Esc.
    pub set_donation_details_modal: Option<SetDonationDetailsModalState>,
    /// One-line status (saved / error), shown in the footer.
    /// Cleared on the next non-modal key press (mirrors the viewer's `export_status`
    /// semantics, app.rs:140 [R0-N5]).
    pub status: Option<String>,
}

impl EditorApp {
    pub fn new(vault_path: PathBuf) -> Self {
        EditorApp {
            vault_path,
            unlock: UnlockState::new(),
            screen: EditorScreen::Unlock,
            tab: Tab::Holdings,
            should_quit: false,
            session: None,
            snapshot: None,
            selected_year: 2025,
            holdings_state: TableState::default(),
            disposals_state: TableState::default(),
            income_state: TableState::default(),
            forms_state: TableState::default(),
            profile_form: None,
            mutation_modal: None,
            classify_inbound_flow: None,
            classify_inbound_modal: None,
            reclassify_outflow_flow: None,
            reclassify_outflow_modal: None,
            reclassify_income_flow: None,
            reclassify_income_modal: None,
            set_fmv_flow: None,
            set_fmv_modal: None,
            void_flow: None,
            void_modal: None,
            select_lots_flow: None,
            select_lots_modal: None,
            set_donation_details_flow: None,
            set_donation_details_modal: None,
            status: None,
        }
    }

    /// Consume the passphrase buffer and attempt to open the vault.
    ///
    /// On success, stores BOTH the live session (holding the VaultLock for the editor's
    /// lifetime) and the built snapshot, then transitions to `EditorScreen::Browse`.
    ///
    /// Passphrase hygiene (identical to the viewer's `do_unlock`):
    /// - Buffer taken via `std::mem::take` — NEVER cloned [R0-I2/M7].
    /// - Moved into `Passphrase::new`; the store's `Passphrase::Drop` zeroizes the copy.
    /// - Raw chars never logged or rendered; `open_session` drops `pp` before
    ///   `build_snapshot` [R0-M5].
    pub fn do_unlock(&mut self) {
        let pp = Passphrase::new(std::mem::take(&mut self.unlock.buffer));
        match open_session(&self.vault_path, pp) {
            SessionOpenOutcome::Success {
                session,
                snapshot,
                year,
            } => {
                // Unbox: `Session` is `Sized`; stored directly (not boxed).
                self.session = Some(*session);
                self.snapshot = Some(*snapshot);
                self.selected_year = year;
                self.screen = EditorScreen::Browse;
                self.unlock.error = None;
            }
            SessionOpenOutcome::Locked => {
                self.screen = EditorScreen::Locked;
            }
            SessionOpenOutcome::Error(msg) => {
                // Buffer already emptied by `mem::take` above.
                self.unlock.error = Some(msg);
            }
        }
    }

    /// `BTCTAX_PASSPHRASE` fast-path: open directly when the env var is set.
    ///
    /// Mirrors the viewer's non-interactive behaviour. Called once at startup.
    pub fn try_env_passphrase(&mut self) {
        if let Ok(pp_str) = std::env::var("BTCTAX_PASSPHRASE") {
            let pp = Passphrase::new(pp_str);
            match open_session(&self.vault_path, pp) {
                SessionOpenOutcome::Success {
                    session,
                    snapshot,
                    year,
                } => {
                    self.session = Some(*session);
                    self.snapshot = Some(*snapshot);
                    self.selected_year = year;
                    self.screen = EditorScreen::Browse;
                    self.unlock.error = None;
                }
                SessionOpenOutcome::Locked => {
                    self.screen = EditorScreen::Locked;
                }
                SessionOpenOutcome::Error(msg) => {
                    self.unlock.error = Some(msg);
                }
            }
        }
    }
}
