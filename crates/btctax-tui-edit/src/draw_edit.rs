//! Terminal rendering for the editor.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! Delegates to the viewer's App-free `tabs::*::render` functions for the Browse screen;
//! uses `btctax_tui::draw::draw_unlock_screen` with EDITOR-branded strings for the
//! Unlock screen. This module performs no writes.

use crate::editor::{EditorApp, EditorScreen};
use btctax_tui::app::Tab;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

/// Top-level draw entry point — dispatches on `EditorScreen`.
pub fn draw(frame: &mut Frame, app: &mut EditorApp) {
    match app.screen {
        EditorScreen::Unlock => draw_unlock(frame, app),
        EditorScreen::Locked => draw_locked(frame),
        EditorScreen::Browse => draw_browse(frame, app),
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

/// Render the browse screen: EDITOR-marked tab bar + viewer tab content + EDITOR footer.
fn draw_browse(frame: &mut Frame, app: &mut EditorApp) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content pane
            Constraint::Length(1), // footer keybindings
        ])
        .split(area);

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

    // ── Content pane — delegate to viewer's App-free tab renderers ────────────
    let content_area = chunks[1];
    if let Some(snap) = app.snapshot.as_ref() {
        let year = app.selected_year;
        match app.tab {
            Tab::Holdings => btctax_tui::tabs::holdings::render(
                frame,
                content_area,
                snap,
                year,
                &mut app.holdings_state,
            ),
            Tab::Disposals => btctax_tui::tabs::disposals::render(
                frame,
                content_area,
                snap,
                year,
                &mut app.disposals_state,
            ),
            Tab::Income => btctax_tui::tabs::income::render(
                frame,
                content_area,
                snap,
                year,
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
        // Should not normally be reached (Browse is only set on SessionOpenOutcome::Success).
        let p = Paragraph::new("Snapshot unavailable — please restart the editor.")
            .alignment(Alignment::Center);
        frame.render_widget(p, content_area);
    }

    // ── Footer: status or keybindings ─────────────────────────────────────────
    let footer_text = if let Some(status) = app.status.as_deref() {
        status.to_string()
    } else {
        "Tab/Shift-Tab: switch tab   ←/→: change year   ↑/↓ j/k: scroll   \
         PgUp/PgDn: page   g/G: top/bottom   p: edit tax profile   q/Esc: quit   [EDITOR]"
            .to_string()
    };
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}
