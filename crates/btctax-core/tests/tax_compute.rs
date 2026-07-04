//! Sub-project B, Task 5 — `compute_tax_year` KATs (mechanics on SYNTHETIC tables).
//!
//! Every golden is hand-derived from `synth()` (Single: ordinary 0→10%, 50k→22%, 250k→32%; §1(h) LT
//! max_zero=40k, max_fifteen=400k) and the statutory NIIT (3.8%, $200k Single threshold). Real-number
//! goldens against the bundled TY2025 table are Task 7. Exact Decimal throughout; no float (NFR5).
//!
//! Fixtures: the all-2025 cases (mining income, ST buy+sell, FMV-missing refusal) run the REAL `project`
//! so the read-from-projection path is exercised end-to-end; the LT-in-2025 and gate/refusal cases build
//! `LedgerState` directly (pre-2025 acquisition + post-2025 disposal needs the §7.4 transition seed, which
//! is orthogonal to the assembly under test) — both drive the same `compute_tax_year`.
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::tax::compute::compute_tax_year;
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

// ── synthetic table + profile ──────────────────────────────────────────────────────────────────────
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
fn profile(ord: Usd, magi: Usd, qd: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: ord,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: qd,
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
/// Single profile with a non-zero `other_net_capital_gain` (LT-character) — for the B-M1 loss-year KATs.
fn profile_with_ncg(ord: Usd, magi: Usd, qd: Usd, ncg: Usd) -> TaxProfile {
    TaxProfile {
        other_net_capital_gain: ncg,
        ..profile(ord, magi, qd)
    }
}
/// Synthetic table keyed for MFS (same bracket/breakpoint shape as `synth`) — for the MFS $1,500 KAT.
/// Only the NII assertion depends on this table; the ordinary/preferential brackets merely need to exist.
fn synth_mfs(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Mfs,
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
        FilingStatus::Mfs,
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

// ── direct-state builders (drive compute_tax_year's read path with exact disposals/income) ──────────
fn disposal(d: time::Date, gain: Usd, term: Term) -> Disposal {
    Disposal {
        event: EventId::decision(1),
        kind: DisposeKind::Sell,
        disposed_at: d,
        legs: vec![DisposalLeg {
            lot_id: LotId {
                origin_event_id: EventId::decision(0),
                split_sequence: 0,
            },
            sat: 100_000,
            proceeds: gain,
            basis: dec!(0),
            gain,
            term,
            basis_source: BasisSource::ComputedFromCost,
            gift_zone: None,
            acquired_at: date!(2025 - 01 - 01), // synthetic; compute_tax_year does not read acquired_at
            wallet: WalletId::Exchange {
                provider: "cb".into(),
                account: "m".into(),
            }, // synthetic
            pseudo: false,
        }],
        fee_mini_disposition: false,
    }
}
fn income_rec(d: time::Date, fmv: Usd) -> IncomeRecord {
    IncomeRecord {
        event: EventId::decision(2),
        recognized_at: d,
        sat: 100_000,
        usd_fmv: fmv,
        kind: IncomeKind::Mining,
        business: false,
    }
}
fn income_rec_interest(d: time::Date, fmv: Usd) -> IncomeRecord {
    IncomeRecord {
        event: EventId::decision(3),
        recognized_at: d,
        sat: 100_000,
        usd_fmv: fmv,
        kind: IncomeKind::Interest,
        business: false,
    }
}
fn state_with(disposals: Vec<Disposal>, income: Vec<IncomeRecord>) -> LedgerState {
    LedgerState {
        disposals,
        income_recognized: income,
        ..Default::default()
    }
}

// ── real-projection event helpers (mirror kat_tax.rs:16-34) ─────────────────────────────────────────
fn wal() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn ev(src: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(src)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(wal()),
        payload: p,
    }
}

// ── KATs ─────────────────────────────────────────────────────────────────────────────────────────

/// Double-count guard (I5): crypto ordinary income is added to the ordinary stack EXACTLY ONCE.
/// $10,000 mining, OTI 60,000 (already in the 22% band). No disposals.
/// ordinary_delta = tax(70,000) − tax(60,000) = 0.22·10,000 = 2,200.00. No LT, no NIIT (magi 70k < 200k).
#[test]
fn double_count_guard_crypto_ordinary_income_added_exactly_once() {
    let inc = ev(
        "MINE",
        datetime!(2025-04-01 00:00:00 UTC),
        EventPayload::Income(Income {
            sat: 100_000_000,
            usd_fmv: Some(dec!(10000.00)),
            fmv_status: FmvStatus::ManualEntry,
            kind: IncomeKind::Mining,
            business: false,
        }),
    );
    let events = vec![inc];
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let out = compute_tax_year(
        &events,
        &st,
        2025,
        Some(&profile(dec!(60000), dec!(60000), dec!(0))),
        &synth(2025),
    );
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    assert_eq!(r.ordinary_from_crypto, dec!(10000.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(2200.00)); // counted ONCE (not 4,400)
    assert_eq!(r.ltcg_tax, dec!(0.00));
    assert_eq!(r.niit, dec!(0.00));
}

/// [P2-D D5] STANDALONE: the §1401 SE tax is NOT folded into `total_federal_tax_attributable`.
/// A Single, $100,000 **business** mining year (OTI 0, synth brackets) has an ordinary-income delta
/// of tax(100,000) − tax(0) = 0.10·50,000 + 0.22·50,000 = $16,000.00 — the crypto mining is taxed as
/// ordinary income exactly once (unchanged by P2-D). The §1401 SE tax ($14,129.55) is computed
/// SEPARATELY by `compute_se_tax` and MUST NOT be added to the engine-B total.
#[test]
fn se_tax_is_standalone_not_in_total_federal_tax_attributable() {
    let biz_mining = IncomeRecord {
        event: EventId::decision(2),
        recognized_at: date!(2025 - 03 - 01),
        sat: 100_000_000,
        usd_fmv: dec!(100000),
        kind: IncomeKind::Mining,
        business: true,
    };
    let st = state_with(vec![], vec![biz_mining]);
    let table = synth(2025);
    let out = compute_tax_year(
        &[],
        &st,
        2025,
        Some(&profile(dec!(0), dec!(0), dec!(0))),
        &table,
    );
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    // Engine-B total = income-tax delta ONLY (mining as ordinary income): tax(100k) − tax(0) = 16,000.
    assert_eq!(r.ordinary_from_crypto, dec!(100000));
    assert_eq!(r.total_federal_tax_attributable, dec!(16000.00));
    // SE tax computed SEPARATELY (standalone) — NOT added to total_federal_tax_attributable (D5).
    let se = btctax_core::compute_se_tax(
        &st,
        2025,
        FilingStatus::Single,
        &table.0,
        btctax_core::conventions::Usd::ZERO,
        btctax_core::conventions::Usd::ZERO,
        btctax_core::conventions::Usd::ZERO,
    )
    .expect("SE tax expected");
    assert_eq!(se.total, dec!(14129.55));
    assert_ne!(
        r.total_federal_tax_attributable,
        dec!(16000.00) + se.total,
        "SE tax must NOT be folded into total_federal_tax_attributable (D5)"
    );
}

/// Net ST gain stacks on the ordinary brackets. OTI 40,000 (10% top is 50,000); crypto ST gain 20,000 →
/// bottom 60,000 crosses 10%→22%. ord_with = 0.10·50,000 + 0.22·10,000 = 7,200; ord_without = 0.10·40,000
/// = 4,000 → delta 3,200.00. No LT, no NIIT (magi 60k < 200k).
#[test]
fn st_gain_stacks_on_ordinary() {
    let buy = ev(
        "BUY",
        datetime!(2025-02-01 00:00:00 UTC),
        EventPayload::Acquire(Acquire {
            sat: 100_000_000,
            usd_cost: dec!(10000.00),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    );
    let sell = ev(
        "SELL",
        datetime!(2025-06-01 00:00:00 UTC),
        EventPayload::Dispose(Dispose {
            sat: 100_000_000,
            usd_proceeds: dec!(30000.00),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    );
    let events = vec![buy, sell];
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    let out = compute_tax_year(
        &events,
        &st,
        2025,
        Some(&profile(dec!(40000), dec!(40000), dec!(0))),
        &synth(2025),
    );
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    assert_eq!(r.st_net, dec!(20000));
    assert_eq!(r.lt_net, dec!(0));
    assert_eq!(r.total_federal_tax_attributable, dec!(3200.00));
    assert_eq!(r.ltcg_tax, dec!(0.00));
    assert_eq!(r.niit, dec!(0.00));
    assert_eq!(r.marginal_rates.ordinary, dec!(0.22));
}

/// §1411 NIIT threshold crossing. OTI/magi 190,000; crypto LT gain 30,000 → magi_with 220,000 crosses the
/// $200,000 Single threshold by 20,000. LT all @15% (bottom 190k > 40k, top 220k < 400k) → ltcg_tax 4,500.
/// niit = 3.8% · min(NII 30,000, over 20,000) = 3.8% · 20,000 = 760.00 (niit_without = 0).
#[test]
fn niit_threshold_crossing() {
    let st = state_with(
        vec![disposal(date!(2025 - 09 - 01), dec!(30000), Term::LongTerm)],
        vec![],
    );
    let out = compute_tax_year(
        &[],
        &st,
        2025,
        Some(&profile(dec!(190000), dec!(190000), dec!(0))),
        &synth(2025),
    );
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    assert_eq!(r.lt_net, dec!(30000));
    assert_eq!(r.ltcg_tax, dec!(4500.00)); // 0.15 · 30,000
    assert_eq!(r.niit, dec!(760.00));
    assert!(r.marginal_rates.niit_applies);
    // identity: total == ordinary_delta(0) + ltcg_tax + niit
    assert_eq!(r.total_federal_tax_attributable, r.ltcg_tax + r.niit);
    assert_eq!(r.total_federal_tax_attributable, dec!(5260.00));
}

/// A full worked example exercising ordinary stacking across three brackets + net ST gain + net LT gain
/// crossing the §1(h) 15%→20% breakpoint + QD sharing the preferential stack + a NIIT delta with BOTH
/// scenarios over threshold (so `niit` is genuinely a delta, not a level).
///
/// Single. OTI 380,000; MAGI(excl crypto) 400,000 (= OTI + 20,000 QD); QD 20,000.
/// Crypto: mining 10,000 (ordinary); net ST gain 10,000; net LT gain 50,000.
///   bottom_with = 380,000 + 10,000 + 10,000 = 400,000 ; bottom_without = 380,000.
///   ord:  tax(400,000)=97,000 − tax(380,000)=90,600                                 → ordinary_delta 6,400
///   §1(h): pref_with (70,000 pref all above 400k @20% = 14,000)
///          − pref_without (20,000 QD in (380k,400k] @15% = 3,000)                   → ltcg_tax 11,000
///   §1411: with  3.8%·min(NII 80,000, over 270,000) = 3,040
///          without 3.8%·min(NII 20,000, over 200,000) = 760                         → niit 2,280
///   total = 6,400 + 11,000 + 2,280 = 19,680.00
#[test]
fn full_worked_example_ordinary_st_lt_qd_niit_delta() {
    let st = state_with(
        vec![
            disposal(date!(2025 - 03 - 01), dec!(10000), Term::ShortTerm),
            disposal(date!(2025 - 07 - 01), dec!(50000), Term::LongTerm),
        ],
        vec![income_rec(date!(2025 - 02 - 01), dec!(10000))],
    );
    let p = profile(dec!(380000), dec!(400000), dec!(20000));
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    assert_eq!(r.st_net, dec!(10000));
    assert_eq!(r.lt_net, dec!(50000));
    assert_eq!(r.ordinary_from_crypto, dec!(10000));
    assert_eq!(r.ltcg_tax, dec!(11000.00));
    assert_eq!(r.niit, dec!(2280.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(19680.00));
    // component identity: total == ordinary_delta + ltcg_tax + niit
    assert_eq!(
        r.total_federal_tax_attributable,
        dec!(6400.00) + r.ltcg_tax + r.niit
    );
    assert!(r.marginal_rates.niit_applies);
    assert_eq!(r.marginal_rates.ordinary, dec!(0.32));
    assert_eq!(r.marginal_rates.ltcg, dec!(0.20)); // top 470,000 > max_fifteen 400,000
}

/// [B-M1 HEADLINE] Loss-year §1411: a net capital loss reduces NII by ONLY the §1211(b)-allowed amount
/// (≤ $3,000), NOT by preserving other-category gains in NII (Form 8960 line 5a / §1.1411-4(d) Example 1).
///
/// Single, threshold $200k. Profile: OTI 270,000; QD 5,000; other_net_capital_gain +15,000 (LT);
/// MAGI(excl crypto) 290,000 (= 270,000 + 5,000 + 15,000). Crypto: net ST −80,000 (single ST disposal),
/// crypto_lt 0, crypto_ord 0, zero carryforward-in.
///
/// §1222 WITH:  st_net −80,000; lt_net +15,000; cross-net → residual ST loss −65,000 (65k loss > 15k gain).
///   ordinary_gain 0; preferential_gain 0; net_loss 65,000 → loss_deduction min(65k, 3k) = 3,000.
/// §1222 WITHOUT: st_net 0; lt_net +15,000 (both gains) → preferential_gain 15,000; loss_deduction 0.
///
/// NII (B-M1 fix — subtract loss_deduction):
///   nii_with    = 5,000 + 0 + 0 − 3,000  = 2,000
///   nii_without = 5,000 + 0 + 15,000 − 0 = 20,000
/// MAGI: crypto_agi = (0−3,000) − (0+15,000−0) + 0 = −18,000; magi_with = 290,000 − 18,000 = 272,000.
/// NIIT: niit_with    = 3.8% × min(2,000,  272,000−200,000=72,000) = 3.8% × 2,000  = 76.00
///       niit_without = 3.8% × min(20,000, 290,000−200,000=90,000) = 3.8% × 20,000 = 760.00
///       niit DELTA   = 76.00 − 760.00 = −684.00.  (PRE-FIX this delta was −570.00.)
///
/// total: ordinary_delta = tax(267,000)=54,440 − tax(270,000)=55,400 = −960.00 (= 3,000 × 32% saving);
///        ltcg_tax = pref(bottom 267k, pref 5k)=750.00 − pref(bottom 270k, pref 20k)=3,000.00 = −2,250.00;
///        niit = −684.00; total = −960 − 2,250 − 684 = −3,894.00.
#[test]
fn niit_loss_year_reduces_nii_by_1211_allowed_loss() {
    let st = state_with(
        vec![disposal(
            date!(2025 - 05 - 01),
            dec!(-80000),
            Term::ShortTerm,
        )],
        vec![],
    );
    let p = profile_with_ncg(dec!(270000), dec!(290000), dec!(5000), dec!(15000));
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    // Headline DELTA: NII reduced by exactly the $3,000 §1211-allowed loss (was −570.00 pre-fix).
    assert_eq!(r.niit, dec!(-684.00));
    assert_eq!(r.total_federal_tax_attributable, dec!(-3894.00));
    // WITH-scenario levels (sanity — not the NIIT figure under test):
    assert_eq!(r.st_net, dec!(-80000));
    assert_eq!(r.loss_deduction, dec!(3000));
}

/// [R0-M2] NIIT base floored at $0 (D2): a net capital loss driving NII negative must NEVER yield a
/// negative/refundable NIIT. Inputs put BOTH scenarios' NIIT at $0 (MAGI at the threshold → over==0),
/// so the observable DELTA `r.niit == 0.00` truly pins the `max(0, …)` floor.
///
/// Single, threshold $200k. QD 0; other_net_capital_gain 0; MAGI(excl crypto) 200,000 (exactly at the
/// threshold → over_without = 0). Crypto: net ST −80,000 → loss_deduction 3,000; crypto_lt 0.
///   nii_with = 0 + 0 + 0 − 3,000 = −3,000 (negative).
///   crypto_agi = (0−3,000) − 0 + 0 = −3,000; magi_with = 200,000 − 3,000 = 197,000 < 200,000 → over_with 0.
///   niit_with    = 3.8% × max(0, min(−3,000, 0)) = 3.8% × 0 = 0.00   (WITHOUT the D2 floor: −114.00)
///   niit_without = 3.8% × max(0, min(0, 0))      = 0.00.  DELTA = 0.00.
#[test]
fn niit_base_floored_at_zero_when_nii_negative() {
    let st = state_with(
        vec![disposal(
            date!(2025 - 05 - 01),
            dec!(-80000),
            Term::ShortTerm,
        )],
        vec![],
    );
    let p = profile(dec!(50000), dec!(200000), dec!(0));
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    // Floored at 0 — must NOT be −114.00 (which is what the pre-D2 `min(nii, over)` would yield).
    assert_eq!(r.niit, dec!(0.00));
}

/// [B-M1 MFS] Loss-year §1411 under the MFS §1211(b) limit of $1,500 (NOT $3,000): NII is reduced by only
/// $1,500. MFS threshold $125,000.
///
/// MFS. OTI 50,000; QD 5,000; other_net_capital_gain 0; MAGI(excl crypto) 300,000. Crypto: net ST −80,000
///   → net loss 80,000 → loss_deduction min(80,000, 1,500) = 1,500 (MFS cap).
///   nii_with    = 5,000 + 0 + 0 − 1,500 = 3,500;   nii_without = 5,000.
///   crypto_agi  = (0−1,500) − 0 + 0 = −1,500; magi_with = 300,000 − 1,500 = 298,500.
///   niit_with    = 3.8% × min(3,500, 298,500−125,000=173,500) = 3.8% × 3,500 = 133.00
///   niit_without = 3.8% × min(5,000, 300,000−125,000=175,000) = 3.8% × 5,000 = 190.00
///   niit DELTA   = 133.00 − 190.00 = −57.00  (= 3.8% × −$1,500; a $3,000 cap would give −114.00).
#[test]
fn niit_loss_year_mfs_1500_limit() {
    let st = state_with(
        vec![disposal(
            date!(2025 - 05 - 01),
            dec!(-80000),
            Term::ShortTerm,
        )],
        vec![],
    );
    let p = TaxProfile {
        filing_status: FilingStatus::Mfs,
        ordinary_taxable_income: dec!(50000),
        magi_excluding_crypto: dec!(300000),
        qualified_dividends_and_other_pref_income: dec!(5000),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth_mfs(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    assert_eq!(r.loss_deduction, dec!(1500)); // MFS §1211(b) cap
    assert_eq!(r.niit, dec!(-57.00)); // NII reduced by $1,500 (a $3,000 cap would give −114.00)
}

/// Only THIS tax-year's disposals/income are counted (out-of-year activity is ignored).
#[test]
fn only_in_year_disposals_and_income_are_counted() {
    let st = state_with(
        vec![
            disposal(date!(2024 - 12 - 31), dec!(99999), Term::ShortTerm), // wrong year → ignored
            disposal(date!(2025 - 06 - 01), dec!(20000), Term::ShortTerm),
            disposal(date!(2026 - 01 - 01), dec!(88888), Term::LongTerm), // wrong year → ignored
        ],
        vec![
            income_rec(date!(2024 - 06 - 01), dec!(7777)), // wrong year → ignored
            income_rec(date!(2025 - 06 - 01), dec!(0)),    // none this year
        ],
    );
    let out = compute_tax_year(
        &[],
        &st,
        2025,
        Some(&profile(dec!(40000), dec!(40000), dec!(0))),
        &synth(2025),
    );
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    assert_eq!(r.st_net, dec!(20000)); // only the 2025 ST disposal
    assert_eq!(r.ordinary_from_crypto, dec!(0)); // 2024 income excluded; 2025 income is 0
    assert_eq!(r.total_federal_tax_attributable, dec!(3200.00)); // matches the st_gain golden
}

/// §B.4 refusal: a disposal whose income FMV is missing → a real Hard `FmvMissing` blocker in the
/// projection → `TaxYearNotComputable` (the gate runs FIRST, before table/profile).
#[test]
fn refuses_year_with_hard_blocker() {
    let bad = ev(
        "BADINC",
        datetime!(2025-05-01 00:00:00 UTC),
        EventPayload::Income(Income {
            sat: 100_000_000,
            usd_fmv: None,
            fmv_status: FmvStatus::Missing,
            kind: IncomeKind::Mining,
            business: false,
        }),
    );
    let events = vec![bad];
    let st = project(
        &events,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        st.blockers
            .iter()
            .any(|b| b.kind.severity() == Severity::Hard),
        "fixture sanity: projection must carry a Hard blocker"
    );
    let out = compute_tax_year(
        &events,
        &st,
        2025,
        Some(&profile(dec!(50000), dec!(50000), dec!(0))),
        &synth(2025),
    );
    assert!(
        matches!(out, TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxYearNotComputable)
    );
}

/// B-I1: an UNRESOLVED out-of-year (2024) `ImportConflict` (Hard) gates 2025 even though 2025's disposal
/// looks otherwise computable — the projection-wide gate refuses rather than emit an authoritative-but-wrong
/// number off a possibly-contaminated basis. The refusal carries the offending EventId (B-N1) for C.
#[test]
fn refuses_year_with_out_of_year_import_conflict_on_consumed_lot() {
    let offender = EventId::import(Source::Coinbase, SourceRef::new("ACQ-2024"));
    let mut st = state_with(
        vec![disposal(date!(2025 - 09 - 01), dec!(50000), Term::LongTerm)],
        vec![],
    );
    st.blockers.push(Blocker {
        kind: BlockerKind::ImportConflict,
        event: Some(offender.clone()),
        detail: "unresolved import conflict (2024)".into(),
    });
    let out = compute_tax_year(
        &[],
        &st,
        2025,
        Some(&profile(dec!(50000), dec!(50000), dec!(0))),
        &synth(2025),
    );
    match out {
        TaxOutcome::NotComputable(b) => {
            assert_eq!(b.kind, BlockerKind::TaxYearNotComputable);
            assert_eq!(b.event, Some(offender)); // B-N1: structured offending EventId carried
        }
        TaxOutcome::Computed(_) => panic!("must refuse on an out-of-year Hard blocker"),
    }
}

/// Missing table → `TaxTableMissing`; missing profile → `TaxProfileMissing` (clean projection).
#[test]
fn missing_table_then_profile_blockers() {
    let st = state_with(vec![], vec![]);
    assert!(matches!(
        compute_tax_year(&[], &st, 2099, Some(&profile(dec!(1), dec!(1), dec!(0))), &synth(2025)),
        TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxTableMissing
    ));
    assert!(matches!(
        compute_tax_year(&[], &st, 2025, None, &synth(2025)),
        TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxProfileMissing
    ));
}

/// Refusal PRECEDENCE: a Hard blocker wins even when the table is also missing AND the profile is absent.
#[test]
fn hard_blocker_precedes_missing_table_and_profile() {
    let mut st = state_with(vec![], vec![]);
    st.blockers.push(Blocker {
        kind: BlockerKind::FmvMissing,
        event: None,
        detail: "x".into(),
    });
    let out = compute_tax_year(&[], &st, 2099, None, &synth(2025));
    assert!(
        matches!(out, TaxOutcome::NotComputable(b) if b.kind == BlockerKind::TaxYearNotComputable)
    );
}

/// B-Nit (advisory-only → Computed KAT): a projection with ONLY Advisory-severity blockers
/// — and NO Hard blocker — must still yield `TaxOutcome::Computed(..)`. The refusal gate
/// (`first_hard_blocker`) matches ONLY `severity()==Hard`; Advisory blockers never gate B.
///
/// Fixture: one `Pre2025MethodNote` (Advisory) + one `SafeHarborTimebar` (Advisory),
/// one 2025 LT disposal. The gate must NOT fire; B must return a `Computed` result.
#[test]
fn advisory_only_blockers_do_not_gate_computation() {
    let mut st = state_with(
        vec![disposal(date!(2025 - 06 - 01), dec!(10000), Term::LongTerm)],
        vec![],
    );
    st.blockers.push(Blocker {
        kind: BlockerKind::Pre2025MethodNote,
        event: None,
        detail: "pre-2025 FIFO advisory".into(),
    });
    st.blockers.push(Blocker {
        kind: BlockerKind::SafeHarborTimebar,
        event: None,
        detail: "safe-harbor timebar advisory".into(),
    });
    // Sanity: both blockers are Advisory, none are Hard.
    assert!(
        st.blockers
            .iter()
            .all(|b| b.kind.severity() == Severity::Advisory),
        "fixture sanity: all blockers must be Advisory"
    );
    let out = compute_tax_year(
        &[],
        &st,
        2025,
        Some(&profile(dec!(40000), dec!(40000), dec!(0))),
        &synth(2025),
    );
    // Advisory blockers MUST NOT gate — the result must be Computed.
    assert!(
        matches!(out, TaxOutcome::Computed(_)),
        "expected TaxOutcome::Computed — advisory blockers must not gate computation; got: {out:?}"
    );
}

/// [R0-M3] P2-B ↔ engine-B reconciliation via INDEPENDENT code paths (NOT a tautology).
///
/// On an ALL-GAINS fixture (ST gain 20,000 + LT gain 50,000, both 2025; no losses) with a profile
/// carrying ZERO `capital_loss_carryforward_in` and ZERO `other_net_capital_gain`, §1222 does no
/// cross-netting, so B's within-character net == the raw part gain. `schedule_d` (P2-B) and
/// `compute_tax_year` (engine B) are SEPARATE functions that each aggregate the same
/// `state.disposals`, so `schedule_d(..).st.gain == TaxResult.st_net` (and LT) is a genuine
/// cross-check — the forms and the tax engine cannot silently diverge. (`profile()` already sets
/// other_net_capital_gain=0 and carryforward=0; do NOT reconcile against a shared helper.)
#[test]
fn schedule_d_reconciles_with_engine_b_on_all_gains_fixture() {
    let st = state_with(
        vec![
            disposal(date!(2025 - 03 - 01), dec!(20000), Term::ShortTerm),
            disposal(date!(2025 - 07 - 01), dec!(50000), Term::LongTerm),
        ],
        vec![],
    );
    let p = profile(dec!(100000), dec!(100000), dec!(0));
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    let sd = btctax_core::forms::schedule_d(&st, 2025);
    // Independent aggregators agree on the within-character gains (net == raw, all gains, cf=0, other=0).
    assert_eq!(
        sd.st.gain, r.st_net,
        "ST Σgain must reconcile with B.st_net"
    );
    assert_eq!(
        sd.lt.gain, r.lt_net,
        "LT Σgain must reconcile with B.lt_net"
    );
    // Sanity: the raw part gains are exactly the all-gains inputs.
    assert_eq!(sd.st.gain, dec!(20000));
    assert_eq!(sd.lt.gain, dec!(50000));
}

/// NFR4 determinism: identical inputs → identical outcome.
#[test]
fn determinism_same_inputs_same_outcome() {
    let st = state_with(
        vec![disposal(date!(2025 - 09 - 01), dec!(30000), Term::LongTerm)],
        vec![income_rec(date!(2025 - 02 - 01), dec!(5000))],
    );
    let p = profile(dec!(190000), dec!(190000), dec!(0));
    let a = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let b = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    assert_eq!(a, b);
}

/// [Task 1 — NII interest slice HEADLINE] §1411(c)(1)(A)(i): crypto-lending interest IS NII.
///
/// Single, synth table (NIIT threshold $200k). OTI $150,000; MAGI(excl crypto) $195,000; QD $0;
/// other_net_capital_gain $0; zero carryforward; NO disposals; one Interest income $20,000 in-year.
///
/// D1 implementation path:
///   interest_nii = 20,000; nii_with = 0+0+0-0+20,000 = 20,000; nii_without = 0.
///   crypto_ord = 20,000; crypto_agi = 0-0+20,000 = 20,000; magi_with = 215,000 → over 15,000.
///   niit_with  = 3.8% × min(20,000, 15,000) = 3.8% × 15,000 = 570.00  (min-cap: nii > over).
///   niit_without = 0 (magi_without 195,000 < 200,000 threshold). niit DELTA = 570.00.
///
/// ord_delta = tax(170,000) − tax(150,000) on synth (both in 22% band above 50k bracket):
///   22% × (170,000−150,000) = 22% × 20,000 = 4,400.00.
/// total = 4,400 + 0 (no LTCG) + 570 = 4,970.00  [R0-N2: pinned absolutely, not just identity].
///
/// Pre-fix (no D1): interest_nii == 0 → niit == 0.00. Golden MUST FAIL red before D1 lands.
#[test]
fn interest_nii_headline_interest_plus_min_cap() {
    let st = state_with(
        vec![],
        vec![income_rec_interest(date!(2025 - 06 - 01), dec!(20000))],
    );
    let p = profile(dec!(150000), dec!(195000), dec!(0));
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    // NIIT: 3.8% × min(NII 20,000, over 15,000) = 570.00 (min-cap exercised: nii > over).
    assert_eq!(r.niit, dec!(570.00));
    // [R0-N2] Absolute total: ord_delta 4,400 + ltcg 0 + niit 570 = 4,970.00.
    assert_eq!(r.total_federal_tax_attributable, dec!(4970.00));
    // Identity: total == ord_delta + ltcg_tax + niit.
    assert_eq!(
        r.total_federal_tax_attributable,
        dec!(4400.00) + r.ltcg_tax + r.niit
    );
    assert!(r.marginal_rates.niit_applies);
}

/// [Task 1 — NII interest slice EXCLUSION-BOUNDARY LOCK] Mixed Mining+Interest: only Interest
/// enters NII; Mining stays excluded (SE income per §1411(c)(6) or non-NII other income).
///
/// Single, synth table (NIIT threshold $200k). OTI any; MAGI(excl crypto) $200,000 (exactly at
/// the threshold → magi_without NOT > threshold → niit_without == 0); NO disposals.
/// Mining $30,000 + Interest $10,000 both in-year.
///
/// D1 implementation path:
///   crypto_ord = 40,000; interest_nii = 10,000 (Interest ONLY); nii_with = 10,000.
///   crypto_agi = 0-0+40,000 = 40,000; magi_with = 240,000 → over 40,000.
///   niit_with  = 3.8% × min(10,000, 40,000) = 3.8% × 10,000 = 380.00 (NII is the cap).
///   niit_without = 0 (magi_without = 200,000, NOT > 200,000 → over == 0). niit DELTA = 380.00.
///
/// Wrong-inclusion guard: if Mining wrongly entered NII → nii_with 40,000 → niit_with
///   3.8% × min(40,000, 40,000) = 1,520.00 — the golden fails that wrong path.
///
/// Pre-fix (no D1): interest_nii == 0 → niit == 0.00. Golden MUST FAIL red before D1 lands.
#[test]
fn interest_nii_mixed_mining_plus_interest_exclusion_boundary() {
    let st = state_with(
        vec![],
        vec![
            income_rec(date!(2025 - 03 - 01), dec!(30000)), // Mining → NOT NII
            income_rec_interest(date!(2025 - 07 - 01), dec!(10000)), // Interest → IS NII
        ],
    );
    // magi_excluding_crypto == 200,000: magi_without NOT > threshold → niit_without == 0.
    let p = profile(dec!(50000), dec!(200000), dec!(0));
    let out = compute_tax_year(&[], &st, 2025, Some(&p), &synth(2025));
    let TaxOutcome::Computed(r) = out else {
        panic!("computable")
    };
    // Interest IS NII; Mining is NOT: 3.8% × min(10,000, 40,000) = 380.00.
    // Wrong-inclusion of Mining would give 3.8% × min(40,000, 40,000) = 1,520.00.
    assert_eq!(r.niit, dec!(380.00));
    assert!(r.marginal_rates.niit_applies);
}
