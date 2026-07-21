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
fn prices() -> StaticPrices {
    StaticPrices::default()
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
