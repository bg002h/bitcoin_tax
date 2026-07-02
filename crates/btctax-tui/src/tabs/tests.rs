//! TestBackend KATs for Holdings, Disposals, Income, Tax, Forms, and Compliance tabs.
//!
//! No vault needed — all fixtures build synthetic LedgerState directly.
//! STRICTLY READ-ONLY: no Session, no persistence, no mutations.

use crate::app::{App, Screen, Snapshot};
use btctax_adapters::BundledTaxTables;
use btctax_core::{
    event::{BasisSource, DisposeKind, IncomeKind},
    identity::{EventId, LotId, Source, SourceRef, WalletId},
    state::{BlockerKind, Disposal, DisposalLeg, IncomeRecord, LedgerState, Lot, Severity, Term},
    Carryforward, FilingStatus, TaxProfile,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
        cli_config: btctax_cli::CliConfig::default(),
        profiles: BTreeMap::new(),
        tables: BundledTaxTables::load(),
        donation_details: BTreeMap::new(),
    }
}

fn make_app(state: LedgerState, year: i32) -> App {
    let mut app = App::new(PathBuf::new());
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(make_snapshot(state));
    app
}

/// Simulate a key press event.
fn press(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
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

/// Render the full viewer frame (tab bar + content + footer) using the top-level draw entry.
fn render_viewer(app: &mut App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            crate::draw::draw(f, app);
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

/// 2. Two-lot fixture — assert TOTAL row (Σ BTC, Σ USD basis) shows the sum,
///    which differs from every individual lot's value.
#[test]
fn holdings_renders_total_row() {
    // lot A: 50M sat = 0.50000000 BTC, $2500.00
    let lot_a = Lot {
        lot_id: make_lot_id("h2a"),
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
    // lot B: 25M sat = 0.25000000 BTC, $1500.00
    let lot_b = Lot {
        lot_id: make_lot_id("h2b"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 6, 1),
        original_sat: 25_000_000,
        remaining_sat: 25_000_000,
        usd_basis: Decimal::new(150000, 2), // 1500.00
        basis_source: BasisSource::ComputedFromCost,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let mut state = LedgerState::default();
    state.lots.push(lot_a);
    state.lots.push(lot_b);

    let mut app = make_app(state, 2025);
    let buf = render_holdings(&mut app);

    assert!(buffer_has(&buf, "TOTAL"), "must have TOTAL row");
    // Individual row values still appear in data rows
    assert!(
        buffer_has(&buf, "0.50000000"),
        "first lot BTC must be present"
    );
    assert!(
        buffer_has(&buf, "0.25000000"),
        "second lot BTC must be present"
    );
    // TOTAL must show the summed values: 0.75000000 BTC, $4000.00
    // (0.75 ≠ 0.50 ≠ 0.25; 4000.00 ≠ 2500.00 ≠ 1500.00 — broken sum would fail)
    assert!(
        buffer_has(&buf, "0.75000000"),
        "TOTAL BTC must be the sum: 0.75000000"
    );
    assert!(
        buffer_has(&buf, "4000.00"),
        "TOTAL USD basis must be the sum: 4000.00"
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

/// Helper: same as make_disposal but with a custom event tag for uniqueness.
fn make_disposal_tagged(
    tag: &str,
    disposed_year: i32,
    sat: i64,
    proceeds: &str,
    basis: &str,
    gain: &str,
) -> Disposal {
    Disposal {
        event: make_event_id(tag),
        kind: DisposeKind::Sell,
        disposed_at: make_date(disposed_year, 6, 15),
        legs: vec![DisposalLeg {
            lot_id: make_lot_id(&format!("dl{tag}")),
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

/// 6. Two-disposal fixture — assert TOTAL row shows Σ proceeds, Σ basis, Σ gain,
///    which differ from every individual disposal's values.
#[test]
fn disposals_renders_total_row() {
    let mut state = LedgerState::default();
    // Disposal A: proceeds=30000.00, basis=20000.00, gain=10000.00
    state.disposals.push(make_disposal_tagged(
        "d2025a", 2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    // Disposal B: proceeds=15000.00, basis=8000.00, gain=7000.00
    // TOTAL:      proceeds=45000.00, basis=28000.00, gain=17000.00
    // (sum ≠ either individual value — broken summation would fail)
    state.disposals.push(make_disposal_tagged(
        "d2025b", 2025, 25_000_000, "15000.00", "8000.00", "7000.00",
    ));

    let mut app = make_app(state, 2025);
    let buf = render_disposals(&mut app);

    assert!(buffer_has(&buf, "TOTAL"), "must have TOTAL row");
    // Individual row values still appear in data rows
    assert!(
        buffer_has(&buf, "30000.00"),
        "first disposal proceeds must be present"
    );
    assert!(
        buffer_has(&buf, "15000.00"),
        "second disposal proceeds must be present"
    );
    // TOTAL must show summed values (45000 ≠ 30000 ≠ 15000; etc.)
    assert!(
        buffer_has(&buf, "45000.00"),
        "TOTAL proceeds must be the sum: 45000.00"
    );
    assert!(
        buffer_has(&buf, "28000.00"),
        "TOTAL basis must be the sum: 28000.00"
    );
    assert!(
        buffer_has(&buf, "17000.00"),
        "TOTAL gain must be the sum: 17000.00"
    );
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
    // Out-of-year disposal: 2024 — distinctive amount 5000.00 must NOT appear
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
    // 2024-only proceeds value (5000.00) must NOT appear anywhere in the render
    assert!(
        !buffer_has(&buf, "5000.00"),
        "2024 disposal proceeds (5000.00) must NOT appear in the filtered view"
    );
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

/// Helper: same as make_income but with a custom event tag for uniqueness.
fn make_income_tagged(
    tag: &str,
    year: i32,
    sat: i64,
    usd_fmv: &str,
    kind: IncomeKind,
) -> IncomeRecord {
    IncomeRecord {
        event: make_event_id(tag),
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

/// 12. Two-record fixture — assert TOTAL row shows Σ BTC and Σ FMV, which differ
///     from every individual record's value.
#[test]
fn income_renders_total_row() {
    let mut state = LedgerState::default();
    // Record A: 1_000_000 sat = 0.01000000 BTC, $600.00
    state.income_recognized.push(make_income_tagged(
        "i2025a",
        2025,
        1_000_000,
        "600.00",
        IncomeKind::Staking,
    ));
    // Record B: 500_000 sat = 0.00500000 BTC, $300.00
    // TOTAL:    1_500_000 sat = 0.01500000 BTC, $900.00
    // (900.00 ≠ 600.00 ≠ 300.00 — broken summation would fail)
    state.income_recognized.push(make_income_tagged(
        "i2025b",
        2025,
        500_000,
        "300.00",
        IncomeKind::Mining,
    ));

    let mut app = make_app(state, 2025);
    let buf = render_income(&mut app);

    assert!(buffer_has(&buf, "TOTAL"), "must have TOTAL row");
    // Individual row values still appear in data rows
    assert!(
        buffer_has(&buf, "600.00"),
        "first record FMV must be present"
    );
    assert!(
        buffer_has(&buf, "300.00"),
        "second record FMV must be present"
    );
    // TOTAL must show summed values (900.00 ≠ 600.00 ≠ 300.00)
    assert!(
        buffer_has(&buf, "900.00"),
        "TOTAL FMV must be the sum: 900.00"
    );
    // TOTAL BTC: (1_000_000 + 500_000) / 100_000_000 = 0.01500000
    assert!(
        buffer_has(&buf, "0.01500000"),
        "TOTAL BTC must be the sum: 0.01500000"
    );
}

// ── Viewer-level tests ────────────────────────────────────────────────────────

/// 13. Spec §6: the viewer tab bar title must include the vault path.
#[test]
fn viewer_header_shows_vault_path() {
    // Use a recognisable short filename so it will not be truncated at 120 cols.
    let mut app = make_app(LedgerState::default(), 2025);
    app.vault_path = PathBuf::from("/tmp/test-vault.pgp");
    let buf = render_viewer(&mut app);
    assert!(
        buffer_has(&buf, "test-vault.pgp"),
        "viewer tab bar must contain the vault path (expected 'test-vault.pgp')"
    );
}

/// 14. ↑/↓ via handle_key wires through to TableState selection.
///     Exercises the KeyCode::Down / KeyCode::Up → scroll_down/scroll_up path
///     in handle_key's Screen::Viewer arm end-to-end.
#[test]
fn up_down_via_handle_key_moves_selection() {
    let lot1 = Lot {
        lot_id: make_lot_id("hk1"),
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
        lot_id: make_lot_id("hk2"),
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

    // Down key through handle_key selects first row
    crate::handle_key(&mut app, press(KeyCode::Down));
    assert_eq!(
        app.holdings_state.selected(),
        Some(0),
        "Down via handle_key must select first row (index 0)"
    );

    // Down key again moves to index 1
    crate::handle_key(&mut app, press(KeyCode::Down));
    assert_eq!(
        app.holdings_state.selected(),
        Some(1),
        "second Down via handle_key must advance to row 1"
    );

    // Up key moves back to index 0
    crate::handle_key(&mut app, press(KeyCode::Up));
    assert_eq!(
        app.holdings_state.selected(),
        Some(0),
        "Up via handle_key must move back to row 0"
    );
}

/// 15. ←/→ year change via handle_key updates the filtered rows rendered by a year-scoped tab.
///     Covers the brief requirement "changes selected_year AND updates the filtered rows".
#[test]
fn year_change_via_handle_key_updates_filtered_rows() {
    let mut state = LedgerState::default();
    // 2025 disposal: distinctive proceeds value 30000.00
    state.disposals.push(make_disposal(
        2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    // 2024 disposal: distinctive proceeds value 5000.00
    state.disposals.push(make_disposal(
        2024, 10_000_000, "5000.00", "3000.00", "2000.00",
    ));

    let mut app = make_app(state, 2025);
    app.tab = crate::app::Tab::Disposals;

    // ── Render at year 2025 ──────────────────────────────────────────────────
    let buf_2025 = render_disposals(&mut app);
    assert!(
        buffer_has(&buf_2025, "2025-06-15"),
        "2025 disposal date must appear at selected_year=2025"
    );
    assert!(
        !buffer_has(&buf_2025, "2024-06-15"),
        "2024 disposal date must NOT appear at selected_year=2025"
    );
    assert!(
        !buffer_has(&buf_2025, "5000.00"),
        "2024 disposal proceeds must NOT appear at selected_year=2025"
    );

    // ── Switch to year 2024 via Left key ────────────────────────────────────
    crate::handle_key(&mut app, press(KeyCode::Left));
    assert_eq!(
        app.selected_year, 2024,
        "Left key must decrement selected_year to 2024"
    );

    // ── Re-render at year 2024 ───────────────────────────────────────────────
    let buf_2024 = render_disposals(&mut app);
    assert!(
        buffer_has(&buf_2024, "2024-06-15"),
        "2024 disposal date must appear after year change to 2024"
    );
    assert!(
        !buffer_has(&buf_2024, "2025-06-15"),
        "2025 disposal date must NOT appear after year change to 2024"
    );
    assert!(
        buffer_has(&buf_2024, "5000.00"),
        "2024 disposal proceeds (5000.00) must appear after year change to 2024"
    );
    assert!(
        !buffer_has(&buf_2024, "30000.00"),
        "2025 disposal proceeds (30000.00) must NOT appear after year change to 2024"
    );
}

// ── Task 4 helpers ────────────────────────────────────────────────────────────

/// Build a `TaxProfile` for Single filer with ordinary income $50,000.
fn make_tax_profile_single_50k() -> TaxProfile {
    TaxProfile {
        filing_status: FilingStatus::Single,
        ordinary_taxable_income: Decimal::from(50_000i64),
        magi_excluding_crypto: Decimal::from(50_000i64),
        qualified_dividends_and_other_pref_income: Decimal::ZERO,
        other_net_capital_gain: Decimal::ZERO,
        capital_loss_carryforward_in: Carryforward::default(),
        w2_ss_wages: Decimal::ZERO,
        w2_medicare_wages: Decimal::ZERO,
    }
}

/// Build a Snapshot with the given state and a 2025 TaxProfile.
fn make_snapshot_with_profile(state: LedgerState) -> Snapshot {
    let mut profiles = BTreeMap::new();
    profiles.insert(2025, make_tax_profile_single_50k());
    Snapshot {
        events: vec![],
        state,
        cli_config: btctax_cli::CliConfig::default(),
        profiles,
        tables: BundledTaxTables::load(),
        donation_details: BTreeMap::new(),
    }
}

/// Build an App with the given state and a 2025 TaxProfile, in Viewer screen at year `year`.
fn make_app_with_profile(state: LedgerState, year: i32) -> App {
    let mut app = App::new(PathBuf::new());
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(make_snapshot_with_profile(state));
    app
}

/// Make a long-term disposal in the given year: 50M sat, proceeds $30,000, basis $20,000, gain $10,000.
fn make_lt_disposal(year: i32) -> Disposal {
    Disposal {
        event: make_event_id(&format!("lt{year}")),
        kind: DisposeKind::Sell,
        disposed_at: make_date(year, 6, 15),
        legs: vec![DisposalLeg {
            lot_id: make_lot_id(&format!("ltleg{year}")),
            sat: 50_000_000,
            proceeds: Decimal::from(30_000i64),
            basis: Decimal::from(20_000i64),
            gain: Decimal::from(10_000i64),
            term: Term::LongTerm,
            basis_source: BasisSource::ExchangeProvided,
            gift_zone: None,
            acquired_at: make_date(year - 2, 1, 1), // > 1 year before disposal
            wallet: make_wallet(),
        }],
        fee_mini_disposition: false,
    }
}

fn render_tax(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::tax::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn render_forms(app: &mut App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::forms::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn render_compliance(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::compliance::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

// ── Tax tab tests ─────────────────────────────────────────────────────────────

/// T1. Computed year: Tax tab shows ST/LT/NIIT/LTCG lines with known figures.
///
/// Fixture: Single filer, ordinary income $50k, 1 LT disposal gain $10k in 2025.
/// Expected: lt_net=10000.00, ltcg_tax=1500.00 (15% × $10k), niit=0.00, total=1500.00.
/// (MAGI = $50k + $10k gain = $60k < $200k NIIT threshold → NIIT = $0.)
#[test]
fn tax_tab_computed_year_shows_known_figures() {
    let mut state = LedgerState::default();
    state.disposals.push(make_lt_disposal(2025));

    let app = make_app_with_profile(state, 2025);
    let buf = render_tax(&app);

    // LT net must appear
    assert!(
        buffer_has(&buf, "10000.00"),
        "Tax tab must show LT net 10000.00"
    );
    // LTCG tax = 15% × $10k = $1500
    assert!(
        buffer_has(&buf, "1500.00"),
        "Tax tab must show LTCG tax 1500.00"
    );
    // Labels for the main sections must appear
    assert!(buffer_has(&buf, "LTCG"), "Tax tab must show LTCG label");
    assert!(buffer_has(&buf, "NIIT"), "Tax tab must show NIIT label");
    assert!(
        buffer_has(&buf, "TOTAL federal"),
        "Tax tab must show TOTAL federal label"
    );
}

/// T2. NotComputable year (no profile): Tax tab shows blocker reason but NO dollar figure.
///
/// profiles map is empty → compute_tax_year returns NotComputable(TaxProfileMissing).
/// Assert "NOT COMPUTABLE" appears and the specific LT figure 10000.00 does NOT appear.
#[test]
fn tax_tab_not_computable_no_profile_shows_blocker_no_numbers() {
    let mut state = LedgerState::default();
    state.disposals.push(make_lt_disposal(2025));

    // No profile in the snapshot — uses make_snapshot (empty profiles map).
    let mut app = make_app(state, 2025);
    // Set tab to Tax for render
    app.tab = crate::app::Tab::Tax;

    let buf = render_tax(&app);

    assert!(
        buffer_has(&buf, "NOT COMPUTABLE"),
        "Tax tab must show NOT COMPUTABLE when no profile"
    );
    assert!(
        buffer_has(&buf, "TaxProfileMissing"),
        "Tax tab must name the blocker kind TaxProfileMissing"
    );
    // Must NOT show any LT-net dollar figure
    assert!(
        !buffer_has(&buf, "10000.00"),
        "Tax tab must NOT show dollar figures when not computable"
    );
}

/// T3. ←/→ year change updates the Tax tab figures.
///
/// At 2025 (profile exists): Computed result appears.
/// After Left → 2024 (no profile): NotComputable appears.
#[test]
fn tax_tab_year_change_updates_figures() {
    let mut state = LedgerState::default();
    state.disposals.push(make_lt_disposal(2025));

    let mut app = make_app_with_profile(state, 2025);
    app.tab = crate::app::Tab::Tax;

    // 2025: Computed
    let buf_2025 = render_tax(&app);
    assert!(
        buffer_has(&buf_2025, "1500.00"),
        "Tax tab at 2025 must show LTCG tax 1500.00"
    );
    assert!(
        !buffer_has(&buf_2025, "NOT COMPUTABLE"),
        "Tax tab at 2025 must NOT show NOT COMPUTABLE"
    );

    // Switch to 2024 via Left key
    crate::handle_key(&mut app, press(KeyCode::Left));
    assert_eq!(app.selected_year, 2024, "Left key must change year to 2024");

    let buf_2024 = render_tax(&app);
    assert!(
        buffer_has(&buf_2024, "NOT COMPUTABLE"),
        "Tax tab at 2024 (no profile) must show NOT COMPUTABLE"
    );
    // 2025 figures must not appear
    assert!(
        !buffer_has(&buf_2024, "1500.00"),
        "Tax tab at 2024 must NOT show 2025 LTCG tax"
    );
}

// ── Forms tab tests ───────────────────────────────────────────────────────────

/// F1. Forms tab shows a known 8949 row (part + box) and Schedule D totals.
///
/// Fixture: 1 LT disposal in 2025 with proceeds=$30,000, basis=$20,000, gain=$10,000.
/// Expected: 8949 Part "LT" + Box "F" appear; Schedule D Part II proceeds=$30,000.
#[test]
fn forms_tab_shows_known_8949_row_and_schedule_d_totals() {
    let mut state = LedgerState::default();
    state.disposals.push(make_lt_disposal(2025));

    let mut app = make_app(state, 2025);
    app.tab = crate::app::Tab::Forms;

    let buf = render_forms(&mut app);

    // Form 8949 table must show the part and box for the LT disposal
    assert!(
        buffer_has(&buf, "LT"),
        "Forms tab must show Part II (LT) for long-term disposal"
    );
    assert!(
        buffer_has(&buf, "F"),
        "Forms tab must show Box F for long-term disposal"
    );
    // Proceeds $30,000 must appear in the 8949 table
    assert!(
        buffer_has(&buf, "30000.00"),
        "Forms tab must show 8949 row proceeds 30000.00"
    );
    // Schedule D section must appear
    assert!(
        buffer_has(&buf, "Schedule D"),
        "Forms tab must show Schedule D section"
    );
}

// ── Compliance tab tests ──────────────────────────────────────────────────────

/// C1. Compliance tab shows Hard-vs-Advisory partition and pre-2025/safe-harbor status.
///
/// Fixture: 1 hard blocker (Unclassified), 1 advisory blocker (Pre2025MethodNote).
/// Expected: Hard blockers and Advisory blockers sections appear with their counts;
/// the CliConfig default (FIFO, unattested) is shown; safe-harbor status appears.
#[test]
fn compliance_tab_shows_hard_advisory_partition_and_status() {
    let mut state = LedgerState::default();
    // Add a hard blocker
    state.blockers.push(btctax_core::Blocker {
        kind: BlockerKind::Unclassified,
        event: Some(make_event_id("ev1")),
        detail: "test hard blocker detail".into(),
    });
    // Add an advisory blocker
    state.blockers.push(btctax_core::Blocker {
        kind: BlockerKind::Pre2025MethodNote,
        event: None,
        detail: "test advisory blocker detail".into(),
    });

    // Verify severity of our test fixtures (KAT: the blockers go to the right partition)
    assert_eq!(
        BlockerKind::Unclassified.severity(),
        Severity::Hard,
        "Unclassified must be Hard"
    );
    assert_eq!(
        BlockerKind::Pre2025MethodNote.severity(),
        Severity::Advisory,
        "Pre2025MethodNote must be Advisory"
    );

    let app = make_app(state, 2025);
    let buf = render_compliance(&app);

    // Both partitions must appear
    assert!(
        buffer_has(&buf, "Hard blockers"),
        "Compliance tab must show Hard blockers section"
    );
    assert!(
        buffer_has(&buf, "Advisory blockers"),
        "Compliance tab must show Advisory blockers section"
    );
    // The hard blocker kind must appear
    assert!(
        buffer_has(&buf, "Unclassified"),
        "Compliance tab must show Unclassified hard blocker"
    );
    // The advisory blocker kind must appear
    assert!(
        buffer_has(&buf, "Pre2025MethodNote"),
        "Compliance tab must show Pre2025MethodNote advisory blocker"
    );
    // Pre-2025 method (FIFO is the default from CliConfig::default)
    assert!(
        buffer_has(&buf, "FIFO"),
        "Compliance tab must show pre-2025 method FIFO"
    );
    // Safe-harbor status must appear
    assert!(
        buffer_has(&buf, "Safe-harbor"),
        "Compliance tab must show safe-harbor status"
    );
}

// ── Minor B KATs ─────────────────────────────────────────────────────────────

/// MB1. `G` on a populated Holdings tab selects the last DATA row, NOT the TOTAL row.
///
/// Fixture: 2 lots → data rows at indices 0 and 1; TOTAL rendered at index 2 but never selectable.
/// `G` (go_bottom) must cap at index 1 (last data row), not 2 (TOTAL).
#[test]
fn total_row_not_selectable_g_selects_last_data_row() {
    let lot1 = Lot {
        lot_id: make_lot_id("mb1"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 1, 1),
        original_sat: 10_000_000,
        remaining_sat: 10_000_000,
        usd_basis: Decimal::from(500i64),
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let lot2 = Lot {
        lot_id: make_lot_id("mb2"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 6, 1),
        original_sat: 20_000_000,
        remaining_sat: 20_000_000,
        usd_basis: Decimal::from(1000i64),
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

    // No selection initially
    assert_eq!(app.holdings_state.selected(), None);

    // Press G → go_bottom
    crate::handle_key(&mut app, press(KeyCode::Char('G')));

    assert_eq!(
        app.holdings_state.selected(),
        Some(1), // last DATA row (index 1), NOT the TOTAL row (which would be index 2)
        "G must select the last DATA row (index 1), not the TOTAL row (index 2)"
    );
}

/// MB2. `scroll_down` on Holdings never lands on the TOTAL row even when at the last data row.
///
/// Fixture: 2 lots; selection starts at last data row (index 1); another scroll_down must stay at 1.
#[test]
fn scroll_down_does_not_advance_past_last_data_row_to_total() {
    let lot1 = Lot {
        lot_id: make_lot_id("mb3"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 1, 1),
        original_sat: 10_000_000,
        remaining_sat: 10_000_000,
        usd_basis: Decimal::from(500i64),
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
    };
    let lot2 = Lot {
        lot_id: make_lot_id("mb4"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 6, 1),
        original_sat: 20_000_000,
        remaining_sat: 20_000_000,
        usd_basis: Decimal::from(1000i64),
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

    // Navigate to last data row
    crate::scroll_down(&mut app); // → 0
    crate::scroll_down(&mut app); // → 1 (last data row)
    assert_eq!(
        app.holdings_state.selected(),
        Some(1),
        "scroll_down twice must reach the last data row (index 1)"
    );

    // One more scroll_down must NOT advance to TOTAL (index 2)
    crate::scroll_down(&mut app);
    assert_eq!(
        app.holdings_state.selected(),
        Some(1),
        "scroll_down past last data row must stay at index 1 (TOTAL is not selectable)"
    );
}
