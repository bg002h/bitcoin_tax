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
}
