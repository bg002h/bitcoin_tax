//! Sub-project C — rate-aware optimizer. ASSIGNS lots to disposals (specific identification);
//! it does NOT advise whether to sell/hold (no investment advice — §C scope). Minimizes B's
//! federal `total_federal_tax_attributable` over feasible per-disposal `LotSelection`s, within the
//! §1.1012-1(j) identification boundary (adequate ID by the time of sale; no compliant post-hoc).
//! Deterministic (NFR4) + exact (NFR5): BTreeMap/sorted iteration, Decimal/i64 only, no float.
//!
//! ## §1091 wash sale (C.5) — crypto is currently EXEMPT.
//! §1091 disallows a loss only on "stock or securities"; the IRS treats convertible virtual currency
//! as property, not a security (Notice 2014-21; Rev. Rul. 2023-14), and **no statute extending §1091
//! to crypto has been enacted** (only recurring Greenbook/legislative proposals). The optimizer
//! therefore selects loss lots **freely** — loss harvesting is unconstrained, and a chosen loss is
//! never disallowed/deferred here. Form 1099-DA box 1i reports wash-sale disallowances only for
//! assets that are in fact securities — not a change to crypto. **MONITOR for enactment**; if §1091
//! is extended, loss-lot selection must add a disallowance rule and this note must change in lockstep
//! (FOLLOWUPS.md — C.5).
use crate::conventions::{is_long_term, one_year_after, Sat, TaxDate, Usd, TRANSITION_DATE};
use crate::event::{DisposeKind, LedgerEvent, LotPick};
use crate::identity::{EventId, LotId, SourceRef, WalletId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::fold::{fold, pools_before, state_as_of};
use crate::project::pools::{pool_key, PoolKey};
use crate::project::resolve::{resolve, Eff, Op};
use crate::project::{
    disposal_compliance, evaluate_disposal, project, CandidateDisposal, ComplianceStatus,
    DisposalCompliance, EvaluateError, ProjectionConfig,
};
use crate::state::{Blocker, LedgerState, Lot};
use crate::tax::{compute_tax_year, MarginalRates, TaxOutcome, TaxProfile, TaxResult, TaxTables};
use std::collections::{BTreeMap, BTreeSet};

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
fn available_lots_before(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    disposal: &EventId,
    date: TaxDate,
    wallet: &WalletId,
) -> Vec<Lot> {
    available_lots_before_with(
        events,
        prices,
        config,
        disposal,
        date,
        wallet,
        &BTreeMap::new(),
    )
}

/// As `available_lots_before`, but with `injected` per-disposal selections folded in FIRST (so an earlier
/// group member's chosen candidate is consumed before this disposal's pool is read). This is the engine of
/// the nested joint enumeration of a contention group (R0-C3): each later disposal draws from the pool left
/// by the prior members' CHOSEN picks, not the baseline — which is exactly what recovers cross-period
/// reassignment optima (a lot ST at `D1`'s date but LT at `D2`'s date). An empty `injected` map degenerates
/// to the plain baseline pre-pass. Deterministic: `injected` is a `BTreeMap` (NFR4).
fn available_lots_before_with(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    disposal: &EventId,
    date: TaxDate,
    wallet: &WalletId,
    injected: &BTreeMap<EventId, Vec<LotPick>>,
) -> Vec<Lot> {
    let mut res = resolve(events, prices, config);
    // "not found ⇒ empty" contract (ordering-independent existence check; `pools_before` would otherwise
    // fold the whole timeline for a missing target).
    if !res.timeline.iter().any(|e| &e.id == disposal) {
        return Vec::new();
    }
    // Override the consumption of the already-chosen group members (BTreeMap order = deterministic).
    for (d, picks) in injected {
        res.selections.insert(d.clone(), picks.clone());
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

// ── Task 5 — pure compliance / persistability helpers (consumed by `optimize_year`'s row build) ────
//
// These pure functions are owned by Task 5 (with their dedicated KAT suite); Task 4's per-disposal row
// construction depends on them (R0-C2), so they live here and are exercised end-to-end by the Mode-1
// optimality/refusal KATs. `optimize_year` judges a PROPOSED pick by its OWN made-date — a standing order
// never rescues a divergent post-hoc cherry-pick (§1.1012-1(j)).

/// §A.5 custody → envelope (R2-M5): `Exchange` = broker (own-books insufficient 2027+); `SelfCustody` =
/// own-books, all years, no relief ever needed.
fn is_broker(w: &WalletId) -> bool {
    matches!(w, WalletId::Exchange { .. })
}

/// The §C.2 `accept`-gate verdict for one disposal (computed in core; enforced by the CLI, Task 10).
/// - made-date ≤ sale → §A.5(b) `Contemporaneous` lever; persist freely (`ContemporaneousNow`).
/// - already-executed (made-date > sale) AND 2027+ broker-held → `ForbiddenBroker2027` (NEVER persist).
/// - already-executed otherwise (self-custody any year, or broker-held 2025–2026) → `NeedsAttestation`.
pub fn persistability(
    wallet: &WalletId,
    sale_date: TaxDate,
    selection_made: TaxDate,
) -> Persistability {
    if selection_made <= sale_date {
        Persistability::ContemporaneousNow
    } else if is_broker(wallet) && sale_date.year() >= 2027 {
        Persistability::ForbiddenBroker2027
    } else {
        Persistability::NeedsAttestation
    }
}

/// R0-C2: compliance status of a PROPOSED pick, judged by ITS OWN timeliness. The proposed pick is a
/// would-be `LotSelection` made at `made` (= proposal/now), NOT a persisted selection in `events`;
/// `disposal_compliance(events, …)` would skip the selection branch and let a standing order RESCUE a
/// divergent post-hoc cherry-pick as `StandingOrder` (FORBIDDEN — §1.1012-1(j)). So this judges it directly:
/// - `proposed == current` (no change): keep `baseline_status` — adopting an identical pick binds nothing
///   new (`accept` skips it). **This is the ONLY path that may report `StandingOrder`.**
/// - diverges: 2027+ broker → `NonCompliant`; else `made ≤ sale` → `Contemporaneous`; else (post-hoc) →
///   `NonCompliant`. The overlay may later upgrade a post-hoc `NonCompliant` → `AttestedRecording` ONLY
///   when attested AND within the own-books envelope AND unchanged.
pub fn proposed_compliance_status(
    wallet: &WalletId,
    sale_date: TaxDate,
    made: TaxDate,
    proposed: &[LotPick],
    current: &[LotPick],
    baseline_status: &ComplianceStatus,
) -> ComplianceStatus {
    if proposed == current {
        return baseline_status.clone(); // no divergence ⇒ the real, already-established status stands
    }
    if is_broker(wallet) && sale_date.year() >= 2027 {
        return ComplianceStatus::NonCompliant;
    }
    if made <= sale_date {
        ComplianceStatus::Contemporaneous
    } else {
        ComplianceStatus::NonCompliant // divergent post-hoc cherry-pick — NEVER StandingOrder
    }
}

/// Upgrade A's per-disposal compliance for the `optimize` surface: a `NonCompliant` disposal is upgraded
/// to `AttestedRecording` IFF (a) it is in `attested`, (b) it is in `unchanged` (the PROPOSED pick equals
/// the in-force persisted-and-attested selection — R2-I1: an attestation binds only the exact attested
/// selection, never a divergent re-run pick), AND (c) it is within the envelope (NOT 2027+ broker-held).
/// `StandingOrder`/`Contemporaneous` rows are left untouched; a non-attested OR divergent post-hoc
/// selection stays `NonCompliant` (the conservative direction).
pub fn compliance_overlay(
    base: &[DisposalCompliance],
    attested: &BTreeSet<EventId>,
    unchanged: &BTreeSet<EventId>,
) -> Vec<DisposalCompliance> {
    base.iter()
        .map(|c| {
            let upgrade = matches!(c.status, ComplianceStatus::NonCompliant)
                && attested.contains(&c.disposal)
                && unchanged.contains(&c.disposal) // R2-I1: attestation binds only the attested pick
                && !(is_broker(&c.wallet) && c.date.year() >= 2027);
            let mut out = c.clone();
            if upgrade {
                out.status = ComplianceStatus::AttestedRecording;
            }
            out
        })
        .collect()
}

// ── Task 4 — contention grouping + joint candidate enumeration (R0-C3) ─────────────────────────────

/// Per-group joint-enumeration ceiling (≤ `MAX_COMBOS`). Beyond it a contended group falls back to
/// per-disposal-independent generation and the proposal is flagged `ContentionUnenumerated`.
const GROUP_COMBO_BOUND: usize = 4_096;

/// Partition the year's `targets` into contention groups: disposals sharing one `PoolKey::Wallet` pool
/// whose `available_lots_before` (baseline) lot-id sets OVERLAP are one group; a non-overlapping disposal
/// is its own singleton group. Deterministic (NFR4): union-find over pre-sorted `targets` (EventId order),
/// members ascending, groups ordered by their first member's EventId.
fn contention_groups(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    targets: &[(EventId, WalletId, TaxDate, Sat)],
) -> Vec<Vec<usize>> {
    let n = targets.len();
    // Per-target (pool key, available-lot-id set) under the baseline consumption.
    let infos: Vec<(PoolKey, BTreeSet<LotId>)> = targets
        .iter()
        .map(|(id, wallet, date, _need)| {
            let lots = available_lots_before(events, prices, config, id, *date, wallet);
            let ids: BTreeSet<LotId> = lots.into_iter().map(|l| l.lot_id).collect();
            (pool_key(*date, wallet), ids)
        })
        .collect();

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]]; // path-halving
            x = parent[x];
        }
        x
    }
    let mut parent: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in (i + 1)..n {
            // Same wallet pool AND overlapping available lots ⇒ contended (reassignment may help).
            if infos[i].0 == infos[j].0 && !infos[i].1.is_disjoint(&infos[j].1) {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    // Group by root; members pushed in ascending index order (== ascending EventId, targets pre-sorted).
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        by_root.entry(r).or_default().push(i);
    }
    let mut groups: Vec<Vec<usize>> = by_root.into_values().collect();
    groups.sort_by(|a, b| targets[a[0]].0.cmp(&targets[b[0]].0));
    groups
}

/// One alias for the (verbose) joint-enumeration return: the per-sequence partial assignments plus the
/// largest heuristic pool's lot-count (`Some` ⇒ a nested `candidate_selections` took the `> LOT_ENUM_BOUND`
/// INCOMPLETE branch, so the caller flags `PoolHeuristic`).
type JointMaps = (Vec<BTreeMap<EventId, Vec<LotPick>>>, Option<usize>);

/// Per-disposal proposal-row metadata threaded between the status pass and the final `DisposalProposal`
/// build: `(disposal, wallet, sale date, current picks, proposed picks)`.
type RowMeta = (EventId, WalletId, TaxDate, Vec<LotPick>, Vec<LotPick>);

/// Joint candidate assignments for ONE contention group, generated by NESTING `candidate_selections` in
/// canonical (time, then EventId) order: the earliest disposal draws from the pre-group pool; each later
/// disposal draws from the pool LEFT by the prior members' chosen candidate (via
/// `available_lots_before_with`, which re-folds with those picks injected). This recovers cross-period
/// reassignment optima the independent per-disposal product cannot reach. Returns `None` when the joint
/// count would exceed `GROUP_COMBO_BOUND` (→ caller flags `ContentionUnenumerated`). Deterministic: the
/// returned maps are sorted + deduped.
fn group_candidate_assignments(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    group: &[(EventId, WalletId, TaxDate, Sat)],
) -> Option<JointMaps> {
    // TIME order (then EventId) so the pool evolution is correct: earlier disposals consume first; later
    // disposals see the remaining pool PLUS lots acquired between them.
    let mut order: Vec<usize> = (0..group.len()).collect();
    order.sort_by(|&a, &b| {
        group[a]
            .2
            .cmp(&group[b].2)
            .then(group[a].0.cmp(&group[b].0))
    });

    let mut partials: Vec<BTreeMap<EventId, Vec<LotPick>>> = vec![BTreeMap::new()];
    let mut max_heur: Option<usize> = None;
    for &mi in &order {
        let (id, wallet, date, need) = &group[mi];
        let mut next: Vec<BTreeMap<EventId, Vec<LotPick>>> = Vec::new();
        for partial in &partials {
            let lots =
                available_lots_before_with(events, prices, config, id, *date, wallet, partial);
            let (cands, heuristic) = candidate_selections(&lots, *need);
            if heuristic {
                max_heur = Some(max_heur.map_or(lots.len(), |m| m.max(lots.len())));
            }
            for c in cands {
                let mut p2 = partial.clone();
                p2.insert(id.clone(), c);
                next.push(p2);
                if next.len() > GROUP_COMBO_BOUND {
                    return None; // beyond the per-group ceiling → caller flags ContentionUnenumerated
                }
            }
        }
        partials = next;
    }
    partials.sort();
    partials.dedup();
    Some((partials, max_heur))
}

/// Independent per-disposal candidate maps for a group that could NOT be jointly enumerated within the
/// bound: each member uses its own `available_lots_before` (baseline) candidates; the group's list is the
/// cartesian product of members' independent lists. Misses cross-period reassignment, hence the caller's
/// `ContentionUnenumerated` flag — but stays baseline-safe (the baseline picks are still seeded separately).
fn independent_group_maps(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    baseline_state: &LedgerState,
    group: &[(EventId, WalletId, TaxDate, Sat)],
) -> Vec<BTreeMap<EventId, Vec<LotPick>>> {
    let mut maps: Vec<BTreeMap<EventId, Vec<LotPick>>> = vec![BTreeMap::new()];
    for (id, wallet, date, need) in group {
        let lots = available_lots_before(events, prices, config, id, *date, wallet);
        let (mut cands, _heuristic) = candidate_selections(&lots, *need);
        if cands.is_empty() {
            cands.push(baseline_selection(baseline_state, id));
        }
        let mut next: Vec<BTreeMap<EventId, Vec<LotPick>>> = Vec::new();
        for m in &maps {
            for c in &cands {
                let mut m2 = m.clone();
                m2.insert(id.clone(), c.clone());
                next.push(m2);
            }
        }
        maps = next;
    }
    maps.sort();
    maps.dedup();
    maps
}

// ── Task 4 — Mode-1 optimizer `optimize_year` ──────────────────────────────────────────────────────

/// Overall cartesian-product ceiling: exhaustive (PROVEN global minimum) below it, baseline-seeded
/// coordinate descent (a disclosed LOCAL optimum) above it.
const MAX_COMBOS: usize = 50_000;

/// §C.1/C.2 holistic single-year optimizer. Assemble per-disposal candidates (grouping + jointly
/// enumerating contended same-wallet disposals — R0-C3), holistically score the cartesian product through
/// B (`score_assignment`), pick the deterministic minimum, and build the what-if `OptimizeProposal`.
///
/// **Baseline-seeded (R0-C1).** The incumbent starts at the current-method (baseline) assignment scored at
/// `base.total_federal_tax_attributable`, so `delta ≤ 0` ALWAYS — the optimizer NEVER recommends an
/// assignment worse than doing nothing, in BOTH the exhaustive and the coordinate-descent path.
///
/// **`approximate` honesty (R2-C1/R0-C1/R0-C3).** `approximate == false` ⇔ the vertex set was FULLY
/// enumerated AND exhaustively scored = a PROVEN global minimum (every pool ≤ `LOT_ENUM_BOUND`, overall
/// product ≤ `MAX_COMBOS`, every contended pool jointly enumerated). Otherwise `approximate == true` with
/// the most-severe `ApproxReason` (precedence `ComboCapExceeded` > `ContentionUnenumerated` >
/// `PoolHeuristic`); `approximate ⇔ approx_reason.is_some()`.
///
/// **Refusals.** Pre-2025 → `PreTransitionYear` (a restatement, not an optimization — M7); a
/// `NotComputable` year → `YearNotComputable` (I6); a year with no method-honoring disposals → `NoDisposals`.
/// Side-effect-free: computes a proposal; appends NOTHING.
///
/// `proposal_made` is the proposed picks' made-date threaded from the CLI seam (core stays clock-free,
/// NFR4); it drives each row's HONEST compliance + persistability. `attested` is the CLI-supplied attested
/// disposal set (empty for pure what-if).
#[allow(clippy::too_many_arguments)]
pub fn optimize_year(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    attested: &BTreeSet<EventId>,
    proposal_made: TaxDate,
) -> Result<OptimizeProposal, OptimizeError> {
    if year < TRANSITION_DATE.year() {
        return Err(OptimizeError::PreTransitionYear(year));
    }
    // Baseline = current filing position (no injected selections).
    let baseline_state = fold_with(events, prices, config, &BTreeMap::new());
    let base = match compute_tax_year(events, &baseline_state, year, profile, tables) {
        TaxOutcome::Computed(r) => r,
        TaxOutcome::NotComputable(b) => return Err(OptimizeError::YearNotComputable(b)),
    };

    // The year's method-honoring disposals (Disposal records for `year`), in EventId order (NFR4).
    let mut targets: Vec<(EventId, WalletId, TaxDate, Sat)> = baseline_state
        .disposals
        .iter()
        .filter(|d| !d.fee_mini_disposition && d.disposed_at.year() == year)
        .filter_map(|d| {
            let wallet = events
                .iter()
                .find(|e| e.id == d.event)
                .and_then(|e| e.wallet.clone())?;
            let sat: Sat = d.legs.iter().map(|l| l.sat).sum();
            Some((d.event.clone(), wallet, d.disposed_at, sat))
        })
        .collect();
    targets.sort_by(|a, b| a.0.cmp(&b.0));
    if targets.is_empty() {
        return Err(OptimizeError::NoDisposals);
    }

    // R0-C3: group into contention groups; each group's candidate list is a Vec of partial assignments
    // (JOINT where contended, independent for singletons). A contended group that cannot be jointly
    // enumerated within GROUP_COMBO_BOUND falls back to independent generation AND flags the proposal.
    let groups = contention_groups(events, prices, config, &targets);
    let mut group_lists: Vec<Vec<BTreeMap<EventId, Vec<LotPick>>>> = Vec::new();
    let mut product: usize = 1;
    let mut approximate = false;
    let mut contended_unenum = 0usize;
    // R2-C1: the largest pool that used the `> LOT_ENUM_BOUND` heuristic (INCOMPLETE) branch.
    let mut pool_heuristic_lots: Option<usize> = None;
    for g in &groups {
        let members: Vec<(EventId, WalletId, TaxDate, Sat)> =
            g.iter().map(|&i| targets[i].clone()).collect();
        let maps: Vec<BTreeMap<EventId, Vec<LotPick>>> = if members.len() == 1 {
            let (id, wallet, date, need) = &members[0]; // singleton: today's independent path
            let lots = available_lots_before(events, prices, config, id, *date, wallet);
            let (mut cands, heuristic) = candidate_selections(&lots, *need);
            if heuristic {
                pool_heuristic_lots =
                    Some(pool_heuristic_lots.map_or(lots.len(), |m| m.max(lots.len())));
            }
            if cands.is_empty() {
                cands.push(baseline_selection(&baseline_state, id));
            }
            cands
                .into_iter()
                .map(|p| BTreeMap::from([(id.clone(), p)]))
                .collect()
        } else {
            match group_candidate_assignments(events, prices, config, &members) {
                Some((joint, heur_lots)) => {
                    if let Some(n) = heur_lots {
                        pool_heuristic_lots = Some(pool_heuristic_lots.map_or(n, |m| m.max(n)));
                    }
                    joint
                }
                None => {
                    approximate = true;
                    contended_unenum += members.len();
                    independent_group_maps(events, prices, config, &baseline_state, &members)
                }
            }
        };
        product = product.saturating_mul(maps.len());
        group_lists.push(maps);
    }
    if pool_heuristic_lots.is_some() {
        approximate = true; // R2-C1: a heuristic pool is never a "proven" optimum
    }

    // R0-C1: BASELINE-SEED so `delta ≤ 0` ALWAYS (never recommend worse-than-doing-nothing).
    let baseline_assignment: BTreeMap<EventId, Vec<LotPick>> = targets
        .iter()
        .map(|(id, ..)| (id.clone(), baseline_selection(&baseline_state, id)))
        .collect();
    // Exhaustive (PROVEN optimum, approximate=false) within MAX_COMBOS; else baseline-seeded coordinate
    // descent (a disclosed LOCAL optimum, approximate=true). Both incumbents START at the baseline score.
    // The tracked `best_total` is the score the search actually selected — baseline-seeded so it is
    // always ≤ baseline_total by construction (the seed is never evicted unless something strictly lower
    // is found). We use it directly for `optimized_tax`/`delta` instead of re-folding `best` to avoid a
    // pro-rata remainder-cent that can shift between ST and LT legs when picks are re-injected in lot-id
    // order rather than the original fold's FIFO order (a ≤$0.01 delta violation on multi-leg disposals).
    let (best, best_total): (BTreeMap<EventId, Vec<LotPick>>, Usd) = if product <= MAX_COMBOS {
        exhaustive_min(
            events,
            prices,
            config,
            year,
            profile,
            tables,
            &group_lists,
            &baseline_assignment,
            &base,
        )
    } else {
        approximate = true;
        coordinate_descent(
            events,
            prices,
            config,
            year,
            profile,
            tables,
            &group_lists,
            &baseline_assignment,
            &base,
        )
    };
    // Reason precedence (R2-C1): blown overall product > un-enumerated contention > per-pool heuristic. All
    // three set `approximate`; precedence only picks which (most-severe) reason is reported.
    let approx_reason = if product > MAX_COMBOS {
        Some(ApproxReason::ComboCapExceeded {
            combos: product,
            cap: MAX_COMBOS,
        })
    } else if contended_unenum > 0 {
        Some(ApproxReason::ContentionUnenumerated {
            contended: contended_unenum,
            combos: product,
            cap: MAX_COMBOS,
        })
    } else {
        pool_heuristic_lots.map(|lots| ApproxReason::PoolHeuristic {
            lots,
            bound: LOT_ENUM_BOUND,
        })
    };

    // Re-fold `best` to extract `marginal_rates` (needed for the proposal). We do NOT use this fold's
    // `.total_federal_tax_attributable` for `optimized_tax` or `delta` — see comment above the search.
    let opt_state = fold_with(events, prices, config, &best);
    let opt = match compute_tax_year(events, &opt_state, year, profile, tables) {
        TaxOutcome::Computed(r) => r,
        TaxOutcome::NotComputable(b) => return Err(OptimizeError::YearNotComputable(b)),
    };

    // Per-disposal proposal rows. R0-C2: status/persistability are judged by the PROPOSED pick's OWN
    // timeliness, NOT by `disposal_compliance(events, opt_state)` (which lacks the injected pick → a
    // divergent post-hoc cherry-pick would fall through to a compliant StandingOrder — FORBIDDEN, §0).
    // A's `disposal_compliance(events, &baseline_state)` supplies only the BASELINE status, used to
    // preserve a genuine StandingOrder/Contemporaneous when the proposal does NOT diverge from current.
    let base_comp = disposal_compliance(events, &baseline_state);
    let mut rows: Vec<DisposalCompliance> = Vec::new();
    let mut row_meta: Vec<RowMeta> = Vec::new();
    for (id, wallet, date, _need) in &targets {
        let current = baseline_selection(&baseline_state, id);
        let proposed = best.get(id).cloned().unwrap_or_else(|| current.clone());
        let baseline_status = base_comp
            .iter()
            .find(|c| &c.disposal == id)
            .map(|c| c.status.clone())
            .unwrap_or(ComplianceStatus::NonCompliant);
        let status = proposed_compliance_status(
            wallet,
            *date,
            proposal_made,
            &proposed,
            &current,
            &baseline_status,
        );
        rows.push(DisposalCompliance {
            disposal: id.clone(),
            wallet: wallet.clone(),
            date: *date,
            status,
        });
        row_meta.push((id.clone(), wallet.clone(), *date, current, proposed));
    }
    // Task-5 overlay: NonCompliant + attested + within envelope + proposed==current → AttestedRecording.
    let unchanged: BTreeSet<EventId> = row_meta
        .iter()
        .filter(|(_, _, _, current, proposed)| proposed == current)
        .map(|(id, ..)| id.clone())
        .collect();
    let rows = compliance_overlay(&rows, attested, &unchanged);

    let per_disposal: Vec<DisposalProposal> = row_meta
        .into_iter()
        .zip(rows)
        .map(
            |((id, wallet, date, current, proposed), row)| DisposalProposal {
                disposal: id,
                wallet: wallet.clone(),
                date,
                current_selection: current,
                proposed_selection: proposed,
                status: row.status,
                // R0-C2/N2: the REAL made-date governs persistability — only genuinely-contemporaneous
                // picks (made ≤ sale) are persistable; 2027+ broker NEVER.
                persistable: persistability(&wallet, date, proposal_made),
            },
        )
        .collect();

    Ok(OptimizeProposal {
        year,
        baseline_tax: base.total_federal_tax_attributable,
        // Use the search's tracked incumbent score, NOT the re-fold's total. The re-fold can shift a
        // pro-rata remainder cent between an ST and an LT leg (picks are in lot-id order rather than
        // the original FIFO order), causing `opt.total` to exceed `base.total` by ≤$0.01 and breaking
        // the "ALWAYS ≤ 0" struct-doc invariant on multi-leg disposals where best==baseline.
        // `best_total` is baseline-seeded and only evicted by a strict improvement, so it is ≤
        // `base.total_federal_tax_attributable` by construction (delta ≤ 0 holds exactly).
        optimized_tax: best_total,
        delta: best_total - base.total_federal_tax_attributable,
        per_disposal,
        marginal_rates: opt.marginal_rates, // re-fold still needed for this field
        approximate,
        approx_reason,
    })
}

/// The lots the CURRENT projection consumes for `disposal` (its baseline disposal legs), as picks, sorted
/// by lot id (canonical — matches `candidate_selections`'s ordering so lex tie-breaks are consistent).
fn baseline_selection(state: &LedgerState, disposal: &EventId) -> Vec<LotPick> {
    let mut picks: Vec<LotPick> = state
        .disposals
        .iter()
        .find(|d| &d.event == disposal)
        .map(|d| {
            d.legs
                .iter()
                .map(|l| LotPick {
                    lot: l.lot_id.clone(),
                    sat: l.sat,
                })
                .collect()
        })
        .unwrap_or_default();
    picks.sort_by(|a, b| a.lot.cmp(&b.lot));
    picks
}

/// Exhaustive cartesian-product minimisation over the per-GROUP candidate lists (odometer, no recursion).
/// Each combination merges one chosen partial-map per group into a single assignment, scores it via
/// `score_assignment`, and keeps the minimum `total_federal_tax_attributable`. Infeasible cross-disposal
/// combinations self-eliminate (`NotComputable` → skipped). R0-C1: the incumbent is SEEDED with
/// `baseline_assignment` at `base.total_federal_tax_attributable`, so the result can never be worse than
/// the baseline; ties break to the lexicographically-smallest assignment (NFR4 §0 total order).
#[allow(clippy::too_many_arguments)]
fn exhaustive_min(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    group_lists: &[Vec<BTreeMap<EventId, Vec<LotPick>>>],
    baseline_assignment: &BTreeMap<EventId, Vec<LotPick>>,
    base: &TaxResult,
) -> (BTreeMap<EventId, Vec<LotPick>>, Usd) {
    let mut best_total = base.total_federal_tax_attributable;
    let mut best_assign = baseline_assignment.clone();
    let lens: Vec<usize> = group_lists.iter().map(|g| g.len()).collect();
    if lens.contains(&0) {
        return (best_assign, best_total); // a group with no candidates → only the baseline is considered
    }
    let mut idx = vec![0usize; group_lists.len()];
    loop {
        let mut assign: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
        for (gi, &ci) in idx.iter().enumerate() {
            for (k, v) in &group_lists[gi][ci] {
                assign.insert(k.clone(), v.clone());
            }
        }
        if let TaxOutcome::Computed(r) =
            score_assignment(events, prices, config, year, profile, tables, &assign)
        {
            let total = r.total_federal_tax_attributable;
            if total < best_total || (total == best_total && assign < best_assign) {
                best_total = total;
                best_assign = assign;
            }
        }
        // odometer increment over the per-group index vector
        let mut k = 0;
        loop {
            if k == idx.len() {
                return (best_assign, best_total);
            }
            idx[k] += 1;
            if idx[k] < lens[k] {
                break;
            }
            idx[k] = 0;
            k += 1;
        }
    }
}

/// Deterministic, BASELINE-SEEDED coordinate descent for products beyond `MAX_COMBOS`. R0-C1: START from
/// `baseline_assignment` (NOT all-HIFO), so the incumbent is the current filing position and
/// `optimized_tax ≤ baseline_tax` holds even if every candidate basin is worse than baseline. Then, per
/// group in order (a singleton group = one disposal), hold the others fixed and pick its best candidate by
/// full-year score, accepting a move ONLY if it strictly lowers the total; iterate to a fixed point
/// (bounded passes). No float, no RNG, no clock (NFR4/NFR5).
#[allow(clippy::too_many_arguments)]
fn coordinate_descent(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    group_lists: &[Vec<BTreeMap<EventId, Vec<LotPick>>>],
    baseline_assignment: &BTreeMap<EventId, Vec<LotPick>>,
    base: &TaxResult,
) -> (BTreeMap<EventId, Vec<LotPick>>, Usd) {
    let mut current = baseline_assignment.clone();
    let mut current_total = base.total_federal_tax_attributable;
    let n_groups = group_lists.len();
    let pass_cap = n_groups + 1; // bounded passes (deterministic termination)
    let mut passes = 0;
    let mut changed = true;
    while changed && passes < pass_cap {
        changed = false;
        passes += 1;
        for group in group_lists.iter() {
            let mut best: Option<(Usd, BTreeMap<EventId, Vec<LotPick>>)> = None;
            for cand in group {
                let mut assign = current.clone();
                for (k, v) in cand {
                    assign.insert(k.clone(), v.clone());
                }
                if let TaxOutcome::Computed(r) =
                    score_assignment(events, prices, config, year, profile, tables, &assign)
                {
                    let total = r.total_federal_tax_attributable;
                    match &best {
                        None => best = Some((total, assign)),
                        Some((bt, ba)) => {
                            if total < *bt || (total == *bt && &assign < ba) {
                                best = Some((total, assign));
                            }
                        }
                    }
                }
            }
            if let Some((bt, ba)) = best {
                if bt < current_total {
                    // strict improvement only ⇒ optimized ≤ baseline (delta ≤ 0); deterministic
                    current = ba;
                    current_total = bt;
                    changed = true;
                }
            }
        }
    }
    (current, current_total)
}

// ── Task 6 — Mode-2 pre-trade consult `consult_sale` (synthetic disposal + ST→LT timing) ────────────

/// §C.3 READ-ONLY pre-trade consultation. For a HYPOTHETICAL sale (sell `req.sell_sat` from
/// `req.wallet` at `req.at`, with `req.proceeds` or dataset FMV) pick the tax-minimizing lot selection,
/// report the resulting ST/LT split + the year's federal tax, and the ST→LT crossover timing insight.
///
/// **Side-effect-free (Mode-2 produces NOTHING — §0).** `events`/`prices`/`config` are borrowed
/// read-only; every fold is clone-fold-discard (`resolve` → mutate an owned `Resolution` → `fold` →
/// read → drop). The function appends NO event, writes NO side-table, makes NO decision — it returns a
/// `ConsultReport` only. It is tax decision-support (consequences), NOT buy/sell advice.
///
/// **Scope.** Optimizes ONLY the synthetic disposal's selection (existing disposals keep their current
/// identification — a single-disposal what-if, not a year-wide re-optimization). Deterministic (NFR4),
/// exact (NFR5 — every dollar comes straight from B; C never re-rounds).
///
/// **Profile threading.** The CLI loads the year's `TaxProfile` and passes it in (`year_profile`) so
/// core stays clock-free; a missing profile → the underlying `compute_tax_year` returns
/// `TaxProfileMissing` → `OptimizeError::YearNotComputable`.
///
/// **Refusals.** Pre-2025 `at` → `PreTransitionYear` (M7); an empty as-of pool / insufficient holdings →
/// `NoLots`; a future date with no dataset price AND no `proceeds` → `Evaluate(ProceedsRequired)`; a
/// `NotComputable` year → `YearNotComputable` (I6).
pub fn consult_sale(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year_profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    req: &ConsultRequest,
) -> Result<ConsultReport, OptimizeError> {
    let year = req.at.year();
    if year < TRANSITION_DATE.year() {
        return Err(OptimizeError::PreTransitionYear(year));
    }

    // R0-M3: available lots = the wallet pool AS OF `at` (fold the canonical timeline truncated to
    // `date() <= at`, seed at the boundary). Correct for an interleaved/past `at`, not only the
    // forward-looking case. Per-wallet (§1.1012-1(j)); `remaining_sat > 0`; sorted by lot_id (NFR4).
    let pre = fold_as_of(events, prices, config, req.at);
    let want = pool_key(req.at, &req.wallet);
    let mut lots: Vec<Lot> = pre
        .lots
        .into_iter()
        .filter(|l| {
            l.remaining_sat > 0 && pool_key(req.at, &l.wallet) == want && l.acquired_at <= req.at
        })
        .collect();
    lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
    if lots.iter().map(|l| l.remaining_sat).sum::<Sat>() < req.sell_sat {
        return Err(OptimizeError::NoLots);
    }

    let candidate = CandidateDisposal {
        existing_event: None, // synthetic (Mode-2)
        wallet: req.wallet.clone(),
        date: req.at,
        sat: req.sell_sat,
        kind: req.kind,
        proceeds: req.proceeds, // None on a future date with no dataset price → ProceedsRequired
    };
    // Resolve proceeds once up front so a missing future price fails fast with ProceedsRequired
    // (mirrors A's `evaluate_disposal` proceeds resolution: explicit > dataset FMV > error).
    if req.proceeds.is_none() && fmv_of(prices, req.at, req.sell_sat).is_none() {
        return Err(OptimizeError::Evaluate(EvaluateError::ProceedsRequired));
    }

    // Enumerate candidate selections for the synthetic disposal and score each via the synthetic
    // evaluate+compute path; pick the deterministic minimum federal tax. R2-C1: Mode-2 reports a what-if
    // tax-min selection, NOT a "proven global minimum" claim (`ConsultReport` has no `approximate` field
    // and the renderer never says "the optimum"), so the heuristic-branch flag is not surfaced here — it
    // governs `OptimizeProposal` (Mode-1), which is what R2-C1 scopes. Every candidate is drawn from the
    // as-of pool with sufficient remaining, so all are feasible (the `?` below never trips on a generated
    // candidate).
    let (cands, _heuristic) = candidate_selections(&lots, req.sell_sat);
    let mut best: Option<(Usd, Vec<LotPick>, Usd, Usd)> = None; // (total, picks, st, lt)
    for picks in &cands {
        let (st, lt, total) = score_synthetic(
            events,
            prices,
            config,
            year_profile,
            tables,
            &candidate,
            picks,
        )?;
        let cand = (total, picks.clone(), st, lt);
        best = Some(match best {
            None => cand,
            Some(b) if (cand.0, &cand.1) < (b.0, &b.1) => cand, // min tax, tie → smallest picks
            Some(b) => b,
        });
    }
    let (total, proposed_selection, st_gain, lt_gain) = best.ok_or(OptimizeError::NoLots)?;

    // ST→LT timing insight (R0-I4/M4): OMITTED (None) — never `Err` — when no leg is short-term, when a
    // contributing lot's crossover hits the `next_day` max-date edge, or when the crossover lands outside
    // `at`'s bundled year/profile. An unbundled crossover year degrades gracefully (the consult still
    // returns the what-if) instead of failing on a missing future table.
    let timing = timing_insight(
        events,
        prices,
        config,
        year_profile,
        tables,
        &candidate,
        &proposed_selection,
        &lots,
        total,
    );

    Ok(ConsultReport {
        req: req.clone(),
        proposed_selection,
        st_gain,
        lt_gain,
        total_federal_tax_attributable: total,
        timing,
    })
}

/// As-of-`at` pool: clone-fold the canonical timeline truncated to events with `date() <= at` (R0-M3),
/// delegating to `fold::state_as_of` (which reuses `fold`'s `sort_canonical` + transition partition and
/// fires the boundary seed at the correct point). Sibling of `available_lots_before` (which truncates
/// before a specific disposal id rather than at a date). Read-only: `resolve` yields an owned
/// `Resolution`, the resulting `LedgerState` is the caller's to read then discard.
fn fold_as_of(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    at: TaxDate,
) -> LedgerState {
    let res = resolve(events, prices, config);
    state_as_of(res, prices, config, at)
}

/// Clone-fold of the canonical timeline with a synthetic `Op::Dispose` appended (mirroring
/// `evaluate.rs`'s synthetic-append) + the candidate selection injected, WITHOUT mutating the ledger.
/// Returns the resulting `LedgerState` (read then discarded by the caller). The synthetic event uses
/// the reserved sentinel id `EventId::Decision { seq: u64::MAX }` (unreachable for real sequences,
/// never persisted — no I/O on this path). Proceeds resolve explicit > dataset FMV > `ProceedsRequired`,
/// identically to `evaluate_disposal`, so the parallel fold's legs match A's split.
fn synthetic_state(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    candidate: &CandidateDisposal,
    picks: &[LotPick],
) -> Result<LedgerState, EvaluateError> {
    let mut res = resolve(events, prices, config);
    let proceeds = match candidate.proceeds {
        Some(p) => p,
        None => {
            fmv_of(prices, candidate.date, candidate.sat).ok_or(EvaluateError::ProceedsRequired)?
        }
    };
    let id = EventId::Decision { seq: u64::MAX };
    // midnight().assume_utc() → UTC 00:00:00 on `candidate.date`; tax_date(utc, UTC) == candidate.date.
    let utc = candidate.date.midnight().assume_utc();
    res.timeline.push(Eff {
        id: id.clone(),
        utc,
        tz: time::UtcOffset::UTC,
        src_priority: 0,
        src_ref: SourceRef::new("__synthetic__"),
        wallet: Some(candidate.wallet.clone()),
        op: Op::Dispose {
            sat: candidate.sat,
            proceeds,
            fee_usd: Usd::ZERO,
            fee_sat: None,
            kind: candidate.kind,
        },
    });
    res.selections.insert(id, picks.to_vec());
    Ok(fold(res, prices, config))
}

/// Score one synthetic-disposal selection: A's `evaluate_disposal` gives the per-leg ST/LT split, and a
/// parallel synthetic fold + `compute_tax_year` gives the YEAR's federal tax (the holistic objective —
/// cross-netting with any other in-year crypto is captured, matching Mode-1). Both are clone-fold-discard
/// (no mutation). Returns `(st_gain, lt_gain, total_federal_tax_attributable)`. An infeasible selection
/// → the fold raises `LotSelectionInvalid` → `compute_tax_year` `NotComputable` → `YearNotComputable`
/// (generated candidates are always feasible, so this only guards a hand-built call).
#[allow(clippy::too_many_arguments)]
fn score_synthetic(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year_profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    candidate: &CandidateDisposal,
    picks: &[LotPick],
) -> Result<(Usd, Usd, Usd), OptimizeError> {
    // 1) ST/LT split for THIS disposal via A's side-effect-free entrypoint.
    let out = evaluate_disposal(events, prices, config, candidate, Some(picks))
        .map_err(OptimizeError::Evaluate)?;
    // 2) Full-year federal tax via a parallel synthetic fold (same append + injected selection).
    let state = synthetic_state(events, prices, config, candidate, picks)
        .map_err(OptimizeError::Evaluate)?;
    let year = candidate.date.year();
    match compute_tax_year(events, &state, year, year_profile, tables) {
        TaxOutcome::Computed(r) => Ok((out.st_gain, out.lt_gain, r.total_federal_tax_attributable)),
        TaxOutcome::NotComputable(b) => Err(OptimizeError::YearNotComputable(b)),
    }
}

/// ST→LT crossover timing insight — returns `Option<TimingInsight>` (OMIT, never error — R0-I4/M4).
///
/// For each pick in the chosen selection, find its source lot; the lot is short-term as of `at` iff
/// `!is_long_term(lot.gain_hp_start(), at)`. If NONE are short-term → `None`. Otherwise
/// `st_sat_in_selection` = Σ their sats and `latest_crossover` = max over them of the first STRICTLY
/// long-term date = `one_year_after(gain_hp_start).next_day()`. **R0-M4:** `Date::next_day()` is `Option`
/// (Dec-31 / max-date edge) — `None` for any contributing lot ⇒ OMIT (no unwrap).
///
/// **R0-I4 (same year/profile, term-flip, degrade).** `tax_if_sold_long_term` is computed WITHIN THE
/// SAME tax year and profile as `at`: re-score the SAME selection with the SAME proceeds, realized as a
/// synthetic disposal dated `latest_crossover` — so the short-term legs flip to long-term while the price
/// is unchanged (lots already LT as of `at` stay LT). Done ONLY when `latest_crossover.year() ==
/// at.year()` AND `at`'s table + profile are present; OTHERWISE `None` — NEVER re-score in a future
/// crossover year (a 2026+ re-score would hit a missing bundled table → `NotComputable` and fail the
/// whole consult). `saving_if_waited = (total_now − tax_if_sold_long_term).max(0)`.
#[allow(clippy::too_many_arguments)]
fn timing_insight(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year_profile: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    candidate: &CandidateDisposal,
    proposed_selection: &[LotPick],
    lots: &[Lot],
    total_now: Usd,
) -> Option<TimingInsight> {
    let at = candidate.date;
    let mut st_sat: Sat = 0;
    let mut crossover: Option<TaxDate> = None;
    for p in proposed_selection {
        let lot = lots.iter().find(|l| l.lot_id == p.lot)?;
        if !is_long_term(lot.gain_hp_start(), at) {
            st_sat += p.sat;
            // First strictly-long-term date = anniversary + 1 day; R0-M4: None (max-date edge) ⇒ omit.
            let lt_date = one_year_after(lot.gain_hp_start()).next_day()?;
            crossover = Some(crossover.map_or(lt_date, |c: TaxDate| c.max(lt_date)));
        }
    }
    let latest_crossover = crossover?; // no short-term leg ⇒ no insight

    // R0-I4: stay within `at`'s bundled year/profile; degrade (omit) otherwise — never re-score forward.
    if latest_crossover.year() != at.year()
        || tables.table_for(at.year()).is_none()
        || year_profile.is_none()
    {
        return None;
    }

    // tax_if_sold_long_term: the SAME selection + SAME proceeds, dated `latest_crossover` (ST legs now
    // LT; price unchanged). Resolve proceeds the same way as the headline score (explicit > FMV).
    let proceeds = candidate
        .proceeds
        .or_else(|| fmv_of(prices, at, candidate.sat))?;
    let lt_candidate = CandidateDisposal {
        existing_event: None,
        wallet: candidate.wallet.clone(),
        date: latest_crossover,
        sat: candidate.sat,
        kind: candidate.kind,
        proceeds: Some(proceeds),
    };
    let (_st, _lt, tax_if_sold_long_term) = score_synthetic(
        events,
        prices,
        config,
        year_profile,
        tables,
        &lt_candidate,
        proposed_selection,
    )
    .ok()?; // any unexpected NotComputable ⇒ omit rather than fail the consult

    let saving_if_waited = (total_now - tax_if_sold_long_term).max(Usd::ZERO);
    Some(TimingInsight {
        st_sat_in_selection: st_sat,
        latest_crossover,
        tax_if_sold_long_term,
        saving_if_waited,
    })
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

    // ── contention grouping + joint enumeration (R0-C3) ────────────────────────────────────────────

    /// Two same-wallet sells drawing from overlapping lots → ONE contention group, and
    /// `group_candidate_assignments` yields the cross-period "deviation" sequence (D1 takes the lot the
    /// baseline gives D2, freeing the other for D2) that the INDEPENDENT per-disposal product cannot reach.
    #[test]
    fn contention_groups_one_group_and_joint_reaches_deviation() {
        // R acquired earlier (LT-able), P acquired later; two 2026 sells of one lot each, FIFO baseline.
        let events = vec![
            buy(
                "R",
                datetime!(2025-05-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(5000),
            ),
            buy(
                "P",
                datetime!(2025-06-15 00:00:00 UTC),
                cold(),
                LOT,
                dec!(5000),
            ),
            sell(
                "D1",
                datetime!(2026-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(10000),
            ),
            sell(
                "D2",
                datetime!(2026-06-20 00:00:00 UTC),
                cold(),
                LOT,
                dec!(10000),
            ),
        ];
        let prices = StaticPrices::default();
        let mut targets = vec![
            (eid("D1"), cold(), date!(2026 - 06 - 01), LOT),
            (eid("D2"), cold(), date!(2026 - 06 - 20), LOT),
        ];
        targets.sort_by(|a, b| a.0.cmp(&b.0));

        let groups = contention_groups(&events, &prices, &cfg(), &targets);
        assert_eq!(
            groups.len(),
            1,
            "overlapping same-wallet disposals → one group"
        );
        assert_eq!(groups[0].len(), 2);

        let members: Vec<_> = groups[0].iter().map(|&i| targets[i].clone()).collect();
        let (maps, heur) =
            group_candidate_assignments(&events, &prices, &cfg(), &members).expect("within bound");
        assert_eq!(heur, None, "small pools → no heuristic branch");
        // Joint set = {(D1=R,D2=P), (D1=P,D2=R)} — the second is the deviation unreachable independently.
        let rp: BTreeMap<EventId, Vec<LotPick>> = [
            (eid("D1"), vec![pick("R", LOT)]),
            (eid("D2"), vec![pick("P", LOT)]),
        ]
        .into_iter()
        .collect();
        let pr: BTreeMap<EventId, Vec<LotPick>> = [
            (eid("D1"), vec![pick("P", LOT)]),
            (eid("D2"), vec![pick("R", LOT)]),
        ]
        .into_iter()
        .collect();
        assert!(maps.contains(&rp), "baseline-consistent sequence present");
        assert!(
            maps.contains(&pr),
            "cross-period deviation sequence present (joint-only)"
        );
        assert_eq!(maps.len(), 2);
    }

    /// Two disposals on DIFFERENT wallets → two singleton groups (disjoint pools, never contended).
    #[test]
    fn contention_groups_singletons_for_different_wallets() {
        let events = vec![
            buy(
                "CL",
                datetime!(2026-05-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(5000),
            ),
            buy(
                "HL",
                datetime!(2026-05-01 00:00:00 UTC),
                hot(),
                LOT,
                dec!(5000),
            ),
            sell(
                "DC",
                datetime!(2026-06-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(10000),
            ),
            sell(
                "DH",
                datetime!(2026-06-01 00:00:00 UTC),
                hot(),
                LOT,
                dec!(10000),
            ),
        ];
        let prices = StaticPrices::default();
        let mut targets = vec![
            (eid("DC"), cold(), date!(2026 - 06 - 01), LOT),
            (eid("DH"), hot(), date!(2026 - 06 - 01), LOT),
        ];
        targets.sort_by(|a, b| a.0.cmp(&b.0));
        let groups = contention_groups(&events, &prices, &cfg(), &targets);
        assert_eq!(groups.len(), 2, "different wallets → two singleton groups");
        assert!(groups.iter().all(|g| g.len() == 1));
    }

    /// A contended group whose joint enumeration would exceed `GROUP_COMBO_BOUND` → `None` (caller then
    /// flags `ContentionUnenumerated`). Four same-wallet 1-lot sells over a 10-lot pool: 10·9·8·7 = 5040.
    #[test]
    fn group_candidate_assignments_none_beyond_bound() {
        let mut events: Vec<LedgerEvent> = (0..10)
            .map(|i| {
                buy(
                    &format!("L{i:02}"),
                    datetime!(2026-05-01 00:00:00 UTC),
                    cold(),
                    LOT,
                    dec!(5000),
                )
            })
            .collect();
        let dates = [
            date!(2026 - 06 - 01),
            date!(2026 - 06 - 02),
            date!(2026 - 06 - 03),
            date!(2026 - 06 - 04),
        ];
        for (k, d) in dates.iter().enumerate() {
            events.push(sell(
                &format!("D{k}"),
                datetime!(2026-06-01 00:00:00 UTC).replace_date(*d),
                cold(),
                LOT,
                dec!(10000),
            ));
        }
        let prices = StaticPrices::default();
        let members: Vec<_> = (0..4)
            .map(|k| (eid(&format!("D{k}")), cold(), dates[k], LOT))
            .collect();
        let res = group_candidate_assignments(&events, &prices, &cfg(), &members);
        assert!(res.is_none(), "joint count 5040 > GROUP_COMBO_BOUND ⇒ None");
    }
}
