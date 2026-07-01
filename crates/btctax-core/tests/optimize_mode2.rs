//! Sub-project C, Task 6 — Mode-2 pre-trade consult `consult_sale` KATs.
//!
//! `consult_sale` is the READ-ONLY what-if: for a HYPOTHETICAL sale (sell `sell_sat` from `wallet`
//! at `at`, with `proceeds` or FMV) it picks the tax-minimizing lot selection, reports the resulting
//! ST/LT split + the year's federal tax, and an ST→LT timing insight — WITHOUT producing any event,
//! ledger mutation, or decision (clone-fold-discard only). It is tax decision-support (consequences),
//! NOT buy/sell advice.
//!
//! Load-bearing invariants exercised here:
//! - Tax-min selection (high-basis lot picked); ST/LT/total are re-derivable hand goldens.
//! - ST→LT timing insight fires for a short-term-but-soon-long-term lot, with the SAME-year/profile
//!   saving (R0-I4); it is OMITTED (None, never Err) when not applicable / data unavailable
//!   (purely-LT selection; crossover into an unbundled year R0-I4; the `next_day` Dec-31 edge R0-M4).
//! - As-of-`at` pool (R0-M3): a lot acquired after `at` is excluded; a lot disposed after `at` is
//!   still available at `at`.
//! - `--proceeds` required for a future date with no dataset price (R0 `ProceedsRequired`).
//! - The consult writes NOTHING: `events` is byte-identical before/after, and the baseline projection
//!   is unperturbed.
//! - Determinism (NFR4): two calls → byte-identical `ConsultReport`.
//!
//! All fixtures are synthetic (privacy — no real reads); exact Decimal, no float (NFR5). Federal-only.
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::optimize::{consult_sale, ConsultRequest, OptimizeError};
use btctax_core::price::StaticPrices;
use btctax_core::project::fold::state_as_of;
use btctax_core::project::resolve::resolve;
use btctax_core::project::{evaluate_disposal, project, EvaluateError, ProjectionConfig};
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxProfile};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

const LOT: i64 = 100_000_000; // one whole BTC per lot

// ── synthetic table + profile (Single; same schedule as optimize_score.rs) ───────────────────────
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
/// Single filer; ordinary == MAGI = 0 so a chosen ST gain stacks cleanly on the 10% band and MAGI
/// stays well below the $200k NIIT threshold — the whole tax is the ordinary/LTCG on the crypto gain.
fn profile() -> TaxProfile {
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
    }
}

// ── event / id builders ──────────────────────────────────────────────────────────────────────────
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn eid(rf: &str) -> EventId {
    EventId::import(Source::Swan, SourceRef::new(rf))
}
fn lid(rf: &str) -> LotId {
    LotId {
        origin_event_id: eid(rf),
        split_sequence: 0,
    }
}
fn pick(rf: &str, sat: i64) -> LotPick {
    LotPick { lot: lid(rf), sat }
}
fn ev(rf: &str, ts: time::OffsetDateTime, w: WalletId, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: eid(rf),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w),
        payload: p,
    }
}
fn buy(rf: &str, ts: time::OffsetDateTime, w: WalletId, sat: i64, cost: Usd) -> LedgerEvent {
    ev(
        rf,
        ts,
        w,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}
fn sell(rf: &str, ts: time::OffsetDateTime, w: WalletId, sat: i64, proceeds: Usd) -> LedgerEvent {
    ev(
        rf,
        ts,
        w,
        EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: proceeds,
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    )
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default() // FIFO default, TreatmentC
}
/// Mode-2 request helper — a Sell of `sat` from `cold` at `at` with explicit `proceeds`.
fn req(sat: i64, at: time::Date, proceeds: Option<Usd>) -> ConsultRequest {
    ConsultRequest {
        sell_sat: sat,
        wallet: cold(),
        at,
        proceeds,
        kind: DisposeKind::Sell,
    }
}

// ── KAT: tax-min lot selection (high-basis lot picked) ─────────────────────────────────────────────

/// Wallet with a low-basis and a high-basis lot; a consult to sell ONE lot's worth picks the
/// HIGH-basis lot (least taxable gain). ST/LT/total match hand goldens. Both lots are 2025-acquired,
/// so the chosen lot's crossover lands in 2026 (unbundled) → timing omitted (None).
#[test]
fn consult_picks_high_basis_lot_min_tax() {
    let events = vec![
        buy(
            "LB",
            datetime!(2025-01-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        buy(
            "HB",
            datetime!(2025-02-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();
    let r = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 06 - 01), Some(dec!(10000))),
    )
    .expect("consult succeeds");

    // High-basis HB chosen: gain 10,000 − 5,000 = 5,000 ST @ 10% = 500.00.
    assert_eq!(r.proposed_selection, vec![pick("HB", LOT)]);
    assert_eq!(r.st_gain, dec!(5000));
    assert_eq!(r.lt_gain, dec!(0));
    assert_eq!(r.total_federal_tax_attributable, dec!(500.00));
    // Both lots 2025-acquired → crossover in 2026 (unbundled) → timing omitted.
    assert!(
        r.timing.is_none(),
        "2026 crossover is unbundled → no insight"
    );

    // Re-derivable: re-scoring the proposed pick via A's evaluate_disposal yields the same split.
    let candidate = btctax_core::project::CandidateDisposal {
        existing_event: None,
        wallet: cold(),
        date: date!(2025 - 06 - 01),
        sat: LOT,
        kind: DisposeKind::Sell,
        proceeds: Some(dec!(10000)),
    };
    let out = evaluate_disposal(
        &events,
        &prices,
        &cfg(),
        &candidate,
        Some(&r.proposed_selection),
    )
    .expect("evaluate the proposed pick");
    assert_eq!(out.st_gain, r.st_gain);
    assert_eq!(out.lt_gain, r.lt_gain);
}

// ── KAT: ST→LT timing insight (R0-I4 — same-year crossover) ────────────────────────────────────────

/// A lot ST as of `at` that crosses to LT later THE SAME bundled year. The chosen selection includes
/// it → `timing` is `Some`; `latest_crossover` equals the hand-computed crossover date; and
/// `saving_if_waited == total_now − tax_if_sold_long_term`, where `tax_if_sold_long_term` is the SAME
/// selection/proceeds scored with the term flipped (same year/profile/table).
///
/// Fixture: one lot acquired 2024-06-01 (Path A residue, basis preserved). Consult to sell 1 BTC on
/// 2025-02-15 for $50,000 → gain $40,000. As of 2025-02-15 the lot is ST (1-yr anniversary 2025-06-01);
/// it becomes LT on 2025-06-02 (same year). total_now = $40,000 ST @ 10% = $4,000; sold LT the $40,000
/// sits entirely in the 0% LTCG band (max_zero = $40,000) → $0; saving = $4,000.
#[test]
fn timing_insight_same_year_crossover() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();
    let r = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 02 - 15), Some(dec!(50000))),
    )
    .expect("consult succeeds");

    assert_eq!(r.st_gain, dec!(40000));
    assert_eq!(r.lt_gain, dec!(0));
    assert_eq!(r.total_federal_tax_attributable, dec!(4000.00));

    let t = r
        .timing
        .expect("a short-term-but-soon-long-term lot fires the insight");
    assert_eq!(t.st_sat_in_selection, LOT);
    assert_eq!(t.latest_crossover, date!(2025 - 06 - 02)); // one_year_after(2024-06-01)=2025-06-01, +1d
    assert_eq!(t.tax_if_sold_long_term, dec!(0.00)); // $40k LT in the 0% band
    assert_eq!(t.saving_if_waited, dec!(4000.00)); // 4000 − 0
}

/// A purely long-term selection → no short-term legs → `timing == None`. Lot acquired 2024-01-01 is
/// already LT as of 2025-06-01 (anniversary 2025-01-01).
#[test]
fn purely_long_term_selection_omits_timing() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();
    let r = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 06 - 01), Some(dec!(50000))),
    )
    .expect("consult succeeds");
    assert_eq!(r.st_gain, dec!(0));
    assert_eq!(r.lt_gain, dec!(40000));
    assert!(r.timing.is_none(), "no short-term leg ⇒ no timing insight");
}

// ── KAT: R0-I4 degrade — crossover into an unbundled year ───────────────────────────────────────────

/// A lot whose `latest_crossover` lands in a year with NO bundled table/profile (2026): `consult_sale`
/// still returns `Ok(ConsultReport)` with `timing == None` (OMITTED) — it does NOT `Err`. The lot is
/// 2025-06-01-acquired and ST as of the 2025-12-15 consult; it crosses on 2026-06-02 (unbundled).
#[test]
fn timing_degrades_to_none_for_unbundled_crossover_year() {
    let events = vec![buy(
        "L",
        datetime!(2025-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025); // only 2025 bundled
    let prof = profile();
    let r = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 12 - 15), Some(dec!(50000))),
    );
    let report = r.expect("degrade, never error: consult still returns Ok");
    assert!(
        report.timing.is_none(),
        "crossover year 2026 is unbundled ⇒ timing omitted, not an error"
    );
    // The headline what-if is still computed (the 2025 year IS bundled).
    assert_eq!(report.st_gain, dec!(40000));
    assert_eq!(report.total_federal_tax_attributable, dec!(4000.00));
}

// ── KAT: R0-M4 — next_day Dec-31/max-date edge ─────────────────────────────────────────────────────

/// A lot whose 1-year anniversary is 9999-12-31, so `one_year_after(start).next_day()` is `None`
/// (the `time::Date` max-date edge). The timing insight must be OMITTED (no panic / no unwrap).
#[test]
fn timing_omitted_on_next_day_max_date_edge() {
    let events = vec![buy(
        "L",
        datetime!(9998-12-31 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(9999);
    let prof = profile();
    let r = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(9999 - 06 - 01), Some(dec!(50000))),
    );
    let report = r.expect("no panic; consult returns Ok");
    assert!(
        report.timing.is_none(),
        "next_day(9999-12-31) is None ⇒ timing omitted (R0-M4)"
    );
    assert_eq!(report.st_gain, dec!(40000)); // ST as of 9999-06-01
}

// ── KAT: R0-M3 — as-of-`at` pool ───────────────────────────────────────────────────────────────────

/// An interleaved fixture where a later acquisition AND a later disposal both exist after `at`. The
/// consult pool must reflect holdings AS OF `at`: a lot acquired after `at` is EXCLUDED; a lot disposed
/// after `at` is STILL available at `at`. With the end-of-timeline pool the answer would be inverted
/// (EARLY consumed by the later disposal; LATE present).
#[test]
fn consult_pool_is_as_of_at_not_end_of_timeline() {
    let events = vec![
        buy(
            "EARLY",
            datetime!(2025-02-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        buy(
            "LATE",
            datetime!(2025-09-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(2000),
        ),
        // a later REAL disposal (method-based, no injected selection) that consumes a lot after `at`.
        sell(
            "DISP",
            datetime!(2025-10-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(20000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();
    let r = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 06 - 01), Some(dec!(10000))),
    )
    .expect("consult succeeds");

    // As of 2025-06-01 only EARLY is held (LATE not yet acquired; DISP not yet executed).
    assert_eq!(
        r.proposed_selection,
        vec![pick("EARLY", LOT)],
        "the as-of-`at` pool holds only EARLY"
    );
    assert!(
        !r.proposed_selection.iter().any(|p| p.lot == lid("LATE")),
        "a lot acquired after `at` must never be selectable"
    );
}

// ── KAT: state_as_of mixed-tz straddle (fold.rs break→continue fix) ────────────────────────────────

/// Mixed-timezone straddle: an event dated `at+1` in a +14:00 timezone has an EARLIER utc timestamp
/// than a disposal dated `at` in +00:00. `sort_canonical` (utc-ascending) therefore places the `at+1`
/// event BEFORE the `at` disposal in the timeline. Under the old `break`, the loop fires on the `at+1`
/// event (date > at) and exits without processing the `at` disposal — the lot is wrongly still held.
/// Under the corrected `continue`, the `at+1` event is skipped and the `at` disposal IS processed —
/// the lot is consumed and absent from `st.lots`.
///
/// UTC order after sort_canonical: [ACQ (2025-03-01 00:00Z), NOISE (2025-06-01 12:00Z / +14→date 2025-06-02),
/// DISP (2025-06-01 23:00Z / +00→date 2025-06-01)].
/// at = 2025-06-01.  NOISE.date() = 2025-06-02 > at → old break skips DISP; new continue does not.
///
/// FAILS under break (st.lots is non-empty; no disposal recorded).
/// PASSES under continue (lot consumed; one disposal recorded).
#[test]
fn state_as_of_mixed_tz_straddle_disposal_not_skipped() {
    let at = date!(2025 - 06 - 01);

    // ACQ: UTC 2025-03-01 00:00:00 +00:00, tz +00:00  →  tax date 2025-03-01 (well before `at`)
    let acq = LedgerEvent {
        id: eid("ACQ"),
        utc_timestamp: datetime!(2025-03-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold()),
        payload: EventPayload::Acquire(Acquire {
            sat: LOT,
            usd_cost: dec!(50000),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };

    // NOISE: UTC 2025-06-01 12:00:00, tz +14:00
    //   local = 2025-06-02 02:00:00 +14:00  →  tax date 2025-06-02 (> at)
    // Sorts BEFORE DISP in UTC order, triggering the old break.
    let noise = LedgerEvent {
        id: eid("NOISE"),
        utc_timestamp: datetime!(2025-06-01 12:00:00 UTC),
        original_tz: offset!(+14:00),
        wallet: Some(cold()),
        payload: EventPayload::Acquire(Acquire {
            sat: 1_000,
            usd_cost: dec!(50),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    };

    // DISP: UTC 2025-06-01 23:00:00 +00:00, tz +00:00  →  tax date 2025-06-01 (= at, must be included)
    // Sorts AFTER NOISE in UTC order; the old break skipped it.
    let disp = LedgerEvent {
        id: eid("DISP"),
        utc_timestamp: datetime!(2025-06-01 23:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(cold()),
        payload: EventPayload::Dispose(Dispose {
            sat: LOT,
            usd_proceeds: dec!(60000),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    };

    let events = vec![acq, noise, disp];
    let prices = StaticPrices::default();
    let config = cfg();

    let res = resolve(&events, &prices, &config);
    let st = state_as_of(res, &prices, &config, at);

    assert!(
        st.blockers.is_empty(),
        "no blockers expected (clean disposal); got: {:?}",
        st.blockers
    );
    // The at-dated disposal (DISP, date = 2025-06-01 = at) must have consumed the lot from ACQ.
    // Under the old `break` this fails: st.lots has one entry (lot not consumed).
    assert!(
        st.lots.is_empty(),
        "the at-dated disposal must consume the lot: st.lots should be empty but is {:?}",
        st.lots
    );
    // Cross-check: exactly one disposal record must have been produced.
    assert_eq!(
        st.disposals.len(),
        1,
        "exactly one disposal (DISP, date = at) must be recorded; got {:?}",
        st.disposals.len()
    );
}

// ── KAT: `--proceeds` required for a future date ────────────────────────────────────────────────────

/// A future-ish date with NO dataset price and `proceeds = None` → `Err(Evaluate(ProceedsRequired))`;
/// supplying `proceeds = Some(..)` → `Ok`.
#[test]
fn future_date_requires_proceeds() {
    let events = vec![buy(
        "X",
        datetime!(2025-02-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let prices = StaticPrices::default(); // empty → no FMV for any date
    let tables = synth(2025);
    let prof = profile();

    let err = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 12 - 20), None),
    )
    .expect_err("no price + no proceeds ⇒ error");
    assert_eq!(
        err,
        OptimizeError::Evaluate(EvaluateError::ProceedsRequired)
    );

    let ok = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 12 - 20), Some(dec!(60000))),
    );
    assert!(ok.is_ok(), "explicit proceeds ⇒ Ok");
}

// ── KAT: the consult writes NOTHING ────────────────────────────────────────────────────────────────

/// READ-ONLY (load-bearing): `consult_sale` produces no event / no mutation / no decision. `events`
/// is byte-identical before/after, and the canonical projection is unperturbed.
#[test]
fn consult_writes_nothing() {
    let events = vec![
        buy(
            "LB",
            datetime!(2025-01-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        buy(
            "HB",
            datetime!(2025-02-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
    ];
    let events_before = events.clone();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let proj_before = project(&events, &prices, &cfg());
    let _ = consult_sale(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 06 - 01), Some(dec!(10000))),
    )
    .expect("consult succeeds");

    assert_eq!(
        events, events_before,
        "events slice is byte-identical (no append)"
    );
    let proj_after = project(&events, &prices, &cfg());
    assert_eq!(
        proj_before.lots, proj_after.lots,
        "the canonical projection's lots are unperturbed"
    );
    assert_eq!(
        proj_before.disposals.len(),
        proj_after.disposals.len(),
        "no disposal was added to the ledger"
    );
}

// ── KAT: determinism ───────────────────────────────────────────────────────────────────────────────

/// NFR4: identical inputs → byte-identical `ConsultReport`.
#[test]
fn consult_is_deterministic() {
    let events = vec![
        buy(
            "L",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
        buy(
            "M",
            datetime!(2025-02-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();
    let r = || {
        consult_sale(
            &events,
            &prices,
            &cfg(),
            Some(&prof),
            &tables,
            &req(LOT, date!(2025 - 02 - 15), Some(dec!(50000))),
        )
        .unwrap()
    };
    assert_eq!(r(), r());
}
