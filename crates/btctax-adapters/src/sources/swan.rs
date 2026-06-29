//! Swan adapter (§9.1, confirmed per-role schemas). Three files = one batch (trades / transfers /
//! withdrawals), routed to roles by header signature. *trades* (universal Received/Sent, no id,
//! `MM/DD/YYYY`) → Acquire, semantic source_ref. *transfers* (`Event` discriminator, native
//! `Transaction ID`, `…+00`): purchase→Acquire, deposit→TransferIn, monthly_fee/prepaid_fee→
//! Unclassified. *withdrawals* (implicit, `Created At`+`Timezone`, `…+00`) → TransferOut, semantic
//! source_ref (its `Transaction ID` is not a stable per-row id). No on-chain txid column in any role.
//!
//! FOUND GAP: a Swan `transfers` row carries `USD Cost Basis` + `Acquisition Date`, but core's
//! `TransferIn` has no field for either. They are dropped at ingest and must be re-supplied by
//! reconciliation (`ClassifyInbound`) for externally-sourced coins (for self-transfers the source lot
//! is authoritative anyway, §9.1). Logged to FOLLOWUPS as a Phase-1 limitation.
use crate::adapter::{Adapter, FileGroup, GroupOutput, SourceFile};
use crate::normalize::{exchange_wallet, raw_of, Direction, SourceRefMint};
use crate::parse::{parse_btc_to_sat, parse_timestamp, parse_usd};
use crate::read::{peek_text, read_table, RawRow, ReadOpts, TableRole};
use crate::AdapterError;
use btctax_core::{
    Acquire, BasisSource, EventId, EventPayload, LedgerEvent, PriceProvider, Source, SourceRef,
    TransferIn, TransferOut, Unclassified, Usd,
};
use time::{OffsetDateTime, UtcOffset};

const SRC: &str = "swan";
const ASSET_BTC: &str = "BTC";

mod cols {
    // §9.1 CONFIRMED real headers (per role; no OPEN items remain).
    // trades (Role A) — universal Received/Sent shape, empty `Tag`, NO id column:
    pub const T_DATE: &str = "Date";
    pub const T_RECV_QTY: &str = "Received Quantity";
    pub const T_RECV_CUR: &str = "Received Currency";
    pub const T_SENT_QTY: &str = "Sent Quantity";
    pub const T_SENT_CUR: &str = "Sent Currency";
    pub const T_FEE_AMT: &str = "Fee Amount";
    // transfers (Role B):
    pub const X_EVENT: &str = "Event";
    pub const X_DATE: &str = "Date";
    pub const X_TXN_ID: &str = "Transaction ID";
    pub const X_TRANSACTION_USD: &str = "Transaction USD";
    pub const X_FEE_USD: &str = "Fee USD";
    pub const X_UNIT_COUNT: &str = "Unit Count";
    pub const X_ASSET_TYPE: &str = "Asset Type";
    pub const X_USD_COST_BASIS: &str = "USD Cost Basis"; // FOUND GAP — no TransferIn home (dropped)
    #[allow(dead_code)] // FOUND GAP — no TransferIn home; intentionally dropped at ingest
    pub const X_ACQ_DATE: &str = "Acquisition Date";
    // withdrawals (Role C) — implicit type; `Transaction ID` present but NOT a stable per-row id:
    pub const W_CREATED_AT: &str = "Created At";
    pub const W_BTC_AMOUNT: &str = "Bitcoin Amount";
}

/// Route a Swan file to its role by confirmed header signature.
fn role_of(snip: &str) -> Option<TableRole> {
    if snip.contains(cols::X_EVENT) && snip.contains(cols::X_USD_COST_BASIS) {
        Some(TableRole::SwanTransfers)
    } else if snip.contains(cols::T_RECV_QTY) && snip.contains(cols::T_SENT_QTY) {
        Some(TableRole::SwanTrades)
    } else if snip.contains(cols::W_CREATED_AT) && snip.contains(cols::W_BTC_AMOUNT) {
        Some(TableRole::SwanWithdrawals)
    } else {
        None
    }
}

/// Per-role read options — a header-token signature so the reader skips each role's preamble (trades has
/// none; transfers/withdrawals have 2 company lines before the header).
fn opts_for(role: TableRole) -> ReadOpts {
    match role {
        TableRole::SwanTransfers => ReadOpts {
            header_signature: &[cols::X_EVENT, cols::X_TXN_ID, cols::X_USD_COST_BASIS],
            ..Default::default()
        },
        TableRole::SwanTrades => ReadOpts {
            header_signature: &[cols::T_RECV_QTY, cols::T_SENT_QTY],
            ..Default::default()
        },
        TableRole::SwanWithdrawals => ReadOpts {
            header_signature: &[cols::W_CREATED_AT, cols::W_BTC_AMOUNT],
            ..Default::default()
        },
        TableRole::Single => ReadOpts::default(),
    }
}

pub struct Swan;

impl Swan {
    fn mk(
        &self,
        id_ref: SourceRef,
        utc: OffsetDateTime,
        tz: UtcOffset,
        payload: EventPayload,
    ) -> LedgerEvent {
        LedgerEvent {
            id: EventId::import(Source::Swan, id_ref),
            utc_timestamp: utc,
            original_tz: tz,
            wallet: Some(exchange_wallet(Source::Swan)),
            payload,
        }
    }
}

impl Adapter for Swan {
    fn source(&self) -> Source {
        Source::Swan
    }

    fn detect(&self, file: &SourceFile) -> Result<bool, AdapterError> {
        let snip = peek_text(&file.path, 4096)?;
        Ok(role_of(&snip).is_some())
    }

    /// 3 files (or however many Swan files are present) → ONE batch (§9.1).
    fn group(&self, files: Vec<SourceFile>) -> Vec<FileGroup> {
        if files.is_empty() {
            return Vec::new();
        }
        vec![FileGroup {
            source: Source::Swan,
            label: "swan-batch".to_string(),
            files,
        }]
    }

    fn parse(&self, group: &FileGroup) -> Result<Vec<RawRow>, AdapterError> {
        let mut rows = Vec::new();
        for f in &group.files {
            let snip = peek_text(&f.path, 4096)?;
            // M-6: the actual trigger is an unrecognized role, not a missing file — use
            // UnrecognizedSwanRole (renamed from IncompleteSwanBatch).
            let role = role_of(&snip).ok_or_else(|| AdapterError::UnrecognizedSwanRole {
                path: f.path.display().to_string(),
            })?;
            rows.extend(read_table(&f.path, role, SRC, &opts_for(role))?);
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
            match row.role {
                // trades: universal Received/Sent shape; the BTC leg is whichever side is BTC. Trades are
                // buys (BTC received for USD sent); BTC on the sent side (an unexpected disposition) →
                // Unclassified (never guess a sell). No id column → semantic source_ref.
                TableRole::SwanTrades => {
                    let recv_is_btc = row
                        .opt(cols::T_RECV_CUR)
                        .unwrap_or("")
                        .eq_ignore_ascii_case(ASSET_BTC);
                    let sent_is_btc = row
                        .opt(cols::T_SENT_CUR)
                        .unwrap_or("")
                        .eq_ignore_ascii_case(ASSET_BTC);
                    if !recv_is_btc && !sent_is_btc {
                        out.dropped_no_btc += 1;
                        continue;
                    }
                    let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::T_DATE)?)?;
                    let utc_ms = (utc.unix_timestamp_nanos() / 1_000_000) as i64;
                    if recv_is_btc {
                        let sat = parse_btc_to_sat(
                            SRC,
                            row.line,
                            "Received Quantity",
                            row.get(SRC, cols::T_RECV_QTY)?,
                        )?
                        .abs();
                        let cost = match row.opt(cols::T_SENT_QTY) {
                            Some(s) => parse_usd(SRC, row.line, "Sent Quantity", s)?,
                            None => Usd::ZERO,
                        };
                        let fee = match row.opt(cols::T_FEE_AMT) {
                            Some(s) => parse_usd(SRC, row.line, "Fee Amount", s)?,
                            None => Usd::ZERO,
                        };
                        let id_ref = mint.semantic(Direction::Trade, utc_ms, "trade", sat);
                        out.events.push(self.mk(
                            id_ref,
                            utc,
                            tz,
                            EventPayload::Acquire(Acquire {
                                sat,
                                usd_cost: cost,
                                fee_usd: fee,
                                basis_source: BasisSource::ExchangeProvided,
                            }),
                        ));
                    } else {
                        let sat = parse_btc_to_sat(
                            SRC,
                            row.line,
                            "Sent Quantity",
                            row.get(SRC, cols::T_SENT_QTY)?,
                        )?
                        .abs();
                        out.unclassified += 1;
                        let id_ref = mint.semantic(Direction::Trade, utc_ms, "trade", sat);
                        out.events.push(self.mk(
                            id_ref,
                            utc,
                            tz,
                            EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                        ));
                    }
                }
                // transfers: `Event` discriminator; native `Transaction ID` source_ref (dir per Event).
                TableRole::SwanTransfers => {
                    let asset = row.opt(cols::X_ASSET_TYPE).unwrap_or("");
                    if !asset.eq_ignore_ascii_case(ASSET_BTC) {
                        out.dropped_no_btc += 1; // FR2
                        continue;
                    }
                    let sat = parse_btc_to_sat(
                        SRC,
                        row.line,
                        "Unit Count",
                        row.get(SRC, cols::X_UNIT_COUNT)?,
                    )?
                    .abs();
                    let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::X_DATE)?)?;
                    let id = row.get(SRC, cols::X_TXN_ID)?;
                    let event_lower = row.get(SRC, cols::X_EVENT)?.to_ascii_lowercase();
                    let (dir, payload): (Direction, EventPayload) = match event_lower.as_str() {
                        "purchase" => {
                            let cost = match row.opt(cols::X_TRANSACTION_USD) {
                                Some(s) => parse_usd(SRC, row.line, "Transaction USD", s)?,
                                None => Usd::ZERO,
                            };
                            let fee = match row.opt(cols::X_FEE_USD) {
                                Some(s) => parse_usd(SRC, row.line, "Fee USD", s)?,
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
                        // FOUND GAP: USD Cost Basis + Acquisition Date have no home on TransferIn
                        // (dropped; reconciliation re-supplies them for externally-sourced coins).
                        "deposit" => (
                            Direction::In,
                            EventPayload::TransferIn(TransferIn {
                                sat,
                                src_addr: None,
                                txid: None,
                            }),
                        ),
                        // Fee events: could be a BTC spend/disposition OR a USD-only fee — do not assume.
                        "monthly_fee" | "prepaid_fee" => {
                            out.unclassified += 1;
                            (
                                Direction::Trade,
                                EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                            )
                        }
                        _ => {
                            out.unclassified += 1;
                            (
                                Direction::Trade,
                                EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                            )
                        }
                    };
                    out.events
                        .push(self.mk(mint.native(dir, id), utc, tz, payload));
                }
                // withdrawals: implicit TransferOut; `Transaction ID` is not stable → semantic source_ref.
                TableRole::SwanWithdrawals => {
                    let sat = parse_btc_to_sat(
                        SRC,
                        row.line,
                        "Bitcoin Amount",
                        row.get(SRC, cols::W_BTC_AMOUNT)?,
                    )?
                    .abs();
                    if sat == 0 {
                        out.dropped_no_btc += 1; // defensive (Swan is BTC-only)
                        continue;
                    }
                    let (utc, tz) =
                        parse_timestamp(SRC, row.line, row.get(SRC, cols::W_CREATED_AT)?)?;
                    let utc_ms = (utc.unix_timestamp_nanos() / 1_000_000) as i64;
                    let id_ref = mint.semantic(Direction::Out, utc_ms, "withdrawal", sat);
                    out.events.push(self.mk(
                        id_ref,
                        utc,
                        tz,
                        EventPayload::TransferOut(TransferOut {
                            sat,
                            fee_sat: None,
                            dest_addr: None,
                            txid: None,
                        }),
                    ));
                }
                // Unreachable: Swan `parse` always assigns a Swan role (never `Single`).
                TableRole::Single => continue,
            }
        }
        Ok(out)
    }
}
