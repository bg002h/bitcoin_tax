//! Income tab — renders year-filtered income records as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Snapshot};
use crate::sort::{self, Dir, ViewSort};
use btctax_core::state::IncomeRecord;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use rust_decimal::Decimal;
use std::cmp::Ordering;

use super::tags::{income_kind_rank, income_kind_tag};
use super::utils::{sat_to_btc, MIN_ROWS_FOR_TOTALS};

/// Number of rendered columns (Recognized · Kind · Business · BTC · USD FMV). NO wallet column —
/// `IncomeRecord` has no wallet field [R0-I3].
pub const COLUMN_COUNT: usize = 5;

/// Column header titles, left→right; the sort/cursor decorations are added by [`sort::header_row`].
pub const HEADERS: [&str; COLUMN_COUNT] = ["Recognized", "Kind", "Business", "BTC", "USD FMV"];

/// Default sort: `Recognized` (column 0) ascending — the primary date column [R0-M-1].
pub const DEFAULT_SORT: ViewSort = ViewSort::new(0, Dir::Asc);

/// Column-keyed comparator over the typed `IncomeRecord` fields (display-only; never mutates state).
fn cmp_col(col: usize, a: &IncomeRecord, b: &IncomeRecord) -> Ordering {
    match col {
        0 => a.recognized_at.cmp(&b.recognized_at),
        1 => income_kind_rank(a.kind).cmp(&income_kind_rank(b.kind)),
        2 => a.business.cmp(&b.business), // bool: false < true
        3 => a.sat.cmp(&b.sat),
        4 => a.usd_fmv.cmp(&b.usd_fmv),
        _ => Ordering::Equal,
    }
}

/// Build the typed, borrowed, SORTED working set of income records for `year`. Total order: the
/// focused column + direction, tie-broken on the record's stable id (`event`) via a stable sort
/// [R0-M-2]. Never mutates state (sorts borrows).
pub(crate) fn sorted_income(
    income: &[IncomeRecord],
    year: i32,
    sort: ViewSort,
) -> Vec<&IncomeRecord> {
    let mut items: Vec<&IncomeRecord> = income
        .iter()
        .filter(|rec| rec.recognized_at.year() == year)
        .collect();
    sort::stable_sort_by(
        &mut items,
        sort.dir,
        |a, b| cmp_col(sort.col, a, b),
        |a, b| a.event.cmp(&b.event),
    );
    items
}

/// App-free renderer for the Income tab.
///
/// Extracted from `draw` so the editor crate can call this directly with its own
/// `Snapshot`, `year`, and `TableState`, without holding an `App`.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    snap: &Snapshot,
    year: i32,
    sort: ViewSort,
    cursor: usize,
    table_state: &mut TableState,
) {
    // [R0-I2] Sort the TYPED, borrowed working set FIRST, then format the sorted records into rows.
    let sorted = sorted_income(&snap.state.income_recognized, year, sort);

    let mut total_sat: i64 = 0;
    let mut total_fmv = Decimal::ZERO;

    let rows: Vec<Row> = sorted
        .iter()
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

    // TOTAL row — a FROZEN footer (pinned, non-scrolling, non-selectable) built via
    // `Table::footer` below: Σ BTC (from Σ sat) and Σ income (FMV recognized), both sums.
    let total_btc = sat_to_btc(total_sat);
    let total_row = Row::new(vec![
        Cell::from("TOTAL"),
        Cell::from(""),
        Cell::from(""),
        Cell::from(format!("{:.8}", total_btc)),
        Cell::from(format!("{:.2}", total_fmv)),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let header = sort::header_row(&HEADERS, sort, cursor);

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

    // Height gate: only pin the frozen totals footer when the area is tall enough; otherwise
    // give the vertical space to data rows.
    let table = if area.height >= MIN_ROWS_FOR_TOTALS {
        table.footer(total_row)
    } else {
        table
    };

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
    render(
        frame,
        area,
        snap,
        app.selected_year,
        app.income_sort,
        app.income_cursor,
        &mut app.income_state,
    );
}

#[cfg(test)]
mod sort_tests {
    use super::*;
    use btctax_core::event::IncomeKind;
    use btctax_core::identity::{EventId, Source, SourceRef};

    fn date(y: i32, m: u8, d: u8) -> btctax_core::TaxDate {
        time::Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    fn rec(
        tag: &str,
        at: (i32, u8, u8),
        kind: IncomeKind,
        business: bool,
        sat: i64,
        fmv: i64,
    ) -> IncomeRecord {
        IncomeRecord {
            event: EventId::import(Source::Coinbase, SourceRef::new(tag)),
            recognized_at: date(at.0, at.1, at.2),
            sat,
            usd_fmv: Decimal::from(fmv),
            kind,
            business,
            pseudo: false,
        }
    }

    fn fixture() -> Vec<IncomeRecord> {
        vec![
            rec("A", (2025, 5, 1), IncomeKind::Reward, true, 300, 900), // late, kind=Reward(4), biz, hi
            rec("B", (2025, 1, 1), IncomeKind::Mining, false, 100, 500), // early, Mining(0), no, lo
            rec("C", (2025, 3, 1), IncomeKind::Staking, false, 200, 700), // mid, Staking(1), no, mid
        ]
    }

    fn tags<'a>(v: &[&'a IncomeRecord]) -> Vec<&'a str> {
        v.iter()
            .map(|r| match &r.event {
                EventId::Import { source_ref, .. } => source_ref.0.as_str(),
                _ => "?",
            })
            .collect()
    }

    #[test]
    fn default_sort_is_recognized_ascending() {
        let f = fixture();
        let out = sorted_income(&f, 2025, DEFAULT_SORT);
        assert_eq!(tags(&out), vec!["B", "C", "A"], "recognized asc");
    }

    #[test]
    fn sort_by_kind_and_fmv_asc_desc() {
        let f = fixture();
        let kind = sorted_income(&f, 2025, ViewSort::new(1, Dir::Asc));
        assert_eq!(
            tags(&kind),
            vec!["B", "C", "A"],
            "kind Mining<Staking<Reward"
        );
        let fmv_desc = sorted_income(&f, 2025, ViewSort::new(4, Dir::Desc));
        assert_eq!(tags(&fmv_desc), vec!["A", "C", "B"], "FMV desc 900>700>500");
    }

    #[test]
    fn sort_by_business_bool() {
        let f = fixture();
        // business asc: false < true. B,C are false (tie-break by event: B<C), A true last.
        let out = sorted_income(&f, 2025, ViewSort::new(2, Dir::Asc));
        assert_eq!(tags(&out), vec!["B", "C", "A"]);
    }

    #[test]
    fn tie_break_is_event_id_and_year_filtered() {
        // Same recognized date + kind → tie-break by event id (stable, total).
        let f = vec![
            rec("z", (2025, 2, 2), IncomeKind::Mining, false, 1, 1),
            rec("x", (2025, 2, 2), IncomeKind::Mining, false, 1, 1),
            rec("y", (2025, 2, 2), IncomeKind::Mining, false, 1, 1),
            rec("old", (2024, 2, 2), IncomeKind::Mining, false, 1, 1),
        ];
        let asc = sorted_income(&f, 2025, DEFAULT_SORT);
        assert_eq!(
            tags(&asc),
            vec!["x", "y", "z"],
            "event-id tie-break, 2024 excluded"
        );
        let desc = sorted_income(&f, 2025, ViewSort::new(0, Dir::Desc));
        assert_eq!(
            tags(&desc),
            vec!["x", "y", "z"],
            "tie-break stays ascending under a descending key sort"
        );
    }
}
