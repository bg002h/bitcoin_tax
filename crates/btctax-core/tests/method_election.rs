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

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// FR-34 — THE DEFAULTED IDENTIFICATION. Reg. §1.1012-1(j)(1) / (j)(3)(i).
//
// The reg, from the archived text layer
// (`legal/primary-sources/regulations-cfr/26CFR_1.1012-1_basis.xml`, sha256 matching
// `legal/SHA256SUMS` line 21):
//
//   (j)(1)     "If a specific identification is not made, the basis and holding period of the units
//               sold, disposed of, or transferred are determined by treating the units not held in
//               the custody of a broker as sold, disposed of, or transferred in order of time from
//               the earliest date on which units of the same digital asset not held in the custody
//               of a broker were acquired by the taxpayer."
//   (j)(3)(i)  the same deemed acquisition order for broker-custodied units where "the taxpayer does
//               not provide the broker with an adequate identification".
//   (j)(3)(ii) "A standing order or instruction for the specific identification of digital assets is
//               treated as an adequate identification made at the time of sale, disposition, or
//               transfer."
//   (j)(6)     paragraph (j) applies to dispositions on or after 2025-01-01 (= `TRANSITION_DATE`).
//
// btctax's no-election default is HIFO. That is an OWNER MANDATE
// (`design/SPEC_reconcile_defaults.md` Change 1; FOLLOWUPS.md §reconcile-defaults), pinned by
// `default_method_is_hifo` above, and FR-34 does NOT touch it. What FR-34 fixes is that the default
// was applied SILENTLY: the engine's own §A.5 verdict called such a disposal `NonCompliant` and that
// row reached only `verify` — it changed no filed number and gated no filed artifact. The pre-2025
// half of the identical question has had `Pre2025MethodNote` since §7.4; the post-2025 half — the
// only half paragraph (j) actually governs — had nothing.
//
// A standing order IS an adequate identification, so the escape is real and one command wide:
// `btctax config --set-forward-method <m>`. It cannot be back-dated (`MethodElectionBackdated`),
// which is why this blocker is ADVISORY and not Hard: a filer whose PAST sales rest on the default
// has no clearing action for them, and an unclearable Hard gate would refuse their return forever.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn lot_selection(
    seq: u64,
    ts: time::OffsetDateTime,
    disposal_ref: &str,
    picks: Vec<LotPick>,
) -> LedgerEvent {
    dec_ev(
        seq,
        ts,
        EventPayload::LotSelection(LotSelection {
            disposal_event: EventId::import(Source::Coinbase, SourceRef::new(disposal_ref)),
            lots: picks,
            attested: false,
        }),
    )
}
fn pid(rf: &str) -> LotId {
    LotId {
        origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        split_sequence: 0,
    }
}
fn detail_of(st: &LedgerState, k: BlockerKind) -> String {
    st.blockers
        .iter()
        .find(|b| b.kind == k)
        .unwrap_or_else(|| panic!("no {k:?} blocker in {:?}", st.blockers))
        .detail
        .clone()
}

/// ★ FR-34 REPRO — the understatement on the DEFAULT path, and the silence about it.
///
/// Three lots, no `MethodElection`, no `LotSelection`: the filer identified NOTHING. btctax applies
/// its HIFO default and draws lot B ($90) against a $95 sale — gain $5. §1.1012-1(j)(1) treats
/// unidentified units as sold in acquisition order, which draws lot A ($50) — gain $45. **$40 of
/// gain is not on this return**, and before FR-34 nothing in the projection said so.
#[test]
fn a_disposal_identified_by_nothing_at_all_is_disclosed_with_its_dollar_cost() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());

    // The gap, MEASURED off the projection — not quoted from a doc comment.
    assert_eq!(
        st.disposals[0].legs[0].basis,
        dec!(90.00),
        "the owner-mandated HIFO default draws lot B — FR-34 does not change this (see \
         `default_method_is_hifo`)"
    );
    assert_eq!(
        st.disposals[0].legs[0].gain,
        dec!(5.00),
        "$95 proceeds − $90 basis; §1.1012-1(j)(1)'s deemed acquisition order (lot A, $50) gives $45"
    );

    // THE DEFECT: the engine knows this disposal rests on nothing, and says nothing.
    assert!(
        has(&st, BlockerKind::IdentificationDefaulted),
        "a filed basis chosen by btctax's default, on a disposal the filer identified in no way at \
         all, MUST be disclosed — this is the §1.1012-1(j) verdict finally reaching the number"
    );
    let d = detail_of(&st, BlockerKind::IdentificationDefaulted);
    for want in [
        "90.00",
        "50.00",
        "40.00",
        "1.1012-1(j)",
        "--set-forward-method",
    ] {
        assert!(
            d.contains(want),
            "the advisory must NAME the consequence, not describe it — missing {want:?} in: {d}"
        );
    }
}

/// FR-34 — §1.1012-1(j)(3)(ii): a standing order IS an adequate identification. An explicit HIFO
/// election lands on the IDENTICAL $90 basis as the silent default — and that is exactly why the
/// advisory has to exist. Two returns that print the same number are not the same return: one rests
/// on the filer's standing order, the other on btctax answering for them.
#[test]
fn an_in_force_election_is_an_identification_and_raises_no_advisory() {
    let mut evs = three_post2025();
    evs.push(election(
        1,
        datetime!(2025-01-02 00:00:00 UTC),
        date!(2025 - 01 - 02),
        LotMethod::Hifo,
    ));
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
        "same number as the default path"
    );
    assert!(
        !has(&st, BlockerKind::IdentificationDefaulted),
        "a standing order is an adequate identification (§1.1012-1(j)(3)(ii)) — nothing to disclose"
    );
}

/// FR-34 — a contemporaneous `LotSelection` (§1.1012-1(j)(2)) is an identification too. Recorded on
/// the day of the sale, naming lot C, it governs and raises no advisory — even though no election
/// exists, so the METHOD would otherwise have come from the default.
#[test]
fn a_contemporaneous_selection_is_an_identification_and_raises_no_advisory() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    evs.push(lot_selection(
        1,
        datetime!(2025-07-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("C"),
            sat: 100_000,
        }],
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        st.disposals[0].legs[0].basis,
        dec!(40.00),
        "the named lot C governs"
    );
    assert!(
        !has(&st, BlockerKind::IdentificationDefaulted),
        "the filer identified these units on the date of the sale — no default was used"
    );
}

/// FR-34 — §1.1012-1(j)(6) scopes paragraph (j) to dispositions on or after 2025-01-01. A pre-2025
/// disposal draws from the Universal pool under `config.pre2025_method`, which is
/// `Pre2025MethodNote`'s territory (§7.4) and has its own attestation surface. It must NOT also
/// raise this advisory — two notes for one assumption is noise, and the reg does not reach it.
///
/// ★ TWO lots at different prices, for the same reason the self-transfer scope test has them: on a
/// single-lot pool the exposure suppression keeps this green whatever the date guard says, and the
/// guard would be asserted only by the walkthrough goldens. (Found by mutation: deleting
/// `date >= TRANSITION_DATE` left every targeted test green and reddened only goldens.)
#[test]
fn a_pre_2025_disposal_raises_no_identification_advisory() {
    let evs = vec![
        buy(
            "A",
            datetime!(2024-01-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        buy(
            "B",
            datetime!(2024-02-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
        sell(
            "D",
            datetime!(2024-06-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(
        has(&st, BlockerKind::Pre2025MethodNote),
        "§7.4's own note still owns the pre-2025 period"
    );
    assert!(
        !has(&st, BlockerKind::IdentificationDefaulted),
        "§1.1012-1(j)(6) does not reach a 2024 disposition"
    );
}

/// FR-34 — ADVISORY, deliberately, and this test is the reason. An election cannot be back-dated
/// (`MethodElectionBackdated`: "a standing order cannot be back-dated"), so a filer whose PAST sales
/// rest on the default has NO action that clears them. A Hard gate would therefore be unclearable —
/// it would refuse every such return forever, which is worse than telling the filer nothing. The
/// year must still compute, with the disclosure attached.
#[test]
fn the_identification_advisory_never_gates_the_year() {
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
    assert_eq!(
        BlockerKind::IdentificationDefaulted.severity(),
        Severity::Advisory,
        "Hard would be unclearable for past sales — an election cannot be back-dated"
    );
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(has(&st, BlockerKind::IdentificationDefaulted));

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
    let tables = OneTable(TaxTable {
        year: 2025,
        source: "SYNTHETIC",
        ordinary,
        ltcg,
        gift_annual_exclusion: dec!(19000),
        ss_wage_base: dec!(176100),
        gift_lifetime_exclusion: dec!(13_990_000),
    });
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
    assert!(
        matches!(
            compute_tax_year(&evs, &st, 2025, Some(&prof), &tables),
            TaxOutcome::Computed(..)
        ),
        "IdentificationDefaulted must never make a year NotComputable"
    );
}

/// FR-34 SCOPE — a self-transfer between the filer's own wallets recognizes no gain, so no
/// §1.1012-1(j) identification obligation attaches at the move itself. This is the SAME boundary
/// `disposal_compliance` draws (it iterates only `disposals`/`removals`) and the one FR-33 kept.
///
/// ★ The source wallet deliberately holds TWO lots at different prices, and only one lot's worth
/// moves — so the HIFO default and the deemed acquisition order pick DIFFERENT units here. Without
/// that, the exposure suppression would keep this test green on a single-lot pool no matter what the
/// scope argument said, and the scope guard would be asserted by nothing. (Found by mutation: with a
/// one-lot fixture, flipping this call site to `IdScope::TaxableDisposition` left the whole suite
/// green.)
#[test]
fn a_self_transfer_alone_raises_no_identification_advisory() {
    let cold = WalletId::SelfCustody {
        label: "cold".into(),
    };
    let evs = vec![
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("Z1")),
            utc_timestamp: datetime!(2025-01-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(cold.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(40.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        },
        LedgerEvent {
            id: EventId::import(Source::Coinbase, SourceRef::new("Z2")),
            utc_timestamp: datetime!(2025-02-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(cold.clone()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000,
                usd_cost: dec!(90.00),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        },
        LedgerEvent {
            id: EventId::import(Source::Swan, SourceRef::new("OUT")),
            utc_timestamp: datetime!(2025-09-01 00:00:00 UTC),
            original_tz: offset!(+00:00),
            wallet: Some(cold),
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
                in_event_or_wallet: TransferTarget::Wallet(w()),
            }),
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(
        !has(&st, BlockerKind::IdentificationDefaulted),
        "a non-taxable relocation identifies nothing because it need not — no gain is recognized"
    );
}

/// FR-34 SCOPE — a `GiftOut` IS a disposition the reg reaches ("sells, disposes of, or transfers")
/// and `disposal_compliance` emits a row for it, so the un-identified default must disclose there
/// too. This exercises a DIFFERENT `consume_principal` call site than the sale above — the taxable
/// sites are covered separately on purpose, because the scope argument is per-call-site.
#[test]
fn a_gift_out_with_no_identification_also_discloses() {
    let mut evs = three_post2025();
    evs.push(LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new("GOUT")),
        utc_timestamp: datetime!(2025-07-01 00:00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: Some(w()),
        payload: EventPayload::TransferOut(TransferOut {
            sat: 100_000,
            fee_sat: None,
            dest_addr: None,
            txid: None,
        }),
    });
    evs.push(dec_ev(
        1,
        datetime!(2025-07-02 00:00:00 UTC),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: EventId::import(Source::Coinbase, SourceRef::new("GOUT")),
            as_: OutflowClass::GiftOut,
            principal_proceeds_or_fmv: dec!(95.00),
            fee_usd: None,
            donee: None,
        }),
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert!(
        has(&st, BlockerKind::IdentificationDefaulted),
        "a gift out is a disposition under §1.1012-1(j); its lots were picked by the default too"
    );
}

/// FR-34 — the advisory fires on EXPOSURE, not on formality, and this pair is the reason.
///
/// §1.1012-1(j)(1) prescribes an OUTCOME — which units are treated as sold, and hence their basis
/// AND holding period — not a piece of paperwork. A pool holding ONE lot has nothing to choose: the
/// HIFO default and the deemed acquisition order take the identical unit, so the return already IS
/// what the reg prescribes and the filer has no exposure. Warning there would fire on nearly every
/// disposal in an un-elected vault and train the filer to ignore the one that matters.
///
/// The two halves share one fixture and differ only in whether a SECOND, cheaper lot exists — so
/// this is the same disposal, silent when nothing is at stake and loud the moment something is.
#[test]
fn the_advisory_is_silent_when_the_default_takes_the_very_units_the_reg_deems_sold() {
    let one_lot = vec![
        buy(
            "A",
            datetime!(2025-02-01 00:00:00 UTC),
            100_000,
            dec!(50.00),
        ),
        sell(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            100_000,
            dec!(95.00),
        ),
    ];
    let st = project(
        &one_lot,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        !has(&st, BlockerKind::IdentificationDefaulted),
        "one lot: HIFO and §1.1012-1(j)(1)'s acquisition order take the SAME unit, so this return is \
         already the reg's outcome — there is nothing to disclose"
    );

    // Add a dearer, later lot and nothing else. Now HIFO takes B ($90) where the deemed order takes
    // A ($50), and the same disposal must speak.
    let mut two_lots = one_lot.clone();
    two_lots.insert(
        1,
        buy(
            "B",
            datetime!(2025-03-01 00:00:00 UTC),
            100_000,
            dec!(90.00),
        ),
    );
    let st = project(
        &two_lots,
        &StaticPrices::default(),
        &ProjectionConfig::default(),
    );
    assert!(
        has(&st, BlockerKind::IdentificationDefaulted),
        "the moment the default and the deemed order diverge, the assumption costs money and must be \
         disclosed"
    );
}

/// FR-34 — ORDER is not divergence. When a sale drains a pool, the deemed acquisition order returns
/// the very same fragments the HIFO default returns, just listed the other way round:
/// FIFO `[A($50), B($90)]` versus HIFO `[B($90), A($50)]`. The legs, their bases and their terms are
/// identical, so the tax is identical and §1.1012-1(j)(1) has nothing to say. A comparison that
/// zipped the two lists pairwise would fire here — on a disposal with no exposure whatsoever.
#[test]
fn the_advisory_is_silent_when_the_deemed_order_takes_the_same_units_in_a_different_order() {
    let evs = vec![
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
        // Drains the pool: BOTH lots are consumed under either order.
        sell(
            "D",
            datetime!(2025-07-01 00:00:00 UTC),
            200_000,
            dec!(300.00),
        ),
    ];
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        st.disposals[0].legs.len(),
        2,
        "both lots must be consumed, or this fixture is not testing ordering"
    );
    assert!(
        !has(&st, BlockerKind::IdentificationDefaulted),
        "the same units in a different sequence are the SAME disposal — basis, term and tax are \
         untouched, so §1.1012-1(j)(1) changes nothing and there is nothing to disclose"
    );
}

/// FR-33 ✕ FR-34 SEAM — the two fixes compose, and BOTH advisories must speak.
///
/// A late, unattested `LotSelection` is dropped by FR-33's §A.4 timeliness pass, and the disposal
/// falls back to the method in force. When no election is in force either, that fallback is the HIFO
/// DEFAULT — so after FR-33 has removed the cherry-pick the disposal still rests on nothing the filer
/// identified, which is exactly what FR-34 discloses. FR-33's own report says so in as many words:
/// *"after the drop a no-election disposal falls back to HIFO (fold.rs:49) where (j)(1) says
/// acquisition order. FR-33 removes the cherry-pick; FR-34 owns the default."*
///
/// Each advisory names its own cause; neither suppresses the other.
#[test]
fn a_dropped_post_hoc_selection_with_no_election_raises_both_advisories() {
    let mut evs = three_post2025();
    evs.push(sell(
        "D",
        datetime!(2025-07-01 00:00:00 UTC),
        100_000,
        dec!(95.00),
    ));
    // Recorded 2026-04-01 — nine months after the sale, and unattested: FR-33 drops it.
    evs.push(lot_selection(
        1,
        datetime!(2026-04-01 00:00:00 UTC),
        "D",
        vec![LotPick {
            lot: pid("C"),
            sat: 100_000,
        }],
    ));
    let st = project(&evs, &StaticPrices::default(), &ProjectionConfig::default());
    assert_eq!(
        st.disposals[0].legs[0].basis,
        dec!(90.00),
        "FR-33 drops the late selection; with no election in force the fallback is the HIFO default"
    );
    assert!(
        has(&st, BlockerKind::LotSelectionPostHoc),
        "FR-33 must still disclose that a selection was dropped"
    );
    assert!(
        has(&st, BlockerKind::IdentificationDefaulted),
        "and FR-34 must disclose that what remains is btctax's default, identified by nothing — the \
         drop does not make the disposal identified"
    );
}
