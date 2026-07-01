//! Sub-project C, Task 2 — `score_assignment` KATs (holistic year scorer).
//!
//! `score_assignment` injects a per-disposal `LotPick` selection set, folds the canonical timeline
//! ONCE with those selections overriding the persisted/default identification, and runs B's
//! `compute_tax_year` for the year — WITHOUT mutating the ledger (clone-fold-discard). It is the
//! single primitive every optimizer mode builds on.
//!
//! Fixture (synthetic, privacy-safe — no real reads): a 2025 self-custody wallet with two equal-sat
//! lots of DIFFERENT basis (low-basis "A" acquired first, high-basis "B" acquired second) and one
//! 2025 `Sell` of exactly one lot's worth of sats. Under the FIFO default the sell consumes all of
//! low-basis A (max gain); a selection that draws from high-basis B lowers the taxable gain. Tax is
//! scored against the synthetic table/profile (mirrors `tax_compute.rs`); exact Decimal, no float.
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::optimize::score_assignment;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::tax::compute::compute_tax_year;
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{datetime, offset};

const LOT_SAT: i64 = 100_000_000; // one whole BTC per lot
const HALF: i64 = 50_000_000;

// ── synthetic table + profile (same Single schedule as tax_compute.rs) ───────────────────────────
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
    })
}
fn profile() -> TaxProfile {
    // ordinary/MAGI = 0 so the ST gain stacks cleanly on the 10% band and stays well below the
    // $200k NIIT threshold — the whole tax is the ordinary delta on the crypto ST gain.
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

// ── synthetic ledger (self-custody wallet, all post-2025) ────────────────────────────────────────
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn ev(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Swan, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(cold()),
        payload: p,
    }
}
fn buy(rf: &str, ts: time::OffsetDateTime, sat: i64, cost: Usd) -> LedgerEvent {
    ev(
        rf,
        ts,
        EventPayload::Acquire(Acquire {
            sat,
            usd_cost: cost,
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    )
}
fn sell(rf: &str, ts: time::OffsetDateTime, sat: i64, proceeds: Usd) -> LedgerEvent {
    ev(
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
fn lot(rf: &str) -> LotId {
    LotId {
        origin_event_id: EventId::import(Source::Swan, SourceRef::new(rf)),
        split_sequence: 0,
    }
}
fn pick(l: LotId, sat: i64) -> LotPick {
    LotPick { lot: l, sat }
}
fn sell_id() -> EventId {
    EventId::import(Source::Swan, SourceRef::new("SELL"))
}

/// Two equal-sat lots of different basis + one 2025 sell of one lot's worth.
/// FIFO consumes low-basis "A" (gain 40,000); high-basis "B" is held in reserve.
fn fixture() -> Vec<LedgerEvent> {
    vec![
        buy(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            LOT_SAT,
            dec!(10000.00),
        ),
        buy(
            "B",
            datetime!(2025-03-01 00:00:00 UTC),
            LOT_SAT,
            dec!(30000.00),
        ),
        sell(
            "SELL",
            datetime!(2025-06-01 00:00:00 UTC),
            LOT_SAT,
            dec!(50000.00),
        ),
    ]
}

fn cfg() -> ProjectionConfig {
    ProjectionConfig::default() // FIFO default, TreatmentC
}

// ── KATs ─────────────────────────────────────────────────────────────────────────────────────────

/// (a) An EMPTY assignment scores identically to a plain projection's `compute_tax_year`: injecting
/// nothing must be a no-op over the canonical timeline.
#[test]
fn empty_assignment_equals_plain_projection() {
    let events = fixture();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let plain = compute_tax_year(
        &events,
        &project(&events, &prices, &cfg()),
        2025,
        Some(&prof),
        &tables,
    );
    let empty: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
    let scored = score_assignment(&events, &prices, &cfg(), 2025, Some(&prof), &tables, &empty);
    assert_eq!(scored, plain);
}

/// (b) Drawing from the HIGH-basis lot lowers the taxable gain vs the FIFO baseline → strictly less
/// federal tax. Exact goldens: FIFO consumes all of A (gain 40,000 → 4,000.00 @10%); the half-B/half-A
/// pick has basis 20,000 (gain 30,000 → 3,000.00). Both ST, both below the NIIT threshold.
#[test]
fn high_basis_pick_lowers_tax_below_fifo_baseline() {
    let events = fixture();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let plain = compute_tax_year(
        &events,
        &project(&events, &prices, &cfg()),
        2025,
        Some(&prof),
        &tables,
    );
    let mut pick_high: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
    pick_high.insert(sell_id(), vec![pick(lot("B"), HALF), pick(lot("A"), HALF)]);
    let scored = score_assignment(
        &events,
        &prices,
        &cfg(),
        2025,
        Some(&prof),
        &tables,
        &pick_high,
    );

    let (TaxOutcome::Computed(hi), TaxOutcome::Computed(base)) = (scored, plain) else {
        panic!("both feasible assignments must be computable");
    };
    assert_eq!(base.total_federal_tax_attributable, dec!(4000.00)); // FIFO: gain 40,000 @10%
    assert_eq!(hi.total_federal_tax_attributable, dec!(3000.00)); //  half-B: gain 30,000 @10%
    assert!(hi.total_federal_tax_attributable < base.total_federal_tax_attributable);
}

/// An INFEASIBLE (but principal-conserving) assignment self-eliminates: picking a non-existent lot
/// for the full principal conserves Σ but folds to a hard `LotSelectionInvalid`, so `compute_tax_year`
/// returns `NotComputable` (the LotSelectionInvalid→Hard→NotComputable path the optimizer relies on).
#[test]
fn infeasible_assignment_scores_not_computable() {
    let events = fixture();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let mut bad: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
    bad.insert(sell_id(), vec![pick(lot("NOPE"), LOT_SAT)]); // conserves Σ, but lot doesn't exist
    let scored = score_assignment(&events, &prices, &cfg(), 2025, Some(&prof), &tables, &bad);
    assert!(matches!(scored, TaxOutcome::NotComputable(_)));
}

/// NFR4: identical inputs → identical score (byte-identical `TaxOutcome`).
#[test]
fn scoring_is_deterministic() {
    let events = fixture();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let mut a: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
    a.insert(sell_id(), vec![pick(lot("B"), HALF), pick(lot("A"), HALF)]);
    let s1 = score_assignment(&events, &prices, &cfg(), 2025, Some(&prof), &tables, &a);
    let s2 = score_assignment(&events, &prices, &cfg(), 2025, Some(&prof), &tables, &a);
    assert_eq!(s1, s2);
}

/// Side-effect-free: scoring an assignment does NOT mutate the events nor perturb the canonical
/// projection — the baseline before and after a non-empty scoring call are identical.
#[test]
fn scoring_does_not_mutate_ledger() {
    let events = fixture();
    let events_before = events.clone();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let baseline_before = compute_tax_year(
        &events,
        &project(&events, &prices, &cfg()),
        2025,
        Some(&prof),
        &tables,
    );

    let mut a: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
    a.insert(sell_id(), vec![pick(lot("B"), HALF), pick(lot("A"), HALF)]);
    let _ = score_assignment(&events, &prices, &cfg(), 2025, Some(&prof), &tables, &a);

    // events slice is byte-identical, and the canonical projection is unchanged.
    assert_eq!(events, events_before);
    let baseline_after = compute_tax_year(
        &events,
        &project(&events, &prices, &cfg()),
        2025,
        Some(&prof),
        &tables,
    );
    assert_eq!(baseline_before, baseline_after);
}

/// R0-M1 precondition: a NON-conserving assignment (Σpicks ≠ principal) trips the debug-only
/// `debug_assert!` rather than silently under-consuming to a falsely-low score. Debug builds only
/// (the assert is compiled out in release).
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "R0-M1")]
fn non_conserving_assignment_trips_debug_assert() {
    let events = fixture();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile();

    let mut bad: BTreeMap<EventId, Vec<LotPick>> = BTreeMap::new();
    bad.insert(sell_id(), vec![pick(lot("A"), HALF)]); // Σ = HALF != LOT_SAT principal
    let _ = score_assignment(&events, &prices, &cfg(), 2025, Some(&prof), &tables, &bad);
}
