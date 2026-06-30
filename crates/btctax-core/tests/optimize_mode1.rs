//! Sub-project C, Task 4 — Mode-1 optimizer `optimize_year` KATs.
//!
//! The headline guarantee: the optimizer's `optimized_tax` equals an INDEPENDENT exhaustive brute-force
//! oracle (`oracle_min_total`) on small whole-lot fixtures, AND beats the named naive baseline. The oracle
//! enumerates every whole-lot subset assignment per disposal and scores each through B's
//! `score_assignment` — it shares NO code with the optimizer's candidate generators / contention logic, so
//! agreement proves optimality, not self-consistency.
//!
//! All fixtures are synthetic (privacy — no real reads); exact Decimal, no float (NFR5); two calls are
//! byte-identical (NFR4). Federal-only.
use btctax_core::conventions::Usd;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::optimize::{optimize_year, score_assignment, ApproxReason};
use btctax_core::price::{PriceProvider, StaticPrices};
use btctax_core::project::{LotMethod, ProjectionConfig};
use btctax_core::tax::tables::{
    LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
};
use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
use rust_decimal_macros::dec;
use std::collections::{BTreeMap, BTreeSet};
use time::macros::{datetime, offset};

const LOT: i64 = 100_000_000; // one whole BTC per lot

// ── synthetic table + profile (Single; 32% band starts at $90k so a $100k profile is marginal-32%) ───
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
    })
}
/// Single filer; ordinary == MAGI so a chosen ordinary income places the marginal rate AND keeps MAGI
/// below the $200k NIIT threshold for the small crypto gains in these KATs.
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

// ── event / id builders ──────────────────────────────────────────────────────────────────────────
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn wallet(label: &str) -> WalletId {
    WalletId::SelfCustody {
        label: label.into(),
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
/// A standing-order `MethodElection` (decision event; wallet None). `effective_from` 2025-01-01 binds all
/// post-2025 disposals to `method` — used to make the BASELINE a naive HIFO/LIFO pick.
fn method_election(seq: u64, ts: time::OffsetDateTime, method: LotMethod) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::MethodElection(MethodElection {
            effective_from: time::macros::date!(2025 - 01 - 01),
            method,
        }),
    }
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default() // FIFO default, TreatmentC
}
fn made() -> time::Date {
    time::macros::date!(2026 - 07 - 01)
}
fn no_attest() -> BTreeSet<EventId> {
    BTreeSet::new()
}

/// Reconstruct the optimizer's chosen assignment from the proposal (disposal → proposed picks), so we can
/// re-score it (e.g. to read `loss_deduction`).
fn assignment_of(p: &btctax_core::optimize::OptimizeProposal) -> BTreeMap<EventId, Vec<LotPick>> {
    p.per_disposal
        .iter()
        .map(|d| (d.disposal.clone(), d.proposed_selection.clone()))
        .collect()
}

/// `(disposal, its available (lot, sat) universe, need)` — one oracle input row per disposal.
type OracleDisposal = (EventId, Vec<(LotId, i64)>, i64);

/// INDEPENDENT exhaustive oracle: for each disposal enumerate ALL whole-lot subsets of its available lots
/// summing to `need`, take the cartesian product, score each via `score_assignment`, return the min total.
/// Infeasible cross-disposal combinations self-eliminate (`NotComputable` → skipped). No optimizer code.
fn oracle_min_total(
    events: &[LedgerEvent],
    prices: &dyn PriceProvider,
    config: &ProjectionConfig,
    year: i32,
    prof: Option<&TaxProfile>,
    tables: &dyn TaxTables,
    per_disposal: &[OracleDisposal],
) -> Usd {
    let mut per: Vec<Vec<Vec<LotPick>>> = Vec::new();
    for (_id, lots, need) in per_disposal {
        let n = lots.len();
        let mut subs: Vec<Vec<LotPick>> = Vec::new();
        for mask in 0u32..(1u32 << n) {
            let mut v = Vec::new();
            let mut sum = 0i64;
            for (i, (lot, sat)) in lots.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    v.push(LotPick {
                        lot: lot.clone(),
                        sat: *sat,
                    });
                    sum += *sat;
                }
            }
            if sum == *need {
                v.sort_by(|a, b| a.lot.cmp(&b.lot));
                subs.push(v);
            }
        }
        per.push(subs);
    }
    let mut assigns: Vec<BTreeMap<EventId, Vec<LotPick>>> = vec![BTreeMap::new()];
    for (di, subs) in per.iter().enumerate() {
        let id = &per_disposal[di].0;
        let mut next = Vec::new();
        for a in &assigns {
            for s in subs {
                let mut a2 = a.clone();
                a2.insert(id.clone(), s.clone());
                next.push(a2);
            }
        }
        assigns = next;
    }
    let mut best: Option<Usd> = None;
    for a in &assigns {
        if let TaxOutcome::Computed(r) =
            score_assignment(events, prices, config, year, prof, tables, a)
        {
            let t = r.total_federal_tax_attributable;
            best = Some(best.map_or(t, |b| b.min(t)));
        }
    }
    best.expect("oracle: at least one feasible assignment")
}

// ── (a) HIFO-beats-FIFO ────────────────────────────────────────────────────────────────────────────

/// One wallet, two long-term lots (low-basis old, high-basis newer), one all-LT sell. FIFO baseline picks
/// the low-basis lot (max gain); the optimum picks the high-basis lot. Both LT → pure basis minimisation.
#[test]
fn hifo_beats_fifo_matches_oracle() {
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
        sell(
            "D",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    let oracle = oracle_min_total(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &[(eid("D"), vec![(lid("LB"), LOT), (lid("HB"), LOT)], LOT)],
    );
    assert_eq!(p.optimized_tax, oracle, "optimizer == independent oracle");
    assert_eq!(p.optimized_tax, dec!(750.00)); // $5,000 LT gain @ 15%
    assert_eq!(p.baseline_tax, dec!(1350.00)); // FIFO: $9,000 LT gain @ 15%
    assert!(p.optimized_tax < p.baseline_tax);
    assert!(p.delta <= dec!(0));
    assert_eq!(p.delta, dec!(-600.00));
    assert!(!p.approximate);
    assert_eq!(p.approx_reason, None);
    assert_eq!(p.per_disposal.len(), 1);
    assert_eq!(p.per_disposal[0].proposed_selection, vec![pick("HB", LOT)]);
    assert_eq!(p.per_disposal[0].current_selection, vec![pick("LB", LOT)]);
}

// ── (b) Rate-awareness: naive-HIFO LOSES to a long-term pick ─────────────────────────────────────────

/// A short-term HIGH-basis lot vs a long-term LOWER-basis lot, with the ordinary marginal rate (32%) far
/// above the 15% LT rate. Naive HIFO takes the ST high-basis lot (smaller gain, ordinary rate); the true
/// optimum takes the LT lot (slightly larger gain, 15%) for a STRICTLY lower total tax. Baseline is a HIFO
/// standing order, so the optimizer is shown overriding the in-force basis-greedy method.
#[test]
fn rate_aware_naive_hifo_loses_to_long_term() {
    let events = vec![
        method_election(1, datetime!(2025-01-01 00:00:00 UTC), LotMethod::Hifo),
        buy(
            "LT_LB",
            datetime!(2025-01-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(9000),
        ),
        buy(
            "ST_HB",
            datetime!(2026-05-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(9500),
        ),
        sell(
            "D",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    let oracle = oracle_min_total(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &[(
            eid("D"),
            vec![(lid("LT_LB"), LOT), (lid("ST_HB"), LOT)],
            LOT,
        )],
    );
    // The naive basis-greedy (all-HIFO) pick = the ST high-basis lot.
    let all_hifo = btreemap_assign(&[(eid("D"), vec![pick("ST_HB", LOT)])]);
    let TaxOutcome::Computed(hifo) = score_assignment(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &all_hifo,
    ) else {
        panic!("HIFO pick computable");
    };

    assert_eq!(p.optimized_tax, oracle, "optimizer == independent oracle");
    assert_eq!(p.optimized_tax, dec!(150.00)); // $1,000 LT gain @ 15%
    assert_eq!(hifo.total_federal_tax_attributable, dec!(160.00)); // $500 ST gain @ 32%
    assert!(
        p.optimized_tax < hifo.total_federal_tax_attributable,
        "rate-aware optimum strictly beats naive HIFO ({} !< {})",
        p.optimized_tax,
        hifo.total_federal_tax_attributable
    );
    assert_eq!(p.baseline_tax, dec!(160.00)); // HIFO standing order == naive-HIFO
    assert!(p.delta <= dec!(0));
    assert_eq!(
        p.per_disposal[0].proposed_selection,
        vec![pick("LT_LB", LOT)]
    );
    assert!(!p.approximate);
}

fn btreemap_assign(entries: &[(EventId, Vec<LotPick>)]) -> BTreeMap<EventId, Vec<LotPick>> {
    entries.iter().cloned().collect()
}

// ── (d) Loss-harvest within the $3k limit (R0-I3: assert only what the single-year objective pins) ───

/// A forced gain disposal (wallet `g`) + a wallet (`h`) holding loss lots of different magnitudes. The
/// single-year objective is carryforward-blind: once a pick offsets all gains AND takes the $3,000 §1211
/// cap, any EXTRA realized loss only grows the carryforward — the objective is identical. So "harvest
/// exactly enough" and "over-harvest" TIE; we assert ONLY (a) optimizer == oracle and < baseline, and
/// (b) the in-year `loss_deduction == $3,000` — NOT a carryforward split.
#[test]
fn loss_harvest_within_3k_limit() {
    let g = wallet("g");
    let h = wallet("h");
    let events = vec![
        buy(
            "GLO",
            datetime!(2026-05-01 00:00:00 UTC),
            g.clone(),
            LOT,
            dec!(1000),
        ),
        // FIFO-first loss lot is INSUFFICIENT (only $4k loss); the better picks come later in time.
        buy(
            "HB3",
            datetime!(2026-05-01 00:00:00 UTC),
            h.clone(),
            LOT,
            dec!(5000),
        ),
        buy(
            "HB1",
            datetime!(2026-05-02 00:00:00 UTC),
            h.clone(),
            LOT,
            dec!(9000),
        ),
        buy(
            "HB2",
            datetime!(2026-05-03 00:00:00 UTC),
            h.clone(),
            LOT,
            dec!(11000),
        ),
        sell(
            "DG",
            datetime!(2026-06-01 00:00:00 UTC),
            g.clone(),
            LOT,
            dec!(6000),
        ), // +$5,000 ST gain
        sell(
            "DL",
            datetime!(2026-06-01 00:00:00 UTC),
            h.clone(),
            LOT,
            dec!(1000),
        ), // a loss lot
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    let oracle = oracle_min_total(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &[
            (eid("DG"), vec![(lid("GLO"), LOT)], LOT),
            (
                eid("DL"),
                vec![(lid("HB1"), LOT), (lid("HB2"), LOT), (lid("HB3"), LOT)],
                LOT,
            ),
        ],
    );
    assert_eq!(p.optimized_tax, oracle, "optimizer == independent oracle");
    assert!(
        p.optimized_tax < p.baseline_tax,
        "harvest beats the FIFO baseline"
    );
    assert!(p.delta < dec!(0));
    assert!(!p.approximate);

    // (b) the in-year loss_deduction is pinned at the $3,000 §1211 cap (gains fully offset + cap taken).
    let best = assignment_of(&p);
    let TaxOutcome::Computed(r) =
        score_assignment(&events, &prices, &cfg(), 2026, Some(&prof), &tables, &best)
    else {
        panic!("best assignment computable");
    };
    assert_eq!(r.loss_deduction, dec!(3000.00));
}

// ── Per-wallet constraint (§1.1012-1(j)) ─────────────────────────────────────────────────────────────

/// The globally cheapest lot lives in ANOTHER wallet; cross-account identification is forbidden, so the
/// optimum is the oracle restricted to the disposal's OWN wallet and the cross-wallet lot never appears.
#[test]
fn per_wallet_constraint_respected() {
    let hot = wallet("hot");
    let events = vec![
        buy(
            "CL_LOW",
            datetime!(2026-05-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        buy(
            "CL_HIGH",
            datetime!(2026-05-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(8000),
        ),
        buy(
            "HL_SUPER",
            datetime!(2026-05-01 00:00:00 UTC),
            hot.clone(),
            LOT,
            dec!(9500),
        ),
        sell(
            "DC",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    // oracle restricted to cold's own wallet lots only.
    let oracle = oracle_min_total(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &[(
            eid("DC"),
            vec![(lid("CL_LOW"), LOT), (lid("CL_HIGH"), LOT)],
            LOT,
        )],
    );
    assert_eq!(p.optimized_tax, oracle);
    assert_eq!(p.optimized_tax, dec!(640.00)); // $2,000 ST gain @ 32%
    assert!(p.optimized_tax < p.baseline_tax); // FIFO baseline = CL_LOW ($9,000 gain)
    assert_eq!(
        p.per_disposal[0].proposed_selection,
        vec![pick("CL_HIGH", LOT)]
    );
    // the cross-wallet lot is NEVER selected.
    for d in &p.per_disposal {
        assert!(
            d.proposed_selection
                .iter()
                .all(|pk| pk.lot != lid("HL_SUPER")),
            "cross-wallet lot must never appear in a proposed selection"
        );
    }
    assert!(!p.approximate);
}

// ── (c) Contended same-wallet sells across an ST/LT crossover ────────────────────────────────────────

/// Two same-wallet sells D1 (earlier) and D2 (later) in 2026, with lot `P` ST at D1's date but LT at D2's.
/// Under the LIFO standing order the baseline puts `P` at D1 (ST/ordinary). The true optimum REASSIGNS
/// `P` to D2 (LT/15%) and `R` to D1 — a JOINT sequence the independent per-disposal product cannot reach.
/// Within `GROUP_COMBO_BOUND` the optimizer finds it and `approximate == false`; the oracle enumerates the
/// JOINT space and agrees.
#[test]
fn contended_st_lt_crossover_finds_joint_optimum() {
    let events = vec![
        method_election(1, datetime!(2025-01-01 00:00:00 UTC), LotMethod::Lifo),
        buy(
            "R",
            datetime!(2025-05-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
        buy(
            "P",
            datetime!(2025-06-15 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
        sell(
            "D1",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
        sell(
            "D2",
            datetime!(2026-06-20 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    // Oracle over the JOINT whole-lot space: both disposals may draw R or P (both acquired before both).
    let oracle = oracle_min_total(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &[
            (eid("D1"), vec![(lid("R"), LOT), (lid("P"), LOT)], LOT),
            (eid("D2"), vec![(lid("R"), LOT), (lid("P"), LOT)], LOT),
        ],
    );
    assert_eq!(p.optimized_tax, oracle, "optimizer == JOINT oracle");
    assert_eq!(p.optimized_tax, dec!(1500.00)); // both LT: $10,000 @ 15%
    assert_eq!(p.baseline_tax, dec!(2350.00)); // LIFO: $5k ST @ 32% + $5k LT @ 15% = $1,600 + $750
    assert!(
        p.optimized_tax < p.baseline_tax,
        "the joint reassignment beats the (best-independent) baseline"
    );
    assert!(p.delta <= dec!(0));
    assert!(!p.approximate, "contention jointly enumerated within bound");
    assert_eq!(p.approx_reason, None);
    // D1 took the LT lot R; D2 took P (LT at D2's date).
    let by: BTreeMap<&EventId, &Vec<LotPick>> = p
        .per_disposal
        .iter()
        .map(|d| (&d.disposal, &d.proposed_selection))
        .collect();
    assert_eq!(by[&eid("D1")], &vec![pick("R", LOT)]);
    assert_eq!(by[&eid("D2")], &vec![pick("P", LOT)]);
}

/// Variant forcing the contended group PAST `GROUP_COMBO_BOUND` (4 same-wallet sells over a 10-lot pool:
/// 10·9·8·7 = 5040 > 4096). The proposal is flagged `approximate = true, ContentionUnenumerated`, and the
/// baseline-seed still guarantees `delta ≤ 0`.
#[test]
fn contended_beyond_bound_flags_unenumerated() {
    let mut events: Vec<LedgerEvent> = (0..10)
        .map(|i| {
            buy(
                &format!("L{i:02}"),
                datetime!(2026-05-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(5000),
            )
        })
        .collect();
    for (k, d) in [
        datetime!(2026-06-01 00:00:00 UTC),
        datetime!(2026-06-02 00:00:00 UTC),
        datetime!(2026-06-03 00:00:00 UTC),
        datetime!(2026-06-04 00:00:00 UTC),
    ]
    .into_iter()
    .enumerate()
    {
        events.push(sell(&format!("D{k}"), d, cold(), LOT, dec!(10000)));
    }
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    assert!(p.approximate);
    match p.approx_reason {
        Some(ApproxReason::ContentionUnenumerated { contended, .. }) => {
            assert_eq!(contended, 4, "all four contended disposals counted");
        }
        other => panic!("expected ContentionUnenumerated, got {other:?}"),
    }
    assert!(p.delta <= dec!(0)); // equal-basis lots ⇒ delta == 0
}

// ── approximate honesty: ComboCapExceeded (coordinate-descent fallback) ──────────────────────────────

/// Two wallets, each a 5-of-10-lot sell: product = C(10,5)² = 252² = 63,504 > MAX_COMBOS → baseline-seeded
/// coordinate descent. `approximate = true, ComboCapExceeded`, and `delta ≤ 0`.
#[test]
fn combo_cap_exceeded_falls_back_baseline_seeded() {
    let a = wallet("a");
    let b = wallet("b");
    let mut events: Vec<LedgerEvent> = Vec::new();
    for i in 0..10 {
        events.push(buy(
            &format!("A{i:02}"),
            datetime!(2026-05-01 00:00:00 UTC),
            a.clone(),
            LOT,
            dec!(5000),
        ));
        events.push(buy(
            &format!("B{i:02}"),
            datetime!(2026-05-01 00:00:00 UTC),
            b.clone(),
            LOT,
            dec!(5000),
        ));
    }
    events.push(sell(
        "DA",
        datetime!(2026-06-01 00:00:00 UTC),
        a.clone(),
        5 * LOT,
        dec!(50000),
    ));
    events.push(sell(
        "DB",
        datetime!(2026-06-01 00:00:00 UTC),
        b.clone(),
        5 * LOT,
        dec!(50000),
    ));
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    assert!(p.approximate);
    assert_eq!(
        p.approx_reason,
        Some(ApproxReason::ComboCapExceeded {
            combos: 63_504,
            cap: 50_000
        })
    );
    assert!(p.delta <= dec!(0)); // equal-basis ⇒ delta == 0; never worse than baseline
}

// ── approximate honesty: PoolHeuristic (incomplete vertex subset) both ways ──────────────────────────

/// A single disposal over a `> LOT_ENUM_BOUND` (20-lot) pool: `candidate_selections` returns the heuristic
/// INCOMPLETE subset, so the overall product is small yet the result is NOT a proven global minimum.
/// `approximate = true, PoolHeuristic { lots: 20, bound: 12 }`, `delta ≤ 0`.
#[test]
fn pool_heuristic_disclosed_above_bound() {
    let mut events: Vec<LedgerEvent> = (0..20)
        .map(|i| {
            buy(
                &format!("L{i:02}"),
                datetime!(2026-05-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(5000),
            )
        })
        .collect();
    events.push(sell(
        "D",
        datetime!(2026-06-01 00:00:00 UTC),
        cold(),
        2 * LOT,
        dec!(20000),
    ));
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    assert!(p.approximate);
    assert_eq!(
        p.approx_reason,
        Some(ApproxReason::PoolHeuristic {
            lots: 20,
            bound: 12
        })
    );
    assert!(p.delta <= dec!(0));
}

/// Mirror: a `≤ LOT_ENUM_BOUND` (12-lot) pool with a small product → fully enumerated ⇒ proven global
/// minimum ⇒ `approximate == false, approx_reason == None`. Together with the above this pins
/// `approximate == false ⇔ fully-enumerated-global`.
#[test]
fn small_pool_is_not_approximate() {
    let mut events: Vec<LedgerEvent> = (0..12)
        .map(|i| {
            buy(
                &format!("L{i:02}"),
                datetime!(2026-05-01 00:00:00 UTC),
                cold(),
                LOT,
                dec!(5000),
            )
        })
        .collect();
    events.push(sell(
        "D",
        datetime!(2026-06-01 00:00:00 UTC),
        cold(),
        2 * LOT,
        dec!(20000),
    ));
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    assert!(!p.approximate);
    assert_eq!(p.approx_reason, None);
    assert!(p.delta <= dec!(0));
}

// ── refusals ─────────────────────────────────────────────────────────────────────────────────────

#[test]
fn refuses_pre_2025_year() {
    let events = vec![
        buy(
            "LB",
            datetime!(2025-01-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        sell(
            "D",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2024);
    let prof = profile(dec!(100000));
    let err = optimize_year(
        &events,
        &prices,
        &cfg(),
        2024,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .unwrap_err();
    assert_eq!(
        err,
        btctax_core::optimize::OptimizeError::PreTransitionYear(2024)
    );
}

#[test]
fn refuses_year_with_no_disposals() {
    let events = vec![buy(
        "LB",
        datetime!(2026-01-02 00:00:00 UTC),
        cold(),
        LOT,
        dec!(1000),
    )];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let err = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .unwrap_err();
    assert_eq!(err, btctax_core::optimize::OptimizeError::NoDisposals);
}

#[test]
fn refuses_year_not_computable_missing_profile() {
    let events = vec![
        buy(
            "LB",
            datetime!(2025-01-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(1000),
        ),
        sell(
            "D",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    // No profile → B refuses (TaxProfileMissing, Hard) → optimizer returns YearNotComputable (I6).
    let err = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        None,
        &tables,
        &no_attest(),
        made(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        btctax_core::optimize::OptimizeError::YearNotComputable(_)
    ));
}

// ── determinism + tie-break ─────────────────────────────────────────────────────────────────────────

#[test]
fn optimize_year_is_deterministic() {
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
        sell(
            "D",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let run = || {
        optimize_year(
            &events,
            &prices,
            &cfg(),
            2026,
            Some(&prof),
            &tables,
            &no_attest(),
            made(),
        )
        .unwrap()
    };
    assert_eq!(run(), run(), "byte-identical OptimizeProposal across calls");
}

/// All-equal-tax tie: two equal-basis lots, one sell of one lot's worth → every pick is a tie on the
/// objective, broken to the lexicographically-smallest assignment (NFR4 §0 total order).
#[test]
fn tie_returns_lexicographically_smallest_assignment() {
    let events = vec![
        buy(
            "A",
            datetime!(2025-01-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
        buy(
            "B",
            datetime!(2025-02-02 00:00:00 UTC),
            cold(),
            LOT,
            dec!(5000),
        ),
        sell(
            "D",
            datetime!(2026-06-01 00:00:00 UTC),
            cold(),
            LOT,
            dec!(10000),
        ),
    ];
    let prices = StaticPrices::default();
    let tables = synth(2026);
    let prof = profile(dec!(100000));
    let p = optimize_year(
        &events,
        &prices,
        &cfg(),
        2026,
        Some(&prof),
        &tables,
        &no_attest(),
        made(),
    )
    .expect("computable");

    // Equal basis ⇒ identical tax ⇒ delta 0; the proposed pick is the smaller-lot-id of {A, B}.
    assert_eq!(p.delta, dec!(0));
    let want_lot = if lid("A") < lid("B") {
        pick("A", LOT)
    } else {
        pick("B", LOT)
    };
    assert_eq!(p.per_disposal[0].proposed_selection, vec![want_lot]);
}
