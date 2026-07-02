//! Tax-profile form state, field buffers, validation, and the mutation-modal payload.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! This module performs NO writes — it only holds form state and validates input.

use btctax_core::{
    Carryforward, EventId, FilingStatus, InboundClass, IncomeKind, Sat, TaxDate, TaxProfile, Usd,
    WalletId,
};
use ratatui::widgets::TableState;
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

// ── TargetList widget (shared by classify-inbound and reclassify-outflow) ─────

/// Selectable list of actionable targets rendered as a `ratatui` Table.
///
/// Callers (flow-open code) guarantee `items` is non-empty — an empty filtered
/// list never opens a flow [R0-M8]. The render's defensive "no items" placeholder
/// and Enter-swallow are belt-and-suspenders; they are unreachable under the
/// flow-open rule and carry no KAT.
pub struct TargetList<T> {
    pub items: Vec<T>,
    pub table_state: TableState,
}

impl<T> TargetList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut table_state = TableState::default();
        if !items.is_empty() {
            table_state.select(Some(0));
        }
        Self { items, table_state }
    }

    pub fn selected(&self) -> Option<&T> {
        self.table_state.selected().and_then(|i| self.items.get(i))
    }

    pub fn scroll_up(&mut self) {
        let next = match self.table_state.selected() {
            Some(i) if i > 0 => Some(i - 1),
            Some(_) => Some(0),
            None => None,
        };
        self.table_state.select(next);
    }

    pub fn scroll_down(&mut self) {
        let count = self.items.len();
        if count == 0 {
            return;
        }
        let next = match self.table_state.selected() {
            Some(i) => Some((i + 1).min(count - 1)),
            None => Some(0),
        };
        self.table_state.select(next);
    }

    pub fn go_top(&mut self) {
        if !self.items.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn go_bottom(&mut self) {
        let count = self.items.len();
        if count > 0 {
            self.table_state.select(Some(count - 1));
        }
    }
}

// ── Display data types for list items ─────────────────────────────────────────

/// Pre-computed display data for a classify-inbound list row.
#[derive(Clone)]
pub struct InboundListItem {
    /// The TransferIn event targeted by the `UnknownBasisInbound` blocker.
    pub blocker_event: EventId,
    /// Calendar date (tax timezone) of the TransferIn event.
    pub date: TaxDate,
    /// Principal sat from the TransferIn payload.
    pub sat: Sat,
    /// Wallet of the TransferIn event (None → displayed as "(no wallet)").
    pub wallet: Option<WalletId>,
    /// Blocker detail string.
    pub detail: String,
}

/// Pre-computed display data for a reclassify-outflow list row.
/// Added here per the spec Task-1 file list; fully used in Task 2 [R0-N3].
#[allow(dead_code)]
#[derive(Clone)]
pub struct OutflowListItem {
    /// The `PendingTransfer.event` EventId.
    pub transfer_out_event: EventId,
    /// Calendar date (tax timezone) of the TransferOut event.
    pub date: TaxDate,
    /// Principal sat from `PendingTransfer.principal_sat`.
    pub principal_sat: Sat,
    /// Wallet of the TransferOut event.
    pub wallet: Option<WalletId>,
}

// ── Classify-inbound flow types ───────────────────────────────────────────────

/// Which variant the picker is showing for classify-inbound step 2.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InboundVariant {
    Income,
    GiftReceived,
}

/// Step in the classify-inbound flow.
pub enum ClassifyInboundStep {
    List,
    VariantPicker {
        item: InboundListItem,
        variant: InboundVariant,
    },
    IncomeForm {
        item: InboundListItem,
        /// Current IncomeKind selection; initial: Mining [R0-M3].
        kind: IncomeKind,
        fmv_buf: FieldBuffer,
        /// Default: false (CLI parity).
        business: bool,
        /// 0 = kind (Tab-cycles), 1 = fmv (text), 2 = business (Space toggle).
        focus: usize,
        error: Option<String>,
    },
    GiftForm {
        item: InboundListItem,
        fmv_at_gift_buf: FieldBuffer,
        donor_basis_buf: FieldBuffer,
        donor_acquired_at_buf: FieldBuffer,
        /// 0 = fmv_at_gift, 1 = donor_basis, 2 = donor_acquired_at.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the classify-inbound flow.  Owns its target list.
pub struct ClassifyInboundFlowState {
    /// Owned list — no standalone list field on EditorApp [R0-I2].
    pub list: TargetList<InboundListItem>,
    pub step: ClassifyInboundStep,
}

/// Payload for the classify-inbound confirmation modal.
pub struct ClassifyInboundModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    /// The VALIDATED classification — what will be persisted.
    pub as_: InboundClass,
}

// ── Helper: IncomeKind cycling and display ────────────────────────────────────

/// Cycle through the 5 `IncomeKind` variants in declaration order (event.rs:29–35).
/// Mining → Staking → Interest → Airdrop → Reward → Mining.
pub fn cycle_income_kind(kind: IncomeKind) -> IncomeKind {
    match kind {
        IncomeKind::Mining => IncomeKind::Staking,
        IncomeKind::Staking => IncomeKind::Interest,
        IncomeKind::Interest => IncomeKind::Airdrop,
        IncomeKind::Airdrop => IncomeKind::Reward,
        IncomeKind::Reward => IncomeKind::Mining,
    }
}

/// Return the lowercase display tag for an `IncomeKind` (matches CLI render convention).
pub fn income_kind_display(kind: IncomeKind) -> &'static str {
    match kind {
        IncomeKind::Mining => "mining",
        IncomeKind::Staking => "staking",
        IncomeKind::Interest => "interest",
        IncomeKind::Airdrop => "airdrop",
        IncomeKind::Reward => "reward",
    }
}

// ── Classify-inbound validation ───────────────────────────────────────────────

/// Validate the Income variant of the classify-inbound form.
///
/// `kind` is always structurally valid (picker).  `fmv_buf` is optional:
/// empty (byte-len 0) → `None`; non-empty → `parse_usd_arg(trim)`.
/// [R0-M4] whitespace-only is NOT empty.
///
/// Returns the validated `InboundClass::Income` or an error string.
pub fn validate_classify_inbound_income(
    kind: IncomeKind,
    fmv_buf: &FieldBuffer,
    business: bool,
) -> Result<InboundClass, String> {
    let fmv = if fmv_buf.is_empty() {
        None
    } else {
        let trimmed = fmv_buf.buf.trim();
        Some(Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed:?}"))?)
    };
    Ok(InboundClass::Income {
        kind,
        fmv,
        business,
    })
}

/// Validate the GiftReceived variant of the classify-inbound form.
///
/// `fmv_at_gift_buf` is REQUIRED (empty → "fmv-at-gift is required").
/// `donor_basis_buf` and `donor_acquired_at_buf` are optional.
///
/// Date format: YYYY-MM-DD (`parse_date_arg` semantics — `Date::parse(trim, "[year]-[month]-[day]")`).
/// [R0-M4] whitespace-only is NOT empty.
///
/// Returns the validated `InboundClass::GiftReceived` or an error string.
pub fn validate_classify_inbound_gift(
    fmv_at_gift_buf: &FieldBuffer,
    donor_basis_buf: &FieldBuffer,
    donor_acquired_at_buf: &FieldBuffer,
) -> Result<InboundClass, String> {
    if fmv_at_gift_buf.is_empty() {
        return Err("fmv-at-gift is required".to_string());
    }
    let trimmed = fmv_at_gift_buf.buf.trim();
    let fmv_at_gift = Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed:?}"))?;

    let donor_basis = if donor_basis_buf.is_empty() {
        None
    } else {
        let t = donor_basis_buf.buf.trim();
        Some(Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?)
    };

    let donor_acquired_at = if donor_acquired_at_buf.is_empty() {
        None
    } else {
        let t = donor_acquired_at_buf.buf.trim();
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        Some(time::Date::parse(t, fmt).map_err(|e| format!("bad date {t:?}: {e}"))?)
    };

    Ok(InboundClass::GiftReceived {
        donor_basis,
        donor_acquired_at,
        fmv_at_gift,
    })
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

    // ── KAT-V-CI-9: IncomeKind cycles in declaration order, initial = Mining ──

    #[test]
    fn kat_v_ci_9_income_kind_cycles_five_variants_wraps_to_mining() {
        let mut kind = IncomeKind::Mining; // initial [R0-M3]
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Staking);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Interest);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Airdrop);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Reward);
        kind = cycle_income_kind(kind);
        assert_eq!(kind, IncomeKind::Mining, "5 cycles must wrap to Mining");
    }

    // ── KAT-V-CI-1: Income fmv empty → None (valid, no error) ───────────────

    #[test]
    fn kat_v_ci_1_income_fmv_empty_gives_none() {
        let result =
            validate_classify_inbound_income(IncomeKind::Mining, &FieldBuffer::new(), false);
        let cls = result.unwrap();
        if let InboundClass::Income { fmv, .. } = cls {
            assert!(fmv.is_none(), "empty fmv_buf must produce fmv=None");
        } else {
            panic!("expected Income variant");
        }
    }

    // ── KAT-V-CI-2: Income fmv valid decimal → parses correctly ──────────────

    #[test]
    fn kat_v_ci_2_income_fmv_valid_decimal_parses() {
        use rust_decimal_macros::dec;
        let mut buf = FieldBuffer::new();
        buf.set("45.50");
        let result = validate_classify_inbound_income(IncomeKind::Staking, &buf, false);
        let cls = result.unwrap();
        if let InboundClass::Income {
            fmv,
            kind,
            business,
        } = cls
        {
            assert_eq!(fmv, Some(dec!(45.50)));
            assert_eq!(kind, IncomeKind::Staking);
            assert!(!business);
        } else {
            panic!("expected Income variant");
        }
    }

    // ── KAT-V-CI-3: Income fmv non-numeric → parse error "bad USD…" ──────────

    #[test]
    fn kat_v_ci_3_income_fmv_nonnumeric_is_parse_error() {
        let mut buf = FieldBuffer::new();
        buf.set("abc");
        let err = validate_classify_inbound_income(IncomeKind::Mining, &buf, false).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "non-numeric fmv must produce 'bad USD' error; got: {err}"
        );
    }

    // ── KAT-V-CI-4: Income fmv whitespace-only → parse error (not None) ──────

    #[test]
    fn kat_v_ci_4_income_fmv_whitespace_only_is_parse_error_not_none() {
        let mut buf = FieldBuffer::new();
        buf.set("   "); // whitespace-only: is_empty()==false [R0-M4]
        let err = validate_classify_inbound_income(IncomeKind::Mining, &buf, false).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only fmv must be a parse error, not None; got: {err}"
        );
    }

    // ── KAT-V-CI-5: GiftReceived fmv_at_gift empty → "fmv-at-gift is required" ─

    #[test]
    fn kat_v_ci_5_gift_fmv_at_gift_empty_is_required_error() {
        let err = validate_classify_inbound_gift(
            &FieldBuffer::new(),
            &FieldBuffer::new(),
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("fmv-at-gift") && err.contains("required"),
            "empty fmv_at_gift must produce 'required' error; got: {err}"
        );
    }

    // ── KAT-V-CI-6: GiftReceived fmv_at_gift valid → parses correctly ─────────

    #[test]
    fn kat_v_ci_6_gift_fmv_at_gift_valid_parses() {
        use rust_decimal_macros::dec;
        let mut buf = FieldBuffer::new();
        buf.set("500.00");
        let cls =
            validate_classify_inbound_gift(&buf, &FieldBuffer::new(), &FieldBuffer::new()).unwrap();
        if let InboundClass::GiftReceived {
            fmv_at_gift,
            donor_basis,
            donor_acquired_at,
        } = cls
        {
            assert_eq!(fmv_at_gift, dec!(500.00));
            assert!(donor_basis.is_none());
            assert!(donor_acquired_at.is_none());
        } else {
            panic!("expected GiftReceived variant");
        }
    }

    // ── KAT-V-CI-7: GiftReceived donor_acquired_at valid YYYY-MM-DD → parses ──

    #[test]
    fn kat_v_ci_7_gift_donor_acquired_at_valid_parses() {
        use time::macros::date;
        let mut fmv_buf = FieldBuffer::new();
        fmv_buf.set("500.00");
        let mut date_buf = FieldBuffer::new();
        date_buf.set("2022-04-01");
        let cls = validate_classify_inbound_gift(&fmv_buf, &FieldBuffer::new(), &date_buf).unwrap();
        if let InboundClass::GiftReceived {
            donor_acquired_at, ..
        } = cls
        {
            assert_eq!(donor_acquired_at, Some(date!(2022 - 04 - 01)));
        } else {
            panic!("expected GiftReceived variant");
        }
    }

    // ── KAT-V-CI-8: GiftReceived donor_acquired_at bad format → "bad date…" ───

    #[test]
    fn kat_v_ci_8_gift_donor_acquired_at_bad_format_is_error() {
        let mut fmv_buf = FieldBuffer::new();
        fmv_buf.set("500.00");
        let mut date_buf = FieldBuffer::new();
        date_buf.set("not-a-date");
        let err =
            validate_classify_inbound_gift(&fmv_buf, &FieldBuffer::new(), &date_buf).unwrap_err();
        assert!(
            err.contains("bad date"),
            "bad date format must produce 'bad date' error; got: {err}"
        );
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
