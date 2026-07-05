//! Holdings tab — renders current lots as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Snapshot};
use crate::sort::{self, Dir, ViewSort};
use btctax_cli::render::wallet_label;
use btctax_core::state::Lot;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use rust_decimal::Decimal;
use std::cmp::Ordering;

use super::tags::{basis_source_rank, basis_source_tag};
use super::utils::{sat_to_btc, MIN_ROWS_FOR_TOTALS};

/// Number of rendered columns (Wallet · Acquired · BTC · USD Basis · Source · Pending).
pub const COLUMN_COUNT: usize = 6;

/// Column header titles, left→right; the sort/cursor decorations are added by [`sort::header_row`].
pub const HEADERS: [&str; COLUMN_COUNT] = [
    "Wallet",
    "Acquired",
    "BTC",
    "USD Basis",
    "Source",
    "Pending",
];

/// Default sort: `Acquired` (column 1) ascending — the primary date column [R0-M-1].
pub const DEFAULT_SORT: ViewSort = ViewSort::new(1, Dir::Asc);

/// Column-keyed comparator over the typed `Lot` fields (display-only; borrows never mutate state).
fn cmp_col(col: usize, a: &Lot, b: &Lot) -> Ordering {
    match col {
        0 => a.wallet.cmp(&b.wallet), // WalletId Ord = (provider, account) for exchanges
        1 => a.acquired_at.cmp(&b.acquired_at),
        2 => a.remaining_sat.cmp(&b.remaining_sat),
        3 => a.usd_basis.cmp(&b.usd_basis),
        4 => basis_source_rank(a.basis_source).cmp(&basis_source_rank(b.basis_source)),
        5 => a.basis_pending.cmp(&b.basis_pending), // bool: false < true
        _ => Ordering::Equal,
    }
}

/// Build the typed, borrowed, SORTED working set of lots for display. Total order: the focused
/// column + direction, tie-broken on the lot's stable id (`lot_id`) via a stable sort [R0-M-2].
/// Never mutates `lots` (sorts borrows).
pub(crate) fn sorted_lots(lots: &[Lot], sort: ViewSort) -> Vec<&Lot> {
    let mut items: Vec<&Lot> = lots.iter().collect();
    sort::stable_sort_by(
        &mut items,
        sort.dir,
        |a, b| cmp_col(sort.col, a, b),
        |a, b| a.lot_id.cmp(&b.lot_id),
    );
    items
}

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
    sort: ViewSort,
    cursor: usize,
    table_state: &mut TableState,
) {
    let lots = &snap.state.lots;

    if lots.is_empty() {
        let p = Paragraph::new("no holdings")
            .block(Block::default().title(" Holdings ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    // [R0-I2] Sort the TYPED, borrowed working set FIRST, then format the sorted lots into rows.
    let sorted = sorted_lots(lots, sort);

    let mut total_sat: i64 = 0;
    let mut total_basis = Decimal::ZERO;

    let rows: Vec<Row> = sorted
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

    let header = sort::header_row(&HEADERS, sort, cursor);

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
        app.holdings_sort,
        app.holdings_cursor,
        &mut app.holdings_state,
    );
}

#[cfg(test)]
mod sort_tests {
    use super::*;
    use btctax_core::event::BasisSource;
    use btctax_core::identity::{EventId, LotId, Source, SourceRef, WalletId};

    fn wallet(provider: &str) -> WalletId {
        WalletId::Exchange {
            provider: provider.into(),
            account: "main".into(),
        }
    }

    fn lot_id(tag: &str) -> LotId {
        LotId {
            origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(tag)),
            split_sequence: 0,
        }
    }

    fn date(y: i32, m: u8, d: u8) -> btctax_core::TaxDate {
        time::Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    fn lot(tag: &str, w: &str, acq: (i32, u8, u8), sat: i64, basis: i64) -> Lot {
        Lot {
            lot_id: lot_id(tag),
            wallet: wallet(w),
            acquired_at: date(acq.0, acq.1, acq.2),
            original_sat: sat,
            remaining_sat: sat,
            usd_basis: Decimal::from(basis),
            basis_source: BasisSource::ExchangeProvided,
            dual_loss_basis: None,
            donor_acquired_at: None,
            basis_pending: false,
            pseudo: false,
        }
    }

    /// Distinct fixture: acquired dates, BTC, basis, and wallet all disagree so each column proves
    /// a DIFFERENT order.
    fn fixture() -> Vec<Lot> {
        vec![
            lot("a", "kraken", (2024, 3, 1), 300, 900), // later date, mid sat, high basis, W=kraken
            lot("b", "coinbase", (2022, 1, 1), 100, 500), // earliest date, low sat, low basis, W=coinbase
            lot("c", "gemini", (2023, 6, 1), 200, 700),   // mid date, mid, mid, W=gemini
        ]
    }

    fn tags<'a>(v: &[&'a Lot]) -> Vec<&'a str> {
        v.iter()
            .map(|l| match &l.lot_id.origin_event_id {
                EventId::Import { source_ref, .. } => source_ref.0.as_str(),
                _ => "?",
            })
            .collect()
    }

    #[test]
    fn default_sort_is_acquired_ascending() {
        let lots = fixture();
        let out = sorted_lots(&lots, DEFAULT_SORT);
        // Acquired asc: b(2022) < c(2023) < a(2024).
        assert_eq!(tags(&out), vec!["b", "c", "a"]);
    }

    #[test]
    fn sort_by_btc_asc_desc() {
        let lots = fixture();
        let asc = sorted_lots(&lots, ViewSort::new(2, Dir::Asc));
        assert_eq!(tags(&asc), vec!["b", "c", "a"], "BTC asc: 100<200<300");
        let desc = sorted_lots(&lots, ViewSort::new(2, Dir::Desc));
        assert_eq!(tags(&desc), vec!["a", "c", "b"], "BTC desc");
    }

    #[test]
    fn sort_by_usd_basis_asc_desc() {
        let lots = fixture();
        let asc = sorted_lots(&lots, ViewSort::new(3, Dir::Asc));
        assert_eq!(tags(&asc), vec!["b", "c", "a"], "basis asc: 500<700<900");
        let desc = sorted_lots(&lots, ViewSort::new(3, Dir::Desc));
        assert_eq!(tags(&desc), vec!["a", "c", "b"]);
    }

    #[test]
    fn sort_by_wallet_asc() {
        let lots = fixture();
        let asc = sorted_lots(&lots, ViewSort::new(0, Dir::Asc));
        // Provider order: coinbase < gemini < kraken → b, c, a.
        assert_eq!(tags(&asc), vec!["b", "c", "a"]);
    }

    #[test]
    fn sort_is_total_and_stable_on_equal_keys() {
        // Three lots share the SAME acquired date; the tie-break is the stable lot_id.
        // lot_id origin tags "z", "y", "x" → LotId Ord sorts them x < y < z regardless of
        // insertion order or direction of the (equal) primary key.
        let same = date(2024, 1, 1);
        let mk = |tag: &str| {
            let mut l = lot(tag, "coinbase", (2024, 1, 1), 100, 100);
            l.acquired_at = same;
            l
        };
        let lots = vec![mk("z"), mk("y"), mk("x")];
        let asc = sorted_lots(&lots, ViewSort::new(1, Dir::Asc));
        assert_eq!(tags(&asc), vec!["x", "y", "z"], "tie-break asc by lot_id");
        let desc = sorted_lots(&lots, ViewSort::new(1, Dir::Desc));
        assert_eq!(
            tags(&desc),
            vec!["x", "y", "z"],
            "tie-break stays ascending even when the key sort is descending"
        );
    }
}
