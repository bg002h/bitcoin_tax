//! Disposals tab — renders year-filtered disposals as a ratatui Table.
//!
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Snapshot};
use crate::sort::{self, Dir, ViewSort};
use btctax_cli::render::wallet_label;
use btctax_core::state::{Disposal, DisposalLeg};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use std::cmp::Ordering;

use super::tags::{term_rank, term_tag};
use super::utils::{sat_to_btc, MIN_ROWS_FOR_TOTALS};

/// Number of rendered columns (Disposed · Acquired · BTC · Proceeds · Basis · Gain · Term · Wallet).
pub const COLUMN_COUNT: usize = 8;

/// Column header titles, left→right; the sort/cursor decorations are added by [`sort::header_row`].
pub const HEADERS: [&str; COLUMN_COUNT] = [
    "Disposed", "Acquired", "BTC", "Proceeds", "Basis", "Gain", "Term", "Wallet",
];

/// Default sort: `Disposed` (column 0) ascending — the primary date column [R0-M-1].
pub const DEFAULT_SORT: ViewSort = ViewSort::new(0, Dir::Asc);

/// A single rendered row's typed source: the parent disposal, one of its legs, and the leg index
/// within that disposal (used by the total-order tie-break) — Disposals render PER-LEG [R0-M-2].
type LegRef<'a> = (&'a Disposal, &'a DisposalLeg, usize);

/// Column-keyed comparator over the typed leg fields (display-only; borrows never mutate state).
fn cmp_col(col: usize, a: &LegRef, b: &LegRef) -> Ordering {
    match col {
        0 => a.0.disposed_at.cmp(&b.0.disposed_at),
        1 => a.1.acquired_at.cmp(&b.1.acquired_at),
        2 => a.1.sat.cmp(&b.1.sat),
        3 => a.1.proceeds.cmp(&b.1.proceeds),
        4 => a.1.basis.cmp(&b.1.basis),
        5 => a.1.gain.cmp(&b.1.gain),
        6 => term_rank(a.1.term).cmp(&term_rank(b.1.term)), // short < long
        7 => a.1.wallet.cmp(&b.1.wallet),
        _ => Ordering::Equal,
    }
}

/// Build the typed, borrowed, SORTED per-leg working set for `year`. Flatten each in-year disposal
/// into `(disposal, leg, leg_idx)`, then apply the focused column + direction with a TOTAL-order
/// tie-break `(disposed_at, EventId, leg index)` via a stable sort [R0-M-2]. Never mutates state.
pub(crate) fn sorted_legs(disposals: &[Disposal], year: i32, sort: ViewSort) -> Vec<LegRef<'_>> {
    let mut items: Vec<LegRef> = Vec::new();
    for disposal in disposals {
        if disposal.disposed_at.year() != year {
            continue;
        }
        for (idx, leg) in disposal.legs.iter().enumerate() {
            items.push((disposal, leg, idx));
        }
    }
    sort::stable_sort_by(
        &mut items,
        sort.dir,
        |a, b| cmp_col(sort.col, a, b),
        |a, b| {
            a.0.disposed_at
                .cmp(&b.0.disposed_at)
                .then_with(|| a.0.event.cmp(&b.0.event))
                .then_with(|| a.2.cmp(&b.2))
        },
    );
    items
}

/// App-free renderer for the Disposals tab.
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
    // [R0-I2] Build the typed, borrowed, SORTED per-leg working set FIRST, then format it into rows.
    let sorted = sorted_legs(&snap.state.disposals, year, sort);

    if sorted.is_empty() {
        let p = Paragraph::new(format!("no disposals in {year}"))
            .block(Block::default().title(" Disposals ").borders(Borders::ALL));
        frame.render_widget(p, area);
        return;
    }

    let mut rows: Vec<Row> = Vec::with_capacity(sorted.len());
    let mut total_sat: i64 = 0;
    let mut total_proceeds = rust_decimal::Decimal::ZERO;
    let mut total_basis = rust_decimal::Decimal::ZERO;
    let mut total_gain = rust_decimal::Decimal::ZERO;
    // Cycle-5: any flagged row shown → surface the `[est]` legend in the block title.
    let mut any_estimated = false;

    for (disposal, leg, _idx) in &sorted {
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

        total_sat += leg.sat;
        total_proceeds += leg.proceeds;
        total_basis += leg.basis;
        total_gain += leg.gain;

        let btc = sat_to_btc(leg.sat);
        rows.push(Row::new(vec![
            Cell::from(disposed_str),
            Cell::from(leg.acquired_at.to_string()),
            Cell::from(format!("{:.8}", btc)),
            Cell::from(format!("{:.2}", leg.proceeds)),
            Cell::from(format!("{:.2}", leg.basis)),
            Cell::from(format!("{:.2}", leg.gain)),
            Cell::from(term_tag(leg.term)),
            Cell::from(wallet_label(&leg.wallet)),
        ]));
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

    let header = sort::header_row(&HEADERS, sort, cursor);

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
        app.disposals_sort,
        app.disposals_cursor,
        &mut app.disposals_state,
    );
}

#[cfg(test)]
mod sort_tests {
    use super::*;
    use btctax_core::event::{BasisSource, DisposeKind};
    use btctax_core::identity::{EventId, LotId, Source, SourceRef, WalletId};
    use btctax_core::state::Term;
    use rust_decimal::Decimal;

    fn date(y: i32, m: u8, d: u8) -> btctax_core::TaxDate {
        time::Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
    }

    fn event(tag: &str) -> EventId {
        EventId::import(Source::Coinbase, SourceRef::new(tag))
    }

    fn leg(
        sat: i64,
        proceeds: i64,
        basis: i64,
        gain: i64,
        term: Term,
        acq: (i32, u8, u8),
    ) -> DisposalLeg {
        DisposalLeg {
            lot_id: LotId {
                origin_event_id: event("lot"),
                split_sequence: 0,
            },
            sat,
            proceeds: Decimal::from(proceeds),
            basis: Decimal::from(basis),
            gain: Decimal::from(gain),
            term,
            basis_source: BasisSource::ExchangeProvided,
            gift_zone: None,
            acquired_at: date(acq.0, acq.1, acq.2),
            wallet: WalletId::Exchange {
                provider: "coinbase".into(),
                account: "main".into(),
            },
            pseudo: false,
        }
    }

    fn disposal(tag: &str, disposed: (i32, u8, u8), legs: Vec<DisposalLeg>) -> Disposal {
        Disposal {
            event: event(tag),
            kind: DisposeKind::Sell,
            disposed_at: date(disposed.0, disposed.1, disposed.2),
            legs,
            fee_mini_disposition: false,
        }
    }

    /// Distinct single-leg disposals in 2025 so each column proves a different order.
    fn fixture() -> Vec<Disposal> {
        vec![
            // tag, disposed, (sat, proceeds, basis, gain, term)
            disposal(
                "A",
                (2025, 5, 1),
                vec![leg(300, 900, 100, 800, Term::LongTerm, (2020, 1, 1))],
            ),
            disposal(
                "B",
                (2025, 1, 1),
                vec![leg(100, 500, 400, 100, Term::ShortTerm, (2024, 1, 1))],
            ),
            disposal(
                "C",
                (2025, 3, 1),
                vec![leg(200, 700, 250, 450, Term::LongTerm, (2022, 6, 1))],
            ),
        ]
    }

    fn tags<'a>(v: &[LegRef<'a>]) -> Vec<&'a str> {
        v.iter()
            .map(|(d, _, _)| match &d.event {
                EventId::Import { source_ref, .. } => source_ref.0.as_str(),
                _ => "?",
            })
            .collect()
    }

    #[test]
    fn default_sort_is_disposed_ascending() {
        let d = fixture();
        let out = sorted_legs(&d, 2025, DEFAULT_SORT);
        // Disposed asc: B(Jan) < C(Mar) < A(May).
        assert_eq!(tags(&out), vec!["B", "C", "A"]);
    }

    #[test]
    fn sort_by_proceeds_asc_desc() {
        let d = fixture();
        let asc = sorted_legs(&d, 2025, ViewSort::new(3, Dir::Asc));
        assert_eq!(tags(&asc), vec!["B", "C", "A"], "proceeds 500<700<900");
        let desc = sorted_legs(&d, 2025, ViewSort::new(3, Dir::Desc));
        assert_eq!(tags(&desc), vec!["A", "C", "B"]);
    }

    #[test]
    fn sort_by_gain_and_term() {
        let d = fixture();
        let gain = sorted_legs(&d, 2025, ViewSort::new(5, Dir::Asc));
        assert_eq!(tags(&gain), vec!["B", "C", "A"], "gain 100<450<800");
        // Term col: short < long. B is short; A,C long. B first; A,C tie-break by (date,event,idx):
        // A disposed May, C disposed Mar → tie-break (disposed_at asc) → C before A.
        let term = sorted_legs(&d, 2025, ViewSort::new(6, Dir::Asc));
        assert_eq!(tags(&term), vec!["B", "C", "A"]);
    }

    #[test]
    fn year_filter_excludes_other_years() {
        let mut d = fixture();
        d.push(disposal(
            "OLD",
            (2024, 5, 1),
            vec![leg(999, 1, 1, 0, Term::LongTerm, (2019, 1, 1))],
        ));
        let out = sorted_legs(&d, 2025, DEFAULT_SORT);
        assert_eq!(tags(&out), vec!["B", "C", "A"], "2024 disposal excluded");
    }

    #[test]
    fn per_leg_ties_keep_leg_index_order() {
        // ONE disposal with 3 legs that share (disposed_at, event); sorting by a column where all
        // legs are EQUAL (same sat) must keep the original leg-index order — total & stable [R0-M-2].
        let legs = vec![
            leg(100, 10, 1, 9, Term::LongTerm, (2020, 1, 1)), // idx 0
            leg(100, 20, 2, 18, Term::LongTerm, (2020, 1, 1)), // idx 1
            leg(100, 30, 3, 27, Term::LongTerm, (2020, 1, 1)), // idx 2
        ];
        let d = vec![disposal("X", (2025, 6, 1), legs)];
        // Sort by BTC (col 2) where all sat are equal → tie-break decides → leg index order.
        let asc = sorted_legs(&d, 2025, ViewSort::new(2, Dir::Asc));
        let idxs: Vec<usize> = asc.iter().map(|(_, _, i)| *i).collect();
        assert_eq!(
            idxs,
            vec![0, 1, 2],
            "equal-key legs keep leg-index order (asc)"
        );
        let desc = sorted_legs(&d, 2025, ViewSort::new(2, Dir::Desc));
        let idxs_d: Vec<usize> = desc.iter().map(|(_, _, i)| *i).collect();
        assert_eq!(
            idxs_d,
            vec![0, 1, 2],
            "leg-index tie-break stays ascending even under a descending key sort"
        );
    }
}
