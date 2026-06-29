//! §7.4 / TP6: the 2025 basis transition. Pre-2025 lots live in a single FIFO `PoolKey::Universal`;
//! at the 2025-01-01 boundary the per-wallet pools are seeded from the Universal residue via either
//! Path A (FIFO reconstruction, the legal default) or Path B (an effective Rev. Proc. 2024-28 safe-harbor
//! allocation). Effectiveness/Path selection is decided by `resolve`; this module only (a) computes the
//! allocation-independent pre-2025 Universal snapshot used by the conservation guard, and (b) performs the
//! one-shot boundary seed during the pass-2 fold.
use crate::conventions::{Sat, Usd, TRANSITION_DATE};
use crate::event::BasisSource;
use crate::price::PriceProvider;
use crate::project::fold::fold_event; // the SHARED per-event dispatcher (so the pre-fold cannot diverge)
use crate::project::pools::{PoolKey, PoolSet};
use crate::project::resolve::{sort_canonical, Eff, TransitionMode};
use crate::state::{FoldStats, LedgerState, Lot};
use crate::ProjectionConfig;

/// Σ held sat + Σ basis remaining in the single Universal pool at the 2025-01-01 boundary.
pub struct UniversalSnapshot {
    pub held_sat: Sat,
    pub basis: Usd,
}

/// I-1: a TRANSITION-FREE fold of ONLY the pre-2025 effective timeline into the Universal pool. Reuses the
/// exact pass-2 `fold_event` (so it cannot diverge) and NEVER seeds — so it depends only on pre-2025 history
/// and can be called from pass-1 effectiveness evaluation without infinite regress (§7.2: not circular).
pub fn universal_snapshot(
    timeline: &[Eff],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
) -> UniversalSnapshot {
    let mut pre: Vec<Eff> = timeline
        .iter()
        .filter(|e| e.date() < TRANSITION_DATE)
        .cloned()
        .collect();
    sort_canonical(&mut pre); // same canonical FIFO order pass 2 uses
    let mut pools = PoolSet::default();
    let mut sink = LedgerState::default(); // discarded; we only read the pool residue
    let mut stats = FoldStats::default();
    for eff in &pre {
        fold_event(eff, prices, config, &mut pools, &mut sink, &mut stats);
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
                lot.basis_source = BasisSource::ReconstructedPerWallet;
                let key = PoolKey::Wallet(lot.wallet.clone());
                pools.push_lot(key, lot);
            }
        }
        TransitionMode::PathB { seed } => {
            // Discard the Universal remainder; seed fresh per-wallet lots from the effective allocation.
            for lot in seed.iter().cloned() {
                let key = PoolKey::Wallet(lot.wallet.clone());
                pools.push_lot(key, lot);
            }
        }
    }
}
