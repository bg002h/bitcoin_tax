//! Shared pure utility helpers for the viewer tabs.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use rust_decimal::Decimal;

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
