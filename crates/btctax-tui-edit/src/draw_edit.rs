//! Terminal rendering for the editor.
//!
//! "writes ONLY append-only events + typed side-table upserts via `edit/persist.rs`,
//! each behind an explicit payload-showing confirmation; the vault file only via
//! `Vault::save`'s atomic path."
//!
//! Delegates to the viewer's App-free `tabs::*::render` functions for the Browse screen;
//! uses `btctax_tui::draw::draw_unlock_screen` with EDITOR-branded strings for the
//! Unlock screen. This module performs no writes.

use crate::edit::form::{MutationModalState, ProfileFormState, FIELD_LABELS};
use crate::editor::{EditorApp, EditorScreen};
use btctax_tui::app::Tab;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
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
/// Form and modal overlays are drawn on top.
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

    // ── Overlays (drawn AFTER content so they appear on top) ─────────────────
    if let Some(form) = app.profile_form.as_ref() {
        draw_profile_form(frame, area, form);
    }
    if let Some(modal) = app.mutation_modal.as_ref() {
        draw_mutation_modal(frame, area, modal);
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

/// Compute a centered `Rect` of the given dimensions within `area`.
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
            tables: BundledTaxTables::load(),
            donation_details: BTreeMap::new(),
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
}
