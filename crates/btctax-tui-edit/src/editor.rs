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
    BulkIncomeFlowState, BulkIncomeModalState, BulkLinkFlowState, BulkLinkModalState,
    BulkReclassifyOutflowFlowState, BulkReclassifyOutflowModalState, BulkResolveFlowState,
    BulkResolveModalState, BulkStiFlowState, BulkStiModalState, BulkVoidFlowState,
    BulkVoidModalState, ClassifyInboundFlowState, ClassifyInboundModalState, ClassifyRawFlowState,
    ClassifyRawModalState, LinkTransferFlowState, LinkTransferModalState,
    MatchSelfTransfersFlowState, MatchSelfTransfersModalState, MutationModalState,
    OptimizeAcceptFlowState, OptimizeAcceptModalState, ProfileFormState, ReclassifyIncomeFlowState,
    ReclassifyIncomeModalState, ReclassifyOutflowFlowState, ReclassifyOutflowModalState,
    ResolveConflictFlowState, ResolveConflictModalState, SafeHarborAllocateFlowState,
    SafeHarborAllocateModalState, SafeHarborAttestFlowState, SelectLotsFlowState,
    SelectLotsModalState, SetDonationDetailsFlowState, SetDonationDetailsModalState,
    SetFmvFlowState, SetFmvModalState, VoidFlowState, VoidModalState,
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
    /// `?` help overlay open (Browse screen). Pure runtime UI state (no serde). Modal while open.
    pub help_open: bool,
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
    /// Full link-transfer flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: link_transfer_modal (modal layer) → link_transfer_flow (flow layer) → ...
    pub link_transfer_flow: Option<LinkTransferFlowState>,
    /// Link-transfer confirmation modal.  `Some` while awaiting Enter/Esc.
    pub link_transfer_modal: Option<LinkTransferModalState>,
    /// Full classify-raw flow state.  `Some` while the flow is open.
    ///
    /// Dispatch order: classify_raw_modal (modal layer) → classify_raw_flow (flow layer) → ...
    pub classify_raw_flow: Option<ClassifyRawFlowState>,
    /// Classify-raw confirmation modal.  `Some` while awaiting Enter/Esc.
    pub classify_raw_modal: Option<ClassifyRawModalState>,
    /// Full safe-harbor-attest flow state.  `Some` while the flow is open.
    ///
    /// No separate modal — TypedWord step is the gate (layer 9 only) [R0-M4].
    pub safe_harbor_attest_flow: Option<SafeHarborAttestFlowState>,
    /// Full resolve-conflict flow state (chunk 4b, D3).  `Some` while the flow is open.
    ///
    /// Dispatch order: resolve_conflict_modal (modal layer) → resolve_conflict_flow (flow layer) → ...
    pub resolve_conflict_flow: Option<ResolveConflictFlowState>,
    /// Resolve-conflict confirmation modal.  `Some` while awaiting Enter/Esc.
    pub resolve_conflict_modal: Option<ResolveConflictModalState>,
    /// Full optimize-accept flow state (chunk 4b, D4).  `Some` while the flow is open.
    ///
    /// Dispatch order: optimize_accept_modal (modal layer) → optimize_accept_flow (flow layer) → ...
    pub optimize_accept_flow: Option<OptimizeAcceptFlowState>,
    /// Optimize-accept confirmation modal.  `Some` while awaiting Enter/Esc.
    pub optimize_accept_modal: Option<OptimizeAcceptModalState>,
    /// Full safe-harbor-allocate flow state (chunk 5, D2).  `Some` while the flow is open.
    ///
    /// Dispatch order: safe_harbor_allocate_modal (modal layer) → safe_harbor_allocate_flow (flow
    /// layer). Creation is REVOCABLE, so the confirmation is a plain modal — NO typed-word gate.
    pub safe_harbor_allocate_flow: Option<SafeHarborAllocateFlowState>,
    /// Safe-harbor-allocate confirmation modal.  `Some` while awaiting Enter/Esc.
    pub safe_harbor_allocate_modal: Option<SafeHarborAllocateModalState>,
    /// Full bulk-link-transfer flow state (bulk-link-transfer D3).  `Some` while the flow is open.
    ///
    /// Dispatch order: bulk_link_modal (modal layer) → bulk_link_flow (flow layer). Creation is
    /// REVOCABLE (each link voidable via `v`), so the confirmation is a plain modal — NO typed-word.
    pub bulk_link_flow: Option<BulkLinkFlowState>,
    /// Bulk-link-transfer confirmation modal.  `Some` while awaiting Enter/Esc.
    pub bulk_link_modal: Option<BulkLinkModalState>,
    /// Full bulk classify-inbound-self-transfer flow state (bulk-classify-inbound-self-transfer D3).
    ///
    /// Dispatch order: bulk_sti_modal (modal layer) → bulk_sti_flow (flow layer). Creation is
    /// REVOCABLE (each classification voidable via `v`), so the confirmation is a plain modal — NO
    /// typed-word.
    pub bulk_sti_flow: Option<BulkStiFlowState>,
    /// Bulk STI confirmation modal.  `Some` while awaiting Enter/Esc.
    pub bulk_sti_modal: Option<BulkStiModalState>,
    /// Full bulk classify-inbound-income flow state (bulk-classify-inbound-income, Cycle 4).
    ///
    /// Dispatch order: bulk_income_modal (modal layer) → bulk_income_flow (flow layer). Creation is
    /// REVOCABLE (each classification voidable via `v`, matching the STI tier), so the confirmation is
    /// a plain modal — NO typed-word.
    pub bulk_income_flow: Option<BulkIncomeFlowState>,
    /// Bulk classify-income confirmation modal.  `Some` while awaiting Enter/Esc.
    pub bulk_income_modal: Option<BulkIncomeModalState>,
    /// Full bulk resolve-conflict flow state (bulk-resolve-conflict D3).  `Some` while open.
    ///
    /// Dispatch order: bulk_resolve_modal (modal layer) → bulk_resolve_flow (flow layer). Each
    /// resolution is NON-REVOCABLE (`SupersedeImport`/`RejectImport` excluded from
    /// `is_revocable_payload`), so the confirm is Tier-B (non-revocable warning) — NOT typed-word.
    pub bulk_resolve_flow: Option<BulkResolveFlowState>,
    /// Bulk resolve-conflict confirmation modal.  `Some` while awaiting Enter/Esc.
    pub bulk_resolve_modal: Option<BulkResolveModalState>,
    /// Full bulk-void flow state (bulk-void D3).  `Some` while open.
    ///
    /// Dispatch order: bulk_void_modal (modal layer) → bulk_void_flow (flow layer). Each void is
    /// NON-REVOCABLE (a `VoidDecisionEvent` is excluded from `is_revocable_payload`) AND high
    /// blast-radius, so the confirm is Tier-B (red, prominent warning) — NOT typed-word.
    pub bulk_void_flow: Option<BulkVoidFlowState>,
    /// Bulk-void confirmation modal.  `Some` while awaiting Enter/Esc.
    pub bulk_void_modal: Option<BulkVoidModalState>,
    /// Full bulk reclassify-outflow flow state (bulk-reclassify-outflow, Cycle 5).  `Some` while open.
    ///
    /// Dispatch order: bulk_reclassify_outflow_modal (modal layer) → bulk_reclassify_outflow_flow
    /// (flow layer). Creation is REVOCABLE (each `ReclassifyOutflow` voidable via `v`), so the confirm
    /// is a plain modal + a prominent ESTIMATED-proceeds warning — NO typed-word.
    pub bulk_reclassify_outflow_flow: Option<BulkReclassifyOutflowFlowState>,
    /// Bulk reclassify-outflow confirmation modal.  `Some` while awaiting Enter/Esc.
    pub bulk_reclassify_outflow_modal: Option<BulkReclassifyOutflowModalState>,
    /// Full match-self-transfers flow state (self-transfer-passthrough C3).  `Some` while open.
    ///
    /// Dispatch order: match_self_transfers_modal (modal layer) → match_self_transfers_flow (flow
    /// layer). The DROP is REVOCABLE (voidable via `v`); the modal is a plain confirm — NO typed-word.
    pub match_self_transfers_flow: Option<MatchSelfTransfersFlowState>,
    /// Match-self-transfers confirmation modal.  `Some` while awaiting Enter/Esc.
    pub match_self_transfers_modal: Option<MatchSelfTransfersModalState>,
    /// Residue latch [R0-C1]: set to `true` when `persist_safe_harbor_attest` returns Err.
    ///
    /// While `true`, ALL mutating openers (p/c/o/r/f/v/s/d/a) refuse with the
    /// quit-first latch status. Cleared only by quitting (discards in-memory residue).
    pub attest_save_failed: bool,
    /// Sibling residue latch [save-rollback]: set ONLY by `on_persist_error`'s `ResidueLive` arm,
    /// i.e. when one of the 8 rollback persist fns failed to save AND the in-memory rollback ALSO
    /// failed, so unsaved residue is live. Like `attest_save_failed`, while `true` every mutating
    /// opener refuses (via `residue_latch_status`) until quit (which discards the residue).
    pub rollback_failed: bool,
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
            help_open: false,
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
            link_transfer_flow: None,
            link_transfer_modal: None,
            classify_raw_flow: None,
            classify_raw_modal: None,
            safe_harbor_attest_flow: None,
            resolve_conflict_flow: None,
            resolve_conflict_modal: None,
            optimize_accept_flow: None,
            optimize_accept_modal: None,
            safe_harbor_allocate_flow: None,
            safe_harbor_allocate_modal: None,
            bulk_link_flow: None,
            bulk_link_modal: None,
            bulk_sti_flow: None,
            bulk_sti_modal: None,
            bulk_income_flow: None,
            bulk_income_modal: None,
            bulk_resolve_flow: None,
            bulk_resolve_modal: None,
            bulk_void_flow: None,
            bulk_void_modal: None,
            bulk_reclassify_outflow_flow: None,
            bulk_reclassify_outflow_modal: None,
            match_self_transfers_flow: None,
            match_self_transfers_modal: None,
            attest_save_failed: false,
            rollback_failed: false,
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
