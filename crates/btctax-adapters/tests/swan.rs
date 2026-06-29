use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::swan::Swan;
use btctax_core::{EventPayload, Source};

// SYNTHETIC Swan 3-file batch: the REAL §9.1 per-role header names, INVENTED values. trades = no
// preamble + `MM/DD/YYYY` dates; transfers/withdrawals = 2 preamble lines + `…+00` dates. No on-chain
// txid column in any role.
const TRADES: &str = "Date,Received Quantity,Received Currency,Sent Quantity,Sent Currency,Fee Amount,Fee Currency,Tag\n\
03/01/2025 12:00:00,0.10000000,BTC,8400.00,USD,40.00,USD,\n";
const TRANSFERS: &str = "Swan Bitcoin Inc\n\
123 Main St · 555-0100\n\
Event,Date,Timezone,Status,Transaction ID,Total USD,Transaction USD,Fee USD,Unit Count,Asset Type,BTC Price,Address Label,USD Cost Basis,Acquisition Date\n\
deposit,2025-03-02 09:00:00+00,UTC,settled,sw-x1,3000.00,3000.00,0,0.05000000,BTC,60000.00,cold,3000.00,2024-01-15\n\
purchase,2025-03-04 09:00:00+00,UTC,settled,sw-p1,2100.00,2080.00,20.00,0.02500000,BTC,84000.00,,2080.00,2025-03-04\n\
monthly_fee,2025-03-31 23:59:00+00,UTC,settled,sw-f1,9.99,0,9.99,0.00012000,BTC,83250.00,,,\n";
const WITHDRAWALS: &str = "Swan Bitcoin Inc\n\
123 Main St · 555-0100\n\
Created At,Timezone,Transaction ID,Executed At,Canceled At,Status,Bitcoin Amount,Automatic,IP Address\n\
2025-03-03 10:00:00+00,UTC,sw-w1,2025-03-03 10:05:00+00,,settled,0.02000000,true,1.2.3.4\n";

#[test]
fn swan_groups_three_files_routes_roles_and_events() {
    let dir = tempfile::tempdir().unwrap();
    let t = dir.path().join("swan_trades.csv");
    let x = dir.path().join("swan_transfers.csv");
    let w = dir.path().join("swan_withdrawals.csv");
    std::fs::write(&t, TRADES).unwrap();
    std::fs::write(&x, TRANSFERS).unwrap();
    std::fs::write(&w, WITHDRAWALS).unwrap();

    let prices = BundledPrices::load().unwrap();
    let sw = Swan;
    let files = vec![
        SourceFile::new(&t),
        SourceFile::new(&x),
        SourceFile::new(&w),
    ];
    // all three detect as Swan (by per-role header signature)
    for f in &files {
        assert!(sw.detect(f).unwrap());
    }
    let groups = sw.group(files);
    assert_eq!(groups.len(), 1); // 3 files → 1 batch
    let g = &groups[0];
    let rows = sw.parse(g).unwrap();
    let out = sw.normalize(g, rows, &prices).unwrap();

    // trade Acquire + transfers{deposit→TransferIn, purchase→Acquire, monthly_fee→Unclassified}
    // + withdrawal TransferOut = 5 events; monthly_fee is the lone Unclassified.
    assert_eq!(out.events.len(), 5);
    assert_eq!(out.unclassified, 1);
    assert_eq!(
        out.events
            .iter()
            .filter(|e| matches!(&e.payload, EventPayload::Acquire(_)))
            .count(),
        2
    );

    // the trade Acquire (sat 0.10 BTC): cost = Sent Quantity, fee = Fee Amount.
    let trade = out
        .events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::Acquire(a) if a.sat == 10_000_000 => Some(a),
            _ => None,
        })
        .unwrap();
    assert_eq!(trade.usd_cost.to_string(), "8400.00");
    assert_eq!(trade.fee_usd.to_string(), "40.00");

    // deposit → TransferIn (sat = Unit Count; no txid column → None); native Transaction ID source_ref.
    let tin = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::TransferIn(_)))
        .unwrap();
    match &tin.payload {
        EventPayload::TransferIn(t) => {
            assert_eq!(t.sat, 5_000_000);
            assert_eq!(t.txid, None);
        }
        _ => unreachable!(),
    }
    assert!(out
        .events
        .iter()
        .any(|e| matches!(&e.payload, EventPayload::TransferOut(_))));

    // source_refs: purchase = native Transaction ID (dir trade); deposit = native (dir in);
    // trades + withdrawals are id-less → semantic (direction-scoped).
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|swan|trade|sw-p1")); // purchase
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|swan|in|sw-x1")); // deposit
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical() == "import|swan|trade|sw-f1")); // monthly_fee (Unclassified)
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical().starts_with("import|swan|trade|")
            && e.id.canonical().contains('#'))); // trade (semantic)
    assert!(out
        .events
        .iter()
        .any(|e| e.id.canonical().starts_with("import|swan|out|"))); // withdrawal (semantic)
}

/// A zero-sat Swan withdrawal row must increment `skipped_zero_sat` (degenerate BTC row) and must
/// NOT increment `dropped_no_btc` (which tracks rows with no BTC leg, a semantically distinct case).
#[test]
fn swan_withdrawal_zero_sat_increments_skipped_zero_sat_not_dropped_no_btc() {
    // SYNTHETIC withdrawals file: one normal row (0.02 BTC) + one zero-sat degenerate row.
    const WITHDRAWALS_ZERO_SAT: &str = "Swan Bitcoin Inc\n\
        123 Main St · 555-0100\n\
        Created At,Timezone,Transaction ID,Executed At,Canceled At,Status,Bitcoin Amount,Automatic,IP Address\n\
        2025-04-01 10:00:00+00,UTC,sw-w1,2025-04-01 10:05:00+00,,settled,0.02000000,true,1.2.3.4\n\
        2025-04-02 11:00:00+00,UTC,sw-w2,,,pending,0.00000000,false,1.2.3.5\n";

    let dir = tempfile::tempdir().unwrap();
    let w = dir.path().join("swan_withdrawals.csv");
    std::fs::write(&w, WITHDRAWALS_ZERO_SAT).unwrap();

    let prices = BundledPrices::load().unwrap();
    let sw = Swan;
    let sf = SourceFile::new(&w);
    assert!(sw.detect(&sf).unwrap());

    let group = FileGroup {
        source: Source::Swan,
        label: "swan-batch".to_string(),
        files: vec![sf],
    };
    let rows = sw.parse(&group).unwrap();
    let out = sw.normalize(&group, rows, &prices).unwrap();

    // The normal row produces one TransferOut event; the zero-sat row is skipped.
    assert_eq!(
        out.events.len(),
        1,
        "only the non-zero withdrawal should produce an event"
    );
    assert_eq!(
        out.skipped_zero_sat, 1,
        "zero-sat withdrawal must increment skipped_zero_sat"
    );
    assert_eq!(
        out.dropped_no_btc, 0,
        "zero-sat BTC row must NOT increment dropped_no_btc (that counter is for non-BTC-leg rows)"
    );
}
