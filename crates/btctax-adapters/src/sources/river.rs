//! River universal-CSV adapter (§9.1, confirmed schema). CRLF (handled by the reader). Id-less →
//! semantic `source_ref` (`dir|utc_ms|type|sat` + occurrence_index, §6.2). Universal Sent/Received
//! shape: the BTC leg is whichever currency is BTC. `Buy`→Acquire (usd_cost=`Sent Amount`,
//! fee=`Fee Amount`, sat=`Received Amount`); `Income`→Income{Reward} / `Interest`→Income{Interest}
//! (no USD → dataset FMV, FR3; sat=`Received Amount`); `Withdrawal`→TransferOut (sat=`Sent Amount`);
//! unknown `Tag`→Unclassified. FR2: neither currency BTC → drop.
use crate::adapter::{Adapter, FileGroup, GroupOutput, SourceFile};
use crate::normalize::{exchange_wallet, raw_of, resolve_fmv, Direction, SourceRefMint};
use crate::parse::{parse_btc_to_sat, parse_timestamp, parse_usd};
use crate::read::{peek_text, read_table, RawRow, ReadOpts, TableRole};
use crate::AdapterError;
use btctax_core::conventions::tax_date;
use btctax_core::{
    Acquire, BasisSource, EventId, EventPayload, Income, IncomeKind, LedgerEvent, PriceProvider,
    Source, TransferOut, Unclassified, Usd,
};

const SRC: &str = "river";
const ASSET_BTC: &str = "BTC";

mod cols {
    // §9.1 CONFIRMED real headers (no OPEN items remain):
    pub const DATE: &str = "Date";
    pub const SENT_AMOUNT: &str = "Sent Amount";
    pub const SENT_CURRENCY: &str = "Sent Currency";
    pub const RECEIVED_AMOUNT: &str = "Received Amount";
    pub const RECEIVED_CURRENCY: &str = "Received Currency";
    pub const FEE_AMOUNT: &str = "Fee Amount";
    pub const TAG: &str = "Tag";
}

pub struct River;

impl Adapter for River {
    fn source(&self) -> Source {
        Source::River
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
        // River universal Sent/Received `Amount` shape + `Tag` (distinct from Swan's `Quantity`/`Event`
        // and Coinbase's `Transaction Type`/`Subtotal`).
        Ok(snip.contains(cols::SENT_AMOUNT)
            && snip.contains(cols::RECEIVED_AMOUNT)
            && snip.contains(cols::TAG))
    }

    fn group(&self, files: Vec<SourceFile>) -> Vec<FileGroup> {
        files
            .into_iter()
            .map(|f| FileGroup {
                source: Source::River,
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
        prices: &dyn PriceProvider,
    ) -> Result<GroupOutput, AdapterError> {
        let mut mint = SourceRefMint::default();
        let mut out = GroupOutput {
            parsed_rows: rows.len(),
            ..Default::default()
        };
        for row in &rows {
            // FR2: the BTC leg is whichever currency is BTC; if neither, no BTC leg → drop.
            let recv_is_btc = row
                .opt(cols::RECEIVED_CURRENCY)
                .unwrap_or("")
                .eq_ignore_ascii_case(ASSET_BTC);
            let sent_is_btc = row
                .opt(cols::SENT_CURRENCY)
                .unwrap_or("")
                .eq_ignore_ascii_case(ASSET_BTC);
            if !recv_is_btc && !sent_is_btc {
                out.dropped_no_btc += 1;
                continue;
            }
            let sat = if recv_is_btc {
                parse_btc_to_sat(
                    SRC,
                    row.line,
                    "Received Amount",
                    row.get(SRC, cols::RECEIVED_AMOUNT)?,
                )?
                .abs()
            } else {
                parse_btc_to_sat(
                    SRC,
                    row.line,
                    "Sent Amount",
                    row.get(SRC, cols::SENT_AMOUNT)?,
                )?
                .abs()
            };
            let tag = row.get(SRC, cols::TAG)?;
            let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::DATE)?)?;
            let date = tax_date(utc, tz);
            let utc_ms = (utc.unix_timestamp_nanos() / 1_000_000) as i64;
            let lower = tag.to_ascii_lowercase();

            let (dir, payload): (Direction, EventPayload) = match lower.as_str() {
                "buy" => {
                    let cost = match row.opt(cols::SENT_AMOUNT) {
                        Some(s) => parse_usd(SRC, row.line, "Sent Amount", s)?,
                        None => Usd::ZERO,
                    };
                    let fee = match row.opt(cols::FEE_AMOUNT) {
                        Some(s) => parse_usd(SRC, row.line, "Fee Amount", s)?,
                        None => Usd::ZERO,
                    };
                    (
                        Direction::Trade,
                        EventPayload::Acquire(Acquire {
                            sat,
                            usd_cost: cost,
                            fee_usd: fee,
                            basis_source: BasisSource::ExchangeProvided,
                        }),
                    )
                }
                "income" => {
                    let (fmv, status) = resolve_fmv(None, date, sat, prices); // no export USD → dataset
                    (
                        Direction::In,
                        // `business: false` is hard-coded at the adapter layer. To flip this flag
                        // (and optionally the kind) after import, use:
                        //   `reconcile reclassify-income <event_ref> --business true [--kind mining|…]`
                        // SE Chunk C ships this path; professional miners subject to SE-tax should
                        // use it. Voidable and conflict-checked (Hard DecisionConflict on duplicate).
                        EventPayload::Income(Income {
                            sat,
                            usd_fmv: fmv,
                            fmv_status: status,
                            kind: IncomeKind::Reward,
                            business: false,
                        }),
                    )
                }
                "interest" => {
                    let (fmv, status) = resolve_fmv(None, date, sat, prices);
                    (
                        Direction::In,
                        // `business: false` is hard-coded at the adapter layer. To flip this flag
                        // (and optionally the kind) after import, use:
                        //   `reconcile reclassify-income <event_ref> --business true [--kind mining|…]`
                        // SE Chunk C ships this path; professional miners/stakers subject to SE-tax
                        // should use it. Voidable and conflict-checked (Hard DecisionConflict on duplicate).
                        EventPayload::Income(Income {
                            sat,
                            usd_fmv: fmv,
                            fmv_status: status,
                            kind: IncomeKind::Interest,
                            business: false,
                        }),
                    )
                }
                "withdrawal" => (
                    Direction::Out,
                    EventPayload::TransferOut(TransferOut {
                        sat,
                        fee_sat: None,
                        dest_addr: None,
                        txid: None,
                    }),
                ),
                _ => {
                    out.unclassified += 1;
                    (
                        Direction::Trade,
                        EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                    )
                }
            };

            let id_ref = mint.semantic(dir, utc_ms, &lower, sat);
            out.events.push(LedgerEvent {
                id: EventId::import(Source::River, id_ref),
                utc_timestamp: utc,
                original_tz: tz,
                wallet: Some(exchange_wallet(Source::River)),
                payload,
            });
        }
        Ok(out)
    }
}
