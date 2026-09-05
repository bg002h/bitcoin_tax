//! FR-38 (FOLLOWUPS.md): a structurally-impossible value — a negative cost basis, FMV, proceeds,
//! fee, or satoshi quantity — must never become a persisted event row, **from any door**.
//!
//! FR-38 was filed as a `classify-raw` finding. The 2026-09-05 recon
//! (`design/agent-reports/2026-09-05-recon-fr38-validation-seam.md`) established it is wider: the
//! CLI JSON door, the TUI `classify-raw` form, three of four CSV adapters, and `accept-conflict`
//! (which has no code of its own) all admit one. The fix is therefore NOT another call site but the
//! **persistence boundary** — `persistence::insert`, the workspace's sole `INSERT INTO events`
//! (verified by grep) — through which every one of those doors necessarily passes.
//!
//! These are the SEAM tests: they exercise the two public append functions directly, so they hold
//! the guarantee for doors that do not exist yet. The per-door tests live in
//! `btctax-cli/tests/fr38_payload_polarity_cli.rs` (CLI + CSV import) and in
//! `btctax-tui-edit`'s form tests (the TUI `classify-raw` path).
//!
//! ★ B1 kill-test: delete the `check_payload_polarity(&ev.payload)?` line at the top of
//! `persistence::insert` and every `*_is_refused` test here goes RED.
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::persistence;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};
use time::{OffsetDateTime, UtcOffset};

fn conn() -> rusqlite::Connection {
    let c = rusqlite::Connection::open_in_memory().unwrap();
    persistence::init_schema(&c).unwrap();
    c
}

fn wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}

/// An imported `Acquire` event with a caller-chosen payload.
fn imported(source_ref: &str, payload: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(source_ref)),
        utc_timestamp: datetime!(2025-03-01 12:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(wallet()),
        payload,
    }
}

fn acquire(usd_cost: btctax_core::Usd) -> EventPayload {
    EventPayload::Acquire(Acquire {
        sat: 100_000,
        usd_cost,
        fee_usd: dec!(1.00),
        basis_source: BasisSource::ExchangeProvided,
    })
}

fn decide(c: &rusqlite::Connection, p: EventPayload) -> Result<EventId, btctax_core::CoreError> {
    persistence::append_decision(c, p, OffsetDateTime::now_utc(), UtcOffset::UTC, None)
}

/// The refusal must name the offending field, so the filer can find it. (Not merely "an error".)
fn assert_refusal_names(e: &btctax_core::CoreError, field: &str) {
    let s = e.to_string();
    assert!(
        s.contains(field),
        "the refusal must name the field {field:?}: {s}"
    );
}

// ── the imported payloads (D1 CSV import, D2 CLI classify-raw, D5 TUI classify-raw) ──────────

#[test]
fn a_negative_acquire_usd_cost_is_refused_and_nothing_persists() {
    let c = conn();
    let err = persistence::append_import_batch(&c, &[imported("A", acquire(dec!(-1.00)))])
        .expect_err("a negative usd_cost must be refused at the persistence boundary");
    assert_refusal_names(&err, "usd_cost");
    assert!(
        persistence::load_all(&c).unwrap().is_empty(),
        "the refused batch must persist nothing (append_import_batch is atomic)"
    );
}

#[test]
fn a_negative_acquire_fee_usd_is_refused() {
    let c = conn();
    let err = persistence::append_import_batch(
        &c,
        &[imported(
            "A",
            EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(60.00),
                fee_usd: dec!(-0.50),
                basis_source: BasisSource::ExchangeProvided,
            }),
        )],
    )
    .expect_err("a negative fee_usd must be refused");
    assert_refusal_names(&err, "fee_usd");
}

#[test]
fn a_negative_income_usd_fmv_is_refused() {
    let c = conn();
    let err = persistence::append_import_batch(
        &c,
        &[imported(
            "A",
            EventPayload::Income(Income {
                sat: 50_000,
                usd_fmv: Some(dec!(-30.00)),
                fmv_status: FmvStatus::ManualEntry,
                kind: IncomeKind::Reward,
                business: false,
            }),
        )],
    )
    .expect_err("a negative usd_fmv must be refused");
    assert_refusal_names(&err, "usd_fmv");
}

#[test]
fn a_negative_dispose_usd_proceeds_is_refused() {
    let c = conn();
    let err = persistence::append_import_batch(
        &c,
        &[imported(
            "A",
            EventPayload::Dispose(Dispose {
                sat: 25_000,
                usd_proceeds: dec!(-40.00),
                fee_usd: dec!(0.50),
                kind: DisposeKind::Sell,
            }),
        )],
    )
    .expect_err("a negative usd_proceeds must be refused");
    assert_refusal_names(&err, "usd_proceeds");
}

#[test]
fn a_negative_sat_is_refused() {
    let c = conn();
    let err = persistence::append_import_batch(
        &c,
        &[imported(
            "A",
            EventPayload::TransferIn(TransferIn {
                sat: -10_000,
                src_addr: None,
                txid: None,
            }),
        )],
    )
    .expect_err("a negative sat must be refused");
    assert_refusal_names(&err, "sat");
}

#[test]
fn a_negative_transfer_out_fee_sat_is_refused() {
    let c = conn();
    let err = persistence::append_import_batch(
        &c,
        &[imported(
            "A",
            EventPayload::TransferOut(TransferOut {
                sat: 10_000,
                fee_sat: Some(-150),
                dest_addr: None,
                txid: None,
            }),
        )],
    )
    .expect_err("a negative fee_sat must be refused");
    assert_refusal_names(&err, "fee_sat");
}

// ── the recursive carriers (D2/D5 classify-raw, D6 accept-conflict) ──────────────────────────

/// `ClassifyRaw` is the FR-38 door as filed: the impossible value rides inside `as_`, one box down.
#[test]
fn a_negative_usd_inside_classify_raw_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::ClassifyRaw(ClassifyRaw {
            target: EventId::import(Source::Coinbase, SourceRef::new("T")),
            as_: Box::new(acquire(dec!(-5000.00))),
        }),
    )
    .expect_err("classify-raw must not smuggle a negative usd_cost past the guard");
    assert_refusal_names(&err, "usd_cost");
    assert!(
        persistence::load_all(&c).unwrap().is_empty(),
        "the refused decision must persist nothing"
    );
}

/// `accept-conflict` has no validation code of its own: an `ImportConflict` row is what later gets
/// promoted into force, so the check must reach `new_payload`. Only this seam covers that door.
#[test]
fn a_negative_usd_inside_an_import_conflict_new_payload_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::ImportConflict(ImportConflict {
            target: EventId::import(Source::Coinbase, SourceRef::new("T")),
            new_payload: Box::new(acquire(dec!(-1.00))),
            new_fingerprint: Fingerprint::of_bytes(&[7u8; 32]),
        }),
    )
    .expect_err("an ImportConflict must not carry an impossible new_payload");
    assert_refusal_names(&err, "usd_cost");
}

// ── the decision payloads ────────────────────────────────────────────────────────────────────

#[test]
fn a_negative_manual_fmv_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::ManualFmv(ManualFmv {
            event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            usd_fmv: dec!(-1.00),
        }),
    )
    .expect_err("a negative ManualFmv must be refused");
    assert_refusal_names(&err, "usd_fmv");
}

#[test]
fn a_negative_reclassify_outflow_amount_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Spend,
            },
            principal_proceeds_or_fmv: dec!(-150.00),
            fee_usd: None,
            donee: None,
        }),
    )
    .expect_err("a negative reclassified-outflow amount must be refused");
    assert_refusal_names(&err, "principal_proceeds_or_fmv");
}

#[test]
fn a_negative_gift_donor_basis_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            as_: InboundClass::GiftReceived {
                donor_basis: Some(dec!(-10.00)),
                donor_acquired_at: None,
                fmv_at_gift: dec!(100.00),
            },
        }),
    )
    .expect_err("a negative §1015(a) donor basis must be refused");
    assert_refusal_names(&err, "donor_basis");
}

#[test]
fn a_negative_self_transfer_basis_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            as_: InboundClass::SelfTransferMine {
                basis: Some(dec!(-1.00)),
                acquired_at: None,
            },
        }),
    )
    .expect_err("a negative self-transfer basis must be refused");
    assert_refusal_names(&err, "basis");
}

#[test]
fn a_negative_safe_harbor_lot_basis_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots: vec![AllocLot {
                wallet: wallet(),
                sat: 50_000,
                usd_basis: dec!(-35.00),
                acquired_at: date!(2024 - 01 - 15),
                dual_loss_basis: None,
                donor_acquired_at: None,
            }],
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ProRata,
            timely_allocation_attested: true,
            pre2025_method: btctax_core::LotMethod::Fifo,
        }),
    )
    .expect_err("a negative allocated lot basis must be refused");
    assert_refusal_names(&err, "usd_basis");
}

#[test]
fn a_negative_declared_tranche_sat_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::DeclareTranche(DeclareTranche {
            sat: -1,
            wallet: wallet(),
            window_start: date!(2018 - 01 - 01),
            window_end: date!(2018 - 12 - 31),
        }),
    )
    .expect_err("a negative declared-tranche sat must be refused");
    assert_refusal_names(&err, "sat");
}

#[test]
fn a_negative_lot_selection_sat_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            lots: vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::import(Source::Coinbase, SourceRef::new("O")),
                    split_sequence: 0,
                },
                sat: -5,
            }],
            attested: false,
        }),
    )
    .expect_err("a negative named-lot sat must be refused");
    assert_refusal_names(&err, "sat");
}

// ── [G-I5]: the legitimately SIGNED fields must still record ─────────────────────────────────

/// `ConsentTerm`'s four delta fields are DIFFERENCES (`conservative_promote.rs`:
/// `let delta_usd = t_without - t_with;`) — a promotion that RAISES tax yields a negative, and that
/// is the truthful figure shown to the filer. A blanket "every `Usd` >= 0" would refuse a correct
/// §6664(c) good-faith record. This is the repo's own `[G-I5]` ruling — guard per FIELD, never
/// per TYPE — and this test is what stops a future "tidy-up" from making the rule blanket.
///
/// ★ Mutation: change the `ConsentTerm` arms to run the non-negative check and this goes RED.
#[test]
fn a_negative_consent_term_delta_is_accepted_because_deltas_are_signed() {
    let c = conn();
    let id = decide(
        &c,
        EventPayload::PromoteTranche(PromoteTranche {
            target: EventId::decision(1),
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(1234.00),
            coverage: btctax_core::conservative::Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I ACCEPT".into(),
                shown_terms: vec![
                    ConsentTerm::ComputedTax {
                        year: 2024,
                        delta_usd: dec!(-500.00),
                        deduction_delta_usd: Some(dec!(-25.00)),
                    },
                    ConsentTerm::Uncomputable {
                        year: 2023,
                        gain_delta_usd: dec!(-1000.00),
                        deduction_delta_usd: dec!(-10.00),
                    },
                ],
                provenance_text: "…".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "…".into(),
        }),
    )
    .expect("a signed ConsentTerm delta is legitimate and must record [G-I5]");
    assert_eq!(persistence::load_all(&c).unwrap().len(), 1, "{id:?}");
}

/// …but `Unrealized`'s two non-delta fields are NOT differences: `sat` is an undisposed quantity
/// and `hypothetical_reduction` a reduction magnitude. The per-field split has to hold INSIDE the
/// same enum, which is precisely what a per-type rule cannot express.
#[test]
fn a_negative_unrealized_hypothetical_reduction_is_refused() {
    let c = conn();
    let err = decide(
        &c,
        EventPayload::PromoteTranche(PromoteTranche {
            target: EventId::decision(1),
            method: FloorMethod::WindowLowClose,
            filed_basis: dec!(1234.00),
            coverage: btctax_core::conservative::Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I ACCEPT".into(),
                shown_terms: vec![ConsentTerm::Unrealized {
                    sat: 100,
                    hypothetical_reduction: Some(dec!(-5.00)),
                    as_of: None,
                }],
                provenance_text: "…".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "…".into(),
        }),
    )
    .expect_err("a negative hypothetical reduction is not a delta and must be refused");
    assert_refusal_names(&err, "hypothetical_reduction");
}

// ── the not-over-refusing controls ───────────────────────────────────────────────────────────

/// ZERO must stay legal everywhere: `$0` basis is this application's conservative default for an
/// undocumented tranche AND for an inbound self-transfer, and a `$0` fee is the common case. A
/// guard that refused zero would refuse the app's own filing posture.
#[test]
fn zero_usd_and_zero_sat_still_record() {
    let c = conn();
    persistence::append_import_batch(
        &c,
        &[imported(
            "A",
            EventPayload::Acquire(Acquire {
                sat: 0,
                usd_cost: dec!(0),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        )],
    )
    .expect("zero is a legitimate value at every guarded field");
    decide(
        &c,
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("T")),
            as_: InboundClass::SelfTransferMine {
                basis: Some(dec!(0)),
                acquired_at: None,
            },
        }),
    )
    .expect("an attested $0 self-transfer basis is the conservative default and must record");
    assert_eq!(persistence::load_all(&c).unwrap().len(), 2);
}

/// The ordinary happy path is untouched: this is a record-time gate, not a change of behaviour.
#[test]
fn an_ordinary_import_is_unaffected() {
    let c = conn();
    let r = persistence::append_import_batch(
        &c,
        &[
            imported("A", acquire(dec!(60.00))),
            imported("B", acquire(dec!(61.00))),
        ],
    )
    .unwrap();
    assert_eq!(r.appended, 2);
    assert_eq!(persistence::load_all(&c).unwrap().len(), 2);
}

/// Read-side is deliberately NOT guarded: the invariant is forward-only. A vault that somehow
/// already holds an impossible value must still LOAD (so it can be inspected and voided) — a
/// `deserialize_with` guard would instead make it unloadable, which is the failure mode the recon
/// rejected candidate (b) for. This test pins that choice: it writes the row behind the seam.
#[test]
fn load_all_still_reads_a_pre_existing_impossible_row() {
    let c = conn();
    let ev = imported("A", acquire(dec!(-1.00)));
    c.execute(
        "INSERT INTO events (event_id, kind, source, source_ref, decision_seq, utc_timestamp, \
         tz_offset_sec, wallet_json, payload_json, fingerprint) \
         VALUES (?1,'import','coinbase','A',NULL,'2025-03-01T12:00:00Z',0,NULL,?2,'x')",
        rusqlite::params![
            ev.id.canonical(),
            serde_json::to_string(&ev.payload).unwrap()
        ],
    )
    .unwrap();
    assert_eq!(
        persistence::load_all(&c).unwrap().len(),
        1,
        "a record-time gate must never make an existing vault unloadable"
    );
}
