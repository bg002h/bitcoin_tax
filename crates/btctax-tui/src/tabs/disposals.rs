//! Disposals tab — renders year-filtered disposals as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Snapshot};
use btctax_cli::render::wallet_label;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use super::tags::term_tag;
use super::utils::{sat_to_btc, MIN_ROWS_FOR_TOTALS};

/// App-free renderer for the Disposals tab.
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
    // Flatten disposals for the selected year into legs.
    let mut rows: Vec<Row> = Vec::new();
    let mut total_sat: i64 = 0;
    let mut total_proceeds = rust_decimal::Decimal::ZERO;
    let mut total_basis = rust_decimal::Decimal::ZERO;
    let mut total_gain = rust_decimal::Decimal::ZERO;
    // Cycle-5: any flagged row shown → surface the `[est]` legend in the block title.
    let mut any_estimated = false;

    for disposal in &snap.state.disposals {
        if disposal.disposed_at.year() != year {
            continue;
        }
        // [est] marker: this disposal's proceeds were derived from an auto-FMV by the bulk-reclassify-
        // outflow path (join the side-table against `Disposal.event`). The EXACT fold-computed
        // proceeds/basis/gain are rendered as-is — the marker only annotates provenance.
        let estimated = snap.bulk_estimated.contains_key(&disposal.event);
        any_estimated |= estimated;
        let disposed_str = if estimated {
            format!("{} [est]", disposal.disposed_at)
        } else {
            disposal.disposed_at.to_string()
        };
        for leg in &disposal.legs {
            total_sat += leg.sat;
            total_proceeds += leg.proceeds;
            total_basis += leg.basis;
            total_gain += leg.gain;

            let btc = sat_to_btc(leg.sat);
            rows.push(Row::new(vec![
                Cell::from(disposed_str.clone()),
                Cell::from(leg.acquired_at.to_string()),
                Cell::from(format!("{:.8}", btc)),
                Cell::from(format!("{:.2}", leg.proceeds)),
                Cell::from(format!("{:.2}", leg.basis)),
                Cell::from(format!("{:.2}", leg.gain)),
                Cell::from(term_tag(leg.term)),
                Cell::from(wallet_label(&leg.wallet)),
            ]));
        }
    }

    if rows.is_empty() {
        let p = Paragraph::new(format!("no disposals in {year}"))
            .block(Block::default().title(" Disposals ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    // TOTAL row — a FROZEN footer (pinned, non-scrolling, non-selectable) built via
    // `Table::footer` below. Σ BTC is now surfaced; basis stays SUMMED so the row is additive
    // (`Σ gain = Σ proceeds − Σ basis`).
    let total_btc = sat_to_btc(total_sat);
    let total_row = Row::new(vec![
        Cell::from("TOTAL"),
        Cell::from(""),
        Cell::from(format!("{:.8}", total_btc)),
        Cell::from(format!("{:.2}", total_proceeds)),
        Cell::from(format!("{:.2}", total_basis)),
        Cell::from(format!("{:.2}", total_gain)),
        Cell::from(""),
        Cell::from(""),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let header = Row::new(vec![
        Cell::from("Disposed"),
        Cell::from("Acquired"),
        Cell::from("BTC"),
        Cell::from("Proceeds"),
        Cell::from("Basis"),
        Cell::from("Gain"),
        Cell::from("Term"),
        Cell::from("Wallet"),
    ]);

    let widths = vec![
        Constraint::Percentage(13),
        Constraint::Percentage(13),
        Constraint::Percentage(13),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
        Constraint::Percentage(8),
        Constraint::Percentage(17),
    ];

    // Legend note only when a flagged row is actually shown (Cycle-5 `[est]` provenance marker).
    let title = if any_estimated {
        format!(" Disposals — {year}   [est] = estimated FMV proceeds ")
    } else {
        format!(" Disposals — {year} ")
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    // Height gate: only pin the frozen totals footer when the area is tall enough; otherwise
    // give the vertical space to data rows.
    let table = if area.height >= MIN_ROWS_FOR_TOTALS {
        table.footer(total_row)
    } else {
        table
    };

    frame.render_stateful_widget(table, area, table_state);
}

/// Render the Disposals tab into `area`.
///
/// Thin `pub(crate)` wrapper over [`render`]: handles the `snapshot == None` placeholder
/// exactly as before, then delegates to the App-free `render` fn.
/// Call sites in `draw.rs` and `tabs/tests.rs` call this wrapper — unchanged.
pub(crate) fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Disposals ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };
    render(
        frame,
        area,
        snap,
        app.selected_year,
        &mut app.disposals_state,
    );
}
