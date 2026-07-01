//! Holdings tab — renders current lots as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::App;
use btctax_cli::render::wallet_label;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use rust_decimal::Decimal;

use super::tags::basis_source_tag;

/// Render the Holdings tab into `area`.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Holdings ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };

    let lots = &snap.state.lots;

    if lots.is_empty() {
        let p = Paragraph::new("no holdings")
            .block(Block::default().title(" Holdings ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    let sat_div = Decimal::from(100_000_000i64);

    let mut total_sat: i64 = 0;
    let mut total_basis = Decimal::ZERO;

    let mut rows: Vec<Row> = lots
        .iter()
        .map(|lot| {
            total_sat += lot.remaining_sat;
            total_basis += lot.usd_basis;

            let btc = Decimal::from(lot.remaining_sat) / sat_div;
            Row::new(vec![
                Cell::from(wallet_label(&lot.wallet)),
                Cell::from(lot.acquired_at.to_string()),
                Cell::from(format!("{:.8}", btc)),
                Cell::from(format!("{:.2}", lot.usd_basis)),
                Cell::from(basis_source_tag(lot.basis_source)),
                Cell::from(if lot.basis_pending { "pending" } else { "" }),
            ])
        })
        .collect();

    // TOTAL row
    let total_btc = Decimal::from(total_sat) / sat_div;
    rows.push(Row::new(vec![
        Cell::from("TOTAL"),
        Cell::from(""),
        Cell::from(format!("{:.8}", total_btc)),
        Cell::from(format!("{:.2}", total_basis)),
        Cell::from(""),
        Cell::from(""),
    ]));

    let header = Row::new(vec![
        Cell::from("Wallet"),
        Cell::from("Acquired"),
        Cell::from("BTC"),
        Cell::from("USD Basis"),
        Cell::from("Source"),
        Cell::from("Pending"),
    ]);

    let widths = vec![
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(17),
        Constraint::Percentage(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title(" Holdings ").borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, area, &mut app.holdings_state);
}
