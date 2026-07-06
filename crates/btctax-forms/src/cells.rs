//! Money-cell emission shared by the flat forms (Schedule SE / Form 1040 / Form 8283).
//!
//! A [`MoneyCell`] is either a single field (2024/2025) carrying the whole `fmt_money` string, or a
//! **dollars+cents [`MoneyPair`]** (the 2017 forms). For a pair the geometric oracle treats the two
//! widgets as ONE logical cell at the **dollars-field** geometry: the dollars field is the
//! column-x-/descent-checked [`FlatPlacement::cell`]/[`FlatPlacement::col_only`]; the cents field
//! rides along as an authorized-but-geometry-exempt [`FlatPlacement::free`] write. A map that swaps
//! the two members writes the whole-dollars value into the narrow cents widget → its center-x leaves
//! the dollars column cluster → the read-back FAILS CLOSED.

use crate::fmt_money;
use crate::map::MoneyCell;
use crate::pdf::FieldValue;
use crate::verify::FlatPlacement;
use btctax_core::Usd;
use rust_decimal_macros::dec;

/// 0-based page index a field lives on (page 2 = the 2017 §B long Schedule SE / the Rev. 12-2014
/// Section B tables; page 1 otherwise). Derived from the fully-qualified name, exactly as the
/// verifier's own `page_of` — so a placement's page always agrees with its widget.
pub fn page_of(fqn: &str) -> usize {
    if fqn.contains("Page2") {
        1
    } else {
        0
    }
}

/// Split a money amount into (whole-dollars, 2-digit-zero-padded-cents) — the REAL formatter the
/// dollars+cents pairs need (`fmt_money` is the raw `Decimal` Display, which pads no cents). The value
/// is rounded to cents first, so `100 → ("100","00")`, `45500.5 → ("45500","50")`,
/// `100.05 → ("100","05")`.
pub fn fmt_money_pair(d: Usd) -> (String, String) {
    let r = d.round_dp(2);
    let whole = r.trunc();
    let cents = ((r - whole).abs() * dec!(100)).round();
    (whole.trunc().to_string(), format!("{:02}", cents))
}

/// Emit the write(s) + flat placement(s) for a money cell. `descent = Some((group, ordinal))` puts
/// the (dollars) field into a strictly-descending-y sequence; `None` makes it column-only.
pub fn push_money(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cell: &MoneyCell,
    value: Usd,
    col: usize,
    descent: Option<(u32, u32)>,
) {
    match cell {
        MoneyCell::Single(fqn) => {
            w.push((fqn.clone(), FieldValue::Text(fmt_money(value))));
            p.push(geo_placement(fqn, col, descent));
        }
        MoneyCell::Pair(mp) => {
            let (dollars, cents) = fmt_money_pair(value);
            w.push((mp.dollars_field.clone(), FieldValue::Text(dollars)));
            w.push((mp.cents_field.clone(), FieldValue::Text(cents)));
            p.push(geo_placement(&mp.dollars_field, col, descent));
            // The cents widget is authorized (no-unmapped) + page-checked, but not column-checked.
            p.push(FlatPlacement::free(
                mp.cents_field.clone(),
                page_of(&mp.cents_field),
            ));
        }
    }
}

/// Emit a NON-numeric literal (e.g. the "-0-" active-and-netted-to-zero marker) into a money cell's
/// (dollars) field. Column-checked, no cents.
pub fn push_literal(
    w: &mut Vec<(String, FieldValue)>,
    p: &mut Vec<FlatPlacement>,
    cell: &MoneyCell,
    literal: &str,
    col: usize,
) {
    let fqn = match cell {
        MoneyCell::Single(f) => f,
        MoneyCell::Pair(mp) => &mp.dollars_field,
    };
    w.push((fqn.clone(), FieldValue::Text(literal.to_string())));
    p.push(FlatPlacement::col_only(fqn.clone(), page_of(fqn), col));
}

fn geo_placement(fqn: &str, col: usize, descent: Option<(u32, u32)>) -> FlatPlacement {
    let page = page_of(fqn);
    match descent {
        Some((grp, ord)) => FlatPlacement::cell(fqn.to_string(), page, col, grp, ord),
        None => FlatPlacement::col_only(fqn.to_string(), page, col),
    }
}
