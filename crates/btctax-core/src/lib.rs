//! btctax-core: domain model + pure deterministic event-sourced projection for the bitcoin_tax ledger.
//! The projection (`project`) is total and never panics (spec §7.1); only `persistence` performs I/O.
pub mod conventions;
pub mod event;
pub mod identity;
pub mod optimize;
pub mod persistence;
pub mod price;
pub mod project;
pub mod state;
pub mod tax;

pub use conventions::{Sat, TaxDate, Usd};
pub use event::*;
pub use identity::{EventId, Fingerprint, LotId, Source, SourceRef, WalletId};
pub use optimize::{
    ApproxReason, ConsultReport, ConsultRequest, DisposalProposal, OptimizeError, OptimizeProposal,
    Persistability, TimingInsight,
};
pub use price::PriceProvider;
pub use project::{
    conservation_report, disposal_compliance, evaluate_disposal, project, CandidateDisposal,
    ComplianceStatus, ConservationReport, DisposalCompliance, EvaluateError, EvaluateOutcome,
    FeeTreatment, LotMethod, ProjectionConfig,
};
pub use state::*;
pub use tax::{
    carryforward_consistency, compute_tax_year, loss_limit, niit_threshold, Carryforward,
    FilingStatus, LtcgBreakpoints, MarginalRates, OrdinaryBracket, OrdinarySchedule, TaxOutcome,
    TaxProfile, TaxResult, TaxTable, TaxTables, NIIT_RATE,
};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("event (de)serialization: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("persistence: {0}")]
    Persistence(String),
}
