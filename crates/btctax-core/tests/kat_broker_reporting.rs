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
