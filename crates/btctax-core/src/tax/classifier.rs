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
    ScheduleAInputs, ScheduleCInputs, W2,
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
        w2s,
        int_1099,
        div_1099,
        g_1099,
        b_1099,
        schedule_c,
        schedule_a,
        itemize_election,
        mfs_spouse_itemizes,
        sch1,
        payments,
        capital_loss_carryforward_in,
        capital_loss_carryforward_in_provenance,
        charitable_carryover_in_provenance,
        amt_carryover_same_as_regular,
        amt_depreciation_same_as_regular,
        charitable_carryover_in,
        qbi,
        foreign_accounts,
        foreign_trust,
        fbar_filing_required,
        foreign_country_names: _, // String — scalar
        donations_had_restrictions,
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
    } = ri;
    c.exempt(
        filing_status,
        Class::SerdeRequired,
        "filing_status has no #[serde(default)] — a TOML without it refuses to parse, so no default to \
         launder (§2.1)",
    );
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
    c.declaration(dual_status_alien, QuestionId::DualStatusAlien);
    c.declaration(has_income_exclusion, QuestionId::HasIncomeExclusion);
    c.declaration(other_out_of_scope_income, QuestionId::OtherOutOfScopeIncome);
    c.declaration(
        amt_carryover_same_as_regular,
        QuestionId::AmtCarryoverSameAsRegular,
    );
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
    if let Some(sc) = schedule_c {
        classify_schedule_c(&mut c, sc);
    }
    if let Some(a) = schedule_a {
        classify_schedule_a(&mut c, a);
    }
    classify_schedule1(&mut c, sch1);
    classify_payments(&mut c, payments);
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
    } = h;
    c.declaration(
        can_be_claimed_as_dependent_taxpayer,
        QuestionId::DependentTaxpayer,
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

fn classify_dependent(_c: &mut Census, d: &Dependent) {
    // No classifiable leaves — but destructured with no `..` so a future bool here is a compile error.
    let Dependent {
        name: _,
        ssn: _,
        relationship: _,
        date_of_birth: _,
    } = d;
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
    } = w;
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
        box12_exempt_interest_dividends: _,
        box13_private_activity_amt: _,
    } = d;
}

fn classify_1099g(_c: &mut Census, g: &Form1099G) {
    let Form1099G {
        payer: _,
        box1_unemployment: _,
        box4_fed_withheld: _,
    } = g;
}

/// §G-28/B4 — Form 1099-B totals for Schedule D lines 1a/8a.
fn classify_1099b(c: &mut Census, b: &crate::tax::return_inputs::Form1099B) {
    let crate::tax::return_inputs::Form1099B {
        payer: _,
        short_term_proceeds: _,
        short_term_basis: _,
        long_term_proceeds: _,
        long_term_basis: _,
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
        mortgage_dwelling_is_amt_qualified,
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

fn classify_schedule1(c: &mut Census, s: &Schedule1Inputs) {
    let Schedule1Inputs {
        state_refund_taxable: _,
        student_loan_interest_paid: _,
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
