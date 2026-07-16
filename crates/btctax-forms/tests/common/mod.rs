//! Shared synthetic fixtures for the btctax-forms KATs. No real user file is ever read (NFR);
//! everything is built inline from invented values.
#![allow(dead_code)] // each integration-test crate uses a different subset of these helpers

use btctax_core::tax::packet::assemble_printed_return;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{
    build_golden_household, ty2024_params, ty2024_table, GoldenHousehold,
};
use btctax_core::{
    Form8949Box, Form8949Part, Form8949Row, ScheduleDPart, ScheduleDTotals, WalletId,
};
use btctax_forms::{fill_full_return, NamedForm};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;
use time::macros::date;

/// Build a synthetic Form 8949 row (gain = proceeds − cost; no adjustment).
pub fn row(
    part: Form8949Part,
    desc: &str,
    proceeds: Decimal,
    cost: Decimal,
    exchange: bool,
) -> Form8949Row {
    Form8949Row {
        part,
        // The core taxonomy is C/F; the forms crate must map to Box I/L regardless.
        box_: if part == Form8949Part::ShortTerm {
            Form8949Box::C
        } else {
            Form8949Box::F
        },
        box_needs_review: exchange,
        description: desc.to_string(),
        date_acquired: date!(2024 - 02 - 03),
        date_sold: date!(2025 - 06 - 15),
        proceeds,
        cost_basis: cost,
        adjustment_code: String::new(),
        adjustment_amount: dec!(0),
        gain: proceeds - cost,
        wallet: if exchange {
            WalletId::Exchange {
                provider: "acme".into(),
                account: "a1".into(),
            }
        } else {
            WalletId::SelfCustody {
                label: "cold".into(),
            }
        },
        disposition_kind: btctax_core::DisposeKind::Sell,
    }
}

/// A canonical mixed fixture: 2 short-term + 1 long-term row.
pub fn mixed_rows() -> Vec<Form8949Row> {
    vec![
        row(
            Form8949Part::ShortTerm,
            "0.53000000 BTC",
            dec!(30000.50),
            dec!(25000),
            false,
        ),
        row(
            Form8949Part::ShortTerm,
            "0.10000000 BTC",
            dec!(6000),
            dec!(5500),
            true,
        ),
        row(
            Form8949Part::LongTerm,
            "1.00000000 BTC",
            dec!(60000),
            dec!(20000),
            false,
        ),
    ]
}

/// Sum a slice of rows of one part into a Schedule D part total (mirrors `btctax_core::schedule_d`).
pub fn sum_part(rows: &[Form8949Row], part: Form8949Part) -> ScheduleDPart {
    let mut p = ScheduleDPart::default();
    for r in rows.iter().filter(|r| r.part == part) {
        p.proceeds += r.proceeds;
        p.cost_basis += r.cost_basis;
        p.gain += r.gain;
    }
    p
}

/// The Schedule D totals consistent with a set of Form 8949 rows.
pub fn totals_for(rows: &[Form8949Row]) -> ScheduleDTotals {
    ScheduleDTotals {
        st: sum_part(rows, Form8949Part::ShortTerm),
        lt: sum_part(rows, Form8949Part::LongTerm),
    }
}

// ══════════════════ the golden packet — ONE builder, shared by every consumer ═════════════════════
//
// Both `golden_packet.rs` (the P7 round-trip) and `oracle_sweep_readback.rs` (T4) fill the SAME twelve
// households the oracles blessed. A second copy of this builder could drift, and a drifted round-trip
// would fill forms for a different taxpayer than the one the oracle validated — while still passing.
// One fixture, one packet: keep it here and let both integration-test crates share it via `mod common;`.

/// Fill the whole packet for one golden household — the exact chain the return itself takes:
/// `assemble_absolute` → `assemble_printed_return` → `fill_full_return`.
pub fn packet(h: &GoldenHousehold) -> Vec<NamedForm> {
    let (ri, state) = build_golden_household(h);
    let params = ty2024_params();
    let table = ty2024_table();
    let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
    // No golden household makes a charitable donation, so there are no §170(e) details to carry.
    let pr = assemble_printed_return(&ri, &state, &BTreeMap::new(), &ar, &table, 2024)
        .expect("the golden households carry well-formed SSNs");
    fill_full_return(&pr, 2024).unwrap_or_else(|e| panic!("{}: the packet must fill — {e}", h.name))
}

/// The named form in a filled packet (panics if the packet is missing it).
pub fn form<'a>(pkt: &'a [NamedForm], name: &str) -> &'a NamedForm {
    pkt.iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("the packet is missing {name}"))
}

// ══════════════════ on-paper read-back — the sign table and blank regimes (§6.3) ══════════════════
//
// The double-oracle sweep reads a FILLED PDF and must recover the SIGNED integer each cell represents.
// The paper does not carry signed integers uniformly: some cells lead with a minus, some are
// pre-printed parenthesized boxes whose bare magnitude MEANS a negative. `Sign` names which convention
// a cell uses; `Blank` names how an empty/zero cell is to be read.

/// How a filled money cell encodes its sign (§6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// A leading-minus cell — the string carries its own sign (1040 line 7, `printed.rs:387-390`).
    Leading,
    /// A pre-printed PARENTHESIZED box: the cell holds a POSITIVE MAGNITUDE that means a negative;
    /// negate it (Schedule D lines 6/14/21).
    ParenMagnitude,
    /// No sign convention — read the value as written.
    Unsigned,
}

/// Parse a filled cell to the SIGNED integer it represents, per its `Sign` convention (§6.3).
///
/// An ABSENT key is `None` (the line is not on this return). A key that is PRESENT but does not parse
/// as an integer is a bug in the filler or the map, and PANICS with the raw string — parse discipline
/// forbids silently swallowing garbage as `None`.
pub fn on_paper_signed(cells: &BTreeMap<String, String>, key: &str, sign: Sign) -> Option<i64> {
    let raw = cells.get(key)?;
    let value: i64 = raw.parse().unwrap_or_else(|_| {
        panic!("on_paper_signed: cell {key:?} is present but not a parseable integer: {raw:?}")
    });
    Some(match sign {
        // A leading-minus cell already carries its sign; an unsigned cell has none to apply.
        Sign::Leading | Sign::Unsigned => value,
        // The parentheses on the form ARE the minus sign, so the magnitude on paper is negated.
        Sign::ParenMagnitude => -value,
    })
}

/// How a "blank" cell is to be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blank {
    /// The line MUST be present and printed as `"0"` — dropped-line detection (`golden_packet.rs`
    /// asserts line 17 / line 21 this way before cross-footing line 24). An absent or non-zero cell
    /// is a defect and PANICS; a valid cell yields 0.
    PresentZero,
    /// An absent key reads as 0 (the line is not on this return); a present key reads as its value.
    AbsentIsZero,
}

/// Read a cell that is expected to be blank/zero, under an explicit `Blank` regime.
pub fn cell_or_zero(cells: &BTreeMap<String, String>, key: &str, regime: Blank) -> i64 {
    match regime {
        Blank::PresentZero => {
            let raw = cells.get(key).unwrap_or_else(|| {
                panic!(
                    "cell_or_zero: PresentZero requires {key:?} to be present-and-\"0\", but it is \
                     absent — the filler stopped writing the line this guard depends on"
                )
            });
            assert_eq!(
                raw, "0",
                "cell_or_zero: PresentZero requires {key:?} == \"0\", got {raw:?}"
            );
            0
        }
        Blank::AbsentIsZero => match cells.get(key) {
            None => 0,
            Some(raw) => raw.parse().unwrap_or_else(|_| {
                panic!("cell_or_zero: cell {key:?} is present but not a parseable integer: {raw:?}")
            }),
        },
    }
}
