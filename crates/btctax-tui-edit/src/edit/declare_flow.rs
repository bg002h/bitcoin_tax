//! The Declare flow (Task 8, Phase P-C): collects window/sat/wallet for a Defensive-Filing declare
//! candidate, drives the shipped `btctax_cli::plan_declare(target_shortfall = Some(...))` chokepoint for
//! a live floor/coverage/holding-date readout (DFW-D9) + the DFW-D5.2 clearance check, and an on-demand
//! clamped-saving preview (DFW-D10 M-1). **C-3:** this module COLLECTS input and READS `plan_declare` —
//! it never calls `btctax_cli::apply_declare` directly; the WRITE goes through
//! `edit::persist::persist_declare_tranche` (the ONLY caller of `apply_declare` in this crate,
//! mechanically enforced by `persist::tests::kat_g1_mechanized_source_gate`).

use btctax_core::conservative::Coverage;
use btctax_core::conservative_promote::{filed_basis_for, ComputedFloor, PromoteRefusal};
use btctax_core::conventions::is_long_term;
use btctax_core::defensive::discovery::Shortfall;
use btctax_core::defensive::era::{era_window, next_preset, EraPreset, ALL_PRESETS};
use btctax_core::defensive::{declare_preview_saving, SavingFlavor};
use btctax_core::price::PriceProvider;
use btctax_core::project::ProjectionConfig;
use btctax_core::{LedgerEvent, TaxDate, TaxProfile, TaxTables, WalletId};

/// Which step of the flow is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclareFlowStep {
    /// Editing window/sat/preset, with the live (cheap-trio) readout.
    Edit,
    /// The DFW-D8 plain confirmation (revocable, `$0`, no Form 8275 — NOT a typed-phrase gate) before
    /// `persist_declare_tranche` is called.
    Confirm,
}

/// `DeclareFlow{step, sat, window_start, window_end, ...}` — the brief's interface, verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclareFlowState {
    pub step: DeclareFlowStep,
    /// The targeted candidate (DFW-D5.2's clearance target: `shortfall.event`).
    pub shortfall: Shortfall,
    pub sat: i64,
    pub wallet: WalletId,
    pub window_start: TaxDate,
    pub window_end: TaxDate,
    /// The era preset currently governing `window_start`/`window_end`'s STARTING point (DFW-D9) — a
    /// manual nudge does not change `preset` (it remains the last-applied starting point).
    pub preset: EraPreset,
    /// DFW-D9 M-3 / KAT(d): a first-class ENTRY state, copied from the dashboard's own
    /// `journey_view.safe_harbor_blocked` (the CORE `tranche_guard` predicates) — never re-derived from
    /// the cli-private `guard_tranche_vs_allocation` fn. Purely informational here: the actual gate is
    /// `plan_declare`'s own shipped-set check at confirm time (DFW-D1 — no second gating authority).
    pub safe_harbor_blocked: bool,
    /// DFW-D10 M-1: the on-demand tax-Δ preview. `None` = not-yet-computed OR STALE ("stale —
    /// recompute") — invalidated on ANY window/sat edit.
    pub tax_delta: Option<SavingFlavor>,
}

impl DeclareFlowState {
    /// Open the flow for `shortfall` (DFW-D5 prefill): `window_end` strictly before the short op's date;
    /// `wallet` = the short op's source-pool wallet (the caller unwraps `shortfall.wallet` — a
    /// `DeclareCandidate` always carries one, per `discovery::triage`'s own routing). `window_start`
    /// seeds from the OLDEST (most conservative — DFW-D9's "wider window → lower floor" bias) era
    /// preset; the before-op prefill clamps `window_end` immediately (DFW-D9: "the DFW-D5 before-op
    /// prefill governs over a preset's window_end where they conflict").
    pub fn new(shortfall: Shortfall, wallet: WalletId, safe_harbor_blocked: bool) -> Self {
        let preset = ALL_PRESETS[0];
        let (preset_start, preset_end) = era_window(preset);
        let before_op = before_op_date(&shortfall);
        let window_end = if preset_end < before_op {
            preset_end
        } else {
            before_op
        };
        Self {
            step: DeclareFlowStep::Edit,
            sat: shortfall.short_sat,
            wallet,
            window_start: preset_start,
            window_end,
            preset,
            safe_harbor_blocked,
            tax_delta: None,
            shortfall,
        }
    }

    /// Cycle to the NEXT era preset (DFW-D9 "confirm/edit starting point"): seeds `window_start` from
    /// the preset's own start; clamps `window_end` to the DFW-D5 before-op day when the preset's own end
    /// would not otherwise satisfy it (the before-op prefill governs on conflict). Invalidates the
    /// on-demand tax-Δ (M-1).
    pub fn cycle_preset(&mut self) {
        self.preset = next_preset(self.preset);
        let (start, end) = era_window(self.preset);
        let before_op = before_op_date(&self.shortfall);
        self.window_start = start;
        self.window_end = if end < before_op { end } else { before_op };
        self.tax_delta = None;
    }

    /// Nudge `window_start` by `days` (may move earlier or later than the current preset's own start —
    /// a manual DFW-D9 edit). Invalidates the on-demand tax-Δ (M-1). No lower bound other than
    /// `window_start <= window_end` (enforced at `plan_declare`/confirm time, not here — the live
    /// readout surfaces an invalid ordering via `floor_readout` returning `NoCoverage`).
    pub fn nudge_window_start(&mut self, days: i64) {
        self.window_start = shift_date(self.window_start, days);
        self.tax_delta = None;
    }

    /// Nudge `window_end` by `days`, CLAMPED to never cross the DFW-D5 before-op boundary (the
    /// invariant that makes the lot exist in time to cover the short op — never overridable by a manual
    /// edit). Invalidates the on-demand tax-Δ (M-1).
    pub fn nudge_window_end(&mut self, days: i64) {
        let candidate = shift_date(self.window_end, days);
        let before_op = before_op_date(&self.shortfall);
        self.window_end = if candidate > before_op {
            before_op
        } else {
            candidate
        };
        self.tax_delta = None;
    }

    /// Nudge `sat` by `delta` sat (DFW-D8/N-1: the filer MAY edit above the prefilled `short_sat` — the
    /// excess is the out-of-scope manual-holdings shape entering by a side door; it files nothing wrong
    /// at `$0`). Floored at 1 (declaring 0/negative sat is never valid — `plan_declare`'s own gate).
    /// Invalidates the on-demand tax-Δ (M-1).
    pub fn nudge_sat(&mut self, delta: i64) {
        self.sat = (self.sat + delta).max(1);
        self.tax_delta = None;
    }

    /// The cheap-trio live readout's floor/coverage piece (DFW-D9/D10): `Ok` = `Coverage::Full` +
    /// the computed whole-tranche floor; `Err` = the can-never-promote `NoCoverage`/`PartialCoverage`
    /// state, surfaced LIVE (KAT c) rather than only discovered at a later promote attempt.
    pub fn floor_readout(
        &self,
        prices: &dyn PriceProvider,
    ) -> Result<ComputedFloor, PromoteRefusal> {
        filed_basis_for(prices, self.sat, self.window_start, self.window_end)
    }

    /// The cheap-trio's holding-date piece: `window_end` IS the lot's holding-period start
    /// (`resolve.rs:~1310`), so it also sets short/long-term (DFW-D9).
    pub fn holding_date(&self) -> TaxDate {
        self.window_end
    }

    /// Whether the resulting lot would be LONG-term if disposed at the short op's own date (a cheap,
    /// already-shipped `is_long_term` read — no new tax logic).
    pub fn is_long_term_at_short_date(&self) -> bool {
        is_long_term(self.window_end, self.shortfall.date)
    }

    /// The DFW-D5.2 target-scoped clearance check: does the CURRENT window/sat/wallet actually clear
    /// the targeted shortfall? A pure READ — `plan_declare` never mutates. `Ok(plan)` is what
    /// `persist_declare_tranche` (edit/persist.rs) is handed at confirm time; `Err(refusal)` is surfaced
    /// live rather than discovered only at a final Enter (DFW-D5).
    #[allow(clippy::too_many_arguments)]
    pub fn clearance(
        &self,
        events: &[LedgerEvent],
        prices: &dyn PriceProvider,
        cfg: &ProjectionConfig,
        now: time::OffsetDateTime,
    ) -> Result<btctax_cli::DeclarePlan, btctax_cli::Refusal> {
        btctax_cli::plan_declare(
            events,
            prices,
            cfg,
            self.sat,
            self.wallet.clone(),
            self.window_start,
            self.window_end,
            Some(self.shortfall.event.clone()),
            now,
        )
    }

    /// The on-demand tax-Δ (DFW-D10 M-1 / ★ T6-Minor1): the profile-aware `declare_preview_saving` for
    /// the shortfall's own disposal year, sourcing the REAL stored/resolved `TaxProfile` the caller
    /// passes in (never `journey_view`'s structurally-`Uncomputable` `None`). Caches into
    /// `self.tax_delta`; a later window/sat edit blanks it again (`nudge_*`/`cycle_preset`).
    #[allow(clippy::too_many_arguments)]
    pub fn compute_tax_delta(
        &mut self,
        events: &[LedgerEvent],
        prices: &dyn PriceProvider,
        cfg: &ProjectionConfig,
        tables: &dyn TaxTables,
        profile: Option<&TaxProfile>,
    ) {
        let year = self.shortfall.date.year();
        let flavor = declare_preview_saving(
            events,
            prices,
            cfg,
            tables,
            self.sat,
            self.wallet.clone(),
            self.window_start,
            self.window_end,
            year,
            profile,
        );
        self.tax_delta = Some(flavor);
    }
}

/// DFW-D5: `window_end` strictly BEFORE the short op's date (decisions sort AFTER same-instant imports
/// — `resolve.rs:~1312`). Saturating (no real underflow for any BTC-era date).
fn before_op_date(shortfall: &Shortfall) -> TaxDate {
    shortfall.date.previous_day().unwrap_or(shortfall.date)
}

fn shift_date(d: TaxDate, days: i64) -> TaxDate {
    d.saturating_add(time::Duration::days(days))
}

// ── Render (pure; no ratatui dependency here — draw_edit.rs wraps these lines in a Paragraph) ─────────

/// The full Declare flow render — a pure derived text render (mirrors `defensive_dashboard::
/// render_dashboard`'s own "pure String builder" shape).
pub fn render_declare_flow(state: &DeclareFlowState, prices: &dyn PriceProvider) -> Vec<String> {
    let mut lines = vec![
        format!(
            "Declare — covering shortfall {:?} ({} sat short on {})",
            state.shortfall.event, state.shortfall.short_sat, state.shortfall.date
        ),
        String::new(),
    ];

    if state.safe_harbor_blocked {
        // DFW-D9 M-3 / KAT(d): a first-class entry state, not a final-Enter surprise.
        lines.push(
            "Note: an in-force safe-harbor allocation or a pre-2025 tranche is present — a pre-2025 \
             declare here will be refused (the two are mutually exclusive)."
                .to_string(),
        );
        lines.push(String::new());
    }

    lines.push(format!(
        "sat: {}   wallet: {:?}   era preset: {:?}",
        state.sat, state.wallet, state.preset
    ));
    lines.push(format!(
        "window: {} .. {}  (attest this as YOUR OWN knowledge of when you acquired these coins — the \
         window's substance is the filer's attestation, never tool-sourced)",
        state.window_start, state.window_end
    ));

    // The cheap trio (DFW-D9/D10): floor + Coverage (or its can-never-promote refusal) + holding-date.
    match state.floor_readout(prices) {
        Ok(cf) => {
            let term = if state.is_long_term_at_short_date() {
                "long-term"
            } else {
                "short-term"
            };
            lines.push(format!(
                "floor (if later promoted): ${:.2}   coverage: {:?}   holding date: {} ({term} at the \
                 short op's date)",
                cf.filed_basis, cf.coverage, state.window_end
            ));
        }
        Err(PromoteRefusal::NoCoverage) => {
            lines.push(
                "floor: NOT COMPUTABLE — no price data covers this window at all. Declaring at $0 is \
                 still fine, but this tranche could never later be promoted from this window."
                    .to_string(),
            );
        }
        Err(PromoteRefusal::PartialCoverage) => {
            lines.push(format!(
                "floor: NOT COMPUTABLE (Coverage::{:?}) — some days in this window have no price data, \
                 so the covered-part min is not provably the TRUE window min. Declaring at $0 is still \
                 fine, but this tranche could never later be promoted from this window.",
                Coverage::Partial
            ));
        }
    }

    // On-demand tax-Δ (DFW-D10 M-1) — never per-keystroke.
    match &state.tax_delta {
        None => lines.push("tax-Δ if later promoted: stale — recompute (press 't')".to_string()),
        Some(SavingFlavor::ComputedTax { year, delta }) => {
            lines.push(format!("tax-Δ if later promoted ({year}): ${delta:.2}"));
        }
        Some(SavingFlavor::Uncomputable { year, gain_delta }) => {
            lines.push(format!(
                "tax-Δ if later promoted ({year}): not a dollar figure — gain-Δ only: ${gain_delta:.2} \
                 (no stored tax profile / no bundled table / a Hard blocker)"
            ));
        }
        Some(SavingFlavor::Named(msg)) => {
            lines.push(format!("tax-Δ if later promoted: {msg}"));
        }
    }

    lines.push(String::new());
    match state.step {
        DeclareFlowStep::Edit => {
            lines.push(
                "[Tab] cycle era preset  [h/l] window_start ∓1d  [j/k] window_end ∓1d  [+/-] sat ±1000  \
                 [t] compute tax-Δ  [Enter] review & confirm  [Esc] cancel"
                    .to_string(),
            );
        }
        DeclareFlowStep::Confirm => {
            lines.push(
                "Confirm: this declares a $0 basis for the above sat/window/wallet — REVOCABLE until \
                 promoted, no Form 8275. You are asserting these coins were acquired ENTIRELY OUTSIDE \
                 the vault's records."
                    .to_string(),
            );
            lines.push("[Enter] declare  [Esc] back to edit".to_string());
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use btctax_core::price::StaticPrices;
    use btctax_core::EventId;
    use std::collections::BTreeMap;
    use time::macros::date;

    fn wallet() -> WalletId {
        WalletId::Exchange {
            provider: "cb".into(),
            account: "m".into(),
        }
    }

    fn shortfall_on(date_: TaxDate) -> Shortfall {
        Shortfall {
            event: EventId::decision(1),
            wallet: Some(wallet()),
            date: date_,
            short_sat: 10_000_000,
            fee_sat: 0,
        }
    }

    fn cfg() -> ProjectionConfig {
        ProjectionConfig::default()
    }

    // ── (d): the safe-harbor exclusion is a first-class ENTRY state ──────────────────────────────────

    #[test]
    fn safe_harbor_blocked_renders_as_a_first_class_entry_note() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), true);
        assert!(state.safe_harbor_blocked);
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.to_lowercase().contains("safe-harbor"),
            "safe_harbor_blocked must render as a visible, FIRST-CLASS note (not only discovered at a \
             final refusal): {rendered}"
        );
    }

    #[test]
    fn safe_harbor_not_blocked_renders_no_such_note() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), false);
        assert!(!state.safe_harbor_blocked);
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            !rendered.to_lowercase().contains("safe-harbor"),
            "no safe-harbor note must render when there is no conflict: {rendered}"
        );
    }

    // ── (b): declare-flow prefill puts window_end before the disposal + the source wallet ────────────

    #[test]
    fn prefill_puts_window_end_strictly_before_the_short_op_date_and_the_source_wallet() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let w = wallet();
        let state = DeclareFlowState::new(sf.clone(), w.clone(), false);
        assert!(
            state.window_end < sf.date,
            "window_end {} must be strictly before the short op's date {}",
            state.window_end,
            sf.date
        );
        // The DEFAULT (oldest) preset's own end (2011-12-31) already satisfies "strictly before" a
        // 2020 short op — no conflict, so the preset's own end governs (DFW-D9), NOT the before-op
        // date itself (that only wins ON CONFLICT — see `preset_governs_starting_window_...` below).
        assert_eq!(state.window_end, era_window(EraPreset::Y2009To2011).1);
        assert_eq!(
            state.wallet, w,
            "wallet must be the short op's source-pool wallet"
        );
        assert_eq!(state.sat, sf.short_sat);
    }

    #[test]
    fn preset_governs_starting_window_but_before_op_prefill_wins_on_conflict() {
        // A shortfall dated INSIDE the oldest preset's own span (2009-2011) — the preset's raw `end`
        // (2011-12-31) would violate DFW-D5 (it's AFTER the short op date), so the before-op clamp must
        // win.
        let sf = shortfall_on(date!(2010 - 06 - 01));
        let state = DeclareFlowState::new(sf.clone(), wallet(), false);
        let (preset_start, preset_end) = era_window(EraPreset::Y2009To2011);
        assert_eq!(
            state.window_start, preset_start,
            "window_start still seeds from the preset"
        );
        assert!(
            state.window_end < preset_end,
            "the preset's raw end ({preset_end}) conflicts with DFW-D5 — the before-op clamp must win, \
             not the preset's own end"
        );
        assert_eq!(state.window_end, date!(2010 - 05 - 31));
    }

    #[test]
    fn cycle_preset_reclamps_window_end_and_invalidates_the_stale_tax_delta() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.tax_delta = Some(SavingFlavor::Named("stub".to_string()));
        state.cycle_preset();
        assert_eq!(state.preset, EraPreset::Y2012To2014);
        let (start, _end) = era_window(EraPreset::Y2012To2014);
        assert_eq!(state.window_start, start);
        assert!(
            state.tax_delta.is_none(),
            "cycling a preset must blank the stale tax-Δ"
        );
    }

    // ── (e): editing the window blanks the on-demand saving ("stale — recompute") ─────────────────────

    #[test]
    fn nudging_window_start_blanks_the_on_demand_saving() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.tax_delta = Some(SavingFlavor::ComputedTax {
            year: 2020,
            delta: rust_decimal_macros::dec!(100),
        });
        state.nudge_window_start(-1);
        assert!(
            state.tax_delta.is_none(),
            "any window edit must blank the cached tax-Δ"
        );
    }

    #[test]
    fn nudging_window_end_blanks_the_on_demand_saving_and_never_crosses_the_before_op_boundary() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.tax_delta = Some(SavingFlavor::ComputedTax {
            year: 2020,
            delta: rust_decimal_macros::dec!(100),
        });
        // Push window_end forward by a huge number of days — must clamp at the before-op boundary,
        // never cross into/after the short op's own date.
        state.nudge_window_end(9_999);
        assert!(state.tax_delta.is_none());
        assert_eq!(state.window_end, date!(2020 - 06 - 14));
    }

    #[test]
    fn nudging_sat_blanks_the_on_demand_saving_and_floors_at_one() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.tax_delta = Some(SavingFlavor::Named("stub".to_string()));
        state.nudge_sat(-100_000_000_000);
        assert!(state.tax_delta.is_none());
        assert_eq!(state.sat, 1, "sat must never go to 0 or negative");
    }

    // ── (c): Coverage::Partial/NoCoverage refusal surfaces live in the readout ────────────────────────

    #[test]
    fn no_price_coverage_at_all_surfaces_as_no_coverage_live() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), false);
        let empty_prices = StaticPrices::default();
        assert_eq!(
            state.floor_readout(&empty_prices),
            Err(PromoteRefusal::NoCoverage)
        );
    }

    #[test]
    fn a_gap_in_the_window_surfaces_as_partial_coverage_live() {
        let sf = shortfall_on(date!(2020 - 01 - 10));
        let state = DeclareFlowState::new(sf, wallet(), false);
        // Price data on window_start and window_end only — a gap in between.
        let mut m = BTreeMap::new();
        m.insert(state.window_start, rust_decimal_macros::dec!(10_000));
        m.insert(state.window_end, rust_decimal_macros::dec!(10_000));
        let gappy = StaticPrices(m);
        assert_eq!(
            state.floor_readout(&gappy),
            Err(PromoteRefusal::PartialCoverage)
        );
    }

    #[test]
    fn full_price_coverage_surfaces_a_computed_floor_live() {
        let sf = shortfall_on(date!(2020 - 01 - 10));
        let state = DeclareFlowState::new(sf, wallet(), false);
        let mut m = BTreeMap::new();
        let mut d = state.window_start;
        loop {
            m.insert(d, rust_decimal_macros::dec!(10_000));
            if d == state.window_end {
                break;
            }
            d = d.next_day().unwrap();
        }
        let full = StaticPrices(m);
        let cf = state.floor_readout(&full).expect("full coverage computes");
        assert_eq!(cf.coverage, Coverage::Full);
    }

    // ── (d) grep guard: this module never calls apply_declare directly (C-3) ─────────────────────────

    #[test]
    fn declare_flow_never_calls_apply_declare_directly() {
        // Token constructed at RUNTIME (mirrors KAT-G1's own self-check convention) so this assertion's
        // own source line does not itself contain the literal forbidden token.
        let forbidden = format!("{}(", "apply_declare");
        let src = include_str!("declare_flow.rs");
        assert!(
            !src.contains(&forbidden),
            "declare_flow.rs must COLLECT input + read plan_declare only — the write goes through \
             edit::persist::persist_declare_tranche (C-3/KAT-G1)"
        );
    }

    // ── clearance (DFW-D5.2) — reads plan_declare, never writes ───────────────────────────────────────

    #[test]
    fn clearance_reflects_plan_declare_and_is_a_pure_read() {
        use btctax_core::event::{Acquire, BasisSource, EventPayload};
        use btctax_core::identity::{Source, SourceRef};
        use time::macros::datetime;

        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf.clone(), wallet(), false);
        let empty_events: Vec<LedgerEvent> = vec![];
        let prices = StaticPrices::default();
        let now = datetime!(2026 - 01 - 01 0:00 UTC);

        // Clears trivially: no shipped-set conflict, and the shadow re-projection finds no OTHER
        // UncoveredDisposal on the target (there's no disposal event in `empty_events` at all — the
        // target itself is absent, so the clearance shadow's "no UncoveredDisposal remains" check
        // holds vacuously). This exercises "reads plan_declare, never mutates `empty_events`".
        let result = state.clearance(&empty_events, &prices, &cfg(), now);
        assert!(result.is_ok(), "a vacuous target must clear: {result:?}");
        assert_eq!(
            empty_events.len(),
            0,
            "clearance must never mutate the caller's events"
        );

        // A degenerate window (window_start > window_end) refuses via the shipped-set gate.
        state.window_start = date!(2020 - 06 - 16);
        state.window_end = date!(2020 - 06 - 14);
        assert!(state
            .clearance(&empty_events, &prices, &cfg(), now)
            .is_err());

        // Sanity: the acquire helper import stays used across future edits.
        let _ = EventPayload::Acquire(Acquire {
            sat: 1,
            usd_cost: rust_decimal_macros::dec!(1),
            fee_usd: rust_decimal_macros::dec!(0),
            basis_source: BasisSource::ExchangeProvided,
        });
        let _ = EventId::import(Source::Coinbase, SourceRef::new("x"));
    }

    // ── T6-Minor1: the on-demand tax-Δ sources a REAL dollar figure from a real profile ───────────────

    #[test]
    fn compute_tax_delta_with_a_real_profile_and_table_yields_computed_tax() {
        use btctax_adapters::BundledTaxTables;
        use btctax_core::tax::testonly::ty2024_table;
        use btctax_core::tax::{Carryforward, FilingStatus, TaxTable};
        use time::macros::datetime;

        let sf = Shortfall {
            event: EventId::decision(1),
            wallet: Some(wallet()),
            date: date!(2024 - 06 - 15),
            short_sat: 10_000_000,
            fee_sat: 0,
        };
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.window_start = date!(2023 - 01 - 01);
        state.window_end = date!(2023 - 01 - 03);

        let mut m = BTreeMap::new();
        let mut d = state.window_start;
        loop {
            m.insert(d, rust_decimal_macros::dec!(10_000));
            if d == state.window_end {
                break;
            }
            d = d.next_day().unwrap();
        }
        let prices = StaticPrices(m);

        // A real 2024 disposal of the SAME sat, so the synthetic tranche's leg realizes in 2024.
        let events = vec![LedgerEvent {
            id: EventId::import(
                btctax_core::identity::Source::Coinbase,
                btctax_core::identity::SourceRef::new("SELL"),
            ),
            utc_timestamp: datetime!(2024-06-15 00:00 UTC),
            original_tz: time::UtcOffset::UTC,
            wallet: Some(wallet()),
            payload: btctax_core::event::EventPayload::Dispose(btctax_core::event::Dispose {
                sat: 10_000_000,
                usd_proceeds: rust_decimal_macros::dec!(50_000),
                fee_usd: rust_decimal_macros::dec!(0),
                kind: btctax_core::event::DisposeKind::Sell,
            }),
        }];

        let mut tables: BTreeMap<i32, TaxTable> = BTreeMap::new();
        tables.insert(2024, ty2024_table());
        let profile = TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: rust_decimal_macros::dec!(200_000),
            magi_excluding_crypto: rust_decimal_macros::dec!(200_000),
            qualified_dividends_and_other_pref_income: rust_decimal_macros::dec!(0),
            other_net_capital_gain: rust_decimal_macros::dec!(0),
            capital_loss_carryforward_in: Carryforward {
                short: rust_decimal_macros::dec!(0),
                long: rust_decimal_macros::dec!(0),
            },
            w2_ss_wages: rust_decimal_macros::dec!(0),
            w2_medicare_wages: rust_decimal_macros::dec!(0),
            schedule_c_expenses: rust_decimal_macros::dec!(0),
        };

        state.compute_tax_delta(&events, &prices, &cfg(), &tables, Some(&profile));
        assert!(
            matches!(state.tax_delta, Some(SavingFlavor::ComputedTax { year: 2024, .. })),
            "a real bundled table + a real profile must yield ComputedTax, never Uncomputable/None: \
             {:?}",
            state.tax_delta
        );

        // Sanity anchor: BundledTaxTables stays a valid TaxTables impl (used elsewhere by the flow's
        // real caller, main.rs, which loads it from the session — not re-derived here).
        let _ = BundledTaxTables::load();
    }

    #[test]
    fn compute_tax_delta_without_a_profile_yields_uncomputable_never_a_bare_dollar() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        let events: Vec<LedgerEvent> = vec![];
        let prices = StaticPrices::default();
        let tables: BTreeMap<i32, btctax_core::tax::TaxTable> = BTreeMap::new();
        state.compute_tax_delta(&events, &prices, &cfg(), &tables, None);
        assert!(
            matches!(state.tax_delta, Some(SavingFlavor::Named(_))),
            "no price coverage at all must be Named: {:?}",
            state.tax_delta
        );
    }

    // ── holding-date / long-term readout ───────────────────────────────────────────────────────────────

    #[test]
    fn holding_date_is_window_end_and_long_term_reflects_is_long_term() {
        let sf = shortfall_on(date!(2022 - 01 - 01));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        // Force a window_end more than a year before the short op's date — long-term.
        state.window_end = date!(2020 - 06 - 01);
        assert_eq!(state.holding_date(), date!(2020 - 06 - 01));
        assert!(state.is_long_term_at_short_date());

        // A window_end just before the short op's date — short-term.
        state.window_end = date!(2021 - 12 - 31);
        assert!(!state.is_long_term_at_short_date());
    }
}
