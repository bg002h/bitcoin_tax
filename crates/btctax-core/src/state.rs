use crate::conventions::{Sat, TaxDate, Usd};
use crate::event::{BasisSource, DisposeKind, IncomeKind};
use crate::identity::{EventId, LotId, WalletId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Term {
    ShortTerm,
    LongTerm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiftZone {
    Gain,
    Loss,
    NoGainNoLoss,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Hard,
    Advisory,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockerKind {
    FmvMissing,
    UncoveredDisposal,
    ImportConflict,
    DecisionConflict,
    UnknownBasisInbound,
    Unclassified,
    SafeHarborUnconservable,
    SafeHarborTimebar,
    UnmatchedOutflows,
    Pre2025MethodNote,
}
impl BlockerKind {
    pub fn severity(self) -> Severity {
        use BlockerKind::*;
        match self {
            FmvMissing
            | UncoveredDisposal
            | ImportConflict
            | DecisionConflict
            | UnknownBasisInbound
            | Unclassified
            | SafeHarborUnconservable => Severity::Hard,
            SafeHarborTimebar | UnmatchedOutflows | Pre2025MethodNote => Severity::Advisory,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocker {
    pub kind: BlockerKind,
    pub event: Option<EventId>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lot {
    pub lot_id: LotId,
    pub wallet: WalletId,
    pub acquired_at: TaxDate, // gift loss-zone HP start = this (gift date); see donor_acquired_at for tacking
    pub original_sat: Sat,
    pub remaining_sat: Sat,
    pub usd_basis: Usd, // gain basis
    pub basis_source: BasisSource,
    pub dual_loss_basis: Option<Usd>, // received gifts (TP11): loss basis when FMV-at-gift < donor basis
    pub donor_acquired_at: Option<TaxDate>, // tacking (TP11/§1223(2)); gain/no-dual HP start
    pub basis_pending: bool, // FMV-missing income / unknown-basis gift: gain is gated until resolved
}
impl Lot {
    /// HP start used on the gain side / no-dual case (tacks donor period when present).
    pub fn gain_hp_start(&self) -> TaxDate {
        self.donor_acquired_at.unwrap_or(self.acquired_at)
    }
    /// HP start used on the loss side of a dual-basis gift (the gift/received date).
    pub fn loss_hp_start(&self) -> TaxDate {
        self.acquired_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisposalLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub proceeds: Usd, // allocated net proceeds (gross − disposition fee, TP2)
    pub basis: Usd,    // tax-reported basis (zone-dependent for dual-basis gifts)
    pub gain: Usd,
    pub term: Term,
    pub basis_source: BasisSource,
    pub gift_zone: Option<GiftZone>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disposal {
    pub event: EventId,
    pub kind: DisposeKind,
    pub disposed_at: TaxDate,
    pub legs: Vec<DisposalLeg>,
    /// TP8 config-(b) fee-sat mini-disposition: a RECOGNITION record only — excluded from FR9 Σdisposed
    /// (its sats live in Σ on-chain-fee-sats; no second conservation entry).
    pub fee_mini_disposition: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalKind {
    Gift,
    Donation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub basis: Usd,
    pub fmv_at_transfer: Usd,
    pub term: Term,
    pub basis_source: BasisSource,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removal {
    pub event: EventId,
    pub kind: RemovalKind,
    pub removed_at: TaxDate,
    pub legs: Vec<RemovalLeg>,
    pub appraisal_required: bool, // donation (>$5k FMV over-flag, FOLLOWUPS)
    pub donor_acquired_at: Option<TaxDate>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeRecord {
    pub event: EventId,
    pub recognized_at: TaxDate,
    pub sat: Sat,
    pub usd_fmv: Usd,
    pub kind: IncomeKind,
    pub business: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingTransfer {
    pub event: EventId,
    pub principal_sat: Sat,
    pub fee_sat: Option<Sat>,
    pub legs: Vec<PendingLeg>, // lots removed into pending (carry basis + acquired_at)
}

/// Fold accumulators that are NOT directly reconstructable from the post-fold `LedgerState` vectors
/// (FR9 `Σ in` / `Σ on-chain-fee-sats` / `Σ pending`). Carried as a FIELD on `LedgerState` (M3) —
/// `project` always returns `LedgerState` (NO `(LedgerState, FoldStats)` tuple). Populated in `finalize`;
/// a deterministic function of the events, so it is included in `PartialEq` and the determinism tests hold.
/// Zero-valued by `Default` (the early tasks leave it zero; Task 11 fills `fee_sats_consumed`, Task 13 the rest).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FoldStats {
    pub sigma_in: Sat, // externally-sourced acquisitions (Acquire + Income + classified GiftReceived)
    pub fee_sats_consumed: Sat, // sole FR9 conservation home for network-fee sats
    pub sigma_pending: Sat, // principal + fee sats sitting in pending_reconciliation
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerState {
    pub lots: Vec<Lot>,
    pub holdings_by_wallet: BTreeMap<WalletId, Sat>,
    pub disposals: Vec<Disposal>,
    pub removals: Vec<Removal>,
    pub income_recognized: Vec<IncomeRecord>,
    pub pending_reconciliation: Vec<PendingTransfer>,
    pub blockers: Vec<Blocker>,
    pub stats: FoldStats, // M3: fold accumulators (FR9), on-state field — never a tuple return
}
impl LedgerState {
    pub(crate) fn add_blocker(
        &mut self,
        kind: BlockerKind,
        event: Option<EventId>,
        detail: impl Into<String>,
    ) {
        self.blockers.push(Blocker {
            kind,
            event,
            detail: detail.into(),
        });
    }
}
