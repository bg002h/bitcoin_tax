//! P2-B Task 2 + Task 3 KATs — Form 8949 rows (`form_8949`) + Schedule D part totals (`schedule_d`).
//!
//! Both builders are pure, year-scoped projections over `state.disposals`. These KATs drive them
//! with DIRECT-STATE `DisposalLeg`/`Disposal` fixtures (mirroring `tax_compute.rs`) so ST/LT term,
//! gift zone, wallet, acquired/sold dates, multi-leg disposals, and multi-year scenarios are all
//! under exact control — a real projection cannot place an LT disposition and an ST disposition in
//! the same tax year without the §7.4 transition seed, which is orthogonal to the output projection
//! under test. PRIVACY: synthetic values only; no real user file is read (NFR).
use btctax_core::conventions::{TaxDate, Usd};
use btctax_core::event::{BasisSource, DisposeKind};
use btctax_core::forms::{form_8949, schedule_d, Form8949Box, Form8949Part};
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, GiftZone, LedgerState, Term};
use rust_decimal_macros::dec;
use time::macros::date;

// ── direct-state builders ────────────────────────────────────────────────────────────────────────
fn exch() -> WalletId {
    WalletId::Exchange {
        provider: "cb".into(),
        account: "m".into(),
    }
}
fn cold() -> WalletId {
    WalletId::SelfCustody {
        label: "cold".into(),
    }
}
fn lot(origin_seq: u64, split: u32) -> LotId {
    LotId {
        origin_event_id: EventId::decision(origin_seq),
        split_sequence: split,
    }
}
/// A baseline ST exchange leg (100 sat, all-zero money, acquired 2025-01-01). Tests override fields
/// via struct-update syntax, e.g. `DisposalLeg { term: Term::LongTerm, ..base_leg() }`.
fn base_leg() -> DisposalLeg {
    DisposalLeg {
        lot_id: lot(0, 0),
        sat: 100,
        proceeds: dec!(0),
        basis: dec!(0),
        gain: dec!(0),
        term: Term::ShortTerm,
        basis_source: BasisSource::ComputedFromCost,
        gift_zone: None,
        acquired_at: date!(2025 - 01 - 01),
        wallet: exch(),
    }
}
fn disposal(seq: u64, disposed_at: TaxDate, kind: DisposeKind, legs: Vec<DisposalLeg>) -> Disposal {
    Disposal {
        event: EventId::decision(seq),
        kind,
        disposed_at,
        legs,
        fee_mini_disposition: false,
    }
}
fn state(disposals: Vec<Disposal>) -> LedgerState {
    LedgerState {
        disposals,
        ..Default::default()
    }
}

// ── Task 2 — Form 8949 rows ────────────────────────────────────────────────────────────────────

/// ST leg → Part I / box C; LT leg → Part II / box F.
#[test]
fn st_leg_is_part_i_box_c_and_lt_leg_is_part_ii_box_f() {
    let st = state(vec![
        disposal(
            1,
            date!(2025 - 03 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                term: Term::ShortTerm,
                ..base_leg()
            }],
        ),
        disposal(
            2,
            date!(2025 - 04 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                term: Term::LongTerm,
                ..base_leg()
            }],
        ),
    ]);
    let rows = form_8949(&st, 2025);
    assert_eq!(rows.len(), 2);
    let st_row = rows
        .iter()
        .find(|r| r.part == Form8949Part::ShortTerm)
        .unwrap();
    let lt_row = rows
        .iter()
        .find(|r| r.part == Form8949Part::LongTerm)
        .unwrap();
    assert_eq!(st_row.box_, Form8949Box::C);
    assert_eq!(lt_row.box_, Form8949Box::F);
}

/// Column (a) description is the EXACT BTC amount, 8dp + " BTC" — "0.53000000 BTC" (never a float).
#[test]
fn description_is_exact_btc_amount_8dp() {
    let st = state(vec![disposal(
        1,
        date!(2025 - 06 - 01),
        DisposeKind::Sell,
        vec![DisposalLeg {
            sat: 53_000_000,
            ..base_leg()
        }],
    )]);
    let rows = form_8949(&st, 2025);
    assert_eq!(rows[0].description, "0.53000000 BTC");
}

/// Dates / proceeds / basis / gain on the row match the leg (and the disposal's disposed_at).
#[test]
fn row_fields_match_the_leg() {
    let st = state(vec![disposal(
        1,
        date!(2025 - 09 - 15),
        DisposeKind::Spend,
        vec![DisposalLeg {
            proceeds: dec!(100.50),
            basis: dec!(60.00),
            gain: dec!(40.50),
            acquired_at: date!(2024 - 02 - 20),
            term: Term::LongTerm,
            ..base_leg()
        }],
    )]);
    let r = &form_8949(&st, 2025)[0];
    assert_eq!(r.date_acquired, date!(2024 - 02 - 20));
    assert_eq!(r.date_sold, date!(2025 - 09 - 15));
    assert_eq!(r.proceeds, dec!(100.50));
    assert_eq!(r.cost_basis, dec!(60.00));
    assert_eq!(r.gain, dec!(40.50));
    assert_eq!(r.disposition_kind, DisposeKind::Spend);
    // adjustment columns are always blank / zero (no §1091, no other adjustments).
    assert_eq!(r.adjustment_code, "");
    assert_eq!(r.adjustment_amount, Usd::ZERO);
}

/// A multi-leg disposal spanning ST + LT → two rows in the correct parts. Also proves within-disposal
/// ordering is by `lot_id` (NOT by term): the legs are pushed ST-then-LT but the LT leg has the
/// lower lot split, so it must come out first.
#[test]
fn multi_leg_disposal_spanning_st_and_lt_yields_two_rows_ordered_by_lot() {
    let st = state(vec![disposal(
        1,
        date!(2025 - 05 - 01),
        DisposeKind::Sell,
        vec![
            // pushed first, but higher lot split (0,1) → must sort AFTER the LT leg below.
            DisposalLeg {
                lot_id: lot(0, 1),
                term: Term::ShortTerm,
                gain: dec!(5),
                ..base_leg()
            },
            // pushed second, lower lot split (0,0) → must sort FIRST.
            DisposalLeg {
                lot_id: lot(0, 0),
                term: Term::LongTerm,
                gain: dec!(9),
                ..base_leg()
            },
        ],
    )]);
    let rows = form_8949(&st, 2025);
    assert_eq!(rows.len(), 2);
    // lot (0,0) < (0,1) → LT leg first, ST leg second (ordering is by lot_id, not term).
    assert_eq!(rows[0].part, Form8949Part::LongTerm);
    assert_eq!(rows[0].gain, dec!(9));
    assert_eq!(rows[1].part, Form8949Part::ShortTerm);
    assert_eq!(rows[1].gain, dec!(5));
}

/// A NoGainNoLoss dual-basis gift-zone leg → row present, gain 0, adjustment columns blank. The fold
/// set basis == proceeds for that zone, so the row is internally consistent with no special code.
#[test]
fn no_gain_no_loss_gift_leg_row_present_with_zero_gain_and_blank_adjustments() {
    let st = state(vec![disposal(
        1,
        date!(2025 - 08 - 01),
        DisposeKind::Sell,
        vec![DisposalLeg {
            proceeds: dec!(80.00),
            basis: dec!(80.00), // NGNL: basis == proceeds → gain 0
            gain: dec!(0),
            gift_zone: Some(GiftZone::NoGainNoLoss),
            ..base_leg()
        }],
    )]);
    let rows = form_8949(&st, 2025);
    assert_eq!(rows.len(), 1, "the NGNL disposition IS a reported row");
    assert_eq!(rows[0].gain, Usd::ZERO);
    assert_eq!(rows[0].proceeds, rows[0].cost_basis);
    assert_eq!(rows[0].adjustment_code, "");
    assert_eq!(rows[0].adjustment_amount, Usd::ZERO);
}

/// Year filter: a prior-year (and a future-year) disposal is excluded from the year's rows.
#[test]
fn year_filter_excludes_out_of_year_disposals() {
    let st = state(vec![
        disposal(
            1,
            date!(2024 - 12 - 31),
            DisposeKind::Sell,
            vec![base_leg()],
        ),
        disposal(
            2,
            date!(2025 - 06 - 01),
            DisposeKind::Sell,
            vec![base_leg()],
        ),
        disposal(
            3,
            date!(2026 - 01 - 01),
            DisposeKind::Sell,
            vec![base_leg()],
        ),
    ]);
    let rows = form_8949(&st, 2025);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].date_sold, date!(2025 - 06 - 01));
}

/// Deterministic ordering: rows are sorted by (disposed_at, event id, lot_id), regardless of the
/// disposal push order. Two disposals share a date → tie broken by event id.
#[test]
fn deterministic_ordering_by_date_then_event_then_lot() {
    // Pushed scrambled: 06-01(dec2), 03-01(dec1), 03-01(dec3). Distinct sat → identify each row.
    let st = state(vec![
        disposal(
            2,
            date!(2025 - 06 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                sat: 300_000_000, // "3.00000000 BTC"
                ..base_leg()
            }],
        ),
        disposal(
            1,
            date!(2025 - 03 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                sat: 100_000_000, // "1.00000000 BTC"
                ..base_leg()
            }],
        ),
        disposal(
            3,
            date!(2025 - 03 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                sat: 200_000_000, // "2.00000000 BTC"
                ..base_leg()
            }],
        ),
    ]);
    let rows = form_8949(&st, 2025);
    let order: Vec<&str> = rows.iter().map(|r| r.description.as_str()).collect();
    // 03-01/dec1 (1 BTC), then 03-01/dec3 (2 BTC), then 06-01/dec2 (3 BTC).
    assert_eq!(
        order,
        vec!["1.00000000 BTC", "2.00000000 BTC", "3.00000000 BTC"]
    );
}

/// Box needs review: an EXCHANGE-wallet disposition → `box_needs_review == true` (may carry a
/// 1099-B/1099-DA); a self-custody disposition → `false`. Box stays the C/F default in both cases.
#[test]
fn exchange_wallet_flags_box_needs_review_self_custody_does_not() {
    let st = state(vec![
        disposal(
            1,
            date!(2025 - 03 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                lot_id: lot(1, 0),
                wallet: exch(),
                ..base_leg()
            }],
        ),
        disposal(
            2,
            date!(2025 - 04 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                lot_id: lot(2, 0),
                wallet: cold(),
                ..base_leg()
            }],
        ),
    ]);
    let rows = form_8949(&st, 2025);
    let ex = rows.iter().find(|r| r.wallet == exch()).unwrap();
    let sc = rows.iter().find(|r| r.wallet == cold()).unwrap();
    assert!(ex.box_needs_review, "exchange disposition must be flagged");
    assert!(!sc.box_needs_review, "self-custody must NOT be flagged");
    // box is still the conservative C default (never auto-assigned to A/B) regardless of the flag.
    assert_eq!(ex.box_, Form8949Box::C);
    assert_eq!(sc.box_, Form8949Box::C);
}

// ── Task 3 — Schedule D part totals ────────────────────────────────────────────────────────────

/// Hand-derived mixed golden: ST Σ and LT Σ (proceeds/basis/gain) over the year's legs, including a
/// signed LT loss leg, with an out-of-year disposal that must be excluded.
#[test]
fn schedule_d_part_totals_hand_derived_golden() {
    let st = state(vec![
        // 2024 — must be excluded from the 2025 totals.
        disposal(
            9,
            date!(2024 - 12 - 31),
            DisposeKind::Sell,
            vec![DisposalLeg {
                proceeds: dec!(999),
                basis: dec!(1),
                gain: dec!(998),
                term: Term::ShortTerm,
                ..base_leg()
            }],
        ),
        // 2025 ST legs.
        disposal(
            1,
            date!(2025 - 03 - 01),
            DisposeKind::Sell,
            vec![
                DisposalLeg {
                    proceeds: dec!(100.00),
                    basis: dec!(60.00),
                    gain: dec!(40.00),
                    term: Term::ShortTerm,
                    ..base_leg()
                },
                DisposalLeg {
                    lot_id: lot(0, 1),
                    proceeds: dec!(50.00),
                    basis: dec!(30.00),
                    gain: dec!(20.00),
                    term: Term::ShortTerm,
                    ..base_leg()
                },
            ],
        ),
        // 2025 LT legs (one gain, one loss → signed sum).
        disposal(
            2,
            date!(2025 - 07 - 01),
            DisposeKind::Sell,
            vec![
                DisposalLeg {
                    proceeds: dec!(200.00),
                    basis: dec!(150.00),
                    gain: dec!(50.00),
                    term: Term::LongTerm,
                    ..base_leg()
                },
                DisposalLeg {
                    lot_id: lot(0, 1),
                    proceeds: dec!(10.00),
                    basis: dec!(40.00),
                    gain: dec!(-30.00),
                    term: Term::LongTerm,
                    ..base_leg()
                },
            ],
        ),
    ]);
    let sd = schedule_d(&st, 2025);
    // ST: proceeds 150, basis 90, gain 60.
    assert_eq!(sd.st.proceeds, dec!(150.00));
    assert_eq!(sd.st.cost_basis, dec!(90.00));
    assert_eq!(sd.st.gain, dec!(60.00));
    // LT: proceeds 210, basis 190, gain 20 (50 + (−30)).
    assert_eq!(sd.lt.proceeds, dec!(210.00));
    assert_eq!(sd.lt.cost_basis, dec!(190.00));
    assert_eq!(sd.lt.gain, dec!(20.00));
}

/// Year filter: only the year's legs are summed (mirrors the Form 8949 year filter).
#[test]
fn schedule_d_year_filter() {
    let st = state(vec![
        disposal(
            1,
            date!(2024 - 06 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                proceeds: dec!(500),
                basis: dec!(100),
                gain: dec!(400),
                term: Term::ShortTerm,
                ..base_leg()
            }],
        ),
        disposal(
            2,
            date!(2025 - 06 - 01),
            DisposeKind::Sell,
            vec![DisposalLeg {
                proceeds: dec!(70),
                basis: dec!(30),
                gain: dec!(40),
                term: Term::ShortTerm,
                ..base_leg()
            }],
        ),
    ]);
    let sd = schedule_d(&st, 2025);
    assert_eq!(sd.st.gain, dec!(40)); // only the 2025 leg
    assert_eq!(sd.st.proceeds, dec!(70));
}

/// An empty year (no in-year disposals) → all-zero part totals.
#[test]
fn schedule_d_empty_year_is_all_zero() {
    let st = state(vec![disposal(
        1,
        date!(2024 - 06 - 01),
        DisposeKind::Sell,
        vec![DisposalLeg {
            gain: dec!(400),
            ..base_leg()
        }],
    )]);
    let sd = schedule_d(&st, 2099);
    assert_eq!(sd.st.proceeds, Usd::ZERO);
    assert_eq!(sd.st.cost_basis, Usd::ZERO);
    assert_eq!(sd.st.gain, Usd::ZERO);
    assert_eq!(sd.lt.proceeds, Usd::ZERO);
    assert_eq!(sd.lt.cost_basis, Usd::ZERO);
    assert_eq!(sd.lt.gain, Usd::ZERO);
}

/// Form 8949 rows and Schedule D part totals agree by construction: Σ of the year's ST row gains ==
/// `schedule_d(..).st.gain` (and likewise LT). A consistency cross-check between the two builders.
#[test]
fn form_8949_rows_aggregate_to_schedule_d_totals() {
    let st = state(vec![disposal(
        1,
        date!(2025 - 03 - 01),
        DisposeKind::Sell,
        vec![
            DisposalLeg {
                lot_id: lot(0, 0),
                proceeds: dec!(100),
                basis: dec!(60),
                gain: dec!(40),
                term: Term::ShortTerm,
                ..base_leg()
            },
            DisposalLeg {
                lot_id: lot(0, 1),
                proceeds: dec!(200),
                basis: dec!(150),
                gain: dec!(50),
                term: Term::LongTerm,
                ..base_leg()
            },
        ],
    )]);
    let rows = form_8949(&st, 2025);
    let sd = schedule_d(&st, 2025);
    let st_gain: Usd = rows
        .iter()
        .filter(|r| r.part == Form8949Part::ShortTerm)
        .map(|r| r.gain)
        .sum();
    let lt_gain: Usd = rows
        .iter()
        .filter(|r| r.part == Form8949Part::LongTerm)
        .map(|r| r.gain)
        .sum();
    assert_eq!(st_gain, sd.st.gain);
    assert_eq!(lt_gain, sd.lt.gain);
}
