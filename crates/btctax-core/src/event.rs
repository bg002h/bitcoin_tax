//! The canonical event taxonomy (§6.4). One `EventPayload` enum; imported, system, and decision variants.
use crate::conventions::{Sat, TaxDate, Usd};
use crate::identity::{EventId, Fingerprint, WalletId};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FmvStatus {
    ExchangeProvided,
    PriceDataset,
    ManualEntry,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BasisSource {
    ExchangeProvided,
    ComputedFromCost,
    FmvAtIncome,
    CarriedFromTransfer,
    GiftCarryover,
    GiftFmvFallback,
    SafeHarborAllocated,
    ReconstructedPerWallet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncomeKind {
    Mining,
    Staking,
    Interest,
    Airdrop,
    Reward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisposeKind {
    Sell,
    Spend,
}

// ---- imported payloads ----
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Acquire {
    pub sat: Sat,
    pub usd_cost: Usd,
    pub fee_usd: Usd,
    pub basis_source: BasisSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Income {
    pub sat: Sat,
    pub usd_fmv: Option<Usd>,
    pub fmv_status: FmvStatus,
    pub kind: IncomeKind,
    pub business: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dispose {
    pub sat: Sat,
    pub usd_proceeds: Usd, // GROSS; fee_usd reduces proceeds (TP2)
    pub fee_usd: Usd,
    pub kind: DisposeKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferOut {
    pub sat: Sat,
    pub fee_sat: Option<Sat>,
    pub dest_addr: Option<String>,
    pub txid: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferIn {
    pub sat: Sat,
    pub src_addr: Option<String>,
    pub txid: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unclassified {
    pub raw: String,
}

// ---- system payload ----
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportConflict {
    pub target: EventId,
    pub new_payload: Box<EventPayload>,
    pub new_fingerprint: Fingerprint,
}

// ---- decision payloads (§6.4) ----
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferTarget {
    InEvent(EventId),
    Wallet(WalletId),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferLink {
    pub out_event: EventId,
    pub in_event_or_wallet: TransferTarget,
}
/// What a TransferOut is reclassified to (the proceeds/FMV ride in `principal_proceeds_or_fmv`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutflowClass {
    Dispose { kind: DisposeKind },
    GiftOut,
    Donate { appraisal_required: bool },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclassifyOutflow {
    pub transfer_out_event: EventId,
    pub as_: OutflowClass,
    pub principal_proceeds_or_fmv: Usd,
    pub fee_usd: Option<Usd>, // TP8: fee handling for a reclassified outflow
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InboundClass {
    Income {
        kind: IncomeKind,
        fmv: Option<Usd>,
        business: bool,
    },
    GiftReceived {
        donor_basis: Option<Usd>,
        donor_acquired_at: Option<TaxDate>,
        fmv_at_gift: Usd,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifyInbound {
    pub transfer_in_event: EventId,
    pub as_: InboundClass,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualFmv {
    pub event: EventId,
    pub usd_fmv: Usd,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocMethod {
    ActualPosition,
    ProRata,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocLot {
    pub wallet: WalletId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeHarborAllocation {
    pub lots: Vec<AllocLot>,
    pub as_of_date: TaxDate, // fixed 2025-01-01 snapshot
    pub method: AllocMethod,
    pub timely_allocation_attested: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersedeImport {
    pub conflict_event: EventId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectImport {
    pub conflict_event: EventId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoidDecisionEvent {
    pub target_event_id: EventId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifyRaw {
    pub target: EventId,
    pub as_: Box<EventPayload>, // the supplied imported payload
}

/// The single payload sum-type carried by every `LedgerEvent` (§6.3/§6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPayload {
    // imported
    Acquire(Acquire),
    Income(Income),
    Dispose(Dispose),
    TransferOut(TransferOut),
    TransferIn(TransferIn),
    Unclassified(Unclassified),
    // system
    ImportConflict(ImportConflict),
    // decisions
    TransferLink(TransferLink),
    ReclassifyOutflow(ReclassifyOutflow),
    ClassifyInbound(ClassifyInbound),
    ManualFmv(ManualFmv),
    SafeHarborAllocation(SafeHarborAllocation),
    SupersedeImport(SupersedeImport),
    RejectImport(RejectImport),
    VoidDecisionEvent(VoidDecisionEvent),
    ClassifyRaw(ClassifyRaw),
}

impl EventPayload {
    /// True for the six adapter-emitted imported payloads (the only ones folded as primary movements).
    pub fn is_imported(&self) -> bool {
        matches!(
            self,
            EventPayload::Acquire(_)
                | EventPayload::Income(_)
                | EventPayload::Dispose(_)
                | EventPayload::TransferOut(_)
                | EventPayload::TransferIn(_)
                | EventPayload::Unclassified(_)
        )
    }
}

/// An immutable ledger event (§6.3). `utc_timestamp` is the UTC instant (decisions: creation time);
/// `original_tz` drives the §6.1 tax-date. For decisions, `id` is `EventId::Decision { seq }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEvent {
    pub id: EventId,
    #[serde(with = "time::serde::rfc3339")]
    pub utc_timestamp: OffsetDateTime,
    pub original_tz: UtcOffset,
    pub wallet: Option<WalletId>,
    pub payload: EventPayload,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{EventId, Source, SourceRef};
    use rust_decimal_macros::dec;
    use time::macros::{datetime, offset};

    fn sample(payload: EventPayload) -> LedgerEvent {
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("X")),
            utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
            original_tz: offset!(-05:00),
            wallet: Some(crate::identity::WalletId::Exchange {
                provider: "coinbase".into(),
                account: "main".into(),
            }),
            payload,
        }
    }

    #[test]
    fn every_variant_serde_round_trips() {
        let payloads = vec![
            EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(60.00),
                fee_usd: dec!(1.00),
                basis_source: BasisSource::ExchangeProvided,
            }),
            EventPayload::Income(Income {
                sat: 50_000,
                usd_fmv: Some(dec!(30.00)),
                fmv_status: FmvStatus::PriceDataset,
                kind: IncomeKind::Interest,
                business: false,
            }),
            EventPayload::Dispose(Dispose {
                sat: 25_000,
                usd_proceeds: dec!(40.00),
                fee_usd: dec!(0.50),
                kind: DisposeKind::Sell,
            }),
            EventPayload::TransferOut(TransferOut {
                sat: 10_000,
                fee_sat: Some(150),
                dest_addr: Some("bc1q…".into()),
                txid: Some("ab12".into()),
            }),
            EventPayload::TransferIn(TransferIn {
                sat: 10_000,
                src_addr: None,
                txid: Some("ab12".into()),
            }),
            EventPayload::Unclassified(Unclassified {
                raw: "weird row".into(),
            }),
        ];
        for p in payloads {
            let ev = sample(p);
            let json = serde_json::to_string(&ev).unwrap();
            let back: LedgerEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(ev, back);
        }
    }

    #[test]
    fn decimal_is_serialized_losslessly_as_string() {
        let ev = sample(EventPayload::Acquire(Acquire {
            sat: 1,
            usd_cost: dec!(0.10),
            fee_usd: dec!(0),
            basis_source: BasisSource::ComputedFromCost,
        }));
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("\"0.10\"")); // serde-str: exact, not a 0.1 float
    }
}
