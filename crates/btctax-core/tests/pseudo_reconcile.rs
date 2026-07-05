//! Pseudo-reconcile mode (sub-project 2) — core KATs.
//!
//! Load-bearing tax-safety invariants proven here:
//!  - mode OFF ⇒ projection is byte-identical (no synthetics; blockers intact);
//!  - a REAL decision on an event ⇒ NO synthetic for it (real supersedes — fault-injected);
//!  - [★ C1] pseudo taint rides the DATA: a REAL Sell consuming a pseudo `$0`-basis lot renders its
//!    disposal leg FLAGGED (`leg.pseudo == true`), never a clean `proceeds − 0`;
//!  - determinism: two pseudo projections are byte-identical;
//!  - synthetics are NEVER events (projection is a pure read of `&[LedgerEvent]`).
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{datetime, offset};

fn w() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w()),
        payload: p,
    }
}
fn transfer_in(rf: &str, ts: time::OffsetDateTime, sat: i64) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        }),
    )
}
fn sell(
    rf: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    proceeds: rust_decimal::Decimal,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
fn unclassified(rf: &str, ts: time::OffsetDateTime) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::Unclassified(Unclassified {
            raw: "weird row".into(),
        }),
    )
}
fn cfg_off() -> ProjectionConfig {
    ProjectionConfig::default()
}
fn cfg_on() -> ProjectionConfig {
    ProjectionConfig {
        pseudo_reconcile: true,
        ..ProjectionConfig::default()
    }
}
fn prices() -> StaticPrices {
    // A generous price map so any FMV lookup (fee/mini-disposition) resolves.
    let mut m = BTreeMap::new();
    for d in [
        time::macros::date!(2025 - 03 - 01),
        time::macros::date!(2025 - 06 - 01),
        time::macros::date!(2025 - 09 - 01),
    ] {
        m.insert(d, dec!(100000)); // $100k/BTC
    }
    StaticPrices(m)
}

/// Mode OFF ⇒ byte-identical to today: the Hard classification blockers are NOT cleared and no lot/leg
/// carries pseudo taint, and no PseudoReconcileActive advisory is added.
#[test]
fn mode_off_is_byte_identical_blockers_intact() {
    let evs = vec![transfer_in(
        "in-1",
        datetime!(2025-03-01 12:00 UTC),
        1_000_000,
    )];
    let st = project(&evs, &prices(), &cfg_off());
    // Unknown-basis inbound stays a HARD blocker (today's behavior).
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert_eq!(st.pseudo_synthetic_count, 0);
    assert!(!st.pseudo_active());
    assert!(!st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::PseudoReconcileActive));
    // No lot was created for the unknown-basis inbound (FR9/§7.3).
    assert!(st.lots.is_empty());
}

/// Mode ON clears the unknown-basis inbound: it becomes a $0-basis self-transfer lot, flagged pseudo,
/// and the Hard `UnknownBasisInbound` blocker is gone; a loud advisory + a synthetic count are present.
#[test]
fn mode_on_clears_unknown_basis_inbound_with_pseudo_lot() {
    let evs = vec![transfer_in(
        "in-1",
        datetime!(2025-03-01 12:00 UTC),
        1_000_000,
    )];
    let st = project(&evs, &prices(), &cfg_on());
    assert!(!st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    assert_eq!(st.lots.len(), 1);
    let lot = &st.lots[0];
    assert_eq!(lot.usd_basis, dec!(0)); // conservative $0 (max eventual gain)
    assert!(
        lot.pseudo,
        "the synthetic self-transfer lot must be flagged pseudo"
    );
    assert_eq!(lot.basis_source, BasisSource::SelfTransferInbound);
    assert_eq!(st.pseudo_synthetic_count, 1);
    assert!(st.pseudo_active());
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::PseudoReconcileActive));
}

/// [★ C1 — the headline correctness point] A REAL imported Sell consuming a pseudo `$0`-basis lot MUST
/// render its disposal leg FLAGGED (`leg.pseudo == true`) — never a clean `proceeds − 0` treated as real.
#[test]
fn real_sell_on_pseudo_lot_flags_the_disposal_leg() {
    let evs = vec![
        transfer_in("in-1", datetime!(2025-03-01 12:00 UTC), 1_000_000),
        // A REAL Sell of the same coins in the same wallet, later.
        sell(
            "sell-1",
            datetime!(2025-06-01 12:00 UTC),
            1_000_000,
            dec!(900),
        ),
    ];
    let st = project(&evs, &prices(), &cfg_on());
    assert_eq!(
        st.disposals.len(),
        1,
        "the real Sell is still a taxable disposal"
    );
    let leg = &st.disposals[0].legs[0];
    assert_eq!(leg.basis, dec!(0)); // consumed the pseudo $0-basis lot
    assert_eq!(leg.gain, dec!(900)); // proceeds − 0 = max gain
    assert!(
        leg.pseudo,
        "a real Sell on a pseudo $0-basis lot MUST flag the leg [PSEUDO], not present it as clean"
    );
}

/// Real supersedes pseudo (fault-inject the precedence): a real `ClassifyInbound(SelfTransferMine)` with
/// a REAL basis on the inbound ⇒ NO synthetic is injected for it (lot carries the real basis, NOT pseudo).
#[test]
fn real_decision_supersedes_no_synthetic_injected() {
    let real_class = LedgerEvent {
        id: EventId::decision(0),
        utc_timestamp: datetime!(2025-03-02 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: EventId::import(Source::Coinbase, SourceRef::new("in-1")),
            as_: InboundClass::SelfTransferMine {
                basis: Some(dec!(500)), // a REAL, attested basis
                acquired_at: None,
            },
        }),
    };
    let evs = vec![
        transfer_in("in-1", datetime!(2025-03-01 12:00 UTC), 1_000_000),
        real_class,
    ];
    let st = project(&evs, &prices(), &cfg_on());
    assert_eq!(st.lots.len(), 1);
    let lot = &st.lots[0];
    assert_eq!(
        lot.usd_basis,
        dec!(500),
        "real basis governs, not the pseudo $0"
    );
    assert!(!lot.pseudo, "a real decision ⇒ NO synthetic ⇒ NOT pseudo");
    // No synthetic contributed at all.
    assert_eq!(st.pseudo_synthetic_count, 0);
    assert!(!st.pseudo_active());
}

/// Unclassified (determinable-inbound) is cleared by a `ClassifyRaw` zero-value placeholder — the row
/// carries no structured amount, so no holdings are fabricated, but the Hard `Unclassified` blocker is gone.
#[test]
fn unclassified_inbound_cleared_via_classify_raw_placeholder() {
    let evs = vec![unclassified("u-1", datetime!(2025-03-01 12:00 UTC))];
    let off = project(&evs, &prices(), &cfg_off());
    assert!(off
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::Unclassified));
    let on = project(&evs, &prices(), &cfg_on());
    assert!(
        !on.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::Unclassified),
        "pseudo clears the Unclassified classification blocker"
    );
    assert_eq!(on.pseudo_synthetic_count, 1);
    assert!(
        on.lots.is_empty(),
        "a 0-amount placeholder fabricates no holdings"
    );
}

/// A wallet-less `Unclassified` has nowhere to home a lot ⇒ LEFT SURFACED (not cleared) even in pseudo mode.
#[test]
fn walletless_unclassified_left_surfaced() {
    let mut ev = unclassified("u-1", datetime!(2025-03-01 12:00 UTC));
    ev.wallet = None;
    let evs = vec![ev];
    let st = project(&evs, &prices(), &cfg_on());
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind == BlockerKind::Unclassified),
        "a wallet-less Unclassified stays surfaced (no synthetic)"
    );
    assert_eq!(st.pseudo_synthetic_count, 0);
}

/// Per-default [R0-C2]: an unresolved `ImportConflict` is cleared by accept-first — the first-seen
/// conflict's new payload governs its target, the target lot is flagged pseudo, and the Hard
/// `ImportConflict` blocker is gone.
#[test]
fn import_conflict_cleared_via_accept_first() {
    let target_id = EventId::import(Source::Coinbase, SourceRef::new("a-1"));
    // The original import: Acquire 1 BTC @ $100 basis.
    let original = imp(
        "a-1",
        datetime!(2025-03-01 12:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 1_000_000,
            usd_cost: dec!(100),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    // A re-import that CONFLICTS (different cost $700) → an unresolved ImportConflict.
    let new_payload = EventPayload::Acquire(Acquire {
        sat: 1_000_000,
        usd_cost: dec!(700),
        fee_usd: dec!(0),
        basis_source: BasisSource::ExchangeProvided,
    });
    let fp = Fingerprint::of_bytes(b"new-content");
    let conflict = LedgerEvent {
        id: EventId::conflict(Source::Coinbase, SourceRef::new("a-1"), &fp),
        utc_timestamp: datetime!(2025-03-01 12:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(w()),
        payload: EventPayload::ImportConflict(ImportConflict {
            target: target_id,
            new_payload: Box::new(new_payload),
            new_fingerprint: fp.clone(),
        }),
    };
    let evs = vec![original, conflict];

    // Mode OFF ⇒ the ImportConflict is a Hard blocker (today's behavior); the ORIGINAL $100 basis stands.
    let off = project(&evs, &prices(), &cfg_off());
    assert!(off
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::ImportConflict));
    assert_eq!(off.lots[0].usd_basis, dec!(100));
    assert!(!off.lots[0].pseudo);

    // Mode ON ⇒ accept-first adopts the new $700 payload onto the target, flagged pseudo, blocker gone.
    let on = project(&evs, &prices(), &cfg_on());
    assert!(!on
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::ImportConflict));
    assert_eq!(on.lots.len(), 1);
    assert_eq!(
        on.lots[0].usd_basis,
        dec!(700),
        "accept-first adopted the new payload"
    );
    assert!(
        on.lots[0].pseudo,
        "the accepted-first target lot is flagged pseudo"
    );
    assert_eq!(on.pseudo_synthetic_count, 1);
}

/// Determinism (NFR4): two pseudo projections of the same ledger are byte-identical (PartialEq on the
/// whole `LedgerState`, incl. the new `pseudo` bits and the synthetic count).
#[test]
fn pseudo_projection_is_deterministic() {
    let evs = vec![
        transfer_in("in-1", datetime!(2025-03-01 12:00 UTC), 1_000_000),
        unclassified("u-1", datetime!(2025-03-01 13:00 UTC)),
        sell(
            "sell-1",
            datetime!(2025-06-01 12:00 UTC),
            400_000,
            dec!(500),
        ),
    ];
    let a = project(&evs, &prices(), &cfg_on());
    let b = project(&evs, &prices(), &cfg_on());
    assert_eq!(
        a, b,
        "identical (events, prices, config) ⇒ identical projection"
    );
}

// ── I2-precise: "0 Hard classification blockers; a tax TOTAL only at 0 Hard total" ───────────────
// Pseudo clears the Hard *classification* kinds it can honestly default, but a tax number computes
// only when 0 Hard blockers of ANY kind remain (compute_tax_year returns NotComputable on the first
// Hard blocker). Excluded kinds (native-Income FmvMissing, UncoveredDisposal, DecisionConflict, …) are
// NOT cleared and still gate the total.
use btctax_core::tax::compute::compute_tax_year;
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};

struct OneTable(TaxTable);
impl TaxTables for OneTable {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        (year == self.0.year).then_some(&self.0)
    }
}
fn synth_table(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![OrdinaryBracket {
                lower: dec!(0),
                rate: dec!(0.10),
            }],
        },
    );
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(40000),
            max_fifteen: dec!(400000),
        },
    );
    OneTable(TaxTable {
        year,
        source: "SYNTHETIC",
        ordinary,
        ltcg,
        gift_annual_exclusion: dec!(19000),
        ss_wage_base: dec!(176100),
        gift_lifetime_exclusion: dec!(13_990_000),
    })
}
fn single_zero_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(0),
        magi_excluding_crypto: dec!(0),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    }
}
fn native_income_fmv_missing(rf: &str, ts: time::OffsetDateTime, sat: i64) -> LedgerEvent {
    imp(
        rf,
        ts,
        EventPayload::Income(Income {
            sat,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Mining,
            business: false,
        }),
    )
}

/// With pseudo ON and the ONLY Hard blockers being classification kinds it clears, a tax TOTAL computes
/// (I2). NOTE (M1 residual): the total is HIGH, not zero — the real Sell consumes a pseudo `$0`-basis lot
/// so gain = proceeds − 0. Clearing all Hard blockers makes *a* total assertable, not a ≈zero one.
#[test]
fn tax_total_computes_when_pseudo_clears_all_hard_blockers() {
    let evs = vec![
        transfer_in("in-1", datetime!(2025-03-01 12:00 UTC), 1_000_000),
        sell(
            "sell-1",
            datetime!(2025-06-01 12:00 UTC),
            1_000_000,
            dec!(900),
        ),
    ];
    let st = project(&evs, &prices(), &cfg_on());
    let out = compute_tax_year(
        &evs,
        &st,
        2025,
        Some(&single_zero_profile()),
        &synth_table(2025),
    );
    match out {
        TaxOutcome::Computed(r) => {
            // A number IS produced — and it is not zero (the pseudo $0-basis Sell realizes gain).
            assert!(r.total_federal_tax_attributable >= dec!(0));
            assert!(
                r.st_net > dec!(0),
                "the $0-basis Sell produced a positive ST gain"
            );
        }
        TaxOutcome::NotComputable(b) => {
            panic!("expected a computable total once pseudo cleared all Hard blockers, got {b:?}")
        }
    }
}

/// A native-`Income` `FmvMissing` is a Hard blocker pseudo does NOT clear (pseudo defaults only inbound
/// TransferIns, never invents income). So even with the classification blocker cleared, NO total computes
/// — proving "a tax TOTAL only at 0 Hard total" (I2-precise).
#[test]
fn no_tax_total_while_a_non_classification_hard_blocker_remains() {
    let evs = vec![
        transfer_in("in-1", datetime!(2025-03-01 12:00 UTC), 1_000_000),
        native_income_fmv_missing("inc-1", datetime!(2025-03-01 13:00 UTC), 500_000),
    ];
    let st = project(&evs, &prices(), &cfg_on());
    // The classification blocker IS cleared…
    assert!(!st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::UnknownBasisInbound));
    // …but the native-Income FmvMissing remains Hard and is NOT cleared.
    assert!(st
        .blockers
        .iter()
        .any(|b| b.kind == BlockerKind::FmvMissing));
    let out = compute_tax_year(
        &evs,
        &st,
        2025,
        Some(&single_zero_profile()),
        &synth_table(2025),
    );
    assert!(
        matches!(out, TaxOutcome::NotComputable(_)),
        "no tax total while ANY Hard blocker (native-Income FmvMissing) remains"
    );
}
