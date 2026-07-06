use crate::conventions::{split_pro_rata, Sat, TaxDate, Usd, TRANSITION_DATE};
use crate::event::{BasisSource, LotPick};
use crate::identity::{EventId, LotId, WalletId};
use crate::state::Lot;
use crate::LotMethod;
use std::cmp::Ordering;
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
    /// Live lots per pool (push on acquire/relocate). Consumption order is chosen by `method_order`
    /// (acquisition-date for FIFO/LIFO, gain-basis for HIFO) — NOT raw push/insertion order (C1).
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

    /// FIFO-consume `need` sats from `key` (the FIFO-pinned wrapper for `consume_fee`/PendingOut).
    /// Delegates to `consume(.., Fifo, None)`: acquisition-date order (NOT raw insertion order, C1).
    /// Returns the consumed fragments and a shortfall (>0 if the pool could not cover `need`).
    pub fn consume_fifo(&mut self, key: &PoolKey, need: Sat) -> (Vec<Consumed>, Sat) {
        let r = self.consume(key, need, LotMethod::Fifo, None);
        (r.consumed, r.shortfall)
    }

    /// General pool consumption (§A.4): by `method` total-order (FIFO/LIFO/HIFO), or by an explicit
    /// named-lot `selection`. A `selection` that is infeasible *within this pool* (unknown lot,
    /// cross-wallet lot, or insufficient remaining) falls back to `method` order and reports the
    /// reason in `selection_error` — Σsat/Σbasis are conserved on every path.
    pub fn consume(
        &mut self,
        key: &PoolKey,
        need: Sat,
        method: LotMethod,
        selection: Option<&[LotPick]>,
    ) -> ConsumeResult {
        // ---- selection path: validate feasibility within THIS pool; fall back to method order on failure ----
        if let Some(picks) = selection {
            if let Err(reason) = self.selection_feasible(key, picks) {
                let (consumed, shortfall) = self.consume_ordered(key, need, method);
                return ConsumeResult {
                    consumed,
                    shortfall,
                    selection_error: Some(reason),
                };
            }
            let (consumed, shortfall) = self.consume_picks(key, picks);
            return ConsumeResult {
                consumed,
                shortfall,
                selection_error: None,
            };
        }
        // ---- method path ----
        let (consumed, shortfall) = self.consume_ordered(key, need, method);
        ConsumeResult {
            consumed,
            shortfall,
            selection_error: None,
        }
    }

    /// Is the named-lot `selection` consumable entirely from `key`? Distinguishes cross-wallet
    /// (lot lives in another pool — §1.1012-1(j) forbids cross-account ID) from truly unknown lots,
    /// and rejects over-draw. Pure check (no mutation); handles repeated picks of one lot.
    fn selection_feasible(&self, key: &PoolKey, picks: &[LotPick]) -> Result<(), String> {
        let pool = self.pools.get(key).map(Vec::as_slice).unwrap_or(&[]);
        // tentative per-lot remaining (handles multiple picks of one lot)
        let mut rem: BTreeMap<LotId, Sat> = BTreeMap::new();
        for l in pool {
            if l.remaining_sat > 0 {
                *rem.entry(l.lot_id.clone()).or_insert(0) += l.remaining_sat;
            }
        }
        for p in picks {
            match rem.get_mut(&p.lot) {
                None => {
                    // distinguish cross-wallet (lot exists in another pool) from truly unknown, for a precise reason
                    let elsewhere = self
                        .pools
                        .iter()
                        .any(|(k, v)| k != key && v.iter().any(|l| l.lot_id == p.lot));
                    return Err(if elsewhere {
                        format!(
                            "picked lot {}#{} is in another wallet — cross-account identification is not permitted (§1.1012-1(j))",
                            p.lot.origin_event_id.canonical(),
                            p.lot.split_sequence
                        )
                    } else {
                        format!(
                            "picked lot {}#{} does not exist",
                            p.lot.origin_event_id.canonical(),
                            p.lot.split_sequence
                        )
                    });
                }
                Some(r) if *r < p.sat => {
                    return Err(format!(
                        "picked lot {}#{} has {} sat remaining < {} requested",
                        p.lot.origin_event_id.canonical(),
                        p.lot.split_sequence,
                        *r,
                        p.sat
                    ))
                }
                Some(r) => {
                    *r -= p.sat;
                }
            }
        }
        Ok(())
    }

    /// Consume exactly the named lots in `picks` order (feasibility already guaranteed by caller).
    fn consume_picks(&mut self, key: &PoolKey, picks: &[LotPick]) -> (Vec<Consumed>, Sat) {
        let mut out = Vec::new();
        if let Some(lots) = self.pools.get_mut(key) {
            for p in picks {
                let mut take = p.sat;
                for lot in lots.iter_mut() {
                    if take <= 0 {
                        break;
                    }
                    if lot.lot_id != p.lot || lot.remaining_sat <= 0 {
                        continue;
                    }
                    let t = take.min(lot.remaining_sat);
                    out.push(Self::take_from(lot, t));
                    take -= t;
                }
            }
            lots.retain(|l| l.remaining_sat > 0);
        }
        (out, 0) // feasibility already guaranteed by selection_feasible
    }

    /// Consume `need` sats from `key` in `method` total order.
    fn consume_ordered(
        &mut self,
        key: &PoolKey,
        need: Sat,
        method: LotMethod,
    ) -> (Vec<Consumed>, Sat) {
        let mut out = Vec::new();
        let mut remaining = need;
        if let Some(lots) = self.pools.get_mut(key) {
            for i in method_order(lots, method) {
                if remaining <= 0 {
                    break;
                }
                let lot = &mut lots[i];
                if lot.remaining_sat <= 0 {
                    continue;
                }
                let take = remaining.min(lot.remaining_sat);
                out.push(Self::take_from(lot, take));
                remaining -= take;
            }
            lots.retain(|l| l.remaining_sat > 0);
        }
        (out, remaining)
    }

    /// Take `take` sat from `lot`, returning the `Consumed` fragment and reducing the lot
    /// (conserves Σbasis: gain_basis subtracted, remainder stays). This is the EXACT prior
    /// `consume_fifo` arithmetic (pools.rs split_pro_rata fragment math) — only the ORDER changes (C1).
    fn take_from(lot: &mut Lot, take: Sat) -> Consumed {
        let (gain_basis, _rest) = split_pro_rata(lot.usd_basis, take, lot.remaining_sat);
        let loss_basis = lot
            .dual_loss_basis
            .map(|l| split_pro_rata(l, take, lot.remaining_sat).0);
        let c = Consumed {
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
            pseudo: lot.pseudo, // [R0-C1] taint rides the DATA: pseudo lot → pseudo consumed fragment
        };
        lot.usd_basis -= gain_basis;
        if let (Some(dl), Some(taken)) = (lot.dual_loss_basis.as_mut(), loss_basis) {
            *dl -= taken;
        }
        lot.remaining_sat -= take;
        c
    }
}

/// The outcome of `PoolSet::consume`: consumed fragments, an uncovered `shortfall` (>0 ⇒ caller
/// raises `UncoveredDisposal`), and a `selection_error` set iff a named-lot selection was infeasible
/// and the engine fell back to method order.
#[derive(Debug, Clone)]
pub struct ConsumeResult {
    pub consumed: Vec<Consumed>,
    pub shortfall: Sat,
    pub selection_error: Option<String>,
}

/// Total-order ranking (NFR4): the indices of `lots` with `remaining_sat > 0`, in consumption order.
/// FIFO = acquisition-date asc (tie `lot_id` asc); LIFO = desc; HIFO via `hifo_cmp`.
fn method_order(lots: &[Lot], method: LotMethod) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..lots.len())
        .filter(|&i| lots[i].remaining_sat > 0)
        .collect();
    match method {
        LotMethod::Fifo => idx.sort_by(|&a, &b| {
            lots[a]
                .acquired_at
                .cmp(&lots[b].acquired_at)
                .then(lots[a].lot_id.cmp(&lots[b].lot_id))
        }),
        LotMethod::Lifo => idx.sort_by(|&a, &b| {
            lots[b]
                .acquired_at
                .cmp(&lots[a].acquired_at)
                .then(lots[b].lot_id.cmp(&lots[a].lot_id))
        }),
        LotMethod::Hifo => idx.sort_by(|&a, &b| hifo_cmp(&lots[a], &lots[b])),
    }
    idx
}

/// HIFO key: gain basis (`usd_basis`) per sat DESC; basis-pending (`usd_basis == 0`) LAST;
/// ties → oldest, then `lot_id`. Cross-multiplied (NFR5: exact Decimal, never division to a float).
/// Loss-basis (`dual_loss_basis`) is intentionally ignored — HIFO keys on gain basis only.
fn hifo_cmp(a: &Lot, b: &Lot) -> Ordering {
    let (az, bz) = (a.usd_basis == Usd::ZERO, b.usd_basis == Usd::ZERO);
    match (az, bz) {
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    let lhs = a.usd_basis * Usd::from(b.remaining_sat); // a.perSat vs b.perSat, no division
    let rhs = b.usd_basis * Usd::from(a.remaining_sat);
    rhs.cmp(&lhs) // DESC: higher per-sat first
        .then(a.acquired_at.cmp(&b.acquired_at))
        .then(a.lot_id.cmp(&b.lot_id))
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
    /// Pseudo-reconcile taint (sub-project 2, [R0-C1]): copied from the source lot's `pseudo` bit in
    /// `take_from`, so a disposal/relocation that consumes a pseudo lot carries the taint onto its legs.
    pub pseudo: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conventions::Usd;
    use crate::event::{BasisSource, LotPick};
    use crate::identity::{EventId, LotId, Source, SourceRef, WalletId};
    use crate::LotMethod;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn w() -> WalletId {
        WalletId::SelfCustody { label: "x".into() }
    }
    fn lot(rf: &str, acq: time::Date, sat: i64, basis: Usd) -> Lot {
        Lot {
            lot_id: LotId {
                origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
                split_sequence: 0,
            },
            wallet: w(),
            acquired_at: acq,
            original_sat: sat,
            remaining_sat: sat,
            usd_basis: basis,
            basis_source: BasisSource::ExchangeProvided,
            dual_loss_basis: None,
            donor_acquired_at: None,
            basis_pending: false,
            pseudo: false,
        }
    }
    fn pid(rf: &str) -> LotId {
        LotId {
            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
            split_sequence: 0,
        }
    }
    // Three lots whose method orders are all DISTINCT:
    //  A 2025-02-01 basis $50 ; B 2025-03-01 basis $90 (highest) ; C 2025-04-01 basis $40
    //  FIFO -> A,B,C ; LIFO -> C,B,A ; HIFO -> B,A,C
    fn three() -> PoolSet {
        let mut p = PoolSet::default();
        p.push_lot(
            PoolKey::Universal,
            lot("A", date!(2025 - 02 - 01), 100_000, dec!(50.00)),
        );
        p.push_lot(
            PoolKey::Universal,
            lot("B", date!(2025 - 03 - 01), 100_000, dec!(90.00)),
        );
        p.push_lot(
            PoolKey::Universal,
            lot("C", date!(2025 - 04 - 01), 100_000, dec!(40.00)),
        );
        p
    }

    #[test]
    fn fifo_consumes_oldest_first() {
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Fifo, None);
        assert_eq!(r.shortfall, 0);
        assert_eq!(r.consumed[0].lot_id, pid("A"));
    }
    #[test]
    fn pools_mechanic_stays_fifo() {
        // [reconcile-defaults] The FIFO-pinned mechanic (`consume_fifo`, used for fee/PendingOut/relocation
        // consumption) is INDEPENDENT of the electable/default lot method: it always consumes the
        // oldest-acquired lot first (A), even though the app's no-election tax default is now HIFO (B $90).
        let (consumed, shortfall) = three().consume_fifo(&PoolKey::Universal, 100_000);
        assert_eq!(shortfall, 0);
        assert_eq!(
            consumed[0].lot_id,
            pid("A"),
            "consume_fifo must pin oldest-first (FIFO), never follow the HIFO default"
        );
    }
    #[test]
    fn lifo_consumes_newest_first() {
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Lifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("C"));
    }
    #[test]
    fn hifo_consumes_highest_gain_basis_per_sat_first() {
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("B"));
    }
    // C1 divergence KAT at the pool level: a RELOCATED older lot is push_lot'd AFTER a newer one,
    // so insertion order is [NEW, OLD] but acquisition order is [OLD, NEW]. Acquisition-date FIFO
    // MUST consume OLD first (legacy insertion-order FIFO wrongly took NEW). LIFO/HIFO pin the rest.
    #[test]
    fn fifo_consumes_older_relocated_lot_before_newer_despite_insertion_order() {
        // push the NEWER lot first, then the OLDER (relocated) lot — insertion order != acq order
        let relocated = || {
            let mut p = PoolSet::default();
            p.push_lot(
                PoolKey::Universal,
                lot("NEW", date!(2025 - 08 - 01), 100_000, dec!(80.00)),
            );
            p.push_lot(
                PoolKey::Universal,
                lot("OLD", date!(2025 - 01 - 01), 100_000, dec!(40.00)),
            );
            p
        };
        // FIFO -> OLD (older acquired_at), diverging from insertion order which would take NEW
        let f = relocated().consume(&PoolKey::Universal, 100_000, LotMethod::Fifo, None);
        assert_eq!(f.consumed[0].lot_id, pid("OLD"));
        // LIFO -> NEW (newest acquired_at)
        let l = relocated().consume(&PoolKey::Universal, 100_000, LotMethod::Lifo, None);
        assert_eq!(l.consumed[0].lot_id, pid("NEW"));
        // HIFO -> NEW (higher per-sat basis $80 > $40)
        let h = relocated().consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(h.consumed[0].lot_id, pid("NEW"));
    }
    #[test]
    fn hifo_basis_pending_sorts_last() {
        let mut p = PoolSet::default();
        let mut pend = lot("P", date!(2025 - 01 - 01), 100_000, dec!(0));
        pend.basis_pending = true; // usd_basis == 0
        p.push_lot(PoolKey::Universal, pend);
        p.push_lot(
            PoolKey::Universal,
            lot("Q", date!(2025 - 06 - 01), 100_000, dec!(10.00)),
        );
        let r = p.consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("Q")); // pending ($0) sorts last
    }
    #[test]
    fn hifo_ties_break_oldest_then_lotid() {
        let mut p = PoolSet::default();
        p.push_lot(
            PoolKey::Universal,
            lot("OLD", date!(2025 - 02 - 01), 100_000, dec!(50.00)),
        );
        p.push_lot(
            PoolKey::Universal,
            lot("NEW", date!(2025 - 05 - 01), 100_000, dec!(50.00)),
        ); // same per-sat
        let r = p.consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("OLD"));
    }
    #[test]
    fn hifo_ignores_dual_loss_basis() {
        // Same gain basis per sat; only dual_loss_basis differs -> order must NOT change (oldest first).
        let mut p = PoolSet::default();
        let mut g = lot("G", date!(2025 - 02 - 01), 100_000, dec!(50.00));
        g.dual_loss_basis = Some(dec!(5.00));
        p.push_lot(PoolKey::Universal, g);
        p.push_lot(
            PoolKey::Universal,
            lot("H", date!(2025 - 05 - 01), 100_000, dec!(50.00)),
        );
        let r = p.consume(&PoolKey::Universal, 100_000, LotMethod::Hifo, None);
        assert_eq!(r.consumed[0].lot_id, pid("G")); // keyed on usd_basis only; tie -> oldest
    }
    #[test]
    fn selection_consumes_exactly_named_lots() {
        let picks = vec![
            LotPick {
                lot: pid("C"),
                sat: 100_000,
            },
            LotPick {
                lot: pid("A"),
                sat: 100_000,
            },
        ];
        let r = three().consume(&PoolKey::Universal, 200_000, LotMethod::Hifo, Some(&picks));
        assert!(r.selection_error.is_none());
        assert_eq!(
            r.consumed
                .iter()
                .map(|c| c.lot_id.clone())
                .collect::<Vec<_>>(),
            vec![pid("C"), pid("A")]
        );
    }
    #[test]
    fn selection_unknown_lot_reports_error_and_falls_back_to_method() {
        let picks = vec![LotPick {
            lot: pid("ZZZ"),
            sat: 100_000,
        }];
        let r = three().consume(&PoolKey::Universal, 100_000, LotMethod::Fifo, Some(&picks));
        assert!(r.selection_error.is_some());
        assert_eq!(r.consumed[0].lot_id, pid("A")); // fell back to FIFO order (sats conserved)
    }
    #[test]
    fn selection_insufficient_remaining_reports_error() {
        let picks = vec![LotPick {
            lot: pid("A"),
            sat: 999_999,
        }];
        let r = three().consume(&PoolKey::Universal, 999_999, LotMethod::Fifo, Some(&picks));
        assert!(r.selection_error.is_some());
    }
    #[test]
    fn selection_cross_wallet_lot_reports_error() {
        let mut p = PoolSet::default();
        p.push_lot(
            PoolKey::Wallet(WalletId::SelfCustody { label: "a".into() }),
            lot("A", date!(2025 - 02 - 01), 100_000, dec!(50.00)),
        );
        p.push_lot(
            PoolKey::Wallet(WalletId::SelfCustody { label: "b".into() }),
            lot("B", date!(2025 - 02 - 01), 100_000, dec!(50.00)),
        );
        // disposal is in wallet "a"; pick references lot "B" living in wallet "b" -> cross-account ID forbidden.
        let picks = vec![LotPick {
            lot: pid("B"),
            sat: 100_000,
        }];
        let r = p.consume(
            &PoolKey::Wallet(WalletId::SelfCustody { label: "a".into() }),
            100_000,
            LotMethod::Fifo,
            Some(&picks),
        );
        assert!(r.selection_error.as_deref().unwrap().contains("wallet"));
    }
    // Conservation under ANY ordering: Σ consumed sat and Σ consumed gain_basis are method-invariant.
    #[test]
    fn consumption_conserves_sat_and_basis_under_every_method() {
        let total_sat = 300_000;
        let total_basis = dec!(180.00); // 50 + 90 + 40
        for m in [LotMethod::Fifo, LotMethod::Lifo, LotMethod::Hifo] {
            let r = three().consume(&PoolKey::Universal, total_sat, m, None);
            assert_eq!(r.shortfall, 0);
            let sat: i64 = r.consumed.iter().map(|c| c.sat).sum();
            let basis: Usd = r.consumed.iter().map(|c| c.gain_basis).sum();
            assert_eq!(sat, total_sat, "Σsat must conserve for {m:?}");
            assert_eq!(basis, total_basis, "Σbasis must conserve for {m:?}");
        }
    }
}
