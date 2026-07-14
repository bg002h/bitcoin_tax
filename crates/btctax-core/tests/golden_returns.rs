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
//! So this test diffs btctax against a **separate implementation of the US individual income tax**
//! (`tenforty`, wrapping Open Tax Solver) over a matrix of synthetic households. The oracle's answers
//! are generated offline by `scripts/oracle/gen_goldens.py` and committed to
//! `tests/goldens/full_return_goldens.json` — CI stays hermetic and network-free (btctax is an
//! offline-first tool), and the licensing posture is observe-only: we RUN the oracle and compare
//! FIGURES; we never read, copy, link or distribute its GPL source (recon 05 / SPEC §9).
//!
//! ★ **Divergences are DECLARED, never silently tolerated.** Where btctax and the oracle disagree, the
//! test asserts *btctax's* value and states the statute that makes it right — with the oracle's figure
//! recorded beside it. A cross-check whose disagreements can be waved away proves nothing; the whole
//! value is that every difference must be explained.

use btctax_core::conventions::{round_dollar, Usd};
use btctax_core::event::BasisSource;
use btctax_core::event::{DisposeKind, IncomeKind};
use btctax_core::identity::{EventId, LotId, WalletId};
use btctax_core::state::{Disposal, DisposalLeg, IncomeRecord, LedgerState, Term};
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
    expected: Expected,
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

#[derive(Debug, Deserialize)]
struct Expected {
    federal_adjusted_gross_income: f64,
    federal_taxable_income: f64,
    federal_income_tax: f64,
    federal_se_tax: f64,
    federal_niit: f64,
    federal_additional_medicare_tax: f64,
    federal_total_tax: f64,
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

/// A line where btctax and the oracle DISAGREE, and btctax is right.
///
/// Every entry must carry the statute or the form text that settles it. This list is the whole point of
/// the cross-check: a divergence that can be waved away proves nothing, so each one is named, explained,
/// and asserted in btctax's favour — and any UNdeclared difference fails the test.
struct Divergence {
    household: &'static str,
    line: &'static str,
    /// What btctax prints (and why it is right).
    btctax: Usd,
    /// What the oracle says.
    oracle: Usd,
    why: &'static str,
}

/// ★ **The oracle's SE tax ignores W-2 wages entirely — MEASURED, not assumed.** This is the divergence
/// the whole exercise was worth running for.
///
/// **What the law says.** Schedule SE states it on its face: **line 8a** = "Total social security wages
/// and tips (total of boxes 3 and 7 on Form(s) W-2)"; **line 9** = "Subtract line 8d from line 7 [the
/// wage base]. **If zero or less, enter -0- here and on line 10 and go to line 11**"; **line 10** =
/// "Multiply the smaller of line 6 or line 9 by 12.4%". §1402(b)(1) is the statute: the 12.4% OASDI band
/// is a per-person ANNUAL band, and W-2 wages fill it first. This household has $220,000 of W-2 Social
/// Security wages against the 2024 base of $168,600 — the band is already full, so line 9 = 0, the SS
/// portion is ZERO, and only the uncapped 2.9% Medicare portion remains.
///
/// **What the oracle does.** Holding SE income at $80,000 (MFJ) and sweeping the W-2 wages, its SE tax
/// does not move at all:
///
/// | W-2 wages | oracle SE tax | SS band left | correct SE tax |
/// |---:|---:|---:|---:|
/// | 0 | 11,304 | 168,600 | 11,304 ✓ |
/// | 100,000 | 11,304 | 68,600 | 10,649 |
/// | 168,600 | 11,304 | 0 | 2,143 |
/// | 220,000 | 11,304 | 0 | 2,143 |
/// | 300,000 | 11,304 | 0 | 2,143 |
///
/// It is FLAT. The oracle never consults Schedule SE line 8a, so it levies the full 12.4% on the SE
/// earnings even when the wage base has already been consumed — inventing $9,161 of Social Security tax
/// the law does not impose (and, through the §164(f) ½-SE deduction, a different AGI and bottom line).
/// Whether that lives in the engine or in the Python wrapper we cannot say, and deliberately do not
/// investigate: the clean-room posture forbids reading their source, and the figure is wrong either way.
///
/// **Why it only bites here.** The oracle agrees with btctax on the OTHER SE household
/// (`single_crypto_business_se`: $40,000 wages, $60,000 SE) because there the band is NOT binding —
/// `min(base, room)` = base either way. The divergence appears exactly when the W-2 wages bind, which is
/// the case btctax analysed in recon deep/02 C4 and modelled deliberately (`se.rs`:
/// `ss = 12.4% × min(base, ss_wage_base − w2_ss_wages)`). The cross-check confirms that analysis was
/// right and an independent implementation got it wrong.
const DECLARED_DIVERGENCES: &[Divergence] = &[
    Divergence {
        household: "mfj_se_over_the_addl_medicare_threshold",
        line: "SE tax (Sch 2 L4)",
        btctax: dec!(2143),
        oracle: dec!(11304),
        why: "§1402(b)(1) / Sch SE L9: W-2 SS wages ($220,000) already exceed the $168,600 wage base, \
              so the 12.4% SS portion is ZERO and only the 2.9% Medicare portion remains. The oracle \
              charges the full 12.4% regardless.",
    },
    Divergence {
        household: "mfj_se_over_the_addl_medicare_threshold",
        line: "AGI (1040 L11)",
        btctax: dec!(298929),
        oracle: dec!(294348),
        why: "Consequence of the SE divergence above: a smaller SE tax means a smaller §164(f) ½-SE \
              deduction, hence a HIGHER AGI. btctax's AGI is the one the law produces.",
    },
    Divergence {
        household: "mfj_se_over_the_addl_medicare_threshold",
        line: "taxable income (L15)",
        btctax: dec!(269729),
        oracle: dec!(265148),
        why: "Same root cause — the ½-SE deduction feeds AGI, which feeds taxable income.",
    },
    Divergence {
        household: "mfj_se_over_the_addl_medicare_threshold",
        line: "tax (L16)",
        btctax: dec!(50820),
        oracle: dec!(49721),
        why: "Same root cause — the Tax Table is applied to the (correct, higher) taxable income.",
    },
    Divergence {
        household: "mfj_se_over_the_addl_medicare_threshold",
        line: "TOTAL TAX (L24)",
        btctax: dec!(53357),
        oracle: dec!(61419),
        why: "Net of both effects: btctax's total is LOWER because it does not levy the $9,161 of \
              Social Security tax the wage base has already absorbed, even though its income tax is \
              slightly higher. The oracle over-taxes this household by $8,062.",
    },
];

/// ★ **The independent cross-check.** Every household, every line.
#[test]
fn every_golden_household_matches_the_independent_oracle() {
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

        let mut check = |line: &str, ours: Usd, theirs: f64| {
            // The return is FILED in whole dollars (SPEC §3.1); the oracle reports cents. Compare the
            // figures as they would be filed.
            let ours = round_dollar(ours);
            let theirs = round_dollar(usd(theirs));
            if ours == theirs {
                return;
            }
            // A DECLARED divergence: btctax is right, and the entry says why. Assert btctax's value
            // (so a regression in OUR engine still fails) and the oracle's (so a change in the oracle's
            // answer re-opens the question instead of silently passing).
            if let Some(d) = DECLARED_DIVERGENCES
                .iter()
                .find(|d| d.household == h.name && d.line == line)
            {
                assert_eq!(
                    ours, d.btctax,
                    "{} {}: btctax's value MOVED — the declared divergence is stale, re-examine it.\n\
                     The divergence was: {}",
                    h.name, line, d.why
                );
                assert_eq!(
                    theirs, d.oracle,
                    "{} {}: the ORACLE's value moved — re-examine the divergence",
                    h.name, line
                );
                return;
            }
            diffs.push(format!(
                "  {:<42} {:<22} btctax {:>12}  oracle {:>12}   ({})",
                h.name, line, ours, theirs, h.why
            ));
        };

        let e = &h.expected;
        check("AGI (1040 L11)", ar.agi, e.federal_adjusted_gross_income);
        check(
            "taxable income (L15)",
            ar.taxable_income,
            e.federal_taxable_income,
        );
        check("tax (L16)", ar.regular_tax, e.federal_income_tax);
        check("SE tax (Sch 2 L4)", ar.se_tax_sch2_l4, e.federal_se_tax);
        check(
            "Additional Medicare",
            ar.additional_medicare.additional_medicare_tax,
            e.federal_additional_medicare_tax,
        );
        check("NIIT (Form 8960)", ar.niit.tax, e.federal_niit);
        check("TOTAL TAX (L24)", ar.total_tax, e.federal_total_tax);
    }

    assert!(
        diffs.is_empty(),
        "btctax disagrees with the INDEPENDENT oracle on {} line(s).\n\
         Every difference must be EXPLAINED — either btctax is wrong, or the oracle is and the \
         divergence is declared with its statute. Do not weaken this test to make it pass.\n\n{}",
        diffs.len(),
        diffs.join("\n")
    );
}
