//! **Test scaffolding** — synthetic households and TY2024 parameters shared by core's own tests, the
//! `btctax-forms` fill/read-back KATs, and (P7) the golden-return matrix.
//!
//! This is `#[doc(hidden)]` and contains no tax logic: every figure here is a FIXTURE, not a fact. It
//! lives in the library (rather than a `#[cfg(test)]` module) for one reason — the forms crate and the
//! P7 golden-return matrix are *downstream* crates, and a household that produces every form in the
//! packet is exactly the thing they must not rebuild independently. One fixture, one packet, one set of
//! expectations.

use crate::conventions::Usd;
use crate::event::{BasisSource, DisposeKind, IncomeKind};
use crate::identity::{EventId, LotId, WalletId};
use crate::state::{Disposal, DisposalLeg, IncomeRecord, LedgerState, Term};
use crate::tax::return_inputs::{
    CharitableClass, CharitableGift, Dependent, Form1099Div, Form1099G, Form1099Int,
    HouseholdHeader, Owner, Payments, Person, ReturnInputs, ScheduleAInputs, ScheduleCInputs, W2,
};
use crate::tax::tables::{
    AmtParams, FullReturnParams, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule, TaxTable,
};
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;
use serde::Deserialize;
use std::collections::BTreeMap;
use time::macros::date;

/// The TY2024 §63 / §199A / §164(b) parameters (the real Rev. Proc. 2023-34 figures).
pub fn ty2024_params() -> FullReturnParams {
    let mut std_deduction = BTreeMap::new();
    std_deduction.insert(FilingStatus::Single, dec!(14600));
    std_deduction.insert(FilingStatus::Mfj, dec!(29200));
    std_deduction.insert(FilingStatus::Mfs, dec!(14600));
    std_deduction.insert(FilingStatus::HoH, dec!(21900));
    FullReturnParams {
        year: 2024,
        std_deduction,
        std_aged_blind_married: dec!(1550),
        std_aged_blind_unmarried: dec!(1950),
        dependent_std_floor: dec!(1300),
        dependent_std_earned_addon: dec!(450),
        salt_cap: dec!(10000),
        kiddie_unearned_threshold: dec!(2600),
        elective_deferral_limit: dec!(23000),
        ftc_ceiling: dec!(300),
        qbi_ti_threshold_unmarried: dec!(191950),
        qbi_ti_threshold_married: dec!(383900),
        student_loan_phaseout_unmarried: (dec!(80000), dec!(95000)),
        student_loan_phaseout_married: (dec!(165000), dec!(195000)),
        amt: AmtParams {
            exemption_single_hoh: dec!(85700),
            exemption_mfj_qss: dec!(133300),
            exemption_mfs: dec!(66650),
            phaseout_start_single_hoh_mfs: dec!(609350),
            phaseout_start_mfj_qss: dec!(1218700),
            breakpoint_28pct: dec!(232600),
            breakpoint_28pct_mfs: dec!(116300),
        },
    }
}

/// The real TY2024 ordinary + §1(h) schedules for Single and MFJ (Rev. Proc. 2023-34). Enough for every
/// fixture household here; MFS/HoH pricing is not exercised by the packet KATs.
pub fn ty2024_table() -> TaxTable {
    let mut ordinary = BTreeMap::new();
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(11600), dec!(0.12)),
                bracket(dec!(47150), dec!(0.22)),
                bracket(dec!(100525), dec!(0.24)),
                bracket(dec!(191950), dec!(0.32)),
                bracket(dec!(243725), dec!(0.35)),
                bracket(dec!(609350), dec!(0.37)),
            ],
        },
    );
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(23200), dec!(0.12)),
                bracket(dec!(94300), dec!(0.22)),
                bracket(dec!(201050), dec!(0.24)),
                bracket(dec!(383900), dec!(0.32)),
                bracket(dec!(487450), dec!(0.35)),
                bracket(dec!(731200), dec!(0.37)),
            ],
        },
    );
    let mut ltcg = BTreeMap::new();
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(47025),
            max_fifteen: dec!(518900),
        },
    );
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(94050),
            max_fifteen: dec!(583750),
        },
    );
    TaxTable {
        year: 2024,
        source: "TEST-TY2024",
        ordinary,
        ltcg,
        gift_annual_exclusion: dec!(18000),
        ss_wage_base: dec!(168600),
        gift_lifetime_exclusion: dec!(13_610_000),
    }
}

fn bracket(lower: Usd, rate: Usd) -> OrdinaryBracket {
    OrdinaryBracket { lower, rate }
}

fn person(first: &str, last: &str, ssn: &str, occupation: &str) -> Person {
    Person {
        first_name: first.into(),
        last_name: last.into(),
        ssn: ssn.into(),
        occupation: occupation.into(),
        ..Default::default()
    }
}

/// ★ **The kitchen-sink household** — one synthetic MFJ family tuned so that EVERY form in the packet
/// files: Schedules 1, 2, 3, A, B, C, D and Forms 8959, 8960, 8995.
///
/// The four Schedule-D Part III routings cannot coexist on one return (they are mutually exclusive by
/// construction — SPEC §7.2), so this is the gain-both primary; the routing variants are separate
/// fixtures. Every figure is chosen to clear a threshold, and the reason is stated where it is not
/// obvious — a fixture that silently stops tripping a threshold is a KAT that silently stops testing.
pub fn kitchen_sink_household() -> (ReturnInputs, LedgerState) {
    let ri = ReturnInputs {
        filing_status: FilingStatus::Mfj,
        header: HouseholdHeader {
            taxpayer: person("John", "Doe", "123-45-6789", "Engineer"),
            spouse: Some(person("Jane", "Doe", "987-65-4321", "Architect")),
            address_street: "100 Main St".into(),
            address_city: "Springfield".into(),
            address_state: "IL".into(),
            address_zip: "62704".into(),
            dependents: vec![Dependent {
                name: "Sam Doe".into(),
                ssn: "111-22-3333".into(),
                relationship: "Son".into(),
                date_of_birth: Some(date!(2012 - 04 - 15)),
                ..Default::default()
            }],
            ..Default::default()
        },
        // Σ box 5 = 290,000 > the $250,000 MFJ threshold ⇒ Form 8959 Part I fires.
        w2s: vec![
            W2 {
                owner: Owner::Taxpayer,
                employer: "ACME".into(),
                box1_wages: dec!(200000),
                box2_fed_withheld: dec!(40000),
                box3_ss_wages: dec!(168600),
                box4_ss_withheld: dec!(10453.20),
                box5_medicare_wages: dec!(200000),
                box6_medicare_withheld: dec!(2900),
                box17_state_tax_withheld: dec!(9000),
                ..Default::default()
            },
            W2 {
                owner: Owner::Spouse,
                employer: "GLOBEX".into(),
                box1_wages: dec!(90000),
                box2_fed_withheld: dec!(15000),
                box3_ss_wages: dec!(90000),
                box4_ss_withheld: dec!(5580),
                box5_medicare_wages: dec!(90000),
                box6_medicare_withheld: dec!(1305),
                box17_state_tax_withheld: dec!(4000),
                ..Default::default()
            },
        ],
        // Interest 2,000 > the $1,500 Schedule B threshold; box 6 feeds the §904(j) FTC (Sch 3 L1).
        int_1099: vec![Form1099Int {
            payer: "First Bank".into(),
            box1_interest: dec!(2000),
            box4_fed_withheld: dec!(100),
            box6_foreign_tax: dec!(100),
            ..Default::default()
        }],
        // box 2a ⇒ Schedule D L13; box 5 ⇒ Form 8995 QBI; box 7 ⇒ the rest of the FTC.
        div_1099: vec![Form1099Div {
            payer: "Broker LLC".into(),
            box1a_ordinary: dec!(3000),
            box1b_qualified: dec!(1000),
            box2a_capgain_distr: dec!(500),
            box5_section_199a: dec!(1200),
            box7_foreign_tax: dec!(50),
            ..Default::default()
        }],
        g_1099: vec![Form1099G {
            payer: "State of IL".into(),
            box1_unemployment: dec!(1000),
            ..Default::default()
        }],
        // Business crypto (the ledger's SE-eligible income, below) ⇒ Schedule C ⇒ Schedule SE ⇒ Sch 2 L4.
        schedule_c: Some(ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "Bitcoin mining".into(),
            naics_code: "518210".into(),
            expenses: dec!(1000),
            ..Default::default()
        }),
        // Itemized ≈ 10,000 SALT (capped) + 22,000 mortgage + 5,000 charity ⇒ well over the $29,200
        // MFJ standard deduction, so L12 IS Schedule A line 17 (the tie-out the packet KAT asserts).
        schedule_a: Some(ScheduleAInputs {
            medical: dec!(2000),
            salt_state_estimated_payments: dec!(12000),
            salt_real_estate: dec!(6000),
            mortgage_interest_1098: dec!(22000),
            charitable: vec![CharitableGift {
                class: CharitableClass::Cash60,
                amount: dec!(5000),
            }],
            ..Default::default()
        }),
        payments: Payments {
            estimated_tax_payments: dec!(1000),
            extension_payment: dec!(500), // ⇒ Schedule 3 L10
            ..Default::default()
        },
        foreign_accounts: Some(false), // Schedule B Part III must be ANSWERED when Sch B files (I7)
        foreign_trust: Some(false),
        ..Default::default()
    };

    let state = LedgerState {
        // Business mining income ⇒ Schedule C gross ⇒ SE tax.
        income_recognized: vec![IncomeRecord {
            event: EventId::decision(1),
            recognized_at: date!(2024 - 06 - 01),
            sat: 100_000_000,
            usd_fmv: dec!(20000),
            kind: IncomeKind::Mining,
            business: true,
            pseudo: false,
        }],
        // A long-term crypto sale ⇒ Schedule D Part II ⇒ the gain-both routing.
        disposals: vec![Disposal {
            event: EventId::decision(2),
            kind: DisposeKind::Sell,
            disposed_at: date!(2024 - 05 - 01),
            legs: vec![DisposalLeg {
                lot_id: LotId {
                    origin_event_id: EventId::decision(3),
                    split_sequence: 0,
                },
                sat: 100_000_000,
                proceeds: dec!(30000),
                basis: dec!(10000),
                gain: dec!(20000),
                term: Term::LongTerm,
                basis_source: BasisSource::ExchangeProvided,
                gift_zone: None,
                acquired_at: date!(2020 - 01 - 01),
                wallet: WalletId::SelfCustody {
                    label: "cold".into(),
                },
                pseudo: false,
            }],
            fee_mini_disposition: false,
        }],
        ..Default::default()
    };

    (ri, state)
}

/// The kitchen sink's [`ReturnHeader`](crate::tax::packet::ReturnHeader) — for the forms crate's
/// identity KATs, which need a header but not a whole return.
pub fn kitchen_sink_header() -> crate::tax::packet::ReturnHeader {
    let (ri, _) = kitchen_sink_household();
    crate::tax::packet::ReturnHeader::build(&ri, 2024).expect("the fixture's SSNs are canonical")
}

/// The opposite pole: a plain Single W-2 household that files a **1040 and nothing else**. Every
/// optional form must be absent — the packet's `None` arms are as load-bearing as its `Some` ones (an
/// over-eager `Some` staples a blank schedule to the return).
///
/// Box 6 is EXACTLY 1.45% × box 5, so Form 8959 line 24 (the withholding reconciliation) is zero too —
/// the form is not required on either leg.
pub fn w2_only_household() -> (ReturnInputs, LedgerState) {
    let ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        header: HouseholdHeader {
            taxpayer: person("Pat", "Roe", "222-33-4444", "Teacher"),
            address_street: "9 Elm St".into(),
            address_city: "Springfield".into(),
            address_state: "IL".into(),
            address_zip: "62704".into(),
            ..Default::default()
        },
        w2s: vec![W2 {
            owner: Owner::Taxpayer,
            employer: "SCHOOL DISTRICT".into(),
            box1_wages: dec!(60000),
            box2_fed_withheld: dec!(6000),
            box3_ss_wages: dec!(60000),
            box4_ss_withheld: dec!(3720),
            box5_medicare_wages: dec!(60000),
            box6_medicare_withheld: dec!(870), // 1.45% × 60,000 ⇒ no Part V excess
            ..Default::default()
        }],
        ..Default::default()
    };
    (ri, LedgerState::default())
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// P7 — the GOLDEN-RETURN matrix.
//
// The households the two independent oracles (OpenTaxSolver, driven directly; and the PSL
// Tax-Calculator) were run over, together with their answers. It lives HERE, in the library, for the
// reason this whole module exists: `btctax-forms` is a DOWNSTREAM crate, and its packet round-trip
// must fill the PDFs for *exactly* the households the oracles blessed. A second copy of this builder
// in the forms crate could drift, and a drifted round-trip would be checking a different taxpayer
// than the one the oracle validated — while still passing.
//
// `include_str!` cannot reach across a crate boundary without breaking `cargo package`, so the JSON is
// exposed from here too. One fixture, one packet, one set of expectations.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The committed oracle answers. Regenerate with `scripts/oracle/gen_goldens.py` (see its header).
pub const GOLDEN_RETURNS_JSON: &str =
    include_str!("../../tests/goldens/full_return_goldens.json");

/// Parse the committed golden matrix.
pub fn golden_households() -> Vec<GoldenHousehold> {
    let g: Goldens = serde_json::from_str(GOLDEN_RETURNS_JSON).expect("the golden file parses");
    g.households
}

#[derive(Debug, Deserialize)]
pub struct Goldens {
    pub households: Vec<GoldenHousehold>,
}

#[derive(Debug, Deserialize)]
pub struct GoldenHousehold {
    pub name: String,
    pub why: String,
    pub inputs: GoldenInputs,
    /// Oracle 1 — **OpenTaxSolver 2024, its own binaries driven directly** (GPL, observe-only).
    ///
    /// Formerly this was `tenforty`, a Python wrapper around OTS. The wrapper turned out to drop two
    /// inputs on the floor — Schedule SE line 8a and the §199A deduction on 1040 line 13 — each of which
    /// OVERSTATES a self-employed filer's tax. Reported upstream (mmacpherson/tenforty#278, fix in #279).
    /// **The engine was never at fault**: driven directly it reproduces btctax to the cent, and every
    /// divergence the wrapper used to force into the list below is gone.
    pub expected_ots: ExpectedOts,
    /// Oracle 2 — PSL Tax-Calculator (CC0). A completely separate lineage.
    pub expected_taxcalc: ExpectedTaxcalc,
}

/// Oracle 1's outputs. `total_tax` is OTS's 1040 line 24 plus the NIIT it computes on Form 8960 —
/// directly comparable to btctax's line 24.
#[derive(Debug, Deserialize)]
pub struct ExpectedOts {
    pub adjusted_gross_income: f64,
    pub taxable_income: f64,
    /// Form 8995 line 15 — the §199A deduction. Committed by BOTH oracles and asserted: pinning only
    /// AGI and taxable income constrains their SUM, so a deduction that is wrong by +X against a QBI
    /// that is wrong by −X would slip through.
    pub qbi_deduction: f64,
    pub income_tax_before_credits: f64,
    pub se_tax: f64,
    pub niit: f64,
    pub additional_medicare_tax: f64,
    pub total_tax: f64,
}

/// The second oracle's outputs. Only the lines whose definitions are unambiguous across engines: we do
/// NOT take its `combined`/`iitax` totals, which bundle payroll tax on W-2 wages that 1040 line 24 does
/// not include.
#[derive(Debug, Deserialize)]
pub struct ExpectedTaxcalc {
    pub adjusted_gross_income: f64,
    pub taxable_income: f64,
    pub qbi_deduction: f64,
    pub income_tax_before_credits: f64,
    pub se_tax: f64,
    pub niit: f64,
    pub additional_medicare_tax: f64,
}

#[derive(Debug, Deserialize)]
pub struct GoldenInputs {
    pub filing_status: String,
    #[serde(default)]
    pub w2_income: f64,
    #[serde(default)]
    pub taxable_interest: f64,
    #[serde(default)]
    pub qualified_dividends: f64,
    #[serde(default)]
    pub ordinary_dividends: f64,
    #[serde(default)]
    pub short_term_capital_gains: f64,
    #[serde(default)]
    pub long_term_capital_gains: f64,
    #[serde(default)]
    pub self_employment_income: f64,
    #[serde(default)]
    pub itemized_deductions: f64,
    /// Schedule A line 5a — state & local INCOME tax. Separate from 5b so the §164(b)(5) SALT cap
    /// can actually be exercised: a lump sum would sail straight past it.
    #[serde(default)]
    pub state_income_tax: f64,
    /// Schedule A line 5b — real estate tax.
    #[serde(default)]
    pub real_estate_tax: f64,
    /// Schedule A line 8a — mortgage interest reported on a Form 1098.
    #[serde(default)]
    pub mortgage_interest: f64,
}

pub fn golden_usd(v: f64) -> Usd {
    Usd::try_from(v).expect("the oracle emits finite figures")
}

/// Build the SAME household in btctax's own input model.
///
/// The mapping is deliberately literal: the oracle's `w2_income` is a W-2's box 1 (and its box 3 / box 5,
/// which is what a real W-2 carries), its capital gains are crypto disposals on the ledger (which is how
/// btctax gets to Schedule D at all), and its `self_employment_income` is business crypto — a Schedule C
/// trade or business, which is the only way btctax produces SE tax.
pub fn build_golden_household(h: &GoldenHousehold) -> (ReturnInputs, LedgerState) {
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
    ri.header.taxpayer = crate::tax::return_inputs::Person {
        first_name: "Golden".into(),
        last_name: "Household".into(),
        ssn: "123456789".into(),
        ..Default::default()
    };
    if status == FilingStatus::Mfj {
        ri.header.spouse = Some(crate::tax::return_inputs::Person {
            first_name: "Golden".into(),
            last_name: "Spouse".into(),
            ssn: "987654321".into(),
            ..Default::default()
        });
    }

    // ★ ONE W-2 carrying the household's whole wage figure. `mfj_two_w2_standard`'s name is about the
    // household, not the paperwork — and the MFJ-SE household's box 3 of $220,000 exceeds what any single
    // employer could report (the $168,600 wage base). Both are fine here and neither affects a number:
    // every engine is told the same thing, and all three read these as PER-PERSON totals — btctax off
    // box 3, OTS off Schedule SE line 8a, Tax-Calculator off `e00200p`. Splitting them across two W-2
    // records would change nothing but the fixture's realism.
    if i.w2_income > 0.0 {
        let w = golden_usd(i.w2_income);
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
            box1_interest: golden_usd(i.taxable_interest),
            ..Default::default()
        });
    }
    if i.ordinary_dividends > 0.0 || i.qualified_dividends > 0.0 {
        ri.div_1099.push(Form1099Div {
            payer: "ORACLE BROKER".into(),
            box1a_ordinary: golden_usd(i.ordinary_dividends), // INCLUDES the qualified subset
            box1b_qualified: golden_usd(i.qualified_dividends),
            ..Default::default()
        });
    }
    if i.itemized_deductions > 0.0
        || i.state_income_tax > 0.0
        || i.real_estate_tax > 0.0
        || i.mortgage_interest > 0.0
    {
        ri.schedule_a = Some(ScheduleAInputs {
            // 5a, the income-tax path of §164(b)(5). The oracles get this as OTS's `A5a` /
            // Tax-Calculator's `e18400`, so all three see the same figure on the same line.
            salt_state_estimated_payments: golden_usd(i.state_income_tax),
            salt_real_estate: golden_usd(i.real_estate_tax),
            mortgage_interest_1098: golden_usd(i.itemized_deductions + i.mortgage_interest),
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
            (golden_usd(gain), Usd::ZERO)
        } else {
            (Usd::ZERO, golden_usd(-gain))
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
            usd_fmv: golden_usd(i.self_employment_income),
            kind: IncomeKind::Mining,
            business: true, // ⇒ Schedule C ⇒ Schedule SE
            pseudo: false,
        });
    }

    (ri, state)
}
