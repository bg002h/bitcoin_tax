//! btctax-core: domain model + pure deterministic event-sourced projection for the bitcoin_tax ledger.
//! The projection (`project`) is total and never panics (spec §7.1); only `persistence` performs I/O.
pub mod conventions;
pub mod event;
pub mod identity;

pub use conventions::{Sat, TaxDate, Usd};
pub use event::{EventPayload, LedgerEvent};
pub use identity::{EventId, Fingerprint, LotId, Source, SourceRef, WalletId};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("event (de)serialization: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("persistence: {0}")]
    Persistence(String),
}
