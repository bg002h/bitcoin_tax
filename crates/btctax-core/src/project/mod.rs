pub mod fold;
pub mod pools;
pub mod resolve;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotMethod {
    /// Default per TP5; specific-ID is a future hook (Phase 3 optimizer).
    Fifo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectionConfig {
    pub self_transfer_fee: FeeTreatment,
    pub lot_method: LotMethod,
}
impl Default for ProjectionConfig {
    fn default() -> Self {
        // DO NOT change: TP8 default is (c); the spec/memory forbid flipping it to (b).
        ProjectionConfig {
            self_transfer_fee: FeeTreatment::TreatmentC,
            lot_method: LotMethod::Fifo,
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
