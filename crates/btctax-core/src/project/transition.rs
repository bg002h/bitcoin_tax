//! §7.4 / TP6: the 2025 basis transition. Pre-2025 lots live in a single FIFO `PoolKey::Universal`;
//! at the 2025-01-01 boundary the per-wallet pools are seeded from the Universal residue via either
//! Path A (FIFO reconstruction, the legal default) or Path B (an effective Rev. Proc. 2024-28 safe-harbor
//! allocation). Effectiveness/Path selection is decided by `resolve`; this module only (a) computes the
//! allocation-independent pre-2025 Universal snapshot used by the conservation guard, and (b) performs the
//! one-shot boundary seed during the pass-2 fold.
use crate::conventions::{Sat, Usd, TRANSITION_DATE};
use crate::event::{BasisSource, LotPick};
use crate::identity::EventId;
use crate::price::PriceProvider;
use crate::project::fold::{fold_event, FoldCtx}; // the SHARED per-event dispatcher (so the pre-fold cannot diverge)
use crate::project::pools::{PoolKey, PoolSet};
use crate::project::resolve::{sort_canonical, Eff, ElectionRec, TransitionMode};
use crate::state::{FoldStats, LedgerState, Lot};
use crate::{LotMethod, ProjectionConfig};
use std::collections::BTreeMap;

/// Σ held sat + Σ basis remaining in the single Universal pool at the 2025-01-01 boundary.
pub struct UniversalSnapshot {
    pub held_sat: Sat,
    pub basis: Usd,
}

/// I-1: a TRANSITION-FREE fold of ONLY the pre-2025 effective timeline into the Universal pool. Reuses the
/// exact pass-2 `fold_event` (so it cannot diverge) and NEVER seeds — so it depends only on pre-2025 history
/// and can be called from pass-1 effectiveness evaluation without infinite regress (§7.2: not circular).
///
/// §A.7: method-aware. The residue is computed under the supplied `method` (an allocation's RECORDED
/// `pre2025_method`), NOT necessarily the live `config.pre2025_method` — so a non-FIFO filer's safe-harbor
/// Path B conserves against the residue the allocation actually listed. The conflict between live config and
/// the recorded method is surfaced separately (`Pre2025MethodConflictsAllocation`), never here.
pub fn universal_snapshot(
    timeline: &[Eff],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    method: LotMethod,
    elections: &[ElectionRec],
    selections: &BTreeMap<EventId, Vec<LotPick>>,
) -> UniversalSnapshot {
    let cfg = ProjectionConfig {
        pre2025_method: method,
        ..*config
    }; // method-aware residue (§A.7): fold the pre-2025 Universal pool under the RECORDED method
    let mut pre: Vec<Eff> = timeline
        .iter()
        .filter(|e| e.date() < TRANSITION_DATE)
        .cloned()
        .collect();
    sort_canonical(&mut pre); // same canonical FIFO order pass 2 uses
    let mut pools = PoolSet::default();
    let mut sink = LedgerState::default(); // discarded; we only read the pool residue
    let mut stats = FoldStats::default();
    // Same FoldCtx the real fold uses, so the pre-2025 residue cannot diverge (I-1). Pre-2025 disposals
    // route through the Universal pool → the recorded `pre2025_method`; elections (forward-only) never apply.
    let ctx = FoldCtx {
        config: &cfg,
        elections,
        selections,
    };
    for eff in &pre {
        fold_event(eff, prices, &ctx, &mut pools, &mut sink, &mut stats);
    }
    let lots = pools
        .pools
        .get(&PoolKey::Universal)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    UniversalSnapshot {
        held_sat: lots.iter().map(|l| l.remaining_sat).sum(),
        basis: lots.iter().map(|l| l.usd_basis).sum(),
    }
}

/// Seed the per-wallet pools at the boundary, exactly once (called by `fold`).
pub fn seed_transition(mode: &TransitionMode, pools: &mut PoolSet, _st: &mut LedgerState) {
    // Take the Universal remainder out of the pool set (Path A drains it; Path B discards it).
    let universal: Vec<Lot> = pools.pools.remove(&PoolKey::Universal).unwrap_or_default();
    match mode {
        TransitionMode::PathA => {
            // Reconstruct: each remaining Universal lot moves to ITS holding wallet's pool, basis/acquired_at kept.
            // (Sats already removed into pending_reconciliation are not in the pool, so they are excluded here.)
            for mut lot in universal.into_iter().filter(|l| l.remaining_sat > 0) {
                // D-8: a conservative-filing tranche keeps its `EstimatedConservative` tag through the
                // 2025 Path-A reconstruction — else the tag never reaches 2025+ disposal legs and the P3
                // dip / P7 mandatory-disclosure layer silently drops. Path A keeps basis/acquired_at and
                // routes to `PoolKey::Wallet`, so the exemption changes ONLY the tag (position identical).
                if lot.basis_source != BasisSource::EstimatedConservative {
                    lot.basis_source = BasisSource::ReconstructedPerWallet;
                }
                let key = PoolKey::Wallet(lot.wallet.clone());
                pools.push_lot(key, lot);
            }
        }
        TransitionMode::PathB { seed } => {
            // Discard the Universal remainder; seed fresh per-wallet lots from the effective allocation.
            let seed_len = seed.len() as u32;
            for lot in seed.iter().cloned() {
                let key = PoolKey::Wallet(lot.wallet.clone());
                pools.push_lot(key, lot);
            }
            // I-2: Claim split indices 0..seed_len-1 in the counter so a later SelfTransfer
            // relocation's bump_split(allocation_id) returns seed_len, seed_len+1, ... — no
            // collision with the seeded lots' own split_sequence values.
            if let Some(first) = seed.first() {
                pools.init_split_counter(&first.lot_id.origin_event_id, seed_len);
            }
        }
    }
}
