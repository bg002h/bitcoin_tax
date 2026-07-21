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
    method_inversion_advisory, tranche_broker_specific_id_advisory, tranche_dip_advisory,
    tranche_report_advisory, window_reference, Coverage,
};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::{DisposalLeg, LedgerState};
use btctax_core::LotMethod;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

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
    let adv = tranche_report_advisory(&st, &events, &prices(), &config(), 2027)
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
    let adv = tranche_report_advisory(&st, &events, &prices(), &config(), 2027)
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
    let adv = tranche_report_advisory(&st, &events, &prices(), &config(), 2026)
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
