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
}
