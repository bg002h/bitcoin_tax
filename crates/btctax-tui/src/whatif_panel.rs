//! The read-only WHAT-IF planner panel (task #43, phase P3) — a viewer overlay opened with `w`.
//!
//! Type a hypothetical sell amount → a live `SellReport` (marginal federal tax + §1212 carryforward +
//! §1(h) bracket + NIIT); or pick a harvest target → a `HarvestReport`. Everything routes through the
//! ALREADY-VERIFIED `btctax_core::whatif::{sell,harvest}` (clone-fold-discard) — this module adds NO
//! tax logic and NEVER writes the vault. The panel merely reads the read-only [`Snapshot`], parses the
//! text inputs, calls the pure core, and formats the result (reusing `btctax_cli::render`).
//!
//! # Read-only contract
//! The panel calls ONLY the non-persisting core + reads `snap`. It NEVER touches the vault, `Session`,
//! or any writer. This module lives OUTSIDE `export.rs` so the mechanized source-gate KAT-E10 scans it
//! for free (it must contain no write-class tokens). See [`WhatIfPanel::compute`].

use crate::app::Snapshot;
use btctax_core::whatif::{self, HarvestRequest, HarvestTarget, SellRequest};
use btctax_core::{
    Carryforward, EvaluateError, FilingStatus, TaxDate, TaxProfile, Usd, WalletId, WhatIfError,
};
use std::str::FromStr;
use time::{Date, Month, OffsetDateTime, UtcOffset};

/// Which hypothetical the panel is currently posing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhatIfMode {
    /// A single hypothetical sale of a fixed BTC amount.
    Sell,
    /// A max-N harvest under a target constraint (bracket ceiling / gain cap / tax cap).
    Harvest,
}

/// Which text/picker sub-field currently has focus (receives keystrokes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The as-of `TaxDate` (`YYYY-MM-DD`) — FMV + ST/LT boundary + the tax year key on this.
    At,
    /// Sell-mode: the BTC amount to sell (a decimal, parsed to satoshis).
    Amount,
    /// Harvest-mode: the target selector (`zero-ltcg | fifteen-ltcg | gain=$X | tax=$X`).
    Target,
    /// The wallet picker (cycled with ←/→) over the pool's distinct wallets.
    Wallet,
    /// The optional per-BTC price override (empty ⇒ the bundled dataset daily-close FMV for `at`).
    Price,
}

/// Live state for the what-if planner overlay. Owned by `App.whatif` (`Some` while open).
///
/// All fields are UI state; compute is EXPLICIT (Enter) — never per-keystroke, because harvest is a
/// multi-fold segment walk (`[R0-M2]`). `output`/`error`/`caveat` hold the last computed result.
#[derive(Debug, Clone)]
pub struct WhatIfPanel {
    /// Sell ⇄ Harvest (toggled with Tab; `s` selects Sell, `h` selects Harvest).
    pub mode: WhatIfMode,
    /// The editable as-of date buffer (`YYYY-MM-DD`). Defaults to today when `selected_year` is the
    /// current year, else the last day of `selected_year` (`[R0-I3]`).
    pub at_buf: String,
    /// Sell-mode BTC amount buffer (a decimal; parsed to sats, over-precision rejected).
    pub amount_buf: String,
    /// Harvest-mode target buffer (`zero-ltcg | fifteen-ltcg | gain=$X | tax=$X`).
    pub target_buf: String,
    /// Optional per-BTC price buffer (empty ⇒ dataset FMV).
    pub price_buf: String,
    /// The pool's distinct wallets (the picker options), sorted for determinism.
    pub wallets: Vec<WalletId>,
    /// The currently-selected wallet index into `wallets` (cycled with ←/→).
    pub wallet_idx: usize,
    /// Which sub-field has focus.
    pub focus: Focus,
    /// The rendered report (set on Enter when the core returns `Ok`).
    pub output: Option<String>,
    /// The refusal / parse-error line (set on Enter when the core returns `Err`, or a bad input).
    pub error: Option<String>,
    /// The placeholder-profile caveat (set when `selected_year` has no stored `TaxProfile`).
    pub caveat: Option<String>,
}

impl WhatIfPanel {
    /// Build a fresh panel from the read-only snapshot. Collects the wallet picker from the projected
    /// lots and derives the default as-of date from `selected_year` + `now` (`[R0-I3]`).
    pub fn new(snap: &Snapshot, selected_year: i32, now: OffsetDateTime) -> Self {
        let today = btctax_core::conventions::tax_date(now, UtcOffset::UTC);
        // Default `at`: today when planning the CURRENT year, else the last day of the selected year.
        let default_at: TaxDate = if selected_year == today.year() {
            today
        } else {
            Date::from_calendar_date(selected_year, Month::December, 31).unwrap_or(today)
        };

        // Distinct wallets present in the projected pool (the picker options).
        let mut wallets: Vec<WalletId> = snap.state.lots.iter().map(|l| l.wallet.clone()).collect();
        wallets.sort();
        wallets.dedup();

        WhatIfPanel {
            mode: WhatIfMode::Sell,
            at_buf: fmt_date(default_at),
            amount_buf: String::new(),
            target_buf: "zero-ltcg".to_string(),
            price_buf: String::new(),
            wallets,
            wallet_idx: 0,
            focus: Focus::At,
            output: None,
            error: None,
            caveat: None,
        }
    }

    /// The focus-order for the current mode (drives `focus_next`/`focus_prev`).
    fn field_order(&self) -> [Focus; 4] {
        match self.mode {
            WhatIfMode::Sell => [Focus::At, Focus::Amount, Focus::Wallet, Focus::Price],
            WhatIfMode::Harvest => [Focus::At, Focus::Target, Focus::Wallet, Focus::Price],
        }
    }

    /// Advance focus to the next sub-field (wrapping).
    pub fn focus_next(&mut self) {
        let order = self.field_order();
        let i = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = order[(i + 1) % order.len()];
    }

    /// Move focus to the previous sub-field (wrapping).
    pub fn focus_prev(&mut self) {
        let order = self.field_order();
        let i = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = order[(i + order.len() - 1) % order.len()];
    }

    /// Toggle Sell ⇄ Harvest, resetting focus and clearing the stale result.
    pub fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            WhatIfMode::Sell => WhatIfMode::Harvest,
            WhatIfMode::Harvest => WhatIfMode::Sell,
        });
    }

    /// Set the mode explicitly (`s`→Sell, `h`→Harvest), resetting focus + clearing the stale result.
    pub fn set_mode(&mut self, mode: WhatIfMode) {
        if self.mode != mode {
            self.mode = mode;
            self.focus = Focus::At;
            self.output = None;
            self.error = None;
            self.caveat = None;
        }
    }

    /// Cycle the wallet picker one entry forward (no-op with no wallets or when not on the picker).
    pub fn wallet_next(&mut self) {
        if self.focus == Focus::Wallet && !self.wallets.is_empty() {
            self.wallet_idx = (self.wallet_idx + 1) % self.wallets.len();
        }
    }

    /// Cycle the wallet picker one entry back (no-op with no wallets or when not on the picker).
    pub fn wallet_prev(&mut self) {
        if self.focus == Focus::Wallet && !self.wallets.is_empty() {
            self.wallet_idx = (self.wallet_idx + self.wallets.len() - 1) % self.wallets.len();
        }
    }

    /// Append a character to the focused TEXT field (the wallet picker ignores chars).
    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Focus::At => self.at_buf.push(c),
            Focus::Amount => self.amount_buf.push(c),
            Focus::Target => self.target_buf.push(c),
            Focus::Price => self.price_buf.push(c),
            Focus::Wallet => {} // picker only — chars ignored
        }
    }

    /// Remove the last character from the focused TEXT field.
    pub fn backspace(&mut self) {
        match self.focus {
            Focus::At => {
                self.at_buf.pop();
            }
            Focus::Amount => {
                self.amount_buf.pop();
            }
            Focus::Target => {
                self.target_buf.pop();
            }
            Focus::Price => {
                self.price_buf.pop();
            }
            Focus::Wallet => {}
        }
    }

    /// The currently-selected wallet, if any.
    fn selected_wallet(&self) -> Option<WalletId> {
        self.wallets.get(self.wallet_idx).cloned()
    }

    /// Parse the optional price buffer (`None` when empty ⇒ use the dataset FMV).
    fn parse_price(&self) -> Result<Option<Usd>, String> {
        let cleaned = self.price_buf.trim().replace(['$', ','], "");
        if cleaned.is_empty() {
            return Ok(None);
        }
        Usd::from_str(&cleaned)
            .map(Some)
            .map_err(|e| format!("bad price {:?}: expected a USD number: {e}", self.price_buf))
    }

    /// [★ R0-M2] EXPLICIT compute (Enter): parse the inputs, resolve the profile, and call the pure
    /// `whatif::{sell,harvest}` core. Sets `output` on success, `error` on refusal/parse-failure, and
    /// `caveat` when a placeholder profile was substituted. NEVER writes — clone-fold-discard only.
    pub fn compute(&mut self, snap: &Snapshot, selected_year: i32) {
        self.output = None;
        self.error = None;
        self.caveat = None;

        // As-of date.
        let at = match parse_tax_date(&self.at_buf) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        // Wallet (from the picker).
        let Some(wallet) = self.selected_wallet() else {
            self.error =
                Some("no wallet available in this vault to plan against (no lots)".to_string());
            return;
        };
        // Optional price override.
        let price = match self.parse_price() {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };

        // [P2-C1] A full-return year that resolves refused/uncomputable cannot be planned against — show
        // the reason, never a placeholder-based (wrong) number. `snap.profiles` holds the DERIVED profile
        // for a ReturnInputs year, so the plan below matches `report`.
        if let Some(reason) = snap.refused.get(&selected_year) {
            self.error = Some(format!(
                "full-return inputs for {selected_year} are not computable: {reason}"
            ));
            return;
        }
        // Profile: the resolved one for `selected_year` ([M1] `.get`, NEVER `[year]`), else a clearly
        // labeled placeholder (single / $0) that clears `TaxProfileMissing` like the CLI ad-hoc path.
        let profile: Option<TaxProfile> = match snap.profiles.get(&selected_year) {
            Some(p) => Some(p.clone()),
            None => {
                self.caveat = Some(format!(
                    "no stored tax profile for {selected_year} \u{2014} figures assume single / $0 \
                     other income; set one via `tax-profile set`"
                ));
                Some(placeholder_profile())
            }
        };
        let config = snap.cli_config.to_projection();

        match self.mode {
            WhatIfMode::Sell => {
                let sell_sat = match parse_btc_to_sat(&self.amount_buf) {
                    Ok(s) => s,
                    Err(e) => {
                        self.error = Some(e);
                        return;
                    }
                };
                let req = SellRequest {
                    sell_sat,
                    wallet,
                    at,
                    price,
                    method: None,
                };
                match whatif::sell(
                    &snap.events,
                    &snap.prices,
                    &config,
                    profile.as_ref(),
                    &snap.tables,
                    &req,
                ) {
                    Ok(report) => {
                        self.output = Some(btctax_cli::render::render_whatif_sell(&report, false));
                    }
                    Err(e) => self.error = Some(refusal_message(&e)),
                }
            }
            WhatIfMode::Harvest => {
                let target = match parse_harvest_target(&self.target_buf) {
                    Ok(t) => t,
                    Err(e) => {
                        self.error = Some(e);
                        return;
                    }
                };
                let req = HarvestRequest {
                    wallet,
                    at,
                    price,
                    target,
                };
                match whatif::harvest(
                    &snap.events,
                    &snap.prices,
                    &config,
                    profile.as_ref(),
                    &snap.tables,
                    &req,
                ) {
                    Ok(report) => {
                        self.output =
                            Some(btctax_cli::render::render_whatif_harvest(&report, false));
                    }
                    Err(e) => self.error = Some(refusal_message(&e)),
                }
            }
        }
    }

    /// Format the full panel body (inputs + the last computed output/refusal) as plain text for the
    /// `Paragraph` overlay. The focused field is prefixed with `>`. Also used by the render KATs.
    pub fn render_body(&self) -> String {
        let mut s = String::new();
        s.push_str("What-if planner \u{2014} READ-ONLY (the vault is never written)\n");
        let mode = match self.mode {
            WhatIfMode::Sell => "SELL",
            WhatIfMode::Harvest => "HARVEST",
        };
        s.push_str(&format!(
            "Mode: {mode}    [Tab] toggle  \u{00b7}  [s] sell  \u{00b7}  [h] harvest\n\n"
        ));

        let mark = |f: Focus| if self.focus == f { ">" } else { " " };
        s.push_str(&format!(
            "{} As-of date (YYYY-MM-DD): {}\n",
            mark(Focus::At),
            self.at_buf
        ));
        match self.mode {
            WhatIfMode::Sell => s.push_str(&format!(
                "{} Sell amount (BTC): {}\n",
                mark(Focus::Amount),
                self.amount_buf
            )),
            WhatIfMode::Harvest => s.push_str(&format!(
                "{} Target (zero-ltcg | fifteen-ltcg | gain=$X | tax=$X): {}\n",
                mark(Focus::Target),
                self.target_buf
            )),
        }
        let wallet = self
            .selected_wallet()
            .map(|w| btctax_cli::render::wallet_label(&w))
            .unwrap_or_else(|| "(no wallets in this vault)".to_string());
        s.push_str(&format!(
            "{} Wallet [\u{2190}/\u{2192} to change]: {}\n",
            mark(Focus::Wallet),
            wallet
        ));
        let price = if self.price_buf.trim().is_empty() {
            "(bundled dataset FMV for the as-of date)"
        } else {
            self.price_buf.as_str()
        };
        s.push_str(&format!(
            "{} Price USD/BTC (optional): {}\n",
            mark(Focus::Price),
            price
        ));
        s.push_str("\n[Enter] compute   [\u{2191}/\u{2193}] field   [Esc] close\n");

        if let Some(caveat) = &self.caveat {
            s.push_str(&format!("\n\u{26a0} {caveat}\n"));
        }
        s.push('\n');
        if let Some(err) = &self.error {
            s.push_str(&format!("Refused: {err}\n"));
        } else if let Some(out) = &self.output {
            s.push_str(out);
        } else {
            s.push_str("(fill in the inputs above, then press Enter to compute)\n");
        }
        s
    }
}

/// The CLI-layer PLACEHOLDER tax profile — a single filer with $0 income / MAGI / qualified-dividends /
/// carryforward. Mirrors `btctax-cli`'s `placeholder_tax_profile`; substituted (never persisted) when
/// the selected year has no stored profile, to clear `TaxProfileMissing` ONLY.
fn placeholder_profile() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Usd::ZERO,
        magi_excluding_crypto: Usd::ZERO,
        qualified_dividends_and_other_pref_income: Usd::ZERO,
        other_net_capital_gain: Usd::ZERO,
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: Usd::ZERO,
        w2_medicare_wages: Usd::ZERO,
        schedule_c_expenses: Usd::ZERO,
    }
}

/// Format a `TaxDate` as `YYYY-MM-DD` for the editable buffer.
fn fmt_date(d: TaxDate) -> String {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    d.format(&fmt).unwrap_or_else(|_| d.to_string())
}

/// Parse an editable `YYYY-MM-DD` date buffer into a `TaxDate`.
fn parse_tax_date(s: &str) -> Result<TaxDate, String> {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    TaxDate::parse(s.trim(), &fmt).map_err(|e| format!("bad date {s:?}: expected YYYY-MM-DD: {e}"))
}

/// Parse a BTC decimal (e.g. `0.05`) into satoshis, REJECTING over-precision (finer than 1 sat). The
/// TUI amount field means BTC — a bare `1` = 1 BTC — so it delegates to the shared BTC-only core
/// parser [`btctax_core::whatif::parse_btc_amount`] (dedup, task #48). It is NOT pointed at the CLI's
/// smart `parse_sell_arg`, which would read a bare `1` as 1 sat. (Calls `btctax_core`, never `cmd::` —
/// KAT-E10 stays green.)
pub fn parse_btc_to_sat(s: &str) -> Result<i64, String> {
    whatif::parse_btc_amount(s)
}

/// Parse a harvest `--target`: `zero-ltcg | fifteen-ltcg | gain=$X | tax=$X` (X ≥ 0; `$`/commas
/// optional). Delegates to the single source of truth — [`HarvestTarget`]'s `FromStr` in
/// `btctax_core` — so the panel and the CLI parse identically; the core error is rendered to the
/// panel's UI string. (Calls `btctax_core`, never `cmd::`, so the KAT-E10 source-gate stays green.)
fn parse_harvest_target(s: &str) -> Result<HarvestTarget, String> {
    HarvestTarget::from_str(s).map_err(|e| e.to_string())
}

/// Format a `WhatIfError` refusal verbatim for the panel (missing table/profile, pre-2025,
/// ProceedsRequired, NoLots, YearNotComputable). Mirrors `btctax-cli`'s `map_whatif_err` messages.
fn refusal_message(e: &WhatIfError) -> String {
    match e {
        WhatIfError::YearNotComputable(b) => format!(
            "year not computable \u{2014} resolve the blocker or set a tax profile first: [{:?}] {}",
            b.kind, b.detail
        ),
        WhatIfError::PreTransitionYear(y) => {
            format!("{y} is pre-2025: a pre-2025 sale restates a closed year \u{2014} not a plan")
        }
        WhatIfError::NoLots {
            wallet,
            at,
            available,
            requested,
        } => btctax_cli::render::no_lots_message(wallet, *at, *available, *requested),
        WhatIfError::Evaluate(EvaluateError::ProceedsRequired) => {
            "a price is required for a future/off-dataset date with no bundled price \u{2014} \
             enter a USD/BTC price"
                .to_string()
        }
        WhatIfError::Evaluate(ev) => format!("evaluate error: {ev:?}"),
        WhatIfError::InvalidTarget(msg) => msg.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_adapters::{BundledTaxTables, LayeredPrices};
    use btctax_cli::CliConfig;
    use btctax_core::event::{Acquire, BasisSource, EventPayload, LedgerEvent};
    use btctax_core::identity::{EventId, Source, SourceRef, WalletId};
    use btctax_core::project::project;
    use std::collections::BTreeMap;
    use time::macros::datetime;

    fn cold() -> WalletId {
        WalletId::SelfCustody {
            label: "cold".into(),
        }
    }

    /// A synthetic snapshot with ONE 1-BTC lot in `cold` (basis $10,000, acquired 2024-06-01) + a
    /// stored 2025 profile. Uses the REAL bundled prices/tables so the panel path matches production.
    fn snapshot_one_lot(with_profile: bool) -> Snapshot {
        let events = vec![LedgerEvent {
            id: EventId::import(Source::Swan, SourceRef::new("L")),
            utc_timestamp: datetime!(2024-06-01 00:00:00 UTC),
            original_tz: time::macros::offset!(+00:00),
            wallet: Some(cold()),
            payload: EventPayload::Acquire(Acquire {
                sat: 100_000_000,
                usd_cost: Usd::from(10_000i64),
                fee_usd: Usd::ZERO,
                basis_source: BasisSource::ExchangeProvided,
            }),
        }];
        let prices = LayeredPrices::load_with_cache(None).unwrap();
        let cli_config = CliConfig::default();
        let state = project(&events, &prices, &cli_config.to_projection());
        let mut profiles = BTreeMap::new();
        if with_profile {
            profiles.insert(
                2025,
                TaxProfile {
                    filing_status: FilingStatus::Single,
                    ordinary_taxable_income: Usd::from(60_000i64),
                    magi_excluding_crypto: Usd::from(60_000i64),
                    qualified_dividends_and_other_pref_income: Usd::ZERO,
                    other_net_capital_gain: Usd::ZERO,
                    capital_loss_carryforward_in: Carryforward::default(),
                    w2_ss_wages: Usd::ZERO,
                    w2_medicare_wages: Usd::ZERO,
                    schedule_c_expenses: Usd::ZERO,
                },
            );
        }
        Snapshot {
            events,
            state,
            cli_config,
            profiles,
            refused: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices,
        }
    }

    fn empty_snapshot() -> Snapshot {
        Snapshot {
            events: vec![],
            state: btctax_core::state::LedgerState::default(),
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            refused: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: LayeredPrices::load_with_cache(None).unwrap(),
        }
    }

    fn panel_for(snap: &Snapshot, year: i32) -> WhatIfPanel {
        // A fixed clock so the default-at derivation is deterministic.
        WhatIfPanel::new(snap, year, datetime!(2026-07-06 12:00:00 UTC))
    }

    // ── BTC → sat parsing ([★] whatif_panel_btc_input_parses_to_sat) ────────────────

    #[test]
    fn btc_input_parses_to_sat() {
        assert_eq!(parse_btc_to_sat("0.05"), Ok(5_000_000));
        assert_eq!(parse_btc_to_sat("1"), Ok(100_000_000));
        assert_eq!(parse_btc_to_sat("0.00000001"), Ok(1)); // 1 sat exactly
                                                           // Over-precision (finer than 1 sat) is REJECTED, not silently truncated.
        assert!(parse_btc_to_sat("0.000000001").is_err());
        assert!(parse_btc_to_sat("0.123456789").is_err());
        assert!(parse_btc_to_sat("abc").is_err());
        assert!(parse_btc_to_sat("-1").is_err());
    }

    /// [task #48 dedup] The TUI amount field parses BTC via the shared core `parse_btc_amount` — a bare
    /// `1` stays **1 BTC** (100,000,000 sat), NOT 1 sat. The field must NOT be pointed at the CLI's
    /// smart `parse_sell_arg` (which would read `1` as 1 sat). This pins the BTC-only semantics + the
    /// delegation to the shared parser.
    #[test]
    fn tui_amount_field_uses_parse_btc_amount() {
        // Bare `1` = 1 BTC (the TUI meaning), NOT 1 sat.
        assert_eq!(parse_btc_to_sat("1"), Ok(100_000_000));
        // The field delegates to the shared core BTC parser, byte-for-byte.
        for s in ["1", "0.05", "0.00000001", "2.5", "abc", "-1", "0.000000001"] {
            assert_eq!(
                parse_btc_to_sat(s),
                whatif::parse_btc_amount(s),
                "amount field must delegate to core parse_btc_amount: {s:?}"
            );
        }
    }

    #[test]
    fn harvest_target_parses_all_forms() {
        assert_eq!(
            parse_harvest_target("zero-ltcg"),
            Ok(HarvestTarget::ZeroLtcg)
        );
        assert_eq!(
            parse_harvest_target("FIFTEEN-LTCG"),
            Ok(HarvestTarget::FifteenLtcg)
        );
        assert_eq!(
            parse_harvest_target("gain=$25,000"),
            Ok(HarvestTarget::Gain(Usd::from(25_000i64)))
        );
        assert_eq!(
            parse_harvest_target("tax=$0"),
            Ok(HarvestTarget::Tax(Usd::ZERO))
        );
        assert!(parse_harvest_target("nonsense").is_err());
    }

    /// [task #48 dedup] The panel's target parse is a thin wrapper over the shared core
    /// `HarvestTarget: FromStr` — the SAME parser the CLI `--target` uses — so the two never drift.
    /// (The panel calls `btctax_core`, never `cmd::`; KAT-E10 stays green.)
    #[test]
    fn panel_target_parse_shares_fromstr() {
        for s in [
            "zero-ltcg",
            "GAIN=$1,000",
            "tax=$0",
            "gain=-1",
            "nonsense",
            "gain=abc",
        ] {
            assert_eq!(
                parse_harvest_target(s).ok(),
                HarvestTarget::from_str(s).ok(),
                "panel must delegate to the shared FromStr: {s:?}"
            );
        }
    }

    // ── Sell renders the report ([★] whatif_panel_sell_renders_report) ──────────────

    #[test]
    fn sell_renders_report() {
        let snap = snapshot_one_lot(true);
        let mut panel = panel_for(&snap, 2025);
        panel.at_buf = "2025-08-01".to_string();
        panel.amount_buf = "1".to_string();
        panel.price_buf = "30000".to_string();
        panel.compute(&snap, 2025);

        assert!(
            panel.error.is_none(),
            "sell must compute: {:?}",
            panel.error
        );
        let out = panel.output.as_deref().expect("output set");
        assert!(out.contains("marginal federal tax"), "report:\n{out}");
        assert!(out.contains("lots consumed"), "report:\n{out}");
        assert!(out.contains("LTCG bracket"), "report:\n{out}");
        // The full body embeds the report.
        assert!(panel.render_body().contains("marginal federal tax"));
    }

    // ── Harvest renders the report ([★] whatif_panel_harvest_renders_report) ────────

    #[test]
    fn harvest_renders_report() {
        let snap = snapshot_one_lot(true);
        let mut panel = panel_for(&snap, 2025);
        panel.set_mode(WhatIfMode::Harvest);
        panel.at_buf = "2025-08-01".to_string();
        panel.target_buf = "zero-ltcg".to_string();
        panel.price_buf = "30000".to_string();
        panel.compute(&snap, 2025);

        assert!(
            panel.error.is_none(),
            "harvest must compute: {:?}",
            panel.error
        );
        let out = panel.output.as_deref().expect("output set");
        assert!(out.contains("status:"), "report:\n{out}");
        assert!(out.contains("bound by:"), "report:\n{out}");
        assert!(out.contains("Tax decision-support"), "report:\n{out}");
    }

    // ── Refusal renders verbatim ([★] whatif_panel_error_renders_refusal) ───────────

    #[test]
    fn error_renders_refusal_no_lots() {
        // Empty ledger but a wallet in the picker is required to reach the core; inject one directly.
        let snap = empty_snapshot();
        // Give the picker a wallet even though there are no lots (forces the NoLots refusal path).
        let mut panel = panel_for(&snap, 2025);
        panel.wallets = vec![cold()];
        panel.wallet_idx = 0;
        panel.amount_buf = "0.1".to_string();
        panel.at_buf = "2025-08-01".to_string();
        panel.price_buf = "30000".to_string();
        panel.compute(&snap, 2025);
        assert!(panel.output.is_none());
        let err = panel.error.as_deref().expect("error set");
        // UX-P4-9: an empty pool is the honest "no BTC" case, and the panel mirrors the CLI —
        // naming the wallet + as-of date (not the old false "no lots available" wording).
        assert!(
            err.contains("no BTC available in self:cold as of 2025-08-01"),
            "refusal: {err}"
        );
        assert!(!err.contains("no lots available"), "refusal: {err}");
        assert!(panel.render_body().contains("Refused:"));
    }

    #[test]
    fn error_renders_refusal_pre_2025() {
        let snap = snapshot_one_lot(true);
        let mut panel = panel_for(&snap, 2025);
        panel.at_buf = "2024-08-01".to_string(); // pre-transition
        panel.amount_buf = "0.5".to_string();
        panel.price_buf = "30000".to_string();
        panel.compute(&snap, 2025);
        let err = panel.error.as_deref().expect("error set");
        assert!(err.contains("pre-2025"), "refusal: {err}");
    }

    // ── Sell ⇄ Harvest toggle ([★] whatif_panel_toggle_sell_harvest) ────────────────

    #[test]
    fn toggle_sell_harvest() {
        let snap = empty_snapshot();
        let mut panel = panel_for(&snap, 2025);
        assert_eq!(panel.mode, WhatIfMode::Sell);
        panel.toggle_mode();
        assert_eq!(panel.mode, WhatIfMode::Harvest);
        panel.toggle_mode();
        assert_eq!(panel.mode, WhatIfMode::Sell);
        panel.set_mode(WhatIfMode::Harvest);
        assert_eq!(panel.mode, WhatIfMode::Harvest);
        panel.set_mode(WhatIfMode::Sell);
        assert_eq!(panel.mode, WhatIfMode::Sell);
    }

    // ── No-profile placeholder caveat ([M5] whatif_panel_no_profile_shows_placeholder_caveat) ──

    #[test]
    fn no_profile_shows_placeholder_caveat() {
        let snap = snapshot_one_lot(false); // NO stored profile
        let mut panel = panel_for(&snap, 2025);
        panel.at_buf = "2025-08-01".to_string();
        panel.amount_buf = "1".to_string();
        panel.price_buf = "30000".to_string();
        panel.compute(&snap, 2025);
        // The placeholder clears TaxProfileMissing so the sale still computes.
        assert!(
            panel.error.is_none(),
            "placeholder must compute: {:?}",
            panel.error
        );
        let caveat = panel.caveat.as_deref().expect("caveat set");
        assert!(caveat.contains("no stored tax profile"), "caveat: {caveat}");
        assert!(panel.render_body().contains("no stored tax profile"));
    }

    // ── Field focus + wallet picker mechanics ───────────────────────────────────────

    #[test]
    fn focus_cycles_and_char_routes_to_focused_field() {
        let snap = snapshot_one_lot(true);
        let mut panel = panel_for(&snap, 2025);
        assert_eq!(panel.focus, Focus::At);
        panel.focus_next();
        assert_eq!(panel.focus, Focus::Amount);
        panel.push_char('2');
        assert_eq!(panel.amount_buf, "2");
        // Wallet picker is char-immune.
        panel.focus_next();
        assert_eq!(panel.focus, Focus::Wallet);
        panel.push_char('x');
        assert!(panel.wallets.iter().any(|w| *w == cold()));
        // Wrap around.
        panel.focus_next();
        assert_eq!(panel.focus, Focus::Price);
        panel.focus_next();
        assert_eq!(panel.focus, Focus::At);
        panel.focus_prev();
        assert_eq!(panel.focus, Focus::Price);
    }

    #[test]
    fn default_at_uses_selected_year_last_day_when_not_current() {
        let snap = empty_snapshot();
        // selected_year 2025 != clock year 2026 ⇒ default at = 2025-12-31.
        let panel = panel_for(&snap, 2025);
        assert_eq!(panel.at_buf, "2025-12-31");
    }
}
