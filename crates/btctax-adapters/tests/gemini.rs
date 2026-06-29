use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::gemini::Gemini;
use btctax_core::{EventPayload, Source};
use rust_xlsxwriter::Workbook;

// SYNTHETIC Gemini XLSX: the REAL §9.1 header names (a subset of the 30 cols — the parser reads only
// what it needs), INVENTED values. One sheet.
// M-1 / IP-1: the Buy row's `Date` cell is written as an Excel serial number (not a string) to
// exercise the calamine numeric→Data::Float→parse_timestamp_flex(serial)→UTC path end-to-end.
// Serial 45717.5 ≈ 2025-03-01 12:00:00 UTC (anchor: 25569 = 1970-01-01 UTC; 45717 - 25569 = 20148
// days = ~55 years + 59 days to 2025-03-01; 0.5 fraction = 12:00:00). Remaining rows use ISO text
// (parse_timestamp_flex handles both — all string-path tests are in `parse::tests`).
fn write_fixture(path: &std::path::Path) {
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
    // Buy row (row 1): Date as numeric Excel serial (exercises IP-1 path); all other cells as strings.
    ws.write_number(1, 0, 45717.5f64).unwrap(); // M-1: serial → Data::Float → parse_timestamp_flex
    for (c, v) in [
        "2025-03-01 12:00:00",
        "Buy",
        "BTCUSD",
        "0.02000000",
        "1680.00",
        "5.00",
        "0.02000000",
        "T-1",
        "O-1",
        "",
        "",
        "",
    ]
    .iter()
    .enumerate()
    {
        ws.write_string(1, (c + 1) as u16, *v).unwrap();
    }
    // Remaining rows: all cells as strings (ISO text — parse_timestamp handles them).
    // Sell 0.01; Debit (BTC out → TransferOut); Credit BTC (→ TransferIn); Credit USD (→ dropped).
    let rows: [[&str; 13]; 4] = [
        [
            "2025-03-02 09:00:00",
            "2025-03-02 09:00:00",
            "Sell",
            "BTCUSD",
            "0.01000000",
            "842.50",
            "2.50",
            "0.01000000",
            "T-2",
            "O-2",
            "",
            "",
            "",
        ],
        [
            "2025-03-02 10:00:00",
            "2025-03-02 10:00:00",
            "Debit",
            "BTC",
            "0.00500000",
            "",
            "",
            "0.00500000",
            "",
            "",
            "deadbeef",
            "",
            "bc1qwd",
        ],
        [
            "2025-03-02 11:00:00",
            "2025-03-02 11:00:00",
            "Credit",
            "BTC",
            "0.00100000",
            "",
            "",
            "0.00600000",
            "",
            "",
            "feedface",
            "bc1qdp",
            "",
        ],
        [
            "2025-03-02 12:00:00",
            "2025-03-02 12:00:00",
            "Credit",
            "USD",
            "",
            "500.00",
            "",
            "0.00600000",
            "",
            "",
            "",
            "",
            "",
        ],
    ];
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            ws.write_string((r + 2) as u32, c as u16, *v).unwrap();
        }
    }
    wb.save(path).unwrap();
}

#[test]
fn gemini_maps_btc_legs_native_and_semantic_refs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gemini_ledger.xlsx");
    write_fixture(&path);
    let prices = BundledPrices::load().unwrap();
    let gm = Gemini;
    let g = FileGroup {
        source: Source::Gemini,
        label: "gemini".into(),
        files: vec![SourceFile::new(path)],
    };
    let rows = gm.parse(&g).unwrap();
    let out = gm.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.dropped_no_btc, 1); // Credit(USD) cash (no BTC leg)
    assert_eq!(out.unclassified, 0); // Credit(BTC) is a TransferIn now, not Unclassified
                                     // Buy, Sell, Debit→TransferOut, Credit→TransferIn = 4 BTC events.
    assert_eq!(out.events.len(), 4);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(&e.payload, EventPayload::Acquire(_))));
    assert!(out
        .events
        .iter()
        .any(|e| matches!(&e.payload, EventPayload::Dispose(_))));

    // Debit → TransferOut (txid = Tx Hash, dest = Withdrawal Destination); semantic id-less source_ref.
    let debit = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::TransferOut(_)))
        .unwrap();
    assert!(debit.id.canonical().starts_with("import|gemini|out|"));
    match &debit.payload {
        EventPayload::TransferOut(t) => {
            assert_eq!(t.txid.as_deref(), Some("deadbeef"));
            assert_eq!(t.dest_addr.as_deref(), Some("bc1qwd"));
        }
        _ => unreachable!(),
    }
    // Credit(BTC) → TransferIn (txid + src = Deposit Destination); semantic id-less source_ref.
    let credit = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::TransferIn(_)))
        .unwrap();
    assert!(credit.id.canonical().starts_with("import|gemini|in|"));
    match &credit.payload {
        EventPayload::TransferIn(t) => {
            assert_eq!(t.txid.as_deref(), Some("feedface"));
            assert_eq!(t.src_addr.as_deref(), Some("bc1qdp"));
        }
        _ => unreachable!(),
    }
    // native `Trade ID`+`Order ID` source_ref for the Buy (combined, direction-scoped).
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|gemini|trade|T-1.O-1"));

    // KAT: Buy basis — pin usd_cost and fee_usd from the XLSX cells.
    // M-1: the Buy row's Date was written as a numeric Excel serial (45717.5); verify it round-trips
    // to the correct UTC instant (2025-03-01 12:00:00 UTC = 1899-12-30 + 45717.5 days).
    let buy_event = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Acquire(_)))
        .unwrap();
    assert_eq!(
        buy_event.utc_timestamp,
        time::macros::datetime!(2025-03-01 12:00:00 UTC),
        "numeric-serial Date cell must round-trip to 2025-03-01 12:00:00 UTC"
    );
    match &buy_event.payload {
        EventPayload::Acquire(a) => {
            assert_eq!(a.sat, 2_000_000); // 0.02 BTC
            assert_eq!(a.usd_cost.to_string(), "1680.00"); // USD Amount USD → basis cost
            assert_eq!(a.fee_usd.to_string(), "5.00"); // Fee (USD) USD → separate fee
        }
        _ => unreachable!(),
    }

    // KAT: Sell gross proceeds + fee pinned separately (gross = USD Amount USD, fee = Fee (USD) USD).
    let sell_event = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Dispose(_)))
        .unwrap();
    match &sell_event.payload {
        EventPayload::Dispose(d) => {
            assert_eq!(d.sat, 1_000_000); // 0.01 BTC
            assert_eq!(d.usd_proceeds.to_string(), "842.50"); // GROSS proceeds (USD Amount USD)
            assert_eq!(d.fee_usd.to_string(), "2.50"); // Fee (USD) USD — separate from proceeds
        }
        _ => unreachable!(),
    }
}
