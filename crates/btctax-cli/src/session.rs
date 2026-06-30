//! `Session` wraps one open `btctax_store::Vault` and is the single seam every command opens. The
//! passphrase is ALWAYS a parameter — production resolves it in `main` (prompt/env); tests inject a
//! constructed `Passphrase`. `project()` runs the pure core projection over the bundled price dataset.
use crate::config::{self, CliConfig};
use crate::tax_profile;
use crate::CliError;
use btctax_adapters::BundledPrices;
use btctax_core::persistence::{init_schema, load_all};
use btctax_core::TaxProfile;
use btctax_core::{project, LedgerEvent, LedgerState, ProjectionConfig};
use btctax_store::{Passphrase, Vault};
use rusqlite::Connection;
use std::path::Path;

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
}
