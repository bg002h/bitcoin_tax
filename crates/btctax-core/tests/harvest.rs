//! btctax-core::whatif::harvest — phase P2 KATs (task #43): the harvest optimizer.
//!
//! `whatif::harvest` finds the MAX N (sats) sellable from a wallet's as-of pool such that a target
//! (`zero-ltcg` / `fifteen-ltcg` / `gain=$X` / `tax=$X`) holds on the ENTIRE PREFIX [0, N], computed
//! ONLY through `compute_tax_year` via the STANDING-method consumption schedule (the architect's
//! lot-edge segment walk — NOT global bisection, which is UNSOUND: marginal-tax(N) is non-monotone).
//!
//! The ★ non-monotone traps come FIRST (fable report §6). Goldens are hand-derived from the `synth`
//! table (Single: ord 0→10%, 50k→22%, 250k→32%; §1(h) max_zero=40k, max_fifteen=400k; Mfj max_zero=80k;
//! NIIT 3.8% / $200k Single). Answers are within the documented τ = 1,024-sat tolerance (< $0.05 tax);
//! bands (±) absorb the per-leg cent-rounding wiggle. All fixtures synthetic; exact Decimal, no float.
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxProfile};
use btctax_core::whatif::{harvest, HarvestRequest, HarvestStatus, HarvestTarget, WhatIfError};
use btctax_core::LotMethod;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

const LOT: i64 = 100_000_000; // one whole BTC per lot
const TAU: i64 = 1_024;

// ── synthetic table (Single + Mfs + Mfj so MFS/Qss lookups resolve) ────────────────────────────────
struct OneTable(TaxTable);
impl TaxTables for OneTable {
    fn table_for(&self, year: i32) -> Option<&TaxTable> {
        (year == self.0.year).then_some(&self.0)
    }
}
fn sched() -> OrdinarySchedule {
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
    }
}
fn synth(year: i32) -> OneTable {
    let mut ordinary = BTreeMap::new();
    let mut ltcg = BTreeMap::new();
    for fs in [FilingStatus::Single, FilingStatus::Mfs] {
        ordinary.insert(fs, sched());
        ltcg.insert(
            fs,
            LtcgBreakpoints {
                max_zero: dec!(40000),
                max_fifteen: dec!(400000),
            },
        );
    }
    // Mfj (used by Qss → Mfj mapping) with a DISTINCT max_zero so the mapping is observable.
    ordinary.insert(FilingStatus::Mfj, sched());
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(80000),
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
#[allow(clippy::too_many_arguments)]
fn profile_of(fs: FilingStatus, ord: Usd, magi: Usd, qd: Usd, cf_long: Usd) -> TaxProfile {
    TaxProfile {
        filing_status: fs,
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
fn single(ord: Usd, magi: Usd) -> TaxProfile {
    profile_of(FilingStatus::Single, ord, magi, dec!(0), dec!(0))
}

// ── event / id builders ────────────────────────────────────────────────────────────────────────────
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
/// A GLOBAL forward FIFO election (effective 2025-01-01, made same day ⇒ never back-dated).
fn fifo_election(seq: u64) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: datetime!(2025-01-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None, // scope lives in the payload; None ⇒ global
        payload: EventPayload::MethodElection(MethodElection {
            effective_from: date!(2025 - 01 - 01),
            method: LotMethod::Fifo,
            wallet: None,
        }),
    }
}
/// A dual-basis received-gift lot (TransferIn + ClassifyInbound::GiftReceived).
#[allow(clippy::too_many_arguments)]
fn gift_lot(
    rf: &str,
    seq: u64,
    w: WalletId,
    sat: i64,
    donor_basis: Option<Usd>,
    donor_acq: time::Date,
    fmv_at_gift: Usd,
    recv: time::OffsetDateTime,
) -> Vec<LedgerEvent> {
    let in_ev = LedgerEvent {
        id: eid(rf),
        utc_timestamp: recv,
        original_tz: offset!(+00:00),
        wallet: Some(w),
        payload: EventPayload::TransferIn(TransferIn {
            sat,
            src_addr: None,
            txid: None,
        }),
    };
    let cls = LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: datetime!(2026-12-31 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: eid(rf),
            as_: InboundClass::GiftReceived {
                donor_basis,
                donor_acquired_at: Some(donor_acq),
                fmv_at_gift,
            },
        }),
    };
    vec![in_ev, cls]
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default()
}
fn hreq(at: time::Date, price: Usd, target: HarvestTarget) -> HarvestRequest {
    HarvestRequest {
        wallet: cold(),
        at,
        price: Some(price),
        target,
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// ★ trap KATs (the non-monotone shapes) — fable report §6.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// ★ DIP. Pool = [high-basis loss lot A $80k, low-basis gain lot B $10k], price $50k between. HIFO
/// consumes the LOSS lot first ⇒ marginal(N) goes NEGATIVE (−660 at 1 BTC) then RISES (+1,500 at 2 BTC).
/// A naive global bisection over `tax ≤ 0` lands wrong; the segment walk pins the answer at ~1.75 BTC —
/// where the second lot's gain has just cancelled the first lot's loss (net crypto gain 0 ⇒ marginal 0).
#[test]
fn harvest_dip() {
    let events = vec![
        buy(
            "A",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(80000),
        ),
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));

    // Document the DIP directly (standing HIFO): loss lot first ⇒ marginal −660, then +1,500.
    let sell1 = btctax_core::whatif::sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &btctax_core::whatif::SellRequest {
            sell_sat: LOT,
            wallet: cold(),
            at: date!(2025 - 08 - 01),
            price: Some(dec!(50000)),
            method: None,
        },
    )
    .unwrap();
    assert_eq!(
        sell1.marginal_tax,
        dec!(-660.00),
        "loss lot first ⇒ the dip"
    );
    let sell2 = btctax_core::whatif::sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &btctax_core::whatif::SellRequest {
            sell_sat: 2 * LOT,
            wallet: cold(),
            at: date!(2025 - 08 - 01),
            price: Some(dec!(50000)),
            method: None,
        },
    )
    .unwrap();
    assert_eq!(
        sell2.marginal_tax,
        dec!(1500.00),
        "second (gain) lot pulls marginal back up"
    );

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Tax(dec!(0)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        r.marginal_tax <= dec!(0),
        "engine-verified feasible: marginal ≤ 0 at N*"
    );
    assert!(
        r.n_sat > LOT,
        "the answer extends PAST the pure-loss lot (the dip trap)"
    );
    assert!(r.n_sat < 2 * LOT, "…but not the whole pool");
    assert!(
        (r.n_sat - 175_000_000).abs() <= 4 * TAU,
        "hand truth: ~1.75 BTC (net crypto gain crosses 0), got {}",
        r.n_sat
    );
}

/// ★ FIFO NON-CONTIGUOUS. Under a FIFO election the schedule is gain(+10k) → gain(+20k) → loss(−35k),
/// so `tax ≤ $3,000` is TRUE (1 BTC) → FALSE (2 BTC) → TRUE (3 BTC, a net loss). PREFIX semantics return
/// the FIRST boundary (~1.5 BTC), NOT the later feasible island at 3 BTC.
#[test]
fn harvest_fifo_non_contiguous() {
    let events = vec![
        fifo_election(0),
        buy(
            "L1",
            datetime!(2024-01-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(40000),
        ), // +10k
        buy(
            "L2",
            datetime!(2024-02-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(30000),
        ), // +20k
        buy(
            "L3",
            datetime!(2024-03-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(85000),
        ), // −35k
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(100000), dec!(100000));

    // The later feasible ISLAND exists: selling all 3 BTC is a net loss ⇒ marginal −660 ≤ $3,000.
    let sell_all = btctax_core::whatif::sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &btctax_core::whatif::SellRequest {
            sell_sat: 3 * LOT,
            wallet: cold(),
            at: date!(2025 - 08 - 01),
            price: Some(dec!(50000)),
            method: None,
        },
    )
    .unwrap();
    assert_eq!(
        sell_all.marginal_tax,
        dec!(-660.00),
        "the 3-BTC island is feasible"
    );

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Tax(dec!(3000)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(r.marginal_tax <= dec!(3000));
    assert!(
        r.n_sat > LOT && r.n_sat < 2 * LOT,
        "prefix answer is the FIRST boundary, not the island"
    );
    assert!(
        (r.n_sat - 150_000_000).abs() <= 4 * TAU,
        "hand truth: ~1.5 BTC (cumulative gain 20k ⇒ pref $3,000), got {}",
        r.n_sat
    );
}

/// ★ $3k PIN (flat-at-a-loss) + all-loss NotBinding. An all-loss pool: every N keeps `tax ≤ $0`, so the
/// target NEVER binds ⇒ NotBinding, answer = the whole pool. marginal is FLAT at the §1211(b) pin (−660
/// at both 1 BTC and 2 BTC); only $3,000 is deductible this year; the rest is carried (the disclosure).
#[test]
fn harvest_3k_pin_flat_notbinding() {
    let events = vec![
        buy(
            "A",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(50000),
        ), // −40k @ $10k
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(60000),
        ), // −50k @ $10k
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(100000), dec!(100000));

    // The pin: marginal is FLAT across the loss segment (−660 at 1 BTC AND at 2 BTC).
    for n in [LOT, 2 * LOT] {
        let s = btctax_core::whatif::sell(
            &events,
            &prices,
            &cfg(),
            Some(&prof),
            &tables,
            &btctax_core::whatif::SellRequest {
                sell_sat: n,
                wallet: cold(),
                at: date!(2025 - 08 - 01),
                price: Some(dec!(10000)),
                method: None,
            },
        )
        .unwrap();
        assert_eq!(
            s.marginal_tax,
            dec!(-660.00),
            "§1211(b) $3k pin ⇒ marginal flat"
        );
    }

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(10000),
            HarvestTarget::Tax(dec!(0)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::NotBinding);
    assert_eq!(
        r.n_sat,
        2 * LOT,
        "the full all-loss pool fits under tax ≤ $0"
    );
    assert_eq!(r.marginal_tax, dec!(-660.00));
    // §1212(b): $90,000 loss − $3,000 used = $87,000 carried (LT character).
    assert_eq!(r.carryforward_delta.long, dec!(87000));
    let note = r.plateau_note.expect("all-loss ⇒ §1211(b)/$3k disclosure");
    assert!(
        note.contains("\u{00a7}1211(b)") && note.contains("carried"),
        "note: {note}"
    );
}

/// ★ CARRYFORWARD BURN. Profile cf_long = $50k, an all-gain pool, `tax=$0`. Gains ABSORB the carried loss
/// for $0 current-year tax (marginal flat $0 across the absorption), then the boundary bites where the
/// §1211(b) offset starts shrinking (g = 47k, the ordinary-rate slope). The report shows the BURN:
/// carryforward_out(0) − carryforward_out(N*) == the gain absorbed.
#[test]
fn harvest_carryforward_burn_disclosed() {
    let events = vec![
        buy(
            "A",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ), // +40k
        buy(
            "B",
            datetime!(2024-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ), // +40k
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_of(
        FilingStatus::Single,
        dec!(100000),
        dec!(100000),
        dec!(0),
        dec!(50000),
    );

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Tax(dec!(0)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        r.marginal_tax <= dec!(0),
        "gains absorbed for $0 current-year tax"
    );
    assert_eq!(
        r.marginal_tax,
        dec!(0.00),
        "flat at $0 across the §1211(b) pin"
    );
    // THE IDENTITY: the carryforward BURNED == the gain realized at N* (exact in the pinned region).
    assert!(
        r.carryforward_delta.long < dec!(0),
        "a gain BURNS the carried loss (negative delta)"
    );
    assert_eq!(
        r.lt_gain + r.carryforward_delta.long,
        dec!(0),
        "carryforward_out(0) − carryforward_out(N*) == gains absorbed"
    );
    assert!(
        (r.n_sat - 117_500_000).abs() <= 4 * TAU,
        "hand truth: ~1.175 BTC (g=47k), got {}",
        r.n_sat
    );
    let note = r.plateau_note.expect("a burn must be disclosed");
    assert!(
        note.contains("carryforward") && note.contains("SPENT"),
        "note: {note}"
    );
}

/// ★ ST FEEDBACK shrinks the 0% room. HIFO consumes an ST-gain lot ($30k basis) first — its $20k gain is
/// ORDINARY, raising `bottom_with` to $20k and eating half the $40k 0% zone — then the LT lot. `zero-ltcg`
/// pins N* at 1.5 BTC (only $20k of LT fits in 0%), STRICTLY below the naive LT-only answer (a full BTC).
#[test]
fn harvest_st_feedback_shrinks_zero_room() {
    let events = vec![
        buy(
            "ST",
            datetime!(2025-01-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(30000),
        ), // ST +20k, basis 30k
        buy(
            "LT",
            datetime!(2024-01-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ), // LT +40k, basis 10k
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(0), dec!(0));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(date!(2025 - 08 - 01), dec!(50000), HarvestTarget::ZeroLtcg),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        r.n_sat > LOT,
        "past the ST lot (which is pure ordinary feedback)"
    );
    assert_eq!(
        r.st_gain,
        dec!(20000),
        "the ST gain is realized (ordinary — the feedback)"
    );
    assert_eq!(
        r.with_result.pref_split.at_15,
        dec!(0),
        "all surviving LT dollars stay in 0%"
    );
    assert_eq!(
        r.with_result.pref_split.at_0,
        dec!(20000),
        "ST gain ate $20k of the $40k 0% zone"
    );
    assert!(
        (r.n_sat - 150_000_000).abs() <= 4 * TAU,
        "hand truth: ~1.5 BTC, got {}",
        r.n_sat
    );
}

/// ★ CROSS-NET EXPANDS the room. An ST LOSS (−$10k) consumed first cross-nets against LT gain (+$45k),
/// so the SURVIVING preferential gain is only $35k ≤ max_zero — the WHOLE pool fits in 0% (NotBinding),
/// STRICTLY above the naive per-lot answer (the $45k LT lot alone would spill $5k over max_zero).
#[test]
fn harvest_cross_net_expands_room() {
    let events = vec![
        buy(
            "STL",
            datetime!(2025-01-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(60000),
        ), // ST −10k (basis 60k)
        buy(
            "LTG",
            datetime!(2024-01-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ), // LT +45k (basis 5k)
    ];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(0), dec!(0));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(date!(2025 - 08 - 01), dec!(50000), HarvestTarget::ZeroLtcg),
    )
    .expect("harvest computes");
    assert_eq!(
        r.status,
        HarvestStatus::NotBinding,
        "cross-net shrinks pref to $35k ⇒ full pool fits 0%"
    );
    assert_eq!(r.n_sat, 2 * LOT);
    assert_eq!(
        r.st_gain,
        dec!(-10000),
        "the ST loss is the cross-net lever"
    );
    assert_eq!(r.lt_gain, dec!(45000));
    assert_eq!(
        r.with_result.pref_split.at_0,
        dec!(35000),
        "surviving pref = 45k − 10k, all in 0%"
    );
    assert_eq!(r.with_result.pref_split.at_15, dec!(0));
}

/// ★ QD STACKING shrinks the answer. Baseline QD $20k already fills half the $40k 0% zone, so `zero-ltcg`
/// leaves only $20k of room ⇒ N* = 0.5 BTC (half what it would be with no QD).
#[test]
fn harvest_qd_stacking_shrinks() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )]; // +40k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_of(FilingStatus::Single, dec!(0), dec!(0), dec!(20000), dec!(0)); // QD 20k

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(date!(2025 - 08 - 01), dec!(50000), HarvestTarget::ZeroLtcg),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        (r.n_sat - 50_000_000).abs() <= 4 * TAU,
        "hand truth: 0.5 BTC (QD ate half the zone), got {}",
        r.n_sat
    );
    assert_eq!(r.with_result.pref_split.at_15, dec!(0));
    assert_eq!(
        r.with_result.pref_split.at_0,
        dec!(40000),
        "QD $20k + LT $20k fill the 0% zone"
    );
}

/// ★ QD alone over max_zero ⇒ AlreadyBreached at N=0. QD $45k > $40k puts `at_15 > 0` before any sale.
#[test]
fn harvest_qd_alone_already_breached() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_of(FilingStatus::Single, dec!(0), dec!(0), dec!(45000), dec!(0)); // QD 45k

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(date!(2025 - 08 - 01), dec!(50000), HarvestTarget::ZeroLtcg),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::AlreadyBreached);
    assert_eq!(r.n_sat, 0);
    assert_eq!(
        r.baseline.pref_split.at_15,
        dec!(5000),
        "QD $45k − $40k spills $5k into 15%"
    );
}

/// ★ DUAL-BASIS gift zones (§1015). HIFO consumes a dual-basis gift lot (gain_basis $100k) FIRST; sold at
/// $80k it is the NoGainNoLoss zone (60k ≤ 80k ≤ 100k) ⇒ ZERO gain — a FLAT (zero-slope) segment the walk
/// traverses for free before reaching the +$70k gain lot. `gain=$35k` lands mid the gain lot at ~1.5 BTC.
#[test]
fn harvest_dual_basis_ngnl_zero_slope() {
    let mut events = gift_lot(
        "GIFT",
        0,
        cold(),
        LOT,
        Some(dec!(100000)), // gain basis 100k (HIFO key — highest ⇒ consumed first)
        date!(2024 - 01 - 01), // donor acq ⇒ tacked long-term
        dec!(60000),        // fmv-at-gift = loss basis 60k (dual: 60k < 100k)
        datetime!(2024-06-01 00:00:00 UTC),
    );
    events.push(buy(
        "G",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )); // +70k @ $80k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(80000),
            HarvestTarget::Gain(dec!(35000)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        r.n_sat > LOT,
        "the whole NGNL gift lot (zero gain) is traversed for free"
    );
    assert!(
        r.st_gain + r.lt_gain <= dec!(35000),
        "prefix gain ≤ the cap"
    );
    assert_eq!(
        r.st_gain,
        dec!(0),
        "NGNL gift lot tacks LT; the gain lot is LT too"
    );
    assert!(
        (r.n_sat - 150_000_000).abs() <= 4 * TAU,
        "hand truth: ~1.5 BTC, got {}",
        r.n_sat
    );
}

/// ★ TWO-EDGE 0→15→20. One 2-BTC lot ($10k/BTC basis, $410k/BTC price) whose gain spans all three §1(h)
/// zones; `fifteen-ltcg` (at_20 == 0) binds mid-lot at 1 BTC (cumulative gain $400k = max_fifteen).
#[test]
fn harvest_two_edge_0_15_20() {
    let events = vec![buy(
        "BIG",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        2 * LOT,
        dec!(20000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(0), dec!(0));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(410000),
            HarvestTarget::FifteenLtcg,
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        (r.n_sat - 100_000_000).abs() <= 4 * TAU,
        "boundary mid-lot at gain $400k, got {}",
        r.n_sat
    );
    assert_eq!(
        r.with_result.pref_split.at_20,
        dec!(0),
        "stays at/under 15% at N*"
    );
    assert_eq!(r.with_result.pref_split.at_0, dec!(40000));
    assert_eq!(
        r.with_result.pref_split.at_15,
        dec!(360000),
        "0→15 both crossed before the 20 edge"
    );
}

/// ★ NIIT KINK. ord/magi $190k; a growing LT gain crosses the $200k MAGI threshold mid-segment, so
/// `tax=$2,000` is decided by the §1411 3.8% kink (at g > $10k). The report DISCLOSES `niit_applies`.
#[test]
fn harvest_niit_kink() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )]; // +60k @ $70k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(190000), dec!(190000));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(70000),
            HarvestTarget::Tax(dec!(2000)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(
        r.marginal_tax <= dec!(2000) && r.marginal_tax >= dec!(1998),
        "at the cap: {}",
        r.marginal_tax
    );
    assert!(
        r.niit_incremental > dec!(0),
        "the boundary is PAST the NIIT kink (g > $10k)"
    );
    assert!(
        r.niit_applies,
        "bracket/tax answer discloses the +3.8% NIIT kink"
    );
}

/// ★ T1 per-segment monotonicity: within ONE lot segment marginal(N) is monotone (non-decreasing for a
/// gain lot) up to the ⌈legs/2⌉ = 1-cent band. Probe 10%..100% of a single LT gain lot.
#[test]
fn harvest_per_segment_monotone() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));
    let mut prev = dec!(-1000000);
    for k in 1..=10 {
        let n = LOT / 10 * k;
        let s = btctax_core::whatif::sell(
            &events,
            &prices,
            &cfg(),
            Some(&prof),
            &tables,
            &btctax_core::whatif::SellRequest {
                sell_sat: n,
                wallet: cold(),
                at: date!(2025 - 08 - 01),
                price: Some(dec!(50000)),
                method: None,
            },
        )
        .unwrap();
        assert!(
            s.marginal_tax >= prev - dec!(0.01),
            "T1: monotone within a cent band ({} < {})",
            s.marginal_tax,
            prev
        );
        prev = s.marginal_tax;
    }
}

/// ★ BOUNDARY EXACTNESS. `gain=$20,000` on a single linear-gain lot: the predicate holds at N* and FAILS
/// at N* + τ' (here 4,096 sats > τ). N* is within τ of the true crossing (~0.5 BTC).
#[test]
fn harvest_boundary_exactness() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )]; // +40k @ $50k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Gain(dec!(20000)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::Found);
    assert!(r.st_gain + r.lt_gain <= dec!(20000), "predicate TRUE at N*");
    assert!(
        (r.n_sat - 50_000_000).abs() <= 2 * TAU,
        "within τ of the 0.5 BTC crossing, got {}",
        r.n_sat
    );
    // FALSE at N* + τ': selling 4,096 sats more realizes > $20,000 of gain.
    let over = btctax_core::whatif::sell(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &btctax_core::whatif::SellRequest {
            sell_sat: r.n_sat + 4096,
            wallet: cold(),
            at: date!(2025 - 08 - 01),
            price: Some(dec!(50000)),
            method: None,
        },
    )
    .unwrap();
    assert!(
        over.st_gain + over.lt_gain > dec!(20000),
        "predicate FALSE at N* + τ'"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// status / refusal KATs
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Empty as-of pool ⇒ Ok(NoLots), n_sat 0 (the baseline position is still surfaced).
#[test]
fn harvest_no_lots() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        hot(),
        LOT,
        dec!(10000),
    )]; // in HOT, not cold
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&single(dec!(0), dec!(0))),
        &tables,
        &hreq(date!(2025 - 08 - 01), dec!(50000), HarvestTarget::ZeroLtcg),
    )
    .expect("computes a NoLots report");
    assert_eq!(r.status, HarvestStatus::NoLots);
    assert_eq!(r.n_sat, 0);
}

/// Basis-pending lot ⇒ REFUSAL. A gift with unknown donor basis is `basis_pending`, but it ALSO raises a
/// resting `UnknownBasisInbound` Hard blocker at its origin (independent of any disposal), so the year
/// itself is NOT computable — the honest, conservative behavior is to refuse (`YearNotComputable`), not
/// to silently harvest around it. (The architect's N_avail truncation-at-first-pending is retained in
/// core as belt-and-suspenders — see the code comment — but the baseline Hard-blocker gate fires first
/// in this engine: every basis-pending origin, gift OR FMV-missing income, raises a Hard blocker.)
#[test]
fn harvest_pending_basis_refuses_year_not_computable() {
    let mut events = vec![buy(
        "G",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    events.extend(gift_lot(
        "PEND",
        0,
        cold(),
        LOT,
        None,
        date!(2024 - 01 - 01),
        dec!(60000),
        datetime!(2024-06-01 00:00:00 UTC),
    ));
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));

    let err = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Gain(dec!(1000000)),
        ),
    )
    .expect_err("an unresolved pending-basis lot gates the year");
    assert!(matches!(err, WhatIfError::YearNotComputable(_)));
    // The refusal maps to a `HarvestStatus::YearNotComputable` label for the CLI.
    assert!(matches!(
        HarvestStatus::of_refusal(&err),
        HarvestStatus::YearNotComputable(_)
    ));
}

/// MFS ⇒ the §1211(b) limit is $1,500 (not $3,000). An all-loss pool: NotBinding, only $1,500 deductible.
#[test]
fn harvest_mfs_1500() {
    let events = vec![buy(
        "A",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(50000),
    )]; // −40k @ $10k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_of(
        FilingStatus::Mfs,
        dec!(100000),
        dec!(100000),
        dec!(0),
        dec!(0),
    );

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(10000),
            HarvestTarget::Tax(dec!(0)),
        ),
    )
    .expect("harvest computes");
    assert_eq!(r.status, HarvestStatus::NotBinding);
    assert_eq!(
        r.with_result.loss_deduction,
        dec!(1500),
        "MFS §1211(b) cap is $1,500"
    );
    assert_eq!(
        r.carryforward_delta.long,
        dec!(38500),
        "$40k − $1,500 carried"
    );
}

/// [R0-M5] Qss → Mfj status mapping: the breakpoint lookup uses the Mfj max_zero ($80k, not Single's
/// $40k). An $80k LT gain fits ENTIRELY in the Mfj 0% zone ⇒ NotBinding (Single would have bound at 0.5 BTC).
#[test]
fn harvest_qss_maps_to_mfj() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )]; // +80k @ $90k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = profile_of(FilingStatus::Qss, dec!(0), dec!(0), dec!(0), dec!(0));

    let r = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(date!(2025 - 08 - 01), dec!(90000), HarvestTarget::ZeroLtcg),
    )
    .expect("harvest computes");
    assert_eq!(
        r.status,
        HarvestStatus::NotBinding,
        "Qss uses the Mfj $80k 0% ceiling"
    );
    assert_eq!(r.n_sat, LOT);
    assert_eq!(r.with_result.pref_split.at_0, dec!(80000));
    assert_eq!(r.with_result.pref_split.at_15, dec!(0));
}

/// A negative `gain`/`tax` cap is ill-posed (empty prefix set) ⇒ InvalidTarget.
#[test]
fn harvest_invalid_negative_target() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let err = harvest(
        &events,
        &StaticPrices::default(),
        &cfg(),
        Some(&single(dec!(0), dec!(0))),
        &synth(2025),
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Gain(dec!(-1)),
        ),
    )
    .expect_err("negative cap rejected");
    assert!(matches!(err, WhatIfError::InvalidTarget(_)));
}

/// Refusal taxonomy mirrors `sell`: pre-2025, missing profile, future-no-price.
#[test]
fn harvest_refusals() {
    let events = vec![buy(
        "L",
        datetime!(2024-06-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let prices = StaticPrices::default();
    // pre-2025
    assert_eq!(
        harvest(
            &events,
            &prices,
            &cfg(),
            Some(&single(dec!(0), dec!(0))),
            &synth(2024),
            &hreq(date!(2024 - 06 - 01), dec!(50000), HarvestTarget::ZeroLtcg)
        )
        .unwrap_err(),
        WhatIfError::PreTransitionYear(2024)
    );
    // missing profile
    assert!(matches!(
        harvest(
            &events,
            &prices,
            &cfg(),
            None,
            &synth(2025),
            &hreq(date!(2025 - 08 - 01), dec!(50000), HarvestTarget::ZeroLtcg)
        )
        .unwrap_err(),
        WhatIfError::YearNotComputable(_)
    ));
    // future/off-dataset date, NO price, no dataset FMV ⇒ ProceedsRequired
    let fut = HarvestRequest {
        wallet: cold(),
        at: date!(2025 - 12 - 20),
        price: None,
        target: HarvestTarget::ZeroLtcg,
    };
    assert_eq!(
        harvest(
            &events,
            &StaticPrices::default(),
            &cfg(),
            Some(&single(dec!(0), dec!(0))),
            &synth(2025),
            &fut
        )
        .unwrap_err(),
        WhatIfError::Evaluate(btctax_core::project::EvaluateError::ProceedsRequired)
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// hard invariants: marginal identity, engine-verified answer, determinism, non-persistence
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The reported `marginal_tax` IS EXACTLY `with_result.total − baseline.total`; the answer is
/// engine-verified (the predicate holds at N*); and the result is deterministic (NFR4).
#[test]
fn harvest_marginal_identity_engine_verified_and_deterministic() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )]; // +40k @ $50k
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));
    let call = || {
        harvest(
            &events,
            &prices,
            &cfg(),
            Some(&prof),
            &tables,
            &hreq(
                date!(2025 - 08 - 01),
                dec!(50000),
                HarvestTarget::Tax(dec!(3000)),
            ),
        )
        .unwrap()
    };
    let r = call();
    assert_eq!(
        r.marginal_tax,
        r.with_result.total_federal_tax_attributable - r.baseline.total_federal_tax_attributable,
        "marginal identity (exact subtraction)"
    );
    assert!(
        r.marginal_tax <= dec!(3000),
        "engine-verified: the predicate holds at N*"
    );
    assert_eq!(r, call(), "NFR4: identical inputs ⇒ identical report");
}

/// Core-level non-persistence: `whatif::harvest` mutates NOTHING — `events` byte-identical, the canonical
/// projection unperturbed (the CLI vault-bytes KAT is `harvest_never_persists`).
#[test]
fn harvest_writes_nothing() {
    let events = vec![buy(
        "L",
        datetime!(2024-01-01 00:00:00 UTC),
        cold(),
        LOT,
        dec!(10000),
    )];
    let before = events.clone();
    let prices = StaticPrices::default();
    let tables = synth(2025);
    let prof = single(dec!(60000), dec!(60000));
    let proj_before = project(&events, &prices, &cfg());

    let _ = harvest(
        &events,
        &prices,
        &cfg(),
        Some(&prof),
        &tables,
        &hreq(
            date!(2025 - 08 - 01),
            dec!(50000),
            HarvestTarget::Tax(dec!(3000)),
        ),
    )
    .unwrap();

    assert_eq!(events, before, "events slice byte-identical");
    let proj_after = project(&events, &prices, &cfg());
    assert_eq!(
        proj_before.lots, proj_after.lots,
        "canonical projection unperturbed"
    );
    assert_eq!(proj_before.disposals.len(), proj_after.disposals.len());
}
