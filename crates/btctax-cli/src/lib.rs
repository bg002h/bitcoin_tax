//! btctax-cli: the CLI + reconciliation library that wires the encrypted vault (btctax-store),
//! ingest (btctax-adapters), and the pure projection (btctax-core) into the Phase-1 command surface
//! (spec §11). The library is I/O-explicit and deterministic; the binary (`main.rs`) is a thin clap
//! dispatch. PRIVACY: tests use only temp vaults + synthetic fixtures; no real user file is ever read.
pub mod bulk_estimated;
pub mod cli;
pub mod cmd;
pub mod config;
pub mod donation_details;
pub mod eventref;
pub mod optimize_attest;
pub mod price_cache;
pub mod render;
pub mod session;
pub mod tax_profile;

pub use cli::Cli;
pub use config::CliConfig;
pub use session::{
    BulkFilter, BulkIncomeFilter, BulkIncomePlan, BulkIncomeRow, BulkLinkPlan, BulkLinkRow,
    BulkReclassifyOutflowPlan, BulkReclassifyOutflowRow, BulkResolvePlan, BulkResolveRow,
    BulkStiFilter, BulkStiPlan, BulkStiRow, BulkVoidPlan, BulkVoidRow, Frame, MatchAction,
    MatchProposal, Session,
};

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    Store(#[from] btctax_store::StoreError),
    #[error(transparent)]
    Core(#[from] btctax_core::CoreError),
    #[error(transparent)]
    Adapter(#[from] btctax_adapters::AdapterError),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// C1: `write_csv_exports` (Task 15) uses `?` on `csv::Writer` ops (→ `csv::Error`); `csv::Error`
    /// is its own type (NOT covered by `Io(#[from] io::Error)`, whose `From` goes the other way), so it
    /// needs its own variant or Task 15 will not compile.
    #[error("csv: {0}")]
    Csv(#[from] csv::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A user-supplied event reference did not parse as a canonical `EventId` (eventref.rs).
    #[error("not a valid event reference: {0:?}")]
    BadEventRef(String),
    /// A CLI argument was malformed (bad USD/date/enum/wallet spec, or a contradictory flag set).
    #[error("usage: {0}")]
    Usage(String),
    /// M1: a `cli_config` row held an unrecognized value (corrupt DB, future-written value, or manual
    /// edit gone wrong). Returning an error is safer than silently misreading the stored intent.
    #[error("unrecognized stored config value: key={key:?} value={value:?}")]
    BadConfigValue { key: String, value: String },
    /// Sub-project 3 attestation gate: an export was attempted while the ledger is pseudo-reconciled
    /// (a synthetic, non-persisted default contributes to the projection) and NO attestation phrase was
    /// supplied. Producing a form/data file from a fictional draft requires typing the exact phrase.
    /// (Supersedes sub-2's interim [I3] blanket refusal.)
    #[error(
        "export refused: the ledger is pseudo-reconciled (a synthetic default contributes to the \
         projection). To export this draft ON PURPOSE, attest the exact phrase {:?} (pass --attest, or \
         type it at the prompt). Otherwise run `reconcile pseudo off` (or approve + attest the defaults).",
        ATTEST_PHRASE
    )]
    AttestationRequired,
    /// Sub-project 3 attestation gate: an export was attempted while the ledger is pseudo-reconciled and
    /// the supplied attestation phrase did NOT match (trimmed, case-sensitive, exact). A wrong phrase is
    /// FAILED regardless of environment [R0-I1] — no fictional form leaves the machine.
    #[error(
        "export refused: the attestation phrase did not match. The ledger is pseudo-reconciled; type the \
         phrase EXACTLY (trimmed, case-sensitive): {:?}.",
        ATTEST_PHRASE
    )]
    AttestationFailed,
}

/// The exact phrase a user must affirm to export a form/data file while the ledger is pseudo-reconciled
/// (sub-project 3). Compared TRIMMED, case-SENSITIVE, exact. The prompt + both error strings are BUILT
/// from this constant [R0-M1] so there is no drift (a KAT asserts they contain it). `pub` so btctax-tui
/// shares it [R0-r2-N2].
pub const ATTEST_PHRASE: &str = "I attest this is true";

/// PURE exact-compare attestation gate — NO I/O, NO TTY read [R0-I2]. The interactive prompt lives in
/// the caller (the `export-snapshot` main.rs arm / the btctax-tui export modal); this helper only
/// compares, keeping the library I/O-explicit and the KATs deterministic (no env-dependent branch).
///
/// - `attest.map(str::trim) == Some(ATTEST_PHRASE)` → `Ok(())`.
/// - `Some(_)` non-matching → `Err(AttestationFailed)` (a wrong phrase FAILS regardless of env) [R0-I1].
/// - `None` → `Err(AttestationRequired)`.
///
/// `pub` so btctax-tui shares the exact-compare [R0-r2-N2].
pub fn require_attestation(attest: Option<&str>) -> Result<(), CliError> {
    match attest.map(str::trim) {
        Some(p) if p == ATTEST_PHRASE => Ok(()),
        Some(_) => Err(CliError::AttestationFailed),
        None => Err(CliError::AttestationRequired),
    }
}
