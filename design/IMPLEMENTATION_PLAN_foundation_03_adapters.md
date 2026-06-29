# btctax-adapters (Ingest: Exchange Parsers + Price Dataset) Implementation Plan — Foundation Plan 3 of 4 (v1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `btctax-adapters`, the **ingest** crate: the four exchange-export parsers (Coinbase, Gemini, River, Swan) and the bundled daily-close BTC/USD price dataset. It turns on-disk exports into `btctax_core::LedgerEvent`s by the §9 pipeline — **detect → group (Swan) → strip preamble (CRLF) → parse → normalize → `source_ref` → (FR2 filter) → (FR3 FMV)** — and implements core's `PriceProvider` trait over the bundled dataset (§9.2). It produces events; it does **not** persist them (the CLI, Plan 4, calls `btctax_core::persistence::append_import_batch`). BTC-only (FR2): rows with no BTC leg are dropped+counted; ambiguous BTC-side rows become `Unclassified` events (never dropped). Exact arithmetic only (NFR5): money is parsed from decimal strings into `Decimal`, BTC into integer satoshis — **no float parsing of money anywhere.**

**Architecture:** Three layers with a hard boundary.
1. **Format-agnostic reading (`read`).** A `RawRow` is a header→cell **string** map; both CSV (the `csv` crate) and XLSX (`calamine`) reduce to it. The reader strips a CSV preamble (by scanning for the documented header tokens, robust to preamble-length drift) and tolerates CRLF; XLSX numeric cells (IEEE-754 doubles in the file format) are rendered with Rust's shortest-round-trip `{}` formatting, which reproduces the intended ≤8-dp exchange decimal exactly, then parsed by the exact decimal parser (NFR5 — documented bound, FOLLOWUPS).
2. **Per-source parsers (`sources::{coinbase,gemini,river,swan}`).** Each is a unit struct implementing the `Adapter` trait (`detect`/`group`/`parse`/`normalize`). The only source-specific knowledge is (a) a `mod cols` block of header-name constants and (b) a `match` over the documented §9.1 type→event mapping. Everything else (decimal/BTC/date parsing, FMV resolution, `source_ref` synthesis, wallet construction) is shared.
3. **Orchestration (`ingest`).** `ingest_files(paths, &dyn PriceProvider)` detects each file's source, groups (Swan's three files → one batch), dispatches to the parser, and aggregates FR2 counts into a `FileReport` per group. `ingest_files_bundled(paths)` wires the bundled `PriceProvider` as the FR3 default.

**The adapters PRODUCE, never project.** Output is `Vec<LedgerEvent>` (+ reports). The events carry stable `EventId::Import { source, source_ref }` so the CLI's `append_import_batch` gets idempotency + `ImportConflict` detection for free (Plan 2 owns that). No projection, no I/O beyond reading the named input files + the bundled dataset.

**Tech Stack:** Rust (edition 2021, rust-version 1.74 — workspace pins), `btctax-core` (path dep; the event model + `PriceProvider` + `fmv_of` + `tax_date`), `rust_decimal` (exact money; NFR5 — same pin as core), `time` (timestamp parse → `OffsetDateTime` + `UtcOffset`; §6.1 dates), `csv` 1.3 (RFC4180 reader; Coinbase/River CSV; CRLF + preamble), `calamine` 0.26 (read-only XLSX; Gemini ledger + any `.xlsx` source), `thiserror` (typed errors with file/row/column context, matching `btctax-store`/`btctax-core`). Dev: `rust_decimal_macros` (`dec!`), `time` `macros`, `rust_xlsxwriter` (build **synthetic** `.xlsx` fixtures in-test — it only ever writes invented data; it never reads a real export), `tempfile`.

## Global Constraints
(Spec `design/SPEC_foundation.md`; every task implicitly includes these. Values are verbatim from the spec.)

- **PRIVACY (CRITICAL).** The real exchange exports live in `~/Documents/BitcoinTax/ReadOnly` — **OUTSIDE this repo**. They MUST NEVER be read by this crate, its tests, this plan, or any tool invocation, and MUST NEVER be committed. **Every test uses SYNTHETIC fixtures** that use the **confirmed real §9.1 header names** with **invented values only** (built in-test as CSV strings / `rust_xlsxwriter` workbooks, or committed under `crates/btctax-adapters/tests/fixtures/`). The §9.1 schemas (headers + type enums + timestamp formats) are confirmed schema-only (no PII), so each parser's `mod cols` constants are the real header names — **no `// OPEN` placeholders remain**; the only residual unknowns are the data-level owner questions in **Schema items** below. Do **not** invent or hard-code any value content from a real export.
- **FR2 BTC-only filter (verbatim).** "Drop a row **only if it has no BTC leg**. Any BTC leg is retained (a crypto↔BTC trade = BTC disposition/acquisition at FMV; non-BTC leg ignored). **Unknown/ambiguous BTC-side rows → `Unclassified` (blocker), never dropped.** Report dropped (no-BTC) + unclassified counts per file."
- **FR3 FMV resolution (verbatim).** "Prefer export USD; else dataset; else `Missing` (blocker)." → `FmvStatus::ExchangeProvided` / `PriceDataset` / `Missing`. (`ManualEntry` is never produced at ingest; it arises only from a `ManualFmv` decision in core/CLI.)
- **NFR5 Exact arithmetic — no floats anywhere.** Money is `Usd = rust_decimal::Decimal`, parsed from the export's decimal **string**. BTC is converted to integer satoshis `Sat = i64` via `Decimal`-exact `× 100_000_000`; a fractional satoshi is an error, never a silent round. XLSX numeric cells are stringified via shortest-round-trip `{}` then parsed exactly (documented bound; FOLLOWUPS).
- **§9.2 Price dataset.** "Bundled daily BTC/USD behind `PriceProvider` (trait in core). **Daily close** = documented FMV convention." Stored as a bundled CSV keyed by calendar date; `BundledPrices` implements `btctax_core::PriceProvider` over it. Exact-date lookup (BTC has a close every calendar day); a missing date → `None` → FR3 `Missing`.
- **§6.2 `source_ref`.** Stable real-world-row identity scoped by `(source, direction)`. **Native id where present** (Coinbase `ID`, Gemini `Trade ID`, Swan `Transaction ID`). **Id-less sources (River):** `(source, direction, utc_ms, type, sat)` + last-resort `occurrence_index` (file-order fragility already in FOLLOWUPS). `txid` is a cross-source **match signal — NOT a global dedup key**.
- **§9 Adapter trait.** `detect`/`group`/`parse`/`normalize`. Each source module's doc-comment + a fixture test states: its `source_ref`/dedup, gross-vs-net proceeds, fee placement, and unknown-type → `Unclassified`.
- **Out of scope (this crate).** Projection / lot engine / `verify` / reconciliation / CLI / persistence-append (those are Plans 2/4); non-BTC assets, the optimizer, forms (other phases). The adapter only *produces* events.
- **Licensing:** workspace `license = "MIT OR Unlicense"`; `edition = "2021"`; `rust-version = "1.74"`.
- **Validation gate ("green"):** `cargo test -p btctax-adapters` + `cargo clippy --all-targets -p btctax-adapters -- -D warnings` + `cargo fmt --check` all green; plus 0 Critical / 0 Important on review.

## Schema items — CONFIRMED 2026-06-29 (privacy: real exports still NOT read; values invented)
The real export schemas (column headers + type enums + timestamp formats, schema-only / no PII) are now **confirmed** and folded into spec §9.1 and into each parser's `mod cols` block — **there are no `// OPEN` constants left**; every header constant is the real header name. Fixtures remain **synthetic** (real header names, invented values; PRIVACY constraint unchanged). The confirmed shapes per source:

- **Coinbase.** 13 cols (`ID, Timestamp, Transaction Type, Asset, Quantity Transacted, Price Currency, Price at Transaction, Subtotal, Total (inclusive of fees and/or spread), Fees and/or Spread, Notes, Sender Address, Recipient Address`); 3-line preamble (header line 4); `Timestamp` = `YYYY-MM-DD HH:MM:SS UTC`. `Transaction Type` enum (10): Buy, Sell, Send, Receive, Withdrawal, Order, Exchange Deposit, Exchange Withdrawal, Pro Deposit, Pro Withdrawal. **No `Convert` and no reward/income type** in the 2012-2019 vocabulary → those (and any future/unknown type) map to `Unclassified`. `Order` + the four `Exchange/Pro Deposit/Withdrawal` internal-move types → `Unclassified` (likely self-transfers, user-confirmed via reconciliation).
- **Gemini.** 30 cols; `Type` enum (4): Buy, Sell, Credit, Debit; BTC leg = `BTC Amount BTC`; cost/proceeds `USD Amount USD` + `Fee (USD) USD`; ids `Trade ID`+`Order ID` (trade rows) — Credit/Debit lack them → semantic `source_ref`; `Tx Hash` = txid; `Deposit/Withdrawal Destination` = address. `Date`/`Time (UTC)` are **Excel serial numbers** (numeric) → `parse_timestamp_flex` converts the serial. **`Credit`(BTC)→`TransferIn`, `Debit`(BTC)→`TransferOut`** (per §9.1; supersedes the earlier Credit→Unclassified); USD-cash Credit/Debit (no BTC leg) dropped (FR2).
- **River.** 8 cols (`Date, Sent Amount, Sent Currency, Received Amount, Received Currency, Fee Amount, Fee Currency, Tag`); `Tag` enum (4): Buy, Income, Interest, Withdrawal; universal Sent/Received shape (BTC leg = whichever currency is BTC); `Date` = naive `YYYY-MM-DD HH:MM:SS` (UTC). `Income`→`Income{Reward}`, `Interest`→`Income{Interest}` (dataset FMV). Id-less → semantic `source_ref`.
- **Swan.** **3 files = one batch**, routed to roles by header signature (CSV; reader dispatches on extension so XLSX would also work). *trades* (8 cols, universal Sent/Received, empty `Tag`, no id, `Date`=`MM/DD/YYYY HH:MM:SS`) → `Acquire`. *transfers* (14 cols, `Event` enum deposit/purchase/monthly_fee/prepaid_fee, native `Transaction ID`, `Date`=`…+00`+`Timezone`): purchase→Acquire, deposit→TransferIn, monthly_fee/prepaid_fee→Unclassified. *withdrawals* (9 cols, implicit type, `Created At`=`…+00`+`Timezone`, BTC = `Bitcoin Amount`) → `TransferOut`. **No on-chain txid column in any Swan role** (`txid`=None).
- **REMAINING owner questions (data-level, not column names):**
  - **Swan withdrawals `source_ref`.** The withdrawals file carries a `Transaction ID` column, but per the owner it is **not a stable per-row id** (schema-only doc cannot show values; cf. Swan-trades' empty `Tag`), so withdrawals is treated as **id-less** (semantic `source_ref`) — confirm whether the withdrawals `Transaction ID` is in fact stable/unique (if so, switch to native, a one-line change).
  - **Swan trades `Total/Transaction USD` vs purchase cost.** trades→Acquire uses `Sent Quantity` as USD cost; transfers `purchase`→Acquire uses `Transaction USD` (principal) + `Fee USD` (fee), with `Total USD` as the basis check. Confirm `Total USD == Transaction USD + Fee USD` by fixture once real values are available.
  - **Coinbase internal-move treatment.** `Exchange/Pro Deposit/Withdrawal` are routed to `Unclassified` (likely Coinbase↔Coinbase-Pro self-transfers) pending the user's reconciliation classification — confirm this is the desired default vs auto-`TransferIn`/`TransferOut`.
- **FOUND GAP (cross-crate, not a column).** Swan `transfers` carry `USD Cost Basis` + `Acquisition Date`, but core's `TransferIn { sat, src_addr?, txid? }` has **no field to hold a basis/date**. At ingest the transfer becomes a plain `TransferIn`; the Swan-provided basis/date are dropped from the event and must be re-supplied by reconciliation (`ClassifyInbound`) for externally-sourced coins (for self-transfers the source lot is authoritative anyway, §9.1). This is surfaced (Task 8) and logged to FOLLOWUPS as a Phase-1 limitation / candidate for a reconciliation-hints side-table.

## File Structure
```
Cargo.toml                          # [workspace] root — ADD "crates/btctax-adapters" to members
crates/btctax-adapters/
  Cargo.toml                        # pinned deps (btctax-core, rust_decimal, time, csv, calamine, thiserror)
  data/btc_usd_daily_close.csv      # bundled daily-close dataset (public price data; date,usd_close)
  src/lib.rs                        # pub API + AdapterError + module wiring + re-exports
  src/parse.rs                      # NFR5 primitives: parse_usd, parse_btc_to_sat, parse_timestamp
  src/price.rs                      # BundledPrices: PriceProvider over the bundled CSV (§9.2)
  src/read.rs                       # RawRow + read_csv/read_xlsx/read_table, TableRole, ReadOpts, peek_text
  src/normalize.rs                  # resolve_fmv (FR3), SourceRefMint (§6.2), Direction, exchange_wallet, raw_of
  src/adapter.rs                    # Adapter trait + SourceFile/FileGroup/GroupOutput/FileReport/IngestBatch
  src/sources/coinbase.rs          # §9.1 Coinbase parser
  src/sources/gemini.rs            # §9.1 Gemini parser
  src/sources/river.rs             # §9.1 River parser
  src/sources/swan.rs              # §9.1 Swan parser (3-file group)
  src/ingest.rs                     # detect→group→dispatch→IngestBatch; FR2 reporting; FR3 default wiring
  tests/fixtures/                   # SYNTHETIC fixtures only (invented names/values)
  tests/coinbase.rs                 # per-source fixture test (mapping/source_ref/FR2/preamble)
  tests/gemini.rs
  tests/river.rs
  tests/swan.rs
  tests/fmv_fr3.rs                  # FR3 matrix end-to-end (ExchangeProvided/PriceDataset/Missing)
  tests/integration.rs             # multi-source synthetic batch → events (counts/kinds/refs/FMV)
```

**Public interface this plan PRODUCES (consumed by Plan 4 `btctax-cli`):**
- `pub enum AdapterError` (`thiserror`): `Io`, `Csv`, `Xlsx`, `EmptyXlsx`, `MissingColumn`, `Parse`, `FractionalSat`, `UnknownSource`, `UnrecognizedSwanRole`, `PriceDataset`.
- `pub struct BundledPrices` (`impl btctax_core::PriceProvider`): `BundledPrices::load() -> Result<Self, AdapterError>`, `from_csv_str(&str)`.
- `pub trait Adapter` (`source`/`detect`/`group`/`parse`/`normalize`) + the four unit structs `Coinbase`/`Gemini`/`River`/`Swan`.
- `pub struct SourceFile`, `FileGroup`, `GroupOutput`, `FileReport`, `IngestBatch`.
- `pub fn ingest_files(paths: &[PathBuf], prices: &dyn PriceProvider) -> Result<IngestBatch, AdapterError>`.
- `pub fn ingest_files_bundled(paths: &[PathBuf]) -> Result<IngestBatch, AdapterError>` — FR3-wired (bundled dataset).
- Shared (also `pub` for reuse/testing): `parse::{parse_usd, parse_btc_to_sat, parse_timestamp, SATS_PER_BTC}`, `read::{RawRow, TableRole, ReadOpts, read_table}`, `normalize::{resolve_fmv, SourceRefMint, Direction, exchange_wallet}`.

---

### Task 0: Workspace member + crate scaffold + NFR5 numeric/date parse primitives

**Files:** Modify root `Cargo.toml` (members). Create `crates/btctax-adapters/Cargo.toml`, `src/lib.rs`, `src/parse.rs`.

**Interfaces — Produces:** the pinned `csv`/`calamine` versions; `AdapterError`; `parse::{SATS_PER_BTC, BTC_DP, USD_DP, parse_usd, parse_btc_to_sat, parse_timestamp}` (exact, no float money).

- [ ] **Step 1: Add the workspace member.** Edit root `Cargo.toml` `members` to `["crates/btctax-store", "crates/btctax-core", "crates/btctax-adapters"]` (preserve `[workspace.package]`).

- [ ] **Step 2: `crates/btctax-adapters/Cargo.toml`**
```toml
[package]
name = "btctax-adapters"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
btctax-core = { path = "../btctax-core" }
# Exact decimal money (NFR5); same pin as core. serde-str unused here but keeps the feature set aligned.
rust_decimal = { version = "1.36", default-features = false, features = ["std"] }
# Timestamp parse → OffsetDateTime + UtcOffset (§6.1).
time = { version = "0.3", features = ["macros", "parsing", "formatting"] }
csv = "1.3"            # RFC4180 reader; Coinbase/River CSV; CRLF + preamble handling
calamine = "0.26"      # read-only XLSX reader (Gemini ledger; any .xlsx source)
thiserror = "1"

[dev-dependencies]
rust_decimal_macros = "1.36"
rust_xlsxwriter = "0.79"   # build SYNTHETIC .xlsx fixtures in-test (writes invented data only)
tempfile = "3"
```
*(Pin note, mirroring the store's R3 discipline: after the first `cargo build`, record the exact resolved `csv`/`calamine`/`rust_xlsxwriter` versions in FOLLOWUPS. If a cited symbol differs from the pin — `calamine::{open_workbook, Data, Range, Reader, Xlsx}`, `Xlsx::worksheet_range_at`, the `Data` enum variants, `csv::ReaderBuilder::flexible`, `rust_xlsxwriter::Workbook` — fix to the compiler before proceeding. **First-build verification checklist (M-3/M-7):** (a) Confirm calamine 0.26's `Data` variant list: verify `DateTime`, `DateTimeIso`, and `DurationIso` exist; delete any arm from `read::cell_to_string` that does not appear in the resolved 0.26 enum. (b) Confirm the `ExcelDateTime` serial accessor in `Data::DateTime(dt)`: try `dt.as_f64()` first (most likely in 0.26); if the accessor name differs, update `cell_to_string`'s `DateTime` arm accordingly; if `Data::DateTime` itself does not exist in 0.26, delete the arm. (c) Confirm `Decimal::from_scientific` exists in rust_decimal 1.36: if it is absent (or its functionality is already covered by `Decimal::from_str` in 1.x), remove the `.or_else(|_| Decimal::from_scientific(&cleaned))` fallback from `parse_usd` in Task 0 Step 6 — `Decimal::from_str` already handles scientific notation in rust_decimal 1.x.)*

- [ ] **Step 3: Stub `src/lib.rs`**
```rust
//! btctax-adapters: exchange-export parsers + the bundled daily-close price dataset (§9).
//! Parses Coinbase/Gemini/River/Swan exports into `btctax_core::LedgerEvent`s — BTC-only (FR2),
//! ingest-time FMV (FR3) over the bundled `PriceProvider` (§9.2). Exact arithmetic only (NFR5).
//!
//! PRIVACY: only SYNTHETIC fixtures are used in tests; the real exports in
//! ~/Documents/BitcoinTax/ReadOnly are NEVER read by this crate or its tests.
pub mod parse;

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("io reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("csv {path}: {source}")]
    Csv {
        path: String,
        #[source]
        source: csv::Error,
    },
    #[error("xlsx {path}: {source}")]
    Xlsx {
        path: String,
        #[source]
        source: calamine::Error,
    },
    /// Used by `read_xlsx` when the workbook has no worksheet (calamine returns `None`, not an error).
    /// Distinct from `Xlsx` (which wraps a calamine error) so callers can pattern-match the cause.
    #[error("xlsx {path}: no worksheet found")]
    EmptyXlsx { path: String },
    #[error("{source} row {line}: missing required column {column:?}")]
    MissingColumn {
        source: &'static str,
        line: usize,
        column: String,
    },
    #[error("{source} row {line}: cannot parse {field} from {value:?}: {reason}")]
    Parse {
        source: &'static str,
        line: usize,
        field: &'static str,
        value: String,
        reason: String,
    },
    #[error("{source} row {line}: fractional satoshi in BTC amount {value:?}")]
    FractionalSat {
        source: &'static str,
        line: usize,
        value: String,
    },
    #[error("unrecognized file (no adapter matched): {path}")]
    UnknownSource { path: String },
    /// A file was detected as Swan (matched at least one role signature) but its header did not
    /// match any of the three confirmed roles (trades / transfers / withdrawals). The actual trigger
    /// is an unrecognized role, not a missing file — hence the rename from `IncompleteSwanBatch`.
    #[error("unrecognized Swan file role (header did not match trades/transfers/withdrawals): {path}")]
    UnrecognizedSwanRole { path: String },
    #[error("price dataset: {0}")]
    PriceDataset(String),
}
```

- [ ] **Step 4: Failing tests in `src/parse.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::{datetime, offset};

    #[test]
    fn parses_usd_exactly_no_float() {
        assert_eq!(parse_usd("t", 1, "f", "1234.56").unwrap(), dec!(1234.56));
        assert_eq!(parse_usd("t", 1, "f", "$1,234.56").unwrap(), dec!(1234.56));
        assert_eq!(parse_usd("t", 1, "f", " 0.10 ").unwrap(), dec!(0.10));
        assert_eq!(parse_usd("t", 1, "f", "(2.50)").unwrap(), dec!(-2.50)); // accounting negative
        assert_eq!(parse_usd("t", 1, "f", "").unwrap(), dec!(0));
    }

    #[test]
    fn btc_to_sat_is_exact_integer() {
        assert_eq!(parse_btc_to_sat("t", 1, "f", "1").unwrap(), 100_000_000);
        assert_eq!(parse_btc_to_sat("t", 1, "f", "0.00000001").unwrap(), 1);
        assert_eq!(parse_btc_to_sat("t", 1, "f", "0.12345678 BTC").unwrap(), 12_345_678);
        assert_eq!(parse_btc_to_sat("t", 1, "f", "-0.5").unwrap(), -50_000_000); // signed; callers .abs()
    }

    #[test]
    fn fractional_satoshi_is_an_error_never_a_silent_round() {
        let e = parse_btc_to_sat("river", 7, "amount", "0.000000001").unwrap_err();
        assert!(matches!(e, AdapterError::FractionalSat { line: 7, .. }));
    }

    #[test]
    fn timestamp_rfc3339_keeps_offset_then_normalizes_to_utc() {
        let (utc, tz) = parse_timestamp("t", 1, "2025-03-01T20:30:00-05:00").unwrap();
        assert_eq!(utc, datetime!(2025-03-02 01:30:00 UTC));
        assert_eq!(tz, offset!(-05:00));
    }

    #[test]
    fn timestamp_naive_assumed_utc() {
        let (utc, tz) = parse_timestamp("t", 1, "2025-03-01 12:00:00").unwrap();
        assert_eq!(utc, datetime!(2025-03-01 12:00:00 UTC));
        assert_eq!(tz, offset!(+00:00));
        let (utc2, _) = parse_timestamp("t", 1, "2025-03-01").unwrap();
        assert_eq!(utc2, datetime!(2025-03-01 00:00:00 UTC));
    }

    #[test]
    fn timestamp_confirmed_export_formats() {
        // Coinbase: trailing " UTC".
        let (utc, tz) = parse_timestamp("coinbase", 1, "2025-03-01 12:00:00 UTC").unwrap();
        assert_eq!((utc, tz), (datetime!(2025-03-01 12:00:00 UTC), offset!(+00:00)));
        // Swan transfers/withdrawals: `YYYY-MM-DD HH:MM:SS+00` (space sep, short offset).
        let (utc, tz) = parse_timestamp("swan", 1, "2025-03-02 09:00:00+00").unwrap();
        assert_eq!((utc, tz), (datetime!(2025-03-02 09:00:00 UTC), offset!(+00:00)));
        // Swan trades: US-locale MM/DD/YYYY, assumed UTC.
        let (utc, tz) = parse_timestamp("swan", 1, "03/01/2025 12:00:00").unwrap();
        assert_eq!((utc, tz), (datetime!(2025-03-01 12:00:00 UTC), offset!(+00:00)));
    }

    #[test]
    fn excel_serial_and_flex_parse() {
        // Anchor: serial 25569 = the Unix epoch; the fraction is the time of day.
        assert_eq!(excel_serial_to_utc(25569.0), datetime!(1970-01-01 00:00:00 UTC));
        assert_eq!(excel_serial_to_utc(25569.5), datetime!(1970-01-01 12:00:00 UTC));
        // Gemini stores Date/Time as numeric serials; flex parse converts them at UTC.
        let (utc, tz) = parse_timestamp_flex("gemini", 1, "25569.5").unwrap();
        assert_eq!((utc, tz), (datetime!(1970-01-01 12:00:00 UTC), offset!(+00:00)));
        // flex still handles ISO text (used by the synthetic Gemini fixtures).
        let (utc, _) = parse_timestamp_flex("gemini", 1, "2025-03-01 12:00:00").unwrap();
        assert_eq!(utc, datetime!(2025-03-01 12:00:00 UTC));
    }
}
```

- [ ] **Step 5: Run → FAIL.** `cargo test -p btctax-adapters parse`

- [ ] **Step 6: Implement `src/parse.rs`**
```rust
//! Exact numeric/date parse primitives (NFR5: NO float parsing of money). Shared by every parser.
use crate::AdapterError;
use btctax_core::{Sat, Usd};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::macros::{datetime, format_description};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, UtcOffset};

/// Satoshis per whole BTC.
pub const SATS_PER_BTC: i64 = 100_000_000;
/// BTC decimal places (1 sat = 1e-8 BTC).
pub const BTC_DP: u32 = 8;
/// USD decimal places (the cent).
pub const USD_DP: u32 = 2;

/// Parse a USD money string EXACTLY (NFR5). Strips `$`, thousands `,`, surrounding whitespace, and a
/// parenthesized accounting negative `(1.23)`. An empty/blank string is `0`. Never uses float.
pub fn parse_usd(
    source: &'static str,
    line: usize,
    field: &'static str,
    raw: &str,
) -> Result<Usd, AdapterError> {
    let t = raw.trim();
    let (neg, body) = match t.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
        Some(inner) => (true, inner),
        None => (false, t),
    };
    let cleaned: String = body
        .chars()
        .filter(|c| !matches!(c, '$' | ',' | ' ' | '\u{a0}'))
        .collect();
    if cleaned.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let mut d = Decimal::from_str(&cleaned)
        .or_else(|_| Decimal::from_scientific(&cleaned))
        .map_err(|e| AdapterError::Parse {
            source,
            line,
            field,
            value: raw.to_string(),
            reason: e.to_string(),
        })?;
    if neg {
        d.set_sign_negative(true);
    }
    Ok(d)
}

/// Parse a BTC amount string → integer satoshis EXACTLY (NFR5). Keeps sign (callers `.abs()` for the
/// payload `sat`; the sign is available to disambiguate a signed/directional amount if a source needs
/// it). A value with finer-than-satoshi precision is a `FractionalSat` error, never a silent round.
pub fn parse_btc_to_sat(
    source: &'static str,
    line: usize,
    field: &'static str,
    raw: &str,
) -> Result<Sat, AdapterError> {
    let t = raw.trim();
    let cleaned: String = t
        .chars()
        .filter(|c| !matches!(c, ',' | ' ' | '\u{a0}' | '\u{20bf}'))
        .collect();
    let body = cleaned
        .strip_suffix("BTC")
        .or_else(|| cleaned.strip_suffix("btc"))
        .unwrap_or(&cleaned)
        .trim();
    if body.is_empty() {
        return Ok(0);
    }
    let btc = Decimal::from_str(body).map_err(|e| AdapterError::Parse {
        source,
        line,
        field,
        value: raw.to_string(),
        reason: e.to_string(),
    })?;
    let sats = btc * Decimal::from(SATS_PER_BTC);
    if !sats.fract().is_zero() {
        return Err(AdapterError::FractionalSat {
            source,
            line,
            value: raw.to_string(),
        });
    }
    sats.trunc().to_i64().ok_or_else(|| AdapterError::Parse {
        source,
        line,
        field,
        value: raw.to_string(),
        reason: "satoshi value out of i64 range".to_string(),
    })
}

/// Parse a timestamp → (UTC instant, original_tz). Handles every confirmed §9.1 export format:
/// RFC3339 (keeps the source offset → `original_tz`); Coinbase `YYYY-MM-DD HH:MM:SS UTC`; Swan
/// transfers/withdrawals `YYYY-MM-DD HH:MM:SS+00` (space separator + short numeric offset, `Timezone`
/// col confirms); Swan trades `MM/DD/YYYY HH:MM:SS` (US-locale, assumed UTC); River naive
/// `YYYY-MM-DD HH:MM:SS` (assumed UTC); bare `YYYY-MM-DD`. Gemini's Excel-serial cells go through
/// `parse_timestamp_flex`. (NFR5 bars float *money*, not timestamps.)
pub fn parse_timestamp(
    source: &'static str,
    line: usize,
    raw: &str,
) -> Result<(OffsetDateTime, UtcOffset), AdapterError> {
    let t = raw.trim();
    // 1. RFC3339 (offset or `Z`) — keeps the source offset as `original_tz` (§6.1).
    if let Ok(odt) = OffsetDateTime::parse(t, &Rfc3339) {
        return Ok((odt.to_offset(UtcOffset::UTC), odt.offset()));
    }
    let dt_fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    // 2. Coinbase: trailing ` UTC` → naive instant at UTC.
    if let Some(stripped) = t.strip_suffix(" UTC").or_else(|| t.strip_suffix(" utc")) {
        if let Ok(pdt) = PrimitiveDateTime::parse(stripped.trim(), &dt_fmt) {
            return Ok((pdt.assume_utc(), UtcOffset::UTC));
        }
    }
    // 3. Swan transfers/withdrawals: `YYYY-MM-DD HH:MM:SS+00` (space separator, short offset).
    //    Normalize to RFC3339 (space→`T`, `+HH`→`+HH:00`, `+HHMM`→`+HH:MM`) and keep the offset.
    if let Some(idx) = t.find(' ') {
        let candidate = fix_short_offset(&format!("{}T{}", &t[..idx], &t[idx + 1..]));
        if let Ok(odt) = OffsetDateTime::parse(&candidate, &Rfc3339) {
            return Ok((odt.to_offset(UtcOffset::UTC), odt.offset()));
        }
    }
    // 4. Swan trades: `MM/DD/YYYY HH:MM:SS` (US-locale, no TZ → UTC).
    let us_fmt = format_description!("[month]/[day]/[year] [hour]:[minute]:[second]");
    if let Ok(pdt) = PrimitiveDateTime::parse(t, &us_fmt) {
        return Ok((pdt.assume_utc(), UtcOffset::UTC));
    }
    // 5. River naive `YYYY-MM-DD HH:MM:SS` (no TZ → UTC).
    if let Ok(pdt) = PrimitiveDateTime::parse(t, &dt_fmt) {
        return Ok((pdt.assume_utc(), UtcOffset::UTC));
    }
    // 6. Bare date → UTC midnight.
    let date_fmt = format_description!("[year]-[month]-[day]");
    if let Ok(d) = Date::parse(t, &date_fmt) {
        return Ok((d.midnight().assume_utc(), UtcOffset::UTC));
    }
    Err(AdapterError::Parse {
        source,
        line,
        field: "timestamp",
        value: raw.to_string(),
        reason: "unrecognized timestamp format".to_string(),
    })
}

/// Normalize a short numeric UTC offset to RFC3339 form: `+00`→`+00:00`, `-0500`→`-05:00`. Only looks
/// past the date (sign index > 10) so the date's own hyphens are untouched. A full `±HH:MM` is unchanged.
fn fix_short_offset(s: &str) -> String {
    match s.rfind(['+', '-']).filter(|&p| p > 10) {
        Some(pos) => {
            let (head, off) = s.split_at(pos);
            let (sign, digits) = off.split_at(1);
            let norm = match digits.len() {
                2 => format!("{sign}{digits}:00"),
                4 => format!("{sign}{}:{}", &digits[..2], &digits[2..]),
                _ => off.to_string(),
            };
            format!("{head}{norm}")
        }
        None => s.to_string(),
    }
}

/// Convert an Excel/spreadsheet serial date number (days since 1899-12-30; the fractional part is the
/// time of day) to a UTC datetime — used for Gemini's numeric `Date`/`Time (UTC)` cells. `f64` is fine
/// here: NFR5 bars float *money*, not timestamps, and tax-date comparisons are day-granular (§6.1).
/// Anchor check: serial 25569 == 1970-01-01 (the Unix epoch).
pub fn excel_serial_to_utc(serial: f64) -> OffsetDateTime {
    let epoch = datetime!(1899-12-30 00:00:00 UTC);
    let whole = serial.trunc() as i64;
    let secs = (serial.fract() * 86_400.0).round() as i64;
    epoch + Duration::days(whole) + Duration::seconds(secs)
}

/// Like `parse_timestamp`, but also accepts a bare Excel serial number (Gemini exports `Date`/`Time`
/// as numeric serials). Tries the text formats first; a numeric value is treated as a serial at UTC.
pub fn parse_timestamp_flex(
    source: &'static str,
    line: usize,
    raw: &str,
) -> Result<(OffsetDateTime, UtcOffset), AdapterError> {
    match parse_timestamp(source, line, raw) {
        Ok(r) => Ok(r),
        Err(e) => match raw.trim().parse::<f64>() {
            Ok(serial) => Ok((excel_serial_to_utc(serial), UtcOffset::UTC)),
            Err(_) => Err(e),
        },
    }
}
```

- [ ] **Step 7: Run → PASS.** `cargo test -p btctax-adapters parse`
- [ ] **Step 8: Wire + gate + commit.** Confirm `pub mod parse;` is in `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git add Cargo.toml crates/btctax-adapters/Cargo.toml crates/btctax-adapters/src/lib.rs crates/btctax-adapters/src/parse.rs FOLLOWUPS.md
git commit -m "feat(adapters): scaffold + NFR5 exact money/btc/timestamp parse primitives"
```

---

### Task 1: Bundled price dataset + `BundledPrices` (`PriceProvider`, §9.2)

**Files:** Create `data/btc_usd_daily_close.csv`, `src/price.rs`; Modify `src/lib.rs`.

**Interfaces — Consumes:** `parse::parse_usd`, `AdapterError`, `btctax_core::{PriceProvider, TaxDate, Usd}`. **Produces:** `BundledPrices` (`load`/`from_csv_str`) implementing `PriceProvider` via exact-date daily-close lookup.

- [ ] **Step 1: Create `data/btc_usd_daily_close.csv`** (bundled; **public price data**, not private). Header + one row per calendar day; ISO date + exact decimal close. Seed with the dates the tests use (extend with the full public history later — it is just price data):
```csv
date,usd_close
2024-01-15,42500.00
2024-02-01,43100.50
2025-01-10,91000.00
2025-03-01,84000.00
2025-03-02,84250.25
2025-06-15,67500.00
```

- [ ] **Step 2: Failing tests in `src/price.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::price::fmv_of;
    use rust_decimal_macros::dec;
    use time::macros::date;

    #[test]
    fn looks_up_daily_close_exact_date() {
        let p = BundledPrices::from_csv_str("date,usd_close\n2025-03-01,84000.00\n").unwrap();
        assert_eq!(p.usd_per_btc(date!(2025 - 03 - 01)), Some(dec!(84000.00)));
        assert_eq!(p.usd_per_btc(date!(2025 - 03 - 02)), None); // no gap-fill → FR3 Missing
    }

    #[test]
    fn fmv_of_uses_provider_for_sat_quantity() {
        let p = BundledPrices::from_csv_str("date,usd_close\n2025-03-01,84000.00\n").unwrap();
        // 0.5 BTC = 50_000_000 sat @ 84000 = 42000.00
        assert_eq!(fmv_of(&p, date!(2025 - 03 - 01), 50_000_000), Some(dec!(42000.00)));
    }

    #[test]
    fn parses_exact_decimals_not_floats() {
        let p = BundledPrices::from_csv_str("date,usd_close\n2024-02-01,43100.50\n").unwrap();
        assert_eq!(p.usd_per_btc(date!(2024 - 02 - 01)), Some(dec!(43100.50)));
    }
}
```

- [ ] **Step 3: Run → FAIL.** `cargo test -p btctax-adapters price`

- [ ] **Step 4: Implement `src/price.rs`**
```rust
//! The bundled daily-close BTC/USD price dataset (§9.2) behind core's `PriceProvider`.
//! Pure & deterministic: identical (events, prices) → identical ledger (NFR4).
use crate::parse::parse_usd;
use crate::AdapterError;
use btctax_core::{PriceProvider, TaxDate, Usd};
use std::collections::BTreeMap;
use time::macros::format_description;

/// Bundled CSV: header `date,usd_close`; one row per calendar day; ISO date + exact decimal close.
const DATASET_CSV: &str = include_str!("../data/btc_usd_daily_close.csv");

/// Daily-close provider over the bundled dataset (§9.2). Exact-date lookup (BTC closes every day).
#[derive(Debug, Clone)]
pub struct BundledPrices {
    by_date: BTreeMap<TaxDate, Usd>,
}

impl BundledPrices {
    /// Load the compiled-in dataset.
    pub fn load() -> Result<Self, AdapterError> {
        Self::from_csv_str(DATASET_CSV)
    }

    /// Parse a `date,usd_close` CSV (used by `load` and by tests with synthetic data).
    pub fn from_csv_str(csv: &str) -> Result<Self, AdapterError> {
        let date_fmt = format_description!("[year]-[month]-[day]");
        let mut by_date = BTreeMap::new();
        for (i, line) in csv.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || (i == 0 && line.starts_with("date")) {
                continue;
            }
            let (d, p) = line.split_once(',').ok_or_else(|| {
                AdapterError::PriceDataset(format!("line {}: expected `date,usd_close`", i + 1))
            })?;
            let date = TaxDate::parse(d.trim(), &date_fmt).map_err(|e| {
                AdapterError::PriceDataset(format!("line {}: bad date {:?}: {e}", i + 1, d))
            })?;
            let close = parse_usd("price-dataset", i + 1, "usd_close", p)?;
            by_date.insert(date, close);
        }
        Ok(Self { by_date })
    }
}

impl PriceProvider for BundledPrices {
    fn usd_per_btc(&self, date: TaxDate) -> Option<Usd> {
        self.by_date.get(&date).copied()
    }
}
```

- [ ] **Step 5: Run → PASS.** `cargo test -p btctax-adapters price`
- [ ] **Step 6: Wire + gate + commit.** Add `pub mod price;` + `pub use price::BundledPrices;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git add crates/btctax-adapters/data crates/btctax-adapters/src/price.rs crates/btctax-adapters/src/lib.rs
git commit -m "feat(adapters): bundled daily-close price dataset + BundledPrices PriceProvider (§9.2)"
```

---
### Task 2: Format-agnostic table reading — `RawRow`, CSV (preamble/CRLF) + XLSX readers

**Files:** Create `src/read.rs`; Modify `src/lib.rs`.

**Interfaces — Consumes:** `AdapterError`, `calamine`, `csv`. **Produces:** `TableRole`, `RawRow` (`get`/`opt`), `ReadOpts` (preamble skip + header-token scan), `read_csv`/`read_csv_str`/`read_xlsx`/`read_table` (dispatch on extension), `peek_text` (for detection). XLSX numeric cells stringified via shortest-round-trip `{}` (NFR5 bound).

- [ ] **Step 1: Failing tests in `src/read.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_preamble_via_header_signature_and_handles_crlf() {
        // 3 preamble lines then the header; CRLF line endings (River-style).
        let text = "Transactions\r\nUser,acct\r\n\r\nID,Amount,Total\r\nX1,0.5,100.00\r\nX2,0.25,50.00\r\n";
        let opts = ReadOpts {
            header_signature: &["ID", "Total"],
            ..Default::default()
        };
        let rows = read_csv_str(text, TableRole::Single, &opts).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("t", "ID").unwrap(), "X1");
        assert_eq!(rows[0].get("t", "Total").unwrap(), "100.00");
        assert_eq!(rows[1].line, 2);
    }

    #[test]
    fn missing_column_is_a_typed_error() {
        let rows = read_csv_str("A,B\n1,2\n", TableRole::Single, &ReadOpts::default()).unwrap();
        let e = rows[0].get("t", "C").unwrap_err();
        assert!(matches!(e, crate::AdapterError::MissingColumn { .. }));
        assert_eq!(rows[0].opt("B"), Some("2"));
        assert_eq!(rows[0].opt("missing"), None);
    }

    #[test]
    fn fixed_skip_preamble_count_works_when_no_signature() {
        let text = "junk1\njunk2\nA,B\n1,2\n";
        let opts = ReadOpts {
            skip_preamble_lines: 2,
            ..Default::default()
        };
        let rows = read_csv_str(text, TableRole::Single, &opts).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("t", "A").unwrap(), "1");
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters read`

- [ ] **Step 3: Implement `src/read.rs`**
```rust
//! Format-agnostic table reading. A `RawRow` is a header→cell string map; CSV and XLSX both reduce
//! to it, so every parser is format-independent. Money/amount cells stay strings here (NFR5: the
//! exact decimal parse happens in `parse`).
use crate::AdapterError;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use std::collections::BTreeMap;
use std::path::Path;

/// Which logical table a row came from. Swan ships three files in one batch; everyone else is `Single`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableRole {
    Single,
    SwanTrades,
    SwanTransfers,
    SwanWithdrawals,
}

/// One parsed data row: the originating table role, a 1-based data-row number (error context), and
/// the header→cell map (cells are trimmed).
#[derive(Debug, Clone)]
pub struct RawRow {
    pub role: TableRole,
    pub line: usize,
    pub cells: BTreeMap<String, String>,
}
impl RawRow {
    /// Required column; `MissingColumn` if absent.
    pub fn get(&self, source: &'static str, col: &str) -> Result<&str, AdapterError> {
        self.cells
            .get(col)
            .map(|s| s.as_str())
            .ok_or_else(|| AdapterError::MissingColumn {
                source,
                line: self.line,
                column: col.to_string(),
            })
    }
    /// Optional column: `None` if absent OR blank.
    pub fn opt(&self, col: &str) -> Option<&str> {
        self.cells
            .get(col)
            .map(|s| s.as_str())
            .filter(|s| !s.trim().is_empty())
    }
}

/// CSV preamble handling: either scan for the first line containing all `header_signature` tokens
/// (robust to preamble-length drift — preferred), or skip a fixed `skip_preamble_lines` count.
#[derive(Debug, Clone, Default)]
pub struct ReadOpts {
    pub skip_preamble_lines: usize,
    pub header_signature: &'static [&'static str],
}

/// Read a CSV file's data rows. Reads the whole file as text first (so any error is a clean parse
/// error with path context); the `csv` crate handles CRLF transparently.
pub fn read_csv(path: &Path, role: TableRole, opts: &ReadOpts) -> Result<Vec<RawRow>, AdapterError> {
    let text = std::fs::read_to_string(path).map_err(|e| AdapterError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    read_csv_str(&text, role, opts).map_err(|e| with_path(e, path))
}

/// CSV-from-string (used by `read_csv` and tests).
pub fn read_csv_str(
    text: &str,
    role: TableRole,
    opts: &ReadOpts,
) -> Result<Vec<RawRow>, AdapterError> {
    let start = if !opts.header_signature.is_empty() {
        text.lines()
            .position(|l| opts.header_signature.iter().all(|t| l.contains(t)))
            .unwrap_or(0)
    } else {
        opts.skip_preamble_lines
    };
    let body: String = text.split_inclusive('\n').skip(start).collect();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(body.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map_err(csv_err)?
        .iter()
        .map(|h| h.trim().to_string())
        .collect();
    let mut out = Vec::new();
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.map_err(csv_err)?;
        let mut cells = BTreeMap::new();
        for (h, v) in headers.iter().zip(rec.iter()) {
            if !h.is_empty() {
                cells.insert(h.clone(), v.trim().to_string());
            }
        }
        out.push(RawRow {
            role,
            line: i + 1,
            cells,
        });
    }
    Ok(out)
}

/// Read the first worksheet of an XLSX file's data rows (header = first row).
pub fn read_xlsx(path: &Path, role: TableRole) -> Result<Vec<RawRow>, AdapterError> {
    let mut wb: Xlsx<_> = open_workbook(path).map_err(|e| AdapterError::Xlsx {
        path: path.display().to_string(),
        source: e.into(),
    })?;
    let range = wb
        .worksheet_range_at(0)
        // M-2: use EmptyXlsx (not PriceDataset — wrong category) for the "no worksheet" case.
        // calamine returns None when the workbook has no sheet, which is a reader-layer concern.
        .ok_or_else(|| AdapterError::EmptyXlsx { path: path.display().to_string() })?
        .map_err(|e| AdapterError::Xlsx {
            path: path.display().to_string(),
            source: e.into(),
        })?;
    Ok(rows_from_range(&range, role))
}

/// Dispatch on file extension: `.xlsx`/`.xls` → calamine; everything else → CSV.
pub fn read_table(path: &Path, role: TableRole, opts: &ReadOpts) -> Result<Vec<RawRow>, AdapterError> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("xlsx") | Some("xls") => read_xlsx(path, role),
        _ => read_csv(path, role, opts),
    }
}

/// Peek the first `max_bytes` of a file as lossy UTF-8 (for source detection). For XLSX, returns the
/// path's bytes (binary) — XLSX detection keys on the extension, not the snippet.
pub fn peek_text(path: &Path, max_bytes: usize) -> Result<String, AdapterError> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| AdapterError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let mut buf = vec![0u8; max_bytes];
    let n = f.read(&mut buf).map_err(|e| AdapterError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    buf.truncate(n);
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// XLSX numeric cells are IEEE-754 doubles in the file format. Rust's shortest-round-trip `{}` for f64
/// reproduces the intended ≤8-dp exchange decimal exactly (e.g. 0.12345678 → "0.12345678"); that string
/// is then parsed by the exact decimal parser (NFR5 — documented bound, FOLLOWUPS).
fn cell_to_string(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => format!("{f}"),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        // Excel-serial datetimes: extract the underlying serial number and format it as a string
        // so it flows through the same Data::Float → parse_timestamp_flex(serial) path used for
        // Gemini's numeric Date/Time columns. dt.to_string() must NOT be used here — it formats
        // as an ISO date/time string that parse_timestamp_flex cannot convert back to a serial, so
        // the fractional-day time component (hour/minute/second) is silently lost.
        // FIRST-BUILD VERIFICATION (Task 0 / Task 2): confirm the calamine 0.26 ExcelDateTime serial
        // accessor — `as_f64()` is the expected method name; if it differs, update this arm. If
        // Data::DateTime does not exist in calamine 0.26, delete this arm entirely (see Task-0
        // first-build checklist M-3).
        Data::DateTime(dt) => format!("{}", dt.as_f64()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}

fn rows_from_range(range: &Range<Data>, role: TableRole) -> Vec<RawRow> {
    let mut iter = range.rows();
    let header: Vec<String> = match iter.next() {
        Some(h) => h.iter().map(cell_to_string).collect(),
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for (i, r) in iter.enumerate() {
        let mut cells = BTreeMap::new();
        for (h, c) in header.iter().zip(r.iter()) {
            if !h.is_empty() {
                cells.insert(h.clone(), cell_to_string(c));
            }
        }
        if cells.values().all(|v| v.is_empty()) {
            continue;
        }
        out.push(RawRow {
            role,
            line: i + 1,
            cells,
        });
    }
    out
}

fn csv_err(e: csv::Error) -> AdapterError {
    AdapterError::Csv {
        path: "<csv>".to_string(),
        source: e,
    }
}

fn with_path(e: AdapterError, path: &Path) -> AdapterError {
    match e {
        AdapterError::Csv { source, .. } => AdapterError::Csv {
            path: path.display().to_string(),
            source,
        },
        other => other,
    }
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters read`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod read;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): format-agnostic table reading (RawRow, CSV preamble/CRLF, XLSX)"
```

---

### Task 3: Shared normalize helpers — FR3 `resolve_fmv`, `SourceRefMint` (§6.2), wallet, `raw_of`

**Files:** Create `src/normalize.rs`; Modify `src/lib.rs`.

**Interfaces — Consumes:** `btctax_core::{price::fmv_of, FmvStatus, PriceProvider, Sat, Source, SourceRef, TaxDate, Usd, WalletId}`, `read::RawRow`. **Produces:** `resolve_fmv` (FR3), `Direction`, `SourceRefMint` (native + semantic-with-occurrence-index, §6.2), `exchange_wallet`, `raw_of`.

- [ ] **Step 1: Failing tests in `src/normalize.rs`**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::price::StaticPrices;
    use btctax_core::FmvStatus;
    use rust_decimal_macros::dec;
    use time::macros::date;

    fn prices() -> StaticPrices {
        let mut m = std::collections::BTreeMap::new();
        m.insert(date!(2025 - 03 - 01), dec!(84000.00));
        StaticPrices(m)
    }

    #[test]
    fn fr3_prefers_export_then_dataset_then_missing() {
        let p = prices();
        // export present → ExchangeProvided (verbatim value, dataset ignored)
        let (v, s) = resolve_fmv(Some(dec!(123.45)), date!(2025 - 03 - 01), 50_000_000, &p);
        assert_eq!((v, s), (Some(dec!(123.45)), FmvStatus::ExchangeProvided));
        // no export, dataset hit → PriceDataset (0.5 BTC @ 84000 = 42000.00)
        let (v, s) = resolve_fmv(None, date!(2025 - 03 - 01), 50_000_000, &p);
        assert_eq!((v, s), (Some(dec!(42000.00)), FmvStatus::PriceDataset));
        // no export, dataset miss → Missing
        let (v, s) = resolve_fmv(None, date!(2025 - 06 - 15), 50_000_000, &p);
        assert_eq!((v, s), (None, FmvStatus::Missing));
    }

    #[test]
    fn native_source_ref_is_direction_scoped_id() {
        let mint = SourceRefMint::default();
        assert_eq!(mint.native(Direction::Out, "TX-9").0, "out|TX-9");
    }

    #[test]
    fn semantic_source_ref_disambiguates_identical_rows_by_occurrence() {
        let mut mint = SourceRefMint::default();
        let a = mint.semantic(Direction::In, 1_700_000_000_000, "income", 1000);
        let b = mint.semantic(Direction::In, 1_700_000_000_000, "income", 1000); // identical row
        assert_eq!(a.0, "in|1700000000000|income|1000#0");
        assert_eq!(b.0, "in|1700000000000|income|1000#1"); // occurrence_index increments
        assert_ne!(a, b);
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters normalize`

- [ ] **Step 3: Implement `src/normalize.rs`**
```rust
//! Shared normalize helpers: FR3 ingest-time FMV resolution, §6.2 `source_ref` synthesis, wallet
//! construction, and `Unclassified` raw capture. Used by every parser so the policy is identical.
use crate::read::RawRow;
use btctax_core::price::fmv_of;
use btctax_core::{FmvStatus, PriceProvider, Sat, Source, SourceRef, TaxDate, Usd, WalletId};
use std::collections::HashMap;

/// FR3: prefer the export's own USD (`ExchangeProvided`); else the bundled daily-close dataset
/// (`PriceDataset`); else `Missing` (a hard blocker in core). `sat` is taken in magnitude.
pub fn resolve_fmv(
    export_usd: Option<Usd>,
    date: TaxDate,
    sat: Sat,
    prices: &dyn PriceProvider,
) -> (Option<Usd>, FmvStatus) {
    if let Some(u) = export_usd {
        return (Some(u), FmvStatus::ExchangeProvided);
    }
    match fmv_of(prices, date, sat.abs()) {
        Some(u) => (Some(u), FmvStatus::PriceDataset),
        None => (None, FmvStatus::Missing),
    }
}

/// (source, direction)-scoping for `source_ref` (§6.2) and the semantic key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
    Trade,
}
impl Direction {
    pub fn tag(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
            Direction::Trade => "trade",
        }
    }
}

/// Mints stable `SourceRef`s (§6.2). Native-id rows pass the id through, direction-scoped. Id-less rows
/// (River; Gemini transfer rows lacking a `Trade ID`) get the semantic key `dir|utc_ms|type|sat` plus a
/// deterministic per-key `occurrence_index` to disambiguate identical rows in file order (the file-order
/// fragility is the documented §6.2 / FOLLOWUPS limitation).
#[derive(Debug, Default)]
pub struct SourceRefMint {
    seen: HashMap<String, u32>,
}
impl SourceRefMint {
    pub fn native(&self, dir: Direction, id: &str) -> SourceRef {
        SourceRef::new(format!("{}|{}", dir.tag(), id))
    }
    pub fn semantic(&mut self, dir: Direction, utc_ms: i64, type_tag: &str, sat: Sat) -> SourceRef {
        let key = format!("{}|{}|{}|{}", dir.tag(), utc_ms, type_tag, sat);
        let occ = self.seen.entry(key.clone()).or_insert(0);
        let r = SourceRef::new(format!("{key}#{occ}"));
        *occ += 1;
        r
    }
}

/// Single-account exchange wallet for a source (multi-account is future — FOLLOWUPS).
pub fn exchange_wallet(source: Source) -> WalletId {
    WalletId::Exchange {
        provider: source.tag().to_string(),
        account: "default".to_string(),
    }
}

/// Deterministic raw capture for an `Unclassified` event (sorted keys via `BTreeMap`).
pub fn raw_of(row: &RawRow) -> String {
    row.cells
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters normalize`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod normalize;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): shared normalize helpers (FR3 resolve_fmv, SourceRefMint §6.2, wallet)"
```

---
### Task 4: `Adapter` trait + ingest types (`SourceFile`/`FileGroup`/`GroupOutput`/`FileReport`/`IngestBatch`)

**Files:** Create `src/adapter.rs`; Modify `src/lib.rs`.

**Interfaces — Consumes:** `read::RawRow`, `AdapterError`, `btctax_core::{LedgerEvent, PriceProvider, Source}`. **Produces:** the §9 `Adapter` trait (`source`/`detect`/`group`/`parse`/`normalize`) + the ingest data types. No source logic yet — this fixes the shapes the four parsers (Tasks 5–8) and `ingest` (Task 9) implement.

- [ ] **Step 1: Failing tests in `src/adapter.rs`** (shape-only; a stub adapter proves the trait is object-safe and `GroupOutput::merge` accumulates FR2 counts)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_output_merge_accumulates_fr2_counts() {
        let mut a = GroupOutput::default();
        a.dropped_no_btc = 1;
        a.unclassified = 2;
        a.parsed_rows = 3;
        let b = GroupOutput {
            dropped_no_btc: 4,
            unclassified: 5,
            parsed_rows: 6,
            events: Vec::new(),
        };
        a.merge(b);
        assert_eq!((a.dropped_no_btc, a.unclassified, a.parsed_rows), (5, 7, 9));
    }

    #[test]
    fn trait_is_object_safe() {
        fn _takes(_: &dyn Adapter) {}
    }
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters adapter`

- [ ] **Step 3: Implement `src/adapter.rs`**
```rust
//! The §9 `Adapter` contract (detect → group → parse → normalize) and the ingest data types. Parsers
//! PRODUCE `LedgerEvent`s; the CLI (Plan 4) persists them via `btctax_core::persistence`.
use crate::read::RawRow;
use crate::AdapterError;
use btctax_core::{LedgerEvent, PriceProvider, Source};
use std::path::PathBuf;

/// One input file on disk (content is read lazily by the reader).
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
}
impl SourceFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

/// A unit of ingest: one file for most sources; Swan groups its three files into one batch.
#[derive(Debug, Clone)]
pub struct FileGroup {
    pub source: Source,
    pub label: String,
    pub files: Vec<SourceFile>,
}

/// Per-group parse result: the BTC events (incl. `Unclassified`) + FR2 counts.
#[derive(Debug, Default)]
pub struct GroupOutput {
    pub events: Vec<LedgerEvent>,
    /// FR2: rows with no BTC leg, dropped (not evented).
    pub dropped_no_btc: usize,
    /// FR2: BTC-side rows that became `Unclassified` events (NOT dropped).
    pub unclassified: usize,
    pub parsed_rows: usize,
}
impl GroupOutput {
    pub fn merge(&mut self, o: GroupOutput) {
        self.events.extend(o.events);
        self.dropped_no_btc += o.dropped_no_btc;
        self.unclassified += o.unclassified;
        self.parsed_rows += o.parsed_rows;
    }
}

/// FR2 per-group report surfaced to the CLI ("Report dropped (no-BTC) + unclassified counts per file").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    pub source: Source,
    pub label: String,
    pub parsed_rows: usize,
    pub btc_events: usize,
    pub dropped_no_btc: usize,
    pub unclassified: usize,
}

/// The whole ingest result: every BTC event across all groups + one report per group.
#[derive(Debug, Default)]
pub struct IngestBatch {
    pub events: Vec<LedgerEvent>,
    pub reports: Vec<FileReport>,
}

/// §9 Adapter contract. Each impl's doc-comment states its `source_ref`/dedup, gross-vs-net proceeds,
/// fee placement, and unknown-type → `Unclassified` policy (asserted by a fixture test).
pub trait Adapter {
    fn source(&self) -> Source;
    /// True if this adapter recognizes `file` (header/preamble signature or extension). Signatures are
    /// matched against documented §9 tokens / synthetic-fixture headers (confirm vs real exports).
    fn detect(&self, file: &SourceFile) -> Result<bool, AdapterError>;
    /// Group recognized files into ingest units (Swan merges 3 → 1; others 1:1).
    fn group(&self, files: Vec<SourceFile>) -> Vec<FileGroup>;
    /// Parse raw rows from a group (preamble/CRLF handled by the reader; Swan tags rows by role).
    fn parse(&self, group: &FileGroup) -> Result<Vec<RawRow>, AdapterError>;
    /// Map rows → BTC `LedgerEvent`s: FR2 filter (drop no-BTC; unknown BTC → `Unclassified`) + FR3 FMV.
    fn normalize(
        &self,
        group: &FileGroup,
        rows: Vec<RawRow>,
        prices: &dyn PriceProvider,
    ) -> Result<GroupOutput, AdapterError>;
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters adapter`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod adapter;` + `pub use adapter::{Adapter, FileGroup, FileReport, GroupOutput, IngestBatch, SourceFile};` to `lib.rs`. Also add `pub mod sources { pub mod coinbase; pub mod gemini; pub mod river; pub mod swan; }` placeholder modules (empty for now, filled in Tasks 5–8) — or add each `pub mod` as its task lands. (If adding now, create empty files so the crate compiles.)
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): Adapter trait (§9 detect/group/parse/normalize) + ingest types"
```

---

### Task 5: Coinbase parser (§9.1) — CSV, 3-line preamble, native `ID`, internal-move/Order → Unclassified

**Files:** Create `src/sources/coinbase.rs`; Modify `src/lib.rs` (`pub mod sources;`/`pub mod coinbase;`). Test `tests/coinbase.rs`.

**Interfaces — Consumes:** `parse`, `read`, `normalize`, `adapter`, core event types. **Produces:** `pub struct Coinbase` (`impl Adapter`).

**§9.1 mapping (module doc states these; confirmed `Transaction Type` enum = Buy/Sell/Send/Receive/Withdrawal/Order/Exchange Deposit/Exchange Withdrawal/Pro Deposit/Pro Withdrawal):** `Buy`→`Acquire` (usd_cost=`Subtotal`, fee=`Fees and/or Spread`; basis=cost+fee=`Total (…)`); `Sell`→`Dispose{Sell}` (gross=`Subtotal`, fee=`Fees and/or Spread`); `Send`/`Withdrawal`→`TransferOut` (dest=`Recipient Address`); `Receive`→`TransferIn` (src=`Sender Address`); `Order` + `Exchange Deposit`/`Exchange Withdrawal`/`Pro Deposit`/`Pro Withdrawal` (internal Coinbase↔Coinbase-Pro moves — likely self-transfers, user-confirmed) + **any unknown/future type (incl. the absent `Convert`/reward) → `Unclassified`** (conservative — never guess). **FR2:** `Asset`≠BTC → drop+count. `source_ref` = native `ID`, direction-scoped. (No FMV/`PriceProvider` needed: every Coinbase event carries its own USD or is a transfer/Unclassified.)

- [ ] **Step 1: Failing test in `tests/coinbase.rs`** (synthetic fixture — invented headers/values; 3-line preamble; CRLF)
```rust
use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::coinbase::Coinbase;
use btctax_core::{BasisSource, DisposeKind, EventPayload, Source};

// SYNTHETIC Coinbase export: the REAL §9.1 header names (13 cols), INVENTED values. 3-line preamble
// (empty / `Transactions` / user-identity), header on line 4, CRLF. Rows exercise the confirmed
// `Transaction Type` enum: Buy, Sell, Send, Receive, a non-BTC (ETH) row, an Order (→ Unclassified),
// and an Exchange Deposit (internal Coinbase↔Pro move → Unclassified). Real timestamp format `… UTC`.
const CSV: &str = "\
\r\n\
Transactions\r\n\
User,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-1,2025-03-01 12:00:00 UTC,Buy,BTC,0.01000000,USD,84000.00,840.00,845.00,5.00,,,\r\n\
cb-2,2025-03-02 09:00:00 UTC,Sell,BTC,0.00500000,USD,84250.00,421.25,419.25,2.00,,,\r\n\
cb-3,2025-03-02 10:00:00 UTC,Send,BTC,0.00250000,,,,,0,,,bc1qrcv\r\n\
cb-4,2025-03-02 11:00:00 UTC,Receive,BTC,0.00250000,,,,,0,,bc1qsnd,\r\n\
cb-5,2025-03-01 08:00:00 UTC,Buy,ETH,1.00000000,USD,2000.00,2000.00,2010.00,10.00,,,\r\n\
cb-6,2025-03-01 08:30:00 UTC,Order,BTC,0.00100000,,,,,0,,,\r\n\
cb-7,2025-03-01 07:00:00 UTC,Exchange Deposit,BTC,0.00010000,,,,,0,,,\r\n";

fn group(path: std::path::PathBuf) -> FileGroup {
    FileGroup { source: Source::Coinbase, label: "coinbase".into(), files: vec![SourceFile::new(path)] }
}

#[test]
fn coinbase_maps_types_filters_non_btc_and_unclassifies_internal_moves() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coinbase_2025.csv");
    std::fs::write(&path, CSV).unwrap();
    let prices = BundledPrices::load().unwrap();
    let cb = Coinbase;
    let g = group(path);
    let rows = cb.parse(&g).unwrap();
    let out = cb.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.dropped_no_btc, 1); // the ETH row (no BTC leg)
    assert_eq!(out.unclassified, 2); // Order + Exchange Deposit (internal-move) rows
    // Buy, Sell, Send, Receive, Order(Unclass), Exchange Deposit(Unclass) = 6 BTC events; ETH dropped.
    assert_eq!(out.events.len(), 6);

    let buy = out.events.iter().find(|e| matches!(&e.payload, EventPayload::Acquire(_))).unwrap();
    match &buy.payload {
        EventPayload::Acquire(a) => {
            assert_eq!(a.sat, 1_000_000);
            assert_eq!(a.usd_cost.to_string(), "840.00"); // Subtotal (cost); fee separate
            assert_eq!(a.fee_usd.to_string(), "5.00"); // Fees and/or Spread → basis = 845.00 = Total
            assert_eq!(a.basis_source, BasisSource::ExchangeProvided);
        }
        _ => unreachable!(),
    }
    let sell = out.events.iter().find(|e| matches!(&e.payload, EventPayload::Dispose(_))).unwrap();
    match &sell.payload {
        EventPayload::Dispose(d) => {
            assert_eq!(d.kind, DisposeKind::Sell);
            assert_eq!(d.usd_proceeds.to_string(), "421.25"); // GROSS Subtotal
            assert_eq!(d.fee_usd.to_string(), "2.00");
        }
        _ => unreachable!(),
    }
    // Send → TransferOut with Recipient Address as dest; Receive → TransferIn with Sender Address as src.
    let send = out.events.iter().find(|e| matches!(&e.payload, EventPayload::TransferOut(_))).unwrap();
    match &send.payload {
        EventPayload::TransferOut(t) => assert_eq!(t.dest_addr.as_deref(), Some("bc1qrcv")),
        _ => unreachable!(),
    }
    let recv = out.events.iter().find(|e| matches!(&e.payload, EventPayload::TransferIn(_))).unwrap();
    match &recv.payload {
        EventPayload::TransferIn(t) => assert_eq!(t.src_addr.as_deref(), Some("bc1qsnd")),
        _ => unreachable!(),
    }
    // native `ID` source_ref, direction-scoped.
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|coinbase|trade|cb-1"));
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|coinbase|out|cb-3"));
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|coinbase|in|cb-4"));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters --test coinbase`

- [ ] **Step 3: Implement `src/sources/coinbase.rs`**
```rust
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
    Source, TransferIn, TransferOut, Unclassified, Usd,
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
        id_ref: btctax_core::SourceRef,
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
        Ok(snip.contains(cols::TX_TYPE) && snip.contains(cols::QTY) && snip.contains(cols::SUBTOTAL))
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
            rows.extend(read_table(&f.path, TableRole::Single, &opts)?);
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
        let mut out = GroupOutput::default();
        out.parsed_rows = rows.len();
        for row in &rows {
            // FR2: keep only the BTC leg.
            let asset = row.opt(cols::ASSET).unwrap_or("");
            if !asset.eq_ignore_ascii_case(ASSET_BTC) {
                out.dropped_no_btc += 1;
                continue;
            }
            let ttype = row.get(SRC, cols::TX_TYPE)?;
            let sat =
                parse_btc_to_sat(SRC, row.line, "Quantity Transacted", row.get(SRC, cols::QTY)?)?.abs();
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
                        EventPayload::TransferIn(TransferIn { sat, src_addr: sender, txid: None }),
                    ),
                    // Internal Coinbase↔Coinbase-Pro moves: likely self-transfers, but user-confirmed → Unclassified.
                    "exchange deposit" | "pro deposit" => {
                        out.unclassified += 1;
                        (Direction::In, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
                    }
                    "exchange withdrawal" | "pro withdrawal" => {
                        out.unclassified += 1;
                        (Direction::Out, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
                    }
                    // `Order` = a known Coinbase type (order-book fill; an order may net BTC/USD
                    // differently from a simple Buy/Sell). Explicit arm so the type is documented and
                    // findable by grep — do not rely on the `_` catch-all for a known type (M-4).
                    "order" => {
                        out.unclassified += 1;
                        (Direction::Trade, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
                    }
                    // Any unknown/future type (incl. the absent `Convert`/reward in the confirmed
                    // 2012-2019 Coinbase vocabulary) → Unclassified (never guess).
                    _ => {
                        out.unclassified += 1;
                        (Direction::Trade, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
                    }
                };

            let id_ref = mint.native(dir, id);
            out.events.push(self.event(id_ref, utc, tz, payload));
        }
        Ok(out)
    }
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters --test coinbase`
- [ ] **Step 5: Gate + commit.** `cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check`
```bash
git commit -am "feat(adapters): Coinbase parser (§9.1) — types + internal-move/Order→Unclassified, FR2, native ID"
```

---
### Task 6: Gemini parser (§9.1) — XLSX ledger, `Trade ID`+`Order ID`/`Tx Hash`, `Credit`(BTC)→TransferIn

**Files:** Create `src/sources/gemini.rs`; Modify `src/lib.rs`. Test `tests/gemini.rs` (builds a SYNTHETIC `.xlsx` with `rust_xlsxwriter`).

**Interfaces — Produces:** `pub struct Gemini` (`impl Adapter`).

**§9.1 mapping (module doc states these; confirmed `Type` enum = Buy/Sell/Credit/Debit):** `Buy`→`Acquire` (usd_cost=`USD Amount USD`, fee=`Fee (USD) USD`); `Sell`→`Dispose{Sell}` (gross=`USD Amount USD`, fee=`Fee (USD) USD` separate); `Debit`(BTC)→`TransferOut` (dest=`Withdrawal Destination`); **`Credit`(BTC)→`TransferIn`** (src=`Deposit Destination`; supersedes the earlier Credit→Unclassified — §9.1 confirmed); `Credit`/`Debit`(USD)=cash → **dropped** (no BTC leg, FR2). BTC-leg presence = `BTC Amount BTC` populated/non-zero. `source_ref` = native `Trade ID`+`Order ID` (direction-scoped) on trade rows, else semantic (`Credit`/`Debit` rows lack trade ids). `txid` = `Tx Hash` (match signal). `Date`/`Time (UTC)` are Excel serial numbers → `parse_timestamp_flex` converts them (synthetic fixtures use ISO text for readability). `BTC Balance BTC` is reconciliation data (FR9, CLI) — captured by the reader, not folded here.

- [ ] **Step 1: Failing test in `tests/gemini.rs`**
```rust
use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::gemini::Gemini;
use btctax_core::{EventPayload, Source};
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
        "Date", "Time (UTC)", "Type", "Symbol", "BTC Amount BTC", "USD Amount USD", "Fee (USD) USD",
        "BTC Balance BTC", "Trade ID", "Order ID", "Tx Hash", "Deposit Destination", "Withdrawal Destination",
    ];
    for (c, h) in header.iter().enumerate() {
        ws.write_string(0, c as u16, *h).unwrap();
    }
    // Buy row (row 1): Date as numeric Excel serial (exercises IP-1 path); all other cells as strings.
    ws.write_number(1, 0, 45717.5f64).unwrap(); // M-1: serial → Data::Float → parse_timestamp_flex
    for (c, v) in ["2025-03-01 12:00:00", "Buy", "BTCUSD", "0.02000000", "1680.00", "5.00",
                    "0.02000000", "T-1", "O-1", "", "", ""].iter().enumerate() {
        ws.write_string(1, (c + 1) as u16, v).unwrap();
    }
    // Remaining rows: all cells as strings (ISO text — parse_timestamp handles them).
    // Sell 0.01; Debit (BTC out → TransferOut); Credit BTC (→ TransferIn); Credit USD (→ dropped).
    let rows: [[&str; 13]; 4] = [
        ["2025-03-02 09:00:00", "2025-03-02 09:00:00", "Sell", "BTCUSD", "0.01000000", "842.50", "2.50", "0.01000000", "T-2", "O-2", "", "", ""],
        ["2025-03-02 10:00:00", "2025-03-02 10:00:00", "Debit", "BTC", "0.00500000", "", "", "0.00500000", "", "", "deadbeef", "", "bc1qwd"],
        ["2025-03-02 11:00:00", "2025-03-02 11:00:00", "Credit", "BTC", "0.00100000", "", "", "0.00600000", "", "", "feedface", "bc1qdp", ""],
        ["2025-03-02 12:00:00", "2025-03-02 12:00:00", "Credit", "USD", "", "500.00", "", "0.00600000", "", "", "", "", ""],
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
    let g = FileGroup { source: Source::Gemini, label: "gemini".into(), files: vec![SourceFile::new(path)] };
    let rows = gm.parse(&g).unwrap();
    let out = gm.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.dropped_no_btc, 1); // Credit(USD) cash (no BTC leg)
    assert_eq!(out.unclassified, 0); // Credit(BTC) is a TransferIn now, not Unclassified
    // Buy, Sell, Debit→TransferOut, Credit→TransferIn = 4 BTC events.
    assert_eq!(out.events.len(), 4);
    assert!(out.events.iter().any(|e| matches!(&e.payload, EventPayload::Acquire(_))));
    assert!(out.events.iter().any(|e| matches!(&e.payload, EventPayload::Dispose(_))));

    // Debit → TransferOut (txid = Tx Hash, dest = Withdrawal Destination); semantic id-less source_ref.
    let debit = out.events.iter().find(|e| matches!(&e.payload, EventPayload::TransferOut(_))).unwrap();
    assert!(debit.id.canonical().starts_with("import|gemini|out|"));
    match &debit.payload {
        EventPayload::TransferOut(t) => {
            assert_eq!(t.txid.as_deref(), Some("deadbeef"));
            assert_eq!(t.dest_addr.as_deref(), Some("bc1qwd"));
        }
        _ => unreachable!(),
    }
    // Credit(BTC) → TransferIn (txid + src = Deposit Destination); semantic id-less source_ref.
    let credit = out.events.iter().find(|e| matches!(&e.payload, EventPayload::TransferIn(_))).unwrap();
    assert!(credit.id.canonical().starts_with("import|gemini|in|"));
    match &credit.payload {
        EventPayload::TransferIn(t) => {
            assert_eq!(t.txid.as_deref(), Some("feedface"));
            assert_eq!(t.src_addr.as_deref(), Some("bc1qdp"));
        }
        _ => unreachable!(),
    }
    // native `Trade ID`+`Order ID` source_ref for the Buy (combined, direction-scoped).
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|gemini|trade|T-1.O-1"));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters --test gemini`

- [ ] **Step 3: Implement `src/sources/gemini.rs`**
```rust
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
            rows.extend(read_table(&f.path, TableRole::Single, &opts)?);
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
        let mut out = GroupOutput::default();
        out.parsed_rows = rows.len();
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
                    (Direction::Trade, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
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
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters --test gemini`
- [ ] **Step 5: Gate + commit.**
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): Gemini XLSX parser (§9.1) — Credit(BTC)→TransferIn, Debit→TransferOut, USD-cash drop"
```

---

### Task 7: River parser (§9.1) — universal CSV (CRLF), semantic `source_ref`, Income/Interest→dataset FMV

**Files:** Create `src/sources/river.rs`; Modify `src/lib.rs`. Test `tests/river.rs`.

**Interfaces — Produces:** `pub struct River` (`impl Adapter`).

**§9.1 mapping (module doc states these; confirmed universal Sent/Received shape; `Tag` enum = Buy/Income/Interest/Withdrawal):** `Buy`→`Acquire` (usd_cost=`Sent Amount`, fee=`Fee Amount`, sat=`Received Amount`; basis=cost+fee); `Income`→`Income{Reward}` / `Interest`→`Income{Interest}` (no export USD → **dataset FMV**, FR3; sat=`Received Amount`); `Withdrawal`→`TransferOut` (sat=`Sent Amount`); unknown `Tag`→`Unclassified`. **`source_ref` is always semantic** (`dir|utc_ms|type|sat` + occurrence_index — River is id-less, §6.2). FR2: keep iff `Sent Currency`==BTC ∨ `Received Currency`==BTC (the BTC leg is whichever side is BTC); else dropped.

- [ ] **Step 1: Failing test in `tests/river.rs`**
```rust
use btctax_adapters::adapter::{Adapter, FileGroup, SourceFile};
use btctax_adapters::price::BundledPrices;
use btctax_adapters::sources::river::River;
use btctax_core::{EventPayload, FmvStatus, IncomeKind, Source};

// SYNTHETIC River CSV: the REAL §9.1 header names (8-col universal Sent/Received shape), INVENTED
// values. CRLF, no preamble, real naive timestamp format. The Interest row carries NO USD → must
// resolve from the bundled dataset (date 2025-06-15 = 67500.00).
const CSV: &str = "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-03-01 12:00:00,4200.00,USD,0.05000000,BTC,3.00,USD,Buy\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-03-02 08:00:00,0.01000000,BTC,,,,,Withdrawal\r\n";

#[test]
fn river_semantic_refs_and_dataset_fmv_for_interest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("river_universal.csv");
    std::fs::write(&path, CSV).unwrap();
    let prices = BundledPrices::load().unwrap();
    let rv = River;
    let g = FileGroup { source: Source::River, label: "river".into(), files: vec![SourceFile::new(path)] };
    let rows = rv.parse(&g).unwrap();
    let out = rv.normalize(&g, rows, &prices).unwrap();

    assert_eq!(out.events.len(), 3);
    assert_eq!(out.dropped_no_btc, 0);

    let buy = out.events.iter().find(|e| matches!(&e.payload, EventPayload::Acquire(_))).unwrap();
    match &buy.payload {
        EventPayload::Acquire(a) => {
            assert_eq!(a.sat, 5_000_000); // Received Amount (BTC)
            assert_eq!(a.usd_cost.to_string(), "4200.00"); // Sent Amount (USD)
            assert_eq!(a.fee_usd.to_string(), "3.00"); // Fee Amount → basis = Sent + Fee
        }
        _ => unreachable!(),
    }
    let inc = out.events.iter().find(|e| matches!(&e.payload, EventPayload::Income(_))).unwrap();
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
    let wd = out.events.iter().find(|e| matches!(&e.payload, EventPayload::TransferOut(_))).unwrap();
    match &wd.payload {
        EventPayload::TransferOut(t) => assert_eq!(t.sat, 1_000_000),
        _ => unreachable!(),
    }
    // semantic source_ref (River is id-less); Buy direction = trade.
    assert!(buy.id.canonical().starts_with("import|river|trade|"));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters --test river`

- [ ] **Step 3: Implement `src/sources/river.rs`**
```rust
//! River universal-CSV adapter (§9.1, confirmed schema). CRLF (handled by the reader). Id-less →
//! semantic `source_ref` (`dir|utc_ms|type|sat` + occurrence_index, §6.2). Universal Sent/Received
//! shape: the BTC leg is whichever currency is BTC. `Buy`→Acquire (usd_cost=`Sent Amount`,
//! fee=`Fee Amount`, sat=`Received Amount`); `Income`→Income{Reward} / `Interest`→Income{Interest}
//! (no USD → dataset FMV, FR3; sat=`Received Amount`); `Withdrawal`→TransferOut (sat=`Sent Amount`);
//! unknown `Tag`→Unclassified. FR2: neither currency BTC → drop.
use crate::adapter::{Adapter, FileGroup, GroupOutput, SourceFile};
use crate::normalize::{exchange_wallet, raw_of, resolve_fmv, Direction, SourceRefMint};
use crate::parse::{parse_btc_to_sat, parse_timestamp, parse_usd};
use crate::read::{read_table, RawRow, ReadOpts, TableRole};
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
        let snip = crate::read::peek_text(&file.path, 4096)?;
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
            rows.extend(read_table(&f.path, TableRole::Single, &opts)?);
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
        let mut out = GroupOutput::default();
        out.parsed_rows = rows.len();
        for row in &rows {
            // FR2: the BTC leg is whichever currency is BTC; if neither, no BTC leg → drop.
            let recv_is_btc = row.opt(cols::RECEIVED_CURRENCY).unwrap_or("").eq_ignore_ascii_case(ASSET_BTC);
            let sent_is_btc = row.opt(cols::SENT_CURRENCY).unwrap_or("").eq_ignore_ascii_case(ASSET_BTC);
            if !recv_is_btc && !sent_is_btc {
                out.dropped_no_btc += 1;
                continue;
            }
            let sat = if recv_is_btc {
                parse_btc_to_sat(SRC, row.line, "Received Amount", row.get(SRC, cols::RECEIVED_AMOUNT)?)?.abs()
            } else {
                parse_btc_to_sat(SRC, row.line, "Sent Amount", row.get(SRC, cols::SENT_AMOUNT)?)?.abs()
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
                        EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: fee, basis_source: BasisSource::ExchangeProvided }),
                    )
                }
                "income" => {
                    let (fmv, status) = resolve_fmv(None, date, sat, prices); // no export USD → dataset
                    (
                        Direction::In,
                        EventPayload::Income(Income { sat, usd_fmv: fmv, fmv_status: status, kind: IncomeKind::Reward, business: false }),
                    )
                }
                "interest" => {
                    let (fmv, status) = resolve_fmv(None, date, sat, prices);
                    (
                        Direction::In,
                        EventPayload::Income(Income { sat, usd_fmv: fmv, fmv_status: status, kind: IncomeKind::Interest, business: false }),
                    )
                }
                "withdrawal" => (
                    Direction::Out,
                    EventPayload::TransferOut(TransferOut { sat, fee_sat: None, dest_addr: None, txid: None }),
                ),
                _ => {
                    out.unclassified += 1;
                    (Direction::Trade, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
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
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters --test river`
- [ ] **Step 5: Gate + commit.**
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): River CSV parser (§9.1) — semantic source_ref, Income/Interest dataset FMV"
```

---
### Task 8: Swan parser (§9.1) — 3 files = 1 batch, role routing, native `Transaction ID`, basis-gap note

**Files:** Create `src/sources/swan.rs`; Modify `src/lib.rs`. Test `tests/swan.rs`.

**Interfaces — Produces:** `pub struct Swan` (`impl Adapter`). `group` merges all detected Swan files into ONE `FileGroup`; `parse` routes each file to a `TableRole` (`SwanTrades`/`SwanTransfers`/`SwanWithdrawals`) by header signature.

**§9.1 mapping (module doc states these; confirmed per-role schemas):**
- *trades* (universal Received/Sent shape, empty `Tag`, no id, `Date`=`MM/DD/YYYY`): `Received Currency`==BTC → `Acquire` (usd_cost=`Sent Quantity`, fee=`Fee Amount`, sat=`Received Quantity`); BTC on the *sent* side (unexpected disposition) → `Unclassified` (never guess a sell); no BTC → drop. `source_ref` = **semantic** (no id column).
- *transfers* (`Event` discriminator deposit/purchase/monthly_fee/prepaid_fee; native `Transaction ID`; `Date`=`…+00`+`Timezone`; sat=`Unit Count`; `Asset Type`→FR2): `purchase`→`Acquire` (usd_cost=`Transaction USD`, fee=`Fee USD`); `deposit`→`TransferIn` (**`USD Cost Basis`+`Acquisition Date` have NO home on core's `TransferIn` — dropped at ingest, re-supplied by reconciliation — see FOUND GAP**); `monthly_fee`/`prepaid_fee`→`Unclassified`. `source_ref` = native `Transaction ID` (direction-scoped per `Event`).
- *withdrawals* (implicit type; `Created At`=`…+00`+`Timezone`; sat=`Bitcoin Amount`): → `TransferOut`. `source_ref` = **semantic** — the `Transaction ID` column is present but NOT a stable per-row id in the confirmed data (treated as id-less; owner to confirm).

**No on-chain txid column in any Swan role** (`txid`=None throughout). Swan is BTC-only (the trades/transfers currency-/`Asset Type` guard still drops any non-BTC row).

- [ ] **Step 1: Failing test in `tests/swan.rs`**
```rust
use btctax_adapters::adapter::{Adapter, SourceFile};
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
    let files = vec![SourceFile::new(&t), SourceFile::new(&x), SourceFile::new(&w)];
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
    assert_eq!(out.events.iter().filter(|e| matches!(&e.payload, EventPayload::Acquire(_))).count(), 2);

    // the trade Acquire (sat 0.10 BTC): cost = Sent Quantity, fee = Fee Amount.
    let trade = out.events.iter().find_map(|e| match &e.payload {
        EventPayload::Acquire(a) if a.sat == 10_000_000 => Some(a),
        _ => None,
    }).unwrap();
    assert_eq!(trade.usd_cost.to_string(), "8400.00");
    assert_eq!(trade.fee_usd.to_string(), "40.00");

    // deposit → TransferIn (sat = Unit Count; no txid column → None); native Transaction ID source_ref.
    let tin = out.events.iter().find(|e| matches!(&e.payload, EventPayload::TransferIn(_))).unwrap();
    match &tin.payload {
        EventPayload::TransferIn(t) => {
            assert_eq!(t.sat, 5_000_000);
            assert_eq!(t.txid, None);
        }
        _ => unreachable!(),
    }
    assert!(out.events.iter().any(|e| matches!(&e.payload, EventPayload::TransferOut(_))));

    // source_refs: purchase = native Transaction ID (dir trade); deposit = native (dir in);
    // trades + withdrawals are id-less → semantic (direction-scoped).
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|swan|trade|sw-p1")); // purchase
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|swan|in|sw-x1")); // deposit
    assert!(out.events.iter().any(|e| e.id.canonical() == "import|swan|trade|sw-f1")); // monthly_fee (Unclassified)
    assert!(out.events.iter().any(|e| e.id.canonical().starts_with("import|swan|trade|") && e.id.canonical().contains('#'))); // trade (semantic)
    assert!(out.events.iter().any(|e| e.id.canonical().starts_with("import|swan|out|"))); // withdrawal (semantic)
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters --test swan`

- [ ] **Step 3: Implement `src/sources/swan.rs`**
```rust
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
    Acquire, BasisSource, EventId, EventPayload, LedgerEvent, PriceProvider, Source, TransferIn,
    TransferOut, Unclassified, Usd,
};

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
        id_ref: btctax_core::SourceRef,
        utc: time::OffsetDateTime,
        tz: time::UtcOffset,
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
            rows.extend(read_table(&f.path, role, &opts_for(role))?);
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
        let mut out = GroupOutput::default();
        out.parsed_rows = rows.len();
        for row in &rows {
            match row.role {
                // trades: universal Received/Sent shape; the BTC leg is whichever side is BTC. Trades are
                // buys (BTC received for USD sent); BTC on the sent side (an unexpected disposition) →
                // Unclassified (never guess a sell). No id column → semantic source_ref.
                TableRole::SwanTrades => {
                    let recv_is_btc =
                        row.opt(cols::T_RECV_CUR).unwrap_or("").eq_ignore_ascii_case(ASSET_BTC);
                    let sent_is_btc =
                        row.opt(cols::T_SENT_CUR).unwrap_or("").eq_ignore_ascii_case(ASSET_BTC);
                    if !recv_is_btc && !sent_is_btc {
                        out.dropped_no_btc += 1;
                        continue;
                    }
                    let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::T_DATE)?)?;
                    let utc_ms = (utc.unix_timestamp_nanos() / 1_000_000) as i64;
                    if recv_is_btc {
                        let sat = parse_btc_to_sat(SRC, row.line, "Received Quantity", row.get(SRC, cols::T_RECV_QTY)?)?.abs();
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
                            EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: fee, basis_source: BasisSource::ExchangeProvided }),
                        ));
                    } else {
                        let sat = parse_btc_to_sat(SRC, row.line, "Sent Quantity", row.get(SRC, cols::T_SENT_QTY)?)?.abs();
                        out.unclassified += 1;
                        let id_ref = mint.semantic(Direction::Trade, utc_ms, "trade", sat);
                        out.events.push(self.mk(id_ref, utc, tz, EventPayload::Unclassified(Unclassified { raw: raw_of(row) })));
                    }
                }
                // transfers: `Event` discriminator; native `Transaction ID` source_ref (dir per Event).
                TableRole::SwanTransfers => {
                    let asset = row.opt(cols::X_ASSET_TYPE).unwrap_or("");
                    if !asset.eq_ignore_ascii_case(ASSET_BTC) {
                        out.dropped_no_btc += 1; // FR2
                        continue;
                    }
                    let sat = parse_btc_to_sat(SRC, row.line, "Unit Count", row.get(SRC, cols::X_UNIT_COUNT)?)?.abs();
                    let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::X_DATE)?)?;
                    let id = row.get(SRC, cols::X_TXN_ID)?;
                    let (dir, payload): (Direction, EventPayload) =
                        match row.get(SRC, cols::X_EVENT)?.to_ascii_lowercase().as_str() {
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
                                    EventPayload::Acquire(Acquire { sat, usd_cost: cost, fee_usd: fee, basis_source: BasisSource::ExchangeProvided }),
                                )
                            }
                            // FOUND GAP: USD Cost Basis + Acquisition Date have no home on TransferIn
                            // (dropped; reconciliation re-supplies them for externally-sourced coins).
                            "deposit" => (
                                Direction::In,
                                EventPayload::TransferIn(TransferIn { sat, src_addr: None, txid: None }),
                            ),
                            // Fee events: could be a BTC spend/disposition OR a USD-only fee — do not assume.
                            "monthly_fee" | "prepaid_fee" => {
                                out.unclassified += 1;
                                (Direction::Trade, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
                            }
                            _ => {
                                out.unclassified += 1;
                                (Direction::Trade, EventPayload::Unclassified(Unclassified { raw: raw_of(row) }))
                            }
                        };
                    out.events.push(self.mk(mint.native(dir, id), utc, tz, payload));
                }
                // withdrawals: implicit TransferOut; `Transaction ID` is not stable → semantic source_ref.
                TableRole::SwanWithdrawals => {
                    let sat = parse_btc_to_sat(SRC, row.line, "Bitcoin Amount", row.get(SRC, cols::W_BTC_AMOUNT)?)?.abs();
                    if sat == 0 {
                        out.dropped_no_btc += 1; // defensive (Swan is BTC-only)
                        continue;
                    }
                    let (utc, tz) = parse_timestamp(SRC, row.line, row.get(SRC, cols::W_CREATED_AT)?)?;
                    let utc_ms = (utc.unix_timestamp_nanos() / 1_000_000) as i64;
                    let id_ref = mint.semantic(Direction::Out, utc_ms, "withdrawal", sat);
                    out.events.push(self.mk(
                        id_ref,
                        utc,
                        tz,
                        EventPayload::TransferOut(TransferOut { sat, fee_sat: None, dest_addr: None, txid: None }),
                    ));
                }
                // Unreachable: Swan `parse` always assigns a Swan role (never `Single`).
                TableRole::Single => continue,
            }
        }
        Ok(out)
    }
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters --test swan`
- [ ] **Step 5: Gate + commit.** Record the Swan transfer-basis FOUND GAP in `FOLLOWUPS.md`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): Swan 3-file batch parser (§9.1) — role routing + Event discriminator, native/semantic refs"
```

---
### Task 9: Ingest orchestration — detect → group → dispatch → `IngestBatch` (FR2 reporting)

**Files:** Create `src/ingest.rs`; Modify `src/lib.rs`.

**Interfaces — Consumes:** all four adapters, `adapter` types. **Produces:** `pub fn ingest_files(paths, &dyn PriceProvider) -> Result<IngestBatch, AdapterError>` — the FR2 capstone: detects each file's source, buckets, groups (Swan 3→1), dispatches, and emits one `FileReport` per group with dropped/unclassified counts.

- [ ] **Step 1: Failing test in `tests/integration.rs`** (orchestration slice — fuller multi-source assertions land in Task 11)
```rust
use btctax_adapters::price::BundledPrices;
use btctax_adapters::{ingest_files, AdapterError};

#[test]
fn unrecognized_file_is_a_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mystery.csv");
    std::fs::write(&path, "foo,bar\n1,2\n").unwrap();
    let prices = BundledPrices::load().unwrap();
    let err = ingest_files(&[path], &prices).unwrap_err();
    assert!(matches!(err, AdapterError::UnknownSource { .. }));
}
```

- [ ] **Step 2: Run → FAIL.** `cargo test -p btctax-adapters --test integration unrecognized`

- [ ] **Step 3: Implement `src/ingest.rs`**
```rust
//! Ingest orchestration: detect each file's source, group (Swan merges its files), dispatch to the
//! parser, and aggregate FR2 counts into one `FileReport` per group. Produces events only — the CLI
//! (Plan 4) persists them via `btctax_core::persistence::append_import_batch`.
use crate::adapter::{Adapter, FileReport, IngestBatch, SourceFile};
use crate::sources::{coinbase::Coinbase, gemini::Gemini, river::River, swan::Swan};
use crate::AdapterError;
use btctax_core::{PriceProvider, Source};
use std::collections::HashMap;
use std::path::PathBuf;

fn adapters() -> Vec<Box<dyn Adapter>> {
    // Detection order: highest-specificity detectors first. Swan/Coinbase/River detect by CSV
    // header tokens (high specificity — content-based). Gemini detects by .xlsx extension alone
    // (very broad: any .xlsx file matches). By running Gemini last, the content-based detectors
    // claim their files first; Gemini only picks up .xlsx files that the others declined.
    // Coinbase and River also explicitly return false for .xlsx extensions, so no cross-source
    // confusion is possible regardless of order — but Gemini last is the correct idiom.
    vec![Box::new(Swan), Box::new(Coinbase), Box::new(River), Box::new(Gemini)]
}

/// FR2 capstone. Detect → bucket → group → parse → normalize → report (dropped/unclassified per group).
pub fn ingest_files(
    paths: &[PathBuf],
    prices: &dyn PriceProvider,
) -> Result<IngestBatch, AdapterError> {
    let adapters = adapters();
    let mut buckets: HashMap<Source, Vec<SourceFile>> = HashMap::new();
    for p in paths {
        let f = SourceFile::new(p.clone());
        let mut matched = None;
        for a in &adapters {
            if a.detect(&f)? {
                matched = Some(a.source());
                break;
            }
        }
        match matched {
            Some(s) => buckets.entry(s).or_default().push(f),
            None => {
                return Err(AdapterError::UnknownSource {
                    path: p.display().to_string(),
                })
            }
        }
    }

    let mut batch = IngestBatch::default();
    for a in &adapters {
        let Some(files) = buckets.remove(&a.source()) else {
            continue;
        };
        for group in a.group(files) {
            let rows = a.parse(&group)?;
            let out = a.normalize(&group, rows, prices)?;
            batch.reports.push(FileReport {
                source: group.source,
                label: group.label.clone(),
                parsed_rows: out.parsed_rows,
                btc_events: out.events.len(),
                dropped_no_btc: out.dropped_no_btc,
                unclassified: out.unclassified,
            });
            batch.events.extend(out.events);
        }
    }
    Ok(batch)
}
```

- [ ] **Step 4: Run → PASS.** `cargo test -p btctax-adapters --test integration unrecognized`
- [ ] **Step 5: Wire + gate + commit.** Add `pub mod ingest;` + `pub use ingest::ingest_files;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): ingest orchestration (detect/group/dispatch) + FR2 per-file reporting"
```

---

### Task 10: FR3 FMV wiring end-to-end — `ingest_files_bundled` + the Resolved/Missing/ExchangeProvided matrix

**Files:** Modify `src/ingest.rs` (add `ingest_files_bundled`); Modify `src/lib.rs`. Test `tests/fmv_fr3.rs`.

**Interfaces — Produces:** `pub fn ingest_files_bundled(paths) -> Result<IngestBatch, AdapterError>` — loads `BundledPrices` and calls `ingest_files`, making the bundled daily-close dataset the FR3 default for the whole pipeline. The test proves the three FR3 outcomes flow through real `ingest`.

- [ ] **Step 1: Add to `src/ingest.rs`**
```rust
use crate::price::BundledPrices;

/// FR3-wired entrypoint: resolve ingest-time FMV against the bundled daily-close dataset (§9.2).
pub fn ingest_files_bundled(paths: &[PathBuf]) -> Result<IngestBatch, AdapterError> {
    let prices = BundledPrices::load()?;
    ingest_files(paths, &prices)
}
```

- [ ] **Step 2: Failing test in `tests/fmv_fr3.rs`** (one River file exercising all three FR3 statuses)
```rust
use btctax_adapters::ingest_files_bundled;
use btctax_core::{EventPayload, FmvStatus};

// River rows (REAL §9.1 headers, invented values): Interest on a dataset date (→ PriceDataset),
// Interest on a NON-dataset date (→ Missing), Buy (no FMV needed). Bundled dataset has 2025-06-15 but
// NOT 2025-07-04.
const CSV: &str = "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-07-04 00:00:00,,,0.00010000,BTC,,,Interest\r\n\
2025-03-01 12:00:00,4200.00,USD,0.05000000,BTC,3.00,USD,Buy\r\n";

#[test]
fn fr3_matrix_through_full_pipeline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("river.csv");
    std::fs::write(&path, CSV).unwrap();
    let batch = ingest_files_bundled(&[path]).unwrap();

    let statuses: Vec<FmvStatus> = batch
        .events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::Income(i) => Some(i.fmv_status.clone()),
            _ => None,
        })
        .collect();
    assert!(statuses.contains(&FmvStatus::PriceDataset)); // 2025-06-15 present
    assert!(statuses.contains(&FmvStatus::Missing)); // 2025-07-04 absent → Missing blocker
    assert_eq!(statuses.iter().filter(|s| **s == FmvStatus::PriceDataset).count(), 1);
    // A Missing income still produces the sat-bearing event (never dropped); core gates its amount.
    assert_eq!(batch.events.iter().filter(|e| matches!(&e.payload, EventPayload::Income(_))).count(), 2);
}
```

- [ ] **Step 3: Run → FAIL → implement (Step 1 already adds the fn) → PASS.** `cargo test -p btctax-adapters --test fmv_fr3`
- [ ] **Step 4: Wire + gate + commit.** Add `pub use ingest::ingest_files_bundled;` to `lib.rs`.
```bash
cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "feat(adapters): FR3 FMV wiring (ingest_files_bundled) + ExchangeProvided/PriceDataset/Missing matrix"
```

---

### Task 11: Multi-source integration test — synthetic batch across all four sources → events

**Files:** Modify `tests/integration.rs`.

**Interfaces — Consumes:** the whole public surface. A realistic synthetic batch (Coinbase CSV + Gemini XLSX + River CSV + Swan 3 CSVs) proves end-to-end detection, grouping, FR2 counts, FMV statuses, source-priority co-existence, and stable `EventId`s.

- [ ] **Step 1: Failing test in `tests/integration.rs`**
```rust
use btctax_adapters::ingest_files_bundled;
use btctax_core::{EventPayload, Source};
use rust_xlsxwriter::Workbook;

fn write_gemini(path: &std::path::Path) {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let header = [
        "Date", "Time (UTC)", "Type", "Symbol", "BTC Amount BTC", "USD Amount USD", "Fee (USD) USD",
        "BTC Balance BTC", "Trade ID", "Order ID", "Tx Hash", "Deposit Destination", "Withdrawal Destination",
    ];
    for (c, h) in header.iter().enumerate() {
        ws.write_string(0, c as u16, *h).unwrap();
    }
    let rows: [[&str; 13]; 2] = [
        ["2025-03-01 12:00:00", "2025-03-01 12:00:00", "Buy", "BTCUSD", "0.02000000", "1680.00", "5.00", "0.02", "GT-1", "GO-1", "", "", ""],
        ["2025-03-02 11:00:00", "2025-03-02 11:00:00", "Credit", "BTC", "0.00100000", "", "", "0.021", "", "", "feedface", "bc1qdp", ""],
    ];
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            ws.write_string((r + 1) as u32, c as u16, *v).unwrap();
        }
    }
    wb.save(path).unwrap();
}

#[test]
fn multi_source_batch_ingests_into_events() {
    let dir = tempfile::tempdir().unwrap();

    // Coinbase: REAL 13-col header, 3-line preamble; cb-1 BTC Buy + cb-2 ETH (dropped).
    let cb = dir.path().join("coinbase.csv");
    std::fs::write(&cb, "\r\nTransactions\r\nUser,00000000-0000-0000-0000-000000000000\r\n\
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address\r\n\
cb-1,2025-03-01 12:00:00 UTC,Buy,BTC,0.01000000,USD,84000.00,840.00,845.00,5.00,,,\r\n\
cb-2,2025-03-01 08:00:00 UTC,Buy,ETH,1.00000000,USD,2000.00,2000.00,2010.00,10.00,,,\r\n").unwrap();

    let gm = dir.path().join("gemini.xlsx");
    write_gemini(&gm);

    let rv = dir.path().join("river.csv");
    std::fs::write(&rv, "Date,Sent Amount,Sent Currency,Received Amount,Received Currency,Fee Amount,Fee Currency,Tag\r\n\
2025-06-15 00:00:00,,,0.00010000,BTC,,,Interest\r\n").unwrap();

    // Swan 3-file batch (real per-role headers; transfers/withdrawals carry the 2-line preamble).
    let st = dir.path().join("swan_trades.csv");
    std::fs::write(&st, "Date,Received Quantity,Received Currency,Sent Quantity,Sent Currency,Fee Amount,Fee Currency,Tag\n\
03/01/2025 12:00:00,0.10000000,BTC,8400.00,USD,40.00,USD,\n").unwrap();
    let sx = dir.path().join("swan_transfers.csv");
    std::fs::write(&sx, "Swan Bitcoin Inc\n123 Main St\n\
Event,Date,Timezone,Status,Transaction ID,Total USD,Transaction USD,Fee USD,Unit Count,Asset Type,BTC Price,Address Label,USD Cost Basis,Acquisition Date\n\
deposit,2025-03-02 09:00:00+00,UTC,settled,sw-x1,3000.00,3000.00,0,0.05000000,BTC,60000.00,cold,3000.00,2024-01-15\n").unwrap();
    let sw = dir.path().join("swan_withdrawals.csv");
    std::fs::write(&sw, "Swan Bitcoin Inc\n123 Main St\n\
Created At,Timezone,Transaction ID,Executed At,Canceled At,Status,Bitcoin Amount,Automatic,IP Address\n\
2025-03-03 10:00:00+00,UTC,sw-w1,2025-03-03 10:05:00+00,,settled,0.02000000,true,1.2.3.4\n").unwrap();

    let batch = ingest_files_bundled(&[cb, gm, rv, st, sx, sw]).unwrap();

    // One report per group: Coinbase(1) + Gemini(1) + River(1) + Swan(1 batch) = 4 reports.
    assert_eq!(batch.reports.len(), 4);
    let swan_report = batch.reports.iter().find(|r| r.source == Source::Swan).unwrap();
    assert_eq!(swan_report.btc_events, 3); // trades+transfers+withdrawals merged into one report
    let cb_report = batch.reports.iter().find(|r| r.source == Source::Coinbase).unwrap();
    assert_eq!(cb_report.dropped_no_btc, 1); // ETH row

    // Events: CB buy(1) + Gemini buy(1)+credit-TransferIn(1) + River interest(1) + Swan(3) = 7.
    assert_eq!(batch.events.len(), 7);
    // N-4: assert the absence of Unclassified with a diagnostic message naming which event IDs
    // failed, so a mis-routed event type (e.g. Gemini Credit → Unclassified instead of TransferIn,
    // or a Coinbase/Swan arm silently falling to the catch-all) is immediately identifiable.
    // This fixture has no Order/Exchange/Pro Coinbase rows, no Gemini unknown types, and no Swan
    // fee rows — so zero Unclassified is the correct tight assertion.
    let unclassified_ids: Vec<_> = batch.events.iter()
        .filter(|e| matches!(&e.payload, EventPayload::Unclassified(_)))
        .map(|e| e.id.canonical())
        .collect();
    assert!(unclassified_ids.is_empty(),
        "unexpected Unclassified events ({} found): {:?}", unclassified_ids.len(), unclassified_ids);
    // Two TransferIns: Gemini Credit(BTC) + Swan deposit.
    assert_eq!(batch.events.iter().filter(|e| matches!(&e.payload, EventPayload::TransferIn(_))).count(), 2);
    assert!(batch.events.iter().any(|e| matches!(&e.payload, EventPayload::Income(_)))); // River interest

    // Stable, source-scoped EventIds across all four venues coexist.
    assert!(batch.events.iter().any(|e| e.id.canonical() == "import|coinbase|trade|cb-1"));
    assert!(batch.events.iter().any(|e| e.id.canonical() == "import|gemini|trade|GT-1.GO-1"));
    assert!(batch.events.iter().any(|e| e.id.canonical() == "import|swan|in|sw-x1"));
    assert!(batch.events.iter().any(|e| e.id.canonical().starts_with("import|river|in|")));
}
```

- [ ] **Step 2: Run → PASS** (all parsers already implemented). `cargo test -p btctax-adapters --test integration`
- [ ] **Step 3: Full-suite gate + commit.**
```bash
cargo test -p btctax-adapters && cargo clippy --all-targets -p btctax-adapters -- -D warnings && cargo fmt --check
git commit -am "test(adapters): multi-source synthetic integration (detect/group/FR2/FR3/source_ref)"
```

---
## Self-Review — spec coverage map (every §9 sub-requirement + FR2/FR3/NFR5/§9.2 → its task)

**§9 pipeline & adapter contract:**
- **detect** → Task 4 (`Adapter::detect`), implemented per source in Tasks 5–8, dispatched in Task 9.
- **group** (Swan 3→1) → Task 8 (`Swan::group`) + Task 4 (trait); single-file groups in Tasks 5/6/7.
- **strip preamble (CRLF)** → Task 2 (`read_csv` header-token scan + `csv` CRLF handling); Coinbase 3-line preamble exercised in Task 5.
- **parse** → Task 2 (`RawRow`/readers) + each parser's `parse` (Tasks 5–8).
- **normalize** → each parser's `normalize` (Tasks 5–8) using shared helpers (Task 3).
- **`source_ref`** → Task 3 (`SourceRefMint`: native + semantic-with-occurrence-index); native used by Coinbase/Gemini/Swan, semantic by River + id-less Gemini rows.
- **validate / atomic append** → **out of this crate** (Plan 2 `persistence::append_import_batch` + Plan 4 CLI). The adapter produces stable `EventId::Import { source, source_ref }` so the CLI gets idempotency + `ImportConflict` for free; module docs (Tasks 5–8) state each source's dedup/gross-net/fee/unknown policy (the §9 "module doc + fixture test" requirement).

**§9.1 per-source mappings (confirmed enums, exhaustive):**
- **Coinbase** (`Transaction Type` ∈ {Buy, Sell, Send, Receive, Withdrawal, Order, Exchange Deposit, Exchange Withdrawal, Pro Deposit, Pro Withdrawal}): Buy→Acquire (basis=Total=Subtotal+Fees), Sell→Dispose{Sell} (gross=Subtotal), Send/Withdrawal→TransferOut (dest=Recipient Address), Receive→TransferIn (src=Sender Address), Order + the four Exchange/Pro internal-move types + any unknown→Unclassified; `Asset`≠BTC drop. No Convert/reward in the confirmed vocabulary. → **Task 5**.
- **Gemini** (`Type` ∈ {Buy, Sell, Credit, Debit}): Buy→Acquire, Sell→Dispose{Sell} (gross=USD Amount USD + Fee (USD) USD), Debit→TransferOut, **Credit(BTC)→TransferIn**; USD-cash Credit/Debit dropped; native Trade ID+Order ID else semantic; Tx Hash txid; Deposit/Withdrawal Destination address; Excel-serial dates; BTC Balance BTC captured. → **Task 6**.
- **River** (`Tag` ∈ {Buy, Income, Interest, Withdrawal}): Buy→Acquire (usd_cost=Sent Amount, fee=Fee Amount, sat=Received Amount), Income→Income{Reward} / Interest→Income{Interest} (dataset FMV), Withdrawal→TransferOut (sat=Sent Amount); universal Sent/Received BTC-leg test; semantic source_ref. → **Task 7**.
- **Swan** (3 files=1 batch): trades (no `Tag`/id)→Acquire (semantic ref); transfers (`Event` ∈ {deposit, purchase, monthly_fee, prepaid_fee})→ deposit:TransferIn / purchase:Acquire / monthly_fee,prepaid_fee:Unclassified (native Transaction ID); withdrawals→TransferOut (semantic ref). No on-chain txid in any role. → **Task 8**.

**§9.2 price dataset:** bundled daily-close CSV + `BundledPrices: PriceProvider` → **Task 1**; threaded as the FR3 default → **Task 10**.

**FR2 (BTC-only):** drop only no-BTC-leg rows (counted), unknown BTC-side → `Unclassified` (never dropped), report per file. Per-parser drop/unclassify logic → **Tasks 5–8**; aggregate `FileReport` (dropped/unclassified counts per group) → **Task 9**; asserted multi-source → **Task 11**.

**FR3 (ingest-time FMV):** prefer export USD → dataset → `Missing`. Helper `resolve_fmv` → **Task 3**; applied at ingest only by River `Income`/`Interest` (the sole no-USD income rows) → **Task 7**; end-to-end matrix + bundled wiring → **Task 10**. (Coinbase carries its own USD or is a transfer/Unclassified, so it needs no `PriceProvider`. `ManualEntry` is never produced at ingest — it is a core/CLI `ManualFmv` decision; stated in Global Constraints.)

**NFR5 (exact arithmetic):** `parse_usd`/`parse_btc_to_sat` (string→Decimal→integer sats; fractional-sat error; no float money) → **Task 0**; XLSX float→shortest-round-trip-string→exact parse (documented bound) → **Task 2**; used everywhere.

**Out of scope (correctly not placed):** projection/lot engine/`verify`/reconciliation (Plan 2), persistence-append/CLI (Plans 2/4), non-BTC/optimizer/forms (other phases). The crate only produces events.

**Cross-task type/signature consistency (verified against the read btctax-core source):**
- Event construction matches `LedgerEvent { id, utc_timestamp: OffsetDateTime, original_tz: UtcOffset, wallet: Option<WalletId>, payload }` and `EventId::import(Source, SourceRef)` exactly (every parser, Tasks 5–8).
- Payload field names/types match core verbatim: `Acquire { sat: Sat, usd_cost: Usd, fee_usd: Usd, basis_source: BasisSource }`, `Income { sat, usd_fmv: Option<Usd>, fmv_status: FmvStatus, kind: IncomeKind, business: bool }`, `Dispose { sat, usd_proceeds: Usd, fee_usd: Usd, kind: DisposeKind }`, `TransferOut { sat, fee_sat: Option<Sat>, dest_addr: Option<String>, txid: Option<String> }`, `TransferIn { sat, src_addr, txid }`, `Unclassified { raw: String }`.
- Enums used by value match core: `FmvStatus::{ExchangeProvided, PriceDataset, Missing}` (Manual not at ingest), `BasisSource::{ExchangeProvided, ComputedFromCost}`, `IncomeKind::{Mining,Staking,Interest,Airdrop,Reward}`, `DisposeKind::Sell`, `Source::{Coinbase,Gemini,River,Swan}`, `WalletId::Exchange{provider,account}`.
- Trait/method paths verified present in core: `btctax_core::PriceProvider::usd_per_btc(&self, TaxDate) -> Option<Usd>` (implemented by `BundledPrices`); `btctax_core::price::fmv_of(&dyn PriceProvider, TaxDate, Sat) -> Option<Usd>` and `btctax_core::price::StaticPrices` (test stub); `btctax_core::conventions::tax_date(OffsetDateTime, UtcOffset) -> Date`; `Sat = i64`, `Usd = Decimal`, `TaxDate = time::Date`; `EventId::canonical()` produces `import|<tag>|<source_ref>` (so the tests' `"import|coinbase|trade|cb-1"` form is exactly `import|{Source::tag()}|{dir}|{id}`). `Source::tag()` returns `swan|coinbase|gemini|river`.
- `resolve_fmv` returns `(Option<Usd>, FmvStatus)` and is consumed identically by Coinbase (Task 5) and River (Task 7). `SourceRefMint::native(&self,…)`/`semantic(&mut self,…)` arity matches all call sites. `GroupOutput`/`FileReport`/`IngestBatch` shapes (Task 4) are populated, never reshaped, by Tasks 5–11.
- `AdapterError` is the single error type across the crate (Task 0), used by every fallible fn; thiserror style matches `btctax-store::StoreError`/`btctax-core::CoreError`.

**Confirmed-schema folds + gaps handled inline:**
- *Gemini `Credit`(BTC) → `TransferIn` (not Unclassified).* The confirmed mapping (§9.1) routes inbound on-chain BTC to `TransferIn` (reconciliation supplies basis); `Debit`(BTC)→`TransferOut`. Spec §13/§14 R2 were aligned to match.
- *Coinbase `Exchange/Pro Deposit/Withdrawal` + `Order` → `Unclassified`.* The confirmed 2012-2019 vocabulary has no `Convert`/reward; internal-move types are likely self-transfers but require user confirmation → conservative `Unclassified` (never auto-transfer/auto-dispose). The earlier `Convert`-by-sign + reward-income logic was removed (dead for the confirmed enum; any future type → Unclassified).
- *Gemini id-less Credit/Debit rows.* Native `source_ref` = `Trade ID`+`Order ID` on trade rows; `Credit`/`Debit` lack trade ids → **semantic** fallback (same machinery as River).
- *Swan per-role columns + `Event` discriminator.* trades/transfers/withdrawals carry different headers/timestamps; transfers route on `Event` (purchase→Acquire / deposit→TransferIn / monthly_fee,prepaid_fee→Unclassified). Withdrawals `Transaction ID` is not a stable per-row id → **semantic** `source_ref` (owner to confirm — Open-schema items / FOLLOWUPS).
- *Swan transfer basis/date have no event home.* Surfaced as the **FOUND GAP**: emit a plain `TransferIn`, drop the Swan `USD Cost Basis`/`Acquisition Date` at ingest, rely on reconciliation. No silent data invention.
- *Confirmed timestamp formats + Excel serials.* `parse_timestamp` now handles Coinbase `… UTC`, Swan `…+00`, Swan US-locale `MM/DD/YYYY`; `parse_timestamp_flex`/`excel_serial_to_utc` convert Gemini's numeric serials (anchor: serial 25569 = 1970-01-01). Float is used for the serial only (a timestamp, not money — NFR5 unaffected).
- *XLSX float money vs NFR5.* Resolved by shortest-round-trip stringification + exact parse (Task 2), with the precision bound documented (FOLLOWUPS), instead of a lossy `f64→Decimal`.

## Notes for Plan 4 (CLI) and FOLLOWUPS candidates
- **Plan 4 `btctax-cli`:** call `ingest_files_bundled(paths)` (or `ingest_files` with an injected provider for tests), then `btctax_core::persistence::append_import_batch(vault.conn(), &batch.events)` inside a session; surface `batch.reports` (dropped/unclassified per file) to the user; drive `reconcile` to resupply Swan transfer basis/date for externally-sourced inbounds.
- **FOLLOWUPS candidates (record after first build):** the **remaining owner questions** above (Swan withdrawals `Transaction ID` stability → native vs semantic ref; Swan `Total/Transaction USD` purchase-cost semantics; Coinbase internal-move `Unclassified` default); the **Swan transfer-basis FOUND GAP** (cross-crate, already appended to FOLLOWUPS); the XLSX-float→decimal precision bound; the River/Swan-trades/Swan-withdrawals/Gemini-Credit-Debit `occurrence_index` file-order fragility (already in spec FOLLOWUPS); single-account `WalletId` (multi-account future); pin the resolved `csv`/`calamine`/`rust_xlsxwriter` versions and re-verify the `calamine::Data` variant list. (All `// OPEN` header constants are now resolved against the confirmed real schemas — §9.1.)

## Fold record
(To be completed when independent reviews are folded, per STANDARD_WORKFLOW §2 — persist each reviewer's output verbatim under `reviews/` before folding; re-review after every fold, including the last; gate is 0 Critical / 0 Important from both reviewers.)

---

## Fold record (round 1) — 2026-06-29

Findings folded from the round-1 review. Each entry: finding → exact location changed → how fixed.

| Finding | Location | Fix applied |
|---------|----------|-------------|
| **IP-1** (Important) — `Data::DateTime` `dt.to_string()` loses time component → wrong `utc_ms` for Gemini serial path | Task 2 `cell_to_string`; Task 6 fixture | (1) `DateTime` arm changed to `format!("{}", dt.as_f64())` — extracts serial as `f64` string, flows through `Data::Float → parse_timestamp_flex(serial)` path. Added first-build verification comment: confirm `as_f64()` accessor or delete arm if `Data::DateTime` absent in calamine 0.26. (2) Task 6 Gemini fixture: Buy row's `Date` cell now written with `ws.write_number(1, 0, 45717.5f64)` (serial, not string) — exercises the numeric→serial→UTC path end-to-end. |
| **M-1** — Gemini integration fixture all cells `write_string`; numeric→serial path never exercised | Task 6 Step 1 `write_fixture` | Restructured `write_fixture`: Buy row writes `Date` column with `ws.write_number(1, 0, 45717.5f64)` (serial 45717.5 ≈ 2025-03-01 12:00:00 UTC). Remaining cells/rows use `write_string`. Added comment explaining the serial anchor. |
| **M-2** — `read_xlsx` "no worksheet" used `AdapterError::PriceDataset` (wrong category) | Task 2 `read_xlsx` | Changed `.ok_or_else(|| AdapterError::PriceDataset(…))` → `.ok_or_else(|| AdapterError::EmptyXlsx { path: … })`. Added new `EmptyXlsx { path }` variant to `AdapterError` with its own `#[error]` message. Updated public interface variant list. |
| **M-3** — calamine 0.26 `Data` variant list not confirmed; `DateTime`/`DateTimeIso`/`DurationIso` arms may not exist | Task 0 Step 2 (pin note) | Added explicit first-build verification checklist item (a): confirm all three variants; delete any arm absent from resolved 0.26 enum. |
| **M-4** — Coinbase `Order` routed by silent `_` catch-all (known type, implicit) | Task 5 Step 3 `normalize` match | Added explicit `"order" => { … }` arm before `_`. Comment explains the rationale (known Coinbase type; explicit > implicit). The `_` arm still catches truly unknown/future types. Match remains exhaustive. |
| **M-5** — Gemini `Credit`'s `Deposit Destination` stored in `TransferIn.src_addr` without naming caveat; Plan-4 reconciler may misinterpret it as the true on-chain source | Task 6 Step 3 module doc | Added `//! NOTE (M-5)` doc block to `gemini.rs` module doc: `Deposit Destination` = Gemini's own receiving address (on-chain destination of the inbound), NOT the originating sender's address. `TransferIn.src_addr` field holds the Gemini-side address. |
| **M-6** — `AdapterError::IncompleteSwanBatch` misnames the trigger (actual: unrecognized role, not missing file) | `AdapterError` enum (Task 0/lib.rs); Swan `parse` (Task 8) | Renamed to `UnrecognizedSwanRole { path: String }` with updated error message. Usage in `Swan::parse` changed from `ok_or(…)` → `ok_or_else(|| … { path: f.path.display().to_string() })`. Updated public interface variant list. |
| **M-7** — `Decimal::from_scientific` may not exist in rust_decimal 1.36 | Task 0 Step 2 (pin note) | Added first-build verification checklist item (c): confirm `from_scientific` exists; if absent or covered by `from_str`, remove the `.or_else` fallback. |
| **N-3** — `adapters()` detection-order comment said "so an .xlsx with other signatures is not misrouted" — backwards rationale | Task 9 Step 3 `adapters()` | Rewrote comment: Gemini is last because its detection is extension-only (very broad); content-based detectors (Swan/Coinbase/River) run first to claim their files. Also notes Coinbase/River explicitly return false for .xlsx. |
| **N-4** — Integration test `Unclassified count == 0` is a global count with no diagnostic context | Task 11 Step 1 | Replaced `assert_eq!(…count(), 0)` with a named-ID collection + `assert!(is_empty(), "… {:?}", ids)`. Failure now prints the canonical EventIds of unexpected Unclassified events. Added comment naming which adapter arms are expected to produce zero Unclassified in this fixture. |

**Self-consistency pass (post-fold):**
- No `// OPEN` code comments remain. The `Data::DateTime` arm that was flagged OPEN is replaced with a verified approach + first-build verification comment (not OPEN).
- `AdapterError` variant names are consistent throughout: `EmptyXlsx` (Task 2 read_xlsx), `UnrecognizedSwanRole` (Task 8 Swan::parse), public interface description (line 70), and Task 0 lib.rs stub — all aligned.
- Coinbase `match` is exhaustive: all 10 confirmed `Transaction Type` values have explicit or known-catch arms (`buy`, `sell`, `send`, `withdrawal`, `receive`, `exchange deposit`, `exchange withdrawal`, `pro deposit`, `pro withdrawal`, `order`) plus `_` for future/unknown types. No type is silently swallowed.
- The Task 6 fixture restructuring keeps all 5 data rows (Buy×1 + Sell×1 + Debit×1 + Credit BTC×1 + Credit USD×1) intact; only the write method for the Buy Date cell changed (string → number). Event counts (4 BTC events, 1 dropped) are unchanged.

