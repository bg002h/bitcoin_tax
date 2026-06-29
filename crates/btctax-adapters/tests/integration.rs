use btctax_adapters::price::BundledPrices;
use btctax_adapters::{ingest_files, ingest_files_bundled, AdapterError};
use btctax_core::{EventPayload, Source};
use rust_xlsxwriter::Workbook;

#[test]
fn unrecognized_file_is_a_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mystery.csv");
    std::fs::write(&path, "foo,bar\n1,2\n").unwrap();
    let prices = BundledPrices::load().unwrap();
    let err = ingest_files(&[path], &prices).unwrap_err();
    assert!(matches!(err, AdapterError::UnknownSource { .. }));
}

fn write_gemini(path: &std::path::Path) {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let header = [
        "Date",
        "Time (UTC)",
        "Type",
        "Symbol",
        "BTC Amount BTC",
        "USD Amount USD",
        "Fee (USD) USD",
        "BTC Balance BTC",
        "Trade ID",
        "Order ID",
        "Tx Hash",
        "Deposit Destination",
        "Withdrawal Destination",
    ];
    for (c, h) in header.iter().enumerate() {
        ws.write_string(0, c as u16, *h).unwrap();
    }
    let rows: [[&str; 13]; 2] = [
        [
            "2025-03-01 12:00:00",
            "2025-03-01 12:00:00",
            "Buy",
            "BTCUSD",
            "0.02000000",
            "1680.00",
            "5.00",
            "0.02",
            "GT-1",
            "GO-1",
            "",
            "",
            "",
        ],
        [
            "2025-03-02 11:00:00",
            "2025-03-02 11:00:00",
            "Credit",
            "BTC",
            "0.00100000",
            "",
            "",
            "0.021",
            "",
            "",
            "feedface",
            "bc1qdp",
            "",
        ],
    ];
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            ws.write_string((r + 1) as u32, c as u16, *v).unwrap();
        }
    }
    wb.save(path).unwrap();
}

#[test]
fn multi_source_batch_ingests_into_events() {
    let dir = tempfile::tempdir().unwrap();

    // Coinbase: REAL 13-col header, 3-line preamble; cb-1 BTC Buy + cb-2 ETH (dropped).
    let cb = dir.path().join("coinbase.csv");
    std::fs::write(&cb, "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-1,2025-03-01 12:00:00 UTC,Buy,BTC,0.01000000,USD,84000.00,840.00,845.00,5.00,,,\r\n\
cb-2,2025-03-01 08:00:00 UTC,Buy,ETH,1.00000000,USD,2000.00,2000.00,2010.00,10.00,,,\r\n").unwrap();

    let gm = dir.path().join("gemini.xlsx");
    write_gemini(&gm);

    let rv = dir.path().join("river.csv");
    std::fs::write(&rv, "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n").unwrap();

    // Swan 3-file batch (real per-role headers; transfers/withdrawals carry the 2-line preamble).
    let st = dir.path().join("swan_trades.csv");
    std::fs::write(&st, "Date,Received Quantity,Received Currency,Sent Quantity,Sent Currency,Fee Amount,Fee Currency,Tag\n\
03/01/2025 12:00:00,0.10000000,BTC,8400.00,USD,40.00,USD,\n").unwrap();
    let sx = dir.path().join("swan_transfers.csv");
    std::fs::write(&sx, "Swan Bitcoin Inc\n123 Main St\n\
Event,Date,Timezone,Status,Transaction ID,Total USD,Transaction USD,Fee USD,Unit Count,Asset Type,BTC Price,Address Label,USD Cost Basis,Acquisition Date\n\
deposit,2025-03-02 09:00:00+00,UTC,settled,sw-x1,3000.00,3000.00,0,0.05000000,BTC,60000.00,cold,3000.00,2024-01-15\n").unwrap();
    let sw = dir.path().join("swan_withdrawals.csv");
    std::fs::write(&sw, "Swan Bitcoin Inc\n123 Main St\n\
Created At,Timezone,Transaction ID,Executed At,Canceled At,Status,Bitcoin Amount,Automatic,IP Address\n\
2025-03-03 10:00:00+00,UTC,sw-w1,2025-03-03 10:05:00+00,,settled,0.02000000,true,1.2.3.4\n").unwrap();

    let batch = ingest_files_bundled(&[cb, gm, rv, st, sx, sw]).unwrap();

    // One report per group: Coinbase(1) + Gemini(1) + River(1) + Swan(1 batch) = 4 reports.
    assert_eq!(batch.reports.len(), 4);
    let swan_report = batch
        .reports
        .iter()
        .find(|r| r.source == Source::Swan)
        .unwrap();
    assert_eq!(swan_report.btc_events, 3); // trades+transfers+withdrawals merged into one report
    let cb_report = batch
        .reports
        .iter()
        .find(|r| r.source == Source::Coinbase)
        .unwrap();
    assert_eq!(cb_report.dropped_no_btc, 1); // ETH row

    // Events: CB buy(1) + Gemini buy(1)+credit-TransferIn(1) + River interest(1) + Swan(3) = 7.
    assert_eq!(batch.events.len(), 7);
    // N-4: assert the absence of Unclassified with a diagnostic message naming which event IDs
    // failed, so a mis-routed event type (e.g. Gemini Credit → Unclassified instead of TransferIn,
    // or a Coinbase/Swan arm silently falling to the catch-all) is immediately identifiable.
    // This fixture has no Order/Exchange/Pro Coinbase rows, no Gemini unknown types, and no Swan
    // fee rows — so zero Unclassified is the correct tight assertion.
    let unclassified_ids: Vec<_> = batch
        .events
        .iter()
        .filter(|e| matches!(&e.payload, EventPayload::Unclassified(_)))
        .map(|e| e.id.canonical())
        .collect();
    assert!(
        unclassified_ids.is_empty(),
        "unexpected Unclassified events ({} found): {:?}",
        unclassified_ids.len(),
        unclassified_ids
    );
    // Two TransferIns: Gemini Credit(BTC) + Swan deposit.
    assert_eq!(
        batch
            .events
            .iter()
            .filter(|e| matches!(&e.payload, EventPayload::TransferIn(_)))
            .count(),
        2
    );
    assert!(batch
        .events
        .iter()
        .any(|e| matches!(&e.payload, EventPayload::Income(_)))); // River interest

    // Stable, source-scoped EventIds across all four venues coexist.
    assert!(batch
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|coinbase|trade|cb-1"));
    assert!(batch
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|gemini|trade|GT-1.GO-1"));
    assert!(batch
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|swan|in|sw-x1"));
    assert!(batch
        .events
        .iter()
        .any(|e| e.id.canonical().starts_with("import|river|in|")));
}
