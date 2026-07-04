//! Holdings tab — renders current lots as a ratatui Table.
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
use rust_decimal::Decimal;

use super::tags::basis_source_tag;
use super::utils::{sat_to_btc, MIN_ROWS_FOR_TOTALS};

/// App-free renderer for the Holdings tab.
///
/// Extracted from `draw` so the editor crate can call this directly with its own
/// `Snapshot` and `TableState`, without holding an `App`.
///
/// Note: `year` is accepted for API consistency with other stateful tab renderers
/// but is not used — the Holdings tab shows all lots regardless of year.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    snap: &Snapshot,
    _year: i32,
    table_state: &mut TableState,
) {
    let lots = &snap.state.lots;

    if lots.is_empty() {
        let p = Paragraph::new("no holdings")
            .block(Block::default().title(" Holdings ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    let mut total_sat: i64 = 0;
    let mut total_basis = Decimal::ZERO;

    let rows: Vec<Row> = lots
        .iter()
        .map(|lot| {
            total_sat += lot.remaining_sat;
            total_basis += lot.usd_basis;

            let btc = sat_to_btc(lot.remaining_sat);
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

    // TOTAL row — a FROZEN footer (pinned, non-scrolling, non-selectable) built via
    // `Table::footer` below. The basis cell is the WEIGHTED-AVERAGE cost $/BTC of the stack
    // (`round_cents((Σ usd_basis × 1e8) / Σ sat)`, multiply-first) — an unrealized gain cannot
    // be summed, so a total-basis-$ pairs with nothing. Guard `Σ sat == 0` → `—` (reachable only
    // with a non-empty `lots` whose remaining_sat sum to 0; empty `lots` short-circuits above).
    let total_btc = sat_to_btc(total_sat);
    let avg_basis_cell = if total_sat == 0 {
        "—".to_string()
    } else {
        let avg = btctax_core::conventions::round_cents(
            (total_basis * Decimal::from(100_000_000i64)) / Decimal::from(total_sat),
        );
        format!("{:.2}", avg)
    };
    let total_row = Row::new(vec![
        Cell::from("TOTAL"),
        Cell::from(""),
        Cell::from(format!("{:.8}", total_btc)),
        Cell::from(avg_basis_cell),
        Cell::from(""),
        Cell::from(""),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

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

    // Height gate: only pin the frozen totals footer when the area is tall enough; otherwise
    // give the vertical space to data rows.
    let table = if area.height >= MIN_ROWS_FOR_TOTALS {
        table.footer(total_row)
    } else {
        table
    };

    frame.render_stateful_widget(table, area, table_state);
}

/// Render the Holdings tab into `area`.
///
/// Thin `pub(crate)` wrapper over [`render`]: handles the `snapshot == None` placeholder
/// exactly as before, then delegates to the App-free `render` fn.
/// Call sites in `draw.rs` and `tabs/tests.rs` call this wrapper — unchanged.
pub(crate) fn draw(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        let p = Paragraph::new("no snapshot loaded")
            .block(Block::default().title(" Holdings ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    };
    render(
        frame,
        area,
        snap,
        app.selected_year,
        &mut app.holdings_state,
    );
}
