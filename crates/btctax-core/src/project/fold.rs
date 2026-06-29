use crate::conventions::{is_long_term, round_cents, split_pro_rata, Sat, TaxDate, Usd};
use crate::event::BasisSource;
use crate::identity::{EventId, LotId};
use crate::price::{fmv_of, PriceProvider};
use crate::project::pools::{pool_key, Consumed, PoolSet};
use crate::project::resolve::{sort_canonical, Op, Resolution};
use crate::state::{
    BlockerKind, Disposal, DisposalLeg, FoldStats, GiftZone, IncomeRecord, LedgerState, Lot,
    PendingLeg, PendingTransfer, Removal, RemovalKind, RemovalLeg, Term,
};
use crate::ProjectionConfig;
use std::collections::BTreeMap;

/// TP4 term for a consumed fragment given the disposition date (gain side / no-dual uses gain_hp_start).
fn term_for(start: TaxDate, disposed: TaxDate) -> Term {
    if is_long_term(start, disposed) {
        Term::LongTerm
    } else {
        Term::ShortTerm
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

pub fn fold(
    mut res: Resolution,
    prices: &dyn PriceProvider,
    _config: &ProjectionConfig,
) -> LedgerState {
    sort_canonical(&mut res.timeline);
    let mut st = LedgerState {
        blockers: res.blockers,
        ..Default::default()
    };
    let mut pools = PoolSet::default();
    let mut stats = FoldStats::default(); // M3/FR9: fee_sats_consumed filled in Task 11, sigma_in here

    for eff in &res.timeline {
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
                        continue;
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
                        continue;
                    }
                };
                let key = pool_key(date, &wallet);
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
                    let legs = make_disposal_legs(&consumed, net, date, &mut st, &eff.id);
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
                        continue;
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
                        continue;
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
            Op::SelfTransfer {
                sat,
                fee_sat: _,
                dest,
            } => {
                let wallet = match &eff.wallet {
                    Some(w) => w.clone(),
                    None => {
                        st.add_blocker(
                            BlockerKind::UncoveredDisposal,
                            Some(eff.id.clone()),
                            "self transfer without source wallet",
                        );
                        continue;
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
                // Task 11: fee handling (TP8 (c) basis re-home / (b) mini-disposition) slots in here,
                // between building `relocated` and pushing to the destination pool.
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
                        continue;
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
                        continue;
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
            Op::GiftOut { sat, fmv, .. } => {
                // TP10: gift outbound → Removal with zero recognized gain; no Disposal.
                let wallet = match &eff.wallet {
                    Some(w) => w.clone(),
                    None => {
                        st.add_blocker(
                            BlockerKind::UncoveredDisposal,
                            Some(eff.id.clone()),
                            "gift out without wallet",
                        );
                        continue;
                    }
                };
                let key = pool_key(date, &wallet);
                let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
                if shortfall > 0 {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        format!("gift out short by {shortfall} sat"),
                    );
                }
                if !consumed.is_empty() {
                    let (legs, donor_acquired_at) =
                        make_removal_legs(&consumed, *fmv, date, &mut st, &eff.id);
                    // Task 11: fee step (TP8 (c) fee-sat basis carry) slots in here between legs and push.
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
                        continue;
                    }
                };
                let key = pool_key(date, &wallet);
                let (consumed, shortfall) = pools.consume_fifo(&key, *sat);
                if shortfall > 0 {
                    st.add_blocker(
                        BlockerKind::UncoveredDisposal,
                        Some(eff.id.clone()),
                        format!("donate short by {shortfall} sat"),
                    );
                }
                if !consumed.is_empty() {
                    let (legs, donor_acquired_at) =
                        make_removal_legs(&consumed, *fmv, date, &mut st, &eff.id);
                    // Task 11: fee step (TP8 (c) fee-sat basis carry) slots in here between legs and push.
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

    finalize(&mut st, pools, stats);
    st
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
