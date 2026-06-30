//! Tax-rate engine types and computation (Sub-project B). Federal-only (NFR5 exact Decimal;
//! NFR4 determinism). No float anywhere — all rates are `Decimal` literals.
//! Tables and compute modules are added in Tasks 2–5.
pub mod compute;
pub mod tables;
pub mod types;

pub use compute::{
    carryforward_consistency, compute_tax_year, net_1222, ordinary_tax_on, preferential_tax,
    CapNet, PrefSplit,
};
pub use tables::{
    loss_limit, niit_threshold, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable,
    TaxTables, NIIT_RATE,
};
pub use types::{Carryforward, FilingStatus, MarginalRates, TaxOutcome, TaxProfile, TaxResult};
