//! SYNTHETIC export fixtures for CLI integration tests — real §9.1 header names, invented values only.
//! PRIVACY: no test ever reads ~/Documents/BitcoinTax/ReadOnly; these are written into a tempdir.
use std::path::{Path, PathBuf};

/// A Coinbase CSV (3-line preamble + real 13-col header) with a Buy(Acquire), a Sell(Dispose), and a
/// Send(TransferOut→pending). `dir` is a tempdir; returns the file path.
#[allow(dead_code)] // used in later task tests
pub fn coinbase_buy_sell_send(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.02000000,USD,67500.00,1350.00,1340.00,10.00,,,\r\n\
cb-send,2025-06-20 12:00:00 UTC,Send,BTC,0.03000000,USD,68000.00,,,,,,bc1qsyntheticdest\r\n",
    )
    .unwrap();
    p
}

/// A Coinbase CSV with a Buy + Receive (unclassified TransferIn → hard UnknownBasisInbound blocker).
/// Receive rows without a ClassifyInbound decision fold as Op::UnknownInbound → hard blocker (§7.3).
#[allow(dead_code)]
pub fn coinbase_buy_receive(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_recv.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy3,2025-04-01 12:00:00 UTC,Buy,BTC,0.05000000,USD,85000.00,4250.00,4265.00,15.00,,,\r\n\
cb-recv,2025-05-01 12:00:00 UTC,Receive,BTC,0.02000000,USD,90000.00,,,,,,\r\n",
    )
    .unwrap();
    p
}

/// Two-lot donation fixture: two Buy events (LT lot A + ST lot B) + one Send consuming both.
/// Used by the multi-leg donation CSV KAT (P2-A Minor: no double-counting in removals.csv).
///
/// Lot A (LT): acquired 2024-01-01, 1 BTC (100,000,000 sat), basis $5,000.
/// Lot B (ST): acquired 2025-12-01, 1 BTC (100,000,000 sat), basis $2,000.
/// Send:       2026-03-01, 2 BTC → reclassified as Donate with FMV $100,000.
///
/// With total FMV $100,000 (pro-rata $50,000 each):
///   LT leg (lot A): deduction = FMV = $50,000.
///   ST leg (lot B): deduction = min(FMV, basis) = min($50,000, $2,000) = $2,000.
///   Total claimed_deduction = $52,000.
#[allow(dead_code)]
pub fn coinbase_two_lot_donation(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_two_lot_donation.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-donate-buy-a,2024-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,5000.00,5000.00,5000.00,0.00,,,\r\n\
cb-donate-buy-b,2025-12-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,2000.00,2000.00,2000.00,0.00,,,\r\n\
cb-donate-send,2026-03-01 12:00:00 UTC,Send,BTC,2.00000000,USD,50000.00,,,,,,bc1qsyntheticcharity\r\n",
    )
    .unwrap();
    p
}

/// A Coinbase CSV with a single Buy only (self-contained USD; no price-dataset dependency).
#[allow(dead_code)] // used in init_import.rs; appears unused in verify_report.rs compilation unit
pub fn coinbase_single_buy(dir: &Path) -> PathBuf {
    let p = dir.join("coinbase_buy.csv");
    std::fs::write(
        &p,
        "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-eth,2025-03-01 08:00:00 UTC,Buy,ETH,1.00000000,USD,2000.00,2000.00,2010.00,10.00,,,\r\n",
    )
    .unwrap();
    p
}
