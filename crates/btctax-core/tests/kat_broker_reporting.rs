//! spec 1099-DA T1 — the Form 1099-DA answers on `ReturnInputs`: their own census class, never
//! auto-answered, and absent-in-storage means nothing answered.

use btctax_core::event::{BasisSource, DisposeKind};
use btctax_core::forms::{
    broker_key, route_8949_boxes, BrokerReported, BrokerRouteError, Cohort, CohortAnswers,
    Form8949Box, InformationReturnRegime,
};
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, LedgerState, Term};
use btctax_core::tax::classifier::classify;
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::tax::return_refuse::{screen_broker_reporting, RefuseReason};
use btctax_core::TaxDate;
use rust_decimal_macros::dec;
use time::macros::date;

/// The answers are their own census class: every (provider, cohort, answer) the filer gave is
/// recorded as testimony, nothing is defaulted, and an absent slot is absent.
#[test]
fn broker_answers_are_their_own_census_class() {
    let mut ri = ReturnInputs::default();
    assert!(
        classify(&ri).broker_answers.is_empty(),
        "nothing answered ⇒ nothing recorded"
    );
    ri.broker_reporting.0.insert(
        "coinbase".into(),
        CohortAnswers {
            covered: Some(BrokerReported::BasisMatches),
            noncovered: None,
        },
    );
    ri.broker_reporting.0.insert(
        "gemini".into(),
        CohortAnswers {
            covered: None,
            noncovered: Some(BrokerReported::ProceedsOnly),
        },
    );
    assert_eq!(
        classify(&ri).broker_answers,
        vec![
            (
                "coinbase".to_string(),
                Cohort::Covered,
                BrokerReported::BasisMatches
            ),
            (
                "gemini".to_string(),
                Cohort::Noncovered,
                BrokerReported::ProceedsOnly
            ),
        ]
    );
}

/// (r1 I1) `testonly::answer_all_live_declarations` fills every live registry question with its
/// declared neutral; there is NO neutral answer to "what did your broker report", so the block must
/// come out exactly as it went in: empty.
#[test]
fn the_auto_answerer_never_touches_broker_reporting() {
    let mut ri = ReturnInputs {
        tax_year: 2026,
        ..Default::default()
    };
    btctax_core::tax::testonly::answer_all_live_declarations(&mut ri);
    assert!(
        ri.broker_reporting.0.is_empty(),
        "answer_all_live_declarations must not invent a 1099-DA answer: {:?}",
        ri.broker_reporting
    );
}

/// A stored `ReturnInputs` written before the field existed deserialises with NOTHING answered
/// (`#[serde(default)]`), so the screen refuses on a live year rather than the load failing or a
/// value being assumed.
#[test]
fn an_older_return_inputs_json_loads_with_nothing_answered() {
    let ri = ReturnInputs::default();
    let mut v: serde_json::Value = serde_json::to_value(&ri).unwrap();
    assert!(v
        .as_object_mut()
        .unwrap()
        .remove("broker_reporting")
        .is_some());
    let back: ReturnInputs = serde_json::from_value(v).unwrap();
    assert!(back.broker_reporting.0.is_empty());
    assert_eq!(back, ri);
}

// ── a small ledger: exchange and self-custody dispositions in one year ─────────────────────────────

fn exch(provider: &str) -> WalletId {
    WalletId::Exchange {
        provider: provider.into(),
        account: "default".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn lot(origin_seq: u64) -> LotId {
    LotId {
        origin_event_id: EventId::decision(origin_seq),
        split_sequence: 0,
    }
}
fn leg(
    seq: u64,
    wallet: WalletId,
    basis_source: BasisSource,
    bought: TaxDate,
    term: Term,
) -> DisposalLeg {
    DisposalLeg {
        lot_id: lot(seq),
        sat: 100_000,
        proceeds: dec!(1000),
        basis: dec!(400),
        gain: dec!(600),
        term,
        basis_source,
        gift_zone: None,
        acquired_at: bought,
        lot_acquired_at: bought,
        wallet,
        pseudo: false,
    }
}
fn sold_in(year: i32, legs: Vec<DisposalLeg>) -> Disposal {
    Disposal {
        event: EventId::decision(900 + year as u64),
        disposed_at: TaxDate::from_calendar_date(year, time::Month::June, 15).unwrap(),
        kind: DisposeKind::Sell,
        legs,
        fee_mini_disposition: false,
    }
}
fn ledger(disposals: Vec<Disposal>) -> LedgerState {
    LedgerState {
        disposals,
        ..LedgerState::default()
    }
}
/// One Coinbase lot bought on the venue in 2026 (Covered), one transferred in (Noncovered), one
/// self-custody sale; all sold in `year`.
fn owner_like(year: i32) -> LedgerState {
    ledger(vec![sold_in(
        year,
        vec![
            leg(
                1,
                exch("coinbase"),
                BasisSource::ExchangeProvided,
                date!(2026 - 02 - 01),
                Term::ShortTerm,
            ),
            leg(
                2,
                exch("coinbase"),
                BasisSource::SelfTransferInbound,
                date!(2021 - 05 - 05),
                Term::LongTerm,
            ),
            leg(
                3,
                cold(),
                BasisSource::ComputedFromCost,
                date!(2021 - 05 - 05),
                Term::LongTerm,
            ),
        ],
    )])
}
fn answers(entries: &[(&str, Cohort, BrokerReported)]) -> ReturnInputs {
    let mut ri = ReturnInputs::default();
    for (p, c, a) in entries {
        let e = ri.broker_reporting.0.entry((*p).to_string()).or_default();
        match c {
            Cohort::Covered => e.covered = Some(*a),
            Cohort::Noncovered => e.noncovered = Some(*a),
        }
    }
    ri
}
const LIVE: InformationReturnRegime = InformationReturnRegime::PROCEEDS_AND_BASIS;

// ── the screen (spec T1) ──────────────────────────────────────────────────────────────────────────

#[test]
fn a_live_year_with_an_unanswered_key_refuses_and_names_the_exit() {
    let st = owner_like(2026);
    let r =
        screen_broker_reporting(&ReturnInputs::default(), &st, 2026, LIVE).expect("must refuse");
    assert!(
        matches!(
            r.reason,
            RefuseReason::BrokerReportingUnanswered { ref provider, cohort: Cohort::Covered, year: 2026 } if provider == "coinbase"
        ),
        "{:?}",
        r.reason
    );
    assert!(
        r.detail.contains("income import")
            && r.detail.contains("[broker_reporting.coinbase] covered"),
        "{}",
        r.detail
    );
    let ri = answers(&[("coinbase", Cohort::Covered, BrokerReported::BasisMatches)]);
    let r = screen_broker_reporting(&ri, &st, 2026, LIVE).expect("the other key");
    assert!(matches!(
        r.reason,
        RefuseReason::BrokerReportingUnanswered {
            cohort: Cohort::Noncovered,
            ..
        }
    ));
    let ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::BasisMatches),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
    ]);
    assert!(screen_broker_reporting(&ri, &st, 2026, LIVE).is_none());
}

#[test]
fn the_question_is_not_asked_on_a_proceeds_only_or_pre_regime_year() {
    assert!(screen_broker_reporting(
        &ReturnInputs::default(),
        &owner_like(2025),
        2025,
        InformationReturnRegime::PROCEEDS_ONLY
    )
    .is_none());
    assert!(screen_broker_reporting(
        &ReturnInputs::default(),
        &owner_like(2024),
        2024,
        InformationReturnRegime::NONE
    )
    .is_none());
    let only_cold = ledger(vec![sold_in(
        2026,
        vec![leg(
            3,
            cold(),
            BasisSource::ComputedFromCost,
            date!(2021 - 05 - 05),
            Term::LongTerm,
        )],
    )]);
    assert!(screen_broker_reporting(&ReturnInputs::default(), &only_cold, 2026, LIVE).is_none());
}

#[test]
fn an_answer_nothing_would_read_refuses() {
    let ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::BasisMatches),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
        ("gemini", Cohort::Covered, BrokerReported::NotReported),
    ]);
    let r =
        screen_broker_reporting(&ri, &owner_like(2026), 2026, LIVE).expect("gemini has no rows");
    assert!(
        matches!(r.reason, RefuseReason::BrokerAnswerUnread { ref provider, .. } if provider == "gemini"),
        "{:?}",
        r.reason
    );
    let ri = answers(&[("coinbase", Cohort::Covered, BrokerReported::BasisMatches)]);
    let r = screen_broker_reporting(
        &ri,
        &owner_like(2025),
        2025,
        InformationReturnRegime::PROCEEDS_ONLY,
    )
    .expect("not live");
    assert!(matches!(
        r.reason,
        RefuseReason::BrokerAnswerUnread { year: 2025, .. }
    ));
    assert!(r.detail.contains("proceeds only"), "{}", r.detail);
    let only_cold = ledger(vec![sold_in(
        2026,
        vec![leg(
            3,
            cold(),
            BasisSource::ComputedFromCost,
            date!(2021 - 05 - 05),
            Term::LongTerm,
        )],
    )]);
    let r = screen_broker_reporting(&ri, &only_cold, 2026, LIVE).expect("no exchange disposition");
    assert!(matches!(r.reason, RefuseReason::BrokerAnswerUnread { .. }));
}

#[test]
fn mixed_and_basis_differs_refuse_naming_the_import() {
    let st = owner_like(2026);
    let ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::Mixed),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
    ]);
    let r = screen_broker_reporting(&ri, &st, 2026, LIVE).expect("Mixed refuses");
    assert!(matches!(
        r.reason,
        RefuseReason::BrokerReportingMixed {
            cohort: Cohort::Covered,
            ..
        }
    ));
    assert!(r.detail.contains("import"), "{}", r.detail);
    let ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::BasisDiffers),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
    ]);
    let r = screen_broker_reporting(&ri, &st, 2026, LIVE).expect("BasisDiffers refuses");
    assert!(matches!(
        r.reason,
        RefuseReason::BrokerBasisDiffers {
            cohort: Cohort::Covered,
            ..
        }
    ));
    assert!(
        r.detail.contains("column (e)") && r.detail.contains("(g)"),
        "{}",
        r.detail
    );
}

// ── the router (spec T2) ──────────────────────────────────────────────────────────────────────────

#[test]
fn every_cell_of_the_routing_table() {
    for (answer, st_box, lt_box) in [
        (BrokerReported::NotReported, Form8949Box::I, Form8949Box::L),
        (BrokerReported::ProceedsOnly, Form8949Box::H, Form8949Box::K),
        (BrokerReported::BasisMatches, Form8949Box::G, Form8949Box::J),
    ] {
        let st = ledger(vec![sold_in(
            2026,
            vec![
                leg(
                    1,
                    exch("coinbase"),
                    BasisSource::ExchangeProvided,
                    date!(2026 - 02 - 01),
                    Term::ShortTerm,
                ),
                leg(
                    2,
                    exch("coinbase"),
                    BasisSource::ExchangeProvided,
                    date!(2026 - 02 - 01),
                    Term::LongTerm,
                ),
            ],
        )]);
        let mut rows = btctax_core::form_8949(&st, 2026);
        let ri = answers(&[("coinbase", Cohort::Covered, answer)]);
        route_8949_boxes(&mut rows, LIVE, &ri.broker_reporting).unwrap();
        assert_eq!((rows[0].box_, rows[1].box_), (st_box, lt_box), "{answer:?}");
        // T5 (R4): no answer prints a code or an amount the tool did not collect — (f)/(g) stay
        // blank, never an automatic code B, never a 0
        for r in &rows {
            assert!(
                r.adjustment_code.is_empty(),
                "{answer:?}: (f) = {:?}",
                r.adjustment_code
            );
            assert!(
                r.adjustment_amount.is_zero(),
                "{answer:?}: (g) = {}",
                r.adjustment_amount
            );
        }
    }
    for answer in [BrokerReported::Mixed, BrokerReported::BasisDiffers] {
        let st = owner_like(2026);
        let mut rows = btctax_core::form_8949(&st, 2026);
        let ri = answers(&[
            ("coinbase", Cohort::Covered, answer),
            ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
        ]);
        let e = route_8949_boxes(&mut rows, LIVE, &ri.broker_reporting).unwrap_err();
        assert!(
            matches!(
                e,
                BrokerRouteError::Mixed { .. } | BrokerRouteError::BasisDiffers { .. }
            ),
            "{e:?}"
        );
    }
}

/// S4's kill: one provider, two cohorts, two different answers → two different boxes; the
/// self-custody row routes I/L by mechanism and needs no key (I-2).
#[test]
fn one_provider_two_cohorts_two_boxes_and_self_custody_by_mechanism() {
    let st = owner_like(2026);
    let mut rows = btctax_core::form_8949(&st, 2026);
    assert_eq!(
        broker_key(&rows[0]),
        Some(("coinbase".to_string(), Cohort::Covered))
    );
    assert_eq!(
        broker_key(&rows[1]),
        Some(("coinbase".to_string(), Cohort::Noncovered))
    );
    assert_eq!(broker_key(&rows[2]), None, "self-custody has no key");
    let ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::BasisMatches),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
    ]);
    route_8949_boxes(&mut rows, LIVE, &ri.broker_reporting).unwrap();
    assert_eq!(
        rows[0].box_,
        Form8949Box::G,
        "covered, basis matches, short-term"
    );
    assert_eq!(
        rows[1].box_,
        Form8949Box::K,
        "noncovered, proceeds only, long-term"
    );
    assert_eq!(
        rows[2].box_,
        Form8949Box::L,
        "self-custody: not reported, by mechanism"
    );
    let mut rows = btctax_core::form_8949(&st, 2026);
    let e =
        route_8949_boxes(&mut rows, LIVE, &ReturnInputs::default().broker_reporting).unwrap_err();
    assert!(
        matches!(
            e,
            BrokerRouteError::Unanswered {
                cohort: Cohort::Covered,
                ..
            }
        ),
        "{e:?}"
    );
    let mut rows = btctax_core::form_8949(&owner_like(2025), 2025);
    let before: Vec<Form8949Box> = rows.iter().map(|r| r.box_).collect();
    route_8949_boxes(
        &mut rows,
        InformationReturnRegime::PROCEEDS_ONLY,
        &ri.broker_reporting,
    )
    .unwrap();
    assert_eq!(rows.iter().map(|r| r.box_).collect::<Vec<_>>(), before);
    assert_eq!(before, [Form8949Box::I, Form8949Box::L, Form8949Box::L]);
}

// ── the WIRINGS (build review I-2): the gates are gates only if their placement is watched ─────────

/// `screen_absolute` runs the broker screen — FIRST — on a live year: an unanswered key refuses
/// through the real full-return path, not only through `screen_broker_reporting` called directly.
#[test]
fn screen_absolute_runs_the_broker_screen_on_a_live_year() {
    // ★ TY2025 with the LIVE regime handed in as the VALUE: the wiring is what is under test, and the
    //   assembly refuses TY2026 outright today (Form 6251's 2026 Part I is untranscribed by design).
    use btctax_core::tax::return_1040::{assemble_absolute, screen_absolute};
    use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
    let st = owner_like(2025);
    let ri = ReturnInputs {
        tax_year: 2025,
        ..Default::default()
    };
    let ar = assemble_absolute(&ri, &st, &ty2024_params(), &ty2024_table(), 2025);
    let r =
        screen_absolute(&ri, &ar, &ty2024_params(), &st, 2025, LIVE).expect("the wiring refuses");
    assert!(
        matches!(r.reason, RefuseReason::BrokerReportingUnanswered { .. }),
        "the broker screen is the first screen: {:?}",
        r.reason
    );
    // and NOT live: the same inputs pass this screen (whatever the other screens say, it is not this one)
    let r = screen_absolute(
        &ri,
        &ar,
        &ty2024_params(),
        &st,
        2026,
        InformationReturnRegime::NONE,
    );
    assert!(!matches!(
        r.map(|r| r.reason),
        Some(RefuseReason::BrokerReportingUnanswered { .. })
    ));
}

/// The printed packet ROUTES the boxes from the answers on a live year: the printed Form 8949's rows
/// carry G/J/K, not the map's I/L — through `assemble_printed_forms`, the function the packet uses.
#[test]
fn the_printed_packet_routes_the_boxes_on_a_live_year() {
    use btctax_core::tax::packet::assemble_printed_forms;
    use btctax_core::tax::return_1040::assemble_absolute;
    use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
    use std::collections::BTreeMap;
    let st = owner_like(2025);
    let mut ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::BasisMatches),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
    ]);
    ri.tax_year = 2025;
    let ar = assemble_absolute(&ri, &st, &ty2024_params(), &ty2024_table(), 2025);
    let printed = assemble_printed_forms(
        &ri,
        &st,
        &BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2025,
        &[],
        LIVE,
    );
    let f = printed.f8949.expect("a year with disposals prints an 8949");
    let boxes: Vec<Form8949Box> = f
        .short_term
        .iter()
        .chain(f.long_term.iter())
        .map(|r| r.box_)
        .collect();
    assert_eq!(
        boxes,
        [Form8949Box::G, Form8949Box::K, Form8949Box::L],
        "covered/basis → G; noncovered/proceeds → K; self-custody → L"
    );
}

// ── T4: Schedule D's per-box lines ────────────────────────────────────────────────────────────────

/// The G page-set's totals land on line 1b, NOT line 3; the K set on line 9; the self-custody L row
/// on line 10; and lines 7 and 15 combine the per-box lines as the form says ("Combine lines 1a
/// through 6" / "8a through 14").
#[test]
fn the_g_total_lands_on_line_1b_not_3() {
    use btctax_core::tax::packet::assemble_printed_forms;
    use btctax_core::tax::return_1040::assemble_absolute;
    use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
    use std::collections::BTreeMap;
    let st = owner_like(2025);
    let mut ri = answers(&[
        ("coinbase", Cohort::Covered, BrokerReported::BasisMatches),
        ("coinbase", Cohort::Noncovered, BrokerReported::ProceedsOnly),
    ]);
    ri.tax_year = 2025;
    let ar = assemble_absolute(&ri, &st, &ty2024_params(), &ty2024_table(), 2025);
    let printed = assemble_printed_forms(
        &ri,
        &st,
        &BTreeMap::new(),
        &ar,
        &ty2024_table(),
        2025,
        &[],
        LIVE,
    );
    let d = printed.sch_d;
    // every fixture leg: proceeds 1000, basis 400, gain 600
    assert_eq!(
        (d.line1b_d, d.line1b_e, d.line1b_h),
        (dec!(1000), dec!(400), dec!(600)),
        "G → 1b"
    );
    assert_eq!(
        (d.line2_d, d.line2_e, d.line2_h),
        (dec!(0), dec!(0), dec!(0)),
        "no H set"
    );
    assert_eq!(
        (d.line3_d, d.line3_e, d.line3_h),
        (dec!(0), dec!(0), dec!(0)),
        "no I set: line 3 is empty"
    );
    assert_eq!((d.line8b_d, d.line8b_h), (dec!(0), dec!(0)), "no J set");
    assert_eq!(
        (d.line9_d, d.line9_e, d.line9_h),
        (dec!(1000), dec!(400), dec!(600)),
        "K → 9"
    );
    assert_eq!(
        (d.line10_d, d.line10_e, d.line10_h),
        (dec!(1000), dec!(400), dec!(600)),
        "self-custody L → 10"
    );
    assert_eq!(
        d.line7,
        d.line1a_h + d.line1b_h + d.line2_h + d.line3_h - d.line6,
        "line 7 combines 1a–6"
    );
    assert_eq!(
        d.line15,
        d.line8a_h + d.line8b_h + d.line9_h + d.line10_h + d.line13 - d.line14,
        "line 15 combines 8a–14"
    );
    // the per-box totals on the printed 8949 agree with the lines they feed
    let f = printed.f8949.unwrap();
    assert_eq!(f.st_by_box[&Form8949Box::G].proceeds_d, d.line1b_d);
    assert_eq!(f.lt_by_box[&Form8949Box::K].proceeds_d, d.line9_d);
    assert_eq!(f.lt_by_box[&Form8949Box::L].proceeds_d, d.line10_d);
}

// ── T6: the surfaces ──────────────────────────────────────────────────────────────────────────────

/// The census `report` prints: every keyed row counted under its (provider, cohort), self-custody
/// rows carrying no key, and the names the surfaces print being the serde spellings.
#[test]
fn the_key_census_counts_every_keyed_row_and_no_self_custody_row() {
    use btctax_core::forms::{broker_key, broker_key_census};
    let st = owner_like(2026);
    let rows = btctax_core::form_8949(&st, 2026);
    let census = broker_key_census(&rows);
    let keyed = rows.iter().filter(|r| broker_key(r).is_some()).count();
    assert!(
        keyed > 0 && keyed < rows.len(),
        "the fixture mixes keyed and self-custody rows"
    );
    assert_eq!(census.values().sum::<usize>(), keyed);
    assert!(census.contains_key(&("coinbase".to_string(), Cohort::Covered)));
    assert!(census.contains_key(&("coinbase".to_string(), Cohort::Noncovered)));
    assert_eq!(Cohort::Covered.slot_name(), "covered");
    assert_eq!(BrokerReported::BasisMatches.toml_name(), "basis_matches");
    // the TOML spelling round-trips through serde, so what `report` prints is what `income import` reads
    for a in [
        BrokerReported::NotReported,
        BrokerReported::ProceedsOnly,
        BrokerReported::BasisMatches,
        BrokerReported::BasisDiffers,
        BrokerReported::Mixed,
    ] {
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, format!("\"{}\"", a.toml_name()));
    }
}
