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
}
