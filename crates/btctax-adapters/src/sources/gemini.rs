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
use crate::normalize::{exchange_wallet, raw_of, resolve_fmv, Direction, SourceRefMint};
use crate::parse::{parse_btc_to_sat, parse_timestamp_flex, parse_usd};
use crate::read::{read_table, RawRow, ReadOpts, TableRole};
use crate::AdapterError;
use btctax_core::conventions::tax_date;
use btctax_core::{
    Acquire, BasisSource, Dispose, DisposeKind, EventId, EventPayload, LedgerEvent, PriceProvider,
    Source, TransferIn, TransferOut, Unclassified, Usd,
};

const SRC: &str = "gemini";

mod cols {
    // §9.1 CONFIRMED real headers (no OPEN items remain):
    pub const TYPE: &str = "Type";
    pub const DATE: &str = "Date"; // Excel serial (Time (UTC) carries the same instant)
    pub const SYMBOL: &str = "Symbol"; // trading pair (e.g. "BTCUSD", "ETHBTC") — I-1 gate
                                       // FR-45: the row's own account of WHAT it is. `Type` says only Credit/Debit/Buy/Sell; the
                                       // character (card reward vs ACH deposit vs on-chain deposit) lives here and nowhere else.
    pub const SPECIFICATION: &str = "Specification";
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

/// FR-45 — does this row's `Specification` name a **credit-card reward payout**?
///
/// The confirmed vocabulary in a real export is
/// `Deposit (Gemini Credit Card Reward Payout BTC)`, alongside `Deposit (Instant ACH Transfer)`,
/// `Deposit (ACH Transfer)`, `Deposit (Pre-Credited BTC)` and `Administrative Credit`.
///
/// ★ The match is deliberately on **"credit card reward"** rather than on the full string. Gemini
/// has already varied the trailing asset token (`… Payout BTC`), and a byte-exact match would fall
/// back to the zero-basis TransferIn path — SILENTLY — the first time it varies again. Failing open
/// into the old defect is the outcome this predicate exists to prevent, so it matches the phrase
/// that carries the meaning and ignores the decoration around it.
fn is_card_reward(spec: Option<&str>) -> bool {
    spec.map(str::to_ascii_lowercase)
        .is_some_and(|s| s.contains("credit card reward"))
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
        prices: &dyn PriceProvider,
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
            // ★ FR-45 M-1 — `sat` above is unconditionally `.abs()`ed (file-wide, pre-existing), so
            //   the SIGN has to be captured from the raw cell or it is gone by the time any arm
            //   runs. A negative reward credit is a reversal/clawback, and turning one into a
            //   positive acquisition would put real basis in the pool that the filer never
            //   received — the UNDERSTATEMENT direction, which is the worse one here.
            let btc_is_negative = row
                .opt(cols::BTC_AMOUNT)
                .is_some_and(|s| s.trim_start().starts_with('-'));
            // I-2: abs-normalize fee magnitude at parse time — Type fixes the field's role
            // (fee is always a cost regardless of Gemini's sign convention). Applied only in
            // this Gemini parser; `parse_usd` is unchanged.
            let fee = match row.opt(cols::FEE_USD) {
                Some(s) => parse_usd(SRC, row.line, "Fee (USD) USD", s)?.abs(),
                None => Usd::ZERO,
            };
            // Note: usd_amount is evaluated inside the buy/sell arm so the Symbol gate (I-1)
            // can inspect the raw opt reference before deciding to parse it.

            let lower = ttype.to_ascii_lowercase();
            let (dir, payload): (Direction, EventPayload) = match lower.as_str() {
                "buy" | "sell" => {
                    // I-1: gate Acquire/Dispose on a USD-quoted BTCUSD trade.
                    // A BTC-quoted pair (e.g. ETHBTC, BCHBTC) disposes BTC in the opposite
                    // direction from a naive Type=Buy read, and carries no USD amount → falling
                    // through to usd_cost/proceeds = ZERO would produce a phantom zero-basis lot
                    // or wrong-direction event. Gate on Symbol=="BTCUSD" (case-insensitive) or
                    // USD Amount USD present-and-non-empty as a safety net. Any Buy/Sell that
                    // fails both checks is emitted as Unclassified — never guess direction or basis.
                    let symbol = row.opt(cols::SYMBOL).unwrap_or("").trim();
                    let usd_str = row.opt(cols::USD_AMOUNT);
                    let is_btcusd = symbol.eq_ignore_ascii_case("btcusd");
                    let has_usd = usd_str.is_some(); // opt already filters blank strings
                    if !is_btcusd && !has_usd {
                        // BTC-quoted or crypto-crypto pair → Unclassified; user classifies.
                        out.unclassified += 1;
                        (
                            Direction::Trade,
                            EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                        )
                    } else {
                        // USD-quoted BTCUSD trade confirmed.
                        // I-2: abs-normalize usd magnitude — Type fixes the field's role.
                        let usd_abs = usd_str
                            .map(|s| parse_usd(SRC, row.line, "USD Amount USD", s))
                            .transpose()?
                            .unwrap_or(Usd::ZERO)
                            .abs();
                        if lower == "buy" {
                            (
                                Direction::Trade,
                                EventPayload::Acquire(Acquire {
                                    sat,
                                    usd_cost: usd_abs,
                                    fee_usd: fee, // already abs from computation above
                                    basis_source: BasisSource::ExchangeProvided,
                                }),
                            )
                        } else {
                            (
                                Direction::Trade,
                                EventPayload::Dispose(Dispose {
                                    sat,
                                    usd_proceeds: usd_abs,
                                    fee_usd: fee, // already abs from computation above
                                    kind: DisposeKind::Sell,
                                }),
                            )
                        }
                    }
                }
                "debit" => (
                    Direction::Out,
                    EventPayload::TransferOut(TransferOut {
                        sat,
                        fee_sat: None,
                        dest_addr: row.opt(cols::WITHDRAWAL_DEST).map(|s| s.to_string()),
                        txid: txid.clone(),
                    }),
                ),
                // ★ FR-45 — a Gemini CREDIT CARD REWARD payout, which is NOT an inbound transfer.
                //
                //   `Type` alone cannot tell these apart: every one of them says "Credit". The row's
                //   own `Specification` says which, and until FR-45 this adapter never read that
                //   column — so a reward became a TransferIn, took the inbound self-transfer path,
                //   and landed at ZERO basis. Two structural facts contradict that reading and both
                //   are on the row: a reward carries NO `Tx Hash` and NO `Deposit Destination`,
                //   while a genuine on-chain deposit carries both. A credit with no txid is not an
                //   on-chain transfer and cannot be "my own coins returning".
                //
                //   TREATMENT (owner determination 2026-09-05, and the general rule for a reward
                //   earned by SPENDING): a card reward is a purchase-price REBATE — not gross income
                //   at receipt — and the coins take basis = FMV at receipt. So it is an `Acquire`
                //   whose `usd_cost` IS that FMV, tagged `CardRewardRebate`. Booking it as `Income`
                //   would tax a rebate; leaving it a `TransferIn` gives it no basis at all.
                //
                //   ★ Gemini states NO USD value on these rows (measured: EVERY reward row in a real export has an empty
                //   `USD Amount USD`), so the FMV can only come from the price dataset — and when
                //   the dataset cannot price the day, this REFUSES to guess and emits Unclassified
                //   for the filer to classify. A fabricated basis is the defect this item is about;
                //   substituting a different fabricated basis would not be a fix.
                "credit" if is_card_reward(row.opt(cols::SPECIFICATION)) => {
                    let date = tax_date(utc, tz);
                    // M-4 / FR3 precedence: prefer the export's OWN stated USD over the dataset
                    // close. Measured blank on every reward row of a real export, so this is dormant today —
                    // but the alternative is discarding the exchange's own figure if it ever
                    // appears, and FR3 says the export wins.
                    let export_usd = match row.opt(cols::USD_AMOUNT) {
                        Some(v) => Some(parse_usd(SRC, row.line, "USD Amount USD", v)?.abs()),
                        None => None,
                    };
                    match resolve_fmv(export_usd, date, sat, prices) {
                        // M-1: a negative reward credit is a reversal, not an acquisition. Refuse
                        // rather than inherit the file-wide `.abs()` and mint basis from a clawback.
                        _ if btc_is_negative => {
                            out.unclassified += 1;
                            (
                                Direction::In,
                                EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                            )
                        }
                        (Some(fmv), _) => (
                            Direction::In,
                            EventPayload::Acquire(Acquire {
                                sat,
                                usd_cost: fmv,
                                // I-2: CARRY the stated fee. Measured blank on every reward row of a real
                                // export, so this is normally zero — but hardcoding the zero
                                // discarded a real figure, and a discarded acquisition fee
                                // understates basis and overstates the later gain.
                                fee_usd: fee,
                                basis_source: BasisSource::CardRewardRebate,
                            }),
                        ),
                        (None, _) => {
                            // No price for the day ⇒ no defensible basis. Never guess.
                            out.unclassified += 1;
                            (
                                Direction::In,
                                EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                            )
                        }
                    }
                }
                // ★★ FR-45 I-1 — a Credit is a TransferIn only on POSITIVE on-chain EVIDENCE.
                //
                //   `row.opt` returns `None` for a column that is absent OR blank, so the reward
                //   guard above cannot distinguish "not a reward" from "the Specification column is
                //   missing entirely". Falling through to TransferIn in the second case silently
                //   restored the exact zero-basis defect FR-45 exists to remove — no error, no
                //   counter, no test. An adversarial review proved it by execution.
                //
                //   A `Tx Hash` or a `Deposit Destination` is affirmative evidence that coins
                //   arrived on-chain; without either, this row could be a reward, a rebate, an
                //   administrative credit or a deposit, and guessing "deposit" is what assigns zero
                //   basis. So: evidence ⇒ TransferIn, no evidence ⇒ Unclassified, never a guess.
                "credit" if txid.is_some() || row.opt(cols::DEPOSIT_DEST).is_some() => (
                    Direction::In,
                    EventPayload::TransferIn(TransferIn {
                        sat,
                        src_addr: row.opt(cols::DEPOSIT_DEST).map(|s| s.to_string()),
                        txid: txid.clone(),
                    }),
                ),
                // I-1 fall-through: a BTC Credit with neither a reward specification nor any
                // on-chain marker. Its character is genuinely unknown — refuse, do not default.
                "credit" => {
                    out.unclassified += 1;
                    (
                        Direction::In,
                        EventPayload::Unclassified(Unclassified { raw: raw_of(row) }),
                    )
                }
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
