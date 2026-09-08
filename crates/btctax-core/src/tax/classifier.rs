//! ★ P9 §3.3 — THE CLASSIFIER. Makes the answered-ness invariant STRUCTURAL: every `bool` /
//! `Option<bool>` / defaulted-enum field reachable from [`ReturnInputs`] is classified — as a registry
//! DECLARATION (class A), or EXEMPTED with its §2 class and the statutory reason it is lawful.
//!
//! **The compile-time guarantee.** Every struct is destructured with **NO `..`**, so a newly-added field
//! is a `pattern does not mention field` COMPILE ERROR until a human edits this file. `#![deny(unused_variables)]`
//! then makes a named binding that *ignores* its field a hard error too (the `w2s: whatever` evasion r3 I-5
//! defeated the r3 wording with). So: **a new such field does not compile until a human LOOKS.**
//!
//! **★ The honest limit (r3 I-5), stated at its true strength.** The compiler forces "a human must EDIT
//! the classifier", NOT "classified it correctly." The residual evasions — a `_`-prefixed binding, or
//! `let _ = x;` — are **grep-able REVIEW residue**, not compile errors. That is the whole guarantee, and it
//! is worth having: every recurrence of this class began with a field nobody looked at.
//!
//! **★ The `_` rule (r2 M-6).** `_` — and every `_`-prefixed binding — is **FORBIDDEN** on structs,
//! collections, and `bool` / `Option<bool>` / **`Option<Usd>`** / defaulted-enum leaves (they must recurse
//! or be classified). `_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`,
//! `Option<Date>`, `Option<String>`).
//!
//! ★★★ **`Option<Usd>` was added to the forbidden set, and the codebase had already ROUTED AROUND its
//! absence.** `return_inputs.rs` says of the §911/931/933 gate: *"The gate carries the answered-ness, not
//! the amounts, and that is deliberate. `Option<bool>` is a leaf the classifier forbids `_` on, so it
//! cannot be added without a human classifying it — whereas `Option<Usd>` is a scalar the `_` rule
//! permits."* A design choosing its TYPE to avoid a hole in this rule is the clearest possible evidence
//! the hole was real. `Option<Usd>` is the answered-ness shape for money — `None` = never asked — and
//! letting it slip past unclassified is exactly the class this module exists to prevent.
//!
//! ★ Enforced by [`tests::no_option_money_leaf_is_bound_with_underscore`], which reads BOTH sources and
//! is exercised on planted defects — not by doctrine.
//!
//! This module does no tax arithmetic and is never on a compute path; [`classify`] returns a [`Census`] only
//! so a test can prove the registry declarations line up with [`FORM_QUESTIONS`].
#![deny(unused_variables)]

use crate::tax::questions::QuestionId;
use crate::tax::return_inputs::{
    Box12Entry, CharitableCarryItem, CharitableGift, Dependent, Form1099Div, Form1099G,
    Form1099Int, HouseholdHeader, Payments, Person, QbiInputs, ReturnInputs, Schedule1Inputs,
    Schedule1aInputs, Schedule1aOvertime, Schedule1aTips, Schedule1aVehicle, ScheduleAInputs,
    ScheduleCInputs, W2,
};
use crate::tax::types::Carryforward;

/// The §2 class under which a defaulted input is lawful. (Class **A** is not here — it is a registry
/// DECLARATION, recorded via [`Census::declaration`]; class **D** is gone, deleted in step 9.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// **(B) BENEFIT CLAIM** — *New Colonial Ice*: the burden to CLAIM is the filer's, so `false`/absent is
    /// lawful, but the forgone benefit gets an advisory (§2.2).
    BenefitClaim,
    /// **(C) NO TAX DIRECTION** — neither asserts nor claims; a lawful silent default (§2.1).
    NoTaxDirection,
    /// serde-REQUIRED at import (the field has no `#[serde(default)]`): a TOML without it refuses to parse,
    /// so there is no default to launder (§2.1 / §2.8).
    SerdeRequired,
    /// The value IS data the record carries (a typed classification), not a defaulted answer for the filer.
    DataDerived,
    /// A defaulted enum WITH tax direction, tracked as an OPEN issue owned by a follow-up (§2.8).
    TrackedFollowup,
}

/// The census [`classify`] produces: which registry questions it declared, and every exemption with its
/// class and statutory reason. Used only by the tests — the load-bearing guarantee is the compile.
#[derive(Debug, Default)]
pub struct Census {
    pub declarations: Vec<QuestionId>,
    pub exemptions: Vec<(Class, &'static str)>,
    /// ★ spec 1099-DA R1 — the Form 1099-DA answers the filer GAVE, per (provider, cohort): their own
    /// class, neither a registry declaration (no neutral exists) nor an exemption. Unanswered keys are
    /// not visible here — the classifier has no ledger; the screen (`screen_absolute`) counts them
    /// against the year's rows and refuses.
    pub broker_answers: Vec<(String, crate::forms::Cohort, crate::forms::BrokerReported)>,
    /// ★★★ T7 / R6 — every per-row dependent gate this walk classified, one entry per gate per row,
    /// **paired with the leaf it was classified FROM** (seam review N-2).
    /// `the_classifiers_gate_rows_line_up_with_the_registry` checks the gate set against
    /// `DEPENDENT_GATES` in both directions, and
    /// `every_classifier_row_pairs_its_gate_with_its_own_leaf` checks the pairing.
    pub dependent_gates: Vec<(crate::tax::provenance::DependentGate, Option<bool>)>,
}

impl Census {
    /// A class-(A) DECLARATION: the filer asserts it; its liveness and refusal live in [`FORM_QUESTIONS`].
    fn declaration(&mut self, _leaf: &Option<bool>, id: QuestionId) {
        self.declarations.push(id);
    }
    /// An exempted default — lawful, with its §2 class and the statutory reason it is lawful.
    fn exempt<T>(&mut self, _leaf: &T, class: Class, statutory_reason: &'static str) {
        self.exemptions.push((class, statutory_reason));
    }
    /// ★★★ **T7 / R6 — a class-(A) DECLARATION on a dependent ROW.** Its liveness and refusal live
    /// in [`crate::tax::dependent_gates::DEPENDENT_GATES`], and its answer-log key is the row's
    /// `ssn_hash`, not its index. Separate from [`Self::declaration`] because the key space is
    /// different, not because the class is.
    ///
    /// ★★ **The LEAF is recorded, not discarded** (seam review N-2). Dropping it made the census a
    ///    set of gate names, and a set is blind to a MIS-PAIRING: swapping two calls in
    ///    `classify_dependent` (`c.dependent_gate(married, G::FilingJointReturn)`) left the set
    ///    identical and red nothing. The value read off the leaf is what ties a row to the field it
    ///    is about.
    fn dependent_gate(&mut self, leaf: &Option<bool>, gate: crate::tax::provenance::DependentGate) {
        self.dependent_gates.push((gate, *leaf));
    }
}

/// ★ Destructure EVERY struct reachable from [`ReturnInputs`] with no `..`, classifying every
/// `bool`/`Option<bool>`/defaulted-enum leaf. Never called on a compute path — its whole job is to fail
/// COMPILATION when a new such field appears unclassified.
pub fn classify(ri: &ReturnInputs) -> Census {
    let mut c = Census::default();
    let ReturnInputs {
        // ★ §G-15 — the tax year is not a DECLARATION or a benefit claim; it is the scope those are
        // asked in. It carries no answered-ness of its own, so the classifier ignores it (naming it
        // explicitly rather than `..`, so the next field added is still a compile error).
        tax_year: _,
        filing_status,
        header,
        documents,
        w2s,
        int_1099,
        div_1099,
        g_1099,
        b_1099,
        form_1098e,
        sa_1099,
        sa_5498,
        schedule_b_filer_records,
        w2_wages_without_w2,
        interest_or_dividends_without_1099,
        state_refund_without_1099g,
        hsa_distribution_without_1099sa,
        itemized_prior_year,
        digital_asset_activity,
        schedule_c,
        schedule_a,
        itemize_election,
        mfs_spouse_itemizes,
        sch1,
        hsa,
        schedule_1a,
        payments,
        broker_reporting,
        capital_loss_carryforward_in,
        capital_loss_carryforward_in_provenance,
        charitable_carryover_in_provenance,
        carryover_includes_spouses_joint_loss,
        excluded_canceled_debt,
        amt_carryover_same_as_regular,
        amt_depreciation_same_as_regular,
        charitable_carryover_in,
        qbi,
        foreign_accounts,
        foreign_trust,
        fbar_filing_required,
        foreign_country_names: _, // String — scalar
        donations_had_restrictions,
        charitable_cwa_obtained,
        filing_form_4952,
        form_8960_line9b,
        dual_status_alien,
        // §164(b)(7)(B)(iv) / Schedule 1-A Part I MAGI add-backs. Plain `Usd` scalar leaves, which the
        // `_` rule permits — their answered-ness lives in the `has_income_exclusion` GATE above, which
        // is an `Option<bool>` for exactly that reason. ★ This comment used to call them
        // `Option<Usd>`; they are not, and the distinction now matters because `Option<Usd>` is
        // forbidden a `_`. They are NOT class-(B) forgone benefits like `medical`: an unasked add-back
        // understates MAGI and RAISES two deductions, so `None` on the gate is refused at the point of
        // need (`SaltLimitation::line_5e`), never defaulted to zero.
        has_income_exclusion,
        other_out_of_scope_income,
        excluded_puerto_rico_income: _,
        form_2555_line45: _,
        form_2555_line50: _,
        form_4563_line15: _,
        answer_log,
        answer_log_history,
        opened_from,
        filing_status_confirmed,
    } = ri;
    // ★★★ **THE GROUND, CORRECTED (T4b seam review I-1).** The old sentence stopped at serde, and
    //     the premise behind it — *"every path onto a `ReturnInputs` forces a human to state it"* —
    //     stopped being true the moment `open_next_year::seed` began CONSTRUCTING one in Rust, where
    //     deserialization never runs and the value arrives from year N. So the exemption now names
    //     BOTH grounds, one per path, and neither is a default:
    //       * a year the filer started themselves — serde refuses a TOML without it;
    //       * a year the OPENER made — `FilingStatusConfirmed`, the class-(A) declaration above,
    //         which `None` blocks on and `No` refuses.
    c.exempt(
        filing_status,
        Class::SerdeRequired,
        "filing_status has no #[serde(default)] — a TOML without it refuses to parse, so no default to \
         launder (§2.1); and on a year the OPENER made (`opened_from`), where no TOML is parsed, the \
         carried status is asserted this year by the class-(A) `FilingStatusConfirmed` declaration \
         (R10.4 / §7703(a)(1))",
    );
    // ★★★ R10.4 / T4b seam review I-1 — the CARRIED filing status has a surface, and this is it. The
    // exemption above says a TOML cannot omit `filing_status`; `open_next_year::seed` is the first
    // path that builds a `ReturnInputs` in Rust, where serde never runs, so the value arrives from
    // year N. This declaration is what the filer answers on such a year, and it is class (A): §7703(a)(1)
    // determines marital status on the LAST DAY of the tax year, so a prior year's status is not
    // testimony for this one.
    c.declaration(filing_status_confirmed, QuestionId::FilingStatusConfirmed);
    c.declaration(mfs_spouse_itemizes, QuestionId::MfsSpouseItemizes);
    c.declaration(foreign_accounts, QuestionId::ForeignAccounts);
    c.declaration(foreign_trust, QuestionId::ForeignTrust);
    // ★ §G-20a — provenance for the two benefit carryovers. Class (C): no print, no tax direction.
    // They exist so a ZERO can be told apart from an UNASKED, which is what makes an honest advisory
    // possible; the per-item provenance on `CharitableCarryItem` cannot speak for an EMPTY list.
    c.exempt(
        capital_loss_carryforward_in_provenance,
        Class::NoTaxDirection,
        "§2.8: CarryProvenance (§1212(b) capital-loss carryover, §G-20a) — no print, no tax direction",
    );
    c.exempt(
        charitable_carryover_in_provenance,
        Class::NoTaxDirection,
        "§2.8: CarryProvenance (§170(d)(1) charitable carryover LIST, §G-20a) — no print, no tax direction",
    );
    // ★ Schedule B 7a's unnumbered FBAR sub-question. NOT a declaration: no figure on the return
    // reads it (the printed chain writes the checkbox and nothing else), so its silence neither
    // asserts nor forgoes — class (C). Asked as `SkippableId::FbarFilingRequired`, and skipping it
    // fires `Advisory::FbarSubQuestionNotAnswered` quoting the form's Caution verbatim.
    c.exempt(
        fbar_filing_required,
        Class::NoTaxDirection,
        "Schedule B line 7a's FBAR sub-question — no figure on the return reads it, and the penalty \
         the form's Caution names attaches to NOT FILING FinCEN Form 114 (a FinCEN obligation \
         independent of this box), not to leaving the box blank. Silence is lawful and prints a true \
         blank; `Advisory::FbarSubQuestionNotAnswered` fires (§2.1)",
    );
    // ★★ Form 8283 5a/5b/5c, asked as ONE return-level universal (§G-21). Class (B) HERE — offered
    // always, silence lawful — because the donations are in the LEDGER and liveness cannot see them.
    // The MANDATORY half lives in `screen_absolute`, which can: on an ITEMIZING year an
    // unanswered or `Some(true)` answer REFUSES.
    c.exempt(
        donations_had_restrictions,
        Class::BenefitClaim,
        "Form 8283 5a/5b/5c (§G-21) — offered as a skippable so a filer who donated nothing is never \
         blocked; on an itemizing year that claims the §170 deduction `screen_absolute` makes it \
          mandatory (§2.2)",
    );
    // ★★ §170(f)(8)'s CWA, asked as ONE return-level universal (P4). Class (B) HERE for the SAME two
    // reasons as its sibling above — the donations are in the ledger and the §63(e) itemize election
    // is computed, neither of which liveness can see. The MANDATORY half is in `screen_absolute`: on
    // an itemizing year that claims a §170 deduction with at least one single gift of $250 or more,
    // an unanswered or `Some(false)` answer REFUSES.
    c.exempt(
        charitable_cwa_obtained,
        Class::BenefitClaim,
        "§170(f)(8) contemporaneous written acknowledgment — offered as a skippable so a standard-\
         deduction filer, or one whose every gift is under $250, is never asked; `screen_absolute` \
         makes it mandatory where the deduction is actually claimed (§2.2)",
    );
    c.declaration(filing_form_4952, QuestionId::FilingForm4952);
    // ★★ Form 8960 line 9b — an `Option<Usd>`, the answered-ness shape for money, so the `_` rule
    // forbids a bare binding and it must be classified. Class (B): §1411(c)(1)(B) allows the
    // deduction, it does not impose it, so `None` CLAIMS NOTHING and is lawful — New Colonial Ice
    // again. It is not class (A): btctax must not pick the filer's "reasonable method" of allocation,
    // so there is no answer a refusal could demand, only an amount they may choose to enter.
    c.exempt(
        form_8960_line9b,
        Class::BenefitClaim,
        "Form 8960 line 9b (§1411(c)(1)(B)) — the state/local income tax the filer allocates to net \
         investment income, by \"any reasonable method\" that is THEIR election (i8960). Silence \
         forgoes the whole Part II deduction, which can only OVERSTATE the tax; \
         `Advisory::Form8960Line9bNotClaimed` names the forgone amount and the bound (§2.2)",
    );
    c.declaration(dual_status_alien, QuestionId::DualStatusAlien);
    c.declaration(has_income_exclusion, QuestionId::HasIncomeExclusion);
    c.declaration(other_out_of_scope_income, QuestionId::OtherOutOfScopeIncome);
    c.declaration(
        amt_carryover_same_as_regular,
        QuestionId::AmtCarryoverSameAsRegular,
    );
    // ★ The Capital Loss Carryover Worksheet's two unnumbered header conditions. Class (A) like
    //   their line-2k sibling above: each ASSERTS a fact about this year under §6065, each refuses
    //   when `None`, and each refuses again at `Some(true)` — a YES names a computation btctax does
    //   not perform (a per-spouse split; §108(b) attribute reduction), not a value it can absorb.
    c.declaration(
        carryover_includes_spouses_joint_loss,
        QuestionId::CarryoverIncludesSpousesJointLoss,
    );
    c.declaration(excluded_canceled_debt, QuestionId::ExcludedCanceledDebt);
    c.declaration(
        amt_depreciation_same_as_regular,
        QuestionId::AmtDepreciationSameAsRegular,
    );
    c.exempt(
        itemize_election,
        Class::NoTaxDirection,
        "§2.8: `Auto` takes max(standard, itemized) — an OPTIMIZATION that cannot lose money, not an \
         assertion and not a forgone benefit; §63(e) `ForceItemize` is opt-in",
    );
    classify_header(&mut c, header);
    classify_document_census(&mut c, documents);
    for w in w2s {
        classify_w2(&mut c, w);
    }
    for i in int_1099 {
        classify_1099int(&mut c, i);
    }
    for d in div_1099 {
        classify_1099div(&mut c, d);
    }
    for g in g_1099 {
        classify_1099g(&mut c, g);
    }
    for b in b_1099 {
        classify_1099b(&mut c, b);
    }
    for e in form_1098e {
        classify_1098e(&mut c, e);
    }
    for r in sa_1099 {
        classify_1099sa(&mut c, r);
    }
    for r in sa_5498 {
        classify_5498sa(&mut c, r);
    }
    for r in schedule_b_filer_records {
        classify_schedule_b_record(&mut c, r);
    }
    // ★★★ R3 — the three DOCUMENT-LESS INCOME questions, each class (A). A census `No` never closes
    //     income the instructions say to report without the document: the form names incomes that
    //     exist with no information return behind them, and each is small money in the
    //     UNDERSTATEMENT direction, where the residual attestation is the only net.
    c.declaration(w2_wages_without_w2, QuestionId::WagesWithoutW2Question);
    c.declaration(
        interest_or_dividends_without_1099,
        QuestionId::InterestOrDividendsWithout1099,
    );
    c.declaration(
        state_refund_without_1099g,
        QuestionId::StateRefundWithout1099g,
    );
    // ★★★ Seam review M-1 — R3's fourth door, Form 8889 line 14a. Class (A) for the same reason
    //     its three siblings are: a `false` btctax assumed rather than asked would leave line 14a
    //     blank on the filer's behalf, and a distribution missing from line 14a is missing gross
    //     income under §223(f) plus the 20% additional tax.
    c.declaration(
        hsa_distribution_without_1099sa,
        QuestionId::HsaDistributionWithout1099sa,
    );
    // ★★★ R3/I1 — the prior-year itemize gate, RETURN-LEVEL because a filer with no 1099-G owes the
    //     same answer. Class (A): §111(a)'s tax-benefit rule decides whether the refund is income,
    //     and a `false` btctax assumed rather than asked would blank Schedule 1 line 1 on the
    //     filer's behalf — an understatement laundered as a lawful blank.
    c.declaration(itemized_prior_year, QuestionId::ItemizedPriorYear);
    // ★★★ R9 / T6 — the Form 1040 page-1 DIGITAL ASSETS question. Class (A) and ALWAYS live: the
    //     form prints it on every return, and the box that prints is now this ANSWER rather than
    //     the ledger predicate, which could only ever say *Yes* or leave a mandatory question
    //     blank on a §6065-signed page.
    c.declaration(digital_asset_activity, QuestionId::DigitalAssetActivity);
    if let Some(sc) = schedule_c {
        classify_schedule_c(&mut c, sc);
    }
    classify_schedule_1a(&mut c, schedule_1a);
    if let Some(a) = schedule_a {
        classify_schedule_a(&mut c, a);
    }
    classify_schedule1(&mut c, sch1);
    classify_hsa(&mut c, hsa);
    classify_payments(&mut c, payments);
    // ★★★ R10.3 — THE ANSWER LOG. Not an answer, and never a default that could answer FOR the filer:
    // a record exists only because a writer observed an act. Class (C): no figure on the return reads
    // it, and its absence asserts nothing (an absent record is precisely "never asked", which is the
    // distinction the log exists to make). Its `AnswerState` enum has **no** `Default`, so there is no
    // defaulted answer to launder — the shape the D-8 laundering took.
    // ★★★ R10.4 / T4b — WHICH YEAR THIS RETURN WAS OPENED FROM. Provenance about the OPENING, in
    // the same class as the answer log: no printed figure reads it, the filer never types it, and it
    // has no neutral value to launder. It exists so the surfaces the filer meets DAYS later can tell
    // a carried identity from one this year gave — which is what makes the confirmation above, and
    // the date-of-birth HINT, possible at all.
    c.exempt(
        opened_from,
        Class::NoTaxDirection,
        "R10.4 opener provenance — which year this return was opened FROM: no printed figure reads \
         it, the filer never types it, and `None` means \"the filer started this year themselves\" \
         (§2.1)",
    );
    c.exempt(
        answer_log,
        Class::NoTaxDirection,
        "R10.3 answer log — provenance about the ASKING, not an answer: no printed figure reads it, an \
         absent record means \"never asked\", and `AnswerState` has no Default to launder (§2.1)",
    );
    c.exempt(
        answer_log_history,
        Class::NoTaxDirection,
        "R10.3 answer-log history — append-only superseded records that NOTHING reads as an answer \
         (§5.6); it can neither assert nor forgo anything on the return (§2.1)",
    );
    classify_broker_reporting(&mut c, broker_reporting);
    classify_carryforward(&mut c, capital_loss_carryforward_in);
    for item in charitable_carryover_in {
        classify_charitable_carry(&mut c, item);
    }
    classify_qbi(&mut c, qbi);
    c
}

fn classify_header(c: &mut Census, h: &HouseholdHeader) {
    let HouseholdHeader {
        taxpayer,
        spouse,
        address_street: _,
        address_city: _,
        address_state: _,
        address_zip: _,
        dependents,
        can_be_claimed_as_dependent_taxpayer,
        can_be_claimed_as_dependent_spouse,
        spouse_had_no_income,
        spouse_not_filing_a_return,
        presidential_fund_taxpayer,
        presidential_fund_spouse,
        taxpayer_died_during_year,
        spouse_died_during_year,
        ip_pin: _, // Option<String> — scalar
        form8615_condition3_age_support,
        form8615_condition4_parent_alive,
        form8615_parent_identity_unobtainable,
        filer_tin_issued_by_due_date,
        hoh_marital_basis,
        hoh_qualifying_person,
        hoh_paid_over_half_cost_of_keeping_up_home,
        hoh_qualifying_child_name: _, // String — a scalar the `_` rule permits
        nra_spouse_resident_election,
        qss_spouse_died_in_window_and_not_remarried,
        qss_child_you_can_claim,
        qss_child_lived_in_your_home_all_year,
        qss_paid_over_half_cost_of_keeping_up_home,
        qss_could_have_filed_jointly_in_year_of_death,
    } = h;
    // ★★★ R7 / T8 — HEAD OF HOUSEHOLD and QUALIFYING SURVIVING SPOUSE. Every one is a class-(A)
    //     DECLARATION: checking either box is an assertion about the filer's household that unlocks
    //     money (a wider bracket and standard deduction; the joint rates), so silence may not stand
    //     for a *yes*, and none of them is a benefit the filer may lawfully forgo — forgoing would
    //     mean filing under a status whose tests were never met.
    //
    // ★ `hoh_marital_basis` is the ONE non-boolean here, so it is not a `declaration` (which takes
    //   an `Option<bool>`). It is class (A) all the same, through
    //   `SkippableQuestion::unanswered` — `SKIPPABLE_QUESTIONS` is where the CHOICE shape lives, and
    //   that field is what says its silence is not lawful. See `questions.rs`.
    c.exempt(
        hoh_marital_basis,
        Class::SerdeRequired,
        "R7 — the HoH marital basis is a CHOICE (`HohMaritalBasis`), so it is asked through          `SkippableId::HohMaritalBasis` rather than as an `Option<bool>` declaration. Its silence is          NOT lawful: the registry entry carries `unanswered: Some(RefuseReason::         HohMaritalBasisUnanswered)`, which `screen_inputs` and the R12 panel both read, and no          serde default names a variant",
    );
    c.declaration(hoh_qualifying_person, QuestionId::HohQualifyingPerson);
    c.declaration(
        hoh_paid_over_half_cost_of_keeping_up_home,
        QuestionId::HohPaidOverHalfCostOfKeepingUpHome,
    );
    c.declaration(
        nra_spouse_resident_election,
        QuestionId::NraSpouseResidentElection,
    );
    c.declaration(
        qss_spouse_died_in_window_and_not_remarried,
        QuestionId::QssSpouseDiedInWindowAndNotRemarried,
    );
    c.declaration(qss_child_you_can_claim, QuestionId::QssChildYouCanClaim);
    c.declaration(
        qss_child_lived_in_your_home_all_year,
        QuestionId::QssChildLivedInYourHomeAllYear,
    );
    c.declaration(
        qss_paid_over_half_cost_of_keeping_up_home,
        QuestionId::QssPaidOverHalfCostOfKeepingUpHome,
    );
    c.declaration(
        qss_could_have_filed_jointly_in_year_of_death,
        QuestionId::QssCouldHaveFiledJointlyInYearOfDeath,
    );
    c.declaration(
        can_be_claimed_as_dependent_taxpayer,
        QuestionId::DependentTaxpayer,
    );
    // ★ T7 / R6 — Step 5 question 1: a class-(A) declaration like its two neighbours, live iff the
    //   return carries a dependent row.
    c.declaration(
        filer_tin_issued_by_due_date,
        QuestionId::FilerTinIssuedByDueDate,
    );
    c.declaration(
        can_be_claimed_as_dependent_spouse,
        QuestionId::DependentSpouse,
    );
    c.exempt(
        presidential_fund_taxpayer,
        Class::NoTaxDirection,
        "§2.1: 1040 presidential-fund box — §6096 is a fund DESIGNATION, not a tax liability",
    );
    c.exempt(
        presidential_fund_spouse,
        Class::NoTaxDirection,
        "§2.1: 1040 presidential-fund box (spouse) — §6096, no tax direction",
    );
    // ★★ §G-20 — i1040gi's two uncaptured MFS conditions for the spouse's §63(f) boxes. BENEFIT
    // CLAIMS (New Colonial Ice): silence FORGOES the boxes and never grants them, so `false`/absent is
    // lawful and the forgone benefit fires `Mfs63fSpouseBoxesForgone`.
    c.exempt(
        spouse_had_no_income,
        Class::BenefitClaim,
        "§63(f)/i1040gi MFS condition (spouse had no income) — silence forgoes the spouse's aged/blind \
         boxes rather than granting them; `Advisory::Mfs63fSpouseBoxesForgone` names the cost (§2.2)",
    );
    c.exempt(
        spouse_not_filing_a_return,
        Class::BenefitClaim,
        "§63(f)/i1040gi MFS condition (spouse isn't filing a return) — same reasoning (§2.2)",
    );
    // §G-9: the §63(f) death carve-out. i1040gi states it twice — "Death of a taxpayer" and "Death of
    // spouse" — so each is its own leaf. ★ BENEFIT CLAIMS, not declarations: silence FORGOES the
    // age-65 addition (`is_aged`'s `(None, None)` arm returns `false`), so by the sharp test it is
    // class (B), asked as `SkippableId::{Taxpayer,Spouse}DiedDuringYear`. They were briefly class-(A)
    // refusals, and the taxpayer one was `live: |_| true` — it blocked every return for a fail-safe
    // the compute layer already had.
    c.exempt(
        taxpayer_died_during_year,
        Class::BenefitClaim,
        "§63(f)/§G-9 death carve-out (taxpayer) — New Colonial Ice: silence forgoes the age-65 \
         addition rather than granting it on an unresolved carve-out, so `false`/absent is lawful; \
         `Advisory::AgedBoxForfeitedDeathUnanswered` fires when a qualifying DOB makes the skip cost \
         money (§2.2)",
    );
    c.exempt(
        spouse_died_during_year,
        Class::BenefitClaim,
        "§63(f)/§G-9 death carve-out (spouse) — same reasoning; the box is counted exactly when \
         `spouse_63f_boxes_count` says so (MFJ, or a qualifying MFS), which is the same predicate \
         that gates the prompt (§2.2)",
    );
    // ★★★ FR-29 — Form 8615's three leaves. NONE is a `declaration` (that is reserved for
    // `FORM_QUESTIONS`) and none is a `BenefitClaim` — no benefit is claimed by answering. They take
    // the `Class::NoTaxDirection` idiom `qbi_w2_wages` / `qbi_ubia` already established: refused
    // where they are needed, unread where they are not, so they default in NEITHER direction.
    c.exempt(
        form8615_condition3_age_support,
        Class::NoTaxDirection,
        "Form 8615 condition 3 (i1040gi:3932-3940) — `None` is REFUSED by screen_compute_dependent \
         wherever conditions 1 and 5 hold and age 24+ is not provable, and is unread everywhere else, \
         so it defaults in neither direction",
    );
    // ★★ `CannotKnow` is an ANSWER, not a blank — answered-ness is about whether the filer SPOKE,
    // never about which way. It is why this leaf is `Option<ParentAliveAnswer>` and not
    // `Option<bool>`: `None` refuses (Form8615ParentAliveUnanswered) while `Some(CannotKnow)` opens
    // the SPEC §6.3 dead-end path. The classifier cannot assert that — `exempt` discards the leaf —
    // so the gate is where it is provable, and `cannot_know_is_answered_and_none_is_not` proves it.
    c.exempt(
        form8615_condition4_parent_alive,
        Class::NoTaxDirection,
        "Form 8615 condition 4 (i1040gi:3941-3942) — `None` is REFUSED by screen_compute_dependent \
         wherever condition 3 is answered YES, and is unread everywhere else, so it defaults in \
         neither direction; `Some(CannotKnow)` is an ANSWER, not a blank",
    );
    c.exempt(
        form8615_parent_identity_unobtainable,
        Class::NoTaxDirection,
        "the §6.3 dead-end fact — `None` and `Some(false)` are the SAME outcome (Form8615ParentUnidentifiable \
         refuses), so silence defaults in neither direction; only the filer's own `Some(true)`, and only \
         after they answered condition 4 CannotKnow, opens the certification path",
    );
    classify_person(c, taxpayer);
    if let Some(sp) = spouse {
        classify_person(c, sp);
    }
    for d in dependents {
        classify_dependent(c, d);
    }
}

fn classify_person(c: &mut Census, p: &Person) {
    let Person {
        first_name: _,
        last_name: _,
        ssn: _,
        date_of_birth: _, // Option<Date> — scalar
        // Option<Date> — scalar. Its answered-ness is carried by the HEADER gates
        // `taxpayer_died_during_year` / `spouse_died_during_year`, declared in `classify_header`.
        date_of_death: _,
        blind,
        occupation: _,
    } = p;
    c.exempt(
        blind,
        Class::BenefitClaim,
        "§63(f) blindness — New Colonial Ice: the burden to CLAIM is the filer's, so `false`/absent is \
         lawful; the forgone benefit fires `BlindBoxForfeitedNotDeclared` (§2.2)",
    );
}

/// ★★★ **T7 / R6 — the twenty per-row DECLARATIONS of `Who Qualifies as Your Dependent`.**
///
/// Every one is class (A): the filer asserts it, `DEPENDENT_GATES` owns its liveness and its
/// refusal, and `screen_inputs` refuses a live `None`. They are recorded through
/// [`Census::dependent_gate`] rather than [`Census::declaration`] because their key is a
/// [`DependentGate`] on a ROW, not a return-level [`QuestionId`] — and
/// `the_classifiers_gate_rows_line_up_with_the_registry` pins the two lists against each other in
/// both directions, so a gate classified here but absent from the registry (or the reverse) reds.
fn classify_dependent(c: &mut Census, d: &Dependent) {
    use crate::tax::provenance::DependentGate as G;
    // No `..` and no `_` on an `Option<bool>` — a new gate is a compile error here until classified.
    let Dependent {
        name: _,
        ssn: _,
        relationship: _,
        // An `Option<Date>` — a scalar, not a leaf the classifier's `_` rule covers. Its
        // answered-ness is carried by `DependentGate::DateOfBirth` in the same registry, which is
        // why it is REQUIRED (R6) rather than a class-(B) forgo like the taxpayer's.
        date_of_birth: _,
        lived_with_you_over_half_year,
        lived_with_you_in_us,
        full_time_student,
        permanently_and_totally_disabled,
        qc_relationship,
        younger_than_you_or_spouse,
        provided_over_half_own_support,
        filing_joint_return,
        joint_return_only_to_claim_refund,
        qualifying_child_of_another_person,
        citizen_national_resident_or_canada_mexico,
        married,
        tin_issued_by_due_date,
        citizen_national_or_resident_alien,
        ssns_valid_for_employment_issued_by_due_date,
        qr_relationship_or_member_of_household,
        qualifying_child_of_any_taxpayer,
        gross_income_under_limit,
        you_provided_over_half_support,
        divorced_separated_multiple_support_or_kidnapped_rule_applies,
    } = d;
    c.dependent_gate(lived_with_you_over_half_year, G::LivedWithYouOverHalfYear);
    c.dependent_gate(lived_with_you_in_us, G::LivedWithYouInUs);
    c.dependent_gate(full_time_student, G::FullTimeStudent);
    c.dependent_gate(
        permanently_and_totally_disabled,
        G::PermanentlyAndTotallyDisabled,
    );
    c.dependent_gate(qc_relationship, G::QcRelationship);
    c.dependent_gate(younger_than_you_or_spouse, G::YoungerThanYouOrSpouse);
    c.dependent_gate(
        provided_over_half_own_support,
        G::ProvidedOverHalfOwnSupport,
    );
    c.dependent_gate(filing_joint_return, G::FilingJointReturn);
    c.dependent_gate(
        joint_return_only_to_claim_refund,
        G::JointReturnOnlyToClaimRefund,
    );
    c.dependent_gate(
        qualifying_child_of_another_person,
        G::QualifyingChildOfAnotherPerson,
    );
    c.dependent_gate(
        citizen_national_resident_or_canada_mexico,
        G::CitizenNationalResidentOrCanadaMexico,
    );
    c.dependent_gate(married, G::Married);
    c.dependent_gate(tin_issued_by_due_date, G::TinIssuedByDueDate);
    c.dependent_gate(
        citizen_national_or_resident_alien,
        G::CitizenNationalOrResidentAlien,
    );
    c.dependent_gate(
        ssns_valid_for_employment_issued_by_due_date,
        G::SsnsValidForEmploymentIssuedByDueDate,
    );
    c.dependent_gate(
        qr_relationship_or_member_of_household,
        G::QrRelationshipOrMemberOfHousehold,
    );
    c.dependent_gate(
        qualifying_child_of_any_taxpayer,
        G::QualifyingChildOfAnyTaxpayer,
    );
    c.dependent_gate(gross_income_under_limit, G::GrossIncomeUnderLimit);
    c.dependent_gate(
        you_provided_over_half_support,
        G::YouProvidedOverHalfSupport,
    );
    c.dependent_gate(
        divorced_separated_multiple_support_or_kidnapped_rule_applies,
        G::DivorcedSeparatedMultipleSupportOrKidnappedRuleApplies,
    );
}

fn classify_w2(c: &mut Census, w: &W2) {
    let W2 {
        owner,
        employer: _,
        // ★ An IDENTIFIER, not a benefit claim and not testimony about an amount. `None` forgoes no
        //   deduction and asserts nothing; it only means §6413(c)'s "more than one employer" test
        //   cannot be run, which `excess_social_security` answers by refusing to credit rather than by
        //   guessing. Nothing to classify.
        ein: _,
        box1_wages: _,
        box2_fed_withheld: _,
        box3_ss_wages: _,
        box4_ss_withheld: _,
        box5_medicare_wages: _,
        box6_medicare_withheld: _,
        box7_ss_tips: _,
        box17_state_tax_withheld: _,
        box19_local_tax: _,
        box12,
        box8_allocated_tips: _,
        box10_dependent_care: _,
        box13_statutory_employee,
        // A published Treasury taxonomy CODE the employer printed — a `String` scalar the `_` rule
        // permits, and data rather than an answer (`Schedule1aTips::treasury_occupation_code` is
        // classified the same way).
        box14b_treasury_tipped_occupation_codes: _,
    } = w;
    // ★★★ Box 13 *Statutory employee* — the DOCUMENT'S own checkbox, not an answer btctax defaulted
    //     for the filer. The row exists only because the census declared the document (R3), so an
    //     unchecked box is the employer's testimony that the wages are ordinary wages. `true`
    //     REFUSES (`StatutoryEmployeeW2`, Schedule C line 1), so a checked box can never be laundered
    //     into 1040 line 1a; `false` claims nothing and forgoes nothing.
    c.exempt(
        box13_statutory_employee,
        Class::DataDerived,
        "W-2 box 13 *Statutory employee* — a checkbox the EMPLOYER printed on a document the filer \
         declared (R3), not a defaulted answer; `true` refuses StatutoryEmployeeW2 naming Schedule C \
         line 1, so it can never file box 1 on the wrong line (§2.8)",
    );
    c.exempt(
        owner,
        Class::SerdeRequired,
        "§2.8: W2.owner has no #[serde(default)], so the TOML import REQUIRES it; the enum's #[default] \
         Taxpayer reaches only Rust-side fixtures",
    );
    for e in box12 {
        classify_box12(c, e);
    }
}

fn classify_box12(_c: &mut Census, e: &Box12Entry) {
    let Box12Entry {
        code: _, // String — scalar
        amount: _,
    } = e;
}

fn classify_1099int(_c: &mut Census, i: &Form1099Int) {
    let Form1099Int {
        payer: _,
        box1_interest: _,
        box2_early_withdrawal_penalty: _,
        box3_treasury_interest: _,
        box4_fed_withheld: _,
        box6_foreign_tax: _,
        box8_tax_exempt_interest: _,
        box9_private_activity_bond_amt: _,
        box10_market_discount: _,
        box11_bond_premium: _,
        box12_bond_premium_treasury: _,
        box13_bond_premium_tax_exempt: _,
        // R10.2 document identity — a `String` and an `Option<Date>`, both scalar leaves the `_` rule
        // permits. Neither carries answered-ness: an empty TIN is "not transcribed", and the
        // transcription date is a fact about OUR handling of the paper, never testimony.
        payer_tin: _,
        transcribed_on: _,
    } = i;
}

fn classify_1099div(_c: &mut Census, d: &Form1099Div) {
    let Form1099Div {
        payer: _,
        box1a_ordinary: _,
        box1b_qualified: _,
        box2a_capgain_distr: _,
        box2b_unrecap_1250: _,
        box2c_section_1202: _,
        box2d_collectibles_28: _,
        box4_fed_withheld: _,
        box5_section_199a: _,
        box7_foreign_tax: _,
        box9_cash_liquidation: _,
        box10_noncash_liquidation: _,
        box12_exempt_interest_dividends: _,
        box13_private_activity_amt: _,
        // R10.2 document identity — scalar leaves; see `classify_1099int`.
        payer_tin: _,
        transcribed_on: _,
    } = d;
}

fn classify_1099g(_c: &mut Census, g: &Form1099G) {
    let Form1099G {
        payer: _,
        box1_unemployment: _,
        box2_state_refund: _,
        box4_fed_withheld: _,
        box5_rtaa_payments: _,
        box6_taxable_grants: _,
        box7_agriculture_payments: _,
        box9_market_gain: _,
        box10_family_leave_benefits: _,
        // R10.2 document identity — scalar leaves; see `classify_1099int`.
        payer_tin: _,
        transcribed_on: _,
    } = g;
}

/// R4 / §5.2 — Form 1098-E. Four leaves, none of them an answered-ness shape: a lender, a TIN, a
/// transcription date and one box the §221 chain reads.
fn classify_1098e(_c: &mut Census, e: &crate::tax::return_inputs::Form1098E) {
    let crate::tax::return_inputs::Form1098E {
        lender: _,
        lender_tin: _,
        transcribed_on: _,
        box1_interest: _,
    } = e;
}

/// R5 — one filer's-records row for Schedule B lines 1 / 5. The row EXISTS only because
/// `interest_or_dividends_without_1099` was answered YES (a class-(A) declaration above), so nothing
/// here is a default that could answer for the filer; `kind` is data (which list the row joins).
fn classify_schedule_b_record(c: &mut Census, r: &crate::tax::return_inputs::ScheduleBRecord) {
    let crate::tax::return_inputs::ScheduleBRecord {
        payer_name: _,
        payer_ssn: _,
        payer_address: _,
        amount: _,
        kind,
    } = r;
    c.exempt(
        kind,
        Class::DataDerived,
        "the row's Schedule B list (interest → line 1, dividend → line 5) is DATA about the figure, \
         not a defaulted answer for the filer; the row exists only because the class-(A) \
         `InterestOrDividendsWithout1099` declaration was answered YES (§2.8)",
    );
}

/// §G-28/B4 — Form 1099-B totals for Schedule D lines 1a/8a.
fn classify_1099b(c: &mut Census, b: &crate::tax::return_inputs::Form1099B) {
    let crate::tax::return_inputs::Form1099B {
        payer: _,
        short_term_proceeds: _,
        short_term_basis: _,
        long_term_proceeds: _,
        long_term_basis: _,
        box13_bartering: _,
        // R10.2 document identity — scalar leaves; see `classify_1099int`.
        payer_tin: _,
        transcribed_on: _,
        basis_reported_and_no_adjustments,
    } = b;
    // ★★★ The two conditions Schedule D line 1a itself imposes. `None` and `Some(false)` BOTH refuse
    //     (`Form1099BNeedsForm8949`), so this can never answer for the filer — which is why it is a
    //     `BenefitClaim` rather than an exemption: entering totals on line 1a IS the claim that the
    //     broker reported basis and no adjustment is needed.
    c.exempt(
        basis_reported_and_no_adjustments,
        Class::BenefitClaim,
        "Schedule D line 1a/8a's own two conditions (§G-28/B4) — \"basis was reported to the IRS\" \
         and \"you have no adjustments\". Not a default: `None` and `Some(false)` both REFUSE, because \
         anything else belongs on Form 8949 with per-transaction detail",
    );
}

/// ★★★ Schedule 1-A (TY2025) — every eligibility declaration is class **(B) BENEFIT CLAIM**.
///
/// *New Colonial Ice*: the burden to CLAIM a deduction is the filer's, so `false` is lawful and
/// cannot overstate — it forgoes. That is precisely why the whole surface defaults to `false`: an
/// omission fails closed. The forgone-benefit direction is what an advisory is for, not a refusal.
///
/// ★ The money leaves are `Usd`, not `Option<Usd>`, and carry no answered-ness of their own: they are
/// unreachable unless the filer opted into the part (`Option<…>` / a non-empty vehicle list), so a
/// defaulted zero inside an un-claimed part is never printed. `return_refuse` screens them for sign.
fn classify_schedule_1a(c: &mut Census, s1a: &Schedule1aInputs) {
    let Schedule1aInputs {
        tips,
        overtime,
        vehicles,
    } = s1a;
    if let Some(t) = tips {
        let Schedule1aTips {
            qualified_tips_reported: _,
            occupation_on_treasury_list,
            treasury_occupation_code: _, // a Treasury code the filer states — data, not an answer
            excludes_unlisted_occupation_tips,
            meets_qualified_tip_criteria,
        } = t;
        c.exempt(
            occupation_on_treasury_list,
            Class::BenefitClaim,
            "§224 / Sch 1-A Part II Caution — tips must be received in an occupation listed at              IRS.gov/TippedOccupations. `false` no longer merely forgoes: a claimed Part II              (line 4a > 0) with this or either sibling condition false REFUSES              (`RefuseReason::QualifiedTipsCautionNotMet`), because the three are              `#[serde(default)] bool` and a silent default would take the deduction with the              Caution unmet. Unclaimed, `false` still forgoes and cannot overstate.",
        );
        c.exempt(
            excludes_unlisted_occupation_tips,
            Class::BenefitClaim,
            "Sch 1-A Part II line 4 — \"Do not include tips received in occupations that are              not included on this list in line 4a, 4b, or 4c\". `false` beside a claimed              line 4a refuses (`RefuseReason::QualifiedTipsCautionNotMet`); unclaimed, it              forgoes.",
        );
        c.exempt(
            meets_qualified_tip_criteria,
            Class::BenefitClaim,
            "§224(d) — cash medium, voluntary, unnegotiated, customer-determined; service charges              and automatic gratuities are not qualified tips. `false` beside a claimed line 4a              refuses (`RefuseReason::QualifiedTipsCautionNotMet`); unclaimed, it forgoes.",
        );
    }
    if let Some(o) = overtime {
        let Schedule1aOvertime {
            qualified_overtime_reported: _,
            is_flsa_premium_half_only,
            entitlement_arises_under_flsa,
            excludes_amounts_counted_as_tips,
        } = o;
        c.exempt(
            is_flsa_premium_half_only,
            Class::BenefitClaim,
            "§225 — only the FLSA premium HALF qualifies, not double-time's second half nor              holiday/weekend premiums paid absent >40 hours",
        );
        c.exempt(
            entitlement_arises_under_flsa,
            Class::BenefitClaim,
            "§225 — the entitlement must arise under FLSA §7; state-law-only overtime paid to an              FLSA-ineligible employee does not qualify",
        );
        c.exempt(
            excludes_amounts_counted_as_tips,
            Class::BenefitClaim,
            "§225 — excludes any amount received as a qualified tip; the same dollars must not be              deducted under Part II and Part III both",
        );
    }
    for v in vehicles {
        let Schedule1aVehicle {
            description: _, // free text, scrubbed in `scrub.rs`; carries no tax direction
            interest_paid: _,
            loan_originated_after_2024,
            loan_originated_by_you,
            proceeds_used_to_purchase,
            personal_use,
            secured_by_first_lien,
            original_use_starts_with_you,
            is_applicable_vehicle_class_under_14000_lbs,
            final_assembly_in_us,
            excludes_negative_equity,
        } = v;
        // ★★ All nine are one statutory test (§163(h)(4)) and share a reason. Listing them
        //    individually rather than looping is deliberate: the destructure above is what makes a
        //    tenth condition a compile error, and a loop over a hand-built array would not be.
        for (leaf, why) in [
            (
                loan_originated_after_2024,
                "loan originated after 2024-12-31",
            ),
            (loan_originated_by_you, "loan originated by the filer"),
            (
                proceeds_used_to_purchase,
                "proceeds used to PURCHASE; lease payments do not qualify",
            ),
            (personal_use, "the APV is for personal use"),
            (
                secured_by_first_lien,
                "secured by a first lien on the purchased APV",
            ),
            (
                original_use_starts_with_you,
                "original use starts with the filer; a used vehicle does not qualify",
            ),
            (
                is_applicable_vehicle_class_under_14000_lbs,
                "applicable vehicle class, GVWR under 14,000 lb",
            ),
            (final_assembly_in_us, "final assembly in the United States"),
            (
                excludes_negative_equity,
                "excludes traded-in negative equity, which is not eligible",
            ),
        ] {
            let _ = why;
            c.exempt(
                leaf,
                Class::BenefitClaim,
                "§163(h)(4) / Sch 1-A Part IV — a stated eligibility condition; `false` forgoes the                  deduction. An earlier round shipped this part with NO eligibility at all and handed                  every filer who typed a figure up to $10,000, which UNDERSTATES tax.",
            );
        }
    }
}

fn classify_schedule_c(c: &mut Census, sc: &ScheduleCInputs) {
    let ScheduleCInputs {
        owner,
        business_description: _,
        naics_code: _,
        accounting_method,
        expenses: _,
        other_gross_receipts: _,
        payments_requiring_1099,
        will_file_required_1099,
        qbi_w2_wages,
        qbi_ubia,
        is_sstb,
        is_cooperative_patron,
    } = sc;
    // ★★★ §G-28/B1b — the two §199A(b)(2) limitation amounts. `Option<Usd>` is the answered-ness shape
    //     for money, and the `_` rule now FORBIDS waving one past, so these must be classified here.
    //     They are NOT class-(B) forgone benefits like `medical`: an unasked W-2-wage figure defaulting
    //     to zero would CAP the deduction at zero for a filer who does pay wages (overstating tax), and
    //     any other default would invent wages they never reported (understating it). Neither direction
    //     is safe, so `None` refuses at the point of need — above the threshold, where Part II is
    //     reached — and is simply unread below it.
    c.exempt(
        qbi_w2_wages,
        Class::NoTaxDirection,
        "Form 8995-A line 4: `None` is REFUSED above the §199A(e)(2) threshold (screen_absolute) and \
         unread below it, so it defaults to nothing in either direction",
    );
    c.exempt(
        qbi_ubia,
        Class::NoTaxDirection,
        "Form 8995-A line 7: same treatment as line 4 — refused where it is needed, unread where it \
         is not",
    );
    // ★★ The SSTB checkbox is a DECLARATION: above the phase-in range an SSTB's QBI is excluded
    //    entirely (§199A(d)(3)), so an unasked "no" hands the filer a deduction the statute denies.
    c.exempt(
        is_sstb,
        Class::BenefitClaim,
        "Form 8995-A Part I column (b) (§G-28/B1b) — offered as a SKIPPABLE so a filer below the \
         §199A(e)(2) threshold, for whom the answer changes nothing, is never blocked; above the \
         threshold `screen_absolute` makes it MANDATORY, because past the phase-in range an SSTB's \
         QBI is excluded entirely and an unasked `no` would understate tax",
    );
    // ★★★ The patron flag decides WHICH FORM IS FILED, not just a checkbox: Form 8995-A's header sends
    //     a cooperative patron to 8995-A at ANY income. A `yes` refuses (Schedule D (Form 8995-A) is
    //     not filled), so the only answer that reaches a filed return is a `no`, and an unasked one
    //     would print the simplified Form 8995 for a filer the form sends elsewhere.
    c.exempt(
        is_cooperative_patron,
        Class::BenefitClaim,
        "Form 8995-A Part I column (e) (§G-28/B1b) — offered as a SKIPPABLE; a `yes` REFUSES because \
         the Part II line 14 patron reduction comes from Schedule D (Form 8995-A), which btctax does \
         not fill, and printing line 14 blank would OVERSTATE the deduction",
    );
    c.exempt(
        owner,
        Class::SerdeRequired,
        "§2.8: ScheduleCInputs.owner has no #[serde(default)] — serde-required at import",
    );
    c.exempt(
        accounting_method,
        Class::TrackedFollowup,
        "§2.8: Cash default; `accrual` is accepted, unmodeled and UNREFUSED and flips the printed Sch C \
         line F — a known open Important, filed → P8",
    );
    // ★ Schedule C lines I and J — the Form-1099 compliance pair. Class (B): asked as
    // `SkippableId::ScheduleC1099{Required,Filed}`, silence lawful. Not `declaration(..)` because no
    // figure on the return reads them and the form prints no Caution — but NOT plain "no tax
    // direction" either, because §6721/§6722 exposure is real, which is what the skip advisory names.
    //
    // ★★ PRE-MERGE M8 — these are RECORDED, not bound with `_`. This module's own r2 M-6 rule is that
    // an `Option<bool>` leaf never gets a bare `_`: the whole point of the census is to distinguish
    // "this encodes no decision" from "we forgot it", and `_` erases exactly that distinction. Both
    // were silently under-reported as answered-ness decisions and nothing redded. The four other
    // class-(B) leaves this branch added were all recorded via `exempt`; these two were missed.
    c.exempt(
        payments_requiring_1099,
        Class::BenefitClaim,
        "Schedule C line I — asked as `SkippableId::ScheduleC1099Required`; silence forgoes nothing on \
         the return itself, and the §6721/§6722 exposure is named by the skip advisory (§2.2)",
    );
    c.exempt(
        will_file_required_1099,
        Class::BenefitClaim,
        "Schedule C line J — asked as `SkippableId::ScheduleC1099Filed`, and gated on line I being \
         Yes; an answer to J without a Yes on I is not a mark the form has a place for (§2.2)",
    );
}

fn classify_schedule_a(c: &mut Census, a: &ScheduleAInputs) {
    let ScheduleAInputs {
        medical: _,
        salt_use_sales_tax,
        salt_sales_tax_amount: _,
        salt_state_estimated_payments: _,
        salt_prior_year_balance_paid: _,
        salt_real_estate: _,
        salt_personal_property: _,
        mortgage_interest_1098: _,
        mortgage_all_used_to_buy_build_improve,
        mortgage_within_debt_limit,
        mortgage_dwelling_is_amt_qualified,
        investment_interest: _, // plain `Usd` — the `_` rule permits a money scalar
        charitable,
    } = a;
    c.exempt(
        salt_use_sales_tax,
        Class::BenefitClaim,
        "§164(b)(5) sales-tax election — New Colonial Ice: lawful unasked; `SalesTaxElectionNotAsked` \
         advises when a Schedule A exists (§2.2)",
    );
    c.declaration(
        mortgage_all_used_to_buy_build_improve,
        QuestionId::MortgageAllUsedToBuyBuildImprove,
    );
    c.declaration(
        mortgage_within_debt_limit,
        QuestionId::MortgageWithinDebtLimit,
    );
    c.declaration(
        mortgage_dwelling_is_amt_qualified,
        QuestionId::AmtQualifiedDwelling,
    );
    for g in charitable {
        classify_charitable_gift(c, g);
    }
}

fn classify_charitable_gift(c: &mut Census, g: &CharitableGift) {
    let CharitableGift { class, amount: _ } = g;
    c.exempt(
        class,
        Class::DataDerived,
        "the gift's CONTRIBUTION CLASS is data (which property was given), not a defaulted answer for the \
         filer",
    );
}

/// ★★★ **T16 — Form 8889's seven answers, every one a class-(A) DECLARATION.**
///
/// Their liveness is `sch1.hsa_activity == Some(true)`, so a filer with no HSA trigger is asked
/// none of them and a filer with one is refused until every one is answered. There is no lawful
/// default here: each answer moves the §223(b) contribution limit, the taxable distribution, or the
/// additional tax, and every direction a default could take understates one of them.
fn classify_hsa(c: &mut Census, h: &crate::tax::return_inputs::HsaInputs) {
    let crate::tax::return_inputs::HsaInputs {
        family_coverage,
        spouse_family_coverage,
        eligible_every_month_same_coverage,
        age_55_or_older_at_year_end,
        enrolled_in_medicare_any_month,
        both_spouses_have_hsas,
        line2_contributions_you_made: _,
        archer_msa_activity,
        employer_contributions_prior_year: _,
        employer_contributions_next_year: _,
        line10_qualified_funding_distribution: _,
        line14b_rollovers_and_withdrawn_excess: _,
        line15_qualified_medical_expenses: _,
        line16_amount_meeting_an_exception: _,
        testing_period_failure,
    } = h;
    c.declaration(family_coverage, QuestionId::HsaFamilyCoverage);
    // ★★★ Seam review I-3 — the SPOUSE's plan. Class (A) and live only with a spouse: the
    //     instructions' line 1 and line 3 rule 1 both read "you OR your spouse", so a filer with
    //     self-only coverage and a family-covered spouse is on the FAMILY limit — and answering it
    //     for them would either overstate their limit or, in the other direction, refuse a lawful
    //     contribution as an excess one.
    c.declaration(spouse_family_coverage, QuestionId::HsaSpouseFamilyCoverage);
    c.declaration(
        eligible_every_month_same_coverage,
        QuestionId::HsaEligibleEveryMonth,
    );
    c.declaration(age_55_or_older_at_year_end, QuestionId::HsaAge55OrOlder);
    c.declaration(
        enrolled_in_medicare_any_month,
        QuestionId::HsaMedicareEnrollment,
    );
    c.declaration(both_spouses_have_hsas, QuestionId::HsaBothSpousesHaveHsas);
    c.declaration(archer_msa_activity, QuestionId::HsaArcherMsaActivity);
    c.declaration(testing_period_failure, QuestionId::HsaTestingPeriodFailure);
}

/// ★ T16 — a Form 1099-SA row. Its one non-money leaf is box 5's account-type checkbox, which is
/// TRANSCRIBED DATA (which box the trustee ticked), not a defaulted answer for the filer — and its
/// `None` refuses rather than defaulting to `Hsa`.
fn classify_1099sa(c: &mut Census, r: &crate::tax::return_inputs::Form1099Sa) {
    let crate::tax::return_inputs::Form1099Sa {
        payer: _,
        payer_tin: _,
        transcribed_on: _,
        box1_gross_distribution: _,
        box2_earnings_on_excess: _,
        box3_distribution_code: _,
        box4_fmv_on_date_of_death: _,
        box5_account_type,
    } = r;
    c.exempt(
        box5_account_type,
        Class::DataDerived,
        "Form 1099-SA box 5 — which of HSA / Archer MSA / MA MSA the TRUSTEE ticked, transcribed off \
         the paper. `None` is \"not transcribed\" and REFUSES (SaAccountTypeNotTranscribed); it never \
         defaults to HSA, because that would route an Archer MSA's distribution onto Form 8889 \
         instead of Form 8853",
    );
}

/// ★ T16 — a Form 5498-SA row. Box 6 is the same transcribed checkbox as the 1099-SA's box 5.
fn classify_5498sa(c: &mut Census, r: &crate::tax::return_inputs::Form5498Sa) {
    let crate::tax::return_inputs::Form5498Sa {
        trustee: _,
        trustee_tin: _,
        transcribed_on: _,
        box1_archer_msa_contributions: _,
        box2_total_contributions: _,
        box3_contributions_next_year_for_this_year: _,
        box4_rollover_contributions: _,
        box5_fair_market_value: _,
        box6_account_type,
    } = r;
    c.exempt(
        box6_account_type,
        Class::DataDerived,
        "Form 5498-SA box 6 — the same three-way account checkbox as the Form 1099-SA's box 5, \
         transcribed off the paper; `None` REFUSES rather than defaulting to HSA",
    );
}

fn classify_schedule1(c: &mut Census, s: &Schedule1Inputs) {
    let Schedule1Inputs {
        state_refund_taxable: _,
        ira_deduction_claimed: _,
        hsa_activity,
    } = s;
    c.declaration(hsa_activity, QuestionId::HsaActivity);
}

fn classify_payments(_c: &mut Census, p: &Payments) {
    let Payments {
        estimated_tax_payments: _,
        extension_payment: _,
        other_withholding: _,
    } = p;
}

/// ★ spec 1099-DA R1 — every answer the filer gave, recorded as testimony (provider, cohort, what the
/// form showed). Destructured with no `..` so a new slot on `CohortAnswers` is a compile error here.
fn classify_broker_reporting(c: &mut Census, br: &crate::forms::BrokerReporting) {
    use crate::forms::{Cohort, CohortAnswers};
    for (provider, answers) in &br.0 {
        let CohortAnswers {
            covered,
            noncovered,
        } = answers;
        if let Some(a) = covered {
            c.broker_answers
                .push((provider.clone(), Cohort::Covered, *a));
        }
        if let Some(a) = noncovered {
            c.broker_answers
                .push((provider.clone(), Cohort::Noncovered, *a));
        }
    }
}

fn classify_carryforward(_c: &mut Census, cf: &Carryforward) {
    // FROZEN struct — destructuring it READS it, modifies nothing (§3.3). No classifiable leaves today; the
    // guarantee is about the bool added tomorrow.
    let Carryforward { short: _, long: _ } = cf;
}

fn classify_charitable_carry(c: &mut Census, item: &CharitableCarryItem) {
    let CharitableCarryItem {
        class,
        amount: _,
        origin_year: _,
        provenance,
    } = item;
    c.exempt(
        class,
        Class::DataDerived,
        "the carryover item's contribution class is data, not a defaulted answer",
    );
    c.exempt(
        provenance,
        Class::NoTaxDirection,
        "§2.8: CarryProvenance — no print, no tax direction",
    );
}

fn classify_qbi(c: &mut Census, q: &QbiInputs) {
    let QbiInputs {
        reit_ptp_carryforward_in: _,
        reit_ptp_carryforward_in_provenance,
        qbi_carryforward_in: _,
        qbi_carryforward_in_provenance,
    } = q;
    c.exempt(
        reit_ptp_carryforward_in_provenance,
        Class::NoTaxDirection,
        "§2.8: CarryProvenance — no print, no tax direction",
    );
    c.exempt(
        qbi_carryforward_in_provenance,
        Class::NoTaxDirection,
        "§2.8: CarryProvenance (Form 8995 line 3) — no print, no tax direction",
    );
}

/// ★★★ **R3 / R14 — the DOCUMENT CENSUS is class (A), every row.**
///
/// Eighteen `Option<bool>` leaves, each a [`FORM_QUESTIONS`] declaration, destructured with **no
/// `..` and no `_`** — so a nineteenth row does not compile until a human classifies it. That is the
/// whole point of the census: its own answered-ness must be structural, or it merely relocates the
/// trap it exists to close.
///
/// [`FORM_QUESTIONS`]: crate::tax::questions::FORM_QUESTIONS
fn classify_document_census(c: &mut Census, d: &crate::tax::document_census::DocumentCensus) {
    let crate::tax::document_census::DocumentCensus {
        w2,
        int_1099,
        div_1099,
        b_1099,
        g_1099,
        form_1098,
        form_1098e,
        sa_1099,
        sa_5498,
        r_1099,
        ssa_1099,
        nec_misc_k_1099,
        k1,
        schedule_e_rental,
        s_1099,
        oid_1099,
        w2g,
        c_1099,
        a_1095,
        t_1098,
    } = d;
    c.declaration(w2, QuestionId::DocW2);
    c.declaration(int_1099, QuestionId::DocInt1099);
    c.declaration(div_1099, QuestionId::DocDiv1099);
    c.declaration(b_1099, QuestionId::DocB1099);
    c.declaration(g_1099, QuestionId::DocG1099);
    c.declaration(form_1098, QuestionId::DocForm1098);
    c.declaration(form_1098e, QuestionId::DocForm1098e);
    c.declaration(sa_1099, QuestionId::DocSa1099);
    c.declaration(sa_5498, QuestionId::DocSa5498);
    c.declaration(r_1099, QuestionId::DocR1099);
    c.declaration(ssa_1099, QuestionId::DocSsa1099);
    c.declaration(nec_misc_k_1099, QuestionId::DocNecMiscK1099);
    c.declaration(k1, QuestionId::DocK1);
    c.declaration(schedule_e_rental, QuestionId::DocScheduleERental);
    c.declaration(s_1099, QuestionId::DocS1099);
    c.declaration(oid_1099, QuestionId::DocOid1099);
    c.declaration(w2g, QuestionId::DocW2g);
    c.declaration(c_1099, QuestionId::DocC1099);
    c.declaration(a_1095, QuestionId::DocA1095);
    c.declaration(t_1098, QuestionId::DocT1098);
}

#[cfg(test)]
mod tests {
    /// The names bound with a bare `_` (or a `_`-prefixed binding) anywhere in `classifier_src`.
    fn underscored(classifier_src: &str) -> std::collections::BTreeSet<String> {
        classifier_src
            .lines()
            .filter_map(|l| {
                let t = l.trim().trim_end_matches(',');
                let (name, rhs) = t.split_once(": ")?;
                let name = name.trim();
                (rhs.trim() == "_" || rhs.trim().starts_with('_'))
                    .then(|| name.to_string())
                    // ★ alphanumeric, NOT just lowercase: `form_2555_line45` and `qbi_w2_wages` carry
                    //   digits, and a filter that dropped them would make this rule blind to exactly
                    //   the field names this codebase uses.
                    .filter(|n: &String| n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            })
            .collect()
    }

    /// Every `pub <name>: Option<Usd>` declared in `inputs_src`.
    fn option_money_fields(inputs_src: &str) -> std::collections::BTreeSet<String> {
        inputs_src
            .lines()
            .filter_map(|l| {
                let t = l.trim();
                let rest = t.strip_prefix("pub ")?;
                let (name, ty) = rest.split_once(": ")?;
                (ty.trim_end_matches(',').trim() == "Option<Usd>").then(|| name.trim().to_string())
            })
            .collect()
    }

    /// The rule, as a pure function of the two sources, so a planted defect can reach it.
    fn violations(inputs_src: &str, classifier_src: &str) -> Vec<String> {
        let under = underscored(classifier_src);
        option_money_fields(inputs_src)
            .into_iter()
            .filter(|f| under.contains(f))
            .collect()
    }

    /// ★★★ `_` is FORBIDDEN on an `Option<Usd>` leaf — the answered-ness shape for money.
    ///
    /// `Option<Usd>`'s `None` means **never asked**, which is precisely what this module exists to stop
    /// anyone adding without looking at. The rule permitted it until 2026-08-02, and the codebase had
    /// already ROUTED AROUND the gap: `return_inputs.rs` explains that the §911/931/933 answered-ness
    /// lives in an `Option<bool>` gate *because* `Option<bool>` forbids `_` "whereas `Option<Usd>` is a
    /// scalar the `_` rule permits". A design picking its type to dodge a hole is the strongest evidence
    /// the hole was real.
    ///
    /// ★ Today the real sources have ZERO `Option<Usd>` fields, so a test asserting only on them would
    /// pass vacuously and prove nothing — B1's "shipped an instrument never seen discriminating". The
    /// rule is therefore a pure function over both sources, and the planted cases below drive it.
    #[test]
    fn no_option_money_leaf_is_bound_with_underscore() {
        // The real sources, which must be clean.
        assert!(
            violations(
                include_str!("return_inputs.rs"),
                include_str!("classifier.rs")
            )
            .is_empty(),
            "an Option<Usd> leaf is bound with `_` — classify it instead"
        );

        // ── The kills. ──────────────────────────────────────────────────────────────────────────
        let inputs =
            "pub qbi_w2_wages: Option<Usd>,\n    pub medical: Usd,\n    pub blind: Option<bool>,";

        // (1) THE DEFECT: an Option<Usd> money leaf waved past with `_`.
        assert_eq!(
            violations(inputs, "        qbi_w2_wages: _,"),
            vec!["qbi_w2_wages".to_string()],
            "a `_` on an Option<Usd> leaf must be a finding"
        );
        // (2) …and a `_`-PREFIXED binding is the same evasion wearing a name.
        assert_eq!(
            violations(inputs, "        qbi_w2_wages: _unused,"),
            vec!["qbi_w2_wages".to_string()]
        );
        // (3) Bound by name (i.e. classified) ⇒ silent. Without this the rule could red on everything,
        //     which reds on nothing.
        assert!(violations(inputs, "        qbi_w2_wages,").is_empty());
        // (4) A plain `Usd` keeps its permission — narrowing the rule further is a separate decision,
        //     and a test that quietly forbade `_` on every money field would be making it here.
        assert!(violations(inputs, "        medical: _,").is_empty());
    }

    use super::*;

    /// ★ §3.3 / §4 — the classifier's registry DECLARATIONS line up EXACTLY with [`FORM_QUESTIONS`]: every
    /// registry `QuestionId` is declared once, and nothing is declared that is not a registry question. A
    /// wrong or dropped `declaration(..)` (a mis-classification the COMPILER cannot catch — the honest
    /// limit) fails here. Exercised on a fixture with a Schedule A so the mortgage declaration runs.
    #[test]
    fn every_registry_question_is_declared_exactly_once() {
        let mut ri = ReturnInputs {
            schedule_a: Some(ScheduleAInputs::default()),
            // ★ A Schedule C too, because the SSTB declaration (§G-28/B1b) is declared inside
            //   `classify_schedule_c` — a census over a return with no business would simply never
            //   visit it, and this test would report a missing declaration that is really a missing
            //   FIXTURE. Populate every optional section the registry has a question for.
            schedule_c: Some(crate::tax::return_inputs::ScheduleCInputs::default()),
            ..Default::default()
        };
        // A spouse Person exercises the spouse branch too (its bools are on the header, always destructured,
        // but this keeps the census exercise faithful to a populated return).
        ri.header.spouse = Some(Person::default());
        let census = classify(&ri);

        for id in QuestionId::ALL {
            assert_eq!(
                census.declarations.iter().filter(|d| *d == id).count(),
                1,
                "classifier must declare {id:?} exactly once (registry ⇔ classifier, §3.3)"
            );
        }
        assert_eq!(
            census.declarations.len(),
            QuestionId::ALL.len(),
            "the classifier declares EXACTLY the registry questions — no more, no less"
        );
    }

    /// ★★★ **T7 / R6 — the classifier's DEPENDENT GATES line up EXACTLY with `DEPENDENT_GATES`,
    /// both directions.** A gate classified here but absent from the registry has no prompt, no
    /// liveness and no refusal; one in the registry but not classified is a leaf whose answered-ness
    /// nobody decided. Neither is a compile error — the `Dependent` destructure catches only a NEW
    /// field, not a `c.dependent_gate(x, WrongGate)` — so it is caught here.
    ///
    /// ★ `DateOfBirth` is deliberately NOT in the classifier's list: it is an `Option<Date>`, a
    ///   scalar the classifier's `_` rule permits, and its answered-ness is carried by its
    ///   `DEPENDENT_GATES` entry. The exclusion is asserted rather than assumed.
    /// ★★★ **SEAM REVIEW N-2 — EVERY CLASSIFIER ROW PAIRS ITS GATE WITH ITS OWN LEAF.**
    ///
    /// `Census::dependent_gate` used to ignore the leaf and push only the gate, and the lineup test
    /// above compares SETS — so mis-pairing two calls in `classify_dependent` (say
    /// `c.dependent_gate(married, G::FilingJointReturn)`) left the set identical and red nothing.
    /// Nothing user-visible moves today, because the census only counts; the two registries look
    /// alike and only the input form's was protected.
    ///
    /// The pairing is checked one gate at a time, DERIVED from the registry: set exactly one gate
    /// through its own `set` accessor and require the census to report that value against that gate
    /// and `None` against every other.
    #[test]
    fn every_classifier_row_pairs_its_gate_with_its_own_leaf() {
        use crate::tax::dependent_gates::{GateKind, DEPENDENT_GATES};
        for q in DEPENDENT_GATES.iter().filter(|q| q.kind == GateKind::YesNo) {
            let mut ri = ReturnInputs::default();
            ri.header.dependents = vec![crate::tax::return_inputs::Dependent::default()];
            (q.set)(&mut ri.header.dependents[0], true);
            let rows = classify(&ri).dependent_gates;
            let answered: Vec<_> = rows
                .iter()
                .filter(|(_, leaf)| leaf.is_some())
                .copied()
                .collect();
            assert_eq!(
                answered,
                vec![(q.gate, Some(true))],
                "the ONE answered leaf must be classified as {:?} and nothing else — a swapped \
                 pairing reports another gate's name against this field",
                q.gate
            );
        }
    }

    #[test]
    fn the_classifiers_gate_rows_line_up_with_the_registry() {
        use crate::tax::dependent_gates::{GateKind, DEPENDENT_GATES};
        use crate::tax::provenance::DependentGate;
        let mut ri = ReturnInputs::default();
        ri.header.dependents = vec![crate::tax::return_inputs::Dependent::default()];
        let census = classify(&ri);

        let classified: std::collections::BTreeSet<DependentGate> =
            census.dependent_gates.iter().map(|(g, _)| *g).collect();
        let registered: std::collections::BTreeSet<DependentGate> = DEPENDENT_GATES
            .iter()
            .filter(|q| q.kind == GateKind::YesNo)
            .map(|q| q.gate)
            .collect();
        assert_eq!(
            classified, registered,
            "every `Option<bool>` gate in the registry is classified, and nothing else is"
        );
        assert_eq!(
            census.dependent_gates.len(),
            registered.len(),
            "…exactly once per row, no duplicates"
        );
        assert!(
            !classified.contains(&DependentGate::DateOfBirth),
            "the date of birth is an Option<Date> scalar; its answered-ness is the registry's"
        );

        // ★ ONE ENTRY PER ROW: two dependents ⇒ twice the rows. A classifier that visited only the
        //   first would be blind to exactly the second child's leaves.
        ri.header
            .dependents
            .push(crate::tax::return_inputs::Dependent::default());
        assert_eq!(
            classify(&ri).dependent_gates.len(),
            registered.len() * 2,
            "the census is per ROW, not per struct type"
        );
    }

    /// The classifier runs over a fully-defaulted return without panicking, and records exemptions with
    /// real statutory reasons (never an empty string).
    #[test]
    fn classify_runs_and_every_exemption_carries_a_reason() {
        let census = classify(&ReturnInputs::default());
        assert!(
            !census.exemptions.is_empty(),
            "a defaulted return still has exempt leaves (filing_status, itemize_election, presidential ×2, \
             taxpayer.blind, …)"
        );
        for (_class, reason) in &census.exemptions {
            assert!(
                !reason.trim().is_empty(),
                "every exemption states WHY it is lawful"
            );
        }
    }
}
