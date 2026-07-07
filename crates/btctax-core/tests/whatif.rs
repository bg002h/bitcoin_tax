//! btctax-core::whatif — phase P1 KATs (task #43).
//!
//! `whatif::sell` is the READ-ONLY hypothetical-sale what-if: inject a synthetic `Op::Dispose`, read
//! the MARGINAL federal tax (`withhyp.total − baseline.total`; the no-crypto term cancels), the §1212
//! carryforward delta, the §1(h) bracket, and the §1411 NIIT delta — every dollar straight from
//! `compute_tax_year`. It writes NOTHING (clone-fold-discard).
//!
//! Load-bearing invariants exercised here (goldens hand-derived from the `synth()` table — Single:
//! ordinary 0→10%, 50k→22%, 250k→32%; §1(h) max_zero=40k, max_fifteen=400k; NIIT 3.8% / $200k Single):
//! - marginal == withhyp.total − baseline.total (exact), and it CANCELS the shared no-crypto term
//!   (a year WITH real disposals → marginal ≠ the whole-year figure).
//! - [R0-I1] §1212 carryforward delta reported; the this-year ordinary offset is the `loss_deduction`
//!   DELTA — $0 (NOT $3,000) when the baseline already consumes the §1211(b) cap.
//! - [R0-I2] `niit_incremental` = `withhyp.niit − baseline.niit` (the DELTA), NEGATIVE for a
//!   NIIT-reducing loss harvest — NOT the raw `MarginalRates.niit_applies` flag.
//! - §1(h) bracket 0/15/20 by stacking; NIIT crossing; effective rate (guarded for gain ≤ 0).
//! - refusals inherited: missing table/profile, pre-2025, future-no-price, NoLots, Hard blocker.
//! - non-persistence: `events` byte-identical + the projection unperturbed.
//!
//! All fixtures synthetic (privacy); exact Decimal, no float (NFR5). Federal-only.
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::Term;
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxProfile};
use btctax_core::whatif::{
    sell, HarvestTarget, HarvestTargetParseError, LtcgBracket, SellRequest, SellStatus, WhatIfError,
};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

const LOT: i64 = 100_000_000; // one whole BTC per lot

// ── synthetic table + profile builders (same schedule as optimize_mode2.rs) ───────────────────────
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
fn profile_full(ord: Usd, magi: Usd, qd: Usd, cf_long: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: qd,
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: cf_long,
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    }
}
fn profile(ord: Usd, magi: Usd) -> TaxProfile {
    profile_full(ord, magi, dec!(0), dec!(0))
}

// ── event / id builders ──────────────────────────────────────────────────────────────────────────
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn hot() -> WalletId {
    WalletId::SelfCustody {
        label: "hot".into(),
    }
}
fn eid(rf: &str) -> EventId {
    EventId::import(Source::Swan, SourceRef::new(rf))
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
fn real_sell(
    rf: &str,
    ts: time::OffsetDateTime,
    w: WalletId,
    sat: i64,
    proceeds: Usd,
) -> LedgerEvent {
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
/// A hypothetical Sell of `sat` from `cold` at `at`, priced per-BTC (`price`; None ⇒ dataset FMV),
/// consumed by the STANDING method (no injected selection).
fn req(sat: i64, at: time::Date, price: Option<Usd>) -> SellRequest {
    SellRequest {
        sell_sat: sat,
        wallet: cold(),
        at,
        price,
        method: None,
    }
}

// ── marginal identity ──────────────────────────────────────────────────────────────────────────────

/// The report's `marginal_tax` IS EXACTLY `withhyp.total − baseline.total`. Single, ord 60,000; a lone
/// hyp LT sale of 1 BTC for $30,000 (basis $10,000) → $20,000 LT gain sitting (60k,80k] in the 15%
/// band → $3,000; baseline (no crypto) total 0 → marginal $3,000.
#[test]
fn whatif_marginal_equals_withhyp_minus_baseline() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(60000), dec!(60000));
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(30000))),
    )
    .expect("sell computes");

    assert_eq!(r.lt_gain, dec!(20000));
    assert_eq!(r.st_gain, dec!(0));
    // Definitional: the report field is the exact subtraction.
    assert_eq!(
        r.marginal_tax,
        r.withhyp.total_federal_tax_attributable - r.baseline.total_federal_tax_attributable
    );
    assert_eq!(r.marginal_tax, dec!(3000.00));
    assert_eq!(r.baseline.total_federal_tax_attributable, dec!(0.00));
}

/// The no-crypto term cancels: on a year that ALREADY has a real disposal, `marginal_tax` isolates the
/// hypothetical's own effect and is NOT the whole-year figure. Single, ord 60,000, QD 10,000
/// (magi 70,000). Real LT disposal $20,000 gain; hyp LT sale $20,000 gain (both 15% band).
///   baseline: pref(qd 10k + 20k LT on 60k) − pref(qd 10k) = 4,500 − 1,500 = 3,000.
///   withhyp:  pref(qd 10k + 40k LT on 60k) − pref(qd 10k) = 7,500 − 1,500 = 6,000.
///   marginal = 6,000 − 3,000 = 3,000 (≠ the 6,000 whole-year figure — the naive over-report).
#[test]
fn whatif_marginal_cancels_no_crypto_term() {
    let events = vec![
        buy(
            "A",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
        // a REAL LT disposal already in the year (consumes A under FIFO): $30,000 − $10,000 = $20,000.
        // Dated 2025-07-01 so it is LONG-TERM (> 1yr after the 2024-06-01 acquisition).
        real_sell(
            "DISP",
            datetime!(2025-07-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(30000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_full(dec!(60000), dec!(70000), dec!(10000), dec!(0));
    // hyp sale of the remaining 1 BTC (B) for $30,000 → $20,000 LT gain.
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(30000))),
    )
    .expect("sell computes");

    assert_eq!(r.lt_gain, dec!(20000));
    assert_eq!(r.baseline.total_federal_tax_attributable, dec!(3000.00)); // the real disposal alone
    assert_eq!(r.withhyp.total_federal_tax_attributable, dec!(6000.00)); // whole-year (real + hyp)
    assert_eq!(r.marginal_tax, dec!(3000.00)); // the sale's OWN effect — NOT 6,000
    assert_ne!(r.marginal_tax, r.withhyp.total_federal_tax_attributable);
}

// ── §1(h) bracket ──────────────────────────────────────────────────────────────────────────────────

/// The §1(h) bracket is read from the WITH-scenario `pref_split` (P0). Three stacking cases:
/// 0% (top ≤ max_zero), 15% ((max_zero, max_fifteen]), 20% (> max_fifteen). Room = dollars until the
/// next breakpoint (`None` at 20%).
#[test]
fn sell_reports_correct_ltcg_bracket() {
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];

    // 0% band: ord 0, LT gain 30,000 → top 30,000 < 40,000.
    let r0 = sell(
        &events,
        &prices,
        &cfg(),
        Some(&profile(dec!(0), dec!(0))),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(31000))),
    )
    .unwrap();
    assert_eq!(r0.lt_gain, dec!(30000));
    assert_eq!(r0.bracket, LtcgBracket::Zero);
    assert_eq!(r0.bracket_room, Some(dec!(10000))); // 40,000 − 30,000
    assert_eq!(r0.marginal_tax, dec!(0.00)); // all in the 0% band

    // 15% band: ord 60,000, LT gain 20,000 → top 80,000 in (40k, 400k].
    let r15 = sell(
        &events,
        &prices,
        &cfg(),
        Some(&profile(dec!(60000), dec!(60000))),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(21000))),
    )
    .unwrap();
    assert_eq!(r15.bracket, LtcgBracket::Fifteen);
    assert_eq!(r15.bracket_room, Some(dec!(320000))); // 400,000 − 80,000

    // 20% band: ord 500,000, LT gain 100,000 → top 600,000 > 400,000.
    let r20 = sell(
        &events,
        &prices,
        &cfg(),
        Some(&profile(dec!(500000), dec!(500000))),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(101000))),
    )
    .unwrap();
    assert_eq!(r20.bracket, LtcgBracket::Twenty);
    assert_eq!(r20.bracket_room, None); // top bracket — no headroom
}

// ── §1411 NIIT crossing ──────────────────────────────────────────────────────────────────────────────

/// A hyp LT sale pushes MAGI over the $200k Single threshold. ord/magi 190,000; LT gain 30,000 →
/// magi_with 220,000 (over by 20,000); NII 30,000 → niit 3.8%·min(30k,20k)=760. Baseline niit 0 ⇒
/// `niit_incremental` == 760, `niit_applies` true; marginal = ltcg 4,500 + niit 760 = 5,260.
#[test]
fn sell_niit_crossing() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(190000), dec!(190000));
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(40000))),
    )
    .expect("sell computes");
    assert_eq!(r.lt_gain, dec!(30000));
    assert_eq!(r.niit_incremental, dec!(760.00));
    assert!(r.niit_applies);
    assert_eq!(r.marginal_tax, dec!(5260.00));
    assert_eq!(r.bracket, LtcgBracket::Fifteen);
}

// ── effective rate ───────────────────────────────────────────────────────────────────────────────────

/// `effective_rate` = marginal ÷ gain for a gain sale; `None` for a loss/zero sale. ord 60,000, LT gain
/// 20,000 all at 15% → marginal 3,000 → 0.15.
#[test]
fn sell_effective_rate() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&profile(dec!(60000), dec!(60000))),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(30000))),
    )
    .unwrap();
    assert_eq!(r.effective_rate, Some(dec!(0.15)));
    assert_eq!(r.status, SellStatus::Gain);
}

// ── §1212 carryforward + this-year offset (R0-I1) ────────────────────────────────────────────────────

/// A loss sale reports the §1212(b) carryforward DELTA + the this-year ordinary offset (= the
/// `loss_deduction` delta), NEVER a hard-coded $3,000. ord 60,000; hyp LT LOSS $40,000 (basis $50,000,
/// sold $10,000). §1211(b) caps the current offset at $3,000 (22% band → marginal −660); $37,000 carried.
#[test]
fn whatif_sell_loss_reports_carryforward_delta() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(50000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(60000), dec!(60000));
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(10000))),
    )
    .expect("sell computes");

    assert_eq!(r.lt_gain, dec!(-40000));
    assert_eq!(r.status, SellStatus::Loss);
    assert_eq!(r.effective_rate, None); // gain ≤ 0 → guarded
                                        // this-year ordinary offset = the loss_deduction delta = $3,000 (baseline consumed none).
    assert_eq!(r.ordinary_offset_delta, dec!(3000));
    // §1212(b): $40,000 loss − $3,000 used = $37,000 carried to next year (LT character).
    assert_eq!(r.carryforward_delta.long, dec!(37000));
    assert_eq!(r.carryforward_delta.short, dec!(0));
    // marginal = ordinary_delta only: tax(57,000) − tax(60,000) = 6,540 − 7,200 = −660.
    assert_eq!(r.marginal_tax, dec!(-660.00));
    assert_eq!(r.niit_incremental, dec!(0.00));
}

/// [R0-I1] When the baseline ALREADY consumes the §1211(b) cap (here via a $10,000 carryforward-in),
/// the sale's this-year ordinary offset = the `loss_deduction` DELTA = **$0** (NOT $3,000) — the whole
/// hyp loss is carried. ord 60,000, carryforward-in long $10,000; hyp LT LOSS $40,000.
///   baseline: cf 10,000 → loss_deduction 3,000, carryforward_out 7,000.
///   withhyp:  cf 10,000 + hyp 40,000 = 50,000 loss → loss_deduction 3,000 (same cap), carryforward 47,000.
///   offset delta = 3,000 − 3,000 = 0 ; carryforward delta = 47,000 − 7,000 = 40,000 (all carried).
#[test]
fn whatif_sell_offset_delta_is_zero_when_baseline_caps() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(50000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_full(dec!(60000), dec!(60000), dec!(0), dec!(10000)); // $10k LT carryforward-in
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(10000))),
    )
    .expect("sell computes");

    assert_eq!(r.lt_gain, dec!(-40000));
    // THE POINT: $0 additional ordinary offset (baseline already at the §1211(b) cap) — NOT $3,000.
    assert_eq!(r.ordinary_offset_delta, dec!(0));
    // ALL $40,000 of the hyp loss is carried forward (LT character).
    assert_eq!(r.carryforward_delta.long, dec!(40000));
    // and the current-year marginal is $0 (no additional this-year deduction unlocked).
    assert_eq!(r.marginal_tax, dec!(0.00));
}

/// A carried-in loss is consumed FIRST by a hyp GAIN: the gain is absorbed (zero marginal LTCG) and
/// BURNS carryforward (a negative carryforward delta). ord 60,000, carryforward-in long $50,000; hyp LT
/// GAIN $20,000 → net still a $30,000 loss → loss_deduction 3,000 both scenarios, marginal 0; the gain
/// burned $20,000 of carryforward (47,000 → 27,000, a −20,000 delta).
#[test]
fn carryforward_in_consumed_first() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_full(dec!(60000), dec!(60000), dec!(0), dec!(50000)); // $50k LT carryforward-in
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(30000))),
    )
    .expect("sell computes");

    assert_eq!(r.lt_gain, dec!(20000));
    // The gain is absorbed by the carried-in loss → no preferential tax, zero marginal.
    assert_eq!(r.marginal_tax, dec!(0.00));
    assert_eq!(r.ordinary_offset_delta, dec!(0)); // cap already consumed both sides
                                                  // carryforward BURNED by the gain: −$20,000 (consumed-first).
    assert_eq!(r.carryforward_delta.long, dec!(-20000));
}

// ── §1411 NIIT DELTA, not the raw flag (R0-I2) ───────────────────────────────────────────────────────

/// [R0-I2] A NIIT-REDUCING loss harvest on a year with a real GAIN disposal: `niit_incremental` is the
/// DELTA (`withhyp.niit − baseline.niit`) and is NEGATIVE — the raw `MarginalRates.niit_applies` (which
/// still says "applies") is NOT used as the signal. ord/magi 190,000. The low-basis lot (in `hot`) is
/// the REAL gain disposal; the high-basis lot (in `cold`) is the hyp LOSS sale — separate wallets, so
/// the schedule is unambiguous regardless of the standing method.
///   baseline (real gain $40k): magi 230k, NII 40k, niit 3.8%·min(40k,30k)=1,140; pref 6,000 → total 7,140.
///   withhyp  (gain $40k + hyp loss $20k = net $20k): magi 210k, NII 20k, niit 3.8%·min(20k,10k)=380;
///            pref 3,000 → total 3,380.
///   niit_incremental = 380 − 1,140 = −760 (< 0); marginal = 3,380 − 7,140 = −3,760.
#[test]
fn whatif_niit_incremental_not_raw_flag() {
    let events = vec![
        // low-basis lot in `hot` → the REAL gain disposal: $50,000 − $10,000 = $40,000 LT gain.
        buy(
            "A",
            datetime!(2024-01-01 00:00:00 UTC),
            hot(),
            LOT,
            dec!(10000),
        ),
        real_sell(
            "DISP",
            datetime!(2025-03-01 00:00:00 UTC),
            hot(),
            LOT,
            dec!(50000),
        ),
        // high-basis lot in `cold` → the hyp LOSS sale.
        buy(
            "B",
            datetime!(2024-02-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(60000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(190000), dec!(190000));
    // hyp LOSS sale of the remaining 1 BTC (B) for $40,000 → $40,000 − $60,000 = −$20,000 LT loss.
    let r = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(40000))),
    )
    .expect("sell computes");

    assert_eq!(r.lt_gain, dec!(-20000)); // the hyp sale itself is a loss
                                         // THE POINT: the NIIT DELTA is NEGATIVE (the harvest REDUCED NIIT)…
    assert_eq!(r.niit_incremental, dec!(-760.00));
    assert!(r.niit_incremental < Usd::ZERO);
    // …even though the RAW crypto-vs-no-crypto flag on the with-scenario still says "applies".
    assert!(
        r.withhyp.marginal_rates.niit_applies,
        "the raw flag would MISreport 'NIIT applies' — the delta is the honest signal"
    );
    assert_eq!(r.marginal_tax, dec!(-3760.00));
}

// ── refusal taxonomy ─────────────────────────────────────────────────────────────────────────────────

/// Pre-2025 date ⇒ `PreTransitionYear` (a restatement of a closed year, not a plan).
#[test]
fn sell_refuses_pre_2025() {
    let events = vec![buy(
        "L",
        datetime!(2023-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let err = sell(
        &events,
        &StaticPrices::default(),
        &cfg(),
        Some(&profile(dec!(0), dec!(0))),
        &synth(2024),
        &req(LOT, date!(2024 - 06 - 01), Some(dec!(10000))),
    )
    .expect_err("pre-2025 refuses");
    assert_eq!(err, WhatIfError::PreTransitionYear(2024));
}

/// A future/off-dataset date with NO `--price` and no dataset FMV ⇒ `Evaluate(ProceedsRequired)`.
/// (Lots ARE available, so this is not `NoLots`.)
#[test]
fn sell_refuses_future_no_price() {
    let events = vec![buy(
        "L",
        datetime!(2025-02-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let err = sell(
        &events,
        &StaticPrices::default(), // empty → no FMV for any date
        &cfg(),
        Some(&profile(dec!(0), dec!(0))),
        &synth(2025),
        &req(LOT, date!(2025 - 12 - 20), None),
    )
    .expect_err("future date, no price ⇒ ProceedsRequired");
    assert_eq!(
        err,
        WhatIfError::Evaluate(btctax_core::project::EvaluateError::ProceedsRequired)
    );
}

/// A missing profile ⇒ `YearNotComputable` (the engine's `TaxProfileMissing`, inherited verbatim).
#[test]
fn sell_refuses_missing_profile() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let err = sell(
        &events,
        &StaticPrices::default(),
        &cfg(),
        None, // no profile
        &synth(2025),
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(10000))),
    )
    .expect_err("no profile refuses");
    assert!(matches!(err, WhatIfError::YearNotComputable(_)));
}

/// A missing table for the sale year ⇒ `YearNotComputable` (`TaxTableMissing`). Sell in 2026 with only
/// 2025 bundled.
#[test]
fn sell_refuses_missing_table() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let err = sell(
        &events,
        &StaticPrices::default(),
        &cfg(),
        Some(&profile(dec!(0), dec!(0))),
        &synth(2025), // only 2025 bundled
        &req(LOT, date!(2026 - 06 - 01), Some(dec!(10000))),
    )
    .expect_err("no table for 2026 refuses");
    assert!(matches!(err, WhatIfError::YearNotComputable(_)));
}

/// Selling more than the as-of pool holds ⇒ `NoLots`.
#[test]
fn sell_refuses_no_lots() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let err = sell(
        &events,
        &StaticPrices::default(),
        &cfg(),
        Some(&profile(dec!(0), dec!(0))),
        &synth(2025),
        &req(2 * LOT, date!(2025 - 08 - 01), Some(dec!(10000))), // pool holds only 1 BTC
    )
    .expect_err("insufficient pool ⇒ NoLots");
    assert_eq!(err, WhatIfError::NoLots);
}

/// A Hard blocker ANYWHERE in the projection ⇒ `YearNotComputable` (inherited from the engine gate). An
/// uncovered real disposal in `hot` is Hard; the hyp sale from `cold` (which HAS lots) still refuses.
#[test]
fn sell_refuses_on_hard_blocker() {
    let events = vec![
        buy(
            "L",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        // an UNCOVERED disposal in `hot` (no prior acquire there) → Hard UncoveredDisposal blocker.
        real_sell(
            "BAD",
            datetime!(2025-03-01 00:00:00 UTC),
            hot(),
            LOT,
            dec!(20000),
        ),
    ];
    let err = sell(
        &events,
        &StaticPrices::default(),
        &cfg(),
        Some(&profile(dec!(0), dec!(0))),
        &synth(2025),
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(10000))),
    )
    .expect_err("a Hard blocker gates the year");
    assert!(matches!(err, WhatIfError::YearNotComputable(_)));
}

// ── non-persistence (core level) ─────────────────────────────────────────────────────────────────────

/// `whatif::sell` writes NOTHING: the `events` slice is byte-identical and the canonical projection is
/// unperturbed after any sell (the CLI-level vault-bytes KAT is `whatif_never_persists`).
#[test]
fn sell_writes_nothing() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let before = events.clone();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(60000), dec!(60000));
    let proj_before = project(&events, &prices, &cfg());

    let _ = sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &req(LOT, date!(2025 - 08 - 01), Some(dec!(30000))),
    )
    .expect("sell computes");

    assert_eq!(events, before, "events slice is byte-identical (no append)");
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

// ── lots-consumed schedule + determinism ─────────────────────────────────────────────────────────────

/// The per-lot schedule reports the consumed lot's id/sat/basis/term/gain and the sold date; and the
/// result is deterministic (NFR4).
#[test]
fn sell_reports_lot_schedule_and_is_deterministic() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(60000), dec!(60000));
    let call = || {
        sell(
            &events,
            &prices,
            &cfg(),
            Some(&prof),
            &tables,
            &req(LOT, date!(2025 - 08 - 01), Some(dec!(30000))),
        )
        .unwrap()
    };
    let r = call();
    assert_eq!(r.lots.len(), 1);
    let leg = &r.lots[0];
    assert_eq!(leg.sat, LOT);
    assert_eq!(leg.basis, dec!(10000));
    assert_eq!(leg.gain, dec!(20000));
    assert_eq!(leg.term, Term::LongTerm);
    assert_eq!(leg.sold_at, date!(2025 - 08 - 01));
    assert_eq!(leg.acquired_at, date!(2024 - 06 - 01));
    assert_eq!(r, call(), "NFR4: identical inputs → identical report");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// P1 — harvest-target `FromStr` dedup (task #48). The single source of truth shared by the CLI
// `--target` parse and the TUI panel; a PURE LEXER — accepts/rejects EXACTLY what the legacy
// `parse_harvest_target` did, adding no new checks (in particular it does NOT reject negatives).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// KAT: every accepted `--target` form parses to the same `HarvestTarget` the pre-refactor parsers
/// produced — the three aliases each (incl. case-insensitive `GAIN=`), `$`/comma-optional amounts
/// (`gain=$1,000` == `gain=1000`), `tax=$0`. Rejections are limited to unrecognized strings / empty /
/// a `Usd`-invalid amount (`gain=abc`). Separator note: only `$`/`,` are stripped; `_` is left intact
/// but `rust_decimal` accepts it as a digit separator, so `gain=1_000` → `Gain(1000)` (exactly what the
/// legacy lexer produced — byte-for-byte parity; NOT a `BadAmount`, and a `_`-reject would be a NEW
/// check that breaks parity).
#[test]
fn harvest_target_fromstr_matches_prior_parsers() {
    use HarvestTarget::*;
    // Bracket aliases, case-insensitive, all three spellings.
    for s in [
        "zero-ltcg",
        "zero_ltcg",
        "zeroltcg",
        "ZERO-LTCG",
        "  ZeroLtcg  ",
    ] {
        assert_eq!(s.parse::<HarvestTarget>(), Ok(ZeroLtcg), "{s:?}");
    }
    for s in [
        "fifteen-ltcg",
        "fifteen_ltcg",
        "fifteenltcg",
        "FIFTEEN-LTCG",
    ] {
        assert_eq!(s.parse::<HarvestTarget>(), Ok(FifteenLtcg), "{s:?}");
    }
    // gain=/tax= with the `$`/comma cleaning; case-insensitive prefix.
    assert_eq!("gain=1000".parse::<HarvestTarget>(), Ok(Gain(dec!(1000))));
    assert_eq!("gain=$1,000".parse::<HarvestTarget>(), Ok(Gain(dec!(1000))));
    assert_eq!(
        "gain=$1,000".parse::<HarvestTarget>(),
        "gain=1000".parse::<HarvestTarget>(),
        "$/comma are optional"
    );
    assert_eq!(
        "GAIN=$25,000".parse::<HarvestTarget>(),
        Ok(Gain(dec!(25000)))
    );
    assert_eq!("tax=$0".parse::<HarvestTarget>(), Ok(Tax(dec!(0))));
    assert_eq!(
        "tax=1500.50".parse::<HarvestTarget>(),
        Ok(Tax(dec!(1500.50)))
    );
    // Rejections — unrecognized / empty → UnrecognizedTarget; bad amount → BadAmount.
    assert!(matches!(
        "nonsense".parse::<HarvestTarget>(),
        Err(HarvestTargetParseError::UnrecognizedTarget(_))
    ));
    assert!(matches!(
        "".parse::<HarvestTarget>(),
        Err(HarvestTargetParseError::UnrecognizedTarget(_))
    ));
    assert!(matches!(
        "gain=abc".parse::<HarvestTarget>(),
        Err(HarvestTargetParseError::BadAmount(_))
    ));
    // Separator golden: `_` is NOT stripped (only `$`/`,`), but `rust_decimal` accepts `_` as a digit
    // separator, so `gain=1_000` parses to `Gain(1000)` — byte-identical to the legacy lexer (which
    // also only stripped `$`/`,`). Rejecting `_` here would be a NEW check that breaks parity.
    assert_eq!("gain=1_000".parse::<HarvestTarget>(), Ok(Gain(dec!(1000))));
}

/// [★ C1] KAT: the lexer does NOT reject negatives. `gain=-1` → `Gain(-1)` (NOT a parse error); the
/// ENGINE refuses it downstream as `InvalidTarget`. A parser-side reject would move the refusal
/// (different class/path/message) and break parity — and is untested at the CLI, so it would ship
/// silently. This pins the pure-lexer contract.
#[test]
fn harvest_target_gain_negative_parses_not_rejected() {
    assert_eq!(
        "gain=-1".parse::<HarvestTarget>(),
        Ok(HarvestTarget::Gain(dec!(-1)))
    );
    assert_eq!(
        "tax=-1".parse::<HarvestTarget>(),
        Ok(HarvestTarget::Tax(dec!(-1)))
    );
    // With the `$`/comma cleaning too.
    assert_eq!(
        "gain=-$1,000".parse::<HarvestTarget>(),
        Ok(HarvestTarget::Gain(dec!(-1000)))
    );
}
