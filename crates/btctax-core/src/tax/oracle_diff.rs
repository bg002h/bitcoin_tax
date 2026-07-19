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
use std::collections::BTreeSet;

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

// ─── Divergence-class machinery (§6.2/§6.4, the intricate core) ───────────────────────────────────
//
// When btctax's figure on a line disagrees with an oracle, the disagreement is only tolerated if it
// falls into a *named, lawful class* — a methodology difference we expect, or a per-oracle provenance
// difference we can independently witness on that oracle's own leaves. Anything else is btctax alone
// against the world, which is the exact shape of a confidently-wrong engine, and it fails.

/// Which independent oracle a leaf/figure came from. OTS carries only a provenance class; taxcalc
/// carries both a methodology class (Tax-Table vs schedule) and a provenance class.
///
/// Forward interface: this tag is CONSUMED at the compute/paper levels in T5/T6 (which oracle a leaf
/// belongs to when wiring `stacking_ok` into the differential sweep); it is not yet read at T3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleId {
    /// OpenTaxSolver.
    Ots,
    /// Tax-Calculator (PSL `taxcalc`).
    Taxcalc,
}

/// An oracle's *own* exact §1(h) line-16 leaves (not btctax's) — `ti` = 1040 L15, `qd_l3a` = L3a,
/// `net_ltcg_qd_excl` = the §1(h) net capital gain QD-exclusive. Uniform arity so the four fields feed
/// [`table_l16`]/[`consulted_table`] directly. Reproducing an oracle's L16 means running *these* leaves
/// through btctax's own worksheet — a paper-to-paper comparison, never a printed dollar vs an exact float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct L16Operands {
    /// 1040 filing status.
    pub status: crate::tax::FilingStatus,
    /// 1040 line 15 (taxable income).
    pub ti: Usd,
    /// 1040 line 3a (qualified dividends).
    pub qd_l3a: Usd,
    /// The §1(h) net capital gain, QD-exclusive.
    pub net_ltcg_qd_excl: Usd,
}

/// **taxcalc methodology class** (r3-I1) — the *condition* under which a btctax-vs-taxcalc line-16
/// disagreement is an expected Tax-Table-vs-schedule methodology difference: btctax's worksheet
/// CONSULTED the IRS Tax Table on these operands while taxcalc always uses the continuous schedule.
///
/// Condition-only — **no value check** (r4-N1 declined: a value check under-absorbs mixed
/// methodology+provenance households; the OTS provenance conjunct under [`stacking_ok`] is the backstop
/// against a genuine below-ceiling taxcalc bug — if taxcalc were wrong and OTS right, OTS would dissent
/// from btctax and its provenance conjunct-1 would fail, re-opening the line).
pub fn taxcalc_methodology_class(reproduced_ops: &L16Operands) -> bool {
    consulted_table(
        reproduced_ops.status,
        reproduced_ops.ti,
        reproduced_ops.qd_l3a,
        reproduced_ops.net_ltcg_qd_excl,
    )
}

/// **Per-oracle provenance class** (§6.2(b), r4-I1) — fires iff btctax's OWN Tax-Table lookup, run on
/// the *oracle's* leaves, reproduces the oracle's printed L16 (`table_l16(oracle_ops) ==
/// round_leaf(oracle_l16)` — the falsifiable witness: a real `Table_btctax` semantics bug fails this
/// conjunct and stays red) AND does *not* reproduce it on btctax's own leaves (`table_l16(reproduced_ops)
/// != round_leaf(oracle_l16)` — so the class only fires where the two genuinely diverge).
///
/// `oracle_ops == None` (the oracle's L16 leaves are not yet baked, i.e. pre-T11) ⇒ **`false`, the class
/// CANNOT fire** (M4). A default of `true` would make the anti-world guard vacuous — this `None` arm is
/// the named mutation-check target.
pub fn provenance_class_fires(
    oracle_ops: Option<&L16Operands>,
    reproduced_ops: &L16Operands,
    oracle_l16: f64,
) -> bool {
    let Some(o) = oracle_ops else {
        return false; // M4: leaves not baked ⇒ the class cannot fire (never vacuously true).
    };
    let oracle_printed = round_leaf(oracle_l16);
    // Conjunct-1 (the witness): btctax's lookup reproduces the oracle's L16 on the oracle's own leaves.
    table_l16(o.status, o.ti, o.qd_l3a, o.net_ltcg_qd_excl) == oracle_printed
        // Conjunct-2: but NOT on btctax's own leaves — otherwise there is nothing to explain.
        && table_l16(
            reproduced_ops.status,
            reproduced_ops.ti,
            reproduced_ops.qd_l3a,
            reproduced_ops.net_ltcg_qd_excl,
        ) != oracle_printed
}

/// A caught btctax bug, pinned against an open `FOLLOWUPS.md` id (§10, user-mandated) — the ONE
/// sanctioned way a both-oracle disagreement passes without a lawful class. Separate, loudly-named
/// category; never a class. A stale pin (btctax's value moved — bug fixed or changed) fails
/// [`stacking_ok`], forcing the entry's removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownDefect {
    /// The open `FOLLOWUPS.md` id this wrong value is pinned against.
    pub fu_id: &'static str,
    /// btctax's current (WRONG) value on the line — the guard passes only while `figure` still equals it.
    pub btctax_value: Usd,
}

/// **The guard, in class form** (r3-I2a) — replaces the old `agrees_with:"neither"` + `outlier_alt`
/// stack (`golden_returns.rs:358-372`). `figure` is btctax's value on the line (the on-paper value at
/// the paper level, or the compute figure at the compute level). If `figure` agrees with an oracle,
/// that oracle needs no class; a both-oracle disagreement passes **only when each dissenting oracle's
/// diff independently matches its own class** — taxcalc: methodology OR its provenance; OTS: its
/// provenance. `taxcalc_l16 == None` means taxcalc reports no comparable figure on this line, so it
/// does not dissent and needs no class.
///
/// The ONE sanctioned exception (§10): a declared `KnownDefect` is AUTHORITATIVE for this
/// `(household, line)` — evaluated BEFORE the class path, it passes iff btctax still prints the pinned
/// WRONG value. A stale pin FAILS in BOTH directions — the bug is fixed (`figure` moves to the correct
/// value ≠ `btctax_value`) OR the wrong value changed — forcing the entry's removal; it never lingers
/// green re-armed to silently re-absorb a same-value regression. Once the pin is deleted the class path
/// catches any real regression. A known-defect is a SEPARATE category, never a lawful class.
#[allow(clippy::too_many_arguments)]
pub fn stacking_ok(
    figure: Usd,
    ots_l16: f64,
    taxcalc_l16: Option<f64>,
    ots_ops: Option<&L16Operands>,
    taxcalc_ops: Option<&L16Operands>,
    reproduced_ops: &L16Operands,
    known_defect: Option<&KnownDefect>,
) -> bool {
    // §10, authoritative: a declared pin decides the line on its own — it passes iff btctax still prints
    // the pinned WRONG value. Evaluated BEFORE the class path so a FIXED bug (figure now agrees with the
    // oracles ≠ btctax_value) does not slip through `ots_ok && taxcalc_ok` and leave the pin lingering.
    if let Some(kd) = known_defect {
        return figure == kd.btctax_value;
    }
    // OTS: agree, or its provenance class explains the dissent (OTS has no methodology class).
    let ots_ok =
        figure == round_leaf(ots_l16) || provenance_class_fires(ots_ops, reproduced_ops, ots_l16);
    // taxcalc: no opinion ⇒ not dissenting; else agree, or its methodology OR provenance class explains it.
    let taxcalc_ok = match taxcalc_l16 {
        None => true,
        Some(v) => {
            figure == round_leaf(v)
                || taxcalc_methodology_class(reproduced_ops)
                || provenance_class_fires(taxcalc_ops, reproduced_ops, v)
        }
    };
    ots_ok && taxcalc_ok
}

/// **Class-liveness ledger** (r3-I2b) — the predicate analogue of the never-fired sweep at
/// `golden_returns.rs:388-401`. A declared divergence class that never `fired` and is not held by a
/// `pinned` §5.1 cell is DEAD: it explains a disagreement that no longer happens and is now just an
/// unread claim about the tax code, to be deleted.
#[derive(Debug, Clone, Default)]
pub struct LivenessLedger {
    fired: BTreeSet<&'static str>,
    pinned: BTreeSet<&'static str>,
}

impl LivenessLedger {
    /// Record that `class` fired (a real household exercised it this run).
    pub fn record_fire(&mut self, class: &'static str) {
        self.fired.insert(class);
    }

    /// Record that `class` is held alive by a pinned §5.1 cell (kept without needing to fire).
    pub fn declare_pinned(&mut self, class: &'static str) {
        self.pinned.insert(class);
    }

    /// The `declared` classes that are neither fired nor pinned — the dead ones (N4). Preserves the
    /// caller's declaration order.
    pub fn dead(&self, declared: &[&'static str]) -> Vec<&'static str> {
        declared
            .iter()
            .copied()
            .filter(|c| !self.fired.contains(c) && !self.pinned.contains(c))
            .collect()
    }
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
        assert!(consulted_table(
            FilingStatus::Single,
            usd(112_400.0),
            usd(8_000.0),
            usd(25_000.0)
        ));
        // TI 253_943, no preferential ⇒ remainder = TI ≥ ceiling ⇒ false.
        assert!(!consulted_table(
            FilingStatus::Mfj,
            usd(253_943.0),
            usd(0.0),
            usd(0.0)
        ));
    }

    // taxcalc methodology class fires on single_qdcgt_both_slices (remainder below ceiling), the anchor
    // the old "TI < $100k" gloss wrongly excluded (r3-I1).
    #[test]
    fn methodology_class_fires_on_qdcgt_both_slices() {
        let ops = L16Operands {
            status: FilingStatus::Single,
            ti: usd(112_400.0),
            qd_l3a: usd(8_000.0),
            net_ltcg_qd_excl: usd(25_000.0),
        };
        assert!(taxcalc_methodology_class(&ops));
    }

    // Below the ceiling the taxcalc PROVENANCE conjunct-1 fails (Table_btctax bins; taxcalc uses the schedule)
    // ⇒ the provenance class cannot fire/over-absorb there (§6.4 composition).
    #[test]
    fn taxcalc_provenance_cannot_fire_below_ceiling() {
        // single_crypto_business_se: taxcalc TI 70_008.908, L16 10_454.96 (baked); Table_btctax bins to 10_459.
        let ops = L16Operands {
            status: FilingStatus::Single,
            ti: usd(70_008.908),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        assert!(!provenance_class_fires(Some(&ops), &ops, 10_454.96));
    }

    // A real Table_btctax semantics bug fails conjunct-1 ⇒ NOT absorbed (teeth). Simulated by feeding an
    // oracle L16 that btctax's own lookup does NOT reproduce on the oracle's operands.
    #[test]
    fn provenance_class_keeps_teeth_against_a_semantics_mismatch() {
        let ops = L16Operands {
            status: FilingStatus::Mfj,
            ti: usd(253_942.94),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        assert!(!provenance_class_fires(Some(&ops), &ops, 99_999.0)); // 99,999 ≠ Table_btctax(253,942.94)
    }

    // Conjunct-2 (the anti-over-absorption guard): btctax's OWN leaves already reproduce the oracle's L16,
    // so there is nothing to explain ⇒ the class must NOT fire. Same leaves for oracle & reproduced ⇒
    // c1 true (Table_btctax(253_942.94)=47_031 == round_leaf(47_031.31)) ∧ c2 false ⇒ false. Kills a
    // conjunct-2 drop (which would leave c1 alone = true).
    #[test]
    fn provenance_conjunct2_blocks_over_absorption_on_identical_leaves() {
        let ops = L16Operands {
            status: FilingStatus::Mfj,
            ti: usd(253_942.94),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        assert!(!provenance_class_fires(Some(&ops), &ops, 47_031.31));
    }

    // Positive fire on DISTINCT operands (the class actually working): oracle = Mfj 253_942.94 with
    // L16 47_031.31 (c1: Table_btctax==round_leaf ⇒ true); reproduced = Single 70_008.908 (Table_btctax
    // bins to 10_459 ≠ 47_031 ⇒ c2 true) ⇒ fires. Kills an always-`false` predicate.
    #[test]
    fn provenance_class_fires_on_distinct_operands() {
        let oracle_ops = L16Operands {
            status: FilingStatus::Mfj,
            ti: usd(253_942.94),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        let reproduced_ops = L16Operands {
            status: FilingStatus::Single,
            ti: usd(70_008.908),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        assert!(provenance_class_fires(
            Some(&oracle_ops),
            &reproduced_ops,
            47_031.31
        ));
    }

    // ★ M4 (the NAMED mutation target): with the oracle's L16 leaves not yet baked (pre-T11), the
    // provenance class CANNOT fire — `None` must return false, never vacuously true, or the whole
    // anti-world guard is dead weight for the entire pre-T11 period.
    #[test]
    fn provenance_class_cannot_fire_without_baked_oracle_leaves() {
        let ops = L16Operands {
            status: FilingStatus::Single,
            ti: usd(112_400.0),
            qd_l3a: usd(8_000.0),
            net_ltcg_qd_excl: usd(25_000.0),
        };
        assert!(!provenance_class_fires(None, &ops, 17_477.0)); // oracle_ops absent ⇒ false
    }

    // stacking_ok — below the ceiling, a taxcalc dissent (schedule vs Tax Table) is ABSORBED by the
    // methodology class while OTS agrees with btctax; the guard passes. (single_crypto_business_se:
    // btctax L16 = 10_459 = OTS Tax-Table; taxcalc = 10_454.96 via the continuous schedule.)
    #[test]
    fn stacking_ok_absorbs_a_below_ceiling_methodology_dissent() {
        let ops = L16Operands {
            status: FilingStatus::Single,
            ti: usd(70_008.908),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        assert!(stacking_ok(
            usd(10_459.0),
            10_459.0,
            Some(10_454.96),
            None,
            None,
            &ops,
            None
        ));
    }

    // stacking_ok — above the ceiling (methodology cannot fire) with no baked provenance leaves and no
    // pin, btctax alone against BOTH oracles is REJECTED (the anti-world guard, incl. the M4 None arm).
    #[test]
    fn stacking_ok_rejects_btctax_alone_against_both() {
        let ops = L16Operands {
            status: FilingStatus::Mfj,
            ti: usd(253_942.94),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        // btctax 47_030 (a hypothetical wrong value) vs OTS/taxcalc 47_031 — no class, no pin ⇒ fails.
        assert!(!stacking_ok(
            usd(47_030.0),
            47_031.31,
            Some(47_031.31),
            None,
            None,
            &ops,
            None
        ));
    }

    // stacking_ok — the §10 KnownDefect pin is AUTHORITATIVE: it holds btctax's caught-wrong value while
    // btctax still prints it, and a STALE pin FAILS in BOTH directions — the wrong value CHANGED, and (the
    // review's arm) the bug is FIXED so btctax now AGREES with both oracles. A fixed bug must NOT pass with
    // the pin still declared, or the pin lingers green re-armed to silently re-absorb a same-value regression.
    #[test]
    fn stacking_ok_known_defect_pin_holds_then_goes_stale() {
        let ops = L16Operands {
            status: FilingStatus::Mfj,
            ti: usd(253_942.94),
            qd_l3a: usd(0.0),
            net_ltcg_qd_excl: usd(0.0),
        };
        let kd = KnownDefect {
            fu_id: "FU-EXAMPLE",
            btctax_value: usd(47_030.0),
        };
        // Held: btctax still prints the pinned wrong 47_030 against both oracles' 47_031.
        assert!(stacking_ok(
            usd(47_030.0),
            47_031.31,
            Some(47_031.31),
            None,
            None,
            &ops,
            Some(&kd)
        ));
        // Stale (value moved to another wrong value): fails, forcing removal.
        assert!(!stacking_ok(
            usd(47_029.0),
            47_031.31,
            Some(47_031.31),
            None,
            None,
            &ops,
            Some(&kd)
        ));
        // Stale (bug FIXED): btctax now agrees with both oracles (47_031), but the pin (old wrong 47_030)
        // is still declared — the authoritative pin FAILS, forcing the entry's deletion.
        assert!(!stacking_ok(
            usd(47_031.0),
            47_031.31,
            Some(47_031.31),
            None,
            None,
            &ops,
            Some(&kd)
        ));
    }

    // LivenessLedger: a declared-but-neither-fired-nor-pinned class is "dead".
    #[test]
    fn liveness_flags_a_dead_class() {
        let mut l = LivenessLedger::default();
        l.declare_pinned("ots_provenance"); // held by a §5.1 pinned cell
        l.record_fire("taxcalc_methodology");
        // "taxcalc_provenance" declared below but neither fired nor pinned ⇒ dead.
        assert_eq!(
            l.dead(&[
                "taxcalc_methodology",
                "ots_provenance",
                "taxcalc_provenance"
            ]),
            vec!["taxcalc_provenance"]
        );
    }
}
