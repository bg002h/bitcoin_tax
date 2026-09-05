use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::gemini::Gemini;
use btctax_core::conventions::Usd;
use btctax_core::{BasisSource, DisposeKind, EventPayload, Source};
use rust_decimal_macros::dec;
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

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// FR-45 — Gemini credit-card REWARD payouts
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// A second fixture carrying the `Specification` column, which the §9.1 fixture above does not have.
/// Kept separate on purpose: the older fixture asserts exact row/ref counts, and widening it would
/// make every FR-45 assertion a change to an unrelated test.
fn write_reward_fixture(path: &std::path::Path) {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let header = [
        "Date",
        "Time (UTC)",
        "Type",
        "Symbol",
        "Specification",
        "BTC Amount BTC",
        "USD Amount USD",
        "Fee (USD) USD",
        "Tx Hash",
        "Deposit Destination",
    ];
    for (c, h) in header.iter().enumerate() {
        ws.write_string(0, c as u16, *h).unwrap();
    }
    // Every row is Type=Credit. ONLY `Specification` tells them apart — which is the whole finding.
    // ★ Rewards carry NO Tx Hash and NO Deposit Destination; the on-chain deposit carries both.
    //   That asymmetry is real, measured on a live export, not invented for the fixture.
    let rows: [[&str; 10]; 8] = [
        // (1) reward on a day the bundled dataset CAN price → Acquire at FMV.
        [
            "2025-03-02 11:00:00",
            "2025-03-02 11:00:00",
            "Credit",
            "BTC",
            "Deposit (Gemini Credit Card Reward Payout BTC)",
            "0.00100000",
            "",
            "",
            "",
            "",
        ],
        // (2) the SAME thing under a differently-decorated specification. If the predicate ever
        //     tightens to a byte-exact match, this row silently reverts to the zero-basis defect.
        [
            "2025-03-02 11:30:00",
            "2025-03-02 11:30:00",
            "Credit",
            "BTC",
            "Gemini Credit Card Reward Payout",
            "0.00100000",
            "",
            "",
            "",
            "",
        ],
        // (3) reward on a day BEYOND the bundled dataset → no defensible basis → Unclassified.
        [
            "2026-08-01 11:00:00",
            "2026-08-01 11:00:00",
            "Credit",
            "BTC",
            "Deposit (Gemini Credit Card Reward Payout BTC)",
            "0.00100000",
            "",
            "",
            "",
            "",
        ],
        // (4) a GENUINE on-chain deposit — must still be a TransferIn. The regression guard.
        [
            "2025-03-02 13:00:00",
            "2025-03-02 13:00:00",
            "Credit",
            "BTC",
            "Deposit (Pre-Credited BTC)",
            "0.00100000",
            "",
            "",
            "feedface",
            "bc1qdp",
        ],
        // (5) I-2 — a reward that DOES state a fee. The fee must be carried, not discarded.
        [
            "2025-03-02 14:00:00",
            "2025-03-02 14:00:00",
            "Credit",
            "BTC",
            "Deposit (Gemini Credit Card Reward Payout BTC)",
            "0.00100000",
            "",
            "0.25",
            "",
            "",
        ],
        // (6) M-1 — a NEGATIVE reward credit: a reversal/clawback, never an acquisition.
        [
            "2025-03-02 15:00:00",
            "2025-03-02 15:00:00",
            "Credit",
            "BTC",
            "Gemini Credit Card Reward Payout Reversal",
            "-0.00100000",
            "",
            "",
            "",
            "",
        ],
        // (7) I-1 — a Credit with a BLANK Specification and NO on-chain marker. Character unknown.
        [
            "2025-03-02 16:00:00",
            "2025-03-02 16:00:00",
            "Credit",
            "BTC",
            "",
            "0.00100000",
            "",
            "",
            "",
            "",
        ],
        // (8) M-4 — a reward that states its OWN USD value; FR3 says the export beats the dataset.
        [
            "2025-03-02 17:00:00",
            "2025-03-02 17:00:00",
            "Credit",
            "BTC",
            "Deposit (Gemini Credit Card Reward Payout BTC)",
            "0.00100000",
            "100.00",
            "",
            "",
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

fn reward_payloads() -> Vec<EventPayload> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gemini_rewards.xlsx");
    write_reward_fixture(&path);
    let prices = BundledPrices::load().unwrap();
    let gm = Gemini;
    let g = FileGroup {
        source: Source::Gemini,
        label: "gemini".into(),
        files: vec![SourceFile::new(path)],
    };
    let rows = gm.parse(&g).unwrap();
    gm.normalize(&g, rows, &prices)
        .unwrap()
        .events
        .into_iter()
        .map(|e| e.payload)
        .collect()
}

/// ★★★ **FR-45, the headline.** A Gemini credit-card reward is a purchase-price REBATE: not gross
/// income at receipt, and the coins take basis = FMV at receipt. Before this, `Type=Credit` alone
/// routed it to `TransferIn`, it took the inbound self-transfer path, and it landed at **zero
/// basis** — overstating the gain on every later disposal.
#[test]
fn a_credit_card_reward_is_an_acquire_at_fmv_not_a_zero_basis_transfer() {
    let ps = reward_payloads();

    let acquires: Vec<_> = ps
        .iter()
        .filter_map(|p| match p {
            EventPayload::Acquire(a) => Some(a),
            _ => None,
        })
        .collect();
    assert_eq!(
        acquires.len(),
        4,
        "every priced, positive reward row must become an Acquire, got payloads: {ps:?}"
    );

    for a in &acquires {
        assert_eq!(
            a.basis_source,
            BasisSource::CardRewardRebate,
            "a reward is a rebate, NOT ExchangeProvided and NOT FmvAtIncome"
        );
        // 0.001 BTC × the 2025-03-02 bundled close of 88,710.78 = 88.71078, which `fmv_of`
        // rounds to whole CENTS — 88.71. The rounding is the money type's, not a fudge here:
        // a basis is a USD amount and USD has two places. Row (8) states its own USD (100.00),
        // which FR3 prefers over the dataset — so the set is {88.71, 100.00}, never a zero.
        assert!(
            a.usd_cost == dec!(88.71) || a.usd_cost == dec!(100.00),
            "basis must be the FMV at receipt (dataset 88.71, or the export's own 100.00), got {}",
            a.usd_cost
        );
        assert_ne!(a.usd_cost, Usd::ZERO, "the defect was a ZERO-basis lot");
    }

    // And the reward must NOT have become income: a rebate is not gross income at receipt.
    assert!(
        !ps.iter().any(|p| matches!(p, EventPayload::Income(_))),
        "a card reward is a REBATE — booking it as Income would tax it. Got: {ps:?}"
    );
}

/// The predicate matches the PHRASE, not the whole decorated string. Gemini already varies the
/// trailing asset token, and a byte-exact match would fail OPEN — straight back into the zero-basis
/// TransferIn path, silently. This test is what makes that rationale enforceable.
#[test]
fn the_reward_predicate_ignores_the_decoration_around_the_phrase() {
    let ps = reward_payloads();
    let n = ps
        .iter()
        .filter(|p| {
            matches!(p, EventPayload::Acquire(a) if a.basis_source == BasisSource::CardRewardRebate)
        })
        .count();
    assert_eq!(
        n, 4,
        "`Deposit (… Payout BTC)` and a bare `Gemini Credit Card Reward Payout` are the same event"
    );
}

/// ★★ **The refusal.** Gemini states no USD value on a reward row (measured blank on every reward row of a real export), so
/// the basis can only come from the price dataset. When the dataset cannot price the day, this must
/// REFUSE rather than substitute a number. Fabricating a basis is the defect FR-45 is about;
/// fabricating a *different* basis would not be a fix.
#[test]
fn a_reward_with_no_price_for_the_day_refuses_instead_of_inventing_a_basis() {
    let ps = reward_payloads();
    assert_eq!(
        ps.iter()
            .filter(|p| matches!(p, EventPayload::Unclassified(_)))
            .count(),
        3,
        "unpriced (row 3), negative (row 6) and character-unknown (row 7) must all refuse: {ps:?}"
    );
    // The specific failure that must never happen: an unpriced reward booked at $0.
    assert!(
        !ps.iter().any(|p| matches!(
            p,
            EventPayload::Acquire(a)
                if a.basis_source == BasisSource::CardRewardRebate && a.usd_cost == Usd::ZERO
        )),
        "an unpriced reward must never become a zero-basis lot — that is the original defect"
    );
}

/// The regression guard. FR-45 narrows the `Credit` arm; a genuine on-chain deposit must be
/// untouched by it.
#[test]
fn a_genuine_on_chain_credit_is_still_a_transfer_in() {
    let ps = reward_payloads();
    assert_eq!(
        ps.iter()
            .filter(|p| matches!(p, EventPayload::TransferIn(_)))
            .count(),
        1,
        "`Deposit (Pre-Credited BTC)` carries a Tx Hash and a Deposit Destination — it IS a \
         transfer and must stay one: {ps:?}"
    );
}

// ── FR-45 review fold: one test per finding ──────────────────────────────────────────────────────

/// ★★★ **I-1.** `row.opt` returns `None` for a column that is absent OR blank, so the reward guard
/// cannot tell "not a reward" from "no `Specification` at all". Falling through to `TransferIn` in
/// the second case silently restored the zero-basis defect — no error, no counter, no test. A Credit
/// is now a transfer only on POSITIVE on-chain evidence (`Tx Hash` or `Deposit Destination`).
#[test]
fn a_credit_with_no_specification_and_no_on_chain_marker_refuses_instead_of_assuming_a_transfer() {
    let ps = reward_payloads();
    // Row (7): Type=Credit, blank Specification, no Tx Hash, no Deposit Destination.
    assert_eq!(
        ps.iter()
            .filter(|p| matches!(p, EventPayload::TransferIn(_)))
            .count(),
        1,
        "only the row carrying on-chain markers may be a TransferIn — a character-unknown credit \
         must NOT default into the zero-basis path: {ps:?}"
    );
}

/// ★★ **I-2.** `fee_usd` was hardcoded to zero, and the test that "pinned" it asserted the hardcode,
/// so it could never fail. A discarded acquisition fee understates basis and overstates the later
/// gain. Row (5) states a fee; it must arrive.
#[test]
fn a_reward_that_states_a_fee_carries_it_rather_than_discarding_it() {
    let fees: Vec<_> = reward_payloads()
        .into_iter()
        .filter_map(|p| match p {
            EventPayload::Acquire(a) if a.basis_source == BasisSource::CardRewardRebate => {
                Some(a.fee_usd)
            }
            _ => None,
        })
        .collect();
    assert!(
        fees.contains(&dec!(0.25)),
        "the stated 0.25 fee must reach the lot, not be replaced by a hardcoded zero: {fees:?}"
    );
}

/// ★★ **M-1.** `sat` is `.abs()`ed file-wide, so a NEGATIVE reward credit — a reversal or clawback —
/// would become a positive acquisition carrying real basis the filer never received. That is the
/// UNDERSTATEMENT direction, which this project treats as the worse one. It must refuse.
#[test]
fn a_negative_reward_credit_is_a_reversal_and_never_mints_basis() {
    let ps = reward_payloads();
    // Row (6) is -0.00100000 on a day the dataset CAN price, so nothing but the sign check stops
    // it. The discriminating count is the number of reward LOTS: rows (1), (2), (5) and (8) mint
    // one each and row (6) must not — so removing the sign check reads 5, not 4.
    let reward_lots = ps
        .iter()
        .filter(|p| {
            matches!(p, EventPayload::Acquire(a) if a.basis_source == BasisSource::CardRewardRebate)
        })
        .count();
    assert_eq!(
        reward_lots, 4,
        "a negative reward credit is a reversal and must refuse, not mint a 5th lot: {ps:?}"
    );
    // …and it must land in the refusal bucket rather than vanishing silently.
    assert_eq!(
        ps.iter()
            .filter(|p| matches!(p, EventPayload::Unclassified(_)))
            .count(),
        3,
        "unpriced (3), negative (6) and character-unknown (7) must all be refusals: {ps:?}"
    );
}

/// **M-4 / FR3.** The export's own stated USD beats the bundled dataset close. Row (8) states
/// 100.00 on a day whose close would give 88.71, so the two are distinguishable.
#[test]
fn a_reward_that_states_its_own_usd_value_uses_it_over_the_dataset_close() {
    let costs: Vec<_> = reward_payloads()
        .into_iter()
        .filter_map(|p| match p {
            EventPayload::Acquire(a) if a.basis_source == BasisSource::CardRewardRebate => {
                Some(a.usd_cost)
            }
            _ => None,
        })
        .collect();
    assert!(
        costs.contains(&dec!(100.00)),
        "FR3 prefers the export's own USD over the dataset close (88.71 that day): {costs:?}"
    );
}
