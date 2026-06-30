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
    }
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
