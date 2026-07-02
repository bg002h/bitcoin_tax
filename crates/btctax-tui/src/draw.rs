//! Terminal rendering: dispatches on `Screen`/`Tab` and delegates to per-tab modules.

use crate::app::{App, Screen, Tab};
use crate::tabs::{compliance, disposals, forms, holdings, income, tax};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

/// Top-level draw entry point.  Called once per iteration of the run loop.
pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::Unlock => draw_unlock(frame, app),
        Screen::Locked => draw_locked(frame),
        Screen::Viewer => draw_viewer(frame, app),
    }
}

fn draw_unlock(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Outer block provides the border and title
    let block = Block::default()
        .title(" btctax-tui — Unlock Vault ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Masked passphrase: one ● per character in the buffer (NEVER render actual chars)
    let bullet_count = app.unlock.buffer.chars().count();
    let masked = "●".repeat(bullet_count);

    // Vault path line (shows which vault is being opened)
    let vault_line = format!("Vault: {}", app.vault_path.display());

    // Error line (empty string when no error, so layout stays stable)
    let error_line = app
        .unlock
        .error
        .as_deref()
        .map(|e| format!("  ✗ {e}"))
        .unwrap_or_default();

    let content = format!(
        "\n\
         {vault_line}\n\
         \n\
         Passphrase:  {masked}\n\
         \n\
         offline · local · read-only · PGP-encrypted\n\
         \n\
         {error_line}\n\
         \n\
         Enter: unlock    Esc: quit"
    );

    let msg = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(msg, inner);
}

fn draw_locked(frame: &mut Frame) {
    let area = frame.area();
    let block = Block::default()
        .title(" btctax-tui — Vault Locked ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let msg = Paragraph::new(
        "Vault is in use by another process (the CLI or another viewer).\n\
         Close it and retry.\n\n\
         r  retry   q  quit",
    )
    .alignment(Alignment::Center);
    frame.render_widget(msg, inner);
}

fn draw_viewer(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content pane
            Constraint::Length(1), // footer keybindings
        ])
        .split(area);

    // ── Tab bar ──────────────────────────────────────────────────────────────
    let tab_titles: Vec<&str> = Tab::ALL.iter().map(|t| t.title()).collect();
    let tabs_widget = Tabs::new(tab_titles)
        .select(app.tab.index())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" btctax-tui — {} ", app.vault_path.display())),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs_widget, chunks[0]);

    // ── Content pane ─────────────────────────────────────────────────────────
    let content_area = chunks[1];
    match app.tab {
        Tab::Holdings => holdings::draw(frame, content_area, app),
        Tab::Disposals => disposals::draw(frame, content_area, app),
        Tab::Income => income::draw(frame, content_area, app),
        Tab::Tax => tax::draw(frame, content_area, app),
        Tab::Forms => forms::draw(frame, content_area, app),
        Tab::Compliance => compliance::draw(frame, content_area, app),
    }

    // ── Footer ───────────────────────────────────────────────────────────────
    // When export_status is set, show it; otherwise show the normal keybindings hint.
    let footer_text = if let Some(status) = app.export_status.as_deref() {
        status.to_string()
    } else {
        "Tab/Shift-Tab: switch tab   ←/→: change year   ↑/↓ j/k: scroll   \
         PgUp/PgDn: page   g/G: top/bottom   e: export CSVs   q/Esc: quit"
            .to_string()
    };
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);

    // ── Export confirmation modal overlay ─────────────────────────────────────
    if let Some(modal) = app.export_modal.as_ref() {
        draw_export_modal(frame, area, modal);
    }
}

/// Render the export confirmation modal centered over `area`.
///
/// Clears the underlying content first, then draws the modal box with the export details.
/// This is drawn AFTER the tab content so it appears on top.
fn draw_export_modal(frame: &mut Frame, area: Rect, modal: &crate::export::ExportConfirmState) {
    // Build modal content text.
    let file_lines: String = modal.files.iter().map(|f| format!("    {f}\n")).collect();

    let content = format!(
        "  Output directory:\n  {}\n\n  Files to write:\n{}\n  The vault is never written.\n  \
         Exported files contain your tax data and are\n  owner-only (0o600 on Unix).\n\n  \
         [Enter] Confirm     [Esc] Cancel — writes nothing",
        modal.out_dir.display(),
        file_lines.trim_end(),
    );

    // Compute a centered rect: ~62 wide, height based on content line count.
    let modal_width: u16 = 64;
    let content_lines = content.lines().count() as u16 + 2; // +2 for block border
    let modal_height = content_lines.max(10);

    let modal_rect = centered_rect(modal_width, modal_height, area);

    // Clear the background area first (so the modal appears on top of tab content).
    frame.render_widget(Clear, modal_rect);

    let p = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!(" Export form CSVs for {} ", modal.year))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, modal_rect);
}

/// Compute a horizontally and vertically centered `Rect` of the given dimensions within `area`.
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
