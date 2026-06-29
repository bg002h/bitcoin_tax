use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::coinbase::Coinbase;
use btctax_core::{BasisSource, DisposeKind, EventPayload, Source};

// SYNTHETIC Coinbase export: the REAL §9.1 header names (13 cols), INVENTED values. 3-line preamble
// (empty / `Transactions` / user-identity), header on line 4, CRLF. Rows exercise the confirmed
// `Transaction Type` enum: Buy, Sell, Send, Receive, a non-BTC (ETH) row, an Order (→ Unclassified),
// and an Exchange Deposit (internal Coinbase↔Pro move → Unclassified). Real timestamp format `… UTC`.
const CSV: &str = "\
\r\n\
Transactions\r\n\
User,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-1,2025-03-01 12:00:00 UTC,Buy,BTC,0.01000000,USD,84000.00,840.00,845.00,5.00,,,\r\n\
cb-2,2025-03-02 09:00:00 UTC,Sell,BTC,0.00500000,USD,84250.00,421.25,419.25,2.00,,,\r\n\
cb-3,2025-03-02 10:00:00 UTC,Send,BTC,0.00250000,,,,,0,,,bc1qrcv\r\n\
cb-4,2025-03-02 11:00:00 UTC,Receive,BTC,0.00250000,,,,,0,,bc1qsnd,\r\n\
cb-5,2025-03-01 08:00:00 UTC,Buy,ETH,1.00000000,USD,2000.00,2000.00,2010.00,10.00,,,\r\n\
cb-6,2025-03-01 08:30:00 UTC,Order,BTC,0.00100000,,,,,0,,,\r\n\
cb-7,2025-03-01 07:00:00 UTC,Exchange Deposit,BTC,0.00010000,,,,,0,,,\r\n";

fn group(path: std::path::PathBuf) -> FileGroup {
    FileGroup {
        source: Source::Coinbase,
        label: "coinbase".into(),
        files: vec![SourceFile::new(path)],
    }
}

#[test]
fn coinbase_maps_types_filters_non_btc_and_unclassifies_internal_moves() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coinbase_2025.csv");
    std::fs::write(&path, CSV).unwrap();
    let prices = BundledPrices::load().unwrap();
    let cb = Coinbase;
    let g = group(path);
    let rows = cb.parse(&g).unwrap();
    let out = cb.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.dropped_no_btc, 1); // the ETH row (no BTC leg)
    assert_eq!(out.unclassified, 2); // Order + Exchange Deposit (internal-move) rows
                                     // Buy, Sell, Send, Receive, Order(Unclass), Exchange Deposit(Unclass) = 6 BTC events; ETH dropped.
    assert_eq!(out.events.len(), 6);

    let buy = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Acquire(_)))
        .unwrap();
    match &buy.payload {
        EventPayload::Acquire(a) => {
            assert_eq!(a.sat, 1_000_000);
            assert_eq!(a.usd_cost.to_string(), "840.00"); // Subtotal (cost); fee separate
            assert_eq!(a.fee_usd.to_string(), "5.00"); // Fees and/or Spread → basis = 845.00 = Total
            assert_eq!(a.basis_source, BasisSource::ExchangeProvided);
        }
        _ => unreachable!(),
    }
    let sell = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Dispose(_)))
        .unwrap();
    match &sell.payload {
        EventPayload::Dispose(d) => {
            assert_eq!(d.kind, DisposeKind::Sell);
            assert_eq!(d.usd_proceeds.to_string(), "421.25"); // GROSS Subtotal
            assert_eq!(d.fee_usd.to_string(), "2.00");
        }
        _ => unreachable!(),
    }
    // Send → TransferOut with Recipient Address as dest; Receive → TransferIn with Sender Address as src.
    let send = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::TransferOut(_)))
        .unwrap();
    match &send.payload {
        EventPayload::TransferOut(t) => assert_eq!(t.dest_addr.as_deref(), Some("bc1qrcv")),
        _ => unreachable!(),
    }
    let recv = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::TransferIn(_)))
        .unwrap();
    match &recv.payload {
        EventPayload::TransferIn(t) => assert_eq!(t.src_addr.as_deref(), Some("bc1qsnd")),
        _ => unreachable!(),
    }
    // native `ID` source_ref, direction-scoped.
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|coinbase|trade|cb-1"));
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|coinbase|out|cb-3"));
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|coinbase|in|cb-4"));
}
