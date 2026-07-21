//! KATs for BG-D3 — the `filed_basis` compute + `Coverage::Full` hard-refuse guard (Task 2 of the
//! conservative-filing promotion engine). `filed_basis_for` is a PURE wrapper over `window_reference`
//! (conservative.rs): it turns the window's min daily CLOSE (a per-BTC PRICE) into a whole-tranche basis
//! by scaling `min * sat / SATS_PER_BTC` (the SAME formula `overpayment_delta_one` uses), and REFUSES to
//! produce a floor unless the window has `Coverage::Full` (a `Partial`-covered min can EXCEED the true
//! window min — conservative.rs `window_reference` doc; filing on it would UNDERSTATE a floor). PRIVACY:
//! synthetic values only.

use btctax_core::conservative::Coverage;
use btctax_core::conservative_promote::{filed_basis_for, PromoteRefusal};
use btctax_core::price::StaticPrices;
use rust_decimal_macros::dec;
use time::macros::date;

// ── fixture harness (mirrors tests/kat_tranche.rs / tests/kat_conservative.rs) ─────────────────────

/// A short, FULLY-covered window (2017-12-01..2017-12-03) whose min daily close is `min_price`.
fn prices_with_window_min(min_price: i64) -> StaticPrices {
    StaticPrices(
        [
            (
                date!(2017 - 12 - 01),
                rust_decimal::Decimal::from(min_price),
            ),
            (
                date!(2017 - 12 - 02),
                rust_decimal::Decimal::from(min_price + 3_000),
            ),
            (
                date!(2017 - 12 - 03),
                rust_decimal::Decimal::from(min_price + 5_000),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

/// A window (2013-01-01..2013-01-03) where the middle day has NO bundled close — a gap, so
/// `window_reference` returns `Coverage::Partial` over the covered days.
fn prices_with_partial_window() -> StaticPrices {
    StaticPrices(
        [
            (date!(2013 - 01 - 01), dec!(100)),
            // 2013-01-02 missing — the gap.
            (date!(2013 - 01 - 03), dec!(80)),
        ]
        .into_iter()
        .collect(),
    )
}

/// A window with NO bundled close on any day — `window_reference` returns `None`.
fn prices_with_no_coverage() -> StaticPrices {
    StaticPrices(
        [(date!(2019 - 06 - 01), dec!(9_000))] // outside the queried window
            .into_iter()
            .collect(),
    )
}

/// BG-D3: whole-tranche scaling (per-BTC price × sat / SATS_PER_BTC), NOT a per-BTC price.
#[test]
fn filed_basis_is_whole_tranche_scaled() {
    let prices = prices_with_window_min(12_000); // min daily close = $12,000/BTC, Full coverage
    let cf = filed_basis_for(
        &prices,
        50_000_000, // 0.5 BTC
        date!(2017 - 12 - 01),
        date!(2017 - 12 - 03),
    )
    .unwrap();
    assert_eq!(cf.filed_basis, dec!(6_000)); // 12_000 × 0.5, not 12_000
    assert_eq!(cf.coverage, Coverage::Full);
}

/// BG-D3: a Coverage::Partial window is HARD-refused — never file a floor that could exceed the true
/// window min.
#[test]
fn partial_coverage_is_hard_refused() {
    let prices = prices_with_partial_window(); // 2013-01-02 has no close
    let err = filed_basis_for(
        &prices,
        100_000_000,
        date!(2013 - 01 - 01),
        date!(2013 - 01 - 03),
    )
    .unwrap_err();
    assert!(matches!(err, PromoteRefusal::PartialCoverage));
}

/// BG-D3: a window with NO covered day at all is likewise HARD-refused (`NoCoverage`) — never fabricate
/// a floor over a total data gap.
#[test]
fn no_coverage_is_hard_refused() {
    let prices = prices_with_no_coverage();
    let err = filed_basis_for(
        &prices,
        100_000_000,
        date!(2013 - 01 - 01),
        date!(2013 - 01 - 03),
    )
    .unwrap_err();
    assert!(matches!(err, PromoteRefusal::NoCoverage));
}
