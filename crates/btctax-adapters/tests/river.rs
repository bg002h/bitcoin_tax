use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::river::River;
use btctax_core::{EventPayload, FmvStatus, IncomeKind, Source};

// SYNTHETIC River CSV: the REAL §9.1 header names (8-col universal Sent/Received shape), INVENTED
// values. CRLF, no preamble, real naive timestamp format. The Interest row carries NO USD → must
// resolve from the price provider (date 2025-06-15 = 67500.00). We inject a CONTROLLED synthetic
// price at that date [R0-C1] so the asserted $6.75 FMV is independent of the shipped dataset.
const CSV: &str = "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-03-01 12:00:00,4200.00,USD,0.05000000,BTC,3.00,USD,Buy\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-03-02 08:00:00,0.01000000,BTC,,,,,Withdrawal\r\n";

#[test]
fn river_semantic_refs_and_dataset_fmv_for_interest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("river_universal.csv");
    std::fs::write(&path, CSV).unwrap();
    let prices = BundledPrices::from_csv_str("date,usd_close\n2025-06-15,67500.00\n").unwrap();
    let rv = River;
    let g = FileGroup {
        source: Source::River,
        label: "river".into(),
        files: vec![SourceFile::new(path)],
    };
    let rows = rv.parse(&g).unwrap();
    let out = rv.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.events.len(), 3);
    assert_eq!(out.dropped_no_btc, 0);

    let buy = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Acquire(_)))
        .unwrap();
    match &buy.payload {
        EventPayload::Acquire(a) => {
            assert_eq!(a.sat, 5_000_000); // Received Amount (BTC)
            assert_eq!(a.usd_cost.to_string(), "4200.00"); // Sent Amount (USD)
            assert_eq!(a.fee_usd.to_string(), "3.00"); // Fee Amount → basis = Sent + Fee
        }
        _ => unreachable!(),
    }
    let inc = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::Income(_)))
        .unwrap();
    match &inc.payload {
        EventPayload::Income(i) => {
            assert_eq!(i.kind, IncomeKind::Interest);
            assert_eq!(i.sat, 10_000); // Received Amount (BTC)
            assert_eq!(i.fmv_status, FmvStatus::PriceDataset); // no export USD → dataset
                                                               // 0.0001 BTC = 10_000 sat @ 67500 = 6.75
            assert_eq!(i.usd_fmv.as_ref().unwrap().to_string(), "6.75");
        }
        _ => unreachable!(),
    }
    // Withdrawal → TransferOut, sat from the BTC `Sent Amount`.
    let wd = out
        .events
        .iter()
        .find(|e| matches!(&e.payload, EventPayload::TransferOut(_)))
        .unwrap();
    match &wd.payload {
        EventPayload::TransferOut(t) => assert_eq!(t.sat, 1_000_000),
        _ => unreachable!(),
    }
    // semantic source_ref (River is id-less); Buy direction = trade.
    assert!(buy.id.canonical().starts_with("import|river|trade|"));
}
