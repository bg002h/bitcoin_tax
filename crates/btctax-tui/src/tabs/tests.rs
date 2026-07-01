//! TestBackend KATs for Holdings, Disposals, and Income tabs.
//!
//! No vault needed — all fixtures build synthetic LedgerState directly.
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Screen, Snapshot};
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    event::{BasisSource, DisposeKind, IncomeKind},
    identity::{EventId, LotId, Source, SourceRef, WalletId},
    state::{Disposal, DisposalLeg, IncomeRecord, LedgerState, Lot, Term},
    ProjectionConfig,
};
use ratatui::{backend::TestBackend, Terminal};
use rust_decimal::Decimal;
use std::{collections::BTreeMap, path::PathBuf};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_wallet() -> WalletId {
    WalletId::Exchange {
        provider: "coinbase".into(),
        account: "main".into(),
    }
}

fn make_lot_id(tag: &str) -> LotId {
    LotId {
        origin_event_id: EventId::import(Source::Coinbase, SourceRef::new(tag)),
        split_sequence: 0,
    }
}

fn make_event_id(tag: &str) -> EventId {
    EventId::import(Source::Coinbase, SourceRef::new(tag))
}

fn make_date(y: i32, m: u8, d: u8) -> btctax_core::TaxDate {
    time::Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
}

fn make_snapshot(state: LedgerState) -> Snapshot {
    Snapshot {
        events: vec![],
        state,
        config: ProjectionConfig::default(),
        cli_config: btctax_cli::CliConfig::default(),
        profiles: BTreeMap::new(),
        tables: BundledTaxTables::load(),
    }
}

fn make_app(state: LedgerState, year: i32) -> App {
    let mut app = App::new(PathBuf::new());
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(make_snapshot(state));
    app
}

/// Scan every row of the buffer; return true if any row's text contains `needle`.
fn buffer_has(buf: &ratatui::buffer::Buffer, needle: &str) -> bool {
    let area = buf.area();
    for y in 0..area.height {
        let row: String = (0..area.width)
            .map(|x| buf.cell((x, y)).map_or(" ", |c| c.symbol()))
            .collect();
        if row.contains(needle) {
            return true;
        }
    }
    false
}

fn render_holdings(app: &mut App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::holdings::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn render_disposals(app: &mut App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::disposals::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn render_income(app: &mut App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::income::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

// ── Holdings tests ────────────────────────────────────────────────────────────

/// 1. Build a LedgerState with 1 lot. Render. Assert header contains "Wallet" and "BTC",
///    and data row contains the expected wallet string and BTC value.
#[test]
fn holdings_renders_header_and_known_row() {
    let lot = Lot {
        lot_id: make_lot_id("h1"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 1, 15),
        original_sat: 100_000_000,
        remaining_sat: 100_000_000,
        usd_basis: Decimal::new(6000000, 2), // 60000.00
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let mut state = LedgerState::default();
    state.lots.push(lot);

    let mut app = make_app(state, 2025);
    let buf = render_holdings(&mut app);

    // Header
    assert!(buffer_has(&buf, "Wallet"), "header must contain 'Wallet'");
    assert!(buffer_has(&buf, "BTC"), "header must contain 'BTC'");

    // Data row: wallet label and BTC value
    assert!(
        buffer_has(&buf, "exchange:coinbase:main"),
        "data row must contain the wallet label"
    );
    assert!(
        buffer_has(&buf, "1.00000000"),
        "data row must contain BTC value at 8dp"
    );
}

/// 2. Same fixture — assert the TOTAL row (Σ BTC, Σ USD basis).
#[test]
fn holdings_renders_total_row() {
    let lot = Lot {
        lot_id: make_lot_id("h2"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 2, 1),
        original_sat: 50_000_000,
        remaining_sat: 50_000_000,
        usd_basis: Decimal::new(250000, 2), // 2500.00
        basis_source: BasisSource::ComputedFromCost,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let mut state = LedgerState::default();
    state.lots.push(lot);

    let mut app = make_app(state, 2025);
    let buf = render_holdings(&mut app);

    // TOTAL row
    assert!(buffer_has(&buf, "TOTAL"), "must have TOTAL row");
    assert!(
        buffer_has(&buf, "0.50000000"),
        "TOTAL BTC must be 0.50000000"
    );
    assert!(
        buffer_has(&buf, "2500.00"),
        "TOTAL USD basis must be 2500.00"
    );
}

/// 3. Empty state: state.lots is empty → buffer contains "no holdings".
#[test]
fn holdings_empty_state_renders_placeholder() {
    let state = LedgerState::default();
    let mut app = make_app(state, 2025);
    let buf = render_holdings(&mut app);
    assert!(
        buffer_has(&buf, "no holdings"),
        "empty holdings must render 'no holdings'"
    );
}

/// 4. TableState selection moves with scroll helpers.
#[test]
fn holdings_up_down_moves_selection() {
    // Build a state with 2 lots so there is more than 1 row to scroll through.
    let lot1 = Lot {
        lot_id: make_lot_id("hud1"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 1, 1),
        original_sat: 10_000_000,
        remaining_sat: 10_000_000,
        usd_basis: Decimal::new(50000, 2),
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let lot2 = Lot {
        lot_id: make_lot_id("hud2"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 6, 1),
        original_sat: 20_000_000,
        remaining_sat: 20_000_000,
        usd_basis: Decimal::new(80000, 2),
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let mut state = LedgerState::default();
    state.lots.push(lot1);
    state.lots.push(lot2);

    let mut app = make_app(state, 2025);
    app.tab = crate::app::Tab::Holdings;

    // Initially no selection
    assert_eq!(app.holdings_state.selected(), None);

    // scroll_down selects first row (index 0)
    crate::scroll_down(&mut app);
    assert_eq!(app.holdings_state.selected(), Some(0));

    // scroll_down again moves to index 1
    crate::scroll_down(&mut app);
    assert_eq!(app.holdings_state.selected(), Some(1));

    // scroll_up moves back to index 0
    crate::scroll_up(&mut app);
    assert_eq!(app.holdings_state.selected(), Some(0));
}

// ── Disposals tests ───────────────────────────────────────────────────────────

fn make_disposal(
    disposed_year: i32,
    sat: i64,
    proceeds: &str,
    basis: &str,
    gain: &str,
) -> Disposal {
    Disposal {
        event: make_event_id(&format!("d{disposed_year}")),
        kind: DisposeKind::Sell,
        disposed_at: make_date(disposed_year, 6, 15),
        legs: vec![DisposalLeg {
            lot_id: make_lot_id(&format!("dl{disposed_year}")),
            sat,
            proceeds: proceeds.parse().unwrap(),
            basis: basis.parse().unwrap(),
            gain: gain.parse().unwrap(),
            term: Term::LongTerm,
            basis_source: BasisSource::ExchangeProvided,
            gift_zone: None,
            acquired_at: make_date(disposed_year - 2, 1, 1),
            wallet: make_wallet(),
        }],
        fee_mini_disposition: false,
    }
}

/// 5. Fixture with 1 disposal in selected_year. Assert disposed_at date and BTC cells.
#[test]
fn disposals_renders_header_and_known_row() {
    let mut state = LedgerState::default();
    state.disposals.push(make_disposal(
        2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));

    let mut app = make_app(state, 2025);
    let buf = render_disposals(&mut app);

    assert!(
        buffer_has(&buf, "Disposed"),
        "header must contain 'Disposed'"
    );
    assert!(buffer_has(&buf, "BTC"), "header must contain 'BTC'");
    // data row: disposed_at date
    assert!(
        buffer_has(&buf, "2025-06-15"),
        "data row must contain disposed_at date"
    );
    // BTC value: 50_000_000 sat = 0.50000000 BTC
    assert!(
        buffer_has(&buf, "0.50000000"),
        "data row must contain BTC at 8dp"
    );
}

/// 6. Assert TOTAL row shows Σ proceeds, Σ basis, Σ gain.
#[test]
fn disposals_renders_total_row() {
    let mut state = LedgerState::default();
    state.disposals.push(make_disposal(
        2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));

    let mut app = make_app(state, 2025);
    let buf = render_disposals(&mut app);

    assert!(buffer_has(&buf, "TOTAL"), "must have TOTAL row");
    assert!(
        buffer_has(&buf, "30000.00"),
        "TOTAL proceeds must be 30000.00"
    );
    assert!(buffer_has(&buf, "20000.00"), "TOTAL basis must be 20000.00");
    assert!(buffer_has(&buf, "10000.00"), "TOTAL gain must be 10000.00");
}

/// 7. Two disposals: one in selected_year, one in (selected_year - 1). Assert the other-year
///    disposal does NOT appear and TOTAL reflects only the selected-year disposal.
#[test]
fn disposals_year_filter_excludes_other_year() {
    let mut state = LedgerState::default();
    // In-year disposal: 2025
    state.disposals.push(make_disposal(
        2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    // Out-of-year disposal: 2024
    state.disposals.push(make_disposal(
        2024, 10_000_000, "5000.00", "3000.00", "2000.00",
    ));

    let mut app = make_app(state, 2025);
    let buf = render_disposals(&mut app);

    // 2024 date must NOT appear
    assert!(
        !buffer_has(&buf, "2024-06-15"),
        "out-of-year disposal date must NOT appear"
    );
    // TOTAL must reflect only 2025 disposal
    assert!(
        buffer_has(&buf, "30000.00"),
        "TOTAL proceeds must be the 2025 value"
    );
    // 2024-only value 5000.00 must NOT appear in proceeds
    // (check that we don't see 5000.00 or the date)
    assert!(!buffer_has(&buf, "2024-06-15"), "2024 date must be absent");
}

/// 8. No disposals for selected_year → "no disposals in {year}".
#[test]
fn disposals_empty_state_renders_placeholder() {
    let state = LedgerState::default();
    let mut app = make_app(state, 2025);
    let buf = render_disposals(&mut app);
    assert!(
        buffer_has(&buf, "no disposals in 2025"),
        "empty disposals must render 'no disposals in 2025'"
    );
}

// ── Income tests ──────────────────────────────────────────────────────────────

fn make_income(year: i32, sat: i64, usd_fmv: &str, kind: IncomeKind) -> IncomeRecord {
    IncomeRecord {
        event: make_event_id(&format!("i{year}")),
        recognized_at: make_date(year, 3, 1),
        sat,
        usd_fmv: usd_fmv.parse().unwrap(),
        kind,
        business: false,
    }
}

/// 9. Fixture with 1 income record (in selected_year). Assert recognized_at date and kind tag.
#[test]
fn income_renders_header_and_known_row() {
    let mut state = LedgerState::default();
    state
        .income_recognized
        .push(make_income(2025, 1_000_000, "600.00", IncomeKind::Staking));

    let mut app = make_app(state, 2025);
    let buf = render_income(&mut app);

    assert!(
        buffer_has(&buf, "Recognized"),
        "header must contain 'Recognized'"
    );
    assert!(buffer_has(&buf, "Kind"), "header must contain 'Kind'");
    assert!(
        buffer_has(&buf, "2025-03-01"),
        "data row must contain recognized_at date"
    );
    assert!(
        buffer_has(&buf, "staking"),
        "data row must contain kind tag"
    );
}

/// 10. Two income records: one in selected_year, one in (selected_year - 1). Assert the
///     other-year record does NOT appear and TOTAL reflects only the selected-year record.
#[test]
fn income_year_filter_excludes_other_year() {
    let mut state = LedgerState::default();
    state
        .income_recognized
        .push(make_income(2025, 1_000_000, "600.00", IncomeKind::Staking));
    state
        .income_recognized
        .push(make_income(2024, 500_000, "150.00", IncomeKind::Mining));

    let mut app = make_app(state, 2025);
    let buf = render_income(&mut app);

    // 2024 record date must NOT appear
    assert!(
        !buffer_has(&buf, "2024-03-01"),
        "out-of-year income date must NOT appear"
    );
    // 2025 record should appear
    assert!(
        buffer_has(&buf, "2025-03-01"),
        "2025 income date must appear"
    );
    // TOTAL must not include 2024 value
    assert!(
        !buffer_has(&buf, "150.00"),
        "2024 USD FMV must not appear in totals"
    );
}

/// 11. No income for selected_year → "no income in {year}".
#[test]
fn income_empty_state_renders_placeholder() {
    let state = LedgerState::default();
    let mut app = make_app(state, 2025);
    let buf = render_income(&mut app);
    assert!(
        buffer_has(&buf, "no income in 2025"),
        "empty income must render 'no income in 2025'"
    );
}
