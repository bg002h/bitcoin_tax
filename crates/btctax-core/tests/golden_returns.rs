//! **P7 — golden returns vs an INDEPENDENT oracle** (SPEC §10 Layer 2).
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
//! ★ **Divergences are DECLARED, never silently tolerated.** Where btctax and the oracle disagree, the
//! test asserts *btctax's* value and states the statute that makes it right — with the oracle's figure
//! recorded beside it. A cross-check whose disagreements can be waved away proves nothing; the whole
//! value is that every difference must be explained.

use std::collections::{BTreeMap, BTreeSet};

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::tax::packet::assemble_printed_forms;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::testonly::{
    build_golden_household, golden_households, golden_usd as usd, ty2024_params, ty2024_table,
};
use rust_decimal_macros::dec;

/// A line where the two oracles DISAGREE with each other. btctax follows one of them, and this entry
/// says which, and why.
///
/// This is what a second oracle buys you. With one oracle a disagreement is a stand-off: it can tell you
/// that you differ, never which of you is wrong — and "we disagree with the only oracle we have, but
/// we're confident we're right" is not a position to file a tax return from. With two, a disagreement
/// becomes evidence, and the outlier is identified.
struct Divergence {
    household: &'static str,
    line: &'static str,
    /// btctax's figure — and the oracle it agrees with.
    btctax: Usd,
    agrees_with: &'static str,
    /// The dissenting oracle's figure — the one this entry reconciles against.
    outlier: Usd,
    /// When BOTH oracles dissent (two declared effects stacking), the second one's figure. Recorded so a
    /// change in EITHER engine's answer re-opens the question instead of slipping past.
    outlier_alt: Option<Usd>,
    why: &'static str,
}

/// ★ **The only surviving disagreement is the Tax TABLE, and btctax is on the right side of it.**
///
/// This list used to be dominated by a self-employment-tax divergence. It is gone — and the story of why
/// is worth keeping, because it is the whole argument for owning your oracle rather than renting it.
///
/// **What we thought we had.** Oracle 1 was `tenforty`, a Python wrapper around Open Tax Solver. It
/// computed a self-employment tax that was **invariant to W-2 wages** — flat at $11,304 on $80,000 of SE
/// income whether the wages were $0 or $300,000 — where Schedule SE lines 8a/9/10 and §1402(b)(1) say the
/// 12.4% OASDI portion must fall to ZERO once W-2 social-security wages have consumed the wage base. We
/// broke the tie with a second engine (PSL Tax-Calculator), which tracked btctax to the dollar, and
/// declared the divergence against the wrapper.
///
/// **What was actually true.** Driving OTS's own binaries directly reproduces btctax **to the cent** on
/// every row of that sweep. The engine was never wrong; the *wrapper* never populated Schedule SE line 8a,
/// and separately never supplied the §199A deduction on 1040 line 13. Both fields exist in OTS and both
/// were simply never passed. Reported upstream as [mmacpherson/tenforty#278] with a fix in [#279].
///
/// So oracle 1 is now **OTS itself**, and every divergence the wrapper forced into this list has vanished:
/// on AGI, taxable income, SE tax, NIIT and Additional Medicare, **all three engines now agree exactly, on
/// all twelve households**. A cross-check whose disagreements all turned out to be the harness is a
/// cross-check that was measuring the wrong thing.
///
/// One nuance we got wrong in the first pass and should not repeat: tenforty's omission is **deliberate
/// for Married/Joint**, and defensibly so. `w2_income` there is a household aggregate while Schedule SE is
/// a per-person form, so attributing the household's wages to the self-employed spouse would wrongly wipe
/// out that spouse's wage base. btctax has no such ambiguity — `se_w2_ss_wages` is the filer's own box-3
/// figure, read off an actual W-2 — and we give Tax-Calculator the same attribution via `e00200p`/`e00900p`,
/// so all three engines answer the same question. The wrapper is unambiguously wrong only for *Single*
/// filers, where there is no spouse to attribute wages to at all.
///
/// **What remains.** The Tax TABLE. Below $100,000 of taxable income the 1040 instructions do not merely
/// permit the Table, they **require** it, and the Table taxes each $50 bin at its MIDPOINT. taxcalc models
/// the exact rate schedule instead, so it lands a few dollars away on precisely the households where the
/// Table is mandatory — and nowhere else. btctax and OTS agree; taxcalc is the outlier; the difference
/// vanishes above $100,000, which is exactly where the Table stops applying. An engine of a completely
/// separate lineage thereby confirms the bin semantics that P6 spent a review round getting right.
///
/// [mmacpherson/tenforty#278]: https://github.com/mmacpherson/tenforty/issues/278
/// [#279]: https://github.com/mmacpherson/tenforty/pull/279
const DECLARED_DIVERGENCES: &[Divergence] = &[
    Divergence {
        household: "single_w2_only_standard",
        line: "tax (L16)",
        btctax: dec!(5487),
        agrees_with: "OTS-direct (and the IRS Tax Table)",
        outlier: dec!(5481),
        outlier_alt: None,
        why: "The TAX TABLE is mandatory below $100,000 of taxable income and taxes each $50 bin at its \
              MIDPOINT; taxcalc computes the exact rate schedule instead. btctax files what the \
              instructions require.",
    },
    Divergence {
        household: "single_w2_plus_crypto_ltcg",
        line: "tax (L16)",
        btctax: dec!(8487),
        agrees_with: "OTS-direct (and the IRS Tax Table)",
        outlier: dec!(8481),
        outlier_alt: None,
        why: "Tax Table bin midpoint vs the exact rate schedule — see above.",
    },
    Divergence {
        household: "single_qdcgt_both_slices",
        line: "tax (L16)",
        btctax: dec!(17477),
        agrees_with: "OTS-direct (and the IRS Tax Table)",
        outlier: dec!(17471),
        outlier_alt: None,
        why: "Tax Table bin midpoint vs the exact rate schedule. Note this household's taxable income is \
              ABOVE $100,000 — but the QDCGT worksheet looks up its ORDINARY remainder in the Table, and \
              that remainder is below the threshold. The Table reaches further than the headline figure \
              suggests.",
    },
    Divergence {
        household: "single_short_term_crypto_gain",
        line: "tax (L16)",
        btctax: dec!(6587),
        agrees_with: "OTS-direct (and the IRS Tax Table)",
        outlier: dec!(6581),
        outlier_alt: None,
        why: "Tax Table bin midpoint vs the exact rate schedule — see above.",
    },
    Divergence {
        household: "single_capital_loss_capped",
        line: "tax (L16)",
        btctax: dec!(6587),
        agrees_with: "OTS-direct (and the IRS Tax Table)",
        outlier: dec!(6581),
        outlier_alt: None,
        why: "Tax Table bin midpoint vs the exact rate schedule — see above.",
    },
    // ── CROSS-FOOTING: Σround ≠ roundΣ. A DELIBERATE DEPARTURE from the instructions, not a reading
    //    of them. Read the `why` — the first draft of this entry cited the instruction backwards, and
    //    the reviewer caught it. A divergence whose citation does not check out IS the escape hatch.
    Divergence {
        household: "single_miner_qbi_limited_by_net_capital_gain",
        line: "TOTAL TAX (L24)",
        btctax: dec!(16833),
        agrees_with: "neither — a declared departure (taxcalc reports no comparable TOTAL)",
        outlier: dec!(16832),
        outlier_alt: None,
        why: "★ btctax DEPARTS from the 1040 instructions here, knowingly, and OTS is the one following \
              them. Form 1040 instructions, 'Rounding Off to Whole Dollars' (2024, p. 23), verbatim: \
              'If you have to add two or more amounts to figure the amount to enter on a line, include \
              cents when adding the amounts and round off only the total.' Line 24 IS such a line \
              (22 + 23), so the instruction gives 8,354.59 + 8,477.73 = 16,832.32 → 16,832. That is \
              OTS's figure. SPEC §3.1 instead elects round-at-each-line and CROSS-FOOTS the printed \
              totals, so line 24 adds the printed 8,355 and 8,478 to 16,833 — which is what makes the \
              filed form's own lines add up when a reader adds them. \
              The cost is bounded by ~$0.50 per addend and it can fall EITHER WAY: here it overstates \
              the tax by $1 (the filer overpays), but a different cents pattern would understate it by \
              $1. That is a real, if tiny, exposure in the direction btctax otherwise refuses, and \
              whether §3.1's election is right is a SPEC question, not a P7 one — filed as \
              `spec-3.1-crossfoot-vs-round-the-total`. Every COMPONENT line agrees exactly, including \
              the §199A deduction (8,232) this household exists to test.",
    },
    Divergence {
        household: "single_crypto_business_se",
        line: "tax (L16)",
        btctax: dec!(10459),
        agrees_with: "OTS-direct (and the IRS Tax Table)",
        outlier: dec!(10455),
        outlier_alt: None,
        why: "Tax Table bin midpoint vs the exact rate schedule (taxable income 70,009 < $100,000, where \
              the Table is mandatory). Both engines apply the §199A deduction identically; only the \
              lookup differs.",
    },

    // ── §199A QBI on Schedule C — RESOLVED, not declared. ──
    //
    // ★ The oracle found btctax silently OVERSTATING a miner's tax by omitting the §199A qualified-
    // business-income deduction (20% of the Schedule C profit, net of the §164(f) half-SE deduction).
    // taxcalc applied it; btctax did not; SPEC §4.5 called it a v1 scope decision. The user's call was to
    // follow the law — "20% is way too much to give away for free" — so btctax now COMPUTES it, and the
    // divergences that used to live here are gone: btctax matches BOTH oracles on taxable income. That is
    // the strongest possible outcome for a cross-check — it found a real defect in US, and the fix is
    // confirmed by the engines that found it.
];

/// ★ **The independent cross-check — against TWO engines.**
///
/// The rule that makes a second oracle worth having:
/// - the oracles **agree** ⇒ btctax must match them. No escape hatch.
/// - the oracles **disagree** ⇒ btctax must match one of them, and a [`Divergence`] must name which and
///   why. An undeclared difference — from either oracle — fails.
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
    // ★ Fable P7 r1 M4 — divergence LIVENESS. An entry is consulted only when a mismatch occurs, so a
    // divergence that stops happening (a taxcalc release adopts Tax-Table semantics; a household is
    // renamed) would rot here forever, silently, still claiming to explain something. Track which
    // entries actually fire and demand that every one of them does.
    let mut fired: BTreeSet<usize> = BTreeSet::new();

    for h in &households {
        let (ri, state) = build_golden_household(h);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        // The FILED forms. No golden household makes a charitable donation, so there are no §170(e)
        // details to carry.
        let printed = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, &table, 2024);
        let e = &h.expected_ots;
        let t = &h.expected_taxcalc;

        // (line, btctax, oracle-1 (OTS-direct), oracle-2 (taxcalc) — `None` where taxcalc reports no
        // comparable figure).
        let lines: [(&str, Usd, f64, Option<f64>); 8] = [
            (
                "QBI deduction (8995 L15)",
                ar.qbi_deduction,
                e.qbi_deduction,
                Some(t.qbi_deduction),
            ),
            (
                "AGI (1040 L11)",
                ar.agi,
                e.adjusted_gross_income,
                Some(t.adjusted_gross_income),
            ),
            (
                "taxable income (L15)",
                ar.taxable_income,
                e.taxable_income,
                Some(t.taxable_income),
            ),
            (
                "tax (L16)",
                ar.regular_tax,
                e.income_tax_before_credits,
                Some(t.income_tax_before_credits),
            ),
            (
                "SE tax (Sch 2 L4)",
                ar.se_tax_sch2_l4,
                e.se_tax,
                Some(t.se_tax),
            ),
            (
                "Additional Medicare",
                ar.additional_medicare.additional_medicare_tax,
                e.additional_medicare_tax,
                Some(t.additional_medicare_tax),
            ),
            ("NIIT (Form 8960)", ar.niit.tax, e.niit, Some(t.niit)),
            // ★ The TOTAL is compared against the **printed** chain, not the absolute one — and that is a
            // semantic distinction, not a convenience. Under the SPEC §3.1 round-all-amounts election a
            // printed COMPONENT line is just `round_dollar` of its exact value (so for the six lines above,
            // absolute-rounded and printed are the same number by construction). A printed TOTAL is not: it
            // sums the already-ROUNDED lines, which is what cross-footing means and what the filer actually
            // writes on line 24. Here the two chains differ by $1 — exact cents accumulate to $49,568.43
            // while the filed lines sum to 47,031 + 2,143 + 395 = $49,569 — and it is the filed figure the
            // oracle must be held against, because it is the filed figure the IRS receives.
            //
            // taxcalc's totals bundle payroll tax on W-2 wages, which 1040 L24 does not — no comparison.
            ("TOTAL TAX (L24)", printed.f1040.line24, e.total_tax, None),
        ];

        for (line, ours, o1, o2) in lines {
            // Filed in whole dollars (SPEC §3.1); the oracles report cents.
            let ours = round_dollar(ours);
            let o1 = round_dollar(usd(o1));
            let o2 = o2.map(|v| round_dollar(usd(v)));

            let matches_1 = ours == o1;
            let matches_2 = o2.is_none_or(|v| ours == v);
            if matches_1 && matches_2 {
                continue; // both oracles agree with btctax
            }

            if let Some((idx, d)) = DECLARED_DIVERGENCES
                .iter()
                .enumerate()
                .find(|(_, d)| d.household == h.name && d.line == line)
            {
                fired.insert(idx);
                assert_eq!(
                    ours, d.btctax,
                    "{} {}: btctax's value MOVED — the declared divergence is stale.\nIt was: {}",
                    h.name, line, d.why
                );
                if !matches_1 && !matches_2 {
                    // BOTH dissent (a declared stack) — pin both, so a change in either re-opens it.
                    assert_eq!(
                        o2,
                        Some(d.outlier),
                        "{} {}: taxcalc's value moved — re-examine.\nIt was: {}",
                        h.name,
                        line,
                        d.why
                    );
                    assert_eq!(
                        Some(o1),
                        d.outlier_alt,
                        "{} {}: OTS's value moved — re-examine.\nIt was: {}",
                        h.name,
                        line,
                        d.why
                    );
                } else {
                    let outlier = if matches_1 { o2.unwrap_or(o1) } else { o1 };
                    assert_eq!(
                        outlier, d.outlier,
                        "{} {}: the DISSENTING oracle's value moved — re-examine.\nIt was: {}",
                        h.name, line, d.why
                    );
                }
                // ★ btctax must agree with ONE of the oracles — unless the entry explicitly declares
                // that two known effects STACK on this line. "btctax against the world" is exactly the
                // shape of a confidently-wrong engine, so it is allowed only when it is named as such
                // and the difference reconciles.
                assert!(
                    matches_1 || matches_2 || d.agrees_with.starts_with("neither"),
                    "{} {}: btctax disagrees with BOTH oracles ({} vs OTS {} and taxcalc {:?}), and \
                     the declared divergence claims it agrees with {}. Either the claim is stale or \
                     btctax is alone against the world — re-derive from the statute.",
                    h.name,
                    line,
                    ours,
                    o1,
                    o2,
                    d.agrees_with
                );
                continue;
            }

            diffs.push(format!(
                "  {:<42} {:<22} btctax {:>10}  OTS {:>10}  taxcalc {:>10}   ({})",
                h.name,
                line,
                ours,
                o1,
                o2.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                h.why
            ));
        }
    }

    let dead: Vec<&str> = DECLARED_DIVERGENCES
        .iter()
        .enumerate()
        .filter(|(i, _)| !fired.contains(i))
        .map(|(_, d)| d.line)
        .collect();
    assert!(
        dead.is_empty(),
        "{} DECLARED_DIVERGENCES entr(ies) never fired — they explain a disagreement that no longer \
         happens, and are now just an unread claim about the tax code: {:?}\n\
         Delete them (the oracles agree now) or fix the household/line they name.",
        dead.len(),
        dead
    );

    assert!(
        diffs.is_empty(),
        "btctax disagrees with an INDEPENDENT oracle on {} line(s).\n\
         Every difference must be EXPLAINED — either btctax is wrong, or an oracle is and the \
         divergence is DECLARED with the statute that settles it. Do not weaken this test to make it \
         pass.\n\n{}",
        diffs.len(),
        diffs.join("\n")
    );
}
