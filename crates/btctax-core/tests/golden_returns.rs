//! **P7 — golden returns vs an INDEPENDENT oracle** (SPEC §10 Layer 2), COMPUTE level.
//!
//! ★ Why this file is the most important test in the repo.
//!
//! Every other test btctax has is **self-referential**: the core printed chains agree with the
//! fillers, the forms tie out to each other, the packet cross-foots. None of that can catch an
//! *internally consistent wrong number* — a return where every form adds up beautifully and the tax
//! is simply wrong. Three rounds of adversarial review caught missing forms, blank cells and
//! contradictory arithmetic; none of them could have caught a wrong tax.
//!
//! So this test diffs btctax against **two separate implementations of the US individual income tax** —
//! Open Tax Solver (its own binaries, driven directly) and the PSL Tax-Calculator, engines of completely
//! different lineage — over a matrix of synthetic households. The oracles' answers
//! are generated offline by `scripts/oracle/gen_goldens.py` and committed to
//! `tests/goldens/full_return_goldens.json` — CI stays hermetic and network-free (btctax is an
//! offline-first tool), and the licensing posture is observe-only: we RUN the oracle and compare
//! FIGURES; we never read, copy, link or distribute its GPL source (recon 05 / SPEC §9).
//!
//! ★ **Divergences are ADJUDICATED BY CLASS, never silently tolerated.** Where btctax and an oracle
//! disagree, the difference passes only when it falls into a *named, lawful class* — a Tax-Table-vs-schedule
//! methodology difference we expect, or a per-oracle provenance difference we can independently witness on
//! that oracle's own leaves (`tax::oracle_diff`, the §6.2/§6.4 machinery). Anything else is btctax alone
//! against the world — the exact shape of a confidently-wrong engine — and it FAILS. This file is the
//! COMPUTE-level witness (`golden_packet.rs` is the paper level); it holds btctax's compute + printed-chain
//! figures against the class machinery over the full line set. A cross-check whose disagreements can be
//! waved away proves nothing; the whole value is that every difference must be explained.

use std::collections::BTreeMap;

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::tax::oracle_diff::{
    round_leaf, stacking_ok, sum_round, table_l16, taxcalc_methodology_class, L16Operands,
    LivenessLedger,
};
use btctax_core::tax::packet::assemble_printed_forms;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{
    build_golden_household, golden_households, golden_usd as usd, ty2024_params, ty2024_table,
};
use btctax_core::tax::FilingStatus;

/// ★ **The independent cross-check — against TWO engines, adjudicated by divergence CLASS.**
///
/// The rule that makes a second oracle worth having:
/// - the oracles **agree** ⇒ btctax must match them. No escape hatch.
/// - the oracles **disagree** ⇒ the difference passes only when a named class ([`stacking_ok`]) absorbs
///   it: the taxcalc Tax-Table-vs-schedule methodology class, or a per-oracle provenance class witnessed
///   on that oracle's own leaves. btctax alone against BOTH oracles, with no absorbing class, FAILS.
///
/// ★ **The only structural disagreement is the Tax TABLE, and btctax is on the right side of it.** Below
/// $100,000 of taxable income the 1040 instructions do not merely permit the Table, they **require** it,
/// and the Table taxes each $50 bin at its MIDPOINT. taxcalc models the exact rate schedule instead, so it
/// lands a few dollars away on precisely the households where the Table is mandatory — and nowhere else.
/// btctax and OTS agree; taxcalc is the outlier; the difference vanishes above $100,000, exactly where the
/// Table stops applying. That difference is now absorbed by the `taxcalc_methodology_class` (`consulted_table`),
/// not a hand-written per-household entry: the class fires on any household whose QDCGT worksheet consulted
/// the Table — including `single_qdcgt_both_slices`, whose taxable income is ABOVE $100,000 but whose
/// ORDINARY remainder falls in the Table (the Table reaches further than the headline figure suggests).
///
/// ★ **The self-employment-tax divergence is gone, and the story is the whole argument for owning your
/// oracle.** Oracle 1 was once `tenforty`, a Python wrapper around OTS; it computed an SE tax invariant to
/// W-2 wages, where Schedule SE lines 8a/9/10 and §1402(b)(1) drop the 12.4% OASDI portion to ZERO once the
/// wage base is consumed. Driving OTS's own binaries directly reproduces btctax **to the cent** — the
/// *wrapper* never populated Schedule SE line 8a nor the §199A deduction on 1040 line 13. Reported upstream
/// (mmacpherson/tenforty#278, fix in #279). So oracle 1 is now **OTS itself**, and on AGI, taxable income,
/// SE tax, NIIT, Additional Medicare and the QBI deduction **all three engines agree exactly, on all
/// twelve households** — those six leaves stay a plain exact-vs-both-oracles check (no class).
///
/// ★ **The line-24 "divergence" was a phantom, and the cross-foot dissolves it.** btctax's printed line 24
/// is `round(L22) + round(L23)` (Σround); OTS's exact total is roundΣ; they differ by the lawful §6102
/// Σround≠roundΣ residual. Comparing btctax's cross-foot to `sum_round` of OTS's own *component* totals —
/// cross-foot vs cross-foot — makes them equal on all twelve, so OTS's exact total is never consulted and
/// the divergence disappears (`design/full-return/ROUNDING_AUTHORITY.md` for the §6102 authority).
///
/// [mmacpherson/tenforty#278]: https://github.com/mmacpherson/tenforty/issues/278
/// [#279]: https://github.com/mmacpherson/tenforty/pull/279
#[test]
fn every_golden_household_matches_the_independent_oracles() {
    let households = golden_households();
    assert!(
        households.len() >= 10,
        "the matrix must cover the SPEC §10 branches"
    );

    let params = ty2024_params();
    let table = ty2024_table();
    let mut diffs: Vec<String> = Vec::new();

    // ── Class liveness (§6.2/§6.4) ────────────────────────────────────────────────────────────────────
    // The taxcalc Tax-Table methodology class is LIVE now — it fires on the households whose QDCGT
    // worksheet consulted the IRS Tax Table (btctax + OTS bin at the $50 midpoint; taxcalc uses the
    // continuous schedule). Registered here via `record_fire`; the per-oracle PROVENANCE classes cannot fire
    // until the oracle L16 leaves bake (T11), so their liveness and the full `LivenessLedger::dead()` sweep
    // (over ALL declared classes, held alive by the §5.1 pinned cells) are enabled in T11 with the pinned
    // cells. A plain positive check (`methodology_class_fired`) proves the live class engaged this run.
    let mut liveness = LivenessLedger::default();
    let mut methodology_class_fired = false;

    for h in &households {
        let (ri, state) = build_golden_household(h);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        // The FILED forms. No golden household makes a charitable donation, so there are no §170(e)
        // details to carry.
        let printed = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, &table, 2024);
        let e = &h.expected_ots;
        let t = &h.expected_taxcalc;

        // ── The §6.2(b) reproduced operands — btctax's OWN return operands, sourced from `ar` ──────────
        // These feed `table_l16` / the methodology class. They are the RETURN's own §1(h) worksheet
        // inputs — 1040 L15 taxable income, L3a qualified dividends, and the QD-EXCLUSIVE preferential net
        // LTCG — exactly the three args `assemble_absolute` passes to `qdcgt_line16` (return_1040.rs:1216).
        // Derivable PRE-T11: they are btctax's own figures, not the oracle's `Option` leaves.
        let reproduced_ops = L16Operands {
            status: ri.filing_status,
            ti: ar.taxable_income,
            qd_l3a: ar.qualified_dividends,
            net_ltcg_qd_excl: ar.net_ltcg,
        };

        // ── Level 1: the six cent-exact leaf totals — EXACT vs BOTH oracles (as HEAD) ──────────────────
        // `round_dollar` both sides; btctax must equal `round_leaf(OTS)` AND `round_leaf(taxcalc)`. No
        // class escape — these have matched both engines on all twelve households since the SE-tax wrapper
        // bug was fixed, and a class here would only hide a regression. (bit-equal on SE tax / NIIT /
        // Add'l Medicare; within the §3.1 whole-dollar residual on TI / AGI / QBI, where OTS line-rounds
        // the 8995 chain and taxcalc does not — the `round_dollar`-both-sides shape is what passes, r3-M1.)
        let cent_exact: [(&str, Usd, f64, f64); 6] = [
            (
                "QBI deduction (8995 L15)",
                ar.qbi_deduction,
                e.qbi_deduction,
                t.qbi_deduction,
            ),
            (
                "AGI (1040 L11)",
                ar.agi,
                e.adjusted_gross_income,
                t.adjusted_gross_income,
            ),
            (
                "taxable income (L15)",
                ar.taxable_income,
                e.taxable_income,
                t.taxable_income,
            ),
            ("SE tax (Sch 2 L4)", ar.se_tax_sch2_l4, e.se_tax, t.se_tax),
            (
                "Additional Medicare",
                ar.additional_medicare.additional_medicare_tax,
                e.additional_medicare_tax,
                t.additional_medicare_tax,
            ),
            ("NIIT (Form 8960)", ar.niit.tax, e.niit, t.niit),
        ];
        for (line, ours, ots, taxcalc) in cent_exact {
            let ours = round_dollar(ours);
            if ours == round_leaf(ots) && ours == round_leaf(taxcalc) {
                continue; // every oracle agrees with btctax exactly
            }
            diffs.push(format!(
                "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   ({})",
                h.name,
                line,
                ours,
                round_leaf(ots),
                round_leaf(taxcalc),
                h.why
            ));
        }

        // ── Level 2a: L16 (tax) — the §6.2 two-part (structural reproduction + class stacking) ──────────
        // Part 1 (structural, r2-I2 Table-semantics witness): `oracle_diff::table_l16`, run on btctax's
        // OWN return operands, must reproduce the compute engine's L16 exactly. A drift between the
        // reproduction and `method::qdcgt_line16` breaks this before any oracle is consulted.
        assert_eq!(
            table_l16(
                reproduced_ops.status,
                reproduced_ops.ti,
                reproduced_ops.qd_l3a,
                reproduced_ops.net_ltcg_qd_excl,
            ),
            ar.regular_tax,
            "{}: oracle_diff::table_l16 must reproduce btctax's own compute-engine L16 exactly",
            h.name,
        );
        // Part 2 (class stacking): btctax and OTS agree (both consult the Table where mandatory); taxcalc
        // dissents on the Table anchors via its continuous schedule. `stacking_ok` absorbs the dissent ONLY
        // through a named class, and is the anti-world guard — a both-oracle disagreement with no absorbing
        // class FAILS (see `stacking_ok_guards_golden_returns_against_btctax_alone`).
        let l16_line = "tax (L16)";
        let l16_ours = round_dollar(ar.regular_tax);
        let l16_ots = e.income_tax_before_credits;
        let l16_taxcalc = t.income_tax_before_credits;
        let l16_agrees_all = l16_ours == round_leaf(l16_ots) && l16_ours == round_leaf(l16_taxcalc);
        if !l16_agrees_all {
            // Provenance leaves are None pre-T11 (⇒ `provenance_class_fires == false`), so the only class
            // that can absorb a dissent now is the taxcalc methodology class. OTS necessarily agrees when
            // `stacking_ok` passes pre-T11 (its provenance conjunct cannot fire), so a surviving taxcalc
            // dissent is the methodology difference — register it live.
            if stacking_ok(
                l16_ours,
                l16_ots,
                Some(l16_taxcalc),
                None, // ots_ops: provenance leaves inert until T11
                None, // taxcalc_ops: provenance leaves inert until T11
                &reproduced_ops,
                None, // no known-defect pin
            ) {
                if taxcalc_methodology_class(&reproduced_ops) {
                    liveness.record_fire("taxcalc_methodology");
                    methodology_class_fired = true;
                }
            } else {
                diffs.push(format!(
                    "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   \
                     (btctax alone — no lawful class absorbs it)",
                    h.name,
                    l16_line,
                    l16_ours,
                    round_leaf(l16_ots),
                    round_leaf(l16_taxcalc),
                ));
            }
        }

        // ── Level 2b: L24 (TOTAL TAX) — the cross-foot dissolves the phantom Σround≠roundΣ divergence ───
        // btctax's PRINTED line 24 (`round(L22) + round(L23)`) is held against `sum_round` of OTS's own
        // COMPONENT totals — `Σ round_dollar(leg)`, NOT `round_dollar(OTS's exact total)`. The latter is the
        // lawful §6102 roundΣ residual that used to force the `single_miner_qbi` divergence; comparing
        // cross-foot to cross-foot dissolves it (OTS's exact total is never consulted). OTS-single-witness —
        // taxcalc bundles payroll tax on W-2 wages that 1040 L24 does not, so it reports no comparable total.
        // Pre-T11 fallback (plan lines 68-72): the legs are the baked per-line TOTALS; the leg form
        // `sum_round([se_l10_oasdi, se_l11_medicare, f8959_l7, f8959_l13, …])` activates when they bake (T11).
        let l24_ours = round_dollar(printed.f1040.line24);
        let l24_ots = sum_round(&[
            e.income_tax_before_credits,
            e.se_tax,
            e.additional_medicare_tax,
            e.niit,
        ]);
        if l24_ours != l24_ots {
            // OTS-single-witness: with no taxcalc opinion, a mismatch is btctax alone against OTS → fail.
            diffs.push(format!(
                "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   \
                 (btctax alone vs OTS on the L24 cross-foot)",
                h.name, "TOTAL TAX (L24)", l24_ours, l24_ots, "—",
            ));
        }

        // ── Deeper-line rows — INERT until the oracle leaves bake (T11) ─────────────────────────────────
        // Every deeper oracle leaf is `None` in today's baked JSON, so each `if let Some` block is a no-op
        // NOW; they light up at the T11 re-bake without another rewrite (and are validated there). Compute
        // level: btctax's figure is its own compute value (`ar.*`), held against `round_leaf(oracle leaf)` —
        // OTS + taxcalc where both witness, OTS-single-witness where the reproduction table (plan §6.1)
        // marks it so.
        let deeper: [(&str, Usd, Option<f64>, Option<f64>); 3] = [
            (
                "deduction taken (L12)",
                ar.deduction,
                e.deduction_taken,
                t.deduction_taken,
            ),
            (
                "Sch D → 1040 L7",
                ar.capital_gain,
                e.sch_d_to_l7,
                t.sch_d_to_l7,
            ),
            (
                "SALT capped (Sch A L5e)",
                ar.schedule_a.as_ref().map_or(Usd::ZERO, |a| a.salt_5e),
                e.salt_capped,
                t.salt_capped,
            ),
        ];
        for (line, ours, ots_leaf, taxcalc_leaf) in deeper {
            let ours = round_dollar(ours);
            if let Some(o) = ots_leaf {
                if ours != round_leaf(o) {
                    diffs.push(format!(
                        "  {:<42} {:<22} btctax {:>10}  OTS {:>10}   (deeper-line, T11)",
                        h.name,
                        line,
                        ours,
                        round_leaf(o)
                    ));
                }
            }
            if let Some(tc) = taxcalc_leaf {
                if ours != round_leaf(tc) {
                    diffs.push(format!(
                        "  {:<42} {:<22} btctax {:>10}  taxcalc {:>10}   (deeper-line, T11)",
                        h.name,
                        line,
                        ours,
                        round_leaf(tc)
                    ));
                }
            }
        }
        // 8995 L12 net-capital-gain cap — OTS single-witness / WEAK (plan §6.1: OTS's is driver-hand-fed,
        // §14.2 closure is a follow-up), so it is gated on the OTS leaf alone.
        if let Some(o) = e.qbi_cap_l12 {
            let ours = round_dollar(ar.printed_inputs.qbi_net_capital_gain);
            if ours != round_leaf(o) {
                diffs.push(format!(
                    "  {:<42} {:<22} btctax {:>10}  OTS {:>10}   (deeper-line, T11, OTS single-witness)",
                    h.name, "8995 L12 net-cap-gain", ours, round_leaf(o)
                ));
            }
        }
    }

    // ── Liveness (positive) — the methodology class must have engaged this run ──────────────────────────
    // The full `LivenessLedger::dead()` sweep (over ALL declared classes, incl. the per-oracle provenance
    // classes held alive by the §5.1 pinned cells) and provenance-class liveness are enabled in T11 with the
    // pinned cells. `liveness` is registered now (via `record_fire`) so T11 extends it without a rewrite.
    assert!(
        methodology_class_fired,
        "the taxcalc Tax-Table methodology class never fired: no household's QDCGT worksheet CONSULTED the \
         IRS Tax Table on operands where taxcalc's continuous schedule dissents from btctax + OTS. The class \
         is declared live and must engage on the Table anchors — re-derive if the anchors changed."
    );

    assert!(
        diffs.is_empty(),
        "btctax disagrees with an INDEPENDENT oracle on {} line(s).\n\
         Every difference must be EXPLAINED — either btctax is wrong, or an oracle is and the difference is \
         absorbed by a named divergence CLASS (`tax::oracle_diff`). Do not weaken this test to make it \
         pass.\n\n{}",
        diffs.len(),
        diffs.join("\n")
    );
}

/// ★ **The anti-world guard has TEETH — a synthetic both-oracle disagreement.**
///
/// None of the twelve real households is a both-oracle disagreement (OTS always agrees with btctax on
/// L16), so the FAIL branch in the main loop never fires on real data. This synthetic scenario pins the
/// guard directly: btctax alone (47,030) against OTS and taxcalc (both 47,031), ABOVE the Tax-Table ceiling
/// so the methodology class cannot fire, with no baked provenance leaves and no pin — [`stacking_ok`] MUST
/// reject it. **Mutation-check:** force `stacking_ok` to return `true` and this test fails; restore and it
/// passes. That is the difference between a guard and decoration.
#[test]
fn stacking_ok_guards_golden_returns_against_btctax_alone() {
    let ops = L16Operands {
        status: FilingStatus::Mfj,
        ti: usd(253_942.94),
        qd_l3a: usd(0.0),
        net_ltcg_qd_excl: usd(0.0),
    };
    assert!(
        !stacking_ok(
            usd(47_030.0),      // btctax's (hypothetical wrong) figure
            47_031.31,          // OTS
            Some(47_031.31),    // taxcalc — both oracles agree with each other, against btctax
            None,               // ots_ops: no baked provenance leaves
            None,               // taxcalc_ops: no baked provenance leaves
            &ops,               // above the ceiling ⇒ methodology class cannot fire
            None,               // no known-defect pin
        ),
        "btctax alone against BOTH oracles, above the ceiling, with no absorbing class must be REJECTED — \
         the anti-world guard is the whole point of the class machinery."
    );
}
