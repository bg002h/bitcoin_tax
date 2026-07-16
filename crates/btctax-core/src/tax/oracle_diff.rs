//! **Test-support** — reproduces btctax's §3.1 printed chain on an independent oracle's figures — the
//! seam the differential tests hold the paper against; see `design/SPEC_oracle_sweep.md` §6.2.
//!
//! No tax logic lives here: each function re-prints an ORACLE's leaves through the SAME rounding and
//! Tax-Table-vs-TCW selection btctax's own filed-return path uses (`tax/method.rs`, `tax/other_taxes.rs`),
//! so a differential test can compare paper-to-paper — the oracle's figures printed btctax's way against
//! btctax's own printed line — instead of comparing a printed dollar to an exact-cents float.

use crate::conventions::{round_dollar, Usd};
use crate::tax::method::{qdcgt_line16, TAX_TABLE_CEILING};
use crate::tax::testonly::ty2024_table;

/// An oracle's finite `f64` figure as exact [`Usd`] — the `golden_usd` convention (a NaN/∞ oracle
/// figure is a generator bug, not a tax case, so it panics rather than silently coercing).
pub fn usd(x: f64) -> Usd {
    Usd::try_from(x).expect("finite oracle figure")
}

/// **Leaf pattern** — a single oracle line printed btctax's way: `round_dollar` at the line
/// (SPEC §3.1). Use for an operand that is transcribed straight onto the paper (e.g. NIIT from
/// Form 8960 line 17).
pub fn round_leaf(oracle_line: f64) -> Usd {
    round_dollar(usd(oracle_line))
}

/// **Cross-footed pattern** — a printed TOTAL is the sum of its already-rounded LEGS, `Σ round_dollar`
/// (SPEC §3.1; the `golden_packet.rs:120-123` line-24 pattern, generalized).
///
/// `components` are the oracle's per-line LEGS (Sch SE L10+L11 → L12; 8959 L7+L13 → L18; the line-24
/// legs L16+SE-L12+8959-L18+NIIT), **never a single exact total** — `sum_round(&[exact_total])`
/// degenerates to `round(exact)` and drops the cross-foot, which is a bug: with legs 274.50 and 499.50
/// the printed total is `275 + 500 = 775`, not `round_dollar(774.00) = 774`.
pub fn sum_round(components: &[f64]) -> Usd {
    components.iter().map(|&c| round_dollar(usd(c))).sum()
}

/// **Rate-on-printed pattern** — a rate applied to an already-PRINTED (rounded) operand, then rounded:
/// `round_dollar(rate * printed_operand)` (SPEC §3.1; Form 8959 lines 7/13, Form 8960 line 17 —
/// `other_taxes.rs`). The operand is the printed line, not the exact-cents value, so this may differ
/// from `round_dollar(rate * exact)` by a dollar — and that is the filed form, not an error.
pub fn rate_on_printed(rate: Usd, printed_operand: Usd) -> Usd {
    round_dollar(rate * printed_operand)
}

/// **Tax-Table pattern** (`Table_btctax`) — the oracle's taxable income run through btctax's own
/// Qualified Dividends & Capital Gain Tax Worksheet → 1040 line 16, whole dollars.
///
/// `ti` = 1040 line 15; `qd_l3a` = 1040 line 3a; `net_ltcg_qd_excl` = the §1(h) net capital gain,
/// QD-EXCLUSIVE (r5-N2). Delegates to [`qdcgt_line16`], which already caps the preferential amount at
/// `min(ti, qd+ltcg)` (Fable F-A) and returns `round_dollar(min(L23, L24))` (Fable F-B) — so a pure
/// ordinary case (`qd = ltcg = 0`) is just the Tax-Table / TCW value on `ti`.
pub fn table_l16(
    status: crate::tax::FilingStatus,
    ti: Usd,
    qd_l3a: Usd,
    net_ltcg_qd_excl: Usd,
) -> Usd {
    let table = ty2024_table();
    let schedule = table
        .ordinary
        .get(&status)
        .expect("TY2024 ordinary schedule for this filing status");
    let bp = table
        .ltcg
        .get(&status)
        .expect("TY2024 §1(h) breakpoints for this filing status");
    qdcgt_line16(schedule, bp, ti, qd_l3a, net_ltcg_qd_excl)
}

/// Whether btctax's line-16 worksheet CONSULTED the IRS Tax Table for this household (r3-I1
/// methodology): true iff any worksheet operand is `< TAX_TABLE_CEILING` — the ordinary remainder
/// `L5 = max(0, ti − (qd+ltcg))` (worksheet line 22's operand) OR the full `ti` (line 24's operand).
/// Computed from the SAME operands [`qdcgt_line16`] consumes (`method.rs:83-89`); `worksheet_tax`
/// reads the Tax Table iff its operand `< TAX_TABLE_CEILING` (`method.rs:49`).
///
/// `status` is part of the §6.2 predicate signature (symmetry with [`table_l16`]); the schedule does
/// not affect WHICH worksheet is consulted — that is a pure magnitude test against the ceiling.
pub fn consulted_table(
    status: crate::tax::FilingStatus,
    ti: Usd,
    qd_l3a: Usd,
    net_ltcg_qd_excl: Usd,
) -> bool {
    let _ = status;
    let z = Usd::ZERO;
    let ti = ti.max(z);
    let pref_full = qd_l3a.max(z) + net_ltcg_qd_excl.max(z); // L4 = L2 + L3
    let l5 = (ti - pref_full).max(z); // L5 = max(0, L1 − L4) — line 22's operand
    l5 < TAX_TABLE_CEILING || ti < TAX_TABLE_CEILING
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax::FilingStatus; // the re-export (N2), not crate::tax::return_inputs::
    use rust_decimal_macros::dec;

    // ★ C1: sum_round is Σround(legs), NOT round(exact_total). Pin the cross-foot with a SYNTHETIC
    // flipping pair (mirrors the repo's own KAT, other_taxes.rs:80-85): 274.50 → 275, 499.50 → 500,
    // so the legs sum to 775, but round(774.00) = 774. The legs are NOT baked until T11 and no anchor
    // flips anyway, so this uses literals — do NOT read the JSON here (r2-I1).
    #[test]
    fn sum_round_cross_foots_the_legs_not_the_exact_total() {
        assert_eq!(sum_round(&[274.50, 499.50]), dec!(775)); // round(274.50)+round(499.50)
        assert_ne!(sum_round(&[274.50, 499.50]), round_leaf(274.50 + 499.50)); // ≠ round(774.00) = 774 — the whole point
    }

    // Table_btctax reproduces OTS's exact-cents L16 at whole dollars for the above-ceiling SE anchor.
    #[test]
    fn table_l16_reproduces_ots_above_ceiling() {
        // mfj_se_over_the_addl_medicare_threshold: OTS TI 253_942.94, L16 47_031.31 (baked).
        let got = table_l16(FilingStatus::Mfj, usd(253_942.94), usd(0.0), usd(0.0));
        assert_eq!(got, dec!(47031)); // round_dollar(47_031.31) — no preferential income ⇒ pure TCW
    }

    // consulted_table: true when the ordinary remainder is below the ceiling (single_qdcgt_both_slices),
    // false when every operand is at/above it (a pure high-TI ordinary household).
    #[test]
    fn consulted_table_tracks_the_worksheet_operands() {
        // TI 112_400, QD 8_000, net-LTCG(qd-excl) 25_000 ⇒ remainder 79_400 < ceiling ⇒ true.
        assert!(consulted_table(FilingStatus::Single, usd(112_400.0), usd(8_000.0), usd(25_000.0)));
        // TI 253_943, no preferential ⇒ remainder = TI ≥ ceiling ⇒ false.
        assert!(!consulted_table(FilingStatus::Mfj, usd(253_943.0), usd(0.0), usd(0.0)));
    }
}
