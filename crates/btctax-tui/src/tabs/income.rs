//! Income tab — renders year-filtered income records as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Snapshot};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use rust_decimal::Decimal;

use super::tags::income_kind_tag;
use super::utils::sat_to_btc;

/// App-free renderer for the Income tab.
///
/// Extracted from `draw` so the editor crate can call this directly with its own
/// `Snapshot`, `year`, and `TableState`, without holding an `App`.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    snap: &Snapshot,
    year: i32,
    table_state: &mut TableState,
) {
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

            let btc = sat_to_btc(rec.sat);
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

    // TOTAL row — rendered but NEVER selectable (selection capped at data_rows-1 in scroll helpers).
    let total_btc = sat_to_btc(total_sat);
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

    frame.render_stateful_widget(table, area, table_state);
}

/// Render the Income tab into `area`.
///
/// Thin `pub(crate)` wrapper over [`render`]: handles the `snapshot == None` placeholder
/// exactly as before, then delegates to the App-free `render` fn.
/// Call sites in `draw.rs` and `tabs/tests.rs` call this wrapper — unchanged.
pub(crate) fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Income ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };
    render(frame, area, snap, app.selected_year, &mut app.income_state);
}
