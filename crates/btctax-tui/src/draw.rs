//! Terminal rendering: dispatches on `Screen`/`Tab` and renders placeholder widgets.
//! Real tab content is implemented in Tasks 3–4.

use crate::app::{App, Screen, Tab};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

/// Top-level draw entry point.  Called once per iteration of the run loop.
pub fn draw(frame: &mut Frame, app: &App) {
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
         Enter: unlock    Esc/q: quit"
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

fn draw_viewer(frame: &mut Frame, app: &App) {
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
        .block(Block::default().borders(Borders::ALL).title(" btctax-tui "))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs_widget, chunks[0]);

    // ── Content placeholder (real content in Tasks 3–4) ──────────────────────
    let content_block = Block::default()
        .title(format!(" {} ", app.tab.title()))
        .borders(Borders::ALL);
    let placeholder = Paragraph::new(format!("{} — data loads in Tasks 3–4", app.tab.title()))
        .block(content_block);
    frame.render_widget(placeholder, chunks[1]);

    // ── Footer ───────────────────────────────────────────────────────────────
    let footer = Paragraph::new(
        "Tab/Shift-Tab: switch tab   ←/→: change year   ↑/↓ j/k: scroll   \
         g/G: top/bottom   r: refresh   q/Esc: quit   ?: help",
    )
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}
