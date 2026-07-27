//! Terminal rendering for the editor.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! Delegates to the viewer's App-free `tabs::*::render` functions for the Browse screen;
//! uses `btctax_tui::draw::draw_unlock_screen` with EDITOR-branded strings for the
//! Unlock screen. This module performs no writes.

use crate::edit::form::{
    amount_label, basis_source_display, bulk_checked_totals, bulk_income_checked_totals,
    bulk_reclassify_outflow_checked_totals, bulk_resolve_checked_count, bulk_sti_checked_totals,
    bulk_usd_floor_label, bulk_void_checked_count, bulk_void_lot_selection_checked_count,
    income_kind_display, lot_method_label, wallet_label, BulkIncomeFlowState, BulkIncomeModalState,
    BulkIncomeStep, BulkLinkFlowState, BulkLinkModalState, BulkLinkStep,
    BulkReclassifyOutflowFlowState, BulkReclassifyOutflowModalState, BulkReclassifyOutflowStep,
    BulkResolveFlowState, BulkResolveModalState, BulkResolveStep, BulkStiFlowState,
    BulkStiModalState, BulkStiStep, BulkVoidFlowState, BulkVoidModalState,
    ClassifyInboundModalState, ClassifyInboundStep, ClassifyRawFlowState, ClassifyRawModalState,
    ClassifyRawStep, ClassifyRawVariant, DisposalKind, FieldBuffer, InboundVariant, LinkMode,
    LinkTransferFlowState, LinkTransferModalState, LinkTransferStep, MatchSelfTransfersFlowState,
    MatchSelfTransfersModalState, MethodElectionFlowState, MethodElectionModalState,
    MethodElectionStep, MutationModalState, OptimizeAcceptFlowState, OptimizeAcceptModalState,
    OptimizeAcceptStep, OutflowKind, ProfileFormState, ReclassifyIncomeFlowState,
    ReclassifyIncomeModalState, ReclassifyIncomeStep, ReclassifyOutflowModalState,
    ReclassifyOutflowStep, ResolveConflictFlowState, ResolveConflictModalState,
    ResolveConflictStep, ResolveKind, SafeHarborAllocateFlowState, SafeHarborAllocateModalState,
    SafeHarborAttestFlowState, SafeHarborAttestStep, SelectLotsFlowState, SelectLotsModalState,
    SelectLotsStep, SetDonationDetailsFlowState, SetDonationDetailsModalState,
    SetDonationDetailsStep, SetFmvFlowState, SetFmvModalState, SetFmvStep, TaxInputsFormState,
    VoidFlowState, VoidModalState, DONATION_FIELD_LABELS, FIELD_LABELS,
};
use crate::edit::form::{filing_status_field, live_fields, live_sections};
use crate::editor::{EditorApp, EditorScreen};
use btctax_core::tax::return_inputs::ReturnInputs;
use btctax_core::{DisposeKind, InboundClass, OutflowClass, Persistability};
use btctax_input_form::{
    FieldKind, FieldValue, RowAddr, SecretView, Section, SectionId, SectionKind,
};
use btctax_tui::app::Tab;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

/// Top-level draw entry point — dispatches on `EditorScreen`.
pub fn draw(frame: &mut Frame, app: &mut EditorApp) {
    match app.screen {
        EditorScreen::Unlock => draw_unlock(frame, app),
        EditorScreen::Locked => draw_locked(frame),
        EditorScreen::Browse => draw_browse(frame, app),
        EditorScreen::DefensiveFiling => draw_defensive_filing(frame, app),
    }
}

/// Render the unlock screen with EDITOR-branded title and note line.
fn draw_unlock(frame: &mut Frame, app: &EditorApp) {
    btctax_tui::draw::draw_unlock_screen(
        frame,
        &app.vault_path,
        &app.unlock,
        " btctax-tui-edit — Unlock Vault [EDITOR] ",
        "offline · local · EDITOR — writes on explicit confirmation only",
    );
}

/// Render the locked screen with EDITOR marker.
fn draw_locked(frame: &mut Frame) {
    let area = frame.area();
    let block = Block::default()
        .title(" btctax-tui-edit [EDITOR] — Vault Locked ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let msg = Paragraph::new(
        "Vault is in use by another process (the CLI or another viewer/editor).\n\
         Close it and retry.\n\n\
         r  retry   q  quit",
    )
    .alignment(Alignment::Center);
    frame.render_widget(msg, inner);
}

/// Render the Defensive Filing Wizard screen (Task 7, Phase P-B dashboard; Tasks 8/9 write flows): a
/// derived text render of `crate::defensive_dashboard::render_dashboard` (or of whichever `*_flow` has
/// taken over the content area), PLUS a NOTICE line fed from `app.status`.
///
/// ★ P-C gate arch I-2: this is a FULL-FRAME screen — it replaces the Browse footer, the only other
/// place `app.status` is rendered — so without the NOTICE line every refusal/error/outcome this
/// (P-C: WRITE) surface produces is invisible: `declare_flow_confirm`'s "declare refused: {err}"
/// (DFW-D5's mandated "a refusal with a reason, not a silent append"), both openers' stale-target
/// refusals, the success statuses, and — worst — `on_persist_error`'s CRITICAL unrevertable-residue
/// notice (which `close_all_mutation_surfaces` leaves on THIS screen). Mirrors the reviewed precedent
/// for full-frame overlays, `draw_tax_inputs_form`'s own ★ I-2 status threading.
fn draw_defensive_filing(frame: &mut Frame, app: &EditorApp) {
    let area = frame.area();
    let block = Block::default()
        .title(" btctax-tui-edit [EDITOR] — Defensive Filing ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve the NOTICE rows whenever there IS a status OR the stale latch is armed (D-7, DESIGN.md
    // §2.4(a)) — the marker renders independent of `app.status`, so gating the reservation on
    // `status.is_some()` alone leaves no rect for it to draw into from the SECOND keypress onward
    // (`main.rs:494` clears `app.status` before every dispatch). An empty, unarmed screen keeps its
    // full height, exactly as before.
    let status = app.status.as_deref();
    let armed = app.stale_after_write.is_some();

    // The notice's actual content, built ONCE — used both to MEASURE how tall the reservation needs to
    // be (below) and to draw it (at the bottom of this fn). Keeping these in lock-step BY CONSTRUCTION
    // (one `Vec<Line>`, one wrap setting, shared between the measuring pass and the real render) is what
    // makes the measured height provably match what is actually drawn.
    let mut notice_lines: Vec<Line> = Vec::new();
    if armed {
        notice_lines.push(Line::from(Span::styled(
            format!("  {STALE_MARKER_DEFENSIVE}"),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }
    if let Some(s) = status {
        // The unrevertable-residue notice is the one status that must SHOUT (it tells the filer to quit
        // NOW); every other refusal/outcome reads as an ordinary notice.
        let style = if s.starts_with("CRITICAL") {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        notice_lines.push(Line::from(Span::styled(format!("  {s}"), style)));
    }

    let (content_area, notice_area) = if status.is_some() || armed {
        // ★ fix round 2 Critical: a FLAT `DEFENSIVE_NOTICE_LINES` reservation whenever `status.is_some()`
        // — regardless of that status's actual LENGTH — was itself the bug fix round 1 only half-fixed.
        // It clipped `[x] export` with a 385-char status, a 36-char one, and even a ONE-CHARACTER one;
        // worse, since `status.is_some()` is true on the ORDINARY (unarmed) mainline too — every
        // declare/promote outcome message — raising `DEFENSIVE_NOTICE_LINES` 3→8 in the first commit
        // silently regressed every unarmed filer's clipping cliff from ~9 candidates down to ~5. The
        // reservation must be sized to what will ACTUALLY be drawn, not a worst-case constant reserved
        // by mere PRESENCE. `content_sized_notice_height` measures it by rendering `notice_lines` into a
        // throwaway buffer — the same wrap algorithm the real render below uses — capped at
        // `DEFENSIVE_NOTICE_LINES` (the true worst case: the longest composed arm status stacked under
        // the marker) so a pathologically long/unbounded status can't blow the reservation out further.
        //
        // Floor at 1: `.min(inner.height.saturating_sub(1))` yields 0 at a terminal height ≤ 3, which
        // would silently drop the marker instead of shrinking it.
        let notice_h = content_sized_notice_height(&notice_lines, inner.width)
            .min(inner.height.saturating_sub(1))
            .max(1);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(notice_h)])
            .split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };

    // The Declare flow (Task 8), when open, takes over the content area — mirrors every other
    // `*_flow` overlay drawn atop its own screen. `app.declare_flow.is_some()` is only reachable via
    // `open_declare_flow`, which requires a live `snapshot` (it reads the dashboard's own already-
    // computed view) — the no-snapshot arm is therefore unreached in practice, mirroring the
    // dashboard's own defensive fallback below. The Promote flow (Task 9) likewise takes over next.
    let lines: Vec<Line> = if let Some(flow) = app.declare_flow.as_ref() {
        match app.snapshot.as_ref() {
            Some(snap) => crate::edit::declare_flow::render_declare_flow(flow, &snap.prices)
                .into_iter()
                .map(Line::from)
                .collect(),
            None => vec![Line::from("No snapshot — press Esc to cancel.")],
        }
    } else if let Some(flow) = app.promote_flow.as_ref() {
        crate::edit::promote_flow::render_promote_flow(flow)
            .into_iter()
            .map(Line::from)
            .collect()
    } else if let Some(dash) = app.defensive_dashboard.as_ref() {
        crate::defensive_dashboard::render_dashboard(&dash.view, dash.cursor, dash.uncomputable)
            .into_iter()
            .map(Line::from)
            .collect()
    } else {
        vec![Line::from(
            "No dashboard state — press Esc to return to Browse.",
        )]
    };
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, content_area);

    // The fixed marker renders whenever armed, independent of `app.status` — DESIGN.md §2.4(a): "render,
    // don't merely reserve". When a status is ALSO present, both appear, stacked. `notice_lines` was
    // already built above (shared with the measuring pass).
    if let Some(rect) = notice_area {
        let notice = Paragraph::new(notice_lines).wrap(Wrap { trim: false });
        frame.render_widget(notice, rect);
    }
}

/// The fixed, `app.status`-independent claim BOTH armed screens show (D-7 marker — the THIRD
/// specification of this marker; the first two were unbuildable, DESIGN.md §2.4). From the second
/// keypress onward this is the only notice the filer sees on Browse (`main.rs:414`/`:494` clear
/// `app.status` before every dispatch), so it names the CONSEQUENCE — "figures below predate your last
/// write" — not the cause. Register precedent: the PSEUDO banner's own "FICTIONAL placeholders — DO NOT
/// FILE" (`:214-217` below). Kept ≤ 78 cols (measured) so it never wraps inside Browse's own one-row
/// marker constraint.
const STALE_MARKER_CLAIM: &str =
    "STALE — figures below predate your last write. DO NOT FILE from them.";

/// DefensiveFiling's own marker: the same claim, PLUS the remedy sentence (DESIGN.md §2.4(c)) — unlike
/// Browse, this screen has no separate footer or one-row slot of its own to carry the remedy, so it has
/// to live in the marker text itself, not only in whatever `app.status` happens to hold at the moment.
const STALE_MARKER_DEFENSIVE: &str =
    "STALE — figures below predate your last write. DO NOT FILE from them. Quit and reopen the vault.";

/// The CAP on the notice reservation both screens use — NOT a flat reservation size (★ fix round 2
/// Critical: it was exactly that in fix round 1, and `status.is_some()` alone drove the flat 8 whether
/// the status was 385 chars or 1, clipping `[x] export` at every length and regressing the ORDINARY
/// unarmed mainline — every declare/promote outcome message — from a ~9-candidate clipping cliff to
/// ~5). `content_sized_notice_height` below measures what will ACTUALLY be drawn and clamps it to this
/// many rows, so a pathologically long/unbounded status can't blow the reservation out past the true
/// worst case.
///
/// **That worst case is measured, not estimated**, against the real production path:
/// `EditorApp::apply_reprojection`'s `Err` arm → `arm_stale`, driven directly (see
/// `main.rs::arm_stale_status_survives_a_failing_draft_flush`, temporarily instrumented to print the
/// composed string during this sizing pass) with the longest of the three per-tail prefixes
/// (`"the safe-harbor attest write landed — "`, 38 chars — longer than
/// `"parked the full return for {year} — "` at 34 and `"committed {year} as {label} — "` at 28 for its
/// `"(unset)"` filing-status fallback, the longest real label) and a representative io error
/// (`"io: No such file or directory (os error 2)"`, 42 chars) standing in for BOTH the re-projection
/// failure and a genuine fail-safe-flush failure (they share a root cause — disk/session I/O — per
/// `arm_stale`'s own doc). That composes to a 385-char status: `prefix`, then `STALE_FACT_1`, then
/// `({reason})`, then fact 2's LOSS variant (longer than the no-loss `FACT_2_SCREEN_CLOSED` sentence),
/// then fact 3. Stacked under the 96-char `STALE_MARKER_DEFENSIVE` marker line, this wraps to 8 rows at
/// this screen's 78-col inner width — confirmed by an actual render, not arithmetic alone
/// (`the_full_arm_status_renders_unclipped_at_80_columns_on_both_screens`).
///
/// Previously 3 (sized only for the ~230-char CRITICAL residue notice, the only thing that ever used
/// this rect before this task), then a flat 8 (fix round 1), now a cap on a content-sized measurement
/// (fix round 2).
const DEFENSIVE_NOTICE_LINES: u16 = 8;

/// The exact number of rows `lines`, wrapped at `width`, will need when actually drawn — MEASURED by
/// rendering them into a throwaway buffer with ratatui's own wrapping algorithm (the same one the real
/// render uses), not estimated and not re-implemented. Capped at `DEFENSIVE_NOTICE_LINES` and floored
/// at 1: a status this measures against is `app.status`, an unbounded `String` (ultimately a `CliError`
/// `Display`), so an absurdly long one must still be BOUNDED, never allowed to consume the whole screen.
///
/// `ratatui::widgets::Paragraph::line_count(width)` would compute this directly, but it is gated behind
/// the `unstable-rendered-line-info` cargo feature — its own doc says "the design for text wrapping is
/// not stable and might affect this API" — so this measures via a REAL render instead, keeping this
/// crate's `ratatui` dependency feature-flag-free and immune to that instability.
///
/// ★ fix round 2 Critical: this is the actual fix. Fix round 1 reserved a FLAT `DEFENSIVE_NOTICE_LINES`
/// whenever `status.is_some()`, regardless of that status's length — verified (by the round-2 reviewer)
/// to clip `[x] export` with a 385-char status, a 36-char one, and even a ONE-CHARACTER one, and to
/// regress the ordinary UNARMED mainline (every declare/promote outcome message sets `app.status`, no
/// latch involved) from a ~9-candidate clipping cliff down to ~5, since raising the flat constant 3→8
/// in the first commit applied unconditionally to every status, not just the armed one.
///
/// A side benefit noted (non-blocking) in the round-2 review: because this measures at the CALLER's
/// actual `width` rather than assuming 80 columns, a narrower terminal automatically gets a taller
/// reservation for the same text (verified: `narrow_terminal_still_reserves_enough_for_the_full_marker`
/// below) — the old flat-2-at-80-columns constant this replaces would have clipped the marker's own
/// remedy sentence at ≤50 columns; this no longer can, up to the `DEFENSIVE_NOTICE_LINES` cap.
fn content_sized_notice_height(lines: &[Line], width: u16) -> u16 {
    let w = width.max(1);
    let max_rows = DEFENSIVE_NOTICE_LINES;
    let backend = ratatui::backend::TestBackend::new(w, max_rows);
    let mut terminal = ratatui::Terminal::new(backend).expect("in-memory backend never errors");
    let probe_area = Rect::new(0, 0, w, max_rows);
    terminal
        .draw(|f| {
            let p = Paragraph::new(lines.to_vec()).wrap(Wrap { trim: false });
            f.render_widget(p, probe_area);
        })
        .expect("drawing into an in-memory backend never errors");
    let buf = terminal.backend().buffer();
    let mut used = 0u16;
    for (i, row) in buf.content().chunks(w as usize).enumerate() {
        if row.iter().any(|c| c.symbol() != " ") {
            used = i as u16 + 1;
        }
    }
    used.max(1)
}

/// Render the browse screen: EDITOR-marked tab bar + viewer tab content + EDITOR footer.
/// Form and modal overlays are drawn on top.
fn draw_browse(frame: &mut Frame, app: &mut EditorApp) {
    let area = frame.area();
    // Pseudo-reconcile (sub-project 2): a LOUD banner whenever a synthetic default contributes to the
    // projection — keyed off `state.pseudo_active()` (the effect), so it stays in lock-step with the
    // re-projected snapshot the TUI already redraws. A dedicated row inserted below the tab bar.
    let (show_banner, pseudo_count) = app
        .snapshot
        .as_ref()
        .map(|s| (s.state.pseudo_active(), s.state.pseudo_synthetic_count))
        .unwrap_or((false, 0));
    // D-7 stale-latch marker (DESIGN.md §2.4(b)): the notice machinery in `draw_defensive_filing` has no
    // Browse counterpart, and Browse is the screen that renders Form 8949 / Schedule D figures straight
    // off `snap` — the stale image — so it needs the marker most. Its footer below is a single
    // UN-WRAPPED `Paragraph` that truncates, so the full three-fact composed status cannot live there:
    // while armed, Browse gets a fixed one-row marker PLUS — only when a status is present — a separate
    // wrapped multi-row band, inserted exactly as the PSEUDO banner already is.
    let armed = app.stale_after_write.is_some();
    let show_stale_band = armed && app.status.is_some();
    let mut constraints = vec![Constraint::Length(3)]; // tab bar
    if show_banner {
        constraints.push(Constraint::Length(1)); // pseudo banner
    }
    if armed {
        constraints.push(Constraint::Length(1)); // STALE marker row
    }
    if show_stale_band {
        // Content-sized (fix round 2 Critical, extended here for the same reason — see
        // `content_sized_notice_height`'s doc): a flat `DEFENSIVE_NOTICE_LINES` would over-reserve for
        // an ordinary short status exactly as it did in `draw_defensive_filing`. Browse's band shows
        // ONLY the status (the marker has its own separate row above), so it measures just that line.
        let band_h = content_sized_notice_height(
            &[Line::from(format!(
                "  {}",
                app.status.as_deref().unwrap_or_default()
            ))],
            area.width,
        );
        constraints.push(Constraint::Length(band_h)); // wrapped stale status band
    }
    constraints.push(Constraint::Min(0)); // content pane
    constraints.push(Constraint::Length(1)); // footer keybindings
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    // Index bookkeeping: each conditional row above shifts every index after it down by one — extends
    // the pre-existing banner-only scheme to the two new stale-latch rows.
    let mut next_idx = 1usize;
    let banner_idx = if show_banner {
        next_idx += 1;
        Some(next_idx - 1)
    } else {
        None
    };
    let marker_idx = if armed {
        next_idx += 1;
        Some(next_idx - 1)
    } else {
        None
    };
    let band_idx = if show_stale_band {
        next_idx += 1;
        Some(next_idx - 1)
    } else {
        None
    };
    let content_idx = next_idx;
    next_idx += 1;
    let footer_idx = next_idx;

    // ── Tab bar with [EDITOR] badge ───────────────────────────────────────────
    let tab_titles: Vec<&str> = Tab::ALL.iter().map(|t| t.title()).collect();
    let tabs_widget = Tabs::new(tab_titles)
        .select(app.tab.index())
        .block(Block::default().borders(Borders::ALL).title(format!(
            " btctax-tui-edit [EDITOR] — {} ",
            app.vault_path.display()
        )))
        .highlight_style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );
    frame.render_widget(tabs_widget, chunks[0]);

    // ── Pseudo-reconcile banner (loud red/reversed) ───────────────────────────
    if let Some(idx) = banner_idx {
        let banner = Paragraph::new(format!(
            " PSEUDO-RECONCILE MODE ACTIVE — {pseudo_count} synthetic default(s): [PSEUDO] rows are \
             FICTIONAL placeholders — DO NOT FILE. Export blocked. 'P' to approve, off via CLI. "
        ))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );
        frame.render_widget(banner, chunks[idx]);
    }

    // ── Stale-latch marker (D-7, loud red/reversed — precedent: the PSEUDO banner above) ─────────────
    if let Some(idx) = marker_idx {
        let marker = Paragraph::new(format!(" {STALE_MARKER_CLAIM} "))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            );
        frame.render_widget(marker, chunks[idx]);
    }

    // ── Stale-latch status band — a WRAPPED Paragraph, unlike the truncating footer below ────────────
    if let Some(idx) = band_idx {
        let s = app.status.as_deref().unwrap_or_default();
        let band = Paragraph::new(format!("  {s}"))
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(band, chunks[idx]);
    }

    // ── Content pane — delegate to viewer's App-free tab renderers ────────────
    let content_area = chunks[content_idx];
    if let Some(snap) = app.snapshot.as_ref() {
        let year = app.selected_year;
        match app.tab {
            Tab::Holdings => btctax_tui::tabs::holdings::render(
                frame,
                content_area,
                snap,
                year,
                app.holdings_sort,
                app.holdings_cursor,
                &mut app.holdings_state,
            ),
            Tab::Disposals => btctax_tui::tabs::disposals::render(
                frame,
                content_area,
                snap,
                year,
                app.disposals_sort,
                app.disposals_cursor,
                &mut app.disposals_state,
            ),
            Tab::Income => btctax_tui::tabs::income::render(
                frame,
                content_area,
                snap,
                year,
                app.income_sort,
                app.income_cursor,
                &mut app.income_state,
            ),
            Tab::Tax => {
                btctax_tui::tabs::tax::render(frame, content_area, snap, year);
            }
            Tab::Forms => btctax_tui::tabs::forms::render(
                frame,
                content_area,
                snap,
                year,
                &mut app.forms_state,
            ),
            Tab::Compliance => {
                btctax_tui::tabs::compliance::render(frame, content_area, snap, year);
            }
        }
    } else {
        let p = Paragraph::new("Snapshot unavailable — please restart the editor.")
            .alignment(Alignment::Center);
        frame.render_widget(p, content_area);
    }

    // ── Footer: status or keybindings ─────────────────────────────────────────
    // ★ P-C gate arch I-3 note: Browse `w` is listed in the KEYMAP overlay (`help_overlay_lines`), NOT
    // here. This footer is the NAVIGATION hint and already overflows an 80/120-col terminal (its tail
    // clips mid-line), so appending a feature key here would push "?: help" — the pointer to the overlay
    // that lists EVERY feature key, `w` included — off the visible row, making discoverability strictly
    // worse. Feature keys (c/o/r/f/v/S/d/L/u/m/i/z/e/a/A/b/B/C/V/I/O/P/T/w) live in the overlay by
    // convention.
    let keybindings_hint =
        "Tab/Shift-Tab: tab   ←/→ h/l: column   s: sort   [/]: year   ↑/↓ j/k: scroll   \
         g/G: top/bottom   p: profile   ?: help   q/Esc: quit   [EDITOR]";
    let footer_text = if armed {
        // D-7: while armed the footer ALWAYS shows the keybinding hint, never the raw status — the
        // composed status now has its own wrapped band above (`band_idx`) when present, and this footer
        // is a single UN-WRAPPED Paragraph that would truncate it (DESIGN.md §2.4(b)).
        keybindings_hint.to_string()
    } else if let Some(status) = app.status.as_deref() {
        status.to_string()
    } else {
        keybindings_hint.to_string()
    };
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center);
    frame.render_widget(footer, chunks[footer_idx]);

    // ── Overlays (drawn AFTER content so they appear on top) ─────────────────
    if app.help_open {
        draw_help_overlay(frame, area);
    }
    if let Some(form) = app.profile_form.as_ref() {
        draw_profile_form(frame, area, form);
    }
    if let Some(form) = app.tax_inputs_form.as_ref() {
        // ★ I-2: thread `app.status` INTO the overlay — this full-frame overlay clears the Browse footer
        // that normally renders it, so an in-flow status is invisible unless the flow draws it itself.
        draw_tax_inputs_form(frame, area, form, app.status.as_deref());
    }
    if let Some(modal) = app.mutation_modal.as_ref() {
        draw_mutation_modal(frame, area, modal);
    }
    // Classify-inbound flow overlay.
    if app.classify_inbound_flow.is_some() {
        let is_list = matches!(
            app.classify_inbound_flow.as_ref().map(|f| &f.step),
            Some(ClassifyInboundStep::List)
        );
        if is_list {
            if let Some(flow) = app.classify_inbound_flow.as_mut() {
                draw_classify_inbound_list(frame, area, flow);
            }
        } else if let Some(flow) = app.classify_inbound_flow.as_ref() {
            draw_classify_inbound_form(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.classify_inbound_modal.as_ref() {
        draw_classify_inbound_modal(frame, area, modal);
    }
    // Reclassify-outflow flow overlay.
    if app.reclassify_outflow_flow.is_some() {
        let is_list = matches!(
            app.reclassify_outflow_flow.as_ref().map(|f| &f.step),
            Some(ReclassifyOutflowStep::List)
        );
        if is_list {
            if let Some(flow) = app.reclassify_outflow_flow.as_mut() {
                draw_reclassify_outflow_list(frame, area, flow);
            }
        } else if let Some(flow) = app.reclassify_outflow_flow.as_ref() {
            draw_reclassify_outflow_form(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.reclassify_outflow_modal.as_ref() {
        draw_reclassify_outflow_modal(frame, area, modal);
    }
    // Reclassify-income flow overlay.
    if app.reclassify_income_flow.is_some() {
        let is_list = matches!(
            app.reclassify_income_flow.as_ref().map(|f| &f.step),
            Some(ReclassifyIncomeStep::List)
        );
        if is_list {
            if let Some(flow) = app.reclassify_income_flow.as_mut() {
                draw_reclassify_income_list(frame, area, flow);
            }
        } else if let Some(flow) = app.reclassify_income_flow.as_ref() {
            draw_reclassify_income_form(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.reclassify_income_modal.as_ref() {
        draw_reclassify_income_modal(frame, area, modal);
    }
    // Set-FMV flow overlay.
    if app.set_fmv_flow.is_some() {
        let is_list = matches!(
            app.set_fmv_flow.as_ref().map(|f| &f.step),
            Some(SetFmvStep::List)
        );
        if is_list {
            if let Some(flow) = app.set_fmv_flow.as_mut() {
                draw_set_fmv_list(frame, area, flow);
            }
        } else if let Some(flow) = app.set_fmv_flow.as_ref() {
            draw_set_fmv_form(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.set_fmv_modal.as_ref() {
        draw_set_fmv_modal(frame, area, modal);
    }
    // Void-decision flow overlay.
    if app.void_flow.is_some() {
        if let Some(flow) = app.void_flow.as_mut() {
            draw_void_list(frame, area, flow);
        }
    }
    if let Some(modal) = app.void_modal.as_ref() {
        draw_void_modal(frame, area, modal);
    }
    // Select-lots flow overlay.
    if app.select_lots_flow.is_some() {
        if let Some(flow) = app.select_lots_flow.as_mut() {
            match &mut flow.step {
                SelectLotsStep::List => draw_select_lots_list(frame, area, flow),
                SelectLotsStep::LotsForm { .. } => draw_lots_form(frame, area, &mut flow.step),
            }
        }
    }
    if let Some(modal) = app.select_lots_modal.as_ref() {
        draw_select_lots_modal(frame, area, modal);
    }
    // Set-donation-details flow overlay.
    if app.set_donation_details_flow.is_some() {
        if let Some(flow) = app.set_donation_details_flow.as_mut() {
            match &mut flow.step {
                SetDonationDetailsStep::List => draw_donation_details_list(frame, area, flow),
                SetDonationDetailsStep::FieldForm { .. } => {
                    draw_donation_details_form(frame, area, &mut flow.step)
                }
            }
        }
    }
    if let Some(modal) = app.set_donation_details_modal.as_ref() {
        draw_donation_details_modal(frame, area, modal);
    }
    // Link-transfer flow overlay.
    if app.link_transfer_flow.is_some() {
        let is_out_list = matches!(
            app.link_transfer_flow.as_ref().map(|f| &f.step),
            Some(LinkTransferStep::OutList)
        );
        if is_out_list {
            if let Some(flow) = app.link_transfer_flow.as_mut() {
                draw_link_transfer_out_list(frame, area, flow);
            }
        } else if let Some(flow) = app.link_transfer_flow.as_mut() {
            draw_link_transfer_target_pick(frame, area, flow);
        }
    }
    if let Some(modal) = app.link_transfer_modal.as_ref() {
        draw_link_transfer_modal(frame, area, modal);
    }
    // Classify-raw flow overlay.
    if app.classify_raw_flow.is_some() {
        let is_list = matches!(
            app.classify_raw_flow.as_ref().map(|f| &f.step),
            Some(ClassifyRawStep::List)
        );
        if is_list {
            if let Some(flow) = app.classify_raw_flow.as_mut() {
                draw_classify_raw_list(frame, area, flow);
            }
        } else if let Some(flow) = app.classify_raw_flow.as_ref() {
            draw_classify_raw_form(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.classify_raw_modal.as_ref() {
        draw_classify_raw_modal(frame, area, modal);
    }
    if let Some(flow) = app.safe_harbor_attest_flow.as_ref() {
        match &flow.step {
            SafeHarborAttestStep::Info => draw_attest_info(frame, area, flow),
            SafeHarborAttestStep::TypedWord { .. } => {
                draw_attest_typed_word(frame, area, &flow.step)
            }
        }
    }
    // Resolve-conflict flow overlay.
    if app.resolve_conflict_flow.is_some() {
        let is_list = matches!(
            app.resolve_conflict_flow.as_ref().map(|f| &f.step),
            Some(ResolveConflictStep::List)
        );
        if is_list {
            if let Some(flow) = app.resolve_conflict_flow.as_mut() {
                draw_resolve_conflict_list(frame, area, flow);
            }
        } else if let Some(flow) = app.resolve_conflict_flow.as_ref() {
            draw_resolve_conflict_choose(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.resolve_conflict_modal.as_ref() {
        draw_resolve_conflict_modal(frame, area, modal);
    }
    // Optimize-accept flow overlay.
    if app.optimize_accept_flow.is_some() {
        let is_list = matches!(
            app.optimize_accept_flow.as_ref().map(|f| &f.step),
            Some(OptimizeAcceptStep::List)
        );
        if is_list {
            if let Some(flow) = app.optimize_accept_flow.as_mut() {
                draw_optimize_accept_list(frame, area, flow);
            }
        } else if let Some(flow) = app.optimize_accept_flow.as_ref() {
            draw_optimize_accept_attest_text(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.optimize_accept_modal.as_ref() {
        draw_optimize_accept_modal(frame, area, modal);
    }
    // Safe-harbor-allocate flow overlay (chunk 5, D2/D4).
    if let Some(flow) = app.safe_harbor_allocate_flow.as_mut() {
        draw_safe_harbor_allocate_preview(frame, area, flow);
    }
    if let Some(modal) = app.safe_harbor_allocate_modal.as_ref() {
        draw_safe_harbor_allocate_modal(frame, area, modal);
    }
    // Bulk-link-transfer flow overlay (bulk-link-transfer D3).
    if let Some(flow) = app.bulk_link_flow.as_mut() {
        draw_bulk_link_flow(frame, area, flow);
    }
    if let Some(modal) = app.bulk_link_modal.as_ref() {
        draw_bulk_link_modal(frame, area, modal);
    }
    // Bulk classify-inbound-self-transfer flow overlay (bulk-classify-inbound-self-transfer D3).
    if let Some(flow) = app.bulk_sti_flow.as_mut() {
        draw_bulk_sti_flow(frame, area, flow);
    }
    if let Some(modal) = app.bulk_sti_modal.as_ref() {
        draw_bulk_sti_modal(frame, area, modal);
    }
    // Pseudo-reconcile approve confirmation modal (sub-project 2).
    if let Some(modal) = app.pseudo_approve_modal.as_ref() {
        draw_pseudo_approve_modal(frame, area, modal);
    }
    // Bulk classify-inbound-income flow overlay (bulk-classify-inbound-income, Cycle 4).
    if let Some(flow) = app.bulk_income_flow.as_mut() {
        draw_bulk_income_flow(frame, area, flow);
    }
    if let Some(modal) = app.bulk_income_modal.as_ref() {
        draw_bulk_income_modal(frame, area, modal);
    }
    // Bulk resolve-conflict flow overlay (bulk-resolve-conflict D3).
    if let Some(flow) = app.bulk_resolve_flow.as_mut() {
        draw_bulk_resolve_flow(frame, area, flow);
    }
    if let Some(modal) = app.bulk_resolve_modal.as_ref() {
        draw_bulk_resolve_modal(frame, area, modal);
    }
    // Bulk-void flow overlay (bulk-void D3).
    if let Some(flow) = app.bulk_void_flow.as_mut() {
        draw_bulk_void_flow(frame, area, flow);
    }
    if let Some(modal) = app.bulk_void_modal.as_ref() {
        draw_bulk_void_modal(frame, area, modal);
    }
    // Bulk reclassify-outflow flow overlay (bulk-reclassify-outflow, Cycle 5).
    if let Some(flow) = app.bulk_reclassify_outflow_flow.as_mut() {
        draw_bulk_reclassify_outflow_flow(frame, area, flow);
    }
    if let Some(modal) = app.bulk_reclassify_outflow_modal.as_ref() {
        draw_bulk_reclassify_outflow_modal(frame, area, modal);
    }
    // Match-self-transfers flow overlay (self-transfer-passthrough C3).
    if let Some(flow) = app.match_self_transfers_flow.as_mut() {
        draw_match_self_transfers_flow(frame, area, flow);
    }
    if let Some(modal) = app.match_self_transfers_modal.as_ref() {
        draw_match_self_transfers_modal(frame, area, modal);
    }
    // Method-election flow overlay (§A.5(a) per-account cost-basis method).
    if app.method_election_flow.is_some() {
        let is_list = matches!(
            app.method_election_flow.as_ref().map(|f| &f.step),
            Some(MethodElectionStep::List)
        );
        if is_list {
            if let Some(flow) = app.method_election_flow.as_mut() {
                draw_method_election_list(frame, area, flow);
            }
        } else if let Some(flow) = app.method_election_flow.as_ref() {
            draw_method_election_choose(frame, area, &flow.step);
        }
    }
    if let Some(modal) = app.method_election_modal.as_ref() {
        draw_method_election_modal(frame, area, modal);
    }
}

/// Render the tax-profile form overlaid on the Browse screen.
fn draw_profile_form(frame: &mut Frame, area: Rect, form: &ProfileFormState) {
    let modal_width: u16 = 70;
    let modal_height: u16 = 16; // 1 filing_status + 9 fields + 3 (error/hints/border)
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    // Build content lines
    let filing_tag = match form.filing_status {
        btctax_core::FilingStatus::Single => "single",
        btctax_core::FilingStatus::Mfj => "mfj",
        btctax_core::FilingStatus::Mfs => "mfs",
        btctax_core::FilingStatus::HoH => "hoh",
        btctax_core::FilingStatus::Qss => "qss",
    };

    let focus_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default();

    let inner_width = modal_rect.width.saturating_sub(2) as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Row 0: filing_status
    let fs_style = if form.focus == 0 {
        focus_style
    } else {
        normal_style
    };
    lines.push(Line::from(vec![
        Span::styled(format!("  filing_status: [{filing_tag}]"), fs_style),
        Span::raw("  (Tab to cycle)"),
    ]));

    // Rows 1-9: money fields
    for (i, label) in FIELD_LABELS.iter().enumerate() {
        let field_style = if form.focus == i + 1 {
            focus_style
        } else {
            normal_style
        };
        let content = &form.fields[i].buf;
        let display = format!("  {label}: [{content}]");
        let display = if display.len() > inner_width {
            display[..inner_width].to_string()
        } else {
            display
        };
        lines.push(Line::from(Span::styled(display, field_style)));
    }

    // Error line
    if let Some(err) = form.error.as_deref() {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(Color::Red),
        )));
    } else {
        lines.push(Line::from(""));
    }

    // Hints
    lines.push(Line::from(Span::styled(
        "  [Enter] Submit   [↑/↓] Move   [Tab] Cycle status   [Esc] Cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .title(format!(" Tax Profile for {} — EDITOR ", form.year))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, modal_rect);
}

/// Render the mutation-confirmation modal overlaid on the Browse screen.
///
/// Shows the EXACT validated payload (all 10 leaf fields + year) before writing.
/// Follows the spec's payload-showing modal (D4).
fn draw_mutation_modal(frame: &mut Frame, area: Rect, modal: &MutationModalState) {
    let p = &modal.profile;
    let fs_tag = btctax_cli::render::filing_status_tag(p.filing_status);

    // Single-spaced per the D4 mock: 10 leaf fields + year + notes + legend must ALL
    // fit inside a standard 80x24 terminal (centered_rect clamps height to the area;
    // the payload-showing guarantee requires every field AND the Enter/Esc legend
    // visible — double-spacing would clip the bottom fields and the legend).
    let content = format!(
        "  year: {year}\n\
           filing_status: {fs}\n\
           ordinary_taxable_income: {oti}\n\
           magi_excluding_crypto: {magi}\n\
           qualified_dividends_and_other_pref_income: {qd}\n\
           other_net_capital_gain: {oncg}\n\
           capital_loss_carryforward_in.short: {cfs}\n\
           capital_loss_carryforward_in.long: {cfl}\n\
           w2_ss_wages: {w2ss}\n\
           w2_medicare_wages: {w2med}\n\
           schedule_c_expenses: {sce}\n\
         \n\
           Replaces any existing profile for this year (upsert).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        year = modal.year,
        fs = fs_tag,
        oti = p.ordinary_taxable_income,
        magi = p.magi_excluding_crypto,
        qd = p.qualified_dividends_and_other_pref_income,
        oncg = p.other_net_capital_gain,
        cfs = p.capital_loss_carryforward_in.short,
        cfl = p.capital_loss_carryforward_in.long,
        w2ss = p.w2_ss_wages,
        w2med = p.w2_medicare_wages,
        sce = p.schedule_c_expenses,
    );

    let modal_width: u16 = 64;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(10);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(format!(
            " Confirm: set tax profile for {} — WRITES THE VAULT ",
            modal.year
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Classify-inbound flow draw functions ──────────────────────────────────────

/// Render the classify-inbound target list overlay.
///
/// Receives `&mut ClassifyInboundFlowState` to call `render_stateful_widget` on the
/// `TableState` (stateful widget requires `&mut TableState`).
fn draw_classify_inbound_list(
    frame: &mut Frame,
    area: Rect,
    flow: &mut crate::edit::form::ClassifyInboundFlowState,
) {
    let modal_width: u16 = 90;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Classify Inbound — select TransferIn target  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Sat"),
        Cell::from("Wallet"),
        Cell::from("EventId"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = if flow.list.items.is_empty() {
        vec![Row::new(vec![Cell::from(
            "(no unclassified inbound transfers)",
        )])]
    } else {
        flow.list
            .items
            .iter()
            .map(|item| {
                let wallet_str = match &item.wallet {
                    Some(btctax_core::WalletId::Exchange { provider, account }) => {
                        format!("{provider}/{account}")
                    }
                    Some(btctax_core::WalletId::SelfCustody { label }) => label.clone(),
                    None => "(no wallet)".to_string(),
                };
                Row::new(vec![
                    Cell::from(item.date.to_string()),
                    Cell::from(item.sat.to_string()),
                    Cell::from(wallet_str),
                    Cell::from(item.blocker_event.canonical()),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(16),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    // Footer hint
    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the classify-inbound variant picker or field form overlay.
fn draw_classify_inbound_form(frame: &mut Frame, area: Rect, step: &ClassifyInboundStep) {
    let modal_width: u16 = 70;
    let modal_height: u16 = 16;
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let focus_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default();

    let (title, content) = match step {
        ClassifyInboundStep::VariantPicker { item, variant } => {
            let variant_str = match variant {
                InboundVariant::Income => "> Income    GiftReceived    SelfTransferMine",
                InboundVariant::GiftReceived => "  Income  > GiftReceived    SelfTransferMine",
                InboundVariant::SelfTransferMine => "  Income    GiftReceived  > SelfTransferMine",
            };
            let c = format!(
                "  target: {target}\n\
                 \n\
                   Select variant (Tab to cycle, Enter to confirm):\n\
                 \n\
                 {variant_str}\n\
                 \n\
                 \n  Esc: back to list",
                target = item.blocker_event.canonical(),
                variant_str = variant_str,
            );
            (" Classify Inbound — variant picker  [EDITOR] ", c)
        }
        ClassifyInboundStep::IncomeForm {
            item,
            kind,
            fmv_buf,
            business,
            focus,
            error,
        } => {
            let kind_line = format!(
                "  {} kind:     {}  (Tab: cycle Mining/Staking/Interest/Airdrop/Reward)",
                if *focus == 0 { ">" } else { " " },
                income_kind_display(*kind),
            );
            let fmv_line = format!(
                "  {} fmv (USD): {}  (empty = FmvMissing will fire)",
                if *focus == 1 { ">" } else { " " },
                fmv_buf.buf,
            );
            let biz_line = format!(
                "  {} business:  {}  (Space: toggle)",
                if *focus == 2 { ">" } else { " " },
                business,
            );
            let err_line = error
                .as_deref()
                .map(|e| format!("\n  Error: {e}"))
                .unwrap_or_default();
            let c = format!(
                "  target: {target}\n\
                 \n\
                 {kind_line}\n\
                 {fmv_line}\n\
                 {biz_line}\
                 {err_line}\n\
                 \n\
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move focus",
                target = item.blocker_event.canonical(),
            );
            (" Classify Inbound — Income  [EDITOR] ", c)
        }
        ClassifyInboundStep::GiftForm {
            item,
            fmv_at_gift_buf,
            donor_basis_buf,
            donor_acquired_at_buf,
            focus,
            error,
        } => {
            let fmv_line = format!(
                "  {} fmv_at_gift (USD) [REQUIRED]: {}",
                if *focus == 0 { ">" } else { " " },
                fmv_at_gift_buf.buf,
            );
            let basis_line = format!(
                "  {} donor_basis (USD, optional):  {}",
                if *focus == 1 { ">" } else { " " },
                donor_basis_buf.buf,
            );
            let date_line = format!(
                "  {} donor_acquired_at (YYYY-MM-DD, optional): {}",
                if *focus == 2 { ">" } else { " " },
                donor_acquired_at_buf.buf,
            );
            let both_none_warn = if donor_basis_buf.is_empty() && donor_acquired_at_buf.is_empty() {
                "\n  NOTE: both donor fields empty → UnknownBasisInbound will re-fire."
            } else {
                ""
            };
            let err_line = error
                .as_deref()
                .map(|e| format!("\n  Error: {e}"))
                .unwrap_or_default();
            let c = format!(
                "  target: {target}\n\
                 \n\
                 {fmv_line}\n\
                 {basis_line}\n\
                 {date_line}\
                 {both_none_warn}\
                 {err_line}\n\
                 \n\
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move focus",
                target = item.blocker_event.canonical(),
            );
            (" Classify Inbound — GiftReceived  [EDITOR] ", c)
        }
        ClassifyInboundStep::SelfTransferForm {
            item,
            basis_buf,
            acquired_buf,
            focus,
            error,
        } => {
            let basis_line = format!(
                "  {} basis (USD, optional):  {}",
                if *focus == 0 { ">" } else { " " },
                basis_buf.buf,
            );
            let acquired_line = format!(
                "  {} acquired_at (YYYY-MM-DD, optional): {}",
                if *focus == 1 { ">" } else { " " },
                acquired_buf.buf,
            );
            let zero_basis_note = if basis_buf.is_empty() {
                "\n  NOTE: empty basis → $0 default (non-gating advisory); supply real cost if known."
            } else {
                ""
            };
            let err_line = error
                .as_deref()
                .map(|e| format!("\n  Error: {e}"))
                .unwrap_or_default();
            let c = format!(
                "  target: {target}   (my own coins — non-taxable)\n\
                 \n\
                 {basis_line}\n\
                 {acquired_line}\
                 {zero_basis_note}\
                 {err_line}\n\
                 \n\
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move focus",
                target = item.blocker_event.canonical(),
            );
            (" Classify Inbound — SelfTransferMine  [EDITOR] ", c)
        }
        // List step is rendered by draw_classify_inbound_list.
        ClassifyInboundStep::List => ("", String::new()),
    };

    if title.is_empty() {
        return; // defensive; List is handled separately
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
    let _ = focus_style;
    let _ = normal_style;
}

/// Render the classify-inbound confirmation modal.
fn draw_classify_inbound_modal(frame: &mut Frame, area: Rect, modal: &ClassifyInboundModalState) {
    let as_content = match &modal.as_ {
        InboundClass::Income {
            kind,
            fmv,
            business,
        } => {
            let fmv_str = fmv
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(empty = FmvMissing will fire)".to_string());
            format!(
                "  as: Income\n\n    kind:     {kind}\n    fmv:      {fmv_str}\n    business: {business}",
                kind = income_kind_display(*kind),
            )
        }
        InboundClass::GiftReceived {
            donor_basis,
            donor_acquired_at,
            fmv_at_gift,
        } => {
            let basis_str = donor_basis
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(empty = unknown)".to_string());
            let date_str = donor_acquired_at
                .map(|d| d.to_string())
                .unwrap_or_else(|| "(empty = unknown)".to_string());
            let both_none_warn = if donor_basis.is_none() && donor_acquired_at.is_none() {
                "\n\n  WARNING: both donor fields empty → UnknownBasisInbound will re-fire."
            } else {
                ""
            };
            format!(
                "  as: GiftReceived\n\n    fmv_at_gift:       {fmv_at_gift}   (REQUIRED)\n    donor_basis:       {basis_str}\n    donor_acquired_at: {date_str}{both_none_warn}",
            )
        }
        InboundClass::SelfTransferMine { basis, acquired_at } => {
            let basis_str = basis
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(empty = default $0, conservative)".to_string());
            let date_str = acquired_at
                .map(|d| d.to_string())
                // The engine defaults to `long_term_default_acquired` = 1 yr + 1 day before receipt →
                // LONG-TERM (btctax_core `long_term_default_acquired`, applied in the reconcile fold; the
                // `reconcile classify-inbound-self-transfer --acquired` help states the same). The modal is
                // the informed-consent point, so it must state the rate-determining default correctly
                // (was backwards: "receipt date, short-term").
                .unwrap_or_else(|| {
                    "(empty = default = 1 yr + 1 day before receipt \u{2192} long-term)".to_string()
                });
            let zero_basis_note = if basis.is_none() {
                "\n\n  NOTE: basis defaults to $0 (non-gating advisory) — supply real cost if you have it."
            } else {
                ""
            };
            format!(
                "  as: SelfTransferMine (my own coins — non-taxable)\n\n    basis:       {basis_str}\n    acquired_at: {date_str}{zero_basis_note}",
            )
        }
    };

    let content = format!(
        "  target:  {target}  (TransferIn)\n\
           date:    {date}\n\
           sat:     {sat}\n\
         \n\
         {as_content}\n\
         \n\
           Appended as a decision event (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        target = modal.target_event.canonical(),
        date = modal.target_date,
        sat = modal.target_sat,
    );

    let modal_width: u16 = 68;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(12);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: classify-inbound — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Reclassify-outflow flow draw functions ────────────────────────────────────

/// Render the reclassify-outflow target list overlay.
///
/// Receives `&mut ReclassifyOutflowFlowState` to call `render_stateful_widget`.
fn draw_reclassify_outflow_list(
    frame: &mut Frame,
    area: Rect,
    flow: &mut crate::edit::form::ReclassifyOutflowFlowState,
) {
    let modal_width: u16 = 90;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Reclassify Outflow — select pending TransferOut target  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Principal Sat"),
        Cell::from("Wallet"),
        Cell::from("EventId"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = if flow.list.items.is_empty() {
        vec![Row::new(vec![Cell::from(
            "(no pending outbound transfers)",
        )])]
    } else {
        flow.list
            .items
            .iter()
            .map(|item| {
                let wallet_str = match &item.wallet {
                    Some(btctax_core::WalletId::Exchange { provider, account }) => {
                        format!("{provider}/{account}")
                    }
                    Some(btctax_core::WalletId::SelfCustody { label }) => label.clone(),
                    None => "(no wallet)".to_string(),
                };
                Row::new(vec![
                    Cell::from(item.date.to_string()),
                    Cell::from(item.principal_sat.to_string()),
                    Cell::from(wallet_str),
                    Cell::from(item.transfer_out_event.canonical()),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Length(14),
        Constraint::Length(16),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    // Footer hint
    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the reclassify-outflow kind picker or field form overlay.
fn draw_reclassify_outflow_form(frame: &mut Frame, area: Rect, step: &ReclassifyOutflowStep) {
    let modal_width: u16 = 72;
    let modal_height: u16 = 18;
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let (title, content) = match step {
        ReclassifyOutflowStep::KindPicker { item, kind } => {
            let kind_row = |tag: &str, k: OutflowKind| {
                if *kind == k {
                    format!("> {tag}")
                } else {
                    format!("  {tag}")
                }
            };
            let c = format!(
                "  target: {target}\n\
                 \n\
                   Select kind (Tab to cycle, Enter to confirm):\n\
                 \n\
                 {sell}   {spend}   {gift}   {donate}\n\
                 \n\
                 \n  Esc: back to list",
                target = item.transfer_out_event.canonical(),
                sell = kind_row("sell", OutflowKind::Sell),
                spend = kind_row("spend", OutflowKind::Spend),
                gift = kind_row("gift", OutflowKind::Gift),
                donate = kind_row("donate", OutflowKind::Donate),
            );
            (" Reclassify Outflow — kind picker  [EDITOR] ", c)
        }
        ReclassifyOutflowStep::FieldForm {
            item,
            kind,
            amount_buf,
            fee_buf,
            appraisal,
            donee_buf,
            focus,
            error,
        } => {
            let lbl = amount_label(*kind);
            let amount_line = format!(
                "  {} {lbl}: {}",
                if *focus == 0 { ">" } else { " " },
                amount_buf.buf,
            );
            let fee_line = format!(
                "  {} fee (USD, optional): {}",
                if *focus == 1 { ">" } else { " " },
                fee_buf.buf,
            );
            // Appraisal row: shown only for donate.
            let appraisal_line = if *kind == OutflowKind::Donate {
                format!(
                    "\n  {} appraisal required: {}  (Space: toggle)",
                    if *focus == 2 { ">" } else { " " },
                    appraisal,
                )
            } else {
                String::new()
            };
            // Donee row: shown for gift and donate.
            let donee_line = if matches!(kind, OutflowKind::Gift | OutflowKind::Donate) {
                format!(
                    "\n  {} donee (free-form, optional): {}",
                    if *focus == 3 { ">" } else { " " },
                    donee_buf.buf,
                )
            } else {
                String::new()
            };
            let err_line = error
                .as_deref()
                .map(|e| format!("\n  Error: {e}"))
                .unwrap_or_default();
            let c = format!(
                "  target: {target}\n\
                 \n\
                 {amount_line}\n\
                 {fee_line}{appraisal_line}{donee_line}\
                 {err_line}\n\
                 \n\
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move",
                target = item.transfer_out_event.canonical(),
            );
            (
                match kind {
                    OutflowKind::Sell => " Reclassify Outflow — Sell  [EDITOR] ",
                    OutflowKind::Spend => " Reclassify Outflow — Spend  [EDITOR] ",
                    OutflowKind::Gift => " Reclassify Outflow — Gift  [EDITOR] ",
                    OutflowKind::Donate => " Reclassify Outflow — Donate  [EDITOR] ",
                },
                c,
            )
        }
        // List step is rendered by draw_reclassify_outflow_list.
        ReclassifyOutflowStep::List => ("", String::new()),
    };

    if title.is_empty() {
        return; // defensive
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

/// Render the reclassify-outflow confirmation modal.
///
/// Shows the complete payload (target, principal_sat, kind, amount, fee, appraisal, donee).
/// Donee is shown for BOTH gift and donate [R0-I7].
fn draw_reclassify_outflow_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &ReclassifyOutflowModalState,
) {
    let ro = &modal.payload;

    let kind_section = match &ro.as_ {
        OutflowClass::Dispose {
            kind: DisposeKind::Sell,
        } => {
            let fee_str = ro
                .fee_usd
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none)".to_string());
            format!(
                "  as: sell\n    gross_proceeds: {proceeds}\n    fee_usd:        {fee_str}",
                proceeds = ro.principal_proceeds_or_fmv,
            )
        }
        OutflowClass::Dispose {
            kind: DisposeKind::Spend,
        } => {
            let fee_str = ro
                .fee_usd
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none)".to_string());
            format!(
                "  as: spend\n    gross_proceeds: {proceeds}\n    fee_usd:        {fee_str}",
                proceeds = ro.principal_proceeds_or_fmv,
            )
        }
        OutflowClass::GiftOut => {
            let fee_str = ro
                .fee_usd
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none)".to_string());
            let donee_str = ro.donee.as_deref().unwrap_or("(none)");
            format!(
                "  as: gift\n    fmv:     {fmv}\n    fee_usd: {fee_str}\n    donee:   {donee_str}",
                fmv = ro.principal_proceeds_or_fmv,
            )
        }
        OutflowClass::Donate { appraisal_required } => {
            let fee_str = ro
                .fee_usd
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none)".to_string());
            let donee_str = ro.donee.as_deref().unwrap_or("(none)");
            format!(
                "  as: donate\n    fmv:                {fmv}\n    fee_usd:            {fee_str}\n    appraisal_required: {appraisal_required}\n    donee:              {donee_str}",
                fmv = ro.principal_proceeds_or_fmv,
            )
        }
    };

    let content = format!(
        "  target:        {target}  (TransferOut)\n\
           date:          {date}\n\
           principal_sat: {sat}\n\
         \n\
         {kind_section}\n\
         \n\
           Appended as a decision event (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        target = modal.target_event.canonical(),
        date = modal.target_date,
        sat = modal.principal_sat,
    );

    let modal_width: u16 = 70;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(12);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: reclassify-outflow — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Reclassify-income flow draw functions ─────────────────────────────────────

/// Render the reclassify-income target list overlay.
fn draw_reclassify_income_list(
    frame: &mut Frame,
    area: Rect,
    flow: &mut ReclassifyIncomeFlowState,
) {
    let modal_width: u16 = 100;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Reclassify Income — select Income event target  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Sat"),
        Cell::from("Kind"),
        Cell::from("Business"),
        Cell::from("FMV"),
        Cell::from("EventId"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = if flow.list.items.is_empty() {
        vec![Row::new(vec![Cell::from(
            "(no reclassifiable income events)",
        )])]
    } else {
        flow.list
            .items
            .iter()
            .map(|item| {
                let fmv_str = item
                    .fmv
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "(pending)".to_string());
                Row::new(vec![
                    Cell::from(item.date.to_string()),
                    Cell::from(item.sat.to_string()),
                    Cell::from(income_kind_display(item.kind)),
                    Cell::from(item.business.to_string()),
                    Cell::from(fmv_str),
                    Cell::from(item.income_event.canonical()),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the reclassify-income field form overlay.
fn draw_reclassify_income_form(frame: &mut Frame, area: Rect, step: &ReclassifyIncomeStep) {
    let modal_width: u16 = 72;
    let modal_height: u16 = 14;
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let ReclassifyIncomeStep::FieldForm {
        item,
        business,
        kind,
        focus,
        error,
    } = step
    else {
        return;
    };

    let biz_display = match business {
        None => "---  [required]",
        Some(true) => "true",
        Some(false) => "false",
    };
    let kind_display = match kind {
        None => "keep original",
        Some(k) => income_kind_display(*k),
    };

    let biz_line = format!(
        "  {} business: {}  (Tab: cycle true/false/---)",
        if *focus == 0 { ">" } else { " " },
        biz_display,
    );
    let kind_line = format!(
        "  {} kind:     {}  (Tab: cycle None/Mining/Staking/Interest/Airdrop/Reward)",
        if *focus == 1 { ">" } else { " " },
        kind_display,
    );
    let err_line = error
        .as_deref()
        .map(|e| format!("\n  Error: {e}"))
        .unwrap_or_default();

    let content = format!(
        "  target: {target}\n\
         \n\
         {biz_line}\n\
         {kind_line}\
         {err_line}\n\
         \n\
         \n  Enter: validate   Esc: back to list   ↑/↓: move focus",
        target = item.income_event.canonical(),
    );

    let block = Block::default()
        .title(" Reclassify Income — field form  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

/// Render the reclassify-income confirmation modal.
fn draw_reclassify_income_modal(frame: &mut Frame, area: Rect, modal: &ReclassifyIncomeModalState) {
    // Spec D1: when kind is Some(k), show "{display} (was {original})";
    // when None, show "keep original".
    let new_kind_display = match modal.new_kind {
        Some(k) => format!(
            "{} (was {})",
            income_kind_display(k),
            income_kind_display(modal.original_kind)
        ),
        None => "keep original".to_string(),
    };

    let content = format!(
        "  target:  {target}   (Income)\n\
           date:    {date}\n\
           sat:     {sat}\n\
         \n\
           original: kind={orig_kind}  business={orig_biz}\n\
           override:\n\
             business: {new_biz}    (was {orig_biz})\n\
             kind:     {new_kind}\n\
         \n\
           Effects: income_recognized updates; SE/NIIT exposure\n\
           may change depending on the flip direction.\n\
         \n\
           Appended as a decision event (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        target = modal.target_event.canonical(),
        date = modal.target_date,
        sat = modal.target_sat,
        orig_kind = income_kind_display(modal.original_kind),
        orig_biz = modal.original_business,
        new_biz = modal.new_business,
        new_kind = new_kind_display,
    );

    let modal_width: u16 = 64;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(14);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: reclassify-income — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Set-FMV flow draw functions ───────────────────────────────────────────────

/// Render the set-fmv target list overlay.
fn draw_set_fmv_list(frame: &mut Frame, area: Rect, flow: &mut SetFmvFlowState) {
    let modal_width: u16 = 90;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Set FMV — select FmvMissing Income event  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Sat"),
        Cell::from("Kind"),
        Cell::from("EventId"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = if flow.list.items.is_empty() {
        vec![Row::new(vec![Cell::from("(no FMV-missing income events)")])]
    } else {
        flow.list
            .items
            .iter()
            .map(|item| {
                Row::new(vec![
                    Cell::from(item.date.to_string()),
                    Cell::from(item.sat.to_string()),
                    Cell::from(income_kind_display(item.kind)),
                    Cell::from(item.event.canonical()),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the set-fmv field form overlay.
fn draw_set_fmv_form(frame: &mut Frame, area: Rect, step: &SetFmvStep) {
    let modal_width: u16 = 70;
    let modal_height: u16 = 12;
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let SetFmvStep::FieldForm {
        item,
        usd_fmv_buf,
        error,
    } = step
    else {
        return;
    };

    let fmv_line = format!("  > usd_fmv (USD) [REQUIRED]: {}", usd_fmv_buf.buf);
    let err_line = error
        .as_deref()
        .map(|e| format!("\n  Error: {e}"))
        .unwrap_or_default();

    let content = format!(
        "  target: {target}\n\
         \n\
         {fmv_line}\
         {err_line}\n\
         \n\
         \n  Enter: validate   Esc: back to list",
        target = item.event.canonical(),
    );

    let block = Block::default()
        .title(" Set FMV — field form  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

/// Render the set-fmv confirmation modal.
fn draw_set_fmv_modal(frame: &mut Frame, area: Rect, modal: &SetFmvModalState) {
    let content = format!(
        "  target:  {target}   (Income)\n\
           date:    {date}\n\
           sat:     {sat}\n\
           kind:    {kind}\n\
         \n\
           usd_fmv: {usd_fmv}   (REQUIRED — sets the income FMV)\n\
         \n\
           Effects: FmvMissing blocker will clear; income_recognized\n\
           will gain an entry with this FMV.\n\
         \n\
           Appended as a decision event (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        target = modal.target_event.canonical(),
        date = modal.target_date,
        sat = modal.target_sat,
        kind = income_kind_display(modal.target_kind),
        usd_fmv = modal.usd_fmv,
    );

    let modal_width: u16 = 64;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(14);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: set-fmv — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Method-election flow draw functions (§A.5(a) per-account cost-basis method) ─

/// Render the method-election account list: each Exchange account, its currently-resolved method, and
/// whether that method is an explicit per-account election ("elected") or inherited (global / FIFO).
fn draw_method_election_list(frame: &mut Frame, area: Rect, flow: &mut MethodElectionFlowState) {
    let modal_width: u16 = 84;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Method Election — select exchange account  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Account"),
        Cell::from("Resolved method"),
        Cell::from("Source"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = if flow.list.items.is_empty() {
        vec![Row::new(vec![Cell::from("(no exchange accounts)")])]
    } else {
        flow.list
            .items
            .iter()
            .map(|item| {
                Row::new(vec![
                    Cell::from(wallet_label(&item.wallet)),
                    Cell::from(lot_method_label(item.current)),
                    Cell::from(if item.scoped {
                        "elected (per-account)"
                    } else {
                        "inherited (global/FIFO)"
                    }),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Min(28),
        Constraint::Length(16),
        Constraint::Length(24),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: choose method   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the method-election Choose step: a single FIFO/HIFO/LIFO picker (Tab cycles).
fn draw_method_election_choose(frame: &mut Frame, area: Rect, step: &MethodElectionStep) {
    let modal_width: u16 = 72;
    let modal_height: u16 = 12;
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let MethodElectionStep::Choose {
        item,
        method,
        error,
    } = step
    else {
        return;
    };

    let method_line = format!(
        "  > method: {}   (Tab: cycle FIFO/HIFO/LIFO)",
        lot_method_label(*method)
    );
    let err_line = error
        .as_deref()
        .map(|e| format!("\n  Error: {e}"))
        .unwrap_or_default();

    let content = format!(
        "  account: {account}\n\
           currently resolved: {current} ({src})\n\
         \n\
         {method_line}\
         {err_line}\n\
         \n\
         \n  Enter: attest & confirm   Esc: back to list",
        account = wallet_label(&item.wallet),
        current = lot_method_label(item.current),
        src = if item.scoped { "elected" } else { "inherited" },
    );

    let block = Block::default()
        .title(" Method Election — choose method  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

/// Render the method-election confirmation ("attest") modal.
fn draw_method_election_modal(frame: &mut Frame, area: Rect, modal: &MethodElectionModalState) {
    let content = format!(
        "  account: {account}\n\
           method:  {method}\n\
         \n\
           Setting a per-account method IS the attestation: this affirms you\n\
           use/elected {method} for {account} (IRS 2025+ per-account rule).\n\
         \n\
           A forward standing order you can update going forward. Governs\n\
           method-honoring disposals on this account on/after today.\n\
         \n\
           Appended as a decision event (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & attest     [Esc] Cancel — writes nothing",
        account = wallet_label(&modal.wallet),
        method = lot_method_label(modal.method),
    );

    let modal_width: u16 = 66;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(14);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: method-election — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Void-decision flow draw functions ─────────────────────────────────────────

/// Render the void-decision target list overlay.
///
/// Columns: Seq | Type | Target summary.
/// The void flow has NO FieldForm step — Enter from the list goes DIRECTLY to the modal.
fn draw_void_list(frame: &mut Frame, area: Rect, flow: &mut VoidFlowState) {
    let modal_width: u16 = 100;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Void Decision — select decision to void  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Seq"),
        Cell::from("Type"),
        Cell::from("Target summary"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = if flow.list.items.is_empty() {
        vec![Row::new(vec![Cell::from("(no revocable decisions)")])]
    } else {
        flow.list
            .items
            .iter()
            .map(|item| {
                Row::new(vec![
                    Cell::from(item.seq.to_string()),
                    Cell::from(item.payload_tag),
                    Cell::from(item.target_summary.clone()),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Length(8),
        Constraint::Length(24),
        Constraint::Min(40),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select → modal   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the void-decision confirmation modal.
///
/// Shows the decision being voided + the full cascade consequence note (D3.1).
/// Appends the SafeHarbor conditional warning when `modal.is_safe_harbor` is true.
fn draw_void_modal(frame: &mut Frame, area: Rect, modal: &VoidModalState) {
    let consequence = "\
  Consequence: this decision's effects un-project.\n\
  Prior blockers may return (e.g. voiding a ClassifyInbound\n\
  returns UnknownBasisInbound; the pending row re-lists).\n\
  Decisions that DEPENDED on this one (e.g. a ManualFmv or\n\
  ReclassifyIncome on a ClassifyRaw'd event, or a\n\
  LotSelection picking its lots) may now fire\n\
  DecisionConflict/LotSelectionInvalid — void those too.\n";

    let sha_warning = if modal.is_safe_harbor {
        "\n  WARNING: If this allocation is effective (Path B), voiding\n\
  it fires DecisionConflict — irrevocable (§7.4). If inert,\n\
  the void applies and the Path A default resumes.\n\
  A rejected void permanently removes this allocation from\n\
  this list (CLI void remains available).\n"
    } else {
        ""
    };

    let content = format!(
        "  decision: decision|{seq}  ({tag})\n\
           target:   {summary}\n\
         \n\
         {consequence}\
         {sha_warning}\
           Appended as a VoidDecisionEvent (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        seq = modal.seq,
        tag = modal.payload_tag,
        summary = modal.target_summary,
        consequence = consequence,
        sha_warning = sha_warning,
    );

    let modal_width: u16 = 70;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(16);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: void decision — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

/// Compute a centered `Rect` of the given dimensions within `area`.
/// The `?` full-keymap help overlay — a centered modal, the SAME on every tab (the reconcile action
/// keys are global). Lines are sized to fit an 80×24 terminal (no scroll; `centered_rect` truncates).
/// KEEP IN SYNC with the Browse key handler (`handle_key` in main.rs) — the KAT
/// `help_lists_every_browse_action_key` pins that every action key appears here.
/// Render the "tax inputs" editing flow — the 3-region layout (section list · field pane · status
/// line), a full-area overlay over Browse (plan 3 task 2).
///
/// ★ Branch order (review M7): `discard_offered` FIRST — the P2-a stale-parked state renders ONLY the
/// discard message (no editing surface). Then NI-2 (`working == None`) renders ONLY the filing-status
/// choice. Otherwise the full 3-region render over the LIVE sections.
///
/// All field access is through `form_spec()` accessors (`live_sections`/`live_fields`/`field.get`) —
/// this renderer NEVER names a `ReturnInputs` struct field (spec §9A/§13).
fn draw_tax_inputs_form(
    frame: &mut Frame,
    area: Rect,
    form: &TaxInputsFormState,
    status: Option<&str>,
) {
    frame.render_widget(Clear, area);

    // ★ P2-a FIRST: a stale PARKED draft is discard-only — no editing surface.
    if form.discard_offered {
        draw_tax_inputs_discard(frame, area, form, status);
        return;
    }

    // 3 regions: [left section list | right field pane] over a bottom status block. The block is 5 rows
    // (border + 3 content lines): active-source/screen-status, the key legend, and a NOTICE line that
    // surfaces `app.status`/the stale-WIP note inside the flow (I-2 — the overlay clears the Browse footer).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(5)])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(1)])
        .split(rows[0]);
    let (left_area, right_area, status_area) = (cols[0], cols[1], rows[1]);

    let cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let yellow = Style::default().fg(Color::Yellow);

    // ── NI-2: no return yet — show ONLY the filing-status choice ──────────────
    let Some(ri) = form.working.as_ref() else {
        let left = Paragraph::new(vec![Line::from("  Return options")]).block(
            Block::default()
                .title(" Sections ")
                .borders(Borders::ALL)
                .border_style(cyan),
        );
        frame.render_widget(left, left_area);

        let fs = filing_status_field();
        let choices = match fs.kind {
            FieldKind::Enum(opts) => opts.join(" / "),
            _ => String::new(),
        };
        let right = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("  {}", fs.label),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("    choices: {choices}")),
            Line::from(""),
            Line::from(Span::styled(
                "  no return yet — choose a filing status to begin (NI-2)",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .title(" Return options ")
                .borders(Borders::ALL)
                .border_style(yellow),
        );
        frame.render_widget(right, right_area);

        draw_tax_inputs_status(frame, status_area, form, status);
        return;
    };

    // ── Live sections (Some(ri)) ─────────────────────────────────────────────
    let sections = live_sections(ri);
    let sel = form.section_idx.min(sections.len().saturating_sub(1));

    // Left pane: the section list with a per-section status glyph + focus marker.
    let mut left_lines: Vec<Line> = Vec::with_capacity(sections.len());
    for (i, s) in sections.iter().enumerate() {
        let glyph = section_glyph(s, ri, form.refused_section);
        let marker = if i == sel { '>' } else { ' ' };
        let style = if i == sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        left_lines.push(Line::from(Span::styled(
            format!("{marker} {glyph} {}", s.title),
            style,
        )));
    }
    let left = Paragraph::new(left_lines).block(
        Block::default()
            .title(" Sections ")
            .borders(Borders::ALL)
            .border_style(cyan),
    );
    frame.render_widget(left, left_area);

    // Right pane: the pane the nav model projects for the current (section, addr, descent) — descending
    // into a nested group swaps in that group's sub-list / sub-row fields and title (Task-5 fix).
    let section = sections[sel];
    let (pane_title, right_lines) = field_pane_lines(form, section, ri);
    let right = Paragraph::new(right_lines).block(
        Block::default()
            .title(format!(" {pane_title} "))
            .borders(Borders::ALL)
            .border_style(yellow),
    );
    frame.render_widget(right, right_area);

    draw_tax_inputs_status(frame, status_area, form, status);

    // ★ Task 5: the remove-row payload-confirm, drawn ON TOP of the editing surface.
    if form.pending_remove.is_some() {
        draw_tax_inputs_remove_modal(frame, area, form);
    }

    // ★ Task 7: the commit payload-confirm, drawn ON TOP of the editing surface.
    if form.modal.is_some() {
        draw_tax_inputs_modal(frame, area, form);
    }
}

/// ★ Task 7: the commit payload-confirm modal — the summary (filing status, sections present, and the
/// shadow/all-zero warning when a tax-profile is shadowed) + the `[Enter] commit / [Esc] cancel` legend.
/// Mirrors `draw_mutation_modal`'s centered-`Clear` shape.
fn draw_tax_inputs_modal(frame: &mut Frame, area: Rect, form: &TaxInputsFormState) {
    use crate::edit::form::TaxInputsModalKind;
    let Some(m) = form.modal.as_ref() else {
        return;
    };
    // ★ Task 8: one modal serves three confirms — the header, title, and action verb branch on the kind.
    let (header, title, verb) = match m.kind {
        TaxInputsModalKind::Commit => (
            format!("  Commit tax inputs for {} — WRITES THE VAULT", m.year),
            format!(" Confirm commit for {} ", m.year),
            "commit",
        ),
        TaxInputsModalKind::ParkToProfile => (
            format!("  Park the full return for {} — WRITES THE VAULT", m.year),
            format!(" Switch to tax-profile for {} ", m.year),
            "park",
        ),
        TaxInputsModalKind::DiscardParked => (
            format!(
                "  Discard the parked draft for {} — WRITES THE VAULT",
                m.year
            ),
            format!(" Discard parked draft for {} ", m.year),
            "discard",
        ),
    };
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            header,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for s in m.summary.lines() {
        lines.push(Line::from(format!("  {s}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  [Enter] {verb}   [Esc] cancel — writes nothing"),
        Style::default().fg(Color::DarkGray),
    )));

    let height = (lines.len() as u16 + 2).max(10);
    let rect = centered_rect(70, height, area);
    frame.render_widget(Clear, rect);
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, rect);
}

/// ★ Task 5: the remove-row payload-confirm modal — names the exact row being removed ("remove W-2 #2?")
/// and its `[Enter] remove / [Esc] cancel` legend. Mirrors `draw_mutation_modal`'s centered-`Clear` shape.
fn draw_tax_inputs_remove_modal(frame: &mut Frame, area: Rect, form: &TaxInputsFormState) {
    let Some(pr) = form.pending_remove.as_ref() else {
        return;
    };
    let rect = centered_rect(60, 9, area);
    frame.render_widget(Clear, rect);
    let lines = vec![
        Line::from(Span::styled(
            format!("  {}", pr.label),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] remove   [Esc] cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let p = Paragraph::new(lines).block(
        Block::default()
            .title(" Confirm remove ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(p, rect);
}

/// ★ P2-a: the stale-PARKED-draft discard-only screen — the message + the back-out hint, NO editing
/// surface (Task 8 wires the 'X' → `discard_parked_draft`).
fn draw_tax_inputs_discard(
    frame: &mut Frame,
    area: Rect,
    form: &TaxInputsFormState,
    status: Option<&str>,
) {
    let rect = centered_rect(78, 14, area);
    frame.render_widget(Clear, rect);
    let mut v = vec![
        Line::from(Span::styled(
            format!("Stale parked draft for {}", form.year),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if let Some(err) = form.error.as_ref() {
        v.push(Line::from(err.clone()));
        v.push(Line::from(""));
    }
    // ★ I-2: a discard refusal / save error in this P2-a state routes to `app.status`; the full-frame Clear
    // hides the Browse footer, so surface it HERE (else the `X` failure looks identical to a success).
    if let Some(s) = status {
        v.push(Line::from(Span::styled(
            s.to_string(),
            Style::default().fg(Color::Yellow),
        )));
        v.push(Line::from(""));
    }
    v.push(Line::from(
        "Press X to discard the parked draft, Esc to back out.",
    ));
    let p = Paragraph::new(v).block(
        Block::default()
            .title(" Tax inputs — stale parked draft ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(p, rect);
}

/// The bottom status block (border + 3 content lines):
/// 1. the CACHED active source (`full return` / `tax-profile` / `(none)`, Task 8) + the §9A **screen
///    status** (`screens clean, except what report computes` / `1 issue: <section>` — I-4);
/// 2. the key legend (`t` toggle source, `X` discard-parked when parked; the close hint is dirty-aware — I-3);
/// 3. a NOTICE line surfacing `app.status` (I-2 — the overlay clears the Browse footer that normally renders
///    it) or, absent one, the §6.3 stale-WIP-discard note.
fn draw_tax_inputs_status(
    frame: &mut Frame,
    area: Rect,
    form: &TaxInputsFormState,
    status: Option<&str>,
) {
    // ★ I-4 (§9A): the screen status — a recorded screen refusal names its section; else clean-but-honest.
    let (screen_status, screen_style) = match form.refused_section {
        Some(id) => {
            let title = btctax_input_form::form_spec()
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.title)
                .unwrap_or("a section");
            (
                format!("1 issue: {title}"),
                Style::default().fg(Color::Yellow),
            )
        }
        None => (
            "screens clean, except what report computes".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    };
    // ★ I-3: the close hint reflects the real state — an unflushed (dirty) draft is NOT yet autosaved.
    let close_hint = if form.dirty {
        "[Esc/q] save & close"
    } else {
        "[Esc/q] close (autosaved)"
    };
    // `X` discard-parked is offered only when a parked draft is loaded (else the store would refuse it).
    let legend = if form.parked {
        format!("   [↑/↓] field · [←/→ or Tab] section · [s] commit · [t] source · [X] discard-parked · {close_hint}")
    } else {
        format!("   [↑/↓] field · [←/→ or Tab] section · [s] commit · [t] source · {close_hint}")
    };
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::raw("  active source: "),
            Span::styled(form.active_source_label, Style::default().fg(Color::Cyan)),
            Span::raw("   ·   "),
            Span::styled(screen_status, screen_style),
        ]),
        Line::from(Span::styled(legend, Style::default().fg(Color::DarkGray))),
    ];
    // ★ I-2: the NOTICE line — `app.status` (every in-flow refusal/error/outcome routed there stays VISIBLE)
    // takes precedence over the one-time §6.3 stale-WIP note, which shows only when there is no live status.
    if let Some(s) = status {
        lines.push(Line::from(Span::styled(
            format!("  {s}"),
            Style::default().fg(Color::Cyan),
        )));
    } else if let Some(note) = form.stale_note.as_ref() {
        lines.push(Line::from(Span::styled(
            format!("  {note}"),
            Style::default().fg(Color::Yellow),
        )));
    }
    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(p, area);
}

/// The field-pane lines + title for the CURRENT pane the nav model projects (Task 5 + the Task-5-fix
/// nested drill-down), all keyed off `edit::tax_inputs::active_pane` so the field cursor (`field_focus`)
/// never advances off the drawn pane (the Task-2 Minor fold):
/// - a `[create]` affordance for an ABSENT optional-singleton;
/// - the live `Field`s as `label  [value]` for a singleton / PRESENT optional-singleton (with an `[x]
///   delete` hint), PLUS a synthetic "… (n) →" drill entry when the section has a nested child;
/// - a selectable ROW LIST for a repeating group at its container path (`addr` empty), each row with an
///   index and a `[a] add / [d] remove / [Enter] edit row` legend;
/// - INSIDE a row (`addr` non-empty) that row's fields, read/written at `addr`, with a `[Left/Esc] back`
///   hint, PLUS the synthetic "Box 12 entries (n) →" drill entry;
/// - DESCENDED into a nested group (`form.descent` set) that group's sub-list or a sub-row's fields.
///
/// Returns the pane TITLE too (the nested group's title while descended, else the section's).
fn field_pane_lines(
    form: &TaxInputsFormState,
    section: &'static Section,
    ri: &ReturnInputs,
) -> (String, Vec<Line<'static>>) {
    use crate::edit::tax_inputs::{active_pane, nested_section, Pane};
    let dark = Style::default().fg(Color::DarkGray);
    let focus = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let addr = &form.addr;
    let field_focus = form.field_focus;
    let editing = form.editing;
    let buf = form.buf.as_str();
    let error = form.error.as_deref();
    let mut lines: Vec<Line> = Vec::new();

    // ── DESCENDED into a nested group: render its sub-list or a sub-row's fields (Task-5 fix). ──
    if let Some(nested_id) = form.descent {
        let nested = nested_section(nested_id);
        match active_pane(form) {
            Pane::RowList(n) => {
                if n == 0 {
                    lines.push(Line::from(Span::styled("  (no entries yet)", dark)));
                }
                for i in 0..n {
                    let is_focus = i == field_focus;
                    let marker = if is_focus { '>' } else { ' ' };
                    let style = if is_focus { focus } else { Style::default() };
                    lines.push(Line::from(Span::styled(
                        format!(
                            "{marker} #{}{}",
                            i + 1,
                            nested_row_preview(nested, ri, addr, i)
                        ),
                        style,
                    )));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  [a] add  [d] remove  [Enter] edit  [Left/Esc] back",
                    dark,
                )));
            }
            _ => {
                // A nested sub-row's fields (read/written at `addr`) + a back hint.
                if let Some(row) = addr.0.last() {
                    lines.push(Line::from(Span::styled(
                        format!("  {} #{}", nested.title, row + 1),
                        dark,
                    )));
                }
                push_field_lines(&mut lines, nested, ri, addr, field_focus, editing, buf);
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("  [Left/Esc] back", dark)));
            }
        }
        push_error(&mut lines, error);
        return (nested.title.to_string(), lines);
    }

    // An ABSENT optional-singleton shows a [create] affordance instead of fields.
    if let SectionKind::OptionalSingleton { present, .. } = section.kind {
        if !present(ri) {
            lines.push(Line::from("  (not present)"));
            lines.push(Line::from(Span::styled(
                "  [create] — press c to add this section",
                dark,
            )));
            push_error(&mut lines, error);
            return (section.title.to_string(), lines);
        }
    }

    // A repeating group: the ROW LIST at its container path (addr empty), or INSIDE a row (addr non-empty).
    if let SectionKind::Repeating { len, .. } = section.kind {
        if addr.0.is_empty() {
            let n = len(ri, &RowAddr::default());
            if n == 0 {
                lines.push(Line::from(Span::styled("  (no rows yet)", dark)));
            }
            for i in 0..n {
                let is_focus = i == field_focus;
                let marker = if is_focus { '>' } else { ' ' };
                let style = if is_focus { focus } else { Style::default() };
                lines.push(Line::from(Span::styled(
                    format!("{marker} #{}{}", i + 1, row_preview(section, ri, i)),
                    style,
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  [a] add  [d] remove  [Enter] edit row",
                dark,
            )));
            push_error(&mut lines, error);
            return (section.title.to_string(), lines);
        }
        // Inside a row: header + the row's fields (read/written at `addr`) + the nested drill entry + a back hint.
        if let Some(row) = addr.0.last() {
            lines.push(Line::from(Span::styled(
                format!("  {} #{}", section.title, row + 1),
                dark,
            )));
        }
        push_field_lines(&mut lines, section, ri, addr, field_focus, editing, buf);
        push_nested_drill_entry(&mut lines, form, section, ri);
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  [Left/Esc] back to rows", dark)));
        push_error(&mut lines, error);
        return (section.title.to_string(), lines);
    }

    // Singleton / PRESENT optional-singleton: the live fields as `label  [value]` + the nested drill entry.
    push_field_lines(&mut lines, section, ri, addr, field_focus, editing, buf);
    push_nested_drill_entry(&mut lines, form, section, ri);
    if matches!(section.kind, SectionKind::OptionalSingleton { .. }) {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  [x] delete this section", dark)));
    }
    push_error(&mut lines, error);
    (section.title.to_string(), lines)
}

/// Append the synthetic "Box 12 entries (n) →" / "Charitable gifts (n) →" drill entry to a parent-fields
/// pane whose section has a nested child (Task-5 fix). It is a navigable item at index == live-field count;
/// highlighted (yellow, bold) when the field cursor is on it — `Enter` there descends into the group.
fn push_nested_drill_entry(
    lines: &mut Vec<Line<'static>>,
    form: &TaxInputsFormState,
    section: &'static Section,
    ri: &ReturnInputs,
) {
    let Some(child) = crate::edit::tax_inputs::nested_child_here(form) else {
        return;
    };
    let (label, count) = nested_drill_summary(child, ri, &form.addr);
    let on_entry = form.field_focus == live_fields(section, ri).len() && !form.editing;
    let style = if on_entry {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    lines.push(Line::from(Span::styled(
        format!("  {label} ({count}) \u{2192}"),
        style,
    )));
}

/// The label + current row count for a nested group's synthetic drill entry, counted via the group's
/// `Repeating::len` accessor at its parent address (`[w2_i]` for box-12, `[]` for charitable).
fn nested_drill_summary(
    child: SectionId,
    ri: &ReturnInputs,
    parent_addr: &RowAddr,
) -> (&'static str, usize) {
    let count = match crate::edit::tax_inputs::nested_section(child).kind {
        SectionKind::Repeating { len, .. } => len(ri, parent_addr),
        _ => 0,
    };
    let label = match child {
        SectionId::W2Box12 => "Box 12 entries",
        SectionId::ScheduleACharitable => "Charitable gifts",
        _ => "entries",
    };
    (label, count)
}

/// A one-line preview for a NESTED repeating row (box-12 / charitable) at `parent_addr + [i]`: the group's
/// first field rendered there, or empty when uninformative. Mirrors `row_preview` but at a deeper address.
fn nested_row_preview(
    section: &'static Section,
    ri: &ReturnInputs,
    parent_addr: &RowAddr,
    i: usize,
) -> String {
    let mut addr = parent_addr.clone();
    addr.0.push(i);
    let Some(f) = section.fields.first() else {
        return String::new();
    };
    match (f.get)(ri, &addr) {
        Some(v) => {
            let s = render_field_value(&v);
            if s.is_empty() || s == "—" {
                String::new()
            } else {
                format!("  — {s}")
            }
        }
        None => String::new(),
    }
}

/// Push the live `Field`s of `section` (read at `addr`) as `label  [value]`, the focused field highlighted.
/// Shared by the singleton pane and the inside-a-row pane so per-row editing renders identically.
fn push_field_lines(
    lines: &mut Vec<Line<'static>>,
    section: &'static Section,
    ri: &ReturnInputs,
    addr: &RowAddr,
    field_focus: usize,
    editing: bool,
    buf: &str,
) {
    let focus_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let dark = Style::default().fg(Color::DarkGray);
    let fields = live_fields(section, ri);
    if fields.is_empty() {
        lines.push(Line::from(Span::styled("  (no live fields)", dark)));
    }
    for (i, f) in fields.iter().enumerate() {
        let is_focus = i == field_focus;
        // While editing the focused field, show the raw buffer being typed with a cursor block — NOT the
        // committed value (which only updates on a successful parse+apply). ★ A Secret field is NO-ECHO:
        // one bullet per typed char (mirrors `draw_unlock_screen`), NEVER the buffer content — digits must
        // not reach the screen during entry.
        let value = if is_focus && editing {
            if matches!(f.kind, FieldKind::Secret) {
                "\u{25cf}".repeat(buf.chars().count())
            } else {
                format!("{buf}\u{2588}")
            }
        } else {
            (f.get)(ri, addr)
                .map(|v| render_field_value(&v))
                .unwrap_or_else(|| "—".to_string())
        };
        let style = if is_focus {
            focus_style
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("  {}  [{}]", f.label, value),
            style,
        )));
    }
}

/// A one-line preview for a repeating row: the section's first field rendered at `[i]` (e.g. a W-2's owner,
/// a dependent's name), or empty when it has no informative value. Depth-1 groups only (the nested
/// box-12/charitable groups are never top-level rows).
fn row_preview(section: &'static Section, ri: &ReturnInputs, i: usize) -> String {
    let addr = RowAddr(vec![i]);
    let Some(f) = section.fields.first() else {
        return String::new();
    };
    match (f.get)(ri, &addr) {
        Some(v) => {
            let s = render_field_value(&v);
            if s.is_empty() || s == "—" {
                String::new()
            } else {
                format!("  — {}", s)
            }
        }
        None => String::new(),
    }
}

/// Append the flow's inline error (parse/apply/store failure) under the field pane, in red.
fn push_error(lines: &mut Vec<Line<'static>>, error: Option<&str>) {
    if let Some(err) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  error: {err}"),
            Style::default().fg(Color::Red),
        )));
    }
}

/// The per-section status glyph: `!` when a screen refusal (from the last `commit`) is attributed to this
/// section (I-4/§9A — takes precedence, it is the one thing the filer must fix), else `✓` when every live
/// field is answered, `…` otherwise. An absent optional-singleton is `…`; a repeating group is `✓` (its
/// rows are optional here).
fn section_glyph(
    section: &'static Section,
    ri: &ReturnInputs,
    refused_section: Option<btctax_input_form::SectionId>,
) -> char {
    // ★ I-4: a refusal attributed to this section wins over completeness — the filer must resolve it first.
    if refused_section == Some(section.id) {
        return '!';
    }
    if let SectionKind::OptionalSingleton { present, .. } = section.kind {
        if !present(ri) {
            return '…';
        }
    }
    if let SectionKind::Repeating { .. } = section.kind {
        return '✓';
    }
    let fields = live_fields(section, ri);
    let complete = !fields.is_empty()
        && fields.iter().all(|f| {
            (f.get)(ri, &RowAddr::default())
                .map(|v| value_is_answered(&v))
                .unwrap_or(false)
        });
    if complete {
        '✓'
    } else {
        '…'
    }
}

/// Whether a field value counts as "answered" for the section-completeness glyph.
fn value_is_answered(v: &FieldValue) -> bool {
    match v {
        FieldValue::Money(m) => !m.is_zero(),
        FieldValue::Text(s) => !s.is_empty(),
        FieldValue::Bool(b) => *b,
        FieldValue::TriState(t) => t.is_some(),
        FieldValue::Date(d) => d.is_some(),
        FieldValue::Choice(_) => true,
        FieldValue::Secret(sv) => matches!(sv, SecretView::Set { .. }),
        // `get` never returns `SecretEntry` (inbound-only); treat defensively as answered.
        FieldValue::SecretEntry(_) => true,
    }
}

/// Render a field value for display, per kind. A `Secret` shows its `SecretView` (masked or
/// `(unset)`) — NEVER raw digits.
fn render_field_value(v: &FieldValue) -> String {
    match v {
        FieldValue::Money(m) => format!("${m}"),
        FieldValue::Text(s) => s.clone(),
        FieldValue::Bool(b) => if *b { "[x]" } else { "[ ]" }.to_string(),
        FieldValue::TriState(t) => match t {
            Some(true) => "yes",
            Some(false) => "no",
            None => "—",
        }
        .to_string(),
        FieldValue::Date(d) => match d {
            Some(d) => d.to_string(),
            None => "—".to_string(),
        },
        FieldValue::Choice(c) => c.clone(),
        FieldValue::Secret(sv) => match sv {
            SecretView::Empty => "(unset)".to_string(),
            SecretView::Set { masked } => masked.clone(),
        },
        // Unreachable via `get` (SecretEntry is inbound-only); never echo it.
        FieldValue::SecretEntry(_) => "(unset)".to_string(),
    }
}

/// The KEYMAP overlay's own lines. Extracted from `draw_help_overlay` so the sync guard
/// (`tests::kat_keymap_overlay_lists_every_browse_char_binding`) can read the SAME text the filer sees —
/// ★ P-C gate arch I-3: Browse `w` (the Defensive Filing Wizard's ONLY entry point) shipped without an
/// overlay entry despite the "KEEP IN SYNC with KEYMAP overlay" contract in `main.rs`, making the whole
/// feature undiscoverable in-product. Any NEW `KeyCode::Char(_)` arm in the Browse match must gain a
/// line here, or that test reds.
fn help_overlay_lines() -> Vec<Line<'static>> {
    let hdr = |s: &'static str| {
        Line::from(Span::styled(
            s,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
    };
    vec![
        hdr("Navigation"),
        Line::from("  Tab/Shift-Tab switch tab    ←/→ or h/l column cursor    s sort column"),
        Line::from("  [ / ] change year    j/k or ↑/↓ scroll    PgUp/PgDn page    g/G top/bottom"),
        Line::from(""),
        hdr("Reconcile"),
        Line::from("  c classify-inbound   o reclassify-outflow   r reclassify-income"),
        Line::from("  f set-fmv   v void   S select-lots   d donation-details"),
        Line::from("  L link-transfer   u classify-raw   m match-self-transfers"),
        Line::from("  i resolve-conflict   z optimize   e method-election"),
        Line::from("  a/A safe-harbor attest/allocate"),
        Line::from("  w defensive-filing wizard (declare / promote a no-records tranche)"),
        Line::from("  Bulk:  b link   B self-transfer-in   C resolve-conflict"),
        Line::from("         V void   I income   O reclassify-outflow"),
        Line::from("  P approve pseudo-reconcile defaults (when the [PSEUDO] banner shows)"),
        Line::from(""),
        hdr("App"),
        Line::from("  p profile   T tax-inputs   ? help   q/Esc close"),
        Line::from(""),
        Line::from(Span::styled(
            "  ? · Esc · q  to close",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    let lines = help_overlay_lines();
    let width: u16 = 72;
    let height: u16 = lines.len() as u16 + 2;
    let rect = centered_rect(width, height, area);
    frame.render_widget(Clear, rect);
    let p = Paragraph::new(lines).block(
        Block::default()
            .title(" Help — keyboard shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(p, rect);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

// ── Select-lots draw functions ────────────────────────────────────────────────

/// Render the select-lots disposal list overlay.
///
/// Title: `" Select Lots — select disposal event "`.
/// Columns: `Date | Kind | Principal Sat | Wallet | EventId`.
fn draw_select_lots_list(frame: &mut Frame, area: Rect, flow: &mut SelectLotsFlowState) {
    let modal_rect = centered_rect(90, 22, area);
    frame.render_widget(Clear, modal_rect);

    let header_cells = ["Date", "Kind", "Principal Sat", "Wallet", "EventId"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let kind_str = |k: DisposalKind| match k {
        DisposalKind::Sell => "sell",
        DisposalKind::Spend => "spend",
        DisposalKind::Gift => "gift",
        DisposalKind::Donate => "donate",
        DisposalKind::SelfTransfer => "self-transfer",
    };

    let selected_idx = flow.list.table_state.selected();
    let items: Vec<Row> = flow
        .list
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = selected_idx == Some(i);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let wallet_str = match &item.wallet {
                Some(w) => format!("{w:?}"),
                None => "(no wallet)".to_string(),
            };
            Row::new(vec![
                Cell::from(item.date.to_string()).style(style),
                Cell::from(kind_str(item.kind)).style(style),
                Cell::from(item.principal_sat.to_string()).style(style),
                Cell::from(wallet_str).style(style),
                Cell::from(item.disposal_event.canonical()).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        items,
        [
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(14),
            Constraint::Length(20),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Select Lots — select disposal event "),
    );

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);
}

/// Render the select-lots LotsForm overlay.
///
/// Scrollable multi-row form with editable `pick_sat` fields.
/// Running-total footer: `"Picked: {Σ_pick_sat} / {principal_sat} sat"`.
fn draw_lots_form(frame: &mut Frame, area: Rect, step: &mut SelectLotsStep) {
    let SelectLotsStep::LotsForm {
        item,
        rows,
        cursor,
        error,
    } = step
    else {
        return;
    };

    let modal_rect = centered_rect(90, 24, area);
    frame.render_widget(Clear, modal_rect);

    // Header and rows.
    let principal_sat = item.principal_sat;
    let picked_sat: btctax_core::Sat = rows.iter().map(|r| r.pick_sat().unwrap_or(0)).sum();

    let inner = Block::default().borders(Borders::ALL).title(format!(
        " Select Lots — {event}  [Enter=submit  Esc=back] ",
        event = item.disposal_event.canonical()
    ));
    let inner_area = inner.inner(modal_rect);
    frame.render_widget(inner, modal_rect);

    // Calculate content layout.
    let available_height = inner_area.height as usize;
    let footer_lines = 2usize; // 1 for totals, 1 for error/blank
    let visible_rows = available_height.saturating_sub(footer_lines + 1 /*header*/);

    // Scroll window.
    let start = (*cursor).saturating_sub(visible_rows.saturating_sub(1));
    let end = (start + visible_rows).min(rows.len());
    let scroll_rows = &rows[start..end];

    let header = Line::from(vec![Span::styled(
        format!(
            "{:<14} {:<32} {:>12} {:>12}  Pick Sat",
            "Acquired", "LotId", "Remaining", "Basis USD"
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )]);

    let row_lines: Vec<Line> = scroll_rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let abs_idx = start + idx;
            let is_cursor = abs_idx == *cursor;
            let pick_str = if row.pick_sat_buf.is_empty() {
                "0".to_string()
            } else {
                row.pick_sat_buf.buf.clone()
            };
            let lot_str = format!(
                "{}#{}",
                row.lot_id.origin_event_id.canonical(),
                row.lot_id.split_sequence
            );
            let line = format!(
                "{:<14} {:<32} {:>12} {:>12}  {}",
                row.acquired_at.to_string(),
                &lot_str[..lot_str.len().min(32)],
                row.remaining_sat,
                row.usd_basis,
                pick_str
            );
            if is_cursor {
                Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::Black).bg(Color::Cyan),
                ))
            } else {
                Line::from(line)
            }
        })
        .collect();

    let mut all_lines = vec![header];
    all_lines.extend(row_lines);

    // Footer: running total and error.
    let total_line = Line::from(format!("Picked: {picked_sat} / {principal_sat} sat"));
    all_lines.push(total_line);

    if let Some(err) = error {
        all_lines.push(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )));
    }

    let para = Paragraph::new(all_lines);
    frame.render_widget(para, inner_area);
}

/// Render the select-lots confirmation modal.
///
/// Shows disposal info + picks (up to 8; overflow line for the rest).
fn draw_select_lots_modal(frame: &mut Frame, area: Rect, modal: &SelectLotsModalState) {
    let kind_str = match modal.disposal_kind {
        DisposalKind::Sell => "sell",
        DisposalKind::Spend => "spend",
        DisposalKind::Gift => "gift",
        DisposalKind::Donate => "donate",
        DisposalKind::SelfTransfer => "self-transfer",
    };

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(format!(
            "  disposal: {}  ({})",
            modal.disposal_event.canonical(),
            kind_str
        )),
        Line::from(format!("  date:     {}", modal.disposal_date)),
        Line::from(format!("  principal: {} sat", modal.principal_sat)),
        Line::from(""),
        Line::from(format!(
            "  Picks: {} lot(s), {} sat total",
            modal.pick_count, modal.total_sat
        )),
    ];

    const MAX_PICKS_SHOWN: usize = 8;
    let show_count = modal.picks.len().min(MAX_PICKS_SHOWN);
    for pick in &modal.picks[..show_count] {
        lines.push(Line::from(format!(
            "    {}#{}   →  {} sat",
            pick.lot.origin_event_id.canonical(),
            pick.lot.split_sequence,
            pick.sat
        )));
    }
    if modal.picks.len() > MAX_PICKS_SHOWN {
        let remainder_count = modal.picks.len() - MAX_PICKS_SHOWN;
        let remainder_sat: btctax_core::Sat =
            modal.picks[MAX_PICKS_SHOWN..].iter().map(|p| p.sat).sum();
        lines.push(Line::from(format!(
            "    … and {remainder_count} more picks ({remainder_sat} sat in the remainder)"
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  Appended as a decision event (append-only log).",
    ));
    lines.push(Line::from(
        "  Saved immediately via the vault's atomic write path.",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  [Enter] Confirm & save     [Esc] Cancel — writes nothing",
    ));
    lines.push(Line::from(""));

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(70, height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm: select-lots — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Set-donation-details draw functions ──────────────────────────────────────

/// Render the set-donation-details list overlay.
///
/// Title: `" Set Donation Details — select Donation event "`.
/// Columns: `Date | Sat | Donee | Completeness | EventId`.
fn draw_donation_details_list(
    frame: &mut Frame,
    area: Rect,
    flow: &mut SetDonationDetailsFlowState,
) {
    let modal_rect = centered_rect(90, 20, area);
    frame.render_widget(Clear, modal_rect);

    let header_cells = ["Date", "Sat", "Donee", "Completeness", "EventId"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let selected_idx = flow.list.table_state.selected();
    let items: Vec<Row> = flow
        .list
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = selected_idx == Some(i);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(item.date.to_string()).style(style),
                Cell::from(item.total_sat.to_string()).style(style),
                Cell::from(item.donee.as_deref().unwrap_or("")).style(style),
                Cell::from(item.completeness_str()).style(style),
                Cell::from(item.event_id.canonical()).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        items,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Length(14),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Set Donation Details — select Donation event "),
    );

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);
}

/// Render the donation-details field form overlay.
///
/// 10-field form; focused field is highlighted.
fn draw_donation_details_form(frame: &mut Frame, area: Rect, step: &mut SetDonationDetailsStep) {
    let SetDonationDetailsStep::FieldForm {
        item,
        donee_name_buf,
        donee_address_buf,
        donee_ein_buf,
        appraiser_name_buf,
        appraiser_address_buf,
        appraiser_tin_buf,
        appraiser_ptin_buf,
        appraiser_qualifications_buf,
        appraisal_date_buf,
        fmv_method_override_buf,
        focus,
        error,
    } = step
    else {
        return;
    };

    let modal_rect = centered_rect(80, 20, area);
    frame.render_widget(Clear, modal_rect);

    let labels = DONATION_FIELD_LABELS;
    let bufs: [&FieldBuffer; 10] = [
        donee_name_buf,
        donee_address_buf,
        donee_ein_buf,
        appraiser_name_buf,
        appraiser_address_buf,
        appraiser_tin_buf,
        appraiser_ptin_buf,
        appraiser_qualifications_buf,
        appraisal_date_buf,
        fmv_method_override_buf,
    ];

    let mut lines: Vec<Line> = vec![Line::from(format!(
        "  event: {}  (Donation)",
        item.event_id.canonical()
    ))];

    for (i, (label, buf)) in labels.iter().zip(bufs.iter()).enumerate() {
        let val = if buf.is_empty() { "" } else { buf.buf.as_str() };
        let line_text = format!("  {:30} {}", label, val);
        if i == *focus {
            lines.push(Line::from(Span::styled(
                line_text,
                Style::default().fg(Color::Black).bg(Color::Cyan),
            )));
        } else {
            lines.push(Line::from(line_text));
        }
    }

    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    lines.push(Line::from(
        "  [Enter] Confirm → modal   [Esc] Back   [↑/↓] Move focus",
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Set Donation Details — field form ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the donation-details confirmation modal.
///
/// Shows only non-None fields + "last-write-wins; not a decision event" note.
fn draw_donation_details_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &SetDonationDetailsModalState,
) {
    let d = &modal.details;
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(format!(
            "  event:  {}  (Donation)",
            modal.event_id.canonical()
        )),
        Line::from(format!("  date:   {}", modal.event_date)),
        Line::from(format!("  sat:    {}", modal.total_sat)),
        Line::from(""),
        Line::from(format!("  donee_name:           {}", d.donee_name)),
    ];
    if let Some(v) = &d.donee_address {
        lines.push(Line::from(format!("  donee_address:        {v}")));
    }
    if let Some(v) = &d.donee_ein {
        lines.push(Line::from(format!("  donee_ein:            {v}")));
    }
    lines.push(Line::from(format!(
        "  appraiser_name:       {}",
        d.appraiser_name
    )));
    if let Some(v) = &d.appraiser_address {
        lines.push(Line::from(format!("  appraiser_address:    {v}")));
    }
    if let Some(v) = &d.appraiser_tin {
        lines.push(Line::from(format!("  appraiser_tin:        {v}")));
    }
    if let Some(v) = &d.appraiser_ptin {
        lines.push(Line::from(format!("  appraiser_ptin:       {v}")));
    }
    if let Some(v) = &d.appraiser_qualifications {
        lines.push(Line::from(format!("  appraiser_qualifications: {v}")));
    }
    if let Some(v) = &d.appraisal_date {
        lines.push(Line::from(format!("  appraisal_date:       {v}")));
    }
    if let Some(v) = &d.fmv_method_override {
        lines.push(Line::from(format!("  fmv_method_override:  {v}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  Stored in side-table (last-write-wins; not a decision event).",
    ));
    lines.push(Line::from("  Saved via vault's atomic write path."));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  [Enter] Confirm & save     [Esc] Cancel — writes nothing",
    ));
    lines.push(Line::from(""));

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(70, height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm: set-donation-details — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Link-transfer draw functions (chunk 4a, D1) ──────────────────────────────

/// Render the link-transfer step-1 (out-list) overlay.
///
/// Title: `" Link Transfer — select the outgoing transfer "`.
/// Columns: `Date | Principal Sat | Wallet | EventId`.
fn draw_link_transfer_out_list(frame: &mut Frame, area: Rect, flow: &mut LinkTransferFlowState) {
    let modal_width: u16 = 90;
    let modal_height: u16 =
        (flow.out_list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Link Transfer — select the outgoing transfer  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Principal Sat"),
        Cell::from("Wallet"),
        Cell::from("EventId"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = flow
        .out_list
        .items
        .iter()
        .map(|item| {
            let wallet_str = match &item.wallet {
                Some(w) => wallet_label(w),
                None => "(no wallet)".to_string(),
            };
            Row::new(vec![
                Cell::from(item.date.to_string()),
                Cell::from(item.principal_sat.to_string()),
                Cell::from(wallet_str),
                Cell::from(item.transfer_out_event.canonical()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(14),
        Constraint::Length(18),
        Constraint::Min(28),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.out_list.table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the link-transfer step-2 (target pick) overlay: a mode toggle (Tab cycles
/// InEvent ⇄ Wallet) over either the in-event list or the wallet list.
fn draw_link_transfer_target_pick(frame: &mut Frame, area: Rect, flow: &mut LinkTransferFlowState) {
    let mode = match &flow.step {
        LinkTransferStep::TargetPick { mode, .. } => *mode,
        _ => return,
    };
    let out_canonical = match &flow.step {
        LinkTransferStep::TargetPick { out, .. } => out.transfer_out_event.canonical(),
        _ => String::new(),
    };

    let modal_width: u16 = 92;
    let modal_height: u16 = 22.min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let (mode_tag, header, rows): (&str, Row, Vec<Row>) = match mode {
        LinkMode::InEvent => (
            "InEvent (link to a TransferIn)",
            Row::new(vec![
                Cell::from("Date"),
                Cell::from("Sat"),
                Cell::from("Wallet"),
                Cell::from("EventId"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            if flow.in_list.items.is_empty() {
                vec![Row::new(vec![Cell::from(
                    "(no linkable TransferIn events — Tab to Wallet mode)",
                )])]
            } else {
                flow.in_list
                    .items
                    .iter()
                    .map(|item| {
                        Row::new(vec![
                            Cell::from(item.date.to_string()),
                            Cell::from(item.sat.to_string()),
                            Cell::from(wallet_label(&item.wallet)),
                            Cell::from(item.in_event.canonical()),
                        ])
                    })
                    .collect()
            },
        ),
        LinkMode::Wallet => (
            "Wallet (link to a known wallet)",
            Row::new(vec![Cell::from("Wallet")])
                .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            if flow.wallet_list.items.is_empty() {
                vec![Row::new(vec![Cell::from("(no known wallets)")])]
            } else {
                flow.wallet_list
                    .items
                    .iter()
                    .map(|item| Row::new(vec![Cell::from(wallet_label(&item.wallet))]))
                    .collect()
            },
        ),
    };

    let title = format!(" Link Transfer — {out_canonical} → pick a target  [EDITOR] ");
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let widths = match mode {
        LinkMode::InEvent => vec![
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Min(24),
        ],
        LinkMode::Wallet => vec![Constraint::Min(40)],
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    let table_state = match mode {
        LinkMode::InEvent => &mut flow.in_list.table_state,
        LinkMode::Wallet => &mut flow.wallet_list.table_state,
    };
    frame.render_stateful_widget(table, modal_rect, table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new(format!(
        "mode: {mode_tag}   Tab: switch mode   ↑/↓: scroll   Enter: confirm   Esc: back"
    ))
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the link-transfer confirmation modal (non-taxable relocation framing, TP8-c).
fn draw_link_transfer_modal(frame: &mut Frame, area: Rect, modal: &LinkTransferModalState) {
    let content = format!(
        "  out:    {out}  ({sat} sat, {date})\n\
         \n\
           →link:  {target}\n\
         \n\
           Records a NON-TAXABLE self-transfer (relocation).\n\
           Basis carries; any fee is non-taxable (TP8-c).\n\
           Appended as a revocable decision (void with 'v').\n\
         \n\
         [Enter] Confirm & save   [Esc] Cancel — writes nothing",
        out = modal.out_event.canonical(),
        sat = modal.out_sat,
        date = modal.out_date,
        target = modal.target_label,
    );

    let modal_width: u16 = 74;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(12);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: link-transfer — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Classify-raw draw functions (chunk 4a, D2) ───────────────────────────────

/// Render the classify-raw list overlay.
///
/// Title: `" Classify Raw — select an unclassified import "`.
/// Columns: `Date | Raw-text (elided) | Wallet | EventId`.
fn draw_classify_raw_list(frame: &mut Frame, area: Rect, flow: &mut ClassifyRawFlowState) {
    let modal_width: u16 = 96;
    let modal_height: u16 = (flow.list.items.len() as u16 + 6).min(area.height.saturating_sub(2));
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Classify Raw — select an unclassified import  [EDITOR] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Raw"),
        Cell::from("Wallet"),
        Cell::from("EventId"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED));

    let rows: Vec<Row> = flow
        .list
        .items
        .iter()
        .map(|item| {
            let raw_elided: String = item.raw.chars().take(40).collect();
            let wallet_str = match &item.wallet {
                Some(w) => wallet_label(w),
                None => "(no wallet)".to_string(),
            };
            Row::new(vec![
                Cell::from(item.date.to_string()),
                Cell::from(raw_elided),
                Cell::from(wallet_str),
                Cell::from(item.target.canonical()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Length(40),
        Constraint::Length(16),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);

    let footer_area = Rect {
        x: modal_rect.x,
        y: modal_rect.y + modal_rect.height.saturating_sub(1),
        width: modal_rect.width,
        height: 1,
    };
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, footer_area);
}

/// Render the classify-raw variant picker or per-variant sub-form overlay.
fn draw_classify_raw_form(frame: &mut Frame, area: Rect, step: &ClassifyRawStep) {
    let modal_width: u16 = 76;
    let modal_height: u16 = 18;
    let modal_rect = centered_rect(modal_width, modal_height, area);
    frame.render_widget(Clear, modal_rect);

    let (title, content) = match step {
        ClassifyRawStep::VariantPicker { item, variant } => {
            let row = |tag: &str, v: ClassifyRawVariant| {
                if *variant == v {
                    format!("> {tag}")
                } else {
                    format!("  {tag}")
                }
            };
            let c = format!(
                "  target: {target}\n  raw:    {raw}\n\
                 \n\
                   Select variant (Tab to cycle, Enter to confirm):\n\
                 \n\
                 {acq}   {inc}\n\
                 \n\
                 \n  Esc: back to list",
                target = item.target.canonical(),
                raw = item.raw.chars().take(48).collect::<String>(),
                acq = row("Acquire", ClassifyRawVariant::Acquire),
                inc = row("Income", ClassifyRawVariant::Income),
            );
            (" Classify Raw — variant picker  [EDITOR] ", c)
        }
        ClassifyRawStep::AcquireForm {
            item,
            sat_buf,
            usd_cost_buf,
            fee_buf,
            basis_source,
            focus,
            error,
        } => {
            let cur = |f: usize| if *focus == f { ">" } else { " " };
            let err_line = error
                .as_deref()
                .map(|e| format!("\n  Error: {e}"))
                .unwrap_or_default();
            let c = format!(
                "  target: {target}\n\
                 \n\
                 \n  {c0} sat: {sat}\
                 \n  {c1} usd_cost (USD): {uc}\
                 \n  {c2} fee_usd (USD, optional): {fee}\
                 \n  {c3} basis_source: [{bs}]  (Tab: cycle)\
                 {err}\n\
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move",
                target = item.target.canonical(),
                c0 = cur(0),
                sat = sat_buf.buf,
                c1 = cur(1),
                uc = usd_cost_buf.buf,
                c2 = cur(2),
                fee = fee_buf.buf,
                c3 = cur(3),
                bs = basis_source_display(*basis_source),
                err = err_line,
            );
            (" Classify Raw — Acquire  [EDITOR] ", c)
        }
        ClassifyRawStep::IncomeForm {
            item,
            sat_buf,
            fmv_buf,
            kind,
            business,
            focus,
            error,
        } => {
            let cur = |f: usize| if *focus == f { ">" } else { " " };
            let err_line = error
                .as_deref()
                .map(|e| format!("\n  Error: {e}"))
                .unwrap_or_default();
            let c = format!(
                "  target: {target}\n\
                 \n\
                 \n  {c0} sat: {sat}\
                 \n  {c1} usd_fmv (USD, optional → Missing): {fmv}\
                 \n  {c2} kind: [{kind}]  (Tab: cycle)\
                 \n  {c3} business: {business}  (Space: toggle)\
                 {err}\n\
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move",
                target = item.target.canonical(),
                c0 = cur(0),
                sat = sat_buf.buf,
                c1 = cur(1),
                fmv = fmv_buf.buf,
                c2 = cur(2),
                kind = income_kind_display(*kind),
                c3 = cur(3),
                business = business,
                err = err_line,
            );
            (" Classify Raw — Income  [EDITOR] ", c)
        }
        ClassifyRawStep::List => ("", String::new()),
    };

    if title.is_empty() {
        return; // defensive — List is rendered by draw_classify_raw_list.
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

/// Render the classify-raw confirmation modal (target + raw + the built imported payload).
fn draw_classify_raw_modal(frame: &mut Frame, area: Rect, modal: &ClassifyRawModalState) {
    let built_section = match &modal.built {
        btctax_core::EventPayload::Acquire(a) => format!(
            "  as: Acquire\n    sat:          {sat}\n    usd_cost:     {usd}\n    \
             fee_usd:      {fee}\n    basis_source: {bs}",
            sat = a.sat,
            usd = a.usd_cost,
            fee = a.fee_usd,
            bs = basis_source_display(a.basis_source),
        ),
        btctax_core::EventPayload::Income(i) => {
            let fmv_str = i
                .usd_fmv
                .map(|v| v.to_string())
                .unwrap_or_else(|| "(none → Missing)".to_string());
            format!(
                "  as: Income\n    sat:        {sat}\n    usd_fmv:    {fmv}\n    \
                 fmv_status: {status:?}\n    kind:       {kind}\n    business:   {business}",
                sat = i.sat,
                fmv = fmv_str,
                status = i.fmv_status,
                kind = income_kind_display(i.kind),
                business = i.business,
            )
        }
        _ => "  as: (unsupported)".to_string(),
    };

    let raw_elided: String = modal.raw.chars().take(52).collect();
    let content = format!(
        "  target: {target}  (Unclassified)\n\
           raw:    {raw}\n\
         \n\
         {built_section}\n\
         \n\
           Appended as a revocable decision (void with 'v').\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save     [Esc] Cancel — writes nothing",
        target = modal.target.canonical(),
        raw = raw_elided,
    );

    let modal_width: u16 = 72;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(12);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Confirm: classify-raw — WRITES THE VAULT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Safe-harbor-attest flow draw functions ─────────────────────────────────────

/// Render the attest-flow Info step (spec D3 mockup): allocation details, the
/// §5.02(4) time-bar STATUS, the §7.4 IRREVOCABLE warning, and the post-attest
/// void-is-permanent warning (the doomed void is itself append-only).
///
/// `[Enter]` → advance to TypedWord step.  `[Esc]` → cancel (closes flow).
fn draw_attest_info(frame: &mut Frame, area: Rect, flow: &SafeHarborAttestFlowState) {
    let modal_width: u16 = 72;
    let modal_height: u16 = 24;
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let alloc = &flow.prior_alloc;
    let lots_count = alloc.lots.len();
    let total_sat: btctax_core::Sat = alloc.lots.iter().map(|l| l.sat).sum();
    let method = format!("{:?}", alloc.method);
    let pre2025 = format!("{:?}", alloc.pre2025_method);
    let prior_id_str = flow.prior_id.canonical();

    let warn = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let lines: Vec<Line> = vec![
        Line::from(format!("  Allocation: {prior_id_str}")),
        Line::from(format!(
            "  As-of date: {}  (§5.02(4) universal snapshot)",
            alloc.as_of_date
        )),
        Line::from(format!("  Method:     {method}")),
        Line::from(format!("  Pre-2025 method: {pre2025}")),
        Line::from(format!("  Lots:       {lots_count}  ({total_sat} sat)")),
        Line::from("  Attested:   false  ←  time-bar active (§5.02(4))"),
        Line::from(""),
        Line::from("  STATUS: this allocation is inert due to the §5.02(4)"),
        Line::from("  time-bar. Attestation CURES the time-bar and makes the"),
        Line::from("  allocation EFFECTIVE and IRREVOCABLE (§7.4)."),
        Line::from(""),
        Line::from(Span::styled("  !! IRREVOCABLE WARNING:", warn)),
        Line::from(Span::styled(
            "  Once attested, this allocation CANNOT be voided — any",
            warn,
        )),
        Line::from(Span::styled(
            "  void attempt fires a PERMANENT Hard DecisionConflict",
            warn,
        )),
        Line::from(Span::styled(
            "  that gates tax computation (§7.4): the doomed void is",
            warn,
        )),
        Line::from(Span::styled(
            "  itself append-only and cannot be undone. Do NOT attest",
            warn,
        )),
        Line::from(Span::styled(
            "  unless the lot list and method match your filed return.",
            warn,
        )),
        Line::from(""),
        Line::from("  The operation voids the current allocation and re-"),
        Line::from("  appends it as attested (TWO decision events written)."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Proceed to confirmation   [Esc] Cancel",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Safe-Harbor Attestation — IRREVOCABLE ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the attest-flow TypedWord step: user must type "ATTEST" to confirm.
///
/// Buffer is preserved on wrong word (error is shown) [R0-I7].
/// `[Esc]` → back to the Info step (one step per press [I4]).
fn draw_attest_typed_word(frame: &mut Frame, area: Rect, step: &SafeHarborAttestStep) {
    let (buf_str, error) = match step {
        SafeHarborAttestStep::TypedWord { buf, error } => (buf.buf.as_str(), error.as_deref()),
        _ => return,
    };

    let modal_width: u16 = 64;
    let modal_height: u16 = 13;
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from("  Type exactly:  ATTEST"),
        Line::from(format!("  Your input:    {buf_str}_")),
        Line::from(""),
        Line::from(Span::styled(
            "  This attestation is permanent. The allocation becomes",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  immediately irrevocable upon save.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
    ];

    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  {err}"),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  [Enter] Submit (if \"ATTEST\" typed)  [Esc] Cancel",
        Style::default().fg(Color::Cyan),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" IRREVOCABLE: type ATTEST to confirm — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Resolve-conflict flow draw functions (chunk 4b, D3) ──────────────────────

/// Render the resolve-conflict step-1 list overlay.
///
/// Title: `" Resolve Import Conflict — select a conflict "`.
/// Columns: `Date | Target | New-fingerprint | conflict EventId`.
fn draw_resolve_conflict_list(frame: &mut Frame, area: Rect, flow: &mut ResolveConflictFlowState) {
    let modal_rect = centered_rect(96, 20, area);
    frame.render_widget(Clear, modal_rect);

    let header_cells = ["Date", "Target", "New-fingerprint", "Conflict EventId"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let selected_idx = flow.list.table_state.selected();
    let items: Vec<Row> = flow
        .list
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = selected_idx == Some(i);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(item.date.to_string()).style(style),
                Cell::from(item.target.canonical()).style(style),
                Cell::from(item.new_fingerprint.clone()).style(style),
                Cell::from(item.conflict_event.canonical()).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        items,
        [
            Constraint::Length(12),
            Constraint::Length(26),
            Constraint::Length(12),
            Constraint::Min(24),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Resolve Import Conflict — select a conflict "),
    );

    frame.render_stateful_widget(table, modal_rect, &mut flow.list.table_state);
}

/// Render the resolve-conflict step-2 accept/reject choice overlay (an in-flow toggle).
fn draw_resolve_conflict_choose(frame: &mut Frame, area: Rect, step: &ResolveConflictStep) {
    let ResolveConflictStep::Choose { conflict, kind } = step else {
        return;
    };

    let (accept_span, reject_span) = match kind {
        ResolveKind::Accept => (
            Span::styled(
                " ACCEPT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  reject  "),
        ),
        ResolveKind::Reject => (
            Span::raw("  accept  "),
            Span::styled(
                " REJECT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ),
    };

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(format!(
            "  conflict: {}",
            conflict.conflict_event.canonical()
        )),
        Line::from(format!("  target:   {}", conflict.target.canonical())),
        Line::from(""),
        Line::from(format!("  current:  {}", conflict.current_summary)),
        Line::from(format!("  →new:     {}", conflict.new_summary)),
        Line::from(""),
        Line::from(vec![Span::raw("  choose:  "), accept_span, reject_span]),
        Line::from(""),
        Line::from(Span::styled(
            "  ←/→ (h/l): toggle   Enter: confirm → modal   Esc: back",
            Style::default().fg(Color::Cyan),
        )),
    ];

    let modal_rect = centered_rect(80, (lines.len() + 2) as u16, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Resolve Import Conflict — accept or reject ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the resolve-conflict confirmation modal — BOTH sides + the NON-REVOCABLE warning.
fn draw_resolve_conflict_modal(frame: &mut Frame, area: Rect, modal: &ResolveConflictModalState) {
    let (title, adopt_note) = match modal.kind {
        ResolveKind::Accept => (
            " Confirm: ACCEPT conflict — WRITES THE VAULT ",
            "(ACCEPT adopts new)",
        ),
        ResolveKind::Reject => (
            " Confirm: REJECT conflict — WRITES THE VAULT ",
            "(REJECT keeps current, discards new)",
        ),
    };

    let content = format!(
        "  conflict: {conflict}\n\
           target:   {target}\n\
         \n\
           current:  {current}\n\
           →new:     {new}   {adopt_note}\n\
         \n\
           !! This decision CANNOT be voided (non-revocable).\n\
           Appended as a decision event (append-only log).\n\
           Saved immediately via the vault's atomic write path.\n\
         \n\
         [Enter] Confirm & save   [Esc] Cancel — writes nothing",
        conflict = modal.conflict_event.canonical(),
        target = modal.target.canonical(),
        current = modal.old_summary,
        new = modal.new_summary,
    );

    let modal_width: u16 = 74;
    let content_lines = content.lines().count() as u16 + 2;
    let modal_height = content_lines.max(13);
    let modal_rect = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, modal_rect);
}

// ── Optimize-accept flow draw functions (chunk 4b, D4) ───────────────────────

/// Short display label for a `Persistability` (list column).
fn persistability_label(p: Persistability) -> &'static str {
    match p {
        Persistability::ContemporaneousNow => "contemporaneous",
        Persistability::NeedsAttestation => "needs-attestation",
        Persistability::ForbiddenBroker2027 => "forbidden",
    }
}

/// Render the optimize-accept step-1 list overlay + the flow-level year-Δtax banner.
///
/// Banner: whole-year `delta` (≤ 0) + APPROXIMATE caveat. Table: Date | Wallet | Persistability |
/// disposal EventId — NO per-disposal Δtax column [R0-I1].
fn draw_optimize_accept_list(frame: &mut Frame, area: Rect, flow: &mut OptimizeAcceptFlowState) {
    let modal_rect = centered_rect(96, 22, area);
    frame.render_widget(Clear, modal_rect);

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" Optimize — accept a proposed lot selection ");
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    // ── Banner (year-level Δtax; approximate caveat) ─────────────────────────
    let mut banner_lines = vec![Line::from(Span::styled(
        format!(
            "Expected year Δtax if the FULL proposal is accepted: {} (≤ 0)",
            flow.delta
        ),
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    if flow.approximate {
        banner_lines.push(Line::from(Span::styled(
            "APPROXIMATE — not a guaranteed global minimum",
            Style::default().fg(Color::Yellow),
        )));
    }
    banner_lines.push(Line::from(Span::styled(
        "(per-row dollar figures are not shown — the delta is a whole-year figure)",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(Paragraph::new(banner_lines), chunks[0]);

    // ── Table ─────────────────────────────────────────────────────────────────
    let header_cells = ["Date", "Wallet", "Persistability", "Disposal EventId"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let selected_idx = flow.list.table_state.selected();
    let items: Vec<Row> = flow
        .list
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = selected_idx == Some(i);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(item.date.to_string()).style(style),
                Cell::from(wallet_label(&item.wallet)).style(style),
                Cell::from(persistability_label(item.persistable)).style(style),
                Cell::from(item.disposal.canonical()).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        items,
        [
            Constraint::Length(12),
            Constraint::Length(22),
            Constraint::Length(18),
            Constraint::Min(20),
        ],
    )
    .header(header);

    frame.render_stateful_widget(table, chunks[1], &mut flow.list.table_state);
}

/// Render the optimize-accept attestation-text step (NeedsAttestation only).
fn draw_optimize_accept_attest_text(frame: &mut Frame, area: Rect, step: &OptimizeAcceptStep) {
    let OptimizeAcceptStep::AttestText { item, buf, error } = step else {
        return;
    };

    let modal_rect = centered_rect(76, 16, area);
    frame.render_widget(Clear, modal_rect);

    let mut lines: Vec<Line> = vec![
        Line::from(format!("  disposal: {}", item.disposal.canonical())),
        Line::from(format!(
            "  picks:    {} lot(s) (already executed — attestation required)",
            item.picks.len()
        )),
        Line::from(""),
        Line::from("  Type your contemporaneous-ID statement (the --attest value):"),
        Line::from(format!("  > {}_", buf.buf)),
        Line::from(""),
        Line::from(Span::styled(
            "  This narrow attestation records that the selection was identified",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  contemporaneously; it is co-persisted with the LotSelection.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
    ];
    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  {err}"),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "  [Enter] Continue (non-empty)   [Esc] Back",
        Style::default().fg(Color::Cyan),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Optimize — attestation (contemporaneous ID) ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the optimize-accept confirmation modal — proposed picks (elide past 8) + basis label +
/// (attested) the attestation text and the co-persist note. NO per-disposal Δtax [R0-I1].
fn draw_optimize_accept_modal(frame: &mut Frame, area: Rect, modal: &OptimizeAcceptModalState) {
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(format!("  disposal: {}", modal.disposal.canonical())),
        Line::from(format!(
            "  Picks: {} lot(s), {} sat total",
            modal.pick_count, modal.total_sat
        )),
        Line::from(format!("  basis: {}", modal.basis_label)),
        Line::from(""),
    ];

    const MAX_PICKS_SHOWN: usize = 8;
    let show_count = modal.picks.len().min(MAX_PICKS_SHOWN);
    for pick in &modal.picks[..show_count] {
        lines.push(Line::from(format!(
            "    {}#{}   →  {} sat",
            pick.lot.origin_event_id.canonical(),
            pick.lot.split_sequence,
            pick.sat
        )));
    }
    if modal.picks.len() > MAX_PICKS_SHOWN {
        let remainder_count = modal.picks.len() - MAX_PICKS_SHOWN;
        let remainder_sat: btctax_core::Sat =
            modal.picks[MAX_PICKS_SHOWN..].iter().map(|p| p.sat).sum();
        lines.push(Line::from(format!(
            "    … and {remainder_count} more picks ({remainder_sat} sat in the remainder)"
        )));
    }
    lines.push(Line::from(""));

    if let Some(att) = modal.attestation.as_deref() {
        lines.push(Line::from(Span::styled(
            format!("  attestation: {att}"),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(
            "  an attestation row is written alongside the LotSelection;",
        ));
        lines.push(Line::from("  voiding the LotSelection clears it."));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(
        "  Appended as a revocable decision (void with 'v').",
    ));
    lines.push(Line::from(
        "  Saved immediately via the vault's atomic write path.",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  [Enter] Confirm & save     [Esc] Cancel — writes nothing",
    ));
    lines.push(Line::from(""));

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(74, height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: optimize-accept — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Exact BTC string from a satoshi count (8 dp, no float). `100_000_000 sat = 1.00000000 BTC`.
fn fmt_btc(sat: btctax_core::Sat) -> String {
    format!("{}.{:08}", sat / 100_000_000, (sat % 100_000_000).abs())
}

/// Render the safe-harbor-allocate Preview (chunk 5, D2). Header + live method toggle + recorded
/// pre-2025 method + scrollable residue-lot table + totals footer + revocable framing/hint.
fn draw_safe_harbor_allocate_preview(
    frame: &mut Frame,
    area: Rect,
    flow: &mut SafeHarborAllocateFlowState,
) {
    let modal_rect = centered_rect(96, 24, area);
    frame.render_widget(Clear, modal_rect);

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" SAFE-HARBOR ALLOCATE — pre-2025 Universal residue snapshot @ 2025-01-01 ");
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(inner);

    // ── Banner: method (live toggle) + recorded pre-2025 method ───────────────
    // The NOTE is split into SHORT lines (≤ ~62 cols) so it renders in full inside the ≤96-col modal on an
    // 80-col terminal — a single long line was clipped mid-word and the actionable sentence never showed
    // (review r1 Important-1). `.wrap()` is belt-and-suspenders for narrower terminals.
    let note_style = Style::default().fg(Color::DarkGray);
    let banner_lines = vec![
        Line::from(vec![
            Span::raw("Method (Tab to change): "),
            Span::styled(
                format!("{:?}", flow.method),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("pre-2025 method (recorded): {:?}", flow.pre2025_method),
            note_style,
        )),
        Line::from(Span::styled(
            "NOTE: ProRata is NOT auto-computed here (both methods attest",
            note_style,
        )),
        Line::from(Span::styled(
            "the SAME per-wallet lots; the tag sets only the timebar rule).",
            note_style,
        )),
        Line::from(Span::styled(
            "For a true pro-rata global split, compute + attest it yourself.",
            note_style,
        )),
    ];
    frame.render_widget(
        Paragraph::new(banner_lines).wrap(Wrap { trim: false }),
        chunks[0],
    );

    // ── Residue-lot table ─────────────────────────────────────────────────────
    let header_cells = [
        "Wallet",
        "BTC",
        "usd_basis",
        "acquired_at",
        "loss_basis",
        "donor_date",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let selected_idx = flow.list.table_state.selected();
    let rows: Vec<Row> = flow
        .list
        .items
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if selected_idx == Some(i) {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let loss = r
                .dual_loss_basis
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "—".to_string());
            let donor = r
                .donor_acquired_at
                .map(|d| d.to_string())
                .unwrap_or_else(|| "—".to_string());
            Row::new(vec![
                Cell::from(wallet_label(&r.wallet)).style(style),
                Cell::from(fmt_btc(r.sat)).style(style),
                Cell::from(format!("{:.2}", r.usd_basis)).style(style),
                Cell::from(r.acquired_at.to_string()).style(style),
                Cell::from(loss).style(style),
                Cell::from(donor).style(style),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(22),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(10),
        ],
    )
    .header(header);
    frame.render_stateful_widget(table, chunks[1], &mut flow.list.table_state);

    // ── Totals footer + framing + hint ────────────────────────────────────────
    let footer_lines = vec![
        Line::from(Span::styled(
            format!(
                "{} lot(s) · Σ {} BTC · Σ basis ${:.2}",
                flow.lots.len(),
                fmt_btc(flow.total_sat),
                flow.total_basis
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Creates a REVOCABLE allocation (unattested). TIMEBARRED until you attest with 'a'. \
             Void with 'v' while inert.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "Tab: method  ↑/↓: scroll  Enter: confirm  Esc: cancel",
            Style::default().fg(Color::Cyan),
        )),
    ];
    frame.render_widget(Paragraph::new(footer_lines), chunks[2]);
}

/// Render the safe-harbor-allocate confirmation modal (chunk 5, D4). Revocable framing — NOT
/// typed-word (creation is reversible; contrast attest's ATTEST gate).
fn draw_safe_harbor_allocate_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &SafeHarborAllocateModalState,
) {
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Create SAFE-HARBOR ALLOCATION?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("    method          : {:?}", modal.method)),
        Line::from(format!(
            "    pre-2025 method : {:?}   (recorded, immutable)",
            modal.pre2025_method
        )),
        Line::from(format!(
            "    as_of_date      : {}",
            btctax_core::conventions::TRANSITION_DATE
        )),
        Line::from(format!(
            "    lots            : {}  (Σ {} BTC, Σ basis ${:.2})",
            modal.lot_count,
            fmt_btc(modal.total_sat),
            modal.total_basis
        )),
        Line::from("    timely_attested : false  → REVOCABLE"),
        Line::from(""),
        Line::from(Span::styled(
            "  This is a REVOCABLE snapshot: voidable ('v') while inert, TIMEBARRED until",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  you attest ('a', which makes it §7.4-IRREVOCABLE).",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  Appended as ONE decision event, saved via the vault's atomic write path."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Create     [Esc] Cancel — writes nothing",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(78, height, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: safe-harbor allocate — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Bulk link-transfer overlays (bulk-link-transfer D3) ──────────────────────

/// Render the bulk link-transfer flow overlay (four steps on one panel).
fn draw_bulk_link_flow(frame: &mut Frame, area: Rect, flow: &mut BulkLinkFlowState) {
    let modal_rect = centered_rect(96, 26, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" BULK SELF-TRANSFER — link many pending outflows to ONE wallet (non-taxable) ");
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .split(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let focus = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Banner: chosen destination.
    let dest_line = match &flow.dest {
        Some(w) => format!("destination: {}", wallet_label(w)),
        None => "destination: (not chosen)".to_string(),
    };
    frame.render_widget(
        Paragraph::new(vec![Line::from(Span::styled(
            dest_line,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))]),
        chunks[0],
    );

    match flow.step {
        BulkLinkStep::DestPick => {
            let selected = flow.wallet_list.table_state.selected();
            let rows: Vec<Row> = flow
                .wallet_list
                .items
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let style = if selected == Some(i) {
                        hl
                    } else {
                        Style::default()
                    };
                    Row::new(vec![Cell::from(wallet_label(w)).style(style)])
                })
                .collect();
            let header = Row::new(vec![Cell::from(
                "Step 1/4 — destination wallet (pick one, or press 'n' to type a new one)",
            )
            .style(bold)]);
            let table = Table::new(rows, [Constraint::Min(10)]).header(header);
            frame.render_stateful_widget(table, chunks[1], &mut flow.wallet_list.table_state);
        }
        BulkLinkStep::DestType => {
            let lines = vec![
                Line::from(Span::styled("Step 1/4 — type a destination wallet", bold)),
                Line::from(""),
                Line::from("  self:LABEL   or   exchange:PROVIDER:ACCOUNT"),
                Line::from("  (a never-seen cold wallet like self:cold-wallet is reachable here)"),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  > {}", flow.dest_buf.buf),
                    Style::default().fg(Color::Yellow),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        BulkLinkStep::Filter => {
            let src_label = match flow.source_choices.get(flow.source_idx) {
                Some(Some(w)) => wallet_label(w),
                _ => "Any".to_string(),
            };
            let year_label = match flow.year_choices.get(flow.year_idx) {
                Some(Some(y)) => y.to_string(),
                _ => "All".to_string(),
            };
            let src_style = if flow.filter_focus == 0 {
                focus
            } else {
                Style::default()
            };
            let yr_style = if flow.filter_focus == 1 {
                focus
            } else {
                Style::default()
            };
            let lines = vec![
                Line::from(Span::styled("Step 2/4 — filter", bold)),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  source wallet: "),
                    Span::styled(format!("[{src_label}]"), src_style),
                ]),
                Line::from(vec![
                    Span::raw("  time frame   : "),
                    Span::styled(format!("[{year_label}]"), yr_style),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        BulkLinkStep::Preview => {
            let selected = flow.preview.table_state.selected();
            let rows: Vec<Row> = flow
                .preview
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let base = if selected == Some(i) {
                        hl
                    } else {
                        Style::default()
                    };
                    let mark = if it.checked { "[x]" } else { "[ ]" };
                    let usd = it
                        .usd_value
                        .map(|v| format!("${v}"))
                        .unwrap_or_else(|| "—".to_string());
                    let wl = it
                        .source_wallet
                        .as_ref()
                        .map(wallet_label)
                        .unwrap_or_else(|| "(no wallet)".to_string());
                    Row::new(vec![
                        Cell::from(mark).style(base),
                        Cell::from(it.date.to_string()).style(base),
                        Cell::from(wl).style(base),
                        Cell::from(fmt_btc(it.principal_sat)).style(base),
                        Cell::from(usd).style(base),
                    ])
                })
                .collect();
            let header = Row::new(
                ["", "date", "source wallet", "BTC", "USD value"]
                    .iter()
                    .map(|h| Cell::from(*h).style(bold)),
            );
            let table = Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Length(12),
                    Constraint::Length(26),
                    Constraint::Length(16),
                    Constraint::Min(12),
                ],
            )
            .header(header);
            frame.render_stateful_widget(table, chunks[1], &mut flow.preview.table_state);
        }
    }

    // Footer: live totals (Preview only) + step hint + transient error.
    let mut footer: Vec<Line> = Vec::new();
    if matches!(flow.step, BulkLinkStep::Preview) {
        let (count, sat, floor, missing) = bulk_checked_totals(&flow.preview.items);
        footer.push(Line::from(Span::styled(
            format!(
                "checked {count} · Σ {} BTC · total USD reclassified non-taxable {}",
                fmt_btc(sat),
                bulk_usd_floor_label(floor, missing)
            ),
            bold,
        )));
    }
    let hint = match flow.step {
        BulkLinkStep::DestPick => "↑/↓: scroll  n: type a wallet  Enter: choose  Esc: cancel",
        BulkLinkStep::DestType => "Enter: parse  Esc: back",
        BulkLinkStep::Filter => "↑/↓: focus  ←/→: change  Enter: preview  Esc: back",
        BulkLinkStep::Preview => "↑/↓: scroll  Space/x: toggle  Enter: confirm  Esc: back",
    };
    footer.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Cyan),
    )));
    if let Some(err) = &flow.error {
        footer.push(Line::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

/// Render the bulk link-transfer confirmation modal (explicit; NOT typed-word — each link voidable).
fn draw_bulk_link_modal(frame: &mut Frame, area: Rect, modal: &BulkLinkModalState) {
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Apply BULK self-transfer?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("    destination : {}", wallet_label(&modal.dest))),
        Line::from(format!("    outflows    : {}", modal.count)),
        Line::from(format!("    Σ BTC       : {}", fmt_btc(modal.total_sat))),
        Line::from(format!(
            "    Σ USD made non-taxable : {}",
            bulk_usd_floor_label(modal.total_usd_value_floor, modal.missing_price_count)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Each link is individually voidable ('v').",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  Appended as N decisions, saved via the vault's atomic write path."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Apply — writes the vault    [Esc] Cancel — writes nothing",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(78, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: bulk self-transfer — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Bulk classify-inbound-self-transfer overlays (bulk-classify-inbound-self-transfer D3) ─────

/// Render the bulk STI flow overlay (two steps on one panel: Filter → Preview).
fn draw_bulk_sti_flow(frame: &mut Frame, area: Rect, flow: &mut BulkStiFlowState) {
    let modal_rect = centered_rect(96, 26, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default().borders(Borders::ALL).title(
        " BULK CLASSIFY INBOUND SELF-TRANSFER — give many unknown-basis deposits $0 basis (non-taxable) ",
    );
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let focus = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Banner.
    frame.render_widget(
        Paragraph::new(vec![Line::from(Span::styled(
            "each selected deposit → SelfTransferMine ($0 conservative basis, receipt-date HP; voidable)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))]),
        chunks[0],
    );

    match flow.step {
        BulkStiStep::Filter => {
            let wallet_label_str = match flow.wallet_choices.get(flow.wallet_idx) {
                Some(Some(w)) => wallet_label(w),
                _ => "Any".to_string(),
            };
            let year_label = match flow.year_choices.get(flow.year_idx) {
                Some(Some(y)) => y.to_string(),
                _ => "All".to_string(),
            };
            let w_style = if flow.filter_focus == 0 {
                focus
            } else {
                Style::default()
            };
            let yr_style = if flow.filter_focus == 1 {
                focus
            } else {
                Style::default()
            };
            let lines = vec![
                Line::from(Span::styled("Step 1/2 — filter", bold)),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  receiving wallet: "),
                    Span::styled(format!("[{wallet_label_str}]"), w_style),
                ]),
                Line::from(vec![
                    Span::raw("  time frame      : "),
                    Span::styled(format!("[{year_label}]"), yr_style),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        BulkStiStep::Preview => {
            let selected = flow.preview.table_state.selected();
            let rows: Vec<Row> = flow
                .preview
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let base = if selected == Some(i) {
                        hl
                    } else {
                        Style::default()
                    };
                    let mark = if it.checked { "[x]" } else { "[ ]" };
                    let usd = it
                        .usd_fmv
                        .map(|v| format!("${v}"))
                        .unwrap_or_else(|| "—".to_string());
                    let wl = it
                        .wallet
                        .as_ref()
                        .map(wallet_label)
                        .unwrap_or_else(|| "(no wallet)".to_string());
                    Row::new(vec![
                        Cell::from(mark).style(base),
                        Cell::from(it.date.to_string()).style(base),
                        Cell::from(wl).style(base),
                        Cell::from(fmt_btc(it.sat)).style(base),
                        Cell::from(usd).style(base),
                    ])
                })
                .collect();
            let header = Row::new(
                ["", "date", "receiving wallet", "BTC", "USD FMV"]
                    .iter()
                    .map(|h| Cell::from(*h).style(bold)),
            );
            let table = Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Length(12),
                    Constraint::Length(26),
                    Constraint::Length(16),
                    Constraint::Min(12),
                ],
            )
            .header(header);
            frame.render_stateful_widget(table, chunks[1], &mut flow.preview.table_state);
        }
    }

    // Footer: live totals (Preview only) + step hint + transient error.
    let mut footer: Vec<Line> = Vec::new();
    if matches!(flow.step, BulkStiStep::Preview) {
        let (count, sat, floor, missing) = bulk_sti_checked_totals(&flow.preview.items);
        footer.push(Line::from(Span::styled(
            format!(
                "checked {count} · Σ {} BTC · total USD given $0 basis {}",
                fmt_btc(sat),
                bulk_usd_floor_label(floor, missing)
            ),
            bold,
        )));
    }
    let hint = match flow.step {
        BulkStiStep::Filter => "↑/↓: focus  ←/→: change  Enter: preview  Esc: cancel",
        BulkStiStep::Preview => "↑/↓: scroll  Space/x: toggle  Enter: confirm  Esc: back",
    };
    footer.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Cyan),
    )));
    if let Some(err) = &flow.error {
        footer.push(Line::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

/// Render the bulk STI confirmation modal (explicit; NOT typed-word — each classification voidable).
fn draw_pseudo_approve_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &crate::editor::PseudoApproveModalState,
) {
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Approve pseudo-reconcile defaults as REAL decisions?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("    pending synthetic defaults : {}", modal.count)),
        Line::from(""),
        Line::from(Span::styled(
            "  Each becomes a REAL (attested) decision — no longer [PSEUDO]. Every one is voidable",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  ('v'). Unknown-basis inbounds are approved at a conservative $0 basis; correct any",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  with a real cost afterward. This does NOT turn the mode off (`reconcile pseudo off`).",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  Appended as N decisions, saved via the vault's atomic write path."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Approve — writes the vault    [Esc] Cancel — writes nothing",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];
    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(84, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: approve pseudo-reconcile defaults — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_bulk_sti_modal(frame: &mut Frame, area: Rect, modal: &BulkStiModalState) {
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Apply BULK classify-inbound-self-transfer?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("    deposits    : {}", modal.count)),
        Line::from(format!("    Σ BTC       : {}", fmt_btc(modal.total_sat))),
        Line::from(format!(
            "    Σ USD → $0 basis : {}",
            bulk_usd_floor_label(modal.total_usd_fmv_floor, modal.missing_price_count)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Each is a voidable classify-inbound decision ('v'). For any deposit whose real cost",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  you can substantiate, exclude it here and classify it single-item with a real basis",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  (classify-inbound-self-transfer --basis).",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  Appended as N decisions, saved via the vault's atomic write path."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Apply — writes the vault    [Esc] Cancel — writes nothing",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(82, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: bulk classify-inbound-self-transfer — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the bulk classify-inbound-income flow (Cycle 4). Step 1 chooses the uniform income-kind,
/// business-flag, receiving-wallet, and time-frame. Step 2 is a per-row exclude checklist over the
/// PRICED plan with date/BTC/income-USD columns, the total income recognized, and the excluded note.
fn draw_bulk_income_flow(frame: &mut Frame, area: Rect, flow: &mut BulkIncomeFlowState) {
    let modal_rect = centered_rect(96, 26, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default().borders(Borders::ALL).title(
        " BULK CLASSIFY INBOUND INCOME — recognize many unknown-basis deposits as income at auto-FMV ",
    );
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(6),
        ])
        .split(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let focus = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(
        Paragraph::new(vec![Line::from(Span::styled(
            "each selected deposit → Income at its receipt-date FMV (ordinary income + lot basis; voidable)",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))]),
        chunks[0],
    );

    match flow.step {
        BulkIncomeStep::Filter => {
            let wallet_label_str = match flow.wallet_choices.get(flow.wallet_idx) {
                Some(Some(w)) => wallet_label(w),
                _ => "Any".to_string(),
            };
            let year_label = match flow.year_choices.get(flow.year_idx) {
                Some(Some(y)) => y.to_string(),
                _ => "All".to_string(),
            };
            let style_for = |i: usize| {
                if flow.filter_focus == i {
                    focus
                } else {
                    Style::default()
                }
            };
            let lines = vec![
                Line::from(Span::styled("Step 1/2 — kind + filter", bold)),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  income kind     : "),
                    Span::styled(
                        format!("[{}]", income_kind_display(flow.kind)),
                        style_for(0),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  business (SE)   : "),
                    Span::styled(
                        format!("[{}]", if flow.business { "yes" } else { "no" }),
                        style_for(1),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  receiving wallet: "),
                    Span::styled(format!("[{wallet_label_str}]"), style_for(2)),
                ]),
                Line::from(vec![
                    Span::raw("  time frame      : "),
                    Span::styled(format!("[{year_label}]"), style_for(3)),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        BulkIncomeStep::Preview => {
            let selected = flow.preview.table_state.selected();
            let rows: Vec<Row> = flow
                .preview
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let base = if selected == Some(i) {
                        hl
                    } else {
                        Style::default()
                    };
                    let mark = if it.checked { "[x]" } else { "[ ]" };
                    Row::new(vec![
                        Cell::from(mark).style(base),
                        Cell::from(it.date.to_string()).style(base),
                        Cell::from(fmt_btc(it.sat)).style(base),
                        Cell::from(format!("${}", it.fmv)).style(base),
                    ])
                })
                .collect();
            let header = Row::new(
                ["", "date", "BTC", "income USD"]
                    .iter()
                    .map(|h| Cell::from(*h).style(bold)),
            );
            let table = Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Length(12),
                    Constraint::Length(16),
                    Constraint::Min(12),
                ],
            )
            .header(header);
            frame.render_stateful_widget(table, chunks[1], &mut flow.preview.table_state);
        }
    }

    // Footer: live totals (Preview only) + excluded-missing-price note + step hint + transient error.
    let mut footer: Vec<Line> = Vec::new();
    if matches!(flow.step, BulkIncomeStep::Preview) {
        let (count, sat, income) = bulk_income_checked_totals(&flow.preview.items);
        footer.push(Line::from(Span::styled(
            format!(
                "checked {count} · Σ {} BTC · total income recognized ${income}",
                fmt_btc(sat)
            ),
            bold,
        )));
        if flow.excluded_missing_price > 0 {
            footer.push(Line::from(Span::styled(
                format!(
                    "⚠ {} inbound(s) excluded — no price available for their date (stay pending)",
                    flow.excluded_missing_price
                ),
                Style::default().fg(Color::Yellow),
            )));
        }
    }
    let hint = match flow.step {
        BulkIncomeStep::Filter => "↑/↓: focus  ←/→: change  Enter: preview  Esc: cancel",
        BulkIncomeStep::Preview => "↑/↓: scroll  Space/x: toggle  Enter: confirm  Esc: back",
    };
    footer.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Cyan),
    )));
    if let Some(err) = &flow.error {
        footer.push(Line::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

/// Render the bulk classify-income confirmation modal (explicit; NOT typed-word — each classification is
/// voidable). Prominently shows the TOTAL income being recognized + the excluded-missing-price count.
fn draw_bulk_income_modal(frame: &mut Frame, area: Rect, modal: &BulkIncomeModalState) {
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Apply BULK classify-inbound-income?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "    deposits    : {}  ({}{})",
            modal.count,
            income_kind_display(modal.kind),
            if modal.business { ", business" } else { "" }
        )),
        Line::from(format!("    Σ BTC       : {}", fmt_btc(modal.total_sat))),
        Line::from(Span::styled(
            format!("    income recognized : ${}", modal.total_income_usd),
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ];
    if modal.excluded_missing_price > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "    excluded (no price): {} inbound(s) stay pending",
                modal.excluded_missing_price
            ),
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Each is a voidable classify-inbound decision ('v'). The FMV is the daily-close market",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(Span::styled(
        "  value at receipt — ordinary income now AND the lot's cost basis.",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  Appended as N decisions, saved via the vault's atomic write path.",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Enter] Apply — writes the vault    [Esc] Cancel — writes nothing",
        Style::default().fg(Color::Cyan),
    )));
    lines.push(Line::from(""));

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(82, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: bulk classify-inbound-income — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the bulk resolve-conflict flow (bulk-resolve-conflict D3): step 1 = batch-wide Accept/Reject
/// toggle; step 2 = per-row exclude checklist over the live conflicts (`date · target · current → new`).
fn draw_bulk_resolve_flow(frame: &mut Frame, area: Rect, flow: &mut BulkResolveFlowState) {
    let modal_rect = centered_rect(98, 26, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default().borders(Borders::ALL).title(
        " BULK RESOLVE IMPORT CONFLICTS — accept (adopt new) or reject (keep current) MANY at once ",
    );
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Banner — the batch-wide action toggle (highlighted side).
    let (accept_span, reject_span) = match flow.kind {
        ResolveKind::Accept => (
            Span::styled(
                " ACCEPT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  reject  "),
        ),
        ResolveKind::Reject => (
            Span::raw("  accept  "),
            Span::styled(
                " REJECT ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ),
    };
    frame.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled("action:  ", bold),
            accept_span,
            reject_span,
            Span::styled(
                match flow.kind {
                    ResolveKind::Accept => "   (ACCEPT adopts each new payload)",
                    ResolveKind::Reject => "   (REJECT keeps each current payload)",
                },
                Style::default().fg(Color::Cyan),
            ),
        ])]),
        chunks[0],
    );

    match flow.step {
        BulkResolveStep::Choose => {
            let lines = vec![
                Line::from(Span::styled("Step 1/2 — choose the batch-wide action", bold)),
                Line::from(""),
                Line::from(format!("  {} conflict(s) flagged", flow.preview.items.len())),
                Line::from(""),
                Line::from(Span::styled(
                    "  These resolutions are NON-REVOCABLE (a wrong accept/reject cannot be voided).",
                    Style::default().fg(Color::Yellow),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        BulkResolveStep::Preview => {
            let selected = flow.preview.table_state.selected();
            let rows: Vec<Row> = flow
                .preview
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let base = if selected == Some(i) {
                        hl
                    } else {
                        Style::default()
                    };
                    let mark = if it.checked { "[x]" } else { "[ ]" };
                    let change = format!("{} → {}", it.current_summary, it.new_summary);
                    Row::new(vec![
                        Cell::from(mark).style(base),
                        Cell::from(it.date.to_string()).style(base),
                        Cell::from(it.target.canonical()).style(base),
                        Cell::from(it.new_fingerprint.clone()).style(base),
                        Cell::from(change).style(base),
                    ])
                })
                .collect();
            let header = Row::new(
                ["", "date", "target", "new-fp", "current → new"]
                    .iter()
                    .map(|h| Cell::from(*h).style(bold)),
            );
            let table = Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Length(12),
                    Constraint::Length(22),
                    Constraint::Length(10),
                    Constraint::Min(24),
                ],
            )
            .header(header);
            frame.render_stateful_widget(table, chunks[1], &mut flow.preview.table_state);
        }
    }

    // Footer: checked count + action (Preview only) + step hint + transient error.
    let mut footer: Vec<Line> = Vec::new();
    if matches!(flow.step, BulkResolveStep::Preview) {
        let count = bulk_resolve_checked_count(&flow.preview.items);
        let action = match flow.kind {
            ResolveKind::Accept => "Accept",
            ResolveKind::Reject => "Reject",
        };
        footer.push(Line::from(Span::styled(
            format!("checked {count} · action {action}"),
            bold,
        )));
    }
    let hint = match flow.step {
        BulkResolveStep::Choose => "←/→ (h/l): toggle accept/reject  Enter: preview  Esc: cancel",
        BulkResolveStep::Preview => "↑/↓: scroll  Space/x: toggle  Enter: confirm  Esc: back",
    };
    footer.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Cyan),
    )));
    if let Some(err) = &flow.error {
        footer.push(Line::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

/// Render the bulk resolve-conflict confirmation modal — Tier-B NON-REVOCABLE (NOT typed-word). Reuses
/// the shipped non-revocable warning framing, PLURALIZED, plus the checked count + the chosen action.
fn draw_bulk_resolve_modal(frame: &mut Frame, area: Rect, modal: &BulkResolveModalState) {
    let (action, adopt_note) = match modal.kind {
        ResolveKind::Accept => ("ACCEPT", "(ACCEPT adopts each new payload)"),
        ResolveKind::Reject => ("REJECT", "(REJECT keeps each current, discards new)"),
    };
    let lines: Vec<Line> =
        vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Apply BULK ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                action,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" over {} import conflict(s)?", modal.count),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("    conflicts : {}   {adopt_note}", modal.count)),
        Line::from(""),
        Line::from(Span::styled(
            "  !! These decisions CANNOT be voided (non-revocable).",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  A wrong accept/reject is recoverable only out-of-band (re-import / classify-raw).",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  Appended as N decisions, saved via the vault's atomic write path."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Apply — writes the vault    [Esc] Cancel — writes nothing",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(84, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: bulk resolve-conflict — WRITES THE VAULT (NON-REVOCABLE) ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the bulk-void flow (bulk-void D3): a single per-row-exclude checklist over the voidable
/// decisions (`seq · type · what the void undoes`); each row rendered from `summarize_void_payload`.
fn draw_bulk_void_flow(frame: &mut Frame, area: Rect, flow: &mut BulkVoidFlowState) {
    let modal_rect = centered_rect(98, 26, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default().borders(Borders::ALL).title(
        " BULK VOID DECISIONS — sweep-void MANY revocable decisions at once (NON-REVOCABLE) ",
    );
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let selected = flow.preview.table_state.selected();
    let rows: Vec<Row> = flow
        .preview
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let base = if selected == Some(i) {
                hl
            } else {
                Style::default()
            };
            let mark = if it.checked { "[x]" } else { "[ ]" };
            // Flag the blast-radius rows (LotSelection voids re-expose disposals + clear attestations).
            let ls_flag = if it.disposal_to_clear.is_some() {
                " (re-exposes disposal)"
            } else {
                ""
            };
            let summary = format!("{}{}", it.target_summary, ls_flag);
            Row::new(vec![
                Cell::from(mark).style(base),
                Cell::from(it.seq.to_string()).style(base),
                Cell::from(it.payload_tag).style(base),
                Cell::from(summary).style(base),
            ])
        })
        .collect();
    let header = Row::new(
        ["", "seq", "type", "what the void undoes"]
            .iter()
            .map(|h| Cell::from(*h).style(bold)),
    );
    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(24),
            Constraint::Min(30),
        ],
    )
    .header(header);
    frame.render_stateful_widget(table, chunks[0], &mut flow.preview.table_state);

    // Footer: checked count + LotSelection blast-radius count + hint + transient error.
    let checked = bulk_void_checked_count(&flow.preview.items);
    let ls = bulk_void_lot_selection_checked_count(&flow.preview.items);
    let mut footer: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("checked {checked} · {ls} lot-selection void(s) re-expose disposals"),
            bold,
        )),
        Line::from(Span::styled(
            "↑/↓: scroll  Space/x: toggle  Enter: confirm  Esc: cancel",
            Style::default().fg(Color::Cyan),
        )),
    ];
    if let Some(err) = &flow.error {
        footer.push(Line::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(footer), chunks[1]);
}

/// Render the bulk-void confirmation modal — Tier-B NON-REVOCABLE + high blast-radius (red border,
/// prominent warning, NOT a typed-word). States N voids, that these voids CANNOT themselves be undone
/// (re-apply the original decision to restore), and how many are LotSelection voids that re-expose
/// disposals + clear attestations.
fn draw_bulk_void_modal(frame: &mut Frame, area: Rect, modal: &BulkVoidModalState) {
    let blast = if modal.lot_selection_count > 0 {
        format!(
            "{} of these are LotSelection voids — they re-expose their disposals to the default \
             method and clear their optimizer attestation.",
            modal.lot_selection_count
        )
    } else {
        "None of these are LotSelection voids (no disposals re-exposed).".to_string()
    };

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Void ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{}", modal.count),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " revocable decision(s)?",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  !! These voids CANNOT themselves be undone (a void is non-revocable).",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  To restore a voided decision, RE-APPLY the original decision.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {blast}"),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("  Appended as N VoidDecisionEvents, saved via the vault's atomic write path."),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Void — writes the vault    [Esc] Cancel — writes nothing",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
    ];

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(88, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: BULK VOID — WRITES THE VAULT (NON-REVOCABLE) ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Short display for the batch-wide disposition kind (Sell/Spend).
fn dispose_kind_display(kind: DisposeKind) -> &'static str {
    match kind {
        DisposeKind::Sell => "sell",
        DisposeKind::Spend => "spend",
    }
}

/// Render the bulk reclassify-outflow flow (Cycle 5). Step 1 chooses the uniform disposition kind
/// (Sell/Spend), source-wallet, and time-frame. Step 2 is a per-row exclude checklist over the PRICED
/// plan with date/BTC/est.proceeds/est.basis/est.gain columns + the total ESTIMATED gain + excluded note.
fn draw_bulk_reclassify_outflow_flow(
    frame: &mut Frame,
    area: Rect,
    flow: &mut BulkReclassifyOutflowFlowState,
) {
    let modal_rect = centered_rect(98, 26, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default().borders(Borders::ALL).title(
        " BULK RECLASSIFY OUTFLOWS — reclassify many pending outflows as dispositions at ESTIMATED FMV ",
    );
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(6),
        ])
        .split(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let focus = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(
        Paragraph::new(vec![Line::from(Span::styled(
            "each selected outflow → a disposition; the daily-close FMV is the ESTIMATED proceeds (voidable)",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))]),
        chunks[0],
    );

    match flow.step {
        BulkReclassifyOutflowStep::Filter => {
            let wallet_label_str = match flow.wallet_choices.get(flow.wallet_idx) {
                Some(Some(w)) => wallet_label(w),
                _ => "Any".to_string(),
            };
            let year_label = match flow.year_choices.get(flow.year_idx) {
                Some(Some(y)) => y.to_string(),
                _ => "All".to_string(),
            };
            let style_for = |i: usize| {
                if flow.filter_focus == i {
                    focus
                } else {
                    Style::default()
                }
            };
            let lines = vec![
                Line::from(Span::styled("Step 1/2 — kind + filter", bold)),
                Line::from(""),
                Line::from(vec![
                    Span::raw("  disposition kind: "),
                    Span::styled(
                        format!("[{}]", dispose_kind_display(flow.kind)),
                        style_for(0),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  source wallet   : "),
                    Span::styled(format!("[{wallet_label_str}]"), style_for(1)),
                ]),
                Line::from(vec![
                    Span::raw("  time frame      : "),
                    Span::styled(format!("[{year_label}]"), style_for(2)),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), chunks[1]);
        }
        BulkReclassifyOutflowStep::Preview => {
            let selected = flow.preview.table_state.selected();
            let rows: Vec<Row> = flow
                .preview
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let base = if selected == Some(i) {
                        hl
                    } else {
                        Style::default()
                    };
                    let mark = if it.checked { "[x]" } else { "[ ]" };
                    Row::new(vec![
                        Cell::from(mark).style(base),
                        Cell::from(it.date.to_string()).style(base),
                        Cell::from(fmt_btc(it.principal_sat)).style(base),
                        Cell::from(format!("${}", it.fmv)).style(base),
                        Cell::from(format!("${}", it.basis_usd)).style(base),
                        Cell::from(format!("${}", it.estimated_gain)).style(base),
                    ])
                })
                .collect();
            let header = Row::new(
                ["", "date", "BTC", "est.proceeds", "est.basis", "est.gain"]
                    .iter()
                    .map(|h| Cell::from(*h).style(bold)),
            );
            let table = Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Length(12),
                    Constraint::Length(14),
                    Constraint::Length(14),
                    Constraint::Length(14),
                    Constraint::Min(12),
                ],
            )
            .header(header);
            frame.render_stateful_widget(table, chunks[1], &mut flow.preview.table_state);
        }
    }

    // Footer: live totals (Preview only) + excluded-missing-price note + step hint + transient error.
    let mut footer: Vec<Line> = Vec::new();
    if matches!(flow.step, BulkReclassifyOutflowStep::Preview) {
        let (count, sat, proceeds, _basis, gain) =
            bulk_reclassify_outflow_checked_totals(&flow.preview.items);
        footer.push(Line::from(Span::styled(
            format!(
                "checked {count} · Σ {} BTC · total ESTIMATED proceeds ${proceeds} · total ESTIMATED gain ${gain}",
                fmt_btc(sat)
            ),
            bold,
        )));
        if flow.excluded_missing_price > 0 {
            footer.push(Line::from(Span::styled(
                format!(
                    "⚠ {} outflow(s) excluded — no price available for their date (stay pending)",
                    flow.excluded_missing_price
                ),
                Style::default().fg(Color::Yellow),
            )));
        }
    }
    let hint = match flow.step {
        BulkReclassifyOutflowStep::Filter => "↑/↓: focus  ←/→: change  Enter: preview  Esc: cancel",
        BulkReclassifyOutflowStep::Preview => {
            "↑/↓: scroll  Space/x: toggle  Enter: confirm  Esc: back"
        }
    };
    footer.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::Cyan),
    )));
    if let Some(err) = &flow.error {
        footer.push(Line::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

/// Render the bulk reclassify-outflow confirmation modal (explicit; NOT typed-word — each reclassify is
/// voidable, the REVOCABLE tier). Prominently BOLDS the total ESTIMATED proceeds AND the total ESTIMATED
/// gain (the word "ESTIMATED" adjacent to both), plus the excluded-missing-price note.
fn draw_bulk_reclassify_outflow_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &BulkReclassifyOutflowModalState,
) {
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Apply BULK reclassify-outflow?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "    outflows    : {}  ({})",
            modal.count,
            dispose_kind_display(modal.kind)
        )),
        Line::from(format!("    Σ BTC       : {}", fmt_btc(modal.total_sat))),
        Line::from(Span::styled(
            format!("    ESTIMATED proceeds : ${}", modal.total_proceeds_usd),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "    basis              : ${}",
            modal.total_basis_usd
        )),
        Line::from(Span::styled(
            format!("    ESTIMATED gain     : ${}", modal.total_estimated_gain),
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ];
    if modal.excluded_missing_price > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "    excluded (no price): {} outflow(s) stay pending",
                modal.excluded_missing_price
            ),
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  The proceeds are ESTIMATED — the daily-close market FMV of the BTC that left, not a",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(Span::styled(
        "  recorded sale price. Each is a VOIDABLE decision ('v') — refine the FMV later.",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(
        "  Appended as N decisions + flagged [est], saved via the vault's atomic write path.",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Enter] Apply — writes the vault    [Esc] Cancel — writes nothing",
        Style::default().fg(Color::Cyan),
    )));
    lines.push(Line::from(""));

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(84, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm: bulk reclassify-outflow — WRITES THE VAULT (ESTIMATED proceeds) ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the match-self-transfers proposal list (self-transfer-passthrough C3). One row per proposed
/// pair: suggested action, both legs' date/wallet/sat, USD value, and the AMBIGUOUS / txid flags.
fn draw_match_self_transfers_flow(
    frame: &mut Frame,
    area: Rect,
    flow: &mut MatchSelfTransfersFlowState,
) {
    let modal_rect = centered_rect(98, 24, area);
    frame.render_widget(Clear, modal_rect);
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" MATCH SELF-TRANSFERS — confirm a matched in/out pair (Enter); never automatic ");
    let inner = outer.inner(modal_rect);
    frame.render_widget(outer, modal_rect);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let hl = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let warn = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    let selected = flow.list.table_state.selected();
    let rows: Vec<Row> = flow
        .list
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let action = match it.suggested {
                crate::edit::form::MatchPairAction::Drop => "DROP",
                crate::edit::form::MatchPairAction::Relocate => "RELOCATE",
            };
            let in_w = it
                .in_wallet
                .as_ref()
                .map(wallet_label)
                .unwrap_or_else(|| "(no wallet)".to_string());
            let out_w = it
                .out_wallet
                .as_ref()
                .map(wallet_label)
                .unwrap_or_else(|| "(no wallet)".to_string());
            let usd = match it.usd_value {
                Some(v) => format!("${v}"),
                None => "\u{2014}".to_string(),
            };
            let mut flags = String::new();
            if it.ambiguous {
                flags.push_str(" [AMBIGUOUS]");
            }
            if it.txid_match {
                flags.push_str(" [txid]");
            }
            let text = format!(
                "{action:<8} in {} {} {} sat  →  out {} {} {} sat  {usd}{flags}",
                it.in_date, in_w, it.in_sat, it.out_date, out_w, it.out_principal_sat,
            );
            let style = if selected == Some(i) {
                hl
            } else if it.ambiguous {
                warn
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(text).style(style)])
        })
        .collect();
    let header = Row::new(vec![Cell::from(
        "proposed pairs — k/j move, Enter confirm (choose DROP/RELOCATE), Esc cancel",
    )
    .style(bold)]);
    let table = Table::new(rows, [Constraint::Min(10)]).header(header);
    frame.render_stateful_widget(table, chunks[0], &mut flow.list.table_state);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  A coincidental amount match of a real income + a real sale must NOT be dropped — \
             confirm only true self-transfers.",
            warn,
        ))),
        chunks[1],
    );
}

/// Render the match-self-transfers confirm modal (DROP vs RELOCATE choice; explicit confirm).
fn draw_match_self_transfers_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &MatchSelfTransfersModalState,
) {
    let in_w = modal
        .in_wallet
        .as_ref()
        .map(wallet_label)
        .unwrap_or_else(|| "(no wallet)".to_string());
    let out_w = modal
        .out_wallet
        .as_ref()
        .map(wallet_label)
        .unwrap_or_else(|| "(no wallet)".to_string());
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Confirm this self-transfer match?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "    in  : {} — {} sat @ {}",
            modal.in_event.canonical(),
            modal.in_sat,
            in_w
        )),
        Line::from(format!(
            "    out : {} — {} sat @ {}",
            modal.out_event.canonical(),
            modal.out_principal_sat,
            out_w
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("    action : "),
            Span::styled(
                format!("[{}]", modal.action.label()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "    (←/→ or Tab, or 'd'/'r', to switch DROP↔RELOCATE)",
            Style::default().fg(Color::Yellow),
        )),
    ];
    if modal.ambiguous {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  AMBIGUOUS: this leg matched more than one counterpart — be sure this is the right pair.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [Enter] Apply — writes the vault    [Esc] Cancel — writes nothing",
        Style::default().fg(Color::Cyan),
    )));
    lines.push(Line::from(""));

    let height = (lines.len() + 2) as u16;
    let modal_rect = centered_rect(84, height, area);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Confirm: self-transfer match — WRITES THE VAULT ");
    let inner = block.inner(modal_rect);
    frame.render_widget(block, modal_rect);
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::form::MutationModalState;
    use btctax_core::{Carryforward, FilingStatus, TaxProfile};
    use ratatui::{backend::TestBackend, Terminal};
    use rust_decimal_macros::dec;
    use std::path::PathBuf;

    fn fixture_profile() -> TaxProfile {
        TaxProfile {
            filing_status: FilingStatus::Mfj,
            ordinary_taxable_income: dec!(120000),
            magi_excluding_crypto: dec!(130000),
            qualified_dividends_and_other_pref_income: dec!(5000),
            other_net_capital_gain: dec!(1000),
            capital_loss_carryforward_in: Carryforward {
                short: dec!(500),
                long: dec!(250),
            },
            w2_ss_wages: dec!(80000),
            w2_medicare_wages: dec!(85000),
            schedule_c_expenses: dec!(3000),
        }
    }

    // ── KAT-F2: modal payload exactness ─────────────────────────────────────

    #[test]
    fn kat_f2_modal_renders_year_and_all_10_leaf_fields() {
        // A standard 80x24 terminal: the WHOLE payload (all 10 leaf fields + year)
        // AND the Enter/Esc legend must be visible — centered_rect clamps the modal
        // height to the area, so an oversized modal would clip its bottom lines.
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let modal = MutationModalState {
            year: 2025,
            profile: fixture_profile(),
        };
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_mutation_modal(f, area, &modal))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            rendered.contains("2025"),
            "modal must contain the year 2025"
        );
        assert!(
            rendered.contains("mfj"),
            "modal must contain filing_status tag"
        );
        assert!(
            rendered.contains("ordinary_taxable_income"),
            "modal must show ordinary_taxable_income"
        );
        assert!(
            rendered.contains("magi_excluding_crypto"),
            "modal must show magi_excluding_crypto"
        );
        assert!(
            rendered.contains("qualified_dividends_and_other_pref_income"),
            "modal must show qualified_dividends"
        );
        assert!(
            rendered.contains("other_net_capital_gain"),
            "modal must show other_net_capital_gain"
        );
        assert!(
            rendered.contains("capital_loss_carryforward_in.short"),
            "modal must show carryforward short"
        );
        assert!(
            rendered.contains("capital_loss_carryforward_in.long"),
            "modal must show carryforward long"
        );
        assert!(
            rendered.contains("w2_ss_wages"),
            "modal must show w2_ss_wages"
        );
        assert!(
            rendered.contains("w2_medicare_wages"),
            "modal must show w2_medicare_wages"
        );
        assert!(
            rendered.contains("schedule_c_expenses"),
            "modal must show schedule_c_expenses"
        );

        // ── Value assertions — spec requires "with the validated values" ─────────
        // Fixture values are pairwise-distinct; three need contextual anchors
        // because their digit sequences are substrings of other values:
        //   "5000" ⊂ "85000", "500" ⊂ "85000", "3000" ⊂ "130000".
        assert!(
            rendered.contains("120000"),
            "modal must show ordinary_taxable_income value 120000"
        );
        assert!(
            rendered.contains("130000"),
            "modal must show magi_excluding_crypto value 130000"
        );
        // "5000" is a substring of "85000"; anchor to the field name.
        assert!(
            rendered.contains("pref_income: 5000"),
            "modal must show qualified_dividends value 5000 (anchored to avoid collision with 85000)"
        );
        assert!(
            rendered.contains("1000"),
            "modal must show other_net_capital_gain value 1000"
        );
        // "500" is a substring of "85000"; anchor to the field name.
        assert!(
            rendered.contains("short: 500"),
            "modal must show carryforward short value 500 (anchored to avoid collision with 85000)"
        );
        assert!(
            rendered.contains("250"),
            "modal must show carryforward long value 250"
        );
        assert!(
            rendered.contains("80000"),
            "modal must show w2_ss_wages value 80000"
        );
        assert!(
            rendered.contains("85000"),
            "modal must show w2_medicare_wages value 85000"
        );
        // "3000" is a substring of "130000"; anchor to the colon-space prefix.
        assert!(
            rendered.contains(": 3000"),
            "modal must show schedule_c_expenses value 3000 (anchored to avoid collision with 130000)"
        );

        assert!(
            rendered.contains("WRITES THE VAULT"),
            "modal title must say WRITES THE VAULT"
        );
        assert!(
            rendered.contains("writes nothing"),
            "modal must say Esc writes nothing"
        );
    }

    /// UX-P4-12(h): a list-flow footer legend shows the exit affordance and is free of the dev-speak
    /// "q: swallowed" (which also wrapped mid-word at narrow widths). Rendered in a tall buffer so the
    /// footer row is not clipped.
    #[test]
    fn list_flow_footer_has_no_dev_speak_swallowed() {
        use crate::edit::form::{TargetList, VoidFlowState, VoidStep};
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut flow = VoidFlowState {
            list: TargetList::new(vec![]),
            step: VoidStep::List,
        };
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_void_list(f, area, &mut flow))
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            rendered.contains("Esc: close"),
            "the footer legend is visible:\n{rendered}"
        );
        assert!(
            !rendered.contains("swallowed"),
            "no dev-speak 'q: swallowed' in the footer legend"
        );
    }

    // ── Form renders without panic ───────────────────────────────────────────

    #[test]
    fn profile_form_renders_without_panic() {
        use crate::edit::form::ProfileFormState;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut form = ProfileFormState::new(2025);
        form.fields[0].set("120000");
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_profile_form(f, area, &form))
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(rendered.contains("2025"), "form must contain the year 2025");
        assert!(
            rendered.contains("ordinary_taxable_income"),
            "form must show field label"
        );
    }

    // ── Tax-inputs 3-region render (plan 3 task 2) ───────────────────────────

    /// Flatten a rendered `Buffer` into a row-major `String` (one char per cell) — the recon's
    /// cell-collect pattern, named for the tax-inputs snapshot tests.
    fn flatten(buf: &ratatui::buffer::Buffer) -> String {
        buf.content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect()
    }

    /// NI-2: on a `None` working copy the render shows ONLY the filing-status choice — no other
    /// section is offered until a filing status is chosen.
    #[test]
    fn tax_inputs_renders_only_filing_status_when_fresh() {
        use crate::edit::form::TaxInputsFormState;
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let form = TaxInputsFormState::fresh(2024); // working = None
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("Filing status"),
            "the fresh screen shows the filing-status choice"
        );
        assert!(
            !r.contains("W-2"),
            "no other section is offered until filing status is chosen (NI-2)"
        );
    }

    /// Review r1 Important-1: the ProRata attest-only NOTE must render IN FULL inside the ≤96-col modal on
    /// an 80-col terminal. The old single 225-char line clipped mid-word so the actionable sentence never
    /// showed. Pins that BOTH the "NOT auto-computed" warning and the "attest it yourself" instruction render.
    #[test]
    fn safe_harbor_allocate_prorata_note_renders_in_full_on_80_cols() {
        use crate::edit::form::{SafeHarborAllocateFlowState, SafeHarborAllocateStep, TargetList};
        use btctax_core::event::AllocMethod;
        use btctax_core::{LotMethod, Usd};
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut flow = SafeHarborAllocateFlowState {
            lots: vec![],
            total_sat: 0,
            total_basis: Usd::ZERO,
            method: AllocMethod::ProRata,
            pre2025_method: LotMethod::Fifo,
            list: TargetList::new(vec![]),
            step: SafeHarborAllocateStep::Preview,
        };
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_safe_harbor_allocate_preview(f, area, &mut flow))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("NOT auto-computed"),
            "the attest-only warning must render on 80 cols; got: {r}"
        );
        assert!(
            r.contains("attest it yourself"),
            "the actionable instruction must render in full (it was clipped before the fix)"
        );
    }

    /// Task 3: while editing a focused text-kind field the pane shows the RAW buffer being typed (with a
    /// cursor block), NOT the committed value — the value only updates on a successful parse+apply.
    #[test]
    fn tax_inputs_renders_the_edit_buffer_while_editing() {
        use crate::edit::form::{live_fields, live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        // Focus Payments → PayEstimated and enter edit mode with a partial buffer.
        let ri = form.working.as_ref().unwrap();
        let sections = live_sections(ri);
        let sec = sections
            .iter()
            .position(|s| s.id == SectionId::Payments)
            .unwrap();
        let fld = live_fields(sections[sec], ri)
            .iter()
            .position(|f| f.id == FieldId::PayEstimated)
            .unwrap();
        form.section_idx = sec;
        form.field_focus = fld;
        form.editing = true;
        form.buf.set("123");

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("123\u{2588}"),
            "the edit buffer shows the typed text with a cursor while editing"
        );
    }

    /// Task 4 (a) — render-layer masking KAT (folds the Task-2 review Minor): a SET Secret (SSN) field's
    /// DISPLAY value shows the masked form (`***-**-NNNN`) and NEVER the raw or middle digits. The SSN is
    /// set through the engine (`SecretEntry` inbound), never a leaf assignment.
    #[test]
    fn tax_inputs_secret_display_is_masked_never_digits() {
        use crate::edit::form::{live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        // Set the taxpayer SSN via the engine (SecretEntry is inbound-only).
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::TpSsn,
                addr: RowAddr::default(),
                value: FieldValue::SecretEntry("123456789".into()),
            },
        )
        .unwrap();
        // Focus the Taxpayer section (display, NOT editing).
        let ri = form.working.as_ref().unwrap();
        let sections = live_sections(ri);
        form.section_idx = sections
            .iter()
            .position(|s| s.id == SectionId::Taxpayer)
            .unwrap();

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(r.contains("***-**-"), "a set SSN renders its masked form");
        assert!(
            !r.contains("123456789"),
            "the raw SSN digits must never render on display"
        );
        assert!(
            !r.contains("12345"),
            "the masked middle digits must never render on display"
        );
    }

    /// Task 4 (b) — no-echo entry: while ENTERING a Secret (SSN), the pane shows one bullet per typed char
    /// and NEVER the typed digits. This is the render the no-leak mutation-check targets.
    #[test]
    fn tax_inputs_secret_entry_shows_bullets_never_digits() {
        use crate::edit::form::{live_fields, live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        // Focus Taxpayer → SSN and enter no-echo mode with digits already in the buffer.
        let ri = form.working.as_ref().unwrap();
        let sections = live_sections(ri);
        let sec = sections
            .iter()
            .position(|s| s.id == SectionId::Taxpayer)
            .unwrap();
        let fld = live_fields(sections[sec], ri)
            .iter()
            .position(|f| f.id == FieldId::TpSsn)
            .unwrap();
        form.section_idx = sec;
        form.field_focus = fld;
        form.editing = true;
        form.buf.set("123456789");

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains(&"\u{25cf}".repeat(9)),
            "no-echo entry renders one bullet per typed char"
        );
        assert!(
            !r.contains("123456789"),
            "the typed SSN digits must never render during entry"
        );
        assert!(
            !r.contains("12345"),
            "no run of typed digits may render during entry"
        );
    }

    /// A materialized `Single` return: the left pane lists the live sections in §9A order (Spouse
    /// hidden; the nested charitable section skipped), the right pane shows the selected section's
    /// live fields as `label  [value]`, and the bottom shows the active source + a key legend.
    #[test]
    fn tax_inputs_lists_live_sections_for_single_return() {
        use crate::edit::form::TaxInputsFormState;
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        // Materialize a Single return via `apply` — never construct a `ReturnInputs` directly (NI-2).
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());

        // Left pane lists every live section title.
        for title in [
            "Return options",
            "Taxpayer",
            "Address",
            "Dependents",
            "W-2s",
            "Schedule A",
            "Payments",
            "Declarations",
            "Skippables",
        ] {
            assert!(
                r.contains(title),
                "left pane must list the section {title:?}"
            );
        }
        // Spouse is HIDDEN on Single; the nested charitable section is not a top-level entry.
        assert!(!r.contains("Spouse"), "Spouse is hidden on a Single return");
        assert!(
            !r.contains("charitable"),
            "the nested Schedule-A charitable section is not a top-level left-pane entry"
        );

        // §9A order (left-pane rows are top-to-bottom → monotonic in the row-major flatten).
        let pos = |t: &str| r.find(t).unwrap_or_else(|| panic!("missing {t:?}"));
        let order = [
            "Return options",
            "Taxpayer",
            "Address",
            "Dependents",
            "W-2s",
            "Schedule A",
            "Payments",
            "Declarations",
            "Skippables",
        ];
        for w in order.windows(2) {
            assert!(
                pos(w[0]) < pos(w[1]),
                "section order: {:?} must precede {:?}",
                w[0],
                w[1]
            );
        }

        // Per-section status glyph: ReturnOptions is complete (its one live field is set) → ✓; an
        // incomplete section (e.g. Taxpayer, blank name) → …. Both appear in the left pane.
        assert!(r.contains('✓'), "a complete section shows the ✓ glyph");
        assert!(r.contains('…'), "an incomplete section shows the … glyph");

        // Right pane: the selected section (ReturnOptions, idx 0) shows its field as `label  [value]`.
        assert!(
            r.contains("[Single]"),
            "the filing-status value renders as the chosen enum"
        );

        // Bottom: the active-source line + a key legend naming section navigation.
        assert!(
            r.contains("active source"),
            "status line shows the active source"
        );
        assert!(
            r.contains("section"),
            "the key legend mentions section navigation"
        );
    }

    /// Task 8: the status line renders the CACHED active-source label (`full return` / `tax-profile` /
    /// `(none)`), replacing Task 2's hardcoded placeholder — a `tax-profile` cache renders "tax-profile".
    #[test]
    fn tax_inputs_status_renders_cached_active_source_label() {
        use crate::edit::form::TaxInputsFormState;
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        form.active_source_label = "tax-profile"; // the cache the opener/park handler sets
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("active source"),
            "the status line labels the active source"
        );
        assert!(
            r.contains("tax-profile"),
            "the status line renders the cached active-source label, not a hardcoded placeholder"
        );
    }

    /// Task 5: a repeating section (W-2s) at its row-list level renders the rows as a selectable list
    /// (index per row, the selected row marked) plus the `[a] add / [d] remove / [Enter] edit row` legend —
    /// NOT the section's 13 W-2 fields (the field-cursor fold at the render layer).
    #[test]
    fn tax_inputs_repeating_section_renders_a_selectable_row_list() {
        use crate::edit::form::{live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        // Two W-2 rows via the engine.
        for _ in 0..2 {
            apply(
                &mut form.working,
                Edit::AddRow {
                    section: SectionId::W2s,
                    parent: RowAddr::default(),
                },
            )
            .unwrap();
        }
        // Select the W-2s section (row list, addr []), cursor on the second row.
        let ri = form.working.as_ref().unwrap();
        form.section_idx = live_sections(ri)
            .iter()
            .position(|s| s.id == SectionId::W2s)
            .unwrap();
        form.field_focus = 1;

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(r.contains("#1"), "the row list shows the first row's index");
        assert!(
            r.contains("#2"),
            "the row list shows the second row's index"
        );
        assert!(
            r.contains("[a] add") && r.contains("[d] remove"),
            "the row list shows the add/remove legend"
        );
        // The selected-row marker '>' appears (row 2 is focused).
        assert!(r.contains("> #2"), "the focused row is marked");
    }

    /// Task-5 fix: inside a W-2 row the pane renders a synthetic "Box 12 entries (n) →" drill entry, and
    /// DESCENDING (`descent = W2Box12`) swaps in the box-12 sub-list with its own add/remove/back legend and
    /// title — so the nested group is reachable and visible, not just addressable.
    #[test]
    fn tax_inputs_renders_box12_drill_entry_and_nested_sublist() {
        use crate::edit::form::{live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        apply(
            &mut form.working,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
        )
        .unwrap();
        let ri = form.working.as_ref().unwrap();
        form.section_idx = live_sections(ri)
            .iter()
            .position(|s| s.id == SectionId::W2s)
            .unwrap();

        // Inside the W-2 row (addr [0], descent None): the synthetic drill entry renders.
        form.addr = RowAddr(vec![0]);
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("Box 12 entries (0)"),
            "the W-2 row pane shows the synthetic box-12 drill entry"
        );

        // Descended into the box-12 group (addr [0] is the parent path): the sub-list + its legend render.
        form.descent = Some(SectionId::W2Box12);
        form.field_focus = 0;
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("W-2 box 12"),
            "the descended pane titles the nested box-12 group"
        );
        assert!(
            r.contains("[a] add") && r.contains("[Left/Esc] back"),
            "the box-12 sub-list shows the add + back legend"
        );
    }

    /// Task 5: the remove-confirm modal renders the payload it will delete ("remove W-2 #1?") + its legend.
    #[test]
    fn tax_inputs_remove_confirm_modal_names_the_row() {
        use crate::edit::form::{live_sections, PendingRemove, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        apply(
            &mut form.working,
            Edit::AddRow {
                section: SectionId::W2s,
                parent: RowAddr::default(),
            },
        )
        .unwrap();
        let ri = form.working.as_ref().unwrap();
        form.section_idx = live_sections(ri)
            .iter()
            .position(|s| s.id == SectionId::W2s)
            .unwrap();
        form.pending_remove = Some(PendingRemove {
            section: SectionId::W2s,
            addr: RowAddr(vec![0]),
            label: "remove W-2 #1?".to_string(),
        });

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("remove W-2 #1?"),
            "the confirm names the exact row"
        );
        assert!(
            r.contains("[Enter] remove") && r.contains("[Esc] cancel"),
            "the confirm shows its legend"
        );
    }

    // ── Task 9: §9A/§10 snapshot + KAT coverage sweep ───────────────────────────────────────────────────
    //
    // §10 KAT #1 (empty year) is already pinned above as a named §9A KAT by
    // `tax_inputs_renders_only_filing_status_when_fresh` — no new test needed.

    /// §10 KAT #2: a two-W-2 MFJ return, built via `apply` edits (Mfj + two `AddRow{W2s}` + Box-1 wages on
    /// each row — never a direct `ReturnInputs` construction, per NI-2). The render shows BOTH W-2 rows
    /// (the row list at the W2s section), the MFJ filing status, and the Spouse section now present in the
    /// left pane (hidden on Single — `tax_inputs_lists_live_sections_for_single_return`).
    #[test]
    fn tax_inputs_two_w2_mfj_return_renders_rows_status_and_spouse() {
        use crate::edit::form::{live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Mfj".into()),
            },
        )
        .unwrap();
        for _ in 0..2 {
            apply(
                &mut form.working,
                Edit::AddRow {
                    section: SectionId::W2s,
                    parent: RowAddr::default(),
                },
            )
            .unwrap();
        }
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![0]),
                value: FieldValue::Money(dec!(60000)),
            },
        )
        .unwrap();
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::Box1Wages,
                addr: RowAddr(vec![1]),
                value: FieldValue::Money(dec!(45000)),
            },
        )
        .unwrap();

        // Both box-1 values round-trip through the `get` accessor (never a leaf) — proves the `apply`
        // edits landed on the RIGHT rows.
        let ri = form.working.as_ref().unwrap();
        let box1 = btctax_input_form::form_spec()
            .iter()
            .find(|s| s.id == SectionId::W2s)
            .unwrap()
            .fields
            .iter()
            .find(|f| f.id == FieldId::Box1Wages)
            .unwrap();
        assert_eq!(
            (box1.get)(ri, &RowAddr(vec![0])),
            Some(FieldValue::Money(dec!(60000))),
            "row 0's box-1 wages"
        );
        assert_eq!(
            (box1.get)(ri, &RowAddr(vec![1])),
            Some(FieldValue::Money(dec!(45000))),
            "row 1's box-1 wages"
        );

        // Render 1 (ReturnOptions selected, the default `section_idx = 0`): the MFJ filing status renders,
        // and the left pane now lists Spouse (hidden on Single).
        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("[Mfj]"),
            "the filing-status value renders as Mfj"
        );
        assert!(
            r.contains("Spouse"),
            "MFJ offers the Spouse section (hidden on Single)"
        );

        // Render 2: select the W2s section — both rows appear in the row list.
        form.section_idx = live_sections(ri)
            .iter()
            .position(|s| s.id == SectionId::W2s)
            .unwrap();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(r.contains("#1"), "the first W-2 row renders");
        assert!(r.contains("#2"), "the second W-2 row renders");
    }

    /// §10 KAT #4: the commit payload-confirm modal RENDERS the filing status + the full payload summary
    /// (n W-2s, Schedule A yes/no, n dependents) — Task 7 built the modal (key-driven tests pin its
    /// behavior), but no render-layer snapshot pinned its ON-SCREEN content until now. The summary comes
    /// from the real `commit_summary` pipeline (built off a real `apply`-constructed return), not a
    /// fabricated string.
    #[test]
    fn tax_inputs_commit_modal_renders_filing_status_and_summary() {
        use crate::edit::form::{TaxInputsFormState, TaxInputsModalKind, TaxInputsModalState};
        use crate::edit::tax_inputs::commit_summary;
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        for _ in 0..2 {
            apply(
                &mut form.working,
                Edit::AddRow {
                    section: SectionId::W2s,
                    parent: RowAddr::default(),
                },
            )
            .unwrap();
        }
        let ri = form.working.as_ref().unwrap();
        let summary = commit_summary(ri, false);
        form.modal = Some(TaxInputsModalState {
            kind: TaxInputsModalKind::Commit,
            year: 2024,
            filing_status_label: "Single".to_string(),
            summary,
            shadows: false,
        });

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains("Confirm commit for 2024"),
            "the modal title names the year"
        );
        assert!(
            r.contains("filing status: Single"),
            "the modal names the filing status"
        );
        assert!(r.contains("2 W-2(s)"), "the modal names the W-2 count");
        assert!(
            r.contains("Schedule A: no"),
            "the modal names Schedule A absence"
        );
        assert!(
            r.contains("[Enter] commit") && r.contains("[Esc] cancel"),
            "the modal shows its commit/cancel legend"
        );
    }

    /// §10 KAT #7 (★ spec §9A/§13): the renderer NEVER names a `ReturnInputs` field — a rendered field
    /// shows the `FormSpec` `Field.label`, never a struct field name. `AddrStreet`'s struct field is
    /// `header.address_street`; its label is "Street address" — the render must show the label text and
    /// never the raw struct-field name.
    #[test]
    fn tax_inputs_render_uses_field_label_never_a_struct_field_name() {
        use crate::edit::form::{live_sections, TaxInputsFormState};
        use btctax_input_form::{apply, Edit, FieldId, FieldValue, RowAddr, SectionId};
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut form = TaxInputsFormState::fresh(2024);
        apply(
            &mut form.working,
            Edit::SetField {
                id: FieldId::FilingStatus,
                addr: RowAddr::default(),
                value: FieldValue::Choice("Single".into()),
            },
        )
        .unwrap();
        let ri = form.working.as_ref().unwrap();
        form.section_idx = live_sections(ri)
            .iter()
            .position(|s| s.id == SectionId::Address)
            .unwrap();

        let field = btctax_input_form::form_spec()
            .iter()
            .find(|s| s.id == SectionId::Address)
            .unwrap()
            .fields
            .iter()
            .find(|f| f.id == FieldId::AddrStreet)
            .unwrap();
        assert_eq!(
            field.label, "Street address",
            "sanity: the label text the render must show"
        );

        let area = terminal.get_frame().area();
        terminal
            .draw(|f| draw_tax_inputs_form(f, area, &form, None))
            .unwrap();
        let r = flatten(terminal.backend().buffer());
        assert!(
            r.contains(field.label),
            "the rendered field shows the FormSpec `Field.label`"
        );
        assert!(
            !r.contains("address_street"),
            "the renderer must never name the ReturnInputs struct field"
        );
    }

    /// §10 KAT #7, mechanized half: a permanent regression scanner over this file's own non-test source —
    /// every `draw_tax_inputs_*` fn (and its helpers, which all share the `ri: &ReturnInputs` parameter
    /// name in this file) reads ONLY through `form_spec()` accessors (`Field::get`/`Section::kind`), never
    /// a bare `ri.<field>` struct access. Self-checks the scanner FIRST (a planted violation must be
    /// caught; a lookalike identifier — `bri.get()` — must not be a false positive), then scans the real
    /// file. Zero hits today; a future direct-field regression trips this.
    #[test]
    fn tax_inputs_render_never_reads_a_bare_return_inputs_field() {
        fn strip_comment(line: &str) -> &str {
            match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            }
        }
        fn bare_ri_field_hits(content: &str) -> Vec<(usize, String)> {
            let mut hits = Vec::new();
            for (n, raw) in content.lines().enumerate() {
                let line = strip_comment(raw);
                let bytes = line.as_bytes();
                let mut i = 0;
                while let Some(rel) = line[i..].find("ri.") {
                    let idx = i + rel;
                    let prev_is_ident = idx > 0 && {
                        let c = bytes[idx - 1] as char;
                        c.is_ascii_alphanumeric() || c == '_'
                    };
                    if !prev_is_ident {
                        hits.push((n + 1, raw.to_string()));
                        break;
                    }
                    i = idx + 3;
                }
            }
            hits
        }

        // ── Self-check FIRST: the scanner must catch a planted bare `ri.` access, and must NOT flag a
        //    lookalike identifier that merely ENDS in "ri" (`bri.get()`) ──
        let planted = "fn bad(ri: &ReturnInputs) -> Usd {\n    ri.filing_status\n}\n\
                        fn fine(bri: &Thing) {\n    bri.get();\n}\n";
        let self_check = bare_ri_field_hits(planted);
        assert_eq!(
            self_check.len(),
            1,
            "self-check FAILED: scanner must catch exactly the planted bare `ri.` access — gate is broken: {self_check:?}"
        );

        // ── The real scan: this file's own non-test region ──
        let path = {
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR must be set in tests");
            std::path::PathBuf::from(manifest)
                .join("src")
                .join("draw_edit.rs")
        };
        let content = std::fs::read_to_string(&path).expect("must read draw_edit.rs");
        let non_test = match content.find("#[cfg(test)]") {
            Some(pos) => &content[..pos],
            None => content.as_str(),
        };
        let hits = bare_ri_field_hits(non_test);
        assert!(
            hits.is_empty(),
            "draw_edit.rs must never read a bare `ri.<field>` — only `form_spec()` accessors \
             (Field::get / Section::kind); the renderer never names a ReturnInputs field (§9A/§13). \
             Violations: {hits:?}"
        );
    }

    // ── EDITOR marker in Browse screen ───────────────────────────────────────

    #[test]
    fn browse_screen_contains_editor_marker() {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_tui::app::Snapshot;
        use std::collections::BTreeMap;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let snap = Snapshot {
            events: vec![],
            state: btctax_core::state::LedgerState::default(),
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            refused: std::collections::BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
        };

        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snap);
        app.selected_year = 2025;

        terminal.draw(|f| draw(&mut *f, &mut app)).unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect();

        assert!(
            rendered.contains("EDITOR"),
            "Browse screen must contain [EDITOR] marker; rendered:\n{rendered}"
        );
    }

    // ── ★ P-C gate arch I-2: `app.status` is VISIBLE on the Defensive Filing write surface ──────────
    //
    // The DefensiveFiling screen is full-frame: it replaces the Browse footer, the only other place
    // `app.status` renders. In P-B that was cosmetic (read-only); P-C turns the screen into a WRITE
    // surface, so a swallowed status hides every refusal reason — including `on_persist_error`'s
    // CRITICAL unrevertable-residue notice, which `close_all_mutation_surfaces()` leaves on THIS screen
    // and the next keypress destroys. Mutation: stop threading `app.status` into `draw_defensive_filing`
    // and both tests below red.

    fn render_defensive_filing_to_string(status: Option<&str>) -> String {
        use btctax_core::defensive::DefensiveFilingView;
        use std::collections::BTreeSet;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::DefensiveFiling;
        app.defensive_dashboard = Some(crate::defensive_dashboard::DefensiveDashboardState::new(
            DefensiveFilingView {
                candidates: vec![],
                resolve_first: vec![],
                tranches: vec![],
                still_short: vec![],
                flagged_years: BTreeSet::new(),
                safe_harbor_blocked: false,
            },
        ));
        app.status = status.map(|s| s.to_string());
        terminal.draw(|f| draw(&mut *f, &mut app)).unwrap();
        flatten(terminal.backend().buffer())
    }

    #[test]
    fn defensive_filing_screen_renders_a_refusal_status_it_would_otherwise_swallow() {
        let rendered = render_defensive_filing_to_string(Some("declare refused: no-price-window"));
        assert!(
            rendered.contains("declare refused: no-price-window"),
            "★ arch I-2: every refusal/error/outcome routed to `app.status` must be VISIBLE on this \
             (write) screen — the Browse footer that normally renders it is gone here: {rendered}"
        );
        // And with no status the screen is unchanged (no stray empty NOTICE row).
        let clean = render_defensive_filing_to_string(None);
        assert!(
            clean.contains("Defensive Filing"),
            "the screen still renders without a status: {clean}"
        );
    }

    #[test]
    fn defensive_filing_screen_renders_the_critical_unrevertable_residue_notice() {
        // The exact shipped prefix of `on_persist_error`'s ResidueLive status — the one message the
        // filer MUST see before the next keypress clears it.
        let rendered = render_defensive_filing_to_string(Some(
            "CRITICAL: a save failed and could not be reverted",
        ));
        assert!(
            rendered.contains("CRITICAL: a save failed and could not be reverted"),
            "the CRITICAL residue notice must reach the filer on this screen: {rendered}"
        );
    }

    // ── D-7: the stale-latch marker — third specification (DESIGN.md §2.4); the first two were
    // unbuildable: a block-title marker (clipped — Browse's title carries a variable-length vault path,
    // truncated without wrapping) and a Browse mechanism copied from a place (`draw_defensive_filing`)
    // that has no Browse counterpart. From the SECOND keypress onward the marker is the ONLY notice the
    // filer sees on either screen (`main.rs:414` Browse, `:494` DefensiveFiling clear `app.status`
    // before every dispatch), so it must render with `status == None`, independent of `app.status`
    // entirely, and it must name the CONSEQUENCE ("figures below predate your last write"), not the
    // cause. Register precedent: the PSEUDO banner's own "FICTIONAL placeholders — DO NOT FILE".

    /// A search-friendly rendering of a buffer for assertions that may span a WRAPPED row boundary.
    /// `flatten` concatenates a buffer row-major with NO row separator, so a BORDERED widget's own
    /// box-drawing edge glyphs (drawn down every interior row's left/right column) land literally
    /// between two wrapped rows — e.g. `"...Quit││and reopen..."` — which plain whitespace-collapsing
    /// alone does not fix, since `│` is not whitespace. This strips those glyphs per row, joins rows
    /// with a single space, then collapses any remaining whitespace run to one space. Word order and
    /// adjacency are still verified; only the wrap/border machinery's own incidental noise is
    /// suppressed. (Browse's own marker/band rows have no such border, so this is a no-op there beyond
    /// the whitespace collapse.)
    fn searchable(buf: &ratatui::buffer::Buffer) -> String {
        let width = (buf.area().width as usize).max(1);
        let joined = buf
            .content()
            .chunks(width)
            .map(|row| {
                row.iter()
                    .map(|c| c.symbol().chars().next().unwrap_or(' '))
                    .filter(|c| !"│─┌┐└┘┬┴├┤┼".contains(*c))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        joined.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// `render_defensive_filing_to_string`, plus arming the stale latch — a field independent of
    /// `status` (`EditorApp::stale_after_write` vs. `app.status`; bridging that gap is exactly what the
    /// marker exists for). `rows` lets the height-floor KAT drive a very short terminal without
    /// duplicating the fixture. Returns the raw `Buffer` so callers pick `flatten` (exact) or
    /// `searchable` (wrap/border-tolerant) as the assertion needs.
    fn draw_defensive_filing_armed_buffer(
        status: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> ratatui::buffer::Buffer {
        use btctax_core::defensive::DefensiveFilingView;
        use std::collections::BTreeSet;

        let backend = TestBackend::new(cols, rows);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::DefensiveFiling;
        app.defensive_dashboard = Some(crate::defensive_dashboard::DefensiveDashboardState::new(
            DefensiveFilingView {
                candidates: vec![],
                resolve_first: vec![],
                tranches: vec![],
                still_short: vec![],
                flagged_years: BTreeSet::new(),
                safe_harbor_blocked: false,
            },
        ));
        app.stale_after_write = Some("armed-for-test".to_string());
        app.status = status.map(|s| s.to_string());
        terminal.draw(|f| draw(&mut *f, &mut app)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_defensive_filing_to_string_armed_sized(
        status: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> String {
        flatten(&draw_defensive_filing_armed_buffer(status, cols, rows))
    }

    fn render_defensive_filing_to_string_armed(status: Option<&str>, cols: u16) -> String {
        render_defensive_filing_to_string_armed_sized(status, cols, 40)
    }

    /// A Browse render, armed, with an injectable vault path and terminal height — the WITHDRAWN
    /// (draft 2) block-title marker specification died exactly here: Browse's own title carries a
    /// variable-length vault path, and ratatui truncates a title without wrapping. Returns the raw
    /// `Buffer` for the same reason as `draw_defensive_filing_armed_buffer` above.
    fn draw_browse_armed_buffer(
        status: Option<&str>,
        cols: u16,
        rows: u16,
        vault_path: PathBuf,
    ) -> ratatui::buffer::Buffer {
        use btctax_adapters::BundledTaxTables;
        use btctax_cli::CliConfig;
        use btctax_tui::app::Snapshot;
        use std::collections::BTreeMap;

        let backend = TestBackend::new(cols, rows);
        let mut terminal = Terminal::new(backend).unwrap();

        let snap = Snapshot {
            events: vec![],
            state: btctax_core::state::LedgerState::default(),
            cli_config: CliConfig::default(),
            profiles: BTreeMap::new(),
            refused: std::collections::BTreeMap::new(),
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
            bulk_estimated: BTreeMap::new(),
            prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
        };

        let mut app = EditorApp::new(vault_path);
        app.screen = EditorScreen::Browse;
        app.snapshot = Some(snap);
        app.selected_year = 2025;
        app.stale_after_write = Some("armed-for-test".to_string());
        app.status = status.map(|s| s.to_string());
        terminal.draw(|f| draw(&mut *f, &mut app)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_browse_to_string_armed_with_vault_path(
        status: Option<&str>,
        cols: u16,
        vault_path: PathBuf,
    ) -> String {
        flatten(&draw_browse_armed_buffer(status, cols, 40, vault_path))
    }

    fn render_browse_to_string_armed(status: Option<&str>, cols: u16) -> String {
        render_browse_to_string_armed_with_vault_path(
            status,
            cols,
            PathBuf::from("/test/vault.pgp"),
        )
    }

    /// The measured LONGEST composed arm status (DESIGN.md §2.4(a2) — "measure it, do not estimate").
    /// See `DEFENSIVE_NOTICE_LINES`'s own doc comment for exactly how the scenario (the longest of the
    /// three per-tail prefixes plus a representative io-error reason/flush-err pair — they share a root
    /// cause per `arm_stale`'s own doc, so using the same shape for both is not an arbitrary choice) was
    /// chosen: a temporarily instrumented run of `main.rs::arm_stale_status_survives_a_failing_draft_
    /// flush`, printing the REAL `EditorApp::apply_reprojection` → `arm_stale` output.
    ///
    /// ★ fix round 1 Important: earlier this built fact 2's LOSS sentence and fact 3 as HAND-COPIED
    /// literals — only `STALE_FACT_1` was shared with production. That sentence had already been
    /// reworded twice on this branch with no test able to catch a THIRD rewording drifting the two
    /// copies apart (the char-count pin below only pins this fn's OWN literal). Fixed by calling the
    /// same functions/consts `arm_stale` itself calls — `crate::fact2_loss_sentence` and
    /// `crate::FACT_3_QUIT_AND_REOPEN`, alongside the already-shared `crate::STALE_FACT_1` — so only the
    /// `prefix`/`reason`/`flush_err`/`year` SCENARIO CHOICES below are this test's own; the wording
    /// itself cannot drift from what `arm_stale` actually composes.
    fn longest_arm_status() -> String {
        let prefix = "the safe-harbor attest write landed — ";
        let reason = "io: No such file or directory (os error 2)";
        let flush_err = "io: No such file or directory (os error 2)";
        let year = 2024;
        let fact2 = crate::fact2_loss_sentence(year, flush_err);
        format!(
            "{prefix}{fact1} ({reason}). {fact2}{fact3}",
            fact1 = crate::STALE_FACT_1,
            fact3 = crate::FACT_3_QUIT_AND_REOPEN,
        )
    }

    /// Pins the measurement itself (385 chars) so a future edit to `longest_arm_status` — or to
    /// `STALE_FACT_1`'s/`fact2_loss_sentence`'s/`FACT_3_QUIT_AND_REOPEN`'s PRODUCTION wording, in
    /// `main.rs` — cannot silently drift the sizing basis `DEFENSIVE_NOTICE_LINES`'s doc comment cites
    /// without this failing first.
    ///
    /// ★ fix round 1 Important: this is the mechanical proof that the round-1 fix (extracting fact 2's
    /// loss sentence + fact 3 into `main.rs::fact2_loss_sentence`/`FACT_3_QUIT_AND_REOPEN`, shared by
    /// `arm_stale` AND this test) actually closes the drift hole — before the fix, `longest_arm_status`
    /// hand-copied fact 2's sentence as an independent literal, so a PRODUCTION wording change could not
    /// have moved this pin at all.
    ///
    /// Mutation: reworded `main.rs::fact2_loss_sentence` (added "possibly, under any circumstances, "
    /// mid-sentence — a purely production-side change, this file untouched) — RED (385 → 420). Restored
    /// via a `cp` backup (never `git checkout --`) and re-ran — GREEN. Before the round-1 fix, this same
    /// production edit would NOT have moved this test at all.
    #[test]
    fn longest_arm_status_is_the_measured_385_char_worst_case() {
        assert_eq!(longest_arm_status().chars().count(), 385);
    }

    /// From the SECOND keypress onward the marker is the only notice the filer sees — `main.rs:414`
    /// (Browse) and `:494` (DefensiveFiling) clear `app.status` before dispatch. So it must render with
    /// `status == None`, and it must name the CONSEQUENCE, not the cause. Register precedent: the PSEUDO
    /// banner's "FICTIONAL placeholders — DO NOT FILE".
    ///
    /// Mutation 1: reverted the `:109` reservation predicate from `status.is_some() || armed` back to
    /// `status.is_some()` alone (the pre-Task-7 shape) — RED (`d.contains("predate your last write")`
    /// failed: with `status == None` the DefensiveFiling notice rect was never reserved, so the render
    /// guard — unchanged by this mutation — never ran). Restored via a `cp` backup (never
    /// `git checkout --`) and re-ran — GREEN.
    /// Mutation 2: kept the `:109` reservation fixed, but gated `:152-163`'s marker `Line` push on
    /// `if false` instead of `if armed` (i.e. "reserve, but don't render into it" — the SECOND of the
    /// three failed prior specifications this task's brief warns about) — RED, same assertion. Restored
    /// via a `cp` backup (never `git checkout --`) and re-ran — GREEN.
    /// Mutation 3: gated the Browse marker row's insertion (both the `constraints.push` and
    /// `marker_idx`) on `armed && app.status.is_some()` instead of `armed` alone — RED on the Browse half
    /// (`rendered.contains("STALE — figures below...")` failed: with `status == None` no marker row was
    /// inserted at all). Restored via a `cp` backup (never `git checkout --`) and re-ran — GREEN.
    #[test]
    fn the_stale_marker_renders_on_both_screens_with_no_status_and_names_the_consequence() {
        let rendered = render_browse_to_string_armed(None, 80);
        assert!(
            rendered.contains("STALE — figures below predate your last write. DO NOT FILE from them."),
            "Browse renders 8949/Sch D figures off the stale snap; the marker must say so: {rendered}"
        );
        let d = render_defensive_filing_to_string_armed(None, 80);
        assert!(d.contains("predate your last write"), "{d}");
    }

    /// The composed arm status is ~300-350 chars (measured: 385, see `longest_arm_status`);
    /// `DEFENSIVE_NOTICE_LINES` was 3, giving ~228 usable at 80 columns, and the Browse footer is a
    /// single UN-WRAPPED Paragraph that truncates. Without the resize (DefensiveFiling) and the wrapped
    /// band (Browse), fact 3 (the remedy) is invisible at all 27 arming tails.
    ///
    /// `searchable` guards against an incidental artifact of buffer flattening: the DefensiveFiling
    /// screen's own outer `Borders::ALL` block draws a `│` down every interior row's edges, and the
    /// assertion phrase happens to straddle a wrap-row boundary at 80 columns for this measured string —
    /// stripping those border glyphs (and collapsing whitespace) makes the check robust to that, without
    /// weakening what it verifies (word order/adjacency).
    ///
    /// ★ fix round 1 Important: the assertion must target a fragment unique to the STATUS's own tail,
    /// not `"Quit and reopen the vault."` alone — that phrase is ALSO the tail of `STALE_MARKER_
    /// DEFENSIVE` (`:198-199`), which renders on every armed DefensiveFiling frame regardless of
    /// `status`. A prior draft of this KAT asserted the bare phrase and stayed GREEN down to
    /// `DEFENSIVE_NOTICE_LINES == 2` on the DefensiveFiling half (only the Browse half's roomier,
    /// unbordered band geometry was actually exercising the 8-row sizing) — verified by re-running it
    /// at 2 through 8 during this fix round. `"that in-progress entry is lost. Quit and reopen the
    /// vault."` is unique to fact 2's LOSS sentence + fact 3 (the marker never says "in-progress entry")
    /// and is verified lethal at `DEFENSIVE_NOTICE_LINES == 7` (RED) vs. `== 8` (GREEN).
    ///
    /// Mutation: reverted `DEFENSIVE_NOTICE_LINES` from 8 to 3 — RED on both screens (the Browse wrapped
    /// band, sized off the same constant, clipped before fact 3; the DefensiveFiling notice rect was
    /// too short to reach the status line's own tail at all). Restored via a `cp` backup (never
    /// `git checkout --`) and re-ran — GREEN.
    ///
    /// (The Browse FOOTER's own bypass — never echoing `app.status` while armed — is proven separately
    /// by `browse_footer_shows_keybindings_not_a_truncated_status_echo_while_armed` below: reverting it
    /// does NOT turn *this* KAT red, since the wrapped band alone already satisfies "unclipped
    /// somewhere on screen" — the footer bypass exists to stop a truncated, confusing SECOND copy from
    /// also appearing, which is a distinct property this test does not probe.)
    #[test]
    fn the_full_arm_status_renders_unclipped_at_80_columns_on_both_screens() {
        const STATUS_TAIL: &str = "that in-progress entry is lost. Quit and reopen the vault.";
        let arm = longest_arm_status();
        let d = draw_defensive_filing_armed_buffer(Some(&arm), 80, 40);
        assert!(
            searchable(&d).contains(STATUS_TAIL),
            "fact 2's loss sentence + fact 3 must survive, unique from the fixed marker's own tail: {}",
            flatten(&d)
        );
        let b = draw_browse_armed_buffer(Some(&arm), 80, 40, PathBuf::from("/test/vault.pgp"));
        assert!(
            searchable(&b).contains(STATUS_TAIL),
            "fact 2's loss sentence + fact 3 must survive on Browse: {}",
            flatten(&b)
        );
    }

    /// The Browse footer is a single UN-WRAPPED `Paragraph` that TRUNCATES (DESIGN.md §2.4(b): "the
    /// three facts cannot live there"). While armed, the full composed status already has its own
    /// wrapped band (`band_idx`) above the content pane, so the footer must show the keybindings hint
    /// instead of `app.status` — never a second, truncated echo that would read as an incomplete or
    /// different message from the band above it.
    ///
    /// Mutation: dropped the `if armed { keybindings_hint } else { .. }` branch, restoring the
    /// pre-Task-7 `if let Some(status) = app.status.as_deref() { status } else { hint }` shape — RED
    /// (the footer row echoed the arm status' own opening words, truncated, instead of the keybindings
    /// hint). Restored via a `cp` backup (never `git checkout --`) and re-ran — GREEN.
    #[test]
    fn browse_footer_shows_keybindings_not_a_truncated_status_echo_while_armed() {
        let arm = longest_arm_status();
        let buf = draw_browse_armed_buffer(Some(&arm), 80, 40, PathBuf::from("/test/vault.pgp"));
        let width = (buf.area().width as usize).max(1);
        let last_row: String = buf
            .content()
            .chunks(width)
            .last()
            .expect("a non-empty buffer has at least one row")
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            last_row.contains("Tab/Shift-Tab: tab"),
            "the footer must show the keybindings hint while armed, not app.status: {last_row:?}"
        );
        assert!(
            !last_row.contains("safe-harbor"),
            "the footer must NOT echo (even truncated) the arm status while armed: {last_row:?}"
        );
    }

    // ★ fix round 1 minor, INVESTIGATED and left OPEN (not cheap): "Browse has no `.max(1)` floor
    // counterpart, so at height 3 with banner + band its marker row vanishes entirely." True, but
    // root-caused to be a DIFFERENT mechanism than DefensiveFiling's floor, and NOT fixable by adding an
    // analogous clamp. DefensiveFiling's `.max(1)` exists because ITS OWN arithmetic
    // (`DEFENSIVE_NOTICE_LINES.min(inner.height.saturating_sub(1))`) can compute a height of 0 even when
    // ratatui's `Layout::split` had a spare row available (height 3 → `inner.height == 1`, genuinely
    // enough room for 1 row) — the bug was OUR formula discarding real capacity, not ratatui.
    //
    // Browse's height-3 case is different: a standalone `Layout` probe (`ratatui::layout::Layout` at
    // width 80, height 3, constraints `[Length(3) tabs, Length(1) banner, Length(1) marker, Length(1)
    // band, Min(0) content, Length(1) footer]`) shows ratatui's OWN Cassowary solver already shrinks
    // every oversized `Length` request as far as it can — but when the total genuinely exceeds the
    // area (3+1+1+1+1 ≥ 3, tab bar ALONE already needs the entire terminal), the solver drops the
    // "marker" row to height 0 and gives the LATER "band" row height 1 instead, with no dependence on
    // whether the band's own requested constant is clamped first: adding a `.min(available).max(1)`
    // clamp to the band constraint (tried during this fix round, then reverted) produced BYTE-IDENTICAL
    // chunk heights to the un-clamped version at every height probed (3, 5, 6, 7, 8, 9, 10, 12, 14) —
    // ratatui already performs equivalent clamping internally, so the "fix" was an inert no-op. The
    // marker's actual survival at extreme scarcity is a function of ratatui's internal tie-breaking
    // between competing `Length` constraints, not of anything under-clamped in this file. Guaranteeing
    // the marker wins over the banner/band under absolute scarcity would require abandoning
    // `Layout::split`'s automatic solving for a manually priority-ordered height computation (tab
    // bar/marker/footer guaranteed first, banner/band get only what's left) — a real layout redesign,
    // not a cheap fold. Left as an explicitly-flagged residual rather than papered over with dead code.

    /// The WITHDRAWN (draft 2) block-title marker specification died exactly here: Browse's own title
    /// (`" btctax-tui-edit [EDITOR] — {vault_path} "`) carries a variable-length vault path, and ratatui
    /// truncates a title without wrapping — so embedding the marker in the title silently ate it behind
    /// a long enough path. The marker is now its own dedicated `Constraint::Length(1)` row, independent
    /// of the title, so it must survive regardless of how long the path is.
    ///
    /// Mutation: gated the Browse marker row's insertion on `armed && app.status.is_some()` (i.e. tied
    /// it back to `app.status`, the very coupling this task removes) — RED (with `status == None` no
    /// marker row was inserted at all, long path or short). Restored via a `cp` backup (never
    /// `git checkout --`) and re-ran — GREEN.
    #[test]
    fn browse_stale_marker_survives_a_long_vault_path_at_80_columns() {
        let long_path = PathBuf::from(
            "/home/filer/vaults/very/deeply/nested/directory/structure/that/keeps/going/vault.pgp",
        );
        let rendered = render_browse_to_string_armed_with_vault_path(None, 80, long_path);
        assert!(
            rendered.contains("STALE — figures below predate your last write. DO NOT FILE from them."),
            "the marker row must survive a long vault path truncating the title above it: {rendered}"
        );
    }

    /// DESIGN.md §2.4(a): `content_sized_notice_height(..).min(inner.height.saturating_sub(1))` yields 0
    /// at a terminal height of 3 (a `Borders::ALL` block's own top/bottom border leaves
    /// `inner.height == 1`, so `saturating_sub(1) == 0`, regardless of what the content-sized left
    /// operand measures) — silently dropping the marker instead of shrinking it. `.max(1)` floors it at
    /// one row.
    ///
    /// Mutation: dropped the trailing `.max(1)` — RED (`notice_h` became 0, `Layout::split` handed the
    /// notice rect zero rows, and the marker text rendered nowhere — the full-claim assertion below
    /// failed). Restored via a `cp` backup (never `git checkout --`) and re-ran — GREEN.
    ///
    /// ★ fix round 1 minor: asserts the FULL 69-char `STALE_MARKER_CLAIM`, not merely the word "STALE"
    /// — at height 3 the reserved rect is exactly
    /// `content_sized_notice_height(marker_alone, 78).min(0).max(1) == 1` row (the middle `.min(0)`
    /// against `inner.height.saturating_sub(1)`, NOT `.min(1)` — fix round 2 correction: an earlier
    /// draft of this note misstated that middle operand as `1`), and the marker's first wrapped line at
    /// 78 columns is long enough to carry the whole claim before wrapping into the remedy sentence, so
    /// this is not a weaker check than it looks.
    #[test]
    fn defensive_filing_notice_floors_at_one_row_on_a_very_short_terminal() {
        let rendered = render_defensive_filing_to_string_armed_sized(None, 80, 3);
        assert!(
            rendered
                .contains("STALE — figures below predate your last write. DO NOT FILE from them."),
            "the marker must still get SOME rect at height 3, not vanish: {rendered}"
        );
    }

    /// ★ fix round 1 Critical (superseded by fix round 2's `content_sized_notice_height`, but the
    /// scenario this proves is unchanged): the reservation must match what is ACTUALLY drawn, not a
    /// worst-case constant. The armed+`status == None` case renders the marker ALONE, which MEASURES to
    /// ~2 rows — not the full `DEFENSIVE_NOTICE_LINES` (8) CAP, which only applies when marker+status
    /// are STACKED together. Reserving 8 unconditionally (fix round 1's own bug, before content-sizing)
    /// shrank the content pane at 80×24 from 22 rows to 14, and a REAL (armed, pseudo-active) dashboard
    /// can need more than that: D-7's own `safe_journey_view` stub sets `uncomputable =
    /// Some(PSEUDO_ACTIVE_DASHBOARD_NOTICE)` (a ~7-row wrapped notice) and does NOT suppress
    /// `safe_harbor_blocked` (a pure events-scan fact, true for any filer with an in-force safe-harbor
    /// allocation or a pre-2025 tranche — the wizard's core audience), adding a further 2-row note.
    /// Title(2) + blank(1) + notice(7) + blank(1) + safe-harbor-note(2) + blank(1) + the always-last
    /// `[x] export`(1) = 15 rows, against a 14-row pane under either an unconditional-8 reservation (fix
    /// round 1) OR a flat reservation keyed on `status.is_some()` alone (fix round 1's ACTUAL shipped
    /// bug, since `status` was `None` here so that half looked fine until fix round 2 found the OTHER
    /// half) — clipping `[x] export` off a non-scrolling `Paragraph` with no notice text to even hint
    /// `x` still works (DFW-D3/M-5).
    ///
    /// Mutation: reverted `content_sized_notice_height`'s call to the flat `DEFENSIVE_NOTICE_LINES`
    /// (dropped the content-sizing entirely) — RED (`[x] export` absent from the rendered buffer at
    /// 80×24). Restored via a `cp` backup (never `git checkout --`) and re-ran — GREEN.
    #[test]
    fn defensive_filing_marker_alone_reserves_only_2_rows_leaving_room_for_a_full_dashboard() {
        use btctax_core::defensive::DefensiveFilingView;
        use std::collections::BTreeSet;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::DefensiveFiling;
        app.defensive_dashboard = Some(crate::defensive_dashboard::DefensiveDashboardState {
            view: DefensiveFilingView {
                candidates: vec![],
                resolve_first: vec![],
                tranches: vec![],
                still_short: vec![],
                flagged_years: BTreeSet::new(),
                safe_harbor_blocked: true,
            },
            cursor: 0,
            uncomputable: Some(crate::defensive_dashboard::PSEUDO_ACTIVE_DASHBOARD_NOTICE),
        });
        app.stale_after_write = Some("armed-for-test".to_string());
        app.status = None;
        terminal.draw(|f| draw(&mut *f, &mut app)).unwrap();
        let rendered = flatten(terminal.backend().buffer());
        assert!(
            rendered.contains("[x] export"),
            "the always-available export line must survive at 80x24 with only the marker (no status) \
             reserved: {rendered}"
        );
    }

    /// Non-blocking note from the fix round 2 review, addressed here as a real KAT rather than left as
    /// prose: `content_sized_notice_height` measures at the CALLER's own `width`, so a narrower terminal
    /// automatically reserves MORE rows for the SAME marker text, instead of assuming 80 columns. The
    /// constant this replaced (`STALE_MARKER_ONLY_LINES = 2`, fix round 1) was measured only at 80
    /// columns and would have clipped the marker's own remedy sentence ("Quit and reopen the vault.") at
    /// ≤50 columns, where the identical text wraps to MORE than 2 rows — with no reservation growth to
    /// compensate.
    ///
    /// Mutation: hardcoded `content_sized_notice_height`'s `width` parameter to a fixed `78` inside the
    /// function (ignoring the caller's actual, narrower `inner.width`) — RED at 50 columns (the
    /// reservation was sized for 78-column wrapping, too short for the text's ACTUAL 50-column wrap, so
    /// "Quit and reopen the vault." clipped off the bottom of the reserved rect). Restored via a `cp`
    /// backup (never `git checkout --`) and re-ran — GREEN.
    #[test]
    fn narrow_terminal_still_reserves_enough_for_the_full_marker() {
        let buf = draw_defensive_filing_armed_buffer(None, 50, 40);
        assert!(
            searchable(&buf).contains("Quit and reopen the vault."),
            "the marker's own remedy sentence must survive at 50 columns, not just 80: {}",
            flatten(&buf)
        );
    }

    /// ★ fix round 2 Critical — the CORE regression, entirely independent of the armed latch: fix round
    /// 1's flat `DEFENSIVE_NOTICE_LINES` (8) fired on mere `status.is_some()`, and `status` is `Some` on
    /// the ORDINARY UNARMED mainline too — every declare/promote outcome message. Reserving a flat 8
    /// rows for even a one-line outcome message shrank a REAL (unarmed) dashboard's clipping cliff from
    /// ~9 candidates down to ~5 (measured by the round-2 reviewer). This drives the exact repro: 5 real
    /// declare candidates, no latch involved at all, a short ordinary status.
    ///
    /// Mutation: reverted `content_sized_notice_height` to unconditionally return `DEFENSIVE_NOTICE_
    /// LINES` (8) regardless of `lines`' actual content — RED (`[x] export` absent at 80×24 with only 5
    /// candidates and a 25-char status). Restored via a `cp` backup (never `git checkout --`) and
    /// re-ran — GREEN.
    #[test]
    fn ordinary_unarmed_declare_outcome_does_not_clip_export_with_five_candidates() {
        use btctax_core::defensive::discovery::Shortfall;
        use btctax_core::defensive::DefensiveFilingView;
        use std::collections::BTreeSet;
        use time::macros::date;

        let candidates = (0..5)
            .map(|i| Shortfall {
                event: btctax_core::EventId::decision(i),
                wallet: None,
                date: date!(2020 - 06 - 15),
                short_sat: 10_000_000,
                fee_sat: 0,
            })
            .collect();
        let view = DefensiveFilingView {
            candidates,
            resolve_first: vec![],
            tranches: vec![],
            still_short: vec![],
            flagged_years: BTreeSet::new(),
            safe_harbor_blocked: false,
        };

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = EditorApp::new(PathBuf::from("/test/vault.pgp"));
        app.screen = EditorScreen::DefensiveFiling;
        app.defensive_dashboard = Some(crate::defensive_dashboard::DefensiveDashboardState::new(
            view,
        ));
        app.stale_after_write = None; // UNARMED — no latch involved at all
        app.status = Some("committed 2024 as Single".to_string()); // an ordinary, short outcome message
        terminal.draw(|f| draw(&mut *f, &mut app)).unwrap();
        let rendered = flatten(terminal.backend().buffer());
        assert!(
            rendered.contains("[x] export"),
            "an ordinary UNARMED declare/promote outcome message must not clip export with only 5 \
             real candidates: {rendered}"
        );
    }

    /// Extended to Browse's own band for the same reason (fix round 2): a flat `DEFENSIVE_NOTICE_LINES`
    /// would over-reserve for an ordinary short status here too. At a height where the difference is
    /// visible (10 rows: tab bar 3 + marker 1 + footer 1 = 5 fixed, leaving only 5 for band+content), a
    /// flat 8-row band would consume MORE than the entire remaining budget, starving the tab content
    /// pane's `Min(0)` constraint down to 0 rows — "no holdings" (the empty-Holdings-tab text) would
    /// never render at all.
    ///
    /// Mutation: reverted Browse's band constraint from the content-sized `band_h` back to the flat
    /// `Constraint::Length(DEFENSIVE_NOTICE_LINES)` — RED ("no holdings" absent from the rendered
    /// buffer at height 10). Restored via a `cp` backup (never `git checkout --`) and re-ran — GREEN.
    #[test]
    fn browse_band_is_content_sized_leaving_room_for_tab_content_at_a_short_terminal() {
        let buf = draw_browse_armed_buffer(Some("x"), 80, 10, PathBuf::from("/test/vault.pgp"));
        let rendered = flatten(&buf);
        assert!(
            rendered.contains("no holdings"),
            "a 1-char status must not force a flat 8-row band that starves the tab content pane: \
             {rendered}"
        );
    }

    // ── ★ P-C gate arch I-3: the KEYMAP overlay stays in sync with the Browse key handler ───────────

    /// The overlay's own filer-visible text (spans joined per line).
    fn help_overlay_text() -> String {
        help_overlay_lines()
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|sp| sp.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn kat_keymap_overlay_lists_every_browse_char_binding() {
        // Scan the REAL Browse match arms between the KEYMAP-SYNC sentinels in `main.rs`. The in-source
        // "KEEP IN SYNC with KEYMAP overlay" comment alone did not hold: Browse `w` — the Defensive
        // Filing Wizard's ONLY entry point — shipped absent from the overlay, leaving the feature
        // undiscoverable in-product. This test is the mechanical guard.
        let src = include_str!("main.rs");
        let begin = src
            .find("KEYMAP-SYNC-BEGIN")
            .expect("main.rs must carry the KEYMAP-SYNC-BEGIN sentinel");
        let end = src
            .find("KEYMAP-SYNC-END")
            .expect("main.rs must carry the KEYMAP-SYNC-END sentinel");
        assert!(begin < end, "the sentinels must bracket the Browse match");
        let region = &src[begin..end];

        // Collect every `KeyCode::Char('x')` bound in the region (skipping comment lines, so the
        // sentinel's own prose cannot satisfy the check).
        let needle = "KeyCode::Char('";
        let mut bound: Vec<char> = Vec::new();
        for line in region.lines() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let mut rest = code;
            while let Some(i) = rest.find(needle) {
                rest = &rest[i + needle.len()..];
                if let Some(ch) = rest.chars().next() {
                    if !bound.contains(&ch) {
                        bound.push(ch);
                    }
                }
            }
        }
        assert!(
            bound.len() > 20,
            "sanity: the Browse match binds many char keys, found {bound:?}"
        );
        assert!(
            bound.contains(&'w'),
            "sanity: Browse `w` (the wizard entry) must be inside the sentinels"
        );

        let text = help_overlay_text();
        // A key is "listed" when it appears as its own token — `/`-separated groups like `a/A` and
        // `q/Esc` count for both sides.
        let listed = |ch: char| {
            text.split(|c: char| c.is_whitespace() || c == '/')
                .any(|t| t.chars().eq(std::iter::once(ch)))
        };
        let missing: Vec<char> = bound.into_iter().filter(|c| !listed(*c)).collect();
        assert!(
            missing.is_empty(),
            "these Browse keys are bound in main.rs but absent from the KEYMAP overlay \
             (draw_edit::help_overlay_lines) — add a line for each: {missing:?}\noverlay:\n{text}"
        );
    }

    // ── KAT-VOID-SHA-WARNING — SafeHarbor void modal carries the mandatory warning ─
    //
    // Spec D3.1 [M3]: SafeHarborAllocation IS in the void list, but its modal MUST
    // display the conditional-revocability warning (Path B conflict / Path A inert)
    // INCLUDING the permanence line ("a rejected void permanently removes this
    // allocation from this list"). Non-SafeHarbor modals must NOT show it.
    // The cascade consequence note [I1] is ALWAYS present, both cases.

    fn render_void_modal_to_string(modal: &VoidModalState) -> String {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = terminal.get_frame().area();
        terminal.draw(|f| draw_void_modal(f, area, modal)).unwrap();
        terminal
            .backend()
            .buffer()
            .clone()
            .content()
            .iter()
            .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
            .collect()
    }

    #[test]
    fn kat_void_sha_warning_present_for_safe_harbor_absent_otherwise() {
        // SafeHarborAllocation modal: warning MUST be present.
        let sha_modal = VoidModalState {
            target_event_id: btctax_core::EventId::Decision { seq: 7 },
            seq: 7,
            payload_tag: "SafeHarborAllocation",
            target_summary: "alloc 2 lots as_of 2025-01-01".to_string(),
            inner_target: None,
            is_safe_harbor: true,
        };
        let rendered = render_void_modal_to_string(&sha_modal);
        assert!(
            rendered.contains("WARNING: If this allocation is effective (Path B)"),
            "SHA-WARN: SafeHarbor void modal must show the Path-B conditional warning; \
             rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("irrevocable"),
            "SHA-WARN: warning must state irrevocability (§7.4); rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("permanently removes this allocation"),
            "SHA-WARN: warning must carry the [M3] permanence line; rendered:\n{rendered}"
        );
        // The cascade consequence note [I1] is always present.
        assert!(
            rendered.contains("void those too"),
            "SHA-WARN: cascade consequence note must be present; rendered:\n{rendered}"
        );
        assert!(
            rendered.contains("Prior blockers may return"),
            "SHA-WARN: returned-blocker consequence note must be present; rendered:\n{rendered}"
        );

        // Non-SafeHarbor modal (MethodElection): warning must be ABSENT.
        let me_modal = VoidModalState {
            target_event_id: btctax_core::EventId::Decision { seq: 3 },
            seq: 3,
            payload_tag: "MethodElection",
            target_summary: "method=Fifo from 2024-01-01".to_string(),
            inner_target: None,
            is_safe_harbor: false,
        };
        let rendered_me = render_void_modal_to_string(&me_modal);
        assert!(
            !rendered_me.contains("WARNING: If this allocation is effective"),
            "SHA-WARN: non-SafeHarbor void modal must NOT show the SafeHarbor warning; \
             rendered:\n{rendered_me}"
        );
        // Cascade note still present for non-SafeHarbor.
        assert!(
            rendered_me.contains("void those too"),
            "SHA-WARN: cascade note must be present for non-SafeHarbor too; \
             rendered:\n{rendered_me}"
        );
    }
}
