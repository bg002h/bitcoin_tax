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
use crate::tax::questions::FORM_QUESTIONS;
use crate::tax::return_inputs::{
    CharitableClass, CharitableGift, Dependent, Form1099B, Form1099Div, Form1099G, Form1099Int,
    HouseholdHeader, Owner, Payments, Person, ReturnInputs, ScheduleAInputs, ScheduleCInputs, W2,
};
use crate::tax::tables::{
    AmtParams, FullReturnParams, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule,
    SaltLimitation, TaxTable,
};
use crate::tax::types::FilingStatus;
use rust_decimal_macros::dec;

/// ★ P9 — answer every LIVE `FORM_QUESTIONS` declaration that a fixture left blank, so a computing fixture
/// is not tripped by the registry's unanswered screen (§3.1 churn note). Derived from the registry itself,
/// so a new always-live declaration is covered here with zero new fixture edits — the whole point of P9.
///
/// Every declaration answers "no" EXCEPT the mixed-use-mortgage question, which answers "yes" (all of the
/// loan was used to buy/build/improve): a fixture that reports mortgage interest means to DEDUCT it, and
/// "yes" is what keeps line 8a full and the box unchecked (a "no" would zero 8a — §2.7). A deliberate
/// answer already set by the caller is preserved (the loop only fills `None`).
pub fn answer_all_live_declarations(ri: &mut ReturnInputs) {
    // ★★★ R3 — THE DOCUMENT CENSUS FIRST, and its answer is READ OFF THE FIXTURE'S OWN ROWS.
    //
    // The generic loop below would answer every census row at its neutral (`false` — "I received
    // none"), which on a fixture that transcribes a W-2 is exactly the `DocumentCensusContradicted`
    // state: a "no" the data contradicts. So each countable row is answered from what the fixture
    // actually carries — `Some(true)` where a row exists, and the neutral `false` otherwise.
    //
    // ★ This is legitimate for a FIXTURE helper and would not be for a product surface: the helper
    //   is not the filer, and it is deriving a fixture's intent from the fixture's own data, not
    //   answering for a human. It is also what makes §5.7's *"TY2024 fixtures gain
    //   `documents.w2 = Some(true)`"* true with zero per-fixture edits.
    reconcile_document_census(ri);
    for q in FORM_QUESTIONS {
        if (q.live)(ri) && (q.get)(ri).is_none() {
            (q.set)(ri, q.neutral); // ★ declared per question — see FormQuestion::neutral
        }
    }
}

/// ★★★ **R3 — make a fixture's document census COHERENT with the rows the fixture carries.**
///
/// Flips a COUNTABLE row to `Some(true)` when the return holds rows of that document, whether the row
/// was unanswered or was answered `Some(false)` by an earlier
/// [`answer_all_live_declarations`] pass. Nothing else is touched: a `Some(true)` stays, and a row
/// with no section (`declared_rows` = `None`) is left to the caller.
///
/// **Why it is separate and re-runnable.** Fixture builders answer the declarations and THEN shape
/// the return, so the answering pass cannot see the rows the shape is about to add — and a `false`
/// beside a transcribed W-2 is exactly `RefuseReason::DocumentCensusContradicted`. Calling this after
/// the shape fixes that without re-answering anything the shape deliberately BLANKED, which is the
/// property several fixtures depend on (they un-answer one declaration to prove the screen refuses).
///
/// ★ Legitimate for a fixture helper and not for a product surface: it derives a fixture's intent
/// from the fixture's own data; it never answers for a human.
pub fn reconcile_document_census(ri: &mut ReturnInputs) {
    for row in crate::tax::document_census::DocumentRow::ALL {
        if crate::tax::document_census::declared_rows(ri, *row).is_some_and(|n| n > 0)
            && ri.documents.get(*row) != Some(true)
        {
            ri.documents.set(*row, Some(true));
        }
    }
}

/// ★★★ **R9 / T6 — make a fixture's Digital Assets ANSWER coherent with the LEDGER it is computed
/// against**, exactly as [`reconcile_document_census`] does for the document rows.
///
/// [`answer_all_live_declarations`] answers every live declaration at its declared neutral, and the
/// Digital Assets question's neutral is `false` — which on a fixture whose ledger holds a 2026
/// disposal is precisely `RefuseReason::DigitalAssetAnswerContradictsLedger`, a *"No"* the data
/// contradicts. The helper cannot see the ledger (it takes only `&mut ReturnInputs`), so the flip
/// lives here, where the caller has both.
///
/// **ONE DIRECTION ONLY**, exactly like [`reconcile_document_census`]: a `None` or a `Some(false)`
/// becomes `Some(true)` when the year's ledger witnesses a qualifying event, and nothing else is
/// touched. The predicate is the SAME one the refusal reads, never a second copy of it.
///
/// ★★ **Why one direction.** The other direction would silently overwrite a fixture's deliberate
///    `Some(true)` — and the case is real, not hypothetical: `extension.rs`'s pseudo-reconcile
///    fixture stores its return BEFORE `pseudo_set_mode` is switched on, so the projection the
///    helper can see holds no disposal while the projection the export computes does. A two-way
///    flip re-answered that fixture `No` and the export then refused on the fixture instead of on
///    the attestation gate it exists to measure.
///
/// ★ Legitimate for a fixture helper and not for a product surface, for the same reason
///   `reconcile_document_census` is: it derives a fixture's intent from the fixture's own data. A
///   product surface that did this would be answering a §6065 declaration for a human.
pub fn reconcile_digital_asset_activity(ri: &mut ReturnInputs, state: &LedgerState, year: i32) {
    if crate::tax::return_1040::digital_asset_activity(state, year) {
        ri.digital_asset_activity = Some(true);
    }
}

/// [`answer_all_live_declarations`] as a by-value wrapper, for fixtures that pass a `&ReturnInputs { ... }`
/// temporary straight into `set` and expect it to COMPUTE: `set(conn, year, &testonly::answered(ReturnInputs { .. }))`.
pub fn answered(mut ri: ReturnInputs) -> ReturnInputs {
    answer_all_live_declarations(&mut ri);
    ri
}
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
        salt: SaltLimitation::FlatCap {
            cap: dec!(10000),
            cap_mfs: dec!(5000),
        },
        kiddie_unearned_threshold: dec!(2600),
        elective_deferral_limit: dec!(23000),
        ftc_ceiling: dec!(300),
        qbi_ti_threshold_unmarried: dec!(191950),
        qbi_ti_threshold_married: dec!(383900),
        qbi_phase_in_range_unmarried: dec!(50000),
        qbi_phase_in_range_married: dec!(100000),
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
            mfs_kicker_start: dec!(875950),
            mfs_kicker_max: dec!(66650),
            exemption_phaseout_rate: dec!(0.25),
            mfs_kicker_rate: dec!(0.25),
            rate_26: dec!(0.26),
            rate_28: dec!(0.28),
            rate_28_subtrahend: dec!(4652),
            rate_28_subtrahend_mfs: dec!(2326),
        },
    }
}

/// The real TY2024 ordinary + §1(h) schedules, transcribed from **Rev. Proc. 2023-34** — §3.01 Tables
/// 1–4 (rates) and §3.03 (the §1(h) breakpoints).
///
/// ★★★ **MFS and HoH were added because they were UNWITNESSED, and the closure had to be a
/// TRANSCRIPTION.** `shipped_tables_are_the_validated_tables.rs` found that the binary ships ordinary
/// brackets for all five statuses while this table carried only Single and MFJ — so the rates applied
/// to a real married-filing-separately or head-of-household filer had never been compared to anything.
/// Pasting the shipped numbers across would have made that test pass by construction and proved
/// nothing: two artifacts agree only if they were derived **independently**. These come from the
/// revenue procedure the shipped table's own `source` field cites, read off its text layer.
/// ★ The independence is not just asserted — transcribing Single and MFJ from the same pass reproduced
/// the committed values exactly, which is what says the reading is right.
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
    // TABLE 2 - Section 1(j)(2)(B) - Heads of Households.
    // ★ The revenue procedure's own "the excess over" column carries a TYPO in Tables 2, 3 and 4 — it
    //   reads "$191,150" where the bracket boundary column reads "Over $191,950 but not over ...". The
    //   BOUNDARY is authoritative (and agrees with Single and MFJ, which are unaffected); the running
    //   "plus $X" figures are a convenience restatement, not the bracket definition.
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(16550), dec!(0.12)),
                bracket(dec!(63100), dec!(0.22)),
                bracket(dec!(100500), dec!(0.24)),
                bracket(dec!(191950), dec!(0.32)),
                bracket(dec!(243700), dec!(0.35)),
                bracket(dec!(609350), dec!(0.37)),
            ],
        },
    );
    // TABLE 4 - Section 1(j)(2)(D) - Married Individuals Filing Separate Returns.
    // ★ Identical to Single through the 35% bracket and then DIVERGES: the 37% bracket starts at
    //   $365,600, not $609,350 — half the joint figure, per §1(j)(2)(D). That single row is the whole
    //   reason a copy-paste closure would have been worthless.
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(11600), dec!(0.12)),
                bracket(dec!(47150), dec!(0.22)),
                bracket(dec!(100525), dec!(0.24)),
                bracket(dec!(191950), dec!(0.32)),
                bracket(dec!(243725), dec!(0.35)),
                bracket(dec!(365600), dec!(0.37)),
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
    // §3.03 Maximum Capital Gains Rate (§1(h), §1(j)(5)).
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(47025),
            max_fifteen: dec!(291850),
        },
    );
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(63000),
            max_fifteen: dec!(551350),
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
    let mut ri = ReturnInputs {
        // ★★ §G-15 — this fixture is a TY2024 household (it is paired with `ty2024_table` /
        // `ty2024_params` throughout), so it states 2024. A year-scoped question is therefore
        // correctly NOT live on it: `HasIncomeExclusion` computes modified AGI, which TY2024's
        // `FlatCap` never reads, so a TY2024 return legitimately carries no answer to it.
        tax_year: 2024,
        filing_status: FilingStatus::Mfj,
        // ★★ §G-21 — Form 8283 Section B lines 5a/5b/5c, asked ONCE for the whole return. This
        // household donates over $5,000, so a Section B files and the answer is MANDATORY: unanswered
        // or `Some(true)` REFUSES the year, because a restricted gift's §170 deduction is smaller than
        // the full fair market value btctax computes. "No strings" is the ordinary case.
        donations_had_restrictions: Some(false),
        // ★★ §170(f)(8) — the contemporaneous written acknowledgment, asked ONCE for the whole return.
        // This household itemizes and both its gifts (a $5,000 cash gift and a crypto donation) are far
        // over $250, so the answer is MANDATORY: unanswered or `Some(false)` REFUSES the year, because
        // §170(f)(8)(A) makes the acknowledgment a condition of the deduction itself. Holding one is
        // the ordinary case. ★ NOT covered by `answer_all_live_declarations`, which walks the
        // DECLARATION registry — this is a class-(B) skippable, mandatory only in `screen_absolute`.
        charitable_cwa_obtained: Some(true),
        // Schedule D line 20 / Schedule A line 9 — not filing Form 4952, so line 20 is "Yes" and the
        // tax routes to the Qualified Dividends and Capital Gain Tax Worksheet. `answered()` DOES
        // cover this one (it is a class-(A) declaration), but it is stated explicitly because the
        // kitchen sink is the oracle fixture: its answers must be visible, not inferred.
        filing_form_4952: Some(false),
        // §G-22/B11 — the scope attestation. ANSWERED, never defaulted: `None` refuses, and a fixture
        // that let it default would be re-asserting the silence the question exists to break.
        other_out_of_scope_income: Some(false),
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
            // ★ §G-28/B1b — ANSWERED, and both answers are facts about this synthetic household
            //   rather than fixture grease: crypto mining is not one of §199A(d)(2)'s specified
            //   service fields, and a miner is not a patron of an agricultural cooperative.
            is_sstb: Some(false),
            is_cooperative_patron: Some(false),
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
                lot_acquired_at: date!(2020 - 01 - 01), // = the lot's own date in these fixtures (spec 1099-DA T2)
                wallet: WalletId::SelfCustody {
                    label: "cold".into(),
                },
                pseudo: false,
            }],
            fee_mini_disposition: false,
        }],
        ..Default::default()
    };

    answer_all_live_declarations(&mut ri);
    // ★★★ R9 / T6 — and the Digital Assets ANSWER, read off THIS fixture's own ledger. The household
    //     disposes 1 BTC on 2024-05-01 and recognizes mining income, so its answer is `Yes`; the
    //     neutral `No` the loop above writes would be a "no" its own ledger contradicts.
    let y = ri.tax_year;
    reconcile_digital_asset_activity(&mut ri, &state, y);
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
    let mut ri = ReturnInputs {
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
    answer_all_live_declarations(&mut ri);
    (ri, LedgerState::default())
}

/// **§G-6 — a household that OWES AMT and must attach Form 6251.**
///
/// ★★★ Neither `kitchen_sink_household` nor `w2_only_household` triggers the AMT, so a test that
/// merely iterated those two would assert `None == None` twice and pass forever while the emitter was
/// broken. This fixture exists so the Schedule-2-line-2 biconditional has a TRUE case to discriminate
/// against — the vacuity was real and was caught by probing, not by reading.
///
/// The mechanism is the post-TCJA one that survives for a filer with no ISOs: a large **long-term**
/// gain plus a modest ordinary slice. Both systems tax the gain at the same preferential rates, so
/// Part III runs, but AMTI of ~$2.3M has phased the $85,700 exemption away entirely and the ordinary
/// slice meets a flat 26/28% instead of the regular graduated brackets. Line 7 clears line 10 by a
/// few thousand dollars — which is what *Who Must File* condition 1 tests, and it is deliberately a
/// NARROW margin, because a fixture that owed AMT by a mile would also pass with the exemption
/// phase-out miscomputed.
///
/// ★ Ledger-free on purpose: the gain arrives as a §G-28/B4 Form 1099-B, so the fixture exercises the
/// packet without depending on a lot construction that a basis-method change could move underneath it.
///
/// ★★★ THIS IS THE VECTOR BOTH ORACLES WITNESSED — `design/direction/G6-AMT-ORACLE-VALIDATION.md`.
/// btctax files AMT = $11,322 and total tax = $481,225 on it; Tax-Calculator reproduces the AMT to $3
/// once its own open line-2a defect (PSLmodels#3108) is corrected for, and OpenTaxSolver reproduces the
/// whole Part I/Part III structure. Keeping the fixture identical to the validated vector is the point:
/// a fixture that drifts from the vector its oracles blessed is validating a different taxpayer.
///
/// ★ It also carries a **Schedule C**, so the packet contains Form 8995-A (sequence 55A) *and* Form
/// 6251 (sequence 32) at once — which is what lets the attachment-order guard discriminate. Without a
/// household holding both, Form 6251 stapling after Form 8995 passes every ordering test (r1 Minor).
pub fn amt_owing_household() -> (ReturnInputs, LedgerState) {
    let mut ri = ReturnInputs {
        filing_status: FilingStatus::Single,
        header: HouseholdHeader {
            // ★ An ALLOWED synthetic SSN (`scripts/pii-scan-generic.sh`'s list). A fresh fake number
            //   is not free here: the scan runs over HEAD, so inventing one reds the gate after the
            //   commit lands, which is exactly how this fixture first broke it.
            taxpayer: person("Dana", "Quill", "987-65-4321", "Miner"),
            address_street: "77 Ridge Rd".into(),
            address_city: "Boulder".into(),
            address_state: "CO".into(),
            address_zip: "80301".into(),
            ..Default::default()
        },
        schedule_c: Some(ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "Bitcoin mining".into(),
            naics_code: "518210".into(),
            other_gross_receipts: dec!(85000),
            expenses: Usd::ZERO,
            is_sstb: Some(false),
            is_cooperative_patron: Some(false),
            // ★ Both ANSWERED as zero, which is a fact about this household (no employees, no
            //   qualified property) and not fixture grease — it is also what makes Form 8995-A
            //   Part II cap the §199A deduction at $0, the figure Tax-Calculator independently
            //   confirms.
            qbi_w2_wages: Some(Usd::ZERO),
            qbi_ubia: Some(Usd::ZERO),
            ..Default::default()
        }),
        b_1099: vec![Form1099B {
            payer: "BROKER LLC".into(),
            long_term_proceeds: dec!(2500000),
            long_term_basis: dec!(500000),
            basis_reported_and_no_adjustments: Some(true),
            ..Default::default()
        }],
        ..Default::default()
    };
    answer_all_live_declarations(&mut ri);
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
pub const GOLDEN_RETURNS_JSON: &str = include_str!("../../tests/goldens/full_return_goldens.json");

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
    #[serde(default)]
    pub deduction_taken: Option<f64>, // 1040 L12
    #[serde(default)]
    pub salt_capped: Option<f64>, // Sch A L5e
    #[serde(default)]
    pub sch_d_to_l7: Option<f64>, // 1040 L7 (signed)
    #[serde(default)]
    pub qbi_cap_l12: Option<f64>, // 8995 L12 — OTS single-witness/WEAK (I1): driver-hand-fed, NOT an independent check; §14.2 closure = follow-up
    /// **1040 L17 — the AMT** (⊇ Form 6251 line 11). `None` means OTS is DISQUALIFIED as a witness on
    /// this household, not that the AMT is $0 — its TY2024 solver carries the 2023 §55(d)(3) MFS
    /// constants and applies no §170(b) cash ceiling (both fixed in OTS 2025). `ots_direct.py` gates
    /// it and records the reason. Never coerce this to 0.0.
    #[serde(default)]
    pub amt: Option<f64>,
    // provenance leaves for the §6.2(b) predicate (Table_btctax inputs):
    #[serde(default)]
    pub qual_div_l3a: Option<f64>, // 1040 L3a
    #[serde(default)]
    pub net_ltcg_qd_exclusive: Option<f64>, // §1(h) term, QD-EXCLUSIVE (r5-N2)
    // C1 cross-foot legs — OTS only (taxcalc has no split):
    #[serde(default)]
    pub se_l10_oasdi: Option<f64>, // Sch SE L10 (OASDI leg)
    #[serde(default)]
    pub se_l11_medicare: Option<f64>, // Sch SE L11 (Medicare leg)
    #[serde(default)]
    pub f8959_l7: Option<f64>, // 8959 L7 leg
    #[serde(default)]
    pub f8959_l13: Option<f64>, // 8959 L13 leg
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
    #[serde(default)]
    pub deduction_taken: Option<f64>, // 1040 L12
    #[serde(default)]
    pub salt_capped: Option<f64>, // Sch A L5e
    #[serde(default)]
    pub sch_d_to_l7: Option<f64>, // 1040 L7 (signed)
    #[serde(default)]
    pub qbi_cap_l12: Option<f64>, // 8995 L12 — OTS single-witness/WEAK (I1): driver-hand-fed, NOT an independent check; §14.2 closure = follow-up
    /// **Tax-Calculator `c09600` — the AMT.** `None` when the goldens predate the AMT comparison.
    /// ★ Known-suspect for STANDARD-DEDUCTION filers: taxcalc's AMTI omits Form 6251 line 2a's
    /// standard-deduction add-back (PSLmodels/Tax-Calculator#3108, open). Treat a divergence here on
    /// a non-itemizing household as the ORACLE's, pending that issue.
    #[serde(default)]
    pub amt: Option<f64>,
    // provenance leaves for the §6.2(b) predicate (Table_btctax inputs):
    #[serde(default)]
    pub qual_div_l3a: Option<f64>, // 1040 L3a
    #[serde(default)]
    pub net_ltcg_qd_exclusive: Option<f64>, // §1(h) term, QD-EXCLUSIVE (r5-N2)
    #[serde(default)]
    pub total_tax: Option<f64>, // OTS's is required f64; taxcalc's is optional — §6.4 M-4
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
    /// **Schedule A line 11 — a §170(b)(1)(G) cash gift to a public charity** (the 60%-of-AGI class).
    ///
    /// ★ It exists so the corpus can witness a NON-interaction: charitable deductions appear on Form
    /// 8960 only at line 18b, inside Part III's *Estates and Trusts* block, and §170 is not in Reg.
    /// §1.1411-4(f)'s properly-allocable list — so a gift must move 1040 line 15 and NOT one line of
    /// Form 8960. Until a corpus household combined a large gift with NIIT, neither engine had ever
    /// been asked (N-6/N-7).
    ///
    /// ★★ **Keep it under the §170(b) 60%-of-AGI ceiling.** OTS 2024 applies no such ceiling, so a
    /// household that crosses it loses an oracle — the fate that disqualified the Form 6251 fixture's
    /// V2b. `gen_goldens.py` has no equivalent guard, so the constraint lives in the corpus cell's
    /// own `why`.
    #[serde(default)]
    pub charitable_cash: f64,
}

pub fn golden_usd(v: f64) -> Usd {
    Usd::try_from(v).expect("the oracle emits finite figures")
}

/// A header that has **answered** the §63(c)(5) "someone can claim you as a dependent" question — with a
/// "no", which is what an ordinary filer says.
///
/// Every `ReturnInputs` fixture needs this. `Default` leaves the flag `None` on purpose (D-8: unanswered
/// is not "no"), and an unanswered return now REFUSES, so a fixture that skips it is testing the refusal
/// rather than whatever it meant to test. Answering it in `Default` would reinstate the very guess we
/// removed — which is why this is a separate, explicit call.
pub fn not_a_dependent() -> HouseholdHeader {
    HouseholdHeader {
        can_be_claimed_as_dependent_taxpayer: Some(false),
        // §G-9: an ordinary filer has ALSO answered that they did not die during the tax year — the
        // answer that reproduces the pre-§G-9 §63(f) behaviour, so no aged-box figure moves.
        taxpayer_died_during_year: Some(false),
        ..Default::default()
    }
}

/// Build the SAME household in btctax's own input model.
///
/// The mapping is deliberately literal: the oracle's `w2_income` is a W-2's box 1 (and its box 3 / box 5,
/// which is what a real W-2 carries), its capital gains are crypto disposals on the ledger (which is how
/// btctax gets to Schedule D at all), and its `self_employment_income` is business crypto — a Schedule C
/// trade or business, which is the only way btctax produces SE tax.
pub fn build_golden_household(h: &GoldenHousehold) -> (ReturnInputs, LedgerState) {
    build_golden_return(&h.inputs)
}

/// Build btctax's `(ReturnInputs, LedgerState)` from the oracle [`GoldenInputs`] **alone** — the core of
/// [`build_golden_household`], factored out (T7) so the §9 oracle-sweep harness can assemble the SAME
/// return from a bare `GoldenInputs` read off stdin. The harness has no oracle-expectation fields with
/// which to fill a whole `GoldenHousehold`, and `build_golden_household` never read anything but
/// `h.inputs` — so `build_golden_household(h)` is now exactly `build_golden_return(&h.inputs)`, and the
/// two produce an IDENTICAL return by construction.
pub fn build_golden_return(i: &GoldenInputs) -> (ReturnInputs, LedgerState) {
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
    // Nobody can claim a golden household — but it must SAY so (D-8: unanswered refuses).
    ri.header.can_be_claimed_as_dependent_taxpayer = Some(false);
    // ★★★ **FR-29 (SPEC §7 / G10) — every golden household is an ADULT, and it must SAY so too.**
    //
    // Four of the sweep's households carry capital gains over the §1(g) threshold, and with no date
    // of birth on file Form 8615's condition 3 is UNKNOWN — so the corpus started refusing them
    // rather than reconciling. **The floor was not relaxed and no answer was invented:** a date of
    // birth proves condition 3 FALSE by arithmetic (§5.1), which is what an adult household's date
    // of birth actually does.
    //
    // ★ The band is `[year - 63, year - 24]` and 1980 is inside it for BOTH supported years
    //   (2024 ⇒ [1961, 2000], 2025 ⇒ [1962, 2001]), so it can neither leave the filer under 24 (the
    //   question comes back) nor reach §63(f)'s age-65 addition (which would move the standard
    //   deduction and every golden built on it). It is therefore invisible to both oracles: neither
    //   `GoldenInputs` nor either engine models age at all.
    //
    // ★★ THE HAZARD, NAMED: if the corpus ever gains a genuinely-under-24 household, this line would
    //    exempt it from §1(g) by fiat. `GoldenInputs` carries no age today, so the day it does, this
    //    line must read it instead of assuming.
    ri.header.taxpayer.date_of_birth = Some(time::macros::date!(1980 - 05 - 05));
    if status == FilingStatus::Mfj {
        ri.header.spouse = Some(crate::tax::return_inputs::Person {
            first_name: "Golden".into(),
            last_name: "Spouse".into(),
            ssn: "987654321".into(),
            ..Default::default()
        });
        ri.header.can_be_claimed_as_dependent_spouse = Some(false);
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
        || i.charitable_cash > 0.0
    {
        ri.schedule_a = Some(ScheduleAInputs {
            // 5a, the income-tax path of §164(b)(5). The oracles get this as OTS's `A5a` /
            // Tax-Calculator's `e18400`, so all three see the same figure on the same line.
            salt_state_estimated_payments: golden_usd(i.state_income_tax),
            salt_real_estate: golden_usd(i.real_estate_tax),
            mortgage_interest_1098: golden_usd(i.itemized_deductions + i.mortgage_interest),
            // Schedule A line 11 — OTS's `A11` (cash/check charity, NOT `A16` "other", which sails
            // past Schedule A's own handling of the gift) and Tax-Calculator's `e19800`.
            charitable: if i.charitable_cash > 0.0 {
                vec![CharitableGift {
                    class: CharitableClass::Cash60,
                    amount: golden_usd(i.charitable_cash),
                }]
            } else {
                Vec::new()
            },
            ..Default::default()
        });
    }
    // ★ §170(f)(8) — an itemizing return claiming a gift of $250 or more must state whether it holds
    //   a contemporaneous written acknowledgment, and P4 refuses one that has not. This household
    //   HOLDS one; that is a fact about the fixture, stated rather than defaulted, and it is not
    //   grease — a corpus cell that refused would simply be dropped by the admission loop and the
    //   non-interaction would go on being unwitnessed.
    if i.charitable_cash > 0.0 {
        ri.charitable_cwa_obtained = Some(true);
    }
    if i.self_employment_income > 0.0 {
        ri.schedule_c = Some(ScheduleCInputs {
            owner: Owner::Taxpayer,
            business_description: "Bitcoin mining".into(),
            // ★ §G-28/B1b — see the kitchen-sink fixture above: mining is neither an SSTB nor a
            //   cooperative patronage, so these are the household's real answers.
            is_sstb: Some(false),
            is_cooperative_patron: Some(false),
            ..Default::default()
        });
    }
    // Schedule B Part III must be answered when Schedule B files.
    ri.foreign_accounts = Some(false);
    ri.foreign_trust = Some(false);
    answer_all_live_declarations(&mut ri);

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
                lot_acquired_at: if term == Term::LongTerm {
                    date!(2020 - 01 - 01)
                } else {
                    date!(2024 - 01 - 02)
                }, // = the lot's own date in this fixture (spec 1099-DA T2)
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

    // ★★★ R9 / T6 — the Digital Assets ANSWER, read off the ledger this very function just built.
    //     `answer_all_live_declarations` above answered the neutral `No`, which is right for the
    //     wage-only households and a "no" the data contradicts for every household that carries a
    //     capital gain or SE income — the corpus's whole crypto side.
    //
    // ★ INVISIBLE TO BOTH ORACLES by construction: `GoldenInputs` models no such box, and neither
    //   engine reads one. It moves no compared line; it only stops the return refusing.
    //
    // ★ 2024 is the year every leg this function pushes is dated (`date!(2024 - ..)` above), which is
    //   the only year this corpus is swept for (`btctax-oracle-harness`'s `YEAR`).
    reconcile_digital_asset_activity(&mut ri, &state, 2024);

    (ri, state)
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// The VALIDATED tax tables — one per shipped year, transcribed INDEPENDENTLY from each year's
// revenue procedure. `shipped_tables_are_the_validated_tables.rs` compares `tax_tables.rs` against
// these, so a figure here that was copied from there would make that test assert a number equals
// itself. They are placed ABOVE the test module deliberately: appended below it they were `items
// after a test module`, which clippy rejects.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The real TY2025 ordinary + §1(h) schedules, transcribed from **Rev. Proc. 2024-40**, 2024-45 I.R.B.
/// 1100 — §2.01 Tax Rate Tables (§1(j)(2)(A)–(E)) Tables 1–4, and §2.03 Maximum Capital Gains Rate
/// (§1(h), §1(j)(5)). The gift figures are §2.43(1) (§2503 annual exclusion) and §2.41 (§2010 basic
/// exclusion amount). The Social Security wage base is **not in the revenue procedure at all** — it is
/// an SSA determination, taken here from 89 FR 85279 (Vol. 89 No. 207, Friday 25 October 2024),
/// committed at `legal/text/federal-register/SSA_COLA_Determinations_2025.txt`.
///
/// ★★★ **Transcribed from the procedure's TEXT LAYER, never from `tax_tables.rs`.** The shipped table
/// was not opened until this function was complete; had it been copied across, the equality in
/// `shipped_tables_are_the_validated_tables.rs` would assert that a number equals itself — an echo,
/// not a witness — and would certify a typo forever. TY2025 was the file's own worst exposure: only
/// 8 of 28 shipped bracket thresholds were asserted by anything, the whole interior of the MFJ and
/// HoH schedules among them.
///
/// ★★ **Every threshold below is independently confirmed by the procedure's own cumulative-tax
/// column**, which is a second, redundant encoding of the same breakpoints: reading TABLE 2's
/// "$55,484 plus 32%" backwards gives `38,460 + 0.32 × (250,500 − 197,300) = 55,484`, which pins the
/// HoH 35% floor at **$250,500** and rules out $250,525 (that would yield $55,492). All 24 interior
/// rows were checked this way and all 24 agree — so a mis-keyed digit is caught by arithmetic rather
/// than by a re-read, which is the failure mode `CLAUDE.md` records for Form 6251 line 33.
///
/// ★ **`Qss` is deliberately absent**, matching [`ty2024_table`]: §1(j)(2)(A) gives a qualifying
/// surviving spouse the joint schedule (TABLE 1 is titled "Married Individuals Filing Joint Returns
/// **and Surviving Spouses**"), and `TaxTable::key` normalises `Qss → Mfj` at lookup. That absence is
/// lawful only when BOTH sides omit it, which the ratchet checks rather than assumes.
pub fn ty2025_table() -> TaxTable {
    let mut ordinary = BTreeMap::new();
    // §2.01 TABLE 3 - Section 1(j)(2)(C) - Unmarried Individuals (other than Surviving Spouses and
    // Heads of Households).
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(11925), dec!(0.12)),
                bracket(dec!(48475), dec!(0.22)),
                bracket(dec!(103350), dec!(0.24)),
                bracket(dec!(197300), dec!(0.32)),
                bracket(dec!(250525), dec!(0.35)),
                bracket(dec!(626350), dec!(0.37)),
            ],
        },
    );
    // §2.01 TABLE 1 - Section 1(j)(2)(A) - Married Individuals Filing Joint Returns and Surviving
    // Spouses.
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(23850), dec!(0.12)),
                bracket(dec!(96950), dec!(0.22)),
                bracket(dec!(206700), dec!(0.24)),
                bracket(dec!(394600), dec!(0.32)),
                bracket(dec!(501050), dec!(0.35)),
                bracket(dec!(751600), dec!(0.37)),
            ],
        },
    );
    // §2.01 TABLE 4 - Section 1(j)(2)(D) - Married Individuals Filing Separate Returns.
    // ★ Identical to Single through the 35% bracket and then DIVERGES: the 37% bracket starts at
    //   $375,800, exactly half the joint $751,600, per §1(j)(2)(D). Same shape as TY2024, and the
    //   same reason a copy-paste closure across statuses would be worthless.
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(11925), dec!(0.12)),
                bracket(dec!(48475), dec!(0.22)),
                bracket(dec!(103350), dec!(0.24)),
                bracket(dec!(197300), dec!(0.32)),
                bracket(dec!(250525), dec!(0.35)),
                bracket(dec!(375800), dec!(0.37)),
            ],
        },
    );
    // §2.01 TABLE 2 - Section 1(j)(2)(B) - Heads of Households.
    // ★ The 35% floor is $250,500 — TWENTY-FIVE DOLLARS below Single/MFS's $250,525, and the two are
    //   equal in the 24% and 32% rows either side of it, so the digit is easy to lose. TABLE 2's own
    //   "$55,484 plus 35%" is what settles it (see the arithmetic note on this function).
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(17000), dec!(0.12)),
                bracket(dec!(64850), dec!(0.22)),
                bracket(dec!(103350), dec!(0.24)),
                bracket(dec!(197300), dec!(0.32)),
                bracket(dec!(250500), dec!(0.35)),
                bracket(dec!(626350), dec!(0.37)),
            ],
        },
    );
    // §2.03 Maximum Capital Gains Rate (§1(h), §1(j)(5)) — the "Maximum Zero Rate Amount" and
    // "Maximum 15% Rate Amount" columns, verbatim.
    let mut ltcg = BTreeMap::new();
    // "All Other Individuals".
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(48350),
            max_fifteen: dec!(533400),
        },
    );
    // "Married Individuals Filing Joint Returns and Surviving Spouse".
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(96700),
            max_fifteen: dec!(600050),
        },
    );
    // "Married Individuals Filing Separate Returns". ★ The 15% ceiling is $300,000 — a round number,
    // and NOT half of the joint $600,050 ($300,025). The procedure prints $300,000; it governs.
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(48350),
            max_fifteen: dec!(300000),
        },
    );
    // "Heads of Household".
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(64750),
            max_fifteen: dec!(566700),
        },
    );
    TaxTable {
        year: 2025,
        source: "TEST-TY2025",
        ordinary,
        ltcg,
        gift_annual_exclusion: dec!(19000),
        ss_wage_base: dec!(176100),
        gift_lifetime_exclusion: dec!(13_990_000),
    }
}

/// The real TY2026 ordinary + §1(h) schedules, transcribed from **Rev. Proc. 2025-32**, 2025-45 I.R.B.
/// — §4.01 Tax Rate Tables (§1(j)(2)(A)–(E)) Tables 1–4, and §4.03 Maximum Capital Gains Rate (§1(h),
/// §1(j)(5)). The gift annual exclusion is §4.42(1) (§2503). The gift **lifetime** figure is NOT in
/// §4 at all this year: OBBBA §70106 amended §2010(c)(3) to set the basic exclusion amount at a flat
/// $15,000,000 for calendar year 2026, stated in **§2.14** (SECTION 2, CHANGES) rather than as an
/// inflation adjustment. The Social Security wage base is not in the revenue procedure at all — it is
/// an SSA determination, taken here from **90 FR 49047, 49050** (Vol. 90 No. 210, Monday 3 November
/// 2025), committed at `legal/text/federal-register/SSA_COLA_Determinations_2026.txt`.
///
/// ★★★ **Transcribed from the procedure's TEXT LAYER, never from `tax_tables.rs`.** The shipped table
/// was not opened until this function was complete; had it been copied across, the equality in
/// `shipped_tables_are_the_validated_tables.rs` would assert that a number equals itself — an echo,
/// not a witness — and would certify a typo forever.
///
/// ★★ **TY2026 is the OBBBA year, and Rev. Proc. 2025-32 is a MODIFYING procedure, not a fresh one.**
/// Its §1 says it "modifies certain sections of Rev. Proc. 2024-40 … to reflect the amendments to the
/// Internal Revenue Code … by Public Law 119-21 … (OBBBA)", for the Code "as in effect on October 9,
/// 2025"; §6 says flatly "Rev. Proc. 2024-40 is modified." Three consequences bear on this table:
///
///   1. **§2.01** — OBBBA §70101 made the post-TCJA §1(j) rate tables **permanent**. The seven rates
///      10/12/22/24/32/35/37% "remain in effect", so the SHAPE of every schedule below is the same
///      seven-bracket shape as TY2024/TY2025 and there is no 2026 sunset back to pre-TCJA §1(a)–(d).
///      That sunset is the single largest thing that could have gone wrong in a TY2026 port.
///   2. **What it REMOVES is retroactive to 2025, not 2026.** §3.01 removes §2.15(1) of Rev. Proc.
///      2024-40 (the 2025 standard deduction, superseded by §63(c)(7): MFJ $31,500 / HoH $23,625 /
///      Single $15,750 / MFS $15,750) and §3.02 removes §2.25 (the 2025 §179 expensing limits,
///      superseded by $2,500,000 / $4,000,000). ★ **Neither removal touches anything in this
///      function** — §2.01 (rate tables) and §2.03 (capital gains) of Rev. Proc. 2024-40 are NOT
///      removed, because they were only ever the TY2025 figures and the TY2026 ones live in §4 here.
///      Recorded because "what did it supersede" is exactly the question a silent port never asks.
///   3. §2.04 removes the §36B(f)(2)(B) adjustment outright; not a field of this table.
///
/// ★★ **Every threshold below is independently confirmed by the procedure's own cumulative-tax
/// column**, which is a second, redundant encoding of the same breakpoints. Reading TABLE 2's
/// "$56,631 plus 35%" backwards gives `39,207 + 0.32 × (256,200 − 201,750) = 56,631`, which pins the
/// HoH 35% floor at **$256,200** and rules out $256,225 (that would yield $56,639). All 24 interior
/// rows were checked this way, by script, and all 24 agree — so a mis-keyed digit is caught by
/// arithmetic rather than by a re-read, which is the failure mode `CLAUDE.md` records for Form 6251
/// line 33.
///
/// ★ **Statuses round INDEPENDENTLY, and TY2026 is worse than TY2025 for it.** In TY2025 exactly one
/// HoH row sat $25 below Single's; in TY2026 **two** do — the 32% floor ($201,750 vs $201,775) and
/// the 35% floor ($256,200 vs $256,225) — while the 24% floor ($105,700) and the 37% floor
/// ($640,600) are IDENTICAL across the two schedules, on both sides of the divergence. Never compute
/// one status from another.
///
/// ★ **`Qss` is deliberately absent**, matching [`ty2024_table`] and [`ty2025_table`]: §1(j)(2)(A)
/// gives a qualifying surviving spouse the joint schedule (TABLE 1 is titled "Married Individuals
/// Filing Joint Returns and Surviving Spouses"), and `TaxTable::key` normalises `Qss → Mfj` at
/// lookup. That absence is lawful only when BOTH sides omit it, which the ratchet checks rather than
/// assumes.
pub fn ty2026_table() -> TaxTable {
    let mut ordinary = BTreeMap::new();
    // §4.01 TABLE 3 - Section 1(j)(2)(C) - Unmarried Individuals (other than Surviving Spouses and
    // Heads of Households).
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(12400), dec!(0.12)),
                bracket(dec!(50400), dec!(0.22)),
                bracket(dec!(105700), dec!(0.24)),
                bracket(dec!(201775), dec!(0.32)),
                bracket(dec!(256225), dec!(0.35)),
                bracket(dec!(640600), dec!(0.37)),
            ],
        },
    );
    // §4.01 TABLE 1 - Section 1(j)(2)(A) - Married Individuals Filing Joint Returns and Surviving
    // Spouses.
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(24800), dec!(0.12)),
                bracket(dec!(100800), dec!(0.22)),
                bracket(dec!(211400), dec!(0.24)),
                bracket(dec!(403550), dec!(0.32)),
                bracket(dec!(512450), dec!(0.35)),
                bracket(dec!(768700), dec!(0.37)),
            ],
        },
    );
    // §4.01 TABLE 4 - Section 1(j)(2)(D) - Married Individuals Filing Separate Returns.
    // ★ Identical to Single through the 35% bracket and then DIVERGES: the 37% bracket starts at
    //   $384,350, exactly half the joint $768,700, per §1(j)(2)(D). Same shape as TY2024/TY2025.
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(12400), dec!(0.12)),
                bracket(dec!(50400), dec!(0.22)),
                bracket(dec!(105700), dec!(0.24)),
                bracket(dec!(201775), dec!(0.32)),
                bracket(dec!(256225), dec!(0.35)),
                bracket(dec!(384350), dec!(0.37)),
            ],
        },
    );
    // §4.01 TABLE 2 - Section 1(j)(2)(B) - Heads of Households.
    // ★★ TWO floors sit $25 BELOW Single/MFS this year — 32% at $201,750 (vs $201,775) and 35% at
    //    $256,200 (vs $256,225) — and they are bracketed by rows that are EQUAL across the two
    //    schedules ($105,700 below, $640,600 above), so a digit is easy to lose in either direction.
    //    TABLE 2's own "$39,207 plus 32%" and "$56,631 plus 35%" are what settle both.
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(17700), dec!(0.12)),
                bracket(dec!(67450), dec!(0.22)),
                bracket(dec!(105700), dec!(0.24)),
                bracket(dec!(201750), dec!(0.32)),
                bracket(dec!(256200), dec!(0.35)),
                bracket(dec!(640600), dec!(0.37)),
            ],
        },
    );
    // §4.03 Maximum Capital Gains Rate (§1(h), §1(j)(5)) — the "Maximum Zero Rate Amount" and
    // "Maximum 15% Rate Amount" columns, verbatim.
    let mut ltcg = BTreeMap::new();
    // "All Other Individuals".
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(49450),
            max_fifteen: dec!(545500),
        },
    );
    // "Married Individuals Filing Joint Returns and Surviving Spouse".
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(98900),
            max_fifteen: dec!(613700),
        },
    );
    // "Married Individuals Filing Separate Returns". ★ For TY2026 both MFS figures DO happen to be
    // exactly half the joint ones ($49,450 = 98,900/2, $306,850 = 613,700/2) — unlike TY2025, whose
    // MFS 15% ceiling was a round $300,000 and NOT half of $600,050. That coincidence is read off
    // the printed table, never derived from it; the arithmetic is a check, not a source.
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(49450),
            max_fifteen: dec!(306850),
        },
    );
    // "Heads of Household".
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(66200),
            max_fifteen: dec!(579600),
        },
    );
    TaxTable {
        year: 2026,
        // ★ Deliberately NOT the shipped table's string. `compare_tables` destructures `source: _`
        //   precisely so the two artifacts cite different provenance; equal strings would be the
        //   signature of a copy.
        source: "TEST-TY2026 (Rev. Proc. 2025-32 §4.01 Tables 1-4, §4.03, §4.42(1), §2.14; \
                 90 FR 49047)",
        ordinary,
        ltcg,
        // §4.42(1) — "the first $19,000 of gifts to any person (other than gifts of future interests
        // in property) are not included in the total amount of taxable gifts under § 2503".
        // ★ Unchanged from TY2025's $19,000; a flat year-over-year figure is a real reading here, not
        //   a carried-over one — §4.42(1) prints it for calendar year 2026 in its own words.
        gift_annual_exclusion: dec!(19000),
        // ★ NOT in Rev. Proc. 2025-32. SSA determination: "The OASDI contribution and benefit base is
        //   $184,500 for remuneration paid in 2026 and self-employment income earned in tax years
        //   beginning in 2026" — 90 FR 49050 (the summary at 90 FR 49047 prints the same figure).
        ss_wage_base: dec!(184500),
        // §2.14 — OBBBA §70106 amends §2010(c)(3), "increasing the basic exclusion amount to
        // $15,000,000 for calendar year 2026". Set by statute, not by an inflation adjustment, which
        // is why it appears in SECTION 2 (CHANGES) and has no §4 entry.
        gift_lifetime_exclusion: dec!(15_000_000),
    }
}

/// The real **TY2017** ordinary + §1(h) schedules, transcribed from **Rev. Proc. 2016-55**, 2016-45
/// I.R.B. 707 — §3.01 *Tax Rate Tables* TABLE 1 (§1(a)), TABLE 2 (§1(b)), TABLE 3 (§1(c)) and TABLE 4
/// (§1(d)). The gift figures are §3.37(1) (§2503 annual exclusion, $14,000) and §3.35 (§2010 basic
/// exclusion amount, $5,490,000). The Social Security wage base is **not in the revenue procedure at
/// all** — it is an SSA determination, taken here from 81 FR 74854 (Vol. 81 No. 208, Thursday 27
/// October 2016), committed at `legal/text/federal-register/SSA_COLA_Determinations_2017.txt`.
///
/// ★★★ **Transcribed from the procedure's TEXT LAYER, never from `tax_tables.rs`.** The shipped table
/// was not opened until this function was complete. Copying it in would make the equality in
/// `shipped_tables_are_the_validated_tables.rs` assert that a number equals itself — an echo, not a
/// witness — and would certify a typo forever. TY2017 shipped in the binary with **no** counterpart at
/// all until this landed.
///
/// ★★ **PRE-TCJA — this is the 10 / 15 / 25 / 28 / 33 / 35 / 39.6 schedule of §1(a)–(d), NOT §1(j).**
/// §1(j) was added by TCJA §11001 and applies only to taxable years beginning after 2017, so it does
/// not exist for TY2017. Seven rates either way, but different rates at different thresholds: a TY2017
/// transcription reading 10/12/22/24/32/35/37 is wrong no matter what artifact it agrees with. Note
/// also that the 33%→35% breakpoint is **$416,700 for Single, MFJ *and* HoH alike** (and $208,350 =
/// half of it for MFS) — pre-TCJA the top of the 33% bracket was shared, which post-TCJA it is not.
///
/// ★★ **Every threshold below is independently confirmed by the procedure's own cumulative-tax
/// column**, which is a second, redundant encoding of the same breakpoints. All 24 interior rows were
/// re-derived and all 24 agree, so a mis-keyed digit is caught by arithmetic rather than by a re-read —
/// the failure mode `CLAUDE.md` records for Form 6251 line 33. Worked examples, one per table:
///
/// - TABLE 1 (MFJ): `52,222.50 + 0.33 × (416,700 − 233,350) = 112,728` ✓ pins the 35% floor at
///   **$416,700**.
/// - TABLE 2 (HoH): `117,202.50 + 0.35 × (444,550 − 416,700) = 126,950` ✓ pins the 39.6% floor at
///   **$444,550**.
/// - TABLE 3 (Single): `120,910.25 + 0.35 × (418,400 − 416,700) = 121,505.25` ✓ — a $1,700-wide 35%
///   bracket, the narrowest row in the procedure and the easiest to lose.
/// - TABLE 4 (MFS): `26,111.25 + 0.33 × (208,350 − 116,675) = 56,364` ✓ pins the 35% floor at
///   **$208,350**.
///
/// ★★ **The §1(h) breakpoints are NOT printed anywhere in Rev. Proc. 2016-55** — there is no
/// "Maximum Capital Gains Rate" section in it, because pre-TCJA §1(h) defines the breakpoints *by
/// reference to the ordinary brackets* rather than by its own indexed dollar figures (§1(j)(5), which
/// gives them independent amounts, did not yet exist). Read against
/// `legal/primary-sources/statute-irc/26USC_s1.html`:
///
/// - §1(h)(1)(B)(i) — 0% applies up to *"the amount of taxable income which would … be taxed at a rate
///   **below 25 percent**"*, i.e. the **top of the 15% bracket** = the "not over" figure on each
///   table's second row.
/// - §1(h)(1)(C)(ii)(I) — 15% applies up to *"the amount of taxable income which would … be taxed at a
///   rate **below 39.6 percent**"*, i.e. the **top of the 35% bracket** = the "not over" figure on each
///   table's sixth row (equivalently, where the 39.6% bracket starts).
///
/// These are therefore *derived*, not transcribed — the one place in this function where that is
/// unavoidable — but the derivation is a statutory quotation applied to figures transcribed above, not
/// a closed form. They reproduce the 2017 *Qualified Dividends and Capital Gain Tax Worksheet*'s own
/// printed constants ($37,950 / $75,900 / $50,800 and $418,400 / $470,700 / $235,350 / $444,550), which
/// is the independent confirmation.
///
/// ★ Statuses were read INDEPENDENTLY, never computed from one another. For TY2017 several MFS figures
/// *do* happen to be exactly half the joint ones ($235,350 = ½ × $470,700; $208,350 = ½ × $416,700)
/// while others are not ($76,550 ≠ ½ × $153,100 = $76,550 — that one is; but $116,675 = ½ × $233,350 —
/// also is, and HoH's $50,800 is unrelated to anything). Coincidence in one year is not a rule; every
/// row below came off its own table.
///
/// ★ `Qss` is deliberately absent, matching [`ty2024_table`] and [`ty2025_table`]: §1(a) gives a
/// surviving spouse the joint schedule (TABLE 1 is titled "Married Individuals Filing Joint Returns
/// **and Surviving Spouses**"), and `TaxTable::key` normalises `Qss → Mfj` at lookup. That absence is
/// lawful only when BOTH sides omit it, which the ratchet checks rather than assumes.
pub fn ty2017_table() -> TaxTable {
    let mut ordinary = BTreeMap::new();
    // §3.01 TABLE 3 - Section 1(c) – Unmarried Individuals (other than Surviving Spouses and Heads of
    // Households).
    ordinary.insert(
        FilingStatus::Single,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(9325), dec!(0.15)),
                bracket(dec!(37950), dec!(0.25)),
                bracket(dec!(91900), dec!(0.28)),
                bracket(dec!(191650), dec!(0.33)),
                bracket(dec!(416700), dec!(0.35)),
                bracket(dec!(418400), dec!(0.396)),
            ],
        },
    );
    // §3.01 TABLE 1 - Section 1(a) - Married Individuals Filing Joint Returns and Surviving Spouses.
    ordinary.insert(
        FilingStatus::Mfj,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(18650), dec!(0.15)),
                bracket(dec!(75900), dec!(0.25)),
                bracket(dec!(153100), dec!(0.28)),
                bracket(dec!(233350), dec!(0.33)),
                bracket(dec!(416700), dec!(0.35)),
                bracket(dec!(470700), dec!(0.396)),
            ],
        },
    );
    // §3.01 TABLE 4 - Section 1(d) – Married Individuals Filing Separate Returns.
    // ★ Identical to Single only through the 15% bracket ($9,325 / $37,950) and then diverges at EVERY
    //   subsequent row: $76,550 vs $91,900, $116,675 vs $191,650, $208,350 vs $416,700, $235,350 vs
    //   $418,400. Read off TABLE 4, not halved from TABLE 1.
    ordinary.insert(
        FilingStatus::Mfs,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(9325), dec!(0.15)),
                bracket(dec!(37950), dec!(0.25)),
                bracket(dec!(76550), dec!(0.28)),
                bracket(dec!(116675), dec!(0.33)),
                bracket(dec!(208350), dec!(0.35)),
                bracket(dec!(235350), dec!(0.396)),
            ],
        },
    );
    // §3.01 TABLE 2 - Section 1(b) – Heads of Households.
    // ★ The 35% floor is $416,700 — the SAME figure as Single and MFJ, which is a pre-TCJA property and
    //   not a copy: TABLE 2's own "$117,202.50 plus 35%" is what settles it
    //   (49,816.50 + 0.33 × (416,700 − 212,500) = 117,202.50).
    ordinary.insert(
        FilingStatus::HoH,
        OrdinarySchedule {
            brackets: vec![
                bracket(dec!(0), dec!(0.10)),
                bracket(dec!(13350), dec!(0.15)),
                bracket(dec!(50800), dec!(0.25)),
                bracket(dec!(131200), dec!(0.28)),
                bracket(dec!(212500), dec!(0.33)),
                bracket(dec!(416700), dec!(0.35)),
                bracket(dec!(444550), dec!(0.396)),
            ],
        },
    );
    // §1(h)(1)(B)(i) and §1(h)(1)(C)(ii)(I), applied to the §3.01 tables above — see the derivation
    // note on this function. `max_zero` = top of the 15% bracket; `max_fifteen` = top of the 35%
    // bracket.
    let mut ltcg = BTreeMap::new();
    // TABLE 3 rows 2 and 6.
    ltcg.insert(
        FilingStatus::Single,
        LtcgBreakpoints {
            max_zero: dec!(37950),
            max_fifteen: dec!(418400),
        },
    );
    // TABLE 1 rows 2 and 6.
    ltcg.insert(
        FilingStatus::Mfj,
        LtcgBreakpoints {
            max_zero: dec!(75900),
            max_fifteen: dec!(470700),
        },
    );
    // TABLE 4 rows 2 and 6.
    ltcg.insert(
        FilingStatus::Mfs,
        LtcgBreakpoints {
            max_zero: dec!(37950),
            max_fifteen: dec!(235350),
        },
    );
    // TABLE 2 rows 2 and 6.
    ltcg.insert(
        FilingStatus::HoH,
        LtcgBreakpoints {
            max_zero: dec!(50800),
            max_fifteen: dec!(444550),
        },
    );
    TaxTable {
        year: 2017,
        // ★ Deliberately NOT the shipped table's `source` string. Identical provenance strings on the
        //   two artifacts would be the signature of a copy, which is the one thing this pair exists to
        //   rule out. `compare_tables` excludes `source` from the equality for exactly that reason.
        source: "TEST-TY2017 (Rev. Proc. 2016-55 §3.01 TABLES 1-4 = §1(a)-(d); §1(h)(1)(B)-(C) \
                 breakpoints derived from those tables; §3.37(1); §3.35; SSA 81 FR 74854)",
        ordinary,
        ltcg,
        // §3.37(1) — "For calendar year 2017, the first $14,000 of gifts to any person … are not
        // included in the total amount of taxable gifts under § 2503 made during that year."
        gift_annual_exclusion: dec!(14000),
        // 81 FR 74854 (27 Oct 2016), "OASDI Contribution and Benefit Base": "The OASDI contribution and
        // benefit base is $127,200 for remuneration paid in 2017 and self-employment income earned in
        // taxable years beginning in 2017."
        ss_wage_base: dec!(127200),
        // §3.35 — "For an estate of any decedent dying in calendar year 2017, the basic exclusion
        // amount is $5,490,000 … under § 2010."
        gift_lifetime_exclusion: dec!(5_490_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goldens_carry_the_deeper_and_provenance_leaves() {
        // Post-T11 the baked corpus CARRIES the deeper-line + provenance leaves on every household — the
        // deeper differential and the per-oracle provenance classes depend on them; a re-bake that dropped
        // them would silently make those checks inert. (`qbi_cap_l12` + the C1 SE legs are per-form, so
        // they bake only on the relevant households — checked on an SE household below — not on the
        // W-2-only floor case.)
        let hs = golden_households(); // parses GOLDEN_RETURNS_JSON
        let floor = &hs[0]; // single_w2_only_standard
        assert!(floor.expected_ots.net_ltcg_qd_exclusive.is_some());
        assert!(floor.expected_ots.qual_div_l3a.is_some());
        assert!(floor.expected_ots.deduction_taken.is_some());
        assert!(floor.expected_taxcalc.total_tax.is_some());
        assert!(floor.expected_taxcalc.net_ltcg_qd_exclusive.is_some());

        let se = hs
            .iter()
            .find(|h| h.name == "single_crypto_business_se")
            .expect("the SE anchor is in the matrix");
        assert!(se.expected_ots.qbi_cap_l12.is_some());
        assert!(se.expected_ots.se_l10_oasdi.is_some());
        assert!(se.expected_ots.se_l11_medicare.is_some());

        // …and the `#[serde(default)]` optionals STILL default to None when a record OMITS them (a
        // hand-written or older JSON must keep parsing) — the schema tolerance the original test guarded.
        let minimal: ExpectedOts = serde_json::from_str(
            r#"{"adjusted_gross_income":0.0,"taxable_income":0.0,"qbi_deduction":0.0,
                "income_tax_before_credits":0.0,"se_tax":0.0,"niit":0.0,
                "additional_medicare_tax":0.0,"total_tax":0.0}"#,
        )
        .expect("a record without the optional deeper leaves still parses");
        assert!(minimal.net_ltcg_qd_exclusive.is_none());
        assert!(minimal.qbi_cap_l12.is_none());
        assert!(minimal.se_l10_oasdi.is_none());
    }
}
