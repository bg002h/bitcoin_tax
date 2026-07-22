use crate::conservative_promote::{PromoteEntry, PromoteSet};
use crate::conventions::{tax_date, Sat, TaxDate, Usd, TRANSITION_DATE, TY2025_RETURN_DUE};
use crate::event::*;
use crate::identity::{EventId, LotId, SourceRef, WalletId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::{FeeTreatment, LotMethod, ProjectionConfig};
use crate::state::{Blocker, BlockerKind, Lot};
use std::collections::{BTreeMap, BTreeSet};
use time::{OffsetDateTime, UtcOffset};

/// Shared guard: returns `true` when a `MethodElection` is a valid forward standing order —
/// non-backdated and on/after `TRANSITION_DATE` (§1.1012-1(j): no post-hoc identification, ever).
///
/// Used by both `resolve.rs` (to decide whether to emit a `MethodElectionBackdated` blocker) and
/// `compliance.rs` (to filter eligible elections for the §A.5(a) standing-order check). Keeping
/// the condition in one place ensures both callers stay in sync with the shared spec rule.
pub(crate) fn method_election_is_forward(me: &MethodElection, made: TaxDate) -> bool {
    me.effective_from >= TRANSITION_DATE && me.effective_from >= made
}

/// What an imported event behaves as in PASS 2, after decisions are applied. Variants are ADDED across tasks
/// (Task 7: decisions, Task 8: transfers, Task 9: gift/donation, Task 10: dual-basis, Task 11: fee, Task 12: seed).
#[derive(Debug, Clone)]
pub enum Op {
    Acquire(Acquire),
    Dispose {
        sat: Sat,
        proceeds: Usd,
        fee_usd: Usd,
        /// Task 11: on-chain fee-sats consumed per TP8. Set from `TransferOut.fee_sat` when a
        /// `ReclassifyOutflow{Dispose}` is applied; `None` for a native `EventPayload::Dispose`
        /// (exchange disposals carry no on-chain fee-sat field — their fee is already in `fee_usd`).
        fee_sat: Option<Sat>,
        kind: DisposeKind,
    },
    Income {
        sat: Sat,
        fmv: Option<Usd>,
        kind: IncomeKind,
        business: bool,
    },
    // Task 8:
    /// Unmatched TransferOut: sats leave holdings into pending_reconciliation; advisory UnmatchedOutflows blocker.
    PendingOut {
        sat: Sat,
        fee_sat: Option<Sat>,
    },
    /// Confirmed self-transfer (TransferLink): principal relocates from source pool to dest pool; non-taxable.
    SelfTransfer {
        sat: Sat,
        fee_sat: Option<Sat>,
        dest: WalletId,
    },
    /// Unclassified TransferIn: hard UnknownBasisInbound blocker; no lot (sats not yet in the ledger, FR9/§7.3).
    UnknownInbound {
        sat: Sat,
    },
    /// ClassifyInbound::Income applied to a TransferIn: income lot at FMV + IncomeRecord.
    IncomeInbound {
        sat: Sat,
        fmv: Option<Usd>,
        kind: IncomeKind,
        business: bool,
    },
    /// ClassifyInbound::GiftReceived applied to a TransferIn: gift lot (dual-basis in Task 10).
    GiftReceived {
        sat: Sat,
        donor_basis: Option<Usd>,
        donor_acquired_at: Option<TaxDate>,
        fmv_at_gift: Usd,
    },
    /// Cycle A: ClassifyInbound::SelfTransferMine on a TransferIn — CREATES a new NON-taxable origin lot
    /// (basis default $0, acquired_at default = receipt date). NOT a relocation (no source lot exists),
    /// so — unlike `SelfTransfer` — it is neither a disposition nor method-honoring (outside FIFO).
    SelfTransferInbound {
        sat: Sat,
        basis: Option<Usd>,
        acquired_at: Option<TaxDate>,
    },
    // Task 9: gift/donation outbound (TP10) and reclassified disposal.
    /// ReclassifyOutflow{GiftOut}: Removal with zero recognized gain; per-lot basis + FMV + ST/LT.
    GiftOut {
        sat: Sat,
        fmv: Usd,
        fee_sat: Option<Sat>,  // Task 11: on-chain fee consumed per TP8 (c)
        fee_usd: Option<Usd>,  // Task 11: USD fee from ReclassifyOutflow
        donee: Option<String>, // Chunk 2: free-form donee label (None for legacy records)
    },
    /// ReclassifyOutflow{Donate}: Removal with zero recognized gain + appraisal_required flag.
    Donate {
        sat: Sat,
        fmv: Usd,
        appraisal_required: bool,
        fee_sat: Option<Sat>,  // Task 11: on-chain fee consumed per TP8 (c)
        fee_usd: Option<Usd>,  // Task 11: USD fee from ReclassifyOutflow
        donee: Option<String>, // Chunk 2: free-form donee label (None for legacy records)
    },
    // (Task 12) seeded — added as those tasks land.
    Unclassified,
    Skip, // e.g. a TransferIn consumed by a TransferLink; folds to nothing
}

#[derive(Debug, Clone)]
pub struct Eff {
    pub id: EventId,
    pub utc: OffsetDateTime,
    pub tz: UtcOffset,
    pub src_priority: u8,
    pub src_ref: SourceRef,
    pub wallet: Option<crate::identity::WalletId>,
    pub op: Op,
    /// Pseudo-reconcile taint (sub-project 2, [R0-I1/C1]): `true` when this event's effective `Op` is
    /// governed by a synthetic (non-persisted) default injected in pseudo mode — the seam that carries
    /// pseudo-ness from the map/`Eff` layer into every `Lot`/leg the fold builds from it. Always `false`
    /// outside pseudo mode (⇒ projection byte-identical).
    pub pseudo: bool,
}
impl Eff {
    pub fn date(&self) -> TaxDate {
        tax_date(self.utc, self.tz)
    }
}

#[derive(Debug, Clone)]
pub enum TransitionMode {
    /// Default: pass 2 reconstructs per-wallet pools from the Universal remainder at 2025-01-01.
    PathA,
    /// An effective `SafeHarborAllocation` governs: pass 2 discards the Universal remainder and seeds
    /// these pre-built per-wallet lots (`LotId = (allocation EventId, index)`, `basis_source =
    /// SafeHarborAllocated`). Built by `resolve` in Task 12; empty/`PathA` until then. (N4: no `(())` placeholder.)
    PathB { seed: Vec<crate::state::Lot> },
}

/// A `VoidDecisionEvent` whose target is a `SafeHarborAllocation` — collected in pass-1 step 1a and
/// adjudicated by §7.4 effectiveness (step 3): a void of an EFFECTIVE allocation → `DecisionConflict`
/// (irrevocable, it stays in force); a void of an inert allocation simply applies (no conflict, Path A).
/// `void_id`: the `VoidDecisionEvent`'s `EventId`; `target`: the `SafeHarborAllocation`'s `EventId`.
#[derive(Debug, Clone)]
pub struct AllocationVoid {
    pub void_id: EventId,
    pub target: EventId,
}

/// BG-D9 (Task 7): a `VoidDecisionEvent` whose target is a `DeclareTranche` that HAS at least one
/// `PromoteTranche` referencing it. Collected in pass-1a (never classified "live" inline — that would be
/// order-dependent, arch r2 M-1) and adjudicated AFTER `live_promotes` builds the FINAL non-voided-promote
/// set, but BEFORE step 2's admit branch reads `voided` (the load-bearing insertion point, arch r1 I-2):
/// the target still carries a LIVE promote → the void is inert + `DecisionConflict` (never a dangling
/// target); else the void APPLIES (`voided.insert(target)`) so the step-2 admit branch drops the tranche.
/// Mirrors `AllocationVoid`'s deferred step-3 pattern, at the pre-step-2 insertion point.
#[derive(Debug, Clone)]
struct TrancheVoid {
    void_id: EventId,
    target: EventId,
}

/// An in-force forward method election (§A.5(a)). Collected in `resolve` from non-voided, non-backdated
/// `MethodElection` decisions; the latest-in-force by `(effective_from, decision_seq)` governs per-wallet
/// disposals on/after `effective_from` (NFR4: a total order, no `Date::now`/RNG).
///
/// `wallet` carries the election's scope through resolve→fold/compliance: `None` = GLOBAL,
/// `Some(w)` = scoped to exchange account `w`. The shared `resolve_election` resolver applies the
/// two-independent-tiers precedence (scoped-then-global).
#[derive(Debug, Clone)]
pub struct ElectionRec {
    pub effective_from: TaxDate,
    pub method: crate::LotMethod,
    pub decision_seq: u64,
    pub wallet: Option<WalletId>,
}

/// The SHARED wallet-aware election resolver (§A.5(a)) — the SOLE method-resolution path, used by BOTH
/// `fold::applicable_method` (the fold) AND `disposal_compliance` (compliance). Returns the winning
/// `ElectionRec` (or `None` ⇒ HIFO in the fold / no `StandingOrder` in compliance) for a disposal on
/// `wallet` at `date`, via **TWO INDEPENDENT TIERS** [R0-M2]:
///
///   tier 1 — the latest election SCOPED to `wallet` (`wallet == Some(w)`) with `effective_from ≤ date`,
///            ordered by `(effective_from, decision_seq)`;
///   tier 2 — ONLY if tier 1 is empty: the latest GLOBAL election (`wallet == None`) with
///            `effective_from ≤ date`, same order;
///   else   — `None`.
///
/// The tiers are resolved INDEPENDENTLY: a later-dated GLOBAL election NEVER overrides an in-force
/// SCOPED one for its wallet (tier 1 is decided purely among `wallet == Some(w)` records). Tier 1
/// respects `effective_from ≤ date` [R0-r2-M1]: a not-yet-effective scoped election does NOT suppress an
/// in-force global one — it simply fails the tier-1 filter, so tier 2 (global) is consulted, NOT FIFO.
/// This is deliberately NOT a single `max_by` over all elections merged — that would let a fresh global
/// election silently flip a wallet the user scoped (fault-injected by `later_global_does_not_override_*`).
pub(crate) fn resolve_election<'a>(
    date: TaxDate,
    wallet: &WalletId,
    elections: &'a [ElectionRec],
) -> Option<&'a ElectionRec> {
    let latest = |scoped: bool| -> Option<&'a ElectionRec> {
        elections
            .iter()
            .filter(|e| e.effective_from <= date)
            .filter(|e| {
                if scoped {
                    e.wallet.as_ref() == Some(wallet)
                } else {
                    e.wallet.is_none()
                }
            })
            .max_by(|a, b| {
                a.effective_from
                    .cmp(&b.effective_from)
                    .then(a.decision_seq.cmp(&b.decision_seq))
            })
    };
    // tier 1 (scoped) is resolved FIRST and INDEPENDENTLY; only an EMPTY tier 1 falls to tier 2 (global).
    latest(true).or_else(|| latest(false))
}

pub struct Resolution {
    pub timeline: Vec<Eff>,
    pub transition: TransitionMode,
    pub blockers: Vec<Blocker>,
    /// In-force forward method elections (§A.5(a)); empty ⇒ HIFO default for all per-wallet disposals (fold::applicable_method fall-through).
    pub elections: Vec<ElectionRec>,
    /// Per-disposal named-lot selections (§A.4). Empty this task; populated in Task 4.
    pub selections: BTreeMap<EventId, Vec<crate::event::LotPick>>,
    /// Pseudo-reconcile synthetic defaults injected this projection (sub-project 2). EMPTY unless
    /// `config.pseudo_reconcile` is on. Each entry is a materializable REAL decision — the SAME payload
    /// `reconcile pseudo approve` persists — so "what you see == what you approve". Never written to the
    /// ledger by projection; only surfaced (count/advisory/`[PSEUDO]`) and consumed by `approve`.
    pub pseudo_decisions: Vec<PseudoDefault>,
    /// Approach-B / BG-D1 (Task 3): the promotions in force this projection — target `DeclareTranche`
    /// `EventId` -> the `PromoteEntry` (stored `filed_basis` + `tranche_sat`) that rewrote its
    /// `Op::Acquire.usd_cost` at step 2 (below). Built by `live_promotes`; empty unless a `PromoteTranche`
    /// decision is in force. Threaded onto `Resolution` (rather than consumed and discarded here) so the
    /// fold (Task 4) can reach the same decomposition key for its own leg-builder needs — the ★ shared
    /// type has one owner (`conservative_promote.rs`, arch r1 I-1).
    pub promotes: PromoteSet,
}

/// The TYPE of a pseudo-reconcile synthetic default (sub-project 2) — drives the `approve` filter and the
/// per-default KATs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoKind {
    /// Unknown-basis inbound `TransferIn` → `ClassifyInbound(SelfTransferMine{$0})` (conservative $0,
    /// non-taxable; never income — assumption 3).
    SelfTransferInbound,
    /// `Unclassified` row (determinable-inbound, has a wallet) → `ClassifyRaw` to a zero-value placeholder
    /// (the row carries no structured amount, so pseudo books nothing until the user classifies it for real).
    RawInbound,
    /// Unresolved `ImportConflict` → accept-first `SupersedeImport` of the first-seen conflict.
    AcceptConflict,
    /// #41 Part B: an unresolved NATIVE `Income` with no effective FMV, on a date the local price data
    /// covers → synthetic `ManualFmv` at the daily close (`fmv_of(prices, date, sat)`). NO price ⇒ NO
    /// synthetic (the row stays Hard `FmvMissing` — the residual the online updater, Part C, addresses).
    PseudoFmv,
}

/// One pseudo-reconcile synthetic default (sub-project 2). `target` is the IMPORTED event whose effective
/// `Op` the synthetic governs (the pseudo-taint carrier + the display/filter anchor); `decision` is the
/// REAL decision payload `approve` persists to make it permanent (attested). For `AcceptConflict`, `target`
/// is the ImportConflict event while the decision's `SupersedeImport.conflict_event` points at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoDefault {
    pub target: EventId,
    pub decision: EventPayload,
    pub kind: PseudoKind,
}

/// Private outcome of resolving an `ImportConflict` via a decision. `Accept` boxes its payload:
/// `EventPayload` grew past clippy's `large_enum_variant` threshold once Approach-B's `PromoteTranche`
/// decision (task-1-brief.md) joined the sum type — the same reason `ImportConflict.new_payload` and
/// `ClassifyRaw.as_` already box an inner `EventPayload`.
enum Resolved {
    /// The new payload is accepted; inner `EventId` is the original import target.
    Accept(Box<EventPayload>, EventId),
    /// The conflict is rejected; the original import stands unchanged.
    Reject,
}

/// Map an effective imported payload → `Op`, applying any `ManualFmv` override and Task 8/9 classification maps.
/// ManualFmv on an `Income` replaces the FMV and clears the would-be `fmv_missing` gate.
/// `income_reclassify` (SE Chunk C): a `ReclassifyIncome` override flips `business` and optionally `kind`
/// on an `Income` event — the only projected-state change in this pass (no fold change needed).
#[allow(clippy::too_many_arguments)]
fn build_op(
    id: &EventId,
    payload: &EventPayload,
    manual_fmv: &BTreeMap<EventId, Usd>,
    links: &BTreeMap<EventId, TransferTarget>,
    consumed_ins: &BTreeSet<EventId>,
    inbound_class: &BTreeMap<EventId, InboundClass>,
    outflow_class: &BTreeMap<EventId, ReclassifyOutflow>,
    income_reclassify: &BTreeMap<EventId, ReclassifyIncome>,
    passthrough_skip: &BTreeSet<EventId>,
    by_id: &BTreeMap<EventId, &LedgerEvent>,
) -> Op {
    // Cycle B (G-PRECEDENCE): a confirmed-passthrough leg is skipped BEFORE any TransferLink /
    // ReclassifyOutflow / ClassifyInbound / PendingOut branch below — so a skipped TransferOut never
    // ALSO lands in `pending_reconciliation`. Safe because the [R0-I1] guard guarantees a leg in
    // `passthrough_skip` carries no competing (taxable) classification.
    if passthrough_skip.contains(id) {
        return Op::Skip;
    }
    match payload {
        EventPayload::Acquire(a) => Op::Acquire(a.clone()),
        EventPayload::Dispose(d) => Op::Dispose {
            sat: d.sat,
            proceeds: d.usd_proceeds,
            fee_usd: d.fee_usd,
            fee_sat: None, // native exchange disposal: no on-chain fee_sat (fee already in fee_usd)
            kind: d.kind,
        },
        EventPayload::Income(x) => {
            let fmv_override = manual_fmv.get(id).copied();
            // ManualFmv wins; otherwise use the event's own FMV (if not Missing).
            let fmv =
                fmv_override.or_else(|| x.usd_fmv.filter(|_| x.fmv_status != FmvStatus::Missing));
            // SE Chunk C: apply ReclassifyIncome override if present (validated at collection time).
            let (business, kind) = if let Some(o) = income_reclassify.get(id) {
                (o.business, o.kind.unwrap_or(x.kind))
            } else {
                (x.business, x.kind)
            };
            Op::Income {
                sat: x.sat,
                fmv,
                kind,
                business,
            }
        }
        EventPayload::TransferOut(t) => {
            if let Some(target) = links.get(id) {
                // Confirmed self-transfer via TransferLink.
                let dest = match target {
                    TransferTarget::InEvent(in_id) => {
                        by_id.get(in_id).and_then(|e| e.wallet.clone())
                    }
                    TransferTarget::Wallet(w) => Some(w.clone()),
                };
                if let Some(dest_wallet) = dest {
                    return Op::SelfTransfer {
                        sat: t.sat,
                        fee_sat: t.fee_sat,
                        dest: dest_wallet,
                    };
                }
                // Link target has no resolvable wallet — fall through.
            }
            // Task 9: elif in outflow_class → GiftOut/Donate/Dispose
            if let Some(ro) = outflow_class.get(id) {
                return match &ro.as_ {
                    OutflowClass::GiftOut => Op::GiftOut {
                        sat: t.sat,
                        fmv: ro.principal_proceeds_or_fmv,
                        fee_sat: t.fee_sat,
                        fee_usd: ro.fee_usd,
                        donee: ro.donee.clone(),
                    },
                    OutflowClass::Donate { appraisal_required } => Op::Donate {
                        sat: t.sat,
                        fmv: ro.principal_proceeds_or_fmv,
                        appraisal_required: *appraisal_required,
                        fee_sat: t.fee_sat,
                        fee_usd: ro.fee_usd,
                        donee: ro.donee.clone(),
                    },
                    OutflowClass::Dispose { kind } => Op::Dispose {
                        sat: t.sat,
                        proceeds: ro.principal_proceeds_or_fmv,
                        fee_usd: ro.fee_usd.unwrap_or(Usd::ZERO),
                        fee_sat: t.fee_sat, // I-1: pass on-chain fee through (was silently dropped)
                        kind: *kind,
                    },
                };
            }
            Op::PendingOut {
                sat: t.sat,
                fee_sat: t.fee_sat,
            }
        }
        EventPayload::TransferIn(t) => {
            if consumed_ins.contains(id) {
                // Consumed by a TransferLink — do nothing (the link relocates the lots).
                Op::Skip
            } else if let Some(cls) = inbound_class.get(id) {
                match cls {
                    InboundClass::Income {
                        kind,
                        fmv,
                        business,
                    } => Op::IncomeInbound {
                        sat: t.sat,
                        fmv: *fmv,
                        kind: *kind,
                        business: *business,
                    },
                    InboundClass::GiftReceived {
                        donor_basis,
                        donor_acquired_at,
                        fmv_at_gift,
                    } => Op::GiftReceived {
                        sat: t.sat,
                        donor_basis: *donor_basis,
                        donor_acquired_at: *donor_acquired_at,
                        fmv_at_gift: *fmv_at_gift,
                    },
                    InboundClass::SelfTransferMine { basis, acquired_at } => {
                        Op::SelfTransferInbound {
                            sat: t.sat,
                            basis: *basis,
                            acquired_at: *acquired_at,
                        }
                    }
                }
            } else {
                Op::UnknownInbound { sat: t.sat }
            }
        }
        EventPayload::Unclassified(_) => Op::Unclassified,
        // Conservative-filing (SPEC D-1): a tranche folds as a $0-basis acquire via the shared Op::Acquire
        // arm — which yields acquired_at = eff.date() = window_end, usd_basis = $0, pool_key(window_end),
        // and bumps sigma_in. No new fold arm needed; D-1a-e (sigma_in) is satisfied structurally.
        //
        // P9/T15 hardening (review Minor): the arm is guarded on `EventId::Decision` AND `sat > 0`. The
        // legitimate decision-admit path (resolve pass-2, ~:1090) already matches only a Decision-id
        // DeclareTranche, but the IMPORT-processing path also calls `build_op` — so a hand-crafted vault
        // routing a DeclareTranche payload through an Import id (or a `ClassifyRaw{as_: DeclareTranche}`)
        // would otherwise fold a bogus lot homed at the IMPORT timestamp (bypassing D-2's window_end) and
        // skip cmd/tranche.rs's record-time `sat > 0` guard (a `sat <= 0` acquire corrupts Σ-conservation
        // by bumping `sigma_in` non-positively). Guarding here folds NOTHING (`Op::Skip`) for either
        // malformed shape, matching the engine's posture on any other malformed hand-crafted payload.
        EventPayload::DeclareTranche(t) if matches!(id, EventId::Decision { .. }) && t.sat > 0 => {
            Op::Acquire(Acquire {
                sat: t.sat,
                usd_cost: Usd::ZERO,
                fee_usd: Usd::ZERO,
                basis_source: BasisSource::EstimatedConservative,
            })
        }
        // Approach-B (BG-D1, arch census item 11): a `PromoteTranche` decision never folds as its OWN
        // `Op` — its entire effect is the target `DeclareTranche`'s `Op::Acquire.usd_cost` rewrite,
        // performed in `resolve`'s step-2 admit branch (via `live_promotes`/`Resolution.promotes`), not a
        // movement of its own. The `_ => Op::Skip` catch-all below already handled this correctly; this
        // arm names it explicitly so the census has a non-silent home for the guarantee.
        EventPayload::PromoteTranche(_) => Op::Skip,
        _ => Op::Skip,
    }
}

/// Approach-B / BG-D1 (Task 3): the promotions in force at pass-2 — target `DeclareTranche` `EventId` ->
/// the stored WHOLE-tranche `filed_basis` + the target's `sat` (the BG-D4 decomposition key,
/// `conservative_promote::PromoteEntry`). Built ONCE, before the step-2 timeline loop, from non-voided
/// `PromoteTranche` decisions whose `target` resolves to a PRESENT, non-voided `DeclareTranche` event.
///
/// BG-D9 (Task 7): `blockers` is now FILLED — a `PromoteTranche` decision is engine-adjudicated, not
/// silently dropped. A target named by ≥2 non-voided promotes → `DecisionConflict` on EACH such promote,
/// apply NEITHER (NOT last-wins). A non-voided promote whose target is absent / wrong-type / voided →
/// `DecisionConflict`. Only NON-voided promotes are adjudicated (a voided promote is inert — no blocker;
/// arch r3 N-1). The result set `promotes` therefore contains a target iff EXACTLY ONE non-voided promote
/// names a present, non-voided `DeclareTranche` — i.e. iff voiding that tranche would be inert (the
/// invariant `voidable_decisions`' `promoted_target` exclusion keys on).
fn live_promotes(
    events: &[LedgerEvent],
    voided: &BTreeSet<EventId>,
    blockers: &mut Vec<Blocker>,
) -> PromoteSet {
    // The one unified DecisionConflict remedy pointer (kept in sync with `resolve`'s local const).
    const CONFLICT_HINT: &str = "see `btctax events list` for event refs + decision status";
    let by_id: BTreeMap<EventId, &LedgerEvent> = events.iter().map(|e| (e.id.clone(), e)).collect();
    let mut decisions: Vec<(u64, &LedgerEvent)> = events
        .iter()
        .filter_map(|e| match e.id {
            EventId::Decision { seq } => Some((seq, e)),
            _ => None,
        })
        .collect();
    decisions.sort_by_key(|(s, _)| *s);

    // Count NON-VOIDED promotes per target so a target named by ≥2 is a conflict (both inert), NOT a
    // silent latest-seq-wins. Built over the SAME non-voided promote set the apply loop below iterates.
    let mut promote_count: BTreeMap<EventId, usize> = BTreeMap::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        if let EventPayload::PromoteTranche(p) = &d.payload {
            *promote_count.entry(p.target.clone()).or_insert(0) += 1;
        }
    }

    let mut promotes = PromoteSet::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue; // a voided promote is inert — never a conflict (arch r3 N-1)
        }
        let EventPayload::PromoteTranche(p) = &d.payload else {
            continue;
        };
        // ≥2 non-voided promotes on one target → conflict, apply NEITHER (not last-wins).
        if promote_count.get(&p.target).copied().unwrap_or(0) >= 2 {
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: Some(d.id.clone()),
                detail: format!(
                    "multiple live PromoteTranche decisions name the same tranche {} — none applies; \
                     void all but one to choose a floor — {CONFLICT_HINT}",
                    p.target.canonical()
                ),
            });
            continue;
        }
        // The target must be a PRESENT, non-voided `DeclareTranche`; else the promote is a conflict
        // (absent / wrong-type / voided target). A promoted tranche is never in `voided` at this point
        // (its void is deferred, not applied inline) — the `!voided` guard is defensive.
        let live_target = match by_id.get(&p.target).map(|e| &e.payload) {
            Some(EventPayload::DeclareTranche(t)) if !voided.contains(&p.target) => Some(t),
            _ => None,
        };
        match live_target {
            Some(t) => {
                promotes.insert(
                    p.target.clone(),
                    PromoteEntry {
                        filed_basis: p.filed_basis,
                        tranche_sat: t.sat,
                    },
                );
            }
            None => {
                blockers.push(Blocker {
                    kind: BlockerKind::DecisionConflict,
                    event: Some(d.id.clone()),
                    detail: format!(
                        "PromoteTranche targets {} which is not a live DeclareTranche (absent, wrong \
                         type, or voided) — {CONFLICT_HINT}",
                        p.target.canonical()
                    ),
                });
            }
        }
    }
    promotes
}

/// PASS 1. Task 7: staged decision resolution (§7.2 step 1); Task 12: §7.4 transition effectiveness.
///
/// `prices`/`config` are USED by the Task-12 transition: `config` keys the TP8(b) first-2025-disposition
/// trigger, and `prices` feeds the allocation-independent pre-2025 Universal snapshot that the safe-harbor
/// conservation guard checks against (`transition::universal_snapshot`, I-1).
pub fn resolve(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
) -> Resolution {
    // Index events by id for O(log n) lookup.
    let by_id: BTreeMap<EventId, &LedgerEvent> = events.iter().map(|e| (e.id.clone(), e)).collect();
    let mut blockers: Vec<Blocker> = Vec::new();

    // UX-P4-3: the ONE unified `DecisionConflict` remedy pointer (SPEC §3.2, names `events list` §3.6).
    // Surface-neutral by design — it works at `verify` (the conflicting decision IS recorded; the list
    // shows its `decision|N` to void) AND at record time (nothing was recorded; the list shows the
    // valid refs). Duplicate details add "; void the prior decision to re-decide" (a prior decision
    // exists on both surfaces) — EXCEPT the classify-raw arm, whose prior may be a non-revocable
    // accepted `SupersedeImport`, so it says "if the prior decision is revocable, void it to re-decide"
    // (review r2 M1). Deliberately NOT "void the decision to clear this blocker" — that misreads at
    // record time, where nothing was appended.
    const CONFLICT_HINT: &str = "see `btctax events list` for event refs + decision status";

    // ── Pseudo-reconcile mode (sub-project 2) ────────────────────────────────────────────────────
    // When on, synthesize DELIBERATELY-FICTIONAL default decisions at the map/`Eff` layer (never
    // persisted) to clear the Hard classification blockers. Real decisions are collected FIRST (below)
    // so an event with ANY real decision gets NO synthetic (real supersedes). `pseudo_ids` = the IMPORT
    // events whose effective `Op` a synthetic governs (→ `Eff.pseudo` → `Lot`/leg taint, [R0-C1]);
    // `pseudo_decisions` = the materializable REAL decisions `approve` persists ("see == approve").
    // NOTE [R0-I1]: synthetics are MAP-layer entries — we NEVER mint `EventId::Decision{seq}` here
    // (that u64 collides the real decision_seq space, identity.rs:69); seq-minting lives only in `approve`.
    let pseudo_on = config.pseudo_reconcile;
    let mut pseudo_ids: BTreeSet<EventId> = BTreeSet::new();
    let mut pseudo_decisions: Vec<PseudoDefault> = Vec::new();

    // ── 1a. Collect Voids; classify each target's revocability ──────────────────────────────────
    //   Revocable targets: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw,
    //   MethodElection, LotSelection, ReclassifyIncome.
    //   NON-revocable targets: SupersedeImport, RejectImport, VoidDecisionEvent.
    //   Void of a non-revocable target → DecisionConflict (target stays in force; void is inert).
    //   Void of SafeHarborAllocation → collected in allocation_voids (deferred to Task 12).
    let mut voided: BTreeSet<EventId> = BTreeSet::new();
    let mut allocation_voids: Vec<AllocationVoid> = Vec::new();
    // BG-D9 (Task 7): a void of a `DeclareTranche` that ANY `PromoteTranche` references is DEFERRED
    // (order-independent, arch r2 M-1) and adjudicated after `live_promotes`. `promote_targets` is the
    // STATIC set of ids any promote (voided or not) names — the defer trigger. Do NOT read "live" here.
    let mut tranche_voids: Vec<TrancheVoid> = Vec::new();
    let promote_targets: BTreeSet<EventId> = events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::PromoteTranche(p) => Some(p.target.clone()),
            _ => None,
        })
        .collect();
    for e in events {
        if let EventPayload::VoidDecisionEvent(v) = &e.payload {
            match by_id.get(&v.target_event_id).map(|t| &t.payload) {
                Some(EventPayload::SupersedeImport(_))
                | Some(EventPayload::RejectImport(_))
                | Some(EventPayload::VoidDecisionEvent(_)) => {
                    // Non-revocable: the void itself is the conflict; target stays in force.
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(e.id.clone()),
                        detail: format!(
                            "void targets a non-revocable decision (accept/reject-conflict and \
                             void are permanent) — {CONFLICT_HINT}"
                        ),
                    });
                }
                Some(EventPayload::SafeHarborAllocation(_)) => {
                    // Deferred to Task 12 (effective allocation → conflict; inert → apply void).
                    allocation_voids.push(AllocationVoid {
                        void_id: e.id.clone(),
                        target: v.target_event_id.clone(),
                    });
                }
                Some(EventPayload::PromoteTranche(_)) => {
                    // BG-D9: a promote-void ALWAYS applies inline+unconditionally — its liveness never
                    // depends on another decision, so there is no order-dependence to defer. (This is
                    // what the `Some(_)` catch-all already did; named explicitly for the census.)
                    voided.insert(v.target_event_id.clone());
                }
                Some(EventPayload::DeclareTranche(_)) => {
                    // BG-D9: a void of a tranche that a promote references is DEFERRED (adjudicated
                    // against the FINAL live-promote set after `live_promotes`, before step 2). A void of
                    // a tranche NO promote references applies inline (existing D-1a-d behavior).
                    if promote_targets.contains(&v.target_event_id) {
                        tranche_voids.push(TrancheVoid {
                            void_id: e.id.clone(),
                            target: v.target_event_id.clone(),
                        });
                    } else {
                        voided.insert(v.target_event_id.clone());
                    }
                }
                Some(_) => {
                    voided.insert(v.target_event_id.clone());
                }
                None => {
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(e.id.clone()),
                        detail: format!("void targets unknown event — {CONFLICT_HINT}"),
                    });
                }
            }
        }
    }

    // ── 1b. Resolve ImportConflicts ─────────────────────────────────────────────────────────────
    // Gather decisions in decision_seq order; for each conflict, the latest SupersedeImport /
    // RejectImport governs (§7.2, latest-seq-wins). Build applied: target_id → effective payload.
    let mut applied: BTreeMap<EventId, EventPayload> = BTreeMap::new();

    // Collect and sort decisions ascending by seq.
    let mut decisions: Vec<(u64, &LedgerEvent)> = events
        .iter()
        .filter_map(|e| match e.id {
            EventId::Decision { seq } => Some((seq, e)),
            _ => None,
        })
        .collect();
    decisions.sort_by_key(|(s, _)| *s);

    // Map conflict_event_id → latest governing resolution (ascending iteration = last write wins).
    let mut conflict_res: BTreeMap<EventId, Resolved> = BTreeMap::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        match &d.payload {
            EventPayload::SupersedeImport(s) => {
                if let Some(EventPayload::ImportConflict(c)) =
                    by_id.get(&s.conflict_event).map(|e| &e.payload)
                {
                    conflict_res.insert(
                        s.conflict_event.clone(),
                        Resolved::Accept(c.new_payload.clone(), c.target.clone()),
                    );
                }
            }
            EventPayload::RejectImport(r) => {
                if let Some(EventPayload::ImportConflict(_)) =
                    by_id.get(&r.conflict_event).map(|e| &e.payload)
                {
                    conflict_res.insert(r.conflict_event.clone(), Resolved::Reject);
                }
            }
            _ => {}
        }
    }

    // Unresolved conflicts → ImportConflict blocker; accepted → applied override on the target id.
    for e in events {
        if let EventPayload::ImportConflict(c) = &e.payload {
            match conflict_res.get(&e.id) {
                Some(Resolved::Accept(payload, target)) => {
                    applied.insert(target.clone(), (**payload).clone());
                }
                Some(Resolved::Reject) => {} // original import stands unchanged
                None => {
                    // [R0-C2] pseudo accept-first: an unresolved ImportConflict adopts the FIRST-SEEN
                    // conflict's new payload onto its target (map-clearable), flagged pseudo — no blocker.
                    // First-wins guard: skip if the target already has a governing override (real or a
                    // prior accept-first). `DecisionConflict` (a REAL-decision collision) is NOT cleared.
                    if pseudo_on && !applied.contains_key(&c.target) {
                        applied.insert(c.target.clone(), (*c.new_payload).clone());
                        pseudo_ids.insert(c.target.clone()); // the TARGET lot's basis now traces to pseudo
                        pseudo_decisions.push(PseudoDefault {
                            target: e.id.clone(), // the conflict event the SupersedeImport references
                            decision: EventPayload::SupersedeImport(SupersedeImport {
                                conflict_event: e.id.clone(),
                            }),
                            kind: PseudoKind::AcceptConflict,
                        });
                    } else {
                        blockers.push(Blocker {
                            kind: BlockerKind::ImportConflict,
                            event: Some(e.id.clone()),
                            detail: "unresolved import conflict".into(),
                        });
                    }
                }
            }
        }
    }

    // ── 1c. ClassifyRaw ─────────────────────────────────────────────────────────────────────────
    // Replace an Unclassified target's effective payload (preserve target EventId).
    // Contradictory ClassifyRaw on a target already overridden → DecisionConflict.
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        if let EventPayload::ClassifyRaw(cr) = &d.payload {
            if applied.contains_key(&cr.target) {
                blockers.push(Blocker {
                    kind: BlockerKind::DecisionConflict,
                    event: Some(d.id.clone()),
                    detail: format!(
                        "duplicate classify-raw: {} is already classified \
                         — {CONFLICT_HINT}; if the prior decision is revocable, void it to re-decide",
                        cr.target.canonical()
                    ),
                });
            } else {
                applied.insert(cr.target.clone(), (*cr.as_).clone());
            }
        }
    }

    // ── 1d. ManualFmv ───────────────────────────────────────────────────────────────────────────
    // Collect event_id → usd_fmv; latest decision_seq wins (ascending iteration = last write wins).
    // Validates the effective target payload at collection time (all four decision types in passes
    // 1d/1e now share this invariant: target absent or wrong type → Hard DecisionConflict, EXCLUDED).
    // Note: ManualFmv deliberately keeps latest-seq-wins with NO duplicate blocker — a valid
    // re-pointing of an FMV is a correction flow, not a conflict.
    let mut manual_fmv: BTreeMap<EventId, Usd> = BTreeMap::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        if let EventPayload::ManualFmv(m) = &d.payload {
            let target = &m.event;
            let effective_payload = by_id
                .get(target)
                .map(|raw| applied.get(target).unwrap_or(&raw.payload));
            match effective_payload {
                None => {
                    // Target event does not exist in the ledger at all.
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: format!(
                            "ManualFmv targets unknown event {} — {CONFLICT_HINT}",
                            target.canonical()
                        ),
                    });
                    // Decision EXCLUDED: not inserted into manual_fmv.
                }
                Some(EventPayload::Income(_)) => {
                    // Valid Income target. Latest-seq-wins: ascending iteration = last write wins.
                    // NO duplicate blocker (deliberate — re-pointing an FMV is a correction flow).
                    manual_fmv.insert(target.clone(), m.usd_fmv);
                }
                Some(_) => {
                    // Target exists but its effective payload is not Income. Hard blocker; EXCLUDED.
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: format!(
                            "ManualFmv targets non-Income event {} \
                             — for a TransferIn classified as income, set the FMV via \
                             classify-inbound-income (its own `fmv` field); {CONFLICT_HINT}",
                            target.canonical()
                        ),
                    });
                    // Decision EXCLUDED: not inserted into manual_fmv.
                }
            }
        }
    }

    // ── 1e. Classification decisions ────────────────────────────────────────────────────────────
    // TransferLink → links + consumed_ins; ClassifyInbound → inbound_class; ReclassifyOutflow → outflow_class;
    // ReclassifyIncome (SE Chunk C) → income_reclassify.
    // Contradiction detection (same-target multi-class) for all four.
    let mut links: BTreeMap<EventId, TransferTarget> = BTreeMap::new();
    let mut consumed_ins: BTreeSet<EventId> = BTreeSet::new();
    let mut inbound_class: BTreeMap<EventId, InboundClass> = BTreeMap::new();
    let mut outflow_class: BTreeMap<EventId, ReclassifyOutflow> = BTreeMap::new();
    let mut income_reclassify: BTreeMap<EventId, ReclassifyIncome> = BTreeMap::new();
    // Cycle B: both legs of a confirmed passthrough self-transfer → `Op::Skip`. `passthroughs` keeps the
    // accepted (decision id, in_event, out_event) triples so the [R0-I1] cross-type overlap guard can run
    // AFTER every pass-1e map is built (below the loop).
    let mut passthrough_skip: BTreeSet<EventId> = BTreeSet::new();
    let mut passthroughs: Vec<(EventId, EventId, EventId)> = Vec::new();

    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        match &d.payload {
            EventPayload::TransferLink(tl) => {
                if links.contains_key(&tl.out_event) {
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: "duplicate TransferLink for the same out_event".into(),
                    });
                } else {
                    let mut link_ok = true;
                    if let TransferTarget::InEvent(in_id) = &tl.in_event_or_wallet {
                        if consumed_ins.contains(in_id) {
                            // M-3: two distinct TransferLinks name the same in-event → conflict.
                            blockers.push(Blocker {
                                kind: BlockerKind::DecisionConflict,
                                event: Some(d.id.clone()),
                                detail: "duplicate TransferLink targeting the same in_event".into(),
                            });
                            link_ok = false;
                        } else if by_id.get(in_id).and_then(|e| e.wallet.as_ref()).is_none() {
                            // I-1: linked in-event has no resolvable destination wallet → hard blocker.
                            // Do NOT add to consumed_ins so the in-event is NOT silently Skipped.
                            blockers.push(Blocker {
                                kind: BlockerKind::DecisionConflict,
                                event: Some(d.id.clone()),
                                detail:
                                    "TransferLink in-event has no resolvable destination wallet"
                                        .into(),
                            });
                            link_ok = false;
                        } else {
                            consumed_ins.insert(in_id.clone());
                        }
                    }
                    if link_ok {
                        links.insert(tl.out_event.clone(), tl.in_event_or_wallet.clone());
                    }
                }
            }
            EventPayload::ClassifyInbound(ci) => {
                let target = &ci.transfer_in_event;
                let effective_payload = by_id
                    .get(target)
                    .map(|raw| applied.get(target).unwrap_or(&raw.payload));
                match effective_payload {
                    None => {
                        // Target event does not exist in the ledger at all.
                        blockers.push(Blocker {
                            kind: BlockerKind::DecisionConflict,
                            event: Some(d.id.clone()),
                            detail: format!(
                                "ClassifyInbound targets unknown event {} — {CONFLICT_HINT}",
                                target.canonical()
                            ),
                        });
                        // Decision EXCLUDED: not inserted into inbound_class.
                    }
                    Some(EventPayload::TransferIn(_)) => {
                        // Valid TransferIn target. Duplicate → conflict; FIRST-WINS.
                        // Deliberately NOT validated (D1 non-goal): a TransferIn consumed by a
                        // TransferLink (`consumed_ins`) passes type-validation here — the link
                        // consumes first in build_op. That is a *precedence* question, not a bad
                        // target; unchanged.
                        if inbound_class.contains_key(target) {
                            blockers.push(Blocker {
                                kind: BlockerKind::DecisionConflict,
                                event: Some(d.id.clone()),
                                detail: format!(
                                    "duplicate ClassifyInbound: {} is already classified \
                                     — {CONFLICT_HINT}; void the prior decision to re-decide",
                                    target.canonical()
                                ),
                            });
                            // Second decision EXCLUDED; first-wins value stays in map.
                        } else {
                            inbound_class.insert(target.clone(), ci.as_.clone());
                        }
                    }
                    Some(_) => {
                        // Target exists but its effective payload is not TransferIn. Hard blocker; EXCLUDED.
                        blockers.push(Blocker {
                            kind: BlockerKind::DecisionConflict,
                            event: Some(d.id.clone()),
                            detail: format!(
                                "ClassifyInbound targets non-TransferIn event {} \
                                 — only a deposit (TransferIn) can be classified inbound; {CONFLICT_HINT}",
                                target.canonical()
                            ),
                        });
                        // Decision EXCLUDED: not inserted into inbound_class.
                    }
                }
            }
            EventPayload::ReclassifyOutflow(ro) => {
                let target = &ro.transfer_out_event;
                let effective_payload = by_id
                    .get(target)
                    .map(|raw| applied.get(target).unwrap_or(&raw.payload));
                match effective_payload {
                    None => {
                        // Target event does not exist in the ledger at all.
                        blockers.push(Blocker {
                            kind: BlockerKind::DecisionConflict,
                            event: Some(d.id.clone()),
                            detail: format!(
                                "ReclassifyOutflow targets unknown event {} — {CONFLICT_HINT}",
                                target.canonical()
                            ),
                        });
                        // Decision EXCLUDED: not inserted into outflow_class.
                    }
                    Some(EventPayload::TransferOut(_)) => {
                        // Valid TransferOut target. Duplicate → conflict; FIRST-WINS.
                        // Deliberately NOT validated (D1 non-goal): a TransferOut that is ALSO
                        // TransferLink'd passes type-validation but stays overridden by the link
                        // (links win in build_op, before outflow_class). That is a *precedence*
                        // question, not a bad target; unchanged.
                        if outflow_class.contains_key(target) {
                            blockers.push(Blocker {
                                kind: BlockerKind::DecisionConflict,
                                event: Some(d.id.clone()),
                                detail: format!(
                                    "duplicate ReclassifyOutflow: {} is already reclassified \
                                     — {CONFLICT_HINT}; void the prior decision to re-decide",
                                    target.canonical()
                                ),
                            });
                            // Second decision EXCLUDED; first-wins value stays in map.
                        } else {
                            outflow_class.insert(target.clone(), ro.clone());
                        }
                    }
                    Some(_) => {
                        // Target exists but its effective payload is not TransferOut. Hard blocker; EXCLUDED.
                        blockers.push(Blocker {
                            kind: BlockerKind::DecisionConflict,
                            event: Some(d.id.clone()),
                            detail: format!(
                                "ReclassifyOutflow targets non-TransferOut event {} \
                                 — for Income corrections use reclassify-income; {CONFLICT_HINT}",
                                target.canonical()
                            ),
                        });
                        // Decision EXCLUDED: not inserted into outflow_class.
                    }
                }
            }
            EventPayload::ReclassifyIncome(ri) => {
                // SE Chunk C (D2) / burndown-3 D1: bad-target validation at collection time against the
                // EFFECTIVE payload (`applied.get(&target).unwrap_or(raw)` — so a ClassifyRaw'd row that
                // became Income stays reclassifiable, and a by_id miss counts as bad).
                // All four decision types in passes 1d/1e now validate the effective target payload at
                // collection time: ReclassifyOutflow→TransferOut, ClassifyInbound→TransferIn,
                // ManualFmv→Income (pass 1d), and ReclassifyIncome→Income (here, pass 1e).
                // Precedents: TransferLink in-event check (~456-466), LotSelection (~604-611).
                let target = &ri.income_event;
                let effective_payload = by_id
                    .get(target)
                    .map(|raw| applied.get(target).unwrap_or(&raw.payload));
                match effective_payload {
                    None => {
                        // Target event does not exist in the ledger at all.
                        blockers.push(Blocker {
                            kind: BlockerKind::DecisionConflict,
                            event: Some(d.id.clone()),
                            detail: format!(
                                "ReclassifyIncome targets unknown event {} \
                                 — for TransferIn rows use classify-inbound-income; {CONFLICT_HINT}",
                                target.canonical()
                            ),
                        });
                        // Decision EXCLUDED: not inserted into income_reclassify.
                    }
                    Some(EventPayload::Income(_)) => {
                        // Valid Income target. Duplicate (second non-voided for same target) → conflict;
                        // FIRST-WINS (ascending decision_seq iteration = first write wins).
                        if income_reclassify.contains_key(target) {
                            blockers.push(Blocker {
                                kind: BlockerKind::DecisionConflict,
                                event: Some(d.id.clone()),
                                detail: format!(
                                    "duplicate ReclassifyIncome: {} is already reclassified \
                                     — {CONFLICT_HINT}; void the prior decision to re-decide",
                                    target.canonical()
                                ),
                            });
                            // Second decision EXCLUDED; first-wins value stays in map.
                        } else {
                            income_reclassify.insert(target.clone(), ri.clone());
                        }
                    }
                    Some(_) => {
                        // Target exists but its effective payload is not Income (e.g. a TransferIn,
                        // Acquire, or a reclassified TransferOut). Hard blocker; decision EXCLUDED.
                        blockers.push(Blocker {
                            kind: BlockerKind::DecisionConflict,
                            event: Some(d.id.clone()),
                            detail: format!(
                                "ReclassifyIncome targets non-Income event {} \
                                 — for TransferIn rows use classify-inbound-income; {CONFLICT_HINT}",
                                target.canonical()
                            ),
                        });
                        // Decision EXCLUDED: not inserted into income_reclassify.
                    }
                }
            }
            EventPayload::SelfTransferPassthrough(stp) => {
                // Cycle B (C1): the DROP primitive — validate BOTH targets against their EFFECTIVE
                // payloads (mirror ClassifyInbound/ReclassifyOutflow): in_event must be a TransferIn AND
                // out_event a TransferOut, or the WHOLE decision is excluded [R0-N2] (bad target → Hard
                // DecisionConflict; NEITHER leg enters passthrough_skip). Duplicate = EITHER leg already
                // claimed by a prior passthrough [R0-N3] → conflict + first-wins (mirror TransferLink's
                // dual check, :507/516). The [R0-I1] cross-type overlap guard runs BELOW, after all maps.
                let in_target = &stp.in_event;
                let out_target = &stp.out_event;
                let in_payload = by_id
                    .get(in_target)
                    .map(|raw| applied.get(in_target).unwrap_or(&raw.payload));
                let out_payload = by_id
                    .get(out_target)
                    .map(|raw| applied.get(out_target).unwrap_or(&raw.payload));
                let in_ok = matches!(in_payload, Some(EventPayload::TransferIn(_)));
                let out_ok = matches!(out_payload, Some(EventPayload::TransferOut(_)));
                if !in_ok || !out_ok {
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: format!(
                            "SelfTransferPassthrough requires in_event {} to be a TransferIn and \
                             out_event {} to be a TransferOut — {CONFLICT_HINT}",
                            in_target.canonical(),
                            out_target.canonical()
                        ),
                    });
                    // WHOLE decision EXCLUDED: neither leg enters passthrough_skip (G-BOTH-ATOMIC).
                } else if passthrough_skip.contains(in_target)
                    || passthrough_skip.contains(out_target)
                {
                    // Duplicate: EITHER leg is already claimed by an earlier passthrough. First-wins;
                    // this (later-seq) decision is a conflict and is EXCLUDED.
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: format!(
                            "duplicate SelfTransferPassthrough claims a leg already in another \
                             passthrough — {CONFLICT_HINT}; void the prior decision to re-decide"
                        ),
                    });
                } else {
                    passthrough_skip.insert(in_target.clone());
                    passthrough_skip.insert(out_target.clone());
                    passthroughs.push((d.id.clone(), in_target.clone(), out_target.clone()));
                }
            }
            _ => {}
        }
    }

    // ── 1e-I1. [R0-I1] cross-type overlap guard (MANDATORY, load-bearing tax-safety) ─────────────
    // A passthrough leg must be UNRECONCILED on BOTH legs. This runs AFTER every pass-1e map is built
    // (a passthrough may be appended BEFORE the conflicting classification, so the check cannot live in
    // the collector arm). EXCLUDE any passthrough whose out_event ALSO carries a ReclassifyOutflow/
    // TransferLink OR whose in_event ALSO carries a ClassifyInbound / is consumed by a TransferLink —
    // raise a Hard DecisionConflict and remove BOTH ids from passthrough_skip. The passthrough LOSES so
    // the taxable classification WINS: otherwise a passthrough would `Op::Skip` a leg that has a real
    // Dispose/Income and SILENTLY ERASE a taxable event (the exact failure the governing policy forbids).
    for (dec_id, in_ev, out_ev) in &passthroughs {
        let out_overlaps = outflow_class.contains_key(out_ev) || links.contains_key(out_ev);
        let in_overlaps = inbound_class.contains_key(in_ev) || consumed_ins.contains(in_ev);
        if out_overlaps || in_overlaps {
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: Some(dec_id.clone()),
                detail:
                    "SelfTransferPassthrough leg also carries a taxable classification \
                     (ReclassifyOutflow/TransferLink on the out-leg, or ClassifyInbound/TransferLink on \
                     the in-leg) — the passthrough is EXCLUDED so the taxable event is recognized; \
                     void the conflicting decision if the passthrough is correct"
                        .into(),
            });
            // Remove BOTH ids (G-BOTH-ATOMIC): each leg belongs to at most one accepted passthrough
            // (duplicate detection above), so this never disturbs another passthrough's membership.
            passthrough_skip.remove(in_ev);
            passthrough_skip.remove(out_ev);
        }
    }

    // ── 1f. Pseudo-reconcile classification defaults (sub-project 2) ──────────────────────────────
    // Injected AFTER every real-decision map is built (real supersedes) and BEFORE the timeline build,
    // so the synthetics flow through the SAME `build_op` → fold path as real decisions.
    //   Phase A — `Unclassified` (determinable-inbound: has a wallet) with no real ClassifyRaw →
    //             synthetic `ClassifyRaw` to a zero-value placeholder [R0-M2]. The `Unclassified` row
    //             carries NO structured amount, so pseudo books a $0/0-sat placeholder (Op::Acquire) that
    //             clears the blocker but fabricates no holdings — the user supplies the real classification
    //             + amount when correcting. NOT `ClassifyInbound` (rejected on a non-`TransferIn` target →
    //             `DecisionConflict`). A wallet-less `Unclassified` is LEFT SURFACED (nowhere to home a lot).
    //   Phase B — an effective `TransferIn` (real, OR one just conjured) with no real inbound
    //             classification / link / passthrough → synthetic `ClassifyInbound(SelfTransferMine{$0})`
    //             (conservative $0, non-taxable — never income, assumption 3). Clears `UnknownBasisInbound`.
    if pseudo_on {
        // Phase A: Unclassified → ClassifyRaw(zero-value placeholder). Iterate in event order.
        for e in events {
            if !matches!(e.id, EventId::Import { .. }) || e.wallet.is_none() {
                continue;
            }
            if applied.contains_key(&e.id) {
                continue; // a real (or accept-first) override already governs this row
            }
            if matches!(&e.payload, EventPayload::Unclassified(_)) {
                let placeholder = EventPayload::Acquire(Acquire {
                    sat: 0,
                    usd_cost: Usd::ZERO,
                    fee_usd: Usd::ZERO,
                    basis_source: BasisSource::SelfTransferInbound,
                });
                applied.insert(e.id.clone(), placeholder.clone());
                pseudo_ids.insert(e.id.clone());
                pseudo_decisions.push(PseudoDefault {
                    target: e.id.clone(),
                    decision: EventPayload::ClassifyRaw(ClassifyRaw {
                        target: e.id.clone(),
                        as_: Box::new(placeholder),
                    }),
                    kind: PseudoKind::RawInbound,
                });
            }
        }
        // Phase B: effective TransferIn with no real classification → SelfTransferMine{$0}.
        for e in events {
            if !matches!(e.id, EventId::Import { .. }) || e.wallet.is_none() {
                continue;
            }
            let eff_payload = applied.get(&e.id).unwrap_or(&e.payload);
            let is_unresolved_transfer_in = matches!(eff_payload, EventPayload::TransferIn(_))
                && !inbound_class.contains_key(&e.id)
                && !consumed_ins.contains(&e.id)
                && !passthrough_skip.contains(&e.id);
            if is_unresolved_transfer_in {
                let as_ = InboundClass::SelfTransferMine {
                    basis: None, // defaulted $0 (conservative, max eventual gain) + the honesty advisory
                    acquired_at: None, // defaulted to 1yr+1day before receipt → long-term (disclosed)
                };
                inbound_class.insert(e.id.clone(), as_.clone());
                pseudo_ids.insert(e.id.clone());
                pseudo_decisions.push(PseudoDefault {
                    target: e.id.clone(),
                    decision: EventPayload::ClassifyInbound(ClassifyInbound {
                        transfer_in_event: e.id.clone(),
                        as_,
                    }),
                    kind: PseudoKind::SelfTransferInbound,
                });
            }
        }
        // Phase C (#41 Part B): a NATIVE `Income` whose EFFECTIVE FMV is missing (no real `ManualFmv`
        // AND the import carried no usable FMV) → synthetic `ManualFmv` at the daily close. Injected as
        // a `manual_fmv` entry so it flows through the SAME `build_op` path as a real ManualFmv, and the
        // event id joins `pseudo_ids` so the taint reaches the `IncomeRecord` [R0-I2]. Guarded on BOTH a
        // wallet (wallet-less income is a separate FmvMissing the FMV can't fix) AND `prices` HAVING a
        // close at the date — NO price ⇒ NO synthetic (stay [FmvMissing]; the ★ fault-inject guard).
        for e in events {
            if !matches!(e.id, EventId::Import { .. }) || e.wallet.is_none() {
                continue;
            }
            let eff_payload = applied.get(&e.id).unwrap_or(&e.payload);
            let EventPayload::Income(x) = eff_payload else {
                continue;
            };
            // Already has an effective FMV (a real ManualFmv, or the import's own non-Missing FMV)? skip.
            if manual_fmv.contains_key(&e.id)
                || (x.usd_fmv.is_some() && x.fmv_status != FmvStatus::Missing)
            {
                continue;
            }
            let date = tax_date(e.utc_timestamp, e.original_tz);
            let Some(synth) = fmv_of(prices, date, x.sat) else {
                continue; // NO local price ⇒ NO synthetic — the row stays Hard FmvMissing.
            };
            manual_fmv.insert(e.id.clone(), synth);
            pseudo_ids.insert(e.id.clone());
            pseudo_decisions.push(PseudoDefault {
                target: e.id.clone(),
                decision: EventPayload::ManualFmv(ManualFmv {
                    event: e.id.clone(),
                    usd_fmv: synth,
                }),
                kind: PseudoKind::PseudoFmv,
            });
        }
        // NFR4/[N2]: a stable order independent of the input `events` order — approve materializes in
        // this order, and two projections of the same ledger produce byte-identical `pseudo_decisions`.
        pseudo_decisions.sort_by(|a, b| {
            a.target
                .canonical()
                .cmp(&b.target.canonical())
                .then((a.kind as u8).cmp(&(b.kind as u8)))
        });
    }

    // Approach-B / BG-D1 (Task 3): the live promotions, built BEFORE the step-2 loop (needs `voided`,
    // already collected in step 1a) so the DeclareTranche admit branch below can rewrite the promoted
    // tranche's `Op::Acquire.usd_cost` while THIS SAME pass-2 build is still in progress — never after.
    let promotes = live_promotes(events, &voided, &mut blockers);

    // ── 1a-adjudicate. BG-D9 deferred tranche-void resolution (arch r1 I-2) ───────────────────────
    // Adjudicate the deferred `tranche_voids` against the FINAL non-voided-promote set — HERE, after
    // `live_promotes` (so `promotes` is settled) but BEFORE step 2's admit branch reads `voided` (a
    // `voided.insert` at step 3, after the timeline is built, would be a no-op). A target that still
    // carries a LIVE promote → the void is INERT + `DecisionConflict` (never a dangling target); else the
    // void APPLIES so the step-2 admit branch (`if voided.contains(&e.id)`) drops the tranche. This mirrors
    // `allocation_voids`' deferred step-3 pattern, at the pre-step-2 insertion point.
    //
    // ACYCLICITY: promote-liveness (`promotes`) depends ONLY on promote-targeted voids — all applied
    // inline in pass-1a — so `live_promotes` above never observed a deferred tranche-void; inserting them
    // now cannot change `promotes`. The two-stage evaluation is therefore order-independent (both void
    // orders converge: BOTH voided ⇒ promote dead + tranche dropped, no spurious conflict).
    for tv in &tranche_voids {
        if promotes.contains_key(&tv.target) {
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: Some(tv.void_id.clone()),
                detail: format!(
                    "void targets a DeclareTranche held in force by a live PromoteTranche — the void is \
                     inert (never a dangling target); void the promote to revert the tranche to $0, or \
                     void both to drop it — {CONFLICT_HINT}"
                ),
            });
        } else {
            voided.insert(tv.target.clone());
        }
    }

    // ── 2. Build the effective imported timeline ─────────────────────────────────────────────────
    // For each imported event, apply `applied` overrides then `manual_fmv`, emit an `Eff`.
    // Unclassified with no ClassifyRaw → Op::Unclassified (blocker added in fold).
    // Non-import events (decisions, conflicts) are skipped — they have no timeline entry.
    let mut timeline = Vec::new();
    for e in events {
        // Conservative-filing (SPEC D-1/D-1a): a `DeclareTranche` decision is the ONE decision that folds
        // as a PRIMARY movement (via `Op::Acquire`). Admit it with a projection `Eff` whose date IS
        // `window_end` — built from `window_end.midnight()` so `eff.date() == window_end` and `pool_key` /
        // the pre-2025 conservation snapshot bucket it correctly, WITHOUT back-dating the persisted
        // `utc_timestamp` (which keeps its creation-time meaning). Match on `&e.payload` directly (never
        // `applied`): `ClassifyRaw` overrides are scoped to Unclassified imports, so a decision is never
        // legitimately overridden, and reading through `applied` would let a hand-crafted `ClassifyRaw`
        // suppress or forge a tranche. Guard on the Decision id for the same reason.
        if let (EventId::Decision { .. }, EventPayload::DeclareTranche(t)) = (&e.id, &e.payload) {
            if voided.contains(&e.id) {
                continue; // a voided tranche folds nothing (D-1a-d)
            }
            let op = build_op(
                &e.id,
                &e.payload,
                &manual_fmv,
                &links,
                &consumed_ins,
                &inbound_class,
                &outflow_class,
                &income_reclassify,
                &passthrough_skip,
                &by_id,
            );
            // ★ BG-D1 (Task 3, the LOAD-BEARING placement): if this tranche is PROMOTED, rewrite its
            // Op::Acquire.usd_cost to the stored filed_basis HERE — inside resolve's step-2 build, before
            // this Eff is pushed onto `timeline` — so step-3's `universal_snapshot` (§7.4 conservation)
            // and every downstream void/relocation pass see the FLOOR, never the stale $0. A post-resolve
            // mutation (the `overpayment_delta_one` what-if seam, conservative.rs) is the WRONG timing —
            // it would adjudicate conservation against a residue that still reads $0. Only `usd_cost`
            // changes: `basis_source` stays `EstimatedConservative` (the D-8 backstop keys on the TAG,
            // not the amount — a promoted, > $0 tranche still denies a SafeHarborAllocation), and
            // `acquired_at`/the tag exemptions elsewhere are untouched (term-invariance, relocation carry).
            let op = match (op, promotes.get(&e.id)) {
                (Op::Acquire(mut a), Some(entry))
                    if a.basis_source == BasisSource::EstimatedConservative =>
                {
                    a.usd_cost = entry.filed_basis;
                    Op::Acquire(a)
                }
                (op, _) => op,
            };
            timeline.push(Eff {
                id: e.id.clone(),
                utc: t.window_end.midnight().assume_utc(), // effective date = window_end (D-1a-a)
                tz: UtcOffset::UTC,
                src_priority: u8::MAX, // decisions sort after same-instant imports
                // ★ CONSTANT src_ref (D-1a-b): sort_canonical compares src_ref (String-Ord) at key 3,
                //   before the numeric EventId key — a per-seq string would misorder seq 10 vs 2. A
                //   constant lets same-window ties fall through to the numeric id key.
                src_ref: SourceRef::new(""),
                wallet: Some(t.wallet.clone()),
                op,
                pseudo: false, // D-5: a tranche is filing-ready, never pseudo
            });
            continue;
        }
        let (src_priority, src_ref) = match &e.id {
            EventId::Import { source, source_ref } => (source.priority(), source_ref.clone()),
            _ => continue,
        };
        let effective_payload = applied.get(&e.id).unwrap_or(&e.payload);
        let op = build_op(
            &e.id,
            effective_payload,
            &manual_fmv,
            &links,
            &consumed_ins,
            &inbound_class,
            &outflow_class,
            &income_reclassify,
            &passthrough_skip,
            &by_id,
        );
        timeline.push(Eff {
            id: e.id.clone(),
            utc: e.utc_timestamp,
            tz: e.original_tz,
            src_priority,
            src_ref,
            wallet: e.wallet.clone(),
            op,
            pseudo: pseudo_ids.contains(&e.id), // [R0-I1/C1] carry pseudo-ness into the fold's Lot/leg build
        });
    }

    // ── 2b. §A.5(a) MethodElection collection ────────────────────────────────────────────────────
    // Non-voided forward standing orders. A `MethodElection` whose `effective_from` precedes its
    // made-date (back-dating) or TRANSITION_DATE (pre-transition) is a HARD `MethodElectionBackdated`
    // blocker and contributes no in-force record (§1.1012-1(j): no post-hoc identification, ever).
    let mut elections: Vec<ElectionRec> = Vec::new();
    for (seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        if let EventPayload::MethodElection(me) = &d.payload {
            let made = tax_date(d.utc_timestamp, d.original_tz);
            if !method_election_is_forward(me, made) {
                blockers.push(Blocker {
                    kind: BlockerKind::MethodElectionBackdated,
                    event: Some(d.id.clone()),
                    detail: "MethodElection effective_from precedes its made-date or TRANSITION_DATE (2025-01-01) — a standing order cannot be back-dated".into(),
                });
                continue;
            }
            elections.push(ElectionRec {
                effective_from: me.effective_from,
                method: me.method,
                decision_seq: *seq,
                wallet: me.wallet.clone(), // §A.5(a) scope: None = global, Some(w) = per-account
            });
        }
    }
    // ── 2c. §A.4 LotSelection collection + validation ────────────────────────────────────────────
    // Per-disposal specific identification, keyed by its target `disposal_event`. Reuses `decisions`
    // (seq order, NFR4) and `voided`:
    //   - voided LotSelections are excluded;
    //   - two non-voided LotSelections targeting the SAME disposal → DecisionConflict (NEITHER applies),
    //     mirroring the duplicate-ReclassifyOutflow pattern above;
    //   - a selection targeting a non-honoring/unknown event (only Dispose/GiftOut/Donate/SelfTransfer
    //     are selectable — NOT PendingOut/fee legs) → hard `LotSelectionInvalid`;
    //   - principal conservation (§A.4(a)): Σ picked sat MUST equal the disposal's principal sat. The
    //     on-chain `fee_sat` is excluded and consumes FIFO from the post-selection remainder. NOTE
    //     (from Task 2): the pool's `consume_picks` returns shortfall=0, so this Σ check MUST live here.
    // Existence / per-wallet / over-draw are surfaced in the fold (the pool's `selection_error` raises
    // `LotSelectionInvalid`). Every rejection DROPS the selection so the fold falls back to method
    // order — Σsat/Σbasis stay conserved on every path.
    let honoring: BTreeMap<EventId, Sat> = timeline
        .iter()
        .filter_map(|e| honoring_principal(&e.op).map(|s| (e.id.clone(), s)))
        .collect();

    let mut selections: BTreeMap<EventId, Vec<crate::event::LotPick>> = BTreeMap::new();
    let mut seen: BTreeSet<EventId> = BTreeSet::new(); // disposal_events already claimed (dup detection)
    let mut dup: BTreeSet<EventId> = BTreeSet::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        let EventPayload::LotSelection(ls) = &d.payload else {
            continue;
        };
        if !seen.insert(ls.disposal_event.clone()) {
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: Some(d.id.clone()),
                detail: "duplicate LotSelection for the same disposal_event".into(),
            });
            dup.insert(ls.disposal_event.clone());
            continue;
        }
        selections.insert(ls.disposal_event.clone(), ls.lots.clone());
    }
    for id in &dup {
        selections.remove(id); // a conflicted disposal applies NEITHER selection
    }
    // targeting + principal-conservation (§A.4(a)); existence/per-wallet are checked in the fold.
    selections.retain(|disposal, picks| match honoring.get(disposal) {
        None => {
            blockers.push(Blocker {
                kind: BlockerKind::LotSelectionInvalid,
                event: Some(disposal.clone()),
                detail: "LotSelection targets a non-honoring or unknown event (only Dispose/GiftOut/Donate/SelfTransfer — not PendingOut/fee legs)".into(),
            });
            false
        }
        Some(&principal) => {
            let picked: Sat = picks.iter().map(|p| p.sat).sum();
            if picked != principal {
                blockers.push(Blocker {
                    kind: BlockerKind::LotSelectionInvalid,
                    event: Some(disposal.clone()),
                    detail: format!("LotSelection must conserve principal: picked {picked} sat != disposal principal {principal} sat (on-chain fee_sat is excluded and consumes FIFO from the remainder)"),
                });
                false
            } else {
                true
            }
        }
    });

    // ── 3. §7.4 / TP6 transition effectiveness ───────────────────────────────────────────────────
    // Re-evaluated deterministically on every rebuild; reads ONLY the pre-2025 Universal snapshot, the
    // allocations, and `first_2025_disposition` — none of which depend on `transition` (acyclic, I-1/§7.2).
    //
    // (1) Earliest tax-date among 2025 effective DISPOSITION ops. Provisional `PendingOut` and confirmed
    //     (c) self-transfers do NOT count; under TP8 (b) a self-transfer fee-sat mini-disposition DOES.
    let first_2025_disposition: Option<TaxDate> = timeline
        .iter()
        .filter(|e| e.date() >= TRANSITION_DATE)
        .filter(|e| is_disposition_op(&e.op, config))
        .map(|e| e.date())
        .min();

    // (3-prereq) §A.7: the pre-2025 Universal residue is METHOD-aware — each candidate allocation's
    // conservation is checked against the residue computed under ITS OWN recorded `pre2025_method` (below),
    // not a single live-config snapshot. So a non-FIFO filer's allocation conserves against the residue it
    // actually listed; a live-config/recorded-method drift is flagged after Path selection (M2), never here.
    let due = TY2025_RETURN_DUE;
    // (allocation id, pre-built seed lots, the allocation's RECORDED pre2025_method)
    let mut effective: Vec<(EventId, Vec<Lot>, LotMethod)> = Vec::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        let EventPayload::SafeHarborAllocation(a) = &d.payload else {
            continue;
        };
        let made = tax_date(d.utc_timestamp, d.original_tz); // §6.1 calendar-date made-date

        // (2) Method-keyed deadline bar; `timely_allocation_attested` bypasses BOTH prongs.
        //     ActualPosition: barred past the EARLIER-of (first disposition, return-due).
        //     ProRata:        barred past the LATER-of, and additionally requires its pre-2025 method
        //                     description (modeled as the same attestation) — unattested ProRata is barred.
        let bar = match a.method {
            AllocMethod::ActualPosition => min_opt(first_2025_disposition, Some(due)),
            AllocMethod::ProRata => max_opt(first_2025_disposition, Some(due)),
        };
        // Factored: (¬attested ∧ past-bar) ∨ (ProRata ∧ ¬attested) ≡ ¬attested ∧ (past-bar ∨ ProRata)
        // (behavior-identical; stable clippy::nonminimal_bool — surfaced by the first CI run).
        let timebarred = !a.timely_allocation_attested
            && (bar.is_some_and(|b| made > b) || a.method == AllocMethod::ProRata);

        // (3) Conservation vs the pre-2025 Universal snapshot, computed under THIS allocation's RECORDED
        //     method (§A.7) — so a non-FIFO filer conserves against the residue it actually listed. HARD on
        //     failure; attestation cannot bypass it. (A live-config/recorded drift is NOT a conservation
        //     failure: it is flagged after Path selection as `Pre2025MethodConflictsAllocation`, M2.)
        let snap = crate::project::transition::universal_snapshot(
            &timeline,
            prices,
            config,
            a.pre2025_method,
            &elections,
            &selections,
            &promotes,
        );
        let alloc_sat: Sat = a.lots.iter().map(|l| l.sat).sum();
        let alloc_basis: Usd = a.lots.iter().map(|l| l.usd_basis).sum();
        // D-8 backstop (arch r3 New-1 — the real correctness invariant): a SafeHarborAllocation can NEVER
        // go effective while the pre-2025 Universal residue still holds a conservative-filing tranche
        // ($0 EstimatedConservative, remaining_sat > 0). Otherwise a Path-B seed would silently DISCARD
        // the tranche (transition.rs). Independent of declaration order — this closes the inert-then-declare
        // ordering the record-time refusal alone cannot. Kept inert → Path A → the tranche tag survives.
        // Fires for EVERY allocation (voided or not) — a VOIDED-inert allocation's inert blockers are then
        // RETRACTED by the §7.4 retirement pass below, so the supported void-inert-then-declare flow
        // computes via Path A while a NON-voided allocation keeps its Hard (the guarantee) — T16 review r2.
        let has_tranche_residue = snap.estimated_conservative_remaining_sat > 0;
        let unconservable =
            has_tranche_residue || alloc_sat != snap.held_sat || alloc_basis != snap.basis;
        if unconservable {
            blockers.push(Blocker {
                kind: BlockerKind::SafeHarborUnconservable,
                event: Some(d.id.clone()),
                detail: if has_tranche_residue {
                    "a conservative-filing tranche ($0 or a promoted floor, EstimatedConservative) \
                     remains in the pre-2025 residue — a safe-harbor allocation cannot conserve over it \
                     (v1 makes them mutually exclusive; unallocated pre-2025 units are a \
                     facts-and-circumstances matter)"
                        .into()
                } else {
                    "allocation totals != Universal remainder at 2025-01-01".into()
                },
            });
            continue; // inert → Path A
        }
        if timebarred {
            blockers.push(Blocker {
                kind: BlockerKind::SafeHarborTimebar,
                event: Some(d.id.clone()),
                detail: "allocation made past its method-keyed §5.02(4) bar".into(),
            });
            continue; // inert → Path A
        }

        // (4) Capital-asset eligibility (§4.02): assumed for a personal investor (no Phase-1 dealer flag).
        let seed = a
            .lots
            .iter()
            .enumerate()
            .map(|(i, l)| Lot {
                lot_id: LotId {
                    origin_event_id: d.id.clone(),
                    split_sequence: i as u32,
                },
                wallet: l.wallet.clone(),
                acquired_at: l.acquired_at,
                original_sat: l.sat,
                remaining_sat: l.sat,
                usd_basis: l.usd_basis,
                basis_source: BasisSource::SafeHarborAllocated,
                dual_loss_basis: l.dual_loss_basis,
                donor_acquired_at: l.donor_acquired_at,
                basis_pending: false,
                pseudo: false, // SafeHarbor seed lots are real attested allocations — never pseudo
            })
            .collect();
        effective.push((d.id.clone(), seed, a.pre2025_method));
    }

    // (5) Irrevocability (§7.4(2)): a Void of an EFFECTIVE allocation → conflict (it stays in force); a
    //     Void of an inert allocation APPLIES → the allocation is RETIRED.
    for v in &allocation_voids {
        if effective.iter().any(|(id, _, _)| id == &v.target) {
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: Some(v.void_id.clone()),
                detail: "void targets an effective SafeHarborAllocation (irrevocable, §7.4)".into(),
            });
        } else {
            // The void APPLIES → the inert allocation is RETIRED (Path A already governs). RETRACT the Hard
            // `SafeHarborUnconservable` the backstop pushed for it (T16 review r2 / I-1): a retired
            // allocation is gone, so a Hard left on it — e.g. from the tranche-residue arm on the SUPPORTED
            // void-inert-then-declare flow, or a totals-mismatch after a later pre-2025 disposal re-keys the
            // FIFO draw — would brick every year with NO clearing move (the void cannot be re-issued).
            // Retracting is independent of WHICH inert reason produced the Hard, so there is no blind spot.
            // ONLY the Hard `SafeHarborUnconservable` is retracted; the Advisory `SafeHarborTimebar` is
            // deliberately LEFT as a stale advisory (existing behavior, `verify_report.rs:161`) — it is
            // non-blocking, so it never bricks a year. The tranche's tag survives via Path A; a NON-voided
            // allocation keeps its Hard (the D-8 deny-effectiveness guarantee is untouched — still pinned
            // by the Task-5 backstop KATs).
            blockers.retain(|b| {
                !(b.event.as_ref() == Some(&v.target)
                    && b.kind == BlockerKind::SafeHarborUnconservable)
            });
        }
    }

    // Multiple effective allocations → conflict; exactly one governs Path B; none → Path A default.
    let transition = match effective.len() {
        0 => TransitionMode::PathA,
        1 => {
            let (id, seed, recorded_method) = effective.into_iter().next().expect("len == 1");
            // §A.7.3 (M2): the conflict is "live config != the GOVERNING allocation's recorded method",
            // emitted ONCE, only for the single effective allocation. Conservation already passed (the
            // snapshot used `recorded_method`), so this is NEVER `SafeHarborUnconservable`; Path B stays
            // effective — the irrevocable allocation (§7.4) pins the method and is never rewritten. Clears
            // by reverting the live config to the recorded method (no deadlock with irrevocability).
            if config.pre2025_method != recorded_method {
                blockers.push(Blocker {
                    kind: BlockerKind::Pre2025MethodConflictsAllocation,
                    event: Some(id),
                    detail: format!(
                        "live pre2025_method ({:?}) differs from this allocation's recorded method ({:?}); revert the config to the recorded method (the irrevocable allocation pins it, §7.4)",
                        config.pre2025_method, recorded_method
                    ),
                });
            }
            TransitionMode::PathB { seed }
        }
        _ => {
            // Multiple effective: already hard-blocked by DecisionConflict → Path A. Do NOT evaluate the
            // method-conflict here (M2: it must fire only in the single-effective arm, never spuriously).
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: None,
                detail: "multiple effective SafeHarborAllocations".into(),
            });
            TransitionMode::PathA
        }
    };

    Resolution {
        timeline,
        transition,
        blockers,
        elections,
        selections,
        pseudo_decisions,
        promotes,
    }
}

/// Earlier-of two optional tax-dates (`None` = "this prong is absent").
fn min_opt(a: Option<TaxDate>, b: Option<TaxDate>) -> Option<TaxDate> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, None) | (None, x) => x,
    }
}
/// Later-of two optional tax-dates (`None` = "this prong is absent").
fn max_opt(a: Option<TaxDate>, b: Option<TaxDate>) -> Option<TaxDate> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (x, None) | (None, x) => x,
    }
}
/// §7.4: which 2025 ops count toward the "first-2025-disposition" deadline prong. Confirmed (c)
/// self-transfers and provisional `PendingOut` do NOT count; under TP8 (b) a self-transfer fee-sat
/// mini-disposition does.
fn is_disposition_op(op: &Op, config: &ProjectionConfig) -> bool {
    match op {
        Op::Dispose { .. } | Op::GiftOut { .. } | Op::Donate { .. } => true,
        Op::SelfTransfer { fee_sat, .. } => {
            config.self_transfer_fee == FeeTreatment::TreatmentB && fee_sat.unwrap_or(0) > 0
        }
        _ => false,
    }
}

/// §A.4: the principal sat of a method-honoring disposition — the ONLY events a `LotSelection` may
/// target. `PendingOut`, fee legs, and non-dispositions are not selectable (→ `LotSelectionInvalid`).
fn honoring_principal(op: &Op) -> Option<Sat> {
    match op {
        Op::Dispose { sat, .. }
        | Op::GiftOut { sat, .. }
        | Op::Donate { sat, .. }
        | Op::SelfTransfer { sat, .. } => Some(*sat),
        _ => None, // PendingOut, fee legs, non-disposals -> not selectable
    }
}

/// Canonical PASS-2 order (§6.2): utc_timestamp → source priority → source_ref.
pub fn sort_canonical(timeline: &mut [Eff]) {
    timeline.sort_by(|a, b| {
        a.utc
            .cmp(&b.utc)
            .then(a.src_priority.cmp(&b.src_priority))
            .then(a.src_ref.cmp(&b.src_ref))
            // ★ D-1a-b: final numeric tie-break on the EventId. Two same-window DeclareTranche Effs share
            //   (utc, src_priority=MAX, src_ref=""); EventId::Decision{seq} compares `seq` as u64, so they
            //   order 2-before-10 (NOT lexicographically). Import Effs never reach here (distinct src_ref).
            .then(a.id.cmp(&b.id))
    });
}
