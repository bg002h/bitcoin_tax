//! The Declare flow (Task 8, Phase P-C): collects window/sat/wallet for a Defensive-Filing declare
//! candidate, drives the shipped `btctax_cli::plan_declare(target_shortfall = Some(...))` chokepoint for
//! a live floor/coverage/holding-date readout (DFW-D9) + the DFW-D5.2 clearance check, and an on-demand
//! clamped-saving preview (DFW-D10 M-1). **C-3:** this module COLLECTS input and READS `plan_declare` —
//! it never calls `btctax_cli::apply_declare` directly; the WRITE goes through
//! `edit::persist::persist_declare_tranche` (the ONLY caller of `apply_declare` in this crate,
//! mechanically enforced by `persist::tests::kat_g1_mechanized_source_gate`).
//!
//! ★ **OWNER DECISION (era table, P-D/ship): the era preset is NOT pre-selected — the filer must
//! ACTIVELY pick one.** On the `$0`-declare branch the window's only filing-substantive effect is the
//! HOLDING PERIOD (the basis is `$0` either way), because `window_end` **is** the lot's acquisition date
//! (`resolve.rs:~1310`). Pre-selecting the OLDEST bucket therefore defaulted nearly every covered
//! disposal to LONG-term at preferential rates — the tool answering a filing question, in the
//! taxpayer-favorable direction, that the filer never answered. That is precisely what this project's
//! answered-ness invariant forbids. So `preset`/`window_start` are `Option` and start `None`, the flow
//! REFUSES to advance or confirm without a pick (fail-closed — nothing is recorded), and the pick is
//! preserved across a Confirm→Edit bounce. This mirrors the BG-D5 provenance step
//! (`promote_flow.rs`) verbatim in shape. The DFW-D5 prefill is INDEPENDENT of the preset and is
//! unchanged: `window_end` opens strictly before the short op's date and `wallet` is the short op's
//! source pool, pick or no pick.

use btctax_core::conservative::Coverage;
use btctax_core::conservative_promote::{filed_basis_for, ComputedFloor, PromoteRefusal};
use btctax_core::conventions::is_long_term;
use btctax_core::defensive::discovery::Shortfall;
use btctax_core::defensive::era::{era_window, EraPreset, ALL_PRESETS};
use btctax_core::defensive::{declare_preview_saving, SavingFlavor};
use btctax_core::price::PriceProvider;
use btctax_core::project::ProjectionConfig;
use btctax_core::{LedgerEvent, TaxDate, TaxProfile, TaxTables, WalletId};

/// The prompt shown when the filer presses Enter on the Edit step WITHOUT having picked an era. NOT a
/// refusal text (no engine gate has run): it is the UI insisting the filer answer, which is the whole
/// point of the step (the answered-ness invariant — the tool must never silently answer a filing
/// question for the filer). Mirrors `promote_flow::PROVENANCE_UNANSWERED`.
const ERA_UNANSWERED: &str =
    "choose when you acquired these coins — press the number of an era above. This is YOUR attestation \
     about YOUR coins; the tool will not answer it for you. It sets the acquisition date of the \
     declared lot, and with it whether the disposal it covers is short- or long-term.";

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
    /// ★ OWNER DECISION (explicit pick): `None` until the filer picks an era. The tool never supplies a
    /// `window_start` the filer did not choose, so an unanswered window is unrepresentable rather than
    /// merely undisplayed. A manual `nudge_window_start` moves it afterwards (a legitimate DFW-D9 edit).
    pub window_start: Option<TaxDate>,
    /// The DFW-D5 prefill — INDEPENDENT of the preset: strictly before the short op's date from the
    /// moment the flow opens, clamped there forever after (a preset can only ever pull it EARLIER).
    pub window_end: TaxDate,
    /// The era preset the filer PICKED, which seeded `window_start`/`window_end`'s starting point
    /// (DFW-D9) — `None` until they actively choose one, and a later manual nudge does not change it (it
    /// remains the last-applied starting point). Lives OUTSIDE `step` (mirroring
    /// `PromoteFlowState::provenance`) so a Confirm→Edit bounce shows the filer their previous pick.
    pub preset: Option<EraPreset>,
    /// The era step's inline message: either `ERA_UNANSWERED` (Enter with nothing picked) or the
    /// "that era cannot apply to this shortfall" explanation. Cleared by a successful pick.
    pub era_error: Option<String>,
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
    /// `DeclareCandidate` always carries one, per `discovery::triage`'s own routing).
    ///
    /// ★ OWNER DECISION: **NO era preset is pre-selected**, so `preset`/`window_start` open `None` — the
    /// tool does not answer "when did you acquire these coins?" for the filer (see the module doc). The
    /// DFW-D5 prefill is independent of that and is fully applied here: `window_end` is the
    /// before-the-short-op day from the moment the flow opens, and picking an era can only ever pull it
    /// EARLIER ("the DFW-D5 before-op prefill governs over a preset's window_end where they conflict").
    pub fn new(shortfall: Shortfall, wallet: WalletId, allocation_in_force: bool) -> Self {
        let window_end = before_op_date(&shortfall);
        Self {
            step: DeclareFlowStep::Edit,
            sat: shortfall.short_sat,
            wallet,
            window_start: None,
            window_end,
            preset: None,
            era_error: None,
            allocation_in_force,
            tax_delta: None,
            tax_delta_stale: false,
            shortfall,
        }
    }

    /// The window the filer has actually chosen — `None` until an era is picked. Every window consumer
    /// (`floor_readout`, `holding_date`, `is_long_term_at_short_date`, `compute_tax_delta`, the render,
    /// and `main.rs`'s confirm tail) goes through this, so none of them can read a window the filer
    /// never answered.
    pub fn window(&self) -> Option<(TaxDate, TaxDate)> {
        Some((self.window_start?, self.window_end))
    }

    /// ★ OWNER DECISION / KAT (c): record the filer's ERA pick (the `1..=ALL_PRESETS.len()` picker) and
    /// seed the window from it — `window_start` from the preset's own start, `window_end` clamped to the
    /// DFW-D5 before-op day whenever the preset's own end would violate it (the before-op prefill
    /// governs on conflict). Then `window_start` is floored at the (already-clamped) `window_end` so no
    /// pick can leave an INVERTED window (★ tax M-1).
    ///
    /// ★ tax Minor 1 (P-C gate r2), preserved in its STRONGER form: a preset whose own `start` is AFTER
    /// the DFW-D5 before-op boundary cannot apply to this shortfall at all — the M-1 clamp would make it
    /// merely COHERENT by collapsing it to a degenerate ONE-DAY window pinned at the before-op day,
    /// silently in the LEAST conservative direction (the highest floor the flow can produce, and a
    /// SHORT-term holding date — the opposite of DFW-D9's "wider window → lower floor"). The old
    /// Tab-cycling silently SKIPPED such presets; an explicit pick cannot silently skip, so it is
    /// REFUSED instead, with the reason — and any previous pick is left untouched (fail-closed).
    /// Invalidates the on-demand tax-Δ (M-1).
    pub fn select_preset(&mut self, preset: EraPreset) {
        let before_op = before_op_date(&self.shortfall);
        let (start, end) = era_window(preset);
        if start > before_op {
            self.era_error = Some(format!(
                "{preset:?} starts {start}, which is after {before_op} — a lot must exist BEFORE the \
                 disposal it covers ({}), so that era cannot apply to this shortfall. Choose an \
                 earlier one.",
                self.shortfall.date
            ));
            return;
        }
        self.preset = Some(preset);
        self.window_end = if end < before_op { end } else { before_op };
        self.window_start = Some(start.min(self.window_end));
        self.era_error = None;
        self.invalidate_tax_delta();
    }

    /// The Edit step's Enter: advance to the DFW-D8 confirmation. ★ OWNER DECISION / KAT (b): REFUSED
    /// while no era has been picked — fail-closed (the flow stays on Edit, nothing is recorded, and no
    /// window is substituted), with the prompt insisting the filer answer. Mirrors
    /// `PromoteFlowState::attest_provenance`'s unanswered arm.
    pub fn review(&mut self) {
        if self.preset.is_none() || self.window_start.is_none() {
            self.era_error = Some(ERA_UNANSWERED.to_string());
            return;
        }
        self.era_error = None;
        self.step = DeclareFlowStep::Confirm;
    }

    /// Invalidate the cached on-demand tax-Δ after a window/sat edit (DFW-D10 M-1). Only marks it STALE
    /// when there WAS a computed value to invalidate (★ tax N-2 — an edit before the first `t` leaves the
    /// readout on "not computed yet", not "stale — recompute").
    fn invalidate_tax_delta(&mut self) {
        self.tax_delta_stale |= self.tax_delta.take().is_some();
    }

    /// Nudge `window_start` by `days` (may move earlier or later than the picked preset's own start —
    /// a manual DFW-D9 edit), CLAMPED so it can never move PAST `window_end` (`window_start <= window_end`
    /// is preserved here defensively — T8-review Minor-2: `plan_declare`/confirm-time already refuses a
    /// degenerate ordering, but leaving it unbounded here let a manual nudge surface a nonsensical
    /// inverted-window `NoCoverage` in the live readout for no reason) and never before the earliest
    /// sensible date — Bitcoin's genesis block, `era_window(ALL_PRESETS[0]).0` (2009-01-03), the SAME
    /// floor already governing the oldest era preset — no new date invented here. Invalidates the
    /// on-demand tax-Δ (M-1). **Inert before an era is picked** (there is no window to nudge yet, and
    /// nudging one into existence would be the tool answering the era question by a side door).
    pub fn nudge_window_start(&mut self, days: i64) {
        let Some(current) = self.window_start else {
            return;
        };
        let candidate = shift_date(current, days);
        let genesis = era_window(ALL_PRESETS[0]).0;
        let floored = if candidate < genesis {
            genesis
        } else {
            candidate
        };
        self.window_start = Some(if floored > self.window_end {
            self.window_end
        } else {
            floored
        });
        self.invalidate_tax_delta();
    }

    /// Nudge `window_end` by `days`, CLAMPED to never cross the DFW-D5 before-op boundary (the
    /// invariant that makes the lot exist in time to cover the short op — never overridable by a manual
    /// edit) and — ★ tax M-1 — FLOORED at `window_start`, so a downward nudge can never invert the
    /// window (the sibling bound `nudge_window_start` already carries). Invalidates the on-demand
    /// tax-Δ (M-1). **Inert before an era is picked**, for the same reason as its sibling: half a window
    /// is not an edit, and the unpicked `window_end` must stay exactly on the DFW-D5 prefill.
    pub fn nudge_window_end(&mut self, days: i64) {
        let Some(window_start) = self.window_start else {
            return;
        };
        let candidate = shift_date(self.window_end, days);
        let before_op = before_op_date(&self.shortfall);
        let capped = if candidate > before_op {
            before_op
        } else {
            candidate
        };
        self.window_end = if capped < window_start {
            window_start
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
    /// state, surfaced LIVE (KAT c) rather than only discovered at a later promote attempt. The OUTER
    /// `None` is "no era picked yet" — there is no window to score, and none is invented.
    pub fn floor_readout(
        &self,
        prices: &dyn PriceProvider,
    ) -> Option<Result<ComputedFloor, PromoteRefusal>> {
        let (start, end) = self.window()?;
        Some(filed_basis_for(prices, self.sat, start, end))
    }

    /// The cheap-trio's holding-date piece: `window_end` IS the lot's holding-period start
    /// (`resolve.rs:~1310`), so it also sets short/long-term (DFW-D9). `None` until an era is picked —
    /// the acquisition date is the filer's answer, never the tool's.
    pub fn holding_date(&self) -> Option<TaxDate> {
        self.window().map(|(_start, end)| end)
    }

    /// Whether the resulting lot would be LONG-term if disposed at the short op's own date (a cheap,
    /// already-shipped `is_long_term` read — no new tax logic). `None` until an era is picked.
    pub fn is_long_term_at_short_date(&self) -> Option<bool> {
        self.holding_date()
            .map(|acquired| is_long_term(acquired, self.shortfall.date))
    }

    // ★ whole-branch arch M-4: `DeclareFlowState::clearance()` was DELETED here. It had no production
    // caller — only its own test — and its doc already conceded that the real (and only) declare gate is
    // `declare_flow_confirm`'s FRESH `plan_declare` at the Confirm-step Enter, which surfaces the refusal
    // with its reason (DFW-D5/DFW-D1 "no second gating authority"). Keeping a same-shaped pure probe
    // alive alongside that gate was a standing invitation to grow a second, drifting authority; a test
    // that needs the probe calls `btctax_cli::plan_declare` directly, exactly as the confirm tail does.

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
        // No era picked ⇒ no window ⇒ nothing to preview (fail-closed; never previews a window the
        // filer did not choose).
        let Some((window_start, window_end)) = self.window() else {
            return;
        };
        let year = self.shortfall.date.year();
        let flavor = declare_preview_saving(
            events,
            prices,
            cfg,
            tables,
            self.sat,
            self.wallet.clone(),
            window_start,
            window_end,
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

    lines.push(format!("sat: {}   wallet: {:?}", state.sat, state.wallet));

    // ★ OWNER DECISION / KAT (a): the era picker — NOTHING pre-selected. Mirrors the BG-D5 provenance
    // picker (`render_promote_flow`'s Provenance arm): a marked list, an explicit "(none yet — press
    // 1-N)" line, and no value supplied by the tool.
    lines.push(
        "When did you acquire these coins? This is YOUR attestation about YOUR coins — the tool cannot \
         and will not answer it for you. It sets the declared lot's acquisition date, and with it \
         whether the disposal it covers is SHORT- or LONG-term."
            .to_string(),
    );
    let before_op = before_op_date(&state.shortfall);
    for (i, preset) in ALL_PRESETS.iter().enumerate() {
        let (start, end) = era_window(*preset);
        let marker = if state.preset == Some(*preset) {
            '>'
        } else {
            ' '
        };
        // Presets that START after the DFW-D5 before-op boundary cannot apply to this shortfall at all
        // (★ tax Minor 1) — say so on the picker rather than letting the filer pick a dead end.
        let applies = if start <= before_op {
            String::new()
        } else {
            format!("   — cannot apply: starts after {before_op}")
        };
        lines.push(format!(
            "{marker} [{}] {start} .. {end}  ({preset:?}){applies}",
            i + 1
        ));
    }
    lines.push(match state.preset {
        Some(p) => format!("era: {p:?}"),
        None => format!("era: (none yet — press 1-{})", ALL_PRESETS.len()),
    });
    if let Some(e) = &state.era_error {
        lines.push(e.clone());
    }

    match state.window() {
        Some((start, end)) => lines.push(format!(
            "window: {start} .. {end}  (attest this as YOUR OWN knowledge of when you acquired these \
             coins — the window's substance is the filer's attestation, never tool-sourced)"
        )),
        None => lines.push(format!(
            "window: (not chosen yet) .. {} — the latest day a lot could still cover this disposal \
             (DFW-D5); the start comes from the era YOU pick above",
            state.window_end
        )),
    }

    // The cheap trio (DFW-D9/D10): floor + Coverage (or its can-never-promote refusal) + holding-date.
    // Nothing is scored until the filer has answered the era question.
    match state.floor_readout(prices) {
        None => lines.push(
            "floor / coverage / holding date: pick an era above — none of them exists until you say \
             when you acquired these coins."
                .to_string(),
        ),
        Some(Ok(cf)) => {
            let term = if state.is_long_term_at_short_date() == Some(true) {
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
        Some(Err(PromoteRefusal::NoCoverage)) => {
            lines.push(
                "floor: NOT COMPUTABLE — no price data covers this window at all. Declaring at $0 is \
                 still fine, but this tranche could never later be promoted from this window."
                    .to_string(),
            );
        }
        Some(Err(PromoteRefusal::PartialCoverage)) => {
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
            lines.push(format!(
                "[1-{}] choose era  [h/l] window_start ∓1d  [j/k] window_end ∓1d  [+/-] sat ±1000  \
                 [t] compute tax-Δ  [Enter] review & confirm  [Esc] cancel",
                ALL_PRESETS.len()
            ));
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

    /// Drive the era step the way the filer now must: open the flow, then ACTIVELY pick the oldest
    /// bucket. Every test that needs a window goes through this (there is no default any more).
    fn opened_with_oldest_era(sf: Shortfall) -> DeclareFlowState {
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        state.select_preset(ALL_PRESETS[0]);
        assert_eq!(
            state.preset,
            Some(ALL_PRESETS[0]),
            "fixture: the pick landed"
        );
        state
    }

    // ── ★ OWNER DECISION (era table): NOTHING is pre-selected; an explicit pick is required ──────────
    //
    // KAT (a) fresh flow has NO preset and renders as such · (b) advancing/confirming without a pick is
    // refused, fail-closed · (c) after a pick the window seeds correctly AND the before-op clamp still
    // governs · (d) the pick survives a bounce.

    /// KAT (a). On the `$0`-declare branch the window's only filing-substantive effect is the HOLDING
    /// PERIOD (`window_end` IS the lot's acquisition date, `resolve.rs:~1310`), so pre-selecting the
    /// OLDEST bucket answered "when did you acquire these?" for the filer — in the taxpayer-favorable
    /// (long-term) direction — which the answered-ness invariant forbids.
    #[test]
    fn a_fresh_declare_flow_has_no_era_selected_and_renders_as_such() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), false);
        assert_eq!(state.preset, None, "★ no era may be pre-selected");
        assert_eq!(
            state.window_start, None,
            "★ no window_start may exist before the filer picks an era"
        );
        assert_eq!(state.window(), None);
        assert_eq!(state.holding_date(), None);
        assert_eq!(state.is_long_term_at_short_date(), None);

        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.contains(&format!("(none yet — press 1-{})", ALL_PRESETS.len())),
            "the unanswered era must render as explicitly unanswered: {rendered}"
        );
        assert!(
            rendered.contains("(not chosen yet)"),
            "no window may be implied before a pick: {rendered}"
        );
        // Every bucket is offered, numbered — the filer picks, the tool does not order-of-preference
        // one of them into a de-facto default.
        for (i, p) in ALL_PRESETS.iter().enumerate() {
            assert!(
                rendered.contains(&format!("[{}] ", i + 1)) && rendered.contains(&format!("{p:?}")),
                "preset {p:?} must be offered as [{}]: {rendered}",
                i + 1
            );
        }
        assert!(
            !rendered.contains("long-term at the short op") && !rendered.contains("short-term at"),
            "no holding-period CHARACTER may be asserted before the filer answers: {rendered}"
        );
    }

    /// KAT (b): fail-closed. `review()` (the Edit-step Enter) must not advance, and the flow must stay
    /// exactly where it was — no window invented, nothing recorded.
    #[test]
    fn advancing_without_picking_an_era_is_refused_and_records_nothing() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf.clone(), wallet(), false);
        let before = state.clone();

        state.review();

        assert_eq!(
            state.step,
            DeclareFlowStep::Edit,
            "★ Enter with no era picked must NOT advance to Confirm"
        );
        assert_eq!(
            state.preset, None,
            "and must not answer the question itself"
        );
        assert_eq!(state.window_start, None);
        assert_eq!(
            state.window_end, before.window_end,
            "the DFW-D5 prefill is untouched by a refused advance"
        );
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.contains("the tool will not answer it for you"),
            "the refusal must insist the FILER answer: {rendered}"
        );
    }

    /// KAT (c): after an explicit pick the window seeds from the preset AND the DFW-D5 before-op clamp
    /// still governs `window_end` — for every bucket that can apply at all.
    #[test]
    fn picking_an_era_seeds_the_window_and_the_before_op_clamp_still_governs() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let before_op = date!(2020 - 06 - 14);
        for &p in &ALL_PRESETS {
            let (start, end) = era_window(p);
            let mut state = DeclareFlowState::new(sf.clone(), wallet(), false);
            state.select_preset(p);
            if start > before_op {
                // Cannot apply — refused, not landed on (see the degenerate-window KAT below).
                assert_eq!(state.preset, None, "{p:?} must be refused, not applied");
                continue;
            }
            assert_eq!(state.preset, Some(p));
            assert_eq!(
                state.window_start,
                Some(start),
                "{p:?}: window_start seeds from the preset's own start"
            );
            assert_eq!(
                state.window_end,
                end.min(before_op),
                "{p:?}: the DFW-D5 before-op clamp governs on conflict"
            );
            assert!(
                state.window_end < sf.date,
                "{p:?}: window_end must stay strictly before the short op's date"
            );
            assert!(state.window_start.unwrap() <= state.window_end);
        }
    }

    /// KAT (d): the pick lives on the state (not inside `step`), so a Confirm→Edit bounce — an `Esc`,
    /// or a `plan_declare` refusal at the confirm tail — shows the filer what they already answered
    /// instead of silently re-asking (mirrors `PromoteFlowState::provenance`).
    #[test]
    fn the_era_pick_survives_a_confirm_to_edit_bounce() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
        state.select_preset(EraPreset::Y2015To2017);
        state.review();
        assert_eq!(state.step, DeclareFlowStep::Confirm);

        // The bounce (what `handle_declare_flow_key`'s Esc arm and `declare_flow_confirm`'s refusal
        // arm both do).
        state.step = DeclareFlowStep::Edit;

        assert_eq!(state.preset, Some(EraPreset::Y2015To2017));
        assert_eq!(state.window_start, Some(date!(2015 - 01 - 01)));
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.contains("era: Y2015To2017") && rendered.contains("> [3]"),
            "the surviving pick must render as selected: {rendered}"
        );
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
        let mut state = DeclareFlowState::new(
            shortfall_on(date!(2020 - 06 - 15)),
            wallet(),
            in_force_allocation_exists(&events),
        );
        state.select_preset(ALL_PRESETS[0]);
        let (window_start, window_end) = state.window().expect("the fixture picked an era");
        let prices = StaticPrices::default();
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            !rendered.to_lowercase().contains("will be refused"),
            "no refusal may be claimed for a declare plan_declare accepts: {rendered}"
        );

        // And the engine agrees: this declare is NOT refused by the allocation guard. (★ whole-branch
        // arch M-4: calls the REAL gate — `plan_declare`, the same one `declare_flow_confirm` runs —
        // directly, now that the same-shaped `DeclareFlowState::clearance` probe is gone.)
        let now = datetime!(2026 - 01 - 01 0:00 UTC);
        let refusal = btctax_cli::plan_declare(
            &events,
            &prices,
            &cfg(),
            state.sat,
            state.wallet.clone(),
            window_start,
            window_end,
            Some(state.shortfall.event.clone()),
            now,
        )
        .err();
        let text = format!("{refusal:?}").to_lowercase();
        assert!(
            !text.contains("safe-harbor") && !text.contains("safe harbor"),
            "plan_declare must not refuse this on safe-harbor grounds: {refusal:?}"
        );
    }

    // ── (b): declare-flow prefill puts window_end before the disposal + the source wallet ────────────

    /// ★ The DFW-D5 prefill is INDEPENDENT of the era pick and survives the explicit-pick change: from
    /// the moment the flow opens, `window_end` is strictly before the short op's date and `wallet` is
    /// the short op's source pool — with NO era answered.
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
        assert_eq!(
            state.window_end,
            date!(2020 - 06 - 14),
            "with no era picked the prefill IS the before-op day — not a preset's end, and not a \
             window the filer never chose"
        );
        assert_eq!(state.preset, None, "★ and no era is answered for them");
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
        let state = opened_with_oldest_era(sf.clone());
        let (preset_start, preset_end) = era_window(EraPreset::Y2009To2011);
        assert_eq!(
            state.window_start,
            Some(preset_start),
            "window_start still seeds from the preset the filer picked"
        );
        assert!(
            state.window_end < preset_end,
            "the preset's raw end ({preset_end}) conflicts with DFW-D5 — the before-op clamp must win, \
             not the preset's own end"
        );
        assert_eq!(state.window_end, date!(2010 - 05 - 31));
    }

    #[test]
    fn select_preset_reseeds_the_window_and_invalidates_the_stale_tax_delta() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
        state.tax_delta = Some(SavingFlavor::Named("stub".to_string()));
        state.select_preset(EraPreset::Y2012To2014);
        assert_eq!(state.preset, Some(EraPreset::Y2012To2014));
        let (start, end) = era_window(EraPreset::Y2012To2014);
        assert_eq!(state.window_start, Some(start));
        assert_eq!(state.window_end, end);
        assert!(
            state.tax_delta.is_none(),
            "re-picking an era must blank the stale tax-Δ"
        );
    }

    // ── (e): editing the window blanks the on-demand saving ("stale — recompute") ─────────────────────

    #[test]
    fn nudging_window_start_blanks_the_on_demand_saving() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
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

    /// ★ OWNER DECISION corollary: a window nudge cannot bring a window into existence — that would be
    /// the era question answered by a side door (a `window_start` the filer never chose).
    #[test]
    fn nudging_before_any_era_is_picked_is_inert() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = DeclareFlowState::new(sf, wallet(), false);
        let before = state.clone();
        state.nudge_window_start(-1);
        state.nudge_window_start(1);
        state.nudge_window_end(-1);
        state.nudge_window_end(1);
        assert_eq!(
            state, before,
            "no window edit may materialize a window before the filer picks an era"
        );
    }

    // ── T8-review Minor-2: nudge_window_start is bounded (never past window_end, never before genesis) ─

    #[test]
    fn nudging_window_start_never_crosses_past_window_end_or_before_genesis() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
        // Pin window_end close to window_start so a large positive nudge would otherwise cross it.
        state.window_end = date!(2009 - 01 - 10);
        state.nudge_window_start(9_999);
        assert_eq!(
            state.window_start,
            Some(state.window_end),
            "window_start must clamp AT window_end, never past it: {:?}",
            state.window_start
        );

        // A large negative nudge must clamp at Bitcoin's genesis block, never before it.
        state.nudge_window_start(-3_650);
        assert_eq!(
            state.window_start,
            Some(date!(2009 - 01 - 03)),
            "window_start must clamp at Bitcoin's genesis block (2009-01-03), never before it"
        );
    }

    #[test]
    fn nudging_window_end_blanks_the_on_demand_saving_and_never_crosses_the_before_op_boundary() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
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
        let mut state = opened_with_oldest_era(sf);
        state.tax_delta = Some(SavingFlavor::Named("stub".to_string()));
        state.nudge_sat(-100_000_000_000);
        assert!(state.tax_delta.is_none());
        assert_eq!(state.sat, 1, "sat must never go to 0 or negative");
    }

    // ── (c): Coverage::Partial/NoCoverage refusal surfaces live in the readout ────────────────────────

    #[test]
    fn no_price_coverage_at_all_surfaces_as_no_coverage_live() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = opened_with_oldest_era(sf);
        let empty_prices = StaticPrices::default();
        assert_eq!(
            state.floor_readout(&empty_prices),
            Some(Err(PromoteRefusal::NoCoverage))
        );
    }

    /// ★ OWNER DECISION: no floor/coverage/holding-date is scored — or rendered — before the filer has
    /// answered the era question. `None` is "unanswered", never a computed-looking readout.
    #[test]
    fn no_floor_is_scored_before_an_era_is_picked() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let state = DeclareFlowState::new(sf, wallet(), false);
        let prices = StaticPrices::default();
        assert_eq!(state.floor_readout(&prices), None);
        let rendered = render_declare_flow(&state, &prices).join("\n");
        assert!(
            rendered.contains("pick an era above"),
            "the cheap trio must say WHY it is absent: {rendered}"
        );
        assert!(
            !rendered.contains("floor (if later promoted)"),
            "no floor may be quoted for a window the filer did not choose: {rendered}"
        );
    }

    #[test]
    fn a_gap_in_the_window_surfaces_as_partial_coverage_live() {
        let sf = shortfall_on(date!(2020 - 01 - 10));
        let state = opened_with_oldest_era(sf);
        let (window_start, window_end) = state.window().unwrap();
        // Price data on window_start and window_end only — a gap in between.
        let mut m = BTreeMap::new();
        m.insert(window_start, rust_decimal_macros::dec!(10_000));
        m.insert(window_end, rust_decimal_macros::dec!(10_000));
        let gappy = StaticPrices(m);
        assert_eq!(
            state.floor_readout(&gappy),
            Some(Err(PromoteRefusal::PartialCoverage))
        );
    }

    #[test]
    fn full_price_coverage_surfaces_a_computed_floor_live() {
        let sf = shortfall_on(date!(2020 - 01 - 10));
        let state = opened_with_oldest_era(sf);
        let (window_start, window_end) = state.window().unwrap();
        let mut m = BTreeMap::new();
        let mut d = window_start;
        loop {
            m.insert(d, rust_decimal_macros::dec!(10_000));
            if d == window_end {
                break;
            }
            d = d.next_day().unwrap();
        }
        let full = StaticPrices(m);
        let cf = state
            .floor_readout(&full)
            .expect("an era is picked")
            .expect("full coverage computes");
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

    // ★ whole-branch arch M-4 / N-r2-4(a): `clearance_reflects_plan_declare_and_is_a_pure_read` was
    // DELETED with the `DeclareFlowState::clearance` probe it was the sole caller of. Its two r1 Nits
    // (the tautological `empty_events.len() == 0` assert over a shared `&[LedgerEvent]` the borrow
    // checker already protects, and the two `let _ = ...` import-keepers that tested nothing) died with
    // it. The behavior that MATTERS — the DFW-D5.2 clearance shadow itself — is pinned where it actually
    // runs: `chokepoint::plan_declare`'s own KATs (`btctax-cli`) and the Confirm-step tail below.

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
        let mut state = opened_with_oldest_era(sf);
        state.window_start = Some(date!(2023 - 01 - 01));
        state.window_end = date!(2023 - 01 - 03);

        let mut m = BTreeMap::new();
        let mut d = state.window_start.unwrap();
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
        let mut state = opened_with_oldest_era(sf);
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
    fn no_era_pick_can_ever_leave_an_inverted_window() {
        // A 2018 shortfall: every preset from Y2018To2020 on STARTS after the before-op clamp bites.
        let sf = shortfall_on(date!(2018 - 06 - 01));
        for &p in &ALL_PRESETS {
            let mut state = opened_with_oldest_era(sf.clone());
            state.select_preset(p);
            let (start, end) = state
                .window()
                .expect("the oldest-era fixture leaves a window");
            assert!(
                start <= end,
                "picking {p:?} left an INVERTED window: {start} .. {end}"
            );
            assert!(
                end < state.shortfall.date,
                "the DFW-D5 before-op clamp must still hold after picking {p:?}"
            );
            // And the readout never blames missing price data for an incoherent window.
            let prices = StaticPrices::default();
            let rendered = render_declare_flow(&state, &prices).join("\n");
            assert!(!rendered.is_empty());
        }
    }

    /// ★ tax Minor 1 (P-C gate r2), preserved in its STRONGER form. For a 2018-06-01 shortfall
    /// (before-op = 2018-05-31), `Y2021To2024`/`Y2025Onward` start strictly AFTER the before-op
    /// boundary: applying one would collapse `window_start == window_end == 2018-05-31` — a ONE-DAY
    /// window, the HIGHEST floor the flow can produce (the LEAST conservative direction, opposite
    /// DFW-D9's "wider window → lower floor") forcing a SHORT-term holding date.
    ///
    /// The old Tab-cycling SKIPPED such presets silently. An explicit pick cannot silently skip — a
    /// skip would apply a DIFFERENT era than the one the filer pressed, which is the same
    /// answering-for-the-filer defect one layer down. So the pick is REFUSED, with the reason, and any
    /// previous pick is left intact (fail-closed).
    #[test]
    fn picking_an_era_that_cannot_apply_is_refused_never_collapsed_to_a_degenerate_day() {
        let sf = shortfall_on(date!(2018 - 06 - 01));
        let before_op = date!(2018 - 05 - 31);
        for &p in &ALL_PRESETS {
            let (preset_start, _) = era_window(p);
            if preset_start <= before_op {
                continue;
            }
            // From a clean flow: refused, and NOTHING is answered on the filer's behalf.
            let mut fresh = DeclareFlowState::new(sf.clone(), wallet(), false);
            fresh.select_preset(p);
            assert_eq!(
                fresh.preset, None,
                "{p:?} starts {preset_start}, after this shortfall's before-op boundary \
                 ({before_op}) — it cannot apply and must be REFUSED, never applied"
            );
            assert_eq!(fresh.window_start, None);
            let prices = StaticPrices::default();
            let rendered = render_declare_flow(&fresh, &prices).join("\n");
            assert!(
                rendered.contains("cannot apply to this shortfall"),
                "the refusal must say WHY: {rendered}"
            );

            // From a flow that already has a pick: the previous, VALID answer survives untouched.
            let mut picked = opened_with_oldest_era(sf.clone());
            let before = picked.clone();
            picked.select_preset(p);
            assert_eq!(
                picked.preset, before.preset,
                "a refused pick must not disturb the filer's existing answer"
            );
            assert_eq!(picked.window_start, before.window_start);
            assert_eq!(picked.window_end, before.window_end);
            assert_ne!(
                picked.window_start,
                Some(picked.window_end),
                "{p:?} must never collapse the window to a single degenerate day"
            );
        }
    }

    #[test]
    fn nudging_window_end_down_never_crosses_below_window_start() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
        state.nudge_window_end(-9_999);
        assert_eq!(
            Some(state.window_end),
            state.window_start,
            "window_end must clamp AT window_start, never below it"
        );
        assert!(state.window_start.unwrap() <= state.window_end);
    }

    // ── ★ tax M-2 (SPEC DFW-D8): the over-coverage confirm-note ──────────────────────────────────────

    #[test]
    fn declaring_more_sat_than_the_shortfall_needs_carries_a_confirm_note() {
        let sf = shortfall_on(date!(2020 - 06 - 15));
        let mut state = opened_with_oldest_era(sf);
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
        let mut state = opened_with_oldest_era(sf);
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
        let mut state = opened_with_oldest_era(sf);
        // Force a window_end more than a year before the short op's date — long-term.
        state.window_end = date!(2020 - 06 - 01);
        assert_eq!(state.holding_date(), Some(date!(2020 - 06 - 01)));
        assert_eq!(state.is_long_term_at_short_date(), Some(true));

        // A window_end just before the short op's date — short-term.
        state.window_end = date!(2021 - 12 - 31);
        assert_eq!(state.is_long_term_at_short_date(), Some(false));
    }
}
