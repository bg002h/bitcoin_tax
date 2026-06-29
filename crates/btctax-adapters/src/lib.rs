//! btctax-adapters: exchange-export parsers + the bundled daily-close price dataset (§9).
//! Parses Coinbase/Gemini/River/Swan exports into `btctax_core::LedgerEvent`s — BTC-only (FR2),
//! ingest-time FMV (FR3) over the bundled `PriceProvider` (§9.2). Exact arithmetic only (NFR5).
//!
//! PRIVACY: only SYNTHETIC fixtures are used in tests; the real exports in
//! ~/Documents/BitcoinTax/ReadOnly are NEVER read by this crate or its tests.
pub mod adapter;
pub mod normalize;
pub mod parse;
pub mod price;
pub mod read;
pub mod sources;

pub use adapter::{Adapter, FileGroup, FileReport, GroupOutput, IngestBatch, SourceFile};
pub use price::BundledPrices;

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("io reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("csv {path}: {source}")]
    Csv {
        path: String,
        #[source]
        source: csv::Error,
    },
    #[error("xlsx {path}: {source}")]
    Xlsx {
        path: String,
        #[source]
        source: calamine::Error,
    },
    /// Used by `read_xlsx` when the workbook has no worksheet (calamine returns `None`, not an error).
    /// Distinct from `Xlsx` (which wraps a calamine error) so callers can pattern-match the cause.
    #[error("xlsx {path}: no worksheet found")]
    EmptyXlsx { path: String },
    #[error("{adapter} row {line}: missing required column {column:?}")]
    MissingColumn {
        adapter: &'static str,
        line: usize,
        column: String,
    },
    #[error("{adapter} row {line}: cannot parse {field} from {value:?}: {reason}")]
    Parse {
        adapter: &'static str,
        line: usize,
        field: &'static str,
        value: String,
        reason: String,
    },
    #[error("{adapter} row {line}: fractional satoshi in BTC amount {value:?}")]
    FractionalSat {
        adapter: &'static str,
        line: usize,
        value: String,
    },
    #[error("unrecognized file (no adapter matched): {path}")]
    UnknownSource { path: String },
    /// A file was detected as Swan (matched at least one role signature) but its header did not
    /// match any of the three confirmed roles (trades / transfers / withdrawals). The actual trigger
    /// is an unrecognized role, not a missing file — hence the rename from `IncompleteSwanBatch`.
    #[error(
        "unrecognized Swan file role (header did not match trades/transfers/withdrawals): {path}"
    )]
    UnrecognizedSwanRole { path: String },
    #[error("{adapter}: header signature not found in file")]
    HeaderNotFound { adapter: &'static str },
    #[error("price dataset: {0}")]
    PriceDataset(String),
}
