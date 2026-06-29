//! Exact money/time conventions (NFR5, §6.1). No floats anywhere.
use rust_decimal::{Decimal, RoundingStrategy};
use time::{Date, OffsetDateTime, UtcOffset};

/// Bitcoin is integer satoshis (NFR5/§6.1). Signed so shortfall/overshoot math is total; quantities are non-negative.
pub type Sat = i64;
/// USD is exact decimal (NFR5).
pub type Usd = Decimal;
/// Tax dates are calendar dates in `original_tz`, day granularity (§6.1).
pub type TaxDate = Date;

/// `ROUND_HALF_EVEN` (§6.1).
pub const MONEY_ROUNDING: RoundingStrategy = RoundingStrategy::MidpointNearestEven;
/// Satoshis per whole BTC.
pub const SATS_PER_BTC: i64 = 100_000_000;
/// The per-wallet basis snapshot date (§7.4).
pub const TRANSITION_DATE: TaxDate = time::macros::date!(2025 - 01 - 01);
/// App-observable TY2025 unextended return due date (§7.4); the extended date is not app-observable.
pub const TY2025_RETURN_DUE: TaxDate = time::macros::date!(2026 - 04 - 15);

/// Round a USD value to the cent, ties-to-even.
pub fn round_cents(v: Usd) -> Usd {
    v.round_dp_with_strategy(2, MONEY_ROUNDING)
}

/// Split `total` so the `part_sat`/`whole_sat` portion is rounded to cents (ties-to-even) and the
/// remainder is `total - part`, conserving the sum EXACTLY (Σbasis invariant, §13/§6.3).
/// `whole_sat` is assumed > 0 by callers (consumption guards remaining_sat > 0).
///
/// §7.1 totality (M6): uses **checked** Decimal ops so it can never panic on overflow. The primary
/// `total * part / whole` form holds for all in-range money (USD magnitudes within Decimal's 96-bit
/// mantissa; `Sat` ≤ 21e6·1e8 = 2.1e15); on the (practically unreachable) overflow it falls back to the
/// magnitude-safe divide-first form. Both forms round to cents and conserve via remainder-takes-the-rest.
pub fn split_pro_rata(total: Usd, part_sat: Sat, whole_sat: Sat) -> (Usd, Usd) {
    if whole_sat <= 0 || part_sat <= 0 {
        return (Usd::ZERO, total);
    }
    if part_sat >= whole_sat {
        return (total, Usd::ZERO);
    }
    let (p, w) = (Usd::from(part_sat), Usd::from(whole_sat));
    let part = total
        .checked_mul(p)
        .and_then(|x| x.checked_div(w))
        .or_else(|| total.checked_div(w).and_then(|x| x.checked_mul(p)))
        .map(round_cents)
        .unwrap_or(Usd::ZERO); // unreachable for in-range money; never panics
    (part, total - part)
}

/// Calendar date in `original_tz` (§6.1).
pub fn tax_date(utc: OffsetDateTime, tz: UtcOffset) -> TaxDate {
    utc.to_offset(tz).date()
}

/// One calendar year after `d`; a Feb-29 anniversary in a non-leap year falls back to Feb 28 (documented convention).
pub fn one_year_after(d: TaxDate) -> TaxDate {
    let y = d.year() + 1;
    Date::from_calendar_date(y, d.month(), d.day()).unwrap_or_else(|_| {
        Date::from_calendar_date(y, d.month(), 28).expect("Feb 28 is always valid")
    })
}

/// TP4: long-term iff the disposition date is strictly more than one year after acquisition.
pub fn is_long_term(acquired: TaxDate, disposed: TaxDate) -> bool {
    disposed > one_year_after(acquired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::{date, datetime, offset};

    #[test]
    fn rounds_half_even_to_cents() {
        assert_eq!(round_cents(dec!(1.005)), dec!(1.00)); // ties-to-even: 0 is even
        assert_eq!(round_cents(dec!(1.015)), dec!(1.02)); // ties-to-even: 2 is even
        assert_eq!(round_cents(dec!(2.675)), dec!(2.68));
    }

    #[test]
    fn pro_rata_split_conserves_exactly() {
        // split 100.00 across takes that don't divide evenly: parts must sum to the whole.
        let (part, rest) = split_pro_rata(dec!(100.00), 333, 1000);
        assert_eq!(part + rest, dec!(100.00));
        assert_eq!(part, dec!(33.30)); // 100 * 333/1000 = 33.3 -> 33.30
    }

    #[test]
    fn tax_date_uses_original_tz_calendar_date() {
        // 2025-01-01T01:30:00Z is still 2024-12-31 in UTC-05:00 (day-granularity boundary, §6.1).
        let utc = datetime!(2025-01-01 01:30:00 UTC);
        assert_eq!(tax_date(utc, offset!(-05:00)), date!(2024 - 12 - 31));
        assert_eq!(tax_date(utc, offset!(+00:00)), date!(2025 - 01 - 01));
    }

    #[test]
    fn holding_period_boundary_tp4() {
        // Pub 544 example: acquire 2020-06-19; sell 2021-06-19 = ST (exactly 1yr); 2021-06-20 = LT.
        let acq = date!(2020 - 06 - 19);
        assert!(!is_long_term(acq, date!(2021 - 06 - 19)));
        assert!(is_long_term(acq, date!(2021 - 06 - 20)));
        assert!(!is_long_term(acq, acq)); // same-day = ST
    }

    #[test]
    fn leap_day_anniversary_falls_back_to_feb_28() {
        assert_eq!(one_year_after(date!(2020 - 02 - 29)), date!(2021 - 02 - 28));
    }
}
