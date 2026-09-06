//! spec 1099-DA (build review I-3): the Form 8949 CSV's `box` column is an emitted box — on a live
//! year it is routed from the stored answers, or the export refuses BEFORE any byte.

use btctax_core::event::*;
use btctax_core::forms::{BrokerReported, BrokerReporting, Cohort, CohortAnswers};
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use btctax_core::InformationReturnRegime;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{datetime, offset};

fn ev(rf: &str, ts: time::OffsetDateTime, p: EventPayload) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(WalletId::Exchange {
            provider: "coinbase".into(),
            account: "default".into(),
        }),
        payload: p,
    }
}

fn live_state() -> btctax_core::state::LedgerState {
    let evs = vec![
        ev(
            "b",
            datetime!(2026-02-01 12:00 UTC),
            EventPayload::Acquire(Acquire {
                sat: 1_000_000,
                usd_cost: dec!(900),
                fee_usd: dec!(0),
                basis_source: BasisSource::ExchangeProvided,
            }),
        ),
        ev(
            "s",
            datetime!(2026-06-15 12:00 UTC),
            EventPayload::Dispose(Dispose {
                sat: 1_000_000,
                usd_proceeds: dec!(1200),
                fee_usd: dec!(0),
                kind: DisposeKind::Sell,
            }),
        ),
    ];
    project(&evs, &StaticPrices::default(), &ProjectionConfig::default())
}

#[test]
fn the_8949_csv_is_gated_and_routed_on_a_live_year() {
    let st = live_state();
    let live = InformationReturnRegime::PROCEEDS_AND_BASIS;
    // no answers → refuse before any byte
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().join("snap");
    let err = btctax_cli::render::write_csv_exports(
        &dir,
        &st,
        Some(2026),
        None,
        &BTreeMap::new(),
        Some((live, None)),
    )
    .unwrap_err();
    assert!(err.to_string().contains("Form 1099-DA answers"), "{err}");
    assert!(!dir.exists(), "a refusal writes NO bytes");
    // answers → routed: the CSV's box column reads G, not I
    let mut answers = BrokerReporting::default();
    answers.0.insert(
        "coinbase".into(),
        CohortAnswers {
            covered: Some(BrokerReported::BasisMatches),
            noncovered: None,
        },
    );
    btctax_cli::render::write_csv_exports(
        &dir,
        &st,
        Some(2026),
        None,
        &BTreeMap::new(),
        Some((live, Some(&answers))),
    )
    .unwrap();
    let csv = std::fs::read_to_string(dir.join("form8949.csv")).unwrap();
    let row = csv.lines().nth(1).expect("one row");
    assert!(
        row.starts_with("short,G,") || row.contains(",G,"),
        "routed box in the CSV: {row}"
    );
    // not live (TY2025): the rows as built, no answers needed
    let dir2 = out.path().join("snap-2025");
    btctax_cli::render::write_csv_exports(
        &dir2,
        &live_state(),
        Some(2026),
        None,
        &BTreeMap::new(),
        Some((InformationReturnRegime::PROCEEDS_ONLY, None)),
    )
    .unwrap();
    let csv = std::fs::read_to_string(dir2.join("form8949.csv")).unwrap();
    assert!(
        csv.lines().nth(1).unwrap().contains(",I,"),
        "not live: I as built"
    );
    let _ = Cohort::Covered;
}
