use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::gemini::Gemini;
use btctax_core::{DisposeKind, EventPayload, Source};
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
    // (d) Pin DisposeKind::Sell on the Sell KAT (closes M-2 gap).
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
            assert_eq!(d.kind, DisposeKind::Sell); // (d) Sell KAT kind pin
        }
        _ => unreachable!(),
    }
}

// ── I-1 KAT: ETHBTC Buy (BTC-quoted pair) → Unclassified, never Acquire, never zero-basis ────────
//
// `Symbol=ETHBTC, Type=Buy` means "buy ETH with BTC" — BTC is the quote currency being disposed,
// NOT a BTCUSD purchase. Emitting Acquire{usd_cost=ZERO} would create a phantom zero-basis lot.
// Emitting the row as Unclassified forces the user to classify the BTC leg explicitly.
fn write_ethbtc_fixture(path: &std::path::Path) {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    // Minimal column set; Symbol is present (I-1 gate reads it).
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
    // ETHBTC Buy row: BTC leg present (0.01 BTC as the quote currency disposed), no USD Amount.
    let row: [&str; 13] = [
        "2025-04-01 10:00:00",
        "2025-04-01 10:00:00",
        "Buy",
        "ETHBTC",     // BTC-quoted pair — NOT BTCUSD
        "0.01000000", // BTC Amount BTC present → passes FR2
        "",           // USD Amount USD absent → has_usd = false
        "",           // Fee (USD) USD absent
        "0.01000000",
        "T-E1",
        "O-E1",
        "",
        "",
        "",
    ];
    for (c, v) in row.iter().enumerate() {
        ws.write_string(1, c as u16, *v).unwrap();
    }
    wb.save(path).unwrap();
}

#[test]
fn gemini_btcquoted_pair_buy_is_unclassified() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ethbtc.xlsx");
    write_ethbtc_fixture(&path);
    let prices = BundledPrices::load().unwrap();
    let gm = Gemini;
    let g = FileGroup {
        source: Source::Gemini,
        label: "gemini_ethbtc".into(),
        files: vec![SourceFile::new(path)],
    };
    let rows = gm.parse(&g).unwrap();
    let out = gm.normalize(&g, rows, &prices).unwrap();

    // The ETHBTC Buy row has a BTC leg (sat>0) → NOT dropped by FR2.
    assert_eq!(out.dropped_no_btc, 0);
    // I-1: must be routed to Unclassified, not Acquire.
    assert_eq!(out.unclassified, 1);
    assert_eq!(out.events.len(), 1);
    match &out.events[0].payload {
        EventPayload::Unclassified(_) => {} // correct
        EventPayload::Acquire(a) => {
            panic!(
                "ETHBTC Buy must not become Acquire; got usd_cost={}",
                a.usd_cost
            )
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

// ── I-2 KAT: negative/parenthesized USD columns → positive basis and proceeds ──────────────────
//
// Gemini may encode outflow magnitudes as accounting-negatives or parenthesized values.
// `parse_usd` preserves sign; the Gemini parser must abs-normalize so that a negative-encoded
// Buy doesn't produce a negative usd_cost (phantom negative basis), and a parenthesized Sell
// doesn't produce a negative usd_proceeds.
fn write_negative_usd_fixture(path: &std::path::Path) {
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
    // Row 1: BTCUSD Buy with negative USD Amount and negative Fee.
    let buy_row: [&str; 13] = [
        "2025-05-01 09:00:00",
        "2025-05-01 09:00:00",
        "Buy",
        "BTCUSD",
        "0.01000000",
        "-1000.00", // negative USD Amount USD (cost magnitude, negative encoding)
        "-5.00",    // negative Fee (USD) USD
        "0.01000000",
        "T-N1",
        "O-N1",
        "",
        "",
        "",
    ];
    // Row 2: BTCUSD Sell with parenthesized (accounting-negative) USD Amount and Fee.
    let sell_row: [&str; 13] = [
        "2025-05-02 09:00:00",
        "2025-05-02 09:00:00",
        "Sell",
        "BTCUSD",
        "0.01000000",
        "(900.00)", // parenthesized → parse_usd returns -900.00; Gemini parser must abs()
        "(4.00)",   // parenthesized fee
        "0.00000000",
        "T-N2",
        "O-N2",
        "",
        "",
        "",
    ];
    for (c, v) in buy_row.iter().enumerate() {
        ws.write_string(1, c as u16, *v).unwrap();
    }
    for (c, v) in sell_row.iter().enumerate() {
        ws.write_string(2, c as u16, *v).unwrap();
    }
    wb.save(path).unwrap();
}

#[test]
fn gemini_negative_usd_normalized_to_positive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("neg_usd.xlsx");
    write_negative_usd_fixture(&path);
    let prices = BundledPrices::load().unwrap();
    let gm = Gemini;
    let g = FileGroup {
        source: Source::Gemini,
        label: "gemini_neg".into(),
        files: vec![SourceFile::new(path)],
    };
    let rows = gm.parse(&g).unwrap();
    let out = gm.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.events.len(), 2);

    // I-2: Buy — usd_cost and fee_usd must be POSITIVE even though input was negative.
    let buy = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Acquire(_)))
        .unwrap();
    match &buy.payload {
        EventPayload::Acquire(a) => {
            assert_eq!(
                a.usd_cost.to_string(),
                "1000.00",
                "usd_cost must be positive (abs of -1000.00)"
            );
            assert_eq!(
                a.fee_usd.to_string(),
                "5.00",
                "fee_usd must be positive (abs of -5.00)"
            );
        }
        _ => unreachable!(),
    }

    // I-2: Sell — usd_proceeds and fee_usd must be POSITIVE even though input was parenthesized.
    let sell = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Dispose(_)))
        .unwrap();
    match &sell.payload {
        EventPayload::Dispose(d) => {
            assert_eq!(
                d.usd_proceeds.to_string(),
                "900.00",
                "usd_proceeds must be positive (abs of (900.00))"
            );
            assert_eq!(
                d.fee_usd.to_string(),
                "4.00",
                "fee_usd must be positive (abs of (4.00))"
            );
        }
        _ => unreachable!(),
    }
}

// ── (c) FR2 KAT: ETH-amount-only row (no BTC leg) → dropped, never Unclassified ───────────────
//
// A Gemini row with Symbol=ETH and no BTC Amount BTC must be dropped by FR2 (BTC-only filter),
// not forwarded to Unclassified. This confirms FR2 operates on the BTC Amount BTC column
// regardless of what other amount columns (ETH, BCH, …) may carry.
fn write_eth_only_fixture(path: &std::path::Path) {
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
    // Credit ETH row: Symbol=ETH, BTC Amount BTC absent (ETH-only event, no BTC leg).
    let row: [&str; 13] = [
        "2025-06-01 08:00:00",
        "2025-06-01 08:00:00",
        "Credit",
        "ETH",
        "", // BTC Amount BTC absent → sat=0 → FR2 drop
        "",
        "",
        "0.00000000",
        "",
        "",
        "",
        "",
        "",
    ];
    for (c, v) in row.iter().enumerate() {
        ws.write_string(1, c as u16, *v).unwrap();
    }
    wb.save(path).unwrap();
}

#[test]
fn gemini_eth_only_row_dropped_by_fr2() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("eth_only.xlsx");
    write_eth_only_fixture(&path);
    let prices = BundledPrices::load().unwrap();
    let gm = Gemini;
    let g = FileGroup {
        source: Source::Gemini,
        label: "gemini_eth".into(),
        files: vec![SourceFile::new(path)],
    };
    let rows = gm.parse(&g).unwrap();
    let out = gm.normalize(&g, rows, &prices).unwrap();

    // (c) ETH-amount-only row must be dropped (FR2), not forwarded.
    assert_eq!(out.dropped_no_btc, 1, "ETH-only row must be dropped by FR2");
    assert_eq!(out.unclassified, 0);
    assert_eq!(out.events.len(), 0);
}

// ── Sub-satoshi KAT [SPEC gemini-subsatoshi-round]: Gemini exports 10-dp internal-ledger amounts finer
// than a satoshi. The BTC Amount cell reaches parse_btc_to_sat via the xlsx READ path
// (Data::Float → format!("{f}")), which now ROUNDS to the nearest satoshi instead of aborting the import
// with FractionalSat. This is the exact bug the user hit ("gemini row 2: fractional satoshi …"). Covers
// BOTH a NUMERIC (Data::Float, the real Gemini shape) and a STRING cell — both must round to 102162.
fn write_subsatoshi_fixture(path: &std::path::Path) {
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
    // Row 1: BTCUSD Buy, BTC Amount 0.0010216163 (= 102161.63 sat) as a NUMERIC cell → Data::Float path.
    ws.write_string(1, 0, "2025-07-01 09:00:00").unwrap();
    ws.write_string(1, 1, "2025-07-01 09:00:00").unwrap();
    ws.write_string(1, 2, "Buy").unwrap();
    ws.write_string(1, 3, "BTCUSD").unwrap();
    ws.write_number(1, 4, 0.0010216163f64).unwrap(); // NUMERIC sub-sat → Data::Float → format!("{f}")
    ws.write_string(1, 5, "70.00").unwrap();
    ws.write_string(1, 6, "0.50").unwrap();
    ws.write_string(1, 7, "0.0010216163").unwrap();
    ws.write_string(1, 8, "T-SS1").unwrap();
    ws.write_string(1, 9, "O-SS1").unwrap();
    // Row 2: identical Buy but BTC Amount as a STRING cell → Data::String path; must also round to 102162.
    ws.write_string(2, 0, "2025-07-02 09:00:00").unwrap();
    ws.write_string(2, 1, "2025-07-02 09:00:00").unwrap();
    ws.write_string(2, 2, "Buy").unwrap();
    ws.write_string(2, 3, "BTCUSD").unwrap();
    ws.write_string(2, 4, "0.0010216163").unwrap(); // STRING sub-sat
    ws.write_string(2, 5, "70.00").unwrap();
    ws.write_string(2, 6, "0.50").unwrap();
    ws.write_string(2, 7, "0.0010216163").unwrap();
    ws.write_string(2, 8, "T-SS2").unwrap();
    ws.write_string(2, 9, "O-SS2").unwrap();
    wb.save(path).unwrap();
}

#[test]
fn gemini_subsatoshi_btc_amount_rounds_and_imports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subsat.xlsx");
    write_subsatoshi_fixture(&path);
    let prices = BundledPrices::load().unwrap();
    let gm = Gemini;
    let g = FileGroup {
        source: Source::Gemini,
        label: "gemini_subsat".into(),
        files: vec![SourceFile::new(path)],
    };
    // MUST NOT error on the sub-satoshi amounts (the bug previously aborted the whole import here).
    let rows = gm.parse(&g).unwrap();
    let out = gm.normalize(&g, rows, &prices).unwrap();
    // Both Buy rows (numeric cell + string cell) → Acquire with sat rounded 102161.63 → 102162.
    assert_eq!(out.events.len(), 2);
    for e in &out.events {
        match &e.payload {
            EventPayload::Acquire(a) => assert_eq!(
                a.sat, 102_162,
                "0.0010216163 BTC (102161.63 sat) must round to nearest satoshi 102162"
            ),
            other => panic!("unexpected payload: {other:?}"),
        }
    }
}
