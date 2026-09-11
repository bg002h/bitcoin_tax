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

/// ★★★ **T9 — one ORDINARY Form 1098 row carrying `interest` in box 1.**
///
/// The fixture form of `schedule_a.mortgage_interest_1098`, which T9 removed. "Ordinary" is stated,
/// not defaulted: box 4 is zero (any amount refuses `MortgageInterestRefundNotComputed`) and the
/// shared-interest gate is answered `no` (a blank refuses `SharedMortgageInterestUnanswered`,
/// because line 8a sums box 1 in full). Box 2 is left at zero so no fixture picks up a
/// §163(h)(3)(B) ceiling warning it was not written to exercise.
#[must_use]
pub fn form_1098_with_interest(interest: Usd) -> crate::tax::return_inputs::Form1098 {
    crate::tax::return_inputs::Form1098 {
        lender: "Fixture Mortgage".to_string(),
        box1_interest: interest,
        other_borrower_paid_interest: Some(false),
        ..Default::default()
    }
}
use crate::identity::{EventId, LotId, WalletId};
use crate::state::{Disposal, DisposalLeg, IncomeRecord, LedgerState, Term};
use crate::tax::questions::FORM_QUESTIONS;
use crate::tax::return_inputs::{
    CharitableClass, CharitableGift, Dependent, Form1099B, Form1099Div, Form1099G, Form1099Int,
    HouseholdHeader, Owner, Payments, Person, ReturnInputs, ScheduleAInputs, ScheduleCInputs, W2,
};
use crate::tax::tables::{
    AmtParams, FullReturnParams, HsaParams, LtcgBreakpoints, OrdinaryBracket, OrdinarySchedule,
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
    // ★★★ T7 / R6 — the DEPENDENT GATES first, because `FilerTinIssuedByDueDate` is live iff a
    //     dependent row exists and the loop below must see a coherent row when it answers it.
    answer_all_dependent_gates(ri);
    for q in FORM_QUESTIONS {
        if (q.live)(ri) && (q.get)(ri).is_none() {
            (q.set)(ri, q.neutral); // ★ declared per question — see FormQuestion::neutral
        }
    }
    // ★★★ **AND AGAIN, because the flowchart WAITS on two return-level declarations.** Step 2
    //     question 4 (*"Could you be claimed as a dependent…"*) and Step 5 question 1 (the filer's
    //     own TIN) are `FormQuestion`s, and a row that reaches one of them unanswered stops at
    //     `DependentVerdict::WaitingOnQuestion` — which is not `Unanswered`, so the sweep above
    //     BREAKS and every later gate on that row is left blank. The loop has now answered both, so
    //     one more sweep finishes the rows that were waiting.
    //
    // ★ Found by T8: a TY2025 fixture with two dependents that did not pre-answer
    //   `can_be_claimed_as_dependent_taxpayer` got `CreditColumn::Neither` on both children, with
    //   this helper reporting nothing. Idempotent on a fixture that was already complete.
    answer_all_dependent_gates(ri);
    answer_all_class_a_skippables(ri);
}

/// ★★★ **R7 / T8 — answer every LIVE class-(A) entry of `SKIPPABLE_QUESTIONS`.**
///
/// That registry is class (B) by rule, and the entries that declare otherwise
/// ([`crate::tax::questions::SkippableQuestion::unanswered`]) BLOCK — so a fixture that leaves one
/// blank refuses, exactly as it would for a `FormQuestion`. Which entries those are is DERIVED from
/// the declaration, never listed here.
///
/// ★ **The chosen option is the first that leaves the screen clean**, not a name typed here: two of
///   the HoH marital basis's four states refuse by design, and a fixture helper that picked one of
///   them would make every HoH fixture in the workspace refuse for a reason the fixture never meant.
///   The fallback is the first option, for a fixture already refusing for some unrelated reason.
pub fn answer_all_class_a_skippables(ri: &mut ReturnInputs) {
    use crate::tax::questions::{SkippableKind, SKIPPABLE_QUESTIONS};
    for sk in SKIPPABLE_QUESTIONS {
        if sk.unanswered.is_none() || !(sk.live)(ri) {
            continue;
        }
        match sk.kind {
            SkippableKind::YesNo => {
                if (sk.get_bool)(ri).is_none() {
                    (sk.set_bool)(ri, true);
                }
            }
            SkippableKind::Date => {}
            SkippableKind::Choice(options) => {
                if (sk.get_choice)(ri).is_some() {
                    continue;
                }
                let mut chosen = options.first().copied();
                for o in options {
                    (sk.set_choice)(ri, o);
                    if crate::tax::return_refuse::screen_param_free(ri).is_none() {
                        chosen = None; // this one stands
                        break;
                    }
                }
                if let Some(first) = chosen {
                    (sk.set_choice)(ri, first);
                }
            }
        }
    }
}

/// ★★★ **T7 / R6 — answer every LIVE dependent gate a fixture left blank, at its declared
/// `claim_path` polarity**, so a fixture that carries a dependent row is not tripped by
/// `screen_dependent_gates`.
///
/// **DERIVED, in two senses.** Which gates to answer comes from
/// [`walk_dependent`] — the same liveness the screen uses, never a list here — and the ANSWER comes
/// from the registry's own [`DependentGateQuestion::claim_path`] column, which
/// `every_gate_at_its_claim_path_answer_reaches_the_ctc_edge` proves lands on the CTC edge. So a new
/// gate is covered with zero fixture edits, which is the property `answer_all_live_declarations`
/// exists for.
///
/// ★★ **The date of birth is DERIVED FROM THE FIXTURE'S OWN TAX YEAR** (ten years before it), not
///    typed: R6 makes it required, and `claim_path` leaves `full_time_student` and
///    `permanently_and_totally_disabled` at `false`, so the age test needs a child under 19. A
///    fixture that already carries a date keeps it.
///
/// ★ Legitimate for a FIXTURE helper and not for a product surface, exactly as
///   [`reconcile_document_census`] records: it derives a fixture's intent from the fixture's own
///   data (a dependent row means to claim that person), and it never answers for a human.
pub fn answer_all_dependent_gates(ri: &mut ReturnInputs) {
    use crate::tax::dependent_gates::{entry, walk_dependent, DependentVerdict, DEPENDENT_GATES};
    for row in 0..ri.header.dependents.len() {
        if ri.header.dependents[row].date_of_birth.is_none() {
            // ★★★ **FAILS LOUDLY on the `tax_year = 0` sentinel**, which §G-15 defines as *"not
            //     stated"* and `return_inputs::set` stamps from the STORE KEY. A fixture in that
            //     state would get a date ten years before year zero, and the age test — computed
            //     against the stamped year — would then place a 2034-year-old on the
            //     qualifying-RELATIVE branch. Silent, and it moves the verdict, so it is an
            //     assertion rather than a guess.
            assert!(
                ri.tax_year > 0,
                "answer_all_dependent_gates derives a dependent's date of birth from the fixture's \
                 own tax year, and this fixture has not stated one (tax_year = 0). Set `tax_year` \
                 before calling, or give the row a date of birth."
            );
            let dob = time::Date::from_calendar_date(ri.tax_year - 10, time::Month::June, 1)
                .expect("June 1 exists in every year");
            ri.header.dependents[row].date_of_birth = Some(dob);
        }
        // Answering one gate opens the next block, so sweep to a fixpoint. The bound is the gate
        // count: each pass answers at least one, or breaks.
        for _ in 0..=DEPENDENT_GATES.len() {
            let DependentVerdict::Unanswered(gate) = walk_dependent(ri, row).verdict else {
                break;
            };
            let q = entry(gate);
            let v = q
                .claim_path
                .expect("only the Date gate has no claim_path, and it is answered above");
            (q.set)(&mut ri.header.dependents[row], v);
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
        // ★★★ **T9 — a row whose LIVENESS the shape decides.** `form_1098` is live iff
        //     `schedule_a.is_some()`, so a fixture builder that answers every live declaration
        //     BEFORE its shape adds a Schedule A cannot reach it: the row was not live when the
        //     answers were written and is live, and blank, by the time the screen runs. Answering
        //     it here — with the honest *"none arrived"*, on a row that carries no transcribed
        //     document — is the same one-direction reconciliation the branch above performs.
        //
        // ★ Only rows with a `Vec` to count are touched, so the §2.2 unsupported families (whose
        //   `Some(true)` refuses by design) keep whatever the fixture stated.
        if crate::tax::document_census::declared_rows(ri, *row) == Some(0)
            && crate::tax::document_census::row_is_live(ri, *row)
            && ri.documents.get(*row).is_none()
        {
            ri.documents.set(*row, Some(false));
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
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::macros::date;

/// ★★★ **The §24(h)(2) per-child ceiling `advisories::ctc_provably_zero` multiplies by**, exposed so
/// a DOWNSTREAM crate can pin it against every year the bundle actually registers.
///
/// ★★ This accessor exists because of T8 seam review **I-2**. The pin that claimed to hold this
/// constant *"for every year whose package is bundled"* lived in core and read
/// [`ty2024_params`] — a core-local fixture literal hardcoded to 2024. `BundledFullReturnTables`
/// lives in `btctax-adapters`, so core cannot see a bundled package at all and the pin could not red
/// on the defect it documented: bundling TY2026 (whose figure is $2,200) left it green while
/// `ctc_odc_line19` swore a `0` on line 19 for a household that still had credit. The assertion had
/// to move to the crate that can see the bundle; this is the one thing it needs from core.
/// The pin is `btctax-adapters`'
/// `shipped_tables_are_the_validated_tables::every_bundled_years_ctc_per_child_is_the_named_ceiling`.
#[must_use]
pub fn ctc_per_child_ss24h2() -> Usd {
    crate::tax::advisories::CTC_PER_CHILD_SS24H2
}

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
        // §152(d)(1)(B), Rev. Proc. 2023-34 §3.24; i1040gi--2024.txt:1700 prints it.
        // ★★★ §163(h)(3)(B) — the four figures the Schedule A instructions print under "Limits on
        //     home mortgage interest" (`design/forms/extract/i1040sca--2025.txt:1027-1046`; Pub.
        //     936, Part II). Statute, not an indexed amount: Pub. L. 119-21 (OBBBA) §70108(a) made
        //     the $750,000 limit permanent. They drive a WARNING over Σ Form 1098 box 2 and write
        //     no line.
        acquisition_debt_ceiling: crate::tax::tables::AcquisitionDebtCeiling {
            after_dec_15_2017: dec!(750000),
            after_dec_15_2017_mfs: dec!(375000),
            on_or_before_dec_15_2017: dec!(1000000),
            on_or_before_dec_15_2017_mfs: dec!(500000),
        },
        qualifying_relative_gross_income_limit: dec!(5050),
        // §24(h)(2) / §24(h)(4) — the TCJA figures, in force through TY2024 (Pub. L. 119-21
        // §70104(a)(2)+(f) raises the child credit to $2,200 from TY2025). Nothing computes from
        // them; the R12 panel SIZES the line-19 forgo with them.
        child_tax_credit_per_child: dec!(2000),
        credit_for_other_dependents_per_person: dec!(500),
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
        // §223(b)(2) HSA contribution limitation (Rev. Proc. 2023-23 §2.01(1)) — $4,150 self-only,
        // $8,300 family. §223(b)(3)(B)'s additional contribution at 55+ is a flat statutory
        // $1,000, NOT indexed.
        hsa: HsaParams {
            self_only_limit: dec!(4150),
            family_limit: dec!(8300),
            additional_contribution_55: dec!(1000),
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
        // ★★★ R8 / T9 — Schedule A's Line 8a Caution: this household holds no mortgage credit
        //     certificate, so line 8a keeps the whole Form 1098 figure.
        claiming_mortgage_interest_credit: Some(false),
        // ★★★ R8 / T9 — the sale-of-a-main-home question, always live and never defaulted. This
        //     household sold no home; the three Schedule D tests are then not live at all.
        home_sale: crate::tax::return_inputs::HomeSale {
            sold_main_home: Some(false),
            ..Default::default()
        },
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
                // ★ T7 / R6 — the CTC edge: Step 1 Yes, Step 2 claimable, Step 3 all Yes and under
                //   17, so row (7)'s "Child tax credit" box. Answered here rather than left to
                //   `answer_all_dependent_gates`, because this fixture is the MAXIMAL one and a
                //   `None` leaf drops out of every derived axis built on it.
                lived_with_you_over_half_year: Some(true),
                lived_with_you_in_us: Some(true),
                full_time_student: Some(false),
                permanently_and_totally_disabled: Some(false),
                qc_relationship: Some(true),
                younger_than_you_or_spouse: Some(true),
                provided_over_half_own_support: Some(false),
                filing_joint_return: Some(false),
                joint_return_only_to_claim_refund: None,
                qualifying_child_of_another_person: Some(false),
                citizen_national_resident_or_canada_mexico: Some(true),
                married: Some(false),
                tin_issued_by_due_date: Some(true),
                citizen_national_or_resident_alien: Some(true),
                ssns_valid_for_employment_issued_by_due_date: Some(true),
                qr_relationship_or_member_of_household: None,
                qualifying_child_of_any_taxpayer: None,
                gross_income_under_limit: None,
                you_provided_over_half_support: None,
                divorced_separated_multiple_support_or_kidnapped_rule_applies: None,
            }],
            // ★ T7 / R6 — Step 5 question 1, live because this fixture carries a dependent row.
            filer_tin_issued_by_due_date: Some(true),
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
        // ★ T9 — the mortgage interest that used to be the `mortgage_interest_1098` scalar, now the
        //   document it was always copied off. Box 2 is under the §163(h)(3)(B) ceiling, box 4 is
        //   zero, and the shared-interest gate is answered.
        form_1098: vec![crate::tax::return_inputs::Form1098 {
            lender: "Home Savings".into(),
            box1_interest: dec!(22000),
            box2_outstanding_principal: dec!(400000),
            box3_origination_date: Some(
                time::Date::from_calendar_date(2019, time::Month::June, 1).expect("a valid date"),
            ),
            other_borrower_paid_interest: Some(false),
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
        // ★ B3 C-1 — the vector IS a TY2024 return (the §55 exemption, the 26/28% split and both
        //   oracles' witness are all TY2024's), and `screen_inputs` now refuses a return that does
        //   not say so. Nothing computed moves: every figure here is derived from the package and the
        //   `year` argument, never from `ri.tax_year` (`return_1040.rs` records why).
        tax_year: 2024,
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

/// ★★★ **THE STRUCTURAL FIXTURE — every money leaf of `ReturnInputs` is realized AND non-zero.**
///
/// Built for the seam review's I-1 disposition: the two-chain comparison
/// (`packet::tests::the_absolute_total_tax_equals_the_printed_1040_line_24` and its AGI twin) ran
/// over a HAND-LISTED pair of households, so T16 could add a term to the printed 1040 line 24 and
/// not to `AbsoluteReturn::total_tax`, and the one test written to catch exactly that passed. **A
/// fixture list is the wrong instrument for "did a NEW leaf reach only one chain", because the list
/// is written by the person who added the leaf.**
///
/// So this fixture is DERIVED, in two steps, and neither step names a field:
///
/// 1. [`crate::tax::scrub_axis::maximal_sentinel`] is the repo's maximal `ReturnInputs` — every
///    `Option` `Some`, every `Vec` two rows, every nested struct present, written as an exhaustive
///    struct literal with **no `..`**, so a field added anywhere fails to compile *there*.
/// 2. Every leaf [`leaf_walk::money_leaves`] classifies as money — by TYPE, by round-tripping a
///    decimal probe through `Decimal`'s own deserializer, never by a list — is then overwritten with
///    a distinct non-zero whole-dollar amount.
///
/// A money leaf added to `ReturnInputs` tomorrow is therefore populated here with nobody
/// remembering to do it, and if it reaches a printed line without reaching the absolute chain, the
/// comparison reds. That is the difference between an instrument that catches this class and one
/// that catches the instance somebody thought of.
///
/// ★ **Distinct amounts, not one constant.** Two leaves carrying the same figure can cancel — a
///   missing term on one side matched by a spurious one on the other — and the whole point is that
///   the two chains are assembled by different code. The step of 7 makes every pair of leaves differ
///   and every sum distinct from every other.
///
/// ★★ **Whole dollars.** The absolute chain sums exact cents and rounds once; the printed chain
///   rounds every line. Whole-dollar leaves keep the two comparable, so a red is a MISSING TERM
///   rather than SPEC §3.1's per-line rounding — which is the strictness
///   `the_absolute_total_tax_equals_the_printed_1040_line_24` deliberately keeps.
///
/// ★ `hsa_activity` is affirmed (the sentinel answers it `Some(false)`, which closes Form 8889
///   entirely), and the declarations are then answered at their registry neutrals. Without that the
///   HSA money leaves are realized and reach nothing, and the row would be vacuous on the very legs
///   the review found missing.
///
/// This is a FIXTURE, not a taxpayer: the household it describes is nobody, and the figures are
/// sentinels. Nothing here is validated against an oracle and nothing should be.
#[must_use]
pub fn every_money_leaf_household() -> (ReturnInputs, LedgerState) {
    use crate::tax::provenance::leaf_walk::{money_leaves, set_at};

    let base = crate::tax::scrub_axis::maximal_sentinel();
    let mut doc = serde_json::to_value(&base).expect("ReturnInputs serializes");
    for (i, path) in money_leaves(&base).into_iter().enumerate() {
        // Distinct, whole-dollar, and large enough that the household owes tax — a fixture whose
        // taxable income is $0 compares $0 to $0 on 1040 line 24 and is silent on every term.
        let amount = 1_000 + 137 * i;
        assert!(
            set_at(
                &mut doc,
                &path,
                serde_json::Value::String(amount.to_string())
            ),
            "money leaf {path} is walkable but not settable — the two walks have diverged"
        );
    }
    let mut ri: ReturnInputs =
        serde_json::from_value(doc).expect("every money leaf takes a decimal string");
    // ★ §223's trigger, affirmed: Form 8889 files exactly on this declaration, so the HSA leaves
    //   above reach Schedule 1 lines 8f/13 and Schedule 2 lines 17c/17d only with it.
    ri.sch1.hsa_activity = Some(true);
    // ★★ ONE ORDERING CONSTRAINT, stated rather than discovered. Form 8889 line 17b is *"20%
    //    (0.20) of the distributions included on line 16 that are subject to the additional 20%
    //    tax"* — line 16 LESS the part meeting an exception — so an exception amount at or above
    //    line 16 zeroes the line, and the fixture would then be silent on the §223(f)(4) leg the
    //    seam review found missing from `schedule_2_other_taxes`. The derived pass above cannot
    //    know that two of its leaves are on opposite sides of one subtraction; the anti-vacuity
    //    guard in `packet::tests::two_chain_households` is what forces it to be said here.
    ri.hsa.line16_amount_meeting_an_exception = Usd::ONE;
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

/// The row-(7) credit column an oracle household's dependent lands in — the same three states
/// [`crate::tax::dependent_gates::CreditColumn`] has, restated here because a `GoldenInputs` is the
/// ORACLE ROW and must be readable (and writable) without a `ReturnInputs` in hand.
///
/// It is not a second transcription of the form: it carries the form's two printed labels and the
/// forgo, and [`GoldenDependent::credit`] is produced from `CreditColumn` by an exhaustive match, so
/// a new arm on either side reds the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoldenCreditColumn {
    /// *"Child tax credit"* — Tax-Calculator's `n24`.
    ChildTaxCredit,
    /// *"Credit for other dependents"* — Tax-Calculator counts these as `XTOT − n24 − num`.
    CreditForOtherDependents,
    /// Neither box. The person is still a dependent (so still an exemption in `XTOT`), and the
    /// credit is FORGONE, not refused (`i1040gi--2025.txt:1650-1653`).
    ///
    /// ★ **Both oracles disagree with btctax here by construction**, and that is a fact about the
    ///   ENGINES rather than a bug: Tax-Calculator has no *"neither box"* state — its Schedule 8812
    ///   line 7 is `ODC_c × max(0, XTOT − childnum − num)`, so every non-CTC member of the unit gets
    ///   the $500. The divergence is inside the line-19 excuse either way (btctax's line 19 is blank
    ///   while Schedule 8812 is unbuilt), so it is not separately excused.
    #[default]
    Neither,
}

/// ★★★ **One dependent of an oracle household** (R13). Ages, not dates: neither engine takes a date
/// of birth — Tax-Calculator counts `n24` / `nu18` / `n1820` / `n21` and OpenTaxSolver takes a
/// dependent COUNT — so the oracle row carries the quantity the engines actually read, and
/// [`build_golden_return`] materialises a date of birth that reproduces it.
///
/// ★ **The counts are DERIVED from these rows, never stored beside them** ([`GoldenInputs::n24`],
///   [`GoldenInputs::nu18`], …). A stored count and a stored row set are two encodings of one fact,
///   and this repo's standing failure is a hand-written list beside derived data going stale — twice
///   in the oracle harness alone (`CLAUDE.md`, *"An excuse list keyed by VECTOR NAME is a
///   liability"*). Deriving them also removes the only way a corpus cell could state a set of counts
///   no set of rows can realise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GoldenDependent {
    /// The age the person is **considered** to be at the end of the tax year — the IRS's own
    /// January-1 convention ([`crate::tax::return_1040::considered_age_at_year_end`],
    /// `i1040gi--2025.txt:3945-3951`), which is the boundary every age test on the return uses.
    pub age: u32,
    /// Row (7) of the Dependents grid, as the flowchart concluded it.
    #[serde(default)]
    pub credit: GoldenCreditColumn,
    /// §32(c)(3)(A) — is this person an **EIC qualifying child**? A §152(c) qualifying child (Step 1
    /// of *Who Qualifies as Your Dependent* concluded *"Yes. Go to Step 2"*) whose principal abode
    /// was in the **United States** for more than half the year.
    ///
    /// ★ btctax computes NO earned income credit (there is no Schedule EIC), so this exists only to
    ///   make Tax-Calculator's `EIC` — and therefore the line-27 excuse — a real number rather than
    ///   a structural zero.
    #[serde(default)]
    pub eic_qualifying_child: bool,
}

/// The age [`build_golden_return`] gives a golden household's taxpayer when the row does not state
/// one — **the FR-29 adult sentinel, expressed as data instead of as a hidden date**.
///
/// It was `date!(1980 - 05 - 05)` written inline in `build_golden_return`, with its own comment
/// naming the hazard: *"if the corpus ever gains a genuinely-under-24 household, this line would
/// exempt it from §1(g) by fiat. `GoldenInputs` carries no age today, so the day it does, this line
/// must read it instead of assuming."* T11 is that day, so it reads [`GoldenInputs::age_head`] and
/// this is only the default for the 107 committed cells that state nothing.
///
/// ★ 44 reproduces `1980-05-05` exactly for the corpus's tax year (2024) — the same date the fixture
///   used before — so no committed golden's figures move. The band the old comment required
///   (`[year - 63, year - 24]`, i.e. an age in `[24, 63]`) still holds: 44 can neither leave the
///   filer under 24 (§1(g)'s question comes back) nor reach §63(f)'s age-65 addition.
pub const GOLDEN_ADULT_AGE: u32 = 44;

fn golden_adult_age() -> Option<u32> {
    Some(GOLDEN_ADULT_AGE)
}

/// The tax year every golden household is dated in — `build_golden_return`'s own `date!(2024 - ..)`
/// legs and `btctax-oracle-harness`'s `YEAR`, named once so the projection can compute an age
/// against the same year the fixture was built in.
pub const GOLDEN_TAX_YEAR: i32 = 2024;

/// ★★★ **THE DEDUCTION ACTUALLY CLAIMED ON 1040 LINE 12 — a NON-`Usd` fact that ROUTES money**
/// (T11 fold, C-2).
///
/// `scripts/oracle/ots_direct.py` gates its **entire** Schedule A block — `A5a`/`A5b`/`A8a`/`A11`/
/// `A16`/`A18` — on `standard_or_itemized == "Itemized"`. `scripts/oracle/corpus.py:164` has written
/// that key since the SALT axis landed, with the note *"read by the Python oracles (not a
/// `GoldenInputs` field)"* — and it was not one, so **no projected row could ever carry it**. An
/// itemizing filer was handed to OpenTaxSolver with no Schedule A at all: 1040 line 12 DIVERGED on a
/// correct return, the run exited 1 telling the filer to adjudicate against the form, and Schedule A
/// line 5e silently lost its OTS witness while the census reported *"only one engine models this
/// line"*.
///
/// ★★ **It is written from btctax's OWN line-12 decision** ([`crate::tax::return_1040::AbsoluteReturn::deduction_is_itemized`]),
///    never re-derived — which is why [`project_to_golden`] takes the assembled return. And
///    [`build_golden_return`] deliberately IGNORES it: btctax decides §63(e)/(c)(6) for itself from
///    the amounts, exactly as the corpus intends. That is what makes the inverse KAT a real check —
///    a corpus cell claiming `Itemized` whose amounts lose to the standard deduction describes a
///    household btctax and the drivers disagree about, and the round trip reds.
///
/// ★ **The §G-9 residual, stated rather than hidden.** OTS's `A18` is *"Elect to itemize, even when
///   less than standard deduction"*, so handing it this token makes OTS's line-12 BRANCH an input
///   rather than an independent opinion; only the AMOUNT stays independently computed. Tax-Calculator
///   still chooses the branch for itself, so line 12 keeps a genuine second witness on the choice —
///   but a divergence on the branch alone is a one-witness finding. Filed as FR-94.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GoldenDeduction {
    /// §63(c) — the standard deduction was claimed on 1040 line 12.
    #[default]
    Standard,
    /// The Schedule A total was claimed on 1040 line 12 (larger, or elected under §63(e)).
    Itemized,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// ★★★ **1040 line 12 — WHICH deduction was claimed** (T11 fold, C-2). See [`GoldenDeduction`]
    /// for the whole mechanism: it is the one key `ots_direct.py` gates its entire Schedule A block
    /// on, `corpus.py` has written it since the SALT axis landed, and until this field existed no
    /// projected row could express it.
    ///
    /// ★ `#[serde(default)]` ⇒ `Standard`, which is what the 80 corpus cells with no Schedule A
    ///   already mean; the 27 that write `"Itemized"` now deserialize into it rather than being
    ///   dropped on the floor.
    #[serde(default)]
    pub standard_or_itemized: GoldenDeduction,
    /// ★★★ **Schedule 1 line 13 — the §223 HSA DEDUCTION** (T16 / FR-76).
    ///
    /// The figure the filer CONTRIBUTED to their own HSA, which Form 8889 line 13 deducts after its
    /// §223(b) limit. Both engines take it directly: OTS's `S1_13` (*"Health savings account
    /// deduction. Attach Form 8889"*) and Tax-Calculator's `e03290` (*"Health savings account
    /// deduction from Form 8889"*).
    ///
    /// ★★ **Keep it at or under the year's §223(b)(2) limit.** Neither oracle applies the limit —
    /// each takes the deduction as given — so a household over it would have btctax REFUSE (excess
    /// contributions need Form 5329) while both engines happily deducted the excess. That is the
    /// V2b shape recorded on `charitable_cash` above: a corpus cell that crosses a ceiling one
    /// engine does not model loses that engine. The constraint lives in the corpus cell's own `why`.
    #[serde(default)]
    pub hsa_deduction: f64,
    /// ★★★ **Schedule 1 line 7 — unemployment compensation** (T11). Tax-Calculator's `e02300`,
    /// OpenTaxSolver's `S1_7`; btctax's Σ Form 1099-G box 1.
    ///
    /// R13 names `e02300 = Σ g_1099.box1` in the projection list, and there was no field to put it
    /// in: without one, every 1099-G box 1 on a real return would have had to be laundered into
    /// [`ORACLE_INVISIBLE`] — *"blank because nothing populated it"*, the one blank `CLAUDE.md`
    /// calls a defect. Both engines model the line, so it is carried rather than excused.
    #[serde(default)]
    pub unemployment: f64,
    // ── The DEPENDENTS BLOCK (R13) ───────────────────────────────────────────────────────────────
    //
    // ★★★ Why it exists at all, in one sentence: without it `project_to_golden` could not carry a
    //     dependent, Tax-Calculator's `n24` would stay 0, `oracle_line19` would be 0, btctax's line
    //     19 is blank-as-0, and *"the CTC excuse is `oracle_line19 − 0`"* would be `0 − 0` on EVERY
    //     return — including one with three qualifying children. The same blindness covers the EIC
    //     (line 27) and the §63(f) aged and blind additions.
    /// The household's dependents, in grid order. See [`GoldenDependent`]; the counts both engines
    /// read ([`Self::n24`], [`Self::xtot`], …) are DERIVED from this vector.
    #[serde(default)]
    pub dependents: Vec<GoldenDependent>,
    /// Tax-Calculator's `age_head` — the taxpayer's considered age at the end of the tax year;
    /// OpenTaxSolver's `You_65+Over?`.
    ///
    /// `None` = the filer DECLINED the date of birth ([`crate::tax::questions::SkippableId::DobTaxpayer`]
    /// is class (B): silence forgoes the §63(f) addition, it never grants it), and neither engine is
    /// told an age. Absent from a corpus cell it defaults to [`GOLDEN_ADULT_AGE`], which is what
    /// `build_golden_return` assumed before this field existed.
    #[serde(default = "golden_adult_age")]
    pub age_head: Option<u32>,
    /// Tax-Calculator's `age_spouse`; OpenTaxSolver's `Spouse_65+Over?`. `None` on a return with no
    /// spouse **and** on a joint return whose spouse declined the date of birth — the two are
    /// indistinguishable to both engines, which ask only *"was the spouse born before January 2, …"*.
    ///
    /// ★ Its default is `None`, not [`GOLDEN_ADULT_AGE`]: `build_golden_return` has never given the
    ///   golden spouse a date of birth, and a default that invented one would make the projection
    ///   inverse assert a fact the fixture does not hold.
    #[serde(default)]
    pub age_spouse: Option<u32>,
    /// §63(f)(2) blindness, taxpayer — Tax-Calculator's `blind_head`, OpenTaxSolver's `You_Blind?`.
    ///
    /// A `bool`, not a tri-state, and that is not a D-8 regression: this is the ORACLE ROW, and
    /// neither engine has a *"never asked"* state. [`crate::tax::return_inputs::Person::blind`] keeps
    /// the tri-state; both `None` (never asked) and `Some(false)` project to `false`, because a
    /// forgone benefit and a declined one are the same instruction to an engine.
    #[serde(default)]
    pub blind_head: bool,
    /// §63(f)(2) blindness, spouse — Tax-Calculator's `blind_spouse`, OpenTaxSolver's
    /// `Spouse_Blind?`. See [`Self::blind_head`].
    #[serde(default)]
    pub blind_spouse: bool,
}

impl Default for GoldenInputs {
    /// All-zero, no dependents, and the taxpayer at [`GOLDEN_ADULT_AGE`] — the same household
    /// `serde` produces from `{"filing_status": "Single"}`, so a hand-built `GoldenInputs` and a
    /// parsed one cannot disagree about the defaults.
    fn default() -> Self {
        serde_json::from_str(r#"{"filing_status":"Single"}"#)
            .expect("GoldenInputs's own serde defaults parse")
    }
}

impl GoldenInputs {
    /// **Tax-Calculator's `XTOT`** — *"Total number of exemptions for filing unit"*: the filer, the
    /// spouse on a joint return, and every dependent claimed.
    ///
    /// ★ Derived from the FILING STATUS and the row set, never from the age bands. Schedule 8812's
    ///   other-dependent leg is `ODC_c × max(0, XTOT − childnum − num)`
    ///   (`taxcalc/calcfunctions.py:3362`), so `XTOT` is the variable the $500 credit actually turns
    ///   on — and a unit member whose age is unknown must still be counted.
    #[must_use]
    pub fn xtot(&self) -> u32 {
        1 + u32::from(self.filing_status == "Married/Joint") + self.dependents.len() as u32
    }
    /// **Tax-Calculator's `n24`** — *"Number of children who are Child-Tax-Credit eligible"*: the
    /// rows whose row (7) is the *"Child tax credit"* box.
    #[must_use]
    pub fn n24(&self) -> u32 {
        self.dependents
            .iter()
            .filter(|d| d.credit == GoldenCreditColumn::ChildTaxCredit)
            .count() as u32
    }
    /// **Tax-Calculator's `EIC`** — the number of §32(c)(3) qualifying children, which taxcalc
    /// documents as *"(range: 0 to 3)"* and §32(b) caps at three for the credit percentage, so the
    /// count is clamped rather than passed through.
    #[must_use]
    pub fn eic_qualifying_children(&self) -> u32 {
        (self
            .dependents
            .iter()
            .filter(|d| d.eic_qualifying_child)
            .count() as u32)
            .min(3)
    }
    /// **Tax-Calculator's `nu18`** — people under 18 in the filing unit.
    ///
    /// ★★ Inert under BASELINE law, and said here rather than assumed: the three age-band counts
    ///    reach only `UBI` (a reform-only universal-basic-income parameter, all rates $0) and
    ///    `AGI`'s `pre_c04600 = max(0, XTOT − nu18) × II_em`, whose `II_em` is $0 for 2018–2025.
    ///    They are carried because they are part of the row both engines' input space defines, not
    ///    because a compared line moves — and a filer whose age is unknown is in NO band, which is
    ///    why [`Self::xtot`] does not sum them.
    #[must_use]
    pub fn nu18(&self) -> u32 {
        self.age_band(&|a| a < 18)
    }
    /// **Tax-Calculator's `n1820`** — people aged 18 to 20 in the filing unit. See [`Self::nu18`].
    #[must_use]
    pub fn n1820(&self) -> u32 {
        self.age_band(&|a| (18..=20).contains(&a))
    }
    /// **Tax-Calculator's `n21`** — people 21 or older in the filing unit. See [`Self::nu18`].
    #[must_use]
    pub fn n21(&self) -> u32 {
        self.age_band(&|a| a >= 21)
    }
    fn age_band(&self, f: &dyn Fn(u32) -> bool) -> u32 {
        let mut n = self.dependents.iter().filter(|d| f(d.age)).count() as u32;
        n += u32::from(self.age_head.is_some_and(f));
        n += u32::from(self.age_spouse.is_some_and(f));
        n
    }

    /// ★★★ **The row's CANONICAL form — the fixture's own many-to-one collapse, written down.**
    ///
    /// [`build_golden_return`] is not injective, in exactly two places, and neither is a hole in
    /// [`project_to_golden`]:
    ///
    /// 1. **`itemized_deductions` is the corpus's catch-all *"other itemized"* lump**, and the
    ///    fixture materialises it by ADDING it to the Form 1098 row that carries `mortgage_interest`
    ///    (`let interest = golden_usd(i.itemized_deductions + i.mortgage_interest)`). Two rows whose
    ///    split differs build the byte-identical return, so nothing downstream — no oracle, no
    ///    projection — could tell them apart.
    /// 2. **A spouse's age and blindness are dropped on a return with no spouse.**
    ///    `build_golden_return` creates the spouse `Person` only on *Married/Joint*.
    ///
    /// The inverse KAT compares canonical forms, and separately asserts that `build(g)` and
    /// `build(g.canonical())` produce the IDENTICAL return — which is what makes this a statement
    /// about the fixture rather than a fudge that lets the kill pass.
    #[must_use]
    pub fn canonical(&self) -> GoldenInputs {
        let mut c = self.clone();
        c.mortgage_interest += c.itemized_deductions;
        c.itemized_deductions = 0.0;
        if c.filing_status != "Married/Joint" {
            c.age_spouse = None;
            c.blind_spouse = false;
        }
        c
    }

    /// ★★★ **The row as the AMOUNT partition compares it** — every ROUTING fact normalised away
    /// (T11 fold).
    ///
    /// `oracle_projection.rs`'s money probe asks one question: *does this `Usd` leaf's FIGURE reach
    /// an oracle box?* Without this, the answer would be contaminated by a leaf's side effects on a
    /// routing fact — perturbing `schedule_a.medical` to $777,777 flips 1040 line 12 from the
    /// standard deduction to the itemized one, which moves [`Self::standard_or_itemized`] and would
    /// make medical read as "visible" even though **not one dollar of it** reaches any oracle box.
    /// The routing dimension has its own partition, which asks the right question about it.
    #[must_use]
    pub fn money_view(&self) -> GoldenInputs {
        GoldenInputs {
            standard_or_itemized: GoldenDeduction::default(),
            ..self.clone()
        }
    }
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

/// ★★★ **THE FIVE FILING-STATUS TOKENS BOTH DRIVERS AGREE ON** (T11 fold, I-4).
///
/// OpenTaxSolver's `Status` field takes exactly these five words
/// (`tax_form_files/US_1040/US_1040_template.txt`), and `scripts/oracle/gen_goldens.py`'s
/// `TAXCALC_MARS` is keyed by the same five (`"Single"`, `"Married/Joint"`, `"Married/Sep"`,
/// `"Head_of_House"`, `"Widow(er)"` → `MARS` 1–5). So the token set is the drivers', not ours.
///
/// ★★ **The match is `_`-free on purpose.** [`project_to_golden`] used to fall through
///    `other => format!("{other:?}")`, emitting the enum's own `Debug` name — `"HoH"`, `"Mfs"`,
///    `"Qss"` — which matches no `TAXCALC_MARS` key and which [`build_golden_return`] then panicked
///    on. `income project` exited 0 and printed a full row, and the failure surfaced three steps
///    later as a Rust panic inside the harness. That is the standing *"no decision keys on a list
///    you typed beside derived data"* rule, written against a status set T8 had just widened; a
///    sixth `FilingStatus` variant now fails to COMPILE here until a human gives it a token.
#[must_use]
pub fn golden_filing_status_token(status: FilingStatus) -> &'static str {
    match status {
        FilingStatus::Single => "Single",
        FilingStatus::Mfj => "Married/Joint",
        FilingStatus::Mfs => "Married/Sep",
        FilingStatus::HoH => "Head_of_House",
        FilingStatus::Qss => "Widow(er)",
    }
}

/// Every filing status the oracle row models, in one place so a KAT can round-trip all of them.
/// Paired with [`golden_filing_status_token`], whose exhaustive match is the compiler's net.
pub const GOLDEN_FILING_STATUSES: [FilingStatus; 5] = [
    FilingStatus::Single,
    FilingStatus::Mfj,
    FilingStatus::Mfs,
    FilingStatus::HoH,
    FilingStatus::Qss,
];

/// The inverse of [`golden_filing_status_token`], DERIVED from it rather than retyped: a token is
/// looked up by emitting each status's own word and comparing. `None` ⇒ a token neither driver
/// knows, which [`build_golden_return`] refuses rather than guessing at.
#[must_use]
pub fn golden_filing_status(token: &str) -> Option<FilingStatus> {
    GOLDEN_FILING_STATUSES
        .into_iter()
        .find(|s| golden_filing_status_token(*s) == token)
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
    // ★ T11 fold (I-4) — all five statuses, through the drivers' own token table. The `Single`/`Mfj`
    //   hand-list this replaced is what made `income project` emit a row that panicked here.
    let status = golden_filing_status(&i.filing_status)
        .unwrap_or_else(|| panic!("unmapped filing status {:?}", i.filing_status));

    let mut ri = ReturnInputs {
        filing_status: status,
        // ★ T11 — the corpus's own year, stated rather than left at the `0` "not stated" sentinel.
        //   Every leg this function pushes is dated 2024 and `btctax-oracle-harness`'s `YEAR` is
        //   2024; the age tests the dependents block drives read `ri.tax_year`, so a `0` here would
        //   make a ten-year-old child 2,014 years old.
        tax_year: GOLDEN_TAX_YEAR,
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
    // ★★ THE HAZARD, NAMED — **and closed at T11.** This line used to be
    //    `Some(date!(1980 - 05 - 05))`, with the comment *"`GoldenInputs` carries no age today, so
    //    the day it does, this line must read it instead of assuming."* It now reads
    //    [`GoldenInputs::age_head`], whose default ([`GOLDEN_ADULT_AGE`] = 44) reproduces that exact
    //    date for the corpus's tax year, so every committed cell builds the identical return.
    //
    // ★ `None` is the DECLINED date of birth and is left absent: §1(g)'s question then comes back,
    //   which is the honest outcome rather than an invented birthday.
    ri.header.taxpayer.date_of_birth = i.age_head.map(golden_dob);
    ri.header.taxpayer.blind = i.blind_head.then_some(true);
    // ★★★ §G-9 — the DEATH question, answered, exactly as [`not_a_dependent`] answers it: *"an
    //     ordinary filer has ALSO answered that they did not die during the tax year."* It is class
    //     (B), so silence FORGOES the §63(f) age-65 addition — and a corpus cell that states
    //     `age_head: 66` and is then given no addition would describe a household neither engine can
    //     be handed. Measured: with the gate blank, a 66-year-old MFJ filer's 1040 line 12 stays at
    //     the flat $29,200 while both oracles add $1,550.
    //
    // ★ Invisible on the 107 committed cells, whose taxpayer is 44: the addition needs a date of
    //   birth before January 2 of the year − 64, and the advisory that reports the forfeit fires
    //   only when a qualifying date of birth is on file.
    ri.header.taxpayer_died_during_year = Some(false);
    if status == FilingStatus::Mfj {
        ri.header.spouse = Some(crate::tax::return_inputs::Person {
            first_name: "Golden".into(),
            last_name: "Spouse".into(),
            ssn: "987654321".into(),
            date_of_birth: i.age_spouse.map(golden_dob),
            blind: i.blind_spouse.then_some(true),
            ..Default::default()
        });
        ri.header.can_be_claimed_as_dependent_spouse = Some(false);
        ri.header.spouse_died_during_year = Some(false);
    }
    for (n, d) in i.dependents.iter().enumerate() {
        ri.header.dependents.push(golden_dependent_row(n, *d));
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
        // ★★★ T9 — the oracles' `e19200` / OTS `A8a` mortgage interest is now a Form 1098 ROW, and
        //     `itemized_deductions` (the corpus's catch-all "other itemized") rides with it exactly
        //     as it did on the scalar, so every engine still sees the same figure on line 8a.
        let interest = golden_usd(i.itemized_deductions + i.mortgage_interest);
        if interest > Usd::ZERO {
            ri.form_1098.push(crate::tax::return_inputs::Form1098 {
                lender: "ORACLE MORTGAGE".into(),
                box1_interest: interest,
                other_borrower_paid_interest: Some(false),
                ..Default::default()
            });
            ri.documents.set(
                crate::tax::document_census::DocumentRow::Form1098,
                Some(true),
            );
        }
    }
    // ★ §170(f)(8) — an itemizing return claiming a gift of $250 or more must state whether it holds
    //   a contemporaneous written acknowledgment, and P4 refuses one that has not. This household
    //   HOLDS one; that is a fact about the fixture, stated rather than defaulted, and it is not
    //   grease — a corpus cell that refused would simply be dropped by the admission loop and the
    //   non-interaction would go on being unwitnessed.
    if i.charitable_cash > 0.0 {
        ri.charitable_cwa_obtained = Some(true);
    }
    if i.unemployment > 0.0 {
        ri.g_1099.push(Form1099G {
            payer: "ORACLE STATE".into(),
            box1_unemployment: golden_usd(i.unemployment),
            ..Default::default()
        });
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
    // ★★★ T16 — the §223 HSA deduction. The oracle gives a DEDUCTION; btctax gives Form 8889 the
    //     CONTRIBUTION and lets line 13 produce the deduction — which is the point of driving the
    //     form rather than the figure. They agree exactly while the contribution is within the
    //     year's §223(b) limit, which the corpus cell's own `why` is required to keep it under.
    //
    // ★ The seven Form 8889 declarations are answered the way an ordinary single-coverage HSA
    //   holder answers them: self-only coverage, eligible every month with the same coverage, under
    //   55, no Medicare, one HSA, no Archer MSA, no testing-period failure. Every one of those is a
    //   real answer to a question the form asks — none is a default, and each is what makes line 3
    //   the flat limit rather than the worksheet's.
    if i.hsa_deduction > 0.0 {
        ri.sch1.hsa_activity = Some(true);
        ri.hsa = crate::tax::return_inputs::HsaInputs {
            family_coverage: Some(false),
            eligible_every_month_same_coverage: Some(true),
            age_55_or_older_at_year_end: Some(false),
            enrolled_in_medicare_any_month: Some(false),
            both_spouses_have_hsas: Some(false),
            archer_msa_activity: Some(false),
            testing_period_failure: Some(false),
            line2_contributions_you_made: golden_usd(i.hsa_deduction),
            ..Default::default()
        };
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

    // ★★★ T11 — THE FIXTURE CHECKS ITSELF. `golden_dependent_row` answers fifteen §152 gates to
    //     steer the flowchart to a stated credit column; if the answers and the column ever drift
    //     apart, a corpus cell would silently model a DIFFERENT household from the one it names —
    //     and `n24` is the variable the whole line-19 excuse turns on. FR-88 is this exact shape:
    //     three consecutive tasks shipped a correct guard whose fixture never reached the case.
    for (row, want) in i.dependents.iter().enumerate() {
        let got = crate::tax::dependent_gates::credit_column(&ri, row);
        let got = match got {
            crate::tax::dependent_gates::CreditColumn::ChildTaxCredit => {
                GoldenCreditColumn::ChildTaxCredit
            }
            crate::tax::dependent_gates::CreditColumn::CreditForOtherDependents => {
                GoldenCreditColumn::CreditForOtherDependents
            }
            crate::tax::dependent_gates::CreditColumn::Neither => GoldenCreditColumn::Neither,
        };
        assert_eq!(
            got,
            want.credit,
            "golden dependent row {row} (age {}) was built to land in {:?} and the flowchart put it \
             in {got:?}. The gate answers in `golden_dependent_row` and the stated credit column \
             have drifted apart. Verdict: {:?}",
            want.age,
            want.credit,
            crate::tax::dependent_gates::walk_dependent(&ri, row).verdict,
        );
    }

    (ri, state)
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// T11 / SPEC_interview.md R13 — THE ORACLE PATH.
//
// `build_golden_return` runs one way: an oracle row becomes btctax's return. Everything below runs
// the OTHER way, which is the direction a real filer needs — `project_to_golden` turns a
// `ReturnInputs` (+ its ledger) into the row BOTH engines take, so a return that has never been
// near the corpus can still be put in front of two independent tax engines before it is printed.
//
// ★★★ Box-named on both sides, and NO ARITHMETIC OF ITS OWN. Every figure below is read from the
//     same `return_1040` helper the printed return reads (`sum_wages`, `sum_taxable_interest`,
//     `schedule_c_net_profit`, `forms::schedule_d`). That is the sweep's own I4 rule turned inward:
//     a projection that re-summed the boxes would be a THIRD chain, and `CLAUDE.md`'s standing
//     example is `total_tax` short by the whole AMT with every test green because two chains
//     computed it separately.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// A date of birth whose **considered age at the end of [`GOLDEN_TAX_YEAR`]** is `age`.
///
/// May 5 is not the IRS's January-1 boundary, so `considered_age_at_year_end` is simply
/// `year − dob.year()` and the inverse is exact. It also reproduces `1980-05-05` for
/// [`GOLDEN_ADULT_AGE`], which is the literal date `build_golden_return` used before T11 — so every
/// committed golden household builds byte-identically.
fn golden_dob(age: u32) -> time::Date {
    time::Date::from_calendar_date(
        GOLDEN_TAX_YEAR - i32::try_from(age).expect("a golden age fits in an i32"),
        time::Month::May,
        5,
    )
    .expect("May 5 exists in every year")
}

/// One `Dependent` row realising a [`GoldenDependent`] — every §152 gate answered so that the
/// flowchart reaches exactly the stated credit column.
///
/// ★ It ASSERTS the outcome (`build_golden_return` re-walks the row and panics on a mismatch), so a
///   corpus cell cannot ask for a column the answers below do not actually produce. Without that
///   assertion this function would be the FR-88 shape: a fixture that decides what the checker can
///   see, quietly one step short of the case it exists to reach.
fn golden_dependent_row(n: usize, d: GoldenDependent) -> Dependent {
    // Step 1's age test is `age < 19 && younger`, `age < 24 && student && younger`, or `disabled`.
    // Student and disabled are answered "no" below, so a row is a §152(c) QUALIFYING CHILD exactly
    // when it is under 19 — which makes the Step-1-vs-Step-4 routing a function of the stated age.
    let qualifying_child = d.age < 19;
    Dependent {
        name: format!("Golden Dependent {}", n + 1),
        // Nine digits, distinct per row: two rows sharing one SSN is a REFUSAL
        // (`DependentSsnDuplicated`), and a blank one blocks before any gate.
        ssn: format!("90000{:04}", n + 1),
        relationship: if qualifying_child { "Child" } else { "Parent" }.into(),
        date_of_birth: Some(golden_dob(d.age)),
        // ── Step 1 ───────────────────────────────────────────────────────────────────────────────
        qc_relationship: Some(true),
        younger_than_you_or_spouse: Some(true),
        full_time_student: Some(false),
        permanently_and_totally_disabled: Some(false),
        provided_over_half_own_support: Some(false),
        filing_joint_return: Some(false),
        joint_return_only_to_claim_refund: None, // not demanded: the first limb is "no"
        lived_with_you_over_half_year: Some(true),
        // §32(c)(3)(A)'s extra limb over §152(c): the principal abode is in the UNITED STATES.
        lived_with_you_in_us: Some(d.eic_qualifying_child),
        // ── Step 2 / Step 4's shared gates ───────────────────────────────────────────────────────
        qualifying_child_of_another_person: Some(false),
        citizen_national_resident_or_canada_mexico: Some(true),
        married: Some(false),
        // ── Step 3 / Step 5 — the CREDIT COLUMN. ─────────────────────────────────────────────────
        // "Neither box" is a FORGO the instruction states in its own words, and the gate it hangs
        // on is the TIN question (`:1639-1643`, "No ⇒ no credit box"), not a refusal.
        tin_issued_by_due_date: Some(d.credit != GoldenCreditColumn::Neither),
        citizen_national_or_resident_alien: Some(true),
        // Step 3 question 4 — "Yes ⇒ the child tax credit box; No ⇒ Step 5". Only reached by a
        // qualifying child under 17, which is why the CTC column asserts that age below.
        ssns_valid_for_employment_issued_by_due_date: Some(
            d.credit == GoldenCreditColumn::ChildTaxCredit,
        ),
        // ── Step 4 — answered whether or not this row takes that branch. Harmless when it does
        //    not: every screen and the walk itself consult only the gates the walk DEMANDED.
        qr_relationship_or_member_of_household: Some(true),
        qualifying_child_of_any_taxpayer: Some(false),
        gross_income_under_limit: Some(true),
        you_provided_over_half_support: Some(true),
        divorced_separated_multiple_support_or_kidnapped_rule_applies: Some(false),
    }
}

/// ★★★ **THE PROJECTION — `ReturnInputs` (+ its ledger) → the oracle row.** SPEC_interview.md R13.
///
/// Box-named on both sides:
///
/// | oracle row | Tax-Calculator | OpenTaxSolver | btctax |
/// |---|---|---|---|
/// | `w2_income` | `e00200` | `L1a` | Σ `w2s[].box1_wages` |
/// | `taxable_interest` | `e00300` | `L2b` | Σ `int_1099[].(box1 + box3 + box10)` + Schedule B filer records |
/// | `ordinary_dividends` | `e00600` | `L3b` | Σ `div_1099[].box1a` + Schedule B filer records |
/// | `qualified_dividends` | `e00650` | `L3a` | Σ `div_1099[].box1b` |
/// | `short_term_capital_gains` | `p22250` | 8949 rows | Schedule D Part I gain (the LEDGER) |
/// | `long_term_capital_gains` | `p23250` | 8949 rows | Schedule D Part II gain + Σ `div_1099[].box2a` |
/// | `self_employment_income` | `e00900` | `S1_3` | Schedule C line 31 |
/// | `unemployment` | `e02300` | `S1_7` | Σ `g_1099[].box1` |
/// | `state_income_tax` | `e18400` | `A5a` | **the filed Schedule A's own line 5a** |
/// | `real_estate_tax` | `e18500` | `A5b` | **the filed Schedule A's own line 5b** |
/// | `mortgage_interest` | `e19200` | `A8a` | **the filed Schedule A's own 8a + 8b + 8c** |
/// | `charitable_cash` | `e19800` | `A11` | Σ `schedule_a.charitable[]` on Schedule A **line 11** |
/// | `standard_or_itemized` | (taxcalc decides) | `A5a…A18` gate | `ar.deduction_is_itemized` |
/// | `hsa_deduction` | `e03290` | `S1_13` | Form 8889 line 2, iff `sch1.hsa_activity` |
/// | the dependents block | `n24`/`XTOT`/`EIC`/ages/blindness | `Dependents`/`You_65+Over?`/… | the T7/T8 answers |
///
/// ★★★ **The four Schedule A figures are READ OFF THE FILED SCHEDULE A** ([`crate::tax::return_1040::ScheduleAParts`]),
///     which is why this takes the assembled [`crate::tax::return_1040::AbsoluteReturn`]. They used
///     to be re-derived here from `ReturnInputs`, and all three re-derivations were WRONG in the
///     taxpayer's favour — each one described a household with a bigger deduction than the return
///     btctax files, so both engines would have been asked a different question and 1040 line 12
///     would have diverged on a correct return:
///
///  1. **State income tax.** `schedule_a_line5a` summed `salt_state_estimated_payments +
///     salt_prior_year_balance_paid`; the filed line 5a is
///     [`crate::tax::return_1040::income_tax_salt`], which ALSO includes W-2 box 17 + box 19.
///  2. **Mortgage interest.** It summed Form 1098 box 1 only; the filed line 8a is
///     Σ(box 1 + box 6 points) — and is **$0** on a MIXED-USE mortgage
///     (`mortgage_all_used_to_buy_build_improve == Some(false)`, §163(h)(3)(F)), which the
///     re-derivation ignored entirely.
///  3. Both were invisible to every KAT, because a leaf that reaches the row **at the wrong value**
///     is still "visible" to the amount partition. They were found by the T11-fold routing probe.
///
/// ★★ **`charitable_cash` carries Schedule A LINE 11 only** — a §170 gift *by cash or check*
///    (`Cash60`/`Cash30`). A line-12 gift (`CapGainProp30`, `OrdinaryProp50`, …) has its own
///    §170(b) ceiling that OTS 2024 does not apply, so carrying it into `e19800`/`A11` would invite
///    a false divergence on a correct return; it is REPORTED by
///    [`unprojected_nonzero_leaves`] instead, exactly as
///    [`crate::tax::return_1040::unprojected_ledger_lines`] already reports a crypto donation on the
///    same line. [`charitable_gift_projects`] is the single predicate both readers share.
///
/// ★★ **`itemized_deductions` is deliberately always $0.** It is the corpus's catch-all *"other
///    itemized"* lump, which `build_golden_return` folds into the Form 1098 row; a real return has
///    no such lump, so projecting one would double-count line 8a. The round trip still holds
///    because the sum `itemized_deductions + mortgage_interest` is what the fixture materialises,
///    and the projection returns the whole sum as `mortgage_interest` — see
///    [`GoldenInputs::canonical`], which is why the inverse KAT compares canonical forms.
///
/// ★ **What it does NOT carry** is [`ORACLE_INVISIBLE`], asserted complete by
///   `oracle_projection.rs` in BOTH dimensions: every `Usd` leaf of `ReturnInputs` either moves this
///   projection or is named there with a reason (the AMOUNT partition), and every non-`Usd` leaf
///   whose perturbation makes the described household stop reproducing the filed return is named
///   there too (the ROUTING partition — T11 fold).
#[must_use]
pub fn project_to_golden(
    ri: &ReturnInputs,
    state: &LedgerState,
    ar: &crate::tax::return_1040::AbsoluteReturn,
) -> GoldenInputs {
    use crate::tax::dependent_gates::{credit_column, walk_dependent, CreditColumn};
    use crate::tax::return_1040 as r1040;

    let year = ri.tax_year;
    let f = |u: Usd| -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        u.to_f64().expect("a return's figures are finite")
    };
    let sched_d = crate::forms::schedule_d(state, year);
    let (b1099_st, b1099_lt) = r1040::form_1099b_gains(ri);
    let a = ri.schedule_a.as_ref();
    // ★★★ THE FILED SCHEDULE A, not a second derivation of it — see the doc comment's three
    //     measured defects. `None` on a return with no Schedule A, in which case every one of its
    //     four figures is $0 and no engine is told about a deduction it does not take.
    let sa = ar.schedule_a.as_ref();

    let dependents = (0..ri.header.dependents.len())
        .filter(|row| walk_dependent(ri, *row).verdict.is_claimable())
        .map(|row| {
            let walk = walk_dependent(ri, row);
            let d = &ri.header.dependents[row];
            GoldenDependent {
                age: d
                    .date_of_birth
                    .map(|dob| r1040::considered_age_at_year_end(dob, year))
                    .and_then(|a| u32::try_from(a).ok())
                    .unwrap_or(0),
                // Exhaustive on purpose — a new `CreditColumn` arm must be given an oracle meaning
                // by a human, because the difference between "neither box" and "the CTC box" is up
                // to $2,000 a child and `n24` is what the whole line-19 excuse turns on.
                credit: match credit_column(ri, row) {
                    CreditColumn::ChildTaxCredit => GoldenCreditColumn::ChildTaxCredit,
                    CreditColumn::CreditForOtherDependents => {
                        GoldenCreditColumn::CreditForOtherDependents
                    }
                    CreditColumn::Neither => GoldenCreditColumn::Neither,
                },
                // §32(c)(3)(A): a §152(c) qualifying child (Step 1 said "Go to Step 2") whose
                // principal abode was in the UNITED STATES for more than half the year — row (5)(b)
                // of the Dependents grid, which btctax already collects.
                eic_qualifying_child: walk.is_qualifying_child()
                    && d.lived_with_you_in_us == Some(true),
            }
        })
        .collect();

    GoldenInputs {
        // ★ I-4 — the drivers' own token, from an exhaustive match. Never the enum's `Debug` name.
        filing_status: golden_filing_status_token(ri.filing_status).to_string(),
        w2_income: f(r1040::sum_wages(ri)),
        taxable_interest: f(r1040::sum_taxable_interest(ri)),
        qualified_dividends: f(r1040::sum_qualified_dividends(ri)),
        ordinary_dividends: f(r1040::sum_ordinary_dividends(ri)),
        // ★★★ THE LEDGER'S Schedule D **PLUS the Form 1099-B rows' netted gains**, which is where
        //     `capital_net` joins them (§G-28/B4: *"the broker totals join the crypto nets HERE, at
        //     the §1222 within-character netting"*). Omitting them would have understated a
        //     brokered filer's `p22250`/`p23250` by the whole securities gain — found by the
        //     DERIVED visible/invisible probe, which is the entire reason it is a probe and not a
        //     hand-written list of what the author remembered to carry.
        short_term_capital_gains: f(sched_d.st.gain + b1099_st),
        // Box 2a capital-gain distributions have LONG-TERM character and enter AGI through
        // Schedule D exactly as a disposal does, so they ride `p23250` rather than vanishing.
        long_term_capital_gains: f(sched_d.lt.gain + b1099_lt + r1040::sum_cap_gain_distr(ri)),
        self_employment_income: f(r1040::schedule_c_net_profit(ri, state, year)),
        // See the note above: the corpus's "other itemized" lump has no counterpart on a real
        // return, and line 8a already carries everything the projection can see.
        itemized_deductions: 0.0,
        state_income_tax: f(sa.map_or(Usd::ZERO, |s| s.salt_5a)),
        real_estate_tax: f(sa.map_or(Usd::ZERO, |s| s.salt_5b)),
        mortgage_interest: f(
            sa.map_or(Usd::ZERO, |s| s.mortgage_8a + s.mortgage_8b + s.mortgage_8c)
        ),
        charitable_cash: f(a.map_or(Usd::ZERO, |a| {
            a.charitable
                .iter()
                .filter(|g| charitable_gift_projects(g.class))
                .map(|g| g.amount)
                .sum()
        })),
        // ★ T11 fold — WHICH deduction 1040 line 12 actually claimed, from btctax's own decision.
        //   `ots_direct.py` gates its whole Schedule A block on this; see [`GoldenDeduction`].
        standard_or_itemized: if ar.deduction_is_itemized {
            GoldenDeduction::Itemized
        } else {
            GoldenDeduction::Standard
        },
        // Form 8889 line 2 — the CONTRIBUTION. Both engines take a DEDUCTION and neither applies
        // the §223(b) limit, and the two are equal exactly while the contribution is inside it;
        // btctax REFUSES an over-limit return (excess contributions need Form 5329), so a return
        // that reaches this projection is inside the limit by construction.
        //
        // ★ T11 fold — gated on `sch1.hsa_activity`, the declaration Form 8889 files on. Without
        //   the gate a filer who answered "no HSA activity" but still had a stale line-2 figure was
        //   described to both engines WITH a §223 deduction btctax does not take. Found by the
        //   routing probe, which watched the described return stop reproducing the filed one.
        hsa_deduction: if ri.sch1.hsa_activity == Some(true) {
            f(ri.hsa.line2_contributions_you_made)
        } else {
            0.0
        },
        unemployment: f(r1040::sum_unemployment(ri)),
        dependents,
        age_head: age_of(ri.header.taxpayer.date_of_birth, year),
        age_spouse: ri
            .header
            .spouse
            .as_ref()
            .and_then(|s| age_of(s.date_of_birth, year)),
        blind_head: ri.header.taxpayer.blind == Some(true),
        blind_spouse: ri
            .header
            .spouse
            .as_ref()
            .is_some_and(|s| s.blind == Some(true)),
    }
}

/// Why one money leaf of [`ReturnInputs`] does not reach [`project_to_golden`]'s output.
///
/// ★★★ It is an ENUM rather than free text because the three cases are not equally benign, and a
///     flat list would let the worst of them hide among the harmless ones. `CLAUDE.md`: *"two blanks
///     look identical on the printed page and are not the same thing."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvisibleBecause {
    /// **Withholding or a payment.** 1040 lines 25–33 sit BELOW every line either engine is asked
    /// for (11, 13, 15, 16, 17, 19, 24, 27), so the figure cannot move a comparison on either side.
    /// Harmless, and complete: a payment does not change a liability.
    PaymentOrWithholding,
    /// **A prior-year carry-in.** Neither engine has an input for one — Tax-Calculator and
    /// OpenTaxSolver both compute a single tax year from scratch — so a return carrying one models a
    /// household neither engine can be asked about. The divergence lands on 1040 line 7 (§1212) or
    /// line 12 (§170(d)), loudly.
    PriorYearCarryIn,
    /// **The oracle row models no such quantity.** [`GoldenInputs`] is the CORPUS's household model,
    /// which is deliberately smaller than the 1040; a real return can carry boxes it has no field
    /// for. A non-zero leaf here makes the projected household DIFFERENT from the filer's, so
    /// `btctax income project` prints it and `check_return.py` reports it beside the diff — the
    /// divergence is then attributable rather than mistaken for a btctax defect.
    ///
    /// ★ This is the variant that must never become a dumping ground. Each entry's `note` names the
    ///   oracle variable that WOULD take the figure, or says why neither engine has one.
    NotInTheOracleRow,
    /// ★★★ **A NON-`Usd` fact that ROUTES money, and is not carried** (T11 fold).
    ///
    /// The other three variants are all about an AMOUNT. This one is about a fact BESIDE an amount
    /// that decides where — or whether — the amount goes: a gift's §170 class, an election, a
    /// declaration that gates a form. It is a separate variant because its failure mode is the
    /// worst of the four and looks like none of them: the amount beside it is perfectly "visible"
    /// to the amount partition, and the row still describes a household the filer is not. The whole
    /// gift is simply absent from the description, and nothing about the row looks wrong.
    ///
    /// ★ Its completeness is derived by `oracle_projection.rs`'s ROUTING partition, which perturbs
    ///   every non-`Usd` leaf to every alternative the TYPE admits (serde's own `unknown variant`
    ///   list; `true`/`false` for a boolean) and asks whether `build_golden_return(project(ri))`
    ///   still reproduces the return `ri` files.
    RoutingFactNotCarried,
}

/// One entry of [`ORACLE_INVISIBLE`].
#[derive(Debug, Clone, Copy)]
pub struct OracleInvisibleLeaf {
    /// A serde leaf path with every `Vec` index normalised to `[]`, matched as a PREFIX (the entry
    /// itself, `entry.`, or `entry[`) — the same notion of "prefix" `LEAF_SOURCE` and
    /// `coverage.rs`'s exemptions use.
    pub prefix: &'static str,
    pub because: InvisibleBecause,
    /// The mechanism, in one sentence: which oracle variable would take the figure, or why none can.
    pub note: &'static str,
}

/// ★★★ **EVERY LEAF OF [`ReturnInputs`] THAT DOES NOT REACH THE ORACLE ROW, WITH ITS REASON**
/// (SPEC_interview.md R13; the routing half added by the T11 fold).
///
/// ★★★ **TWO partitions, and the second one exists because the first cannot see it.** The AMOUNT
///     partition asks *"does this `Usd` leaf's figure reach an oracle box?"*. The ROUTING partition
///     asks *"does this non-`Usd` fact reach the drivers?"* — and it is a different question, because
///     a fact that ROUTES money is invisible to a probe that only watches figures. Two Criticals of
///     the T11 seam review were one defect of exactly that shape: the itemize election (now the row's
///     `standard_or_itemized`) and a charitable gift's §170 class, both of which decide where money
///     goes without being money.
///
/// ★★★ **§G-9 — NOTHING BELOW IS VALIDATED BY ORACLE AGREEMENT, AND NEITHER ARE THE GATES**
///     (T11 seam review M-3). *"A value the oracles take as INPUT is never validated by their
///     agreement."* Every fact this table names is either absent from the description (so no engine
///     has an opinion at all) or handed to the drivers AS AN INPUT — the filing status, the
///     dependents block, the aged/blind boxes, the itemize election. Their agreement on a line those
///     facts feed is agreement about a household we described; it is never evidence that the fact
///     itself is right. What validates the gates is R2's transcription checks — the census, the
///     answered-ness classifier, `cite_check`, the line-coverage ratchet — and `check_return.py`
///     says the same thing for 1040 lines 19/27/28 in its own docstring. A reader of this path must
///     take no comfort from a green run about any answer btctax put INTO the row.
///
/// ★★ **The partition is DERIVED, never declared.** `oracle_projection.rs` walks the money leaves of
///    the maximal sentinel with `leaf_walk::money_leaves` — by TYPE, through `Decimal`'s own
///    deserializer — perturbs each one, and asks whether [`project_to_golden`] moved. A leaf that
///    moved it is VISIBLE and may not appear here; a leaf that did not must appear exactly once.
///    Nothing below decides what is visible: the projection does, and this table only has to account
///    for its complement. A `Usd` field added to `ReturnInputs` tomorrow reds that KAT until a human
///    either projects it or writes a line here.
///
/// ★ That derivation is what found the one real gap in the first draft: the Form 1099-B rows'
///   `short_term_proceeds`/`long_term_proceeds`/`…_basis` were classified INVISIBLE, and they are
///   not — `capital_net` joins the broker totals to the crypto nets at the §1222 netting
///   (§G-28/B4), so the projection was understating a brokered filer's `p22250`/`p23250` by the
///   whole securities gain. A hand-written list would have recorded the author's belief instead.
pub const ORACLE_INVISIBLE: &[OracleInvisibleLeaf] = &[
    // ── Withholding and payments — 1040 lines 25-33, below every compared line ────────────────────
    OracleInvisibleLeaf {
        prefix: "payments",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "1040 lines 26 and 31 (estimated tax, extension payment, other withholding) — \
               payments against a liability, never a term of one",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box2_fed_withheld",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "W-2 box 2 → 1040 line 25a",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box4_ss_withheld",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "W-2 box 4 → the §6413(c) excess-social-security credit, 1040 line 31 / Schedule 3 \
               line 11",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box6_medicare_withheld",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "W-2 box 6 → Form 8959 line 24 → 1040 line 25c",
    },
    // ★ T11 fold — `w2s[].box17_state_tax_withheld` and `w2s[].box19_local_tax` USED to sit here,
    //   as PaymentOrWithholding, on the ground that "btctax is federal-only and neither engine is
    //   asked for a state return". Both halves were false: `income_tax_salt` puts them on Schedule A
    //   line 5a, so they reach `e18400`/`A5a` on every itemizing return. The projection now reads
    //   the FILED line 5a, so both are VISIBLE and the KAT reds if either is listed again.
    OracleInvisibleLeaf {
        prefix: "int_1099[].box4_fed_withheld",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "Form 1099-INT box 4 → 1040 line 25b",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box4_fed_withheld",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "Form 1099-DIV box 4 → 1040 line 25b",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box4_fed_withheld",
        because: InvisibleBecause::PaymentOrWithholding,
        note: "Form 1099-G box 4 → 1040 line 25b",
    },
    // ── Prior-year carry-ins — a one-year engine has no input for one ────────────────────────────
    OracleInvisibleLeaf {
        prefix: "capital_loss_carryforward_in",
        because: InvisibleBecause::PriorYearCarryIn,
        note: "§1212(b) — `capital_net` applies it at the §1222 netting, moving 1040 line 7; \
               neither engine takes a carry-in",
    },
    OracleInvisibleLeaf {
        prefix: "charitable_carryover_in",
        because: InvisibleBecause::PriorYearCarryIn,
        note: "§170(d)(1) — moves Schedule A line 11/12; neither engine takes a carry-in",
    },
    OracleInvisibleLeaf {
        prefix: "qbi",
        because: InvisibleBecause::PriorYearCarryIn,
        note: "§199A(c)(2) / (b)(1)(B) negative-QBI and REIT/PTP carryforwards into the year; \
               neither engine takes one",
    },
    // ── Quantities the oracle row does not model ─────────────────────────────────────────────────
    OracleInvisibleLeaf {
        prefix: "w2s[].box3_ss_wages",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the row carries ONE wage figure, and both drivers reuse it for the §1402(b)(1) \
               OASDI channel (OTS Schedule SE line 8a, taxcalc `e00200p`) — so a filer whose box 3 \
               differs from box 1 (deferrals, or the wage base) is modelled with box 1 in both",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box5_medicare_wages",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the Form 8959 Part I channel; see box 3 — one wage figure, reused",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box7_ss_tips",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "W-2 box 7 — the row has no tips field (TY2025 Schedule 1-A Part II is btctax-only; \
               taxcalc models the deduction, not the box)",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box8_allocated_tips",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "W-2 box 8 — see box 7",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box10_dependent_care",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "W-2 box 10 → Form 2441; taxcalc has `e32800`/`f2441` and OTS a Form 2441 solver, \
               but the oracle row models no dependent-care benefit",
    },
    OracleInvisibleLeaf {
        prefix: "w2s[].box12",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "W-2 box 12 codes — deferrals, HSA (code W), and the rest; the oracle row models none \
               of them",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box2_early_withdrawal_penalty",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Schedule 1 line 18 (OTS `S1_18`); taxcalc has no early-withdrawal-penalty input",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box6_foreign_tax",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the §901 foreign tax credit (taxcalc `e07300`, Form 1116); the oracle row models no \
               credit",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box8_tax_exempt_interest",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§103 tax-exempt interest, 1040 line 2a (taxcalc `e00400`) — outside AGI, but inside \
               the §86 and §469 modified-AGI tests neither the row nor btctax v1 exercises",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box9_private_activity_bond_amt",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 6251 line 2g; the AMT is COMPARED (1040 line 17) but the oracle row has no \
               private-activity-bond field, so a non-zero box 9 understates both engines' AMTI",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box11_bond_premium",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§171 amortizable bond premium — an offset to interest neither engine takes as its \
               own input",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box12_bond_premium_treasury",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "see box 11",
    },
    OracleInvisibleLeaf {
        prefix: "int_1099[].box13_bond_premium_tax_exempt",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "see box 11",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box2b_unrecap_1250",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§1250 recapture, taxed at 25% by the Schedule D worksheet; the row's \
               `long_term_capital_gains` carries one §1(h) rate group only",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box2c_section_1202",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§1202 gain; see box 2b — one rate group",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box2d_collectibles_28",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the 28% collectibles rate group; see box 2b",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box5_section_199a",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§199A(b)(1)(B) REIT/PTP dividends (taxcalc `e26270`-adjacent); the row's QBI is the \
               Schedule C figure only",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box7_foreign_tax",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "see Form 1099-INT box 6 — the foreign tax credit",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box9_cash_liquidation",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§331 liquidating distributions — a basis-recovery figure neither engine inputs",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box10_noncash_liquidation",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "see box 9",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box12_exempt_interest_dividends",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "see Form 1099-INT box 8 — tax-exempt interest",
    },
    OracleInvisibleLeaf {
        prefix: "div_1099[].box13_private_activity_amt",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "see Form 1099-INT box 9 — Form 6251 line 2g",
    },
    OracleInvisibleLeaf {
        prefix: "b_1099[].box13_bartering",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 1099-B box 13, bartering income → Schedule 1 line 8z; the row has no other-income \
               field. (The broker PROCEEDS and BASIS boxes DO project — `capital_net` joins them to \
               the crypto nets at the §1222 netting.)",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box2_state_refund",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the taxable state refund is taken from `sch1.state_refund_taxable` (§111 recovery); \
               see that entry",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box5_rtaa_payments",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "RTAA payments → Schedule 1 line 8z; the row has no other-income field",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box6_taxable_grants",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "taxable grants → Schedule 1 line 8z; see box 5",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box7_agriculture_payments",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "agriculture payments → Schedule F, which btctax does not model at all",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box9_market_gain",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "CCC-loan market gain → Schedule F; see box 7",
    },
    OracleInvisibleLeaf {
        prefix: "g_1099[].box10_family_leave_benefits",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "paid-family-leave benefits → Schedule 1 line 8z; see box 5",
    },
    OracleInvisibleLeaf {
        prefix: "form_1098[].box2_outstanding_principal",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the §163(h)(3)(B) acquisition-debt ceiling is a btctax WARNING computed from the \
               principal; neither engine models the ceiling, so neither takes the principal",
    },
    OracleInvisibleLeaf {
        prefix: "form_1098[].box4_refund_overpaid_interest",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "a §111 recovery of previously deducted interest; btctax REFUSES a non-zero box 4 \
               (`MortgageInterestRefundNotComputed`), so it never reaches an oracle",
    },
    OracleInvisibleLeaf {
        prefix: "form_1098[].box5_mortgage_insurance",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§163(h)(3)(E) mortgage insurance premiums — the deduction expired after 2021 and \
               Schedule A no longer carries the line",
    },
    // ★ T11 fold — `form_1098[].box6_points` USED to sit here, claiming *"Schedule A line 8a takes
    //   them inside box 1's figure"*. It does not: line 8a is Σ(box 1 + box 6)
    //   (`i1040sca--2025.txt:1060-1061`), which `schedule_a_parts` implements and the old
    //   re-derivation in this file did not. Reading the FILED 8a makes the box visible.
    OracleInvisibleLeaf {
        prefix: "form_1098e[].box1_interest",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§221 student loan interest, Schedule 1 line 21 (taxcalc `e03210`, OTS `S1_21`) — \
               both engines model it and the oracle row has no field, so a filer with a 1098-E is \
               projected without the deduction",
    },
    OracleInvisibleLeaf {
        prefix: "sch1.state_refund_taxable",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§111 taxable state refund, Schedule 1 line 1 (taxcalc `e00700`, OTS `S1_1`) — both \
               engines model it and the oracle row has no field",
    },
    OracleInvisibleLeaf {
        prefix: "sch1.ira_deduction_claimed",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§219 IRA deduction, Schedule 1 line 20 (taxcalc `e03150`, OTS `S1_20`) — both \
               engines model it and the oracle row has no field",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_a.medical",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Schedule A line 1, the §213(a) floor's numerator (taxcalc `e17500`, OTS `A1`) — both \
               engines model it and the oracle row has no field",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_a.investment_interest",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Schedule A line 9. taxcalc's `e19200` is the line-15 TOTAL and would take it, but \
               OTS reads line 9 as its own `A9` while `GoldenInputs::mortgage_interest` is line 8a \
               on both sides — carrying it there would put investment interest on OTS's line 8a",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_a.salt_personal_property",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Schedule A line 5c. OTS has `A5c`; taxcalc's `e18400`/`e18500` split is income/sales \
               versus real estate with no personal-property slot, so carrying it would be a \
               single-witness figure",
    },
    // ── Facts that ROUTE money and are not carried (T11 fold) ────────────────────────────────────
    OracleInvisibleLeaf {
        prefix: "header.taxpayer_died_during_year",
        because: InvisibleBecause::RoutingFactNotCarried,
        note: "§G-9 — a filer who died during the year FORGOES §63(f)'s age-65 addition (class (B): \
               silence forgoes), so 1040 line 12 falls by the addition. The oracle row carries an \
               AGE and neither engine models a death: OTS asks only `You_65+Over?` and taxcalc only \
               `age_head`, so the described household keeps the box the filed return gives up. \
               FR-93 records the consequence — such a filer diverges from BOTH engines on line 12 \
               by a multiple of the addition, deliberately NOT excused, because answering the \
               question is the filer's own remedy",
    },
    OracleInvisibleLeaf {
        prefix: "header.spouse_died_during_year",
        because: InvisibleBecause::RoutingFactNotCarried,
        note: "the spouse's half of the §G-9 death gate — see `header.taxpayer_died_during_year`; \
               OTS's `Spouse_65+Over?` and taxcalc's `age_spouse` are ages, never a death",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_a.charitable[].class",
        because: InvisibleBecause::RoutingFactNotCarried,
        note: "the §170 class decides Schedule A line 11 (cash: taxcalc `e19800`, OTS `A11` — the \
               row's `charitable_cash`) from line 12 (property: `e20100` / `A12`). A line-12 gift \
               is REPORTED rather than carried: it has its own §170(b) 30%/50%-of-AGI ceiling and \
               OTS 2024 applies no §170(b) ceiling at all, so putting it into `A12` would hand \
               OpenTaxSolver a deduction btctax caps and OTS does not — a false divergence on a \
               correct return. `charitable_gift_projects` is the one predicate that decides, and \
               `unprojected_nonzero_leaves` reports the dropped gift's amount under this entry",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_c.qbi_ubia",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§199A(b)(2)(B) UBIA of qualified property — a Form 8995-A operand; the corpus keeps \
               every household under the §199A(e)(2) threshold, where the limitation does not apply",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_c.qbi_w2_wages",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§199A(b)(2)(A) W-2 wages of the business; see `qbi_ubia`",
    },
    OracleInvisibleLeaf {
        prefix: "schedule_1a",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "TY2025 Schedule 1-A (tips, overtime, car-loan interest, the senior deduction). \
               `verify_schedule_1a.py` is its own two-oracle harness; the golden corpus is TY2024",
    },
    OracleInvisibleLeaf {
        prefix: "hsa.line10_qualified_funding_distribution",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§408(d)(9) IRA-to-HSA funding distribution, Form 8889 line 10 — the row carries the \
               §223 DEDUCTION (`e03290` / `S1_13`), not Form 8889's interior",
    },
    OracleInvisibleLeaf {
        prefix: "hsa.employer_contributions_prior_year",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the Employer Contribution Worksheet's calendar-versus-tax-year adjustment; see \
               `line10_qualified_funding_distribution`",
    },
    OracleInvisibleLeaf {
        prefix: "hsa.employer_contributions_next_year",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the other half of the Employer Contribution Worksheet; see above",
    },
    OracleInvisibleLeaf {
        prefix: "hsa.line14b_rollovers_and_withdrawn_excess",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 8889 Part II distributions; the row models no HSA distribution",
    },
    OracleInvisibleLeaf {
        prefix: "hsa.line15_qualified_medical_expenses",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 8889 line 15; see line 14b",
    },
    OracleInvisibleLeaf {
        prefix: "hsa.line16_amount_meeting_an_exception",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "the §223(f)(4)(C) exception to the 20% additional tax; see line 14b",
    },
    OracleInvisibleLeaf {
        prefix: "sa_1099",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 1099-SA rows (HSA distributions) → Form 8889 Part II; see `hsa.line14b_…`",
    },
    OracleInvisibleLeaf {
        prefix: "sa_5498",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 5498-SA rows — transcribed so Form 8889 line 2 can be checked against what the \
               trustee reported; no line of the return sums them, so nothing to project",
    },
    OracleInvisibleLeaf {
        prefix: "form_8960_line9b",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "Form 8960 line 9b, i8960's \"any reasonable method\" state-tax allocation. OTS's \
               Form 8960 solver is driven with lines 1/2/5a/13 only and taxcalc computes `niit` \
               with no 9b input, so neither engine can be told the allocation",
    },
    OracleInvisibleLeaf {
        prefix: "excluded_puerto_rico_income",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§933 — a §164(b)(7)(B)(iv) / Schedule 1-A modified-AGI add-back; neither engine \
               models the exclusion",
    },
    OracleInvisibleLeaf {
        prefix: "form_2555_line45",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§911 foreign earned income exclusion (Form 2555); neither engine takes it",
    },
    OracleInvisibleLeaf {
        prefix: "form_2555_line50",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§911 housing exclusion; see line 45",
    },
    OracleInvisibleLeaf {
        prefix: "form_4563_line15",
        because: InvisibleBecause::NotInTheOracleRow,
        note: "§931 American Samoa exclusion (Form 4563); see Form 2555",
    },
];

/// Resolve one money-leaf path to its [`ORACLE_INVISIBLE`] entry, or `None` when the leaf projects.
///
/// Vec indices are normalised (`w2s[0].box2_fed_withheld` → `w2s[].box2_fed_withheld`) so one entry
/// covers every row, and the match is a PREFIX in the same sense `LEAF_SOURCE` uses.
#[must_use]
pub fn oracle_invisible_entry(path: &str) -> Option<&'static OracleInvisibleLeaf> {
    let p = normalize_leaf_path(path);
    ORACLE_INVISIBLE.iter().find(|e| {
        p == e.prefix
            || p.starts_with(&format!("{}.", e.prefix))
            || p.starts_with(&format!("{}[", e.prefix))
    })
}

/// `w2s[0].box12[1].amount` → `w2s[].box12[].amount`.
#[must_use]
pub fn normalize_leaf_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut in_index = false;
    for c in path.chars() {
        match c {
            '[' => {
                in_index = true;
                out.push('[');
            }
            ']' => {
                in_index = false;
                out.push(']');
            }
            _ if in_index => {}
            _ => out.push(c),
        }
    }
    out
}

/// ★★★ **The non-zero money leaves of `ri` that [`project_to_golden`] does NOT carry**, each with
/// its [`ORACLE_INVISIBLE`] reason — what `btctax income project` prints beside the row and
/// `check_return.py` reports beside the diff.
///
/// Without it the oracle path has a silent-truncation failure mode: a filer with $8,000 of medical
/// expenses would be projected as a filer with none, both engines would answer a DIFFERENT
/// household's question, and the resulting divergence on 1040 line 12 would read as a btctax defect.
/// Naming the omission is what makes a divergence attributable.
///
/// ★★★ **AND the gifts a ROUTING fact drops** (T11 fold, I-1). A money leaf can be "visible" —
/// `schedule_a.charitable[].amount` reaches `charitable_cash` — and still be dropped on a particular
/// return, because a non-`Usd` fact beside it decides whether it rides at all. The class is that
/// fact. So a gift [`charitable_gift_projects`] rejects is reported here, by the SAME predicate the
/// projection sums by; a hand-list would be a second thing to keep true.
#[must_use]
pub fn unprojected_nonzero_leaves(
    ri: &ReturnInputs,
) -> Vec<(String, Usd, &'static OracleInvisibleLeaf)> {
    let mut out: Vec<(String, Usd, &'static OracleInvisibleLeaf)> =
        crate::tax::provenance::leaf_walk::nonzero_money_leaves(ri)
            .into_iter()
            .filter_map(|(path, amount)| oracle_invisible_entry(&path).map(|e| (path, amount, e)))
            .collect();
    let class_entry = oracle_invisible_entry("schedule_a.charitable[0].class")
        .expect("ORACLE_INVISIBLE names the charitable class");
    for (i, g) in ri
        .schedule_a
        .iter()
        .flat_map(|a| a.charitable.iter())
        .enumerate()
    {
        if !charitable_gift_projects(g.class) && g.amount != Usd::ZERO {
            out.push((
                format!("schedule_a.charitable[{i}].amount"),
                g.amount,
                class_entry,
            ));
        }
    }
    out
}

fn age_of(dob: Option<time::Date>, year: i32) -> Option<u32> {
    dob.map(|d| crate::tax::return_1040::considered_age_at_year_end(d, year))
        .and_then(|a| u32::try_from(a).ok())
}

/// ★★★ **Does a §170 gift of this class reach the oracle row?** — the ONE predicate
/// [`project_to_golden`] sums by and [`unprojected_nonzero_leaves`] reports by, so the two can never
/// disagree about which gift was carried (T11 fold, I-1).
///
/// ★★ **Only Schedule A line 11's cash class projects.** `charitable_cash` is Tax-Calculator's
///    `e19800` and OTS's `A11` — *"Gifts by cash or check"*. A line-12 gift of PROPERTY
///    (`CapGainProp30` for long-term capital-gain property, `OrdinaryProp50` for ordinary-income or
///    basis property — the two classes a filed btctax return can carry) has its own §170(b)
///    30%/50%-of-AGI ceiling, and **OTS 2024 applies no §170(b) ceiling at all**: putting such a
///    gift into `A12` would hand OpenTaxSolver a deduction btctax caps and OTS does not, i.e. a
///    FALSE divergence on a correct return. That is the V2b shape `charitable_cash`'s own doc
///    records. So it is reported instead — the same disposition
///    [`crate::tax::return_1040::unprojected_ledger_lines`] already gives a crypto donation, which
///    lands on the identical line.
///
/// ★ The remaining three classes (`Cash30`, `CapGainProp20`, `OrdinaryProp30`) are non-50%-
///   organization gifts, which `RefuseReason::NonPublicCharityContribution` refuses upstream, so no
///   filed return reaches this function with one. The match is `_`-free: a seventh class must be
///   given an answer by a human.
#[must_use]
pub fn charitable_gift_projects(class: CharitableClass) -> bool {
    match class {
        CharitableClass::Cash60 => true,
        CharitableClass::Cash30
        | CharitableClass::CapGainProp30
        | CharitableClass::CapGainProp20
        | CharitableClass::OrdinaryProp50
        | CharitableClass::OrdinaryProp30 => false,
    }
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
