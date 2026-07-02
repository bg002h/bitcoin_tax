//! Tax-profile form state, field buffers, validation, and the mutation-modal payload.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! This module performs NO writes — it only holds form state and validates input.

use btctax_core::{Carryforward, FilingStatus, TaxProfile, Usd};
use std::str::FromStr;

/// Maximum byte-length of a money field buffer (64 chars is ample for any Decimal).
pub const FIELD_CAP: usize = 64;

/// A single money-field text input buffer.
///
/// Follows the `UnlockState` push/pop discipline (unlock.rs:42–63 — the only
/// text-input precedent): pre-allocated to `FIELD_CAP`, never reallocates.
/// Rendered **plaintext** (not masked — these are not secrets).
pub struct FieldBuffer {
    pub buf: String,
}

impl FieldBuffer {
    pub fn new() -> Self {
        Self {
            buf: String::with_capacity(FIELD_CAP),
        }
    }

    /// Push one character, silently ignoring input past FIELD_CAP.
    pub fn push_char(&mut self, c: char) {
        if self.buf.len() + c.len_utf8() <= FIELD_CAP {
            self.buf.push(c);
        }
    }

    /// Remove the last character (backspace). No-op when empty.
    pub fn pop_char(&mut self) {
        self.buf.pop();
    }

    /// Set the buffer content, respecting FIELD_CAP.
    pub fn set(&mut self, s: &str) {
        self.buf.clear();
        for c in s.chars() {
            self.push_char(c);
        }
    }

    /// True when byte-length is 0.
    ///
    /// [R0-M4] "empty" = len==0, checked BEFORE any trimming. Whitespace-only is NOT empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for FieldBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Field ordering within `ProfileFormState::fields[0..=8]`:
///
/// 0 = ordinary_taxable_income        (REQUIRED)
/// 1 = magi_excluding_crypto          (REQUIRED)
/// 2 = qualified_dividends_and_other_pref_income (REQUIRED)
/// 3 = other_net_capital_gain         (optional, default 0)
/// 4 = carryforward_short             (optional, default 0)
/// 5 = carryforward_long              (optional, default 0)
/// 6 = w2_ss_wages                    (optional, default 0, must be ≥ 0)
/// 7 = w2_medicare_wages              (optional, default 0, must be ≥ 0)
/// 8 = schedule_c_expenses            (optional, default 0, must be ≥ 0)
pub const FIELD_LABELS: [&str; 9] = [
    "ordinary_taxable_income *req",
    "magi_excluding_crypto *req",
    "qualified_dividends_and_other_pref_income *req",
    "other_net_capital_gain",
    "capital_loss_carryforward_in.short",
    "capital_loss_carryforward_in.long",
    "w2_ss_wages (≥0)",
    "w2_medicare_wages (≥0)",
    "schedule_c_expenses (≥0)",
];

/// Live state for the tax-profile form.
///
/// `focus == 0` = filing_status (cycled via Tab); `focus == 1..=9` = money fields.
pub struct ProfileFormState {
    pub year: i32,
    pub filing_status: FilingStatus,
    pub fields: [FieldBuffer; 9],
    pub focus: usize,
    pub error: Option<String>,
}

impl ProfileFormState {
    pub fn new(year: i32) -> Self {
        Self {
            year,
            filing_status: FilingStatus::Single,
            fields: std::array::from_fn(|_| FieldBuffer::new()),
            focus: 0,
            error: None,
        }
    }
}

/// Payload for the per-mutation confirmation modal.
///
/// Contains the VALIDATED profile (not raw buffers) — what will be written, verbatim.
pub struct MutationModalState {
    pub year: i32,
    pub profile: TaxProfile,
}

/// Cycle through the 5 `FilingStatus` variants in declaration order.
/// Tab from the last wraps back to the first.
pub fn cycle_filing_status(fs: FilingStatus) -> FilingStatus {
    match fs {
        FilingStatus::Single => FilingStatus::Mfj,
        FilingStatus::Mfj => FilingStatus::Mfs,
        FilingStatus::Mfs => FilingStatus::HoH,
        FilingStatus::HoH => FilingStatus::Qss,
        FilingStatus::Qss => FilingStatus::Single,
    }
}

/// Validate the form and return a `TaxProfile` or an error string.
///
/// Mirrors the CLI's clap-side rules (main.rs:688–760) EXACTLY:
/// - Rule 1: filing_status always valid (structural)
/// - Rules 2–4: required fields (empty = len-0 → "... is required"; else parse)
/// - Rules 5–7: optional (empty → 0; else parse; negatives accepted — CLI parity)
/// - Rules 8–10: optional (empty → 0; else parse; negative → error)
///
/// [R0-M4] "empty" = byte-len 0, checked BEFORE trimming. Whitespace-only → parse error.
pub fn validate(form: &ProfileFormState) -> Result<TaxProfile, String> {
    let oti = parse_required(&form.fields[0], "ordinary-taxable-income")?;
    let magi = parse_required(&form.fields[1], "magi-excluding-crypto")?;
    let qd = parse_required(&form.fields[2], "qualified-dividends-and-other-pref-income")?;

    let oncg = parse_optional(&form.fields[3])?;
    let cf_short = parse_optional(&form.fields[4])?;
    let cf_long = parse_optional(&form.fields[5])?;

    let w2_ss = parse_optional(&form.fields[6])?;
    if w2_ss.is_sign_negative() {
        return Err("w2-ss-wages must not be negative".to_string());
    }
    let w2_medicare = parse_optional(&form.fields[7])?;
    if w2_medicare.is_sign_negative() {
        return Err("w2-medicare-wages must not be negative".to_string());
    }
    let sce = parse_optional(&form.fields[8])?;
    if sce.is_sign_negative() {
        return Err("schedule-c-expenses must not be negative".to_string());
    }

    Ok(TaxProfile {
        filing_status: form.filing_status,
        ordinary_taxable_income: oti,
        magi_excluding_crypto: magi,
        qualified_dividends_and_other_pref_income: qd,
        other_net_capital_gain: oncg,
        capital_loss_carryforward_in: Carryforward {
            short: cf_short,
            long: cf_long,
        },
        w2_ss_wages: w2_ss,
        w2_medicare_wages: w2_medicare,
        schedule_c_expenses: sce,
    })
}

/// Parse a REQUIRED field: byte-len-0 → "name is required"; else Decimal::from_str(trim).
fn parse_required(buf: &FieldBuffer, name: &str) -> Result<Usd, String> {
    if buf.is_empty() {
        return Err(format!("{name} is required"));
    }
    let trimmed = buf.buf.trim();
    Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed}"))
}

/// Parse an OPTIONAL field: byte-len-0 → 0; else Decimal::from_str(trim).
fn parse_optional(buf: &FieldBuffer) -> Result<Usd, String> {
    if buf.is_empty() {
        return Ok(Usd::ZERO);
    }
    let trimmed = buf.buf.trim();
    Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed}"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_valid_form() -> ProfileFormState {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("120000");
        f.fields[1].set("130000");
        f.fields[2].set("0");
        f
    }

    // ── KAT-V1: FilingStatus cycles through all 5 variants and wraps ─────────

    #[test]
    fn kat_v1_filing_status_cycles_five_times_returns_to_start() {
        let mut fs = FilingStatus::Single;
        let start = fs;
        for _ in 0..5 {
            fs = cycle_filing_status(fs);
        }
        assert_eq!(fs, start, "5 cycles must return to Single");
    }

    #[test]
    fn kat_v1_all_variants_reachable_in_cycle() {
        let mut seen = std::collections::HashSet::new();
        let mut fs = FilingStatus::Single;
        for _ in 0..5 {
            seen.insert(format!("{fs:?}"));
            fs = cycle_filing_status(fs);
        }
        assert_eq!(seen.len(), 5, "all 5 variants must be reachable");
    }

    // ── KAT-V2..V4: required fields ─────────────────────────────────────────

    #[test]
    fn kat_v2_empty_ordinary_taxable_income_is_required_error() {
        let f = ProfileFormState::new(2025);
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("ordinary-taxable-income") && err.contains("required"),
            "got: {err}"
        );
    }

    #[test]
    fn kat_v3_empty_magi_is_required_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("100000"); // fill required[0]
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("magi-excluding-crypto") && err.contains("required"),
            "got: {err}"
        );
    }

    #[test]
    fn kat_v4_empty_qualified_dividends_is_required_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("100000");
        f.fields[1].set("100000");
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("qualified-dividends") && err.contains("required"),
            "got: {err}"
        );
    }

    // ── KAT-V5..V7: empty optional fields default to 0 ──────────────────────

    #[test]
    fn kat_v5_empty_other_net_capital_gain_defaults_to_zero() {
        let f = make_valid_form();
        let p = validate(&f).unwrap();
        assert_eq!(p.other_net_capital_gain, Usd::ZERO);
    }

    #[test]
    fn kat_v6_empty_carryforward_defaults_to_zero() {
        let f = make_valid_form();
        let p = validate(&f).unwrap();
        assert_eq!(p.capital_loss_carryforward_in.short, Usd::ZERO);
        assert_eq!(p.capital_loss_carryforward_in.long, Usd::ZERO);
    }

    #[test]
    fn kat_v7_empty_optional_defaults_to_zero() {
        let f = make_valid_form();
        let p = validate(&f).unwrap();
        assert_eq!(p.w2_ss_wages, Usd::ZERO);
        assert_eq!(p.w2_medicare_wages, Usd::ZERO);
        assert_eq!(p.schedule_c_expenses, Usd::ZERO);
    }

    // ── KAT-V8..V10: negative optional non-negative fields → error ───────────

    #[test]
    fn kat_v8_negative_w2_ss_wages_is_rejected() {
        let mut f = make_valid_form();
        f.fields[6].set("-1");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("w2-ss-wages"), "got: {err}");
    }

    #[test]
    fn kat_v9_negative_w2_medicare_wages_is_rejected() {
        let mut f = make_valid_form();
        f.fields[7].set("-1");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("w2-medicare-wages"), "got: {err}");
    }

    #[test]
    fn kat_v10_negative_schedule_c_expenses_is_rejected() {
        let mut f = make_valid_form();
        f.fields[8].set("-1");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("schedule-c-expenses"), "got: {err}");
    }

    // ── KAT-V8b..V10b: fields 2–7 accept negatives (CLI parity) ────────────

    #[test]
    fn kat_v8b_negative_values_accepted_for_required_and_optional_fields() {
        // Required fields accept negative (CLI parity: no negativity check for 2-4)
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("-50000"); // ordinary_taxable_income: negative accepted
        f.fields[1].set("-1000");
        f.fields[2].set("-500");
        f.fields[3].set("-100"); // other_net_capital_gain: negative accepted
        f.fields[4].set("-50"); // carryforward_short: negative accepted
        f.fields[5].set("-50"); // carryforward_long: negative accepted
        let p = validate(&f).unwrap();
        assert_eq!(p.ordinary_taxable_income, dec!(-50000));
        assert_eq!(p.other_net_capital_gain, dec!(-100));
        assert_eq!(p.capital_loss_carryforward_in.short, dec!(-50));
    }

    // ── KAT-V11: whitespace-only buffers ────────────────────────────────────

    #[test]
    fn kat_v11_whitespace_only_optional_is_parse_error_not_zero() {
        let mut f = make_valid_form();
        f.fields[3].set("  "); // other_net_capital_gain — whitespace-only
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only optional must be a parse error, not 0; got: {err}"
        );
    }

    #[test]
    fn kat_v11_whitespace_only_required_is_parse_error_not_required_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("  "); // ordinary_taxable_income — whitespace-only
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only required must be parse error, not 'required'; got: {err}"
        );
        assert!(
            !err.contains("required"),
            "must not say 'required' for whitespace-only; got: {err}"
        );
    }

    #[test]
    fn kat_v11_len_zero_required_is_required_error() {
        let f = ProfileFormState::new(2025); // buffers all empty (len==0)
        let err = validate(&f).unwrap_err();
        assert!(
            err.contains("required"),
            "len-0 must be 'required' error; got: {err}"
        );
    }

    #[test]
    fn kat_v11_len_zero_optional_is_zero() {
        let f = make_valid_form(); // optional buffers are len-0
        let p = validate(&f).unwrap();
        assert_eq!(p.other_net_capital_gain, Usd::ZERO);
    }

    // ── Parse failure: non-numeric ───────────────────────────────────────────

    #[test]
    fn non_numeric_required_field_is_parse_error() {
        let mut f = ProfileFormState::new(2025);
        f.fields[0].set("abc");
        let err = validate(&f).unwrap_err();
        assert!(err.contains("bad USD"), "got: {err}");
    }

    // ── Full valid form round-trips ──────────────────────────────────────────

    #[test]
    fn valid_form_produces_correct_tax_profile() {
        let mut f = ProfileFormState::new(2025);
        f.filing_status = FilingStatus::Mfj;
        f.fields[0].set("120000");
        f.fields[1].set("130000");
        f.fields[2].set("5000");
        f.fields[3].set("1000");
        f.fields[4].set("500");
        f.fields[5].set("250");
        f.fields[6].set("80000");
        f.fields[7].set("85000");
        f.fields[8].set("3000");
        let p = validate(&f).unwrap();
        assert_eq!(p.filing_status, FilingStatus::Mfj);
        assert_eq!(p.ordinary_taxable_income, dec!(120000));
        assert_eq!(p.magi_excluding_crypto, dec!(130000));
        assert_eq!(p.qualified_dividends_and_other_pref_income, dec!(5000));
        assert_eq!(p.other_net_capital_gain, dec!(1000));
        assert_eq!(p.capital_loss_carryforward_in.short, dec!(500));
        assert_eq!(p.capital_loss_carryforward_in.long, dec!(250));
        assert_eq!(p.w2_ss_wages, dec!(80000));
        assert_eq!(p.w2_medicare_wages, dec!(85000));
        assert_eq!(p.schedule_c_expenses, dec!(3000));
    }
}
