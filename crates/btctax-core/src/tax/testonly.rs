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
