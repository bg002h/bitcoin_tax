use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::state::*;
use btctax_core::LotMethod;
use rust_decimal_macros::dec;
use time::macros::{date, datetime, offset};

fn w() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn imp(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w()),
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
fn buy(rf: &str, ts: time::OffsetDateTime, sat: i64, cost: rust_decimal::Decimal) -> LedgerEvent {
    imp(
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
fn sell(
    rf: &str,
    ts: time::OffsetDateTime,
    sat: i64,
    proceeds: rust_decimal::Decimal,
) -> LedgerEvent {
    imp(
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
fn election(seq: u64, made: time::OffsetDateTime, eff: time::Date, m: LotMethod) -> LedgerEvent {
    dec_ev(
        seq,
        made,
        EventPayload::MethodElection(MethodElection {
            effective_from: eff,
            method: m,
            wallet: None,
        }),
    )
}
fn has(st: &LedgerState, k: BlockerKind) -> bool {
    st.blockers.iter().any(|b| b.kind == k)
}

// Post-2025 pool with 3 lots whose method orders are distinct (FIFO->A, LIFO->C, HIFO->B).
fn three_post2025() -> Vec<LedgerEvent> {
    vec![
        buy(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy(
            "B",
            datetime!(2025-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        buy(
            "C",
            datetime!(2025-04-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ),
    ]
}

/// [reconcile-defaults] With NO election on file, the post-2025 default is HIFO (was FIFO): the sale
/// consumes the HIGHEST-basis lot (B $90), not the oldest (A $50). ★ fault-inject target.
#[test]
fn default_method_is_hifo() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        st.disposals[0].legs[0].basis,
        dec!(90.00),
        "no election → HIFO default → highest-basis lot B"
    );
}

/// [reconcile-defaults] An explicit GLOBAL FIFO election still yields FIFO — the flip changed ONLY the
/// no-election default, not the resolver: FIFO stays electable and honored (consumes oldest A $50).
#[test]
fn explicit_fifo_election_still_fifo() {
    let mut evs = three_post2025();
    evs.push(election(
        1,
        datetime!(2025-01-01 00:00:00 UTC),
        date!(2025 - 01 - 01),
        LotMethod::Fifo,
    ));
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(
        st.disposals[0].legs[0].basis,
        dec!(50.00),
        "explicit FIFO election → oldest lot A"
    );
}

#[test]
fn election_applies_on_or_after_effective_from_else_fifo() {
    let mut evs = three_post2025();
    // [reconcile-defaults] pin the pre-election baseline to FIFO explicitly (default is now HIFO), so
    // "before effective_from" resolves to this in-force FIFO election rather than the HIFO default.
    evs.push(election(
        5,
        datetime!(2025-01-01 00:00:00 UTC),
        date!(2025 - 01 - 01),
        LotMethod::Fifo,
    ));
    // HIFO standing order recorded 2025-05-01, effective 2025-06-01.
    evs.push(election(
        1,
        datetime!(2025-05-01 00:00:00 UTC),
        date!(2025 - 06 - 01),
        LotMethod::Hifo,
    ));
    // Disposal BEFORE effective_from -> FIFO (consumes A).
    evs.push(sell(
        "D1",
        datetime!(2025-05-15 00:00:00 UTC),
        100_000,
        dec!(70.00),
    ));
    // Disposal ON/AFTER effective_from -> HIFO (of what remains: B then C; picks B).
    evs.push(sell(
        "D2",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(!has(&st, BlockerKind::MethodElectionBackdated));
    let d1 = st
        .disposals
        .iter()
        .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D1")))
        .unwrap();
    assert_eq!(d1.legs[0].basis, dec!(50.00)); // FIFO -> A
    let d2 = st
        .disposals
        .iter()
        .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D2")))
        .unwrap();
    assert_eq!(d2.legs[0].basis, dec!(90.00)); // HIFO -> B
}

#[test]
fn latest_in_force_election_wins() {
    let mut evs = three_post2025();
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Lifo,
    )); // effective first
    evs.push(election(
        2,
        datetime!(2025-05-01 00:00:00 UTC),
        date!(2025 - 06 - 01),
        LotMethod::Hifo,
    )); // later, governs after
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let d = &st.disposals[0];
    assert_eq!(d.legs[0].basis, dec!(90.00)); // latest-in-force HIFO -> B
}

#[test]
fn backdated_election_is_rejected() {
    let mut evs = three_post2025();
    // [reconcile-defaults] a valid global FIFO election is the fall-through once the backdated one is
    // rejected (the default is now HIFO), so the rejection is still observable as FIFO -> A.
    evs.push(election(
        5,
        datetime!(2025-01-01 00:00:00 UTC),
        date!(2025 - 01 - 01),
        LotMethod::Fifo,
    ));
    // effective_from (2025-02-10) precedes the made-date (2025-05-01) -> backdated.
    evs.push(election(
        1,
        datetime!(2025-05-01 00:00:00 UTC),
        date!(2025 - 02 - 10),
        LotMethod::Hifo,
    ));
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // rejected election -> FIFO -> A
}

#[test]
fn pre_transition_election_is_rejected() {
    let mut evs = three_post2025();
    // [reconcile-defaults] valid FIFO fall-through so the pre-transition rejection reads as FIFO -> A
    // (the default is now HIFO).
    evs.push(election(
        5,
        datetime!(2025-01-01 00:00:00 UTC),
        date!(2025 - 01 - 01),
        LotMethod::Fifo,
    ));
    evs.push(election(
        1,
        datetime!(2024-06-01 00:00:00 UTC),
        date!(2024 - 06 - 01),
        LotMethod::Hifo,
    )); // effective_from < TRANSITION_DATE
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::MethodElectionBackdated));
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // FIFO default
}

#[test]
fn voided_election_is_excluded() {
    let mut evs = three_post2025();
    // [reconcile-defaults] valid FIFO fall-through so voiding the HIFO election is observable as a revert
    // to FIFO -> A (default is now HIFO; without this the voided-HIFO and HIFO-default picks would tie).
    evs.push(election(
        5,
        datetime!(2025-01-01 00:00:00 UTC),
        date!(2025 - 01 - 01),
        LotMethod::Fifo,
    ));
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
    ));
    evs.push(dec_ev(
        2,
        datetime!(2025-06-01 00:00:00 UTC),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent {
            target_event_id: EventId::decision(1),
        }),
    ));
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(st.disposals[0].legs[0].basis, dec!(50.00)); // voided HIFO -> back to FIFO -> A
}

#[test]
fn pre2025_universal_uses_pre2025_method() {
    // Pre-2025 pool A/B/C in Universal; pre-2025 sell under HIFO consumes B.
    let evs = vec![
        buy(
            "A",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy(
            "B",
            datetime!(2024-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        buy(
            "C",
            datetime!(2024-04-01 00:00:00 UTC),
            100_000,
            dec!(40.00),
        ),
        sell(
            "D",
            datetime!(2024-09-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
    ];
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Hifo,
        ..ProjectionConfig::default()
    };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    assert_eq!(st.disposals[0].legs[0].basis, dec!(90.00)); // HIFO -> B
}

#[test]
fn pre2025_method_note_renders_declared_method() {
    let evs = vec![
        buy(
            "A",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell("D", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(40.00)),
    ];
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Hifo,
        ..ProjectionConfig::default()
    };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    let note = st
        .blockers
        .iter()
        .find(|b| b.kind == BlockerKind::Pre2025MethodNote)
        .unwrap();
    assert!(
        note.detail.contains("HIFO"),
        "note must name the declared method, got: {}",
        note.detail
    );
}

// ── Task 2 KATs: attestation-aware note_pre2025_once ─────────────────────────────────────────────

/// (a) Unattested: detail contains "have NOT declared" + "config --set-pre2025-method" guidance.
#[test]
fn pre2025_note_unattested_detail_is_actionable() {
    let evs = vec![
        buy(
            "A",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell("D", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(40.00)),
    ];
    // Default: pre2025_method_attested = false
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        pre2025_method_attested: false,
        ..ProjectionConfig::default()
    };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    let note = st
        .blockers
        .iter()
        .find(|b| b.kind == BlockerKind::Pre2025MethodNote)
        .expect("Pre2025MethodNote must fire on a pre-2025 disposal");
    assert_eq!(
        note.kind.severity(),
        Severity::Advisory,
        "Pre2025MethodNote must be Advisory (never gates compute_tax_year)"
    );
    assert!(
        note.detail.contains("have NOT declared"),
        "unattested detail must contain 'have NOT declared', got: {}",
        note.detail
    );
    assert!(
        note.detail.contains("config --set-pre2025-method"),
        "unattested detail must contain config guidance, got: {}",
        note.detail
    );
    assert!(
        note.detail.contains("FIFO"),
        "unattested detail must name the method, got: {}",
        note.detail
    );
}

/// (b) Attested: detail contains "DECLARED + ATTESTED".
#[test]
fn pre2025_note_attested_detail_is_informational() {
    let evs = vec![
        buy(
            "A",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell("D", datetime!(2024-09-01 00:00:00 UTC), 50_000, dec!(40.00)),
    ];
    let cfg = ProjectionConfig {
        pre2025_method: LotMethod::Fifo,
        pre2025_method_attested: true,
        ..ProjectionConfig::default()
    };
    let st = project(&evs, &StaticPrices::default(), &cfg);
    let note = st
        .blockers
        .iter()
        .find(|b| b.kind == BlockerKind::Pre2025MethodNote)
        .expect("Pre2025MethodNote must fire on a pre-2025 disposal");
    assert_eq!(
        note.kind.severity(),
        Severity::Advisory,
        "Pre2025MethodNote must be Advisory even when attested"
    );
    assert!(
        note.detail.contains("DECLARED + ATTESTED"),
        "attested detail must contain 'DECLARED + ATTESTED', got: {}",
        note.detail
    );
    assert!(
        note.detail.contains("FIFO"),
        "attested detail must name the method, got: {}",
        note.detail
    );
    assert!(
        !note.detail.contains("have NOT declared"),
        "attested detail must NOT contain the unattested warning, got: {}",
        note.detail
    );
}

/// (c) Fire-once: a second pre-2025 disposal in the same projection does NOT emit a second note.
#[test]
fn pre2025_note_fires_only_once() {
    let evs = vec![
        buy(
            "A",
            datetime!(2024-01-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell(
            "D1",
            datetime!(2024-06-01 00:00:00 UTC),
            30_000,
            dec!(20.00),
        ),
        sell(
            "D2",
            datetime!(2024-09-01 00:00:00 UTC),
            30_000,
            dec!(20.00),
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    let notes: Vec<_> = st
        .blockers
        .iter()
        .filter(|b| b.kind == BlockerKind::Pre2025MethodNote)
        .collect();
    assert_eq!(
        notes.len(),
        1,
        "Pre2025MethodNote must fire exactly once, got {}",
        notes.len()
    );
}

/// (c) Advisory note does not gate compute_tax_year: a year whose ONLY blocker is Pre2025MethodNote
/// still yields TaxOutcome::Computed(..) for both attested and unattested configurations.
#[test]
fn pre2025_advisory_note_does_not_gate_compute_tax_year() {
    use btctax_core::tax::compute::compute_tax_year;
    use btctax_core::tax::tables::{
        LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable, TaxTables,
    };
    use btctax_core::tax::types::{Carryforward, FilingStatus, TaxOutcome, TaxProfile};
    use std::collections::BTreeMap;

    struct OneTable(TaxTable);
    impl TaxTables for OneTable {
        fn table_for(&self, year: i32) -> Option<&TaxTable> {
            (year == self.0.year).then_some(&self.0)
        }
    }
    fn synth_2024() -> OneTable {
        let mut ordinary = BTreeMap::new();
        ordinary.insert(
            FilingStatus::Single,
            OrdinarySchedule {
                brackets: vec![OrdinaryBracket {
                    lower: dec!(0),
                    rate: dec!(0.22),
                }],
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
            year: 2024,
            source: "SYNTHETIC",
            ordinary,
            ltcg,
            gift_annual_exclusion: dec!(19000),
            ss_wage_base: dec!(176100),
            gift_lifetime_exclusion: dec!(13_990_000),
        })
    }
    let prof = TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: dec!(0),
        magi_excluding_crypto: dec!(0),
        qualified_dividends_and_other_pref_income: dec!(0),
        other_net_capital_gain: dec!(0),
        capital_loss_carryforward_in: Carryforward {
            short: dec!(0),
            long: dec!(0),
        },
        w2_ss_wages: dec!(0),
        w2_medicare_wages: dec!(0),
        schedule_c_expenses: dec!(0),
    };
    let evs = vec![
        buy(
            "A",
            datetime!(2024-01-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell("D", datetime!(2024-06-01 00:00:00 UTC), 50_000, dec!(40.00)),
    ];
    let tables = synth_2024();

    // Unattested: note fires with warning, compute_tax_year still returns Computed.
    let st_unattested = project(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig {
            pre2025_method_attested: false,
            ..ProjectionConfig::default()
        },
    );
    assert!(has(&st_unattested, BlockerKind::Pre2025MethodNote));
    assert!(
        matches!(
            compute_tax_year(&evs, &st_unattested, 2024, Some(&prof), &tables),
            TaxOutcome::Computed(..)
        ),
        "unattested Pre2025MethodNote must not gate compute_tax_year"
    );

    // Attested: note fires with informational detail, compute_tax_year still returns Computed.
    let st_attested = project(
        &evs,
        &StaticPrices::default(),
        &ProjectionConfig {
            pre2025_method_attested: true,
            ..ProjectionConfig::default()
        },
    );
    assert!(has(&st_attested, BlockerKind::Pre2025MethodNote));
    assert!(
        matches!(
            compute_tax_year(&evs, &st_attested, 2024, Some(&prof), &tables),
            TaxOutcome::Computed(..)
        ),
        "attested Pre2025MethodNote must not gate compute_tax_year"
    );
}

// ── C1 divergence KAT (a) — acquisition-date FIFO vs legacy insertion-order on a RELOCATED lot ──
// A confirmed SelfTransfer relocates the OLDER lot Z (acquired 2025-01-01, basis $40) from COLD into HOT,
// which already holds the NEWER directly-acquired A (acquired 2025-08-01, basis $80). Z' carries its original
// acquired_at and is push_lot'd AFTER A, so HOT's insertion order is [A, Z'] while acquisition order is
// [Z', A]. A partial FIFO Dispose MUST consume the OLDER Z' first (legacy insertion-order wrongly took A).
// Basis AND term flip. LIFO/HIFO variants over the same fixture pin the full total order (both pick A).
#[test]
fn relocated_older_lot_consumed_first_under_acq_date_fifo_diverging_from_insertion_order() {
    let hot = WalletId::Exchange {
        provider: "cb".into(),
        account: "hot".into(),
    };
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let acq = |rf: &str, ts: time::OffsetDateTime, wal: &WalletId, cost: rust_decimal::Decimal| {
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
            utc_timestamp: ts,
            original_tz: offset!(+00:00),
            wallet: Some(wal.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: cost,
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        }
    };
    let scenario = |extra: Vec<LedgerEvent>| -> LedgerState {
        let mut evs = vec![
            acq("Z", datetime!(2025-01-01 00:00:00 UTC), &cold, dec!(40.00)), // COLD, OLDER, $40
            acq("A", datetime!(2025-08-01 00:00:00 UTC), &hot, dec!(80.00)),  // HOT,  NEWER, $80
            LedgerEvent {
                id: EventId::import(Source::Swan, SourceRef::new("OUT")),
                utc_timestamp: datetime!(2025-09-01 00:00:00 UTC),
                original_tz: offset!(+00:00),
                wallet: Some(cold.clone()),
                payload: EventPayload::TransferOut(TransferOut {
                    sat: 100_000,
                    fee_sat: None,
                    dest_addr: None,
                    txid: None,
                }),
            },
            dec_ev(
                1,
                datetime!(2025-09-02 00:00:00 UTC),
                EventPayload::TransferLink(TransferLink {
                    out_event: EventId::import(Source::Swan, SourceRef::new("OUT")),
                    in_event_or_wallet: TransferTarget::Wallet(hot.clone()),
                }),
            ), // relocate Z' -> HOT (pushed AFTER A)
        ];
        // [reconcile-defaults] pin the no-election baseline to FIFO (default is now HIFO); the LIFO/HIFO
        // `extra` elections (effective 2025-10-01) still supersede this for those branches.
        evs.push(election(
            3,
            datetime!(2025-01-01 00:00:00 UTC),
            date!(2025 - 01 - 01),
            LotMethod::Fifo,
        ));
        evs.extend(extra);
        evs.push(LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("D")),
            utc_timestamp: datetime!(2026-02-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(hot.clone()),
            payload: EventPayload::Dispose(Dispose {
                sat: 100_000,
                usd_proceeds: dec!(150.00),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        });
        project(&evs, &StaticPrices::default(), &ProjectionConfig::default())
    };
    let leg0 = |st: &LedgerState| {
        st.disposals
            .iter()
            .find(|d| d.event == EventId::import(Source::Coinbase, SourceRef::new("D")))
            .unwrap()
            .legs[0]
            .clone()
    };

    // FIFO (no election): acquisition-date FIFO consumes the OLDER relocated Z' — basis $40, LT (2025-01-01→2026-02-01).
    let l = leg0(&scenario(vec![]));
    assert_eq!(
        l.basis,
        dec!(40.00),
        "legacy insertion-order FIFO would have wrongly picked A ($80)"
    );
    assert_eq!(l.term, Term::LongTerm);
    // LIFO: newest acquisition first -> A ($80), ST (2025-08-01→2026-02-01).
    let l = leg0(&scenario(vec![election(
        2,
        datetime!(2025-10-01 00:00:00 UTC),
        date!(2025 - 10 - 01),
        LotMethod::Lifo,
    )]));
    assert_eq!(l.basis, dec!(80.00));
    assert_eq!(l.term, Term::ShortTerm);
    // HIFO: highest gain-basis/sat first -> A ($80 > $40), ST.
    let l = leg0(&scenario(vec![election(
        2,
        datetime!(2025-10-01 00:00:00 UTC),
        date!(2025 - 10 - 01),
        LotMethod::Hifo,
    )]));
    assert_eq!(l.basis, dec!(80.00));
    assert_eq!(l.term, Term::ShortTerm);
}
