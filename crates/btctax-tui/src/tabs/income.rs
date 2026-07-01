//! Income tab — renders year-filtered income records as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use rust_decimal::Decimal;

use super::tags::income_kind_tag;

/// Render the Income tab into `area`.
pub fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Income ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };

    let year = app.selected_year;
    let sat_div = Decimal::from(100_000_000i64);

    let mut total_sat: i64 = 0;
    let mut total_fmv = Decimal::ZERO;

    let mut rows: Vec<Row> = snap
        .state
        .income_recognized
        .iter()
        .filter(|rec| rec.recognized_at.year() == year)
        .map(|rec| {
            total_sat += rec.sat;
            total_fmv += rec.usd_fmv;

            let btc = Decimal::from(rec.sat) / sat_div;
            Row::new(vec![
                Cell::from(rec.recognized_at.to_string()),
                Cell::from(income_kind_tag(rec.kind)),
                Cell::from(if rec.business { "yes" } else { "no" }),
                Cell::from(format!("{:.8}", btc)),
                Cell::from(format!("{:.2}", rec.usd_fmv)),
            ])
        })
        .collect();

    if rows.is_empty() {
        let p = Paragraph::new(format!("no income in {year}"))
            .block(Block::default().title(" Income ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    // TOTAL row
    let total_btc = Decimal::from(total_sat) / sat_div;
    rows.push(Row::new(vec![
        Cell::from("TOTAL"),
        Cell::from(""),
        Cell::from(""),
        Cell::from(format!("{:.8}", total_btc)),
        Cell::from(format!("{:.2}", total_fmv)),
    ]));

    let header = Row::new(vec![
        Cell::from("Recognized"),
        Cell::from("Kind"),
        Cell::from("Business"),
        Cell::from("BTC"),
        Cell::from("USD FMV"),
    ]);

    let widths = vec![
        Constraint::Percentage(20),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Income — {year} "))
                .borders(Borders::ALL),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, area, &mut app.income_state);
}
