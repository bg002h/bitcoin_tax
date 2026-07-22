//! KATs for the Defensive Filing Wizard's `journey_view` (Task 6) — the composed, pure/read-only
//! dashboard view over discovery + per-tranche status/advisories/clamped-saving + the pool-level
//! "still short" state + the DFW-D11 export year-set + the safe-harbor mutual-exclusion flag.
//! PRIVACY: synthetic values only.

use btctax_core::conservative::{flagged_years, method_inversion_advisory, tranche_dip_advisory};
use btctax_core::conservative_promote::filed_basis_for;
use btctax_core::defensive::{journey_view, Advisory, SavingFlavor, TrancheStatus};
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, LotMethod, ProjectionConfig};
use btctax_core::tax::testonly::ty2024_table;
use btctax_core::tax::{TaxTable, TaxTables};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

// ── fixture harness (mirrors tests/kat_promote.rs / tests/defensive_discovery.rs) ──────────────────

fn exch() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
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
fn dec_ev(seq: u64, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: p,
    }
}
fn tranche_ev(seq: u64, w: &WalletId, sat: i64, ws: time::Date, we: time::Date) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2030-01-01 00:00 UTC),
        EventPayload::DeclareTranche(DeclareTranche {
            sat,
            wallet: w.clone(),
            window_start: ws,
            window_end: we,
        }),
    )
}
fn promote_ev(seq: u64, target: EventId, filed_basis: rust_decimal::Decimal) -> LedgerEvent {
    dec_ev(
        seq,
        datetime!(2030-02-01 00:00 UTC),
        EventPayload::PromoteTranche(PromoteTranche {
            target,
            method: FloorMethod::WindowLowClose,
            filed_basis,
            coverage: btctax_core::conservative::Coverage::Full,
            provenance_attested: true,
            acknowledgment: Acknowledgment {
                phrase: "I understand and accept the risk".into(),
                shown_terms: vec![],
                provenance_text: "acquired by purchase within the declared window".into(),
                provenance_version: "v1".into(),
            },
            part_ii_narrative: "cash P2P purchase, no records; window bounded on-chain".into(),
        }),
    )
}
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
/// A confirmed self-transfer: TransferOut (from) + TransferIn (to) + a TransferLink decision.
#[allow(clippy::too_many_arguments)]
fn self_transfer(
    out_rf: &str,
    in_rf: &str,
    from: &WalletId,
    to: &WalletId,
    sat: i64,
    fee_sat: Option<i64>,
    ts: time::OffsetDateTime,
    link_seq: u64,
) -> Vec<LedgerEvent> {
    vec![
        imp(
            out_rf,
            ts,
            from,
            EventPayload::TransferOut(TransferOut {
                sat,
                fee_sat,
                dest_addr: None,
                txid: None,
            }),
        ),
        imp(
            in_rf,
            ts,
            to,
            EventPayload::TransferIn(TransferIn {
                sat,
                src_addr: None,
                txid: None,
            }),
        ),
        dec_ev(
            link_seq,
            ts,
            EventPayload::TransferLink(TransferLink {
                out_event: EventId::import(Source::Coinbase, SourceRef::new(out_rf)),
                in_event_or_wallet: TransferTarget::InEvent(EventId::import(
                    Source::Coinbase,
                    SourceRef::new(in_rf),
                )),
            }),
        ),
    ]
}
/// A bare (unlinked) `TransferIn` — pseudo-clearable, blocked (Hard `UnknownBasisInbound`) pseudo-off.
fn bare_transfer_in(rf: &str, ts: time::OffsetDateTime, w: &WalletId, sat: i64) -> LedgerEvent {
    imp(
        rf,
        ts,
        w,
        EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        }),
    )
}
fn prices() -> StaticPrices {
    StaticPrices::default()
}
/// A window `[start,end]` (inclusive, consecutive days) with a bundled close on EVERY day — the
/// `Coverage::Full` `filed_basis_for` requires.
fn full_window_prices(start: time::Date, end: time::Date, min_price: i64) -> StaticPrices {
    let mut m = BTreeMap::new();
    let mut d = start;
    let mut px = min_price;
    loop {
        m.insert(d, rust_decimal::Decimal::from(px));
        px += 500;
        if d == end {
            break;
        }
        d = d.next_day().unwrap();
    }
    StaticPrices(m)
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default()
}
fn no_tables() -> BTreeMap<i32, TaxTable> {
    BTreeMap::new()
}
const FAR_FUTURE: i32 = 9999;

/// A pre-2025 disposal-reorder scenario: a documented 0.6-BTC lot ($3,000 basis) co-held with a promoted
/// 0.4-BTC tranche (floor $12,000 ⇒ HIGHER per-sat, HIFO draws it FIRST once promoted; unpromoted at $0
/// it sorts LAST). A 2018 sell of EXACTLY 0.4 BTC drains the tranche WITH the promote (documented lot
/// untouched) and the documented lot WITHOUT it (tranche untouched) — mirrors `kat_promote.rs`'s
/// `mixed_vintage_hifo_2018_disposal` (shipped, unrelated crate — reconstructed locally; integration
/// tests cannot share code across files).
fn mixed_vintage(w: &WalletId) -> Vec<LedgerEvent> {
    vec![
        documented_buy("BUY", datetime!(2017-01-01 00:00 UTC), w, 60_000_000, 3_000),
        tranche_ev(
            1,
            w,
            40_000_000,
            date!(2018 - 01 - 01),
            date!(2018 - 03 - 31),
        ),
        promote_ev(2, EventId::decision(1), dec!(12_000)),
        sell_ev(
            "SELL",
            datetime!(2018-09-01 00:00 UTC),
            w,
            40_000_000,
            20_000,
        ),
    ]
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// OverCovered (DFW-D5.3, M-1 scope)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn fully_undisposed_tranche_shows_no_over_covered_advisory() {
    let w = exch();
    let events = vec![tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2026 - 01 - 01),
        date!(2026 - 01 - 10),
    )];
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row.advisories
            .iter()
            .any(|a| matches!(a, Advisory::OverCovered { .. })),
        "a fully-undisposed tranche (covered_sat == 0) must never show OverCovered: {:?}",
        row.advisories
    );
}

#[test]
fn over_sized_tranche_shows_over_covered_by_excess() {
    let w = exch();
    let events = vec![
        documented_buy(
            "DOC",
            datetime!(2025-01-05 00:00 UTC),
            &w,
            60_000_000,
            6_000,
        ),
        tranche_ev(
            1,
            &w,
            100_000_000,
            date!(2025 - 01 - 01),
            date!(2025 - 01 - 10),
        ),
        sell_ev(
            "SELL",
            datetime!(2025-06-01 00:00 UTC),
            &w,
            100_000_000,
            50_000,
        ),
    ];
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        row.advisories
            .contains(&Advisory::OverCovered { by_sat: 60_000_000 }),
        "declare 100M, 60M in-pool documented import → OverCovered{{by_sat:60_000_000}}: {:?}",
        row.advisories
    );
}

#[test]
fn a_correctly_sized_cover_and_mixed_vintage_show_no_over_covered() {
    // (a) A correctly-sized cover: the tranche exactly matches the shortfall it plugs, nothing else in
    // the pool at all — removing it reopens EXACTLY its own size, never an excess.
    let w1 = exch();
    let correctly_sized = vec![
        tranche_ev(
            1,
            &w1,
            50_000_000,
            date!(2025 - 01 - 01),
            date!(2025 - 01 - 10),
        ),
        sell_ev(
            "SELL",
            datetime!(2025-06-01 00:00 UTC),
            &w1,
            50_000_000,
            25_000,
        ),
    ];
    let state1 = project(&correctly_sized, &prices(), &cfg());
    let view1 = journey_view(
        &correctly_sized,
        &state1,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    let row1 = view1
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row1
            .advisories
            .iter()
            .any(|a| matches!(a, Advisory::OverCovered { .. })),
        "a correctly-sized cover must show no OverCovered: {:?}",
        row1.advisories
    );

    // (b) mixed_vintage: the documented lot ALONE could cover the sell without the tranche at all
    // (covered_sat == 0) — a legitimate forward-hold, not an over-covered fix (DFW-D5.3 M-1 scope).
    let w2 = cold();
    let events2 = mixed_vintage(&w2);
    let state2 = project(&events2, &prices(), &cfg());
    let view2 = journey_view(
        &events2,
        &state2,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    let row2 = view2
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row2
            .advisories
            .iter()
            .any(|a| matches!(a, Advisory::OverCovered { .. })),
        "mixed_vintage must show no OverCovered: {:?}",
        row2.advisories
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// NowDisplacing (DFW-D5.3/tax-N-1) — basis_source COMPOSITION, not leg-set inequality
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn promoted_tranche_now_displacing_shows_now_displacing_advisory() {
    let w = exch();
    let events = mixed_vintage(&w);
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert_eq!(row.status, TrancheStatus::Promoted);
    assert!(
        row.advisories.contains(&Advisory::NowDisplacing),
        "promoting this tranche now draws it FIRST under HIFO, displacing the documented lot the \
         without-promote fold would have drawn instead: {:?}",
        row.advisories
    );
}

#[test]
fn now_displacing_uses_basis_source_composition_not_leg_set_inequality() {
    // A correctly-sized cover whose ONLY change with/without-promote is its OWN leg's $0 → floor basis
    // (same lot, no other lot in the pool at all) must NOT show NowDisplacing — the leg VEC differs
    // (basis/gain), but the basis_source COMPOSITION does not ({EstimatedConservative} both times).
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            40_000_000,
            date!(2018 - 01 - 01),
            date!(2018 - 03 - 31),
        ),
        promote_ev(2, EventId::decision(1), dec!(12_000)),
        sell_ev(
            "SELL",
            datetime!(2018-09-01 00:00 UTC),
            &w,
            40_000_000,
            20_000,
        ),
    ];
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row.advisories.contains(&Advisory::NowDisplacing),
        "a correctly-sized cover's own $0→floor leg change must NOT trip NowDisplacing (composition, \
         not leg-set inequality): {:?}",
        row.advisories
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// SavingFlavor three-flavor discipline (BG-D6/DFW-D10)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn uncomputable_audience_year_2020_shows_gain_delta_not_a_dollar_tax() {
    let w = exch();
    let ws = date!(2019 - 01 - 01);
    let we = date!(2019 - 01 - 03);
    let prices = full_window_prices(ws, we, 10_000);
    let events = vec![
        tranche_ev(1, &w, 10_000_000, ws, we),
        sell_ev(
            "SELL",
            datetime!(2020-06-01 00:00 UTC),
            &w,
            10_000_000,
            5_000,
        ),
    ];
    let state = project(&events, &prices, &cfg());
    let tables = no_tables(); // 2020 is a table-less audience year — never ships
    let view = journey_view(&events, &state, &prices, &tables, &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert_eq!(row.status, TrancheStatus::DeclaredZero);

    let cf = filed_basis_for(&prices, 10_000_000, ws, we).unwrap();
    assert_eq!(
        row.clamped_saving,
        vec![SavingFlavor::Uncomputable {
            year: 2020,
            gain_delta: cf.filed_basis,
        }],
        "a table-less audience year must show the gain-delta flavor, never a bare dollar tax figure: {:?}",
        row.clamped_saving
    );
}

#[test]
fn table_year_with_no_tax_profile_shows_uncomputable_not_a_bare_dollar() {
    let w = exch();
    let ws = date!(2023 - 01 - 01);
    let we = date!(2023 - 01 - 03);
    let prices = full_window_prices(ws, we, 10_000);
    let events = vec![
        tranche_ev(1, &w, 10_000_000, ws, we),
        sell_ev(
            "SELL",
            datetime!(2024-06-01 00:00 UTC),
            &w,
            10_000_000,
            5_000,
        ),
    ];
    let state = project(&events, &prices, &cfg());
    let mut tables: BTreeMap<i32, TaxTable> = BTreeMap::new();
    tables.insert(2024, ty2024_table()); // a REAL bundled table for 2024 — but no stored TaxProfile
    let view = journey_view(&events, &state, &prices, &tables, &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();

    let cf = filed_basis_for(&prices, 10_000_000, ws, we).unwrap();
    assert_eq!(
        row.clamped_saving,
        vec![SavingFlavor::Uncomputable {
            year: 2024,
            gain_delta: cf.filed_basis,
        }],
        "a 2024 table exists but no stored TaxProfile is threaded — must be Uncomputable, NEVER \
         ComputedTax: {:?}",
        row.clamped_saving
    );
    assert!(
        tables.table_for(2024).is_some(),
        "sanity: the 2024 table really is present"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// still_short (★ I-3/arch-I-5 pool-level, residual values)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn a_live_tranche_not_clearing_its_pool_shows_pool_still_short() {
    let w = exch();
    let sell_date = datetime!(2026-06-01 00:00 UTC);
    // A SECOND tranche, declared AFTER the shortfall's own date, in the SAME pool — it must NOT count
    // toward `live_tranche_sat` (it wasn't declared in time to have covered this short; mutation: drop
    // the `window_end <= short date` filter → this huge 999M sat wrongly inflates the residual value).
    let events = vec![
        tranche_ev(
            1,
            &w,
            30_000_000,
            date!(2026 - 01 - 01),
            date!(2026 - 01 - 10),
        ),
        sell_ev("SELL", sell_date, &w, 100_000_000, 50_000),
        tranche_ev(
            2,
            &w,
            999_000_000,
            date!(2026 - 07 - 01),
            date!(2026 - 07 - 10),
        ),
    ];
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    assert_eq!(
        view.still_short.len(),
        1,
        "exactly ONE combined pool-level row, no per-tranche attribution: {:?}",
        view.still_short
    );
    let ps = &view.still_short[0];
    assert_eq!(
        ps.short_sat, 70_000_000,
        "the residual shortfall (100M need - 30M tranche)"
    );
    assert_eq!(
        ps.live_tranche_sat, 30_000_000,
        "the live tranche's own declared sat (residual value) — the LATER tranche (declared after the \
         short) must not count"
    );
}

#[test]
fn two_shortfalls_in_the_same_pool_sum_short_sat_without_double_counting_the_live_tranche() {
    // ★ Task-6-review Minor-2: a SINGLE live tranche, too small to clear EITHER of two distinct
    // shortfalls in the SAME pool. `short_sat` must be the SUM of both shortfalls; `live_tranche_sat`
    // must be the tranche's OWN (shared) supply, NOT double-counted across the two shortfall records
    // that both "match" it (mutation: change the short_sat SUM reducer to `.max()` — the KAT must red).
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            20_000_000,
            date!(2026 - 01 - 01),
            date!(2026 - 01 - 10),
        ),
        // Consumes the ENTIRE 20M tranche; short by 50M - 20M = 30M.
        sell_ev(
            "SELL1",
            datetime!(2026-03-01 00:00 UTC),
            &w,
            50_000_000,
            25_000,
        ),
        // The tranche is now fully consumed (by SELL1) — this second, LATER disposal in the SAME pool
        // is short by its FULL 60M (nothing left to draw).
        sell_ev(
            "SELL2",
            datetime!(2026-04-01 00:00 UTC),
            &w,
            60_000_000,
            30_000,
        ),
    ];
    let state = project(&events, &prices(), &cfg());

    // Sanity: two DISTINCT shortfall records exist (one per disposal event) before we even reach
    // `journey_view` — confirms the fixture actually produces two aggregates, not one.
    let raw = btctax_core::defensive::discovery::shortfalls(&state);
    assert_eq!(
        raw.len(),
        2,
        "fixture must produce two distinct per-event shortfalls: {raw:?}"
    );
    let expected_sum: i64 = raw.iter().map(|s| s.short_sat).sum();
    assert_eq!(
        expected_sum, 90_000_000,
        "sanity: 30M (SELL1 residual) + 60M (SELL2, tranche already exhausted): {raw:?}"
    );

    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    assert_eq!(
        view.still_short.len(),
        1,
        "exactly ONE combined pool-level row for the two same-pool shortfalls: {:?}",
        view.still_short
    );
    let ps = &view.still_short[0];
    assert_eq!(
        ps.short_sat, expected_sum,
        "short_sat must be the SUM of both shortfalls (90M), never a `.max()` (60M): {ps:?}"
    );
    assert_eq!(
        ps.live_tranche_sat, 20_000_000,
        "live_tranche_sat must be the tranche's OWN (shared) supply — NOT double-counted to 40M just \
         because two shortfall records both match it: {ps:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// FeeOnlyPromoteNoop (★ arch-I-2/tax-M-1)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn fee_only_coverage_tranche_shows_fee_only_promote_noop() {
    // A pure-fee coverage tranche: principal is fully covered by a documented Acquire, only the on-chain
    // fee draw is short (fold.rs's shared `consume_fee` site) — the tranche exists ONLY to plug that.
    let wa = exch();
    let wb = cold();
    let mut fee_events = vec![documented_buy(
        "A1",
        datetime!(2025-01-05 00:00 UTC),
        &wa,
        50_000_000,
        1_000,
    )];
    fee_events.push(tranche_ev(
        1,
        &wa,
        1_000,
        date!(2025 - 01 - 10),
        date!(2025 - 01 - 20),
    ));
    fee_events.extend(self_transfer(
        "XOUT1",
        "XIN1",
        &wa,
        &wb,
        50_000_000,
        Some(1_000),
        datetime!(2025-02-01 00:00 UTC),
        2,
    ));
    let state_fee = project(&fee_events, &prices(), &cfg());
    let view_fee = journey_view(
        &fee_events,
        &state_fee,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    let row_fee = view_fee
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        row_fee.advisories.contains(&Advisory::FeeOnlyPromoteNoop),
        "a purely fee-component coverage tranche must show FeeOnlyPromoteNoop: {:?}",
        row_fee.advisories
    );

    // A principal-coverage tranche (no fee component at all) must show NO FeeOnlyPromoteNoop.
    let wc = WalletId::Exchange {
        provider: "cb".into(),
        account: "principal".into(),
    };
    let wd = WalletId::SelfCustody {
        label: "principal-cold".into(),
    };
    let mut principal_events = vec![tranche_ev(
        1,
        &wc,
        50_000_000,
        date!(2025 - 01 - 01),
        date!(2025 - 01 - 10),
    )];
    principal_events.extend(self_transfer(
        "XOUT2",
        "XIN2",
        &wc,
        &wd,
        50_000_000,
        None,
        datetime!(2025-03-01 00:00 UTC),
        2,
    ));
    let state_pr = project(&principal_events, &prices(), &cfg());
    let view_pr = journey_view(
        &principal_events,
        &state_pr,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    let row_pr = view_pr
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row_pr.advisories.contains(&Advisory::FeeOnlyPromoteNoop),
        "a principal-coverage tranche must NOT show FeeOnlyPromoteNoop: {:?}",
        row_pr.advisories
    );

    // A MIXED-coverage tranche (one covered shortfall is fee-only, another is principal-only) must show
    // NO FeeOnlyPromoteNoop either — the predicate is "ALL covered shortfalls are fee-component", never
    // "ANY" (mutation: `.any` instead of `.all` reds here, though NOT on the single-shortfall cases above).
    let we = WalletId::Exchange {
        provider: "cb".into(),
        account: "mixed".into(),
    };
    let wf = WalletId::SelfCustody {
        label: "mixed-cold".into(),
    };
    let mut mixed_events = vec![documented_buy(
        "A2",
        datetime!(2025-01-01 00:00 UTC),
        &we,
        50_000_000,
        1_000,
    )];
    mixed_events.push(tranche_ev(
        1,
        &we,
        1_500,
        date!(2025 - 01 - 05),
        date!(2025 - 01 - 10),
    ));
    // Transfer #1: principal (50M) fully covered by the documented buy; fee (1000 sat) is short by
    // exactly 1000 in the without-tranche shadow — a PURE-fee shortfall.
    mixed_events.extend(self_transfer(
        "MOUT1",
        "MIN1",
        &we,
        &wf,
        50_000_000,
        Some(1_000),
        datetime!(2025-02-01 00:00 UTC),
        2,
    ));
    // Transfer #2: a further 500-sat principal draw with NOTHING left in the without-tranche shadow (the
    // documented buy was already fully consumed by transfer #1's principal) — a PURE-principal shortfall.
    mixed_events.extend(self_transfer(
        "MOUT2",
        "MIN2",
        &we,
        &wf,
        500,
        None,
        datetime!(2025-03-01 00:00 UTC),
        3,
    ));
    let state_mixed = project(&mixed_events, &prices(), &cfg());
    let view_mixed = journey_view(
        &mixed_events,
        &state_mixed,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    let row_mixed = view_mixed
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row_mixed
            .advisories
            .iter()
            .any(|a| matches!(a, Advisory::OverCovered { .. })),
        "sanity: the combined tranche (1500) exactly matches the combined shortfall (1000+500): {:?}",
        row_mixed.advisories
    );
    assert!(
        !row_mixed.advisories.contains(&Advisory::FeeOnlyPromoteNoop),
        "a tranche covering a MIX of fee-only and principal-only shortfalls must NOT show \
         FeeOnlyPromoteNoop (ALL, not ANY): {:?}",
        row_mixed.advisories
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// MethodInversion / TrancheDip (verbatim, ★ arch-I-2/tax-N-2)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn hifo_steered_promote_surfaces_method_inversion_advisory() {
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            40_000_000,
            date!(2018 - 01 - 01),
            date!(2018 - 03 - 31),
        ),
        documented_buy(
            "BUY",
            datetime!(2018-04-01 00:00 UTC),
            &w,
            60_000_000,
            3_000,
        ),
    ];
    let state = project(&events, &prices(), &cfg());

    let mut fifo_cfg = cfg();
    fifo_cfg.pre2025_method = LotMethod::Fifo; // non-HIFO in-force method pre-2025
    let view_inverted = journey_view(&events, &state, &prices(), &no_tables(), &fifo_cfg, 2020);
    let row_inverted = view_inverted
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    let expected = method_inversion_advisory(&state, &w, LotMethod::Fifo).unwrap();
    assert!(
        row_inverted
            .advisories
            .iter()
            .any(|a| *a == Advisory::MethodInversion(expected.clone())),
        "a non-HIFO in-force method with both a tranche AND a documented lot must surface \
         method_inversion_advisory VERBATIM: {:?}",
        row_inverted.advisories
    );

    let hifo_cfg = cfg(); // default pre2025_method is HIFO — never inverts
    let view_hifo = journey_view(&events, &state, &prices(), &no_tables(), &hifo_cfg, 2020);
    let row_hifo = view_hifo
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row_hifo
            .advisories
            .iter()
            .any(|a| matches!(a, Advisory::MethodInversion(_))),
        "HIFO itself never inverts — no MethodInversion advisory: {:?}",
        row_hifo.advisories
    );
}

#[test]
fn tranche_dip_surfaces_on_tranche_row() {
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            40_000_000,
            date!(2018 - 01 - 01),
            date!(2018 - 03 - 31),
        ),
        sell_ev(
            "SELL",
            datetime!(2018-09-01 00:00 UTC),
            &w,
            40_000_000,
            20_000,
        ),
    ];
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    let expected = tranche_dip_advisory(&state.disposals[0]).unwrap();
    assert!(
        row.advisories.contains(&Advisory::TrancheDip(expected.clone())),
        "a disposal consuming an EstimatedConservative tranche leg must surface tranche_dip_advisory \
         VERBATIM: {:?}",
        row.advisories
    );

    // Negative: a fully-undisposed tranche shows NO TrancheDip.
    let w2 = cold();
    let events2 = vec![tranche_ev(
        1,
        &w2,
        40_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 03 - 31),
    )];
    let state2 = project(&events2, &prices(), &cfg());
    let view2 = journey_view(
        &events2,
        &state2,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    let row2 = view2
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row2
            .advisories
            .iter()
            .any(|a| matches!(a, Advisory::TrancheDip(_))),
        "an undisposed tranche must show no TrancheDip: {:?}",
        row2.advisories
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Pseudo-off discipline (DFW-D6) + DFW-D3 status fork
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// journey_view's OWN shadow projections must force `pseudo_reconcile = false` regardless of the
/// caller's `cfg` — a bare unresolved `TransferIn` that would pseudo-clear differently inside the
/// without-this-tranche shadow must NOT change the derived view depending on the caller's cfg bit.
#[test]
fn journey_view_forces_pseudo_off() {
    let w = exch();
    let events = vec![
        tranche_ev(
            1,
            &w,
            50_000_000,
            date!(2025 - 01 - 01),
            date!(2025 - 01 - 10),
        ),
        bare_transfer_in("PIN", datetime!(2025-01-05 00:00 UTC), &w, 30_000_000),
        sell_ev(
            "SELL",
            datetime!(2025-06-01 00:00 UTC),
            &w,
            50_000_000,
            25_000,
        ),
    ];
    // The `state` passed to journey_view is ALWAYS the pseudo-off projection (the realistic call
    // pattern per DFW-D6 — the precondition is on `state`, not on `cfg`).
    let mut cfg_off = cfg();
    cfg_off.pseudo_reconcile = false;
    let state = project(&events, &prices(), &cfg_off);
    assert!(!state.pseudo_active());

    let mut cfg_on = cfg();
    cfg_on.pseudo_reconcile = true;

    let view_on = journey_view(
        &events,
        &state,
        &prices(),
        &no_tables(),
        &cfg_on,
        FAR_FUTURE,
    );
    let view_off = journey_view(
        &events,
        &state,
        &prices(),
        &no_tables(),
        &cfg_off,
        FAR_FUTURE,
    );

    assert_eq!(
        view_on.candidates, view_off.candidates,
        "candidates must be unchanged by the caller's cfg pseudo bit"
    );
    assert_eq!(
        view_on.tranches, view_off.tranches,
        "tranche rows (advisories + clamped_saving) must be unchanged by the caller's cfg pseudo bit \
         — every internal shadow must force pseudo off on its OWN copy"
    );
    let row = view_off
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert!(
        !row.advisories.iter().any(|a| matches!(a, Advisory::OverCovered { .. })),
        "sanity (pseudo-off, correct behavior): the bare TransferIn stays blocked, so the tranche is \
         correctly-sized against the genuine 50M shortfall: {:?}",
        row.advisories
    );
}

#[test]
fn zero_declared_tranche_status_is_declared_zero_never_incomplete() {
    let w = exch();
    let events = vec![tranche_ev(
        1,
        &w,
        40_000_000,
        date!(2026 - 01 - 01),
        date!(2026 - 01 - 10),
    )];
    let state = project(&events, &prices(), &cfg());
    let view = journey_view(&events, &state, &prices(), &no_tables(), &cfg(), FAR_FUTURE);
    let row = view
        .tranches
        .iter()
        .find(|r| r.target == EventId::decision(1))
        .unwrap();
    assert_eq!(
        row.status,
        TrancheStatus::DeclaredZero,
        "a $0-declared (never-promoted) tranche is DeclaredZero — DFW-D3 forbids an 'incomplete/step \
         N' status"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★ T3-Nit burndown: journey_view.flagged_years must be the SAME `< current`-filtered set the export
// (plan_export) computes — no display-vs-export drift.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn journey_view_flagged_years_matches_the_export_computation() {
    let w = exch();
    let events = mixed_vintage(&w);
    let state = project(&events, &prices(), &cfg());
    let tables = no_tables();

    // Both sides of the `< current` boundary (mirrors kat_promote.rs's own
    // `the_current_cutoff_excludes_the_year_still_being_authored`) — this is the load-bearing check: if
    // `journey_view` ever ignored/hard-coded `current` instead of threading it through, a fixture with
    // only ONE flagged year on ONE side of the boundary would not catch it, but comparing BOTH sides does.
    for current in [2018, 2019] {
        let view = journey_view(&events, &state, &prices(), &tables, &cfg(), current);
        let export_set = flagged_years(&events, &state, &prices(), &tables, &cfg(), current);
        assert_eq!(
            view.flagged_years, export_set,
            "journey_view.flagged_years must equal the SAME flagged_years(..., current) plan_export \
             computes (current={current}) — no display-vs-export drift"
        );
    }

    let excluded = journey_view(&events, &state, &prices(), &tables, &cfg(), 2018).flagged_years;
    assert!(
        !excluded.contains(&2018),
        "current=2018 must exclude year 2018 (still being authored): {excluded:?}"
    );
    let included = journey_view(&events, &state, &prices(), &tables, &cfg(), 2019).flagged_years;
    assert!(
        included.contains(&2018),
        "current=2019 must still include the (now presumed-filed) year 2018: {included:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// safe_harbor_blocked (the CORE tranche_guard predicates, C-2 — never the cli-private guard)
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn safe_harbor_blocked_reflects_the_core_tranche_guard_predicates() {
    // Neither predicate present → not blocked.
    let w = exch();
    let plain = vec![tranche_ev(
        1,
        &w,
        10_000_000,
        date!(2026 - 01 - 01),
        date!(2026 - 01 - 10),
    )];
    let state_plain = project(&plain, &prices(), &cfg());
    let view_plain = journey_view(
        &plain,
        &state_plain,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    assert!(
        !view_plain.safe_harbor_blocked,
        "neither predicate present → not blocked"
    );

    // An in-force SafeHarborAllocation → blocked (in_force_allocation_exists).
    let alloc = dec_ev(
        1,
        datetime!(2024-12-01 00:00 UTC),
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots: vec![AllocLot {
                wallet: w.clone(),
                sat: 10_000_000,
                usd_basis: dec!(1_000),
                acquired_at: date!(2020 - 01 - 01),
                dual_loss_basis: None,
                donor_acquired_at: None,
            }],
            as_of_date: date!(2025 - 01 - 01),
            method: AllocMethod::ActualPosition,
            timely_allocation_attested: true,
            pre2025_method: LotMethod::Hifo,
        }),
    );
    let alloc_events = vec![alloc];
    let state_alloc = project(&alloc_events, &prices(), &cfg());
    let view_alloc = journey_view(
        &alloc_events,
        &state_alloc,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    assert!(
        view_alloc.safe_harbor_blocked,
        "an in-force SafeHarborAllocation must set safe_harbor_blocked"
    );

    // A pre-2025 DeclareTranche alone → blocked (pre2025_tranche_exists).
    let pre2025 = vec![tranche_ev(
        1,
        &w,
        10_000_000,
        date!(2018 - 01 - 01),
        date!(2018 - 03 - 31),
    )];
    let state_pre2025 = project(&pre2025, &prices(), &cfg());
    let view_pre2025 = journey_view(
        &pre2025,
        &state_pre2025,
        &prices(),
        &no_tables(),
        &cfg(),
        FAR_FUTURE,
    );
    assert!(
        view_pre2025.safe_harbor_blocked,
        "a pre-2025 DeclareTranche alone must set safe_harbor_blocked"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Grep guard (DFW-D7): journey_view must consume the STRUCTURED short_sat signal, never re-parse
// `Blocker.detail` (mirrors defensive_discovery.rs's own grep guard for discovery.rs).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn journey_view_never_parses_blocker_detail() {
    let src = include_str!("../src/defensive/mod.rs");
    assert!(
        !src.contains(".detail"),
        "defensive/mod.rs must never parse Blocker.detail — DFW-D7 reads only the structured \
         {{event,wallet,date,short_sat}} signal"
    );
}
