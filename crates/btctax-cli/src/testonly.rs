//! Shared synthetic corpora + per-journey fixtures for the worked-example journeys (J1–J9).
//!
//! Hoisted here (from xtask, TUI-walkthrough spec §4.1) so ALL consumers share ONE source of truth: the
//! `xtask` examples generator, and the `btctax-tui` / `btctax-tui-edit` screen-walkthrough emit tests. A
//! plain `pub` module (like `btctax_forms::testonly`) — the corpora are tiny synthetic CSVs, and the
//! generator's `generate()` is a non-test fn, so they cannot be `#[cfg(test)]`.
//!
//! The CSV corpora carry explicit CRLF: committed `.csv` files are force-LF'd by `.gitattributes` and would
//! break the Coinbase parser, so they live as string consts (a driver writes them to a tempdir at runtime).
//! These bytes are byte-identical to their previous home in `xtask/src/examples.rs` — the
//! `examples_golden_matches_committed` gate proves the move changed nothing.

// ── Synthetic corpora (explicit CRLF) ──────────────────────────────────────────────────────────────

/// J1 corpus: a single-buyer happy path — one Buy + one partial Sell in 2025.
pub const J1_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-03-01 12:00:00 UTC,Buy,BTC,0.10000000,USD,84000.00,8400.00,8450.00,50.00,,,\r\n\
cb-sell,2025-06-15 12:00:00 UTC,Sell,BTC,0.02000000,USD,67500.00,1350.00,1340.00,10.00,,,\r\n";

/// J2 corpus: an LT lot (2023) + an ST lot (2025) + a 2025 Send of 2 BTC donated to charity.
pub const J2_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy-lt,2023-06-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,5000.00,5000.00,5000.00,0.00,,,\r\n\
cb-buy-st,2025-03-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,2000.00,2000.00,2000.00,0.00,,,\r\n\
cb-donate,2025-09-01 12:00:00 UTC,Send,BTC,2.00000000,USD,108996.17,,,,,,bc1qcharity\r\n";

/// J3 corpus: a Buy + a Receive (an inbound transfer with unknown basis → a hard blocker until classified).
pub const J3_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2025-02-01 12:00:00 UTC,Buy,BTC,0.50000000,USD,95000.00,47500.00,47550.00,50.00,,,\r\n\
cb-recv,2025-08-01 12:00:00 UTC,Receive,BTC,0.20000000,USD,110000.00,,,,,,\r\n";

/// J4 corpus: two River staking-income deposits in 2025 (FMV resolved from the bundled dataset).
pub const J4_CSV: &str =
    "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Tag\r\n\
2025-04-15 12:00:00 UTC,,,0.05000000,BTC,,income\r\n\
2025-05-20 12:00:00 UTC,,,0.03000000,BTC,,income\r\n";

/// J5 corpus: an LT lot + a higher-basis ST lot + a 2025 sell — a genuine changed-selection scenario
/// (HIFO ≠ FIFO) so the optimizer has a tax-saving pick to propose.
pub const J5_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
opt-buy-lt,2023-01-01 12:00:00 UTC,Buy,BTC,1.00000000,USD,30000.00,30000.00,30000.00,0.00,,,\r\n\
opt-buy-st,2025-01-02 12:00:00 UTC,Buy,BTC,1.00000000,USD,80000.00,80000.00,80000.00,0.00,,,\r\n\
opt-sell,2025-06-01 12:00:00 UTC,Sell,BTC,1.00000000,USD,50000.00,50000.00,50000.00,0.00,,,\r\n";

/// J6 River corpus: one small 2024 business mining-income deposit (FMV from the bundled dataset).
/// Kept modest deliberately: the kitchen-sink household clears the 2024 Form-6251 AMT-screen worksheet by
/// only a thin margin — a corpus editor who enlarges the sale, income, or donation must keep the household
/// on the computable side of that screen.
pub const J6_RIVER_CSV: &str =
    "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Tag\r\n\
2024-03-15 12:00:00 UTC,,,0.05000000,BTC,,income\r\n";

/// J6 Coinbase corpus: a cheap 2020 long-term lot, a small 2024 long-term sale (Schedule D Part II / Form
/// 8949), and a 2024 charitable Send of 0.1 BTC (§170(e) donation ⇒ Form 8283). Amounts kept small so the
/// return stays under the AMT screen.
pub const J6_COINBASE_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-buy,2020-01-01 12:00:00 UTC,Buy,BTC,0.30000000,USD,30000.00,9000.00,9000.00,0.00,,,\r\n\
cb-sell,2024-05-01 12:00:00 UTC,Sell,BTC,0.05000000,USD,63000.00,3150.00,3130.00,20.00,,,\r\n\
cb-donate,2024-09-01 12:00:00 UTC,Send,BTC,0.10000000,USD,60000.00,,,,,,bc1qcharity\r\n";

/// The committed full-return ReturnInputs (the `kitchen_sink_household()` oracle, TOML-serialized). J6
/// imports it via `income import`. The fixture lives in this crate (self-contained), so this is a
/// SAME-crate `include_str!` — retiring the cross-crate include the xtask copy carried (M-5 exception).
pub const J6_FULLRETURN_TOML: &str =
    include_str!("../tests/fixtures/examples/fullreturn_inputs.toml");

/// J7 corpus (UX-P1-7): a single 2024 Coinbase Receive of staking rewards — an unknown-basis inbound the
/// single-event `classify-inbound-income` command values only from a hand-supplied `--fmv`.
pub const J7_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2024-06-15 12:00:00 UTC,Receive,BTC,0.05000000,USD,,,,,,,\r\n";

/// J8 River corpus (UX-P1-8): a buy (to give the coins a basis) then a Withdrawal OUT of 0.10 BTC — the
/// out-leg of a cross-exchange self-transfer whose in-leg lands on Coinbase below.
pub const J8_RIVER_CSV: &str =
    "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Tag\r\n\
2025-01-05 12:00:00 UTC,4000.00,USD,0.10000000,BTC,,buy\r\n\
2025-03-10 12:00:00 UTC,0.10000000,BTC,,,,withdrawal\r\n";

/// J8 Coinbase corpus (UX-P1-8): the matching inbound Receive of 0.10 BTC — the SAME coins landing at a
/// second exchange, so the pair is a cross-wallet RELOCATE (not a same-wallet DROP).
pub const J8_COINBASE_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-recv,2025-03-10 12:00:00 UTC,Receive,BTC,0.10000000,USD,,,,,,,\r\n";

/// J9 corpus (UX-P1-10): a cheap 2023 long-term lot (0.60) + a pricier 2024 lot (0.40), then a 2025 sale of
/// only 0.50 — smaller than either combined holding, so which lots cover it is a GENUINE choice.
pub const J9_CSV: &str = "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
lot-a,2023-01-01 12:00:00 UTC,Buy,BTC,0.60000000,USD,25000.00,15000.00,15000.00,0.00,,,\r\n\
lot-b,2024-01-02 12:00:00 UTC,Buy,BTC,0.40000000,USD,60000.00,24000.00,24000.00,0.00,,,\r\n\
sale,2025-06-01 12:00:00 UTC,Sell,BTC,0.50000000,USD,47500.00,47500.00,47500.00,0.00,,,\r\n";

// ── Per-journey fixtures ───────────────────────────────────────────────────────────────────────────

/// One journey's synthetic input: the named corpus files a driver writes to a tempdir before `import`.
/// The SAME fixture seeds both walkthrough capture halves (the editor emit test drives its flows; the
/// viewer emit test re-seeds this corpus and replays the equivalent decisions via `btctax-cli`), so the
/// two halves of a journey converge by construction on one source of truth (walkthrough spec §4.2).
pub struct JourneyFixture {
    /// Short journey id, e.g. `"j8"`.
    pub name: &'static str,
    /// `(filename, CRLF content)` pairs written to the tempdir before ingest.
    pub corpus: &'static [(&'static str, &'static str)],
}

/// The nine journey fixtures, keyed by the same corpus filenames the examples journeys use.
pub fn j1() -> JourneyFixture {
    JourneyFixture {
        name: "j1",
        corpus: &[("coinbase.csv", J1_CSV)],
    }
}
pub fn j2() -> JourneyFixture {
    JourneyFixture {
        name: "j2",
        corpus: &[("coinbase.csv", J2_CSV)],
    }
}
pub fn j3() -> JourneyFixture {
    JourneyFixture {
        name: "j3",
        corpus: &[("coinbase.csv", J3_CSV)],
    }
}
pub fn j4() -> JourneyFixture {
    JourneyFixture {
        name: "j4",
        corpus: &[("river.csv", J4_CSV)],
    }
}
pub fn j5() -> JourneyFixture {
    JourneyFixture {
        name: "j5",
        corpus: &[("coinbase.csv", J5_CSV)],
    }
}
pub fn j6() -> JourneyFixture {
    JourneyFixture {
        name: "j6",
        corpus: &[
            ("river.csv", J6_RIVER_CSV),
            ("coinbase.csv", J6_COINBASE_CSV),
        ],
    }
}
pub fn j7() -> JourneyFixture {
    JourneyFixture {
        name: "j7",
        corpus: &[("coinbase.csv", J7_CSV)],
    }
}
/// J8 — the PoC journey: a cross-exchange self-transfer (River Withdrawal out → Coinbase Receive in).
pub fn j8() -> JourneyFixture {
    JourneyFixture {
        name: "j8",
        corpus: &[
            ("river.csv", J8_RIVER_CSV),
            ("coinbase.csv", J8_COINBASE_CSV),
        ],
    }
}
pub fn j9() -> JourneyFixture {
    JourneyFixture {
        name: "j9",
        corpus: &[("coinbase.csv", J9_CSV)],
    }
}
