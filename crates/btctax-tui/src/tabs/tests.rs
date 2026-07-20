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
        refused: BTreeMap::new(),
        tables: BundledTaxTables::load(),
        donation_details: BTreeMap::new(),
        bulk_estimated: BTreeMap::new(),
        prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
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
        pseudo: false,
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

/// 2. Two-lot fixture — assert the frozen footer shows Σ BTC and the WEIGHTED-AVERAGE basis
///    $/BTC (`round_cents((Σ usd_basis × 1e8) / Σ sat)`), provably distinct from each lot's
///    $/BTC, the simple average, and the summed basis. (Re-pointed from the old summed-basis
///    `holdings_renders_total_row`.)
#[test]
fn holdings_footer_weighted_average_basis() {
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
        pseudo: false,
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
        pseudo: false,
    };
    let mut state = LedgerState::default();
    state.lots.push(lot_a);
    state.lots.push(lot_b);

    let mut app = make_app(state, 2025);
    let buf = render_holdings(&mut app);

    assert!(buffer_has(&buf, "TOTAL"), "must have TOTAL footer");
    // Individual row values still appear in data rows
    assert!(
        buffer_has(&buf, "0.50000000"),
        "first lot BTC must be present"
    );
    assert!(
        buffer_has(&buf, "0.25000000"),
        "second lot BTC must be present"
    );
    // Σ BTC unchanged: 0.75000000 (0.75 ≠ 0.50 ≠ 0.25).
    assert!(
        buffer_has(&buf, "0.75000000"),
        "footer Σ BTC must be the sum: 0.75000000"
    );

    // Weighted-average basis $/BTC — computed with the SAME expression/rounding as the impl.
    // Σ usd_basis = 2500 + 1500 = 4000.00; Σ sat = 75_000_000; (4000 × 1e8) / 75e6 = 5333.33̅.
    let total_basis = Decimal::new(400000, 2); // 4000.00
    let total_sat: i64 = 75_000_000;
    let expected = btctax_core::conventions::round_cents(
        (total_basis * Decimal::from(100_000_000i64)) / Decimal::from(total_sat),
    );
    assert_eq!(
        expected,
        "5333.33".parse::<Decimal>().unwrap(),
        "pinned weighted-avg formula must yield 5333.33"
    );
    assert!(
        buffer_has(&buf, "5333.33"),
        "footer basis must be the WEIGHTED AVERAGE 5333.33"
    );
    // Provably distinct from lot A $/BTC (2500/0.5=5000), lot B (1500/0.25=6000),
    // the simple average (5500), and the summed basis (4000).
    assert!(
        !buffer_has(&buf, "5000.00"),
        "must NOT be lot A $/BTC 5000.00"
    );
    assert!(
        !buffer_has(&buf, "6000.00"),
        "must NOT be lot B $/BTC 6000.00"
    );
    assert!(
        !buffer_has(&buf, "5500.00"),
        "must NOT be the simple average 5500.00"
    );
    assert!(
        !buffer_has(&buf, "4000.00"),
        "must NOT be the summed basis 4000.00"
    );
}

/// 2b. Zero-sat guard: a NON-empty `lots` whose `remaining_sat` sum to 0 (a fully-consumed lot
///     still in `state.lots`) must render the footer basis as an em-dash `—`, not panic on a
///     divide-by-zero. (Empty `lots` short-circuits to "no holdings" before any total.)
#[test]
fn holdings_footer_zero_sat_shows_dash() {
    // Fully-consumed lot: remaining_sat = 0 but a non-zero basis → without the guard this would
    // divide by zero. The lot is still present, so we do NOT hit the "no holdings" placeholder.
    let lot = Lot {
        lot_id: make_lot_id("hz"),
        wallet: make_wallet(),
        acquired_at: make_date(2024, 1, 1),
        original_sat: 50_000_000,
        remaining_sat: 0,
        usd_basis: Decimal::new(100000, 2), // 1000.00 — non-zero, to make the div-by-zero real
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
        pseudo: false,
    };
    let mut state = LedgerState::default();
    state.lots.push(lot);

    let mut app = make_app(state, 2025);
    let buf = render_holdings(&mut app); // must not panic

    assert!(
        !buffer_has(&buf, "no holdings"),
        "a non-empty lots vec must NOT hit the 'no holdings' placeholder"
    );
    assert!(buffer_has(&buf, "TOTAL"), "footer must still render");
    assert!(
        buffer_has(&buf, "\u{2014}"),
        "footer basis must show the em-dash — when Σ sat == 0"
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
        pseudo: false,
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
        pseudo: false,
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
            pseudo: false,
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
            pseudo: false,
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
        pseudo: false,
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
        pseudo: false,
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
        pseudo: false,
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
        pseudo: false,
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

/// SPEC §3.4 clock seam: the export-confirm modal renders the INJECTED clock's timestamp in the export
/// directory name, not a fresh `now_utc()`. With `app.clock = Pinned(2024-06-01T12:00:00Z)`, pressing `e`
/// opens a modal showing `btctax-export-20240601-120000Z` — deterministic and byte-stable across renders.
/// (Before the seam both `handle_key` clock reads were `OffsetDateTime::now_utc()`, so this rendered the
/// wall clock and could not be pinned — this test is the guard that the routing stays in place.)
#[test]
fn export_modal_dir_name_uses_the_injected_clock() {
    use crate::clock::Clock;
    use time::macros::datetime;
    let mut app = make_app(LedgerState::default(), 2025);
    app.clock = Clock::Pinned(datetime!(2024 - 06 - 01 12:00:00 UTC));
    crate::handle_key(&mut app, press(KeyCode::Char('e')));
    assert!(
        app.export_modal.is_some(),
        "pressing `e` with a snapshot must open the export modal"
    );
    let buf = render_viewer(&mut app);
    assert!(
        buffer_has(&buf, "btctax-export-20240601-120000Z"),
        "the export-confirm modal must render the PINNED clock's dir name (SPEC §3.4 seam)"
    );
    // Pinned ⇒ deterministic: a second render is byte-identical.
    let buf2 = render_viewer(&mut app);
    assert_eq!(
        buf, buf2,
        "a pinned-clock render must be byte-identical across runs"
    );
}

// ══════════════════ P3 style-aware TUI goldens (SPEC §8) ══════════════════════════════════════════
//
// Committed under `docs/examples-tui/`, gated `regen == committed` (mirrors the CLI golden, Task 1.4).
// `#[cfg(unix)]` (like I4): the export-modal frame renders a joined path (`./btctax-export-…`) whose
// separator diverges on Windows. Every frame is captured under a PINNED clock + a fixed (empty) vault
// path, so the goldens are a pure function of (code, synthetic state).

/// The pinned wall-clock every TUI golden renders under (matches the CLI docs pipeline's fixed instant).
#[cfg(unix)]
fn golden_clock() -> crate::clock::Clock {
    crate::clock::Clock::Pinned(time::macros::datetime!(2024 - 06 - 01 12:00:00 UTC))
}

#[cfg(unix)]
fn tui_golden_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/examples-tui"
    ))
}

/// The btctax-tui goldens: `(file stem, captured frame)`. A one-lot Holdings tab (style-aware capture of a
/// real table) + the export-confirm modal (the viewer clock seam — its dir name is the pinned instant).
#[cfg(unix)]
fn btctax_tui_goldens() -> Vec<(&'static str, String)> {
    let holdings = {
        let lot = Lot {
            lot_id: make_lot_id("h1"),
            wallet: make_wallet(),
            acquired_at: make_date(2024, 1, 15),
            original_sat: 100_000_000,
            remaining_sat: 100_000_000,
            usd_basis: Decimal::new(6_000_000, 2), // 60000.00
            basis_source: BasisSource::ExchangeProvided,
            dual_loss_basis: None,
            donor_acquired_at: None,
            basis_pending: false,
            pseudo: false,
        };
        let mut state = LedgerState::default();
        state.lots.push(lot);
        let mut app = make_app(state, 2025);
        app.clock = golden_clock();
        crate::capture::to_golden(&render_holdings(&mut app))
    };
    let export_modal = {
        let mut app = make_app(LedgerState::default(), 2025);
        app.clock = golden_clock();
        crate::handle_key(&mut app, press(KeyCode::Char('e')));
        assert!(
            app.export_modal.is_some(),
            "the export modal must open for the golden"
        );
        crate::capture::to_golden(&render_viewer(&mut app))
    };
    vec![
        ("viewer-holdings", holdings),
        ("viewer-export-modal", export_modal),
    ]
}

/// The committed btctax-tui goldens match a fresh capture, byte-for-byte (reds when stale).
#[cfg(unix)]
#[test]
fn btctax_tui_goldens_match_committed() {
    for (stem, captured) in btctax_tui_goldens() {
        let path = tui_golden_dir().join(format!("btctax-tui-{stem}.txt"));
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "committed {} missing ({e}); regenerate with \
                 `cargo test -p btctax-tui --lib emit_btctax_tui_goldens -- --ignored`",
                path.display()
            )
        });
        assert_eq!(
            captured, committed,
            "docs/examples-tui/btctax-tui-{stem}.txt is STALE; regenerate via the ignored \
             emit_btctax_tui_goldens test"
        );
    }
}

/// Regeneration helper (`#[ignore]`) — rewrites the committed btctax-tui goldens from a fresh capture.
#[cfg(unix)]
#[test]
#[ignore = "regeneration helper: rewrites docs/examples-tui/btctax-tui-*.txt"]
fn emit_btctax_tui_goldens() {
    let dir = tui_golden_dir();
    std::fs::create_dir_all(&dir).expect("create docs/examples-tui");
    for (stem, captured) in btctax_tui_goldens() {
        std::fs::write(dir.join(format!("btctax-tui-{stem}.txt")), captured).expect("write golden");
    }
}

// ── TUI screen-walkthrough (design/tui-walkthrough) — viewer frames (J8, J1, …) ───────────────────
// The VIEWER half of the J8 journey: after the editor confirms the RELOCATE, the coins land at Coinbase
// and Holdings reads BALANCED. Re-seeds the SAME shared J8 fixture and replays the SAME decision via
// btctax-cli (RELOCATE = link_transfer out→in), then builds the REAL snapshot — so this "after" state
// provably matches the editor's mutation (walkthrough spec §4.2). Goldens live under the walkthrough tree.

#[cfg(unix)]
fn tui_walkthrough_golden_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/examples-tui-walkthrough"
    ))
}

#[cfg(unix)]
/// All journeys' VIEWER frames, aggregated. Phase 2 appends one `jN_viewer_frames()` per journey; the
/// gate + emit iterate this one list, and `WALKTHROUGH_VIEWER_STEMS` declares the expected stem set.
fn btctax_tui_walkthrough_frames() -> Vec<(&'static str, String)> {
    let mut frames = j8_viewer_frames();
    frames.extend(j1_viewer_frames());
    frames.extend(j4_viewer_frames());
    frames.extend(j7_viewer_frames());
    frames.extend(j3_viewer_frames());
    frames.extend(j9_viewer_frames());
    frames.extend(j2_viewer_frames());
    frames.extend(j5_viewer_frames());
    frames.extend(j6_viewer_frames());
    frames
}

fn j8_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    // Determinism (walkthrough spec §7 / I-3): pin the price cache so a dev's live cache can't perturb the
    // frame. Safe under nextest's process-per-test model.
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j8-pass".into());
    let now = time::macros::datetime!(2025 - 04 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    // Seed + apply the RELOCATE INSIDE btctax-cli (the read-only viewer forbids the write path in its own
    // source — the e10 gate); the viewer half then only OPENS + reads the resulting vault (spec §4.2).
    let vault = btctax_cli::testonly::seed_j8_relocated(dir.path(), &pp, now);
    // Build the REAL snapshot + render the full viewer frame (Holdings tab) — now BALANCED.
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let holdings = crate::capture::to_golden(&render_viewer(&mut app));
    vec![("j8/04-holdings-balanced", holdings)]
}

/// J1 (single buyer) VIEWER frames — J1 has NO editor half (a buy + a sell, no transfers to reconcile), so
/// its CLI setup (`init`/`import`/`verify`/`tax-profile`) is the console transcript and the viewer shows
/// the result tabs: Holdings (the 0.08 BTC left), Disposals (the 0.02 BTC sale), Tax (the 2025 numbers —
/// needs the seeded profile). Seeds via `seed_j1_with_profile` and only OPENS + reads (e10: no write path
/// in this crate's source).
fn j1_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j1-pass".into());
    let now = time::macros::datetime!(2025 - 07 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j1_with_profile(dir.path(), &pp);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j1/01-holdings", crate::app::Tab::Holdings),
        ("j1/02-disposals", crate::app::Tab::Disposals),
        ("j1/03-tax", crate::app::Tab::Tax),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// J4 (staking income → Schedule SE) VIEWER frames — after the editor reclassifies both receipts as
/// a trade or business, the viewer shows Income (now business) and Tax (the self-employment tax). Seeds via
/// `seed_j4_reclassified` (the post-reclassify state) and only opens + reads.
fn j4_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j4-pass".into());
    let now = time::macros::datetime!(2025 - 08 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j4_reclassified(dir.path(), &pp, now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j4/02-income", crate::app::Tab::Income),
        ("j4/03-tax", crate::app::Tab::Tax),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// J7 (off-exchange income valued by hand) VIEWER frames — after the editor classifies the Receive as
/// staking income with a supplied FMV, the viewer shows Income (the $3,300 FMV as ordinary income) + Tax.
/// Seeds via `seed_j7_income` (2024) and only reads.
fn j7_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j7-pass".into());
    let now = time::macros::datetime!(2024 - 07 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j7_income(dir.path(), &pp, now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j7/02-income", crate::app::Tab::Income),
        ("j7/03-tax", crate::app::Tab::Tax),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// J3 (self-transfer, unknown-basis inbound) VIEWER frame — after the editor classifies the Receive as the
/// filer's own coins returning, Holdings shows both lots (the original buy + the returned coins carrying
/// their supplied $19,000 basis / 2024-11-01 date). Seeds via `seed_j3_self_transfer` and only reads.
fn j3_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j3-pass".into());
    let now = time::macros::datetime!(2025 - 08 - 02 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j3_self_transfer(dir.path(), &pp, now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    app.tab = crate::app::Tab::Holdings;
    vec![(
        "j3/02-holdings",
        crate::capture::to_golden(&render_viewer(&mut app)),
    )]
}

/// J9 (select-lots) VIEWER frames — after the editor identifies the 0.50 BTC sale against the cheap
/// long-term lot-a, Disposals shows the sale drawing from lot-a (a long-term gain), and Compliance shows
/// the post-2025 per-disposal requirement now satisfied (specific lots identified). Seeds via
/// `seed_j9_selected` and only reads.
fn j9_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j9-pass".into());
    let now = time::macros::datetime!(2025 - 06 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j9_selected(dir.path(), &pp, now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j9/02-disposals", crate::app::Tab::Disposals),
        ("j9/03-compliance", crate::app::Tab::Compliance),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// J2 (§170(e) charitable donation) VIEWER frames — after the editor reclassifies the Send as a donation
/// and records the Form 8283 details, the viewer shows Forms (the two 8283 rows: the carrier leg at FMV,
/// the second leg flagged for the paper form) + Tax (the charitable deduction, before §170(b) AGI limits).
/// Seeds via `seed_j2_donated` and only reads.
fn j2_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j2-pass".into());
    let now = time::macros::datetime!(2025 - 09 - 15 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j2_donated(dir.path(), &pp, now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j2/03-forms", crate::app::Tab::Forms),
        ("j2/04-tax", crate::app::Tab::Tax),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// J5 (lot-selection optimization) VIEWER frames — after the editor accepts the optimizer's pick (the sale
/// moved onto the short-term lot to realize a loss), the viewer shows Disposals (the sale now drawing from
/// the $80k ST lot → a $30,000 short-term loss) and Tax (the §1211 $3,000 loss deduction, the $27,000
/// short carryforward, and the −$660 attributable). Seeds via `seed_j5_optimized`. The accept made-date
/// (2025-04-01, ≤ the sale) is DELIBERATELY earlier than the review display clock (2025-07-01) — the
/// identification is made before the sale, reviewed after; neither Disposals nor Tax renders that made-date.
fn j5_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j5-pass".into());
    let accept_now = time::macros::datetime!(2025 - 04 - 01 12:00:00 UTC);
    let display_now = time::macros::datetime!(2025 - 07 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j5_optimized(dir.path(), &pp, accept_now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(display_now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j5/03-disposals", crate::app::Tab::Disposals),
        ("j5/04-tax", crate::app::Tab::Tax),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// J6 (a complete Form 1040) VIEWER frames — the payoff of the kitchen-sink MFJ household merged with the
/// crypto ledger. Forms shows the CRYPTO forms (Form 8949 for the BTC sale, Schedule D totals, Form 8283
/// for the donation — NOT the whole 14-form packet, which lives in `export-irs-pdf`); Tax shows the return
/// computed from the full-return-derived MFJ profile (capital gains, the charitable deduction, Schedule
/// SE on the mining income). Seeds via `seed_j6_full` and only reads.
fn j6_viewer_frames() -> Vec<(&'static str, String)> {
    use btctax_store::Passphrase;
    std::env::set_var(
        "BTCTAX_PRICE_CACHE",
        "/nonexistent-walkthrough-price-cache.csv",
    );
    let pp = Passphrase::new("golden-j6-pass".into());
    let now = time::macros::datetime!(2025 - 02 - 01 12:00:00 UTC);
    let dir = tempfile::tempdir().unwrap();
    let vault = btctax_cli::testonly::seed_j6_full(dir.path(), &pp, now);
    let session = btctax_cli::Session::open(&vault, &pp).unwrap();
    let (snapshot, year) = crate::unlock::build_snapshot(&session).unwrap();
    let mut app = App::new(std::path::PathBuf::from("/vault.pgp"));
    app.screen = Screen::Viewer;
    app.selected_year = year;
    app.snapshot = Some(snapshot);
    app.clock = crate::clock::Clock::Pinned(now);
    let mut out = Vec::new();
    for (stem, tab) in [
        ("j6/04-forms", crate::app::Tab::Forms),
        ("j6/05-tax", crate::app::Tab::Tax),
    ] {
        app.tab = tab;
        out.push((stem, crate::capture::to_golden(&render_viewer(&mut app))));
    }
    out
}

/// The frame stems this crate (the viewer half) is responsible for capturing. Declared EXPLICITLY so a
/// dropped/renamed capture tuple reds the gate below (NEW-I-1): the byte-compare loop alone iterates only
/// over what's captured, so a shrunk set passes vacuously — and the xtask manifest bijection would still
/// hold (manifest⇄disk unchanged), leaving an orphaned golden that keeps rendering a never-re-verified,
/// silently stale screen. This const pins disk⇄capture; Phase 2 extends it per journey, on purpose.
#[cfg(unix)]
const WALKTHROUGH_VIEWER_STEMS: &[&str] = &[
    "j8/04-holdings-balanced",
    "j1/01-holdings",
    "j1/02-disposals",
    "j1/03-tax",
    "j4/02-income",
    "j4/03-tax",
    "j7/02-income",
    "j7/03-tax",
    "j3/02-holdings",
    "j9/02-disposals",
    "j9/03-compliance",
    "j2/03-forms",
    "j2/04-tax",
    "j5/03-disposals",
    "j5/04-tax",
    "j6/04-forms",
    "j6/05-tax",
];

#[cfg(unix)]
#[test]
fn btctax_tui_walkthrough_goldens_match_committed() {
    let frames = btctax_tui_walkthrough_frames();
    let got: std::collections::BTreeSet<&str> = frames.iter().map(|(s, _)| *s).collect();
    let expected: std::collections::BTreeSet<&str> =
        WALKTHROUGH_VIEWER_STEMS.iter().copied().collect();
    assert_eq!(
        got, expected,
        "the viewer walkthrough capture set changed — a dropped/renamed frame would ship a stale \
         screen (NEW-I-1). If intentional, update WALKTHROUGH_VIEWER_STEMS and the manifest together."
    );
    for (stem, captured) in frames {
        let path = tui_walkthrough_golden_dir().join(format!("{stem}.txt"));
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "committed {} missing ({e}); regenerate with \
                 `cargo test -p btctax-tui --lib emit_btctax_tui_walkthrough_goldens -- --ignored`",
                path.display()
            )
        });
        assert_eq!(
            captured, committed,
            "docs/examples-tui-walkthrough/{stem}.txt is STALE; regenerate via the ignored \
             emit_btctax_tui_walkthrough_goldens test"
        );
    }
}

#[cfg(unix)]
#[test]
#[ignore = "regeneration helper: rewrites docs/examples-tui-walkthrough/{j8,j1,…}/*.txt viewer frames"]
fn emit_btctax_tui_walkthrough_goldens() {
    for (stem, captured) in btctax_tui_walkthrough_frames() {
        let path = tui_walkthrough_golden_dir().join(format!("{stem}.txt"));
        std::fs::create_dir_all(path.parent().unwrap()).expect("create walkthrough golden dir");
        std::fs::write(&path, captured).expect("write walkthrough golden");
    }
}

/// 15. `[`/`]` year change via handle_key updates the filtered rows rendered by a year-scoped tab.
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

    // ── Switch to year 2024 via '[' key ([R0-M-3] year MOVED off arrows to [ / ]) ────
    crate::handle_key(&mut app, press(KeyCode::Char('[')));
    assert_eq!(
        app.selected_year, 2024,
        "'[' key must decrement selected_year to 2024"
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
        schedule_c_expenses: Decimal::ZERO,
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
        refused: BTreeMap::new(),
        tables: BundledTaxTables::load(),
        donation_details: BTreeMap::new(),
        bulk_estimated: BTreeMap::new(),
        prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
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
            pseudo: false,
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

/// T3. `[`/`]` year change updates the Tax tab figures.
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

    // Switch to 2024 via '[' key ([R0-M-3] year MOVED off arrows to [ / ])
    crate::handle_key(&mut app, press(KeyCode::Char('[')));
    assert_eq!(app.selected_year, 2024, "'[' key must change year to 2024");

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
    // Pre-2025 method (HIFO is the default from CliConfig::default; [reconcile-defaults] was FIFO)
    assert!(
        buffer_has(&buf, "HIFO"),
        "Compliance tab must show pre-2025 method HIFO"
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
        pseudo: false,
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
        pseudo: false,
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
        pseudo: false,
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
        pseudo: false,
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

// ── Frozen column-totals footer KATs (feat/tui-column-totals) ────────────────

/// Render a tab into an explicitly-sized backend (for the height-gate KAT).
fn render_disposals_sized(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            super::disposals::draw(f, area, app);
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

/// Return the y-coordinate of the first buffer row whose text contains `needle`.
fn buffer_find_y(buf: &ratatui::buffer::Buffer, needle: &str) -> Option<u16> {
    let area = buf.area();
    for y in 0..area.height {
        let row: String = (0..area.width)
            .map(|x| buf.cell((x, y)).map_or(" ", |c| c.symbol()))
            .collect();
        if row.contains(needle) {
            return Some(y);
        }
    }
    None
}

/// CT1. Disposals footer shows the SUMMED Σ BTC / Σ proceeds / Σ basis / Σ gain, and the
/// gain identity `Σ gain == Σ proceeds − Σ basis` holds.
#[test]
fn disposals_footer_shows_summed_totals() {
    let mut state = LedgerState::default();
    // Leg A: 50M sat, proceeds 30000.00, basis 20000.00, gain 10000.00
    state.disposals.push(make_disposal_tagged(
        "ctd_a", 2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    // Leg B: 25M sat, proceeds 15000.00, basis 8000.00, gain 7000.00
    // Σ:      75M sat = 0.75000000 BTC, proceeds 45000.00, basis 28000.00, gain 17000.00
    state.disposals.push(make_disposal_tagged(
        "ctd_b", 2025, 25_000_000, "15000.00", "8000.00", "7000.00",
    ));

    let mut app = make_app(state, 2025);
    let buf = render_disposals(&mut app);

    // Σ BTC = (50M + 25M) / 1e8 = 0.75000000 (appears ONLY in the footer — legs are 0.50/0.25).
    assert!(
        buffer_has(&buf, "0.75000000"),
        "footer must show Σ BTC 0.75000000"
    );
    assert!(
        buffer_has(&buf, "45000.00"),
        "footer must show Σ proceeds 45000.00"
    );
    assert!(
        buffer_has(&buf, "28000.00"),
        "footer must show Σ basis 28000.00 (SUMMED, not averaged)"
    );
    assert!(
        buffer_has(&buf, "17000.00"),
        "footer must show Σ gain 17000.00"
    );

    // Gain identity: Σ gain == Σ proceeds − Σ basis (keeps the row additive).
    let proceeds: Decimal = "45000.00".parse().unwrap();
    let basis: Decimal = "28000.00".parse().unwrap();
    let gain: Decimal = "17000.00".parse().unwrap();
    assert_eq!(
        gain,
        proceeds - basis,
        "Σ gain must equal Σ proceeds − Σ basis"
    );
}

/// CT2. The Disposals "TOTAL" renders as the PINNED footer (bottom of the table), not as a
/// scrolling body row, and is NOT selectable (G caps at the last data leg).
#[test]
fn disposals_total_row_no_longer_scrolls() {
    let mut state = LedgerState::default();
    state.disposals.push(make_disposal_tagged(
        "ctns_a", 2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    state.disposals.push(make_disposal_tagged(
        "ctns_b", 2025, 25_000_000, "15000.00", "8000.00", "7000.00",
    ));

    let mut app = make_app(state, 2025);
    app.tab = crate::app::Tab::Disposals;
    let buf = render_disposals(&mut app); // TestBackend 120×40

    let h = buf.area().height;
    let total_y = buffer_find_y(&buf, "TOTAL").expect("TOTAL must render");
    let data_y = buffer_find_y(&buf, "0.50000000").expect("a data leg must render");

    // Footer is pinned just above the bottom border (y == h-2), FAR below the top-of-table
    // data rows — a scrolling body TOTAL would sit immediately after the data (small y).
    assert!(
        total_y >= h - 2,
        "TOTAL must be the pinned footer at the bottom (y={total_y}, h={h}), not a body row"
    );
    assert!(
        data_y < total_y,
        "data rows must sit above the pinned footer (data_y={data_y}, total_y={total_y})"
    );

    // Not selectable: G (go_bottom) caps at the last DATA leg (index 1), never the TOTAL.
    crate::handle_key(&mut app, press(KeyCode::Char('G')));
    assert_eq!(
        app.disposals_state.selected(),
        Some(1),
        "G must select the last data leg (index 1), never the TOTAL footer"
    );
}

/// CT3. Height gate: below MIN_ROWS_FOR_TOTALS the frozen footer is omitted (data gets the
/// space); at/above the threshold it is present. Pins the boundary at exactly 10.
#[test]
fn totals_footer_hidden_on_short_terminal() {
    let mut state = LedgerState::default();
    state.disposals.push(make_disposal_tagged(
        "ctg_a", 2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    state.disposals.push(make_disposal_tagged(
        "ctg_b", 2025, 25_000_000, "15000.00", "8000.00", "7000.00",
    ));

    let mut app = make_app(state, 2025);

    // Height 9 (< 10): footer omitted → no "TOTAL".
    let short = render_disposals_sized(&mut app, 120, 9);
    assert!(
        !buffer_has(&short, "TOTAL"),
        "footer must be hidden on a <10-row area (height 9)"
    );

    // Height 10 (== threshold): footer present → "TOTAL".
    let tall = render_disposals_sized(&mut app, 120, 10);
    assert!(
        buffer_has(&tall, "TOTAL"),
        "footer must be present at the threshold (height 10)"
    );
}

/// CT4. Income footer sums Σ sat (as BTC) and Σ income (FMV), and renders as the pinned footer
/// (bottom), not a scrolling body row.
#[test]
fn income_footer_sums_sat_and_fmv() {
    let mut state = LedgerState::default();
    state.income_recognized.push(make_income_tagged(
        "if_a",
        2025,
        1_000_000,
        "600.00",
        IncomeKind::Staking,
    ));
    state.income_recognized.push(make_income_tagged(
        "if_b",
        2025,
        500_000,
        "300.00",
        IncomeKind::Mining,
    ));
    // Σ: 1_500_000 sat = 0.01500000 BTC, FMV 900.00

    let mut app = make_app(state, 2025);
    let buf = render_income(&mut app);

    assert!(
        buffer_has(&buf, "0.01500000"),
        "footer must show Σ BTC 0.01500000"
    );
    assert!(buffer_has(&buf, "900.00"), "footer must show Σ FMV 900.00");

    // TOTAL is the pinned footer (bottom), not a scrolling body row.
    let h = buf.area().height;
    let total_y = buffer_find_y(&buf, "TOTAL").expect("TOTAL must render");
    assert!(
        total_y >= h - 2,
        "income TOTAL must be the pinned footer at the bottom (y={total_y}, h={h})"
    );
}

// ── KAT-E7 — Disclosure-line KATs (Tax tab, render_tax_content) ──────────────

/// Fixture builder: Snapshot with business mining income and a TaxProfile.
fn make_se_snapshot(
    fmv: i64,
    schedule_c_expenses: i64,
    w2_ss_wages: i64,
    w2_medicare_wages: i64,
) -> Snapshot {
    let mut state = LedgerState::default();
    state.income_recognized.push(IncomeRecord {
        event: make_event_id(&format!("se-mining-{fmv}")),
        recognized_at: make_date(2025, 3, 1),
        sat: 100_000_000,
        usd_fmv: Decimal::from(fmv),
        kind: IncomeKind::Mining,
        business: true,
        pseudo: false,
    });

    let mut profiles = BTreeMap::new();
    profiles.insert(
        2025,
        TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: Decimal::from(50_000i64),
            magi_excluding_crypto: Decimal::from(50_000i64),
            qualified_dividends_and_other_pref_income: Decimal::ZERO,
            other_net_capital_gain: Decimal::ZERO,
            capital_loss_carryforward_in: Carryforward::default(),
            w2_ss_wages: Decimal::from(w2_ss_wages),
            w2_medicare_wages: Decimal::from(w2_medicare_wages),
            schedule_c_expenses: Decimal::from(schedule_c_expenses),
        },
    );

    Snapshot {
        events: vec![],
        state,
        cli_config: btctax_cli::CliConfig::default(),
        profiles,
        refused: BTreeMap::new(),
        tables: BundledTaxTables::load(),
        donation_details: BTreeMap::new(),
        bulk_estimated: BTreeMap::new(),
        prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
    }
}

/// UX-P4-1 surface 3(a): a pseudo-active viewer Tax tab LEADS with the banner AND suffixes the TOTAL line —
/// so the viewer is no longer a silent authoritative surface (SPEC C2/[T-C2]). (★ fault-inject: drop the
/// banner `if pseudo` block or the TOTAL `[PSEUDO]` in tabs/tax.rs and this goes RED.)
#[test]
fn e7_pseudo_active_tax_tab_carries_banner_and_total_suffix() {
    let mut snap = make_se_snapshot(50_000, 0, 0, 0);
    snap.state.pseudo_synthetic_count = 1; // pseudo_active() ⇒ true (a synthetic contributes)
    let content = super::tax::render_tax_content(&snap, 2025);
    assert!(
        content.contains("[PSEUDO] This vault has pseudo-reconciled"),
        "the pseudo banner must lead the Tax tab:\n{content}"
    );
    let total = content
        .lines()
        .find(|l| l.contains("TOTAL federal tax attributable"))
        .expect("a computed tab has a TOTAL line");
    assert!(
        total.contains("[PSEUDO]"),
        "the TUI TOTAL line must carry the [PSEUDO] suffix under pseudo: {total}"
    );
}

/// UX-P4-1 surface 3(f): the enumeration invariant that licenses the count-only signal — a year with NO
/// stored profile renders NOT COMPUTABLE in the viewer (never a fictional number and never a `[PSEUDO]`
/// figure), so the CLI's PseudoPlaceholder channel cannot reach this surface as an unflagged number.
#[test]
fn e7_no_profile_year_renders_not_computable_never_a_pseudo_number() {
    let state = LedgerState::default(); // count == 0 — placeholder-eligible in the CLI
    let snap = make_snapshot(state); // …but `profiles` is empty: no stored profile for 2025
    let content = super::tax::render_tax_content(&snap, 2025);
    assert!(
        content.contains("NOT COMPUTABLE"),
        "an unprofiled year must render NOT COMPUTABLE, never a number:\n{content}"
    );
    assert!(
        !content.contains("[PSEUDO]"),
        "the placeholder channel must NOT surface a [PSEUDO] figure in the viewer:\n{content}"
    );
}

/// KAT-E7(a): disclosure lines present when schedule_c_expenses > 0 and w2_ss_wages > 0.
///
/// Fixture: TY2025, mining $50,000, profile with schedule_c_expenses=$5,000, w2_ss_wages=$30,000.
/// Expected: gross breakout, I3-mechanism advisory, §164(f) advisory, W-2 coordination all present.
#[test]
fn e7a_disclosure_lines_with_expenses_and_w2() {
    let snap = make_se_snapshot(50_000, 5_000, 30_000, 30_000);
    let content = super::tax::render_tax_content(&snap, 2025);

    // (a) Gross breakout line.
    assert!(
        content.contains("gross business income"),
        "gross breakout must appear: 'gross business income' not found"
    );
    assert!(
        content.contains("Schedule C expenses"),
        "gross breakout must appear: 'Schedule C expenses' not found"
    );
    assert!(
        content.contains("net SE earnings"),
        "gross breakout must appear: 'net SE earnings' not found"
    );

    // (b) I3-mechanism advisory.
    assert!(
        content.contains("ORDINARY taxable income"),
        "I3 advisory must appear: 'ORDINARY taxable income' not found"
    );
    assert!(
        content.contains("OVERSTATES"),
        "I3 advisory must appear: 'OVERSTATES' not found"
    );
    assert!(
        content.contains("coordinate"),
        "I3 advisory must appear: 'coordinate' not found"
    );

    // (c) §164(f) advisory.
    assert!(
        content.contains("§164(f)"),
        "§164(f) advisory must appear: '§164(f)' not found"
    );
    assert!(
        content.contains("NOT auto-coordinated"),
        "§164(f) advisory must appear: 'NOT auto-coordinated' not found"
    );

    // (d) W-2 coordination disclosure.
    assert!(
        content.contains("W-2 coordination applied"),
        "W-2 disclosure must appear: 'W-2 coordination applied' not found"
    );
    assert!(
        content.contains("Box 3+7"),
        "W-2 disclosure must appear: 'Box 3+7' not found"
    );
}

/// KAT-E7(b): when schedule_c_expenses = 0, "no Schedule C expenses supplied" appears;
/// the gross breakout and I3 advisory are absent.
#[test]
fn e7b_no_schedule_c_expenses_line_when_zero() {
    let snap = make_se_snapshot(50_000, 0, 0, 0);
    let content = super::tax::render_tax_content(&snap, 2025);

    assert!(
        content.contains("no Schedule C expenses supplied"),
        "'no Schedule C expenses supplied' must appear when expenses=0; content:\n{content}"
    );
    // Gross breakout and I3 advisory must NOT appear (no expenses).
    assert!(
        !content.contains("gross business income"),
        "gross breakout must NOT appear when expenses=0"
    );
    assert!(
        !content.contains("OVERSTATES"),
        "I3 advisory must NOT appear when expenses=0"
    );
}

/// KAT-E7(c): fully-expensed case — gross $10,000, expenses $15,000, net ≤ $0.
/// "fully expensed" and "no §1401 SE tax" appear; "SS wage base unavailable" does NOT.
#[test]
fn e7c_fully_expensed_shows_correct_message() {
    let snap = make_se_snapshot(10_000, 15_000, 0, 0);
    let content = super::tax::render_tax_content(&snap, 2025);

    assert!(
        content.contains("fully expensed"),
        "'fully expensed' must appear when expenses >= gross; content:\n{content}"
    );
    assert!(
        content.contains("no \u{00a7}1401 SE tax"),
        "'no §1401 SE tax' must appear in fully-expensed case"
    );
    // "SS wage base unavailable" must NOT appear (table IS present for 2025).
    assert!(
        !content.contains("SS wage base unavailable"),
        "'SS wage base unavailable' must NOT appear when table is present"
    );
}

/// KAT-E7(d): [R0-I2] Profile gate — business income + table present + NO profile → no SE section.
///
/// This verifies the intentional behaviour change from the old hand-rolled SE block
/// (which defaulted to Single/$0 wages when no profile).
#[test]
fn e7d_profile_gate_no_profile_means_no_se_section() {
    // Snapshot with business income but NO profile for 2025.
    let mut state = LedgerState::default();
    state.income_recognized.push(IncomeRecord {
        event: make_event_id("e7d-mining"),
        recognized_at: make_date(2025, 3, 1),
        sat: 100_000_000,
        usd_fmv: Decimal::from(50_000i64),
        kind: IncomeKind::Mining,
        business: true,
        pseudo: false,
    });
    let snap = make_snapshot(state); // empty profiles BTreeMap

    let content = super::tax::render_tax_content(&snap, 2025);

    // NO profile → NO SE section.
    assert!(
        !content.contains("Schedule SE"),
        "NO SE section must appear when no profile (profile gate); content:\n{content}"
    );
    assert!(
        !content.contains("§1401"),
        "§1401 SE tax must NOT appear when no profile"
    );
}

/// KAT-E7(e): [R0-I2] Outcome-independent placement — NotComputable year with
/// profile + business income shows the SE section (matches CLI report behaviour).
#[test]
fn e7e_not_computable_year_with_profile_shows_se_section() {
    // Add a hard blocker so compute_tax_year returns NotComputable.
    let mut state = LedgerState::default();
    state.blockers.push(btctax_core::Blocker {
        kind: BlockerKind::Unclassified,
        event: Some(make_event_id("e7e-blocker")),
        detail: "test hard blocker for KAT-E7e".into(),
    });
    state.income_recognized.push(IncomeRecord {
        event: make_event_id("e7e-mining"),
        recognized_at: make_date(2025, 3, 1),
        sat: 100_000_000,
        usd_fmv: Decimal::from(50_000i64),
        kind: IncomeKind::Mining,
        business: true,
        pseudo: false,
    });

    // Snapshot with a profile for 2025 (even though the year is NotComputable).
    let mut profiles = BTreeMap::new();
    profiles.insert(
        2025,
        TaxProfile {
            filing_status: FilingStatus::Single,
            ordinary_taxable_income: Decimal::from(50_000i64),
            magi_excluding_crypto: Decimal::from(50_000i64),
            qualified_dividends_and_other_pref_income: Decimal::ZERO,
            other_net_capital_gain: Decimal::ZERO,
            capital_loss_carryforward_in: Carryforward::default(),
            w2_ss_wages: Decimal::ZERO,
            w2_medicare_wages: Decimal::ZERO,
            schedule_c_expenses: Decimal::ZERO,
        },
    );
    let snap = Snapshot {
        events: vec![],
        state,
        cli_config: btctax_cli::CliConfig::default(),
        profiles,
        refused: BTreeMap::new(),
        tables: BundledTaxTables::load(),
        donation_details: BTreeMap::new(),
        bulk_estimated: BTreeMap::new(),
        prices: btctax_adapters::LayeredPrices::load_with_cache(None).unwrap(),
    };

    let content = super::tax::render_tax_content(&snap, 2025);

    // The year is NotComputable (hard blocker).
    assert!(
        content.contains("NOT COMPUTABLE"),
        "NOT COMPUTABLE must appear (hard blocker present)"
    );

    // BUT the SE section must still be present (outcome-independent placement).
    assert!(
        content.contains("Schedule SE") || content.contains("§1401"),
        "SE section must appear even for NotComputable year when profile + business income present; \
         content:\n{content}"
    );
}

/// [P2-C1] A full-return year that resolves refused/uncomputable renders its REASON in the Tax tab —
/// never a computed number (and never the SE section). This is the same fail-closed answer the CLI gives;
/// the viewer must not diverge from `report`.
#[test]
fn tax_tab_refused_full_return_year_renders_reason_not_a_number() {
    let mut snap = make_snapshot(LedgerState::default());
    snap.refused.insert(
        2024,
        "an HSA requires Form 8889 — out of scope for v1".to_string(),
    );
    let content = super::tax::render_tax_content(&snap, 2024);
    assert!(
        content.contains("NOT COMPUTABLE (full-return inputs)"),
        "refused year must render its reason header; content:\n{content}"
    );
    assert!(
        content.contains("HSA"),
        "the refusal reason must appear; content:\n{content}"
    );
    // No computed figures, and no SE section, leak for the refused year.
    assert!(
        !content.contains("TOTAL federal tax attributable") && !content.contains("Schedule SE"),
        "no number/SE for a refused year; content:\n{content}"
    );
    // A different (non-refused) year is unaffected — it renders normally (no full-return refusal banner).
    let other = super::tax::render_tax_content(&snap, 2025);
    assert!(
        !other.contains("full-return inputs"),
        "a non-refused year must not show the refusal banner; content:\n{other}"
    );
}

// ── [R0-I1] display-only: sorting NEVER mutates events/state ───────────────────

/// Driving every sort/cursor key across all three sortable views (and rendering after each) leaves
/// `snapshot.state` and `snapshot.events` BYTE-IDENTICAL — sorting reorders DISPLAY rows only (it
/// sorts borrows/indices, never the ledger). Regression guard for the display-only invariant.
#[test]
fn sorting_does_not_mutate_events_or_state() {
    let inline_lot = |tag: &str, wprov: &str, acq: (i32, u8, u8), sat: i64, basis: i64| Lot {
        lot_id: make_lot_id(tag),
        wallet: WalletId::Exchange {
            provider: wprov.into(),
            account: "main".into(),
        },
        acquired_at: make_date(acq.0, acq.1, acq.2),
        original_sat: sat,
        remaining_sat: sat,
        usd_basis: Decimal::from(basis),
        basis_source: BasisSource::ExchangeProvided,
        dual_loss_basis: None,
        donor_acquired_at: None,
        basis_pending: false,
        pseudo: false,
    };

    let mut state = LedgerState::default();
    state
        .lots
        .push(inline_lot("l1", "kraken", (2024, 3, 1), 300, 900));
    state
        .lots
        .push(inline_lot("l2", "coinbase", (2022, 1, 1), 100, 500));
    state.disposals.push(make_disposal_tagged(
        "dA", 2025, 50_000_000, "30000.00", "20000.00", "10000.00",
    ));
    state.disposals.push(make_disposal_tagged(
        "dB", 2025, 25_000_000, "15000.00", "8000.00", "7000.00",
    ));
    state.income_recognized.push(make_income_tagged(
        "iA",
        2025,
        1_000_000,
        "600.00",
        IncomeKind::Staking,
    ));
    state.income_recognized.push(make_income_tagged(
        "iB",
        2025,
        2_000_000,
        "700.00",
        IncomeKind::Mining,
    ));

    let mut app = make_app(state, 2025);

    // Snapshot the ledger BEFORE any sorting.
    let events_before = app.snapshot.as_ref().unwrap().events.clone();
    let state_before = app.snapshot.as_ref().unwrap().state.clone();

    // Exercise cursor + sort keys on each sortable view, rendering after each to run the sort path.
    for tab in [
        crate::app::Tab::Holdings,
        crate::app::Tab::Disposals,
        crate::app::Tab::Income,
    ] {
        app.tab = tab;
        for key in [
            KeyCode::Char('l'),
            KeyCode::Char('s'),
            KeyCode::Char('s'), // toggle back
            KeyCode::Char('h'),
            KeyCode::Char('s'),
            KeyCode::Right,
            KeyCode::Left,
        ] {
            crate::handle_key(&mut app, press(key));
            let _ = match tab {
                crate::app::Tab::Holdings => render_holdings(&mut app),
                crate::app::Tab::Disposals => render_disposals(&mut app),
                _ => render_income(&mut app),
            };
        }
    }

    let snap = app.snapshot.as_ref().unwrap();
    assert_eq!(
        snap.events, events_before,
        "sorting must NOT mutate snapshot.events"
    );
    assert!(
        snap.state == state_before,
        "sorting must NOT mutate snapshot.state (display-only)"
    );
}
