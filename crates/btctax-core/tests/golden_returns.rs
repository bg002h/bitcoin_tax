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

use std::collections::BTreeMap;

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::event::BasisSource;
use btctax_core::event::{DisposeKind, IncomeKind};
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, IncomeRecord, LedgerState, Term};
use btctax_core::tax::packet::assemble_printed_forms;
use btctax_core::tax::return_1040::assemble_absolute;
use btctax_core::tax::return_inputs::{
    Form1099Div, Form1099Int, Owner, ReturnInputs, ScheduleAInputs, ScheduleCInputs, W2,
};
use btctax_core::tax::testonly::{ty2024_params, ty2024_table};
use btctax_core::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use serde::Deserialize;
use time::macros::date;

#[derive(Debug, Deserialize)]
struct Goldens {
    households: Vec<Household>,
}

#[derive(Debug, Deserialize)]
struct Household {
    name: String,
    why: String,
    inputs: Inputs,
    /// Oracle 1 — **OpenTaxSolver 2024, its own binaries driven directly** (GPL, observe-only).
    ///
    /// Formerly this was `tenforty`, a Python wrapper around OTS. The wrapper turned out to drop two
    /// inputs on the floor — Schedule SE line 8a and the §199A deduction on 1040 line 13 — each of which
    /// OVERSTATES a self-employed filer's tax. Reported upstream (mmacpherson/tenforty#278, fix in #279).
    /// **The engine was never at fault**: driven directly it reproduces btctax to the cent, and every
    /// divergence the wrapper used to force into the list below is gone.
    expected_ots: ExpectedOts,
    /// Oracle 2 — PSL Tax-Calculator (CC0). A completely separate lineage.
    expected_taxcalc: ExpectedTaxcalc,
}

/// Oracle 1's outputs. `total_tax` is OTS's 1040 line 24 plus the NIIT it computes on Form 8960 —
/// directly comparable to btctax's line 24.
#[derive(Debug, Deserialize)]
struct ExpectedOts {
    adjusted_gross_income: f64,
    taxable_income: f64,
    income_tax_before_credits: f64,
    se_tax: f64,
    niit: f64,
    additional_medicare_tax: f64,
    total_tax: f64,
}

/// The second oracle's outputs. Only the lines whose definitions are unambiguous across engines: we do
/// NOT take its `combined`/`iitax` totals, which bundle payroll tax on W-2 wages that 1040 line 24 does
/// not include.
#[derive(Debug, Deserialize)]
struct ExpectedTaxcalc {
    adjusted_gross_income: f64,
    taxable_income: f64,
    income_tax_before_credits: f64,
    se_tax: f64,
    niit: f64,
    additional_medicare_tax: f64,
}

#[derive(Debug, Deserialize)]
struct Inputs {
    filing_status: String,
    #[serde(default)]
    w2_income: f64,
    #[serde(default)]
    taxable_interest: f64,
    #[serde(default)]
    qualified_dividends: f64,
    #[serde(default)]
    ordinary_dividends: f64,
    #[serde(default)]
    short_term_capital_gains: f64,
    #[serde(default)]
    long_term_capital_gains: f64,
    #[serde(default)]
    self_employment_income: f64,
    #[serde(default)]
    itemized_deductions: f64,
}

fn usd(v: f64) -> Usd {
    Usd::try_from(v).expect("the oracle emits finite figures")
}

/// Build the SAME household in btctax's own input model.
///
/// The mapping is deliberately literal: the oracle's `w2_income` is a W-2's box 1 (and its box 3 / box 5,
/// which is what a real W-2 carries), its capital gains are crypto disposals on the ledger (which is how
/// btctax gets to Schedule D at all), and its `self_employment_income` is business crypto — a Schedule C
/// trade or business, which is the only way btctax produces SE tax.
fn build(h: &Household) -> (ReturnInputs, LedgerState) {
    let i = &h.inputs;
    let status = match i.filing_status.as_str() {
        "Single" => FilingStatus::Single,
        "Married/Joint" => FilingStatus::Mfj,
        other => panic!("unmapped filing status {other:?}"),
    };

    let mut ri = ReturnInputs {
        filing_status: status,
        ..Default::default()
    };
    ri.header.taxpayer = btctax_core::tax::return_inputs::Person {
        first_name: "Golden".into(),
        last_name: "Household".into(),
        ssn: "123456789".into(),
        ..Default::default()
    };
    if status == FilingStatus::Mfj {
        ri.header.spouse = Some(btctax_core::tax::return_inputs::Person {
            first_name: "Golden".into(),
            last_name: "Spouse".into(),
            ssn: "987654321".into(),
            ..Default::default()
        });
    }

    if i.w2_income > 0.0 {
        let w = usd(i.w2_income);
        ri.w2s.push(W2 {
            owner: Owner::Taxpayer,
            employer: "ORACLE CO".into(),
            box1_wages: w,
            box3_ss_wages: w,       // the §1402(b)(1) SS-cap channel
            box5_medicare_wages: w, // the Form 8959 Part I channel
            ..Default::default()
        });
    }
    if i.taxable_interest > 0.0 {
        ri.int_1099.push(Form1099Int {
            payer: "ORACLE BANK".into(),
            box1_interest: usd(i.taxable_interest),
            ..Default::default()
        });
    }
    if i.ordinary_dividends > 0.0 || i.qualified_dividends > 0.0 {
        ri.div_1099.push(Form1099Div {
            payer: "ORACLE BROKER".into(),
            box1a_ordinary: usd(i.ordinary_dividends), // INCLUDES the qualified subset
            box1b_qualified: usd(i.qualified_dividends),
            ..Default::default()
        });
    }
    if i.itemized_deductions > 0.0 {
        ri.schedule_a = Some(ScheduleAInputs {
            mortgage_interest_1098: usd(i.itemized_deductions),
            ..Default::default()
        });
    }
    if i.self_employment_income > 0.0 {
        ri.schedule_c = Some(ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "Bitcoin mining".into(),
            ..Default::default()
        });
    }
    // Schedule B Part III must be answered when Schedule B files.
    ri.foreign_accounts = Some(false);
    ri.foreign_trust = Some(false);

    // ── The ledger: capital gains are DISPOSALS; SE income is business crypto. ──────────────────
    let mut state = LedgerState::default();
    let mut leg = |gain: f64, term: Term, ev: u64| {
        // proceeds − basis = the gain; a loss is basis > proceeds.
        let (proceeds, basis) = if gain >= 0.0 {
            (usd(gain), Usd::ZERO)
        } else {
            (Usd::ZERO, usd(-gain))
        };
        state.disposals.push(Disposal {
            event: EventId::decision(ev),
            kind: DisposeKind::Sell,
            disposed_at: date!(2024 - 05 - 01),
            legs: vec![DisposalLeg {
                lot_id: LotId {
                    origin_event_id: EventId::decision(ev + 100),
                    split_sequence: 0,
                },
                sat: 100_000_000,
                proceeds,
                basis,
                gain: proceeds - basis,
                term,
                basis_source: BasisSource::ExchangeProvided,
                gift_zone: None,
                acquired_at: if term == Term::LongTerm {
                    date!(2020 - 01 - 01)
                } else {
                    date!(2024 - 01 - 02)
                },
                wallet: WalletId::SelfCustody {
                    label: "cold".into(),
                },
                pseudo: false,
            }],
            fee_mini_disposition: false,
        });
    };
    if i.short_term_capital_gains != 0.0 {
        leg(i.short_term_capital_gains, Term::ShortTerm, 1);
    }
    if i.long_term_capital_gains != 0.0 {
        leg(i.long_term_capital_gains, Term::LongTerm, 2);
    }
    if i.self_employment_income > 0.0 {
        state.income_recognized.push(IncomeRecord {
            event: EventId::decision(3),
            recognized_at: date!(2024 - 06 - 01),
            sat: 100_000_000,
            usd_fmv: usd(i.self_employment_income),
            kind: IncomeKind::Mining,
            business: true, // ⇒ Schedule C ⇒ Schedule SE
            pseudo: false,
        });
    }

    (ri, state)
}

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
/// all ten households**. A cross-check whose disagreements all turned out to be the harness is a
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
    let raw = include_str!("goldens/full_return_goldens.json");
    let goldens: Goldens = serde_json::from_str(raw).expect("the golden file parses");
    assert!(
        goldens.households.len() >= 10,
        "the matrix must cover the SPEC §10 branches"
    );

    let params = ty2024_params();
    let table = ty2024_table();
    let mut diffs: Vec<String> = Vec::new();

    for h in &goldens.households {
        let (ri, state) = build(h);
        let ar = assemble_absolute(&ri, &state, &params, &table, 2024);
        // The FILED forms. No golden household makes a charitable donation, so there are no §170(e)
        // details to carry.
        let printed = assemble_printed_forms(&ri, &state, &BTreeMap::new(), &ar, &table, 2024);
        let e = &h.expected_ots;
        let t = &h.expected_taxcalc;

        // (line, btctax, oracle-1 (OTS-direct), oracle-2 (taxcalc) — `None` where taxcalc reports no
        // comparable figure).
        let lines: [(&str, Usd, f64, Option<f64>); 7] = [
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

            if let Some(d) = DECLARED_DIVERGENCES
                .iter()
                .find(|d| d.household == h.name && d.line == line)
            {
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
