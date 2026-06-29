use crate::conventions::{split_pro_rata, Sat, TaxDate, Usd, TRANSITION_DATE};
use crate::event::BasisSource;
use crate::identity::{EventId, LotId, WalletId};
use crate::state::Lot;
use std::collections::BTreeMap;

/// Pool key: a single UniversalPool before 2025-01-01 (un-partitioned by wallet, §7.4), then per-wallet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolKey {
    Universal,
    Wallet(WalletId),
}
pub fn pool_key(date: TaxDate, wallet: &WalletId) -> PoolKey {
    if date < TRANSITION_DATE {
        PoolKey::Universal
    } else {
        PoolKey::Wallet(wallet.clone())
    }
}

#[derive(Debug, Default)]
pub struct PoolSet {
    /// Live lots per pool, kept in FIFO order (push on acquire/relocate; consume from the front).
    pub pools: BTreeMap<PoolKey, Vec<Lot>>,
    /// Per-origin split counter for deterministic split_sequence assignment (§6.2).
    next_split: BTreeMap<EventId, u32>,
}

impl PoolSet {
    /// Assign the next split_sequence for an origin (origin's first lot uses 0 via `new_origin`).
    /// Exposed as `pub` so the SelfTransfer fold arm can allocate IDs for relocated lot fragments.
    pub fn bump_split(&mut self, origin: &EventId) -> u32 {
        let e = self.next_split.entry(origin.clone()).or_insert(0);
        let v = *e;
        *e += 1;
        v
    }
    /// Register a brand-new origin (Acquire/Income/seeded), claiming split_sequence 0.
    pub fn new_origin_lot(&mut self, key: PoolKey, mut lot: Lot) {
        let s = self.bump_split(&lot.lot_id.origin_event_id);
        lot.lot_id.split_sequence = s;
        self.pools.entry(key).or_default().push(lot);
    }
    /// Push a pre-built lot (already carrying a final LotId), e.g. relocated/seeded lots.
    pub fn push_lot(&mut self, key: PoolKey, lot: Lot) {
        self.pools.entry(key).or_default().push(lot);
    }
    /// Seed the per-origin split counter to `next`, so the next `bump_split(origin)` returns
    /// `next`. Used for Path-B seed lots (I-2): seed lots occupy indices 0..seed_len-1; without
    /// initialising here, a later `SelfTransfer` relocation's `bump_split` returns 0 — colliding
    /// with seed-lot index 0. Calling this once with `next = seed.len()` claims those indices.
    pub fn init_split_counter(&mut self, origin: &EventId, next: u32) {
        self.next_split.insert(origin.clone(), next);
    }

    /// FIFO-consume `need` sats from `key`. Returns the consumed (lot_id, sat, gain_basis, loss_basis, term-anchors)
    /// fragments and a shortfall (>0 if the pool could not cover `need` — caller raises uncovered_disposal).
    pub fn consume_fifo(&mut self, key: &PoolKey, need: Sat) -> (Vec<Consumed>, Sat) {
        let mut out = Vec::new();
        let mut remaining = need;
        if let Some(lots) = self.pools.get_mut(key) {
            let mut idx = 0;
            while remaining > 0 && idx < lots.len() {
                let lot = &mut lots[idx];
                if lot.remaining_sat <= 0 {
                    idx += 1;
                    continue;
                }
                let take = remaining.min(lot.remaining_sat);
                let (gain_basis, _rest) = split_pro_rata(lot.usd_basis, take, lot.remaining_sat);
                let loss_basis = lot
                    .dual_loss_basis
                    .map(|l| split_pro_rata(l, take, lot.remaining_sat).0);
                out.push(Consumed {
                    lot_id: lot.lot_id.clone(),
                    sat: take,
                    gain_basis,
                    loss_basis,
                    gain_hp_start: lot.gain_hp_start(),
                    loss_hp_start: lot.loss_hp_start(),
                    basis_source: lot.basis_source,
                    dual: lot.dual_loss_basis.is_some(),
                    basis_pending: lot.basis_pending,
                    wallet: lot.wallet.clone(),
                    acquired_at: lot.acquired_at,
                    donor_acquired_at: lot.donor_acquired_at,
                });
                // reduce the lot exactly (conserves Σbasis: gain_basis subtracted, remainder stays)
                lot.usd_basis -= gain_basis;
                if let (Some(dl), Some(taken)) = (lot.dual_loss_basis.as_mut(), loss_basis) {
                    *dl -= taken;
                }
                lot.remaining_sat -= take;
                remaining -= take;
                idx += 1;
            }
            lots.retain(|l| l.remaining_sat > 0);
        }
        (out, remaining)
    }
}

/// A consumed fragment (used to build Disposal/Removal/relocation legs).
#[derive(Debug, Clone)]
pub struct Consumed {
    pub lot_id: LotId,
    pub sat: Sat,
    pub gain_basis: Usd,
    pub loss_basis: Option<Usd>,
    pub gain_hp_start: TaxDate,
    pub loss_hp_start: TaxDate,
    pub basis_source: BasisSource,
    pub dual: bool,
    pub basis_pending: bool,
    pub wallet: WalletId,
    pub acquired_at: TaxDate,
    pub donor_acquired_at: Option<TaxDate>,
}
