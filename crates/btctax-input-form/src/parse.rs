//! Task 8: the field-parse tier (spec §5.7) — turns a raw user string into a validated `FieldValue` (or a
//! `ParseError`), so a bad SSN / negative money / malformed date is rejected at the door, before it ever
//! reaches `apply`'s `SetField` (a renderer builds the `Edit` from `parse`'s `Ok`, never from the raw text).
//!
//! **Secret is the one kind `parse` cannot dispatch alone.** Both an SSN and an IP PIN are
//! `FieldKind::Secret` — the seam's `FieldKind` carries no sub-kind, and adding one is a seam change out of
//! this task's scope (frozen: `FieldKind`/`FieldValue`/`ParseError` are consumed, not modified). So the real
//! API is two dedicated entry points, [`parse_ssn`] and [`parse_ip_pin`]; the caller (a renderer) already
//! knows which by the field's `FieldId` (`TpSsn`/`SpSsn`/`DepSsn` vs `IpPin`) and picks the matching one.
//! `parse(FieldKind::Secret, _)` is a defensive fallback for a caller that dispatches generically anyway —
//! see its match arm for why it refuses rather than guesses.

use crate::seam::{FieldKind, FieldValue, ParseError};
use btctax_core::tax::packet::{IpPin, Ssn};
use rust_decimal::Decimal;
use std::str::FromStr;

/// Parse a raw user string into a validated `FieldValue` for the given `kind` (spec §5.7).
///
/// `Secret` is handled by [`parse_ssn`]/[`parse_ip_pin`] — see the module docs for why.
pub fn parse(kind: FieldKind, raw: &str) -> Result<FieldValue, ParseError> {
    match kind {
        FieldKind::Money => parse_money(raw),
        FieldKind::Text => Ok(FieldValue::Text(raw.to_string())),
        FieldKind::Bool => parse_bool(raw).map(FieldValue::Bool),
        FieldKind::TriState => parse_tristate(raw).map(FieldValue::TriState),
        FieldKind::Date => parse_date(raw),
        FieldKind::Enum(options) => parse_enum(options, raw),
        // A caller reaching this arm dispatched generically on a Secret field instead of using the required
        // `parse_ssn`/`parse_ip_pin` entry points. We refuse rather than guess: silently validating a
        // would-be IP PIN under SSN rules (or vice versa) would be a WRONG-but-successful parse (e.g. a
        // 9-digit string that was meant to be rejected as a bad IP PIN could pass as a fine-looking SSN) —
        // strictly worse than a clean, loud error.
        FieldKind::Secret => Err(ParseError::BadSsn),
    }
}

/// Money: trim whitespace, parse as `Decimal`; non-numeric → `NotANumber`; negative → `Negative`. No `$`/comma
/// handling (kept simple per the brief) — a renderer that wants that strips it before calling `parse`.
fn parse_money(raw: &str) -> Result<FieldValue, ParseError> {
    let d = Decimal::from_str(raw.trim()).map_err(|_| ParseError::NotANumber)?;
    if d.is_sign_negative() {
        return Err(ParseError::Negative);
    }
    Ok(FieldValue::Money(d))
}

/// Bool tokens (case-insensitive, whitespace-trimmed): `y`/`yes`/`true` → `true`; `n`/`no`/`false` → `false`.
/// `FieldKind::Bool` is normally checkbox-driven (the renderer sets `Bool` directly, no text parse — see
/// `presidential_fund_*`), but a text-parse path must still behave sanely rather than panic. `ParseError` has
/// no dedicated "bad bool" variant, so an unrecognized token maps to `NotAChoice` (a bool is, in effect, a
/// fixed two-token choice).
fn parse_bool(raw: &str) -> Result<bool, ParseError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" => Ok(true),
        "n" | "no" | "false" => Ok(false),
        _ => Err(ParseError::NotAChoice),
    }
}

/// TriState tokens (case-insensitive, whitespace-trimmed): `y`/`yes` → `Some(true)`; `n`/`no` → `Some(false)`;
/// empty/blank → `None`. Anything else → `NotAChoice`.
fn parse_tristate(raw: &str) -> Result<Option<bool>, ParseError> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    match t.to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(Some(true)),
        "n" | "no" => Ok(Some(false)),
        _ => Err(ParseError::NotAChoice),
    }
}

/// Strict `YYYY-MM-DD` (the same format the CLI/TUI already parse dates with elsewhere in this workspace —
/// `time::macros::format_description!("[year]-[month]-[day]")`). Any other shape, or an out-of-range
/// component (month 13, day 32, ...), is `BadDate`. `parse` is for actual typed content; a blank/cleared date
/// goes through `apply`'s `ClearField`, not `parse("")`.
fn parse_date(raw: &str) -> Result<FieldValue, ParseError> {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    let d = time::Date::parse(raw.trim(), fmt).map_err(|_| ParseError::BadDate)?;
    Ok(FieldValue::Date(Some(d)))
}

/// The raw must equal one of `options` exactly (no case-folding, no trimming — options are stable tokens a
/// renderer presents as a closed choice, e.g. a dropdown, not free text).
fn parse_enum(options: &'static [&'static str], raw: &str) -> Result<FieldValue, ParseError> {
    if options.contains(&raw) {
        Ok(FieldValue::Choice(raw.to_string()))
    } else {
        Err(ParseError::NotAChoice)
    }
}

/// Parse a raw SSN entry (`FieldId::TpSsn`/`SpSsn`/`DepSsn`, all `FieldKind::Secret`).
///
/// Canonicalizes via [`Ssn::canonical`] — exactly nine digits, however typed (hyphens/whitespace stripped) —
/// so a too-short/too-long/non-digit entry is `BadSsn`, never a `SecretEntry`. The payload is the canonical
/// DIGITS (`Ssn::digits`), not the original punctuation: downstream re-canonicalization at print time
/// (`ReturnHeader::build`) is then a formality on an already-clean value, not a second decision point.
pub fn parse_ssn(raw: &str) -> Result<FieldValue, ParseError> {
    let ssn = Ssn::canonical(raw).map_err(|_| ParseError::BadSsn)?;
    Ok(FieldValue::SecretEntry(ssn.digits().to_string()))
}

/// Parse a raw IP PIN entry (`FieldId::IpPin`, `FieldKind::Secret`).
///
/// Canonicalizes via [`IpPin::canonical`] — exactly six digits, however typed — so anything else is
/// `BadIpPin`, never a `SecretEntry`.
pub fn parse_ip_pin(raw: &str) -> Result<FieldValue, ParseError> {
    let pin = IpPin::canonical(raw).map_err(|_| ParseError::BadIpPin)?;
    Ok(FieldValue::SecretEntry(pin.digits().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── Money ────────────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn money_negative_is_rejected() {
        assert_eq!(parse(FieldKind::Money, "-5"), Err(ParseError::Negative));
    }

    #[test]
    fn money_valid_parses_to_decimal() {
        assert_eq!(
            parse(FieldKind::Money, "50000"),
            Ok(FieldValue::Money(dec!(50000)))
        );
    }

    #[test]
    fn money_non_numeric_is_rejected() {
        assert_eq!(parse(FieldKind::Money, "abc"), Err(ParseError::NotANumber));
    }

    #[test]
    fn money_trims_surrounding_whitespace() {
        assert_eq!(
            parse(FieldKind::Money, "  123.45  "),
            Ok(FieldValue::Money(dec!(123.45)))
        );
    }

    // ── Text ─────────────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn text_is_unvalidated_passthrough() {
        assert_eq!(
            parse(FieldKind::Text, "Satoshi"),
            Ok(FieldValue::Text("Satoshi".into()))
        );
        assert_eq!(parse(FieldKind::Text, ""), Ok(FieldValue::Text("".into())));
    }

    // ── Date ─────────────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn date_valid_iso_parses() {
        assert_eq!(
            parse(FieldKind::Date, "1980-01-02"),
            Ok(FieldValue::Date(Some(time::macros::date!(1980 - 01 - 02))))
        );
    }

    #[test]
    fn date_out_of_range_month_is_bad_date() {
        assert_eq!(
            parse(FieldKind::Date, "1980-13-02"),
            Err(ParseError::BadDate)
        );
    }

    #[test]
    fn date_garbage_is_bad_date() {
        assert_eq!(parse(FieldKind::Date, "garbage"), Err(ParseError::BadDate));
    }

    // ── Enum ─────────────────────────────────────────────────────────────────────────────────────────────

    const FILING_STATUS_ENUM: FieldKind = FieldKind::Enum(&["Single", "Mfj"]);

    #[test]
    fn enum_valid_choice_is_accepted() {
        assert_eq!(
            parse(FILING_STATUS_ENUM, "Mfj"),
            Ok(FieldValue::Choice("Mfj".into()))
        );
    }

    #[test]
    fn enum_unknown_choice_is_rejected() {
        assert_eq!(parse(FILING_STATUS_ENUM, "Xx"), Err(ParseError::NotAChoice));
    }

    // ── TriState ─────────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn tristate_yes_no_blank() {
        assert_eq!(
            parse(FieldKind::TriState, "y"),
            Ok(FieldValue::TriState(Some(true)))
        );
        assert_eq!(
            parse(FieldKind::TriState, "Yes"),
            Ok(FieldValue::TriState(Some(true)))
        );
        assert_eq!(
            parse(FieldKind::TriState, "n"),
            Ok(FieldValue::TriState(Some(false)))
        );
        assert_eq!(
            parse(FieldKind::TriState, "No"),
            Ok(FieldValue::TriState(Some(false)))
        );
        assert_eq!(
            parse(FieldKind::TriState, ""),
            Ok(FieldValue::TriState(None))
        );
        assert_eq!(
            parse(FieldKind::TriState, "   "),
            Ok(FieldValue::TriState(None))
        );
    }

    #[test]
    fn tristate_unrecognized_token_is_rejected() {
        assert_eq!(
            parse(FieldKind::TriState, "maybe"),
            Err(ParseError::NotAChoice)
        );
    }

    // ── Bool ─────────────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn bool_tokens() {
        assert_eq!(parse(FieldKind::Bool, "yes"), Ok(FieldValue::Bool(true)));
        assert_eq!(parse(FieldKind::Bool, "TRUE"), Ok(FieldValue::Bool(true)));
        assert_eq!(parse(FieldKind::Bool, "no"), Ok(FieldValue::Bool(false)));
        assert_eq!(parse(FieldKind::Bool, "false"), Ok(FieldValue::Bool(false)));
        assert_eq!(parse(FieldKind::Bool, "nope"), Err(ParseError::NotAChoice));
    }

    // ── Secret: generic `parse` fallback ────────────────────────────────────────────────────────────────

    #[test]
    fn secret_via_generic_parse_always_refuses() {
        // Documents the fallback contract: `parse(Secret, _)` never succeeds, regardless of content —
        // callers must use `parse_ssn`/`parse_ip_pin`.
        assert_eq!(
            parse(FieldKind::Secret, "123456789"),
            Err(ParseError::BadSsn)
        );
        assert_eq!(parse(FieldKind::Secret, "112233"), Err(ParseError::BadSsn));
    }

    // ── Secret: SSN ──────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_ssn_non_digits_is_bad_ssn() {
        assert_eq!(parse_ssn("abc"), Err(ParseError::BadSsn));
    }

    /// The canonical-length gate (closes a known follow-up): a too-short digit string must never become a
    /// `SecretEntry` — `Ssn::canonical` enforces exactly nine digits (`SsnError::WrongLength`).
    #[test]
    fn parse_ssn_too_short_is_bad_ssn() {
        assert_eq!(parse_ssn("123"), Err(ParseError::BadSsn));
    }

    #[test]
    fn parse_ssn_too_long_is_bad_ssn() {
        assert_eq!(parse_ssn("1234567890"), Err(ParseError::BadSsn));
    }

    #[test]
    fn parse_ssn_valid_canonicalizes_to_digits() {
        assert_eq!(
            parse_ssn("123-45-6789"),
            Ok(FieldValue::SecretEntry("123456789".into()))
        );
        assert_eq!(
            parse_ssn("123456789"),
            Ok(FieldValue::SecretEntry("123456789".into()))
        );
    }

    // ── Secret: IP PIN ───────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn parse_ip_pin_non_digits_is_bad_ip_pin() {
        assert_eq!(parse_ip_pin("abc"), Err(ParseError::BadIpPin));
    }

    /// The canonical-length gate for IP PIN: a too-short digit string is never a `SecretEntry` —
    /// `IpPin::canonical` enforces exactly six digits.
    #[test]
    fn parse_ip_pin_too_short_is_bad_ip_pin() {
        assert_eq!(parse_ip_pin("123"), Err(ParseError::BadIpPin));
    }

    #[test]
    fn parse_ip_pin_valid_canonicalizes_to_digits() {
        assert_eq!(
            parse_ip_pin("112233"),
            Ok(FieldValue::SecretEntry("112233".into()))
        );
    }
}
