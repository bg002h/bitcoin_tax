pub mod compliance;
pub mod conservation;
pub mod evaluate;
pub mod fold;
pub mod pools;
pub mod resolve;
pub mod transition;

pub use compliance::{disposal_compliance, ComplianceStatus, DisposalCompliance};
pub use conservation::{conservation_report, ConservationReport};
pub use evaluate::{evaluate_disposal, CandidateDisposal, EvaluateError, EvaluateOutcome};
pub use resolve::{PseudoDefault, PseudoKind};

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
    /// Whether the taxpayer has declared + attested their filed pre-2025 lot method.
    /// `false` (the default) makes the advisory louder and actionable; `true` produces an
    /// informational acknowledgment. Neither value gates `compute_tax_year` (§D1).
    pub pre2025_method_attested: bool,
    /// Pseudo-reconcile mode (sub-project 2). When `true`, `resolve` synthesizes DELIBERATELY-FICTIONAL
    /// default decisions at PROJECTION time (never persisted) to clear the Hard *classification* blockers,
    /// producing a loudly-flagged on-screen estimate the user corrects toward truth. Default `false` [N1];
    /// mode-off ⇒ projection is byte-identical to today (no synthetics injected). Real decisions always
    /// supersede synthetics. Synthetics are NEVER written to the ledger by projection — only
    /// `reconcile pseudo approve` promotes chosen defaults to real (attested) decisions.
    pub pseudo_reconcile: bool,
}
impl Default for ProjectionConfig {
    fn default() -> Self {
        // DO NOT change: TP8 default is (c); the spec/memory forbid flipping it to (b).
        ProjectionConfig {
            self_transfer_fee: FeeTreatment::TreatmentC,
            pre2025_method: LotMethod::Fifo,
            pre2025_method_attested: false,
            pseudo_reconcile: false,
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

/// Pseudo-reconcile (sub-project 2): the ordered list of synthetic default decisions the projection WOULD
/// inject in pseudo mode — the SAME `PseudoDefault`s carried on the `Resolution` (so "what you see == what
/// you approve"). Pure/deterministic (NFR4). Pseudo mode is FORCED on for this computation, so `approve`
/// can enumerate the defaults independent of the stored flag; each `PseudoDefault.decision` is a
/// materializable REAL decision. NEVER writes anything — approve persists via the CLI `apply_bulk_*` loop.
pub fn pseudo_plan(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
) -> Vec<PseudoDefault> {
    let mut cfg = *config;
    cfg.pseudo_reconcile = true;
    resolve::resolve(events, prices, &cfg).pseudo_decisions
}

/// The cost-basis method currently in force for a wallet, plus its provenance — the UI-facing answer
/// to "what method governs this account, and is it an explicit per-account election or inherited?"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InForceMethod {
    pub method: LotMethod,
    /// `true` ⇒ the method came from a PER-ACCOUNT (scoped) election for this wallet; `false` ⇒ it is
    /// inherited from a GLOBAL election or the FIFO default (pre-2025 dates: the `pre2025_method`).
    pub scoped: bool,
}

/// §A.5(a) UI helper: the in-force method for each of `wallets` as of `date`, resolved by the SAME
/// shared two-tier resolver (`resolve::resolve_election`) the fold and compliance use — scoped
/// election → global election → FIFO — never a re-implementation of the precedence. Pre-2025 `date`s
/// report `pre2025_method` (inherited). Runs `resolve` ONCE; the returned Vec is aligned with
/// `wallets`. Used by the btctax-tui-edit method-election flow to show each account's resolved method.
pub fn in_force_methods(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    date: crate::conventions::TaxDate,
    wallets: &[crate::identity::WalletId],
) -> Vec<InForceMethod> {
    let res = resolve::resolve(events, prices, config);
    wallets
        .iter()
        .map(|w| {
            if date < crate::conventions::TRANSITION_DATE {
                InForceMethod {
                    method: config.pre2025_method,
                    scoped: false,
                }
            } else {
                match resolve::resolve_election(date, w, &res.elections) {
                    Some(e) => InForceMethod {
                        method: e.method,
                        scoped: e.wallet.is_some(),
                    },
                    None => InForceMethod {
                        method: LotMethod::Fifo,
                        scoped: false,
                    },
                }
            }
        })
        .collect()
}
