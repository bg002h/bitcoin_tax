use btctax_adapters::ingest_files_bundled;
use btctax_core::{EventPayload, FmvStatus};

// River rows (REAL §9.1 headers, invented values): Interest on a dataset date (→ PriceDataset),
// Interest on a NON-dataset date (→ Missing), Buy (no FMV needed). Bundled dataset has 2025-06-15 but
// NOT 2025-07-04.
const CSV: &str = "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-07-04 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-03-01 12:00:00,4200.00,USD,0.05000000,BTC,3.00,USD,Buy\r\n";

#[test]
fn fr3_matrix_through_full_pipeline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("river.csv");
    std::fs::write(&path, CSV).unwrap();
    let batch = ingest_files_bundled(&[path]).unwrap();

    let statuses: Vec<FmvStatus> = batch
        .events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Income(i) => Some(i.fmv_status.clone()),
            _ => None,
        })
        .collect();
    assert!(statuses.contains(&FmvStatus::PriceDataset)); // 2025-06-15 present
    assert!(statuses.contains(&FmvStatus::Missing)); // 2025-07-04 absent → Missing blocker
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == FmvStatus::PriceDataset)
            .count(),
        1
    );
    // A Missing income still produces the sat-bearing event (never dropped); core gates its amount.
    assert_eq!(
        batch
            .events
            .iter()
            .filter(|e| matches!(&e.payload, EventPayload::Income(_)))
            .count(),
        2
    );
}
