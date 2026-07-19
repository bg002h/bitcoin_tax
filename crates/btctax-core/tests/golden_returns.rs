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
    provenance_class_fires, round_leaf, stacking_ok, sum_round, table_l16,
    taxcalc_methodology_class, L16Operands, LivenessLedger,
};
use btctax_core::tax::packet::assemble_printed_forms;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::tables::{FullReturnParams, TaxTable};
use btctax_core::tax::testonly::{
    build_golden_household, golden_households, golden_usd as usd, ty2024_params, ty2024_table,
    GoldenHousehold,
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

    // ── Class liveness (§6.2/§6.4) — T11 ACTIVE ─────────────────────────────────────────────────────
    // Every declared divergence class must ENGAGE this run: the taxcalc Tax-Table methodology class on the
    // Table anchors, and the two per-oracle L16 PROVENANCE classes on the §5.1 pinned cells (now baked,
    // with their own oracle L16 leaves present so `provenance_class_fires` can witness them).
    // `adjudicate_household` records the class that ABSORBS each L16 dissent; the `dead()` sweep below
    // deletes any declared class that never fired — a class that no household exercises is an unread claim
    // about the tax code, not a live guard.
    let mut liveness = LivenessLedger::default();

    for h in &households {
        diffs.extend(adjudicate_household(h, &params, &table, &mut liveness));
    }

    // ── Liveness sweep — no declared class may be DEAD (T11: pinned cells hold the provenance classes) ──
    // ★ §12 CLASS-LIVENESS (this `dead()` sweep, mirrored on paper by
    // `golden_packet::the_paper_differential_engages_every_divergence_class`): every declared divergence
    // class must fire for ≥1 corpus household or be held by its §5.1 pinned cell — a class matching nothing
    // is an unread claim about the tax code and is deleted. (T13 satisfies the §12 obligation by REFERENCE.)
    // The other §12 KATs live at the paper level (`golden_packet.rs`): deeper-line teeth, read-back
    // fault-injection, anchor-derivation; determinism is the offline `scripts/oracle/check_determinism.py`.
    let dead = liveness.dead(&[
        "taxcalc_methodology",
        "ots_provenance",
        "taxcalc_provenance",
    ]);
    assert!(
        dead.is_empty(),
        "declared divergence class(es) never fired and are not pinned: {dead:?}. Each names a lawful \
         disagreement that MUST actually occur in the corpus — the Table anchors hold the methodology class \
         live, and the two §5.1 pinned cells hold the per-oracle provenance classes live (bin-edge ⇒ \
         `ots_provenance`, cents-flip ⇒ `taxcalc_provenance`). A dead class is dead weight: re-derive the \
         pinned cell that should hold it (an engine bump may have moved the edge), or delete the class."
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

/// Reproduce btctax's whole-dollar 1040 L15 from an oracle's OWN line-rounded component leaves (C1 table):
/// `round_leaf(AGI) − round_leaf(deduction_taken) − round_leaf(qbi_deduction)`. btctax prints L15 as
/// `L11 − L12 − L13` on already-whole-dollar lines, so this cross-foot matches it EXACTLY — the lawful
/// rounding-order residual, where an oracle carries cents through the 8995 chain and its exact L15
/// straddles a dollar (r3-M1), never appears. It stays an EXACT witness (no tolerance): reproduced from
/// the oracle's own AGI/deduction/QBI leaves, all of which are separately held cent-exact against btctax.
/// `None` while the deduction leaf is unbaked (pre-T11), falling the caller back to `round_leaf(total)`.
fn ti_crossfoot(agi: f64, deduction_taken: Option<f64>, qbi_deduction: f64) -> Option<Usd> {
    // 1040 L15 is floored at 0 ("if zero or less, enter -0-") — the oracle's AGI − deduction can go
    // negative (a low-income filer whose deduction exceeds AGI), where btctax prints 0.
    deduction_taken
        .map(|ded| (round_leaf(agi) - round_leaf(ded) - round_leaf(qbi_deduction)).max(Usd::ZERO))
}

/// An oracle's OWN §1(h) line-16 operands, assembled from its baked provenance leaves (`qual_div_l3a`,
/// `net_ltcg_qd_exclusive`, and its exact-cents taxable income). Returns `None` while either leaf is
/// unbaked (pre-T11) — which is exactly what keeps [`provenance_class_fires`] inert then (the named M4
/// mutation target). Post-bake both are `Some`, so the per-oracle provenance class can witness the §5.1
/// pinned cells on the oracle's own figures.
fn oracle_ops(
    status: FilingStatus,
    taxable_income: f64,
    qual_div_l3a: Option<f64>,
    net_ltcg_qd_exclusive: Option<f64>,
) -> Option<L16Operands> {
    match (qual_div_l3a, net_ltcg_qd_exclusive) {
        (Some(qd), Some(ltcg)) => Some(L16Operands {
            status,
            ti: usd(taxable_income),
            qd_l3a: usd(qd),
            net_ltcg_qd_excl: usd(ltcg),
        }),
        _ => None,
    }
}

/// Adjudicate ONE household into its divergence diffs (empty ⇒ it agrees with the oracles under the class
/// machinery), recording the class that ABSORBS each L16 dissent into `liveness`.
///
/// ★ This **IS** the per-household body of the main loop, extracted so the loop's own FAIL-wiring
/// (`else { diffs.push(...) }`) is itself exercised on a synthetic divergence
/// (`the_main_loop_reports_a_both_oracle_l16_divergence`, T5-m1) — the `stacking_ok`-returns-false path
/// wiring, not just the `stacking_ok` function. And it records *which* class fired (methodology /
/// OTS-provenance / taxcalc-provenance), so [`LivenessLedger::dead`] is meaningful (T5-m2).
fn adjudicate_household(
    h: &GoldenHousehold,
    params: &FullReturnParams,
    table: &TaxTable,
    liveness: &mut LivenessLedger,
) -> Vec<String> {
    let mut diffs: Vec<String> = Vec::new();
    let (ri, state) = build_golden_household(h);
    let ar = assemble_absolute(&ri, &state, params, table, 2024);
    // The FILED forms. No golden household makes a charitable donation, so there are no §170(e)
    // details to carry.
    let printed = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, table, 2024);
    let e = &h.expected_ots;
    let t = &h.expected_taxcalc;

    // ── The §6.2(b) reproduced operands — btctax's OWN return operands, sourced from `ar` ──────────────
    // These feed `table_l16` / the methodology class. They are the RETURN's own §1(h) worksheet inputs —
    // 1040 L15 taxable income, L3a qualified dividends, and the QD-EXCLUSIVE preferential net LTCG —
    // exactly the three args `assemble_absolute` passes to `qdcgt_line16` (return_1040.rs:1216).
    let reproduced_ops = L16Operands {
        status: ri.filing_status,
        ti: ar.taxable_income,
        qd_l3a: ar.qualified_dividends,
        net_ltcg_qd_excl: ar.net_ltcg,
    };

    // ── Level 1: the five cent-exact leaf totals — EXACT vs BOTH oracles ────────────────────────────────
    // `round_dollar` both sides; btctax must equal `round_leaf(OTS)` AND `round_leaf(taxcalc)`. No class
    // escape — these have matched both engines since the SE-tax wrapper bug was fixed, and a class here
    // would only hide a regression. (Taxable income is handled just below by the C1 CROSS-FOOT, not here.)
    let cent_exact: [(&str, Usd, f64, f64); 5] = [
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

    // ── Taxable income (1040 L15) — the C1 CROSS-FOOT, EXACT vs BOTH oracles ────────────────────────────
    // btctax prints L15 = L11 − L12 − L13 on whole-dollar lines. Reproducing each oracle's L15 the SAME
    // way — `round_leaf(AGI) − round_leaf(deduction) − round_leaf(QBI)` from its own component leaves —
    // matches btctax exactly and dissolves the lawful rounding-order residual (an oracle carrying cents
    // through the 8995 chain, its exact L15 straddling a dollar; r3-M1). Both oracles remain exact
    // witnesses (no tolerance); pre-T11 the deduction leaf is `None`, so this falls back to the HEAD shape
    // `round_leaf(oracle taxable_income)`.
    let ti_ours = round_dollar(ar.taxable_income);
    let ti_ots = ti_crossfoot(e.adjusted_gross_income, e.deduction_taken, e.qbi_deduction)
        .unwrap_or_else(|| round_leaf(e.taxable_income));
    let ti_tc = ti_crossfoot(t.adjusted_gross_income, t.deduction_taken, t.qbi_deduction)
        .unwrap_or_else(|| round_leaf(t.taxable_income));
    if !(ti_ours == ti_ots && ti_ours == ti_tc) {
        diffs.push(format!(
            "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   ({})",
            h.name, "taxable income (L15)", ti_ours, ti_ots, ti_tc, h.why
        ));
    }

    // ── Level 2a: L16 (tax) — the §6.2 two-part (structural reproduction + class stacking) ──────────────
    // Part 1 (structural, Table-semantics witness): `table_l16`, run on btctax's OWN return operands, must
    // reproduce the compute engine's L16 exactly, before any oracle is consulted.
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
    // Part 2 (class stacking). The oracles' OWN L16 operands (baked provenance leaves) are `Some` at T11,
    // so the per-oracle PROVENANCE classes can now witness the §5.1 pinned cells (bin-edge ⇒ OTS, cents-flip
    // ⇒ taxcalc). btctax alone against BOTH oracles, with no absorbing class, still FAILS (the anti-world
    // guard — see `stacking_ok_guards_golden_returns_against_btctax_alone`).
    let ots_ops = oracle_ops(
        ri.filing_status,
        e.taxable_income,
        e.qual_div_l3a,
        e.net_ltcg_qd_exclusive,
    );
    let taxcalc_ops = oracle_ops(
        ri.filing_status,
        t.taxable_income,
        t.qual_div_l3a,
        t.net_ltcg_qd_exclusive,
    );
    let l16_ours = round_dollar(ar.regular_tax);
    let l16_ots = e.income_tax_before_credits;
    let l16_taxcalc = t.income_tax_before_credits;
    let l16_agrees_all = l16_ours == round_leaf(l16_ots) && l16_ours == round_leaf(l16_taxcalc);
    if !l16_agrees_all {
        if stacking_ok(
            l16_ours,
            l16_ots,
            Some(l16_taxcalc),
            ots_ops.as_ref(),
            taxcalc_ops.as_ref(),
            &reproduced_ops,
            None, // no known-defect pin
        ) {
            // ── T5-m2: record WHICH class actually absorbed each oracle's dissent ───────────────────────
            // OTS carries a provenance class only: a surviving OTS dissent is witnessed by it.
            if l16_ours != round_leaf(l16_ots)
                && provenance_class_fires(ots_ops.as_ref(), &reproduced_ops, l16_ots)
            {
                liveness.record_fire("ots_provenance");
            }
            // taxcalc: the Tax-Table methodology class takes precedence (as in `stacking_ok`), else its
            // provenance class explains the dissent.
            if l16_ours != round_leaf(l16_taxcalc) {
                if taxcalc_methodology_class(&reproduced_ops) {
                    liveness.record_fire("taxcalc_methodology");
                } else if provenance_class_fires(taxcalc_ops.as_ref(), &reproduced_ops, l16_taxcalc)
                {
                    liveness.record_fire("taxcalc_provenance");
                }
            }
        } else {
            diffs.push(format!(
                "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   \
                 (btctax alone — no lawful class absorbs it)",
                h.name,
                "tax (L16)",
                l16_ours,
                round_leaf(l16_ots),
                round_leaf(l16_taxcalc),
            ));
        }
    }

    // ── Level 2b: L24 (TOTAL TAX) — the cross-foot dissolves the phantom Σround≠roundΣ divergence ───────
    // btctax's PRINTED line 24 (`round(L22) + round(L23)`) held against `round_leaf(L16) + reproduced
    // SE-L12 + reproduced 8959-L18 + round_leaf(NIIT)`. At T11 the OTS component LEGS bake, so SE-L12 /
    // 8959-L18 switch from the pre-T11 `round_leaf(total)` fallback to the true cross-foot `sum_round(legs)`
    // — and L24 INHERITS them, so the lawful §6102 Σround≠roundΣ residual can never resurrect on L24 (it
    // would, with no class to absorb it, if L24 instead summed `round(exact total)`). OTS-single-witness —
    // taxcalc bundles payroll tax on W-2 wages that 1040 L24 does not, so it reports no comparable total.
    let se_l12_ots = match (e.se_l10_oasdi, e.se_l11_medicare) {
        (Some(l10), Some(l11)) => sum_round(&[l10, l11]),
        _ => round_leaf(e.se_tax),
    };
    let f8959_l18_ots = match (e.f8959_l7, e.f8959_l13) {
        (Some(l7), Some(l13)) => sum_round(&[l7, l13]),
        _ => round_leaf(e.additional_medicare_tax),
    };
    // The L16 leg is btctax's OWN FILED L16 (`printed.f1040.line16` — the exact value btctax summed into
    // its printed L24), NOT the oracle's L16 and NOT the compute `round_dollar(ar.regular_tax)` (the filed
    // QDCGT-worksheet L16 can differ from the compute figure by the §3.1 rounding point — printed.rs:612).
    // The L16 VALUE is separately adjudicated by the two-part comparison above (with its
    // provenance/methodology class), so L24 must not re-litigate it: here it witnesses btctax's cross-foot
    // arithmetic and the SE-L12 / 8959-L18 / NIIT legs against OTS. This keeps L24 green on BOTH §5.1
    // pinned cells (whose L16 legitimately differs from an oracle's by the class-absorbed Tax-Table bin /
    // cents flip) while still reddening on any real btctax cross-foot, Sch-2-leg, or L16 bug.
    let l24_ours = round_dollar(printed.f1040.line24);
    let l24_ots = printed.f1040.line16 + se_l12_ots + f8959_l18_ots + round_leaf(e.niit);
    if l24_ours != l24_ots {
        // OTS-single-witness: with no taxcalc opinion, a mismatch is btctax alone against OTS → fail.
        diffs.push(format!(
            "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   \
             (btctax alone vs OTS on the L24 cross-foot)",
            h.name, "TOTAL TAX (L24)", l24_ours, l24_ots, "—",
        ));
    }

    // ── Deeper-line rows — LIVE at T11 (oracle leaves baked) ────────────────────────────────────────────
    // btctax's figure is its own compute value (`ar.*`), held against `round_leaf(oracle leaf)` — OTS +
    // taxcalc where both witness. Deduction taken (L12) and Sch D → 1040 L7 apply to every household.
    let deeper: [(&str, Usd, Option<f64>, Option<f64>); 2] = [
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

    // ── SALT (Sch A L5e) — SCOPED to ITEMIZING households (T9 salt_capped scoping) ──────────────────────
    // btctax's `ar.schedule_a` is populated whenever there are Schedule-A INPUTS, even when the standard
    // deduction wins; taxcalc's `salt_capped` (`c18300`) is "SALT as DEDUCTED" (0 when standard wins) and
    // OTS's is `None` when standard wins. Comparing btctax's computed L5e on a SALT-paying-but-standard
    // household would be a false red — so gate the row on whether btctax's DEDUCTION is actually the
    // itemized one (`ar.deduction_is_itemized`), the deduction PATH. (In the corpus D-3 forces itemizing to
    // win wherever there are itemized inputs, so this is defensive; it makes the row correct by
    // construction rather than by that invariant.)
    if ar.deduction_is_itemized {
        if let Some(a) = ar.schedule_a.as_ref() {
            let ours = round_dollar(a.salt_5e);
            if let Some(o) = e.salt_capped {
                if ours != round_leaf(o) {
                    diffs.push(format!(
                        "  {:<42} {:<22} btctax {:>10}  OTS {:>10}   (SALT L5e, T11)",
                        h.name,
                        "SALT capped (Sch A L5e)",
                        ours,
                        round_leaf(o)
                    ));
                }
            }
            if let Some(tc) = t.salt_capped {
                if ours != round_leaf(tc) {
                    diffs.push(format!(
                        "  {:<42} {:<22} btctax {:>10}  taxcalc {:>10}   (SALT L5e, T11)",
                        h.name,
                        "SALT capped (Sch A L5e)",
                        ours,
                        round_leaf(tc)
                    ));
                }
            }
        }
    }

    // ── 8995 L12 net-capital-gain cap — OTS single-witness / WEAK ───────────────────────────────────────
    // (plan §6.1: OTS's is driver-hand-fed, §14.2 closure is a follow-up), so it is gated on the OTS leaf
    // alone.
    if let Some(o) = e.qbi_cap_l12 {
        let ours = round_dollar(ar.printed_inputs.qbi_net_capital_gain);
        if ours != round_leaf(o) {
            diffs.push(format!(
                "  {:<42} {:<22} btctax {:>10}  OTS {:>10}   (deeper-line, T11, OTS single-witness)",
                h.name,
                "8995 L12 net-cap-gain",
                ours,
                round_leaf(o)
            ));
        }
    }

    diffs
}

/// ★ **The anti-world guard has TEETH — a synthetic both-oracle disagreement (the FUNCTION).**
///
/// This pins [`stacking_ok`] directly: btctax alone (47,030) against OTS and taxcalc (both 47,031), ABOVE
/// the Tax-Table ceiling so the methodology class cannot fire, with no baked provenance leaves and no pin —
/// it MUST reject. **Mutation-check:** force `stacking_ok` to return `true` and this test fails; restore
/// and it passes. That is the difference between a guard and decoration. Its companion,
/// `the_main_loop_reports_a_both_oracle_l16_divergence`, pins the main loop's own FAIL-wiring (T5-m1).
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

/// ★ **Every deeper line has TEETH at the COMPUTE level (§12).** The paper-level twin
/// (`golden_packet::deeper_lines_have_teeth`) proves the read-off-paper comparisons bite; this proves the
/// compute-level comparison code — a SEPARATE path in `adjudicate_household` — is equally load-bearing.
/// For each compute-level deeper line, a baked witness where perturbing ONLY the oracle's leaf must surface
/// a disagreement NAMING that line. Drop the line's comparison and the perturbed witness goes quiet, so
/// this reddens — the mutation that turns "compared" into "load-bearing". (SE-L12 / 8959-L18 are cross-foot
/// LEGS folded into the L24 witness at the compute level, so they carry their own line tag only on paper;
/// their teeth live in the paper-level twin.)
#[test]
fn deeper_lines_have_teeth_at_the_compute_level() {
    let params = ty2024_params();
    let table = ty2024_table();
    // (baked witness, perturb ONLY the OTS leaf the line compares, the compute-level line tag that MUST
    // then appear). Each perturbation is a whole $100 — always across a `round_leaf` dollar boundary.
    // A deeper-line teeth case, aliased so the case table stays clippy-clean (type_complexity).
    type TeethCase = (&'static str, fn(&mut GoldenHousehold), &'static str);
    let cases: &[TeethCase] = &[
        (
            "single_w2_only_standard",
            |h| h.expected_ots.deduction_taken = h.expected_ots.deduction_taken.map(|v| v + 100.0),
            "deduction taken (L12)",
        ),
        (
            "single_w2_plus_crypto_ltcg",
            |h| h.expected_ots.sch_d_to_l7 = h.expected_ots.sch_d_to_l7.map(|v| v + 100.0),
            "Sch D → 1040 L7",
        ),
        (
            "mfj_itemized_salt_over_the_cap",
            |h| h.expected_ots.salt_capped = h.expected_ots.salt_capped.map(|v| v + 100.0),
            "SALT capped (Sch A L5e)",
        ),
        (
            "single_miner_qbi_limited_by_net_capital_gain",
            |h| h.expected_ots.qbi_cap_l12 = h.expected_ots.qbi_cap_l12.map(|v| v + 100.0),
            "8995 L12 net-cap-gain",
        ),
    ];

    let mut toothless = Vec::new();
    for &(name, perturb, tag) in cases {
        let mut h = golden_households()
            .into_iter()
            .find(|h| h.name == name)
            .unwrap_or_else(|| {
                panic!("§12 compute-teeth witness {name:?} is not in the baked corpus")
            });
        // Clean before perturbation (the corpus is green), so the disagreement below is provably caused by
        // the perturbation. A fresh ledger each run — liveness is irrelevant here.
        let clean = adjudicate_household(&h, &params, &table, &mut LivenessLedger::default());
        if !clean.is_empty() {
            toothless.push(format!(
                "  {name:<46} [{tag}] witness is NOT clean before perturbation: {clean:?}"
            ));
            continue;
        }
        perturb(&mut h);
        let diffs = adjudicate_household(&h, &params, &table, &mut LivenessLedger::default());
        if !diffs.iter().any(|d| d.contains(tag)) {
            toothless.push(format!(
                "  {name:<46} [{tag}] perturbing the oracle leaf surfaced NO `{tag}` disagreement — the \
                 compute-level comparison has no teeth (drop it and this differential would not notice). \
                 diffs: {diffs:?}"
            ));
        }
    }
    assert!(
        toothless.is_empty(),
        "compute-level deeper line(s) whose comparison does NOT bite — a compared line not load-bearing \
         in any corpus scenario is decoration, not a check (§12):\n{}",
        toothless.join("\n")
    );
}

/// ★ **The MAIN LOOP reports a both-oracle divergence — the loop's own FAIL-wiring (T5-m1).**
///
/// `stacking_ok_guards_golden_returns_against_btctax_alone` pins the *function*; this pins that the loop
/// body actually TURNS a `stacking_ok`-returns-false into a reported diff. It runs the SAME
/// `adjudicate_household` the main loop runs, on a real ABOVE-ceiling anchor whose oracle L16 has been
/// forced to disagree with btctax and whose provenance leaves are cleared (so no class can absorb it) —
/// and asserts the L16 "btctax alone" line is reported. Delete the `else { diffs.push(...) }` wiring and
/// this goes red; the function-level guard alone would not.
#[test]
fn the_main_loop_reports_a_both_oracle_l16_divergence() {
    let params = ty2024_params();
    let table = ty2024_table();

    // A real ABOVE-ceiling anchor (TI ≈ 253,943 ⇒ methodology class cannot fire).
    let mut h = golden_households()
        .into_iter()
        .find(|h| h.name == "mfj_se_over_the_addl_medicare_threshold")
        .expect("the above-ceiling SE anchor is in the matrix");

    // Force BOTH oracles' L16 to a value btctax cannot possibly print (its real L16 here is ~47,031), and
    // CLEAR the provenance leaves so no provenance class can fire either — btctax alone against both, with
    // no lawful class. Every other baked leaf stays real (it agrees with btctax), so the loop reports
    // exactly the engineered L16 (and the L24 it feeds) — not a wall of noise.
    h.expected_ots.income_tax_before_credits = 999_999.0;
    h.expected_taxcalc.income_tax_before_credits = 999_999.0;
    h.expected_ots.qual_div_l3a = None;
    h.expected_ots.net_ltcg_qd_exclusive = None;
    h.expected_taxcalc.qual_div_l3a = None;
    h.expected_taxcalc.net_ltcg_qd_exclusive = None;

    let mut liveness = LivenessLedger::default();
    let diffs = adjudicate_household(&h, &params, &table, &mut liveness);

    assert!(
        diffs
            .iter()
            .any(|d| d.contains("tax (L16)") && d.contains("btctax alone")),
        "the main loop must REPORT a both-oracle L16 divergence that no class absorbs — its \
         `else {{ diffs.push(...) }}` FAIL-wiring, not just `stacking_ok`. Got diffs:\n{}",
        diffs.join("\n")
    );
}
