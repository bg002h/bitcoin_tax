use crate::conventions::{tax_date, Sat, TaxDate, Usd};
use crate::event::*;
use crate::identity::{EventId, SourceRef};
use crate::price::PriceProvider;
use crate::project::ProjectionConfig;
use crate::state::Blocker;
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
        kind: DisposeKind,
    },
    // (Task 6) Income, (Task 8) SelfTransfer/PendingOut/GiftReceived/IncomeInbound,
    // (Task 9) GiftOut/Donate, (Task 12) seeded — added as those tasks land.
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

pub struct Resolution {
    pub timeline: Vec<Eff>,
    pub transition: TransitionMode,
    pub blockers: Vec<Blocker>,
}

/// PASS 1. Task 4: copy imported events straight through (no decisions yet). Task 7 rewrites this.
/// `_prices`/`_config` are unused until Task 12 (transition effectiveness needs `config` for the TP8(b)
/// first-2025-disposition trigger and `prices` for the pre-2025 basis snapshot); they are part of the
/// signature from the START so `resolve`/`project` never change shape across tasks (I-2).
pub fn resolve(
    events: &[LedgerEvent],
    _prices: &dyn PriceProvider,
    _config: &ProjectionConfig,
) -> Resolution {
    let mut timeline = Vec::new();
    for ev in events {
        let (src_priority, src_ref) = match &ev.id {
            EventId::Import { source, source_ref } => (source.priority(), source_ref.clone()),
            _ => continue, // decisions/conflicts handled in Task 7
        };
        let op = match &ev.payload {
            EventPayload::Acquire(a) => Op::Acquire(a.clone()),
            EventPayload::Dispose(d) => Op::Dispose {
                sat: d.sat,
                proceeds: d.usd_proceeds,
                fee_usd: d.fee_usd,
                kind: d.kind,
            },
            EventPayload::Unclassified(_) => Op::Unclassified,
            _ => Op::Skip, // other imported variants land in Tasks 6/8
        };
        timeline.push(Eff {
            id: ev.id.clone(),
            utc: ev.utc_timestamp,
            tz: ev.original_tz,
            src_priority,
            src_ref,
            wallet: ev.wallet.clone(),
            op,
        });
    }
    Resolution {
        timeline,
        transition: TransitionMode::PathA,
        blockers: Vec::new(),
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
