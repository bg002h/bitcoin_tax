//! Exact money/time conventions (NFR5, §6.1). No floats anywhere.
use rust_decimal::{Decimal, RoundingStrategy};
use time::{Date, Month, OffsetDateTime, UtcOffset};

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

/// `ROUND_HALF_UP` (away-from-zero) to whole dollars — the IRS Form-1040 rounding convention
/// ("drop under 50¢, round 50–99¢ up; **$2.50 becomes $3**", 2024 i1040 p.23).
pub const DOLLAR_ROUNDING: RoundingStrategy = RoundingStrategy::MidpointAwayFromZero;

/// Round a USD value to whole dollars, ties **away from zero** (IRS half-up).
///
/// DELIBERATELY DISTINCT from [`round_cents`] (ties-to-even): the full-return absolute-liability path
/// (`tax/method.rs`) and every filed 1040/schedule form-line use this; the crypto-**delta** path keeps
/// `round_cents`. Reusing the ties-to-even cent path mis-prints real IRS Tax-Table cells — a $50 bin
/// whose midpoint tax ends in `.50` prints the away-from-zero value (e.g. MFJ [11,600,11,650) → $1,163,
/// not $1,162). See SPEC_full_return §3.1 / recon deep/01 (spec §8 assertion).
pub fn round_dollar(v: Usd) -> Usd {
    v.round_dp_with_strategy(0, DOLLAR_ROUNDING)
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

/// The DEFAULT acquisition date for an inbound self-transfer when the taxpayer supplies no `--acquired`:
/// **one calendar year + one day before the receipt `date`**, which GUARANTEES long-term treatment for
/// ANY disposal on or after receipt. Proof: `one_year_after(acquired) ≤ acquired + 366d` and this returns
/// at most `date − 1d`, so `one_year_after(acquired) < date ≤ disposed` ⇒ `is_long_term` (conventions
/// §65). Leap-safe: `replace_year(y−1)` falls back Feb-29 → Feb-28 (the prior year has no Feb-29), then
/// one calendar day is subtracted. Deliberately NOT `date − Duration::days(366)`, which FAILS across a leap
/// boundary (a 366-day span lands `one_year_after` exactly ON `date`, so a same-day sale is NOT long-term).
/// Saturating on the (unreachable for BTC dates ≥ 2009) day-underflow.
pub fn long_term_default_acquired(date: TaxDate) -> TaxDate {
    let prior_year = date.replace_year(date.year() - 1).unwrap_or_else(|_| {
        // Feb-29 receipt: the prior (non-leap) year has no Feb-29 — anchor to Feb-28.
        Date::from_calendar_date(date.year() - 1, Month::February, 28)
            .expect("Feb 28 is always valid")
    });
    prior_year.previous_day().unwrap_or(prior_year) // −1 calendar day; saturating (no real underflow)
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
        assert_eq!(round_cents(dec!(1.234)), dec!(1.23)); // non-tie case
    }

    /// `round_dollar` is IRS half-up (away-from-zero) to whole dollars, and is DISTINCT from the
    /// engine's half-even cents rounding on the real IRS Tax-Table discriminating cells (deep/01):
    /// a $50 bin whose midpoint tax ends in `.50` must print the away-from-zero value.
    /// (SPEC_full_return §3.1 / P0 task 1 KAT.)
    #[test]
    fn round_dollar_is_half_up_and_differs_from_half_even() {
        // IRS convention: "$2.50 becomes $3" (2024 i1040 p.23); "$1.39 becomes $1".
        assert_eq!(round_dollar(dec!(2.50)), dec!(3));
        assert_eq!(round_dollar(dec!(1.39)), dec!(1));
        // Discriminating printed Tax-Table cells: MFJ [11,600,11,650) → 1,163; Single [3,000,3,050) → 303.
        assert_eq!(round_dollar(dec!(1162.50)), dec!(1163));
        assert_eq!(round_dollar(dec!(302.50)), dec!(303));
        // Fault-inject: the frozen half-even mode gives the WRONG table value (1162 / 302) — the very
        // reason round_dollar must exist and must not reuse the crypto-delta path's rounding.
        assert_eq!(dec!(1162.50).round_dp_with_strategy(0, MONEY_ROUNDING), dec!(1162));
        assert_eq!(dec!(302.50).round_dp_with_strategy(0, MONEY_ROUNDING), dec!(302));
        // Away-from-zero is symmetric on negatives (loss-line magnitudes are handled by sign policy,
        // but the rounding itself must be symmetric): -2.50 → -3.
        assert_eq!(round_dollar(dec!(-2.50)), dec!(-3));
    }

    #[test]
    fn pro_rata_split_conserves_exactly() {
        // split 100.00 across takes that don't divide evenly: parts must sum to the whole.
        let (part, rest) = split_pro_rata(dec!(100.00), 333, 1000);
        assert_eq!(part + rest, dec!(100.00));
        assert_eq!(part, dec!(33.30)); // 100 * 333/1000 = 33.3 -> 33.30
    }

    #[test]
    fn split_pro_rata_edges() {
        // Edge case: part_sat == 0 → (ZERO, total), conserves exactly
        let (part, rest) = split_pro_rata(dec!(10.00), 0, 1000);
        assert_eq!(part, Usd::ZERO);
        assert_eq!(rest, dec!(10.00));
        assert_eq!(part + rest, dec!(10.00));

        // Edge case: part_sat == whole_sat → (total, ZERO), conserves exactly
        let (part, rest) = split_pro_rata(dec!(10.00), 500, 500);
        assert_eq!(part, dec!(10.00));
        assert_eq!(rest, Usd::ZERO);
        assert_eq!(part + rest, dec!(10.00));

        // Edge case: part_sat > whole_sat → (total, ZERO), conserves exactly
        let (part, rest) = split_pro_rata(dec!(10.00), 600, 500);
        assert_eq!(part, dec!(10.00));
        assert_eq!(rest, Usd::ZERO);
        assert_eq!(part + rest, dec!(10.00));

        // Edge case: whole_sat == 0 → (ZERO, total), conserves exactly
        let (part, rest) = split_pro_rata(dec!(10.00), 100, 0);
        assert_eq!(part, Usd::ZERO);
        assert_eq!(rest, dec!(10.00));
        assert_eq!(part + rest, dec!(10.00));
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
