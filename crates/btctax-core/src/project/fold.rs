use crate::conventions::Sat;
use crate::identity::LotId;
use crate::price::PriceProvider;
use crate::project::pools::{pool_key, PoolSet};
use crate::project::resolve::{sort_canonical, Op, Resolution};
use crate::state::{BlockerKind, FoldStats, LedgerState, Lot};
use crate::ProjectionConfig;
use std::collections::BTreeMap;

pub fn fold(
    mut res: Resolution,
    _prices: &dyn PriceProvider,
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
