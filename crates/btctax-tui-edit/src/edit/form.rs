//! Tax-profile form state, field buffers, validation, and the mutation-modal payload.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! This module performs NO writes — it only holds form state and validates input.

use btctax_core::{
    AllocLot, AllocMethod, BasisSource, Carryforward, DisposalProposal, DisposeKind,
    DonationDetails, EventId, FilingStatus, FmvStatus, Form8283Section, InboundClass, IncomeKind,
    LotId, LotMethod, LotPick, ManualFmv, OutflowClass, Persistability, ReclassifyIncome,
    ReclassifyOutflow, Sat, TaxDate, TaxProfile, TransferTarget, Usd, WalletId,
};
use ratatui::widgets::TableState;
use std::str::FromStr;

/// Maximum byte-length of a money field buffer (64 chars is ample for any Decimal).
pub const FIELD_CAP: usize = 64;

/// Byte-length cap for donation **free-text** fields (donee/appraiser name+address,
/// appraiser qualifications, fmv-method override). [R0-N1] A generous BOUND for TUI
/// rendering — the CLI (`Option<String>`) is unbounded; 512 covers realistic
/// addresses / multi-clause qualifications while keeping the buffer render-safe.
pub const FREETEXT_CAP: usize = 512;

/// A single text input buffer.
///
/// Follows the `UnlockState` push/pop discipline (unlock.rs:42–63 — the only
/// text-input precedent): pre-allocated to its `cap`, never reallocates.
/// Rendered **plaintext** (not masked — these are not secrets).
///
/// `cap` is per-instance: money/structured fields keep `FIELD_CAP`; donation
/// free-text fields use `FREETEXT_CAP` (see `with_cap`).
#[derive(Debug)]
pub struct FieldBuffer {
    pub buf: String,
    cap: usize,
}

impl FieldBuffer {
    pub fn new() -> Self {
        Self::with_cap(FIELD_CAP)
    }

    /// Construct a buffer with an explicit byte-length cap (pre-allocated, never reallocates).
    pub fn with_cap(cap: usize) -> Self {
        Self {
            buf: String::with_capacity(cap),
            cap,
        }
    }

    /// Push one character, silently ignoring input past this buffer's `cap`.
    pub fn push_char(&mut self, c: char) {
        if self.buf.len() + c.len_utf8() <= self.cap {
            self.buf.push(c);
        }
    }

    /// Remove the last character (backspace). No-op when empty.
    pub fn pop_char(&mut self) {
        self.buf.pop();
    }

    /// Set the buffer content, respecting this buffer's `cap`.
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

/// Cycle the safe-harbor allocation `AllocMethod` (mirrors `cycle_filing_status`). The toggle changes
/// ONLY the recorded `method` tag — the displayed residue lots are method-INDEPENDENT (gotcha G3):
/// both `ActualPosition` and `ProRata` seed from the SAME per-wallet actuals; the method affects only
/// the engine's timebar/effectiveness rule, never the lot list. ProRata cross-wallet redistribution is
/// NOT implemented (core O4) — the tag records the election; it does not redistribute basis.
pub fn cycle_alloc_method(m: AllocMethod) -> AllocMethod {
    match m {
        AllocMethod::ActualPosition => AllocMethod::ProRata,
        AllocMethod::ProRata => AllocMethod::ActualPosition,
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

// ── Reclassify-outflow flow types ────────────────────────────────────────────

/// Which outflow kind is selected in the reclassify-outflow kind picker.
///
/// Tab cycles: Sell → Spend → Gift → Donate → Sell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutflowKind {
    Sell,
    Spend,
    Gift,
    Donate,
}

/// Cycle through the 4 `OutflowKind` variants in picker order.
/// Sell → Spend → Gift → Donate → Sell.
pub fn cycle_outflow_kind(kind: OutflowKind) -> OutflowKind {
    match kind {
        OutflowKind::Sell => OutflowKind::Spend,
        OutflowKind::Spend => OutflowKind::Gift,
        OutflowKind::Gift => OutflowKind::Donate,
        OutflowKind::Donate => OutflowKind::Sell,
    }
}

/// Step in the reclassify-outflow flow.
pub enum ReclassifyOutflowStep {
    List,
    KindPicker {
        item: OutflowListItem,
        /// Initial: Sell.
        kind: OutflowKind,
    },
    FieldForm {
        item: OutflowListItem,
        kind: OutflowKind,
        /// Gross proceeds (sell/spend) or FMV (gift/donate); REQUIRED.
        amount_buf: FieldBuffer,
        /// Network fee; optional.
        fee_buf: FieldBuffer,
        /// Appraisal required toggle; donate only (default false).
        appraisal: bool,
        /// Donee free-form label; optional; gift and donate only.
        donee_buf: FieldBuffer,
        /// Focus: 0=amount, 1=fee, 2=appraisal (donate only), 3=donee (gift/donate only).
        /// Hidden rows are skipped in focus cycling.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the reclassify-outflow flow. Owns its target list [R0-I2].
pub struct ReclassifyOutflowFlowState {
    /// Owned list — no standalone list field on EditorApp.
    pub list: TargetList<OutflowListItem>,
    pub step: ReclassifyOutflowStep,
}

/// Payload for the reclassify-outflow confirmation modal.
pub struct ReclassifyOutflowModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub principal_sat: Sat,
    /// The VALIDATED payload — what will be persisted.
    pub payload: ReclassifyOutflow,
}

/// Return the label for the `amount` field based on the outflow kind.
///
/// [R0-I3]: gross proceeds for sell AND spend; FMV for gift/donate.
pub fn amount_label(kind: OutflowKind) -> &'static str {
    match kind {
        OutflowKind::Sell | OutflowKind::Spend => "gross proceeds (USD)",
        OutflowKind::Gift | OutflowKind::Donate => "FMV (USD)",
    }
}

/// Compute the maximum focus index for the reclassify-outflow field form.
///
/// Focus order: 0=amount, 1=fee, 2=appraisal (donate only), 3=donee (gift/donate only).
/// Hidden rows are skipped: for sell/spend max focus is 1; for gift max is 3 (skipping 2);
/// for donate max is 3.
pub fn max_focus_for_kind(kind: OutflowKind) -> usize {
    match kind {
        OutflowKind::Sell | OutflowKind::Spend => 1,
        OutflowKind::Gift | OutflowKind::Donate => 3,
    }
}

/// Step to the next visible focus row (skipping hidden rows).
pub fn next_focus(focus: usize, kind: OutflowKind) -> usize {
    let max = max_focus_for_kind(kind);
    let next = (focus + 1).min(max);
    // Row 2 (appraisal) is only shown for donate; skip for gift.
    if next == 2 && kind == OutflowKind::Gift {
        3.min(max)
    } else {
        next
    }
}

/// Step to the previous visible focus row (skipping hidden rows).
pub fn prev_focus(focus: usize, kind: OutflowKind) -> usize {
    if focus == 0 {
        return 0;
    }
    let prev = focus - 1;
    // Row 2 (appraisal) is only shown for donate; skip for gift.
    if prev == 2 && kind == OutflowKind::Gift {
        1
    } else {
        prev
    }
}

// ── Reclassify-outflow validation ────────────────────────────────────────────

/// Validate the reclassify-outflow field form and build a `ReclassifyOutflow` payload.
///
/// `amount` is REQUIRED (empty byte-len-0 → "amount is required"; whitespace-only → parse error).
/// `fee` is optional (empty → `None`). `appraisal` is a toggle bool (always valid).
/// `donee` is optional free-form text; empty → `None`; non-empty → `Some(trim().to_owned())`.
/// [R0-M4] whitespace-only is NOT empty.
/// [R0-I3] `amount` is gross proceeds for sell/spend; FMV for gift/donate — same field, different label.
///
/// Returns the validated `ReclassifyOutflow` or an error string.
pub fn validate_reclassify_outflow(
    item: &OutflowListItem,
    kind: OutflowKind,
    amount_buf: &FieldBuffer,
    fee_buf: &FieldBuffer,
    appraisal: bool,
    donee_buf: &FieldBuffer,
) -> Result<ReclassifyOutflow, String> {
    // amount: REQUIRED
    if amount_buf.is_empty() {
        return Err("amount is required".to_string());
    }
    let amount_trimmed = amount_buf.buf.trim();
    let principal_proceeds_or_fmv =
        Usd::from_str(amount_trimmed).map_err(|_| format!("bad USD {amount_trimmed:?}"))?;

    // fee: optional
    let fee_usd = if fee_buf.is_empty() {
        None
    } else {
        let t = fee_buf.buf.trim();
        Some(Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?)
    };

    // donee: optional free-form; trimmed + capped at FIELD_CAP
    let donee = if donee_buf.is_empty() {
        None
    } else {
        Some(donee_buf.buf.trim().to_owned())
    };

    // Build OutflowClass
    let as_ = match kind {
        OutflowKind::Sell => OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        },
        OutflowKind::Spend => OutflowClass::Dispose {
            kind: DisposeKind::Spend,
        },
        OutflowKind::Gift => OutflowClass::GiftOut,
        OutflowKind::Donate => OutflowClass::Donate {
            appraisal_required: appraisal,
        },
    };

    Ok(ReclassifyOutflow {
        transfer_out_event: item.transfer_out_event.clone(),
        as_,
        principal_proceeds_or_fmv,
        fee_usd,
        donee,
    })
}

// ── Reclassify-income flow types ─────────────────────────────────────────────

/// Pre-computed display data for a reclassify-income list row.
#[derive(Clone)]
pub struct IncomeListItem {
    pub income_event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: IncomeKind,
    pub business: bool,
    /// FMV from income_recognized if present; None if FmvMissing.
    pub fmv: Option<Usd>,
    pub wallet: Option<WalletId>,
}

/// Pre-computed display data for a set-fmv list row.
#[derive(Clone)]
pub struct FmvListItem {
    pub event: EventId,
    pub date: TaxDate,
    pub sat: Sat,
    pub kind: IncomeKind,
    pub wallet: Option<WalletId>,
}

/// Step in the reclassify-income flow.
pub enum ReclassifyIncomeStep {
    List,
    FieldForm {
        item: IncomeListItem,
        /// 3-state: None = not chosen (REQUIRED-EXPLICIT).
        business: Option<bool>,
        /// None = keep original.
        kind: Option<IncomeKind>,
        /// 0 = business, 1 = kind.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the reclassify-income flow.
pub struct ReclassifyIncomeFlowState {
    pub list: TargetList<IncomeListItem>,
    pub step: ReclassifyIncomeStep,
}

/// Payload for the reclassify-income confirmation modal.
pub struct ReclassifyIncomeModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub original_kind: IncomeKind,
    pub original_business: bool,
    pub new_business: bool,
    pub new_kind: Option<IncomeKind>,
}

/// Step in the set-fmv flow.
pub enum SetFmvStep {
    List,
    FieldForm {
        item: FmvListItem,
        usd_fmv_buf: FieldBuffer,
        error: Option<String>,
    },
}

/// Full state for the set-fmv flow.
pub struct SetFmvFlowState {
    pub list: TargetList<FmvListItem>,
    pub step: SetFmvStep,
}

/// Payload for the set-fmv confirmation modal.
pub struct SetFmvModalState {
    pub target_event: EventId,
    pub target_date: TaxDate,
    pub target_sat: Sat,
    pub target_kind: IncomeKind,
    pub usd_fmv: Usd,
}

// ── Reclassify-income validation ─────────────────────────────────────────────

/// Validate the reclassify-income field form.
///
/// `business`: None → "business is required (press Tab to choose true or false)".
/// `kind`: always valid (None = keep original).
///
/// Returns the validated `EventPayload::ReclassifyIncome(…)` or an error string.
pub fn validate_reclassify_income(
    item: &IncomeListItem,
    business: Option<bool>,
    kind: Option<IncomeKind>,
) -> Result<btctax_core::EventPayload, String> {
    let b = match business {
        None => return Err("business is required (press Tab to choose true or false)".to_string()),
        Some(b) => b,
    };
    Ok(btctax_core::EventPayload::ReclassifyIncome(
        ReclassifyIncome {
            income_event: item.income_event.clone(),
            business: b,
            kind,
        },
    ))
}

/// Validate the set-fmv field form.
///
/// `usd_fmv_buf`: empty (len==0) → "usd-fmv is required"; non-empty → parse.
///
/// Returns the validated `EventPayload::ManualFmv(…)` or an error string.
pub fn validate_set_fmv(
    item: &FmvListItem,
    usd_fmv_buf: &FieldBuffer,
) -> Result<btctax_core::EventPayload, String> {
    if usd_fmv_buf.is_empty() {
        return Err("usd-fmv is required".to_string());
    }
    let trimmed = usd_fmv_buf.buf.trim();
    let usd_fmv = Usd::from_str(trimmed).map_err(|_| format!("bad USD {trimmed:?}"))?;
    Ok(btctax_core::EventPayload::ManualFmv(ManualFmv {
        event: item.event.clone(),
        usd_fmv,
    }))
}

/// Cycle through IncomeKind with an extra None state (None = keep original).
///
/// None → Mining → Staking → Interest → Airdrop → Reward → None.
pub fn cycle_income_kind_optional(kind: Option<IncomeKind>) -> Option<IncomeKind> {
    match kind {
        None => Some(IncomeKind::Mining),
        Some(IncomeKind::Mining) => Some(IncomeKind::Staking),
        Some(IncomeKind::Staking) => Some(IncomeKind::Interest),
        Some(IncomeKind::Interest) => Some(IncomeKind::Airdrop),
        Some(IncomeKind::Airdrop) => Some(IncomeKind::Reward),
        Some(IncomeKind::Reward) => None,
    }
}

/// Cycle through the business 3-state (None → true → false → None).
pub fn cycle_business_optional(b: Option<bool>) -> Option<bool> {
    match b {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
}

// ── Void flow types ───────────────────────────────────────────────────────────

/// Pre-computed display data for a void-decision list row.
#[derive(Clone)]
pub struct VoidListItem {
    /// The decision EventId being offered for void.
    pub event_id: EventId,
    /// Sequence number (for display in the Seq column).
    pub seq: u64,
    /// Static payload type tag ("TransferLink", "ClassifyInbound", etc.).
    pub payload_tag: &'static str,
    /// Human-readable target summary computed at list-open time.
    pub target_summary: String,
    /// The decision's OWN inner target event (used by `derive_void_status`'s
    /// returned-blocker check). None for MethodElection and SafeHarborAllocation.
    pub inner_target: Option<EventId>,
}

/// Step in the void flow. Only `List` — Enter goes DIRECTLY to `VoidModalState`.
pub enum VoidStep {
    List,
}

/// Full state for the void flow. Owns its target list [R0-I2 discipline].
pub struct VoidFlowState {
    /// Owned target list — no standalone list field on EditorApp.
    pub list: TargetList<VoidListItem>,
    pub step: VoidStep,
}

/// Payload for the void confirmation modal.
pub struct VoidModalState {
    /// The decision EventId being voided.
    pub target_event_id: EventId,
    pub seq: u64,
    pub payload_tag: &'static str,
    pub target_summary: String,
    /// Carried from VoidListItem for derive_void_status [M5].
    pub inner_target: Option<EventId>,
    /// True when the target is SafeHarborAllocation — triggers the conditional warning.
    pub is_safe_harbor: bool,
}

/// Return `true` when `payload` is a revocable decision type.
///
/// Revocable: TransferLink, ReclassifyOutflow, ClassifyInbound, ManualFmv, ClassifyRaw,
/// MethodElection, LotSelection, ReclassifyIncome, SafeHarborAllocation.
/// Non-revocable (excluded from void list): SupersedeImport, RejectImport, VoidDecisionEvent,
/// and imported event payloads (Acquire, Income, Dispose, TransferOut, TransferIn, Unclassified,
/// ImportConflict — these carry Import EventIds, not Decision EventIds, so they cannot appear in
/// the void list; the check on Decision-id'd events guards the decision payload variants only).
pub fn is_revocable_payload(payload: &btctax_core::EventPayload) -> bool {
    use btctax_core::EventPayload;
    matches!(
        payload,
        EventPayload::TransferLink(_)
            | EventPayload::ReclassifyOutflow(_)
            | EventPayload::ClassifyInbound(_)
            | EventPayload::ManualFmv(_)
            | EventPayload::ClassifyRaw(_)
            | EventPayload::MethodElection(_)
            | EventPayload::LotSelection(_)
            | EventPayload::ReclassifyIncome(_)
            | EventPayload::SafeHarborAllocation(_)
    )
}

// ── Select-lots flow types ────────────────────────────────────────────────────

/// Display kind for a disposal-list row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisposalKind {
    Sell,
    Spend,
    Gift,
    Donate,
    /// #1: a TransferOut projected to `Op::SelfTransfer` by a non-voided TransferLink.
    /// Reconstructed in-TUI (not in `snap.state.disposals`/`removals`); `principal_sat` =
    /// `TransferOut.sat` (fee excluded — matches `honoring_principal`, resolve.rs:211-214).
    SelfTransfer,
}

/// Pre-computed display data for a select-lots disposal list row.
///
/// `wallet` is ALWAYS sourced from the raw `LedgerEvent.wallet` via `events_by_id` [R0-I1].
/// `RemovalLeg` has no wallet field — Gift/Donate rows would have no wallet source otherwise.
#[derive(Clone)]
pub struct DisposalListItem {
    pub disposal_event: EventId,
    pub date: TaxDate,
    pub kind: DisposalKind,
    /// Principal sat = Σ legs.sat (from Disposal or Removal).
    pub principal_sat: Sat,
    /// From the raw `LedgerEvent.wallet` via `events_by_id`.
    pub wallet: Option<WalletId>,
}

/// A single lot-pick row in the LotsForm.
pub struct LotPickFormRow {
    pub lot_id: LotId,
    pub remaining_sat: Sat,
    pub acquired_at: TaxDate,
    pub usd_basis: Usd,
    /// Editable sat amount (digits only; initially empty = 0).
    pub pick_sat_buf: FieldBuffer,
}

impl LotPickFormRow {
    /// Parse pick_sat_buf as i64; returns 0 when buffer is empty.
    pub fn pick_sat(&self) -> Result<Sat, String> {
        if self.pick_sat_buf.is_empty() {
            return Ok(0);
        }
        self.pick_sat_buf.buf.trim().parse::<i64>().map_err(|e| {
            format!(
                "bad sat in row {}: {e}",
                self.lot_id.origin_event_id.canonical()
            )
        })
    }
}

/// Step in the select-lots flow.
pub enum SelectLotsStep {
    List,
    LotsForm {
        item: DisposalListItem,
        rows: Vec<LotPickFormRow>,
        /// Focused row index.
        cursor: usize,
        error: Option<String>,
    },
}

/// Full state for the select-lots flow.
pub struct SelectLotsFlowState {
    /// Owned by the flow [R0-I2 discipline].
    pub list: TargetList<DisposalListItem>,
    pub step: SelectLotsStep,
}

/// Payload for the select-lots confirmation modal.
pub struct SelectLotsModalState {
    pub disposal_event: EventId,
    pub disposal_date: TaxDate,
    pub disposal_kind: DisposalKind,
    pub principal_sat: Sat,
    /// Validated picks (non-zero only).
    pub picks: Vec<btctax_core::LotPick>,
    pub pick_count: usize,
    /// Σ picks.sat (== principal_sat by construction).
    pub total_sat: Sat,
}

/// Validate the LotsForm at Enter-press.
///
/// Returns `EventPayload::LotSelection(…)` or an error string.
///
/// Validation rules (spec D1):
/// 1. Parse every `pick_sat_buf` → error on any non-integer.
/// 2. Collect rows with `pick_sat > 0` → `Vec<LotPick>`.
/// 3. If no picks (all zero) → `"pick at least one lot"`.
/// 4. `Σ picked_sat` must equal `item.principal_sat`.
pub fn validate_select_lots(
    item: &DisposalListItem,
    rows: &[LotPickFormRow],
) -> Result<btctax_core::EventPayload, String> {
    // Step 1: parse every buffer.
    let mut total: Sat = 0;
    let mut picks: Vec<btctax_core::LotPick> = Vec::new();
    for row in rows {
        let sat = row.pick_sat()?;
        if sat > 0 {
            total += sat;
            picks.push(btctax_core::LotPick {
                lot: row.lot_id.clone(),
                sat,
            });
        }
    }

    // Step 3: at least one pick required.
    if picks.is_empty() {
        return Err("pick at least one lot".to_string());
    }

    // Step 4: principal conservation.
    if total != item.principal_sat {
        return Err(format!(
            "picked {total} sat != disposal principal {} sat; adjust to match exactly",
            item.principal_sat
        ));
    }

    Ok(btctax_core::EventPayload::LotSelection(
        btctax_core::event::LotSelection {
            disposal_event: item.disposal_event.clone(),
            lots: picks,
        },
    ))
}

// ── Set-donation-details flow types ──────────────────────────────────────────

/// Pre-computed display data for a set-donation-details list row.
#[derive(Clone)]
pub struct DonationListItem {
    pub event_id: EventId,
    pub date: TaxDate,
    /// Σ removal.legs.iter().map(|l| l.sat).sum().
    pub total_sat: Sat,
    /// From `Removal.donee` (free-form label, if any).
    pub donee: Option<String>,
    /// From `snap.donation_details.get(&event_id).cloned()` [R0-I3].
    pub existing_details: Option<DonationDetails>,
}

impl DonationListItem {
    /// Return the completeness string for the Completeness column.
    pub fn completeness_str(&self) -> &'static str {
        match &self.existing_details {
            None => "(none)",
            Some(d) if d.is_review_complete(Form8283Section::B) => "B-complete",
            Some(_) => "present",
        }
    }
}

/// Step in the set-donation-details flow.
// FieldForm has 10 FieldBuffer fields by design; boxing the variant is not worth the
// refactor cost given TUI-only usage (stack frames are short-lived).
#[allow(clippy::large_enum_variant)]
pub enum SetDonationDetailsStep {
    List,
    FieldForm {
        item: DonationListItem,
        donee_name_buf: FieldBuffer,
        donee_address_buf: FieldBuffer,
        donee_ein_buf: FieldBuffer,
        appraiser_name_buf: FieldBuffer,
        appraiser_address_buf: FieldBuffer,
        appraiser_tin_buf: FieldBuffer,
        appraiser_ptin_buf: FieldBuffer,
        appraiser_qualifications_buf: FieldBuffer,
        appraisal_date_buf: FieldBuffer,
        fmv_method_override_buf: FieldBuffer,
        /// 0..=9 focus index.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the set-donation-details flow.
pub struct SetDonationDetailsFlowState {
    pub list: TargetList<DonationListItem>,
    pub step: SetDonationDetailsStep,
}

/// Payload for the set-donation-details confirmation modal.
pub struct SetDonationDetailsModalState {
    pub event_id: EventId,
    pub event_date: TaxDate,
    pub total_sat: Sat,
    /// The VALIDATED details payload.
    pub details: DonationDetails,
}

/// Labels for the 10 donation-details fields (focus index 0..=9).
pub const DONATION_FIELD_LABELS: [&str; 10] = [
    "donee_name (REQUIRED)",
    "donee_address",
    "donee_ein",
    "appraiser_name (REQUIRED)",
    "appraiser_address",
    "appraiser_tin",
    "appraiser_ptin",
    "appraiser_qualifications",
    "appraisal_date (YYYY-MM-DD)",
    "fmv_method_override",
];

/// Validate the donation-details FieldForm at Enter-press.
///
/// Returns `DonationDetails` or an error string.
///
/// Validation rules (spec D2):
/// 1. `donee_name`: REQUIRED (empty → error).
/// 2. `appraiser_name`: REQUIRED (empty → error).
/// 3. `appraisal_date`: if non-empty → `parse_date_arg(trim)` (YYYY-MM-DD → error on bad format).
/// 4. All other optionals: empty → `None`.
#[allow(clippy::too_many_arguments)]
pub fn validate_donation_details(
    donee_name_buf: &FieldBuffer,
    donee_address_buf: &FieldBuffer,
    donee_ein_buf: &FieldBuffer,
    appraiser_name_buf: &FieldBuffer,
    appraiser_address_buf: &FieldBuffer,
    appraiser_tin_buf: &FieldBuffer,
    appraiser_ptin_buf: &FieldBuffer,
    appraiser_qualifications_buf: &FieldBuffer,
    appraisal_date_buf: &FieldBuffer,
    fmv_method_override_buf: &FieldBuffer,
) -> Result<DonationDetails, String> {
    // donee_name: REQUIRED
    if donee_name_buf.is_empty() {
        return Err("donee-name is required".to_string());
    }
    let donee_name = donee_name_buf.buf.trim().to_owned();

    // appraiser_name: REQUIRED
    if appraiser_name_buf.is_empty() {
        return Err("appraiser-name is required".to_string());
    }
    let appraiser_name = appraiser_name_buf.buf.trim().to_owned();

    // Optional string fields.
    let opt_str = |buf: &FieldBuffer| -> Option<String> {
        if buf.is_empty() {
            None
        } else {
            let t = buf.buf.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_owned())
            }
        }
    };

    // appraisal_date: optional; if non-empty must parse as YYYY-MM-DD.
    let appraisal_date = if appraisal_date_buf.is_empty() {
        None
    } else {
        let t = appraisal_date_buf.buf.trim();
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        Some(time::Date::parse(t, fmt).map_err(|e| format!("bad date {t:?}: {e}"))?)
    };

    Ok(DonationDetails {
        donee_name,
        donee_address: opt_str(donee_address_buf),
        donee_ein: opt_str(donee_ein_buf),
        appraiser_name,
        appraiser_address: opt_str(appraiser_address_buf),
        appraiser_tin: opt_str(appraiser_tin_buf),
        appraiser_ptin: opt_str(appraiser_ptin_buf),
        appraiser_qualifications: opt_str(appraiser_qualifications_buf),
        appraisal_date,
        fmv_method_override: opt_str(fmv_method_override_buf),
    })
}

// ── Safe-harbor-attest flow types ─────────────────────────────────────────────

/// Step in the safe-harbor-attest flow.
///
/// TypedWord is the FINAL gate — no separate modal exists for this flow [R0-M4].
// TypedWord carries FieldBuffer which is not large; no boxing needed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum SafeHarborAttestStep {
    /// Step 1: displays allocation details + IRREVOCABLE warnings.
    Info,
    /// Step 2: user must type "ATTEST" (all-caps) to confirm.
    TypedWord {
        buf: FieldBuffer,
        error: Option<String>,
    },
}

/// Full state for the safe-harbor-attest flow.
///
/// No separate modal field on EditorApp — TypedWord is the gate [R0-M4].
pub struct SafeHarborAttestFlowState {
    /// EventId of the live (non-voided, timebarred) SafeHarborAllocation being re-attested.
    pub prior_id: btctax_core::EventId,
    /// The allocation payload (cloned from the pre-flight load — for display and re-append).
    pub prior_alloc: btctax_core::event::SafeHarborAllocation,
    pub step: SafeHarborAttestStep,
}

// ── Link-transfer flow types (chunk 4a, D1) ──────────────────────────────────

/// Human-readable one-line label for a `WalletId` (used in the modal target line
/// and the wallet pick-list). Mirrors the `provider/account` render convention.
pub fn wallet_label(w: &WalletId) -> String {
    match w {
        WalletId::Exchange { provider, account } => format!("{provider}/{account}"),
        WalletId::SelfCustody { label } => format!("self:{label}"),
    }
}

/// Pre-computed display data for a link-transfer step-1 (out-list) row.
///
/// Sourced from `snap.state.pending_reconciliation` (the same inherently-post-filtered
/// source reclassify-outflow uses): exactly the unlinked, unreconciled TransferOuts.
#[derive(Clone)]
pub struct TransferOutItem {
    /// The `PendingTransfer.event` EventId (the raw TransferOut).
    pub transfer_out_event: EventId,
    /// Calendar date (tax timezone) of the TransferOut event.
    pub date: TaxDate,
    /// Principal sat from `PendingTransfer.principal_sat`.
    pub principal_sat: Sat,
    /// Wallet of the TransferOut event (SOURCE wallet).
    pub wallet: Option<WalletId>,
}

/// Pre-computed display data for a link-transfer step-2 in-event (InEvent mode) row.
///
/// Only `TransferIn` events whose raw `LedgerEvent.wallet.is_some()` (the engine requires a
/// resolvable destination wallet) AND not already targeted by a non-voided `TransferLink::InEvent`.
#[derive(Clone)]
pub struct InEventItem {
    /// The TransferIn event id.
    pub in_event: EventId,
    /// Calendar date (tax timezone) of the TransferIn event.
    pub date: TaxDate,
    /// Principal sat from the TransferIn payload.
    pub sat: Sat,
    /// Destination wallet (guaranteed `Some` by the pre-filter).
    pub wallet: WalletId,
}

/// Pre-computed display data for a link-transfer step-2 wallet (Wallet mode) row.
///
/// ALL distinct `snap.events[].wallet` Some-values [R0-I2] — NOT just `holdings_by_wallet`
/// keys (which would hide a zero-balance destination wallet, the primary Wallet-target use case).
#[derive(Clone)]
pub struct WalletItem {
    pub wallet: WalletId,
}

/// Which target mode the step-2 picker is showing (Tab cycles InEvent ⇄ Wallet).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LinkMode {
    InEvent,
    Wallet,
}

/// Step in the link-transfer flow.
pub enum LinkTransferStep {
    OutList,
    TargetPick {
        out: TransferOutItem,
        mode: LinkMode,
    },
}

/// Full state for the link-transfer flow. Owns all three target lists [R0-I2 discipline].
pub struct LinkTransferFlowState {
    /// Step-1 list: pending TransferOuts.
    pub out_list: TargetList<TransferOutItem>,
    pub step: LinkTransferStep,
    /// Step-2 InEvent-mode list (built once at open).
    pub in_list: TargetList<InEventItem>,
    /// Step-2 Wallet-mode list (built once at open).
    pub wallet_list: TargetList<WalletItem>,
}

/// Payload for the link-transfer confirmation modal.
pub struct LinkTransferModalState {
    pub out_event: EventId,
    pub out_date: TaxDate,
    pub out_sat: Sat,
    /// The VALIDATED target — what will be persisted (InEvent(id) or Wallet(w)).
    pub target: TransferTarget,
    /// Human-readable target label (shown in the modal).
    pub target_label: String,
}

// ── Classify-raw flow types (chunk 4a, D2) ───────────────────────────────────

/// Pre-computed display data for a classify-raw list row.
///
/// Events carrying `BlockerKind::Unclassified` whose payload is `EventPayload::Unclassified`,
/// minus those already targeted by a non-voided `ClassifyRaw`.
#[derive(Clone)]
pub struct RawListItem {
    /// The Unclassified event id (the ClassifyRaw target — its EventId is preserved).
    pub target: EventId,
    /// Calendar date (tax timezone) of the raw event.
    pub date: TaxDate,
    /// The raw import text (`Unclassified.raw`).
    pub raw: String,
    /// Wallet of the raw event (kept by the classified effective payload).
    pub wallet: Option<WalletId>,
}

/// Which imported variant the classify-raw picker is building (Tab cycles).
///
/// **Scoped to Acquire + Income this cycle** — the two that dominate raw-row classification.
/// Dispose/TransferOut/TransferIn/Unclassified are a CLI-only parity FOLLOWUP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifyRawVariant {
    Acquire,
    Income,
}

/// Cycle the classify-raw variant (Acquire ⇄ Income).
pub fn cycle_classify_raw_variant(v: ClassifyRawVariant) -> ClassifyRawVariant {
    match v {
        ClassifyRawVariant::Acquire => ClassifyRawVariant::Income,
        ClassifyRawVariant::Income => ClassifyRawVariant::Acquire,
    }
}

/// Cycle through the 8 `BasisSource` variants in declaration order (event.rs:16-26).
pub fn cycle_basis_source(bs: BasisSource) -> BasisSource {
    match bs {
        BasisSource::ExchangeProvided => BasisSource::ComputedFromCost,
        BasisSource::ComputedFromCost => BasisSource::FmvAtIncome,
        BasisSource::FmvAtIncome => BasisSource::CarriedFromTransfer,
        BasisSource::CarriedFromTransfer => BasisSource::GiftCarryover,
        BasisSource::GiftCarryover => BasisSource::GiftFmvFallback,
        BasisSource::GiftFmvFallback => BasisSource::SafeHarborAllocated,
        BasisSource::SafeHarborAllocated => BasisSource::ReconstructedPerWallet,
        BasisSource::ReconstructedPerWallet => BasisSource::ExchangeProvided,
    }
}

/// Lowercase display tag for a `BasisSource`.
pub fn basis_source_display(bs: BasisSource) -> &'static str {
    match bs {
        BasisSource::ExchangeProvided => "exchange-provided",
        BasisSource::ComputedFromCost => "computed-from-cost",
        BasisSource::FmvAtIncome => "fmv-at-income",
        BasisSource::CarriedFromTransfer => "carried-from-transfer",
        BasisSource::GiftCarryover => "gift-carryover",
        BasisSource::GiftFmvFallback => "gift-fmv-fallback",
        BasisSource::SafeHarborAllocated => "safe-harbor-allocated",
        BasisSource::ReconstructedPerWallet => "reconstructed-per-wallet",
    }
}

/// Step in the classify-raw flow.
// AcquireForm / IncomeForm carry several FieldBuffers; boxing is not worth it (TUI-only).
#[allow(clippy::large_enum_variant)]
pub enum ClassifyRawStep {
    List,
    VariantPicker {
        item: RawListItem,
        variant: ClassifyRawVariant,
    },
    AcquireForm {
        item: RawListItem,
        sat_buf: FieldBuffer,
        usd_cost_buf: FieldBuffer,
        fee_buf: FieldBuffer,
        /// A required BasisSource PICK (default ExchangeProvided); Tab cycles on the picker row.
        basis_source: BasisSource,
        /// 0=sat, 1=usd_cost, 2=fee, 3=basis_source.
        focus: usize,
        error: Option<String>,
    },
    IncomeForm {
        item: RawListItem,
        sat_buf: FieldBuffer,
        /// Optional; typed → fmv_status=ManualEntry, empty → None + Missing.
        fmv_buf: FieldBuffer,
        kind: IncomeKind,
        business: bool,
        /// 0=sat, 1=usd_fmv, 2=kind, 3=business.
        focus: usize,
        error: Option<String>,
    },
}

/// Full state for the classify-raw flow. Owns its target list.
pub struct ClassifyRawFlowState {
    pub list: TargetList<RawListItem>,
    pub step: ClassifyRawStep,
}

/// Payload for the classify-raw confirmation modal.
pub struct ClassifyRawModalState {
    /// The Unclassified target event id.
    pub target: EventId,
    /// The raw import text (shown in the modal).
    pub raw: String,
    /// The BUILT imported payload (Acquire/Income) — boxed into `ClassifyRaw.as_` at persist.
    pub built: btctax_core::EventPayload,
}

/// Parse a REQUIRED sat field: empty (len==0) → "name is required"; else parse i64.
fn parse_required_sat(buf: &FieldBuffer, name: &str) -> Result<Sat, String> {
    if buf.is_empty() {
        return Err(format!("{name} is required"));
    }
    let t = buf.buf.trim();
    t.parse::<i64>().map_err(|_| format!("bad sat {t:?}"))
}

/// Validate the classify-raw Acquire form → `EventPayload::Acquire(…)` built DIRECTLY (NOT via
/// `InboundClass` [R0-I1]).
///
/// `sat`/`usd_cost` REQUIRED; `fee_usd` optional → $0; `basis_source` is the required PICK.
/// NO acquired-at field — the effective event keeps the TARGET's timestamp (resolve.rs) [R0-I1].
pub fn validate_classify_raw_acquire(
    sat_buf: &FieldBuffer,
    usd_cost_buf: &FieldBuffer,
    fee_buf: &FieldBuffer,
    basis_source: BasisSource,
) -> Result<btctax_core::EventPayload, String> {
    let sat = parse_required_sat(sat_buf, "sat")?;
    if usd_cost_buf.is_empty() {
        return Err("usd-cost is required".to_string());
    }
    let uc = usd_cost_buf.buf.trim();
    let usd_cost = Usd::from_str(uc).map_err(|_| format!("bad USD {uc:?}"))?;
    let fee_usd = if fee_buf.is_empty() {
        Usd::ZERO
    } else {
        let t = fee_buf.buf.trim();
        Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?
    };
    Ok(btctax_core::EventPayload::Acquire(
        btctax_core::event::Acquire {
            sat,
            usd_cost,
            fee_usd,
            basis_source,
        },
    ))
}

/// Validate the classify-raw Income form → `EventPayload::Income(…)` built DIRECTLY (NOT via
/// `InboundClass` [R0-I1]).
///
/// `sat` REQUIRED; `usd_fmv` optional: typed → `Some` + `fmv_status=ManualEntry`, empty → `None` +
/// `fmv_status=Missing` (resolve.rs discards `usd_fmv` when `Missing`; the empty case fires a
/// `FmvMissing` blocker surfaced by status arm 3). `kind` PICK; `business` toggle.
pub fn validate_classify_raw_income(
    sat_buf: &FieldBuffer,
    fmv_buf: &FieldBuffer,
    kind: IncomeKind,
    business: bool,
) -> Result<btctax_core::EventPayload, String> {
    let sat = parse_required_sat(sat_buf, "sat")?;
    let (usd_fmv, fmv_status) = if fmv_buf.is_empty() {
        (None, FmvStatus::Missing)
    } else {
        let t = fmv_buf.buf.trim();
        let v = Usd::from_str(t).map_err(|_| format!("bad USD {t:?}"))?;
        (Some(v), FmvStatus::ManualEntry)
    };
    Ok(btctax_core::EventPayload::Income(
        btctax_core::event::Income {
            sat,
            usd_fmv,
            fmv_status,
            kind,
            business,
        },
    ))
}

// ── Resolve-conflict flow types (chunk 4b, D3) ───────────────────────────────

/// Pre-computed display data for a resolve-conflict list row.
///
/// Sourced from events carrying `BlockerKind::ImportConflict` — the blocker's `.event` is the
/// `ImportConflict` event id (`conflict_event`), whose payload names the `target` import event and
/// the `new_payload` proposed to supersede it. The two summaries are computed at open time (against
/// the CURRENT `target` payload vs the conflict's `new_payload`), so the flow/modal render no live
/// event lookups.
#[derive(Clone)]
pub struct ConflictItem {
    /// The `ImportConflict` event id — the resolution target (`SupersedeImport`/`RejectImport`
    /// carry this as `conflict_event`).
    pub conflict_event: EventId,
    /// The TARGET import event id whose payload the conflict proposes to supersede.
    pub target: EventId,
    /// Calendar date (tax tz) of the conflict event.
    pub date: TaxDate,
    /// Short `new_fingerprint` string (table column).
    pub new_fingerprint: String,
    /// One-line summary of the TARGET's CURRENT payload (kept on reject; replaced on accept).
    pub current_summary: String,
    /// One-line summary of the conflict's NEW payload (adopted on accept).
    pub new_summary: String,
}

/// Accept vs Reject branch of the resolve-conflict flow (an in-flow toggle — NOT Browse `a`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveKind {
    /// `SupersedeImport` — adopt `new_payload` onto the target id.
    Accept,
    /// `RejectImport` — keep the original; discard the new payload.
    Reject,
}

/// Step in the resolve-conflict flow.
pub enum ResolveConflictStep {
    /// Step 1: pick a conflict.
    List,
    /// Step 2: accept/reject choice for the picked conflict (`←/→` or `h/l` toggles).
    Choose {
        conflict: ConflictItem,
        kind: ResolveKind,
    },
}

/// Full state for the resolve-conflict flow. Owns its list [R0-I2 discipline].
pub struct ResolveConflictFlowState {
    pub list: TargetList<ConflictItem>,
    pub step: ResolveConflictStep,
}

/// Payload for the resolve-conflict confirmation modal.
///
/// Shows BOTH sides (the target's CURRENT payload vs the conflict's NEW payload) and the
/// NON-REVOCABLE warning. `SupersedeImport`/`RejectImport` are excluded from `is_revocable_payload`;
/// a later void fires `DecisionConflict`, so the decision cannot be undone in-editor.
pub struct ResolveConflictModalState {
    pub conflict_event: EventId,
    pub target: EventId,
    pub kind: ResolveKind,
    pub old_summary: String,
    pub new_summary: String,
}

// ── Optimize-accept flow types (chunk 4b, D4) ────────────────────────────────

/// Pre-computed display data for one optimize-accept candidate disposal (a per-disposal proposal row
/// that survived the pre-filter). NO per-disposal Δtax [R0-I1]: `DisposalProposal` carries no
/// per-disposal delta, and the year-level `OptimizeProposal.delta` is shown at the flow level only.
#[derive(Clone)]
pub struct OptimizeCandidateItem {
    /// The disposal EventId the proposed selection targets.
    pub disposal: EventId,
    /// Wallet of the disposal (table column).
    pub wallet: WalletId,
    /// Calendar date (tax tz) of the disposal.
    pub date: TaxDate,
    /// §C.2 gate verdict — drives the step-2 branch (`ContemporaneousNow` → modal;
    /// `NeedsAttestation` → attestation-text step). `ForbiddenBroker2027` is pre-filtered out.
    pub persistable: Persistability,
    /// The optimizer's tax-minimizing pick (the `LotSelection.lots` to persist).
    pub picks: Vec<LotPick>,
}

/// Step in the optimize-accept flow.
// AttestText carries a FieldBuffer + a cloned item; boxing is not worth it (TUI-only).
#[allow(clippy::large_enum_variant)]
pub enum OptimizeAcceptStep {
    /// Step 1: pick a proposed disposal.
    List,
    /// Step 2 (NeedsAttestation only): type the contemporaneous-ID attestation (non-empty required).
    AttestText {
        item: OptimizeCandidateItem,
        buf: FieldBuffer,
        error: Option<String>,
    },
}

/// Full state for the optimize-accept flow. Owns its candidate list [R0-I2 discipline].
pub struct OptimizeAcceptFlowState {
    pub list: TargetList<OptimizeCandidateItem>,
    pub step: OptimizeAcceptStep,
    /// Whole-year `optimized − baseline` figure (≤ 0) — the flow-level banner [R0-I1].
    pub delta: Usd,
    /// `true` ⇔ the proposal is only APPROXIMATE (banner caveat).
    pub approximate: bool,
}

/// Payload for the optimize-accept confirmation modal.
pub struct OptimizeAcceptModalState {
    pub disposal: EventId,
    /// The proposed `LotSelection.lots`.
    pub picks: Vec<LotPick>,
    pub pick_count: usize,
    /// Σ picks.sat.
    pub total_sat: Sat,
    /// `None` → `Contemporaneous` (basis label; no attest row); `Some(text)` → `AttestedRecording`
    /// (the attestation text co-persisted alongside the LotSelection).
    pub attestation: Option<String>,
    /// §A.5 basis label (`"Contemporaneous"` / `"AttestedRecording"`).
    pub basis_label: &'static str,
}

// ── Safe-harbor-allocate flow types (chunk 5, D2) ────────────────────────────

/// Pre-computed display row for one pre-2025 residue `AllocLot` in the allocate Preview table.
/// Carries the AllocLot's display-relevant fields; the residue is method-INDEPENDENT (G3), so these
/// rows are computed ONCE at open and never recomputed when the `method` toggle changes.
#[derive(Clone)]
pub struct AllocLotRow {
    pub wallet: WalletId,
    pub sat: Sat,
    pub usd_basis: Usd,
    pub acquired_at: TaxDate,
    /// §1015(a) LOSS basis (FMV-at-gift); `Some` only for a dual-basis gift lot.
    pub dual_loss_basis: Option<Usd>,
    /// §1223(2) donor acquisition date (tacking); `Some` only for a gift lot.
    pub donor_acquired_at: Option<TaxDate>,
}

/// Step in the safe-harbor-allocate flow. There is exactly ONE step (the Preview); the confirmation
/// is a SEPARATE modal (`SafeHarborAllocateModalState`) — creation is REVOCABLE, so NO typed-word gate
/// (contrast the attest flow, whose TypedWord step IS the gate).
pub enum SafeHarborAllocateStep {
    Preview,
}

/// Full state for the safe-harbor-allocate flow (chunk 5, D2).
///
/// The ONLY user input is `method` (an `AllocMethod` toggle). `lots`/`total_sat`/`total_basis` are the
/// residue computed ONCE at open via `Session::safe_harbor_residue`; the `method` toggle changes ONLY
/// the recorded tag, never the displayed lots (G3). `pre2025_method` is the method the residue was
/// computed under, threaded UNCHANGED through flow→modal→persist (G5 immutability).
pub struct SafeHarborAllocateFlowState {
    pub lots: Vec<AllocLot>,
    pub total_sat: Sat,
    pub total_basis: Usd,
    pub method: AllocMethod,
    pub pre2025_method: LotMethod,
    pub list: TargetList<AllocLotRow>,
    pub step: SafeHarborAllocateStep,
}

/// Payload for the safe-harbor-allocate confirmation modal (revocable framing; NOT typed-word).
pub struct SafeHarborAllocateModalState {
    pub lots: Vec<AllocLot>,
    pub total_sat: Sat,
    pub total_basis: Usd,
    pub method: AllocMethod,
    pub pre2025_method: LotMethod,
    pub lot_count: usize,
}

/// The §A.5 basis label for a `Persistability`. `ForbiddenBroker2027` is never persisted (pre-filtered),
/// so it maps to a placeholder that is unreachable at the modal.
pub fn optimize_basis_label(p: Persistability) -> &'static str {
    match p {
        Persistability::ContemporaneousNow => "Contemporaneous",
        Persistability::NeedsAttestation => "AttestedRecording",
        Persistability::ForbiddenBroker2027 => "Forbidden",
    }
}

/// Pre-filter the optimizer proposal's per-disposal rows into persistable candidates (chunk 4b, D4).
///
/// Keeps a row iff ALL hold:
/// - `proposed_selection != current_selection` (a no-change row is the CLI "already optimal" skip);
/// - `persistable != ForbiddenBroker2027` (2027+ broker lots NEVER persist — no attestation cures);
/// - the disposal has NO live (non-voided) `LotSelection` (`already_selected`) — else the append is a
///   DUPLICATE ⇒ `DecisionConflict` NEITHER-applies (the MANDATORY duplicate guard).
pub fn filter_optimize_candidates(
    per_disposal: &[DisposalProposal],
    already_selected: &std::collections::BTreeSet<EventId>,
) -> Vec<OptimizeCandidateItem> {
    per_disposal
        .iter()
        .filter(|d| d.proposed_selection != d.current_selection)
        .filter(|d| d.persistable != Persistability::ForbiddenBroker2027)
        .filter(|d| !already_selected.contains(&d.disposal))
        .map(|d| OptimizeCandidateItem {
            disposal: d.disposal.clone(),
            wallet: d.wallet.clone(),
            date: d.date,
            persistable: d.persistable,
            picks: d.proposed_selection.clone(),
        })
        .collect()
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

    // ── Helper: build a minimal OutflowListItem for validation tests ──────────

    fn dummy_outflow_item() -> OutflowListItem {
        use btctax_core::{
            identity::{Source, SourceRef},
            EventId,
        };
        use time::{OffsetDateTime, UtcOffset};
        OutflowListItem {
            transfer_out_event: EventId::import(Source::River, SourceRef::new("test-ro-1")),
            date: btctax_core::conventions::tax_date(
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
            ),
            principal_sat: 1_000_000,
            wallet: None,
        }
    }

    // ── KAT-V-RO-1: amount empty → "amount is required" ─────────────────────

    #[test]
    fn kat_v_ro_1_amount_empty_is_required_error() {
        let item = dummy_outflow_item();
        let err = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &FieldBuffer::new(),
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("amount") && err.contains("required"),
            "empty amount must produce 'required' error; got: {err}"
        );
    }

    // ── KAT-V-RO-2: amount valid decimal → parses correctly ──────────────────

    #[test]
    fn kat_v_ro_2_amount_valid_decimal_parses() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert_eq!(ro.principal_proceeds_or_fmv, dec!(640.00));
        assert!(matches!(
            ro.as_,
            OutflowClass::Dispose {
                kind: DisposeKind::Sell
            }
        ));
        assert!(ro.fee_usd.is_none());
        assert!(ro.donee.is_none());
    }

    // ── KAT-V-RO-3: amount whitespace-only → parse error (not "required") ────

    #[test]
    fn kat_v_ro_3_amount_whitespace_only_is_parse_error_not_required() {
        let item = dummy_outflow_item();
        let mut buf = FieldBuffer::new();
        buf.set("   "); // whitespace-only: is_empty()==false [R0-M4]
        let err = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only amount must be a parse error; got: {err}"
        );
        assert!(
            !err.contains("required"),
            "whitespace-only amount must NOT say 'required'; got: {err}"
        );
    }

    // ── KAT-V-RO-4: fee empty → None (no error) ──────────────────────────────

    #[test]
    fn kat_v_ro_4_fee_empty_gives_none() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(ro.fee_usd.is_none(), "empty fee must produce None");
    }

    // ── KAT-V-RO-5: fee valid → parses correctly ─────────────────────────────

    #[test]
    fn kat_v_ro_5_fee_valid_parses() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");
        let mut fee_buf = FieldBuffer::new();
        fee_buf.set("2.50");
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Sell,
            &amount_buf,
            &fee_buf,
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert_eq!(ro.fee_usd, Some(dec!(2.50)));
    }

    // ── KAT-V-RO-6: appraisal toggle default false; Space toggles ────────────

    #[test]
    fn kat_v_ro_6_appraisal_toggle_default_false_then_true() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");

        // Default: appraisal=false
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Donate,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(matches!(
            ro.as_,
            OutflowClass::Donate {
                appraisal_required: false
            }
        ));

        // Toggled: appraisal=true
        let ro2 = validate_reclassify_outflow(
            &item,
            OutflowKind::Donate,
            &amount_buf,
            &FieldBuffer::new(),
            true,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(matches!(
            ro2.as_,
            OutflowClass::Donate {
                appraisal_required: true
            }
        ));
    }

    // ── KAT-V-RO-7: donee empty → None; non-empty → Some(trimmed) ────────────

    #[test]
    fn kat_v_ro_7_donee_empty_is_none_nonempty_is_some_trimmed() {
        let item = dummy_outflow_item();
        let mut amount_buf = FieldBuffer::new();
        amount_buf.set("640.00");

        // Empty → None
        let ro = validate_reclassify_outflow(
            &item,
            OutflowKind::Gift,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &FieldBuffer::new(),
        )
        .unwrap();
        assert!(ro.donee.is_none(), "empty donee must produce None");

        // Non-empty (with leading/trailing spaces) → Some(trimmed)
        let mut donee_buf = FieldBuffer::new();
        donee_buf.set("  Alice  ");
        let ro2 = validate_reclassify_outflow(
            &item,
            OutflowKind::Gift,
            &amount_buf,
            &FieldBuffer::new(),
            false,
            &donee_buf,
        )
        .unwrap();
        assert_eq!(
            ro2.donee,
            Some("Alice".to_string()),
            "non-empty donee must be Some(trimmed)"
        );
    }

    // ── KAT-V-RO-8: OutflowKind Tab cycles: sell → spend → gift → donate → sell ─

    #[test]
    fn kat_v_ro_8_outflow_kind_cycles_four_variants_wraps_to_sell() {
        let mut kind = OutflowKind::Sell; // initial
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Spend);
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Gift);
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Donate);
        kind = cycle_outflow_kind(kind);
        assert_eq!(kind, OutflowKind::Sell, "4 cycles must wrap to Sell");
    }

    // ── Helper: build a minimal IncomeListItem for RI validation tests ────────

    fn dummy_income_item() -> IncomeListItem {
        use btctax_core::{
            identity::{Source, SourceRef},
            EventId,
        };
        use time::{OffsetDateTime, UtcOffset};
        IncomeListItem {
            income_event: EventId::import(Source::River, SourceRef::new("test-ri-1")),
            date: btctax_core::conventions::tax_date(
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
            ),
            sat: 100_000,
            kind: IncomeKind::Reward,
            business: false,
            fmv: None,
            wallet: None,
        }
    }

    // ── Helper: build a minimal FmvListItem for FMV validation tests ──────────

    fn dummy_fmv_item() -> FmvListItem {
        use btctax_core::{
            identity::{Source, SourceRef},
            EventId,
        };
        use time::{OffsetDateTime, UtcOffset};
        FmvListItem {
            event: EventId::import(Source::River, SourceRef::new("test-fmv-1")),
            date: btctax_core::conventions::tax_date(
                OffsetDateTime::from_unix_timestamp(1_748_000_000).unwrap(),
                UtcOffset::UTC,
            ),
            sat: 100_000,
            kind: IncomeKind::Staking,
            wallet: None,
        }
    }

    // ── KAT-RI-REQUIRED-BUSINESS: business=None → Err "business is required" ─

    #[test]
    fn kat_ri_required_business_none_is_error() {
        let item = dummy_income_item();
        let err = validate_reclassify_income(&item, None, None).unwrap_err();
        assert!(
            err.contains("business is required"),
            "None business must produce 'business is required' error; got: {err}"
        );
    }

    // ── KAT-V-RI-1: business=None → blocked ─────────────────────────────────

    #[test]
    fn kat_v_ri_1_business_none_is_blocked() {
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, None, None);
        assert!(result.is_err(), "None business must be blocked");
        let err = result.unwrap_err();
        assert!(
            err.contains("required"),
            "error must say 'required'; got: {err}"
        );
    }

    // ── KAT-V-RI-2: business=Some(true), kind=None → valid ──────────────────

    #[test]
    fn kat_v_ri_2_business_true_kind_none_is_valid() {
        use btctax_core::EventPayload;
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, Some(true), None);
        let payload = result.unwrap();
        if let EventPayload::ReclassifyIncome(ri) = payload {
            assert!(ri.business, "business must be true");
            assert!(ri.kind.is_none(), "kind must be None (keep original)");
        } else {
            panic!("expected ReclassifyIncome");
        }
    }

    // ── KAT-V-RI-3: business=Some(false), kind=Some(Mining) → valid ──────────

    #[test]
    fn kat_v_ri_3_business_false_kind_mining_is_valid() {
        use btctax_core::EventPayload;
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, Some(false), Some(IncomeKind::Mining));
        let payload = result.unwrap();
        if let EventPayload::ReclassifyIncome(ri) = payload {
            assert!(!ri.business, "business must be false");
            assert_eq!(ri.kind, Some(IncomeKind::Mining));
        } else {
            panic!("expected ReclassifyIncome");
        }
    }

    // ── KAT-V-RI-4: business=Some(true), kind=Some(Reward) → valid ──────────

    #[test]
    fn kat_v_ri_4_business_true_kind_reward_is_valid() {
        use btctax_core::EventPayload;
        let item = dummy_income_item();
        let result = validate_reclassify_income(&item, Some(true), Some(IncomeKind::Reward));
        let payload = result.unwrap();
        if let EventPayload::ReclassifyIncome(ri) = payload {
            assert!(ri.business);
            assert_eq!(ri.kind, Some(IncomeKind::Reward));
        } else {
            panic!("expected ReclassifyIncome");
        }
    }

    // ── KAT-V-FMV-1: empty buf → Err "usd-fmv is required" ──────────────────

    #[test]
    fn kat_v_fmv_1_empty_buf_is_required_error() {
        let item = dummy_fmv_item();
        let err = validate_set_fmv(&item, &FieldBuffer::new()).unwrap_err();
        assert!(
            err.contains("usd-fmv") && err.contains("required"),
            "empty buf must produce 'usd-fmv is required' error; got: {err}"
        );
    }

    // ── KAT-V-FMV-2: "45.00" → valid, usd_fmv=45.00 ─────────────────────────

    #[test]
    fn kat_v_fmv_2_valid_decimal_parses() {
        use btctax_core::EventPayload;
        use rust_decimal_macros::dec;
        let item = dummy_fmv_item();
        let mut buf = FieldBuffer::new();
        buf.set("45.00");
        let payload = validate_set_fmv(&item, &buf).unwrap();
        if let EventPayload::ManualFmv(mf) = payload {
            assert_eq!(mf.usd_fmv, dec!(45.00));
        } else {
            panic!("expected ManualFmv");
        }
    }

    // ── KAT-V-FMV-3: whitespace-only → parse error, NOT "required" [R0-M4] ───

    #[test]
    fn kat_v_fmv_3_whitespace_only_is_parse_error_not_required() {
        let item = dummy_fmv_item();
        let mut buf = FieldBuffer::new();
        buf.set("   "); // whitespace-only: is_empty()==false [R0-M4]
        let err = validate_set_fmv(&item, &buf).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "whitespace-only usd-fmv must be a parse error; got: {err}"
        );
        assert!(
            !err.contains("required"),
            "whitespace-only usd-fmv must NOT say 'required'; got: {err}"
        );
    }

    // ── Extra: non-numeric usd-fmv → parse error ─────────────────────────────

    #[test]
    fn set_fmv_non_numeric_is_parse_error() {
        let item = dummy_fmv_item();
        let mut buf = FieldBuffer::new();
        buf.set("abc");
        let err = validate_set_fmv(&item, &buf).unwrap_err();
        assert!(
            err.contains("bad USD"),
            "non-numeric must produce 'bad USD' error; got: {err}"
        );
    }

    // ── Cycle pins: business 3-state and optional-kind 6-state ───────────────
    //
    // KAT-RI-REQUIRED-BUSINESS cycle pins (spec D5): Tab cycles
    // None → Some(true) → Some(false) → None.

    #[test]
    fn kat_ri_business_optional_cycles_three_states_and_wraps() {
        let mut b: Option<bool> = None; // initial: not chosen (REQUIRED-EXPLICIT)
        b = cycle_business_optional(b);
        assert_eq!(b, Some(true));
        b = cycle_business_optional(b);
        assert_eq!(b, Some(false));
        b = cycle_business_optional(b);
        assert_eq!(b, None, "3 cycles must wrap back to None");
        b = cycle_business_optional(b);
        assert_eq!(b, Some(true), "4th cycle lands on Some(true) again");
    }

    // Optional-kind picker: None → Mining → Staking → Interest → Airdrop →
    // Reward → None (spec D1: None = keep original).

    #[test]
    fn kat_ri_kind_optional_cycles_six_states_and_wraps() {
        let mut k: Option<IncomeKind> = None; // initial: keep original
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Mining));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Staking));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Interest));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Airdrop));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, Some(IncomeKind::Reward));
        k = cycle_income_kind_optional(k);
        assert_eq!(k, None, "6 cycles must wrap back to None (keep original)");
    }

    // ── KAT-V-RO-9: amount label is "gross proceeds" for sell/spend; "FMV" for gift/donate ─

    #[test]
    fn kat_v_ro_9_amount_label_per_kind() {
        assert!(
            amount_label(OutflowKind::Sell).contains("gross proceeds"),
            "sell must have 'gross proceeds' label; got: {}",
            amount_label(OutflowKind::Sell)
        );
        assert!(
            amount_label(OutflowKind::Spend).contains("gross proceeds"),
            "spend must have 'gross proceeds' label [R0-I3]; got: {}",
            amount_label(OutflowKind::Spend)
        );
        assert!(
            amount_label(OutflowKind::Gift).contains("FMV"),
            "gift must have 'FMV' label; got: {}",
            amount_label(OutflowKind::Gift)
        );
        assert!(
            amount_label(OutflowKind::Donate).contains("FMV"),
            "donate must have 'FMV' label; got: {}",
            amount_label(OutflowKind::Donate)
        );
    }

    // ── KAT-V-SL-1..3 — select-lots validation ───────────────────────────────

    fn dummy_disposal_item(principal_sat: Sat) -> DisposalListItem {
        DisposalListItem {
            disposal_event: btctax_core::EventId::Decision { seq: 1 },
            date: time::macros::date!(2025 - 09 - 15),
            kind: DisposalKind::Sell,
            principal_sat,
            wallet: None,
        }
    }

    fn dummy_lot_row(lot_sat: Sat, pick: &str) -> LotPickFormRow {
        let mut buf = FieldBuffer::new();
        buf.set(pick);
        LotPickFormRow {
            lot_id: btctax_core::LotId {
                origin_event_id: btctax_core::EventId::Decision { seq: 99 },
                split_sequence: 0,
            },
            remaining_sat: lot_sat,
            acquired_at: time::macros::date!(2024 - 01 - 01),
            usd_basis: rust_decimal_macros::dec!(30000),
            pick_sat_buf: buf,
        }
    }

    /// KAT-V-SL-1: all picks zero → "pick at least one lot".
    #[test]
    fn kat_v_sl_1_all_picks_zero_is_required_error() {
        let item = dummy_disposal_item(100_000);
        let rows = vec![
            dummy_lot_row(60_000, ""),  // empty = 0
            dummy_lot_row(40_000, "0"), // explicit zero
        ];
        let err = validate_select_lots(&item, &rows).unwrap_err();
        assert!(
            err.contains("pick at least one lot"),
            "all-zero picks must produce 'pick at least one lot'; got: {err}"
        );
    }

    /// KAT-V-SL-2: Σ picked_sat < principal_sat → error.
    #[test]
    fn kat_v_sl_2_underpick_is_principal_mismatch_error() {
        let item = dummy_disposal_item(100_000);
        let rows = vec![dummy_lot_row(100_000, "50000")]; // 50k < 100k
        let err = validate_select_lots(&item, &rows).unwrap_err();
        assert!(
            err.contains("50000 sat") && err.contains("100000 sat"),
            "underpick must mention totals; got: {err}"
        );
        assert!(
            !err.contains("pick at least"),
            "error must NOT say 'pick at least one lot' for underpick; got: {err}"
        );
    }

    /// KAT-V-SL-3: Σ picked_sat == principal_sat → valid; builds correct LotPick list.
    #[test]
    fn kat_v_sl_3_exact_pick_is_valid_builds_lot_pick_list() {
        let item = dummy_disposal_item(100_000);
        let lot_id = btctax_core::LotId {
            origin_event_id: btctax_core::EventId::Decision { seq: 42 },
            split_sequence: 1,
        };
        let mut buf = FieldBuffer::new();
        buf.set("100000");
        let rows = vec![LotPickFormRow {
            lot_id: lot_id.clone(),
            remaining_sat: 100_000,
            acquired_at: time::macros::date!(2024 - 06 - 01),
            usd_basis: rust_decimal_macros::dec!(50000),
            pick_sat_buf: buf,
        }];
        let payload = validate_select_lots(&item, &rows).unwrap();
        match payload {
            btctax_core::EventPayload::LotSelection(ls) => {
                assert_eq!(ls.disposal_event, item.disposal_event);
                assert_eq!(ls.lots.len(), 1);
                assert_eq!(ls.lots[0].lot, lot_id);
                assert_eq!(ls.lots[0].sat, 100_000);
            }
            other => panic!("expected LotSelection, got {other:?}"),
        }
    }

    // ── KAT-V-DD-1..3 — set-donation-details validation ──────────────────────

    fn empty_bufs() -> [FieldBuffer; 10] {
        std::array::from_fn(|_| FieldBuffer::new())
    }

    fn call_validate_dd(bufs: &[FieldBuffer; 10]) -> Result<DonationDetails, String> {
        validate_donation_details(
            &bufs[0], &bufs[1], &bufs[2], &bufs[3], &bufs[4], &bufs[5], &bufs[6], &bufs[7],
            &bufs[8], &bufs[9],
        )
    }

    /// KAT-V-DD-1: donee_name empty → "donee-name is required".
    #[test]
    fn kat_v_dd_1_donee_name_empty_is_required_error() {
        let bufs = empty_bufs();
        let err = call_validate_dd(&bufs).unwrap_err();
        assert!(
            err.contains("donee-name") && err.contains("required"),
            "empty donee_name must say 'donee-name is required'; got: {err}"
        );
    }

    /// KAT-V-DD-2: appraiser_name empty → "appraiser-name is required".
    #[test]
    fn kat_v_dd_2_appraiser_name_empty_is_required_error() {
        let mut bufs = empty_bufs();
        bufs[0].set("Community Foundation"); // fill donee_name
        let err = call_validate_dd(&bufs).unwrap_err();
        assert!(
            err.contains("appraiser-name") && err.contains("required"),
            "empty appraiser_name must say 'appraiser-name is required'; got: {err}"
        );
    }

    /// KAT-V-DD-3: appraisal_date non-empty with bad format → parse error.
    #[test]
    fn kat_v_dd_3_bad_appraisal_date_format_is_error() {
        let mut bufs = empty_bufs();
        bufs[0].set("Community Foundation");
        bufs[3].set("Jane Appraiser");
        bufs[8].set("not-a-date"); // appraisal_date = bad format
        let err = call_validate_dd(&bufs).unwrap_err();
        assert!(
            err.contains("bad date"),
            "bad appraisal_date must produce 'bad date' error; got: {err}"
        );
    }

    // KAT-V-DD-4 (pre-population round-trip) is implemented in `main.rs` as
    // `kat_v_dd_4_pre_population_drives_real_path`. The prior version here
    // re-implemented the production List→FieldForm pre-population mapping IN the test
    // body — round-1 whole-branch review [I1] found it to be coverage theatre (dropping
    // a production optional-field pre-population passed uncaught). The real-path version
    // drives `d` → List → Enter → FieldForm so a wiring regression in any of the 10
    // fields fails, then Enter → modal to assert the validator round-trip.

    // ── KAT-OA-FILTER — optimize-accept pre-filter (chunk 4b, D4) ────────────
    //
    // filter_optimize_candidates keeps ONLY rows where proposed != current AND persistable !=
    // ForbiddenBroker2027 AND the disposal has NO live LotSelection. Constructs `DisposalProposal`s
    // by hand (all pub) to pin ForbiddenBroker2027-excluded + live-LotSelection-excluded + no-change
    // excluded, mirroring the spec's optimize-accept KAT bullets.
    #[test]
    fn kat_oa_filter_excludes_nochange_forbidden_and_already_selected() {
        use btctax_core::identity::{LotId, Source, SourceRef};
        use btctax_core::project::ComplianceStatus;
        use btctax_core::{DisposalProposal, EventId, LotPick, Persistability, WalletId};
        use time::macros::date;

        let wallet = WalletId::Exchange {
            provider: "River".into(),
            account: "main".into(),
        };
        let d = |tag: &str| EventId::import(Source::River, SourceRef::new(tag));
        let pick = |tag: &str, sat: i64| LotPick {
            lot: LotId {
                origin_event_id: d(tag),
                split_sequence: 0,
            },
            sat,
        };
        let row = |disp: EventId, cur: Vec<LotPick>, prop: Vec<LotPick>, p: Persistability| {
            DisposalProposal {
                disposal: disp,
                wallet: wallet.clone(),
                date: date!(2025 - 05 - 23),
                current_selection: cur,
                proposed_selection: prop,
                status: ComplianceStatus::NonCompliant,
                persistable: p,
            }
        };

        let a = d("oa-A"); // ContemporaneousNow, changed → KEPT
        let b = d("oa-B"); // no-change → EXCLUDED
        let c = d("oa-C"); // ForbiddenBroker2027, changed → EXCLUDED
        let sel = d("oa-D"); // NeedsAttestation, changed, already-selected → EXCLUDED
        let e = d("oa-E"); // NeedsAttestation, changed → KEPT

        let per_disposal = vec![
            row(
                a.clone(),
                vec![pick("lotA1", 100)],
                vec![pick("lotA2", 100)],
                Persistability::ContemporaneousNow,
            ),
            row(
                b.clone(),
                vec![pick("lotB", 100)],
                vec![pick("lotB", 100)],
                Persistability::ContemporaneousNow,
            ),
            row(
                c.clone(),
                vec![pick("lotC1", 100)],
                vec![pick("lotC2", 100)],
                Persistability::ForbiddenBroker2027,
            ),
            row(
                sel.clone(),
                vec![pick("lotD1", 100)],
                vec![pick("lotD2", 100)],
                Persistability::NeedsAttestation,
            ),
            row(
                e.clone(),
                vec![pick("lotE1", 100)],
                vec![pick("lotE2", 100)],
                Persistability::NeedsAttestation,
            ),
        ];

        let already_selected: std::collections::BTreeSet<EventId> = [sel.clone()].into();
        let kept = filter_optimize_candidates(&per_disposal, &already_selected);
        let kept_ids: Vec<EventId> = kept.iter().map(|k| k.disposal.clone()).collect();

        assert!(kept_ids.contains(&a), "ContemporaneousNow changed row kept");
        assert!(kept_ids.contains(&e), "NeedsAttestation changed row kept");
        assert!(!kept_ids.contains(&b), "no-change row excluded");
        assert!(!kept_ids.contains(&c), "ForbiddenBroker2027 excluded");
        assert!(
            !kept_ids.contains(&sel),
            "already-selected (live LotSelection) excluded"
        );
        assert_eq!(kept.len(), 2, "exactly A and E survive");

        // The kept ContemporaneousNow row carries its proposed picks + verdict.
        let a_item = kept.iter().find(|k| k.disposal == a).unwrap();
        assert_eq!(a_item.persistable, Persistability::ContemporaneousNow);
        assert_eq!(a_item.picks, vec![pick("lotA2", 100)]);
        assert_eq!(optimize_basis_label(a_item.persistable), "Contemporaneous");
        let e_item = kept.iter().find(|k| k.disposal == e).unwrap();
        assert_eq!(
            optimize_basis_label(e_item.persistable),
            "AttestedRecording"
        );
    }
}
