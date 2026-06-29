//! Gemini XLSX-ledger adapter (§9.1, confirmed schema). Native `Trade ID`+`Order ID` `source_ref`
//! (direction-scoped) on trade rows, else semantic (`Credit`/`Debit` lack trade ids); `Tx Hash` = txid
//! match signal; gross proceeds in `USD Amount USD` with `Fee (USD) USD` separate; Buy basis =
//! `USD Amount USD`(+`Fee (USD) USD`); `Debit`(BTC)→TransferOut (dest = `Withdrawal Destination`);
//! `Credit`(BTC)→TransferIn (src = `Deposit Destination`); `Credit`/`Debit`(USD) cash dropped (FR2).
//! BTC-leg = `BTC Amount BTC` populated. `Date`/`Time (UTC)` are Excel serials → `parse_timestamp_flex`.
//!
//! NOTE (M-5 — naming caveat for Plan-4 reconciler): Gemini `Credit`'s `Deposit Destination` column
//! is stored in `TransferIn.src_addr`. Despite the field name (`src_addr`), this address is Gemini's
//! own deposit address — the on-chain DESTINATION of the inbound transfer, not the originating
//! sender's address. Plan-4 address-matching must account for this: `TransferIn.src_addr` for a
//! Gemini Credit identifies the receiving-end (Gemini) address, not the true on-chain source wallet.
use crate::adapter::{Adapter, FileGroup, GroupOutput, SourceFile};
use crate::normalize::{exchange_wallet, raw_of, Direction, SourceRefMint};
use crate::parse::{parse_btc_to_sat, parse_timestamp_flex, parse_usd};
use crate::read::{read_table, RawRow, ReadOpts, TableRole};
use crate::AdapterError;
use btctax_core::{
    Acquire, BasisSource, Dispose, DisposeKind, EventId, EventPayload, LedgerEvent, PriceProvider,
    Source, TransferIn, TransferOut, Unclassified, Usd,
};

const SRC: &str = "gemini";

mod cols {
    // §9.1 CONFIRMED real headers (no OPEN items remain):
    pub const TYPE: &str = "Type";
    pub const DATE: &str = "Date"; // Excel serial (Time (UTC) carries the same instant)
    pub const BTC_AMOUNT: &str = "BTC Amount BTC"; // BTC leg amount + presence test
    pub const USD_AMOUNT: &str = "USD Amount USD";
    pub const FEE_USD: &str = "Fee (USD) USD";
    pub const TRADE_ID: &str = "Trade ID";
    pub const ORDER_ID: &str = "Order ID";
    pub const TX_HASH: &str = "Tx Hash";
    pub const DEPOSIT_DEST: &str = "Deposit Destination";
    pub const WITHDRAWAL_DEST: &str = "Withdrawal Destination";
    #[allow(dead_code)] // reconciliation/verify data (FR9, CLI) — captured by the reader, not folded here
    pub const BTC_BALANCE: &str = "BTC Balance BTC";
}

pub struct Gemini;

impl Adapter for Gemini {
    fn source(&self) -> Source {
        Source::Gemini
    }

    fn detect(&self, file: &SourceFile) -> Result<bool, AdapterError> {
        // Gemini ships an XLSX ledger; detect by extension (the reader dispatches XLSX → calamine).
        Ok(file
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xlsx"))
            .unwrap_or(false))
    }

    fn group(&self, files: Vec<SourceFile>) -> Vec<FileGroup> {
        files
            .into_iter()
            .map(|f| FileGroup {
                source: Source::Gemini,
                label: f.path.display().to_string(),
                files: vec![f],
            })
            .collect()
    }

    fn parse(&self, group: &FileGroup) -> Result<Vec<RawRow>, AdapterError> {
        let opts = ReadOpts::default();
        let mut rows = Vec::new();
        for f in &group.files {
            rows.extend(read_table(&f.path, TableRole::Single, SRC, &opts)?);
        }
        Ok(rows)
    }

    fn normalize(
        &self,
        _group: &FileGroup,
        rows: Vec<RawRow>,
        _prices: &dyn PriceProvider,
    ) -> Result<GroupOutput, AdapterError> {
        let mut mint = SourceRefMint::default();
        let mut out = GroupOutput {
            parsed_rows: rows.len(),
            ..Default::default()
        };
        for row in &rows {
            // BTC-leg presence: `BTC Amount BTC` must be populated and non-zero (FR2).
            let sat = match row.opt(cols::BTC_AMOUNT) {
                Some(s) => parse_btc_to_sat(SRC, row.line, "BTC Amount BTC", s)?.abs(),
                None => 0,
            };
            if sat == 0 {
                out.dropped_no_btc += 1; // no BTC leg (e.g. Credit/Debit USD cash)
                continue;
            }
            let ttype = row.get(SRC, cols::TYPE)?;
            let (utc, tz) = parse_timestamp_flex(SRC, row.line, row.get(SRC, cols::DATE)?)?;
            let txid = row.opt(cols::TX_HASH).map(|s| s.to_string());
            let fee = match row.opt(cols::FEE_USD) {
                Some(s) => parse_usd(SRC, row.line, "Fee (USD) USD", s)?,
                None => Usd::ZERO,
            };
            let usd_amount = row
                .opt(cols::USD_AMOUNT)
                .map(|s| parse_usd(SRC, row.line, "USD Amount USD", s))
                .transpose()?
                .unwrap_or(Usd::ZERO);

            let lower = ttype.to_ascii_lowercase();
            let (dir, payload): (Direction, EventPayload) = match lower.as_str() {
                "buy" => (
                    Direction::Trade,
                    EventPayload::Acquire(Acquire {
                        sat,
                        usd_cost: usd_amount,
                        fee_usd: fee,
                        basis_source: BasisSource::ExchangeProvided,
                    }),
                ),
                "sell" => (
                    Direction::Trade,
                    EventPayload::Dispose(Dispose {
                        sat,
                        usd_proceeds: usd_amount,
                        fee_usd: fee,
                        kind: DisposeKind::Sell,
                    }),
                ),
                "debit" => (
                    Direction::Out,
                    EventPayload::TransferOut(TransferOut {
                        sat,
                        fee_sat: None,
                        dest_addr: row.opt(cols::WITHDRAWAL_DEST).map(|s| s.to_string()),
                        txid: txid.clone(),
                    }),
                ),
                // Credit(BTC) is an inbound on-chain transfer (§9.1 confirmed) → TransferIn.
                "credit" => (
                    Direction::In,
                    EventPayload::TransferIn(TransferIn {
                        sat,
                        src_addr: row.opt(cols::DEPOSIT_DEST).map(|s| s.to_string()),
                        txid: txid.clone(),
                    }),
                ),
                // Any unknown/future BTC-side type → Unclassified (never guess).
                _ => {
                    out.unclassified += 1;
                    (
                        Direction::Trade,
                        EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                    )
                }
            };

            // Native source_ref = `Trade ID`(+`Order ID`) on trade rows; else semantic (Credit/Debit).
            let id_ref = match row.opt(cols::TRADE_ID) {
                Some(tid) => {
                    let combined = match row.opt(cols::ORDER_ID) {
                        Some(oid) => format!("{tid}.{oid}"),
                        None => tid.to_string(),
                    };
                    mint.native(dir, &combined)
                }
                None => {
                    let utc_ms = (utc.unix_timestamp_nanos() / 1_000_000) as i64;
                    mint.semantic(dir, utc_ms, &lower, sat)
                }
            };
            out.events.push(LedgerEvent {
                id: EventId::import(Source::Gemini, id_ref),
                utc_timestamp: utc,
                original_tz: tz,
                wallet: Some(exchange_wallet(Source::Gemini)),
                payload,
            });
        }
        Ok(out)
    }
}
