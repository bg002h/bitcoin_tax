//! Tax-rate engine types and computation (Sub-project B). Federal-only (NFR5 exact Decimal;
//! NFR4 determinism). No float anywhere — all rates are `Decimal` literals.
//! Tables and compute modules are added in Tasks 2–5.
pub mod advisories;
pub mod amt;
pub mod charitable;
pub mod compute;
pub mod frozen_guard;
pub mod method;
pub mod other_taxes;
pub mod qbi;
pub mod return_1040;
pub mod return_inputs;
pub mod return_refuse;
pub mod se;
pub mod tables;
pub mod types;

pub use return_1040::{
    apply_carryover_writeback, assemble_absolute, derive_tax_profile, screen_absolute,
    AbsoluteReturn,
};

pub use method::{
    assert_edges_binnable, first_unbinnable_edge, qdcgt_line16, regular_tax, TAX_TABLE_CEILING,
};

pub use compute::{
    carryforward_consistency, compute_tax_year, net_1222, ordinary_tax_on, preferential_tax,
    CapNet, PrefSplit,
};
pub use se::{compute_se_tax, se_net_income, SeTaxResult};
pub use tables::{
    loss_limit, niit_threshold, se_addl_medicare_threshold, FullReturnParams, FullReturnTables,
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables, NIIT_RATE,
    QUALIFIED_APPRAISAL_THRESHOLD, SE_NET_EARNINGS_FACTOR, SE_RATE_ADDL_MEDICARE, SE_RATE_MEDICARE,
    SE_RATE_SS,
};
pub use types::{Carryforward, FilingStatus, MarginalRates, TaxOutcome, TaxProfile, TaxResult};
