//! spec 1099-DA T1 — the Form 1099-DA answers on `ReturnInputs`: their own census class, never
//! auto-answered, and absent-in-storage means nothing answered.

use btctax_core::forms::{BrokerReported, Cohort, CohortAnswers};
use btctax_core::tax::classifier::classify;
use btctax_core::tax::return_inputs::ReturnInputs;

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
