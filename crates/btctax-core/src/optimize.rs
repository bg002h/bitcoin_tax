//! Sub-project C — rate-aware optimizer. ASSIGNS lots to disposals (specific identification);
//! it does NOT advise whether to sell/hold (no investment advice — §C scope). Minimizes B's
//! federal `total_federal_tax_attributable` over feasible per-disposal `LotSelection`s, within the
//! §1.1012-1(j) identification boundary (adequate ID by the time of sale; no compliant post-hoc).
//! Deterministic (NFR4) + exact (NFR5): BTreeMap/sorted iteration, Decimal/i64 only, no float.
//! §1091 wash-sale does NOT apply to crypto — loss lots are freely selectable (Task 7; monitor).
use crate::conventions::{Sat, TaxDate, Usd};
use crate::event::{DisposeKind, LotPick};
use crate::identity::{EventId, WalletId};
use crate::project::ComplianceStatus;
use crate::project::EvaluateError;
use crate::state::Blocker;
use crate::tax::MarginalRates;

/// The `accept`-gate verdict for one disposal (computed in core; enforced by the CLI, Task 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistability {
    /// The selection's made-date is at/before the sale → §A.5(b) `Contemporaneous`; persist freely.
    ContemporaneousNow,
    /// Already-executed (made-date after the sale) but within the own-books envelope → persist ONLY
    /// behind the narrow contemporaneous-ID attestation (→ `AttestedRecording`).
    NeedsAttestation,
    /// 2027+ broker-held: own-books is insufficient; C may NEVER persist (no attestation can cure it).
    ForbiddenBroker2027,
}

/// One disposal's line in a Mode-1 proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalProposal {
    pub disposal: EventId,
    pub wallet: WalletId,
    pub date: TaxDate,
    pub current_selection: Vec<LotPick>, // lots the CURRENT projection consumes (baseline)
    pub proposed_selection: Vec<LotPick>, // the optimizer's tax-minimizing pick
    pub status: ComplianceStatus,        // overlay-aware (may be AttestedRecording, Task 5)
    pub persistable: Persistability,
}

/// Why a proposal is only APPROXIMATE (not a proven global minimum). Carried OUT of core (core has no
/// logger) so the CLI can log the cap/why and the renderer can show the banner. Plain counts only →
/// deterministic + serde/Eq-friendly (R0-C1/C3 fold).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproxReason {
    /// The cartesian product of per-group candidate lists exceeded `MAX_COMBOS`; the baseline-seeded
    /// coordinate-descent fallback ran (a LOCAL optimum — disclosed, and never worse than baseline).
    ComboCapExceeded { combos: usize, cap: usize },
    /// ≥1 contended same-wallet pool could not be JOINTLY enumerated within the bound; its disposals
    /// fell back to per-disposal-independent generation (a cross-period reassignment optimum may be
    /// missed — R0-C3). `contended` = number of disposals in the un-enumerated contention group(s).
    ContentionUnenumerated {
        contended: usize,
        combos: usize,
        cap: usize,
    },
    /// ≥1 target disposal's available pool exceeded `LOT_ENUM_BOUND`, so `candidate_selections`
    /// returned a deterministic but INCOMPLETE heuristic SUBSET of that pool's vertices (not the full
    /// vertex enumeration) — the result over that pool is therefore NOT a proven global minimum
    /// (R2-C1). Common in practice (weekly-DCA / active-trading pools with > 12 lots). `lots` = the
    /// largest heuristic pool's lot count; `bound` = `LOT_ENUM_BOUND`. Baseline-seeded, so `delta ≤ 0`
    /// still holds — the disclosure corrects the false "proven optimum" claim, not the pick's safety.
    PoolHeuristic { lots: usize, bound: usize },
}

/// Mode-1 proposal: what-if by default (running this binds NOTHING — §C.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizeProposal {
    pub year: i32,
    pub baseline_tax: Usd, // total_federal_tax_attributable under current identification
    pub optimized_tax: Usd, // under the proposed selections
    pub delta: Usd, // optimized − baseline — ALWAYS ≤ 0 (baseline-seeded search; never worsens)
    pub per_disposal: Vec<DisposalProposal>,
    pub marginal_rates: MarginalRates,
    /// `false` ⇔ the vertex set was **FULLY enumerated AND exhaustively scored** — i.e. EVERY target
    /// disposal's pool was ≤ `LOT_ENUM_BOUND` (complete vertex enumeration, NOT a heuristic subset —
    /// R2-C1), the overall `product` was ≤ `MAX_COMBOS` (exhaustive, not coordinate-descent), AND every
    /// contended pool was jointly enumerated. ONLY then is the result the PROVEN global minimum over the
    /// vertex space. `true` ⇔ ANY of those failed (a disclosed LOCAL / under-enumerated / heuristic-pool
    /// result) — the renderer MUST print the "APPROXIMATE — not a guaranteed global minimum" banner and
    /// the CLI MUST log `approx_reason` (R0-C1/C3, R2-C1). NEVER render `optimized_tax` as "the optimum"
    /// when this is `true`.
    pub approximate: bool,
    pub approx_reason: Option<ApproxReason>,
}

/// Mode-2 (pre-trade consultation) request — a hypothetical sale NOT in the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultRequest {
    pub sell_sat: Sat,
    pub wallet: WalletId,
    pub at: TaxDate,
    pub proceeds: Option<Usd>, // required when no dataset price exists for `at` (future dates)
    pub kind: DisposeKind,
}

/// §C.3 ST→LT crossover timing insight (tax decision-support; NOT a hold/sell recommendation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingInsight {
    pub st_sat_in_selection: Sat, // sats in the best selection that are short-term as of `at`
    pub latest_crossover: TaxDate, // the last date any of those lots becomes long-term
    pub tax_if_sold_long_term: Usd, // same lots, scored as if sold on/after `latest_crossover`
    pub saving_if_waited: Usd,    // total_now − tax_if_sold_long_term (≥ 0)
}

/// Mode-2 read-only what-if result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsultReport {
    pub req: ConsultRequest,
    pub proposed_selection: Vec<LotPick>,
    pub st_gain: Usd,
    pub lt_gain: Usd,
    pub total_federal_tax_attributable: Usd,
    pub timing: Option<TimingInsight>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizeError {
    /// B refuses to compute the year (any Hard blocker anywhere, or missing profile/table) — I6.
    YearNotComputable(Blocker),
    /// A synthetic consult disposal needs `--proceeds` (no dataset price for `at`), etc.
    Evaluate(EvaluateError),
    /// Mode 1: the year has no method-honoring disposals to optimize.
    NoDisposals,
    /// Mode 2: the wallet has no lots available to sell at `at`.
    NoLots,
    /// The requested year is pre-2025 — a restatement of a closed year, not an optimization (M7).
    PreTransitionYear(i32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::LotPick;

    #[test]
    fn error_variants_are_constructible_and_eq() {
        let e = OptimizeError::PreTransitionYear(2024);
        assert_eq!(e, OptimizeError::PreTransitionYear(2024));
        assert_ne!(e, OptimizeError::NoDisposals);
        assert_eq!(
            Persistability::ForbiddenBroker2027,
            Persistability::ForbiddenBroker2027
        );
    }

    #[test]
    fn lot_pick_is_totally_ordered() {
        // R0-I2: the dedup/tie-break machinery requires `Vec<LotPick>: Ord`. A BTreeSet of pick-vecs
        // must compile and sort deterministically.
        use std::collections::BTreeSet;
        let mut s: BTreeSet<Vec<LotPick>> = BTreeSet::new();
        s.insert(vec![/* pick(b) */]);
        s.insert(vec![/* pick(a) */]);
        let _sorted: Vec<Vec<LotPick>> = s.into_iter().collect(); // compiles ⇒ LotPick: Ord
    }

    #[test]
    fn approx_reason_variants_are_eq() {
        assert_eq!(
            ApproxReason::ComboCapExceeded {
                combos: 100,
                cap: 50_000
            },
            ApproxReason::ComboCapExceeded {
                combos: 100,
                cap: 50_000
            }
        );
        assert_eq!(
            ApproxReason::ContentionUnenumerated {
                contended: 2,
                combos: 60_000,
                cap: 50_000
            },
            ApproxReason::ContentionUnenumerated {
                contended: 2,
                combos: 60_000,
                cap: 50_000
            }
        );
        assert_eq!(
            ApproxReason::PoolHeuristic {
                lots: 15,
                bound: 12
            },
            ApproxReason::PoolHeuristic {
                lots: 15,
                bound: 12
            }
        );
    }
}
