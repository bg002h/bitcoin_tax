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
    amount_label, income_kind_display, ClassifyInboundModalState, ClassifyInboundStep,
    InboundVariant, MutationModalState, OutflowKind, ProfileFormState, ReclassifyIncomeFlowState,
    ReclassifyIncomeModalState, ReclassifyIncomeStep, ReclassifyOutflowModalState,
    ReclassifyOutflowStep, SetFmvFlowState, SetFmvModalState, SetFmvStep, FIELD_LABELS,
};
use crate::editor::{EditorApp, EditorScreen};
use btctax_core::{DisposeKind, InboundClass, OutflowClass};
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
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close   q: swallowed")
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
                InboundVariant::Income => "> Income         GiftReceived",
                InboundVariant::GiftReceived => "  Income       > GiftReceived",
            };
            let c = format!(
                "  target: {target}\n\
                 \n\
                   Select variant (Tab to cycle, Enter to confirm):\n\
                 \n\
                 {variant_str}\n\
                 \n\
                 \n  Esc: back to list   q: swallowed",
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
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move focus   q: swallowed",
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
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move focus   q: swallowed",
                target = item.blocker_event.canonical(),
            );
            (" Classify Inbound — GiftReceived  [EDITOR] ", c)
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
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close   q: swallowed")
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
                 \n  Esc: back to list   q: swallowed",
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
                 \n  Enter: validate   Esc: back to picker   ↑/↓/Tab: move   q: swallowed",
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
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close   q: swallowed")
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
         \n  Enter: validate   Esc: back to list   ↑/↓: move focus   q: swallowed",
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
    let footer = Paragraph::new("↑/↓: scroll   Enter: select   Esc: close   q: swallowed")
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
         \n  Enter: validate   Esc: back to list   q: swallowed",
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
