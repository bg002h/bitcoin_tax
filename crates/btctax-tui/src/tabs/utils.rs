//! Shared pure utility helpers for the viewer tabs.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use rust_decimal::Decimal;

/// Minimum content-area height (in rows, border-INCLUSIVE) at which a tabular output tab
/// renders its frozen column-totals footer. Below this, the vertical space is given to data
/// rows instead (the footer is dropped).
///
/// The gate measures the `area: Rect` passed to each tab's `render` (`chunks[1]` / `Min(0)`,
/// border-inclusive). At 10 the usable inner height is 8 = header(1) + footer(1) + 6 data rows,
/// comfortably more than one data row. On a standard ≥24-row terminal the content pane is ~20
/// rows, so the footer always shows; only a very short terminal drops it.
pub(crate) const MIN_ROWS_FOR_TOTALS: u16 = 10;

/// Convert a satoshi count to its exact BTC representation as a [`Decimal`].
///
/// 100_000_000 sat = 1.00000000 BTC.
///
/// **Exact integer arithmetic — no float ([R0-M5]/NFR5).**
/// `Decimal::from(sat) / Decimal::from(100_000_000i64)` is lossless at 8 dp.
pub fn sat_to_btc(sat: i64) -> Decimal {
    Decimal::from(sat) / Decimal::from(100_000_000i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn sat_to_btc_one_whole_coin() {
        assert_eq!(sat_to_btc(100_000_000), Decimal::from(1));
    }

    #[test]
    fn sat_to_btc_one_satoshi() {
        // 0.00000001 BTC — must not lose the last digit.
        let result = sat_to_btc(1);
        let expected: Decimal = "0.00000001".parse().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn sat_to_btc_zero() {
        assert_eq!(sat_to_btc(0), Decimal::ZERO);
    }
}
