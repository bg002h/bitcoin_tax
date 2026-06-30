pub mod compliance;
pub mod conservation;
pub mod fold;
pub mod pools;
pub mod resolve;
pub mod transition;

pub use compliance::{disposal_compliance, ComplianceStatus, DisposalCompliance};
pub use conservation::{conservation_report, ConservationReport};

use crate::event::LedgerEvent;
use crate::price::PriceProvider;
use crate::state::LedgerState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeTreatment {
    /// TP8 DEFAULT: fee_sat consumed at zero proceeds (non-taxable); full basis carries. USER-MANDATED default.
    TreatmentC,
    /// TP8 config: taxable mini-disposition of fee-sats (recognition record only; not a 2nd conservation entry).
    TreatmentB,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LotMethod {
    #[default]
    Fifo,
    Lifo,
    Hifo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionConfig {
    pub self_transfer_fee: FeeTreatment,
    /// Historical identification method for pre-2025 lots (attested via `CliConfig`).
    pub pre2025_method: LotMethod,
}
impl Default for ProjectionConfig {
    fn default() -> Self {
        // DO NOT change: TP8 default is (c); the spec/memory forbid flipping it to (b).
        ProjectionConfig {
            self_transfer_fee: FeeTreatment::TreatmentC,
            pre2025_method: LotMethod::Fifo,
        }
    }
}

/// The projection contract (§7.1): pure, deterministic, no I/O, total (never panics).
pub fn project(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
) -> LedgerState {
    // I-2: `resolve` takes (events, prices, config) — Task-12 transition effectiveness needs both.
    let resolution = resolve::resolve(events, prices, config);
    fold::fold(resolution, prices, config)
}
