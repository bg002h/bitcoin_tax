//! CLI configuration — projection settings (TP8 fee treatment + lot method) stored in the vault's
//! `cli_config` key-value side-table. Defaults match `ProjectionConfig::default()` (TreatmentC + FIFO).
use crate::CliError;
use btctax_core::{FeeTreatment, LotMethod, ProjectionConfig};
use rusqlite::Connection;
use rusqlite::OptionalExtension;

/// Session-wide projection configuration loaded from the vault's `cli_config` table.
/// Mirrors `btctax_core::ProjectionConfig` so it can be read/written independently of the core.
#[derive(Debug, Clone)]
pub struct CliConfig {
    pub self_transfer_fee: FeeTreatment,
    pub lot_method: LotMethod,
}

impl Default for CliConfig {
    fn default() -> Self {
        // Mirror ProjectionConfig::default() — TP8 mandates TreatmentC; never flip to TreatmentB.
        let p = ProjectionConfig::default();
        CliConfig {
            self_transfer_fee: p.self_transfer_fee,
            lot_method: p.lot_method,
        }
    }
}

impl CliConfig {
    /// Convert to the core projection type for use in `project()`.
    pub fn to_projection(&self) -> ProjectionConfig {
        ProjectionConfig {
            self_transfer_fee: self.self_transfer_fee,
            lot_method: self.lot_method,
        }
    }
}

/// Create the `cli_config` key-value side-table if it does not exist.
/// Called by `Session::create` immediately after `init_schema`.
pub fn init_config_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cli_config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;
    Ok(())
}

/// Read the persisted config, returning defaults for any unset keys.
/// Called by `Session::config()`.
pub fn read_config(conn: &Connection) -> Result<CliConfig, CliError> {
    let mut cfg = CliConfig::default();

    let fee: Option<String> = conn
        .query_row(
            "SELECT value FROM cli_config WHERE key = 'self_transfer_fee'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(f) = fee {
        match f.as_str() {
            "c" => cfg.self_transfer_fee = FeeTreatment::TreatmentC,
            "b" => cfg.self_transfer_fee = FeeTreatment::TreatmentB,
            _ => {}
        }
    }

    let method: Option<String> = conn
        .query_row(
            "SELECT value FROM cli_config WHERE key = 'lot_method'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(m) = method {
        if m.as_str() == "fifo" {
            cfg.lot_method = LotMethod::Fifo;
        }
    }

    Ok(cfg)
}
