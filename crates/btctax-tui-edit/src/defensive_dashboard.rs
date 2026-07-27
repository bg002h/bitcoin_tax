//! The Defensive Filing Wizard dashboard (Task 7, Phase P-B): a READ-ONLY, derived render of
//! `btctax_core::defensive::journey_view` plus the key-dispatch SCAFFOLDING the Task 8-10 flows plug
//! into. Per DFW-D1(c) this module is a THIN driver — it derives no tax logic and holds no second
//! gating authority; every number/advisory shown here is `journey_view`'s own output, rendered verbatim.
//!
//! **READ-ONLY + DISPATCH ONLY (C-3).** This file contains no `chokepoint::apply_*` call and no
//! write-class token — KAT-G1's `kat_g1_mechanized_source_gate` (`edit/persist.rs:1897`) enforces this
//! mechanically for every non-test line in this crate outside `edit/persist.rs`. [`handle_defensive_
//! dashboard_key`] only NAMES the intent a key press represents ([`DashboardIntent`]) and moves the
//! cursor — it never opens a flow, never calls a chokepoint, and mutates nothing but its own `cursor`
//! field. `main.rs`'s dispatch matches `DashboardIntent::{Declare,Promote,RouteResolveFirst,Export}` and
//! opens the corresponding flow (Declare/Promote, Phase P-C) or drives the chokepoint directly (Export,
//! Task 10, Phase P-D — a degenerate one-shot action with no multi-step flow of its own; see
//! `main.rs::execute_defensive_export`). This module's own Task 10 contribution is limited to the two
//! PURE helpers below (`defensive_export_dir_for`/`render_export_status`) — the actual
//! `plan_export`/`persist_defensive_export` orchestration lives in `main.rs`, mirroring where
//! `plan_declare`/`plan_promote` are driven from (`declare_flow_confirm`/`promote_flow_confirm`), so this
//! file's READ-ONLY + DISPATCH ONLY claim above stays literally true.

use btctax_core::defensive::discovery::{Shortfall, Triage};
use btctax_core::defensive::{
    Advisory, DefensiveFilingView, PoolShort, SavingFlavor, TrancheRow, TrancheStatus,
};
use btctax_core::{BlockerKind, EventId};
use crossterm::event::{KeyCode, KeyEvent};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Per-screen UI-only state for `EditorScreen::DefensiveFiling`: the ONE `journey_view` computed at
/// entry (`EditorApp::open_defensive_filing`, DFW-D6 gate) plus a row cursor for
/// [`handle_defensive_dashboard_key`]. Nothing here is ever written to a chokepoint (C-3) — it is a
/// derived read plus pure UI navigation, exactly like every other screen's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefensiveDashboardState {
    pub view: DefensiveFilingView,
    pub cursor: usize,
    /// `Some(reason)` when `view` is a STUB — [`safe_journey_view`] could not safely call
    /// `journey_view` because the projection was pseudo-active, so every row list in `view` is an
    /// empty PLACEHOLDER, NOT a validated "nothing outstanding" (fix round 1, C-1). `render_dashboard`
    /// must render this explicitly instead of falling into its "Nothing outstanding" branch, which
    /// would read as an affirmative all-clear over an UNCOMPUTED state — and
    /// [`handle_defensive_dashboard_key`] must refuse every row-addressed action BY CONSTRUCTION while
    /// this is `Some`, not merely because the row lists happen to be empty (fix round 1, I-1).
    pub uncomputable: Option<&'static str>,
}

impl DefensiveDashboardState {
    pub fn new(view: DefensiveFilingView) -> Self {
        Self {
            view,
            cursor: 0,
            uncomputable: None,
        }
    }
}

/// The dashboard's own "could not compute" notice (fix round 1, C-1). Rendered by `render_dashboard`
/// INSTEAD of the "Nothing outstanding" fallback whenever [`DefensiveDashboardState::uncomputable`] is
/// `Some`. Says ALL of: the dashboard could not be computed; pseudo-reconcile synthetic defaults are
/// contributing; this is explicitly NOT a statement that nothing is outstanding; an uncovered disposal
/// may exist and would be MISSING from Form 8949 until it is declared (a fully-uncovered disposal never
/// reaches `state.disposals` — `fold.rs:733` — and `form_8949` iterates exactly that list,
/// `forms.rs:127-131`); the remedy; and that `x` stays reachable but re-projects fresh and may itself
/// refuse (DFW-D11) rather than acting on this stub.
pub const PSEUDO_ACTIVE_DASHBOARD_NOTICE: &str =
    "this dashboard could NOT be computed: pseudo-reconcile synthetic defaults are contributing to \
     this projection. This is NOT a statement that nothing is outstanding — an uncovered disposal may \
     exist and would be MISSING from Form 8949 until it is declared. Resolve or approve the pending \
     defaults ('P' from Browse), or turn pseudo mode off via the CLI's reconcile command, then \
     re-enter. 'x' (export) still re-projects fresh and may itself refuse.";

/// Build the dashboard's STATE WITHOUT ever calling `journey_view` on a pseudo-active state —
/// `journey_view` documents (and `debug_assert!`s) that precondition, because a Phase-B pseudo default
/// can silently mask a real shortfall this journey exists to surface, so a pseudo-active projection has
/// no trustworthy read-only view to compute at all.
///
/// Normally this can never fire: `EditorApp::open_defensive_filing`'s DFW-D6 gate refuses entry before
/// ever building a view, so `state.pseudo_active()` is always false here — EXCEPT while the stale latch
/// is armed, where D-7 (Task 6) SKIPS that gate so `w` → dashboard → `x` stays reachable even from the
/// one arming tail (pseudo-approve) that leaves the STALE image still reporting `pseudo_active()`. In
/// that one case, an unguarded `journey_view` call would hit its own precondition assert. Returning an
/// EMPTY, EXPLICITLY-`uncomputable` state instead is safe: `handle_defensive_dashboard_key`'s `x` check
/// is recognized UNCONDITIONALLY before `view` is ever read (`x` needs no row — DFW-D3/M-5), and its
/// `uncomputable` guard (fix round 1, I-1) refuses every OTHER action by construction. This also
/// protects `execute_defensive_export`'s own post-export `refresh_defensive_dashboard` call, which
/// re-projects the vault's CURRENT (not necessarily stale) state and could otherwise hit the same
/// assert if the ledger is genuinely — not just staleness-apparently — pseudo-active at that moment.
///
/// `safe_harbor_blocked` is NOT suppressed in the stub (fix round 1, C-1): unlike every other field of
/// `DefensiveFilingView`, it is a pure EVENTS-only scan (`tranche_guard::{in_force_allocation_exists,
/// pre2025_tranche_exists}` — neither takes `state`), so it is genuinely computable here and must not
/// be hard-coded `false` under a notice that already reads as a false all-clear.
pub fn safe_journey_view(
    events: &[btctax_core::LedgerEvent],
    state: &btctax_core::state::LedgerState,
    prices: &dyn btctax_core::PriceProvider,
    tables: &dyn btctax_core::TaxTables,
    cfg: &btctax_core::ProjectionConfig,
    current: i32,
) -> DefensiveDashboardState {
    if state.pseudo_active() {
        let safe_harbor_blocked = btctax_core::tranche_guard::in_force_allocation_exists(events)
            || btctax_core::tranche_guard::pre2025_tranche_exists(events);
        return DefensiveDashboardState {
            view: DefensiveFilingView {
                candidates: Vec::new(),
                resolve_first: Vec::new(),
                tranches: Vec::new(),
                still_short: Vec::new(),
                flagged_years: std::collections::BTreeSet::new(),
                safe_harbor_blocked,
            },
            cursor: 0,
            uncomputable: Some(PSEUDO_ACTIVE_DASHBOARD_NOTICE),
        };
    }
    DefensiveDashboardState::new(btctax_core::defensive::journey_view(
        events, state, prices, tables, cfg, current,
    ))
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
///
/// ★ fix round 1, I-1: while `state.uncomputable` is `Some` (the view is a pseudo-active STUB, not a
/// validated read — see [`safe_journey_view`]), every action OTHER than `x` returns `None` BY
/// CONSTRUCTION, not merely because the stub's row lists happen to be empty. Before this guard, nothing
/// stopped a future change that populated `view`'s rows while `uncomputable` stayed `Some` from handing
/// out a real `Declare`/`Promote`/`RouteResolveFirst` target derived from a state `journey_view` itself
/// asserts cannot exist.
pub fn handle_defensive_dashboard_key(
    state: &mut DefensiveDashboardState,
    key: KeyEvent,
) -> DashboardIntent {
    if key.code == KeyCode::Char('x') {
        return DashboardIntent::Export;
    }
    if state.uncomputable.is_some() {
        return DashboardIntent::None;
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

// ── Export (Task 10, Phase P-D) — pure helpers only; this file stays READ-ONLY + DISPATCH ONLY (C-3) ──
//
// The `x` action's WRITE orchestration (`btctax_cli::plan_export` → `edit::persist::persist_defensive_
// export`) lives in `main.rs` (mirrors `declare_flow_confirm`/`promote_flow_confirm`, which likewise keep
// the chokepoint calls at the dispatch layer, not inside a flow/dashboard module) — this file contributes
// only the two PURE functions below, so `render_dashboard`'s own doc claim ("no chokepoint::apply_* call
// and no write-class token") stays literally true even after Task 10.

/// Compute the DEFENSIVE-FILING multi-year export's BASE output directory from the vault path and an
/// injected timestamp — mirrors `btctax_tui::export::export_dir_for`'s pure, testable shape, under a
/// DISTINCT prefix (`btctax-defensive-export-` vs the viewer's `btctax-export-`) so a same-second
/// single-year CLI/viewer export into the same vault-parent directory can never collide with this
/// multi-year packet. `apply_export` then writes each planned year into its OWN `<this>/<year>/`
/// subdirectory (★ T3-M1 — the P-A layout contract). Pure — no filesystem access.
pub fn defensive_export_dir_for(vault_path: &Path, now: OffsetDateTime) -> PathBuf {
    use time::macros::format_description;
    let parent = vault_path.parent().unwrap_or(Path::new("."));
    let ts = now
        .format(format_description!(
            "[year][month][day]-[hour][minute][second]Z"
        ))
        .expect("timestamp format is infallible");
    parent.join(format!("btctax-defensive-export-{ts}"))
}

/// Render the filer-facing outcome of a defensive-filing export — the ONE place that turns
/// `persist_defensive_export`'s per-year `btctax_cli::ExportOutcomes` (★ T3-M2: per-year isolation,
/// never an all-or-nothing abort) into the `app.status` NOTICE text (★ T3-M1: the per-year
/// `out_dir/<year>` paths are named, not just "done"). `reports` is iterated in the order given (the
/// caller passes `plan.years`' own ascending `BTreeSet` order); a year with no bundled IRS-form template
/// (or any other per-year failure) is named individually alongside its reason, WITHOUT hiding the years
/// that DID export. Pure; no I/O.
///
/// ★ whole-branch tax M-2: `unsupported` (`ExportPlan::unsupported_years`) is rendered as its OWN
/// informational clause, NEVER as a failure — those years were deliberately never attempted (this build
/// bundles no IRS form templates for them). This is what stops every 2026 filer's `x` from reading
/// "0 of 1 year(s) written — 2026 failed: unsupported tax year 2026". The clause still tells the filer
/// the year is on the hook, because a flagged PRIOR year in this set must be amended by hand.
pub fn render_export_status(
    out_dir: &Path,
    reports: &[btctax_cli::ExportOutcome],
    unsupported: &std::collections::BTreeSet<i32>,
) -> String {
    let total = reports.len();
    let ok_years: Vec<i32> = reports
        .iter()
        .filter(|(_, r)| r.is_ok())
        .map(|(y, _)| *y)
        .collect();
    let failed: Vec<(i32, String)> = reports
        .iter()
        .filter_map(|(y, r)| r.as_ref().err().map(|e| (*y, e.to_string())))
        .collect();

    let mut msg = format!(
        "export: {} of {total} year(s) written under {}",
        ok_years.len(),
        out_dir.display()
    );
    if !ok_years.is_empty() {
        let parts: Vec<String> = ok_years
            .iter()
            .map(|y| format!("{y} \u{2192} {}", out_dir.join(y.to_string()).display()))
            .collect();
        msg.push_str(&format!(" [{}]", parts.join(", ")));
    }
    for (y, reason) in &failed {
        msg.push_str(&format!(" \u{2014} {y} failed: {reason}"));
    }
    if !unsupported.is_empty() {
        let years: Vec<String> = unsupported.iter().map(|y| y.to_string()).collect();
        msg.push_str(&format!(
            " \u{2014} no bundled IRS templates for {} yet: not attempted (nothing was written for \
             {}); if a year listed here is a PRIOR year, its filed forms changed and it still needs \
             amending by hand",
            years.join(", "),
            if years.len() == 1 { "it" } else { "them" },
        ));
    }
    msg
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
        // ★ whole-branch tax M-1: this used to say the figure was "shown above" when NOTHING drew it —
        // `TrancheRow.clamped_saving` was computed and never rendered. The `[assess]` line now exists
        // (see `render_saving_line`), but it is ABSENT for a tranche with no disposed estimate legs,
        // which is a common shape for this advisory (`covered_sat == 0`). So the copy is conditional:
        // it caveats the figure WHERE ONE IS SHOWN and never presupposes one.
        Advisory::WouldDisplaceIfPromoted => "  [advisory] promoting this tranche would displace \
             documented basis on a real disposal (a HIFO reorder) — so any saving/gain-\u{394} quoted \
             for it on an [assess] line above UNDERSTATES the gain a documented lot would actually \
             realize; treat that figure as a caveat, not a straightforward saving"
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

/// ★ whole-branch tax M-1: `TrancheRow.clamped_saving` — DFW-D10's BG-D6 three-flavor discipline —
/// rendered. It was computed by `journey_view` (two full projections per realized year per unpromoted
/// tranche) and then DRAWN NOWHERE, which left both an expensive dead computation AND
/// `Advisory::WouldDisplaceIfPromoted`'s caveat ("any saving/gain-Δ **shown above** would UNDERSTATE
/// the gain…") pointing at a figure the filer could not see.
///
/// The three flavors keep their contract verbatim: a bare `$X (year Y)` is NEVER printed for a
/// non-computing year. `ComputedTax` is the only dollar-tax flavor; `Uncomputable` prints the
/// profile-free realized-gain delta, explicitly labelled as such and NOT as a tax saving (`journey_view`
/// passes `profile: None`, so in practice every year it computes lands here); `Named` prints its note
/// verbatim. Rendered BEFORE the advisories so `WouldDisplaceIfPromoted`'s "shown above" is literally
/// true.
fn render_saving_line(f: &SavingFlavor) -> String {
    match f {
        SavingFlavor::ComputedTax { year, delta } => format!(
            "  [assess] promoting would lower {year}'s federal tax by ~${:.2} (clamped; never below $0)",
            delta
        ),
        SavingFlavor::Uncomputable { year, gain_delta } => format!(
            "  [assess] {year}: tax is not computable for this year (no bundled table, no stored tax \
             profile, or a blocking issue) — promoting would change the REALIZED GAIN by ~${:.2}; this \
             is a gain figure, not a tax saving",
            gain_delta
        ),
        SavingFlavor::Named(msg) => format!("  [assess] {msg}"),
    }
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

    for f in &row.clamped_saving {
        lines.push(render_saving_line(f));
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
///
/// `uncomputable` (fix round 1, C-1) is `DefensiveDashboardState::uncomputable`: when `Some(reason)`,
/// `reason` is rendered as an explicit notice near the top AND the "Nothing outstanding" fallback below
/// is SUPPRESSED — an all-empty stub view must never read as an affirmative all-clear.
///
/// `stale_armed` (fix wave, BLOCKING 1) is `app.stale_after_write.is_some()`: while the latch is armed,
/// `journey_view` still runs to completion over the STALE pre-write snapshot (`uncomputable` stays
/// `None` — the projection did not fail, it merely ran over old data), so an all-empty result is NOT
/// evidence of an all-clear either. This is the THIRD route to the same false-negative sentence on this
/// branch (T6 closed the pseudo-active route via `uncomputable`; this closes the armed-but-not-pseudo
/// route, which `uncomputable` alone cannot see).
pub fn render_dashboard(
    view: &DefensiveFilingView,
    cursor: usize,
    uncomputable: Option<&str>,
    stale_armed: bool,
) -> Vec<String> {
    let mut lines = vec![
        "Defensive Filing — journey dashboard (derived; nothing here is filed until you act)"
            .to_string(),
        String::new(),
    ];

    if let Some(reason) = uncomputable {
        lines.push(format!("UNCOMPUTABLE — {reason}"));
        lines.push(String::new());
    }

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

    if uncomputable.is_none()
        && !stale_armed
        && view.resolve_first.is_empty()
        && view.candidates.is_empty()
        && view.tranches.is_empty()
        && view.still_short.is_empty()
    {
        lines.push("Nothing outstanding right now.".to_string());
        lines.push(String::new());
    } else if stale_armed
        && uncomputable.is_none()
        && view.resolve_first.is_empty()
        && view.candidates.is_empty()
        && view.tranches.is_empty()
        && view.still_short.is_empty()
    {
        lines.push(
            "This is NOT a statement that nothing is outstanding — the figures on this screen were \
             derived from an image of the ledger taken BEFORE the last write, which could not be \
             re-projected."
                .to_string(),
        );
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
    use btctax_cli::{CliConfig, CliError, IrsPdfReport};
    use btctax_core::project::pools::PoolKey;
    use btctax_core::state::LedgerState;
    use btctax_core::{Source, SourceRef, WalletId};
    use btctax_tui::app::Snapshot;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;
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
        // ★ whole-branch tax M-1: the copy must not claim a figure is "shown above" — this row carries
        // NO `clamped_saving` at all, which is a normal shape for this advisory (it fires when
        // `covered_sat == 0`, and a fully-undisposed tranche yields no saving years).
        assert!(
            !rendered.contains("shown above"),
            "the caveat must not assert a figure is displayed when none is: {rendered}"
        );
    }

    // ── ★ whole-branch tax M-1: `TrancheRow.clamped_saving` is RENDERED (it was computed and drawn
    //    nowhere — two full projections per realized year per unpromoted tranche, dead) ────────────────

    /// The BG-D6/DFW-D10 three-flavor discipline survives the render: `Uncomputable` prints the
    /// profile-free realized-gain delta EXPLICITLY labelled as a gain, never as a tax saving, and a bare
    /// `$X (year Y)` is never printed for a non-computing year. (`journey_view` passes `profile: None`,
    /// so this is the flavor a filer actually sees.)
    ///
    /// Mutation: drop the `for f in &row.clamped_saving` loop from `render_tranche_row` → reds (and
    /// `WouldDisplaceIfPromoted`'s caveat goes back to pointing at nothing).
    #[test]
    fn tranche_row_renders_the_clamped_saving_flavors_without_quoting_a_tax_figure_for_an_uncomputable_year(
    ) {
        let mut row = tranche_row(1, 40_000_000, TrancheStatus::DeclaredZero, vec![]);
        row.clamped_saving = vec![
            SavingFlavor::Uncomputable {
                year: 2019,
                gain_delta: rust_decimal_macros::dec!(1234.50),
            },
            SavingFlavor::Named("no clamped saving is computable for this tranche".to_string()),
        ];
        let rendered = render_tranche_row(&row).join("\n");

        assert!(
            rendered.contains("1234.50") && rendered.contains("2019"),
            "the computed figure must actually reach the filer: {rendered}"
        );
        assert!(
            rendered.contains("REALIZED GAIN") || rendered.contains("gain figure"),
            "an Uncomputable year must be labelled a GAIN delta, never a tax saving (DFW-D10): \
             {rendered}"
        );
        assert!(
            rendered.contains("not a tax saving"),
            "…and say so explicitly: {rendered}"
        );
        assert!(
            rendered.contains("no clamped saving is computable for this tranche"),
            "the Named flavor is surfaced verbatim: {rendered}"
        );
    }

    /// The `[assess]` figure is rendered BEFORE the advisories, so `WouldDisplaceIfPromoted`'s caveat
    /// ("…on an [assess] line above…") is literally true when a figure exists.
    #[test]
    fn the_assess_figure_is_rendered_above_the_advisory_that_caveats_it() {
        let mut row = tranche_row(
            1,
            40_000_000,
            TrancheStatus::DeclaredZero,
            vec![Advisory::WouldDisplaceIfPromoted],
        );
        row.clamped_saving = vec![SavingFlavor::Uncomputable {
            year: 2019,
            gain_delta: rust_decimal_macros::dec!(900),
        }];
        let lines = render_tranche_row(&row);
        let assess = lines
            .iter()
            .position(|l| l.contains("[assess]"))
            .expect("the assess line must be rendered");
        let advisory = lines
            .iter()
            .position(|l| l.contains("[advisory]"))
            .expect("the advisory must be rendered");
        assert!(
            assess < advisory,
            "the caveated figure must appear ABOVE the caveat: {lines:?}"
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
        let rendered = render_dashboard(&view, 1, None, false);
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
        let rendered = render_dashboard(&view, 0, None, false);
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
        let rendered_empty = render_dashboard(&empty_view(), 0, None, false).join("\n");
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
        let rendered_busy = render_dashboard(&busy, 0, None, false).join("\n");
        assert!(
            rendered_busy.contains("[x] export"),
            "export must ALSO be offered on a busy dashboard: {rendered_busy}"
        );
        assert!(
            !rendered_busy.to_lowercase().contains("done"),
            "export must never read as 'done' even once other work exists: {rendered_busy}"
        );
    }

    // ── fix wave BLOCKING 1: `stale_armed` suppresses the all-clear even when `uncomputable` is `None`
    // ────────────────────────────────────────────────────────────────────────────────────────────────
    //
    // `uncomputable` alone cannot see this route: while the latch is armed, `journey_view` runs to
    // completion (over the STALE pre-write snapshot) rather than failing, so `uncomputable` stays
    // `None`. An all-empty result under that condition is not evidence of an all-clear — it may simply
    // be that the write which WOULD have produced a disposal never got re-projected.
    //
    // Mutations, each independently proven:
    // (1) delete the `&& !stale_armed` conjunct on the all-clear guard → this test's `!contains("Nothing
    //     outstanding right now.")` assertion reds (confirmed: RED).
    // (2) unarmed empty sibling (`stale_armed = false`, all four lists empty) asserts the all-clear
    //     STILL renders — guards against over-suppression from a `stale_armed` that defaulted to `true`
    //     (confirmed: a `stale_armed` default flip alone reds this half; it does NOT catch a dropped
    //     `is_empty()` conjunct, since every list here is already empty).
    // (3) unarmed NON-empty sibling (`stale_armed = false`, one declare candidate present) asserts the
    //     all-clear does NOT render — this is what catches a guard that dropped the original
    //     four-empty-lists condition (confirmed: RED when the four `view.*.is_empty()` conjuncts are
    //     deleted from the all-clear guard).
    #[test]
    fn stale_armed_suppresses_the_all_clear_even_with_uncomputable_none() {
        let rendered = render_dashboard(&empty_view(), 0, None, true).join("\n");
        assert!(
            !rendered.contains("Nothing outstanding right now."),
            "an armed-but-not-pseudo stale latch must NOT let the all-clear render: {rendered}"
        );
        assert!(
            rendered.contains("NOT a statement that nothing is outstanding"),
            "the stale analogue of the pseudo notice must render instead: {rendered}"
        );
    }

    #[test]
    fn unarmed_empty_dashboard_still_renders_the_all_clear() {
        // Guards against over-suppression: an unarmed, uncomputable=None, all-empty dashboard must
        // STILL show the all-clear — `stale_armed` must gate ONLY the armed case.
        let rendered = render_dashboard(&empty_view(), 0, None, false).join("\n");
        assert!(
            rendered.contains("Nothing outstanding right now."),
            "an unarmed all-empty dashboard must still read as an all-clear: {rendered}"
        );
    }

    #[test]
    fn unarmed_non_empty_dashboard_never_renders_the_all_clear() {
        // This is route #4 to a false "Nothing outstanding right now.": neither `uncomputable` nor
        // `stale_armed` gates it — only the four `view.*.is_empty()` conjuncts on the all-clear guard
        // do. A view with a real declare candidate present, unarmed and computable, must NEVER print
        // the all-clear alongside it.
        let view = DefensiveFilingView {
            candidates: vec![shortfall(1, 10_000)],
            ..empty_view()
        };
        let rendered = render_dashboard(&view, 0, None, false).join("\n");
        assert!(
            !rendered.contains("Nothing outstanding right now."),
            "a dashboard with a live declare candidate must NEVER also claim nothing is outstanding: \
             {rendered}"
        );
    }

    // ── Task 10 (P-D): the export helpers — `defensive_export_dir_for` / `render_export_status` ────────

    #[test]
    fn defensive_export_dir_for_is_deterministic_vault_parent_scoped_and_distinct_from_the_viewer_export_dir(
    ) {
        let vault = PathBuf::from("/home/filer/vault.pgp");
        let ts = time::macros::datetime!(2025-10-24 14:30:22 UTC);
        let dir1 = defensive_export_dir_for(&vault, ts);
        let dir2 = defensive_export_dir_for(&vault, ts);
        assert_eq!(dir1, dir2, "pure function: same inputs, same output");
        assert_eq!(
            dir1,
            PathBuf::from("/home/filer/btctax-defensive-export-20251024-143022Z")
        );
        assert!(
            dir1.starts_with("/home/filer"),
            "must be scoped under the VAULT's own parent directory: {dir1:?}"
        );
        // Distinct prefix from `btctax_tui::export::export_dir_for`'s "btctax-export-" — a same-second
        // single-year CLI/viewer export can never collide with this multi-year packet's directory.
        assert!(
            dir1.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("btctax-defensive-export-"),
            "must NOT collide with the viewer's own 'btctax-export-' prefix: {dir1:?}"
        );
    }

    /// ★ T3-M1: a fully-successful export names EVERY year's own `out_dir/<year>` path — "done" alone is
    /// not enough (the P-A layout contract must be surfaced, not just implied).
    #[test]
    fn render_export_status_names_every_year_and_its_own_out_dir_path_on_full_success() {
        let out_dir = PathBuf::from("/tmp/btctax-defensive-export-x");
        let ok_report = |year: i32| IrsPdfReport {
            f8949_path: None,
            schedule_d_path: None,
            tax_year: year,
            unresolved_hard: 0,
            broker_reported_rows: 0,
            watermarked: false,
            schedule_se_path: None,
            se_below_floor: false,
            se_addl_medicare: None,
            se_income_without_profile: false,
            form_8283_path: None,
            form_8283_needs_review: false,
            form_8283_section_b: None,
            form_1040_path: None,
            form_1040_filled_7a: false,
            form_1040_loss: false,
            form_8275_path: None,
            full_return_paths: vec![],
            full_return_manifest: None,
            forms_ignored_full_return: false,
            experimental_notice_active: false,
        };
        let reports: Vec<btctax_cli::ExportOutcome> =
            vec![(2024, Ok(ok_report(2024))), (2025, Ok(ok_report(2025)))];
        let status = render_export_status(&out_dir, &reports, &BTreeSet::new());
        assert!(
            status.contains("2 of 2"),
            "must name the full success count: {status:?}"
        );
        assert!(
            status.contains(&out_dir.join("2024").display().to_string()),
            "must name 2024's OWN out_dir/<year> path (T3-M1): {status:?}"
        );
        assert!(
            status.contains(&out_dir.join("2025").display().to_string()),
            "must name 2025's OWN out_dir/<year> path (T3-M1): {status:?}"
        );
        assert!(
            !status.to_lowercase().contains("fail"),
            "a fully-successful export must not read as a failure: {status:?}"
        );
    }

    /// ★ T3-M2: a mixed success/failure batch is reported PER YEAR — "2 of 3 exported; year 3 failed:
    /// <reason>" — never a bare all-or-nothing abort, and the reason string is preserved verbatim.
    #[test]
    fn render_export_status_reports_a_partial_success_and_names_the_failing_year_and_reason() {
        let out_dir = PathBuf::from("/tmp/btctax-defensive-export-x");
        let ok_2025 = IrsPdfReport {
            f8949_path: Some(out_dir.join("2025").join("f8949.pdf")),
            schedule_d_path: None,
            tax_year: 2025,
            unresolved_hard: 0,
            broker_reported_rows: 0,
            watermarked: false,
            schedule_se_path: None,
            se_below_floor: false,
            se_addl_medicare: None,
            se_income_without_profile: false,
            form_8283_path: None,
            form_8283_needs_review: false,
            form_8283_section_b: None,
            form_1040_path: None,
            form_1040_filled_7a: false,
            form_1040_loss: false,
            form_8275_path: None,
            full_return_paths: vec![],
            full_return_manifest: None,
            forms_ignored_full_return: false,
            experimental_notice_active: false,
        };
        let reports: Vec<btctax_cli::ExportOutcome> = vec![
            (
                2016,
                Err(CliError::Usage(
                    "unsupported tax year 2016: this build bundles IRS forms for 2017, 2024 and 2025 \
                     only"
                        .to_string(),
                )),
            ),
            (2025, Ok(ok_2025)),
        ];
        let status = render_export_status(&out_dir, &reports, &BTreeSet::new());
        assert!(
            status.contains("1 of 2"),
            "must name the partial-success count (T3-M2): {status:?}"
        );
        assert!(
            status.contains("2016") && status.to_lowercase().contains("unsupported"),
            "must name the FAILING year and its reason verbatim: {status:?}"
        );
        assert!(
            status.contains(&out_dir.join("2025").display().to_string()),
            "the SUCCESSFUL year's own path must still be named, not swallowed by the failure (T3-M1): \
             {status:?}"
        );
    }

    /// ★ whole-branch tax M-2: a year with NO bundled IRS templates is reported as a NOT-ATTEMPTED
    /// informational clause — never as a per-year FAILURE. Before the `plan_export` partition, every
    /// filer in 2026 pressed `x` and read "0 of 1 year(s) written — 2026 failed: unsupported tax year
    /// 2026", with a half-populated `out_dir/2026/` on disk beside it.
    ///
    /// Mutation: render `unsupported` through the `failed` loop (or drop the clause) → the "failed"
    /// assertion (or the naming assertion) reds.
    #[test]
    fn render_export_status_reports_an_unsupported_year_as_not_attempted_not_as_a_failure() {
        let out_dir = PathBuf::from("/tmp/btctax-defensive-export-x");
        // The universal 2026 shape: nothing fillable at all, one unsupported current year.
        let status = render_export_status(
            &out_dir,
            &[],
            &[2026].into_iter().collect::<BTreeSet<i32>>(),
        );
        assert!(
            status.contains("2026"),
            "the unsupported year must still be NAMED — silence would hide it: {status:?}"
        );
        assert!(
            status.to_lowercase().contains("no bundled irs templates"),
            "…with the real reason, in filer language: {status:?}"
        );
        assert!(
            !status.to_lowercase().contains("failed"),
            "a year this build simply has no templates for is NOT a failure: {status:?}"
        );
        assert!(
            status.to_lowercase().contains("amend"),
            "a PRIOR year landing here still needs amending by hand — say so: {status:?}"
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

    // ── fix round 1 (I-1): `uncomputable` refuses every row-addressed action BY CONSTRUCTION ─────────

    /// Mirrors `d_on_a_candidate_row_names_declare_intent` /
    /// `p_on_a_declared_zero_tranche_names_promote_intent_but_not_on_a_promoted_one` /
    /// `enter_on_a_resolve_first_row_names_route_intent` EXACTLY — same non-empty, cursor-addressable
    /// rows each of those tests proves DOES yield a real intent — but with `uncomputable` set. Proves
    /// the refusal is a CONSTRUCTED check on `uncomputable`, not an accident of the row lists happening
    /// to be empty (the failure mode fix round 1 flagged: nothing but empty rows was stopping a
    /// Declare/Promote/RouteResolveFirst target from being derived off a state `journey_view` itself
    /// asserts cannot exist).
    ///
    /// Mutation: delete the `if state.uncomputable.is_some() { return DashboardIntent::None; }` guard
    /// in `handle_defensive_dashboard_key` → the `d`/`p`/Enter assertions below fail (they instead
    /// return the real `Declare`/`Promote`/`RouteResolveFirst` intent) → reds.
    #[test]
    fn uncomputable_state_refuses_every_row_addressed_action_by_construction() {
        let mut d_state = DefensiveDashboardState::new(DefensiveFilingView {
            candidates: vec![shortfall(1, 10_000)],
            ..empty_view()
        });
        d_state.uncomputable = Some("test");
        assert_eq!(
            handle_defensive_dashboard_key(&mut d_state, key(KeyCode::Char('d'))),
            DashboardIntent::None,
            "d must refuse over a REAL candidate row while uncomputable"
        );

        let mut p_state = DefensiveDashboardState::new(DefensiveFilingView {
            tranches: vec![tranche_row(1, 10_000, TrancheStatus::DeclaredZero, vec![])],
            ..empty_view()
        });
        p_state.uncomputable = Some("test");
        assert_eq!(
            handle_defensive_dashboard_key(&mut p_state, key(KeyCode::Char('p'))),
            DashboardIntent::None,
            "p must refuse over a REAL DeclaredZero tranche row while uncomputable"
        );

        let mut enter_state = DefensiveDashboardState::new(DefensiveFilingView {
            resolve_first: vec![Triage::ResolveFirst {
                shortfall: shortfall(1, 10_000),
                blocker: BlockerKind::Unclassified,
            }],
            ..empty_view()
        });
        enter_state.uncomputable = Some("test");
        assert_eq!(
            handle_defensive_dashboard_key(&mut enter_state, key(KeyCode::Enter)),
            DashboardIntent::None,
            "Enter must refuse over a REAL resolve-first row while uncomputable"
        );

        // x stays reachable regardless (D-7) — the ONE action `uncomputable` does not gate.
        assert_eq!(
            handle_defensive_dashboard_key(&mut d_state, key(KeyCode::Char('x'))),
            DashboardIntent::Export
        );
    }

    // ── fix round 1 (C-1): the uncomputable stub must NEVER render as an affirmative all-clear ───────

    /// `safe_journey_view` on a genuinely pseudo-active state must mark the result `uncomputable` and
    /// STILL genuinely compute `safe_harbor_blocked` (an events-only scan — no hard-coded `false`), and
    /// `render_dashboard` must render an explicit notice INSTEAD of the "Nothing outstanding" fallback,
    /// which would otherwise read as a false affirmative all-clear over an uncomputed state (a filer
    /// could file omitting a real Form-8949 disposition with no marker at all once `x` clears the
    /// latch — see the fix-round-1 report).
    ///
    /// Mutations, each independently proven (see the task report's mutation-proof table):
    /// (1) `safe_journey_view`'s `if state.pseudo_active()` guard deleted → panics via `journey_view`'s
    ///     own precondition assert (covered by the Task 6 KATs already).
    /// (2) `render_dashboard`'s `uncomputable.is_none() &&` dropped from the "Nothing outstanding" guard
    ///     → the false all-clear resurfaces alongside the notice → reds here.
    /// (3) `safe_harbor_blocked` hard-coded back to `false` in `safe_journey_view`'s stub → reds here.
    #[test]
    fn safe_journey_view_on_a_pseudo_active_state_renders_an_explicit_notice_not_an_all_clear() {
        use btctax_core::event::{AllocMethod, EventPayload, SafeHarborAllocation};
        use btctax_core::{LedgerEvent, LotMethod};
        use time::macros::datetime;

        // A real in-force safe-harbor allocation, so `safe_harbor_blocked` has something genuine to
        // compute (`tranche_guard::in_force_allocation_exists` is an events-only scan) — proving the
        // stub does NOT hard-code it to `false`.
        let events = vec![LedgerEvent {
            id: EventId::decision(1),
            utc_timestamp: datetime!(2025-01-01 0:00 UTC),
            original_tz: time::UtcOffset::UTC,
            wallet: None,
            payload: EventPayload::SafeHarborAllocation(SafeHarborAllocation {
                lots: vec![],
                as_of_date: date!(2025 - 01 - 01),
                method: AllocMethod::ProRata,
                timely_allocation_attested: true,
                pre2025_method: LotMethod::Fifo,
            }),
        }];
        let state = LedgerState {
            pseudo_synthetic_count: 3,
            ..Default::default()
        };
        let prices = btctax_adapters::LayeredPrices::load_with_cache(None).unwrap();
        let tables = BundledTaxTables::load();
        let cfg = CliConfig::default().to_projection();

        let dash = safe_journey_view(&events, &state, &prices, &tables, &cfg, 2025);
        assert!(
            dash.uncomputable.is_some(),
            "a pseudo-active state must mark the dashboard uncomputable"
        );
        assert!(
            dash.view.safe_harbor_blocked,
            "safe_harbor_blocked is an EVENTS-only scan — it must be genuinely computed, not \
             hard-coded false, even in the uncomputable stub"
        );

        let rendered =
            render_dashboard(&dash.view, dash.cursor, dash.uncomputable, false).join("\n");
        assert!(
            rendered.contains("UNCOMPUTABLE"),
            "the notice must render: {rendered}"
        );
        assert!(
            rendered.to_lowercase().contains("pseudo"),
            "the notice must name the pseudo-reconcile cause: {rendered}"
        );
        assert!(
            rendered.to_lowercase().contains("not a statement"),
            "the notice must explicitly deny being an all-clear: {rendered}"
        );
        assert!(
            rendered.to_lowercase().contains("form 8949"),
            "the notice must name the concrete filing consequence: {rendered}"
        );
        assert!(
            !rendered.contains("Nothing outstanding"),
            "the false all-clear must NOT render alongside the notice: {rendered}"
        );
        assert!(
            rendered.contains("[x] export"),
            "export must stay reachable per D-7: {rendered}"
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
