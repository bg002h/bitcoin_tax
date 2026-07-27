//! Conservative-filing Phase 2 (P2 / D-9) — CHARACTERIZATION pins: steered matching is EMERGENT
//! under HIFO, and its FIFO inversion. No new matching code exists (nor should any be added) — the
//! SPEC's claim is that HIFO's existing `hifo_cmp` already sorts `usd_basis == 0` lots LAST
//! (`pools.rs`), so a sale naturally draws the documented (higher-basis) lot before the $0 tranche.
//! These tests PIN that dependence; per the plan they are the one case where passing-on-write is
//! correct. If either FAILS on write the emergence assumption is wrong — STOP, do not add matching code.
//!
//! Method-staging (arch M-2): `config.pre2025_method` governs ONLY pre-2025-dated disposals; post-2025
//! method comes from `MethodElection` and defaults to HIFO. Both tests therefore stage the tranche +
//! documented buy + sale ALL pre-2025, so the disposal routes through the Universal pool under the
//! config's `pre2025_method`. Years are pinned explicitly so the fixtures aren't a confusing RED.
//! PRIVACY: synthetic values only.

use btctax_core::conservative::{
    basis_methodology, method_inversion_advisory, overpayment_delta, self_custody_nudge,
    tranche_broker_specific_id_advisory, tranche_dip_advisory, tranche_report_advisory,
    window_reference, Coverage,
};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::{DisposalLeg, LedgerState};
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxProfile};
use btctax_core::LotMethod;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

// ── tax profile + table fixtures (mirror tests/whatif.rs `synth`/`profile`) ─────────────────────────
struct OneTable(TaxTable);
impl TaxTables for OneTable {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        (year == self.0.year).then_some(&self.0)
    }
}
fn synth(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.10),
                },
                OrdinaryBracket {
                    lower: dec!(50000),
                    rate: dec!(0.22),
                },
                OrdinaryBracket {
                    lower: dec!(250000),
                    rate: dec!(0.32),
                },
            ],
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
/// A Single filer with ordinary taxable income `ord` (so a crypto ST gain stacks at the ordinary rate).
fn tax_profile(ord: i64) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: rust_decimal::Decimal::from(ord),
        magi_excluding_crypto: rust_decimal::Decimal::from(ord),
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

// ── fixtures (mirror tests/kat_tranche.rs) ──────────────────────────────────────────────────────────
fn exch() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn self_custody() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn prices() -> StaticPrices {
    StaticPrices::default()
}
/// A deterministic date→close map for the P5 `window_reference` KATs.
fn priced(entries: &[(time::Date, i64)]) -> StaticPrices {
    StaticPrices(
        entries
            .iter()
            .map(|(d, p)| (*d, rust_decimal::Decimal::from(*p)))
            .collect(),
    )
}
/// The default projection config (post-2025 method = HIFO default; used for the ≥2025 P4 disposals).
fn config() -> ProjectionConfig {
    ProjectionConfig::default()
}
fn config_hifo_pre2025() -> ProjectionConfig {
    ProjectionConfig {
        pre2025_method: LotMethod::Hifo,
        ..ProjectionConfig::default()
    }
}
fn config_fifo_pre2025() -> ProjectionConfig {
    ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        ..ProjectionConfig::default()
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, w: &WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: p,
    }
}
/// A DOCUMENTED buy: `ExchangeProvided` basis > $0 at the given tax-date.
fn documented_buy(
    rf: &str,
    ts: time::OffsetDateTime,
    w: &WalletId,
    sat: i64,
    cost: i64,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: rust_decimal::Decimal::from(cost),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}
/// A `DeclareTranche` decision homed at `window_end` ($0 `EstimatedConservative`).
fn tranche_ev(seq: u64, w: &WalletId, sat: i64, ws: time::Date, we: time::Date) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: datetime!(2026-01-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::DeclareTranche(DeclareTranche {
            sat,
            wallet: w.clone(),
            window_start: ws,
            window_end: we,
        }),
    }
}
fn sell_ev(
    rf: &str,
    ts: time::OffsetDateTime,
    w: &WalletId,
    sat: i64,
    proceeds: i64,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: rust_decimal::Decimal::from(proceeds),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
/// The single disposal leg a partial sale produces (exactly one lot consumed).
fn only_disposal_leg(st: &LedgerState) -> &DisposalLeg {
    let legs: Vec<&DisposalLeg> = st.disposals.iter().flat_map(|d| &d.legs).collect();
    assert_eq!(
        legs.len(),
        1,
        "a partial sale under a single method draws exactly one lot"
    );
    legs[0]
}

/// P2 / D-9: under HIFO a partial sale draws the DOCUMENTED (higher-basis) lot before the $0 tranche —
/// steered matching is emergent from `hifo_cmp` sorting `usd_basis == 0` LAST. The tranche is the more
/// recent window (2018) than the buy would matter under FIFO, but HIFO keys on basis, so the $0 tranche
/// is consumed LAST regardless of date. Higher basis used first = the conservative gain outcome.
#[test]
fn under_hifo_a_sale_draws_the_documented_lot_before_the_zero_basis_tranche() {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2019-01-01 00:00 UTC),
        &w,
        100_000_000,
        30_000,
    );
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2020-06-01 00:00 UTC),
        &w,
        50_000_000,
        40_000,
    );
    let st = project(&[buy, t, sell], &prices(), &config_hifo_pre2025());
    let leg = only_disposal_leg(&st);
    assert_ne!(
        leg.basis_source,
        BasisSource::EstimatedConservative,
        "HIFO draws the documented lot first; the $0 tranche sorts LAST (P2 emergent)"
    );
}

/// P2 / D-9: the FIFO INVERSION — under FIFO an OLD $0 tranche (early `window_end`) is consumed FIRST,
/// a gain-maximizing outcome. This is the correct application of the in-force method (never an
/// understatement — a $0-basis lot maximizes reported gain), and the reason P3's method-inversion
/// advisory exists. Same fixture as above but the tranche is the OLDEST lot and the method is FIFO.
#[test]
fn under_fifo_the_old_zero_basis_tranche_is_consumed_first_inversion() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2015 - 01 - 01),
        date!(2015 - 12 - 31),
    );
    let buy = documented_buy(
        "BUY",
        datetime!(2019-01-01 00:00 UTC),
        &w,
        100_000_000,
        30_000,
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2020-06-01 00:00 UTC),
        &w,
        50_000_000,
        40_000,
    );
    let st = project(&[t, buy, sell], &prices(), &config_fifo_pre2025());
    let leg = only_disposal_leg(&st);
    assert_eq!(
        leg.basis_source,
        BasisSource::EstimatedConservative,
        "FIFO consumes the OLDEST lot first — the old $0 tranche (window_end 2015) — an inversion (D-9)"
    );
}

// ── Phase 3 / Task 9: dip + method-inversion advisory builders (D-9) ────────────────────────────────

/// P3 / D-9: `tranche_dip_advisory` fires for a disposal that consumed a tranche leg; it states the
/// basis AS FILED (`$0` here — printed from `leg.basis`, never hard-coded) and the resulting gain, and
/// is provenance-neutral (never "purchase"/"bought" — a tranche is undocumented BTC, not a known buy).
#[test]
fn dip_advisory_fires_states_basis_as_filed_and_is_provenance_neutral() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2020-06-01 00:00 UTC),
        &w,
        50_000_000,
        40_000,
    );
    let st = project(&[t, sell], &prices(), &config_hifo_pre2025());
    let disposal = st.disposals.first().expect("a disposal");
    let adv = tranche_dip_advisory(disposal).expect("a tranche disposal produces a dip advisory");
    assert!(adv.contains("$0"), "basis AS FILED ($0) must appear: {adv}");
    assert!(
        adv.contains("40000"),
        "the reported gain must appear: {adv}"
    );
    let low = adv.to_lowercase();
    assert!(
        !low.contains("purchase") && !low.contains("bought"),
        "provenance-neutral: no purchase/bought (tax min-8c): {adv}"
    );
}

/// P3: no dip advisory for a fully-documented disposal (no tranche leg consumed).
#[test]
fn dip_advisory_absent_for_a_fully_documented_disposal() {
    let w = exch();
    let buy = documented_buy(
        "BUY",
        datetime!(2019-01-01 00:00 UTC),
        &w,
        100_000_000,
        30_000,
    );
    let sell = sell_ev(
        "SELL",
        datetime!(2020-06-01 00:00 UTC),
        &w,
        50_000_000,
        40_000,
    );
    let st = project(&[buy, sell], &prices(), &config_hifo_pre2025());
    let disposal = st.disposals.first().expect("a disposal");
    assert!(
        tranche_dip_advisory(disposal).is_none(),
        "a documented disposal must not produce a dip advisory"
    );
}

/// P3 / D-9: `method_inversion_advisory` fires when a NON-HIFO in-force method could consume a $0
/// tranche lot while a documented lot remains in the same wallet (the gain-maximizing inversion), and
/// recommends a HIFO election. State = a tranche lot + a documented lot both remaining (no sale yet).
#[test]
fn inversion_advisory_fires_for_a_non_hifo_method_when_both_lot_kinds_remain() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let buy = documented_buy(
        "BUY",
        datetime!(2019-01-01 00:00 UTC),
        &w,
        100_000_000,
        30_000,
    );
    let st = project(&[t, buy], &prices(), &config_fifo_pre2025());
    let adv = method_inversion_advisory(&st, &w, LotMethod::Fifo)
        .expect("a non-HIFO method with both lot kinds produces an inversion advisory");
    assert!(
        adv.to_uppercase().contains("HIFO"),
        "the advisory recommends a HIFO election: {adv}"
    );
}

/// P3: no inversion advisory under HIFO (HIFO already steers documented-first — no inversion).
#[test]
fn inversion_advisory_absent_under_hifo() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let buy = documented_buy(
        "BUY",
        datetime!(2019-01-01 00:00 UTC),
        &w,
        100_000_000,
        30_000,
    );
    let st = project(&[t, buy], &prices(), &config_hifo_pre2025());
    assert!(
        method_inversion_advisory(&st, &w, LotMethod::Hifo).is_none(),
        "HIFO does not invert — no advisory"
    );
}

/// P3: no inversion advisory when the wallet holds NO documented lot (nothing to draw before the
/// tranche — the inversion needs both a $0 tranche lot AND a documented lot present).
#[test]
fn inversion_advisory_absent_without_a_documented_lot() {
    let w = exch();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let st = project(&[t], &prices(), &config_fifo_pre2025());
    assert!(
        method_inversion_advisory(&st, &w, LotMethod::Fifo).is_none(),
        "a tranche-only wallet cannot invert (no documented lot to draw first)"
    );
}

// ── Phase 4 / Task 10: custody-aware compliance warning (P4 / D-3; reuse) ────────────────────────────
//
// Pure REUSE of the optimizer's `persistability` broker envelope (D-3, verified TRUE by both lenses):
// the warning fires exactly when a disposal draws an undocumented (tranche) unit held at an EXCHANGE
// (broker) in the 2027+ envelope, where own-books specific identification is insufficient (Notices
// 2025-7/2026-20 own-books transitional relief ended 2026-12-31). SelfCustody (own-books never expires)
// and ≤2026 sales are silent. No transfer-statement modeling (D-3). The three discrimination KATs are
// the test (Step-5 mutation is n/a for pure reuse); the marker phrase `Broker specific-ID warning`
// distinguishes it from the dip/inversion advisories that share the assembler.

/// P4 / D-3: the warning FIRES for a ≥2027 disposal that draws a tranche lot held at an exchange.
#[test]
fn broker_specific_id_warning_fires_for_a_2027_exchange_tranche_disposal() {
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2026 - 01 - 01),
            date!(2026 - 01 - 31),
        ),
        sell_ev(
            "SELL",
            datetime!(2027-06-01 00:00 UTC),
            &w,
            100_000_000,
            90_000,
        ),
    ];
    let st = project(&events, &prices(), &config());
    let adv = tranche_report_advisory(&st, &events, &prices(), &config(), 2027, None, &synth(2027))
        .expect("a 2027 exchange tranche disposal produces advisories");
    assert!(
        adv.contains("Broker specific-ID warning"),
        "the P4 broker envelope warning must fire for a 2027 exchange tranche disposal: {adv}"
    );
}

/// P4 / D-3: SILENT for self-custody — own-books specific identification never expires there. The dip
/// advisory still fires (a tranche leg was consumed), so the assertion is marker-ABSENCE, not `None`.
#[test]
fn broker_specific_id_warning_silent_for_a_2027_self_custody_tranche_disposal() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2026 - 01 - 01),
            date!(2026 - 01 - 31),
        ),
        sell_ev(
            "SELL",
            datetime!(2027-06-01 00:00 UTC),
            &w,
            100_000_000,
            90_000,
        ),
    ];
    let st = project(&events, &prices(), &config());
    let adv = tranche_report_advisory(&st, &events, &prices(), &config(), 2027, None, &synth(2027))
        .expect("a tranche disposal still yields a dip advisory");
    assert!(
        !adv.contains("Broker specific-ID warning"),
        "self-custody never triggers the broker envelope (own-books specific-ID never expires): {adv}"
    );
}

/// P4 / D-3: SILENT for a ≤2026 exchange disposal — the Notices 2025-7/2026-20 own-books transitional
/// relief still applies through 2026-12-31. Again marker-ABSENCE (the dip advisory still fires).
#[test]
fn broker_specific_id_warning_silent_below_2027() {
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2025 - 06 - 01),
            date!(2025 - 06 - 30),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            90_000,
        ),
    ];
    let st = project(&events, &prices(), &config());
    let adv = tranche_report_advisory(&st, &events, &prices(), &config(), 2026, None, &synth(2026))
        .expect("a tranche disposal still yields a dip advisory");
    assert!(
        !adv.contains("Broker specific-ID warning"),
        "≤2026 own-books relief (Notices 2025-7/2026-20) — no broker envelope warning: {adv}"
    );
}

/// P4 / D-3: the builder itself is a faithful `persistability` gate — `Some` iff `ForbiddenBroker2027`
/// (broker wallet + ≥2027 sale). This pins the reuse independent of the assembler wiring above.
#[test]
fn broker_specific_id_advisory_builder_gates_on_the_2027_broker_envelope() {
    assert!(
        tranche_broker_specific_id_advisory(&exch(), date!(2027 - 06 - 01), date!(2027 - 06 - 01))
            .is_some(),
        "2027 exchange → ForbiddenBroker2027 → warns"
    );
    assert!(
        tranche_broker_specific_id_advisory(
            &self_custody(),
            date!(2027 - 06 - 01),
            date!(2027 - 06 - 01)
        )
        .is_none(),
        "self-custody → never the broker envelope"
    );
    assert!(
        tranche_broker_specific_id_advisory(&exch(), date!(2026 - 12 - 31), date!(2026 - 12 - 31))
            .is_none(),
        "2026 exchange → own-books transitional relief still applies"
    );
}

// ── Phase 5 / Task 11: window reference-price engine (informational only; NEVER filed — D-7) ─────────
//
// `window_reference` is the MIN daily CLOSE over [start, end]. It is NOT a true floor (intraday lows can
// be lower — tax I-3), so the return type CARRIES a `Coverage` caveat (arch M-6) that P6 must surface: a
// covered-part min over a partially-covered window can EXCEED the true window min. Never filed (D-7).

/// P5: a fully-covered window returns the MIN daily close and `Coverage::Full`.
#[test]
fn window_reference_full_coverage_returns_min_daily_close() {
    let p = priced(&[
        (date!(2018 - 01 - 01), 100),
        (date!(2018 - 01 - 02), 80),
        (date!(2018 - 01 - 03), 120),
    ]);
    let wr = window_reference(&p, date!(2018 - 01 - 01), date!(2018 - 01 - 03))
        .expect("a fully-covered window has a min");
    assert_eq!(
        wr.min,
        rust_decimal::Decimal::from(80),
        "min daily close over the window"
    );
    assert_eq!(
        wr.coverage,
        Coverage::Full,
        "every day in the window has a close → Full"
    );
}

/// P5: a partially-covered window (a gap in the data) returns the min over the COVERED days and flags
/// `Coverage::Partial` — the caveat P6 must surface (tax r1 N-3), since the covered-part min can exceed
/// the true window min.
#[test]
fn window_reference_partial_overlap_returns_covered_min_and_flags_partial() {
    // 2018-01-02 is MISSING → covered = {01, 03}; min over the covered part; Partial.
    let p = priced(&[(date!(2018 - 01 - 01), 100), (date!(2018 - 01 - 03), 60)]);
    let wr = window_reference(&p, date!(2018 - 01 - 01), date!(2018 - 01 - 03))
        .expect("a partially-covered window still has a covered min");
    assert_eq!(
        wr.min,
        rust_decimal::Decimal::from(60),
        "min over the COVERED days only"
    );
    assert_eq!(
        wr.coverage,
        Coverage::Partial,
        "a gap in coverage must be flagged (tax r1 N-3)"
    );
}

/// P5: a window with NO covered day returns `None` — never fabricate a floor over a data gap (D-7).
#[test]
fn window_reference_no_overlap_returns_none() {
    let p = priced(&[(date!(2019 - 01 - 01), 50)]); // outside the queried window
    assert!(
        window_reference(&p, date!(2018 - 01 - 01), date!(2018 - 01 - 03)).is_none(),
        "no covered day in the window → None (never fabricate a floor)"
    );
}

// ── Phase 6 / Task 12: overpayment-delta nudge (informational; the G-3 lever; NEVER filed — D-7) ─────
//
// `overpayment_delta` is the basis-replacement clone-fold-discard what-if (arch M-4): Σ over `refs` of
// tax($0) − tax(reference), each term a re-fold with ONLY that tranche's Op::Acquire.usd_cost swapped.
// Every dollar comes from the single audited `compute_tax_year`; NOTHING >$0 is ever filed (D-7). The
// scenarios stage a post-2025 self-custody tranche disposal (avoids the broker/transition machinery) so
// the ST/LT gain is realized under a Single profile's ordinary/LTCG stack.

/// P6: reconstructing a consumed tranche to a >$0 reference lowers the realized gain, so the delta is a
/// positive federal-tax saving.
#[test]
fn overpayment_delta_positive_when_reconstructing_a_consumed_tranche_lowers_the_gain() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2025 - 06 - 01),
            date!(2025 - 06 - 30),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    let refs = vec![(EventId::decision(1), rust_decimal::Decimal::from(20_000))];
    let d = overpayment_delta(
        &events,
        &prices(),
        &config(),
        2026,
        Some(&tax_profile(60_000)),
        &synth(2026),
        &refs,
    );
    assert!(
        d > dec!(0),
        "reconstructing the $0 tranche to a $20k basis lowers the gain and the tax: {d}"
    );
}

/// P6: a $0 reference (or no references) recovers nothing — replacing $0 with $0 changes no gain (D-7).
#[test]
fn overpayment_delta_is_zero_when_reference_is_zero_or_absent() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2025 - 06 - 01),
            date!(2025 - 06 - 30),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    let prof = tax_profile(60_000);
    let zero_ref = overpayment_delta(
        &events,
        &prices(),
        &config(),
        2026,
        Some(&prof),
        &synth(2026),
        &[(EventId::decision(1), dec!(0))],
    );
    assert_eq!(zero_ref, dec!(0), "a $0 reference recovers nothing");
    let none = overpayment_delta(
        &events,
        &prices(),
        &config(),
        2026,
        Some(&prof),
        &synth(2026),
        &[],
    );
    assert_eq!(none, dec!(0), "no tranche references → $0 delta");
}

/// P6: a year consuming legs from two differently-windowed tranches SUMS the per-tranche deltas, each
/// with ITS OWN reference (not one joint number). The two-tranche delta == the sum of the two
/// single-tranche deltas; different references make the "one reference for all" mutation discriminable.
#[test]
fn overpayment_delta_sums_per_tranche_with_each_tranches_own_reference() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2024 - 01 - 01),
            date!(2024 - 06 - 30),
        ),
        tranche_ev(
            2,
            &w,
            100_000_000,
            date!(2025 - 01 - 01),
            date!(2025 - 06 - 30),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            200_000_000,
            120_000,
        ),
    ];
    let prof = tax_profile(60_000);
    let r1 = (EventId::decision(1), rust_decimal::Decimal::from(10_000));
    let r2 = (EventId::decision(2), rust_decimal::Decimal::from(30_000));
    let call = |refs: &[(EventId, rust_decimal::Decimal)]| {
        overpayment_delta(
            &events,
            &prices(),
            &config(),
            2026,
            Some(&prof),
            &synth(2026),
            refs,
        )
    };
    let just1 = call(std::slice::from_ref(&r1));
    let just2 = call(std::slice::from_ref(&r2));
    let both = call(&[r1.clone(), r2.clone()]);
    assert!(
        just1 > dec!(0) && just2 > dec!(0),
        "each tranche independently recovers tax: {just1} / {just2}"
    );
    assert_eq!(
        both,
        just1 + just2,
        "the two-tranche delta is the SUM of per-tranche deltas, each with its own reference (P6)"
    );
}

/// P6: the nudge surfaces through `tranche_report_advisory` with the mandatory §1014 note and, for a
/// partially-covered window, the partial-coverage caveat — and stays provenance-neutral (no purchase).
#[test]
fn overpayment_nudge_surfaces_in_the_report_with_1014_note_and_partial_caveat() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2025 - 06 - 01),
            date!(2025 - 06 - 30),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    // Prices cover only two days in the window → Partial coverage; min close $18,000.
    let px = priced(&[
        (date!(2025 - 06 - 01), 20_000),
        (date!(2025 - 06 - 15), 18_000),
    ]);
    let st = project(&events, &px, &config());
    let adv = tranche_report_advisory(
        &st,
        &events,
        &px,
        &config(),
        2026,
        Some(&tax_profile(60_000)),
        &synth(2026),
    )
    .expect("a tranche disposal with a recoverable delta produces a nudge");
    assert!(
        adv.contains("Overpayment nudge"),
        "the P6 nudge must surface: {adv}"
    );
    assert!(
        adv.contains("\u{00a7}1014"),
        "the mandatory §1014 note must be present: {adv}"
    );
    assert!(
        adv.contains("Partial-window estimate"),
        "the partial-coverage caveat must surface (tax r1 N-3): {adv}"
    );
    let low = adv.to_lowercase();
    assert!(
        !low.contains("purchase") && !low.contains("bought"),
        "provenance-neutral (tax min-8c): {adv}"
    );
}

/// P6: without a profile there is no computable tax, so no QUANTIFIED nudge surfaces (the dip advisory
/// still does) — the nudge is gated on the tax engine.
#[test]
fn overpayment_nudge_absent_without_a_profile() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2025 - 06 - 01),
            date!(2025 - 06 - 30),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    let px = priced(&[
        (date!(2025 - 06 - 01), 20_000),
        (date!(2025 - 06 - 15), 18_000),
    ]);
    let st = project(&events, &px, &config());
    let text = tranche_report_advisory(&st, &events, &px, &config(), 2026, None, &synth(2026))
        .unwrap_or_default();
    assert!(
        !text.contains("Overpayment nudge"),
        "no profile ⇒ no quantified overpayment nudge: {text}"
    );
}

// ── Phase 7 / Task 13: mandatory methodology disclosure (D-4; basis-as-filed, term-correct) ──────────
//
// `basis_methodology(state, year)` is the i8949 basis explanation the conservative flow MUST emit
// whenever a tranche is filed. Provenance-neutral (a tranche is undocumented BTC, never a purchase) and
// term-correct (short/long DERIVED from the leg, never hard-coded "long-term" — G-4). Never files >$0.

/// P7: present for a filed-tranche year, enumerates EACH filed tranche, and is provenance-neutral.
#[test]
fn basis_methodology_present_enumerates_each_tranche_and_is_provenance_neutral() {
    let w = self_custody();
    let events = vec![
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2015 - 01 - 01),
            date!(2015 - 12 - 31),
        ),
        tranche_ev(
            2,
            &w,
            100_000_000,
            date!(2016 - 01 - 01),
            date!(2016 - 12 - 31),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            200_000_000,
            120_000,
        ),
    ];
    let st = project(&events, &prices(), &config());
    let text = basis_methodology(&st, 2026)
        .expect("a filed-tranche year has a mandatory disclosure (D-4)");
    assert!(
        text.contains("2015-12-31") && text.contains("2016-12-31"),
        "each filed tranche is enumerated by its estimated acquisition date: {text}"
    );
    let low = text.to_lowercase();
    assert!(
        !low.contains("purchase") && !low.contains("bought"),
        "provenance-neutral (tax min-8c): {text}"
    );
}

/// P7: absent when no tranche is in the year's filed set (a fully-documented disposal needs no §basis
/// explanation from this flow).
#[test]
fn basis_methodology_absent_when_no_tranche_is_filed_this_year() {
    let w = self_custody();
    let events = vec![
        documented_buy(
            "BUY",
            datetime!(2025-06-01 00:00 UTC),
            &w,
            100_000_000,
            30_000,
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    let st = project(&events, &prices(), &config());
    assert!(
        basis_methodology(&st, 2026).is_none(),
        "no tranche filed this year → no disclosure"
    );
}

/// P7 / G-4: term is DERIVED — a SHORT-term tranche disposal states "short-term" and the text contains
/// NO hard-coded "long-term". (An ST fixture is REQUIRED for the mutation to discriminate — a legitimately
/// long-term fixture would honestly contain "long-term".)
#[test]
fn basis_methodology_is_term_correct_short_term_never_hard_codes_long_term() {
    let w = self_custody();
    let events = vec![
        // window_end 2026-01-31; disposed 2026-06-01 → < 1yr → SHORT-term.
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2026 - 01 - 01),
            date!(2026 - 01 - 31),
        ),
        sell_ev(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    let st = project(&events, &prices(), &config());
    let text = basis_methodology(&st, 2026).expect("a filed tranche has a disclosure");
    assert!(
        text.contains("short-term"),
        "an ST tranche disposal states short-term (derived): {text}"
    );
    assert!(
        !text.contains("long-term"),
        "term is DERIVED, never hard-coded long-term (G-4): {text}"
    );
}

// ── Phase 8 / Task 14: self-custody nudge (advisory) ─────────────────────────────────────────────────

/// P8: present for an EXCHANGE-held tranche lot — suggest moving no-records units to self-custody
/// (own-books specific-ID never expires there) and recommend a HIFO election (D-9).
#[test]
fn self_custody_nudge_present_for_an_exchange_tranche() {
    let w = exch();
    let st = project(
        &[tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2018 - 01 - 01),
            date!(2018 - 12 - 31),
        )],
        &prices(),
        &config(),
    );
    let adv =
        self_custody_nudge(&st).expect("an exchange-held tranche gets the self-custody nudge");
    assert!(
        adv.to_uppercase().contains("HIFO"),
        "the nudge recommends a HIFO election: {adv}"
    );
    assert!(
        adv.to_lowercase().contains("self-custody"),
        "the nudge names self-custody: {adv}"
    );
}

/// P8: absent for a SelfCustody-held tranche — own-books specific-ID already never expires there.
#[test]
fn self_custody_nudge_absent_for_a_self_custody_tranche() {
    let w = self_custody();
    let st = project(
        &[tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2018 - 01 - 01),
            date!(2018 - 12 - 31),
        )],
        &prices(),
        &config(),
    );
    assert!(
        self_custody_nudge(&st).is_none(),
        "a self-custody tranche needs no self-custody nudge"
    );
}

// ── Phase 9 / Task 15: no-loss-FROM-THE-ESTIMATE invariant + the two documented-fee corners + the ─────
//    engine-integrity pins (SPEC §6 amended; tax min-7 / tax r1 I-1). CHARACTERIZATION — passes on write.
//    The invariant is SCOPED: any negative / >$0 tranche-leg amount traces to DOCUMENTED fee_usd/fee-sat
//    (or cent-scale pro-rata rounding), NEVER to the $0 estimate. If the ESTIMATE ever drove a loss / a
//    >$0 filed basis, that is a Critical — STOP.

/// A `Dispose` carrying a documented USD fee (corner (a)).
fn sell_with_usd_fee(
    rf: &str,
    ts: time::OffsetDateTime,
    w: &WalletId,
    sat: i64,
    proceeds: i64,
    fee_usd: i64,
) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: rust_decimal::Decimal::from(proceeds),
            fee_usd: rust_decimal::Decimal::from(fee_usd),
            kind: DisposeKind::Sell,
        }),
    )
}

/// P9 invariant CORE: absent fees, a tranche leg's gain = proceeds − $0 ≥ 0 — the $0 estimate can never
/// file a loss. A $0-proceeds fee-free disposal yields exactly $0 (never negative).
#[test]
fn fee_free_tranche_disposal_never_files_a_loss() {
    let w = self_custody();
    let t = tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    );
    let st = project(
        &[
            t.clone(),
            sell_ev(
                "SELL",
                datetime!(2020-06-01 00:00 UTC),
                &w,
                100_000_000,
                50_000,
            ),
        ],
        &prices(),
        &config_hifo_pre2025(),
    );
    assert!(
        only_disposal_leg(&st).gain >= dec!(0),
        "fee-free tranche gain ≥ 0 (the estimate never loses)"
    );

    let st0 = project(
        &[
            t,
            sell_ev("SELL", datetime!(2020-06-01 00:00 UTC), &w, 100_000_000, 0),
        ],
        &prices(),
        &config_hifo_pre2025(),
    );
    assert_eq!(
        only_disposal_leg(&st0).gain,
        dec!(0),
        "a $0-proceeds fee-free disposal is exactly $0"
    );
}

/// P9 corner (a): a documented USD fee > proceeds nets the amount realized below $0 (§1001(b)), so the
/// tranche leg's GAIN is negative — but its filed cost basis is still the $0 ESTIMATE. The loss is the
/// documented fee, never the estimate.
#[test]
fn negative_tranche_gain_comes_only_from_documented_fees_corner_a_usd_fee() {
    let w = self_custody();
    let st = project(
        &[
            tranche_ev(
                1,
                &w,
                100_000_000,
                date!(2018 - 01 - 01),
                date!(2018 - 12 - 31),
            ),
            sell_with_usd_fee(
                "SELL",
                datetime!(2020-06-01 00:00 UTC),
                &w,
                100_000_000,
                10,
                40,
            ),
        ],
        &prices(),
        &config_hifo_pre2025(),
    );
    let leg = only_disposal_leg(&st);
    assert!(
        leg.gain < dec!(0),
        "a documented USD fee > proceeds drives the tranche leg negative"
    );
    assert_eq!(
        leg.basis,
        dec!(0),
        "the ESTIMATE is still $0 — the loss is the documented fee, not the estimate"
    );
}

/// P9 pin (tax r1 Nit 2): the tranche 8949 row's date-acquired IS the window_end, end-to-end through the
/// D-6 wiring (col b). Held indirectly by `lot.acquired_at == window_end` + forms copying it; this pins
/// col (b) directly on the emitted row.
#[test]
fn tranche_8949_row_date_acquired_is_window_end() {
    let w = exch();
    let st = project(
        &[
            tranche_ev(
                1,
                &w,
                100_000_000,
                date!(2015 - 01 - 01),
                date!(2015 - 12 - 31),
            ),
            sell_ev(
                "SELL",
                datetime!(2020-06-01 00:00 UTC),
                &w,
                100_000_000,
                40_000,
            ),
        ],
        &prices(),
        &config_hifo_pre2025(),
    );
    let row = btctax_core::form_8949(&st, 2020)
        .into_iter()
        .find(|r| r.cost_basis == dec!(0))
        .expect("the $0 tranche 8949 row");
    assert_eq!(
        row.date_acquired,
        date!(2015 - 12 - 31),
        "col (b) date-acquired == window_end (D-2/D-6)"
    );
}

/// P9 pin (tax r1 Nit 3): Σ-conservation (FR9) holds over a projection containing a tranche — the tranche
/// Acquire bumps `sigma_in` structurally (D-1a-e), so in == disposed + held.
#[test]
fn sigma_conservation_holds_with_a_tranche() {
    let w = self_custody();
    let st = project(
        &[
            tranche_ev(
                1,
                &w,
                100_000_000,
                date!(2018 - 01 - 01),
                date!(2018 - 12 - 31),
            ),
            sell_ev(
                "SELL",
                datetime!(2020-06-01 00:00 UTC),
                &w,
                40_000_000,
                20_000,
            ),
        ],
        &prices(),
        &config_hifo_pre2025(),
    );
    assert!(
        btctax_core::conservation_report(&st).balanced,
        "a tranche projection must conserve sat (FR9): {:?}",
        btctax_core::conservation_report(&st)
    );
}

/// P9 FOLLOWUP (review Minor): the build_op `sat > 0` guard — a hand-crafted Decision-id DeclareTranche
/// with `sat <= 0` (the CLI record path refuses it) folds NOTHING (Op::Skip), so it can neither create a
/// bogus lot nor corrupt Σ-conservation by bumping `sigma_in` non-positively.
#[test]
fn sat_le_zero_decision_tranche_folds_nothing_and_conserves() {
    let w = self_custody();
    let bad = LedgerEvent {
        id: EventId::decision(1),
        utc_timestamp: datetime!(2026-01-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::DeclareTranche(DeclareTranche {
            sat: -1, // hand-crafted: the CLI refuses sat <= 0
            wallet: w.clone(),
            window_start: date!(2018 - 01 - 01),
            window_end: date!(2018 - 12 - 31),
        }),
    };
    let st = project(&[bad], &prices(), &config());
    assert!(
        !st.lots
            .iter()
            .any(|l| l.basis_source == BasisSource::EstimatedConservative),
        "a sat <= 0 tranche folds no lot (build_op sat>0 guard)"
    );
    assert!(
        btctax_core::conservation_report(&st).balanced,
        "and Σ-conservation is intact (no non-positive sigma_in bump)"
    );
}

/// P9 corner (b), reachable staging (plan-tax r2 NEW-1): a specific-ID sale NAMES the full tranche as the
/// principal while a DOCUMENTED lot remains; the on-chain `fee_sat` then consumes FIFO from that remainder
/// (resolve §A.4(a)) and its DOCUMENTED basis re-homes onto the last disposal leg — the tranche leg —
/// under TP8(c) (fold.rs `rehome_onto_disposal_leg`). So the tranche leg's FILED basis is > $0: real
/// documented basis (§1011), NEVER the estimate. This is why P3/P7 print the basis AS FILED, not "$0".
#[test]
fn tp8c_fee_sat_basis_can_land_on_the_last_tranche_leg_corner_b() {
    let w = self_custody();
    let doc = documented_buy("DOC", datetime!(2025-02-01 00:00 UTC), &w, 60_000, 30);
    let t = tranche_ev(1, &w, 100_000, date!(2025 - 01 - 01), date!(2025 - 01 - 31));
    let out = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
        utc_timestamp: datetime!(2025-07-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: Some(500),
            dest_addr: None,
            txid: None,
        }),
    };
    let reclass = LedgerEvent {
        id: EventId::decision(2),
        utc_timestamp: datetime!(2025-08-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            as_: OutflowClass::Dispose {
                kind: DisposeKind::Sell,
            },
            principal_proceeds_or_fmv: rust_decimal::Decimal::from(120),
            fee_usd: None,
            donee: None,
        }),
    };
    let select = LedgerEvent {
        id: EventId::decision(3),
        utc_timestamp: datetime!(2025-08-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new("OUT")),
            lots: vec![LotPick {
                lot: LotId {
                    origin_event_id: EventId::decision(1),
                    split_sequence: 0,
                },
                sat: 100_000,
            }],
        }),
    };
    let st = project(&[doc, t, out, reclass, select], &prices(), &config());
    let tranche_leg = st
        .disposals
        .iter()
        .flat_map(|d| &d.legs)
        .find(|l| l.basis_source == BasisSource::EstimatedConservative)
        .expect("a tranche disposal leg");
    assert!(
        tranche_leg.basis > dec!(0),
        "documented fee-sat basis re-homed onto the tranche leg (TP8c) — basis AS FILED > $0, never the \
         estimate: got {}",
        tranche_leg.basis
    );
}

/// P9 FOLLOWUP (review Minor): the build_op `EventId::Decision` id-guard. A hand-crafted vault can route a
/// DeclareTranche payload through the IMPORT path via `ClassifyRaw{as_: DeclareTranche}` (pass-1c applies
/// the override WITHOUT payload-type validation). Without the id-guard that would fold a bogus $0 lot
/// homed at the IMPORT timestamp (bypassing D-2's window_end); the guard makes it fold NOTHING (Op::Skip).
/// No product surface can author this (the `classify-raw` verb refuses non-`is_imported()` payloads).
#[test]
fn classify_raw_declaretranche_on_an_import_folds_nothing_id_guard() {
    let w = self_custody();
    let raw = LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("RAW")),
        utc_timestamp: datetime!(2023-06-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: EventPayload::Unclassified(Unclassified {
            raw: "hand-crafted".into(),
        }),
    };
    let classify = LedgerEvent {
        id: EventId::decision(1),
        utc_timestamp: datetime!(2026-01-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ClassifyRaw(ClassifyRaw {
            target: EventId::import(Source::Coinbase, SourceRef::new("RAW")),
            as_: Box::new(EventPayload::DeclareTranche(DeclareTranche {
                sat: 100_000_000,
                wallet: w.clone(),
                window_start: date!(2018 - 01 - 01),
                window_end: date!(2018 - 12 - 31),
            })),
        }),
    };
    let st = project(&[raw, classify], &prices(), &config());
    assert!(
        !st.lots
            .iter()
            .any(|l| l.basis_source == BasisSource::EstimatedConservative),
        "a ClassifyRaw-routed DeclareTranche on an IMPORT id folds NO lot (build_op EventId::Decision guard)"
    );
}

// ── T16 review folds (r1) ────────────────────────────────────────────────────────────────────────────

/// I-2 (both lenses): `overpayment_delta` scales the per-BTC window reference to the WHOLE-LOT basis
/// (`reference × sat / 1e8`). A 2-BTC tranche at $20k/BTC has a $40k basis, so its delta is EXACTLY 2× a
/// 1-BTC tranche's at the same price + proportional proceeds (both ST in the same 22% bracket, no NIIT).
/// Before the fix both used a $20k basis → equal deltas (the bug the all-1-BTC fixtures hid).
#[test]
fn overpayment_delta_scales_the_per_btc_reference_to_the_whole_lot_basis() {
    let w = self_custody();
    let prof = tax_profile(60_000);
    let reference = rust_decimal::Decimal::from(20_000); // USD per WHOLE BTC
    let delta = |sat: i64, proceeds: i64| {
        let events = vec![
            tranche_ev(1, &w, sat, date!(2025 - 06 - 01), date!(2025 - 06 - 30)),
            sell_ev("SELL", datetime!(2026-06-01 00:00 UTC), &w, sat, proceeds),
        ];
        overpayment_delta(
            &events,
            &prices(),
            &config(),
            2026,
            Some(&prof),
            &synth(2026),
            &[(EventId::decision(1), reference)],
        )
    };
    let one_btc = delta(100_000_000, 50_000);
    let two_btc = delta(200_000_000, 100_000);
    assert!(one_btc > dec!(0), "the 1-BTC delta is positive: {one_btc}");
    assert_eq!(
        two_btc,
        one_btc * dec!(2),
        "a 2-BTC tranche has 2× the reconstructed basis → 2× the saving (per-BTC scaling, I-2): \
         1-BTC={one_btc} 2-BTC={two_btc}"
    );
}

/// C-1 (arch): `tranche_report_advisory` must NOT panic on an out-of-range `--tax-year` when a tranche
/// lot is held (`--tax-year` is an unvalidated CLI i32; year 10000 is outside `time::Date`'s range and
/// cannot build a Dec-31 as-of). It skips the inversion lookup gracefully.
#[test]
fn tranche_report_advisory_does_not_panic_on_an_out_of_range_year() {
    let w = self_custody();
    let events = vec![tranche_ev(
        1,
        &w,
        100_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 12 - 31),
    )];
    let st = project(&events, &prices(), &config());
    // Year 10000 is outside time::Date's ±9999 range — must not panic (returns without the inversion warn).
    let _ = tranche_report_advisory(
        &st,
        &events,
        &prices(),
        &config(),
        10_000,
        None,
        &synth(10_000),
    );
}
