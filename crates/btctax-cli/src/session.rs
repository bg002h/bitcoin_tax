//! `Session` wraps one open `btctax_store::Vault` and is the single seam every command opens. The
//! passphrase is ALWAYS a parameter — production resolves it in `main` (prompt/env); tests inject a
//! constructed `Passphrase`. `project()` runs the pure core projection over the bundled price dataset.
use crate::config::{self, CliConfig};
use crate::donation_details;
use crate::optimize_attest;
use crate::tax_profile;
use crate::CliError;
use btctax_adapters::{BundledPrices, BundledTaxTables};
use btctax_core::conventions::{tax_date, TRANSITION_DATE};
use btctax_core::persistence::{init_schema, load_all};
use btctax_core::{project, LedgerEvent, LedgerState, ProjectionConfig};
use btctax_core::{
    AllocLot, DonationDetails, EventId, EventPayload, LotMethod, PendingTransfer, Sat, TaxDate,
    TaxProfile, Usd, WalletId,
};
use btctax_store::{Passphrase, Vault};
use rusqlite::Connection;
use std::path::Path;

// ── Bulk link-transfer plan (bulk-link-transfer D1) ──────────────────────────
//
// The shared, READ-ONLY plan both the CLI (`cmd::reconcile::bulk_link_plan`) and the TUI priced
// preview compute from the HELD session. Modeled on `optimize_proposal`/`safe_harbor_residue`: a
// `&self` read helper that appends and persists NOTHING.

/// Time-frame selector for a bulk link-transfer plan. `Range` bounds are INCLUSIVE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    All,
    Year(i32),
    Range { from: TaxDate, to: TaxDate },
}

/// Filter narrowing which pending outbound transfers a bulk plan selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkFilter {
    pub frame: Frame,
    pub from_wallet: Option<WalletId>,
}

/// One enriched pending outbound transfer in a bulk link-transfer plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkLinkRow {
    pub out_event: EventId,
    pub date: TaxDate,
    /// [R0-N2] ALWAYS `Some` for a pending out (a wallet-less TransferOut never reaches
    /// `pending_reconciliation`); `Option` kept defensively.
    pub source_wallet: Option<WalletId>,
    pub principal_sat: Sat,
    /// `fmv_of(&prices, date, principal_sat)` [R0-M1]; advisory, `None` on missing price / overflow.
    pub usd_value: Option<Usd>,
    /// Σ leg `usd_basis` carried (over the principal+fee sats the legs cover); non-taxable → carries.
    pub basis_usd: Usd,
}

/// The read-only plan a bulk link-transfer would execute: the eligible/in-frame `included` rows, the
/// `skipped_same_wallet` rows (source == dest — a meaningless self-link), and the preview totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkLinkPlan {
    pub dest: WalletId,
    /// Eligible + in-frame + passes `from_wallet` + source != dest. Sorted by `date`.
    pub included: Vec<BulkLinkRow>,
    /// Source wallet == dest → cannot self-link to itself.
    pub skipped_same_wallet: Vec<BulkLinkRow>,
    pub total_sat: Sat,
    /// [R0-I2] Σ of the priced `usd_value`s — a FLOOR, always a real number.
    pub total_usd_value_floor: Usd,
    /// [R0-I2] rows priced `None` → render "≥ $X (N unavailable)" vs exact "$X".
    pub missing_price_count: usize,
    /// Σ `basis_usd` over `included`.
    pub total_basis_usd: Usd,
}

pub struct Session {
    vault: Vault,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

impl Session {
    /// Create a brand-new encrypted vault, then initialize the core event schema and the CLI config
    /// table, and persist. (`Vault::create` already saved once; we re-save after the DDL.)
    pub fn create(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        Self::from_fresh_vault(Vault::create(vault_path, pp)?)
    }

    /// Like `create`, but first clears a half-created vault (orphan key, no pgp/bak) under
    /// explicit `--repair` consent. Delegates to `Vault::repair` which refuses if a real or
    /// recoverable vault is present (see `Vault::repair` safety invariant).
    pub fn repair(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        Self::from_fresh_vault(Vault::repair(vault_path, pp)?)
    }

    /// Initialize the core schema + CLI config + tax profile table on a freshly-created vault,
    /// then persist.
    fn from_fresh_vault(mut vault: Vault) -> Result<Session, CliError> {
        init_schema(vault.conn())?;
        config::init_config_table(vault.conn())?;
        tax_profile::init_table(vault.conn())?;
        optimize_attest::init_table(vault.conn())?;
        donation_details::init_table(vault.conn())?;
        vault.save()?;
        Ok(Session { vault })
    }

    /// Open an existing vault (acquires the store single-instance lock; NFR7).
    pub fn open(vault_path: &Path, pp: &Passphrase) -> Result<Session, CliError> {
        Ok(Session {
            vault: Vault::open(vault_path, pp)?,
        })
    }

    /// Borrow the live in-memory SQLite handle (core appenders use interior mutability over `&Connection`).
    pub fn conn(&self) -> &Connection {
        self.vault.conn()
    }

    /// Persist the current DB image (encrypted, atomic; NFR2/NFR3).
    pub fn save(&mut self) -> Result<(), CliError> {
        self.vault.save()?;
        Ok(())
    }

    /// Snapshot the in-memory DB image (no disk I/O) for a possible `restore()` after a failed save.
    pub fn snapshot(&self) -> Result<Vec<u8>, CliError> {
        Ok(self.vault.snapshot()?)
    }

    /// Restore the in-memory DB from a prior `snapshot()` (no disk I/O). On `Err`, the in-memory DB
    /// is UNCHANGED and unsaved residue may still be live — the caller MUST latch, never swallow.
    pub fn restore(&mut self, image: &[u8]) -> Result<(), CliError> {
        self.vault.restore(image)?;
        Ok(())
    }

    /// Borrow the vault for store-level operations (`export_snapshot` / `backup_key`).
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// The persisted projection config (TP8 treatment + lot method); default = (c)+FIFO if unset.
    pub fn config(&self) -> Result<CliConfig, CliError> {
        config::read_config(self.conn())
    }

    /// The stored per-year `TaxProfile` for `year`, or `None` if none has been set.
    /// Robust to older vaults (calls `tax_profile::init_table` as a defensive guard).
    pub fn tax_profile(&self, year: i32) -> Result<Option<TaxProfile>, CliError> {
        tax_profile::get(self.conn(), year)
    }

    /// All stored `TaxProfile`s, sorted by year ascending.
    pub fn all_tax_profiles(
        &self,
    ) -> Result<std::collections::BTreeMap<i32, TaxProfile>, CliError> {
        tax_profile::all(self.conn())
    }

    /// All attested disposal `EventId`s (NFR4-stable `BTreeSet`; feeds `compliance_overlay`).
    /// Robust to older vaults (defensive `init_table` guard inside `attested_set`).
    pub fn optimize_attested_set(
        &self,
    ) -> Result<std::collections::BTreeSet<btctax_core::EventId>, CliError> {
        optimize_attest::attested_set(self.conn())
    }

    /// All stored `DonationDetails`, keyed by donation `EventId` (NFR4-stable `BTreeMap`).
    /// Robust to older vaults (defensive `init_table` guard inside `donation_details::all`).
    pub fn donation_details(
        &self,
    ) -> Result<std::collections::BTreeMap<EventId, DonationDetails>, CliError> {
        donation_details::all(self.conn())
    }

    /// Load all events and run the pure deterministic projection (NFR4) over the bundled daily-close
    /// dataset (§9.2). Returns the resolved `ProjectionConfig` too (so `verify` can display it).
    pub fn project(&self) -> Result<(LedgerState, ProjectionConfig), CliError> {
        let events = load_all(self.conn())?;
        let cfg = self.config()?.to_projection();
        let prices = BundledPrices::load()?;
        let state = project(&events, &prices, &cfg);
        Ok((state, cfg))
    }

    /// Single-load variant: loads events ONCE and returns them alongside the projection. Callers
    /// that need both the raw event log and the projected state (e.g. `verify`, `safe_harbor_attest`)
    /// use this to avoid the double `load_all` call that the `project()` + separate `load_all()`
    /// pattern incurs.
    pub fn load_events_and_project(
        &self,
    ) -> Result<(Vec<LedgerEvent>, LedgerState, ProjectionConfig), CliError> {
        let events = load_all(self.conn())?;
        let cfg = self.config()?.to_projection();
        let prices = BundledPrices::load()?;
        let state = project(&events, &prices, &cfg);
        Ok((events, state, cfg))
    }

    /// Recompute the Mode-1 optimizer proposal for `year` on the HELD session. READ-ONLY: appends and
    /// persists NOTHING (a clone-fold-discard recompute).
    ///
    /// The TUI editor's optimize-accept opener calls this to obtain a FRESH proposal (NFR4 — never
    /// trusts a stale one) WITHOUT opening a second `Session` (a second open would deadlock on the
    /// held VaultLock, and `cmd::optimize::accept` is forbidden to the editor for the same reason).
    /// Assembles `optimize_year`'s inputs exactly as `cmd::optimize::run`/`accept` do — events + config
    /// from this conn, bundled prices + tables, a FRESH `tax_profile(year)` read (not the cached snap),
    /// the attested set, and `proposal_made = tax_date(now, UtcOffset::UTC)` — and maps `OptimizeError`
    /// through the crate-internal `map_opt_err` (which is `pub(crate)` and not TUI-reachable). `now`
    /// is injected by the caller for determinism.
    pub fn optimize_proposal(
        &self,
        year: i32,
        now: time::OffsetDateTime,
    ) -> Result<btctax_core::OptimizeProposal, CliError> {
        let (events, _state, cfg) = self.load_events_and_project()?;
        let profile = self.tax_profile(year)?;
        let prices = BundledPrices::load()?;
        let tables = BundledTaxTables::load();
        let attested = self.optimize_attested_set()?;
        let proposal_made = tax_date(now, time::UtcOffset::UTC);
        btctax_core::optimize_year(
            &events,
            &prices,
            &cfg,
            year,
            profile.as_ref(),
            &tables,
            &attested,
            proposal_made,
        )
        .map_err(crate::cmd::optimize::map_opt_err)
    }

    /// READ-ONLY: the 2025-01-01 pre-2025 Universal residue as `AllocLot`s, plus the `pre2025_method`
    /// (`LotMethod`) it was computed under. Appends/persists NOTHING. The single source of the pre-2025
    /// subset, shared by `cmd::reconcile::safe_harbor_allocate` and the TUI allocate opener.
    ///
    /// Reads the config ONCE: `cfg.pre2025_method` is the recorded method returned to the caller, and
    /// `cfg.to_projection()` is the projection the residue is computed under — the two are STRUCTURALLY
    /// the same config read, so the returned method can never diverge from the residue's [R0-M1]. The
    /// pre-2025 subset keeps only imports whose tax-date `< 2025-01-01` plus ALL reconciliation decisions
    /// (which shape the residue), and DROPs any prior `SafeHarborAllocation` so the residue stays
    /// allocation-INDEPENDENT (matches `transition::universal_snapshot`).
    pub fn safe_harbor_residue(&self) -> Result<(Vec<AllocLot>, LotMethod), CliError> {
        let cfg = self.config()?;
        let pre2025_method = cfg.pre2025_method; // recorded field == the one used below
        let proj = cfg.to_projection();
        let pre2025: Vec<LedgerEvent> = load_all(self.conn())?
            .into_iter()
            .filter(|e| match &e.id {
                EventId::Import { .. } => {
                    tax_date(e.utc_timestamp, e.original_tz) < TRANSITION_DATE
                }
                _ => !matches!(e.payload, EventPayload::SafeHarborAllocation(_)),
            })
            .collect();
        let prices = BundledPrices::load()?;
        let residue = project(&pre2025, &prices, &proj);
        let lots = residue
            .lots
            .iter()
            .filter(|l| l.remaining_sat > 0)
            .map(|l| AllocLot {
                wallet: l.wallet.clone(),
                sat: l.remaining_sat,
                usd_basis: l.usd_basis,
                acquired_at: l.acquired_at,
                dual_loss_basis: l.dual_loss_basis,
                donor_acquired_at: l.donor_acquired_at,
            })
            .collect();
        Ok((lots, pre2025_method))
    }

    /// READ-ONLY: compute the bulk link-transfer plan (bulk-link-transfer D1). Selects over the
    /// PROJECTED `pending_reconciliation` (which already excludes already-decided / already-linked
    /// outs), enriches each with date / source wallet / principal / advisory USD value / carried
    /// basis, applies the frame + `from_wallet` filters, and routes `source == dest` rows to
    /// `skipped_same_wallet`. Appends and persists NOTHING; mirrors `safe_harbor_residue`.
    ///
    /// The USD value is `btctax_core::price::fmv_of(&prices, date, principal_sat)` [R0-M1] — the
    /// vetted checked helper (round_cents + overflow→`None`), NOT a hand-rolled `principal × price`.
    /// The total is the HONEST FLOOR [R0-I2]: `total_usd_value_floor` is Σ of the PRICED rows only,
    /// and `missing_price_count` records how many rows lacked a price, so the caller renders exact
    /// `$X` (when 0) or `≥ $X (N unavailable)`.
    pub fn bulk_link_transfer_plan(
        &self,
        filter: BulkFilter,
        dest: WalletId,
    ) -> Result<BulkLinkPlan, CliError> {
        let (events, state, _cfg) = self.load_events_and_project()?;
        let prices = BundledPrices::load()?;
        let index: std::collections::HashMap<EventId, &LedgerEvent> =
            events.iter().map(|e| (e.id.clone(), e)).collect();

        let enrich = |pt: &PendingTransfer| -> BulkLinkRow {
            let ev = index.get(&pt.event).copied();
            let date = ev
                .map(|e| tax_date(e.utc_timestamp, e.original_tz))
                .unwrap_or_else(|| {
                    // Defensive: a pending out always has an indexed source event; fall back to the
                    // epoch date rather than panic (mirrors the single link-transfer opener).
                    tax_date(
                        time::OffsetDateTime::from_unix_timestamp(0).unwrap(),
                        time::UtcOffset::UTC,
                    )
                });
            let source_wallet = ev.and_then(|e| e.wallet.clone());
            let usd_value = btctax_core::price::fmv_of(&prices, date, pt.principal_sat);
            let basis_usd: Usd = pt.legs.iter().map(|l| l.usd_basis).sum();
            BulkLinkRow {
                out_event: pt.event.clone(),
                date,
                source_wallet,
                principal_sat: pt.principal_sat,
                usd_value,
                basis_usd,
            }
        };

        let in_frame = |date: TaxDate| match &filter.frame {
            Frame::All => true,
            Frame::Year(y) => date.year() == *y,
            Frame::Range { from, to } => *from <= date && date <= *to,
        };

        let mut included: Vec<BulkLinkRow> = Vec::new();
        let mut skipped_same_wallet: Vec<BulkLinkRow> = Vec::new();
        for pt in &state.pending_reconciliation {
            let row = enrich(pt);
            if !in_frame(row.date) {
                continue;
            }
            if let Some(w) = &filter.from_wallet {
                if row.source_wallet.as_ref() != Some(w) {
                    continue;
                }
            }
            // Same-wallet guard: a self-link to the SAME wallet is meaningless — report, never link.
            if row.source_wallet.as_ref() == Some(&dest) {
                skipped_same_wallet.push(row);
            } else {
                included.push(row);
            }
        }
        included.sort_by_key(|r| r.date);

        let total_sat: Sat = included.iter().map(|r| r.principal_sat).sum();
        let total_usd_value_floor: Usd = included.iter().filter_map(|r| r.usd_value).sum();
        let missing_price_count = included.iter().filter(|r| r.usd_value.is_none()).count();
        let total_basis_usd: Usd = included.iter().map(|r| r.basis_usd).sum();

        Ok(BulkLinkPlan {
            dest,
            included,
            skipped_same_wallet,
            total_sat,
            total_usd_value_floor,
            missing_price_count,
            total_basis_usd,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_store::Passphrase;

    fn pp() -> Passphrase {
        Passphrase::new("test-pass".into())
    }

    #[test]
    fn create_then_open_round_trips_over_a_temp_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        {
            let _s = Session::create(&vault, &pp()).unwrap(); // schema + config table initialized + saved
        }
        // Re-open with the same passphrase: an empty ledger projects cleanly.
        let s = Session::open(&vault, &pp()).unwrap();
        let (state, _cfg) = s.project().unwrap();
        assert!(state.lots.is_empty());
        assert!(state.blockers.is_empty());
    }

    /// `Session::snapshot`/`restore` delegate to the vault and revert an in-memory mutation
    /// without touching disk (the wrapper the persist layer uses for save-rollback).
    #[test]
    fn session_snapshot_restore_reverts_in_memory_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        let mut s = Session::create(&vault, &pp()).unwrap();
        s.conn().execute("CREATE TABLE t (x INTEGER)", []).unwrap();
        s.save().unwrap();

        let snap = s.snapshot().unwrap();
        s.conn().execute("INSERT INTO t VALUES (7)", []).unwrap();
        let n: i64 = s
            .conn()
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "pre-restore: the inserted row is present in memory");

        let before = std::fs::read(&vault).unwrap();
        s.restore(&snap).unwrap();
        let after = std::fs::read(&vault).unwrap();
        assert_eq!(before, after, "restore must not write the vault file");

        let n: i64 = s
            .conn()
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "restore must revert the in-memory insert");
    }

    #[test]
    fn wrong_passphrase_is_surfaced_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Session::create(&vault, &pp()).unwrap();
        let err = Session::open(&vault, &Passphrase::new("nope".into())).unwrap_err();
        assert!(matches!(
            err,
            CliError::Store(btctax_store::StoreError::WrongPassphrase)
        ));
    }

    /// `load_events_and_project` must return the same (events, state, config) triple as calling
    /// `load_all` + `project` separately. Verifies the single-load contract (no double DB round-trip).
    #[test]
    fn load_events_and_project_matches_separate_calls() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Session::create(&vault, &pp()).unwrap();
        let s = Session::open(&vault, &pp()).unwrap();

        let (events, state, cfg) = s.load_events_and_project().unwrap();

        // Reference path: separate load_all + project calls.
        let events2 = btctax_core::persistence::load_all(s.conn()).unwrap();
        let (state2, cfg2) = s.project().unwrap();

        assert_eq!(events.len(), events2.len(), "event count must match");
        assert_eq!(state.lots.len(), state2.lots.len(), "lots count must match");
        assert_eq!(
            state.blockers.len(),
            state2.blockers.len(),
            "blocker count must match"
        );
        assert_eq!(cfg, cfg2, "ProjectionConfig must match");
    }

    /// `Session::optimize_proposal` recomputes the Mode-1 proposal on the HELD session (READ-ONLY,
    /// no second open). For a 2025 year with two same-wallet lots and a 500k sale, the FIFO baseline
    /// consumes the cheaper lot A (higher gain); the optimizer prefers the dearer lot B → a
    /// per-disposal row whose proposed_selection differs from current_selection, with `delta ≤ 0`.
    #[test]
    fn optimize_proposal_recomputes_a_persistable_proposal_on_held_session() {
        use btctax_core::event::{
            Acquire, BasisSource, DisposeKind, EventPayload, LedgerEvent, OutflowClass,
            ReclassifyOutflow, TransferOut,
        };
        use btctax_core::identity::{Source, SourceRef};
        use btctax_core::persistence::{append_decision, append_import_batch};
        use btctax_core::{Carryforward, EventId, FilingStatus, TaxProfile, WalletId};
        use rust_decimal_macros::dec;
        use time::{OffsetDateTime, UtcOffset};

        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("vault.pgp");
        Session::create(&vault, &pp()).unwrap();
        let mut s = Session::open(&vault, &pp()).unwrap();

        let wallet = Some(WalletId::Exchange {
            provider: "River".to_string(),
            account: "main".to_string(),
        });
        let lot_a = EventId::import(Source::River, SourceRef::new("op-lot-a"));
        let lot_b = EventId::import(Source::River, SourceRef::new("op-lot-b"));
        let to_id = EventId::import(Source::River, SourceRef::new("op-sell"));
        let ta = OffsetDateTime::from_unix_timestamp(1_739_000_000).unwrap();
        let tb = OffsetDateTime::from_unix_timestamp(1_741_000_000).unwrap();
        let tc = OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap();
        let td = OffsetDateTime::from_unix_timestamp(1_748_100_000).unwrap();
        let batch = vec![
            LedgerEvent {
                id: lot_a.clone(),
                utc_timestamp: ta,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(30000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: lot_b.clone(),
                utc_timestamp: tb,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::Acquire(Acquire {
                    sat: 1_000_000,
                    usd_cost: dec!(50000),
                    fee_usd: dec!(0),
                    basis_source: BasisSource::ExchangeProvided,
                }),
            },
            LedgerEvent {
                id: to_id.clone(),
                utc_timestamp: tc,
                original_tz: UtcOffset::UTC,
                wallet: wallet.clone(),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 500_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
        ];
        append_import_batch(s.conn(), &batch).unwrap();
        let ro = EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: to_id.clone(),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: dec!(30000),
            fee_usd: None,
            donee: None,
        });
        append_decision(s.conn(), ro, td, UtcOffset::UTC, None).unwrap();
        let profile = TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: dec!(100000),
            magi_excluding_crypto: dec!(100000),
            qualified_dividends_and_other_pref_income: dec!(0),
            other_net_capital_gain: dec!(0),
            capital_loss_carryforward_in: Carryforward::default(),
            w2_ss_wages: dec!(0),
            w2_medicare_wages: dec!(0),
            schedule_c_expenses: dec!(0),
        };
        crate::tax_profile::set(s.conn(), 2025, &profile).unwrap();
        s.save().unwrap();

        let now = OffsetDateTime::from_unix_timestamp(1_752_000_000).unwrap();
        let proposal = s.optimize_proposal(2025, now).unwrap();
        assert!(
            proposal.delta <= dec!(0),
            "delta must be ≤ 0 (baseline-seeded)"
        );
        let row = proposal
            .per_disposal
            .iter()
            .find(|d| d.disposal == to_id)
            .expect("the 2025 sale must be in the proposal");
        assert_ne!(
            row.proposed_selection, row.current_selection,
            "the optimizer must propose the dearer lot (a change from FIFO)"
        );
    }
}
