use crate::conventions::{
    is_long_term, round_cents, split_pro_rata, Sat, TaxDate, Usd, TRANSITION_DATE,
};
use crate::event::{BasisSource, DisposeKind};
use crate::identity::{EventId, LotId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::pools::{pool_key, Consumed, PoolKey, PoolSet};
use crate::project::resolve::{sort_canonical, Eff, Op, Resolution};
use crate::project::transition;
use crate::state::{
    BlockerKind, Disposal, DisposalLeg, FoldStats, GiftZone, IncomeRecord, LedgerState, Lot,
    PendingLeg, PendingTransfer, Removal, RemovalKind, RemovalLeg, Term,
};
use crate::{FeeTreatment, ProjectionConfig};
use std::collections::BTreeMap;

/// TP4 term for a consumed fragment given the disposition date (gain side / no-dual uses gain_hp_start).
fn term_for(start: TaxDate, disposed: TaxDate) -> Term {
    if is_long_term(start, disposed) {
        Term::LongTerm
    } else {
        Term::ShortTerm
    }
}

/// §7.4: emit the pre-2025 FIFO disposal advisory ONCE (a Dispose/Removal consumed the Universal pool).
/// Pre-2025 ⇔ the disposition routed through `PoolKey::Universal` (see `pool_key`).
fn note_pre2025_once(st: &mut LedgerState, date: TaxDate, ev: &EventId) {
    if date < TRANSITION_DATE
        && !st
            .blockers
            .iter()
            .any(|b| b.kind == BlockerKind::Pre2025MethodNote)
    {
        st.add_blocker(
            BlockerKind::Pre2025MethodNote,
            Some(ev.clone()),
            "pre-2025 lots reconstructed under FIFO (the legal default, §7.4); if your filed pre-2025 returns used a different lot method, your carryforward basis may differ — verify against those filings",
        );
    }
}

/// Build disposal legs from consumed fragments and a TOTAL net proceeds amount, allocated pro-rata by sat
/// (remainder-takes-the-rest so Σproceeds is exact). Dual-basis gift logic (TP11) is added in Task 10;
/// here every leg is the simple `gift_zone = None` path.
fn make_disposal_legs(
    consumed: &[Consumed],
    total_net_proceeds: Usd,
    disposed: TaxDate,
    st: &mut LedgerState,
    ev: &EventId,
) -> Vec<DisposalLeg> {
    let total_sat: i64 = consumed.iter().map(|c| c.sat).sum();
    let mut legs = Vec::new();
    let mut allocated = Usd::ZERO;
    for (i, c) in consumed.iter().enumerate() {
        let proceeds = if i + 1 == consumed.len() {
            total_net_proceeds - allocated
        } else {
            let (p, _) = split_pro_rata(total_net_proceeds, c.sat, total_sat);
            allocated += p;
            p
        };
        if c.basis_pending {
            // FMV-missing income / unknown-basis gift in this lot's history → gate the gain (§7.3).
            st.add_blocker(
                BlockerKind::FmvMissing,
                Some(ev.clone()),
                "disposal consumes a basis-pending lot",
            );
        }
        // Task 10: four-zone §1015(a) dual-basis computation (TP11).
        // When `c.dual = false` (no dual basis): simple single-carryover path.
        // When `c.dual = true` (dual-basis gift, FMV-at-gift < donor-basis at gift date):
        //   Gain zone  : proceeds > gain_basis  → basis = gain_basis, term tacks (gain_hp_start).
        //   Loss zone  : proceeds < loss_basis  → basis = loss_basis, HP from gift date (loss_hp_start).
        //   NoGainNoLoss: otherwise             → reported basis = proceeds, gain = 0, term from gain_hp_start.
        // Note: in the NoGainNoLoss zone, `lot.usd_basis` was already reduced by pro-rata `gain_basis`
        // on consume (pools.rs), so Σbasis is conserved exactly even though we report basis = proceeds.
        let (basis, gain, term, gift_zone) = if c.dual {
            let loss_basis = c.loss_basis.expect("dual=true implies loss_basis is Some");
            if proceeds > c.gain_basis {
                // Gain zone: basis = gain_basis (tacks, gain_hp_start).
                let t = term_for(c.gain_hp_start, disposed);
                (
                    c.gain_basis,
                    round_cents(proceeds - c.gain_basis),
                    t,
                    Some(GiftZone::Gain),
                )
            } else if proceeds < loss_basis {
                // Loss zone: basis = FMV-at-gift (loss_basis), HP from gift date.
                let t = term_for(c.loss_hp_start, disposed);
                (
                    loss_basis,
                    round_cents(proceeds - loss_basis),
                    t,
                    Some(GiftZone::Loss),
                )
            } else {
                // NoGainNoLoss zone: reported basis = proceeds → gain = 0; term from gain_hp_start.
                let t = term_for(c.gain_hp_start, disposed);
                (proceeds, Usd::ZERO, t, Some(GiftZone::NoGainNoLoss))
            }
        } else {
            let basis = c.gain_basis;
            let t = term_for(c.gain_hp_start, disposed);
            (basis, round_cents(proceeds - basis), t, None)
        };
        legs.push(DisposalLeg {
            lot_id: c.lot_id.clone(),
            sat: c.sat,
            proceeds,
            basis,
            gain,
            term,
            basis_source: c.basis_source,
            gift_zone,
        });
    }
    legs
}

/// Build removal legs from consumed fragments and a TOTAL FMV amount, allocated pro-rata by sat
/// (remainder-takes-the-rest so Σfmv is exact). Zero recognized gain (TP10): no Disposal emitted.
/// Returns (legs, donor_acquired_at) where donor_acquired_at is the first non-None across lots.
fn make_removal_legs(
    consumed: &[Consumed],
    total_fmv: Usd,
    removed: TaxDate,
    st: &mut LedgerState,
    ev: &EventId,
) -> (Vec<RemovalLeg>, Option<TaxDate>) {
    let total_sat: i64 = consumed.iter().map(|c| c.sat).sum();
    let mut legs = Vec::new();
    let mut allocated = Usd::ZERO;
    let mut donor = None;
    for (i, c) in consumed.iter().enumerate() {
        if c.basis_pending {
            st.add_blocker(
                BlockerKind::UnknownBasisInbound,
                Some(ev.clone()),
                "removal consumes a basis-pending lot",
            );
        }
        let fmv = if i + 1 == consumed.len() {
            total_fmv - allocated
        } else {
            let (f, _) = split_pro_rata(total_fmv, c.sat, total_sat);
            allocated += f;
            f
        };
        donor = donor.or(c.donor_acquired_at);
        legs.push(RemovalLeg {
            lot_id: c.lot_id.clone(),
            sat: c.sat,
            basis: c.gain_basis,
            fmv_at_transfer: fmv,
            term: term_for(c.gain_hp_start, removed),
            basis_source: c.basis_source,
        });
    }
    (legs, donor)
}

/// Carried basis of the burned fee-sats, to be RE-HOMED onto a surviving destination lot / removal leg
/// under TP8 (c) so the FULL basis carries (C1). Under (b) this is empty (basis rode the mini-disposition).
#[derive(Default)]
struct FeeCarry {
    gain_basis: Usd,
    loss_basis: Option<Usd>,
}

impl FeeCarry {
    /// Re-home the fee-sat basis onto the surviving destination lot (C1: full basis carries).
    /// `gain_basis` always carries onto `lot.usd_basis` (C1 invariant; must not be dropped).
    /// `loss_basis` carries onto `lot.dual_loss_basis` ONLY when the survivor is ALREADY a
    /// dual-basis lot (`Some(existing)` → add to existing). When the survivor is non-dual
    /// (`None`), the `loss_basis` fragment is dropped instead of promoting the lot to `Some`:
    /// promoting would set `dual_loss_basis.is_some() == true`, causing a later disposition to
    /// route through the §1015(a) four-zone logic (`make_disposal_legs` keys on this field)
    /// and misclassify a normal purchased/transferred lot as a received-gift dual-basis lot —
    /// a worse error than the cents-scale conservative loss-basis understatement that results
    /// from the drop. Conservative: future loss-zone basis understated by fee-cents; gain basis
    /// fully conserved (C1 intact).
    fn rehome_onto_lot(&self, lot: &mut Lot) {
        lot.usd_basis += self.gain_basis;
        if let Some(l) = self.loss_basis {
            // Add to existing dual_loss_basis only; when None (non-dual survivor) the fragment
            // is dropped — promoting None → Some would misroute a later disposition through the
            // §1015(a) four-zone logic (see doc comment above for full rationale).
            if let Some(dl) = lot.dual_loss_basis.as_mut() {
                *dl += l;
            }
        }
    }

    /// Re-home the fee-sat gain basis onto the last removal leg (C1: full basis carries to donee).
    /// Note: `loss_basis` is a donor's private tax attribute and does not carry onto removal legs.
    fn rehome_onto_removal_leg(&self, leg: &mut RemovalLeg) {
        leg.basis += self.gain_basis;
    }

    /// Re-home the fee-sat gain basis onto the last disposal leg (I-1: Dispose+fee_sat, TP8 (c)).
    /// Under (c): adds gain_basis to the reported leg basis → gain decreases by carry amount.
    /// Under (b): carry is empty (gain_basis = 0) so this is a no-op; the fee-sat basis rode the
    /// mini-disposition emitted by consume_fee instead.
    fn rehome_onto_disposal_leg(&self, leg: &mut DisposalLeg) {
        leg.basis += self.gain_basis;
        leg.gain = round_cents(leg.proceeds - leg.basis);
    }
}

/// Consume `fee_sat` FIFO from the source pool, record them in the FR9 fee-sat home, and (per config)
/// either return their carried basis for re-homing (c) or emit a mini-disposition recognition record (b).
/// §7.1 totality: a fee shortfall raises `uncovered_disposal`, never panics.
#[allow(clippy::too_many_arguments)]
fn consume_fee(
    pools: &mut PoolSet,
    key: &PoolKey,
    fee_sat: Sat,
    config: &ProjectionConfig,
    prices: &dyn PriceProvider,
    date: TaxDate,
    stats: &mut FoldStats,
    st: &mut LedgerState,
    ev: &EventId,
) -> FeeCarry {
    if fee_sat <= 0 {
        return FeeCarry::default();
    }
    let (consumed, shortfall) = pools.consume_fifo(key, fee_sat);
    if shortfall > 0 {
        st.add_blocker(
            BlockerKind::UncoveredDisposal,
            Some(ev.clone()),
            format!("self-transfer/gift fee short by {shortfall} sat"),
        );
    }
    stats.fee_sats_consumed += consumed.iter().map(|c| c.sat).sum::<Sat>(); // sole FR9 home
    match config.self_transfer_fee {
        FeeTreatment::TreatmentC => {
            // Non-taxable: return the fee-sat basis for re-homing onto the survivor (C1: full basis carries).
            let gain_basis: Usd = consumed.iter().map(|c| c.gain_basis).sum();
            let has_loss = consumed.iter().any(|c| c.loss_basis.is_some());
            let loss_basis = has_loss.then(|| consumed.iter().filter_map(|c| c.loss_basis).sum());
            FeeCarry {
                gain_basis,
                loss_basis,
            }
        }
        FeeTreatment::TreatmentB => {
            // mini-disposition recognition record; proceeds = FMV(fee_sat); basis rides it (NOT re-homed).
            if !consumed.is_empty() {
                let net = fmv_of(prices, date, fee_sat).unwrap_or(Usd::ZERO);
                let legs = make_disposal_legs(&consumed, net, date, st, ev);
                st.disposals.push(Disposal {
                    event: ev.clone(),
                    kind: DisposeKind::Spend,
                    disposed_at: date,
                    legs,
                    fee_mini_disposition: true,
                });
            }
            FeeCarry::default()
        }
    }
}

pub fn fold(
    mut res: Resolution,
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
) -> LedgerState {
    sort_canonical(&mut res.timeline);
    // Eng-review Minor (§7.4): the boundary seed must fire on the TAX-DATE, not raw UTC order. A STABLE
    // partition by tax-date side (pre-2025 first) means a sub-day offset straddling 2025-01-01 (e.g. a
    // +12:00 post-2025 event with an earlier UTC than a −05:00 pre-2025 event) folds on the correct side
    // of the one-shot seed, and the pre-seed Universal residue matches `transition::universal_snapshot`
    // exactly (I-1). `sort_by_key` is stable, so canonical FIFO order is preserved within each side.
    res.timeline.sort_by_key(|e| e.date() >= TRANSITION_DATE);
    let mut st = LedgerState {
        blockers: res.blockers,
        ..Default::default()
    };
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default(); // M3/FR9: fee_sats_consumed (Task 11), sigma_in here
    let mut seeded = false;

    for eff in &res.timeline {
        if !seeded && eff.date() >= TRANSITION_DATE {
            // Path A drain / Path B seed of the per-wallet pools from the Universal residue, ONCE (§7.4).
            transition::seed_transition(&res.transition, &mut pools, &mut st);
            seeded = true;
        }
        fold_event(eff, prices, config, &mut pools, &mut st, &mut stats);
    }

    finalize(&mut st, pools, stats); // if no ≥2025 event ever seeds, Universal lots remain (carry their wallet)
    st
}

/// PASS-2 per-event dispatcher. Lifted out of `fold` so that BOTH the real fold and the pass-1
/// `transition::universal_snapshot` pre-fold run the IDENTICAL per-event arms — the conservation guard's
/// pre-2025 residue therefore provably matches the real fold's pre-seed residue (I-1). Pure: mutates only
/// the passed pools/state/stats.
pub(crate) fn fold_event(
    eff: &Eff,
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    pools: &mut PoolSet,
    st: &mut LedgerState,
    stats: &mut FoldStats,
) {
    let date = eff.date();
    match &eff.op {
        Op::Acquire(a) => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::Unclassified,
                        Some(eff.id.clone()),
                        "acquire without wallet",
                    );
                    return;
                }
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: a.sat,
                remaining_sat: a.sat,
                usd_basis: a.usd_cost + a.fee_usd, // TP2: basis = cost + acquisition fee
                basis_source: a.basis_source,
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: false,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += a.sat; // FR9 Σin: externally-sourced acquisition
        }
        Op::Dispose {
            sat,
            proceeds,
            fee_usd,
            fee_sat,
            kind,
        } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "dispose without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            note_pre2025_once(st, date, &eff.id); // §7.4: pre-2025 disposal advisory (once)
            let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("dispose short by {shortfall} sat"),
                );
            }
            if !consumed.is_empty() {
                let net = round_cents(*proceeds - *fee_usd); // TP2: disposition fee reduces proceeds
                let mut legs = make_disposal_legs(&consumed, net, date, st, &eff.id);
                // I-1: Task 11 fee step — consume fee_sat FIFO from source pool AFTER principal.
                // Mirrors the gift/SelfTransfer pattern; native Dispose passes fee_sat=None (no-op).
                // (c) default: re-home carry onto last disposal leg; fee-sat basis rolls into the
                //     disposition (reported basis increases → gain decreases); fee non-taxable.
                // (b) config:  emits mini-disposition; returns empty carry; leg basis unchanged.
                let carry = consume_fee(
                    pools,
                    &key,
                    fee_sat.unwrap_or(0),
                    config,
                    prices,
                    date,
                    stats,
                    st,
                    &eff.id,
                );
                if let Some(last) = legs.last_mut() {
                    carry.rehome_onto_disposal_leg(last);
                } else if carry.gain_basis > Usd::ZERO {
                    // m3: degenerate guard — no surviving leg (principal == 0); unreachable for real events.
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "fee carry has no surviving disposal leg to re-home onto (principal == 0)",
                    );
                }
                st.disposals.push(Disposal {
                    event: eff.id.clone(),
                    kind: *kind,
                    disposed_at: date,
                    legs,
                    fee_mini_disposition: false,
                });
            }
        }
        Op::Income {
            sat,
            fmv,
            kind,
            business,
        } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income without wallet",
                    );
                    return;
                }
            };
            let (basis, pending) = match fmv {
                Some(v) => {
                    st.income_recognized.push(IncomeRecord {
                        event: eff.id.clone(),
                        recognized_at: date,
                        sat: *sat,
                        usd_fmv: *v,
                        kind: *kind,
                        business: *business,
                    });
                    (*v, false)
                }
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income FMV missing",
                    );
                    (Usd::ZERO, true) // basis pending; lot still created so Σsat conservation holds (§7.3)
                }
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis: basis,
                basis_source: BasisSource::FmvAtIncome,
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: pending,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat; // FR9 Σin: income is externally-sourced (counts even while FMV is pending)
        }
        Op::PendingOut { sat, fee_sat } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "pending out without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            let total_sat = *sat + fee_sat.unwrap_or(0);
            let (consumed, shortfall) = pools.consume_fifo(&key, total_sat);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("pending out short by {shortfall} sat"),
                );
            }
            let legs: Vec<PendingLeg> = consumed
                .iter()
                .map(|c| PendingLeg {
                    lot_id: c.lot_id.clone(),
                    sat: c.sat,
                    usd_basis: c.gain_basis,
                    acquired_at: c.acquired_at,
                })
                .collect();
            st.pending_reconciliation.push(PendingTransfer {
                event: eff.id.clone(),
                principal_sat: *sat,
                fee_sat: *fee_sat,
                legs,
            });
            // Advisory blocker: unmatched outflow (may be resolved by a later TransferLink in Task 8+).
            st.add_blocker(
                BlockerKind::UnmatchedOutflows,
                Some(eff.id.clone()),
                "unmatched transfer out",
            );
        }
        Op::SelfTransfer { sat, fee_sat, dest } => {
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "self transfer without source wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("self transfer short by {shortfall} sat"),
                );
            }
            // Relocate consumed fragments to the destination pool: carry basis, HP, donor_acquired_at.
            // Non-taxable (TP7): no Disposal or Removal records. basis_source = CarriedFromTransfer.
            let mut relocated: Vec<Lot> = Vec::new();
            for c in &consumed {
                let seq = pools.bump_split(&c.lot_id.origin_event_id);
                relocated.push(Lot {
                    lot_id: LotId {
                        origin_event_id: c.lot_id.origin_event_id.clone(),
                        split_sequence: seq,
                    },
                    wallet: dest.clone(),
                    acquired_at: c.acquired_at,
                    original_sat: c.sat,
                    remaining_sat: c.sat,
                    usd_basis: c.gain_basis,
                    basis_source: BasisSource::CarriedFromTransfer,
                    dual_loss_basis: c.loss_basis,
                    donor_acquired_at: c.donor_acquired_at,
                    basis_pending: c.basis_pending,
                });
            }
            // Task 11: fee handling — consume fee_sat FIFO from source pool AFTER principal (FIFO order).
            // (c) default: returns FeeCarry to re-home onto relocated.last(), so FULL basis carries (C1).
            // (b) config:  emits mini-disposition; returns empty carry; destination lot stays at principal basis.
            let carry = consume_fee(
                pools,
                &key,
                fee_sat.unwrap_or(0),
                config,
                prices,
                date,
                stats,
                st,
                &eff.id,
            );
            if let Some(last) = relocated.last_mut() {
                carry.rehome_onto_lot(last);
            } else if carry.gain_basis > Usd::ZERO {
                // m3: degenerate guard — no surviving lot to re-home onto (principal == 0).
                // Unreachable for a real TransferLink (always moves principal > 0), but never silent.
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    "fee carry has no surviving lot to re-home onto (principal == 0)",
                );
            }
            let dest_key = pool_key(date, dest);
            for lot in relocated {
                pools.push_lot(dest_key.clone(), lot);
            }
        }
        Op::UnknownInbound { sat: _ } => {
            // Hard blocker: basis is unknown. NO lot — sats not yet in the ledger (FR9/§7.3).
            st.add_blocker(
                BlockerKind::UnknownBasisInbound,
                Some(eff.id.clone()),
                "unclassified TransferIn — basis unknown",
            );
        }
        Op::IncomeInbound {
            sat,
            fmv,
            kind,
            business,
        } => {
            // Identical to Op::Income: income lot at FMV + IncomeRecord. sigma_in += sat.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income inbound without wallet",
                    );
                    return;
                }
            };
            let (basis, pending) = match fmv {
                Some(v) => {
                    st.income_recognized.push(IncomeRecord {
                        event: eff.id.clone(),
                        recognized_at: date,
                        sat: *sat,
                        usd_fmv: *v,
                        kind: *kind,
                        business: *business,
                    });
                    (*v, false)
                }
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "income inbound FMV missing",
                    );
                    (Usd::ZERO, true)
                }
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis: basis,
                basis_source: BasisSource::FmvAtIncome,
                dual_loss_basis: None,
                donor_acquired_at: None,
                basis_pending: pending,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat;
        }
        Op::GiftReceived {
            sat,
            donor_basis,
            donor_acquired_at,
            fmv_at_gift,
        } => {
            // Task 10: §1015(a) dual-basis lot construction (TP11).
            // Four cases by (donor_basis, donor_acquired_at) × (fmv_at_gift vs donor_basis):
            //   1. donor_basis=Some(b), fmv_at_gift >= b  → single carryover (Gain zone only); tacks.
            //   2. donor_basis=Some(b), fmv_at_gift < b   → dual basis; tacks on gain side.
            //   3. donor_basis=None, donor_acquired_at=Some(d) → GiftFmvFallback: look up price at d.
            //   4. donor_basis=None, donor_acquired_at=None    → basis unknown; hard blocker + pending lot.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::FmvMissing,
                        Some(eff.id.clone()),
                        "gift received without wallet",
                    );
                    return;
                }
            };
            let (usd_basis, dual_loss_basis, basis_source, pending) = match donor_basis {
                Some(b) => {
                    if *fmv_at_gift >= *b {
                        // Case 1: FMV ≥ donor basis — single carryover; no dual.
                        (*b, None, BasisSource::GiftCarryover, false)
                    } else {
                        // Case 2: FMV < donor basis — dual: gain basis = donor basis, loss basis = FMV.
                        (*b, Some(*fmv_at_gift), BasisSource::GiftCarryover, false)
                    }
                }
                None => match donor_acquired_at {
                    Some(d) => {
                        // Case 3: GiftFmvFallback — derive basis from BTC price at donor's acquisition date.
                        match fmv_of(prices, *d, *sat) {
                            Some(fmv) => (fmv, None, BasisSource::GiftFmvFallback, false),
                            None => {
                                // Price unavailable at donor acquisition date → basis indeterminate.
                                st.add_blocker(
                                    BlockerKind::UnknownBasisInbound,
                                    Some(eff.id.clone()),
                                    "gift received: donor basis unknown and price unavailable at donor acquisition date",
                                );
                                (Usd::ZERO, None, BasisSource::GiftFmvFallback, true)
                            }
                        }
                    }
                    None => {
                        // Case 4: both donor basis and acquisition date unknown — hard blocker.
                        st.add_blocker(
                            BlockerKind::UnknownBasisInbound,
                            Some(eff.id.clone()),
                            "gift received: donor basis and acquisition date both unknown",
                        );
                        (Usd::ZERO, None, BasisSource::GiftCarryover, true)
                    }
                },
            };
            let lot = Lot {
                lot_id: LotId {
                    origin_event_id: eff.id.clone(),
                    split_sequence: 0,
                },
                wallet: wallet.clone(),
                acquired_at: date,
                original_sat: *sat,
                remaining_sat: *sat,
                usd_basis,
                basis_source,
                dual_loss_basis,
                donor_acquired_at: *donor_acquired_at,
                basis_pending: pending,
            };
            pools.new_origin_lot(pool_key(date, &wallet), lot);
            stats.sigma_in += *sat; // classified GiftReceived is externally-sourced (FR9)
        }
        Op::GiftOut {
            sat, fmv, fee_sat, ..
        } => {
            // TP10: gift outbound → Removal with zero recognized gain; no Disposal.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "gift out without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            note_pre2025_once(st, date, &eff.id); // §7.4: pre-2025 removal advisory (once)
            let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("gift out short by {shortfall} sat"),
                );
            }
            if !consumed.is_empty() {
                let (mut legs, donor_acquired_at) =
                    make_removal_legs(&consumed, *fmv, date, st, &eff.id);
                // Task 11: fee step — consume fee_sat FIFO from source pool AFTER principal.
                // (c) default: re-home carry onto last removal leg so donee carries FULL basis (C1).
                // (b) config:  emits mini-disposition; empty carry; donee gets principal-only basis.
                let carry = consume_fee(
                    pools,
                    &key,
                    fee_sat.unwrap_or(0),
                    config,
                    prices,
                    date,
                    stats,
                    st,
                    &eff.id,
                );
                if let Some(last) = legs.last_mut() {
                    carry.rehome_onto_removal_leg(last);
                } else if carry.gain_basis > Usd::ZERO {
                    // m3: degenerate guard (unreachable for real gifts, which always move principal > 0).
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "fee carry has no surviving removal leg to re-home onto (principal == 0)",
                    );
                }
                st.removals.push(Removal {
                    event: eff.id.clone(),
                    kind: RemovalKind::Gift,
                    removed_at: date,
                    legs,
                    appraisal_required: false,
                    donor_acquired_at,
                });
            }
        }
        Op::Donate {
            sat,
            fmv,
            appraisal_required,
            fee_sat,
            ..
        } => {
            // TP10: donation outbound → Removal with zero recognized gain; no Disposal.
            let wallet = match &eff.wallet {
                Some(w) => w.clone(),
                None => {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "donate without wallet",
                    );
                    return;
                }
            };
            let key = pool_key(date, &wallet);
            note_pre2025_once(st, date, &eff.id); // §7.4: pre-2025 removal advisory (once)
            let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
            if shortfall > 0 {
                st.add_blocker(
                    BlockerKind::UncoveredDisposal,
                    Some(eff.id.clone()),
                    format!("donate short by {shortfall} sat"),
                );
            }
            if !consumed.is_empty() {
                let (mut legs, donor_acquired_at) =
                    make_removal_legs(&consumed, *fmv, date, st, &eff.id);
                // Task 11: fee step — consume fee_sat FIFO from source pool AFTER principal.
                // (c) default: re-home carry onto last removal leg so donee carries FULL basis (C1).
                // (b) config:  emits mini-disposition; empty carry; donee gets principal-only basis.
                let carry = consume_fee(
                    pools,
                    &key,
                    fee_sat.unwrap_or(0),
                    config,
                    prices,
                    date,
                    stats,
                    st,
                    &eff.id,
                );
                if let Some(last) = legs.last_mut() {
                    carry.rehome_onto_removal_leg(last);
                } else if carry.gain_basis > Usd::ZERO {
                    // m3: degenerate guard (unreachable for real donations, which always move principal > 0).
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        "fee carry has no surviving removal leg to re-home onto (principal == 0)",
                    );
                }
                st.removals.push(Removal {
                    event: eff.id.clone(),
                    kind: RemovalKind::Donation,
                    removed_at: date,
                    legs,
                    appraisal_required: *appraisal_required,
                    donor_acquired_at,
                });
            }
        }
        Op::Unclassified => {
            st.add_blocker(
                BlockerKind::Unclassified,
                Some(eff.id.clone()),
                "unclassified BTC-side row",
            );
        }
        Op::Skip => {}
    }
}

/// Collect remaining lots + holdings; sort all output deterministically (NFR4); commit the FoldStats (M3).
pub fn finalize(st: &mut LedgerState, pools: PoolSet, mut stats: FoldStats) {
    let mut holdings: BTreeMap<crate::identity::WalletId, Sat> = BTreeMap::new();
    let mut lots: Vec<Lot> = Vec::new();
    for (_key, pool) in pools.pools {
        for lot in pool {
            if lot.remaining_sat > 0 {
                *holdings.entry(lot.wallet.clone()).or_insert(0) += lot.remaining_sat;
                lots.push(lot);
            }
        }
    }
    lots.sort_by(|a, b| {
        a.wallet
            .cmp(&b.wallet)
            .then(a.acquired_at.cmp(&b.acquired_at))
            .then(a.lot_id.cmp(&b.lot_id))
    });
    st.lots = lots;
    st.holdings_by_wallet = holdings;
    // M1: sort blockers by the DERIVED Ord of (kind, Option<EventId>, detail) — a total order, no Debug strings.
    st.blockers.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.event.cmp(&b.event))
            .then_with(|| a.detail.cmp(&b.detail))
    });
    // Σpending is reconstructable from the queue; sigma_in/fee_sats_consumed are accumulated during the fold.
    stats.sigma_pending = st
        .pending_reconciliation
        .iter()
        .map(|p| p.principal_sat + p.fee_sat.unwrap_or(0))
        .sum();
    st.stats = stats;
}
