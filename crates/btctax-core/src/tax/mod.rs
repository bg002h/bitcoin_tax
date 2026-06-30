//! Tax-rate engine types and computation (Sub-project B). Federal-only (NFR5 exact Decimal;
//! NFR4 determinism). No float anywhere — all rates are `Decimal` literals.
//! Tables and compute modules are added in Tasks 2–5.
pub mod types;

pub use types::{Carryforward, FilingStatus, MarginalRates, TaxOutcome, TaxProfile, TaxResult};
