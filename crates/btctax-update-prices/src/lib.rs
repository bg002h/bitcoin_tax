//! `btctax-update-prices` — the opt-in ONLINE updater (#41 Part C).
//!
//! This is the ONLY crate in the btctax workspace that links an HTTP client (`ureq`, rustls-tls,
//! blocking). It fetches gap/recent daily BTC/USD closes into a LOCAL price cache (`date,usd_close`
//! CSV) that the pure `btctax_adapters::LayeredPrices` provider layers over the bundled dataset. The
//! tax binaries carry NO network dependency at all (verifiable via `cargo tree`) — offline,
//! deterministic, private-by-default stays intact. Mirrors `update_prices.py`: Binance klines primary,
//! CoinGecko `market_chart/range` fallback, an 8-day settling lag, forward-only idempotent append.
//!
//! The cache is a documented LOCAL INPUT (like the vault): a projection is reproducible GIVEN
//! (events + bundled + cache); the bundled-only projection is the published-reproducible baseline.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use btctax_adapters::BundledPrices;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use time::{Date, OffsetDateTime, UtcOffset};

/// Match the bundled dataset's money convention: 2dp, banker's rounding (mirrors core's `round_cents`).
const MONEY_ROUNDING: RoundingStrategy = RoundingStrategy::MidpointNearestEven;
const USER_AGENT: &str = concat!("btctax-update-prices/", env!("CARGO_PKG_VERSION"));

/// The env var overriding the default cache location (shared convention with btctax-cli).
pub const PRICE_CACHE_ENV: &str = "BTCTAX_PRICE_CACHE";

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("adapter: {0}")]
    Adapter(#[from] btctax_adapters::AdapterError),
    #[error("io {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("json parse ({provider}): {detail}")]
    Parse {
        provider: &'static str,
        detail: String,
    },
    #[error("network ({provider}): {detail}")]
    Network {
        provider: &'static str,
        detail: String,
    },
    #[error("all price sources failed — check your network connection (binance: {binance}; coingecko: {coingecko})")]
    AllSourcesFailed { binance: String, coingecko: String },
    #[error("no cache path could be resolved (set --price-cache or $BTCTAX_PRICE_CACHE)")]
    NoCachePath,
    #[error("bad date {0:?} (expected YYYY-MM-DD)")]
    BadDate(String),
    #[error("time: {0}")]
    Time(String),
}

// ── CLI (clap derive; also drives the generated man page) ─────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum SourceArg {
    /// Binance first, CoinGecko fallback (the default).
    Auto,
    /// Binance klines only.
    Binance,
    /// CoinGecko market_chart/range only.
    Coingecko,
}

/// Fetch newer daily BTC/USD closes into the local price cache (opt-in, online).
///
/// The btctax tax binaries never touch the network; this SEPARATE tool is the only online path. It
/// appends closes AFTER the last one already known (bundled dataset + existing cache), skipping the most
/// recent `--lag` days (sources revise recent closes for ~7 days). Forward-only and idempotent: re-runs
/// append nothing already present, and it NEVER modifies the bundled dataset.
#[derive(clap::Parser, Debug)]
#[command(name = "btctax-update-prices", version, about, long_about = None)]
pub struct Cli {
    /// First date to fetch (YYYY-MM-DD). Default: the day after the last known close.
    #[arg(long, value_parser = parse_cli_date)]
    pub from: Option<Date>,
    /// Last date to fetch (YYYY-MM-DD). Default: today − <lag> days.
    #[arg(long, value_parser = parse_cli_date)]
    pub to: Option<Date>,
    /// Skip the N most recent days (the settling window; sources revise recent closes for ~7 days).
    #[arg(long, default_value_t = 8)]
    pub lag: i64,
    /// Preview what WOULD be appended; write nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// Price source (`auto` = Binance then CoinGecko fallback).
    #[arg(long, value_enum, default_value_t = SourceArg::Auto)]
    pub source: SourceArg,
    /// Override the cache path (default: $BTCTAX_PRICE_CACHE, else <data_dir>/btctax/price_cache.csv).
    #[arg(long, value_name = "PATH")]
    pub price_cache: Option<PathBuf>,
}

fn parse_cli_date(s: &str) -> Result<Date, String> {
    parse_iso_date(s).map_err(|_| format!("bad date {s:?} (expected YYYY-MM-DD)"))
}

/// Parse an ISO `YYYY-MM-DD` date.
pub fn parse_iso_date(s: &str) -> Result<Date, UpdateError> {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    Date::parse(s.trim(), &fmt).map_err(|_| UpdateError::BadDate(s.to_string()))
}

// ── Default cache path (dirs lives HERE + in btctax-cli; NOT in btctax-adapters) ──────────────────

/// The default local price-cache path: `$BTCTAX_PRICE_CACHE`, else `<data_dir>/btctax/price_cache.csv`.
pub fn default_cache_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(PRICE_CACHE_ENV) {
        return Some(PathBuf::from(p));
    }
    dirs::data_dir().map(|d| d.join("btctax").join("price_cache.csv"))
}

// ── Range computation (pure; testable) ────────────────────────────────────────────────────────────

/// The inclusive `[start, end]` date range to fetch, or `None` when already up to date. `start` =
/// `--from`, else the day after `last_known`. `end` = `--to`, else `today − lag` days (the settling
/// window). Forward-only: an empty/inverted range yields `None`.
pub fn fetch_range(
    last_known: Option<Date>,
    today: Date,
    lag: i64,
    from: Option<Date>,
    to: Option<Date>,
) -> Option<(Date, Date)> {
    let start = match from {
        Some(d) => d,
        None => last_known?.next_day()?,
    };
    let end = match to {
        Some(d) => d,
        None => (today.midnight().assume_utc() - Duration::from_secs((lag.max(0) as u64) * 86_400))
            .date(),
    };
    if start > end {
        None
    } else {
        Some((start, end))
    }
}

// ── Parsers (PURE — the canned-JSON KATs target these; no network) ────────────────────────────────

fn round_money(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, MONEY_ROUNDING)
}

fn date_from_unix_ms(ms: i64) -> Result<Date, UpdateError> {
    let secs = ms.div_euclid(1000);
    OffsetDateTime::from_unix_timestamp(secs)
        .map(|dt| dt.to_offset(UtcOffset::UTC).date())
        .map_err(|e| UpdateError::Time(e.to_string()))
}

/// Parse Binance `/api/v3/klines` (an array of candle arrays): open-time (ms, idx 0) → close (str, idx 4).
pub fn parse_binance_klines(json: &str) -> Result<BTreeMap<Date, Decimal>, UpdateError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| UpdateError::Parse {
        provider: "binance",
        detail: e.to_string(),
    })?;
    let arr = v.as_array().ok_or(UpdateError::Parse {
        provider: "binance",
        detail: "expected a top-level array of candles".into(),
    })?;
    let mut out = BTreeMap::new();
    for candle in arr {
        let c = candle.as_array().ok_or(UpdateError::Parse {
            provider: "binance",
            detail: "candle is not an array".into(),
        })?;
        let open_ms = c
            .first()
            .and_then(|x| x.as_i64())
            .ok_or(UpdateError::Parse {
                provider: "binance",
                detail: "candle[0] (open time) missing".into(),
            })?;
        let close_str = c
            .get(4)
            .and_then(|x| x.as_str())
            .ok_or(UpdateError::Parse {
                provider: "binance",
                detail: "candle[4] (close) missing/not a string".into(),
            })?;
        let close = Decimal::from_str(close_str).map_err(|e| UpdateError::Parse {
            provider: "binance",
            detail: format!("close {close_str:?}: {e}"),
        })?;
        out.insert(date_from_unix_ms(open_ms)?, round_money(close));
    }
    Ok(out)
}

/// Parse CoinGecko `market_chart/range` (`{"prices": [[ts_ms, price], …]}`), keeping the LAST point per
/// UTC calendar day (closest to the daily close).
pub fn parse_coingecko_range(json: &str) -> Result<BTreeMap<Date, Decimal>, UpdateError> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| UpdateError::Parse {
        provider: "coingecko",
        detail: e.to_string(),
    })?;
    let prices = v
        .get("prices")
        .and_then(|p| p.as_array())
        .ok_or(UpdateError::Parse {
            provider: "coingecko",
            detail: "missing `prices` array".into(),
        })?;
    let mut by_date: BTreeMap<Date, (i64, Decimal)> = BTreeMap::new();
    for point in prices {
        let p = point.as_array().ok_or(UpdateError::Parse {
            provider: "coingecko",
            detail: "price point is not an array".into(),
        })?;
        let ts_ms =
            p.first()
                .and_then(|x| x.as_f64())
                .map(|f| f as i64)
                .ok_or(UpdateError::Parse {
                    provider: "coingecko",
                    detail: "price[0] (timestamp) missing".into(),
                })?;
        let price_f = p
            .get(1)
            .and_then(|x| x.as_f64())
            .ok_or(UpdateError::Parse {
                provider: "coingecko",
                detail: "price[1] (value) missing".into(),
            })?;
        let price = Decimal::from_f64(price_f).ok_or(UpdateError::Parse {
            provider: "coingecko",
            detail: format!("price {price_f} not representable"),
        })?;
        let date = date_from_unix_ms(ts_ms)?;
        // Keep the latest ts within each UTC day.
        by_date
            .entry(date)
            .and_modify(|slot| {
                if ts_ms > slot.0 {
                    *slot = (ts_ms, round_money(price));
                }
            })
            .or_insert((ts_ms, round_money(price)));
    }
    Ok(by_date.into_iter().map(|(d, (_, p))| (d, p)).collect())
}

// ── Cache append (idempotent, forward-only; the KATs target this) ─────────────────────────────────

/// The outcome of an append: how many rows were written vs skipped (already present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendSummary {
    pub cache_path: PathBuf,
    pub appended: usize,
    /// Fetched dates already present in the cache OR the bundled dataset (idempotency / never-touch-bundled).
    pub skipped_present: usize,
    pub dry_run: bool,
}

fn read_cache_or_empty(path: &Path) -> Result<BundledPrices, UpdateError> {
    if path.exists() {
        let csv = std::fs::read_to_string(path).map_err(|source| UpdateError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(BundledPrices::from_csv_str(&csv)?)
    } else {
        Ok(BundledPrices::from_csv_str("date,usd_close\n")?)
    }
}

/// Append `fetched` closes to the cache at `cache_path`, SKIPPING any date already in the cache OR the
/// bundled dataset (idempotent; never re-covers bundled). Forward-only (fetched dates are beyond the
/// last known), so the appended rows keep the file sorted. `dry_run` writes NOTHING. Creates the parent
/// dir + a `date,usd_close` header on first write.
pub fn append_to_cache(
    cache_path: &Path,
    fetched: &BTreeMap<Date, Decimal>,
    bundled: &BundledPrices,
    dry_run: bool,
) -> Result<AppendSummary, UpdateError> {
    let existing = read_cache_or_empty(cache_path)?;
    let to_append: Vec<(Date, Decimal)> = fetched
        .iter()
        .filter(|(d, _)| !existing.contains(**d) && !bundled.contains(**d))
        .map(|(d, p)| (*d, *p))
        .collect();
    let skipped_present = fetched.len() - to_append.len();

    if !dry_run && !to_append.is_empty() {
        if let Some(parent) = cache_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| UpdateError::Io {
                    path: parent.display().to_string(),
                    source,
                })?;
            }
        }
        let file_exists = cache_path.exists();
        let mut body = String::new();
        if !file_exists {
            body.push_str("date,usd_close\n");
        }
        for (d, p) in &to_append {
            // `Date` Displays as ISO `YYYY-MM-DD`; `Decimal` keeps the 2dp scale from `round_money`.
            body.push_str(&format!("{d},{p}\n"));
        }
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(cache_path)
            .map_err(|source| UpdateError::Io {
                path: cache_path.display().to_string(),
                source,
            })?;
        f.write_all(body.as_bytes())
            .map_err(|source| UpdateError::Io {
                path: cache_path.display().to_string(),
                source,
            })?;
    }

    Ok(AppendSummary {
        cache_path: cache_path.to_path_buf(),
        appended: to_append.len(),
        skipped_present,
        dry_run,
    })
}

// ── Network (the ONLY non-hermetic code; not unit-tested — see the #[ignore] live smoke) ──────────

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
}

fn http_get(agent: &ureq::Agent, url: &str, provider: &'static str) -> Result<String, UpdateError> {
    let resp = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| UpdateError::Network {
            provider,
            detail: e.to_string(),
        })?;
    resp.into_string().map_err(|e| UpdateError::Network {
        provider,
        detail: e.to_string(),
    })
}

fn day_start_ms(d: Date) -> i64 {
    d.midnight().assume_utc().unix_timestamp() * 1000
}
fn day_end_ms(d: Date) -> i64 {
    (d.midnight().assume_utc() + Duration::from_secs(86_399)).unix_timestamp() * 1000
}

fn binance_url(start: Date, end: Date) -> String {
    format!(
        "https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1d&startTime={}&endTime={}&limit=1000",
        day_start_ms(start),
        day_end_ms(end)
    )
}
fn coingecko_url(start: Date, end: Date) -> String {
    format!(
        "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart/range?vs_currency=usd&from={}&to={}",
        day_start_ms(start) / 1000,
        day_end_ms(end) / 1000
    )
}

/// LIVE fetch from Binance (network).
pub fn fetch_binance(start: Date, end: Date) -> Result<BTreeMap<Date, Decimal>, UpdateError> {
    parse_binance_klines(&http_get(&agent(), &binance_url(start, end), "binance")?)
}
/// LIVE fetch from CoinGecko (network).
pub fn fetch_coingecko(start: Date, end: Date) -> Result<BTreeMap<Date, Decimal>, UpdateError> {
    parse_coingecko_range(&http_get(
        &agent(),
        &coingecko_url(start, end),
        "coingecko",
    )?)
}

/// Fetch per `source`, with `auto` = Binance then CoinGecko fallback.
pub fn fetch(
    source: SourceArg,
    start: Date,
    end: Date,
) -> Result<BTreeMap<Date, Decimal>, UpdateError> {
    match source {
        SourceArg::Binance => fetch_binance(start, end),
        SourceArg::Coingecko => fetch_coingecko(start, end),
        SourceArg::Auto => match fetch_binance(start, end) {
            Ok(m) => Ok(m),
            Err(binance) => {
                fetch_coingecko(start, end).map_err(|coingecko| UpdateError::AllSourcesFailed {
                    binance: binance.to_string(),
                    coingecko: coingecko.to_string(),
                })
            }
        },
    }
}

// ── Orchestration ────────────────────────────────────────────────────────────────────────────────

/// What a run did.
#[derive(Debug)]
pub enum RunOutcome {
    /// Already up to date within the settling window — nothing fetched.
    UpToDate,
    /// Fetched `[start, end]`; the append summary reports written vs skipped.
    Updated {
        start: Date,
        end: Date,
        summary: AppendSummary,
    },
}

/// Resolve the range, fetch, and append (honoring `--dry-run`). `today` is injected for determinism.
pub fn run(cli: &Cli, today: Date) -> Result<RunOutcome, UpdateError> {
    let cache_path = cli
        .price_cache
        .clone()
        .or_else(default_cache_path)
        .ok_or(UpdateError::NoCachePath)?;
    let bundled = BundledPrices::load()?;
    let existing = read_cache_or_empty(&cache_path)?;
    let last_known = [existing.max_date(), bundled.max_date()]
        .into_iter()
        .flatten()
        .max();

    let Some((start, end)) = fetch_range(last_known, today, cli.lag, cli.from, cli.to) else {
        return Ok(RunOutcome::UpToDate);
    };
    let fetched = fetch(cli.source, start, end)?;
    let summary = append_to_cache(&cache_path, &fetched, &bundled, cli.dry_run)?;
    Ok(RunOutcome::Updated {
        start,
        end,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    /// Milliseconds at 00:00 UTC of `d` (mirrors the private URL helper — used to build canned JSON).
    fn day_ms(d: Date) -> i64 {
        d.midnight().assume_utc().unix_timestamp() * 1000
    }
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// A tiny synthetic bundled set (a stand-in for the shipped dataset) so the append KATs can prove
    /// "never re-cover a bundled date" without depending on the 5,801-row file.
    fn synth_bundled() -> BundledPrices {
        BundledPrices::from_csv_str("date,usd_close\n2020-01-01,9000.00\n").unwrap()
    }

    #[test]
    fn parse_binance_klines_maps_close_by_utc_day() {
        let d1 = date!(2026 - 06 - 04);
        let d2 = date!(2026 - 06 - 05);
        // [openMs, o,h,l,close,vol, closeMs, …] — index 4 is the close; rounded to 2dp on parse.
        let json = format!(
            "[[{},\"1\",\"2\",\"0\",\"64000.005\",\"5\",{},\"0\",0,\"0\",\"0\",\"0\"],\
              [{},\"1\",\"2\",\"0\",\"63500.5\",\"5\",{},\"0\",0,\"0\",\"0\",\"0\"]]",
            day_ms(d1),
            day_ms(d1) + 1,
            day_ms(d2),
            day_ms(d2) + 1
        );
        let m = parse_binance_klines(&json).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[&d1], dec("64000.00")); // banker's rounding: 64000.005 → 64000.00
        assert_eq!(m[&d2], dec("63500.50"));
    }

    #[test]
    fn parse_coingecko_range_keeps_last_per_day() {
        let d1 = date!(2026 - 06 - 04);
        let d2 = date!(2026 - 06 - 05);
        // Two points on d1 (the LATER wins) + one on d2. Prices are JSON floats.
        let json = format!(
            "{{\"prices\":[[{},60000.0],[{},61000.0],[{},62000.0]]}}",
            day_ms(d1),
            day_ms(d1) + 3_600_000, // one hour later, same UTC day → wins
            day_ms(d2)
        );
        let m = parse_coingecko_range(&json).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[&d1], dec("61000.00"), "the later intraday point wins");
        assert_eq!(m[&d2], dec("62000.00"));
    }

    #[test]
    fn update_prices_dry_run_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("price_cache.csv");
        let mut fetched = BTreeMap::new();
        fetched.insert(date!(2026 - 06 - 04), dec("64000.00"));

        let summary = append_to_cache(&cache, &fetched, &synth_bundled(), true).unwrap();
        assert!(summary.dry_run);
        assert_eq!(summary.appended, 1, "reports what it WOULD append");
        assert!(
            !cache.exists(),
            "a dry run must not create or write the cache file"
        );
    }

    #[test]
    fn update_prices_appends_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("price_cache.csv");
        let bundled = synth_bundled();
        let mut fetched = BTreeMap::new();
        fetched.insert(date!(2020 - 01 - 01), dec("9999.00")); // already in bundled → must be skipped
        fetched.insert(date!(2026 - 06 - 04), dec("64000.00"));
        fetched.insert(date!(2026 - 06 - 05), dec("63500.50"));

        // First append: the two new dates land; the bundled date is skipped (never re-covered).
        let s1 = append_to_cache(&cache, &fetched, &bundled, false).unwrap();
        assert_eq!(s1.appended, 2);
        assert_eq!(s1.skipped_present, 1, "the bundled 2020-01-01 is skipped");
        let body = std::fs::read_to_string(&cache).unwrap();
        assert!(body.starts_with("date,usd_close\n"), "header written once");
        assert!(body.contains("2026-06-04,64000.00"));
        assert!(body.contains("2026-06-05,63500.50"));
        assert!(
            !body.contains("2020-01-01"),
            "bundled date never written to cache"
        );

        // Second append of the SAME fetch: everything is now present → 0 appended (idempotent).
        let s2 = append_to_cache(&cache, &fetched, &bundled, false).unwrap();
        assert_eq!(s2.appended, 0, "re-run appends nothing");
        assert_eq!(s2.skipped_present, 3);
        assert_eq!(
            std::fs::read_to_string(&cache).unwrap(),
            body,
            "an idempotent re-run leaves the file byte-identical"
        );

        // The cache is a valid provider input (parses back).
        let reparsed =
            BundledPrices::from_csv_str(&std::fs::read_to_string(&cache).unwrap()).unwrap();
        assert!(
            reparsed.contains(date!(2026 - 06 - 04)),
            "the cache reparses as a valid provider input"
        );
    }

    #[test]
    fn update_prices_respects_lag() {
        // No --from/--to: start = day after last_known; end = today − lag (the settling window).
        let last_known = date!(2026 - 06 - 03);
        let today = date!(2026 - 06 - 20);
        let (start, end) = fetch_range(Some(last_known), today, 8, None, None).unwrap();
        assert_eq!(
            start,
            date!(2026 - 06 - 04),
            "start = day after the last known close"
        );
        assert_eq!(end, date!(2026 - 06 - 12), "end = today − 8 days (lag)");

        // A smaller lag reaches closer to today.
        let (_s, end0) = fetch_range(Some(last_known), today, 0, None, None).unwrap();
        assert_eq!(end0, today, "lag 0 → end = today");

        // Already caught up (last_known within the settling window) → nothing to fetch.
        assert!(fetch_range(Some(date!(2026 - 06 - 18)), today, 8, None, None).is_none());
    }

    /// Live network smoke test — IGNORED by default (the hermetic KATs above use canned JSON).
    #[test]
    #[ignore = "hits the live Binance/CoinGecko network"]
    fn live_fetch_smoke() {
        let end = date!(2025 - 01 - 10);
        let start = date!(2025 - 01 - 08);
        let m = fetch(SourceArg::Auto, start, end).expect("a live source should return closes");
        assert!(!m.is_empty(), "expected at least one live daily close");
    }
}
