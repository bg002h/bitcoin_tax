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
//! collections, and `bool` / `Option<bool>` / defaulted-enum leaves (they must recurse or be classified).
//! `_` is **PERMITTED** on other scalar leaves (`String`, `Usd`, `Date`, `Option<Date>`, `Option<String>`).
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
        filing_status,
        header,
        w2s,
        int_1099,
        div_1099,
        g_1099,
        schedule_c,
        schedule_a,
        itemize_election,
        mfs_spouse_itemizes,
        sch1,
        payments,
        capital_loss_carryforward_in,
        charitable_carryover_in,
        qbi,
        foreign_accounts,
        foreign_trust,
        foreign_country_names: _, // String — scalar
        dual_status_alien,
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
    c.declaration(dual_status_alien, QuestionId::DualStatusAlien);
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
        presidential_fund_taxpayer,
        presidential_fund_spouse,
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

fn classify_schedule_c(c: &mut Census, sc: &ScheduleCInputs) {
    let ScheduleCInputs {
        owner,
        business_description: _,
        naics_code: _,
        accounting_method,
        expenses: _,
    } = sc;
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
    } = q;
    c.exempt(
        reit_ptp_carryforward_in_provenance,
        Class::NoTaxDirection,
        "§2.8: CarryProvenance — no print, no tax direction",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ §3.3 / §4 — the classifier's registry DECLARATIONS line up EXACTLY with [`FORM_QUESTIONS`]: every
    /// registry `QuestionId` is declared once, and nothing is declared that is not a registry question. A
    /// wrong or dropped `declaration(..)` (a mis-classification the COMPILER cannot catch — the honest
    /// limit) fails here. Exercised on a fixture with a Schedule A so the mortgage declaration runs.
    #[test]
    fn every_registry_question_is_declared_exactly_once() {
        let mut ri = ReturnInputs {
            schedule_a: Some(ScheduleAInputs::default()),
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
