//! Sub-project C — rate-aware optimizer. ASSIGNS lots to disposals (specific identification);
//! it does NOT advise whether to sell/hold (no investment advice — §C scope). Minimizes B's
//! federal `total_federal_tax_attributable` over feasible per-disposal `LotSelection`s, within the
//! §1.1012-1(j) identification boundary (adequate ID by the time of sale; no compliant post-hoc).
//! Deterministic (NFR4) + exact (NFR5): BTreeMap/sorted iteration, Decimal/i64 only, no float.
//! §1091 wash-sale does NOT apply to crypto — loss lots are freely selectable (Task 7; monitor).
use crate::conventions::{Sat, TaxDate, Usd};
use crate::event::{DisposeKind, LedgerEvent, LotPick};
use crate::identity::{EventId, WalletId};
use crate::price::PriceProvider;
use crate::project::fold::{fold, pools_before};
use crate::project::pools::pool_key;
use crate::project::resolve::resolve;
use crate::project::ComplianceStatus;
use crate::project::EvaluateError;
use crate::project::{project, ProjectionConfig};
use crate::state::{Blocker, LedgerState, Lot};
use crate::tax::{compute_tax_year, MarginalRates, TaxOutcome, TaxProfile, TaxTables};
use std::collections::BTreeMap;

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

// ── Task 2 — holistic year scorer ────────────────────────────────────────────────────────────────

/// Fold the canonical timeline with `assignment`'s per-disposal selections injected (overriding any
/// persisted selection for those events), WITHOUT mutating the ledger. Clone-fold-discard (mirrors
/// `evaluate.rs`'s `resolve` → inject `selections` → `fold` path): `events`/`prices`/`config` are
/// borrowed read-only, `resolve` yields an owned `Resolution` we mutate, and the resulting
/// `LedgerState` is the caller's to read then discard. Iteration is over a `BTreeMap` (NFR4).
fn fold_with(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    assignment: &BTreeMap<EventId, Vec<LotPick>>,
) -> LedgerState {
    let mut res = resolve(events, prices, config);
    for (disposal, picks) in assignment {
        // Override any persisted/default selection for this disposal; BTreeMap order = deterministic.
        res.selections.insert(disposal.clone(), picks.clone());
    }
    fold(res, prices, config)
}

/// R0-M1 precondition check: does every injected pick-set conserve its disposal's principal?
///
/// `fold_with` injects straight into `res.selections`, bypassing `resolve`'s `Σ == principal` guard;
/// and the fold's `consume_picks` hardcodes `shortfall = 0` while `selection_feasible` checks only
/// per-lot availability — NOT the sum. A non-conserving assignment would therefore under-consume
/// *silently* (no blocker) → a falsely-low score. So we fold the BASELINE once (no injection), map
/// each disposal → `Σ legs.sat` (its principal), and require every injected entry's `Σ picks.sat` to
/// match. A disposal id absent from the baseline, or a zero principal, ⇒ fail.
fn assignment_conserves_principal(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    assignment: &BTreeMap<EventId, Vec<LotPick>>,
) -> bool {
    let base = project(events, prices, config);
    let mut principal: BTreeMap<EventId, Sat> = BTreeMap::new();
    for d in &base.disposals {
        let sum: Sat = d.legs.iter().map(|l| l.sat).sum();
        principal.insert(d.event.clone(), sum);
    }
    assignment.iter().all(|(disposal, picks)| {
        let picked: Sat = picks.iter().map(|p| p.sat).sum();
        matches!(principal.get(disposal), Some(&p) if p > 0 && p == picked)
    })
}

/// Holistic score: B's federal `TaxOutcome` for `year` under `assignment`. Inject the per-disposal
/// selections, **fold once**, run `compute_tax_year`. Side-effect-free (clone-fold-discard),
/// deterministic (NFR4), exact (NFR5 — all dollars come straight from B; C never re-rounds).
///
/// An infeasible selection (cross-disposal contention / over-draw / unknown or cross-wallet lot)
/// folds to a hard `LotSelectionInvalid` (the fold's `consume_principal` maps the selection error)
/// → `compute_tax_year` refuses with `NotComputable` — the caller skips that combination.
///
/// **PRECONDITION — principal conservation (R0-M1).** Each injected pick-set MUST satisfy
/// `Σ LotPick.sat == the disposal's principal sat`. `fold_with` injects straight into
/// `res.selections`, bypassing `resolve`'s `Σ == principal` guard, and the fold does NOT enforce the
/// sum (`consume_picks` hardcodes `shortfall = 0`; `selection_feasible` checks only per-lot
/// availability), so a NON-conserving assignment under-consumes *silently* → a falsely-low score.
/// `optimize_year`/`consult_sale` generators always conserve, but this fn is `pub` for reuse + KATs,
/// so it `debug_assert!`s the sum against the per-disposal principal (looked up from a baseline fold).
/// The check runs only under `debug_assert!` (zero release cost).
pub fn score_assignment(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    assignment: &BTreeMap<EventId, Vec<LotPick>>,
) -> TaxOutcome {
    // R0-M1 guard: in debug builds, assert each injected pick-set conserves the disposal's principal.
    debug_assert!(
        assignment_conserves_principal(events, prices, config, assignment),
        "score_assignment: injected assignment violates Σpicks == principal (R0-M1)"
    );
    let state = fold_with(events, prices, config, assignment);
    compute_tax_year(events, &state, year, profile, tables)
}

// ── Task 3 — candidate generation (available-lots pre-pass + bounded-complete vertex enumeration) ──

/// At/below this many available lots a disposal's candidate set is the COMPLETE vertex enumeration
/// (every whole-lot subset + ≤1 partial). Above it, `candidate_selections` returns a deterministic but
/// INCOMPLETE heuristic subset and signals that fact (R2-C1). `2^12 = 4096` masks is the per-pool
/// ceiling — well inside `MAX_COMBOS`.
const LOT_ENUM_BOUND: usize = 12;

/// Lots available to `disposal` at `date` in `wallet`, computed by a clone-fold of the timeline up to
/// (but NOT including) the disposal (NFR4: deterministic; no fold modification). Post-2025 → the
/// disposal's own wallet pool (§1.1012-1(j) per-wallet); pre-2025 → the universal pool. Returns lots
/// with `remaining_sat > 0`, sorted by `lot_id` (a total order).
///
/// **Delegates the fold to `fold::pools_before`** (which mirrors the real fold's canonical + transition
/// ordering AND fires the §7.4 boundary seed at the correct point). This is the fix for the Task-3 review
/// IMPORTANT: the previous "truncate-then-refold" never crossed `TRANSITION_DATE` when the target disposal
/// was the chronologically-first 2025 timeline event, so the re-fold never seeded and returned the
/// UN-seeded Universal residue — correct under Path A (residue relocates by wallet, lot_ids preserved) but
/// WRONG under Path B (the seed DISCARDS the residue and installs allocation lots with different
/// lot_ids/basis). `pools_before` reuses the real `seed_transition`, so the pool matches the live fold for
/// pre-2025 disposals (Universal), Path-A post-2025 (per-wallet relocated), and Path-B post-2025 (seeded
/// lots) — for the FIRST 2025 disposal and every later one. R0-I1 canonical ordering is preserved inside
/// `pools_before`; an absent disposal still yields an empty `Vec` (existence checked here first).
// (allow dead_code: consumed by `optimize_year`'s candidate assembly in Task 4; tested directly here.)
#[allow(dead_code)]
fn available_lots_before(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    disposal: &EventId,
    date: TaxDate,
    wallet: &WalletId,
) -> Vec<Lot> {
    let res = resolve(events, prices, config);
    // "not found ⇒ empty" contract (ordering-independent existence check; `pools_before` would otherwise
    // fold the whole timeline for a missing target).
    if !res.timeline.iter().any(|e| &e.id == disposal) {
        return Vec::new();
    }
    // Pool state just before the disposal, with the transition seed already applied at the correct
    // boundary (Path A drain / Path B seed) — matches the real fold under both paths (Task-3 IMPORTANT).
    let pools = pools_before(res, prices, config, disposal);
    let want = pool_key(date, wallet); // post-2025 → Wallet(wallet); pre-2025 → Universal
    let mut lots: Vec<Lot> = pools
        .pools
        .into_values()
        .flatten()
        .filter(|l| l.remaining_sat > 0 && pool_key(date, &l.wallet) == want)
        .collect();
    lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id)); // total order (NFR4)
    lots
}

/// All principal-conserving vertex selections of `need` sats over `lots` (same pool): every whole-lot
/// subset summing to `need`, plus each strict subset (Σ < `need`) extended by ONE partial lot to reach
/// `need`. Deduped + sorted (NFR4). On pools `> LOT_ENUM_BOUND`, a deterministic heuristic set instead.
///
/// **Returns `(candidates, heuristic)` (R2-C1).** `heuristic == false` ⇔ the COMPLETE vertex set was
/// enumerated (`lots.len() <= LOT_ENUM_BOUND`); `heuristic == true` ⇔ the pool exceeded the bound and
/// only a deterministic INCOMPLETE subset was returned — the caller (`optimize_year`, Task 4) MUST then
/// flag the proposal `approximate = true, PoolHeuristic { .. }` (a heuristic-pool result is not a proven
/// global minimum). Without this signal a single `> 12`-lot pool would score `approximate = false` and
/// render as "the optimum" — the headline-forbidden false-global claim.
// (allow dead_code: consumed by `optimize_year`'s candidate assembly in Task 4; tested directly here.)
#[allow(dead_code)]
fn candidate_selections(lots: &[Lot], need: Sat) -> (Vec<Vec<LotPick>>, bool) {
    let heuristic = lots.len() > LOT_ENUM_BOUND; // R2-C1: did we take the incomplete branch?
    let mut out: std::collections::BTreeSet<Vec<LotPick>> = std::collections::BTreeSet::new();
    // canonical key for a pick-set: sort the picks by lot id (so the BTreeSet dedups by identity).
    let canon = |mut v: Vec<LotPick>| {
        v.sort_by(|a, b| a.lot.cmp(&b.lot));
        v
    };

    if lots.len() <= LOT_ENUM_BOUND {
        // complete vertex enumeration over 2^n subsets (n ≤ 12)
        for mask in 0u32..(1u32 << lots.len()) {
            let mut whole: Vec<LotPick> = Vec::new();
            let mut sum: Sat = 0;
            for (i, l) in lots.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    whole.push(LotPick {
                        lot: l.lot_id.clone(),
                        sat: l.remaining_sat,
                    });
                    sum += l.remaining_sat;
                }
            }
            if sum == need {
                out.insert(canon(whole)); // whole-lot vertex
            } else if sum < need {
                let short = need - sum; // top up with ONE partial lot not in the mask
                for (i, l) in lots.iter().enumerate() {
                    if mask & (1 << i) == 0 && l.remaining_sat >= short {
                        let mut v = whole.clone();
                        v.push(LotPick {
                            lot: l.lot_id.clone(),
                            sat: short,
                        });
                        out.insert(canon(v));
                    }
                }
            }
        }
    } else {
        // deterministic heuristic generators (greedy-fill in a given lot order)
        let fill = |order: Vec<usize>| -> Option<Vec<LotPick>> {
            let mut v = Vec::new();
            let mut rem = need;
            for i in order {
                if rem <= 0 {
                    break;
                }
                let take = rem.min(lots[i].remaining_sat);
                if take > 0 {
                    v.push(LotPick {
                        lot: lots[i].lot_id.clone(),
                        sat: take,
                    });
                    rem -= take;
                }
            }
            (rem == 0).then(|| canon(v))
        };
        let by = |key: &dyn Fn(&Lot, &Lot) -> std::cmp::Ordering| {
            let mut ix: Vec<usize> = (0..lots.len()).collect();
            ix.sort_by(|&a, &b| key(&lots[a], &lots[b]));
            ix
        };
        use std::cmp::Ordering;
        let hifo = |a: &Lot, b: &Lot| {
            (b.usd_basis * Usd::from(a.remaining_sat))
                .cmp(&(a.usd_basis * Usd::from(b.remaining_sat)))
                .then(a.acquired_at.cmp(&b.acquired_at))
                .then(a.lot_id.cmp(&b.lot_id))
        };
        let fifo = |a: &Lot, b: &Lot| {
            a.acquired_at
                .cmp(&b.acquired_at)
                .then(a.lot_id.cmp(&b.lot_id))
        };
        let lifo = |a: &Lot, b: &Lot| {
            b.acquired_at
                .cmp(&a.acquired_at)
                .then(b.lot_id.cmp(&a.lot_id))
        };
        let lt_first = |a: &Lot, b: &Lot| {
            a.gain_hp_start()
                .cmp(&b.gain_hp_start())
                .then(a.lot_id.cmp(&b.lot_id))
        };
        for k in [
            &hifo as &dyn Fn(&Lot, &Lot) -> Ordering,
            &fifo,
            &lifo,
            &lt_first,
        ] {
            if let Some(v) = fill(by(k)) {
                out.insert(v);
            }
        }
        for lead in 0..lots.len() {
            // per-lot lead, then HIFO-fill
            let mut order = vec![lead];
            order.extend(by(&hifo).into_iter().filter(|&i| i != lead));
            if let Some(v) = fill(order) {
                out.insert(v);
            }
        }
    }
    (out.into_iter().collect(), heuristic) // (sorted Vec — NFR4, heuristic-branch flag — R2-C1)
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

/// Task 3 — candidate generation KATs. These are in-crate UNIT tests (not `tests/`) because
/// `available_lots_before` and `candidate_selections` are private generators (the §2 public surface is
/// the documented `optimize_year`/`consult_sale`/`score_assignment`); a `tests/` integration crate
/// cannot see them, while a child module can. All fixtures are synthetic (privacy — no real reads).
#[cfg(test)]
mod candidate_tests {
    use super::*;
    use crate::event::{
        Acquire, AllocLot, AllocMethod, BasisSource, Dispose, EventPayload, SafeHarborAllocation,
    };
    use crate::identity::{LotId, Source, SourceRef};
    use crate::price::StaticPrices;
    use crate::LotMethod;
    use rust_decimal_macros::dec;
    use std::collections::BTreeSet;
    use time::macros::{date, datetime, offset};

    const LOT: Sat = 100_000_000; // one whole BTC per lot

    // ── builders ─────────────────────────────────────────────────────────────────────────────────
    fn cold() -> WalletId {
        WalletId::SelfCustody {
            label: "cold".into(),
        }
    }
    fn hot() -> WalletId {
        WalletId::SelfCustody {
            label: "hot".into(),
        }
    }
    fn eid(rf: &str) -> EventId {
        EventId::import(Source::Swan, SourceRef::new(rf))
    }
    fn lid(rf: &str) -> LotId {
        LotId {
            origin_event_id: eid(rf),
            split_sequence: 0,
        }
    }
    fn pick(rf: &str, sat: Sat) -> LotPick {
        LotPick { lot: lid(rf), sat }
    }
    /// Test-local mirror of `candidate_selections`'s canonicalization (picks sorted by lot id), so
    /// expected sets can be compared regardless of the EventId hash order.
    fn canon(mut v: Vec<LotPick>) -> Vec<LotPick> {
        v.sort_by(|a, b| a.lot.cmp(&b.lot));
        v
    }
    /// A whole `Lot` built directly (for the pure `candidate_selections`).
    fn mklot(rf: &str, acquired: TaxDate, sat: Sat, basis: Usd, wallet: WalletId) -> Lot {
        Lot {
            lot_id: lid(rf),
            wallet,
            acquired_at: acquired,
            original_sat: sat,
            remaining_sat: sat,
            usd_basis: basis,
            basis_source: BasisSource::ExchangeProvided,
            dual_loss_basis: None,
            donor_acquired_at: None,
            basis_pending: false,
        }
    }
    fn ev(rf: &str, ts: time::OffsetDateTime, wallet: WalletId, p: EventPayload) -> LedgerEvent {
        LedgerEvent {
            id: eid(rf),
            utc_timestamp: ts,
            original_tz: offset!(+00:00),
            wallet: Some(wallet),
            payload: p,
        }
    }
    fn buy(
        rf: &str,
        ts: time::OffsetDateTime,
        wallet: WalletId,
        sat: Sat,
        cost: Usd,
    ) -> LedgerEvent {
        ev(
            rf,
            ts,
            wallet,
            EventPayload::Acquire(Acquire {
                sat,
                usd_cost: cost,
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        )
    }
    fn sell(
        rf: &str,
        ts: time::OffsetDateTime,
        wallet: WalletId,
        sat: Sat,
        proceeds: Usd,
    ) -> LedgerEvent {
        ev(
            rf,
            ts,
            wallet,
            EventPayload::Dispose(Dispose {
                sat,
                usd_proceeds: proceeds,
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        )
    }
    fn cfg() -> ProjectionConfig {
        ProjectionConfig::default()
    }
    /// An effective (attested ⇒ §5.02(4) bar bypassed) `ActualPosition` safe-harbor allocation decision
    /// event (Path B). `id = EventId::decision(seq)` ⇒ the seed lots' `origin_event_id` is THIS decision,
    /// distinct from any imported buy's lot id. Recorded method FIFO; `as_of_date` the 2025-01-01 snapshot.
    fn alloc_event(seq: u64, made: time::OffsetDateTime, lots: Vec<AllocLot>) -> LedgerEvent {
        LedgerEvent {
            id: EventId::decision(seq),
            utc_timestamp: made,
            original_tz: offset!(+00:00),
            wallet: None,
            payload: EventPayload::SafeHarborAllocation(SafeHarborAllocation {
                lots,
                as_of_date: date!(2025 - 01 - 01),
                method: AllocMethod::ActualPosition,
                timely_allocation_attested: true, // bypass the §5.02(4) bar so Path B governs
                pre2025_method: LotMethod::Fifo,
            }),
        }
    }
    fn alloc_lot(w: WalletId, sat: Sat, basis: Usd, acq: TaxDate) -> AllocLot {
        AllocLot {
            wallet: w,
            sat,
            usd_basis: basis,
            acquired_at: acq,
            dual_loss_basis: None,
            donor_acquired_at: None,
        }
    }

    // ── candidate_selections (pure vertex enumeration) ─────────────────────────────────────────────

    /// Complete vertex set on a small pool: three whole 100k lots, `need = 200k` → exactly the whole-lot
    /// pairs `{A,B},{A,C},{B,C}` (the brute-force vertex set), each conserving principal; `heuristic`
    /// false (≤ LOT_ENUM_BOUND).
    #[test]
    fn complete_vertex_set_three_whole_lots() {
        let lots = vec![
            mklot("A", date!(2025 - 02 - 01), LOT, dec!(10000), cold()),
            mklot("B", date!(2025 - 03 - 01), LOT, dec!(20000), cold()),
            mklot("C", date!(2025 - 04 - 01), LOT, dec!(30000), cold()),
        ];
        let (cands, heuristic) = candidate_selections(&lots, 2 * LOT);
        assert!(!heuristic, "≤ LOT_ENUM_BOUND ⇒ complete enumeration");
        let got: BTreeSet<Vec<LotPick>> = cands.iter().cloned().collect();
        let want: BTreeSet<Vec<LotPick>> = [
            canon(vec![pick("A", LOT), pick("B", LOT)]),
            canon(vec![pick("A", LOT), pick("C", LOT)]),
            canon(vec![pick("B", LOT), pick("C", LOT)]),
        ]
        .into_iter()
        .collect();
        assert_eq!(got, want, "enumerated vertices == brute-force vertex set");
        for c in &cands {
            assert_eq!(
                c.iter().map(|p| p.sat).sum::<Sat>(),
                2 * LOT,
                "principal conservation"
            );
        }
    }

    /// One-partial top-up: three 100k lots, `need = 150k` (no whole-lot subset sums to it) → every
    /// strict subset summing < need extended by ONE partial; the full set is the six (whole + partial)
    /// vertices including `{A(100k),B(50k)}` and `{B(100k),A(50k)}`. All conserve; `heuristic` false.
    #[test]
    fn one_partial_top_up_vertices() {
        let half = LOT / 2;
        let lots = vec![
            mklot("A", date!(2025 - 02 - 01), LOT, dec!(10000), cold()),
            mklot("B", date!(2025 - 03 - 01), LOT, dec!(20000), cold()),
            mklot("C", date!(2025 - 04 - 01), LOT, dec!(30000), cold()),
        ];
        let (cands, heuristic) = candidate_selections(&lots, LOT + half);
        assert!(!heuristic);
        let got: BTreeSet<Vec<LotPick>> = cands.iter().cloned().collect();
        let want: BTreeSet<Vec<LotPick>> = [
            canon(vec![pick("A", LOT), pick("B", half)]),
            canon(vec![pick("A", LOT), pick("C", half)]),
            canon(vec![pick("B", LOT), pick("A", half)]),
            canon(vec![pick("B", LOT), pick("C", half)]),
            canon(vec![pick("C", LOT), pick("A", half)]),
            canon(vec![pick("C", LOT), pick("B", half)]),
        ]
        .into_iter()
        .collect();
        assert_eq!(got, want);
        // explicit membership of the two distinct "lead lot + 50k partial" forms (R0/plan example)
        assert!(got.contains(&canon(vec![pick("A", LOT), pick("B", half)])));
        assert!(got.contains(&canon(vec![pick("B", LOT), pick("A", half)])));
        for c in &cands {
            assert_eq!(c.iter().map(|p| p.sat).sum::<Sat>(), LOT + half);
        }
    }

    /// NFR4 determinism: byte-identical `Vec` across calls.
    #[test]
    fn candidate_selections_is_deterministic() {
        let lots = vec![
            mklot("A", date!(2025 - 02 - 01), LOT, dec!(10000), cold()),
            mklot("B", date!(2025 - 03 - 01), LOT, dec!(20000), cold()),
            mklot("C", date!(2025 - 04 - 01), LOT, dec!(30000), cold()),
        ];
        let (c1, h1) = candidate_selections(&lots, 2 * LOT);
        let (c2, h2) = candidate_selections(&lots, 2 * LOT);
        assert_eq!(c1, c2);
        assert_eq!(h1, h2);
    }

    /// R2-C1, `<= bound`: a pool of exactly LOT_ENUM_BOUND lots → `heuristic == false` (complete).
    #[test]
    fn heuristic_flag_false_at_bound() {
        let lots: Vec<Lot> = (0..LOT_ENUM_BOUND)
            .map(|i| {
                mklot(
                    &format!("L{i}"),
                    date!(2025 - 02 - 01),
                    LOT,
                    dec!(10000),
                    cold(),
                )
            })
            .collect();
        let (_cands, heuristic) = candidate_selections(&lots, 2 * LOT);
        assert!(!heuristic, "exactly LOT_ENUM_BOUND lots ⇒ still complete");
    }

    /// R2-C1, `> bound`: a pool of 13 lots (> LOT_ENUM_BOUND) → `heuristic == true` and the returned
    /// candidates are a STRICT SUBSET of the full vertex set (here all whole-lot pairs, C(13,2) = 78),
    /// each still principal-conserving. This is the incomplete-branch signal `optimize_year` propagates
    /// into `approximate`/`PoolHeuristic`.
    #[test]
    fn heuristic_flag_true_above_bound_returns_strict_subset() {
        let n = LOT_ENUM_BOUND + 1; // 13
        let lots: Vec<Lot> = (0..n)
            .map(|i| {
                mklot(
                    &format!("L{i:02}"),
                    date!(2025 - 02 - 01),
                    LOT,
                    dec!(10000),
                    cold(),
                )
            })
            .collect();
        let (cands, heuristic) = candidate_selections(&lots, 2 * LOT);
        assert!(heuristic, "> LOT_ENUM_BOUND ⇒ heuristic branch");
        // full vertex set for equal 100k lots / need=200k = all unordered whole-lot pairs.
        let mut full: BTreeSet<Vec<LotPick>> = BTreeSet::new();
        for i in 0..n {
            for j in (i + 1)..n {
                full.insert(canon(vec![
                    LotPick {
                        lot: lots[i].lot_id.clone(),
                        sat: LOT,
                    },
                    LotPick {
                        lot: lots[j].lot_id.clone(),
                        sat: LOT,
                    },
                ]));
            }
        }
        assert_eq!(full.len(), 78);
        let got: BTreeSet<Vec<LotPick>> = cands.iter().cloned().collect();
        assert!(got.is_subset(&full), "heuristic candidates are vertices");
        assert!(
            got.len() < full.len(),
            "STRICT subset (incomplete): {} of {}",
            got.len(),
            full.len()
        );
        for c in &cands {
            assert_eq!(c.iter().map(|p| p.sat).sum::<Sat>(), 2 * LOT);
        }
    }

    // ── available_lots_before (pre-pass: fold::pools_before — canonical + transition seed at boundary) ─

    /// R0-I1 — load-order ≠ canonical-order. A lot acquired EARLIER in time but appended LATER in
    /// `events`, and a lot acquired LATER in time but appended EARLIER. `available_lots_before(D)` must
    /// return exactly the lots acquired-before-`D`-in-TIME: the early-time/late-load lot PRESENT, the
    /// late-time/early-load lot ABSENT. Without the `sort_canonical` + partition replication (now inside
    /// `fold::pools_before`) this fails (it would cut on load order and keep the wrong lot).
    #[test]
    fn available_lots_before_respects_canonical_not_load_order() {
        // load order: LATE (09-01), D (06-01), EARLY (02-01) — deliberately NOT time order.
        let events = vec![
            buy(
                "LATE",
                datetime!(2025-09-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(50000),
            ),
            sell(
                "D",
                datetime!(2025-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(60000),
            ),
            buy(
                "EARLY",
                datetime!(2025-02-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(10000),
            ),
        ];
        let prices = StaticPrices::default();
        let lots = available_lots_before(
            &events,
            &prices,
            &cfg(),
            &eid("D"),
            date!(2025 - 06 - 01),
            &cold(),
        );
        let ids: BTreeSet<LotId> = lots.iter().map(|l| l.lot_id.clone()).collect();
        assert!(
            ids.contains(&lid("EARLY")),
            "acquired-before-D-in-time ⇒ available"
        );
        assert!(
            !ids.contains(&lid("LATE")),
            "acquired-after-D-in-time ⇒ NOT available"
        );
    }

    /// Per-wallet (§1.1012-1(j)): a lot in another wallet is EXCLUDED from a post-2025 disposal's pool.
    #[test]
    fn available_lots_before_excludes_cross_wallet_lot() {
        let events = vec![
            buy(
                "CL",
                datetime!(2025-02-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(10000),
            ),
            buy(
                "HL",
                datetime!(2025-02-01 00:00:00 UTC),
                hot(),
                LOT,
                dec!(10000),
            ),
            sell(
                "D",
                datetime!(2025-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(60000),
            ),
        ];
        let prices = StaticPrices::default();
        let lots = available_lots_before(
            &events,
            &prices,
            &cfg(),
            &eid("D"),
            date!(2025 - 06 - 01),
            &cold(),
        );
        let ids: BTreeSet<LotId> = lots.iter().map(|l| l.lot_id.clone()).collect();
        assert!(ids.contains(&lid("CL")), "same-wallet lot available");
        assert!(
            !ids.contains(&lid("HL")),
            "cross-wallet lot excluded (per-wallet pool)"
        );
    }

    /// Task-3 review IMPORTANT — Path B, the FIRST 2025 timeline event IS the disposal. An effective
    /// `SafeHarborAllocation` (Path B) DISCARDS the pre-2025 FIFO residue and installs seed lots whose
    /// `lot_id`s (origin = the allocation decision) and per-lot basis DIFFER from the residue. Because
    /// decision events are not timeline events (resolve.rs:491), this sell is the chronologically-first
    /// ≥2025 timeline event, so the boundary seed must STILL fire before it. `available_lots_before` MUST
    /// return the SEEDED lots (origin = allocation, `basis_source == SafeHarborAllocated`), NOT the
    /// discarded residue (origin = the pre-2025 buy). FAILS without the `pools_before` seed-at-boundary
    /// fix (the old truncate-then-refold never crosses TRANSITION_DATE → surfaces the un-seeded residue).
    #[test]
    fn available_lots_before_path_b_first_2025_disposal_returns_seeded_lots() {
        let alloc_id = EventId::decision(1);
        let events = vec![
            // pre-2025 buy → Universal residue: 100M sat @ basis 60 (origin = "OLD", ExchangeProvided).
            buy(
                "OLD",
                datetime!(2024-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(60),
            ),
            // FIRST 2025 timeline event is THIS disposal (the allocation below is a decision, not a tl event).
            sell(
                "D",
                datetime!(2025-02-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(80),
            ),
            // Effective Path-B allocation: seed = 40M@20 + 60M@40 = 100M @ 60 — conserves vs the residue,
            // but DIFFERENT lot_ids (origin = decision id) AND per-lot basis than the single residue lot.
            alloc_event(
                1,
                datetime!(2025-03-01 00:00:00 UTC),
                vec![
                    alloc_lot(cold(), 40_000_000, dec!(20), date!(2024 - 05 - 01)),
                    alloc_lot(cold(), 60_000_000, dec!(40), date!(2024 - 06 - 01)),
                ],
            ),
        ];
        let prices = StaticPrices::default();
        let lots = available_lots_before(
            &events,
            &prices,
            &cfg(),
            &eid("D"),
            date!(2025 - 02 - 01),
            &cold(),
        );
        assert!(!lots.is_empty(), "the post-seed Path-B pool is non-empty");
        assert!(
            lots.iter().all(|l| l.lot_id.origin_event_id == alloc_id),
            "Path-B first-2025-disposal MUST return the SEEDED lots (origin = allocation); got origins {:?}",
            lots.iter()
                .map(|l| l.lot_id.origin_event_id.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            lots.iter()
                .all(|l| l.basis_source == BasisSource::SafeHarborAllocated),
            "seeded lots carry the SafeHarborAllocated basis_source (not the residue's ExchangeProvided)"
        );
        let ids: BTreeSet<LotId> = lots.iter().map(|l| l.lot_id.clone()).collect();
        assert!(
            !ids.contains(&lid("OLD")),
            "the DISCARDED FIFO residue lot must NOT appear in a Path-B pool"
        );
        assert_eq!(lots.len(), 2, "both seed lots present");
        assert_eq!(
            lots.iter().map(|l| l.remaining_sat).sum::<Sat>(),
            LOT,
            "seed conserves principal"
        );
        assert_eq!(
            lots.iter().map(|l| l.usd_basis).sum::<Usd>(),
            dec!(60),
            "seed conserves basis"
        );
    }

    /// Path-A counterpart of the above — FIRST 2025 event is the disposal, NO allocation (Path A default).
    /// The pre-2025 Universal residue must be RELOCATED into its wallet pool (lot_id + basis preserved,
    /// `basis_source == ReconstructedPerWallet`) by the boundary seed before the disposal. Confirms the
    /// seed fires at the boundary under Path A too — and that `available_lots_before` matches the real fold.
    #[test]
    fn available_lots_before_path_a_first_2025_disposal_relocates_residue() {
        let events = vec![
            buy(
                "OLD",
                datetime!(2024-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(60),
            ),
            sell(
                "D",
                datetime!(2025-02-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(80),
            ),
        ];
        let prices = StaticPrices::default();
        let lots = available_lots_before(
            &events,
            &prices,
            &cfg(),
            &eid("D"),
            date!(2025 - 02 - 01),
            &cold(),
        );
        assert_eq!(lots.len(), 1, "the single relocated residue lot");
        let l = &lots[0];
        assert_eq!(l.lot_id, lid("OLD"), "Path A preserves the residue lot_id");
        assert_eq!(l.usd_basis, dec!(60), "Path A preserves basis");
        assert_eq!(l.remaining_sat, LOT);
        assert_eq!(
            l.basis_source,
            BasisSource::ReconstructedPerWallet,
            "Path A drains the residue into the per-wallet pool at the boundary seed"
        );
    }

    /// NFR4 determinism for the pre-pass.
    #[test]
    fn available_lots_before_is_deterministic() {
        let events = vec![
            buy(
                "A",
                datetime!(2025-02-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(10000),
            ),
            buy(
                "B",
                datetime!(2025-03-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(20000),
            ),
            sell(
                "D",
                datetime!(2025-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(60000),
            ),
        ];
        let prices = StaticPrices::default();
        let l1 = available_lots_before(
            &events,
            &prices,
            &cfg(),
            &eid("D"),
            date!(2025 - 06 - 01),
            &cold(),
        );
        let l2 = available_lots_before(
            &events,
            &prices,
            &cfg(),
            &eid("D"),
            date!(2025 - 06 - 01),
            &cold(),
        );
        assert_eq!(l1, l2);
    }
}
