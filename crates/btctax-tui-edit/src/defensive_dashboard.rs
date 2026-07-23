//! The Defensive Filing Wizard dashboard (Task 7, Phase P-B): a READ-ONLY, derived render of
//! `btctax_core::defensive::journey_view` plus the key-dispatch SCAFFOLDING the Task 8-10 flows plug
//! into. Per DFW-D1(c) this module is a THIN driver — it derives no tax logic and holds no second
//! gating authority; every number/advisory shown here is `journey_view`'s own output, rendered verbatim.
//!
//! **READ-ONLY + DISPATCH ONLY (C-3).** This file contains no `chokepoint::apply_*` call and no
//! write-class token — KAT-G1's `kat_g1_mechanized_source_gate` (`edit/persist.rs:1897`) enforces this
//! mechanically for every non-test line in this crate outside `edit/persist.rs`.
//!
//! **The flow-launch seam (Tasks 8-10).** `declare_flow`/`promote_flow` (Phase P-C) and the chokepoint-
//! driven export step (Phase P-D, Task 10) do not exist yet. [`handle_defensive_dashboard_key`] therefore
//! only NAMES the intent a key press represents ([`DashboardIntent`]) and moves the cursor — it never
//! opens a flow, never calls a chokepoint, and mutates nothing but its own `cursor` field. The future
//! `main.rs` wiring matches `DashboardIntent::{Declare,Promote,RouteResolveFirst,Export}` and opens the
//! corresponding flow/chokepoint step once those tasks land.

use btctax_core::defensive::discovery::{Shortfall, Triage};
use btctax_core::defensive::{Advisory, DefensiveFilingView, PoolShort, TrancheRow, TrancheStatus};
use btctax_core::{BlockerKind, EventId};
use crossterm::event::{KeyCode, KeyEvent};

/// Per-screen UI-only state for `EditorScreen::DefensiveFiling`: the ONE `journey_view` computed at
/// entry (`EditorApp::open_defensive_filing`, DFW-D6 gate) plus a row cursor for
/// [`handle_defensive_dashboard_key`]. Nothing here is ever written to a chokepoint (C-3) — it is a
/// derived read plus pure UI navigation, exactly like every other screen's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefensiveDashboardState {
    pub view: DefensiveFilingView,
    pub cursor: usize,
}

impl DefensiveDashboardState {
    pub fn new(view: DefensiveFilingView) -> Self {
        Self { view, cursor: 0 }
    }
}

/// The row-address the cursor can occupy, in FIXED display order — MUST mirror [`render_dashboard`]'s
/// own section order (resolve-first, candidates, tranches, still-short) so `d`/`p`/Enter act on the row
/// the filer is actually looking at. `x` (export) needs no row — DFW-D3/M-5: always available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DashRow {
    ResolveFirst(usize),
    Candidate(usize),
    Tranche(usize),
    PoolShort(usize),
}

fn row_order(view: &DefensiveFilingView) -> Vec<DashRow> {
    let mut rows = Vec::new();
    rows.extend((0..view.resolve_first.len()).map(DashRow::ResolveFirst));
    rows.extend((0..view.candidates.len()).map(DashRow::Candidate));
    rows.extend((0..view.tranches.len()).map(DashRow::Tranche));
    rows.extend((0..view.still_short.len()).map(DashRow::PoolShort));
    rows
}

/// The intent a dashboard key press names — SCAFFOLDING (the Task 8-10 seam). `Declare`/`Promote` name
/// the flows Tasks 8-9 build; `Export` names Task 10's chokepoint-driven export step; `RouteResolveFirst`
/// names the shipped remedial flow DFW-D4 already routes a resolve-first shortfall to (classify /
/// set-fmv / reconcile, depending on the open blocker kind). Producing an intent mutates ONLY
/// `DefensiveDashboardState::cursor` — no `EditorApp` flow field, no chokepoint, no write (C-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardIntent {
    None,
    Declare(EventId),
    Promote(EventId),
    Export,
    RouteResolveFirst(EventId),
}

/// Key-dispatch over the dashboard's OWN cursor + the ALREADY-COMPUTED `journey_view` (never
/// independently re-gated — DFW-D1 "no second gating authority": every intent this fn returns is a
/// pointer into `state.view`, not a re-derived judgement). `x` is recognized unconditionally, from ANY
/// cursor position or dashboard state (DFW-D3/M-5: export is never a "done" checkbox that could be
/// missing or disabled).
pub fn handle_defensive_dashboard_key(
    state: &mut DefensiveDashboardState,
    key: KeyEvent,
) -> DashboardIntent {
    if key.code == KeyCode::Char('x') {
        return DashboardIntent::Export;
    }

    let rows = row_order(&state.view);

    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            if !rows.is_empty() {
                state.cursor = (state.cursor + 1).min(rows.len() - 1);
            }
            DashboardIntent::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.cursor = state.cursor.saturating_sub(1);
            DashboardIntent::None
        }
        KeyCode::Char('d') => match rows.get(state.cursor) {
            Some(DashRow::Candidate(i)) => {
                DashboardIntent::Declare(state.view.candidates[*i].event.clone())
            }
            _ => DashboardIntent::None,
        },
        KeyCode::Char('p') => match rows.get(state.cursor) {
            Some(DashRow::Tranche(i)) => {
                let row = &state.view.tranches[*i];
                if row.status == TrancheStatus::DeclaredZero {
                    DashboardIntent::Promote(row.target.clone())
                } else {
                    // Already promoted — DFW-D3: nothing left to fork on this row.
                    DashboardIntent::None
                }
            }
            _ => DashboardIntent::None,
        },
        KeyCode::Enter => match rows.get(state.cursor) {
            Some(DashRow::ResolveFirst(i)) => match &state.view.resolve_first[*i] {
                Triage::ResolveFirst { shortfall, .. } => {
                    DashboardIntent::RouteResolveFirst(shortfall.event.clone())
                }
                _ => DashboardIntent::None,
            },
            _ => DashboardIntent::None,
        },
        _ => DashboardIntent::None,
    }
}

// ── Render (pure; no ratatui dependency here — draw_edit.rs wraps these lines in a Paragraph) ────────

fn render_candidate_row(s: &Shortfall) -> String {
    format!(
        "declare candidate — {:?}: short {} sat on {} — no acquisition record covers this; press 'd' \
         to declare a $0 tranche",
        s.event, s.short_sat, s.date
    )
}

/// DFW-D4 resolve-data-first ordering: an open acquisition-shaped blocker stands behind this shortfall —
/// route to ITS shipped remedy BEFORE ever offering declare.
fn render_resolve_first_row(shortfall: &Shortfall, blocker: BlockerKind) -> String {
    format!(
        "resolve first — {:?}: an open {:?} blocker on the same pool/timeframe may still supply these \
         {} sat — resolve it first (press Enter to route to its remedy), THEN reconsider declaring",
        shortfall.event, blocker, shortfall.short_sat
    )
}

/// ★ arch-m-3 (I-5(b) render): the pool-level "still short" state — ONE combined row, never a
/// per-tranche attribution (mirrors `journey_view`'s own `still_short` composition).
fn render_pool_short_row(ps: &PoolShort) -> String {
    format!(
        "{:?}: a tranche of {} sat is live here but this pool is still short by {} sat — don't declare \
         again (review the window/wallet instead)",
        ps.pool, ps.live_tranche_sat, ps.short_sat
    )
}

fn render_advisory_line(a: &Advisory) -> String {
    match a {
        // DFW-D5.3 void+re-declare copy, verbatim (SPEC.md D5).
        Advisory::OverCovered { by_sat } => format!(
            "  [advisory] this tranche is larger than the shortfall it covers by {by_sat} sat — if a \
             later import supplied those coins, promoting files an estimated basis on documented coins \
             (understated gain); void + re-declare at the covered size. If these are genuinely your \
             no-records coins, promoting is fine."
        ),
        Advisory::NowDisplacing => "  [advisory] this recorded promote is now displacing documented \
             basis on a real disposal — review before relying on it further"
            .to_string(),
        Advisory::MethodInversion(msg) => format!("  [advisory] {msg}"),
        Advisory::TrancheDip(msg) => format!("  [advisory] {msg}"),
        Advisory::FeeOnlyPromoteNoop => "  [advisory] the shortfall(s) this tranche covers are all \
             fee-component — promoting would only ever substantiate fee-sat basis, never principal"
            .to_string(),
        // ★ Task 8 / P-B-tax-Minor: caveat a displacement-driven gain-Δ — never shown as an unqualified
        // "saving".
        Advisory::WouldDisplaceIfPromoted => "  [advisory] promoting this tranche would displace \
             documented basis on a real disposal (a HIFO reorder) — any saving/gain-\u{394} shown above \
             would UNDERSTATE the gain a documented lot would actually realize; treat it as a caveat, \
             not a straightforward saving"
            .to_string(),
    }
}

/// DFW-D3 fork: the `$0`/promote choice rendered as TWO EQUAL branches. `[current]` is never
/// "incomplete" — a `$0`-declared tranche IS complete (revocable-until-promoted, DFW-D3/tax-M-4 carve).
/// `[optional]` is never a default (G-1) — and it is SUPPRESSED/annotated (tax-N-1) when this tranche's
/// only coverage is fee-component (promoting a fee-only cover yields ~$0: fee-sats draw acquisition-date
/// FIFO, method-independent, and BG-D4's fee-evaporation forfeits the estimate component).
fn render_declared_zero_fork(row: &TrancheRow) -> Vec<String> {
    let fee_only = row
        .advisories
        .iter()
        .any(|a| matches!(a, Advisory::FeeOnlyPromoteNoop));
    let mut lines = vec!["  [current] filed $0 — complete (revocable until promoted)".to_string()];
    if fee_only {
        lines.push(
            "  [optional, SUPPRESSED] promote: this tranche covers only a fee-only shortfall — \
             fee-sats draw acquisition-date FIFO (method-independent) and would yield ~$0 from \
             promoting; not recommended"
                .to_string(),
        );
    } else {
        lines.push(
            "  [optional] promote: file the computed floor as documented basis instead of $0 (press \
             'p')"
                .to_string(),
        );
    }
    lines
}

fn render_tranche_row(row: &TrancheRow) -> Vec<String> {
    let status_word = match row.status {
        TrancheStatus::DeclaredZero => "declared",
        TrancheStatus::Promoted => "promoted",
    };
    let mut lines = vec![format!(
        "tranche {:?} — {} sat ({status_word}):",
        row.target, row.sat
    )];

    match row.status {
        TrancheStatus::DeclaredZero => lines.extend(render_declared_zero_fork(row)),
        TrancheStatus::Promoted => {
            lines.push("  [current] promoted — basis filed >$0 (no longer revocable)".to_string());
        }
    }

    for a in &row.advisories {
        lines.push(render_advisory_line(a));
    }

    lines
}

/// ★ arch-Minor1: prefix `"> "` on `text` iff `idx == cursor` — the ONE marker primitive
/// `render_dashboard` uses at every cursor-addressable row, so `j`/`k` movement (already wired,
/// Task 7) is VISIBLE before `d`/`p`/Enter act on the cursor row.
fn mark_row(idx: usize, cursor: usize, text: String) -> String {
    if idx == cursor {
        format!("> {text}")
    } else {
        text
    }
}

/// The full dashboard render — a pure derived text render of `journey_view`'s own output. Section
/// order (resolve-first, candidates, tranches, still-short) MUST mirror `row_order`'s cursor addressing.
/// `cursor` (★ arch-Minor1) is `DefensiveDashboardState::cursor` — the SAME index `row_order` addresses,
/// so the marked line is always the row `d`/`p`/Enter would act on.
///
/// DFW-D3/M-5: the `[x] export` line is pushed UNCONDITIONALLY, last — always available, regardless of
/// dashboard state, and never phrased as a "done" checkbox (exports write files, not events).
pub fn render_dashboard(view: &DefensiveFilingView, cursor: usize) -> Vec<String> {
    let mut lines = vec![
        "Defensive Filing — journey dashboard (derived; nothing here is filed until you act)"
            .to_string(),
        String::new(),
    ];
    let mut row_idx = 0usize;

    if !view.resolve_first.is_empty() {
        lines.push("Resolve first:".to_string());
        for t in &view.resolve_first {
            if let Triage::ResolveFirst { shortfall, blocker } = t {
                lines.push(mark_row(
                    row_idx,
                    cursor,
                    render_resolve_first_row(shortfall, *blocker),
                ));
                row_idx += 1;
            }
        }
        lines.push(String::new());
    }

    if !view.candidates.is_empty() {
        lines.push("Declare candidates:".to_string());
        for s in &view.candidates {
            lines.push(mark_row(row_idx, cursor, render_candidate_row(s)));
            row_idx += 1;
        }
        lines.push(String::new());
    }

    if !view.tranches.is_empty() {
        lines.push("Declared tranches:".to_string());
        for row in &view.tranches {
            let mut rendered = render_tranche_row(row);
            if let Some(first) = rendered.first_mut() {
                *first = mark_row(row_idx, cursor, std::mem::take(first));
            }
            lines.extend(rendered);
            row_idx += 1;
        }
        lines.push(String::new());
    }

    if !view.still_short.is_empty() {
        lines.push("Still short:".to_string());
        for ps in &view.still_short {
            lines.push(mark_row(row_idx, cursor, render_pool_short_row(ps)));
            row_idx += 1;
        }
        lines.push(String::new());
    }

    if view.safe_harbor_blocked {
        lines.push(
            "Note: an in-force safe-harbor allocation or a pre-2025 tranche is present — the two are \
             mutually exclusive."
                .to_string(),
        );
        lines.push(String::new());
    }

    if view.resolve_first.is_empty()
        && view.candidates.is_empty()
        && view.tranches.is_empty()
        && view.still_short.is_empty()
    {
        lines.push("Nothing outstanding right now.".to_string());
        lines.push(String::new());
    }

    lines.push("[x] export — plan the IRS-PDF packet (always available)".to_string());

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::form::{
        SelectLotsFlowState, SelectLotsStep, TargetList, VoidFlowState, VoidListItem, VoidStep,
    };
    use crate::editor::{EditorApp, EditorScreen};
    use btctax_adapters::BundledTaxTables;
    use btctax_cli::CliConfig;
    use btctax_core::project::pools::PoolKey;
    use btctax_core::state::LedgerState;
    use btctax_core::{Source, SourceRef, WalletId};
    use btctax_tui::app::Snapshot;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use time::macros::date;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn wallet() -> WalletId {
        WalletId::Exchange {
            provider: "cb".into(),
            account: "m".into(),
        }
    }

    fn shortfall(seq: u64, short_sat: i64) -> Shortfall {
        Shortfall {
            event: EventId::decision(seq),
            wallet: Some(wallet()),
            date: date!(2025 - 06 - 01),
            short_sat,
            fee_sat: 0,
        }
    }

    fn tranche_row(
        seq: u64,
        sat: i64,
        status: TrancheStatus,
        advisories: Vec<Advisory>,
    ) -> TrancheRow {
        TrancheRow {
            target: EventId::decision(seq),
            sat,
            status,
            clamped_saving: vec![],
            advisories,
        }
    }

    fn empty_view() -> DefensiveFilingView {
        DefensiveFilingView {
            candidates: vec![],
            resolve_first: vec![],
            tranches: vec![],
            still_short: vec![],
            flagged_years: Default::default(),
            safe_harbor_blocked: false,
        }
    }

    // ── (b)/(c): DFW-D3 fork — $0 renders complete, never incomplete; promote is an explicit optional
    // branch ──────────────────────────────────────────────────────────────────────────────────────────

    #[test]
    fn zero_declared_tranche_renders_filed_zero_complete_never_incomplete_or_step() {
        let row = tranche_row(1, 40_000_000, TrancheStatus::DeclaredZero, vec![]);
        let rendered = render_tranche_row(&row).join("\n");
        assert!(
            rendered.contains("filed $0 — complete"),
            "a $0-declared tranche must render 'filed $0 — complete': {rendered}"
        );
        assert!(
            !rendered.to_lowercase().contains("incomplete"),
            "DFW-D3: a $0-declared tranche must NEVER render as incomplete: {rendered}"
        );
        assert!(
            !rendered.to_lowercase().contains("step "),
            "DFW-D3: a $0-declared tranche must NEVER render step-tracking copy: {rendered}"
        );
    }

    #[test]
    fn fork_renders_promote_as_explicit_optional_branch() {
        let row = tranche_row(1, 40_000_000, TrancheStatus::DeclaredZero, vec![]);
        let lines = render_tranche_row(&row);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("[current]") && l.contains("filed $0 — complete")),
            "the $0 branch must be an explicit, present-tense 'current' branch: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("[optional]") && l.to_lowercase().contains("promote")),
            "the promote branch must be rendered as an explicit OPTIONAL branch, never a default: \
             {lines:?}"
        );
    }

    #[test]
    fn promoted_tranche_renders_no_fork_and_no_revocable_claim() {
        let row = tranche_row(1, 40_000_000, TrancheStatus::Promoted, vec![]);
        let rendered = render_tranche_row(&row).join("\n");
        assert!(
            rendered.contains("promoted"),
            "a promoted tranche must say so: {rendered}"
        );
        assert!(
            !rendered.to_lowercase().contains("[optional]"),
            "tax-M-4: once promoted there is nothing left to fork on: {rendered}"
        );
    }

    // ── (e): OverCovered void+re-declare copy; fee-only suppresses/annotates the promote branch ──────

    #[test]
    fn over_covered_advisory_renders_void_and_redeclare_copy() {
        let row = tranche_row(
            1,
            100_000_000,
            TrancheStatus::DeclaredZero,
            vec![Advisory::OverCovered { by_sat: 6_000_000 }],
        );
        let rendered = render_tranche_row(&row).join("\n");
        assert!(
            rendered.contains("larger than the shortfall it covers by 6000000 sat"),
            "must name the excess sat figure: {rendered}"
        );
        assert!(
            rendered.contains("void + re-declare at the covered size"),
            "must render the DFW-D5.3 void+re-declare remedy: {rendered}"
        );
        assert!(
            rendered.contains("understated gain"),
            "must name the hazard (understated gain): {rendered}"
        );
        // The over-covered advisory is NOT a fee-only cover — the plain promote branch stays offered.
        assert!(
            rendered.contains("[optional] promote"),
            "an over-covered (non-fee-only) tranche's promote branch is NOT suppressed: {rendered}"
        );
    }

    #[test]
    fn fee_only_coverage_tranche_suppresses_promote_branch() {
        let row = tranche_row(
            1,
            1_000,
            TrancheStatus::DeclaredZero,
            vec![Advisory::FeeOnlyPromoteNoop],
        );
        let rendered = render_tranche_row(&row).join("\n");
        assert!(
            rendered.contains("SUPPRESSED"),
            "a fee-only-coverage tranche must suppress/annotate its promote branch (tax-N-1): {rendered}"
        );
        assert!(
            !rendered.contains(
                "[optional] promote: file the computed floor as documented basis \
                 instead of $0 (press 'p')"
            ),
            "the PLAIN unconditional promote invite must NOT appear on a fee-only-coverage row: \
             {rendered}"
        );
        // The $0 branch is STILL rendered as complete — suppression only touches the promote branch.
        assert!(
            rendered.contains("filed $0 — complete"),
            "the $0 branch stays complete regardless of the fee-only advisory: {rendered}"
        );
    }

    // ── (f) ★ arch-m-3: PoolShort renders "still short by S — don't declare again" ────────────────────

    #[test]
    fn pool_short_row_renders_still_short_by_dont_declare_again() {
        let ps = PoolShort {
            pool: PoolKey::Wallet(wallet()),
            short_sat: 42_000,
            live_tranche_sat: 7_000,
        };
        let rendered = render_pool_short_row(&ps);
        assert!(
            rendered.contains("still short by 42000 sat"),
            "must name the residual short sat: {rendered}"
        );
        assert!(
            rendered.to_lowercase().contains("don't declare again"),
            "must carry the 'don't declare again' guidance: {rendered}"
        );
        assert!(
            rendered.contains("a tranche of 7000 sat is live here"),
            "must name the live tranche's own residual sat (not double-counted): {rendered}"
        );
    }

    // ── ★ arch-Minor2 (P-B-tax-Minor render): WouldDisplaceIfPromoted caveat copy ─────────────────────

    #[test]
    fn would_displace_advisory_renders_the_understatement_caveat() {
        let row = tranche_row(
            1,
            40_000_000,
            TrancheStatus::DeclaredZero,
            vec![Advisory::WouldDisplaceIfPromoted],
        );
        let rendered = render_tranche_row(&row).join("\n");
        assert!(
            rendered.to_lowercase().contains("understate"),
            "must caveat the gain-\u{394} as an understatement, never an unqualified saving: {rendered}"
        );
        assert!(
            rendered.contains("[optional] promote"),
            "WouldDisplaceIfPromoted is a caveat, not a suppression — the promote branch stays offered: \
             {rendered}"
        );
    }

    // ── ★ arch-Minor1: the dashboard cursor marker ─────────────────────────────────────────────────────

    #[test]
    fn cursor_marks_only_the_addressed_row() {
        let view = DefensiveFilingView {
            candidates: vec![shortfall(1, 10_000), shortfall(2, 20_000)],
            ..empty_view()
        };
        // row_order addresses [Candidate(0), Candidate(1)] — cursor=1 is the SECOND candidate.
        let rendered = render_dashboard(&view, 1);
        let first_candidate = rendered
            .iter()
            .find(|l| l.contains("short 10000 sat"))
            .expect("first candidate row present");
        let second_candidate = rendered
            .iter()
            .find(|l| l.contains("short 20000 sat"))
            .expect("second candidate row present");
        assert!(
            !first_candidate.starts_with("> "),
            "the NON-cursor row must carry no marker: {first_candidate:?}"
        );
        assert!(
            second_candidate.starts_with("> "),
            "the cursor-addressed row must carry the '> ' marker: {second_candidate:?}"
        );
    }

    #[test]
    fn cursor_marks_the_tranche_header_line_only_not_its_advisory_lines() {
        let view = DefensiveFilingView {
            tranches: vec![tranche_row(
                1,
                40_000_000,
                TrancheStatus::DeclaredZero,
                vec![Advisory::WouldDisplaceIfPromoted],
            )],
            ..empty_view()
        };
        // row_order addresses [Tranche(0)] — cursor=0 is the (only) tranche.
        let rendered = render_dashboard(&view, 0);
        let header = rendered
            .iter()
            .find(|l| l.contains("sat (declared):"))
            .expect("tranche header present");
        assert!(
            header.starts_with("> "),
            "the tranche header line must carry the marker: {header:?}"
        );
        let advisory_line = rendered
            .iter()
            .find(|l| l.contains("[advisory]"))
            .expect("advisory line present");
        assert!(
            !advisory_line.starts_with("> "),
            "a sub-line (advisory) of the addressed row must NOT itself carry the marker: \
             {advisory_line:?}"
        );
    }

    // ── (d): x/export is ALWAYS-available, never a "done" checkbox ────────────────────────────────────

    #[test]
    fn export_is_always_available_never_a_done_checkbox() {
        // Empty dashboard: nothing to declare, nothing tranched, nothing short.
        let rendered_empty = render_dashboard(&empty_view(), 0).join("\n");
        assert!(
            rendered_empty.contains("[x] export"),
            "export must be offered even with an EMPTY dashboard: {rendered_empty}"
        );
        assert!(
            !rendered_empty.to_lowercase().contains("done"),
            "export must never be phrased as a completed/'done' checkbox: {rendered_empty}"
        );

        // A busy dashboard (candidates + tranches + still-short all present) still ends with the SAME
        // unconditional export line.
        let busy = DefensiveFilingView {
            candidates: vec![shortfall(1, 10_000)],
            resolve_first: vec![],
            tranches: vec![tranche_row(2, 5_000, TrancheStatus::DeclaredZero, vec![])],
            still_short: vec![PoolShort {
                pool: PoolKey::Wallet(wallet()),
                short_sat: 1,
                live_tranche_sat: 1,
            }],
            flagged_years: Default::default(),
            safe_harbor_blocked: false,
        };
        let rendered_busy = render_dashboard(&busy, 0).join("\n");
        assert!(
            rendered_busy.contains("[x] export"),
            "export must ALSO be offered on a busy dashboard: {rendered_busy}"
        );
        assert!(
            !rendered_busy.to_lowercase().contains("done"),
            "export must never read as 'done' even once other work exists: {rendered_busy}"
        );
    }

    // ── key-dispatch scaffolding ───────────────────────────────────────────────────────────────────────

    #[test]
    fn x_key_is_export_regardless_of_cursor_or_view_contents() {
        let mut state = DefensiveDashboardState::new(empty_view());
        assert_eq!(
            handle_defensive_dashboard_key(&mut state, key(KeyCode::Char('x'))),
            DashboardIntent::Export
        );
    }

    #[test]
    fn d_on_a_candidate_row_names_declare_intent() {
        let view = DefensiveFilingView {
            candidates: vec![shortfall(1, 10_000)],
            ..empty_view()
        };
        let mut state = DefensiveDashboardState::new(view);
        // cursor 0 is the (only) candidate row.
        assert_eq!(
            handle_defensive_dashboard_key(&mut state, key(KeyCode::Char('d'))),
            DashboardIntent::Declare(EventId::decision(1))
        );
    }

    #[test]
    fn p_on_a_declared_zero_tranche_names_promote_intent_but_not_on_a_promoted_one() {
        let view = DefensiveFilingView {
            tranches: vec![tranche_row(1, 10_000, TrancheStatus::DeclaredZero, vec![])],
            ..empty_view()
        };
        let mut state = DefensiveDashboardState::new(view);
        assert_eq!(
            handle_defensive_dashboard_key(&mut state, key(KeyCode::Char('p'))),
            DashboardIntent::Promote(EventId::decision(1))
        );

        let view_promoted = DefensiveFilingView {
            tranches: vec![tranche_row(1, 10_000, TrancheStatus::Promoted, vec![])],
            ..empty_view()
        };
        let mut state_promoted = DefensiveDashboardState::new(view_promoted);
        assert_eq!(
            handle_defensive_dashboard_key(&mut state_promoted, key(KeyCode::Char('p'))),
            DashboardIntent::None,
            "an already-promoted tranche has nothing left to fork on"
        );
    }

    #[test]
    fn enter_on_a_resolve_first_row_names_route_intent() {
        let view = DefensiveFilingView {
            resolve_first: vec![Triage::ResolveFirst {
                shortfall: shortfall(1, 10_000),
                blocker: BlockerKind::Unclassified,
            }],
            ..empty_view()
        };
        let mut state = DefensiveDashboardState::new(view);
        assert_eq!(
            handle_defensive_dashboard_key(&mut state, key(KeyCode::Enter)),
            DashboardIntent::RouteResolveFirst(EventId::decision(1))
        );
    }

    // ── (a) DFW-D6: entry refuses when pseudo-active, with routing guidance ──────────────────────────

    fn snapshot_with_pseudo_count(count: usize) -> Snapshot {
        Snapshot {
            events: vec![],
            state: LedgerState {
                pseudo_synthetic_count: count,
                ..Default::default()
            },
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            refused: BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
        }
    }

    #[test]
    fn entry_refuses_when_pseudo_active_with_routing_guidance() {
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snapshot_with_pseudo_count(3));

        app.open_defensive_filing();

        assert_eq!(
            app.screen,
            EditorScreen::Browse,
            "DFW-D6: entry must refuse (stay on Browse), never transition, while pseudo-active"
        );
        assert!(app.defensive_dashboard.is_none());
        let status = app
            .status
            .expect("a refusal must set routing guidance status");
        assert!(
            status.to_lowercase().contains("pseudo"),
            "routing guidance must name the pseudo-reconcile cause: {status}"
        );
        assert!(
            status.to_lowercase().contains("resolve") || status.to_lowercase().contains("approve"),
            "routing guidance must tell the filer what to do next: {status}"
        );
    }

    // ── ★ arch-Minor2: the residue-latch guard (mirrors ~26/35 sibling `open_*` fns) ──────────────────

    #[test]
    fn entry_refuses_when_the_rollback_failed_latch_is_set() {
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snapshot_with_pseudo_count(0));
        app.rollback_failed = true;

        app.open_defensive_filing();

        assert_eq!(
            app.screen,
            EditorScreen::Browse,
            "the residue latch must refuse entry (stay on Browse), never transition"
        );
        assert!(app.defensive_dashboard.is_none());
        let status = app
            .status
            .expect("a residue-latch refusal must set a status");
        assert!(
            status.to_lowercase().contains("quit"),
            "the residue-latch status must carry the quit-first remedy: {status}"
        );
    }

    #[test]
    fn entry_succeeds_and_computes_the_view_when_not_pseudo_active() {
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snapshot_with_pseudo_count(0));

        app.open_defensive_filing();

        assert_eq!(app.screen, EditorScreen::DefensiveFiling);
        assert!(app.defensive_dashboard.is_some());
    }

    // ── M-4: the one-flow debug assertion ─────────────────────────────────────────────────────────────

    #[test]
    #[should_panic(expected = "one-flow")]
    #[cfg(debug_assertions)]
    fn one_flow_debug_assertion_panics_when_two_flows_open_simultaneously() {
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snapshot_with_pseudo_count(0));

        // Two flows "open" at once is a contract violation elsewhere in the editor (a test-only
        // contrivance — normal dispatch order never allows it) — M-4 must catch it here rather than
        // let the dashboard silently interleave with a mid-transaction flow.
        app.void_flow = Some(VoidFlowState {
            list: TargetList::<VoidListItem>::new(Vec::new()),
            step: VoidStep::List,
        });
        app.select_lots_flow = Some(SelectLotsFlowState {
            list: TargetList::new(Vec::new()),
            step: SelectLotsStep::List,
        });

        app.open_defensive_filing();
    }

    #[test]
    fn open_flow_count_counts_exactly_the_open_flows() {
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        assert_eq!(app.open_flow_count(), 0);

        app.void_flow = Some(VoidFlowState {
            list: TargetList::<VoidListItem>::new(Vec::new()),
            step: VoidStep::List,
        });
        assert_eq!(app.open_flow_count(), 1);

        app.select_lots_flow = Some(SelectLotsFlowState {
            list: TargetList::new(Vec::new()),
            step: SelectLotsStep::List,
        });
        assert_eq!(app.open_flow_count(), 2);
    }

    // Sanity: EventId/Source/SourceRef stay imported (used by fixtures above) — silence an unused-import
    // false alarm if a future edit trims a fixture.
    #[allow(dead_code)]
    fn _unused_import_anchor() -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new("x"))
    }
}
