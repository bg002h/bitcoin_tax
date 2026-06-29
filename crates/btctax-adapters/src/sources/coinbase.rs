//! Coinbase yearly-CSV adapter (§9.1, confirmed schema). 3-line preamble (found by header-token scan);
//! native `ID` `source_ref` (direction-scoped); gross proceeds in `Subtotal` with `Fees and/or Spread`
//! separate; Buy basis = `Subtotal`(+`Fees`) = `Total (…)`; `Send`/`Withdrawal`→TransferOut (dest =
//! `Recipient Address`); `Receive`→TransferIn (src = `Sender Address`); `Order` + the internal
//! Coinbase↔Coinbase-Pro `Exchange/Pro Deposit/Withdrawal` types + any unknown/future type (the
//! confirmed 2012-2019 vocabulary has NO `Convert`/reward) → `Unclassified` (conservative — never
//! guess). FR2: `Asset`≠BTC dropped. No FMV/`PriceProvider` needed (every event carries its own USD or
//! is a transfer/Unclassified).
use crate::adapter::{Adapter, FileGroup, GroupOutput, SourceFile};
use crate::normalize::{exchange_wallet, raw_of, Direction, SourceRefMint};
use crate::parse::{parse_btc_to_sat, parse_timestamp, parse_usd};
use crate::read::{peek_text, read_table, RawRow, ReadOpts, TableRole};
use crate::AdapterError;
use btctax_core::{
    Acquire, BasisSource, Dispose, DisposeKind, EventId, EventPayload, LedgerEvent, PriceProvider,
    Source, SourceRef, TransferIn, TransferOut, Unclassified, Usd,
};

const SRC: &str = "coinbase";
const ASSET_BTC: &str = "BTC";

mod cols {
    // §9.1 CONFIRMED real headers (no OPEN items remain):
    pub const ID: &str = "ID";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const TX_TYPE: &str = "Transaction Type";
    pub const ASSET: &str = "Asset";
    pub const QTY: &str = "Quantity Transacted";
    pub const SUBTOTAL: &str = "Subtotal";
    #[allow(dead_code)] // Documents the confirmed schema; Buy basis = Subtotal + Fees = Total
    pub const TOTAL: &str = "Total (inclusive of fees and/or spread)";
    pub const FEES: &str = "Fees and/or Spread";
    pub const SENDER_ADDR: &str = "Sender Address";
    pub const RECIPIENT_ADDR: &str = "Recipient Address";
}

fn read_opts() -> ReadOpts {
    ReadOpts {
        // Distinctive header tokens (AND-matched), robust to the 3-line preamble; the `Transactions`
        // preamble line cannot be mistaken for the header (it lacks `Transaction Type`/`Quantity …`).
        header_signature: &[cols::ID, cols::TX_TYPE, cols::QTY],
        ..Default::default()
    }
}

pub struct Coinbase;

impl Coinbase {
    fn event(
        &self,
        id_ref: SourceRef,
        utc: time::OffsetDateTime,
        tz: time::UtcOffset,
        payload: EventPayload,
    ) -> LedgerEvent {
        LedgerEvent {
            id: EventId::import(Source::Coinbase, id_ref),
            utc_timestamp: utc,
            original_tz: tz,
            wallet: Some(exchange_wallet(Source::Coinbase)),
            payload,
        }
    }
}

impl Adapter for Coinbase {
    fn source(&self) -> Source {
        Source::Coinbase
    }

    fn detect(&self, file: &SourceFile) -> Result<bool, AdapterError> {
        if file
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("xlsx"))
            .unwrap_or(false)
        {
            return Ok(false);
        }
        let snip = peek_text(&file.path, 4096)?;
        Ok(snip.contains(cols::TX_TYPE)
            && snip.contains(cols::QTY)
            && snip.contains(cols::SUBTOTAL))
    }

    fn group(&self, files: Vec<SourceFile>) -> Vec<FileGroup> {
        files
            .into_iter()
            .map(|f| FileGroup {
                source: Source::Coinbase,
                label: f.path.display().to_string(),
                files: vec![f],
            })
            .collect()
    }

    fn parse(&self, group: &FileGroup) -> Result<Vec<RawRow>, AdapterError> {
        let opts = read_opts();
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
        let mint = SourceRefMint::default();
        let mut out = GroupOutput {
            parsed_rows: rows.len(),
            ..Default::default()
        };
        for row in &rows {
            // FR2: keep only the BTC leg.
            let asset = row.opt(cols::ASSET).unwrap_or("");
            if !asset.eq_ignore_ascii_case(ASSET_BTC) {
                out.dropped_no_btc += 1;
                continue;
            }
            let ttype = row.get(SRC, cols::TX_TYPE)?;
            let sat = parse_btc_to_sat(
                SRC,
                row.line,
                "Quantity Transacted",
                row.get(SRC, cols::QTY)?,
            )?
            .abs();
            let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::TIMESTAMP)?)?;
            let id = row.get(SRC, cols::ID)?;
            let subtotal = row
                .opt(cols::SUBTOTAL)
                .map(|s| parse_usd(SRC, row.line, "Subtotal", s))
                .transpose()?
                .unwrap_or(Usd::ZERO);
            let fees = match row.opt(cols::FEES) {
                Some(s) => parse_usd(SRC, row.line, "Fees and/or Spread", s)?,
                None => Usd::ZERO,
            };
            let sender = row.opt(cols::SENDER_ADDR).map(|s| s.to_string());
            let recipient = row.opt(cols::RECIPIENT_ADDR).map(|s| s.to_string());

            let (dir, payload): (Direction, EventPayload) =
                match ttype.to_ascii_lowercase().as_str() {
                    "buy" => (
                        Direction::Trade,
                        EventPayload::Acquire(Acquire {
                            sat,
                            usd_cost: subtotal,
                            fee_usd: fees,
                            basis_source: BasisSource::ExchangeProvided,
                        }),
                    ),
                    "sell" => (
                        Direction::Trade,
                        EventPayload::Dispose(Dispose {
                            sat,
                            usd_proceeds: subtotal,
                            fee_usd: fees,
                            kind: DisposeKind::Sell,
                        }),
                    ),
                    "send" | "withdrawal" => (
                        Direction::Out,
                        EventPayload::TransferOut(TransferOut {
                            sat,
                            fee_sat: None,
                            dest_addr: recipient,
                            txid: None,
                        }),
                    ),
                    "receive" => (
                        Direction::In,
                        EventPayload::TransferIn(TransferIn {
                            sat,
                            src_addr: sender,
                            txid: None,
                        }),
                    ),
                    // Internal Coinbase↔Coinbase-Pro moves: likely self-transfers, but user-confirmed → Unclassified.
                    "exchange deposit" | "pro deposit" => {
                        out.unclassified += 1;
                        (
                            Direction::In,
                            EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                        )
                    }
                    "exchange withdrawal" | "pro withdrawal" => {
                        out.unclassified += 1;
                        (
                            Direction::Out,
                            EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                        )
                    }
                    // `Order` = a known Coinbase type (order-book fill; an order may net BTC/USD
                    // differently from a simple Buy/Sell). Explicit arm so the type is documented and
                    // findable by grep — do not rely on the `_` catch-all for a known type (M-4).
                    "order" => {
                        out.unclassified += 1;
                        (
                            Direction::Trade,
                            EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                        )
                    }
                    // Any unknown/future type (incl. the absent `Convert`/reward in the confirmed
                    // 2012-2019 Coinbase vocabulary) → Unclassified (never guess).
                    _ => {
                        out.unclassified += 1;
                        (
                            Direction::Trade,
                            EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                        )
                    }
                };

            let id_ref = mint.native(dir, id);
            out.events.push(self.event(id_ref, utc, tz, payload));
        }
        Ok(out)
    }
}
