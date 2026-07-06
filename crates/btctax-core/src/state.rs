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
    /// §A.5(a): a `MethodElection` whose `effective_from` precedes its made-date or TRANSITION_DATE —
    /// a standing order cannot be back-dated (§1.1012-1(j) no post-hoc identification). Hard.
    MethodElectionBackdated,
    /// §A.4: a `LotSelection` that fails validation (unknown/cross-wallet/over-drawn lot, or principal
    /// mismatch). Hard — the named identification is unusable so the disposal's tax is gated.
    LotSelectionInvalid,
    /// §A.7 / §7.4: the live `pre2025_method` config differs from the GOVERNING (effective) allocation's
    /// recorded `pre2025_method`. The allocation conserves under ITS recorded method, so this is a method
    /// drift — NOT bad data (never `SafeHarborUnconservable`). Hard: a post-attestation method change would
    /// silently break conservation between the pre-2025 residue and the irrevocable allocation. Clearable by
    /// reverting the live config to the recorded method; the irrevocable allocation is never rewritten.
    Pre2025MethodConflictsAllocation,
    /// §B.4 / B-I1: the projection carries an unresolved Hard blocker (`severity()==Hard`) anywhere, so B
    /// refuses to present a number for the year (projection-wide gate). Returns a `TaxOutcome::NotComputable`
    /// with this kind. Clearable by resolving the underlying Hard blocker. Hard.
    TaxYearNotComputable,
    /// §B.1: no `tax_profile` is set for the year being computed. B does not guess the surrounding tax
    /// context — the user must supply it via `tax-profile set`. Hard.
    TaxProfileMissing,
    /// §B.2: no bundled tax table is available for the year being computed. Hard.
    TaxTableMissing,
    /// §170(f)(11)(C): the term-aware claimed-deduction proxy for a charitable donation exceeds
    /// $5,000 — a qualified appraisal is likely required (CCA 202302012 confirms the
    /// exchange-price/readily-valued exception does NOT apply to crypto).
    /// **Advisory** — never gates `compute_tax_year`; emitted per qualifying Donate event.
    QualifiedAppraisalNote,
    /// Cycle A: an inbound self-transfer (`InboundClass::SelfTransferMine`) whose basis was DEFAULTED
    /// to $0 (`basis == None`). $0 is a *computable*, conservative value (never gates a later disposal —
    /// contrast the Hard `UnknownBasisInbound`, which creates NO lot); this is a pure honesty prompt to
    /// supply the real cost. Fires on `None` only, never on an attested `Some(0)`.
    /// **Advisory** — never gates `compute_tax_year`.
    SelfTransferInboundZeroBasis,
    /// Pseudo-reconcile mode (sub-project 2) is ON and at least one synthetic (non-persisted) default
    /// is CONTRIBUTING to this projection. A loud "this picture is a placeholder — correct it toward
    /// truth, and do NOT file it" banner (renders automatically via `{:?}` in `verify`). Drives the
    /// interim export-refusal guard (sub-2) until the sub-3 typed-attest gate ships.
    /// **Advisory** — never gates `compute_tax_year` (the mode's whole point is to PRESENT a number).
    PseudoReconcileActive,
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
            | SafeHarborUnconservable
            | MethodElectionBackdated
            | LotSelectionInvalid
            | Pre2025MethodConflictsAllocation
            | TaxYearNotComputable
            | TaxProfileMissing
            | TaxTableMissing => Severity::Hard,
            SafeHarborTimebar
            | UnmatchedOutflows
            | Pre2025MethodNote
            | QualifiedAppraisalNote
            | SelfTransferInboundZeroBasis
            | PseudoReconcileActive => Severity::Advisory,
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
    /// Pseudo-reconcile taint (sub-project 2, [R0-C1]): `true` when this lot's EXISTENCE or its BASIS
    /// traces to a synthetic (non-persisted) default decision — e.g. a `SelfTransferMine{$0}` conjured
    /// for an unknown-basis inbound, or a relocated fragment carrying a pseudo source lot's taint. Rides
    /// the DATA (`Lot`→`Consumed`→leg) so a REAL Sell consuming a pseudo `$0`-basis lot renders FLAGGED,
    /// never as a clean `proceeds − 0`. A DEDICATED bool — never a `BasisSource` variant — so the
    /// CSV/form writers OMIT it (it must NEVER reach any export file). Always `false` outside pseudo mode.
    pub pseudo: bool,
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
    /// Zone-aware holding-period start: the SAME HP-start passed to `term_for` for this leg.
    /// In the §1015 dual-basis loss zone this is `loss_hp_start` (the gift date — loss basis
    /// does NOT tack); in all other zones it is `gain_hp_start` (tacked donor date for gifts,
    /// acquisition date otherwise). Must never contradict `leg.term`. [R0-C1]
    pub acquired_at: TaxDate,
    /// The wallet that held the consumed lot at disposal time — the ONLY sound source (D1 [R0-I1]).
    pub wallet: WalletId,
    /// Pseudo-reconcile taint (sub-project 2, [R0-C1]): `true` when this leg's basis/existence traces to
    /// a synthetic default — set from the consumed lot's `pseudo` bit OR the disposal event itself being
    /// synthetic. Renders `[PSEUDO]` on screen; OMITTED from every CSV/form writer.
    pub pseudo: bool,
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
    /// Holding-period start = the SAME HP-start passed to `term_for` for this leg (`gain_hp_start`).
    /// A removal (gift/donation) recognizes NO gain/loss (TP10), so there is NO §1015 dual-basis
    /// loss-zone HP-start divergence like a `DisposalLeg` — this is ALWAYS `gain_hp_start` and can
    /// never contradict `leg.term`. For a gift-received-then-donated lot, `gain_hp_start` is the
    /// tacked donor acquisition date (§1223), which is the correct Form 8283 "date acquired" because
    /// it matches the leg's holding-period `term` [R0-M2]. Must never contradict `leg.term`. [D1]
    pub acquired_at: TaxDate,
    /// Pseudo-reconcile taint (sub-project 2, [R0-C1]): `true` when this removal leg's basis/existence
    /// traces to a synthetic default (consumed lot pseudo OR the removal event synthetic). Renders
    /// `[PSEUDO]` on screen; OMITTED from removals.csv / Form 8283.
    pub pseudo: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removal {
    pub event: EventId,
    pub kind: RemovalKind,
    pub removed_at: TaxDate,
    pub legs: Vec<RemovalLeg>,
    pub appraisal_required: bool, // donation (>$5k FMV over-flag, FOLLOWUPS)
    pub donor_acquired_at: Option<TaxDate>,
    /// §170(e)(1)(A) charitable-deduction amount for a Donation: `Some(Σ(LT→fmv; ST→min(fmv,basis)))`.
    /// `None` for a Gift (not a charitable deduction). Standalone Schedule-A figure — does NOT
    /// feed engine B / `compute_tax_year`. Pre-§170(b) AGI limits and carryover.
    pub claimed_deduction: Option<Usd>,
    /// Free-form donee identifier (Chunk 2). `None` for legacy events without the field.
    /// Used by removals.csv and (for Donations) Form 8283. Does NOT feed engine B / tax math.
    pub donee: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeRecord {
    pub event: EventId,
    pub recognized_at: TaxDate,
    pub sat: Sat,
    pub usd_fmv: Usd,
    pub kind: IncomeKind,
    pub business: bool,
    /// Pseudo-reconcile taint (sub-project 2 / #41 Part B, [R0-I2]): `true` when this income's
    /// recognized `usd_fmv` was SYNTHESIZED from the daily-close default in pseudo mode (a
    /// `PseudoKind::PseudoFmv` — the import carried no FMV and a local price existed). Set from
    /// `eff.pseudo` at BOTH fold push sites. The on-screen report flags such a row `[PSEUDO]`; the CSV
    /// writers OMIT it (never exported — the estimate is export-gated). Always `false` outside pseudo
    /// mode ⇒ projection byte-identical.
    pub pseudo: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingLeg {
    pub lot_id: LotId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
    /// Pseudo-reconcile taint (sub-project 2, [R0-C1]): `true` when the lot removed into pending traces
    /// to a synthetic default (e.g. a pseudo self-transfer-in lot later withdrawn). Never exported.
    pub pseudo: bool,
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
    /// Pseudo-reconcile (sub-project 2): count of synthetic (non-persisted) default decisions
    /// CONTRIBUTING to this projection. `0` outside pseudo mode. Queryable signal for the banner, the
    /// interim export-refusal guard [R0-I3], and sub-3's typed-attest gate. `> 0` ⇔ a
    /// `PseudoReconcileActive` advisory blocker is present and every `[PSEUDO]`-flagged row is fictional.
    pub pseudo_synthetic_count: usize,
}
impl LedgerState {
    /// Pseudo-reconcile (sub-project 2): `true` when any synthetic default contributes to this
    /// projection. The single load-bearing signal for the export-refusal guard [R0-I3].
    pub fn pseudo_active(&self) -> bool {
        self.pseudo_synthetic_count > 0
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_blockers_are_hard() {
        assert_eq!(
            BlockerKind::MethodElectionBackdated.severity(),
            Severity::Hard
        );
        assert_eq!(BlockerKind::LotSelectionInvalid.severity(), Severity::Hard);
        assert_eq!(
            BlockerKind::Pre2025MethodConflictsAllocation.severity(),
            Severity::Hard
        );
        assert_eq!(BlockerKind::TaxProfileMissing.severity(), Severity::Hard);
        assert_eq!(BlockerKind::TaxTableMissing.severity(), Severity::Hard);
        assert_eq!(BlockerKind::TaxYearNotComputable.severity(), Severity::Hard);
        // Task 1 KAT: QualifiedAppraisalNote MUST be Advisory — never Hard; must never gate compute_tax_year.
        assert_eq!(
            BlockerKind::QualifiedAppraisalNote.severity(),
            Severity::Advisory
        );
    }
}
