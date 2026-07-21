//! Phase 7 / Task 13 — the MANDATORY conservative-filing methodology disclosure (D-4) is written
//! (`basis_methodology.txt`) alongside the year's form CSVs whenever a tranche is in the filed set, and
//! is ABSENT for a fully-documented year. PRIVACY: synthetic values only.
use btctax_cli::render::write_form_csvs;
use btctax_core::event::*;
use btctax_core::identity::*;
use btctax_core::price::StaticPrices;
use btctax_core::project::{project, ProjectionConfig};
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::{date, datetime, offset};

fn sc() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn prices() -> StaticPrices {
    StaticPrices::default()
}
fn cfg() -> ProjectionConfig {
    ProjectionConfig::default()
}
fn tranche(seq: u64, w: &WalletId, sat: i64, ws: time::Date, we: time::Date) -> LedgerEvent {
    LedgerEvent {
        id: EventId::decision(seq),
        utc_timestamp: datetime!(2026-01-01 00:00 UTC),
        original_tz: offset!(+00:00),
        wallet: None,
        payload: EventPayload::DeclareTranche(DeclareTranche {
            sat,
            wallet: w.clone(),
            window_start: ws,
            window_end: we,
        }),
    }
}
fn sell(rf: &str, ts: time::OffsetDateTime, w: &WalletId, sat: i64, proceeds: i64) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: EventPayload::Dispose(Dispose {
            sat,
            usd_proceeds: rust_decimal::Decimal::from(proceeds),
            fee_usd: dec!(0),
            kind: DisposeKind::Sell,
        }),
    }
}
fn buy(rf: &str, ts: time::OffsetDateTime, w: &WalletId, sat: i64, cost: i64) -> LedgerEvent {
    LedgerEvent {
        id: EventId::import(Source::Coinbase, SourceRef::new(rf)),
        utc_timestamp: ts,
        original_tz: offset!(+00:00),
        wallet: Some(w.clone()),
        payload: EventPayload::Acquire(Acquire {
            sat,
            usd_cost: rust_decimal::Decimal::from(cost),
            fee_usd: dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        }),
    }
}

/// A tranche filed in `year` ⇒ `basis_methodology.txt` is written alongside the form CSVs, and carries
/// the disclosure (provenance-neutral, term-derived).
#[test]
fn basis_methodology_txt_is_written_when_a_tranche_is_filed() {
    let w = sc();
    let evs = vec![
        tranche(
            1,
            &w,
            100_000_000,
            date!(2015 - 01 - 01),
            date!(2015 - 12 - 31),
        ),
        sell(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            90_000,
        ),
    ];
    let st = project(&evs, &prices(), &cfg());
    let dir = tempfile::tempdir().unwrap();
    let empty: BTreeMap<EventId, btctax_core::DonationDetails> = BTreeMap::new();
    write_form_csvs(dir.path(), &st, 2026, None, &empty).unwrap();

    let path = dir.path().join("basis_methodology.txt");
    assert!(
        path.exists(),
        "basis_methodology.txt must be written when a tranche is filed (D-4)"
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("Basis methodology disclosure"),
        "the disclosure header must be present: {text}"
    );
    assert!(
        text.contains("2015-12-31") && text.contains("long-term"),
        "the filed tranche is enumerated (window_end + derived term): {text}"
    );
    let low = text.to_lowercase();
    assert!(
        !low.contains("purchase") && !low.contains("bought"),
        "provenance-neutral: {text}"
    );
}

/// A fully-documented year writes NO `basis_methodology.txt` (the i8949 basis explanation is only
/// required when actual cost is not used).
#[test]
fn basis_methodology_txt_absent_for_a_fully_documented_year() {
    let w = sc();
    let evs = vec![
        buy(
            "BUY",
            datetime!(2025-06-01 00:00 UTC),
            &w,
            100_000_000,
            30_000,
        ),
        sell(
            "SELL",
            datetime!(2026-06-01 00:00 UTC),
            &w,
            100_000_000,
            90_000,
        ),
    ];
    let st = project(&evs, &prices(), &cfg());
    let dir = tempfile::tempdir().unwrap();
    let empty: BTreeMap<EventId, btctax_core::DonationDetails> = BTreeMap::new();
    write_form_csvs(dir.path(), &st, 2026, None, &empty).unwrap();

    assert!(
        !dir.path().join("basis_methodology.txt").exists(),
        "no tranche filed ⇒ no disclosure file"
    );
    // sanity: the form CSVs themselves WERE written.
    assert!(
        dir.path().join("form8949.csv").exists(),
        "the form CSVs are still written"
    );
}
