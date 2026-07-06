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

    /// The latest date present, or `None` when empty. Used by `btctax-update-prices` (Part C) to compute
    /// the fetch start (the day after the last known close) — a pure read, no network.
    pub fn max_date(&self) -> Option<TaxDate> {
        self.by_date.keys().next_back().copied()
    }

    /// True when `date` already has a close (idempotency guard for the updater's append).
    pub fn contains(&self, date: TaxDate) -> bool {
        self.by_date.contains_key(&date)
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

/// The bundled daily-close dataset with a LOCAL price cache layered OVER it (#41 Part C). The cache is a
/// `date,usd_close` CSV the separate `btctax-update-prices` binary appends newer/gap closes into — a
/// documented LOCAL INPUT (like the vault). `usd_per_btc` is CACHE-over-bundled; both sources are local
/// so the projection stays pure/deterministic (NFR4). A cache ABSENT (or `cache_path == None`) is
/// byte-identical to bundled-only. This crate carries NO `dirs` and NO network — the caller resolves the
/// path (btctax-cli via `dirs`), and the online refresh lives ONLY in `btctax-update-prices`.
#[derive(Debug, Clone)]
pub struct LayeredPrices {
    bundled: BundledPrices,
    /// The cache rows (empty ⇒ bundled-only). Same `date,usd_close` format as the bundled dataset.
    cache: BundledPrices,
}

impl LayeredPrices {
    /// Load the compiled-in dataset, layering the cache CSV at `cache_path` over it. `None` or a
    /// non-existent file ⇒ bundled-only (byte-identical). A PRESENT-but-malformed cache is a LOUD error
    /// (a corrupt local input must not silently alter prices). NO network; pure.
    pub fn load_with_cache(cache_path: Option<&std::path::Path>) -> Result<Self, AdapterError> {
        let bundled = BundledPrices::load()?;
        let cache = match cache_path {
            Some(p) if p.exists() => {
                let csv = std::fs::read_to_string(p).map_err(|source| AdapterError::Io {
                    path: p.display().to_string(),
                    source,
                })?;
                BundledPrices::from_csv_str(&csv)?
            }
            // None, or a not-yet-created cache → an empty overlay (bundled-only).
            _ => BundledPrices {
                by_date: BTreeMap::new(),
            },
        };
        Ok(Self { bundled, cache })
    }
}

impl PriceProvider for LayeredPrices {
    fn usd_per_btc(&self, date: TaxDate) -> Option<Usd> {
        // Cache-over-bundled: a cached close (a newer/gap-filled day) wins; else the shipped dataset.
        self.cache
            .usd_per_btc(date)
            .or_else(|| self.bundled.usd_per_btc(date))
    }
}

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
        assert_eq!(
            fmv_of(&p, date!(2025 - 03 - 01), 50_000_000),
            Some(dec!(42000.00))
        );
    }

    #[test]
    fn parses_exact_decimals_not_floats() {
        let p = BundledPrices::from_csv_str("date,usd_close\n2024-02-01,43100.50\n").unwrap();
        assert_eq!(p.usd_per_btc(date!(2024 - 02 - 01)), Some(dec!(43100.50)));
    }

    // ── #41 A-KATs: the SHIPPED bundled daily-close dataset (5,801 rows, 2010-07-17 → 2026-06-03) ──

    /// Exact row count — a truncation / accidental-regeneration guard.
    #[test]
    fn bundled_dataset_row_count() {
        let p = BundledPrices::load().unwrap();
        assert_eq!(
            p.by_date.len(),
            5801,
            "bundled daily-close row count changed — regenerate intentionally + update this pin"
        );
    }

    /// Every non-header CSV line is a UNIQUE, strictly-ascending date: len == raw line count proves no
    /// row was collapsed by dedup (already deduped), and the ascending window proves sorted + unique.
    #[test]
    fn bundled_dataset_parses_sorted_deduped() {
        let raw_lines = DATASET_CSV
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("date"))
            .count();
        let p = BundledPrices::load().unwrap();
        assert_eq!(
            p.by_date.len(),
            raw_lines,
            "a duplicate date collapsed on load — the source dataset is not deduped"
        );
        let dates: Vec<_> = p.by_date.keys().copied().collect();
        assert!(
            dates.windows(2).all(|w| w[0] < w[1]),
            "dates must be strictly ascending (sorted, no duplicates)"
        );
    }

    /// Coverage spot-check across the whole span + out-of-range → None (no gap-fill / extrapolation).
    #[test]
    fn bundled_dataset_covers_2010_to_2026() {
        let p = BundledPrices::load().unwrap();
        assert_eq!(
            p.usd_per_btc(date!(2010 - 07 - 17)),
            Some(dec!(0.05)),
            "first close (2010-07-17)"
        );
        assert_eq!(
            p.usd_per_btc(date!(2025 - 06 - 15)),
            Some(dec!(105651.98)),
            "mid-range spot (2025-06-15)"
        );
        assert_eq!(
            p.usd_per_btc(date!(2026 - 06 - 03)),
            Some(dec!(64813.38)),
            "last close (2026-06-03)"
        );
        assert_eq!(
            p.usd_per_btc(date!(2009 - 01 - 03)),
            None,
            "before coverage → None"
        );
        assert_eq!(
            p.usd_per_btc(date!(2030 - 01 - 01)),
            None,
            "after coverage → None"
        );
    }

    /// [M4] Pin ONE real date's FMV end-to-end through `fmv_of` (the exact 2dp cents source is fine —
    /// `fmv_of` already `round_cents`): 2025-06-15 close 105651.98/BTC.
    #[test]
    fn real_date_fmv_is_exact() {
        let p = BundledPrices::load().unwrap();
        // 1 BTC = 100_000_000 sat → the whole-BTC close.
        assert_eq!(
            fmv_of(&p, date!(2025 - 06 - 15), 100_000_000),
            Some(dec!(105651.98))
        );
        // 0.5 BTC = 50_000_000 sat → round_cents(52825.99).
        assert_eq!(
            fmv_of(&p, date!(2025 - 06 - 15), 50_000_000),
            Some(dec!(52825.99))
        );
    }

    // ── #41 Part C: LayeredPrices (cache-over-bundled; no network) ──────────────────────────────────

    /// The cache OVERRIDES the bundled close for a shared date AND supplies a NEW date beyond the bundled
    /// range; bundled-only dates still resolve from the shipped dataset.
    #[test]
    fn layered_prices_cache_over_bundled() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("price_cache.csv");
        // 2026-06-03 exists in the bundled set (64813.38) — override it; 2026-07-01 is NEW (beyond it).
        std::fs::write(
            &cache,
            "date,usd_close\n2026-06-03,70000.00\n2026-07-01,71234.56\n",
        )
        .unwrap();

        let p = LayeredPrices::load_with_cache(Some(cache.as_path())).unwrap();
        assert_eq!(
            p.usd_per_btc(date!(2026 - 06 - 03)),
            Some(dec!(70000.00)),
            "the cache overrides the bundled close for a shared date"
        );
        assert_eq!(
            p.usd_per_btc(date!(2026 - 07 - 01)),
            Some(dec!(71234.56)),
            "the cache supplies a date beyond the bundled range"
        );
        assert_eq!(
            p.usd_per_btc(date!(2025 - 06 - 15)),
            Some(dec!(105651.98)),
            "a bundled-only date still resolves from the shipped dataset"
        );
    }

    /// A cache ABSENT (path missing) or `None` is byte-identical to bundled-only.
    #[test]
    fn cache_absent_is_bundled_only() {
        let bundled = BundledPrices::load().unwrap();
        for cache_path in [
            None,
            Some(std::path::Path::new("/nonexistent/price_cache.csv")),
        ] {
            let layered = LayeredPrices::load_with_cache(cache_path).unwrap();
            for d in [
                date!(2010 - 07 - 17),
                date!(2025 - 06 - 15),
                date!(2026 - 06 - 03),
                date!(2030 - 01 - 01), // uncovered → None on both
            ] {
                assert_eq!(
                    layered.usd_per_btc(d),
                    bundled.usd_per_btc(d),
                    "layered (no cache) must equal bundled-only at {d}"
                );
            }
        }
    }
}
