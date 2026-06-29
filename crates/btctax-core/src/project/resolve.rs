use crate::conventions::{tax_date, Sat, TaxDate, Usd, TRANSITION_DATE, TY2025_RETURN_DUE};
use crate::event::*;
use crate::identity::{EventId, LotId, SourceRef, WalletId};
use crate::price::PriceProvider;
use crate::project::{FeeTreatment, ProjectionConfig};
use crate::state::{Blocker, BlockerKind, Lot};
use std::collections::{BTreeMap, BTreeSet};
use time::{OffsetDateTime, UtcOffset};

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
    // Task 9: gift/donation outbound (TP10) and reclassified disposal.
    /// ReclassifyOutflow{GiftOut}: Removal with zero recognized gain; per-lot basis + FMV + ST/LT.
    GiftOut {
        sat: Sat,
        fmv: Usd,
        fee_sat: Option<Sat>, // Task 11: on-chain fee consumed per TP8 (c)
        fee_usd: Option<Usd>, // Task 11: USD fee from ReclassifyOutflow
    },
    /// ReclassifyOutflow{Donate}: Removal with zero recognized gain + appraisal_required flag.
    Donate {
        sat: Sat,
        fmv: Usd,
        appraisal_required: bool,
        fee_sat: Option<Sat>, // Task 11: on-chain fee consumed per TP8 (c)
        fee_usd: Option<Usd>, // Task 11: USD fee from ReclassifyOutflow
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

pub struct Resolution {
    pub timeline: Vec<Eff>,
    pub transition: TransitionMode,
    pub blockers: Vec<Blocker>,
}

/// Private outcome of resolving an `ImportConflict` via a decision.
enum Resolved {
    /// The new payload is accepted; inner `EventId` is the original import target.
    Accept(EventPayload, EventId),
    /// The conflict is rejected; the original import stands unchanged.
    Reject,
}

/// Map an effective imported payload → `Op`, applying any `ManualFmv` override and Task 8/9 classification maps.
/// ManualFmv on an `Income` replaces the FMV and clears the would-be `fmv_missing` gate.
#[allow(clippy::too_many_arguments)]
fn build_op(
    id: &EventId,
    payload: &EventPayload,
    manual_fmv: &BTreeMap<EventId, Usd>,
    links: &BTreeMap<EventId, TransferTarget>,
    consumed_ins: &BTreeSet<EventId>,
    inbound_class: &BTreeMap<EventId, InboundClass>,
    outflow_class: &BTreeMap<EventId, ReclassifyOutflow>,
    by_id: &BTreeMap<EventId, &LedgerEvent>,
) -> Op {
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
            Op::Income {
                sat: x.sat,
                fmv,
                kind: x.kind,
                business: x.business,
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
                    },
                    OutflowClass::Donate { appraisal_required } => Op::Donate {
                        sat: t.sat,
                        fmv: ro.principal_proceeds_or_fmv,
                        appraisal_required: *appraisal_required,
                        fee_sat: t.fee_sat,
                        fee_usd: ro.fee_usd,
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
                }
            } else {
                Op::UnknownInbound { sat: t.sat }
            }
        }
        EventPayload::Unclassified(_) => Op::Unclassified,
        _ => Op::Skip,
    }
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

    // ── 1a. Collect Voids; classify each target's revocability ──────────────────────────────────
    //   Revocable targets: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw.
    //   NON-revocable targets: SupersedeImport, RejectImport, VoidDecisionEvent.
    //   Void of a non-revocable target → DecisionConflict (target stays in force; void is inert).
    //   Void of SafeHarborAllocation → collected in allocation_voids (deferred to Task 12).
    let mut voided: BTreeSet<EventId> = BTreeSet::new();
    let mut allocation_voids: Vec<AllocationVoid> = Vec::new();
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
                        detail: "void targets a non-revocable decision".into(),
                    });
                }
                Some(EventPayload::SafeHarborAllocation(_)) => {
                    // Deferred to Task 12 (effective allocation → conflict; inert → apply void).
                    allocation_voids.push(AllocationVoid {
                        void_id: e.id.clone(),
                        target: v.target_event_id.clone(),
                    });
                }
                Some(_) => {
                    voided.insert(v.target_event_id.clone());
                }
                None => {
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(e.id.clone()),
                        detail: "void targets unknown event".into(),
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
                        Resolved::Accept((*c.new_payload).clone(), c.target.clone()),
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
        if let EventPayload::ImportConflict(_) = &e.payload {
            match conflict_res.get(&e.id) {
                Some(Resolved::Accept(payload, target)) => {
                    applied.insert(target.clone(), payload.clone());
                }
                Some(Resolved::Reject) => {} // original import stands unchanged
                None => blockers.push(Blocker {
                    kind: BlockerKind::ImportConflict,
                    event: Some(e.id.clone()),
                    detail: "unresolved import conflict".into(),
                }),
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
                    detail: "multiple classifications of one target".into(),
                });
            } else {
                applied.insert(cr.target.clone(), (*cr.as_).clone());
            }
        }
    }

    // ── 1d. ManualFmv ───────────────────────────────────────────────────────────────────────────
    // Collect event_id → usd_fmv; latest decision_seq wins (ascending iteration = last write wins).
    let mut manual_fmv: BTreeMap<EventId, Usd> = BTreeMap::new();
    for (_seq, d) in &decisions {
        if voided.contains(&d.id) {
            continue;
        }
        if let EventPayload::ManualFmv(m) = &d.payload {
            manual_fmv.insert(m.event.clone(), m.usd_fmv);
        }
    }

    // ── 1e. Classification decisions ────────────────────────────────────────────────────────────
    // TransferLink → links + consumed_ins; ClassifyInbound → inbound_class; ReclassifyOutflow → outflow_class.
    // Contradiction detection (same-target multi-class) for all three.
    let mut links: BTreeMap<EventId, TransferTarget> = BTreeMap::new();
    let mut consumed_ins: BTreeSet<EventId> = BTreeSet::new();
    let mut inbound_class: BTreeMap<EventId, InboundClass> = BTreeMap::new();
    let mut outflow_class: BTreeMap<EventId, ReclassifyOutflow> = BTreeMap::new();

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
                if inbound_class.contains_key(&ci.transfer_in_event) {
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: "duplicate ClassifyInbound for the same TransferIn event".into(),
                    });
                } else {
                    inbound_class.insert(ci.transfer_in_event.clone(), ci.as_.clone());
                }
            }
            EventPayload::ReclassifyOutflow(ro) => {
                if outflow_class.contains_key(&ro.transfer_out_event) {
                    blockers.push(Blocker {
                        kind: BlockerKind::DecisionConflict,
                        event: Some(d.id.clone()),
                        detail: "duplicate ReclassifyOutflow for the same TransferOut event".into(),
                    });
                } else {
                    outflow_class.insert(ro.transfer_out_event.clone(), ro.clone());
                }
            }
            _ => {}
        }
    }

    // ── 2. Build the effective imported timeline ─────────────────────────────────────────────────
    // For each imported event, apply `applied` overrides then `manual_fmv`, emit an `Eff`.
    // Unclassified with no ClassifyRaw → Op::Unclassified (blocker added in fold).
    // Non-import events (decisions, conflicts) are skipped — they have no timeline entry.
    let mut timeline = Vec::new();
    for e in events {
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
        });
    }

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

    // (3-prereq) The pre-2025 Universal residue is allocation-INDEPENDENT — compute it ONCE.
    let snap = crate::project::transition::universal_snapshot(&timeline, prices, config);

    let due = TY2025_RETURN_DUE;
    let mut effective: Vec<(EventId, Vec<Lot>)> = Vec::new(); // (allocation id, pre-built seed lots)
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
        let timebarred = (!a.timely_allocation_attested && bar.is_some_and(|b| made > b))
            || (a.method == AllocMethod::ProRata && !a.timely_allocation_attested);

        // (3) Conservation vs the pre-2025 Universal snapshot. HARD on failure; attestation cannot bypass it.
        let alloc_sat: Sat = a.lots.iter().map(|l| l.sat).sum();
        let alloc_basis: Usd = a.lots.iter().map(|l| l.usd_basis).sum();
        let unconservable = alloc_sat != snap.held_sat || alloc_basis != snap.basis;
        if unconservable {
            blockers.push(Blocker {
                kind: BlockerKind::SafeHarborUnconservable,
                event: Some(d.id.clone()),
                detail: "allocation totals != Universal remainder at 2025-01-01".into(),
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
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: false,
            })
            .collect();
        effective.push((d.id.clone(), seed));
    }

    // (5) Irrevocability (§7.4(2)): a Void of an EFFECTIVE allocation → conflict (it stays in force); a
    //     Void of an inert/absent allocation simply applies (no conflict; Path A already governs).
    for v in &allocation_voids {
        if effective.iter().any(|(id, _)| id == &v.target) {
            blockers.push(Blocker {
                kind: BlockerKind::DecisionConflict,
                event: Some(v.void_id.clone()),
                detail: "void targets an effective SafeHarborAllocation (irrevocable, §7.4)".into(),
            });
        }
    }

    // Multiple effective allocations → conflict; exactly one governs Path B; none → Path A default.
    let transition = match effective.len() {
        0 => TransitionMode::PathA,
        1 => TransitionMode::PathB {
            seed: effective.into_iter().next().expect("len == 1").1,
        },
        _ => {
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

/// Canonical PASS-2 order (§6.2): utc_timestamp → source priority → source_ref.
pub fn sort_canonical(timeline: &mut [Eff]) {
    timeline.sort_by(|a, b| {
        a.utc
            .cmp(&b.utc)
            .then(a.src_priority.cmp(&b.src_priority))
            .then(a.src_ref.cmp(&b.src_ref))
    });
}
