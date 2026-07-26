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
    /// DFW-D9 M-3 / KAT(d): a first-class ENTRY state — is a safe-harbor allocation IN FORCE?
    ///
    /// ★ P-C gate tax I-3: this is the **DIRECTIONAL** predicate, `tranche_guard::
    /// in_force_allocation_exists`, NOT `journey_view.safe_harbor_blocked`. That flag is the SYMMETRIC
    /// mutual-exclusion state (`in_force_allocation_exists || pre2025_tranche_exists`), so it also goes
    /// true after the filer's own FIRST pre-2025 declare — and the declare-side gate
    /// (`guard_tranche_vs_allocation`) refuses only in the `window_end < TRANSITION_DATE &&
    /// in_force_allocation_exists` direction. Keying the note on the symmetric flag told every filer
    /// covering their SECOND shortfall that a declare `plan_declare` accepts "will be refused" — the
    /// wizard's own majority path. Still a CORE predicate (never the cli-private
    /// `guard_tranche_vs_allocation`), and still purely informational: the actual gate is
    /// `plan_declare`'s own shipped-set check at confirm time (DFW-D1 — no second gating authority).
    pub allocation_in_force: bool,
    /// DFW-D10 M-1: the on-demand tax-Δ preview. `None` + `tax_delta_stale` = STALE ("recompute");
    /// `None` alone = not computed yet. Invalidated on ANY window/sat edit.
    pub tax_delta: Option<SavingFlavor>,
    /// ★ tax N-2: distinguishes "invalidated by an edit" (stale) from "never computed" (first open), so
    /// the readout does not tell a filer to *re*compute something they never computed.
    pub tax_delta_stale: bool,
}

impl DeclareFlowState {
    /// Open the flow for `shortfall` (DFW-D5 prefill): `window_end` strictly before the short op's date;
    /// `wallet` = the short op's source-pool wallet (the caller unwraps `shortfall.wallet` — a
    /// `DeclareCandidate` always carries one, per `discovery::triage`'s own routing). `window_start`
    /// seeds from the OLDEST (most conservative — DFW-D9's "wider window → lower floor" bias) era
    /// preset; the before-op prefill clamps `window_end` immediately (DFW-D9: "the DFW-D5 before-op
    /// prefill governs over a preset's window_end where they conflict").
    pub fn new(shortfall: Shortfall, wallet: WalletId, allocation_in_force: bool) -> Self {
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
            window_start: preset_start.min(window_end),
            window_end,
            preset,
            allocation_in_force,
            tax_delta: None,
            tax_delta_stale: false,
            shortfall,
        }
    }

    /// Invalidate the cached on-demand tax-Δ after a window/sat edit (DFW-D10 M-1). Only marks it STALE
    /// when there WAS a computed value to invalidate (★ tax N-2 — an edit before the first `t` leaves the
    /// readout on "not computed yet", not "stale — recompute").
    fn invalidate_tax_delta(&mut self) {
        self.tax_delta_stale |= self.tax_delta.take().is_some();
    }

    /// Cycle to the NEXT era preset (DFW-D9 "confirm/edit starting point"): seeds `window_start` from
    /// the preset's own start; clamps `window_end` to the DFW-D5 before-op day when the preset's own end
    /// would not otherwise satisfy it (the before-op prefill governs on conflict). ★ tax M-1: the seeded
    /// `window_start` is then floored at the (already-clamped) `window_end`, so cycling to a preset that
    /// STARTS after the short op can never leave an INVERTED window — which `window_reference` reports as
    /// "no price data covers this window at all", blaming missing data for an incoherent window (mirrors
    /// the clamp `nudge_window_start` already carries). Invalidates the on-demand tax-Δ (M-1).
    pub fn cycle_preset(&mut self) {
        self.preset = next_preset(self.preset);
        let (start, end) = era_window(self.preset);
        let before_op = before_op_date(&self.shortfall);
        self.window_end = if end < before_op { end } else { before_op };
        self.window_start = start.min(self.window_end);
        self.invalidate_tax_delta();
    }

    /// Nudge `window_start` by `days` (may move earlier or later than the current preset's own start —
    /// a manual DFW-D9 edit), CLAMPED so it can never move PAST `window_end` (`window_start <= window_end`
    /// is preserved here defensively — T8-review Minor-2: `plan_declare`/confirm-time already refuses a
    /// degenerate ordering, but leaving it unbounded here let a manual nudge surface a nonsensical
    /// inverted-window `NoCoverage` in the live readout for no reason) and never before the earliest
    /// sensible date — Bitcoin's genesis block, `era_window(ALL_PRESETS[0]).0` (2009-01-03), the SAME
    /// floor already governing the oldest era preset (`DeclareFlowState::new`'s own seed) — no new date
    /// invented here. Invalidates the on-demand tax-Δ (M-1).
    pub fn nudge_window_start(&mut self, days: i64) {
        let candidate = shift_date(self.window_start, days);
        let genesis = era_window(ALL_PRESETS[0]).0;
        let floored = if candidate < genesis {
            genesis
        } else {
            candidate
        };
        self.window_start = if floored > self.window_end {
            self.window_end
        } else {
            floored
        };
        self.invalidate_tax_delta();
    }

    /// Nudge `window_end` by `days`, CLAMPED to never cross the DFW-D5 before-op boundary (the
    /// invariant that makes the lot exist in time to cover the short op — never overridable by a manual
    /// edit) and — ★ tax M-1 — FLOORED at `window_start`, so a downward nudge can never invert the
    /// window (the sibling bound `nudge_window_start` already carries). Invalidates the on-demand
    /// tax-Δ (M-1).
    pub fn nudge_window_end(&mut self, days: i64) {
        let candidate = shift_date(self.window_end, days);
        let before_op = before_op_date(&self.shortfall);
        let capped = if candidate > before_op {
            before_op
        } else {
            candidate
        };
        self.window_end = if capped < self.window_start {
            self.window_start
        } else {
            capped
        };
        self.invalidate_tax_delta();
    }

    /// Nudge `sat` by `delta` sat (DFW-D8/N-1: the filer MAY edit above the prefilled `short_sat` — the
    /// excess is the out-of-scope manual-holdings shape entering by a side door; it files nothing wrong
    /// at `$0`). Floored at 1 (declaring 0/negative sat is never valid — `plan_declare`'s own gate).
    /// Invalidates the on-demand tax-Δ (M-1).
    pub fn nudge_sat(&mut self, delta: i64) {
        self.sat = (self.sat + delta).max(1);
        self.invalidate_tax_delta();
    }

    /// ★ tax M-2 (SPEC DFW-D8, verbatim: "the excess is the out-of-scope manual-holdings shape … a
    /// confirm-note suffices"): how much MORE than the targeted shortfall actually needs is being
    /// declared, if any. `None` when the declare is sized at or below the shortfall.
    pub fn excess_sat(&self) -> Option<i64> {
        let excess = self.sat - self.shortfall.short_sat;
        (excess > 0).then_some(excess)
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
    /// the targeted shortfall? A pure READ — `plan_declare` never mutates.
    ///
    /// ★ tax N-1 / arch M-2 (doc truth): this is **not** wired into the live readout — `render_declare_flow`
    /// renders only the cheap trio (correct per DFW-D10: no per-keystroke re-projection). The REAL and
    /// only declare gate is `declare_flow_confirm`'s own FRESH `plan_declare` at the Confirm-step Enter,
    /// which surfaces any refusal with its reason (DFW-D5). This fn exists as the same-shaped pure probe
    /// (its `Ok(plan)` is exactly what `persist_declare_tranche` is handed) and is exercised by this
    /// module's own tests; wiring it into the readout is a filed follow-up, not shipped behavior.
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
        self.tax_delta_stale = false;
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

    if state.allocation_in_force {
        // DFW-D9 M-3 / KAT(d): a first-class entry state, not a final-Enter surprise. ★ tax I-3: keyed
        // on the DIRECTIONAL predicate (an in-force ALLOCATION), which is the only condition under which
        // the declare gate actually refuses — an existing pre-2025 tranche of the filer's own does NOT
        // block the next declare, and claiming otherwise made them abandon a correct, available action.
        lines.push(
            "Note: a safe-harbor allocation is in force — a declare whose window ends before 2025-01-01 \
             will be refused (a safe-harbor allocation and a pre-2025 tranche are mutually exclusive)."
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
        // ★ tax N-2: nothing is "stale" until something was computed and then edited away.
        None if state.tax_delta_stale => {
            lines.push("tax-Δ if later promoted: stale — recompute (press 't')".to_string())
        }
        None => lines.push("tax-Δ if later promoted: not computed yet (press 't')".to_string()),
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
            // ★ tax M-2 (SPEC DFW-D8): declaring MORE than the shortfall needs is allowed and files
            // nothing wrong at $0 — but the excess is a phantom $0-basis lot that, if later promoted,
            // would file a >$0 floor on sat the shortfall never needed. A confirm-note suffices.
            if let Some(excess) = state.excess_sat() {
                lines.push(format!(
                    "Note: this declares {excess} sat MORE than the shortfall needs ({} vs {} sat \
                     short). The excess is a $0-basis lot in its own right — it files nothing wrong at \
                     $0, but if you later PROMOTE this tranche the >$0 floor is filed on the excess \
                     too. Declare only what you actually held.",
                    state.sat, state.shortfall.short_sat
                ));
            }
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
    fn an_in_force_allocation_renders_as_a_first_class_entry_note() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), true);
        assert!(state.allocation_in_force);
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.to_lowercase().contains("safe-harbor"),
            "an in-force allocation must render as a visible, FIRST-CLASS note (not only discovered at \
             a final refusal): {rendered}"
        );
        assert!(
            rendered.to_lowercase().contains("will be refused"),
            "with an allocation in force the refusal claim is TRUE and must be stated: {rendered}"
        );
    }

    #[test]
    fn no_allocation_in_force_renders_no_such_note() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), false);
        assert!(!state.allocation_in_force);
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            !rendered.to_lowercase().contains("safe-harbor"),
            "no safe-harbor note must render when there is no conflict: {rendered}"
        );
    }

    /// ★ P-C gate tax I-3 — the wizard's own MAJORITY path: after the filer's first declare (the default
    /// preset ends in 2011, so `pre2025_tranche_exists` is true), a vault with NO allocation at all must
    /// NOT be told the next declare "will be refused". `plan_declare` accepts it; the old note keyed on
    /// the SYMMETRIC `journey_view.safe_harbor_blocked` and claimed otherwise, so a filer abandoned a
    /// correct, available action and left an `UncoveredDisposal` Hard blocker standing.
    #[test]
    fn a_pre2025_tranche_without_an_allocation_never_claims_a_declare_will_be_refused() {
        use btctax_core::event::{DeclareTranche, EventPayload};
        use btctax_core::tranche_guard::{in_force_allocation_exists, pre2025_tranche_exists};
        use time::macros::datetime;

        // A vault holding exactly what the filer's OWN first wizard declare leaves behind.
        let events = vec![LedgerEvent {
            id: EventId::decision(1),
            utc_timestamp: datetime!(2026-01-01 0:00 UTC),
            original_tz: time::UtcOffset::UTC,
            wallet: None,
            payload: EventPayload::DeclareTranche(DeclareTranche {
                sat: 10_000_000,
                wallet: wallet(),
                window_start: date!(2009 - 01 - 03),
                window_end: date!(2011 - 12 - 31),
            }),
        }];
        assert!(
            pre2025_tranche_exists(&events),
            "fixture: the first declare leaves a pre-2025 tranche on record"
        );
        assert!(
            !in_force_allocation_exists(&events),
            "fixture: there is NO safe-harbor allocation — the declare gate does not refuse"
        );

        // What `open_declare_flow` now threads: the DIRECTIONAL predicate, not the symmetric flag.
        let state = DeclareFlowState::new(
            shortfall_on(date!(2020 - 06 - 15)),
            wallet(),
            in_force_allocation_exists(&events),
        );
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            !rendered.to_lowercase().contains("will be refused"),
            "no refusal may be claimed for a declare plan_declare accepts: {rendered}"
        );

        // And the engine agrees: this declare is NOT refused by the allocation guard.
        let now = datetime!(2026 - 01 - 01 0:00 UTC);
        let refusal = state.clearance(&events, &prices, &cfg(), now).err();
        let text = format!("{refusal:?}").to_lowercase();
        assert!(
            !text.contains("safe-harbor") && !text.contains("safe harbor"),
            "plan_declare must not refuse this on safe-harbor grounds: {refusal:?}"
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

    // ── T8-review Minor-2: nudge_window_start is bounded (never past window_end, never before genesis) ─

    #[test]
    fn nudging_window_start_never_crosses_past_window_end_or_before_genesis() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        // Pin window_end close to window_start so a large positive nudge would otherwise cross it.
        state.window_end = date!(2009 - 01 - 10);
        state.nudge_window_start(9_999);
        assert_eq!(
            state.window_start, state.window_end,
            "window_start must clamp AT window_end, never past it: {:?}",
            state.window_start
        );

        // A large negative nudge must clamp at Bitcoin's genesis block, never before it.
        state.nudge_window_start(-3_650);
        assert_eq!(
            state.window_start,
            date!(2009 - 01 - 03),
            "window_start must clamp at Bitcoin's genesis block (2009-01-03), never before it"
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

    /// ★ arch M-4 (renamed, P-C gate): this fixture has NO price coverage, so `declare_preview_saving`
    /// short-circuits to `SavingFlavor::Named` BEFORE `profile` is ever consulted — the old name
    /// (`…without_a_profile_yields_uncomputable_never_a_bare_dollar`) claimed a property it never
    /// exercised. The real no-profile property is covered at core level by
    /// `btctax-core/tests/defensive_journey.rs::declare_preview_saving_is_uncomputable_without_a_profile`.
    /// What this pins is the DFW-D10 invariant that matters here: an uncomputable preview is NEVER
    /// rendered as a bare dollar figure.
    #[test]
    fn compute_tax_delta_without_price_coverage_yields_named_never_a_bare_dollar() {
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
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            !rendered.contains("tax-Δ if later promoted (2020): $"),
            "an uncomputable preview must never render as a bare dollar figure: {rendered}"
        );
    }

    // ── ★ tax M-1 (P-C gate): an INVERTED window (window_start > window_end) is unreachable ──────────
    //
    // `window_reference` returns `None` for start > end, which the readout reports as "no price data
    // covers this window at all" — blaming missing data for an incoherent window, right across the
    // 2018-2023 audience years. `nudge_window_start` was already clamped (T8-review Minor-2); these pin
    // the two remaining doors.

    #[test]
    fn cycling_to_a_preset_that_starts_after_the_short_op_never_inverts_the_window() {
        // A 2018 shortfall: every preset from Y2018To2020 on STARTS after the before-op clamp bites.
        let sf = shortfall_on(date!(2018 - 06 - 01));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        for _ in 0..(ALL_PRESETS.len() * 2) {
            state.cycle_preset();
            assert!(
                state.window_start <= state.window_end,
                "preset {:?} left an INVERTED window: {} .. {}",
                state.preset,
                state.window_start,
                state.window_end
            );
            assert!(
                state.window_end < state.shortfall.date,
                "the DFW-D5 before-op clamp must still hold for {:?}",
                state.preset
            );
        }

        // And the readout never blames missing price data for it.
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            state.window_start <= state.window_end,
            "post-cycle window must be coherent: {rendered}"
        );
    }

    #[test]
    fn nudging_window_end_down_never_crosses_below_window_start() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.nudge_window_end(-9_999);
        assert_eq!(
            state.window_end, state.window_start,
            "window_end must clamp AT window_start, never below it"
        );
        assert!(state.window_start <= state.window_end);
    }

    // ── ★ tax M-2 (SPEC DFW-D8): the over-coverage confirm-note ──────────────────────────────────────

    #[test]
    fn declaring_more_sat_than_the_shortfall_needs_carries_a_confirm_note() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        let prices = StaticPrices::default();

        // Sized exactly at the shortfall: no excess, no note.
        state.step = DeclareFlowStep::Confirm;
        assert_eq!(state.excess_sat(), None);
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            !rendered.contains("MORE than the shortfall"),
            "no over-coverage note when the declare is correctly sized: {rendered}"
        );

        // Nudged above `short_sat`: the DFW-D8 confirm-note must name the excess.
        state.step = DeclareFlowStep::Edit;
        state.nudge_sat(30_000_000);
        state.step = DeclareFlowStep::Confirm;
        assert_eq!(state.excess_sat(), Some(30_000_000));
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.contains("30000000 sat MORE than the shortfall"),
            "SPEC DFW-D8: the excess must be surfaced as a confirm-note: {rendered}"
        );
        assert!(
            rendered.to_uppercase().contains("PROMOTE"),
            "the note must name the real consequence (a later promote files the floor on the excess \
             too): {rendered}"
        );
    }

    // ── ★ tax N-2: "stale — recompute" only after something WAS computed ─────────────────────────────

    #[test]
    fn the_tax_delta_readout_says_not_computed_yet_on_first_open_and_stale_only_after_an_edit() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        let prices = StaticPrices::default();

        let fresh = render_declare_flow(&state, &prices).join("\n");
        assert!(
            fresh.contains("not computed yet"),
            "nothing is STALE on first open — the filer never computed anything: {fresh}"
        );
        assert!(!fresh.contains("stale"), "{fresh}");

        // An edit BEFORE any compute still is not "stale".
        state.nudge_sat(1_000);
        let edited = render_declare_flow(&state, &prices).join("\n");
        assert!(edited.contains("not computed yet"), "{edited}");

        // Compute, then edit → NOW it is stale.
        state.tax_delta = Some(SavingFlavor::Named("computed".to_string()));
        state.nudge_window_start(-1);
        let stale = render_declare_flow(&state, &prices).join("\n");
        assert!(
            stale.contains("stale — recompute"),
            "an edit that invalidates a COMPUTED value is stale: {stale}"
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
