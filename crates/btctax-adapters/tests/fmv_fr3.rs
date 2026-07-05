use btctax_adapters::ingest_files;
use btctax_adapters::price::BundledPrices;
use btctax_core::{EventPayload, FmvStatus};
use rust_decimal_macros::dec;

// River rows (REAL §9.1 headers, invented values): Interest on a dataset date (→ PriceDataset),
// Interest on a NON-dataset date (→ Missing), Buy (no FMV needed). We inject a CONTROLLED synthetic
// price set (2025-06-15 @ 67500.00, and NOTHING on 2025-07-04) via `ingest_files` [R0-C1] so the FR3
// PriceDataset/Missing matrix is independent of the shipped daily-close dataset (which now covers BOTH
// 2025-06-15 and 2025-07-04 with real closes — the old `ingest_files_bundled` path would resolve BOTH).
// FR3 income events can only produce two statuses through the income path:
// - PriceDataset: when the date exists in the bundled price dataset (e.g., 2025-06-15)
// - Missing: when the date does not exist in the dataset (e.g., 2025-07-04)
// ExchangeProvided is not produced by any Phase-1 adapter (all income paths pass None to resolve_fmv).
const CSV: &str = "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-07-04 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-03-01 12:00:00,4200.00,USD,0.05000000,BTC,3.00,USD,Buy\r\n";

#[test]
fn fr3_matrix_through_full_pipeline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("river.csv");
    std::fs::write(&path, CSV).unwrap();
    // Synthetic controlled prices: 2025-06-15 present (→ PriceDataset), 2025-07-04 absent (→ Missing).
    let prices = BundledPrices::from_csv_str("date,usd_close\n2025-06-15,67500.00\n").unwrap();
    let batch = ingest_files(&[path], &prices).unwrap();

    let income_events: Vec<_> = batch
        .events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Income(i) => Some(i.clone()),
            _ => None,
        })
        .collect();

    // Verify that exactly 2 income events are produced (one PriceDataset, one Missing).
    assert_eq!(income_events.len(), 2);

    // Verify PriceDataset status count (exactly 1 event with PriceDataset FMV).
    let price_dataset_count = income_events
        .iter()
        .filter(|i| i.fmv_status == FmvStatus::PriceDataset)
        .count();
    assert_eq!(price_dataset_count, 1);

    // Verify Missing status count (exactly 1 event with Missing FMV).
    let missing_count = income_events
        .iter()
        .filter(|i| i.fmv_status == FmvStatus::Missing)
        .count();
    assert_eq!(missing_count, 1);

    // Pin the PriceDataset income event's resolved FMV value:
    // Date 2025-06-15, price 67500.00/BTC, quantity 10,000 sat
    // FMV = (10,000 sat / 1e8) * 67500 = 6.75
    let price_dataset_event = income_events
        .iter()
        .find(|i| i.fmv_status == FmvStatus::PriceDataset)
        .expect("PriceDataset income event");
    assert_eq!(
        price_dataset_event.usd_fmv,
        Some(dec!(6.75)),
        "PriceDataset income FMV should be resolved to 6.75"
    );

    // Verify the Missing income event carries no fabricated FMV (usd_fmv is None).
    let missing_event = income_events
        .iter()
        .find(|i| i.fmv_status == FmvStatus::Missing)
        .expect("Missing income event");
    assert_eq!(
        missing_event.usd_fmv, None,
        "Missing income event should have no FMV value"
    );

    // Both income events must preserve their sat quantities.
    for event in &income_events {
        assert_eq!(
            event.sat, 10_000,
            "Each income event should have 10,000 sat"
        );
    }
}
