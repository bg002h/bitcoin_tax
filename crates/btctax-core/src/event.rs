//! The canonical event taxonomy (§6.4). One `EventPayload` enum; imported, system, and decision variants.
use crate::conventions::{Sat, TaxDate, Usd};
use crate::identity::{EventId, Fingerprint, LotId, WalletId};
use crate::LotMethod;
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
    /// Donee identifier (free-form label; structured name/address/EIN = Chunk 3).
    /// `#[serde(default)]` ensures existing vault records without this field deserialize to `None`
    /// — the back-compat guarantee that lets legacy `"GiftOut"` unit-variant JSON still load.
    #[serde(default)]
    pub donee: Option<String>,
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
    pub usd_basis: Usd,       // GAIN basis = donor carryover basis (§1015(a))
    pub acquired_at: TaxDate, // gift date = loss-zone HP start (no tacking on loss side)
    #[serde(default)]
    pub dual_loss_basis: Option<Usd>, // §1015(a) LOSS basis = FMV-at-gift; Some only when FMV-at-gift < donor basis
    #[serde(default)]
    pub donor_acquired_at: Option<TaxDate>, // §1223(2) tacking; gain/no-dual-zone HP start; None otherwise
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeHarborAllocation {
    pub lots: Vec<AllocLot>,
    pub as_of_date: TaxDate, // fixed 2025-01-01 snapshot
    pub method: AllocMethod,
    pub timely_allocation_attested: bool,
    /// §A.7: the historical lot-consumption method (FIFO/LIFO/HIFO) used to derive the pre-2025
    /// Universal residue THIS allocation lists — captured at attestation time and IMMUTABLE thereafter.
    /// `universal_snapshot` conserves under THIS value (not the live config); a later live-config change
    /// fires the hard `Pre2025MethodConflictsAllocation` (never `SafeHarborUnconservable`) and never
    /// rewrites the irrevocable allocation (§7.4). `#[serde(default)]` -> Fifo for pre-A.7 records.
    #[serde(default)]
    pub pre2025_method: LotMethod,
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

/// SE-completion Chunk C (D1): flip the `business` flag (and optionally `kind`) of an already-imported
/// `Income` event. Corrects the `business: false` hard-code that River (and other adapters) emit at
/// ingest time, enabling SE-tax treatment for professional miners / stakers who cannot use `ClassifyRaw`
/// on Income events.
///
/// **Old-binary limitation:** this variant was added post-initial-release. A vault that CONTAINS a
/// `ReclassifyIncome` event cannot be opened by a binary that predates Chunk C — the same accepted
/// trade-off as every prior decision-type addition (each new variant is a forward-only change). Reading
/// a vault WITHOUT this variant works fine with any binary (serde unknown-variant handling is silent-skip
/// for future variants in old vaults — no data at all, nothing to skip). Voidable via the generic
/// `VoidDecisionEvent`; a second non-voided decision for the same `income_event` → `DecisionConflict`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclassifyIncome {
    /// The target imported Income event whose `business` (and optionally `kind`) is being corrected.
    pub income_event: EventId,
    /// The corrected `business` flag (true = trade-or-business income, subject to SE tax).
    pub business: bool,
    /// Optional kind correction. `None` = keep the original kind; `Some(k)` overrides to `k`.
    #[serde(default)]
    pub kind: Option<IncomeKind>,
}

/// A named-lot selection element (§A.4): consume exactly `sat` from lot `lot`.
/// Used by `PoolSet::consume` (Task 2) and carried by the `LotSelection` decision payload (Task 4).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LotPick {
    pub lot: LotId,
    pub sat: Sat,
}

/// §A.5(a): a dated standing-order method election. `effective_from` binds per-wallet disposals on/after
/// that tax-date to `method`; it CANNOT be back-dated (must be ≥ its made-date and ≥ TRANSITION_DATE, else
/// the `MethodElectionBackdated` hard blocker fires in `resolve`). The latest-in-force election (by
/// `effective_from`, tie `decision_seq`) governs; FIFO is the regulatory default before any election.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodElection {
    pub effective_from: TaxDate,
    pub method: LotMethod,
}

/// §A.4: a per-disposal specific-identification decision. `disposal_event` names the method-honoring
/// disposition (Dispose/GiftOut/Donate/SelfTransfer) whose principal is satisfied by EXACTLY these
/// `lots`. Σ `LotPick.sat` MUST equal the disposal's principal sat — the on-chain `fee_sat` is excluded
/// and consumes FIFO from the post-selection remainder (§A.4(a)). Validated in `resolve` (targeting,
/// principal conservation, duplicate→`DecisionConflict`, voided→excluded) and in the fold (existence,
/// per-wallet, over-draw → hard `LotSelectionInvalid`). On any violation, consumption falls back to
/// method order so Σsat/Σbasis stay conserved. §1.1012-1(j): the identification must exist by the time
/// of sale — `LotSelection` contemporaneity is reported truthfully (never back-dated as compliant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LotSelection {
    pub disposal_event: EventId,
    pub lots: Vec<LotPick>,
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
    MethodElection(MethodElection),
    LotSelection(LotSelection),
    /// SE-completion Chunk C: flip `business` (and optionally `kind`) on an already-imported Income event.
    /// Old-binary limitation: a vault containing this variant cannot be opened by a pre-Chunk-C binary;
    /// see `ReclassifyIncome` struct doc-comment for the full caveat. Reading a vault WITHOUT this variant
    /// (i.e. any vault created before Chunk C) works with any binary — old or new.
    ReclassifyIncome(ReclassifyIncome),
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
            // ---- imported (6 variants) ----
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
            // ---- system (1 variant) ----
            EventPayload::ImportConflict(ImportConflict {
                target: EventId::import(Source::Coinbase, SourceRef::new("Y")),
                new_payload: Box::new(EventPayload::Acquire(Acquire {
                    sat: 50_000,
                    usd_cost: dec!(30.00),
                    fee_usd: dec!(0.75),
                    basis_source: BasisSource::ComputedFromCost,
                })),
                new_fingerprint: Fingerprint::of_bytes(&[1u8; 32]),
            }),
            // ---- decision (9 variants) ----
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new("Z")),
                in_event_or_wallet: TransferTarget::Wallet(crate::identity::WalletId::Exchange {
                    provider: "kraken".into(),
                    account: "trading".into(),
                }),
            }),
            EventPayload::ReclassifyOutflow(ReclassifyOutflow {
                transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("W")),
                as_: OutflowClass::Dispose {
                    kind: DisposeKind::Spend,
                },
                principal_proceeds_or_fmv: dec!(150.00),
                fee_usd: Some(dec!(2.50)),
                donee: None,
            }),
            EventPayload::ClassifyInbound(ClassifyInbound {
                transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("V")),
                as_: InboundClass::Income {
                    kind: IncomeKind::Staking,
                    fmv: Some(dec!(45.50)),
                    business: true,
                },
            }),
            EventPayload::ManualFmv(ManualFmv {
                event: EventId::import(Source::Coinbase, SourceRef::new("U")),
                usd_fmv: dec!(125.75),
            }),
            EventPayload::SafeHarborAllocation(SafeHarborAllocation {
                lots: vec![
                    AllocLot {
                        wallet: crate::identity::WalletId::Exchange {
                            provider: "coinbase".into(),
                            account: "cold".into(),
                        },
                        sat: 50_000,
                        usd_basis: dec!(35.00),
                        acquired_at: time::Date::from_calendar_date(2024, time::Month::January, 15)
                            .unwrap(),
                        dual_loss_basis: None,
                        donor_acquired_at: None,
                    },
                    AllocLot {
                        wallet: crate::identity::WalletId::Exchange {
                            provider: "kraken".into(),
                            account: "main".into(),
                        },
                        sat: 30_000,
                        usd_basis: dec!(21.00),
                        acquired_at: time::Date::from_calendar_date(2024, time::Month::February, 1)
                            .unwrap(),
                        dual_loss_basis: None,
                        donor_acquired_at: None,
                    },
                ],
                as_of_date: time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
                method: AllocMethod::ProRata,
                timely_allocation_attested: true,
                pre2025_method: LotMethod::Fifo,
            }),
            EventPayload::SupersedeImport(SupersedeImport {
                conflict_event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            }),
            EventPayload::RejectImport(RejectImport {
                conflict_event: EventId::import(Source::Coinbase, SourceRef::new("S")),
            }),
            EventPayload::VoidDecisionEvent(VoidDecisionEvent {
                target_event_id: EventId::import(Source::Coinbase, SourceRef::new("R")),
            }),
            EventPayload::ClassifyRaw(ClassifyRaw {
                target: EventId::import(Source::Coinbase, SourceRef::new("Q")),
                as_: Box::new(EventPayload::Income(Income {
                    sat: 100_000,
                    usd_fmv: Some(dec!(65.00)),
                    fmv_status: FmvStatus::ManualEntry,
                    kind: IncomeKind::Mining,
                    business: true,
                })),
            }),
            EventPayload::MethodElection(MethodElection {
                effective_from: time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap(),
                method: crate::LotMethod::Hifo,
            }),
            EventPayload::LotSelection(LotSelection {
                disposal_event: EventId::import(Source::Coinbase, SourceRef::new("P")),
                lots: vec![
                    LotPick {
                        lot: LotId {
                            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("O")),
                            split_sequence: 0,
                        },
                        sat: 75_000,
                    },
                    LotPick {
                        lot: LotId {
                            origin_event_id: EventId::decision(7),
                            split_sequence: 3,
                        },
                        sat: 25_000,
                    },
                ],
            }),
            // SE Chunk C: ReclassifyIncome — with kind override (Some arm)
            EventPayload::ReclassifyIncome(ReclassifyIncome {
                income_event: EventId::import(Source::River, SourceRef::new("in|river-income-001")),
                business: true,
                kind: Some(IncomeKind::Mining),
            }),
            // SE Chunk C: ReclassifyIncome — kind: None (keep original)
            EventPayload::ReclassifyIncome(ReclassifyIncome {
                income_event: EventId::import(Source::River, SourceRef::new("in|river-income-002")),
                business: true,
                kind: None,
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
    fn method_election_decision_has_no_fingerprint() {
        // Global Constraints (§0): new decision events carry `fingerprint = None`.
        let me = EventPayload::MethodElection(MethodElection {
            effective_from: time::Date::from_calendar_date(2025, time::Month::June, 1).unwrap(),
            method: crate::LotMethod::Lifo,
        });
        assert!(crate::persistence::fingerprint(&me).is_none());
    }

    /// [R0-Minor] SE Chunk C KAT: `ReclassifyIncome.fingerprint() == None` (decision variant;
    /// catch-all `_ => None` in `persistence::fingerprint` covers it).
    #[test]
    fn reclassify_income_decision_has_no_fingerprint() {
        let ri = EventPayload::ReclassifyIncome(ReclassifyIncome {
            income_event: EventId::import(Source::Coinbase, SourceRef::new("X")),
            business: true,
            kind: Some(IncomeKind::Mining),
        });
        assert!(crate::persistence::fingerprint(&ri).is_none());
        // Confirm kind=None arm also has no fingerprint.
        let ri_no_kind = EventPayload::ReclassifyIncome(ReclassifyIncome {
            income_event: EventId::import(Source::Coinbase, SourceRef::new("Y")),
            business: false,
            kind: None,
        });
        assert!(crate::persistence::fingerprint(&ri_no_kind).is_none());
    }

    #[test]
    fn lot_selection_decision_has_no_fingerprint() {
        // Global Constraints (§0): new decision events carry `fingerprint = None`.
        let ls = EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new("P")),
            lots: vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("O")),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        });
        assert!(crate::persistence::fingerprint(&ls).is_none());
    }

    #[test]
    fn lot_pick_round_trips() {
        let pick = LotPick {
            lot: LotId {
                origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("L")),
                split_sequence: 2,
            },
            sat: 123_456,
        };
        let json = serde_json::to_string(&pick).unwrap();
        let back: LotPick = serde_json::from_str(&json).unwrap();
        assert_eq!(pick, back);
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

    /// [R0-I1] back-compat KAT: legacy `EventPayload::ReclassifyOutflow` JSON records that predate
    /// the `donee` field (Chunk 2) MUST still deserialize successfully, with `donee: None`.
    ///
    /// This is the "existing vault still opens" guarantee: the `#[serde(default)]` on
    /// `ReclassifyOutflow.donee` makes the field optional in JSON. Crucially, `OutflowClass::GiftOut`
    /// remains a UNIT variant serialized as the bare string `"GiftOut"` — if donee had been added to
    /// the variant itself, legacy `"GiftOut"` records would fail to parse. It lives on the struct.
    #[test]
    fn reclassify_outflow_legacy_json_back_compat_donee_defaults_to_none() {
        // Legacy GiftOut: bare unit-variant string, no donee field.
        // This is exactly the format written by pre-Chunk-2 code.
        // EventId::Import serializes as {"Import": {"source": "Coinbase", "source_ref": "OUT-1"}}.
        let gift_json = r#"{
            "ReclassifyOutflow": {
                "transfer_out_event": {"Import": {"source": "Coinbase", "source_ref": "OUT-1"}},
                "as_": "GiftOut",
                "principal_proceeds_or_fmv": "25000.00",
                "fee_usd": null
            }
        }"#;
        let parsed: EventPayload =
            serde_json::from_str(gift_json).expect("legacy GiftOut JSON must deserialize");
        match parsed {
            EventPayload::ReclassifyOutflow(ro) => {
                assert!(
                    matches!(ro.as_, OutflowClass::GiftOut),
                    "as_ must be GiftOut"
                );
                assert_eq!(
                    ro.donee, None,
                    "donee must be None for legacy records without the field"
                );
            }
            other => panic!("expected ReclassifyOutflow, got {other:?}"),
        }

        // Legacy Donate: struct variant, no donee field.
        let donate_json = r#"{
            "ReclassifyOutflow": {
                "transfer_out_event": {"Import": {"source": "Coinbase", "source_ref": "OUT-2"}},
                "as_": {"Donate": {"appraisal_required": true}},
                "principal_proceeds_or_fmv": "60000.00",
                "fee_usd": null
            }
        }"#;
        let parsed: EventPayload =
            serde_json::from_str(donate_json).expect("legacy Donate JSON must deserialize");
        match parsed {
            EventPayload::ReclassifyOutflow(ro) => {
                assert!(
                    matches!(ro.as_, OutflowClass::Donate { .. }),
                    "as_ must be Donate"
                );
                assert_eq!(
                    ro.donee, None,
                    "donee must be None for legacy Donate records without the field"
                );
            }
            other => panic!("expected ReclassifyOutflow, got {other:?}"),
        }
    }

    #[test]
    fn ledger_event_round_trips_with_non_standard_utc_offset() {
        // Test with non-standard UtcOffset (not half-hour aligned) to pin timezone serde
        let mut ev = sample(EventPayload::Acquire(Acquire {
            sat: 75_000,
            usd_cost: dec!(50.00),
            fee_usd: dec!(0.25),
            basis_source: BasisSource::FmvAtIncome,
        }));
        ev.original_tz = offset!(+05:45); // Nepal Standard Time, unusual offset

        let json = serde_json::to_string(&ev).unwrap();
        let back: LedgerEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, back);
        assert_eq!(back.original_tz, offset!(+05:45));
    }
}
