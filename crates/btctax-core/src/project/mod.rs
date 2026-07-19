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
            // Realistic no-election default: HIFO (kept in sync with `CliConfig::default`, §reconcile-defaults).
            pre2025_method: LotMethod::Hifo,
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

/// UX-P4-3 record-time validation, DEFINITIONALLY the resolver. Answers "would appending `incoming`
/// (a reconcile decision payload, e.g. `ClassifyInbound`/`ManualFmv`/`VoidDecisionEvent`) introduce a
/// NEW `DecisionConflict`?" — returning the offending blocker's `detail` if so, else `None`.
///
/// It does NOT hand-rebuild the resolver's `applied` map (a subset view drifts — a prior draft was one
/// writer short and both false-refused and false-accepted). Instead it RUNS the real projection twice
/// and diffs the `DecisionConflict` set: baseline (`events`) vs `events` + the candidate appended as
/// the resolver would append it (the next decision seq — the highest, so it is the LOSING side of any
/// first-wins race). A conflict present WITH the candidate but not in the baseline is one the candidate
/// introduced. Baseline-diff (not a candidate-id match) is required because the passthrough-overlap
/// guard keys its blocker to the EXISTING decision, not the newcomer.
///
/// **Pseudo is forced OFF** so the shadow is the real (non-synthetic) projection — this keeps
/// void→re-decide and the FIRST real classify of a pseudo-defaulted target working, and honors an
/// accepted-conflict `SupersedeImport` override the resolver sees. Every per-verb rule (first-wins for
/// ClassifyInbound/ReclassifyOutflow/ReclassifyIncome/ClassifyRaw; `ManualFmv` last-wins so `set-fmv`
/// is duplicate-exempt yet still existence/type-validated; wrong-type / unknown-target) falls out for
/// free because this IS the resolver. Pure/total (NFR4); two `project` calls, cheap for infrequent
/// record-time use. Never sees the stored pseudo cfg's taint.
pub fn would_conflict(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    incoming: &crate::event::EventPayload,
    now: time::OffsetDateTime,
) -> Option<String> {
    use crate::identity::EventId;
    use crate::state::BlockerKind;
    use std::collections::BTreeSet;

    let mut cfg = *config;
    cfg.pseudo_reconcile = false;

    let conflicts = |evs: &[LedgerEvent]| -> Vec<(Option<EventId>, String)> {
        project(evs, prices, &cfg)
            .blockers
            .into_iter()
            .filter(|b| b.kind == BlockerKind::DecisionConflict)
            .map(|b| (b.event, b.detail))
            .collect()
    };

    let baseline: BTreeSet<(Option<EventId>, String)> = conflicts(events).into_iter().collect();

    let next_seq = events
        .iter()
        .filter_map(|e| match &e.id {
            EventId::Decision { seq } => Some(*seq),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1;
    let candidate = LedgerEvent {
        id: EventId::decision(next_seq),
        utc_timestamp: now,
        original_tz: time::UtcOffset::UTC,
        wallet: None,
        payload: incoming.clone(),
    };
    let mut with_candidate = events.to_vec();
    with_candidate.push(candidate);

    conflicts(&with_candidate)
        .into_iter()
        .find(|c| !baseline.contains(c))
        .map(|(_, detail)| detail)
}

/// The cost-basis method currently in force for a wallet, plus its provenance — the UI-facing answer
/// to "what method governs this account, and is it an explicit per-account election or inherited?"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InForceMethod {
    pub method: LotMethod,
    /// `true` ⇒ the method came from a PER-ACCOUNT (scoped) election for this wallet; `false` ⇒ it is
    /// inherited from a GLOBAL election or the HIFO default (pre-2025 dates: the `pre2025_method`).
    pub scoped: bool,
}

/// §A.5(a) UI helper: the in-force method for each of `wallets` as of `date`, resolved by the SAME
/// shared two-tier resolver (`resolve::resolve_election`) the fold and compliance use — scoped
/// election → global election → HIFO default — never a re-implementation of the precedence. Pre-2025 `date`s
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
                        // No election on file → the HIFO app default (§reconcile-defaults); reported so
                        // the UI matches the computation's `applicable_method` fall-through exactly.
                        method: LotMethod::Hifo,
                        scoped: false,
                    },
                }
            }
        })
        .collect()
}
