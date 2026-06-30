//! The CLI's persisted projection-config knob (TP8 self-transfer fee treatment + lot method), stored
//! in a `cli_config(key,value)` table inside the vault DB. It is a projection *input parameter*, not
//! ledger state (NFR6): the event log remains the sole source of truth; this only selects a swappable
//! rule. TP8 default is (c), USER-MANDATED — never default to (b).
use crate::CliError;
use btctax_core::{FeeTreatment, LotMethod, ProjectionConfig};
use rusqlite::{Connection, OptionalExtension};

/// Session-wide projection configuration loaded from the vault's `cli_config` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliConfig {
    pub fee_treatment: FeeTreatment,
    pub pre2025_method: LotMethod,
    pub pre2025_method_attested: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        // DO NOT change: TP8 default is (c). Spec §2/TP8 + user memory forbid flipping it to (b).
        CliConfig {
            fee_treatment: FeeTreatment::TreatmentC,
            pre2025_method: LotMethod::Fifo,
            pre2025_method_attested: false,
        }
    }
}

impl CliConfig {
    /// Convert to the core projection type for use in `project()`.
    pub fn to_projection(self) -> ProjectionConfig {
        ProjectionConfig {
            self_transfer_fee: self.fee_treatment,
            pre2025_method: self.pre2025_method,
        }
    }
}

/// Create the `cli_config` key-value side-table if it does not exist.
/// M2: `CREATE TABLE IF NOT EXISTS` makes this idempotent — safe to call on any vault (old, new,
/// or restored from snapshot). Called by `Session::create`; also called at the top of `read_config`
/// as a defensive ensure-table-then-read guard.
pub fn init_config_table(conn: &Connection) -> Result<(), CliError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cli_config (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;
    Ok(())
}

fn get(conn: &Connection, key: &str) -> Result<Option<String>, CliError> {
    Ok(conn
        .query_row("SELECT value FROM cli_config WHERE key=?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .optional()?)
}

fn lot_method_tag(m: LotMethod) -> &'static str {
    match m {
        LotMethod::Fifo => "fifo",
        LotMethod::Lifo => "lifo",
        LotMethod::Hifo => "hifo",
    }
}

/// Read the persisted config, falling back to the (c)+FIFO default for any *unset* key (so a freshly
/// created vault, or a future-added key, reads as the safe default).
///
/// M2 (robust to older vaults): calls `init_config_table` first so that a vault created before this
/// table existed never fails with "no such table". The CREATE IF NOT EXISTS is a no-op when the table
/// already exists.
///
/// M1 (no silent misread): returns `CliError::BadConfigValue` for any stored value that is not a
/// recognized enum tag. A corrupt DB or a value written by a future version of the tool will surface
/// as an error rather than being silently re-interpreted as the default.
pub fn read_config(conn: &Connection) -> Result<CliConfig, CliError> {
    // M2: ensure-table-then-read so that older/restored vaults don't get "no such table".
    init_config_table(conn)?;

    let mut cfg = CliConfig::default();
    if let Some(v) = get(conn, "fee_treatment")? {
        cfg.fee_treatment = match v.as_str() {
            "c" => FeeTreatment::TreatmentC,
            "b" => FeeTreatment::TreatmentB,
            _ => {
                // M1: surface corrupt or future-written values instead of silently misreading them.
                return Err(CliError::BadConfigValue {
                    key: "fee_treatment".into(),
                    value: v,
                });
            }
        };
    }
    if let Some(v) = get(conn, "pre2025_method")? {
        cfg.pre2025_method = match v.as_str() {
            "fifo" => LotMethod::Fifo,
            "lifo" => LotMethod::Lifo,
            "hifo" => LotMethod::Hifo,
            _ => {
                return Err(CliError::BadConfigValue {
                    key: "pre2025_method".into(),
                    value: v,
                })
            }
        };
    }
    if let Some(v) = get(conn, "pre2025_method_attested")? {
        cfg.pre2025_method_attested = match v.as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return Err(CliError::BadConfigValue {
                    key: "pre2025_method_attested".into(),
                    value: v,
                })
            }
        };
    }
    Ok(cfg)
}

/// Persist the pre-2025 lot identification method and its attestation flag.
pub fn set_pre2025_method(conn: &Connection, m: LotMethod, attested: bool) -> Result<(), CliError> {
    conn.execute(
        "INSERT INTO cli_config(key,value) VALUES('pre2025_method',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [lot_method_tag(m)],
    )?;
    conn.execute(
        "INSERT INTO cli_config(key,value) VALUES('pre2025_method_attested',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [if attested { "true" } else { "false" }],
    )?;
    Ok(())
}

/// Persist the TP8 fee treatment. Both (c) and (b) are writable; (b) is opt-in only.
/// The application enforces (c) as the default — callers must explicitly pass TreatmentB to opt in.
pub fn set_fee_treatment(conn: &Connection, t: FeeTreatment) -> Result<(), CliError> {
    let v = match t {
        FeeTreatment::TreatmentC => "c",
        FeeTreatment::TreatmentB => "b",
    };
    conn.execute(
        "INSERT INTO cli_config(key,value) VALUES('fee_treatment',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [v],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::{FeeTreatment, LotMethod};

    fn mem() -> rusqlite::Connection {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        init_config_table(&c).unwrap();
        c
    }

    #[test]
    fn default_is_treatment_c_user_mandated() {
        let c = mem();
        assert_eq!(
            read_config(&c).unwrap().fee_treatment,
            FeeTreatment::TreatmentC
        );
    }

    #[test]
    fn set_then_read_b_opt_in_round_trips() {
        let c = mem();
        set_fee_treatment(&c, FeeTreatment::TreatmentB).unwrap();
        assert_eq!(
            read_config(&c).unwrap().fee_treatment,
            FeeTreatment::TreatmentB
        );
        // and back to the mandated default
        set_fee_treatment(&c, FeeTreatment::TreatmentC).unwrap();
        assert_eq!(
            read_config(&c).unwrap().fee_treatment,
            FeeTreatment::TreatmentC
        );
    }

    #[test]
    fn default_pre2025_method_is_fifo_unattested() {
        let c = mem();
        let cfg = read_config(&c).unwrap();
        assert!(!matches!(cfg.pre2025_method, LotMethod::Hifo));
        assert_eq!(cfg.pre2025_method, LotMethod::Fifo);
        assert!(!cfg.pre2025_method_attested);
        assert_eq!(cfg.to_projection().pre2025_method, LotMethod::Fifo);
    }
    #[test]
    fn set_pre2025_method_round_trips_with_attestation() {
        let c = mem();
        set_pre2025_method(&c, LotMethod::Hifo, true).unwrap();
        let cfg = read_config(&c).unwrap();
        assert_eq!(cfg.pre2025_method, LotMethod::Hifo);
        assert!(cfg.pre2025_method_attested);
        assert_eq!(cfg.to_projection().pre2025_method, LotMethod::Hifo);
    }
    #[test]
    fn bad_pre2025_method_value_is_an_error() {
        let c = mem();
        c.execute(
            "INSERT INTO cli_config(key,value) VALUES('pre2025_method','zzz')",
            [],
        )
        .unwrap();
        assert!(matches!(read_config(&c).unwrap_err(),
            CliError::BadConfigValue { ref key, .. } if key == "pre2025_method"));
    }

    // M2: read_config must not fail with "no such table" on a vault that was created
    // before the cli_config table was added (e.g. an older/restored vault).
    #[test]
    fn read_config_on_table_less_vault_returns_default() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        // Deliberately do NOT call init_config_table — simulate an older vault.
        let cfg = read_config(&c).unwrap(); // must not error
        assert_eq!(cfg.fee_treatment, FeeTreatment::TreatmentC);
    }

    // M1: an unrecognized stored value must surface as an error, not a silent default.
    #[test]
    fn bad_stored_value_is_an_error_not_silent_default() {
        let c = mem();
        // Manually insert a corrupt / future value.
        c.execute(
            "INSERT INTO cli_config(key,value) VALUES('fee_treatment','z')",
            [],
        )
        .unwrap();
        let err = read_config(&c).unwrap_err();
        assert!(
            matches!(err, CliError::BadConfigValue { ref key, .. } if key == "fee_treatment"),
            "expected BadConfigValue, got: {err}"
        );
    }
}
