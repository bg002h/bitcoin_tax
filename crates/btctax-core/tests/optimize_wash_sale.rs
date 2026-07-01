//! Sub-project C, Task 7 — §1091 wash-sale exemption KAT (C.5).
//!
//! §1091 disallows a loss only on "stock or securities". Convertible virtual currency is
//! **property**, not a security (IRS Notice 2014-21; Rev. Rul. 2023-14). No statute extending
//! §1091 to crypto has been enacted. The optimizer therefore selects loss lots **freely** — there
//! is no 30-day disallowance window to observe for crypto disposals. This KAT pins that legal
//! intent as a regression: the optimizer must freely pick a loss lot even when it was acquired
//! within the §1091 30-day window, and the loss must be applied in full (not disallowed or
//! deferred as §1091 would require for a "stock or security").
//!
//! MONITOR: if §1091 is ever extended to digital assets, the optimizer must add a 30-day
//! disallowance guard and this KAT must be updated to test that guard instead (FOLLOWUPS.md C.5).

use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::optimize::{optimize_year, score_assignment};
use btctax_core::price::StaticPrices;
use btctax_core::project::ProjectionConfig;
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::{datetime, offset};

const LOT: i64 = 100_000_000; // one whole BTC in satoshis

// ── synthetic tax table + profile ────────────────────────────────────────────────────────────────

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
                    lower: dec!(90000),
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
    })
}

fn profile(ordinary: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: ordinary,
        magi_excluding_crypto: ordinary,
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
    }
}

// ── event / id helpers ────────────────────────────────────────────────────────────────────────────

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
fn sell_event(
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
fn no_attest() -> BTreeSet<EventId> {
    BTreeSet::new()
}
fn btreemap_assign(entries: &[(EventId, Vec<LotPick>)]) -> BTreeMap<EventId, Vec<LotPick>> {
    entries.iter().cloned().collect()
}

// ── §1091 wash-sale exemption KAT ────────────────────────────────────────────────────────────────

/// §1091 wash-sale exemption: loss lots are freely selectable for crypto.
///
/// Fixture:
///   - "GAIN" lot: acquired 2023-07-01, cost $2,000 → long-term as of the 2025-07-01 sale date;
///     low basis → realizes a $28,000 LT gain if selected.
///   - "LOSS" lot: acquired **2025-06-15** — only **16 days** before the 2025-07-01 sale.
///     Cost $50,000 → realizes a $20,000 ST loss if selected. For a *stock or security*, buying
///     within the 30-day window before a loss sale would trigger §1091 wash-sale disallowance.
///     Crypto is **property**, not a security (Notice 2014-21) — the 30-day bar does not apply.
///   - "D" disposal: sell 1 BTC on 2025-07-01 at $30,000.
///
/// FIFO baseline selects "GAIN" (acquired first) → large LT gain, higher tax.
/// Tax-minimizing optimum selects "LOSS" (within the §1091 window!) → loss reduces tax.
///
/// Assertions:
///   1. `proposed_selection == [pick("LOSS")]` — optimizer freely picked the 30-day-window lot.
///   2. `optimized_tax < baseline_tax` — the loss reduced total federal tax attributable.
///   3. Independent `score_assignment` of the loss selection equals `optimized_tax` — the loss is
///      applied in full, not disallowed/deferred as §1091 would require for a security.
///   4. NFR4 determinism — two calls produce identical proposals.
#[test]
fn loss_lot_freely_selectable_no_wash_sale_bar() {
    let events = vec![
        // "GAIN" lot: long-term, low basis — FIFO baseline default (acquired first).
        buy(
            "GAIN",
            datetime!(2023-07-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(2000),
        ),
        // "LOSS" lot: acquired 2025-06-15, only 16 days before the 2025-07-01 disposal.
        // §1091 would disallow the resulting loss if this were a "stock or security" (30-day window).
        // Crypto is property per Notice 2014-21 → optimizer selects this lot freely, no bar.
        buy(
            "LOSS",
            datetime!(2025-06-15 00:00:00 UTC),
            cold(),
            LOT,
            dec!(50000),
        ),
        sell_event(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(30000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile(dec!(80000));
    let made = time::macros::date!(2026 - 01 - 15);

    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2025,
        Some(&prof),
        &tables,
        &no_attest(),
        made,
    )
    .expect("optimize_year must be computable for this synthetic fixture");

    // 1. Optimizer picked the loss lot — no 30-day bar, freely selectable for crypto.
    assert_eq!(
        p.per_disposal[0].proposed_selection,
        vec![pick("LOSS", LOT)],
        "optimizer must freely select the loss lot despite 16-day acquisition proximity \
         (§1091 does not apply to crypto; Notice 2014-21)"
    );
    // The FIFO baseline used the gain lot (acquired first: 2023-07-01 < 2025-06-15).
    assert_eq!(
        p.per_disposal[0].current_selection,
        vec![pick("GAIN", LOT)],
        "FIFO baseline must have selected the earlier (gain) lot"
    );

    // 2. The loss strictly reduced total federal tax attributable.
    assert!(
        p.optimized_tax < p.baseline_tax,
        "loss harvesting must reduce total_federal_tax_attributable: {} < {}",
        p.optimized_tax,
        p.baseline_tax,
    );
    assert!(
        p.delta < dec!(0),
        "delta must be negative (optimizer improved on baseline)"
    );

    // 3. Independent score confirms the loss was applied IN FULL — not washed/disallowed.
    //    If §1091 were enforced, the optimizer's score would equal the baseline (loss zeroed out).
    //    Since crypto is property, the loss goes through and reduces the total tax.
    let loss_assignment = btreemap_assign(&[(eid("D"), vec![pick("LOSS", LOT)])]);
    let TaxOutcome::Computed(loss_score) = score_assignment(
        &events,
        &prices,
        &cfg(),
        2025,
        Some(&prof),
        &tables,
        &loss_assignment,
    ) else {
        panic!("score_assignment with the loss-lot selection must be computable");
    };
    assert_eq!(
        p.optimized_tax, loss_score.total_federal_tax_attributable,
        "optimizer result must equal independent loss-lot score (loss applied in full, not washed)"
    );
    assert!(
        loss_score.total_federal_tax_attributable < p.baseline_tax,
        "§1091-exempt loss reduces tax: {} < baseline {} \
         (if §1091 applied, the loss would be disallowed and tax would equal or exceed baseline)",
        loss_score.total_federal_tax_attributable,
        p.baseline_tax,
    );

    // 4. NFR4 determinism — two consecutive calls produce byte-identical proposals.
    let p2 = optimize_year(
        &events,
        &prices,
        &cfg(),
        2025,
        Some(&prof),
        &tables,
        &no_attest(),
        made,
    )
    .expect("second call must also be computable");
    assert_eq!(p, p2, "NFR4: two calls must produce identical proposals");
}
