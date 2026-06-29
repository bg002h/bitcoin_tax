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
///
/// NOTE: `Decimal::from_scientific` does not exist in rust_decimal 1.x; `Decimal::from_str`
/// already parses scientific notation in rust_decimal 1.x, so no fallback is needed.
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
    let mut d = Decimal::from_str(&cleaned).map_err(|e| AdapterError::Parse {
        adapter: source,
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
        adapter: source,
        line,
        field,
        value: raw.to_string(),
        reason: e.to_string(),
    })?;
    let sats = btc * Decimal::from(SATS_PER_BTC);
    if !sats.fract().is_zero() {
        return Err(AdapterError::FractionalSat {
            adapter: source,
            line,
            value: raw.to_string(),
        });
    }
    sats.trunc().to_i64().ok_or_else(|| AdapterError::Parse {
        adapter: source,
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
        adapter: source,
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
        assert_eq!(
            parse_btc_to_sat("t", 1, "f", "0.12345678 BTC").unwrap(),
            12_345_678
        );
        assert_eq!(parse_btc_to_sat("t", 1, "f", "-0.5").unwrap(), -50_000_000);
        // signed; callers .abs()
    }

    #[test]
    fn fractional_satoshi_is_an_error_never_a_silent_round() {
        let e = parse_btc_to_sat("river", 7, "amount", "0.000000001").unwrap_err();
        assert!(matches!(e, AdapterError::FractionalSat { line: 7, .. })); // adapter + value are wildcards
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
        assert_eq!(
            (utc, tz),
            (datetime!(2025-03-01 12:00:00 UTC), offset!(+00:00))
        );
        // Swan transfers/withdrawals: `YYYY-MM-DD HH:MM:SS+00` (space sep, short offset).
        let (utc, tz) = parse_timestamp("swan", 1, "2025-03-02 09:00:00+00").unwrap();
        assert_eq!(
            (utc, tz),
            (datetime!(2025-03-02 09:00:00 UTC), offset!(+00:00))
        );
        // Swan trades: US-locale MM/DD/YYYY, assumed UTC.
        let (utc, tz) = parse_timestamp("swan", 1, "03/01/2025 12:00:00").unwrap();
        assert_eq!(
            (utc, tz),
            (datetime!(2025-03-01 12:00:00 UTC), offset!(+00:00))
        );
    }

    #[test]
    fn excel_serial_and_flex_parse() {
        // Anchor: serial 25569 = the Unix epoch; the fraction is the time of day.
        assert_eq!(
            excel_serial_to_utc(25569.0),
            datetime!(1970-01-01 00:00:00 UTC)
        );
        assert_eq!(
            excel_serial_to_utc(25569.5),
            datetime!(1970-01-01 12:00:00 UTC)
        );
        // Gemini stores Date/Time as numeric serials; flex parse converts them at UTC.
        let (utc, tz) = parse_timestamp_flex("gemini", 1, "25569.5").unwrap();
        assert_eq!(
            (utc, tz),
            (datetime!(1970-01-01 12:00:00 UTC), offset!(+00:00))
        );
        // flex still handles ISO text (used by the synthetic Gemini fixtures).
        let (utc, _) = parse_timestamp_flex("gemini", 1, "2025-03-01 12:00:00").unwrap();
        assert_eq!(utc, datetime!(2025-03-01 12:00:00 UTC));
    }
}
