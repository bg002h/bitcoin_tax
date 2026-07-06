//! btctax-core: domain model + pure deterministic event-sourced projection for the bitcoin_tax ledger.
//! The projection (`project`) is total and never panics (spec §7.1); only `persistence` performs I/O.
pub mod conventions;
pub mod donation;
pub mod event;
pub mod forms;
pub mod identity;
pub mod optimize;
pub mod persistence;
pub mod price;
pub mod project;
pub mod state;
pub mod tax;
pub mod void;
pub mod whatif;

pub use conventions::{Sat, TaxDate, Usd};
pub use donation::DonationDetails;
pub use event::*;
pub use forms::{
    form_8283, form_8949, schedule_d, year_donation_deduction, Form8283HowAcquired, Form8283Row,
    Form8283Section, Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals,
};
pub use identity::{EventId, Fingerprint, LotId, Source, SourceRef, WalletId};
pub use optimize::{
    consult_sale, optimize_year, score_assignment, ApproxReason, ConsultReport, ConsultRequest,
    DisposalProposal, OptimizeError, OptimizeProposal, Persistability, TimingInsight,
};
pub use price::PriceProvider;
pub use project::{
    conservation_report, disposal_compliance, evaluate_disposal, in_force_methods, project,
    pseudo_plan, CandidateDisposal, ComplianceStatus, ConservationReport, DisposalCompliance,
    EvaluateError, EvaluateOutcome, FeeTreatment, InForceMethod, LotMethod, ProjectionConfig,
    PseudoDefault, PseudoKind,
};
pub use state::*;
pub use tax::{
    carryforward_consistency, compute_se_tax, compute_tax_year, loss_limit, niit_threshold,
    se_addl_medicare_threshold, se_net_income, Carryforward, FilingStatus, LtcgBreakpoints,
    MarginalRates, OrdinaryBracket, OrdinarySchedule, PrefSplit, SeTaxResult, TaxOutcome,
    TaxProfile, TaxResult, TaxTable, TaxTables, NIIT_RATE, QUALIFIED_APPRAISAL_THRESHOLD,
    SE_NET_EARNINGS_FACTOR, SE_RATE_ADDL_MEDICARE, SE_RATE_MEDICARE, SE_RATE_SS,
};
pub use void::{is_revocable_payload, voidable_decisions};
pub use whatif::{
    CarryforwardDelta, ConsumedLot, LtcgBracket, SellMethod, SellReport, SellRequest, SellStatus,
    WhatIfError,
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
